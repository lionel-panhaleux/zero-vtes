//! Punishing habits for an opponent seat in training (owner, 2026-10-04:
//! "opponents that punish", from the owner's games: the network never
//! rescues, diablerizes or burns an opponent's card, and taps out every
//! turn). A frozen network keeps its own play and, by rule, also does what
//! makes those habits costly for the learner: a vampire left in torpor is
//! diablerized, a seat tapped out is bled with no blocker in the way. As
//! strong as the network it rides (DESIGN §11.2: training opponents must be
//! good ones), not a scripted bot. Agent behaviour like `politics`, not an
//! engine mask (rules-qa 17.4 untouched): every option stays legal.
//!
//! By rule, in the minion phase (`choice`), the first that applies:
//! - rescue one's own vampire from torpor, by a minion with at least 2
//!   blood (the rescue's cost, split);
//! - burn another Methuselah's card as the Ⓓ action that card grants
//!   (Creeping Sabotage, The Gate of Acheron, Toreador Grand Ball);
//! - diablerize another Methuselah's vampire in torpor.
//!
//! Kept home (`mask`): once some of a seat's minions have acted this
//! turn, its last ready unlocked minion takes no action but a hunt or a
//! rescue: a blocker for the predator's turn.
//!
//! Each habit can be switched off (`Set`): forced, most of them cost the
//! network strength (DESIGN §11.2, `examples/strength.rs` in
//! `crates/policy`): a forced burn, diablerie or minion kept home, about
//! 0.3 VP a seat; a forced rescue, nothing measurable.
//!
//! Reads only public state.

use crate::cards::{AbilitySource, Effect};
use crate::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use crate::game::Engine;
use crate::ids::{MinionId, SeatId};

/// The minion taking the action an option declares, if it is one.
fn actor(o: &Choice) -> Option<MinionId> {
    match *o {
        Choice::TakeAction { minion, .. } => Some(minion),
        Choice::Play { actor, .. } | Choice::UseAbility { actor, .. } => actor,
        _ => None,
    }
}

/// The habits a seat plays with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Set {
    pub rescue: bool,
    pub burn: bool,
    pub diablerie: bool,
    /// The last ready minion kept home (`mask`).
    pub home: bool,
}

impl Set {
    pub const ALL: Set = Set {
        rescue: true,
        burn: true,
        diablerie: true,
        home: true,
    };

    /// From names, comma-separated: `rescue`, `burn`, `diablerie`, `home`.
    pub fn parse(names: &str) -> Result<Set, String> {
        let mut set = Set {
            rescue: false,
            burn: false,
            diablerie: false,
            home: false,
        };
        for name in names.split(',').map(str::trim).filter(|n| !n.is_empty()) {
            match name {
                "rescue" => set.rescue = true,
                "burn" => set.burn = true,
                "diablerie" => set.diablerie = true,
                "home" => set.home = true,
                _ => return Err(format!("habit {name:?}: rescue, burn, diablerie, home")),
            }
        }
        Ok(set)
    }

    fn has(&self, h: Habit) -> bool {
        match h {
            Habit::Rescue => self.rescue,
            Habit::Burn => self.burn,
            Habit::Diablerie => self.diablerie,
        }
    }
}

/// The plays the habits are about, by any seat's choice (training counts
/// how often the learner makes them when offered).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Habit {
    /// Rescue one's own vampire from torpor.
    Rescue,
    /// Burn another Methuselah's card as the Ⓓ action it grants.
    Burn,
    /// Diablerize another Methuselah's vampire.
    Diablerie,
}

/// The habit an option of `me`'s is, if any.
pub fn habit(e: &Engine, me: SeatId, o: &Choice) -> Option<Habit> {
    let st = &e.state;
    let owner = |m: MinionId| st.minions[m.index()].owner;
    match *o {
        Choice::TakeAction {
            action: ActionKind::Rescue,
            target: Some(t),
            ..
        } if owner(t) == me => Some(Habit::Rescue),
        Choice::TakeAction {
            action: ActionKind::Diablerie,
            target: Some(t),
            ..
        } if owner(t) != me => Some(Habit::Diablerie),
        Choice::UseAbility {
            source: AbilitySource::Card(card),
            ability,
            actor: Some(_),
            ..
        } if st.card_controller(card) != me
            && AbilitySource::Card(card)
                .mode(st, ability)
                .effects
                .contains(&Effect::DeclareAction(ActionKind::BurnCard)) =>
        {
            Some(Habit::Burn)
        }
        _ => None,
    }
}

/// The option the habits take, if one applies (`None`: the network
/// chooses, within `mask`).
pub fn choice(e: &Engine, req: &DecisionRequest, set: Set) -> Option<usize> {
    if req.window != WindowLabel::MinionPhase {
        return None;
    }
    let (st, me) = (&e.state, req.seat);
    let allowed = mask(e, req, set);
    let ok = |i: usize| allowed.as_ref().is_none_or(|m| m[i]);
    let find = |h: Habit, ok: &dyn Fn(usize) -> bool| {
        let on = set.has(h);
        (0..req.options.len()).find(|&i| on && ok(i) && habit(e, me, &req.options[i]) == Some(h))
    };
    // A rescue by a minion that can pay it alone, kept home or not.
    let rescue = find(Habit::Rescue, &|i| {
        matches!(req.options[i], Choice::TakeAction { minion, .. }
            if st.minions[minion.index()].blood >= 2)
    });
    rescue
        .or_else(|| find(Habit::Burn, &ok))
        .or_else(|| find(Habit::Diablerie, &ok))
}

/// The options the habits leave to the network in the minion phase: all
/// but the actions of the last ready unlocked minion once another has
/// acted (`None`: no opinion).
pub fn mask(e: &Engine, req: &DecisionRequest, set: Set) -> Option<Vec<bool>> {
    if !set.home || req.window != WindowLabel::MinionPhase {
        return None;
    }
    let (st, me) = (&e.state, req.seat);
    let ready = st.ready_unlocked(me);
    let acted = st
        .minions_of(me)
        .any(|m| st.minions[m.index()].locked && !st.minions[m.index()].in_torpor);
    let [last] = ready[..] else { return None };
    if !acted {
        return None;
    }
    let mask: Vec<bool> = req
        .options
        .iter()
        .map(|o| {
            actor(o) != Some(last)
                || matches!(
                    o,
                    Choice::TakeAction {
                        action: ActionKind::Hunt | ActionKind::Rescue,
                        ..
                    }
                )
        })
        .collect();
    (mask.iter().any(|&b| b) && mask.iter().any(|&b| !b)).then_some(mask)
}
