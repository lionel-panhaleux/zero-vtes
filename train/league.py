"""The league (DESIGN §5, P4). Every table is 4 or 5 seats (tournament
tables) with `learner_seats` seats for the learner (`net:0`, 1 by
default; on `partner_rate` of the tables a second one across the table
from the first: 2 seats away, or 3 at 5 seats) and, with `exploiter`, one for the exploiter (`net:EXPLOITER`),
a second trained network that plays only the learner's tables; the other
seats are drawn by prioritized fictitious self-play among the scripted
bots (`bots`), the learner's last `opponents_kept` snapshots
(`net:<iteration>`) and the exploiter's last retired copies
(`net:EXPLOITER + <iteration>`): weight (1 − p)² + 0.02, p
the learner's beat rate against it (pairwise: more VPs than that seat, a
tie ½), within the bots (probability `bot_seats`, at most `table_bots` a
table) or the snapshots.
Beat rates move once per iteration, by `pfsp_rate` toward the
iteration's mean. The random seat retires once the learner beats it over
`retire_random` of the time. The exploiter is retired into the pool and
restarted from the learner's weights once the learner's beat rate against
it falls under `exploiter_beaten`, or after `exploiter_reset` iterations.

With `opponents` (checkpoint paths, comma-separated), every other seat is
one of those frozen networks (`net:FROZEN + k`, by PFSP among them): no
bots, no snapshots (owner, 2026-10-03: only good opponents, the previous
bests). They read the encoding they were trained on (`frozen_versions`).
With `habit_seats`, that share of those seats plays with the punishing
habits (`net:k+habits`, `zvtes_env::habits`: rescues, burns others' Ⓓ-burnable
cards, diablerizes, keeps its last minion home); their beat rates are kept
apart."""

import copy
import json
from pathlib import Path

import numpy as np
import torch

from train.model import remap_vocab

BOTS = ["aggro", "wall", "random"]
TABLES = (4, 5)
# Network ids: 0 the learner, iterations its snapshots, EXPLOITER the live
# exploiter and EXPLOITER + iteration its retired copies.
EXPLOITER = 30_000
EXP = f"net:{EXPLOITER}"
# The frozen opponents: FROZEN + their index in `opponents`.
FROZEN = 20_000


HABITS = "+habits"


def net_id(name):
    """A network seat's id (`net:k`, `net:k+habits`), else None."""
    if not name.startswith("net:"):
        return None
    return int(name[4:].removesuffix(HABITS))


def label(name, frozen=()):
    """A seat name for logs: `exploiter`, `exploiter:<it>`, `net:<it>`, a
    frozen opponent by its run and iteration (`+habits` kept)."""
    k = net_id(name)
    habits = HABITS if name.endswith(HABITS) else ""
    if k is not None and FROZEN <= k < EXPLOITER:
        k -= FROZEN
        return (frozen[k] if k < len(frozen) else f"frozen:{k}") + habits
    if k is not None and k >= EXPLOITER:
        k -= EXPLOITER
        return f"exploiter:{k}" if k else "exploiter"
    return name


class League:
    def __init__(self, cfg, rng, out):
        if not set(cfg.bots.split(",")) <= set(BOTS):
            raise ValueError(f"bots: {', '.join(BOTS)} (got {cfg.bots!r})")
        self.cfg = cfg
        self.rng = rng
        self.dir = Path(out) / "snapshots"
        self.dir.mkdir(parents=True, exist_ok=True)
        self.snapshots = {}  # net id -> frozen model
        self.p = {}  # opponent name -> the learner's beat rate
        self.pending = {}  # opponent name -> this iteration's outcomes
        self.exploiter_since = 0
        self.frozen = {}  # net id -> frozen opponent (`opponents`)
        self.frozen_labels = []
        self.frozen_versions = {}  # net id -> the encoding it reads

    def load_opponents(self, make, vocab, device):
        """The `opponents` checkpoints, each into a network `make(config)`
        builds (their sizes may differ), its embeddings remapped to `vocab`."""
        paths = [p.strip() for p in self.cfg.opponents.split(",") if p.strip()]
        for k, path in enumerate(paths):
            saved = torch.load(path, map_location=device)
            net = make(saved.get("config", {}))
            old = saved.get("vocab", vocab)
            net.load_state_dict(remap_vocab(saved["model"], old, vocab, net.state_dict()))
            self.frozen[FROZEN + k] = net.eval()
            self.frozen_versions[FROZEN + k] = saved.get("config", {}).get("obs", 1)
            self.frozen_labels.append(f"{Path(path).parent.name}@{saved.get('iteration', '?')}")

    def load(self, model, vocab):
        """The snapshots saved by an earlier run, into copies of `model`
        (their embeddings remapped if the vocabulary changed since)."""
        path = self.dir / "vocab.json"
        old = json.loads(path.read_text()) if path.exists() else vocab
        for path in sorted(self.dir.glob("*.pt"), key=lambda p: int(p.stem)):
            net = copy.deepcopy(model)
            state = torch.load(path, map_location=next(model.parameters()).device)
            net.load_state_dict(remap_vocab(state, old, vocab, model.state_dict()))
            self.snapshots[int(path.stem)] = net.eval()
            if old != vocab:
                torch.save(net.state_dict(), path)
        (self.dir / "vocab.json").write_text(json.dumps(vocab))
        self.trim()

    def add(self, key, model):
        net = copy.deepcopy(model).eval()
        torch.save(net.state_dict(), self.dir / f"{key}.pt")
        self.snapshots[key] = net
        self.trim()

    def learner_snapshots(self):
        return sorted(k for k in self.snapshots if k < EXPLOITER)

    def exploiter_snapshots(self):
        return sorted(k for k in self.snapshots if k > EXPLOITER)

    def trim(self):
        # Kept in memory three times longer than they are drawn: a game
        # started against a snapshot outlives its turn as a candidate.
        keep = 3 * self.cfg.opponents_kept
        for k in self.learner_snapshots()[:-keep] + self.exploiter_snapshots()[:-keep]:
            del self.snapshots[k]

    def candidates(self):
        if self.frozen:
            return [f"net:{k}" for k in sorted(self.frozen)]
        n = self.cfg.opponents_kept
        bots = [
            b
            for b in self.cfg.bots.split(",")
            if not (b == "random" and self.p.get(b, 0.5) > self.cfg.retire_random)
        ]
        return (
            bots
            + [f"net:{k}" for k in self.learner_snapshots()[-n:]]
            + [f"net:{k}" for k in self.exploiter_snapshots()[-n:]]
        )

    def draw(self, bots_ok=True):
        """An opponent: a bot with probability `bot_seats` (else a
        snapshot, when there is one; always one once `bots_ok` is off),
        then by PFSP within the group."""
        names = self.candidates()
        bots = [n for n in names if n in BOTS]
        nets = [n for n in names if n not in BOTS]
        if nets and (not bots_ok or self.rng.random() >= self.cfg.bot_seats):
            names = nets
        elif bots:
            names = bots
        w = [(1 - self.p.get(n, 0.5)) ** 2 + 0.02 for n in names]
        name = names[self.rng.choice(len(names), p=[x / sum(w) for x in w])]
        if self.frozen and self.rng.random() < self.cfg.habit_seats:
            name += HABITS
        return name

    def spec(self, n=None, at=None):
        """A table: `n` seats (4 or 5 drawn when None), the learner at seat
        `at` (drawn when None)."""
        n = int(self.rng.choice(TABLES)) if n is None else n
        seats = []
        for _ in range(n):
            bots_ok = sum(x in BOTS for x in seats) < self.cfg.table_bots
            seats.append(self.draw(bots_ok))
        order = self.rng.permutation(n)
        if at is not None:
            order = np.concatenate([[at], order[order != at]])
        k = self.cfg.learner_seats
        if self.cfg.partner_rate > 0 and self.rng.random() < self.cfg.partner_rate:
            mate = (order[0] + int(self.rng.choice(range(2, n - 1)))) % n
            order = np.concatenate([[order[0], mate], order[(order != order[0]) & (order != mate)]])
            k = max(k, 2)
        for i in order[:k]:
            seats[i] = "net:0"
        if self.cfg.exploiter:
            seats[order[k]] = EXP
        return {"seats": seats}

    def record(self, ep, vps):
        """Collect the learner's pairwise outcomes of a finished game
        (truncated ones are skipped: a looping seat says nothing about
        strength)."""
        if ep["truncated"]:
            return
        seats = ep["seats"]
        for s, me in enumerate(seats):
            if me != "net:0":
                continue
            for o, them in enumerate(seats):
                if them == "net:0":
                    continue
                outcome = 1.0 if vps[s] > vps[o] else 0.5 if vps[s] == vps[o] else 0.0
                self.pending.setdefault(them, []).append(outcome)

    def step(self):
        """End of an iteration: move each beat rate toward the iteration's
        mean; whether the exploiter is due for a restart."""
        for name, outcomes in self.pending.items():
            p = self.p.get(name, 0.5)
            self.p[name] = p + self.cfg.pfsp_rate * (float(np.mean(outcomes)) - p)
        self.pending = {}
        self.exploiter_since += 1
        return self.cfg.exploiter and (
            self.p.get(EXP, 0.5) < self.cfg.exploiter_beaten
            or self.exploiter_since >= self.cfg.exploiter_reset
        )

    def retire_exploiter(self, iteration, exploiter):
        """The exploiter joins the pool as a frozen copy (its beat rate
        carried over); the live one restarts at ½."""
        name = f"net:{EXPLOITER + iteration}"
        self.add(EXPLOITER + iteration, exploiter)
        self.p[name] = self.p.pop(EXP, 0.5)
        self.exploiter_since = 0

    def nets(self, learner, exploiter=None):
        nets = {0: learner, **self.snapshots, **self.frozen}
        if exploiter is not None:
            nets[EXPLOITER] = exploiter
        return nets

    def latest(self):
        snaps = self.learner_snapshots()
        return snaps[-1] if snaps else None

    def state(self):
        return {"p": self.p, "exploiter_since": self.exploiter_since}

    def restore(self, state):
        if "p" in state:  # older checkpoints saved the beat rates alone
            self.p, self.exploiter_since = state["p"], state["exploiter_since"]
        else:
            self.p = state
