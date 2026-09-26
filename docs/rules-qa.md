# Rules Q&A

Running design-level rules clarifications with the project owner (VEKN judge).
Each answered entry must eventually be backed by a scenario test; the test name is
recorded next to the answer so ruling and engine behavior cannot drift apart.

Conventions: **Q** — question, **A** — authoritative answer (owner), **Test** —
scenario test id(s) covering it. Unanswered entries are marked _open_.

---

## Batch 1 — Sequencing, impulse, and window mechanics

### 1.1 Vote-casting granularity

During polling, what is the atomic unit of an irrevocable commitment?

- Must all votes from a single source (e.g. a Prince's 2 votes) be cast together,
  or can they be split across rotations (1 now, 1 later)?
- May a Methuselah spread *different* sources across rotations — Prince votes on
  one rotation, burn the Edge on a later one, political card burn later still?
- "All votes/ballots from a single source must agree" — confirm "source" =
  vampire (or Edge, or card), not Methuselah.

**A** (2026-08-03):

- A source is atomic: it casts all its votes at once, in one commitment, all
  agreeing in direction. Once cast, that source is spent for this referendum.
- Yes — sources are independent commitments. A Methuselah may act with one
  source on one rotation and hold others for later rotations (Prince now, Edge
  later, political card later still).
- "Source" is either a **vampire** or a **Methuselah**. The Edge and
  discard-a-card ballots are Methuselah-sourced. Card effects depend on the
  card: most modify a vampire's votes (and thus fold into that vampire's
  source), some grant Methuselah votes.
- Methuselah-sourced ballots are **not** bound together: each one (Edge burn,
  each card discard, each Methuselah-vote effect) is its own independent
  commitment and may go either way.

**Test** — `referendum_source_atomicity`, `referendum_cross_rotation_sources`,
`referendum_methuselah_ballots_independent` (P0, on the blood hunt with the Edge
as the only Methuselah ballot; the multi-ballot case waits for card discards in
P1) — `crates/engine/tests/scenarios.rs`.

### 1.2 Impulse inside combat

The sequencing rule says any card/effect play returns the impulse to the acting
Methuselah. Within a combat round, "the acting minion has the first opportunity
at every step."

- After the defending side plays an effect at a combat step (say a maneuver),
  does opportunity return to the acting side *at that same step* (impulse-reset
  style), or does the step proceed strictly alternating?
- In combats not arising from a block (card-created combat, e.g. a rush), which
  minion is "acting" for the purposes of first opportunity at each step?

**A** (2026-08-03):

- **Impulse-reset.** Any play by either side returns the opportunity to the
  acting side at that same step; a prior pass is voided by a subsequent play.
  The step closes when both sides decline in succession.
- **Acting minion = the minion performing the action**, whether it's a normal
  action that got blocked or an unblocked rush. The other combatant is simply
  "the opponent" — they are "the blocker" only if they actually blocked (a
  rushed minion is not a blocker).
- **Combat with no acting minion is possible** (e.g. Taunt the Caged Beast,
  Illusions of the Kindred, Siren's Lure; the Slave rule typically creates
  such combats). When there is no acting minion, first opportunity at each
  step goes to the acting Methuselah's minion; if the acting Methuselah has no
  minion in the fight, go around clockwise starting from them.

**Test** — _implemented_ (tests/combat.rs): `combat_step_impulse_reset`.
_Pending_: `combat_rush_acting_minion`, `combat_no_acting_minion_ordering`
(no card-created combat in the slice yet).

### 1.3 Block loop details

- After a failed block attempt (stealth ≥ intercept at resolution), state returns
  to "no current block attempt". May the *same* minion immediately declare another
  attempt (assuming still eligible), with accumulated stealth/intercept persisting
  for the action's duration?
- A Methuselah passed on blocking in state A, then the action's target changes.
  Confirm: their block opportunity fully reopens (the earlier pass is void), and
  this is the only thing that reopens it.
- "Only the blocking Methuselah can force the current blocker to attempt" (state
  B) — give a worked example of who may do what in state B; which effects are
  legal for third parties there?

**A** (2026-08-03) — corrects the model assumed in the question:

- **An attempt does not "fail" at a stealth ≥ intercept checkpoint.** It only
  conclusively fails once *all* Methuselahs have passed (consecutive-pass
  closure). While the attempt is ongoing, the defending Methuselah can keep
  stacking intercept, pass when they can't reach stealth, and the impulse moves
  on to opponents who may still intervene for them (e.g. KRCG News Radio to
  give intercept, Draba / Night Terrors to remove stealth) — after which the
  defender may continue (impulse-reset: a play voids prior passes, per 1.2).
  There is no separate "re-declare" transition: the attempt persists until
  closure.
- **Failing to block is just passing. Declining to block is passing without an
  ongoing attempt — and that decision is final.** If the target passes without
  attempting, a third party's subsequent stealth-removal (Draba, Night Terrors)
  does not let them change their mind and block (e.g. via Second Tradition).
- **Only a change of target resets block opportunities.** Note: the card
  "Change of Target" does *not* change the action's target — it terminates the
  action. Target changes here means bounces: Deflection, Telepathic
  Misdirection, etc.
- **Third parties:** may intervene whenever their cards allow it *and the
  impulse reaches them* — which is normally only after both acting and target
  have passed (the impulse mostly ping-pongs between those two). Non-acting
  Methuselahs cannot play action modifiers; only the acting minion plays
  modifiers, though some cards (e.g. Cloak the Gathering) let other minions of
  the *acting* Methuselah do so. Everyone can play reactions with an
  unlocked/awoken minion. "+X intercept" always means the minion playing the
  card, so granting intercept to someone else's minion is uncommon — effects
  like KRCG News Radio are the exception.
- **No sequencing gotchas:** the sequencing rule assigns an order to effects; it
  is not a bait mechanism. A third party cannot promise intercept, wait for the
  defender to pass, then retract so the block dies without response opportunity
  (consecutive-pass closure + impulse-reset prevent this). The real ordering
  constraint: a third party wanting to strip stealth (Night Terrors) needs the
  defender to *attempt first* (e.g. Second Tradition) — since impulse reaches
  the third party only after the target passes, and passing without attempting
  is a final decline. Once attempting, the defender need not have matching
  intercept to keep the attempt alive.

**Test** — `block_decline_is_final` (`crates/engine/tests/scenarios.rs`);
`block_attempt_persists_until_all_pass`, `block_reopen_only_on_bounce`
(`crates/engine/tests/bleed.rs`). _Pending_: `block_third_party_intercept_after_attempt`,
`block_no_bait_retraction` (need a third-party intercept source such as KRCG
News Radio: an in-play ability slice).

### 1.4 No-repeat-action rule, precisely

State the NRA rule as the engine should enforce it:

- Which outcomes trigger it — blocked only? canceled as announced? failed
  referendum? unsuccessful for any reason?
- Scope: same *named* action card, per *minion* per turn? What about the same
  card played by a different minion, or basic actions (bleed twice after a first
  bleed was blocked)?
- Interaction with "cannot use the same copy of a card in play for an action more
  than once per turn."

**A** (2026-08-03) — corrects the question's framing: NRA is not about
*unsuccessful* actions. **A minion cannot do the same action twice per turn,
successful or not; what matters is that the action *resolved*.**

- **Trigger = resolution.** Blocked or failed-referendum actions resolved, so
  the lock applies. A canceled-as-announced action (e.g. Direct Intervention)
  never resolved — the minion may choose the same action again.
- **"Same action" identity:**
  - Card played from hand: same card **name**, per minion.
  - Card in play: same **copy** — two Anarch Revolts in play provide two
    *different* actions, one each.
  - **Special case — bleeds and political actions:** a minion can perform only
    ONE bleed and ONE political action per turn, regardless of which card (if
    any) provides it. Other rulebook actions (hunt, leave torpor, rescue from
    torpor, …) are *not* limited this way — a minion may hunt twice.
- **Per-copy rule for cards in play** is a facet of the same rule (cards in
  play use copy identity where played cards use name identity), but the engine
  may implement them as independent locks.
- **Card text "cannot take the same action" (e.g. Change of Target)** is
  similar to NRA but *also* covers rulebook actions the generic NRA exempts
  (e.g. hunt). Rare corner: the rulebook equip-from-minion action (take
  equipment from another minion controlled by the same Methuselah) is not
  subject to generic NRA, and a minion may equip repeatedly from the same
  minion — but fetching the *same equipment* as the previous action makes it
  "the same action" for such card texts.

**Test** — `nra_resolution_not_success`, `nra_one_bleed_per_turn_hunt_repeatable`
(P0, `crates/engine/tests/scenarios.rs`; the bleed half of the special cap and
the hunt exemption). _Pending P1 (need cards)_: `nra_cancel_allows_retry`,
`nra_one_political_per_turn`, `nra_in_play_copy_identity`,
`nra_same_action_card_text_covers_hunt`.

### 1.5 Master phase structure _(answered 2026-09-06; out-of-turn accounting in 3.3)_

- During a Methuselah's master phase, what may *other* Methuselahs legally do,
  exhaustively? (Cancel-as-played responses, out-of-turn masters — anything
  else? No wakes without an action, presumably.)
- Confirm the master phase is "spend MPA one effect at a time, each play opening
  its cancel window, until the player moves on" — i.e. an offer window whose only
  actor is the phase owner, punctuated by reaction sub-windows.

**Answer (2026-09-06)** — Yes to both: the others may cancel in the
as-played window and play out-of-turn masters, nothing else (no wakes
without an action); the master phase is the owner's offer window,
punctuated by the cancel windows — and, per 7.9, closed by an impulse
rotation over the other seats for their untimed and mandatory effects.

### 1.6 "Only when needed" stealth/intercept as a legality rule _(answered in 3.2)_

Stealth may be added only when currently blocked with sufficient intercept;
intercept only when stealth exceeds it.

- Confirm the engine should treat surplus stealth/intercept plays as *illegal*
  (masked out), not merely pointless.
- Corner cases: effects that add stealth/intercept as a side effect of a card
  played for another purpose; stealth added when no block attempt has been
  declared at all (e.g. pre-emptively before asking for blocks) — legal or not?

---

## Batch 2 — Window-matrix arbitration (from `docs/window-review-questions.md`)

The hand review of the window matrix (`data/window-matrix-reviewed.csv`) raised
per-card questions and introduced new window names/flags; the owner arbitrated
them here. Entries reference the numbered items of the review queue.

### 2.1 "After block resolution" plays are after-combat plays (queue 2, 11)

Cats' Guidance [ani] and Forced Vigilance [for]: "Only usable by a locked
vampire who has blocked, after block resolution. Unlock this vampire." Is the
play point the after-combat window, or a point between block resolution and
combat start?

**A** (2026-09-02): It is after combat — and after **all** combats of the
action, in case more are queued (e.g. Psyche!). So `ACTION_AFTER_COMBAT` opens
once, after the last combat arising from the block, not after each one.

**Test** — _pending_: `after_combat_window_after_all_combats`.

### 2.2 `COMBAT_UNTIMED` — combat cards with no printed step (queue 4, 12)

Claws of the Dead [pro], Wolf Claws [pro] ("hand strikes aggravated this
round"), Pack Alpha [ANI] (burn retainer to attach): one window "any combat
impulse, deadline derived from the effect", or per-card step assignment?

**A** (2026-09-02): One window with an effect-derived deadline. The play must
precede the step it modifies (advanced-rules §2.1.4): aggravated-hands plays
must precede the strike resolution they affect.

**Test** — _implemented_ (tests/combat.rs): `combat_untimed_deadline_precedes_step`
(Claws of the Dead [pro]: offered at every impulse through the before-resolution
window, not in the prevention window).

### 2.3 Vocabulary introduced by the review (queue 1, 3, 8, 9)

Veto/rename requested on: the `ACTION_BLOCK_ATTEMPT` (attempt pending) /
`ACTION_BLOCKED` (blocked, before block resolution) split; `CARD_AS_PLAYED`
(the as-played cancel sub-window); `OUT_OF_TURN` demoted from a window to the
`out_of_turn` flag (Archon Investigation, Sudden Reversal).

**A** (2026-09-02): All accepted. The split makes sense, `CARD_AS_PLAYED` too,
and out-of-turn is rightly a flag: for those cards the timing depends on the
card text, not on a shared window.

**Test** — covered by the per-window scenario tests of P1 (no dedicated test).

### 2.4 Wakes vs unlock reactions, and what the as-played window admits (queue 10)

The review had classified as `WAKE` every reaction "usable by a locked minion":
true wakes (Wake with Evening's Freshness, On the Qui Vive, Forced Awakening,
Eyes of Argus [AUS], Shadow Sentinel) *and* unlock / unlock-and-attempt
reactions (Dogged Pursuit, Eyes of the Wild, Guard Dogs, Rat's Warning, Second
Tradition [2nd mode], Sense the Savage Way [ANI], One With the Land [PRO],
Organized Resistance, Forced Vigilance [FOR]). Keep one window or split?

**A** (2026-09-02): Split — keep wakes separate from unlock-and-attempt /
unlock reactions, because their windows differ:

- **Wakes are playable in the `CARD_AS_PLAYED` window** — e.g. before a card
  is cancelled by something like Rewind Time, or, more subtly, before a
  modifier's effect resolves: one can wake before Consign to Oblivion raises
  the cost of the wake.
- **Unlock-and-attempt / unlock reactions are not** playable in the as-played
  window.
- The as-played window admits **only wakes and cancellation effects**. Not
  even card effects such as locking The Barrens to discard can be used there.

**Applied** (2026-09-02, review — names not yet seen by the owner, veto
welcome): the as-played window opens after *every* card play during an action,
not only after cancellable ones, and a wake played there resolves before the
played card's effect. Wakes are action-scoped ("until the end of the action"),
so master-phase as-played windows admit cancels only, pending 1.5. In the
matrix: true wakes get `WAKE;CARD_AS_PLAYED`; "unlocks and attempts to block"
cards get `REACTION_UNLOCK_ATTEMPT` (legal only while a block attempt is
possible, i.e. in the block loop); plain "Unlock this vampire" reactions (Guard
Dogs, Rat's Warning, Forced Vigilance [FOR]) get `REACTION_UNLOCK` (any
reaction window of the qualifying action). Organized Resistance's first mode (a
*locked* baron gives an Anarch +1 intercept) is neither: it is an intercept
play whose lock requirement is waived, flagged `usable_locked`.

Cross-reference for 1.6: Second Tradition [2nd mode] and Organized Resistance
print "attempts to block … even if intercept is not yet needed" — direct card
evidence that "only when needed" is the default legality rule and that cards
must opt out of it explicitly.

**Test** — `wake_playable_in_as_played_window`, `as_played_window_wakes_and_cancels_only`
(`crates/engine/tests/bleed.rs`). _Pending_: `unlock_reaction_not_as_played`
(needs an unlock reaction definition, e.g. Guard Dogs).

---

## Batch 3 — open items collected during P0 (2026-09-03)

### 3.1 A contested vampire card in torpor _(answered 2026-09-06)_

A vampire in torpor whose card becomes contested (another Methuselah brings out
the same vampire) is face down and out of play for the contest. If it wins,
the rulebook says only that the card "is unlocked and turned face up during
your next unlock phase". Does it return to the torpor region (the engine's
current reading: torpor state untouched through the contest) or to the ready
region? And while contested, is it still a legal rescue/diablerie target (the
engine says no: out of play)?

Related reading, not exercised by any precon: the rulebook exempts only
*cards* from same-controller contests, so the engine lets two vampires under
one Methuselah contest a title (a Prince and a Baron of the same city). No V5
precon holds both Mannheim or both Pittsburgh title holders, so nothing in
v1 play reaches this; confirm or veto when convenient.

**Answer (2026-09-06)** — Back to torpor: the torpor state is untouched
through the contest, as the engine reads it (and, out of play while
contested, it is no rescue or diablerie target). The title reading is
right: two vampires under one Methuselah do contest a title.

### 3.2 "Only when needed" is a mask-level rule (closes 1.6)

Stealth may be added only when a block attempt is pending with intercept ≥
stealth; surplus plays are illegal, not merely pointless. Pre-emptive stealth
before any block attempt?

**A** (2026-09-03): Stealth only when needed; no pre-emptive stealth without a
block attempt — except when the card says so ("even if stealth is not yet
needed": Form of the Cobra [PRO], Paths in Two Worlds, Form of Mist's
continuation, Creepshow Casino; intercept: Organized Resistance, Second
Tradition), or when the effect is a *permanent bonus* rather than stealth
added to the current action — the owner's example is Loup (Toreador get +1
stealth while it is in play), playable with no block attempt at all.

**Applied**: `StealthNeeded` / `InterceptNeeded` are requirements on
stealth/intercept-adding plays; the "not yet needed" texts waive them; put-in-
play bonuses are layers and carry no such requirement.

**Test** — `stealth_only_when_needed`, `stealth_waived_by_card_text`
(`crates/engine/tests/bleed.rs`).

### 3.3 Out-of-turn master cards and the master phase action

Archon Investigation, Sudden Reversal (and trifles such as Wash outside V5):
what is the master-phase-action accounting?

**A** (2026-09-03): An out-of-turn master card counts toward the next master
phase of the player who plays it. An out-of-turn *trifle* also consumes that
next master phase action, and gives an additional master phase action back
only when the player reaches their master phase (because it is a trifle).
Ergo a player can never play two out-of-turn cards between their turns.

**Applied**: per-seat `out_of_turn_used: Option<{trifle}>`; an out-of-turn
play is legal only while unset; at the seat's next master phase the action
count is 1 − 1 (+1 if trifle), then the flag clears.

**Test** — `out_of_turn_consumes_next_mpa`, `no_two_out_of_turn_between_turns`
(`crates/engine/tests/scenarios.rs`). _Pending_: `out_of_turn_trifle_refunds_at_master_phase`
(no out-of-turn trifle in the V5 pool; the accounting is implemented).

### 3.4 Form of Mist continuation and earlier declines

On "continue the action as if unblocked", are the defenders' earlier declines
reset?

**A** (2026-09-03): There is no "earlier decline" minion by minion: a
Methuselah declines *as a whole* if they do not block. A Form of Mist
continuation implies there was a block. If the target had declined and another
player blocked, then after the continuation the target's decline still stands
and they cannot block — it is the same action.

**Applied**: continuation re-enters the block loop with the action's context
(stealth/intercept layers, `declined` set) preserved; only the pending-attempt
state resets. Matches rules-qa 1.3 (only a target change reopens attempts).

**Test** — _implemented_ (tests/combat.rs): `continuation_keeps_declines`.

### 3.5 When numeric terms are chosen

Villein's "move 2 to 5 blood", Kine Resources Contested's "allocate 4 points":
stepwise sub-decisions rather than enumerated options — when are they taken?

**A** (2026-09-03): Terms are chosen and announced when the card is played
(Villein): it matters because it can change what the other players do. For
Kine Resources Contested the terms are announced as the terms of the
referendum — after the action succeeds, before votes are cast.

**Applied**: the play option is (card, mode, targets); numeric terms are a
sub-decision at the `AtPlay` point (before the as-played window opens) for
cards like Villein, and at the referendum's terms step for political cards.

**Test** — `terms_announced_at_play` (`crates/engine/tests/scenarios.rs`).
_Implemented_ (tests/politics.rs): `referendum_terms_before_votes`.

### 3.6 Unpayable cost at resolution: the action fizzles

A cost paid at resolution (rescue's 2 blood, leave torpor's 2 blood) can become
unpayable during the action (blood spent on an as-announced modifier). The
engine had treated the action as unsuccessful.

**A** (2026-09-04): The action *fizzles*: it is successful — it was not
blocked, so e.g. Freak Drive can be played — but it has no effect.

**Applied**: `actions::Outcome::Fizzled`, distinct from `Failed` (Archon
Investigation) and `Blocked`; after-resolution riders will read it as
successful.

**Test** — _pending_: `fizzled_action_is_successful` (needs an after-resolution
card such as Freak Drive).

### 3.7 Unlock-phase impulse and other players' abilities

Where does another Methuselah's unlock-phase ability (Homunculus: "during any
Methuselah's unlock phase") sit in the active player's unlock phase?

**A** (2026-09-04): The acting player does all the effects they want, then
passes the impulse so Homunculus can be used, then the impulse goes back to
the acting player. It is not outside the unlock-phase ordering.

**Applied**: after the active seat's Edge and contest decisions, an ordered
window over the seats clockwise from the active one offers unlock-phase
abilities; a use returns the impulse to the active seat (DESIGN §10 item 7).

## Batch 4 — combat, collected during slice 3 (2026-09-04)

Modeling calls made while building the combat machine, each the default the
engine now applies; the owner's answer overrides.

### 4.1 The after-combat window and the Form of Mist continuation

Form of Mist [PRO] ("after combat ends … continue the action") and Cats'
Guidance [ani] / Forced Vigilance [for] ("after block resolution", the
after-combat window per 2.1) occupy the same window. Does the continuation
happen *inside* that window — occupants ordering freely (spec §2.4.1), so the
blocker may unlock before or after it — or does taking it close the window and
resume the action at once?

**Default applied**: inside the window. The acting Methuselah has the first
opportunity; the continuation is a play in that window (impulse returns to the
acting seat, then rotates); the block loop resumes only when the window
closes. Consequence: an unlocked blocker may attempt to block again after the
continuation (rulings: "the maneuver is provided again if a second block
happens on the same action").

**A** (2026-09-04): Yes.

**Test** — `continuation_keeps_declines` scripts this order.

### 4.2 Mixed normal and aggravated damage: which points go unprevented

Spec §4.5.2: "where both kinds are inflicted, the preventing minion chooses
which points go unprevented." With a ready vampire, preventing aggravated
first is never worse (aggravated always wounds; normal wounds only past the
blood), so the engine allocated prevention to aggravated first automatically
and offered no choice. Is there a case where the choice matters?

**A** (2026-09-04): Yes — one might *want* to go to torpor with no blood
lost. Super niche, but why not.

**Applied**: a prevention that covers only part of mixed damage opens a
`CombatPreventionAllocation` decision for the preventer: how many aggravated
points it covers (`SetX`, in the feasible range); a full cover or a single
kind needs no choice. Non-aggravated-only prevention (Soak) never touches the
aggravated points.

**Test** — `partial_prevention_of_mixed_damage_is_the_preventers_allocation`
(unit, `combat.rs`).

### 4.3 Ineffective strikes are offered

The mask offers the hand strike at long range (legal, ineffective) and a
dodge against no opposing strike, consistent with spec §1.6.5 ("futility is no
bar") and §4.6.4. Prevention, by contrast, is masked out with no damage to
prevent (§4.5.1). Fine as a mask policy?

**A** (2026-09-04): OK, although "no strike" is rare — a strike must be
chosen, defaulting to the hand strike; it arises from a committed strike that
can no longer be performed, or from a card that lets the minion not strike.
Careful with these masks: playing an ineffective card just to cycle it is
very common, so do not mask that possibility.

**Applied** (principle, DESIGN §4.5): the mask removes only what the rules or
the card forbid (alternation, limits, "cannot be played with no damage to
prevent", "only when needed"); a legal play with no effect stays in the mask —
cycling is a purpose.

### 4.4 Limited additional strikes before the normal pair

Spec §4.3.5 (Quickness): the card granting a limited additional strike may be
played before or after the normal pair. The engine opens `COMBAT_ADDITIONAL_STRIKE`
in the before-strikes and before-resolution windows as well as in the
additional-strikes step, for every card with that window (Blur, Side Strike
[CEL], Pursuit [CEL]…). Is the ruling general, or Quickness-specific?

**A** (2026-09-04): Yes, general.

### 4.5 Combat cards by a combatant no longer ready _(answered 2026-09-06)_

Taste of Vitae and Disarm print "not usable by a vampire being burned or going
to torpor", which suggests the end-of-round window is otherwise open to such a
vampire. The engine lets a combatant that is no longer ready play end-of-round
combat cards unless the card excludes it, and no other combat window. Right?

**Answer (2026-09-06)** — Yes.

## Batch 5 — politics (referendum machine, slice 4)

Modeling calls made while implementing the referendum (tests/politics.rs);
each is implemented as stated and awaits confirmation or correction.

### 5.1 Votes gained after casting

Bewitching Oration played by a vampire that has already cast: the extra
votes join its cast on the same side (all votes of a source agree), with no
new commitment. Before casting they fold into the source.

**A** (2026-09-04): ok.

### 5.2 Political action cards burned for a vote in a blood hunt

The rulebook's "burn a political action card for 1 vote" is offered in the
blood hunt too (a vote source, not a card play; the blood hunt is "handled
like any other referendum"), one per Methuselah per referendum. DESIGN's
"vote sources only" is read as including it.

**A** (2026-09-04): yes.

### 5.3 Card plays during polling reopen the rotation

A card played in the polling step is a commitment of the polling rotation:
it reopens a response opportunity for every other Methuselah before the
tally, like a vote. Windows with one option (a seat with nothing left to
cast or play) elide.

**A** (2026-09-04): yes.

### 5.4 The calling card's own vote

The political action card that called the referendum provides 1 vote for
the acting Methuselah, cast either way, as an independent ballot; it
consumes that Methuselah's one political-action-card vote.

**A** (2026-09-04): yes.

### 5.5 Delaying Tactics / Telepathic Vote Counting: "discard down afterward"

An immediate decision (`DiscardDown`): the acting Methuselah discards to
hand size right after the card returns, not in the discard phase. The
action still ends successful (no referendum effect), the after-resolution
window opens (Voter Captivation needs a passed referendum, so nothing), and
the acting vampire has taken its political action for the turn.

**A** (2026-09-04): yes.

### 5.6 Parity Shift shortfall

When the chosen Methuselah has less pool than the 3 allocated at
resolution, the caller re-allocates what there is, each recipient capped at
its term amount (LSJ 20010606), as a stepwise decision.

**A** (2026-09-04): yes.

### 5.7 Banishment and cards on the vampire

The banished vampire goes to its controller's uncontrolled region with its
blood as counters; cards on it leave play and do **not** come back when it
is re-influenced (known gap — nothing in the pool re-attaches yet).

**A** (2026-09-04): no — they leave play but stay face down on the banished
vampire, and come back with it when it is re-influenced.

**Applied**: the uncontrolled slot remembers the arena entry
(`UncontrolledSlot::banished`); influencing it out restores that entry — its
history (titles lost, contests) and the cards on it, face up and unlocked —
rather than creating a new one.

**Test** — `banishment_sends_a_younger_vampire_to_the_uncontrolled_region`
(tests/politics.rs).

### 5.8 The after-resolution window

Opened after every successful action, over the action's order, elided when
nothing can be played (Voter Captivation, later Freak Drive); for a
political action it follows the referendum (spec §3.7.5.1). Not opened
after a diablerie (the blood hunt follows the action directly).

**A** (2026-09-04): after a diablerie too.

**Applied**: the blood hunt is inside the action (spec §3.7.8.1) and the
after-resolution window follows it, then the action ends; a torpor-block
diablerie's blood hunt ends the blocked action with no such window.

### 5.9 Acting minion no longer ready at resolution

A political action whose acting vampire is no longer ready when the action
would resolve (torpor from the block's combat, then a Form of Mist
continuation) ends with no referendum (Yawp Court ruling). The engine
still pays the card's cost at that resolution; whether it should is
unstated.

**A** (2026-09-04): yes (the cost is paid).

## Batch 6 — cards in play (slice 5: riders, granted actions)

Modeling calls made while implementing Anarch Revolt (tests/in_play.rs):
effects carried by a card in play — mandatory ones fired at a hook and
resolved by whichever Methuselah the text names, optional ones offered as
abilities. Each is implemented as stated and awaits confirmation or
correction. (Events, a controlled card type — 20 in the pool, none in a
precon — are P2 and will reuse this shape.)

### 6.1 When "during their unlock phase" is checked

Anarch Revolt's burn fires once the active Methuselah's own unlock-phase
effects are done — unlocking, delayed replacements, the Edge, contest
upkeep (a contested Anarch won or yielded there counts or not) — and
before the impulse leaves them for other players' unlock-phase abilities
(Homunculus), per spec §2.4.4 ("finish the mandatory ones before another
Methuselah gets the impulse"). Spec §5.2.2 / LSJ 20080106 say "checked
when the phase ends"; the two readings differ only if another player's
unlock-phase effect could give the active Methuselah a ready Anarch, which
nothing in the pool does. The active seat's own optional unlock abilities
(Homunculus) are not orderable before the burn in the engine — also
unobservable (blood vs pool). Fine, or should the check sit after the
whole rotation?

**A** (2026-09-06): a mandatory effect must be resolved before the impulse
passes to another player; the active Methuselah orders it before or after
their other unlock-phase effects as they choose.

**Applied**: the unlock-phase abilities window opens with the active
seat's pending mandatory riders as `Choice::ResolveRider` options
alongside their optional abilities, `Pass` masked until all are resolved
(a lone rider elides); the condition is read when the rider resolves; other
seats get the impulse after. `Hook::UnlockPhase`.

**Test** — `anarch_revolt_burns_pool_at_each_unlock_phase_without_a_ready_anarch`,
`anarch_revolt_copies_burn_cumulatively`, `anarch_revolt_ousts_at_unlock_and_the_turn_passes`,
`anarch_revolt_burn_is_ordered_by_the_active_methuselah_before_the_impulse_passes`.

### 6.2 A referendum granted by a card in play

"Vampires can call a referendum to burn this card as a +1 stealth
political action": any Methuselah's ready unlocked vampire (allies
excluded), during their own minion phase; a political action in every
respect (locks, +1 stealth, blockable, one political action per minion per
turn, no card-play cost). The rulebook's "the political action card that
called the referendum provides 1 vote" does not apply: no calling card, so
no card vote for the caller (political action cards burned from hand still
give theirs). Delaying Tactics cancels it with nothing to return to hand,
and the ban on "the same political action this turn" is keyed on the card
in play's name (so no minion of that Methuselah may call Anarch Revolt's
referendum again this turn — any copy). Right?

**A** (2026-09-06): locks, blockable, one political action per minion per
turn — but the stealth is not implied: it must be granted by the card's
text (it most often is). NRA for a card in play is keyed to the *copy*,
not the name; and since it is a political action, the type-level cap
(one bleed, one political action per minion per turn) applies as well.

**A** (2026-09-06, follow-up): the default +1 stealth applies to
political action *cards* only; an action granted by a card in play has no
default, directed or not — only what its text prints.

**Applied**: a granted action's stealth is exactly the ability's printed
`AddStealth` (Anarch Revolt: 1; a "burn this card as a Ⓓ action" on one's
own copy: 0, undirected, blockable by prey and predator). `Minion.acted` records
`ActionId::InPlay(copy)` at resolution (hand cards: `ActionId::Card`, name
identity) and the enumeration masks the same copy; `political_this_turn`
keeps the type cap. The Delaying Tactics ban stays by name (the card's own
"the same political action").

**Test** — `anarch_revolt_referendum_burns_it`,
`anarch_revolt_failed_referendum_keeps_it`,
`anarch_revolt_referendum_canceled_by_delaying_tactics`,
`granted_action_nra_is_per_copy`.

### 6.3 Burning the source

A passed referendum burns the copy whose action was taken (not every copy),
to its owner's ash heap, its layers gone; if that copy already left play
(its controller ousted by a Scorn of Adonis tally burn in the same
referendum), the effect does nothing. Confirm.

**A** (2026-09-06): confirmed.

### 6.4 Fame's timing

"After this vampire goes to torpor, their controller burns 3 pool" resolves
at once inside whatever sent the vampire to torpor (mid-combat, before the
after-combat window), and stacks with Tension in the Ranks in the
controller's chosen order.

**A** (2026-09-06): correct.

### 6.5 Hunting grounds

"A vampire can gain blood from only one hunting ground each turn" is per
vampire: two grounds may feed two vampires; which vampire is the
controller's choice as the ability resolves.

**A** (2026-09-06): correct.

### 6.6 Blood Doll / Vessel reuse

"During their master phase, this vampire's controller can move 1 blood
from this vampire to their pool or from their pool to this vampire" (Vessel:
unlock phase). Spec §1.5.4: an ability is reusable unless its text limits
it; KRCG lists no ruling. Once per phase, or repeatable within the phase
(three uses move 3 blood)?

**A** (2026-09-06): once. That is the whole "during X, do Y" template: a
card text saying "during" always means "only once per" that X.

**Applied**: an in-play ability offered in a phase window
(`Window::is_phase`) is once per that phase per copy —
`GameState.phase_uses`, cleared at each phase start — no per-card limit
needed.

**Test** — `blood_doll_moves_blood_either_way_once_per_master_phase`,
`vessel_on_any_vampire_burns_any_blood_doll_and_is_the_controllers_to_use`.

### 6.7 Vessel's Blood Doll burn

"Put this card on a vampire and you can burn a card named Blood Doll (if
any)": any Blood Doll in play, or only one on the target vampire?

**A** (2026-09-06): any in play — "card text" in rulings terms: the card
does what it says.

**A** (2026-09-06, on the timing): the choice is announced when the card
is played, like Villein's amount and any target — the as-played (cancel)
window already knows it — and the burn lands as part of resolution; it is
not an after-resolution trigger.

**Applied**: `Terms::BurnNamedInPlay(name)` chosen in the play's terms
window (`Choice::BurnInPlay` / `Pass`, elided when no copy is in play,
any controller's), `Effect::BurnChosenInPlay` at resolution.

### 6.8 A granted action on one's own card

"Minions can burn this card as a Ⓓ action" (Creeping Sabotage): for the
controller's own minion the action is undirected (prey and predator may
block), for another Methuselah's it is directed at the controller; in both
cases the stealth is only what the card prints — none here (6.2).

**Test** — `creeping_sabotage_burn_action_is_directed_at_its_controller`.

### 6.9 Every remaining Methuselah ousted at once _(open)_

Random play reaches it (a "burns 1 pool for each minion" referendum with
everyone at low pool; 11 of 200 fuzz games): all remaining Methuselahs
reach 0 pool in one simultaneous batch. The engine ousts them together —
each scores the VP for their prey (an ousted predator included, spec
§6.5.4), nobody gets the 6 pool, and the game ends with no game-win VP
since nobody survives. Right? (Tournament rules 3.7: is there a game win
at all in that case?)

**A** (2026-09-06): legal, and it happens. Every Methuselah still in play
at that moment scores the VP for their ousted prey, ousted themselves or
not; the 6 pool goes only to a predator who is not ousted. A game win is
still possible: a Methuselah with at least 2 VP and strictly more VP than
any other has the game win, ousted or not — a 3-VP player ousted by their
last opponent (who scores 2: the oust and last-standing) still wins.
Draws are legal results (two players at 2 VP; four or five all at 1 VP).

**Applied**: `oust_batch` was already scoring each predator and paying the
6 pool to surviving predators only; `GameState::game_win()` derives the
game win from the VPs alone (≥ 2 VP and strictly more than every other
Methuselah, ousted or not), `None` on a draw.

## Batch 7 — library-card contests, hunting grounds (slice 5c)

### 7.1 Self-contest while your own copy is contested

"You cannot voluntarily contest cards with yourself": the engine masks a
unique card's play whenever a copy is in play under the same Methuselah —
also while that copy is *contested* (out of play, not controlled, but
playing another would contest with yourself). Right?

**Test** — `unique_card_is_masked_while_a_copy_is_in_play_under_the_same_seat`.

**Answer (2026-09-06)** — Yes: it is self-contesting, so forbidden. A
contested card is out of play but keeps contesting incoming copies.
Implemented as stated (`present()` excludes contested copies from every
effect; the self-contest mask counts them).

### 7.2 Hunting ground at capacity

"A ready vampire you control can gain 1 blood": a vampire at capacity is
still offered (DESIGN §4.5 mask policy: legal no-effect plays stay
offered), and the pick counts as that vampire's one hunting ground this
turn. Fine?

**Test** — `hunting_ground_at_capacity_is_a_legal_pick_that_still_feeds`.

**Answer (2026-09-06)** — Fine either way; the branch may be cut if it
matters (it brings nothing — no card cycles from hand). Kept offered for
now (§4.5); prune later if it costs the policy.

### 7.3 A contested card's layers

A contested card's continuous effects are suspended, not ended (spec
§1.13.3 / §6.4.3): a contested Guardian Angel gives no intercept and
its host cannot prevent with it; both resume when the contest resolves
in its favor. Implemented as stated (`GameState::layer_active`).

### 7.4 Carfax Abbey's second pick

"A ready Anarch you control can gain 1 blood, and, if you control a ready
baron, another ready Anarch you control can gain 1 blood as well": one
"during your unlock phase" clause with two picks. The engine models them
as two independent once-per-phase uses (`Limit::PerPhaseEach`): the
second may be taken without the first (with a ready baron and a single
Anarch, only the second is offered, feeding that Anarch). Should the
second pick require the first to have been used ("another … as well")?

**Answer (2026-09-06)** — No, the independent reading is fine.

### 7.5 Control of a card on another Methuselah's minion

Fame goes on any ready vampire: who controls A's Fame on B's vampire,
and what becomes of it when either Methuselah is ousted?

**Answer (2026-09-06)** — A *minion* card on a minion is controlled by
the minion's controller; a *master* card like Fame stays controlled by
the player who played it, so it is removed when that player is ousted.
A card on a removed vampire is burned (to its owner's ash heap, not
removed). Applied: `GameState::card_controller` (master → player, minion
card → host's controller) keys the contest paths, so A's Fame on B's
vampire contests B's own Fame as usual; `remove_seat` removes what the
ousted Methuselah controlled and burns other players' cards on their
minions.

### 7.6 Smiling Jack's blood sources

"Burns 1 blood from a vampire they control": the engine offers every
vampire the Methuselah controls that has blood — in torpor included, a
torpid vampire being a vampire they control — and never an empty one
(LSJ 19971212). Right on the torpor half? And may one vampire pay for
several counters (the engine allows it: each unit is a fresh pick), or
does ANK 20181006's "different vampires" mean one blood per vampire?

**Answer (2026-09-06)** — Yes and yes: torpid vampires are sources, and
one vampire may pay for several counters. Implemented as stated.

### 7.7 Untimed "you can lock this card" abilities

The Barrens and Dreams of the Sphinx carry no timing: the engine offers
them at every decision point where their controller holds the impulse —
their own master, minion, influence and discard phases, every seat's
unlock-phase rotation, every action and combat window (as announced,
block loop, strike choice, prevention, end of round, after combat, after
resolution), the polling step (as a commitment, reopening the rotation)
— and never in the as-played window (LSJ 20061207), nor inside a choice
that is part of a resolution: the play terms and rider payments, the
Edge decision at the start of the unlock phase (the Edge can also be
burned later, among the unlock effects), the torpor-block diablerie
choice, the rescue cost split, the discard-down after a canceled
referendum. Anything to trim or add (the influence phase? the strike
choice)?
Dreams' third lock: its effect (the +2 hand size, the blood) lands, then
the card burns at 3 counters. Right?

**Tests** — `the_barrens_discards_a_chosen_card_at_any_impulse`,
`the_barrens_is_not_usable_in_the_as_played_window`,
`dreams_of_the_sphinx_blood_to_uncontrolled_and_burn_at_three_counters`,
`untimed_ability_offered_alongside_a_forced_hunt`.

**Answer (2026-09-06)** — The window set and the third-lock ordering are
right, with two corrections: the Edge's pool is itself a normal unlock
effect usable anytime during the unlock phase (not a separate opening
decision — offered among the unlock effects now; whether contest upkeep
is likewise freely ordered is asked in 7.8), and an untimed ability can
be used *before* a forced hunt (The Coven's blood sparing the hunt) —
applied: the forced hunt is offered alongside them.

### 7.8 Contest upkeep among the unlock effects

Is paying or yielding a contest freely ordered among the active
Methuselah's other unlock-phase effects (the Edge, mandatory riders,
abilities), as the riders are (6.1)? If so the engine folds them all
into one ordered window and drops its opening Edge decision (DESIGN
§10.7 revised).

**Answer (2026-09-06)** — Yes. The difference: contest upkeep is a
mandatory decision (pay or yield), so the impulse cannot move on until
it is made, whereas the Edge's pool is optional — if the impulse is
passed and nobody takes it, the opportunity is lost. Unlocking cards
happens at the beginning of the phase and is not ordered. Applied: one
unlock effects window (`Flow::Abilities`, label `UnlockAbilities`) —
the active Methuselah's pending contests and riders as mandatory
choices (Pass masked until none is left), the Edge and the abilities as
optional ones, offered whenever the active seat holds the impulse; the
opening Edge window is gone; contests with no rival left and forced
yields settle at once. Unlocking stays in `start_turn`.

### 7.9 The impulse in the active Methuselah's other phases

The Coven's ruling (LSJ 20100723) has the predator "get the opportunity
to play effects" in the controller's discard phase, and take control
then. The engine's master, minion, influence and discard phases are
offer windows for the active Methuselah alone: the others hold no
impulse there, so their untimed abilities (The Barrens) are offered only
during actions, referendums and unlock phases. Do the other Methuselahs
get the impulse in each of the active player's phases (after the active
player passes, before the phase ends), as in the unlock phase? If so
each phase ends on a rotation, and The Coven moves when the predator
holds it; the engine moves it as the controller passes out of the phase.

**Answer (2026-09-06)** — Yes. Applied: every phase of the active
Methuselah's turn ends on an ordered rotation over the other seats
(`WindowLabel::PhaseImpulse`): their mandatory effects first (The
Coven's transfer, taken by the predator as they get the impulse), then
their untimed abilities; an effect returns the impulse to the active
Methuselah in their phase; the phase ends once everyone passes.

### 7.10 Constant Revolution's random discards

LSJ 20090618: "the cards are all discarded at once (no replace); one
cannot discard more cards than they have in hand, the rest must be
pool". The engine has the Methuselah announce the card count (at most
the hand), burns that many at random at once, takes the rest from the
pool, then draws the hand back up. Is "no replace" only "not one at a
time" (so the hand is refilled afterward, as modeled), or are those
cards not replaced at all?

**Test** — `constant_revolution_others_burn_pool_or_random_cards`.

**Answer (2026-09-06)** — "No replace" means not one at a time, as
modeled: the cards go at once, then the hand is drawn back up.

## Batch 8 — Cards & effects (rulings harness, replacement, events)

### 8.1 Bonding [DOM] when the stealth is not needed

Ruling [TOM 19951109] on Bonding: "[DOM] Cannot be used if you do not
need the stealth at the time you play it." The V5 printing folds the
stealth into the superior mode ("+1 stealth and +1 bleed"), and the
engine offers that mode for its bleed half with no block attempt pending
(rules-qa 3.2's "only when needed" applies to the stealth alone; test
`bonding_superior_is_playable_for_its_bleed_half`). Does the ruling
stand for the V5 card — the [DOM] mode only once stealth is needed, so a
bleed bonus before any attempt is the [dom] mode's alone — or is the
combined effect playable for its bleed?

**Test** — `bonding_superior_needs_the_stealth_to_be_needed`.

**Answer (2026-09-07)** — The ruling applies: the superior mode provides
the stealth and is not playable unless a block attempt matches the
current stealth with equal intercept. Applied: `Requirement::StealthNeeded`
on Bonding's [DOM] mode (the [dom] bleed bonus alone is playable before an
attempt).

### 8.2 Discarding down when a hand-size bonus ends

Slice 5g let the cards drawn under Dreams of the Sphinx's "+2 hand size
until the end of the turn" stay once the bonus ended, citing ANK 20180318
— which only says that cards not replaced count against the hand size.
The rulebook ("Drawing Cards": "whenever an effect changes your hand size
or adds or removes cards from your hand, immediately discard down to or
draw up to match your hand size") says otherwise, and the engine now
discards down at once whenever a hand is over size: after the discard
phase for Dreams of the Sphinx (LSJ 20100812 still holds: no discard-down
during the phase), and for the controller of an Elder Library that
becomes contested (its +1 is gone, spec §1.13.3). Is that right?

**Tests** — `dreams_of_the_sphinx_hand_size_lock_lasts_the_turn`,
`dreams_of_the_sphinx_bonus_lasts_through_the_discard_phase`,
`elder_library_enters_contest_without_modifying_the_hand_size`.

**Answer (2026-09-13)** — Yes, both: the hand is discarded down at once
when Dreams of the Sphinx's bonus ends, and when an Elder Library becomes
contested.

### 8.3 What Visit from the Capuchin counts as replacing a card

"Each time you would replace a card other than this card, instead burn 1
counter from this card." The engine burns a counter for every card the
rules replace, whatever took it out of hand (spec §6.8.3): a card played
or canceled, the discard phase action, a political action card burned for
a ballot, The Barrens' and Tension in the Ranks' discards, Constant
Revolution's random burns, and a delayed replacement when it comes due
(Old Friends at the unlock phase). A draw-up after a hand-size increase
(Dreams of the Sphinx) is not a replacement. Two points: (a) do The
Barrens' "discard a card (draw up afterward)" and the refill after
Constant Revolution's random burns count as replacing cards? (b) With an
empty library, is a counter still burned (the engine: yes — the
replacement itself is what the Capuchin replaces)?

**Tests** — `visit_from_the_capuchin_burns_a_counter_instead_of_each_replacement`,
`visit_from_the_capuchin_burns_nothing_with_an_empty_library`.

**Answer (2026-09-13)** — (a) Yes: The Barrens' "draw up afterward" and
Constant Revolution's refill are replacements. (b) No: with an empty
library no card is replaced, so no counter is burned. Applied:
`Engine::replace` offers nothing to the `WouldReplace` riders once the
library is empty.

### 8.4 Mirror Walk: the discard phase it waits for, the blocked end

"Do not replace until your next discard phase": played in its
Methuselah's own minion phase, the engine replaces it as that same turn's
discard phase begins. [THA] "end the action before block resolution": the
engine records the action as blocked (its no-repeat-action identity kept,
the blocker locked by the effect), with no combat and nothing more played
on it. Is that right?

**Tests** — `mirror_walk_superior_ends_the_blocked_action_before_block_resolution`,
`mirror_walk_on_a_blocked_leave_torpor_gives_no_diablerie_opportunity`.

**Answer (2026-09-13)** — Correct.

## Batch 9 — Equipment (slice 8)

### 9.1 Text naming the bearer's clan or sect, on a bearer without it

ANK 20180719-3 lets a minion take a clan equipment from another minion
without meeting its requirement. Treasured Samadji: "This Ravnos gets +1
bleed. Once each combat, this Ravnos can strike: dodge." The engine reads
the named trait on the current bearer: a non-Ravnos holding it gets
neither. Likewise Stolen Police Cruiser's "This Anarch gets +1 bleed" (it
has no requirement, so a non-Anarch can equip it from hand), its younger
vampires' -1 intercept and its lock when burned; Shilmulo Tarot's "While
this Ravnos is ready". Is that right?

**Tests** — `treasured_samadji_taken_by_a_non_ravnos_gives_nothing`,
`stolen_police_cruiser_anarch_bleeds_one_more_and_younger_vampires_intercept_less`.

**Answer (2026-09-13)** — Right.

### 9.2 Equipping from another minion; one vehicle

The engine offers the rulebook equip action from any other minion the
Methuselah controls, ready or in torpor, taking one or more of its
equipment announced with the action; a blocked action leaves them. May the
equipment come from a minion in torpor? And "a minion can have only one
vehicle": a second vehicle is simply not offered, from hand or from
another minion (nor two vehicles in one equip action). Right?

**Tests** — `a_minion_can_have_only_one_vehicle`,
`a_blocked_equip_from_another_minion_leaves_the_equipment`.

**Answer (2026-09-13)** — (a) Yes, from a minion in torpor too. (b) Right.

### 9.3 Searching for equipment (slice 8c)

Magic of the Smith: the engine would offer, at resolution, every
equipment in the library the vampire can equip — requirements met, not a
second vehicle — or none (a search may find nothing; PIB 20121028: cards
are not shown); a pool cost that ousts is allowed. Should an equipment
whose blood cost the vampire cannot pay be offered? Vast Wealth ("the
first equipment you find"): no choice; if that equipment's requirements
are not met, it cannot be placed or its blood cost is unpayable, it is
burned without cost (TOM 19950407, extended to requirements), and a pool
cost that ousts is paid (RTR 19941109). Right?

**Answer (2026-09-13)** — Magic of the Smith: no, an equipment whose blood
cost the vampire cannot pay is not offered. Vast Wealth: yes to both.

### 9.4 Draw alternatives (slice 8c)

Shilmulo Tarot and Black Market Cache: each card drawn from the library —
a replacement, a draw-up, Heart of Nizchetus' draws — may be taken from
the stored cards instead, a choice per card; with Visit from the Capuchin
in play, the player picks one treatment per replacement (ANK 20200905).
With an empty library no draw happens (8.3b), so the Tarot supplies no
card either. Right?

**Answer (2026-09-13)** — Yes: a choice per card drawn; one treatment
per replacement beside the Capuchin; nothing from an empty library.

### 9.5 Heart of Nizchetus (slice 8c)

"Draw up to 3 cards without discarding and then move the same number of
cards from your hand to the bottom of your library": the number is chosen
as the ability is used (0 to 3), the cards are drawn with no discard-down
in between, then the same number is chosen from hand one at a time, each
placed under the previous one (the last chosen is the bottom card). Is
the order at the bottom the player's choice?

**Answer (2026-09-13)** — Yes.

### 9.6 Bowl of Convergence's burn

"The bearer with superior Auspex [AUS] can burn 1 blood during an action
to get an additional +1 intercept." Once per action, or any number of
times (each an additional +1)? And "during an action": any action the
bearer could block, used while it holds the impulse (a block attempt
pending or not)?

**Answer (2026-09-13)** — "During" in the sentence makes it the "during
X" template (rules-qa 6.6): once per action, not repeatable; and only
while the bearer is attempting to block. Applied: `Limit::PerAction` on
the ability (`ActionCtx.abilities_used`), `InterceptNeeded` (rules-qa 3.2).

**Tests** — `bowl_of_convergence_burns_blood_once_per_action_only_while_attempting_to_block`.

## Batch 10 — Weapons (slice 8b)

### 10.1 A gun's maneuver commits the strike

.44 Magnum, AK-47, Assault Rifle: "Strike: 2R damage, with 1 optional
maneuver each combat." The engine follows the rulebook ("Determine Range":
using the maneuver from a strike card or a weapon chooses the strike;
"Choose Strike": no other initial strike that round) and LSJ 20090528-2-2 /
PIB 20141026: once the maneuver is used, only that gun's strike is offered
for the round's initial strike — none at all if an effect then bars it
(Immortal Grapple played after, spec §4.3.1), no hand strike instead — and
no second strike card or weapon may maneuver that round; additional strikes
are free. The allowance tracks the gun (LSJ 19980302-2). Spec §4.10.1 reads
"a minion who maneuvers with a gun may still strike with that gun that
round" and lists a maneuver among weapon functions that never commit: is
that about maneuvers printed apart from the strike, the engine reading
right for these three guns?

**Answer (2026-09-14)** — Right.

**Tests** — `a_gun_maneuver_commits_the_initial_strike_to_that_gun`,
`a_committed_strike_barred_by_immortal_grapple_leaves_no_strike`.

### 10.2 AK-47's additional strike

"After the bearer strikes with this gun, they get 1 optional additional
strike (limited), only usable to strike with this gun, this round." The
engine grants it once the gun's strike has resolved, dodged or not (not
when a strike: combat ends resolved first, spec §4.7.1); it is the bearer's
one limited source that round, so none if a limited additional strike
(Blur) was already used, and Blur is no longer playable after it; in the
additional pair the bearer chooses the gun's strike or no strike. Right?

**Answer (2026-09-14)** — Right.

**Tests** — `ak_47_gives_one_optional_additional_strike_only_with_the_gun`.

### 10.3 Kevlar Vest

"Once each combat, the bearer can prevent 2 damage from gun strikes or 1
damage from any other source. A minion can have only one Kevlar Vest." LSJ
20081210 lets it prevent the damage of two separate gun strikes doing 1
each, and optional prevention carries over (ANK 20200318, LSJ 20001114), so
the reading is an allowance for the rest of the combat, into later rounds,
spent at whichever prevention steps the bearer chooses. (a) Is the branch —
gun or other — fixed at the first use? (b) Can leftover gun points prevent
non-gun damage, or a use against gun damage take only 1 point and keep 1?
(c) Is a gun's damage delivered other than by its strike (none in the pool
yet) "damage from gun strikes"?

**Answer (2026-09-14)** — Part of the prevention may be used in one round
or exchange and the rest, if any, later in the combat. Gun or non-gun is
chosen once each combat; leftover points for gun strikes prevent nothing
else. Gun damage not from the strike is not "damage from gun strikes".
Applied: the gun branch prevents one point a use while points are left
(`CombatCtx.gun_prevention_left`), `Damage.gun` counts gun-strike points;
using either branch spends the other.

**Tests** — `kevlar_vest_spends_its_gun_points_over_separate_gun_strikes`,
`kevlar_vest_chooses_gun_or_other_once_each_combat`.

### 10.4 Righteous Blade

"Strike: strength+1 damage, with 1 optional press, only usable to continue
combat, each combat." (a) Is the press had when the Blade's strike is
chosen — usable though the strike is dodged, or never resolves because a
strike: combat ends resolved first — or only once it resolves? (b) "Each
combat": one such press per combat per Blade, so the Blade's strike in a
second round gives none?

**Answer (2026-09-14)** — (a) Kept if the strike does not resolve or is
dodged. (b) Once per combat ("each combat"). Applied: the press is banked
as the strike is chosen (`Side.continue_presses`), once per Blade per
combat.

**Tests** — `righteous_blade_press_survives_a_dodge_once_each_combat`.

### 10.5 Sniper Rifle

"If the bearer blocks, they can, before range is determined, set the range
for the first round of the resulting combat to long, and their initial
strike that round must be with this weapon." The engine offers it to the
blocker in the before-range step of the first round, only while no other
effect has set that round's range; used, it skips the determine range step
(RTR 19970630, ANK 20180720) and commits the initial strike to the rifle.
Right?

**Answer (2026-09-14)** — Right; and the option is offered only if the
bearer had the weapon when blocking — not after equipping it with
Concealed Weapon or a similar effect once the block happened. Applied:
`CombatCtx.held_at_block`, `Requirement::SourceHeldWhenBlocking` (LSJ
20021028 stays `blocked:` until an effect equips during combat).

**Tests** — `sniper_rifle_sets_long_range_skipping_determine_range_and_commits_the_strike`.

## Batch 11 — Stored cards and draw alternatives (slice 8c-ii)

### 11.1 Heart of Nizchetus with a short library

"You can draw up to 3 cards without discarding and then move the same
number of cards from your hand to the bottom of your library." The number
is chosen as the ability is used (1 to 3), the cards drawn one at a time
(each may be a draw alternative, 9.4). With fewer cards in the library
than the number chosen, is "the same number" the number actually drawn?

**Answer (2026-09-14)** — The number is picked before drawing and the
cards are drawn at once: the number picked, capped by the library's size;
then that same number is moved to the bottom. Applied: the draw count is
fixed as the ability resolves (`min(X, library)`), each of those draws
still a draw alternative's choice (9.4); the bottom move follows with no
discard-down in between.

### 11.2 Shilmulo Tarot's stored cards

On equip, "move the top 2 cards of your library to this equipment": with
fewer than 2 cards, the ones there are. "During your unlock phase, you
can move the top card of your library to this equipment": once per unlock
phase per Tarot (the "during" template, 6.6), whoever bears it. Right?

**Answer (2026-09-14)** — Right; and the Tarot is unique, so a Methuselah
has only one in play. Applied: `Effect::StoreTopCards` on equip and in
the unlock ability; the self-contest mask also bars a search from
equipping a unique the seat already has in play (rules-qa 7.1).

### 11.3 Black Market Cache

"Search your library (shuffle afterward) for up to three non-unique
equipment cards and put them on this location": chosen as the card
resolves, one at a time, 0 to 3. "If this location has no cards on it,
burn it": checked whenever it is empty — at once after a search finding
none, and as its last card is drawn. Right?

**Answer (2026-09-14)** — Right.

**Tests** — `black_market_cache_stores_non_unique_equipment_offered_for_each_draw`,
`black_market_cache_finding_nothing_is_burned_at_once`,
`black_market_cache_is_burned_as_its_last_card_is_drawn`,
`shilmulo_tarot_moves_the_top_card_once_each_unlock_phase`,
`heart_of_nizchetus_draws_the_number_picked_capped_by_the_library_then_as_many_to_the_bottom`.

## Batch 12 — Readings of slice 8c-ii

### 12.1 Visit from the Capuchin beside a draw alternative

With the Capuchin and a Black Market Cache holding cards, each replacement
offers the Capuchin's counter or a card from the Cache — not the top card
of the library, since the Capuchin's "instead" applies unless the other
treatment is chosen (ANK 20200905). Right?

### 12.2 Shilmulo Tarot taken by another minion

"Move the top 2 cards of your library to this equipment" happens as the
Tarot is equipped from hand or by a search (LSJ 20090506). Taken from
another minion with the rulebook equip action, the engine moves no new
cards (the card is moved, not played). Right?

### 12.3 A search finding a unique already in play

Magic of the Smith does not offer a unique equipment its Methuselah
already has in play (the self-contest rule, 7.1), and Vast Wealth's first
find of one is burned as an equipment it cannot place. Right?

### 12.4 Black Market Cache's own replacement

The search resolves with the card, before its replacement is drawn, so
that replacement may already be one of the cards just put on it. Right?

### 12.5 Shilmulo Tarot on a minion that is not Ravnos

A minion that is not Ravnos may take the Tarot from another minion (ANK
20180719-3). "While this Ravnos is ready" is read on the current bearer,
as Treasured Samadji's "this Ravnos" (9.1): such a bearer's Methuselah
draws nothing from it, while the unlock-phase move ("you can move the top
card of your library to this equipment") still applies. Right?

**Answer (2026-09-15)** — 12.4: no — the Cache is replaced before its
search, so its replacement is never one of the cards the search puts on
it. 12.1, 12.2, 12.3, 12.5: right. Applied: `resolve_play` queues the
search once the card is replaced.

**Tests** — `visit_from_the_capuchin_or_a_draw_alternative_is_one_treatment_per_replacement`,
`black_market_cache_is_replaced_before_storing_non_unique_equipment`,
`shilmulo_tarot_offers_its_cards_only_while_a_ready_ravnos_bears_it`.

## Batch 13 — Printed vampire abilities (slice 9)

### 13.1 A vampire whose text has two parts

Every vampire is dealt today with none of its printed text. Several pair
a plain bonus with another ability: Theo Bell (+1 strength, and entering
combat as a Ⓓ action), Kasim Bayar, Marchesa Liliana, Mora, Nonu Dis,
Sreelekha (+1 bleed, and another ability), Faruq (+1 strength, and his
corruption rule). For library cards, a card with a mode dropped is not
implemented. For vampires, which do you want: (a) a vampire gets its
text only once all of it is implemented, staying textless until then;
or (b) the plain bonuses land on every vampire first and the other
abilities follow, since the vampire is in play either way? Slice 9a
takes, meanwhile, only vampires whose whole text is plain stat bonuses
(bleed, stealth, intercept, strength, votes, hand size).

### 13.2 Readings of the plain bonuses (slice 9a)

- A printed bonus is read live on the vampire while it is in play, in
  torpor included (LSJ 20011022), and not while it is contested.
- "+1 strength" adds to the strength of 1 or to a strength an effect
  sets (Torn Signpost); "in combat with X" reads the opposing minion.
- "Intercept against X" (Azucena, Bret Stryker) reads the acting minion;
  "against titled vampires" reads the effective title, so a contested
  or yielded title does not count; a primogen is titled.
- "During directed / non-directed actions" (Castellan, The Dowager,
  Ariane) reads the action performed, whoever performs it.
- Hesha Ruhadze counts the unique equipment on him that are not
  contested; Neserian's "control a location" counts uncontested
  locations, hunting grounds included; Branimira reads the pool as the
  intercept is compared.
- "While X is ready" (André the Manipulator, Carmelita Neillson,
  Weirich Waldburg) is read live, so a bonus is lost as the vampire goes
  to torpor, and a hand grown over size is discarded down at once
  (8.2).

Right?

**Answer (2026-09-15)** — 13.1: an implementation order question only:
every vampire must end with all its abilities (full effect
capabilities); the order is the implementer's call. Applied: a vampire
may land with part of its text, the rest named in
`VampireDef.incomplete` until it is implemented. 13.2: yes to all.

**Tests** — `tests/vampires.rs`.

## Batch 14 — Printed combat abilities (slice 9b)

### 14.1 Adrino Manauara's mandatory press

"Adrino gets 1 press (mandatory) each combat, only usable to continue
combat." Mandatory effects apply at their timing point, before their
controller passes (spec §1.1.2). The reading I would build: at the first
press step of each combat where no press to continue stands and both
combatants are ready, Adrino's controller must use it before passing.
Three points only you can settle: (a) may Adrino's controller play a
press card to continue first, so the mandatory press is not used that
round; (b) if the opponent already pressed to continue, the press is not
usable — does it wait for a later round of the same combat, or is it
lost; (c) if the round ended early (a combatant not ready), is it simply
not used? Adrino is held back until the answer.

### 14.2 Readings of the other combat abilities

- Abaddon, Saku Pihlajamäki: "1 optional maneuver / press each combat" is
  one use over the whole combat, in any round, under the no-two-in-a-row
  rule of maneuvers and presses.
- Flávio Gonçalves, Phaibun: "once each combat, … strike: dodge" is chosen
  as their strike; Immortal Grapple bars it as any dodge. Phaibun's card
  is discarded at random as he chooses the strike, then replaced; not
  offered with an empty hand.
- Opikun: usable in torpor (LSJ 20011022), and when he is himself the
  vampire in combat; the blood is burned as he uses it, only while
  non-aggravated damage is pending on that vampire in its prevention
  step; it prevents non-aggravated damage only (LSJ 20040812-2). Every
  damage pending at that step is strike damage in the engine today.
- Rowena: 1 non-aggravated damage once each round, in her prevention
  step.
- Kasim Bayar: the political action card is picked as he uses the
  ability, then replaced; the +2 strength counts in every round of that
  combat and adds to a strength an effect sets.
- Chun Hei: usable where additional strike cards are; "does not count
  against the limit": a limited additional strike card still fits that
  round.
- Noluthando: +1 damage on each of his ranged strikes (a weapon's or a
  card's) at either range, not on hand or melee strikes; on a gun it is
  gun-strike damage (Kevlar Vest).

Right?

**Answer (2026-09-15)** — 14.2: yes to all. 14.1 open: Adrino is held
back.

**Tests** — `tests/vampires.rs`.

## Batch 15 — Printed abilities outside combat (slice 9c-i)

### 15.1 Gathii's reveal

"During an action Gathii performs, you can reveal the top card of your
library. If it is a master card, he burns 1 blood; otherwise, he gets +1
stealth." The revealed card stays on top and the text prints no periodicity
clause, so by advanced rules §1.5.4 the ability could be used again,
revealing the same card each time: a non-master card on top would give +1
stealth at every use while stealth is needed (ANK 20231031: usable before
stealth is needed, the stealth then not gained). (a) Repeatable in that way,
or once per action? (b) Before stealth is needed, is the only effect the
blood burned on a master card? Gathii is held back until the answer.

### 15.2 Readings of the abilities built

- Referendum abilities (Aisha az-Zahra, Alexa Draper, Yewon Ong, Queen
  Anne): once per referendum, usable while locked (advanced rules §1.2.1,
  ANK 20220705); those printing no condition are offered in torpor too
  (§3.7.6.4), although a vampire in torpor casts no votes. "Any referendum"
  includes blood hunts. Aisha counts every ready Anarch her controller
  controls — herself included, locked ones too — as the ability resolves.
  "A card requiring Dominate" is any library card whose disciplines include
  it, either-or and combo cards included.
- Queen Anne: "a referendum called by Anne" is the referendum of her
  political action, not a blood hunt; the target is any younger Camarilla
  vampire, any Methuselah's, in torpor included, not one already
  abstaining.
- Stealth and intercept printed on vampires are masked as cards are
  (rules-qa 3.2): Abraham DuSable's +1 stealth only while stealth is needed
  on his action, his +1 intercept only while intercept is needed on his own
  block attempt; Daria's lock only while the acting Nosferatu (another
  Methuselah's included) needs stealth; Oskar Anasov's +1 intercept only to
  his controller's vampire attempting to block while it needs intercept;
  Gwenllian Arwyn's -1 intercept only to the vampire attempting to block her
  action while stealth is needed. Oskar's and Gwenllian's last the rest of
  the action (a failed blocker is not offered an unreachable second
  attempt). All repeat while payable (§1.5.4), except Daria's (a lock cost:
  once per unlock). One asymmetry to settle: intercept given this way
  survives a failed attempt, while the engine resets intercept from a
  reaction card (Eyes of Argus) when an attempt fails (rules-qa 1.3 does
  not say which). Does intercept gained during an attempt carry into a new
  attempt by the same minion, for both, or for neither?
- Larissa Moreira: on her own bleed only, before resolution (ANK 20231124),
  repeatable.
- Nonu Dis: once per master phase (ANK 20220617), offered at any point of
  that master phase once a master card was played in it — played as it is
  announced, canceled or not; his target is any ready Ministry vampire his
  controller controls, himself included.
- Mora, the Death Seer: once per discard phase; any library card in the ash
  heap, chosen by name.
- Sreelekha: +1 discard phase action if she is ready as the discard phase
  begins.
- Ayse Dhanial: each use spends a discard phase action, so twice with
  Sreelekha's; not offered at capacity (as the blood shuttles of cards in
  play).
- Kalinda: whenever her controller holds the impulse on their turn, while
  she is locked, in torpor too; not during a combat she is not in (a
  vampire outside the combat is not offered its abilities there, as for the
  host of a card in play).
- Marchesa Liliana is held back: the engine does not keep burned vampires
  in the ash heap yet, and "seven cards in your ash heap" counts them.

Right?

**Answer (2026-09-15)** — "All repeat while payable": no — most use the
"during" template, meaning once during it. The asymmetry: bonus stealth or
intercept survives for the whole action and ends when the action finishes,
in both cases. 15.1 is not answered.
Applied: Abraham DuSable (each mode), Oskar Anasov, Gwenllian Arwyn and
Larissa Moreira once per action, Kalinda once per turn; intercept from a
reaction card (Eyes of Argus) kept by the minion across a failed attempt
too. Read from the template, not answered: Gathii once per action (15.1 (a))
— to confirm with 15.1 (b).
Tests: `intercept_from_a_failed_attempt_carries_into_a_new_one`,
`abraham_dusable_discards_blood_sorcery_cards_for_stealth_or_intercept`,
`larissa_moreira_discards_animalism_cards_for_bleed_during_a_bleed`.

## Batch 16 — Printed abilities after an action (slice 9c-ii)

### 16.1 "After …" with no periodicity

Richard de Worde ("After Richard successfully bleeds, he can look at the
target Methuselah's hand, then he can burn 1 blood to discard 1 card from
it") and Alicia Cortez ("After Alicia performs a successful directed action,
you can discard a political action card to lock a younger vampire …") print
no periodicity clause. Read like the "during" template (15.2): once per
action. Sybren van Oosten's and Queen Anne's unlocks limit themselves (the
vampire must be locked). Right?

### 16.2 Queen Anne's unlock after a diablerie as a blocker

A blocked leave-torpor action ends blocked, with no after-resolution window.
When Anne blocks it and diablerizes the acting vampire (then the blood hunt),
can she unlock — and at which point? Only her own diablerie action is built;
the rest is held.

### 16.3 Looking at a hand

The engine keeps no hidden information yet. Richard de Worde: the use
chooses the card (the choice shows the hand) and burns the blood; looking
without discarding is not offered. The card goes to its owner's ash heap and
is replaced, as any card leaving the hand. Ilonka: the use looks, then her
controller chooses to discard at random or not; offered while she is ready
in the after-resolution window of a successful bleed against her controller,
once each turn. Right?

### 16.4 Other readings

- Aline Gädeke: the acting minion's sect is read after resolution; usable in
  torpor (LSJ 20011022); a fizzled action counts (ANK 20220218); once each
  turn, whoever's turn.
- Alicia Cortez: "directed" as for the bleed, a rescue or diablerie of
  another Methuselah's vampire, and a burn action on their card; the target
  any younger vampire (in torpor too) controlled by the Methuselah the action
  was directed at. Locked ones are not offered as targets (as Kalinda is not
  offered her unlock while unlocked), although the mask policy (rules-qa 4.3)
  keeps legal no-effect plays — which?
- Keegan and "Mother" Anja Giovanni are held back: no implemented action card
  requires a Gangrel, Hecata or Oblivion yet.

Right?

### 16.5 Riders and printed votes

- Roy: "if Roy is unlocked during your discard phase, lock him" applies as
  the discard phase ends (the engine's convention for "during your discard
  phase" effects: as the impulse leaves the active Methuselah), in torpor too
  (LSJ 20011022).
- Cedrick Calhoun: he goes to torpor as the action resolves — before the
  after-resolution window — if the referendum failed or was canceled, while
  he is ready.
- Egidia Arrú: a minion in combat with her leaving the ready region (burned
  or sent to torpor, by damage or anything else during the combat) costs its
  controller 1 pool, even if Egidia falls in the same event (advanced rules
  §6.4.4).
- Marialena: the blood is burned as the block resolves, with the acting
  minion's "if blocked" card riders, before combat or the blocker's
  diablerie choice.
- Jason "Son" Newberry: +2 only on votes he casts against a blood hunt, so a
  Jason with no title votes can still cast 2 against one.
- Styles Margs: the +2 in favor stand as the blood hunt is called on him,
  counted like a vote no one casts (not his, whatever happens to him in the
  polling).

Right?

### 16.6 Held vampires: what each needs

- Evan Klein: "As a minion announces an action directed at Evan, flip a coin;
  if it is tails, the action fails." On tails: does the acting minion still
  lock, does the action count for NRA (and a bleed/political cap), are cost
  and blocks skipped altogether, and is anything played "as the action is
  announced" still possible before the flip?
- Alabástrom: "If Alabástrom is ready during your turn, you can unlock
  another ready Gargoyle you control." Once per turn (the "during" template),
  whichever Gargoyle? Flight is inert for now (no card in the engine pool
  requires it) — fine to land him with it as a bare trait?
- Gnaeus Aemilius waits for out-of-combat damage in another Methuselah's
  unlock phase (a new window shape); Keegan and "Mother" Anja Giovanni for an
  action card requiring their clans; Gathii for 15.1 (b); Marchesa Liliana
  for burned vampires kept in the ash heap.

**Answer (2026-09-15)** — 16.1 yes: once per action. 16.2 yes: Anne can
unlock after a diablerie as the blocker of a leave-torpor action too. 16.3
right. 16.4: keep legal plays with no effect (a cycling opportunity) —
Alicia Cortez may target a locked younger vampire. 16.5 right. 16.6 yes to
all: on Evan Klein's tails the acting minion still locks, the action counts
for NRA and the bleed/political caps, cost and blocks are skipped, and
as-announced plays remain possible before the flip; Alabástrom once per
turn, Flight a bare trait for now.

## Batch 17 — Printed actions (slice 9d)

### 17.1 Readings of the printed actions built

- Sakhar, Dominica: the card is chosen at resolution, any card of the kind
  (Sakhar: every equipment, whether or not he could equip it; Dominica: a
  master card labeled archetype), revealed and moved to hand — or none is
  found; the library is shuffled either way. Hand size is not checked then.
- Seraphina: "a minion you control" includes Seraphina herself and a
  vampire in torpor; a vampire's blood stops at its capacity. Undirected.
- Lenelle: both cards named with the announcement (LSJ 20000222), the ash
  card by card and the hand card by card (copies alike); the action is not
  offered with an empty hand or ash heap. At resolution, if either named
  card has left its zone, the action is still successful and its blood
  paid, with no exchange (spec §3.4.1).
- Elen Kamjian: "if you control a locked minion" counts any minion you
  control, one in torpor included. While it holds and she has not taken
  her bleed this turn, it is the only action any of your minions may
  declare (spec §3.9: mandatory actions first), untimed abilities aside,
  and the minion phase cannot end; with 0 blood the rulebook hunt takes
  over ("unless she must hunt"). Stuck (she bled before it attached), she
  declares no action but may still block and play reactions.

Right?

### 17.2 Two mandatory actions at once

Elen Kamjian's bleed is mandatory while you control a locked minion; the
rulebook hunt of a ready vampire with no blood is mandatory too. When another
of your vampires must hunt while Elen's bleed is attached, the engine
currently has the hunt come first (Elen's bleed is only offered once no
vampire must hunt). Does the rulebook hunt pre-empt, or does the
Methuselah choose the order between the two mandatory actions?

**Answer (2026-09-25)** — 17.1 right. 17.2: the acting Methuselah chooses
the order between mandatory actions. Directedness, beyond minions: an
action is directed at the Methuselah whose hand, library, crypt,
uncontrolled region or controlled card (any, not only a minion: stealing
The Rack is directed) it targets — to cover in the end, not necessarily
now. Actions targeting an ash heap are undirected.

### 17.3 Entering combat as a Ⓓ action (Theo Bell, Barachiel, Dafina
Hanganu, Nathaniel Bordruff, Adisa)

- Target: a ready minion controlled by another Methuselah (none of your
  own, none in torpor), locked or not unless printed (Nathaniel: locked
  vampires; Adisa: younger vampires). The action is directed at its
  controller, who alone may block.
- Unblocked, the combat begins at resolution and is part of the action,
  the acting minion acting first; when it ends, the after-resolution window
  follows (Aline Gädeke, Queen Anne), with no after-combat window and no
  "continue as if unblocked". The target no longer qualifying at resolution
  (unlocked, in torpor, changed hands): successful, no effect.
- Adisa: "successfully hunts" only when the hunt gains blood (advanced rules
  §3.7.2.1) — at full capacity she unlocks as normal.

Right?

**Answer (2026-09-25)** — 17.3 yes, except: the combat of an unblocked rush
(the common name for an action to enter combat, after Bum's Rush) has its
after-combat window like any combat; nothing there can "continue as if
unblocked", since those effects need the action to have been blocked. A
blocked rush can use one, and the continued action then enters combat with
its target at resolution.

### 17.4 Steals and card targets (Apolonia Czarnecki, Darius Shirazi, Saankaláxt, Aniel)

- Own targets: the Ⓓ icon being a reminder (rulebook "Directed and
  Undirected Actions"), Apolonia may steal from a younger vampire of her
  own Methuselah, Darius and Saankaláxt may steal equipment from another
  minion of theirs, Aniel may burn a card on another of his Methuselah's
  ready minions — each then an undirected action. (Unlike 17.3's rushes,
  where a minion does not enter combat with one its controller controls.)
- Apolonia: any younger vampire, one in torpor or with no blood included
  (rulebook "Target": she takes what is there); blood beyond her capacity
  drains off.
- Equipment steals: the equipment changes host only — its controller
  follows, its owner stays; no requirement is checked (moving is not
  playing nor equipping, spec §3.7.3.2). A locquipment (Living Manse) is
  a location once in play, so not stealable (spec §1.3.1) — nor taken by
  the rulebook equip from another minion, which the engine allowed until
  now. A card on a minion is targeted without targeting the minion
  (Evan Klein does not flip).
- Saankaláxt holding a vehicle: another vehicle is not offered (as for the
  rulebook equip from another minion, whose take stays where it is when it
  could no longer be held). Or is the steal offered, the second vehicle
  then burned (as TOM 19950407 for Vast Wealth) or left in place?
- A target gone by resolution (burned, moved to another minion, its host
  now in torpor for Aniel's "ready minion", or a younger vampire no longer
  younger): successful, no effect, the cost paid (spec §3.4.1).
- Darius: "non-weapon" excludes melee weapons ("Melee weapon." is a weapon,
  the generated weapon label missed it until now). No V5 equipment is
  electronic.
- Aniel: "a card requiring a Discipline" is any library card printing a
  discipline requirement (in the implemented pool: Homunculus); the
  corruption half waits for corruption counters.

Right?

**Answer (2026-09-25)** — 17.4 right. Living Manse cannot be stolen as an
equipment, but other cards can steal it as a location. Saankaláxt holding a
vehicle: rightly not offered. What differs from Vast Wealth is whether the
object is known when the action is taken: an action sure to aim at an
illegal target is not allowed at all, while one whose object comes from a
choice or randomness at resolution (Vast Wealth's search) is not illegal by
principle and may be taken, even when the player can tell it will fail.
Camera Phone and Laptop Computer, common legacy electronic equipment, may
join the pool.

---

## Batch 18 — Precon coverage (slice 11)

### 18.1 A canceled card and Parijat's toll _(answered 2026-09-25)_

ANK 20260317: a card forcing a block attempt (Second Tradition: Domain)
pays Parijat's library toll "if the card is not cancelled and after the
replacement card has been drawn". ANK 20260216 (the cancel group): a
canceled non-action card's cost is still paid, additional costs included —
its example, Eagle's Sight's Parijat toll paid though canceled. Which holds?

**Answer (2026-09-25)** — ANK 20260317: a canceled card pays no toll.
**Applied**: the toll is paid as the forcing card resolves; a canceled card
never resolves. **Test** — `a_canceled_forced_attempt_pays_no_parijat_toll`
(`crates/engine/tests/precon.rs`); the ANK 20260216 Parijat row is `n/a`
(superseded).

### 18.2 True Love's Face and a last pool _(answered 2026-09-25)_

Burning one's last pool to cancel True Love's Face would oust its player in
the middle of the play; the cancel is not offered then. **Answer** — fine.

### 18.3 Form of the Bat's equipment _(answered 2026-09-25)_

"Cannot use equipment during the resulting combat" bars the equipment's
abilities; no equipment in the pool has a passive combat effect (every use
is an ability). **Answer** — right: a passive combat bonus hardly exists
without a strike, which is using the equipment.

### 18.4 Conditions read live _(answered 2026-09-25)_

An effect's condition is evaluated throughout, never snapshotted (Aire of
Elation, Protection Racket, The Warrens), and so is whether a limited bleed
counts: Foreshadowing Destruction [DOM] counts only while its target has 9
or fewer pool; played at 10 or more it lingers outside the limit, and if
Conditioning was played meanwhile and the bleed is bounced onto 9 or fewer,
both apply. Night Terrors is the ruling-backed exception (a fixed -X, RTR
20030519). **Answer** — yes; that is what the rulings say.


## Batch 19 — Titles and votes (slice 11e)

### 19.1 Saulot's Guiding Wisdom contests as a unique card _(answered 2026-09-25)_

"Unique archetype. Title." Is a second copy, played by another Methuselah,
contested as the title (paid in blood, glossary "Title") or as a unique
card (paid in pool)?

**Answer (2026-09-25)** — the unique card contests first, so the title does
not. **Applied**: a unique title card entering contested gives its vampire
no title until its contest is over. Face down, neither copy's title is
claimed. A non-unique title card (Toreador Justicar) still contests as its
title, paid in blood. **Test** —
`saulots_guiding_wisdom_contests_as_a_unique_card_not_as_a_title`.

### 19.2 A title card replaces a printed title for good _(answered 2026-09-25)_

A prince given the Toreador Justicar card loses Cape Town for the rest of
the game; once the card is gone, he is untitled. **Answer** — yes.
**Test** — `a_title_card_replaces_a_printed_title_for_good`.

### 19.3 Vote grants from locations are read live _(answered 2026-09-25)_

Elysium: The Palace of Versailles, Power Structure, and Toreador Justicar's
"+1 vote in this referendum" (for every ready Toreador, an Anarch's
included): a vampire that has already cast adds the vote to its cast; one
casting later gets it if it qualifies as it casts. **Answer** — yes.
**Test** — `vote_locations_give_each_vampire_they_name_a_vote`,
`toreador_justicar_titles_a_ready_camarilla_toreador`.

### 19.4 Ventrue Headquarters' votes _(answered 2026-09-25)_

"Lock … to get +3 votes": the Methuselah's own votes, cast all on one side
as the card locks, like the Edge. **Answer** — yes. **Test** —
`ventrue_headquarters_locks_for_3_votes`.

### 19.5 Saulot's Guiding Wisdom ending a combat _(answered 2026-09-25)_

No strike resolves (advanced rules §4.7.2); the end-of-round window still
opens (§4.9). **Answer** — yes. **Test** —
`saulots_guiding_wisdom_ends_a_combat_of_another_minion`.

### 19.6 Valerius's steal _(answered 2026-09-25)_

Would a card representing the stolen title move to Valerius?

**Answer (2026-09-25)** — no: the card is burned and he just gets the
title. **Applied**: the prince loses the title (its card, if any, burned);
Valerius gains the title with no card, losing any title he held; a baron of
the same city would contest it. **Test** —
`valerius_steals_a_princes_title` (no pool title is a card for a prince:
the card branch is the shared `lose_title`).

## Batch 20 — Archetypes (slice 11g)

### 20.1 Directed actions without a printed Ⓓ _(answered 2026-09-25)_

Saulot's Healing Touch's "+1 stealth action" adds blood to another ready
minion, and prints no Ⓓ. Is it undirected even at another Methuselah's
minion?

**Answer (2026-09-25)** — no. Ⓓ, printed or not, is only indicative: an
action targeting your own minions is undirected; one targeting another
Methuselah's minion is directed at them. **Test** —
`saulots_healing_touch_adds_blood_to_another_ready_minion`.

## Batch 21 — Combat cards (slice 11i)

### 21.1 Strikes that put their card on the opposing minion _(answered 2026-09-26)_

Touch of Oblivion [obl] ("Strike: put this card on the opposing minion with
2 counters") and Sculpt the Flesh [PRO] ("… and put this card on the
opposing minion"): the card goes on as the strike resolves, only if it
lands; a dodge stops it and the card is discarded, as does a combat ended
before it resolves. Does Touch of Oblivion's strike, dealing no damage,
work at any range?

**Answer (2026-09-26)** — yes for the dodge; but a strike works at close
range only by default, never at long range unless it says so. **Applied**:
both Touch of Oblivion strikes are close-range.

**Follow-up (owner, 2026-09-26)** — only action cards wait in limbo, until
successful or blocked. Every other card goes to the ash heap as it resolves,
unless its effect puts it in play, even when part of its effect lingers or
is delayed. Players often leave such cards on the table as reminders; that
is display only. The delayed effect applies whether or not the card is still
there, and the card need not go anywhere (owner, 2026-09-26). **Applied**:
Touch of Oblivion, Sculpt the Flesh [PRO] and Pass Through Shadow [OBL] go
to the ash heap as they resolve and stay there. Their lingering effect is an
effect-only card in play (`CardInPlay.effect_only`). Burning it ends the
effect and moves no card; an oust removes it without touching the ash
heap. **Tests** —
`sculpt_the_flesh_superior_stays_on_the_opposing_minion`,
`touch_of_oblivion_keeps_the_minion_locked_two_unlock_phases`.

### 21.2 Touch of Oblivion: "instead of unlocking as normal" _(answered 2026-09-26)_

Per the advanced rules §5.2.2 substitution template, the attached minion
burns a counter even if it is already unlocked. One kept from unlocking by
another effect (Stolen Police Cruiser) burns none that phase. At 0 counters
the card is burned, and the minion unlocks as normal from the next unlock
phase. **Answer** — right. **Test** —
`touch_of_oblivion_keeps_the_minion_locked_two_unlock_phases`.

### 21.3 Rolling with the Punches [FOR] _(answered 2026-09-26)_

"Prevent all damage from the opposing minion's strikes this round" also
reaches that minion's additional strikes later in the round. It does not
reach environmental or retainer damage. It is playable only with strike
damage pending (LSJ 20001114). **Answer** — yes. **Test** —
`rolling_with_the_punches_prevents_the_strikes_for_1_blood`.

## Batch 22 — Combat cards with their own mechanisms (slice 11j)

### 22.1 A strike card played at the maneuver _(answered 2026-09-26)_

Slam [POT] and Aid from Bats, played during determine range (window review:
a maneuver rider commits the strike early), give their maneuver at once and
commit the round's initial strike. At strike choice, that strike is the only
one offered; the card itself goes to the ash heap as it resolves. Slam's
maneuver only closes a long range. Neither is offered at the maneuver while
the strike could not be used (LSJ 20021028). **Answer** — yes. **Tests** —
`slam_played_at_the_maneuver_closes_the_range_and_commits_the_strike`,
`aid_from_bats_maneuvers_either_way_and_commits_its_strike`,
`a_strike_with_a_maneuver_is_not_played_while_strikes_are_barred`.

### 22.2 Pack Alpha [ani]'s employ _(answered 2026-09-26)_

The retainer and its version are named as Pack Alpha is played, among those
the vampire could employ now (requirements met, cost affordable). At
resolution the retainer card, if still in hand, is paid for and put in play,
and the Methuselah replaces it like a card played from hand. Not being an
action, an employ action's riders do not apply (Feral Hound's unlocks).
**Answer** — yes. **Test** — `pack_alpha_employs_an_animal_retainer_from_hand`.

### 22.3 Voracious Vermin [ANI]'s additional strike _(answered 2026-09-26)_

"With 1 additional ranged strike: burn weapon": is the additional strike
optional?

**Answer (2026-09-26)** — it is not a matter of declining. Either there is a
weapon to burn, and the superior version can be chosen, or there is none,
and it cannot. It works at any range, and a dodge stops it. **Applied**: the
[ANI] mode requires a weapon on the opposing minion (`R::OpposingWeapon`).
Its additional strike burns one of them (the only choice is which), and
there is none if no weapon is left by then. **Test** —
`voracious_vermin_dodges_then_burns_a_weapon`.

### 22.4 Weighted Walking Stick as a weapon _(answered 2026-09-26)_

Once in play it counts as weapon equipment everywhere the engine asks:
Blood Rage and Soul Burn nullify it, Form of the Bat bars it, and Voracious
Vermin can burn it. **Answer** — yes, as long as it is in play; in hand or in
the ash heap it is just a combat card. **Test** —
`weighted_walking_stick_loses_counters_per_damage_inflicted`.

## Batch 23 — Action modifiers and reactions (slice 11k)

### 23.1 Change of Target and the blocker _(answered 2026-09-26)_

The card is played "before the blocker is locked" (LSJ 19980224, RTR
19991206), so a blocker that was unlocked when it blocked stays unlocked.
The action ends as blocked: no combat, no diablerie. **Answer** — yes. **Test** —
`change_of_target_ends_a_blocked_action_before_combat`.

### 23.2 "Cannot perform the same action again this turn" _(answered 2026-09-26)_

For a card action this is the ordinary NRA identity. For a rulebook action,
"the same action" is the same kind at the same target: another rescue of a
different vampire stays open (RTR 19950509). **Answer** — yes. **Test** —
`change_of_target_forbids_the_same_rulebook_action_only`.

### 23.3 Daring the Dawn's timing _(answered 2026-09-26)_

The damage applies after resolution. LSJ 20010715 (Force of Will) lets
after-resolution cards be played before or after such damage. **Answer** —
either: players order effects as they wish. **Applied**: in the
after-resolution window the acting Methuselah takes the damage
(`Choice::TakeDamageAfterResolution`) whenever they choose among their
after-resolution effects, and before passing. An action with no such
window … _(corrected by the owner, 2026-09-26: there is always one.
A blocked action has its after-resolution window after combat, and an
action ended by Change of Target or Mirror Walk has one too — no card is
played in it (RTR 20180719), but Daring the Dawn's damage is taken there.
A failed action has one as well (ANK 20221205).)_ **Tests** —
`daring_the_dawn_damage_is_ordered_among_after_resolution_effects`,
`daring_the_dawn_burns_after_combat_or_an_ended_action`.

### 23.4 Unleashing the Bestial Soul [DOM]'s surcharge _(answered 2026-09-26)_

"The next reaction card played this action costs +1 blood or life": the
first reaction card to be paid for after it takes the surcharge. **Answer** — yes. **Test** —
`unleashing_the_bestial_soul_bars_and_taxes_reactions`.

### 23.5 Visions of Zapathasura [OBF][PRE]: "reduce … to 0" _(answered 2026-09-26)_

The reduction is the bleed amount as the card resolves, a fixed amount, so
later increases still count (the card's own parenthesis). **Answer** — yes. **Test** —
`visions_of_zapathasura_superior_reduces_the_bleed_to_zero`.

### 23.6 Veil the Legions [OBF] _(answered 2026-09-26)_

X is chosen as the card is played, from 0 to the vampire's blood, and burned
as it resolves. The next X actions of the Methuselah's minions this turn get
+1 stealth, counted as each is announced. Is "only one at superior each
turn" per Methuselah?

**Answer (2026-09-26)** — no: one in all, and a turn is one player's turn.
**Applied**: a game-wide flag for the turn (`GameState.veil_superior`).
**Test** — `veil_the_legions_superior_gives_the_next_actions_stealth`.

## Batch 24 — Cards remembering a Methuselah (slice 11l)

### 24.1 A bleed whose target is ousted before it resolves _(answered 2026-09-26)_

LSJ 20050607: if the cost of canceling Spying Mission ousts a player, the
oust is handled before the bleed resolves. If the ousted player is the
bleed's target, the bleed has no one left to burn pool. **Applied**: the
bleed resolves without effect: fizzled (after-resolution cards apply), no
pool, no Edge. The oust already paid its predator. Right? **Test** —
`spying_mission_canceled_lets_the_bleed_resolve_after_the_oust`.

### 24.2 The "would be successful" window _(answered 2026-09-26)_

After the blocks are declined, a bleed of 1 or more opens one more window
over the action's order (acting Methuselah first). Only cards timed "if a
bleed would be successful" are played in it (Spying Mission here; Dis
Pater, Major Boon and Andre LeRoux are not in V5), and none once the
action has failed (after a Spying Mission). A canceled Spying Mission lets
only such cards follow, never a reaction (LSJ 20050607). Right? **Test** —
`spying_mission_superior_replaces_a_bleed_then_boosts_the_next_ones`.

### 24.3 Spying Mission [OBF]'s memory _(answered 2026-09-26)_

"The same Methuselah" is the bleed's target as the card resolves, after
any redirect. The copies already on the vampire burn only as a later bleed
of that Methuselah resolves for 1 or more, after the would-be-successful
window. So a second Spying Mission played in that window leaves the first
in place: the bleed is no longer successful (ANK 20200609's "multiple
Spying Missions against the same Methuselah"). Right? **Test** —
`spying_mission_superior_replaces_a_bleed_then_boosts_the_next_ones`.

### 24.4 A bleed of 0 _(answered 2026-09-26)_

The rulebook calls a bleed successful only for 1 or more: a bleed at 0
takes no Edge, and "after a successful bleed" effects (Ilonka, Richard de
Worde) do not apply. Before this, the engine gave the Edge at 0. The
action itself still succeeded (Freak Drive [for] can follow). Right?
**Test** — `spying_mission_needs_a_bleed_of_one_or_more`.

### 24.5 Shadow Cast [OBL]'s burn _(answered 2026-09-26)_

It is a stealth effect, so it is usable only when stealth is needed (not
Loup's exception), by the vampire it is on, during its own action. The
action must be directed at the remembered Methuselah (an action is
directed at one Methuselah at most here). Right? **Test** —
`shadow_cast_superior_gives_stealth_against_the_same_methuselah`.

**Answers (2026-09-26)** — 24.1 to 24.5: yes. On 24.5 the owner adds that
an action can be directed at several Methuselahs. Shadow Cast can then be
burned when the set it recorded as played (its vampire's controller not
counted, LSJ 20100527) is fully included in the set the current action is
directed at. **Applied**: `CardInPlay.against` is a `SeatSet`;
`R::DirectedAtRemembered` tests inclusion in `ActionCtx::directed_seats`
(one seat at most for now: no action here is directed at several yet).
**Test** — `shadow_cast_does_not_count_its_vampires_controller_in_the_set`.

## Batch 25 — Corruption counters and multi-Discipline cards (slice 11m)

### 25.1 Corruption counters _(answered 2026-09-26)_

Counters belong to the Methuselah who put them ("your corruption
counters"). They stay on a minion through torpor and control changes, and
go when it leaves play: burned, banished, or removed with its ousted
controller. Counters are not cards, so a banished vampire keeps only its
blood and the cards on it. An ousted Methuselah's counters stay on the
minions of others. No card reads them as "yours" any more, but Aniel can
still burn them as "a corruption counter". Right?

### 25.2 Multi-Discipline cards _(answered 2026-09-26)_

"More than one Discipline can be used to play this card": the card is
played with any non-empty set of its Disciplines the vampire has, and gets
each effect of the set (LSJ 20011204-3). So The Platinum Protocol from a
vampire with [pre] and [pro] offers [pre], [pro] or [pre][pro]. Right?
**Test** — `platinum_protocol_uses_any_set_of_its_disciplines_and_corrupts`.

### 25.3 Revelation of the Serpent's "or" _(answered 2026-09-26)_

"+1 bleed (limited), or +1 stealth and +1 bleed (limited)": two ways to
play it. The bleed alone is always available during the bleed; the
stealth option only when stealth is needed, as any stealth. The bleed
alone stays a choice even when stealth is needed, for a player who wants
the block. Right? **Test** —
`revelation_of_the_serpent_superior_unlocks_for_two_counters`.

### 25.4 Aniel's "burn 1 corruption counter" _(answered 2026-09-26)_

On a minion carrying several Methuselahs' counters, Aniel's controller
chooses whose counter burns, as the action resolves. The target must
still carry a counter then, or the action fizzles. The counter half and
the card half are one printed action for NRA. Right? **Test** —
`aniel_burns_a_corruption_counter_as_one_printed_action`.

### 25.5 Enchanting Gaze [PRE][PRO] and Revelation [PRE][PRO] _(answered 2026-09-26)_

"You" is the acting Methuselah (who plays the card). Enchanting Gaze
gives one use per card this action, usable while a minion carrying at
least 1 of their counters attempts to block, whether or not stealth is
needed. Revelation's unlock is offered after resolution while the acting
vampire is locked, not under The Sleeping Mind [DOM]'s "cannot unlock".
Right? **Tests** —
`enchanting_gaze_superior_burns_a_counter_to_fail_a_block`,
`revelation_of_the_serpent_superior_unlocks_for_two_counters`.

### 25.6 "If the bleed is successful" choices _(answered 2026-09-26)_

After the pool is burned: Break the Bonds [obf]'s lock is optional and may
pick any minion of the bled Methuselah, a locked one or one in torpor
included. The Platinum Protocol [pro]'s counter is mandatory, on any
minion that Methuselah controls, one in torpor included. When a mode has
both [obf] and [pre], the lock comes first, then the blood, in printed
order. Right? **Tests** —
`break_the_bonds_locks_and_feeds_the_uncontrolled_region`,
`platinum_protocol_uses_any_set_of_its_disciplines_and_corrupts`.

**Answers (2026-09-26)**:
- **25.1** — no: a banished vampire keeps everything, its counters of any
  kind included. **Applied**: banishment no longer clears them. A burned
  minion, or one removed with its ousted Methuselah, loses them.
- **25.2** — the printed layout decides. "[dom][pre] X" on one line is a
  combo: both are needed (Iron Glare; KRCG `combo`). "[pre] X / [tem] Y" on
  separate lines is a choice: one Discipline, at one level (Line Brawl).
  Some cards say that several can be combined: any set of them, each
  effect applying (Break the Bonds, The Platinum Protocol). Outside the
  Hourglass is a choice with an "outferior" option (see the glossary,
  DESIGN "Glossary"). **Applied**: already the model — one mode per line
  (a choice), a combo line needing every tag, and `Label::MultiDiscipline`
  subset modes only for "more than one Discipline can be used".
- **25.3** — yes, a real choice. The stealth option cannot be used when
  stealth is not needed (no block attempt), so the bleed option is what
  makes the card playable without one. **Applied**: as is.
- **25.4** — yes.
- **25.5** — Revelation's unlock does not need the acting vampire to be
  locked: an unlocked vampire can unlock, and a locked one lock, to no
  effect. Only a "lock to" or "unlock to" cost cannot be paid that way.
  **Applied**: the lock condition is dropped. Under The Sleeping Mind the
  unlock is still usable, with no effect, like Freak Drive.
- **25.6** — yes.

## Batch 26 — Actions, wave 1 (slice 11o)

Defaults chosen while implementing; each is applied and tested.

### 26.1 "Enter combat with a minion" from an action card _(answered 2026-09-26)_

The target is another Methuselah's ready minion, as for the printed
actions (rules-qa 17.3: no combat with one's own). It is re-checked at
resolution. Fleetness [CEL]'s "locked minion" unlocked by then fizzles the
action (ANK 20210124). The combat is the resolution, and the action
succeeds after it. Right? **Tests** —
`fleetness_superior_enters_combat_with_a_locked_minion_or_fizzles`,
`hunters_mark_gives_a_press_and_bars_combat_ends_in_the_first_round`.

### 26.2 Show of Force's "can gain +N strength" _(answered 2026-09-26)_

This is a choice offered before range in round 1 of the block's combat
(`Choice::GainStrength`), not an automatic gain. Right? **Test** —
`show_of_force_superior_gains_strength_when_blocked`.

### 26.3 Targets of Line Brawl, Rewilding, Touch of Valeren _(answered 2026-09-26)_

- Line Brawl [cel]'s "another Methuselah" is any other seat, not only
  prey or predator.
- Rewilding can target one's own location (undirected), burning one's own
  2 pool.
- Touch of Valeren [ACTION]'s "a minion" is any minion, directed when it
  is another Methuselah's.

Right? **Tests** — `line_brawl_steals_a_pool_from_any_other_methuselah`,
`rewilding_burns_a_location_and_two_pool_from_its_controller`,
`touch_of_valeren_heals_and_prevents_a_strike_even_from_outside`.

### 26.4 Shroud of Decay [OBL] needs 7 cards _(answered 2026-09-26)_

"Remove 7 cards in your prey's ash heap from the game to burn 3 of their
pool": playable only with 7 or more cards there, re-checked at resolution
(fewer by then fizzles). Right? **Test** —
`shroud_of_decay_superior_removes_seven_ash_cards_for_three_pool`.

### 26.5 Rutor's Hand's unlock _(answered 2026-09-26)_

"During your minion phase, this vampire can lock this card to unlock":
offered in one's own minion phase between actions, as Slaughtering the
Herd [DOM]'s burn is. The card enters play locked and unlocks in its
controller's unlock phase. Right? **Test** —
`rutors_hand_wounds_or_costs_blood_then_unlocks_its_vampire`.

### 26.6 Heroic Might's "strike: burn equipment" _(answered 2026-09-26)_

A strike at close range only (rules-qa 21.1), naming the opposing
minion's equipment as it is chosen; a dodge stops it. Right? **Test** —
`heroic_might_strikes_to_burn_equipment_and_burns_in_torpor`.

### 26.7 Monstrous Form's locked-card strength _(answered 2026-09-26)_

"+1 strength this round" is usable at any combat impulse before strike
resolution (as Claws of the Dead [pro]); the maneuver at the maneuver
step, the press at the press step. Right? **Test** —
`monstrous_form_adds_strength_and_its_card_locks_for_a_bonus`.

### 26.8 Feast of the Soul's Secrets [DOM]'s "younger vampire" _(answered 2026-09-26)_

"Younger" is relative to the acting vampire (lower capacity). A younger
vampire one controls may be in torpor. Right? **Test** —
`feast_of_the_souls_secrets_superior_feeds_a_younger_vampire`.

**Answers (2026-09-26)** — yes to all, except:
- **26.5** — "during your minion phase" is any time of it, during another
  action too: e.g. another vampire is acting, and one unlocks with Rutor's
  Hand to then play Veil the Legions. **Applied**:
  `Engine::in_own_minion_phase` includes one's own actions and their
  combats; Slaughtering the Herd [DOM]'s burn follows. **Test** —
  `rutors_hand_unlocks_during_another_vampires_action`.
- **26.8** — yes, and that younger target is chosen when the card is
  played (as applied).

## Batch 27 — Actions, wave 2 (slice 11p)

Defaults chosen while implementing; each is applied and tested. Owner
(2026-09-26): yes to all.

### 27.1 Puppet Master [DOM]'s take is mandatory _(answered 2026-09-26)_

"During your next minion phase, burn this card to unlock … and take
control": it can be done at any point of that phase (LSJ 20001201), but
the phase cannot end before it is done. Right? **Test** —
`puppet_master_superior_takes_a_vampire_for_the_next_turn`.

### 27.2 Targets _(answered 2026-09-26)_

- Puppet Master [DOM]'s "a younger vampire" may be one of one's own
  (undirected, a no-op in effect).
- Deep Song [ANI]'s "a vampire" is another Methuselah's ready vampire
  (no combat with one's own, rules-qa 17.3).

Right?

### 27.3 Stealing mid-action _(answered 2026-09-26)_

A stolen blocker's attempt fails without barring it: its new controller
could attempt again if eligible. A stolen combatant's combat ends after
the damage still pending resolves (LSJ 20020618). Right? **Test** —
`puppet_master_steals_a_blocker_or_a_combatant`.

### 27.4 A minion removed from the game _(answered 2026-09-26)_

When a temporarily controlled minion cannot return (its Methuselah is
ousted), it is removed from the game. Its owner's cards on it go with it;
other Methuselahs' cards on it are burned to their ash heaps (as for an
ousted Methuselah's minions). Right? **Test** —
`puppet_master_temporary_control_and_ousts`.

### 27.5 Deep Song [ANI]'s "considered the acting minion" _(answered 2026-09-26)_

In that combat, the target takes the acting minion's side, so its
Methuselah acts first in each combat step. Effects of the action on "this
acting vampire" (Invigorate [pro]'s aggravated hands, Form of Mist's
continuation) stay with the vampire that took the action. Right? **Test** —
`deep_song_superior_locks_and_fights_a_vampire_as_the_acting_minion`.

### 27.6 The Gate of Acheron's payment _(answered 2026-09-26)_

"For each counter … they burn 1 pool or remove a library card at random in
their ash heap from the game": announced as one count of ash heap cards,
the rest pool. The cards are random, so this is the same as unit by unit
(as Constant Revolution, LSJ 20090618). Right? **Test** —
`gate_of_acheron_grows_and_drains_the_prey`.

### 27.7 Pit of Contemplation's "owner's uncontrolled region" _(answered 2026-09-26)_

A vampire held on temporary control goes to the region of its controller
(the Methuselah it would return to). The move is Banishment's: blood kept
as counters, cards with it. Right? **Test** —
`pit_of_contemplation_moves_a_young_vampire_to_the_uncontrolled_region`.

## Batch 28 — Masters and Childe of the Revolution (slice 11q)

Defaults chosen while implementing; each is applied and tested. Owner
(2026-09-26): yes to the ones without an answer below.

### 28.1 Childe of the Revolution's new vampire _(answered 2026-09-26)_

It enters the ready region unlocked with no blood (the text gives none),
capacity 1 plus any Discipline card put on it, and may act that turn: it
must hunt, even with blood, and does so at once when that is its only
option. The library is shuffled after the search whatever was taken
(or nothing). Right? **Test** —
`childe_of_the_revolution_becomes_a_vampire_that_must_hunt`.

### 28.2 "An Anarch", "a Ravnos", "a Nosferatu" on a location _(answered 2026-09-26)_

Where the text does not say "you control", any Methuselah's minion:
Club Illusion's Anarch burns its own blood; Anarch Railroad, Fortune
Teller Shop and Warsaw Station are locked by their controller for another
Methuselah's acting minion. Right? **Tests** —
`club_illusion_lets_any_anarch_burn_blood_for_one_bleed_once`,
`anarch_railroad_gives_any_acting_anarch_stealth`.

### 28.3 Capacity in the uncontrolled region _(answered 2026-09-26)_

Counters added by an effect (Wasserschloss Anif's, Powerbase: Montreal's,
Family Gathering's) stop at the vampire's capacity there, the rest to the
blood bank — as transfers do. A banished vampire keeps counters a
Discipline card let it hold. Right? **Test** —
`a_banished_vampire_keeps_the_blood_its_discipline_card_held`.

**Answer (owner):** "While a minion is uncontrolled you can put any amount
on them and go over their capacity. It just drains off to bank if
influenced to controlled, which is a choice." Applied: transfers and
effects (Wasserschloss Anif, Powerbase: Montreal, Family Gathering) put any
amount on; the excess drains as the vampire is influenced out.

### 28.4 Capacity lost _(answered 2026-09-26)_

When a Discipline card leaves a vampire in play (burned, or its player
ousted), blood above the new capacity drains off at once. Right?

### 28.5 Choices made as the card resolves _(answered 2026-09-26)_

The Rack's vampire is the play's target (none ready: not playable);
Toreador Grand Ball's locked Toreador is the target and the other chosen
as it resolves; Priority Contract's Assamite is chosen as it resolves; on a
change of control with no ready vampire, The Rack has none chosen.
Chantry's blood payer, Anarch Troublemaker's two vampires (a locked one
allowed) and Garibaldi-Meucci Museum's two cards (no window between use
and effect) are chosen as they resolve. Right?

**Answer (owner):** "Rack, Grand Ball, Contract: chosen when played. The
others have an effect when in play, target chosen when used." Applied:
Toreador Grand Ball's second Toreador and Priority Contract's Assamite are
now the play's terms (`Terms::OwnOfClan`), chosen as the card is announced.
Chantry, Anarch Troublemaker and Garibaldi-Meucci Museum choose as the
ability is used, which the engine does with nothing in between.

### 28.6 Wasserschloss Anif's Tremere _(answered 2026-09-26)_

"A Tremere you control can move 1 blood from themselves": a ready one.
Right? **Test** —
`wasserschloss_anif_banks_one_tremeres_blood_for_the_uncontrolled_region`.

**Answer (owner):** "Does not say ready, so it does not limit to ready. You
can have blood in torpor (eg. if you take aggravated damage)." Applied: a
Tremere in torpor can move its blood too.

### 28.7 Out of torpor before resolution _(answered 2026-09-26)_

A rescue whose target already left torpor fizzles unpaid; a leave-torpor
action fizzles paid (ANK 20220218); a diablerie fizzles with no blood hunt.
Right? **Test** —
`warsaw_station_saves_a_nosferatu_from_a_diablerie`.

**Answer (owner):** "Unsure about leave fizzling unpaid — is it the ruling?
Why the difference with rescue?" The leave-torpor half is the ruling (ANK
20220218, filed under Warsaw Station: "the action immediately fizzles
(cost, if any, is paid, and it is considered successful, but has no
effect)"). No ruling covered the rescue, and nothing justified the
difference: a rescue whose vampire left torpor now fizzles paid too (the
split between rescuer and rescued as normal; a vampire gone from play
pays nothing, the rescuer paying what it can). **Test** —
`a_rescue_fizzles_paid_once_warsaw_station_raised_its_vampire`.

### 28.8 Week of Nightmares _(answered 2026-09-26)_

A Ravnos with no blood (or that must hunt) must take the special hunt if it
has a target, else nothing holds it. At the end, a Ravnos pays its
nightmare blood if it can, and is burned only if it cannot. "Only one …
can be played in a game" counts a play canceled as played. Right? **Tests**
— `a_starving_ravnos_must_steal_once`,
`week_of_nightmares_ends_in_nightmares_and_never_comes_back`.

**Answer (owner):** "Must hunt, frozen if no target." Applied: a Ravnos
that must hunt and has no target for the special hunt takes no action
at all; other minions act. **Test** —
`a_starving_ravnos_without_a_target_is_frozen`.

### 28.9 Frontal Assault's influence-phase burn _(answered 2026-09-26)_

Applied as the influence phase begins, each copy (before any transfer).
Right? **Test** —
`frontal_assault_rushes_the_prey_pays_for_losses_and_costs_at_influence`.

**Answer (owner):** "Mandatory effect must be resolved before passing the
impulse, but can be ordered (so you can influence first)." Applied: each
copy's burn is a choice of the influence phase, Pass withheld until all
are resolved; the prey's ready minions are counted then.

### 28.10 Piper's Anarch _(answered 2026-09-26)_

The locked Anarch is the card's actor: it meets the ally's or retainer's
requirements and pays its blood cost; its Disciplines pick the version.
Piper itself stays a master card Sudden Reversal can cancel. Right?
**Tests** — `piper_has_an_anarch_recruit_an_ally_outside_an_action`,
`piper_employs_by_the_anarchs_disciplines_and_is_no_action`.

### 28.11 Meditative Grove's "played on a Salubri" _(answered 2026-09-26)_

The frenzy card's target (Deep Song [ANI]), or its player for a card
played on itself (Terror Frenzy, once defined). Right? **Test** —
`meditative_grove_cancels_a_frenzy_card_played_on_its_salubri`.

**Answer (owner):** "Check rulings on frenzy and frenzy-cancel cards.
Really depends but basically if the minion is affected it's the target."
The rulings agree: "any frenzy card that targets/selects/chooses/affects
the minion when played is considered being played on the minion" (LSJ
20051113, on the frenzy-cancel cards), and for a card its player plays on
their own side of a combat, "the opponent is the one affected by frenzy"
(ANK 20220409, Dam the Heart's River). Applied: Terror Frenzy is played on
the opposing minion.

### 28.12 Powerbase: Los Angeles's discard phase action _(answered 2026-09-26)_

The player spends whichever discard phase action serves best: Los
Angeles's on a card requiring an Anarch (its requirements name Anarch or
baron) or making a vampire Anarch, another otherwise. Right? **Test** —
`powerbase_los_angeles_unlocks_an_anarch_for_an_anarch_discard`.

### 28.13 Note on advanced rules 6.4.1

It says a temporarily controlled minion stays with its current controller
if its permanent controller is ousted; Puppet Master's rulings (ANK
20181110, a reversal) remove it from the game, which the engine follows
(27.4). The advanced rules may want the same.

**Answer (owner):** fix the advanced rules. Done (uncommitted there): 6.4.1
in `../vtes-advanced-rules` now agrees with its own 6.2.4 — the minion is
removed from the game when it should return to the ousted Methuselah (ANK
20181110); only a return that can no longer occur (Parmenides) leaves it
with the current controller.

## Batch 29 — Combat cards, wave 2 (slice 11s)

Defaults chosen while implementing; each is applied and tested. Owner
(2026-09-26): yes to the ones without an answer below.

### 29.1 Prevention from outside the combat _(answered 2026-09-26)_

Martyr's Resilience ("an unlocked vampire not involved in the combat", any
Methuselah's, ANK 20181101): after both combatants' prevention, every
Methuselah in turn, clockwise from the acting one, may prevent with such a
vampire — on either combatant or a retainer on one. A combatant's
controller may also play it during their own prevention. A partial cover
of mixed damage is the Martyr's player's allocation. Right? **Tests** —
`martyrs_resilience_prevents_from_outside_the_combat`,
`martyrs_resilience_superior_saves_a_retainer`.

### 29.2 Damage to retainers waits for prevention _(answered 2026-09-26)_

Damage a strike aims at a retainer now waits with the combatants' damage
and is applied with it (it used to apply as the strike resolved: no pool
card could prevent it before Martyr's Resilience). Life stolen from a
retainer still moves at once. Right?

### 29.3 Disarm's "successfully inflicted more damage" _(answered 2026-09-26)_

The damage from strikes that lands after prevention, all of the round's
strike pairs counted. With strike and environmental damage pending
together and part prevented, the prevention is taken from the
environmental damage first. Right? **Test** —
`disarm_sends_the_weaker_striker_to_torpor_with_less_strength`.

**Answer (owner):** "No, the player preventing chooses what they prevent."
Applied: a prevention that could cover strike or other damage asks its
player which (`Step::AllocateSource`), and Disarm counts the strike damage
left. **Test** — `disarm_counts_the_strike_damage_the_preventer_let_through`.

### 29.4 Flesh of Marble _(answered 2026-09-26)_

Applied as the damage lands, after every other prevention. Weather
Control's before-range damage is "successfully inflicted" and so the
round's first: strike damage that round is then prevented. Right?
**Tests** — `flesh_of_marble_lets_one_damage_through_each_round`,
`flesh_of_marble_leaves_aggravated_damage_unless_superior`.

### 29.5 Tranquility Shield against Terror Frenzy _(answered 2026-09-26)_

Terror Frenzy is played on the opposing minion (28.11), so a shielded
vampire's opponent cannot play either version of it, and a shield played
after cancels both versions' effects on the shielded vampire: it may
maneuver to close range, press to continue and use equipment again, and
its combat cards cost no more. Right? **Test** —
`tranquility_shield_cancels_and_bars_frenzy_cards`.

### 29.6 Contraband [OBF]'s blood _(answered 2026-09-26)_

The blood share is chosen as the card is played, with the equipment's
name: from what the pool cannot pay up to half the pool cost, rounded
down. Right? **Test** — `contraband_superior_equips_paying_half_in_blood`.

**Answer (owner):** "You choose what amount the vampire pays in blood, up
to half; the rest is paid with pool (can be more than half)." As applied:
the blood part, chosen as the card is played, at most half; the pool pays
the rest (at least what the blood part leaves).

### 29.7 Anticipation [aus] with a weapon _(answered 2026-09-26)_

"Use a melee weapon strike" is the weapon's own strike, riders included
(Righteous Blade's press), at +2 damage, from that weapon (gone by
resolution, no effect). [AUS] also cancels a strike card played at the
maneuver (Slam). Right? **Test** —
`anticipation_strikes_with_the_hand_or_a_melee_weapon_at_plus_two`.

### 29.8 Lightning Reflexes [CEL] with X = 0 _(answered 2026-09-26)_

Playable, getting no strike but using the round's limited slot (a legal
no-op, kept like the others). Right?

**Answer (owner):** "If you get no additional strike you don't consume the
limit." Applied. **Test** — `lightning_reflexes_for_no_blood_leaves_the_limit`.

### 29.9 A new round with a combatant gone _(answered 2026-09-26)_

Telepathic Tracking starts a new round, then Disarm sends the opponent to
torpor in the same window: the combat ends instead. Right? **Test** —
`disarm_and_telepathic_tracking_share_the_end_of_round`.

### 29.10 Disarm's "they can burn 3 blood to burn this card" _(answered 2026-09-26)_

No timing printed: any time the vampire's controller has the impulse, in
torpor too. Right?

### 29.11 Hide the Mind [obf] _(answered 2026-09-26)_

"A combat card requiring Auspex": any, the player's own included (the text
does not say opposing). Right?

## Batch 30 — Reactions and action modifiers, wave 2 (slice 11t)

Defaults chosen while implementing; each is applied and tested. Owner
(2026-09-26): yes to the ones without an answer below.

### 30.1 Dawn Operation [for]'s cancel _(answered 2026-09-26)_

"If a vampire is currently attempting to block, they can cancel their
block attempt": asked of that vampire's controller once, as the card
resolves. A canceled attempt is withdrawn, not failed: the vampire may
attempt again. Right? **Test** —
`dawn_operation_lets_the_attempting_vampire_cancel`.

### 30.2 Forced Awakening's blood _(answered 2026-09-26)_

Burned as the action begins to resolve (LSJ 19990421) — successful,
fizzled or failed. An action blocked by another minion does not resolve,
so no blood is burned. Right? **Tests** —
`forced_awakening_costs_a_blood_unless_it_blocks`,
`forced_awakening_counts_only_blocks_after_it`.

**Answer (owner):** "No, it resolves. The block and ensuing combat are the
resolution (blood is burned)." Applied: another minion's block burns the
blood as it happens, once.

### 30.3 Go-getter [OBF] _(answered 2026-09-26)_

Played in the after-resolution window of the blocked action (after its
combat), by the acting vampire if still ready: the action goes back to the
block step, modifiers and declines standing, as Form of Mist's
continuation. Right? **Test** —
`go_getter_superior_continues_a_blocked_action`.

### 30.4 Dogged Pursuit [ANI]'s card _(answered 2026-09-26)_

Put on the vampire after resolution when it did not block, unless it
already has one. Its +1 intercept is for any later block attempt of that
vampire, the card burned then. Right? **Test** —
`dogged_pursuit_follows_a_missed_block_with_intercept_for_later`.

### 30.5 Melange [AUS] _(answered 2026-09-26)_

Put on the acting minion as the block succeeds, even if the action is
then continued as if unblocked. Its "+1 bleed" is not limited (the text
does not say so). Right? **Test** —
`melange_superior_marks_the_acting_minion_for_a_later_bleed`.

### 30.6 Beast Meld [ani][pro]'s prevention _(answered 2026-09-26)_

One damage, usable at any point of the block's combat — any round, once.
Right? **Test** — `beast_meld_prevents_a_damage_once_blocked`.

## Batch 31 — The pool's actions, wave 1 (slice 11u)

Defaults chosen while implementing; each is applied and tested. Owner
(2026-09-26): yes to the ones without an answer below.

### 31.1 Stun (Mind Numb) _(answered 2026-09-26)_

No stun definition is in the card data. Implemented as the old Mind Numb
wording: the vampire locks and does not unlock during its controller's
next unlock phase (LSJ 20050114 reads that). Right? **Test** —
`mind_numb_stuns_through_the_next_unlock_phase`.

**Answer (owner):** "Check the rulebook." The V5 Sabbat terms (2025-06-12)
define it: "lock a minion and put a stun counter on them. A minion with one
or more stun counters does not unlock as normal at the beginning of their
controller's unlock phase; during that unlock phase, burn all stun
counters they had at the beginning of the turn." Same behaviour as
implemented (`Minion::skip_unlock`: several stuns before one unlock phase
are one skipped unlock).

### 31.2 Propaganda [PRE]'s lock _(answered 2026-09-26)_

"The target Methuselah locks a ready unlocked minion they control": their
choice, mandatory, as the unblocked bleed resolves, whatever its amount
(0 included). None if the bleed ousted them. Right? **Test** —
`propaganda_bars_titled_blockers_and_locks_a_prey_minion`.

**Answer (owner):** "No, choice of the player playing the card when
played." Applied: [PRE] names a ready unlocked minion of the prey as the
card is played; it locks as the bleed resolves.

### 31.3 Hunting the Beast [AUS]'s Salubri _(answered 2026-09-26)_

Chosen at the end of the combat, not upon declaration (ANK 20240706 is
Public Trust's, a different timing: "if the bleed is successful"). "The
opposing vampire is not ready": in torpor or burned. Right? **Test** —
`hunting_the_beast_superior_feeds_an_uncontrolled_salubri_after_winning`.

### 31.4 Dominate Kine [DOM] on Living Manse _(answered 2026-09-26)_

The new host is chosen by the new controller as the action resolves,
among their minions able to hold it (its Tzimisce requirement is not
checked again, as for any card changing hands). If none can hold it, it
stays where it is. Right? **Test** —
`dominate_kine_superior_steals_a_location`.

### 31.5 Graverobbing _(answered 2026-09-26)_

The stolen vampire keeps its cards and blood; burned later, it goes to
its owner's ash heap. Master cards on it (a Discipline master, Villein)
stay their player's (a master card on a minion is controlled by its
player, 11q); minion cards on it (equipment, retainers) move with it.
[DOM]'s move is offered as the action resolves, only if the acting
vampire has the 2 blood. Right? **Test** —
`graverobbing_superior_steals_a_torpid_vampire_and_may_rescue_it`.

### 31.6 A vampire burned under someone else's control _(answered 2026-09-26)_

Goes to its owner's ash heap — also one under temporary control (Puppet
Master [DOM]); before this slice it went to its controller's. Right?

### 31.7 Make the Misere's Disciplines _(answered 2026-09-26)_

[pot]'s +1 strength lasts the whole combat; [cel]'s maneuver is one for
the combat. Right? **Test** —
`make_the_misere_gives_what_each_discipline_used_adds`.

## Batch 32 — The pool's actions, wave 2 (slice 11v)

Defaults chosen while implementing; each is applied and tested. Owner
(2026-09-26): yes to the ones without an answer below.

### 32.1 Two actions from one card _(answered 2026-09-26)_

Generalized from Open War (LSJ 20100909): each action a card in play
grants is its own action for the "same action" rule — once each per
minion per turn (Haven Uncovered's two actions likewise). Right?

### 32.2 Army of Rats's once a turn _(answered 2026-09-26)_

"You can burn only 1 pool each turn with Army of Rats cards": its
controller's copies; the first one resolved in the unlock phase burns,
the others' burns lapse. Right? **Test** —
`army_of_rats_burns_one_prey_pool_a_turn_whatever_the_copies`.

### 32.3 Judgment: Camarilla Segregation's burn _(answered 2026-09-26)_

"Methuselahs can burn a non-Camarilla vampire they control during their
master phase": any Methuselah, in their own master phase, no master phase
action spent; a vampire in torpor qualifies. Right? **Test** —
`judgment_camarilla_segregation_taxes_non_camarilla_methuselahs`.

### 32.4 Tier of Souls [ANI]'s prey _(answered 2026-09-26)_

"+1 bleed against your prey": the card's controller's prey, read live (a
new prey after an oust counts). Stealing an ally's last life burns it.
Right? **Test** —
`tier_of_souls_superior_steals_blood_and_bleeds_the_prey_harder`.

### 32.5 Phantasmagoria's target _(answered 2026-09-26)_

"Put this card on a minion": any minion, one's own included (undirected
then, rules-qa 20.1). Right? **Test** —
`phantasmagoria_superior_costs_the_blocked_minion_a_blood`.

### 32.6 Waters of Duat's sect _(answered 2026-09-26)_

"It becomes a 1-capacity (non-unique) Follower of Set": no sect printed.
Implemented as Independent, the Followers of Set's sect when the card was
printed (V5's Ministry are Anarch). Right? **Test** —
`waters_of_duat_creates_a_follower_of_set_that_must_hunt`.

**Answer (owner):** "The acting vampire's sect actually (new default for
Embrace-like cards, newish rule)" — rulebook "Sect": new vampires take
their sire's sect. Applied (`BecomesVampire(None)`).

### 32.7 Open War's payout _(answered 2026-09-26)_

"Methuselahs can use a master phase action to move 1 counter from their
pool to this card. If this card has 4 counters, burn it and gain 4 pool":
the Methuselah who moves the fourth counter gains the 4 pool; paying the
counter with their last pool ousts them. Right? **Test** —
`open_war_grants_two_anarch_actions_and_pays_out_at_four_counters`.

**Answer (owner):** "No, the player controlling it gets the pool — card
text is addressed at the controller by default." Applied.

### 32.8 Aranthebes, The Immortal _(answered 2026-09-26)_

Shuffled into its owner's library. The lock gives -1 stealth to the
predator's acting minion (stealth only matters there). The -1 bleed is for
vampires only, not allies, while the card is unlocked. Right? **Tests** —
`aranthebes_shields_against_young_bleeds_and_shuffles_away`,
`aranthebes_locks_for_minus_one_stealth_on_the_predators_action`.

### 32.9 Fee Stake's non-Anarch titles _(answered 2026-09-26)_

"Non-Anarch titles are worth -1 vote": every title but baron (primogen,
prince, justicar, an independent title card), to a minimum of 0. The +1
vote goes to the Fee Stake's Anarch only when it calls the referendum.
Right? **Test** —
`fee_stake_represents_a_baron_and_its_burn_weakens_other_titles`.

### 32.10 Khabar: Glory's condition _(answered 2026-09-26)_

"Not usable if any non-mandatory actions have been performed this turn":
any action of the player's minions already announced this turn (blocked or
fizzled too), except a must-hunt vampire's hunt or a printed mandatory
action. Right? **Test** —
`khabar_glory_is_a_first_action_that_pays_for_an_oust`.

### 32.11 Split the Veil's ally _(answered 2026-09-26)_

Chosen when the last counter burns, among the wraith and zombie allies in
the player's ash heap; with none there, the card is burned all the same.
Right? **Test** —
`split_the_veil_raises_a_wraith_or_zombie_from_the_ash_heap`.

### 32.12 Psychophagia's ally _(answered 2026-09-26)_

Chosen as the action resolves: whose ash heap, then which ally; an ousted
Methuselah's ash heap does not count. Right? **Test** —
`psychophagia_eats_an_ash_heap_ally`.

**Answer (owner):** "No, chosen at declaration." Applied
(`Terms::AnyAshAlly`: the ash heap, then the ally, as the card is played;
gone by resolution, no effect). **Test** —
`psychophagia_names_its_ally_at_play_and_serves_a_forced_hunt`.

### 32.13 Psychophagia and a vampire that must hunt _(answered 2026-09-26)_

Psychophagia is a hunt action, but a vampire that must hunt (Childe of
the Revolution, Waters of Duat) is only offered the rulebook hunt as its
mandatory action, and Psychophagia does not end its "must hunt". Should a
must-hunt vampire be able to hunt with Psychophagia instead?

**Answer (owner):** "Yes it should." Applied: offered beside the rulebook
hunt to any vampire that must hunt, and it ends the "must hunt".
