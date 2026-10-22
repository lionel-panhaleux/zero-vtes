"""The critic against the position (training board, "The critic against
the position, measured"; DESIGN §11.2): how well the omniscient critic
predicts a seat's result — (VPs + `gw_weight` × win) / 5, the reward it
learns — at a quarter, half and three quarters of the game, against a
linear fit of hand-made position features (`zvtes_py.position_names`:
the pools of the seat, its prey and predator, the minions each side has
ready, the lunge) and against both together. Fitted on self-play games
of a checkpoint (every seat its policy), scored on held-out ones and on
the human records.

`uv run python -m train.position runs/<name>/best.pt [--games 512]
[--records data/records,…] [--out report.json]`
"""

import argparse
import glob
import json
import os

import numpy as np
import torch
import zvtes_py

from train.assess import load
from train.ppo import act, collate_critic, device_of, rewards, shaped

STAGES = ((0.0, 0.25), (0.25, 0.5), (0.5, 0.75), (0.75, 1.01))
CRITIC_KEYS = [f"critic_{a}_{b}" for a in ("tok", "deck") for b in ("ids", "f", "n")]


@torch.no_grad()
def self_play(policy, critic, cfg, games, seats, seed, device, budget=20_000):
    """Every decision of `games` self-play games: (game, seat, features,
    critic value); per game its turns and per-seat targets; the games
    dropped (a panic, or truncated at `budget` decisions)."""
    families = dict(zvtes_py.deck_families())
    decks = [d for f in cfg.get("decks", "precons").split(",") for d in families[f.strip()]]
    tables = [{"seats": ["net:0"] * (seats or 4 + g % 2)} for g in range(games)]
    pool = zvtes_py.GamePool(
        games,
        ["net:0"] * (seats or 4),
        seed=seed,
        decks=decks,
        tables=tables,
        budget=budget,
        critic=True,
        obs=cfg.get("obs", 1),
    )
    # Refilled, so a row stays its slot's (`observe` reads every game);
    # the games past the queued ones are not counted.
    policy, critic = policy.to(device), critic.to(device)
    rng = np.random.default_rng(seed)
    open_rows = {}
    out, over, done, dropped = [], {}, 0, 0
    while done < games:
        obs = pool.observe()
        feats = pool.positions()
        actions, _, _ = act({0: policy}, obs, device, rng)
        idx = np.arange(len(obs["tok_n"]))
        part = shaped({k: obs[k] for k in CRITIC_KEYS}, idx)
        args = [torch.from_numpy(part[k]).to(device) for k in CRITIC_KEYS]
        args = [a.long() if not k.endswith("_f") else a for k, a in zip(CRITIC_KEYS, args)]
        values = critic(*args).float().cpu().numpy()[: len(idx)]
        for i in idx:
            open_rows.setdefault(i, []).append((int(obs["seat"][i]), feats[i], float(values[i])))
        for ep in pool.act(actions.tolist()):
            rows = open_rows.pop(ep["slot"], [])
            if ep["game"] >= games:
                continue
            done += 1
            # A panic or a truncated game has no result to predict.
            if "panic" in ep or ep["truncated"]:
                dropped += 1
                continue
            g = ep["game"]
            over[g] = (ep["turns"], rewards(ep, cfg.get("gw_weight", 5)))
            out += [(g, s, f, v) for s, f, v in rows]
    return out, over, dropped


def from_records(critic, cfg, paths):
    """The records' decisions read whole: (game, seat, features, critic
    value), and per game its turns and per-seat targets."""
    out, over = [], {}
    for g, path in enumerate(paths):
        try:
            game = zvtes_py.replay_decisions(open(path).read(), cfg.get("obs", 1))
        except ValueError:
            continue
        if "over" not in game:
            continue
        vps = np.array(game["over"]["vps"], np.float32)
        ep = {"half_vps": (vps * 2).astype(int).tolist()}
        decisions = game["decisions"]
        turns = max(d["turn"] for d in decisions)
        over[g] = (turns, rewards(ep, cfg.get("gw_weight", 5)))
        values = critic_values(critic, decisions)
        out += [(g, d["seat"], np.array(d["position"], np.float32), v) for d, v in zip(decisions, values)]
    return out, over


@torch.no_grad()
def critic_values(critic, decisions, batch=64):
    keys = ("critic_tok_ids", "critic_tok_f", "critic_deck_ids", "critic_deck_f")
    rows = [(None,) * 6 + tuple(d[k] for k in keys) for d in decisions]
    values = []
    for i in range(0, len(rows), batch):
        values += critic(*collate_critic(rows[i : i + batch], "cpu")).tolist()
    return values


def table(samples, over):
    """Arrays: features X, critic values v, targets y, stages (the
    decision's turn over the game's), games."""
    names = zvtes_py.position_names()
    turn = names.index("turn")
    X = np.array([f for _, _, f, _ in samples], np.float64)
    v = np.array([c for _, _, _, c in samples], np.float64)
    y = np.array([over[g][1][s] for g, s, _, _ in samples], np.float64)
    stage = np.array([f[turn] * 60 / max(over[g][0], 1) for g, _, f, _ in samples])
    games = np.array([g for g, _, _, _ in samples])
    return X, v, y, stage, games


def fit(X, y, ridge=1e-3):
    """Least squares with an intercept and a small ridge: the weights,
    intercept last."""
    A = np.hstack([X, np.ones((len(X), 1))])
    reg = ridge * len(X) * np.eye(A.shape[1])
    reg[-1, -1] = 0
    return np.linalg.solve(A.T @ A + reg, A.T @ y)


def predict(w, X):
    return np.hstack([X, np.ones((len(X), 1))]) @ w


def r2(y, p):
    """Explained variance of `p` on `y` (1 − SSE / SST)."""
    sst = ((y - y.mean()) ** 2).sum()
    return float(1 - ((y - p) ** 2).sum() / sst) if sst > 0 else float("nan")


def measure(train, tests):
    """Per stage, fit on `train` (self-play) and score on each of `tests`:
    the explained variance of the critic as it is, of the features' fit
    and of both together (the critic's value as one more feature); and
    the features' weights scaled by their spread (a weight per standard
    deviation of the feature)."""
    names = zvtes_py.position_names()
    report = {"weights": {}}
    fits = {}
    X, v, y, stage, _ = train
    for lo, hi in STAGES:
        m = (stage >= lo) & (stage < hi)
        key = f"{lo:.2f}-{min(hi, 1):.2f}"
        w_f = fit(X[m], y[m])
        w_b = fit(np.hstack([X[m], v[m, None]]), y[m])
        fits[key] = (lo, hi, w_f, w_b)
        sd = X[m].std(0)
        report["weights"][key] = dict(zip(names, (w_f[:-1] * sd).round(3).tolist()))
    for name, (X, v, y, stage, _) in tests.items():
        rows = []
        for key, (lo, hi, w_f, w_b) in fits.items():
            m = (stage >= lo) & (stage < hi)
            if m.sum() < 20:
                continue
            rows.append(
                {
                    "stage": key,
                    "n": int(m.sum()),
                    "critic": round(r2(y[m], v[m]), 3),
                    "features": round(r2(y[m], predict(w_f, X[m])), 3),
                    "both": round(r2(y[m], predict(w_b, np.hstack([X[m], v[m, None]]))), 3),
                }
            )
        report[name] = rows
    return report


def split(t, held):
    """`t` (table's arrays) cut by game: the games in `held`, the others."""
    m = np.isin(t[4], list(held))
    return tuple(a[~m] for a in t), tuple(a[m] for a in t)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("checkpoint")
    ap.add_argument("--games", type=int, default=512)
    ap.add_argument("--seats", type=int, default=0, help="4 or 5; 0 alternates")
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--records", default="")
    ap.add_argument("--device", default="auto")
    ap.add_argument("--budget", type=int, default=20_000, help="decisions before a game is truncated")
    ap.add_argument("--out", default="")
    a = ap.parse_args()
    policy, critic, cfg, _ = load(a.checkpoint)
    if critic is None:
        raise SystemExit("the checkpoint has no critic")
    samples, over, dropped = self_play(policy, critic, cfg, a.games, a.seats, a.seed, device_of(a.device), a.budget)
    t = table(samples, over)
    games = sorted(over)
    held = set(games[: len(games) * 3 // 10])
    train, test = split(t, held)
    tests = {"self-play (held out)": test}
    if a.records:
        paths = sorted(p for spec in a.records.split(",") for p in (glob.glob(os.path.join(spec, "*.json")) if os.path.isdir(spec) else [spec]))
        rs, ro = from_records(critic.cpu(), cfg, paths)
        if rs:
            tests["human records"] = table(rs, ro)
    report = {"checkpoint": a.checkpoint, "games": len(over), "dropped": dropped, "decisions": len(samples)}
    report |= measure(train, tests)
    print(json.dumps(report, indent=1))
    if a.out:
        json.dump(report, open(a.out, "w"), indent=1)


if __name__ == "__main__":
    main()
