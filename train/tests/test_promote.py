"""Promotion of harvested games to the corpus (`scripts/promote.py`; the
training skill's "Promotion", data/records/README.md): a game played out
with a human is copied and listed; one failing the stranger's check, not
played out, a test client's or in the corpus already is refused."""

import copy
import importlib.util
import json
import pathlib
import shutil
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("promote", REPO / "scripts/promote.py")
promote = importlib.util.module_from_spec(spec)
spec.loader.exec_module(promote)

GAME = "5a98a36c86873c35"
RECORD = json.loads((REPO / f"data/records/{GAME}.json").read_text())
# The three bots' decks as the server's catalog names them (no network);
# the human's deck is custom.
NAMES = [None, "Nosferatu (Fifth Edition)", "The Endless Dance (Thirtieth Anniversary)",
         "Hecata (Fifth Edition)"]
CATALOG = [(n, promote.games_report.clean(s["deck"])) for n, s in zip(NAMES, RECORD["spec"]["seats"]) if n]


def changed(f):
    r = copy.deepcopy(RECORD)
    f(r)
    return r


def test_the_stranger_check():
    assert promote.refusals(RECORD, catalog=CATALOG) == []
    feedback = {"type": "feedback", "seat": 0, "at": 1, "decision": 3, "text": "mail me at a@b.c"}
    cases = [
        (lambda r: r["spec"]["seats"][0].update(name="Alice Martin"), "text: spec.seats[0].name ('Alice Martin')"),
        (lambda r: r["spec"]["seats"][1].update(name="Bot1 - call me"), "text: spec.seats[1].name ('Bot1 - call me')"),
        (lambda r: r["spec"].update(client="alice-client"), "text: spec.client ('alice-client')"),
        (lambda r: r["entries"].append(feedback), "text: a feedback entry's text at decision 3"),
        (lambda r: r["spec"]["seats"][0]["player"].update(account="a@b.c"), "an unknown field: spec.seats[0].player.account"),
        (lambda r: r.update(chat=[]), "an unknown field: chat"),
        (lambda r: r["entries"][0].update(note="hi"), "an unknown field: entries[0].note"),
        (lambda r: r["bots"][0].update(owner="alice"), "an unknown field: bots[0].owner"),
        (lambda r: r["result"].update(failure="Alice left"), "text: result.failure"),
        (lambda r: r["entries"].append({"type": "chat", "text": "hi"}), "an unknown entry: chat"),
        (lambda r: r["entries"].append({"type": "player", "type_": 1}), "an unknown entry: player"),
        (lambda r: r["entries"].append({"type": "refused", "seat": 0, "at": 1,
                                         "error": {"error": "malformed", "reason": "Alice here"}}), "text: entries["),
        (lambda r: r["result"].update(reason={"type": "abandoned", "seat": 0}), "not played out: abandoned"),
        (lambda r: r["result"].update(reason={"type": "humans_ousted"}), "not played out: humans_ousted"),
        (lambda r: r.pop("result"), "not finished"),
        (lambda r: r["spec"].update(client="client.mjs"), "a test client's: client.mjs"),
        (lambda r: r["spec"].update(client="succubus-club-demo"), "a test client's: succubus-club-demo"),
        (lambda r: r["spec"]["seats"][0]["player"].update(type="bot", profile="network"), "no human seat"),
    ]
    for f, why in cases:
        assert any(w.startswith(why) for w in promote.refusals(changed(f), catalog=CATALOG)), why
    # The host's own refusal text is not a player's.
    host = {"type": "refused", "seat": 0, "at": 1, "error": {"error": "malformed", "reason": "feedback: 1 to 2000 characters"}}
    assert promote.refusals(changed(lambda r: r["entries"].append(host)), catalog=CATALOG) == []
    named = changed(cases[0][0])
    assert promote.refusals(named, allow_text=True, catalog=CATALOG) == [], "the owner read it"


def test_a_scripted_bots_row():
    # A scripted bot records no weights nor temperature: named by profile.
    def scripted(r):
        r["bots"][0] = {"seat": 1, "profile": "wall", "kind": "scripted", "seed": 1}
    line = promote.row(GAME, changed(scripted), CATALOG)
    assert "wall (scripted)" in line and "None" not in line


def test_promotes_and_refuses(tmp_path):
    source, corpus = tmp_path / "harvest", tmp_path / "records"
    source.mkdir()
    corpus.mkdir()
    readme = (REPO / "data/records/README.md").read_text()
    shutil.copyfile(REPO / f"data/records/{GAME}.json", source / f"{GAME}.json")
    bad = changed(lambda r: r["entries"].append({"type": "feedback", "seat": 0, "at": 1, "decision": 3, "text": "hello"}))
    (source / "bad.json").write_text(json.dumps(bad))
    rows = [l for l in readme.split("\n") if l.startswith(f"| `{GAME}.json`")]
    (corpus / "README.md").write_text(readme.replace(rows[0] + "\n", ""))
    (tmp_path / "catalog.json").write_text(json.dumps([[n, d] for n, d in CATALOG]))

    def run(*games):
        return subprocess.run(
            [sys.executable, str(REPO / "scripts/promote.py"), *games, "--from", str(source), "--to", str(corpus),
             "--catalog", str(tmp_path / "catalog.json")],
            capture_output=True, text=True, cwd=REPO,
        )

    out = run(GAME, "bad")
    assert out.returncode == 1, out.stdout + out.stderr
    assert f"{GAME}: promoted" in out.stdout and "bad: refused — text: a feedback entry's text" in out.stdout
    assert (corpus / f"{GAME}.json").read_text() == (source / f"{GAME}.json").read_text()
    assert not (corpus / "bad.json").exists()
    assert rows[0] in (corpus / "README.md").read_text(), "its row listed"
    again = run(GAME)
    assert again.returncode == 1 and "in the corpus already" in again.stdout
