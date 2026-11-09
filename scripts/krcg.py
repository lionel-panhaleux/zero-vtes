"""KRCG data in its v5 schema (https://static.krcg.org/data/): the loaders
and the card fields the generators read, so their logic stays schema-free.

`vtes.json` is a list of cards, `twda.json` a dict of decks by id;
`expansions.json` (under `v5/` only) names the sets and their bundles (a
precon is a bundle). A path given offline looks for `expansions.json`
beside it, else downloads it. The old schema stays under `data/v4/`.
"""
import json
import os
import re
import urllib.request

ROOT_URL = "https://static.krcg.org/data/"
EXPANSIONS_URL = ROOT_URL + "v5/expansions.json"


def _load(path, url):
    if path:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    with urllib.request.urlopen(url) as f:
        return json.load(f)


def load_cards(path=None):
    """vtes.json (downloaded when no path is given), each print's set given
    its `name` and `release_date` and each occurrence its `bundle_name`
    from the expansions, for `sets`."""
    cards = _load(path, ROOT_URL + "vtes.json")
    beside = path and os.path.join(os.path.dirname(os.path.abspath(path)), "expansions.json")
    exp = _load(beside if beside and os.path.exists(beside) else None, EXPANSIONS_URL)
    exp = {e["code"]: e for e in exp}
    for c in cards:
        for p in c["prints"]:
            e = exp[p["set"]["code"]]
            p["set"]["name"], p["set"]["release_date"] = e["name"], e["release_date"]
            for o in p["occurrences"]:
                o["bundle_name"] = e["bundles"].get(o["bundle"], {}).get("name") or ""
    return cards


def load_twda(path=None):
    """twda.json as a list in archive order, each deck flattened to {id,
    name, event, date, players_count, crypt, library}: crypt and library
    lists of {id, count}."""
    decks = _load(path, ROOT_URL + "twda.json")
    out = []
    for d in decks.values():
        def side(kind):
            return [{"id": c["id"], "count": c["count"]} for c in d["cards"] if c["kind"] == kind]

        event = d.get("event") or {}
        out.append({
            "id": d["id"], "name": d.get("name") or "", "event": event.get("name") or "",
            "date": event.get("date") or "", "players_count": event.get("players_count") or 0,
            "crypt": side("Crypt"), "library": side("Library"),
        })
    return out


def name(card):
    """The table name (KRCG v4's `_name`): the printed name, a leading
    article moved to the end ("Ankou, The"), ™ spelled (TM)."""
    n = card["printed_name"].replace("™", "(TM)")
    m = re.match(r"^(The|An) (.*)$", n)
    return f"{m.group(2)}, {m.group(1)}" if m else n


def full_name(card):
    """The unique name (KRCG v4's `name`): the printed name and its suffix,
    "(G2 ADV)", "(ANY)" for a vampire of any group; a leading "An" moved
    to the end ("Anarch Manifesto, An"), not "The"."""
    s = card["suffix"] or ("ANY" if is_crypt(card) and card.get("group") == "Any" else "")
    n = re.sub(r"^An (.*)$", r"\1, An", card["printed_name"])
    return n + (f" ({s})" if s else "")


def is_crypt(card):
    return card["kind"] == "Crypt"


def text(card):
    """The card text, its card references plain ("<Blood Doll>": KRCG v5's
    markup)."""
    return re.sub(r"<([^<>]*)>", r"\1", card.get("text", ""))


def clans(card):
    """A vampire's clan, a library card's clan requirement."""
    if is_crypt(card):
        return [card["clan"]] if card.get("clan") else []
    return list(card.get("clan_requirement", []))


def disciplines(card):
    """A vampire's Disciplines, a library card's requirement, as printed."""
    if is_crypt(card):
        return list(card.get("disciplines", []))
    req = (card.get("discipline_requirement") or {}).get("disciplines", [])
    # Flight requirements spelled as the engine and the card text do.
    return ["FLIGHT" if d == "fli" else d for d in req]


def combo(card):
    """Every listed Discipline required (KRCG `Combo`)."""
    return (card.get("discipline_requirement") or {}).get("type") == "Combo"


def group(card):
    """The crypt group, 0 for any group."""
    g = card["group"]
    return 0 if g == "Any" else int(g.lstrip("G"))


def path(card):
    """A vampire's path, a library card's path requirement; None if none."""
    if is_crypt(card):
        return card.get("path") or None
    req = card.get("path_requirement") or []
    return req[0] if req else None


def cost(card, kind):
    """The printed cost of `kind` (Blood, Pool, Conviction): an int, "X", or
    None."""
    c = card.get("cost")
    return c["value"] if c and c["type"] == kind else None


def set_codes(card):
    """The codes of the sets the card is printed in ("V5", "EoG")."""
    return {p["set"]["code"] for p in card["prints"] if p["occurrences"]}


def in_bundle(card, codes):
    """Printed in a bundle (a precon "PTr") of one of the `codes` sets."""
    return any(p["set"]["code"] in codes and o["bundle"].startswith("P")
               for p in card["prints"] for o in p["occurrences"])


def sets(card, packs=()):
    """{set name: [{release_date, copies, precon (a bundle's name)}]}, one
    entry an occurrence; the `packs` bundles, (set code, bundle code) pairs
    typed "Precon" but sold as card packs, no deck, name no precon."""
    out = {}
    for p in card["prints"]:
        if not p["occurrences"]:
            continue
        entries = out.setdefault(p["set"]["name"], [])
        for o in p["occurrences"]:
            e = {"release_date": o["date"] or p["set"]["release_date"]}
            if o["type"] == "Precon" and o["bundle_name"] and (p["set"]["code"], o["bundle"]) not in packs:
                e["precon"] = o["bundle_name"]
            if o["copies"]:
                e["copies"] = o["copies"]
            entries.append(e)
    return out
