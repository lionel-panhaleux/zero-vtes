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
- **Turn-scoped minion fields into `Minion::timed`**: fold `bleed_bonus_turn` (Andre LeRoux) and `unlock_end_of_turn` (Enkidu) into round 12's `Vec<(Lasting, Expiry, SeatId)>` as `Lasting` variants, swept at the same lines. Done when both fields are gone and the goldens moved in hashes only. Docs: engine.md (timed state).
- **DESIGN.md as a standing page**: what the engine and the agent are and the decisions that hold, each with one line of why where the rejected option is attractive; out: the slice table's narratives and superseded plans (§11.2 is the training board's "DESIGN §11.2 as the run record"), git keeping them. Done when every section is "what is" (the upkeep's wiki lint passes it) and `docs/index.md` points at its parts. Docs: DESIGN.md, `docs/index.md`, the citations of moved sections.
- **`docs/online-plan.md` and `docs/human-play.md` as standing pages**: the architecture as built, "Checks" and "Running the fork" kept; the done-row tables and the history out. Done when neither holds a done row or a dated plan step. Docs: both, `docs/index.md`.
- **A page's promise has its test, shown failing** (accepted by the owner, 2026-10-22; the ratchet, 2026-10-17: `half-done` 10, `test-gap` 8 and its kin; `narrating-comment` 5, `misplaced-doc` 4, `trap-undocumented` 3; `defensive-bloat` 3): `ship` step 3 asks, for each behaviour the change writes into a standing page, a test that fails without it, the stub probe named in the commit; the egress charter applies docs/engine.md's comment policy to every crate (not only engine commits) and cuts a check nothing reaches. Done when the ship skill and `.claude/agents/egress-reviewer.md` say so. Docs: the ship skill, the egress-reviewer agent.
- **Advisory classes from one list** (accepted by the owner, 2026-10-22; the ratchet, 2026-10-17: one kind under several slugs: `duplicate-body`/`duplicated-body`, `trap`/`latent-trap`/`trap-undocumented`, `test-boundary`/`test-at-boundary-manual`): the ship skill's §5 lists the classes, one slug a kind, and `harness.py ratchet` counts the trailers since the last upkeep by them. Done when the list is in the skill and the command counts by it. Docs: the ship skill, the upkeep skill (§3).
