# Human play: the human seat and the fork

A human plays the engine and its bots through a fork of
[Succubus Club](https://github.com/thomasWajs/succubus-club) (SC),
served at `https://sc-bot.krcg.org` against `https://engine.krcg.org`.
What a client reads is [`protocol.md`](protocol.md); how the server is
built, [`online-plan.md`](online-plan.md); the fork's working log
(Thomas's style, branches, PRs, recipes, traps),
[`platforms/sc.md`](platforms/sc.md). This page: the decisions on human
play, what the host does for the human seat beyond what the protocol
states, the fork's interface, the checks and running the fork. Open
work is in the boards; stops and auto-pass preferences, the redaction
audit and spectators are the roadmap's "Deferred by the owner".

## Settled

- **The view is a dedicated DTO** (`TableView`, protocol §6), never the
  engine's state serialized: the engine's layout keeps changing, a
  platform must not depend on it.
- **The bots a human meets are the trained network** (the server's
  `network` profile, online-plan.md "The bots"), not the scripted bots.
- **License**: MIT + the Dark Pack notice (`LICENSE`, `README.md`).
- **The natural move is the input** (owner, 2026-09-30): every option
  is reachable by a real move on the table, a panel only the fallback;
  refusing table moves and asking to "pick an option" did not work as
  an interface in the first owner session.
- **JOL, Elysium**: not plugged in; what it would take in
  [`platforms/jol.md`](platforms/jol.md),
  [`platforms/elysium.md`](platforms/elysium.md).

## The host's human ergonomics

The commands a human seat has beyond `answer` (`allocate`, `vote_all`,
`undo`, protocol §4), the options it is offered (copies alike in hand
one option, a search's zone, protocol §7) and its settings (the
opportunities filter and the lone pass passing by itself, protocol §8)
are the protocol's. What the host (`crates/host/src/session.rs`) adds
to them:

- **A wake's moment**, opportunities off: an action concerns the seat
  when it is directed at the seat, or undirected and its prey's or
  predator's; never a bleed directed at another seat (owner: "not
  bleeds directed at my cross"). The owner's reason for each kind the
  filter hides is beside it in `Session::opportunity`; each kind has
  its test, shown on and hidden off
  (`*_is_hidden_with_the_opportunities_off`). A hidden option keeps its
  engine index; the record keeps the options shown.
- **All votes one way** casts the seat's vampires' votes and its
  political card's vote as each turn of the polling comes; the Edge, a
  card burned for a vote, plays and abilities stay single options. A
  vote bonus reaches only a vampire that has not cast yet: play cards
  before ordering the votes.
- **Undo** against bots (owner, 2026-10-18: "cancel anything anytime"):
  the session keeps the engine before each human decision, the bots
  with it so they answer again alike, 64 at most (an engine clone late
  in a game is 20–30 KB: 64 games of 64 about 120 MB, under the
  service's 3 GB); past them, no undo.

## The fork's interface

The table is the interface (owner, 2026-10-01): the engine knows what
can be done, so it replaces SC's guesses (the hand's glow, a
declaration's valid targets, the usage editor) instead of a list of
options. The fork's code says how (`src/client/engine/interaction.ts`,
`driver.ts`, `projection.ts`, `log.ts`); where each option is answered
is sc.md "The integration's shape".

- **Highlights carry the meaning**: usable (a card that can be played,
  a minion with something to do), candidate (click to choose it), staged
  (the card being played).
- **A click narrows the engine's options** (each a whole combination:
  card, mode, actor, target); what still differs is asked in table
  order (the card, the minion playing it, the Disciplines and level, the
  target), candidates glowing; one option left, it is taken. Escape or a
  right click cancels; drags say the same.
- **The central box** shows what is going on (the action, the
  referendum, the combat, on SC's own panels, read-only) and the choice
  under way; SC's controls that mean an engine option are that option,
  the others hidden. SC has no combat UI to imitate: a combat's choices
  are buttons.
- **SC's controls**: Cancel (Backspace) is the engine's undo, the fixed
  Pass (→ / Space) the decision's pass; one switch for the
  opportunities.
- **The log is the engine's**: composed from the events' data, SC's
  per-mutation lines off while the fork projects the engine's views.

## Checks

- `cargo test --profile ci -q` covers the host and the renderer (default
  members); `HOST_SEEDS=n` deepens `crates/host/tests/events.rs` (the
  table kept from the events against each view, undo, allocations and
  vote orders, every option and decision told), `HOST_SEED_FROM=n`
  starts there, `HOST_REDUCE=1` names each view field the events missed
  and `HOST_TRACE=1` prints the events and views; engine-side
  allocation shapes in `tests/politics.rs` / `tests/precon.rs`;
  `crates/host/tests/records.rs` reads the corpus whole.
- `cargo clippy -p zvtes-server --all-targets --profile ci -- -D warnings`
  and `cargo test -p zvtes-server --profile ci` (not a default member: a
  restart resuming a game left going and ending a stale one, the games
  left going ended when they cannot be, on the file store, and on Postgres
  with `ZVTES_TEST_DATABASE_URL` naming a scratch database);
  `scripts/check_ts.sh` (a game typed against `protocol.ts`); with a
  server running (Node 22 or later, for its `WebSocket`), `node
  crates/server/tests/client.mjs <url>` plays a game over HTTP and
  WebSocket (its seed the server's, not the one it asks), has a deck
  with a card outside the pool refused and one checked by its cards'
  names (`/cards`, `/decks/check`) (with `ZVTES_ADMIN_TOKEN`, its
  feedback read back from `/admin/feedback`, the refused card counted at
  `/admin/unsupported`, one spec dealt twice differently and the
  operator's seed kept), `node crates/server/tests/load.mjs <url> <n>` n
  at once, `node crates/server/tests/rejoin.mjs <url>` a seat joining
  again (after its last `seq`, after an older one, afresh, on a second
  socket taking the seat from the first). `uv run --script
  scripts/minimal_client.py <url>` plays protocol §2's Python client
  against it.

## Running the fork

Served at `https://sc-bot.krcg.org`: the fork's `zero-vtes-engine`, the
bots-only edition (the engine game, decks, preferences, About), built on
frankfurt by `deploy/sc_bot.py` (`SC_COMMIT=<sha>`; Z-main deploys).

Locally, the fork plays on `https://engine.krcg.org` (the server's network bots:
temperature 0.5 and the nudges their weights trained with,
`zvtes_policy::profiles::NETWORK`):

```
cd ../succubus-club && npm install                # once
npx vite --config vite/config.dev.mjs --port 5188 # then http://localhost:5188
```

`VITE_ENGINE_SERVER=http://localhost:8040` plays on a local server
instead (`ZVTES_POLICY=target/policy.zvp cargo run -p zvtes-server
--release`; its configuration: `crates/server/src/main.rs`).

Pick a deck (Preconstructed → Fifth Edition …: the server refuses a deck with
cards outside its pool, the fork names them), then "Play against Bots (rules engine)".
Dev flags: `VITE_FAST_TRACK_ENGINE_GAME=1` starts a game at once;
`VITE_ENGINE_AUTOPLAY=1` plays your seat at random with the consistency check.
The fork's own checks: `npx vue-tsc --noEmit`, `npx eslint .`,
`npx prettier --check .`. Look at the table in a visible tab: a hidden one
gets no animation frame (Phaser draws nothing) and throttled timers (the
soak mode uses no timer). Editing the fork while a game runs hot-reloads
the game screen: don't.

What the fork changes in SC: `src/client/engine/` (new: `host.ts` the
server client, `api.ts` copied from `crates/api/protocol.ts`, `log.ts` the
log), `store/engine.ts`, `Engine*.vue` components (`central/`,
`ui/ingame/EngineText.vue`, `rightColumn/EngineLog*.vue`; sc.md "The
integration's shape"), and hooks in `types/state.ts` (`GameType`),
`state/gameMutations.ts` (the gate), `state/setup.ts` (stop on reset),
the table's objects, `MainMenu.vue`: inline `GameType.Engine` tests (the
re-cut's are `useEngineMode()`). The bots-only edition removes the rest
of SC: the lobby, rooms, chat, availability, TrainBot, Puppeteer, saved
games, the SC server and Vercel functions, firebase, Ably and Sentry.
