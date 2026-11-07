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

- **A fuzz game chooses an option out of range**: the weekly deep sweep of 2026-11-07 on e1c295b (`data/sweeps/2026-11-07-e1c295b24352.txt`, `FUZZ_SEEDS=600`) panicked in `twda_round_23a` at `Engine::choose`'s "option index out of range" (`game.rs`), the seed unnamed: a replay that diverged from its game (nondeterminism, which a record's replay or a server resume would meet) or a driver choosing from a stale request; fuzz passed at 600 on 8de7bf8 (2026-10-30). Done when `FUZZ_SEEDS=600` passes `twda_round_23a`, the commit names the seed and the cause, and a panic inside a fuzz game names its seed. Docs: none, or `docs/engine.md` if the cause is a determinism rule.
- **The events miss `.seats[].limbo`**: the same sweep's host pass (`HOST_SEEDS=400`), `the_events_tell_what_the_views_show`: seed 210, a card in limbo (`{"copy":176,"id":101056,"nth":4}`) leaves it with no event telling the move. Done when `HOST_SEEDS=400 cargo test --profile ci -p zvtes-host --test events` passes. Docs: `docs/protocol.md` if an event changes.
- **A referendum's found card without its zone**: the same sweep's host pass, `a_found_card_names_the_zone_it_is_in`: seed 178, `ReferendumTerms` offers `FindCard(16)` with no zone. Done when `HOST_SEEDS=400 cargo test --profile ci -p zvtes-host --test keys` passes. Docs: `docs/protocol.md` if the key changes.
- **The fuzz capacity invariant reads `present`**: since 0f5939d (rules-qa 108.1) a contested vampire, out of play, keeps its excess blood, but `fuzz.rs`'s "blood over capacity" assert exempts only `!m.in_play`; the same sweep failed it on contested vampires: `masters` seed 143, `events` 532, `twda_round_13a` 573, `twda_round_17a` 369 (Vampire 14 and 89). Done when the assert reads `present()` (or the capacity the rules give a contested vampire) and `FUZZ_SEEDS=600` passes those four fixtures. Docs: none.
