# Protocol changelog

Every `PROTOCOL_VERSION` of [`docs/protocol.md`](docs/protocol.md), semver
as its §14 says what a minor and a major are. `GET /version` says which the
server runs.
`python3 scripts/schema_compat.py` lists what a schema change adds or
removes; its `--check` fails a removal without a major version.

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
