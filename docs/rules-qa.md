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

### 1.5 Master phase structure _(open; out-of-turn accounting answered in 3.3)_

- During a Methuselah's master phase, what may *other* Methuselahs legally do,
  exhaustively? (Cancel-as-played responses, out-of-turn masters — anything
  else? No wakes without an action, presumably.)
- Confirm the master phase is "spend MPA one effect at a time, each play opening
  its cancel window, until the player moves on" — i.e. an offer window whose only
  actor is the phase owner, punctuated by reaction sub-windows.

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

### 3.1 A contested vampire card in torpor _(open)_

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

### 4.5 Combat cards by a combatant no longer ready _(open)_

Taste of Vitae and Disarm print "not usable by a vampire being burned or going
to torpor", which suggests the end-of-round window is otherwise open to such a
vampire. The engine lets a combatant that is no longer ready play end-of-round
combat cards unless the card excludes it, and no other combat window. Right?

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

Batch 8 — cards & effects (replacement, delayed replacement, events).
