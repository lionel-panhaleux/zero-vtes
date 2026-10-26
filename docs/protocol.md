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

Version **2.16.2**. The types: the models of the
[API reference](https://engine.krcg.org/docs), also as
[JSON Schema](https://engine.krcg.org/schema.json) (2020-12) and
[TypeScript](https://engine.krcg.org/protocol.ts).

## 1. Model

- A **game** has 2 to 6 seats in seating order (0, 1, …), each a human
  or a bot. The server hosts one human per game. A game of 2 seats plays
  VEKN's two-player variant: a contested vampire stays in play and acts
  (a `contested` marker, 1 pool in each of its controller's unlock
  phases or yielded; no `leaves_play`), two contested copies' title is not
  contested, and the first player gets 3 transfers in their first
  influence phase, then everyone 4. Its decks follow the variant: a
  library of 40 to 60 cards, every card on VEKN's list of allowed cards
  in 2-player VTES (else `SpecError` `two_player_deck`: `seat`, `cards`
  off the list, `library`); `GET /catalog?seats=2` lists VEKN's suggested
  2-player decklists.
- A Methuselah whose library is exhausted and whose turn began short of
  a full hand may announce a **withdrawal** in their unlock phase
  (`withdraw_from_game`): kept until their next turn begins — no pool
  lost or spent, no blood a vampire of theirs loses or spends, no minion
  of theirs in combat — they leave the game with half a victory point,
  their predator gaining nothing (rules-qa 95; tournament rules).
- In their influence phase, a Methuselah's Sabbat vampire following no
  **path** may burn 1 blood to follow one (`follow_path`: its `minion`,
  the path its `value`: `caine`, `cathari`, `death_and_the_soul`,
  `power_and_the_inner_voice`); the path a vampire follows is its trait
  (`path_of_…`, §6) (rules-qa 97; Black Chantry 2025-06-12).
- A combat's windows ask every Methuselah in turn: the acting one, the
  defending one, then the others clockwise from the acting one, a
  Methuselah outside the combat offered only what reaches it (rules-qa
  98; from 2.15.0).
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
  after the idle timeout (`joined.idle_timeout`). A server restart
  resumes the games going, rebuilt from their records: the seat joining
  again (its token as before) gets a fresh start, `joined` and the
  pending decision, the human's seat named `human` (a record keeps no
  name). One the restart cannot resume (its idle time run out, a rules
  change its record no longer replays on, a bot no longer served as
  recorded) is ended, `server_restart` (§11): joining it then finds no
  game (its socket's URL answers 404, §10).

### A minimal client

Python, with the `websockets` package: it plays the first option of
every decision against three scripted bots. It is
`scripts/minimal_client.py` (the api crate's tests keep the two alike):
`uv run --script scripts/minimal_client.py [server]`, the server's base
URL (`https://engine.krcg.org` by default, `http://127.0.0.1:8040` for a
local one).

```python
# /// script
# dependencies = ["websockets"]
# ///
"""Plays the first option of every decision against three scripted bots:
`uv run --script scripts/minimal_client.py [https://engine.krcg.org]`."""

import asyncio, json, sys, urllib.request
import websockets

SERVER = sys.argv[1] if len(sys.argv) > 1 else "https://engine.krcg.org"

def get(path):
    return json.load(urllib.request.urlopen(f"{SERVER}{path}"))

async def main():
    decks = dict(get("/catalog"))
    deck = decks["Toreador (Fifth Edition)"]
    seats = [{"name": "Me", "deck": deck, "player": {"type": "human"}}] + [
        {"name": f"Bot {i}", "deck": deck, "player": {"type": "bot", "profile": "wall"}}
        for i in range(1, 4)
    ]
    body = json.dumps({"seats": seats, "client": "my-client 0.1"}).encode()
    request = urllib.request.Request(
        f"{SERVER}/games", body, {"Content-Type": "application/json"}
    )
    game = json.load(urllib.request.urlopen(request))
    socket = SERVER.replace("http", "ws", 1)
    async with websockets.connect(f"{socket}/games/{game['game']}/ws") as ws:
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
| Card | `KrcgId` | KRCG card id ([vtes.json](https://static.krcg.org/data/vtes.json)); a deck lists ids as strings, or names (`GET /cards`, §10), recorded by ids |
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
told to every seat by a `revealed` event, its `id` without a number;
another seat's hand an effect lets the seat look at (SchreckNET,
Vaticination) is told to that seat alone by a `looked` event, the cards
as they stood, ids without numbers (the look lasts while the action
does). A card coming into
a numbered place from a hidden one takes a fresh number, never used
before for this viewer, once a view has shown it out of sight; one out
of sight and back between two views keeps its number (its moves in and
out told: the same physical copy, nothing more). So a number tells
nothing the card's face and its moves do not. Two seats may number the same card differently; records use the
engine's own numbers.

**Labels.** A card whose face the viewer sees carries `nth`: its label
among the copies of that card from the same deck ("Embrace #2"), the
lowest number no other visible copy holds, given as it comes into sight.
A minion, in the view and as it enters or leaves play, also carries
`numbered` from the second minion of its name the viewer meets in the
game, whatever its deck ("War Ghoul (2)"), kept once given (an undo
takes back those given since); never a ghoul token.

## 4. Commands (client → host)

A JSON object tagged by `type`, with an optional client `id` (echoed by
an `error`).

| `type` | Fields | Effect |
|---|---|---|
| `join` | `token`?, `after`? | take the seat (the first frame on a server connection: the seat's token); `after`: resend the messages after that `seq` |
| `answer` | `decision`, `option` | take an option |
| `allocate` | `decision`, `amounts` | an allocation of points answered whole (one amount per `allocation.recipients`) |
| `vote_all` | `decision`, `favor` | all the votes `votes` counts, one way; holds for the rest of the polling |
| `undo` | `decision` | take back the seat's previous decision, from the pending `decision` (when `undo` is true); sent twice, undoes once. With every bot decision, vote order (`vote_all`'s, taken back with its decision) and pass since, whatever was revealed or drawn since; one decision of its own back at each undo, as far as 64 of them; the bots answer again as they did (a game has one human seat); the view as first sent at that decision, the numbers given since never given again (§3) |
| `settings` | `opportunities`?, `views`?, `text`? | change some settings (§8); the pending decision is sent again |
| `leave` | — | end the game, recorded as abandoned |
| `feedback` | `text` (1 to 4,000 characters, trimmed) | a problem or a remark, kept in the record at the pending decision (else the last one), also once the game is over; answered `noted` |

## 5. Messages (host → client)

A JSON object with `seq`, `reply_to`? and a `type`:

| `type` | Fields |
|---|---|
| `joined` | `protocol`, `engine` (commit), `game`, `seat`, `seats` (`name`, `bot` profile), `settings`, `idle_timeout`, `view` |
| `events` | `events`: one step — a decision taken and what it did until the next — or the engine's run before a decision or the end (§9); `view` after a step when `views` is `steps` |
| `decision` | `decision`, `seat`, `window`, `source`?, `purpose`?, `about`?, `due`?, `options`, `undo`, `allocation`?, `votes`?, `view`; `prompt` with `text` |
| `over` | `vp_halves` (per seat), `winner`? (the most VPs, at least 2, alone), `reason` (`{"type": …}`: `last_standing`, `time_limit`, `humans_ousted`, `abandoned` + `seat`, `engine_error`; in a record only, `server_restart`), `view` |
| `noted` | `decision`: the seat's `feedback` recorded, at that decision (`reply_to` its `id`) |
| `error` | `error`: `stale` (+ `expected`), `not_your_turn`, `option` (+ `index`), `no_undo`, `allocation` (+ `reason`: `none`, `length`, `total`, `cap`, `min_nonzero`), `no_votes`, `over`, `not_joined`, `malformed` (+ `reason`), `rate_limited`, `seat_taken` |

`window` is a stable id (`minion_phase`, `action_block`, `combat_strike`,
… 41: the schema's `Window`). `source` is the card whose terms or effect
the decision chooses, when it is one (a play's or an action's terms, a
referendum's): the options' `effects` say what each choice gets.
`purpose` is what an effect asks the choice for, when one does (88 ids,
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
between, a block attempt's; `margin_blood`: the recipient its pass
skips), `card` (named or in play), `clan` (or a
circle), and `detail`, what sets it apart within its purpose (`minion_of`:
`lock`, `lock_vampires`, `corrupt`…; a terms window: `blood_range`
(from `minion`, to `to` or else the pool), `x_blood`, `discard`,
`discard_any`, `put_on_source` (a card from the hand put on the
source), `cost_reductions`, `cost_counters` (how many of `card`'s),
an ability's count by what it counts — `count` (cards drawn, as many
then to the library's bottom), `blood_on_target` (on `minion`),
`hand_to_bottom`, `discard_at_random` (from `seat`'s hand) —, a rider
paid unit by unit by its units — `pool_or_blood`, `pool_or_hand_cards`,
`pool_or_ash_cards`, `pool_or_burn` (or the source burned) —,
`burn_allocated`, `gains_pool`…).
A client tells its prompt from these alone, as the server's English
rendering (the `text` setting) does. `due` says why a minion phase
offers no `pass`: what it must do before it ends,
each `{kind, copy}` — `hunt` (a vampire with no blood, or one a card
says must hunt, hunts before any other action: rules-qa 17.2; other
minions' actions wait), `action` (a vampire's printed mandatory action,
Elen Kamjian's bleed), `take_control` (the card taking control of its
host this phase, Puppet Master); empty whenever `pass` is offered.

## 6. The view

`TableView`: the table as the seat sees it — a snapshot to diff against
the previous one, never the engine's state.

- `turn`, `active`, `phase` (`unlock`, `master`, `minion`, `influence`,
  `discard`; `over` in the `over` message's view, however the game
  ended), `decision` (the next one's number).
- `seats[]`: `name`, `pool`, `vp_halves`, `ousted` (out of the game:
  ousted, or withdrawn), `withdrawing`? (a withdrawal announced, not
  broken), `withdrew`? (out by a withdrawal), `edge`, `prey`,
  `predator`; zones as cards `{copy?, id?, crypt?, nth?, numbered?}` — `crypt`,
  `library` (top last; while the seat searches it, the cards it may take
  show their `id`), `hand`, `ash_heap`, `removed`, `limbo` (cards
  being played); an ousted seat's places empty (its cards out of the
  game); `training[]` (library cards in the uncontrolled region
  with training counters, Web of Knives Recruit: as `in_play[]`'s, the
  counters the training's); `uncontrolled[]` (`card`, `counters`, `banished`,
  `cards`); `ready[]`, `torpor[]` and `incapacitated[]` minions (an
  imbued that lost its last life, controlled and not ready, its `blood`
  the life it has gained there since (Rejuvenate): the rulebook's
  incapacitated region, its own zone `incapacitated`); `in_play[]` (cards on no
  minion).
- Minion: `card`, `kind` (`vampire`, `ally`, `created`, `token`,
  `imbued`: a crypt card as a mortal ally, its `blood` its life), `name`,
  `blood`, `capacity`, `locked`, `contested`, `title` (id), `owner`,
  `cards[]` (in play on it), `markers[]` (`kind`, `seat`, `n`, `card`),
  `traits[]` (`black_hand`, `infernal`, `red_list`, `blood_cursed`,
  `scarce`, `sterile`, `seraph`, `path_of_caine`, `path_of_cathari`,
  `path_of_death_and_the_soul`, `path_of_power_and_the_inner_voice`: as
  they stand, Black Hand gained, infernal made and a path followed
  included; a Red List minion marked this turn carries a
  `red_list_mark` marker, its `seat` the marking Methuselah; a minion out
  of play with its cards, an `out_of_play` one; one contested yet in play,
  a `contested` one, its `contested` field false: a two-player game, §1).
- `action`: `seat`, `acting`, `kind` (`bleed`, `hunt`, …, `card`), `step`
  (`announced`, `open`, `attempt`, `declined`, `would_succeed`,
  `blocked`, `after_combat`, `resolved`), the `card` and its `mode` and
  `x`, `source`, `target`, `bleed_target`, `bleed`, `stealth`, `blocker`,
  `intercept`, `declined`, `declining` (the attempt under way is short of
  the stealth and its Methuselah passed: blocks are not declined yet, and
  it declines unless the table helps the attempt before the round ends,
  rules-qa 82.1).
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
  splits into, offered or not), `amount`, `favor`, `blood`, `zone` (where a
  card taken or played is: `find_card`'s library, hand or ash heap and whose,
  none for a card only named, Master: Discipline, or stored on a card;
  `take_from`'s, `ash_vampire`'s, its crypt or the ash heap and whose (Shroud
  of Decay: the prey's); the ash heap a card is played from, Spiritual
  Guidance), `value` (a clan, Discipline or circle id), `ability`, `slot`
  (`{seat, slot}`: an uncontrolled vampire by its place in the view's
  `uncontrolled`, its card face down to the others — an option's `target` or
  `card` names it only to who sees it). Two options of a decision never share
  a key (copies alike in hand are one option: the copy a `play` took is its
  `card_moves` into limbo, §9). Each key names what its kind is about, so a
  log reads it without its English (`crates/host/tests/keys.rs` holds the
  list): a `play` its `card` and `mode`, a `use_ability` its `card` or
  `minion` and `ability`, a `take_action` its `actor`, `action` and, a bleed,
  the seat bled (`target`: the prey); a choice among minions its `minion`
  (`declare_block`, `cast_votes`, `choose_minion`, `resolve_printed`,
  `burn_blood_from`, …); `transfer`, `withdraw`, `influence_out`, `merge` and
  `slot` their `slot` (the Great Beast's Discipline slot its `value`);
  `follow_path` its `minion` and the path's `value`; a combatant's choice
  (`hand_strike`, `round_strike`, `use_maneuver`, `use_press`, `use_prevent`,
  `strike_combat_ends`, `committed_strike`, `gain_strength`, `cancel_combat`)
  and the acting minion's own (`continue_action`,
  `take_damage_after_resolution`) its `actor`; `diablerise` the blocker
  diablerising as `actor`, the acting vampire as `minion`; a card chosen
  (`discard`, `bottom`, `find_card`, `take_from`, `resolve_rider`,
  `burn_in_play`, `burn_for_vote`, …) its `card`; `set_x` and `pay_rescue`
  their `amount`; `allocate_to` and `allocate_to_card` their `target` and
  `amount`; `choose_seat` its `target`; `choose_clan`, `choose_circle`,
  `choose_discipline` their `value`; `cast_edge`, `cast_card_vote` their
  `favor`; a contest's its `minion` or `card`. The rest (`pass`, `edge_pool`,
  `withdraw_from_game`, `burn_pool`, `draw_library`, `draw_crypt`,
  `pay_block_toll`, `pay_to_cancel`, `burn_blood_to_avoid`) read from their
  `effects` and the decision's `source` and `purpose`, which `decided`
  repeats.
- **`effects`** — what taking it does: an ability's effects, a play's
  mode's, a cost, what a chosen Methuselah or minion gets. A curated
  vocabulary, not the engine's own effects: each
  `{kind, n?, x?, on?, seat?, copy?, cost?, when?, card?, to?, action?,
  favor?, strike?, range?, role?, clan?, detail?}`.
  - `kind` (39, open): `pool`, `blood`, `life`, `counters`,
    `transfers`, `damage`, `strike`, `strength`, `stealth`,
    `intercept`, `bleed`, `prevent`, `press`, `maneuver`, `range`,
    `end_combat`, `enter_combat`, `immune`, `burn`, `lock`, `unlock`,
    `move`, `steal`, `draw`, `discard`, `search`, `votes`,
    `referendum` (called), `fails`, `cancel`, `action` (declared:
    `action`; without it, a phase's action: `detail` `master_phase`, the
    master phase action spent as a cost, or `discard_phase`, a discard
    phase action gained), `hunt`, `hand_size`, `cost` (reduced),
    `corruption`, `keep`, `role`, `end_phase` and `block` (a pass's,
    below; §12).
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
    `from_pool`, `per_counter`, `then_bottom`, `master_phase`,
    `conviction` (a burn of the imbued's convictions, `n` of them),
    `opponent_strength`….
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
    so (`lock`, `on: target`). A play's or an ability's effects `on:
    target` carry the `clan` its target must be (Grand Ball's first:
    `toreador`); a card put on its target, `move` `to: in_play`, `on:
    target`, `card` the card played (Priority Contract).
  - A round's strike granted by a card (`round_strike` without a card):
    `strike`.
  - A cost's terms: `cost` (kind) for a card's or a vampire's reduction,
    `blood` (`cost`, `on: chosen`) for a vampire paying 1 of it.
  - A `set_x` option's (the key's `amount`): what the amount does, by
    the terms asking it — `blood` (`x`: the card calls it X; `copy`:
    the vampire it is taken from, or with `on: target` and a positive
    `n` put on; a blood range's, then where it goes: `pool`, or `blood`
    `on: uncontrolled`, its `copy`), `pool` (`on: target` given away;
    `detail: from_blood` the referendum's margin to the pool),
    `counters` (a cost reduced), `draw` (`detail: then_bottom`), `move`
    (`to: library`, `detail: hand_to_bottom`), `discard` (`detail:
    at_random`, from `seat`'s hand); a rider's units: `burn` (`detail:
    hand_at_random`) or `move` (`to: removed`, `detail: ash_at_random`),
    the rest in `pool` (`cost`); burning pool for votes: `pool` (`cost`)
    and `votes`.
    A `discard` putting the card on the source instead (Fleshforge
    Chamber): `move` (`to: in_play`, `on: card`, `card` the source,
    `detail: on_source`).
- `abilities` — a `use_ability`'s: how many its source has (`ability`
  in the key says which); `apart` — what sets it apart from its
  source's other abilities, offered or not: its effects not all of them
  have (as a mode's `apart`), alike on every option of the ability
  whatever its actor or target, so a client asking which ability names
  each by it; empty for a lone ability or one its effects do not tell
  apart (then its number, `ability` + 1).
- `user` — a `use_ability`'s: who uses it, `{"kind":"minion","copy":n}`
  (the key's `actor` when it has one — the minion declaring the action
  it grants, a vampire using a card it is not on —, else the minion the
  card is on, the ally it is — War Ghoul burning a location —, the
  vampire whose text it is) or `{"kind":"methuselah","seat":n}` (the
  deciding seat, with no minion: a master's, a location's, a vampire's
  text its controller's prey uses; the seat said, so a `user` reads
  alone). An absent `actor` is not the
  Methuselah: `user` says.
- A pass doing more than declining the options beside it says what in
  its `effects`: `end_phase` (`detail`: `master`, `minion`,
  `influence`, `discard`) ends the phase; in `action_block`, `block`
  `detail: declines` is the seat's final decline to block (no attempt
  pending), and `block` with `copy`, its own minion's attempt under way,
  lets it stand (it blocks if it meets the stealth as the round ends;
  with `declines`, short of it, the pass is the decline); `block`
  `detail: resigned` resigns a block attempt (a toll unpaid); what it
  burns, locks or damages (`burn`, `lock`, `damage`, `copy`), a vampire
  made Camarilla (`role: camarilla`), transfers lost (`transfers`,
  `detail: none_next_influence`), pool gained. A pass with no effects
  takes none of the options beside it: a search stops, nothing more is
  moved or drawn, the card stays; the decision's `purpose` says which
  choice, `combat_press` presses no further, a polling's passes for now.
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

- `opportunities` (default `true`): `false` hides the options that interrupt
  play out of their moment — a wake outside an action that concerns the
  seat, as a card is played (unless the woken minion could then cancel that
  card: Rewind Time, Touch of Clarity), or, while blocks are open, outside
  the seat's own block moment (before the would-be blockers ahead of it
  declined, after its own decline), or, in the blocks-declined window, on
  anything but a bleed at the seat (Shadow Sentinel: to wake and bounce it),
  or in a referendum, a block denial short of the action's stealth, the
  acting seat's options other than the pass while the attempt under way is
  short of it (the round is the blocker's and the table's), a card or
  ability making a strike's damage aggravated (Claws of the Dead,
  Crossbreaker) before the strikes are chosen when it is usable once they
  are, an any-time ability where nothing else is on offer, an ability that
  would gain nothing now (legal all the same: burning the counters of a card
  that has none, votes for a vampire that cannot vote, Gathii's reveal
  before stealth is needed), a block attempt that cannot reach the action's
  stealth (every legal attempt is offered: with no intercept to gain, it
  fails). A decision left with a pass alone passes by itself
  (`auto_passed`).
- `views` (default `decisions`): `steps` adds the view to every `events`
  message — a client animating each move. Larger: compression
  (permessage-deflate) is on at the server.
- `text` (default `false`): English renderings beside the data — `text`
  on options, events, strikes and markers, a decision's `prompt` (a
  minion phase's: what it owes first, its `due`) — for a client that
  does not compose its own. Options and events are told from the
data sent alone, the names the client was shown, as a client could
tell them itself.

## 9. Events

`{"kind": …, …}` in order (`text` added with the `text` setting). Each
`events` message tells, in order, what happened since the last message:
a step (`decided`, `auto_passed` or `undone` first, then what the
decision did, up to the next decision), or the engine's run before a
decision or the game's end is sent (no decision event: the turn, draws,
an action resolving through windows no one could act in). Every change
is told, a decision's own included, in the order it happened: an
action's events between its `action_starts` and its `action_ends` (an
ousting bleed's `pool` and `ousted` before its end), a minion's
`enters_play` before the actions it takes, a turn's `turn` before the
events of that turn. `text` is set on the events the English log tells
(it leaves out what a decision's own text says).

Events are public: a card they name is shown (`id`), numbered as the
recipient knows it (§3), a card that came and went between two views
numbered fresh; but a `card_moves` (and a `looked`) is told to each
seat as it may know the card: a back where neither place shows it to
that seat, as its places showed it when it moved. `effect`: a lasting effect whose card is in the ash
heap (a marker on its minion, §6), no copy of its own.

| `kind` | Fields |
|---|---|
| `decided` | `decision`, `seat`, `window`, `source`?, `purpose`? (as the decision tells them to its seat: the card whose terms or effect it chose, what an effect asked for), `choice` (an option, as the recipient may know it) |
| `auto_passed` | `decision`, `seat`, `window` |
| `undone` | `seat`, `to`: drop what was shown since decision `to` |
| `turn` | `turn`, `active`: its unlock phase begun |
| `phase` | `phase`: a phase begun (never `unlock`, its `turn`'s; never `over`, the `over` message's) |
| `transfers` | `left`: the active seat's transfers left, as its influence phase begins and at each change |
| `draw` | `seat`, `n`, `crypt` |
| `revealed` | `seat`, `card` (its `id`, `crypt`; no number): a card of a crypt or library an effect revealed to all (a top card, a search's find) |
| `looked` | `seat`, `cards` (their `id`s; no number), `zone`?: `seat`'s hand as it stood when an effect let this seat look at it (SchreckNET, Vaticination), told to the looking seat only; `zone: library`, the top of its own library, top first (Anna "Dictatrix11" Suljic) |
| `resolved` | `seat`, `card`, `effects`? (the options' vocabulary, §7, `on` as it applies): a card played resolved — what its mode does (none when it takes no effect: its payer burned paying), the changes it makes told by the events after it, so its `pool`, `blood` and `life` effects at once are not among `effects` (their events follow, the card their `cause`; one that comes later, its `when` set, stays); an action card's end is its `action_ends` |
| `canceled` | `seat`, `card`: a card played canceled (still played: to the ash heap) |
| `pool` | `seat`, `delta`, `pool`, `cause`? |
| `blood` | `minion`, `delta`, `blood` (an ally's life), `cause`? |
| `enters_play` | `card`, `seat` (its controller), `on`?, `minion`? (its kind, for a minion), `effect`? |
| `moves` | `card`, `on`?, `effect`? |
| `card_moves` | `card` (as the recipient may know it: a back, `{}`, where neither place shows it), `from`, `to` (`{seat, zone, on?}`: `zone` a §6 place, `stored` on a card, `training`; `on` the card it is stored on, the minion it is on, a banished vampire's slot its cards are with, the minion a merged vampire's card is now): every card's move between places, in play included (drawn, discarded, played into limbo, entering or leaving play, a card on a minion moved to another, a card or minion changing hands, a merge: the base card onto its minion, in no view's list; a vampire moved within its uncontrolled region, a reshuffle's); `seat` the place's holder as it moved (a zone's seat, a card's controller, a minion's, the minion's for a card on it); `slot`, in an uncontrolled region: `from`'s which of its cards left (a face-down one's too), `to`'s where it goes, each from the first, the moves told before applied |
| `leaves_play` | `card`, `to`? (the zone it went to), `minion`? (its kind), `contested`? (into a contest: a vampire, a unique card, or the cards on a contested minion with it, each told `enters_play` again as the contest resolves in its favor; with no `to` and not contested, out of play with its minion: Descent into Darkness, its `out_of_play` marker, `enters_play` again on return), `effect`? |
| `torpor` | `minion`, `enters` |
| `incapacitated` | `minion`, `enters`: an imbued into the incapacitated region (true) or out of it |
| `locked` | `copy`, `locked`: a minion or a card in play locked or unlocked, or one in a contest (its unlock phase) |
| `counters` | `copy`? (a card in play or in training), `slot`? (`{seat, slot}`: an uncontrolled vampire, its place in the region as the events have kept it when told; another seat's a back), `delta`, `counters`, `cause`? (as `pool`'s) |
| `controller` | `copy`, `seat`: a minion or a card in play now controlled by `seat` (a minion taken, The Coven's predator) |
| `capacity` | `minion`, `capacity`?: a minion's capacity now |
| `title` | `minion`, `title`? (absent: none): a vampire's title now |
| `markers` | `minion`, `markers` (§6's): a minion's markers now, told when they changed since the last view (a Red List mark, a counter of a kind; a lasting effect, its card told entering play too) |
| `traits` | `minion`, `traits` (§6's): a minion's traits now, told when they changed since the last view (Black Hand gained, made Red List or infernal) |
| `victory_points` | `seat`, `vp_halves`: a Methuselah's victory points now, told when they changed since the last view (an oust's predator, as the table stood before the Methuselahs ousted at once; the last standing; a survivor at the time limit) |
| `ousted` | `seat` |
| `withdrawal_announced` | `seat`: a withdrawal announced (§1) |
| `withdrawal_broken` | `seat`: the withdrawal broken (pool or a vampire's blood lost or spent, a minion in combat) |
| `withdrew` | `seat`: out of the game by the withdrawal (in place of `ousted`: the seat's `ousted` and `withdrew` true; no `victory_points` for the predator) |
| `edge` | `seat`? (who holds the Edge now; absent: burned) |
| `block_fails` | `minion` (its block attempt ended, the action goes on) |
| `action_starts` | `seat`, `acting` (the minion's card, numbered by its name, as `enters_play` tells one: it may have come into play in the same run), `minion` (its kind), `action`: an action announced, by a decision or by the engine itself (a forced action: a hunt, Elen Kamjian's bleed); its `action_ends` follows, however short the action, before the next is announced |
| `acting_changes` | `seat`, `acting`, `minion` (as `action_starts` tells them): another minion of the action's Methuselah now acting in it (Mask of a Thousand Faces); the action goes on as theirs, its `action_ends` naming them |
| `action_state` | `action` (§6's): the action under way as it stands, told when it changed since the last view (its step, stealth and intercept, the bleed, the seats that declined) |
| `action_ends` | `seat`, `acting`, `action`, `outcome` (`succeeds`, `blocked`, `fails`, `fizzles`, `ousted`, `ends`), `blocker`? |
| `referendum_state` | `referendum` (§6's): the referendum under way as it stands, told when it changed since the last view (its step, terms and votes) |
| `referendum_ends` | `outcome` (`passes`, `fails`, `canceled`), `votes_for`, `votes_against`, `action`? (the political action), `hunted`? (a blood hunt's diablerist) |
| `combat_begins` | `minions` |
| `combat_state` | `combat` (§6's): the combat under way as it stands, told when it changed since the last view (its round, range, each side's strike, maneuvers and presses) |
| `strike_declared` | `minion`, `strike`? (none: no strike) |
| `strikes` | `round`, `pair`, `range`, `sides` (`minion`, `strike`?, `damage`: before prevention, its retainers' and environmental damage included; `strike_damage`: the strike's share) |
| `combat_ends` | — |

`action`: `{kind, card?, source?, target?}` — the action's kind, its
card, what grants it (a card in play, another vampire), what it is aimed
at as it stands when told (`{"kind":"seat"}`: a Methuselah, a bleed's
the one bled; `{"kind":"card"}`: the minion or card in play it
targets; an uncontrolled vampire not named).

`cause`: `{kind, card?, rule?}` — what made a `pool`, `blood` or
`counters` change
(always sent; absent only from a host before 2.5.0). `kind`: `cost`
(paid for it) or `effect` (done by it). `card`: a card played or in
play, or a minion (its own action — a bleed, a hunt — its ability, its
rider, its strike's damage after combat, its own cost: a maneuver, an
infernal unlocking; its blood or life as it comes into play, when
nothing else put it there); an action's own effects name its card, or its
acting minion. `rule`, with no card: `strikes` (combat blood: damage, a
strike stealing it; the `strikes` event before it), `transfer` (the
influence phase: 1 pool to or from an uncontrolled vampire, or for a
crypt card drawn), `oust` (the
predator's 6 pool, the ousted Methuselah's pool gone), `edge` (the Edge
burned for pool), `contest` (a contested card or title paid for),
`diablerie`, `capacity` (blood above capacity draining, an uncontrolled
vampire's counters beyond it as it comes into play), `vote` (pool
burned for votes: Business Pressure's round), `disease` (a disease
counter's blood as its vampire unlocks), `withdrawal` (a withdrawing
Methuselah's pool gone with them). Pool burned by several cards at
once (their ousts simultaneous) names the first of them.

Events tell what happened; the view says what is. A client keeping the
table from events alone has the next view: the last view, each event
applied in order, is the next decision's (`over`'s too) — the host's
tests hold every view to it. What it applies:

- each card's place by its `card_moves` (a reducer reads those; `draw`,
  `enters_play`, `leaves_play`, `moves` and `resolved` tell the same
  moves for a log, in words): a card told into a place goes last (into
  an uncontrolled region, at its `slot`: the region's order is the
  events', which `counters` name places by); it keeps its state from
  place to place (a slot its counters, a card in play its own, a minion
  its own), a hidden place shows it as a back;
  an uncontrolled vampire comes into play with its counters as blood, a
  minion or card leaving play into the uncontrolled region is banished
  there (face up), a minion taken keeps its `owner`, a card onto a
  minion is in its `cards`; a merge makes the minion the base vampire's
  under the advanced card (the advanced card's counters burned), its
  `numbered` the move's;
- a minion or card coming into play (or back from a contest, or into
  one at once) has what it brought (a vampire its counters, a minion
  back its own blood, else nothing; unlocked; a card its place's holder
  as controller, a card back its counters), then each difference told
  after its move: `blood`, `locked`, `counters`, `controller`, and a
  minion's `capacity`, `title`, `markers` and `traits` as the view
  finds them (its name is its card's, its kind its `enters_play`'s, its
  `numbered` its move's or its `enters_play`'s);
- a contest: `leaves_play` `contested` puts a minion out of play there
  (no title while it lasts) or a card in its own contest (`contested`;
  the cards on a contested minion are out of play with it, unmarked),
  told after its move when it comes into the contest at once (a second
  copy influenced out, a unique card played); `enters_play` ends it;
- the rest by the event of its field: `pool`, `blood`, `torpor` (ready
  or in torpor), `incapacitated` (ready or incapacitated), `locked` (a card or minion in a contest too),
  `counters` (a card's, in training too; a slot by its place),
  `controller`, `capacity`, `title`, `markers`, `traits`,
  `victory_points`, `edge`, `turn` (its unlock phase) and `phase`,
  `transfers` (none once a phase begins), a decision's number after
  `decided` and `auto_passed`, and the action, combat and referendum
  under way: `*_state` to set it, `action_ends`, `combat_ends`,
  `referendum_ends` to end it; `over`'s view the phase `over`;
- what follows from other events: prey and predator from the seats
  `ousted` or `withdrew` (each seat's next still in the game, a gone
  one's too), every card of a seat `ousted` or `withdrew` gone from its
  places (its own and those it controlled on others' minions); a
  `withdrew` seat's `ousted`, `withdrew` true and `withdrawing` false.

Where a card sits within its place is the next view's to show (a minion
out of torpor, a card come into play); so are the cards a search shows
its seat in its library (the decision's, §6). A card is numbered as the
next view numbers it (one that went out of sight and back between two
views keeps its number, §3). The `*_state` events, `markers`, `traits`,
`victory_points`, training counters and a minion's state as it comes
into play are told as a view finds them (after the events of the step
that moved them; none against the view an `undone` drops): their values
are computed (a stealth, an intercept, the votes), not moved by one
change. A client animating from events resyncs on the next view. The
host's tests also hold their order: an action told announced before a
view shows it, and ended before the next is announced; a combat begun,
its strikes, then ended; every card `resolved` or `canceled` told played
(`card_moves` into limbo, the copy the play took).

## 10. Transports

**Server** (`https://engine.krcg.org`):

| | |
|---|---|
| `GET /docs` | the API reference: this document, the routes and the types (its test requests are reads only) |
| `GET /openapi.json`, `/schema.json`, `/protocol.ts` | the API reference's document, and the types as JSON Schema and TypeScript |
| `GET /version` | protocol, engine commit, bot profiles offered, limits (seats, time limit, idle timeout) |
| `GET /catalog` | the decklists the engine ships, `[name, list]`; `?seats=2`: VEKN's suggested 2-player decklists (the variant's, §1) |
| `GET /cards` | the cards the engine plays, `[id, name]`: KRCG id, and the name a decklist may give for it (a vampire whose name several carry with KRCG's suffix: `Theo Bell (G2)`, `Theo Bell (G6)`; `(ADV)` an advanced vampire) |
| `POST /decks/check?seats=` | body: a decklist (`DeckList`, as a seat's `deck`); `{"playable": true}`, or the `SpecError` (400) a game of `seats` (default 4; 2: the two-player variant, §1) would refuse it with, its `seat` 0: `unsupported` naming each card outside the pool, `deck_size`, `two_player_deck`. A deck checked is not counted at `/admin/unsupported` |
| `POST /games` | body: a `GameSpec` (one human seat); `{game, seat, token}`, or a `SpecError` (400). The deal's seed is the server's, 53 random bits (too many to try from a known opening hand): a spec's `seed` is kept only with the admin token (a playtest's fixed deal), at most `SEED_MAX` (2^53 − 1, exact as a JSON number), else `SpecError` `seed` (its `max`) |
| `GET /games/{game}/ws` | WebSocket: one JSON command or message per text frame; the first command is `join` with the token. 404 to any request once the game has left the server (the idle timeout passed, after its end or with no command, or a server restart ended it), so a client whose socket will not reopen can `fetch` it to tell |
| `GET /games/{game}/record` | the game's record (§11), `Authorization: Bearer <token>`, once the game is over (409 before: the record replayed would show the hidden cards) |
| `GET /admin/games?since=&engine=&limit=` | the games recorded, oldest first: `id`, `engine`, `client`, `started`, `ended` (absent: going on the server now), `result`, `entries` (the operator's: `Authorization: Bearer <admin token>`) |
| `GET /admin/games/{game}/record` | any game's record, as far as it went (the operator's) |
| `GET /admin/feedback?since=&limit=` | the players' feedback, oldest first: `game`, `engine`, `at` (Unix ms), `decision`, `text` (the operator's) |
| `GET /admin/unsupported` | the cards refused decks named (`SpecError` `unsupported`), counted: `[{id, count}]`, KRCG ids, the most asked first (ids and counts only; the operator's; a card a deck named by name is not counted: the engine knows no name outside its pool) |

No account: the token is the seat, never in a URL. Any page may play
(`engine.krcg.org` allows every origin, the WebSocket's too): the
token is the gate, no cookie or credential rides a request.

## 11. Records

Every game served is recorded (`GameRecord`), and served to its seat
once the game is over (§10): the spec with its seed (53 bits from 2.0.0,
32 before; human names replaced; its decks by KRCG ids, a card the
request named by its id from 2.12.0), the bot profiles (weights' hash,
temperature, seed, also at most `SEED_MAX`, a network's nudges by name), each
decision answered — seat, time, option index, its key with the engine's copy
numbers, by whom (`human`, `bot`, `order`, `auto`), the options shown when
settings hid some, from 2.4.0 its window and the digest of the position it was
taken on — undos (decision numbers then repeat from `to`: a reader drops what
they take back), settings changes, refused commands, the players' feedback (at
its decision), the result (with the turn the game ended on, `turns`; for a
game a restart ended, the turn its record reads to) and any engine failure. A
game going when its server restarts is resumed from its record, its entries
kept (the record names the engine it started on); one the restart cannot
resume is ended at the restart, `server_restart`: its victory points where its
record reads to on the restarted server's engine, `failure` saying why when it
does not read to its end (rules fixes change the engine, §14). A decision's
`digest` is 16 hex digits of FNV-1a 64 over the position as KRCG ids, the same
on every engine while the position is: for each seat in table order its pool,
hand, library ash heap and crypt ash heap (sorted), library and crypt (bottom
first), uncontrolled region (sorted, with counters; a banished created vampire
by its library card), cards in play on no minion (sorted), and its minions in
play (sorted, each with its blood, lock, torpor and the cards on it, sorted;
contested and out-of-play ones included). An opaque hash: the record holds no
hidden card in plain while the game runs. Its definition is frozen: changed,
it would stop every record written since 2.4.0, so a new one is read by the
record's `protocol`. A record read on a later engine stops at the first
decision whose position differs (a rules change that moved hidden state with
no decision, as a shuffle: the game read is no longer the game played, though
its choices are still offered), or that was taken in a window other than the
one asked (a window newly asked). A replay reads past the Passes recorded in a
window the engine no longer asks when the decision after them is the one
asked, in its window and position; a resumed game ends there instead,
`server_restart`, as its decision numbers would no longer be the session's.
The digest is strict: a fixed cost or gain stops the records it touches too. A
record replays by index on its engine commit, and reads by key on a later
engine (a key written before its kind's fields grew matches the option whose
fields it has, alike: a bleed's before it named its seat; a record from before
every block attempt was offered reads past the block windows its engine
skipped; one from before rules-qa 82.1 (protocol before 1.9.0), whose engine
reopened the block window at the acting Methuselah after an attempt failed
short of the stealth, reads past the passes it recorded there, and after a
play there (read as the engine asks it) the round of passes that closed it;
the engines from 22cf131 to it, never deployed, left no record read; one from
before rules-qa 88.5, whose engine ordered Anarch Revolt's burn among the
unlock-phase riders, reads past the rider orders the engine no longer offers
by their key, a last one too as the game ends before it; one from before
2.12.0 passes unread the burn option of a master card requiring a ready Seraph
(Watchtower: Four Ride Forth), never offered then; one from before rules-qa 98
(protocol before 2.15.0), whose engine polled the combatants' Methuselahs
alone, passes unread a combat's ordered window asked of a Methuselah outside
the combat; and any record reads past a Pass recorded by a seat this engine
elides there, as sure as the digest of the decision after it: a second Wall
of Filth before rules-qa 99.1, refused, left the seat a lone Pass). Kept two
years, to train and test the engine; anonymous.

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
renamed field or a changed meaning, a major one (2.0.0: the seeds'
range widened past 32 bits). The api crate's own readers,
1.3.0 on, read past a record entry type they do not know, and read an
end reason, an error or a value of a §12 closed set (a decision's `by`
among them) they do not know as `unknown` (never sent); any other new
value still fails the read, and a reader built before 1.3.0 needs
records no newer than its engine. `engine` is the commit: rules
fixes ship without a protocol change. `CHANGELOG.md` lists every version;
`scripts/schema_compat.py --check` fails a schema change that removes or
renames a field or an id unless the major moves.
