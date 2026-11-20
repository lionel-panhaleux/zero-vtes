# zero-vtes — Engine guide

## 1. Purpose and use

The engine guide for implementers adding cards and reviewers checking a
diff: how a definition becomes options and effects, what the compiler and
the tests enforce, what is still convention, how to add a card, where
comments go, and where each mechanism lives with the modeling calls not
obvious from the code or the rulebook. Decisions, phases and open gaps stay
in [`DESIGN.md`](../DESIGN.md) (§4 the architecture, §10 the modeling
decisions, §11 the slices, gaps and what remains); rules answers live in
[`docs/rules-qa.md`](rules-qa.md), cited here as `rules-qa N.M` pointers
only; "spec §x" is
[vtes-advanced-rules](https://github.com/vtes-biased/vtes-advanced-rules).
Sections and their `###` headings are stable: cite them as
"docs/engine.md §6.5 Actions". Paths are under `crates/engine/src` unless
said otherwise.

## 2. Dispatch architecture

**Decision requests.** `Engine::run` (`game.rs`) settles what needs no
choice (interrupts, a deferred oust, draw-up, excess blood, hostages), then
asks `Engine::request`: an `Interrupt` first (`draws.rs`: a choice owed by
one player in the middle of a resolution), then a waiting oust
(`ousting.rs`), a discard-down, then the current `Flow`. A request is a
`DecisionRequest { seat, window, options: Vec<Choice> }` (`decision.rs`),
each option listed once. A request with one option is applied at once and
recorded, not logged (window elision, DESIGN §4.3); otherwise `run` returns
`Step::Decision`. `Engine::choose(idx)` logs the index and applies the
`Choice` (`Engine::apply`). A game is `(seed, log)`. `Choice` variants and
their order are the policy's action space and the `host`/`text` crates'
input: changing them is not a refactor.

**Windows.** `window.rs`: `Rotation<Ordered>` (impulse and sequencing:
a play resets the impulse to the acting Methuselah) and `Rotation<Polling>`
(commitments, no reset); both close on a full rotation of consecutive
passes (DESIGN §4.2). `Window` (`cards/mod.rs`) is a definition's timing;
`WindowLabel` (`decision.rs`) is the request's.

**Definitions.** One representation per card (DESIGN §4.5):
`cards/defs.rs` (`DEFS`: `CardDef { modes, abilities, riders,
delayed_replacement, … }`), `cards/vampires.rs` (`VAMPIRES`: `VampireDef`,
printed text: stat modifiers, abilities, riders), over the generated static
half (`library.rs`, `crypt.rs`: costs, Disciplines, labels; §6.18). A mode
or an ability is a `ModeDef { windows, actor, requires, target, terms,
limit, effects }`; a rider is `Rider { hook, scope, requires, effects }`.
Options are generated from these, never hand-written:

- card plays: `Engine::play_options` (`plays.rs`) walks the hand's modes
  open in the current windows (actor, Discipline tags, `requires`, target
  filter, costs through layers and potential reductions, limits,
  out-of-turn);
- in-play abilities: `Engine::ability_options` and
  `vampire_ability_options` find each ability's candidate users by
  `Actor`; `ability_gate` checks every candidate alike (§2, gate below);
- riders are not options: they fire at their hook (§2, riders below), and
  the ones a player orders or may decline become pending choices.

**Applying a choice.** A play: `announce_play` → `Flow::PlayTerms`
(terms) → `Flow::CostTerms` (reductions) → `Flow::AsPlayed` (the
as-played window, a nested cancel stack) → `resolve_play`, which runs the
mode's effects one by one through `apply_card_effect`. An action card
declares its action (`Effect::DeclareAction`); its effects resolve as the
action succeeds (`actions.rs` `succeed`). An ability: `use_ability(seat,
…)` → `Flow::AbilityTerms` → `resolve_ability` (`resolve_ability_with`
when the terms named a hand card: Nosferatu Bestial's employ, through
`ability_employ_apply`; `ability_gate` masks the ability with none to
name).

**Effect families** (`cards/families.rs`). `Effect::family` is exhaustive
(no `_` arm): a new variant does not compile until classified. Each family
names its interpreter:

| Family | Interpreter |
|---|---|
| `Play` | `Engine::apply_card_effect`'s own arms (`plays.rs`) |
| `Shared` | `Engine::apply_shared_effect`: the same body from a play and an ability (`By::Play` / `By::Ability` where the reading differs) |
| `Ability` | `Engine::resolve_ability`'s own arms, costs included |
| `Action` | `Engine::apply_action_effect` (`actions.rs`), an action under way |
| `Combat` | `Engine::apply_combat_effect` (`combat.rs`) |
| `Vote` | `Engine::apply_vote_effect` (`referendum.rs`) |
| `Success` | `Engine::succeed`: an action card's, a granted action's, a successful bleed's (`bleed_successful_effects`) |
| `Entry` | `Engine::apply_entry_effects`: an ally or retainer entering play |
| `Rider` | the loop firing its hook only |
| `Marker` | read by name where offered, declared or played (`effects.contains(&Effect::X)`); the interpreters pass it |

`apply_card_effect` and `resolve_ability` end in one `match e.family()`;
a family that cannot reach them is `unreachable!`. A granted action's
success (`succeed`'s `ActionKind::Ability`) dispatches on
`Effect::granted` instead (`Granted`: passed, its own arm, the play's or
the shared body): an effect it does not list cannot sit there. An
`Action` effect from an ability needs an action under way: such
abilities are offered in an action's windows only (tested, §3).

**Requirement slots** (`cards/slots.rs`). `Requirement::slots` (exhaustive)
names the evaluators that really read each requirement: `holds` (`Slot::Play`:
the play's context), `holds_for_host`, `holds_for_card`, the target
evaluators (`holds_for_minion`, `holds_for_uncontrolled`, `holds_for_seat`,
`holds_for_card_target`), `GameState::printed_holds` (`Slot::Printed`),
`rider_holds`' own arms (`Slot::Rider`), or engine code reading it by name
(`Slot::Engine(Site)`: `play_options`, `ability_gate`, `ability_holds`,
`as_played_request`, a hook's rider site, …). Any other evaluator passes it
as `true`. A requirement belongs in a position whose evaluators meet its
slots. `implied_requirement(payer, cost)`: the requirement an ability cost
implies (`LockSelf` → `CardUnlocked`, `LockHost` → `HostUnlocked`,
`HostBurnBlood`/`HostBurnLife(n)` → `HostBloodAtLeast(n)`, or
`ActorBloodAtLeast(n)` where the user pays); abilities only, a rider burns
what there is.

**Riders.** `Engine::fired_riders(hook, &HookAt)` (`plays.rs`) is the one
collector of card riders: present, text not lost, on the cards
`HookAt::on` names, scope read against `HookAt::seat`, requirements by
`rider_holds` — or in the action's context at a `Hook::reads_action` hook
(`HookAt::acting`). `rider_fires` is its per-rider test, read again as a
rider chosen earlier resolves. `Engine::printed_riders(m, hook, seat)` is
the vampire counterpart. Each site keeps its own order and batching;
`apply_rider_effect` resolves the effects whose body is the same at every
hook (`BurnSelf`, `AddCounter`, `RemoveCounter`, `BurnSelfWithoutCounters`,
`BurnSelfWithoutStored`) and owns the "not a <hook> rider effect" panic.
§6.11 has the semantics.

**Ability legality.** `ability_user(source, ability, seat, user) ->
AbilityUser { seat, host, actor }` (`plays.rs`) is the one answer to who
uses an ability, read by the enumeration, `resolve_ability`'s header and
every terms request; `use_ability` takes the seat that chose it.
`ability_gate(seat, window, source, ability, user)` holds every check: the
user's seat, phase requirements and phase uses, the limit ledger, cost bars
(an ally pays no blood; `cards::plays_card_as_vampire`), each terms kind
having an option (cards to discard: `GameState::discard_terms_met`; an
equipment or retainer from hand; an ash heap card), `cb.allows_from`,
granted-action checks (`may_take_granted`, the turn's political action,
Delaying Tactics, a referendum feasible), then `ability_holds` (the
requirements read by name, the card's, host's and play's evaluators).
`limit_slot(limit, source, ability)` maps a `Limit` to its `Ledger`
(combat, round, exchange, action, referendum, turn, game:
`GameState::game_uses`), key and allowed uses, read by `limit_spent` and
written by `mark_limit`.
`Engine::gains_nothing` tells a legal no-op for interfaces that hide it
(rules-qa 17.4: the engine still offers it); `Engine::ability_used_by`
tells interfaces who uses an offered ability (the seat, and the minion when
it is the user's own); `Engine::dues` what a minion phase withholding its
end waits for (the hunts due, read from the same `starving` the options are
built from; the mandatory actions; Puppet Master).

**Placement.** "Put this card on … / in play": `Engine::place_now(card,
ModeLayers, one_per_vampire)` as the card resolves (the card itself enters
play; a title card's title; "a vampire can have only one" as a legal no-op
through `GameState::holds_its_name`). `ModeLayers::FromEffects`: the mode's
own `Effect::Layer`s follow the placement in the effect loop;
`ModeLayers::Push`: no loop runs them (a referendum's card, Pillars Fall
after combat). `place_held` (`combat.rs`) only for a card already in the
ash heap (after combat, on a hit): an effect-only stand-in
(`CardInPlay::effect_only`; rules-qa 21.1, 47.1). `enter_play` is the
primitive under both.

**Held and Release.** `state::Release` names the moments held replacements are
drawn; `Held` is `[u8; Release::COUNT]` indexed by it (`Seat::held`;
`Seat::held_draws`: the extra draws waiting, by source and card). `Until` is
the definitions' vocabulary (`CardDef::delayed_replacement`); `Until::release`
maps it to a `Release`, `LeavesPlay` to none (`Seat::held_in_play`: a list of
cards). `Weakens`, `MasterPhase`, `MinionPhaseEnd` are engine delays. Every
release goes through `Engine::replace_held` (`release_held` loops it over the
seats).

**Timed seat modifiers.** `Seat::timed` (`state::Timed`): an amount per
`TimedStat` (`HandSize`, `HuntStealth`, `ExtraDraws`) until each `Expiry`
(`EndOfTurn`, `DiscardPhase`, `UnlockPhase`, `MasterPhase`, `Combat`, `Round`,
`Never`), each swept at one line (`Timed::expire`): `DiscardPhase` in
`end_influence` after the held discard-phase replacements, `EndOfTurn` in
`end_turn` after temporary control ends (every seat's: Shahid's, gained in
another's turn), `UnlockPhase` in `begin_turn` before the held unlock-phase
replacements, `MasterPhase` in `start_master_phase`, `Combat` as the combat
ends. Other shapes stay fields (`combat_open_hand`, `no_transfers_next`,
`transfers_next`, `mpa_next`, `veil_left`).

**Timed minion bonuses.** `Minion::timed`: `(Lasting, Expiry, SeatId)`
entries — `Lasting::{Stealth, Strength, Superior(discipline), Bleed,
AggravatedHands, UnlockAtEndOfTurn, Votes}` — read by `GameState::lasting`
(stealth, strength), `lasting_bleed` (Andre LeRoux's bleed this turn),
`lasting_votes` (Political Struggle's, `Expiry::Never`, summed into both
vote formulas), the Discipline level and the combat's hand strike damage
(aggravated), swept beside `Seat::timed`: `EndOfTurn` in `next_turn`,
`UnlockPhase` in `begin_turn` and `MasterPhase` in `start_master_phase` for the
seat recorded (the controller's, "until your unlock phase", Camarilla Vitae
Slave's "until their next master phase"); `Round` in `new_round` (Patagia's
flight, a superior Discipline for the round), `Round` and `Combat` for
every minion in `GameState::leave_combat`, which every combat end passes
through (a Psyche! or Coordinate Attacks combat next included). Not `Timed` (a numeric table,
and a Discipline is not a number) nor a layer (`Scope` has no turn
scope). Dive into Madness [dem], Gear Up. An `UnlockAtEndOfTurn` entry
(`GameState::unlock_at_end_of_turn`: Enkidu, The Noah's employ) unlocks its
minion as `next_turn` sweeps it.

**Layers.** `Layer { scope: InPlay | Action, modifier, source }` in
`GameState::layers`, pushed by `Effect::Layer` / `Effect::ActionLayer` and
`push_mode_layers`; `GameState::layer_active` suspends a contested card's.
Stats are read live through them (`in_play_layers`, `global`). Printed
vampire text is not layered: `GameState::printed_mod` reads it live
(§6.12).

## 3. Enforced and conventional

**Enforced by the compiler.** `Effect::family` and `Requirement::slots`
are exhaustive; `Effect::prevents` is classified in `family`'s own match
(the one prevention class, never a list by hand; the host's
`every_prevention_is_told` fails for one it has no arm for);
`Release`, `TimedStat`, `Expiry` are enums indexing their
tables; `limit_slot` matches every `Limit`.

**Enforced by tests over every definition** (unit tests in the engine's
`--lib`, the rest under `crates/engine/tests/`):

- `every_effect_sits_where_its_family_resolves` (`cards/families.rs`): each
  effect of `DEFS`/`VAMPIRES` sits in a position whose interpreter resolves
  its family (a mode, an action card's mode, an ability, a rider, an `If`
  bonus, an `IfBleedSuccessful` body); a granted action's, what
  `Effect::granted` lists, or for another kind what its resolver,
  stealth or declaration reads by name (the test's list by kind);
- `abilities_changing_the_action_are_used_in_one` (same file): an
  ability's `Action` effect is offered in an action's windows only (a
  wake outside one has its own arm: Gracetius);
- `every_requirement_sits_where_it_is_read` (`cards/slots.rs`): each
  requirement of every mode, ability (by actor), rider (by hook), target
  filter, printed modifier, `If`, term choice and `AnyOf` sits where an
  evaluator reads it; an unclassified actor or hook fails closed;
  `KNOWN_GAPS` is empty (an entry nothing matches fails as stale);
- `every_ability_cost_carries_its_requirement` (same file): every ability
  cost carries its `implied_requirement`;
- `every_card_put_on_the_opponent_needs_it_present` (same file): a mode
  with `PutOnOpponent` carries `OpponentPresent` (rules-qa 75.1);
- `no_reduction_waits_for_a_need` (same file): a mode or ability reducing
  stealth or intercept requires no `StealthNeeded` / `InterceptNeeded`
  unless it adds some too (LSJ 20020612; rules-qa 3.2);
- `every_card_is_defined_and_its_modes_match_the_reviewed_windows`
  (`tests/plays.rs`): every library card defined, every vampire definition
  resolving to its crypt card, the modes' windows equal to
  `data/window-matrix-reviewed.csv`;
- `plain_stat_halves_apply_and_every_vampire_has_all_its_text`
  (`tests/vampires.rs`): no `incomplete` left (rules-qa 13.1);
- `legal_no_ops_are_offered_and_known_to_gain_nothing` (`tests/precon.rs`).
- `no_card_name_in_engine_code` (`tests/lint.rs`): no card name as a string
  in hand-written engine code (`crates/env`'s included) outside the
  definitions; each exception in `CARD_NAMES_ALLOWED` carries its reason,
  a stale one fails.
- `pool_and_blood_go_down_through_their_primitives` (`tests/lint.rs`):
  pool and blood are lowered only by `Seat::lose_pool` / `set_pool` and
  `GameState::lose_blood` / `set_blood` / `burn_blood` (a minion leaving
  play: `Minion::clear_blood`; a non-decreasing set: `raise_blood`), the
  one place a loss breaks a withdrawal (§6.17); pool rises only by
  `GameState::gain_pool` (The Rising's bank, §6.16); a blood gain may add
  in place.
- `during_abilities_are_once` (`tests/lint.rs`): a definition whose quoted
  text grants a "during" ability has an ability of its own once in its span
  (a `Limit`, phase windows only, a cost spending the card or its host's
  unlock) or says in `DURING_ONCE_ELSEWHERE` how it is once, a rider of a
  played card included (rules-digest D7); a stale entry fails.
- `circle_requirements_read_angelo_only_on_sanguinus_cards`
  (`tests/twda_round17.rs`): a circle requirement's `sanguinus` flag is
  the card's Sanguinus requirement, never on a vampire, and the circle
  readers without the flag sit on Sanguinus cards only (rules-qa 84.5).

**Enforced by play.** The fuzz fixtures (`tests/fuzz.rs`: invariants,
conservation of copies, replay of every 4th seed, each fixture's `covers`
keys taken by random play) and their goldens (`tests/golden/`, DESIGN
§7.3); perft counts (§7.5); `tests/elision.rs` (elision is observationally
equivalent); `crates/env/tests/obs.rs` and `bots.rs` (nothing read beyond the
legal view); `crates/host/tests/keys.rs` (no choice offered twice, a key
per option); `crates/host/tests/events.rs` (each decision's view the
last one with the events since applied, protocol.md §9). A refactor
keeps the goldens and perft green without `GOLDEN_BLESS`; a bless names
the moved seeds and decision ranges.

**Enforced by scripts.** `scripts/rulings_v5.py --check
data/rulings-v5.csv` (every ledger row on an implemented card backed:
`test:` / `n/a:`, never `blocked:`); `scripts/window_matrix.py --check
data/window-matrix-reviewed.csv`; the
generators' `--check` (`gen_crypt.py`, `gen_library.py`, `gen_twda.py`).

**Still convention** (recurring hazards no check enforces):

- A `Rider`-family effect, or a shared one at a new hook: the families test
  checks its position, not that the hook's site handles it; a site not
  knowing it panics in a game (`apply_rider_effect`'s `unreachable!`).
- A predicate meaningful on printed text: `holds` and `printed_holds` are
  two evaluators; `slots` checks where a requirement sits, not that both
  agree. Shared readings go through one helper (`ActionCtx::is_hunt` /
  `PrintedCtx::is_hunt`, `ActionCtx::is_employ_action` /
  `PrintedCtx::employ_action`, `GameState::ready_named`).
- An engine-checked requirement (`Slot::Engine`) needs its real check in
  the site its slot names; the slot is declared by hand from the arm that
  reads it.
- `Marker` effects are read by name scans; a new marker needs its reader.
- A new seat number "until …" of another shape than `Seat::timed`
  (a flag, a grant consumed later) is a field with its reset at the line
  the card names, cited with its ruling.
- Placement timing: `place_now` vs `place_held` is chosen per effect; the
  test asserts the card in play is not `effect_only` and is out of the ash
  heap (CLAUDE.md).
- "Only one" is checked twice by design: a mask from definition data as
  the card is played (`ActorWithout`, `TargetWithout`), a legal no-op as it
  resolves (`holds_its_name`).
- The `Interrupt` pay-or-suffer family (`PayOrBurn`, `BloodOrBurned`,
  `LockOrPay`, …) is one request/apply pair each: factored only once a
  round brings several more, keeping the `Choice` values.
- `host/src/effects.rs` tells an effect to clients where curated (`add`)
  and names every other one, by its family, in its untold arms: no `_` arm,
  so a new `Effect` does not compile until told or listed untold (owner,
  2026-10-25).
- Card-name literals in engine code: three sites with a reason (Mithraic,
  Anarch Convert, Life Boon) and the Blood Brother circle checks; any other
  name or printed number belongs in `cards/defs.rs`.
- Conditions are read live, never snapshotted (CLAUDE.md "Owner rules"): no
  test can see a snapshot that nothing changes yet.
- Third-party intercept and stealth strips must be counted by
  `block_can_reach` (§6.6), or the host's opportunities filter hides a live
  attempt (the engine offers it) and the network bots never make it, in
  training and on the server alike (`zvtes_policy::reachable_blocks`): no
  game would show the loss. It also stands for the pruning of the
  engines before rules-qa 73.8 when a record of theirs is read
  (`Engine::futile_blocks`, `Replay::elided`): a change to it moves which
  windows such a record reads past, and only `data/records` checks it.

## 4. Adding a card

1. **Read the rulings first** (CLAUDE.md "Owner rules"): the card's KRCG
   rulings, its rows in `data/rulings-v5.csv`, `../vtes-advanced-rules`,
   and rules-qa (its topical index, then the "Default readings": choices
   at declaration; a card another card plays or puts in play is played,
   by that card and not in the normal fashion; costs payable even if the
   payer burns; caps read narrowly; card text addresses its controller,
   32.7; Ⓓ only indicative, 20.1). The design satisfies each ruling. Read
   `docs/rules-digest.md`'s sections for the card's mechanisms first.
2. **Find an existing primitive** before adding a variant. Search
   `Effect` by family: the arms of `Effect::family` in `cards/families.rs`
   list every variant of a family together; grep a sibling card's
   definition in `cards/defs.rs` (sections by slice) for its spelling.
   Search `Requirement` by slot in `cards/slots.rs` (the evaluator the
   position runs). A `Modifier` behind `Effect::Layer` /
   `Effect::ActionLayer` covers most "+n X" text; a timed seat amount is a
   `TimedStat`; a delayed replacement an `Until`. Parameters live in the
   definition, never in which interpreter runs (`CircleBloodMoves {
   equipment }`).
3. **A new variant**: classify it in `Effect::family` / `Requirement::slots`
   with the arm that reads it; implement it in that one interpreter; a
   cost effect gets an `implied_requirement` line; a rider effect is
   handled at its hook's site or in `apply_rider_effect`. A doc of one or
   two lines (§5).
4. **The definition**: printed numbers and names in the definition, not
   engine code; a vampire carries all its text (rules-qa 13.1). The card's
   windows must match `data/window-matrix-reviewed.csv`; a new timing is
   a generator change (`scripts/window_matrix.py`) kept in sync with the
   reviewed matrix. Legal no-ops stay offered; only illegal targets are
   masked (rules-qa 17.4), but for one owner exception: Spiritual
   Guidance and Reality Mirror, offered only where an ash heap card is
   playable (rules-qa 122.215).
5. **Tests**: one file per family under `crates/engine/tests/` (the
   round's file for TWDA rounds); `tests/common` helpers
   (`CardInPlay::new`, `never_offered_until` to prove a non-offer, `plain`
   for a vanilla vampire). Window elision affects naive tests (docs/hazards.md
   "Engine tests"). Futile block attempts are offered (rules-qa 73.8):
   `pick` and `expect` decline a block window offering only attempts that
   cannot reach the stealth on their way (`futile_blocks`; the window the
   engine elided before), `request` returns it; `decline_futile` passes
   them, `never_reaching_until` proves an attempt cannot reach the stealth.
   A bare fixture's seats (empty library and hand) are offered a
   withdrawal each unlock phase: `expect` and `pick` pass a window
   offering only it on the way (`withdrawal_only`), `request` returns it.
   A test in `tests/rulings.rs` backs a ledger row; regressions go
   elsewhere.
6. **Ledger**: each of the card's rows in `data/rulings-v5.csv` (CRLF)
   gets `test:<fn>` or `n/a:<reason>` (a gap is built, never `blocked:`);
   run the `--check`.
7. **Fuzz**: add the card to a fixture's `slice` and a `covers` key
   (`"play:<card>"`, `"ability:<name>"`, …) for what random play must
   reach; `seeds` about twice the seed where the last key first appears
   (at most 300). Adding a card reshuffles the fixture's games: re-bless
   with `GOLDEN_BLESS=1`, the change explained in the commit. Also diff a
   `{:#?}` dump of `DEFS`/`VAMPIRES` for a refactor.
8. **Unsettled readings**: a reading no ruling settles is implemented and
   recorded as a question in a new rules-qa batch; an issue met on the way
   is fixed in the change, or, clearly outside its line, goes through the
   `intake` skill (CLAUDE.md "Findings").
9. **Elsewhere**: a new effect is told in `crates/host/src/effects.rs` or
   named in its untold arms (the host does not compile until it is); a new
   `Choice` kind needs its obs slot (§6.21); the checks of docs/operations.md
   "Checks" before calling it done.

## 5. Comment policy

The owner's policy (2026-10-14, applied to the engine in one pass), for
every crate and script (owner, 2026-10-22). Code comments are limited to:

- (a) the card's printed text on its definition (every vampire ends with
  all its text, CLAUDE.md);
- (b) ruling citations on the code they justify (`rules-qa N.M`, `LSJ
  yyyymmdd`, `ANK yyyymmdd`, `PIB …`, `RTR …`, advanced rules §);
- (c) a one-line "why" where the code cannot say it.

Doc comments on vocabulary variants (`Effect`, `Requirement`, `Hook`,
`Modifier`, `Stat`, …) are one or two lines: what it does, which cards,
the ruling. Mechanism explanations live here, not in the code.

**Before** (`cards/mod.rs`, `Effect::HostBurnBlood`):

```rust
/// The host of this card in play burns n blood: blood for a vampire,
/// life for an ally, an ally left with no life burned (owner, rules-qa
/// 73.5; `GameState::burn_blood_or_life`). A rider's "blood or life"
/// (Phantasmagoria, Sculpt the Flesh), a vampire's "burns 1 blood"
/// (Marialena, Condemnation: Mute); as an ability's cost (Bowl of
/// Convergence [AUS]) a blood cost, which an ally never pays (advanced
/// rules 5.5.1, `ability_gate`): an ally's own "burn n life" is
/// `HostBurnLife`.
HostBurnBlood(u8),
```

**After:**

```rust
/// The host burns n blood, an ally life (Phantasmagoria, Marialena, Bowl
/// of Convergence; rules-qa 73.5; as a cost never an ally's, adv. 5.5.1).
HostBurnBlood(u8),
```

**Before** (`plays.rs`, `Engine::replace`):

```rust
/// `n` cards that left the seat's hand are replaced now (rulebook
/// "Drawing Cards"; spec §6.8.3: every replacement, whatever took the
/// card out of hand). Each replacement is first offered to the
/// `WouldReplace` riders of the cards the seat controls, `except` the
/// card being replaced itself ("a card other than this card"): Visit
/// from the Capuchin burns a counter instead, its hand size shrinking
/// with it. With an empty library nothing is replaced, so nothing is
/// replaced instead (rules-qa 8.3). Beside a draw alternative (Black
/// Market Cache, Shilmulo Tarot) the player picks one treatment per
/// replacement (ANK 20200905; rules-qa 9.4), queued in order. The hand
/// is then drawn up.
```

**After** (the mechanism is §6.14):

```rust
/// Replace `n` cards that left `s`'s hand, `except` this card's own
/// rider (spec §6.8.3; rules-qa 8.3, 9.4; ANK 20200905).
```

**Already right** (`plays.rs`, `limit_slot`): an arm with its ruling
and nothing else.

```rust
// Per copy per action (Haqim's Law: Retribution, ANK 20221102-2);
// once each action or referendum for this ability, whoever uses it
// ("after …", "during …": rules-qa 16.1, 58.7 — Forward Momentum,
// Alamut).
Limit::PerAction => (Ledger::Action, source.action_key(ability), 1),
```

## 6. Reference by mechanism

Where each mechanism lives and the modeling calls not obvious from the
code or the rulebook.

### 6.1 Crate layout

`crates/engine/src`: `game.rs` (`Engine`, `Flow`, run loop, turn
structure, `public_choice`), `window.rs` (primitives), `decision.rs`
(`Choice`, `WindowLabel`, `ActionKind`), `plays.rs` (card plays,
abilities, costs, riders' collector, `apply_card_effect`), `actions.rs`
(`ActionCtx`, block loop, resolution, `succeed`), `combat.rs`
(`CombatCtx`, `Side`), `referendum.rs` (`ReferendumCtx`), `contests.rs`,
`influence.rs`, `draws.rs` (`Interrupt`, draws, searches), `ousting.rs`
(the oust decision point), `state.rs` (`GameState`, `Minion`,
`CardInPlay`, `Seat`, `Held`, `Timed`, live stat reads, `legal_view`),
`copies.rs` (copy identities and their census, §6.22), `profile.rs` (each
seat's public choices counted, `Engine::profiles`), `decks.rs` and
`twda.rs` (decklists), `agent.rs` (the `Agent` trait, `RandomLegal`),
`cards/mod.rs` (the vocabulary: `Window`, `Actor`, `Requirement`,
`Terms`, `Effect`, `Modifier`, `Hook`, `Limit`, `Until`, `CardDef`,
`ModeDef`, `Rider`), `cards/families.rs` (`Effect::family`),
`cards/slots.rs` (`Requirement::slots`, `implied_requirement`),
`cards/defs.rs` and `cards/vampires.rs` (definitions),
`crypt.rs`/`library.rs` (generated). Tests: one file per family under
`crates/engine/tests/` (`precon.rs` holds the slice-11 cards by section).

The engine crate holds the rules only; what sits between it and an agent
is `crates/env` (`zvtes_env`, over the engine's public API): `obs.rs`
(observations), `context.rs` (the seats' context they carry), `bots.rs` (the
scripted bots), `politics.rs` and `habits.rs` (agent behaviour around a
network, not engine masks), their tests in `crates/env/tests/`. `policy`, `py`
and `host` (the scripted bots' profiles) read it; `tests/lint.rs` holds its
sources to the engine's conventions.

### 6.2 Run loop and interrupts

`game.rs`, `draws.rs`.

- `Engine::run`: settle interrupts and a deferred oust, draw up, drain
  excess blood (`drain_all_excess`), release hostages and torture, settle
  a hand size risen, then request; single-option requests auto-apply.
  `Engine::elided_passes`, kept for the record reader only (`None`
  otherwise), notes each lone Pass so (its seat and window): a record's
  engine that offered the seat more there asked it, and its Pass is read
  past.
- Hand size is settled live both ways (rulebook "Drawing Cards": at once):
  discard-down whenever `hand_excess` > 0; draw-up (`settle_draw_up`)
  whenever a hand is below its size with a library to draw from, between
  plays only (no interrupt, ousting, torpor wait, action, combat or
  referendum; a phase flow), so a card played but not yet replaced is never
  drawn for early; a hand size risen draws at once, in an action or a combat
  too, by the rise alone (`settle_hand_rises`, spec §6.8.1: Nar-Sheptha,
  Raptor's opponent as a combat ends). `GameState::hand_load` counts the
  hand, `held` and `held_in_play`.
- A choice owed by a specific player in the middle of a resolution is an
  `Interrupt` (`Engine.interrupts`), served in order before any other
  decision but a Life Boon's (§6.17):
  discard-down, draw alternatives, replacement treatment, entry effects
  (`EntryBurn`), margin distribution, `MinionOf` picks, block tolls, etc. A player
  with nothing to choose resolves at once.
- An ousted seat drops out of open windows (`OrderedWindow::remove`).
- `Window::AnyImpulse` (untimed "you can lock this card" abilities) is offered
  wherever a seat holds the impulse — phases, unlock rotation, action/combat
  windows, polling — never in as-played or terms windows; using one is an effect
  (impulse reset / polling commitment). Hand cards timed so are offered with
  them (`any_impulse_options`: Angel of Berlin, rules-qa 103.6). A starving
  vampire's forced hunt is offered beside untimed abilities, not instead.
- Settle steps run beside the interrupts: `settle_bound_burns` (a card in play
  with `Modifier::SectBound` burns once its host's live sect is another: Field
  Training, The Red Question; `InfernalBound` once it is not infernal:
  Maleficia; `BurnAtMethuselahCounters` once its counters reach the
  Methuselahs in the game, an oust counting: Strained Vitae Supply; the
  other burns a live state calls for: The Becoming, Anima Gathering,
  Martinelli's Ring, Lynn Thompson's havens), `settle_moving_titles` (Osebo
  Kholo's standing move, marked by `Effect::MovesToOldestUntitled`; a tie among
  the oldest is `Interrupt::MoveCardToOldest`, the card's controller's) and
  `settle_burn_to_prevent` (Superior Mettle's offer each later round,
  `Side::burn_to_prevent` and `burn_to_prevent_asked`,
  `Interrupt::MayPreventForBlood`), `settle_discard_looks` (Count Jocalo:
  `GameState::discard_actions_used`, pushed at the discard phase action's spend
  sites) and `settle_pool_losses` (Conniver: `Seat::lose_pool` flags
  `Seat::lost_pool`, every loss a cost included; `Hook::PreyLosesPool`'s cards
  offered `Interrupt::MayLockForBlood`).
- A hand over size waits for no pending choice but a draw's own
  (`Engine::discards_first`, read in `raw_request` and `apply`).
- The notes (`notes.rs`; `Engine::notes`, `None` by default: training,
  fuzz and replays keep nothing): what happened, in order, for a
  narrator — actions, plays, reveals, looks, combats — and before each
  note and at each cause's bounds the table as it stood
  (`Note::Ledger`: the phase and the transfers left, pools, ousts, the
  Edge, minions' presence, torpor, blood, lock, capacity and title,
  cards in play with their minion, lock, counters and controller,
  uncontrolled vampires' counters, the census, a referendum under way;
  a handler running parks the flow as `GameOver`, so a ledger noted
  then keeps the phase and transfers it left) with what
  changed it (`Cause`: a cost or an effect, of a card played or in play,
  a minion, or a rule). The host diffs two ledgers in a row, so a change
  is told in its place and with its cause. A pool or blood change runs
  inside a cause: `caused` (a closure, early returns safe), or
  `begin_cause`/`end_cause` around a stretch with none; `paying` turns
  the effect under way into its cost; a cause nested inside another
  names its own source (a cost inside an action's effect, a card's
  rider inside combat damage). An interrupt keeps the cause it was
  queued under (`interrupt_causes`, tagged at each cause's bounds), its
  answer and its settling noted as that cause's; one that is its card's
  own effect or cost names it in its arm. The host sweep
  (`crates/host/tests/events.rs`) fails a `pool` or `blood` event with
  no cause, a cost gained, or a rule moving what it cannot (a transfer
  off the active seat's pool). A strike pair's note is inserted before
  its own damage (`insert_note`); a failed block attempt is a ledger
  diff (`Ledger::attempting`).
- `caused` borrows the whole engine: a cost met while `self.combat` or
  `self.action` is borrowed is collected and paid after the borrow
  (`combat_apply`'s maneuver blood, Ossian's lives).

### 6.3 Turn and phases

`game.rs`, `contests.rs`, `influence.rs`, `plays.rs`.

- Unlock: DESIGN §10.7 (`Flow::Abilities`, `Pending`). Cards a Methuselah
  controls unlock in their unlock phase. Substitutions: `Minion.skip_unlock`,
  `Modifier::CounterInsteadOfUnlock`, `HostDoesNotUnlock` (read on the
  card's bearer: an ally's own card too, Cry Wolf); a card's own unlock
  (`Effect::UnlockHost`, `HostBurnBloodToUnlock`) unlocks its bearer only
  once every other such card on it was paid this phase
  (`Engine::unlock_released`: two Nightmare Curses, 2 pool). The rulebook burn
  option is a discard in any unlock phase (`Seat::burn_option_used`).
- Unlock-phase effects (`start_unlock_effects`, `Flow::Abilities`): one
  ordered impulse sequence clockwise from the active Methuselah, who settles
  their mandatory effects (contest upkeep, every pending rider) in the order
  they choose, among their optional ones (the Edge's pool, a withdrawal
  §6.17, abilities), and cannot pass before; then the other seats'
  abilities. The Edge's pool goes through `Interrupt::MayMoveEdgePool`
  when a vampire of the seat prints `Stat::EdgePoolToHost` (Heather
  Florent: her blood or the pool). A contest with no rival left is released and
  a forced yield settles at once: neither is a decision.
- Phase rotation (`Flow::PhaseRotation`, rules-qa 7.9): `resume` is the
  phase flow the impulse returns to when another seat plays an effect, and
  the phase to end when everyone passes. After the Enkil Cog's action
  (`Flow::CogAction`, rules-qa 49.6) the impulse returns to the active
  Methuselah's minion phase. `Engine::phase` (for clients) is the outermost
  flow under any play, terms or rotation: an action, its combat and
  referendum belong to the minion phase, the unlock-phase effects to the
  unlock phase.
- Mandatory phase effects: `Seat::master_resolved` / `discard_resolved` list
  the cards whose effect resolved this phase; the rest in play are due
  (`Engine::master_dues`, `discard_dues`). A vampire's printed
  `Hook::MasterPhaseStart` rider (Rico Loco) is due the same way
  (`Engine::master_printed_dues`, `Choice::ResolvePrinted`,
  `Seat::master_resolved_minions`), its requirements read live. Pariah's
  `Effect::DiscardOfElseLockHost` is Nightmares upon Nightmares'
  `Interrupt::LockOrDiscard` with `of`: a card of the filter, replaced at
  once.
- Master phase: out-of-turn masters are accounted against the next master phase
  (rules-qa 3.3); a canceled master gives its player a master phase action back
  when Wash says so (`Seat.mpa_next`). Abilities paid with a master phase action
  are not once-per-phase. `GameState::mpa_left` adds Rumors of Gehenna's
  "two (instead of one)" neither to Anson's second nor to another copy's.
  "At the end of your master phase" riders fire as it closes
  (`Hook::MasterPhaseEnd`, `master_phase_end_riders`; Winchester Mansion's
  `Requirement::NoMasterCardPlayed`, `Seat::master_played`).
- Rowan Ring's and Wooden Stake's hold (`Minion::no_unlock_in_torpor`):
  no normal unlock while in torpor (§6.7).
- An ally's unlock-phase damage on its controller's ready vampire
  (`Effect::HostDamagesOwnReadyVampire`, `MinionPurpose::DamageVampireBy`:
  Wendell) is the ally's: a vampire immune to allies' damage takes none
  (Carolina Vález, ANK 20210228).
- Minion phase: `Engine::mandatory_actions` — a mandatory action (printed,
  Puppet Master's take, marked `Effect::DueThisMinionPhase`; Temptation's
  "you can" is not) is the only offer and the phase cannot end before it; a
  minion unable to take it declares nothing (spec §3.9). Several mandatory
  actions are ordered by the acting Methuselah (rules-qa 17.2). An ally's
  own card may carry one (Cry Wolf), read like a vampire's printed text,
  and so may a card on the minion (Lunatic Eruption, `Actor::Host`); two
  copies of one such card on a minion leave it stuck (LSJ 20041103;
  `mandatory_split`), `Engine::dues` naming the minion either way.
- Influence: `Choice::FollowPath` (rules-qa 97: a Sabbat vampire following
  no path burns 1 blood to follow one of `state::SABBAT_PATHS`, its
  `Minion::path_override`; no transfer spent), transfers, `Choice::Withdraw`
  (2 transfers: a counter back to pool), crypt draw (4 transfers + 1 pool),
  influence-out, no voluntary self-contest; `Window::InfluencePhaseOwn` (once
  per phase), `Window::Transfers` (paid in transfers);
  `GameState::transfer_bonus`; `Window::InfluencedOut` card plays in the same
  request, at a vampire entered since its player's minion phase (Proxy
  Kissed, out-of-turn).
- Discard: one discard phase action (`Stat::DiscardPhaseActions` read as the
  phase begins, `Seat::granted_dpa`: Powerbase: Los Angeles's, used to
  discard a card requiring an Anarch or making a vampire Anarch, lets its
  user unlock a ready Anarch); `Hook::DiscardPhase` riders fire as the
  impulse leaves the active Methuselah. The end-of-turn hand size
  (`Seat::timed`, `Expiry::EndOfTurn`) ends at the end of the turn.
- "During X, do Y" abilities are once per that phase per copy (`phase_uses`,
  rules-qa 6.6; a copy's modes share it); an ability spending a resource
  with no "during" clause (`DiscardPhaseAction`, `Transfers`) is limited only
  by the resource.
- A temporarily controlled minion returns through `return_temporary`: its owner
  set back first, then removed if that Methuselah is ousted (`remove_from_game`:
  every card on it burns, rules-qa 27.4). Malkavian Dementia's
  (`Minion::temp_until_unlock`) returns as its taker's unlock phase opens;
  Puppet Master's at the turn's end.
- A turn may be skipped (`Modifier::TurnSkippable`: First Tradition): after
  the withdrawal checks, `start_turn` asks `Interrupt::MaySkipTurn`
  (`Choice::SkipTurn`) and waits; a pass runs `begin_turn`, a skip
  (`skip_turn`) counts on the seat's copies and starts the prey's turn,
  nothing of the skipped one happening (`turn` not advanced). Its 2 pool is an
  unlock rider (`Effect::BurnPoolTowardBurnSelf`, `CardInPlay::tally`).
- King's Rising: `Modifier::NoTransferCounterMoves` withholds its
  controller's `Transfer` and `Withdraw` options (the crypt draw stays);
  its unlock burn reads the Edge through `rider_card_holds` (`HasEdge`).

### 6.4 Card plays

`plays.rs`: `PendingPlay`, `PlayCtx`.

- Offers are enumerated from definitions: windows, actor, requirements,
  targets, limits, affordability counting potential reductions.
- Announcement: terms (`Flow::PlayTerms`), cost reductions (`Flow::CostTerms`),
  then the as-played window (`Flow::AsPlayed`, a nested cancel stack, spec §1.8).
  Costs are folded through layers and paid at resolution for actions (unpayable
  → fizzle: successful, no effect, paid as far as it goes; rules-qa 3.6).
- Cancel as played: the card is replaced; its own "do not replace until" is
  canceled with it; a canceled limited bleed leaves the limit unused; cancelling
  abilities (The Vozhd of Gravesend) mark the play canceled as used
  (`as_played_apply`). Cancels by type/Discipline read the mode played.
  `Effect::LockCanceledActor` locks the minion that played the card
  canceled (Evil Eye [MAL]); `Requirement::PlayedFrenzy` cancels any frenzy
  card (`frenzy_on`), by a combatant, any Methuselah's vampire out of the
  combat (`Actor::OutOfCombatVampire`: locked or in torpor too) or a
  reacting minion (Bliss). `Effect::BarCanceledForActorThisAction` (Shape
  Mastery) bars the canceled card by name for its minion this action
  (`ActionCtx::barred_for_minion`, `CombatCtx`'s outside an action); a
  strike card so canceled restricts an anarch's strikes (Groundfighting,
  `restricts_strikes_of`, LSJ 20050224-2). A cancel by two discarded
  combat cards (Target Vitals, Target Retainer) leaves the canceled card's
  cost paid, surcharges included (PIB 20121031). A card canceled is offered
  for canceling no more, by a card, an ability or a payment
  (`as_played_request`; Santaleous, Khalu, Target Vitals' discards; Louhi's
  `Requirement::PlayedByPredatorsVampire`, a vampire's play only).
- An action card's as-played window, its action not declared yet
  (`reacting_candidates` with no `ActionCtx`): a cancel by another's ready
  minion, unlocked or woken; a wake by a locked one, only a wake with no
  effect on the action (`as_played_action_card`: an action card at the
  stack's bottom), or another's (`Effect::WakeTarget`: Shadow Sentinel
  [OBL], Fillip). The woken status waits in `GameState::pre_woken`, taken
  into `ActionCtx::woken` as the action is declared, cleared when the stack
  ends with none (canceled, an oust). A woken minion meets `ActorUnlocked`
  for a reaction card only (`play_options`' `woken_reaction`; rules-qa
  96.4), never `ActorUnlockedToLock` (a reaction locking its vampire as
  the effect: Hide the Heart [VAL], ANK 20230817).
- A vampire's own wake (Gracetius, `Requirement::HostMayWake`,
  `Engine::may_wake`): the wake cards' moments; outside an action it goes to
  `pre_woken` in `resolve_ability_now`. `Window::Wake` opens to the other
  Methuselahs after combat and after resolution too (`action_windows`; LSJ
  20091123), for the reactions played there (Fast Reaction, Touch of
  Pain); a record before protocol 2.28.0 reads past it
  (`replay::Then::unoffered`).
- "Younger" than the minion playing the card, a reactor too
  (`Requirement::YoungerThanPlayer`, Knotted Cord), not `Younger`'s acting
  minion: checked with the target in `play_options`.
- A play's two ends, `resolve_play` (after its payment; an action card's
  end is its action's) and `settle_canceled`, are noted
  (`Note::Play`, `PlayNote`), which the host tells as `resolved` and
  `canceled`; its payment runs as the card's cost, its effects as its
  effect (§6.2, the notes).
- Multi-Discipline cards: the generator gives one mode per non-empty set of their
  Disciplines (`Label::MultiDiscipline`); a mode's tags are all required, as on a
  combo card. A definition may split a printed "or" into modes, mapped back by
  its `CardDef::lines` (`cards::library_mode`): a stealth mode gated on
  `StealthNeeded` only when the stealth is an option of its own (Subversion,
  Power of One [pre]); stealth with more (Confusion [DEM]) is one mode in both
  windows, offered once. Surcharges read the Disciplines used
  (`played_disciplines`).
- Cards played from a location (Fleshforge Chamber: `virtual_hand`,
  `PlayCtx::from`) are not replaced; `Engine::playable_card` reads a play's card.
- Allies playing cards "as a vampire" (`Modifier::PlaysAsVampire`): meet one
  basic Discipline, pay in life (last life burns them; the card then has no
  effect, an action resolving first). The grant is read for the card it plays
  only (`cards::plays_card_as_vampire`, its "non-action" limit included;
  `has_disciplines` reads it so): `play_options_in` meets that card's
  `ActorSectIn`, `ActorCapacityAtLeast`, `ActorVampire` and
  `ActingAllyOrYounger` by the grant's sect and capacity, `holds` still
  reading an ally for every other reader (rules-qa 73.6).
  `AsVampire::combat_only`: cards played as combat cards only, a dual card's
  combat mode (Reanimated Corpse). `GameState::as_vampire_grants` lists every
  grant, an ally's borne ones, the action's (`Modifier::ActingPlaysAsVampire`,
  Mouthpiece [DOM]) while it acts, then the timed and Bima's (§6.10);
  `plays_card_as_vampire` takes the first fitting the card. An acting vampire
  meets such a basic Discipline too (`disciplines_met`'s `lent`), one missing
  requirement of a combo card at most. Cards requiring a Discipline a minion
  shuns (`Modifier::ShunsDiscipline`) are barred from every minion while it
  acts, attempts or succeeds to block or fights (`Engine::discipline_barred`,
  its `bearer`: an ally's own card; the mode played), and cannot target it at
  any mode of a card printing one (`GameState::shuns_discipline`: Draeven
  Softfoot; rules-digest D29). `discipline_barred` also reads a combatant's own
  bar for its combat (`Side::barred_disciplines`, set by
  `BlockRider::OpponentBarredDisciplines`: Pseudo-Blindness [CHI]), and
  `Modifier::BarsOpposingDisciplines` (Blessing of Chaos): the vampires opposing
  its bearer in combat, and the acting vampire's action modifiers once the
  bearer attempted to block, failed or not (`attempting`, `failed_attempts`,
  `blocked_by`; LSJ 20080818).
- "Requires flight" (the `FLIGHT` tag: As the Crow, Dive Bomb, Pounce,
  Patrol) is checked by `has_disciplines` with the Disciplines, an ally's
  printed flight included (Herald of Topheth).
- Treat the Sick Mind [OBE]'s `PlayedPutsOnUncontrolledWith(d)`: the card
  played puts itself on its minion target (`PutOnTarget`), a vampire with
  `d` its player does not control.
- A play canceled as it opens (Andrew Stuart: `Hook::HostPlaysCard` →
  `Effect::RevealTopCancels`; Veil of Darkness:
  `Modifier::FirstDisciplineCardRevealsTop`, a vampire's first card
  requiring a Discipline each turn, `Minion::discipline_card_turn`, unpaid;
  Two Wrongs) settles at once in `open_as_played`, with no as-played
  window.
- Karl Schrekt (`Stat::PlaysAsJusticar`: a title list naming a justicar,
  rules-qa 59.14) and Victor Gerard (`Stat::PlaysAsCamarilla`,
  `Stat::PlaysAsPrince`) meet a card's requirement through
  `cards::plays_as_sect_or_title`, read on the play and as its action
  resolves (`card_traits_lapsed`), never in `holds`: the card's effects in
  play and an ability's requirement do not take them for one (LSJ
  20101013). Met so, the title implies the Camarilla for that card
  (`cards::plays_as_camarilla`): Closed Session's bar leaves its caller a
  vote (`ReferendumCtx::only_sect_caller`).
- Vlad Tepes (`Stat::MeetsSectAndTitleRequirements`,
  `GameState::meets_sect_and_title_requirements`) holds `ActorSectIn` and
  `ActorTitleIn` for what he plays or uses; a referendum's terms are read
  as if (`ReferendumDef::feasible`'s `as_if`, from the action's card or
  granted mode: `cards::meets_as_if`, `cards::term_filter_holds`).
  `Stat::CannotPlay` (Nehemiah's [cel] modes) and
  `Stat::NoOthersMasterTargeting` (Mimir, `GameState::shielded_from_master`;
  Archon Investigation's burn of an acting minion too) are read in
  `play_options`. A card mode's `Requirement::ActorBloodAtLeast` (a burn
  it asks for, ANK 20210226) is read beyond the cost as it would be paid,
  surcharges included, in the offer's affordability.
- A burned acting minion is offered as no card's actor
  (`actor_candidates`: a blood hunt's diablerist plays nothing after
  resolution). The burn option reads a card's printed trait requirement
  (`cards::trait_requirements`: a Seraph, an infernal or an unmerged
  advanced vampire, a master card's `ControlsReadySeraph`, a sect or an
  untitled one), from a ready minion, a torpid one too for an action
  modifier (rules-qa 33.8).
  `GameState::master_surcharge_on` folds Walker Grimes's
  `Stat::MasterTargetingCostsYou` into the haven surcharge. Burden the Mind's
  toll (`Modifier::HostUnlockToll`, `GameState::unlock_toll_on`, out of the
  minion's turn) is added by `Engine::unlock_toll` to a played card's pool
  cost (`Engine::played_pool_cost`, offer and payment) and to an ability's
  (`ability_gate`, `use_ability`, paid through `pay_pool_for_ability`) for
  each minion its effects unlock, wake or let block as if unlocked:
  `Effect::unlocks`, classified in `families::class` with every effect, names
  who (`families::Unlocks`); Baba Yaga's `Interrupt::MayUnlock` pays it too.
  Skidmark's `Stat::HoldsReplacementsWhileActing` (`holds_replacements_of`)
  holds every card his controller plays during his action until it ends.
- Contingency Planning: `Requirement::PlayedCardRedirectsBleed` (one
  helper, `cards::mode_redirects_bleed`, with Two Wrongs); an action-scoped
  `Modifier::BleedBurnCap` caps whoever is bled (`GameState::bleed_burn_cap`).
- `Modifier::BearerPlaysChosenAsSuperior` (Infernal Pact): the card's
  chosen Discipline (`Terms::Discipline`, any, one already superior a
  legal no-op) met at either level by its bearer in `disciplines_met`
  (`GameState::plays_chosen_as_superior`), its level unchanged for other
  readers.
- Blessed Audience [MEL]: `Terms::XMinionPicks { boost }` reads its filter
  without "younger" and makes a vampire pick younger than the actor's
  capacity raised by the points the oldest pick needs
  (`Engine::capacity_boost_for`), paid inside X (`PlayCtx::x`), read again
  at `LockXPicks`. Clio's Kiss [tem]: `Effect::ExchangeHandForAsh(filter)`
  names both cards as the card action is announced (`declare_action` opens
  `Flow::Exchange`, `exchange_request` reads the filter), exchanged in
  `succeed` (`exchange_named`, Lenelle's too).

### 6.5 Actions

`actions.rs`: `ActionCtx`, `ActionState`; `ActionKind`, `ActionId`.

- An action's two ends, `declare_action` and `drop_action` (resolved, or cut
  short by an oust, the turn's end or the game's: `run` drops it before
  `Step::Over`, and the game's end first puts its card where the action's end
  would, `place_action_card`), are noted (`Note::Action`, `ActionNote`: the
  context then), which the host tells as `action_starts` and `action_ends`; Mask
  of a Thousand Faces' takeover as `Note::Acting` (`acting_changes`). An action
  ends only through `drop_action`; a `self.action.take()` that puts it back is
  no end. Its resolution (`succeed`) and its window's choices (`action_apply`)
  run as its effect (`ActionCtx::cause_source`), its costs inside as costs
  (`paying`). Taken out, the action is not `self.action`: what reads it there
  misses it — an action card's burns go through `Engine::card_burns` into
  `ActionCtx::burned_owners` (Predator's Transformation [ABO], the Trophies;
  rules-qa 94.5).
- A card "only usable by a vampire in torpor" is a torpid unlocked
  vampire's (rulebook: "Only unlocked minions can perform actions"), a
  locked one's with `Requirement::LockedActor` (Movement of the Slow Body)
  and either with `MayBeLocked` (Recure of the Homeland): `play_options`'
  `ActorInTorpor` candidates.
- Kinds: rulebook (bleed, hunt, leave torpor, rescue, diablerie, political),
  `Card`, `Equip`/`EquipFrom`/`EquipFound`, `Recruit`/`Employ`, `BurnCard`,
  `StealCard`, `Ability` (granted by a card in play or printed on a vampire;
  checked by `ability_gate`, `may_take_granted` inside it). A granted action
  has only its printed stealth (rules-qa 6.2); an action card its printed
  stealth (`printed_stealth`), a bleed card's too (rules-qa 104.5). A
  granted action's `Terms::Discard` is its cost, discarded as it resolves
  (`Interrupt::DiscardChosen` from `succeed`'s `Ability` arm; too few in
  hand, those there discarded and the action fizzles; Talbot, rules-digest
  D20). Any ability whose discard terms the hand cannot pay is not offered
  (`GameState::discard_terms_met` in `ability_gate`: an empty choice
  otherwise; Velya). A granted action requiring the Edge fizzles once its
  Methuselah lost it (`ability_void`; Bulscu, D13). `Effect::GainEdge` in
  `succeed`'s `Ability` arm takes the Edge from whoever holds it (Jazz
  Wentworth's `Requirement::EdgeUncontrolled`; Tereza Rostas's seat target
  `TargetSeatHasEdgeAsDeclared`, which `target_still_qualifies` skips: the
  Edge burned meanwhile, the steal still gets it, rules-qa 90.7).
  `Terms::VampireToHoldTarget { anyone, ready }` moves a card to a vampire
  chosen as used: the user's (Luciano Carvalho) or any ready one (Jayne
  Jonestown's Society of Leopold).
- The blood an action's costs took, as paid and whoever paid (a card's,
  a surcharge, a rescue's share, leaving torpor; not a continuation's
  burn): `ActionCtx::blood_cost`, recorded at each payment site, read by
  `Requirement::ActionCostBlood` (Cavalier).
- Steals: every steal of a card in play (`Effect::StealTarget*`,
  `StealLocation`) carries `Requirement::NotControlled` in its target, even
  where the text does not print "another Methuselah's": one cannot steal
  from oneself (rules-qa 90.4; Chameleon excepted by its ruling, New
  Management's own location a legal no-op: LSJ 20080803-2);
  `tests/lint.rs` (`steals_never_target_ones_own_cards`) holds a new steal
  to it.
- Hunting: `actions::is_hunt(kind, card_hunt)`, read through
  `ActionCtx::is_hunt` and `PrintedCtx::is_hunt`: an action a card makes a
  hunt (`Effect::HuntAction`: Loki's Gift, Psychophagia, Abactor, Week of
  Nightmares) is hunting for everything reading one (LSJ 20050720-2;
  rules-qa 72.1–72.5). `ActionCtx::card_hunt` is the card's part only;
  `Requirement::RulebookHunt` is the rulebook hunt (Zöe's "usual +1");
  `Modifier::NoUsualHuntStealth` takes its +1 in `ActionCtx::stealth`
  (Strained Vitae Supply, PIB 20120204).
  Every hunt's blood goes through `Engine::hunt_gain`, successful when it
  gains any (none after Foul Blood's −1: LSJ 20050720-2) and
  `GameState::hunt_gains` (no disease counter, no bar on gaining blood; at
  capacity too, rules-qa 89.1), the one test of `ActionCtx::hunt_successful`
  (The Anarch Free Press, Abactor's blood hunt; rules-qa 63.2): Azrael's
  `Stat::HuntBloodElsewhere` holds that part back for
  `MinionPurpose::HuntBloodTo` (another vampire of his controller's, or
  kept), still gained at capacity; Rabbat's `Stat::HuntBloodToPool` for
  `Interrupt::MayHuntBloodToPool` (her controller's pool, or kept).
- "Your prey" as a seat target (`Requirement::TargetPrey`) is re-read as the
  action resolves (`Engine::reread_prey_target` in `succeed`, a card's or a
  granted action's): the prey ousted, the new prey is the target (rules-qa
  121.120 corrected, 124.42).
- A card action whose mode carries the marker `Effect::RescueAction` (Sense
  Vitality [val], Healing Touch [OBE], …) is a rescue to every reader
  (`ActionCtx::is_rescue`; bearer modifiers `during` a rescue through
  `bearer_kind`; Ritual Goblet's `RescuedFromTorpor`; Boss Callihan's bar in
  `free_action_options`); its card cost takes `rescue_discount`
  (`action_card_cost`); resolved, `Engine::rescue_clots` and
  `Engine::rescued` (Saulot's gain, `HostRescuesYours` riders) as the
  rulebook rescue's `rescue_apply` (rules-qa 124.204).
- Directedness is `ActionCtx::directed_seat`: bleed target; rescue/diablerie of
  another's vampire; a minion, seat or card target's controller unless the acting
  Methuselah (a card target never makes the action directed at its minion). The
  Ⓓ icon is only a reminder (rules-qa 17.4, 20.1). An action at hands,
  or at every other Methuselah's vampires or crypt, is directed at their
  Methuselahs (`directed_seats`: Vaticination's every hand, Sowing
  Dissension, Petra Resonance; Augur's predator and prey, rules-qa 102.11).
  `GameState::may_direct_at` (Shadow Cloak, Secure Haven) filters directed
  actions of every kind.
- An action's required Disciplines (`ActionCtx::requires_discipline`): its
  card's mode tags, or the declaring ability's `ActorHasDiscipline`
  requirements (`cards::ability_disciplines`; LSJ 20080604, rules-qa 123.1:
  Mary Johnson's steal requires Presence). `ActionRequiresDiscipline`,
  `ActionRequiresClanOrDiscipline`, the stealth bonuses, Lobotomy and Cock
  Robin's unlock read it; `GameState::immune_to_directed` (Aggressive Corpse,
  Blessing of Chaos [DEM]) masks a card's action at another Methuselah's
  minion by the mode's tags and an ability's by `ability_disciplines`
  (`may_direct_ability`).
- Windows: as-announced (acting Methuselah, then others: `ReactionAsAnnounced`)
  → block loop (`ActionBlockAttempt`, `ReactionUnlock`,
  `ReactionUnlockAttempt`) → `ActionState::Blocked` (Crypt's Sons continue as if
  unblocked) → block resolution (an acting minion out of the ready region
  by then, Cry Wolf's `BlockRider::ActingToTorpor`: blocked, no combat) →
  combat → after-combat → for a bleed of 1+, `ActionWouldBeSuccessful` →
  resolution → after-resolution.
- **After resolution always opens** once an action reached resolution —
  successful, fizzled, failed, blocked (after its combat), or ended (Change of
  Target, Mirror Walk, Yawp Court's combat: `ActionCtx::ended_early`, no card
  is played there; rules-qa 88.1). Only a cancel before resolution or an
  acting minion leaving skips it (`action_windows`).
  Damage "after resolution" is ordered there by the acting Methuselah, and
  so is Day Operation's torpor (`ActionCtx::torpor_after`,
  `Choice::TorporAfterResolution`, LSJ 20040120-2).
- Resolution (`Engine::succeed`): the targets and what the effect needs are
  re-read first (`resolution_void`): gone or illegal → fizzle, nothing paid
  (ANK 20210124; rules-qa 17.4); then the action's surcharges
  (`pay_surcharges`) and its own cost: one no longer payable → fizzle,
  every part paid as far as it goes, the minion's blood and its
  Methuselah's pool, the last ousting them (`fizzle_short`; rules-qa 3.6,
  Batch 117); a political action's terms are read there too
  (`political_void`, rules-qa 43.1);
  an action card's sect or title requirement re-read as it succeeds
  (`card_traits_lapsed`: The Red Question's Anarch, rules-qa 104.7);
  a card, uncontrolled or Methuselah target by `target_still_qualifies` on
  both paths, a minion by each (an ability's reads its host). An
  action whose acting minion left play fails. A bleed target ousted before
  resolution: fizzled, no pool, no Edge. A **successful bleed** is one of 1+;
  only it takes the Edge (`ActionCtx.bleed_successful`), burned instead under
  Leverage (`ActionCtx::no_edge`); a bleed at 0 is still a
  successful action. A hunt succeeds only if it gains blood.
- Canceled before resolution (The Kiss of Ra, `Effect::CancelAction`):
  `ActionCtx::canceled` ends the action from the declined state with no
  outcome, no after-resolution window and no NRA record unless it was
  blocked before (`blocked_by`).
- **NRA**: `ActionId` — card name for hand cards, `InPlay(card, ability)` per
  copy and ability (LSJ 20100909), `Printed(vampire, ability)`, `Rulebook(kind,
  target)` (Change of Target's ban); `Effect::SameActionAs` unifies two printed
  actions. Recorded at resolution (`mark_resolved`); a failed bleed or
  political action counts. Delaying Tactics bans by `ActionId`. `Printed`
  keeps the vampire so a record the seat holds (`Seat::forbidden_political`)
  still names it. A card may let its action be performed again this turn
  (`Effect::MayRepeatThisTurn`, Nocturn [OBT]: `Minion::repeats`,
  `GameState::performed_this_turn` against `repeats_of`), each earlier one
  +1 blood (`card_surcharge`). A two-action printed ability keeps its
  progress on the minion (`Minion::progress`, Quincy, The Trapper).
- **Queued combat** (Hedonism: `ActionCtx::queued_combat`,
  `begin_queued_combat`, `resume_after_queued_combat`): it begins right
  after the block's own combat (`after_combat`), the after-combat window
  opening once both are over (ANK 20250613, 20250710: "usable after block
  resolution" waits for every chained combat), or after a successful
  action's effects, still part of its resolution (Sabbat V5 rules "Queue a
  combat"; rules-qa 53.3; a fizzled action counts as successful, 56.1);
  begun after the effects, its end goes on to the after-resolution window
  (`queued_at_resolution`). Both minions must still be ready (advanced rules
  4.9.3); an action failing or ending first loses it (LSJ 20090826).
  Psyche!'s queue is the combat's (§6.7). Fast Reaction queues from the
  after-combat window (`QueueCombatAfterBlock`, its riders in
  `ActionCtx::queued_riders`); `Requirement::BlockerOwn` reads
  `last_combat`, so it follows the block's combat only, never its own. No
  action continues while a combat is queued (`Choice::ContinueAction`
  withheld), and a queued combat beginning drops the continuations before
  it (`drop_continuations`): only the final combat may continue the action
  (ANK 20250613; RTR 20020501).
- **Forward Momentum** (`ActionCtx::momentum`, `bleed_ends_riders`,
  `finish_action`): +1 counter after a successful bleed, −1 otherwise (a
  blocked bleed included), pending in the after-resolution window beside
  its unlock, in the controller's order (rules-qa 53.6): a gain can be
  taken by the unlock first, a loss after it; the rest applies as the
  action ends.
- **Diablerie's steps** (`diablerise_to`): one unit, nothing played between
  them (rulebook "Diablerie"); step 2, the victim's equipment the
  diablerist could hold, is `Interrupt::DiablerieEquipment` (a card at a
  time, `ResolutionChoice`), and steps 1 and 3 (`diablerie_burns`: the
  blood, the burn) follow as it ends; with nothing to take, at once. An
  Sacrificial Lamb's burn of a vampire in torpor shares step 2
  (`Interrupt::DiablerieEquipment { diablerie: false }`), then
  `taken_from_burns`: the target's capacity in blood, read before, if it
  burned; no diablerie. An
  older victim (advanced rules 4) then gives a Discipline:
  `Interrupt::DisciplineSearch` (Childe of the Revolution's) on the
  diablerist, not one of a Discipline it has at superior, the card's
  requirements ignored (LSJ 20051116-2), asked before the window of the
  cards played on the diablerie (rules-qa 81.1). A victim whose burn is
  replaced (Byzar, Charigger: `Engine::burn_replaced`) makes it unsuccessful
  at its first step: `diablerise_to` returns false, nothing taken, no blood
  hunt, the action on to its after-resolution window (LSJ 20100228; rules-qa
  121.20).
- **Diablerie's blood hunt** (`end_action_into_blood_hunt`): part of the
  action (spec §3.7.8.1), which ends after it through its after-resolution
  window (rules-qa 5.8); the referendum's rotation starts at the
  diablerist's controller, clockwise (DESIGN §4.2). A diablerie made in a
  combat (§6.7) holds its window and hunt with the combat suspended, the
  only referendum with `Engine::combat()` set: the window takes the combat
  cards keyed to it (Slake the Thirst [san], advanced rules §3.7.8.1; no
  action modifier, `actor_candidates`), the hunt's burn is a burn in
  combat (Hector Trelane), and its end goes back into the combat
  (`resume_hunted_combat`, before the after-resolution riders); an oust
  meanwhile reads the combat as it resumes (`after_deferred_oust`,
  `settle_acting_ousted`), and an action cut short cuts it
  (`cut_combat`).
- **Live conditions** (owner, 2026-09-25): an effect's conditions are evaluated
  throughout, never snapshotted (`Effect::If` → `ActionCtx.conditional`); a
  limited bleed increase counts against the limit only while its condition holds.
  The ruling-backed exception: Night Terrors' −X stealth is fixed at play (RTR
  20030519); Keep it Simple's bleed reduction, "the stealth … when this card
  is played", too (`Effect::ReduceBleedByActingStealth`).
- Intercept given to a minion lasts the whole action, across failed attempts
  (`ActionCtx.intercept_given`, rules-qa 15.2).
- "Only when needed" (`Requirement::StealthNeeded`, `InterceptNeeded`;
  rules-qa 3.2) is for additions. A reduction of stealth or intercept never
  carries it (`no_reduction_waits_for_a_need`, `cards/slots.rs`) and needs
  no block attempt unless its text names one (LSJ 20020612; owner,
  2026-11-11): "only usable if a minion attempts to block", "the vampire
  attempting to block" sit in the attempt's window (Form of the Bat,
  Gwenllian Arwyn, Mélusine d'Anjou); one stating an effect is played from
  the block window on, to no effect with no attempt (Smoke and Mirrors
  [obt]).
- An attempt whose window closed short of the stealth: its Methuselah's
  pass in that round, short, was its decline (`ActionCtx::declining` while
  the table's round runs, the view's `declining`); it is marked declined
  and the open window continues after it (`OrderedWindow::after`, the
  attempt round's passes through it counted): it closes as the passes
  come round to the acting Methuselah, whose turn there is the window's
  first and the one after a play (rules-qa 82.1). An attempt failed otherwise
  (`ActionCtx::attempt_failed`: locked or gone at the closure, Enchanting
  Gaze's counter, Forced Confessional, a cancel; `Engine::resign_attempt`:
  a toll unpaid, an ally's last life, Puppet Master, Dawn Operation)
  starts it anew at the acting Methuselah, as after a play (1.2; 82.2). An
  action ended early (The Kiss of Ra) offers no further attempt.
  `Engine::short_declines`, kept for the record reader only (`None` otherwise),
  notes each short decline with the seats an engine before 82.1 asked again
  (`Reasked`).
- Enter-combat actions (`Effect::EnterCombatWithTarget`): the combat begins at
  resolution (`start_combat_at_resolution`) and is part of the action; no "held
  when blocking" state; a blocked rush continued as if unblocked enters its
  combat at resolution (rules-qa 17.3).
- Action-scoped riders live on `ActionCtx` (block riders read at announcement,
  `barred_blockers`, `vampires_cannot_block`, `no_unlock`, `no_reactions`,
  `put_if_no_block`, `after_resolution_unlocks`, `cost_reduction`, …); combat
  riders from the action go through `Engine::combat_riders`.
- `Minion::next_action_stealth` (Tangle Atropos' Hand [TEM]) is taken as
  the vampire's next action is declared, cleared at the turn's end;
  `ActionCtx::card_to_hand` returns the action's card as it ends. An action
  card burning the Edge fizzles once the Edge is gone (Inside Dirt, rules-qa
  77.13). `Hook::CorruptionBurnedInAction` fires in `finish_action` for the
  seats in `GameState::corruption_burned`: the burners, fed by
  `GameState::burn_corruption`'s `by`, cleared at declaration (Rise of the
  Fallen, rules-qa 77.28). `Hook::HostRecruitsOrEmploys`
  (`recruits_or_employs_riders`) follows a recruit or employ's success and
  `EmployNamed` (Baba Yaga: `Effect::MayUnlock { once_each_turn }`,
  `Interrupt::MayUnlock`, `state::once_unlock_key`).
- `ActionCtx::unlock_on_success` (Hag's Wrinkles), `burn_at_end` (Into
  Thin Air on the vampire for the action, burned in `finish_action`) and
  `ash_to_hand_on_name` (Tye Cooper: the named ash card on success, a Pass
  offered). `Minion::extra_bleeds` lifts the once-a-turn bleed cap
  (`may_bleed`, `mark_bled` at every bleed-cap site: Taking the Skin).
  Madness Network's off-turn actions: `GameState::acts_out_of_turn`, also
  read by Mask of a Thousand Faces. `burn_blocking_clan` lets a clan block
  the burn of its card (Madness Network). `GameState::predator_keeping`
  keeps an acting seat ousted mid-action as the predator it was.
- A diablerie by card (Cloak of Blood, Stealing Years):
  `DeclareAction(ActionKind::Diablerie)`, the card's target the victim, its
  printed stealth read like any action card's; `diablerie_card_text` applies
  what the card adds as the rulebook steps end, `diablerie_card_votes` its
  votes as `ReferendumCtx::bonus`. Red List victims: `red_list_burned`.
- A card's action toll (Enrage [PRE], `Modifier::HostActionToll`) joins the
  printed one in `action_toll`: masked as announced, paid at resolution,
  hunts and combat entries free; `ActionCtx::tolled` burns after the action.
  A bleed's (Greater Curse, `HostBleedToll`) too; an ally pays it in life,
  its last life burning it.
- An action card's layers for its whole action (The Ailing Spirit [obe]'s
  intercept, Cheat the Fates [tem]'s `Modifier::CannotBlockWithout`, read in
  `blocker_barred`): `Effect::DeclaredActionLayer`, pushed by
  `declare_action` as an action-scope layer, inert at success.
  `NoDisciplineCardsThisAction` on an action card is read there too. Lord
  of Serenity [for]'s `Modifier::VampiresBlockAnywhere` opens the block to
  every other Methuselah's vampires (`eligible_blockers`, `may_block_from`).
- "Up to two" targets (`Terms::OptionalSecondTarget`, `same_controller` for
  Shattering Crescendo [mel]; the pick a target, Secure Haven's bar
  included): the action fizzles only once both no longer qualify; each
  effect re-reads its own (`TargetsLeaveTorpor`, `DamageTargets`,
  `BloodToTargets`); `Terms::SecondTarget` is the mandatory pair (Esbat),
  its mode masked in `play_options` with a single target. A card
  to discard as the action resolves (`DiscardNamedFromHand`) is checked
  with the target: none left, it fizzles.
- A granted burn paying the Edge (Shatter the Gate) or blood (Shadowed
  Eyes [AUS]) pays at resolution in `succeed`'s `BurnCard` arm: the Edge
  gone, it fizzles unpaid (`resolution_void`); an ally pays life, its last
  burning it.
- A bleed card's "gain n pool" and "unlock this vampire" (Greater Curse
  [dai], Cheat the Fates [TEM]) resolve in `succeed`'s bleed arm, read by
  name, whatever the amount: the pool before the Edge.
- Acting fails on a burn (`Interrupt::BurnOrFail`): not blocked; the same
  interrupt with `card` burns the card instead (Drink the Blood of Ahriman).
- Failing into a combat (Champion, Ensconced's burn and [VIS]):
  `Engine::fail_into_combat` marks the action failed and
  `ActionCtx::fail_combat`; `action_request` yields to `action_advance`,
  which starts that combat once the play or use resolving is over (both
  still ready), the action going on until it ends, failed, not blocked.
- "About to enter combat with an ally" (Gift of Sleep [obe]):
  `ActionCtx::acting_foe` is the minion the acting one is about to fight
  (its blocker, its rush's target, Psyche!'s opponent), read by
  `FightsAlly`; `LockFoe` locks it (a blocker kept locked as the action
  ends) or, played by the foe, the acting ally.
- Din of the Damned: `ActionCtx::no_gear_intercept` drops an equipment's or
  conviction's intercept (`cards::equipment_or_conviction`: its borne
  layers in `host_intercept`, its `AddIntercept` / `AttemptingIntercept`
  not given); `intercept_reaction_toll` adds blood or life to a reaction
  mode or a power's reaction granting intercept (`cards::grants_intercept`,
  `Engine::intercept_toll`: `reaction_surcharge` for a card, masked and
  paid with the use for a power; an ally pays life).
- One play of a card in a mode each action, by anyone
  (`Limit::OnePerActionThisMode`, `ActionCtx::played_modes`: Hide the Heart
  [val]).
- Hay Ride: `Requirement::ActionCardToAsh` (`Engine::action_card_to_ash`:
  the card `place_action_card` will put in the ash heap, which it reaches
  only as the action ends); `Effect::NextActionFromAsh` sets
  `Seat::next_from_ash`, cleared as the seat declares any action, and the
  virtual hand offers that ash card (`state::ASH_NEXT_ACTION`) to the
  chosen minion for an action only, as from hand, no replacement.
- Strix [STR]: `BleedBurnsNoPool` (`ActionCtx::bleed_no_pool`) caps the
  pool burned at 0 in `succeed`'s bleed arm, the bleed still successful
  (the Edge); `FightActingAtResolution` (`resolution_combat`) begins the
  combat at resolution after it (`start_combat_at_resolution`), the
  after-resolution window after the combat. [str]'s `UnlockAtEndIfBlocks`
  (`unlock_at_end_if_blocked`) unlocks a blocker as the action ends, next
  to Big Game's.
- `ActingBurnsOrFails { or_life }`: an ally pays life, its last burning it
  (Scobax [STR]); without it an ally cannot pay (Detect Authority).
- "Cannot block this action" played into the attempt of that minion fails
  it (`barred_blockers`, re-checked after each action effect: Psalm of the
  Damned [MAL]). `TargetNoReactionsOrPowers` bars reaction cards and
  [REACTION] powers (`no_reaction_powers`, read by `HostMayReact`);
  `TargetNoReactions` bars cards only.
- Momentary Delay's conditions: `ActionCtx::combat_rounds` (every
  combat's rounds, added in `close_combat`), `ended_combat_early` (a
  combat ends strike, Anesthetic Touch), `Minion::blood_lost_action`
  (every `lose_blood` since the action was declared, costs included).
  `ContinueAfterResolution` clears the outcome at once: no after-resolution
  card of the block follows it (LSJ 19981028).
- Directed actions at a minion bearing
  `Modifier::DirectedAtBearerCostsBloodOrLife` (Safe Passage [OBE]) pay it
  with François Loehr's printed toll, blood or life, at resolution.
- `Modifier::DirectedActionStealth` (National Guard Support: every minion's;
  Persona Non Grata: its host's) is read in `ActionCtx::stealth` while
  `directed_seat` names a Methuselah (`GameState::directed_action_stealth`),
  and `Modifier::StealthDirectedAtPrey` its controller's minions' while the
  prey is among `directed_seats`, a bleed only with `bleeds`
  (`GameState::prey_directed_stealth`: Hidden Pathways).
- Turn-long action stealth (Psychic Veil): `Effect::ActionStealthThisTurn`
  adds `TimedStat::MinionActionStealth` or `VampireActionStealth` to the
  seat until the turn ends, read in `ActionCtx::stealth` on the acting
  minion's kind; `Timed` hashes those rows only when set.
- An action card recruiting the ally it finds (`Effect::SearchRecruit`, The
  Summoning [PRE]) opens `Interrupt::AllySearch { recruit }`: the library's
  allies `recruit_versions` finds a version for (clan, Disciplines,
  requirements by `holds`, never as if), recruited, a recruit action for
  its readers (`ActionCtx::is_search_recruit`, `PrintedCtx::recruit_action`),
  its cost the recruiter's reductions and surcharges (`searched_ally_cost`).
- Special Report: `ActionCtx::pool_for_intercept`, its player's
  `Choice::BurnPool` once while intercept is needed, counted by
  `block_can_reach`. DeSalle's `Stat::AttemptingBurnsForIntercept` offers
  the same `Choice::BurnBloodForIntercept` to each attempting minion its
  requirements match, read live (`ActionCtx::printed_burn_open`), once each
  action (`ActionCtx::printed_burned`), counted by `intercept_reachable`. Zoning
  Board: `Effect::MarkTargetCardThisTurn` writes `GameState::marked_cards`, read
  by `Requirement::TargetMarkedThisTurn` on the granted action's card target
  (directed at that card's controller). Powerbase: Chicago's action:
  `CountersToPool` granted, to the acting Methuselah.
- The acting minion's after-resolution harms (`ActionCtx::damage_after`, a
  `Damage` of both kinds: Daring the Dawn, Foul Blood;
  `burn_after_zero_bleed`, due while `zero_bleed_burn` reads the bleed
  resolved for 0: Ecstasy [SER]) are one `Choice::TakeDamageAfterResolution`
  of the acting Methuselah, Pass withheld, taken by `end_action` if never
  chosen (`after_resolution_harms`). "Would successfully hunt" (Foul
  Blood) is the blocks-declined window (`Requirement::HuntWouldSucceed`): a
  hunt opens no would-be-successful window, which recorded games replay
  without. A hunt's blood is `GameState::hunt_amount(m, base)`,
  `ActionCtx::hunt_base` a card hunt's own, its bonus signed
  (`Modifier::ActionHunt(-1)`), and a hunt of 0 is not successful
  (`hunt_gain`; LSJ 20050720-2).
- A minion gone before block resolution leaves its blocker unlocked if the block
  locked it (`action_advance`'s `acting_gone`; RBK resolve-the-action:
  Revelation of Despair [ser]); one stolen before it keeps the block, the
  blocker locked, but no combat: `action_advance` begins none between minions of
  one Methuselah (`Effect::CorruptActingMayStealAtCapacity` →
  `corrupt_may_steal`; ANK 20220127, rules-qa 121.129). `Effect::ActingLasting`
  puts a lasting bonus on the acting minion (Ignis Fatuus' -1 stealth for the
  turn, a strip `block_can_reach` counts).

- Round 30's action cards: Truth of Blood's
  `Effect::BlockerControllerDiscardsAtRandomHeld` applies in `blocked` after
  the host-blocked riders, the discards held to `Release::EndOfAction`;
  Precision names any library card as played (`Terms::AnyCardName`, one
  `FindCard` per name, kept in `PlayCtx::ash_names[0]`; the host shows them
  as one `name_card` option, `describe::names_any_card`) and asks its optional
  burn with `Interrupt::MayBurnBloodToBurnPool`; Possession's
  `Effect::NamedAshVampireTo` moves the vampire named as played
  (`Terms::OwnAshVampire`), the action fizzling if it left (`card_void`
  reads `OwnAshVampire` terms); Poison the Well of Life [QUI]'s
  `Effect::BurnOthersInPlayLabeled` is directed at every other Methuselah
  (`at_every_methuselah`); Haqim's Law: Leadership's
  `Effect::OldestReadyOfClanControllersGainPool`.
- Round 30's picks and names: `Terms::XMinionPicks` picks minions of any
  Methuselah, X the picks less `extra` or plus `fewer` (Bloodlust X or X+1,
  Command the Legion X-1, `one_exempt` its [DOM] older pick); each picked
  minion's controller is directed at (`picked_seats` in `actions.rs`), a frenzy
  card is played on each pick (`GameState::frenzy_immune` masks them), and
  `Requirement::ActorAmongChosen` gates Bloodlust's rush.
  `Terms::ChooseYoungerClan` (Condemn the Sins of the Father, `PlayCtx::clan`,
  directed at every controller of a younger vampire of it,
  `GameState::younger_of_clan`); `Terms::NameInPlayOrAsh` (Gramle: a library
  name in `ash_names[0]` or a crypt name in `ash`; `Interrupt::CryptSearch`'s
  `name`); `Terms::AnyAshUniqueVampireWithBlood` (The Eternal Mask [SER]: heap,
  vampire, then X); `Terms::OwnAshMinionCardBurnedSinceLastTurn` (Compel the
  Spirit) reads `Seat::burned_minion_cards`, (copy, turn) marks of allies and
  retainers burned from play as themselves, a represented card never marked.
  Seat bars for the turn: `GameState::seat_bleed_this_turn` (Dream World
  [PRE], cleared when a bleed is blocked) and `no_bleed_this_turn`
  (Enticement, read by `GameState::may_bleed_now`); Enticement's
  `Effect::BurnEdgeForTargetPool` is never a fizzle (LSJ 20100527), unlike
  `Effect::BurnEdge`. Edged Illusion [CHI]'s
  `Effect::DamageEachReadyMinionOfTargetSeat` lands through
  `damage_out_of_combat_at_once`, so a Dual Form set goes down together.
  Community Justice reads `Minion::diableries` (turn, victim's clan).
- Round 30's action modifiers: `Effect::AttemptFailsIf` fails only an attempt
  whose minion meets its requirements (Heart's Desire's bleed stays without
  one); `Effect::PreventEachCombatThisAction` fills
  `ActionCtx::combat_prevents`, added to `Side::combat_prevents` in
  `begin_combat`, every combat of the action (Divine Image);
  `Effect::ActingCannotActThisTurn` sets `Minion::recruited` (Mirror's Visage).
  `Terms::XPointsAmongMinions` allocates points one at a time (`PlayCtx::picks`
  with `PlayCtx::points`), read by `Effect::PointsInterceptThisAction` into
  `intercept_given` (Will-o'-the-Wisp [chi]); [CHI] reuses `Terms::XMinionPicks`
  with `Effect::PicksCannotBlockOrReact`. Forger's Hammer: no
  would-be-successful window outside a bleed or a political action, so "when an
  action … is successful" is the acting Methuselah's `ActionDuring` once blocks
  are declined (`Requirement::BlocksDeclined`, `Requirement::EquipFromHand`);
  `Effect::PutOnEquippedIfSuccessful` holds the card, placed on the weapon from
  the ash heap by `place_held_at` as `equip_from_hand` returns it;
  `Effect::ReduceActionCost` is `Shared` (a play or an ability). Shroudsight:
  `ActionCtx::played_copies` records every card announced during the action;
  after resolution (a blocked action's too, 55.5) its player is offered
  `Choice::TakeFrom` (ash heap) once for the action card (`card_bound_for_ash`,
  sent to the library's top as the action ends: `card_to_library_top`) or,
  [NEC], each card played this action now in their ash heap.
- Round 30's minions and equipment in actions: `Effect::BurnSourceIfBlocked`
  (an ability's, Malajit Chandramouli, Ambulance) records its card in
  `ActionCtx::burn_if_blocked`, burned in `host_blocked_riders` before any
  other effect, so the burn is that action's, not the lock's;
  `Requirement::ActorStayedReady` reads `ActionCtx::actor_torpored` (set where
  minions go to torpor): an acting minion back from torpor has its action
  ended (Ambulance; ANK 20220218). Ambulance's burn of an imbued is its own
  `ActionKind::Ability` (`Effect::BurnTargetMinion`), not the rulebook's
  `BurnImbued` (rules-qa 121.3): no equipment taken, no avenge.
- After-resolution burn and unlock granted for the turn: The Damned's
  `Modifier::HostBurnsBloodAfterSuccessfulBleed` adds to
  `ActionCtx::burn_after` as the window opens (a successful bleeding
  action, 0 pool included), owed like Daring the Dawn's damage
  (`TakeDamageAfterResolution`); Danse Macabre's
  `Lasting::MayUnlockForBloodAfterSuccess { sect }` pushes the acting
  minion's `after_resolution_unlocks` entry (sect read then), marked
  `ActionCtx::lasting_unlock` and spent (the lasting removed) only when
  used; another grant's entry for the same minion comes first.
- Stealth Ritus' `Effect::StealthIfTargetBurnsBlood`: the target, still
  ready with the blood, burns it and the action gets the stealth; else
  nothing (Gestalt's all-or-nothing). Lobotomy's
  `Modifier::NoEffectFromActionsRequiring`: an action card requiring one
  of its Disciplines, aimed at the bearer, is void in `card_void` (a
  fizzle, offered and blockable). `Effect::PutOnRecruitedIfSuccessful {
  life, lock }` (Ghouled, Lobotomy): `ActionCtx::put_on_recruited`, one
  entry each card, placed from the ash heap only as the recruit succeeds.
- `Terms::AnyAshPick(AshPick)` (Emergency Rations, Ghost-Eater, Kiss of
  Lachesis [TEM]): the heap, then its card (`FindCard`, or `AshVampire`
  with `PlayCtx::ash_crypt`), from `GameState::ash_picks`; masked with
  none by `Requirement::AshPickInAnyAsh`; gone by resolution, the card
  action is void (`card_void`). "Burned from play since your last turn"
  reads the acting player's `previous_turn` against `Seat::burned_minion_cards`
  and, for vampires and imbued, `Seat::burned_from_play` (both cards of a
  merged vampire).
  `Effect::RemoveAshPick`, `EatAshPick` (a hunt for the capacity or starting
  life), `AshEquipmentToActor` (resolved in `succeed`: from another's heap
  `put_equipment` at mode 0, free; from the player's own a found play with
  `PlayCtx::half_cost` handed to `equip_found_with_terms`, its reductions
  offered on the half). Earth-Feeder's `Effect::PutOnActorIfHuntSuccessful`
  places after `HuntAt` (`place_now`, `ModeLayers::Push`, a second not put on);
  a Diablerie card's target filter is read again in `resolution_void` (Fire on
  the Mountain's locked vampire); an action card's `UnlockForBloodIfSuccessful`
  pushes `after_resolution_unlocks` in `succeed` (Ahrimane Protectorate); Masai
  Blood Milk's granted burn action resolves `ActorLasting` in the `BurnCard`
  arm.
- `AshPick::MinionOrRetainer` (Vulture's Buffet: `RemoveAshPick` before
  `HuntAt`, `Effect::UnlockActorIfHuntSuccessful` read beside Earth-Feeder's
  placement) and `AshPick::BurnedSinceYourLastTurnOf(type)` (Resurrection
  [obe]/[OBE]): `Effect::AshMinionCardToPlay` takes from the heap named
  (`PlayCtx::ash`), "since your last turn" the player's, the card still its
  owner's, controlled by the player; a retainer the actor cannot have stays.
  Muricia's Call and Children of Stone [vis]: `Effect::SearchEmploy` pushes
  `Interrupt::RetainerSearch` (the library's retainers of the filter the vampire
  employs now, the hand's too with `hand`, `Engine::employable_from` over each
  zone less `off`, at the version its player chooses (`found_version`);
  `put_retainer` as an employ action, `ActionCtx::is_employ_action`).
  `employable_from` reads the retainer's clan, path, sect and title (Crypt's
  Sons, Twisted Bloodhound), never the requirements needing a target. Patrizia
  Giovanni's `Effect::EmployFromOwnAsh` names the retainer as the action is
  declared (`Interrupt::NameAshRetainer`, `Engine::ash_retainers`: the ash
  heap's, the action's blood included), employed at success if still there
  (`retainer_found`, no library shuffled); her printed action, so not the same
  action as an employ from hand. Spirit Marionette [OBE]:
  `Effect::TakeControlOfTargetUntilNextActionEnd` sets
  `Minion::temp_until_action_end` (2 when taken during an action, aged to 1 as
  that one ends; back as the next action ends, any minion's,
  `Engine::next_action_ended` from `end_action`, or an action card is canceled,
  `settle_canceled`; else with the turn; rules-qa 124.218) and
  `GameState::forced_bleeds` (`mandatory_actions` offers only its bleeds of the
  taker's prey, `forced_bleed_option`, unless it must hunt; beside a must-hunt
  hunt, `free_action_options`' starving branch offers both); [obe]'s
  `Effect::LockMinionOfPrey` pushes `MinionPurpose::LockReady` as the bleed
  resolves. Travelers Obey the Tenets:
  `Modifier::DamagesSuccessfulDirectedVampires` adds to `damage_after` beside
  Shemti's. An action card's own "do not replace until …" (Pandora's Whisper) is
  held at its declaration as a play's is.

- Round 32 (modifiers, reactions): The Grandest Trick's
  `Effect::ActsAsMortalAlly` sets `Minion::as_mortal_ally` for the action
  (`is_ally`, `is_mortal` true, own Discipline levels 0, cleared in
  `release_action_scope`), offered on `Requirement::ActionAllyCouldTake`.
  High Aye's `DiscardUpToIfSuccessful` waits in
  `ActionCtx::discard_if_successful` for after resolution and its combats.
  Conscripted Statue [VIS]'s `FightsBlockerAsAlly(Body)` puts its card in
  play as an ally and sets `ActionCtx::framed`/`statue`: the framed combat
  gives it a press and burns it at the end (`burn_allies_at_end`), at once
  if that combat cannot occur. Gift of Bellona [VAL]'s
  `EquipActionWeaponNow` equips the blocked equip's weapon
  (`action_weapon_cost`: requirements, printed reductions only), the card
  entered play, `ActionCtx::idle_weapon` barring its use in round 1. Echo of
  Harmonies [MEL]'s `StoreActionCardOnActor` sends the political card
  bound for the ash heap (`political_card_to_ash`) to the vampire's
  `Minion::stored` in `place_action_card` (`ActionCtx::store_card_on`).
- Round 32's block bars: `Effect::CannotBlockThisAction {
  allies: AllyBar, vampires: VampireBar }` (`ActionCtx::class_bars`, read
  in `blocker_barred`). `Engine::fail_block` (Phantom Speaker [mel]'s
  `BlockFails`): the block undone, the blocker unlocked as before and
  barred, the action continued; Those Who Endure Judge's
  `YoungerBlockerTollOrFails` is paid in `blocked` before anything of the
  block, or the attempt fails (`younger_blocker_toll_fails`).
  `AllMinionsThisAction` (Peacemaker): `ActionCtx::all_bleed` in
  `bleed_amount`, `all_strength` in `strength_now`, `no_weapons` in
  `ability_gate`. `ActingVotesThisAction`: the referendum's `bonus` at its
  start. Transfusion's `SeatCounterOnLastOpponent`: `Side::inflicted_combat`
  copied to `ActionCtx::last_combat_inflicted` as a combat ends,
  `Minion::transfusion` per Methuselah (`MarkerKind::TransfusionCounters`).
  Massassi's Honor's `HandOffAction`: `GameState::handed_off`; while one is
  owed, `seat_action_options` keeps only the matching options of
  `free_action_options` (`is_handed_off`; `dues` names it), `settle_handed_off`
  burns it if none matches, `declare_action` marks it begun, `drop_action` burns
  the card at that action's end; the action card goes onto it
  (`ActionCtx::store_card_in`). Winged Second's `JoinAsSecond`: `Side::second`.
  In the normal pair, its side's strike chosen, `Engine::second_turn` swaps a
  fresh `Side` of the second into `sides[i]` (the side's own and the range held
  in `CombatCtx::second_swap`, the range read close) and asks
  `second_strike_request` (pass, hand, a borne melee weapon's, a hand or melee
  strike card); chosen, the side is put back and the second's kept in
  `Side::second_strike`, its hit read with it swapped in again as the strikes
  resolve (a dodge avoiding it). Readers outside combat.rs use
  `combatant_side`/`second_side`/`combat_range`, never `sides` during the swap.
  Its targeting "as a retainer could be": Target Retainer's third mode
  (`Requirement::TargetOpposingSecond`) and `with_retainer_targets` set
  `Side::strike_at_second` (`aimed_aside`), its damage resolved out of combat
  after the strikes (`second_hits`).

- Modifiers and reactions (round 33): `Effect::DamageActingAndRetainers`
  (Bloodstorm of Chorazin) damages the acting minion and each retainer on it
  through `Engine::damage_with_retainers`; an Ilomba on a vampire whose fate
  the order decides asks its controller (`Interrupt::RetainerDamageOrder`,
  LSJ 20090617-2); `damage_retainers_on` is Weather Control's retainer loop
  too, each point an Ilomba's host pays. `ActionCtx::library_per_pool_lost`
  (Dirty Little Secrets) burns library cards per pool the bleed's target
  itself loses; `corrupt_if_successful` (Weigh the Heart [AUS][SER]) asks a
  minion of any directed seat after resolution
  (`MinionPurpose::CorruptAmong`); `resolved_modifiers` lists the modifiers
  resolved, not canceled (Permanency [chi], `Effect::ResolvedModifierToHand`).
  `Modifier::HostBleedAnnounceTollVsController` (Tainted Vitae) adds to
  `Engine::announce_toll` for a bleed at the card's controller;
  `ByInterceptVsBearerBleedingYou` (Sins of the Cauchemar) is read in
  `host_intercept`; `OnlyDiablerieWhileAble` (Undying Thirst) leaves its
  bearer only the diableries `Engine::thirst_victims` lists, a hunt due
  beside them.

### 6.6 Block loop

`actions.rs`.

- Eligibility: prey/predator/target rule (`eligible_blockers`), widened per
  attempt by `can_attempt_block_anyone` (Eagle's Sight: one attempt, not the
  acting Methuselah's own minion nor a decliner nor a barred blocker);
  narrowed by the bars in `Engine::blocker_barred` (Ravager's "only
  changelings" on undirected hunts: `only_subtype_blocks_undirected_hunts`;
  Force of Personality's allies of one subtype only,
  `ActionCtx::allies_block_only`; Rabbat's `Stat::CannotBlockUnlessAimedAt`,
  `ActionCtx::aims_at`: a minion it targets, a card on it targeted, burned
  or stolen), read live: as attempts are offered (`block_candidates_locked`,
  with the minion's own state: torpor, lock, toll) and against the attempt under
  way after each action effect (`apply_action_effect`), which then fails barred
  (Daring the Dawn [FOR] into a vampire's attempt, rules-qa 92.1).
- A combat a block begins may be canceled by the acting vampire
  (`ActionCtx::cancel_combat_if_blocked`, Stone Travel [MYT]): the
  `Interrupt::MayCancelCombat` Foresee also asks, `Choice::CancelCombat`;
  the action stays blocked.
- Forced attempts (`Effect::AttemptBlock`, `R::CanAttemptBlock` →
  `can_attempt_block`). A card-made failure bars re-attempts this action.
  A locked one's, not woken (Open Dossier: `Effect::AttemptBlockLocked`,
  `R::CanAttemptBlockLocked` → `can_attempt_block_with`): its
  `ActionCtx::locked_attempt` spares it the locked-out failure of that one
  attempt; it plays no other reaction as if unlocked (LSJ 20080605). A
  successful block's combat canceled by a reaction (`Effect::CancelBlockCombat`,
  `ActionCtx::block_combat_canceled`: Hard Case, Bear-Baiting): blocked, no
  combat.
- A change of blocker (`ActionCtx::changes_blocker`, rules-qa 48.3): during
  its own minion's attempt, short of the stealth, a Methuselah may attempt
  with another, declared or forced by a card (`AttemptBlock`,
  `TargetAttemptsBlock`); the attempt under way fails then
  (`ActionCtx::attempt`), one attempt at a time.
- Tolls (`block_toll`): paid as the attempt is made; an attempt under way when a
  toll lands pays or resigns (`Interrupt::BlockToll`). A forced attempt
  (`can_attempt_block[_anyone]`, `AttemptBlock`, `TargetAttemptsBlock`) needs
  the toll payable (rules-qa 42.8).
- **The block moment** is a query too, `Engine::block_moment(seat)`: its own
  minion attempting, or every would-be blocker before it declined; the
  host's opportunities filter shows a wake only then while blocks are open.
- **Every legal attempt offered** (rules-qa 73.8): one that cannot reach the
  stealth too. **Reachability** is a query, `Engine::block_can_reach(seat, m)`
  (`intercept_reachable`), read by the host's opportunities filter (it hides a
  futile attempt with them off, `crates/host` `Opportunity::Futile`), by the
  scripted bots (`zvtes_env::bots`: a futile attempt scores below the pass), by the
  network bots' mask (`zvtes_policy::reachable_blocks`: never made, a
  futile-only window passed, in training and on the server) and by
  the record reader (`Replay::elided`: a record from an engine before 73.8
  never had the futile-only block windows, elided then); never a mask. It
  counts the blocker's own and printed intercept, intercept abilities of cards
  on it, other cards in play (`location_intercept_reach`: Market Square, KRCG
  News Radio, The Rumor Mill, an ally's own lock: Political Antagonist), stealth
  abilities in play can take off (`stealth_reduction_reach`, rules-qa 66.2), and
  answers yes while a non-acting Methuselah could strip the stealth
  (`stealth_strippable`): the blocker's own Methuselah by a playable card in
  hand, another by holding any card while one of their minions could play a
  stealth strip of the pool (Night Terrors, Scobax [str]'s -1 during the action)
  — their hand is hidden from the blocker (DESIGN §4.3). An intercept card in
  hand also counts its replacement draw (rules-qa 48.5: one to play and a
  library may yet meet any stealth); conditional intercept counts whether or not
  its condition holds now. The rule itself is the closure comparison in
  `action_advance`. Any new third-party intercept/stealth-strip card must be
  counted here, or the host hides a live attempt.
- **Failed attempts** (`ActionCtx::attempt_print`, `failed_alike`,
  `failed_attempts`, rules-qa 48.6): a minion that failed an attempt is not
  offered the same attempt again until what it faces changes, in public
  terms (a hash: the stealth; its intercept, blood and lock; the count of
  cards in play; every hand's size; its Methuselah's library and ash heap
  sizes). Hands themselves are never read (`tests/elision.rs`). A barred
  attempt is recorded there too: Gracis Nostinus and Gwyedd read it as
  failed.
- The engine has legal unbounded repeats here (block re-attempts, DESIGN
  §11.3): cap decisions in any exploratory driver.
- A joint attempt (`Effect::JointAttemptBlock`, Coterie Tactics): one
  attempt, `ActionCtx::joint` its second vampire, `intercept_of` the sum
  (`own_intercept` each), `ActionCtx::attempting` for either one's
  intercept card; a bar, lock or absence of either fails it whole
  (`record_failed_with_joint`). On success both lock (`joint_unlocked`
  keeps who was unlocked), then `MinionPurpose::JointBlocker` picks the
  blocker (`Engine::joint_blocked`): the other is `ActionCtx::joint_blocked`,
  read as having blocked (`ActorHasBlocked`, LSJ 20090309) by `LockBlocker`
  and `BlockerNotLockedForBlock` too. A Coterie Tactics in hand counts its
  best partner in `intercept_reachable`.
- A blocker that does not lock for the block, played after it locked
  (`Effect::BlockerNotLockedForBlock`: Minor Irritation): left as it was
  before the block (`blocker_was_unlocked`), not an unlock.
- Block-time riders for both combatants (`Effect::BlockRiderBoth`: Bar Último
  Trago's no equipment and `BlockRider::EnvironmentalFirstRound`), an attempting
  minion's from another card (`Effect::IfAttemptingBlocks`: Destilaria), and a
  stand-in fighting the blocker, the acting minion out (`ActionCtx::stand_in`:
  Iara).
- `put_on_acting_if_blocks` (Melange [AUS], Darksight [OBT]) places only a
  card still in its ash heap as the block lands (ANK 20220805: removed,
  no effect, no stand-in). Darksight's
  `Modifier::BearerStealthBleedingPlayer`: `GameState::bleed_stealth_against`
  in `ActionCtx::stealth`, the target the controller of the card's `by`
  minion, read live. Command's `Requirement::AttemptingAlly` and
  `Effect::PutOnAttemptingNow` (the card itself on the attempting minion,
  before `AttemptFails` clears the attempt).
- Ohoyo Hopoksia's `Modifier::TakesCombatOfOwnMinion`: as a block's combat would
  begin, `Interrupt::CombatStandIn` (`ActionCtx::stand_in_asked`) lets its
  controller lock the ally to fight in the blocker's place
  (`ActionCtx::blocker_stand_in`), the block standing; her combat is not the
  block's resulting one: the block riders, Dawn Operation, an "if blocked"
  cancel, a toll and `held_at_block` lapse (rules-qa 124.154). Ghoul Messenger's
  granted action: `BurnSourceIfBlocked` and the marker `CancelCombatIfBlocked`
  read as it is declared (`ActionCtx::no_combat_if_blocked`: no combat, the
  blocker unlocked). Swarm's marker `OnActingIfBlocked` puts its employ card on
  the acting minion as the block lands (`put_retainer`, outside the action,
  unpaid). Redcap Wilder's toll: `GameState::small_vampire_block_toll` in
  `block_toll`.
- Block-bound bearers (round 33): `Modifier::BlockCombatOpponentNoEquipment`
  bars the other side's equipment in the block's resulting combat only
  (Spiritual Protector; TOM 19951208-2); `BearerMayCancelBlockCombat`
  queues one `Interrupt::MayCancelCombat { burn: Some(card) }` a copy
  (Ghoul Escort: burned, unlocked, the action blocked; none blocked leaving
  torpor); `BearerBlocksAndReactsOnlyWhenTargeted` masks the bearer
  vampire's block (`blocker_barred`) and reactions unless the action is
  directed and targets it (Polaris Coach); `BlockingVampireBurnedAfterCombat`
  burns a vampire blocking the bearer after the combat, else as the action
  ends (`burn_minions_at_end`: Caiaphas Smith).

- `Hook::VampireBlocked` (Dr. Marisa Fletcher, CDC) fires with the "if
  blocked" riders for any Methuselah's acting minion
  (`Requirement::ActingCapacityBelowCounters` read on the rider's card).

### 6.7 Combat

`combat.rs`: `CombatCtx`, `Side`, `Step`, `Damage`, `Held`.

- Round steps (rulebook "Combat Sequence"): before range → determine range
  (alternating maneuvers) → before strikes → strike choice (acting first) →
  before resolution (spec §4.3.2) → resolution (combat ends first, else
  simultaneous; steal before mend) → prevention (acting side first, spec
  §4.5.4) → damage at once → additional strikes → press (alternating) → end
  of round, shared with the would-end replacement point (spec §4.9.1). Each
  ordered step is a window with impulse reset (rules-qa 1.2) polling every
  Methuselah: the acting one, the defending one, then the others clockwise
  from the acting one (`CombatCtx::ring`, `poll`; rules-qa 98.1); a
  combatant's own options (maneuver, press, Show of Force, Monomancy's
  `Choice::EndCombat` from `Side::end_option`, set by
  `Effect::OpposingMayEndBeforeRangeTakingCard` as the action's combat
  starts: the card put on, then `end_combat_now`) its seat's
  only; strike choice and prevention are single-seat offers, the others'
  prevention (Martyr's Resilience) the ring's order (29.1). A non-combatant
  is offered what its cards allow one outside the combat: a vampire's or a
  card's ability only when its actor is `HostOutOfCombat` (Randall, Maman
  Boumba, Abjure; 15.2, 98.2), or `HostAnyCombat`, offered there to its
  host in the combat too (Draeven Softfoot). A combatant no longer ready
  ends the round at once, the end-of-round window still opening (rulebook
  step 7). Its Methuselah ousted, by a combat effect or a cost (a play's
  out of turn, `oust_if_out`), ends the combat at once, no end-of-round
  window (`combat_ousts`; DESIGN §10.8, rules-qa 45.4). A card there put
  on the opposing minion carries `Requirement::OpponentPresent`: a burned
  one is no target (rules-qa 75.1).
- Only combat cards are played during combat — no modifiers, reactions or wakes
  (spec §4.1.3). A combat is part of its action: `Engine::action` and its layers
  stay in force.
- Strikes: a weapon strike remembers its source (`Side.strike_source`) and does
  nothing if the weapon left; a maneuver from a weapon or strike card commits the
  initial strike (`Side.committed`, `committed_card`), a committed weapon's
  strike made through any play or ability whose `WeaponStrikePlus` names it
  (rules-qa 87.42); the round's one limited
  additional-strike source; a strike not said to be ranged is close range only
  (rules-qa 21.1). Strike riders fire as the strike lands (in range, not dodged,
  not aimed at a retainer). Ranged strikes at long range may be aimed at a
  retainer (`Side::strike_at`). A strike naming the kind it acts on carries
  `OpponentVampire`/`OpponentAlly` (Coma, Entombment: rules-qa 76.11). A
  strike putting its own card on its striker (`Side::held_self`, Molotov
  Cocktail) places it once the strike resolved: a dodged or voided strike
  resolved, without effect on the dodging minion; one combat ended before
  never did (rules-qa 76.10, 122.62); `CombatCtx::placed_this_round` holds
  the "not this round" bar (`CombatCtx::stolen_this_round` a stolen
  equipment's, for every ability of it, in `ability_gate`: rulebook "Steal
  Equipment"), `Effect::BurnAfterStrike` the weapon's burn after a resolved
  strike. A damage strike's damage on the opposing minion before prevention
  (strength and its bonuses, the opposing minion's reductions, the aggravated
  sources) is one computation, `CombatCtx::strike_hit`: the resolution reads it,
  and `strike_told` (adding a harmless weapon's nothing, a weapon-immune
  target's too, and Dawn Operation's aggravation) serves Rötschreck
  (`aggravated_strike_on`), the host's view of a declared strike, and
  `CombatNote::Strikes` (`told`, taken before resolving clears the strike's
  riders). A strike aimed at a retainer is resolved apart. Melee weapon damage
  from cards in play (Blessing of the Name's chosen, Blood Tempering on the
  weapon) is `GameState::melee_damage_bonus`, none on a strike dealing none; an
  equipment's prevention abilities prevent what the cards on it add
  (`GameState::tempered_prevention`). Shadowed Eyes' `HostNoRangedStrikes` bars
  in `Combat::allows` every strike usable at long range (`StrikeKind::ranged`)
  and a gun's maneuver committing to one.
- Potio Martyrium (`Modifier::HostMartyrdom`): its Gargoyle burned or sent
  to torpor by the combat's damage burns instead (`apply_damage_now`), and
  the opposing minion and its retainers take the aggravated damage as
  pending damage; `CombatCtx::martyrdom` sends `after_prevention_or_next`
  back to their prevention before the combat goes on.
- Charge of the Buffalo's `BlockRider::RangeFirstRound(Close)` sets round
  1's range as the combat begins and `InitialHandStrikeFirstRound(n)` commits
  the initial strike (`committed_card`). Presses cost a Decompose [THN] host
  more (`GameState::press_tax`): a card's through `card_surcharge`, an ability's
  in `ability_gate` (masked unpayable) and `resolve_ability_now`.
- Damage: prevention per side, acting side first, then third parties from outside
  the combat (`Step::OthersPrevent`); under a Fortitude-proof strike
  (`Side::no_fortitude_prevention`) no prevention requiring Fortitude, a
  card from hand (`play_options`) nor an ability of a card in play played
  with it (`ability_gate`: Ablative Skin; LSJ 19990216, 20050628); the preventer
  picks which damage a prevention covers (`Step::AllocateSource`); applied at
  once, normal then aggravated (the rulebook's examples are unit tests).
  Prevention "from the opposing minion's strike" (Rego Motum, Opikun, Lodin) is
  `Effect::PreventStrike` behind `Requirement::StrikeDamagePending` (or
  `StrikeNormalDamagePending`): it reads `Side::strike_damage`, never the
  environmental or effect damage beside it. Environmental damage lands
  in the normal strike resolution. Unpreventable damage (`Side::unpreventable`)
  stays out of `pending`. A damage resolution is atomic, no window opening
  midway (PIB 20121031): an untimed lock ability (The Coven) is offered in
  the prevention window, before damage lands. Anathema's "blood reduced to 0
  in combat" is read as each step settles (`Engine::settle_anathema`).
- Going to torpor: after the preventions, a vampire the damage as it stands
  sends to torpor opens `Step::GoingToTorpor(i)` once per side
  (`Side::torpor_offered`; `Window::CombatGoingToTorpor`, label
  `CombatPrevention`), opened for and offering only the plays and abilities of
  that window (an any-impulse ability is not: the prevention window just offered
  it; an ability used there asks the step again): Decapitate [POT] there sets
  `Side::burn_instead_of_torpor`; `going_to_torpor` / `going_down` read the
  pending damage (`OpponentGoingToTorpor`, `ActorGoingToTorpor`,
  `ActorNotGoingDown`): the victim of Liquefy [VIC]'s landed diablerie is going
  down, not going to torpor (rules-qa 113.155). Each side is asked, the acting
  one first, about either combatant; the first card played averts the torpor for
  the other (`going_to_torpor` false once burned, diablerized or persisting; RTR
  20010711). Undead Persistence and Undying Tenacity
  (`Effect::TorporAfterCombat`): `Side::persisting` keeps the vampire ready and
  wounded (its damage read as a torpid one's), `Minion::torpor_after_combat`
  sends it to torpor in `close_combat`; a new combat (Psyche!) carries the flag,
  its torpor due as that combat begins (`sent_to_torpor`, staged with the
  pre-range damage); `CombatCtx::quiet_rounds` ends the combat after
  `quiet_limit` rounds (Undead Persistence's three; none for Undying Tenacity)
  with no card played (`card_played_round`). Amaranth
  (`Side::diablerized_instead_of_torpor`) and Liquefy [VIC]'s strike
  (`CombatCtx::diablerie`, landed) diablerize as the damage lands
  (`apply_damage_now`), inside the combat (rules-qa 107.30, 113.10). Made,
  `CombatCtx::diablerized` holds it until the combat is at rest (no interrupt,
  oust or hunt burn pending, `Flow::Combat` itself): `settle_combat_diablerie`
  suspends the combat (`suspend_for_hunt`: `ActionCtx::hunt_in_combat` keeps the
  action's state) for the after-diablerie window and the blood hunt (§6.8);
  `resume_hunted_combat` puts the state back and `Flow::Combat` where it was,
  the victim gone ending it as any combatant leaving (an end-of-round window,
  then `end_combat`'s offers: Marie-Pierre may join against the diablerist). A
  combat ended before the suspension (a deferred oust) takes the steps at the
  top of `end_combat`, ending once they are over; an oust while it is suspended
  waits for `resume_hunted_combat` (`combat_ousts` reads no combatant's oust
  meanwhile). Liquefy's diablerie lands with the round's damage (its user ready
  before it, both present after), an Amaranth on its victim then diablerizing
  nothing. Each diablerie has its own hunt, a second Amaranth in a chained
  combat included. Byzar's burn replaced (`Engine::replace_burn` without the
  torpor) sets his `sent_to_torpor` and `CombatCtx::back_to_torpor`, the
  going-to-torpor step asked again, where another Amaranth may follow (ANK
  20210424). The step also opens for an opposing minion going down otherwise (an
  ally burned, a vampire burned), for what that moment offers: Lorenzo Detuono's
  `Effect::TakeOpponentInsteadOfGoingDown` sets `Side::taken_instead`
  (`OpponentGoingDownUnreplaced`; it averts the others, `going_to_torpor`
  false): `apply_damage` keeps the minion out of the burns and torpors (an ally
  at no life given its life) and, the round's damage done, `take_control` takes
  it for good, ending the combat (LSJ 20020618). Pentweret's
  `StrikeKind::StealOpponent` (`Side::steals_opponent`, its capacity-or-cost
  bound read again then) is taken there too.
- Combat ends: `Effect::EndCombat` goes to the end-of-round window; no new round
  once a combatant left the ready region; mid-combat ousts DESIGN §10.8.
- "Put this card on …" in combat: a card that says so as it resolves goes
  on its minion itself through `place_now` (Disarm on the opposing vampire,
  Wall of Filth, Focus the Blood: `ModeLayers::FromEffects`). A placement
  after the card reached the ash heap (on a hit, after combat: `Held`)
  takes the card itself back out of the ash heap into play (`place_held`:
  by its copy, else a copy of its name); only with none left there is it an
  effect-only card in play (`CardInPlay.effect_only`) carrying the counters
  and layers. Lost if the strike misses, combat ends first, or combat
  continues (rules-qa 21.1, 47.1). Only action cards wait in limbo. A
  lasting effect with no "put this card on" (Inner Essence's discount:
  `Effect::LastingOnActor`) is an effect-only card in play on the vampire,
  the card itself to the ash heap (rules-qa 68.12).
- Per-combat/round/action/turn ability uses key on the card copy: a card's
  abilities share one use (rules-qa 38.8, 62.32).
- Frenzy: `Effect::Frenzy` marks the frenzy mode; `Engine::frenzy_on` names the
  minion affected (rules-qa 28.11); `frenzy_immune` cancels and bars. A
  frenzy card choosing minions (`frenzy_on_picks`: Bloodlust's
  `Terms::XMinionPicks`) is played on each pick, not its actor; every reader
  (immunity, Meditative Grove's `played_frenzy_on`) takes `frenzy_on_all`.
- Weapons: melee `StrikeKind::Melee` (not a hand strike); a weapon's per-combat
  maneuver is `Limit::PerCombat` per copy; `cards::melee_strike`,
  `cards::becomes_weapon` (Weighted Walking Stick is a weapon only in play).
  "Use a weapon strike at +n" (Lucky Blow, Lam Into, Anticipation,
  Decapitate, Strike with Conviction) is one routine,
  `Engine::weapon_strike_plus`: the weapon's own strike ability
  (`cards::weapon_damage_strike`) resolved, its limit, riders and source
  standing, then `Side::raise_strike`; its target
  (`Requirement::TargetWeaponStrikeUsable`) holds only while that strike's
  gate does, and the strike bars (frozen, hand strikes only, no strike
  cards) read `Effect::WeaponStrikePlus` as a strike.
- Aims (Target Vitals) come after the strike choice: a side whose strike
  is chosen gets one more `ChooseStrike` decision, an aim or pass
  (`Side::aim_declined`; rules-qa 62.1), an ability requiring
  `R::NoAimYet` offered there too (Mikael Birkholm); one aim each strike,
  a canceled one included (`Side::aim_played`, LSJ 20100511). Mikael's
  `Effect::PlayFirstLibraryAim` sets `Side::library_aim`: the virtual hand
  offers it (`state::LIBRARY_PLAY`, played from the library as if from
  hand, not replaced) as the seat's forced next play
  (`Engine::library_play_due`, Spiritual Guidance's shape), or, its
  requirements unmet, spends the aim. Reality Mirror's `Seat::mirror_play`
  offers the opposing minion's controller's ash heap the same way
  (`state::mirror_of`, `Engine::ash_play_due`), the card staying its owner's
  (`played_card_owner`, not removed when burned), a missing basic Discipline
  lent (`cards::met_lending_one_basic`, one of a combo card's). Both carry
  every combat window but the as-played one and are timed by the card they
  play: `play_options_by` offers them only where a play from that ash heap
  exists now (`ash_play_offered` probes `virtual_hand_probing`, a copy of
  the card itself not counted; no legal no-op, rules-qa 122.215); at the
  aim decision, aims only (`ash_plays_fit`, shared by `ash_play_due`); a
  strike card played canceled leaves the strike choice owed. Damage reduction
  is not prevention: `GameState::damage_reduction(m, from)` (Nephandus,
  `Modifier::OpposingDamageLess`; to 0 against a Baali, Saulot's
  `Stat::NoDamageFromOpponent`; to 0 on an immunity, `GameState::damage_immune`,
  the strike inflicted all the same: `CombatCtx::immune_hit`) lowers each
  opposing strike's damage, and a minion's own non-strike damage (Pulled
  Fangs), before it is pending — unpreventable and after-combat strike
  damage included, environmental not. A vampire in torpor is wounded
  (rulebook "Wounded"): `resolve_damage_healing`'s `in_torpor` has him mend
  normal damage, each aggravated point burning 1 blood or him, and reports
  only a vampire newly wounded. Its `heal` is the points one blood mends,
  `GameState::heal_per_blood` (Regenerative Blood's 2, `HostHealsPerBlood`).
  `Side::keep_normal` keeps a never-aggravated strike's damage (Psychic
  Assault) normal under Dawn Operation. `StrikeKind::Corrupt` (Contagion)
  adds the striker's counter as it lands and queues the corruption steal,
  which ends the combat when taken.
- A "combat ends" strike resolves before the other strikes (no damage
  beside it); `Effect::EndCombat` on a played card ends the combat before
  range (Alpha Glint [ANI][FOR], Weather Control's damage first).
  Ossian's first-round freeze is `Side::frozen_round`, set by
  `Engine::freeze_auspex_foes` as the combat begins and as each card is
  played, read by `CombatCtx::allows`, `may_maneuver` and `begin_pair`.
  `may_maneuver` reads the state live: The Crusader Sword's bar on an
  imbued's opposing vampires (`Modifier::OpposingVampiresNoCloseManeuver`),
  `Stat::NoManeuverToLong` (Pedro Cortez, Mowgli) and the opposing vampire's
  `Stat::OpponentsNoManeuverToClose` (Ysador the Foul), for every maneuver
  source: the granted one, a card's, a strike card's at the maneuver.
  The round's imbued cards: `Side::imprisoned` (copies adding their toll,
  `toll_for`), `reflects` (Inflict), `burns_electronic` (Smite),
  `cleaved_hit` (Cleave's damage past an immunity; `ActionCtx::cleaved`, the
  +1 in `strike_hit`), `expiate` (`expiate_burns` as a press step begins,
  `CombatCtx::prev_close`; the conviction as the combat ends).
  `Modifier::AdditionalStrikeEachRound` (Renegade Garou) and the printed
  `Stat::AdditionalStrikeEachRound` (Lorrie Dunsirn) add a mandatory
  strike after the first pair, in the limited slot;
  `AdditionalStrikeOnceEachCombat` (Black Spiral Buddy) the same, the first
  round it can (`Side::once_strike_taken`). Torrent's "continues as
  if unblocked" is `Side::continue_free` → `ActionCtx::continue_free`, taken
  as the after-combat window closes; it and Form of Mist's
  `ActionCtx::continuation` are dropped together
  (`ActionCtx::drop_continuations`) if the combat goes on or another follows
  it.
- Psyche! [CEL] queues a combat as one would end (`CombatCtx::psyche`):
  `end_combat` releases the after-combat held cards, drops the ended combat's
  after-combat effects, its continuations included, and begins the new one with
  the same routing flags (`at_resolution`, `after_resolution`, `queued`,
  `yawp`), no `held_at_block` (rules-qa 65.1; `begin_psyche_combat`). A new
  combat with the action's acting minion waits first in
  `ActionState::QueuedCombat` (`ActionCtx::psyche_next`), a window where "about
  to enter combat with" reactions are played (Obedience, Promise of 1528:
  rules-qa 88.10, 88.11); elided when none can be.
- Flames of Insurrection's offers come as a minion opposing an Anarch leaves
  the ready region, to both controllers when both Anarchs leave
  (`Engine::flames_offers`).
- First strike: printed (`Stat::FirstStrike`) or a card's
  (`Side::first_strike`, Anesthetic Touch [OBE]) holds the opposing
  non-dodge strike back (`CombatCtx::held_strike`) until its damage lands;
  a card's first strike that burns or steals equipment (Donnybrook, Canine
  Horde, Fast Hands, Durga Syn) resolves apart. `StrikeKind::StealEquipment`
  takes what its target filter allows (a weapon for Fast Hands and Spider,
  any equipment for Durga Syn); `Effect::takes_card` counts it, so a unique
  copy its player controls is no target. `Side::ends_combat_after` (Anesthetic
  Touch [obe]): after the strike damage lands the round is over, the held
  strike dropped. `Side::long_maneuvers` and `combat_continue_presses`
  are combat-long allowances carried by `Side::new_round` (Repulsion
  [pre], Harass).
- Damage in the end-of-round window (Pulled Fangs' `DamageOpponentNow`):
  pending, a prevention step, then the window again
  (`CombatCtx::end_of_round_damage`); on a vampire sent to torpor that
  round too (wounded, above); one burned is no target (`OpponentPresent`).
- The Book of Going Forth by Night: a standing burn replacement
  (`Engine::burn_replacements`, §6.11) of an ally opposing a bearer of
  `Modifier::HostStealsOpposingAllyOnBurn`, `book_takes_ally` (stolen,
  locked, 1 life; the round over, no press).
- Environmental damage (`Effect::EnvironmentalDamage { n, close }`,
  `Side::environmental_close`): dealt in normal strike resolution each
  round, at close range only if `close` (Dark Steel). `Side::no_dodge_round`
  bars the opposing minion's dodge, or its combat-ends strike, this round
  (Sanguine Entrapment). A ranged strike's bonuses (`combat::ranged_bonus`:
  `Stat::RangedStrikeDamage`, `RangedStrikeSteal`, The Dracon) are read by
  `CombatCtx::strike_hit` and the retainer arms alike; the damage added to
  any damaging strike (`combat::strike_plus`: `Stat::StrikeDamage`, Harry
  Reese; `WeaponStrikeDamage` on a weapon's, Janni) likewise.
- Before-range damage (`Side::pre_range`, Outside the Hourglass [TEM]) is
  staged by `stage_pre_range_damage` as range opens and by
  `end_combat_now`, aggravated under Domain of Evernight; that round's
  Weather Control damage joins it in the same window (`CombatCtx::
  weather_staged`, added after prevention; `weather_round` keeps it from
  landing twice). `Side::combat_strike` (Mythic Form's strike once each
  round), `round_normal_prevents` (its and Typhonic Beast's prevention each
  round), `Effect::ShieldFromOpposingStrike` (after `resolve_strikes_noted`)
  and `Effect::OpposingCelerityMayBurnToPrevent` (Stunt Cycle, through
  `Interrupt::MayPreventForBlood`). `Side::combat_damage` counts the damage
  taken in the combat for `Stat::BurnedOverCombatDamage` (Franciscus).
  Portia's environmental damage of a blocked hunt
  (`Effect::EnvironmentalDamageBeforeRange` → `ActionCtx::
  blocked_env_before_range` → `Side::pre_range_environmental` as the block's
  combat begins) is staged with `pre_range`, preventable, no reduction;
  Petaniqua's `Stat::EnvironmentalBeforeRangeAfterFirstRound` adds to it in
  `Engine::new_round` (every round from the second).
- Improvised Flamethrower (`Modifier::BackfiresAtLong`, `combat::backfire`):
  each copy on a minion struck at long range (`Side::strike_damage`,
  prevented or not) or hit by pre-range damage with the range set long
  (`CombatCtx::range_set`) adds its aggravated damage to its bearer's own
  (`Side::self_damage`) and burns.
- NSA Trio (`Modifier::NoCombatEndsFirstBlockCombat`): `begin_combat` marks
  the turn's first combat of an acting vampire and its vampire blocker
  (`GameState::vampire_block_combat_turn`, in play or not), barring both
  sides' combat ends in round 1 (`Side::no_combat_ends_round`). Hidden
  Lurker queues its combat with the blocker as Fast Reaction's with the
  acting minion (`QueueCombatAfterBlock { with_blocker }`).

- Dam the Heart's River: `Side::more_damage_taken` is added where each
  kind of damage to that minion is counted — `Side::strike_hit` (a
  damaging strike only), the retainer, environmental and resolution
  damage in `resolve_strikes`, the after-combat damage queued there
  (Catatonic Fear; Mariel's `Effect::CombatantsDamageAfterCombat`, its
  retainers' share in `CombatCtx::after_combat_retainer_damage`), and
  `damage_out_of_combat` for a combatant (Botched Move): a new damage site reads
  it too. The Remnant's cap per effect (`GameState::damage_cap`) is read at
  `strike_hit` and `damage_out_of_combat` only (rules-qa 109.20). Its strike
  toll: `strike_toll` (a card's in its cost, `strike_card_toll`), `toll_due` on
  the strike choices and `pay_strike_tolls` before resolution; a toll the minion
  cannot pay leaves `strike_request` only `Pass` and strike cards. Lucian's
  printed toll on strikes against him (`Engine::printed_strike_toll`) is read in
  `strike_card_toll`, `pay_strike_tolls`, `strike_request` and its declining
  `Pass`: a new toll site reads it too. A strike card committed at the maneuver
  pays its tolls past its reductions (`maneuver_strike_toll`, LSJ 20090529); one
  played as the strike paid them in its cost, no `toll_due` after. Strike bars
  for a combat or a round (Lapse's
  `OpposingNoManeuverThisRound`/`OpposingNoInitialStrikeThisRound`, Thoughts
  Betrayed's `OpposingNoStrikeCardsThisCombat`), the strike's card and mode
  (`Side::strike_play`: Increased Strength's Potence strikes), Rockheart's
  `halve_strikes`; per-round extra damage adds up (`environmental_round`,
  `resolution_damage`).
- The acting side: `sides[0]`, ordered as the combat begins by
  `GameState::combat_order` (Nar-Sheptha's `Modifier::OpponentActsInCombat`
  over Deep Song's order) from `CombatCtx::natural`, the pair as it began
  (what queues a new combat starts from it: Psyche!, `last_combat`).
  `reorder_combat` re-reads it as each step naming no side advances, after
  each pair of strikes and at `new_round` (Nar-Sheptha gone, at once),
  everything indexed by side swapping with the sides. "The acting minion
  in combat" reads `sides[0]` in a combat of the action's acting minion
  (`CombatCtx::acting_in`: FBI Special Affairs Division, Powerbase:
  Savannah), `ActionCtx::acting` otherwise. Blight's `Side::strength_minus`
  and `no_additional_strikes` outlast the round (a card on the minion,
  Decompose's `HostNoAdditionalStrikes`, read live: `Side::no_additional`);
  Serpent's Numbing Kiss's `after_put_rider` becomes
  `CombatCtx::after_combat_put`, placed (and the striker's blood burned,
  the minion locked: the effect's `blood` and `lock`, none for Morphean
  Blow [val]) as the combat ends, lost if it continues (`discard_held`).
- Offers as a combat ends (`end_combat`, resumed by their answers): Free Fight's
  swap first (`offer_free_fight`, only once a press continued it), Haven
  Hunting's new combat (`Interrupt::MayNewCombat`, `c.psyche`), Marie-Pierre's
  entry (`Interrupt::MayJoinCombat`, `c.coordinate` with `c.joined`). Each is
  asked once (`free_fight_ending`, `again_asked`, `join_asked`).
- `resolve_strikes` queues what lands that needs the engine
  (`CombatCtx::landed`, applied by `apply_landed`): Liquefy's burn, Elixir's
  stand-in. Liquefy [VIC]'s strike is a stand-in's ability
  (`CombatCtx::stand_ins`); its diablerie is made in the combat, as
  Amaranth's (above).
  Topaz's swap waits on `Interrupt::SwapEquipFromPrey` before the
  equipment enters (`equip_from_hand`).
- Blood Turnip moves as a combat begins (`turnips_move`); clot counters
  (`Minion::clots`) move on hand or melee damage (`Interrupt::MoveClot`) and
  bite in `start_unlock_effects`.
- In combat: a damage strike's retainers hit (`Side::retainer_strike_damage`:
  Hálito Negro); a strike card held to the round's end, put on the opponent at
  close range with more damage inflicted, settled as the end-of-round step
  closes (`Side::round_end_put`: Chave de Braço); a card-effect strike that
  stuns as it lands (`Landed::Stun`: Father Anthony); a once-a-combat corruption
  as a combatant's strike damage lands (`Stat::CorruptOnDamageOncePerCombat`:
  Elie Sansaricq); unpreventable damage on the minion opposing the user's
  combatant (`Effect::DamageOpposingOwnCombatantNow`: Ron Zertnert);
  aggravated-only prevention (`Effect::PreventAggravated`); printed riders as a
  combat begins (`Hook::HostEntersCombat`: Caique, Padraic Conroy's
  `BlocksOlder`).
- Round 28's combat cards: the card named with a strike taken as it lands
  (`Side::steal_card_on_hit`, Absorb the Mind [MYT]: a Discipline card,
  `GameState::card_discipline`; `Requirement::OpponentHoldsNoCard` masks
  the blood-only line while one is there); a hand strike reduced to zero
  (`Side::strike_zero`) ending the combat unless dodged
  (`CombatEndsAfterStrike { unless_dodged }`, Autonomic Mastery [DOM]); a
  canceled strike card's minion left with no strike (`OpposingGetsNoStrike`,
  Evil Eye [mal]); X limited hand-only additional strikes again after each
  normal pair (`Side::hand_strikes_each_round`, `hand_only_pair` read by
  `CombatCtx::hand_only` wherever `only_hand_strikes` is: Fractura); a card
  in play drained of 1 blood or life each round, before or after the
  strikes' steals as the acting Methuselah orders it beside a steal strike
  (`Interrupt::DrainBeforeSteals`), half back as the combat ends
  (`CombatCtx::drains`, Darkness Within; settled before a new combat
  begins); unpreventable damage each close round
  (`Side::resolution_damage_close`, Blissful Agony [val]); the opposing
  vampire entering combat with a chosen minion, going first, as an undodged
  combat ends strike ends the combat (`CombatCtx::redirect`, Blissful
  Agony [VAL]: lost if the combat continues, read by `NoCombatQueued` and
  the new-combat offers; played by both, the acting Methuselah picks the
  one resolving first, `Interrupt::RedirectFirst` setting
  `CombatCtx::redirect_first`); an area strike (`StrikeKind::Area`: the
  opposing vampire or demon ally unless dodged, its demon retainers
  regardless, the striking vampire and its own demon retainers always, in its
  own pending; the striker's bonuses on each, `CombatCtx::area_damage`) and
  pool for a striker burned by its pair's damage (`Side::pool_if_burned`,
  Eye of Unforgiving Heaven). +n strength against a vampire of a sect from
  a card on the vampire is `Modifier::BearerStrengthVsSect`, read in
  `GameState::strength_against` (The Black Beast); `OpponentSectIn` is read
  at play too (`GameState::sect_in`).
- Round 28's combat cards, second chunk: X the copies already played in
  the combat (`Terms::NamePlayedThisCombat`, `Engine::terms_x` over
  `CombatCtx::played_cards`, read before the play is recorded: Groaning
  Corpse); an unpreventable strike's damage kept aggravated
  (`Side::unpreventable_strike`, `unpreventable_aggravated` beside
  `unpreventable` wherever it is read: Hexe [STR]); a press only to
  continue for blood, once a round (`Side::continue_press_blood`, the
  press's last resort: Lead Fist [vis]); `Choice::UnlockForBlood` in each
  of the side's impulse windows until the combat ends, a new round
  keeping it (`Side::unlock_blood`, Loving Agony [VAL]); the opposing
  retainers' text off this combat (`GameState::retainers_silenced`, read
  by `text_suppressed`, cleared by `leave_combat`) and its close
  maneuvers and continue presses barred, no frenzy (`Side::restrained`:
  Olid Loa); `aggravated_as_normal` for the combat and a card name with no
  effect on the minion (`Side::immune_to`, read by `CombatCtx::immune_to`:
  its Flash Grenade strike barred, its bearer still stunned at close range
  and the grenade burned, its own one stunning it not: Penitent
  Resilience); landing, no more strike for the opposing minion this round
  (its `no_strike_round`, a first strike's held strike dropped), dodged,
  the striker's environmental damage and the opposing press (Pounce); the
  capacity gap strike (`StrikeKind::CapacityGap`, read in `strike_hit`,
  "capacity or cost" by `GameState::capacity_or_cost_by`, an ally's its
  `ally_cost`, its cap counting the recipient's `more_damage_taken`:
  Scourge of Alecto); a weapon's +n this combat (`Side::weapon_bonus`, in
  `strike_hit`'s melee arm beside `melee_damage_bonus`: Sword of the
  Righteous); the bearer's aggravated hand strikes
  (`Modifier::BearerAggravatedHands`, Spirit Claws).
- Round 28's weapons and minions: one strike a round for the card,
  whichever of its abilities (`Limit::PerRoundCard`, its `use_key(0)`); a
  maneuver committing one strike ability (`Effect::CommitStrikeOf`,
  `Side::committed_ability`, filtered in the committed strike request:
  Tinglestripe [MYT]); the round's initial strike's weapon
  (`Side::initial_with`, set as a weapon strike is chosen in pair 0), read
  by `Requirement::InitialStrikeIsOrMayBeThis` with the free-to-commit
  case (Joumlon's Axe, its cancel `PlayedLabeledByOpponent`, Sword of the
  Archangel's generalized); opposing vampires of small capacity barred
  in round 1 (`Modifier::FoesUpToCapacityNoStrikeFirstRound`, set as
  `no_strike_round` by `Engine::bar_small_foes_first_round` beside
  Ossian's freeze, at the combat's start and each play: Rock Cat). A
  weapon strike's own riders (`strike_rider`: the ability whose strike
  kind resolved): burned if any of its damage was prevented
  (`Side::prevented_watch`, read before immunity in
  `dagger_after_prevention` at the top of `apply_damage_now`), out of
  play once its damage is applied until the action ends, or the combat
  without one (`Side::out_after`, `ActionCtx::return_at_end`,
  `CombatCtx::return_at_end`, `Engine::return_out_of_play`: back on its
  bearer, burned with it gone); one Cleave would burn too is ordered by
  the active Methuselah (`Interrupt::ReturnBeforeBurn`, LSJ 20100211-2:
  Dagger). `AggravatedAgainstClan` reaches a ranged strength strike too.
- Healed aggravated damage (Repair the Undead Flesh): an ability in the
  prevention step (`Effect::HealAggravatedForBlood`), one point a use, out
  of `pending` into `Side::healed` (landed, not prevented); a weapon's
  `MendingCost` multiplies it for its aggravated points
  (`Side::costly_aggravated`).
- Range setters before the first round, in order, the first setter's
  (`!range_set`, PIB 20120214): the action's block riders as pushed (Gang
  Tactics and Storm Sewers' `CloseEachRoundIfBlocked` push
  `BlockRider::RangeEachRound(Close)`, setting `CombatCtx::each_round_range`
  for every later round too), then the blocker's own (Asanbonsam Ghoul's
  `HostBlocks` rider `Effect::BlockedCombatRider`, pushed by
  `host_blocks_riders`), then whatever sets in combat. A later round:
  `each_round_range`, else `next_round_range` (Ananasi Vampirephile's
  `Effect::SetNextRoundRange`: `Interrupt::MaySetNextRange { may: false }`,
  asked only while `CombatCtx::next_range_open`: no earlier setter, no
  `each_round_range` nor `default_range` overriding it).
  Neutral Guard [obe]'s `Effect::ActingRangeEachRound` (`ActionCtx::
  acting_range`) sets `CombatCtx::default_range` as each combat of the
  acting minion begins: the round's range at once (`range_by_default`, a
  block rider still overriding it), and every later round's after
  `each_round_range`, before `next_round_range`.
- Retainers: `Modifier::RetainerDamageClose { n, when }` (Dread Mastiff;
  Elephant Guardian's and Stone Dog's `RetainerWhen { who, vs_acting }`, hashed
  only when not `ALWAYS`), read live in the normal pair
  (`GameState::retainer_damage_close`, at every site: the damage, Ghoul
  Retainer's weapon use instead, a reflected strike, Tainted Spring); Duma
  Rafiki's `RetainerBurnsWeaponOrDamagesClose` asks its controller before the
  strikes resolve (`ask_retainer_weapon_burn`, `CombatCtx::retainer_picks`): the
  weapon burned once they resolved, or the damage. A melee weapon's
  `RangedStrength` strike takes the melee bonuses (`melee_weapon_damage`;
  Kerrie, Dagger) and makes it a ranged weapon (`cards::is_ranged_weapon`).
  A dodge's effect reads `GameState::undodgeable(minion, strike_source)`:
  Aggressive Corpse's every strike, Elihu's melee weapon strikes
  (`Stat::MeleeWeaponStrikesUndodgeable`, `GameState::melee_weapon`).
  Poker: `Side::all_prevented_watch`, the strike's prevented points counted by
  source in `Side::strike_prevented` (`preventing_with`: the tags of the
  prevention being applied); burned when every point was prevented by sources
  requiring the Discipline.
- Strikes that dodge, presses by the round's damage, committed strikes, aims and
  oaths. A strike that is also a dodge (`Effect::StrikeAlsoDodges`,
  `Side::also_dodges`: Stutter-Step [CEL]): `resolve_strikes` reads `evades`
  wherever it read a dodge, first-strike holds included; `allows` refuses it
  under "only hand strikes" or a dodge bar. `Side::no_additional_round` joins
  `Side::no_additional`. Presses settled by the round's damage
  (`Effect::RoundPressByDamage`, `Side::damage_presses`,
  `Engine::damage_presses` as the press step opens): Backstep's on
  `inflicted_round` (after prevention), Haymaker's on `Side::struck_round`
  (strike damage not dodged, prevented, immune or not: owner, 2026-11-12;
  rules-qa 116.138).
  `Effect::CommitInitialStrike` commits the initial strike after range
  (`committed_card`, one commitment a round); `Side::initial_first_strike` is
  read with the printed first strike for pair 0, set for the actor too
  (Backstab's `InitialFirstStrikeThisRound`). Song of Serenity [ani]'s
  `Side::strength_minus_round` lapses with the round; Kraken's Kiss's
  `Side::torpor_at` sends its vampire to torpor once `combat_damage` grew by
  n since the play (LSJ 20010610), an ally playing it burned (LSJ 20060409);
  Meld with the Land [vic]'s `unlock_opponent_rider` joins a combat ends
  strike's unlocks, a dodger spared; Skin Trap [VIC]'s
  `Interrupt::BloodOrNoStrike` asks the opposing vampire with the blood, else
  `no_strike_round` at once. `Limit::NotAfterLastRound` reads
  `CombatCtx::played_last_round` (the previous round's `played_round`).
  `Effect::StrikeSelfDamage`'s `close_only` splits Breath of the Dragon from
  Burst of Sunlight. Ammo `AggravatedDamage` (Dragon's Breath Rounds) adds
  aggravated gun damage to a damaging gun strike and burns the gun as its strike
  resolved; `Damage::gun_aggravated` keeps those points gun damage for Kevlar
  Vest. An aim at a retainer (`Effect::AimAtTargetRetainer`) sets
  `Side::strike_at`, the retainer branch taking hand and melee strikes at close
  range; `AimExtraDamage { no_press }` bars the press for Target Vitals only.
  `Effect::HandSizeIfNewRound` (Relentless Pursuit) is moved to `Seat::timed` by
  `new_round`. `Effect::BurnOpposingMeleeWeaponIfItHit` (Armor of Vitality
  [FOR]) reads the opposing `resolved_with` while its strike damage is pending.
  The Oath: steals and Taste of Vitae gains are noted (`GameState::note_taken`,
  `blood_taken`, settled with `blood_gains`, rules-qa 68.2); `Minion::oaths` per
  giver; two counters bar the sworn vampire's block (`GameState::shuns`) and any
  combat between the two (`GameState::sworn_apart`: no rush either way, no
  third-party pairing, the giver's block ending the action with no combat; LSJ
  20090210); its Ⓓ action is the effect-only card `LastingOnActor` leaves on the
  giver, placed once (`place_now`'s one-per for an effect-only card with
  abilities).
- Round 30's combat cards: this round's melee weapon riders
  (`Side::weapon_round`, folded in `strike_hit_past` for every strike of that
  weapon, the weapon burned as the round ends if still in play:
  `CombatCtx::burn_at_round_end`, Baal's Bloody Talons); damage only equipment
  or strikes prevent (`Side::hardened`, set aside by
  `CombatCtx::shelve_hardened` around any other prevention: a played card's or
  an allowance's, a card's play masked when only it is pending: Strike at the
  True Flesh); riders on damage successfully inflicted, settled in
  `apply_damage_now` (`HitDue`: an aim's clause, `Effect::AimOnHit` — Target
  Hand's -1 strength and weapon burn (`Interrupt::MayBurnWeapon`), Target Head's
  bar and next round's range (`Interrupt::MaySetNextRange`,
  `CombatCtx::next_round_range`, the first setter's), Target Leg's
  `Lasting::Hobbled` read where maneuvers and presses are offered and spent:
  only those banked from a card requiring one of its tags (`Side::bank` records
  the tags in `Side::banked_from`, `CombatCtx::spared_bank` picks; a rider
  carries its card's tags, `actions::Rider`; an ability's are its card in play's
  played mode, `in_play_tags`; rules-qa 121.161) —; Cobra Fangs' ally burned as
  the action ends (`ActionCtx::burn_minions_at_end`,
  `CombatCtx::burn_allies_at_end` without one) and its vampire's torpor, an
  effect-only card whose unlock rider is `Effect::HostGoesToTorpor`); opposing
  strikes with no effect on the vampire, not a dodge (`void` in
  `resolve_strikes`, a combat ends one included: `Side::untouchable_round`, its
  cards and retainers' damage too, Eyes of Blades; `Side::mirror`, a coin each
  strike, heads ending it, Gemini's Mirror); a shield on the opposing next hand
  strike (`Side::hand_shield`) and first strike next round
  (`first_strike_next_round` → `initial_first_strike`: Forearm Block); the
  opposing weapons inflicting nothing (`Side::weapon_proof`, Mayaparisatya); a
  strike's cap after every bonus (`Side::strike_cap`, Quick Jab);
  `Side::struck_round_aggravated` beside `struck_round` (Revelation of Wrath); a
  press and additional-strike bar from the moment damage lands (`Side::agony`,
  Sympathetic Agony); a retainer-only allowance (`retainer_prevents` on
  `from_retainers`, the narrowest `UsePrevent` first). A contract's chosen read
  live by `GameState::chosen_for_contract` (Shakar: the Hunt's Black Hand;
  Haqim's Law: Judgment's `Effect::ClanSectChosenForContract`). Fata Amria's
  unlock substitution is `Interrupt::MayBurnInsteadOfUnlock`, asked in
  `begin_turn` after the "does not unlock" checks.
- An ally's own combat traits (Muddled Vampire Hunter's first strike,
  Wendell's +2 melee damage) are layers on itself, read by
  `GameState::bearer_always`. A weapon's hand strike (Bundi) is a melee
  weapon strike too: `cards::melee_strike`, `Side::melee_struck`, the
  melee bonuses, `raise_strike`; a target-filtered `WeaponStrikePlus`
  (Brute Force) passes Immortal Grapple, its filter running the weapon's
  own gate. `Requirement::MeleeWeaponStrikeDamagePending` and
  `Effect::PreventMeleeWeaponStrike` (Bundi), once each strike exchange
  (`Limit::PerExchange`, `CombatCtx::abilities_used_exchange`).
- `Effect::BearerDamageOncePerCombat(n)` carries its amount (Zip Gun 1, Bomb
  5); `Effect::NoAmmo` (Zip Gun) refuses ammo (`StrikingWithUnloadedGun`).
- Rowan Ring, Wooden Stake: `Effect::TransfersToTorporedOpponent` sets
  `Side::transfer_on_torpor`; the damage application moves the weapon onto
  the vampire its strike sent to torpor (or whose torpor Undying Tenacity
  defers), burned if it cannot hold it, and sets
  `Minion::no_unlock_in_torpor` (skipped by `start_turn`'s unlock, cleared
  by `leave_torpor`; the infernal pay-to-unlock unaffected).
  `Effect::TorporAtWeaponDamage(n)` sums the weapon's inflicted damage per
  victim over the combat (`CombatCtx::weapon_damage`, after prevention).
- Ghoul Retainer: `Effect::RetainerUsesWeapon` records the weapon for the round
  (`CombatCtx::retainer_uses`; `Requirement::WeaponFreeThisRound`);
  `allows_from` masks the employer's strikes with it; in the normal pair
  `retainer_weapon_use` resolves the weapon's strike as the retainer's
  environmental damage at its strength, its riders (bearer damage on the
  employer, burn after use, counters even on an immune target (rules-qa
  116.138), Rowan's transfer, the Wooden Stake's tally through
  `Side::retainer_with`) and none of its maneuver, press or additional strike; a
  combat-ends weapon (Flash Grenade) ends the combat before damage. Its bite is
  suppressed that round.
- Round 30's weapons: `Effect::GuardNextHandOrMeleeStrike` (Nightstick's
  `CardEffect { ranged: true }` strike) sets `Side::guard` as the strike is
  chosen; the next opposing hand or melee weapon strike landing damage that
  round spends it whole (`Side::guard_hit`), a gun's or a ranged one leaving it.
  `Requirement::StrikingWithSource` (Garrote) reads `strike_source` or, past
  resolution until the damage applies, `resolved_with`: the going-to-torpor
  step sees the strike made, dodged or not. `Effect::FirstStrike` follows its
  `Strike` in a mode (`set_strike` resets it).
- An ally the damage resolution would burn waits, still in play, when a
  Methuselah could save it (`defer_combat_burns`, checked by
  `holds_card_for` first: `CombatBurnWait` in
  `GameState::combat_burn_wait`, `Window::AllyWouldBurnInCombat`, every
  living Methuselah asked clockwise from the active one, the round over);
  declined, `settle_answers` burns it as the damage would have
  (red list, FBI Special Affairs Division's count). Left for Dead:
  `Effect::SaveFromCombatBurn`, `TargetLifeIfNone`, a `HostEntersCombat`
  rider's `Effect::BurnHostMinion` (handled in `begin_combat`: burned as
  the combat begins, which then ends), and
  `Modifier::HostBarredFromNewCombat` (`barred_from_new_combat`: no rush
  targets it, `may_rush`; no Psyche!, queued, Coordinate Attacks or
  enter-combat-at-resolution combat with it). A burn outside the damage
  resolution (at the combat's end) never waits.
- Escaped Mental Patient's `Effect::BurnSelfAtCombatEndIfResolved`: its
  strike resolving, dodged or voided too (rules-qa 122.62, 122.214; the
  landing loop of `resolve_strikes`), puts the striker in
  `CombatCtx::burn_self_at_end`, burned in `end_combat` while the combat is
  still set (burned by its opponent: FBI's count, the red list).
- Young Bloods (`Modifier::BurnerVampireGainsBlood`, `burner_gains_blood` in
  `burn_minion`): the opposing vampire in combat, or the acting vampire of a
  successful action (an action card's: `GameState::acting` while
  `Engine::card_burns` collects), of another Methuselah; after the strike's
  damage, already mended.
- Haven Affinity: `GameState::block_combatant`, the blocker of the combat
  its block began (`start_combat` from `ActionState::Blocked`, cleared by
  `leave_combat` and for a queued combat), recorded only while it bears
  `Modifier::BlockerPlaysSuperiorCombat`; its combat cards at superior for
  the Disciplines it has (`disciplines_met`, beside Tremere Convocation).
- Battle Frenzy's `Effect::FrenzyStrikesEachRound`:
  `Side::free_strikes_each_round` added after each round's normal pair, the
  limit left alone (played in the additional strikes step, one that round
  too), and `Side::press_blood` burned as each press step begins
  (`press_step_burns`); Tranquility Shield zeroes both.

- Internal Recursion [pre]'s `BlockRider::CombatEndsStrike` sets
  `Side::combat_ends_any`: the strike offered every round, its unlock read
  as it is chosen (an ally or younger opponent). Conscripted Statue
  [vis]'s `BlockRider::Environmental { n, close }` adds to the side's
  `environmental`/`environmental_close`.
- Round 32's combat cards (prevention, torpor, a combatant swap): a strike-only
  allowance each round (`Side::round_strike_prevents`, Armor of Caine's Fury;
  `UsePrevent` takes it after the retainer and normal-only ones); a landed
  strike barring the round's prevention but by cards requiring its Disciplines
  (`Side::prevention_only_by`, read where `no_fortitude_prevention` is: Death of
  the Drum), an allowance's granting card's Disciplines kept per unit
  (`Side::round_prevents_from`, `each_round_from`; `Side::usable_round_prevent`
  admits one of Death of the Drum's Disciplines, refuses a Fortitude one after a
  Fortitude-proof strike, the other allowance kinds barred under Death of the
  Drum); hand-only additional strikes this round
  (`Effect::AdditionalHandStrikes`, Fractura's `hand_only_strikes`); a vampire
  being burned sent to torpor instead (`Requirement::ActorBeingBurned`,
  `Side::torpor_instead_of_burn`, a diablerie it averts reopening the opponent's
  step; the going-to-torpor step opens for a side's own vampire going down too)
  and its own torpor with the damage (`Effect::ActorToTorpor`: Ashes to Ashes);
  a round's hand strikes sending down on a successful hit
  (`Side::hand_hit_down`, read in `apply_damage_now`: Basilisk's Touch; its
  pending hit opens the going-to-torpor step, `basilisk_sends` read by
  `going_to_torpor` and `going_down`, rules-qa 124.26); preventable press-step
  damage (`Side::press_env`, `CombatCtx::press_step_prevention` resuming the
  press step: Crawling Chamber); the opposing chosen weapon strike made harmless
  (`Side::strike_harmless`, cleared by `set_strike`: Darkling Trickery);
  `StrikeKind::BurnEquipment { damage }` (Fractured Armament [POT], its damage
  through `strike_hit`); X cards burned from hand, replaced after combat
  (`Interrupt::DiscardChosen::hold`, `Engine::replace_or_hold`: The Death of My
  Conscience); the block's own combat (`CombatCtx::blocker`,
  `Requirement::ActorBlockedThisCombat`: Collapse the Arches); a donor out of
  the combat mending its player's combatant's damage
  (`Effect::MendOwnCombatant`, into `Side::healed`: Brother's Blood; its
  destruction prevented only where `combat::destruction_pending`); a combatant
  swap keeping the combat (`CombatCtx::free_fight`,
  `MinionPurpose::BecomeCombatant`, asked as the before-range step closes, by
  `apply_damage` as the damage left a combatant out of the ready region
  (`CombatCtx::free_fight_mid`: `free_fight_swapped` goes on from the round's
  press step, rules-qa 124.89) and by `end_combat` after a press continued it;
  `become_combatant` swaps it in by `swap_combatant`, Ohoyo Hopoksia's too: a
  fresh side with its own per-combat bonuses (`init_side`, the first round's
  only in round 1), its contracts re-keyed; Free Fight adds its own [SAN] press
  of the round from `CombatCtx::free_fight_presses`).
- Round 32's combat cards (ammo, restraints, presses): signed gun ammo
  (`Ammo::DamageByRange { close, long }` added at either range: Scattershot,
  Manstopper Rounds); the frenzy restraint without the equipment bar
  (`OpposingRestraint { equipment }`: Whistling Up the Beast), Blight's bar
  on presses to continue read from `no_additional_strikes`, no frenzy flag
  (Tranquility Shield leaves it); the opposing minion's additional strikes
  barred this round (`NoAdditionalStrikesThisRound { opposing }`: Rigor
  Mortis); an opposing maneuver canceled once each round
  (`Side::cancels_maneuvers`, `CombatCtx::maneuver` recording
  `maneuver_undo`, `Engine::offer_maneuver_cancel` asking
  `MinionPurpose::CancelManeuver` after each effect or banked maneuver; the
  cancel restores the range and the maneuvering turn, takes the presses its
  play banked, keeps a committed strike: Rigor Mortis [THN]); the initial
  strike's own strength (`Side::initial_strength_round`, read by
  `CombatCtx::strike_strength` for pair 0: Swoop); a ranged weapon's strike
  through a card (`CardFilter::RangedWeapon`: Projectile); an aim adding to
  a strike's after-combat damage (Riposte with Target Vitals); a minion
  sidelined for the turn (`Lasting::Sidelined`, read by
  `barred_from_new_combat` and `cannot_block`, with `recruited`: Heaven's
  Gate); the card's player read from `CardInPlay::by` (`HostCannotBlockBy`:
  Infection [THN]; `ByInterceptVsBearer`, counted in `host_intercept` and so
  `block_can_reach`: Tracker's Mark, offered by
  `Requirement::OpponentJustLostToDamage` in the window right after the
  damage resolution, `Side::just_lost` aged by `CombatCtx::fresh_window`;
  a next window with no untimed timing (the press step's damage) is
  preceded by a window of its own, `CombatCtx::marking`, the marks and
  any-impulse plays only, open when a combatant holds a mark,
  rules-qa 124.235); blood held on a card in play until its
  player's unlock phase (`OpposingBloodOntoCard { back_after_combat }`,
  `CountersToChosenAsBlood`: Vitae Block); a retake while its minion is
  ready (`cards::Retake::IfMinionReady`, `Seat::retake_by`, read by
  `next_retake`: Roll); animal retainers named from the ash heap as played
  (`Terms::XOwnAshCards`) put in play on the actor and burned as minion
  phases end (`GameState::summoned`, `Engine::summoned_phase_end`: Summon
  Spirit Beast; Ilomba's own text turns that burn into its host's blood).
- `Side::barred_disciplines` is a union of every bar (Pseudo-Blindness's
  block rider, Powder of Rigidity's `Effect::OpposingBarredDisciplines`);
  `OpposingWerewolfStrengthMinus` reads the label as it resolves.

- `Effect::CoordinateAttack { bar }`: the new combat's first-round bar on
  the opposing minion, strike cards (Coordinate Attacks) or maneuvers
  (Follow the Alpha). `CombatCtx::torpor_after` (Legacy of Power) sends the
  combat's ready vampires to torpor as it ends, dropped with the other
  after-combat effects (`discard_held`) or by a new combat (Psyche!,
  coordinate). `Side::combat_strike_once` (Veiled Sight [CHI]'s
  `BlockRider::StrikeOnceInCombat`) is each round's `round_strike` until
  used.

- Round 33's combat cards (track B): a long default range for the combat
  (`CombatCtx::long_by_default`: the range long at once unless set, each
  new round's range long unless a setter's, determine range still open,
  the Flamethrower's backfire as for a set range; Fear of the Void Below
  [dai]), with an end-only press each round (`Side::end_press_each_round`,
  banked by `Side::new_round`) and the opposing controller's random
  discard in each normal strike resolution (`Side::discards_each_round`,
  `CombatCtx::discards_due`, drained after `resolve_strikes`); the next
  round's hand-strike bar (`CombatCtx::next_round_hand_only`: Grasp of the
  Python [SER]); card labels the opposing minion cannot play this combat
  (`Side::barred_labels`, read in `play_options`: Immense Size); a strike's
  aggravated damage treated as normal at resolution only, still aggravated
  for prevention (`Side::strike_aggravated_as_normal`, per pair), its
  aggravated damage prevented (`PreventStrikeAggravated`) and an immunity
  to aggravated damage for the round (`Side::immune_aggravated_round`,
  zeroed with Tye Cooper's in `apply_damage_now`: Ignore the Searing
  Flames); first strike with hand and melee strikes only
  (`Side::hand_melee_first_round`: Silence of Death); the strike-choice
  order swapped for the pair (`CombatCtx::strike_order_swapped`,
  `first_chooser` driving `ChooseStrike` → `BetweenStrikes`, reset by
  `begin_pair`: Rapid Thought [CEL]); a Discipline played "as if"
  (`Lasting::PlaysDisciplineAs`, read by `disciplines_met` only, never
  `discipline_level`: Shared Strength); a canceled frenzy card's target
  pressing (`CanceledFrenzyTargetEndPress`, `frenzy_on_all`: Tranquility
  [DOM]); a card burning its host while a mortal, read live
  (`Modifier::HostBurnedIfMortal` in `settle_bound_burns`: Mercy for Seth).
- Round 33's action cards (track B): an action card's own "that minion
  may not attempt to block" read at declaration (`TargetCannotBlock` into
  `ActionCtx::barred_blockers`: Red List); untitled minions barred
  (`Modifier::CannotBlockUntitled`, a `DeclaredActionLayer`), a title card
  with a computed title (`CardInPlay::title`, read through
  `CardInPlay::represented_title` wherever a card in play's title is:
  `Effect::BaronOfActorCity`, Vlad's `MADE_UP_CITY` or a city held in
  play, `MinionPurpose::SecessionCity` re-titling the card through
  `gain_title`: Anarch Secession); a clan held by the card put on its vampire
  (`CardInPlay::clan_change`, undone as it leaves, as `sect_change`: Revelation
  of the Sire); a title's votes netted down (`Modifier::HostTitleVoteMinus`, the
  prisci ballots untouched: Rastacourere); a granted action's stealth for a sect
  (`StealthForSect`: Covenant of Blood); a chosen vampire's votes changed or
  forced by a card in play (`ChangeChosenVotes`, `CastChosenVotes`: Distant
  Friend); `min` to `max` picks with no X cost (`Terms::MinionPicks`, masked
  with fewer than `min`, directed at each pick's controller; `PicksLoseTitle`,
  `DamagePicks`, `PicksLasting`; `Lasting::CannotBlock`,
  `Lasting::CastsNoVotes`: Inquisition, Shared Nightmare); riders on the card's
  player entering combat and its host's combat ending (`Hook::ByEntersCombat` in
  `begin_combat`, `Hook::HostCombatEnded` in `close_combat`), an unlock drain to
  torpor (`HostBurnsBloodOrLifeElseTorpor`: Consignment to Duat), any burn
  locking its host in one mode (`BurnedLocksBearer`, read in
  `burn_card_in_play_now` from the card's mode) and a card's discard phase
  damage to another of its host's controller's minions (Mind of a Killer); a
  burn replacement removing every copy of the name
  (`Modifier::HostForgottenWhenBurned` → `BurnReplacement::Forgotten`: The Name
  Forgotten); a vampire picked from any ash heap (`AshPick::Vampire`), the card
  given to that heap's Methuselah (`AshPickSeatTakesCard`) and a look at its
  controller's hand (`PeekCardControllersHand`: Lifeless Tongues); a referendum
  called from another's ash heap (`Terms::OthersAshReferendum`,
  `Engine::others_ash_referendums` probing the cards as a virtual hand; the card
  removed at resolution, no card vote, nothing for Delaying Tactics to return:
  Bujo); control of cards requiring a Discipline (`TakeControlOfTargetCard`,
  `TakeControlOfAllRequiring` with `MinionPurpose::HostCardOn`,
  `Holder::OwnVampire`: Shepherd's Innocence); blood from any uncontrolled
  region (`Terms::UncontrolledSourceAny`, `UncontrolledSourceBloodToTarget`) and
  a target slot brought in at capacity (`UncontrolledTargetToReadyIfFull`: Undue
  Influence); a card put on its actor after the play, one per vampire
  (`PutOnActorOnlyOne`: `place_now`'s flag, a second played to the ash heap,
  rules-qa 99.2: Ritual Challenge).

### 6.8 Referendums

`referendum.rs`: `ReferendumCtx`, `RefKind`, `RefStep`, `Source`,
`VoteRecord`, `TermsState`; `ReferendumDef`, `Term`, `RefEffect`.

- A blood hunt may be held inside a suspended combat (§6.7, §6.5's
  "Diablerie's blood hunt"): votes and ballots as in any blood hunt, the
  combatants casting theirs; `after_referendum` resumes the combat first.

- Terms stepwise by the caller after the action succeeds (feasibility masks);
  a `Term` choice is never a target (rules-qa 38.6). Polling with card plays as
  commitments; per-source vote records (abstentions cancel); Methuselah ballots
  (Edge, the calling card's vote, one political card burned from hand); tally:
  more for than against, ties fail; Scorn first.
- A political referendum's polling opens with `ReferendumCtx::pre_vote`, an
  `OrderedWindow` over the action's order (`action_order`), offering the
  `Requirement::NoVotesCast` plays and abilities alone; then the polling
  proper from the caller, its window built as the stage closes (a fail in
  the stage still polled: rules-qa 116.34; rules-qa 64.12).
- A Methuselah's own vote from a card in play, once each referendum for
  each Methuselah whichever side (`Limit::PerReferendumEachSeat`,
  `ReferendumCtx::seat_uses`: Vox Senis); a card played to the ash heap
  for votes records `Source::Card` (`Effect::PlayerVotes`), offered in a
  blood hunt's polling too (masters only).
- Warding the Beast [VAL]'s blood hunt: `ReferendumCtx::favor_only_clans`,
  read by `barred_favor` with Protected District's `forced_against`; a
  vampire Astrid Thomas draws in favor is not barred by that force, its
  card-given votes against (`ReferendumCtx::against_only`) uncounted
  instead (LSJ 20031115).
- Votes are read live (`votes_of`, `clan_votes`, `calling_votes`, `host_votes`);
  grants (`ReferendumCtx.grants`) add to casts already made. An ousted
  Methuselah casts nothing. Mithras's toll on votes against his political
  action is `Engine::vote_against_toll` (beside `block_toll`); Alvaro's "if
  he abstained" gain is read after the referendum (`resolve_political`).
  Benefit Performance reads the standing records in favor
  (`RefEffect::CallerPoolPerReadyClanVotersBlood`); a card put in play by
  its referendum goes through `political_card_in_play` (`RefEffect::PutInPlay`,
  `ChosenGainBlood { put_in_play }`: Fanfare for Elysium [MEL]).
- A canceled referendum returns the card, discarding down at once. Political
  action: +1 stealth, cost at resolution, no referendum if the acting minion is
  no longer ready.
- Charming Lobby (`Terms::NamedReferendum`): a political action card's mode
  in hand (`PlayCtx::named`, from `named_referendums`: `play_options_by` in
  `Window::ActionDeclarePolitical`, an ally's grant reaching it) or an
  in-play political ability (`PlayCtx::named_ability`). As the action
  succeeds the named one takes `ActionCtx::card` (or `source`/`ability`),
  Charming Lobby kept in `ActionCtx::lobby` for the action's identity
  (Delaying Tactics' bar, Hay Ride, Epikasta, `requires_anarch`) and
  `place_action_card`; the hand card's cost is paid then, its own, X asked
  (`Interrupt::NamedX`), the card taken from hand, its play limits counted
  (`Engine::note_play_limits`, as `announce_play` counts them), never
  through an as-played window. `GameState::auto_pass_any` is the next
  referendum's pass, spent by the next one a vampire calls.
- Simultaneous ousts are one batch (`oust_batch`): a VP to every predator, 6 pool
  to survivors only; every remaining Methuselah ousted at once is a legal end
  (rules-qa 6.9).
- Another Methuselah forcing a vampire's cast (Mustafa: `Effect::CastTargetVotes`,
  a cast already made turned, `ReferendumCtx::forcers` for once each user).
  Secret simultaneous picks (Game of Malkav: `Interrupt::SecretNumber`) are
  kept in `Engine::secret_numbers`, outside the state, so no legal view holds
  them; the host tells another viewer a `set_x` was chosen without its amount.
- Alastor's referendum puts its card on the chosen vampire, then
  `Interrupt::SearchEquipFor` (asked under `LibrarySearch`, so the card stays
  hidden): the equipment put on, not equipped (`place_now` with its layers),
  half its cost, each half less its payer's reductions (`alastor_cost`: the
  caller's `seat_equip_discount`, the vampire's printed ones; rules-qa 124.7).
- `tally(&st)` reads `Stat::DoublesControllersVotes` (Orlando: the votes
  of his controller's vampires doubled). Bruce de Guy's bar
  (`ActionCtx::barred_vote_sect` → `ReferendumCtx::barred_sect`) is read by
  `may_vote`. `RefEffect::MoveDistributedFromFirst` (Public Vilification)
  moves the `DistributeBlood` points from the first chosen vampire, short of
  blood `Interrupt::ShortfallPicks`; `TermsState::term_start` keeps a term's
  picks apart from an earlier term's. A cast turned by another's effect pays
  no vote toll (`vampire_cast`'s `first`).
- A choice at a political referendum's tally, before its effects
  (Terrifisto's `Stat::DamagesAgainstVoterForBlood`, after Hartmut's and
  Marcos's burns, rules-qa 51.1's moment): `Interrupt::DamageVoterForBlood`
  with the context back in `Engine::referendum` and `ReferendumCtx::tallied`
  set; `referendum_advance_now` then runs `political_after_tally` (the
  shortfall, the passed referendum's choices, the effects), whatever became
  of the interrupt.
- Velvet Tongue's `ReferendumCtx::corrupted_barred` is read by `may_vote`,
  live: a minion corrupted after voting loses its votes. Skaald's
  `Modifier::HostVotesWhenCallerOfClan` reads the caller in `votes_of`
  (`GameState::caller_clan_votes`).
- Referendums: a held and failed one apart from a canceled one
  (`ActionCtx::referendum_failed`, `Requirement::ReferendumFailed`: Hellene
  Panhard's `StunHost`); votes barred against a caller of a title
  (`Stat::NoVotesAgainst`, `CallerTitleIn`: Samira); a chosen seat among those
  given points (`SeatFilter::Allocated`: Information Exchange); a vote from a
  crypt reveal (`RevealCryptTopVoteIfOlder`: Tubarão); a draw after a passed
  referendum (`ActionCtx::draws_if_passed`: Deadly Secrets); the caller's pool
  burned per vote it failed by (`ReferendumCtx::fail_pool`: Elder Kindred
  Network), a forced fail with the votes for it by none.
- A blood hunt referendum is a diablerie's or a political one whose effect
  is `RefEffect::BloodHuntChosen` (Veles' Hunt): `ReferendumCtx::is_blood_hunt`
  and `blood_hunt_on` (the diablerist, or the chosen minion once the terms
  close) feed every blood-hunt reader (`votes_cast`, `barred_against`,
  `hunted_votes`: Styles Margs, Ilyana); the kind stays `Political`. Its burn
  goes through `defer_hunt_burn` (Redline, Genina), the action's end waiting
  with it in `settle_answers`.
- "No more played this game if it fails or is canceled"
  (`Requirement::NoMoreOnceReferendumFails`): `GameState::once_played`, pushed
  by `own_referendum_failed_riders`, read by `play_options`. Counter votes
  (`RefEffect::BurnSourceCountersInFavor`: National Guard Support) are a
  `Source::Counters` record pushed as the referendum opens, an event's
  (`Modifier::BloodHuntVotesAgainst`: Urban Jungle) a `Source::Event` one pushed
  by `hunted_votes`; neither is a Methuselah's (`VoteRecord::by_methuselah`:
  bribes, Scorn of Adonis, Carlak and the "voting against" burns skip them).
  A per-count vote source (Aisha az-Zahra, Powerbase: Madrid, Bastille Opera
  House, Trumped-Up Charges, a printed `Per` vote) adds a `LiveVotes` entry
  (`ReferendumCtx::track`) beside its record's fixed `n`: every reader goes
  through `ReferendumCtx::n_of`, the count read again (owner, 2026-11-19); a
  turned cast (`force_cast`) rebases the entries (`rebase`).
  Jericho Founding's locations share Peace Treaty's keep-by-repaying path
  (`kept_candidates`). A title card is masked from a vampire that cannot gain
  its title (`GameState::may_gain_title`: Persona Non Grata, Lena Rowe's
  `Stat::CannotGainTitle`) in `ReferendumDef::feasible` and `minion_choices`; a
  title gained without a card (Horatio Ballard's referendum, Valerius's steal)
  is offered, a legal no-op as it resolves (`gain_title`, `steal_title`).
- A passed blood hunt's burn waits on every living Methuselah, the hunted
  vampire's own too (`defer_hunt_burn`), each offered the plays and the
  abilities timed `Window::BloodHuntWouldBurn` (Redline; Dogs of War's
  cancel, LSJ 20081203-2), an ability resolving at once in
  `answer_apply`.
- A referendum's lasting effect on no card in play is an effect-only
  stand-in (rules-qa 21.1): `RefEffect::LastingOnChosen` on the chosen
  vampire (Sabbat Priest, a copy each time), `LastingAgainstEachChosenSeat`
  on no minion, controlled by and `against` each chosen Methuselah (Deploy
  the Hand's target counters: its riders and abilities theirs, outliving
  the caller; the host shows it in their `in_play`). "That Methuselah may"
  is `Requirement::UserIsAgainst` (Hand Intervention).
  `Term::ChooseSeatsUnchosen { max }` caps the Methuselahs chosen.
- `RefEffect::StealPoolFromEach` (each pool read before any moves: Transfer
  of Power, Tithings' `SeatFilter::NoVampireCapacityAbove`),
  `BurnBloodCapacityBelow` (Rabble Razing), `ChosenLosesTitle`
  (Excommunication, `lose_title`), `PoolOrBurnChosen` (Auto-da-fé:
  `Interrupt::PoolOrMinionBurned`), `BurnPerMinion { of: PerMinion }`
  (Perpetual Care's vampires in torpor); `PutOnCaller { may }`: Crusade:
  Rome's unlock, Fiendish Tongue's `Minion::discard_unlock` at 0 blood from
  the discard phase's start, usable once in it.
- `Effect::BurnTargetCryptTop { only_vampire_with }` (Sabbat Inquisitor):
  the top card seen by the user (`Look::crypt`), burned only if a vampire
  with the Discipline.
- Rock-paper-scissors (Cracking the Wall, Malkavian Game:
  `Effect::RockPaperScissors(RpsStakes)`) reuses the secret picks:
  `Interrupt::SecretNumber { game: SecretGame::RockPaperScissors }`, the
  other Methuselah's pick then the player's (`push_rock_paper_scissors`),
  `reveal_rock_paper_scissors` on the last; a tie of `RpsStakes::Pool`
  plays again for `raise` more, unbounded by the text (every bot's pick
  is random: ties end with probability 1).
- A vote change, not a forced cast: `Effect::ChangeTargetVotes` (Kindred
  Manipulation, Michael Luther) turns a standing cast as
  `CastTargetVotes` does (`force_cast`); on a vampire not cast yet it
  records `ReferendumCtx::sides_set`, read by `side_set` where casts are
  offered and made (`barred_favor`, `vampire_cast`): that side only, an
  abstention still open (RTR 19991001; advanced rules §3.7.6.3).
  `CastTargetVotes` (Mustafa, Ellison Humboldt) compels and drops the
  entry. Cheval de Bataille's `Effect::TallyBloodAgainst` adds to
  `ReferendumCtx::blood_against`, burned in Hartmut Stover's tally loop
  with his printed amount.
- Overseer's marker `Effect::FavorIgnoredFromTitled` fills
  `ReferendumCtx::favor_ignored`: those titled vampires' records in favor drop
  out of `tally`. `Term::ChooseCardsTitledMore` (The Secret Must Be Kept: more
  picks for a titled caller), `Term::ChooseNumber` (Political Flux:
  `TermsState::number`, `RefEffect::NextOustPool` → `GameState::next_oust_pool`,
  paid to every predator the next `oust_now` pays, then used up, rules-qa
  124.173). A political action card's `Effect::ActionLayer`s are pushed as its
  referendum starts (The Eldest Command Undeath's
  `Modifier::ActingCannotGainBlood`, read in `cannot_gain_blood`).
- Bastille Opera House's `Effect::MethuselahVotesPerReadyClan` (the
  Methuselah's votes, the clan counted again to the tally: `n_of`) and
  `Requirement::ControlsNoReadyClan`; Conductor's
  `Modifier::ClanVotesUnlockedOrActing` in `GameState::clan_votes`;
  `Modifier::HostVoteWhenCalling` takes `Bearer`s (Fee Stake's clans,
  Ancestor Spirit's Laibon). Vox Domini's "not on a referendum
  automatically passing" needs no requirement: an automatic pass opens no
  polling step (`polling_request`).

- Round 32 tallies: Madrigal's `TallyBloodBySide` (`ReferendumCtx::
  side_blood`, by the final records, after Cheval de Bataille's burn); Hall
  of Hades' Court's `DoubleActingVotes` (`ReferendumCtx::doubled`, read in
  `tally` beside Orlando's stat; `Requirement::MultiplierAllowedOnActing`
  bars a second multiplier and Orlando's controller's vampires); King's
  Favor's `PutInPlayIfNarrowPass(Modifier)` (`ReferendumCtx::narrow_pass`:
  the card from the ash heap into play choosing [voter, caller] if the
  margin is at most its vote, its layer `ChosenCannotBlockChosen` read in
  `GameState::cannot_block`). Igoli's Loyalty's `FavorAuction` starts
  `Interrupt::FavorAuction` (`favor_auction_from`): accepting forces the
  Methuselah's ready vampires' casts in favor (`force_cast`).
- Choices as a referendum passes beyond a ready vampire
  (`RefEffect::chooses_at_passage`, `RefStep::Choose`):
  `PutInPlayClanMayJoinSect` (Ravnos Acceptance: each Methuselah's vampires
  of the card's `Modifier::ClanMayJoinSect` clan, one at a time until a
  pass, join its sect through `Engine::become_sect`; a vampire of the clan
  entering play later is asked once, `Interrupt::MayJoinSect` in
  `vampire_entered`) and `EachBurnsOwnUncontrolledVampire` (Might of the
  Camarilla: `Choice::Slot`, no pass, `TermsState::slot_picks`;
  `Engine::burn_uncontrolled` to the ash heap, not burned from play).
  `CounterAgainstEachChosenSeat { cap }` (Sabbat Threat): one effect-only
  stand-in per Methuselah holding the counters, `Effect::BurnPoolPerCounter`
  at its unlock phase. `Effect::CallerBurnsBloodIfReferendumFails` (The Final
  Nights) in the tally's fail block. `Effect::ChosenControllerVotesPerRedList`
  (Trumped-Up Charges): the chosen minion's controller casts
  `Choice::CastGrantedVotes` either way (`Source::Granted`, counted as cast,
  `Engine::granted_votes`). `RefEffect::Banish` re-checks its choice
  (rules-digest D1).
- Red List (`GameState::is_red_list`): made (Public Enemy), a card on it
  (`Modifier::HostRedList`: Trumped-Up Charges), or printed (a crypt card,
  an ally's own `HostRedList`: Ossian) and not lifted
  (`Minion::red_list_lifted`); `Engine::lift_red_list` (Priority Shift)
  burns the cards on it making it so.

### 6.9 Contests and titles

`contests.rs`; `state.rs`.

- `Contest::{Card, Title, Library}`. A contested card is out of play:
  `CardInPlay::present()` false, its layers suspended (`GameState::layer_active`),
  skipped by abilities, riders, copy counts and "a card named X". A library card
  enters contest only after its own effects resolve (spec §1.13.2). The cards
  on a contested minion go out of play with it (`set_minion_contested`, through
  `Engine::cards_out_with` / `cards_back_with`, as Descent's; rules-qa 83.1).
- Two-player games (`GameState::format`, `Format::TwoPlayer`, set as a
  2-seat game is dealt; rules-digest §18.5): a crypt card's contest goes
  through `set_card_contest`, which keeps it in play
  (`Minion::contested_in_play`: `present()` true, nothing suspended) with
  the same upkeep (`Contest::Card`, 1 pool or yield); library cards contest
  as above. `title_rivals` skips two copies of one vampire both
  contested in play.
  Transfers: `influence::transfers_for_turn` reads the format.
- Self-contest is masked (rules-qa 7.1; `controls_unique_copy` also for
  searches), a unique card's combat modes excepted: they enter no play
  (Spirit Claws, its "Unique." in its action's text: ANK 20180629), nor an
  out-of-turn mode outside the master phase placing nothing (Vox Senis to
  the ash heap; Leadership Vacuum's enters play); a forced one burns the
  incoming copy (The Coven, `Engine::take_card_control`). A card with
  `Effect::IncomingCopyBurns` (San Nicolás de los Servitas) is never contested:
  the copy entering against it burns in `enter_library_contest`, its own burn
  effects firing.
- Titles are read live (`GameState::title_of`): **claimed** (`claimed_title`: a
  title card on the vampire, else `Minion.granted_title`, else printed unless
  lost) vs **effective** (none while contested, out of play, off-sect, or its
  sect fallen). One title per vampire (`gain_title` loses the printed one and
  burns an older card). A unique title card contests as a card (pool), giving its
  title once the contest ends (rules-qa 19.1; in `place_now`); a non-unique one
  (Toreador Justicar) contests as its title (blood). `steal_title` (Valerius)
  burns the card and gains the title itself (rules-qa 19.6). Liaison is
  `Title::Independent { "Liaison", 4 }`, the younger claimant yielding.
- Titles are no part of the self-contest bar (owner, 2026-11-11; rules-qa 3.1):
  two of one Methuselah's vampires claiming the same unique title contest it,
  chosen or forced, through `enter_title_contest` as any two claimants (a title
  card on one's own vampire, a title stolen or gained, a vampire taken or
  influenced out). Zane's referendum (`RefEffect::CallerContestsChosenTitle`)
  gains the chosen prince's very title, contested with its holder.
- A take of control by choice never takes a copy of a vampire the taker holds, a
  contested one too (two players: both in play), nor a unique ally whose copy
  the taker controls (rules-qa 121.33) (`GameState::controls_vampire_copy`;
  rules-qa 58.19). The mask is generated, never a card's requirement (owner,
  2026-11-10): every mode or ability whose effects take control
  (`Effect::takes_control`) filters its minion targets by it, a card's play
  (`play_options`) and an ability's (`ability_targets_by`); a host take (Puppet
  Master's `TakeControlOfHost`) is not offered; the corruption steal checks it
  (`corruption_steal_reached`: Cave of Apples, Contagion). Forced, the incoming
  copy burns (`take_control_now`; 58.19).
- Laibon titles: `Title::Laibon { name, votes }` (from `gen_crypt.py`;
  the host's title id `laibon`); Osebo Kholo moves as §6.2 says, gaining
  through `gain_title`. A named Camarilla title likewise
  (`Title::Camarilla`, id `camarilla`: Karsh's Imperator, merged).

- A title bought as a vampire enters (Victor Pelletier:
  `Interrupt::BuyTitle`, before any contest, not a title its Methuselah
  holds) and a yield instead of a contest (`Stat::YieldsTitleInsteadOfContest`
  in `enter_title_contest`: the title lost).
- A capacity a title card gives its clan's holder while it holds that very
  title, effective (`Bearer::ClanTitled` in `Modifier::BearerCapacity`:
  Praxis Seizure: Barcelona, Rome; TOM 19960210).
- Clio's Kiss: `Requirement::TargetSeatContestsWithYou { vampire_only }`
  (`GameState::contested_with`), `Terms::OwnContestedVampire`;
  `Effect::TargetSeatYieldsContestedVampire` burns the rival copies of
  that Methuselah (a yield) and, no rival left, releases the contest at
  once (`release_contest`), the vampire unlocked.

### 6.10 Minions

`state.rs`: `Minion`, `MinionKind::{Vampire, Ally, Imbued, Created, Token,
CryptAlly}`.

- Entry: `GameState::settle_entries` stamps a minion's `entered_at` and
  `entered_turn` as it enters play (a merge or a control change no entry)
  and clears `was_contested` (set by any card contest, a two-player one
  too) once it is out of play (Toy Chest Test: "put into play since your
  last turn", "not … contested"); both `state::Unhashed`: compared, never
  in the state hash.
- Merging (`Engine::merge_partner`, `Choice::Merge`; rules-qa 69.5): by
  name and advanced flag, never by group — the groups bind the crypt only
  (ANK 20220805-2: a stolen G6 Theo Bell under the G2 advanced card). A
  merged vampire's text is its `vampires::MERGED` definition, found by
  the base card it keeps (`Minion::merged`): a name with several bases
  has one definition a base group (`VampireDef::group`).
- Capacity for another Methuselah: `capacity_by(m, seat)` and
  `younger_by(a, b, seat)` add Orun's +2 (`Modifier::CapacityToOthers`),
  before the minimum of 1 (ANK 20180719-1: Barrenness), over Wormwood's
  ceiling (LSJ 20051205).
  Every card's or vampire ability's capacity read goes through them with
  its reader: the card's player (`CardInPlay::owner`), the referendum's
  caller, an ability's `AbilitySource::player` (who played the vampire or
  card, its thief aside; `Requirement::reads_capacity` picks the
  requirements read so) (rules-qa 109.1, 109.3, 109.50).
  `capacity_of`/`younger_than` are left for rulebook reads (diablerie,
  rescue, blood hunt), blood caps and fills, and printed passive bonuses.
  Cards a holding caps (Orun, Aye over capacity) settle in
  `Engine::settle_over_capacity`: of two kinds, the host's controller
  picks (`Interrupt::BurnOverCapacity`, rules-qa 109.2).
- Vampire traits are read live: `capacity_of` (with `capacity_ceiling`),
  `discipline_level`, `has_discipline`, `Minion::clan`, `GameState::sect_of`
  (`SectFallen` over `considered_sect`, `underlying_sect`, `sect_override`);
  printed data only via `printed_*`. Blood gain caps through `gain_blood`;
  excess drains as the engine settles; out-of-play counters are kept. Gestalt
  [SAN]'s borrowed Disciplines last the action (`GameState::lent_disciplines`,
  read by `discipline_level`). Blood gains are noted between decisions
  (`GameState::blood_gains`, `Engine::note_blood_gains`), a strike's as the
  strikes end (Slake the Thirst, rules-qa 68.2).
- Burned from play: `Seat::burn_vampire` is the one way a vampire card
  reaches the ash heap, `Seat::burned_from_play` naming the copies burned
  while controlled (not contested, not from the crypt or the uncontrolled
  region: ANK 20210813); `GameState::ash_burned_vampires` and
  `Terms`/`Requirement::OwnAshBurnedVampire` read it (Blessed Resilience,
  `Effect::AshVampireToReady`: a new minion, as an influenced one enters).
  An ally's card burned with it, controlled, as an ally card (not a master
  become an ally), is named in `Seat::burned_allies` (`burn_minion_now`, after
  `Seat::to_ash`, every library card's way to the ash heap, cleared it;
  both lists untracked like every copy):
  `GameState::ash_burned_allies`, `Terms::AnyAshBurnedAlly` and
  `Requirement::BurnedAllyInAnyAsh` read it. Pressing Flesh
  (`Effect::RaiseAshAllyWithThis`) puts it in play as Split the Veil does,
  its card its owner's, the card on it: `Modifier::BearerSubtypeOnly` its
  one subtype (`has_subtype`, `is_mortal`, `ally_of`). A burned imbued
  (`GameState::ash_burned_imbued`, `Choice::AshVampire` with
  `PlayCtx::ash_crypt`) comes back as `MinionKind::CryptAlly`: its crypt
  card and text, an ally but no imbued (no creed, virtue, conviction or
  incapacitation; LSJ 20100211; `Engine::raise_crypt_ally`).
- **Allies**: a minion and a card in play at once (`CardInPlay.body`, not
  `host`, so never "a card on a minion"; `CardInPlay::bearer()` = host or body).
  No clan, sect, Disciplines, title, capacity or votes; neither younger nor older
  (`younger_than`, `ally_or_younger`); life uncapped; damage burns life, torpor
  burns it; no hunt/rescue/diablerie/politics, no blood costs; recruited ones act
  next turn (`Minion.recruited`). Its version's type line is kept
  (`MinionKind::Ally.body`). A unique ally contests as a minion (`card_rivals`).
  "Burn n blood" lands on an ally as life (`GameState::burn_blood_or_life`,
  rules-qa 73.5); as an ability's blood cost an ally never pays it (advanced
  rules 5.5.1, `ability_gate`).
  `recruit_ally` takes the recruit's X (0 put in play otherwise):
  `Effect::EntryCountersXPlus` puts X + n counters on its card, none
  removing it from the game at once, and `RemoveSelfWithoutCounters` its
  unlock rider's (Reanimated Corpse). A card's "+n hunt" for a named
  bearer is `Modifier::Bearer { stat: Hunt, who }`, read in
  `GameState::hunt_bonus` (Ritual Scalpel).
- **Imbued** (`MinionKind::Imbued(crypt)`, rulebook "Appendix: Imbued Rules";
  rules-digest §14.12): crypt cards influenced like vampires, in play an ally
  (`is_ally`, `is_imbued`) with its text in `VAMPIRES`: life = cost (the
  counters influenced in), 1 strength, 1 bleed; creed as clan, virtues as
  Disciplines (`printed_disciplines`, `has_disciplines`), excluded from the clan
  and Discipline vocabularies. A Mortal (`GameState::is_mortal`, `ally_of`: the
  Mortal-label readers take it); `is_monster` is a vampire or an ally neither
  mortal nor animal. At its last life `Engine::lifeless` incapacitates it
  (`incapacitate`: `Minion::incapacitated`, out of `ready()`,
  `Hook::HostIncapacitated` riders) where any other ally burns; the host and api
  show it as its own region (`Zone::Incapacitated`). It leaves by
  `Engine::recover` (+1 life), through `vampires::RECOVER` (3 conviction, its
  own unlock phase) or `RECOVER_GRANTED` (Jennie). Burning one is
  `ActionKind::BurnImbued` (D, its equipment taken; `Interrupt::AvengeImbued`
  per ready imbued). Convictions (`GameState::convictions`) are cards on it,
  played in the unlock phase (`Window::UnlockConviction`, once per imbued:
  `Minion::conviction_played`), from the hand or the ash heap
  (`GameState::unlock_phase` adds the ash heap's to the virtual hand as
  `state::ASH_UNLOCK`), at most 5. Entering play it gains one
  (`Interrupt::GainConviction`, unless `Effect::NoEntryConviction`). A "[n
  CONVICTION]" cost: `Effect::BurnConviction(n)` on abilities
  (`Interrupt::BurnConvictions`), `PayConviction` on granted actions
  (`Interrupt::NameConviction` at declaration, paid at resolution: gone, the
  action fizzles). Nickolai's `Terms::OtherOwnReadySuperiorDiscipline` is
  named the same way (`Interrupt::NameActionDiscipline`,
  `ActionCtx::named_discipline`), `Effect::GainNamedDiscipline` re-reading it
  at success (D1). A played card's "[n CONVICTION]" is `cards::conviction_cost`
  (the library's `conviction_cost`, else a mode's `Effect::ConvictionCost`; X
  from `Terms::XConvictions`): an offer needs an imbued holding them; a
  non-action card pays it with its blood (`pay_play_conviction`, a canceled one
  too), an action card names it at declaration and burns it at resolution, an
  equipment burns X as its equip resolves and keeps X in `CardInPlay::fixed_x`,
  one found or named asks its X a conviction at a time (`ask_x_convictions`,
  `Interrupt::XConvictions`) (rules-qa 102.1). The creed reads as a clan
  (`GameState::clan_or_creed`, in `meets_clan`), a virtue card printed without
  its icon gets its virtue as tags (`gen_library.py`; 102.2);
  `Requirement::ControlsReadyImbued`, `ReadyImbuedInPlay` (102.3). Powers:
  `ActionKind::Power` (+1 stealth; success puts it on the actor, unlocked; a
  second copy fizzles).
- **Created vampires** (`MinionKind::Created`: Childe of the Revolution, Waters of
  Duat): 1-capacity, sire's clan, printed or sire's sect, no Discipline or title;
  their card is the body. `Effect::BecomesVampire` sets a printed clan and a
  slave of the sire's clan (Create Gargoyle, Hatchling) and its `Born`
  (capacity, Disciplines); `ActingBloodToNew`, `ActingBloodToNewUpTo`,
  `BankBloodToNew` give it blood; `BurnBodyIfExcessCreated` is Brother in
  Arms' unlock due. Sterility: `GameState::is_sterile` (printed, or a
  `HostSterile` card: Barrenness). **Tokens** (`MinionKind::Token`:
  Absimiliard's Army): no card in play, name or label; removed without burning
  when the Army leaves. `MinionKind::Fury` (Unleash Hell's Fury) is out of play
  but for its own block attempts (`Engine::fury_tokens`). Infernal Servitor's
  removals (no infernal vampire controlled; after a successful action) are read
  as each step settles (`Engine::settle_servitors`) and as its action ends.
- Leaving play has one path each: `burn_minion` (cards on it to owners' ash
  heaps; "ready minion burned" riders only if it was ready) and
  `minions_to_torpor`, both through `leave_riders`, its pool losses burned last
  (`burn_separately`, §6.17). Before either, the host still where it is,
  `Hook::HostBurnedOrToTorpor` fires the riders of the cards on it
  (`Engine::host_burned_or_to_torpor`: Political Struggle's votes read
  then, before a diablerie's blood hunt). Burned vampires reach the ash heap
  (`Seat.ash_crypt`; its owner's, `Minion::origin`, even under stolen control).
  A burn replaced is checked first (`Engine::burn_replaced`, §6.11; Byzar's
  `Engine::replace_burn`, the torpor left to its caller unless `to_torpor`;
  his `Stat::TorporInsteadOfBurn`: a burn counter, `Minion::burn_counters`,
  and torpor; removed from the game at the printed count), so no burn rider
  fires; a site reading the burn's outcome reads `in_play` after it.
- Control: permanent (`Minion::take_for_good`: its owner kept as `origin`,
  a temporary control over; every steal for good, a vampire's or an
  ally's, Form of Corruption's, an ally's card given), temporary
  (`Minion.temp_from`, `Engine::take_control`: returns at the end of the taker's
  turn, removed if its Methuselah was ousted). Banishment (`Engine::banish`,
  through `Engine::banish_home`: the temporary control broken first, to the
  last permanent controller's region, ANK 20200408-2; Banishment, Pit of
  Contemplation, Exile, Count Zaroff, Yavu Matebo) keeps the vampire's blood as
  counters in the uncontrolled region, the cards in play on it (not those burned
  from it; a created vampire's own card included) face down with it, out of play
  until it returns as the arena entry it left (`UncontrolledSlot.banished`,
  rules-qa 25.1). A card on an uncontrolled vampire (Cadet) holds its slot's
  index, `CardInPlay::on_slot`: a slot leaves the region only through
  `GameState::take_slot`, which hands back the cards on it (onto the
  vampire it enters as or merges into, or burned, `Engine::burn_slot`) and
  moves the later ones' index down (rules-qa 107.31).
- Per-minion markers: `corruption` (per Methuselah; kept through torpor, control
  change, banishment; cleared when burned/removed), `hostage` (Carver's Meat
  Packing and Storage: while any, no move to the ready region nor
  diablerie), `torture` (The Path of Lilith: its controller burns 1 pool each
  unlock phase while any of their vampires has one; released once no such
  card is in play, `GameState::release_torture`), `nightmare`,
  `diablerie_shield`, `must_hunt`, `played_since_unlock` (wake limits),
  `unlock_at_discard`, `unlock_at_discard_if` (a clan read at that discard
  phase: Crusade: Istanbul's `RefEffect::PutOnCaller { unlocks_if }`),
  `diablerized_older` (Smudge's `Requirement::DiablerizedOlder`).
- Black Hand: printed (`CryptCard::black_hand`) or gained
  (`Minion::gained_black_hand`, Mateusz), read by `GameState::is_black_hand`.
  Pulled Fangs' "if they must hunt, torpor instead": `Engine::run`'s
  `settle_hunts_to_torpor`, in the controller's minion phase, read live.
- A vampire's text may grant its controller's minions an action
  (`Actor::ActingCandidate` on a `VampireDef` ability, Lucinde, Alastor G7):
  `vampire_ability_options` offers it per candidate from the grants made
  this turn (`GameState::chosen_this_turn`: the Methuselah, the vampire,
  the chosen), its vampire present or not (rules-qa 74.2); read by
  `Requirement::TargetChosenByHostThisTurn`.
- Slaves (rulebook "Slave"): one reading of the master clan,
  `GameState::slave_clan` (the generated `CryptCard::slave`, a created
  slave's `Minion::slave_of`, none once freed). The directed-action bar is
  the minion phase's filter beside `Stat::NoDirectedActions`; the block
  swap is `vampires::SLAVE_FIGHTS` (`Requirement::ActingOfHostsMasterClan`,
  `Effect::SlaveTakesCombat`, `ActionCtx::slave`), in every crypt slave's
  definition (a walking test) and in `vampires::RECRUITED_SLAVE`, which
  `cards::vampire_of` returns for a created one: the slave fights the
  blocker, the acting vampire unlocked, the block riders dropped, no
  `held_at_block` (Sniper Rifle, ANK 20200115).
- Out of play (`Minion::out_of_play`, `CardInPlay::out_of_play`, read by
  `present()`; `in_play` stays true, so a site reading it excludes them):
  Descent into Darkness's `Engine::descend` and `ascend` (temporary control
  broken, the card to its permanent controller; its contests left through
  `leave_contests` and entered again on return), and a contested minion's
  cards; both through `Engine::cards_out_with` (the cards leave their own
  contests) and `cards_back_with` (back unlocked, in their contests again).
  Dual Form's copy (`Minion::copy_of`: capacity and the 34
  `cards::DISCIPLINE_KEYS` levels fixed at resolution; `capacity_less` on the
  original; `dual_of` links them, `burn_dual_forms` burns the rest as one
  leaves ready, `about_to_leave_ready`; several leaving at once,
  `Interrupt::DualFormKept` keeps the one chosen, `GameState::dual_set`); a
  minion's discipline level is `own_printed_level` plus `granted_level`.
  Maleficia and Striga are levels but no Discipline (`cards::is_discipline`:
  what counts, lists or taxes Disciplines skips them — Dabbler's
  `disciplines_played`, Blood Weakens, The Slow Withering, Concoction of
  Vitality, `PlayedRequiresAnyDiscipline`; `state::disciplines()` never
  lists them).
  Derange's `Minion::deranged` (clan and sect as played, read by `clan()` and
  `underlying_sect()`, moved with the card, cleared in `card_leaves_play`)
  over `sect_override`, the actual sect a card or action wrote
  (`TargetBecomesSect`, `ActorBecomesSect`); a card put on the vampire that
  wrote it holds the change (`CardInPlay::sect_change`), undone as the card
  leaves play unless another change replaced it (rules-qa 104.3). Every unlock goes through
  `Engine::unlock_minion`, a disease counter's blood told as the disease's.
  Diabolic Lure's `Minion::pact` and `made_infernal` (read by `is_infernal`
  and `unlocks_as_infernal`). `GameState::immune_non_aggravated` (Tye Cooper)
  in `apply_damage` and `damage_out_of_combat`; `GameState::is_seraph` reads
  the crypt's Seraph trait.

- A ready titled vampire about to be burned, sent to torpor or put out of
  play (`titled_leaving`, from `burn_minion_now`, `torpor_now` and
  `minion_to_crypt_bottom` only) is recorded in `GameState::titled_left`
  (votes read then; cleared each turn) and offers `Interrupt::ReadyTitledLeft`
  (`Window::ReadyTitledLeft`: Leadership Vacuum) to the other Methuselahs
  holding such a card. A fledgling tethered to its vampire
  (`Minion::tethered_to`, Legion) acts, blocks and plays cards only while that
  vampire is ready (`GameState::tether_holds`, read at the action, block and
  play offers).
- Blood Brother circles: `GameState::same_circle` (printed or chosen,
  rules-qa 68.1); `same_circle_for_play` adds Angelo's any circle
  (`Stat::AnyCircleForSanguinus`) and is read only for a card requiring
  Sanguinus: by the `sanguinus` flag of `TargetSameCircle`,
  `TargetOnSameCircle` (a card target's host: Clockwerx, whose
  `Effect::TargetCardToActor` is `StealTargetCard`'s move, not an equip) and
  `SameCircleAsOwnCombatant`, always by the other circle readers, which only
  Sanguinus cards carry (§3's `circle_requirements_read_angelo_only_on_
  sanguinus_cards`).
- An ally leaving play shuffled into its owner's library, not burned
  (`Engine::shuffle_ally_into_library`: Mulher de Branco), its entry toll of any
  n ash heap cards (`Interrupt::EntryRemoveOrBurn`'s `left`, `any`;
  Corrupt Construction's `Interrupt::EntryRemoveForLife`, ash heap and hand,
  a life each); an ally's bleed per life above n (`Modifier::BleedPerLifeAbove`:
  Cãoera).
- A created vampire's `Born::unique` (Abomination) makes it a contest rival
  (`card_rivals`) and masks a second copy (`controls_unique_copy`,
  `cards::makes_unique_vampire`); a born capacity 0 stays 0 (only a
  reduction floors at 1), 1 in the uncontrolled region (The Becoming,
  `BodyBurnedWithoutDisciplineCards` in `settle_bound_burns` once
  `Interrupt::MastersFromHand` is done).
- A timed "as a vampire" grant (`Lasting::PlaysAsVampire`, Leech) joins
  `as_vampire_grants`; an imbued's own lasting grant meets a basic tag in
  `disciplines_met`.
- XTC-Laced Blood: `Modifier::HostMustHunt` makes `GameState::must_hunt`
  (and `starving`) true until the vampire hunted this turn. Hostile
  Takeover: `Interrupt::Bid` walks clockwise from its player, ended when the
  others passed in a row after the last bid; `settle_auction` pays and calls
  `take_control`. Malkavian Prank: `Interrupt::SecretNumber` picks, then
  `Interrupt::GuessNumber`, the last revealing (`Engine::guesses`).
  Instability: `Interrupt::MayTakeEdge` (`ChooseSeat`).
- Plays as a vampire: `AsVampire::once_per_turn` (Ananasi Vampirephile;
  `Minion::as_vampire_turn`), spent by `Engine::spend_turn_grant` once the
  play's effects resolved (its Pack Alpha's Raven Spy met by the same use),
  as an action card is declared, or as a play is canceled. Bima's
  `Modifier::PlaysAsVampireOfCardOn`: one grant per master Discipline card
  on it, read live in `as_vampire_grants`; a master Discipline card on an
  ally gives no level (`discipline_grants`, vampires only). Its entry:
  `Interrupt::DisciplineSearch { hand_or_burn }`. "The acting vampire goes
  to torpor" (Day Operation) burns an ally that played it as a vampire.
- A card in play representing an ally, its own text inert:
  `MinionKind::Represented { card, crypt, body, of }`, `of` the
  `state::Represent` its card prints (Spell of Life's mummies: life its
  capacity, named, a rush through `TextOf::Rush`; Khazar's Diary's wraiths:
  nameless, +1 stealth read with `GameState::lasting`). Put in play by
  `Engine::raise_represented` (an ally, its subtype `only_subtype`'s,
  `origin` the card's owner); burned, its card goes to that owner's ash heap
  (`ash_crypt` or `ash`), never burned from play. Uniqueness is
  `GameState::is_unique_minion`; `card_rivals` contests only unique rivals,
  so a mummy of a non-unique vampire contests another such mummy and not the
  vampire (LSJ 20070928-2).
- Clan and sect overrides: `Minion::deranged`, the latest card's (Derange
  set as played, Feral Spirit's `Effect::TargetClanAndSect`); as one
  leaves or moves, `GameState::override_without` restores the latest Feral
  Spirit still on the vampire. `Minion::scarce_paid`: the pool its scarce
  toll took as it left the uncontrolled region (`scarce_toll`; The Path of
  Tears' `GainPoolIfTargetScarcePaid`). New in Town's
  `Requirement::TargetInfluencedSinceYourLastTurn`: entered between a
  seat's minion phase end and influence phase end stamps (Legendary
  Vampire's), after the player's last turn.
- Sect overrides (`GameState::considered_sect`): `Minion::considered` lists
  the overrides in arrival order (Derange, Feral Spirit through
  `Engine::consider`), a borne `Modifier::BearerConsideredSect` not yet
  listed (Writ of Acceptance) the latest; read in `sect_of` over the
  underlying sect, never on an ally. A sect change (`become_sect`,
  `sect_override`) goes under it: `Minion::actual_sect` is what Ravnos
  Acceptance's offers read (rules-qa 111.32). `Bearer::Ally` ("this ally":
  Orb of Ulain's surcharge; its `AllyImmuneToDirectedActions`).
- Samuel Haight's granted political action:
  `RefEffect::SourceAllyBecomesRedList` (`made_red_list` on the ally whose card
  granted it).

### 6.11 Cards in play

`state.rs` `CardInPlay`; `cards/mod.rs`.

- Host: `GameState::set_host` is the one way a card changes minion (every
  move, steal and transfer); a card marked `CountersClearOnNewBearer`
  (Seal of Veddartha) loses its counters there.
  A granted action moving its card does it through `move_source_card`
  (`Effect::MoveSourceToTarget`, `MoveSourceToActor`: Houngan; the card
  stays its player's, `Requirement::CardControllerActive` reading their
  turn).
- An unlock earned by burning a ready vampire or sending one to torpor, in
  combat or by a directed action (`Modifier::UnlockAfterMinionPhaseForClan`,
  Code of Samiel): `unlock_after_minion_phase_earned` in `leave_riders`
  marks the vampire (`Lasting::MayUnlockAfterMinionPhase`), offered
  `Interrupt::MayUnlock` as that minion phase ends, anyone's.
- Own-bleed riders on the bearer (`own_bleed_riders`): Capitalist's
  `Effect::BearerBloodIfBledPrey`, its turn's use spent only when it
  applies (`Requirement::OncePerTurn`, as Rebel's block rider).
- Shatter the Gate (`Effect::ShatterAtCopies`): its third copy on the
  Baali burns them and writes `Seat::shattered` (a `state::Sparse`, hashed
  only when set) on each other Methuselah without one; the pool burns at
  `start_unlock_effects`, no card behind it (LSJ 20100206-2). Hexaped's
  link counters (`Seat::links`, the recruiter `CardInPlay::by` names,
  `Effect::RemembersRecruiter`): +1 stealth and bleed each, `link_bonus`. A
  card's player for Threading the Path of Orpheus is `CardInPlay::owner`
  (`Requirement::TargetCardPlayedByOther`).
- Control: `CardInPlay.controller` is state; `GameState::card_controller` routes
  a minion card to its host's controller, a master card stays its player's
  (rules-qa 7.5) but a trophy on a vampire (Trophies, below) and a stolen
  one (`GameState::take_card`, every steal of a card: Ethan Locke, Absorb
  the Mind [MYT]; advanced rules §6.3.3), `Effect::StillYours` keeps a card its
  player's on another's minion. `Engine::give_card_control` / `give_location` (a
  location on a minion moves onto a minion of the new controller that can hold
  it, a torpid one too, else burns: rules-qa 88.9). The Admonitions passes in
  its controller's discard phase, a due (`Effect::PassesToSect`:
  counter-clockwise, its controller last, else burned). A card entering play
  (`place_now`) fires the other cards' `Hook::LabelEntersPlay` riders for each
  of its labels (a watchtower burns for another). A once-each-turn bonus records
  its turn on the card (`CardInPlay::spent_turn`: Harvest Rites, spent in
  `GameState::hunt_blood`, a hunt at capacity too: rules-qa 89.1).
- A master: Discipline put on rather than played (`put_on_minion`: Childe
  of the Revolution, a diablerist) with `Terms::AnyDiscipline` asks its
  Discipline then (`Interrupt::PlacedDiscipline`); one standing for
  Legion's fledgling has no text (`GameState::text_suppressed`, LSJ
  20071001-1).
- **Oust**: cards the Methuselah *controlled* are removed (not burned, nothing
  triggers, spec §1.10); other players' cards on their removed minions burn to
  their owners' ash heaps.
- Fields: `mode` (the version it entered with), `by` (the minion that played
  it), `against` (remembered Methuselahs, `SeatSet`), `chosen` (Blessing of
  the Name: `ChooseActorAndPicksUnlock` from `Terms::OwnOtherMinionsUpTo`,
  `Modifier::ChosenStat`, its unlock rider `LockChosen`), `on_card` (a card on
  a card: Magazine, Blood Tempering's and Ensconced's `PutOnTargetCard`, `by`
  its player; an action at it read by `DirectedAtCardItIsOn`; Ensconced's burn
  used by `by` itself, `Actor::AnyReadyVampire` with `ActingIsBy`), `stored`
  (library cards held on it, burned with it to `stored_owner`'s ash heap if
  set: The Erciyes Fragments' prey, Storage Annex's last storer), `counters`,
  `entered`, `effect_only`. Storage Annex stores a hand card as it is played
  (`Interrupt::StoreOwnHand`: `left`, `of`, `replace` — Gift of Proteus'
  several, unreplaced) and swaps one in its master phase
  (`Effect::SwapPickedWithStored`, in `ability_discards_done`, no
  replacement); another's card it puts in a hand is recorded by copy
  (`GameState::foreign`) and routed back to its owner as it leaves the hand
  (`settle_foreign_cards`, each `run`; rules-qa 96.2).
- Riders (`Rider { hook, scope, requires, effects }`, `RiderScope::{Every,
  Controller, Others, Prey}`): requirements read at collection and at
  resolution (`rider_fires` again as a chosen rider resolves; an unmet rider is
  not pending, its window elides); unlock riders are pending mandatory effects
  ordered by their resolver (DESIGN §10.7), but for one whose condition is read
  as the phase ends (`Hook::UnlockPhaseEnd`: Anarch Revolt's), resolved as its
  Methuselah passes (rules-qa 88.5); a burn-when-empty one (Powerbase:
  Barranquilla, room for Church of the Order of St. Blaise, LSJ 20050105-1)
  pending again after its card's ability; one burn per copy; a counter
  moved to the pool (`CounterToPool`: Powerbase: Mexico City) before a
  later `BurnSelfWithoutCounters` of the rider reads it;
  `R::OneCopyPerTurn` drops the other copies once one resolves. An unlock rider
  paid unit by unit as it resolves (Smiling Jack: pool or a vampire's blood,
  per counter) runs through `Flow::RiderTerms`. `GameState::card_riders` lists
  every arena card's riders on a hook, in play or not, in arena order (one per
  hook, but for `Hook::UnlockPhase`). Every hook site collects through
  `Engine::fired_riders` and `Engine::printed_riders` (§2). `HookAt::seat` is
  the active Methuselah at a phase hook, the seat of an "own" event, none at a
  host-anchored hook; `Hook::reads_action` (`HostBlocks`, `HostBlocked`,
  `OwnBleedEnds` for cards; `ActionAnnouncedAtTarget` for vampires) reads
  requirements in the action's context, the card's rider predicates besides
  (`rider_card_holds`, `EnteredBeforeThisAction` among them). `HostBlocks`
  reads an ally's own card too (`bearer`): its `BurnSelf` burns the blocker,
  and the action is then blocked with no combat (rules-qa 53.1: Member of the
  Entourage). `Hook::HostActionSuccessful` fires beside `ClanActionSuccessful`
  on the acting minion's cards (Journal of Hrorsh), then
  `HostDirectedActionSuccessful` for a directed action (Blood of the Sabbat). A
  site applying effects one by one re-checks `present` before each;
  `leave_riders` keeps its pool burns for its end (rules-qa 6.9). Left outside
  the collector on purpose: the resolution of a rider already chosen (draw,
  replacement, master/discard phase choice), Saatet-ta's destination as she
  burns, entry riders read on a crypt card before the vampire is in play, the
  leave-play readers (torture, hostages). The hooks and their tenants: the
  `Hook` enum's docs (`cards/mod.rs`); every hook has tenants (DESIGN §7.4).
- Entry: `enter_play` is the single entry point (contest checked by the
  caller after entry effects); `apply_entry_effects` (a retainer's layers on
  its employer, an ally's on itself); `push_mode_layers`. "Put this card on
  … / in play": `place_now` as the card resolves (the card itself; its
  layers from the mode's `Effect::Layer`s or pushed, `ModeLayers`; "only
  one" of its name, `holds_its_name`; a title card's title), `place_held`
  once it is in the ash heap (rules-qa 21.1, 47.1). "Only one" as the card
  is played is a mask from definition data (`ActorWithout`,
  `TargetWithout`, `OpponentWithout`, `ActingHoldsNone`); as it resolves,
  a legal no-op (rules-qa 17.4). On the acting minion by a reaction
  (`Effect::PutOnActingNow`: Shattered Mirror), its Methuselah's card
  (`card_controller`).
- A card that becomes another (`cards::became`): its windowless mode only.
  Living Manse is equipment to equip and a location in play
  (`cards::is_equipment_in_play`; out of `equipment_on`).
- Powerbase: Luanda's choice (`Effect::CardChoosesTarget`) gates its granted
  action (`Requirement::ActorIsChosen` with `CardLocked`: the ability gate
  reads `CardLocked` for `ActingCandidate`); `give_card_control` clears it.
  A choice for the current action only (Pentex™ Loves You!):
  `Effect::ChooseTargetForAction` into `ActionCtx::chosen_for`, read by
  `Requirement::ActorChosenForThisAction`.
  419 Operation: the prey's use (`Actor::ControllersPrey`),
  `Effect::PreyPoolToCounter`. Modifiers by sect (`Modifier::SectStat`,
  `SectHuntToCapacity`: Festivo's top-up in `successful_hunt_riders`) and
  for the controller's minions (`Modifier::OwnMinionsStat`, in
  `bearer_mods`: Esprit de Corps).
- Curse of Nitocris passes with the Edge (`Effect::TargetSeatTakesCard`,
  `ControllerGivesCard`, `Interrupt::GiveCardTo`). Arcanum Chapterhouse's
  `Effect::BurnPoolPerHuntingGround` with `RiderScope::PredatorOrPrey`.
- A stand-in (`effect_only`, no copy) keeps an effect a burned card leaves
  behind: Elixir of Distillation's mode 1, with an unsatisfiable requirement
  rather than empty windows: an empty mode means `became`, and suppresses the
  card's abilities.
- Trophies (`Effect::Trophy`, `cards::is_trophy`): in play by a master phase
  action, no effect until awarded: no layer pushed while hostless (the play's
  `Effect::Layer` arm), `push_mode_layers` on the award.
  `Interrupt::AwardTrophy` (the Red List rule) stays until a pass: every
  unawarded one in play may follow, one searched (`searched`), none past
  `Effect::TrophyCap`; `Interrupt::TakeChosen` only for a trophy with blood
  (`cards::trophy_blood`). On a vampire, a trophy is its controller's
  (`card_controller`).
- Trophy entries (round 33): a trophy's `Terms::Discipline` or
  `Terms::Clan` is chosen as it is played (`PlayCtx::discipline`,
  `PlayCtx::clan` → `CardInPlay::chosen_clan`), or as it is awarded from out
  of play (`trophy_moved`: `Interrupt::PlacedDiscipline`,
  `Interrupt::PlacedClan`); `trophy_moved` also reads
  `Effect::CountersWhenAwarded` (Trophy: Wealth) and
  `Effect::AwardBecomesVampire` (Trophy: Progeny: the card becomes a
  created vampire of the awardee's clan and sect, `card_becomes_vampire`,
  Childe of the Revolution's body too, with a Discipline master search);
  `Modifier::BurnsAfterBearerDiablerizes` burns in `end_action` when its
  host is `ActionCtx::diablerist` (Trophy: Diablerie).
- A hidden chosen (Betrayer): `Modifier::ChosenHidden` makes `legal_view`
  clear `CardInPlay::chosen` for all but the card's controller
  (`GameState::chosen_seen_by`); legality reads only `against`, which
  `settle_bound_burns` keeps on the chosen's controller
  (`AgainstFollowsChosen`; none while it is contested); `BurnsWithChosen`
  in `burn_minion`; the guess is the unlock rider's
  `Effect::GuessChosenForPool`, a `MinionPurpose::GuessChosen` among the
  guesser's vampires. The host tells no effect naming it and the view has
  no `chosen`.
- Round 33's other card-in-play modifiers: `HostDamagedOnUnlock` (The
  Treatment) and `BurnIfOthersControlSubtype` (Glass Walker Pact) in
  `settle_bound_burns`; `ControllerMasterPhaseActions` and
  `ControllerDiscardPhaseActions` (Not to Be) read as each phase begins;
  `CollectsInfernalUnlockPool` (the `UnlockInfernal` arm) and
  `CountersBurnedInPredatorsDiscardPhase` (`end_influence`: Ruins of
  Charizel); `UntargetableByOthersWhileReady` in `may_direct_at` (Trophy:
  Safe Passage); `ChosenClanCannotBlockBearer` beside Incriminating
  Videotape's bar; `BearerChosenSuperiorDiscipline` beside The Sargon
  Fragment's; `CostSource::BearerEquip` (Trophy: Wealth) a counter source as
  Hackerspace's, its host's equipment only. `Interrupt::BurnLocationsForPool`
  (Insurance Scam) and `MinionPurpose::BloodFromTo` (Glass Walker Pact) pick
  as the ability resolves; `Effect::SearchEmploy` is shared (Trophy:
  Retainers' employ, no action).
- `Modifier::HostUnlockBound` (Anima Gathering): `GameState::unlock` records
  the minion in `unlocked_now`, drained by `settle_bound_burns`, which burns
  the card; a Banishment return is no unlock. `HostMayStayLocked` asks
  `Interrupt::MayUnlock` in the unlock phase instead of unlocking.
- `Modifier::HostReactionsCanceledWhileLocked` (Blood of Sandman):
  `open_as_played` cancels a Reaction card its locked, non-acting bearer
  plays, the cost paid.
- Will of the Council: `Modifier::BurnToPlayAsPrince` meets a prince
  requirement in `plays_as_sect_or_title`, `announce_play` burning the
  card (`will_for_prince`) and marking `PlayCtx::as_prince` for the
  resolution; `MayTakeTorporedPrinceTitle` asks `Interrupt::MayTakeTitle`
  as a prince goes to torpor (`minions_to_torpor`). Both carry the title
  their bearer must still hold (`bearer`: "this primogen", D31), read live
  by `GameState::prince_by_burn` and `may_take_princes_title`.
- `plays_as_sect_or_title` takes the played card: Karsh's `PlaysAsPrince`
  is for Blood Hunt only; `Stat::PlaysFromOwnAsh` adds the card from the
  player's ash heap to the virtual hand (`state::ASH_GRANTED`).
- High Museum of Art, Atlanta: `Modifier::ControlToll`;
  `Engine::settle_control_tolls` compares each such card's controller, none
  while contested or out of play, with the one last seen
  (`GameState::control_tolls`): a loss burns, a gain gains (its entry gain is
  its play's). Bounty: `Effect::CountersToVanquisher` at `HostToTorpor` (in
  combat: `Interrupt::TakeCountersUpTo`, the card burned emptied) and at
  `BearerBurned` (the combat opponent, else a diablerie action's acting
  vampire). Social Ladder: a Frontal Assault-like influence due
  (`influence_burns_due`), its blood through `Interrupt::UncontrolledBlood`'s
  `older_than`.

- Shell Break: `Effect::BecomesRetainerOnBy` at `BearerBurned` moves the
  card to `CardInPlay::by` (in play, not its bearer) and sets
  `CardInPlay::became` (the subtype), its layers pushed then (the employer's
  +1 bleed and intercept; `Modifier::RetainerImmuneNonAggravated`, read where
  a retainer takes damage: a strike, Weather Control). A card in play is a
  retainer by `GameState::is_retainer`, a filter on it by
  `GameState::card_in_play_matches` (the card as it became): target
  filters, the retainers on a minion (combat damage, moves, text loss),
  the counted cards (Can't Take it with You), a referendum's chosen card.
- Khazar's Diary: `Hook::UniqueMinionBurned` (fired at the end of
  `burn_minion_now`, any controller, `is_unique_minion` read before the burn);
  `Modifier::ClanSuperiorAtCounters` read in `granted_level`; its granted
  action names the ash heap's minion card with `Interrupt::NameAnyAshMinion`
  (`ActionCtx::ash_minion_named`), fizzling if it, or the Diary's counters,
  are gone (`ability_void`). An `ActingCandidate` ability reads its card's
  `CountersAtLeast` (`ability_holds`).
- Spell of Life: `Requirement::CopiesInPlayAtLeast` counts any Methuselah's
  copies (`GameState::copies_in_play`);
  `Effect::BurnCopiesToRepresentUncontrolled` burns the copies in play,
  the user picking which, anyone's, when more are in play
  (`Interrupt::BurnCopiesPick`, LSJ 20090428), and asks the vampires one at
  a time (`Interrupt::RepresentPicks`).
- Retain the Quick Blood: `Modifier::SpentBloodToCard` on a vampire;
  `GameState::keep_spent_blood` at each blood payment of a card (a play's
  `pay_play_blood`, an action card's cost in `succeed`, a bleed card's, a
  political card's, an employ or recruit card's) adds the blood to such a
  card when the played mode requires one of its Disciplines: the one, or,
  with several, the one the vampire's controller picks
  (`Interrupt::KeepSpentBlood`, returned by `keep_spent_blood`; options
  `spent_blood_keepers`, live).
- Set's Curse: `Effect::BecomesAllyIfTargetBurned` enters the card as an ally
  only if the mode's burn took the target out of play (Ilomba keeps itself);
  `GameState::ally_cost` is 0 for a card that is no ally card, a represented
  crypt card's capacity (rules-qa 121.131), and the corruption steal
  (`capacity_or_cost_by`) and pact counters read it;
  `Modifier::BearerNoActionCards` masks its action cards beside the printed
  `Stat::NoActionCards`.
- Round 30's cards on a minion: card riders at `Hook::HostEntersCombat` fire
  in `begin_combat` beside the printed ones, every new combat (Blithe
  Acceptance, Fantasy World); the bearer bars `Modifier::BearerPlaysNoCards`,
  `BearerNoReactions` (actor and reaction candidates),
  `BearerImmuneNonAggravated` (with `immune_non_aggravated`, aggravated damage
  treated as normal included, LSJ 20040120) and `BearerInflictsNoCombatDamage`
  (`damage_reduced`); `CostSource::HostDisciplineCounters` (Fatuus Mastery)
  pays its host's blood costs of cards played with the Discipline; a vampire
  bearing an `AsVampire` grant meets one combat card requirement with it
  (`grant_fits`, Bestow Vigor); `Requirement::ChosenInPlay` and
  `Effect::BurnChosenMinion` hold The Eternal Mask to the incarnation it raised;
  `CardInPlay::chosen_card`, `Requirement::ChosenCardInPlay` and
  `Effect::BurnChosenCard` hold Bauble to the equipment it put on (a card
  back in play is a new id: LSJ 20040726), `ChosenCardBearerNotYours` its
  burn once that bearer is another Methuselah's.
- Charigger, The Axe: `Modifier::RemovesPreyBurningMinions` removes from the
  game a minion its controller's prey controls as it would burn
  (`Engine::prey_burn_remover`), before anything keyed to the burn, a counter
  on the card; at a diablerie or Sacrificial Lamb, up front, as Byzar's, the
  diablerie unsuccessful (rules-qa 121.20).
- Standing burn replacements (`Engine::burn_replacements`, read live: the
  minion's own, Byzar's; Book of Going Forth by Night's; Charigger's; The Name
  Forgotten's): `Engine::burn_replaced`, called by `burn_minion_now`,
  `diablerise_to` and Sacrificial Lamb, applies a lone one at once; with two or
  more it pushes `Interrupt::BurnReplacement` for the acting Methuselah (else
  the active one), the minion left in play until the answer while the burning
  effect goes on; settled with fewer than two left, it applies the one left or
  burns (rules-qa 121.19).
- "If this card is unlocked at the start of your turn" (Club Zombie):
  `Requirement::CardUnlockedAtTurnStart`, read in `rider_card_holds` from
  `GameState::unlocked_at_turn_start`, written in `begin_turn` before
  anything unlocks, only for cards with a rider reading it. Its gain:
  `Effect::ControllerChoosesVampireGainsBlood` →
  `MinionPurpose::GainBlood` (mandatory, a ready vampire, a full one
  legal). A vampire's (Dr. Jest): `Requirement::HostUnlockedAtTurnStart`
  from `GameState::vampires_unlocked_at_turn_start`, written beside it.
  Doyle Fincher's `Effect::PreyBelowCapacityVampireAndHostGainBlood`, in
  `discard_phase_vampire_riders`: `GainBlood { below_capacity: true }`.
- `Effect::PutOnActingNow` checks "only one" (`place_now`'s flag) only for
  a card whose mode masks its own name (`ActingHoldsNone`: Shattered
  Mirror); Vendetta and Fata Amria stack. Vendetta's
  `Modifier::ClanStrengthVsBearer` in `GameState::strength_against`.
  Lobotomy's `Modifier::BearerNoManeuver` in `Combat::may_maneuver`.
  Cardinal Sin: Insubordination's `Requirement::TargetFoughtPlayer` reads
  `ActionCtx::combats` (every combat pair of the action) at the play-target
  site.
  `Effect::LookAtBledLibraryTop` and `PeekBledHand` are shared (Courier's
  ability, Patterns in the Chaos' play); Inconnu Tutelage's
  `Effect::RemoveUncontrolledTarget` an ability's (`remove_uncontrolled`).
- A vampire's "would be burned" card (Abandoning the Flesh,
  `Window::VampireWouldBurn`, `Actor::BurningVampire`): `Engine::ask_burn`,
  not asked while a standing replacement applies (it applies instead,
  rules-qa 122.2), in `burn_minion` (its body `burn_minion_now`, which a
  blood hunt's burn calls, LSJ 20070417) and before a diablerie's steps at
  each site (`control::BurnThen`: the diablerie action, a torpor block,
  Amaranth and Liquefy in combat, Sacrificial Lamb), when its
  controller holds a play: `BurnWait` in `GameState::burn_wait`, behind any
  burn already waiting (two vampires burned together, each its controller's
  asked in turn); the vampire gone once the play ends
  (`Effect::RemoveActorFromGame`), the site's "replaced" end runs
  (`burn_saved`: an unsuccessful diablerie), else the burn goes on
  (`burn_goes_on`), at once on a pass. The card's type gates its actor (D18):
  a combatant, or another Methuselah's action under way (`GameState::acting`,
  an action card's effects resolving with the action out of `Engine::action`).
- Jungle Hunting Ground: `Effect::AddCounterBelow(n)` and
  `CountersToTargetBloodFed` (the counters as blood, fed either way). The
  Slaughterhouse: `Effect::BurnPreyLibraryTop`. Secret Passage's
  `Requirement::HostReady` read on the card's bearer for a controller's
  ability (`holds_for_card`). Call the Great Beast's 11th counter: its own
  ability, `CountersAtLeast(10)` with `ActorNotSterile` (`CountersAtMost`
  read by `ability_holds` for an acting candidate's card).
- `Modifier::HostBurnsOnAttempt(n)` (Art's Traumatic Essence [MEL]): as
  Lock's `HostBurnsToAttempt`, for any minion (`GameState::attempt_burn`,
  `Engine::burn_to_attempt`, every such card). `Effect::BurnAtInfluence`
  (Choir) joins the influence dues (`influence_burns_due`), spared by
  `Effect::SpareNamedAtInfluence` for the turn (`Seat::spared_at_influence`,
  Hanging Fermata, played in `Window::InfluencePhaseOwn`;
  `Requirement::NotPlayedByYouLastTurn` reads `Seat::played_at`).
  `Effect::BurnsLeavingUncontrolled` (Founders of the Ebony Kingdom): a slot
  card burned instead of hosted as its vampire leaves the region
  (`Engine::slot_cards_onto`). A bearer's `Stat::AggravatedAsNormal` layer
  (Armor of Terra) applies in combat and out (`bearer_always`), Gotsdam's
  printed one in combat only. `Effect::HostBloodToCapacity` (Concert Tour's
  unlock-phase rider); `Modifier::HandSizePerCounterWhileBearerReady`
  (Hourglass of the Mind [TEM]); `Effect::UnlockTargetHostAtDiscard`
  (Cleansing Ritual [OBE]).
- No Secrets From the Magaji: `Modifier::HostBlocksVampiresLocked(Bearer)`
  (`ActionCtx::blocks_locked`: a candidate and an attempt though locked,
  no wake), `Modifier::BurnsWhenHostFailsToBlockVampire` (read from
  `ActionCtx::failed_attempts` at each settle and as the action ends,
  `failed_block_burns`), `HostInterceptVsVampires { n, who }`;
  `Bearer::Title(class)` reads the title live, Vlad Tepes meeting any.
  Steal the Mind: `Modifier::ByInterceptVsBearer` (the card's `by`, in
  `host_intercept`'s `against`, so `block_can_reach` sees it),
  `BearerBleedAgainstByController` (`bleed_against_mod`). The Wildebeest:
  `Modifier::BearerCannotUseEquipment` (`GameState::equipment_unusable`
  beside a combat's bar: its equipment's Disciplines, bonuses and abilities
  not its, the side barred as a combat starts; equipping allowed),
  `NoRetainersHeld` (`no_retainers`, `can_hold`), `Effect::BurnActorRetainers`.
  Unburdening the Bestial Soul's unlock-phase rider
  `Effect::ByBurnsBloodOrThisBurns` asks the card's `by` (`BloodOrBurned`).
  Shadow of Taint: `Terms::CardDestination(Holder)` chooses the minion as
  played (`Engine::card_destination_ok`; `Holder::WherePlayable` reads the
  card's played mode's target filter for its player, as if its `by`
  played it now), `Effect::TargetCardToChosen` moves it at resolution.
  Persistent Echo: `Effect::StoreOneFromHand`, the marker
  `VampiresPlayStoredAsIf { discipline, superior }` (a store's markers read
  in the card's played mode, `store_markers`; any vampire of its
  controller's, `cards::met_lending` the lend; burned as played, as Gift of
  Proteus's).

- `Effect::ReturnsHostCardToHand` (Permanency [CHI], put on the action's
  card by `PutOnActionCard`): `burn_card_in_play` sends that card to the
  marker card's controller's hand instead, the cards on it burned as it
  leaves play.

### 6.12 Printed vampire text

`cards/vampires.rs`: `VampireDef`, `Printed`.

- `crypt.rs` stays generated; behavior is `VampireDef` keyed by name. Plain stat
  modifiers are **not layers** (a vampire is not a card in play): read live by
  `GameState::printed_mod`, so nothing is pushed or retracted on torpor, contest,
  burn or oust; it reads only the minions whose text prints the stat
  (`cards::printing`, a static index of the definitions, never a state
  cache). Printed text applies in torpor; a contested vampire gives nothing.
  Abilities are `ModeDef`s with actor `Host` = the vampire;
  `AbilitySource::{Card, Vampire}` keys uses and resolution.
- A vampire may land with part of its text, the rest quoted in `incomplete`
  (rules-qa 13.1; none left); fixtures wanting a plain vampire pin one by name.
- Printed predicates read `PrintedCtx`: the action, the opposing minion
  (the blocker for a block toll, the calling vampire for a vote toll) and
  the card played (`PrintedCtx::card`, `R::PlayedCard`). The
  printed-modifier ones (the `Requirement` variants from `ThisVampireReady`
  on) read three minions: the vampire whose text it is, the modified
  minion (the actor), and the opposing one (the acting minion for a block
  attempt, the opposing combatant in combat). Printed cost
  reductions (`GameState::printed_cost_off`: pool, blood, "blood or pool")
  apply wherever the minion pays a card's cost, a card another puts in play
  included — it is still played (Piper, Contraband, a search; rules-qa
  41.13); Regina Blake's blood is announced with the card like a reduction
  (`PlayCtx::blood_payer`, `Engine::blood_payers`), paid with the named or
  found card's cost in `Engine::pay_named_discounts`.
- A card another card plays or puts in play (`Effect::EmployNamed`,
  `EquipNamed`, the searches, `Engine::put_on_minion`,
  `Interrupt::PutInPlaySearch`) is played by that card, not in the normal
  fashion (rules-qa 119): it never goes through `announce_play`, so no
  as-played window opens and nothing keyed to who plays a card runs (a
  recruit or employ rider still does: Baba Yaga, PIB 20150105-2). What
  reads the card itself is applied where it is placed or paid: "only one
  … can be played or equipped in a game" (`Engine::note_once_played`,
  `once_played_out`), and what it costs its minion more
  (`Engine::card_surcharge`, beside the reductions above).
- "Plays cards requiring a <clan> as if" (`VampireDef::plays_as`) counts at
  card plays only (`GameState::meets_clan`: clan requirement, target, terms,
  `R::ControlsReadyClan`); a card's effects in play read the clan itself
  (`GameState::of_card_clan`; `Effect::ChooseActor` chooses no vampire
  playing as the clan). Mata Hari's `Stat::PlaysAsAnySectAndClan` is any
  clan in `meets_clan` and any sect in `plays_as_sect_or_title`. Allies of a
  label recruited "as if he met the requirements" (Winterlich, Petaniqua:
  `VampireDef::recruits_as_if`) count at the recruit's clan check and its sect
  and title (`plays_as_sect_or_title`), from hand only, never for a burn option
  (LSJ 20050225); a searched ally (The Summoning) reads the vampire's own sect
  and title, never Vlad Tepes's as-if either (`recruit_versions`, LSJ 20050119).
- Vampire unlock-phase riders are pending mandatory effects
  (`Pending::Printed`, `Choice::ResolvePrinted`); `Stat::NoDirectedActions`
  filters the minion phase's offers (`Engine::directed_actor`: a Red List
  hunt or an imbued's burn at another's minion directed too), and
  `Stat::NoUndirectedButHunt` (Pariah) the undirected ones but hunts, a
  card's included; a combat an ability begins after resolution returns to that
  window (`CombatCtx::after_resolution`).
- `Stat::ActionCost` is any action's surcharge, read with its kind and
  directedness (Daria's directed actions, Marcellus's political ones;
  `Engine::action_toll`): masked at announce with the action's own blood
  (`announced_blood`), paid first (`pay_surcharges`; short: paid as far
  as it goes, the action fizzles). `Stat::ActionPoolCost` (Lorrie
  Dunsirn) is its pool twin, paid last among the surcharges, never
  masked (D3). The card the action burns (`PrintedCtx::burned_card`, Hiram
  "Hide" DeVries's locations) is read off its card's or granted ability's
  effects (`actions::burned_card`): the target of `BurnTargetCard`, or the
  source of a `BurnCard` action. `Stat::StrikeCardToll` (Djeneba,
  `strike_card_toll` inside `combat_card_surcharge`): on the opposing
  combatant's strike-mode cards (`plays::is_strike_mode`), an ally paying life.
  Liliana's seven cards: `Effect::RemoveOwnAshCards` →
  `Interrupt::RemoveFromAsh` on her own heap (one pick a step; burned vampires,
  `Seat::ash_crypt`, count and are offered, as for Shroud of Decay).
- Ability limits: "each combat / round" count per copy
  (`AbilitySource::use_key`, Kevlar Vest's branches share one use); "during
  an action" per copy and ability (`AbilitySource::action_key`, Bone
  Shambler's two, rules-qa 42.9); both through `limit_slot`'s ledgers (§2).
- A vampire's ability its controller's prey uses (`Actor::HostControllersPrey`,
  Keith Moody): `vampire_ability_options` also walks the user's predator's
  vampires; `ability_user` and `resolve_ability` resolve for the prey.
  `Hook::PoliticalReferendumPassed` (Lutz) and `Hook::MinionPhaseEnd`
  (Tupdog) are vampire riders read on every vampire in play / the active
  Methuselah's.
- Another Methuselah's minions using a vampire's text:
  `Actor::OtherMethuselahsMinion` (an action, Calebros) and
  `Actor::OtherMethuselahsVampire` (any window, torpor too, Mustafa);
  `vampire_ability_options` walks every other seat's vampires, `ability_user`
  makes the actor's controller the user and the vampire the host.
- Bars and tolls printed on vampires, read where the engine offers or pays:
  `Stat::NoPoliticalActions` (Bela: the action offers and the political
  card predicate), `NoVotesAgainstBloodHunt` (Troius,
  `ReferendumCtx::barred_against`), `InterceptAgainst` (Rutor),
  `NoRetainerDamage` (Carolina), `TitleContestToll` (Vasily,
  `GameState::title_contest_cost`); a printed rider on its own torpor
  (Thomas De Lutrius, `leave_riders`); `Stat::BurnedAfterTorpor` (Julius:
  burned once in torpor, after the torpor riders, `minions_to_torpor`);
  `Stat::BurnsWhile` (Al-Muntathir, Obaluaye with
  `Requirement::ControlsOfClan`: burned as each step settles,
  `Engine::settle_printed_burns`); `Stat::CannotAct` (Toy: no action
  offer, `GameState::only_actions`); `Stat::NoBloodHuntIfDiablerized`
  (Harlan Graves: read as `diablerise` begins, `ActionCtx::no_blood_hunt`
  bars that diablerie's hunt, taken by it). Grotesque's
  `Effect::BindLocationOnEntry`: an entry rider asking
  `Interrupt::BindLocation` (`GameState::bindable_locations`; none,
  removed at once), `Minion::bound_location`; the same settle
  removes it from the game once its controller no longer controls that
  location (`GameState::controls_location`).
  Printed surcharges read the card played (`PrintedCtx::card`):
  `Requirement::PlayedInStrikeMode` (Antoinette DuChamp, a dodge
  included).
- Lord Ephraim Wainwright (`Stat::MillsPlayedActionCopy`): after an
  action card he plays (declared or canceled: `Engine::may_mill_copy`),
  `Interrupt::MayMillCopy` mills a library copy, its card recorded in
  `ActionCtx::copy_back`: any copy of it in the ash heap (copies not told
  apart there) goes back to hand as the action succeeds, before the played
  card reaches the ash heap (`finish_action`, a fizzle included). Matthew's
  `R::InOwnMasterPhase` reads `PrintedCtx::master_phase_of`, which
  `Engine::seat_reducers` fills from `GameState::in_master_phase`
  (state, a `Sparse` hashed while set: the flow is out while a choice
  applies).
- Round 28's looks and bars: `Stat::LooksAfterMaster` (Troglodytia) peeks
  at the hand of a Methuselah who played a master, after its replacement,
  in `Engine::others_master_draws` beside Sascha Vykos's draw;
  `Stat::ShowsReferendumReplacements` (Slag) notes what another seat's
  replacements put into its hand during a referendum
  (`Engine::show_referendum_replacements`, from `replace` and from a
  political card burned for a vote, where the referendum context is out;
  `show_referendum_choice` for one chosen in an interrupt: a draw
  alternative, the ash heap, the research area);
  `Stat::StrikesOnlyDodgeOrCombatEnds` (Tanginé against a Salubri,
  `GameState::strikes_only_dodge_or_ends`) refuses other strikes in
  `CombatCtx::allows_from` and puts `Pass` in place of the hand strike;
  `Per::OwnReadyUniqueInfernal` (Xeper).
- An ability's Discipline term (`Terms::InferiorDiscipline`, Camarilla
  Vitae Slave: `ability_discipline_apply`, gated on one at inferior);
  `Effect::ActionCardToLibraryShuffled` sends the action's own card to the
  library as the action ends (Epikasta, `ActionCtx::card_to_library`).
- Vampire abilities reading a bleed's, a blood hunt's or a hunt's outcome:
  `Actor::BledMethuselah` (Alex Wilkins) is the bleed's target as it resolved,
  walked in `vampire_ability_options`, its `Effect::StealHost` for good;
  Count Zaroff's cancel is a `Window::BloodHuntWouldBurn` ability on himself
  (`Effect::SaveFromHuntBurn`, as Dogs of War's) and
  `Effect::HostToUncontrolled` banishes him;
  `Stat::BloodPerOnlyAgainstMethuselah` (Carlak) is read at every tally, before
  the kinds part. The Ankou: a printed `Hook::VampireHuntsSuccessfully` rider
  (`Effect::MarkHunterForHost`) writes `GameState::hunt_marks` as a hunt gets
  its blood, read by `Requirement::TargetMarkedByHost`; his action is also
  offered in `Window::OthersMinionPhaseEnd` (`may_take_granted`'s `out_of_turn`,
  the Cog's moment), the active phase going on after it. A `CostAtMost` card
  target reads an equipment's cost with its bearer's reductions (Ivan
  Krenyenko).
- Vampire abilities over a Methuselah's vampires, damage kinds, blocks and
  cancels: `Actor::AnyMethuselahsVampire` (Johan Wrede) is Mustafa's walk with
  its own Methuselah's vampires; `Stat::AggravatedAsNormal` (Gotsdam) is read
  with Jua Vema's in `apply_damage_now`;
  `Stat::LockFailedYoungerBlockersAfterResolution` (Gwyedd) as
  `after_resolution` ends; `Stat::CancelerBurnsPool` (Johannes Castelein) in
  `canceler_toll`, from both as-played cancels and both referendum cancels;
  `Hook::MasterLocationEntersPlay` (Joseph DiGiaccomo) fires the printed riders
  in `place_now`.
- Vampire text hooked on bleeds, havens, weapons, diableries and control:
  `Hook::BledSuccessfully` (Keller Thiel), read on the bled Methuselah's
  vampires by `Engine::bled_riders` in the bleed's successful branch after the
  Edge, not once they are ousted; `Stat::BurnsHavensReceived` (Lynn Thompson)
  burns a `Label::Haven` card on her in `settle_bound_burns`, however it came;
  `Stat::WeaponDamageImmune` (Mictlantecuhtli,
  `GameState::weapon_damage_immune`): a weapon's strike lands nothing on him
  in `resolve_strikes` (inflicted, not successfully: `CombatCtx::immune_hit`
  counts it for an "inflicts", rules-qa 116.138; its added damage too; his
  retainers still hit), nor a Ghoul Retainer's weapon use, nor Talbot's
  Chainsaw's unlock damage (`MinionPurpose::WeaponDamage`, picked by a weapon's
  `ControllerChoosesMinionDamaged`), while a weapon's own damage on its bearer
  stays environmental; `Stat::OwnReadyVampiresGainOnDiablerie` in
  `diablerie_burns`, before the blood hunt, printed or borne (Codex of the
  Damned, with `Stat::OwnYoungerVampiresMasterOnDiablerie`: a
  `DisciplineSearch` each, the younger read before the victim burns; not
  under an equipment bar); `R::DirectedAtThisVampire` (MacAlister:
  `PrintedCtx::target_minions` holds the text's vampire);
  `ControlTerm::UntilMinionPhaseEnd` (Miguel Cordovera, an
  `Actor::HostControllersPredator` ability:
  `Minion::temp_until_minion_phase_end`, returned by
  `Engine::minion_phase_controls_end` as the minion phase ends, a cut-short one
  in `next_turn` too); `RefEffect::BurnChosenCard` (Montecalme);
  `Effect::RemoveHostFromGame` used by a vampire removes the vampire (Philippe
  de Marseilles). `Stat::MasterDisciplineTrifles` (Reiner Stoschka,
  `GameState::master_discipline_trifle`) at the trifle refund in `resolve_play`;
  `Stat::PreyHandSize` (Tomaine) in `hand_size`, read on the predator's vampires
  with the prey as `PrintedCtx::target`; `Stat::LockPaysLocationPoolWithBlood`
  (Zelios: `PlayCtx::blood_locker`, `Engine::location_blood_lockers`) announced
  with the card as Jack's lock, paid in `take_announced`;
  `Per::UniqueHuntingGroundsInPlay` (Santaleous); `Terms::ActingHandCard`
  (Sergei Voshkov: the acting Methuselah's hand, replaced unseen);
  `Effect::OtherReadyVampireDamaged` (`MinionPurpose::DamageOtherVampire`,
  Shiloh Marie); `Effect::OtherReadyMinionDamaged`
  (`MinionPurpose::DamageOtherMinion`) and `BurnOneHeld` (Rico Loco's unlock
  rider; his master-phase `NamedOntoHostElseDamage` asks
  `Interrupt::NamedOntoOrDamage`, the card moved as Dagger's second copy).
  `Stat::EachMethuselahHandSize` (Servius Marius Pustula) in `hand_size`, every
  printer's, the seat as `PrintedCtx::target`
  (`Per::TargetReadyVampiresCapacityAbove`); `Per::Diableries` (Mugur Sabau,
  `Minion::diableries`); `Stat::DoesNotUnlock` in `GameState::does_not_unlock`
  (Raphael Catarari, `R::SourceBloodOdd`: read as the unlock phase unlocks, LSJ
  20060911). `host_blocked_riders` reads a printed rider's `IsBleed` and
  `IsPolitical` by name (Tears) and `Effect::PreyAshToLibraryTop` (Schuyler,
  `Interrupt::AshToLibraryTop` for his controller's prey).
- Vidal Jarbeaux (`Stat::MeetsRequirementsOncePerGame`): `cards::MetKey` (a
  clan, sect, title, the nondescript votes title for "titled", Black Hand,
  Seraph, slave, circle, a Sabbat path, flight, the "non-" traits `Untitled`,
  `NotRedList`, `NotSterile`; never advanced), each spent once a game into
  `Minion::met_once` with the card it met. `play_options` offers a card whose
  clan, path icon, `ActorSectIn`, `ActorTitleIn`, `ActorBlackHand`,
  `ActorSeraph`, `ActorSlave`, `ActorUntitled`, `ActorNotRedList`,
  `ActorNotSterile`, circle or (his Methuselah's master) `ControlsReady*`
  requirement he meets only so (`GameState::once_key`, `cards::meets_once`: an
  alternative unspent, a title bringing its sect; an `AnyOf` none of whose
  alternatives he meets, `cards::any_of_once_keys`: each one requirement short),
  flight through `disciplines_met`; `Engine::spend_once_keys` spends them as
  `announce_terms` opens, canceled cards too, a requirement with several
  alternatives left asked first (`Choice::MeetAs` in `PlayTerms`,
  `PlayCtx::meeting`; this play's keys from `PlayCtx::met_from`). For the card's
  effects (`cards::met_once_for_card`): its action's sect and title re-read
  (`card_traits_lapsed`), a referendum term's as-if (`ReferendumDef::feasible`,
  `minion_choices`); never its effects in play (`Effect::ChooseActor` does not
  choose him), a card another names (`meets_clan` untouched: Piper, Concealed
  Weapon) nor a burn option.
- Round 30's vampires: `Stat::NoLockForBlocking` read on the blocker as the
  block succeeds, the acting minion its opponent (Durga Syn; Reverend Adams'
  `BlockerDoesNotLock` is the acting minion's side); `Stat::UntargetableByCard`
  (Johann Matheson) in `GameState::shuns_card` for a minion target and, for a
  card with none, on the minion `Engine::aimed_at` names (a combat card's
  opposing combatant, a reaction's acting minion) in `play_options`;
  `Hook::HostAttemptsBlock` printed riders fired in `pay_block_toll` (every
  attempt, forced ones too): `Effect::ActingNoModifiersRequiring` fills
  `ActionCtx::modifier_bars`, read in `play_options` on the acting minion's
  action modifiers by the mode played, outliving Arnulf;
  `Minion::founder_counters` (`Effect::HostFounderCounter`, an unlock rider
  under `GehennaOfOthersInPlay`, now a host predicate too;
  `R::SourceFounderCountersAtLeast`) and `VampireDef::superior_while`, read in
  `granted_level` (Gwen Brand); `Effect::DiscardRandomReplaceFromAsh` and
  `Interrupt::ReplaceFromAsh` (Lady Constancia: the random discard's
  replacement picked among the ash heap's library cards but that copy, a
  draw when none is left; no `WouldReplace` rider is asked, as Development's
  research replacement). A granted action's printed blood (`PayBlood`) must
  be payable as it is announced, with or without a toll (rules-qa 42.1).
- Round 30 (Lords of the Night): `Stat::DamagesSuccessfulDirectedActors`
  (Shemti) adds its normal damage to `ActionCtx::damage_after` as the
  after-resolution window opens, on a successful action directed at its
  controller (`ActionCtx::directed_seats`); `Stat::FailedBlockerBurns` (Ogwon)
  burns each failed attempt's minion in `succeed`, twice for two
  (`failed_blockers_burn`, rules-qa 121.108, beside Faceless Night's lock);
  `Stat::ControllerBurnsPoolWhenBurned` (Sundervere) read in `burn_minion_now`
  beside The Capuchin's, paid through `burn_separately`;
  `Stat::DisciplineCardsAddNoCapacity` (Seterpenre) drops the Discipline cards
  from `capacity_parts`. Entry riders: `Effect::DisciplineMastersOnEntry(n)`
  asks `Interrupt::DisciplineSearch` with `left: n` (one pick a step, the
  library shuffled after the last) and `entering`, its contests staged once it
  ends (a contested vampire is not present: LSJ 20080526);
  `Effect::SearchLibraryOnEntry` (Porphyrion) asks `Interrupt::LibraryToHand`.
  `Hook::AllyOrRetainerEntersPlay(label)` is fired on every vampire in play by
  `Engine::enter_play` (Paolo Sardenzo's `Effect::MayAddHostBlood`:
  `Interrupt::MayGainBlood`). `Terms::RemoveFromAnyAsh` (Luna Giovanni): a heap
  (`Flow::AbilityTerms::heap`), then a card in it, removed as picked, X their
  number, masked with none (`Engine::ash_heaps_with`). A vampire ability's
  `Terms::XBlood` (Bela Kardoza) is announced with the use, 0 up to its
  host's blood, its `Effect::ActorBurnsBloodX` burned from the host in
  `resolve_ability_now`.
  `Requirement::ActorBurnedTitled` (Monty Coven) reads the titles burned
  (`CombatCtx::burned_titles`; `ActionCtx::burned_owners` and
  `Engine::card_burns` carry the victim's title), his `Effect::GainTitle`
  through `gain_title`, one title a moment (`Limit::PerActionCard`).
  `Requirement::OpponentCorruptedBySource` (Nakhthorheb) reads the would-be
  blocker's counters of the printed vampire's Methuselah.
- Round 31's Sabbat vampires: Hukros's stash is `Minion::stored` (a
  `SparsePile`), filled by `Effect::StorePickedOnHost` from `Terms::Discard`
  (replaced, rules-qa 37.1), played through the virtual-hand source
  `state::minion_store(m)` (a `virtual_source`: no card in play, not
  replaced; `play_options` lets only that minion play it), hidden from
  other seats in `legal_view`, burned with him to the ash heap
  (`burn_minion`), carried by a control change as `foreign` cards, a
  `copies::Place::MinionStored` in the census (the host's `Zone::Stored`
  on the minion, `MinionView::stored`). Lucubratio's
  `Stat::MayDiscardInsteadOfBlockLock` beside Lucas Halton's: its printed
  `Requirement::HandHas` (read by `printed_holds` on his controller's
  hand) is the filter `Interrupt::MayDiscardForBlock` offers, once each
  turn (`once_block_discard_key`). Lord Aaron's `Stat::FrenzyPlayedOnToll`
  (`Engine::frenzy_toll`, over `frenzy_on_all`) is added in `play_cost`,
  `resolve_play`'s pool and `pay_play_blood`, and `action_card_cost`: in blood
  by default, in pool when its player picks `Choice::BurnPool` in the play terms
  (`PlayCtx::toll_in_pool`, offered only when both are payable; asked after a
  frenzy card's X-minion picks name him, Bloodlust) or when no minion's blood
  can pay it. `Stat::FirstRoundManeuver` (Leo Washington's opponent) banks
  `Side::round_maneuver` as the combat starts, offered and spent while the
  live read still grants it (`combat::round_maneuver`, Hanna Redmonds'
  shape); `Effect::AggravatedToNormal` (Konrad) moves pending aggravated points
  to normal, the strike's share with them; outside his combat
  `damage_out_of_combat` reads the same printed effect, a once-each-combat
  one aside (Merrill Molitor), and asks first
  (`Interrupt::MayTakeAggravatedAsNormal`, the damage dealt with the answer,
  under the cause it was dealt by); `Stat::UnlockAfterSuccessfulBleed`
  (Luke Fellows) sets `ActionCtx::unlock_at_end` with the bleed's success;
  `RefEffect::StealSourceVampire` (Kai Simmons) gives the source vampire to
  the caller for good; `Requirement::ActingNamed` (Humo, Greensleeves) and
  the printed `PlayedNotAsCombat` (Jane Sims) read the acting minion's name
  and the type a card is played as.
- Round 32's Laibon: Kamiri wa Itherero's `Stat::AllyBlockerBurnsAtActionEnd` is
  read as his block's combat begins (`Engine::start_combat`, the ally kept in
  `ActionCtx::doomed_blockers`, Kamiri gone or not: D33) and the ally burned by
  `Engine::end_action` (with `burned_owners` truncated, no "acting minion
  burned" reader seeing it) and Maskini's `Stat::BurnsAfterUnsuccessfulAttempt`
  (his action blocked or failed, or his `failed_attempts` none of which later
  blocked: one burn an action). Mr. Noir's
  `Stat::ControllerGainsPoolIfBurnedInCombat` beside Hector's in
  `burned_in_combat_gift`. Muricia's `Effect::TargetBecomesClan`,
  `TargetBecomesSterile` (`Minion::made_sterile`, read by `is_sterile`) and
  `TargetGainsLevel` (`gained_levels`). Jack Dawson:
  `Side::used_weapons` (weapons used this round: struck with, dodged too,
  pushed beside AK-47's, or any ability of theirs used by the combatant
  bearer, `resolve_ability_now`: Bundi's prevention, rules-qa 124.122),
  `Requirement::UsedWeaponThisRound` and
  `Effect::AdditionalStrikeWithUsedWeapon` (limited, `additional_with` the
  newest still borne). Olugbenga's `Stat::ContestPaidWithBlood`: a
  vampire's contest paid goes through `Interrupt::ContestPayment`
  (`BurnPool`, or `BurnBloodFrom` the contested vampire or an uncontested
  Olugbenga of its controller's, `Engine::contest_blood_sources`).
- Round 32's Laibon vampires: every loss of the Edge goes through
  `Engine::lose_edge` (taken by `gain_edge`, burned, ousted with its holder
  in `remove_seat`), firing `Hook::MethuselahLosesEdge` printed riders on
  every vampire in play (Sennadurek: `RiderScope::Others` against the active
  Methuselah, `Effect::PeekEdgeLoserHand`). The Edge burned for a vote
  (`Choice::CastEdge`) feeds `Stat::BloodWhenEdgeBurnedForVote` (Melissa
  Barton) beside Tyler McGill's vote. Zhenga's `Effect::HiresOnSuccessX`
  sets `ActionCtx::extra_hires`; the action successful, `after_resolution`
  asks `Interrupt::ExtraHires` (`employable_from_hand`, one pick a step
  through `Engine::hire_from_hand`, the entry effects it queues before the
  next pick, the hand replaced once it ends). `Terms::RemoveFromPreyAsh`
  (Prejudice: `Flow::AbilityTerms::heap` set to the user's prey as used, one
  pick, Luna Giovanni's removal). `Stat::LeaveTorporDiscount` (Zhara,
  `GameState::leave_torpor_cost`, read by the offer and the resolution);
  `Requirement::ActionCardRequiresDiscipline` (Shasa Abu Badr,
  `PrintedCtx::action_card`); `PlayedRequiresNone` read on printed text too
  (Rosemarie); `CombatCtx::end_press_barred` passes the opposing combatant
  (Sayshila's `NoPressToContinue` on her opponent); `Requirement::ActingReady`
  (Wamukota's continue, rules-qa 93.5).
- Round 32's Laibon vampires (unlocks, levels, titles):
  `Stat::UnlockAfterNonHuntSuccess` beside Cock Robin's at `succeed` (one
  `once_unlock_key`, spent even when the action unlocked him: Eze);
  `Per::OwnReadyTitled` (Ayo Igoli); `Effect::HostGainsLevel`
  (`Minion::gained_levels`: Aimee Laroux);
  `Effect::MayUnlock { once_each_turn }` (Baba Yaga's, and Al-Muntaquim's
  printed `GehennaPlayed` rider); `CannotPlay`'s context reads `reacting`
  (D18); a granted action's `LockTargetsSkipUnlock` through `Granted::Play`
  (Falhu Shibaba); printed `OwnBleedSuccessful` riders read `ReadyNamed`
  (`holds_for_host`) and `BleedAgainstPrey` on the bled seat, and gain pool
  (Ganhuru); `Requirement::TargetStrikeRequiresDiscipline`
  (`CombatCtx::strike_tags`) and `Effect::TargetStrikeDamagePlus`
  (`Side::raise_strike`), one use with the action branch by
  `Limit::PerTurnCard` (Gualtiero Ghiberti); a retainer become a
  `MinionKind::Represented` ally (`Effect::TargetRetainerRepresents`: off its
  host, its card out of play with its layers, `raise_represented`'s `life`
  the version's `cards::retainer_starting_life` plus `CardInPlay::extra_life`
  (Tainted Spring [ANI], Muricia's Call [ani][spi]: a starting life), its
  counters its life, `Lasting::StrengthWhileLifeAtLeast` read live by
  `GameState::lasting`: Demdemeh).
- Round 33's vampires, Adrianne to Bianca (track B): `cannot_block` reads
  the action's own directedness, so a printed `CannotBlock` may require
  `ActionUndirected` (Allonzo Montoya, Benedict Giovanni, Phillipe
  Rigaud); a printed `ActionRequiresDiscipline` reads
  `PrintedCtx::action_card` and `action_ability`
  (`actions::action_requires_discipline`: Allonzo); a printed cost
  reduction for an action card putting a vampire in play
  (`PlayedPutsVampireInPlay` over `cards::puts_vampire_in_play`: Azaneal);
  a printed rider on a master Discipline card its controller plays on it
  in their master phase (`Hook::MasterDisciplineOnHostByMpa`, fired in the
  `PutOnTarget` arm: Basir); the casts another vampire draws to its side
  at the tally (`Stat::VotesCastWithIt`,
  `ReferendumCtx::cast_with_leaders` rewriting the records before the
  tally: Astrid Thomas); a stealer older than its host's controller's
  ready vampires of a clan (`ActorOlderThanHostControllersReadyClan`:
  Benedict); a mandatory action of a type, not an ability
  (`Stat::MustDiablerizeOlder`: `Engine::diableries_due` in
  `mandatory_split` and beside a forced hunt, `dues` naming its minion:
  Phillipe Rigaud).
- Round 33's vampires, Black Cat to Democritus (track B): life an ally
  or retainer gets as it enters play, any Methuselah's vampire printing it
  (`Stat::EntrantLife`, the printed `EntrantLabeled` reading
  `PrintedCtx::entrant`, the entrant's labels or its represented subtype;
  `GameState::entrant_life_bonus` in `recruit_ally`, `put_retainer` (a
  retainer's in `CardInPlay::extra_life`, its starting life, with
  Ashur-uballit's zombie bonus, so `RetainerToStartingLife` reads no live
  bonus) and `raise_represented`: Carmine Giovanni); a card contest's pool cost
  raised by a rival's vampire (`Stat::CardContestToll`,
  `GameState::card_contest_cost`, never a title's; Pay withheld when the pool,
  Olugbenga's blood counted, falls short; `Interrupt::ContestPayment::left`
  re-asked unit by unit: Democritus); the action's blocker as a minion target
  (`TargetIsBlocker`: Cagliostro). `TargetOnActor` passes no effect-only
  stand-in (Callirus).
- Round 33's vampires, Roxanne to Tatiana (track B): an entry rider gated
  on its controller's influence phase (`Requirement::InOwnInfluencePhase`,
  read by `influence::entry_effects_in` against `GameState::in_influence_phase`:
  Hermana Hambrienta's burn, Sonja Blue's 4 pool, any way to the ready region in
  it, RTR 20080809); the who-can-block bars reading a blocker's status read
  twice for a vampire that "may block as an ally" (`Stat::BlocksAsAlly`,
  `Engine::status_barred`); a vampire removed from the game as it leaves play
  (`Stat::RemovedOnLeavingPlay`, `Engine::removed_on_leaving`: burned to
  `removed_crypt` after its burn's triggers, removed as a contest starts but in
  two players, as banished, as put out of play); control taken for good into the
  taker's ready region (`ControlTerm::ForGoodToReady`,
  `Effect::StealHostToReady`, read by its controller's predator in its prey's
  discard phase, `HostControllersPreyActive`: Sonja Blue); every Methuselah's
  pool lost together as a vampire goes to torpor (`EachMethuselahBurnsPool`:
  Sylvester Simms).
- Round 33's vampires (track C, vamp_h): a merged vampire's sect is its
  [MERGED] text's (`CryptCard::merged_sect`, `Minion::underlying_sect`:
  Jeremy MacNeil, Count Germaine, Valerius Maior), and its merged
  definition may drop printed traits (`VampireDef::merged_loses`, read by
  `is_infernal`, `unlocks_as_infernal`, `is_red_list`: Valerius Maior). A
  negative printed `Stat::Capacity` lowers capacity as a card's would (the
  minimum of 1), and `drain_all_excess` reads every vampire printing one
  (Tariq, merged Valerius). `Stat::StrikeGrantsLikeStrike` (Valerius
  Maior): after a strike card its requirements take (`PrintedCtx::card` =
  `Side::strike_play`), a limited optional additional strike beside
  AK-47's, `Side::additional_like` leaving only such plays and a pass in
  the additional pair. `Effect::ActingNoCombatCardsRequiring` fills
  `ActionCtx::combat_bars` beside Arnulf's modifier bars (Valerius Maior
  advanced). `Stat::DefectsToRicherPrey` (Uriah Winter, with
  `Requirement::ControllersPreyHasMorePool`) is read at the top of
  `start_unlock_effects`: `take_control` for good to the prey. Printed
  `Requirement::HostInCombat` (merged Dancin' Dana's hand size) and
  `TargetSeatControlsTitledOutside` (Count Germaine: any titled vampire,
  torpor included, of none of the sects); `Effect::LookAtPreyLibraryTop`
  (Tiberius).

### 6.13 Equipment

`plays.rs`, `actions.rs`.

- Equip from hand: +1 stealth plus printed, cost at resolution (unpayable or no
  longer holdable: fizzle), NRA by name. From another own minion
  (`EquipFrom`): free, no requirement check, each taken card's equip stealth
  added, moves what is still there at resolution. One vehicle etc. via
  `GameState::can_hold`, which bars a no-equipment minion only from what is
  still equipment in play: a locquipment is a location (rules-qa 94.4), and
  offers it no strike card stealing equipment (Lorrie Dunsirn, LSJ
  20050315); a card's "Cannot be put on a vampire with superior X"
  (`TargetNotSuperior`) bars every move or put onto such a vampire too
  (Dauntain; rules-qa 81.1). `Engine::put_equipment` is shared by equip,
  searches and Vast Wealth; "only one played or equipped in a game" is recorded
  there, and where Summon History puts an equipment in play
  (`Engine::note_once_played`).
- Found or stored equipment (Vast Wealth, Repo Man, Delivery Truck's
  `EquipStored`) is checked by `equip_requirements_met`: clan, path,
  Disciplines, "only one" and its equip mode's sect and title
  requirements. Nod (`RearrangeEquipment`): one `MinionPurpose::Rearrange`
  per equipment, each Methuselah's in turn; "only one" read against the
  cards already placed (`rearrange_fits`), so vehicles may swap.
- "An equip action" (Hag's Wrinkles, NRA PAC, Roderick Phillips March):
  `ActionCtx::is_equip_action`, an equip from hand, the take from another
  minion (`EquipFrom`) or a search equipping (`SearchEquip`) (rules-qa
  78.28, digest §3.7).
- "An employ action" (Rosa Hernández, Frau Schädel):
  `ActionCtx::is_employ_action` (`PrintedCtx::employ_action`), a retainer's
  own or an action card's (Jack of Both Sides, TOM 19960130), not one
  ignoring requirements (Biothaumaturgic Experiment, ANK 20180817).
- Bearer effects are read live on the current bearer (`Modifier::Bearer`), with
  the named trait (a non-Ravnos holding Treasured Samadji gets nothing,
  rules-qa 9.1).
- An action an equipment grants its bearer (`Actor::Host`) fizzles once the
  equipment left the acting minion (RTR 19960221, `ability_void`). Another
  minion's action on it (Jar of Skin Eaters: `ActorSlaveOfHostClan`, its
  controller's slave of the bearer's clan, rules-qa 84.9) moves its blood
  (`ActorBloodToSourceCard`, as much as it has); its out-of-combat
  aggravated damage is `AggravatedDamageTarget`. A combatant
  barred from equipment (`GameState::equipment_barred`, set with the
  combat's flags by `Side::bar_equipment`, cleared by
  `GameState::leave_combat`) loses the Disciplines its equipment gives (rules-qa 76.16).
  `Effect::NoEquipmentThisCombat` bars the user's own side (Vial of Garou
  Blood). `Modifier::BearerSuperiorIfHas` (Codex of the Damned) raises a
  level read without it (`granted_level`), lost under the bar.
- `GameState::pinned` (Eye of Hazimel, not moved or stolen while its
  bearer is ready) is read by `ActorMayHoldTarget`, equip-from,
  `holdable_equipment` (Heidelberg Castle), Redistribution's and Rave's
  moves (`Interrupt::CircleBlood`, which offers equipment only to a minion
  able to hold it; `MoveAmong::Circle` among a circle, `MoveAmong::From`
  from the acting minion to its Methuselah's other ready vampires, an
  ally's last life moved burning it) and Confiscation's take; its burn
  damages the bearer (`Effect::BurnedDamagesBearer` in `burn_card_in_play`).
  Its `who`, like `Modifier::DirectedAtBearerCostsPool`'s, names the bearer
  it reaches, read on the holder as it burns (San Nicolás's Lasombra).
- `Effect::EntersLocked` (Helicopter): the card locked by its equip mode,
  however equipped (`put_equipment`; a take from another minion in
  `ActionKind::EquipFrom`); a card put on (Alastor's `SearchEquipFor`) runs
  no mode effect; with `linked` (Bauble's `Effect::SearchEquipOntoTarget`)
  it is of the filters, at no cost, and becomes the linked card's
  `chosen_card`.
- Equipped from hand, however (LSJ 20100421), a card with
  `Effect::MaySecondCopyFromHandOrAsh` offers another copy from the hand
  or ash heap (`Interrupt::SecondCopyOnto` from `equip_from_hand`): put on
  through `put_equipment`, no cost, no fetch of its own, a hand card
  replaced (Dagger). A steal of a card on a minion (`ActionKind::StealCard`
  on a hosted card) moves it to the acting minion if it can hold it, else
  fizzles (Shaal Fragment, Skaald). "This vampire can burn n blood to play a
  card requiring basic X" (`Modifier::BearerPaysToPlayAsBasic`): a vampire
  bearer lacking X meets the tag (`cards::has_disciplines`; not the Discipline
  itself, `GameState::grant_payers`), the least blood needed beyond the cost
  in `play_options` (`cards::paid_discipline_blood`) and burned as the card is
  announced, the card's `spent_turn` set (Amulet of Temporal Perception).
  `Modifier::BearerPlaysMissingAsBasic` (Infernal Familiar): any basic
  Discipline the vampire lacks, a counter on it as the card is announced; with
  two payers unlike (`Engine::grant_payers`), the player picks one in the play
  terms (`PlayCtx::paying`, `Choice::ChooseCard`; a lending vampire's
  `Choice::ChooseMinion`), `announce_play` going on in `announce_paid`; a
  vampire's printed lender (`VampireDef::lends_basic`, Phagian: its
  controller's vampires, `GrantPayer::Printed`) is a payer whose blood is the
  card's cost increased (`PlayCtx::grant_surcharge`, paid with the cost); a paid
  grant meets one requirement of a combo card at most (`disciplines_met`'s
  `grant`); `HostBurnedOverCounters` burns the host in `settle_bound_burns`,
  live.
- Stored equipment by index (`EquipStored(i)`, `ActorMayEquipStored(i)`:
  Delivery Truck's 0, Aaron Fletcher's three, a copy of an earlier slot not
  offered again), stored as an ally enters play (`Interrupt::StoreUpTo`); never
  moved (`Effect::PinnedAlways`), its burn sending the bearer to torpor
  (`BurnedSendsBearerToTorpor`), a clan's Discipline level
  (`Modifier::BearerClanDisciplineLevel`: the Vial).
- A unique card is never taken by choice into a contest with oneself
  (rulebook "Contested Cards"; advanced rules §1.13.1): a mode or ability
  whose effect `Effect::takes_card` (a steal, Clockwerx) offers no target
  card its player already controls a copy of
  (`GameState::contests_itself`), and takes nothing if a copy arrived
  meanwhile. A forced move (`Engine::move_card_onto`: a weapon's transfer,
  Arcane Appraiser's `TargetCardToChosen` by its own text) burns the
  incoming copy instead.
  Arcane Appraiser's receiving minion is picked right after the action is
  declared (`MinionPurpose::HolderAtSuccess`, `ActionCtx::holder`, D1);
  `cards::Holder` says who may receive it (Dauntain Black Magician: a
  vampire, torpid too).
- A locquipment's Discipline is no equipment's: kept under an equipment
  bar (`granted_level`, Winchester Mansion). An equip from hand outside an
  action (Pier 13's ability, Angel of Berlin) is recorded for the phase
  (`GameState::equipped_off_action`, `Requirement::EquippedOffActionLabeled`:
  Synner-G, Vulture; PIB 20150105-2); `EquipNamed` reads its terms from the
  card's ability when its mode has none.
- Shuns (`GameState::shuns_card`): Draeven's Disciplines, Martinelli's
  Ring's Disciplines and clans (`Modifier::BearerShunsRequiring`), as a
  card's target and a card in play's ability's target (LSJ 20080604); the
  Ring's shunned cards on its bearer burn (`settle_bound_burns`), its
  bearer's own put on himself not offered (LSJ 20080212).
- Reliquary: Akunanse Remains: its unlock rider
  `Effect::ChooseCardDiscipline` asks `Interrupt::PlacedDiscipline {
  among }`, the level by `Modifier::BearerClanChosenDisciplineLevel`;
  `Modifier::BearerCapacity` in `capacity_parts`. Reliquary: Biague's
  `Effect::LookTopRevealIfNoDisciplineMayPeek` then
  `Interrupt::MayBurnBloodToPeek`. Kduva's Mask:
  `Modifier::BearerActingReactionSurcharge`, in `discipline_surcharge`
  (`acting_reaction_surcharge`). `GameState::holds_a` and its kin skip a
  card out of play (a face-down Kerrie or Dagger).
- `Modifier::BearerInterceptOnModifier` (Phased Motion Detector): +1
  intercept as a vampire of the acting side plays an action modifier of
  the Disciplines, at `open_as_played` (before any cancel), once each action
  per copy (`ActionCtx::modifier_intercepts`).
- `Effect::SearchStore { n, filter, ash }`: Black Market Cache's search
  (`resolve_play`, once replaced) and Mokolé Blood's (`put_equipment`, any
  equip, before its contest); `Interrupt::StoreSearch` offers the library's
  matches (`FindCard`) and, with `ash`, the ash heap's (`TakeFrom`);
  Alastor's put runs no search and burns such a card holding nothing.
  `Effect::BearerStore(Bearer)`: only a matching bearer, read live, plays the
  cards on it (Mokolé Blood's Ministry; Gift of Proteus, Blessing of the Beast
  any). `Modifier::HostDisciplineBloodOff`'s `who` names the bearer it serves
  (Karavalanisha Vrana's Ravnos).
- `Effect::EquipOntoTarget` (Bloodstone): the play's target minion bears
  it (`put_equipment`; a search's target is the actor, LSJ 20090824),
  `CardInPlay::by` its equipper, the equip directed at another's
  (`directed_seat`, the stealth table). `ChooseTarget` on an equipment
  (Incriminating Videotape): the play's choice, or `ChooseForCard { of:
  Some(prey) }` when another card equips it; a put chooses none
  (`Modifier::ChosenCannotBlockBearer`, read live on the bearer).
  `EquipActionUnlocksAtDiscard` (Powder of Rigidity) beside Baseball Bat's
  marker. `HostBurnsBloodOrBurnSelf { to_card }` moves the blood onto the
  card (Polaris Coach), an ally's toll burning it. `DisciplineSearch {
  no_hand }`: Deviki Prasanta's library or ash heap
  (`Effect::SearchDisciplineMasterOntoTarget`).

### 6.14 Replacement, hand size, draws, searches

`plays.rs`, `draws.rs`.

- Hand size is derived (`GameState::hand_size`: seven, `Seat::timed`'s
  `HandSize`, its vampires' printed hand size and its predator's on it, any
  vampire's for each Methuselah (Servius Marius Pustula), the hand-size layers
  of the uncontested cards in play the seat controls); cards not yet replaced
  count against it (`Seat.held`, §2 Held and Release; the longest delay wins,
  spec §1.9.2). Held-until conditions include events' ("until a Methuselah is
  ousted", "as long as this card is in play": waiting on the condition, not the
  card). "+n hand size until …" is a timed modifier (`Seat::timed`, swept where
  it ends). An oversize hand discards down before any other decision.
- Every card leaving hand that the rules replace goes through `Engine::replace`,
  offered first to `Hook::WouldReplace` riders, except the card being replaced
  itself ("a card other than this card"), then the hand drawn up; an empty
  library replaces nothing (and nothing burns instead, rules-qa 8.3). Visit
  from the Capuchin burns a counter instead of drawing, the hand size
  shrinking with it. A hand-size increase is no replacement: it draws by
  `settle_hand_rises`. The Meddling of Semsith holds replacements made
  during a minion phase until it ends (`Release::MinionPhaseEnd`), or until
  it leaves play (`release_unmeddled`, rules-qa 88.8). A card resolved in
  another's as-played window (a wake, a cancel; `Engine::in_as_played`) is
  replaced as the stack's last window closes, before its bottom card
  resolves (`Release::AsPlayedWindow`, `as_played_closed`, rules-qa
  110.1); the extra draws offered meanwhile wait with it
  (`Engine::window_draws`), offered after. A vampire's printed delay
  (Nedal: `Stat::CombatCardsHeldUntilUnlock`) holds its combat cards' as a
  card's own "do not replace until" would, the longest delay first.
- Draw alternatives (`Hook::WouldDraw`): a choice per card drawn (rules-qa 9.4);
  beside Visit from the Capuchin a replacement is the Capuchin's counter or a
  stored card.
- "Draw up to n" (La Viuda Blanca, `Effect::DrawUpToThenDiscardDown`):
  `Interrupt::DrawUpTo` asks the number as the action resolves
  (`Choice::SetX`, capped by the library; rules-qa 11.1), drawn as Kaymakli
  Fragment's, the hand discarded down as it settles.
- Moise Kasavubu's search equips the action's target (`SearchEquipForTarget`)
  and offers the hand too (`Flow::LibrarySearch::hand`: `EmployFromHand`,
  the library unshuffled). Unity's picks: `Interrupt::AshToLibraryTop`, then
  `CutLibrary` (`Choice::SetX`: the top n to the bottom).
- Searches (`Flow::LibrarySearch`, `Interrupt::DisciplineSearch`, `SearchStore`,
  `SearchToHand`) pick by card, not position, at resolution (spec §1.14.1), and
  shuffle with the engine's `Rng`; an equipment search offers only what could
  be equipped as a play (no unpayable blood cost, rules-qa 9.3). A search is
  private unless the card says to reveal (PIB 20121028): `SearchToHand`'s
  `shown`, and the find of Fortschritt Library, The Mithraeum, Disciple of
  Lazarus; a haven found by Goblinism [MYT] (`LibraryToHand::haven_on`) may
  go on the vampire instead (`Interrupt::PutFoundOn`, its cost paid,
  `place_now`), asked before the hand is discarded down (`discards_first`);
  a revealed card (a search's find, a top card) goes to `Note::Reveal`
  (`Reveal`), which the host tells every seat, and to `Engine::reveals`
  (the last `REVEALS`, notes on or off), which observation `V4` encodes.
  A look at another seat's hand goes through `Engine::peek`: the hand
  visible to the viewer until the action ends (`GameState::peeks`), or
  the turn (`Engine::peek_this_turn`, `GameState::turn_peeks`, cleared in
  `next_turn`: Pulse of the Canaille), and the hand as it stood to `Note::Look`
  (`Look`), which the host tells the viewer alone (the next view no longer shows
  it).
- Every library draw goes through `Engine::offer_redraw` (Blackhorse
  Tanner's `Interrupt::Redraw`, once each turn); `Interrupt::AllySearch`
  (Antonio d'Erlette) searches library and hand for a mage ally and puts
  it in play, not recruited. A crypt draw a card gives (Enthrall [OBF]) is
  offered after its resolution as `Choice::DrawCrypt`, none from an empty
  crypt.
- Extra draws: `Engine::extra_draws` lists one card's, each source its own
  (`draws::DrawSource`, `Interrupt::MayDrawExtra` naming it): Ira Rivers's on
  the play (`Stat::ExtraDrawOnPlay`, under Capuchin or a delay too), Learjet's
  (`Modifier::ExtraDrawOnReplace`) and Duality's (`Stat::ExtraDrawOnReplace`)
  with a replacement that draws: none if Visit from the Capuchin takes it
  (`replacements_taken`; deferred, `Interrupt::Replacement` drops the offer),
  delayed in `Seat::held_draws`, offered by `Engine::replace_held` at every
  release, carried with a replacement the Meddling holds again; rules-qa
  73.1; the action card brings it, its action begun as it is replaced, a
  canceled one none, `extra_draws`' `begun`, 73.2; a Steely Tenacity taken
  back none, `Interrupt::RetakeFromAsh`'s `jet`. Infernal Pursuit's
  (`Seat::timed`'s `TimedStat::ExtraDraws`, ending with the combat; read in
  `Engine::replace`; several cards replaced at once go one at a time,
  `Interrupt::PursuitDraw` then `PursuitDiscard`, rules-qa 63.11).
- Summon History (`Effect::SearchPutInPlayUpToX`, `Interrupt::PutInPlaySearch`,
  `put_in_play_finds`; the host shows the crypt as the `AshVampire` zone): the
  card put in play, a vampire through `influence::vampire_entered` (shared
  with influence-out), a Flaming Candle found whatever was played (LSJ
  20060209) and counted as the game's one. `Effect::HandToBottomX`
  (Nehemiah) moves the picked count to the library's bottom
  (`Interrupt::Bottom`).

- "You can get +n hand size until the end of the turn" (Sascha Vykos:
  `Interrupt::MayGainHandSize`, once each turn); Port Authority's delayed
  replacement (`Modifier::DiscardActionReplacesAtUnlock`: the discard phase
  action's card held to the Methuselah's next unlock phase, drawn before
  anything unlocks).
- A held card replaced "instead from the ash heap" (Steely Tenacity,
  `CardDef::retake_from_ash`; Roll's only while the minion that played it
  is ready, `Seat::retake_by`): `Seat::retakes` records it with its
  `Release` and copy; `replace_held` then defers the whole draw to
  `Interrupt::RetakeFromAsh` (`FindCard` while that copy is in the ash
  heap, or `DrawLibrary`), each due card in turn (`next_retake`), the
  draws chosen made at once after the last; re-keyed to `MinionPhaseEnd`
  when the Meddling holds the draw. A canceled card never reaches the hold.
- A look, then a choice on what was seen (Weirding Stone): the ability's
  `Effect::LookAtTurnLibraryTop` then `Effect::MayBurnLookedMaster`, which
  asks `Interrupt::MayBurnLookedMaster` only on a master — no option shows
  the card before the look. Courier: `LookAtBledLibraryTop`, then
  `Interrupt::MayLockToDiscardLooked` while the Courier is unlocked.
- A mode's own blood off (`Effect::CostsLessBlood`, Hay Ride [SAN]):
  `mode_blood_off`, before the other reductions, in `play_cost` and as
  paid.
- Cards played from another Methuselah's ash heap (Vestgeir,
  `Stat::PlaysPoliticalFromOthersAsh`): `state::ash_of(seat)` sources in the
  virtual hand (`others_ash`), the card its owner's and removed instead of
  burned (`played_card_owner`, `played_from_removing_store`).
- Gift of Proteus: `Effect::BearerStore` (its bearer's plays only, burned
  once empty) and `Effect::StoreFromHandX` with `Terms::HandCards`, among
  the cards earmarked as the action is declared, before its replacement
  (`Seat::earmarked`; `Interrupt::StoreOwnHand`'s `earmarked`); a master
  (Blessing of the Beast) earmarks the hand as it resolves.
- Shadow Court Satyr (`Effect::UsesStoredCard`): his stored card in the
  virtual hand while he is in combat, unspent (`cards::stored_use_source`),
  by him only; his grant (`stored_use_grant`, beside his own in
  `plays_card_as_vampire`) the mode's one basic Discipline, or none, and the
  cards it brings in (Pack Alpha's Raven Spy), never a hand card's play.
  No card is played (advanced rules §1.8): `announce_play` records nothing,
  the as-played window offers only the card's own cancel (`used_store`),
  and the card goes back on him (`back_to_store`) unless it entered play;
  `GameState::stored_use` keeps the grant while it resolves.
- The research area (`Seat::research`, a pile `copies.rs` lists): a card
  replaced from it instead of drawing (Development,
  `CardDef::replace_from_research`: its own replacement asks
  `Interrupt::ReplaceFromResearch`, `FindCard` or `DrawLibrary`, while the
  area holds a card; a held one keeps the option to its release,
  `Seat::research_holds` beside `held`, rules-qa 73.1). Research fills it
  (`Effect::SearchLibraryToResearch`, `Interrupt::ResearchSearch`: a card
  at a time, a pass stopping, the library shuffled after); the legal view
  hides it from the others (§6.21).
- Minor Curse (`Modifier::HostPlaysHeldUntil`): a card its host plays is
  held to the end of the action under way or the one it declares ([mal];
  none outside one) or to that turn's discard phase ([MAL],
  `Release::TurnDiscardPhase`, every seat's, released in `end_influence`):
  `cursed_hold` at both replacement sites of `resolve_play`.
- A library search to the top (Scarlet Lore [myt]):
  `Interrupt::LibraryToHand` with `top`, the library shuffled first.
- "Do not replace until after the current turn" (Sonar): `Until::EndOfTurn`
  → `Release::EndOfTurn`, every seat's, released at the top of `next_turn`
  (a turn cut short included).
- A vampire holding replacements while unlocked (Hagar Stone,
  `Release::WhileUnlocked`): read live, as a vampire locks in dozens of ways
  (`settle_unlocked_holds` in `run`), the turn's end releasing them; a shorter
  delay carries into it in `release_held` (the longest wins).
- A reorder then a move to the library's bottom chosen after it
  (`Interrupt::MayBottomTop`: St. Patrick's Cathedral); ash heap cards to the
  bottom, up to n (`AshToLibraryTop`'s `bottom`, `up_to`: Academia); each
  Methuselah's crypt-top pick (`Interrupt::CryptTopPick`: Porto de Santos);
  transfers this turn from the minion phase (`TransfersThisTurn` into
  `Seat::transfers_next`: Julia Sowinski).
- Scrounging's `Interrupt::LookedToHand` picks from the top cards before the
  hand is discarded down (`discards_first` waits for it); Victim of Habit's
  name is a play term (`Terms::PreyAshCard`, `PlayCtx::ash`) kept on the
  card (`CardInPlay::named`).
- `Effect::SearchLibraryToHand { shown }` (Shared): the card shown only
  when printed (Al's Army Apparatus, PIB 20121028).
  `Terms::DiscardSameName(n)`: the first pick names the card, masked
  without n copies of one card (`discard_terms_met`; Specialization).
- "Do not replace until your influence phase" (Lupine Assault):
  `Until::InfluencePhase` → `Release::InfluencePhase`, released for the
  active seat as its influence phase begins.
  `Effect::EachMethuselahDiscardsThenDrawsUp(n)`: an
  `Interrupt::DiscardChosen` per living Methuselah, clockwise from the
  player, each replaced once theirs are made.
- `Interrupt::SearchOnto` also searches hand and library (`from_hand`),
  `left` cards or pairs of `held`'s two kinds, the second of a pair `owed`
  without a Pass (Static Virtue's `Effect::FetchHeldOntoTarget`: pairs
  when its Laibon entered in the last influence phase, one for every two
  capacity above 4, read as it resolves); a hand card replaced.
- `Effect::DrawThenHandToTop(n)` (Hourglass of the Mind [tem]): draws what
  the library holds, then `Interrupt::Bottom { top: true }` moves the
  cards one at a time to the library's top, the last moved on top (the
  host's `about.detail` `top`).
- Pandora's Whisper: `Effect::EachOtherMayDraw(n)` pushes `MayDrawExtra`
  with `DrawSource::Card { card, n }` (all or none) from the prey on;
  `DrawAfterOthers { n, may_top }` queues the player's draw behind them
  (`Interrupt::DrawRise`), `may_top` setting `Seat::discards_may_top` once
  drawn, the discards down recorded in `Seat::top_discards`, then offered
  by `Interrupt::AshToLibraryTop { discarded: true }`. Equipping from hand
  (`equippable_from_hand`) offers each version the vampire's Disciplines
  meet (Tinglestripe; LSJ 20080803); a card a search or a storage card puts
  in play (`equip_found_with_terms`, the ally and retainer searches,
  Patrizia's named retainer) goes through `Engine::found_version`: the
  versions met (`equip_versions`, `recruit_versions`, `employable_from`, a
  bit a mode) asked as `Choice::Version` (`Interrupt::FoundVersion`) when
  more than one, then its reductions (`found_cost_choices`) and its
  `CostThen` (rules-qa 125.17). Pack Alpha's, Zhenga's and Jack of Both
  Sides' cards from hand are offered by version already (`EmployFromHand`)
  and never reach it.
- A look at a library's top is not offered with that library empty
  (rules-qa 122.139): `Requirement::LibraryAtLeast(1)` for the player's own,
  `BledLibraryNotEmpty` for the bled one's, `LookedLibraryNotEmpty` with
  `LibraryOf` (the prey's, the active Methuselah's, any) for the others;
  `cards::slots`' `every_library_look_needs_a_card_there` checks each look.

### 6.15 Costs and reductions

`plays.rs`.

- An X the state counts (`cards::terms_x`): the mode's terms
  (`Terms::CopiesControlled`, …) or, beside terms that name a card,
  `CardDef::x_counts` (`XCount::CopiesInOwnAsh`: Sibyl's Tongue), read
  where the play is offered and again as an action card's cost is paid.
- An action's own reductions: Charisma's layer and Erichtho's printed
  `Stat::ActionRecruitCostOff`, both in `GameState::recruit_discount`, read
  only where the recruit action pays (not Piper's recruit, LSJ 20090115-1).
- Reductions are announced with the card (`Flow::CostTerms`,
  `PlayCtx::discounts`, `CostSource`; no pass while unaffordable without them),
  locked at declaration (not if canceled as played) or at resolution for
  non-action cards; counters spent only as a cost is paid (`spend_counters`).
  One classification (`discount_kind`: blood or pool, blood only, counters;
  locked, burned) and one reckoning (`Engine::reduced`: "blood or pool"
  first, then blood only, then each counter source for what it covers,
  rules-qa 36.2) serve the payment (`take_discounts`, `reduced_cost_with`),
  the cost terms' pass (`play_cost`) and the offer (`with_potential`). Found
  or named cards take their reductions once chosen (`CostThen`,
  `pay_named_discounts`, `payable_with_potential`). A seat-wide once-a-turn
  reducer (Baldesar: `Stat::SeatCostOffOncePerTurn`) is announced as
  `PlayCtx::reducer` (`seat_reducers`, reading a named card too) and spent
  where the cost is paid (`reducer_off`, `spend_reducer`): the play, an equip
  from hand by a card, a found equipment.
- Nergal's once-each-turn blood off (`Stat::BloodCostOffOncePerTurn`) is
  announced as `PlayCtx::once_off` (`once_off_available`) and taken where
  the cost is paid (`once_off_amount`, spent by `spend_once_cost_off` only
  once paid): the play, a canceled card's paid cost, recruit, employ,
  equip, other action cards, a bleed or political card (`card_blood_due`,
  Yawp Court's early payment included). A new payment site takes it too.
  Declined and the card unpayable without it at its action's resolution,
  `offer_reduction_at_resolution` (in `action_advance`, before `succeed`)
  asks `Interrupt::ReductionAtResolution` once (`ActionCtx::once_off_asked`;
  costs by `action_card_cost`, `action_card_payable`; rules-qa 96.3).
- Soul of the Earth: `CostSource::HireRequiringClan`, any recruit or
  employ by its controller of a card requiring the clan (Piper's too),
  burned as taken, its host present.
- Surcharges: `card_surcharge` (Discipline and event surcharges, a host's
  combat cards and presses: Greater Curse, Decompose [THN]; Suspension
  of Disbelief's `Effect::BleedChangeCardsCost`, a marker read on the
  current action's card for the minion cards reducing or redirecting it),
  `discipline_surcharge` (Libertas), `combat_card_surcharge` (Terror Frenzy,
  Djeneba), `torpid_surcharge`, Haven's +1 on masters targeting its minion;
  Daria's on the action (`action_toll`); a label's pool surcharge, every
  Methuselah's, `while_host_ready` only while its bearer is ready (Guarded
  Rubrics); Chanjelin Ward's blood on actions targeting its vampire
  (`GameState::targeted_surcharge`, paid at resolution beside François
  Loehr's, an ally's such action not offered: `may_direct_at`); Lock's
  blood or life on a monster's action at its minion
  (`targeted_monster_surcharge`) and on its minion's attempts
  (`Engine::burn_to_attempt`, in `declare_action` and `pay_block_toll`:
  what it has if short); Blood Cult Awareness Network's −1 stealth and
  blood or life on an action making a vampire (`ActionCtx::makes_vampires`,
  `GameState::vampire_making_taxed`, read live in `stealth` and
  `pay_surcharges`). An ally combatant burned by paying its last life for a card
  ends the combat (`pay_play_blood`). A minion's printed surcharge (Tom's,
  `CostOff::blood_extra`) is paid there too (`printed_extra`); Jenna
  Cross's is added last, on blood left after every reduction
  (`Stat::BloodCostMoreIfAny`, `Engine::blood_if_any` on each card-blood
  path). A master card's pool reduction from its Methuselah's vampire
  (`Stat::SeatCostOff`: Salvador Garcia) is in `cards::pool_cost`.
- The reductions announced with a card are taken as its cost is paid by
  one helper (`take_announced`): at resolution (`resolve_play`) and for a
  card canceled with its cost still paid (`settle_canceled`, without the
  lock-type locations: rules-qa 101.1) — a non-action card only: a canceled
  action card's cost is never paid, Meditative Grove's "(cost is still
  paid)" included (ANK 20260216, rules-qa 77.4).
- A pool cost paid out of turn (True Love's Face's and Golconda's cancels,
  KRCG News Radio's intercept) may be a last pool, paid by
  `pay_pool_for_ability` (rules-qa 101.3).
- One estimate of a card's cost as announced so far, surcharges in, printed
  and announced reductions out (`play_cost`): the hand play's offer, the
  pass while unaffordable (`affordable_with_discounts`) and the blood an X
  may burn (`Terms::XBlood`); at best, the reductions still on offer out too
  (`with_potential`): the offer, a pool card's and a minion's alike. A new
  estimate site reads them.
- Paying the last pool ousts at once, out of turn or from an ability, the
  turn passing only if it was the payer's (`pay_pool_for_ability`; a card an
  ability equips or employs from hand, White Lily's weapon, no play
  resolving it: `oust_if_out` after it); a play whose effects already ousted
  its player does not oust again. An action's cost paid at resolution and
  its effect are simultaneous (rules-qa 45.1, PIB 20130627): a last pool
  paid there (a pool surcharge, Lorrie Dunsirn's, an action card's or an
  action ability's own) sets `ActionCtx::oust_held`, released by
  `Engine::release_held_oust` once the effect is applied — at
  `after_resolution`, before a combat (`start_combat_at_resolution`) or a
  referendum the action leads to, and with the bleed's prey ousted in one
  batch (`acting_pool_out_with`: no 6 pool); a pool the effect gained
  saves the payer. A 3/ fizzle's last pool (`fizzle_short`) ousts at once. An
  action whose Methuselah is out of the game is over however the last pool went
  (an interrupt's cost: Alastor's equipment, read after the referendum
  concluded; a Life Boon declined): `Engine::settle_acting_ousted`, after each
  answer and in `run`, once the interrupts and any Boon are settled, a combat's
  oust left to `combat_ousts` (rules-qa 49.6, 51.5).
- `CostSource::FocusBlood`: a card on its host, burned as the host pays a
  card played as a combat card, 1 blood off (taken in the non-action
  payment beside The Shard). Concealed Weapon's "cannot cost 3 or more
  pool" reads the cost as modified in play, not the announced reductions
  or another payer (`equippable_from_hand`).
- The Ankara Citadel (`GameState::printed_cost_off_x`): the blood cost is
  halved (rounded down) before any other change ("multiplication and
  division first", RTR 20070707), the half taken off with the bearer's
  other reductions; a dual Action Modifier card is halved when played as a
  reaction or in combat, an X cost once X is chosen (rules-qa 70.17).
- A pool-only seat reduction paid with a random discard
  (`Stat::SeatPoolOffForRandomDiscard`: Kali, Baldesar Rossellini's slot); a
  surcharge on the Discipline cards of a vampire in combat with the bearer
  (`Modifier::OpposingVampireDisciplineCardsCost`: Father Anthony); a pool cost
  paid to a card's controller (`PayPoolToCardController`: Convento da Penha); an
  ability once each game (`Limit::PerGame`).
- A card "free or costing n blood or pool" (`CardFilter::CostAtMost`: printed
  pool and blood each at most n, X never; New Management, Bulscu), beside
  `PoolCostAtMost` (a blood cost as 0 pool).
- Powerbase: Tshwane's `CostSource::LockPool(n)`: "blood or pool"
  (`Off::Either`), offered only with a pool cost, so it takes pool alone;
  its in-action lock `ReduceActionCost`. A searched ally
  (`Interrupt::AllySearch`, Antonio d'Erlette) has its reductions asked
  first (`CostThen::AllyFound`, `Engine::ally_found`), as a found
  equipment's (`CostThen::EquipFound`), a searched retainer's
  (`CostThen::RetainerFound`, `retainer_found`) and each of Zhenga's hires
  (`CostThen::ExtraHire`, `extra_hire`), through `found_cost_choices`.
  The costs: `Engine::employ_cost` (printed reductions of the hired mode,
  its surcharges) for a retainer or ally hired from hand (`hire_from_hand`,
  Zhenga) or a searched retainer (`retainer_found`), Jenna Cross's
  `blood_if_any` after the reductions taken, as `employable_from`'s
  affordability; a searched ally its own (`searched_ally_cost`: no target,
  the surcharges on the recruited version only, rules-qa 119.2); a named
  hire (`Effect::EmployNamed`: Pack Alpha) the named card's
  (`pay_named_discounts`, then `card_surcharge`).

- High Orun: `Lasting::DisciplineBloodOff(n)` on the minion, summed in
  `printed_cost_off_x` for a played mode requiring any Discipline;
  `Effect::HandSizeThisAction` on the seat's timed `Expiry::Action`,
  expired with the action's scope.

### 6.16 Events

`plays.rs`; `defs.rs` section "Events".

- A discard phase action, one per phase (`Seat::event_played`), each event once a
  game per player (`GameState::once_played`), controlled by its player.
  `Requirement::GehennaInPlay` is checked as played only (PIB 20121031);
  `Modifier::NoGehennaEvents` bars Gehenna plays, `Modifier::NoBoons` a
  boon entering play (Blood Trade, which burns those in play:
  `Effect::BurnAllBoons`, `Label::Boon`); `Hook::GehennaPlayed`;
  `Hook::InvestmentPlayed` (`Engine::investment_played`, beside it: the
  `Label::Investment` cards, Swiss Cut's rider, Alan Sovereign's counter);
  their counters taken by other cards (`Effect::TargetCardCounterToPool`:
  Alan Sovereign merged, Wall Street Night) or paying an equip's pool
  (`CostSource::Investment`, Alan Sovereign advanced), the card burned
  empty by its own text (`Engine::burn_if_emptied`).
- Tajdid: `Effect::LiftBloodCurse` pushes the clan on `GameState::curse_lifted`,
  read by `cannot_diablerize` only (the vampire stays blood cursed).
- Global layers read live: `VampireMakingTaxed` (Blood Cult Awareness Network,
  while a ready imbued is in play), `capacity_ceiling` (Wormwood), `SectFallen`,
  `OnlyMustHunt`, `torpid_surcharge`. Timed effects: The Uncoiling's burn due in
  its controller's discard phase (`Engine::discard_dues`, read live; one played
  in a discard phase is due in it, rules-qa 57.5), Thirst as the minion phase
  ends, FBI's damage as the combat ends (`CombatCtx::fbi_damage`).
- The Rising (`Modifier::PoolGainNeedsEdgeOrVp`): `GameState::gain_pool`,
  the one way pool rises (the lint, §3), sends a gain to the bank in its
  Methuselah's own turn without the Edge or a victory point. A bleed's own
  pool (`ActionCtx::pool_if_successful`) is paid before its Edge (LSJ
  20100206-2); the oust scores the victory point before the 6 pool.
  `Until::PreyOusted` → `Release::PreyOusted`, released for the predator
  only, in `oust`.
- Hunger Moon: `Hook::VampireHuntsSuccessfully`, every card's, fired by
  `successful_hunt_riders` after the hunter's own
  (`Effect::HunterBloodToCounters`: the blood moved as counters, burned
  at n).
- White Nights Massacre: a `Hook::DiscardPhase` rider with
  `EnteredBeforeThisTurn` (read by `rider_card_holds`), `Effect::TorporMassacre`
  then `Interrupt::TorporMassacre`.

### 6.17 Ousts and Life Boon

`ousting.rs`: `Ousting`; slice 16b, rules-qa Batch 45.

- An oust is part of the effect that ran the pool out, Life Boon played right
  before it (rules-qa 45.1, 45.4; ANK 20220210). `oust_batch` ousts at once
  (`oust_now`) unless a Methuselah other than the active one and outside the
  batch holds a Life Boon (`defer_oust`): an effect cannot suspend, so the
  batch is marked `Seat::oust_pending` and the caller ends as if the pool had
  not run out (pools below 0 kept: what the Boon must make good; the limit
  recorded as rules-qa 118.4), and `Engine::ousting` is served before any other
  decision: the queued choices (interrupts, a control change, a hunt burn, a
  discard down) wait, unsettled and not told by `Engine::interrupt`, until the
  ousts are done, the Boon's play included (`asking_queued`): each holder,
  clockwise from the active one, passes or plays the Boon through
  `announce_play` (terms `Terms::BoonPool`, as-played window, cancels),
  `playing` while the play is in flight. `settle_ousting` then drops the saved
  and ousts the rest, settling what the oust interrupts (`after_deferred_oust`:
  `combat_ousts`, the turn passing, the action's order). Served first, the Boon
  decision lets no window of the action or referendum under way list the
  Methuselah about to be ousted, but the Boon's own play (its as-played window:
  not ousted yet).
- Pools running out behind a waiting oust queue up, one batch an event
  (`Ousting::batches`), each ousted in turn: `Pending::at` is the pool as
  it ran out plus what the ousts ahead paid it (a predator's 6 pool), a
  Methuselah so kept above 0 spared; the Boon is asked for the front batch
  alone (rules-qa 118.1).
- The pool burns of one event's separate triggers (`leave_riders`:
  Tension in the Ranks, Fame, a vampire's own torpor burn, for minions
  lost together) go through `burn_separately` into `Engine::burn_order`,
  collected over the event (`lost_together`: a strike resolution's burns
  and torpors, `minions_to_torpor` with its riders, Expiation's pair, a
  referendum's burns (Justicar Retribution), `settle_printed_burns`, a
  contest's copies, linked burns) and ordered as it ends
  (`order_burns`): one batch unless two Methuselahs or
  more would burn their last pool; then `Interrupt::BurnOrder` asks the
  acting Methuselah (`acting_seat`) which of those burns next
  (`Choice::ChooseSeat`), the whole order set before `burn_in_order`
  burns them one at a time, each oust applied, or queued behind a waiting
  Boon (`Ousting::batches`), before the next burn, the others' together
  last (rules-qa 118.2, LSJ 20090208-2).
- `oust_and_pass` returns whether the oust applied: its callers return (the
  turn passed) only then, else go on as if the pool had not run out. A
  batch oust (`oust_batch`, `burn_pool_batch`) passes no turn: a play
  ousting its own player so settles it in `resolve_play` (Cyscek).
  A cost the acting Methuselah pays for their action goes through
  `acting_pool_out` instead: off their turn (Enkil Cog, Madness Network)
  the action ends with them and the active Methuselah's phase goes on
  (rules-qa 49.6, 78.48).
- Withdrawal (rules-qa 95): `Choice::WithdrawFromGame`, offered in the
  unlock-phase window (`GameState::may_announce_withdrawal`:
  `Seat::began_short`, set as the turn begins, and the library empty) sets
  `Seat::withdrawal` `Announced`; a pool or vampire blood loss (the
  primitives, §3) or a minion entering combat (`begin_combat`) breaks it;
  `start_turn` takes it first: kept, `Engine::withdraw` removes the seat as an
  oust does (`remove_seat`, `Seat::withdrew`, `Rule::Withdrawal`) without the
  predator's reward or the oust's releases, and the next turn begins.
  `Modifier::ForcedWithdrawals` (Brinksmanship) announces it for a seat that
  begins short (`Seat::withdrawal_forced`): broken, `start_turn` ousts the
  seat (`oust_now`, no Boon: no pool loss); any withdrawal kept ousts the
  card's controller.
- Moments a card from hand answers before they happen wait as an oust does
  (`control.rs`, served after the ousting; `Engine::answer_pending` holds
  every other decision and settling that waits for them): a control change
  (`take_control` with a `ControlTerm`; `ControlChange` in
  `GameState::control_change`, the ones behind it in
  `GameState::control_queue`, each asked in turn;
  `Window::ControlAboutToChange`, its loser asked: The Diamond Thunderbolt,
  `Effect::StopControlChange`), and four queued waits sharing one answer core
  (`control::Wait`; `Ask`: the Methuselahs asked in turn about the queue's
  first minion, `next`, `playing`, `saved`; one `settle_answers` /
  `answer_request` / `answer_apply`, the window, the order and the target
  filter by kind; `save_answer` for the cards saving it), each kind keeping
  its own end (`answered`): a passed blood hunt's burn (`HuntBurn` in
  `GameState::hunt_burn`, `Window::BloodHuntWouldBurn`, every living
  Methuselah asked clockwise from the active one, the hunted vampire's own
  included (§6.8): Redline, `Effect::SaveFromHuntBurn`; the referendum's end,
  `end_blood_hunt`, waits with it, `Flow::Referendum` restored over
  `advance`'s placeholder; a turn's end drops it with the referendum, in
  `next_turn`: Count Zaroff's cancel, its pool cost ousting the acting
  Methuselah, the one answer that can; another wait whose answer could
  needs the same), an ally about to burn in combat (`CombatBurnWait`,
  §6.7: Left for Dead), a vampire about to burn (`BurnWait`, its controller
  asked, §6.11: Abandoning the Flesh), and a ready vampire going to torpor
  (`minions_to_torpor` → `defer_torpor`, only when another Methuselah could
  play a `Window::GoingToTorpor` card on it: `TorporWait` in
  `GameState::torpor_wait`, the vampire provisionally in torpor so a combat
  still ends; asked out of combat at once, in combat as `end_combat` begins,
  which waits (`torpor_holds_combat_end`, RTR 19950509-2); Minor Boon,
  `Effect::SaveFromTorpor`; the unsaved ones, once the last is answered,
  `torpor_now` together, lost together, its riders run with the combat still
  there), and a block about to succeed (`block_would_succeed`: the blocker's
  controller holding a `Window::BlockWouldSucceed` card is asked through
  `Interrupt::BlockWouldSucceed` before `blocked` runs; Lesser Boon's
  `Effect::BlockVoided` sends the action to its blocks-declined step, no
  blocker locked, nothing "if blocked" applied; Powerbase: Zürich's granted
  `Effect::BurnEdgeIfBlocked` burns the Edge in `blocked`, before
  `host_blocked_riders`). The change's costs are paid first; Graverobbing's
  and corruption steals go through it (`ControlTerm::ForGood`,
  `take_for_good_now`); a temporary control ending does not (LSJ 20040518).
- Victory points go through `Engine::gain_half_vps` (prey ousts, last
  standing, a withdrawal, the time limit's half point): a Life Boon in play
  remembering the Methuselah (`CardInPlay::against`) takes them, oldest first,
  its counters the half points given, burned at 1 VP.
  `Effect::StealPoolFromBoonTarget` (`Requirement::BoonTargetActive`,
  `Window::UnlockPhaseAny`).

- A pick from the uncontrolled region made as a master is played, read
  against its minion target (`Terms::OwnUncontrolled(filter)`,
  `cards::uncontrolled_picks` over `holds_for_uncontrolled`, the slot and
  card in `PlayCtx::commanded[0]`; none, the card is not offered), read
  again as it resolves (`picked_uncontrolled`): Tend the Flock
  (`Effect::BloodToPickedUncontrolled`), Thicker than Blood
  (`Effect::PickedUncontrolledToReady`, `slot_to_ready`, its counters and
  cards kept; `Requirement::TargetWouldNotContest` reads
  `GameState::name_in_play`, Gather's check).
- Chain of Command (`Effect::CommandUncontrolled`, named as played by
  `Terms::CommandPicks` over `Engine::commandable`, `PlayCtx::commanded`,
  each re-checked as it resolves): its vampires enter as an influence
  would (`slot_to_ready`, `scarce_toll`) with 1 blood more, recorded in
  `GameState::commands`; `mandatory_actions` withholds the minion phase's
  Pass while one has a bleed (`Due::Bleed`), a successful bleed marks the
  command done, and `settle_commands` (between actions) sends those still
  in play to their crypt's bottom (`minion_to_crypt_bottom`, the cards on
  them burned) once done or none can bleed.

- Consume the Dead: `Window::MinionWouldBurn` (a `cards::Window`, no
  label of its own) answers `BurnWait` (never a blood hunt's `HuntBurn`,
  rules-qa 122.1): `ask_burn` waits for any minion when a living
  Methuselah holds a card for it (not a diablerie's burn), the order its
  controller then the others clockwise from the active one; a retainer's
  burn waits too (`burn_card_in_play` → `BurnThen::Retainer(card)`, keyed
  on its host, `MinionWouldBurn` only); `Effect::RemoveBurningFromGame`
  takes what `save_burning` names (`control::Burning`) out of the game.

### 6.18 Generated data

`scripts/gen_crypt.py` → `crypt.rs`, `scripts/gen_library.py` →
`library.rs`, both `--check`.

- Generated labels read from card text/type line: `Weapon` (incl. "Melee
  weapon."), `Electronic`, `Location` (type line only), `Discipline`, `Grapple`,
  `Aim`, `Title`, `RequiresAnarch`, `MakesAnarch`, `OperationAntigen`,
  `MultiDiscipline`, `Haven`, `Watchtower`, an ally's `Changeling` (read by
  `GameState::has_subtype`: Song of Pan's bleed, Ravager, Winterlich);
  "Unique." after the first sentence; ally body (life,
  strength, bleed) and subtypes. `Title::Independent { name, votes }`. Printed
  justicars keep the clan's old name.
- Pool order: the V5 cards (and `EXTRA_CARDS`) sorted by name, then the
  `ADDED_CARDS` batch after batch (`ADDED_BATCHES`), by KRCG id within a
  batch (`window_matrix.pool_order`): an addition never moves an existing
  index. Titles: archbishop (a city's, 2 votes) and Inner Circle (one per
  clan beside the justicar, 4 votes); the Sabbat's bishop (1 vote),
  cardinal (3), priscus (none of its own, a ballot in the prisci block's 3
  votes: `VoteRecord::prisci`) and regent (4, the only unique one), each
  only while Sabbat. "Flight [FLIGHT]" in a vampire's text is its `FLIGHT`
  Discipline, an ally's on its type line its `Label::Flight` (Herald of
  Topheth, read by `Minion::printed_disciplines`), read as a level
  (`discipline_level(m, "flight")`, Patagia's lasting one too). Paths: `CryptCard::path`, `LibraryCard::path` (a path
  icon, a play requirement: rules-qa 49.7), `Minion::path()` read live,
  `GameState::follows` (present, Sabbat, on the path). A mode line "[pot]
  or [pre]" is either Discipline (rules-qa 52.1). `CryptCard::adv` marks an advanced
  vampire (the same vampire as its base card for contests; its text keyed
  apart, `VampireDef::adv`). A legacy master's "Master: " type prefix is skipped
  for labels; "Out-of-turn. Frenzy." labels `Frenzy`.

### 6.19 Decklists

`decks.rs`, `twda.rs`.

- Decklists: `PRECONS`/`PRECON_LIBRARIES` (the 14 full-size precons) and
  `MINI_PRECONS`/`MINI_LIBRARIES` (the 14 New Blood mini-starters: 6 crypt
  cards, 48–49 library cards), each sorted by set then deck, joined by
  `decks::Deck` (`precon`, `mini`, `anniversary`, `twda`, `all`). The
  anniversary decks (`ANNIVERSARY_PRECONS`/`ANNIVERSARY_LIBRARIES`, 12 crypt
  and 88 library cards): the set minus the extras Black Chantry lists apart,
  the reviewed lists in `window_matrix.ANNIVERSARY_DECKS` (owner: every
  anniversary decklist reviewed before it trains; no Fall of London or
  Shadows of Berlin deck), emitted once every card is in the pool — Reign of
  Stanislava and The Endless Dance; a `--decks` family of its own (`anniversary`, trained
  by default with the precons: a seat draws a clan, then a deck of it), first
  versions for the TWDA cap like the precons. The Sabbat V5 decks
  (`SABBAT_PRECONS`/`SABBAT_LIBRARIES`, `Deck::sabbat`, the KRCG "Sabbat V5"
  set, 12 crypt and 77 library cards each) likewise, once complete, in the
  order they were completed — the Path of Power and the Inner Voice, the
  Path of Cathari, the Path of Caine, the Path of Death and the Soul (all
  four); a `sabbat` family, not trained by
  default yet; `Deck::category` (a Sabbat deck's path, else its clan) is
  what a seat draws first.
- `TWDA` (`twda.rs`, `scripts/gen_twda.py` from KRCG twda.json and the
  generated tables' `id:` order): every tournament-winning deck whose cards
  are all in the pool, tournament-legal sizes (crypt 12+, library 60–90),
  versions capped (`SAME_CRYPT` 25%, `VERSIONS` 3, `SAME_LIBRARY` 50%; the
  largest event's winner first, then the newest; generic vampires such as
  Anarch Convert not counted), TWDA order; each a `twda::Decklist`, its
  catalog `name` (`title (event, date)`) beside the archive's own title,
  event, date and id, which a deck's `decks::Kind::Twda` carries (beside
  `Precon` and `Starter`, the New Blood minis; the server's
  `GET /catalog/decks`); `Deck::crypt_share`, `Deck::clan` (most
  crypt copies); regenerated after a pool change, it picks up the decks
  the additions complete.
- `Engine::with_decks(seed, decks)` seats them in order; `Engine::new` draws
  random precons with the same random sequence as before (goldens
  unchanged).

### 6.20 Legacy additions

Slice 16a; `defs.rs` section "Legacy cards for tournament decks",
`tests/legacy.rs`; rules-qa Batch 44.

- First strike (`Effect::FirstStrike`, `Side::first_strike`): resolved in
  `resolve_strikes` after strikes: combat ends and before the others; the
  pool's only one burns an equipment or retainer (Donnybrook [obf]), so a
  weapon burned so loses its strike; a dodge still protects.
- Before-resolution combat window (`Window::CombatBeforeResolution`): cards
  triggered by the chosen opposing strike (Rötschreck, which also lists
  `Window::CombatStrikeAnnounced`, the window of `Step::BetweenStrikes`: after
  the first strike chosen, before the second, under the same label, only the
  cards listing that window offered, no ability, no any-impulse one; rules-qa
  44.8; with `CombatCtx::aggravated_strike_on`, any Methuselah's vampire: its
  target's `Requirement::TargetFacesAggravatedStrike` is read in `play_options`,
  which sees the combat; rules-qa 94.1). Rötschreck ends combat at once
  (`end_combat_now`), the lock and torpor landing as the combat ends
  (`CombatCtx::rotschreck`), dropped if the combat continues.
- `Effect::BecomesAlly(body)`: a master card in play as its ally (Jake
  Washington), not recruited; `Effect::BloodToTargetX` with `Terms::Count`.
- `Hook::SelfBurned` with `ShuffleSourceIntoLibrary`: a burned ally's card
  shuffled into its owner's library instead of the ash heap (Saatet-ta).
- `Effect::AshurTablets`: three copies controlled as one enters play remove
  every copy (`remove_card_in_play`), 3 pool, then `Interrupt::AshurPicks`
  (hand first, then up to 12 more to the library, shuffled);
  `Requirement::NotPlayedThisTurn` (`GameState::turn_played`).
- `Effect::GainTransfers` (Ennoia's Theater), `Effect::RemoveCryptTopAnarch`
  with `Interrupt::BecomesAnarch` (Gilbert Duane), `Modifier::
  HostInterceptVsVampires` and `Minion::diableries` against
  `Seat::previous_turn` (Carlton Van Wyk), `Effect::OpposingControllerBurnsPool`
  at the end of the round (Hell-for-Leather [obt]; ousts via
  `combat_ousts`), `Requirement::ActingCombatantYourVampire` with
  `Effect::LockTargetCard` (Powerbase: Savannah).

### 6.21 Observations and the legal view

What a card change touches; the encoding, the bots and the training runs
are in DESIGN §11.2.

- `GameState::legal_view(viewer)` hides what the seat may not see: a hidden
  identity is replaced by `HIDDEN` in place, every zone keeping its size and
  every counter, blood, pool and flag kept; a hidden card keeps no copy (it
  would tell which card it is, or where it went).
  - Hidden from all: libraries and crypts (a search shuffles afterward), the
    cards Heart of Nizchetus puts at the bottom included (remembering them
    is the policy's business). The viewer's own keep their contents, sorted
    (a Methuselah knows their decklist and sees every card leave or enter),
    but a crypt's or library's top its controller sees stays on top with
    its copy (`GameState::crypt_top_seen_by`, `Stat::SeesOwnCryptTop`:
    Grendel; `library_top_seen_by`, `Stat::SeesOwnLibraryTop`: Tusk; the
    host's view gives it its `id` and number, so seeing it bars an undo).
  - Hands, but an open one (Owl Companion [ani], Harold Tanner, Revelations
    [AUS]: `PreyOpenHand`, `OpponentOpenHandInCombat`, a seat's
    `combat_open_hand`), read through `GameState::hand_visible_to`.
  - The research area (`Seat::research`, Research): its owner's only.
  - Public: ash heaps (`ash`, `ash_crypt`), removed cards, minions and cards
    in play (a contested card turned face down keeps its identity), pool,
    blood, counters, `held` (a count); a banished vampire's uncontrolled
    slot (it left play in view of all), every slot's counters; face-up
    stored cards (Black Market Cache, Fleshforge Chamber), Shilmulo Tarot's
    and Storage Annex's face-down ones (`Effect::StoreTopCards`,
    `StoreFromOwnHand`) and Hukros's (`Minion::stored`) to its controller
    only. The RNG is not in the state.
  - A card's chosen minion hidden by `Modifier::ChosenHidden` (Betrayer):
    its controller only (`GameState::chosen_seen_by`).
  - The action, combat and referendum contexts and the as-played stack are
    public, except a card named from hand (`PlayCtx::named`, Pack Alpha;
    `ActionCtx::exchange`'s hand card, Lenelle; `ActionCtx::card`):
    announced but never shown, so an observation or a bot never reads it,
    and an observation adding these contexts must redact it.
- A new `Choice` kind or a new option shape needs its observation slot
  (`crates/env/src/obs.rs`: option ids and `OPT_F` features); a new rulebook-like choice
  usually shares an existing kind's slot (`Choice::UnlockInfernal` in
  `UnlockForBlood`'s, `BurnBloodToAvoid`'s for the Society of Leopold).
  Vocabulary keys (`obs::vocab_keys`) keep trained networks across pool
  extensions.
- `game::public_choice` is the whitelist of choices recorded as public
  history (`Engine::history`) and counted in each seat's profile
  (`Engine::profiles`, `profile.rs`; read by the observations' seats'
  context, `crates/env/src/context.rs`): a choice that reveals hidden
  information stays out of both.
- `crates/env/tests/obs.rs` (the encoding of the full state equals that of
  the seat's legal view; every pointer names a token) and `bots.rs` (bot
  decisions answered again from `legal_view`) guard all three.

### 6.22 Card copies

`copies.rs`: `CopyId`, `CopyCounter`, `census`.

- Every physical card has a copy number, dealt at the start in decklist
  order (per seat in seating order, crypt then library, as listed),
  independent of the seed and the shuffle, and carried with the card through
  every zone (`Pile` moves a card with its copy). A card the game would
  create takes the next number (`GameState::copy_counter`,
  `CopyCounter::deal`; none yet: a ghoul token is a library card).
- Copies serve clients (a card followed across the table) and records; they
  never take part in the game's identity (a `CopyId` stored in a new field
  goes in `Untracked`, as `CardInPlay::held_vampire`'s): `Pile` compares and
  hashes as its card ids alone, `Untracked` fields not at all, so the state
  hash, observations and options do not see them. The rules that follow one
  physical card read them (`copies.rs` lists them, Steely Tenacity's retake
  and Gift of Proteus' earmark among them), so a fixture numbers its cards
  (`number_unnumbered`) before a game and before its replay alike: two
  games apart there diverge under equal hashes.
- `census` lists every card with its copy and place (`Place`: a seat's
  zone, the uncontrolled region, a card or minion in play, stored on a card or a
  minion, in flight, on a banished vampire, in training); the fuzz tests check,
  at each decision, each dealt copy stays its card and is found exactly once,
  until its Methuselah is ousted (their cards are removed from every zone).
  Mid-run a card is not always so: a play resolving is in no place (one being
  opened is in flight, `Engine::opening`: a play canceled as it is announced is
  told into limbo first), and the action's card is in flight still once in its
  zone; the host's narrator reads one place a copy, a missing one its last
  (`narrate::one_place`, `Session::places`), and tells every move between two
  places (`card_moves`).
