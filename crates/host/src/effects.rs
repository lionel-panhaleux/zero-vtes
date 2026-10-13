//! The engine's effects as the contract's curated ones
//! (`zvtes_api::option::Effect`, docs/protocol.md §7): what an ability
//! or a card's mode does, each effect a kind, an amount, whom it applies
//! to. Effects no option told (the engine's bookkeeping) are left out.

use zvtes::cards::Effect as E;
use zvtes::combat::Range as R;
use zvtes::crypt::CRYPT;
use zvtes_api::ids::Range;
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
        E::TargetBurnsBlood(v) => push(on(n(K::Blood, -i(v)), On::Target)),
        E::TargetBloodToPool(v) => push(detail(on(n(K::Blood, -i(v)), On::Target), "to_pool")),
        E::PoolToTargetBlood(v) => push(detail(on(n(K::Blood, i(v)), On::Target), "from_pool")),
        E::HostBloodToTarget(v) => push(detail(on(n(K::Blood, i(v)), On::Target), "from_host")),
        E::TargetBloodToHost(v) => push(detail(on(n(K::Blood, i(v)), On::Host), "from_target")),
        E::StealBloodToHostX => push(detail(x(on(n(K::Blood, 1), On::Host)), "from_target")),
        E::StealBloodFromTarget(v) => push(on(n(K::Steal, i(v)), On::Target)),
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
        E::StrengthThisRound(v) => push(detail(n(K::Strength, i(v)), "this_round")),
        E::StrengthThisCombat(v) => push(n(K::Strength, i(v))),
        E::Prevent(v) => push(n(K::Prevent, i(v))),
        E::PreventGunStrikeAllowance(v) => push(detail(n(K::Prevent, i(v)), "guns")),
        E::PreventNonGun(v) => push(detail(n(K::Prevent, i(v)), "not_guns")),
        E::PreventNonAggravated(v) => push(detail(n(K::Prevent, i(v)), "non_aggravated")),
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
        E::SearchEquip => push(detail(of(K::Search), "equipment")),
        E::DrawThenBottom => push(detail(of(K::Draw), "then_bottom")),
        E::DrawCryptThenRemove => push(detail(of(K::Draw), "crypt_then_remove")),
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
        E::PayActionWithCountersX => push(cost(x(on(n(K::Counters, -1), On::Card)))),
        E::CounterToTarget(_) => push(on(n(K::Counters, 1), On::Target)),
        E::CountersToUncontrolledTarget => push(on(of(K::Counters), On::Uncontrolled)),
        E::ReduceActionBlood(v) => push(detail(n(K::Cost, -i(v)), "blood")),
        E::ReduceActionCost(v) => push(n(K::Cost, -i(v))),
        _ => {}
    }
}
