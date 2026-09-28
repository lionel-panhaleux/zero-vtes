#!/usr/bin/env python3
"""Draft window matrix for the V5 pool.

Classifies each library card's play window(s) and flags from KRCG card data,
by type and text patterns. This is a *draft* generator: its output seeds the
hand-reviewed per-card matrix (`data/window-matrix-reviewed.csv`); the
conventions encoded here are the owner's window arbitration (`docs/rules-qa.md`
Batch 2).

Classification is per *mode*: card text is split into a preamble (shared
restrictions such as "Only usable during a bleed action.") and one block per
discipline/type line (`[ani] …`, `[ANI] …`, `[pro] [COMBAT] …`). Each mode is
classified on preamble+mode text; "As above" modes inherit the previous mode's
windows; rider phrases that fire after the card's own play step ("with 1
optional press", "gets 1 optional maneuver", granted "can strike:"/"can
prevent") are stripped before matching so they never open a window. Windows
are the union over modes.

Within a mode, an explicit *timing* window ("as the action is announced",
"if a minion attempts to block", "during the polling step", …) suppresses the
generic stealth/intercept/bleed-modifier windows of the same mode: the printed
timing is the legality filter.

Usage: python3 scripts/window_matrix.py [path/to/vtes.json] > draft.csv
       python3 scripts/window_matrix.py [path/to/vtes.json] --check data/window-matrix-reviewed.csv
Downloads https://static.krcg.org/data/vtes.json if no path is given.
`--check` prints every card whose generated windows/flags differ from the
reviewed matrix and exits non-zero if any do (convergence is expected to hold:
the generator encodes the review's conventions).
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

# Legacy cards the owner added to the pool (2026-09-25: common electronic
# equipment, so Darius Shirazi's "non-electronic" has something to exclude).
EXTRA_CARDS = {
    100287: "Camera Phone",
    100545: "Direct Intervention",
    101073: "Laptop Computer",
    102151: "Wash",
}


# Legacy cards the owner added to the pool (2026-09-28) so that tournament
# winning decks (TWDA) become playable. Unlike EXTRA_CARDS they are appended
# after the V5 tables, sorted by id among themselves, never sorted in: the
# existing CRYPT and LIBRARY indices stay put (golden replays, decklist
# tables, trained vocabularies).
ADDED_CARDS = {
    100106: "Ashur Tablets",
    100298: "Carlton Van Wyk",
    100576: "Donnybrook",
    100646: "Ennoia's Theater",
    100910: "Hell-for-Leather",
    101019: "Jake Washington",
    101103: "Life Boon",
    101256: "Murmur of the False Will",
    101442: "Powerbase: Savannah",
    101654: "Rötschreck",
    101665: "Saatet-ta",
    201617: "Gilbert Duane (G6)",
}


def in_pool(card):
    """A card of the engine pool: printed in a V5-era set, or an owner-added
    extra or addition."""
    return (
        bool(V5_SETS.keys() & set(card.get("ordered_sets", [])))
        or card["id"] in EXTRA_CARDS
        or card["id"] in ADDED_CARDS
    )


def pool_order(cards, key):
    """Table order: the V5 pool (and EXTRA_CARDS) sorted by `key`, then the
    ADDED_CARDS by id."""
    base = sorted((c for c in cards if c["id"] not in ADDED_CARDS), key=key)
    return base + sorted((c for c in cards if c["id"] in ADDED_CARDS), key=lambda c: c["id"])

# Phrases removed before window matching: riders and granted abilities that
# fire after the card's own play step (review convention: "riders firing after
# the card's own play step add no play window"; in-play granted abilities are
# the effects framework's concern).
RIDERS = [
    r"\d (optional )?(maneuver|press)s?\b[^.;]*",     # "with 1 optional press", "gets 1 maneuver…"
    r"with \d additional strikes?\b[^.]*",
    r"\d maneuver or press",
    r"(cannot|can't) [^.]*",                              # "cannot maneuver…, press…"
    r"can (also )?strike(, ranged)?:[^.]*",                          # granted strike ability
    r"(can|could) prevent \d[^.]*",                       # granted prevention
    r"to get \+\d intercept",                             # granted intercept ability
    r"skip the determine range step",
]

# Timing windows: an explicit printed play point. Generic windows: legality
# derived from the effect (stealth/intercept, bleed modifier, bleed reduction).
COMBAT_TIMING = [
    (r"before range is determined", "COMBAT_BEFORE_RANGE"),
    (r"\bmaneuver\b", "COMBAT_DETERMINE_RANGE"),
    (r"\bgrapple\b|before strikes are chosen", "COMBAT_BEFORE_STRIKES"),
    (r"additional strike", "COMBAT_ADDITIONAL_STRIKE"),
    (r"\bstrike(, ranged)?:", "COMBAT_STRIKE_DECLARATION"),
    (r"\bprevent\b", "COMBAT_DAMAGE_PREVENTION"),
    (r"\bpress\b", "COMBAT_PRESS"),
    (r"end of (the |a |this )?round", "COMBAT_END_OF_ROUND"),
    # "Only usable if … the opposing minion is not [ready]": after the
    # round's damage, in the end-of-round window (Hell-for-Leather [obt]).
    (r"the opposing minion is not\.", "COMBAT_END_OF_ROUND"),
    (r"combat would end", "COMBAT_WOULD_END"),
]
COMBAT_DEFAULT = "COMBAT_UNTIMED"   # rules-qa 2.2: no printed step

MODIFIER_TIMING = [
    (r"as (an?|the|this)\b[^.]{0,60}?\bis announced", "ACTION_AS_ANNOUNCED"),
    (r"if a minion attempts to block", "ACTION_BLOCK_ATTEMPT"),
    (r"if (this|the acting) minion is blocked", "ACTION_BLOCKED"),
    (r"would be successful", "ACTION_WOULD_BE_SUCCESSFUL"),
    (r"only usable after (the )?(action )?resol", "ACTION_AFTER_RESOLUTION"),
    (r"polling step|during (a|the|this) referendum", "REFERENDUM"),
]
MODIFIER_GENERIC = [
    (r"\+\d+ stealth", "ACTION_STEALTH_INTERCEPT"),
    (r"\+\d+ bleed|\+\d+ votes?", "ACTION_DURING"),
]
MODIFIER_DEFAULT = "ACTION_DURING"

REACTION_TIMING = [
    (r"\bwakes?\b", "WAKE"),                                   # rules-qa 2.4
    (r"unlocks?( and| to)? attempts? to block|unlock[^.]*; that \w+ attempts to block",
     "REACTION_UNLOCK_ATTEMPT"),
    (r"only usable[^.]*after block resolution", "ACTION_AFTER_COMBAT"),  # rules-qa 2.1
    (r"attempts? to block the current action", "ACTION_STEALTH_INTERCEPT"),  # block-loop play
    (r"unlock this (vampire|minion)|burns? \d blood to unlock\.", "REACTION_UNLOCK"),
    (r"after blocks are declined", "ACTION_BLOCKS_DECLINED"),
    (r"as (an?|the|this)\b[^.]{0,60}?\bis announced|cancel (an?|the|that) action",
     "ACTION_AS_ANNOUNCED"),
    (r"polling step|during (a|the|this) referendum", "REFERENDUM"),
    (r"reduce (a|the) bleed", "REACTION_DURING_ACTION"),   # legality: a bleed against you
    # An out-of-turn master triggered by the opposing strike (Rötschreck).
    (r"attempts to inflict aggravated damage", "COMBAT_BEFORE_RESOLUTION"),
    # An out-of-turn master saving a Methuselah from an oust (Life Boon).
    (r"methuselah with no pool to keep them in the game", "OUSTING"),
]
REACTION_GENERIC = [
    (r"\+\d+ intercept", "ACTION_STEALTH_INTERCEPT"),
]
REACTION_DEFAULT = "REACTION_DURING_ACTION"

# Cancel-as-played as the card's *own* play (not a rider letting the opponent
# cancel "this card", not an in-play lock ability): opens CARD_AS_PLAYED.
CANCEL_AS_PLAYED = r"cancel (?!this card)[^.]{0,80}?as it is (being )?played"

FLAGS = [
    (r"do not replace", "delayed_replacement"),
    (r"out-of-turn", "out_of_turn"),
    (r"\btrifle\b", "trifle"),
    (r"has cast (a )?(votes?|ballots?)", "refers_votes_cast"),
    (r"as if (the action were |it were )?unblocked|continue the action", "continues_action"),
    (r"at random", "randomness"),
    (r"becomes an? \d-capacity|becomes an? [^.]{0,30}?\bally\b|ally[^.]*to (your|the) ready region",
     "token_entity"),
    (r"cancel [^.]{0,80}?as it is (being )?played", "cancel_as_played"),
    (r"change the target of the bleed", "bounce"),
    (r"would (draw|replace) a card[^.]*instead", "draw_replacement"),
    (r"play cards from this (location|card) as if from (your )?hand", "alt_hand_source"),
]
# Card in play grants a minion action (Ⓓ actions, referendum-to-burn, granted
# hunt/equip variants); MPA/DPA-consuming abilities excluded; "as an action …
# is announced" timing clauses are not grants.
IN_PLAY_ACTION = r"\bas an? [^.;]{0,40}?\baction\b(?! to )(?![^.]* is announced)"

MODE_LINE = re.compile(r"^(\[[^\]]+\]\s*)+")
TYPE_TAGS = {"COMBAT": "Combat", "ACTION MODIFIER": "Action Modifier",
             "REACTION": "Reaction", "ACTION": "Action",
             "POLITICAL ACTION": "Political Action"}
MODE_TABLES = {
    "Combat": (COMBAT_TIMING, [], COMBAT_DEFAULT),
    "Action Modifier": (MODIFIER_TIMING, MODIFIER_GENERIC, MODIFIER_DEFAULT),
    "Reaction": (REACTION_TIMING, REACTION_GENERIC, REACTION_DEFAULT),
}
ACTION_MODE = re.compile(r"^(\+\d stealth )?(Ⓓ )?action\b|^Ⓓ", re.I)


def split_modes(text):
    """Return (preamble, [(type_tags, raw_tags, mode_text), …]). `raw_tags`
    are the bracket tokens as printed (`dom`, `DOM`, `COMBAT`), case kept."""
    preamble, modes = [], []
    for line in text.split("\n"):
        m = MODE_LINE.match(line)
        if not m:
            (modes[-1][2] if modes else preamble).append(line)
            continue
        raw = [t.strip("[]") for t in re.findall(r"\[[^\]]+\]", m.group(0))]
        types = {TYPE_TAGS[t.upper()] for t in raw if t.upper() in TYPE_TAGS}
        modes.append((types, raw, [line[m.end():]]))
    if not modes:                       # no mode lines: the whole text is one mode
        modes, preamble = [(set(), [], preamble)], []
    pre = " ".join(preamble)
    out = []
    for t, raw, ls in modes:
        for part in re.split(r"(?<=\.)\s+(?=Also usable )", " ".join(ls)):
            out.append((t, raw, part))
    return pre, out


def strip_riders(text):
    for pat in RIDERS:
        text = re.sub(pat, " ", text, flags=re.I)
    return text


def classify_mode(mtype, text, as_above=False):
    timing, generic, default = MODE_TABLES[mtype]
    hits = [w for pat, w in timing if re.search(pat, text, re.I)]
    if "ACTION_AFTER_COMBAT" in hits:
        hits = [w for w in hits if w != "REACTION_UNLOCK"]
    if hits:
        return hits
    hits = [w for pat, w in generic if re.search(pat, text, re.I)]
    if as_above:
        return hits              # inherits the previous mode; no default of its own
    return hits or [default]


def windows_for(card):
    """Per-card union of windows and flags (the matrix rows)."""
    wins, flags = [], []
    for _, mwins, mflags in modes_for(card):
        wins += mwins
        flags += mflags
    return list(dict.fromkeys(wins)), list(dict.fromkeys(flags))


def modes_for(card):
    """Per-mode classification: [(raw tags, windows, flags)]. Card-level
    (type) windows go on the modes of that type — the action mode of an
    Action/Combat card, every mode of a single-type card; card-text flags go
    on every mode."""
    types = card.get("types", [])
    text = card.get("card_text", "")
    wins = []
    flags = [f for pat, f in FLAGS if re.search(pat, text, re.I)]
    if re.search(IN_PLAY_ACTION, re.sub(r"[^.]*announces?\b[^.]*\.", "", text), re.I):
        flags.append("in_play_action")

    out_of_turn = "out_of_turn" in flags
    if "Master" in types and not out_of_turn:
        wins.append("MASTER_PHASE")
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

    mode_types = [t for t in types if t in MODE_TABLES]
    if out_of_turn:
        mode_types.append("Reaction")   # out-of-turn masters are reaction-timed plays
    preamble, modes = split_modes(text)
    if not mode_types:
        return [(raw, list(dict.fromkeys(wins)), flags) for _, raw, _ in modes]

    prev = []
    result = []
    for tags, raw, mtext in modes:
        raw_text = preamble + " " + mtext
        stripped = strip_riders(raw_text)
        as_above = bool(re.match(r"\s*as above", mtext, re.I))
        mwins = list(prev) if as_above else []
        action_mode = False
        if tags:
            mts = [t for t in tags if t in MODE_TABLES]
            action_mode = bool(tags & {"Action", "Political Action"})
        elif "Action" in types and ACTION_MODE.match(mtext.strip()):
            mts = []                     # the action mode of an Action/Combat card
            action_mode = True
        else:
            mts = mode_types
        if re.search(CANCEL_AS_PLAYED, raw_text, re.I) and not re.search(r"lock this", raw_text, re.I):
            mwins.append("CARD_AS_PLAYED")
        else:
            for mt in mts:
                mwins += classify_mode(mt, stripped, as_above)
        if "WAKE" in mwins:
            mwins.append("CARD_AS_PLAYED")          # rules-qa 2.4
        if "COMBAT_STRIKE_DECLARATION" in mwins and \
                re.search(r"with \d optional maneuver", raw_text, re.I):
            mwins.append("COMBAT_DETERMINE_RANGE")  # maneuver rider commits the strike
            flags.append("maneuver_rider")
        mflags = list(flags)
        if "Reaction" in mts and re.search(r"(?<!only )usable by a locked", raw_text, re.I) \
                and not ({"WAKE", "REACTION_UNLOCK", "REACTION_UNLOCK_ATTEMPT"} & set(mwins)):
            mflags.append("usable_locked")
        # Type-level windows: on the action mode of an Action/Combat card,
        # else on every mode (single-type cards; masters with modes).
        multi = len(mode_types) + int("Action" in types) > 1 or (
            "Action" in types and mode_types)
        type_wins = wins if (action_mode or not multi) else []
        result.append((raw, list(dict.fromkeys(type_wins + mwins)), mflags))
        prev = mwins
    return result


def rows_for(cards):
    codes = set(V5_SETS.values())
    for card in sorted(cards, key=lambda c: c.get("name", "")):
        if not in_pool(card):
            continue
        if "Vampire" in card.get("types", []):
            continue
        in_precon = any(
            p.split(":")[0] in codes and re.match(r"^P", p.split(":", 1)[1])
            for p in card.get("_set", "").split(", ") if ":" in p
        )
        wins, flags = windows_for(card)
        yield [str(card["id"]), card["name"], "/".join(card.get("types", [])),
               "/".join(card.get("disciplines", [])),
               "yes" if in_precon else "no",
               ";".join(wins), ";".join(flags), "yes" if len(wins) > 2 else ""]


def check(rows, reviewed_path):
    reviewed = {r["id"]: r for r in csv.DictReader(open(reviewed_path, newline=""))}
    gen = {r[0]: r for r in rows}
    bad = 0
    for cid in sorted(set(gen) | set(reviewed), key=lambda c: gen.get(c, reviewed.get(c))[1]
                      if c in gen else reviewed[c]["name"]):
        if cid not in gen or cid not in reviewed:
            print(f"only in {'generator' if cid in gen else 'reviewed'}: {cid}")
            bad += 1
            continue
        g, r = gen[cid], reviewed[cid]
        for col, idx in (("windows", 5), ("flags", 6)):
            if set(filter(None, g[idx].split(";"))) != set(filter(None, r[col].split(";"))):
                print(f"{r['name']!r:40} {col}: gen={g[idx]}  reviewed={r[col]}")
                bad += 1
    print(f"# {len(gen)} cards, {bad} divergences from {reviewed_path}", file=sys.stderr)
    return bad


def main():
    args = sys.argv[1:]
    reviewed = None
    if "--check" in args:
        i = args.index("--check")
        reviewed = args[i + 1]
        del args[i:i + 2]
    if args:
        cards = json.load(open(args[0]))
    else:
        with urllib.request.urlopen("https://static.krcg.org/data/vtes.json") as f:
            cards = json.load(f)

    rows = list(rows_for(cards))
    if reviewed:
        sys.exit(1 if check(rows, reviewed) else 0)
    out = csv.writer(sys.stdout)
    out.writerow(["id", "name", "types", "disciplines", "in_precon",
                  "windows", "flags", "needs_review"])
    out.writerows(rows)
    print(f"# {len(rows)} library cards, {sum(r[7] == 'yes' for r in rows)} flagged for review",
          file=sys.stderr)


if __name__ == "__main__":
    main()
