#!/usr/bin/env python3
"""The weekly deep sweeps (DESIGN §7.2): the fuzz fixtures at `FUZZ_SEEDS`
seeds and the host's sweeps at `HOST_SEEDS`, on `main`'s tip, the report
under the main checkout's `data/sweeps/` (ignored). A panic or a key
collision found goes through `intake`; a seed a fix needs is kept in the
sweep (`KEPT` in crates/host/tests/events.rs, a fixture's `seeds`).

Refused while a training run shares the machine (`pgrep -f train.ppo`: a
deep sweep's load beside one is what CLAUDE.md forbids) and from a
checkout not on `main` or with uncommitted edits (the sweep is of
`main`'s tip). Z-main runs it on its schedule (CLAUDE.md "Sessions").

Usage: python3 scripts/sweeps.py [--fuzz N] [--host N] [--out DIR]
Exits non-zero when a sweep fails or is refused; the last line names the
report to read.
"""

import argparse
import datetime
import os
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent


def git(*args):
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()


def run(name, env, cmd):
    """One sweep: its command's tail, the failures and panics it names."""
    print(f"sweeps: {name}: {' '.join(cmd)}", flush=True)
    p = subprocess.run(cmd, cwd=ROOT, env={**os.environ, **env}, capture_output=True, text=True)
    out = p.stdout + p.stderr
    totals = [l for l in out.splitlines() if l.startswith("test result:")]
    passed = sum(int(m.group(1)) for l in totals if (m := re.search(r"(\d+) passed", l)))
    failed = sum(int(m.group(1)) for l in totals if (m := re.search(r"(\d+) failed", l)))
    panics = [l for l in out.splitlines() if "panicked at" in l or l.startswith("thread '")]
    notes = [l for l in out.splitlines() if re.search(r"seed|SEED_FROM|stops|collision", l, re.I)]
    ok = p.returncode == 0 and failed == 0
    lines = [f"## {name}: {'ok' if ok else 'FAILED'} ({passed} passed, {failed} failed, exit {p.returncode})",
             f"{' '.join(f'{k}={v}' for k, v in env.items())} {' '.join(cmd)}"]
    if not ok:
        lines += ["", "Panics:", *panics[:40], "", "Seeds named:", *notes[:80], "", "Tail:", *out.splitlines()[-60:]]
    return ok, lines


def main():
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("--fuzz", type=int, default=600, help="FUZZ_SEEDS (default 600)")
    p.add_argument("--host", type=int, default=400, help="HOST_SEEDS (default 400)")
    p.add_argument("--out", type=pathlib.Path, help="the reports (default: the main checkout's data/sweeps)")
    args = p.parse_args()
    if subprocess.run(["pgrep", "-f", "train.ppo"], capture_output=True).returncode == 0:
        sys.exit("sweeps: a training run shares this machine (pgrep -f train.ppo); run once it ends")
    branch = git("rev-parse", "--abbrev-ref", "HEAD")
    dirty = git("status", "--porcelain", "--untracked-files=no")
    if branch != "main" or dirty:
        sys.exit(f"sweeps: this checkout is on {branch}{' with edits' if dirty else ''}; run it from main, clean")
    out = args.out or pathlib.Path(git("rev-parse", "--path-format=absolute", "--git-common-dir")).parent / "data" / "sweeps"
    out.mkdir(parents=True, exist_ok=True)
    tip = git("rev-parse", "--short=12", "HEAD")
    # Every test binary run, a failing one included (cargo stops at the
    # first otherwise, hiding the rest); a gentle load.
    gentle = ["--no-fail-fast", "-j", "4", "--", "--test-threads", "4"]
    sweeps = [
        ("fuzz", {"FUZZ_SEEDS": str(args.fuzz)}, ["cargo", "test", "--profile", "ci", "-q", "--test", "fuzz", *gentle]),
        ("host", {"HOST_SEEDS": str(args.host)}, ["cargo", "test", "--profile", "ci", "-q", "-p", "zvtes-host", *gentle]),
    ]
    results = [run(*s) for s in sweeps]
    day = datetime.date.today().isoformat()
    report = out / f"{day}-{tip}.txt"
    body = [f"# Deep sweeps of {tip} ({day})", ""]
    for _, lines in results:
        body += [*lines, ""]
    report.write_text("\n".join(body))
    ok = all(r[0] for r in results)
    print(f"sweeps: {'ok' if ok else 'FAILED'}: {report}")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
