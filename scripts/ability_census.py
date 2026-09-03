#!/usr/bin/env python3
"""Sizing census of in-play abilities on the V5 permanents.

For every library card that stays in play (master, ally, equipment, retainer,
event, or "put this card in play/on"), split the card text into sentences and
tag each with the ability *kinds* it exhibits. This is a sizing tool for the
effects framework (DESIGN §4.5), not a reviewed matrix: the regexes are broad
(`continuous` in particular over-counts) and the numbers must not be quoted
as facts. Subtype labels ("Unique location.", "Weapon: gun.", "Trifle.") are
tagged `label` — they are static data the rules key on, not abilities.

Usage: python3 scripts/ability_census.py [path/to/vtes.json] > data/ability-census-draft.csv
Reads data/window-matrix-reviewed.csv for the card list and in_play_action flag.
"""
import csv
import json
import re
import sys
import urllib.request

KINDS = [
    ("label", r"^(unique( \w+)?|weapon(: \w+)?|vehicle|archetype|trifle|gehenna|hunting ground|"
              r"title|out-of-turn|\w+ location|(unique )?(mortal|ghoul|zombie|wraith|animal|werewolf|mage|demon|"
              r"changeling|hunter|\w+) with \d+ life[^.]*|\d strength, \d bleed|requires [^.]*)\.$"),
    ("cancel_as_played", r"cancel [^.]{0,80}?as it is (being )?played"),
    ("granted_action", r"\bas an? [^.;]{0,40}?\baction\b(?! to )(?![^.]* is announced)|can call a referendum"),
    ("lock_ability", r"\block (this|and burn this|\w+ to\b)|can lock (this|\w+ to)|lock this (card|location|equipment|ally|retainer)|lock (during|to)|you can lock"),
    ("burn_ability", r"burn (this|it) (card|ally|retainer|location|equipment)?( \(even if[^)]*\))? ?to|can burn (this|it|1 life|\d blood|{name})[^.]* to|burn {name} to"),
    ("combat_grant", r"\b\d (optional )?(maneuver|press)|can prevent|strikes?( with)?:|additional strike|\bdodge\b|\bmaneuver\b|\bpress\b|prevent \d"),
    ("phase_trigger", r"during (your|each|their|this|any|the)( \w+('s)?)? (unlock|master|minion|influence|discard) phase|during (your|each|their) unlock|at the (start|end) of"),
    ("event_trigger", r"\bafter (this|a|an|the|each|any|you)\b|\bwhen(ever)? \b|\bif (this|the attached|the bearer|a|an|you) [^.]*? (is|are|would be|becomes?|enters?|leaves?|successfully|goes|performs?)"),
    ("continuous", r"\bgets? [+-]\d|\bget [+-]\d|[+-]\d (stealth|intercept|bleed|strength|vote|hand size|capacity|damage)|cannot (block|be blocked|bleed|hunt|take|perform|be the target|attempt|play|use)|do(es)? not unlock|costs? [^.]*?(\+|-)?\d (more|less|blood|pool)|treated as|is considered|counts as|\bas if\b"),
    ("replacement", r"\binstead\b|\bwould\b"),
    ("counters", r"\bcounters?\b"),
    ("search", r"\bsearch\b"),
]


def main():
    if len(sys.argv) > 1:
        cards = json.load(open(sys.argv[1]))
    else:
        with urllib.request.urlopen("https://static.krcg.org/data/vtes.json") as f:
            cards = json.load(f)
    cards = {c["id"]: c for c in cards}
    rows = list(csv.DictReader(open("data/window-matrix-reviewed.csv", newline="")))
    out = csv.writer(sys.stdout)
    out.writerow(["id", "name", "types", "flags", "kinds", "sentence"])
    n = 0
    for r in rows:
        c = cards[int(r["id"])]
        text = c["card_text"]
        types = set(r["types"].split("/"))
        permanent = bool(types & {"Master", "Ally", "Equipment", "Retainer", "Event"}) \
            or re.search(r"put this card (in play|on)", text, re.I)
        if not permanent:
            continue
        n += 1
        name = re.escape(c["_name"].split(",")[0])
        for s in re.split(r"(?<=[.])\s+(?=[A-Z\[])", text.replace("\n", " ")):
            s = s.strip()
            if not s:
                continue
            kinds = [k for k, p in KINDS if re.search(p.replace("{name}", name), s, re.I)]
            if "label" in kinds:
                kinds = ["label"]
            out.writerow([r["id"], r["name"], r["types"], r["flags"],
                          ";".join(kinds) or "-", s])
    print(f"# {n} permanents", file=sys.stderr)


if __name__ == "__main__":
    main()
