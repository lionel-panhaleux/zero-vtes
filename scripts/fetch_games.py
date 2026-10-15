#!/usr/bin/env python3
"""Pull the engine server's game records (docs/online-plan.md B4): the
games recorded since a date, of an engine commit, each written as
`<out>/<game id>.json` — what training reads (`--bc`, `--starts`,
`train.assess`) like the corpus (`data/records`). Records already pulled
are kept unless the server has more of them (a game still going then).
Without --since, the pull starts at the cursor `<out>/.since` (Unix ms:
the last game pulled's start + 1, held back to the oldest game still
going so it is pulled once it ends), else at the newest local record;
the cursor moves after a pull that completed (not one of --engine).

Usage: python3 scripts/fetch_games.py [--server https://engine.krcg.org]
    [--out data/server-records] [--since 2026-10-14] [--engine COMMIT]
    [--finished]

The admin token comes from ZVTES_ADMIN_TOKEN, else
~/.config/zvtes/admin_token (the server keeps only its SHA-256).
"""

import argparse
import datetime
import json
import os
import pathlib
import sys
import urllib.parse
import urllib.request

DAY_MS = 24 * 3600 * 1000


def token():
    t = os.environ.get("ZVTES_ADMIN_TOKEN")
    if t:
        return t.strip()
    path = pathlib.Path.home() / ".config" / "zvtes" / "admin_token"
    if path.exists():
        return path.read_text().strip()
    sys.exit("no admin token: ZVTES_ADMIN_TOKEN or ~/.config/zvtes/admin_token")


def cursor(started, going):
    """Where the next pull starts: past the newest game, but no later
    than the oldest one still going — a day back at most (a game going
    when the server restarts is ended at its start; an idle one in 30
    minutes)."""
    if started is None:
        return None
    return min([started + 1] + [g for g in going if g > started - DAY_MS])


def local_cursor(out):
    """The cursor from the records already pulled, when `.since` is absent."""
    newest, going = None, []
    for path in out.glob("*.json"):
        try:
            r = json.loads(path.read_text())
        except (OSError, ValueError):
            continue
        if not r.get("started"):
            continue
        newest = max(newest or 0, r["started"])
        if r.get("ended") is None:
            going.append(r["started"])
    return cursor(newest, going)


def get(server, path, auth, query=None):
    url = server.rstrip("/") + path
    if query:
        url += "?" + urllib.parse.urlencode({k: v for k, v in query.items() if v is not None})
    req = urllib.request.Request(url, headers={"Authorization": f"Bearer {auth}"})
    with urllib.request.urlopen(req, timeout=60) as r:
        return json.load(r)


def main():
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("--server", default="https://engine.krcg.org")
    p.add_argument("--out", default="data/server-records")
    p.add_argument("--since", help="a date (YYYY-MM-DD) or Unix ms")
    p.add_argument("--engine", help="only games of this engine commit")
    p.add_argument("--finished", action="store_true", help="only games that ended")
    args = p.parse_args()
    since = None
    if args.since:
        since = (
            int(args.since)
            if args.since.isdigit()
            else int(datetime.datetime.fromisoformat(args.since).replace(tzinfo=datetime.timezone.utc).timestamp() * 1000)
        )
    auth = token()
    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    cursor_file = out / ".since"
    if since is None:
        since = int(cursor_file.read_text()) if cursor_file.exists() else local_cursor(out)
    start = since
    fetched = kept = 0
    newest, going = None, []
    while True:
        games = get(args.server, "/admin/games", auth, {"since": since, "engine": args.engine, "limit": 1000})
        for g in games:
            newest = max(newest or 0, g["started"])
            if g.get("ended") is None:
                going.append(g["started"])
            if args.finished and g.get("ended") is None:
                continue
            path = out / f"{g['id']}.json"
            if path.exists():
                have = json.loads(path.read_text())
                if len(have.get("entries", [])) >= g["entries"]:
                    kept += 1
                    continue
            record = get(args.server, f"/admin/games/{urllib.parse.quote(g['id'])}/record", auth)
            path.write_text(json.dumps(record))
            fetched += 1
        if len(games) < 1000:
            break
        since = games[-1]["started"] + 1
    after = cursor(newest, going)
    if after is not None and not args.engine:  # an engine's pull skips the others' games
        cursor_file.write_text(f"{after}\n")
    print(f"{fetched} records fetched, {kept} up to date, in {out} (cursor {after or start})")


if __name__ == "__main__":
    main()
