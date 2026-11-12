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
"Default readings" and "What counts as X" sections always). Work in the
shared checkout on `main` (CLAUDE.md "Sessions": trunk-based; a
regenerated table or a re-bless records the whole tree, docs/hazards.md
"The shared tree"); parallel implementers, if used, each in a temporary
worktree under `.claude/worktrees/`, merged by Z-add into the shared
checkout before the round's review, then removed (owner, 2026-11-10);
keep a progress file in the job's tmp (done, todo, readings, commit
checklist): a round outlives one context window.

A standing rule (owner, 2026-11-11, after a Liquefy reading stated from
memory, "since its text says so", that its text does not say), beside
CLAUDE.md "Owner rules"' "A settled decision changed is examined first":

- **Never from memory.** Every claim about a card's text, a ruling or a
  rules-qa item, to the owner, a peer or an agent, is read from its
  source as it is made (`vtes.json`'s text and rulings, the ledger,
  rules-qa, `../vtes-advanced-rules`) and quoted.

## 1. Choose the round

Start from `python3 scripts/coverage.py <vtes.json> --twda <twda.json>
[--demand <counts.json>]`: the pool's coverage by type (and by set with
`--sets`), the TWDA decks in the pool and kept, and the ids players asked
for most (the server's `/admin/unsupported` counts, from the 1.8.0 deploy
on: they need the admin token, so ask Z-main or the owner for the file —
`curl -H "Authorization: Bearer $(cat ~/.config/zvtes/admin_token)"
https://engine.krcg.org/admin/unsupported > demand.json`, run by whoever
holds the token, never printed). Its summary goes in the cards board's
header at the round's end.

A round is whole sets, the cards board's top line (owner, 2026-10-30: "we
are now working set by set and stopped running twda analysis to decide what
cards to add"; 2026-11-09: board lines by groups of sets, every card in the
end, promos last). `python3 scripts/round_choice.py <vtes.json> --twda
<twda.json> --set NAME… --steps 0 --cut 0` prints the chosen sets' missing
ids and what `gen_twda.py`'s rules keep with them (kept, recent kept: a
set's versions can displace kept decks, `VERSIONS`, `SAME_CRYPT`,
`SAME_LIBRARY`), for the board's header, not the choice.
Count cards by id, never by splitting names on commas. Then add the round
to `ADDED_BATCHES` in `scripts/window_matrix.py` (the owner's words in a
comment) and regenerate `crypt.rs`, `library.rs`, `twda.rs`; check the
kept count equals the simulation's. Append the new library cards' draft
rows to `data/window-matrix-reviewed.csv` and review them against analog
rows: when a draft is wrong, fix the generator's pattern, not the row
(`--check` at 0 divergences). Regenerate the ledger
(`scripts/rulings_v5.py vtes.json > data/rulings-v5.csv`). Both CSVs are
CRLF: the csv module with `lineterminator="\r\n"`, or byte-wise.

A newly released set (round 27): download KRCG's `vtes.json`, `twda.json`
and `v5/expansions.json` into the job tmp; list the new ids by their set
and bundle codes, and add them as one `ADDED_BATCHES` batch, so every
card already in the tables keeps its index (the observation's). A set
printed as bundles of an existing one (KRCG's `V5` "SoNY", "NBr") enters
the pool as soon as the tables regenerate from that `vtes.json`: take it
as its own round, or hold its bundles out of `in_pool` until then; a
bundle KRCG types "Precon" that is no deck goes in `PACK_BUNDLES`, or the
precon tables refuse it. Then the steps above: regenerate, the matrix drafts,
the ledger (its generator reads `../vtes-advanced-rules`, its
submodules initialised).

## 2. Research each card, in this order

1. Its text and every KRCG ruling (`vtes.json`), and the
   `../vtes-advanced-rules` rulings YAML for its id (group rulings too).
2. Its ledger rows (`data/rulings-v5.csv`), those mentioning it included
   (`via`: `mention in <card>`): the rulings are the test plan, each row
   `test:` or `n/a:` before the round lands (`--check` refuses `blocked:`;
   six rulings sat unapplied behind `blocked:untested`, Batch 94).
3. `docs/rules-digest.md` sections for the card's mechanisms, then the
   rules-qa items they cite.
4. The closest existing definitions (an Explore subagent maps the round's
   cards to analog consts and missing variants; trust const names, not line
   numbers).
5. The rulebook (`vtes` skill) for the mechanism's general rule.

## 3. Design

- Reuse first: an existing `Effect`/`Requirement`/`Modifier`, with a field
  or filter added if needed, over a card-named variant. `Effect::family()`
  and `Requirement::slots()` are exhaustive: classify a new variant there
  (a prevention says so in `family`'s match: `Effect::prevents`); the
  definition-walking tests say where it may sit.
- A new variant only for a new mechanism, named for the mechanism. If the
  card's shape has no primitive, build the primitive; never bend the
  reading to the nearest existing one (that is how 71.21 and 71.25 went
  wrong).
- New interrupts, purposes, choice kinds or window labels: api ids, host
  ids/describe, text prompt/option, regenerate schema/openapi/protocol.ts,
  a `CHANGELOG.md` line under the coming protocol version
  (`scripts/schema_compat.py` lists what the schema adds); a new choice
  kind, window label or action kind also moves the observation encoder's fixed id
  tables: tell the training session before committing (interrupts and
  purposes are not encoded).
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
  Methuselah's minions targets them; a card another card plays or puts
  in play is played, by that card and not in the normal fashion (no
  as-played window, no minion-plays trigger). It answers, per card:
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

Diff the two reads yourself, item by item, then grade each reading. A
"during" ability defined repeatable fails `during_abilities_are_once`
(`tests/lint.rs`): limit it, or say in its exception list how it is once
(a rider the engine spends, a one-shot) — never list one as repeatable
(rules-digest D7).

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
py side, generators, window matrix, ledger, DESIGN's definitions count) and
`python3 scripts/digest_index.py --check` (every rules-qa item cited in
the digest, its line in the section stating it, and §25 rewritten); the
`engine-review` skill on the round's diff (the `ship` skill's egress);
anything found fixed in the round, only what is clearly outside it taken
through `intake`; the round's line deleted from the
cards board (`docs/boards/cards.md`).
Every "not in the pool" or "no … in the pool" clause in the definitions
and pages (`grep -rn -i "in the pool" crates/engine/src/cards DESIGN.md
docs`) is re-checked against the round's cards: one whose tenant the
round added is implemented in the round, its rules-qa item and comment
corrected (owner, 2026-11-11: seven were found stale at once).
Ask the red items before committing (through Z-main's digest, or the
owner directly): a "no" lands in the same commit, and the commit waits
for their answers only. Commit the round's files by path (CLAUDE.md
"Sessions"; a generated table another commit moved meanwhile is
regenerated, never hand-merged), and tell the training session the
vocabulary deltas (library n→m, vampires, kept decks) and any new encoder
id, and Z-main what it changes for a deploy. The round's
rules-qa batch number comes from Z-main, asked before the batch is written
(CLAUDE.md "Sessions").

## 7. Learn from every correction

When the owner corrects a reading (or a game shows one wrong), find the
root cause and make it impossible to repeat: a new default reading or
"What counts as X" entry in the digest, a new question for the second
reader here, or a new grading trigger. Record the round's numbers as its
commit's `Grading:` trailer (readings, red asked, corrected), and a
correction the grading missed (a green or amber reading found wrong later)
as a `Friction: reading: <round> <rules-qa item>: …` trailer on the fix
(the `ship` skill's format): the `upkeep` retrospective reads them.
