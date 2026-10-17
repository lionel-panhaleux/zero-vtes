# zero-vtes docs: the map

The standing pages: what the product is, what was decided, and the rules
it implements. Every page is reachable from here (`python3
scripts/harness.py orphans` checks it). Lifespans (CLAUDE.md "Harness"):
the code says how; these pages say what is and why; plans, findings and
reasoning journeys live in a task and die with it, git keeping them.

## The product

- [`../README.md`](../README.md) and [`../CONTRIBUTING.md`](../CONTRIBUTING.md):
  the way in for outsiders: what is here, a quick start, how a change is
  made.
- [`../DESIGN.md`](../DESIGN.md): what the engine and the agent are: scope
  (§3), the engine's architecture (§4), the RL surface (§5), the window
  census (§6), testing (§7), the community surface (§8), performance
  targets (§9), modeling decisions (§10), phasing and the training runs
  (§11), the glossary, the resolved questions.
- [`engine.md`](engine.md): the engine guide: dispatch, what is enforced,
  adding a card, the comment policy, the reference by mechanism.
- [`protocol.md`](protocol.md): the contract between the server and its
  clients (semver; embedded in `crates/api/openapi.json`).
- [`online-plan.md`](online-plan.md): the online architecture as built
  (`api` ← `host` ← `server`, `text`).
- [`human-play.md`](human-play.md): how the first fork was built; its
  "Checks" and "Running the fork".
- [`../data/records/README.md`](../data/records/README.md): the curated
  game records.

## The rules (domain)

- [`rules-qa.md`](rules-qa.md): the owner's rulings, one line each, dated
  by batch (the owner is a VEKN judge); cited as `rules-qa N.M`.
- [`rules-digest.md`](rules-digest.md): rules-qa's settled readings by
  mechanism, the contradictions and the open readings.
- Outside the repo: the `vtes` skill (rulebook, tournament rules, judges'
  guides), `../vtes-advanced-rules`, the KRCG rulings (CLAUDE.md).

## Platforms

- [`platforms/sc.md`](platforms/sc.md): Succubus Club: Thomas's style,
  branches, PRs and his reactions, recipes, traps.

## The work

- [`roadmap.md`](roadmap.md): the objectives, the ranking between lines,
  the milestone, what the owner deferred.
- The boards, one per line of work, each a shrinking list:
  [`boards/cards.md`](boards/cards.md), [`boards/engine.md`](boards/engine.md),
  [`boards/training.md`](boards/training.md),
  [`boards/platforms.md`](boards/platforms.md).
- [`sessions.md`](sessions.md): the launch prompt of each line's session
  (CLAUDE.md "Sessions").
- The harness: `../CLAUDE.md` (the rules every session reads) and
  `.claude/skills/` (`intake`, `ship`, `upkeep`; the lines' `add-cards`,
  `engine-review`, `training`, `sc-change`), `.claude/agents/egress-reviewer.md`.
