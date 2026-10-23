#!/usr/bin/env python3
"""Promote harvested games to the corpus (the training skill's
"Promotion"; data/records/README.md): each record is checked, then copied
to `data/records/<game id>.json` and its row added to the README's table
of the server's records.

Usage: python3 scripts/promote.py GAME [GAME ...] [--from data/server-records]
    [--to data/records] [--catalog FILE|URL] [--allow-text] [--check]

GAME is a game id (`<id>.json` in --from) or a record's path. A record is
refused, with every reason, when:
- it is not a game played out with a human: no result, abandoned, an
  engine error, a server restart, the humans ousted (the corpus reads each
  game to its end: `crates/host/tests/records.rs`), a test client's
  (`spec.client`), or no human seat;
- it holds what a stranger's record should not carry into the repository
  (the stranger's check): a field this script does not know, anywhere in
  the record (a later protocol may add one that names a player); text a
  client or player wrote — a seat name other than the server's ("human",
  "Bot N") or Succubus Club's ("BotN - <its deck>"), a client other than a
  known one, a feedback entry's text, any other string that is not an id
  — unless `--allow-text`, once the owner has read them;
- this checkout's engine does not read it to its recorded end (the replay
  example: the result's VPs); `cargo test -p zvtes-host --test records`
  stays the gate;
- the corpus has it already.
`--check` checks and copies nothing.
"""

import argparse
import datetime
import json
import pathlib
import re
import shutil
import subprocess
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import games_report  # noqa: E402

REPO = games_report.REPO
# A record's fields (docs/protocol.md §11, crates/api/src/record.rs) by
# where they sit: any other is refused as unknown. A decision's `choice`
# (the engine's key) and a seat's `deck` (card ids) are not walked.
FIELDS = {
    "record": {"protocol", "engine", "spec", "started", "ended", "entries", "result", "bots"},
    "spec": {"client", "seats", "seed", "time_limit"},
    "seat": {"deck", "name", "player"},
    "player": {"type", "profile", "settings"},
    "settings": {"opportunities", "views", "text"},
    "bot": {"seat", "profile", "kind", "weights", "temperature", "seed", "nudges"},
    "result": {"vp_halves", "winner", "reason", "failure", "turns"},
    "reason": {"type", "seat"},
}
# An entry's fields by its type.
ENTRIES = {
    "decision": {"type", "decision", "seat", "at", "option", "choice", "by", "forced", "shown", "window", "digest"},
    "undo": {"type", "seat", "at", "to"},
    "settings": {"type", "seat", "at", "settings"},
    "refused": {"type", "seat", "at", "error"},
    "feedback": {"type", "seat", "at", "decision", "text"},
}
# The text the host itself writes in a refused entry's error (a
# feedback's length, `crates/host/src/game.rs`): not a player's.
HOST_TEXT = re.compile(r"^feedback: 1 to \d+ characters$")
# The clients whose records are players' (the others: unknown text).
CLIENTS = {"succubus-club"}
# The ends of a game played out (also `train/ppo.py`'s).
PLAYED_OUT = {"last_standing", "time_limit"}
# A string that names nothing a person wrote: ids, hashes, versions.
ID = re.compile(r"^[a-z0-9_.\-]*$")


def unknown(obj, kind, where, fields=FIELDS):
    """The fields of `obj` that `kind` does not have."""
    return [f"an unknown field: {where}.{k}" if where else f"an unknown field: {k}" for k in sorted(set(obj) - fields[kind])]


def texts(obj, where):
    """The strings under `obj` that are not ids, with where they are."""
    if isinstance(obj, str):
        return [] if ID.match(obj) or HOST_TEXT.match(obj) else [where]
    if isinstance(obj, dict):
        return [w for k, v in obj.items() for w in texts(v, f"{where}.{k}")]
    if isinstance(obj, list):
        return [w for i, v in enumerate(obj) for w in texts(v, f"{where}[{i}]")]
    return []


def seat_name_ok(seat, catalog):
    if seat.get("player", {}).get("type") == "human":
        return seat.get("name") == "human"
    name = seat.get("name", "")
    if re.fullmatch(r"Bot \d", name):
        return True
    m = re.fullmatch(r"Bot\d - (.+)", name)
    return bool(m) and m.group(1) == short_deck_name(games_report.deck_name(seat["deck"], catalog))


def short_deck_name(name):
    """A catalog name as Succubus Club names a bot's seat: the deck's
    name, its set left out ("Hecata (Fifth Edition)" → "Hecata")."""
    return re.sub(r" \((?:[^()]|\([^()]*\))*\)$", "", name)


def refusals(record, allow_text=False, catalog=(), test_clients=games_report.TEST_CLIENTS):
    """Why `record` cannot be promoted (empty: it can, but for its replay)."""
    why = unknown(record, "record", "")
    spec = record.get("spec", {})
    why += unknown(spec, "spec", "spec")
    seats = spec.get("seats", [])
    text = []
    for i, s in enumerate(seats):
        why += unknown(s, "seat", f"spec.seats[{i}]")
        player = s.get("player", {})
        why += unknown(player, "player", f"spec.seats[{i}].player")
        why += unknown(player.get("settings", {}), "settings", f"spec.seats[{i}].player.settings")
        text += texts(player, f"spec.seats[{i}].player")
        if not seat_name_ok(s, catalog):
            text.append(f"spec.seats[{i}].name ({s.get('name')!r})")
    client = spec.get("client")
    if client is not None and client not in CLIENTS and client not in test_clients:
        text.append(f"spec.client ({client!r})")
    for i, b in enumerate(record.get("bots", [])):
        why += unknown(b, "bot", f"bots[{i}]")
        text += texts(b, f"bots[{i}]")
    result = record.get("result") or {}
    why += unknown(result, "result", "result")
    why += unknown(result.get("reason", {}), "reason", "result.reason")
    text += texts(result, "result")
    for i, e in enumerate(record.get("entries", [])):
        kind = e.get("type")
        if kind not in ENTRIES:
            why.append(f"an unknown entry: {kind}")
            continue
        why += unknown(e, kind, f"entries[{i}]", ENTRIES)
        if kind == "feedback":
            if e.get("text"):
                text.append(f"a feedback entry's text at decision {e.get('decision')}")
        else:
            text += texts({k: v for k, v in e.items() if k != "choice"}, f"entries[{i}]")
    if text and not allow_text:
        why += [f"text: {t}" for t in text]
    reason = result.get("reason", {}).get("type")
    if record.get("ended") is None or not result:
        why.append("not finished")
    elif reason not in PLAYED_OUT:
        why.append(f"not played out: {reason}")
    if client in test_clients:
        why.append(f"a test client's: {client}")
    if not any(s.get("player", {}).get("type") == "human" for s in seats):
        why.append("no human seat")
    return sorted(set(why))


def reads_whole(binary, path, record):
    """Whether this checkout's engine reads the record to its recorded end:
    the replay example's last line, the result's VPs."""
    out = subprocess.run([str(binary), str(path)], capture_output=True, text=True, timeout=300).stdout
    names = [s["name"] for s in record["spec"]["seats"]]
    vps = [h / 2 for h in record["result"]["vp_halves"]]
    over = "over: " + ", ".join(f"{n} {v:g}" for n, v in zip(names, vps))
    return over in out.splitlines()


def row(game, record, catalog):
    """The README's table row for a promoted record."""
    seats = record["spec"]["seats"]
    vps = [v / 2 for v in record["result"]["vp_halves"]]
    decks = [games_report.deck_name(s["deck"], catalog) for s in seats]
    human = [s["player"]["type"] == "human" for s in seats]
    bots = sorted(
        {
            f"`{b['weights'][:12]}` ({b['kind']}"
            + (f", t {b['temperature']:g})" if b.get("temperature") is not None else ")")
            if b.get("weights")
            else f"{b['profile']} ({b['kind']})"
            for b in record.get("bots", [])
        }
    )
    when = datetime.datetime.fromtimestamp(record["started"] / 1000, datetime.timezone.utc).strftime("%Y-%m-%d")
    scored = [f"{'the human' if h else d} {v:g} VP" for d, h, v in zip(decks, human, vps) if v]
    undos = sum(1 for e in record["entries"] if e.get("type") == "undo")
    result = ", ".join(scored) + f", {record['result']['reason']['type'].replace('_', ' ')}"
    if undos:
        result += f"; {undos} undo{'s' if undos > 1 else ''}"
    seated = ", ".join(f"**{d}**" if h else d for d, h in zip(decks, human))
    return (
        f"| `{game}.json` | `{record['engine'][:12]}` (protocol {record['protocol']}), {when} "
        f"| {seated} | {', '.join(bots)} | {result} |"
    )


def add_row(readme, line):
    """`line` appended to the README's table of the server's records."""
    lines = readme.read_text().split("\n")
    head = next(i for i, l in enumerate(lines) if l.startswith("The server's records promoted"))
    end = next(i for i in range(head + 3, len(lines)) if not lines[i].startswith("|"))
    lines.insert(end, line)
    readme.write_text("\n".join(lines))


def main():
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("games", nargs="+")
    p.add_argument("--from", dest="source", type=pathlib.Path, default=REPO / "data/server-records")
    p.add_argument("--to", type=pathlib.Path, default=REPO / "data/records")
    p.add_argument("--catalog", default=games_report.CATALOG_URL, help="the deck catalog naming the decks: a file or a URL")
    p.add_argument("--allow-text", action="store_true", help="the owner has read the record's names and texts")
    p.add_argument("--check", action="store_true", help="check only, copy nothing")
    args = p.parse_args()
    catalog = games_report.load_catalog(args.catalog, args.source / "reports" / "catalog.json")
    binary = None
    failed = False
    for g in args.games:
        path = pathlib.Path(g) if g.endswith(".json") else args.source / f"{g}.json"
        game = path.stem
        record = json.loads(path.read_text())
        why = refusals(record, args.allow_text, catalog)
        if (args.to / f"{game}.json").exists():
            why.append("in the corpus already")
        if not why:
            binary = binary or games_report.replayer()
            if not binary or not reads_whole(binary, path, record):
                why.append("this engine does not read it to its recorded end")
        if why:
            failed = True
            print(f"{game}: refused — " + "; ".join(why))
            continue
        if args.check:
            print(f"{game}: can be promoted")
            continue
        line = row(game, record, catalog)
        shutil.copyfile(path, args.to / f"{game}.json")
        add_row(args.to / "README.md", line)
        print(f"{game}: promoted to {args.to / (game + '.json')}")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
