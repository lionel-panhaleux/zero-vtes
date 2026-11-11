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
points at, the pages it names, and the sections of `docs/hazards.md`
naming what it touches. A session edits its own board only: a
line for another board is sent to that line's session (or to Z-main when
it crosses lines), which runs `intake` on it.

## 2. Do it

Follow the line's skill: `add-cards` (cards), `engine-review` (engine
commits), `training`, `sc-change`. Work in the shared checkout on `main`
(CLAUDE.md "Sessions": trunk-based, the tree shared; "Harness" names the
two temporary worktrees). Task notes (plan, progress, hazards) go in the
session's job tmp; they die with the task.

Scope grows in place (owner, 2026-10-15): what the work discovers (a
bug, a rough edge, a stale page, a missing test) and every relevant
review finding are done in this change, the minor ones too. `intake`
takes only what is clearly outside the line (work for another line:
code another session works in, the engine crates being shared by the
cards and engine lines; or new work beyond the line's charter, a new
objective or feature, never a defect, gap or stale page the change
met): a rare event, not a way to defer.
Never leave the board longer than you found it, minus the line you
completed.

## 3. Land the trinity

Before review, in the change itself:

- the code and its tests, the checks green (docs/operations.md
  "Checks"); the tests that only drove the work dropped, the main path
  and the failure modes that matter kept (CLAUDE.md "Harness");
- every comment repeating a page, another comment or the code deleted;
- every guard, clamp or fallback for a state no input of today's pool
  or protocol reaches deleted (the reviewer's item 5; owner,
  2026-11-09: it removed one in 10 commits between two upkeeps);
- each assertion pinned to the behaviour claimed (the zone reached, the
  cause and its source, the action ended), never a weaker predicate the
  defect also passes (`!in_play` for "in the ash heap", any cause for
  "this source's"; owner, 2026-11-09: 7 such tests passed without their
  fix between two upkeeps);
- for each behaviour the change writes into a standing page (a rule, a
  guarantee, a "never"), a test that fails without it, shown so: a stub
  probe (the behaviour removed or broken, the test failing, then
  restored), named in the commit message. The Succubus Club fork has no
  test suite: a behaviour the engine or the protocol guarantees is probed
  on a zvtes test (the engine's or the host's); one of the fork alone is
  named in the commit as "no fork test suite: checked by reading (egress)
  and vue-tsc";
- each page the line named updated, or the commit says why none moved;
- the board line deleted (not marked done; git keeps it);
- `python3 scripts/harness.py stale main` clean: no name the diff
  removes (an identifier, a heading, a path) still cited by a page, the
  harness or a code comment; no line it adds to a wrapped page or a
  whole-line comment past 80 columns, nor a stub line next to a line it
  adds (a paragraph or comment block of three lines or more); a hit is
  fixed, or, when the citation is right (a name that lives on
  elsewhere), said in the commit. Its re-read list (a function whose
  body changed under an unchanged doc, a page line citing a changed
  function or variant) is gone through: each still true after the
  change, or fixed. In the shared tree it also reads the other
  sessions' uncommitted edits: a hit in a file not yours is theirs.

## 4. Egress review

A change of pages only (no code, test, script, skill or agent) skips
review: its trailer is `Egress: none (docs only)`. Any other change is
reviewed.

Spawn the `egress-reviewer` agent (`.claude/agents/egress-reviewer.md`)
with, and only with: the checkout and the diff's range (the change's
paths, uncommitted: `-- <paths>`, new files named), the board line's text
as it was, the doc-impact pages, the sections of `docs/hazards.md` the
diff touches, the test files the diff touches, and the stub probe of each
behaviour the change writes into a standing page (the test, what was
broken). Not the conversation: a fresh reader sees what the author's
context hides. A diff touching `crates/engine/src` (generated tables
aside) also gets the `engine-review` charter: pass the skill's checklist
in the same prompt, or run that skill's reviewer alongside.

The line that closes a milestone (the last line carrying its
`[M:<slug>]`, roadmap "Milestones") is reviewed with the whole
milestone: add its done-condition and its commits (`git log --grep
'M:<slug>'`), the reviewer checking the condition over all they changed;
the milestone is deleted from the roadmap in the same change.

Then:

- **Blocking** findings: fix, re-run the checks, review again.
- **Advisory** findings: fixed in this change too, unless clearly outside
  its line (then `intake`, and the trailer says so).
- A third round still blocking: stop and take it to the owner,
  compressed to the one decision it turns on; three rounds that fail
  point at the approach, not its details (owner, 2026-10-25). Blocking
  rounds followed by one with no blocking finding need nobody: commit.

## 5. Commit

Once the checks are green and the review has no blocking finding left:
the owner's standing word (CLAUDE.md "Owner rules"); a red reading is
answered by the owner first. On `main`, your files only, by path
(`git commit -- <paths>`; CLAUDE.md "Sessions"). The message names the
line's milestone token when it has one, and ends with its trailers,
which the upkeep's retrospective reads:

```
Egress: clean | <n> blocking fixed | none (docs only)
Friction: <kind>: <one line>      (one per friction met, see below)
Friction: review: <one line> (intake: <board>)   (a finding taken out of scope)
Friction: none                    (when the change met none)
Grading: <n> readings, <r> red asked, <c> corrected    (card rounds)
```

Then the usual attribution trailers. Tell the lines what the commit
changes for them ("Between lines") and Z-main what it changes for a
deploy; Z-main pushes.

### Friction kinds

What slowed the change or would have gone wrong, one trailer each, the
one line saying what happened, specifically enough that the same
friction met twice can be recognised (the file, the rule, the check):

- `correction`: the owner corrected the session or restated a rule
  (their words, short).
- `review`: a finding of the egress or engine review, blocking or
  advisory, fixed or taken through `intake`.
- `round`: a review round beyond the first, and what made it needed.
- `surprise`: the code, a page or a tool was not what it seemed (a trap:
  a docs/hazards.md candidate).
- `check`: a check failed other than as the change meant it to, and why.
- `reading`: a reading graded green or amber found wrong later (the
  round, the rules-qa item).
