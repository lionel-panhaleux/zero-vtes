---
name: engine-review
description: >
  Review a change to the zero-vtes game engine before it is committed: run on
  every diff touching crates/engine/src (cards included, generated tables
  excluded). A fresh reviewer checks that the change reuses the engine's
  mechanisms instead of duplicating them, applies every ruling, keeps card
  data in the definitions, and follows the comment policy. Use when finishing
  engine work (new cards, fixes, refactors), or when asked to review the
  engine or a diff of it.
---

# Engine review

The engine stays tight only if every change is checked against it as a whole.
The compiler and the definition-walking tests catch the mechanical part
(docs/engine.md §3, "Enforced and conventional"); this review covers what
needs judgement. It is not adversarial: "clean, nothing to change" is a
normal outcome.

## When

Before committing any change under `crates/engine/src` except the generated
`crypt.rs`, `library.rs`, `twda.rs` (CLAUDE.md: "run the engine-review skill
before every engine commit"). One review per commit; for a series (a card
round, a refactor), review each commit's diff, or the series once before the
first commit if the work is committed at the end.

## How

1. Make the change complete first: code, tests, ledger, rules-qa, and the
   CLAUDE.md checks green. The review reads a finished diff.
2. Launch a fresh reviewer: the Agent tool, `model: opus`, `effort: high`,
   general-purpose (or the `egress-reviewer` agent, given this checklist
   too: the `ship` skill's step 4), with the prompt below and the diff's range (e.g.
   `git diff main` in the worktree, or `HEAD~n..HEAD`). The author must not
   review its own diff in its own context: a fresh reader sees what the
   author's context hides.
3. Act on the report: fix each "do" finding (then re-run the checks) and
   each "consider" too, unless clearly outside the change's line (CLAUDE.md
   "Findings": the scope grows to take them); only what lies outside goes
   through the `intake` skill (findings are never left untracked). A
   reading the rules don't settle goes to a new rules-qa batch.
4. Record the review in the commit's trailers (the `ship` skill's format):
   `Egress: clean` or `Egress: <n> blocking fixed`, one
   `Advisory: <class>: …` per "consider", fixed or taken through `intake`
   (the ratchet counts classes, fixed or not).

## Reviewer prompt

Pass this, with `<RANGE>` and `<WORKTREE>` filled in:

> You review a change to the zero-vtes engine (a rules-enforcing VTES engine
> in Rust) before it is committed. Read only; change nothing. Work in
> `<WORKTREE>`; the diff is `git diff <RANGE>` (ignore the generated
> `crypt.rs`, `library.rs`, `twda.rs`).
>
> Read first: CLAUDE.md; docs/engine.md (sections 2–5 whole, and the section
> of section 6 for each mechanism the diff touches); the rules-qa items and
> ledger rows the diff cites. For each card the diff adds or changes, read
> its definition, its KRCG rulings (`data/rulings-v5.csv` rows) and its
> `../vtes-advanced-rules` entry.
>
> Not adversarial: report what is wrong or could be tighter, and say "clean"
> where it is. Do not invent findings. Check:
>
> 1. **Reuse.** Every new `Effect`, `Requirement`, `Hook`, `Modifier`,
>    `Interrupt` or engine function: does an existing one already express it
>    (search `cards/families.rs` by family, `cards/slots.rs` by slot, and
>    the interpreter of that family)? Is a new variant a parameterization of
>    an old one? Two copies of one body are a finding; two variants that look
>    alike but differ for a rules reason are fine if a comment says why.
> 2. **Data in definitions.** Card-printed numbers, names, clans and
>    Disciplines live in `cards/defs.rs` / `cards/vampires.rs`, never in
>    engine code (card names are enforced by `tests/lint.rs`: a new
>    `CARD_NAMES_ALLOWED` entry needs a real reason); no behaviour depends on which interpreter runs an effect
>    (a parameter belongs in the definition). Every vampire carries all its
>    text.
> 3. **Rulings.** The design satisfies each ruling of the cards touched
>    (KRCG rows, rules-qa, advanced rules), each backed by a test where the
>    ledger asks for one; rules-qa's default readings applied; a reading no
>    ruling settles is implemented and recorded as a question in a new
>    rules-qa batch.
> 4. **Live reads and legality.** Conditions read live, never snapshotted;
>    only illegal targets masked, legal no-ops kept (rules-qa 17.4); masks
>    generated from the definitions, no hand-written mask.
> 5. **Placement.** "Put this card on … / in play": the card itself enters
>    play through `place_now`; `place_held` only for a card already in the ash
>    heap; the test asserts the card in play is not `effect_only` and the ash
>    heap lacks it.
> 6. **Riders and abilities.** Riders fire through `fired_riders` /
>    `printed_riders` and their sites read `r.effects`; abilities go through
>    `ability_gate`; delays through `Held`/`Release` or `Seat::timed`; no
>    parallel mechanism beside these.
> 7. **Tests.** Each behaviour change has a test that fails without it; a
>    regression outside the ledger is not in `tests/rulings.rs`; a new card in
>    a fuzz fixture reaches its coverage key; goldens moved only where the
>    change explains it (the commit message names seeds and ranges).
> 8. **Comments** (docs/engine.md "Comment policy"): card text on
>    definitions, ruling citations on the code they justify, one-line whys;
>    no mechanism narrative in code (it belongs in docs/engine.md); vocabulary
>    variants' docs one or two lines, on the right variant.
> 9. **The guide.** If the change adds or changes a mechanism, docs/engine.md
>    says so in the right section; an enforced-vs-convention change updates
>    its §3 lists.
> 10. **Outside the engine.** A new player-visible effect is mapped in
>    `crates/host/src/effects.rs`, not left in its untold arms by default
>    (a place there is no proof of bookkeeping: many listed are untold
>    yet); `Choice` variants and their order unchanged, or the
>    policy/host/text impact stated.
>
> Report: a verdict line ("clean" / "N findings"), then findings ranked by
> value, each with file:line, what, why (the rule, ruling or section), the
> fix, and do / consider. Separate "looks alike, keep apart" notes. Say what
> you did not check.
