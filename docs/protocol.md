# The zvtes protocol

<!-- The one protocol document. The server shows it as the intro of its
API reference (https://engine.krcg.org/docs: `crates/api/openapi.json`,
generated with this file as `info.description`, its title line dropped):
write for outside readers. The reference is `crates/api` (Rust);
`schema.json` (JSON Schema 2020-12), `openapi.json` (OpenAPI 3.1) and
`protocol.ts` (TypeScript) are generated from it, the version is
`PROTOCOL_VERSION`. Code cites sections as `docs/protocol.md §N`: keep
the numbers. -->

How a client plays a game of *Vampire: The Eternal Struggle* on the
zero-vtes engine: a rules-enforcing engine with bots, served at
`https://engine.krcg.org`. The client shows
the table and turns its user's moves into answers; the engine decides
what is legal, runs the bots and says what happened. No account: a game
is created by one request and played over one WebSocket. Every game is
recorded, anonymously, and kept two years to train and test the engine
(§11).

Version **1.7.0**. The types: the models of the
[API reference](https://engine.krcg.org/docs), also as
[JSON Schema](https://engine.krcg.org/schema.json) (2020-12) and
[TypeScript](https://engine.krcg.org/protocol.ts).

## 1. Model

- A **game** has 2 to 6 seats in seating order (0, 1, …), each a human
  or a bot. Version 1 hosts one human per game.
- The engine asks one **decision** at a time, of one seat: a list of
  **options**, every legal move, each a whole move (card, mode, actor,
  target). The seat answers with an option's `index`. Nothing else
  changes the game.
- A client sends **commands** for its seat and receives **messages**
  addressed to it. Between two of the human's decisions the host answers
  the bots' and sends what happened as **events**.
- Windows nobody can act in are skipped (the engine's shortcuts).

The client never needs the rules: what is legal is the option list, what
happened is the events, what is on the table is the view. All three are
structured data — ids, seats, cards, amounts — and the client composes
what it displays, its own way. An English rendering comes alongside only
for a client that asks (the `text` setting, §8).

## 2. Lifecycle

```
client                                  host
  POST /games  {spec}          ──────▶   validates, deals
               ◀──────  {game, seat, token}
  WS /games/{game}/ws          ──────▶
  {"type":"join","token":"…"}  ──────▶
               ◀──────  joined {game, seat, seats, settings, view}   seq 1
               ◀──────  events  (the deal)                            seq 2
               ◀──────  events  (a bot's decision and what followed)  …
               ◀──────  decision {decision: 7, options, view}
  {"type":"answer","decision":7,"option":3} ─▶
               ◀──────  events  (decided 7, its effects)
               ◀──────  events  (each bot decision, one message each)
               ◀──────  decision {decision: 15, …}
  …
               ◀──────  over {vp_halves, winner, reason, view}
```

- Every message carries `seq`, numbered per seat from 1. After a
  disconnection the client sends `join` with `after`: the last `seq` it
  received. The host resends what followed (or, if it no longer holds
  it, `joined` and the pending decision: a fresh start from the view).
- A seat plays on one connection: the newest `join` takes it. The
  connection it was on gets `seat_taken` (unnumbered) and is closed; a
  client receiving it must not join again from there (another tab or
  device has the seat). A refreshed page wins over a socket the host has
  not yet seen die.
- A command accepted has no reply of its own: its effect comes as
  events (an `answer` as an `events` message starting with `decided`);
  but `feedback`, answered `noted`. A refused command changes nothing
  and is answered by an `error`. Both carry the command's `id` as
  `reply_to`.
- Commands are limited per connection (a few dozen a second): over it,
  a command is refused with `rate_limited` (unnumbered: `seq` 0, no
  `reply_to`). It was not taken: send it again a moment later. A client
  answering at once, with no user in the loop, meets it.
- The game ends when one Methuselah is left, at the time limit, when the
  human is ousted (the bots do not play on), when the human leaves, or
  after the idle timeout (`joined.idle_timeout`). A server restart ends
  the games going (recorded `server_restart`, §11): joining one again
  finds no game (its socket's URL answers 404, §10).

### A minimal client

Python, with the `websockets` package: it plays the first option of
every decision against three scripted bots.

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

## 3. Identities

| What | Type | Notes |
|---|---|---|
| Seat | integer from 0 | seating order; prey is the next seat |
| Card | `KrcgId` | KRCG card id ([vtes.json](https://static.krcg.org/data/vtes.json)); a deck lists ids as strings |
| Copy | `CopyId` | one physical card as the viewer follows it; a minion is its card's copy |
| Decision | `DecisionNo` | decisions taken so far, all seats; answers name it |
| Option | `index` | the engine's index in the pending decision |

**Copies.** Cards are numbered where the viewer can follow them: on the
table (minions, cards in play and stored on them, the viewer's own
uncontrolled vampires; another seat's face down ones are backs), in face-up piles (ash heaps, removed, limbo) and in the
viewer's own hand (another seat's while a card reveals it). A card in a
hidden place — another seat's hand, any library or crypt — has no
number: a card back, `{}`; one an effect reveals from a library or a
crypt (a top card: Mesu Bedshet, Petra Resonance, Andrew Stuart; a
search's find the card says to reveal: Sakhar, Disciple of Lazarus) is
told to every seat by a `revealed` event, its `id` without a number. A card coming into
a numbered place from a hidden one takes a fresh number, never used
before for this viewer. So a number tells nothing the card's face does
not. Two seats may number the same card differently; records use the
engine's own numbers.

**Labels.** A card whose face the viewer sees carries `nth`: its label
among the copies of that card from the same deck ("Embrace #2"), the
lowest number no other visible copy holds, given as it comes into sight.

## 4. Commands (client → host)

A JSON object tagged by `type`, with an optional client `id` (echoed by
an `error`).

| `type` | Fields | Effect |
|---|---|---|
| `join` | `token`?, `after`? | take the seat (the first frame on a server connection: the seat's token); `after`: resend the messages after that `seq` |
| `answer` | `decision`, `option` | take an option |
| `allocate` | `decision`, `amounts` | an allocation of points answered whole (one amount per `allocation.recipients`) |
| `vote_all` | `decision`, `favor` | all the votes `votes` counts, one way; holds for the rest of the polling |
| `undo` | `decision` | take back the seat's previous decision, from the pending `decision` (when `undo` is true); sent twice, undoes once |
| `settings` | `opportunities`?, `views`?, `text`? | change some settings (§8); the pending decision is sent again |
| `leave` | — | end the game, recorded as abandoned |
| `feedback` | `text` (1 to 4,000 characters, trimmed) | a problem or a remark, kept in the record at the pending decision (else the last one), also once the game is over; answered `noted` |

## 5. Messages (host → client)

A JSON object with `seq`, `reply_to`? and a `type`:

| `type` | Fields |
|---|---|
| `joined` | `protocol`, `engine` (commit), `game`, `seat`, `seats` (`name`, `bot` profile), `settings`, `idle_timeout`, `view` |
| `events` | `events`: one step — a decision taken and what it did until the next — or the engine's run before a decision or the end (§9); `view` after a step when `views` is `steps` |
| `decision` | `decision`, `seat`, `window`, `source`?, `purpose`?, `about`?, `options`, `undo`, `allocation`?, `votes`?, `view`; `prompt` with `text` |
| `over` | `vp_halves` (per seat), `winner`? (the most VPs, at least 2, alone), `reason` (`{"type": …}`: `last_standing`, `time_limit`, `humans_ousted`, `abandoned` + `seat`, `engine_error`; in a record only, `server_restart`), `view` |
| `noted` | `decision`: the seat's `feedback` recorded, at that decision (`reply_to` its `id`) |
| `error` | `error`: `stale` (+ `expected`), `not_your_turn`, `option` (+ `index`), `no_undo`, `allocation` (+ `reason`: `none`, `length`, `total`, `cap`, `min_nonzero`), `no_votes`, `over`, `not_joined`, `malformed` (+ `reason`), `rate_limited`, `seat_taken` |

`window` is a stable id (`minion_phase`, `action_block`, `combat_strike`,
… 41: the schema's `Window`). `source` is the card whose terms or effect
the decision chooses, when it is one (a play's or an action's terms, a
referendum's): the options' `effects` say what each choice gets.
`purpose` is what an effect asks the choice for, when one does (81 ids,
the schema's `Purpose`: `pay_or_burn`, `store_search`…, one per kind
of effect choice the engine asks). The purpose says the verb
(`remove_uncontrolled`: the slot chosen is removed); where it has
parameters, each option's `effects` give them (`minion_of`: what the
chosen minion gets — `lock`, `unlock`, `corruption`, `blood`, `damage`,
`burn`, a card `move`d onto it; `ashur_picks`: `move` `to` the hand or
the library; `blood_or_burned`: the blood paid, or what burns on a
pass); a secret pick (`secret_number`, Game of Malkav) is told to the
other seats without its amount or effects, its `index` 0. `about` holds the particulars a prompt names, beyond the
purpose, the source and the options: `n` (how many: still to pay, at
most, to discard), `m` (a second amount), `cap`, `seat` (whose
minions, ash heap or hand), `minion` and `to` (the minions blood moves
between, a block attempt's), `card` (named or in play), `clan` (or a
circle), and `detail`, what sets it apart within its purpose (`minion_of`:
`lock`, `lock_vampires`, `corrupt`…; a terms window: `blood_range`,
`x_blood`, `discard`, `cost_reductions`, `burn_allocated`, `gains_pool`…).
A client tells its prompt from these alone, as the server's English
rendering (the `text` setting) does.

## 6. The view

`TableView`: the table as the seat sees it — a snapshot to diff against
the previous one, never the engine's state.

- `turn`, `active`, `phase` (`unlock`, `master`, `minion`, `influence`,
  `discard`, `over`), `decision` (the next one's number).
- `seats[]`: `name`, `pool`, `vp_halves`, `ousted`, `edge`, `prey`,
  `predator`; zones as cards `{copy?, id?, crypt?, nth?}` — `crypt`,
  `library` (top last), `hand`, `ash_heap`, `removed`, `limbo` (cards
  being played); `uncontrolled[]` (`card`, `counters`, `banished`,
  `cards`); `ready[]` and `torpor[]` minions; `in_play[]` (cards on no
  minion).
- Minion: `card`, `kind` (`vampire`, `ally`, `created`, `token`), `name`,
  `blood`, `capacity`, `locked`, `contested`, `title` (id), `owner`,
  `cards[]` (in play on it), `markers[]` (`kind`, `seat`, `n`, `card`),
  `traits[]` (`black_hand`, `infernal`, `red_list`, `blood_cursed`,
  `scarce`, `sterile`: as they stand, Black Hand gained included; a Red
  List minion marked this turn carries a `red_list_mark` marker, its
  `seat` the marking Methuselah).
- `action`: `seat`, `acting`, `kind` (`bleed`, `hunt`, …, `card`), `step`
  (`announced`, `open`, `attempt`, `declined`, `would_succeed`,
  `blocked`, `after_combat`, `resolved`), the `card` and its `mode` and
  `x`, `source`, `target`, `bleed_target`, `bleed`, `stealth`, `blocker`,
  `intercept`, `declined`.
- `combat`: `minions` (acting first), `round`, `range`, `sides[]`
  (`strike` once declared — the blocker chooses knowing the acting
  minion's — `maneuvers`, `presses`). A strike: `kind`, `n`, `ranged`,
  `aggravated`, `card`, `resolved`, `text`. In a combat, a damage
  strike's (`hand`, `melee`, `damage`) `n` and `aggravated` are what it
  inflicts now if it lands, before prevention: the minion's strength and
  its bonuses, the opposing minion's reductions, aggravated as it
  resolves; `resolved`, once the pair resolved, the strike's damage,
  before prevention.
- `referendum`: `caller`, `step` (`terms`, `polling`, `shortfall`),
  `calling`, `card`, `votes_for`, `votes_against`, `terms` (`seats`,
  `cards`, `clan`, `allocated`), every `vote` cast (`seat`, `source`,
  `copy`, `favor`, `n`, `canceled`).

## 7. Options

```json
{"index": 3, "kind": "play", "card": {"id": 100845}, "actor": 41,
 "mode": {"index": 0, "line": 0, "disciplines": [{"discipline": "dom", "superior": false}]},
 "target": {"kind": "seat", "seat": 2}}
{"index": 4, "kind": "resolve_rider", "card": {"id": 101011, "copy": 9},
 "effects": [{"kind": "pool", "seat": 0, "n": -2}, {"kind": "keep", "copy": 9}]}
```

(Play Govern the Unaligned at basic Dominate with minion 41, bleeding
seat 2; burn 2 pool to keep card 9.)

- **The key** — what the option is: `kind` (62 kinds: `pass`, `play`,
  `use_ability`, `take_action`, `declare_block`, `cast_votes`,
  `transfer`, `hand_strike`, `find_card`…), and, all optional: `card`
  (`id`; `copy` for one physical card; `from` a card it is stored on),
  `actor` (the minion playing or acting), `minion` (the minion it is
  otherwise about), `target` (`{"kind":"seat","seat":n}` or
  `{"kind":"card","copy":n}`), `action`, `mode` (`index`, `line`,
  `disciplines` with levels, `card_type` for a multi-type card, `apart`:
  the effects setting it apart from the other modes its printed line
  splits into, offered or not),
  `amount`, `favor`, `blood`, `zone` (where a search takes from), `value`
  (a clan, Discipline or circle id), `ability`. Two options of a
  decision never share a key (copies alike in hand are one option).
- **`effects`** — what taking it does: an ability's effects, a play's
  mode's, a cost, what a chosen Methuselah or minion gets. A curated
  vocabulary, not the engine's own effects: each
  `{kind, n?, x?, on?, seat?, copy?, cost?, when?, card?, to?, action?,
  favor?, strike?, range?, role?, clan?, detail?}`.
  - `kind` (37, open): `pool`, `blood`, `life`, `counters`,
    `transfers`, `damage`, `strike`, `strength`, `stealth`,
    `intercept`, `bleed`, `prevent`, `press`, `maneuver`, `range`,
    `end_combat`, `enter_combat`, `immune`, `burn`, `lock`, `unlock`,
    `move`, `steal`, `draw`, `discard`, `search`, `votes`,
    `referendum` (called), `fails`, `cancel`, `action` (declared:
    `action`), `hunt`, `hand_size`, `cost` (reduced), `corruption`,
    `keep`, `role` (§12).
  - `n` signed (+ gained, − lost or spent), per X with `x`; `cost`: paid
    to take the option; `when`: `successful`, `bleed_successful`.
  - `on`: whom, from the option — `target`, `host` (the minion the card
    or ability is on), `actor`, `acting`, `chosen`, `card` (the card
    itself), `uncontrolled`, `prey`; the seat's own when absent; `copy`
    the wire copy when known.
  - `detail`: what sets it apart within its kind, snake_case —
    `this_round`, `guns`, `not_guns`, `non_aggravated`, `strike` (of
    the opposing strike's damage only), `strike_non_aggravated`,
    `strike_all`, `x_plus` (X + `n`), `each_round`, `beyond_first`,
    `beyond_first_non_aggravated`, `to_pool`,
    `from_pool`, `per_counter`, `then_bottom`, `master_phase`….
  - A vote's: `votes` (`n` cast, `favor`; `detail: prisci_ballot` the
    ballots a priscus casts with them; `on: card` the political action
    card's own vote).
  - A referendum's terms: what the chosen Methuselah or minion gets
    (`on: chosen`: `pool` gained, burned or — `detail: allocated` —
    given, `steal`, `move` `to: removed` banished or the card put on it,
    `lock`, `role: expelled`), what an allocation does (`pool` burned,
    or gained with the chosen's `pool` lost); `view.referendum.terms.
    pending`: who is allocated to next (the last gets the rest; public,
    as the terms it follows from).
  - A card's chosen vampire (Toreador Grand Ball's second, chosen after
    the play): `role` with its `clan`; a play locking its target says
    so (`lock`, `on: target`).
  - A round's strike granted by a card (`round_strike` without a card):
    `strike`.
  - A cost's terms: `cost` (kind) for a card's or a vampire's reduction,
    `blood` (`cost`, `on: chosen`) for a vampire paying 1 of it.
  - A `set_x` option's (the key's `amount`): what the amount does, by
    the terms asking it — `blood` (`x`: the card calls it X; `copy`:
    the vampire it is taken from), `pool` (`on: target` given away;
    `detail: from_blood` the referendum's margin to the pool),
    `counters` (a cost reduced), `draw`, `discard` (and the rest in
    `pool`); burning pool for votes: `pool` (`cost`) and `votes`.
- `abilities` — a `use_ability`'s: how many its source has (`ability`
  in the key says which).
- A pass's plain meaning follows from the window: `master_phase` … ends
  the phase, `action_block` declines to block (the defender's, while
  its minion's attempt meets the stealth — `action.blocker`,
  `intercept` ≥ `stealth` in the view — blocks), `combat_press` presses
  no further, a polling passes for now.
- A client may match a user's gesture to options by their keys
  (dragging a card onto a minion: `play` options with that `card` and
  `actor` or `target`) and ask only what still differs.
- `allocation` (`points`, `recipients` with `to`, `cap` and what a point
  does, `min_nonzero`) and `votes` (castable each way) allow the whole
  answer at once (`allocate`, `vote_all`); the step-by-step options
  stay offered.

## 8. Settings

Per human seat, in the spec (`player.settings`; fields left out take
their default) and changed by `settings`:

- `opportunities` (default `true`): `false` hides the options that
  interrupt play out of their moment — a wake outside an action that
  concerns the seat or as a card is played, a block denial short of the
  action's stealth, an any-time ability where nothing else is on offer,
  an ability that would gain nothing now (legal all the same: burning the
  counters of a card that has none, votes for a vampire that cannot
  vote), a block attempt that cannot reach the action's stealth (every
  legal attempt is offered: with no intercept to gain, it fails). A
  decision left with a pass alone passes by itself (`auto_passed`).
- `views` (default `decisions`): `steps` adds the view to every `events`
  message — a client animating each move. Larger: compression
  (permessage-deflate) is on at the server.
- `text` (default `false`): English renderings beside the data — `text`
  on options, events and strikes, a decision's `prompt` — for a client
  that does not compose its own. Options and events are told from the
data sent alone, the names the client was shown, as a client could
tell them itself.

## 9. Events

`{"kind": …, …}` in order (`text` added with the `text` setting). Each
`events` message tells, in order, what happened since the last message:
a step (`decided`, `auto_passed` or `undone` first, then what the
decision did, up to the next decision), or the engine's run before a
decision or the game's end is sent (no decision event: the turn, draws,
an action resolving through windows no one could act in). Every change
is told, a decision's own included; `text` is set on the events the
English log tells (it leaves out what a decision's own text says).

Events are public: a card they name is shown (`id`), numbered as the
recipient knows it (§3), a card that came and went between two views
numbered fresh. `effect`: a lasting effect whose card is in the ash
heap (a marker on its minion, §6), no copy of its own.

| `kind` | Fields |
|---|---|
| `decided` | `decision`, `seat`, `window`, `choice` (an option, as the recipient may know it) |
| `auto_passed` | `decision`, `seat`, `window` |
| `undone` | `seat`, `to`: drop what was shown since decision `to` |
| `turn` | `turn`, `active` |
| `draw` | `seat`, `n`, `crypt` |
| `revealed` | `seat`, `card` (its `id`, `crypt`; no number): a card of a crypt or library an effect revealed to all (a top card, a search's find) |
| `pool` | `seat`, `delta`, `pool` |
| `blood` | `minion`, `delta`, `blood` |
| `enters_play` | `card`, `seat` (its controller), `on`?, `minion`? (its kind, for a minion), `effect`? |
| `moves` | `card`, `on`?, `effect`? |
| `leaves_play` | `card`, `to`? (the zone it went to), `minion`? (its kind), `contested`?, `effect`? |
| `torpor` | `minion`, `enters` |
| `ousted` | `seat` |
| `edge` | `seat`? (who holds the Edge now; absent: burned) |
| `block_fails` | `minion` (its block attempt ended, the action goes on) |
| `action_ends` | `seat`, `acting`, `action`, `outcome` (`succeeds`, `blocked`, `fails`, `fizzles`, `ousted`, `ends`), `blocker`? |
| `referendum_ends` | `outcome` (`passes`, `fails`, `canceled`), `votes_for`, `votes_against`, `action`? (the political action), `hunted`? (a blood hunt's diablerist) |
| `combat_begins` | `minions` |
| `strike_declared` | `minion`, `strike`? (none: no strike) |
| `strikes` | `round`, `pair`, `range`, `sides` (`minion`, `strike`?, `damage`: before prevention, its retainers' and environmental damage included; `strike_damage`: the strike's share) |
| `combat_ends` | — |

`action`: `{kind, card?, source?}` — the action's kind, its card, what
grants it (a card in play, another vampire).

Events tell what happened; the view says what is. A client animating
from events resyncs on the next view. The host's tests hold the two
together: an action or referendum a view shows and the next does not
is told ended in between; a combat is told begun, its strikes, then
ended; the last `pool`, `blood` and `torpor` told are the next view's.

## 10. Transports

**Server** (`https://engine.krcg.org`):

| | |
|---|---|
| `GET /docs` | the API reference: this document, the routes and the types (its test requests are reads only) |
| `GET /openapi.json`, `/schema.json`, `/protocol.ts` | the API reference's document, and the types as JSON Schema and TypeScript |
| `GET /version` | protocol, engine commit, bot profiles offered, limits (seats, time limit, idle timeout) |
| `GET /catalog` | the precon decklists the engine plays |
| `POST /games` | body: a `GameSpec` (one human seat); `{game, seat, token}`, or a `SpecError` (400). The deal's seed is the server's: a spec's `seed` is kept only with the admin token (a playtest's fixed deal) |
| `GET /games/{game}/ws` | WebSocket: one JSON command or message per text frame; the first command is `join` with the token. 404 to any request once the game has left the server (the idle timeout passed, after its end or with no command, or the server restarted), so a client whose socket will not reopen can `fetch` it to tell |
| `GET /games/{game}/record` | the game's record (§11), `Authorization: Bearer <token>`, once the game is over (409 before: the record replayed would show the hidden cards) |
| `GET /admin/games?since=&engine=&limit=` | the games recorded, oldest first: `id`, `engine`, `client`, `started`, `ended` (absent: going on the server now), `result`, `entries` (the operator's: `Authorization: Bearer <admin token>`) |
| `GET /admin/games/{game}/record` | any game's record, as far as it went (the operator's) |
| `GET /admin/feedback?since=&limit=` | the players' feedback, oldest first: `game`, `engine`, `at` (Unix ms), `decision`, `text` (the operator's) |
| `GET /admin/unsupported` | the cards refused decks named (`SpecError` `unsupported`), counted: `[{id, count}]`, KRCG ids, the most asked first (ids and counts only; the operator's) |

No account: the token is the seat, never in a URL. The server checks
the WebSocket's `Origin` against the platforms it serves.

## 11. Records

Every game served is recorded (`GameRecord`), and served to its seat
once the game is over (§10): the spec with its seed
(human names replaced), the bot profiles (weights' hash, temperature,
seed, a network's nudges by name), each decision answered — seat, time, option index, its key with
the engine's copy numbers, by whom (`human`, `bot`, `order`, `auto`),
the options shown when settings hid some — undos (decision numbers then
repeat from `to`: a reader drops what they take back), settings
changes, refused commands, the players' feedback (at its decision), the
result (with the turn the game ended on, `turns`; for a game a restart
ended, the turn its record reads to) and any engine failure. A game
going when its server restarts is ended at the restart,
`server_restart`: its victory points where its
record reads to on the restarted server's engine, `failure` saying why
when it does not read to its end (rules fixes change the engine, §14).
A record replays by index on its engine commit, and reads by key on a
later engine (a record from before every block attempt was offered reads
past the block windows its engine skipped). Kept two years, to train and
test the engine; anonymous.

## 12. Open sets

Snake_case ids from sets the protocol does not close:

- Clans: KRCG clan names (`banu_haqim`, `ministry`, `toreador`…);
  Disciplines: KRCG abbreviations (`dom`, `aus`…); circles likewise; card
  types (`action_modifier`, `reaction`, `combat`…).
- Titles: `primogen`, `prince`, `baron`, `justicar`, `inner_circle`,
  `archbishop`, `bishop`, `cardinal`, `priscus`, `regent`, `magaji`,
  `independent` or `laibon` (a unique title, its name in `of`), `votes`
  (a title by its votes only).

Closed sets (`Window`, `OptionKind`, `EffectKind`, `Role`, `MarkerKind`,
`Trait`, `Zone`…) are listed in the schema; a client should read a value a later
host adds as unknown.

## 13. In and out of scope

The host does the rules, the bots, what each seat may see, undo, the
settings above, records, and an optional English rendering. The client
does the display and its wording, gestures and their matching to
options, pacing and animation, decklists and accounts (if any). Out of
the protocol: the engine's internal state, card images, layout.

## 14. Versioning

`protocol` is semver: a new field or a new id in a set is a minor
version (readers ignore what they do not know), and so is a route's
new refusal by a status its clients already handle, or a request field
the server now decides itself (`POST /games`'s seed); a removed or
renamed field or a changed meaning, a major one. The api crate's own readers,
1.3.0 on, read past a record entry type they do not know, and read an
end reason, an error or a value of a §12 closed set (a decision's `by`
among them) they do not know as `unknown` (never sent); any other new
value still fails the read, and a reader built before 1.3.0 needs
records no newer than its engine. `engine` is the commit: rules
fixes ship without a protocol change.
