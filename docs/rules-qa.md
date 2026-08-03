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

**Test** — _pending_: `referendum_source_atomicity`,
`referendum_cross_rotation_sources`, `referendum_methuselah_ballots_independent`.

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

**Test** — _pending_: `combat_step_impulse_reset`,
`combat_rush_acting_minion`, `combat_no_acting_minion_ordering`.

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

**Test** — _pending_: `block_attempt_persists_until_all_pass`,
`block_decline_is_final`, `block_third_party_intercept_after_attempt`,
`block_reopen_only_on_bounce`, `block_no_bait_retraction`.

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

**Test** — _pending_: `nra_resolution_not_success`, `nra_cancel_allows_retry`,
`nra_one_bleed_one_political_per_turn`, `nra_hunt_repeatable`,
`nra_in_play_copy_identity`, `nra_same_action_card_text_covers_hunt`.

### 1.5 Master phase structure _(open)_

- During a Methuselah's master phase, what may *other* Methuselahs legally do,
  exhaustively? (Cancel-as-played responses, out-of-turn masters — anything
  else? No wakes without an action, presumably.)
- Confirm the master phase is "spend MPA one effect at a time, each play opening
  its cancel window, until the player moves on" — i.e. an offer window whose only
  actor is the phase owner, punctuated by reaction sub-windows.

### 1.6 "Only when needed" stealth/intercept as a legality rule _(open)_

Stealth may be added only when currently blocked with sufficient intercept;
intercept only when stealth exceeds it.

- Confirm the engine should treat surplus stealth/intercept plays as *illegal*
  (masked out), not merely pointless.
- Corner cases: effects that add stealth/intercept as a side effect of a card
  played for another purpose; stealth added when no block attempt has been
  declared at all (e.g. pre-emptively before asking for blocks) — legal or not?

---

## Batch 2 — planned: actions & resolution (costs, targets, "as announced",
redirects, action continuation). Batch 3 — combat (strike resolution order,
prevention timing, presses/additional strikes). Batch 4 — politics (terms,
Scalpel-class cards, blood hunts). Batch 5 — cards & effects (replacement,
delayed replacement, contests, events).
