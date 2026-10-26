---
name: upkeep
description: >
  The maintenance loop of zero-vtes: board eviction, wiki lint and the
  harness ratchet, every 20 completed board lines or monthly, whichever
  comes first (python3 scripts/harness.py due). Use when ship reports
  upkeep due, when the owner asks for a maintenance pass, or when the
  boards or docs feel stale.
---

# Upkeep

Ingress is easy, eviction is not. A cycle is one board line deleted; a
pass is due after 20 cycles or 30 days (`python3 scripts/harness.py due`).
Deterministic counts come from `scripts/harness.py`; long scans run in
subagents (Haiku for lint sweeps, the session's model for verdicts).

## 1. Board eviction

`python3 scripts/harness.py age` lists every line with the cycles since
it was last touched; 20 or more is "on trial". Walk each board oldest
first, every on-trial line getting a verdict:

- **do**: still wanted; reword or reorder it (that touches it);
- **date**: waiting on a person becomes a dated ask;
- **promote**: not completable after all: its content goes to a page, the
  line is deleted;
- **drop**: deleted. Drops and reorderings are the owner's: propose them
  in the report; a drop that was a promise to someone is something to say
  to them (through the owner).

Then `harness.py size` against the last pass: the boards should shrink.

## 2. Wiki lint

The pages `docs/index.md` lists, plus CLAUDE.md and the skills:

- `harness.py orphans` clean; every page reachable from the index;
- contradictions between pages (a DESIGN decision vs engine.md vs
  protocol.md vs a skill);
- claims stale against the code: counts (DESIGN §11's count line, the
  boards' headers), file and function names, commands that no longer run;
- journals: done-row tables, dated plans, reasoning trails a page grew;
  each cut to "what is" (git keeps the rest);
- pages nothing cited or changed for 20 cycles, challenged: still
  standing knowledge, or merged into another page or deleted;
- the weekly deep sweeps ran: a report a week in the main checkout's
  `data/sweeps/` (`scripts/sweeps.py`, Z-main's schedule), each failure
  through `intake`;
- domain claims: sources still current (the KRCG data's date, the
  advanced-rules repo); rules-qa readings are the owner's and do not age,
  but a contradiction with a newer ruling goes to the digest's §23.

Each finding is fixed in the pass when small, or goes through `intake`.

## 3. Harness ratchet

Recurring problems are harness problems, not code problems. Since the
last pass, `python3 scripts/harness.py ratchet` counts the `Advisory:`
trailers by the ship skill's "Advisory classes" (an older spelling with
its class), marking three of a kind, and names the slugs no class lists:
each joins a class there, or a new one. The readings missed:

```
git log <last-upkeep>..HEAD --format=%B | grep -E '^Grading-missed:'
```

Three of a class (or three owner corrections of one kind, a rule the
owner restated, a reviewer finding that keeps coming back) earn an
amendment: a CLAUDE.md rule, a skill line, an egress charter line, a lint
test (`crates/engine/tests/lint.rs`), a default reading. Propose them to
the owner in the digest, their text in the pass's commit message, tracked
by one dated ask on the engine board; each accepted one becomes its line.

## 4. The frame (monthly)

Are the roadmap's objectives and ranking still true; should anything in
"Deferred by the owner" come back; is total board size trending down?
Questions for the owner, short, with a recommendation.

## 5. Land

One commit, once its checks and egress are clean (CLAUDE.md "Owner
rules"), with the trailer that marks the pass for
`harness.py`:

```
Upkeep: <cycles> cycles, <n> lines evicted, <m> pages linted, <k> amendments proposed
```
