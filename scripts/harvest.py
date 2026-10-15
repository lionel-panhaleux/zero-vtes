#!/usr/bin/env python3
"""The daily harvest: pull the finished online games since the cursor
(`scripts/fetch_games.py --finished`), then write the report
(`scripts/games_report.py`), both on the one corpus,
`data/server-records/` of the main checkout — whichever worktree runs
it: each worktree has its own ignored `data/`, and a pull into one would
split the corpus and its cursor. The report replays each new record on
this script's checkout for its turns and caches them in the corpus for
good, so the script refuses a checkout not on `main` or with uncommitted
edits under `crates/` (`--no-turns` skips the replay and the check).

Usage: python3 scripts/harvest.py [--out DIR] [report options…]
(the other options go to `games_report.py`). Exits non-zero when either
step fails; the last line names the reports to read.
"""

import argparse
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent


def git(*args):
    return subprocess.run(["git", *args], cwd=HERE, capture_output=True, text=True, check=True).stdout.strip()


def main():
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("--out", type=pathlib.Path, help="the corpus (default: the main checkout's data/server-records)")
    args, report = p.parse_known_args()
    # The main working tree: the parent of the repository's common `.git`.
    out = args.out or pathlib.Path(git("rev-parse", "--path-format=absolute", "--git-common-dir")).parent / "data" / "server-records"
    if "--no-turns" not in report:
        branch = git("rev-parse", "--abbrev-ref", "HEAD")
        dirty = git("status", "--porcelain", "--", ":/crates")
        if branch != "main" or dirty:
            sys.exit(f"harvest: this checkout is on {branch}{' with engine edits' if dirty else ''}; run it from main, clean (or --no-turns)")
    for step in (
        [sys.executable, HERE / "fetch_games.py", "--finished", "--out", out],
        [sys.executable, HERE / "games_report.py", out, *report],
    ):
        if subprocess.run([str(a) for a in step]).returncode != 0:
            sys.exit(f"harvest: {step[1].name} failed")


if __name__ == "__main__":
    main()
