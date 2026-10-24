# JOL (V:TES Online): what plugging the engine in would take

[JOL](https://deckserver.net/jol/) (deckserver.net) is the long-running
play-by-post VTES table. Source:
[VTES-JOL/JOL](https://github.com/VTES-JOL/JOL), public, no license file
(all rights reserved). Maintained by Shannon Dowley (most commits), with
Christoph Prenner (stats, tournaments); active through 2026, slowing
(4 commits in September). Read 2026-10-23, the code not run.

## How it runs games

- A Java 21 WAR on Tomcat (JSP, Jersey REST under `/jol/api`, jQuery with
  no build step), behind Traefik; no database: each game is JSON files
  (`games/<uuid>/game.json`, `history.json`, one per deck), flushed every
  few minutes (`README.MD` "Storage Mechanics").
- A free tabletop: no rule is enforced. Every move is a typed command on
  the shared state (`play`, `move`, `burn`, `pool`, `lock`, `contest`,
  `vp`…, cards addressed `caine ready 2 2`; `DoCommand.java`), posted
  with a chat line and a ping (`POST /jol/api/game/{id}/submit`). Phases,
  edge, VP and timeout are tracked, legality is not; judges, a rollback
  to a turn, a player replaced.
- Asynchronous: a player sits in several games, is pinged by web push or
  Discord when it is their move, public lobbies expire after 5 days
  (`GameCleanUp.java`); open tabs refresh on a WebSocket "changed"
  signal. Games of 4–5 humans, tournaments included.
- Accounts (password, JWT cookie); decks imported as text (the JOL
  format KRCG and ARDB export) and checked by Standard, V5 and Duel
  validators; cards by VEKN (KRCG) id.

## What plugging the engine in would take

An engine game would be a new game type beside the free table: the
pending decision's options in place of the command bar, the view drawn
into JOL's regions (they map one for one), the events' `text` in its
history, its chat kept; JOL's server the engine's client, holding each
seat's token, its pings fired by a pending decision. The obstacles:

- JOL tables are several humans over days; the protocol hosts one human
  per game, live, ended by its idle timeout, and a deploy changing the
  rules ends a resumed game (`server_restart`, protocol §2).
- Play-by-post pays a round trip per window (blocks, reactions, combat
  steps): without standing answers an engine game is far slower than a
  free one, where a post compresses an action.
- The engine plays the V5 pool and the added legacy cards: most JOL decks
  would be refused.
- JOL's players referee themselves (judges, rollback, labels); an
  enforcing table changes that culture. The code changes land only as
  PRs Shannon accepts.

## Asks of the engine's API

Deferred by the owner already (roadmap "Deferred by the owner"): several
humans at one table and spectators; stops and auto-pass preferences (the
standing answers). Beyond those:

- Long-lived games: an idle time set per game in days, and a game kept
  playable across deploys (pinned to its engine, or carried over a rules
  change) instead of ended.
- A request/response transport beside the socket (fetch a seat's pending
  decision and view, post an answer) and a notification hook ("decision
  N pending for seat S of game G") for JOL's pings.
- Game creation by a trusted platform naming its players, read back from
  the records.
- Deck checking without a game: whether the engine plays a decklist (by
  id), and which cards it plays.
- Undo across seats (JOL's admin rollback to a turn), once several humans sit.

Answered in protocol 2.12.0 (protocol.md §10): `POST /decks/check`
checks a decklist, by ids or card names, without a game, naming the
cards the engine does not play; `GET /cards` lists the pool. The other
asks are held until a platform commits to plug in, then brought through
`intake` (engine line, 2026-10-23).
