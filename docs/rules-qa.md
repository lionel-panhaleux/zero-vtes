# Rules Q&A — ruling reference

Owner rulings (VEKN judge) on VTES rules questions raised while building the
engine, in batches. Each item is the settled rule, stated as fact; the
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
  choosing its limit: "during" always means once; an ability without it
  is not limited by it (Maris Streck). (6.6, 15.2, 42.3, 58.7, 79.1, 79.3)
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
- **Legacy additions for TWDA decks** — 44.1–44.10, 57.1–57.10, 58.1–58.20, 59.1–59.19, 60.1–60.22, 62.2–62.31, 62.33, 63.1–63.17, 64.1–64.12, 65.1–65.3, 67.1–67.7, 68.1–68.17, 69.1–69.15, 70.1–70.22, 71.1–71.27, 74.1–74.13, 76.1–76.25, 77.1–77.35, 78.1–78.49, 78.54, 79.3, 80.1–80.45, 84.1–84.40, 85.1–85.34
- **Ousts, Life Boon's decision point** — 45.1–45.8
- **Impulse, sequencing, windows** — 1.2, 1.3, 1.5, 2.1, 2.2, 2.3, 2.4, 3.7, 4.1, 5.3, 7.7, 7.8, 7.9, 23.3, 24.2, 26.5, 66.1
- **Mandatory effects and their ordering** — 6.1, 6.4, 7.8, 7.9, 17.2, 27.1, 28.9, 32.10, 40.5
- **Periodicity ("during X" = once per X, once each …)** — 6.6, 7.4, 9.6, 10.4, 11.2, 15.1, 15.2, 16.1, 36.3, 36.4, 37.1, 38.8, 42.3, 42.9, 79.1–79.4
- **No-repeat-action, "same action"** — 1.4, 6.2, 16.6, 23.2, 25.4, 32.1
- **Action outcomes: fizzle, fail, success, after resolution** — 3.6, 5.8, 5.9, 17.1, 17.3, 17.4, 23.3, 24.1, 24.4, 26.1, 26.4, 28.7, 30.2, 30.3, 34.6, 42.6, 43.1, 55.3, 55.4, 55.5, 56.1, 58.1, 58.13, 73.3, 73.4, 77.13, 78.13, 78.38, 78.50
- **Directed / undirected, targeting, choosing** — 6.8, 17.2, 17.4, 20.1, 24.5, 26.3, 27.2, 32.5, 33.3, 34.8, 35.1, 38.2, 38.6, 39.4, 42.1, 57.10, 75.1, 77.12, 77.26, 77.27, 78.30, 78.36, 78.52
- **When choices are made (play vs resolution vs use)** — 3.5, 6.7, 22.2, 26.8, 28.5, 31.2, 31.3, 32.11, 32.12, 33.4, 38.1
- **Bleeds** — 18.4, 23.5, 24.1–24.4, 25.6, 30.5, 31.2, 32.2, 32.4, 32.8, 33.9, 34.4, 34.5, 39.5, 42.3, 42.7, 53.6, 53.8, 57.1, 57.2, 61.2, 76.12
- **Stealth, intercept, blocks** — 1.3, 1.6, 3.2, 3.4, 8.1, 9.6, 15.2, 23.1, 24.5, 25.3, 25.5, 27.3, 30.1, 30.3, 30.4, 23.6, 34.1, 34.2, 35.1, 42.4, 42.8, 48.1, 48.3, 48.5, 48.6, 49.5, 50.4, 52.8, 53.1, 53.2, 53.3, 55.7, 66.2, 72.1–72.4, 73.8, 76.17, 77.18, 77.19, 77.20, 78.20, 78.28, 78.37, 78.53, 82.1, 82.2
- **Combat structure, strikes, maneuvers, presses** — 1.2, 4.1, 4.3, 4.4, 4.5, 10.1, 10.2, 10.4, 10.5, 14.1, 14.2, 19.5, 21.1, 22.1, 22.3, 26.2, 26.6, 26.7, 27.5, 29.7, 29.8, 29.9, 31.7, 33.6, 42.2, 47.2, 48.2, 48.4, 53.3, 54.4, 54.6, 54.7, 54.8, 55.1, 55.2, 55.8, 57.3, 57.4, 58.2, 58.17, 58.18, 59.2, 59.17, 60.11, 61.1, 62.1, 63.4, 63.6, 63.8, 75.1–75.3, 76.9, 76.10, 76.11, 77.9, 78.23, 78.32, 78.40
- **Frenzy cards** — 28.11, 29.5
- **Damage, prevention, immunity** — 4.2, 10.3, 14.2, 21.3, 29.1, 29.2, 29.3, 29.4, 30.6, 38.3, 38.8, 40.9, 77.8, 77.21, 77.22, 77.23, 78.19, 78.33, 78.34, 78.38
- **Rushes (entering combat as an action)** — 17.3, 26.1, 27.2, 27.5, 38.6, 52.6, 54.3, 54.4, 54.7, 55.7, 56.2, 58.6
- **Referendums, votes, political actions** — 1.1, 3.5, 5.1–5.9, 6.2, 6.3, 15.2, 16.5, 19.3, 19.4, 32.9, 33.2, 35.4, 38.6, 39.1–39.4, 42.4, 49.1, 49.2, 50.5, 50.6, 51.1, 51.2, 51.5, 51.6, 51.8, 52.2, 52.3, 52.5, 52.7, 52.9, 52.10, 54.1, 58.8, 58.14, 58.15, 58.16, 59.7, 59.9, 59.18, 59.19, 60.2, 60.14, 60.16, 71.25, 73.4, 76.5, 76.6, 76.14, 76.15, 77.5, 77.14, 77.15, 77.25, 77.26
- **Titles** — 3.1, 13.2, 19.1, 19.2, 19.6, 32.9, 49.4, 52.5, 52.6, 52.9, 58.3, 58.11, 59.13, 59.14, 60.1, 76.1–76.4, 77.1, 77.2, 78.1, 78.7, 78.16
- **Contests, uniqueness** — 3.1, 7.1, 7.3, 7.5, 8.2, 11.2, 12.3, 19.1, 36.3, 38.7, 39.4, 40.1, 57.9, 58.5, 58.19, 59.13, 60.1, 60.9, 60.20, 76.2, 76.3, 85.32
- **Control, steals, ousts, removal from game** — 6.9, 7.5, 17.4, 25.1, 27.3, 27.4, 27.7, 28.13, 31.4, 31.5, 31.6, 33.5, 33.7, 38.4, 39.4, 42.7, 55.4, 55.6
- **Costs and reductions** — 3.6, 5.9, 9.3, 18.1, 18.2, 23.4, 29.6, 27.6, 36.1–36.5, 38.5, 42.1, 42.2, 42.5, 42.6, 54.1, 58.12, 59.5, 60.5, 60.7, 77.24, 78.3, 78.17, 78.21
- **Replacement, draws, stored and held cards** — 5.5, 7.10, 8.2, 8.3, 8.4, 9.4, 9.5, 11.1–11.3, 12.1–12.5, 22.2, 37.1, 40.4, 53.4, 53.5, 73.1, 73.2, 77.35, 78.18, 78.26, 78.51
- **Card destination (ash heap, limbo, lingering effects)** — 21.1, 22.1, 31.6, 46.4, 47.1
- **Equipment and weapons** — 9.1–9.6, 10.1–10.5, 17.4, 18.3, 22.4, 29.7, 35.3, 38.4, 38.5, 38.7, 76.9, 76.10, 76.15, 76.16, 77.19, 77.29, 78.28, 78.42
- **Allies, retainers, ghouls** — 22.2, 28.10, 29.2, 32.4, 32.11, 32.12, 32.13, 37.1, 40.10, 42.2, 42.9, 49.3, 50.6, 73.5, 73.6, 76.7, 76.8, 77.29
- **Locations, hunting grounds, locquipment** — 6.5, 7.2, 7.4, 17.4, 28.2, 28.6, 31.4, 34.7, 39.4, 49.5, 50.4, 51.4, 52.7, 54.1, 57.7, 73.3, 73.6, 77.16
- **Blood, capacity, uncontrolled region, banishment** — 5.7, 7.6, 25.1, 27.7, 28.1, 28.3, 28.4, 32.6, 40.7, 73.5
- **Torpor, rescue, diablerie, blood hunt** — 3.1, 3.6, 5.2, 5.8, 6.4, 16.2, 28.7, 38.2, 48.8, 81.1, 86.1
- **Hunting** — 6.5, 7.7, 17.3, 28.1, 28.8, 32.13, 40.2, 53.8, 72.1–72.5, 77.30
- **Master phase, master cards, out-of-turn** — 1.5, 3.3, 6.6, 7.5, 28.10, 32.3, 33.1, 33.8, 36.4, 49.6, 49.7, 50.1–50.3, 53.7, 57.8, 77.11, 77.27, 78.10, 78.48, 78.49
- **Unlock phase, locking, stun** — 3.7, 6.1, 7.8, 16.5, 21.2, 25.5, 26.5, 31.1, 48.7, 54.2, 54.5, 58.4, 58.9, 59.3, 59.4, 60.6
- **Discard phase, events** — 8.4, 15.2, 28.12, 33.8, 40.1–40.11, 54.5, 57.5, 57.6, 58.10
- **Conditions read live; card-text defaults** — 4.3, 9.1, 13.2, 18.4, 19.3, 28.2, 28.6, 29.11, 32.4, 32.7, 39.5, 49.5, 49.7, 73.7, 77.6, 77.13
- **Corruption counters, multi-Discipline cards** — 25.1–25.6, 34.3, 35.2, 38.4, 52.1, 77.28
- **Printed vampire abilities** — 13.1, 13.2, 14.1, 14.2, 15.1, 15.2, 16.1–16.6, 17.1–17.4, 41.1–41.14, 49.2, 49.3, 52.4, 72.1, 72.2, 72.5, 73.5, 73.7, 79.2, 79.3
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

**16.6** Evan Klein's tails: the acting minion still locks, the action counts for NRA and the bleed/political caps, cost and blocks are skipped, as-announced plays remain possible before the flip. Alabástrom: once per turn, any other ready Gargoyle; Flight is a bare trait for now (revised by 84.37: generated, read).

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

**48.2** Additional strike cards are played one at a time: a combatant with an additional strike still to make gains no other until that pair resolves; after each additional pair, a new chance to play one (owner, 2026-10-03, from play: Quickness offered right after Pursuit [CEL]; "you first have to resolve the strikes pair from the Pursuit before you get the opportunity to get yet another additional strike by playing Quickness"). Refines 47.2; the opponent may still answer in the same window (rulebook "Additional Strikes": the acting minion decides first, then the opposing one). A card granting several strikes (Blur) bars further cards until all of them are made (owner, 2026-10-20).

**48.3** A change of blocker: during its own minion's block attempt, while that attempt is short of the action's stealth, a Methuselah may attempt with another of its minions instead — by declaring that one's attempt, or with a card whose unlocked minion attempts to block (Organized Resistance's second mode: "that Anarch attempts to block", so it must be the one blocking). The attempt under way fails then, with a failed attempt's consequences (owner, 2026-10-03, from play: an attempt left short by the acting minion's stealth could not be changed). Refines the "no re-declare" note (the attempt persists until closure unless its Methuselah changes blocker); a forced attempt on the minion already attempting keeps it (ANK 20181122-2). The switch takes effect at once, with no impulse left to others for the abandoned attempt (owner, 2026-10-20).

**48.4** Slam [POT] ("as above, with 1 optional maneuver") is played only at the maneuver, never as the strike: at strike declaration it would be [pot] for 1 blood, its maneuver unusable (owner, 2026-10-04, from play: "superior gives a maneuver so it's only playable during the maneuver step"). The only card of the shape — a mode identical to another but for an optional maneuver; Aid from Bats and Diversion carry their maneuver on their only strike mode and stay playable as the strike (2.5).

**48.5** An attempt to block is offered whenever the would-be blocker's Methuselah holds an intercept card it could play and has a library to draw from: the card's replacement may be another (owner, 2026-10-04, from play: an action at 2 stealth, one intercept card in hand, "hope to draw the second"). Refines the reachability pruning (DESIGN §4.3): it counts the hand alone only when nothing can be drawn. Superseded by 73.8 for the engine (every legal attempt offered); the host's opportunities filter keeps the reading.

**48.6** A minion whose block attempt failed is not offered the same attempt again while nothing has changed: its Methuselah's hand, the stealth, its intercept, blood and lock, the cards in play, every hand's size (owner, 2026-10-04, from play: a bot attempting again and again with not enough intercept — "legal … but it still probably should not retry if the situation has not changed"). An engine reading, not a rule: the repeat is legal and would fail alike. Another minion's attempt, or the same minion's once something changed, is offered as before. An attempt left for another minion's (48.3) failed alike: swapping back is not offered either (owner, 2026-10-04, from play: two bot minions trading a failing attempt back and forth).

**48.7** An ability whose only effect is to unlock its own minion is not offered while that minion is unlocked (owner, 2026-10-04, from play: Rutor's Hand offered to unlock an unlocked vampire — "legal, but really useless, no card cycled or anything"). An exception to 17.4's legal no-op plays: locking the card or spending a once-each-turn use buys nothing. Rutor's Hand, Valerie Diaz, Sword of the Archangel; the vampires' unlock abilities required it already.

**48.8** Rescue's cost: who pays it (the acting vampire, the rescued one, or a split) is chosen as the action is announced; the blood is paid as it resolves (3.6) (owner, 2026-10-04, from play: "it should ask when the action is declared (who pays)"). The cost is read again at resolution (live conditions). The split stands: a share its payer can no longer meet at resolution fizzles the action, the other paying nothing in its place (owner, 2026-10-20; read first as falling to the other); a vampire gone from play pays nothing (28.7). (Test rescue_cost_split_is_chosen_as_announced_and_paid_at_resolution.)

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

## Batch 62 — Legacy cards for TWDA decks, round 2 (steps 1–6), implemented readings (2026-10-11; owner, 2026-10-20: all confirmed)

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

**62.14** Tupdog: his rush at another Methuselah's minion (17.3); "Tremere antitribu slave" (rulebook "Slave"): no directed action while his controller controls no ready Tremere antitribu (read as it is offered); as a Tremere antitribu of his controller's is blocked, he may lock to fight the blocker instead — the acting vampire unlocked, the canceled combat's riders gone (advanced rules §5.7.2); after the normal end of his controller's minion phase, every Tupdog of theirs burns (in torpor too), each moving one crypt card to the uncontrolled region (an empty crypt: none). No Visceratika card is in the pool; Flight stays a bare trait (16.6; revised by 84.37).

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

**62.28** Sense the Sin: [aus] against a younger acting vampire; [dai]'s extra +1 if the acting minion has anyone's corruption counter; [DAI]'s -1 intercept to every minion with any corruption counter, any Methuselah's, read live during the action (owner, 2026-10-20: corruption counters only).

**62.29** The Unmasking: every Methuselah's allies, +1 intercept while the acting minion is a vampire; replaced as its controller's next turn's discard phase begins.

**62.30** Veneficti: +1 stealth; not the target of a directed action of an ally or of a vampire of capacity 6 or less (read as it is offered); he steals a demon ally — Herald of Topheth, added for it (owner, 2026-10-11) —; no demon retainer is in the pool; his burn option per his printed clan, Baali — any of the pool's Baali is infernal.

**62.31** 41.8's "no mage ally in the pool" no longer holds: Sri Sansa moves blood or life with Nephandus and Veneficti too.

**62.32** "Once each turn" on an in-play ability its controller uses (Ferraille, CrimethInc. [tha]) is now enforced across the turn, not only within one referendum (a fix seen in the random games). One use for the card, whichever mode: Ferraille's votes for or against, Dabbler's unlock or blood (owner, 2026-10-20). (Tests ferraille_votes_once_a_turn_for_or_against, dabbler_rewards_once_each_turn.)

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

**63.16** Gregory Winter: his steal at a vampire with no blood is legal and steals nothing (RTR 20010711); his torpor burn raises his life over his starting life; his two actions do not restrict each other (LSJ 20100909); his unlock-phase life burn can burn him. Aaron's Feeding Razor: +1 to every hunt of its bearer, mandatory. Abombwe: any Methuselah's vampire with Protean, or a Laibon (77.34); put on by Childe of the Revolution's search, its requirement ignored (LSJ 20051116-2). Lisette Vizquel: +1 intercept as she attempts to block a diablerie action. Wise Frog: first strike in a combat with a vampire younger than her, read as the strikes resolve. Cameron and Hester Reed have no text.

**63.17** Seen while testing: an action card's "+1 stealth if this vampire is Ravnos" (Baltimore Purge) is read as the action is announced and skipped at resolution (it crashed the resolution); Focus the Blood's discount was offered but not taken on a combat card's blood cost (now taken, the card burned as the cost is paid); Renewed Vigor's "add enough life to reach their starting life" overflowed its counter.

## Batch 64 — Legacy cards for TWDA decks, rounds 4–5 (the Gangrel decks), implemented readings (2026-10-11; owner, 2026-10-14: 64.7, 64.8 confirmed; 2026-10-20: the rest, 64.6 restated)

**64.1** Ariadne: her "+1 hunt" adds to every hunt of hers, non-standard ones too (RTR 20030519); no blood hunt is ever called on her (her diablerie, an Abactor). Jacob Fermor: "werewolves you control" are allies of the Werewolf subtype (Renegade Garou, Ossian), +1 bleed while Jacob is ready (in torpor: none), read live; his press once each combat. Gunnar, Lillian and Robert Price print no text.

**64.2** Anachronism: on any Methuselah's vampire of capacity 7 or more; "ranged weapons" are the weapons with a ranged strike (LSJ 20060825), Zip Gun included, the Ivory Bow excepted; those on it burn as it enters; it cannot equip one (from hand, by an action or another card), move one to it, nor use one, nor play Zip Gun.

**64.3** Bone Spur: an untimed combat card — before the strike it should affect resolves, after both strikes are chosen too (RTR 19960112); [pro] this round's hand damage, [PRO] every later round's of the combat as well.

**64.4** The Kiss of Ra: against a block attempt by a vampire without Fortitude (the Fury of Unleash Hell's Fury included: LSJ 20100210); the blocker burns 2 blood (as much as it has), [FOR] then goes to torpor; the action is canceled before resolution: nothing more is played in it (RTR 20180719), no outcome — no after-resolution window, so no Daring the Dawn damage —, no torpor-block diablerie (LSJ 19970325), no NRA mark (the same action may be taken again, unless it was blocked then continued before: ANK 20211015), still taken (the optional action, Veil the Legions' count: ANK 20210124); the acting minion stays locked.

**64.5** Mind of the Wilds: [ani][aus] in the action's own window, [ANI][AUS]'s +1 stealth only when needed (3.2); the maneuver banked for the first round of the block's combat only; the blocker barred from "strike: combat ends" for the rest of the action, any round of any combat (LSJ 20100308).

**64.6** Ossian: his rush at any vampire, any Methuselah's; the freeze read as cards are played: a vampire opposing him that played a card requiring Auspex in the current action (any action, LSJ 20050201; the Discipline the card was played with) uses no maneuver nor strike for the rest of the first round — from the combat's start, or from that card on mid-round (an Auspex strike played first stands: ANK 20210614); it may still play an effect setting the range that is not a maneuver, not a Sniper Rifle (a strike commitment, LSJ 20100827), the range set once (owner, 2026-10-20). His life: in his card's words, the blood the opposing vampire used that round to heal damage or to prevent destruction (owner, 2026-10-20), at the end of the round, after the presses, the round ended early too (RTR 20030519), over his starting life (ANK 20171109). "Red List" is a bare trait.

**64.7** The Path of Lilith: offered as the torpor lands — the opposing vampire sent to torpor in combat (not burned), the vampire opposing it non-Camarilla and the card's controller's, the card unlocked; its controller's unlock phase: each Methuselah controlling a vampire with a torture counter (ready or in torpor), its controller included, burns 1 pool. Every torture counter goes once no Path of Lilith is in play (one held by another Methuselah keeps them all: none other in the pool).

**64.8** Predator's Transformation: the reflex by the vampire the frenzy card is played on (rules-qa 28.11), locked or not, its cost still paid (ANK 20260216); [abo] +1 stealth as the action is announced, even if not needed; [ABO] at the end of the round in which this vampire's combat burned a minion of its controller's prey, or after the resolution of its directed action that burned one out of combat (ANK 20220507), once per action. That a locked vampire may play a reflex is a reading, confirmed by the owner (2026-10-14).

**64.9** Read the Winds: replaced after the action (Blood Weakens' rules aside); [ani][aus] keeps its vampire unlocked after a successful block; [ANI][AUS] prints no "as above": its vampire locks as usual; the usual blocking restrictions apply (ANK 20180623, LSJ 20010714-2).

**64.10** Zip Gun: before range is determined in any round, put on the minion; in play it is weapon equipment — a target for "equipment" effects (Canine Horde) as 22.4 has it (this extended 22.4 to card targets; five random opening games changed) — and a gun (Kevlar Vest); 1R damage, its maneuver committing the strike (LSJ 20021028); its bearer takes 1 environmental damage the first time it strikes with it each combat, none if the gun is gone first (LSJ 19970801, LSJ 20100310). No ammo card is in the pool.

**64.11** Generators: the window matrix reads "when … attempts to block" (The Kiss of Ra), "when this vampire burns a minion" (Predator's Transformation [ABO]: no in-play action flag) and "with an optional maneuver each combat" (Zip Gun: a rider, no play window); "As above, with +1 stealth" over a mode with no printed timing keeps the stealth window alone (Mind of the Wilds [ANI][AUS], 3.2; Mask of a Thousand Faces keeps its timings); a mode's two type tags ([ACTION MODIFIER][COMBAT]) are read in a fixed order (the generator was nondeterministic on Predator's Transformation).

**64.12** Owner, 2026-10-11 (relayed by Z-play): as a political referendum's polling step opens, the other Methuselahs get a chance to play the cards usable "before any votes and ballots are cast" before the caller casts — Confusion of the Eye [OBF] (63.3), Fakir al Sidi (59.18), Closed Session. The engine opens it with a pre-vote rotation from the caller's left, the caller last, offering those cards (and their abilities) alone, no vote; once it closes, the polling proper rotates from the caller. Not for a blood hunt, nor a referendum passing automatically.

## Batch 65 — Legacy cards for TWDA decks, round 6, implemented readings (2026-10-11; owner, 2026-10-20: all confirmed)

**65.1** Psyche!: [CEL] in the end-of-round window as the combat would end, both combatants ready, no combat queued (RTR 20020501) — no "would end" replacement after it (Telepathic Tracking, Hunting the Quarry: ANK 20191219), the end-of-round cards still open before or after it (LSJ 20021113). The queued combat begins as this one ends, the same two minions, still in the action (LSJ 20021213) and ending as this one would have (into the after-combat window, or back to an action's step); this combat's after-combat effects are lost (a continuation as if unblocked, Torrent's included; damage or a card after combat), unlocks aside, applied as their strikes resolved (LSJ 19980109); cards held "until after combat" are replaced before it (LSJ 20001122); no block began it, so no block rider nor "when blocking" equipment carries over (Spirit's Touch, Sniper Rifle: LSJ 20010813, LSJ 20010814); a hand size the first combat lowered is drawn up between them and discarded down again (Raptor, LSJ 20030530).

**65.2** Neighborhood Watch Commander: any Methuselah's, after the resolution of any Methuselah's blocked or failed action whose acting minion is a vampire; ready and unlocked, it locks to inflict 1 normal damage out of combat (the vampire burns a blood, else goes to torpor); one Commander each action, any controller's.

**65.3** Tabitha Fisk: as Silvian Popescu — her controller's unlock phase, once, from an archbishop of theirs with blood, she in torpor too. Tatiana Romanov: +1 bleed. Delilah Easton and Isabel de Leon print no text.

## Batch 66 — Play feedback: the as-announced window, stealth a block can still reach (owner, 2026-10-12)

**66.1** The as-announced window holds only the effects whose text says so ("only usable as the action is announced", "as … announces", Seduction, Warsaw Station, Yoruba Shrine, Screamer's fail): an effect usable during the action (Screamer's and Aranthebes's -1 stealth, Haqim's Law: Retribution's +1 bleed) waits for the block attempts — the reacting Methuselah's with its blocks and unlocks, the acting one's with its other modifiers (owner, from play).

**66.2** A block attempt is offered when the stealth may yet be met, and stealth an ability in play can still take off (Screamer, Aranthebes) counts as much as intercept: a woken, locked Monica Giovanni attempts against a 1-stealth bleed bounced back to her Methuselah, who controls Screamer (owner, from play; refines 1.3's pruning). Superseded by 73.8 for the engine; the host's opportunities filter keeps the reading.

## Batch 67 — Legacy cards for TWDA decks, round 7: readings no ruling settles (Claude, 2026-10-12; owner, 2026-10-20: 67.2, 67.3, 67.5, 67.7 confirmed)

Slice 16s: the next hundred cards by TWDA importance (ten new archetypes). Only the readings that no ruling or default reading settles; each is implemented and tested, for the owner to confirm or correct.

**67.1** The Erciyes Fragments (a latent bug the roster reshuffle surfaced): its controller ousted, another Methuselah's card stored on it goes to its owner's ash heap — as another's card on a removed minion does (owner, 2026-09-06) — not out of every zone. (`remove_seat`; test the_erciyes_fragments_card_returns_to_its_owner_when_its_controller_is_ousted.)

**67.2** Emerald Legionnaire: "if you control fewer copies of Emerald Legionnaire than ready unique Harbinger of Skulls" — read as the ready Harbingers of Skulls *you* control (crypt vampires), the ally ready itself (ANK 20180511-1). (Test emerald_legionnaire_raises_another_from_the_ash_heap.)

**67.3** Donatello Giovanni: "if the chosen vampire attempts to bleed before your next unlock phase, they move 1 blood to Donatello" — every bleed it announces until then, not the first only; the chosen vampire any Methuselah's but the predator's, his own included. (Test donatello_giovanni_taxes_the_chosen_bleeder.)

**67.4** Spiritual Guidance: "play a combat card from your ash heap as if from your hand" — owner (2026-10-12): it cannot be forgone: the card is played immediately, as the resolution of Spiritual Guidance, the impulse not going around; its requirements and cost as normal (the ash heap's combat cards the only options of its player's next decision; none playable, nothing happens). (`Engine::ash_play_due`; tests spiritual_guidance_plays_from_the_ash_heap, spiritual_guidance_forces_a_choice_among_the_ash_heaps_plays, spiritual_guidance_with_nothing_to_play_from_the_ash_heap.)

**67.5** The Red List (rulebook "Traits", Enkidu): the master phase action marks another Methuselah's Red List minion (one's own is not offered); the action to enter combat with it is a +1 stealth Ⓓ rulebook action, once per vampire per turn, its 1 blood paid at resolution (unpayable then: it fizzles, as other action costs, rules-qa 3.6); trophies are not modeled (none is in the pool). (Test the_red_list_marks_enkidu_for_a_rush.)

**67.6** New Blood: "choose any circle … New Blood belongs to the chosen circle" — not modeled, no card of the pool reading a circle (Unwholesome Bond and Angelo are not in the pool); the crypt draw as he enters is. (Test new_blood_draws_a_crypt_card_as_he_enters.) Superseded by 68.1 (slice 16t: circles modeled).

**67.7** Trochomancy: "remove thirteen (seven) cards in the target Methuselah's ash heap from the game to get +1 bleed" — the removal is the price: not playable unless that ash heap holds them (burned vampires counted, as Shroud of Decay's, rules-qa 42.7). (Test trochomancy_burns_the_targets_ash_heap_for_bleed.)

## Batch 68 — Blood Brother circles, legacy cards for TWDA decks, round 8: readings no ruling settles (Claude, 2026-10-12; owner, 2026-10-20: all confirmed, 68.12 corrected)

Slice 16t: the sixty cards of the ten Blood Brother TWDA decks. Only the readings that no ruling or default reading settles; each is implemented and tested, for the owner to confirm or correct. A vampire with no circle is in a circle of one (LSJ 20020609): `GameState::same_circle` needs two Blood Brothers of one printed (or chosen) circle.

**68.1** New Blood: "choose any circle" — owner (2026-10-12): a circle printed in the crypt (Mexico City, Torrance, Kiev, Chicago, Budapest), or his own circle of one (Pass), as he enters play (`Interrupt::ChooseCircle`); "the unique circle of one of any vampire" (ANK 20211113) read where a card reads it, Unwholesome Bond [SAN] on one circle-less uncontrolled Blood Brother (68.7). Supersedes 67.6. (Tests new_blood_chooses_a_circle_or_keeps_his_own, a_blood_brother_with_no_circle_is_in_a_circle_of_one, unwholesome_bond_lets_new_blood_feed_a_circle_of_one.)

**68.2** Slake the Thirst: "only usable after this Blood Brother gains blood" — each effect's gain is recorded where blood is added (`GameState::set_blood`); a gain stays usable while the real decisions stay in the window of the first real decision after it (the play's own terms and as-played window, an effect's choices as it resolves, aside), so a strike's gain as the strikes end; several gains: the one answered chosen by its amount (LSJ 20100203). Owner (2026-10-12): it is offered between a diablerie and its blood hunt (ANK 20201228) — a window of its own (`ActionState::AfterDiablerie`, `Window::ActionAfterDiablerie`). (Tests slake_the_thirst_shares_a_hunt, slake_the_thirst_again_on_the_same_gain, slake_the_thirst_answers_one_gain_of_several, slake_the_thirst_after_a_diablerie.)

**68.3** Hermana Hambrienta ("burns 2 blood or is burned"), Robert Carter ("Carter's employer burns 1 blood, or Carter is burned"): the controller's choice, as Society of Leopold's same wording; the Hermanas only as influenced out (after the excess drained, LSJ 20051126). (Tests a_hermana_burns_two_blood_as_she_enters_play, robert_carter_bleeds_for_blood.)

**68.4** Octopod, Three's a Crowd, Coordinate Attacks: "another one (you control) in combat" — its player's combatant, the card played by a minion out of the combat; Octopod's strikes each round come after the normal pair, as Renegade Garou's (mandatory, limited). (Tests octopod_gives_strikes_each_round, threes_a_crowd_closes_and_presses, coordinate_attacks_sends_the_next_one_in.)

**68.5** Shell Game: X = 0 is a legal no-op (17.4); "you may unlock one of those" — offered after a successful action, one (two at [SAN]) of the locked ones, its player's pick. (Test shell_game_locks_the_circle_for_stealth.)

**68.6** Coagulated Entity: the blood moved as the action resolves, before the combat; any chosen Blood Brother unable (no blood, gone) and the action is canceled: no combat. The +X capacity lasts until the action ends, the excess then drains. (Test coagulated_entity_feeds_the_rush.)

**68.7** Unwholesome Bond [SAN] for New Blood: one chosen uncontrolled Blood Brother with no printed circle (LSJ 20050927, ANK 20211113), a mode of its own; [san]'s X counts any Methuselah's ready unlocked Blood Brothers of the circle at resolution. (Tests unwholesome_bond_counts_the_circle, unwholesome_bond_lets_new_blood_feed_a_circle_of_one.)

**68.8** Sanguine Instruction: the target chosen first, then the Discipline — one the actor has at superior and the target not; a target with none of them is not offered; any Methuselah's vampire of the acting vampire's clan. (Test sanguine_instruction_teaches_a_discipline.)

**68.9** Dark Influences: as Direct Intervention, played on another Methuselah's turn; its shield is any Dark Influences in play, whoever controls it, against a cancel of a minion card of another Methuselah than the canceling one. (Test dark_influences_cancels_and_shields.)

**68.10** Jerry ("move any amount of blood from the Blood Brothers in the Chicago Circle to any other Blood Brothers in this circle") and Redistribution ("among the Blood Brothers in this circle"): its controller's Blood Brothers only (32.7); owner (2026-10-12): one move at a time — x blood between two of them (the source, the destination, then the amount), once for a given source and destination, or (Redistribution) any number of equipment, one at a time — each move a separate part of the effect: Slake the Thirst answers one move, x blood. Jerry once each master phase. (Tests jerry_moves_the_circles_blood, redistribution_moves_blood_and_equipment, redistribution_moves_equipment_freely_and_blood_once_a_pair, slake_the_thirst_answers_one_move.)

**68.11** Monocle of Clarity: its question changes no game state — the equip action alone; its rulings n/a. (Test monocle_of_clarity_is_equipped.)

**68.12** Inner Essence: "once this combat" — the card goes to the ash heap; its discount stays on the vampire for the combat as an effect (spent as Focus the Blood's, 36.1), gone as the combat ends if unused. Owner, 2026-10-20 (the card was read as staying on the vampire). (Test inner_essence_discounts_a_combat_card.)

**68.13** Major Boon: after it is played (as blocks are declined) or burned, the blocks-declined window closes — only "would be successful" plays (LSJ 20090205). (Test major_boons_do_not_stack.)

**68.14** Hive Mind: [san]'s "another ready Blood Brother of the same circle" any Methuselah's; "only one Hive Mind can be played at [san] each turn" counts [san] plays only. (Test hive_mind_unlocks_the_circle_or_feeds_it.)

**68.15** Gestalt: all or nothing at resolution (a chosen Blood Brother unable to burn: no effect); [SAN]: the reacting vampire plays cards with the chosen ones' Discipline levels until the action ends. (Test gestalt_lends_the_circles_strength.)

**68.16** Zizi "can lock to unlock another Blood Brother of the same circle": no printed timing — at any impulse, Viri ready. (Test viri_and_zizi_work_together.)

**68.17** Loki's Gift [PRO] "burn 1 blood from any vampire": the vampire chosen as the card is played, its action directed at that vampire's Methuselah when another's. (Test lokis_gift_hunts_and_bites.)

## Batch 69 — Shadow Play, The Embrace, the Appolonius merge, legacy cards for TWDA decks, round 9: readings no ruling settles (Claude, 2026-10-12; graded 2026-10-20: all green)

Slice 16u: the cards of the Lasombra, Toreador antitribu and Ravnos TWDA decks and Popó's. Only the readings that no ruling or default reading settles; each is implemented and tested. Graded after the fact per the add-cards skill (owner, 2026-10-20: "I don't want to go over cards one by one myself"): two blind second readers agreed with every reading.

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

## Batch 70 — Legacy cards for TWDA decks, round 10 (the 2025–2026 decks): readings no ruling settles (Claude, 2026-10-13; owner, 2026-10-14: 70.1–70.4, 70.6–70.8, 70.10–70.12, 70.14–70.16, 70.19 confirmed, 70.17 corrected; graded 2026-10-20: 70.5, 70.13, 70.18, 70.20–70.22 green, 70.9 red and corrected)

**70.1** Rumors of Gehenna: the Methuselahs chosen in its terms (the caller may choose themselves) are remembered by the card in play; "two master phase actions (instead of one)" is the same second action as Anson's — neither another copy nor Anson adds a third. (Test rumors_of_gehenna_gives_the_chosen_two_master_phase_actions.)

**70.2** Victoria Ash (G7): "give an acting Assamite +1 bleed" — an acting Banu Haqim (the clan's V5 name), any Methuselah's, the text naming no controller; her Methuselah discards; once each action (rules-qa 6.6), before the bleed resolves (ANK 20231124). (Test victoria_ash_gives_an_acting_banu_haqim_bleed_before_resolution.)

**70.3** Chameleon: "who entered play since your last minion phase" — since the end of the player's previous minion phase: entries of their current minion phase count; X is the target's capacity, paid in blood; the target ready or in torpor. (Tests chameleon_steals_a_vampire_that_entered_not_one_that_merged, chameleon_makes_ones_own_vampire_anarch.)

**70.4** The Meddling of Semsith: "does not replace cards during any Methuselah's minion phase" — the replacements are held and drawn as that minion phase ends (or the turn ends early), counting against the hand size meanwhile (ANK 20180318); the card leaving play mid-phase does not release them before the phase's end. (Tests meddling_holds_replacements_through_the_minion_phase, meddling_outlasts_a_cards_own_delay.)

**70.5** Camarilla Threat: each copy in play adds its pool; paid as the card is discarded, a last pool ousting its Methuselah. (Test camarilla_threat_charges_a_pool_per_discard.)

**70.6** Eat the Rich: "each priscus gets -1 ballot" — its ballots in all lessened by 1 (Gratiano, a priscus with "+1 ballot", keeps one: owner, 2026-10-14); "each non-priscus titled vampire gets -1 vote" — off its total votes in this referendum, never below 0; the Edge cast is worth 2 votes; "a ready titled vampire" read as it passes. (Tests eat_the_rich_taxes_titles_and_doubles_the_edge, eat_the_rich_leaves_gratiano_one_ballot.)

**70.7** Anathema: "reduced to 0 in combat" read as each combat step settles: any loss to 0 in it (damage, a cost paid in blood); a vampire back above 0 by then is not burned; the pool to the Methuselah controlling the opposing minion then. (Tests anathema_burns_a_vampire_reduced_to_zero_not_one_at_zero and the others citing its rulings.)

**70.8** Protected Resources: copies do not lower the cap below 2; "after a minion you control successfully bleeds" — any minion of its controller's, an ally too, every copy they control burning; the cap is on whoever burns pool for the bleed (Major Boon's payer included). (Tests protected_resources_caps_the_pool_burned_not_the_bleed, protected_resources_burns_after_its_controllers_successful_bleed.)

**70.9** Antediluvian Awakening: the vampire burned may be in torpor; burning it burns the card; chosen as the unlock rider resolves; the card gone by then, nothing is due (owner, 2026-10-20; the pool was read as due anyway). (Tests antediluvian_awakening_costs_a_pool_or_a_vampire, antediluvian_awakening_gone_costs_nothing.)

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

**71.24** Nergal (advanced): "reaction cards cost +1 blood" — any played by another minion while he acts (a dual card only as a reaction, PIB 20150924-2); his stealth burn when needed, once each action (revised by 79.1). (Tests nergal_advanced_taxes_reactions_while_he_acts, nergal_advanced_burns_blood_for_stealth_when_needed.)

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

## Batch 74 — Legacy cards for TWDA decks, round 12 (the next 2025–2026 decks entering the corpus): readings graded (Claude, 2026-10-14; owner, 2026-10-15: 74.1, 74.2, 74.6 corrected, 74.3–74.5 confirmed)

Slice 16x. Graded per the add-cards skill: a blind second reader per nine cards, the two reads compared. 74.1–74.6 were red (the reads disagreed, or no precedent): asked of the owner, answered below. The rest are recorded as implemented and tested.

**74.1** Pulled Fangs: "this minion inflicts 1 damage on them" — read first as landing at once, unpreventable. Owner (2026-10-15): "It's preventable (but note it's not a strike)": the damage, after damage reductions (74.6), is offered to prevention as the end of the round's effect, before it lands; strike-only prevention does not reach it. (Tests pulled_fangs_damage_may_be_prevented, pulled_fangs_goes_on_the_loser_and_sends_a_starving_bearer_to_torpor.)

**74.2** Lucinde, Alastor (G7): "this turn, Camarilla vampires you control can enter combat with the chosen vampire as a Ⓓ action" — read first as riding on her text, gone if she leaves play. Owner (2026-10-15): "It should be kept the whole turn. Triggered effects do not disappear with their source": the grant is the using Methuselah's for the turn (`GameState::chosen_this_turn`). Chosen any vampire (in torpor too); each Camarilla vampire of theirs (ready, unlocked) once, +0 stealth, the chosen one ready and another Methuselah's. (Tests lucinde_g7_lets_camarilla_vampires_attack_her_chosen_vampire, lucinde_g7_grant_outlives_her.)

**74.3** Gear Up [nec]: the card named at declaration (LSJ 20010627) gone from the ash heap as the action resolves — the Anarch still unlocks, nothing is fetched. Owner (2026-10-15): "You resolve as much as you can, yes, unlock (I'm unsure myself)." (Test gear_up_necromancy_fetches_the_card_named_as_it_is_played.)

**74.4** Pulled Fangs: "if they must hunt, they go to torpor instead" — in its controller's minion phase, a ready unlocked bearer that must hunt (no blood, or made to) goes to torpor at once, read live. Owner (2026-10-15): confirmed. (Test pulled_fangs_goes_on_the_loser_and_sends_a_starving_bearer_to_torpor.)

**74.5** Paulo de Castille: "after Paulo enters play during your influence phase" — as he is moved out of the uncontrolled region in that phase, before any contest (as Erlik's distribution). Owner (2026-10-15): confirmed. (Test paulo_de_castille_moves_a_blood_to_an_uncontrolled_lasombra.)

**74.6** Saulot, The Wanderer: "damage Baali inflict on Saulot is reduced to 0". Owner (2026-10-15): it covers the damage a Baali inflicts himself, a strike or not (Pulled Fangs' 1 damage), not environmental damage (Carrion Crows, a retainer's). `GameState::damage_reduction` applies to strikes, their riders and a minion's own damage (Pulled Fangs), as Nephandus's. No pool card reaches the non-strike case yet: Pulled Fangs needs more damage inflicted this round, and a Baali's damage to Saulot counts after the reduction, 0. (Test saulot_takes_no_damage_from_baali.)

**74.7** The Book of Going Forth by Night: any burn of an ally opposing its vampire in combat (damage, an effect, a last life paid) is replaced; the steal ends the combat (a control change, LSJ 20020618): the round is over, no press. Both reads agree. (Test the_book_of_going_forth_by_night_steals_an_ally_about_to_burn.)

**74.8** Tyler McGill: the +1 vote counts in votes he casts after a Methuselah burned the Edge for a vote in that referendum; votes already cast are not revised. "Nosferatu" is the clan only. (Tests tyler_mcgill_votes_after_the_edge_is_burned_for_a_vote, tyler_mcgill_cannot_block_nosferatu.)

**74.9** Island of Yiaros: X is the votes the vampire holds — title, printed, borne and its clan's (New Carthage) — a priscus's ballots none (RTR 19970630); for the controller's own striking vampire, after both strikes are chosen; "for that strike" read as this round's strength (an additional strike that round gets it too). Adhiambo: "actions to enter combat with a minion" include the Red List hunt. (Tests island_of_yiaros_adds_a_strikers_votes_to_its_strength, island_of_yiaros_gives_a_priscus_nothing.)

**74.10** Asguresh: once each combat (rules-digest D7, "during a combat … can"); a card played as a combat card (D18) by the minion opposing him. Thucimia: once each bleed action of hers (D7). (Tests asguresh_cancels_a_combat_card_once_each_combat, thucimia_trades_two_hand_size_for_a_bleed_once_each_bleed.)

**74.11** Repulsion: while a vampire bears one, none of its levels is playable by him; [pre]'s maneuver, only to long range, once in the block's combat, any round. Palatial Estate: nothing on an ally host; in torpor too. Gear Up [nec]: not offered with an empty ash heap (rules-digest D14). Divine Sign: the chosen minion is the action's target (D1), any minion. Dive into Madness: a third [DEM] not offered. (Tests repulsion_superior_stays_on_the_vampire_for_stealth, repulsion_presence_gives_a_maneuver_to_long_range_if_blocked, palatial_estate_gives_its_vampire_a_blood_each_master_phase, divine_sign_superior_guards_against_the_chosen_minion, dive_into_madness_caps_at_two_and_older_vampires_burn_it.)

**74.12** Tainted Spring: the animal brought in keeps the bonus while in play — an ally +1 strength, a damage-dealing retainer +1 damage of its kind (close-only stays close-only), a retainer dealing none nothing; [ANI] +1 starting life. Clandestine Contract: The Black Throne pays for it as for any contract. (Tests tainted_spring_superior_strengthens_a_recruited_animal_ally, tainted_spring_adds_to_a_retainers_damage, clandestine_contract_feeds_the_black_throne.)

**74.13** Pulled Fangs played after a round ended early with the opposing vampire sent to torpor (RTR 20030519 allows the play): the card goes on him and its 1 damage lands on him in torpor — owner (2026-10-15): "if it goes to torpor from aggravated damage, it could matter. In that case Pulled Fang's damage must land on them even in torpor": a vampire sent to torpor by aggravated damage keeps his blood and mends it; a vampire in torpor is wounded (rulebook "Wounded"), so unmended normal damage leaves him as he is and aggravated burns. (Tests pulled_fangs_lands_on_a_vampire_sent_to_torpor, pulled_fangs_is_not_offered_against_a_burned_vampire.)

## Batch 75 — End-of-round cards against a burned opponent: readings graded (Claude, 2026-10-15; owner, 2026-10-15: 75.2 confirmed)

The end-of-round window opens though the round's damage burned the opposing minion (RTR 20030519). Graded per the add-cards skill, a blind second reader on the cards' text; where the reads disagreed (75.3) a ruling settles it.

**75.1** Disarm, Pulled Fangs: a card "put on the opposing vampire" is not offered at the end of a round whose damage burned that vampire: no vampire is left to put it on (17.4: a play without its target is masked); one sent to torpor is still there (RTR 20030519, 74.13). `Requirement::OpponentPresent`. Green (17.4; both reads agree). (Tests disarm_is_not_offered_against_a_burned_vampire, pulled_fangs_is_not_offered_against_a_burned_vampire, every_card_put_on_the_opponent_needs_it_present.)

**75.2** Street Cred, Taste of Vitae: what they read is what the round did (damage inflicted, blood lost), and their target is not the opponent: offered when the opponent burned. Taste of Vitae green (LSJ 20031201-2); Street Cred amber, on that precedent, both reads agreeing; confirmed (owner, 2026-10-15: "yes"). (Tests taste_of_vitae_is_played_when_the_opponent_was_burned, street_cred_is_played_when_the_opposing_minion_burned.)

**75.3** Hell-for-Leather [obt]: "Only usable if this Anarch is ready and the opposing minion is not. The opposing minion's controller burns 2 pool", the opposing minion burned this round: played, its controller burns 2 pool — settled by 44.1 (owner-confirmed: "sent to torpor or burned"). Green. The blind second reader read it unplayable (a burned minion not "not ready", no controller); 44.1 stands. (Test hell_for_leather_obt_burns_pool_against_a_burned_minion.)

## Batch 76 — Round 13 (First Blood, Sabbat Preconstructed, the 25th and 30th Anniversary sets, the TWDA's first step): readings graded (Claude, 2026-10-16; owner, 2026-10-16: 76.24 answered)

Seventy-one cards, graded per the add-cards skill: six blind second readers on the cards' text, their reads diffed against the design; where they disagreed with no ruling to settle it, red (76.24, answered by the owner).

**76.1** Banu Haqim Justicar: the "Assamite Justicar" Kasim Bayar prints is the same title (rules-digest §1, V5 clan names): the two are rivals. Green. (Test banu_haqim_justicar_rivals_kasim_bayar.)

**76.2** No choice of a contest with oneself: Toreador, Malkavian, Banu Haqim and Lasombra Justicar cannot choose one of the caller's own vampires while the caller claims that title already (advanced rules §1.13.1); none left, the card is not played (LSJ 20100129). `Requirement::NoSelfContest`. Green. (Another Methuselah's vampire whose controller holds the title: the cards board's "Reachable edges, verified first".) (Test banu_haqim_justicar_never_contests_its_own_kasim.)

**76.3** Zane: the prince chosen as the referendum's terms, any Methuselah's but his own controller's (a contest with oneself, 76.2); passed, Zane takes that very title and contests it with its holder, his primogen title lost (one title). "If Zane is not a prince" reads the effective title (rules-digest §3.15): a contested gained title makes him no prince. Green (the second reader preferred the claimed title, against §3.15). (Test zane_takes_and_contests_a_princes_title.)

**76.4** Calebros: the steal is offered to another Methuselah's older Camarilla vampire, read again at resolution against the actor (older: capacity, strictly); directed at his controller; Calebros cannot block it, his controller's other minions can; the stealer's own title is replaced (one title); a torpid Calebros may be the target. Green (both reads agree). (Test calebros_loses_his_title_to_an_older_camarilla_vampire.)

**76.5** Mustafa: an older Toreador of another Methuselah, in torpor too (no "ready", rules-digest D25), once each polling step (D7), moves 1 blood to him and chooses the side as it uses the ability; a cast of his already made is cast again on that side (its side's printed votes and toll; unpayable, it stands), the votes gained since kept; the last use decides (LSJ 20090304-2); an abstaining Mustafa is not compelled (advanced rules §3.7.6.3). Amber (Kateline's referendum-ability precedent; the reads agree but on an abstaining Mustafa, where §3.7.6.3 settles it). (Tests mustafa_casts_his_votes_as_an_older_toreador_chooses, mustafa_turned_keeps_the_votes_a_card_added.)

**76.6** Kateline Nadasdy: "non-acting" is not the political action's acting vampire; an abstention after the vampire cast cancels its votes and a priscus's ballots (LSJ 20100311, LSJ 19971001); usable locked or in torpor, once each referendum. Green. (Tests kateline_forces_a_non_acting_vampire_to_abstain, kateline_cancels_votes_cast_from_torpor.)

**76.7** Dr. Morrow: any ally, his controller's too; more counters add nothing (LSJ 20070306); the -1 stealth is his text's, standing: in play (torpor included) it applies, gone with him; the counters stay. Amber (the reads agree; Black Hand a trait, LSJ 20070322). (Test dr_morrow_grafts_an_ally_for_minus_one_stealth.)

**76.8** Charnas the Imp: put on any minion, an employ action whatever the target (ANK 20230408), directed at another Methuselah's minion's controller (+1 stealth: the employ's own), on the target only when played (else on the playing minion, LSJ 20090824); controlled by its employer's controller; 1 damage in that controller's unlock phase, out of combat; the employer about to be burned, its controller may put Charnas on any other minion in play, any Methuselah's — declined, it burns; an oust removes it (ANK 20180408). Amber ("another minion" unaddressed, rules-digest D11; both reads agree). (Tests charnas_the_imp_bites_its_employer_and_moves_on, charnas_is_removed_with_its_ousted_controller.)

**76.9** Talbot's Chainsaw: the unlock damage is mandatory, the weapon's, on a ready minion its controller picks, the bearer included (LSJ 20090304-1); the first-round press is mandatory in every combat the bearer is in, used before any other to continue (ANK 20200525-1); "prevent 1 damage each combat" in combat; the granted action fizzles once the Chainsaw left the actor (RTR 19960221) — as does any action an equipment grants its bearer. Green. (Tests talbots_chainsaw_bars_hunts_presses_and_bites_its_controller, talbots_chainsaw_action_fizzles_without_the_chainsaw.)

**76.10** Molotov Cocktail: a ranged strike, at either range; the card itself goes on the striker as its strike resolves; a strike that never resolves — combat ended by the opposing strike (ANK 20200203-1) or dodged (advanced rules §4.6.2, §4.7.1) — leaves it in the ash heap; its weapon strike not that round, additional strikes included; burned after its strike resolves, or at the end of combat. Green. (Tests molotov_cocktail_becomes_a_weapon_for_one_strike, molotov_cocktail_is_not_put_when_combat_ends_first, molotov_cocktail_does_not_strike_the_round_it_is_put_in_play.)

**76.11** Entombment and Coma: a strike naming the object it acts on is not chosen against the other kind ([obt] against a vampire, [OBT] and Coma against an ally: advanced rules §4.3.3); close range only (LSJ 20000110). Green (revises the engine's earlier no-op reading of Coma). (Tests entombment_burns_an_ally_or_torpors_a_vampire, coma_sends_to_torpor_and_holds_the_unlock, coma_is_not_chosen_against_an_ally.)

**76.12** Two Wrongs: the bleed turned to the bleeding minion's controller's prey, who may block (a change of target); the next card that would change this bleed's target is canceled as it is played, its cost paid (ANK 20260216); abilities are not cards and pass it by. Green. (Test two_wrongs_turns_a_bleed_and_shields_it.)

**76.13** Form of Corruption: "anew" — the prey did not hold the Edge before; the counter optional; the steal in its controller's master phase, no master phase action, a vampire of the current prey's, in torpor too, with no more blood than the counters (0 counters, a 0-blood vampire: literal); for good. Green (both reads agree). (Tests form_of_corruption_steals_a_drained_vampire, form_of_corruption_counts_only_a_new_edge.)

**76.14** Game of Malkav: each pick secret from every other seat until the last (the engine keeps them outside the state, a viewer is told a number was chosen, not which); revealed, the gains and burns at once. Green. (Tests game_of_malkav_pays_the_undercut_numbers, another_seats_secret_number_is_not_told.)

**76.15** Alastor: the vampire chosen as the terms, any Methuselah's ready Camarilla one without an Alastor; the search optional, Alastor attached anyway (LSJ 20050331-2); the equipment put on, not equipped — none of its "as equipped" effects (LSJ 20090506), no lock, its requirements ignored, inferior, X as 0 (LSJ 20100204); half its cost rounded down, the pool its caller's, the blood the chosen vampire's (LSJ 20040518); the attached vampire commits no diablerie. Green. (Tests alastor_equips_its_vampire_and_bars_diablerie, alastor_finding_equipment_is_optional, alastor_blood_cost_is_the_chosen_vampires, alastor_puts_on_without_equipping, alastor_needs_a_vampire_to_choose.)

**76.16** Hand of Conrad: an equipment's Disciplines are not its bearer's while an effect bars it from using equipment (RTR 20010711: Drawing Out the Beast); its action fizzles off its bearer (76.9). Green. (Tests hand_of_conrad_fizzles_off_its_bearer_and_its_potence_needs_equipment, an_equipment_bar_ends_with_a_combat_cut_short_by_the_turn.)

**76.17** Aching Beauty: on any Methuselah's Toreador, in torpor too (Terrifying Visage's precedent, 53.7); the burn applied as the block lands, before any card (RTR 19991206), so an action then ended still costs the blocker; copies add up (LSJ 19980225); after it, no Mask of a Thousand Faces (RTR 19980623). Amber (precedent; both reads agree). (Tests aching_beauty_burns_the_blockers_pool_per_copy, aching_beauty_burns_even_if_the_action_then_ends, no_mask_after_aching_beauty_burns.)

**76.18** João Bilé: "actions to put vampires in play" are those the Sterile trait bars (advanced rules §5.7.6): the actions whose card puts a vampire in play (The Embrace, Childe of the Revolution and their kin). Amber (the engine's sterile card set; both reads apply the Sterile test). (Test joao_bile_blocks_no_titled_vampire_and_embraces_quietly.)

**76.19** Carolina Vález: immune — the damage of allies and retainers, any Methuselah's, is not inflicted (LSJ 20010627); an ally's weapon damage is the ally's. Amber (both reads agree). (Test carolina_takes_no_damage_from_allies_and_retainers.)

**76.20** Thadius Zho: any vampire, a torpid one too (Apolonia's precedent); a 0-blood target a legal no-op. Amber. (Test thadius_zho_burns_a_vampires_blood_with_stealth.)

**76.21** Emily Carson: offered only while she is locked (Kalinda's precedent, 48.7); once each discard phase. Amber. (Test emily_carson_burns_the_edge_to_unlock.)

**76.22** Epikasta Rigatos: the card bound for the ash heap as her action ends — her action's own card, played that action (ANK 20180628), whichever of the action card types, blocked too — goes to her library, shuffled, instead. Amber (both reads agree). (Test epikasta_recycles_her_action_card.)

**76.23** Murat: "Followers of Set" are the Ministry (rules-digest §1); no V5 Ministry has [ser], so inert in the pool. Green. (Test murat_makes_serpentis_combat_cards_cost_the_ministry.)

**76.24** Camarilla Vitae Slave: the chosen Discipline is superior until its controller's next master phase, as printed (advanced rules §2.5.3), not until the next unlock phase (LSJ 19970224); kept if the retainer is burned or stolen (LSJ 19970224, advanced rules §5.6.4). Owner (2026-10-16): the next master phase. `Expiry::MasterPhase`, ended in `start_master_phase`. (Test camarilla_vitae_slave_raises_capacity_and_a_discipline.)

**76.25** The rest read as printed, settled by the default readings and their KRCG rulings: Aidan Lyle and Charles Delmare (a press each combat), Antón de Concepción, Bela (no political actions, a card calling a referendum included), Black Metamorphosis, Computer Hacking, Duality (played at [vic]), Dónal O'Connor (G6, torpor too, LSJ 20011022), Foundation Exhibit (own untitled vampires), Frondator (the action's cost, PIB 20110918), Gem Ghastly (cards played normally only, LSJ 20050119), Grimgroth (himself included), Jeremy, Khobar Towers (an ally's printed cost; no X-cost ally in the pool), L'Épuisette, Lady Vadislava (RTR 19960112), Lasombra Justicar, Lord Leopold, Luca Italicus, Molly MacDonald (read live), Nikolaus Vermeulen G2/G6 (torpor too), Rutor, Selma, Thomas De Lutrius, Troius, Una, University Hunting Ground, Vasily, Animalism; the eighteen whose text is their sect and title. Green. (Tests in twda_round13.rs, the_text_only_vampires_carry_their_printed_traits.)

## Batch 77 — Round 14 (2018 Humble Bundle, 2015 Storyline Rewards, Danse Macabre, the TWDA's steps 1–8): readings graded (Claude, 2026-10-16; owner, 2026-10-16: 77.28 answered)

Sixty-nine cards, graded per the add-cards skill: eight blind second readers on the cards' text, their reads diffed against the design; four corrected to the readers' (77.6, 77.13, 77.19, 77.20), one red (77.28, answered by the owner); one older gap found on the way (77.34); one corrected after the egress review (77.35).

**77.1** Osebo Kholo: the Laibon title of Osebo (2 votes, `Title::Laibon`), played by a Laibon Osebo, titled too (one title: his is replaced), unless an older ready untitled Laibon Osebo exists; an action modifier by the acting vampire, a reaction by another, locked too; not in a referendum passing automatically (PIB 20150105, LSJ 19980107). The move is a standing condition, checked whenever the game settles (`settle_moving_titles`): the card moves to the oldest ready untitled Laibon Osebo, a tie picked by the card's controller (`Interrupt::MoveCardToOldest`); none, it stays. Amber (the tie's chooser unprinted). (Test osebo_kholo_titles_the_oldest_osebo_and_moves_on.)

**77.2** Vlad Tepes: meets every sect and title requirement of the cards he plays or uses (ANK 20201028; the in-play effect of a card requiring them too, PIB 20150306), never a Black Hand requirement (a trait, rules-digest §3.15); each card reads the sect or title it lets him pass as, its referendum terms included (`ReferendumDef::feasible`'s `as_if`). Green. (Tests vlad_tepes_plays_cards_requiring_a_sect_or_title, vlad_tepes_plays_sect_cards_after_the_fall.)

**77.3** Summon History: X chosen as it is played, 0 or more (LSJ 20100314); the search at resolution, the library and/or the ash heap ([TEM]: the crypt and/or the ash heap), against the printed cost or capacity; the card put in play, not played: no requirement, its own X cost 0 (PIB 20111101), a vampire with no blood, able to act this turn (LSJ 20100204). Green. (Tests summon_history_puts_a_cheap_card_in_play, summon_history_puts_a_vampire_in_play_with_no_blood.)

**77.4** Andrew Stuart: as he plays a card at its [tha] mode, the top card revealed, first, before any other cancel (LSJ 20100728); one requiring [tha] at any mode (LSJ 20011204-3) cancels the card played — still played, its cost paid unless an action card (whose cost is paid at resolution), replaced normally — and Andrew burns 1 blood. A play canceled as it opens is settled at once, with no window (`open_as_played`; Two Wrongs likewise). Amber (the reads agree). (Test andrew_stuart_gambles_his_blood_sorcery.)

**77.5** Orlando Oriundus: while he is ready, the votes his controller's vampires cast count double at the tally; the Methuselah's own votes (the Edge, a card's vote) do not. "Other multipliers may not be played on your vampires": no other multiplier is in the pool (inert, DESIGN §11.3). Green. (Test orlando_doubles_his_methuselahs_votes.)

**77.6** Kyle Strathcona: "any other controlled Sabbat vampire" is any Methuselah's, "controlled" unaddressed (49.5), torpid too (D28). Corrected (the design had his controller's only). (Test kyle_strathcona_feeds_a_sabbat_vampire.)

**77.7** Mithras (G3, `group: Some(3)`; the G6 card bound by group): +2 votes, no title (rules-digest §3.15); once each action he performs, a reaction card played by a younger vampire canceled — not an ally's, even played as a vampire — its cost not paid, a wake included. Green. (Test mithras_g3_cancels_a_younger_vampires_reaction.)

**77.8** Superior Mettle: [FOR] not played without the blood of its burn beyond its cost (ANK 20210226), nor with no damage to prevent (LSJ 20001114); each later round of that combat, the vampire is asked whether to burn 1 blood against the opposing minion's initial strike, additional strikes not covered (`settle_burn_to_prevent`, `Interrupt::MayPreventForBlood`). Green. (Tests superior_mettle_buys_prevention_each_round, superior_mettle_needs_the_blood_of_its_burn.)

**77.9** Sanguine Entrapment: the opposing minion cannot choose dodge ([cel]) or combat ends ([CEL]) this round, as printed (the superior mode no "as above"); one per vampire each round. It restricts the opposing minion's choice of strikes: Groundfighting cancels it (beside 71.6's; LSJ 20050221's exclusions are range, maneuvers and undodgeable strikes). Green. (Tests sanguine_entrapment_bars_a_dodge_or_a_combat_end, groundfighting_cancels_sanguine_entrapment.)

**77.10** Tangle Atropos' Hand: played on a pending block attempt; the action canceled, not resolved — the same action may be taken again (LSJ 20070709) — its card back in hand ([tem]; discard down afterward), and +1 stealth on the vampire's next action this turn ([TEM]), applied as that action is declared (a standing bonus, not a play bound by need). Green. (Tests tangle_atropos_hand_cancels_and_returns_the_card, tangle_atropos_hand_leaves_the_action_taken, tangle_atropos_hand_spends_a_veil_of_the_legions_action, tangle_atropos_hand_spares_the_dawns_damage.)

**77.11** Rewind Time: [pre] a bleed against you, read live (after a redirect, not yours); [tem] any action card as played (ANK 20200813-2), not one played otherwise than from hand (RTR 20001020); [TEM] outside an action, never in combat (LSJ 20080618), another Methuselah's non-out-of-turn master, its cost not paid, its player +1 master phase action. Green. (Tests rewind_time_cancels_an_action_card, rewind_time_cancels_a_master_card_outside_an_action.)

**77.12** Esprit de Corps: "would target you (or a card you control)": an action card picking your seat, minion or card, or a bleed against you; not a political action (terms are no target, D6), an employ or a recruit (no target); the played mode's Discipline (advanced rules §1.8.1); not a card played otherwise than from hand (RTR 20001020). Green. (Test esprit_de_corps_strengthens_and_cancels.)

**77.13** Inside Dirt: the Edge burned at resolution; no longer its player's by then, nothing happens (as Patsy, 58.11). Corrected (the design burned the blood or pool anyway). (Tests inside_dirt_burns_the_edge_for_blood_or_pool, inside_dirt_does_nothing_once_the_edge_is_lost.)

**77.14** Demonstration and Quentin: "cancel the votes and ballots of a vampire" is that vampire made to abstain (Quentin's own words): its votes cast and to come in that referendum, not its Methuselah's (`Effect::Abstain`); Quentin's once each political action. Amber (the reads agree). (Tests demonstration_makes_a_vampire_abstain, quentin_forces_a_vampire_to_abstain.)

**77.15** Public Vilification: the chosen vampire another Methuselah's, younger than the acting vampire, ready; the 3 blood allocated among other vampires, the acting one included, as the terms; short of blood as it passes, the caller picks which receive (LSJ 20010606; `RefEffect::MoveDistributedFromFirst`, `Interrupt::ShortfallPicks`). Green. (Test public_vilification_moves_three_blood.)

**77.16** Powerbase: Luanda: the chosen vampire, torpid too (D28), enters combat as its action once that turn while the card stays locked (it unlocks with its controller's unlock phase); the card stolen or burned, the choice is gone; another Methuselah's minion only (17.3). Amber. (Tests powerbase_luanda_sends_its_chosen_vampire_to_fight, powerbase_luanda_stolen_takes_its_choice.)

**77.17** 419 Operation: one of its two unlock-phase moves each phase (6.6); the prey's Edge burn is the prey's at any impulse, with no counter on it too (ANK 20221011-3), the counters to whoever is the controller's prey then. Amber (no window printed). (Tests operation_419_skims_the_prey_until_the_edge_burns_it, operation_419_burns_with_no_counter_on_it.)

**77.18** Changeling Skin Mask, Marked Territory: the intercept burn only once the bearer's block attempt needs it (D15); Marked Territory's during an action directed at its controller. Green. (Tests changeling_skin_mask_gives_obfuscate_and_burns_for_intercept, marked_territory_is_put_on_and_burns_for_intercept.)

**77.19** Changeling Skin Mask: "the vampire with this equipment may burn it": an ally bearing it cannot. Corrected. (Test changeling_skin_mask_gives_an_ally_no_intercept.)

**77.20** Masika (and London Evening Star): the +1 intercept goes to a minion attempting to block that needs it (D15) — Masika's controller's, London's any Methuselah's — for the rest of the action. Corrected (the design let either give intercept to any minion, needed or not). (Tests masika_discards_a_political_action_for_intercept, london_evening_star_gives_intercept.)

**77.21** Dark Steel: [obt][pot] a combo (25.2); its environmental damage dealt in normal strike resolution at close range each round of that combat, against a dodge too, after a combat ended or a combatant sent to torpor by first strike, none (RTR 19941109, ANK 20200422). Green. (Test dark_steel_hurts_at_close_range.)

**77.22** The Dracon: +1 damage on a ranged strike that deals damage, +1 blood or life on one that steals, each where the strike does it, at close range too; none on a strike dealing none (RTR 19960221); [vic] the played mode. Amber ("or" read as whichever the strike does). (Test the_dracon_steals_more_with_ranged_strikes.)

**77.23** Selena: a werewolf's strike damage on her aggravated (no werewolf in the pool deals other damage); Righteous Endeavor: "infernal" the printed trait. Amber. (Tests selena_takes_aggravated_damage_from_werewolves, righteous_endeavor_stands_against_infernal_minions.)

**77.24** Strohmann, Badr: Nergal's shape (38.4): the mode played; Strohmann's surcharge on [aus] reactions while he acts, his combat and referendum included. Green. (Tests strohmann_makes_auspex_reactions_cost_more, badr_plays_obtenebration_for_less.)

**77.25** De Sade: Alexander Silverson's toll (41.3), on a vampire's first cast against only (ANK 20221019); a cast turned by another's effect (Mustafa) pays no toll — Silverson's and Mithras's tolls read the same (one `Stat::VoteAgainstToll`). Green. (Tests de_sade_makes_votes_against_his_referendum_cost_a_blood, de_sade_tolls_a_first_cast_only.)

**77.26** Bruce de Guy, Maldavis, Tabriz Assembly: Bruce's bar, used as he announces a political action, keeps Camarilla vampires from casting votes and ballots in its referendum, not their Methuselahs' votes; Maldavis's +1 to any Methuselah's primogen in a referendum he calls (a granted political action too), read live; Tabriz's burn referendum any vampire's, directed at its controller when another's, its +1 vote its controller's. Amber. (Tests bruce_de_guy_silences_the_camarilla, maldavis_gives_primogen_a_vote, tabriz_assembly_feeds_true_brujah_and_votes_against_its_burning.)

**77.27** Mimir: other Methuselahs' master cards cannot be played targeting him — choosing him as played; Archon Investigation's burn of an acting minion included — an in-play master's later effect may. Green. (Tests mimir_is_no_other_methuselahs_master_target, mimir_is_no_archon_investigation_target.)

**77.28** Rise of the Fallen: "each action during which you have burned corruption counters": counters its controller burned during the action — by their own effect or choice, whoever's counters (another Methuselah burning yours does not count); the prey burns 1 pool once, after that action (`Hook::CorruptionBurnedInAction`; `GameState::burn_corruption` notes the burner). Owner (2026-10-16): burned by you only. (Tests rise_of_the_fallen_taxes_the_prey_for_corruption, rise_of_the_fallen_counts_the_counters_you_burn.)

**77.29** Baba Yaga: a recruit or employ by another card counts (PIB 20150105-2, Pack Alpha's `EmployNamed`), once in each player's turn (ANK 20180719-4; `Hook::HostRecruitsOrEmploys`); Synner-G: a successful equip from hand, a fizzled one no (Keegan's shape). Green. (Tests baba_yaga_unlocks_after_recruiting_or_employing, synner_g_unlocks_after_equipping_electronics.)

**77.30** Festivo dello Estinto: every Methuselah's Sabbat vampires; a successful hunt, card hunts too, tops up from the bank, not the hunt's target (RTR 20030519); a hunt gaining no blood is not successful (LSJ 20050720-2). Green. (Tests festivo_dello_estinto_feasts_the_sabbat, festivo_dello_estinto_fills_card_hunts_from_the_bank.)

**77.31** Shakar: the Hunt: the contract's combat against a minion of another Methuselah only (17.3); "every Black Hand vampire is considered chosen" read by no pool card beyond its own action (inert, DESIGN §11.3); Gerald FitzGerald's Black Hand kept through a sect change (LSJ 20070322), his Seraph read by no pool card (inert; read by Tattoo Signal since round 15, 78.30). Green. (Tests shakar_lets_the_black_hand_hunt_its_minion, gerald_makes_a_younger_sabbat_vampire_black_hand.)

**77.32** Winterlich: no changeling ally in the pool: his text inert (DESIGN §11.3). Green.

**77.33** The rest read as printed, settled by the default readings and their KRCG rulings: Aimery, Alex Camille (threshold, open K23), Bulscu (an action at another's equipment or location directed, D12), Carna (torpor too), Claudio Severino (Ayse Dhanial's discard), Dan Murdock (Sakhar's search), Dr. Julius Sutphen (Mithras G6's toll), Gold Pan Dan (once each turn after combat), Karif al Numair (any Methuselah's, toggled), Lazverinus, Martin Franckel (the bled Methuselah, read live), Mélusine d'Anjou (a younger vampire's pending attempt), Nehemiah (Heart of Nizchetus's moves; his [cel] modes only barred, LSJ 20050322-2), Paul Cordwood (stealth or intercept when needed), Recruitment Exercise, Spoils of War, Suhailah (read live), Taste of Death; the twelve whose text is their sect and title. Green. (Tests in twda_round14.rs, the_round_14_text_only_vampires_carry_their_printed_traits.)

**77.34** My Kin Against the World and Abombwe: their Laibon clauses were left inert as if no Laibon were in the pool — Laibon came with earlier rounds. A Laibon acting vampire may choose younger Laibon of any clan (`Requirement::ActorAndTargetSect`); Abombwe goes on a Laibon or a vampire with Protean. Corrected (found on the way; 63.16's "no Laibon" amended). (Tests my_kin_against_the_world_unlocks_a_laibon_of_another_clan, abombwe_needs_protean_only_when_played.)

**77.35** Recruitment Exercise: played with an empty crypt, it draws nothing (LSJ 20060620, "draw as much as you can"). KRCG's rulings put it under "Can draw from empty" on 2026-07-27 and dropped the "cannot be played when the target crypt is empty" (RTR 20000501, LSJ 20070411-2) that the published vtes.json of 2026-06-30 still attaches to it. Mozambique Allure keeps that ruling as its own (69.14). Corrected (the design had followed vtes.json; the egress review caught it). (Test recruitment_exercise_draws_and_feeds_the_black_hand.)

## Batch 78 — Round 15 (the 2023-07-01 Promo set, the TWDA's steps 1–8): readings graded (Claude, 2026-10-17; owner, 2026-10-17: 78.1 confirmed, 78.32 and 78.38 corrected, 78.28 and 78.54 completed, the ambers confirmed)

Fifty-six cards, graded per the add-cards skill: seven blind second readers on the cards' text, their reads diffed against the design and checked against the code; four corrected (78.20 to the reader's, 78.28 to rules-qa 8.1, 78.34 to the reader's, 78.45 found on the check), three red (78.1 confirmed; 78.32, 78.38 corrected by the owner), one amber completed by the owner (78.28); five older gaps found on the way (78.31, 78.50–78.53); one reading after the egress review (78.54).

**78.1** Appius Claudius Corvus: Black Hand and Seraph generated crypt traits (`gen_crypt.py`'s `seraph`, `GameState::is_seraph`; the api's `seraph` trait), Black Hand kept through a sect change (LSJ 20070322, ANK 20180807); his +1 stealth standing, need or not (D15). Seraph a trait like Black Hand (G00012), not a title: no votes, not "titled" (Lubomira Hradok, Gerald FitzGerald's "non-titled"); a reader read it a title. Red, confirmed (owner: "trait not a title"). (Tests appius_is_a_black_hand_seraph_with_stealth, the_round_15_text_only_vampires_carry_their_printed_traits.)

**78.2** Abdelsobek: a vampire (or mummy: none in the pool) his controller controls, himself and one in torpor included (D28), an unlocked one offered (a legal no-op, 17.4); once a turn, whatever the target (the same printed action, 1.4). Green. (Test abdelsobek_unlocks_a_vampire_of_his_controller.)

**78.3** Ghede: his enter-combat action directed at the chosen minion's controller (D12); combat cards played using Serpentis cost him 1 fewer blood (the mode played, 38.4), with Eye of Hazimel's 1 more. Green. (Test ghede_rushes_and_plays_serpentis_combat_cards_for_less.)

**78.4** Juniper: an ally of her controller's predator or prey, chosen as the action is announced, directed at its controller, re-checked at resolution (D13); a permanent steal, Xipe Totec's. Green. (Test juniper_steals_an_ally_of_her_predator_or_prey.)

**78.5** Onaedo: +1 stealth during her diablerie actions while any Methuselah's Otieno is ready (D11, 73.7); no Otieno in the pool (inert, DESIGN §11.3). Green. (Test onaedo_has_no_stealth_without_otieno.)

**78.6** Saul Meira: once each discard phase (6.6), 1 blood, another ready Black Hand vampire of any Methuselah (D11), Tattoo Signal's reach too (78.30). Amber (the reads agree). (Test saul_meira_unlocks_a_black_hand_vampire_in_the_discard_phase.)

**78.7** Tegyrius, Vizier: his "2 votes (titled)" a generated title kept through a sect change (LSJ 20010714); +2 votes while a ready Victoria Ash is in his controller's ready region; no votes in torpor (D25). Green. (Test tegyrius_gets_two_votes_beside_victoria_ash.)

**78.8** Feo Ramos: in his controller's unlock phase, each older Sabbat vampire of another Methuselah may burn 1 blood to lock him, once each (6.6), locked or in torpor too (D25). Amber (Mustafa's shape; the reads agree). (Test feo_ramos_is_locked_by_an_older_sabbat_vampire.)

**78.9** Shalmath: unlocks once in each Methuselah's turn (ANK 20180719-4), in torpor too (D25); only a card's Celerity mode is barred (LSJ 20050322-2), equipment and abilities not; scarce generated. Green. (Test shalmath_unlocks_once_each_turn_and_shuns_celerity.)

**78.10** Cybele: +1 master phase action if unlocked as her controller's master phase begins, read then, in torpor too (D25); her +2 bleed action costs 1 blood at resolution and is her one bleed of the turn, the rulebook's included (1.4). Green. (Test cybele_gives_a_master_phase_action_and_bleeds_for_three.)

**78.11** César Holfield: at the end of his controller's minion phase, read then: a Gehenna event in play (any Methuselah's) and no hunt by him this turn (a card hunt or a blocked one counts, 72.1, 40.2) — 1 blood, none at 0; in torpor too (D25). Green. (Test cesar_holfield_burns_a_blood_under_gehenna_unless_he_hunted.)

**78.12** Donald Cargill: 1 blood after any referendum his controller's predator called passes, after all its effects, still if it ousted that predator (LSJ 20090115-2): `Requirement::ActingByPredator` reads `GameState::predator_keeping`, the acting seat's place kept once ousted mid-action; in torpor too (D25). Green. (Test donald_cargill_bleeds_after_his_predators_referendum.)

**78.13** Edward Vignes: burns a card printing Dominate from his controller's hand to fail an action directed at him, not once a block is attempted or made (ANK 20260316, LSJ 20041022); the failed action reached resolution (1.4), was not blocked, keeps its after-resolution window (ANK 20221205); the burned card goes to the ash heap and is replaced, as a discard (8.3); in torpor too (D25). Amber (the replacement; the reads agree). (Tests edward_vignes_fails_an_action_at_him_for_a_dominate_card, edward_vignes_cannot_fail_a_blocked_action_and_a_failed_one_is_spent.)

**78.14** Iris Bennett: an older vampire's (capacity, D30) successful block lets its controller look at her controller's hand before block resolution, once that block. Green. (Test iris_bennett_shows_her_hand_to_an_older_blocker.)

**78.15** Katherine Stoddard: once during her controller's turn, ready (locked allowed), a card printing Auspex discarded from hand, then a draw up to hand size. Green. (Test katherine_stoddard_cycles_an_auspex_card_once_a_turn.)

**78.16** Lubomira Hradok: her successful bleed of a Methuselah controlling a ready titled vampire (the bled one after redirects; a contested title not titled, §3.15), checked as the bleed succeeds: she skips her next normal unlock (PIB 20110915-1). Green. (Test lubomira_stays_locked_after_bleeding_a_methuselah_with_a_titled_vampire.)

**78.17** Walker Grimes: +1 pool on each master card his own controller plays targeting him (LSJ 20081124-1), folded with the haven surcharge (`GameState::master_surcharge_on`). Green. (Test walker_grimes_taxes_his_own_masters.)

**78.18** Skidmark: every card his controller plays while he acts, the action card announcing it and his modifiers included, is replaced at the action's end, after its combats and after-resolution window (Bum's Rush's delay, LSJ 20001120); a card with its own delay keeps it. Amber (the reads agree). (Test skidmark_replaces_his_cards_at_the_end_of_the_action.)

**78.19** Franciscus: burned, not sent to torpor, once more than 3 damage has landed on him in one combat (after prevention, 29.3, every round and kind). Green. (Test franciscus_burns_past_three_damage_in_a_combat.)

**78.20** Bolesław Gutowski: once each action he performs (D7), in torpor too (LSJ 20011022); usable before stealth is needed, its stealth then lost (ANK 20231031); the strength for the action; not offered with an empty crypt, nothing to reveal (advanced rules §1.6.5). Corrected (the design offered it, for nothing). (Tests boleslaw_reveals_his_crypt_for_strength_or_stealth, boleslaw_reveals_from_torpor.)

**78.21** Helena (ADV): no blood cost for reaction cards played using Auspex or Dominate, any X (LSJ 20051117), a "burn n blood" effect still paid (D20); merged (`HELENA_MERGED`): Independent, an anarch token burned (ANK 20180524), immune to demons' damage — not inflicted at all (LSJ 20010627) — and +1 level of Daimoinon and Obtenebration (`VampireDef::levels`). Green. (Tests helena_adv_plays_auspex_reactions_free_and_merges, merged_helena_takes_no_damage_from_demons.)

**78.22** Earl, Old Neddacka, Bloodfeud, Denette Stensen, Frank Litzpar, Jephta Hester, Lana Butcher, Jackson Asher, Bituin: their sect, title (Bituin's Baron of Manila) and traits generated, nothing else to define. Green. (Test the_round_15_text_only_vampires_carry_their_printed_traits.)

**78.23** Thrown Gate: a ranged strike with an optional maneuver, committed at the maneuver as Aid from Bats (the window matrix's `maneuver_rider`, now read for "an optional maneuver" too); no maneuver without the strike (LSJ 20021028). Green. (Test thrown_gate_strikes_for_two_at_range.)

**78.24** Typhonic Beast: before range; +1 strength this combat; [pot][ser] 1 damage prevented each round of it, any damage (Horrid Form's); [POT][SER] a press this round. Green. (Test typhonic_beast_strengthens_and_shields.)

**78.25** Engling Fury: 2 blood; [spi] the unlock in this turn's discard phase (Feral Hound's); [SPI] at once. Green. (Test engling_fury_feeds_and_unlocks.)

**78.26** Flurry of Action: a bleed; on success [cel] two cards drawn, as many as the library holds (LSJ 20060620), then a discard down to hand size ("discard afterward" the pool's wording for it: Learjet, Scrounging, The Summoning); [CEL] the vampire unlocks. Amber (a reader read two discarded; the hand size the same at a full hand). (Test flurry_of_action_draws_or_unlocks_after_a_successful_bleed.)

**78.27** Domain of Evernight: [tem] all damage to vampires in the combat its blocked action leads to aggravated, both sides, environmental too, nothing after it (Dawn Operation's, LSJ 20020314); [TEM] after a successful action, still in it (LSJ 19981028, ANK 20190425); one superior play per vampire each turn. Green. (Tests domain_of_evernight_aggravates_a_blocked_combat_and_unlocks_after_success, domain_of_evernight_spares_the_damage_after_combat.)

**78.28** Hag's Wrinkles: an equip action is an equip from hand, Vast Wealth's, or an action card equipping from the library (Magic of the Smith: owner), not a move from another minion (rules-digest §3.7); [thn] the unlock after resolution of a successful equip, at any point of it; [THN]'s +1 stealth the effect's, so only while stealth is needed (8.1, Bonding's). Corrected ([THN] was offered with no stealth needed; the definition's comment named an equip from another minion the code never allowed); completed by the owner (Magic of the Smith, amber missed). (Test hags_wrinkles_unlocks_after_a_successful_equip.)

**78.29** Burnt Offerings: [AUS] after a bleed by a minion of the reactor's predator successfully bled the reactor (redirected included), as Ilonka's; the predator burns 1 pool, an effect. Green. (Test burnt_offerings_burns_the_predators_pool_after_a_bleed.)

**78.30** Tattoo Signal: requires a Seraph (`Requirement::ActorSeraph`); put on the acting Seraph, one a minion; locked by its ready Seraph to unlock another Black Hand vampire of any Methuselah (D11); its burn option open while no ready Seraph is controlled (LSJ 20091203, LSJ 20070410, 33.8). Amber (any Methuselah's; the reads agree). (Test tattoo_signal_lets_a_seraph_unlock_a_black_hand_vampire.)

**78.31** The burn option reads a card's printed trait requirement (`cards::trait_requirements`: a Seraph, an infernal vampire, an unmerged advanced vampire, 33.8): Legacy, Unleash Hell's Fury, Veneficti and Tattoo Signal were burnable with a minion meeting it. Green (a gap found on the way). (Test the_burn_option_reads_the_cards_printed_requirement.)

**78.32** Outside the Hourglass: [obf] a dodge without Temporis (25.2); [TEM] before range, the vampire's 2 damage, not environmental (PIB 20120426), preventable, staged with the other pre-range damage the combatants inflict (ANK 20170930), still if the combat ends early (ANK 20180104); one superior play per vampire each round. Weather Control's damage that round merges with it in a single damage window, unpreventable, added after the prevention. Red, corrected (owner: "they merge in a single damage window"; the design dealt Weather Control's after it). (Tests outside_the_hourglass_inflicts_two_before_range_once_a_round, outside_the_hourglass_dodges_without_temporis, outside_the_hourglass_lands_as_the_combat_ends_before_range, outside_the_hourglass_and_weather_control_land_together.)

**78.33** Mythic Form: before range in the first round, one each combat; +1 strength and 1 non-aggravated damage prevented each round, any damage, never aggravated (LSJ 20040812-2); [PRO] a maneuver this round and a 2R aggravated strike once each round. Green. (Tests mythic_form_grants_an_aggravated_strike_and_a_shield, mythic_form_never_prevents_aggravated_damage.)

**78.34** Stunt Cycle: at long range only; the prevention of the opposing strike's damage covers what lands (RTR 20041202), none carried over (ANK 20200318); an opponent with Celerity may burn 1 blood to prevent the whole strike, no cost (ANK 20210226), never an ally's life (rules-digest §3.14; no ally with Celerity in the pool). Corrected (the design let any minion with Celerity pay). (Tests stunt_cycle_strikes_at_long_range_and_prevents_the_return, stunt_cycle_is_shrugged_off_with_celerity_for_a_blood, stunt_cycle_superior_prevents_what_there_is.)

**78.35** Mantle of the Bestial Majesty: the vampire's hand strikes aggravated until its controller's next unlock phase, as blocker too, other strikes not; [PRO] +1 strength as long (Gear Up's). Green. (Test mantle_of_the_bestial_majesty_aggravates_hand_strikes_until_unlock.)

**78.36** Corporal Reservoir: on any Methuselah's Black Hand vampire (D11), the card its player's (7.5), used by the vampire's controller (ANK 20210309-3): locked to prevent 1 damage in combat or to gain a blood, in torpor too (D25); it unlocks only for a blood in its vampire's unlock phase. Amber (any Methuselah's; the reads agree). (Test corporal_reservoir_locks_for_blood_and_stays_locked.)

**78.37** Into Thin Air: replaced as its controller's unlock phase begins (ANK 20200129); once this action, 1 blood for -1 intercept on a minion attempting to block, an ally or younger than the actor, for the rest of the action; not with Lost in Crowds on the action, either way, any minion's play (`Requirement::NotPlayedThisAction`). Amber (the reads agree). (Test into_thin_air_strips_a_younger_blockers_intercept.)

**78.38** Tye Cooper: burns a minion in torpor of any Methuselah (D11); the ash card picked as the action succeeds (D1); immune to non-aggravated damage, none inflicted (LSJ 20010627). A fizzled burn (its target gone or rescued) is a success (rules-digest §3.2): the ash card still fetched. Red, corrected (owner: "it should give the card from the ash heap"; the design gave none). (Tests tye_cooper_burns_a_torpid_minion_and_fetches_from_the_ash_heap, tye_cooper_takes_no_normal_damage.)

**78.39** Taking the Skin: Minion: the reflex cancels a frenzy card played on the vampire (still played, its cost paid: ANK 20190104, ANK 20260216); the skin when the vampire burns a minion by its action or in combat (LSJ 20090922), not a referendum's or a side effect's burn (RTR 19960124, ANK 20220916); one skin a minion; the extra bleed past the once-a-turn cap (1.4), any bleed; burned in its controller's discard phase. Its play before a diablerie's blood hunt (RTR 19991206) waits on K15 (ledger row blocked). Green. (Tests taking_the_skin_follows_a_burned_minion_with_a_bleed, taking_the_skin_ignores_a_war_ghouls_entry_burn, taking_the_skin_is_not_played_by_a_diablerist_the_hunt_burned.)

**78.40** Pocket Out of Time: [obt]'s blood no cost (ANK 20210226); [TEM] after each combat of the action, the one just ended included, 1 blood for a new combat with the opposing minion, not with a combat queued (RTR 20020501) nor either not ready (ANK 20220429). Amber (the reads agree). (Tests pocket_out_of_time_starts_another_combat, pocket_out_of_time_needs_no_queued_combat_and_a_ready_opponent.)

**78.41** Arcanum Chapterhouse, Alexandria: its controller's predator and prey each burn 1 pool per Hunting Ground they control in their own unlock phase, read then; with two Methuselahs the one rival once (6.8); any minion may burn it, its controller's undirected. Amber (the reads agree). (Test arcanum_chapterhouse_taxes_the_neighbours_hunting_grounds.)

**78.42** Eye of Hazimel: the rush fizzles once the Eye left the actor (RTR 19960221); superior Potence and Chimerstry and the combat-card discount a vampire's only (D16), Ghede's added; not moved or stolen while its bearer is ready (Heidelberg Castle, Redistribution, Confiscation, an equip from it); burned, 5 unpreventable environmental damage on its bearer; in its controller's unlock phase, 1 pool or the Eye. Green. (Tests eye_of_hazimel_empowers_and_costs_its_bearer, eye_of_hazimel_is_not_moved_off_a_ready_bearer, redistribution_leaves_eye_of_hazimel_on_its_ready_bearer, confiscation_cannot_take_eye_of_hazimel_off_a_ready_bearer.)

**78.43** Malkavian Dementia: a ready Malkavian of another Methuselah until its taker's next unlock phase, back as it begins, before any unlock (ANK 20200629-2), whatever its clan by then (ANK 20221208-2), as it is (LSJ 20010119); removed if the Methuselah it returns to was ousted (ANK 20181110), its taker's cards on it burned (owner, 2026-09-06; `return_temporary` gives it back before removing it). Green. (Tests malkavian_dementia_borrows_a_malkavian_until_the_unlock_phase, malkavian_dementia_back_to_an_ousted_methuselah_burns_the_takers_card.)

**78.44** Derange: either clan plays it (ANK 20190203); a younger vampire, torpid too; its clan and sect set as played, kept as it moves (LSJ 20070707), over the underlying ones and a sect change (LSJ 20100811, ANK 20190619), gone with the card (RTR 20201130); a title off its new sect suspended (RTR 20201130, LSJ 20060904); its move to another vampire of any age, never a Malkavian or Malkavian antitribu; one blood unlocks in its unlock phase, a disease's or an infernal's unlock still one (ANK 20221210-2). Amber (the move's reach; the reads agree). (Tests derange_makes_a_younger_vampire_malkavian, derange_keeps_its_clan_and_sect_as_it_moves, derange_unlocks_for_a_blood_under_disease_or_as_infernal, derange_suspends_a_title_off_its_new_sect.)

**78.45** Diabolic Lure: [dai] after its infernal vampire's block, before block resolution; 4 pact counters on the acting minion, kept below the threshold and added across Lures; at its capacity or an ally's printed cost, pool or blood (a token's 0), burned to make it infernal (`Minion::made_infernal`); [DAI] the pool whether or not (LSJ 20100205-2, ANK 20221007). Corrected (a blood-cost ally read cost 0; no such ally above 4 in the pool). (Tests diabolic_lure_makes_the_blocked_minion_infernal, diabolic_lure_burns_pool_short_of_capacity_too.)

**78.46** Descent into Darkness: the vampire, its cards and counters out of play (`Minion::out_of_play`), the card in play; temporary control broken, both to the permanent controller (LSJ 20040526); damage after resolution not applied (LSJ 20040616); in each influence phase of its controller, 2 blood and/or the card burned to return it to its region, locked ([OBT] unlocked), its cards unlocked (LSJ 20060908), its effects and titles remembered (RTR 20000501), effects targeting it resumed (TOM 19960211); an ally never returns (LSJ 20060124). Green. (Tests descent_into_darkness_hides_a_vampire_and_returns_it, descent_into_darkness_breaks_temporary_control_and_skips_the_dawn, descent_into_darkness_returns_its_cards_unlocked, descent_into_darkness_leaves_and_reenters_its_contests.)

**78.47** Dual Form: [pro] a second unlock in that minion phase; [PRO] the actor's capacity -1 for good, never below 1 (LSJ 20050116); the card a non-unique vampire with the actor's clan, sect, capacity and Disciplines as it resolves, added ones too (ANK 20170226), fixed then, no title or trait; up to 2 blood moved, chosen at resolution; the whole set burned when one leaves the ready region, a copy's copy in the original's set, not again (LSJ 20050117); a copy playing it loses its capacity too. The choice when two go to torpor at once (LSJ 20050805) is not built (ledger row blocked). Green. (Tests dual_form_makes_a_linked_copy, dual_form_unlocks_twice_in_the_minion_phase, dual_form_copies_no_title_or_trait_and_burns_its_whole_set, dual_form_played_by_a_copy_joins_the_set.)

**78.48** Madness Network: any Methuselah's unlocked Malkavians (the clan only) take actions in another's minion phase, as that Methuselah ends it (Enkil Cog's moment, 49.6), in torpor too to leave it (RTR 19950509), a hunt with no blood (LSJ 20060508); no Mask of a Thousand Faces over such an action (LSJ 19981007); any unlocked Malkavian, a woken one too, may block its burn besides the eligible blockers (LSJ 20041022-2). Amber (the moment, 49.6; the reads agree). (Tests madness_network_lets_a_malkavian_act_in_anothers_minion_phase, any_malkavian_may_block_the_madness_networks_burn, madness_network_actions_take_no_mask_and_starve_to_a_hunt, a_woken_malkavian_may_block_the_madness_networks_burn.)

**78.49** Curse of Nitocris: put in play under any chosen Methuselah, its player included (D11); its controller burns 1 pool each unlock phase; each time a Methuselah who did not hold the Edge gets it (LSJ 19971006), its controller passes it to any Methuselah, itself included (`Interrupt::GiveCardTo`). Green. (Test curse_of_nitocris_passes_with_the_edge.)

**78.50** A burned acting minion plays no card: the acting minion out of play is no card's actor (`actor_candidates`) — a blood hunt's burned diablerist played Taking the Skin or Predator's Transformation after resolution. Green (found by the round's fuzz). (Test taking_the_skin_is_not_played_by_a_diablerist_the_hunt_burned.)

**78.51** A hand over size waits for no pending choice but a draw's own (rulebook "Drawing Cards"): Flurry of Action's draw is discarded down before Curse of Nitocris's pick (`Engine::discards_first`). Green (found on the way). (Test a_discard_down_comes_before_curse_of_nitocris_passes.)

**78.52** A seat target never names an ousted Methuselah (Curse of Nitocris's choice). Green (found on the way). (Test curse_of_nitocris_names_no_ousted_methuselah.)

**78.53** Lost in Crowds and Into Thin Air exclude each other both ways: Lost in Crowds was playable after Into Thin Air. Green (found on the way). (Test into_thin_air_strips_a_younger_blockers_intercept.)

**78.54** Descent into Darkness's vampire out of play — a contested vampire cannot act, so it never plays Descent contested (owner): its title's contest and its cards' left until it returns (LSJ 20080816), a rival left alone uncontested; no rival of a copy entering meanwhile (advanced rules §1.13.3), Jimmy Dunn's burn not reaching it; back, it and its cards enter their contests again, its remembered title too. Still its controller's for "controls a copy" (LSJ 20040526); not unlocked in the unlock phase, its blood not drained, its cards out of play. Amber, confirmed (owner: a copy entering meanwhile is no contest until the vampire comes back). (Test descent_into_darkness_leaves_and_reenters_its_contests.)

## Batch 79 — "During" always means once: the sweep (owner, 2026-10-17, from a live game: Eyes of the Wild offered its burn more than once)

**79.1** "During" in card text always means once in the span: "The card uses the 'During' wording which means only once during the time span … During always means only once" (owner). Eyes of the Wild's "during this action, this Gangrel can burn 1 blood to get +1 intercept" and Nergal (ADV)'s "during an action, he can burn 1 blood to get +1 stealth" were repeatable while payable: once each action now (supersedes 71.24's "repeatable"). The other 200 pool cards granting a "during" ability were swept and found once in their span. (Tests eyes_of_the_wild_burns_blood_for_intercept_once_and_relocks_without_a_block, nergal_advanced_burns_blood_for_stealth_once_each_action.)

**79.2** Marciana Giovanni: she locks once to grant the burn, and the chosen vampire burns 1 blood for +1 intercept once (owner, correcting a reading of LSJ 20100604-2 as "any number of times": "for the duration of the action" is when, not how often). (Tests marciana_lets_a_vampire_burn_blood_for_intercept, marcianas_grant_keeps_a_block_in_reach.)

**79.3** Maris Streck (added to the pool for it, owner): "Maris can burn a blood to give a blocking minion +1 intercept" has no "during": as often as she pays each action (LSJ 20020814, ANK 20180307-2, owner), while locked (ANK 20220705); any Methuselah's minion attempting to block, while it needs intercept (3.2). Her +1 stealth action looks at and rearranges the top 5 cards at resolution, as Eulogio's (55.3). (Tests maris_streck_burns_blood_for_intercept_as_often_as_she_pays_while_locked, maris_streck_rearranges_the_top_five_of_her_library, a_third_partys_maris_streck_keeps_a_block_in_reach.)

**79.4** Descent into Darkness: "during your influence phase, this vampire can gain 2 blood and you can choose to burn this card to return the vampire" — the blood and the return each once, both in the same influence phase (owner); they shared one use of the phase. (Test descent_into_darkness_hides_a_vampire_and_returns_it.)

## Batch 80 — Round 16 (the TWDA's steps 1–6 and five promo sets: 2019 Promo Pack 1, 2021 Kickstarter Promo, 2021 Promo Pack 3, 2025 CC Promo, 2019 DriveThruCards Promo): readings graded (Claude, 2026-10-18; owner, 2026-10-18: 80.4, 80.35, 80.37, 80.38, 80.41, 80.42 and 80.43 answered)

Fifty-one cards (27 library, 24 crypt: 16 defined, 8 text-only), graded per the add-cards skill. Six blind second readers read the cards' text; their reads were diffed against the design and checked against the code. Results: one corrected to the reader's (80.9); seven red, answered by the owner (80.4, 80.35, 80.37, 80.38, 80.41, 80.42, 80.43); four gaps found while testing (80.31, 80.40, 80.44, 80.45) and one reading found the same way (80.36's lost continuation).

**80.1** Aisling Sturbridge: a younger Tremere (capacity, D30; the clan Tremere) of any Methuselah (D11) is no candidate to block her, read live on the minion attempting; an older Tremere and a younger non-Tremere may block. Green. (Test aisling_sturbridge_cannot_be_blocked_by_younger_tremere.)

**80.2** Printed stats: Blythe Candeleria -1 strength in combat with a Malkavian (the clan only); Kite +1 intercept; Rodolfo +1 bleed; Richard Tauber -1 intercept attempting to block a Lasombra or a Tzimisce; Yazid Tamari (ADV) +1 stealth on his directed actions: a standing bonus for the action, not a "can" ability, so not 79.1's once; his Black Hand generated, the [MERGED] text inert without the base card (DESIGN §11.3). Aaron Bathurst, Almiro Suarez, Blister, Dolphin Black, Hannigan, Korah, Masika St. John, Monica Chang: sect and title (Korah's priscus) generated, nothing else to define. Green. (Test round_sixteen_printed_stats.)

**80.3** Dr. Solomon Grey: his controller burns 1 pool as he enters play (influenced out, or put in play), before any contest (LSJ 20080526); an effect, not a cost. Green. (Test dr_solomon_grey_burns_a_pool_as_he_enters_play.)

**80.4** General Perfidio Díos: burns 1 blood to unlock and attempt to block an action directed at a location his controller controls; usable several times in one action (LSJ 20030605: no "during", 79.3); not bound by "only when needed"; a forced attempt keeps the blocking restrictions (42.8). Offered while he is unlocked too: the owner's answer was "Only while locked when opportunities are off. Technically legal when unlocked but rather useless". So the unlocked use is a legal no-op (`Engine::gains_nothing`), shown only with opportunities on. Red, answered. (Test general_perfidio_dios_unlocks_to_guard_a_location.)

**80.5** Karsh (ADV): +3 votes in blood hunt referendums, untitled, on either side; he cannot block an infernal vampire, read live at the attempt. His Blood Hunt plays are inert, because no card named Blood Hunt is in the pool, and his [MERGED] Imperator text is inert without the base card (DESIGN §11.3). Green. (Tests karsh_cannot_block_an_infernal_vampire, karsh_casts_three_votes_in_a_blood_hunt.)

**80.6** Marie Faucigny: non-weapon equipment costs her 1 less blood or pool to equip, a locquipment's cost included (Palatial Estate, LSJ 20080109); a weapon pays full cost. Her dodge: 1 blood, once each combat (D32). Green. (Test marie_faucigny_equips_non_weapons_cheaper.)

**80.7** Menele (ADV): while any Methuselah's Helena is ready (D11), minion cards (60.3) cost him 1 fewer blood: the blood cost only, read live as it is paid; the [MERGED] text inert without the base card. Green. (Test menele_plays_cheaper_while_helena_is_ready.)

**80.8** Qawiyya el-Ghaduba: burns 2 blood to unlock, once in her controller's minion phase (ANK 20190207, 6.6), any time in it (26.5), in torpor too (D25); only offered while she is locked (48.7). +2 strength, generated Red List. Green. (Test qawiyya_unlocks_once_in_her_minion_phase.)

**80.9** Nefertiti (ADV): her +1 stealth action searches her controller's crypt for Aabbt Kindred, which is not in the pool. The search finds nothing, a legal action (17.4), and the crypt is shuffled after it. It is not offered with an empty crypt, nothing to search (78.20). Corrected (a reader's catch: the design offered it with an empty crypt). The granted action's search (`crypt_named_to_uncontrolled`) was found on the way; it always shuffles. The [MERGED] unlock is inert without the base card. (Test nefertiti_searches_her_crypt_but_not_an_empty_one.)

**80.10** Fiorenza Savona: her controller's Edge burned for a vote, and each political action card burned for a vote, gets +1 vote, in any referendum; in torpor too (D25), because the votes are her Methuselah's and not hers. Green. (Test fiorenza_savona_doubles_edge_and_card_votes_and_praxis_seizure_titles_her.)

**80.11** Okulos: 1 blood to draw 1 card, then discard down to hand size; once in each Methuselah's turn (ANK 20180719-4, Shalmath's), on any impulse, in torpor too (D25). Green. (Test okulos_draws_once_each_turn.)

**80.12** Len Konopka: +1 transfer when he is ready as his controller's influence phase begins, read then; none from torpor. Black Hand generated. Green. (Test len_konopka_gives_a_transfer_while_ready.)

**80.13** Corpse Minion: its vampire burns blood for intercept as often as it pays in the action (TOM 19960109: no "during", 79.3). X is burned 1 at a time, each use only while the intercept is needed (3.2), so the surplus question (K23) is not opened. A ghoul is no mortal (ANK 20200203-2). Amber (a reader left the surplus open; the reads agree on the rest). (Test corpse_minion_buys_intercept_as_often_as_paid.)

**80.14** Death Seeker: cancels a combat card the opposing minion plays, as it is played; the canceled card's cost is not paid (LSJ 20061207, LSJ 20080630, ANK 20190104, LSJ 20030224). A canceled strike is chosen again (LSJ 20100206). One Death Seeker per vampire each round. Its play on an end-of-round card, counted in that round (ANK 20180910-2), is implemented but untested (ledger row blocked). Green. (Tests death_seeker_cancels_an_opposing_combat_card, death_seeker_cancels_a_strike_and_another_is_chosen.)

**80.15** Fee Stake: Perth: requires an Anarch of capacity 5 or more; it goes on that vampire as the unique Anarch title of Baron of Perth, with no clan's calling vote. Any Methuselah's vampire (D11) may call its +1 stealth burn referendum. During that referendum non-Anarch titles are worth 1 vote less (32.9), a baron's not. The Priscus sub-referendum is untouched (ANK 20180307-1; no Priscus in V5). A contested copy is face down and its action unusable (LSJ 20070808-2). Green. (Test fee_stake_perth_titles_an_anarch_and_its_burn_weakens_other_titles.)

**80.16** IR Goggles: an optional maneuver each combat, any round, for any bearer (vampire or ally). Green. (Test ir_goggles_give_a_maneuver_each_combat.)

**80.17** Institution Hunting Ground: the hunting-ground family: in its controller's unlock phase, a ready vampire they control gains 1 blood, one hunting ground per vampire each turn (6.5). Green. (Test institution_hunting_ground_feeds_a_ready_vampire.)

**80.18** Kindred Intelligence: the top card of the acting Methuselah's crypt is moved to their uncontrolled region. It is not playable with an empty crypt (RTR 20000501, LSJ 20070411-2), read again at resolution. Green. (Test kindred_intelligence_moves_the_crypt_top_and_needs_a_crypt.)

**80.19** Nosferatu Kingdom: the influence-location family: once in its controller's influence phase, locked to move 1 blood from the bank to a Nosferatu antitribu (that clan only) in their uncontrolled region. Green. (Test nosferatu_kingdom_feeds_an_uncontrolled_nosferatu_antitribu.)

**80.20** Righteous Aura: played in the end-of-round window, which is where a combat can leave the opposing minion not ready with the vampire still ready, as with Hell-for-Leather (44.1, 75.3). The card goes on the vampire (in play, not the ash heap) for +2 bleed, one Aura per vampire. It is burned before the vampire leaves the ready region (ANK 20180511-1), implemented but untested (ledger row blocked). Amber (a reader read any combat-card window; the end of the round is the only one where the condition holds). (Test righteous_aura_goes_on_its_vampire_over_a_downed_opponent.)

**80.21** Sense Death: [ani] a younger vampire (D30) of another Methuselah; [val] any minion of another Methuselah; [VAL] adds an optional maneuver. The press and maneuver apply only in the action's own combat, not in a block combat. The mode's Discipline is required (LSJ 20011204-3). Green. (Test sense_death_ani_needs_a_younger_vampire_val_any_minion.)

**80.22** Wrong and Crosswise: [dem] reduces by 2 a bleed directed at the reactor. [DEM] gives +4 votes against, in the polling step only. It is not playable by a vampire voting in favour; the votes force its cast against and stop counting while it is forced in favour (LSJ 20031115, LSJ 20031119). Not in an automatic pass (PIB 20150105). Green. (Tests wrong_and_crosswise_reduces_a_bleed_by_two, wrong_and_crosswise_superior_casts_four_votes_against.)

**80.23** Night Moves: a bleed of the acting Methuselah's predator or prey at +3 stealth ([OBF] +6), the action's own stealth. The bleed amount is kept (a bleed for 2 is still for 2); only the pool lost is capped at 1, as with Protected Resources (LSJ 20030211). Green. (Test night_moves_burns_one_pool_at_most.)

**80.24** Patagia: Flaps Allowing Limited Flight: the card goes on the acting Nosferatu, one per vampire, and gives an optional maneuver each combat. Flight costs 1 blood, once each combat, in its own vampire's combat (ANK 20221021), until the end of the round: a combat Psyche! queues starts without it (LSJ 20021213: a new combat; found in review, the round's bonuses were kept when a combat ended into another). Nothing in the pool reads flight yet, so buying it is a legal no-op (17.4; revised by 84.37: Earthshock reads it). Green. (Tests patagia_goes_on_its_nosferatu_and_buys_flight_once_a_combat, patagia_flight_ends_with_the_combat_psyche_replaces.)

**80.25** Ponticulus: +1 intercept during actions directed at his controller. He locks to give +1 intercept to any Methuselah's Tremere (D11) attempting to block while it needs the intercept (3.2). Immune to non-aggravated damage: none is inflicted (LSJ 20010627). He takes no action and holds no equipment, but blocking is no action, so he blocks. Amber (any Methuselah's Tremere; the reads agree). (Test ponticulus_cannot_act_but_locks_to_give_a_tremere_intercept.)

**80.26** Praxis Seizure: York: requires a Camarilla vampire. If the referendum passes, the card goes on the vampire who called it, if that vampire is still in play, as the unique title Prince of York. If it fails, the card goes to the ash heap. Green. (Tests praxis_seizure_york_titles_the_calling_vampire, fiorenza_savona_doubles_edge_and_card_votes_and_praxis_seizure_titles_her.)

**80.27** Rumble: requires a Sabbat vampire; it targets a ready minion of another Methuselah (a locked one too, D28). The +1 hand size applies in the combat it begins only. The 1 pool is burned if the acting vampire is not ready at the end of the action, after its combats, blocked or not. Amber (the reads agree). (Test rumble_burns_a_pool_if_its_vampire_is_down.)

**80.28** Dark Web Procurer: its +1 stealth action is an equip action (TOM 19960130), not the same action as equipping that card from hand (ANK 20180817). It searches for a non-unique equipment costing 1 or fewer pool, requirements and cost applying; a blood cost is no pool cost (caps read narrowly). The search may find nothing, and the library is shuffled. Amber (a reader read a blood cost as barring the card; an ally cannot pay it either way). (Test dark_web_procurer_finds_only_cheap_non_unique_equipment.)

**80.29** Smash and Grab: requires an Anarch, and the mode's Discipline (LSJ 20011204-3). [ani] burns 2 life from an ally or a retainer, directed at the card's controller (LSJ 20090324); an ally burned this way is burned by a Ⓓ action (ANK 20220507). [dem] bleeds at +1 bleed. [pot] burns a location controlled by the acting Methuselah's predator or prey. Predator's Transformation after it is untested (ledger row blocked). Green. (Test smash_and_grab_burns_life_from_an_ally_or_a_retainer.)

**80.30** Vivienne Géroux: she plays cards requiring basic Celerity and/or Fortitude as an Anarch vampire of capacity 1. She is Anarch for those cards only (ANK 20230417), so An Anarch Manifesto gives her nothing. A ghoul is no mortal (ANK 20200203-2). Her last life on a card and no diablerie as for any ally playing as a vampire (ANK 20180928-1, LSJ 20080512, RTR 19970630). Her controller's vampires get -1 stealth on political actions, read live after a control change. A vampire of her controller gives her to another Methuselah by a Ⓓ action. Amber (her capacity as a vampire, which a reader left open, read as 1). (Tests vivienne_geroux_costs_her_vampires_political_stealth, vivienne_geroux_gets_nothing_from_an_anarch_manifesto.)

**80.31** Vivienne Géroux's steal: only a vampire of another Methuselah may take it, as her text says; it is masked for her controller's vampires (`Requirement::OthersControlThisCard`). Green (found in testing: the steal was offered to her own controller). (Test vivienne_geroux_is_given_and_stolen_only_across_the_table.)

**80.32** Seal of Veddartha: any vampire may equip it. "Only usable by a vampire with capacity above 5" bars only the +1 bleed Ⓓ action; the levels go to any bearer (LSJ 20030607, ANK 20180801). A counter is added in each of its controller's unlock phases while it has 3 or fewer, whatever the bearer's Disciplines (LSJ 20070320-2). Each counter adds one level (two of Dominate, then two of Fortitude), capped at superior. All counters are removed as another minion takes it, its controller's other minions included. Its action fizzles once the Seal is off the acting minion (RTR 19960221). Dual Form copying its levels (ANK 20170226) is untested (ledger row blocked). Amber (the reads agree; a reader asked whether a move to its controller's own minion clears the counters, read yes). (Tests seal_of_veddartha_counters_grant_levels_and_clear_on_a_new_bearer, seal_of_veddartha_bleeds_above_capacity_five_and_gains_counters_to_four.)

**80.33** Tremere Convocation: every Methuselah's Tremere (no "you control") gets +1 stealth on actions requiring Blood Sorcery. Vampires with basic Blood Sorcery play combat cards requiring it at the superior level. Any minion may burn it by a Ⓓ action; a Malkavian (that clan) has -1 stealth on that action. Amber (the reads agree). (Test tremere_convocation_gives_stealth_and_superior_combat_cards.)

**80.34** Malkavian Rider Clause: a reaction, in the polling step before any vote or ballot is cast, not in an automatic pass (PIB 20150105). If the referendum passes, the next referendum a vampire of its player calls passes automatically. There is no "this turn", so it waits for that referendum. There is no polling step in that referendum, so none of its "during a referendum" effects and no margin (LSJ 19980107). Nothing happens if this referendum fails. Amber (the reads agree). (Test malkavian_rider_clause_passes_the_next_referendum_if_this_one_passes.)

**80.35** Vaticination: +3 stealth. At resolution the acting Methuselah looks at every hand, chooses the hand and then the card, which is discarded and replaced unseen (PIB 20121031). The owner chose "Every other Methuselah (Recommended)": the action is directed at every other Methuselah, each hand a target (17.2, 20.1), and any of them may block (`directed_seat(s)`: `at_every_hand`). Red, answered. (Test vaticination_discards_from_any_hand_and_is_directed_at_all.)

**80.36** Fast Reaction: played after a combat between the acting minion and a blocking minion of the player other than this vampire. The vampire locks and enters a new combat with the acting minion, in which the acting minion cannot strike in the first round: no strike's maneuver either (LSJ 20001126), extra strikes still gained (LSJ 19970814-2). [AUS] adds an optional press. It is not playable with a combat already queued (RTR 20020501). The block combat's own after-combat effects, a continuation "as if unblocked" included (Torrent [CEL]), are lost to the new combat, unlocks aside, as with Psyche! (LSJ 19980109, 65.1). The window matrix carries it as an after-combat reaction. Green (the continuation found in testing). (Tests fast_reaction_sends_a_second_vampire_at_the_acting_minion, fast_reaction_loses_the_first_combats_continuation_and_follows_only_the_blocks_combat.)

**80.37** Fast Reaction after a Fast Reaction: "only usable after a combat between a blocking minion you control … and the acting minion". The combat a Fast Reaction queued is between the acting minion and a vampire that did not block, so a second Fast Reaction is not offered after it (`Requirement::BlockerOwn` reads the combat just ended). No ruling settles chaining; a reader read it the same way. Red; the owner confirmed it (2026-10-18: "80.37 is right"). (Test fast_reaction_loses_the_first_combats_continuation_and_follows_only_the_blocks_combat.)

**80.38** Nosferatu Bestial acts the turn it enters: the owner answered "Yes, may act". Nothing on the card bars it; Third Tradition, for example, prints such a bar. Entering with no blood, it may hunt that same turn. Red, answered. (Test nosferatu_bestial_becomes_a_hunting_vampire.)

**80.39** Nosferatu Bestial: the card is put in play and becomes a 2-capacity unique Nosferatu with basic Animalism, of the acting Nosferatu's sect, entering with no blood. It performs no action but a hunt, casts no votes or ballots, and holds no equipment. Its unlock-phase employ of a retainer requiring Animalism from hand works as follows: once each unlock phase (6.6); not an action and not an employ action (LSJ 20080815); not played by the minion (ANK 20201229); the version picked by its own Disciplines (LSJ 20080803); requirements and cost apply, but not as an action's (LSJ 20100725, ANK 20180817); no action's cost reduction (LSJ 20090115-1), other reductions apply (ANK 20230824). Green. (Tests nosferatu_bestial_becomes_a_hunting_vampire, nosferatu_bestial_employs_an_animal_retainer_in_the_unlock_phase.)

**80.40** Nosferatu Bestial's unlock-phase employ is not offered without an employable retainer in its controller's hand. The employ is an in-play ability that names its retainer from hand (`Terms::EmployFromHand`). Green (found in testing: the ability offered with none to employ panicked). (Test nosferatu_bestial_employs_an_animal_retainer_in_the_unlock_phase.)

**80.41** Gran Madre di Dio, Italy: equipment while not in play and both as it is played (LSJ 20080114, RTR 20111202); in play a unique location, not equipment, and an action at it is directed at its controller (LSJ 20090324). A minion its controller's predator or prey puts in play (influenced out, recruited, created) out of the unlock phase is locked. A vampire younger than its bearer burns 1 blood, so with an ally bearer none does (63.4). The owner answered that "younger" means younger "than its bearer". Every way into play counts: Soul Gem of Etrius's vampire and Absimiliard's Army's ghoul too (found in review: both skipped the entry effects, the Gem's vampire its own, Dr. Solomon Grey's pool among them). Red, answered. (Tests gran_madre_locks_a_minion_its_prey_puts_in_play, gran_madre_locks_a_soul_gem_vampire.)

**80.42** Light Intensifying Goggles: moving an aim card the bearer used, canceled too, onto the goggles is asked each time. The owner answered "Ask each time": a may-store interrupt (`Interrupt::MayStoreAim`, protocol purpose `may_store_aim`, protocol 2.1.0). Only the bearer plays a stored aim card, as if from hand, and it is then removed from the game. A card played from the goggles has no "do not replace" (LSJ 20081126) and may be canceled as played (LSJ 20060106). Stored cards are not in hand, so no effect discards or burns them (LSJ 20050118), and they burn with the goggles (LSJ 20021128). The maneuver: once, in the first round of a combat. Red, answered. (Tests light_intensifying_goggles_keep_an_aim_card_its_bearer_used, light_intensifying_goggles_bearer_plays_a_stored_aim_card, light_intensifying_goggles_burn_their_stored_card_as_they_leave_play.)

**80.43** Decapitate: [pot] a hand strike or a melee weapon's strike at +2 damage; the +2 takes the base damage's properties, and adds none to a strike without damage (RTR 19960221, RTR 19970630, TOM 19960225). [POT] is played before the torpor lands: after the damage's preventions, in a new window where the damage as it stands sends the opposing vampire to torpor (`Window::CombatGoingToTorpor`), opened only when its player has a card or ability for that moment, and offering only those: an any-impulse ability had the prevention window just closed (found by the corpus of human games: a recorded game read through The Erciyes Fragments there). It is not playable by a vampire being burned or going to torpor itself. The opposing vampire is burned instead and never goes to torpor. An effect stopping the torpor, played first, bars it (RTR 20010711); that case is untested (ledger row blocked). The owner chose "before torpor lands". Red, answered. (Tests decapitate_burns_a_vampire_instead_of_torpor, decapitate_step_offers_no_any_impulse_ability, light_intensifying_goggles_burn_their_stored_card_as_they_leave_play.)

**80.44** NRA PAC's "an equip action" reads as 78.28 does for Hag's Wrinkles (`ActionCtx::is_equip_action`, one reading for both cards). These count: an equip from hand, Vast Wealth's, or an action card or granted action equipping from the library (a search: Magic of the Smith, Michael Harris, Dark Web Procurer). The equip-from-another-minion action does not count: it moves equipment (rules-digest §3.7). Before this, NRA PAC counted that move. Green (found on the way; the fuzz golden 8b moved with it). (Test nra_pac_unlocks_a_search_equip_but_not_a_move_from_another_minion; twda_round8's nra_pac_unlocks_equippers.)

**80.45** Dual Form [PRO] played by the Great Beast, which has no clan (Call the Great Beast): the copy is "a non-unique vampire with the same clan … as the acting vampire", so it has no clan either. Green (found on the way: the p13 run's crash at 331ce66). (Test dual_form_on_the_great_beast_makes_a_clanless_copy.)

## Batch 81 — Diablerie of an older vampire, the master Discipline (Z-main, 2026-10-17)

**81.1** Diablerie of an older vampire (advanced rules 4): the diablerist's controller may search library, hand and ash heap for a master Discipline card (not one of a Discipline the diablerist has at superior; its other requirements ignored, LSJ 20051116-2) and put it on the diablerist, not played; the library is shuffled after the search, a card taken from the hand is replaced at once, and the blood the diablerie gave has drained before the card raises the capacity. Engine reading: the search is asked as the victim burns, before the window of the cards played on the diablerie (Slake the Thirst, Draught of the Soul); advanced rules §2.1.4 lets such a card come before or after the Discipline card is taken. Owner (2026-10-18): asking it first is enough. (Test a_diablerist_of_an_older_vampire_takes_a_master_discipline.)

## Batch 82 — A block attempt short of the stealth is the blocker declining (owner, 2026-10-17 from a playtest via Z-play, 2026-10-18)

**82.1** A blocker whose attempt is short of the stealth, its impulse come, attempts with another minion or passes; that pass is its decline, effective once the table had its round on the attempt still under way (KRCG News Radio, Night Terrors, Draba: help that reaches the stealth gives the acting Methuselah the impulse, to answer; any play voids the passes before it, the decline included, 1.2, and the blocker is asked again). Everyone passing, the attempt fails and the impulse goes on from the next Methuselah after the blocker in the order (for an undirected action, the predator's attempt), not back to the acting one, whose impulse comes once blocks are declined. Owner (2026-10-18): "if as a blocker you are not matching the acting stealth, you have to either begin another attempt or pass. Passing is declining the block and the impulse does not go back to acting but to the next Methuselah in impulse order … I must still have the possibility to wait for other effects around the table to make my attempt successful … this little 'between attempt and decline' round of impulse is rarely materialized, so the _acting_ should not get the impulse in opportunities off, it's for the other opponents" (ANK, vekn.net 79007-netwar-timing #101567: the third parties play once the acting and the blocking Methuselah both pass; everyone passing, the attempt fails and another may be declared, or the blocker declines). The acting Methuselah's impulse in the open window is its first and the one after each play, never one the passes come round to: those come round once every other seat passed, blocks declined (owner, 2026-10-18, "include the cut"). Only that failure: an attempt failed by an effect (Enchanting Gaze, Forced Confessional, a cancel, Puppet Master) returns the impulse to the acting Methuselah, as after a play (1.2), and an action ended early (The Kiss of Ra) takes no further attempt (owner, 2026-10-18: "This does NOT mean that any other kind of failure (eg. Kiss of Ra) you can still attempt to block"). (Tests a_short_block_attempts_pass_is_its_decline_after_the_tables_round, a_block_attempt_failed_by_an_effect_goes_back_to_the_acting_one, the_kiss_of_ra_leaves_no_other_attempt.)
**82.2** An attempt failed with no play at that moment, not short of the stealth: its blocker locked or gone at the closure (Visions of Zapathasura, 47th Street Royals), a toll left unpaid (Where the Veil Thins), an ally's last life. It is not the blocker's decline (owner, 2026-10-18: "a lock at closure or unpaid toll does not count as decline"): as before 82.1, the open window anew at the acting Methuselah, the blocker's Methuselah asked in its turn to attempt with another minion or decline. (Test a_resigned_block_attempt_goes_back_to_the_acting_one.)

## Batch 83 — The cards on a contested vampire (Z-main, 2026-10-18, from the owner's playtest via Z-play; owner, 2026-10-19: 83.1 confirmed)

**83.1** The cards on a contested vampire (or ally) are out of play with it: nothing printed on them applies (Baltimore Purge's unlock-phase rider is not offered, Laptop Computer's +1 bleed is off), as a contested card's own text (rulebook "Contested Cards", advanced rules §1.13.3, 7.3) and as a set-aside vampire's cards (§6.4.1, Lay Low's template). A unique card among them leaves its own contest until they return, a rival left alone uncontested, as Descent into Darkness's cards do (§1.13.3, 78.54). They return with it as the contest resolves in its favor, unlocked (§6.4.1), and enter their contests again. Yielded, the vampire burns with them and none of their burn riders fires (Soul Gem of Etrius, Charnas the Imp), as Byzar's replacement does not save him from being yielded (§1.13.3). Confirmed (owner, 2026-10-19). (Test the_cards_on_a_contested_vampire_are_out_of_play_with_it.)

## Batch 84 — Round 17 (twenty promo sets completed, the TWDA's steps 1–6): readings graded (Claude, 2026-10-18; owner, 2026-10-19: 84.1 and 84.3 confirmed, 84.2 and 84.4 corrected)

Eighty-one cards (45 library, 36 crypt: 31 defined, 5 text-only), graded per the add-cards skill. Eight blind second readers read the cards' text; two checkers diffed their reads against the code. Results: defects the diff, the ledger's tests, the fuzz and the egress review found, fixed in the round (84.5, 84.6, 84.7, 84.8, 84.9, 84.10, 84.11, 84.12, 84.13, 84.14, 84.15, 84.37, 84.38, 84.39, 84.40); four red, answered by the owner (84.1, 84.3 confirmed; 84.2, 84.4 corrected); one gap recorded, not built (84.5's active Angelo or New Blood).

**84.1** Cold Aura: sets the round's range to long before range is determined; once a range is set no effect resets it that round, other pre-range cards still playable (RTR 19970630, ANK 20180720); an earlier setter has priority (PIB 20120214). Played after another effect set the range, the engine does not offer it at all — its [NEC] optional press (only to end combat, this round only: TOM 19960521) lost with it — as every range-setting effect (`Combat::allows`, Groundfighting's 71.6). The reader kept it playable for its press. Red; the owner confirmed: "correct" (2026-10-19). (Tests cold_aura_sets_long_range_with_an_ending_press, brick_by_brick_sets_the_range.)

**84.2** Waxen Poetica: 2R aggravated, burned after use; not usable against a vampire with Celerity (read as the strike is chosen), an ally, or a retainer — never aimed at a retainer at long range (`Requirement::OpponentVampire` bars the aim: found by the diff). Kept when combat ends before its strike resolves (LSJ 20001127-2: a "strike: combat ends"); a dodged strike was used, so it burns. Red, corrected by the owner (2026-10-19: "kept against an SCE, burns against a dodge"); "burn after use" reads so for Molotov Cocktail's equipment strike too (`Effect::BurnAfterStrike`). (Tests waxen_poetica_burns_after_use_and_spares_celerity, waxen_poetica_is_not_aimed_at_a_retainer.)

**84.3** Botched Move: as the opposing minion plays a card requiring a Discipline, any combat card, an end-of-round one too (LSJ 20070214); a copy among the top two: the card canceled, its cost paid, 1 unpreventable environmental damage (RTR 19970630), the copies burned, the rest left on top in order; not a cancel Dark Influences catches (LSJ 20081120-2); replaced after it resolves. Against a combat card played otherwise than from hand (from the ash heap: Spiritual Guidance), the engine offers it (every play opens its as-played window; "a played card is played"); the reader does not, by 77.11's precedent (Rewind Time: "not one played otherwise than from hand", RTR 20001020). Red; the owner confirmed: "yes" (2026-10-19). (Tests botched_move_cancels_a_card_whose_copy_is_revealed, botched_move_answers_a_press_at_the_end_of_the_round, botched_move_is_not_caught_by_dark_influences.)

**84.4** Alvusia: enters with 1 counter per Tremere and Gargoyle its Methuselah controls, torpid ones counted (D28); a counter after a Tremere or Gargoyle enters play from the influence phase (influenced out); burned at 0. Once each action (the "during" template, 79.1), its burn gives +1 strength to any slave Gargoyle its controller controls at any moment of the action — before or after blocks, acting or blocking, in combat too — for the rest of the action (`Expiry::Action`); or +1 bleed only to a slave Gargoyle performing a bleed action, before resolution (ANK 20231124, LSJ 19970718), before or after blocks are declined. Red, corrected by the owner (2026-10-19: "can be before or after blocks, acting or blocking, even in combat; only used to increase bleed during a bleed action a slave gargoyle is performing"; "the bleed bonus can be given after blocks are declined"). (Tests alvusia_gives_a_slave_gargoyle_a_bleed, alvusia_reaches_after_blocks_and_a_blocking_gargoyle.)

**84.5** Angelo: Blood Brothers play cards requiring Sanguinus as if Angelo were of any circle — of the circle of the Blood Brother playing, for that card and all its effects (LSJ 20101013, ANK 20190228, ANK 20200705-2, LSJ 20070708, RTR 20070707), never for a card played otherwise (LSJ 20050119), a card not requiring Sanguinus (Three's a Crowd), a vampire's ability (Pavlo, Zizi) nor a card in play's effect — the circle requirements carry a `sanguinus` flag, pinned by a test to the card's Disciplines (found in review); otherwise his own circle of one (LSJ 20020609); another Methuselah's Angelo counts. An active Angelo (or New Blood) may choose the circle of one of any vampire, another Angelo's included (ANK 20211113): not built — the engine plays him as his own circle of one, one legal choice among those (ledger `blocked:`). He burns a location as a Ⓓ action; no retainer (LSJ 20050315, LSJ 20010210). Green. (Tests angelo_joins_any_circle_for_sanguinus_and_takes_no_retainers, angelos_circle_is_not_a_vampire_abilitys, circle_requirements_read_angelo_only_on_sanguinus_cards.)

**84.6** Dam the Heart's River: a frenzy card on the opposing minion (ANK 20220409); [qui] +1 to every strike or damaging effect against it this combat — environmental damage, a retainer's and an effect's in combat (Botched Move) included (ANK 20181128), after-combat damage played in combat too (ANK 20170111, LSJ 20071011: Catatonic Fear), none to a non-damaging strike (RTR 19960221), the extra of the base damage's kind (RTR 19970630); [QUI] its strikes cost 1 more blood or life: a strike card's toll in its cost, reductions applying (LSJ 20070829-1), another strike's as declared (LSJ 20090703) — a granted one too (LSJ 20090529), an ally's last life burning it; a toll it cannot pay leaves no strike (D3); it may choose not to strike, a strike committed earlier (Sniper Rifle) included (LSJ 20071017). Green; the retainer, Botched Move, the unpaid toll and the granted strikes found by the diff, Catatonic Fear and the committed strike by the ledger's tests. (Tests dam_the_hearts_river_taxes_and_hurts_the_opponent, dam_the_hearts_river_adds_to_environmental_and_after_combat_damage, dam_lets_a_sniper_rifles_bearer_choose_not_to_strike, dam_the_hearts_river_taxes_strike_cards_reductions_included, dam_the_hearts_river_bars_an_unpaid_strike_and_raises_every_damage, dam_the_hearts_river_adds_nothing_to_a_non_damaging_strike, call_the_wild_hunt_keeps_an_earlier_frenzy_card.)

**84.7** Gianna di Canneto: while another minion of her controller is in combat, she locks and burns herself to burn an equipment on the opposing minion — one committed as its strike (a .44 Magnum's maneuver) voids that strike (PIB 20141026) — or locks during normal strike resolution for 1 unpreventable damage, chosen once strikes are, not after a "strike: combat ends" (ANK 20211130: found by the ledger's tests); the turn she is recruited too (ANK 20180517). Green. (Tests gianna_burns_the_opponents_equipment_or_hurts_it, gianna_burns_a_committed_gun_and_its_strike_fails, gianna_inflicts_nothing_after_a_combat_ending_strike.)

**84.8** Thoughts Betrayed: [dom] 1 environmental damage at the first round's normal strike resolution, through a dodge, not once combat ended first (RTR 19941109, TOM 19951216, PIB 20140324, ANK 20200422); two plays add up (no limit printed: found by the diff); [DOM] no strike card for the combat, a card merely granting a strike allowed, one played at the maneuver for its strike not (ANK 20210928-2); Groundfighting cancels it (LSJ 20050224-2). Green. (Tests thoughts_betrayed_bars_strike_cards_for_the_combat, thoughts_betrayeds_damage_lands_through_a_dodge_not_a_combat_end, thoughts_betrayed_twice_adds_two, groundfighting_cancels_superior_thoughts_betrayed.)

**84.9** Raw Recruit: by a slave; a torpid vampire moved onto the card in play, out of play — out of torpor by resolution, no effect (ANK 20210124), its cards burned (LSJ 20100302-2), burned with the card (LSJ 20021128). The second action: a vampire of its controller's of the clan "this Gargoyle" is enslaved to, read live — the Gargoyle gone, none (found by the diff); an action to put a vampire in play, so a sterile vampire cannot (LSJ 20011212). The Gargoyle made: the held vampire's capacity, with basic Fortitude, Potence and Visceratika and flight, as printed, not unique (LSJ 20100221), of the slave's sect, enslaved to the acting vampire's clan: the slave's directed-action bar and its fight in place of a blocked master (advanced rules §5.7.2; `RECRUITED_SLAVE`, found in grading). Amber (62.14's slave). (Tests raw_recruit_turns_a_torpid_vampire_into_a_gargoyle, raw_recruit_leaving_play_burns_its_vampire.)

**84.10** Victor Pelletier: as he enters play, before any contest (LSJ 20080526, LSJ 20100902), his controller may pay 1, 2 or 3 pool (down to 0: a cost) for primogen, Prince of Paris or Ventrue Justicar — never a title his Methuselah already holds (no contest with oneself, 76.2: found by the diff); a contest he would join, he yields instead: the title lost, whether he or the rival came second. Amber (76.2). (Tests victor_pelletier_buys_a_title_and_yields_a_contest, victor_pelletier_is_not_offered_a_title_already_held.)

**84.11** Filchware's Pawn Shop: any minion equips with a non-location equipment from any Methuselah's ash heap as an undirected +1 stealth equip action (78.28), the card named with the announcement (32.12), gone by resolution: no effect; 1 more pool from its controller's ash heap, 1 less pool or blood from another's; a cost it cannot pay: no effect, the card left (38.5: found by the diff); another Methuselah's card stays theirs, burned to their ash heap (found by the fuzz's card count); not a card playable once a game (LSJ 20040518); the trifle's action even contested (RBK contested-cards). Green. (Tests filchwares_pawn_shop_equips_from_an_ash_heap, filchwares_pawn_shop_fizzles_on_an_unpayable_cost.)

**84.12** Baldesar Rossellini: once in each Methuselah's turn (ANK 20180719-4), while ready, 1 blood or pool off a location or equipment his controller plays, announced with it, spent as it is paid (60.7) — a card another card plays included (Contraband's named equipment, a search: 41.13, found by the diff). Green. (Tests baldesar_takes_one_off_an_equipment_once_each_turn, baldesar_reduces_contrabands_equipment.)

**84.13** Public Enemy: any Methuselah's untitled, non-Red List minion (an ally too) that is infernal or anarch, or that successfully bled for 3 or more in one bleed since its player's last turn — from that turn's end, the current turn counting (LSJ 20020429), the bleed against anyone; one's own minion too, the action then undirected (20.1: found by the diff). Retribution reads the same window, a bleed of more than 1 against its player, once each Methuselah in a game. Green. (Tests public_enemy_red_lists_an_anarch, retribution_strikes_back_at_a_bleeder_once_a_game.)

**84.14** Kestrelle Hayes: +1 stealth on an action directed at any Methuselah — any of them, for one directed at several (PIB 20150526, RTR 20080808: Vaticination, found by the diff) — controlling no ready Sabbat vampire, read live. Porphyry: +2 bleed against a Methuselah controlling an uncontested location, on a minion too. Green. (Test porphyry_and_kestrelle_read_the_target_methuselah.)

**84.15** Leadership Vacuum: out-of-turn, on the player's own turn too; when another Methuselah's ready titled vampire is burned or sent to torpor (`READY_TITLED_LEFT`) — never banished, removed from the game or descended (found in review), X = its votes as it left, bonus votes counted, conditional ones and priscus ballots not (ANK 20211009, LSJ 20040518-2, LSJ 20041025), fixed (counters changed later aside: ANK 20221024); in that Methuselah's unlock phases they burn X pool, the burn check (a ready vampire of capacity 8+ or titled) read first. Amber (the order). (Tests leadership_vacuum_taxes_a_methuselah_who_lost_a_titled_vampire, leadership_vacuum_does_not_count_a_priscus_ballot, leadership_vacuum_is_not_offered_for_a_banished_vampire, leadership_vacuum_burns_before_taxing_a_titled_methuselah, leadership_vacuum_follows_a_titled_vampire_into_torpor.)

**84.16** Legion: on its acting vampire (capacity above 7); its non-sterile bearer names a Master: Discipline card in hand or ash heap with the announcement — an action to put a vampire in play — and the rest of the hand is discarded, replaced after (8.3); the fledgling a 1-capacity non-unique vampire of the bearer's clan and sect with 1 level of the Discipline, a vampire card thereafter (advanced rules §5.1.3), acting, blocking and playing cards only while the bearer is ready. Amber. (Test legion_makes_a_fledgling_from_a_master_discipline.)

**84.17** Spontaneous Power: on any Methuselah's vampire; the Discipline chosen as it is played, after its target (68.8), one the vampire lacks at superior (the master: Discipline card's rule); +1 capacity while in play. Amber (the reader also offered a superior one as a no-op). (Test spontaneous_power_grants_a_chosen_discipline.)

**84.18** Call the Wild Hunt ([ani] and [pro] both): immune to frenzy cards for the combat, not against one played before (LSJ 20040210); [ANI][PRO] X animals named as it is played, discarded together, replaced after (LSJ 20060215), X environmental damage at the initial strike resolution, dodge or not (LSJ 20060217); one each combat. Green. (Tests call_the_wild_hunt_burns_animals_for_damage, call_the_wild_hunt_keeps_an_earlier_frenzy_card.)

**84.19** Increased Strength: +1/+2 to the damaging strikes requiring Potence for the combat — a strike card played using Potence, or a card in play's strike (Heroic Might: LSJ 20081120-2), the extra of the strike's kind (RTR 19970630); copies stack. Green. (Tests increased_strength_raises_potence_strikes, increased_strength_raises_heroic_mights_strike.)

**84.20** Promise of 1528: a Giovanni about to enter combat with a Camarilla vampire (a blocker's moment after Banshee Ironwail's burn: ANK 20210627), targeted by a Camarilla vampire's directed action, or chosen in the terms of a Camarilla vampire's referendum (never one passing automatically: PIB 20150105); locked too; the action ends unsuccessfully at once, nothing more played (RTR 20180719, ANK 20200207), a blocker not locked. Green. (Tests promise_of_1528_ends_a_camarilla_vampires_action, promise_of_1528_comes_after_the_banshee_ironwails_burn.)

**84.21** Provision of the Silsila: after a combat between its Assamite and the minion of the contract that chose it, as the combat began (burned since, or its minion burned: PIB 20150512), the opposing minion not ready and the Assamite ready; blood to capacity, the contract burned; ordered among the after-combat effects (ANK 20230314: Monster, Shadow Boxing). Green. (Tests provision_of_the_silsila_fills_the_contracts_assassin, provision_of_the_silsila_orders_with_other_after_combat_effects.)

**84.22** Hand Contract: on any ready minion, a Black Hand vampire chosen as it is played (28.5), any Methuselah's, in play — none, not playable; that vampire's rush at the bearer unless one Methuselah controls both, read live; the trifle's action even contested. Shakar: on a Black Hand vampire, its rush at a younger non-Black Hand ready vampire of another Methuselah (17.3), offered while its bearer is Black Hand (LSJ 20080604). Amber. (Tests hand_contract_lets_its_chosen_black_hand_rush, hand_contract_needs_a_black_hand_vampire_in_play, shakar_rushes_younger_non_black_hand_vampires.)

**84.23** Guide and Mentor: its search finds an archetype at resolution among those the vampire can take and pay for (9.3), X counted then; the blood gained even with none (PIB 20110914). Amber. (Test guide_and_mentor_searches_an_archetype_onto_its_vampire.)

**84.24** Port Authority: a card discarded by a discard phase action is replaced at its Methuselah's next unlock phase, before anything unlocks (ANK 20200629-2, LSJ 20091208); several copies, one delay. Prophecies of Gehenna: locked to look at the prey's hand, then the prey's, locked; any Methuselah burns it in their unlock phase by revealing their hand, an empty one too. Amber. (Tests port_authority_delays_the_discard_phase_replacement, prophecies_of_gehenna_looks_and_passes_to_the_prey.)

**84.25** The Path of Bone: cards requiring Necromancy cost any Methuselah's Giovanni 1 fewer blood, the mode played using it (38.4) — the clan named Giovanni only; burned by any minion's Ⓓ action, 1 unpreventable environmental damage to an acting vampire, an ally untouched (D16). The Path of the Scorched Heart: on a True Brujah following no path (a printed path, or this card already, bars it), unlocking it; frenzy cards barred on it, one already used staying; while it acts or fights, other Methuselahs' minions play no card requiring Presence, a combo card played using it included. Amber. (Tests the_path_of_bone_discounts_necromancy_and_bites_its_burner, the_path_of_the_scorched_heart_bars_presence_around_its_vampire.)

**84.26** Ruins of Villers Abbey: equipped as equipment, a location in play (LSJ 20080114), moved to a minion of a new controller (RTR 19960112); its vampire burns 1 blood per optional maneuver before range on the first round, as often as paid, the maneuvers for the combat. Amber. (Test ruins_of_villers_abbey_buys_maneuvers_for_the_combat.)

**84.27** Combat cards: Rockheart [vis] half the opponent's strike (rounded up), as struck; [VIS] half of each for the combat, the current strike only halved once (PIB 20120116), one superior a combat; not with no damage pending (LSJ 20001114). Stonestrength: the Discipline used decides the mode (LSJ 20011204-3); fewer points pending, all (RTR 20041202), none carried (ANK 20200318). Lapse: [tem] no maneuver, [TEM] no initial strike, its maneuver neither (LSJ 20001126), additional strikes stay. Earthshock: strength ranged, undodgeable (a dodge played, no effect: LSJ 20030902-2), not against flight, aimed at a flying minion's retainer at long range (LSJ 20060509). Spiritual Intervention, Thin Blood (a burn; a legal no-op at 0 blood), Argent Baton (life burned, not damage; a strike, dodged as any; not a ranged weapon). Green. (Tests rockheart_prevents_half_a_strike, superior_rockheart_prevents_half_the_current_strike, lapse_bars_the_opponents_initial_strike, earthshock_is_not_usable_against_flight, spiritual_intervention_ends_combat, thin_blood_burns_the_opponents_blood, argent_baton_burns_a_werewolfs_life.)

**84.28** Locations and costs: Centralized Background Check, every Methuselah's weapon +1 pool, a blood-cost one too, through the cost layers (LSJ 20040701, Concealed Weapon reading the raised cost); Therbold Realty, its controller's locations 1 less blood or pool, locquipment included (LSJ 20080107); Martino della Passaglia and Dr. Oliver Genet, their reductions however the card is put on them (36.5, 41.13). Green. (Tests centralized_background_check_raises_weapon_costs, therbold_realty_discounts_locations, martino_equips_for_one_less, round_seventeen_printed_action_stats.)

**84.29** Bleed modifiers: Command of the Beast [dom] limited, [DOM] unlimited, a limited one before or after (LSJ 20100218, RTR 20180511-2); Ensemble's X counted as played, then the Daughters lock, X 0 a legal no-op; Dis Pater as the Giovanni's bleed on the prey would succeed (LSJ 20050422), once each action, not limited. Anu Diptinatpa: any Methuselah's bleeding minion, herself included (ANK 20220502), before resolution (ANK 20231124), once each action. Green. (Tests command_of_the_beast_stacks_with_a_limited_bleed, ensemble_bleeds_for_each_unlocked_daughter_and_locks_them, dis_pater_adds_a_bleed_for_seven_ash_cards, anu_diptinatpa_gives_any_bleeding_minion_a_bleed.)

**84.30** Black Sunrise: Sense the Savage Way's rulings (LSJ 20090514, ANK 20181122-2, LSJ 20010714-2, ANK 20180623). As the Crow: after resolution of a successful action, after all combats (PIB 20150915), once a turn per minion. Dive Bomb: a rush (17.3), the maneuver in the combat its resolution starts, the blocker's press in the resulting combat. Green. (Tests black_sunrise_unlocks_a_locked_vampire_to_block, as_the_crow_unlocks_a_flying_minion, dive_bomb_gives_its_blocker_a_press.)

**84.31** Crematorium (any Methuselah's empty torpid vampire, the lock its limit); Morgue Hunting Ground (a ready vampire, one hunting ground a turn); Giulia Giovanni Abruzzina (close range, the turn she is recruited too: ANK 20180517; an earlier setter has priority); Infamous Insurgent (any Methuselah's Red List Anarch, not a card requiring an Anarch: ANK 20220718; infernal, Bahari a bare trait, +2 votes; the unlock once in the phase, in torpor too). Green. (Tests crematorium_burns_an_empty_torpid_vampire, morgue_hunting_ground_feeds_a_ready_vampire, giulia_locks_to_set_close_range, infamous_insurgent_makes_its_bearer_infernal.)

**84.32** Printed vampire stats: Kisha Bhimji -1 stealth on non-hunt actions (a card's hunt a hunt, 72.5); Lucita, younger Sabbat blockers -1 intercept, read live; Echo +1 strength against a Red List minion ("Camarilla:" a label); Reverend Adams -1 stealth on political actions, older blockers (capacity) not locking, an ally locking; Jessica (ADV) +1 stealth on political actions, merged title generated; Mitchell's rescue +1 blood, read as announced and as it resolves (48.8); Genina to torpor instead of burned by a successful blood hunt, +1 stealth on diablerie and undirected actions, one for both; Guillaume Giovanni, +1 hand size per location his controller controls while he is ready (uncontested: LSJ 20080107), +1 stealth recruiting or employing; Hesha Ruhadze (G2) +1 stealth hunting, +1 hunt on any hunt (RTR 20030519, RTR 20180511). Green. (Tests round_seventeen_printed_action_stats, round_seventeen_printed_combat_stats, reverend_adams_blockers_older_stay_unlocked, rescuing_mitchell_costs_three_blood, genina_goes_to_torpor_for_a_blood_hunt, guillaume_counts_his_locations_in_hand_size, jessica_advanced_stealth_on_political_actions, hesha_ruhadze_groups_keep_their_own_text.)

**84.33** Vampire abilities, unlocks: Nu, The Pillar once in his minion phase, inside an action or a polling step too (LSJ 20020309), no card's [cel] mode (LSJ 20050322-2), scarce (apparent clan: LSJ 20011015); Hannibal (ADV) once in each Methuselah's minion phase, in torpor too; Tori Longwood locked after her minion phase without an action announced this turn (32.10; one canceled as played is none, 71.10), in torpor too (LSJ 20011022). Green. (Tests nu_unlocks_once_in_his_minion_phase, hannibal_unlocks_in_another_methuselahs_minion_phase, tori_longwood_locks_when_she_did_not_act.)

**84.34** Vampire abilities, combat and actions: Xaviar, 1 damage prevented each combat, +2 strength for 2 blood once before range, votes kept through a sect change (LSJ 20010714), merged with his ADV (the ADV's sect and title: ANK 20180524); Claudio Severino, combat cards cost an opposing vampire 1 more blood (an ally none), his unlock-phase pool burn while no Gehenna card is in play (in torpor too), merged with his ADV; Joe "Boot" Hill's rush at another Methuselah's werewolf (17.3), the maneuver in the combat it enters; Yong-Sun, the prey's crypt top burned, an empty crypt a legal no-op (73.3); Lambach (ADV) equips the Eye of Hazimel at no cost (an equip action: TOM 19960130); Barbaro Lucchese, two actions (LSJ 20100909), the clan change for good, taint on any minion, his own undirected; Fatima al-Faqadi and White Lily equip a weapon from hand before range, once each combat, in their own combat only (ANK 20221021), not an equip action (LSJ 20080815), requirements and cost as normal; Sascha Vykos +2 hand size once in each turn, after a vampire in combat with him goes to torpor. Green. (Tests xaviar_buys_strength_before_range, claudio_severino_burns_a_pool_without_gehenna, joe_boot_hill_hunts_werewolves, yong_sun_burns_the_preys_crypt_top, lambach_equips_the_eye_of_hazimel_at_no_cost, barbaro_lucchese_taints_and_changes_clan, white_lily_equips_a_weapon_from_hand_before_range, fatima_needs_a_weapon_in_hand, fatima_equips_only_in_her_own_combat, sascha_vykos_gains_hand_size_as_his_opponent_goes_to_torpor.)

**84.35** Tremere slaves: Porphyry and Verbruch carry 62.14's slave text (the directed-action bar read as offered; the fight in place of a blocked Tremere); Verbruch's crypt search as he enters, before any contest (LSJ 20080526); the slave clan generated (`CryptCard::slave`). Green. (Tests verbruch_searches_the_crypt_as_he_enters, porphyry_and_kestrelle_read_the_target_methuselah.)

**84.36** Inert or text-only: Alan Sovereign (ADV) — no investment card is in the pool, his base card neither: his text never applies; Lambach's and the other bases' [MERGED] text with no base card (DESIGN §11.3). Agatha, Djuhah, Mariana Gilbert, Reza Fatir, Salvatore Giovanni: titles, Black Hand and sects generated, nothing to define. Green.

**84.37** Flight is generated crypt data ("Flight [FLIGHT]" in the text: Porphyry, Verbruch, the Gargoyles) and an ally's generated label (Herald of Topheth, found in review), read as a Discipline level wherever gained (Patagia's for the round, Raw Recruit's Gargoyle): As the Crow and Dive Bomb require it, Earthshock is barred against it. Supersedes 16.6's and 62.14's "a bare trait for now" and 80.24's no-op. Green. (Tests as_the_crow_unlocks_a_flying_minion, earthshock_is_not_usable_against_flight, patagias_flight_bars_earthshock, herald_of_tophets_flight_bars_earthshock.)

**84.38** Hesha Ruhadze's groups: the G6 definition bound every group, so G2 Hesha played G6's text; each group now its own (found in the round). Green. (Test hesha_ruhadze_groups_keep_their_own_text.)

**84.39** Coordinate Attacks' strike-card filter missed a strike card played at the maneuver (a committed strike); Thoughts Betrayed [DOM] reads the same filter (found in the round). Green. (Test thoughts_betrayed_bars_strike_cards_for_the_combat.)

**84.40** Found while testing, fixed: a vampire's own strike read a card definition that does not exist (a panic in combat); a New Blood mini-starter recognized by its decklist names, not "New Blood" in a deck's event name (a TWDA deck from "Badalona New Blood 2025"). Green.

## Batch 85 — Round 18 (the cards players asked for, the TWDA's steps 1–3): readings graded (Claude, 2026-10-19; owner, 2026-10-19: 85.1 and 85.2 confirmed)

Thirty-four cards (20 library, 14 crypt: 9 defined, 5 text-only), the first fourteen asked for through the server's unsupported counts (owner, via Z-main), graded per the add-cards skill. Four blind second readers read the cards' text; their reads were diffed against the code. Results: defects found and fixed in the round (85.31, 85.32, 85.33, 85.34); two red, confirmed by the owner (85.1, 85.2).

**85.1** Helicopter: "when a minion equips with the Helicopter, lock it" — the Helicopter, not the minion (the ruling's "it comes locked"); locked however it is equipped, from hand, a search (Magic of the Smith, LSJ 20100119); put on rather than equipped (Alastor, LSJ 20090415), it is not. Taken from another minion as an equip action (`ActionKind::EquipFrom`), the engine locks it: that is equipping. The reader read moving equipment as no equip, so unlocked. Red; the owner confirmed: "yes" (2026-10-19). (Tests helicopter_comes_locked_however_equipped, alastor_puts_a_helicopter_on_unlocked, helicopter_locks_as_equipped_and_unlocks_its_bearer.)

**85.2** Julius: "If Julius goes into torpor, burn him" — he goes to torpor, every trigger of that firing (his own and the cards' "after … goes to torpor" riders: Thomas De Lutrius', Fame's), then burns (`Stat::BurnedAfterTorpor`, after `minions_to_torpor`'s riders). The reader agreed on the order of the event but would burn him before the other simultaneous triggers resolve. No ruling, no precedent. Red; the owner confirmed: "yes" (2026-10-19). (Test julius_burns_once_in_torpor.)

**85.3** Antoinette DuChamp: action cards of every type (action, political, ally, equipment, retainer: 60.4) and strike cards, a dodge included (42.2), requiring Celerity in the mode played (38.4), cost her 1 more blood; Side Strike's dodge yes, its additional strike no. Green. (Test antoinette_pays_more_for_celerity_actions_and_strikes.)

**85.4** Chandler Hungerford +1 intercept blocking a Gangrel; Christine Boscacci −1 intercept during bleed actions; Navar McClaren cannot block an older vampire, read live as he attempts. Green. (Test chandler_christine_and_navar_printed_blocks.)

**85.5** Eurayle Gelasia Mylonas: her action unlocks any minion, herself included (D28); +2 stealth if its Methuselah controls the minion, read live; directed at the controller of another's minion (PIB 20121210); no stealth of its own. `Effect::StealthIfOwnTarget` (Rampage's, now for a minion too). Scarce generated. Green. (Tests eurayle_unlocks_a_minion_with_stealth_for_her_own, le_dinh_tho_is_scarce.)

**85.6** Fergus Alexander: enters combat with a ready locked Sabbat vampire controlled by another Methuselah (Nathaniel Bordruff's filter); a target unlocked by resolution: successful, no effect (ANK 20210124). Amber (Nathaniel's "ready"; the reader agreed a torpid vampire is no combat target). (Tests fergus_enters_combat_with_a_locked_sabbat_vampire, fergus_fizzles_on_an_unlocked_target.)

**85.7** Greensleeves: "If you also control Humo …": Humo is not in the pool, the text inert (Alan Sovereign's precedent, `DURING_ONCE_ELSEWHERE`). Green. (Test greensleeves_offers_nothing_without_humo.)

**85.8** Smudge the Ignored: +1 strength for good after he first diablerizes an older vampire (capacity compared live as it happens, D30; `Minion::diablerized_older`, set by `diablerie_burns`); a younger victim does not count. Green. (Test smudge_gets_strength_after_an_older_diablerie.)

**85.9** Bobby Lemon, Lukas (his title generated), Mirembe Kabbada, Ohanna, Samson: text-only, nothing to define. Green.

**85.10** The Admonitions: put in play, unique; untimed lock abilities of its controller — discard a card (replaced) with a ready Sabbat vampire, draw two with a ready Black Hand one, as far as the library holds (LSJ 20060620), the hand then over its size discarded down — not in a card's as-played window (RTR 20040501, The Barrens'). In its controller's discard phase (a due, as The Uncoiling's), control passes counter-clockwise (to the predator first) to the first Methuselah controlling a ready Sabbat vampire, back to its controller at most; none: burned (`Effect::PassesToSect`). The only such Methuselah keeps it (LSJ 20031121); it stays locked as it passes. Green. (Tests the_admonitions_passes_to_the_next_sabbat_methuselah, the_admonitions_locks_need_sabbat_and_black_hand, the_admonitions_not_as_played_and_draws_what_there_is.)

**85.11** Communal Haven: Cathedral: in its controller's master phase, lock to move blood and equipment from one ready Sabbat vampire of theirs to another, as Heidelberg Castle's (57.7: one direction each use, any blood, the excess to the bank); no retainers; a location, not a haven. Amber (57.7; the reader allowed both directions in one use). `Moves { retainers, sect }`. (Test communal_haven_moves_between_sabbat_vampires.)

**85.12** Conflagration: [pot] 2R, [dai] 1R aggravated, [DAI] 2R aggravated, fixed damage; played, it requires the Discipline used (LSJ 20011204-3, PIB 20130704). Green. (Test conflagration_strikes_by_discipline.)

**85.13** Creation Rites: Third Tradition: Progeny's, Sabbat: by a ready non-sterile archbishop, priscus, cardinal or regent; a 1-capacity Sabbat vampire of the actor's clan as it resolves (78.47), idle this turn, a master Discipline searched; up to 1 blood moved, decided as it resolves (PIB 20150418). Green. (Tests creation_rites_makes_a_sabbat_vampire_of_the_actors_clan, a_burned_childe_is_a_library_card_in_the_ash_heap, a_banished_childe_can_be_influenced_again, kaymakli_nightmares_reshuffles_every_uncontrolled_crypt_card.)

**85.14** Crusade: Chicago, Istanbul, London: by a Sabbat vampire; passed, the card on its caller, if still in play, as that archbishop's title (Praxis Seizure: York's); Istanbul's Tzimisce and London's Nosferatu antitribu unlock in the next discard phase, the clan read then (`RefEffect::PutOnCaller { unlocks_if }`, `Minion::unlock_at_discard_if`). Not called while its player claims that title (85.32). Green. (Tests crusade_chicago_titles_its_sabbat_caller, crusades_unlock_their_clan_at_the_discard_phase.)

**85.15** Dodge: strike: dodge; not replaced until after combat. Green. (Test dodge_is_replaced_after_combat.)

**85.16** Eternal Vigilance: by a ready archbishop, priscus, cardinal or regent, put on the acting vampire itself (not a stand-in); during an action, once (D7), the Sabbat bearer burns 1 blood to unlock and attempt to block — Under Siege's: one already attempting too (ANK 20181122-2), not one that cannot attempt (LSJ 20010714-2), the prey, predator and target restrictions kept (ANK 20180623), the attempt standing if the unlock leaves no target (LSJ 20090514); burned as its vampire goes to torpor. Green. (Tests eternal_vigilance_goes_on_its_titled_actor, eternal_vigilance_unlocks_to_block_and_burns_in_torpor, eternal_vigilance_keeps_the_block_restrictions, ambush_target_unlocking_to_block_still_blocks, forced_attempts_keep_the_blocking_restrictions_and_an_ongoing_attempt, black_sunrise_unlocks_a_locked_vampire_to_block.)

**85.17** Hatch the Viper: the version as employed, fixed (RTR 19941109, ANK 20180612); 1 ([PRO][SER] 2) environmental damage at close range each round with the normal strikes, the retainer its source (ANK 20180612, ANK 20211127), through a dodge, not after a combat ends strike (RTR 19941109), not repeated by additional strikes; the employer prevents 1 damage each combat, any damage. Green. (Tests hatch_the_viper_damages_at_close_range_and_prevents, hatch_the_viper_superior_bites_through_a_dodge, murder_of_crows_does_not_repeat_on_additional_strikes.)

**85.18** Helicopter's unlock: after a successful action of its bearer, it may lock the Helicopter to unlock; a vehicle, one only. Green. (Test helicopter_locks_as_equipped_and_unlocks_its_bearer.)

**85.19** Hidden Lurker: Fast Reaction's mirror — by the acting minion's controller only (LSJ 19990425), another ready unlocked vampire of theirs locks and enters combat with the blocker after the block's combat, the blocker unable to strike in its first round, no strike's maneuver either (LSJ 20001126, LSJ 20021028), extra strikes still gained (LSJ 19970814-2); [OBF] a press for the Lurker, any round; not with a combat queued (RTR 20020501), nor against a blocker gone to torpor (LSJ 20100604-1); the block's combat's end effects lost but an unlock (LSJ 19980109). `QueueCombatAfterBlock { with_blocker }`, `Requirement::BlockerReadyAfterItsCombat`. Green. (Tests hidden_lurker_sends_another_vampire_at_the_blocker, hidden_lurker_only_for_the_acting_methuselah_and_one_queued_combat, fast_reaction_loses_the_first_combats_continuation_and_follows_only_the_blocks_combat, status_perfectus_sends_a_second_anarch_the_acting_minion_cannot_strike.)

**85.20** Improvised Flamethrower: 2R aggravated, once each combat per copy (ANK 20230316); any damage the opposing minion inflicts on its bearer at long range, prevented or not (LSJ 20040802) — its strikes, and its pre-range damage with the range set long before (Outside the Hourglass after Cold Aura, as Neutral Guard or Sniper Rifle would: LSJ 20100709) — burns each copy, its bearer taking 2 aggravated damage, preventable, its own; a dodge or no damage: nothing. No other long-range damage of an opposing minion is in the pool (Pulled Fangs at close range). Amber (the reader's "any damage": the pool's only kinds). `Modifier::BackfiresAtLong`. (Tests improvised_flamethrower_backfires_at_long_range, improvised_flamethrower_backfires_through_a_prevention_not_at_close_range, outside_the_hourglass_triggers_the_flamethrower_at_long_range, sawed_off_shotgun_strikes_once_each_combat_per_copy.)

**85.21** Leverage: during a bleed, with the Edge: the Edge burned as it resolves, +1 bleed outside the limit, before or after a limited one (LSJ 20100218, RTR 20180511-2); no Edge gained this action, one it would get burned — no one holds it (`Effect::NoEdgeThisAction`). Canceled as played, the Edge stays. Green. (Tests leverage_burns_the_edge_for_a_bleed, leverage_goes_with_a_limited_bleed.)

**85.22** Nosferatu Justicar: a ready Camarilla Nosferatu chosen (LSJ 20041026), none and it is not played (LSJ 20100129), never one of its caller's while the caller claims the title (76.2); each Nosferatu +1 vote. Green. (Test nosferatu_justicar_titles_a_camarilla_nosferatu.)

**85.23** NSA Trio: the turn's first combat between an acting vampire and the vampire that blocked it — counted whether the event is in play or not, once each turn (LSJ 20050215; `GameState::vampire_block_combat_turn`) — bars both from ending combat as a strike in round 1 (the strike not offered); an ally, a queued combat (Hidden Lurker's) not eligible. A discard phase action burns it for a Methuselah none of whose minions announced an action this turn (`Seat::acted_this_turn`), whatever became of the action. Green. (Tests nsa_trio_bars_combat_ends_in_the_first_block_combat, nsa_trio_burns_for_a_methuselah_who_did_not_act.)

**85.24** Sacrificial Lamb: [obf] a vampire in its player's torpor, [OBF] Ⓓ one in another's (the superior replaces the target; directed); the actor may take its equipment first, one card at a time as a diablerie's step 2 (a Soul Gem of Etrius not taken, its effect applying: LSJ 19970516), then it burns and the actor gains blood equal to its capacity, read before, the excess ignored; not diablerie; out of torpor by resolution, it fizzles (ANK 20210124). Byzar is not in the pool (LSJ 20100423 read as no burn, nothing gained). `Effect::SacrificeTarget`, `Interrupt::DiablerieEquipment { diablerie: false }`. Amber (the equipment before the burn: diablerie's order; equal with no card keeping the vampire from burning). (Tests sacrificial_lamb_burns_a_torpid_vampire_for_its_capacity, sacrificial_lamb_superior_reaches_another_torpor_region, sacrificial_lamb_leaves_a_soul_gem_and_fizzles_off_torpor.)

**85.25** The Secret Library of Alexandria: may draw a card after a successful bleed of the prey by a minion of its controller, once each action, then discard down (Gostoso's moment). Green. (Test secret_library_draws_on_a_successful_bleed.)

**85.26** Shanty Town Hunting Ground: a hunting ground (its clan, as every clan card's); 1 blood to a ready vampire in the unlock phase, a vampire at capacity a legal pick. Green. (Test shanty_town_feeds_a_ready_vampire.)

**85.27** Storage Annex: not played as the only card in hand (LSJ 20100527); as it is played, a card of the hand put face down on it (`Interrupt::StoreOwnHand`, the `store_own_hand` purpose, protocol 2.5.0), replaced as Fleshforge Chamber's (37.1), seen by its controller, hidden from the others; once each master phase (D7), a hand card exchanged with it, neither replaced, to the controller's hand (PIB 20150224); stolen, the card goes with it and its new controller may take it in hand (LSJ 20001031, PIB 20121220) — a gap: the card then in that hand keeps no owner, so it goes to its taker's ash heap, not its owner's (a hand card's owner is not modeled); left on the Annex, it burns to the ash heap of the one who put it there (LSJ 20021128). Green, the gap on the ledger. (Tests storage_annex_stores_a_card_and_exchanges_it, storage_annex_goes_with_its_card_and_burns_it.)

**85.28** Fergus, Eurayle, Sacrificial Lamb [OBF]: an action at another Methuselah's minion is directed at its controller (20.1, the Ⓓ indicative). Green.

**85.29** Crusade: Istanbul / London's "your next discard phase": the caller's turn's own (a political action is taken in one's turn), swept with Feral Hound's "unlock during the next discard phase". Green.

**85.30** Hatch the Viper put in play by a card rather than employed: its basic version (no level used). Green.

**85.31** Found by the tests: Eurayle's action panicked as it resolved — a granted action's stealth effect not skipped at resolution (`StealthIfOwnTarget`). Green.

**85.32** Found by the diff: Praxis Seizure: York (and now the Crusades) could be called while its player claimed that title, a contest with oneself (advanced rules §1.13.1; 76.2) — `Requirement::NoOwnClaimOf`. Green. (Test crusade_chicago_titles_its_sabbat_caller.)

**85.33** Found while designing: Fleshforge Chamber's stored card was documented "not replaced" while it is (37.1): the doc corrected. Green.

**85.34** Found while designing: a stolen card storing another's card returned that card to the storing card's owner on a burn; Storage Annex records who put it there. Green.

## Batch 86 — The master Discipline search found empty or declined (Z-main, 2026-10-19, tracing harvested records; owner, 2026-10-19: 86.1 confirmed)

**86.1** 81.1's search taking nothing (no master Discipline card the diablerist could take in library, hand or ash heap) or declined (its Pass): the library is shuffled all the same: the controller goes through the library either way (advanced rules 4: "may go through … and then shuffle"); a seat with no card to take is not asked, its library shuffled. Confirmed (owner, 2026-10-19). Found tracing two harvested records whose games diverge at that shuffle (05d1a4123dfe22af, 81e9bf16b18eb65c; 7d7d339). (Test a_diablerist_of_an_older_vampire_takes_a_master_discipline, the younger victim's case aside.)
