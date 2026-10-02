"""PPO (DESIGN §5, P3–P4) on 4- and 5-seat tables of a `zvtes_py.GamePool`:
self-play (the learner, `net:0`, at every seat), or with `--league` the
tables a `train.league.League` sets — one learner seat, an exploiter seat
(a second trained network), the rest bots and frozen snapshots. Each
trained seat's decisions form a chain, rewarded at the game's end by
(VPs + `gw_weight` × the game win) / 5. Rollouts are truncated: a chain's
last decision in a rollout bootstraps the ones before it and is carried
into the next rollout. Evaluation plays the learner (sampling) against
random seats and the scripted bots.

    uv run python -m train.ppo --iterations 200 --out runs/first
"""

import argparse
import copy
import dataclasses
import itertools
import json
import time
from pathlib import Path

import numpy as np
import torch
import zvtes_py

from train.league import EXPLOITER, TABLES, League, label
from train.model import Critic, Policy, remap_vocab


@dataclasses.dataclass
class Config:
    games: int = 1024
    steps: int = 64
    iterations: int = 100
    epochs: int = 3
    minibatch: int = 1024
    lr: float = 3e-4
    # The learning rate tapers linearly over the run (from its start
    # iteration, a resumed one too) to `lr_final` × `lr`.
    lr_final: float = 0.1
    gamma: float = 1.0
    lam: float = 0.95
    clip: float = 0.2
    value_coef: float = 0.5
    entropy_coef: float = 0.02
    # Adaptive entropy bonus: after each update the coefficient is scaled by
    # `entropy_step` toward keeping the policy's entropy at `entropy_target`
    # (0: fixed coefficient). Every run so far peaked against the bots near
    # 0.64 and declined once the entropy fell under ~0.6 (DESIGN §11.2).
    entropy_target: float = 0.65
    entropy_step: float = 1.1
    # The omniscient critic (DESIGN §5): the learner's values come from a
    # separate network reading every hand (`encode_critic`), trained on the
    # same returns; it starts as a copy of the policy's encoder and value
    # head. Training only: the policy plays from the legal view.
    critic: bool = True
    # A new critic's values are poor (a new input): its first
    # `critic_warmup` iterations train it alone, the policy frozen.
    critic_warmup: int = 10
    # The game win's weight in the reward, in VPs (tournament ranking puts
    # game wins first).
    gw_weight: float = 2.0
    d: int = 96
    layers: int = 2
    seed: int = 0
    eval_every: int = 10
    eval_games: int = 256
    budget: int = 20_000
    device: str = "auto"
    out: str = "runs/ppo"
    resume: str = ""
    # The league (train/league.py).
    # Deck families drawn per seat, comma-separated: "precons" (the 14 V5
    # precons), "twda" (tournament winners fully in the pool). The New Blood
    # minis are half decks, not legal: never trained on.
    decks: str = "precons"
    league: bool = False
    snapshot_every: int = 10
    opponents_kept: int = 4
    # The share of opponent seats drawn among the bots (the rest among the
    # snapshots): the evaluation is against the bots (owner, 2026-09-29).
    bot_seats: float = 0.6
    # The bots drawn for those seats, comma-separated among aggro, wall,
    # random (the evaluation always plays aggro and wall).
    bots: str = "aggro,wall,random"
    # At most this many bot seats a table (once a snapshot exists).
    table_bots: int = 5
    # Checkpoint selection (`best.pt`): see `run`.
    best_seeds: int = 3
    best_margin: float = 0.1
    pfsp_rate: float = 0.1
    retire_random: float = 0.85
    # Off: with the learner's reward and weights it never led the learner
    # (DESIGN §11.2, second league run).
    exploiter: bool = False
    exploiter_beaten: float = 0.3
    exploiter_reset: int = 50


def device_of(name):
    if name != "auto":
        return torch.device(name)
    return torch.device("mps" if torch.backends.mps.is_available() else "cpu")


FIELDS = ("tok_ids", "tok_f", "opt_ids", "opt_f")


def rows(obs):
    """Per-game observations, trimmed to their token and option counts;
    with the critic's tokens appended (ids, features) when the pool emits
    them."""
    out = []
    critic = "critic_tok_n" in obs
    for i in range(len(obs["tok_n"])):
        t, o = obs["tok_n"][i], obs["opt_n"][i]
        row = (
            obs["tok_ids"][i, :t],
            obs["tok_f"][i, :t].astype(np.float16),
            obs["opt_ids"][i, :o],
            obs["opt_f"][i, :o].astype(np.float16),
        )
        if critic:
            c = obs["critic_tok_n"][i]
            row += (obs["critic_tok_ids"][i, :c], obs["critic_tok_f"][i, :c].astype(np.float16))
        out.append(row)
    return out


def collate_critic(samples, device):
    """Pad the critic's trimmed tokens (row items 4 and 5) into a batch."""
    b = len(samples)
    t = max(s[4].shape[0] for s in samples)
    ids = np.zeros((b, t, samples[0][4].shape[1]), np.int64)
    f = np.zeros((b, t, samples[0][5].shape[1]), np.float32)
    n = np.zeros(b, np.int64)
    for i, s in enumerate(samples):
        n[i] = s[4].shape[0]
        ids[i, : n[i]] = s[4]
        f[i, : n[i]] = s[5]
    return (
        torch.from_numpy(ids).to(device),
        torch.from_numpy(f).to(device),
        torch.from_numpy(n).to(device),
    )


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


def rewards(ep, gw_weight):
    """Per seat: (VPs + `gw_weight` for the game win) / 5."""
    vps, win = scores(ep)
    return (vps + gw_weight * win) / 5


def eval_tables(opponents, games):
    """`games` seatings: the learner (`net:0`) at every seat in turn, then
    the next distinct order of `opponents` around it."""
    orders = sorted(set(itertools.permutations(opponents)))
    n = len(opponents) + 1
    out = []
    for g in range(games):
        seats = list(orders[(g // n) % len(orders)])
        seats.insert(g % n, "net:0")
        out.append(seats)
    return out


@torch.no_grad()
def act(nets, obs, device, rng):
    """Sample every row's option from its network (`obs["agent"]` → `nets`):
    actions, their log-probabilities and the values."""
    b = len(obs["tok_n"])
    actions, logp, values = np.zeros(b, np.int64), np.zeros(b), np.zeros(b, np.float32)
    for agent in np.unique(obs["agent"]):
        idx = np.nonzero(obs["agent"] == agent)[0]
        part = {k: v[idx] for k, v in obs.items()}
        logits, v = forward(nets[int(agent)], batch_of(part, device))
        probs = torch.softmax(logits.float(), -1).cpu().numpy().astype(np.float64)
        probs /= probs.sum(1, keepdims=True)
        a = np.array([rng.choice(len(p), p=p) for p in probs])
        actions[idx] = a
        logp[idx] = np.log(probs[np.arange(len(a)), a] + 1e-12)
        values[idx] = v.float().cpu().numpy()
    return actions, logp, values


class Learner:
    """A trained network: its model, optimizer and open chains."""

    def __init__(self, agent, model, lr):
        self.agent = agent
        self.model = model
        self.opt = torch.optim.Adam(model.parameters(), lr=lr)
        # (slot, seat) -> transitions [obs, action, logp, value, reward, done]
        self.chains = {}

    def restart(self, weights, lr):
        """New weights and a fresh optimizer; the open chains (sampled by
        the old policy) are dropped."""
        self.model.load_state_dict(weights)
        self.opt = torch.optim.Adam(self.model.parameters(), lr=lr)
        self.chains = {}


class Trainer:
    def __init__(self, cfg):
        self.cfg = cfg
        self.device = device_of(cfg.device)
        self.schema = json.loads(zvtes_py.schema())
        torch.manual_seed(cfg.seed)
        self.rng = np.random.default_rng(cfg.seed)
        model = Policy(self.schema, d=cfg.d, layers=cfg.layers).to(self.device)
        self.learner = Learner(0, model, cfg.lr)
        self.model = model
        self.start = 1
        saved, same_shapes = {}, False
        self.vocab = zvtes_py.vocab_keys()
        if cfg.resume:
            saved = torch.load(cfg.resume, map_location=self.device)
            old = saved.get("vocab", self.vocab)
            model.load_state_dict(remap_vocab(saved["model"], old, self.vocab, model.state_dict()))
            # The optimizer's moments are shaped by the vocabulary and the
            # token types: kept only when neither changed.
            same_shapes = old == self.vocab and all(
                v.shape == saved["model"][k].shape for k, v in model.state_dict().items()
            )
            if "optimizer" in saved and same_shapes:
                self.learner.opt.load_state_dict(saved["optimizer"])
            self.start = saved.get("iteration", 0) + 1
        self.critic = None
        self.warmup = 0
        if cfg.critic:
            self.critic = Critic(self.schema, d=cfg.d, layers=cfg.layers).to(self.device)
            if "critic" in saved:
                old = saved.get("vocab", self.vocab)
                self.critic.load_state_dict(
                    remap_vocab(saved["critic"], old, self.vocab, self.critic.state_dict())
                )
            else:
                # A copy of the policy's encoder and value head.
                self.critic.load_state_dict(model.state_dict(), strict=False)
                self.warmup = cfg.critic_warmup
            self.critic_opt = torch.optim.Adam(self.critic.parameters(), lr=cfg.lr)
            if "critic_optimizer" in saved and same_shapes:
                self.critic_opt.load_state_dict(saved["critic_optimizer"])
            self.warmup = saved.get("critic_warmup", self.warmup)
        # The best evaluation against aggro/wall/aggro so far (`best.pt`,
        # kept across a resume into the same directory).
        same = cfg.resume and Path(cfg.resume).parent.resolve() == Path(cfg.out).resolve()
        self.best = saved.get("best", float("-inf")) if same else float("-inf")
        self.entropy_coef = saved.get("entropy_coef", cfg.entropy_coef) if same else cfg.entropy_coef
        self.out = Path(cfg.out)
        self.out.mkdir(parents=True, exist_ok=True)
        self.league = None
        self.exploiter = None
        if cfg.league:
            self.league = League(cfg, self.rng, self.out)
            self.league.load(model, self.vocab)
            if not self.league.learner_snapshots() and self.start > 1:
                self.league.add(self.start - 1, model)
            if "pfsp" in saved:
                self.league.restore(saved["pfsp"])
            if cfg.exploiter:
                self.exploiter = Learner(EXPLOITER, copy.deepcopy(model), cfg.lr)
                if "exploiter" in saved:
                    self.exploiter.model.load_state_dict(saved["exploiter"])
                    self.exploiter.opt.load_state_dict(saved["exploiter_optimizer"])
        families = dict(zvtes_py.deck_families())
        names = [f.strip() for f in cfg.decks.split(",")]
        if not set(names) <= {"precons", "twda"}:
            raise ValueError(f"deck families: precons, twda (got {cfg.decks!r})")
        self.decks = [i for f in names for i in families[f]]
        self.pool = zvtes_py.GamePool(
            cfg.games,
            ["net:0"] * 4,
            seed=cfg.seed,
            decks=self.decks,
            budget=cfg.budget,
            tables=[self.spec() for _ in range(cfg.games)],
            critic=cfg.critic,
        )

    def learners(self):
        return [self.learner] + ([self.exploiter] if self.exploiter else [])

    def nets(self):
        if not self.league:
            return {0: self.model}
        return self.league.nets(self.model, self.exploiter.model if self.exploiter else None)

    def spec(self):
        if self.league:
            return self.league.spec()
        return {"seats": ["net:0"] * int(self.rng.choice(TABLES))}

    def rollout(self):
        """Play `steps` pool steps; each learner's transitions with their
        advantages and returns, and episode statistics."""
        stats = {"episodes": 0, "truncated": 0, "turns": 0}
        by_agent = {lr.agent: lr for lr in self.learners()}
        closed = {a: [] for a in by_agent}
        self.pool.queue([self.spec() for _ in range(self.cfg.games)])
        for _ in range(self.cfg.steps):
            obs = self.pool.observe()
            actions, logp, values = act(self.nets(), obs, self.device, self.rng)
            if self.critic is not None:
                values = self.critic_values(obs, values)
            for i, row in enumerate(rows(obs)):
                learner = by_agent.get(int(obs["agent"][i]))
                if learner is None:
                    continue
                learner.chains.setdefault((i, int(obs["seat"][i])), []).append(
                    [row, int(actions[i]), float(logp[i]), float(values[i]), 0.0, False]
                )
            for ep in self.pool.act(actions.tolist()):
                stats["episodes"] += 1
                stats["truncated"] += ep["truncated"]
                stats["turns"] += ep["turns"]
                if self.league:
                    self.league.record(ep, scores(ep)[0])
                r = rewards(ep, self.cfg.gw_weight)
                for s, name in enumerate(ep["seats"]):
                    learner = by_agent.get(int(name[4:])) if name.startswith("net:") else None
                    chain = learner and learner.chains.pop((ep["slot"], s), None)
                    if chain:
                        chain[-1][4], chain[-1][5] = float(r[s]), True
                        closed[learner.agent].append(chain)
        samples = {}
        for learner in self.learners():
            out = []
            for chain in closed[learner.agent]:
                out += self.advantages(chain, bootstrap=None)
            # Open chains: the last decision bootstraps the others and waits.
            for key, chain in list(learner.chains.items()):
                if len(chain) > 1:
                    out += self.advantages(chain[:-1], bootstrap=chain[-1][3])
                    learner.chains[key] = [chain[-1]]
            samples[learner.agent] = out
        return samples, stats

    @torch.no_grad()
    def critic_values(self, obs, values):
        """The learner's rows valued by the omniscient critic."""
        idx = np.nonzero(obs["agent"] == 0)[0]
        if len(idx):
            ids = torch.from_numpy(obs["critic_tok_ids"][idx]).long().to(self.device)
            f = torch.from_numpy(obs["critic_tok_f"][idx]).to(self.device)
            n = torch.from_numpy(obs["critic_tok_n"][idx]).long().to(self.device)
            values = values.copy()
            values[idx] = self.critic(ids, f, n).cpu().numpy()
        return values

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

    def minibatches(self, samples):
        """An epoch's minibatches, bucketed by length: a random order cut
        into chunks of 8 minibatches, each sorted by token count (the
        critic's when it reads them: the costlier encoder) and cut in
        minibatches, which are then shuffled — every sample once, little
        padding (tokens vary 20–80 a decision)."""
        mb = self.cfg.minibatch
        width = 4 if len(samples[0][0]) > 4 else 0
        size = np.array([s[0][width].shape[0] for s in samples])
        order = self.rng.permutation(len(samples))
        batches = []
        for k in range(0, len(order), 8 * mb):
            chunk = order[k : k + 8 * mb]
            chunk = chunk[np.argsort(size[chunk], kind="stable")]
            batches += [chunk[j : j + mb] for j in range(0, len(chunk), mb)]
        return [batches[i] for i in self.rng.permutation(len(batches))]

    def update(self, learner, samples):
        cfg = self.cfg
        if not samples:
            return {}
        adv = np.array([s[3] for s in samples], np.float32)
        adv = (adv - adv.mean()) / (adv.std() + 1e-8)
        losses = []
        for _ in range(cfg.epochs):
            for idx in self.minibatches(samples):
                batch = collate([samples[i][0] for i in idx], self.device)
                logits, values = forward(learner.model, batch)
                critic = self.critic is not None and learner.agent == 0
                if critic:
                    values = self.critic(*collate_critic([samples[i][0] for i in idx], self.device))
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
                loss = pg + cfg.value_coef * vf - self.entropy_coef * ent
                learner.opt.zero_grad()
                if critic:
                    self.critic_opt.zero_grad()
                if critic and self.warmup > 0:
                    loss = cfg.value_coef * vf
                loss.backward()
                torch.nn.utils.clip_grad_norm_(learner.model.parameters(), 1.0)
                if not (critic and self.warmup > 0):
                    learner.opt.step()
                if critic:
                    torch.nn.utils.clip_grad_norm_(self.critic.parameters(), 1.0)
                    self.critic_opt.step()
                losses.append((pg.item(), vf.item(), ent.item()))
        pg, vf, ent = np.mean(losses, 0)
        return {"pg": pg, "vf": vf, "entropy": ent}

    def evaluate(self, opponents, games, seed=0):
        """The learner's mean VPs and game-win rate over `games` games
        against `opponents`: every seat for the learner and every distinct
        order of the opponents around it, in turn (a fixed order would, at
        4 seats, always sit the same opponent across the table)."""
        tables = [{"seats": seats} for seats in eval_tables(opponents, games)]
        pool = zvtes_py.GamePool(
            min(games, 64),
            tables[0]["seats"],
            seed=10_000 + 1_000 * seed + self.cfg.seed,
            decks=self.decks,
            budget=self.cfg.budget,
            tables=tables,
        )
        rng = np.random.default_rng(seed)
        vps, wins, done, truncated = [], 0, 0, 0
        # The queued tables are games 0..games-1; later ones (the default
        # table, once the queue runs out) are not counted.
        while done < games:
            actions, _, _ = act(self.nets(), pool.observe(), self.device, rng)
            for ep in pool.act(actions.tolist()):
                if ep["game"] >= games:
                    continue
                done += 1
                truncated += ep["truncated"]
                v, w = scores(ep)
                for s, kind in enumerate(ep["seats"]):
                    if kind == "net:0":
                        vps.append(float(v[s]))
                        wins += bool(w[s])
        return {"vps": round(float(np.mean(vps)), 3), "gw": round(wins / games, 3), "truncated": truncated}

    def checkpoint(self, it):
        saved = {
            "model": self.model.state_dict(),
            "optimizer": self.learner.opt.state_dict(),
            "iteration": it,
            "config": dataclasses.asdict(self.cfg),
            "pfsp": self.league.state() if self.league else {},
            "best": self.best,
            "entropy_coef": self.entropy_coef,
            "vocab": self.vocab,
        }
        if self.critic is not None:
            saved["critic"] = self.critic.state_dict()
            saved["critic_optimizer"] = self.critic_opt.state_dict()
            saved["critic_warmup"] = self.warmup
        if self.exploiter:
            saved["exploiter"] = self.exploiter.model.state_dict()
            saved["exploiter_optimizer"] = self.exploiter.opt.state_dict()
        torch.save(saved, self.out / "model.pt")
        return saved

    def run(self):
        cfg = self.cfg
        log = open(self.out / "metrics.jsonl", "a")
        for it in range(self.start, cfg.iterations + 1):
            t0 = time.perf_counter()
            done = (it - self.start) / max(1, cfg.iterations - self.start)
            lr = cfg.lr * (1 - (1 - cfg.lr_final) * done)
            opts = [learner.opt for learner in self.learners()]
            if self.critic is not None:
                opts.append(self.critic_opt)
            for opt in opts:
                for group in opt.param_groups:
                    group["lr"] = lr
            for learner in self.learners():
                learner.model.eval()
            if self.critic is not None:
                self.critic.eval()
            samples, stats = self.rollout()
            t1 = time.perf_counter()
            metrics = {}
            for learner in self.learners():
                learner.model.train()
                m = self.update(learner, samples[learner.agent])
                prefix = "" if learner.agent == 0 else "exploiter_"
                metrics.update({prefix + k: v for k, v in m.items()})
                learner.model.eval()
            t2 = time.perf_counter()
            warming = self.warmup > 0
            self.warmup = max(0, self.warmup - 1)
            if cfg.entropy_target > 0 and "entropy" in metrics and not warming:
                up = metrics["entropy"] < cfg.entropy_target
                self.entropy_coef *= cfg.entropy_step if up else 1 / cfg.entropy_step
                self.entropy_coef = min(max(self.entropy_coef, 1e-3), 0.2)
            stats["turns"] = round(stats["turns"] / max(stats["episodes"], 1), 1)
            row = {
                "iteration": it,
                "samples": len(samples[0]),
                "rollout_s": round(t1 - t0, 2),
                "update_s": round(t2 - t1, 2),
                "lr": float(f"{lr:.3g}"),
                **({"critic_warmup": True} if warming else {}),
                "entropy_coef": float(f"{self.entropy_coef:.3g}"),
                **stats,
                **{k: round(float(v), 4) for k, v in metrics.items()},
            }
            if self.league:
                if self.league.step():
                    self.league.retire_exploiter(it, self.exploiter.model)
                    self.exploiter.restart(self.model.state_dict(), cfg.lr)
                    row["exploiter_retired"] = True
                if it % cfg.snapshot_every == 0:
                    self.league.add(it, self.model)
            if it % cfg.eval_every == 0 or it == cfg.iterations:
                row["vs_random"] = self.evaluate(["random"] * 4, cfg.eval_games)
                row["vs_bots4"] = self.evaluate(["aggro", "wall", "aggro"], cfg.eval_games)
                row["vs_bots5"] = self.evaluate(["aggro", "wall", "aggro", "wall"], cfg.eval_games)
                if self.league and self.league.latest() is not None:
                    snap = f"net:{self.league.latest()}"
                    row["vs_snapshot"] = self.evaluate([snap] * 4, cfg.eval_games)
                    row["pfsp"] = {label(k): round(v, 2) for k, v in sorted(self.league.p.items())}
                # `best.pt`: the bots' VPs at 4 and 5 seats, averaged —
                # a candidate within `best_margin` of the best is measured
                # again on `best_seeds` - 1 more evaluation seeds, and
                # kept on the mean (one evaluation's noise, ±0.15 VP, would
                # otherwise pick lucky checkpoints).
                score = (row["vs_bots4"]["vps"] + row["vs_bots5"]["vps"]) / 2
                better = False
                if score > self.best - cfg.best_margin:
                    scores = [score]
                    for k in range(1, cfg.best_seeds):
                        b4 = self.evaluate(["aggro", "wall", "aggro"], cfg.eval_games, seed=k)
                        b5 = self.evaluate(["aggro", "wall", "aggro", "wall"], cfg.eval_games, seed=k)
                        scores.append((b4["vps"] + b5["vps"]) / 2)
                    mean = float(np.mean(scores))
                    row["bots_mean"] = round(mean, 3)
                    better = mean > self.best
                    if better:
                        self.best = mean
                        row["best"] = True
                saved = self.checkpoint(it)
                if better:
                    torch.save(saved, self.out / "best.pt")
            print(json.dumps(row), flush=True)
            log.write(json.dumps(row) + "\n")
            log.flush()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for f in dataclasses.fields(Config):
        kind = type(f.default)
        if kind is bool:
            kind = lambda s: s.lower() in ("1", "true", "yes")  # noqa: E731
        parser.add_argument("--" + f.name.replace("_", "-"), type=kind, default=f.default)
    args = parser.parse_args()
    Trainer(Config(**vars(args))).run()


if __name__ == "__main__":
    main()
