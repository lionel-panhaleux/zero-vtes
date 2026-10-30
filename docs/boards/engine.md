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

- **The events miss three more view paths**: `HOST_REDUCE=1 HOST_SEEDS=700 cargo test --profile ci -p zvtes-host --test events the_events_tell` on 6a8668b (round 23's deals, 2026-10-30) fails "3 paths the events miss": `.ready[].blood` (seed 386 decision 212, 7 -> 6), `.ready` (seed 516 decision 210, The unnamed's copies), `.ready[].markers` (seed 695 decision 287, `out_of_play` -> none). Done when each path is told by an event and the seeds are kept in the sweep (`KEPT` in crates/host/tests/events.rs). Docs: `docs/protocol.md` if an event changes.
- **A hand size raised mid-action draws up at once**: Z-add (round 23's audit, 2026-10-30): the rulebook ("Drawing Cards") and advanced rules §6.8 draw up *immediately* on a hand-size change (§6.8.1: a fresh combat against Raptor draws up, then discards again; LSJ 20030530), but `Engine::settle_draw_up` draws only between plays (no action, combat or referendum), so Nar-Sheptha's and Rumble's in-combat hand size and Raptor's restore draw nothing until a card played is replaced (`hand_load` already counts the held and unreplaced cards). Done when a hand-size increase during an action or combat draws up at once, a test with Nar-Sheptha or Rumble asserts the hand's length at the first strike, and Raptor draws then discards per §6.8.1. Docs: docs/engine.md §6.2 (the draw-up paragraph).
- **The decks the bot needs taught, served**: the platforms board's "Teach the bot" tab (the owner's idea, 2026-10-26) lists the recent TWDA decks the served weights play worst; the server reads the training line's per-deck file for the weights it serves and offers it as data, weakest first, each deck with its decklist, event, date and the measure (games, VP share, the weights' version), beside `/catalog` (a query or its own route), so the fork renders it without inferring. After the training board's "Bots on any deck, and its weakest recent TWDA decks". Done when the route is in protocol §10 and the schema, a server test serves a fixture file, and engine.krcg.org's list carries the served weights' version. Docs: `docs/protocol.md`, `crates/api` (`schema.json`, `openapi.json`, `protocol.ts`), `CHANGELOG.md`.
