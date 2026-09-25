#!/usr/bin/env python3
"""Extract the rulings backlog for the V5 pool from vtes-rulings.

Every ruling in ../vtes-advanced-rules/vtes-rulings/rulings/rulings.yaml that
is keyed to a V5 card, or that mentions one in braces ({id|name}), becomes a
row: the P1 "rulings as tests" backlog (DESIGN §7). Rulings are prose with
provenance, so each row is hand-translated into a scenario test; `status`
tracks that (empty = not yet):

  test:<fn>        backed by `#[test] fn <fn>` under crates/engine/tests/,
                   whose doc comment cites the row's references
  blocked:<gap>    needs a card or mechanism the engine does not have yet
  n/a:<reason>     not engine-observable, or the V5 card is incidental

Regeneration keeps the statuses of unchanged rows (keyed on the full row
text); a reworded ruling loses its status and is printed as stale, since it
needs re-verification.

Usage: python3 scripts/rulings_v5.py [path/to/vtes.json] > data/rulings-v5.csv
       python3 scripts/rulings_v5.py --check data/rulings-v5.csv

`--check` (offline) verifies the ledger against the engine: every row on an
implemented card (crates/engine/src/cards/defs.rs `DEFS`) has a status; every
`test:` name is exactly one test function whose doc comment cites the
references; every test in tests/rulings.rs backs at least one row.
"""
import csv
import json
import os
import re
import sys
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from window_matrix import in_pool  # noqa: E402

RULINGS_DIR = os.path.join(os.path.dirname(__file__), "..", "..", "vtes-advanced-rules",
                           "vtes-rulings", "rulings")
RULINGS = os.path.join(RULINGS_DIR, "rulings.yaml")
GROUPS = os.path.join(RULINGS_DIR, "groups.yaml")


# A card or group key line; keys holding a colon or an apostrophe are quoted.
KEY_RE = r"^(['\"]?)(G?\d+)\|(.+?)\1:$"


def parse_groups(path):
    """{group_key: [(card_id, card_name)]} from the hand-formatted YAML."""
    groups, cur = {}, None
    for line in open(path, encoding="utf-8"):
        m = re.match(KEY_RE, line)
        if m:
            cur = m.group(2)
            groups[cur] = []
            continue
        m = re.match(r"^\s+(['\"]?)(\d+)\|(.+?)\1:", line)
        if m and cur:
            groups[cur].append((int(m.group(2)), m.group(3)))
    return groups


def parse(path):
    """Yield (key_id, key_name, ruling_text) from the hand-formatted YAML."""
    key = None
    buf = None
    for line in open(path, encoding="utf-8"):
        line = line.rstrip("\n")
        if line.startswith("#") or not line.strip():
            continue
        m = re.match(KEY_RE, line)
        if m:
            if buf is not None:
                yield key, buf
            key, buf = (m.group(2), m.group(3)), None
            continue
        if re.match(r"^\s*- ", line):
            if buf is not None:
                yield key, buf
            buf = line.strip()[2:]
        elif buf is not None:
            buf += " " + line.strip()
    if buf is not None:
        yield key, buf


ENGINE = os.path.join(os.path.dirname(__file__), "..", "crates", "engine")
CSV_PATH = os.path.join(os.path.dirname(__file__), "..", "data", "rulings-v5.csv")
KEY = ("card_id", "via", "references", "ruling")


def implemented():
    """Printed names of the cards in `DEFS`, from the defs source."""
    src = open(os.path.join(ENGINE, "src", "cards", "defs.rs"), encoding="utf-8").read()
    names = dict(re.findall(r"pub static ([A-Z_0-9]+): CardDef = CardDef \{\s*name: \"([^\"]+)\"", src))
    for m in re.finditer(r"(?:political|hunting_ground)!\(\s*([A-Z_0-9]+)\s*,\s*\"([^\"]+)\"", src):
        names[m.group(1)] = m.group(2)
    m = re.search(r"DEFS: \[&CardDef; (\d+)\] = \[(.*?)\];", src, re.S)
    idents = re.findall(r"&([A-Z_0-9]+)", m.group(2))
    assert len(idents) == int(m.group(1)), "DEFS length literal out of sync"
    missing = [i for i in idents if i not in names]
    assert not missing, f"cannot read the printed name of {missing}"
    found = {names[i] for i in idents}
    # Vampires with printed text (crates/engine/src/cards/vampires.rs `VAMPIRES`).
    src = open(os.path.join(ENGINE, "src", "cards", "vampires.rs"), encoding="utf-8").read()
    vnames = dict(re.findall(r"pub static ([A-Z_0-9]+): VampireDef = VampireDef \{\s*name: \"([^\"]+)\"", src))
    m = re.search(r"VAMPIRES: \[&VampireDef; (\d+)\] = \[(.*?)\];", src, re.S)
    idents = re.findall(r"&([A-Z_0-9]+)", m.group(2))
    assert len(idents) == int(m.group(1)), "VAMPIRES length literal out of sync"
    missing = [i for i in idents if i not in vnames]
    assert not missing, f"cannot read the printed name of {missing}"
    return found | {vnames[i] for i in idents}


def tests():
    """{fn_name: (file, doc)} for every #[test] under crates/engine/tests."""
    found = {}
    tdir = os.path.join(ENGINE, "tests")
    for fn in sorted(os.listdir(tdir)):
        if not fn.endswith(".rs"):
            continue
        lines = open(os.path.join(tdir, fn), encoding="utf-8").read().split("\n")
        for i, line in enumerate(lines):
            if line.strip() != "#[test]":
                continue
            j = i + 1
            while not lines[j].startswith("fn "):
                j += 1
            name = re.match(r"fn (\w+)", lines[j]).group(1)
            k, doc = i - 1, []
            while k >= 0 and lines[k].startswith(("///", "//")):
                doc.append(lines[k])
                k -= 1
            # The comment as one line, so a citation may wrap.
            text = " ".join(l.lstrip("/ ") for l in reversed(doc))
            text = re.sub(r"\s+", " ", text)
            if name in found:
                print(f"test {name} defined in both {found[name][0]} and {fn}")
                found[name] = (None, "")
            else:
                found[name] = (fn, text)
    return found


def check(path):
    rows = list(csv.DictReader(open(path, encoding="utf-8", newline="")))
    cards = implemented()
    found = tests()
    bad, used = 0, set()
    for r in rows:
        st = r["status"]
        if r["card"] not in cards:
            if st:
                print(f"{r['card']!r}: status {st!r} on a card not in DEFS")
                bad += 1
            continue
        if not st:
            print(f"{r['card']!r} {r['references']}: no status")
            bad += 1
        elif st.startswith("test:"):
            name = st[5:]
            used.add(name)
            if name not in found or found[name][0] is None:
                print(f"{r['card']!r} {r['references']}: no unique test fn {name}")
                bad += 1
                continue
            tag = re.search(r"\[RBK [^\]]+\]", r["ruling"])
            refs = r["references"] or (tag.group(0) if tag else None)
            if refs is None:
                print(f"{r['card']!r}: no references and no [RBK] tag to cite")
                bad += 1
            elif refs not in found[name][1]:
                print(f"{r['card']!r}: {name} ({found[name][0]}) does not cite {refs}")
                bad += 1
        elif not st.startswith(("blocked:", "n/a:")):
            print(f"{r['card']!r} {r['references']}: unknown status {st!r}")
            bad += 1
    for name, (fn, _) in found.items():
        if fn == "rulings.rs" and name not in used:
            print(f"tests/rulings.rs: {name} backs no ruling row")
            bad += 1
    n = sum(r["card"] in cards for r in rows)
    print(f"# {len(rows)} rows, {n} on {len(cards)} implemented cards, "
          f"{len(used)} tests referenced, {bad} problems", file=sys.stderr)
    return bad


def main():
    args = sys.argv[1:]
    if "--check" in args:
        i = args.index("--check")
        sys.exit(1 if check(args[i + 1]) else 0)
    previous = {}
    if os.path.exists(CSV_PATH):
        for r in csv.DictReader(open(CSV_PATH, encoding="utf-8", newline="")):
            previous[tuple(r[k] for k in KEY)] = r["status"]
    if args:
        cards = json.load(open(args[0]))
    else:
        with urllib.request.urlopen("https://static.krcg.org/data/vtes.json") as f:
            cards = json.load(f)
    v5 = {c["id"]: c["_name"] for c in cards if in_pool(c)}
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
            via = via if via != "mention" else f"mention in {kname}"
            key = (str(cid), via, refs, body)
            out.writerow([cid, v5[cid], via, refs, body, previous.pop(key, "")])
            n += 1
    for key, st in previous.items():
        if st:
            print(f"# stale status dropped: {key[:3]} {st}", file=sys.stderr)
    print(f"# {n} rulings rows on V5 cards", file=sys.stderr)


if __name__ == "__main__":
    main()
