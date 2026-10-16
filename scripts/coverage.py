#!/usr/bin/env python3
"""Card coverage: the engine pool (the generated `crypt.rs` and `library.rs`
tables) against the full KRCG card list, by type and by set; the TWDA decks
fully in the pool and those kept (`twda.rs`, gen_twda.py's rules); and the
card ids players asked for, most asked first, when a demand file is given.
The cards board's header quotes its summary (add-cards §1 runs it before
choosing a round).

Banned cards are out of scope (owner, 2026-10-14): counted apart, never as
missing. A demand file is the server's count of the ids its refusals named
(`/admin/unsupported`, behind the admin token): a JSON list of
{"id": <KRCG id>, "count": n}, most asked first.

Usage: python3 scripts/coverage.py [vtes.json] [--twda twda.json]
                                   [--demand demand.json] [--sets]
Downloads https://static.krcg.org/data/vtes.json (and twda.json) when no
path is given; `--sets` prints every set, the summary only its totals.
"""
import json
import os
import sys
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen_twda  # noqa: E402  (its pool readers and deck rules)

SRC = gen_twda.SRC
CRYPT_TYPES = {"Vampire", "Imbued"}
RECENT = "2025"  # the owner's metric (2026-10-14): recent decks, 2025 on


def load(path, url):
    if path:
        return json.load(open(path))
    with urllib.request.urlopen(url) as f:
        return json.load(f)


def is_crypt(card):
    return bool(CRYPT_TYPES & set(card["types"]))


def ratio(done, total):
    return f"{done:,} of {total:,} ({100 * done / total:.1f}%)" if total else "0 of 0"


def demand_counts(path):
    """The server's `/admin/unsupported`: [{"id": <KRCG id>, "count": n}, …]."""
    return {int(d["id"]): int(d["count"]) for d in json.load(open(path))}


def main():
    args = sys.argv[1:]

    def opt(name):
        if name in args:
            i = args.index(name)
            value = args[i + 1]
            del args[i:i + 2]
            return value
        return None

    twda_path, demand_path = opt("--twda"), opt("--demand")
    all_sets = "--sets" in args
    if all_sets:
        args.remove("--sets")
    cards = load(args[0] if args else None, "https://static.krcg.org/data/vtes.json")
    twda = load(twda_path, "https://static.krcg.org/data/twda.json")

    pool = set(gen_twda.table_ids(os.path.join(SRC, "crypt.rs"), "CRYPT"))
    pool |= set(gen_twda.table_ids(os.path.join(SRC, "library.rs"), "LIBRARY"))
    by_id = {c["id"]: c for c in cards}
    banned = {c["id"] for c in cards if c.get("banned")}

    print("# Coverage")
    for label, part in (("crypt", True), ("library", False)):
        side = [c for c in cards if is_crypt(c) == part]
        scope = [c for c in side if c["id"] not in banned]
        done = sum(c["id"] in pool for c in scope)
        print(f"{label}: {ratio(done, len(scope))}; {len(side) - len(scope)} banned apart")

    print("\n## By type")
    types = sorted({t for c in cards for t in c["types"]})
    for t in types:
        scope = [c for c in cards if t in c["types"] and c["id"] not in banned]
        print(f"{t}: {ratio(sum(c['id'] in pool for c in scope), len(scope))}")

    sets = {}
    for c in cards:
        if c["id"] in banned:
            continue
        for name, prints in c["sets"].items():
            date = min((p.get("release_date") or "9999") for p in prints) if prints else "9999"
            s = sets.setdefault(name, {"date": date, "n": 0, "done": 0})
            s["date"] = min(s["date"], date)
            s["n"] += 1
            s["done"] += c["id"] in pool
    full = [n for n, s in sets.items() if s["done"] == s["n"]]
    print(f"\n## By set: {len(full)} of {len(sets)} sets complete")
    if all_sets:
        for name, s in sorted(sets.items(), key=lambda kv: (kv[1]["date"], kv[0])):
            print(f"{s['date'][:10]} {name}: {ratio(s['done'], s['n'])}")

    decks, keep = gen_twda.select(twda)
    date = {d["id"]: d["date"] for d in twda}
    print("\n## TWDA")
    print(f"decks in the pool: {ratio(len(decks), len(twda))}; kept {len(keep)}")
    recent = [d for d in twda if d["date"] >= RECENT]
    recent_in = [d for d in decks if d[5] >= RECENT]
    recent_kept = [i for i in keep if date[i] >= RECENT]
    print(f"recent ({RECENT}+): {ratio(len(recent_in), len(recent))} in the pool; kept {len(recent_kept)}")

    print("\n## Demand")
    if not demand_path:
        print("none given (--demand: the server's /admin/unsupported counts)")
        return
    asked = demand_counts(demand_path)
    missing = sorted(((n, i) for i, n in asked.items() if i not in pool), reverse=True)
    print(f"{len(asked)} ids asked, {len(missing)} not in the pool:")
    for n, i in missing:
        card = by_id.get(i)
        name = card["name"] if card else "unknown id"
        print(f"{n:>5}  {i}  {name}{' (banned)' if i in banned else ''}")


if __name__ == "__main__":
    main()
