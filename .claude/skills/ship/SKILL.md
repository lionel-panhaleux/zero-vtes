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
cards and engine lines; or new work beyond the line's charter, a new
objective or feature, never a defect, gap or stale page the change
met): a rare event, not a way to defer.
Never leave the board longer than you found it, minus the line you
completed.

## 3. Land the trinity

Before review, in the change itself:

- the code and its tests, the CLAUDE.md checks green;
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
  change, or fixed.

## 4. Egress review

Spawn the `egress-reviewer` agent (`.claude/agents/egress-reviewer.md`)
with, and only with: the diff's range and worktree, the board line's
text as it was, the doc-impact pages, the test files the diff touches
(`git diff --stat <range> -- '*tests*' '*test_*'`), and the stub probe
of each behaviour the change writes into a standing page (the test, what
was broken). Not the conversation: a fresh reader sees what the author's
context hides. A diff touching `crates/engine/src` (generated tables
aside) also gets the `engine-review` charter: pass the skill's checklist
in the same prompt, or run that skill's reviewer alongside.

Then:

- **Blocking** findings: fix, re-run the checks, review again.
- **Advisory** findings: fixed in this change too, unless clearly outside
  its line (then `intake`, and the trailer says so); each recorded as a
  commit trailer.
- A third round still blocking: stop and take it to the owner,
  compressed to the one decision it turns on; three rounds that fail
  point at the approach, not its details (owner, 2026-10-25). Blocking
  rounds followed by one with no blocking finding need nobody: commit.

## 5. Commit

Once the checks are green and the review has no blocking finding left:
the owner's standing word (CLAUDE.md "Owner rules"); a red reading is
answered by the owner first. The message ends with the review's
trailers, which the upkeep's ratchet reads:

```
Egress: clean | <n> blocking fixed
Advisory: <class>: <one line>        (one per advisory finding, fixed or not)
Advisory: <class>: <one line> (intake: <board>)   (one taken out of scope)
Grading: <n> readings, <r> red asked, <c> corrected    (card rounds)
Grading-missed: <round> <rules-qa item>: <one line>  (a reading graded green/amber, found wrong later)
```

`<class>` is one of the classes below, so three of a kind can be
counted (`python3 scripts/harness.py ratchet`). Then the usual
attribution trailers.

The branch reaches `main` through Z-main, who merges a line's request
without asking the owner (CLAUDE.md "Sessions"): send it the branch, its tip
and what it changes for the other lines, and tell those lines yourself
("Between lines"). Z-main, after a merge: `python3 scripts/harness.py due`,
and `upkeep` when it is due.

### Advisory classes

One slug a kind, the first on its line; the ones after it are its older
spellings, counted with it (`*` a prefix; an exact slug wins over a
prefix, a longer prefix over a shorter). A finding of no kind here gets
the nearest, or a new line in this list.

- `stale-doc`: a page or doc comment says what is no longer true; also
  `stale-*`, `inaccurate-doc`, `doc-mismatch`, `doc-vs-code`,
  `false-text`, `dangling-reference`, `ledger-*`, `doc-overclaim`,
  `unbacked-claim`, `wrong-citation`, `miscount`, `contradiction`,
  `inconsistent-rule`, `rule-tension`.
- `stale-comment`: a code comment says what is no longer true; also
  `misleading-comment`, `inaccurate-comment`, `stale-docstring`,
  `comment-accuracy`, `comment-overclaims`, `overclaiming-comment`,
  `test-comment-overclaims`.
- `doc-precision`: true but vague or incomplete; also `imprecise-*`,
  `ambiguous-*`, `unclear-*`, `vague-*`, `doc-imprecise`, `wording`,
  `message-precision`, `comment-precision`, `comment-wording`, `doc-gap`,
  `unstated-case`, `underspecified-design`, `missing-context`,
  `example-too-narrow`, `unfounded-line`.
- `misplaced-doc`: knowledge on the wrong page, or a journal; also
  `misplaced-*`, `doc-placement`, `line-placement`, `journal-*`,
  `duplicated-doc`, `duplicate-doc`, `data-in-text`.
- `narrating-comment`: a comment narrating, not a trap; also
  `narrating-*`.
- `trap-undocumented`: a trap nothing states where it is read; also
  `trap`, `trap-*`, `latent-trap`, `doc-trap`, `hazard`, `hidden-*`,
  `non-local-*`, `implicit-coupling`, `behaviour-not-evident`.
- `test-gap`: a behaviour with no test failing without it; also
  `test-missing`, `missing-test`, `untested-*`, `test-coverage`,
  `test-incomplete`, `coverage-unasserted`, `unverified-done`,
  `done-condition-gap`, `fuzz-coverage`, `test-narrow`.
- `test-weak`: a test that would pass without the behaviour; also
  `test-weak-*`, `vacuous-*`, `test-vacuous`, `test-tautology`,
  `weak-oracle`, `loose-assertion`, `test-not-discriminating`.
- `test-weakened`: a test made weaker or deleted; also `rule-weakened`.
- `test-boundary`: tested below the engine's or the server's boundary;
  also `test-at-boundary-manual`, `test-below-boundary`,
  `unit-not-boundary`.
- `test-hygiene`: a test's name, noise or isolation; also
  `test-naming`, `test-clarity`, `test-noise`, `test-isolation`,
  `test-fragile`, `test-flaky`, `redundant-test`, `duplicated-test`,
  `duplicate-test`.
- `half-done`: a change stopped short (a second copy left, a site
  missed); also `half-done-*`, `missed-site`, `two-mechanisms`,
  `split-mechanism`, `primitive-bypassed`.
- `duplicated-body`: one body written twice; also `duplicate-*`,
  `duplicated-*`, `repeated-*`, `reuse`.
- `dead-code`: code nothing reaches or reads; also `dead-*`,
  `unreachable-path`, `redundant-*`, `leftover-*`, `misleading-code`.
- `defensive-bloat`: a check nothing reaches, a catch-all; also
  `defensive-*`, `catch-all-*`, `masking-check`.
- `shallow-module`: an interface hiding little; also `shallow-*`,
  `visibility-*`.
- `formatting`: wrap, width, layout; also `format`, `wrap-width`,
  `comment-wrap`, `comment-width`, `long-line`, `line-width`,
  `page-reflow`, `doc-reflow`, `stale-wrap`, `trailing-blank`, `style`,
  `cosmetic`, `doc-format`, `comment-format`, `layout`, `readability`,
  `schema-order`, `local-ordering`.
- `naming`: a misleading or colliding name; also `misleading-name`,
  `confusing-name`, `name-collision`.
- `info-leak`: hidden information reaching a seat; also `hidden-info`,
  `hidden-info-leak`, `visibility-leak`, `seed-leak`.
- `commit-message`: the message misstates or omits; also `commit-note`,
  `trailer-format`, `golden-unexplained`, `unannounced-edit`,
  `undeclared-departure`, `unexplained-behaviour-change`, `rebase-needed`.
- `card-data-in-engine`: card-printed data in engine code.
- `correctness`: a defect fixed in passing; also `edge-case`, `overflow`,
  `unchecked-input`, `index-mismatch`, `cost-order-mismatch`.
- `measure`: a measurement reported wrong or not reproducible; also
  `measure-error`, `unreproducible-measure`, `misleading-metric`.
- `reading-unrecorded`: a rules reading no rules-qa item records; also
  `unruled-reading`.
- `performance`: work done twice or too slowly; also `efficiency`.
