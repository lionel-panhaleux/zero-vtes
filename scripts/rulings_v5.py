#!/usr/bin/env python3
"""Extract the rulings backlog for the V5 pool from vtes-rulings.

Every ruling in ../vtes-advanced-rules/vtes-rulings/rulings/rulings.yaml that
is keyed to a V5 card, or that mentions one in braces ({id|name}), becomes a
row: the P1 "rulings as tests" backlog (DESIGN §7). Rulings are prose with
provenance, so each row is hand-translated into a scenario test; `status`
tracks that (empty = not yet).

Usage: python3 scripts/rulings_v5.py [path/to/vtes.json] > data/rulings-v5.csv
"""
import csv
import json
import os
import re
import sys
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from window_matrix import V5_SETS  # noqa: E402

RULINGS_DIR = os.path.join(os.path.dirname(__file__), "..", "..", "vtes-advanced-rules",
                           "vtes-rulings", "rulings")
RULINGS = os.path.join(RULINGS_DIR, "rulings.yaml")
GROUPS = os.path.join(RULINGS_DIR, "groups.yaml")


def parse_groups(path):
    """{group_key: [(card_id, card_name)]} from the hand-formatted YAML."""
    groups, cur = {}, None
    for line in open(path, encoding="utf-8"):
        m = re.match(r"^(G\d+)\|(.+):$", line)
        if m:
            cur = m.group(1)
            groups[cur] = []
            continue
        m = re.match(r"^\s+(\d+)\|([^:]+):", line)
        if m and cur:
            groups[cur].append((int(m.group(1)), m.group(2)))
    return groups


def parse(path):
    """Yield (key_id, key_name, ruling_text) from the hand-formatted YAML."""
    key = None
    buf = None
    for line in open(path, encoding="utf-8"):
        line = line.rstrip("\n")
        if line.startswith("#") or not line.strip():
            continue
        m = re.match(r"^(G?\d+)\|(.+):$", line)
        if m:
            if buf is not None:
                yield key, buf
            key, buf = (m.group(1), m.group(2)), None
            continue
        if re.match(r"^\s*- ", line):
            if buf is not None:
                yield key, buf
            buf = line.strip()[2:]
        elif buf is not None:
            buf += " " + line.strip()
    if buf is not None:
        yield key, buf


def main():
    args = sys.argv[1:]
    if args:
        cards = json.load(open(args[0]))
    else:
        with urllib.request.urlopen("https://static.krcg.org/data/vtes.json") as f:
            cards = json.load(f)
    v5 = {c["id"]: c["_name"] for c in cards if set(V5_SETS) & set(c.get("ordered_sets", []))}
    groups = parse_groups(GROUPS)
    out = csv.writer(sys.stdout)
    out.writerow(["card_id", "card", "via", "references", "ruling", "status"])
    n = 0
    for (kid, kname), text in parse(RULINGS):
        text = text.strip().strip("'\"")
        refs = " ".join(re.findall(r"\[(?:TOM|SFC|JON|LSJ|PIB|ANK|RTR|RBK) [0-9-]+\]", text))
        body = re.sub(r"\s*\[(?:TOM|SFC|JON|LSJ|PIB|ANK|RTR|RBK) [0-9-]+\]", "", text).strip()
        targets = {}
        if not kid.startswith("G") and int(kid) in v5:
            targets[int(kid)] = "key"
        if kid.startswith("G"):
            for gid, _ in groups.get(kid, []):
                if gid in v5:
                    targets[gid] = f"group {kname}"
        for mid, mname in re.findall(r"\{(\d+)\|([^}]+)\}", text):
            if int(mid) in v5:
                targets.setdefault(int(mid), "mention")
        for cid, via in sorted(targets.items()):
            out.writerow([cid, v5[cid], via if via != "mention" else f"mention in {kname}", refs, body, ""])
            n += 1
    print(f"# {n} rulings rows on V5 cards", file=sys.stderr)


if __name__ == "__main__":
    main()
