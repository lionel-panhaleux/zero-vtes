"""What a network makes of a human game (DESIGN §11.2, human games as data:
step 1, the diagnostic). The game is a replay exported from the Succubus
Club fork; every decision is replayed and encoded as the network reads it.

`uv run python -m train.assess <replay.json> [runs/<name>/best.pt] [--top N]`

- The human's choices the policy found least likely (where its play and a
  human's part), with the option it preferred; agreement by window.
- The bots' sharpest falls in the critic's value (its expected
  (VPs + `gw_weight` × win) / 5 for the deciding seat, at that seat's
  decisions), with what happened in between: where a bot lost the game.
- `*` marks a card no deck of the checkpoint's training pool holds (its
  embedding never trained).
"""

import argparse
import json
from collections import defaultdict

import numpy as np
import torch
import zvtes_py

from train.model import Critic, Policy, remap_vocab
from train.ppo import collate, collate_critic


def load(path):
    """The checkpoint's policy and critic (None without one), its config,
    and the KRCG ids of its training decks' cards."""
    saved = torch.load(path, map_location="cpu")
    cfg = saved.get("config", {})
    schema, vocab = json.loads(zvtes_py.schema()), zvtes_py.vocab_keys()
    old = saved.get("vocab", vocab)
    size = dict(d=cfg.get("d", 96), layers=cfg.get("layers", 2))
    policy = Policy(schema, **size)
    policy.load_state_dict(remap_vocab(saved["model"], old, vocab, policy.state_dict()))
    critic = None
    if "critic" in saved:
        critic = Critic(schema, **size)
        critic.load_state_dict(remap_vocab(saved["critic"], old, vocab, critic.state_dict()))
        critic.eval()
    families = dict(zvtes_py.deck_families())
    decks = [d for f in cfg.get("decks", "precons").split(",") for d in families[f.strip()]]
    trained = {c for d in decks for c, _ in zvtes_py.deck_cards(d)}
    return policy.eval(), critic, cfg | {"iteration": saved.get("iteration", "?")}, trained


@torch.no_grad()
def evaluate(policy, critic, decisions, batch=64):
    """Each decision's option probabilities and the critic's value."""
    rows = [
        (d["tok_ids"], d["tok_f"], d["opt_ids"], d["opt_f"], d["critic_tok_ids"], d["critic_tok_f"])
        for d in decisions
    ]
    probs, values = [], []
    for i in range(0, len(rows), batch):
        part = rows[i : i + batch]
        b = collate(part, "cpu")
        logits, v = policy(
            b["tok_ids"], b["tok_f"], b["tok_n"], b["opt_ids"], b["opt_f"], b["opt_n"]
        )
        p = torch.softmax(logits, -1)
        probs += [p[k, : int(b["opt_n"][k])].numpy() for k in range(len(part))]
        if critic is not None:
            v = critic(*collate_critic(part, "cpu"))
        values += v.tolist()
    return probs, values


def mark(card, trained):
    return "*" if card is not None and card not in trained else ""


def human_report(game, probs, trained, top):
    names = game["names"]
    for seat in game["humans"]:
        mine = [
            (d, p)
            for d, p in zip(game["decisions"], probs)
            if d["seat"] == seat and d["chosen"] is not None and len(d["options"]) > 1
        ]
        if not mine:
            continue
        agree = [int(np.argmax(p)) == d["chosen"] for d, p in mine]
        nll = -np.mean([np.log(max(p[d["chosen"]], 1e-9)) for d, p in mine])
        print(f"\n== {names[seat]}: {len(mine)} choices with an alternative")
        print(f"the policy's favourite was the human's {np.mean(agree):.0%} of the time;")
        print(f"mean -log p(human's choice) {nll:.2f}")
        by_window = defaultdict(list)
        for (d, _), a in zip(mine, agree):
            by_window[d["window"]].append(a)
        print("\nagreement by window (choices):")
        for w, a in sorted(by_window.items(), key=lambda kv: -len(kv[1])):
            print(f"  {np.mean(a):4.0%}  {len(a):3}  {w}")
        print(f"\nthe {top} human choices the policy found least likely:")
        surprises = sorted(
            (x for x, a in zip(mine, agree) if not a), key=lambda x: x[1][x[0]["chosen"]]
        )
        for d, p in surprises[:top]:
            c, best = d["chosen"], int(np.argmax(p))
            print(f"#{d['decision']} turn {d['turn']}, {d['window']} ({len(p)} options)")
            print(f"    human  {p[c]:5.1%}  {d['options'][c]}{mark(d['cards'][c], trained)}")
            print(f"    policy {p[best]:5.1%}  {d['options'][best]}{mark(d['cards'][best], trained)}")


def bot_report(game, values, top):
    names, decisions = game["names"], game["decisions"]
    bots = [s for s in range(len(names)) if s not in game["humans"]]
    at = defaultdict(list)  # seat -> its decisions' indices
    for i, d in enumerate(decisions):
        at[d["seat"]].append(i)
    print("\n== The critic's value per seat (start → lowest → last decision)")
    for s in range(len(names)):
        v = [values[i] for i in at[s]]
        if v:
            print(f"  {names[s]}: {v[0]:+.2f} → {min(v):+.2f} → {v[-1]:+.2f}")
    falls = []
    for s in bots:
        idx = at[s]
        for i, j in zip(idx, idx[1:]):
            falls.append((values[j] - values[i], s, i, j))
    falls.sort()
    print(f"\n== The bots' {top} sharpest falls in value (between two of its decisions)")
    for fall, s, i, j in falls[:top]:
        a, b = decisions[i], decisions[j]
        print(
            f"\n{names[s]}: {values[i]:+.2f} → {values[j]:+.2f} ({fall:+.2f}), "
            f"#{a['decision']} turn {a['turn']} → #{b['decision']} turn {b['turn']}"
        )
        lines = [f"  #{a['decision']} {names[s]} ({a['window']}): {a.get('choice')}"]
        for k in range(i + 1, j + 1):
            d = decisions[k]
            lines += [f"      · {e}" for e in d["before"]]
            if k < j:
                lines.append(f"  #{d['decision']} {names[d['seat']]} ({d['window']}): {d.get('choice')}")
        shown = 30
        print("\n".join(lines[:shown]))
        if len(lines) > shown:
            print(f"  … {len(lines) - shown} more lines")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("replay")
    parser.add_argument("checkpoint", nargs="?", default="runs/p5-self/best.pt")
    parser.add_argument("--top", type=int, default=15)
    args = parser.parse_args()
    policy, critic, cfg, trained = load(args.checkpoint)
    obs = cfg.get("obs", 1)
    with open(args.replay) as f:
        game = zvtes_py.replay_decisions(f.read(), obs)
    decisions = game["decisions"]
    if decisions and decisions[-1]["chosen"] is None:
        decisions.pop()  # the log ends on an unanswered decision
    probs, values = evaluate(policy, critic, decisions)
    over = game.get("over", {}).get("vps")
    print(f"{args.replay}: {len(decisions)} decisions, VPs {over or 'unfinished'}")
    print(
        f"{args.checkpoint} (iteration {cfg['iteration']}, "
        f"obs V{obs}, {'critic' if critic is not None else 'no critic: the policy value'})"
    )
    human_report(game, probs, trained, args.top)
    bot_report(game, values, args.top)


if __name__ == "__main__":
    main()
