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
…)` → `Flow::AbilityTerms` → `resolve_ability`.

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
a family that cannot reach them is `unreachable!`. An `Action` effect from
an ability needs an action under way: such abilities are offered in an
action's windows only (tested, §3).

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
(an ally pays no blood; `cards::plays_card_as_vampire`), `cb.allows_from`,
granted-action checks (`may_take_granted`, the turn's political action,
Delaying Tactics, a referendum feasible), then `ability_holds` (the
requirements read by name, the card's, host's and play's evaluators).
`limit_slot(limit, source, ability)` maps a `Limit` to its `Ledger`
(combat, round, action, referendum, turn), key and allowed uses, read by
`limit_spent` and written by `mark_limit`. `Engine::gains_nothing` tells a
legal no-op for interfaces that hide it (rules-qa 17.4: the engine still
offers it).

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

**Held and Release.** `state::Release` names the moments held replacements
are drawn; `Held` is `[u8; Release::COUNT]` indexed by it (`Seat::held`,
and `Seat::learjet_held` for Learjet's extra draws). `Until` is the
definitions' vocabulary (`CardDef::delayed_replacement`); `Until::release`
maps it to a `Release`, `LeavesPlay` to none (`Seat::held_in_play`: a list
of cards). `Weakens`, `MasterPhase`, `MinionPhaseEnd` are engine delays.
Every release goes through `Engine::replace_held` (`release_held` loops it
over the seats).

**Timed seat modifiers.** `Seat::timed` (`state::Timed`): an amount per
`TimedStat` (`HandSize`, `HuntStealth`, `ExtraDraws`) until each `Expiry`
(`EndOfTurn`, `DiscardPhase`, `UnlockPhase`, `Combat`, `Never`), each
swept at one line (`Timed::expire`): `DiscardPhase` in `end_influence`
after the held discard-phase replacements, `EndOfTurn` in `end_turn` after
temporary control ends, `UnlockPhase` in `start_turn` before the held
unlock-phase replacements, `Combat` as the combat ends. Other shapes stay
fields (`combat_open_hand`, `no_transfers_next`, `transfers_next`,
`mpa_next`, `veil_left`).

**Layers.** `Layer { scope: InPlay | Action, modifier, source }` in
`GameState::layers`, pushed by `Effect::Layer` / `Effect::ActionLayer` and
`push_mode_layers`; `GameState::layer_active` suspends a contested card's.
Stats are read live through them (`in_play_layers`, `global`). Printed
vampire text is not layered: `GameState::printed_mod` reads it live
(§6.12).

## 3. Enforced and conventional

**Enforced by the compiler.** `Effect::family` and `Requirement::slots`
are exhaustive; `Release`, `TimedStat`, `Expiry` are enums indexing their
tables; `limit_slot` matches every `Limit`.

**Enforced by tests over every definition** (unit tests in the engine's
`--lib`, the rest under `crates/engine/tests/`):

- `every_effect_sits_where_its_family_resolves` (`cards/families.rs`): each
  effect of `DEFS`/`VAMPIRES` sits in a position whose interpreter resolves
  its family (a mode, an action card's mode, an ability, a granted action,
  a rider, an `If` bonus, an `IfBleedSuccessful` body);
- `abilities_changing_the_action_are_used_in_one` (same file): an
  ability's `Action` effect is offered in an action's windows only;
- `every_requirement_sits_where_it_is_read` (`cards/slots.rs`): each
  requirement of every mode, ability (by actor), rider (by hook), target
  filter, printed modifier, `If`, term choice and `AnyOf` sits where an
  evaluator reads it; an unclassified actor or hook fails closed;
  `KNOWN_GAPS` is empty (an entry nothing matches fails as stale);
- `every_ability_cost_carries_its_requirement` (same file): every ability
  cost carries its `implied_requirement`;
- `every_card_is_defined_and_its_modes_match_the_reviewed_windows`
  (`tests/plays.rs`): every library card defined, every vampire definition
  resolving to its crypt card, the modes' windows equal to
  `data/window-matrix-reviewed.csv`;
- `plain_stat_halves_apply_and_every_vampire_has_all_its_text`
  (`tests/vampires.rs`): no `incomplete` left (rules-qa 13.1);
- `no_in_play_effect_requires_a_justicar` (`tests/twda_unlocks.rs`);
- `legal_no_ops_are_offered_and_known_to_gain_nothing` (`tests/precon.rs`).
- `no_card_name_in_engine_code` (`tests/lint.rs`): no card name as a string
  in hand-written engine code outside the definitions; each exception in
  `CARD_NAMES_ALLOWED` carries its reason, a stale one fails.

**Enforced by play.** The fuzz fixtures (`tests/fuzz.rs`: invariants,
conservation of copies, replay of every 4th seed, each fixture's `covers`
keys taken by random play) and their goldens (`tests/golden/`, DESIGN
§7.3); perft counts (§7.5); `tests/elision.rs` (elision is observationally
equivalent); `tests/obs.rs` and `tests/bots.rs` (nothing read beyond the
legal view); `crates/host/tests/keys.rs` (no choice offered twice, a key
per option). A refactor keeps the goldens and perft green without
`GOLDEN_BLESS`; a bless names the moved seeds and decision ranges.

**Enforced by scripts.** `scripts/rulings_v5.py --check
data/rulings-v5.csv` (every ledger row on an implemented card backed:
`test:` / `blocked:` / `n/a:`); `scripts/window_matrix.py --check`; the
generators' `--check` (`gen_crypt.py`, `gen_library.py`, `gen_twda.py`).

**Still convention** (what the refactor left of the review's recurring
hazards, docs/engine-review-2026-10-14.md §3):

- A `Rider`-family effect, or a shared one at a new hook: the families test
  checks its position, not that the hook's site handles it; a site not
  knowing it panics in a game (`apply_rider_effect`'s `unreachable!`).
- A predicate meaningful on printed text: `holds` and `printed_holds` are
  two evaluators; `slots` checks where a requirement sits, not that both
  agree. Shared readings go through one helper (`ActionCtx::is_hunt` /
  `PrintedCtx::is_hunt`, `GameState::ready_named`).
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
  `LockOrPay`, …) is one request/apply pair each (review F11, left).
- `host/src/effects.rs` maps an effect for clients only where curated;
  `unmapped_effects_by_family` lists the rest, informational.
- Card-name literals in engine code: three sites with a reason (Mithraic,
  Anarch Convert, Life Boon) and the Blood Brother circle checks; any other
  name or printed number belongs in `cards/defs.rs`.
- Conditions are read live, never snapshotted (§6.5, live conditions): no
  test can see a snapshot that nothing changes yet.
- Third-party intercept and stealth strips must be counted by block
  pruning (§6.6), or pruning hides a live attempt.

## 4. Adding a card

1. **Read the rulings first** (CLAUDE.md "Owner rules"): the card's KRCG
   rulings, its rows in `data/rulings-v5.csv`, `../vtes-advanced-rules`,
   and rules-qa (its topical index, then the "Default readings": choices
   at declaration; a card played by another card is still played; costs
   payable even if the payer burns; caps read narrowly; card text
   addresses its controller, 32.7; Ⓓ only indicative, 20.1). The design
   satisfies each ruling.
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
   masked (rules-qa 17.4).
5. **Tests**: one file per family under `crates/engine/tests/` (the
   round's file for TWDA rounds); `tests/common` helpers
   (`CardInPlay::new`, `never_offered_until` to prove a non-offer, `plain`
   for a vanilla vampire). Window elision affects naive tests (CLAUDE.md
   "Working lessons"). A test in `tests/rulings.rs` backs a ledger row;
   regressions go elsewhere.
6. **Ledger**: each of the card's rows in `data/rulings-v5.csv` (CRLF)
   gets `test:<fn>`, `blocked:<gap>` or `n/a:<reason>`; run the `--check`.
7. **Fuzz**: add the card to a fixture's `slice` and a `covers` key
   (`"play:<card>"`, `"ability:<name>"`, …) for what random play must
   reach; `seeds` about twice the seed where the last key first appears
   (at most 300). Adding a card reshuffles the fixture's games: re-bless
   with `GOLDEN_BLESS=1`, the change explained in the commit. Also diff a
   `{:#?}` dump of `DEFS`/`VAMPIRES` for a refactor.
8. **Unsettled readings**: a reading no ruling settles is implemented and
   recorded as a question in a new rules-qa batch; an issue met on the way
   is fixed when small or listed in `docs/findings.md`.
9. **Elsewhere**: a player-visible effect may need a line in
   `crates/host/src/effects.rs`; a new `Choice` kind needs its obs slot
   (§6.21); the checks of CLAUDE.md "Working lessons" before calling it
   done.

## 5. Comment policy

The owner's policy (2026-10-14, applied to the engine in one pass). Code
comments are limited to:

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
`copies.rs` (copy identities and their census, §6.22), `obs.rs` (observations),
`decks.rs` and `twda.rs` (decklists), `agent.rs` and `bots.rs` (policies),
`politics.rs` and `habits.rs` (agent behaviour around a network, not
engine masks), `cards/mod.rs` (the vocabulary: `Window`, `Actor`,
`Requirement`, `Terms`, `Effect`, `Modifier`, `Hook`, `Limit`, `Until`,
`CardDef`, `ModeDef`, `Rider`), `cards/families.rs` (`Effect::family`),
`cards/slots.rs` (`Requirement::slots`, `implied_requirement`),
`cards/defs.rs` and `cards/vampires.rs` (definitions),
`crypt.rs`/`library.rs` (generated). Tests: one file per family under
`crates/engine/tests/` (`precon.rs` holds the slice-11 cards by section).

### 6.2 Run loop and interrupts

`game.rs`, `draws.rs`.

- `Engine::run`: settle interrupts and a deferred oust, draw up, drain
  excess blood (`drain_all_excess`), release hostages and torture, then
  request; single-option requests auto-apply.
- Hand size is settled live both ways (rulebook "Drawing Cards": at once):
  discard-down whenever `hand_excess` > 0; draw-up (`settle_draw_up`)
  whenever a hand is below its size with a library to draw from, between
  plays only (no interrupt, ousting, action, combat or referendum; a phase
  flow), so a card played but not yet replaced is never drawn for early.
  `GameState::hand_load` counts the hand, `held` and `held_in_play`.
- A choice owed by a specific player in the middle of a resolution is an
  `Interrupt` (`Engine.interrupts`), served in order before any other decision:
  discard-down, draw alternatives, replacement treatment, entry effects
  (`EntryBurn`), margin distribution, `MinionOf` picks, block tolls, etc. A player
  with nothing to choose resolves at once.
- An ousted seat drops out of open windows (`OrderedWindow::remove`).
- `Window::AnyImpulse` (untimed "you can lock this card" abilities) is offered
  wherever a seat holds the impulse — phases, unlock rotation, action/combat
  windows, polling — never in as-played or terms windows; using one is an effect
  (impulse reset / polling commitment). A starving vampire's forced hunt is
  offered beside untimed abilities, not instead.

### 6.3 Turn and phases

`game.rs`, `contests.rs`, `influence.rs`, `plays.rs`.

- Unlock: DESIGN §10.7 (`Flow::Abilities`, `Pending`). Cards a Methuselah
  controls unlock in their unlock phase. Substitutions: `Minion.skip_unlock`,
  `Modifier::CounterInsteadOfUnlock`, `HostDoesNotUnlock`. The rulebook burn
  option is a discard in any unlock phase (`Seat::burn_option_used`).
- Unlock-phase effects (`start_unlock_effects`, `Flow::Abilities`): one
  ordered impulse sequence clockwise from the active Methuselah, who settles
  their mandatory effects (contest upkeep, every pending rider) in the order
  they choose, among their optional ones (the Edge's pool, abilities), and
  cannot pass before; then the other seats' abilities. A contest with no
  rival left is released and a forced yield settles at once: neither is a
  decision.
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
  (`Engine::master_dues`, `discard_dues`).
- Master phase: out-of-turn masters are accounted against the next master phase
  (rules-qa 3.3); a canceled master gives its player a master phase action back
  when Wash says so (`Seat.mpa_next`). Abilities paid with a master phase action
  are not once-per-phase. `GameState::mpa_left` adds Rumors of Gehenna's
  "two (instead of one)" neither to Anson's second nor to another copy's.
- Minion phase: `Engine::mandatory_actions` — a mandatory action (printed,
  Puppet Master's take) is the only offer and the phase cannot end before it; a
  minion unable to take it declares nothing (spec §3.9). Several mandatory
  actions are ordered by the acting Methuselah (rules-qa 17.2).
- Influence: transfers, `Choice::Withdraw` (2 transfers: a counter back to pool),
  crypt draw (4 transfers + 1 pool), influence-out, no voluntary self-contest;
  `Window::InfluencePhaseOwn` (once per phase), `Window::Transfers` (paid in
  transfers); `GameState::transfer_bonus`.
- Discard: one discard phase action (`Stat::DiscardPhaseActions` read as the
  phase begins, `Seat::granted_dpa`: Powerbase: Los Angeles's, used to
  discard a card requiring an Anarch or making a vampire Anarch, lets its
  user unlock a ready Anarch); `Hook::DiscardPhase` riders fire as the
  impulse leaves the active Methuselah. The end-of-turn hand size
  (`Seat::timed`, `Expiry::EndOfTurn`) ends at the end of the turn.
- "During X, do Y" abilities are once per that phase per copy (`phase_uses`,
  rules-qa 6.6); an ability spending a resource with no "during" clause
  (`DiscardPhaseAction`, `Transfers`) is limited only by the resource.

### 6.4 Card plays

`plays.rs`: `PendingPlay`, `PlayCtx`.

- Offers are enumerated from definitions: windows, actor, requirements,
  targets, limits, affordability counting potential reductions.
- Announcement: terms (`Flow::PlayTerms`), cost reductions (`Flow::CostTerms`),
  then the as-played window (`Flow::AsPlayed`, a nested cancel stack, spec §1.8).
  Costs are folded through layers and paid at resolution for actions (unpayable
  → fizzle: successful, no effect; rules-qa 3.6).
- Cancel as played: the card is replaced; its own "do not replace until" is
  canceled with it; a canceled limited bleed leaves the limit unused; cancelling
  abilities (The Vozhd of Gravesend) mark the play canceled as used
  (`as_played_apply`). Cancels by type/Discipline read the mode played.
- Multi-Discipline cards: the generator gives one mode per non-empty set of their
  Disciplines (`Label::MultiDiscipline`); a mode's tags are all required, as on a
  combo card. A definition may split a printed "or" into modes, mapped back by
  its `CardDef::lines` (`cards::library_mode`). Surcharges read the Disciplines used
  (`played_disciplines`).
- Cards played from a location (Fleshforge Chamber: `virtual_hand`,
  `PlayCtx::from`) are not replaced; `Engine::playable_card` reads a play's card.
- Allies playing cards "as a vampire" (`Modifier::PlaysAsVampire`): meet one
  basic Discipline, pay in life (last life burns them; the card then has no
  effect, an action resolving first). The grant is read for the card it plays
  only (`cards::plays_card_as_vampire`, its "non-action" limit included;
  `has_disciplines` reads it so): `holds`' `ActorSectIn` /
  `ActorCapacityAtLeast` read sect and capacity alone (rules-qa 73.6).

### 6.5 Actions

`actions.rs`: `ActionCtx`, `ActionState`; `ActionKind`, `ActionId`.

- Kinds: rulebook (bleed, hunt, leave torpor, rescue, diablerie, political),
  `Card`, `Equip`/`EquipFrom`/`EquipFound`, `Recruit`/`Employ`, `BurnCard`,
  `StealCard`, `Ability` (granted by a card in play or printed on a vampire;
  checked by `ability_gate`, `may_take_granted` inside it). A granted action
  has only its printed stealth (rules-qa 6.2).
- Hunting: `actions::is_hunt(kind, card_hunt)`, read through
  `ActionCtx::is_hunt` and `PrintedCtx::is_hunt`: an action a card makes a
  hunt (`Effect::HuntAction`: Loki's Gift, Psychophagia, Abactor, Week of
  Nightmares) is hunting for everything reading one (LSJ 20050720-2;
  rules-qa 72.1–72.5). `ActionCtx::card_hunt` is the card's part only;
  `Requirement::RulebookHunt` is the rulebook hunt (Zöe's "usual +1").
- Directedness is `ActionCtx::directed_seat`: bleed target; rescue/diablerie of
  another's vampire; a minion, seat or card target's controller unless the acting
  Methuselah (a card target never makes the action directed at its minion). The
  Ⓓ icon is only a reminder (rules-qa 17.4, 20.1). `GameState::may_direct_at`
  (Shadow Cloak, Secure Haven) filters directed actions of every kind.
- Windows: as-announced (acting Methuselah, then others: `ReactionAsAnnounced`)
  → block loop (`ActionBlockAttempt`, `ReactionUnlock`,
  `ReactionUnlockAttempt`) → `ActionState::Blocked` (Crypt's Sons continue as if
  unblocked) → block resolution → combat → after-combat → for a bleed of 1+,
  `ActionWouldBeSuccessful` → resolution → after-resolution.
- **After resolution always opens** once an action reached resolution —
  successful, fizzled, failed, blocked (after its combat), or ended before block
  resolution (Change of Target, Mirror Walk; no card is played there). Only a
  cancel before resolution or an acting minion leaving skips it (`action_windows`).
  Damage "after resolution" is ordered there by the acting Methuselah.
- Resolution: costs paid then; targets re-read — gone or illegal → fizzle, paid
  (card actions) or successful with no effect (granted actions, spec §3.4.1). An
  action whose acting minion left play fails. A bleed target ousted before
  resolution: fizzled, no pool, no Edge. A **successful bleed** is one of 1+;
  only it takes the Edge (`ActionCtx.bleed_successful`); a bleed at 0 is still a
  successful action. A hunt succeeds only if it gains blood.
- Canceled before resolution (The Kiss of Ra, `Effect::CancelAction`):
  `ActionCtx::canceled` ends the action from the declined state with no
  outcome, no after-resolution window and no NRA record unless it was
  blocked before (`blocked_by`).
- **NRA**: `ActionId` — card name for hand cards, `InPlay(card, ability)` per
  copy and ability (LSJ 20100909), `Printed(vampire, ability)`, `Rulebook(kind,
  target)` (Change of Target's ban); `Effect::SameActionAs` unifies two printed
  actions. Recorded at resolution (`mark_resolved`); a failed bleed or political
  action counts. Delaying Tactics bans by `ActionId`. `Printed` keeps the
  vampire so a record the seat holds (`Seat::forbidden_political`) still
  names it.
- **Queued combat** (Hedonism: `ActionCtx::queued_combat`,
  `begin_queued_combat`, `resume_after_queued_combat`): it begins once the
  block's own combat is over (block resolution), or after a successful
  action's effects, still part of its resolution (Sabbat V5 rules "Queue a
  combat"; rules-qa 53.3; a fizzled action counts as successful, 56.1);
  begun after the effects, its end goes on to the after-resolution window
  (`queued_at_resolution`). Both minions must still be ready (advanced rules
  4.9.3); an action failing or ending first loses it (LSJ 20090826).
  Psyche!'s queue is the combat's (§6.7).
- **Forward Momentum** (`ActionCtx::momentum`, `bleed_ends_riders`,
  `finish_action`): +1 counter after a successful bleed, −1 otherwise (a
  blocked bleed included), pending in the after-resolution window beside
  its unlock, in the controller's order (rules-qa 53.6): a gain can be
  taken by the unlock first, a loss after it; the rest applies as the
  action ends.
- **Diablerie's blood hunt** (`end_action_into_blood_hunt`): part of the
  action (spec §3.7.8.1), which ends after it through its after-resolution
  window (rules-qa 5.8); the referendum's rotation starts at the
  diablerist's controller, clockwise (DESIGN §4.2).
- **Live conditions** (owner, 2026-09-25): an effect's conditions are evaluated
  throughout, never snapshotted (`Effect::If` → `ActionCtx.conditional`); a
  limited bleed increase counts against the limit only while its condition holds.
  The ruling-backed exception: Night Terrors' −X stealth is fixed at play (RTR
  20030519).
- Intercept given to a minion lasts the whole action, across failed attempts
  (`ActionCtx.intercept_given`, rules-qa 15.2).
- Enter-combat actions (`Effect::EnterCombatWithTarget`): the combat begins at
  resolution (`start_combat_at_resolution`) and is part of the action; no "held
  when blocking" state; a blocked rush continued as if unblocked enters its
  combat at resolution (rules-qa 17.3).
- Action-scoped riders live on `ActionCtx` (block riders read at announcement,
  `barred_blockers`, `vampires_cannot_block`, `no_unlock`, `no_reactions`,
  `put_if_no_block`, `after_resolution_unlocks`, `cost_reduction`, …); combat
  riders from the action go through `Engine::combat_riders`.

### 6.6 Block loop

`actions.rs`.

- Eligibility: prey/predator/target rule (`eligible_blockers`), widened per
  attempt by `can_attempt_block_anyone` (Eagle's Sight: one attempt, not the
  acting Methuselah's own minion nor a decliner nor a barred blocker).
- Forced attempts (`Effect::AttemptBlock`, `R::CanAttemptBlock` →
  `can_attempt_block`). A card-made failure bars re-attempts this action.
- Tolls (`block_toll`): paid as the attempt is made; an attempt under way when a
  toll lands pays or resigns (`Interrupt::BlockToll`). A forced attempt
  (`can_attempt_block[_anyone]`, `AttemptBlock`, `TargetAttemptsBlock`) needs
  the toll payable (rules-qa 42.8).
- **Reachability pruning** (`intercept_reachable`): an attempt that cannot reach
  the stealth is not offered. It counts the blocker's own and printed intercept,
  intercept abilities of cards on it, other cards in play
  (`location_intercept_reach`: Market Square, KRCG News Radio, The Rumor Mill),
  and stops pruning while a non-acting Methuselah could strip the stealth
  (`stealth_strippable`): the blocker's own Methuselah by a playable card in
  hand, another by holding any card while one of their minions could play a
  stealth strip of the pool (Night Terrors) — their hand is hidden from the
  blocker (DESIGN §4.3; reading it leaked, found by `tests/elision.rs`). An
  intercept card in hand also counts its replacement draw (rules-qa 48.5: one
  to play and a library may yet meet any stealth); conditional intercept
  counts whether or not its condition holds now. The rule itself is the
  closure comparison in `action_advance`. Any new third-party
  intercept/stealth-strip card must be counted here, or pruning hides a live
  attempt.
- **Failed attempts** (`ActionCtx::attempt_print`, `failed_alike`,
  `failed_attempts`, rules-qa 48.6): a minion that failed an attempt is not
  offered the same attempt again until what it faces changes, in public
  terms (a hash: the stealth; its intercept, blood and lock; the count of
  cards in play; every hand's size; its Methuselah's library and ash heap
  sizes). Hands themselves are never read (`tests/elision.rs`).
- The engine has legal unbounded repeats here (block re-attempts, DESIGN
  §11.3): cap decisions in any exploratory driver.

### 6.7 Combat

`combat.rs`: `CombatCtx`, `Side`, `Step`, `Damage`, `Held`.

- Round steps (rulebook "Combat Sequence"): before range → determine range
  (alternating maneuvers) → before strikes → strike choice (acting first) →
  before resolution (spec §4.3.2) → resolution (combat ends first, else
  simultaneous; steal before mend) → prevention (acting side first, spec
  §4.5.4) → damage at once → additional strikes → press (alternating) → end
  of round, shared with the would-end replacement point (spec §4.9.1). Each
  ordered step is a two-seat window with impulse reset (rules-qa 1.2);
  strike choice and prevention are single-seat offers. A combatant no longer
  ready ends the round at once, the end-of-round window still opening
  (rulebook step 7).
- Only combat cards are played during combat — no modifiers, reactions or wakes
  (spec §4.1.3). A combat is part of its action: `Engine::action` and its layers
  stay in force.
- Strikes: a weapon strike remembers its source (`Side.strike_source`) and does
  nothing if the weapon left; a maneuver from a weapon or strike card commits the
  initial strike (`Side.committed`, `committed_card`); the round's one limited
  additional-strike source; a strike not said to be ranged is close range only
  (rules-qa 21.1). Strike riders fire as the strike lands (in range, not dodged,
  not aimed at a retainer). Ranged strikes at long range may be aimed at a
  retainer (`Side::strike_at`).
- Damage: prevention per side, acting side first, then third parties from outside
  the combat (`Step::OthersPrevent`); the preventer picks which damage a
  prevention covers (`Step::AllocateSource`); applied at once, normal then
  aggravated (the rulebook's examples are unit tests). Environmental damage lands
  in the normal strike resolution. Unpreventable damage (`Side::unpreventable`)
  stays out of `pending`. A damage resolution is atomic, no window opening
  midway (PIB 20121031): an untimed lock ability (The Coven) is offered in
  the prevention window, before damage lands. Anathema's "blood reduced to 0
  in combat" is read as each step settles (`Engine::settle_anathema`).
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
  continues (rules-qa 21.1, 47.1). Only action cards wait in limbo.
- Per-combat/round/action ability uses key on the card copy: a card's abilities
  share one use (rules-qa 38.8).
- Frenzy: `Effect::Frenzy` marks the frenzy mode; `Engine::frenzy_on` names the
  minion affected (rules-qa 28.11); `frenzy_immune` cancels and bars.
- Weapons: melee `StrikeKind::Melee` (not a hand strike); a weapon's per-combat
  maneuver is `Limit::PerCombat` per copy; `cards::melee_strike`,
  `cards::becomes_weapon` (Weighted Walking Stick is a weapon only in play).
- Aims (Target Vitals) come after the strike choice: a side whose strike
  is chosen gets one more `ChooseStrike` decision, an aim or pass
  (`Side::aim_declined`; rules-qa 62.1). Damage reduction is not prevention:
  `GameState::damage_reduction` (Nephandus, `Modifier::OpposingDamageLess`)
  lowers each opposing strike's damage before it is pending — unpreventable
  and after-combat strike damage included, environmental not.
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
  `Modifier::AdditionalStrikeEachRound` (Renegade Garou) adds a mandatory
  strike after the first pair, in the limited slot. Torrent's "continues as
  if unblocked" is `Side::continue_free` → `ActionCtx::continue_free`, taken
  as the after-combat window closes, cleared if the combat goes on.
- Psyche! [CEL] queues a combat as one would end (`CombatCtx::psyche`):
  `end_combat` releases the after-combat held cards, drops the ended
  combat's after-combat effects and begins the new one with the same
  routing flags (`at_resolution`, `after_resolution`, `queued`, `yawp`),
  no `held_at_block` (rules-qa 65.1).
- Flames of Insurrection's offers come as a minion opposing an Anarch leaves
  the ready region, to both controllers when both Anarchs leave
  (`Engine::flames_offers`).

### 6.8 Referendums

`referendum.rs`: `ReferendumCtx`, `RefKind`, `RefStep`, `Source`,
`VoteRecord`, `TermsState`; `ReferendumDef`, `Term`, `RefEffect`.

- Terms stepwise by the caller after the action succeeds (feasibility masks);
  a `Term` choice is never a target (rules-qa 38.6). Polling with card plays as
  commitments; per-source vote records (abstentions cancel); Methuselah ballots
  (Edge, the calling card's vote, one political card burned from hand); tally:
  more for than against, ties fail; Scorn first.
- Votes are read live (`votes_of`, `clan_votes`, `calling_votes`, `host_votes`);
  grants (`ReferendumCtx.grants`) add to casts already made. An ousted
  Methuselah casts nothing. Mithras's toll on votes against his political
  action is `Engine::vote_against_toll` (beside `block_toll`); Alvaro's "if
  he abstained" gain is read after the referendum (`resolve_political`).
- A canceled referendum returns the card, discarding down at once. Political
  action: +1 stealth, cost at resolution, no referendum if the acting minion is
  no longer ready.
- Simultaneous ousts are one batch (`oust_batch`): a VP to every predator, 6 pool
  to survivors only; every remaining Methuselah ousted at once is a legal end
  (rules-qa 6.9).

### 6.9 Contests and titles

`contests.rs`; `state.rs`.

- `Contest::{Card, Title, Library}`. A contested card is out of play:
  `CardInPlay::present()` false, its layers suspended (`GameState::layer_active`),
  skipped by abilities, riders, copy counts and "a card named X". A library card
  enters contest only after its own effects resolve (spec §1.13.2).
- Self-contest is masked (rules-qa 7.1; `controls_unique_copy` also for
  searches); a forced one burns the incoming copy (The Coven).
- Titles are read live (`GameState::title_of`): **claimed** (`claimed_title`: a
  title card on the vampire, else `Minion.granted_title`, else printed unless
  lost) vs **effective** (none while contested, out of play, off-sect, or its
  sect fallen). One title per vampire (`gain_title` loses the printed one and
  burns an older card). A unique title card contests as a card (pool), giving its
  title once the contest ends (rules-qa 19.1; in `place_now`); a non-unique one
  (Toreador Justicar) contests as its title (blood). `steal_title` (Valerius)
  burns the card and gains the title itself (rules-qa 19.6). Liaison is
  `Title::Independent { "Liaison", 4 }`, the younger claimant yielding.

### 6.10 Minions

`state.rs`: `Minion`, `MinionKind::{Vampire, Ally, Created, Token}`.

- Vampire traits are read live: `capacity_of` (with `capacity_ceiling`),
  `discipline_level`, `has_discipline`, `Minion::clan`, `GameState::sect_of`
  (`SectFallen` over `underlying_sect`, `sect_override`); printed data only via
  `printed_*`. Blood gain caps through `gain_blood`; excess drains as the engine
  settles; out-of-play counters are kept. Gestalt [SAN]'s borrowed
  Disciplines last the action (`GameState::lent_disciplines`, read by
  `discipline_level`). Blood gains are noted between decisions
  (`GameState::blood_gains`, `Engine::note_blood_gains`), a strike's as the
  strikes end (Slake the Thirst, rules-qa 68.2).
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
- **Created vampires** (`MinionKind::Created`: Childe of the Revolution, Waters of
  Duat): 1-capacity, sire's clan, printed or sire's sect, no Discipline or title;
  their card is the body. **Tokens** (`MinionKind::Token`: Absimiliard's Army):
  no card in play, name or label; removed without burning when the Army leaves.
  `MinionKind::Fury` (Unleash Hell's Fury) is out of play but for its own
  block attempts (`Engine::fury_tokens`). Infernal Servitor's removals (no
  infernal vampire controlled; after a successful action) are read as each
  step settles (`Engine::settle_servitors`) and as its action ends.
- Leaving play has one path each: `burn_minion` (cards on it to owners' ash
  heaps; "ready minion burned" riders only if it was ready) and
  `minions_to_torpor`, both through `leave_riders` batching pool losses. Burned
  vampires reach the ash heap (`Seat.ash_crypt`; its owner's, `Minion::origin`,
  even under stolen control).
- Control: permanent steal (`StealTargetVampire`), temporary
  (`Minion.temp_from`, `Engine::take_control`: returns at the end of the taker's
  turn, removed if its Methuselah was ousted). Banishment (`Engine::banish`:
  Banishment, Pit of Contemplation) keeps the vampire's blood as counters in
  the uncontrolled region, the cards in play on it (not those burned from
  it; a created vampire's own card included) face down with it, out of play
  until it returns as the arena entry it left (`UncontrolledSlot.banished`,
  rules-qa 25.1).
- Per-minion markers: `corruption` (per Methuselah; kept through torpor, control
  change, banishment; cleared when burned/removed), `hostage` (Carver's Meat
  Packing and Storage: while any, no move to the ready region nor
  diablerie), `torture` (The Path of Lilith: its controller burns 1 pool each
  unlock phase while any of their vampires has one; released once no such
  card is in play, `GameState::release_torture`), `nightmare`,
  `diablerie_shield`, `must_hunt`, `played_since_unlock` (wake limits),
  `unlock_at_discard`.
- Slaves (rulebook "Slave", Tupdog): the directed-action bar is printed
  (`Stat::NoDirectedActions` with `ControlsNoReadyOfClan`); the block swap is
  the slave's ability in the blocked window (`Effect::SlaveTakesCombat`,
  `ActionCtx::slave`): the slave fights the blocker, the acting vampire
  unlocked, the block riders dropped.

### 6.11 Cards in play

`state.rs` `CardInPlay`; `cards/mod.rs`.

- Control: `CardInPlay.controller` is state; `GameState::card_controller`
  routes a minion card to its host's controller, a master card stays its
  player's (rules-qa 7.5), `Effect::StillYours` keeps a card its player's on
  another's minion. `Engine::give_card_control` / `give_location` (a location on
  a minion moves onto a ready minion of the new controller that can hold it, else
  burns).
- **Oust**: cards the Methuselah *controlled* are removed (not burned, nothing
  triggers, spec §1.10); other players' cards on their removed minions burn to
  their owners' ash heaps.
- Fields: `mode` (the version it entered with), `by` (the minion that played it),
  `against` (remembered Methuselahs, `SeatSet`), `chosen`, `stored` (library
  cards held on it, burned with it), `counters`, `entered`, `effect_only`.
- Riders (`Rider { hook, scope, requires, effects }`, `RiderScope::{Every,
  Controller, Others, Prey}`): requirements read at collection and at
  resolution (`rider_fires` again as a chosen rider resolves; an unmet rider
  is not pending, its window elides); unlock riders are pending mandatory
  effects ordered by their resolver (DESIGN §10.7); one burn per copy;
  `R::OneCopyPerTurn` drops the other copies once one resolves. An unlock
  rider paid unit by unit as it resolves (Smiling Jack: pool or a vampire's
  blood, per counter) runs through `Flow::RiderTerms`.
  `GameState::card_riders` lists every arena card's riders on a hook, in
  play or not, in arena order (one per hook, but for `Hook::UnlockPhase`).
  Every hook
  site collects through `Engine::fired_riders` and `Engine::printed_riders`
  (§2). `HookAt::seat` is the active Methuselah at a phase hook, the seat
  of an "own" event, none at a host-anchored hook; `Hook::reads_action`
  (`HostBlocks`, `HostBlocked`, `OwnBleedEnds` for cards;
  `ActionAnnouncedAtTarget` for vampires) reads requirements in the action's
  context, the card's rider predicates besides (`rider_card_holds`). A site
  applying effects one by one re-checks `present` before each;
  `leave_riders` keeps its pool batch (rules-qa 6.9). Left outside the
  collector on purpose: the resolution of a rider already chosen (draw,
  replacement, master/discard phase choice), Saatet-ta's destination as she
  burns, entry riders read on a crypt card before the vampire is in play,
  the leave-play readers (torture, hostages). The hooks and their tenants:
  the `Hook` enum's docs (`cards/mod.rs`); every hook has tenants
  (DESIGN §7.4).
- Entry: `enter_play` is the single entry point (contest checked by the
  caller after entry effects); `apply_entry_effects` (a retainer's layers on
  its employer, an ally's on itself); `push_mode_layers`. "Put this card on
  … / in play": `place_now` as the card resolves (the card itself; its
  layers from the mode's `Effect::Layer`s or pushed, `ModeLayers`; "only
  one" of its name, `holds_its_name`; a title card's title), `place_held`
  once it is in the ash heap (rules-qa 21.1, 47.1). "Only one" as the card
  is played is a mask from definition data (`ActorWithout`,
  `TargetWithout`, `OpponentWithout`); as it resolves, a legal no-op
  (rules-qa 17.4).
- A card that becomes another (`cards::became`): its windowless mode only.
  Living Manse is equipment to equip and a location in play
  (`cards::is_equipment_in_play`; out of `equipment_on`).

### 6.12 Printed vampire text

`cards/vampires.rs`: `VampireDef`, `Printed`.

- `crypt.rs` stays generated; behavior is `VampireDef` keyed by name. Plain stat
  modifiers are **not layers** (a vampire is not a card in play): read live by
  `GameState::printed_mod`, so nothing is pushed or retracted on torpor, contest,
  burn or oust. Printed text applies in torpor; a contested vampire gives
  nothing. Abilities are `ModeDef`s with actor `Host` = the vampire;
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
- "Plays cards requiring a <clan> as if" (`VampireDef::plays_as`) counts at
  card plays only (`GameState::meets_clan`: clan requirement, target, terms,
  `R::ControlsReadyClan`); a card's effects in play read the clan itself
  (`GameState::of_card_clan`).
- Vampire unlock-phase riders are pending mandatory effects
  (`Pending::Printed`, `Choice::ResolvePrinted`); `Stat::NoDirectedActions`
  filters the minion phase's offers (`Engine::directed_actor`); a combat an
  ability begins after resolution returns to that window
  (`CombatCtx::after_resolution`).
- `Stat::ActionCost` is any action's surcharge, read with its kind and
  directedness (Daria's directed actions, Marcellus's political ones;
  `Engine::action_toll`): masked at announce with the action's own blood
  (`announced_blood`), paid first in `succeed` (unpayable: fizzle).
  `Stat::StrikeCardToll` (Djeneba, `strike_card_toll` inside
  `combat_card_surcharge`): on the opposing combatant's strike-mode cards
  (`plays::is_strike_mode`), an ally paying life. Liliana's seven cards:
  `Effect::RemoveOwnAshCards` → `Interrupt::RemoveFromAsh` on her own heap
  (one pick a step; burned vampires, `Seat::ash_crypt`, count and are
  offered, as for Shroud of Decay).
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

### 6.13 Equipment

`plays.rs`, `actions.rs`.

- Equip from hand: +1 stealth plus printed, cost at resolution (unpayable or no
  longer holdable: fizzle), NRA by name. From another own minion
  (`EquipFrom`): free, no requirement check, each taken card's equip stealth
  added, moves what is still there at resolution. One vehicle etc. via
  `GameState::can_hold`. `Engine::put_equipment` is shared by equip, searches
  and Vast Wealth; "only one played or equipped in a game" is recorded there.
- Bearer effects are read live on the current bearer (`Modifier::Bearer`), with
  the named trait (a non-Ravnos holding Treasured Samadji gets nothing,
  rules-qa 9.1).

### 6.14 Replacement, hand size, draws, searches

`plays.rs`, `draws.rs`.

- Hand size is derived (`GameState::hand_size`: seven, `Seat::timed`'s
  `HandSize`, the hand-size layers of the uncontested cards in play the seat
  controls); cards not yet replaced count against it (`Seat.held`, §2 Held
  and Release; the longest delay wins, spec §1.9.2). Held-until conditions
  include events' ("until a Methuselah is ousted", "as long as this card is
  in play": waiting on the condition, not the card). "+n hand size until …"
  is a timed modifier (`Seat::timed`, swept where it ends). An oversize
  hand discards down before any other decision.
- Every card leaving hand that the rules replace goes through `Engine::replace`,
  offered first to `Hook::WouldReplace` riders, except the card being replaced
  itself ("a card other than this card"), then the hand drawn up; an empty
  library replaces nothing (and nothing burns instead, rules-qa 8.3). Visit
  from the Capuchin burns a counter instead of drawing, the hand size
  shrinking with it. A hand-size increase only draws up
  (`draw_up`). The Meddling of Semsith holds replacements made during a
  minion phase until it ends (`Release::MinionPhaseEnd`).
- Draw alternatives (`Hook::WouldDraw`): a choice per card drawn (rules-qa 9.4);
  beside Visit from the Capuchin a replacement is the Capuchin's counter or a
  stored card.
- Searches (`Flow::LibrarySearch`, `Interrupt::DisciplineSearch`, `SearchStore`,
  `SearchToHand`) pick by card, not position, at resolution (spec §1.14.1), and
  shuffle with the engine's `Rng`; an equipment search offers only what could
  be equipped as a play (no unpayable blood cost, rules-qa 9.3).
- Every library draw goes through `Engine::offer_redraw` (Blackhorse
  Tanner's `Interrupt::Redraw`, once each turn); `Interrupt::AllySearch`
  (Antonio d'Erlette) searches library and hand for a mage ally and puts
  it in play, not recruited. A crypt draw a card gives (Enthrall [OBF]) is
  offered after its resolution as `Choice::DrawCrypt`, none from an empty
  crypt.
- Extra draws on replacement: Learjet's (`Modifier::ExtraDrawOnReplace`,
  `Interrupt::MayDrawExtra` after the replacement, `Seat::learjet_held`
  for a delayed one, offered by `Engine::replace_held` at every release,
  carried with a replacement the Meddling holds again; rules-qa 73.1; an
  action card the bearer plays brings none, replaced before the action,
  73.2) and Infernal Pursuit's (`Seat::timed`'s `TimedStat::ExtraDraws`,
  ending with the combat; read in `Engine::replace`; several cards
  replaced at once go one at a time, `Interrupt::PursuitDraw` then
  `PursuitDiscard`, rules-qa 63.11).

### 6.15 Costs and reductions

`plays.rs`.

- Reductions are announced with the card (`Flow::CostTerms`,
  `PlayCtx::discounts`, `CostSource`; no pass while unaffordable without them),
  locked at declaration (not if canceled as played) or at resolution for
  non-action cards; paid through `reduced_cost_with` / `spend_counters` (counters
  spent only as a cost is paid). Found or named cards take their reductions once
  chosen (`CostThen`, `pay_named_discounts`, `payable_with_potential`).
- Surcharges: `card_surcharge` (Discipline and event surcharges),
  `discipline_surcharge` (Libertas), `combat_card_surcharge` (Terror Frenzy,
  Djeneba), `torpid_surcharge`, Haven's +1 on masters targeting its minion;
  Daria's on the action (`action_toll`). An ally combatant burned by paying
  its last life for a card ends the combat (`pay_play_blood`).
- Paying the last pool ousts: out of turn or from an ability, without passing the
  active turn (`pay_pool_for_ability`); a play whose effects already ousted its
  player does not oust again.
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

### 6.16 Events

`plays.rs`; `defs.rs` section "Events".

- A discard phase action, one per phase (`Seat::event_played`), each event once a
  game per player (`GameState::once_played`), controlled by its player.
  `Requirement::GehennaInPlay` is checked as played only (PIB 20121031);
  `Modifier::NoGehennaEvents` bars Gehenna plays; `Hook::GehennaPlayed`.
- Global layers read live: `capacity_ceiling` (Wormwood), `SectFallen`,
  `OnlyMustHunt`, `torpid_surcharge`. Timed effects: The Uncoiling's burn due in
  its controller's discard phase (`Engine::discard_dues`, read live; one
  played in a discard phase is due in it, rules-qa 57.5), Thirst as the
  minion phase ends, FBI's damage as the combat ends (`CombatCtx::fbi_damage`).

### 6.17 Ousts and Life Boon

`ousting.rs`: `Ousting`; slice 16b, rules-qa Batch 45.

- `oust_batch` defers when a Methuselah other than the active one and outside
  the batch holds a Life Boon (`defer_oust`): the batch is marked
  `Seat::oust_pending`, the caller goes on as if the pool had not run out
  (pools below 0 kept: what the Boon must make good), and `Engine::ousting`
  is served after the interrupts, before the flow: each holder, clockwise from
  the active one, passes or plays the Boon through `announce_play` (terms
  `Terms::BoonPool`, as-played window, cancels), `playing` while the play is
  in flight. `settle_ousting` then drops the saved and ousts the rest
  (`oust_now`), settling what the oust interrupts (`after_deferred_oust`:
  `combat_ousts`, the turn passing, the action's order). With no Boon in an
  eligible hand the oust applies at once, as before.
- `oust_and_pass` returns whether the oust applied: its callers return (the
  turn passed) only then, else go on as if the pool had not run out.
- Victory points go through `Engine::gain_half_vps` (prey ousts, last
  standing, the time limit's half point): a Life Boon in play remembering the
  Methuselah (`CardInPlay::against`) takes them, oldest first, its counters
  the half points given, burned at 1 VP. `Effect::StealPoolFromBoonTarget`
  (`Requirement::BoonTargetActive`, `Window::UnlockPhaseAny`).

### 6.18 Generated data

`scripts/gen_crypt.py` → `crypt.rs`, `scripts/gen_library.py` →
`library.rs`, both `--check`.

- Generated labels read from card text/type line: `Weapon` (incl. "Melee
  weapon."), `Electronic`, `Location` (type line only), `Discipline`, `Grapple`,
  `Aim`, `Title`, `RequiresAnarch`, `MakesAnarch`, `OperationAntigen`,
  `MultiDiscipline`; "Unique." after the first sentence; ally body (life,
  strength, bleed) and subtypes. `Title::Independent { name, votes }`. Printed
  justicars keep the clan's old name.
- Pool order: the V5 cards (and `EXTRA_CARDS`) sorted by name, then the
  `ADDED_CARDS` batch after batch (`ADDED_BATCHES`), by KRCG id within a
  batch (`window_matrix.pool_order`): an addition never moves an existing
  index. Titles: archbishop (a city's, 2 votes) and Inner Circle (one per
  clan beside the justicar, 4 votes); the Sabbat's bishop (1 vote),
  cardinal (3), priscus (none of its own, a ballot in the prisci block's 3
  votes: `VoteRecord::prisci`) and regent (4, the only unique one), each
  only while Sabbat. Paths: `CryptCard::path`, `LibraryCard::path` (a path
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
  Anarch Convert not counted), TWDA order; `Deck::crypt_share`,
  `Deck::clan` (most crypt copies); regenerated after a pool change, it
  picks up the decks the additions complete.
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
  triggered by the chosen opposing strike (Rötschreck, with
  `CombatCtx::aggravated_strike_on`). Rötschreck ends combat at once
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
  HostInterceptVsVampires` and `Minion::diablerie_turn` against
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
    (a Methuselah knows their decklist and sees every card leave or enter).
  - Hands, but an open one (Owl Companion [ani], Harold Tanner, Revelations
    [AUS]: `PreyOpenHand`, `OpponentOpenHandInCombat`, a seat's
    `combat_open_hand`), read through `GameState::hand_visible_to`.
  - Public: ash heaps (`ash`, `ash_crypt`), removed cards, minions and cards
    in play (a contested card turned face down keeps its identity), pool,
    blood, counters, `held` (a count); a banished vampire's uncontrolled
    slot (it left play in view of all), every slot's counters; face-up
    stored cards (Black Market Cache, Fleshforge Chamber), Shilmulo Tarot's
    face-down ones (`Effect::StoreTopCards`) to its controller only. The
    RNG is not in the state.
  - The action, combat and referendum contexts and the as-played stack are
    public, except a card named from hand (`PlayCtx::named`, Pack Alpha;
    `ActionCtx::exchange`'s hand card, Lenelle; `ActionCtx::card`):
    announced but never shown, so an observation or a bot never reads it,
    and an observation adding these contexts must redact it.
- A new `Choice` kind or a new option shape needs its observation slot
  (`obs.rs`: option ids and `OPT_F` features); a new rulebook-like choice
  usually shares an existing kind's slot (`Choice::UnlockInfernal` in
  `UnlockForBlood`'s, `BurnBloodToAvoid`'s for the Society of Leopold).
  Vocabulary keys (`obs::vocab_keys`) keep trained networks across pool
  extensions.
- `game::public_choice` is the whitelist of choices recorded as public
  history (`Engine::history`): a choice that reveals hidden information
  stays out of it.
- `tests/obs.rs` (the encoding of the full state equals that of the
  seat's legal view; every pointer names a token) and `tests/bots.rs` (bot
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
  never take part in the game's identity: `Pile` compares and hashes as its
  card ids alone, `Untracked` fields not at all, so the state hash,
  observations and options do not see them.
- `census` lists every card with its copy and place; the fuzz tests check
  each dealt copy stays its card and is found exactly once, until its
  Methuselah is ousted (their cards are removed from every zone).
