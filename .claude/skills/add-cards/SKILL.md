---
name: add-cards
description: >
  Add VTES cards to the zero-vtes engine: choose a round of cards, research
  each card's text and rulings, design it on the engine's existing
  mechanisms, test it, grade every reading the rules leave open and ask the
  owner only about the risky ones. Use for any round of new library cards or
  vampires, legacy cards for TWDA decks, or a newly released set.
---

# Adding cards

Goal: every card in the pool, added in a loop where the owner intervenes
only on readings likely to be wrong. Read first: CLAUDE.md (owner rules),
`docs/engine.md` §2–5 (dispatch, what is enforced, adding a card, comment
policy), `docs/rules-digest.md` (settled readings by mechanism; its
"Default readings" and "What counts as X" sections always). Work in a
worktree on a round branch (`../zero-vtes-rN`, branch `roundN`), never in
the shared checkout (regenerated tables there get built into the training
session's extension at its next `uv sync`); keep a progress file in the job's tmp (done, todo,
readings, commit checklist): a round outlives one context window.

## 1. Choose the round

The owner's metric (2026-10-14): recent (2025+) TWDA decks entering the
*kept* corpus. Simulate `scripts/gen_twda.py` with candidate cards appended
(monkeypatch `g.table_ids`); greedy over each recent deck's missing set,
maximizing net recent-kept gain per new card; check the ceiling first (more
cards can displace kept decks: `VERSIONS`, `SAME_CRYPT`, `SAME_LIBRARY`).
Present steps (bundle → kept, recent kept) with the cost per recent deck at
each step (round 11 ~2 cards a deck, round 12 ~5–6: diminishing returns are
the owner's cut signal), and let the owner pick the cut.
Count cards by id, never by splitting names on commas. Then add the round
to `ADDED_BATCHES` in `scripts/window_matrix.py` (the owner's words in a
comment) and regenerate `crypt.rs`, `library.rs`, `twda.rs`; check the
kept count equals the simulation's. Append the new library cards' draft
rows to `data/window-matrix-reviewed.csv` and review them against analog
rows: when a draft is wrong, fix the generator's pattern, not the row
(`--check` at 0 divergences). Regenerate the ledger
(`scripts/rulings_v5.py vtes.json > data/rulings-v5.csv`). Both CSVs are
CRLF: the csv module with `lineterminator="\r\n"`, or byte-wise.

## 2. Research each card, in this order

1. Its text and every KRCG ruling (`vtes.json`), and the
   `../vtes-advanced-rules` rulings YAML for its id (group rulings too).
2. Its ledger rows (`data/rulings-v5.csv`): the rulings are the test plan.
3. `docs/rules-digest.md` sections for the card's mechanisms, then the
   rules-qa items they cite.
4. The closest existing definitions (an Explore subagent maps the round's
   cards to analog consts and missing variants; trust const names, not line
   numbers).
5. The rulebook (`vtes` skill) for the mechanism's general rule.

## 3. Design

- Reuse first: an existing `Effect`/`Requirement`/`Modifier`, with a field
  or filter added if needed, over a card-named variant. `Effect::family()`
  and `Requirement::slots()` are exhaustive: classify a new variant there;
  the definition-walking tests say where it may sit.
- A new variant only for a new mechanism, named for the mechanism. If the
  card's shape has no primitive, build the primitive; never bend the
  reading to the nearest existing one (that is how 71.21 and 71.25 went
  wrong).
- New interrupts, purposes, choice kinds or window labels: api ids, host
  ids/describe, text prompt/option, regenerate schema/openapi/protocol.ts,
  and tell the training session before merging (the observation encoder's
  fixed id tables).
- Vampires end with ALL their text; text-only traits (blood cursed, Black
  Hand, scarce, titles, votes) are generated crypt data and need only a
  comment. A `VampireDef` with `group: None` binds every group of the name:
  check the crypt for other groups first (Lucinde G3 vs G7).
- "Put this card on …" enters the card itself (`place_now`); its test
  asserts the card in play is not `effect_only` and the ash heap lacks it
  (the recurring miss). Card data in definitions; conditions read live.

## 4. Grade every reading (the owner reviews only red)

For each card, two independent reads:

- **Yours**, as designed and tested.
- **A second reader**: a fresh subagent (one per ~6–10 cards) that never
  sees the code, your design or your readings. Give it the card text, its KRCG rulings, its advanced-rules
  entry, the digest's "Default readings" and "What counts as X" sections,
  and these rules stated outright: card text addresses its controller
  (32.7); Ⓓ is only indicative (20.1): an action that picks another
  Methuselah's minions targets them; a card played by another card is
  played. It answers, per card:
  1. Who chooses what, and when (declaration vs resolution)?
  2. What is targeted; is the action directed, and at whom?
  3. What does the card or its subject count as (hunt, bleed, directed,
     modifier vs reaction, vampire, played vs put in play)? Which other
     cards read that status?
  4. Whose cards, region or pool do "you"/"your"/"its" mean?
  5. Limits: once per what ("during X" = once per X); caps; what the words
     qualify.
  6. Timing: which window or moment; "as", "after", "each"; replacement or
     after the fact.
  7. Costs vs effects: what is paid, what happens on a fizzle or a burn.
  8. Combinations: two copies, a dual-type card, the effect played by
     another card, the card on an ally or a non-vampire.

Diff the two reads yourself, item by item, then grade each reading:

- **Green**: settled by a KRCG ruling, a rules-qa item, or a default
  reading alone. Recorded, not asked.
- **Amber**: follows a confirmed precedent (cite it) and both reads agree.
  Recorded in rules-qa as a statement with its tests; listed to the owner
  in one line each, no answer needed.
- **Red**, asked of the owner, whatever the reads agree on:
  - the two reads disagree;
  - no precedent at all;
  - a "counts as"/status question unless a ruling scopes which cards read
    it (73.6, 70.17);
  - the design bends the card to an existing primitive (71.21, 71.25);
  - a combination case (two copies, dual type, played by another card)
    with no ruling (69.12, 70.17).

Red items go to the owner as one concrete scenario each, the engine's
outcome, a yes/no, and the test name a "no" would flip. Aim for a handful
per round. Every reading, green to red, still goes in the round's rules-qa
batch (stated, test-cited) so code citations work.

## 5. Tests, ledger, fuzz

One test file per round (`tests/twda_roundN.rs`), each test citing the
ledger refs it backs; at least one test per card and per rules-qa reading;
write them in chunks of ~10 and run as you go (they find engine bugs).
Elision breaks naive tests (add a second minion; `never_offered_until`).
Every new ledger row gets a status (`--check` at 0). Two fuzz fixtures per
round from the new cards' decks (seeds ≈ 2× the last coverage key's first
seed, ≤ 300); re-bless goldens and confirm unrelated fixtures moved only in
hashes (a trajectory change there is a behaviour change to explain).

## 6. Finish

All CLAUDE.md checks green (including `cargo test -p zvtes-policy`, the
py side, generators, window matrix, ledger, DESIGN count line); the
`engine-review` skill on the round's diff; findings logged
(`docs/findings.md`); the cards board updated (`docs/boards/cards.md`).
Ask the red items with the commit request, before committing: a "no"
lands in the same commit. Then rebase the round branch on `main`
(regenerate `crypt.rs`/`library.rs`/`twda.rs` on conflict, never
hand-merge them), merge on the owner's word, and tell the training session
the vocabulary deltas (library n→m, vampires, kept decks) and any new
encoder id. The round's rules-qa batch takes the next number on `main` when
written; tell the other sessions.

## 7. Learn from every correction

When the owner corrects a reading (or a game shows one wrong), find the
root cause and make it impossible to repeat: a new default reading or
"What counts as X" entry in the digest, a new question for the second
reader here, or a new grading trigger. Record the round's numbers on the
cards board: readings, red asked, corrected, corrections missed by the
grading (green/amber found wrong later).
