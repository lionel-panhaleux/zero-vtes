#!/usr/bin/env python3
"""Generate the V5 library static-data module for the engine from KRCG data.

Static card data is generated, never hand-typed (DESIGN §4.5). Output:
`crates/engine/src/library.rs` — the 421 V5 library cards (id, printed name,
types, discipline requirement per mode, clan requirement, costs, labels,
per-mode play windows and flags from `window_matrix.modes_for`), the 14
full-size precon libraries and the 14 New Blood mini-starter libraries.
Behavior (effects) is *not* here: this module is the mask-side static half of
the representation.

Usage: python3 scripts/gen_library.py [path/to/vtes.json] > crates/engine/src/library.rs
       python3 scripts/gen_library.py [path/to/vtes.json] --check crates/engine/src/library.rs
"""
import json
import os
import re
import sys
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from window_matrix import anniversary_decks, in_pool, modes_for, pool_order, sabbat_decks  # noqa: E402

FULL_PRECON_SETS = ["Fifth Edition", "Fifth Edition (Anarch)", "Fifth Edition (Companion)"]
MINI_PRECON_SETS = ["New Blood", "New Blood II", "New Blood III"]
TYPES = ["Master", "Action", "Action Modifier", "Reaction", "Combat", "Political Action",
         "Ally", "Equipment", "Retainer", "Event"]
LABELS = [
    ("Unique", r"^unique\b|\bunique (location|\w+ with)|\. unique\."),
    # The type line only ("Burn a location." is an action's text).
    # A locquipment too ("This equipment card represents a location",
    # Ruins of Ceoris).
    ("Location", r"(^|\. )(unique )?location\.|represents a location"),
    ("HuntingGround", r"(^|\. )hunting ground\."),
    ("Weapon", r"^(unique )?(melee )?weapon\b"),
    ("Gun", r"^weapon: gun"),
    ("Vehicle", r"^vehicle\."),
    ("Electronic", r"^electronic\b"),
    ("Archetype", r"\barchetype\."),
    ("Trifle", r"\btrifle\."),
    ("Gehenna", r"^gehenna\."),
    ("Title", r"(^|\. )title\."),
    ("OutOfTurn", r"^out-of-turn\."),
    ("Frenzy", r"^(out-of-turn\. )?frenzy\."),
    # Combat card kinds a card may name ("cancel a grapple or aim card",
    # Sword of the Archangel).
    ("Grapple", r"^grapple\."),
    ("Aim", r"^aim\."),
    # A Discipline master card ("Discipline. Put this card on a vampire.").
    ("Discipline", r"^discipline\."),
    # Ally and retainer subtypes: the type line only (advanced rules 5.7.1).
    ("Mortal", r"^(unique )?mortal with"),
    ("Animal", r"^(unique )?animal with"),
    ("Ghoul", r"^(unique )?ghoul with"),
    ("Zombie", r"^(unique )?zombie with"),
    ("Wraith", r"^(unique )?wraith with"),
    ("Werewolf", r"^(unique )?werewolf with"),
    ("Mage", r"^(unique )?mage with"),
    ("Demon", r"^(unique )?demon with"),
    # An infernal ally (Herald of Topheth): "… flight [FLIGHT]. Infernal."
    # on its type line.
    ("Infernal", r"^(unique )?\w+ with [^\n]*\. infernal\."),
    # "More than one Discipline can be used to play this card": one mode per
    # non-empty set of its Disciplines, each set required in full (LSJ
    # 20011204-3: played, it requires the Disciplines used).
    ("MultiDiscipline", r"more than one discipline can be used"),
    # A card "requiring an Anarch": its requirements name Anarch or baron
    # (a baron card requires an Anarch, LSJ 20080603); a negative one such
    # as "non-Camarilla" does not (ANK 20250320).
    ("RequiresAnarch", r"\brequires? (?:an?|two) [^.]*(?<!non-)\b(?:anarch|baron)\b"),
    # A card "making a vampire Anarch" (Powerbase: Los Angeles).
    ("MakesAnarch", r"\bmake (?:that|a|this) [^.]*\bvampire (?:you control )?anarch\b"),
    # The Operation Antigen cards (Mobile HQ keys on them): the sentence on
    # the type line, not "An Operation Antigen ally" in the text.
    ("OperationAntigen", r"\boperation antigen\."),
    # "Condemnation." on the type line (Condemnation: Mute burns the
    # other condemnation cards on its minion).
    ("Condemnation", r"\bcondemnation\."),
    # "Hunting ground. Derivative." (Poacher's Hunting Ground counts the
    # non-derivative ones).
    ("Derivative", r"(^|\. )derivative\."),
]
# An ally's or retainer's type line: "Unique mortal with 2 life. 1 strength,
# 0 bleed." (a retainer prints its life only).
# "… 3 strength, 1 bleed, flight [FLIGHT]." (Herald of Topheth): flight is
# a bare trait (rules-qa 16.6).
BODY = re.compile(
    r"^(?:unique )?[\w ]+? with (\d+) life\.(?: (\d+) strength, (\d+) bleed(?:, flight \[FLIGHT\])?\.)?",
    re.I,
)


def rust_str(s):
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def cost(v):
    if v is None:
        return "Cost::None"
    if str(v).upper() == "X":
        return "Cost::X"
    return f"Cost::Fixed({int(v)})"


def labels_of(text):
    # A legacy master prints its type first ("Master: unique location.").
    text = re.sub(r"^Master: ", "", text)
    head = " ".join(line for line in text.split("\n")[:2])
    return [name for name, pat in LABELS if re.search(pat, head, re.I)]


def body_of(c):
    if not {"Ally", "Retainer"} & set(c["types"]):
        return "None"
    m = BODY.match(c["card_text"])
    if not m:
        raise SystemExit(f"no life on the type line of {c['_name']}")
    life, strength, bleed = m.groups()
    if "Ally" in c["types"] and strength is None:
        raise SystemExit(f"no strength and bleed on ally {c['_name']}")
    return f"Some(Body {{ life: {life}, strength: {strength or 0}, bleed: {bleed or 0} }})"


def multi_modes(modes):
    """The modes of a multi-Discipline card: its single-Discipline modes
    first (their indices kept), then every larger set of them, windows and
    flags merged."""
    from itertools import combinations
    out = list(modes)
    for r in range(2, len(modes) + 1):
        for combo in combinations(modes, r):
            tags = [t for m in combo for t in m[0]]
            wins = list(dict.fromkeys(w for m in combo for w in m[1]))
            flags = list(dict.fromkeys(f for m in combo for f in m[2]))
            out.append((tags, wins, flags))
    return out


def generate(cards):
    lib = pool_order([c for c in cards if "Vampire" not in c.get("types", []) and in_pool(c)],
                     key=lambda c: (c["_name"], c["id"]))
    index = {c["id"]: i for i, c in enumerate(lib)}
    out = []
    out.append("//! V5 library static data — GENERATED by scripts/gen_library.py from KRCG")
    out.append("//! vtes.json (windows from scripts/window_matrix.py). Do not edit by hand;")
    out.append("//! regenerate and check with `--check`.")
    out.append("")
    out.append("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]")
    out.append("pub enum CardType {")
    for t in TYPES:
        out.append(f"    {t.replace(' ', '')},")
    out.append("}")
    out.append("")
    out.append("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]")
    out.append("pub enum Cost {")
    out.append("    None,")
    out.append("    Fixed(u8),")
    out.append("    /// Printed \"X\": chosen when the card is played.")
    out.append("    X,")
    out.append("}")
    out.append("")
    out.append("/// Printed labels the rules key on (uniqueness, one hunting ground per")
    out.append("/// turn, one vehicle, one archetype, trifle accounting, …).")
    out.append("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]")
    out.append("pub enum Label {")
    for name, _ in LABELS:
        out.append(f"    {name},")
    out.append("}")
    out.append("")
    out.append("/// One playable mode of a card: its bracket tags as printed (`DOM`,")
    out.append("/// `ani`, `pro`, `COMBAT`; lower case = basic, upper = superior; empty for an")
    out.append("/// untagged card), the windows it is playable in, and per-mode flags.")
    out.append("/// Windows and flags are names from data/window-matrix-reviewed.csv. Two")
    out.append("/// discipline tags on one mode mean *either* unless the card is a `combo`")
    out.append("/// or `Label::MultiDiscipline` (one mode per set of Disciplines used).")
    out.append("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]")
    out.append("pub struct Mode {")
    out.append("    pub tags: &'static [&'static str],")
    out.append("    pub windows: &'static [&'static str],")
    out.append("    pub flags: &'static [&'static str],")
    out.append("}")
    out.append("")
    out.append("/// An ally's or retainer's printed type line: its starting life, and an")
    out.append("/// ally's strength and bleed (0 for a retainer, which has neither).")
    out.append("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]")
    out.append("pub struct Body {")
    out.append("    pub life: u8,")
    out.append("    pub strength: u8,")
    out.append("    pub bleed: u8,")
    out.append("}")
    out.append("")
    out.append("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]")
    out.append("pub struct LibraryCard {")
    out.append("    pub id: u32,")
    out.append("    pub name: &'static str,")
    out.append("    pub types: &'static [CardType],")
    out.append("    /// Discipline requirement(s) at basic level, as printed (`dom`, `aus`, …).")
    out.append("    pub disciplines: &'static [&'static str],")
    out.append("    /// Clan requirement(s), if any.")
    out.append("    pub clans: &'static [&'static str],")
    out.append("    pub blood_cost: Cost,")
    out.append("    pub pool_cost: Cost,")
    out.append("    pub burn_option: bool,")
    out.append("    /// Combo card: every listed discipline is required (KRCG `combo`).")
    out.append("    pub combo: bool,")
    out.append("    pub labels: &'static [Label],")
    out.append("    /// Allies and retainers: the printed type line.")
    out.append("    pub body: Option<Body>,")
    out.append("    pub modes: &'static [Mode],")
    out.append("    /// The Sabbat path icon (KRCG `path`): a vampire following it plays")
    out.append("    /// the card (rules-qa 49.7).")
    out.append("    pub path: Option<&'static str>,")
    out.append("}")
    out.append("")
    out.append("#[rustfmt::skip]")
    out.append(f"pub static LIBRARY: [LibraryCard; {len(lib)}] = [")
    for c in lib:
        out.append("    LibraryCard {")
        out.append(f"        id: {c['id']},")
        out.append(f"        name: {rust_str(c['_name'])},")
        types = ", ".join("CardType::" + t.replace(" ", "") for t in c["types"])
        out.append(f"        types: &[{types}],")
        disc = ", ".join(rust_str(d) for d in c.get("disciplines", []))
        out.append(f"        disciplines: &[{disc}],")
        clans = ", ".join(rust_str(d) for d in c.get("clans", []))
        out.append(f"        clans: &[{clans}],")
        out.append(f"        blood_cost: {cost(c.get('blood_cost'))},")
        out.append(f"        pool_cost: {cost(c.get('pool_cost'))},")
        out.append(f"        burn_option: {'true' if c.get('burn_option') else 'false'},")
        out.append(f"        combo: {'true' if c.get('combo') else 'false'},")
        labels = ", ".join("Label::" + l for l in labels_of(c["card_text"]))
        out.append(f"        labels: &[{labels}],")
        out.append(f"        body: {body_of(c)},")
        out.append("        modes: &[")
        modes = modes_for(c)
        if "MultiDiscipline" in labels_of(c["card_text"]):
            modes = multi_modes(modes)
        for tags, wins, flags in modes:
            t = ", ".join(rust_str(x) for x in tags)
            w = ", ".join(rust_str(x) for x in wins)
            f = ", ".join(rust_str(x) for x in flags)
            out.append("            Mode {")
            out.append(f"                tags: &[{t}],")
            out.append(f"                windows: &[{w}],")
            out.append(f"                flags: &[{f}],")
            out.append("            },")
        out.append("        ],")
        path = f"Some({rust_str(c['path'])})" if c.get("path") else "None"
        out.append(f"        path: {path},")
        out.append("    },")
    out.append("];")
    out.append("")
    precon_table(out, lib, lambda c: index[c["id"]], FULL_PRECON_SETS, "PRECON_LIBRARIES", [
        "/// The 14 full-size preconstructed libraries as (name, [(LIBRARY index, copies)]).",
        "/// Names match crypt::PRECONS.",
    ])
    out.append("")
    precon_table(out, lib, lambda c: index[c["id"]], MINI_PRECON_SETS, "MINI_LIBRARIES", [
        "/// The 14 New Blood mini-starter libraries as (name, [(LIBRARY index, copies)]).",
        "/// Names match crypt::MINI_PRECONS.",
    ])
    out.append("")
    for source, table, size, other in [
        (anniversary_decks, "ANNIVERSARY_LIBRARIES", 88, "ANNIVERSARY_PRECONS"),
        (sabbat_decks, "SABBAT_LIBRARIES", 77, "SABBAT_PRECONS"),
    ]:
        out.append(f"/// The {'anniversary' if size == 88 else 'Sabbat V5'} decks' libraries ({size} cards each) as (name,")
        out.append(f"/// [(LIBRARY index, copies)]). Names match crypt::{other}.")
        out.append("#[rustfmt::skip]")
        decks = source(cards)
        out.append(f"pub static {table}: [(&str, &[(u16, u8)]); {len(decks)}] = [")
        for deck, s, entries in decks:
            body = ", ".join(f"({i}, {k})" for i, k in sorted((index[c], k) for c, k in entries.items() if c in index))
            out.append(f"    ({rust_str(deck + ' (' + s + ')')}, &[{body}]),")
        out.append("];")
        out.append("")
    out.pop()
    return "\n".join(out) + "\n"


def precon_table(out, cards, index_of, sets, name, doc):
    """One table of preconstructed decks from `sets`, sorted by (set, deck)."""
    precons = {}
    for c in cards:
        for s, entries in c["sets"].items():
            if s not in sets:
                continue
            for e in entries:
                if "precon" in e:
                    precons.setdefault((s, e["precon"]), []).append((index_of(c), e["copies"]))
    precons = sorted(precons.items())
    out.extend(doc)
    out.append("#[rustfmt::skip]")
    out.append(f"pub static {name}: [(&str, &[(u16, u8)]); {len(precons)}] = [")
    for (s, p), entries in precons:
        body = ", ".join(f"({i}, {k})" for i, k in sorted(entries))
        out.append(f"    ({rust_str(p + ' (' + s + ')')}, &[{body}]),")
    out.append("];")


def main():
    args = sys.argv[1:]
    check = None
    if "--check" in args:
        i = args.index("--check")
        check = args[i + 1]
        del args[i:i + 2]
    if args:
        cards = json.load(open(args[0]))
    else:
        with urllib.request.urlopen("https://static.krcg.org/data/vtes.json") as f:
            cards = json.load(f)
    text = generate(cards)
    if check:
        if open(check).read() != text:
            print(f"{check} differs from generated output", file=sys.stderr)
            sys.exit(1)
        print(f"# {check} is up to date", file=sys.stderr)
        return
    sys.stdout.write(text)


if __name__ == "__main__":
    main()
