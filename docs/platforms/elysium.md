# Elysium: what plugging the engine in would take

[Elysium](https://elysium.cards) is a live browser VTES table with its
own bots. Source:
[dtagent2000/VTES-Elysium](https://github.com/dtagent2000/VTES-Elysium),
public, no license file ("Johan's to open as he wishes",
`client/ELYSIUM-PROTOCOL.md` §13). One maintainer, Johan, writing it
mostly with Claude; very active (client 2.6.167, server srv-2.6.49, bot
0.6.211 the week of 2026-10-19). Read 2026-10-23, the code not run.

## Its stance

Johan ruled out his bot calling zero-vtes (bot `docs/RULINGS.md` D-83,
2026-10-19/20): zero-vtes "only as a development-time check on card
readings" (its cleanup spec adds "only after asking its maintainer");
Elysium's niche is bots that also work offline in the client. An engine
table in Elysium was never asked of him; it would be his call.

## How it runs games

- A single 19.8k-line vanilla-JS HTML client (no build, `localStorage`),
  which also plays offline (hot-seat, tutorial); a dependency-free Node
  server (hand-rolled WebSocket, JSON files), self-hostable through a
  Cloudflare tunnel.
- A sandbox: "the server never simulates game rules … the table is the
  referee" (protocol §5). Each client pushes its own board; the server
  holds only the hidden piles, turn order, dice, VP and the oust bounty;
  a shared tally panel (stealth vs intercept, votes, combat) any seat
  adjusts. Moves are free gestures (drags, right-click action menus,
  counters) writing log lines.
- Live rooms of up to 6, no accounts (a room's name and password),
  spectators, a host with controls (kick, promote, pass a stuck seat), a
  match clock and reaction windows, saves to resume a match later.
- Cards travel by name; decks pasted from vdb.im exports, card data from
  KRCG. Its bots read the frozen log lines and quick phrases ("No
  block") and run in the host's browser tab.

## What plugging the engine in would take

An "Engine table" room type, as the SC fork's engine game: the view
drawn on its structured board, each gesture matched to an option (a card
dragged onto a vampire, the action menu, the quick phrases as passes and
blocks, the tally panel as the block, vote and combat windows), the
events' `text` in its log. The obstacles: Johan's D-83; no license (a
fork served by us needs his word); offline play, its selling point (the
engine runs on the server only); tables of several humans with its own
bots, which read log lines and cannot sit at an engine table; a client
of free gestures and self-managed values in one file; names, not ids.

## Asks of the engine's API

Deferred by the owner already (roadmap "Deferred by the owner"): several
humans at one table and spectators. Beyond those:

- Seats answered by a client the platform runs (its own bots, any
  third-party agent), not only the server's bot profiles.
- A host role over the game: pass a stuck seat, end the game, give a
  dropped player's seat to a bot.
- Decks by card names, a refusal naming the cards the engine does not
  play.
- A time limit and reaction pacing set per game.
- A game paused and resumed days later (its saves), which the idle
  timeout and `server_restart` forbid today.

On the engine board: "A decklist checked without a game" (by ids or
card names, the unplayable cards named; the pool listed). The other asks
are held until a platform commits to plug in, then brought through
`intake` (engine line, 2026-10-23).
