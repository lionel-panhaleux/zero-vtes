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

- **The events tell a retainer's life counter as it enters a contest** (Z-add, 2026-11-14, round 30's deep host sweep, predating it): `HOST_SEED_FROM=401 HOST_SEEDS=1150 cargo test --profile ci -p zvtes-host --test events the_events_tell` at seed 802, decision 232, a path the events miss: `.seats[].ready[c24].cards[c61].counters: null -> 1` — a Mr. Winthrop seat 3 employs while seat 0 already has one, entering straight into a contest, its life counter told by no event (the next view shows it). The same sweep's 401–650 and 901–1150 ranges stopped earlier at the "vampire moved within its region" check, so they were not checked for misses: reproduced and fixed here, or placed on their own if their cause is apart. Done when that sweep (401–1150) passes at main with seed 802 among `crates/host/tests/events.rs`'s kept seeds. Docs: `docs/protocol.md` and its CHANGELOG if an event or a field is added (semver), else none.
- **The events tell seven more changes the sweep finds seed by seed** (Z-add, 2026-11-18, round 32's per-seed sweep 1–6000, each seed alone: `HOST_SEED_FROM=n HOST_SEEDS=n HOST_REDUCE=1 cargo test --profile ci -p zvtes-host --test events the_events_tell`, the cross-seed totals ignored; all on main d2b1aca, the seeds main's, round 32's catalog reshuffling them): (1) Spider's burn when his blocker is still ready (`Stat::BurnsIfBlockerReady`) has no cause, seed 2607; (2) Rumble's end-of-action pool burn has no cause, 1941; (3) Nocturn's failed bleed tells BlockFails after ActionEnds, 1873; (4) Pentex(TM) Subversion's `contested` flag on a contested New Blood is told by no event (the view clears then sets it), met in round 32's catalog only (its seeds 993, 4185); (5) Mina Grotius's 3 blood when a neighbour's vampire burns, on a contest yield, has no cause (round 32's 5113); (6) Antara's −1 blood as a card enters play has no cause, 2536; (7) "a copy unnumbered", 1801. Done when that per-seed sweep over 1–6000 passes at main, each fixed cause pinned by a kept seed in `crates/host/tests/events.rs`. Docs: `docs/protocol.md` and its CHANGELOG if an event or a field is added (semver), else none.
- **harness.py stale names a changed rules-qa item's citations**: when a diff changes a rules-qa item's line (a superseded or narrowed reading), `stale` lists every page, digest entry, ledger row and code comment citing that item (`rules-qa N.M`, `(N.M)`), to re-read before review; the friction it closes: 59.14's three blocking rounds (e39549c), 100.1 and 76.2 left cited (b1de838), DESIGN:199 and Screamer after ac981e7. Done when a pytest shows `stale` listing a citing page for a changed item. Docs: CLAUDE.md "Harness" (the stale bullet), the ship skill §3.
