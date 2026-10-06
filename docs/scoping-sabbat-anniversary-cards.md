# Per-card verdicts: Sabbat V5, 25th and 30th Anniversary (2026-10-05)

Companion to `scoping-sabbat-anniversary.md`. `existing` cites a pool card
using the same primitives; `extension` names the primitive to widen; `new`
names the mechanism. `[title]` = Sabbat title layer, `[path]` = path layer
(the library path icon is a play requirement, owner 2026-10-05). "printed" = a `Printed`
modifier read by `GameState::printed_holds` (state.rs), whose conditions are a
closed list (`unreachable!` otherwise).

## Crypt (60)

Without text (no `VampireDef`, like Aintzane Arriaga): Aitana, Berenguela,
Coco Thorn, Cyan Sin, El Monseñor, Glissando, Hekla, Kazuki Hanabira,
Lawbringer, Lika, Loyiso, Lyyli Yösydän, Nadezhda, Onkabetse, Surapong, Suwira,
Valentina Osako, Ximena, Yurena, Zbigniew. Title only [title]: Concordia,
Grażyna, Przemysław, Rasha Tarabzouni.

- Aaradhya | existing [title] | Sybren van Oosten / Keegan: `ActionAfterResolution`, `IsPolitical`+`ActionSuccessful`+`HostLocked` → `UnlockActor`; printed Bleed +1.
- Abderrahim | extension | Daria pattern (`OtherActingControlled`, `StealthNeeded`, `HostBurnBlood(1)`+`AddStealth(1)`); new `Requirement::ActingAllyOrYounger`. Printed Strength +1.
- Aelswith | extension [title] | Marialena's `Hook::HostBlocked` rider; new effect "blocker's controller burns n pool" (`OpposingControllerBurnsPool` reads the combat opponent, none yet). May oust before combat.
- Agnieszka | existing [title] | Rowena's `Strike(CombatEnds)` with Phaibun's cost-then-strike (`HostBloodAtLeast(1)`, `HostBurnBlood(1)`).
- Algirdas | existing [title] | Djeneba's `Stat::StrikeCardToll` to Opponent, requires `OpponentYounger`; live in `combat_card_surcharge` (Aid from Bats ruling).
- Anousha | existing [title] | Printed Strength +1 (Massimiliano).
- Anxo Vilela | extension | Printed Strength +1 with `HasEdge`: add `HasEdge` to `printed_holds`.
- Ashur-uballit | new [title] | +1 starting life for zombie allies/retainers recruited or employed while he is ready. Printed Bleed +1.
- Crossbreaker | existing [title] | `CombatUntimed`, `Limit::PerRound`, `HostBurnBlood(1)`+`AggravatedHands` (Claws of the Dead).
- Damian | extension [title] | Printed Bleed +1 with `ControlsReadyTitled(&[Cardinal])`: add to `printed_holds`, `TitleClass::Cardinal`.
- Devorah | existing | Printed Stealth +1 with `PoolAtMost(5)` (Branimira).
- Diana Iadanza | existing | Meditative Grove / Sword of the Archangel: `CardAsPlayed`, `PlayedCardType(Reaction)`+`ActorCalledReferendum` → `HostBurnBlood(1)`+`CancelPlayed`. Known edge: announced blood not taken by `settle_canceled` (§11.3). Printed Bleed +1.
- Eulogio | new | Look at and reorder top 5 of library (new permutation decision, hidden info); action shell exists (Seraphina, Saankaláxt) + `UnlockActor`.
- Fotini | existing | Richard de Worde's after-successful-bleed window + `HandSizeBonusUntilEndOfTurn(1)` (Ennoia's Theater), if "until your next discard phase" reads the same.
- François Villon | existing | Sri Sansa's `UnlockPhaseOwn` + `HostReady`, target `Younger`, `HostBurnBlood(2)`+`LockTarget`. Printed Bleed +1.
- Frau Schädel | extension | Printed Stealth +1: recruit/employ action-kind condition in `printed_holds`.
- Gostoso | extension | Richard de Worde window + `AddHostBlood(1)`; new `Requirement::DirectedAtPrey`.
- Hartmut Stover | extension [title] | (a) post-tally blood burn for vampires voting against (`VoteAgainstToll` is a pre-cast cost); (b) existing: Mora's `DiscardPhase` + `BurnEdge` + `GainPool(1)`.
- Huldu | existing | Opikun / Roger de Camden: `ForOwnCombatant`, `HostReady`, `PerCombat`, `HostBurnBlood(1)`+`Prevent(1)`; check `NotHost` excludes her as combatant. Printed Bleed +1.
- Ingrid Rossler | extension | Printed `Stat::Transfers` summed in `transfer_bonus` (read at phase start: RTR 20180303).
- Jürgen | existing [title] | `Stat::BlockToll` +1 (Sergio Bueno).
- Kalyani | extension [title] | Printed Stealth −1 on hunt: hunt-kind condition in `printed_holds`. Printed Strength +1.
- Khin Aye | extension [title] | Printed HandSize +1 (Carmelita): condition "predator controls more ready minions".
- Marcos Belegrad | extension | Hartmut's post-tally burn, pool per Methuselah casting (including via a minion).
- Mark Decker | extension | Printed Strength +1 vs werewolf: `OpponentSubtype` in `printed_holds`. Printed Strength +1.
- Modius | existing | Alexa Draper / Yewon Ong: `AnyPollingStep`, discard a master, `AddVotes(1)`, `PerReferendum`.
- Osvaldo | extension | Printed Bleed +1: bleed target seat in `PrintedCtx` + "target controls Toreador or Ventrue".
- Rexton | extension [title] | Printed block bar on the actor (allies, capacity ≤3), read like `BlockToll` in `can_attempt_block`.
- Sakura | extension [path] | Keegan + `OwnTurn` + `HostBurnBlood(1)`; `ActionRequiresClanOrDiscipline` widened to "requires the Path of X".
- Serhat Gunde | existing | Abaddon's `CombatPress` `PerCombat`.
- Silvian Popescu | existing | Sri Sansa's `UnlockPhaseOwn` + `TargetBloodToHost(1)`, target `Controlled`+`TargetTitleIn(&[Prince])`.
- Stanislava | new | Retainer ability suppression in combat; "allies cannot block" (Rexton's bar); Inner Circle title. Printed Bleed +2.
- Verrix | existing [title] | Printed Stealth +1 with `IsBleed` (Martina Srnankova).
- Věnceslava | new [title] | Per-action "prey burned pool" tracking, then `ActionAfterResolution`, `PerTurn`, `GainPool(1)`.
- Xaviar (ADV) | existing, needs ADV flag | Text = Sybren van Oosten's; MERGED ruling moot without his base card. `abo` already in `discipline_key`.
- Üresség | extension [title] | Printed Votes +1 / Bleed +1: `PreyPoolAtMost(10)` condition.

## Sabbat library A (29)

- Abbot | extension | `PutOnActor` +1 stealth, one per vampire (`ActorWithout(Named)`, Heart of the City), `UnlockActor` (Feral Hound), `ActorSectIn(Sabbat)`; widen `HostInterceptOnBleedAtYou` (Guardian Angel) to any action directed at you (LSJ 20061222); count in `intercept_reachable`.
- Absolute Tyranny | new [path] | `AddVotes(3)` by `ActingOrReactingMinion` (Ominous Chorus); signed `VoteGrant` with `Voters::NotPath` (−1); `votes_of` already clamps at 0.
- Abuse of Power | extension [path] | `EnterCombatWithTarget` + new `BlockRider` "opposing controller burns 1 pool at end of combat if exactly one combatant ready" (Hunting the Beast precedent).
- Augury of Doom | extension [path] | Put in play, prey unlock rider (Gate of Acheron), `OnePerMethuselah`, `Hook::PreyOusted` (Khabar: Glory); new: "prey burns 1 pool per torpid vampire", state-based self-burn.
- Black Forest Base | extension | Granted +1 stealth political action (Anarch Revolt), `ControlsReadySect(Sabbat)`, `Limit::PerTurn` (counted on use, LSJ 20091224); new `RefEffect` caller's controller gains 2 pool. Changeling clause inert.
- Burial Site Hunting Ground | extension [path] | `hunting_ground_pick!` + path filter; wraith/zombie ally mode `BloodToTargetToStart(1)`-like with `TargetSubtypeIn` (Shadow Sentinel); one use per unlock (rules-qa 6.6).
- Cardinal Benediction | new [title] | Toreador Justicar shape (`ChooseMinions`, `PutOnChosen`, `RepresentsTitle`); `BarNonCamarillaVotes` parametrised by sect on the `ReferendumDef`.
- Catatonic Fear | extension | [pre] `Strike(CombatEnds)`; [PRE] on-hit after-combat 1 unpreventable damage if close (`PutOnOpponentOnHit` + `PutOnActorAfterCombat`).
- Cold War | new [title] | `ActorTitleIn(Any)`+`ActorSectIn`; new Term "Methuselah or location, both if cardinal/regent"; `RefEffect` burn 3 pool / burn location.
- Cursed Abattoir | extension [path] | Dead Pool bleed half (Controller, ally-acting subtype check); counter half `Hook::ReadyMinionBurnedOrTorpor` with a rider requirement on the burned minion.
- Dance of the Dead | extension [path] | `EnterWithCounters(4)`, `BurnSelfWithoutCounters` (Twilight Camp); widen `ContinuePressForOwnCombatant` (Mob Connections) to maneuver/press with combatant subtype, `PerCombat`.
- Dance with the Devil | extension [path] | Maneuver (`ManeuverToClose`, Slam); [OBF]/[THA] `OptionalCloseManeuver` (Angel's Gift) with a 1-blood cost, once per round.
- Depravity | extension | Master on own vampire, `strength_on_bearer`, stealth during diablerie; widen `NoEquipmentOrRetainers` (Bone Shambler) to "recruit allies or employ retainers".
- Dominant Personality | existing [path] | `AttemptingIntercept(-1)` (Stygian Shroud); `ActionAsAnnounced` + `TargetCannotBlock` (Seduction).
- Dread Mastiff | extension [path] | `RetainerDamage(1)` (Murder of Crows) with close-only flag; [ANI] employer press `PerCombat`.
- Día de los Muertos | new | Referendum auto-pass (§11.3 inert clause; unblocks Amici Noctis, Voter Captivation rows); `NotPlayedThisGame`; per-turn "first referendum by your Sabbat vampire" flag.
- Empires Fall | extension | Neonate Breach's `BurnChosen`: capacity comparison ≥8; `ActorSectIn`.
- Ensnare a Beast | extension [path] | `intercept()` reaction; `StrikeCardToll` as combat rider via `IfBlocks` (Guard Dogs).
- Enthrall | extension [path] | Bleed +1 `IfBleedSuccessful` (Break the Bonds); new optional "burn 1 blood: draw 1 crypt card".
- Fever Pitch | extension [path] | Limited `BleedMod`, one per vampire; `PutOnActorIfBleedSuccessful` (sibling of Dogged Pursuit's); in-play `BurnSelf` + `AttemptFails`.
- Fiendish Tongue | extension | Bleed +1; `AlliesIntercept` (Obedient Flesh) by sect (Anarchs); optional pay-1-blood unlock at discard if Tzimisce (Feral Hound).
- Fiorella, Empty One | extension [path] | Saatet-ta's lock-for-stealth / lock-for-intercept with subtype variants (wraith/zombie); count in `intercept_reachable`.
- Forward Momentum | extension [path] | `Hook::OwnBleedSuccessful` (Alamut) + "not successful" counterpart; `RemoveCounter(2)` + `UnlockActing` (The British Museum); path filter.
- Freakish Conglomeration | extension [path] | Zombie ally, `EnterCombatWithTarget` (Rotting Behemoth), `Effect::Version`; unlock-phase `BurnOwnLife(1)` rider (verify in rider context).
- Gifts From Hereafter | extension [path] | `Actor::OtherOwnMinion` (Cloak the Gathering), acting wraith/zombie ally requirement, limited `BleedMod`; deferred "another vampire burns 1 blood to unlock the acting ally".
- Glancing Blow | extension | `delayed_replacement: Until::NextUnlockPhase` (Old Friends); `PreventStrikeDamage` widened to n points, `StrikeDamagePending`.
- Gravebound Drone | extension [path] | Bone Shambler's `TargetOtherCopy` + "target burns 1 life"; martyr-style `PreventOnTarget(1)` paid in own life; equipment-only ban.
- Heartrender | new [path] | Self-removal from game as an action cost (sequenced after the burn); negated `TargetSubtypeIn`.
- Hedonism | new [path] | Queued combat mid-action between another own vampire and the failed blocker; [FOR]/[PRE] `OtherOwnMinion` in `ActionBlockAttempt` + `AttemptFails` + lock both.

## Sabbat library B (30)

- Hunting the Quarry | extension [path] | [CEL] Telepathic Tracking (`CombatWouldEnd` + `NewRound`); [cel] put on opponent (Disarm), `StillYours` (Phantasmagoria), burn-to-enter-combat action (Haven Uncovered); new "combat about to end" window after `CombatWouldEnd` (ANK 20191219; golden churn).
- Iron Glare | existing | Combo card (Enchanting Gaze); `AddVotes(2)` `PollingStep` (Bewitching Oration); `bleed(2)` (Visions of Gehenna). Auto-pass clause inert.
- Kiss of Cathari | extension [path] | [obf] `Strike(CombatEnds)`; [OBF] after-combat stun if close as a strike rider (generalized `PutOnActorAfterCombat`; `Effect::Stun`).
- Magnetic Authority | extension [path] | After-resolution `ReferendumPassed` (Voter Captivation), `BloodToUncontrolledTarget` (Powerbase: Montreal); printed-sect arm in `holds_for_uncontrolled`; each-uncontrolled blood effect.
- Morbidity | new [path] | Card in play holding opponent's blood; end-of-combat trigger returns it and burns the card; `PerRound`, `OpposingCombatCardsCost(1)` (Terror Frenzy).
- Obedience | new | "About to enter combat with an acting younger vampire" reaction window; `EndBlockedAction`-like (Change of Target); combat queuing (LSJ 19991025).
- Pillars Fall | extension [path] | `EnterCombatWithTarget` (Deep Song), bearer Bleed +1, one per vampire; delayed optional put-on-actor if opposing vampire not ready (`HostCombatOpposingVampireNotReady`, Valerie Diaz).
- Powerbase: Madrid | extension [title] | Unlock-phase `AddCounter` (Carfax Abbey) + `CountersAtMost(3)`; lock: votes per counter to a target titled Sabbat vampire; `steal_this`-style remove-all-counters action (Powerbase: Savannah).
- Private Audience | extension [title] | Closed Session with `BarNonSectVotes(Sect)`; `ActorTitleIn` archbishop/priscus/cardinal/regent.
- Privileged Position | existing apart from [path] | After-resolution `ReferendumPassed`, `LockSelf`, `PreyBurnsPool(1)` (Creeping Sabotage); requirement "2+ ready vampires on the Path" (path version of `ControlsReadyClan`).
- Putrescent Sustenance | extension [path] | `EatAshAlly` (Psychophagia) with "add N life to a zombie ally" branch; not a hunt.
- Rage of Apedemak | extension [path] | Frenzy, `CombatBeforeRange`, `PerCombat` (Terror Frenzy); combat-scoped own hand size; hand-strike +N damage (not `StrengthThisCombat`: melee weapons).
- Regent | extension [title] | Put on target + `RepresentsTitle` (Aranthebes); Haven Uncovered enter-combat with `ActorSectIn(Sabbat)`; diablerie moves the card to a Sabbat diablerist.
- Relentlessness | existing [path] | `AddStealth(1)`; `on_attempt!` + `AttemptFails` (Stygian Shroud).
- Seeds of Terror | extension [path] | As announced (Seduction), [OBF] `TargetCannotBlock`; widen `veil_thins: bool` into a counted block toll (blood only).
- Sentry Signal | existing [path] | `unlock_attempt!` + `AttemptBlock` (Sense the Savage Way); [ANI] `ReduceBleed(3)` (Telepathic Counter). ANK 20181122-2: allow the current attempter.
- Soulgrinder | existing [path] | `Strike(Hand{bonus:2})` (Slam) + `FortitudeProofStrike` (Blood Rage; LSJ 20050628).
- Subversion | existing [path] | Revelation of the Serpent's split mode.
- Swift Cover | existing [path] | Swallowed by the Night, Hide the Mind.
- Target Vitals | new | Aim: strike-companion window, one per strike, +2 if damage inflicted after prevention, no press this round, cancel by discarding 2 combat cards (`CancelableForPool` + Tension in the Ranks terms); LSJ 20080409.
- Terrifying Visage | extension [path] | `Hook::HostBlocked` rider (Marialena), one per vampire; blocker's controller burns N pool (as Aelswith).
- The Hungry Coyote | extension | `Stat::Hunt` read in the hunt (actions.rs) and `HuntAction` hunts; `ClanStat` widened to a sect filter.
- Twisted Bloodhound | existing [path] | Retainer, host enter-combat (`ActorIsHost` + `EnterCombatWithTarget`, Archon), target `NotControlled` (Haven Uncovered).
- Under Siege | existing apart from [title] | `EnterWithCounters` (Gate of Acheron), `unlock_attempt!` + `AttemptBlock` + `AddIntercept` (Second Tradition), `PerAction`, `BurnSelfWithoutCounters`; PIB 20150216 / ANK 20181122-2 need the current attempter allowed.
- Unholy Sacrament | extension [title] | `BloodToUncontrolledTarget(3)`; printed title/sect arms in `holds_for_uncontrolled`.
- Unthinkable Humiliation | extension [path] | Counted block toll (blood or life); all-minions −1 intercept (widen `AlliesIntercept`); LSJ 20020612.
- Wall Street Night | extension | KRCG News Radio + `ActionUndirected` (Warsaw Station), arms in `location_intercept_reach`, target "a minion you control"; investment ability inert.
- Wall of Filth | existing [path] | Combat card put on itself (Pack Alpha), host prevention (Guardian Angel), `BurnSelf` + `PreventNonAggravated(2)` (Soak) / `Prevent(2)`.
- War of Ages | existing apart from [title] | Political put in play (Raising the Portcullis), prey unlock rider (Gate of Acheron), `PreyOusted` + `GainPool` (Khabar: Glory), `BURN_THIS_CARD` (Anarch Revolt); check `PayPool(1)` cost timing.
- Yawp Court | new | Window after a successful political action, before the referendum: combat with a chosen own Sabbat vampire; 2 environmental damage if the actor stays ready, else no referendum.

## Anniversary library (18)

KRCG texts are the current errata'd texts (25th Anniversary changes checked).

- Ancient Influence | new | Per-Methuselah choice of a ready vampire at resolution, gain its capacity in pool, burn 5; `NotPlayedThisGame` (Open War); simultaneous ousts via `oust_batch` (rules-qa 6.9).
- Awe | extension | `Terms::XBlood` (Hidden Strength), `AddVotesX` (Alamut) with a 2X+1 multiplier, `AddVotes` (Ominous Chorus).
- Business Pressure | new | Multi-seat pool-for-votes round (RTR 19960530; advanced 3.7.5.3); payload `PayPoolX` + `MethuselahVotesX` (Oxford University).
- Camarilla Conclave | extension | Unique, `ControlsReadyTitled` + InnerCircle (Papillon); counters (Week of Nightmares); stealth half (Creepshow Casino); new per-counter −1 vote penalty for non-Camarilla in `votes_of`, applied to cast votes too (ANK 20251125).
- Charismatic Aura | extension | [aus] cancel (Anticipation); [AUS] `StrikeKind::CombatEnds` (Majesty); widen `PlayedStrikeByOpponent` to strike or grapple; `PerRound` on the basic mode only.
- Confiscation | extension | `ActorTitleIn(Any)`, `ChooseCard(Equipment)` (Disputed Territory) + `ChooseMinions`; new `RefEffect` chosen vampire takes the equipment.
- Enkil Cog | new | Out-of-turn action (lock to act in any minion phase), "pending action right" kept after a cancel as played (LSJ 20081213-1, ANK 20210124); "successful bleed against your prey" requirement; `PutOnActor` (Khabar: Glory), `ActorCapacityAtLeast(10)`.
- Fear of Mekhet | extension | Put on + lock (Fame), `TargetTitleIn(Justicar, InnerCircle)`; unlock rider scoped to the host's controller; burn 5 blood or be burned (Chantry), else move to another justicar/IC (ANK 20210109) or burn.
- Forced March | extension | Freak Drive (ANK 20220218) / Mirror Walk + `UnlockForBloodIfSuccessful(1)`; per-vampire per-turn play limit (plays.rs treats `PerTurn` as a no-op for plays).
- Instantaneous Transformation | extension | Mirror Walk / Freak Drive; the same limit on the superior mode only.
- Loup | extension | Toreador, `NotPlayedThisTurn` (Ashur Tablets), `ClanStat` stealth layer (Khabar: Community), `waives_needed` (LSJ 20011214-2); new end-of-turn hook and "Toreador performs a successful action" hook.
- Monastery of Shadows | extension | `HandSize(1)` (Elder Library) + lock-for-stealth (Creepshow Casino); new acting-minion capacity ≥8 requirement.
- Permanent Vacation | extension | `ChooseMinions` ally, ready (Banishment); new `RefEffect` remove from game.
- Political Stranglehold | extension | `NotPlayedThisGame`; `GainPerClanVampire` (Consanguineous Boon) widened to capacity ≥8, 3 each.
- Reinforcements | new | Each chosen Methuselah moves up to 3 library cards from ash heap to library (interrupt pick, Marchesa); the card is removed from the game whatever happens.
- Reins of Power | new | The Ancient Influence choice mechanism: gain 6, burn the capacity of the predator's chosen vampire.
- The Louvre, Paris | extension | Toreador unique location, `AnyImpulse` (Puppet Master) + `LockSelf` + `LockTarget`, `steal_this` (Powerbase: Savannah); "you control the Prince of Paris" requirement; verify non-active seats' impulse between actions (ANK 20210608).
- Zillah's Valley | existing | Dreams of the Sphinx / Fourth Tradition: uncontrolled target, `TargetCapacityAtLeast(8)` (Golconda), `BloodToUncontrolledTarget(4)`.
