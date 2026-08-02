#!/usr/bin/env python3
"""Draft window matrix for the V5 pool.

Classifies each library card's initial play window (and extra window flags)
from KRCG card data, by type and text patterns. This is a *draft* generator:
its output seeds the hand-reviewed per-card implementation matrix (P1).

Usage: python3 scripts/window_matrix.py [path/to/vtes.json] > data/window-matrix-draft.csv
Downloads https://static.krcg.org/data/vtes.json if no path is given.
"""
import csv
import json
import re
import sys
import urllib.request

V5_SETS = {
    "Fifth Edition": "V5",
    "Fifth Edition (Anarch)": "V5A",
    "Fifth Edition (Companion)": "V5C",
    "New Blood": "NB",
    "New Blood II": "NB2",
    "New Blood III": "NB3",
    "Fall of London": "FoL",
    "Shadows of Berlin": "SoB",
    "Echoes of Gehenna": "EoG",
}

COMBAT_WINDOWS = [
    (r"before range is determined|played before range", "COMBAT_BEFORE_RANGE"),
    (r"\bmaneuver\b", "COMBAT_DETERMINE_RANGE"),
    (r"\bgrapple\b|before strikes are chosen", "COMBAT_BEFORE_STRIKES"),
    (r"additional strike", "COMBAT_ADDITIONAL_STRIKE"),
    (r"\bstrike:", "COMBAT_STRIKE_DECLARATION"),
    (r"\bprevent\b", "COMBAT_DAMAGE_PREVENTION"),
    (r"\bpress\b", "COMBAT_PRESS"),
    (r"end of (the )?round", "COMBAT_END_OF_ROUND"),
]

MODIFIER_WINDOWS = [
    (r"as the action is announced", "ACTION_AS_ANNOUNCED"),
    (r"\+\d+ stealth|stealth \+", "ACTION_STEALTH_INTERCEPT"),
    (r"referendum|polling|votes?\b", "REFERENDUM"),
    (r"after (the )?action resolves|after resolution|if the action (was|is) successful",
     "ACTION_AFTER_RESOLUTION"),
]

REACTION_WINDOWS = [
    (r"wakes?\b|reacts? as if unlocked", "WAKE"),
    (r"is now bleeding|change the target of the bleed", "ACTION_BLOCKS_DECLINED"),
    (r"as the action is announced|cancel (an?|the|that) action", "ACTION_AS_ANNOUNCED"),
    (r"\+\d+ intercept|intercept \+", "ACTION_STEALTH_INTERCEPT"),
    (r"referendum|polling|votes?\b", "REFERENDUM"),
]

FLAGS = [
    (r"do not replace", "delayed_replacement"),
    (r"out-of-turn", "out_of_turn"),
    (r"has cast (a )?(votes?|ballots?)", "refers_votes_cast"),
    (r"as if (the action were |it were )?unblocked|continue the action", "continues_action"),
    (r"at random", "randomness"),
    (r"becomes a .*ally|put .* in play.*ally", "token_entity"),
    (r"cancel .{0,40}as it is (being )?played", "cancel_as_played"),
]


def windows_for(card):
    types = card.get("types", [])
    text = card.get("card_text", "")
    wins = []

    def probe(table, default):
        hits = [w for pat, w in table if re.search(pat, text, re.I)]
        return hits or [default]

    if "Master" in types:
        wins.append("OUT_OF_TURN" if re.search(r"out-of-turn", text, re.I)
                    else "MASTER_PHASE")
    if "Event" in types:
        wins.append("DISCARD_PHASE")
    if "Political Action" in types:
        wins.append("ACTION_DECLARE_POLITICAL")
    if "Action" in types:
        wins.append("ACTION_DECLARE")
    if "Equipment" in types:
        wins.append("ACTION_DECLARE_EQUIP")
    if "Ally" in types:
        wins.append("ACTION_DECLARE_RECRUIT")
    if "Retainer" in types:
        wins.append("ACTION_DECLARE_EMPLOY")
    if "Combat" in types:
        wins += probe(COMBAT_WINDOWS, "COMBAT_UNCLASSIFIED")
    if "Action Modifier" in types:
        wins += probe(MODIFIER_WINDOWS, "ACTION_DURING")
    if "Reaction" in types:
        wins += probe(REACTION_WINDOWS, "REACTION_UNCLASSIFIED")
    return list(dict.fromkeys(wins))


def main():
    if len(sys.argv) > 1:
        cards = json.load(open(sys.argv[1]))
    else:
        with urllib.request.urlopen("https://static.krcg.org/data/vtes.json") as f:
            cards = json.load(f)

    codes = set(V5_SETS.values())
    out = csv.writer(sys.stdout)
    out.writerow(["id", "name", "types", "disciplines", "in_precon",
                  "windows", "flags", "needs_review"])
    n = reviews = 0
    for card in sorted(cards, key=lambda c: c.get("name", "")):
        if not V5_SETS.keys() & set(card.get("ordered_sets", [])):
            continue
        if "Vampire" in card.get("types", []):
            continue
        in_precon = any(
            p.split(":")[0] in codes and re.match(r"^P", p.split(":", 1)[1])
            for p in card.get("_set", "").split(", ") if ":" in p
        )
        wins = windows_for(card)
        flags = [f for pat, f in FLAGS
                 if re.search(pat, card.get("card_text", ""), re.I)]
        review = any("UNCLASSIFIED" in w for w in wins) or len(wins) > 2
        n += 1
        reviews += review
        out.writerow([card["id"], card["name"], "/".join(card.get("types", [])),
                      "/".join(card.get("disciplines", [])),
                      "yes" if in_precon else "no",
                      ";".join(wins), ";".join(flags), "yes" if review else ""])
    print(f"# {n} library cards, {reviews} flagged for review", file=sys.stderr)


if __name__ == "__main__":
    main()
