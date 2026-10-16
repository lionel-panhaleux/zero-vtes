#!/usr/bin/env python3
"""What the online games say: the game records of a directory (those
`scripts/fetch_games.py` pulls, or `data/records`) read into a markdown
report — `<dir>/reports/<date>.md` for the games first seen by this run
(a second run the same day adds to it), `<dir>/reports/total.md` for
every game — and a short summary on stdout.

Usage: python3 scripts/games_report.py [DIR] [--catalog FILE|URL]
    [--date YYYY-MM-DD] [--no-turns] [--no-waste] [--list FILE] [--min-rounds 2] [--min-decisions 30]
    [--test-clients client.mjs,load.mjs]

A game counts when it ended normally with a human seat; abandoned games,
engine errors, games a server restart ended, games still going, games
without a human, the server's test scripts' (`spec.client`) and very
short ones (fewer than --min-rounds rounds, or with turns unknown fewer
than --min-decisions human decisions) are counted apart. A record carries
its turn count from protocol 1.7.0 (`result.turns`); an older one's turns
are read by replaying it on this checkout's engine (`cargo build -p
zvtes-host --profile ci --example replay`, run once a record, cached in
`<dir>/reports/index.json`); "≥" marks a game the engine stopped partway. Decks are named from the server's catalog
(`/catalog`, cached in `<dir>/reports/catalog.json`) when a seat's list
equals one; else "custom", with the closest catalog deck when near.
Each report ends with the bots' waste in its counted games (`waste`:
`cargo build -p zvtes-policy --profile ci --example waste`, by profile and
weights; `--no-waste` skips it). `--list FILE` writes the counted games'
paths, absolute and comma-separated: a run's `--bc` and `--starts` list.
"""

import argparse
import collections
import datetime
import json
import os
import pathlib
import re
import subprocess
import sys
import urllib.request

REPO = pathlib.Path(__file__).resolve().parent.parent
CATALOG_URL = "https://engine.krcg.org/catalog"
SMALL = 10  # samples below this are marked
NOTABLE_VP = 3


def load_catalog(source, cache):
    """The shipped decks: [(name, {id: copies})], from a file or a URL,
    else the cached copy, else none."""
    try:
        if re.match(r"https?://", source):
            with urllib.request.urlopen(source, timeout=60) as r:
                data = json.load(r)
            cache.parent.mkdir(parents=True, exist_ok=True)
            cache.write_text(json.dumps(data))
        else:
            data = json.loads(pathlib.Path(source).read_text())
    except (OSError, ValueError) as e:
        if not cache.exists():
            print(f"no catalog ({e}): every deck reads custom", file=sys.stderr)
            return []
        print(f"catalog unavailable ({e}): the cached copy is used", file=sys.stderr)
        data = json.loads(cache.read_text())
    return [(name, clean(deck)) for name, deck in data]


def clean(deck):
    return {str(k): int(v) for k, v in deck.items() if int(v) > 0}


def deck_name(deck, catalog):
    deck = clean(deck)
    best, score = None, 0.0
    total = sum(deck.values())
    for name, d in catalog:
        if d == deck:
            return name
        common = sum(min(n, d.get(k, 0)) for k, n in deck.items())
        s = common / max(total, sum(d.values()), 1)
        if s > score:
            best, score = name, s
    if best and score >= 0.75:
        return f"custom (≈ {best}, {score:.0%})"
    return "custom"


def replayer():
    """The replay example's binary, built (or brought up to date) once."""
    target = pathlib.Path(os.environ.get("CARGO_TARGET_DIR") or REPO / "target")
    try:
        subprocess.run(
            ["cargo", "build", "-q", "-p", "zvtes-host", "--profile", "ci", "--example", "replay"],
            cwd=REPO,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError) as e:
        print(f"the replay example did not build ({e}): turns unknown", file=sys.stderr)
        return None
    return target / "ci" / "examples" / "replay"


def waste(paths):
    """The `waste` lines of the bots' seats in `paths` (the
    zvtes-policy example, built once): a markdown section, empty when
    there is no game or it does not build."""
    if not paths:
        return ""
    target = pathlib.Path(os.environ.get("CARGO_TARGET_DIR") or REPO / "target")
    try:
        subprocess.run(
            ["cargo", "build", "-q", "-p", "zvtes-policy", "--profile", "ci", "--example", "waste"],
            cwd=REPO,
            check=True,
        )
        run = subprocess.run(
            [str(target / "ci" / "examples" / "waste"), "-", "0", *map(str, paths)],
            capture_output=True,
            text=True,
            check=True,
            timeout=1800,
        )
    except (OSError, subprocess.CalledProcessError, subprocess.TimeoutExpired) as e:
        print(f"waste did not run ({e}): no waste section", file=sys.stderr)
        return ""
    # A record skipped or read partway: said in the section, its seats
    # counted only as far as they were read.
    partial = [line for line in run.stderr.splitlines() if ": skipped (" in line or ": read to its decision" in line]
    # Its groups: a line ending in ":", then their indented lines.
    keep, lines = False, []
    for line in run.stdout.splitlines():
        if not line.startswith(" "):
            keep = line.startswith("bots")
        if keep:
            lines.append(line)
    if not lines:
        return ""
    return (
        "\n## The bots' waste\n\n`cargo run -p zvtes-policy --profile ci --example waste -- - 0 <records>`"
        " on the counted games: the bots' seats by profile and weights."
        + (f" {len(partial)} records skipped or read partway: {'; '.join(partial)}." if partial else "")
        + "\n\n```\n"
        + "\n".join(lines)
        + "\n```\n"
    )


def read_turns(binary, path):
    """(turns, whole): the last turn the engine reached replaying the
    record, and whether it read the record to its end (a game over, or
    left where the record ends: humans ousted, abandoned) rather than
    stopping partway (a decision this engine no longer asks or offers)."""
    try:
        out = subprocess.run([str(binary), str(path)], capture_output=True, text=True, timeout=300).stdout
    except (OSError, subprocess.TimeoutExpired):
        return None, False
    turns = [int(t) for t in re.findall(r"^#\d+ \[t(\d+)", out, re.M)]
    whole = bool(re.search(r"^over:|: the record ends here$", out, re.M))
    return (max(turns) if turns else None), whole


def decisions(record):
    """The decisions standing after the undos: [(seat, by)]."""
    kept = []
    for e in record.get("entries", []):
        if e["type"] == "decision":
            kept.append(e)
        elif e["type"] == "undo":
            kept = [d for d in kept if d["decision"] < e["to"]]
    return [(d["seat"], d["by"]) for d in kept]


def date_of(record, path):
    if record.get("started"):
        t = datetime.datetime.fromtimestamp(record["started"] / 1000, datetime.timezone.utc)
        return t.strftime("%Y-%m-%d %H:%M")
    m = re.search(r"(\d{4}-\d{2}-\d{2})T(\d{2})-(\d{2})", path.name)
    return f"{m[1]} {m[2]}:{m[3]}" if m else "?"


def bot_label(bot, spec_profile):
    """(profile, weights) of a bot seat, as the report groups them."""
    if bot is None:
        return spec_profile or "?", "unrecorded"
    if bot.get("kind") != "network":
        return bot["profile"], "scripted"
    t = bot.get("temperature")
    profile = bot["profile"] + (f"@{t:g}" if t is not None else "")
    return profile, (bot.get("weights") or "unrecorded")[:12]


def game_of(path, record, catalog, turns, args):
    seats = record["spec"]["seats"]
    bots = {b["seat"]: b for b in record.get("bots", [])}
    result = record.get("result")
    vps = [v / 2 for v in result["vp_halves"]] if result else [0.0] * len(seats)
    rows = []
    for i, s in enumerate(seats):
        human = s["player"]["type"] == "human"
        profile, weights = (None, None) if human else bot_label(bots.get(i), s["player"].get("profile"))
        rows.append(
            {
                "seat": i,
                "human": human,
                "deck": deck_name(s["deck"], catalog),
                "profile": profile,
                "weights": weights,
                "vp": vps[i] if i < len(vps) else 0.0,
            }
        )
    made = decisions(record)
    humans = [r["seat"] for r in rows if r["human"]]
    g = {
        "id": path.stem,
        "path": str(path),
        "date": date_of(record, path),
        "engine": record["engine"][:12],
        "client": record["spec"].get("client") or "?",
        "size": len(seats),
        "seats": rows,
        "winner": result.get("winner") if result else None,
        "reason": result["reason"]["type"] if result else None,
        "turns": turns[0],
        "whole": turns[1],
        "decisions": len(made),
        "human_decisions": sum(1 for s, _ in made if s in humans),
    }
    g["against"] = ", ".join(sorted({r["profile"] for r in rows if not r["human"]})) or "-"
    g["weights"] = ", ".join(sorted({r["weights"] for r in rows if not r["human"]})) or "-"
    if record.get("ended") is None or result is None:
        g["status"] = "unfinished"
    elif g["reason"] == "engine_error":
        g["status"] = "engine error"
    elif g["client"] in args.test_clients:
        g["status"] = "test client"
    elif g["reason"] == "abandoned":
        g["status"] = "abandoned"
    elif g["reason"] == "server_restart":
        g["status"] = "server restart"
    elif not humans:
        g["status"] = "no human"
    elif (g["turns"] is not None and g["whole"] and g["turns"] < args.min_rounds * g["size"]) or (
        g["turns"] is None and g["human_decisions"] < args.min_decisions
    ):
        g["status"] = "short"
    else:
        g["status"] = "counted"
    return g


# --- the report ---------------------------------------------------------


def cell(x):
    return str(x).replace("|", "\\|")


def table(head, rows):
    lines = ["| " + " | ".join(head) + " |", "|" + "---|" * len(head)]
    lines += ["| " + " | ".join(cell(x) for x in r) + " |" for r in rows]
    return "\n".join(lines)


def n_mark(n):
    return f"{n}*" if n < SMALL else str(n)


def rate(k, n):
    return f"{k / n:.0%}" if n else "-"


def mean(xs):
    return f"{sum(xs) / len(xs):.2f}" if xs else "-"


def turns_of(g):
    if g["turns"] is None:
        return "?"
    return str(g["turns"]) if g["whole"] else f"≥{g['turns']}"


def seats_of(g):
    return "; ".join(
        (f"**{r['deck']}** (human)" if r["human"] else r["deck"]) + f" {r['vp']:g}" for r in g["seats"]
    )


def human_won(g, r):
    return r["human"] and g["winner"] == r["seat"]


def report(title, games, args, listing):
    counted = [g for g in games if g["status"] == "counted"]
    apart = collections.Counter(g["status"] for g in games if g["status"] != "counted")
    out = [f"# {title}", ""]
    out.append(
        f"{len(games)} games read; {len(counted)} counted (ended normally, a human seat, not a test client, "
        f"at least {args.min_rounds} rounds — or {args.min_decisions} human decisions when the turns are unknown)"
        + "".join(f"; {n} {s}" for s, n in sorted(apart.items()))
        + f". `*`: fewer than {SMALL} in the sample. Turns: as recorded (protocol 1.7.0 on), else "
        "replayed on this checkout's engine, `≥` where it stops partway. Nudges: each network "
        "bot's in its record from 1.7.0 (before, those its server served, "
        "`crates/server/src/games.rs`)."
    )
    out.append("")

    def breakdown(key, name):
        by = collections.defaultdict(list)
        for g in counted:
            by[key(g)].append(g)
        rows = [
            (
                k,
                n_mark(len(gs)),
                sum(1 for g in gs for r in g["seats"] if r["human"]),
                sum(1 for g in gs for r in g["seats"] if not r["human"]),
            )
            for k, gs in sorted(by.items(), key=lambda kv: str(kv[0]))
        ]
        return [f"### By {name}", "", table([name, "games", "human seats", "bot seats"], rows), ""]

    out += ["## Games, humans and bots", ""]
    out += breakdown(lambda g: g["client"], "client")
    out += breakdown(lambda g: g["engine"], "engine")
    out += breakdown(lambda g: g["weights"], "weights")
    out += breakdown(lambda g: g["size"], "table size")

    out += ["## Humans against the bots", ""]
    out.append("A human seat's game wins and VPs; the bots' VPs a seat beside them.")
    out.append("")
    by = collections.defaultdict(list)
    for g in counted:
        by[(g["against"], g["weights"])].append(g)
    rows = []
    for (against, weights), gs in sorted(by.items()):
        hs = [(g, r) for g in gs for r in g["seats"] if r["human"]]
        bs = [r for g in gs for r in g["seats"] if not r["human"]]
        wins = sum(1 for g, r in hs if human_won(g, r))
        rows.append(
            (
                against,
                weights,
                n_mark(len(gs)),
                n_mark(len(hs)),
                wins,
                rate(wins, len(hs)),
                mean([r["vp"] for _, r in hs]),
                mean([r["vp"] for r in bs]),
            )
        )
    out.append(
        table(["bots", "weights", "games", "human seats", "wins", "win rate", "human VP/seat", "bot VP/seat"], rows)
    )
    out.append("")

    out += ["## Decks", "", "### The humans' decks (by wins)", ""]
    decks = collections.defaultdict(list)
    for g in counted:
        for r in g["seats"]:
            if r["human"]:
                decks[r["deck"]].append((human_won(g, r), r["vp"]))
    rows = [
        (d, n_mark(len(v)), sum(w for w, _ in v), rate(sum(w for w, _ in v), len(v)), mean([p for _, p in v]))
        for d, v in decks.items()
    ]
    rows.sort(key=lambda r: (-r[2], r[0]))
    out += [table(["deck", "seats", "wins", "win rate", "VP/seat"], rows), ""]

    out += ["### The bots' decks (by VP a seat, the weakest first)", ""]
    decks = collections.defaultdict(list)
    for g in counted:
        for r in g["seats"]:
            if not r["human"]:
                decks[r["deck"]].append((g["winner"] == r["seat"], r["vp"], any(human_won(g, h) for h in g["seats"])))
    rows = [
        (
            d,
            n_mark(len(v)),
            sum(w for w, _, _ in v),
            rate(sum(w for w, _, _ in v), len(v)),
            mean([p for _, p, _ in v]),
            sum(h for _, _, h in v),
        )
        for d, v in decks.items()
    ]
    rows.sort(key=lambda r: (float(r[4]) if r[4] != "-" else 0, r[0]))
    out += [table(["deck", "seats", "wins", "win rate", "VP/seat", "a human won"], rows), ""]

    out += [f"## Notable human games (a human won, or took {NOTABLE_VP}+ VPs)", ""]
    notable = [
        g for g in counted if any(r["human"] and (human_won(g, r) or r["vp"] >= NOTABLE_VP) for r in g["seats"])
    ]
    for g in sorted(notable, key=lambda g: g["date"]):
        h = [r for r in g["seats"] if r["human"]]
        what = ", ".join(f"{'won' if human_won(g, r) else 'no win'}, {r['vp']:g} VP" for r in h)
        out.append(
            f"- `{g['id']}` ({g['date']}, engine `{g['engine']}`, bots {g['against']} `{g['weights']}`): "
            f"{what}, {turns_of(g)} turns. Seats: {seats_of(g)}.  \n"
            f"  `cargo run -p zvtes-host --profile ci --example replay -- {g['path']} --options`"
        )
    if not notable:
        out.append("None.")
    out.append("")

    if listing:
        out += ["## Every game", ""]
        rows = [
            (
                g["id"],
                g["date"],
                g["client"],
                g["engine"],
                g["weights"],
                g["against"],
                g["size"],
                seats_of(g),
                g["winner"] if g["winner"] is not None else "-",
                turns_of(g),
                g["reason"] or "-",
                g["decisions"],
                g["status"],
            )
            for g in sorted(games, key=lambda g: g["date"])
        ]
        out.append(
            table(
                [
                    "id",
                    "date (UTC)",
                    "client",
                    "engine",
                    "weights",
                    "bots",
                    "seats",
                    "decks in seat order, VPs",
                    "winner",
                    "turns",
                    "end",
                    "decisions",
                    "status",
                ],
                rows,
            )
        )
        out.append("")
    return "\n".join(out)


def main():
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("dir", nargs="?", default="data/server-records")
    p.add_argument("--catalog", default=CATALOG_URL, help="the deck catalog: a file or a URL")
    p.add_argument("--date", default=datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d"))
    p.add_argument("--no-turns", action="store_true", help="do not replay the records for their turns")
    p.add_argument("--no-waste", action="store_true", help="no section of the bots' waste")
    p.add_argument(
        "--list", type=pathlib.Path, help="write the counted games' paths there, absolute and comma-separated (--bc, --starts)"
    )
    p.add_argument("--min-rounds", type=int, default=2)
    p.add_argument("--min-decisions", type=int, default=30)
    p.add_argument(
        "--test-clients",
        default="client.mjs,load.mjs",
        help="clients whose games are counted apart (the server's test scripts)",
    )
    args = p.parse_args()
    args.test_clients = set(filter(None, args.test_clients.split(",")))
    root = pathlib.Path(args.dir)
    reports = root / "reports"
    reports.mkdir(parents=True, exist_ok=True)
    index_path = reports / "index.json"
    index = json.loads(index_path.read_text()) if index_path.exists() else {}
    catalog = load_catalog(args.catalog, reports / "catalog.json")

    binary = None
    games = []
    for path in sorted(root.glob("*.json")):
        record = json.loads(path.read_text())
        if "spec" not in record or "entries" not in record:
            continue
        seen = index.get(path.stem, {})
        finished = record.get("ended") is not None and record.get("result") is not None
        turns = (None, False)
        recorded = record["result"].get("turns") if finished else None
        if recorded is not None:
            # Its end turn as recorded; read partway when a restart's
            # replay stopped (its failure says so).
            turns = (recorded, record["result"].get("failure") is None)
        elif not args.no_turns and finished:
            if seen.get("entries") == len(record["entries"]) and "turns" in seen:
                turns = (seen["turns"], seen["whole"])
            else:
                binary = binary or replayer() or False
                if binary:
                    turns = read_turns(binary, path)
        g = game_of(path, record, catalog, turns, args)
        if finished:
            entry = {"date": seen.get("date", args.date), "entries": len(record["entries"])}
            if turns[0] is not None:
                entry.update(turns=turns[0], whole=turns[1])
            index[path.stem] = entry
            g["first"] = entry["date"]
        else:
            g["first"] = None
        games.append(g)
    index_path.write_text(json.dumps(index, indent=1, sort_keys=True) + "\n")

    run = [g for g in games if g["first"] == args.date or g["first"] is None]
    day = reports / f"{args.date}.md"
    total = reports / "total.md"
    if args.list:
        args.list.write_text(
            ",".join(str(pathlib.Path(g["path"]).resolve()) for g in games if g["status"] == "counted")
        )

    def section(gs):
        return "" if args.no_waste else waste([g["path"] for g in gs if g["status"] == "counted"])

    if run or day.exists():
        day.write_text(
            report(f"Online games — {args.date} (the games first read that day)", run, args, True) + section(run)
        )
    total.write_text(report(f"Online games — total (to {args.date})", games, args, False) + section(games))

    for name, gs in (("this run", run), ("total", games)):
        counted = [g for g in gs if g["status"] == "counted"]
        hs = [(g, r) for g in counted for r in g["seats"] if r["human"]]
        wins = sum(1 for g, r in hs if human_won(g, r))
        apart = collections.Counter(g["status"] for g in gs if g["status"] != "counted")
        print(
            f"{name}: {len(gs)} games, {len(counted)} counted"
            + "".join(f", {n} {s}" for s, n in sorted(apart.items()))
            + f"; humans won {wins} of {len(hs)} seats ({rate(wins, len(hs))}), "
            f"{mean([r['vp'] for _, r in hs])} VP a seat"
        )
    print(f"reports: {day if day.exists() else '(no new game)'}, {total}")


if __name__ == "__main__":
    main()
