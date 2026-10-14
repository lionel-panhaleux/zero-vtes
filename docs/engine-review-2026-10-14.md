# zero-vtes engine review: duplication, ad-hoc mechanisms, clarity

Reviewed: committed `main` at `fae8816`, in a detached worktree (file:line
references are to that tree; the paths are the same as in the repo).
Scope: `crates/engine/src` (the generated `crypt.rs`, `library.rs`, `twda.rs`
only as inputs), plus the `host` crate's effect mapping where it matters.
Method: read DESIGN §4 and §11, the vocabulary in `cards/mod.rs`, the effect
interpreters, rider hook sites, ability enumeration and resolution, and the
diff of round 10 (991b540). I counted patterns with greps and short scripts,
and ran one scratch test in the worktree to check the one behaviour claim
(finding 7).

## 1. Verdict

The design itself is sound and the code follows it. One representation per
card (`CardDef`/`VampireDef` with `ModeDef`, `Effect`, `Requirement`,
`Rider`). Live reads everywhere. Card-name literals in engine code are nearly
absent: three sites (`actions.rs:668` Mithraic, `influence.rs:375` Anarch
Convert, `ousting.rs:44` Life Boon), each with a reason. A grep for
`("Capitalized…")` literals in the engine files also finds a few
card-printed *clan* names (F12). Rulings sit on the
variants that implement them. Card-specific `Effect` variants (Ashur Tablets,
Kaymakli Nightmares, Sudario Refraction…) are the "custom code behind the
same interface" that DESIGN §4.5 allows, so I don't count them as debt.

The drift is in the **dispatch layer**, not the vocabulary. Effects, riders,
requirements and ability gates are interpreted by many hand-written `match`
blocks that end in `_ => true` or `other => unreachable!(…)`, so the compiler
can't tell when a variant is missing from one of them:

- 108 `unreachable!` sites;
- ~35 `=> true // engine-checked` arms in the `holds` family;
- two hand-kept classifier lists (`is_combat_effect`, `is_vote_effect`);
- about 25 rider hook sites, each with its own collection loop and its own
  small interpreter.

As a result, the same effect has been written twice in several places, and
some of those copies now differ a little. Adding a card means remembering N
places. I found one likely behaviour defect (finding 7) and a few latent
divergences. Everything else here is factoring and safety nets, not bugs.

**Cross-cutting constraint for every proposal below.** A factorization is a
refactor only if `tests/golden` and the perft counts stay green *without*
`GOLDEN_BLESS`. A changed golden means a changed trajectory. Also, the
`Choice` variants and their order are the trained policy's action space, and
the `host`/`text` crates read them. Proposals that would change `Choice`
values or their order (noted where relevant) carry that extra cost.

## 2. Findings, ranked by value

### F1 (do): effect routing is hand-maintained, so a missing route only shows at run time

**Pattern.**

- `Engine::apply_card_effect` (`plays.rs:3425`) has a ~60-variant exclusion
  arm ending in `unreachable!("ability or rider effect in a play")`
  (`plays.rs:4585-4630`).
- It then routes by `e if is_combat_effect(&e)` (`combat.rs:3853`),
  `e if is_vote_effect(&e)` (`referendum.rs:2312`), and finally
  `apply_action_effect` (`actions.rs:3090`, which ends
  `unreachable!("not an action effect")` at `actions.rs:3531`).
- `resolve_ability` (`plays.rs:6455`) is a fifth interpreter. It routes the
  same two classifiers (`plays.rs:7553-7559`) but delegates to
  `apply_action_effect` only through a hand-picked list (`plays.rs:6933-6943`).

So adding an `Effect` variant means updating 2-4 lists the compiler doesn't
check: the right classifier, the right interpreter, the exclusion list, and
`host/src/effects.rs` (below). 45 variants are matched in both
`apply_card_effect` and `resolve_ability` (e.g. `GainPool` `plays.rs:3637`
vs `6559`, `LockTarget` `3645` vs `6985`, `PreyBurnsPool` `3717` vs `7252`
vs the unlock-rider batch at `7972`).

**Proposal.** One exhaustive classifier: `impl Effect { pub fn family(self)
-> Family }` in `cards/mod.rs`, with `Family::{Play, Action, Combat, Vote,
AbilityCost, Rider, Marker}` and no `_` arm. A new variant then fails to
compile until someone classifies it. `apply_card_effect` and
`resolve_ability` dispatch on `family()`, and the two `is_*_effect` lists
become `family() == …`. Optionally a unit test asserts that every effect in
`DEFS`/`VAMPIRES` sits in a slot its family allows (a mode, an ability, a
rider).

**Preserves behaviour.** Pure routing. Every arm keeps its body; only the
choice of interpreter becomes compiler-checked. Goldens and perft must stay
green unblessed.

**Risk.** Low. Variants routed differently by plays and abilities today
(below) need a deliberate decision, not a mechanical move.

**Size.** `cards/mod.rs` (+1 function, ~520 arms, mechanical), `plays.rs`,
`combat.rs`, `referendum.rs`. One afternoon.

**Divergences between the two interpreters, to settle while doing it:**

- **`CircleBloodMoves` carries a hidden parameter.** Played, it passes
  `equipment: true` (`plays.rs:3507-3514`, Redistribution: "blood and
  equipment"). As an ability it passes `equipment: false`
  (`plays.rs:7260-7267`, Jerry: "blood"). Both are correct for today's two
  cards, but the parameter comes from which interpreter runs the effect, not
  from the card. Make it `CircleBloodMoves { equipment: bool }` in the
  definitions.
- **`UnlockTarget`.** Played, it unlocks only a `ready()` target
  (`plays.rs:3529-3535`, Hive Mind). As an ability it unlocks any minion
  target (`plays.rs:6956-6960`, Alabástrom, Zizi, Ambrosio, Underbridge
  Stray, Mylan Horseed, Metro Underground). Abilities may re-check targets
  at resolution elsewhere. I did not prove a behaviour difference, but the
  two copies should agree or say why they differ.
- **Copies that could delegate.** `AddIntercept` (`plays.rs:6906` vs
  `actions.rs:3103`), `AttemptingIntercept` (`plays.rs:6916` vs
  `actions.rs:3316`) and `ContinueUnblocked` (`plays.rs:7030` vs
  `actions.rs:3133`) are re-implemented in `resolve_ability` instead of
  delegated. The only real difference is the guard: the ability copy no-ops
  outside an action (`if let Some(a) = self.action`), the action copy
  `expect`s one. Delegating under that guard is equivalent.

### F2 (do): rider hooks are fired by about 25 bespoke loops, and some ignore the rider's own data

**Pattern.** Each hook has its own function that loops
`state.card_riders(hook)` (`state.rs:2264`), checks `present()`, applies a
scope, and interprets `r.effects` with a private `match … unreachable!`.
Examples: `host_announces_riders` `actions.rs:1156`, `host_blocked_riders`
`3711`, `about_to_bleed_riders` `3890`, `host_blocks_riders` `3921`,
`successful_hunt_riders` `5412`, `minion_phase_end_riders` `game.rs:1815`,
`contract_riders` `draws.rs:2025`, `political_passed_riders`
`referendum.rs:2006`, `leave_riders` `plays.rs:8410`, `own_bleed_riders`
`plays.rs:8617`, plus the unlock and discard resolvers `plays.rs:7911`,
`8277`. The handling differs per site:

- **`Rider.requires`.** Read through `rider_holds` or inline at some sites
  (`leave_riders`, `about_to_bleed_riders`, `host_blocked_riders`), not read
  at all at others (`host_announces_riders`, `successful_hunt_riders`,
  `minion_phase_end_riders`, `contract_riders`, `own_bleed_riders`, the
  referendum ones). I checked every rider on those hooks: all have
  `requires: &[]` today. So this is a hazard, not a defect: the first rider
  with a requirement on one of those hooks fires unconditionally, against
  DESIGN §11.2 ("requirements read at collection and at resolution").
- **`Rider.scope`.** `RiderScope::applies` is called at 7 sites. Elsewhere
  the scope is hard-coded by the hook's name (`own_bleed_riders` enforces
  `card_controller == seat`; the "Own…" hooks generally). Today every
  card rider on those hooks says `RiderScope::Controller`, so data and code
  agree, but only by convention.
- **`Rider.effects` is decorative at some hooks.** The site hard-codes the
  effect:
  - `dead_pool_counters` (`plays.rs:8505`) adds one counter per matching
    *hook* and never reads Dead Pool's `effects: &[Effect::AddCounter(1)]`
    (`defs.rs:12250`). An author writing `AddCounter(2)` would get 1.
  - `flames_offers` (`plays.rs:8540`) and `torture_offers` (`plays.rs:8577`)
    implement Flames of Insurrection and The Path of Lilith entirely in
    code; their riders carry `effects: &[]` (`defs.rs:21339`, `18735`).
  - `by_left_ready` (`plays.rs:5913`) and `end_turn`'s `Hook::EndOfTurn`
    loop (`game.rs:1970`) burn the card whatever the effects say.

**Proposal.** Two layers.

1. **(do) One collector.**
   `fn fired_riders(&self, hook, at: HookCtx) -> Vec<(CardId, &'static Rider)>`
   applying `present()`, `not text_suppressed`, `scope.applies(controller,
   at.active)`, host/subject matching (`HookCtx { subject, host_must_be,
   active }`) and `rider_holds`, used by every site. The sites keep their
   own effect handling for now. Where a hook implies its effect (the five
   above), either read `r.effects` there, or add a unit test asserting those
   hooks' riders carry exactly the effect the site performs, so data and
   code can't drift apart silently.
2. **(consider) One rider interpreter.**
   `apply_rider_effect(e, &RiderCtx { card, subject, opponent, seat })` for
   the effects that recur across hooks: `BurnSelf`, `AddCounter`,
   `TheirControllerBurnsPool`, `PreyBurnsPool`, `HostBurnBlood`. Hook-only
   effects stay at the site.

**Preserves behaviour.** `leave_riders`' pool batching (`loss` vector,
`burn_pool_batch`) must stay as it is: simultaneous losses, rules-qa 6.4,
6.9. So must the canonical ordering where unlock riders are pending
mandatory effects ordered by their resolver (§10.7). The collector changes
neither; it only unifies the filter. Behaviour is identical while every
`requires` is empty and every scope matches the hook.

**Risk.** Low for the collector. Medium for the interpreter (ordering inside
a batch).

**Size.** ~15 functions across `actions.rs`, `plays.rs`, `game.rs`,
`draws.rs`, `referendum.rs`.

### F3 (do): exact twin variants

- **Same effect, two variants.** `Effect::BurnActorBlood(u8)`
  (`cards/mod.rs:4724-4726`, applied `actions.rs:3353`) and
  `Effect::ActorBurnsBlood(u8)` (`cards/mod.rs:3518-3520`, applied
  `combat.rs:3563`). Both bodies are `burn_blood(actor, n)`, and both docs
  cite ANK 20210226 ("an effect, not a cost"). They exist twice only
  because routing is by domain (F1). Merge into one variant handled before
  the domain dispatch. Cards: Second Tradition: Domain, Deep Ecology,
  Stygian Shroud, Go-getter (`defs.rs:5713`, `5753`, `5948`, `10347`) /
  Rolling with the Punches, Anticipation, Charismatic Aura (`defs.rs:6915`,
  `10000`, `14220`). The `host` mapping has
  neither, so nothing changes there.
- **Effects that are `ActionLayer` under another name.** `AlliesIntercept(n)`
  (`actions.rs:3194`), `MinionsIntercept(n)` (`actions.rs:3261`) and
  `CorruptedIntercept(n)` (`actions.rs:3254`) only push a `Scope::Action`
  layer with `source: CardId(u32::MAX)`, which is exactly what
  `Effect::ActionLayer(modifier)` does (`actions.rs:3520-3526`). So they
  are `ActionLayer(Modifier::InterceptMod { younger_than: 0, n })`,
  `ActionLayer(Modifier::InterceptMod { younger_than: u8::MAX, n })` and
  `ActionLayer(Modifier::CorruptedInterceptMod(n))`. Cards: Obedient Flesh
  (`defs.rs:8339`), Unthinkable Humiliation (`defs.rs:14920`), Sense the Sin
  (`defs.rs:17671`). `YoungerIntercept` (`actions.rs:3271`) reads the
  actor's capacity at play, so it is not a plain constant layer. Leave it.
  **Preserves behaviour:** same layer pushed (`layer_active` is true for
  `Scope::Action` whatever the source, `state.rs:3114`). No other code
  reads these three variants: a grep over the engine, `host` and `text`
  finds only their interpreter arms (`actions.rs:3194`, `3254`, `3261`).
  So no reach computation (`intercept_reachable`, DESIGN §11.2) depends on
  the variant names; it reads the layers. Goldens should not move. **Size:** 3 variants, 3 definitions, one interpreter.
- **Looks alike, keep apart: `UnlockActor` vs `UnlockActing`.** The actor
  playing or using a card need not be the acting minion. Yoruba Shrine is a
  location whose ability unlocks the acting minion, while Hive Mind unlocks
  its own actor (and `apply_card_effect` treats `UnlockActor` specially
  outside an action, `plays.rs:3475`).
- **Looks alike, keep apart: `IfBleedSuccessful(&[PutOnActor])` vs
  `PutOnActorIfBleedSuccessful`.** The first applies "as it resolves, after
  the pool is burned". The second puts the card "after action resolution"
  (Fever Pitch), one per vampire. Same for `CryptForBloodIfBleedSuccessful`
  and `TzimisceDiscardUnlockIfBleedSuccessful`, which are optional choices
  later on.

### F4 (do): `Held` is a field per delay, listed in five places

**Pattern.** `state.rs:805-886`: 13 `u8` fields (`unlock`, `discard`,
`action`, …, `minion_phase`), each also listed in the `add_diff!` macro
(`state.rs:849-863`) and in `total()` (`state.rs:866-885`). Each is mapped
from `Until` by hand (`plays.rs:3319-3331`) and released at its own site
(`release_held(|h| &mut h.x)`). `Held` and `Until` (`cards/mod.rs:4987`)
are parallel but not equal: `weakens`, `master` and `minion_phase` have no
`Until`, and `Until::LeavesPlay` maps to `Seat::held_in_play`. Round 10
added `minion_phase` by touching four of these places.

**Proposal.** One enum of release moments (extend `Until`, or a
`HeldUntil`), with `Held([u8; N])` indexed by it. `add_diff` and `total`
become loops, the plays.rs match becomes `held[until] += 1`, and the
release sites become `release_held(HeldUntil::MinionPhase)`.

**Preserves behaviour.** Same counters, same release points. The derived
`Hash`/`Debug` change shape, which may show in `{:#?}` state dumps but not
in trajectories. Check that goldens hash or print no state (the round-10
re-bless note says "state fields only: same trajectories", so goldens may
include state fields and may need a re-bless whose diff should show no
decision change).

**Risk.** Low. **Size:** `state.rs`, `plays.rs`, `game.rs`, `draws.rs`,
`combat.rs`. About 20 sites.

### F5 (consider): timed seat modifiers, a field + reset + read per duration

**Pattern.** Hand size has six fields: `Seat::{hand_bonus,
hand_bonus_discard, hand_bonus_game, hand_bonus_unlock, hand_malus_unlock,
combat_hand_bonus}` (`state.rs:924-941`). They are summed in `hand_size`
(`state.rs:2146-2151`), set at `plays.rs:6793`, `8635`, `7082`,
`referendum.rs:1720`, `2066` and `combat.rs:3645`, and reset at
`game.rs:1954`, `1995`, `2257`, `2260` and `combat.rs:3105`. The same shape
recurs for `hunt_stealth_unlock` (`actions.rs:497`, `game.rs:2259`),
`combat_extra_draws`, `combat_open_hand`, `transfers_next`, `mpa_next` and
`veil_left`. Every "+n X until <moment>" card adds a field, a write, a read
and a reset.

**Proposal.** `GameState.timed: Vec<Timed { seat, stat: SeatStat, n: i8,
until: Expiry }>`, swept at the same expiry points that `Held` uses (F4
gives the expiry enum). `hand_size` reads `timed.iter().filter(seat,
HandSize)`.

**Preserves behaviour** only if every expiry is swept at exactly the line
where the field is reset today. Example: Fotini's bonus ends "as the
discard phase begins" (`game.rs:1954`), the end-of-turn bonus at the end
of the discard phase (LSJ 20100812, `game.rs:1995`).

**Risk.** Medium. The resets sit among other order-sensitive bookkeeping in
`start_turn`/`end_turn`. **Size:** ~10 fields, ~25 sites. Worth it mainly
if more "until …" cards are coming. Doing only the hand-size subset first
is cheap.

### F6 (consider): ability enumeration rebuilds its checks per actor branch

**Pattern.** `ability_options` (`plays.rs:4884-5392`) has one branch per
`Actor`, and each branch hand-assembles a different subset of the same
gates:

- **Limits.** `limit_spent` in full (Controller, Host, ActingCandidate),
  only `PerTurn`/`PerAction` inlined (AnyReadyVampire,
  `plays.rs:5040-5051`), only `PerAction` (ActingMinion,
  `plays.rs:4999-5004`), none (Methuselah, ChosenController). Some limits
  are pre-checked above the match (`plays.rs:4903-4911`).
- **`YouControlThisCard`.** Checked by hand in 4 branches.
- **Requirements.** Evaluated with `holds_for_card + holds_live` (Controller,
  Methuselah, AnyReadyVampire), `cards::holds` (ActingMinion,
  ActingCandidate), the Host combination, or **not at all**
  (ChosenController, `plays.rs:5082-5096`, which reads only
  `CardControllersTurn`).
- **Per-effect affordability lists.** The ally blood-cost bar names four
  effects by hand (`plays.rs:5181-5193`); the Methuselah branch has its own
  check for `PayPoolPerOtherEvent` (`plays.rs:5110`).

`vampire_ability_options` (`plays.rs:5628-5765`) repeats the Host branch
with a vampire source. Actor resolution exists three times, and the copies
disagree:

- ability enumeration;
- `resolve_ability`'s header (`plays.rs:6464-6503`);
- `ability_user` (`plays.rs:6182`).

`resolve_ability` gives `Actor::AnyReadyVampire` to the *using vampire's*
owner; `ability_user` gives it to the *card's controller*. Carver's Meat
Packing (`defs.rs:9559`: any Methuselah's vampire, no
`YouControlThisCard`) has no terms or discards, so nothing reads
`ability_user` for it today. The first such ability with terms would ask
the wrong seat.

The same goes for the limit ledger. `resolve_ability` marks uses
(`plays.rs:6515-6538`) and `limit_spent` checks them (`plays.rs:5392-5418`).
They are two hand-paired matches over `Limit`, and a third marker for
granted actions sits at `plays.rs:6020`.

**Proposal.**

- `fn ability_user(&self, source, ai, user: Option<MinionId>) -> (SeatId,
  Option<MinionId> /*host*/, Option<MinionId> /*combatant*/)`, used by the
  enumeration, `resolve_ability` and the terms requests.
- `fn ability_gate(&self, seat, source, ai, mode, host, combatant) -> bool`
  holding the gates every branch needs: the full `limit_spent`, phase use,
  `YouControlThisCard`/`OthersControlThisCard`, all requirement evaluators,
  `cb.allows`, `may_take_granted`.
- `fn limit_slot(&mut self, limit) -> Option<(&mut Vec<_>, key)>`, read by
  both the marker and the checker.

The branches then only compute candidate users.

**Preserves behaviour** only after an audit, branch by branch: where a
branch skips a check today, either the check is vacuous for its cards (then
unifying is neutral) or the skip is deliberate. I found no deliberate one,
but did not prove each. Examples: Kevlar Vest's allowance exception
(rules-qa 10.3) and the `Actor::ActingCandidate` special
`r != CardUnlocked || holds_for_card` (`plays.rs:5378-5381`) must survive.

**Risk.** Medium-high (the most-used legality code). Goldens and perft are
the guard. **Size:** `plays.rs` ~700 lines touched.

### F7 (do, likely defect, owner to rule): "is this action a hunt" is answered three ways

**Pattern.**

- `cards::holds` reads `IsHunt` as `a.kind == Hunt || a.is_hunt`
  (`cards/mod.rs:5650`, also `5811`). `is_hunt` is set by `HuntAction`:
  Psychophagia, Abactor, Loki's Gift, Week of Nightmares (`actions.rs:1101`,
  `plays.rs:6048`), "a hunt for everything reading one" (LSJ 20050720-2,
  doc at `cards/mod.rs:2925-2928`).
- `printed_holds` reads `R::IsHunt => ctx.kind == Some(ActionKind::Hunt)`
  (`state.rs:1840`), and `ActionCtx::printed_ctx` passes `kind: Some(self.kind)`
  only (`actions.rs:823-838`).
- Sunset Strip's hunt stealth reads `self.kind == ActionKind::Hunt`
  (`actions.rs:496`).

**Check.** I ran a scratch test in the worktree. Volo ("-1 stealth when
hunting", `vampires.rs:4683`) hunting as normal has stealth 0, and a 0
intercept blocker is offered the block. Volo playing Loki's Gift [pro] ("+1
stealth hunt action") resolves with no block offered to two 0 intercept
blockers, so its stealth stayed at 1: Volo's -1 was not applied. The same
reading affects Kalyani, Abiku and Le Dinh Tho (`vampires.rs:2544`, `4048`,
`4491`), and Sunset Strip's bonus on a Week of Nightmares or Abactor hunt.

**Rules nuance (do not fix blindly).** Zöe "does not get the usual +1
stealth when hunting" is modeled as `own(Stat::Stealth, -1, &[R::IsHunt])`
(`vampires.rs:5019`). On a card hunt there is no "usual +1" (the card's
stealth is printed), so today's narrow reading is *right* for Zöe and wrong
for the "±1 stealth when hunting" vampires. Proposal:

- one `ActionCtx::is_hunt()` used by `holds`, `printed_holds` (via a
  `PrintedCtx.hunt: bool`) and Sunset Strip;
- Zöe re-expressed as a requirement on the rulebook hunt (`R::RulebookHunt`,
  i.e. `kind == Hunt`), or as removing the base +1.

This is a reading to record in a new rules-qa batch (Volo with Loki's Gift;
Zöe with Loki's Gift; Sunset Strip with Abactor).

**Size.** `state.rs`, `actions.rs`, one or two `cards/mod.rs` arms, 1
vampire definition, ledger rows, 2-3 tests.

**General lesson.** `printed_holds` (`state.rs:1801-1985`) re-implements
~48 `Requirement`s that the `holds` family also implements (`ActorSectIn`,
`IsBleed`, `ControlsLocation`, `PoolAtMost`, …). The rest I sampled agree
except in context plumbing: `ActorSectIn` in `holds` treats an ally playing
as a vampire as Independent (`cards/mod.rs:449-452`), `printed_holds`
doesn't, which is fine since printed text is on vampires. **Consider** one
predicate core taking a small context struct, so a fix to a predicate lands
once.

### F8 (consider): cost effects carry a hand-written affordability requirement

**Pattern.**

- Every in-play ability with `Effect::LockSelf` also lists
  `Requirement::CardUnlocked`: 83 of 83 modes.
- `LockHost` is paired with an unlocked check 8 of 8 times.
- `HostBurnBlood(n)` sits beside a matching `*BloodAtLeast(n)` in 33 of 39
  modes. The 6 without are riders (mandatory, burn what there is), which is
  correct.
- `PayPool` is uneven: Danylo has `PoolAtLeast(1)` (`vampires.rs:4640`),
  Ferraille has none (`defs.rs:11671-11685`). Both are vacuous for a seat
  still in the game, which is consistent with "costs payable even if the
  payer burns" and the last-pool oust.

The mask therefore depends on the author remembering the pairing, while the
cost is already in the effect list. That is the "hand-written checks that
duplicate what the definition already says" pattern.

**Proposal.** `fn implied_requirements(effects) -> impl Iterator<Item =
Requirement>` in `cards/mod.rs` (`LockSelf → CardUnlocked`,
`LockHost → HostUnlocked`, `HostBurnBlood(n) → HostBloodAtLeast(n)` for
abilities only, never for riders). The ability gate evaluates it alongside
`mode.requires`. Then either delete the redundant pairs, or keep them and
add a test asserting they are present (the cheaper first step: a lint, not
a refactor).

**Preserves behaviour.** Exactly, for the 83/8/33 cases. Before deriving, a
card where locking is an effect rather than a cost (a legal no-op, rules-qa
17.4) would need `LockSelf` kept apart. I found none (Bleeding the Vine
uses its own `LockOrBurnPool`).

**Risk.** Low as a lint, low-medium as derivation. **Size:** 1 function +
a test; optional cleanup of ~120 definition lines.

### F9 (consider): "engine-checked" requirements pass as `true`

**Pattern.** The `holds` family (`cards/mod.rs:5587-6980`) has ~35 arms
that return `true` with a comment (`// engine-checked`, `// card
predicate`, `// host predicate`, `// target predicate`), plus several
`_ => true` defaults. The real checks live elsewhere: `OwnMinionPhase`,
`OthersMinionPhase` and `InMasterPhase` at `plays.rs:4913-4918` and
`5650`; `YouControlThisCard` in four `ability_options` branches;
`ActingIsHost`/`ActingIsBy` at `plays.rs:5007-5010`;
`TargetCapacityBelowCounters` at `plays.rs:4972`. A requirement placed in a
slot whose evaluator passes it (e.g. a host predicate in a `ModeDef.requires`
read only by `holds`) makes the mask *permissive*, silently. No panic,
since these are `true`, not `unreachable!`.

**Proposal.** `impl Requirement { fn slot(self) -> Slot }` (exhaustive:
`Play`, `Host`, `Card`, `MinionTarget`, `SeatTarget`, `CardTarget`,
`Printed`, `Engine(&'static str)`), plus a unit test walking every `DEFS`
and `VAMPIRES` mode, ability, rider and target filter, asserting each
requirement is in a slot that some evaluator really reads for that position
(e.g. `Engine("ability_options")` only in `abilities`). Zero behaviour
change; it turns today's convention into a check.

**Size.** `cards/mod.rs` (+1 exhaustive function, 375 arms) + 1 test.

### F10 (consider): the "put this card on…" family

**Pattern.** 14 placement variants: `PutOnTarget`, `PutInPlay`,
`PutOnActor`, `PutOnActorRemembering`, `PutOnActorNow`,
`PutOnActorNowStacking`, `PutOnOpponent`, `PutOnOpponentOnHit`,
`PutOnActorAfterCombat`, `PutOnActorIfNoBlock`, `PutOnActingIfBlocks`,
`PutOnActorIfBleedSuccessful`, `PutOnActorIfOpposingNotReadyAfterCombat`,
`PutOnOpposingKeepControl`. They run over two primitives:

- `enter_play` at resolution, `plays.rs:3944-4039`. `PutOnActorNow` calls
  `push_mode_layers` itself; the `PutOnTarget`/`PutOnActor` arm does not
  (layers pushed later in `resolve_play`), so the same invariant is
  maintained in two ways.
- `place_held` after the card is in the ash heap, `combat.rs:2676`, with
  `effect_only`, rules-qa 21.1.

"One per vampire" is checked in three ways: `holds_a(Named)` in
`PutOnActorNow`, `may_put_held`, and `ActorWithout` requirements on the
mode. CLAUDE.md names this family "a recurring miss".

**Proposal.** Keep the two primitives; they differ for a ruled reason
(21.1). Factor the immediate path into one
`fn place_now(&mut self, ctx, on: Option<MinionId>, opts: PlaceOpts {
stacking, remember, counters })` returning the `CardId` and always pushing
the mode's layers. A full `PutOn { on, when }` parameterization of the
definitions is possible but changes ~40 definitions and the `host` mapping.
Only worth it if more placements come.

**Risk.** Medium (when layers are pushed matters for effects resolving in
the same play).

### F11 (consider / leave): the `Interrupt` pay-or-suffer family

`PayOrBurn`, `BloodOrBurned`, `LockOrPay`, `LockOrDiscard`,
`PoolOrVampire`, `PoolForCard`, `LockForPool`, `MayGainPool`,
`MayAddCounter`, `MayTorture` (`draws.rs`, the `Interrupt` enum spans
`draws.rs:~330-450`) are "do A or suffer B" / "may pay C for G" choices,
each with its own request/apply pair. A generic `Interrupt::Either { seat,
pay: Outcome, or: Outcome }` could replace several. The details differ in
rules-relevant ways: Society of Leopold's "burned when it has fewer"
(all-or-nothing), Bleeding the Vine's lock as a legal no-op on a locked
card, Nightmares upon Nightmares' discard not replaced until the master
phase. The offered `Choice`s also differ (`ResolveRider`, `BurnPool`,
`ChooseMinion`), and unifying them would change the action encoding the
policy and the host see. **Leave**, unless a new batch brings several more
of these; then factor while keeping the `Choice` values.

### F12 (do, small): clarity nits found on the way

- **Misplaced doc comment.** `cards/mod.rs:4321-4324`: the Righteous Blade
  doc ("Rider of a weapon's strike … 1 optional press only usable to
  continue combat …") sits on `DiscardAtRandom`, and
  `ContinuePressEachCombat` (`cards/mod.rs:4330`) has no doc. Move it down
  two variants.
- **Misplaced comment.** `combat.rs:3545-3546`: "Wall of Filth: on the
  vampire now, 'a vampire can have only one'" sits above `AimExtraDamage`.
  It belongs to the `PutOnActorNow` no-op arm a few lines below
  (`combat.rs:3551`).
- **Card-printed clan names in engine code** (CLAUDE.md: card-printed names
  live in `cards/defs.rs`).
  - Fiendish Tongue's "this Tzimisce" is hard-coded twice: in the effect's
    name `TzimisceDiscardUnlockIfBleedSuccessful(u8)` and as
    `clan() == Some("Tzimisce")` (`actions.rs:4228`, `plays.rs:4716`).
  - Baltimore Purge's "if you control a ready Lasombra" is
    `clan() == Some("Lasombra")` inside `start_purge` (`draws.rs:2203`).

  Sibling effects already take the clan from the definition
  (`UnlockActingIfSuccessful(&'static str)`, `OwnClanBurnsBlood(&str)`), so:
  `DiscardUnlockIfBleedSuccessful { clan, blood }` and
  `Purge { chooser_clan }`. Behaviour-neutral. The `"Blood Brother"` checks
  (`plays.rs:2236`, `3460`, `state.rs:3008`, `draws.rs:3299`) are the
  circle mechanic itself (only Blood Brothers have circles), so leave them.
- **Cross-crate silent drop.** `host/src/effects.rs:229` ends in `_ => {}`:
  an effect not mapped there (about 106 of the ~520 are mapped) is
  invisible to clients. That's fine as policy ("effects no option told are
  left out"), but it means a new player-visible effect needs a host line
  too. With F1's `family()`, a `Family::Marker` vs everything else split
  would show which unmapped effects are bookkeeping.
- **Effects used as markers.** About 36 effects are no-ops in
  `apply_card_effect` and read elsewhere by scanning effect lists
  (`effects.contains(&Effect::X)`, about 70 scans in `plays.rs`,
  `actions.rs`, `combat.rs`, `game.rs`). This is a legitimate design, but a
  `Family::Marker` (F1) would make "this does nothing here, it's read at X"
  explicit instead of an `=> {}` arm with a comment.

## 3. Recurring hazards when adding cards (today, by hand)

1. **New `Effect` variant.** Add it to the right interpreter *and* the right
   classifier (`is_combat_effect`/`is_vote_effect`), or to
   `apply_card_effect`'s exclusion arm. If it can be both played and used
   as an ability, it needs both `apply_card_effect` and `resolve_ability`,
   or a delegation. A miss panics only when a game reaches it (F1).
2. **Effect used at a new hook.** That hook's private match must learn it,
   or `unreachable!("not a … rider effect")` fires (F2).
3. **New rider.** Know whether that hook's site honours `requires` and
   `scope`, and whether it reads `effects` at all (F2). Five hooks ignore
   `effects`.
4. **New "until X" delay.** A `Held` field, the `add_diff!` list, `total()`,
   the `Until` match in `resolve_play`, and a release site (F4).
5. **New "+n Y until X" seat bonus.** A `Seat` field, the read in the stat's
   sum, the write, and the reset at the right line of
   `start_turn`/`end_turn`/`enter_discard`/combat end (F5).
6. **New in-play ability.** Pair every cost effect with its affordability
   requirement (`LockSelf`↔`CardUnlocked`, `HostBurnBlood(n)`↔`BloodAtLeast(n)`),
   and add the effect to the ally blood-cost bar if it is a blood cost
   (`plays.rs:5181-5193`) (F8, F6).
7. **New `Requirement`.** Choose the evaluator(s) it belongs to; if it is
   "engine-checked", add the real check in every enumerator that can meet
   it (play options, each `ability_options` branch, vampire abilities). Any
   evaluator that doesn't know it passes it as `true` (F9).
8. **New printed (vampire) predicate.** It may need adding to
   `printed_holds` as well as `holds`, with the same semantics (F7).
9. **New actor kind.** Update three places: enumeration, `resolve_ability`'s
   header and `ability_user` (F6).
10. **New "put this card on…" timing.** Pick `enter_play` vs `place_held`
    correctly (CLAUDE.md), push layers, check one-per-vampire (F10).
11. **Player-visible effect.** Add a line in `host/src/effects.rs` if
    clients should see it (F12).

## 4. Draft checklist for an "engine review" skill

The findings show recurring drift (twins, hidden parameters, decorative rider
data, one likely defect), so a checklist run on each engine change is
worthwhile. Each item is checkable on the diff.

1. Every new `Effect` variant is routed: present in exactly one of
   `is_combat_effect`, `is_vote_effect`, `apply_action_effect`, the
   `apply_card_effect` arms or its ability-or-rider exclusion list. After
   F1, `Effect::family()` compiles.
2. A new `Effect` that can appear in both a mode and an ability is
   implemented once (delegated), or the two copies are diffed and any
   difference is commented with its reason.
3. No new `Effect` whose body equals an existing one's: grep for a variant
   doing the same `burn_blood`/`unlock`/`layers.push`, especially one that
   is just `ActionLayer(Modifier::…)`/`Layer(Modifier::…)`.
4. No effect behaviour depends on which interpreter runs it (the
   `CircleBloodMoves` case): parameters live in the definition.
5. Every new rider: its hook site reads `r.requires` (via `rider_holds`),
   applies `r.scope`, and interprets `r.effects`. Or a test asserts the
   hook's riders carry exactly what the site hard-codes.
6. Every new `Requirement` arm returning `true` names where the real check
   lives, and that check exists in every enumerator that can see the
   requirement (play options, each `ability_options` actor branch,
   `vampire_ability_options`). After F9, `Requirement::slot()` compiles and
   the slot test passes.
7. A predicate added to `holds` that is also meaningful on printed text is
   added to `printed_holds` with the same semantics (or both call one
   core). In particular, "is a hunt" and "is a bleed" agree.
8. A new in-play ability with a cost effect carries its affordability
   requirement (`LockSelf`→`CardUnlocked`, `LockHost`→host unlocked,
   `HostBurnBlood(n)`→`BloodAtLeast(n)`), or a comment citing why not
   (rider; action cost paid at resolution, rules-qa 3.6).
9. A new blood-cost effect is added to the ally blood-cost bar
   (`plays.rs:5181`), or the bar is derived.
10. A new delayed replacement updates `Held`, `add_diff!`, `total()`, the
    `Until` match and a release site. After F4, just the enum.
11. A new "until <moment>" seat or minion field has its reset at the moment
    the card names, cited with its ruling, and is read live (no snapshot).
12. A new `Actor` variant is handled identically in `ability_options`,
    `resolve_ability`'s header and `ability_user`.
13. A new "put this card on…" uses `enter_play` (card itself in play, not
    `effect_only`) unless the card is already in the ash heap; its test
    asserts both (CLAUDE.md).
14. No new card-name string comparison in engine files (`*.rs` except
    `cards/defs.rs`, `cards/vampires.rs` and tests). If unavoidable, it is a
    `Label`/`Holding::Named` or a named helper with a comment.
15. Doc comments on new `Effect`/`Requirement`/`Hook` variants sit on the
    right variant and name the card and ruling. Check the line just above
    each new variant.
16. A player-visible new effect has a mapping in `host/src/effects.rs`, or
    a note that it is bookkeeping.
17. Goldens and perft pass *without* `GOLDEN_BLESS` for anything presented
    as a refactor. A bless is accompanied by the decision ranges that
    changed and why.
18. No change to `Choice` variants or their order without noting the
    policy and `host`/`text` impact (action encoding).

## 5. What I did not check / uncertainty

- **Ability target re-checks (F1 `UnlockTarget`).** I did not trace whether
  abilities re-validate targets at resolution in another place, which could
  make the play/ability difference moot.
- **Unchecked `ability_options` branches (F6).** I did not prove that each
  branch's missing checks are vacuous for today's cards; the
  ChosenController branch evaluating no `requires` is the clearest case to
  look at.
- **The `IsHunt` claim (F7).** It rests on code reading plus one scratch run
  (block not offered, so stealth ≥ 1). Whether Volo/Abiku/Kalyani/Le Dinh
  Tho's text applies to a hunt action card is a rules call for the owner.
- **Out of scope.** I did not review `obs.rs`, `bots.rs`, the referendum
  vote plumbing or the contest code in depth. Searches (`DisciplineSearch`,
  `CryptSearch`, `AllySearch`, `BaseSearch`, `StoreSearch`, …) each end with
  their own shuffle and pick; they looked parallel but specialised, and I
  did not dig further.

## Appendix: the F7 scratch test (run on `fae8816`, worktree removed)

Put in `crates/engine/tests/zz_review_scratch.rs` and run with
`cargo test --profile ci -p zero-vtes-engine --test zz_review_scratch -- --nocapture`.

```rust
use zvtes::decision::ActionKind;
use zvtes::game::Engine;
use zvtes::ids::SeatId;
use zvtes::state::find_card;
mod common;
use common::*;

fn setup() -> (Engine, zvtes::ids::MinionId, zvtes::ids::MinionId) {
    let mut e = Engine::bare(2);
    for s in &mut e.state.seats { s.pool = 10; }
    e.state.turn = 4;
    let volo = e.add_minion(SeatId(0), find_card("Volo"), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    (e, volo, z)
}

#[test]
fn volo_plain_hunt() {          // printed: stealth = 0, a block is offered
    let (mut e, volo, _) = setup();
    pick(&mut e, action(volo, ActionKind::Hunt));
    eprintln!("stealth = {}", e.action().unwrap().stealth(&e.state));
}

#[test]
fn volo_lokis_gift() {          // printed: action=None, block offered=false, Successful
    let (mut e, volo, z) = setup();
    hand(&mut e, 0, &["Loki's Gift"]);
    pick(&mut e, play(0, 0, volo));
    let req = request(&mut e);
    eprintln!("action={:?} block offered={} outcome={:?}",
        e.action().map(|a| (a.kind, a.is_hunt)),
        req.options.contains(&block(z)), e.last_outcome);
}
```

Observed:

- **Plain hunt.** Stealth 0 (+1 hunt, −1 Volo); the run stopped at the
  block decision.
- **Loki's Gift [pro]** ("+1 stealth hunt action"). The action resolved
  `Successful` with no block offered to either 0 intercept Ventrue.

Reachability pruning (`intercept_reachable`) only hides an attempt that
cannot reach the stealth, so stealth was ≥ 1: Volo's −1 "when hunting" did
not apply to the card hunt. The code reading (`state.rs:1840`,
`actions.rs:823-838`) gives the same answer.
