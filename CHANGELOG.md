# Protocol changelog

Every `PROTOCOL_VERSION` of [`docs/protocol.md`](docs/protocol.md), semver
as its §14 says what a minor and a major are. `GET /version` says which the
server runs.
`python3 scripts/schema_compat.py` lists what a schema change adds or
removes; its `--check` fails a removal without a major version.

## 2.14.0

Sabbat paths followed (§1; rules-qa 97); ids added, none removed:

- `OptionKind` `follow_path`: in its influence phase, a Methuselah's
  Sabbat vampire following no path burns 1 blood to follow one; its key's
  `minion`, the path its `value` (`caine`, `cathari`,
  `death_and_the_soul`, `power_and_the_inner_voice`).
- `Trait` `path_of_caine`, `path_of_cathari`, `path_of_death_and_the_soul`,
  `path_of_power_and_the_inner_voice`: the path a vampire follows,
  printed or followed since (the `traits` event tells a change).

## 2.13.0

Tournament withdrawal (§1; rules-qa 95); ids and fields added, none
removed:

- `OptionKind` `withdraw_from_game`: offered in the unlock phase of a
  Methuselah whose library is exhausted and whose turn began short of a
  full hand.
- Events `withdrawal_announced`, `withdrawal_broken`, `withdrew` (a
  withdrawn seat is told `withdrew`, not `ousted`); `CauseRule`
  `withdrawal` (the withdrawing Methuselah's pool gone).
- `SeatView.withdrawing`, `SeatView.withdrew` (absent when false);
  `ousted` stays true for a seat out of the game either way.

## 2.12.0

A decklist checked without a game (§10); routes added, no field or id
removed:

- `GET /cards`: the cards the engine plays, `[id, name]`.
- `POST /decks/check?seats=`: a `DeckList`, answered `{"playable":
  true}` or the `SpecError` (400) a game would refuse it with, its
  `seat` 0.
- A `DeckList` (a seat's `deck`, the check's body) names a card by KRCG
  id or by its name as `/cards` gives it (any case): a vampire whose
  name several carry with KRCG's suffix (`Theo Bell (G2)`, `(ADV)`). A
  game records its decks by KRCG ids.
- `SpecError` `unsupported` names a card as the list does; `deck_size`'s
  doc says what it checks (a deal's 4 crypt and 7 library, a card past
  255); `unknown_card` is documented as never sent.

## 2.11.0

`find_card`'s key names its `zone` again (§7; lost when the
host replaced crates/protocol): the seat whose library (a search, a
reorder), hand (one looked at, Goblinism's haven) or ash heap
(Lenelle, The Erciyes Fragments, Trochomancy, Psychophagia, Sudario
Refraction, …) the card is taken from; none for a card only named
(Master: Discipline) or stored on a card. No field added or removed.

## 2.10.0

Two-player games under VEKN's variant (§1); ids added, none removed:

- A game of 2 seats plays the variant: a contested vampire (or imbued)
  stays in play and is used as normal; its upkeep is due as before
  (`pay_contest` / `yield_contest`, 1 pool), and no `leaves_play`
  `contested` is told for it: its minion carries the new marker
  `contested` instead (`MinionView.contested` stays false, meaning face
  down and out of play). Unique library cards contest as before.
- Two contested copies' title is not contested.
- Transfers: 3 in the first player's first influence phase, then 4.
- Its decks: a library of 40 to 60, every card on VEKN's list of allowed
  cards (`data/two-player.json`, from `scripts/two_player.py`), else the
  new `SpecError` `two_player_deck` (`seat`, `cards` off the list,
  `library`); `GET /catalog?seats=2` lists VEKN's 17 suggested
  decklists the engine plays (its Tzimisce one plays a card off the
  list).
- Games of 3, 4, 5 and 6 seats are unchanged.

## 2.9.0

What a pass does and what a play or an action targets, in the data (§7,
§9); ids and fields added, none removed:

- Effect kinds `end_phase` (`detail`: the phase) and `block`: a pass
  ending the master, minion, influence or discard phase says so; in
  `action_block`, `block` `detail: declines` the seat's final decline
  (no attempt pending), `block` with `copy` its own minion's attempt
  let stand (`declines` beside it: short of the stealth, the decline);
  `block` `detail: resigned` a block toll's pass.
- A pass that burns, locks or damages a minion, makes it Camarilla
  (role `camarilla`, added) or loses the next influence phase's
  transfers (`transfers`, `detail: none_next_influence`) carries that
  effect (Rotting Behemoth, Charnas the Imp, Nightmares upon Nightmares,
  Rutor's Hand, Waiting Game, Leandro).
- A pass with no effects takes none of the options beside it.
- `margin_blood`'s `about.minion`: the recipient its pass skips.
- A play's target told (Z-succubus): a play's or an ability's effects
  `on: target` carry the `clan` the target must be (Toreador Grand
  Ball's lock, Saulot's Avenging Fist's action on a Tremere); a card put on
  its target (Priority Contract and 53 more) tells `move` `to: in_play`
  `on: target`, `card` the card played; told alike in `resolved`.
- `action` (`action_starts`, `action_ends`, `referendum_ends`) gains
  `target`: the Methuselah it is aimed at (a bleed's: the one bled), else
  the minion or card it targets, as it stands when told.
- The English (`text`): a pass reads from its own data; the block
  window's passes beyond these read "Pass"; a decided pass is told in
  the events whenever it has effects.
## 2.8.0

The cards players asked for (rules-qa Batch 89); ids added, none removed:

- Choice kind `cancel_combat`: cancel the combat a minion is in, its
  `actor` the combatant — Stone Travel [MYT]'s blocked acting vampire,
  and Foresee's imbued, whose cancel was offered as `continue_action`
  (the option's index unchanged: a record reads the same).
- Purpose `put_found_on`: Goblinism [MYT]'s haven, just searched to the
  hand, put on the acting vampire instead (`find_card`, its cost paid)
  or kept (`pass`), asked before the hand is discarded down.

## 2.7.1

Behaviours only, no field or id (rules-qa Batch 88, the play-changing
contradictions):

- Anarch Revolt's choice no longer asked: its ready Anarch read as the
  unlock phase ends, resolved as the active Methuselah passes; a record
  answering that choice reads past it (§11).
- A window between the combats Psyche! queues (Obedience, Promise of
  1528): `action_after_combat` again, elided when empty.
- Taking the Skin: Minion offered before a diablerie's blood hunt only,
  Yawp Court's action ended as it reaches resolution, the Meddling of
  Semsith's held replacements drawn when it leaves mid-phase.

## 2.7.0

The Imbued (rulebook "Appendix: Imbued Rules"): a crypt card as a mortal
ally, its region when it has no life, its powers and convictions; ids
added, none removed.

- Minion `kind` `imbued`: a crypt card in play as an ally, its `blood`
  its life, its creed and virtues the card's clan and Disciplines.
- Zone `incapacitated` and `SeatView.incapacitated[]` (its minions, as
  `ready` and `torpor`'s); event `incapacitated` (`minion`, `enters`),
  the minion moved between `ready` and `incapacitated` as `torpor`
  moves it (§9's reducer).
- Action kinds `power` (an imbued gets a power: +1 stealth) and
  `burn_imbued` (an incapacitated imbued burned, its equipment taken).
- Purposes `gain_conviction`, `burn_convictions`, `avenge_imbued`,
  `name_conviction`, `power_search`, `may_cancel_combat` (88 ids).
- Event `looked`: `zone` (`library`: the top of the looking seat's own
  library, top first); absent, the hand as before.

## 2.6.0

The table kept from events alone: the last view with the events since
applied is the next (§9, the rule a client reduces by, held by the
host's tests on every decision's view); one field and one event added,
the rest behaviours with no field or id.

- `card_moves`: `slot`, in an uncontrolled region: `from`'s which of its
  cards left (a face-down one's place: its counters had no other name),
  `to`'s where it goes (a redraw fills the places the old ones left); a
  vampire moved within its region told so (a reshuffle's), from and to
  it, so the region's order is the events' too; a card moved into a
  minion's place `numbered` (a merged vampire had none told).
- Event `victory_points` (`seat`, `vp_halves`), told when a view finds
  them changed: Methuselahs ousted at once score each predator as the
  table stood before them, which `ousted` in a row could not say.
- `counters`: `slot` is the vampire's place in the region as the events
  have kept it when told (it was the next view's: wrong across a
  reshuffle, Kaymakli Nightmares); a vampire's counters coming with it
  into the region, and a training card's counters, now told.
- A minion or card coming into a contest at once (a second copy
  influenced out, a unique card or ally played) told `leaves_play`
  `contested` after its move, its state as for one coming into play.
- A minion or card coming into play, or back from a contest: what it
  has beyond what it brought told after its move — `blood` (an ally's
  life; a vampire's beyond its counters), `locked`, `counters`,
  `controller` (a card on a minion controlled by another than the
  minion's), and a minion's `capacity`, `title`, `markers` and `traits`
  (none were told: the view had them first); a vampire's counters
  beyond its capacity drained as it comes in, `rule: capacity`.
- `locked`: told for a card or minion in a contest too (its unlock phase
  unlocked it untold).
- The view: an ousted seat's places empty (its crypt cards in the ash
  heap and its removed cards stayed shown); the `over` message's
  `phase` `over` and no `transfers`, however the game ended (the
  humans ousted mid-phase kept the phase).

## 2.5.0

Round 18's cards (rules-qa Batch 85), the events `action_starts` and
`acting_changes`, the `cause` of `pool` and `blood`, option keys
complete and `decided`'s `source` and `purpose`, additions only;
`action_ends` now told for every action (one within a message too),
every event in the order it happened, `resolved` without what its events
tell, and undo against bots, behaviours with no field or id; the
table's state and every card's moves told by events of their own.

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
- `text`: markers told (`Marker::text`, from their data); a strike from a
  card named by it in the combat view and the `strike_declared` and
  `strikes` events; every purpose prompted (a draw alternative, a
  margin, Heart of Nizchetus's draws and bottom among those that had
  none) and its pass told by what declining does; a minion phase's
  prompt tells what its minions owe first (`due`). Behaviour, no field.
- Event `acting_changes` (`seat`, `acting`, `minion`): another minion
  now acting in the action under way (Mask of a Thousand Faces); its
  `action_ends` names them (it had ended an action no event announced).
- A banished created vampire's uncontrolled slot: viewed and digested by
  its library card (the view and the digest had failed on it, §11's
  definition otherwise unchanged).
- The table's state told (§9): the events `phase`, `transfers`,
  `locked`, `counters` (on a card in play, or an uncontrolled vampire by
  its `slot`, with its `cause`), `controller` (a minion's or a card's),
  `capacity`, `title`, `markers` and `traits`, and `action_state`, `combat_state` and `referendum_state`
  (each the view's own, told when it changed since the last view): a
  view's state no longer changes untold between two decisions (a lock,
  the phase, a transfer's counters had no event). The view's
  `transfers` stand during a window the influence phase opens (another
  seat's impulse had shown none); the view's `phase` at a found
  equipment's cost terms (Ravnos Cache, Vast Wealth, Filchware's Pawn
  Shop) is the minion phase (it had shown `over`).
- Every card's moves told (§9): the event `card_moves` (`card`, a back
  where neither place shows it; `from`, `to`: `ZoneRef {seat, zone,
  on?}`), for every move between places, in play included (a draw, a
  discard, a play into limbo, entering or leaving play, a card on a
  minion moved, a card or minion changing hands, a merge); the copy a
  play took told as it goes into limbo. `Zone`: `stored`, `training`.
  The view's `training[]`: a seat's Web of Knives Recruits in training
  (they had been in no part of the view). §3 says a card out of sight
  and back between two views keeps its number (it did, unsaid).
- `undo` against bots (one human seat: every server game) is open at
  every decision after the seat's first, one human decision back each
  time, across the bots' decisions, vote orders (`vote_all`, the order
  taken back with its decision), passes, reveals, looks and searches, as
  far as 64 of the seat's own decisions; the bots answer again as they
  did; the view as first sent at the decision undone to, the numbers
  given since never given again (§3: they had been handed out again,
  to other cards).

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
