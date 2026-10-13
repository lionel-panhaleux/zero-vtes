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
| F10 | P6 A2b (`tests/events.rs`) | An undo after `vote_all` (or an allocation answered whole) took back only the order's last cast, asking again a decision the seat was never shown | fixed: the steps a command takes by itself are undone with it (`Taken::Order`) |
| F11 | P6 B7 (first deploy) | The server refused a WebSocket without an `Origin` (non-browser clients: `tests/client.mjs`, bots, tools), though only browsers send one and the seat token is what admits a seat | fixed: an origin sent must be a platform's; none is let through |
| F12 | P6 B7 (redeploy) | `deploy.py` copied the new binary over the running one: "Text file busy", the redeploy failed | fixed: staged as `zvtes-server.new`, renamed into place |
| F13 | P6 deep sweep (`tests/events.rs`, 2,000 games, seed 1045) | A combat cut short by its Methuselah's ousting was never told ended | fixed 76ac3ae: the turn's end notes it |
| F14 | P6 A7 deep sweep | A card ability whose card shows only as a marker (Disarm) was named "a card"; a card granting an action that is a minion too (Call the Great Beast) named as the minion | fixed 2037742 |

## Playtest, 2026-10-13 (Succubus Club fork, main's wasm, one game against 3 bots)

The fork plays the legacy session (main); text = the renderer's now. Chrome
ran without WebGL (Phaser's Canvas renderer): card glows, which mark choices
and targets, could not be judged. P: the fork's UI (the SC re-cut, task C);
E: engine or text (here).

| # | Where | What | Status |
|---|---|---|---|
| P1 | Board | Uncontrolled vampires share the torpor row and cover its "Uncontrolled" label (only "Torpor" readable until one leaves) | todo (C) |
| P2 | Influence | Transfers left are shown nowhere | todo (C; `view`: the transfers left?) |
| P3 | Options | "All options" collapses after every pick: two clicks a transfer | todo (C) |
| P4 | Discard phase | SC's own "Next Turn" button shows and does nothing; the engine's Pass ends the phase | todo (C) |
| P5 | Influence | A card's "Influence" label looks clickable and answers "Not possible now"; dragging the vampire to the ready region is the gesture | todo (C) |
| P6 | Influence | "+" stays enabled past a vampire's capacity (the excess drains on influencing out, a transfer wasted, no hint) | todo (C) |
| P7 | Terms | Villein's amount titled "Choose X" (no X on the card); the prompt ("Villein: how much blood") not shown | todo (C: show `prompt`) |
| P8 | Referendum / action panels | The panel runs over the seat's board: vote totals, "Played during the action" cut off | todo (C) |
| P9 | Referendum | The panel names the card, not what passing does (hover needed) | todo (C) |
| P10 | Minion phase | Lenelle's "Other Actions" took Become Anarch at once (2 blood), unconfirmed, the panel calling it "Action In Play Provided By Lenelle" | todo (C: a lone option behind a menu still shown, named by its action) |
| P11 | Action | The card being played sits in the ready region (with a 0 counter) before it resolves | todo (C) |
| E1 | Log | Card names in KRCG sort form in options and prompts: "Gate of Acheron, The", "Smiling Jack, The Anarch", "Rack, The" | todo (renderer: printed names) |
| E2 | Log | Bots' passes told each time: "Pass (no block)", "Pass (Block attempts)", "Pass (Blocks declined)", "Pass (no vote now)", "Pass (As the card is played)" | todo (renderer/host: a pass told only where it does something) |
| E3 | Log | A failed block told for Marcos Belegrad, not for Modius (Shadow Cast's +1 stealth) | todo |
| E4 | Log | A play's cost and its card entering play told after the effect and the replacement draw (Zillah's Valley's −5 pool after the draw; Villein's blood moved before "Villein comes into play") | todo (events order) |
| E5 | Log | Blood drained on influencing out past capacity, the Edge gained on a bleed: not told | todo (events) |
| E6 | Log | "Bot2 — Use Modius": what the ability does not said | todo (renderer: the ability's effects) |
| E7 | Bots | Bot3 used Wider View until its crypt was empty (8 vampires removed) | todo (bots: a policy guard or training signal) |
