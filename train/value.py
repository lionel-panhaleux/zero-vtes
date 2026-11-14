"""A turn-level value trained offline (DESIGN §11.2, the training board's
"A turn-level value"): a `Critic` fitted, supervised, to the seat's
result — PPO's reward, (VPs + `gw_weight` × win) / 5 — on decisions sampled
from a checkpoint's self-play (every seat its policy, at the settings it
trained on, `train.position.play`), then measured as the critic is: its
explained variance by stage on held-out games and on the human records,
beside the checkpoint's own critic on the same decisions.

  uv run python -m train.value gen runs/served-p18.pt runs/value/train --games 20000 --keep 0.03 --seed 1
  uv run python -m train.value gen runs/served-p18.pt runs/value/held --games 1000 --keep 0.05 --seed 2
  uv run python -m train.value fit runs/served-p18.pt runs/value/train runs/value/held runs/value/v.pt
  uv run python -m train.value measure runs/served-p18.pt runs/value/v.pt runs/value/held --records data/records,…
"""

import argparse
import copy
import glob
import json
import os

import numpy as np
import torch
import zvtes_py

from train.assess import load
from train.model import Critic, remap_vocab
from train.position import STAGES, from_records, play, played_out, r2, record_paths, stage, table
from train.ppo import C_TOK, Config, collate_critic, device_of, rewards


def gen(policy, critic, cfg, games, seed, keep, device, parallel=256, budget=20_000):
    """Per game of `games` self-play games played out, as it ends, its
    decisions kept (a `keep` share): dicts of `row` (the critic's inputs),
    `seat`, `turn` (the position's turn feature), `critic` (the checkpoint
    critic's value), `target` (the seat's result), `game`, `turns`."""
    gw = cfg.get("gw_weight", Config.gw_weight)
    turn = zvtes_py.position_names().index("turn")
    for g, ep, rows in play(policy, critic, cfg, games, 0, seed, device, budget, parallel, keep):
        if not played_out(ep):
            continue
        r = rewards(ep, gw)
        yield [
            {"row": x, "seat": s, "turn": float(f[turn]), "critic": v, "target": float(r[s]), "game": g, "turns": ep["turns"]}
            for s, f, v, x in rows
        ]


def stage_of(x):
    return stage(x["turn"], x["turns"])


def batches(rows, size, rng):
    """Minibatches of similar token counts, in a random order."""
    order = sorted(range(len(rows)), key=lambda i: len(rows[i]["row"][C_TOK]))
    chunks = [order[i : i + size] for i in range(0, len(order), size)]
    rng.shuffle(chunks)
    return chunks


@torch.no_grad()
def predict(net, rows, device, size=256):
    net.eval()
    out = np.zeros(len(rows), np.float32)
    for chunk in batches(rows, size, np.random.default_rng(0)):
        out[chunk] = net(*collate_critic([rows[i]["row"] for i in chunk], device)).float().cpu().numpy()
    return out


def fit(net, shards, held, device, epochs=10, lr=1e-4, size=256, seed=0, log=print):
    """`net` fitted to the targets of `shards` (each a loader of rows, read
    one at a time: the rows of every game do not fit in memory) by mean
    squared error and AdamW, the weights of its best epoch on `held` kept
    — its starting ones if none beats them (a critic it starts from is
    never made worse)."""
    rng = np.random.default_rng(seed)
    net.to(device)
    opt = torch.optim.AdamW(net.parameters(), lr=lr, weight_decay=1e-4)
    y_held = np.array([x["target"] for x in held], np.float32)
    best = float(((predict(net, held, device) - y_held) ** 2).mean())
    best_state = copy.deepcopy(net.state_dict())
    log(json.dumps({"epoch": -1, "held_mse": round(best, 4)}))
    for epoch in range(epochs):
        net.train()
        loss_sum, n = 0.0, 0
        for k in rng.permutation(len(shards)):
            train = shards[k]()
            for chunk in batches(train, size, rng):
                y = torch.tensor([train[i]["target"] for i in chunk], device=device)
                v = net(*collate_critic([train[i]["row"] for i in chunk], device))
                loss = ((v - y) ** 2).mean()
                opt.zero_grad()
                loss.backward()
                torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0)
                opt.step()
                loss_sum, n = loss_sum + float(loss.detach()) * len(chunk), n + len(chunk)
        mse = float(((predict(net, held, device) - y_held) ** 2).mean())
        log(json.dumps({"epoch": epoch, "train_mse": round(loss_sum / max(n, 1), 4), "held_mse": round(mse, 4)}))
        if mse < best:
            best, best_state = mse, copy.deepcopy(net.state_dict())
    net.load_state_dict(best_state)
    return net.eval()


def by_stage(y, preds, stages):
    """Per stage (`train.position.STAGES`), the explained variance of each
    of `preds` ({name: values})."""
    out = []
    for lo, hi in STAGES:
        m = (stages >= lo) & (stages < hi)
        if m.sum() < 20:
            continue
        out.append({"stage": f"{lo:.2f}-{min(hi, 1):.2f}", "n": int(m.sum())} | {k: round(r2(y[m], p[m]), 3) for k, p in preds.items()})
    return out


def measure(net, critic, cfg, held, records, device):
    """The value (`value`) beside the checkpoint's critic (`critic`), by
    stage, on the held-out self-play decisions and on the human records."""
    y = np.array([x["target"] for x in held], np.float64)
    report = {
        "self-play (held out)": by_stage(
            y,
            {"critic": np.array([x["critic"] for x in held]), "value": predict(net, held, device).astype(np.float64)},
            np.array([stage_of(x) for x in held]),
        )
    }
    if records:
        cols = {}
        for name, model in (("critic", critic), ("value", net)):
            _, cols[name], y, stages, _ = table(*from_records(model.cpu(), cfg, records))
        report["human records"] = by_stage(y, cols, stages)
    return report


def save_rows(rows, path):
    os.makedirs(path, exist_ok=True)
    torch.save(rows, os.path.join(path, f"shard-{len(glob.glob(os.path.join(path, 'shard-*.pt'))):03d}.pt"))


def shard_loaders(path):
    return [lambda f=f: torch.load(f, weights_only=False) for f in sorted(glob.glob(os.path.join(path, "shard-*.pt")))]


def load_rows(path):
    return [x for load_shard in shard_loaders(path) for x in load_shard()]


def load_value(path):
    saved = torch.load(path, map_location="cpu")
    net = Critic(json.loads(zvtes_py.schema()), **saved["size"])
    net.load_state_dict(remap_vocab(saved["value"], saved["vocab"], zvtes_py.vocab_keys(), net.state_dict()))
    return net.eval()


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    g = sub.add_parser("gen")
    g.add_argument("checkpoint")
    g.add_argument("out")
    g.add_argument("--games", type=int, default=1000)
    g.add_argument("--shard", type=int, default=500, help="games a shard")
    g.add_argument("--keep", type=float, default=0.1, help="share of the decisions kept")
    g.add_argument("--seed", type=int, default=1)
    g.add_argument("--parallel", type=int, default=256)
    f = sub.add_parser("fit")
    f.add_argument("checkpoint")
    f.add_argument("train")
    f.add_argument("held")
    f.add_argument("out")
    f.add_argument("--init", choices=("critic", "fresh"), default="critic", help="from the checkpoint's critic, or new")
    f.add_argument("--d", type=int, default=0, help="a fresh network's width (0: the checkpoint's)")
    f.add_argument("--layers", type=int, default=0)
    f.add_argument("--epochs", type=int, default=10)
    f.add_argument("--lr", type=float, default=1e-4)
    m = sub.add_parser("measure")
    m.add_argument("checkpoint")
    m.add_argument("value")
    m.add_argument("held")
    m.add_argument("--records", default="")
    m.add_argument("--out", default="")
    for p in (g, f, m):
        p.add_argument("--device", default="auto")
    a = ap.parse_args()
    device = device_of(a.device)
    policy, critic, cfg, _ = load(a.checkpoint)
    if critic is None:
        raise SystemExit("the checkpoint has no critic")
    # A critic trained with shaping learns the result less coef × Φ: read
    # as a prediction of the result, it would be beaten for nothing.
    if torch.load(a.checkpoint, map_location="cpu").get("shaping"):
        raise SystemExit("the checkpoint trained with shaping: its critic does not predict the result")
    if a.cmd == "gen":
        # Shards are numbered after those already there: another run's
        # would be read as this one's.
        if shard_loaders(a.out):
            raise SystemExit(f"{a.out} already holds shards")
        shard, n = [], 0
        for rows in gen(policy, critic, cfg, a.games, a.seed, a.keep, device, a.parallel):
            shard += rows
            n += 1
            if n % a.shard == 0:
                save_rows(shard, a.out)
                shard = []
                print(json.dumps({"games": n}), flush=True)
        if shard:
            save_rows(shard, a.out)
        print(json.dumps({"games": n, "done": True}), flush=True)
    elif a.cmd == "fit":
        held = load_rows(a.held)
        if a.init == "critic":
            net = critic
        else:
            schema = json.loads(zvtes_py.schema())
            net = Critic(schema, d=a.d or cfg.get("d", 96), layers=a.layers or cfg.get("layers", 2))
        net = fit(net, shard_loaders(a.train), held, device, a.epochs, a.lr, log=lambda s: print(s, flush=True))
        size = {"d": net.tok_f.out_features, "layers": len(net.trunk.layers)}
        torch.save({"value": net.cpu().state_dict(), "size": size, "vocab": zvtes_py.vocab_keys(), "checkpoint": a.checkpoint}, a.out)
    else:
        net = load_value(a.value)
        report = {"checkpoint": a.checkpoint, "value": a.value}
        report |= measure(net.to(device), critic.to(device), cfg, load_rows(a.held), record_paths(a.records), device)
        print(json.dumps(report, indent=1))
        if a.out:
            json.dump(report, open(a.out, "w"), indent=1)


if __name__ == "__main__":
    main()
