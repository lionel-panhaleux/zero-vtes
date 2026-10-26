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

A shrinking list (CLAUDE.md "Harness"): one line an ask, its done-condition
and the docs it changes; done is deleted; order is priority; the only
state is `[doing: <session>]`. Order: what the community beta's games meet
first (the server's records and restarts, a rule players hit),
then the protocol leaks the Succubus Club fork works around, then the API
for outside platforms and contributors, then the training layer split,
the remaining mechanisms, the review leftovers, the harness.

- **The deep sweeps' first report triaged**: Z-main runs `python3 scripts/sweeps.py` weekly from the main checkout (`FUZZ_SEEDS=600`, `HOST_SEEDS=400`), the first asked of Z-main 2026-10-26, to run once the d16 training run ends; each failure through `intake`, its seed kept in the sweep (`KEPT` in crates/host/tests/events.rs, a fixture's `seeds`). Done when the first report is triaged. Docs: none.
- **Ask the owner whether to take three CLAUDE.md corrections** (upkeep, 2026-10-26; their text in b645982's message; the ratchet's three amendments taken and built): CLAUDE.md's training decks (the precons and DESIGN §3's), its online plan's order of work (the boards' and the roadmap's), its Harness counts with `ratchet`. Done when the owner has answered and each accepted correction is in CLAUDE.md. Docs: CLAUDE.md.
