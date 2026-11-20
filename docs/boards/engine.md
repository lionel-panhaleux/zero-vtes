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

- **Cut printed_mod's cost** (owner, 2026-11-20, via Z-main: "fuzz is far too long ... slowing us down"): the fuzz suite and self-play spend about 75% of their CPU in `GameState::printed_mod` / `cards::vampire_of` (Z-main's `sample` profile at 203992f: `vampire_of` 42% self, `printed_mod` 33%; hottest callers `settle_printed_burns`, `virtual_hand`, `text_suppressed`, `seat_reducers`, `blood_payers`), every stat read scanning every minion's printing; the cost grows with the crypt and the table, and hits training and the server alike. Done when the fuzz suite's CPU time drops by at least half at unchanged goldens (no game moves), conditions still read live (owner rule: never snapshotted). Docs: `docs/engine.md` if a mechanism changes, DESIGN §9 if numbers are quoted.
- **The events tell five more changes the sweep finds seed by seed** (Z-add, 2026-11-18, round 32's per-seed sweep 1–6000, each seed alone: `HOST_SEED_FROM=n HOST_SEEDS=n HOST_REDUCE=1 cargo test --profile ci -p zvtes-host --test events the_events_tell`, the cross-seed totals ignored; all on main d2b1aca, the seeds main's, round 32's catalog reshuffling them): (1) a hunt's action view at `resolved` told by no event, `.action: null -> {kind: hunt, step: resolved}` (main e202ddd's 959, decision 236, met in the 401–1150 sweep); (2) Pentex(TM) Subversion's `contested` flag on a contested New Blood is told by no event (the view clears then sets it), met in round 32's catalog (its seeds 993, 4185) and on main e202ddd (993, decisions 117 and 218, in the 401–1150 sweep); (3) Mina Grotius's 3 blood when a neighbour's vampire burns, on a contest yield, has no cause (round 32's 5113); (4) Antara's −1 blood as a card enters play has no cause, 2536; (5) "a copy unnumbered", 1801. Done when that per-seed sweep over 1–6000 passes at main, each fixed cause pinned by a kept seed in `crates/host/tests/events.rs`. Docs: `docs/protocol.md` and its CHANGELOG if an event or a field is added (semver), else none.
- **harness.py stale names a changed rules-qa item's citations**: when a diff changes a rules-qa item's line (a superseded or narrowed reading), `stale` lists every page, digest entry, ledger row and code comment citing that item (`rules-qa N.M`, `(N.M)`), to re-read before review; the friction it closes: 59.14's three blocking rounds (e39549c), 100.1 and 76.2 left cited (b1de838), DESIGN:199 and Screamer after ac981e7. Done when a pytest shows `stale` listing a citing page for a changed item. Docs: CLAUDE.md "Harness" (the stale bullet), the ship skill §3.
