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


# Legacy cards the owner added to the pool so that tournament winning decks
# (TWDA) and the anniversary decks become playable. Unlike EXTRA_CARDS they
# are appended after the V5 tables, batch after batch in this order, sorted
# by id within a batch, never sorted in: the existing CRYPT and LIBRARY
# indices stay put (golden replays, decklist tables, trained vocabularies).
ADDED_BATCHES = [
    # 2026-09-28: TWDA decks.
    {
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
    },
    # 2026-10-06: Reign of Stanislava (25th Anniversary), its new cards.
    {
        100064: "Ancient Influence",
        100645: "Enkil Cog",
        100710: "Fear of Mekhet",
        100761: "Forced March",
        100994: "Instantaneous Transformation",
        101238: "Monastery of Shadows",
        101390: "Permanent Vacation",
        101417: "Political Stranglehold",
        101591: "Reins of Power",
        102207: "Zillah's Valley",
        200575: "Hartmut Stover (G3)",
        200634: "Ingrid Rossler (G2)",
        200937: "Mark Decker (G3)",
        201312: "Stanislava (G2)",
        201478: "Xaviar (G3 ADV)",
    },
    # 2026-10-06: The Endless Dance (30th Anniversary), its new cards (the
    # deck's Ancient Influence, Political Stranglehold, Reins of Power and
    # Zillah's Valley came with Reign of Stanislava).
    {
        100119: "Awe",
        100274: "Business Pressure",
        100333: "Charismatic Aura",
        101127: "The Louvre, Paris",
        101590: "Reinforcements",
        102269: "Camarilla Conclave",
        102271: "Confiscation",
        102273: "Loup",
        201694: "Diana Iadanza (G7)",
        201696: "François Villon (G6)",
        201699: "Kazuki Hanabira (G7)",
        201702: "Marcos Belegrad (G7)",
        201703: "Modius (G7)",
        201706: "Silvian Popescu (G7)",
        201707: "Valentina Osako (G7)",
    },
    # 2026-10-07: Sabbat V5, the Path of Power and the Inner Voice deck's
    # new cards (with Under Siege and Unholy Sacrament, shared with later
    # decks).
    {
        100165: "Black Forest Base",
        100294: "Cardinal Benediction",
        100307: "Catatonic Fear",
        100541: "Día de los Muertos",
        101008: "Iron Glare",
        101437: "Powerbase: Madrid",
        101490: "Private Audience",
        101587: "Regent",
        102063: "Under Siege",
        102308: "Absolute Tyranny",
        102312: "Cold War",
        102316: "Dominant Personality",
        102318: "Empires Fall",
        102331: "Magnetic Authority",
        102334: "Privileged Position",
        102337: "Relentlessness",
        102345: "Unholy Sacrament",
        102346: "Unthinkable Humiliation",
        102348: "War of Ages",
        201733: "Aaradhya, The Callous Tyrant (G6)",
        201744: "Concordia (G6)",
        201747: "Damian (G6)",
        201760: "Khin Aye (G6)",
        201762: "Lika, The Purger (G6)",
        201764: "Lyyli Yösydän (G6)",
        201766: "Onkabetse, Arriviste (G6)",
        201769: "Rasha Tarabzouni (G6)",
        201770: "Rexton \"Savage\" Abernathy (G6)",
        201774: "Suwira (G6)",
        201775: "Üresség (G6)",
        201776: "Věnceslava, The Implacable (G6)",
    },
    # 2026-10-07: Sabbat V5, the Path of Cathari deck's new cards (The
    # Hungry Coyote a legacy reprint).
    {
        100945: "Hungry Coyote, The",
        102320: "Enthrall",
        102321: "Fever Pitch",
        102323: "Forward Momentum",
        102328: "Hedonism",
        102330: "Kiss of Cathari",
        102338: "Seeds of Terror",
        102341: "Subversion",
        102342: "Swift Cover",
        102343: "Terrifying Visage",
        201735: "Aelswith, The Irresistible (G6)",
        201736: "Agnieszka, Tempter of Legions (G6)",
        201743: "Coco Thorn (G6)",
        201746: "Cyan Sin (G6)",
        201748: "Devorah (G6)",
        201751: "Fotini Katsikaris (G6)",
        201753: "Glissando (G6)",
        201754: "Gostoso (G6)",
        201758: "Jürgen, The Libertine (G6)",
        201767: "Osvaldo Kühnemann (G6)",
        201777: "Verrix, Naughty Boy (G6)",
        201778: "Ximena (G6)",
    },
    # 2026-10-07: Sabbat V5, the Path of Caine deck's new cards (Abbot,
    # Depravity, Fiendish Tongue and Yawp Court legacy reprints).
    {
        100006: "Abbot",
        100526: "Depravity",
        100726: "Fiendish Tongue",
        102199: "Yawp Court",
        102309: "Abuse of Power",
        102310: "Augury of Doom",
        102315: "Dance with the Devil",
        102317: "Dread Mastiff",
        102319: "Ensnare a Beast",
        102329: "Hunting the Quarry",
        102333: "Pillars Fall",
        102336: "Rage of Apedemak",
        102339: "Sentry Signal",
        102340: "Soulgrinder",
        102344: "Twisted Bloodhound",
        102347: "Wall of Filth",
        201738: "Algirdas, The Solar Prophet (G6)",
        201739: "Anousha, The Maniac (G6)",
        201742: "Berenguela, The Vicious (G6)",
        201745: "Crossbreaker (G6)",
        201755: "Grażyna \"Butcher\" Olesińska (G6)",
        201756: "Hekla \"Bonecrusher\" Jónasdóttir (G6)",
        201759: "Kalyani, Agent of Doom (G6)",
        201761: "Lawbringer (G6)",
        201763: "Loyiso, The Ruthless (G6)",
        201768: "Przemysław, War Beast (G6)",
        201772: "Serhat Gunde, Combat Junkie (G6)",
        201773: "Surapong (G6)",
    },
]
ADDED_CARDS = {k: v for batch in ADDED_BATCHES for k, v in batch.items()}


def in_pool(card):
    """A card of the engine pool: printed in a V5-era set, or an owner-added
    extra or addition."""
    return (
        bool(V5_SETS.keys() & set(card.get("ordered_sets", [])))
        or card["id"] in EXTRA_CARDS
        or card["id"] in ADDED_CARDS
    )


# Anniversary decks (owner, 2026-10-05: each decklist reviewed before it
# trains; not every card of the bundle belongs to the deck). A deck is its
# set's cards minus the extras Black Chantry lists apart (25th: the
# 2019-06-04 announcement; 30th: the product page, as the VTES ONE blog), by
# id with the copies left out. Emitted once every card of the deck is in the
# pool. No Fall of London / Shadows of Berlin deck ever trains (owner): they
# are not real decks.
ANNIVERSARY_DECKS = {
    "Twenty-Fifth Anniversary": ("Reign of Stanislava", {
        100037: 1,  # Alamut
        100071: 1,  # The Ankara Citadel, Turkey
        100169: 1,  # Black Metamorphosis
        100286: 1,  # Camarilla Vitae Slave
        100651: 1,  # Entombment
        100720: 1,  # Femur of Toomler
        100770: 1,  # Form of Corruption
        100882: 1,  # Hand of Conrad
        100908: 1,  # Heidelberg Castle, Germany
        100932: 1,  # Homunculus
        101048: 1,  # Khobar Towers, Al-Khubar
        101091: 1,  # Legendary Vampire
        101103: 1,  # Life Boon
        101664: 1,  # Rutor's Hand
        101681: 1,  # The Sargon Fragment
        101781: 1,  # The Signet of King Saul
        101931: 1,  # Talbot's Chainsaw
        201407: 1,  # Una (G3)
        201522: 2,  # Grimgroth (G6)
    }),
    "Thirtieth Anniversary": ("The Endless Dance", {
        100038: 1,  # Alastor
        100060: 1,  # Anathema
        101500: 1,  # Protected Resources
        101515: 1,  # Pulled Fangs
        101973: 1,  # Third Tradition: Progeny
        102268: 1,  # Banu Haqim Justicar
        102272: 1,  # Lasombra Justicar
        201693: 2,  # Annabelle Triabell (G6)
        201695: 1,  # Dónal O'Connor (G6)
        201697: 2,  # Ian Carfax (G7)
        201698: 2,  # Juliet Parr (G7)
        201700: 2,  # Lucinde, Alastor (G7)
        201704: 2,  # Molly MacDonald (G7)
        201705: 2,  # Nikolaus Vermeulen (G6)
    }),
}


def anniversary_decks(cards):
    """The anniversary decks fully in the pool, in ANNIVERSARY_DECKS order,
    as (deck name, set, {card id: copies}): the set's copies minus its
    extras. Both lists are 20 cards and the decks 12 crypt / 88 library."""
    out = []
    for s, (name, extras) in ANNIVERSARY_DECKS.items():
        assert sum(extras.values()) == 20, s
        deck = {}
        for c in cards:
            n = sum(e.get("copies", 0) for e in c.get("sets", {}).get(s, []))
            n -= extras.get(c["id"], 0)
            assert n >= 0, (s, c["name"])
            if n:
                deck[c["id"]] = (c, n)
        if not all(in_pool(c) for c, _ in deck.values()):
            continue
        crypt = sum(n for c, n in deck.values() if "Vampire" in c["types"])
        assert (crypt, sum(n for _, n in deck.values()) - crypt) == (12, 88), (s, crypt)
        out.append((name, s, {i: n for i, (_, n) in deck.items()}))
    return out


# The Sabbat V5 precons (KRCG `precon` tags), each emitted once every card
# of it is in the pool; drawn by path for training (owner, 2026-10-05).
SABBAT_SET = "Sabbat V5"


def sabbat_decks(cards):
    """The Sabbat V5 precons fully in the pool, by deck name, as (deck name,
    set, {card id: copies}); 12 crypt / 77 library each."""
    decks = {}
    for c in cards:
        for e in c.get("sets", {}).get(SABBAT_SET, []):
            if "precon" in e:
                decks.setdefault(e["precon"], {})[c["id"]] = (c, e["copies"])
    # In the order they were completed (the batch of their last card), so
    # a deck added never moves one already emitted.
    batch = {k: i for i, b in enumerate(ADDED_BATCHES) for k in b}
    order = sorted(decks, key=lambda n: (max(batch.get(i, -1) for i in decks[n]), n))
    out = []
    for name in order:
        deck = decks[name]
        if not all(in_pool(c) for c, _ in deck.values()):
            continue
        crypt = sum(n for c, n in deck.values() if "Vampire" in c["types"])
        assert (crypt, sum(n for _, n in deck.values()) - crypt) == (12, 77), (name, crypt)
        out.append((name, SABBAT_SET, {i: n for i, (_, n) in deck.items()}))
    return out


def pool_order(cards, key):
    """Table order: the V5 pool (and EXTRA_CARDS) sorted by `key`, then the
    ADDED_CARDS batch by batch, by id within a batch."""
    batch = {k: i for i, b in enumerate(ADDED_BATCHES) for k in b}
    base = sorted((c for c in cards if c["id"] not in ADDED_CARDS), key=key)
    return base + sorted((c for c in cards if c["id"] in ADDED_CARDS),
                         key=lambda c: (batch[c["id"]], c["id"]))

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
    r"can burn this card to prevent[^.]*",                # granted prevention (Wall of Filth)
    r"but to prevent \d[^.]*",                            # its [PRO] "as above, but to prevent 2"
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
    # "Only usable if combat is about to end" (Hunting the Quarry [cel]):
    # the same moment, before it ends.
    (r"combat is about to end", "COMBAT_WOULD_END"),
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

# "[cel] or [for]" (the Sabbat V5 layout): either Discipline, one mode
# (rules-qa 52.1); side by side, a combo (rules-qa 25.2).
MODE_LINE = re.compile(r"^(\[[^\]]+\]\s*(or\s+(?=\[))?)+")
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
        elif {"Combat", "Action Modifier"} <= set(mode_types):
            # An untagged mode of an action modifier and combat card (Swift
            # Cover): a strike or combat effect is the combat card's.
            combat = re.search(r"\b(strike|combat|range|maneuver|press|damage)\b", mtext, re.I)
            mts = ["Combat" if combat else "Action Modifier"]
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
            # "As above, with 1 optional maneuver" after a mode without one
            # (Slam [POT]): only at the maneuver — as the strike it is the
            # mode above for more (rules-qa 48.4).
            if as_above and "COMBAT_DETERMINE_RANGE" not in prev and \
                    re.match(r"\s*as above, with \d optional maneuver", mtext, re.I):
                mwins = [w for w in mwins if w != "COMBAT_STRIKE_DECLARATION"]
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
