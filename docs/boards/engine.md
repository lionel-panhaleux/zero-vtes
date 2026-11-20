# Engine board

Goal: the engine clearly separated, complete and legal, agnostic of humans
and bots, behind an API convenient for platforms, agents and developers
(roadmap: architecture, objective C). Skill: `engine-review` on every
engine commit. Docs: `docs/engine.md`, `docs/protocol.md` (the contract,
semver), `crates/api` (`schema.json`, `openapi.json`, `protocol.ts`),
`docs/online-plan.md` (the architecture as built), DESIGN §4, §9. Covers
the engine's architecture, `api`, `host`, `text`, `server`, performance,
the rulebook mechanisms no card carries (card definitions and readings
are the cards board's), and the harness (CLAUDE.md, the `intake`, `ship`
and `upkeep` skills, `docs/index.md`).

A shrinking list (CLAUDE.md "Harness"; the milestone tokens: roadmap
"Milestones"). Order: what the community beta's games meet first (the server's
records and restarts, a rule players hit), then the protocol leaks the Succubus
Club fork works around, then the API for outside platforms and contributors,
then the training layer split, the remaining mechanisms, the review leftovers,
the harness.

- **The events tell what the per-seed sweep finds at bf80f6a** (Z-add, 2026-11-18; seeds at bf80f6a, round 34's catalog: `HOST_SEED_FROM=n HOST_SEEDS=6000 HOST_REDUCE=1 cargo test --profile ci -p zvtes-host --test events the_events_tell`, resumed past each panic): (1) a card's `contested` flag told by no event, set or cleared (3706, 4616, 4765, 4983; Pentex(TM) Subversion's among them); (2) a copy no event numbered ("a copy unnumbered"): a blocker of a Methuselah ousted mid-attempt still the action's `blocker` (2526), a combat still shown with a combatant gone (3373, 3505), an action's `source` card burned (4928); (3) a vampire's `kind` changing to `imbued` (712), a `red_list` trait lost (2525), cards stored on a minion (3968, 5128), two Trophy: Progeny (4561, the reducer's panic). Done when that sweep over 1–6000 passes at main, each fixed cause pinned by a test or a kept seed in `crates/host/tests/events.rs`. Docs: `docs/protocol.md` and its CHANGELOG if an event or a field is added (semver), else none.
- **harness.py stale names a changed rules-qa item's citations**: when a diff changes a rules-qa item's line (a superseded or narrowed reading), `stale` lists every page, digest entry, ledger row and code comment citing that item (`rules-qa N.M`, `(N.M)`), to re-read before review; the friction it closes: 59.14's three blocking rounds (e39549c), 100.1 and 76.2 left cited (b1de838), DESIGN:199 and Screamer after ac981e7. Done when a pytest shows `stale` listing a citing page for a changed item. Docs: CLAUDE.md "Harness" (the stale bullet), the ship skill §3.
