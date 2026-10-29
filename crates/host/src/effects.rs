//! The engine's effects as the contract's curated ones
//! (`zvtes_api::option::Effect`, docs/protocol.md §7): what an ability
//! or a card's mode does, each effect a kind, an amount, whom it applies
//! to. The effects `add` does not tell (bookkeeping, markers, and effects
//! no curated wording tells yet) are listed untold, by family.

use zvtes::cards::Effect as E;
use zvtes::combat::Range as R;
use zvtes::crypt::CRYPT;
use zvtes_api::ids::{KrcgId, Range, Zone};
use zvtes_api::option::{Effect, EffectKind as K, On, When};

use crate::view::{action_kind, strike};

/// Whom the option's effects name: its target, the card's minion, the
/// minion taking it, the card itself (wire copies, when known).
#[derive(Clone, Copy, Default)]
pub struct Whom {
    pub target: Option<u32>,
    pub host: Option<u32>,
    pub actor: Option<u32>,
    pub card: Option<u32>,
    /// The card played's id: what it puts on its target.
    pub id: Option<KrcgId>,
}

fn of(kind: K) -> Effect {
    Effect::new(kind)
}

fn n(kind: K, n: i32) -> Effect {
    Effect {
        n: Some(n),
        ..of(kind)
    }
}

fn detail(mut e: Effect, d: &str) -> Effect {
    e.detail = Some(d.into());
    e
}

fn cost(mut e: Effect) -> Effect {
    e.cost = true;
    e
}

fn x(mut e: Effect) -> Effect {
    e.x = true;
    e
}

/// A card's mode played or an ability used: its effects, those on its
/// target with the clan the target must be of (Toreador Grand Ball's
/// "ready Toreador").
pub fn mode(m: &zvtes::cards::ModeDef, whom: Whom) -> Vec<Effect> {
    use zvtes::cards::Requirement as Q;
    let mut out = curated(m.effects, whom);
    let filter = m.target.map_or(&[][..], |t| t.filter);
    let clan = filter.iter().find_map(|r| match r {
        Q::TargetClan(c) => Some(crate::view::snake(c)),
        _ => None,
    });
    for e in out.iter_mut().filter(|e| e.on == Some(On::Target)) {
        e.clan = e.clan.take().or_else(|| clan.clone());
    }
    out
}

/// The effects of `effects`, in order.
pub fn curated(effects: &[E], whom: Whom) -> Vec<Effect> {
    let mut out = Vec::new();
    for e in effects {
        add(e, whom, None, &mut out);
    }
    out
}

fn add(e: &E, w: Whom, when: Option<When>, out: &mut Vec<Effect>) {
    let i = |v: u8| i32::from(v);
    let on = |mut e: Effect, on: On| {
        e.copy = match on {
            On::Target | On::Uncontrolled => w.target,
            On::Host => w.host,
            On::Actor => w.actor,
            On::Card => w.card,
            _ => None,
        };
        e.on = Some(on);
        e
    };
    let mut push = |mut e: Effect| {
        e.when = e.when.or(when);
        out.push(e);
    };
    match *e {
        E::IfBleedSuccessful(then) => {
            for t in then {
                add(t, w, Some(When::BleedSuccessful), out);
            }
        }
        E::DeclareAction(k) => push(Effect {
            action: Some(action_kind(k)),
            ..of(K::Action)
        }),
        E::SpendTransfers(v) => push(cost(n(K::Transfers, -i(v)))),
        E::GainTransfers(v) => push(n(K::Transfers, i(v))),
        E::PayPool(v) => push(cost(n(K::Pool, -i(v)))),
        E::PayPoolX => push(cost(x(n(K::Pool, -1)))),
        E::BurnPool(v) => push(n(K::Pool, -i(v))),
        E::GainPool(v) => push(n(K::Pool, i(v))),
        E::PoolIfSuccessful(v) => push(Effect {
            when: Some(When::Successful),
            ..n(K::Pool, i(v))
        }),
        E::PayBlood(v) => push(cost(on(n(K::Blood, -i(v)), On::Host))),
        E::HostBurnBlood(v) => push(on(n(K::Blood, -i(v)), On::Host)),
        E::AddHostBlood(v) => push(on(n(K::Blood, i(v)), On::Host)),
        E::HostBloodToPool(v) => push(detail(on(n(K::Blood, -i(v)), On::Host), "to_pool")),
        E::PoolToHostBlood(v) => push(detail(on(n(K::Blood, i(v)), On::Host), "from_pool")),
        E::BloodToTarget(v) => push(on(n(K::Blood, i(v)), On::Target)),
        E::BloodToUncontrolledTarget(v) => push(on(n(K::Blood, i(v)), On::Uncontrolled)),
        E::PoolToUncontrolledTarget(v) => push(detail(
            on(n(K::Counters, i(v)), On::Uncontrolled),
            "from_pool",
        )),
        E::BurnConviction(v) => push(cost(detail(on(n(K::Burn, i(v)), On::Host), "conviction"))),
        E::BurnTargetConviction(v) => {
            push(cost(detail(on(n(K::Burn, i(v)), On::Target), "conviction")))
        }
        E::PayConviction => push(cost(detail(on(n(K::Burn, 1), On::Actor), "conviction"))),
        E::TargetBurnsBlood(v) => push(on(n(K::Blood, -i(v)), On::Target)),
        E::TargetBloodToPool(v) => push(detail(on(n(K::Blood, -i(v)), On::Target), "to_pool")),
        E::PoolToTargetBlood(v) => push(detail(on(n(K::Blood, i(v)), On::Target), "from_pool")),
        E::HostBloodToTarget(v) => push(detail(on(n(K::Blood, i(v)), On::Target), "from_host")),
        E::TargetBloodToHost(v) => push(detail(on(n(K::Blood, i(v)), On::Host), "from_target")),
        E::ActorBloodToTarget(v) => push(detail(on(n(K::Blood, i(v)), On::Target), "from_actor")),
        E::GraftOnTarget => push(detail(on(n(K::Counters, 1), On::Target), "graft")),
        E::StealTargetVampire => push(detail(on(of(K::Steal), On::Target), "vampire")),
        E::CastTargetVotes(favor) => push(Effect {
            favor: Some(favor),
            ..on(of(K::Votes), On::Target)
        }),
        E::StealBloodToHostX => push(detail(x(on(n(K::Blood, 1), On::Host)), "from_target")),
        E::StealBloodFromTarget(v) => push(on(n(K::Steal, i(v)), On::Target)),
        E::StealPoolFromTarget(v) => push(detail(on(n(K::Steal, i(v)), On::Target), "pool")),
        E::AshCardOfToHand(_) => push(Effect {
            to: Some(zvtes_api::ids::Zone::Hand),
            ..detail(of(K::Move), "political_from_ash_heap")
        }),
        E::ActionCardToHand => push(Effect {
            to: Some(zvtes_api::ids::Zone::Hand),
            ..detail(of(K::Move), "action_card")
        }),
        E::BloodToActing(v) => push(on(n(K::Blood, i(v)), On::Acting)),
        E::BloodToChosen(v) | E::HuntingGroundBlood(v) => push(on(n(K::Blood, i(v)), On::Chosen)),
        E::BloodToTargetToStart(_) => push(detail(on(of(K::Blood), On::Target), "to_start")),
        E::BloodPerHostage => push(detail(n(K::Blood, 1), "per_hostage")),
        E::HuntingGroundTwoOldest => push(detail(n(K::Blood, 1), "two_oldest")),
        E::BloodToCard(v) => push(on(n(K::Blood, i(v)), On::Card)),
        E::EatAshAlly { blood, unlock } => push(detail(
            on(n(K::Blood, i(blood)), On::Host),
            if unlock {
                "ash_ally_unlock"
            } else {
                "ash_ally"
            },
        )),
        E::EatAshAllyToTarget(v) => push(detail(on(n(K::Life, i(v)), On::Target), "ash_ally")),
        E::BurnOwnLife(v) => push(on(n(K::Life, -i(v)), On::Host)),
        E::HuntingGroundLife(v) => push(on(n(K::Life, i(v)), On::Chosen)),
        E::TargetBurnLife(v) => push(on(n(K::Life, -i(v)), On::Target)),
        E::CounterToTargetBlood => push(detail(on(n(K::Blood, 1), On::Target), "from_counter")),
        E::CounterToPool => push(detail(n(K::Pool, 1), "from_counter")),
        E::CountersToPool => push(detail(of(K::Pool), "all_counters")),
        E::BurnSelf | E::BurnSource => push(on(of(K::Burn), On::Card)),
        E::BurnTargetCard => push(detail(on(of(K::Burn), On::Target), "card")),
        E::BurnTargetMinion => push(on(of(K::Burn), On::Target)),
        E::AddStealth(v) => push(n(K::Stealth, i(v))),
        E::ReduceStealth(v) => push(n(K::Stealth, -i(v))),
        E::HuntStealthUntilUnlock(v) => push(detail(n(K::Stealth, i(v)), "on_hunts")),
        E::StealthOnNextAction(v) => push(detail(n(K::Stealth, i(v)), "next_action")),
        E::AddIntercept(v) => push(n(K::Intercept, i(v))),
        E::AttemptingIntercept(v) => push(n(K::Intercept, i32::from(v))),
        E::Strike(k) | E::CommittedStrike(k) => push(Effect {
            strike: Some(strike(k, None)),
            ..of(K::Strike)
        }),
        E::CommitStrike => push(detail(of(K::Strike), "commit")),
        E::WeaponStrikePlus(v) | E::CommittedWeaponStrikePlus(v) => {
            push(detail(n(K::Damage, i(v)), "weapon_strike"))
        }
        E::DamageTarget(v) => push(on(n(K::Damage, i(v)), On::Target)),
        E::DamageOpponentByStrength => push(detail(of(K::Damage), "opponent_strength")),
        E::EnvironmentalDamage { n: v, .. } | E::EnvironmentalDamageThisRound(v) => {
            push(detail(n(K::Damage, i(v)), "environmental"))
        }
        E::StrengthThisRound(v) => push(detail(n(K::Strength, i(v)), "this_round")),
        E::StrengthThisCombat(v) => push(n(K::Strength, i(v))),
        E::Prevent(v) => push(n(K::Prevent, i(v))),
        E::PreventGunStrikeAllowance(v) => push(detail(n(K::Prevent, i(v)), "guns")),
        E::PreventNonGun(v) => push(detail(n(K::Prevent, i(v)), "not_guns")),
        E::PreventNonAggravated(v) => push(detail(n(K::Prevent, i(v)), "non_aggravated")),
        E::PreventXPlus(v) => push(detail(n(K::Prevent, i(v)), "x_plus")),
        E::PreventThisRound(v) => push(detail(n(K::Prevent, i(v)), "this_round")),
        E::PreventEachRound(v) => push(detail(n(K::Prevent, i(v)), "each_round")),
        E::PreventOnTarget(v) => push(on(n(K::Prevent, i(v)), On::Target)),
        E::PreventOnTargetXPlus(v) => push(on(detail(n(K::Prevent, i(v)), "x_plus"), On::Target)),
        E::PreventNonAggravatedOnTarget(v) => push(on(
            detail(n(K::Prevent, i(v)), "non_aggravated"),
            On::Target,
        )),
        E::PreventStrikeDamage | E::PreventOpposingStrikesThisRound => {
            push(detail(of(K::Prevent), "strike_all"))
        }
        E::PreventStrike { n: v, aggravated } => push(detail(
            n(K::Prevent, i(v)),
            if aggravated {
                "strike"
            } else {
                "strike_non_aggravated"
            },
        )),
        E::ImmuneStrikeNonAggravated => push(detail(of(K::Immune), "strike")),
        E::ImmuneEffectNonAggravated => push(detail(of(K::Immune), "effects")),
        E::Press | E::OptionalPress => push(of(K::Press)),
        E::ContinuePressForOwnCombatant => push(detail(of(K::Press), "continue")),
        E::Maneuver | E::OptionalManeuver => push(of(K::Maneuver)),
        E::SetRange(r) => push(Effect {
            range: Some(match r {
                R::Close => Range::Close,
                R::Long => Range::Long,
            }),
            ..of(K::Range)
        }),
        E::EndCombat => push(of(K::EndCombat)),
        E::EnterCombatWithTarget => push(on(of(K::EnterCombat), On::Target)),
        E::PutOnTarget => push(Effect {
            card: w.id,
            to: Some(Zone::InPlay),
            ..on(of(K::Move), On::Target)
        }),
        E::UnlockActor | E::UnlockHost | E::UnlockSelf => push(on(of(K::Unlock), On::Host)),
        E::UnlockTarget => push(on(of(K::Unlock), On::Target)),
        E::UnlockActing => push(on(of(K::Unlock), On::Acting)),
        E::UnlockActingIfSuccessful(_) => push(Effect {
            when: Some(When::Successful),
            ..on(of(K::Unlock), On::Acting)
        }),
        E::HostBurnBloodToUnlock => push(detail(on(of(K::Unlock), On::Host), "burn_blood")),
        E::LockHost => push(on(of(K::Lock), On::Host)),
        E::LockTarget => push(on(of(K::Lock), On::Target)),
        E::LockPreyVampires(v) => push(on(n(K::Lock, i(v)), On::Prey)),
        E::ActionFails => push(detail(of(K::Fails), "action")),
        E::FailReferendum => push(detail(of(K::Fails), "referendum")),
        E::CancelPlayed { .. } => push(of(K::Cancel)),
        E::TargetLeavesTorpor => push(detail(on(of(K::Move), On::Target), "leaves_torpor")),
        E::TargetLeavesIncapacitated => {
            push(detail(on(of(K::Move), On::Target), "leaves_incapacitated"))
        }
        E::GiveToPrey => push(detail(on(of(K::Move), On::Prey), "give")),
        E::StoreFromPreyAsh => push(detail(on(of(K::Move), On::Prey), "from_ash_heap")),
        E::CryptNamedToUncontrolled(name) => push(Effect {
            card: CRYPT.iter().find(|c| c.name == name).map(|c| c.id),
            to: Some(zvtes_api::ids::Zone::Uncontrolled),
            ..of(K::Move)
        }),
        E::AddVotes(v) => push(n(K::Votes, i(v))),
        E::MethuselahVotes { n: v, favor } => push(Effect {
            favor: Some(favor),
            ..n(K::Votes, i(v))
        }),
        E::MethuselahVotesX { per, favor } => push(Effect {
            favor: Some(favor),
            ..x(n(K::Votes, i(per)))
        }),
        E::NonCamarillaVotesMinus(v) => push(detail(n(K::Votes, -i(v)), "non_camarilla")),
        E::TargetVotesPerCounter => push(detail(n(K::Votes, 1), "per_counter")),
        E::CallReferendum(_) => push(of(K::Referendum)),
        E::GrantDiscardAction => push(detail(n(K::Action, 1), "discard_phase")),
        E::SpendMasterPhaseAction => push(cost(detail(n(K::Action, -1), "master_phase"))),
        E::HuntAction => push(of(K::Hunt)),
        E::HandSizeBonusUntilEndOfTurn(v) => push(n(K::HandSize, i(v))),
        E::SearchEquip(_) | E::SearchEquipFree(_) | E::SearchEquipForTarget => {
            push(detail(of(K::Search), "equipment"))
        }
        E::DrawThenBottom => push(detail(of(K::Draw), "then_bottom")),
        E::DrawCryptThenRemove => push(detail(of(K::Draw), "crypt_then_remove")),
        E::DrawCryptBloodIfBlackHand(v) => push(detail(n(K::Draw, i(v)), "crypt_black_hand")),
        E::HandToBottomX => push(Effect {
            to: Some(zvtes_api::ids::Zone::Library),
            ..detail(x(of(K::Move)), "hand_to_bottom")
        }),
        // Paul Cordwood's cost.
        E::RemoveCryptTop => push(cost(Effect {
            to: Some(zvtes_api::ids::Zone::Removed),
            ..detail(of(K::Move), "crypt_top")
        })),
        E::DrawThenDiscardDown(v) => push(detail(n(K::Draw, i(v)), "then_discard_down")),
        E::BleedMod { n: v, .. } => push(n(K::Bleed, i(v))),
        E::ReduceBleedX => push(x(n(K::Bleed, -1))),
        E::RemoveCounter(v) => push(on(n(K::Counters, -i(v)), On::Card)),
        E::RemoveCountersX => push(x(on(n(K::Counters, -1), On::Card))),
        E::AddCounter(v) => push(on(n(K::Counters, i(v)), On::Card)),
        E::PoolToCounterPlusBank { pool, .. } => {
            push(detail(on(n(K::Counters, i(pool)), On::Card), "bank"))
        }
        E::PoolToCounter(v) => push(detail(on(n(K::Counters, i(v)), On::Card), "from_pool")),
        E::PreyPoolToCounter(v) => {
            push(detail(on(n(K::Counters, i(v)), On::Card), "from_prey_pool"))
        }
        E::PayActionWithCountersX => push(cost(x(on(n(K::Counters, -1), On::Card)))),
        E::CounterToTarget(_) => push(on(n(K::Counters, 1), On::Target)),
        E::CountersToUncontrolledTarget => push(on(of(K::Counters), On::Uncontrolled)),
        E::ReduceActionBlood(v) => push(detail(n(K::Cost, -i(v)), "blood")),
        E::ReduceActionCost(v) => push(n(K::Cost, -i(v))),
        E::CoagulateFromPicks { prevent: true } => push(x(of(K::Prevent))),
        E::FleshOfMarble { aggravated } => push(detail(
            of(K::Prevent),
            if aggravated {
                "beyond_first"
            } else {
                "beyond_first_non_aggravated"
            },
        )),
        // Told by no curated effect (docs/protocol.md §7), each named so
        // that a new effect is told above or listed here, by its family
        // (owner, 2026-10-25).
        // Play (a card played resolving).
        E::ActingBloodToNewUpTo(..)
        | E::ActingControllerBurnsPool(..)
        | E::ActorTakesAggravated { .. }
        | E::AshurTablets
        | E::AutoPassFirstSabbatReferendum
        | E::BankBloodAmongUncontrolled { .. }
        | E::BeastPresses { .. }
        | E::BecomesAlly(..)
        | E::BecomesVampire { .. }
        | E::BloodToEachUncontrolledOfSect(..)
        | E::BloodToTargetFull
        | E::BloodToUncontrolledCircle
        | E::BurnChosenInPlay
        | E::BurnLibraryTop(..)
        | E::BurnOthersLabeledOnTarget(..)
        | E::BurnRangedWeaponsOnTarget { .. }
        | E::ChooseActor
        | E::ChooseOwnOfClan(..)
        | E::ChooseSecondOfClan(..)
        | E::ChooseTarget
        | E::ChooseTermsMinion
        | E::ConvictionOnTarget
        | E::CountersFromActorCapacity
        | E::CountersFromTargetCapacity
        | E::CryptClanToUncontrolled(..)
        | E::CryptTopToUncontrolled
        | E::DerangeTarget
        | E::DescendActor
        | E::DiscardFromAllHands
        | E::DiscardFromTargetHand
        | E::DiseaseTarget
        | E::DualFormCopy
        | E::EnterWithCounters(..)
        | E::EnterWithCountersPerOwnAnarch
        | E::EnterWithCountersPerOwn { .. }
        | E::EntersLocked
        | E::FourthCycle(..)
        | E::FuryEnters
        | E::GainPerUnlockedCircleMember
        | E::GameOfMalkav
        | E::GiveBoonPool
        | E::GreatBeastAtEleven
        | E::HoldTargetVampire
        | E::If { .. }
        | E::KaymakliNightmares
        | E::LaibonOrCryptDraw(..)
        | E::LastingOnActor
        | E::Layer(..)
        | E::LockPlayedCard
        | E::MergeWithBase
        | E::MorbidityTake(..)
        | E::MoveBloodTargetToPoolX
        | E::MoveBloodXToUncontrolledTarget
        | E::NextReferendumPasses
        | E::OwnReadyVampiresBloodToPool
        | E::PetraResonance
        | E::PoolToVampires
        | E::PutInPlay
        | E::PutOnActor
        | E::PutOnActorNow
        | E::PutOnActorNowStacking
        | E::PutOnActorRemembering
        | E::PutOnOpponent
        | E::RedeemAshVampire
        | E::RememberTargetSeatsVotes
        | E::RemoveBleedTargetAshCards(..)
        | E::RemovePreyAshToBurnPool { .. }
        | E::RetainerToStartingLife
        | E::RevealCryptTop { .. }
        | E::RotschreckEndsCombat
        | E::SearchLibraryToHand(..)
        | E::SearchLibraryToHandOrHavenOnActor(..)
        | E::SearchPutInPlayUpToX { .. }
        | E::StealLocation
        | E::StealTargetAlly
        | E::StealTargetCard
        | E::StolenToReadyForBlood(..)
        | E::StoreFromOwnHand
        | E::StoreFromHandX(..)
        | E::Stun
        | E::SudarioRefraction
        | E::TakeBleedForTarget
        | E::TargetControllerBurnsPool(..)
        | E::TargetSeatTakesCard
        | E::TrainInUncontrolled { .. }
        | E::TrapPresses
        | E::UnlockActorIfClanCapacity { .. } => {}
        // Shared (a play or an in-play ability).
        E::ActorBecomesBlackHand
        | E::ActorBecomesClan(..)
        | E::ActorBloodToCapacity
        | E::ActorLasting(..)
        | E::ActorLooksAtLibraryTop(..)
        | E::AllySkipsNextUnlock
        | E::AshToLibraryTop { .. }
        | E::BurnActorBlood(..)
        | E::BurnEdge
        | E::BurnRememberedContract
        | E::BurnTargetCryptTop
        | E::BurnTargetOnSecondAction
        | E::CircleBloodMoves { .. }
        | E::EmployNamed
        | E::EquipNamed
        | E::HandSizeUntilUnlock(..)
        | E::LockActor
        | E::LockOwnReadyOfClan(..)
        | E::MayBleedAgainThisTurn
        | E::PeekNeighborHands
        | E::PeekActingControllersHand
        | E::PreyBurnsPool(..)
        | E::RemoveTargetForPool
        | E::SacrificeTarget
        | E::StoreTopCards(..)
        | E::TaintTarget
        | E::TakeControlOfTargetUntilUnlock
        | E::TargetBecomesBlackHand
        | E::TargetBecomesRedList
        | E::TargetBecomesSect(..)
        | E::ActorBecomesSect(..)
        | E::TargetLasting(..)
        | E::UnlockChosen
        | E::TargetRetainerBurnsLife(..)
        | E::UnlockActorAtDiscard => {}
        // Ability (an in-play ability used).
        E::ActingControllerDiscardsAtRandom
        | E::ActionCardToLibraryShuffled
        | E::ActorTakesAnyAction
        | E::AddInterceptX
        | E::AddVotesX
        | E::AtCountersBurnAndGain { .. }
        | E::BleedToRemembered
        | E::BloodToTargetX
        | E::BurnSelfAtCounters(..)
        | E::BurnSelfWithoutCounters
        | E::BurnTargetHostages
        | E::BurnUnlessControllerPays(..)
        | E::CardChoosesTarget
        | E::ChooseTargetThisTurn
        | E::ContinuePressEachCombat
        | E::DiscardAtRandom(..)
        | E::EnterCombatWithActing
        | E::ExchangeForAnarchCard
        | E::FailsIntoCombatWithHost
        | E::GainConviction { .. }
        | E::GainMasterPhaseAction
        | E::GhoulFromLibraryTop(..)
        | E::GiveBloodToOthers
        | E::GiveBloodToOwnAnarch
        | E::HostAsIfUnlockedForAuspex
        | E::HostBleedThisTurn(..)
        | E::HostBloodToUncontrolledTarget(..)
        | E::HostBurnLife(..)
        | E::HostLeavesTorpor
        | E::HostSuperiorChosenUntilMaster
        | E::HostVotes(..)
        | E::LeaveIncapacitated
        | E::LockSelf
        | E::LockTargetCard
        | E::LookAtLibraryTop(..)
        | E::LookAtTurnLibraryTop
        | E::MayBurnLookedMaster(..)
        | E::MarkBleedToll
        | E::MayCancelCombat
        | E::MoveBetweenVampires(..)
        | E::OthersHandSmallerIfSame(..)
        | E::OutOfPlayHostGainsBlood(..)
        | E::OwnClanBurnsBlood(..)
        | E::PayPoolPerOtherEvent
        | E::PeekHand
        | E::PeekHandsOfReadyClan(..)
        | E::PeekHostControllersHand
        | E::PeekOpponentControllersHand
        | E::PredatorTakesControl
        | E::PreyUncontrolledBloodToPool(..)
        | E::RaiseAllyWhenEmpty(..)
        | E::RaiseNamedAllyFromAsh(..)
        | E::RemoveCryptTopAnarch(..)
        | E::RemoveOwnAshCards(..)
        | E::ReturnHost { .. }
        | E::RevealCryptTopForCopy { .. }
        | E::RevealOwnHandToAll
        | E::RevealTopCard { .. }
        | E::SearchMageAlly
        | E::ShuffleHandDrawUp
        | E::SlaveTakesCombat
        | E::SpendDiscardPhaseAction
        | E::StealPoolFromBoonTarget
        | E::StoreFromOwnAsh
        | E::StoredToLibraryTop
        | E::TakeBleedGain
        | E::TakeControlOfHost
        | E::TargetEntersCombatWithActing
        | E::TargetHostControllerTakesControl
        | E::TargetSeatDiscardsAtRandom(..)
        | E::TargetSeatTakesSourceAlly
        | E::TargetToUncontrolled
        | E::ToggleLockTarget
        | E::WatchDamagesActing(..)
        | E::YawpCourtCombat => {}
        // Action (the action underway).
        E::ActingSkipsNextUnlock
        | E::ActionLayer(..)
        | E::AggravatedHandsThisAction
        | E::AlliesCannotBlock
        | E::AnimalBoostIfSuccessful { .. }
        | E::AttemptBlock
        | E::AttemptFails
        | E::AttemptFailsBarSeat
        | E::AttemptingBurnsBlood(..)
        | E::AttemptingToTorpor
        | E::BarSectVotesInReferendum(..)
        | E::BleedModX { .. }
        | E::BleedPerPick
        | E::BlockToll(..)
        | E::BlockTollBlood(..)
        | E::BlockerBurnsBloodOrLife(..)
        | E::BurnActingMinion
        | E::BurnForInterceptThisAction
        | E::BurnIfNoBlock(..)
        | E::CancelAction
        | E::CircleIntercept { .. }
        | E::ContinueAfterResolution
        | E::ContinueUnblocked
        | E::DamageAfterResolution(..)
        | E::DawnOperation { .. }
        | E::DeferReactionReplacement
        | E::DistributeMargin { .. }
        | E::EndActionIfBlocked
        | E::EndBlockedAction
        | E::EndBlockedActionLocked
        | E::FailBlockForCorruption
        | E::GainBloodPerMargin
        | E::HuntBonusThisAction(..)
        | E::IfBlocks(..)
        | E::LockAttempting
        | E::LockBlocker
        | E::LockFailedBlockers
        | E::LockIfNoBlock
        | E::LockPicks
        | E::MayCancelCombatIfBlocked
        | E::NextReactionCosts(..)
        | E::NoDisciplineCardsThisAction
        | E::NoEdgeThisAction
        | E::NoLockForBlock
        | E::NoMoreModifiers
        | E::NoUnlockThisAction
        | E::PactCountersOnActing(..)
        | E::PerfectClarity { .. }
        | E::PicksBurnBlood
        | E::PoolIfAttemptingDoesNotBlock
        | E::PutOnActingIfBlocks
        | E::PutOnActorIfBleedSuccessful
        | E::PutOnActorIfNoBlock
        | E::QueueCombatAfterBlock { .. }
        | E::QueueCombatWithAttempting
        | E::QueueCombatWithLast
        | E::RedirectBleed
        | E::RedirectBleedToActingPrey
        | E::ReduceBleed(..)
        | E::ReduceBleedToZero
        | E::ReduceStealthToZero
        | E::SmallMonstersCannotBlock { .. }
        | E::StealthForNextActions
        | E::StealthPerPick
        | E::StrengthThisAction(..)
        | E::StrikeTollIfBlocks(..)
        | E::TakeOverAction
        | E::TargetAttemptsBlock(..)
        | E::TargetBurnsBloodForIntercept
        | E::TargetCannotBlock
        | E::TargetGainsActorGain
        | E::TargetNoReactions
        | E::Unblockable
        | E::UnlockActingForActorBlood(..)
        | E::UnlockActorIfSuccessful
        | E::UnlockBlocker
        | E::UnlockForBloodIfSuccessful(..)
        | E::UnlockForCorruption(..)
        | E::UnlockPicksIfSuccessful(..)
        | E::VampireBlockToll(..)
        | E::VampiresCannotBlock
        | E::VeilThins(..)
        | E::Wake
        | E::WakeTarget
        | E::YoungerIntercept(..) => {}
        // Combat (the combat underway).
        E::ActorBurnsBloodX
        | E::OpposingNoAdditionalStrikes
        | E::OpposingStrengthMinus(..)
        | E::AdditionalStrikeOnly { .. }
        | E::AdditionalStrikes { .. }
        | E::AdditionalStrikesEachRound(..)
        | E::AdditionalStrikesX { .. }
        | E::AggravatedHands
        | E::AggravatedHandsThisCombat
        | E::AggravatedStrike
        | E::AimExtraDamage(..)
        | E::BurnAfterCombat(..)
        | E::BurnElectronicOnCombatants
        | E::BurnOpponentInsteadOfTorpor
        | E::BurnOpposingAllyOnHit
        | E::CleaveWeapon
        | E::CloseManeuverForBlood(..)
        | E::CloseSelfDamage(..)
        | E::CombatEndsAfterStrike
        | E::CombatManeuver
        | E::ContinueAfterCombat { .. }
        | E::ContinueFreeAfterCombat(..)
        | E::ContinuePressThisRound
        | E::CoordinateAttack { .. }
        | E::DamageAfterCombatIfClose(..)
        | E::DamageOpponentNow(..)
        | E::DiscardChosenX(..)
        | E::DisciplineStrikeDamageThisCombat { .. }
        | E::DrawOutBeast { .. }
        | E::EndPressThisRound
        | E::EnvironmentalDamageThisRoundX
        | E::Expiate
        | E::ExtraDrawPerReplacementThisCombat
        | E::FirstStrike
        | E::FortitudeProofStrike
        | E::FrenzyImmuneThisCombat
        | E::FrenzyShield
        | E::GainBloodLostByOpponent
        | E::HalveOpposingStrikesThisCombat
        | E::HandSizeThisCombat(..)
        | E::HandStrikeDamageThisCombat(..)
        | E::ManeuverForBlood(..)
        | E::MayBurnToPreventInitialStrikes(..)
        | E::NeverAggravatedStrike
        | E::NewRound
        | E::NextRoundClose
        | E::NextRoundManeuver
        | E::NullifyOpposingWeapons
        | E::OnlyHandStrikes
        | E::OpponentOpenHandThisCombat
        | E::OpponentToTorpor
        | E::OpposingBurnsBlood(..)
        | E::OpposingCelerityMayBurnToPrevent(..)
        | E::OpposingCombatCardsCost(..)
        | E::OpposingControllerBurnsPool(..)
        | E::OpposingImprisoned
        | E::OpposingNoCombatEndsThisRound
        | E::OpposingNoDodgeThisRound
        | E::OpposingNoInitialStrikeThisRound
        | E::OpposingNoManeuverThisRound
        | E::OpposingNoStrikeCardsThisCombat
        | E::OpposingRestraint
        | E::OpposingStrikeTollThisCombat(..)
        | E::OpposingTakesMoreThisCombat(..)
        | E::OptionalCloseManeuver
        | E::OrgyOfBlood
        | E::PlayCombatFromAsh
        | E::PreRangeDamage(..)
        | E::PressThisRound
        | E::PreventHalfStrike
        | E::PreventNonAggravatedEachRound(..)
        | E::PutOnActorAfterCombat
        | E::PutOnOpponentOnHit { .. }
        | E::PutOnOpposingKeepControl
        | E::PutOnSelfAsStrikeResolves
        | E::QueueCombatWithOpponent
        | E::ReflectsDamageThisCombat
        | E::ResolutionDamageThisRound(..)
        | E::RoundStrike(..)
        | E::SetStrength(..)
        | E::ShieldFromOpposingStrike(..)
        | E::StrengthThisRoundVotes
        | E::StrikeOnceEachRound(..)
        | E::StunAfterCombatIfClose
        | E::PutOnOpponentAfterCombatIfClose
        | E::UndodgeableStrike
        | E::UnpreventableIfBlackHand
        | E::WeatherDamage { .. } => {}
        // Success (the action's success).
        E::ActingGainsConviction
        | E::ActingVampireTakesDamage(..)
        | E::AllyFromAshLocked(..)
        | E::AshCardToHandNamed
        | E::AshCardToHandOnSuccess(..)
        | E::AshVampireToUncontrolled
        | E::BleedPoolCap(..)
        | E::BloodHuntIfHuntGained
        | E::BurnCorruptionFromTarget
        | E::BurnPoolAtEndIfNotReady(..)
        | E::CoagulateFromPicks { prevent: false }
        | E::CorruptMinionOfTarget
        | E::CorruptTargetMayStealAtCapacity
        | E::EnterCombatWithRandomPreyMonster
        | E::EquipFirstFound
        | E::ExchangeHandForAsh
        | E::FledglingFromNamedMaster
        | E::GainLiaison
        | E::HeldVampireBecomesGargoyle
        | E::HuntAt(..)
        | E::InResultingCombat(..)
        | E::LockHostSkipNextUnlock(..)
        | E::LockMinionOfTarget
        | E::MoveSourceToTarget
        | E::PaidConvictionToTarget
        | E::PayLife(..)
        | E::PreyAshPairHunt(..)
        | E::PreySlotToOwnSlot
        | E::RemoveActorToBurnTarget
        | E::RemoveAllSourceCounters
        | E::RemoveUncontrolledForBlood(..)
        | E::ReorderTop { .. }
        | E::SearchArchetypeOntoActor
        | E::SearchToHand { .. }
        | E::StealBloodUpTo(..)
        | E::StealTargetTitle
        | E::TakeControlOfTargetUntilNextTurn
        | E::TargetActsInCombat
        | E::TargetMethuselahDiscards(..)
        | E::TargetYieldsTitle
        | E::TauntCombat(..)
        | E::UnlockAtEndIfReady => {}
        // Vote (the referendum underway).
        E::Abstain { .. }
        | E::ActingAbstains
        | E::AddVotesAgainst(..)
        | E::AddVotesPerReadySect(..)
        | E::AddVotesXLinear { .. }
        | E::BarNonCamarillaVotes
        | E::BarCorruptedVotes
        | E::CorruptTarget
        | E::BarNonSectVotes(..)
        | E::CancelReferendum { .. }
        | E::FailIfTitleRequired
        | E::GrantVotes(..)
        | E::GrantVotesAll(..)
        | E::NextReferendumPassesIfPasses
        | E::NotPathVotesMinus(..)
        | E::PoisonPill
        | E::PoolForVotesRound { .. }
        | E::TallyPenaltyAgainst(..) => {}
        // Entry (an ally or retainer entering play).
        E::ActsWhenRecruited
        | E::CannotGainLife
        | E::EntryBurnAllyOrRetainer
        | E::EntryRemoveFromAshOrBurn
        | E::MayRepeatThisTurn
        | E::UnlockRecruiterOnFirst
        | E::Version(..) => {}
        // Rider (a rider's hook).
        E::ActingBurnsBloodOrLife(..)
        | E::ActiveBurnsPoolFixedX
        | E::AddCounterForEnteringClan(..)
        | E::AnarchControllerMayBloodToUncontrolled(..)
        | E::AnarchsBloodOrCamarillaAlliesBurn
        | E::BleedOutcomeCounters
        | E::BlockerControllerBurnsPool(..)
        | E::BurnHostCryptTopOut
        | E::BurnOfCrowdedClan
        | E::BurnPoolOrNoTransfers(..)
        | E::BurnPoolPerHuntingGround
        | E::BurnPoolUnlessBurnVampire { .. }
        | E::BurnSelfIfActiveControlsReadyCapacityOrTitled(..)
        | E::BurnSelfUnlessPool(..)
        | E::BurnSelfWithoutStored
        | E::BurnsBloodOrIsBurnedOnEntry(..)
        | E::BuyTitleOnEntry(..)
        | E::ChooseCircleOnEntry
        | E::ClearCounters
        | E::CoinFlipActionFails
        | E::ControllerChoosesMinionDamaged(..)
        | E::ControllerGainsPoolIfPrey(..)
        | E::CountersFromOwnBleed(..)
        | E::CryptSearchOnEntry(..)
        | E::DamageBledAnarchs(..)
        | E::DamageHost(..)
        | E::DamageUnbledThisTurn { .. }
        | E::DistributeToUncontrolled { .. }
        | E::DrawCryptOnEntry
        | E::GoToTorpor
        | E::HandSizeUntilDiscard(..)
        | E::HostBloodToBy(..)
        | E::HostBurnsBloodOrBurnedThenMoves(..)
        | E::HostBurnsBloodOrIsBurned(..)
        | E::HostBurnsBloodOrThisBurns(..)
        | E::HostBurnsConviction(..)
        | E::HostageOnTorpor(..)
        | E::LockOldestOfClan(..)
        | E::LockOrBurnPool(..)
        | E::LockUnlessDiscard
        | E::MayAddCounter
        | E::MayGainPool(..)
        | E::MayHandSizeOncePerTurn(..)
        | E::MayLockForTorture
        | E::MayMakeAnarchForCounter
        | E::MayMoveOffBurnedBearer
        | E::MayUnlockOncePerTurn
        | E::NoEntryConviction
        | E::OtherReadyImbuedDamaged(..)
        | E::PassesToSect(..)
        | E::PoolOrAshCardsPerCounter
        | E::PoolOrBloodPerCounter
        | E::PoolOrRandomCardsPerCounter
        | E::PreyBurnsPoolPerTorpor
        | E::PreyChoosesMinionDamaged(..)
        | E::PreyPaysOrBurnsLocations(..)
        | E::Purge { .. }
        | E::RevealTopCancels { .. }
        | E::SearchPowerOnEntry(..)
        | E::SkipNextUnlock
        | E::SoulGemDraw
        | E::TheirControllerBurnsPool(..)
        | E::TortureBurnPool(..)
        | E::UnlockBurnPoolUnlessReady { .. } => {}
        // Marker (read by name where offered, declared or played).
        E::AdditionalStrikeWithSource
        | E::BleedChangeCardsCost(..)
        | E::AggravatedAgainstClan(..)
        | E::AshCardToLibraryBottom
        | E::BarCanceledThisTurn
        | E::BarCanceledForActorThisTurn
        | E::BearerAimStore
        | E::BearerStore
        | E::BearerDamageOncePerCombat
        | E::BecomesGun
        | E::BecomesOnOpponent
        | E::BecomesWeapon
        | E::BleedIfTargetHasNoReadyUnlocked(..)
        | E::BurnAfterStrike
        | E::BurnAtActionEnd
        | E::BurnAtCombatEnd
        | E::BurnAtInfluencePerPreyReady
        | E::BurnSourceOnDeclare
        | E::BurnSourceOnResolution
        | E::BurnedDamagesBearer(..)
        | E::CancelShield
        | E::CancelableByDiscardingCombat(..)
        | E::CancelableForBlockerBlood(..)
        | E::CancelableForPool(..)
        | E::CancelableForPoolByTargetController(..)
        | E::CanceledGetsMasterAction
        | E::ClanBlocksItsBurn(..)
        | E::ControllerGivesCard
        | E::ConvictionCost(..)
        | E::CountersClearOnNewBearer
        | E::CountersPerDamage
        | E::CryptForBloodIfBleedSuccessful(..)
        | E::DiscardUnlockIfBleedSuccessful { .. }
        | E::EmployRetainer { .. }
        | E::EquipFromAnyAsh
        | E::Frenzy
        | E::ImbuedStrikesPlusX
        | E::ImmuneNonAggravated
        | E::MandatoryAction
        | E::MovesToOldestUntitled { .. }
        | E::MovesToSabbatDiablerist
        | E::NoOtherEquipmentBleed
        | E::NotAgainstFlight
        | E::NotTakenByDiablerie
        | E::OpposingBurnsPoolIfOneReadyAfterCombat(..)
        | E::PinnedWhileBearerReady
        | E::PlayableFromStore(..)
        | E::PoolWhenHostLeaves(..)
        | E::PutOnActorIfOpposingNotReadyAfterCombat
        | E::RechoosesOnControlChange
        | E::RemovedAfterSuccessOrWithoutInfernal
        | E::RemovedOncePlayed
        | E::RepresentsTitle(..)
        | E::RevealTopForCopy { .. }
        | E::SameActionAs(..)
        | E::SearchStore(..)
        | E::SectInterceptThisAction(..)
        | E::ShuffleSourceIntoLibrary
        | E::SparedByCleave
        | E::StealthForClan { .. }
        | E::StealthForTitled(..)
        | E::StealthIfOwnTarget(..)
        | E::StillYours
        | E::StorePickedFromHand
        | E::StoredRemovedWhenBurned
        | E::SwapPickedWithStored
        | E::TakeAnyAction
        | E::TargetLocksReadyUnlocked
        | E::TitledCannotBlock
        | E::UnpreventableStrike => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zvtes::cards::defs::DEFS;
    use zvtes::cards::vampires::VAMPIRES;

    // Every prevention of the definitions is told as one (`Effect::prevents`,
    // the classification the bots and the Fortitude-proof mask read too).
    #[test]
    fn every_prevention_is_told() {
        let modes = DEFS
            .iter()
            .flat_map(|d| d.modes.iter().chain(d.abilities))
            .chain(VAMPIRES.iter().flat_map(|v| v.abilities.iter()));
        let mut seen = 0;
        for m in modes {
            for &e in m.effects.iter().filter(|e| e.prevents()) {
                seen += 1;
                let told = curated(&[e], Whom::default());
                assert!(
                    told.iter().any(|t| t.kind == K::Prevent),
                    "{e:?} not told as a prevention: {told:?}"
                );
            }
        }
        assert!(seen > 20, "{seen}");
    }
}
