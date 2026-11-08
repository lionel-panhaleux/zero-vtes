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


- **`GET /catalog` says what each deck is**: the owner (2026-11-08): "we should let the bot play recent (last 6 months) TWDA decks"; the fork's bot picker infers the precons from the names (`setup.ts`'s `ENGINE_BOT_DECK`), and its line "The bots play recent TWDA decks" (platforms board) waits on this. Each catalog deck's kind (precon or TWDA), its `title` (the name without ` (event, date)`), and for a TWDA deck its `twda` id, `event` and `date`, as `TaughtDeck` already carries. The entries are `[name, list]` pairs today (`dict(get("/catalog"))` in protocol §10's example), so either a minor that keeps them (a new route or query, `schema_compat.py --check` clean) or a major with its CHANGELOG entry; tell Z-succubus. Done when the catalog's decks carry them in `schema.json` and protocol §10. Docs: `docs/protocol.md`, `crates/api` (`schema.json`, `openapi.json`, `protocol.ts`), `CHANGELOG.md`.
