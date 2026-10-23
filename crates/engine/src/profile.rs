//! Each seat's public play over the whole game, counted (`Engine::profiles`;
//! DESIGN §5 "The seats' context"): the choices `game::public_choice`
//! lets every seat see, by kind, and the outcomes every seat sees (blocks
//! that land, pool bled). The history keeps only the last few choices.

use crate::cards::Effect;
use crate::decision::{ActionKind, Choice};
use crate::game::Engine;
use crate::ids::{MinionId, SeatId};
use crate::library::{CardType, LIBRARY};

/// What a profile counts, in `PROFILE`'s order: the observations read
/// it by position (`zvtes_env::context::names`, appended at its end: a
/// new count after the context's last group goes there, not here).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Count {
    PlaysAction,
    PlaysModifier,
    PlaysReaction,
    PlaysCombat,
    PlaysPolitical,
    PlaysMaster,
    PlaysOther,
    Bleeds,
    Hunts,
    PoliticalActions,
    Rushes,
    Builds,
    OtherActions,
    Blocks,
    Strikes,
    BlocksLanded,
    PoolBled,
    StealthShown,
    InterceptShown,
}

/// `Count`'s names, by position.
pub const PROFILE: [&str; 19] = [
    "plays_action",
    "plays_modifier",
    "plays_reaction",
    "plays_combat",
    "plays_political",
    "plays_master",
    "plays_other",
    "bleeds",
    "hunts",
    "political_actions",
    "rushes",
    "builds",
    "other_actions",
    "blocks",
    "strikes",
    "blocks_landed",
    "pool_bled",
    "stealth_shown",
    "intercept_shown",
];

const _: () = assert!(PROFILE.len() == Count::InterceptShown as usize + 1 && PROFILE.len() <= 32);

/// A seat's counts (`Count`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Profile {
    pub counts: [u16; PROFILE.len()],
}

impl Profile {
    pub fn add(&mut self, k: Count, n: u16) {
        let c = &mut self.counts[k as usize];
        *c = c.saturating_add(n);
    }

    pub fn get(&self, k: Count) -> u16 {
        self.counts[k as usize]
    }

    /// Each of `set` once.
    pub fn add_all(&mut self, set: Counts) {
        for (i, c) in self.counts.iter_mut().enumerate() {
            if set.0 & (1 << i) != 0 {
                *c = c.saturating_add(1);
            }
        }
    }
}

/// A set of `Count`s (a bit each).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts(u32);

impl Counts {
    pub fn push(&mut self, k: Count) {
        self.0 |= 1 << k as usize;
    }
}

/// What `choice`, a public choice `seat` makes on `e`, counts (before it
/// applies: a hand index still names its card), as a set of `Count`s.
pub fn counts(e: &Engine, seat: SeatId, choice: &Choice) -> Counts {
    let mut out = Counts::default();
    if rush(e, seat, choice).is_some() {
        out.push(Count::Rushes);
    }
    match *choice {
        Choice::Play { card, mode, .. } => {
            let id = e.playable_card(seat, card);
            out.push(match LIBRARY[id as usize].types.first() {
                Some(CardType::Action) => Count::PlaysAction,
                Some(CardType::ActionModifier) => Count::PlaysModifier,
                Some(CardType::Reaction) => Count::PlaysReaction,
                Some(CardType::Combat) => Count::PlaysCombat,
                Some(CardType::PoliticalAction) => Count::PlaysPolitical,
                Some(CardType::Master) => Count::PlaysMaster,
                _ => Count::PlaysOther,
            });
            let effects = crate::cards::lookup(id)
                .and_then(|d| d.modes.get(mode as usize))
                .map_or(&[][..], |m| m.effects);
            if effects.iter().any(|f| {
                matches!(
                    f,
                    Effect::AddStealth(_)
                        | Effect::StealthForNextActions
                        | Effect::StealthPerPick
                        | Effect::HuntStealthUntilUnlock(_)
                )
            }) {
                out.push(Count::StealthShown);
            }
            if effects.iter().any(|f| {
                matches!(
                    f,
                    Effect::AddIntercept(_)
                        | Effect::AttemptingIntercept(_)
                        | Effect::AddInterceptX
                        | Effect::CircleIntercept { .. }
                )
            }) {
                out.push(Count::InterceptShown);
            }
        }
        Choice::TakeAction { action, .. } => out.push(match action {
            ActionKind::Bleed => Count::Bleeds,
            ActionKind::Hunt => Count::Hunts,
            ActionKind::Political => Count::PoliticalActions,
            ActionKind::RedListHunt => return out,
            ActionKind::Employ | ActionKind::Recruit | ActionKind::Equip => Count::Builds,
            _ => Count::OtherActions,
        }),
        Choice::DeclareBlock { .. } => out.push(Count::Blocks),
        Choice::HandStrike
        | Choice::RoundStrike { .. }
        | Choice::CommittedStrike
        | Choice::BurnWeaponStrike { .. } => out.push(Count::Strikes),
        _ => {}
    }
    out
}

/// `o` declares a rush: an action entering combat with its target on
/// success (`Effect::EnterCombatWithTarget`, from a card, a card in play's
/// or a vampire's ability; the Red List hunt). The minion it takes on.
/// Actions only: the reactions entering combat (The Guardian, The Status
/// Perfectus, Hidden Lurker's queued combat …) are not rushes, nor Taunt
/// the Caged Beast's or Yawp Court's combats; no precon holds them.
pub fn rush(e: &Engine, seat: SeatId, o: &Choice) -> Option<MinionId> {
    use crate::cards::Target;
    let (effects, target): (&[Effect], Target) = match *o {
        Choice::TakeAction {
            action: ActionKind::RedListHunt,
            target,
            ..
        } => return target,
        Choice::Play {
            card, mode, target, ..
        } => (
            crate::cards::lookup(e.playable_card(seat, card))
                .and_then(|d| d.modes.get(mode as usize))
                .map_or(&[], |m| m.effects),
            target,
        ),
        Choice::UseAbility {
            source,
            ability,
            target,
            ..
        } => (
            source
                .abilities(&e.state)
                .get(ability as usize)
                .map_or(&[], |m| m.effects),
            target,
        ),
        _ => return None,
    };
    match target {
        Target::Minion(m) if effects.contains(&Effect::EnterCombatWithTarget) => Some(m),
        _ => None,
    }
}
