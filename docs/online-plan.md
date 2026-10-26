# Online play: the architecture

The engine and its bots served online at `https://engine.krcg.org`, so
the games other players play grow the testing vector without anyone
exporting a replay. The contract a client reads is
[`protocol.md`](protocol.md); this page is how it is built and served,
and what the owner settled. Open work is in the boards
([`boards/engine.md`](boards/engine.md),
[`boards/platforms.md`](boards/platforms.md)) and the
[roadmap](roadmap.md) (its ranking and "Deferred by the owner"): the
order of work is theirs.

## Settled

The owner's (2026-10-13 unless dated), holding until the owner changes
them:

1. **The server is authoritative**: the game runs on the server, bots
   included; a browser is a client. Logging is a side effect, hidden
   information stays hidden, one engine version serves everyone and
   every game names its commit. The server is the only host (owner,
   2026-10-14: the in-browser wasm engine removed, Succubus Club's with
   it).
2. **One human and bots** a game; the API per seat from the start, so
   several humans are additive, gated on a redaction audit (roadmap
   "Deferred by the owner").
3. **Domain**: `engine.krcg.org`.
4. **Game records in Postgres** on frankfurt, backed up nightly.
5. **Retention**: games kept 2 years for training, as the client's
   notice says; the server purges older ones.
6. **Succubus Club**: the fork is its own platform, bots only for now,
   agreed with SC's author (2026-10-21); the upstream PR deferred.
7. **Copies per viewer** (no information leak), cards in hidden places
   unnumbered; face-up cards labelled `nth` ("Embrace #2"); the engine's
   copy ids underneath.
8. **Ids are snake_case strings.**
9. **Data first**: the platform composes its display; English text is
   an optional rendering (the `text` setting), by a renderer that reads
   only `api` types.
10. **The only human ousted ends the game** (the bots do not play on).
11. **Clean boundaries over the shortest path**: minimal dependencies
    between crates, refactors welcome.

## Architecture

### Crates and dependencies

| Crate | Depends on | What |
|---|---|---|
| `engine` (lib `zvtes`) | nothing | the rules; `Agent: Send`; card copy ids, behaviour-neutral |
| `env` | engine | between the engine and an agent: observations, habits, default politics, the scripted bots |
| `api` | serde (schemars and serde_json behind `schema`) | the contract as types, no engine |
| `text` | api | the English renderer |
| `host` | engine, env, api, text | engine → api: a game behind the contract |
| `policy` | engine, env, api, host | the network's inference and its bot profiles |
| `server` | engine, api, host, policy (axum, tokio, sqlx) | a thin shell over the host |
| `py` | engine, env, policy, api, host (pyo3) | training; human games through the host's `Replayer` |

`server` and `py` are not default workspace members: plain cargo
commands stay off their dependencies.

- **`api`**: `In`/`Command`, `Out`/`Message`, `GameSpec`, `Settings`,
  `TableView`, `OptionView` (`ChoiceKey` + `effects`), typed `Event`,
  `GameRecord`, errors, `PROTOCOL_VERSION`. `schema.json`,
  `openapi.json` and `protocol.ts` are generated from it (its `schema`
  example, `scripts/gen_ts.py`), checked in and `--check`ed; the server
  shows them at `/docs`.
- **`host`**: options described as keys and effects
  (`describe.rs`), narration as typed events (`narrate.rs`,
  `events.rs`), views and the per-viewer copies (`view.rs`,
  `numbering.rs`), decks by KRCG id and the catalog (`deck.rs`), the
  human seat's ergonomics (undo, opportunities, allocations, vote
  orders: `session.rs`; human-play.md "The host's human ergonomics"),
  records and their reading (`replay.rs`), training starts from
  records (`start.rs`). One entrypoint, `Game::handle(seat, In) ->
  Vec<Out>`, emitting step by step; an injected `Clock`; a step budget
  (an engine loop ends the game `engine_error`, recorded); a panic
  caught per command ends it the same way. Bots through its `Bots`
  trait, so no `policy` dependency; the scripted styles are its
  `BUILT_IN`.
- **`text`**: options, events, strikes, markers and prompts told from
  the data a client was sent alone, card names by a `Cards` trait,
  seats and minions by `Names`; the host's sweeps
  (`crates/host/tests/events.rs`) prove the data complete by telling
  every option, decision and event of their games.
- **`policy`**: inference (`Arc<Net>`, one copy of the weights for every
  game) and the network bot profiles (`profiles`: weights, temperature,
  nudges, by name), implementing `host`'s `Bots`.

### Message model: push

Client → host: an `In` (an optional `id` + a `Command`); host → seat: an
`Out` (`seq`, `reply_to`? + a `Message`) — protocol §4 and §5.
`handle` runs the bots up to the next human decision, emitting each
step as it goes (an `events` message each); the client paces the
display. A reconnect is `join {after: seq}` from a per-seat buffer,
else a fresh `joined`. A settings change resends the pending decision.

### Contract rules

- **Data first**: stable ids, keys, effects; `text` only on request.
- **Per seat**: views, events, settings, undo, copies, redaction.
- **Versioned**: `protocol` (semver) and `engine` (commit) in `joined`
  and every record; unknown ids read as `unknown` (protocol §14).
- **Views where read**: on `decision`, `over`, `joined`; on every step
  with `views: steps` (a client animating each move). The events are a
  complete delta stream: view N with its events is view N+1 (protocol
  §9, held on every decision's view).

### Contract tests

`crates/host/tests/contract.rs`: every message and record of random
games valid against `schema.json`, a game deterministic byte for byte,
no message naming a card the seat was never shown; `scripts/check_ts.sh`
types a game against `protocol.ts` (`tsc --strict`); keys unique
(`keys.rs`), a record read by key as by index (`game.rs`, `records.rs`),
an undo's view the view first sent (`events.rs`).

### The game record

Versions, the spec (human names replaced), the bot profiles (weights'
hash, a network's nudges), one entry per decision (seat, time, index,
`ChoiceKey` with the engine's copies, by whom, the options shown when
filtered), undos, settings, typed refusals, feedback, the result (its
end turn) and any failure (protocol §11). Stored as entries appended (a
`game_entries` row each), not a JSONB document rewritten as it grows.
`train/assess.py`, `zvtes_py.replay_decisions` and the host's `replay`
example read records through `host`'s `Replay` / `Replayer`. The
weights served are archived by hash on the host and the binaries are
release assets by tag, so a record replays on its commit.

### Copies

Engine (`crates/engine/src/copies.rs`): each card copy numbered at the
deal in seating, then decklist order (independent of the seed and the
shuffle), carried beside the card in every zone (`{card, copy}`, no
indirection); out of the state hash, the observations and the options'
order.

Host: per viewer, numbered places only, a fresh number for a card coming
into sight, `nth` the lowest free label (protocol §3). The numbering is
part of the host's state, kept at each decision asked so an undo
restores it; events are translated as they happen.

### The bots

The network bot is `crates/policy`: the forward pass of
`train/model.py`'s `Policy` written by hand for one decision, so it
costs no dependency (tract, the alternative, added 3.3–4.9 MB gzipped
to the wasm build of the time) and the bot stays an engine `Agent`.
Weights come as a `.zvp` file (`python -m train.export`);
`train/tests/test_export.py` holds the Rust and Python forward passes
equal, so a change to `Policy.forward` that `crates/policy` lacks fails
it. A checkpoint older than the observation's features still loads but
plays off its training. `scripts/export_onnx.py` is the ONNX export
check. The server's network profile is `zvtes_policy::profiles::NETWORK`
(temperature 0.5, the nudges its weights trained with); the scripted
bots (`zvtes_env::bots`) are the host's `BUILT_IN` profiles.

## The server

`crates/server`: `GET /version`, `/catalog`, `/cards`, `POST
/decks/check`, `POST /games` (one human seat, no bot-only games), the
game's WebSocket (its first frame `join {token}`), its record, the
admin routes (protocol §10). Each live game runs on its own thread,
reached by a channel; the record streams to the store as the game goes.
Its configuration, from the environment, is `crates/server/src/main.rs`'s
header: live games (64), games an hour per address, the idle timeout,
the time limit, commands a second, the page origins (empty: any, as
`engine.krcg.org` runs), the admin token's SHA-256 (never the token).
The live-games cap stands on the load test (`tests/load.mjs`): 64 games
at once took about 3.5 MB each, far under the service's memory cap, and
a real player takes minutes a game. A restart resumes the games left
going from their records (protocol §2).

## Records and logging

Postgres `zvtes`: `games` (versions, spec, bot profiles, client, start,
end, result, failure) and `game_entries` appended per command; without
`ZVTES_DATABASE_URL`, JSON lines in a directory. On Postgres, a daily
purge drops the games past the retention (730 days). The server's errors
go to the journal (Loki `tag="zvtes"`). Records are pulled by the admin routes:
`scripts/fetch_games.py` writes `data/server-records/<id>.json`, read by
training like `data/records`; the daily harvest and its report are the
training skill's ("Harvest"). On frankfurt, `sudo -u zvtes psql zvtes`
(peer auth).

## Deploy

frankfurt: an OVH VPS, 6 vCPU / 12 GB, shared with archon beta and the
forum (Discourse); pyinfra-deployed systemd units behind nginx and
certbot, sops secrets, Alloy to Loki and Prometheus, Postgres 17 with
nightly restic backups to Scaleway, UFW 22/80/443 only. The engine's
Rust ships as a GitHub Actions release asset on a `v*` tag
(`.github/workflows/release.yml`), never built there: `deploy/deploy.py`
(port 8040, `MemoryMax` 3G and `CPUQuota` 400% in
`deploy/group_data/all.py`: the forum shares the box; the server pings
every 30 s under nginx's read timeout) fetches it (`deploy/release.py`),
`deploy/build.sh` the fallback:

```
cd deploy && ZVTES_REPO=lionel-panhaleux/zero-vtes RELEASE_TAG=v<x.y.z> \
  uv run --group deploy pyinfra inventory.py deploy.py -y
```

Nothing is built on this machine, so a training run need not pause.
The Succubus Club fork is built on the box, niced, and served as a
static site at `https://sc-bot.krcg.org` (`deploy/sc_bot.py`,
server-setup's `spa` nginx site; human-play.md "Running the fork").
Z-main deploys (CLAUDE.md "Sessions").

## Succubus Club

The fork is served as its own platform, bots only; its working log is
[`platforms/sc.md`](platforms/sc.md) (its integration's shape, branches,
the deferred upstream PR #285 and its re-cut). Holding for the fork and
for any upstream PR: engine code in `src/client/engine/` and `Engine*.vue`
components; client types from the schema (`protocol.ts`); the server
client the only engine host; the notice saying games are recorded for 2
years to improve the engine. For an upstream PR only: SC files given
hooks only (the fork tests `GameType.Engine` inline), SC's `AGENTS.md` and
checks. SC's layout is fixed, made to fit any
window size: the engine's panels fit inside it, never a change to its
constants (Thomas refused one). Precon list fixes go to VDB's
`precon_decks.json`.
