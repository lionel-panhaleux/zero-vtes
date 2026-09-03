# Effects framework — P1 design proposal

**Status: accepted by the owner (2026-09-03); the settled shape is recorded in
DESIGN §4.5 and the §9 answers in rules-qa 3.2–3.5. This document keeps the
rationale and the worked stress-set examples. Implementation in progress.**
DESIGN §4.5 fixes the rule this must satisfy: playability and execution derive
from one card representation, and the legal-option mask is *generated*. The
framework must express the stress set (DESIGN §6) before it freezes. This
document proposes the representation, works every stress-set card through it,
lists what the spec (`../vtes-advanced-rules`) already settles, and ends with
the questions only the owner can settle. Once agreed, the settled parts move
into DESIGN §4.5 and implementation starts.

Inputs sized on: the reviewed window matrix (`data/window-matrix-reviewed.csv`,
32 windows, 14 flags), the in-play ability census
(`data/ability-census-draft.csv`, 233 permanents — a sizing tool, its numbers
are indicative), the V5 rulings backlog (`data/rulings-v5.csv`, 1,113 rows on
322 cards, group-keyed rulings expanded to their members), and the generated static halves (`crates/engine/src/crypt.rs`,
`crates/engine/src/library.rs`).

---

## 1. The split that makes the mask generatable

A card has two halves:

- **Legality half** — everything the engine must *enumerate* to build a
  decision request without running card code: window, actor, requirements,
  costs, target predicates, once-per limits. This half is a **closed data
  vocabulary**: a fixed set of enum variants evaluated over game state. A card
  that needs a predicate the vocabulary lacks extends the vocabulary (one
  variant, one evaluator, reused by every card that needs it), never escapes
  into custom code. The static part of this half (types, disciplines, clans,
  costs, labels, per-mode windows) is already generated into `library.rs`.
- **Execution half** — what happens at resolution: a list of effect primitives
  interpreted by the engine, with `Custom(fn)` as the escape hatch DESIGN §4.5
  allows for genuinely weird cards. Custom code may *do* anything, but it
  cannot change *whether* the option was legal: legality was decided by the
  first half.

Consequence: the design question is the **predicate/requirement language**,
not the effect primitives. Scalpel Tongue ("a vampire who has cast votes or
ballots in this referendum") and Archon Investigation ("the bleed amount is 4
or more") are the two stress cards that test whether that language is rich
enough; both are answered below with plain state queries.

## 2. Card representation

Per card, per **mode** (the `[dis]` / `[DIS]` / `[TYPE]` lines of the printed
text, plus "As above" inheritance — the same split `scripts/window_matrix.py`
already performs):

```rust
struct ModeDef {
    tags:     &[Tag],            // [dom], [DOM], [COMBAT]… (generated)
    window:   &[Window],         // names from the reviewed matrix (generated)
    actor:    Actor,             // who may use it (§2.2)
    requires: &[Requirement],    // legality predicates on state (§2.3)
    cost:     &[Cost],           // paid at resolution (spec §1.7.1)
    targets:  &[TargetSpec],     // enumerated by the mask (§2.4)
    limit:    Option<Limit>,     // once-per tracking (spec §1.2.1)
    effects:  &[Effect],         // executed at resolution (§3)
    riders:   &[Rider],          // deferred effects bound to a hook (§4)
}
```

In-play abilities on permanents use the **same shape** with a source of
`InPlay(card)` instead of `Hand`; their windows are combat/action/phase
windows or hooks (§5). A card is `CardDef { static: &LibraryCard, modes: &[ModeDef] }`.

### 2.1 Windows

The 32 names of the reviewed matrix, unchanged. One name per legality filter
(DESIGN §4.4). Two are not card-play windows but hooks and are treated in §4:
`CARD_AS_PLAYED` (the as-played cancel window, spec §1.8.1) and
`COMBAT_WOULD_END` (a would-end replacement point).

### 2.2 Actor

```
Actor = ActingMinion              // action modifiers (spec §2.1.2)
      | ReadyUnlockedOfOtherSeat  // reaction cards; widened by Wake, or by
                                  //   usable_locked / REACTION_UNLOCK texts
      | Combatant                 // combat cards: the acting side or the other
      | Methuselah                // masters, events, Methuselah ballots
      | Bearer | Controller       // in-play abilities
```

Card text that widens the actor ("only usable by a ready vampire other than the
acting minion", spec §2.1.2) is a `Requirement` on the minion, not a new actor
class: the controller stays the acting Methuselah.

### 2.3 Requirements — the closed vocabulary

Evaluated over `GameState` plus the current action/combat/referendum context.
Sized from the review notes and the stress set; grouped by what they read:

- **Minion (self or target)**: `Discipline(d, level)`, `Clan`, `Sect`,
  `Title(kind)`, `CapacityCmp(op, n)`, `Younger | Older(than target)`,
  `Locked`, `Unlocked`, `Ready`, `InTorpor`, `IsVampire | IsAlly`,
  `HasBlocked(this action)`, `HasCastVotes(this referendum)`,
  `HasCounters(kind)`, `IsAnarch`, `IsBaron`, `HasTitle(any | prince | …)`.
- **Action context**: `IsBleed`, `IsPolitical`, `IsNonBleed`,
  `RecruitOrEmploy(wraith | zombie)`, `DirectedAtYou`, `BleedAmountCmp(op, n)`,
  `BlockAttemptPending`, `Blocked(before resolution)`, `BlocksDeclined`,
  `WouldBeSuccessful`, `ActingMinionControlledBy(predator | prey | other)`,
  `StealthNeeded`, `InterceptNeeded` (rules-qa 1.6 — see §7).
- **Combat context**: `Range(close | long)`, `FirstRound`,
  `BothCombatantsReady`, `CombatWouldEnd`, `WasBlocked(this vampire)`.
- **Referendum context**: `PollingStep`, `BeforeVotesCast`, `Passed`,
  `VotesMargin` (Amici Noctis, Voter Captivation).
- **Seat**: `ControlsReadyAnarch`, `HasEdge`, `PoolCmp`, `IsPrey | IsPredator`.
- **Card**: `IsInPlayCopyOf(name)`, `NameIs(name)` (Villein / Minion Tap).

"Only when needed" for stealth/intercept is a requirement (`StealthNeeded`)
so surplus plays are masked out, pending the owner's answer to rules-qa 1.6.

### 2.4 Targets

`TargetSpec = { kind: Minion | Seat | CardInPlay | CardInHand | Slot, filter: &[Requirement] }`.
The mask enumerates every entity satisfying the filter; a play option is
`(card, mode, targets…)`. Cards with no target spec have one option per mode.
Variable numeric choices (`Cost::X`, Villein's 2–5 blood, Kine Resources
Contested's 4 points) are **stepwise sub-decisions after the play is
chosen**, not part of the option list (DESIGN §4.4: terms confirmation is
stepwise and factored). This keeps the option list bounded by entities, never
by numeric ranges.

### 2.5 Costs and limits

`Cost = Blood(n) | Pool(n) | Lock(self) | Burn(self) | BurnBloodFrom(target, n) | X`.
Paid at resolution (spec §1.7.1); a canceled-as-played card pays nothing
(§1.8.3); cost modifiers are layers (§3.2) queried at payment, so Consign to
Oblivion is a layer, not a special case.

`Limit = Per(Action | Combat | Round | Turn | Referendum | UnlockCycle | Game)`
bound to the card copy, or to the minion for "a vampire can play only one X
each …" (spec §1.15). "(limited)" is a rulebook cap bound to what the rulebook
names (bleed increase → the action; additional strike → the combatant per
round, spec §1.2.1).

## 3. Execution half

### 3.1 Effect primitives

Sized from the census kinds and the stress set; each is a small pure
transition or a layer push:

```
Effect = AddStealth(n) | AddIntercept(n)          // action layers
       | BleedMod(n, limited) | ReduceBleed(n | ToZero)
       | RedirectBleed(RedirectRule)                // Deflection class; reopens blocks
       | Lock(t) | Unlock(t) | Wake(t)
       | BurnBlood(t, n) | BurnPool(seat, n) | MoveBlood(from, to, n) | GainBlood(t, n)
       | Strike(StrikeKind) | Maneuver | Press | AdditionalStrike(n, limited) | Dodge
       | CombatEnds | NewRound | SetRange(close)
       | Damage(t, n, kind) | Prevent(n)
       | Cancel(card_being_played, refund)
       | Abstain(vampire)                           // cancels its cast votes
       | AttemptBlock(minion)                       // unlock-and-attempt class
       | Burn(card | minion) | PutInPlay | PutOn(t) | Counters(op)
       | Search(zone, filter) | Draw(n) | ReplaceControl(...)
       | Allocate(points, among: SeatFilter)        // interactive terms
       | Layer(scope, Modifier)                     // continuous effects, §3.2
       | ContinueAction { stealth, as_if_unblocked }  // Form of Mist
       | Custom(fn)
```

Mandatory effects are auto-applied; optional ones ("can burn 1 blood to …")
are decisions in the window where they arise (spec §1.1.1).

### 3.2 Scopes and layers

DESIGN §4.5's scope stack: `Strike < Round < Combat < Action < Phase < Turn <
InPlay(card) < Game`. A **layer** is `(scope, Modifier)`; the stack is popped
wholesale when its scope ends. Every derived quantity is a fold over layers:
stealth and intercept of the current action, bleed amount, strength, votes,
hand size, cost of a card being played, "can block", "cannot be blocked".
Folding order per spec §1.15: absolute setters first, then `+X` adders; copies
add. Permanents contribute `InPlay(card)` layers (the census's `continuous`
kind: 125 cards); events contribute `Game`-scoped global layers (Anarch
Revolt's unlock-phase burn is a hook, §4).

## 4. Hooks

Points where riders, in-play triggers and interrupt classes fire. Each hook is
either an **ordered window** (players may play/use things: sequencing rules
apply, elision applies) or a **trigger point** (mandatory effects fire in
canonical order; optional ones become decisions).

| Hook | Kind | Tenants (examples) |
|---|---|---|
| `AsPlayed(card)` | window (wakes + cancels only, rules-qa 2.4) | Sudden Reversal, Hide the Mind, Anticipation |
| `AsAnnounced` | window | Consign to Oblivion, Form of the Cobra |
| `BlockAttemptDeclared`, `Blocked`, `BlocksDeclined` | windows (block-loop states, rules-qa 1.3) | Elder Impersonation, Change of Target, Deflection |
| `WouldBeSuccessful` | window (last before resolution, spec §3.4.3) | Spying Mission; bounces must precede it |
| `Resolution`, `AfterResolution` | trigger + window | Freak Drive, Amici Noctis |
| `AfterCombat` (after *all* combats, rules-qa 2.1) | window | Form of Mist rider, Cats' Guidance |
| `CombatWouldEnd`, `EndOfRound`, per-step combat windows | window | Telepathic Tracking, Taste of Vitae |
| `Prevention` (inside damage resolution) | window, order-sensitive | Hidden Strength |
| `UnlockPhase(seat)`, `MasterPhase`, `DiscardPhase`, `InfluencePhase` | trigger + window | Homunculus, Anarch Revolt, events |
| `Draw`, `Replace` | trigger, replacement semantics | Black Market Cache, Consign's delayed replacement |
| `EntersPlay`, `LeavesPlay`, `BloodLoss`, `SuccessfulBleed`, `Ousted` | trigger | War Ghoul, Melange, Anathema class |

A `Rider` is `(hook, requires, effects)` attached to a card play — "After
combat ends, if this vampire was blocked, they can burn 1 blood to …". It is
registered when the card resolves and fires (as a decision if optional) when
its hook opens, then expires with its scope.

## 5. In-play abilities

The census kinds map onto the shape above with no new machinery:

| Census kind | Representation |
|---|---|
| `continuous` | `Layer(InPlay(card), Modifier)` pushed on entering play |
| `lock_ability`, `burn_ability` | a `ModeDef` with `source: InPlay`, `cost: Lock(self)` / `Burn(self)`, its own window |
| `combat_grant` (weapon strikes, optional maneuver each combat) | a `ModeDef` in the combat windows, actor `Bearer`, `limit: Per(Combat)` or `Per(Round)` |
| `granted_action` (already flagged `in_play_action`) | an entry in the minion's action list; NRA per copy (rules-qa 1.4) |
| `phase_trigger`, `event_trigger` | a `Rider` on the permanent, registered while in play |
| `replacement` | a `Rider` on `Draw`/`Replace`/`WouldX` hooks with replacement semantics |
| `counters` | state on the card (`Counters(op)` effects) |
| `label` | static data (`Label` in `library.rs`): rules key on it (one hunting ground per turn, one vehicle, one archetype) |

Ability use is not a card play and not an action (spec §1.5): no card-play
costs apply, locked and torpid bearers may use them unless the text says
otherwise, and reuse is unlimited unless the text limits it (spec §1.5.4).

## 6. Decision-protocol extension

```
Choice::Play      { card: HandIdx, mode: u8, targets: [EntityRef] }
Choice::UseAbility{ card: InPlayRef, mode: u8, targets: [EntityRef] }
Choice::TakeAction{ minion, action: ActionKind | Card(HandIdx, mode), target }
Choice::SetX(n) / Choice::Allocate(seat, n)   // stepwise numeric terms
```

The mask builds options by: window match → actor availability → requirements
→ enumerate targets → limit check. Elision unchanged. Windows the engine can
prove empty (no card in any hand or play matches) are elided as today — the
omniscient engine's view, never the player's.

## 7. Stress set, written in the shape

Full form for the five hardest; the rest in one line each.

**Archon Investigation** (Master, pool 3, `out_of_turn`):
window `ACTION_BLOCKS_DECLINED`; actor `Methuselah`; requires
`[DirectedAtYou, IsBleed, BleedAmountCmp(≥, 4), BlocksDeclined]`; cost
`Pool(3)`; effects `[Burn(acting_minion), ActionFails]`. Out-of-turn is
accounting on the master phase (see open question 4), not a window.

**Scalpel Tongue** (Action Modifier / Reaction, blood 1):
window `REFERENDUM`; actor `ActingMinion` *or* `ReadyUnlockedOfOtherSeat`
(two entries, one per type — the type decides the actor, the window is the
same); requires `[PollingStep]`; targets
`[Minion { filter: [HasCastVotes(this referendum)] }]`; effects
`[Lock(t), Abstain(t)]`; `[CEL][PRE]` adds `BurnBlood(t, 1)`. `HasCastVotes`
is the per-vampire vote-cast state rules-qa 1.1 made first-class; the P0
blood hunt already tracks it (`spent`).

**Form of Mist** (Combat, [pro]/[PRO]):
`[pro]`: window `COMBAT_STRIKE_DECLARATION`; effects `[Strike(Dodge)]`.
`[PRO]`: same window; effects `[Strike(CombatEnds)]`; limit
`Per(Action, minion)`; rider `{ hook: AfterCombat, requires: [WasBlocked(self)],
optional cost Blood(1), effects: [ContinueAction { stealth: +1, as_if_unblocked }] }`.
`ContinueAction` re-enters the block loop with the action's stealth/intercept
context preserved (DESIGN §4.4 "action continuation"); the `even if stealth is
not yet needed` clause waives `StealthNeeded` for that +1.

**Telepathic Tracking** (Combat, blood 1, [aus]/[AUS]):
`[aus]`: window `COMBAT_PRESS`; effects `[Press(continue_only)]`; rider
`{ hook: NewRound, effects: [Layer(Round, OptionalManeuver(self))] }`.
`[AUS]`: window `COMBAT_WOULD_END`; requires `[BothCombatantsReady,
CombatWouldEnd]`; effects `[NewRound]` — a replacement of the would-end event
(spec §2.4.1 orders replacements within the window).

**Consign to Oblivion** (Action Modifier, blood 1, [obl]/[OBL]):
window `ACTION_AS_ANNOUNCED`; actor `ActingMinion`; effects
`[Layer(Action, CostMod { card_type: Reaction, blood_or_life: +1 })]`;
`[OBL]` adds `Layer(Action, DelayedReplacement { card_type: Reaction, until:
EndOfAction })`. Rules-qa 2.4's example lives here: a wake played in Consign's
as-played window resolves before the layer exists.

One-liners:

- **Deflection**: `ACTION_BLOCKS_DECLINED`, requires `[DirectedAtYou, IsBleed,
  BlocksDeclined]`, target `Seat { filter: [Not(ActingController), Not(Self)] }`, effects
  `[Lock(self) (basic only), RedirectBleed(to t)]`; `bounce` reopens block
  attempts for the new target (rules-qa 1.3).
- **Eyes of Argus**: `[aus]` `ACTION_STEALTH_INTERCEPT` requires
  `[DirectedAtYou, InterceptNeeded]`, `AddIntercept(2)`; `[AUS]`
  `WAKE;CARD_AS_PLAYED`, requires `[Locked(self)]`, `Wake(self)`.
- **Sudden Reversal**: `CARD_AS_PLAYED`, requires `[PlayedCardIs(Master),
  PlayedBy(other seat)]`, `Cancel(refund)`; out-of-turn accounting.
- **Immortal Grapple**: `COMBAT_BEFORE_STRIKES`, requires `[Range(close)]`,
  limit `Per(Round, minion)`, `Layer(Round, OnlyHandStrikes(both))`; `[POT]`
  adds `Press` rider and `Layer(Combat, NextRoundAtClose)`.
- **Blur**: `COMBAT_ADDITIONAL_STRIKE`, `AdditionalStrike(1 | 2, limited)` —
  the rulebook cap binds the combatant per round.
- **Villein** (trifle): `MASTER_PHASE`, target `Minion { filter: [Controlled,
  BloodCmp(≥,1)] }`, stepwise `SetX(2..=5)`, effects `[PutOn(t),
  MoveBlood(t → pool, X), Layer(InPlay, CostMod { name: "Minion Tap", pool +1 }),
  Layer(InPlay, CostMod { name: "Villein", on: t, pool +1 })]`; trifle flag
  restores the master phase action.
- **Homunculus**: retainer; ability `{ source: InPlay, hook:
  UnlockPhase(any seat), actor: Controller, cost: BurnBlood(employer, 1),
  effects: [Unlock(employer)] }`, optional.
- **Anarch Revolt**: `PutInPlay`; rider `{ hook: UnlockPhase(seat), requires:
  [Not(ControlsReadyAnarch(seat))], effects: [BurnPool(seat, 1)] }` per copy
  (spec §1.15: two Revolts burn 2); granted political action "call a
  referendum to burn this card" with `+1 stealth`.
- **Kine Resources Contested**: political action; terms = stepwise
  `Allocate(4, among ≥2 seats)` before polling; effects on pass
  `[BurnPool(seat, points) for each]`.
- **Contested unique / contested title**: already in P0 (`contested`,
  `title_contested`); a unique *library* card entering play reuses the same
  entry/upkeep flow with `Label::Unique`.

## 8. What the spec already settles

Cited, not re-decided: requirement timing and abnormal entry (§1.6), cost
timing and arithmetic (§1.7), the as-played window and what a cancel reaches
(§1.8), replacement and delayed replacement (§1.9), stacking (§1.15), ability
use vs card play (§1.5), who may play what (§2.1.2), atomic resolution (§2.1.3),
step windows and deadlines (§2.1.4), after-resolution ordering and after-combat
(§2.3). Each becomes a test id when implemented.

## 9. Questions put to the owner (answered 2026-09-03 — see rules-qa Batch 3)

1. **Closed legality vocabulary.** Do you accept the rule that no card may
   decide its own legality in custom code — every requirement and target
   filter is a named predicate evaluated by the engine? (Cost: the predicate
   enum grows with the pool; benefit: the mask is provably generated.)
2. **Authoring format.** The behavior half of 421 cards is hand-written. I
   propose Rust data tables (one file per card type, `static` arrays of
   `ModeDef`, checked by the compiler against the generated static half by
   card id), rather than a text DSL parsed at build time. A DSL would read
   closer to card text but adds a parser to trust. Preference?
3. **"Only when needed" (rules-qa 1.6)** as a mask-level requirement
   (`StealthNeeded` / `InterceptNeeded`): surplus plays are illegal, not merely
   pointless. Pre-emptive stealth before any block attempt: legal or not?
4. **Out-of-turn masters** (Archon Investigation, Sudden Reversal): what is
   the master-phase-action accounting — does the play consume the player's
   next master phase action, and is a trifle relevant? (Rules-qa 1.5 is still
   open on the master phase generally.)
5. **Form of Mist's continuation**: on re-entering the block loop, are the
   defenders' earlier declines reset (it is the same action, but the "target"
   has not changed — rules-qa 1.3 says only a target change reopens
   attempts)? DESIGN §4.4 assumes stealth/intercept context is preserved.
6. **Numeric terms as stepwise sub-decisions** (Villein's X, KRC's allocation,
   `Cost::X`) rather than enumerated options: acceptable for the agent's
   action space? It mirrors table practice ("Villein for 4").

Answers: 1 yes; 2 data tables; 3 stealth only when needed, waived by card
text, permanent bonuses exempt (rules-qa 3.2); 4 out-of-turn counts toward the
next master phase, trifle refunds only there, never two between turns (3.3);
5 declines are per Methuselah and survive the continuation (3.4); 6 terms at
play for Villein, at the referendum's terms step for KRC (3.5).

## 10. Not in this proposal

Observation encoding (P3), the wire protocol (P5), and the combat machine's
step-by-step state (it hosts the combat windows above; its own design is the
P1 implementation step after this shape is agreed).
