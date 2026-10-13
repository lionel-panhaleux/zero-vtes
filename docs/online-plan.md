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
   everyone and every game names its commit. `crates/wasm` stays for
   offline play and as a second client of the same contract.
2. **v1 = one human + bots** per game; the API is per seat from day one
   (multi-human additive, gated on a redaction audit).
3. **Domain**: `engine.krcg.org`.
4. **Game logs in Postgres** on frankfurt (nightly restic backups).
5. **Retention**: games kept 2 years for training — said in the client's
   notice; a purge job in the server.
6. **Contact SC's author** once the contract doc and the reworked SC PR
   exist.
7. **Repo public** shortly before that contact.

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
  temperature and nudges are written by hand in `data/replays/README.md`
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
           ↗   ↖
engine ← host    ↖
   ↑       ↑      ↖
policy ────┴── server (axum, tokio, sqlx)   wasm (wasm-bindgen)
   ↑                    both: api + host + policy
   py (training: engine + policy; reads records through api)
```

- **`engine`** — the rules, no dependencies. `Agent: Send` (a bound, no
  behaviour change: goldens untouched).
- **`policy`** — inference (`Arc<Net>`, one copy of the weights for every
  game) and the **bot profiles**: style, weights, temperature, nudges, by
  name (`p9`, `scripted:wall`…) — out of the wasm shell.
- **`api`** (new) — the contract as types and nothing else: `Command`,
  `Message`, `GameSpec`, `Settings`, `TableView`, `OptionView`, typed
  `Event`, `GameRecord`, errors, `PROTOCOL_VERSION`. No engine dependency:
  JSON Schema and TypeScript types are generated from it in CI; any client
  (SC, a CLI, another platform) binds to the schema without Rust.
- **`host`** (what `protocol` becomes) — engine → api: describing,
  narration, views, copy identity, decks by KRCG id, the ergonomics layer
  (undo, opportunities, allocations, vote orders), the game record. One
  entrypoint, `Game::handle(seat, Command) -> Vec<(seat, Message)>`. Bots
  are injected through an agent factory: `host` does not depend on
  `policy`. The option texts (`describe`, `purpose`, `narrate`: the
  owner's feedback of 2.18–2.33) move intact, guarded by
  `tests/prompts.rs` and the session sweep.
- **`server`** and **`wasm`** — two thin shells over the same
  `handle(json) -> json`: the proof the contract is transport-neutral.
- **`py`** — training only: `engine` + `policy`; human games read as
  `GameRecord`s. `Session::engine()` / `request()` (escape hatches into
  the engine) and `start.rs` move to `py` or go.

### Message model: push

Client → game (`Command`): `join` (also the resync after a reconnect),
`answer {decision, option}`, `allocate {decision, amounts}`,
`vote_all {decision, favor}`, `undo`, `settings {opportunities, …}`.

Game → seat (`Message`): `decision` (options, prompt, view), `update`
(events, what bots and the seat's own answers did; no view), `over`
(result, view), `error`.

`handle` runs the bots up to the next human decision and returns
everything that happened, addressed per seat; the client paces the
display (SC already does). No client-driven `advance()` loop. `join`
returns the pending decision and the view. Decision numbers stay global
(stale answers refused).

### Contract rules

- **Data first, text beside it**: every field a client branches on is a
  stable id (`window`, `phase`, action and minion kinds, strikes,
  markers, event kinds); English `text` accompanies it for display.
- **Per seat**: views, events, settings, undo, redaction.
- **Versioned**: `protocol` (semver) and `engine` (commit) in `join`'s
  reply and in every record.
- **Views only where read**: on `decision`, `over`, `join`; not on
  bot steps (today ~15–30 KB on each).

### The game record

Designed cleanly, not as a superset of today's replay: versions, spec,
settings, bot profiles (with the weights' hash), one entry per decision
(seat, timestamp, command, option index, a **structured choice key**
replacing `Logged.choice`'s `Debug` string), undos kept as entries,
result, end reason, panic. A one-shot converter brings the 30 corpus
games over at a pinned commit; `train/assess.py` and
`examples/replay.rs` read records; the old reader goes.

### Copy identity in the engine (owner, 2026-10-13: yes)

The engine numbers each card copy at the deal; `track.rs`'s zone diffing
goes. Copy ids are still re-issued per viewer when a card leaves that
viewer's sight (shuffled into a library, into a hidden hand), or the id
would leak its path: the host keeps that mapping, by rule rather than by
heuristics. Buys exact conservation and one id space across seats
(multi-human). Costs a re-bless of goldens and perft (an intended engine
change); self-play throughput is measured before and after.

## A. Architecture and contract (zero-vtes)

| # | Item |
|---|---|
| A1 | `api` crate + `docs/protocol.md` (lifecycle, every command and message, errors, settings, a transcript) — **the owner reviews this before anything else lands** |
| A2 | `host` crate: `protocol` re-cut over `api`; `Game::handle`; per-seat state (view, tracker, events, settings, undo) |
| A3 | Stable ids beside text; typed events |
| A4 | Bot profiles in `policy`; agent factory in `host`; `Agent: Send`, `Arc<Net>` |
| A5 | Game record + structured choice key; corpus converter; `assess.py` and `examples/replay.rs` on records |
| A6 | Panics caught per command (`catch_unwind`), the game ended and recorded with the panic |
| A7 | `wasm` shell rewritten over `handle`; `smoke.mjs` answers by option index |
| A8 | Schema + TS generation in CI; a schema round-trip over the session sweep |
| A9 | Engine copy ids (above): zones hold copies, the state hash and the observation encoding follow; goldens and perft re-blessed; perft / fuzz timing and the training step rate measured before and after |
| A10 | Redaction audit (multi-human gate, not v1): cards named from hand, `limbo`, `ActionView.card`, events per seat, the "waiting on X" tell |

## B. Server, logging, deploy (`crates/server`, `engine.krcg.org`)

| # | Item |
|---|---|
| B1 | `POST /games` (spec → id + seat token), `GET /games/{id}/ws?token=` (commands in, messages out), `GET /games/{id}/record` (the player's own game), `/health`, `/version` |
| B2 | One task per game; engine steps on a bounded blocking pool; idle TTL (abandoned games ended and recorded); reconnect = `join` |
| B3 | Postgres `zvtes`: a `games` row (versions, spec, settings, bot profiles, client, start/end, result, end reason, panic) + the record as JSONB, written as the game goes (a crash keeps it); panics also to the journal (Loki `tag="zvtes"`); 2-year purge |
| B4 | Corpus pull: an authenticated export → `scripts/fetch_games.py`, filtered by engine commit |
| B5 | Limits: games per IP, concurrent cap, message size, CORS for SC's origins, WS pings; no accounts (an optional anonymous player id from the client) |
| B6 | Weights deployed beside the binary, hash recorded per game; profiles selectable by name |
| B7 | Deploy per server-setup: GH Actions release asset (linux x86_64) on tag; `deploy/` (pyinfra): user, `/opt/zvtes`, `/etc/zvtes`, systemd on 127.0.0.1:8040 with `MemoryMax`/`CPUQuota` (the forum shares the box), `postgres_db`, `nginx_site` (long WS read timeout); `deploy_targets.py`; OPERATIONS.md row; DNS `engine.krcg.org` → 57.129.110.107 |
| B8 | Load test: concurrent bot-only games, native inference timed; the cap set from it |

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

`main` stays playable (today's wasm + fork) until the server and the
re-cut SC branch replace them in one step: the corpus keeps growing.
`crates/py` builds and trains throughout.

1. A1 — `api` types + `docs/protocol.md`; owner review.
2. A9 first (the engine change), then A2–A7, on a branch.
3. B1–B6 locally → B7 (deploy) → B8.
4. A8; C's small PRs (any time); C's re-cut branch on the live server;
   owner games online.
5. E (video) → repo public → D (contact, with the doc, the PR, the
   video).
6. A10 → multi-human, later.
