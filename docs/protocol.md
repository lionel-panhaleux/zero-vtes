# The zvtes protocol

How a client plays a game of *Vampire: The Eternal Struggle* on the
zero-vtes engine: a rules-enforcing engine with bots, served at
`https://engine.krcg.org` or embedded in a page (wasm). The client shows
the table and turns its user's moves into answers; the engine decides
what is legal, runs the bots and says what happened.

Version **1.0.0** (`PROTOCOL_VERSION`). Sources: `crates/api` (Rust, the
reference); `crates/api/schema.json` (JSON Schema 2020-12, generated);
`crates/api/protocol.ts` (TypeScript types, generated from the schema).
Status: draft — the host implementing it is being built
(`docs/online-plan.md`).

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
- A command accepted has no reply of its own: its effect comes as
  events (an `answer` as an `events` message starting with `decided`).
  A refused command changes nothing and is answered by an `error`
  carrying the command's `id` as `reply_to`.
- The game ends when one Methuselah is left, at the time limit, when the
  human is ousted (the bots do not play on), when the human leaves, or
  after the idle timeout (`joined.idle_timeout`).
- Embedded (wasm), the same JSON goes through function calls (§10).

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
viewer's own hand. A card in a hidden place — another seat's hand, any
library or crypt — has no number: a card back, `{}`. A card coming into
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

## 5. Messages (host → client)

A JSON object with `seq`, `reply_to`? and a `type`:

| `type` | Fields |
|---|---|
| `joined` | `protocol`, `engine` (commit), `game`, `seat`, `seats` (`name`, `bot` profile), `settings`, `idle_timeout`, `view` |
| `events` | `events`: one step — a decision taken and what followed until the next, or the deal; `view` after it when `views` is `steps` |
| `decision` | `decision`, `seat`, `window`, `source`?, `options`, `undo`, `allocation`?, `votes`?, `view`; `prompt` with `text` |
| `over` | `vp_halves` (per seat), `winner`? (the most VPs, at least 2, alone), `reason` (`{"type": …}`: `last_standing`, `time_limit`, `humans_ousted`, `abandoned` + `seat`, `engine_error`), `view` |
| `error` | `error`: `stale` (+ `expected`), `not_your_turn`, `option` (+ `index`), `no_undo`, `allocation` (+ `reason`: `none`, `length`, `total`, `cap`, `min_nonzero`), `no_votes`, `over`, `not_joined`, `malformed` (+ `reason`), `rate_limited` |

`window` is a stable id (`minion_phase`, `action_block`, `combat_strike`,
… 41: the schema's `Window`). `source` is the card whose terms or effect
the decision chooses, when it is one (a referendum's terms, a rider's
cost): the options' `effects` say what each choice gets.

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
  `cards[]` (in play on it), `markers[]` (`kind`, `seat`, `n`, `card`).
- `action`: `seat`, `acting`, `kind` (`bleed`, `hunt`, …, `card`), `step`
  (`announced`, `open`, `attempt`, `declined`, `would_succeed`,
  `blocked`, `after_combat`, `resolved`), the `card` and its `mode` and
  `x`, `source`, `target`, `bleed_target`, `bleed`, `stealth`, `blocker`,
  `intercept`, `declined`.
- `combat`: `minions` (acting first), `round`, `range`, `sides[]`
  (`strike` once declared — the blocker chooses knowing the acting
  minion's — `maneuvers`, `presses`). A strike: `kind`, `n`, `ranged`,
  `aggravated`, `card`, `resolved`.
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
  `disciplines` with levels, `card_type` for a multi-type card),
  `amount`, `favor`, `blood`, `zone` (where a search takes from), `value`
  (a clan, Discipline or circle id), `ability`. Two options of a
  decision never share a key (copies alike in hand are one option).
- **`effects`** — what taking it does when the key does not say: a cost,
  what a chosen Methuselah or minion gets, an ability's effects, what a
  pass lets happen ("let it burn", "take the 2 damage"). Each `{kind,
  seat?, copy?, n?, card?, to?, role?}`; kinds `pool`, `blood`,
  `counters`, `damage` (`n` signed: + gained, − lost), `burn`, `lock`,
  `unlock`, `draw`, `discard`, `move` (`to` a zone), `steal`,
  `corruption`, `keep`, `role` (what a chosen minion becomes, §12).
- A pass's plain meaning follows from the window: `master_phase` … ends
  the phase, `action_block` declines to block, `combat_press` presses
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
  action's stealth, an any-time ability where nothing else is on offer.
  A decision left with a pass alone passes by itself (`auto_passed`).
- `views` (default `decisions`): `steps` adds the view to every `events`
  message — a client animating each move. Larger: compression
  (permessage-deflate) is on at the server.
- `text` (default `false`): English renderings beside the data — `text`
  on options, events and strikes, a decision's `prompt` — for a client
  that does not compose its own.

## 9. Events

`{"kind": …, …}` in order (`text` added with the `text` setting). An
`events` message is one step: its first event is `decided`,
`auto_passed` or `undone` (none for the deal).

| `kind` | Fields |
|---|---|
| `decided` | `decision`, `seat`, `window`, `choice` (an option, as the recipient may know it) |
| `auto_passed` | `decision`, `seat`, `window` |
| `undone` | `seat`, `to`: drop what was shown since decision `to` |
| `turn` | `turn`, `active` |
| `draw` | `seat`, `n`, `crypt` |
| `pool` | `seat`, `delta`, `pool` |
| `blood` | `minion`, `delta`, `blood` |
| `enters_play` | `card`, `seat`, `on`? |
| `moves` | `card`, `on`? |
| `leaves_play` | `card`, `to`? |
| `torpor` | `minion`, `enters` |
| `ousted` | `seat` |
| `block_fails` | `minion` |
| `action_ends` | `seat`, `acting`, `outcome` (`succeeds`, `blocked`, `fails`, `fizzles`, `ends`), `blocker`? |
| `referendum_ends` | `outcome` (`passes`, `fails`, `canceled`), `votes_for`, `votes_against` |
| `combat_begins` | `minions` |
| `strike_declared` | `minion`, `strike` |
| `strikes` | `round`, `pair`, `range`, `sides` (`minion`, `strike`, `damage`) |
| `combat_ends` | — |

Events tell what happened; the view says what is. A client animating
from events resyncs on the next view.

## 10. Transports

**Server** (`https://engine.krcg.org`):

| | |
|---|---|
| `GET /version` | protocol, engine commit, bot profiles offered, limits (seats, time limit, idle timeout) |
| `GET /catalog` | the precon decklists the engine plays |
| `POST /games` | body: a `GameSpec` (one human seat); `{game, seat, token}`, or a `SpecError` (400) |
| `GET /games/{game}/ws` | WebSocket: one JSON command or message per text frame; the first command is `join` with the token |
| `GET /games/{game}/record` | the game's record (§11), `Authorization: Bearer <token>` |

No account: the token is the seat, never in a URL. The server checks
the WebSocket's `Origin` against the platforms it serves.

**Embedded** (`zvtes-wasm`, npm): `new Game(specJson, weights?)` (throws
a `SpecError`); `game.handle(commandJson) → messagesJson` (an array);
`game.record()`; `catalog()`; `checkDeck(deckJson)` (`null` or a
`SpecError`). The same messages; the page runs the bots.

## 11. Records

Every game served is recorded (`GameRecord`): the spec with its seed
(human names replaced), the bot profiles (weights' hash, temperature,
seed), each decision answered — seat, time, option index, its key with
the engine's copy numbers, by whom (`human`, `bot`, `order`, `auto`),
the options shown when settings hid some — undos (decision numbers then
repeat from `to`: a reader drops what they take back), settings
changes, refused commands, the result and any engine failure. A record
replays by index on its engine commit, and reads by key on a later
engine. Kept two years, to train and test the engine; anonymous.

## 12. Open sets

Snake_case ids from sets the protocol does not close:

- Clans: KRCG clan names (`banu_haqim`, `ministry`, `toreador`…);
  Disciplines: KRCG abbreviations (`dom`, `aus`…); circles likewise; card
  types (`action_modifier`, `reaction`, `combat`…).
- Titles: `primogen`, `prince`, `baron`, `justicar`, `inner_circle`,
  `archbishop`, `bishop`, `cardinal`, `priscus`, `regent`, `magaji`, or
  an independent title's name.

Closed sets (`Window`, `OptionKind`, `EffectKind`, `Role`, `MarkerKind`,
`Zone`…) are listed in the schema; a value a later host adds reads as
`unknown` in the Rust types, and a TypeScript client should treat an
unknown string alike.

## 13. In and out of scope

The host does the rules, the bots, what each seat may see, undo, the
settings above, records, and an optional English rendering. The client
does the display and its wording, gestures and their matching to
options, pacing and animation, decklists and accounts (if any). Out of
the protocol: the engine's internal state, card images, layout.

## 14. Versioning

`protocol` is semver: a new field or a new id in a set is a minor
version (readers ignore what they do not know); a removed or renamed
field or a changed meaning, a major one. `engine` is the commit: rules
fixes ship without a protocol change.
