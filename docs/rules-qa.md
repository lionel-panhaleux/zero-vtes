# Rules Q&A — ruling reference

Owner rulings (VEKN judge) on VTES rules questions raised while building the
engine, Batches 1–54. Each item is the settled rule, stated as fact; the
question framing, engine mechanics, test names and confirmations are dropped
deliberately: the code and its tests are the reference for how a rule is
implemented, and they point back here through `rules-qa N.M` comments.
Settled readings by mechanism: `docs/rules-digest.md`.

Conventions:
- **N.M** numbers are stable and cited by code as `rules-qa N.M` — never
  renumber; add new batches at the end.
- One entry per item; a later item that overrides an earlier one says so
  ("supersedes", "revised by"), and the earlier item points to it.
- Ruling refs as in KRCG: [LSJ 19980302], ANK/RTR/TOM/PIB yyyymmdd; "advanced
  rules §x" = `../vtes-advanced-rules`; "spec §x" = its numbered sections.
- _open_ marks an item with no recorded owner answer (engine reading stated).

## Default readings — apply before asking

Patterns the engine's first readings got wrong repeatedly; each was corrected
by the owner. Apply them to every new card; only a ruling overrides them.

- **Choices are made at declaration.** Whatever a card or ability names or
  chooses (a target, an ally to fetch, a retainer and where it goes, an amount)
  is chosen as it is played / used / the action is announced — never at
  resolution. Only a random or searched result is known at resolution. At
  resolution the choices are re-checked: gone or illegal → fizzle. (31.2,
  32.12, 38.1, 41.6, 41.7)
- **A played card is played, however it came.** A card played from a search,
  from the ash heap, by Contraband, Piper, Vast Wealth… is played, not "the
  normal way" but still played: cost reductions apply, "when played" effects
  and "cannot play" bans apply. Only an effect that says *put in play* does not
  play the card. (36.5, 38.5, 40.10, 41.13)
- **A cost may be paid even if paying burns or ousts the payer**, as long as
  the payer has it: an ally pays its last life and burns (its attempt/effect
  then fails), a Methuselah pays their last pool and is ousted. (38.5, 41.4)
- **Limits and caps read on the words they qualify**, no further: "not to exceed
  their starting life" caps allies only; "once each round" is one use for one
  strike or damaging effect. (38.8, 41.8)
- **Properties are the card's, never assumed:** not every vampire is unique
  (created vampires: Embrace, Childe of the Revolution); a card on a minion is
  controlled by the minion's controller (38.4, 41.5).
- **Choosing in a referendum's terms is never targeting.** (38.6)
- **"During X, … can …" is once per X** (per copy, per vampire): "during
  your turn" once a turn, "during your minion phase" once in it, "during an
  action / a bleed / a referendum" once in it — never repeatable while
  payable. Read the word "during" in every ability's wording before
  choosing its limit. (6.6, 15.2, 42.3, 58.7)
- **"After X, … can …" is once after X** (per copy, per vampire): "after a
  successful bleed / an action / a referendum passes" once after it
  (`Limit::PerAction`), unless a cost or condition the use spends limits it
  already (lock this card, must be locked). (16.1; Gostoso, Forward
  Momentum, owner 2026-10-09)
- **A mandatory phase effect applies in the phase its card is played in.**
  "During your/each Methuselah's X phase, …" on a card that enters play
  during an X phase is due in that very phase, ordered among the phase's
  other mandatory effects by the player — not first in the next X phase.
  Phase dues are read live, never snapshotted as the phase begins. (33.1,
  40.5, 57.5)
- **Card text addresses its controller** by default (32.7); **Ⓓ is only a
  reminder** (20.1); **conditions are read live**; **legal no-op plays stay
  offered** (25.5).

## Index

- **Untimed abilities** — 7.7, 29.10, 15.2 (Kalinda)
- **Legacy additions for TWDA decks** — 44.1–44.10, 57.1–57.10, 58.1–58.20, 59.1–59.19, 60.1–60.22, 62.2–62.31, 62.33, 63.1–63.17, 64.1–64.12, 65.1–65.3, 67.1–67.7, 68.1–68.17, 69.1–69.15, 70.1–70.22, 71.1–71.27
- **Ousts, Life Boon's decision point** — 45.1–45.8
- **Impulse, sequencing, windows** — 1.2, 1.3, 1.5, 2.1, 2.2, 2.3, 2.4, 3.7, 4.1, 5.3, 7.7, 7.8, 7.9, 23.3, 24.2, 26.5, 66.1
- **Mandatory effects and their ordering** — 6.1, 6.4, 7.8, 7.9, 17.2, 27.1, 28.9, 32.10, 40.5
- **Periodicity ("during X" = once per X, once each …)** — 6.6, 7.4, 9.6, 10.4, 11.2, 15.1, 15.2, 16.1, 36.3, 36.4, 37.1, 38.8, 42.3, 42.9
- **No-repeat-action, "same action"** — 1.4, 6.2, 16.6, 23.2, 25.4, 32.1
- **Action outcomes: fizzle, fail, success, after resolution** — 3.6, 5.8, 5.9, 17.1, 17.3, 17.4, 23.3, 24.1, 24.4, 26.1, 26.4, 28.7, 30.2, 30.3, 34.6, 42.6, 43.1, 55.3, 55.4, 55.5, 56.1, 58.1, 58.13, 73.3, 73.4
- **Directed / undirected, targeting, choosing** — 6.8, 17.2, 17.4, 20.1, 24.5, 26.3, 27.2, 32.5, 33.3, 34.8, 35.1, 38.2, 38.6, 39.4, 42.1, 57.10
- **When choices are made (play vs resolution vs use)** — 3.5, 6.7, 22.2, 26.8, 28.5, 31.2, 31.3, 32.11, 32.12, 33.4, 38.1
- **Bleeds** — 18.4, 23.5, 24.1–24.4, 25.6, 30.5, 31.2, 32.2, 32.4, 32.8, 33.9, 34.4, 34.5, 39.5, 42.3, 42.7, 53.6, 53.8, 57.1, 57.2, 61.2
- **Stealth, intercept, blocks** — 1.3, 1.6, 3.2, 3.4, 8.1, 9.6, 15.2, 23.1, 24.5, 25.3, 25.5, 27.3, 30.1, 30.3, 30.4, 23.6, 34.1, 34.2, 35.1, 42.4, 42.8, 48.1, 48.3, 48.5, 48.6, 49.5, 50.4, 52.8, 53.1, 53.2, 53.3, 55.7, 66.2, 72.1–72.4, 73.8
- **Combat structure, strikes, maneuvers, presses** — 1.2, 4.1, 4.3, 4.4, 4.5, 10.1, 10.2, 10.4, 10.5, 14.1, 14.2, 19.5, 21.1, 22.1, 22.3, 26.2, 26.6, 26.7, 27.5, 29.7, 29.8, 29.9, 31.7, 33.6, 42.2, 47.2, 48.2, 48.4, 53.3, 54.4, 54.6, 54.7, 54.8, 55.1, 55.2, 55.8, 57.3, 57.4, 58.2, 58.17, 58.18, 59.2, 59.17, 60.11, 61.1, 62.1, 63.4, 63.6, 63.8
- **Frenzy cards** — 28.11, 29.5
- **Damage, prevention, immunity** — 4.2, 10.3, 14.2, 21.3, 29.1, 29.2, 29.3, 29.4, 30.6, 38.3, 38.8, 40.9
- **Rushes (entering combat as an action)** — 17.3, 26.1, 27.2, 27.5, 38.6, 52.6, 54.3, 54.4, 54.7, 55.7, 56.2, 58.6
- **Referendums, votes, political actions** — 1.1, 3.5, 5.1–5.9, 6.2, 6.3, 15.2, 16.5, 19.3, 19.4, 32.9, 33.2, 35.4, 38.6, 39.1–39.4, 42.4, 49.1, 49.2, 50.5, 50.6, 51.1, 51.2, 51.5, 51.6, 51.8, 52.2, 52.3, 52.5, 52.7, 52.9, 52.10, 54.1, 58.8, 58.14, 58.15, 58.16, 59.7, 59.9, 59.18, 59.19, 60.2, 60.14, 60.16, 71.25, 73.4
- **Titles** — 3.1, 13.2, 19.1, 19.2, 19.6, 32.9, 49.4, 52.5, 52.6, 52.9, 58.3, 58.11, 59.13, 59.14, 60.1
- **Contests, uniqueness** — 3.1, 7.1, 7.3, 7.5, 8.2, 11.2, 12.3, 19.1, 36.3, 38.7, 39.4, 40.1, 57.9, 58.5, 58.19, 59.13, 60.1, 60.9, 60.20
- **Control, steals, ousts, removal from game** — 6.9, 7.5, 17.4, 25.1, 27.3, 27.4, 27.7, 28.13, 31.4, 31.5, 31.6, 33.5, 33.7, 38.4, 39.4, 42.7, 55.4, 55.6
- **Costs and reductions** — 3.6, 5.9, 9.3, 18.1, 18.2, 23.4, 29.6, 27.6, 36.1–36.5, 38.5, 42.1, 42.2, 42.5, 42.6, 54.1, 58.12, 59.5, 60.5, 60.7
- **Replacement, draws, stored and held cards** — 5.5, 7.10, 8.2, 8.3, 8.4, 9.4, 9.5, 11.1–11.3, 12.1–12.5, 22.2, 37.1, 40.4, 53.4, 53.5, 73.1, 73.2
- **Card destination (ash heap, limbo, lingering effects)** — 21.1, 22.1, 31.6, 46.4, 47.1
- **Equipment and weapons** — 9.1–9.6, 10.1–10.5, 17.4, 18.3, 22.4, 29.7, 35.3, 38.4, 38.5, 38.7
- **Allies, retainers, ghouls** — 22.2, 28.10, 29.2, 32.4, 32.11, 32.12, 32.13, 37.1, 40.10, 42.2, 42.9, 49.3, 50.6, 73.5, 73.6
- **Locations, hunting grounds, locquipment** — 6.5, 7.2, 7.4, 17.4, 28.2, 28.6, 31.4, 34.7, 39.4, 49.5, 50.4, 51.4, 52.7, 54.1, 57.7, 73.3, 73.6
- **Blood, capacity, uncontrolled region, banishment** — 5.7, 7.6, 25.1, 27.7, 28.1, 28.3, 28.4, 32.6, 40.7, 73.5
- **Torpor, rescue, diablerie, blood hunt** — 3.1, 3.6, 5.2, 5.8, 6.4, 16.2, 28.7, 38.2, 48.8
- **Hunting** — 6.5, 7.7, 17.3, 28.1, 28.8, 32.13, 40.2, 53.8, 72.1–72.5
- **Master phase, master cards, out-of-turn** — 1.5, 3.3, 6.6, 7.5, 28.10, 32.3, 33.1, 33.8, 36.4, 49.6, 49.7, 50.1–50.3, 53.7, 57.8
- **Unlock phase, locking, stun** — 3.7, 6.1, 7.8, 16.5, 21.2, 25.5, 26.5, 31.1, 48.7, 54.2, 54.5, 58.4, 58.9, 59.3, 59.4, 60.6
- **Discard phase, events** — 8.4, 15.2, 28.12, 33.8, 40.1–40.11, 54.5, 57.5, 57.6, 58.10
- **Conditions read live; card-text defaults** — 4.3, 9.1, 13.2, 18.4, 19.3, 28.2, 28.6, 29.11, 32.4, 32.7, 39.5, 49.5, 49.7, 73.7
- **Corruption counters, multi-Discipline cards** — 25.1–25.6, 34.3, 35.2, 38.4, 52.1
- **Printed vampire abilities** — 13.1, 13.2, 14.1, 14.2, 15.1, 15.2, 16.1–16.6, 17.1–17.4, 41.1–41.14, 49.2, 49.3, 52.4, 72.1, 72.2, 72.5, 73.5, 73.7
- **Game end, VP, ousting** — 6.9, 18.2, 24.1, 33.2, 53.1, 58.20
## Batch 1 — sequencing, impulse, windows

**1.1** Vote casting. A *source* is a vampire or a Methuselah. A vampire source is atomic: all its votes cast at once, in one commitment, all the same way; then spent for that referendum. Sources are independent commitments: a Methuselah may cast one source on one rotation and hold others for later rotations. Methuselah-sourced ballots (the Edge, each discarded/burned card, each card granting Methuselah votes) are each independent and may go either way. Card effects that modify a vampire's votes fold into that vampire's source.

**1.2** Impulse in combat. Impulse-reset: any play by either side returns the opportunity to the acting side at the same step; a later play voids prior passes; the step closes when both sides decline in succession. The acting minion is the minion performing the action (blocked action or unblocked rush); the other is "the opponent" (a "blocker" only if it blocked). Combat with no acting minion exists (Taunt the Caged Beast, Illusions of the Kindred, Siren's Lure, Slave rule): first opportunity goes to the acting Methuselah's minion, else clockwise from the acting Methuselah.

**1.3** Block loop.
- A block attempt does not fail at a stealth ≥ intercept checkpoint; it fails only when all Methuselahs pass in succession. While it lasts, the defender may stack intercept or pass; the impulse moves on to others who may help (KRCG News Radio intercept; Draba / Night Terrors stealth removal), then the defender may continue (impulse-reset). No "re-declare": the attempt persists until closure.
- Failing to block = passing. Declining (passing with no ongoing attempt) is final: later stealth removal by a third party does not let them block (e.g. via Second Tradition).
- Only a change of the action's target (bounces: Deflection, Telepathic Misdirection…) reopens block opportunities. The card Change of Target does not change the target — it ends the action.
- Third parties act when their cards allow and the impulse reaches them — normally after acting and target Methuselahs both passed. Only the acting minion plays action modifiers (some cards, e.g. Cloak the Gathering, let other minions of the acting Methuselah). Anyone may play reactions with an unlocked/awoken minion. "+X intercept" applies to the minion playing the card; granting intercept to another's minion (KRCG News Radio) is the exception.
- No bait/retraction: sequencing orders effects. A third party wanting to strip stealth needs the defender to attempt first (e.g. Second Tradition); once attempting, the defender needs no matching intercept to keep the attempt alive.

**1.4** No-repeat-action (NRA). A minion cannot perform the same action twice per turn, successful or not; the trigger is *resolution* (blocked or failed-referendum actions resolved; an action canceled as announced, e.g. Direct Intervention, did not and may be retried). Identity: card from hand = card **name**, per minion; card in play = the **copy** (two Anarch Revolts = two actions). Type cap: one bleed and one political action per minion per turn, whatever provides it; other rulebook actions (hunt, leave/rescue torpor…) are exempt (a minion may hunt twice). Card-text "cannot perform the same action" (Change of Target) also covers the rulebook actions NRA exempts. The equip-from-own-minion action is NRA-exempt and repeatable, but fetching the same equipment again is "the same action" for such card texts.

**1.5** Master phase. Other Methuselahs may only cancel in the as-played window and play out-of-turn masters (no wakes without an action). The master phase is the owner's offer window, punctuated by cancel windows, and closed by an impulse rotation over the other seats for their untimed and mandatory effects (7.9). Out-of-turn accounting: 3.3.

**1.6** "Only when needed" stealth/intercept — settled in 3.2.

## Batch 2 — window-matrix arbitration

**2.1** "After block resolution" plays (Cats' Guidance [ani], Forced Vigilance [for]) are after-combat plays, after **all** combats of the action (e.g. Psyche!): the after-combat window opens once, after the last combat arising from the block.

**2.2** Combat cards with no printed step (Claws of the Dead [pro], Wolf Claws [pro], Pack Alpha [ANI]): one untimed combat window with an effect-derived deadline — the play must precede the step it modifies (advanced rules §2.1.4); aggravated-hands plays precede the strike resolution they affect.

**2.3** Vocabulary accepted: block attempt pending vs blocked-before-resolution split; `CARD_AS_PLAYED` (as-played cancel sub-window); out-of-turn is a card flag (Archon Investigation, Sudden Reversal), not a window.

**2.4** Wakes vs unlock reactions. True wakes (Wake with Evening's Freshness, On the Qui Vive, Forced Awakening, Eyes of Argus [AUS], Shadow Sentinel) are playable in the as-played window — e.g. before a card is canceled (Rewind Time) or before a modifier's effect resolves (wake before Consign to Oblivion raises the wake's cost). Unlock-and-attempt reactions (Dogged Pursuit, Eyes of the Wild, Second Tradition 2nd mode, Sense the Savage Way [ANI], One With the Land [PRO], Organized Resistance) and plain unlock reactions (Guard Dogs, Rat's Warning, Forced Vigilance [FOR]) are not. The as-played window admits **only wakes and cancellations** — not even card abilities (locking The Barrens). It opens after every card play, during an action or not (Rewind Time is usable outside actions). Wakes need an action: an action card just played (still in its cancel window, not yet replaced) has already opened it, so wakes are playable there; a wake resolves before the played card's effect. Outside an action (master phase…) the window admits cancels only. Organized Resistance's first mode (locked baron gives +1 intercept) is an intercept play usable while locked. Second Tradition / Organized Resistance's "even if intercept is not yet needed" is evidence "only when needed" is the default (3.2).

**2.5** Strike-card riders (owner, 2026-08-04): the rider's timing relative to the strike step decides the play window. A strike with an optional *maneuver* rider (Aid from Bats, Slam, Diversion) is playable at determine range, committing the strike early (DESIGN §4.4); one with a *press* or *additional strike* rider is played at strike declaration (Hidden Strength's press is a rider, not a mode — unlike Flash, Indomitability, Resist Earth's Grasp, whose "or press" wordings are real press modes). Any card's riders firing after its own play step add no play window.

## Batch 3 — P0 open items

**3.1** A contested vampire in torpor stays in torpor through the contest (face down, out of play: no rescue or diablerie target); if it wins it returns to torpor. Two vampires under one Methuselah do contest a title (the same-controller exemption covers cards only).

**3.2** "Only when needed" is a legality rule (closes 1.6): stealth only while a block attempt is pending with intercept ≥ stealth; intercept only when stealth exceeds it; surplus plays are illegal; no pre-emptive stealth without a block attempt. Exceptions: card text "even if … not yet needed" (Form of the Cobra [PRO], Paths in Two Worlds, Form of Mist's continuation, Creepshow Casino; intercept: Organized Resistance, Second Tradition), and permanent bonuses rather than stealth added to the current action (Loup: Toreador get +1 stealth while in play — playable with no block attempt).

**3.3** Out-of-turn masters (Archon Investigation, Sudden Reversal): count toward the player's next master phase. An out-of-turn trifle also consumes that next MPA and gives the extra MPA only when the player reaches their master phase. So a player can never play two out-of-turn cards between their turns.

**3.4** Form of Mist continuation: declines are per Methuselah, not per minion. After the continuation, earlier declines stand (same action): a target that declined while another blocked still cannot block. Only the pending attempt resets (1.3).

**3.5** Numeric terms are chosen and announced when the card is played (Villein's "2 to 5 blood") — they may change others' play. For a referendum (Kine Resources Contested's allocation) the terms are announced as the referendum's terms: after the action succeeds, before votes.

**3.6** A cost paid at resolution (rescue's / leave torpor's 2 blood) that became unpayable: the action *fizzles* — it is successful (not blocked; e.g. Freak Drive playable) but has no effect.

**3.7** Unlock-phase impulse: the active Methuselah does all the unlock-phase effects they want, then passes the impulse so others' unlock-phase abilities (Homunculus: "during any Methuselah's unlock phase") can be used; a use returns the impulse to the active player.

## Batch 4 — combat (slice 3)

**4.1** The Form of Mist [PRO] continuation happens *inside* the after-combat window (occupants order freely, acting Methuselah first; the blocker may unlock before or after it); the block loop resumes when the window closes, so an unlocked blocker may block again (the maneuver is provided again on a second block).

**4.2** Mixed normal and aggravated damage: when a prevention covers only part, the preventer chooses which points go unprevented (one may *want* torpor with no blood lost). Non-aggravated-only prevention (Soak) never touches aggravated points.

**4.3** Ineffective plays stay legal: hand strike at long range, dodge with no opposing strike; the mask removes only what rules or card forbid ("cannot be played with no damage to prevent", "only when needed", limits). Cycling an ineffective card is a common purpose — never mask it. "No strike" is rare (a strike must be chosen, default hand strike): a committed strike that can't be performed, or a card letting the minion not strike.

**4.4** ~~A limited additional strike card (Quickness, Blur, Side Strike [CEL], Pursuit [CEL]…) may be played before or after the normal pair — general rule.~~ **Overridden by 47.2:** only after the normal pair.

**4.5** A combatant no longer ready (burned / going to torpor) may play end-of-round combat cards unless the card excludes it (Taste of Vitae, Disarm), and no other combat window.

## Batch 5 — politics

**5.1** Votes gained after casting (Bewitching Oration) join the vampire's cast on the same side, no new commitment; before casting they fold into the source.

**5.2** "Burn a political action card for 1 vote" is available in a blood hunt too, one per Methuselah per referendum.

**5.3** A card played during polling is a commitment of the rotation: it reopens a response opportunity for every other Methuselah before the tally.

**5.4** The calling political action card gives the acting Methuselah 1 vote, either way, as an independent ballot, consuming their one political-action-card vote.

**5.5** Delaying Tactics / Telepathic Vote Counting's "discard down afterward" is immediate, not in the discard phase. The action ends successful (no referendum effect), the after-resolution window opens, and the vampire has taken its political action this turn.

**5.6** Parity Shift shortfall: when the chosen Methuselah has less than 3 pool, the caller re-allocates what there is, each recipient capped at its term amount [LSJ 20010606].

**5.7** Banishment: the vampire goes to its controller's uncontrolled region with its blood as counters; cards on it leave play but stay face down on it and come back (face up, unlocked) when it is re-influenced.

**5.8** The after-resolution window opens after every successful action (Voter Captivation, Freak Drive); for a political action, after the referendum; after a diablerie too (after the blood hunt, which is inside the action). Every action has one, blocked or failed included (23.3).

**5.9** A political action whose acting vampire is no longer ready at resolution ends with no referendum (Yawp Court ruling); the card's cost is still paid.

## Batch 6 — cards in play (riders, granted actions)

**6.1** "During their unlock phase" mandatory effects (Anarch Revolt's burn) must resolve before the impulse passes to another player; the active Methuselah orders them before or after their other unlock-phase effects; the condition is read when the effect resolves.

**6.2** A referendum/action granted by a card in play (Anarch Revolt): a political action in every respect — locks, blockable, the one-political-action-per-minion cap. Stealth is **not** implied: the default +1 stealth applies to political action *cards* only; a granted action has only the stealth its text prints (directed or not). NRA is keyed to the *copy* (1.4). No calling card, so no card vote for the caller. Delaying Tactics cancels it (nothing to return to hand). Its "same political action" ban uses the ordinary NRA identity (23.2): the card's name from hand, the copy for a card in play (another copy's referendum stays open, LSJ 20020201).

**6.3** A passed "burn this card" referendum burns the copy whose action was taken (to its owner's ash heap); if that copy already left play, nothing.

**6.4** Fame ("after this vampire goes to torpor, burn 3 pool") resolves at once inside whatever sent the vampire to torpor (mid-combat, before the after-combat window); stacks with Tension in the Ranks in the controller's chosen order.

**6.5** "Gain blood from only one hunting ground each turn" is per vampire: two grounds may feed two vampires; the vampire is chosen as the ability resolves.

**6.6** "During X, do Y" in card text means once per X (per copy): Blood Doll moves 1 blood once per master phase; Vessel once per unlock phase.

**6.7** Vessel's "burn a card named Blood Doll (if any)": any Blood Doll in play (card does what it says). The choice is announced when the card is played (like targets and terms, 3.5); the burn is part of resolution, not a trigger.

**6.8** "Minions can burn this card as a Ⓓ action" (Creeping Sabotage): the controller's own minion takes it undirected (prey and predator may block); another's minion takes it directed at the controller; stealth only what is printed (6.2).

**6.9** Simultaneous oust of every remaining Methuselah is legal. Every Methuselah in play at that moment scores the VP for their ousted prey, ousted themselves or not; the 6 pool goes only to a non-ousted predator. Game win = ≥ 2 VP and strictly more than any other, ousted or not (a 3-VP player ousted by their last opponent, who scores 2, still wins). Draws are legal (two at 2 VP; four or five all at 1 VP).

## Batch 7 — library-card contests, hunting grounds

**7.1** No self-contest: a unique card cannot be played while a copy is in play under the same Methuselah — including while that copy is contested (out of play, but still contesting incoming copies). Also bars a search from placing such a unique (11.2, 12.3).

**7.2** A hunting-ground pick on a vampire at capacity is legal (no effect) and counts as its one hunting ground this turn; kept offered, may be pruned (brings no cycling).

**7.3** A contested card's continuous effects are suspended, not ended (a contested Guardian Angel gives no intercept/prevention); they resume if the contest resolves in its favor.

**7.4** Carfax Abbey's two picks ("… and, if you control a ready baron, another ready Anarch … as well") are independent once-per-phase uses; the second may be taken without the first.

**7.5** Control of cards on minions: a *minion* card on a minion is controlled by the minion's controller; a *master* card (Fame) stays controlled by the player who played it (it contests that player's copies; it is removed when that player is ousted). A card on a removed vampire is burned (to its owner's ash heap).

**7.6** Smiling Jack's "burns 1 blood from a vampire they control": torpid vampires are sources (never an empty one, LSJ 19971212); one vampire may pay several counters.

**7.7** Untimed "lock this card" abilities (The Barrens, Dreams of the Sphinx) are usable at any point their controller holds the impulse — all own phases, every seat's unlock-phase rotation, every action/combat window, polling (as a commitment) — never in the as-played window (LSJ 20061207) nor inside a choice that is part of a resolution (terms, rider payments, diablerie choice, rescue cost split, discard-down). An untimed ability may be used *before* a forced hunt (The Coven's blood sparing the hunt). Dreams' third lock: its effect lands, then the card burns at 3 counters. The Edge's pool is a normal unlock effect usable anytime in the unlock phase (7.8).

**7.8** Unlock effects are freely ordered by the active Methuselah: contest upkeep (pay/yield), mandatory riders, the Edge's pool, abilities. Contest upkeep is mandatory (the impulse cannot move on until decided); the Edge's pool is optional (passed and not taken: lost). Unlocking cards happens at the start of the phase, not ordered.

**7.9** Every phase of the active Methuselah's turn ends on an impulse rotation over the other seats (as in the unlock phase): their mandatory effects first (The Coven's transfer to the predator, LSJ 20100723), then untimed abilities; an effect returns the impulse to the active Methuselah; the phase ends once all pass.

**7.10** Constant Revolution (LSJ 20090618): the random cards go all at once ("no replace" = not one at a time, capped by hand size, the rest from pool), then the hand is drawn back up.

## Batch 8 — cards & effects (replacement, events)

**8.1** Bonding [DOM]: the superior mode (+1 stealth and +1 bleed) is playable only when stealth is needed [TOM 19951109]; the [dom] bleed bonus alone is playable before any attempt.

**8.2** When a hand-size bonus ends (Dreams of the Sphinx's +2 at end of turn — not during the discard phase, LSJ 20100812; an Elder Library becoming contested), the hand is discarded down at once (rulebook "Drawing Cards").

**8.3** Visit from the Capuchin counts every card the rules replace, whatever took it out of hand (played, canceled, discard phase action, political card burned for a ballot, Tension in the Ranks, delayed replacement coming due — Old Friends), including The Barrens' "draw up afterward" and Constant Revolution's refill. A draw-up after a hand-size increase is not a replacement. With an empty library no card is replaced, so no counter burns.

**8.4** Mirror Walk: "do not replace until your next discard phase" played in one's own minion phase is replaced as that turn's discard phase begins. [THA] "end the action before block resolution": the action counts as blocked (NRA identity kept, blocker locked), no combat, nothing more played on it.

## Batch 9 — equipment

**9.1** Text naming the bearer's clan/sect is read on the current bearer: a non-Ravnos holding Treasured Samadji (taken from another minion, ANK 20180719-3) gets neither bonus; likewise Stolen Police Cruiser's "This Anarch" parts, Shilmulo Tarot's "While this Ravnos is ready" (12.5).

**9.2** The equip-from-another-minion action may take equipment from a minion in torpor. "Only one vehicle": a second vehicle is never offered (from hand, another minion, or two in one equip). A blocked equip action leaves the equipment.

**9.3** Magic of the Smith offers only equipment the vampire can equip and whose blood cost it can pay (a search may find nothing; PIB 20121028); a pool cost that ousts is allowed. Vast Wealth ("the first equipment you find"): no choice; if requirements are unmet, it can't be placed, or its blood cost is unpayable, it is burned without cost (TOM 19950407); a pool cost that ousts is paid (RTR 19941109).

**9.4** Draw alternatives (Shilmulo Tarot, Black Market Cache): each card drawn from the library (replacement, draw-up, Heart of Nizchetus) may instead be taken from the stored cards, a choice per card; with Visit from the Capuchin, one treatment per replacement (ANK 20200905); an empty library means no draw, so no stored card either.

**9.5** Heart of Nizchetus: the player chooses the order of cards put at the bottom (count: 11.1).

**9.6** Bowl of Convergence's "burn 1 blood during an action for +1 intercept": once per action ("during" template, 6.6), only while the bearer is attempting to block (intercept needed, 3.2).

## Batch 10 — weapons

**10.1** A gun's "1 optional maneuver each combat" (.44 Magnum, AK-47, Assault Rifle) commits the round's initial strike to that gun (rulebook; LSJ 20090528-2-2, PIB 20141026): no other initial strike that round, none at all if an effect then bars it (Immortal Grapple), no second strike card/weapon maneuver that round; additional strikes free. The allowance tracks the gun (LSJ 19980302-2).

**10.2** AK-47's additional strike comes once the gun's strike resolved, dodged or not (not if a "strike: combat ends" resolved first); it is the bearer's one limited additional strike that round (none if Blur already used; Blur not playable after); in that pair: gun strike or no strike.

**10.3** Kevlar Vest: gun or non-gun branch chosen once each combat; the prevention may be split across rounds/exchanges of the combat (LSJ 20081210; ANK 20200318, LSJ 20001114); leftover gun points prevent nothing else. Gun damage not from its strike is not "damage from gun strikes".

**10.4** Righteous Blade's press: had once the strike is chosen — kept if the strike is dodged or does not resolve; once per combat per Blade.

**10.5** Sniper Rifle: offered to the blocker before range in the first round, only while no other effect set that round's range, and only if the bearer had the rifle when blocking (not equipped afterwards, e.g. Concealed Weapon); used, it skips determine range (RTR 19970630, ANK 20180720) and commits the initial strike to the rifle.

## Batch 11 — stored cards, draw alternatives

**11.1** Heart of Nizchetus: the number (up to 3) is picked before drawing, capped by the library's size; the cards are drawn at once (each a draw-alternative choice, 9.4), no discard-down in between; then that same number goes to the bottom.

**11.2** Shilmulo Tarot: on equip it takes the top 2 cards (fewer if fewer there); its unlock-phase move is once per unlock phase per Tarot, whoever bears it. The Tarot is unique (one per Methuselah; 7.1).

**11.3** Black Market Cache: 0 to 3 non-unique equipment chosen one at a time as it resolves; "no cards on it, burn it" is checked whenever empty — at once after a search finding none, and as its last card is drawn.

## Batch 12 — readings of stored cards

**12.1** With the Capuchin and a stocked draw alternative, each replacement is the Capuchin counter or a stored card — not the library top (the Capuchin's "instead" applies unless the other treatment is chosen; ANK 20200905).

**12.2** Shilmulo Tarot stores its 2 cards when equipped from hand or by a search (LSJ 20090506); taken from another minion (moved, not played) it stores none.

**12.3** A search does not offer a unique equipment already in play under its Methuselah (7.1); Vast Wealth's first find being such a unique is burned as unplaceable.

**12.4** Black Market Cache is replaced *before* its search, so its replacement is never one of the cards the search puts on it.

**12.5** A non-Ravnos bearer of Shilmulo Tarot (ANK 20180719-3): its Methuselah draws nothing from it ("while this Ravnos is ready", 9.1), but the unlock-phase move still applies.

## Batch 13 — printed vampire abilities

**13.1** Every vampire must end with all its printed abilities; implementation order is the implementer's call (a vampire may land with part of its text).

**13.2** Plain printed bonuses:
- Read live while the vampire is in play, in torpor too (LSJ 20011022), not while contested.
- "+1 strength" adds to base 1 or to a strength an effect sets (Torn Signpost); "in combat with X" reads the opposing minion.
- "Intercept against X" (Azucena, Bret Stryker) reads the acting minion; "against titled vampires" reads the effective title (a contested/yielded title does not count); a primogen is titled.
- "During directed / undirected actions" (Castellan, The Dowager, Ariane) reads the action performed, whoever performs it.
- Hesha Ruhadze counts uncontested unique equipment on him; Neserian counts uncontested locations, hunting grounds included; Branimira reads pool when intercept is compared.
- "While X is ready" (André the Manipulator, Carmelita Neillson, Weirich Waldburg) is read live: lost in torpor; an over-size hand is discarded down at once (8.2).

## Batch 14 — printed combat abilities

**14.1** Adrino Manauara's mandatory press: at the press step his controller must use it to continue before passing, unless a press to continue already stands, in which case it waits for a later round of the combat.

**14.2** Other printed combat abilities:
- "1 optional maneuver / press each combat" (Abaddon, Saku Pihlajamäki): one use over the whole combat, any round, under the no-two-in-a-row rule.
- "Once each combat, strike: dodge" (Flávio Gonçalves, Phaibun) is chosen as the strike; Immortal Grapple bars it. Phaibun's random discard happens as the strike is chosen, then replaced; not offered with an empty hand.
- Opikun: usable in torpor (LSJ 20011022) and on himself; blood burned as used, only while non-aggravated damage is pending on that vampire in its prevention step; non-aggravated only (LSJ 20040812-2).
- Rowena: prevents 1 non-aggravated once each round, in her prevention step.
- Kasim Bayar: political card picked as used, then replaced; +2 strength for every round of that combat, adding to a set strength.
- Chun Hei: usable where additional strike cards are; "does not count against the limit": a limited additional strike card still fits that round.
- Noluthando: +1 damage on each of his ranged strikes (weapon or card) at either range, not hand/melee; on a gun it is gun-strike damage (Kevlar Vest).

## Batch 15 — printed abilities outside combat

**15.1** Gathii's reveal: once per action (the "during" template, 15.2); usable before stealth is needed, the +1 stealth then not gained (ANK 20231031), so the master-card blood burn is the only effect.

**15.2** Printed abilities outside combat:
- **"During" template:** most abilities print "during X" and are once during it — not repeatable while payable. Abraham DuSable (each mode), Oskar Anasov, Gwenllian Arwyn, Larissa Moreira: once per action; Kalinda: once per turn.
- **Bonus stealth/intercept lasts the whole action** and ends when the action ends — including intercept from a reaction card (Eyes of Argus): kept by the minion across a failed attempt into a new one.
- Referendum abilities (Aisha az-Zahra, Alexa Draper, Yewon Ong, Queen Anne): once per referendum, usable while locked (advanced rules §1.2.1, ANK 20220705); those with no condition also in torpor (§3.7.6.4) though a torpid vampire casts no votes; "any referendum" includes blood hunts. Aisha counts every ready Anarch her controller controls (herself, locked ones) as it resolves. "A card requiring Dominate" = any library card whose disciplines include it (either-or and combo too).
- Queen Anne: "a referendum called by Anne" = her political action's referendum, not a blood hunt; target any younger Camarilla vampire of any Methuselah, torpor included, not one already abstaining.
- Printed stealth/intercept obeys "only when needed" (3.2): Abraham DuSable (stealth on his action / intercept on his own attempt), Daria (lock only while the acting Nosferatu — anyone's — needs stealth), Oskar Anasov (+1 intercept to his controller's blocker needing intercept), Gwenllian Arwyn (-1 intercept to the vampire blocking her action while stealth is needed); Oskar's and Gwenllian's last the rest of the action.
- Larissa Moreira: on her own bleed only, before resolution (ANK 20231124).
- Nonu Dis: once per master phase (ANK 20220617), usable any time in it once a master card was played (as announced, canceled or not); target any ready Ministry vampire of his controller, himself included.
- Mora, the Death Seer: once per discard phase; any library card in the ash heap, chosen by name.
- Sreelekha: +1 discard phase action if ready as the discard phase begins. Ayse Dhanial: each use spends a discard phase action (twice with Sreelekha's); not offered at capacity.
- Kalinda: whenever her controller holds the impulse on their turn, while she is locked, in torpor too; not during a combat she is not in (a vampire outside the combat is not offered its abilities there).

## Batch 16 — printed abilities after an action

**16.1** "After …" abilities with no periodicity (Richard de Worde, Alicia Cortez): once per action. Sybren van Oosten's and Queen Anne's unlocks self-limit (must be locked).

**16.2** Queen Anne can unlock after a diablerie as the blocker of a leave-torpor action too.

**16.3** Looking at a hand: Richard de Worde's use chooses the card and burns the blood (the discarded card goes to its owner's ash heap and is replaced); Ilonka looks, then her controller chooses to discard at random or not — while ready, after a successful bleed against her controller, once each turn.

**16.4** Aline Gädeke: the acting minion's sect read after resolution; usable in torpor (LSJ 20011022); a fizzled action counts (ANK 20220218); once each turn, whoever's turn. Alicia Cortez: "directed" as for bleed, rescue/diablerie of another's vampire, a burn action on their card; target any younger vampire (torpor too) of the Methuselah the action was directed at — a locked one may be targeted (legal no-effect plays are kept, 4.3).

**16.5** Riders and printed votes:
- Roy ("if unlocked during your discard phase, lock him"): applies as the discard phase ends, in torpor too.
- Cedrick Calhoun: goes to torpor as the action resolves (before the after-resolution window) if the referendum failed or was canceled, while ready.
- Egidia Arrú: a minion in combat with her leaving the ready region (burned/torpor, by anything during the combat) costs its controller 1 pool, even if Egidia falls in the same event (advanced rules §6.4.4).
- Marialena: the blood burns as the block resolves, with the acting minion's "if blocked" riders, before combat or the blocker's diablerie choice.
- Jason "Son" Newberry: +2 only on votes against a blood hunt; with no title votes he still casts 2 against one.
- Styles Margs: +2 in favor stand as the blood hunt is called on him, a vote no one casts.

**16.6** Evan Klein's tails: the acting minion still locks, the action counts for NRA and the bleed/political caps, cost and blocks are skipped, as-announced plays remain possible before the flip. Alabástrom: once per turn, any other ready Gargoyle; Flight is a bare trait for now.

## Batch 17 — printed actions

**17.1** Printed actions:
- Sakhar, Dominica: card chosen at resolution, any of the kind (Sakhar: any equipment, equippable or not; Dominica: a master card labeled archetype), revealed to hand or none found; library shuffled either way; hand size not checked then.
- Seraphina: "a minion you control" includes herself and torpid vampires; blood stops at capacity. Undirected.
- Lenelle: both cards named at announcement (LSJ 20000222); not offered with an empty hand or ash heap. If a named card left its zone by resolution: successful, blood paid, no exchange.
- Elen Kamjian: "if you control a locked minion" counts any minion, torpor included. While it holds and she has not bled this turn, her bleed is mandatory (other actions wait; the minion phase cannot end); with 0 blood the rulebook hunt takes over. If she already bled, she declares no action but may block and react.

**17.2** Between two mandatory actions (Elen Kamjian's bleed, a mandatory hunt) the acting Methuselah chooses the order. **Directedness:** an action is directed at the Methuselah whose hand, library, crypt, uncontrolled region or controlled card (any card: stealing The Rack is directed) it targets. Actions targeting an ash heap are undirected.

**17.3** Rush (enter combat as a Ⓓ action — Theo Bell, Barachiel, Dafina Hanganu, Nathaniel Bordruff, Adisa): target a ready minion of another Methuselah (not in torpor), locked or not unless printed (Nathaniel: locked; Adisa: younger); directed at its controller, who alone may block. Unblocked: combat at resolution, part of the action, acting minion first; the combat has its after-combat window like any combat, then the after-resolution window. "Continue as if unblocked" effects need a blocked action; a blocked rush can use one, and the continued action then enters combat with its target at resolution. Target no longer qualifying at resolution: successful, no effect. Adisa "successfully hunts" only when the hunt gains blood (advanced rules §3.7.2.1).

**17.4** Steals and card targets (Apolonia Czarnecki, Darius Shirazi, Saankaláxt, Aniel):
- The Ⓓ icon is a reminder: targeting one's own minion/card makes the action undirected (Apolonia on her own younger vampire, Darius/Saankaláxt stealing from their own minion, Aniel burning a card on their own ready minion). (Unlike rushes, which need another's minion.)
- Apolonia: any younger vampire, torpor or no blood included (takes what is there); excess over capacity drains off.
- Stolen equipment changes host (controller follows, owner stays); no requirement checked (moving is not equipping). A locquipment (Living Manse) is a location in play: not stealable as equipment nor taken by the equip action, but stealable by cards that steal locations. Targeting a card on a minion does not target the minion (Evan Klein does not flip).
- A steal that would give a second vehicle is not offered. Principle: an action sure to aim at an illegal target is not allowed; one whose object comes from a choice/randomness at resolution (Vast Wealth's search) may be taken even if it will fail.
- Target gone by resolution (burned, moved, not ready, no longer younger): successful, no effect, cost paid.
- Darius: "non-weapon" excludes melee weapons. Camera Phone and Laptop Computer (electronic) may join the pool.
- Aniel: "a card requiring a Discipline" = any library card printing a discipline requirement.

## Batch 18 — precon coverage

**18.1** A canceled card pays no Parijat toll (ANK 20260317; supersedes ANK 20260216's Eagle's Sight example): the toll is paid as the forcing card resolves.

**18.2** Burning one's last pool to cancel True Love's Face (ousting mid-play) is not offered.

**18.3** Form of the Bat's "cannot use equipment during the resulting combat" bars all equipment use (there is no passive combat bonus without a strike).

**18.4** Conditions are read live, never snapshotted (Aire of Elation, Protection Racket, The Warrens) — including whether a limited bleed counts: Foreshadowing Destruction [DOM] counts toward the limit only while its target has ≤ 9 pool (played at ≥ 10 it lingers outside; bounced onto ≤ 9 after Conditioning, both apply). Exception: Night Terrors' fixed -X (RTR 20030519).

## Batch 19 — titles and votes

**19.1** Saulot's Guiding Wisdom ("Unique archetype. Title."): the unique card contests first (paid in pool), so the title does not; while contested its vampire has no title from it (face down, neither copy's title is claimed). A non-unique title card (Toreador Justicar) contests as its title, paid in blood.

**19.2** A title card replaces a printed title for good: a prince given Toreador Justicar loses Cape Town for the rest of the game; once the card is gone he is untitled.

**19.3** Vote grants from locations/cards (Elysium: The Palace of Versailles, Power Structure, Toreador Justicar's "+1 vote in this referendum" for every ready Toreador, an Anarch's included) are read live: a vampire that already cast adds the vote to its cast; one casting later gets it if it qualifies then.

**19.4** Ventrue Headquarters' "lock … to get +3 votes": the Methuselah's own votes, all on one side as the card locks, like the Edge.

**19.5** Saulot's Guiding Wisdom ending a combat: no strike resolves (advanced rules §4.7.2); the end-of-round window still opens (§4.9).

**19.6** Valerius's steal: the prince loses the title (its card, if any, is burned); Valerius gets the title with no card, losing any title he held; a baron of the same city would contest it.

## Batch 20 — archetypes

**20.1** Ⓓ, printed or not, is only indicative: an action targeting your own minions is undirected; one targeting another Methuselah's minion is directed at them (Saulot's Healing Touch). See 17.2 for non-minion targets.

## Batch 21 — combat cards

**21.1** A strike works at close range only unless it says otherwise (Touch of Oblivion [obl], Sculpt the Flesh [PRO]: close range). Such a strike's "put this card on the opposing minion" happens only if it lands (dodge or early combat end: no effect). **Card destination rule:** only action cards wait in limbo (until successful or blocked); every other card goes to the ash heap as it resolves unless its effect puts it in play — even when part of its effect lingers or is delayed (cards left on the table are display only). The lingering effect applies regardless; burning it ends the effect and moves no card (Touch of Oblivion, Sculpt the Flesh). Revised by 47.1 (after 46.4): a card whose text puts it in play ("put this card on …") goes there itself, out of the ash heap — the general rule for all such cards, future ones included.

**21.2** Touch of Oblivion's "instead of unlocking as normal" (advanced rules §5.2.2 substitution): the minion burns a counter even if already unlocked; one kept from unlocking by another effect (Stolen Police Cruiser) burns none that phase; at 0 counters the card is burned and the minion unlocks normally from the next unlock phase.

**21.3** Rolling with the Punches [FOR] covers the opposing minion's additional strikes later in the round too; not environmental or retainer damage; playable only with strike damage pending (LSJ 20001114).

## Batch 22 — combat cards with their own mechanisms

**22.1** A strike card with a maneuver (Slam [POT], Aid from Bats) played in determine range gives the maneuver at once and commits the round's initial strike (only that strike offered at strike choice); the card goes to the ash heap as it resolves. Slam's maneuver only closes from long range. Not playable at the maneuver while the strike could not be used (LSJ 20021028).

**22.2** Pack Alpha [ani]: the retainer (and its version) is named as Pack Alpha is played, among those the vampire could employ now (requirements, cost); at resolution, if still in hand, it is paid and put in play and replaced like a card played from hand. Not an employ action, so employ-action riders do not apply (Feral Hound's unlocks).

**22.3** Voracious Vermin [ANI]'s additional strike (burn weapon): the superior version can be chosen only if the opposing minion has a weapon; not optional once chosen (the only choice is which weapon); none if no weapon is left by then. Any range; a dodge stops it.

**22.4** Weighted Walking Stick counts as weapon equipment while in play (Blood Rage / Soul Burn nullify it, Form of the Bat bars it, Voracious Vermin burns it); in hand or ash heap it is just a combat card.

## Batch 23 — action modifiers and reactions

**23.1** Change of Target is played before the blocker is locked (LSJ 19980224, RTR 19991206): a blocker unlocked when it blocked stays unlocked. The action ends as blocked: no combat, no diablerie.

**23.2** Card text "cannot perform the same action again this turn": for a card action, the ordinary NRA identity; for a rulebook action, the same kind at the same target (rescuing a different vampire stays open, RTR 19950509).

**23.3** **Every action has an after-resolution window**: a blocked action after combat; an action ended by Change of Target or Mirror Walk (no card may be played there, RTR 20180719, but Daring the Dawn's damage is taken there); a failed action (ANK 20221205). Daring the Dawn's damage applies after resolution, ordered by the player among their after-resolution effects (LSJ 20010715), before passing.

**23.4** Unleashing the Bestial Soul [DOM]: the surcharge applies to the first reaction card paid for after it.

**23.5** Visions of Zapathasura [OBF][PRE] "reduce … to 0": a fixed reduction equal to the bleed amount as it resolves; later increases still count.

**23.6** Veil the Legions [OBF]: X chosen at play (0 to the vampire's blood), burned at resolution; the next X actions of the Methuselah's minions this turn get +1 stealth, counted as announced. "Only one at superior each turn" is one in all (game-wide), a turn being one player's turn.

## Batch 24 — cards remembering a Methuselah

**24.1** A bleed whose target is ousted before it resolves (e.g. by the cost of canceling Spying Mission, LSJ 20050607): the oust is handled first (its predator paid), then the bleed fizzles — no pool, no Edge, after-resolution cards apply.

**24.2** After blocks are declined, a bleed of ≥ 1 opens a "would be successful" window over the action's order (acting Methuselah first): only cards timed "if a bleed would be successful" (Spying Mission) are played, none once the action has failed; after a canceled Spying Mission only such cards follow, never a reaction (LSJ 20050607).

**24.3** Spying Mission [OBF]: "the same Methuselah" = the bleed's target as the card resolves, after redirects. Copies already on the vampire burn only when a later bleed of that Methuselah resolves for ≥ 1, after the would-be-successful window; a second Spying Mission played in that window leaves the first in place (ANK 20200609).

**24.4** A bleed is successful only for ≥ 1: a bleed of 0 takes no Edge and triggers no "after a successful bleed" (Ilonka, Richard de Worde), but the action itself succeeded (Freak Drive [for] may follow).

**24.5** Shadow Cast [OBL]'s burn is a stealth effect (only when needed; not a Loup-style bonus), by its vampire during its own action. An action may be directed at several Methuselahs: the burn is allowed when the recorded set (its vampire's controller not counted, LSJ 20100527) is fully included in the set the current action is directed at.

## Batch 25 — corruption counters, multi-Discipline cards

**25.1** Corruption counters belong to the Methuselah who put them; they stay through torpor and control changes, and survive banishment (a banished vampire keeps everything, counters of any kind included). Lost when the minion is burned or removed with its ousted controller. An ousted Methuselah's counters on others' minions stay (no longer "yours", but Aniel can burn them).

**25.2** Discipline layout decides: "[dom][pre] X" on one line is a combo — both needed (Iron Glare; KRCG `combo`); separate lines are a choice of one Discipline at one level (Line Brawl); "more than one Discipline can be used" allows any non-empty subset, each effect applying (Break the Bonds, The Platinum Protocol; LSJ 20011204-3). Outside the Hourglass: a choice with an "outferior" option.

**25.3** Revelation of the Serpent's "or" is a real choice: the bleed-only option is always available during the bleed (even when stealth is needed, for a player wanting the block); the stealth option only when stealth is needed.

**25.4** Aniel's counter burn: on a minion with several Methuselahs' counters, the controller chooses whose, at resolution; no counter left then: fizzles. The counter and card halves are one printed action for NRA.

**25.5** "You" = the acting Methuselah. Enchanting Gaze [PRE][PRO]: one use per card this action, while a minion with ≥ 1 of their counters attempts to block, stealth needed or not. Revelation's unlock needs no locked vampire: an unlocked vampire can "unlock" and a locked one "lock" to no effect — only "lock to"/"unlock to" costs cannot be paid that way; under The Sleeping Mind [DOM] it is usable with no effect (like Freak Drive).

**25.6** "If the bleed is successful" choices (after the pool burns): Break the Bonds [obf]'s lock is optional, any minion of the bled Methuselah (locked or torpid included); The Platinum Protocol [pro]'s counter is mandatory, on any minion that Methuselah controls (torpor included). With [obf] and [pre] together, printed order: lock, then blood.

## Batch 26 — actions, wave 1

**26.1** "Enter combat with a minion" from an action card: target another Methuselah's ready minion (17.3), re-checked at resolution; Fleetness [CEL]'s "locked minion" unlocked by then fizzles the action (ANK 20210124). The combat is the resolution; the action succeeds after it.

**26.2** Show of Force's "can gain +N strength" is a choice offered before range in round 1 of the block's combat, not automatic.

**26.3** Line Brawl [cel]'s "another Methuselah" is any other seat. Rewilding may target one's own location (undirected), burning one's own 2 pool. Touch of Valeren [ACTION]'s "a minion" is any minion, directed when another's.

**26.4** Shroud of Decay [OBL]: playable only with ≥ 7 cards in the prey's ash heap, re-checked at resolution (fewer: fizzles).

**26.5** "During your minion phase" means any time in it, during another action too (e.g. another vampire acting, one unlocks with Rutor's Hand to then play Veil the Legions) — including one's own actions and their combats; same for Slaughtering the Herd [DOM]'s burn. Rutor's Hand enters play locked and unlocks in its controller's unlock phase.

**26.6** Heroic Might's "strike: burn equipment": close range only (21.1), the equipment named as the strike is chosen; a dodge stops it.

**26.7** Monstrous Form: "+1 strength this round" at any combat impulse before strike resolution (as Claws of the Dead [pro]); the maneuver at the maneuver step, the press at the press step.

**26.8** Feast of the Soul's Secrets [DOM]: "younger" relative to the acting vampire; the younger target (one's own may be in torpor) is chosen when the card is played.

## Batch 27 — actions, wave 2

**27.1** Puppet Master [DOM]'s "during your next minion phase, burn this card to … take control" is mandatory: any point of that phase (LSJ 20001201), but the phase cannot end before it is done.

**27.2** Puppet Master's "a younger vampire" may be one's own (undirected, no-op). Deep Song [ANI]'s "a vampire" is another Methuselah's ready vampire (17.3).

**27.3** Stealing mid-action: a stolen blocker's attempt fails without barring it (its new controller may attempt if eligible). A stolen combatant's combat ends after the damage still pending resolves (LSJ 20020618).

**27.4** A temporarily controlled minion that cannot return (its Methuselah ousted) is removed from the game (ANK 20181110); its owner's cards on it go with it; other Methuselahs' cards on it are burned to their ash heaps. See 28.13.

**27.5** Deep Song [ANI]'s "considered the acting minion": the target takes the acting side in that combat (its Methuselah acts first each step). Effects on "this acting vampire" (Invigorate [pro], Form of Mist's continuation) stay with the vampire that took the action.

**27.6** The Gate of Acheron: payment announced as one count of random ash heap cards, the rest pool (same as unit by unit; cf. 7.10).

**27.7** Pit of Contemplation's "owner's uncontrolled region": a temporarily controlled vampire goes to the region of the Methuselah it would return to; the move is Banishment's (blood as counters, cards with it; 5.7).

## Batch 28 — masters, Childe of the Revolution

**28.1** Childe of the Revolution's new vampire enters ready, unlocked, with no blood, capacity 1 plus any Discipline card on it; it may act that turn and must hunt (even with blood; at once when that is its only option). The library is shuffled after the search regardless.

**28.2** Where a location's text does not say "you control", any Methuselah's minion qualifies: Club Illusion's Anarch burns its own blood; Anarch Railroad, Fortune Teller Shop, Warsaw Station are locked by their controller for another Methuselah's acting minion.

**28.3** An uncontrolled vampire may hold any amount of blood, over capacity (transfers and effects: Wasserschloss Anif, Powerbase: Montreal, Family Gathering); the excess drains to the bank when it is influenced out.

**28.4** When a Discipline card leaves a vampire in play (burned, player ousted), blood above the new capacity drains off at once.

**28.5** The Rack's vampire, Toreador Grand Ball's two Toreadors and Priority Contract's Assamite are chosen when the card is played (terms). Cards with an in-play effect choose when used: Chantry's blood payer, Anarch Troublemaker's two vampires (a locked one allowed), Garibaldi-Meucci Museum's two cards.

**28.6** Wasserschloss Anif's "a Tremere you control" is not limited to ready: a Tremere in torpor can move its blood.

**28.7** Out of torpor before resolution: a leave-torpor action fizzles paid (ANK 20220218, Warsaw Station); a rescue whose vampire left torpor also fizzles paid (split as normal; a vampire gone from play pays nothing, the rescuer pays what it can); a diablerie fizzles with no blood hunt.

**28.8** Week of Nightmares: a Ravnos that must hunt must take the special hunt; with no target it is frozen (takes no action; other minions act). At the end a Ravnos pays its nightmare blood if it can, burned only if it cannot. "Only one … per game" counts a play canceled as played.

**28.9** Frontal Assault's influence-phase burn: mandatory, resolved before passing the impulse but freely ordered (one may influence first); the prey's ready minions counted when it resolves.

**28.10** Piper's locked Anarch is the card's actor: it meets the ally's/retainer's requirements, pays the blood cost, its Disciplines pick the version. Piper stays a master card (Sudden Reversal can cancel it); not an action.

**28.11** A frenzy card is "played on" the minion it targets/selects/chooses/affects (LSJ 20051113); a card played on one's own side of a combat has the opponent as the frenzied one (ANK 20220409, Dam the Heart's River): Terror Frenzy is played on the opposing minion; Deep Song [ANI] on its target. Meditative Grove cancels accordingly.

**28.12** Powerbase: Los Angeles: the player spends whichever discard phase action serves best (Los Angeles's for a card requiring Anarch/baron or making a vampire Anarch, another otherwise).

**28.13** Advanced rules 6.4.1 was fixed to agree with 6.2.4: a temporarily controlled minion is removed from the game when it should return to an ousted Methuselah (ANK 20181110); only a return that can no longer occur (Parmenides) leaves it with the current controller.

## Batch 29 — combat cards, wave 2

**29.1** Martyr's Resilience ("an unlocked vampire not involved in the combat", any Methuselah's, ANK 20181101): after both combatants' prevention, every Methuselah in turn clockwise from the acting one may prevent with such a vampire, on either combatant or a retainer on one; a combatant's controller may also play it in their own prevention. Mixed-damage allocation is the Martyr's player's (4.2).

**29.2** Strike damage to a retainer waits with the combatants' damage (prevention step) and is applied with it; life stolen from a retainer moves at once.

**29.3** The preventing player chooses what they prevent (strike vs other damage, e.g. environmental). Disarm's "successfully inflicted more damage" counts strike damage landing after prevention, all the round's strike pairs.

**29.4** Flesh of Marble applies as damage lands, after every other prevention. Weather Control's before-range damage is "successfully inflicted" and so the round's first (strike damage that round is then prevented).

**29.5** Tranquility Shield vs Terror Frenzy (played on the opposing minion, 28.11): the shielded vampire's opponent cannot play either version; a shield played after cancels both versions' effects on the shielded vampire (it may maneuver to close, press to continue, use equipment; its combat cards cost no more).

**29.6** Contraband [OBF]: the player chooses the blood part as the card is played (with the equipment's name), at most half the cost; pool pays the rest (may be more than half).

**29.7** Anticipation [aus] "use a melee weapon strike": the weapon's own strike, riders included (Righteous Blade's press), at +2 damage (weapon gone by resolution: no effect). [AUS] also cancels a strike card played at the maneuver (Slam).

**29.8** Lightning Reflexes [CEL] with X = 0: playable; getting no additional strike does not consume the limited slot.

**29.9** A new round started (Telepathic Tracking) with a combatant gone in the same window (Disarm to torpor): the combat ends instead.

**29.10** Disarm's "they can burn 3 blood to burn this card": untimed — any time the vampire's controller has the impulse, in torpor too.

**29.11** Hide the Mind [obf] "a combat card requiring Auspex": any, the player's own included.

## Batch 30 — reactions and action modifiers, wave 2

**30.1** Dawn Operation [for]: the attempting vampire's controller is asked once, as the card resolves, whether to cancel the attempt; a canceled attempt is withdrawn, not failed — the vampire may attempt again.

**30.2** Forced Awakening's blood is burned as the action begins to resolve (LSJ 19990421) — successful, fizzled or failed. A block by another minion and its combat *are* the action's resolution, so the blood burns then too (once, as the block happens).

**30.3** Go-getter [OBF]: played in the after-resolution window of the blocked action (after its combat) by the acting vampire if still ready; the action returns to the block step with modifiers and declines standing, as Form of Mist's continuation (3.4).

**30.4** Dogged Pursuit [ANI]'s card goes on the vampire after resolution when it did not block, unless it already has one; its +1 intercept serves any later block attempt of that vampire, burned then.

**30.5** Melange [AUS] goes on the acting minion as the block succeeds, even if the action then continues as if unblocked; its +1 bleed is not limited.

**30.6** Beast Meld [ani][pro]: prevents 1 damage, once, at any point of the block's combat (any round).

## Batch 31 — pool actions, wave 1

**31.1** Stun (V5 Sabbat terms, 2025-06-12): lock the minion and put a stun counter on it; a minion with stun counters does not unlock as normal at the start of its controller's unlock phase, and during that phase burns all stun counters it had at the start of the turn (several stuns = one skipped unlock). Mind Numb stuns.

**31.2** Propaganda [PRE]: the *player of the card* names a ready unlocked minion of the prey as the card is played; it locks as the bleed resolves.

**31.3** Hunting the Beast [AUS]'s Salubri is chosen at the end of the combat (ANK 20240706 is Public Trust's, a different timing). "The opposing vampire is not ready" = in torpor or burned.

**31.4** Dominate Kine [DOM] stealing Living Manse: the new controller chooses the new host as the action resolves, among their **ready** minions able to hold it (Tzimisce requirement not rechecked); burned if none can (revised by 39.4, LSJ 19990928).

**31.5** Graverobbing: the stolen vampire keeps its cards and blood; burned later, it goes to its owner's ash heap. Master cards on it stay their player's (7.5); minion cards on it move with it. [DOM]'s move is offered at resolution only if the acting vampire has the 2 blood.

**31.6** A vampire burned under someone else's control (temporary included, Puppet Master [DOM]) goes to its owner's ash heap.

**31.7** Make the Misere: [pot]'s +1 strength lasts the whole combat; [cel]'s maneuver is one for the combat.

## Batch 32 — pool actions, wave 2

**32.1** Each action a card in play grants is its own action for NRA — once each per minion per turn (Open War, LSJ 20100909; Haven Uncovered's two actions).

**32.2** Army of Rats' "only 1 pool each turn with Army of Rats cards": across its controller's copies; the first resolved in the unlock phase burns, the others lapse.

**32.3** Judgment: Camarilla Segregation's burn: any Methuselah, in their own master phase, no MPA spent; a vampire in torpor qualifies.

**32.4** Tier of Souls [ANI] "+1 bleed against your prey": the card controller's prey, read live (a new prey after an oust counts). Stealing an ally's last life burns it.

**32.5** Phantasmagoria "put this card on a minion": any minion, one's own included (undirected then, 20.1).

**32.6** Embrace-like cards (Waters of Duat: "becomes a … Follower of Set", no sect printed): the new vampire takes the acting vampire's (sire's) sect — rulebook "Sect", new default.

**32.7** Card text is addressed at the controller by default: Open War's 4 pool goes to the card's controller, not the Methuselah who moved the fourth counter. Clarified by 49.5.

**32.8** Aranthebes, The Immortal: shuffled into its owner's library; its lock gives -1 stealth to the predator's acting minion; the -1 bleed applies to vampires only (not allies), while the card is unlocked.

**32.9** Fee Stake: "non-Anarch titles are worth -1 vote" = every title but baron (primogen, prince, justicar, independent title cards), minimum 0; the +1 vote goes to the Fee Stake's Anarch only when it calls the referendum.

**32.10** Khabar: Glory's "not usable if any non-mandatory actions have been performed this turn": any action of the player's minions already announced this turn (blocked or fizzled too), except a must-hunt hunt or a printed mandatory action.

**32.11** Split the Veil's ally is chosen when the last counter burns, among wraith/zombie allies in the player's ash heap; none there: the card burns anyway.

**32.12** Psychophagia's ally is chosen at declaration (whose ash heap, then which ally; an ousted Methuselah's does not count); gone by resolution: no effect.

**32.13** A vampire that must hunt may hunt with Psychophagia instead of the rulebook hunt; it ends the "must hunt".

## Batch 33 — pool masters, wave 1

**33.1** Brujah Debate: during each Methuselah's master phase, one mandatory lock per copy, ordered by that Methuselah among the phase's other mandatory effects; a copy played during a master phase locks in that same phase (owner, 2026-10-09, revising "at the start of each master phase": see Default readings); "the oldest Brujah they control" counts torpid vampires (highest capacity among all their Brujah).

**33.2** Oxford University, England: X from 1 up to the player's pool, named as it locks; paying the last pool ousts them and they cast nothing (as Ferraille, LSJ 20081105).

**33.3** Charisma "put this card on a ready vampire": any Methuselah's.

**33.4** Grooming the Protégé: the giving vampire (not locked) and X (1–3) named at play; it gives what it still has at resolution.

**33.5** Golconda: Inner Peace: capacity read at resolution (Discipline master cards included); the vampire is removed from the game, cards on it removed with it (another Methuselah's burned, as for an ousted Methuselah's minions).

**33.6** Mob Connections: usable by its controller in the press step while a minion of theirs is in combat; one press to continue.

**33.7** The Black Throne is offered as the contracted minion is about to leave the ready region — including by its controller's oust — after the contract's own "burn this card to gain 3 pool".

**33.8** The burn option (rulebook, advanced): usable in any Methuselah's unlock phase; "a minion who meets the requirements" = a *ready* controlled minion (not in torpor) of the card's clan if named, with one of its Disciplines if any (pool: Sight Beyond Sight).

**33.9** The Khabar: Community "Assamites get +1 stealth when bleeding": during bleed actions only.

## Batch 34 — pool masters, wave 2

**34.1** WMRH Talk Radio "give a minion +1 intercept": the minion attempting to block, whoever controls it, while it needs intercept; the pool is burned after resolution unless that minion blocked after the lock.

**34.2** The Rumor Mill: any ready vampire; its "burn 1 blood for +1 intercept" is offered once, while it attempts to block and needs intercept.

**34.3** Dabbler: distinct Disciplines of all cards the vampire played during the action — action card, reactions, and combat cards in the action's combat — each Discipline once whatever its level; a multi-Discipline card counts each used.

**34.4** Dead Pool's "you can add 1 counter": always added; the leaving minion must be a vampire.

**34.5** Alamut's counters = the pool actually lost (not more than the bled Methuselah had).

**34.6** Cappadocian Crypt: anyone's successful action whose card requires Hecata or Oblivion (the Discipline level played), or granted by a card requiring a Hecata.

**34.7** Twilight Camp: its blood as counters; "after you put a … vampire in play from your uncontrolled region" = when influenced out; the new sect lasts; "an Anarch" for its master phase action: any Methuselah's.

**34.8** Mithraic Cultist's intercept "against actions directed at Mithras or a Mithraic Mysteries card or member": a directed action whose target is Mithras, a member, The Mithraeum or a Mithraic Cultist.

## Batch 35 — pool masters, wave 3

**35.1** Secure Haven: "choosing is targeting" (LSJ 20031010) — a master card's vampire chosen at play (Toreador Grand Ball, Priority Contract), the vampire a master card is played for (Piper), Archon Investigation's bleeding vampire each cost 1 pool more. Other Methuselahs' actions (card actions, granted ones like Open War's rush, rescue, diablerie) cannot target the minion; a referendum's terms choosing it can (Banishment, ANK 20221019-3).

**35.2** Cave of Apples "their capacity or cost": a vampire's capacity, an ally's printed pool/blood cost; only the player's own counters count, burned all at once.

**35.3** British Museum's unlock: once each turn per minion; the minion must have successfully performed the equip action with such a card from hand.

**35.4** Yoruba Shrine: the referendum fails and polling ends at once (no votes, nothing keyed to passing); the political action is not canceled (its card is spent).

## Batch 36 — cost reductions

**36.1** A card's applicable reductions (The Line, The Shard, Ravnos Carnival's and Ravnos Cache's counters) are announced before its terms; a card unaffordable without them is offered only with them. The Line and The Shard may be locked as the action is declared or on resolution (just before the cost is paid); never by a card canceled as played (ANK 20230620); a non-action card's Shard as it resolves. Counters are spent as the cost is paid — none by a blocked or fizzled action.

**36.2** The player orders reductions as they wish (by n blood or pool, or blood-only); payment from an alternate source (counters: Carnival blood; Cache pool if the equipment costs pool, else blood) comes after.

**36.3** Ravnos Cache is unique (one copy per Methuselah); its master phase move is once per master phase (6.6); paying its last pool ousts.

**36.4** "Use a master phase action to …" (Open War, Twilight Camp) is not a "during your master phase" ability: usable as often as MPAs are spent.

**36.5** Reductions reach a card put in play by another effect (Piper, Contraband) unless the reduction's text requires the card to be played normally through an action: The Line, The Shard, Ravnos Carnival, Ravnos Cache apply (announced once named); Charisma's "recruit ally actions" does not (33.3). Cards found by a search: 38.5. Such a card is still played (41.13).

## Batch 37 — Fleshforge Chamber

**37.1** Fleshforge Chamber: a ghoul put on the Chamber from hand *is* replaced; one played from the Chamber is not (it did not leave the hand). Once each master phase per copy (6.6).

## Batch 38 — allies, equipment (slice 11aa)

**38.1** Minion Tap's "move any amount of blood": announced at play, 0 up to the vampire's blood then (an empty vampire is a legal target); what is left is taken at resolution (LSJ 20100216).

**38.2** ECTU Operative's "burn a vampire in torpor as a Ⓓ action": any torpid vampire — another's (directed) or one's own (undirected, 20.1); out of torpor by resolution: fizzles.

**38.3** Valerie Diaz's immunity ("immune to all non-aggravated damage from a strike or damaging effect"): used at the damage prevention step, for the opposing strike's normal damage or the other normal damage pending that round (environmental, retainers) taken as one damaging effect. It is an immunity, not a prevention: "cannot be prevented" does not stop it. Use count: 38.8 (supersedes the "multiple times" answer given here).

**38.4** Enhanced Coagulant: close range (21.1); a strike nullified for weapons (Blood Rage) still puts the card on the vampire. Once there it is controlled by the controller of the vampire it is on (minion cards follow the minion's controller, 7.5). "Cards requiring a Discipline" = cards *played using* a Discipline (the played mode; same for Libertas' surcharge). Its burn follows any successful hunt of the vampire.

**38.5** Reductions for searched equipment: an equipment found by a library search (Magic of the Smith, Michael Harris — the search is an equip action, TOM 19960130) or Vast Wealth's find is offered/equipped if payable with the reductions it could take; once found/chosen, the reductions (Ravnos Cache) are offered, then it is paid (a pool cost may still oust) and equipped.

**38.6** Mobile HQ: its granted action enters combat with a ready vampire of another Methuselah. "An action card that targets an Operation Antigen card you control": the action card's announced target is your Operation Antigen ally/location (employ, recruit, equip cards target nothing). **Choosing something in a referendum's terms is never targeting**: a political action card is never such an action card.

**38.7** "Only one Flaming Candle can be played or equipped in a game": equipping via Contraband (from hand outside an action), a library search or Vast Wealth counts; taking it from another minion does not (ANK 20221103).

**38.8** Valerie Diaz "once each round": one use a round, for a single strike or damaging effect in that round (supersedes 38.3's "multiple times").

## Batch 39 — political actions

**39.1** Revolutionary Council: X may be 0 (the vote does little); the acting baron may be chosen if unlocked (e.g. via CrimethInc.).

**39.2** Revolutionary Council's points go to Methuselahs (caller included), and to any location or equipment in play, any Methuselah's; a card may take more than 1 point (the extra wasted — a way not to hurt a Methuselah).

**39.3** Archon's "choose a Camarilla vampire": any Methuselah's, in torpor included.

**39.4** Disputed Territory: the location may be any Methuselah's; the chosen Methuselah may be the caller. A locquipment moved in an ambiguous fashion goes on any ready minion of the new controller able to hold it; none: burned (LSJ 19990928) — also for Dominate Kine [DOM] (31.4). A unique location with another copy in play is contested (out of play), so it cannot be chosen.

**39.5** Raising the Portcullis: +1 bleed to every vampire you control (not allies), whoever it bleeds, read live as the prey's vampires enter/leave torpor.

## Batch 40 — events

**40.1** An event takes any discard phase action, ordinary or not (an ordinary one spent first keeps Powerbase: Los Angeles's open); one event per discard phase whatever the actions left; each event once a game across the table (a second copy anywhere is never offered).

**40.2** Thirst's "did not hunt": a vampire that took a hunt action this turn hunted, blocked or not (LSJ 20050727).

**40.3** Nightmares upon Nightmares: every minion the Methuselah controls, torpid or locked included (a locked one may be "locked", ANK 20210322-2); the discard is per minion, one card each.

**40.4** Blood Weakens: the cards held are those played during the action; an action card played using a Discipline is not replaced as played either — it waits for the action's end (LSJ 20080809, Gear Up). "Until a Methuselah's hand is empty": when the next held card is played, an empty hand draws it and those held before.

**40.5** The Uncoiling's random burn: due in its controller's discard phase, mandatory (the impulse cannot pass before it resolves) but orderable against the phase's other effects by the acting player.

**40.6** Torpid Blood "actions performed by vampires in torpor": the leave torpor action and action cards played by a torpid vampire; +1 blood once.

**40.7** Wormwood: blood above the ceiling drains off as Wormwood enters play and as its counters go down (as an influenced vampire's, LSJ 20100902).

**40.8** Recalled to the Founder: torpid vampires count toward the 3 and may be the one burned.

**40.9** FBI Special Affairs Division: the 2 damage are dealt as the combat ends, before after-combat effects.

**40.10** Absimiliard's Army's ghoul is put in play (not recruited) and may act the turn it enters; the card stays its controller's (removed from the game from their library).

**40.11** Rise of the Nephtali: the Liaison action has no stealth of its own; "younger" = lower capacity (rulebook).

## Batch 41 — vampires' printed text (slice 12a)

**41.1** Abraham Mellon plays cards requiring a Tremere as one only for the cards he plays, played normally (LSJ 20050119) — not his Methuselah's master cards requiring a ready Tremere (Chantry, Arcane Library…), unlike Lady Scarlett's "you and". No minion card in the pool requires a Tremere: the clause has no tenant.

**41.2** Lady Scarlett Churchill: her Methuselah's cards requiring a Toreador count her as one to be played (Art Museum, Society Hunting Ground; Toreador Grand Ball's two Toreador, its locked one, its second one); the card's effects in play never do — as the locked Toreador she unlocks as normal (ANK 20220816-2), as the second one her non-bleed actions are not made unblockable, a hunting ground's blood for a Toreador does not reach her.

**41.3** Alexander Silverson: each vampire casting votes against a referendum his political action called burns 1 blood (himself included; one with no blood cannot vote against); a Methuselah's ballots (the Edge, a political action card's vote) cost nothing. A blood hunt called on him is not called by him.

**41.4** Sergio Bueno: an ally with his Methuselah's corruption counters pays the block toll in life, its last one included — it then burns and its attempt fails (owner correction: it may pay).

**41.5** The Guardian: his combat begins as he uses the ability in the bleed's after-resolution window, the bleeder acting first (ANK 20221102); once it ends the after-resolution window opens again for every Methuselah. "Ready unique older Nosferatu": a vampire from the crypt — a created vampire (Childe of the Revolution, Waters of Duat) is not unique (owner correction) —, older being a higher capacity, any of his Methuselah's. His directed actions are barred wherever an action is declared: bleeds, rescue or diablerie of another Methuselah's vampire, action cards and granted actions aimed at another Methuselah, their minion or their card.

**41.6** Hel-Blá: the ally is named as the action is declared, as Psychophagia's (32.12; owner correction); the action is offered only while her Methuselah's ash heap holds an ally requiring Hecata or printing Oblivion; the ally gone by resolution: no effect (blood paid).

**41.7** Luciano Carvalho: the animal retainer and the vampire it moves to are both chosen as the ability is used (owner correction) — any other vampire his Methuselah controls able to hold it, himself and torpid ones included; once each discard phase (6.6).

**41.8** Sri Sansa: one move each unlock phase, either way; "not to exceed their starting life" caps the ally only (owner correction) — blood moved to Sri above her capacity drains off as usual; no move is offered to an ally at its starting life; an ally giving its last life burns. No mage ally is in the pool.

**41.9** Roger de Camden: his maneuver is given and used as the range is determined, for any minion of his Methuselah in combat, himself included; once each turn. His cost reduction reads the card played: requiring the Hecata clan, or played using Oblivion (38.4); blood only.

**41.10** Tommaso Sforza: usable in the before-range step of a combat whose combatant on his Methuselah's side is a wraith or zombie ally, from out of the combat; once each turn.

**41.11** Gnaeus Aemilius Augustinus: a mandatory unlock-phase effect his Methuselah orders among theirs (7.8), his readiness read then; the prey chooses as it resolves among their ready minions (an ally takes the damage in life); none: nothing.

**41.12** Keegan and "Mother" Anja Giovanni: "an action requiring a Gangrel / Hecata or Oblivion" is an action card, a retainer or ally card, or a card in play's granted action requiring it (Rewilding, Dog Pack's employ, a recruit of Aggressive Corpse) — never a rulebook action. Anja may unlock an acting Hecata already unlocked (no effect, kept, 4.3).

**41.13** A card another card puts in play (a search, Contraband, Piper…) is still played, not the normal way (owner correction): cost reductions reach it — Regina Blake's blood for a location or locquipment found by a search included. Anne-Marie Bourgeois's "-1 pool" takes nothing off a blood cost; Kuyén's "animals (allies and retainers)" are ally and retainer cards whose type line makes them animals. Regina's blood is announced with the card like a reduction (36.1) and burned as the cost is paid, only if something is left to pay.

**41.14** Adelino Da Silva: "an older vampire" is one of higher capacity than his.

## Batch 42 — the last five vampires' text (slice 12b)

**42.1** Daria: "directed actions cost her +1 blood" is a cost of the action itself, not of a card: a plain directed bleed costs her 1 blood, a directed action card its own cost plus 1. It follows the action-cost timing: payable, with the action's own blood cost, as the action is announced (not offered otherwise); paid as it resolves — a blocked action pays nothing. Her undirected actions are unaffected.

**42.2** Djeneba: "strike cards" are cards played in a strike mode, a dodge included; a combat card played otherwise (maneuver, press, additional strike, strength) is untouched, and so are her own. The minion in combat with her pays, whether she acts or blocks; an ally pays in life, its last one too (41.4): it burns and the combat ends (ANK 20180928-1). A strike card its player cannot pay for is not offered.

**42.3** Marchesa Liliana: the "during" template ("during a bleed action, … can …") is once per action.

**42.4** Mithras: "younger vampires" are vampires of lower capacity, read live — never an ally, never Mithras himself; a Methuselah's ballots (the Edge, a political action card's vote) cost nothing, and a blood hunt called on him is not called by him (41.3). The block toll applies from torpor too (his leave-torpor action).

**42.5** Ragnar Nordstrom: "cards requiring Protean" are cards played using Protean (38.4); the discount applies wherever he pays the card's blood, combat cards included.

**42.6** Daria's +1 at resolution is paid first, before the action's own cost: unpayable by then (blood spent during the action), the action fizzles (3.6); paid, then the action card's own cost unpayable, it fizzles with the +1 paid (a fizzled action's cost is paid, ANK 20220218).

**42.7** Marchesa Liliana: her additional +1 bleed is not the rulebook's limited bleed bonus (a "(limited)" card still adds to it). The seven cards are her Methuselah's picks, by name (copies alike), one at a time as she uses the ability. "Cards in your ash heap" are all its cards, burned vampires included (owner correction) — for Shroud of Decay's seven too.

**42.8** A forced block attempt ("… attempts to block": Eagle's Sight, Second Tradition, Organized Resistance, Eyes of the Wild, Dogged Pursuit, Sense the Savage Way, One With the Land) by a minion owing a block toll (Sergio Bueno, Mithras): offered only while the toll is payable; if the card's own blood, paid first, leaves it unpayable, no attempt is made (the card's other effects stand).

**42.9** Bone Shambler: both its abilities are "during" abilities (42.3), each usable once per action per copy — the lock of another copy and the Hecata's blood, both in the same bleed, neither twice; another copy bleeding in a later action uses its own again.

## Batch 43 — found by training (P4)

**43.1** A political action whose referendum terms have no legal choice when they are chosen (after the action succeeds, 3.5) — Parity Shift's Methuselah with more pool than the caller, richer at play but no longer (Tension in the Ranks burning their pool as their ally burns during the action): the action fizzles (3.6) — successful, no referendum held, no effect; a rider keyed to the referendum failing or being canceled (Cedrick Calhoun) does not apply. The play itself is masked when the terms cannot be met (LSJ 20100129). (Owner confirmed, 2026-09-28.)

## Batch 44 — legacy cards for tournament decks (slice 16a)

Owner confirmed 44.1–44.10 (2026-09-28).

**44.1** Hell-for-Leather [obt] ("only usable if this Anarch is ready and the opposing minion is not"): played in the end-of-round window, the round's damage applied — the only point a combat leaves the opposing minion not ready (sent to torpor or burned) with combat cards still playable; once per combat, per Anarch.

**44.2** Ennoia's Theater: the hand-size lock is untimed (any impulse, 7.7), on any Methuselah's turn, for "this turn"; one lock a turn for either use.

**44.3** Saatet-ta: the stealth only to the acting Follower of Set (Ministry) of her controller, while stealth is needed (3.2); the intercept only to her controller's Follower attempting to block, while needed; the +1 bleed not limited.

**44.4** Jake Washington: "a ready vampire with no blood" is any Methuselah's; the amount (1–4) is chosen as he is burned; the ally a master card becomes is not recruited (it may act the turn it enters play).

**44.5** Gilbert Duane: the vampire made Anarch is chosen as the ability resolves, among the non-titled vampires his Methuselah controls, in torpor included; once per influence phase (6.6).

**44.6** Carlton Van Wyk: "+1 intercept against vampires" reads the acting minion (a vampire); his burn may target any vampire that committed diablerie since his controller's last turn ended, his controller's own or one in torpor included.

**44.7** Powerbase: Savannah: the location locked is another unique location its controller controls, unlocked; usable in the before-range step of any round of a combat whose acting side is its controller's vampire. "Elysium" has no effect in the pool.

**44.8** Rötschreck is played once both strikes of a pair are chosen (before they resolve), additional strike pairs included; "an opposing minion attempts to inflict aggravated damage" reads the chosen strike (aggravated at its source, effective at the range, not aimed at a retainer).

**44.9** Donnybrook [obf]: the equipment or retainer is the opposing minion's, named with the strike; resolved with first strike, it burns before the opposing strike resolves (a weapon so burned strikes for nothing), unless dodged.

**44.10** Ashur Tablets: "only one … each turn" counts any Methuselah's play (a canceled one included); the third copy is checked as a copy enters play. The card is not unique: copies never contest, every copy in play counts (owner correction).


## Batch 45 — the decision point before an oust (slice 16b)

Owner confirmed 45.1–45.8 (2026-09-29); it replaces 44.11 (Life Boon blocked), removed.

**45.1** When a Methuselah's pool reaches 0 or below, the oust waits while a Methuselah holding Life Boon may save them: each holder in turn, clockwise from the active Methuselah, never the active one (out-of-turn), nor one ousted by the same effect; one pass over the holders; those saved leave the batch and the rest are ousted together, as one simultaneous oust. A Methuselah may save any Methuselah of the batch.

**45.2** Life Boon's pool is chosen as it is played (its terms): at least what keeps the Methuselah in the game — what they owe below 0, plus 1 — and at most the Boon player's pool less 1: giving pool never ousts the Boon player.

**45.3** The pool moves from the Boon player to the Methuselah saved as the Boon resolves (a gift, not a cost: no reduction applies); canceled as played, the out-of-turn master phase action is spent (3.3) and the next holder may still save them.

**45.4** While the oust waits, the effect that ran the pool out goes on as if the Methuselah were still in the game (the rest of a referendum's effects, the action it is part of); if nobody saves them, the oust applies then — on their own turn, the turn passes then; a combatant's Methuselah ousted ends the combat at once.

**45.5** The Boon player gains every victory point the Methuselah saved gains — a prey's oust, being the last one standing, the half point at the time limit — the oldest Boon first, each until it has given 1 VP (then burned); "until you are being ousted": the Boon player gains nothing from the oust that takes them out — neither the VP for ousting them nor the last-one-standing VP, both the saved Methuselah's (owner).

**45.6** "During each of their unlock phases, you can steal 1 pool from that Methuselah": once per unlock phase of theirs, any time among their unlock-phase effects; their last pool so stolen ousts them (and opens the decision point again).

**45.7** A Methuselah saved by a Boon still scores for a prey ousted by the same effect (the VP to the Boon player, LSJ 20021029) and gains the 6 pool.

**45.8** The pool a Methuselah is below 0 by is kept until the oust applies (bleeds, simultaneous pool losses): what they owe is what a Boon must make good (RTR 19970630).


## Batch 46 — found by the owner's first game (slice 17d)

Owner ruled 46.1–46.4 from play (2026-09-30).

**46.1** A minion cannot play the same action modifier, nor the same reaction, more than once during an action: by card name, whatever the Discipline level, a canceled play counted (rulebook "Action Modifier", "Reaction"; [ANK 20190104] [LSJ 19980212]; advanced rules §1.8.3). During the referendum of a political action too. Combat cards keep their own limits (per round, per combat).

**46.2** The Methuselah whose political action card called the referendum has that card's vote as their one political-action-card vote (5.4): they cannot burn another political action card for a vote. The burn is the other Methuselahs', and anyone's in a referendum with no calling card (a blood hunt, 5.2; a referendum a card in play grants, 6.2).

**46.3** "You can lock this card" on a location in play is its controller's to use (Power Structure, Elysium: The Palace of Versailles, Ventrue Headquarters): no other Methuselah locks it, in any referendum.

**46.4** Pass Through Shadow [OBL] ("after combat ends, put this card on this vampire"): the card itself goes on the vampire after combat — out of the ash heap it went to as it resolved — and burning it for +1 stealth sends it back there (owner, 2026-09-30; revises 21.1 for this card). Touch of Oblivion and Sculpt the Flesh keep 21.1 until the owner says otherwise.

## Batch 47 — found by the owner's second game (slice 17e)

**47.1** "Put this card on …" puts the card itself in play, out of the ash heap it went to as it resolved (owner, 2026-10-01, from play: Touch of Oblivion [obl]; generalizes 46.4, revises 21.1). **General rule: it applies to every card, present and future, whose text puts the card itself on a minion or in play — never an effect-only stand-in.** Today: Touch of Oblivion [obl] and Sculpt the Flesh (on the opposing minion, if the strike lands), Disarm (on the opposing vampire), Pass Through Shadow [OBL] (after combat), Dogged Pursuit [ANI] (if the vampire does not block), Melange [AUS] (on the acting minion, if the vampire blocks). Burned — counters run out, a cost paid with it — it goes back to the ash heap. Its effects are unchanged.

**47.2** An additional strike card (Quickness, Blur, Side Strike [CEL], Pursuit [CEL], Lightning Reflexes [CEL], Arms of Ahriman [obl]…) is played only after the first strike exchange of the round, both combatants still ready — never before strikes nor before the first pair resolves (owner, 2026-10-02, from play: Arms of Ahriman offered before strikes; **overrides 4.4**, rulebook "Additional Strikes": "announced and performed only after the first pair of strikes is completed"). The general rule for every card and effect granting additional strikes, future ones included; Chun Hei's ability likewise.

## Batch 48 — found by the owner's play (2026-10-03)

**48.1** Stealth and intercept are not floored at 0 (owner, 2026-10-03, from play: Form of the Bat [pro] against a 0-intercept blocker of a 0-stealth action blocked). The -1 leaves the blocker at -1 intercept, below the action's 0 stealth: the attempt fails unless intercept is gained back (it is now needed, 3.2). Stealth likewise may be negative (LSJ 20021211: Draba, Night Terrors "can be played on a minion with zero or negative stealth"); "reduce stealth to 0" leaves a negative stealth unchanged. The table shows the signed values.

**48.2** Additional strike cards are played one at a time: a combatant with an additional strike still to make gains no other until that pair resolves; after each additional pair, a new chance to play one (owner, 2026-10-03, from play: Quickness offered right after Pursuit [CEL]; "you first have to resolve the strikes pair from the Pursuit before you get the opportunity to get yet another additional strike by playing Quickness"). Refines 47.2; the opponent may still answer in the same window (rulebook "Additional Strikes": the acting minion decides first, then the opposing one). Question (implemented reading): a card granting several strikes (Blur) bars further cards until all of them are made.

**48.3** A change of blocker: during its own minion's block attempt, while that attempt is short of the action's stealth, a Methuselah may attempt with another of its minions instead — by declaring that one's attempt, or with a card whose unlocked minion attempts to block (Organized Resistance's second mode: "that Anarch attempts to block", so it must be the one blocking). The attempt under way fails then, with a failed attempt's consequences (owner, 2026-10-03, from play: an attempt left short by the acting minion's stealth could not be changed). Refines the "no re-declare" note (the attempt persists until closure unless its Methuselah changes blocker); a forced attempt on the minion already attempting keeps it (ANK 20181122-2). Question (implemented reading): the switch takes effect at once, with no impulse left to others for the abandoned attempt.

**48.4** Slam [POT] ("as above, with 1 optional maneuver") is played only at the maneuver, never as the strike: at strike declaration it would be [pot] for 1 blood, its maneuver unusable (owner, 2026-10-04, from play: "superior gives a maneuver so it's only playable during the maneuver step"). The only card of the shape — a mode identical to another but for an optional maneuver; Aid from Bats and Diversion carry their maneuver on their only strike mode and stay playable as the strike (2.5).

**48.5** An attempt to block is offered whenever the would-be blocker's Methuselah holds an intercept card it could play and has a library to draw from: the card's replacement may be another (owner, 2026-10-04, from play: an action at 2 stealth, one intercept card in hand, "hope to draw the second"). Refines the reachability pruning (DESIGN §4.3): it counts the hand alone only when nothing can be drawn. Superseded by 73.8 for the engine (every legal attempt offered); the host's opportunities filter keeps the reading.

**48.6** A minion whose block attempt failed is not offered the same attempt again while nothing has changed: its Methuselah's hand, the stealth, its intercept, blood and lock, the cards in play, every hand's size (owner, 2026-10-04, from play: a bot attempting again and again with not enough intercept — "legal … but it still probably should not retry if the situation has not changed"). An engine reading, not a rule: the repeat is legal and would fail alike. Another minion's attempt, or the same minion's once something changed, is offered as before. An attempt left for another minion's (48.3) failed alike: swapping back is not offered either (owner, 2026-10-04, from play: two bot minions trading a failing attempt back and forth).

**48.7** An ability whose only effect is to unlock its own minion is not offered while that minion is unlocked (owner, 2026-10-04, from play: Rutor's Hand offered to unlock an unlocked vampire — "legal, but really useless, no card cycled or anything"). An exception to 17.4's legal no-op plays: locking the card or spending a once-each-turn use buys nothing. Rutor's Hand, Valerie Diaz, Sword of the Archangel; the vampires' unlock abilities required it already.

**48.8** Rescue's cost: who pays it (the acting vampire, the rescued one, or a split) is chosen as the action is announced; the blood is paid as it resolves (3.6) (owner, 2026-10-04, from play: "it should ask when the action is declared (who pays)"). The cost is read again at resolution (live conditions). Question (implemented reading): a share its payer can no longer meet at resolution falls to the other, the action fizzling only when both together cannot pay (3.6); a vampire gone from play pays nothing (28.7).

## Batch 49 — Reign of Stanislava (25th Anniversary) and Sabbat paths (scoping, 2026-10-05)

**49.1** Reins of Power, Ancient Influence: the Methuselahs choose their ready vampire in turn, clockwise from the acting Methuselah, in the open — each sees the earlier choices. The choice is optional: with none, Ancient Influence gains 0 (the 5 pool is still burned); for Reins of Power, a predator choosing none makes their prey burn 0 (owner, 2026-10-06).

**49.2** Hartmut Stover's burn ("once results are tallied") is not a cost to cast (unlike Alexander Silverson, 41.3): a vampire with no blood may vote against his referendum and burns nothing. As in 41.3, Hartmut burns if he votes against his own referendum, and a Methuselah's ballots (the Edge, a burned card) cost nothing (owner, 2026-10-06).

**49.3** Stanislava: retainers employed by a minion opposing her in combat lose their card text for that combat (strength or damage bonuses, additional strikes, prevention, intercept); they are still retainers, whose life still takes damage (owner, 2026-10-06).

**49.4** Fear of Mekhet: the 5 blood is all or nothing — a vampire with fewer than 5 is burned without paying any; a vampire in torpor still pays. Moving the card does not lock its new holder, which may be another justicar or Inner Circle member of the same Methuselah (torpid included, ANK 20210109) (owner, 2026-10-06).

**49.5** Monastery of Shadows' +1 stealth goes to any vampire with capacity 8 or more, another Methuselah's included. Clarifies 32.7: "you" in card text is the card's controller; designating a specific player needs that addressing — an unaddressed "a vampire" is any vampire (owner, 2026-10-06).

**49.6** Enkil Cog's action ("during any Methuselah's minion phase") is offered only when the active Methuselah declares the end of their minion phase. If every holder passes, the minion phase ends. If one locks the Cog, that vampire takes its action, then the impulse goes back to the active Methuselah, whose minion phase continues: they may take more actions before declaring its end again, which offers the Cog again (owner, 2026-10-06: "tricky to get right for both players"). Question (implemented reading): with several holders, the offer goes clockwise from the active Methuselah.

**49.7** A library card with a Sabbat path icon requires a vampire following that path: for a minion card, the minion playing it; for a master card, a ready vampire of that path its Methuselah controls — as a clan icon does (owner, 2026-10-05). Path is read live (Black Chantry, 2025-06-12: one path at a time; benefits only while Sabbat; an influence-phase "burn 1 blood to follow a path" for a Sabbat vampire following none).

## Batch 50 — Reign of Stanislava, implemented readings (2026-10-06)

Readings no ruling settles, implemented with the slice; answered by the owner on 2026-10-08.

**50.1** Enkil Cog in its own Methuselah's minion phase: not offered — the vampire takes its actions as usual there, and locking the Cog would add nothing. The Cog is offered only in another Methuselah's phase rotation (49.6). The several-holders reading of 49.6 cannot arise: the Cog is unique, so at most one is in play uncontested. Confirmed (owner, 2026-10-08).

**50.2** Enkil Cog is offered only while its vampire has an action to take (ready, unlocked, some action legal for it): locking it with none would only lose the effect. Confirmed (owner, 2026-10-08).

**50.3** Enkil Cog's "a successful bleed against your prey" reads the bleed's target as the card is played, after resolution: a bleed redirected to another Methuselah (Deflection) is not against your prey. Confirmed (owner, 2026-10-08).

**50.4** Monastery of Shadows' lock is used holding the impulse in the stealth and intercept exchange of a block attempt, as KRCG News Radio's: when stealth is needed (no "even if stealth is not yet needed"), by any Methuselah controlling it (49.5). Confirmed (owner, 2026-10-08).

**50.5** Political Stranglehold counts every vampire with capacity 8 or more its Methuselah controls, in torpor too; Ancient Influence and Reins of Power choose a ready vampire, locked or not (ready is not unlocked). The chosen capacity is read as the effect applies. Confirmed (owner, 2026-10-08).

**50.6** Permanent Vacation: the ally removed from the game leaves its card out of the game (not in the ash heap); the cards on it are burned, and nothing keyed to a burn fires. Confirmed (owner, 2026-10-08).

## Batch 51 — The Endless Dance, implemented readings (2026-10-06)

Readings no ruling settles, implemented with the slice; answered by the owner on 2026-10-07 and 2026-10-08.

**51.1** Marcos Belegrad: each Methuselah casting votes or ballots against his referendum burns 1 pool once, however many of their sources (vampires, the Edge, a burned card, Business Pressure's pool) voted against; all at once at the tally, before the referendum's effects. Confirmed (owner, 2026-10-08).

**51.2** Camarilla Conclave is unique and worded as a "during" ability: it is usable once each political action — the +1 stealth (when needed) or the -1 vote to each non-Camarilla vampire, which reaches votes already cast (a vampire's votes stop at 0, ANK 20251125). A political action is enough, no political action card needed (Anarch Revolt's "call a referendum to burn this card"); a referendum of no political action (a diablerie's blood hunt) is not one (owner, 2026-10-07).

**51.3** Confiscation: the chosen vampire takes the equipment only if it can hold it ("a minion can have only one …"); otherwise nothing happens. Confirmed (owner, 2026-10-08).

**51.4** There is an impulse between two actions: the active Methuselah holds it first, with priority to start the next action; passing it declares the end of their minion phase, and the impulse goes round the other Methuselahs, who may use effects (The Louvre, Paris: ANK 20210608) but not start an action — except Enkil Cog's (49.6). An effect used there returns the impulse to the active Methuselah, whose phase goes on (owner, 2026-10-07). The engine's minion-phase rotation (rules-qa 7.9) is that impulse.

**51.5** Business Pressure resolves as each Methuselah, in impulse order from its player, chooses once how much pool to burn — so how many votes they get — and on which side, then passes to the next; once everyone has chosen, the card resolves: the votes counted and the pool burned at once, the last pool ousting. It is not paid vote by vote (owner, 2026-10-07). [PRE]'s +3 votes a pool are its player's only.

**51.6** Reinforcements canceled by Delaying Tactics: removed from the game, not returned to its owner's hand ("remove this card from the game, even if the action or the referendum fails or is canceled"). Confirmed (owner, 2026-10-08).

**51.7** Loup: +1 stealth to every Toreador, any Methuselah's; burned after any Toreador's successful action (the one it was played in included), else at the end of the turn; playable when stealth is not yet needed (LSJ 20011214-2). Confirmed (owner, 2026-10-08).

**51.8** Diana Iadanza cancels any reaction card as it is played during the referendum of a political action she performs (she calls it); its cost is paid. Confirmed (owner, 2026-10-08).

## Batch 52 — Sabbat V5, the Path of Power and the Inner Voice, implemented readings (2026-10-07)

Readings no ruling settles, implemented with the slice; answered by the owner on 2026-10-08.

**52.1** "[pot] or [pre]" on one line (the Sabbat V5 layout) is either Discipline at that level, one mode — the counterpart of 25.2's side-by-side combo, which needs both (Iron Glare, Absolute Tyranny): Dominant Personality, Magnetic Authority, Relentlessness and Unthinkable Humiliation are played with either. Confirmed (owner, 2026-10-08).

**52.2** Absolute Tyranny [POT][PRE]: the -1 vote to each vampire not following the Path of Power and the Inner Voice reaches votes already cast, each vampire's votes stopping at 0 (ANK 20251125), as Camarilla Conclave's (51.2); played by the acting vampire as an action modifier, by another as a reaction. Confirmed (owner, 2026-10-08).

**52.3** Día de los Muertos: "the first referendum a Sabbat vampire you control calls on this turn" is the first one called after it is played this turn; it passes with no polling step, by a margin of 0 (LSJ 19980107); "only one … in a game" counts every Methuselah's. Confirmed (owner, 2026-10-08).

**52.4** Věnceslava: her prey "burned 1 or more pool" counts any pool her controller's prey loses during her action, costs paid included (a bleed, a referendum's burn); a bleed deflected to another Methuselah is not the prey's. Confirmed (owner, 2026-10-08).

**52.5** Cold War: "if this acting vampire is a cardinal or regent, you can choose both" reads the title as the terms are set. Confirmed (owner, 2026-10-08).

**52.6** Regent's "Sabbat vampires can enter combat with this vampire as a Ⓓ action": another Methuselah's Sabbat vampire, not its own controller's (17.3). Confirmed (owner, 2026-10-08).

**52.7** Powerbase: Madrid locked in a polling step gives its votes to any titled Sabbat vampire, any Methuselah's — that vampire's controller casts them. Confirmed (owner, 2026-10-08).

**52.8** Under Siege: offered in the block window to each ready Sabbat vampire of its controller that could attempt to block the action, locked or not, the one already attempting included (ANK 20181122-2); once each action; the +1 intercept lasts the action. Confirmed (owner, 2026-10-08).

**52.9** The prisci block: each ready priscus casts one ballot, even with no vote of its own; the side with more ballots gets the block's 3 votes, a tie gives none. Confirmed (owner, 2026-10-08).

**52.10** War of Ages' "call a referendum to burn this card as a +1 stealth political action that costs 1 pool": the pool is paid as the action resolves, as an action card's cost. Confirmed (owner, 2026-10-08).

## Batch 53 — Sabbat V5, the Path of Cathari, implemented readings (2026-10-07)

Readings no ruling settles, implemented with the slice; answered by the owner on 2026-10-08.

**53.1** Aelswith and Terrifying Visage: the pool the blocker's controller burns as the block is declared may oust them; their minion leaves play with them, and the action is blocked with no combat. Confirmed (owner, 2026-10-08).

**53.2** Seeds of Terror's "minions must burn 1 blood to attempt to block" is blood only: an ally, which has life, cannot attempt to block that action (Unthinkable Humiliation prints "blood or life"). Confirmed (owner, 2026-10-08).

**53.3** Hedonism [for]/[pre]: a combat is queued (Sabbat V5 rules, "Queue a combat") — it starts after resolving the action, still as part of its resolution: the action blocked, after block resolution (the combat of the block, if any); the action successful, after its effects have taken place. Combats always occur during the action's resolution, and only one can be queued. It is lost if the action ends first (LSJ 20090826) or if either vampire is not ready then. Hedonism's vampire is its acting side (strikes first) (owner, 2026-10-08). A failed or fizzled action: 56.1.

**53.4** Fotini Katsikaris: "+1 hand size until your next discard phase" ends as that discard phase begins; the cards then over hand size are discarded at once. Confirmed (owner, 2026-10-08).

**53.5** Fever Pitch [FOR]/[PRE]: "you can put this card on this vampire" is an option, left open whenever the bleed is successful and the vampire has no Fever Pitch yet — its player's choice (`Interrupt::MayPutOn`), though never worse than the ash heap: an agent may take it unasked (the scripted bots do) (owner, 2026-10-08).

**53.6** Forward Momentum: a blocked bleed is a bleed that is not successful — a counter is burned. "After you perform an action" and "after the bleed is unsuccessful" are the same window, the controller choosing the order: they can burn two counters to unlock before the blocked bleed's counter is burned, or take a successful bleed's counter first. The engine holds the bleed's counter pending in the after-resolution window: the unlock takes a pending gain first, a pending loss comes after it, the rest applies as the action ends (owner, 2026-10-08).

**53.7** Terrifying Visage goes on any ready vampire following the Path of Cathari, another Methuselah's too ("a ready vampire", not "you control"); its path icon needs a ready Cathari vampire of its player's (49.7). Confirmed (owner, 2026-10-08).

**53.8** The Hungry Coyote's +1 hunt: a hunt taking blood from a target takes 1 more from it (RTR 20030519); Psychophagia, which eats an ally from the ash heap, gets 1 more from the bank. Osvaldo Kühnemann's "a Methuselah controlling a Toreador or Ventrue" counts one in torpor (owner, 2026-10-08).

## Batch 54 — Sabbat V5, the Path of Caine, implemented readings (2026-10-07)

Readings no ruling settles, implemented with the slice; answered by the owner on 2026-10-08.

**54.1** Yawp Court is used in the political action's would-be-successful window (its blocks declined, before it resolves); the action card's cost is paid first (LSJ 20080201), then the location and the Sabbat vampire lock and the combat begins, the acting vampire acting first. "Still ready at the end of combat" is read as the combat ends — before a Rötschreck's torpor (LSJ 20080821), not after a torpor left at once (LSJ 20090325-1); the action ending there is a failed action, with no after-resolution window. The would-be-successful window now exists for every political action (The Line and Ravnos Cache can be used there). Confirmed (owner, 2026-10-08).

**54.2** Augury of Doom's "if your prey controls no vampires in torpor … burn this card" is read in its controller's prey's unlock phase, instead of the burn; played while the prey has none, it waits for that phase. Confirmed (owner, 2026-10-08).

**54.3** Pillars Fall's "you can put this card on this acting vampire" is applied whenever the opposing vampire is not ready at the end of that combat and the acting vampire has none; the card enters play from the action. Confirmed (owner, 2026-10-08).

**54.4** Hunting the Quarry: the card is burned as the granted action is declared — not a cost, an action's costs being paid at resolution; its target the minion it was on — gone from play, the action fizzles (owner, 2026-10-08).

**54.5** Fiendish Tongue: "during your next discard phase" is this turn's, an unlock offered there (the vampire still locked, with the blood); the vampire's clan is read twice: as the bleed succeeds, and again in the discard phase — only a Tzimisce can use the unlock then (owner, 2026-10-08).

**54.6** Dance with the Devil [OBF]/[THA]: the extra maneuver is only usable at long range, after the card's own, its blood paid as it is used. Confirmed (owner, 2026-10-08).

**54.7** Abuse of Power: "if only one combatant is ready, the controller of the opposing minion burns 1 pool" — the opponent of the ready one: the acting minion not ready, its own controller pays (owner, 2026-10-08).

**54.8** Rage of Apedemak's +1 hand size lasts the combat: the cards then over hand size are discarded at once. Depravity bars recruit and employ action cards played by its vampire, nothing else. Confirmed (owner, 2026-10-08).

## Batch 55 — Sabbat V5, the Path of Death and the Soul, implemented readings (2026-10-07)

Readings no ruling settles, implemented with the slice; answered by the owner on 2026-10-08.

**55.1** Target Vitals: the aim's +2 damage is added once the strike's damage is successfully inflicted (after the prevention step, so it cannot be prevented), of the strike's kind (RTR 19970630); "the opposing minion cannot press this round" applies only when the aim lands. Confirmed (owner, 2026-10-08).

**55.2** Morbidity: the card takes as much of the opposing vampire's blood as it has, up to 2; the blood goes back as the combat ends (after every round, a press included), then the card burns. Confirmed (owner, 2026-10-08).

**55.3** Eulogio Sánchez de los Reyes: the look-and-reorder and the unlock both happen at the action's resolution, in that order; the 1 blood is the action's cost, paid at resolution as for every action, not at declaration — not paid if the action is blocked (owner, 2026-10-08).

**55.4** Heartrender: "remove itself from the game" is paid as the action succeeds, with the target burned; blocked, it stays in play. Confirmed (owner, 2026-10-08).

**55.5** Gifts From Hereafter [DOM]/[OBL]: "after action resolution" opens whether the bleed succeeded or not (a fizzle included), not after a block; the vampire's 1 blood is paid then, as the ally unlocks. Confirmed (owner, 2026-10-08).

**55.6** Cursed Abattoir: "after a zombie you control is burned, you can add 1 counter" whenever one of its controller's zombie allies burns (in or out of combat, from damage or a card) — optional, its controller's choice (`Interrupt::MayAddCounter`), never worse to add: an agent may take it unasked (the scripted bots do). The pool has no zombie retainer: a burned retainer is not counted yet (owner, 2026-10-08).

**55.7** Obedience: "about to enter combat with an acting younger vampire" is the block — played as the block succeeds, before combat — and also a combat the action itself starts (a rush, Deep Song: ANK 20200702); there the vampire must be unlocked or woken, Obedience being a reaction. The action ends unsuccessfully, no combat (owner, 2026-10-08). When it is played against such a combat: 56.2.

**55.8** Target Vitals under Immortal Grapple: it can be played on a hand strike. LSJ 20080409 is about a committed strike (a .44 Magnum strike committed by using its maneuver): Target Vitals is played in the strike phase even then, while that strike can still be used; a grapple forbidding it (another maneuver having been used to get close), the strike is committed but not usable, and Target Vitals cannot be used on it (owner, 2026-10-08).

## Batch 56 — Owner answers to Batches 50–55, implemented readings (2026-10-08)

Readings no ruling settles, implemented with the answers; answered by the owner on 2026-10-08.

**56.1** Hedonism's queued combat when the action, unblocked, does not succeed: a fizzled action (successful, with no effect: its target gone, its cost unpayable) still has it, after resolution; a failed action (Archon Investigation, Spying Mission [OBF] failing it at resolution; Obedience ending it) loses it, as an action ending early (LSJ 20090826). Confirmed (owner, 2026-10-08).

**56.2** Obedience against a combat the action itself begins (55.7): played in the reaction window once blocks are declined, before the action resolves, by the vampire the action is to enter combat with; there the action ends, failed, at resolution (NRA holds, LSJ 20070709). Not earlier in the action, nor against a combat Hedonism queues, a reacting minion's (The Guardian) or the after-resolution window's. Confirmed (owner, 2026-10-08).

## Batch 57 — Legacy cards for TWDA decks (staples, steps 1–3), implemented readings (2026-10-08)

Readings no ruling settles, implemented with slice 16j; questions for the owner.

**57.1** Alonso Petrodon: "each anarch controlled by the target of the bleed" is every Anarch vampire of the bled Methuselah, those in torpor included (an ally is not an anarch); the damage is inflicted as the bleed resolves, with the pool loss, before after-resolution effects. Confirmed (owner, 2026-10-09).

**57.2** Maila: "a minion you control" is any minion acting for her Methuselah, Maila herself included; the +1 stealth is offered only when stealth is needed (3.2), once each of her Methuselah's turns (ANK 20230122). Confirmed (owner, 2026-10-09).

**57.3** Gabriel Tremblay: his discard for +1 strength is untimed within the combat (before range, at the strike…), once each combat; the strength counts for every round left. Confirmed (owner, 2026-10-09).

**57.4** Lam Into: each Discipline is the hand strike or a melee weapon of the vampire's at that bonus; [for] played at the maneuver commits the round's strike (the weapon's, if chosen), not usable while the strike could not be; "do not replace until after combat" is replaced as the combat ends, before the after-combat window. [vic] is playable since slice 16m (Lord Tremere, Malgorzata and Mistress Fanchon print Vicissitude; one made Anarch may play it). Confirmed (owner, 2026-10-09).

**57.5** Dragonbound: a mandatory effect of each Methuselah's discard phase, ordered among that phase's other mandatory effects (as The Uncoiling, 40.5); played during a discard phase, it applies in that same phase; vampires in torpor are counted when the burn resolves. (Owner, 2026-10-09, correcting the first reading: a mandatory "during X phase" effect applies in the phase its card is played in — see Default readings.)

**57.6** Scourge of the Enochians: "a vampire with capacity 2 or less" is any vampire in play, any Methuselah's, in torpor included; played in its controller's discard phase, it can be used then, and if not, the predator takes it as that phase ends (as The Coven). Confirmed (owner, 2026-10-09).

**57.7** Heidelberg Castle: each use moves from one vampire to the other (blood, then any of the equipment and retainers it could have), not both ways; the blood amount is any, the excess to the bank (ANK 20170804). Confirmed (owner, 2026-10-09).

**57.8** Lilith's Blessing: banned (since 2013), so not in the pool at all; removed (owner, 2026-10-09).

**57.9** Anarch Convert: his ability applies as he is moved out of the uncontrolled region (the pool's only way in); it is not offered with no other non-titled vampire to make Anarch; gaining 1 pool or drawing from the crypt is chosen after the vampire is chosen. Moved out beside a copy in play, he must use it (no voluntary self-contest). Confirmed (owner, 2026-10-09).

**57.10** Mylan Horseed: his unlock can target a vampire already unlocked (a legal no-op play, 17.4); "cannot block vampires" leaves his blocks of allies. Confirmed (owner, 2026-10-09).

## Batch 58 — Legacy cards for TWDA decks (steps 4–9), implemented readings (2026-10-09)

Readings no ruling settles, implemented with slice 16k; questions for the owner.

**58.1** Become Anarch (advanced rules): added as a rulebook action — a ready untitled non-Anarch vampire, +1 stealth, undirected; offered only if its cost is payable as announced, the cost paid at resolution (2 blood, 1 if its Methuselah controls another ready Anarch, read then). Brunhilde's "costs vampires you control 0 blood" reaches her Methuselah's other vampires, read at resolution, in torpor too (a printed passive), not while contested. Confirmed (owner, 2026-10-09).

**58.2** First strike (Elimelech, "strikes with first strike"): his strike resolves alone; the opposing strike waits for its damage to be prevented and applied, and is lost if its minion is no longer ready (burned, in torpor) or the round is over; a dodge opposing it still resolves at once (it protects); both with first strike resolve together. Confirmed (owner, 2026-10-09).

**58.3** "n votes (titled)" (Arishat, Scout Youngwood) is a title of its own, not unique and never contested; it can be removed (Patsy) like any title. Confirmed (owner, 2026-10-09).

**58.4** Infernal (Arishat): the 1-pool unlock is offered in its controller's unlock phase whenever it is locked there (again if something locks it again that phase), ordered among that phase's effects; paying the last pool ousts. Confirmed (owner, 2026-10-09).

**58.5** Valkyrie: "unless /Brunhilde/ is ready" — any ready Brunhilde in play, any Methuselah's; "Non-unique." — copies neither contest nor self-contest (several may be influenced). Confirmed (owner, 2026-10-09).

**58.6** Reginald Moore: "take actions to enter combat with them" bars his action cards entering combat with a Tremere (or Tremere antitribu), read live on the target's clan; combats he does not choose (blocked by one, rushed) are not barred. Confirmed (owner, 2026-10-09).

**58.7** The Medic: "during your turn" — once each of his Methuselah's turns (the "during" template, 6.6, 15.2); any Anarch they control, in torpor too, its blood capped at capacity. (Owner, 2026-10-09, correcting "as often as he has the Edge".)

**58.8** Arishat: "a vampire" is any vampire, any Methuselah's, herself included, one yet to vote as well (it then casts none); Danielle Diron's vote ability is offered in torpor too (a printed ability without "if ready"), though a vampire in torpor casts no vote. Confirmed (owner, 2026-10-09).

**58.9** Scout Youngwood: her lock targets a ready ally or younger vampire of her prey's (not one in torpor), once each unlock phase, usable from torpor; her +1 blood on reaction cards is paid with the card's cost. Confirmed (owner, 2026-10-09).

**58.10** Karen Suadela's Gehenna immunity, beyond the rulings: her actions and rescues are not surcharged by Torpid Blood, her cards not by The Slow Withering, her replacements not held by Blood Weakens, her capacity not ceiled by Wormwood, she does not burn to Thirst, she hunts freely under Restricted Vitae; read live on her claimed prince title. Confirmed (owner, 2026-10-09).

**58.11** Patsy: the Edge no longer its caller's controller's by resolution, nothing happens (no title removed, no pool burned); the title of a card representing it burns the card. Confirmed (owner, 2026-10-09).

**58.12** Hackerspace: "each Anarch you control" counts Anarch vampires, in torpor too; an Anarch "comes into play under your control" as it is moved out of the uncontrolled region; its counters pay any equip an Anarch of its controller's makes, no lock, as Ravnos Cache's are spent. Confirmed (owner, 2026-10-09).

**58.13** Mask of a Thousand Faces: the masker must be capable of the action's kind (not a granted action, a torpor action or one needing a locked minion), of its action card (clan, Disciplines, the card's title and sect requirements) and of each card the acting minion played on it (clan, one of its Disciplines); the masked minion's inherent stealth already used does not bar it; the masker takes the action's NRA marks; [OBF] is not playable unless stealth is needed (3.2). (Owner, 2026-10-09, correcting the [OBF] reading; the rest confirmed.)

**58.14** SchreckNET: "whenever a referendum succeeds" is any referendum passing in an action, a blood hunt included, any Methuselah's; the hand is open to its user until that action ends. Confirmed (owner, 2026-10-09).

**58.15** Benjamin Rose: his referendum is not conditioned on his title; each passed one adds 1 hand size, kept after he leaves play. Confirmed (owner, 2026-10-09).

**58.16** Dmitra Ilyanova: the allocation of the 4 blood is the terms of the referendum she calls — fixed when the terms are set (picks of ready vampires, any Methuselah's, the same one again as often as wished); blood over capacity is lost to the bank. (Owner, 2026-10-09.)

**58.17** Disguised Weapon [OBF]: played as the strike is chosen, the vampire then chooses its strike (the weapon's included); a weapon whose cost cannot be paid is not offered. Confirmed (owner, 2026-10-09).

**58.18** Unflinching Persistence [FOR]: the later prevention is optional, any damage pending on it this round (aggravated included), once, lost as the round ends. Confirmed (owner, 2026-10-09).

**58.19** Taking control of a unique vampire whose copy one already controls: by choice (Puppet Master's take), not offered; forced, the incoming copy burns (advanced rules 1.13.1). Confirmed (owner, 2026-10-09).

**58.20** A bleed ousting the active Methuselah (by another's minion in their minion phase, Enkil Cog): the turn passes with the action. Confirmed (owner, 2026-10-09).

## Batch 59 — Legacy cards for TWDA decks (steps 10–15), implemented readings (2026-10-09)

Readings no ruling settles, implemented with slice 16l; questions for the owner.

**59.1** Giovanni del Georgio: the card is named as the action is announced (a card two copies of which are in his prey's ash heap; the action is not offered without such a pair); both copies are removed at resolution as its cost — fewer left by then, the action fizzles; +2 hunt is 3 blood; once each turn. Confirmed (owner, 2026-10-09).

**59.2** Quick Meld [PRO]: the additional maneuver is usable later in the same determine-range step (after an opposing maneuver), at either range, once this round, for 1 blood paid as it is used. Confirmed (owner, 2026-10-09).

**59.3** Society of Leopold: put on any Methuselah's vampire, in torpor too; the choice is free while the vampire has the blood, forced (burned) without it; its host's controller's unlock phase; the move action targets any other vampire, any Methuselah's (directed when another's), the card moving on success. Confirmed (owner, 2026-10-09).

**59.4** Vampiric Disease: played on any Methuselah's vampire, in torpor too (on a diseased one, a legal no-op); the counter is the vampire's state, kept through torpor, a change of control and banishment; every genuine unlock of a locked vampire burns (the unlock phase and any card or effect unlocking it); "cannot gain blood by hunting" covers every hunt (rulebook, granted or card hunts; a hunt stealing blood takes none); the spread happens at close range as the before-strikes step opens, between two vampires, either way. Confirmed (owner, 2026-10-09).

**59.5** Inyanga: an ally "requiring a Gangrel" is one printed with the Gangrel clan requirement (Double Deuce); the reduction applies to her recruiting it. Confirmed (owner, 2026-10-09).

**59.6** Matasuntha: the pool is optional, for her controller as the opposing minion leaves the ready region by any route (torpor, burned). Confirmed (owner, 2026-10-09).

**59.7** T.J.: usable during the polling step of a referendum called by a vampire whose title, read then, is a Camarilla one (contested or off-sect titles excluded; a blood hunt is called by none); lock and 2 blood are the cost (unlocked, 2 blood, in torpor too); the political action card is not returned. Confirmed (owner, 2026-10-09).

**59.8** Randall: out of the combat only, in the step for minions out of the combat (after the combatants' own prevention), any Methuselah's combat; locked or in torpor too; once each turn (one minion phase a turn); only while non-aggravated damage is pending on the target. Confirmed (owner, 2026-10-09).

**59.9** Justicar Retribution: "all vampires" are every vampire in play, any Methuselah's, the caller's included, those in torpor too (Stanislava in torpor burns to it); the current bleed is the base, printed text, cards borne and layers reaching it, against its controller's prey, no action modifier; read as the referendum passes; burned. (Owner, 2026-10-09, correcting "ready vampires".)

**59.10** Lodin: only damage from an opposing minion's strike, aggravated first (no allocation asked, unlike 4.2: a strike's damage is seldom mixed); damage to Lodin only (not his retainers); once each combat. Confirmed (owner, 2026-10-09).

**59.11** Mary Anne Blaire: "Inner Circle members" are any Methuselah's, read live on their title (contested or off-sect ones excluded); her action adds the blood to any vampire in her controller's uncontrolled region. Confirmed (owner, 2026-10-09).

**59.12** Victor Donaldson: any cancellation of a Golconda: Inner Peace his controller played (its own 2-pool cancel included), Victor in play (in torpor too); offered once the canceled card is replaced, so the replacement cannot draw it; the library shuffled. Confirmed (owner, 2026-10-09).

**59.13** Ephor: put on any Methuselah's Ventrue with capacity 8 or more (read at play; it stays if the capacity drops, but the action then needs it); the +1 vote is no title; the action targets any vampire whose title is contested, which loses it — its rivals' claims return free in their unlock phase. Confirmed (owner, 2026-10-09).

**59.14** Karl Schrekt: he meets a justicar requirement on the cards he plays (never a master card's, Camarilla Conclave), whatever his sect — a justicar implying the Camarilla (LSJ 20050128); a title requirement he meets this way makes him a titled Camarilla vampire from the card's point of view, so he can call Camarilla's Iron Fist even when no longer Camarilla; no in-play effect of the pool requires a justicar. (Owner, 2026-10-09, over ANK 20250410.)

**59.15** The Signet of King Saul: the capacity is read live at the block attempt (Discipline masters count); allies may block; any action of the bearer. Confirmed (owner, 2026-10-09).

**59.16** Aksinya Daclau: ready and unlocked; once in each Methuselah's turn; the master card discarded is the cost, replaced at once like any card leaving the hand; a legal bleed target only. (Owner, 2026-10-09, correcting a deferred replacement.)

**59.17** Trap, Conquer the Beast: put in play (on no minion) by the combatant's Methuselah; any card played in that combat by anyone resets the counters, a canceled one included; each burns a counter as each press step opens, both combatants ready — Trap's press to continue is no combatant's and applies first, a second Trap's adds nothing (its counter still burns); Conquer's is its vampire's for that round (mandatory at [ani], optional at [ANI]), and its vampire cannot use equipment for the whole combat; burned as the combat ends. Confirmed (owner, 2026-10-09).

**59.18** Fakir al Sidi: during the polling step of a political action before any vote, locked or in torpor too, once per referendum; the referendum fails (not canceled). Confirmed (owner, 2026-10-09).

**59.19** Gerald Windham: the ready titled vampires of other Methuselahs, contested or off-sect titles excluded, "n votes (titled)" included; read when votes are counted; nothing while Gerald has a title. Confirmed (owner, 2026-10-09).

## Batch 60 — Legacy cards for TWDA decks (steps 16–20), implemented readings (2026-10-09)

Readings no ruling settles, implemented with slice 16m; questions for the owner.

**60.1** Theo Bell (G2): the ready prince or justicar is another vampire of his controller's, read live; the G2 and G6 cards are one vampire for contests (no merge: the G2 advanced card is not in the pool). Confirmed (owner, 2026-10-09).

**60.2** Juliet Parr: the referendum of her own political action (not a blood hunt); each one passed adds 2 hand size, until her controller's next unlock phase begins. Confirmed (owner, 2026-10-09).

**60.3** Mistress Fanchon: "a minion card" is any library card but a master or an event. (Owner, 2026-10-09.)

**60.4** The Beast: "action cards" are all the action card types — action, political action, ally, equipment and retainer cards. (Owner, 2026-10-09.)

**60.5** Lord Tremere: the Blood Sorcery reduction applies to any card requiring [tha] he plays; his ability targets any Methuselah's ready Tremere or Tremere antitribu. Confirmed (owner, 2026-10-09).

**60.6** Lord Tremere: "during your minion phase" — once in it (6.6, 58.7): the lock of an unlocked vampire or the unlock of a locked one, in torpor too. Confirmed (owner, 2026-10-09).

**60.7** Nergal: the reduction is his choice, of the card he uses it on — announced with that card (among its cost terms), once each Methuselah's turn, spent only when that cost is paid (canceled unpaid or unpayable: not spent). (Owner, 2026-10-09.)

**60.8** The Capuchin: burned by any means, he goes to his owner's uncontrolled region as a new card with no counter; the +3 transfers are his controller's at the burn, at the start of their next influence phase. Confirmed (owner, 2026-10-09).

**60.9** Jimmy Dunn: a second Jimmy entering play (influenced, or by any effect) burns every other Jimmy in play, any Methuselah's, before any contest. Confirmed (owner, 2026-10-09).

**60.10** Malgorzata: the vampire receiving the blood is the action's target, the one removed named as it is announced; either gone by resolution, nothing happens. Confirmed (owner, 2026-10-09).

**60.11** Street Cred: "younger" than the vampire playing it (the blocker too); the damage compared is the strikes' successfully inflicted this round; one each turn per vampire. Confirmed (owner, 2026-10-09).

**60.12** Under My Skin: the card put on the vampire is burned for +1 stealth only when stealth is needed, in any action of a later action number (the same turn too); several copies each burn on their own. Confirmed (owner, 2026-10-09).

**60.13** Crocodile's Tongue: "younger" than the acting vampire; the blocking minion's cancel is offered while it has the blood or life (an ally paying its last life burns); the bar covers every minion of the blocker's Methuselah for that action. Confirmed (owner, 2026-10-09).

**60.14** Condemnation: Mute: [chi] locks any younger vampire, any Methuselah's; [dai] goes on any ready vampire; [DAI]'s blood burns after any referendum passes, a blood hunt included. Confirmed (owner, 2026-10-09).

**60.15** I am Legion [DAI][OBF]: three plays — the cancel, the cancel with the pool, the pool alone (an action modifier any time in the action); the pool after resolution if the action succeeded (a fizzled one included). Confirmed (owner, 2026-10-09).

**60.16** My Kin Against the World: X is announced with the card; the terms choose exactly X younger vampires of the acting vampire's clan (any Methuselah's, in torpor too), fewer if fewer qualify; "only one … each turn" counts it played. Confirmed (owner, 2026-10-09).

**60.17** The Sargon Fragment: an ally bearer gets nothing ("this vampire"); the card is named among the bearer's controller's ash heap as the action is announced; not offered with an empty ash heap. Confirmed (owner, 2026-10-09).

**60.18** Soul Gem of Etrius: "younger" compares the burned vampire's capacity in game (Discipline cards and other capacity increases included) to the drawn crypt card's printed capacity; the bearer's controller draws; the new vampire enters unlocked (it may act this turn); a drawn copy of a vampire its Methuselah controls burns and the Gem draws again; an empty crypt: the Gem burns. (Owner, 2026-10-09.)

**60.19** The Erciyes Fragments: the lock at any impulse but an as-played window; the card on it stays its owner's (the prey's): played from it, it is removed to its owner's removed cards, a political card Delaying Tactics returns goes to its owner's hand (LSJ 20050322), a card entering play from it is its owner's; a master is played with a master phase action; the card on the Fragments as they leave play burns to its owner's ash heap. (Owner, 2026-10-09.)

**60.20** Call the Great Beast: the 11 counters checked as it is played and after each counter action; no play or counter action while its Methuselah controls a Great Beast (LSJ 20101130 read for every capacity); the Great Beast enters unlocked, three distinct Disciplines chosen from the rulebook's list; its prevention: any damage, once each combat. Confirmed (owner, 2026-10-09).

**60.21** Unleash Hell's Fury: no token — the card itself is a minion for the duration of its block attempt (a 9-capacity infernal vampire, +2 intercept, 0 blood); its successful block ends the action at once — damage and burn, no combat, nothing played on it afterward (Change of Target); it is never locked; offered against an action directed at its controller (an action directed at a card of theirs is directed at them). (Owner, 2026-10-09.)

**60.22** Brujah Debate played during a master phase locks in that same phase: 33.1 revised (owner, 2026-10-09).

## Batch 61 — Owner play feedback (2026-10-10)

**61.1** The end-of-round window follows the press step in every round, a press continuing the combat or not (rulebook 2024 "End of Round"): Taste of Vitae after a round a press continued (owner, from play).

**61.2** "A bleed against your prey" (Enkil Cog, Gostoso) still holds when that bleed ousted the prey: the target is read as if still seated, live otherwise (a redirected bleed still is not, 50.3). Owner, from play: Enkil Cog drawn after an ousting bleed was not offered.

## Batch 62 — Legacy cards for TWDA decks, round 2 (steps 1–6), implemented readings (2026-10-11)

**62.1** Aims (Target Vitals): the strike is chosen first, then an aim may be played on top of it — the striking minion's Methuselah gets one decision after its strike choice, an aim or pass, before the strikes resolve; a strike committed at the maneuver likewise (55.8 stands). Owner, 2026-10-11 (the engine had the aim before the strike choice).

**62.2** Adana de Sforza: 1 blood off a card played as a combat card — one of several types counts as the type it is played as (Rapid Change's [PRO] strike, not its [pro] stealth); never off a "burn blood" effect in a card's text (Charismatic Aura).

**62.3** Alexandra: once in each of her controller's turns, at any impulse of it but an as-played window, from torpor too; any Methuselah's other ready Toreador (49.5), locked if unlocked, unlocked if locked (60.6); a blocker locked during its attempt fails it.

**62.4** François Villon (G2): how much he steals, 1 or 2, is chosen at resolution, not as the action is announced, among what the target has left; the excess over his capacity drains. G2 and G6 are one vampire for contests (60.1), each with its own text. (Owner, 2026-10-11.)

**62.5** Anson: his controller gets a second master phase action while he is ready, read live — the ordinary action spent first, the second lost if he leaves the ready region before it is used; a trifle or an extra action adds to them. An out-of-turn master spends one of them, but there is still only one out-of-turn master per full turn (3.3), Anson or not. (Owner, 2026-10-11.)

**62.6** Lutz von Hohenzollern: any Methuselah's political action whose referendum passes, after all its effects; nothing if his controller was ousted (or is pending an oust) by them; "your prey" read then — a new prey if the old one was ousted; his own pool burn ousts through the normal oust (Life Boon's decision point).

**62.7** Marcellus: an action cost, as Daria's (42.1, 42.6): every Nosferatu's political action, card or granted, any Methuselah's; payable with the action's own blood as it is announced, paid first at resolution, unpaid if blocked; Marcellus's readiness read at both points.

**62.8** Rafael de Corazon: each title rival of his yields in its controller's unlock phase (no pay option); Rafael himself pays or yields as usual.

**62.9** Anneke: her Methuselah is an eligible blocker against any action by a vampire of another Methuselah, for her alone; her other restrictions stand — Seduction, her Methuselah's decline (a pass with no attempt pending), allies acting.

**62.10** Keith Moody: offered to his controller's prey, holding the Edge, as the impulse reaches them in his controller's unlock phase; once that phase; not with Keith at 0 blood (nothing to burn); Keith in torpor included.

**62.11** Owain Evans: in every unlock phase (his controller's own too), once per phase, any vampire of the active Methuselah (in torpor too, one at capacity a no-op); offered once the active Methuselah's mandatory effects are resolved (the unlock rotation).

**62.12** Blackhorse Tanner: any library draw of his controller's — a replacement, a draw-up, a card's draw —, each card drawn offered until the use is spent, once in each Methuselah's turn, from torpor too; the card goes under the library and the new card drawn is not offered again; not offered with no other card in the library.

**62.13** Antonio d'Erlette: in his controller's influence phase, 4 of the phase's transfers, Antonio ready; his library and hand searched at once for a mage ally whose clan requirement he meets and whose cost is payable — reduced as LSJ 20090920-2 rules, except by a reduction keyed to an action (Charisma, 36.5); the library shuffled either way; a pick from hand replaced. Whether the ally counts as recruited is moot: in the influence phase, after the minion phase, it cannot act that turn anyway (owner, 2026-10-11). Veneficti (Baali) is never his.

**62.14** Tupdog: his rush at another Methuselah's minion (17.3); "Tremere antitribu slave" (rulebook "Slave"): no directed action while his controller controls no ready Tremere antitribu (read as it is offered); as a Tremere antitribu of his controller's is blocked, he may lock to fight the blocker instead — the acting vampire unlocked, the canceled combat's riders gone (advanced rules §5.7.2); after the normal end of his controller's minion phase, every Tupdog of theirs burns (in torpor too), each moving one crypt card to the uncontrolled region (an empty crypt: none). No Visceratika card is in the pool; Flight stays a bare trait (16.6).

**62.15** Unmada: the discard phase action if she is ready as the phase begins (as Sreelekha); the hand size read live.

**62.16** The unnamed: the strike's blood paid as it is chosen (not refunded if dodged); 2R aggravated, effective at any range; the 2 pool optional, once after each successful bleed.

**62.17** Victoria Ash: any Methuselah's younger ready vampire, a locked one a no-op lock (57.10). Goratrix: a ranged steal of blood or life, not damage. Omaya: any damage pending on her, once over the combat. Neighbor John: a plain +1 intercept.

**62.18** Canine Horde [ANI]: the opposing minion's equipment named with the strike (Weighted Walking Stick included, a location in play not); with first strike, at either range; not offered against a minion with none (RTR 19980928).

**62.19** Contagion: the card itself goes on the vampire, remembering its level; each other Methuselah burns 1 pool as a mandatory effect of their unlock phase while the vampire is ready (read as it resolves); [DAI]'s strike: close range, once over the combat; its counter is the striker's controller's; the steal optional once their counters equal the minion's capacity or cost (35.2), after the strike's resolution, damage pending applied first, the combat ending.

**62.20** Corporate Hunting Ground: as the other hunting grounds (6.5, 6.6, 7.2).

**62.21** CrimethInc.: the actions it follows: the rulebook action to become Anarch (58.1); an action card requiring an Anarch or a baron or making a vampire Anarch; a recruit, employ or equip of a card requiring an Anarch (owner, 2026-10-11 — the ledger's "Equip/Employ/Recruit action" rows superseded); not a card another card fetches or plays as part of the action (LSJ 20100725: none in the pool); an action a card in play grants Anarchs only (Open War). [qui]: any Methuselah's other ready Anarch (49.5). [tha]: once in its controller's minion phase, 1 pool, a locked Anarch of theirs.

**62.22** Liquidation: 7 library cards needed as it is played; replaced as this turn's discard phase begins (8.4).

**62.23** Memory Rift: one mode per set of its Disciplines used (25.2); [dem]'s vampire any Methuselah's, titled or younger, chosen as it is played; [pre]: "strike: combat ends" offered in the first round of the combat its block starts.

**62.24** Nephandus: the reduction is its own step, before prevention, for each strike of the opposing minion — hand, melee weapon, ranged, unpreventable included — and a strike's damage after combat (Catatonic Fear); not environmental damage (Carrion Crows), nor an aim's add-on, which comes after (55.1). His strike: 1R; his press: once over the combat; his torpor burn: any Methuselah's vampire, +1 life over his starting life.

**62.25** Powerbase: Barranquilla: the ready Sabbat vampire chosen as it is played (none: not playable), its capacity read as it resolves; the unlock move once each unlock phase; the burn: any Methuselah's vampire, +1 stealth while the acting vampire is titled, read live; burned once no counter is left.

**62.26** Psychic Assault: normal at its source — an aggravating effect (Dawn Operation) leaves its damage normal; Black Hand read on the striker as the strike resolves; unpreventable damage is still reduced by Nephandus (LSJ 20100519).

**62.27** Ruins of Ceoris: one use over the combat, a maneuver (before or as range is determined) or a press; for a Tremere antitribu bearer only, read live.

**62.28** Sense the Sin: [aus] against a younger acting vampire; [dai]'s extra +1 if the acting minion has anyone's corruption counter; [DAI]'s -1 intercept to every minion with any counter, any Methuselah's, read live during the action.

**62.29** The Unmasking: every Methuselah's allies, +1 intercept while the acting minion is a vampire; replaced as its controller's next turn's discard phase begins.

**62.30** Veneficti: +1 stealth; not the target of a directed action of an ally or of a vampire of capacity 6 or less (read as it is offered); he steals a demon ally — Herald of Topheth, added for it (owner, 2026-10-11) —; no demon retainer is in the pool; his burn option per his printed clan, Baali — any of the pool's Baali is infernal.

**62.31** 41.8's "no mage ally in the pool" no longer holds: Sri Sansa moves blood or life with Nephandus and Veneficti too.

**62.32** "Once each turn" on an in-play ability its controller uses (Ferraille, CrimethInc. [tha]) is now enforced across the turn, not only within one referendum (a fix seen in the random games). Known edge: Ferraille's two modes (votes for, votes against) are limited each on its own — one each in two referendums of a turn is still offered.

**62.33** Herald of Topheth (owner, 2026-10-11): an infernal ally — it does not unlock as normal, its controller may burn 1 pool to unlock it, as an infernal vampire (`GameState::unlocks_as_infernal`); it plays [dai], [pot] and [pre] cards as a capacity-5 Independent vampire; its rush at another Methuselah's minion; its life back to 5 at most, 1 each of its controller's unlock phases.

## Batch 63 — Legacy cards for TWDA decks, round 3 (the owner's list), implemented readings (2026-10-11; owner: all confirmed)

**63.1** Blood cursed (Alu, Wise Frog; rulebook "Blood Cursed"): the vampire commits no diablerie — no diablerie action, no diablerie of a blocked leave-torpor vampire, no Mask take-over of a diablerie; Abactor's blood hunt and a rescue stay open to it.

**63.2** Abactor: +4 hunt is 5 blood with the hunt bonuses on top (59.1: Aaron's Feeding Razor, Hospital Food); "successful resolution" is an unblocked action that gained blood (LSJ 20090411) — at capacity or diseased, no blood hunt; the blood hunt is part of the action, before its after-resolution window (Freak Drive, CrimethInc. after it; Heidelberg Castle not during it), its rotation from the hunting vampire's controller; no diablerie is recorded (Carlton Van Wyk, Rebirth); a special hunt, once per minion each turn, open to a vampire that must hunt and to a Ravnos under Week of Nightmares (Week's own steal does not combine).

**63.3** Confusion of the Eye: [obf] against the bleed of an ally or a vampire younger than the reacting one. [OBF] in a political action's polling step only, before any vote or ballot (a blood hunt takes no reactions; an automatic pass opens no polling step); the acting vampire casts nothing; "If the referendum requires a titled vampire" reads any title requirement on the calling card or granted action (prince, justicar, baron, "titled Sabbat"…), not only the literal "Requires a titled vampire". Owner, 2026-10-11: confirmed.

**63.4** Alpha Glint: "an older vampire" by capacity, read live — equal capacity is not older; against an ally, neither mode. [ani][for]'s "combat ends" strike resolves first, before any first strike (rulebook "Combat Ends"), so no damage lands beside it: of the end-of-round cards RTR 20001020 names, Taste of Vitae is offered (for nothing), Disarm never is (it needs damage inflicted that round). [ANI][FOR] ends the combat in the before-range step of any round, Weather Control's damage of that step landing first (LSJ 19971110); the range stays at its default.

**63.5** Preternatural Evasion [CEL]: the blood burned as the strike is chosen (ANK 20200703-2), so not offered to a vampire with none, nor to an ally ("this vampire"); a burn, not the card's cost — Adana de Sforza does not reduce it.

**63.6** Torrent [CEL]: "the action continues as if unblocked" is mandatory and free, applied as the after-combat window closes — only for a vampire blocked while acting (not after a rush's combat, RTR 19970630); lost if the combat goes on to another round (Telepathic Tracking, RTR 20020501); the action's modifiers, reactions and the Methuselahs' declines stand (ANK 20190116); an action card's resolution follows (LSJ 20070808-1).

**63.7** Focus the Blood: the card itself goes on the vampire with 1 blood from the bank; one play each combat per vampire, copies from earlier combats stacking; each copy, burned as its host pays a card played as a combat card (62.2), takes 1 blood off it, in any later combat too, announced with the card (36.1).

**63.8** Renegade Garou: his additional strike each round is mandatory (PIB 20121028), right after the first pair of strikes, and takes the round's limited slot; his maneuver once each combat; his rush +1 stealth at another Methuselah's minion; his unlock-phase life gain from the bank at 2 or fewer life.

**63.9** Concealed Weapon: the weapon named as the card is played, still in hand until it resolves, then paid and shown (LSJ 20100130); "cannot cost 3 or more pool" reads its cost as modified in play, not the reductions announced with it nor another source paying it (Ravnos Cache's counters pay a .44 Magnum, never let in an AK-47; LSJ 20080125); "inflict (with a regular strike)" reads the card's printed strike — the minion's strength aside (Righteous Blade's strength+1 is 2); requirements apply (a Salubri's Righteous Blade); not the equip action — the minion may still equip that turn (LSJ 20080815) — and no action-keyed reduction (Charisma).

**63.10** Learjet: "a card this minion played as an acting minion" is any card the bearer plays while acting in the action, its combats' cards and a canceled card included; the extra draw is offered after the normal replacement is drawn (LSJ 20050224-4), and with a delayed replacement when that one comes due (PIB 20110718, Blood Weakens); "a minion may have only one vehicle".

**63.11** Infernal Pursuit [CEL]: from its own replacement on, each replacement in the combat draws one extra card per copy played, then the hand is discarded down; none past the library (LSJ 20060620) or once the combat is over. Several cards replaced at once (Blood Weakens' held replacements, released by an empty hand; the two combat cards discarded to cancel Target Vitals) are replaced one at a time, each followed by its extra cards and as many discarded (ANK 20180925-1 literally: "discarding each time X cards" — even below hand size). Owner, 2026-10-11: confirmed.

**63.12** Hospital Food: any Methuselah's anarch's hunt (49.5); "Lock when an anarch announces a hunting action" its controller's option, usable on another Methuselah's anarch too; a hunt bonus of that action — a blocked hunt gains nothing, one at capacity has nothing to add to though successful (LSJ 20050720-2). Owner, 2026-10-11: confirmed.

**63.13** Baltimore Purge: the picks clockwise from its controller, in the open, among each other Methuselah's ready vampires of capacity not above the purging vampire's, read as each pick is made; a Methuselah with none is skipped; a purging vampire already in torpor or gone goes nowhere, the chosen still do; all at once (LSJ 20050111). Its Ravnos +1 stealth is read as the action is announced.

**63.14** Illusory Resources: the unlock-phase move once each unlock phase (6.6), by a ready vampire with Chimerstry; the burn of X counters once each bleed against its controller.

**63.15** Renewed Vigor: [obe] at any Methuselah's torpid vampire, [OBE] at any other vampire, torpid included; a target already at full life or capacity stays a legal no-op (17.4); [for]'s unlock-phase burn only for a bearer in torpor.

**63.16** Gregory Winter: his steal at a vampire with no blood is legal and steals nothing (RTR 20010711); his torpor burn raises his life over his starting life; his two actions do not restrict each other (LSJ 20100909); his unlock-phase life burn can burn him. Aaron's Feeding Razor: +1 to every hunt of its bearer, mandatory. Abombwe: any Methuselah's vampire with Protean (no Laibon in the pool); put on by Childe of the Revolution's search, its requirement ignored (LSJ 20051116-2). Lisette Vizquel: +1 intercept as she attempts to block a diablerie action. Wise Frog: first strike in a combat with a vampire younger than her, read as the strikes resolve. Cameron and Hester Reed have no text.

**63.17** Seen while testing: an action card's "+1 stealth if this vampire is Ravnos" (Baltimore Purge) is read as the action is announced and skipped at resolution (it crashed the resolution); Focus the Blood's discount was offered but not taken on a combat card's blood cost (now taken, the card burned as the cost is paid); Renewed Vigor's "add enough life to reach their starting life" overflowed its counter.

## Batch 64 — Legacy cards for TWDA decks, rounds 4–5 (the Gangrel decks), implemented readings (2026-10-11; owner, 2026-10-14: 64.7, 64.8 confirmed)

**64.1** Ariadne: her "+1 hunt" adds to every hunt of hers, non-standard ones too (RTR 20030519); no blood hunt is ever called on her (her diablerie, an Abactor). Jacob Fermor: "werewolves you control" are allies of the Werewolf subtype (Renegade Garou, Ossian), +1 bleed while Jacob is ready (in torpor: none), read live; his press once each combat. Gunnar, Lillian and Robert Price print no text.

**64.2** Anachronism: on any Methuselah's vampire of capacity 7 or more; "ranged weapons" are the weapons with a ranged strike (LSJ 20060825), Zip Gun included, the Ivory Bow excepted; those on it burn as it enters; it cannot equip one (from hand, by an action or another card), move one to it, nor use one, nor play Zip Gun.

**64.3** Bone Spur: an untimed combat card — before the strike it should affect resolves, after both strikes are chosen too (RTR 19960112); [pro] this round's hand damage, [PRO] every later round's of the combat as well.

**64.4** The Kiss of Ra: against a block attempt by a vampire without Fortitude (the Fury of Unleash Hell's Fury included: LSJ 20100210); the blocker burns 2 blood (as much as it has), [FOR] then goes to torpor; the action is canceled before resolution: nothing more is played in it (RTR 20180719), no outcome — no after-resolution window, so no Daring the Dawn damage —, no torpor-block diablerie (LSJ 19970325), no NRA mark (the same action may be taken again, unless it was blocked then continued before: ANK 20211015), still taken (the optional action, Veil the Legions' count: ANK 20210124); the acting minion stays locked.

**64.5** Mind of the Wilds: [ani][aus] in the action's own window, [ANI][AUS]'s +1 stealth only when needed (3.2); the maneuver banked for the first round of the block's combat only; the blocker barred from "strike: combat ends" for the rest of the action, any round of any combat (LSJ 20100308).

**64.6** Ossian: his rush at any vampire, any Methuselah's; the freeze read as cards are played: a vampire opposing him that played a card requiring Auspex in the current action (any action, LSJ 20050201; the Discipline the card was played with) uses no maneuver nor strike for the rest of the first round — from the combat's start, or from that card on mid-round (an Auspex strike played first stands: ANK 20210614); a range set by an effect stays open, a Sniper Rifle's (a strike commitment) not (LSJ 20100827). His life: the blood the opposing vampire lost to damage that round, at the end of the round, after the presses, the round ended early too (RTR 20030519), over his starting life (ANK 20171109). "Red List" is a bare trait.

**64.7** The Path of Lilith: offered as the torpor lands — the opposing vampire sent to torpor in combat (not burned), the vampire opposing it non-Camarilla and the card's controller's, the card unlocked; its controller's unlock phase: each Methuselah controlling a vampire with a torture counter (ready or in torpor), its controller included, burns 1 pool. Every torture counter goes once no Path of Lilith is in play (one held by another Methuselah keeps them all: none other in the pool).

**64.8** Predator's Transformation: the reflex by the vampire the frenzy card is played on (rules-qa 28.11), locked or not, its cost still paid (ANK 20260216); [abo] +1 stealth as the action is announced, even if not needed; [ABO] at the end of the round in which this vampire's combat burned a minion of its controller's prey, or after the resolution of its directed action that burned one out of combat (ANK 20220507), once per action. That a locked vampire may play a reflex is a reading, confirmed by the owner (2026-10-14).

**64.9** Read the Winds: replaced after the action (Blood Weakens' rules aside); [ani][aus] keeps its vampire unlocked after a successful block; [ANI][AUS] prints no "as above": its vampire locks as usual; the usual blocking restrictions apply (ANK 20180623, LSJ 20010714-2).

**64.10** Zip Gun: before range is determined in any round, put on the minion; in play it is weapon equipment — a target for "equipment" effects (Canine Horde) as 22.4 has it (this extended 22.4 to card targets; five random opening games changed) — and a gun (Kevlar Vest); 1R damage, its maneuver committing the strike (LSJ 20021028); its bearer takes 1 environmental damage the first time it strikes with it each combat, none if the gun is gone first (LSJ 19970801, LSJ 20100310). No ammo card is in the pool.

**64.11** Generators: the window matrix reads "when … attempts to block" (The Kiss of Ra), "when this vampire burns a minion" (Predator's Transformation [ABO]: no in-play action flag) and "with an optional maneuver each combat" (Zip Gun: a rider, no play window); "As above, with +1 stealth" over a mode with no printed timing keeps the stealth window alone (Mind of the Wilds [ANI][AUS], 3.2; Mask of a Thousand Faces keeps its timings); a mode's two type tags ([ACTION MODIFIER][COMBAT]) are read in a fixed order (the generator was nondeterministic on Predator's Transformation).

**64.12** Owner, 2026-10-11 (relayed by Z-play): as a political referendum's polling step opens, the other Methuselahs get a chance to play the cards usable "before any votes and ballots are cast" before the caller casts — Confusion of the Eye [OBF] (63.3), Fakir al Sidi (59.18), Closed Session. The engine opens it with a pre-vote rotation from the caller's left, the caller last, offering those cards (and their abilities) alone, no vote; once it closes, the polling proper rotates from the caller. Not for a blood hunt, nor a referendum passing automatically.

## Batch 65 — Legacy cards for TWDA decks, round 6, implemented readings (2026-10-11)

**65.1** Psyche!: [CEL] in the end-of-round window as the combat would end, both combatants ready, no combat queued (RTR 20020501) — no "would end" replacement after it (Telepathic Tracking, Hunting the Quarry: ANK 20191219), the end-of-round cards still open before or after it (LSJ 20021113). The queued combat begins as this one ends, the same two minions, still in the action (LSJ 20021213) and ending as this one would have (into the after-combat window, or back to an action's step); this combat's after-combat effects are lost (a continuation as if unblocked, Torrent's included; damage or a card after combat), unlocks aside, applied as their strikes resolved (LSJ 19980109); cards held "until after combat" are replaced before it (LSJ 20001122); no block began it, so no block rider nor "when blocking" equipment carries over (Spirit's Touch, Sniper Rifle: LSJ 20010813, LSJ 20010814); a hand size the first combat lowered is drawn up between them and discarded down again (Raptor, LSJ 20030530).

**65.2** Neighborhood Watch Commander: any Methuselah's, after the resolution of any Methuselah's blocked or failed action whose acting minion is a vampire; ready and unlocked, it locks to inflict 1 normal damage out of combat (the vampire burns a blood, else goes to torpor); one Commander each action, any controller's.

**65.3** Tabitha Fisk: as Silvian Popescu — her controller's unlock phase, once, from an archbishop of theirs with blood, she in torpor too. Tatiana Romanov: +1 bleed. Delilah Easton and Isabel de Leon print no text.

## Batch 66 — Play feedback: the as-announced window, stealth a block can still reach (owner, 2026-10-12)

**66.1** The as-announced window holds only the effects whose text says so ("only usable as the action is announced", "as … announces", Seduction, Warsaw Station, Yoruba Shrine, Screamer's fail): an effect usable during the action (Screamer's and Aranthebes's -1 stealth, Haqim's Law: Retribution's +1 bleed) waits for the block attempts — the reacting Methuselah's with its blocks and unlocks, the acting one's with its other modifiers (owner, from play).

**66.2** A block attempt is offered when the stealth may yet be met, and stealth an ability in play can still take off (Screamer, Aranthebes) counts as much as intercept: a woken, locked Monica Giovanni attempts against a 1-stealth bleed bounced back to her Methuselah, who controls Screamer (owner, from play; refines 1.3's pruning). Superseded by 73.8 for the engine; the host's opportunities filter keeps the reading.

## Batch 67 — Legacy cards for TWDA decks, round 7: readings no ruling settles (Claude, 2026-10-12)

Slice 16s: the next hundred cards by TWDA importance (ten new archetypes). Only the readings that no ruling or default reading settles; each is implemented and tested, for the owner to confirm or correct.

**67.1** The Erciyes Fragments (a latent bug the roster reshuffle surfaced): its controller ousted, another Methuselah's card stored on it goes to its owner's ash heap — as another's card on a removed minion does (owner, 2026-09-06) — not out of every zone. (`remove_seat`; test the_erciyes_fragments_card_returns_to_its_owner_when_its_controller_is_ousted.)

**67.2** Emerald Legionnaire: "if you control fewer copies of Emerald Legionnaire than ready unique Harbinger of Skulls" — read as the ready Harbingers of Skulls *you* control (crypt vampires), the ally ready itself (ANK 20180511-1). (Test emerald_legionnaire_raises_another_from_the_ash_heap.)

**67.3** Donatello Giovanni: "if the chosen vampire attempts to bleed before your next unlock phase, they move 1 blood to Donatello" — every bleed it announces until then, not the first only; the chosen vampire any Methuselah's but the predator's, his own included. (Test donatello_giovanni_taxes_the_chosen_bleeder.)

**67.4** Spiritual Guidance: "play a combat card from your ash heap as if from your hand" — owner (2026-10-12): it cannot be forgone: the card is played immediately, as the resolution of Spiritual Guidance, the impulse not going around; its requirements and cost as normal (the ash heap's combat cards the only options of its player's next decision; none playable, nothing happens). (`Engine::ash_play_due`; tests spiritual_guidance_plays_from_the_ash_heap, spiritual_guidance_forces_a_choice_among_the_ash_heaps_plays, spiritual_guidance_with_nothing_to_play_from_the_ash_heap.)

**67.5** The Red List (rulebook "Traits", Enkidu): the master phase action marks another Methuselah's Red List minion (one's own is not offered); the action to enter combat with it is a +1 stealth Ⓓ rulebook action, once per vampire per turn, its 1 blood paid at resolution (unpayable then: it fizzles, as other action costs, rules-qa 3.6); trophies are not modeled (none is in the pool). (Test the_red_list_marks_enkidu_for_a_rush.)

**67.6** New Blood: "choose any circle … New Blood belongs to the chosen circle" — not modeled, no card of the pool reading a circle (Unwholesome Bond and Angelo are not in the pool); the crypt draw as he enters is. (Test new_blood_draws_a_crypt_card_as_he_enters.) Superseded by 68.1 (slice 16t: circles modeled).

**67.7** Trochomancy: "remove thirteen (seven) cards in the target Methuselah's ash heap from the game to get +1 bleed" — the removal is the price: not playable unless that ash heap holds them (burned vampires counted, as Shroud of Decay's, rules-qa 42.7). (Test trochomancy_burns_the_targets_ash_heap_for_bleed.)

## Batch 68 — Blood Brother circles, legacy cards for TWDA decks, round 8: readings no ruling settles (Claude, 2026-10-12)

Slice 16t: the sixty cards of the ten Blood Brother TWDA decks. Only the readings that no ruling or default reading settles; each is implemented and tested, for the owner to confirm or correct. A vampire with no circle is in a circle of one (LSJ 20020609): `GameState::same_circle` needs two Blood Brothers of one printed (or chosen) circle.

**68.1** New Blood: "choose any circle" — owner (2026-10-12): a circle printed in the crypt (Mexico City, Torrance, Kiev, Chicago, Budapest), or his own circle of one (Pass), as he enters play (`Interrupt::ChooseCircle`); "the unique circle of one of any vampire" (ANK 20211113) read where a card reads it, Unwholesome Bond [SAN] on one circle-less uncontrolled Blood Brother (68.7). Supersedes 67.6. (Tests new_blood_chooses_a_circle_or_keeps_his_own, a_blood_brother_with_no_circle_is_in_a_circle_of_one, unwholesome_bond_lets_new_blood_feed_a_circle_of_one.)

**68.2** Slake the Thirst: "only usable after this Blood Brother gains blood" — each effect's gain is recorded where blood is added (`GameState::set_blood`); a gain stays usable while the real decisions stay in the window of the first real decision after it (the play's own terms and as-played window, an effect's choices as it resolves, aside), so a strike's gain as the strikes end; several gains: the one answered chosen by its amount (LSJ 20100203). Owner (2026-10-12): it is offered between a diablerie and its blood hunt (ANK 20201228) — a window of its own (`ActionState::AfterDiablerie`, `Window::ActionAfterDiablerie`). (Tests slake_the_thirst_shares_a_hunt, slake_the_thirst_again_on_the_same_gain, slake_the_thirst_answers_one_gain_of_several, slake_the_thirst_after_a_diablerie.)

**68.3** Hermana Hambrienta ("burns 2 blood or is burned"), Robert Carter ("Carter's employer burns 1 blood, or Carter is burned"): the controller's choice, as Society of Leopold's same wording; the Hermanas only as influenced out (after the excess drained, LSJ 20051126). (Tests a_hermana_burns_two_blood_as_she_enters_play, robert_carter_bleeds_for_blood.)

**68.4** Octopod, Three's a Crowd, Coordinate Attacks: "another one (you control) in combat" — its player's combatant, the card played by a minion out of the combat; Octopod's strikes each round come after the normal pair, as Renegade Garou's (mandatory, limited). (Tests octopod_gives_strikes_each_round, threes_a_crowd_closes_and_presses, coordinate_attacks_sends_the_next_one_in.)

**68.5** Shell Game: X = 0 is a legal no-op (17.4); "you may unlock one of those" — offered after a successful action, one (two at [SAN]) of the locked ones, its player's pick. (Test shell_game_locks_the_circle_for_stealth.)

**68.6** Coagulated Entity: the blood moved as the action resolves, before the combat; any chosen Blood Brother unable (no blood, gone) and the action is canceled: no combat. The +X capacity lasts until the action ends, the excess then drains. (Test coagulated_entity_feeds_the_rush.)

**68.7** Unwholesome Bond [SAN] for New Blood: one chosen uncontrolled Blood Brother with no printed circle (LSJ 20050927, ANK 20211113), a mode of its own; [san]'s X counts any Methuselah's ready unlocked Blood Brothers of the circle at resolution. (Tests unwholesome_bond_counts_the_circle, unwholesome_bond_lets_new_blood_feed_a_circle_of_one.)

**68.8** Sanguine Instruction: the target chosen first, then the Discipline — one the actor has at superior and the target not; a target with none of them is not offered; any Methuselah's vampire of the acting vampire's clan, the actor included. (Test sanguine_instruction_teaches_a_discipline.)

**68.9** Dark Influences: as Direct Intervention, played on another Methuselah's turn; its shield is any Dark Influences in play, whoever controls it, against a cancel of a minion card of another Methuselah than the canceling one. (Test dark_influences_cancels_and_shields.)

**68.10** Jerry ("move any amount of blood from the Blood Brothers in the Chicago Circle to any other Blood Brothers in this circle") and Redistribution ("among the Blood Brothers in this circle"): its controller's Blood Brothers only (32.7); owner (2026-10-12): one move at a time — x blood between two of them (the source, the destination, then the amount), once for a given source and destination, or (Redistribution) any number of equipment, one at a time — each move a separate part of the effect: Slake the Thirst answers one move, x blood. Jerry once each master phase. (Tests jerry_moves_the_circles_blood, redistribution_moves_blood_and_equipment, redistribution_moves_equipment_freely_and_blood_once_a_pair, slake_the_thirst_answers_one_move.)

**68.11** Monocle of Clarity: its question changes no game state — the equip action alone; its rulings n/a. (Test monocle_of_clarity_is_equipped.)

**68.12** Inner Essence: "once this combat" — the card stays on the vampire for the combat (as Focus the Blood's discount, 36.1) and burns as the combat ends if unused. (Test inner_essence_discounts_a_combat_card.)

**68.13** Major Boon: after it is played (as blocks are declined) or burned, the blocks-declined window closes — only "would be successful" plays (LSJ 20090205). (Test major_boons_do_not_stack.)

**68.14** Hive Mind: [san]'s "another ready Blood Brother of the same circle" any Methuselah's; "only one Hive Mind can be played at [san] each turn" counts [san] plays only. (Test hive_mind_unlocks_the_circle_or_feeds_it.)

**68.15** Gestalt: all or nothing at resolution (a chosen Blood Brother unable to burn: no effect); [SAN]: the reacting vampire plays cards with the chosen ones' Discipline levels until the action ends. (Test gestalt_lends_the_circles_strength.)

**68.16** Zizi "can lock to unlock another Blood Brother of the same circle": no printed timing — at any impulse, Viri ready. (Test viri_and_zizi_work_together.)

**68.17** Loki's Gift [PRO] "burn 1 blood from any vampire": the vampire chosen as the card is played, its action directed at that vampire's Methuselah when another's. (Test lokis_gift_hunts_and_bites.)

## Batch 69 — Shadow Play, The Embrace, the Appolonius merge, legacy cards for TWDA decks, round 9: readings no ruling settles (Claude, 2026-10-12)

Slice 16u: the cards of the Lasombra, Toreador antitribu and Ravnos TWDA decks and Popó's. Only the readings that no ruling or default reading settles; each is implemented and tested, for the owner to confirm or correct.

**69.1** Gurchon Hall: "the two oldest ready vampires you control" — oldest by capacity (rulebook); a tie for either place is its controller's pick; one already fed by a hunting ground this turn gains nothing and still counts as one of the two (the next oldest does not step in). (Test gurchon_hall_feeds_the_two_oldest_ready_vampires.)

**69.2** The Eldest Are Kholo: "each Laibon Ravnos gets +2 votes" / "each Ravnos gets +1 vote" — no "you control": every Methuselah's (unlike Power Structure's "each Lasombra you control"), read live for the rest of the referendum. Palla Grande's "Toreador antitribu get +1 bleed" likewise every Methuselah's. (Tests the_eldest_are_kholo_gives_every_laibon_ravnos_votes, palla_grande_gives_toreador_antitribu_bleed_for_three_turns.)

**69.3** Ayelea: "give you control of a Tremere antitribu until the end of your next turn as a Ⓓ action" — another Methuselah's, as it is (locked, in torpor); "your next turn" is the one after the turn the action resolves in: the vampire returns at its end, as Puppet Master's. (Test ayelea_borrows_a_tremere_antitribu_until_the_end_of_her_next_turn.)

**69.4** Gird Minions: the moves chosen as it resolves, as Redistribution's (68.10) — a vampire, then an amount up to its capacity (rulebook: no more blood than capacity), again until Pass; vampires in torpor too; moving the last pool counter ousts. (Test gird_minions_moves_pool_to_vampires.)

**69.5** Merging (rulebook "Advanced"): in the influence phase for 4 transfers and 1 pool, the vampire in play ready or in torpor, not already merged (`Choice::Merge`, offered where influencing the other version out is not). Merged: the advanced card on top — its capacity, Disciplines and sect (an anarch token gone with another sect, advanced rules 5.1.2) —, its [MERGED] title first, else its printed title, else the base card's; the base card's text whole (Appolonius's optional press and +1 bleed beside the advanced card's). Burned, both cards go to the ash heap. Legacy finding no base version is still successful (the vampire unlocked). (Tests appolonius_merges_in_the_influence_phase, legacy_merges_an_advanced_vampire_with_its_base.)

**69.6** Web of Knives Recruit: the card in the uncontrolled region is out of play and no vampire (no influence, no effect on cards in play reaches it); the counter burns as a mandatory unlock-phase effect ordered among the others; with three or more in training, the two are the player's picks; the new vampire enters ready, unlocked, its card the vampire (as Childe of the Revolution's). (Tests web_of_knives_recruit_trains_into_a_vampire, web_of_knives_recruits_train_two_at_a_time.)

**69.7** Marciana Giovanni: "during an action, you can lock her" — any action, her Methuselah's or another's; the non-Sabbat vampire chosen any Methuselah's, herself included (LSJ 20100527). (Tests marciana_lets_a_vampire_burn_blood_for_intercept, marciana_locking_during_her_own_attempt_fails_it.)

**69.8** Ambrosio Luis Monçada: "actions directed at him cost 1 additional pool" — the acting Methuselah's pool, paid with the action's cost as it resolves (not if it is blocked); paid down to 0, they are ousted and the action ends. (Test ambrosio_taxes_directed_actions_and_cannot_block.)

**69.9** Antonio Delgado: "he can unlock during the discard phase" — that turn's discard phase, after his referendum passed. (Test antonio_delgado_unlocks_after_his_referendum_passes.)

**69.10** Poison Pill: the pool its player lost to the referendum's effect (a gain offsetting none, ANK 20180813) is lost by the acting Methuselah in the same batch, ousts at once. (Test poison_pill_hands_the_loss_back_to_the_caller.)

**69.11** Cryptic Rider: "only usable on a successful referendum" — after the resolution of a political action whose referendum passed (Voter Captivation's timing), not after a blood hunt; "the next referendum a vampire you control calls this turn" — the next political action of any of the player's vampires. (Test cryptic_rider_passes_the_next_referendum.)

**69.12** Tenebrous Form [OBT] and Where the Veil Thins [OBL] in one action: each its toll — a minion with neither Discipline burns 2 to attempt; one already attempting pays the new card's 1 as it is played, or resigns. Owner (2026-10-12): a second copy of the same card adds its own 1 too (only reachable through another minion's play, e.g. Mask of a Thousand Faces, not in the pool). (Tests tenebrous_form_and_where_the_veil_thins_add_their_tolls, tenebrous_form_makes_an_attempt_pay_or_resign.)

**69.13** Appolonius (advanced): "+1 bleed for each titled vampire controlled by the target Methuselah" — vampires in torpor count (controlled). (Test appolonius_bleeds_for_each_titled_vampire_of_his_target.)

**69.14** Mozambique Allure: "Requires a Laibon or non-titled vampire" — a Laibon (titled or not) with a crypt card (LSJ 20070411-2), or a non-titled non-Laibon; becoming Laibon replaces the sect; its clans read with their V5 names (Assamite: Banu Haqim, Follower of Set: Ministry). (Test mozambique_allure_makes_a_laibon_or_draws_a_crypt_card.)

**69.15** Flames of Insurrection: the offer goes to the Anarch's controller whoever controls the card (ANK 20180627-2), optional, the vampire in their uncontrolled region their pick. (Test flames_of_insurrection_feeds_the_anarchs_uncontrolled_region.)

## Batch 70 — Legacy cards for TWDA decks, round 10 (the 2025–2026 decks): readings no ruling settles (Claude, 2026-10-13; owner, 2026-10-14: 70.1–70.4, 70.6–70.8, 70.10–70.12, 70.14–70.16, 70.19 confirmed, 70.17 corrected)

**70.1** Rumors of Gehenna: the Methuselahs chosen in its terms (the caller may choose themselves) are remembered by the card in play; "two master phase actions (instead of one)" is the same second action as Anson's — neither another copy nor Anson adds a third. (Test rumors_of_gehenna_gives_the_chosen_two_master_phase_actions.)

**70.2** Victoria Ash (G7): "give an acting Assamite +1 bleed" — an acting Banu Haqim (the clan's V5 name), any Methuselah's, the text naming no controller; her Methuselah discards; once each action (rules-qa 6.6), before the bleed resolves (ANK 20231124). (Test victoria_ash_gives_an_acting_banu_haqim_bleed_before_resolution.)

**70.3** Chameleon: "who entered play since your last minion phase" — since the end of the player's previous minion phase: entries of their current minion phase count; X is the target's capacity, paid in blood; the target ready or in torpor. (Tests chameleon_steals_a_vampire_that_entered_not_one_that_merged, chameleon_makes_ones_own_vampire_anarch.)

**70.4** The Meddling of Semsith: "does not replace cards during any Methuselah's minion phase" — the replacements are held and drawn as that minion phase ends (or the turn ends early), counting against the hand size meanwhile (ANK 20180318); the card leaving play mid-phase does not release them before the phase's end. (Tests meddling_holds_replacements_through_the_minion_phase, meddling_outlasts_a_cards_own_delay.)

**70.5** Camarilla Threat: each copy in play adds its pool; paid as the card is discarded, a last pool ousting its Methuselah. (Test camarilla_threat_charges_a_pool_per_discard.)

**70.6** Eat the Rich: "each priscus gets -1 ballot" — its ballots in all lessened by 1 (Gratiano, a priscus with "+1 ballot", keeps one: owner, 2026-10-14); "each non-priscus titled vampire gets -1 vote" — off its total votes in this referendum, never below 0; the Edge cast is worth 2 votes; "a ready titled vampire" read as it passes. (Tests eat_the_rich_taxes_titles_and_doubles_the_edge, eat_the_rich_leaves_gratiano_one_ballot.)

**70.7** Anathema: "reduced to 0 in combat" read as each combat step settles: any loss to 0 in it (damage, a cost paid in blood); a vampire back above 0 by then is not burned; the pool to the Methuselah controlling the opposing minion then. (Tests anathema_burns_a_vampire_reduced_to_zero_not_one_at_zero and the others citing its rulings.)

**70.8** Protected Resources: copies do not lower the cap below 2; "after a minion you control successfully bleeds" — any minion of its controller's, an ally too, every copy they control burning; the cap is on whoever burns pool for the bleed (Major Boon's payer included). (Tests protected_resources_caps_the_pool_burned_not_the_bleed, protected_resources_burns_after_its_controllers_successful_bleed.)

**70.9** Antediluvian Awakening: the vampire burned may be in torpor; burning it burns the card; chosen as the unlock rider resolves, the pool due anyway if the card is gone by then. (Test antediluvian_awakening_costs_a_pool_or_a_vampire.)

**70.10** Kaymakli Nightmares: a created vampire in an uncontrolled region (banished) is burned, its counters still to the pool and not counted for the draw; a Web of Knives Recruit in training is no crypt card and stays; "only one … per game": anyone's, a canceled play counting. (Test kaymakli_nightmares_reshuffles_every_uncontrolled_crypt_card.)

**70.11** Fast Hands: "steal weapon" — the weapon named with the strike moves to the striker as it lands (a steal: no requirement or cost), not if the striker cannot have it; with first strike, a strike with that weapon resolves no more. (Test fast_hands_steals_a_weapon_and_needs_one.)

**70.12** Ni Dieu ni Maître: [for] "vampires must burn 1 blood" — vampires only, an ally attempts for free; [ani] "cannot maneuver" — no maneuver at all, granted ones included, for the resulting combat. (Tests ni_dieu_ni_maitre_tolls_vampires_not_allies, ni_dieu_ni_maitre_ani_vic_bars_maneuvers_and_strengthens.)

**70.13** Horrific Countenance: "it is now unblockable" — for the rest of the action, no minion may attempt to block it. (Test horrific_countenance_unblocks_the_action_for_good.)

**70.14** Approximation of Loyalty: [pre] any Methuselah's ally or vampire of capacity 6 or less; [PRE] "no more action modifiers" — no card played as an action modifier by the acting Methuselah's minions for the rest of the action. (Tests approximation_of_loyalty_bars_a_small_blocker, approximation_of_loyalty_cancels_a_small_reaction_and_closes_modifiers.)

**70.15** Orgy of Blood: X is the ally card's printed pool or blood cost; "each vampire you control" — in torpor too, up to capacity; played in the end-of-round window of the round the ally burned. (Test orgy_of_blood_by_a_vampire_going_to_torpor.)

**70.16** Drawing Out the Beast: its +1 strength is a frenzy effect like the rest (Tranquility Shield cancels it); [ANI]'s damage lands as the press step begins, before any press. (Tests drawing_out_the_beast_damages_in_the_press_step_unless_combat_ended, drawing_out_the_beast_is_not_played_against_an_ally.)

**70.17** The Ankara Citadel, Turkey: "non-action modifier cards" — every card not played as an action modifier. Owner (2026-10-14): a dual Action Modifier/Reaction card played as a reaction is halved (it does not count as a modifier then), and an X cost is halved once X is chosen. A dual card played by the acting bearer outside combat is a modifier, not halved. (Tests ankara_citadel_halves_its_bearers_non_modifier_costs, ankara_citadel_halves_a_dual_card_played_as_a_reaction, ankara_citadel_halves_an_x_cost.)

**70.18** Third Tradition: Progeny: "cannot perform actions this turn" — as a recruited ally; the blood moved over its capacity drains. (Test third_tradition_progeny_moves_blood_decided_at_resolution.)

**70.19** Infernal Servitor: "if you control no infernal vampires" — one in torpor counts; read as each step settles; "after it performs a successful action" — a fizzled action counts (rules-qa 56.1), removed as the action ends. (Test infernal_servitor_leaves_after_success_or_without_an_infernal.)

**70.20** Zaire River Ferry: "this Lasombra" — the bearer must be Lasombra for either ability (rules-qa 9.1); "a vampire can have only one haven": the only haven in the pool, and unique. (Tests zaire_river_ferry_fails_an_action_at_its_lasombra, zaire_river_ferry_gives_its_lasombra_stealth, zaire_river_ferry_needs_a_lasombra_bearer.)

**70.21** Malabranca: "if your prey has more pool than you" read as the action is announced and again as it resolves (it fizzles otherwise). (Test malabranca_takes_a_pool_from_a_richer_prey.)

**70.22** Ian Carfax: his unlock is offered only while he is locked, after a combat he was in whose opposing minion is not ready. (Test ian_carfax_unlocks_after_a_combat_his_opponent_left.)

## Batch 71 — Legacy cards for TWDA decks, round 11 (the 2025–2026 decks entering the corpus): readings no ruling settles (Claude, 2026-10-14; owner, 2026-10-14: 71.1–71.20, 71.22–71.27 confirmed, 71.21 corrected, 71.25's open part ruled)

Slice 16w: the cards that bring the most 2025–2026 TWDA decks into the kept corpus. Only the readings that no ruling or default reading settles; each is implemented and tested, for the owner to confirm or correct.

**71.1** Mesu Bedshet: "if it is younger" — younger than the acting vampire; "draw it" to the uncontrolled region, with or without the 2 blood; another clan's card shuffled back with the whole crypt. (Test mesu_bedshet_draws_a_follower_of_set_with_blood_if_younger.)

**71.2** Kindred Segregation: each Methuselah in turn, clockwise from the caller, keeps allies one at a time until they pass; a keep may cost a last pool (it ousts); the kept allies' costs are paid, then every other ally burns, in torpor too; an ally with no card (a token) costs 0. (Tests kindred_segregation_keeps_allies_for_their_pool_cost, kindred_segregation_is_callable_without_allies.)

**71.3** Legendary Vampire: "entered your ready region during your last influence phase" — entered play between the end of the player's last minion phase and the end of their last influence phase (moved out of the uncontrolled region, or put in play then); a merged vampire keeps its entry (LSJ 20030519-2). (Test legendary_vampire_goes_on_a_vampire_from_the_last_influence_phase.)

**71.4** Propaganda of the Deed: the +2 votes are its controller's own, cast for or against, in any political action's polling step; the directed action any of the Anarch's. (Test propaganda_of_the_deed_enters_play_then_burns_for_two_votes.)

**71.5** Deer Rifle: two maneuvers each combat per copy; the second commits the strike to the same gun again. (Test deer_rifle_maneuvers_twice_a_combat.)

**71.6** Groundfighting: "a combat card … that would restrict this anarch's choice of strikes this round" — in the pool, a grapple leaving only hand strikes (Immortal Grapple) and Drawing Out the Beast against an armed anarch (LSJ 20050221); the canceled card's cost is paid (Groundfighting does not say otherwise); its maneuver, press and cancel are three plays of one card. (Tests groundfighting_cancels_a_grapple_for_a_blood, groundfighting_cancels_an_equipment_bar_only_with_a_weapon, groundfighting_does_not_cancel_a_weapon_steal, groundfighting_does_not_cancel_a_range_setting.)

**71.7** The Status Perfectus: "a blocking anarch" is the player's (32.7), and so is "another ready anarch" of the master phase move (once each master phase); the second combat begins as the first ends, the acting minion acting first, then back to the after-resolution window. (Tests status_perfectus_moves_a_blood_between_anarchs_in_the_master_phase, status_perfectus_sends_a_second_anarch_the_acting_minion_cannot_strike.)

**71.8** Josef von Bauren: in torpor too (no "if ready"); the Methuselah discarding draws back up. (Test josef_von_bauren_discards_at_random_per_discard_phase_action.)

**71.9** Petra Resonance: a vampire's cost is its capacity; the steal takes what the lowest has, up to 3 (an oust at 0); a crypt emptied since it was played reveals nothing. (Test petra_resonance_steals_for_the_highest_or_taxes_a_tie.)

**71.10** Waiting Game: a counter as a non-Anarch vampire's action is announced, any Methuselah's — an action card canceled as it is played is no action (ANK 20210124); "during your unlock phase" its controller's; "each anarch", "each ally" every Methuselah's, each anarch's controller choosing; one with no blood becomes Camarilla, its title kept; an ally at 0 life burns. (Tests waiting_game_counts_down_non_anarch_actions, waiting_game_bleeds_anarchs_or_makes_them_camarilla, waiting_game_does_not_count_an_action_card_canceled_as_played.)

**71.11** Mental Maze: "Followers of Set" is the Ministry (the clan's V5 name), the reacting vampire's clan; the acting minion stays locked (no unlock printed, unlike Obedience); [OBF]'s blocker is as it was before the block. (Tests mental_maze_ends_the_action_its_blocker_locked_or_not, mental_maze_is_not_played_against_an_older_vampire.)

**71.12** Maabara: locked at any impulse (as The Erciyes Fragments), several cards may be on it; the burn is an action directed at its controller; cards on it go to the ash heap with it. (Test maabara_keeps_an_ash_card_for_the_library_top.)

**71.13** Gabrielle di Righetti: "a ready minion" — any Methuselah's, undirected on her own; an ally robbed of its last life burns. (Test gabrielle_steals_an_allys_last_life_and_burns_it.)

**71.14** An Anarch Manifesto: "actions that require an anarch" — an action card or granted action requiring an Anarch (as CrimethInc.'s); stealth only while its bearer is Anarch; the strength to a titled non-Anarch vampire in combat with the bearer, read live. (Tests anarch_manifesto_stealths_actions_requiring_an_anarch, anarch_manifesto_strengthens_titled_non_anarch_foes, anarch_manifesto_gives_an_ally_nothing.)

**71.15** Free States Rant: X read as the terms are chosen; "ready vampires" anyone's, the caller's included; the blood burned in torpor too as it passes. (Tests free_states_rant_allocates_half_capacity_three_at_most_each, free_states_rant_needs_room_for_every_point.)

**71.16** Fortschritt Library: the library shuffled after the search; "only one … per game" anyone's. (Test fortschritt_library_finds_a_gehenna_event_once_a_game.)

**71.17** Harold Tanner: the hand of the controller of the minion in combat with him is open to every Methuselah for that combat. (Test harold_tanner_opens_his_opponents_hand_in_combat.)

**71.18** Giotto Verducci: "during another Methuselah's discard phase" — in that phase's rotation, after its Methuselah passes; once in each such phase; the card replaced at once. (Tests giotto_discards_in_others_discard_phases_and_tolls_vampire_blockers, giotto_tolls_vampire_blockers_only.)

**71.19** Leandro: the choice is the unlocking Methuselah's, both offered (a pool burn may oust); "lose all transfers" — none in the next influence phase, bonus transfers included. (Tests leandro_makes_others_burn_a_pool_or_lose_their_transfers, leandro_leaves_other_blood_for_the_uncontrolled.)

**71.20** Nose of the Hound: [spi] "an additional optional maneuver" — two in that combat; [SPI] any ready minion. (Test nose_of_the_hound_needs_a_locked_minion_below_superior_spiritus.)

**71.21** Taunt the Caged Beast [ANI]: both vampires chosen as it is played, the prey's then the predator's; no combat if either is no longer ready as it resolves; the 4 blood up to capacity. Owner (2026-10-14): it targets both vampires — directed at them and their controllers, both of whom may block (`ActionCtx::second_target`, `directed_seats`). (Tests taunt_the_caged_beast_pits_prey_against_predator, taunt_the_caged_beast_needs_a_predator_besides_the_prey.)

**71.22** Big Game: "unlocks at the end of the action" — as it ends, after all its combats, if still ready; a blocked action does not unlock it (the card's effects are its resolution's); Red List read on the printed card. (Test big_game_unlocks_after_a_red_list_hunt.)

**71.23** Kaymakli Fragment: "an opposing minion can strike: steal this equipment" — the strike of the minion in combat with its bearer, at close range, dodgeable, as Fast Hands' steal. (Test kaymakli_fragment_is_stolen_by_a_strike.)

**71.24** Nergal (advanced): "reaction cards cost +1 blood" — any played by another minion while he acts (a dual card only as a reaction, PIB 20150924-2); his stealth burn repeatable, when needed. (Tests nergal_advanced_taxes_reactions_while_he_acts, nergal_advanced_burns_blood_for_stealth_when_needed.)

**71.25** Perfect Clarity: "do not affect this vampire" — no reaction requiring Dominate or Presence may be played during the action (each would affect it: LSJ 20010621), except in a referendum's polling step; [THA]: the minion opposing it in any combat of the action. Owner (2026-10-14): in the polling step, a reaction card requiring Dominate or Presence played by another vampire does not affect it either — Scalpel Tongue may not choose it (another vampire who cast votes still), and Absolute Tyranny's [POT][PRE] −1 vote does not apply to it (it applies to the others); the vampire's own dual card is an action modifier, not shielded. The owner notes no scenario plays Scalpel Tongue before Perfect Clarity (the one only in the polling step, the other as the action is announced): nothing already chosen is undone. (Tests perfect_clarity_shields_a_bleed_from_deflection, perfect_clarity_superior_bars_presence_from_its_opponent, perfect_clarity_keeps_scalpel_tongue_off_its_vampire, perfect_clarity_spares_its_vampire_absolute_tyrannys_minus_one.)

**71.26** Touch of Clarity: "a non-combat card" — a card that is not a combat card; a reacting vampire may cancel an action card as it is played, before its action is declared. (Tests touch_of_clarity_cancels_a_presence_action_card, touch_of_clarity_by_the_acting_methuselahs_other_vampire.)

**71.27** Venere Carboni: once in each of his controller's unlock phases. (Test venere_carboni_leaves_torpor_for_a_blood.)

## Batch 72 — Card hunts: what reads "hunting" (Claude, 2026-10-14; owner, 2026-10-14: 72.1–72.5 confirmed)

**72.1** A card hunt (an action a card makes a hunt: Loki's Gift, Psychophagia, Abactor, Week of Nightmares' Ravnos hunt) is a hunt for everything reading one (LSJ 20050720-2): Volo's and Kalyani's "-1 stealth when hunting", Le Dinh Tho's "-1 stealth when hunting", Abiku's "+1 stealth when hunting" apply to it — Volo playing Loki's Gift [pro] ("+1 stealth hunt action") hunts at 0 stealth. (Tests volo_gets_less_stealth_on_a_card_hunt_too, abiku_gets_more_stealth_on_a_card_hunt_too.)

**72.2** Zöe: "does not get the usual +1 stealth when hunting" — the usual +1 is the rulebook hunt's own; a card hunt has none to remove, its stealth is the card's: Zöe playing Loki's Gift or Abactor hunts at the card's +1. (Test zoe_keeps_a_card_hunts_own_stealth.)

**72.3** Sunset Strip, Hollywood: "+1 stealth on hunt actions" applies to a card hunt (Abactor, Week of Nightmares' Ravnos hunt). (Test sunset_strip_gives_a_card_hunt_stealth.)

**72.4** Unlicensed Taxicab: "+1 stealth during hunt … actions" applies to a card hunt (Abactor), as 72.1. (Test unlicensed_taxicab_gives_stealth_during_a_card_hunt.)

**72.5** Black Annis: "other non-hunt actions cost her an additional blood" — a card hunt is a hunt (72.1): Loki's Gift costs her nothing more. (Test black_annis_pays_nothing_more_for_a_card_hunt.)

## Batch 73 — Learjet's extra draw, then the engine review's lints: readings no ruling settles; every legal block attempt offered (Claude, 2026-10-14; owner, 2026-10-14: 73.1 confirmed, 73.2 answered, 73.3–73.8 ruled)

**73.1** Learjet: the extra draw comes with every delayed replacement of a card the bearer played as the acting minion, whatever releases it (PIB 20110718 generalized): at the next unlock phase (Old Friends), at the next discard phase (Mirror Walk), as Blood Weakens' held replacements are drawn when the hand empties, at the end of the action; a replacement The Meddling of Semsith holds again during the minion phase brings its extra draw at the minion phase's end, with it. Read: the draw is keyed to the replacement, so it is offered even if Learjet or its bearer left play meanwhile. (Tests learjet_offers_its_extra_card_with_a_replacement_held_until_the_unlock_phase, learjet_offers_its_extra_card_with_a_replacement_held_until_the_discard_phase, learjet_offers_the_extra_cards_of_replacements_drawn_as_the_hand_empties, learjet_extra_card_waits_with_a_replacement_held_again_by_the_meddling.)

**73.2** Learjet: the action card its bearer plays to take an action brings no extra draw. Owner (2026-10-14): "no, you don't redraw on the action card itself: … replacing the action card is not done during the action, but before." (Test learjet_offers_no_extra_card_for_the_action_card_itself; findings L12.)

**73.3** Powerbase: Madrid: "Vampires controlled by other Methuselahs can burn all the counters from this card as a Ⓓ action" — offered with no counter. Owner (2026-10-14): "legal no-op. On the interface side, maybe hide when opportunities are off. But engine side it's legal" (17.4, 25.5; findings L3, P15).

**73.4** Alamut, Dead Pool: a vampire abstaining from the referendum can still burn counters for votes it cannot cast. Owner (2026-10-14): "legal no-op, same thing" (73.3; findings L4).

**73.5** "Burn n blood or life" on a card's host (Phantasmagoria [PRE], Sculpt the Flesh [PRO]) and a vampire's "burns 1 blood" (Marialena, Spleen, Condemnation: Mute [DAI]) are one effect: blood for a vampire, life for an ally. Owner (2026-10-14): "An ally at zero life is always burned (it needs at least one life to stay alive). The other 3 ride on a vampire so can only burn blood (vampires don't have life). … I think you could use a single "HostBurnsBloodOrLife" for all." As an in-play card's ability cost, "burn 1 blood" stays a blood cost an ally never pays (advanced rules 5.5.1). (Test a_blocked_host_burns_blood_or_its_last_life; findings L7.)

**73.6** An ally playing cards "as a vampire" (Spectral Servitor, Herald of Topheth, The Vozhd of Juiz de Fora, Infernal Servitor) is an Independent vampire of its granted capacity for the card it so plays only. Owner (2026-10-14): "No, it cannot be offered to an ally like Spectral Servitor: it is seen as a vampire only when playing [obl] cards, and only by the card it plays - the powerbase never sees it as a vampire." Powerbase: Munich's burn ("Independent vampires can burn this location") is never the Servitor's. (Test powerbase_munich_never_sees_an_ally_playing_as_a_vampire; findings L8.)

**73.7** Zizi: "if /Viri/ is in your ready region"; Viri: "while /Zizi/ is in your ready region" — the controller's ready region, not any Methuselah's. Owner (2026-10-14): "no, "your" means the controller's ready region (card text "you" always addresses the controller), not any methuselah's" (32.7). Rurik Rakoczy's "while /Ludmijla Rakoczy/ is ready" and Valkyrie's "unless Brunhilde is ready" name no region: any Methuselah's. (Test zizi_needs_viri_in_her_own_ready_region; findings L2.)

**73.8** Every legal block attempt is offered, one that cannot reach the action's stealth included: a minion that may attempt to block is offered the attempt whatever intercept it could still gain. Owner (2026-10-14): "the engine should allow all legal plays even if they're not worth considering overall" (17.4). Supersedes the engine's pruning of 48.5 and 66.2: their reading of what may still reach the stealth stays the host's, which hides an attempt short of it with the opportunities off. A failed attempt is still not offered again while nothing changed (48.6). (Tests a_futile_block_attempt_is_offered_and_fails, a_futile_block_attempt_is_hidden_with_the_opportunities_off.)
