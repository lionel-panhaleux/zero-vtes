# Protocol changelog

Every `PROTOCOL_VERSION` of [`docs/protocol.md`](docs/protocol.md), semver
as its §14 says what a minor and a major are. `GET /version` says which the
server runs.
`python3 scripts/schema_compat.py` lists what a schema change adds or
removes; its `--check` fails a removal without a major version.

## 2.29.0

- Vidal Jarbeaux adds the `OptionKind` id `meet_as`: on a requirement
  listing alternatives ("prince or justicar", a clan list, "titled"), his
  Methuselah chooses which he meets as the card is played (`minion`, the
  alternative its `value`), the decision's `about.detail` `meet_as`; a
  "non-" requirement (non-titled, non-Red List, non-sterile) he meets too,
  once each. No id removed.
- First Tradition: The Masquerade adds the `OptionKind` id `skip_turn` and
  the `Purpose` id `may_skip_turn`: as a turn begins, its Methuselah skips
  it entirely (`skip_turn`) or plays it (`pass`). National Guard Support's
  counter votes are `in_play` votes of its card. No id removed.
- Scrounging, The Becoming and Will of the Council add the `Purpose` ids
  `looked_to_hand` (the top cards looked at moved to hand, `find_card`,
  `about.m` more), `masters_from_hand` (master Discipline cards from hand
  put on the new vampire, `take_from`, `about.n` more) and `may_take_title`
  (burn the card, `resolve_rider`, for its bearer to take the torpid
  prince's title, `about.detail`). Anima Gathering's stay-locked choice is
  `may_unlock`; The Summoning's recruit is `ally_search`, its prompt no
  longer mage-only. No id removed.
- Keepers of Tradition's masters add the `Purpose` ids `may_take_edge`
  (Instability: the prey takes the uncontrolled Edge, `choose_seat`, or
  `pass`), `torpor_massacre` (White Nights Massacre: one torpid vampire, or
  all by locking a werewolf ally, `choose_minion`, or discarding a copy,
  `discard`), `take_counters_up_to` (Bounty: 0 to `about.n` counters from
  `card`, `set_x`), `guess_number` (Malkavian Prank: the number
  `about.seat` holds, `set_x`) and `bid` (Hostile Takeover: a bid for
  `minion` above `about.seat`'s `about.n`, `set_x`, or `pass`). Special
  Report's pool for intercept is `burn_pool` in the block window. No id
  removed.
- Day Operation adds the `OptionKind` id `torpor_after_resolution`: after
  resolution, the acting vampire goes to torpor, its `actor` the acting minion,
  ordered by the acting Methuselah among the window's effects beside
  `take_damage_after_resolution` (no `pass` while either is due). Coterie
  Tactics adds the `minion_of` detail `joint_blocker`: of two vampires of a
  successful joint block, the one to be the blocking vampire (`choose_minion`,
  no `pass`); the effect `block` on the minion chosen. The action view adds the
  optional `joint_blocker`: the second vampire of a joint block attempt, or of a
  joint block the one not picked. Bundi's prevention is a `prevent` effect of
  detail `melee_weapon_strike`. No id removed.
- A wake (a wake card, Gracetius) is offered to the other Methuselahs after
  combat and after resolution too (LSJ 20091123), for the reactions played
  there; a record before 2.29.0 reads past it. No id added or removed.
- A card canceled as it is played is no longer offered for canceling again
  (a cancelling ability, a cancel card, a pay-to-cancel): the as-played
  window after a cancel may elide.
- Charming Lobby adds the `Purpose` id `named_x`: the X of the political
  action card the action named, chosen as the action succeeds (`set_x`, 0
  up to `minion`'s blood; My Kin Against the World). Its terms name that
  card with the existing `employ_from_hand` option (the card and its mode,
  shown to its player only), or a political action a card in play allows
  with the existing `use_ability` option. No id removed.
- Minor Boon: a vampire about to go to torpor that another Methuselah may
  save is asked of them in turn (`resolution_choice`, `pass` or the card's
  `play` at that vampire), in combat as the combat ends; no new id.

## 2.28.0

- The `Purpose` id `burn_order`: minions burned or sent to torpor
  together under Tension in the Ranks, the acting Methuselah sets the
  order in which their controllers burn pool, one `choose_seat` at a
  time, asked only when two Methuselahs or more would burn their last
  pool. No id removed.

## 2.27.0

- The effect `detail` `all_boons`: a `burn` of every boon in play, any
  Methuselah's (Blood Trade). No id removed.

## 2.26.0

- Round 28's cards (Heirs to the Blood) add two `Purpose` ids:
  `replace_from_research` (Development: its replacement taken from the
  research area, `find_card`, or drawn, `draw_library`) and
  `placed_discipline` (a master: Discipline put on a vampire rather than
  played, Agent of Power by Childe of the Revolution's search: its
  Discipline, `choose_discipline`). No id removed.
- Research adds the seat view's `research` (its face-down research area:
  the cards to the seat, backs to the others; omitted while empty), the
  `Zone` id `research` (a card's move into it, Development's take from
  it) and the `Purpose` id `research_search` (a library card to the
  research area, `find_card`, `about.n` more at most, or `pass`).
- Off Kilter and Scarlet Lore add two `Purpose` ids:
  `may_burn_edge_for_pool` (the Edge burned for `about.n` pool,
  `resolve_printed`, or kept, `pass`) and `library_to_top` (a library
  card, revealed, to the library's top once it is shuffled, `find_card`,
  or none, `pass`). Shatter the Gate adds the seat view's `shattered`
  (the pool its shattered counter burns each unlock phase; omitted with
  none).
- Dagger adds two `Purpose` ids: `second_copy_onto` (equipped from the
  hand, another copy of `card` from the hand or ash heap onto `minion`,
  `take_from`, or none, `pass`) and `return_before_burn` (Dagger out of
  play as the action ends with Cleave's burn due: the active Methuselah
  returns it first, `resolve_rider`, the burn then reaching it, or burns
  first, `pass`, the card spared).
- Byzar adds the `MarkerKind` id `burn_counters` (a minion's burn
  counters, `n`).
- La Viuda Blanca and Grotesque add two `Purpose` ids: `draw_up_to`
  (how many cards the action draws, 1 up to `about.n`, `set_x`) and
  `bind_location` (Grotesque entering play: the location bound to
  `minion`, `choose_card`).
- Lord Ephraim Wainwright adds the `Purpose` id `may_mill_copy` (another
  copy of `card`, the action card `minion` just played, from the library
  to the ash heap, `resolve_printed`, or none, `pass`).
- Slag: a `looked` event (no `zone`) may carry only the replacements
  `seat` drew during a referendum, not its whole hand. Troglodytia's look
  at a hand after a master card is a `looked` event as other looks.
- Blissful Agony adds the `Purpose` id `redirect_first` (played by both
  combatants: the acting Methuselah picks the striker whose resolves
  first, `choose_minion`, the other lost).
- Darkness Within adds the `Purpose` id `drain_before_steals` (beside a
  steal strike: the acting Methuselah has the card's drain first,
  `resolve_rider`, or the steals first, `pass`).
- Pressing Flesh: an imbued raised as a zombie is a Minion of `kind`
  `ally` whose `card` is its crypt card.
- The action step `diablerie` is listed in §6; a diablerie in combat
  (Amaranth's, Liquefy [VIC]'s) holds that step and its blood hunt with
  `combat` still set, the combat resuming after (rules-qa 113.10).

## 2.25.0

- Round 27's cards (Secrets of New York, Noites Brasileiras) add four
  `Purpose` ids: `may_bottom_top` (St. Patrick's Cathedral: the reordered
  top cards to the library's bottom, `resolve_rider`, or pass),
  `ash_to_library_bottom` (Academia de Campeões: `move` to the library's
  bottom, a pass stopping), `crypt_top_pick` (Porto de Santos: one of the
  crypt's top cards to the uncontrolled region, `about.n` how many) and
  `store_up_to` (Aaron Fletcher: an equipment card of the hand, then of
  the ash heap, put face down on him, a pass going on, `about.n` how many
  more). Three effect `detail`s: `aggravated` (a prevention of aggravated
  damage only: O Encourado), `if_passed` (a draw if the referendum
  passes: Deadly Secrets) and `to_controller` (a pool cost paid to the
  card's controller: Convento da Penha). No id removed.

## 2.24.2

- A move of a vampire's own blood to an uncontrolled vampire (Zayyat,
  The Sandstorm; Casino Reeds) is told: `blood`, `n` the amount, `on`
  `uncontrolled`, `detail` `from_host`, so each count's ability is told
  apart by its `apart`.
  Casino Reeds' move is three abilities, one a count (1, 2, 3; rules-qa
  111.51), as Zayyat's: a record using his move before 2.24.2 replays it
  as a move of 1. Zayyat's stealth ability moved from index 1 to 3 (his
  counts are 0 to 2): a record that used it before 2.24.2 no longer
  matches its key and replays only up to it.
- A granted political action's printed pool (War of Ages' "costs 1
  pool") is paid as it resolves (rules-qa 52.10): a record holding War of
  Ages' burn referendum before 2.24.2 replays up to it only. A game live
  across the redeploy whose record holds one of these ends
  `server_restart`.

## 2.24.1

- `opportunities` off (§8): a wake in a referendum is shown when the woken
  minion could then play a card there (Delaying Tactics); other wakes there
  stay hidden (the owner, 2026-11-08). No id changed.

## 2.24.0

- `GET /catalog/decks`: the catalog's decks in its order, each with what
  it is: its `kind` (`precon`, `starter` or `twda`), its own `title`, and a
  tournament winner's `twda` id, `event` and `date` (§10; `CatalogDeck`,
  `DeckKind`). `GET /catalog` keeps its `[name, list]` pairs. Ids added,
  none removed.

## 2.23.0

Round 26's cards (rules-qa Batch 111); ids added, none removed:

- `Purpose` `burn_from_library_top` (Tapestry of Blood): one of the top `n`
  cards of `target`'s Methuselah's library burned (`find_card`), or none;
  `crypt_to_ready` (Virolax Facility): a vampire of capacity below `n`
  from the crypt to the ready region (`ash_vampire`), or none;
  `steal_from_slot` (San Lorenzo de El Escorial): the uncontrolled minion
  of `target`'s Methuselah's region blood is stolen from (`slot`);
  `blood_for_votes` (Mob Rule): burn 1 blood of a ready vampire for votes
  (`choose_minion`), or pass, around the table; `store_from_ash_of` (the
  Holds): the card of `target`'s Methuselah's ash heap moved onto `card`
  (`find_card`); `blood_to_card_up_to` (Cold Amber's Hold): 1 up to `n`
  blood onto `card` (`set_x`); `may_lock_card_for_block` (Elysium:
  Sforzesco Castle): lock `card` instead of the blocking `minion`
  (`resolve_rider`), or pass; `blood_or_pool` (Afifa, The Herald): a ready
  vampire burns 1 blood (`choose_minion`), or 1 pool (`burn_pool`).
- `DueKind` `bleed` (Chain of Command): a vampire that must bleed before
  the minion phase ends.
- The title id `camarilla`: a unique Camarilla title named in `of`
  (Karsh's `Imperator`, merged), with its votes.
- A minion's equipment rearranged (Nod) is described with the detail
  `rearrange`.

## 2.22.0

- `GET /catalog/taught`: the decks the served bot plays worst, its
  weights' measure by deck from the training line, the lowest VP share
  first, each with its catalog list, its own title, TWDA id, event and
  date (§10; `TaughtDecks`, `TaughtDeck`); 404 while the served weights
  have none. Ids added, none removed.

## 2.21.1

- A card played in another card's as-played window (a wake, a cancel) is
  replaced as that window closes, not as it resolves: nothing is drawn
  there (rules-qa 110.1). A record whose wake was played there before
  2.21.1 replays up to the next decision only: a game live across the
  redeploy whose record holds one ends `server_restart`.

## 2.21.0

Round 25's cards (rules-qa Batch 109); ids added, none removed:

- `Purpose` `may_move_blood` (Bamba): move `n` blood from `minion` to the new
  vampire `target` (`resolve_printed`), or pass; `may_fail_action` (Familial
  Bond): the blocking Laibon failed, make `minion`'s action fail
  (`resolve_printed`), or pass; `may_lock_to_fight` (Familial Bond): `minion`
  locks to enter combat with the acting `to` (`resolve_printed`), or pass;
  `search_onto` (Nkishi, Umdava): an Aye or an Orun from library or ash heap
  onto `minion` (`take_from`), or pass; `hand_card_away` (Aisata Swanou): a
  hand card to the ash heap (`discard`) or the library's top (`take_from`);
  `exchange_looked` (Jara Drory): burn 1 blood to exchange the card looked at
  (`find_card`), or pass; `may_burn_for_block` (Lucas Halton): burn 1 blood
  instead of locking for the block (`resolve_printed`), or pass and lock;
  `may_bottom_looked` (Kurt Densch): burn `n` blood to move the card looked
  at to the library's bottom (`resolve_printed`), or pass;
  `may_return_discards` (Rachel Brandywine): the discard phase's discards
  shuffled into the library (`resolve_printed`), or pass;
  `burn_over_capacity` (Orun, Aye): a Laibon holding more of them than its
  capacity, its controller picks which kind burns (`burn_in_play`).
- A contested card another seat never showed is named by its id
  (`Pay to keep contesting …`).

## 2.20.3

- `opportunities: false` hides a reaction's unlock (Deep Ecology, Guard
  Dogs, Rat's Warning, Forced Vigilance) wherever it hides a wake: outside
  an action that concerns the seat, outside its own block moment, once
  blocks are declined but on a bleed at the seat, in a referendum (§8).

## 2.20.2

- An action ends (`action_ends`, `ousted`) once its Methuselah is ousted
  mid-way (Alastor's equipment paid with the last pool after the
  referendum) and the effects under way have settled, no longer when the
  turn passes: `acting` no longer shows `4294967295`, a minion gone with
  its seat.

## 2.20.1

- `award_trophy` offers each trophy by zone, `take_from` (`zone`, `card`)
  in place of `find_card`: the player picks hand, library or ash heap;
  only a library searched is reshuffled (a pick from hand no longer
  reshuffles it). A 2.20.0 record answering `award_trophy` with
  `find_card` replays up to that answer only: a game live across the
  redeploy whose record holds one ends `server_restart`.

## 2.20.0

Round 24's cards (rules-qa Batch 107); ids added, none removed:

- `Purpose` `burn_blood_to_anarch` (Jack Drake): the vampire just put in play
  may burn a blood to become Anarch (`choose_minion`), or pass;
  `swap_equip_from_prey` (Topaz): an equipment of the prey's hand instead of
  the one being equipped (`find_card`), or pass to equip it; `may_join_combat`
  (Marie-Pierre): `minion` locks to enter combat with the opposing minion
  (`choose_minion`), or pass; `may_new_combat` (Haven Hunting): the card
  burned for another combat (`resolve_rider`), or pass;
  `gain_victim_discipline` (Cloak of Blood): `choose_discipline`, or pass;
  `burn_or_fail` (Detect Authority, Drink the Blood of Ahriman): `minion`
  burns blood (`burn_blood_to_avoid`) or, passing, the action fails or `card`
  burns; `prey_slot_half` (The World's a Canvas): `slot`; `choose_card_circle`
  (Achilles' Heel): `choose_circle`; `award_trophy` (the Red List rule):
  `find_card` (`take_from` from 2.20.1), `resolve_rider`, or pass;
  `take_chosen` (Trophy: Chosen): `resolve_rider`, or pass to burn it;
  `move_clot` (Blade Clot): `resolve_rider`, or pass; `clot_fate`: `minion`
  burns (`choose_minion`) or goes to torpor (pass).
- `minion_of` details `blind_spot`, `lock_other_anarch`, `burn_ally_up_to`
  (`n` the bleed), `heal_host` and `move_onto_any`.
- `may_draw_extra`: `about` names what brings the draw, a card (Learjet) or a
  minion (Ira Rivers, Duality), and may be asked twice for one card.

## 2.19.0

Round 23's cards (rules-qa Batch 106); ids added, none removed:

- `Purpose` `retake_from_ash` (Steely Tenacity): `seat` replaces the card
  by taking it back from their ash heap (`find_card`) or by drawing
  (`draw_library`); `may_lock_for_blood` (Conniver): `seat` may lock the
  card for 1 blood from the bank on its vampire (`resolve_rider`), or pass;
  `may_burn_looked_master` (Weirding Stone): the master card just looked at
  on the active Methuselah's library burned for `n` blood of the card's
  bearer (`resolve_rider`), or pass.
- `store_own_hand` asked for Gift of Proteus too: its announced number one
  at a time (`about.n` left), not replaced.
- `looked` with `zone: library` may be another seat's library (Weirding
  Stone: the active Methuselah's), its `seat` that library's; a play from
  another Methuselah's ash heap (Vestgeir) names that heap in its `zone`.

## 2.18.0

The events miss no view path (the deep sweep's seven, engine board); an
event added, none removed:

- `hand` (`seat`, `open`, `cards`): a hand as the viewer sees it now,
  told as it turns face up or down for them (an open hand: Aura Reading,
  Revelations; a look), after its step's moves; while open, a card
  moving into it is told with its face. Its faces were the views' alone
  before.
- A minion's `owner` (the Methuselah whose ash heap it goes to, when
  not its controller): an ally stolen for good (Entrancement) and a
  minion under temporary control show it; neither did.
- A minion out of play in a contest has no `capacity` in the views (its
  title neither, as before); told again as it comes back.
- A card burned to an earlier ousted owner's ash heap (Disarm) is told
  moving there (`card_moves`): gone from the table (§9); it was told
  leaving play only.
- Call the Great Beast become a vampire is told as a move from its
  vampire to the ready region, after its `enters_play`; it was told
  leaving play.

## 2.17.1

An undo is recorded before the decisions the game takes on from it, its
`to` the decision undone to (§11: the numbers repeat from it), the
`undone` event's `to` likewise. They differed when the game took the
decision undone to itself (a setting changed since passing it, then the
bots): the undo was recorded after those decisions, `to` the next pending
one, and such a record neither replayed nor resumed. No field or id
changed.

## 2.17.0

Round 21's cards (rules-qa Batch 102); ids added, none removed:

- `Purpose` `ash_to_library_top` (Unity): `n` more cards requiring an
  imbued moved one at a time from `seat`'s ash heap to the top of their
  library (`find_card`); `cut_library`: a cut of `seat`'s library, `set_x`
  the cards moved from the top to the bottom, a pass none;
  `x_convictions`: a found or named "[X CONVICTION]" equipment's X paid a
  conviction at a time (`burn_in_play`), a pass stopping (Living Wood
  Staff through Moise Kasavubu or Angel of Berlin).
- Details: a terms window's `x_convictions` (X paid in convictions: Living
  Wood Staff); an effect's `leaves_incapacitated` (Bond: the target leaves
  the incapacitated region).
- A `library_search` window may offer `employ_from_hand`: an equipment
  from the hand instead of the library (Moise Kasavubu).

## 2.16.2

With `opportunities` off, a wake in a card's `card_as_played` window is
shown when the woken minion could then cancel that card (Rewind Time,
Touch of Clarity: 2.16.0's play), hidden otherwise as before (the owner,
2026-10-25). No field or id changed.

## 2.16.0

The ledger's gaps (rules-qa 96); ids added, none removed:

- `Purpose` `reduction_at_resolution`: Nergal's action card, announced
  without his reduction and unpayable without it as it resolves — he may
  take it (`resolve_printed`, or pass); `minion` is Nergal.
- `Purpose` `dual_form_kept`: several of a Dual Form set left the ready
  region at once — its controller chooses the one kept (`choose_minion`),
  the others burned; `minion` is the set's original.
- A locked minion's wake (On the Qui Vive, Eyes of Argus, Wake with
  Evening's Freshness) is offered in another Methuselah's action card's
  `card_as_played` window, before its action is declared, and lets it
  cancel the card there (Rewind Time, Touch of Clarity). Replays of
  records before 2.16.0 read these three as not offered.

## 2.15.1

`ash_vampire`'s `zone` names the ash heap the vampire is taken from,
another seat's too (superior Shroud of Decay: the prey's), as §7 says; it
named the chooser's own (the owner's playtest, 2026-10-25). No field or
id changed.

## 2.15.0

A combat's windows ask every Methuselah (rules-qa 98): the acting
Methuselah, the defending one, then the others clockwise from the acting
one, a Methuselah outside the combat offered what reaches a combat it is
not in (Rötschreck, Abjure, Maman Boumba, its out-of-turn and "any time"
effects). No field or id added or removed; a reader of a record from
before 2.15.0 passes those windows unread.

## 2.14.1

`GET /catalog?seats=2` lists VEKN's Tzimisce two-player decklist too: its
"Nesserian" is Neserian (in the pool, on the variant's list), read by an
alias; 18 lists. No field or id changed.

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
