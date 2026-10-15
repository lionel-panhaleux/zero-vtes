---
name: ship
description: >
  The execution loop of zero-vtes: take the top line of your line's board
  (docs/boards/*.md), do it with the line's own skill, and land it as one
  change — code, the doc-impact pages, the board line deleted — reviewed
  by a fresh-context egress reviewer before the commit. Use when starting
  or finishing any unit of board work.
---

# Ship

A unit of work lands as one change: code changed, the line's doc-impact
pages updated (or "none" justified), the board line deleted. Wiki
currency is mechanical, not aspirational.

## 1. Take the line

The top line of your board without a `[doing: …]` marker, unless the
owner names another. Mark it `[doing: <session>]`. Read what the line
points at and the pages it names. A session edits its own board only: a
line for another board is sent to that line's session (or to Z-main when
it crosses lines), which runs `intake` on it.

## 2. Do it

Follow the line's skill: `add-cards` (cards), `engine-review` (engine
commits), `training`, `sc-change`. Work in the line's worktree (CLAUDE.md
"Harness": worktrees stay). Task notes (plan, progress, hazards) go in the
session's job tmp; they die with the task.

Scope grows in place (owner, 2026-10-15): what the work discovers (a
bug, a rough edge, a stale page, a missing test) and every relevant
review finding are done in this change, the minor ones too. `intake`
takes only what is clearly outside the line (work for another line:
code another session works in, the engine crates being shared by the
cards and engine lines; or new work the owner did not ask of this
line): a rare event, not a way to defer.
Never leave the board longer than you found it, minus the line you
completed.

## 3. Land the trinity

Before review, in the change itself:

- the code and its tests, the CLAUDE.md checks green;
- each page the line named updated, or the commit says why none moved;
- the board line deleted (not marked done; git keeps it).

## 4. Egress review

Spawn the `egress-reviewer` agent (`.claude/agents/egress-reviewer.md`)
with, and only with: the diff's range and worktree, the board line's text
as it was, the doc-impact pages, and the test files the diff touches
(`git diff --stat <range> -- '*tests*' '*test_*'`). Not the conversation:
a fresh reader sees what the author's context hides. A diff touching
`crates/engine/src` (generated tables aside) also gets the
`engine-review` charter: pass the skill's checklist in the same prompt,
or run that skill's reviewer alongside.

Then:

- **Blocking** findings: fix, re-run the checks, review again.
- **Advisory** findings: fixed in this change too, unless clearly outside
  its line (then `intake`, and the trailer says so); each recorded as a
  commit trailer.
- After two rejection rounds, stop and take the disagreement to the
  owner, compressed to the one decision it turns on.

## 5. Commit

On the owner's word (CLAUDE.md). The message ends with the review's
trailers, which the upkeep's ratchet reads:

```
Egress: clean | <n> blocking fixed
Advisory: <class>: <one line>        (one per advisory finding, fixed or not)
Advisory: <class>: <one line> (intake: <board>)   (one taken out of scope)
Grading: <n> readings, <r> red asked, <c> corrected    (card rounds)
Grading-missed: <round> <rules-qa item>: <one line>  (a reading graded green/amber, found wrong later)
```

`<class>` is a short kebab-case slug ("shallow-module", "stale-doc",
"test-weakened", "card-data-in-engine"), reused across commits so three
of a kind can be counted. Then the usual attribution trailers.

The branch reaches `main` through Z-main, who merges a line's request
without asking the owner again (CLAUDE.md "Sessions"; the commit itself
was on the owner's word): send it the branch, its tip and what it changes
for the other lines, and tell those lines yourself ("Between lines").
Z-main, after a merge: `python3 scripts/harness.py due`, and `upkeep`
when it is due.
