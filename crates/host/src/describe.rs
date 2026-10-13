//! Options as data (docs/protocol.md §7): each engine `Choice` as its
//! `ChoiceKey` — what identifies it among the decision's options, by
//! card ids and copies, never by a position in a hand or a region.
//!
//! One derivation, two namings: a viewer's wire copies (the options a
//! seat is shown) or the engine's own (the record's keys).

use zvtes::cards::{AbilitySource, Target as T};
use zvtes::copies::CopyId;
use zvtes::crypt::CRYPT;
use zvtes::decision::{Choice, DecisionRequest, Zone as Z};
use zvtes::draws::Interrupt;
use zvtes::game::{Contest, Engine};
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::LIBRARY;
use zvtes::state::{CardId, MinionKind, UncontrolledSlot, CREATED};
use zvtes_api::ids::{OptionKind, Zone};
use zvtes_api::option::{CardRef, ChoiceKey, ZoneRef};
use zvtes_api::view::Target;

use crate::view::{action_kind, minion_copy, mode, snake};

/// How copies are named: a viewer's numbers, or the engine's.
pub trait Names {
    fn copy(&self, c: CopyId) -> Option<u32>;
}

/// The engine's own numbers (records).
pub struct EngineNames;

impl Names for EngineNames {
    fn copy(&self, c: CopyId) -> Option<u32> {
        (c != CopyId::NONE).then_some(c.0 as u32)
    }
}

impl Names for crate::numbering::Numbering {
    fn copy(&self, c: CopyId) -> Option<u32> {
        self.get(c).map(|(w, _)| w)
    }
}

/// Copies not named (out of the namer's sight): a sentinel, never a
/// number given.
const UNSEEN: u32 = u32::MAX;

struct Ctx<'a, N: Names> {
    e: &'a Engine,
    seat: SeatId,
    names: &'a N,
}

impl<N: Names> Ctx<'_, N> {
    fn minion(&self, m: MinionId) -> u32 {
        self.names
            .copy(minion_copy(&self.e.state, m.index()))
            .unwrap_or(UNSEEN)
    }

    fn card(&self, c: CardId) -> u32 {
        self.names
            .copy(*self.e.state.cards[c.index()].copy)
            .unwrap_or(UNSEEN)
    }

    /// A card in play: its KRCG id and copy.
    fn in_play(&self, c: CardId) -> CardRef {
        CardRef {
            id: Some(LIBRARY[self.e.state.cards[c.index()].card as usize].id),
            copy: Some(self.card(c)),
            from: None,
        }
    }

    /// The region an uncontrolled-slot index points into: the
    /// interrupt's seat's (a vampire of another Methuselah named), else
    /// the deciding seat's.
    fn region(&self) -> SeatId {
        match self.e.interrupt() {
            Some(
                Interrupt::NameUncontrolled { seat, .. }
                | Interrupt::UncontrolledBlood { seat, .. },
            ) => seat,
            Some(
                Interrupt::PreySlotToPool { of, .. }
                | Interrupt::NameSlotPair { of, from: None, .. },
            ) => of,
            _ => self.seat,
        }
    }

    /// An uncontrolled vampire, by its slot: its card and copy.
    fn slot(&self, i: u8) -> CardRef {
        let region = &self.e.state.seats[self.region().index()].uncontrolled;
        let u: &UncontrolledSlot = &region[i as usize];
        CardRef {
            id: Some(slot_id(self.e, u)),
            copy: Some(self.names.copy(*u.copy).unwrap_or(UNSEEN)),
            from: None,
        }
    }

    fn target(&self, t: T) -> Option<Target> {
        Some(match t {
            T::None => return None,
            T::Seat(s) => Target::Seat { seat: s.0 },
            T::Minion(m) => Target::Card {
                copy: self.minion(m),
            },
            T::Card(c) => Target::Card { copy: self.card(c) },
            T::Uncontrolled(i) => Target::Card {
                copy: self.slot(i).copy.unwrap_or(UNSEEN),
            },
        })
    }

    /// A card in the deciding seat's hand (copies alike are one option):
    /// its KRCG id, and the card it is stored on when played from there.
    fn hand(&self, i: u8) -> CardRef {
        let (c, stored) = self.e.virtual_hand(self.seat)[i as usize];
        CardRef {
            id: Some(LIBRARY[c as usize].id),
            copy: None,
            from: stored.map(|s| self.card(s)),
        }
    }
}

/// A vampire's card in a slot: its crypt card, or a banished created
/// vampire's library card.
fn slot_id(e: &Engine, u: &UncontrolledSlot) -> u32 {
    if u.card != CREATED {
        return CRYPT[u.card as usize].id;
    }
    u.banished
        .as_ref()
        .and_then(|b| match e.state.minions[b.minion.index()].kind {
            MinionKind::Created { card, .. } => Some(LIBRARY[card as usize].id),
            _ => None,
        })
        .unwrap_or(0)
}

fn library(c: u16) -> CardRef {
    CardRef {
        id: Some(LIBRARY[c as usize].id),
        copy: None,
        from: None,
    }
}

/// The key of `choice`, offered to `req.seat`.
pub fn key(e: &Engine, req: &DecisionRequest, choice: &Choice, names: &impl Names) -> ChoiceKey {
    let x = Ctx {
        e,
        seat: req.seat,
        names,
    };
    let mut k = ChoiceKey {
        kind: kind(choice),
        card: None,
        actor: None,
        minion: None,
        target: None,
        action: None,
        mode: None,
        amount: None,
        favor: None,
        blood: None,
        zone: None,
        value: None,
        ability: None,
    };
    match *choice {
        Choice::Pass
        | Choice::BurnEdge
        | Choice::BurnPool
        | Choice::DrawLibrary
        | Choice::DrawCrypt
        | Choice::HandStrike
        | Choice::UseManeuver
        | Choice::UsePress
        | Choice::UsePrevent
        | Choice::PayBlockToll
        | Choice::PayToCancel
        | Choice::StrikeCombatEnds
        | Choice::TakeDamageAfterResolution
        | Choice::CommittedStrike
        | Choice::ContinueAction
        | Choice::GainStrength
        | Choice::BurnBloodToAvoid
        | Choice::Diablerise => {}
        Choice::PayContest(c) | Choice::YieldContest(c) => match c {
            Contest::Card(m) => k.minion = Some(x.minion(m)),
            Contest::Title(m) => {
                k.minion = Some(x.minion(m));
                k.value = Some("title".into());
            }
            Contest::Library(c) => k.card = Some(x.in_play(c)),
        },
        Choice::Play {
            card,
            mode: m,
            actor,
            target,
        } => {
            let r = x.hand(card);
            k.card = Some(r);
            k.mode = Some(mode(e.virtual_hand(req.seat)[card as usize].0, m));
            k.actor = actor.map(|a| x.minion(a));
            k.target = x.target(target);
        }
        Choice::UseAbility {
            source,
            ability,
            actor,
            target,
        } => {
            match source {
                AbilitySource::Card(c) => k.card = Some(x.in_play(c)),
                AbilitySource::Vampire(m) => k.minion = Some(x.minion(m)),
            }
            k.ability = Some(ability);
            k.actor = actor.map(|a| x.minion(a));
            k.target = x.target(target);
        }
        Choice::ResolveRider { card }
        | Choice::BurnInPlay { card }
        | Choice::TakeEquipment { card }
        | Choice::ChooseCard(card)
        | Choice::BurnWeaponStrike { weapon: card } => k.card = Some(x.in_play(CardId(card))),
        Choice::RoundStrike { at } => k.card = at.map(|c| x.in_play(c)),
        Choice::ResolvePrinted { minion }
        | Choice::BurnBloodFrom { minion }
        | Choice::DeclareBlock { minion }
        | Choice::ChooseMinion(minion)
        | Choice::BurnBloodForIntercept { minion }
        | Choice::UnlockForBlood { minion }
        | Choice::UnlockInfernal { minion }
        | Choice::BurnCorruptionToFail { minion }
        | Choice::BurnCorruptionToUnlock { minion } => k.minion = Some(x.minion(minion)),
        Choice::SetX(n) => k.amount = Some(n),
        Choice::Discard(i) | Choice::Bottom(i) => k.card = Some(x.hand(i)),
        Choice::BurnForVote { card, favor } => {
            k.card = Some(x.hand(card));
            k.favor = Some(favor);
        }
        Choice::FindCard(c) => k.card = Some(library(c)),
        Choice::TakeFrom { zone, card } => {
            k.card = Some(library(card));
            k.zone = Some(ZoneRef {
                seat: req.seat.0,
                zone: match zone {
                    Z::Library => Zone::Library,
                    Z::Hand => Zone::Hand,
                    Z::AshHeap => Zone::AshHeap,
                },
            });
        }
        // The Great Beast's Disciplines: a slot of the Discipline list.
        Choice::Slot(i) if matches!(e.interrupt(), Some(Interrupt::BeastDisciplines { .. })) => {
            k.amount = Some(i)
        }
        Choice::Slot(i) => k.card = Some(x.slot(i)),
        Choice::Transfer { slot }
        | Choice::Withdraw { slot }
        | Choice::InfluenceOut { slot }
        | Choice::Merge { slot } => {
            k.target = Some(Target::Card {
                copy: x.slot(slot).copy.unwrap_or(UNSEEN),
            });
        }
        Choice::AshVampire(c) => {
            k.card = Some(CardRef {
                id: Some(CRYPT[c as usize].id),
                copy: None,
                from: None,
            })
        }
        Choice::DrawStored { card, stored } => {
            k.card = Some(CardRef {
                from: Some(x.card(CardId(card))),
                ..library(stored)
            })
        }
        Choice::TakeAction {
            minion,
            action,
            target,
        } => {
            k.actor = Some(x.minion(minion));
            k.action = Some(action_kind(action));
            k.target = target.map(|t| Target::Card { copy: x.minion(t) });
        }
        Choice::PayRescue { from_acting } => k.amount = Some(from_acting),
        Choice::CastVotes { minion, favor } => {
            k.minion = Some(x.minion(minion));
            k.favor = Some(favor);
        }
        Choice::CastEdge { favor } | Choice::CastCardVote { favor } => k.favor = Some(favor),
        Choice::AllocateTo { seat, n } => {
            k.target = Some(Target::Seat { seat: seat.0 });
            k.amount = Some(n);
        }
        Choice::AllocateToCard { card, n } => {
            k.target = Some(Target::Card {
                copy: x.card(CardId(card)),
            });
            k.amount = Some(n);
        }
        Choice::ChooseSeat(s) => k.target = Some(Target::Seat { seat: s.0 }),
        Choice::ChooseClan(v) | Choice::ChooseCircle(v) | Choice::ChooseDiscipline(v) => {
            k.value = Some(snake(v))
        }
        Choice::EmployFromHand { card, mode: m } => {
            k.card = Some(library(card));
            k.mode = Some(mode(card, m));
        }
    }
    k
}

pub fn kind(c: &Choice) -> OptionKind {
    use OptionKind as K;
    match c {
        Choice::Pass => K::Pass,
        Choice::BurnEdge => K::EdgePool,
        Choice::PayContest(_) => K::PayContest,
        Choice::YieldContest(_) => K::YieldContest,
        Choice::Play { .. } => K::Play,
        Choice::UseAbility { .. } => K::UseAbility,
        Choice::ResolveRider { .. } => K::ResolveRider,
        Choice::ResolvePrinted { .. } => K::ResolvePrinted,
        Choice::BurnInPlay { .. } => K::BurnInPlay,
        Choice::BurnPool => K::BurnPool,
        Choice::BurnBloodFrom { .. } => K::BurnBloodFrom,
        Choice::SetX(_) => K::SetX,
        Choice::Discard(_) => K::Discard,
        Choice::FindCard(_) => K::FindCard,
        Choice::TakeFrom { .. } => K::TakeFrom,
        Choice::Slot(_) => K::Slot,
        Choice::AshVampire(_) => K::AshVampire,
        Choice::DrawLibrary => K::DrawLibrary,
        Choice::DrawStored { .. } => K::DrawStored,
        Choice::Bottom(_) => K::Bottom,
        Choice::TakeEquipment { .. } => K::TakeEquipment,
        Choice::TakeAction { .. } => K::TakeAction,
        Choice::DeclareBlock { .. } => K::DeclareBlock,
        Choice::PayRescue { .. } => K::PayRescue,
        Choice::CastVotes { .. } => K::CastVotes,
        Choice::CastEdge { .. } => K::CastEdge,
        Choice::CastCardVote { .. } => K::CastCardVote,
        Choice::BurnForVote { .. } => K::BurnForVote,
        Choice::AllocateTo { .. } => K::AllocateTo,
        Choice::ChooseSeat(_) => K::ChooseSeat,
        Choice::ChooseMinion(_) => K::ChooseMinion,
        Choice::ChooseClan(_) => K::ChooseClan,
        Choice::ChooseCircle(_) => K::ChooseCircle,
        Choice::ChooseDiscipline(_) => K::ChooseDiscipline,
        Choice::ChooseCard(_) => K::ChooseCard,
        Choice::AllocateToCard { .. } => K::AllocateToCard,
        Choice::Transfer { .. } => K::Transfer,
        Choice::Withdraw { .. } => K::Withdraw,
        Choice::DrawCrypt => K::DrawCrypt,
        Choice::InfluenceOut { .. } => K::InfluenceOut,
        Choice::Merge { .. } => K::Merge,
        Choice::HandStrike => K::HandStrike,
        Choice::UseManeuver => K::UseManeuver,
        Choice::UsePress => K::UsePress,
        Choice::UsePrevent => K::UsePrevent,
        Choice::PayBlockToll => K::PayBlockToll,
        Choice::PayToCancel => K::PayToCancel,
        Choice::StrikeCombatEnds => K::StrikeCombatEnds,
        Choice::RoundStrike { .. } => K::RoundStrike,
        Choice::TakeDamageAfterResolution => K::TakeDamageAfterResolution,
        Choice::CommittedStrike => K::CommittedStrike,
        Choice::BurnWeaponStrike { .. } => K::BurnWeaponStrike,
        Choice::EmployFromHand { .. } => K::EmployFromHand,
        Choice::BurnBloodForIntercept { .. } => K::BurnBloodForIntercept,
        Choice::UnlockForBlood { .. } => K::UnlockForBlood,
        Choice::UnlockInfernal { .. } => K::UnlockInfernal,
        Choice::ContinueAction => K::ContinueAction,
        Choice::GainStrength => K::GainStrength,
        Choice::BurnBloodToAvoid => K::BurnBloodToAvoid,
        Choice::BurnCorruptionToFail { .. } => K::BurnCorruptionToFail,
        Choice::BurnCorruptionToUnlock { .. } => K::BurnCorruptionToUnlock,
        Choice::Diablerise => K::Diablerise,
    }
}
