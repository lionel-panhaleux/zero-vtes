---
name: upkeep
description: >
  The maintenance loop of zero-vtes: eviction over board lines, milestones,
  plan and watch items, the docs review, and the retrospective that alone
  changes the harness, every 20 completed board lines or monthly, whichever
  comes first (python3 scripts/harness.py due). Use when the watch file's
  check reports upkeep due, when the owner asks for a maintenance pass, or
  when the boards or docs feel stale.
---

# Upkeep

Intake is easy, eviction is not. A cycle is one board line deleted; a
pass is due after 20 cycles or 30 days (`python3 scripts/harness.py due`,
a daily watch item). Deterministic counts come from `scripts/harness.py`;
long scans run in subagents (Haiku for sweeps and transcripts, the
session's model for verdicts). A finding small and settled is fixed in
the pass; one someone must decide is proposed to them, never fixed in
place.

## 1. Eviction

- **Lines.** `python3 scripts/harness.py age` lists every line with the
  cycles since it was last touched; 20 or more is "on trial". Walk each
  board oldest first, every on-trial line getting a verdict: **do**
  (still wanted: reword or reorder it, which touches it), **watch** (it
  waits on a date, a person or an event: it becomes a `docs/watch.md`
  item), **promote** (not completable after all: its content goes to a
  page, the line is deleted), **drop** (deleted). Drops and reorderings
  are the owner's: propose them; a drop that was a promise to someone is
  something to say to them (through the owner).
- **Milestones.** `python3 scripts/harness.py levels`: each open
  milestone's lines by board, the tokens no milestone names, the
  milestones no line serves. A milestone nobody served since the last
  pass is a subject or a drop, proposed to the owner like a line.
- **Watch items.** Each recurring one still needed, each dated one not
  past its date unfired.
- `harness.py size` against the last pass: the boards should shrink.

## 2. Docs review

The pages `docs/index.md` lists, plus CLAUDE.md, the skills and the
agent:

- **contradictions** between pages that overlap by construction (a
  DESIGN decision vs engine.md vs protocol.md vs a skill vs CLAUDE.md);
- **stale claims**: sample the load-bearing ones against the code; names,
  paths, counts (DESIGN §11's count line, the boards' headers), lists and
  step-by-step procedures rot fastest;
- **domain claims** have only their sources: the KRCG data's date, the
  advanced-rules repo; rules-qa readings are the owner's and do not age,
  but a contradiction with a newer ruling goes to the digest's §23;
- **orphans**: `harness.py orphans` clean; pages nothing cited or changed
  for 20 cycles challenged: still standing knowledge, or merged or
  deleted;
- **size**: a page that became a summary of a summary folds into its
  parent; a journal (done-row tables, dated plans, reasoning trails) cut
  to what is;
- **hazards**: an entry the code or a page now makes impossible leaves
  `docs/hazards.md`;
- the weekly sweeps ran: a report a week in the main checkout's
  `data/sweeps/` (docs/watch.md), each failure through `intake`.

## 3. Retrospective

The only way the harness changes (CLAUDE.md "Harness"). Its inputs since
the last pass:

- `python3 scripts/harness.py friction`: the `Friction:` trailers by kind
  (the ship skill's "Friction kinds"; the older `Advisory:` and
  `Grading-missed:` trailers read as `review` and `reading`);
- where available, the sessions' transcripts
  (`~/.claude/projects/<the checkout's path, its / as ->/*.jsonl`), read
  by a Haiku subagent that returns, verbatim and dated, the owner's
  corrections and restated rules and the steps a session redid.

Then:

- **The same friction twice** (two trailers or quotes naming the same
  thing, not merely the same kind) becomes a proposed rule: its text, the
  evidence (the commits, the quotes), one line of why, and where it goes
  (a CLAUDE.md rule, a skill line, a `docs/hazards.md` entry, an egress
  charter line, a default reading). A correction the owner has had to
  repeat becomes a mechanical check: a test in `crates/engine/tests/lint.rs`
  or a `harness.py` check (this repo's hooks, CLAUDE.md "Harness").
- **Lapsed rules.** Walk CLAUDE.md, the loop skills and the agent: a rule
  whose cause is gone, or with no reason anyone can give, is proposed for
  removal, as readily as a rule is added. A reason too long for the rule
  belongs on the page the rule cites.
- Nothing joins the harness because it seems useful.

Proposals go to the owner in Z-main's digest (CLAUDE.md "Ask the owner":
one decision a message, the recommendation first), their text in the
pass's commit message; the answer awaited is a dated watch item; each
accepted one is made in the pass, or becomes an engine-board line when it
is code.

## 4. The frame (monthly)

Is the plan still true (docs/roadmap.md): who it is for, the phase (has
its trigger happened?), the objectives and the ranking; should anything
in "Deferred by the owner" come back; is total board size trending down?
Questions for the owner, short, with a recommendation; a phase change is
the owner's and may change the harness.

## 5. Land

One commit, through ship §3–5 (a pass of pages only skips the review),
with the trailer that marks the pass for `harness.py`:

```
Upkeep: <cycles> cycles, <n> lines evicted, <m> pages reviewed, <k> rules proposed, <r> removals proposed
```
