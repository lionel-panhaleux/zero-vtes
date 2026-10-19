# Protocol changelog

Every `PROTOCOL_VERSION` of [`docs/protocol.md`](docs/protocol.md), semver
as its §14 says what a minor and a major are. `GET /version` says which the
server runs.
`python3 scripts/schema_compat.py` lists what a schema change adds or
removes; its `--check` fails a removal without a major version.

## 2.5.0

Round 18's cards (rules-qa Batch 85), the events `action_starts` and
`acting_changes`, the `cause` of `pool` and `blood`, option keys
complete and `decided`'s `source` and `purpose`, additions only;
`action_ends` now told for every action (one within a message too),
every event in the order it happened, `resolved` without what its events
tell, and undo against bots, behaviours with no field or id.

- `Purpose`: `store_own_hand` — Storage Annex as it is played: the card of
  the hand put face down on `card` (`find_card`), replaced.
- Event `action_starts` (`seat`, `acting`: the minion's numbered card,
  `minion`: its kind, `action`): every action announced, a forced action
  the engine takes by itself included (a hunt, Elen Kamjian's bleed);
  each is followed by its `action_ends`, an action announced and resolved
  within one message too (a bot's employ had none). The `action` effect
  without its `action` field documented (§7: `master_phase`,
  `discard_phase`).
- Events in the order they happened (§9): an action's between its
  `action_starts` and `action_ends` (an ousting bleed's `pool` and
  `ousted` had come after its end), a minion's `enters_play` before the
  actions it takes (a vampire created and hunting in one run), a turn's
  `turn` before the turn's events (a forced hunt had come before it).
- `pool` and `blood`: `cause` (`kind`: `cost` or `effect`; `card`, or
  `rule`: `strikes`, `transfer`, `oust`, `edge`, `contest`, `diablerie`,
  `capacity`, `vote`, `disease`), what made the change.
- `resolved`: its `pool`, `blood` and `life` effects at once no longer
  among `effects` (they repeated the events after it, which name the card
  as their cause); one that comes later (`when`) stays.
- Option keys complete (§7, each kind's fields listed): `slot`
  (`{seat, slot}`), an uncontrolled vampire by its place, on
  `transfer`, `withdraw`, `influence_out`, `merge`, `slot` and
  uncontrolled targets (a face-down one had named a copy 4294967295, and
  a `slot` choice the id of another seat's face-down vampire); a bleed's
  `target`, the seat bled; the combatant's `actor` on `hand_strike`,
  `round_strike`, `use_maneuver`, `use_press`, `use_prevent`,
  `strike_combat_ends`, `committed_strike`, `gain_strength`, the acting
  minion on `continue_action`, `take_damage_after_resolution`;
  `diablerise` the blocker as `actor`, the acting vampire as `minion`; a
  card come into play since the last view numbered in a told option's
  key and `source`, as the events number it. A record's older key reads
  by the fields it has (§11).
- `decided`: `source` and `purpose`, as the decision told them.
- Event `acting_changes` (`seat`, `acting`, `minion`): another minion
  now acting in the action under way (Mask of a Thousand Faces); its
  `action_ends` names them (it had ended an action no event announced).
- A banished created vampire's uncontrolled slot: viewed and digested by
  its library card (the view and the digest had failed on it, §11's
  definition otherwise unchanged).
- `undo` against bots (one human seat: every server game) is open at
  every decision after the seat's first, one human decision back each
  time, across the bots' decisions, vote orders (`vote_all`, the order
  taken back with its decision), passes, reveals, looks and searches, as
  far as 64 of the seat's own decisions; the bots answer again as they
  did.

## 2.4.0

- A record's `decision` entry: `window`, the window it was asked in, and
  `digest`, the position it was taken on (§11), so a record read on a
  later engine stops where its game is no longer the one played (a rules
  change shuffling a library with no decision read on silently before);
  a replay reads past the Passes of a window the engine no longer asks, a
  resumed game ends there. Records before 2.4.0 have neither and read as
  before.
- Terms told in their unit and for what (the owner: Jake Washington's
  "how many cards"): `about.detail` adds `discard_any`, `put_on_source`,
  `cost_counters`, `blood_on_target`, `hand_to_bottom`,
  `discard_at_random`, and a rider's `pool_or_blood`,
  `pool_or_hand_cards`, `pool_or_ash_cards`, `pool_or_burn`; a blood
  range's `minion` and `to`. A `set_x`'s effects say what each count is
  of (§7): Nehemiah's and Ilonka's were `blood`, The Gate of Acheron's
  and Constant Revolution's cards `discard`; Fleshforge Chamber's pick a
  `move` on it.

## 2.3.0

Round 17's cards (rules-qa Batch 84); additions only.

- `Purpose`: `may_gain_hand_size` — "you can get +n hand size until the
  end of the turn" printed on `minion` (Sascha Vykos): `resolve_printed`,
  or `pass`.
- `Purpose`: `ready_titled_left` — a ready titled vampire of `seat`'s was
  burned or sent to torpor: the out-of-turn card played for it
  (Leadership Vacuum), or `pass`.
- `Purpose`: `name_master_discipline` — Legion's action announced: the
  Master: Discipline card named (`find_card`), from hand or ash heap.
- `Purpose`: `name_ash_equipment` — Filchware's Pawn Shop's action
  announced: the ash heap (`choose_seat`), then the equipment in it
  (`find_card`).
- `Purpose`: `buy_title` — Victor Pelletier entering play: `set_x` pays
  n pool for the title its option's effect names, or `pass`.
- `MarkerKind`: `taint` (`n`; Barbaro Lucchese).
- `StrikeKind`: `burn_life` — life burned from the opposing ally, not
  damage (Argent Baton).
- `discard_chosen`'s `detail` may be `animal` (Call the Wild Hunt's X
  animals) beside `combat`.

## 2.2.0

- The `looked` event (`seat`, `cards`): another seat's hand as it stood
  when an effect let this seat look at it (SchreckNET, Vaticination),
  told to the looking seat only. The look lasts while the action does, so
  the next view hides the hand again (the owner's playtest: "schrecknet
  does not reveal the target's hand"). No undo goes back past it.

## 2.1.0

- `Purpose`: `may_store_aim` — "you can move any aim card the bearer uses
  to this card" (Light Intensifying Goggles): `resolve_rider` moves the aim
  card just used from the ash heap onto it, `pass` leaves it.
- `Purpose`: `discard_from_hand_of` may first be asked with no
  `about.seat` and `choose_seat` options (Vaticination: which other
  Methuselah's hand), then with `about.seat` and `find_card` options as
  before.
- A game never joined ends at its idle timeout, `abandoned`, as one
  joined does; one whose reply failed to render ends `engine_error`:
  both records were left going. No schema change.

## 2.0.0

- Seeds of 53 bits: `GameSpec.seed` and `BotRecord.seed` are at most
  `SEED_MAX` (2^53 − 1, exact as a JSON number in every language), the
  server's drawn from 53 random bits; a reader keeping them in 32 bits
  fails, hence the major. A 32-bit seed could be found from the opening
  hand in a minute, and the deal replayed. `SpecError` `seed` (`max`):
  a spec's seed past it. Records before 2.0.0 hold 32-bit seeds and read
  as before.
- Events name a minion by the copy it had when they happened: a merge
  between two views (the bots' turns) no longer names it in the earlier
  events by its merged copy, one the viewer had not seen. No schema change.

## 1.9.0

- The `resolved` (`seat`, `card`, `effects`) and `canceled` (`seat`,
  `card`) events: how a card played finished.
- `ActionView.declining`: a blocker short of the stealth passed, its
  decline standing unless the table helps the attempt (rules-qa 82.1).
  The block loop it belongs to: that pass is the decline, and the impulse
  goes on from the next seat, never the acting one's until blocks are
  declined. With `opportunities` off, the acting seat's options other
  than the pass against a short attempt are hidden.

## 1.8.0

- A server restart resumes the games going from their records (a fresh
  `joined` for the seat's next join); it ends only those it cannot resume,
  `server_restart`. No schema change.
- `OptionView.apart`: what sets an ability apart from its source's other
  abilities, alike on every option of it (the "which ability?" step).
- `OptionView.user`: who uses an ability, `{kind: minion, copy}` or
  `{kind: methuselah, seat}`; an absent `actor` is not the Methuselah.
- `Decision.due` (`Due {kind, copy}`, `DueKind`: `hunt`, `action`,
  `take_control`): why a minion phase offers no `pass`.
- `Card.numbered`: a minion's number among the minions of its name
  ("War Ghoul (2)"), in the view and as it enters or leaves play.
- `Purpose`: `give_card_to`; `Trait`: `seraph`; `MarkerKind`: `pact`,
  `out_of_play`.

## 1.7.0 (never deployed)

- `GET /games/{game}/record` answers 409 until the game is over: a live
  game's record (its seed, every decision) is no longer served.
- `POST /games`: the server draws the deal's seed; a spec's `seed` is kept
  only with the admin token.
- `MinionView.traits` (`Trait`: `black_hand`, `infernal`, `red_list`,
  `blood_cursed`, `scarce`, `sterile`); `MarkerKind`: `red_list_mark`.
- The `revealed` event (`seat`, `card`): a library or crypt card an effect
  reveals, told to every viewer.
- Records: `BotRecord.nudges`, `GameResult.turns`.
- `GET /admin/unsupported`: the cards refused decks named, counted.
- `Purpose`: `diablerie_equipment`, `may_prevent_for_blood`, `may_unlock`,
  `move_card_to_oldest`, `put_in_play_search`, `shortfall_picks`.

## 1.6.0

- A strike's `n` and `aggravated` are the damage the engine deals now
  (strength, bonuses and prevention applied), in the combat view and the
  strike events; `StruckSide.strike_damage`: the strike's share of the
  damage a side took.

## 1.5.0

- `Purpose`: `move_off_burned_bearer`, `name_ash_vampire`,
  `search_equip_for`, `secret_number`.

## 1.4.0

- One socket a seat: the newest `join` takes the seat, the older
  connection gets the `seat_taken` error and is closed.

## 1.3.0

- Tolerant readers: the api crate reads a record entry type, an end reason,
  an error or a closed-set value it does not know as `unknown` (never
  sent).

## 1.2.0

- The `feedback {text}` command, answered `noted {decision}`, kept in the
  record (`Entry::Feedback`) at its decision; `GET /admin/feedback` lists
  them.

## 1.1.0

- `EndReason`: `server_restart`, in records of the games a restart ended.

## 1.0.0

- The contract as served first on engine.krcg.org: games over HTTP and
  WebSocket, views per viewer, typed events, options with their curated
  effects, the `text` setting's English, records (§11); `GET /docs` and
  `/openapi.json` (the API reference), `GET /admin/games` and
  `/admin/games/{game}/record`.
