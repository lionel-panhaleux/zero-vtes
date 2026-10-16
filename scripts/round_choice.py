#!/usr/bin/env python3
"""Choose a round of cards (add-cards §1): what adding cards to the engine
pool does to the TWDA corpus, read through gen_twda.py's own rules
(`select`: the decks in the pool, the versions cap), the generated tables
extended in memory, nothing written.

Printed, for the owner to pick the cut:
- the ceiling: every recent (2025+) deck's missing cards added at once,
  since more cards can displace kept decks (VERSIONS, SAME_CRYPT,
  SAME_LIBRARY);
- the greedy steps: from the recent decks missing at most `--bundle` cards,
  the bundle adding the most recent kept decks per new card, step after
  step: the cards (by id), kept, recent kept, the cost per recent deck;
- with `--set NAME` (repeated): each set's missing cards, and what
  completing them does, alone and together;
- with `--demand FILE` (coverage.py's): the asked ids among the cards
  added.

`--cut N` prints the ids of the chosen round: the `--set` cards and the
first N greedy steps' (computed over the pool, before the sets), one per
line, for scripts/window_matrix.py's ADDED_BATCHES.

Usage: python3 scripts/round_choice.py vtes.json --twda twda.json
           [--set NAME]… [--steps 15] [--bundle 8] [--cut N]
           [--demand FILE]
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import coverage  # noqa: E402  (the demand reader, the KRCG loaders)
import gen_twda  # noqa: E402

RECENT = coverage.RECENT


class Pool:
    """The generated tables, extended in memory by `extra` KRCG ids."""

    def __init__(self, cards):
        self.orig = gen_twda.table_ids
        self.crypt_card = {c["id"]: coverage.is_crypt(c) for c in cards}
        self.ids = set(self.orig(os.path.join(gen_twda.SRC, "crypt.rs"), "CRYPT"))
        self.ids |= set(self.orig(os.path.join(gen_twda.SRC, "library.rs"), "LIBRARY"))

    def select(self, twda, extra):
        def table_ids(path, table):
            ids = self.orig(path, table)
            crypt = table == "CRYPT"
            return ids + sorted(i for i in extra if self.crypt_card.get(i) == crypt and i not in ids)

        gen_twda.table_ids = table_ids
        try:
            return gen_twda.select(twda)
        finally:
            gen_twda.table_ids = self.orig


def deck_ids(d):
    lib = [c for s in d["library"].get("cards", []) for c in s.get("cards", [])]
    return [c["id"] for c in d["crypt"]["cards"]] + [c["id"] for c in lib]


def main():
    args = sys.argv[1:]

    def opt(name, default=None):
        if name in args:
            i = args.index(name)
            value = args[i + 1]
            del args[i:i + 2]
            return value
        return default

    sets = []
    while "--set" in args:
        sets.append(opt("--set"))
    twda_path, demand_path = opt("--twda"), opt("--demand")
    steps, bundle = int(opt("--steps", 15)), int(opt("--bundle", 8))
    cut = opt("--cut")
    cards = coverage.load(args[0] if args else None, "https://static.krcg.org/data/vtes.json")
    twda = coverage.load(twda_path, "https://static.krcg.org/data/twda.json")
    by_id = {c["id"]: c for c in cards}
    banned = {c["id"] for c in cards if c.get("banned")}
    pool = Pool(cards)
    date = {d["id"]: d["date"] for d in twda}
    recent = {d["id"] for d in twda if d["date"] >= RECENT}

    def measure(extra):
        decks, keep = pool.select(twda, extra)
        return len(decks), len(keep), len(keep & recent)

    def missing(d):
        return frozenset(i for i in deck_ids(d) if i not in pool.ids)

    base = measure(set())
    print(f"# Round choice: in the pool {base[0]}, kept {base[1]}, recent ({RECENT}+) kept {base[2]}")

    wanted = [missing(d) for d in twda if d["id"] in recent]
    ceiling = set().union(*wanted) - banned
    print(f"\n## Ceiling: every recent deck's {len(ceiling)} missing cards: {measure(ceiling)}")

    print(f"\n## Greedy steps (bundles of at most {bundle})")
    bundles = {m for m in wanted if 0 < len(m) <= bundle and not m & banned}
    extra, chosen, current = set(), [], base
    for step in range(1, steps + 1):
        best = None
        for b in bundles:
            new = b - extra
            if not new:
                continue
            got = measure(extra | new)
            score = ((got[2] - current[2]) / len(new), got[1])
            if best is None or score > best[0]:
                best = (score, new, got)
        if best is None or best[0][0] <= 0:
            break
        _, new, current = best
        extra |= new
        chosen.append(sorted(new))
        cost = len(extra) / max(1, current[2] - base[2])
        names = ", ".join(by_id[i]["name"] for i in sorted(new))
        print(f"{step:>2}. +{len(new)} ({len(extra)} in all): kept {current[1]}, recent kept "
              f"{current[2]}, {cost:.1f} cards a recent deck: {names}")

    set_ids = set()
    if sets:
        print("\n## Sets")
        for name in sets:
            ids = {c["id"] for c in cards if name in c["sets"] and c["id"] not in pool.ids | banned}
            set_ids |= ids
            print(f"{name}: {len(ids)} missing: {measure(ids)}")
        print(f"together: {len(set_ids)}: {measure(set_ids)}")

    if demand_path:
        asked = coverage.demand_counts(demand_path)
        hit = sorted((n, i) for i, n in asked.items() if i in extra | set_ids)
        print(f"\n## Demand: {len(hit)} asked ids among the cards above")
        for n, i in reversed(hit):
            print(f"{n:>5}  {i}  {by_id[i]['name']}")

    if cut is not None:
        ids = set_ids | {i for step in chosen[:int(cut)] for i in step}
        print(f"\n## Cut: {len(ids)} cards, {measure(ids)}")
        for i in sorted(ids):
            print(f"{i}  {by_id[i]['name']}")


if __name__ == "__main__":
    main()
