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

- **The decks the bot needs taught, served**: `GET /catalog/taught` is built (protocol 2.22.0); engine.krcg.org serves it once Z-train's per-deck file for the served weights (`runs/<run>/<export>.decks.json`, the training board's "Bots on any deck") is deployed beside them. Done when engine.krcg.org's `/catalog/taught` carries the served weights' `weights` and its decks resolve in `/catalog`: the client smoke test (`crates/server/tests/client.mjs`) prints its `taught: N decks measured …` line (it passes on a 404 too). Docs: none beyond `docs/online-plan.md` "Deploy" if the deploy changes.
