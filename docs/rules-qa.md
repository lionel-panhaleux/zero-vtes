# Rules Q&A — ruling reference

Owner rulings (VEKN judge) on VTES rules questions raised while building the
engine, Batches 1–52. Each item is the settled rule, stated as fact; the
question framing, engine mechanics, test names and confirmations are dropped
deliberately: the code and its tests are the reference for how a rule is
implemented, and they point back here through `rules-qa N.M` comments.

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
- **Card text addresses its controller** by default (32.7); **Ⓓ is only a
  reminder** (20.1); **conditions are read live**; **legal no-op plays stay
  offered** (25.5).

## Index

- **Untimed abilities** — 7.7, 29.10, 15.2 (Kalinda)
- **Legacy additions for TWDA decks** — 44.1–44.10
- **Ousts, Life Boon's decision point** — 45.1–45.8
- **Impulse, sequencing, windows** — 1.2, 1.3, 1.5, 2.1, 2.2, 2.3, 2.4, 3.7, 4.1, 5.3, 7.7, 7.8, 7.9, 23.3, 24.2, 26.5
- **Mandatory effects and their ordering** — 6.1, 6.4, 7.8, 7.9, 17.2, 27.1, 28.9, 32.10, 40.5
- **Periodicity ("during X" = once per X, once each …)** — 6.6, 7.4, 9.6, 10.4, 11.2, 15.1, 15.2, 16.1, 36.3, 36.4, 37.1, 38.8, 42.3, 42.9
- **No-repeat-action, "same action"** — 1.4, 6.2, 16.6, 23.2, 25.4, 32.1
- **Action outcomes: fizzle, fail, success, after resolution** — 3.6, 5.8, 5.9, 17.1, 17.3, 17.4, 23.3, 24.1, 24.4, 26.1, 26.4, 28.7, 30.2, 30.3, 34.6, 42.6, 43.1
- **Directed / undirected, targeting, choosing** — 6.8, 17.2, 17.4, 20.1, 24.5, 26.3, 27.2, 32.5, 33.3, 34.8, 35.1, 38.2, 38.6, 39.4, 42.1
- **When choices are made (play vs resolution vs use)** — 3.5, 6.7, 22.2, 26.8, 28.5, 31.2, 31.3, 32.11, 32.12, 33.4, 38.1
- **Bleeds** — 18.4, 23.5, 24.1–24.4, 25.6, 30.5, 31.2, 32.2, 32.4, 32.8, 33.9, 34.4, 34.5, 39.5, 42.3, 42.7
- **Stealth, intercept, blocks** — 1.3, 1.6, 3.2, 3.4, 8.1, 9.6, 15.2, 23.1, 24.5, 25.3, 25.5, 27.3, 30.1, 30.3, 30.4, 23.6, 34.1, 34.2, 35.1, 42.4, 42.8, 48.1, 48.3, 48.5, 48.6, 49.5, 50.4, 52.8
- **Combat structure, strikes, maneuvers, presses** — 1.2, 4.1, 4.3, 4.4, 4.5, 10.1, 10.2, 10.4, 10.5, 14.1, 14.2, 19.5, 21.1, 22.1, 22.3, 26.2, 26.6, 26.7, 27.5, 29.7, 29.8, 29.9, 31.7, 33.6, 42.2, 47.2, 48.2, 48.4
- **Frenzy cards** — 28.11, 29.5
- **Damage, prevention, immunity** — 4.2, 10.3, 14.2, 21.3, 29.1, 29.2, 29.3, 29.4, 30.6, 38.3, 38.8, 40.9
- **Rushes (entering combat as an action)** — 17.3, 26.1, 27.2, 27.5, 38.6, 52.6
- **Referendums, votes, political actions** — 1.1, 3.5, 5.1–5.9, 6.2, 6.3, 15.2, 16.5, 19.3, 19.4, 32.9, 33.2, 35.4, 38.6, 39.1–39.4, 42.4, 49.1, 49.2, 50.5, 50.6, 51.1, 51.2, 51.5, 51.6, 51.8, 52.2, 52.3, 52.5, 52.7, 52.9, 52.10
- **Titles** — 3.1, 13.2, 19.1, 19.2, 19.6, 32.9, 49.4, 52.5, 52.6, 52.9
- **Contests, uniqueness** — 3.1, 7.1, 7.3, 7.5, 8.2, 11.2, 12.3, 19.1, 36.3, 38.7, 39.4, 40.1
- **Control, steals, ousts, removal from game** — 6.9, 7.5, 17.4, 25.1, 27.3, 27.4, 27.7, 28.13, 31.4, 31.5, 31.6, 33.5, 33.7, 38.4, 39.4, 42.7
- **Costs and reductions** — 3.6, 5.9, 9.3, 18.1, 18.2, 23.4, 29.6, 27.6, 36.1–36.5, 38.5, 42.1, 42.2, 42.5, 42.6
- **Replacement, draws, stored and held cards** — 5.5, 7.10, 8.2, 8.3, 8.4, 9.4, 9.5, 11.1–11.3, 12.1–12.5, 22.2, 37.1, 40.4
- **Card destination (ash heap, limbo, lingering effects)** — 21.1, 22.1, 31.6, 46.4, 47.1
- **Equipment and weapons** — 9.1–9.6, 10.1–10.5, 17.4, 18.3, 22.4, 29.7, 35.3, 38.4, 38.5, 38.7
- **Allies, retainers, ghouls** — 22.2, 28.10, 29.2, 32.4, 32.11, 32.12, 32.13, 37.1, 40.10, 42.2, 42.9, 49.3, 50.6
- **Locations, hunting grounds, locquipment** — 6.5, 7.2, 7.4, 17.4, 28.2, 28.6, 31.4, 34.7, 39.4, 49.5, 50.4, 51.4, 52.7
- **Blood, capacity, uncontrolled region, banishment** — 5.7, 7.6, 25.1, 27.7, 28.1, 28.3, 28.4, 32.6, 40.7
- **Torpor, rescue, diablerie, blood hunt** — 3.1, 3.6, 5.2, 5.8, 6.4, 16.2, 28.7, 38.2, 48.8
- **Hunting** — 6.5, 7.7, 17.3, 28.1, 28.8, 32.13, 40.2
- **Master phase, master cards, out-of-turn** — 1.5, 3.3, 6.6, 7.5, 28.10, 32.3, 33.1, 33.8, 36.4, 49.6, 49.7, 50.1–50.3
- **Unlock phase, locking, stun** — 3.7, 6.1, 7.8, 16.5, 21.2, 25.5, 26.5, 31.1, 48.7
- **Discard phase, events** — 8.4, 15.2, 28.12, 33.8, 40.1–40.11
- **Conditions read live; card-text defaults** — 4.3, 9.1, 13.2, 18.4, 19.3, 28.2, 28.6, 29.11, 32.4, 32.7, 39.5, 49.5, 49.7
- **Corruption counters, multi-Discipline cards** — 25.1–25.6, 34.3, 35.2, 38.4, 52.1
- **Printed vampire abilities** — 13.1, 13.2, 14.1, 14.2, 15.1, 15.2, 16.1–16.6, 17.1–17.4, 41.1–41.14, 49.2, 49.3, 52.4
- **Game end, VP, ousting** — 6.9, 18.2, 24.1, 33.2
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

**33.1** Brujah Debate: at the start of each Methuselah's master phase, one mandatory lock per copy; "the oldest Brujah they control" counts torpid vampires (highest capacity among all their Brujah).

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

**48.5** An attempt to block is offered whenever the would-be blocker's Methuselah holds an intercept card it could play and has a library to draw from: the card's replacement may be another (owner, 2026-10-04, from play: an action at 2 stealth, one intercept card in hand, "hope to draw the second"). Refines the reachability pruning (DESIGN §4.3): it counts the hand alone only when nothing can be drawn.

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

Readings no ruling settles, implemented with the slice; each a question.

**50.1** Enkil Cog in its own Methuselah's minion phase: not offered — the vampire takes its actions as usual there, and locking the Cog would add nothing. The Cog is offered only in another Methuselah's phase rotation (49.6). The several-holders reading of 49.6 cannot arise: the Cog is unique, so at most one is in play uncontested. Question (implemented reading).

**50.2** Enkil Cog is offered only while its vampire has an action to take (ready, unlocked, some action legal for it): locking it with none would only lose the effect. Question (implemented reading).

**50.3** Enkil Cog's "a successful bleed against your prey" reads the bleed's target as the card is played, after resolution: a bleed redirected to another Methuselah (Deflection) is not against your prey. Question (implemented reading).

**50.4** Monastery of Shadows' lock is used holding the impulse in the stealth and intercept exchange of a block attempt, as KRCG News Radio's: when stealth is needed (no "even if stealth is not yet needed"), by any Methuselah controlling it (49.5). Question (implemented reading).

**50.5** Political Stranglehold counts every vampire with capacity 8 or more its Methuselah controls, in torpor too; Ancient Influence and Reins of Power choose a ready vampire, locked or not (ready is not unlocked). The chosen capacity is read as the effect applies. Question (implemented reading).

**50.6** Permanent Vacation: the ally removed from the game leaves its card out of the game (not in the ash heap); the cards on it are burned, and nothing keyed to a burn fires. Question (implemented reading).

## Batch 51 — The Endless Dance, implemented readings (2026-10-06)

Readings no ruling settles, implemented with the slice; each a question.

**51.1** Marcos Belegrad: each Methuselah casting votes or ballots against his referendum burns 1 pool once, however many of their sources (vampires, the Edge, a burned card, Business Pressure's pool) voted against; all at once at the tally, before the referendum's effects. Question (implemented reading).

**51.2** Camarilla Conclave is unique and worded as a "during" ability: it is usable once each political action — the +1 stealth (when needed) or the -1 vote to each non-Camarilla vampire, which reaches votes already cast (a vampire's votes stop at 0, ANK 20251125). A political action is enough, no political action card needed (Anarch Revolt's "call a referendum to burn this card"); a referendum of no political action (a diablerie's blood hunt) is not one (owner, 2026-10-07).

**51.3** Confiscation: the chosen vampire takes the equipment only if it can hold it ("a minion can have only one …"); otherwise nothing happens. Question (implemented reading).

**51.4** There is an impulse between two actions: the active Methuselah holds it first, with priority to start the next action; passing it declares the end of their minion phase, and the impulse goes round the other Methuselahs, who may use effects (The Louvre, Paris: ANK 20210608) but not start an action — except Enkil Cog's (49.6). An effect used there returns the impulse to the active Methuselah, whose phase goes on (owner, 2026-10-07). The engine's minion-phase rotation (rules-qa 7.9) is that impulse.

**51.5** Business Pressure resolves as each Methuselah, in impulse order from its player, chooses once how much pool to burn — so how many votes they get — and on which side, then passes to the next; once everyone has chosen, the card resolves: the votes counted and the pool burned at once, the last pool ousting. It is not paid vote by vote (owner, 2026-10-07). [PRE]'s +3 votes a pool are its player's only.

**51.6** Reinforcements canceled by Delaying Tactics: removed from the game, not returned to its owner's hand ("remove this card from the game, even if the action or the referendum fails or is canceled"). Question (implemented reading).

**51.7** Loup: +1 stealth to every Toreador, any Methuselah's; burned after any Toreador's successful action (the one it was played in included), else at the end of the turn; playable when stealth is not yet needed (LSJ 20011214-2). Question (implemented reading).

**51.8** Diana Iadanza cancels any reaction card as it is played during the referendum of a political action she performs (she calls it); its cost is paid. Question (implemented reading).

## Batch 52 — Sabbat V5, the Path of Power and the Inner Voice, implemented readings (2026-10-07)

Readings no ruling settles, implemented with the slice; each a question.

**52.1** "[pot] or [pre]" on one line (the Sabbat V5 layout) is either Discipline at that level, one mode — the counterpart of 25.2's side-by-side combo, which needs both (Iron Glare, Absolute Tyranny): Dominant Personality, Magnetic Authority, Relentlessness and Unthinkable Humiliation are played with either. Question (implemented reading).

**52.2** Absolute Tyranny [POT][PRE]: the -1 vote to each vampire not following the Path of Power and the Inner Voice reaches votes already cast, each vampire's votes stopping at 0 (ANK 20251125), as Camarilla Conclave's (51.2); played by the acting vampire as an action modifier, by another as a reaction. Question (implemented reading).

**52.3** Día de los Muertos: "the first referendum a Sabbat vampire you control calls on this turn" is the first one called after it is played this turn; it passes with no polling step, by a margin of 0 (LSJ 19980107); "only one … in a game" counts every Methuselah's. Question (implemented reading).

**52.4** Věnceslava: her prey "burned 1 or more pool" counts any pool her controller's prey loses during her action, costs paid included (a bleed, a referendum's burn); a bleed deflected to another Methuselah is not the prey's. Question (implemented reading).

**52.5** Cold War: "if this acting vampire is a cardinal or regent, you can choose both" reads the title as the terms are set. Question (implemented reading).

**52.6** Regent's "Sabbat vampires can enter combat with this vampire as a Ⓓ action": another Methuselah's Sabbat vampire, not its own controller's (17.3). Question (implemented reading).

**52.7** Powerbase: Madrid locked in a polling step gives its votes to any titled Sabbat vampire, any Methuselah's — that vampire's controller casts them. Question (implemented reading).

**52.8** Under Siege: offered in the block window to each ready Sabbat vampire of its controller that could attempt to block the action, locked or not, the one already attempting included (ANK 20181122-2); once each action; the +1 intercept lasts the action. Question (implemented reading).

**52.9** The prisci block: each ready priscus casts one ballot, even with no vote of its own; the side with more ballots gets the block's 3 votes, a tie gives none. Question (implemented reading).

**52.10** War of Ages' "call a referendum to burn this card as a +1 stealth political action that costs 1 pool": the pool is paid as the action resolves, as an action card's cost. Question (implemented reading).
