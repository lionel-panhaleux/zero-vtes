# Online play — plan (P6, 2026-10-13)

The engine and its bots served online, so other players' games grow the
testing vector without anyone exporting a replay. Five workstreams: the
architecture and the contract (A); a multi-game server that logs every
game, on frankfurt (B); a clean Succubus Club PR (C); a demo video (E);
the proposal to SC's author (D). Supersedes `docs/human-play.md` slice 5.

The owner's frame (2026-10-13): the cleanest result, not the shortest
path — clear boundaries, minimal dependencies, refactors welcome; what
was built so far need not be preserved.

## Settled (owner, 2026-10-13)

1. **The server is authoritative** for online games: the game runs on
   our server, bots included; the browser is a client. Logging is a side
   effect, hidden information stays hidden, one engine version serves
   everyone and every game names its commit. (`crates/wasm` was to stay
   for offline play; owner, 2026-10-14: removed, the server is the only
   host — Succubus Club's in-browser engine went with it.)
2. **v1 = one human + bots** per game; the API is per seat from day one
   (multi-human additive, gated on a redaction audit).
3. **Domain**: `engine.krcg.org`.
4. **Game logs in Postgres** on frankfurt (nightly restic backups).
5. **Retention**: games kept 2 years for training — said in the client's
   notice; a purge job in the server.
6. **Contact SC's author** once the contract doc and the reworked SC PR
   exist.
7. **Repo public** shortly before that contact.
8. **Copies per viewer** (no information leak), cards in hidden places
   unnumbered; face-up cards labelled `nth` ("Embrace #2"). Engine copy
   ids underneath.
9. **Ids are snake_case strings.**
10. **Data first**: the platform composes its display; English text is an
    optional rendering (`text` setting), produced by a renderer that
    reads only `api` types.
11. **The only human ousted ends the game** (the bots do not play on).
12. The Opus review of 2026-10-13 (contract, plan): every recommendation
    adopted.

## Where things stand (audit, 2026-10-13)

- **Protocol** (`crates/protocol`): `Session` (advance / answer /
  allocate / vote_all / undo / set_opportunities / replay), `Out`
  (`decision` / `decided` / `over`, each with `events` and a full
  `TableView`), `OptionView`, errors tagged by `error`. Decision number =
  `engine.log.len()`, global: fit for a server.
- **Single human**: `viewer` is the first human seat (session.rs:293);
  every message carries its view, one `events` buffer, opportunities set
  for all humans at once. A second human gets seat 0's view.
- **Not `Send`**: `Agent` has no `Send` bound, `NetAgent` holds `Rc<Net>`.
  `Engine` is `Clone`, globals are read-only `OnceLock`s, `Net` is plain
  `Vec<f32>` (1.68 MB): an `Arc<Net>` shared by every game works.
- **Panics**: ~500 `panic!`/`unreachable!`/`expect` sites; `crates/py`
  already wraps steps in `catch_unwind(AssertUnwindSafe(..))`.
- **Prose as data**: `window`, `phase`, minion/action `kind`, `strike`,
  `markers` and every event are English strings; a client matches on
  prose. `Logged.choice` is a Rust `Debug` string.
- **Replay** = `{spec, log, choices}`; engine commit, bot model,
  temperature and nudges are written by hand in `data/records/README.md`
  ("the fork's build of that day"); undos are truncated away.
- **Wire weight**: a full `TableView` on every message, bot steps
  included: ~15–30 KB each (estimate), 10–20 MB a game.
- **wasm shell**: camelCase and plain method names mixed; bot temperature
  and nudges hard-coded there (`crates/wasm/src/lib.rs:20`). `smoke.mjs`
  answers by list position, not `OptionView.index` (wrong once
  opportunities filter the list).
- **SC fork** (`../succubus-club`, `zero-vtes-engine`, 24 commits on
  upstream `3099b59`, +5.1k lines): a self-contained `src/client/engine/`
  plus hooks in 27 SC files (77 `GameType.Engine`/`isEngine` lines).
  Ungated: the layout constants move (123c52d), the CentralBox scroll, the
  texture-cache reset. `zvtes-wasm` is a `file:` link to this checkout
  (breaks `npm ci` and SC's deploy). Precon fixes edit a file copied from
  VDB. `EngineHost` is already all-Promise but pull-driven, no resync.
- **frankfurt**: OVH VPS, 6 vCPU / 12 GB, near idle; hosts archon beta
  and the forum (Discourse). pyinfra-deployed systemd units behind nginx +
  certbot, sops secrets, Alloy → Loki/Prometheus, Postgres 17 with nightly
  restic backups to Scaleway. Rust ships as a GitHub Actions release
  asset, never built on the host. Free port e.g. 8040; UFW 22/80/443 only.


## Architecture

### Crates and dependencies

```
            api  (serde only; schemars behind a feature)
           ↗  ↑  ↖
engine ← host   text (api + card names: the English renderer)
   ↑      ↑ ↖  ↗
policy ───┴─ server (axum, tokio, sqlx: api + host + text + policy)
   ↑
   py (training: engine + policy + host's replayer)
```

- **`engine`** — the rules, no dependencies. `Agent: Send`. Card copy
  ids (A9), behaviour-neutral.
- **`policy`** — inference (`Arc<Net>`, one copy of the weights for every
  game) and the **bot profiles** (style, weights, temperature, nudges, by
  name), implementing `host`'s `Bots` trait; the scripted styles too.
- **`api`** — the contract as types: `In`/`Command`, `Out`/`Message`,
  `GameSpec`, `Settings`, `TableView`, `OptionView` (`ChoiceKey` +
  `effects`), typed `Event`, `GameRecord`, errors, `PROTOCOL_VERSION`.
  No engine. `schema.json` and `protocol.ts` are generated from it
  (`schema` example, `scripts/gen_ts.py`), checked in, `--check`ed.
- **`host`** (what `protocol` becomes) — engine → api: describing options
  as keys and effects, narration as typed events, views, per-viewer
  copies and `nth`, decks by KRCG id and the catalog, the ergonomics
  layer (undo, opportunities, allocations, vote orders), records and
  their `Replayer`. One entrypoint, `Game::handle(seat, In) -> Vec<(seat,
  Out)>`, emitting step by step; an injected clock and a step budget.
  Bots through the `Bots` trait: no `policy` dependency.
- **`text`** — the English renderer over `api` types and a card-name
  table: `render(&OptionView)`, events, prompts. The owner-tuned wording
  of `describe`/`purpose`/`narrate` re-expressed over data; their tests
  (`tests/prompts.rs`) become its oracle, and prove the data complete.
- **`server`** — a thin shell over `handle`.
- **`py`** — training: `engine` + `policy`; human games through `host`'s
  `Replayer` (engine, request, chosen index, key per decision).

### Message model: push

Client → host (`In`: an optional `id` + a `Command`): `join {token?,
after?}`, `answer`, `allocate`, `vote_all`, `undo {decision}`,
`settings` (a patch), `leave`.

Host → seat (`Out`: `seq`, `reply_to?` + a `Message`): `joined`,
`events` (one step each: a decision and what followed, its first event
`decided` / `auto_passed` / `undone`), `decision`, `over`, `error`.

`handle` runs the bots up to the next human decision, emitting each
step as it goes; the client paces the display. Reconnect = `join
{after: seq}` from a per-seat buffer, else a fresh `joined`. A settings
change resends the pending decision.

### Contract rules

- **Data first**: stable ids, keys, effects; `text` only on request.
- **Per seat**: views, events, settings, undo, copies, redaction.
- **Versioned**: `protocol` (semver) and `engine` (commit) in `joined`
  and every record; unknown ids read as `unknown`.
- **Views where read**: on `decision`, `over`, `joined`; on every step
  with `views: steps` (SC's projection animates each move; WebSocket
  compression on). Events as a complete delta stream (view N + events =
  view N+1): later, decided now as the direction.

### The game record

Versions, spec (human names replaced), bot profiles (weights' hash),
one entry per decision (seat, time, index, `ChoiceKey` with engine
copies, by whom, the options shown when filtered), undos, settings,
typed refusals, result and failure. Stored as entries appended (a
`game_entries` table), not a growing JSONB rewrite. A one-shot
converter brings the 30 corpus games over; `train/assess.py` and the
replay example read records through `host`'s `Replayer`; the old reader
goes. Release binaries and weights archived by tag and hash, so a record
replays on its commit.

### Copies

Engine (A9): each card copy numbered at the deal **in decklist order**
(independent of the seed and the shuffle), carried beside the card in
every zone (`{card, copy}`, no indirection); out of the state hash, the
observations and the options' order. Acceptance: perft identical,
observations bit-identical, goldens unchanged, the 30 corpus games
replaying by index unchanged.

Host: per viewer, numbered places only (the table, face-up piles, the
viewer's own hand); hidden places unnumbered; a card coming into sight
takes a fresh number never used before for that viewer; `nth` the
lowest free label among visible copies of that card from that deck.
The map is part of the host's state (snapshotted with the engine for
undo), events translated as they happen.

## A. Architecture and contract (zero-vtes)

| # | Item |
|---|---|
| A1 | `api` + `schema.json` + `protocol.ts` (`scripts/gen_ts.py`) + `docs/protocol.md` — done, reviewed (Opus, 2026-10-13) |
| A1b | Thin end-to-end slice: `host::Game::handle` (join, answer, events per step, decision, over; data only, today's copies through an adapter), the `wasm` shell, `smoke.mjs` answering by index, a throwaway CLI client — done (2026-10-13): `crates/host` (`Game::handle`, `handle_json`: seq, rejoin, idempotent undo, settings resend, ousted human ends, record, panics caught, step budget, injected clock; today's session through `legacy.rs`), `tests/game.rs`; the wasm shell over `handle` and `smoke.mjs` (network bots: 14.9 ms a decision) |
| A9 | Engine copy ids, behaviour-neutral (above) — in parallel with A1b — done (2026-10-13): `crates/engine/src/copies.rs` (`CopyId`, `Pile`, `Untracked`, `census`), dealt by `CopyCounter` in seating then decklist order; the fixture-only API behind the `fixtures` feature; fuzz checks conservation at every decision; goldens, perft and 1,000 random games' hashes unchanged; `Agent: Send` |
| A2 | `host` whole: per-seat state, per-viewer copies and `nth`, undo by snapshots, settings, allocations, vote orders, `seq` buffer, step budget, injected clock. First test: two options of one decision never share a `ChoiceKey` |
| A3 | Data first: option keys and `effects`, decision `source`, typed events, `Role` and `MarkerKind` completed — what `purpose.rs` and `narrate.rs` say in words |
| A4 | `Bots` trait in `host`; profiles in `policy`; `Agent: Send`, `Arc<Net>` — `Arc<Net>` done; `Bots` trait in `host` (implemented by the shells for now) |
| A5 | Records + `Replayer`; corpus converter; `assess.py` and the replay example on records; `py` on `host` — done (2026-10-14): `host/src/replay.rs` (`Replay`: a record's deal and decisions, undos dropped, read by key, by index where a key is unknown; `Replayer`: told for a seat seeing all), `start.rs` (training starts from records); the 30 corpus games converted to `data/records` (read whole by the legacy reader, keyed by the engine's copies; `tests/records.rs` reads them all whole with their results); `zvtes_py.replay_decisions`, `train/assess.py`, `train/ppo.py` (`--bc`, `--starts`), `policy`'s waste example and `examples/replay.rs` on records |
| A6 | Panics caught per command (`catch_unwind`; a panic hook for the backtrace, and for wasm where unwinding aborts) and a step budget (an engine loop): the game ends as `engine_error`, recorded |
| A7 | `text` crate: the renderer over `api`; `tests/prompts.rs` as its oracle |
| A8 | Contract tests over the session sweep: every message validates against `schema.json`; fixtures type-check against `protocol.ts`; no leak (no KRCG id or copy a seat could not see); keys unique; replay by index = by key; byte-identical transcripts (and protocol.md's transcript generated from one); undo restores the earlier view |
| A10 | Redaction audit (multi-human gate, not v1): cards named from hand, `limbo`, `ActionView.card`, the "waiting on X" tell |

A2–A7 in order (2026-10-13): `legacy.rs` stays the oracle, replaced piece
by piece, each piece differential-tested against today's output over the
session sweep; then the adapter and `crates/protocol` go.

| # | Step | First test |
|---|---|---|
| A2a | `host::view`: `TableView` from `legal_view(viewer)` and engine copies through a per-viewer `Numbering` (fresh number on entering a numbered place from a hidden one, never reused; `nth` the lowest free among visible copies of that card from that deck); events' copies translated as each happens | over the sweep: no copy on a card in a hidden place, no wire copy reused per viewer, a card in sight keeps its number — done (2026-10-13): `host/src/view.rs` + `numbering.rs` (two passes: the copies in sight, then the numbering); `game.rs` test: the engine's view agrees with the session's at every decision (200 games, 8,658 decisions) and keeps the copy rules (13,604 copies) |
| A2b | Undo by snapshot (`Engine: Clone` + the `Numbering` at each human decision), not replay from the deal | the view after an undo equals the view when that decision was first asked — done (2026-10-13): the session keeps the engine before each human decision undo may take back (cleared when a bot decides, 64 at most, else replayed), the host the numbering at each decision asked; `tests/events.rs`: every decision asked again is the view first sent (200 games, 1,152 undos); an undo takes back a command whole (F10) |
| A3a | `ChoiceKey` from `Choice`, viewer-aware (wire key: viewer copies; record key: engine copies); `By::Auto` / `Order` from the engine | distinct options ⇒ distinct omniscient keys (copies alike in hand excepted) — done (2026-10-13): `host/src/describe.rs` (`key`, by a viewer's numbering or `EngineNames`); `tests/keys.rs`: no shared key over 306,528 decisions, 58 kinds; options shown keyed from the engine; records keyed by engine copies (a human's answer before it applies, each bot's as it decides through a recording wrapper — the host now makes every bot); `a_record_replays_by_key_as_by_index` (24 games) |
| A3b | Typed events from `narrate`'s marks, per recipient | `session.rs`'s `Ends` invariants on the typed stream — done (2026-10-13): `narrate` emits `Fact`s in engine ids (marks from the engine and `census`, no view) beside the legacy lines; the session queues them with each decision taken marked; the host drains them after every view and names them by the viewer's numbering (`host/src/events.rs`; `Numbering::name`: in sight, just gone, or fresh); every change told, `text` where the legacy log tells it; the run before a decision or the end now told (F6). `tests/events.rs`: ends told, combat order, last pool/blood/torpor = the next view, every copy numbered (200 games, 115,948 events, 1,255 undos) |
| A7 | `text` crate over `api` + card names, built now as the completeness proof | `render(option)` = legacy text, `render(event)` = legacy event over the sweep; each miss is a missing field (A3c) — events done (2026-10-13): `crates/text` (`zvtes-text`, over `api` alone; card names by a `Cards` trait, seats and minions from the views and events a client was sent: `Names`); `tests/events.rs`: every legacy event line rendered alike from the data (200 games, 21,014 lines); its one miss was a field (`minion`: the minion's kind, a token is nameless). Options under way (2026-10-13): `zvtes_text::option(option, window, purpose, seat, offered, names)`; `tests/events.rs` holds it to the legacy text where it renders (anything it renders must match; what it does not is counted by kind — the work left): 600 games, 165,853 of 178,054 options (93%), none otherwise; with the curated effects (owner: B, 2026-10-13; `host/src/effects.rs`, `text/src/effect.rs`) 173,865 (97.6%), 35 reworded (abilities and plays: every legacy part told, more where legacy left out what all of a source's abilities share), none otherwise. Data added for it: `Decision.purpose` (64 ids, one per engine interrupt), `Decision.source` (terms windows), a play's second-vampire `effects` (Toreador Grand Ball), clan names by the client's card database (`Cards::value`). Then the block window's passes (from the view's action: its blocker, intercept and stealth) and the amounts chosen (`set_x`'s `effects`: what the amount does, by the terms asking it): 175,137 (98.4%). Then the effect purposes (the purpose the verb, the options' `effects` its parameters; `Role::Anarch`; the zone of a vampire searched for): 176,543 (99.15%). Then referendum terms (roles and allocations as `effects`, who is allocated to next in the view's `terms.pending`), votes (`effects`: the votes cast, prisci ballots), riders, cost terms: 177,809 (99.86%). Then contests, Toreador Grand Ball's second vampire (`Effect::clan`), a card's round strike, employing from hand: every option of the 600 games (178,054) rendered alike; at 2,000 games (601,242) too after a split mode's `apart` (from the card's definition, offered or not) and the rarer purposes. The oracle now fails on an option not rendered. Left: the purposes legacy rewrites that no sweep reaches (`text/src/option.rs` `asked`, declined); prompts, markers' and strikes' text (an oracle each first). The host sends the renderer's text (2026-10-13): options and events told by `zvtes-text` from the messages the seat was sent (`Game::tell` at `emit`: the client's `Names` fed in order), the legacy line kept where the renderer declines; `tests/events.rs` `the_host_sends_the_renderers_text` plays each seed twice (legacy lines, renderer's) and finds them alike (200 games: 58,128 options, 39,747 events). Prompts too (`Decision.about`: the particulars a prompt names; `text/src/prompt.rs`): 2,000 games, 2,189 of 2,189 legacy prompts rendered alike, the host sending them. The deep sweep also found: a combat cut short by its Methuselah's ousting never told ended (engine: the turn's end notes it), a card ability whose card shows only as a marker (Disarm) named by its host (key `minion`), a card granting an action that is a minion too (Call the Great Beast). Markers' and strikes' text still legacy; wording now open to improvement (owner, 2026-10-13: clearer or shorter as fits) |
| A3c | `effects`, `source`, `Role`, `MarkerKind` completed until A7 matches | as A7 |
| A2c | The rest of `Session` into `host`: opportunities filter (and `shown` in records), `allocate`, `vote_all`, catalog and deck checks (refuse counts over 255, not clamp); `py/human.rs` on `host`'s `Replayer` (A5) + the corpus converter; `crates/protocol` deleted once `tests/session.rs` and `prompts.rs` are ported or covered | the ported session sweep — the session in the host (2026-10-13): `host/src/session.rs` (the engine stepped: bots, standing vote orders, opportunities, allocations by probe, undo by snapshots), `narrate.rs` (facts, no words; a failed block attempt told when the action ends with it, the Edge changing hands), `deck.rs` (a count past 255 refused: F4); `game.rs` on it, the legacy adapter gone; text only from `zvtes-text` (every purpose rendered from `Decision.about`, every option and decision told and naming its cards over 2,000 games); 60 transcripts identical to the legacy-driven host's but for the additions. `crates/protocol` deleted (2026-10-14): its session tests covered by `host`'s (events, copy rules, keys, records, the transcript differential), its prompts by every option and decision told |
| A8 | Contract tests | as listed above — done (2026-10-14): `crates/host/tests/contract.rs`: every message and record of 200 games valid against `schema.json` (`jsonschema`), a game deterministic byte for byte, another seat's hidden choices naming no card (the views' copy rules: `game.rs`), `scripts/check_ts.sh` (a game typed against `protocol.ts` with `tsc --strict`); keys unique (`keys.rs`), replay by key = by index (`game.rs`, `records.rs`), undo's view (`events.rs`) |

Engine finding (A3a sweep, for the engine's owner): Helena's ability is
offered twice, identically, in the block window (`tests/keys.rs`, seed 201
of `HOST_SEEDS=600`): one choice listed twice, not two choices.

Server polish noted on the way: `Games::len` counts ended games until
their idle timeout (cap undercounts); `GET /record` should fall back to
Postgres once the game's thread is gone.


## B. Server, logging, deploy (`crates/server`, `engine.krcg.org`)

| # | Item |
|---|---|
| B0 | Deploy stub early (with A1b): `/health`, `/version` on `engine.krcg.org` — DNS, TLS, WebSocket through nginx (read timeout, pings), pyinfra, the release asset |
| B1 | `GET /version`, `GET /catalog`, `POST /games` (one human seat; no bot-only games), `GET /games/{id}/ws` (first frame `join {token}`; `Origin` checked), `GET /games/{id}/record` (`Authorization`) — done locally (`crates/server`, `tests/client.mjs` plays a game over HTTP + WebSocket) |
| B2 | One task per game; engine steps on a bounded blocking pool; idle timeout (ended, recorded); `seq` buffer for rejoin — done: a thread per game, a live-games cap, idle timeout, `seq` buffer |
| B3 | Postgres `zvtes`: `games` (versions, spec, bot profiles, client, start/end, result, failure) + `game_entries` appended per step; panics to the journal (Loki `tag="zvtes"`); 2-year purge — done: Postgres (`games`, `game_entries`, appended per command; checked on a scratch cluster) or JSON lines; daily purge at 730 days |
| B4 | Corpus pull: an authenticated export → `scripts/fetch_games.py`, by engine commit — done (2026-10-14): `GET /admin/games` (since, engine) and `/admin/games/{id}/record` behind the admin token (the server holds its SHA-256: `ZVTES_ADMIN_SHA256` from `deploy/group_data/all.py`; the token in `~/.config/zvtes/admin_token`); `scripts/fetch_games.py --since … --engine … --finished` writes `data/server-records/<id>.json`, read by training like `data/records` (`--bc`, `--starts`, `train.assess`); checked on a scratch Postgres and the file store. On frankfurt: `sudo -u zvtes psql zvtes` (peer auth), `select id, engine, started, ended, result->'reason' from games order by started desc limit 20` |
| B5 | Limits: games per IP, concurrent cap, frame size, spec caps (time limit, seats), commands rate (undo especially), step budget per command; WebSocket compression — done: games per address an hour, live games, time limit, commands a second, `Origin` and CORS by `ZVTES_ORIGINS` |
| B6 | Weights beside the binary, hash recorded per game; release binaries and weights archived — done: the weights' SHA-256 in `/version` and each bot's record; archived by hash on the host |
| B7 | Deploy per server-setup: GH Actions release asset (linux x86_64) on tag; `deploy/` (pyinfra): user, `/opt/zvtes`, `/etc/zvtes`, systemd on 127.0.0.1:8040 with `MemoryMax`/`CPUQuota` (the forum shares the box), `postgres_db`, `nginx_site` (its 60 s read timeout: the server pings every 30 s); `deploy_targets.py`; OPERATIONS.md row; DNS → 57.129.110.107 — done (2026-10-13): live on https://engine.krcg.org from `deploy/build.sh` (Docker) + pyinfra; Postgres, TLS, the network bots; `tests/client.mjs` plays a full game there and reads the stored record back; F11, F12 met on the way. Releases from GitHub once the repository is public |
| B8 | Load test: concurrent bot-only games, native inference timed; the cap set from it — done (2026-10-14): `crates/server/tests/load.mjs` (n games at once, network bots, a human answering at random as fast as messages come; the server allows no bot-only game). Release build on the owner's laptop: 1 game 1.7 s; 8 at once 6.7 s; 32 at once 15.1 s; 64 at once 17.8 s (26,862 messages, 2,525 human decisions), peak 235 MB (≈3.5 MB a game), none refused. A real player takes minutes a game: the default cap (64 live games) stays, well under frankfurt's `MemoryMax=3G` / `CPUQuota=400%` |

## C. Succubus Club

Fresh branch off upstream `master`, re-cut (not rebased) from the fork.

- Engine code in `src/client/engine/` and `central/Engine*.vue`; SC files
  get hooks only, through `useUIFeatures` flags (TrainBot's pattern), one
  `useEngineMode()`, one `<EngineCentral/>`.
- Client types generated from the schema; `RemoteEngineHost` (wss, the
  engine URL in config, resync on reconnect) as the PR's only host.
- `AGENTS.md` (no `border-radius` / `box-shadow`, no `!`), prettier /
  eslint / vue-tsc, upstream's commit style (short imperative subjects).
- Engine mode's notice: games recorded for 2 years to improve the engine.

Separate small PRs first: card textures (retry a failed load; reset the
cache per Phaser game), a stored precon follows the shipped list; precon
list fixes go to VDB's `precon_decks.json`. The layout constants and the
CentralBox scroll: gated to engine games, or proposed apart with
free-table screenshots.

Engine PR (~7 commits): mode + UI flags + generic bits; the client
(types, `RemoteEngineHost`); projection, driver, setup, menu entry,
notice; gestures; table interaction; central box; controls (Pass,
Opportunities).

## E. Demo video

A storyboard and a fast-track seed (drafted here) showing: deck pick, the
glows and staging of a play, an action with a block and a combat, a
referendum with "all votes one way", the bots' narration, the
Opportunities switch, undo, the game record. Recorded by the owner on
`engine.krcg.org` with the re-cut SC branch, a few minutes, captioned.

## Order

The fork plays on the server since 2026-10-14 (the wasm module and the
fork's in-browser host removed, owner); `p6-online` merged into `main`
the same day (rebased; its earlier tip tagged
`p6-online-pre-rebase-2026-10-14`, the commits the server's first records
name). Work continues on `main`; `crates/py` builds and trains
throughout.

1. A1 (done) → A1b (the thin slice) with A9 and B0 in parallel.
2. A2–A7 (the host whole, data first, records, bots, the renderer) →
   A8 (contract tests).
3. B1–B6 locally → B7 (deploy) → B8.
4. C's small PRs (any time); C's re-cut branch on the live server
   (types from `protocol.ts`; the PR may opt into `text` for labels while
   matching on data); owner games online.
5. E (video) → repo public → D (contact, with the doc, the PR, the
   video).
6. A10 → multi-human, later.
