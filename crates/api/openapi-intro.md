Play *Vampire: The Eternal Struggle* against bots on **zero-vtes**, a
rules-enforcing engine for the V5 card pool. The engine decides what is
legal, runs the bots and tells what happened; a client shows the table
and turns its user's moves into answers. No account: a game is created
by one request and played over one WebSocket.

Every game is recorded, anonymously (human names are replaced), and
kept two years, to train and test the engine.

## How a game goes

1. `POST /games` with a game spec: 2 to 6 seats in seating order, one
   human and the rest bots (`GET /version` lists the bot profiles), each
   with a deck (`GET /catalog` lists the decks the engine plays). The
   answer is `{game, seat, token}`.
2. Open the WebSocket `wss://engine.krcg.org/games/{game}/ws` and send
   `join` with the token as the first frame. Every frame is one JSON
   object.
3. The host plays the bots and sends what happened as `events`; when it
   is the human's turn to choose it sends a `decision`: every legal
   move, each an option with an `index`. The client answers with
   `answer`, naming the decision and the option. Nothing else changes
   the game.
4. The game ends with an `over` message: when one Methuselah is left, at
   the time limit, when the human is ousted or leaves, or after the
   idle timeout.

```
client                                    host
  POST /games {spec}              ───▶
                     ◀───  {"game": "…", "seat": 0, "token": "…"}
  WS /games/{game}/ws             ───▶
  {"type": "join", "token": "…"}  ───▶
                     ◀───  {"seq": 1, "type": "joined", …, "view": {…}}
                     ◀───  {"seq": 2, "type": "events", "events": [the deal]}
                     ◀───  {"seq": 3, "type": "events", …}   a bot's decision
                     ◀───  {"seq": 4, "type": "decision", "decision": 7, "options": […], …}
  {"type": "answer", "decision": 7, "option": 3} ───▶
                     ◀───  {"seq": 5, "type": "events", "events": [{"kind": "decided", …}, …]}
  …
                     ◀───  {"seq": 90, "type": "over", "vp_halves": […], "winner": 2, …}
```

A decision, as sent:

```json
{"seq": 5, "type": "decision", "decision": 12, "seat": 0,
 "window": "minion_phase", "undo": false,
 "options": [
   {"index": 0, "kind": "pass"},
   {"index": 3, "kind": "play", "card": {"id": 100845}, "actor": 41,
    "target": {"kind": "seat", "seat": 1},
    "mode": {"index": 0, "line": 0,
             "disciplines": [{"discipline": "dom", "superior": false}]}},
   {"index": 4, "kind": "resolve_rider",
    "effects": [{"kind": "pool", "seat": 0, "n": -2}, {"kind": "keep", "copy": 9}]}],
 "view": {"viewer": 0, "decision": 12, "turn": 2, "active": 0,
          "phase": "minion", "seats": […]}}
```

(Pass; play Govern the Unaligned at basic Dominate with minion 41,
bleeding seat 1; or burn 2 pool to keep card 9.)

## A minimal client

Python, with the `websockets` package: it plays the first option of every
decision.

```python
import asyncio, json, urllib.request
import websockets

SERVER = "engine.krcg.org"

def get(path):
    return json.load(urllib.request.urlopen(f"https://{SERVER}{path}"))

async def main():
    decks = dict(get("/catalog"))
    deck = decks["Toreador (Fifth Edition)"]
    seats = [{"name": "Me", "deck": deck, "player": {"type": "human"}}] + [
        {"name": f"Bot {i}", "deck": deck, "player": {"type": "bot", "profile": "wall"}}
        for i in range(1, 4)
    ]
    body = json.dumps({"seats": seats, "client": "my-client 0.1"}).encode()
    request = urllib.request.Request(
        f"https://{SERVER}/games", body, {"Content-Type": "application/json"}
    )
    game = json.load(urllib.request.urlopen(request))
    async with websockets.connect(f"wss://{SERVER}/games/{game['game']}/ws") as ws:
        await ws.send(json.dumps({"type": "join", "token": game["token"]}))
        answer = None
        async for frame in ws:
            message = json.loads(frame)
            if message["type"] == "decision":
                option = message["options"][0]["index"]
                answer = {"type": "answer", "decision": message["decision"], "option": option}
                await ws.send(json.dumps(answer))
            elif message["type"] == "error" and message["error"] == "rate_limited":
                await asyncio.sleep(1)  # not taken: send it again
                await ws.send(json.dumps(answer))
            elif message["type"] == "over":
                print(message["reason"], message["vp_halves"])
                break

asyncio.run(main())
```

## Identities

| What | Type | Notes |
|---|---|---|
| Seat | integer from 0 | seating order; a seat's prey is the next seat |
| Card | integer | the [KRCG](https://static.krcg.org/data/vtes.json) card id; a deck lists ids as strings (JSON object keys) |
| Copy | integer | one physical card as the viewer follows it; a minion is its card's copy |
| Decision | integer | decisions taken so far, all seats; an answer names it |
| Option | integer | the option's `index` in the pending decision |

**Copies.** Cards are numbered where the viewer can follow them: on the
table, in face-up piles (ash heaps, removed, limbo) and in the viewer's
own hand. A card in a hidden place (another seat's hand, any library or
crypt) has no number: a card back, `{}`. A card coming into sight takes
a fresh number, so a number tells nothing its face does not. Two seats
may number the same card differently. A visible card also carries
`nth`, its label among the copies of that card from the same deck
("Embrace #2").

## Commands

Sent by the client, tagged by `type`, with an optional client `id` that
an `error` echoes as `reply_to` (the `In` model).

| `type` | Fields | Effect |
|---|---|---|
| `join` | `token`, `after`? | take the seat (the first frame); `after`: resend the messages after that `seq`, on a reconnection |
| `answer` | `decision`, `option` | take an option |
| `allocate` | `decision`, `amounts` | answer a points allocation whole (one amount per `allocation.recipients`) |
| `vote_all` | `decision`, `favor` | cast all the votes `votes` counts one way |
| `undo` | `decision` | take back the seat's previous decision, when the pending decision says `undo: true` |
| `settings` | `opportunities`?, `views`?, `text`? | change some settings (below); the pending decision is sent again |
| `leave` | | end the game, recorded as abandoned |

A command accepted has no reply of its own: its effect comes as events.
A refused command changes nothing and is answered by an `error`.

## Messages

Sent by the host (the `Out` model), each with `seq` (numbered per seat
from 1), `reply_to`? and a `type`:

| `type` | What |
|---|---|
| `joined` | the protocol version, the engine commit, the game, the seat, the seats' names and bot profiles, the settings, the idle timeout, the view |
| `events` | what happened since the last message: one decision taken and what it did up to the next, or the engine's run before a decision (the turn, draws) |
| `decision` | the pending decision: `decision`, `seat`, `window`, `options`, `undo`, the view, and what the choice is about (`source`, `purpose`, `about`, `allocation`, `votes`) |
| `over` | the victory points (`vp_halves`, per seat), the `winner` if any, the `reason`, the view |
| `error` | why a command was refused (`stale`, `not_your_turn`, `option`, `no_undo`, `allocation`, `no_votes`, `over`, `not_joined`, `malformed`, `rate_limited`) |

## Options

An option is a whole move. Its **key** says what it is: `kind` (`pass`,
`play`, `use_ability`, `take_action`, `declare_block`, `cast_votes`,
`transfer`…), and as it applies, the `card`, the `actor` (the minion
playing or acting), the `minion` it is otherwise about, the `target` (a
seat or a card), the `action`, the `mode` (with its Disciplines),
an `amount`… Two options of a decision never share a key. Its
**effects** say what taking it does (`pool`, `blood`, `bleed`,
`strike`, `lock`, `move`…: the `Effect` model). A client may match a
user's gesture to options by their keys (dragging a card onto a minion:
the `play` options with that `card` and `actor` or `target`) and ask
only about what still differs.

`window` (`minion_phase`, `action_block`, `combat_strike`…) is where in
the turn the decision is asked; a pass's meaning follows from it (in
`action_block`, declining to block).

## The view

A `TableView`: the table as the seat sees it, a snapshot to compare
with the previous one. The turn, the active seat, the phase; per seat
the pool, the victory points, the Edge, the zones (crypt, library,
hand, ash heap, removed, limbo), the uncontrolled vampires, the ready
and torpid minions with their blood, cards and markers, the cards in
play; the action, combat or referendum under way.

## Events

Each `events` message lists in order what happened (`decided`,
`turn`, `draw`, `pool`, `blood`, `enters_play`, `moves`,
`leaves_play`, `torpor`, `ousted`, `edge`, `action_ends`,
`referendum_ends`, `combat_begins`, `strikes`, `combat_ends`…: the
`Event` model). Events are public: a card they name is shown. Events
tell what happened, the view says what is: a client animating from
events resyncs on the next view.

## Settings

Per human seat, in the spec (`player.settings`) and changed by the
`settings` command:

- `opportunities` (default `true`): `false` hides the options that
  interrupt play out of their moment; a decision left with a pass alone
  then passes by itself.
- `views` (default `decisions`): `steps` adds the view to every
  `events` message, for a client animating each move.
- `text` (default `false`): English renderings beside the data (`text`
  on options and events, a decision's `prompt`), for a client that does
  not compose its own.

## Limits

One human per game; 2 to 6 seats; a time limit in turns and an idle
timeout (`GET /version` gives them); a number of games per hour and
commands per second per connection. Over a limit, `POST /games` answers
429, and a command gets an `error` `rate_limited`: it was not taken,
send it again a moment later.

## Types

The wire types are the models below, also served as
[JSON Schema](/schema.json) (2020-12) and as
[TypeScript](/protocol.ts). Closed sets (`Window`, `OptionKind`,
`EffectKind`…) may grow: a value a later version adds should be read as
unknown, and a field a client does not know ignored.

## Versioning

`protocol` (in `/version` and `joined`) is semver: a new field or id is
a minor version, a removed or renamed field or a changed meaning a
major one. `engine` is the commit: rules fixes ship without a protocol
change.
