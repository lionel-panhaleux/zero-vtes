# Findings to correct

Issues met on the way (engine bugs, duplicate choices, rough edges),
each fixed on the spot when small (kaizen), or listed here to address
next (owner, 2026-10-13). One line each: where it showed, what is wrong,
status. Fixed items stay a while with the commit that fixed them.

| # | Found | What | Status |
|---|---|---|---|
| F1 | P6 A3a key sweep (`crates/host/tests/keys.rs`, `HOST_SEEDS=600`, seed 201) | Helena's ability offered twice, identically, in the block window (`Choice::UseAbility { source: Vampire, ability: 0 }` listed twice): one choice listed twice | fixed: `Engine::request` offers a choice once (only Helena duplicated, in every block window she met); golden `twda_round_7b` re-blessed; `crates/host/tests/keys.rs` fails on a choice offered twice |
| F2 | P6 B (server) | `Games::len` counts ended games until their idle timeout: the live-games cap undercounts capacity | fixed: `Games::live` counts games not over |
| F3 | P6 B (server) | `GET /games/{id}/record` answers 404 once the game's thread has exited, though Postgres holds the record | fixed: read back from the store for the seat token's holder (its SHA-256 kept, never the token); `RECORD_AFTER` in `tests/client.mjs` |
| F4 | P6 A1b | Deck counts over 255 are clamped by the session's `u8` lists, not refused (`check_deck`, `Game::new`) | todo (A2c) |
| F5 | P6 A1b | `Entry::Decision.shown` never filled: the options the opportunities filter hid are not recorded | todo (A2c) |
| F6 | P6 A3b | The host dropped the events of the engine's run before a decision or the end (`Out::Decision` / `Over` events ignored): turns, draws, an action resolving through elided windows went untold | fixed: facts drained after every host view; the run's events sent before the decision or the end |
| F7 | P6 A3b | The legacy log leaves out a decision's own changes unless its option's text omits them (`untold_effects`): a data client misses pool and blood a play moved | fixed: every change is a typed event; `text` only on the lines the legacy log tells |
| F8 | P6 A3b | The legacy log tells a lasting effect whose card is in the ash heap as a card coming into play ("Disarm comes into play on X"); its event now says `effect` | todo (A7: the renderer tells it as an effect) |
| F9 | P6 A7 (advisor) | A `decided` event's `choice` is the legacy session's option read back from words (`legacy::Ctx::option`: no `effects`, `value`, `ability`), not `describe::key` as the record and the shown options are | todo (A2c: the host takes decisions itself) |
