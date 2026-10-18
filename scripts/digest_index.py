#!/usr/bin/env python3
"""rules-digest §25, the index of rules-qa items, derived from the digest's
citations: a bare `N.M` naming a rules-qa item cites it in the section it
appears in (`## N.` or `### N.M`). Every item must be cited somewhere: a new
rules-qa item gets its line in the section stating it, then its entry here.

Usage: python3 scripts/digest_index.py            rewrite §25 in place
       python3 scripts/digest_index.py --check    exit 1 if §25 is stale or
                                                  an item is cited nowhere
"""

import re
import sys
import textwrap
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DIGEST = ROOT / "docs" / "rules-digest.md"
QA = ROOT / "docs" / "rules-qa.md"
HEADING = "\n## 25. Index of rules-qa items\n"
# A bare N.M: not part of a §-reference, a longer number, a word or a path.
CITE = re.compile(r"(?<![\w.§/-])(\d{1,3})\.(\d{1,3})(?![\w]|\.\d)")
HEAD = re.compile(r"^#{2,3} (\d+)(?:\.(\d+))?[. ]")
COUNT = re.compile(r"\d+ items, Batches 1–\d+")


def items():
    found = re.findall(r"^\*\*(\d+)\.(\d+)\*\*", QA.read_text(), re.M)
    return sorted({(int(a), int(b)) for a, b in found})


def citations(body, known):
    cites = {}
    section = None
    for line in body.split("\n"):
        m = HEAD.match(line)
        if m:
            section = "§" + m.group(1) + ("." + m.group(2) if m.group(2) else "")
            continue
        if section is None:
            continue
        for a, b in CITE.findall(line):
            k = (int(a), int(b))
            if k in known:
                cites.setdefault(k, set()).add(section)
    return cites


def order(section):
    return tuple(int(x) for x in section[1:].split("."))


def index(all_items, cites):
    out = []
    for batch in sorted({a for a, _ in all_items}):
        parts = [
            f"{a}.{b} " + ",".join(sorted(cites[(a, b)], key=order))
            for a, b in all_items
            if a == batch and (a, b) in cites
        ]
        text = f"- **{batch}** — " + "; ".join(parts)
        out.append(
            textwrap.fill(
                text,
                width=76,
                subsequent_indent="  ",
                break_long_words=False,
                break_on_hyphens=False,
            )
        )
    return "\n".join(out) + "\n"


def main():
    check = "--check" in sys.argv[1:]
    text = DIGEST.read_text()
    body, tail = text.split(HEADING)
    all_items = items()
    cites = citations(body, set(all_items))
    intro, _, entries = tail.partition("\n- **1** — ")
    _, heading, after = entries.partition("\n## ")
    count = f"{len(all_items)} items, Batches 1–{all_items[-1][0]}"
    new = body + HEADING + COUNT.sub(count, intro) + "\n" + index(all_items, cites)
    if heading:
        new += "\n" + heading.lstrip("\n") + after
    uncited = [f"{a}.{b}" for a, b in all_items if (a, b) not in cites]
    if not check:
        DIGEST.write_text(new)
    elif new != text:
        print("# rules-digest §25 is stale: run python3 scripts/digest_index.py")
    if uncited:
        print(f"# {len(uncited)} rules-qa items cited nowhere in the digest:")
        print(textwrap.fill(" ".join(uncited), width=76))
    if check and (new != text or uncited):
        sys.exit(1)
    print(f"# {len(all_items)} items, {len(all_items) - len(uncited)} indexed")


if __name__ == "__main__":
    main()
