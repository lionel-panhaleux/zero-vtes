#!/usr/bin/env python3
"""The harness's counts (CLAUDE.md "Harness"), read from git.

A board line is a list item opening with a bold ask (`- **ask**: …`) in
`docs/boards/*.md`; the ask is its key. A cycle is one line deleted (a key
gone from every board in a commit: a line moved between boards is not a
cycle, nor is an ask reworded in place). The last upkeep is the last commit
carrying an `Upkeep:` trailer.

    python3 scripts/harness.py size      lines per board
    python3 scripts/harness.py age       each line's cycles since it was touched
    python3 scripts/harness.py due       whether an upkeep pass is due
    python3 scripts/harness.py orphans   docs pages docs/index.md does not reach
"""

import datetime
import difflib
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BOARDS = "docs/boards"
KEY = re.compile(r"^- \*\*(.+?)\*\*")
CYCLES_DUE = 20
DAYS_DUE = 30


def git(*args):
    return subprocess.run(
        ["git", *args], cwd=ROOT, check=True, capture_output=True, text=True
    ).stdout


def keys(text):
    return {m.group(1) for line in text.splitlines() if (m := KEY.match(line))}


def board_keys(rev):
    """Every board line's key at `rev` (empty before the boards existed)."""
    out = set()
    listing = git("ls-tree", "--name-only", rev, "--", f"{BOARDS}/") if rev else ""
    for path in listing.split():
        if path.endswith(".md"):
            out |= keys(git("show", f"{rev}:{path}"))
    return out


def deleted(before, after):
    """Keys gone from `before`, less those reworded into a new key."""
    added = list(after - before)
    gone = 0
    for key in before - after:
        close = difflib.get_close_matches(key, added, n=1, cutoff=0.6)
        if close:
            added.remove(close[0])
        else:
            gone += 1
    return gone


def board_commits():
    """(sha, date, cycles, upkeep) of each commit touching the boards, oldest first."""
    log = git("log", "--reverse", "--format=%H %cs", "--", BOARDS).split("\n")
    out = []
    for entry in filter(None, log):
        sha, date = entry.split()
        parents = git("rev-list", "--parents", "-n1", sha).split()[1:]
        before = board_keys(parents[0]) if parents else set()
        cycles = deleted(before, board_keys(sha))
        body = git("show", "-s", "--format=%B", sha)
        upkeep = any(line.startswith("Upkeep:") for line in body.splitlines())
        out.append((sha, date, cycles, upkeep))
    return out


def last_upkeep(commits):
    """Index of the last upkeep commit; the first list-shaped board if none."""
    for i in range(len(commits) - 1, -1, -1):
        if commits[i][3]:
            return i
    for i, (sha, *_rest) in enumerate(commits):
        if board_keys(sha):
            return i
    return len(commits) - 1


def size():
    total = 0
    for path in sorted((ROOT / BOARDS).glob("*.md")):
        n = len(keys(path.read_text()))
        total += n
        print(f"{path.stem:<10} {n}")
    print(f"{'total':<10} {total}")


def age():
    commits = board_commits()
    index = {sha: i for i, (sha, *_rest) in enumerate(commits)}
    # Cycles after each board commit, to the tip.
    after = [0] * (len(commits) + 1)
    for i in range(len(commits) - 1, -1, -1):
        after[i] = after[i + 1] + commits[i][2]
    rows = []
    for path in sorted((ROOT / BOARDS).glob("*.md")):
        rel = path.relative_to(ROOT).as_posix()
        blame = git("blame", "--line-porcelain", "HEAD", "--", rel)
        sha = None
        for line in blame.splitlines():
            if re.match(r"^[0-9a-f]{40} ", line):
                sha = line.split()[0]
            elif line.startswith("\t") and (m := KEY.match(line[1:])):
                i = index.get(sha)
                untouched = after[i + 1] if i is not None else 0
                rows.append((untouched, path.stem, m.group(1)))
    for untouched, board, key in sorted(rows, key=lambda r: (-r[0], r[1], r[2])):
        flag = "  on trial" if untouched >= CYCLES_DUE else ""
        print(f"{untouched:>3}  {board:<10} {key}{flag}")


def due():
    commits = board_commits()
    if not commits:
        print("no board history")
        return
    i = last_upkeep(commits)
    cycles = sum(c[2] for c in commits[i + 1 :])
    since = datetime.date.fromisoformat(commits[i][1])
    days = (datetime.date.today() - since).days
    reasons = []
    if cycles >= CYCLES_DUE:
        reasons.append(f"{cycles} cycles")
    if days >= DAYS_DUE:
        reasons.append(f"{days} days")
    word = "due" if reasons else "not due"
    print(
        f"upkeep {word}: {cycles} cycles and {days} days since"
        f" {commits[i][0][:7]} ({since})"
        + (f" — {', '.join(reasons)}" if reasons else "")
    )


def orphans():
    index = (ROOT / "docs/index.md").read_text()
    links = re.findall(r"\]\(([^)#:]+\.md)\)", index)
    linked = {(ROOT / "docs" / link).resolve() for link in links}
    pages = [p for p in git("ls-files", "docs").split() if p.endswith(".md")]
    missing = [
        p
        for p in pages
        if p != "docs/index.md" and (ROOT / p).resolve() not in linked
    ]
    dangling = [link for link in links if not (ROOT / "docs" / link).exists()]
    for p in missing:
        print(f"not in docs/index.md: {p}")
    for link in dangling:
        print(f"docs/index.md links a missing page: {link}")
    if not missing and not dangling:
        print("every docs page reachable from docs/index.md")
    return 1 if missing or dangling else 0


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    commands = {"size": size, "age": age, "due": due, "orphans": orphans}
    if cmd not in commands:
        sys.exit(__doc__)
    sys.exit(commands[cmd]() or 0)
