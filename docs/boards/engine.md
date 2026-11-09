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


- **A fuzz game ends with three Methuselahs left**: the weekly deep sweep of 2026-11-09 on 1ffb9d3 (`data/sweeps/2026-11-09-1ffb9d3db5c0.txt`, `FUZZ_SEEDS=600`) failed `twda_round_26a` at seed 45, "game over with 3 Methuselahs at turn 4" (`fuzz.rs`'s end invariant); already so at v2.24.2, the version served (`SEED_FROM=45 FUZZ_SEEDS=46 cargo test --profile ci -q --test fuzz random_games_with_twda_round_26a`). Done when `FUZZ_SEEDS=600` passes `twda_round_26a` and the commit names the cause. Docs: none, or `docs/engine.md` if a game-end rule moves.
- **A fuzz seat holds two copies of a vampire**: the same sweep failed `twda_round_26b` at seed 64, "seat holds two copies of Valeriya Zinovieva" (`fuzz.rs`'s uniqueness invariant); already so at v2.24.2 (`SEED_FROM=64 FUZZ_SEEDS=65 cargo test --profile ci -q --test fuzz random_games_with_twda_round_26b`). Done when `FUZZ_SEEDS=600` passes `twda_round_26b` and the commit names the cause. Docs: none, or `docs/engine.md` if the cause is a mechanism.
