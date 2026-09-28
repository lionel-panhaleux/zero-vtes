"""PPO self-play (DESIGN §5, P3): one network plays every seat of a
`zvtes_py.GamePool`; each seat's decisions form its own chain, rewarded at
the game's end by tournament scoring — (VPs + 1 for the game win) / 5.
Rollouts are truncated: a chain's last decision in a rollout bootstraps
the ones before it and is carried into the next rollout. Evaluation plays
the policy (sampling) against random seats and the scripted bots.

    uv run python -m train.ppo --iterations 200 --out runs/first
"""

import argparse
import dataclasses
import json
import time
from pathlib import Path

import numpy as np
import torch
import zvtes_py

from train.model import Policy


@dataclasses.dataclass
class Config:
    games: int = 256
    steps: int = 64
    iterations: int = 100
    epochs: int = 3
    minibatch: int = 1024
    lr: float = 3e-4
    gamma: float = 1.0
    lam: float = 0.95
    clip: float = 0.2
    value_coef: float = 0.5
    entropy_coef: float = 0.01
    d: int = 96
    layers: int = 2
    seed: int = 0
    eval_every: int = 10
    eval_games: int = 96
    budget: int = 20_000
    device: str = "auto"
    out: str = "runs/ppo"
    resume: str = ""


def device_of(name):
    if name != "auto":
        return torch.device(name)
    return torch.device("mps" if torch.backends.mps.is_available() else "cpu")


FIELDS = ("tok_ids", "tok_f", "opt_ids", "opt_f")


def rows(obs):
    """Per-game observations, trimmed to their token and option counts."""
    out = []
    for i in range(len(obs["tok_n"])):
        t, o = obs["tok_n"][i], obs["opt_n"][i]
        out.append(
            (
                obs["tok_ids"][i, :t],
                obs["tok_f"][i, :t].astype(np.float16),
                obs["opt_ids"][i, :o],
                obs["opt_f"][i, :o].astype(np.float16),
            )
        )
    return out


def collate(samples, device):
    """Pad trimmed observations into a batch."""
    b = len(samples)
    t = max(s[0].shape[0] for s in samples)
    o = max(s[2].shape[0] for s in samples)
    out = {
        "tok_ids": np.zeros((b, t, samples[0][0].shape[1]), np.int64),
        "tok_f": np.zeros((b, t, samples[0][1].shape[1]), np.float32),
        "opt_ids": np.zeros((b, o, samples[0][2].shape[1]), np.int64),
        "opt_f": np.zeros((b, o, samples[0][3].shape[1]), np.float32),
    }
    tok_n, opt_n = np.zeros(b, np.int64), np.zeros(b, np.int64)
    for i, s in enumerate(samples):
        tok_n[i], opt_n[i] = s[0].shape[0], s[2].shape[0]
        out["tok_ids"][i, : tok_n[i]] = s[0]
        out["tok_f"][i, : tok_n[i]] = s[1]
        out["opt_ids"][i, : opt_n[i]] = s[2]
        out["opt_f"][i, : opt_n[i]] = s[3]
    batch = {k: torch.from_numpy(v).to(device) for k, v in out.items()}
    batch["tok_n"] = torch.from_numpy(tok_n).to(device)
    batch["opt_n"] = torch.from_numpy(opt_n).to(device)
    return batch


def batch_of(obs, device):
    batch = {k: torch.from_numpy(obs[k]).to(device) for k in FIELDS}
    batch["tok_ids"] = batch["tok_ids"].long()
    batch["opt_ids"] = batch["opt_ids"].long()
    batch["tok_n"] = torch.from_numpy(obs["tok_n"]).long().to(device)
    batch["opt_n"] = torch.from_numpy(obs["opt_n"]).long().to(device)
    return batch


def forward(model, batch):
    return model(
        batch["tok_ids"],
        batch["tok_f"],
        batch["tok_n"],
        batch["opt_ids"],
        batch["opt_f"],
        batch["opt_n"],
    )


def scores(ep):
    """VPs and game wins per seat (tournament rules 3.7: at least 2 VPs and
    strictly more than every other seat)."""
    vps = np.array(ep["half_vps"], np.float32) / 2
    best = vps.max()
    return vps, (vps == best) & (best >= 2) & ((vps == best).sum() == 1)


def rewards(ep):
    """Tournament scoring per seat: (VPs + 1 for the game win) / 5."""
    vps, win = scores(ep)
    return (vps + win) / 5


@torch.no_grad()
def act(model, obs, device, rng):
    logits, values = forward(model, batch_of(obs, device))
    probs = torch.softmax(logits.float(), -1).cpu().numpy().astype(np.float64)
    probs /= probs.sum(1, keepdims=True)
    actions = np.array([rng.choice(len(p), p=p) for p in probs])
    logp = np.log(probs[np.arange(len(actions)), actions] + 1e-12)
    return actions, logp, values.float().cpu().numpy()


class Trainer:
    def __init__(self, cfg):
        self.cfg = cfg
        self.device = device_of(cfg.device)
        self.schema = json.loads(zvtes_py.schema())
        torch.manual_seed(cfg.seed)
        self.rng = np.random.default_rng(cfg.seed)
        self.model = Policy(self.schema, d=cfg.d, layers=cfg.layers).to(self.device)
        self.opt = torch.optim.Adam(self.model.parameters(), lr=cfg.lr)
        self.start = 1
        if cfg.resume:
            saved = torch.load(cfg.resume, map_location=self.device)
            self.model.load_state_dict(saved["model"])
            if "optimizer" in saved:
                self.opt.load_state_dict(saved["optimizer"])
            self.start = saved.get("iteration", 0) + 1
        self.pool = zvtes_py.GamePool(
            cfg.games, ["policy"] * 4, seed=cfg.seed, budget=cfg.budget
        )
        # (slot, seat) -> transitions [obs, action, logp, value, reward, done]
        self.chains = {}
        self.out = Path(cfg.out)
        self.out.mkdir(parents=True, exist_ok=True)

    def rollout(self):
        """Play `steps` pool steps; the trainable transitions with their
        advantages and returns, and episode statistics."""
        closed, stats = [], {"episodes": 0, "truncated": 0, "turns": 0}
        for _ in range(self.cfg.steps):
            obs = self.pool.observe()
            actions, logp, values = act(self.model, obs, self.device, self.rng)
            for i, row in enumerate(rows(obs)):
                key = (i, int(obs["seat"][i]))
                self.chains.setdefault(key, []).append(
                    [row, int(actions[i]), float(logp[i]), float(values[i]), 0.0, False]
                )
            for ep in self.pool.act(actions.tolist()):
                stats["episodes"] += 1
                stats["truncated"] += ep["truncated"]
                stats["turns"] += ep["turns"]
                r = rewards(ep)
                for s in range(len(r)):
                    chain = self.chains.pop((ep["slot"], s), None)
                    if chain:
                        chain[-1][4], chain[-1][5] = float(r[s]), True
                        closed.append(chain)
        # Open chains: the last decision bootstraps the others and waits.
        train = []
        for key, chain in list(self.chains.items()):
            if len(chain) > 1:
                train.append(chain)
                self.chains[key] = [chain[-1]]
        samples = []
        for chain in closed:
            samples += self.advantages(chain, bootstrap=None)
        for chain in train:
            samples += self.advantages(chain[:-1], bootstrap=chain[-1][3])
        return samples, stats

    def advantages(self, chain, bootstrap):
        cfg, out, gae = self.cfg, [], 0.0
        next_value = bootstrap if bootstrap is not None else 0.0
        for row, a, logp, v, r, done in reversed(chain):
            if done:
                next_value, gae = 0.0, 0.0
            delta = r + cfg.gamma * next_value - v
            gae = delta + cfg.gamma * cfg.lam * gae
            out.append((row, a, logp, gae, gae + v))
            next_value = v
        return out

    def update(self, samples):
        cfg = self.cfg
        adv = np.array([s[3] for s in samples], np.float32)
        adv = (adv - adv.mean()) / (adv.std() + 1e-8)
        losses = []
        for _ in range(cfg.epochs):
            order = self.rng.permutation(len(samples))
            for k in range(0, len(order), cfg.minibatch):
                idx = order[k : k + cfg.minibatch]
                batch = collate([samples[i][0] for i in idx], self.device)
                logits, values = forward(self.model, batch)
                logp_all = torch.log_softmax(logits, -1)
                a = torch.tensor([samples[i][1] for i in idx], device=self.device)
                logp = logp_all.gather(1, a[:, None]).squeeze(1)
                old = torch.tensor([samples[i][2] for i in idx], device=self.device)
                ad = torch.tensor(adv[idx], device=self.device)
                ret = torch.tensor([samples[i][4] for i in idx], device=self.device)
                ratio = torch.exp(logp - old)
                pg = -torch.min(ratio * ad, ratio.clamp(1 - cfg.clip, 1 + cfg.clip) * ad).mean()
                vf = ((values - ret) ** 2).mean()
                ent = -(logp_all.exp() * logp_all).sum(-1).mean()
                loss = pg + cfg.value_coef * vf - cfg.entropy_coef * ent
                self.opt.zero_grad()
                loss.backward()
                torch.nn.utils.clip_grad_norm_(self.model.parameters(), 1.0)
                self.opt.step()
                losses.append((pg.item(), vf.item(), ent.item()))
        pg, vf, ent = np.mean(losses, 0)
        return {"pg": pg, "vf": vf, "entropy": ent}

    def evaluate(self, seats, games):
        """The policy's mean VPs and game-win rate over `games` finished
        games, every seat position in turn."""
        pool = zvtes_py.GamePool(
            min(games, 64), seats, seed=10_000 + self.cfg.seed, rotate=True, budget=self.cfg.budget
        )
        rng = np.random.default_rng(0)
        vps, wins, done, truncated = [], 0, 0, 0
        while done < games:
            actions, _, _ = act(self.model, pool.observe(), self.device, rng)
            for ep in pool.act(actions.tolist()):
                if done >= games:
                    break
                done += 1
                truncated += ep["truncated"]
                v, w = scores(ep)
                for s, kind in enumerate(ep["seats"]):
                    if kind == "policy":
                        vps.append(float(v[s]))
                        wins += bool(w[s])
        return {"vps": float(np.mean(vps)), "gw": wins / games, "truncated": truncated}

    def run(self):
        cfg = self.cfg
        log = open(self.out / "metrics.jsonl", "a")
        for it in range(self.start, cfg.iterations + 1):
            t0 = time.perf_counter()
            self.model.eval()
            samples, stats = self.rollout()
            t1 = time.perf_counter()
            self.model.train()
            metrics = self.update(samples)
            t2 = time.perf_counter()
            stats["turns"] = round(stats["turns"] / max(stats["episodes"], 1), 1)
            row = {
                "iteration": it,
                "samples": len(samples),
                "rollout_s": round(t1 - t0, 2),
                "update_s": round(t2 - t1, 2),
                **stats,
                **{k: round(float(v), 4) for k, v in metrics.items()},
            }
            if it % cfg.eval_every == 0 or it == cfg.iterations:
                self.model.eval()
                row["vs_random"] = self.evaluate(["policy", "random", "random", "random"], cfg.eval_games)
                row["vs_bots"] = self.evaluate(["policy", "aggro", "wall", "aggro"], cfg.eval_games)
                torch.save(
                    {
                        "model": self.model.state_dict(),
                        "optimizer": self.opt.state_dict(),
                        "iteration": it,
                        "config": dataclasses.asdict(cfg),
                    },
                    self.out / "model.pt",
                )
            print(json.dumps(row), flush=True)
            log.write(json.dumps(row) + "\n")
            log.flush()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for f in dataclasses.fields(Config):
        parser.add_argument("--" + f.name.replace("_", "-"), type=type(f.default), default=f.default)
    args = parser.parse_args()
    Trainer(Config(**vars(args))).run()


if __name__ == "__main__":
    main()
