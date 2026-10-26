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
- **Advisory classes from one list** (accepted by the owner, 2026-10-22; the ratchet, 2026-10-17: one kind under several slugs: `duplicate-body`/`duplicated-body`, `trap`/`latent-trap`/`trap-undocumented`, `test-boundary`/`test-at-boundary-manual`): the ship skill's §5 lists the classes, one slug a kind, and `harness.py ratchet` counts the trailers since the last upkeep by them. Done when the list is in the skill and the command counts by it. Docs: the ship skill, the upkeep skill (§3).
