//! A replay's decisions by what they are (owner, 2026-10-04: "so the
//! replays I do are not useless"): an option index stops meaning the same
//! option once the engine offers a different list (copies alike in hand
//! made one option, a new option, a removed one), so each decision is also
//! logged as its choice, with what an engine change could renumber left
//! out: a hand position and a card table index become the card's KRCG
//! id; an uncontrolled-region position, the vampire's KRCG id and which
//! of its copies there it is. A later engine finds the option that is that
//! choice among the ones it offers (`resolve`), whatever its index.
//!
//! Kept as they are: minions, seats and cards in play by their game ids
//! (dealt from the seed and the decklists, not renumbered by a change in
//! what is offered), clans, modes and amounts.

use serde::{Deserialize, Serialize};
use zvtes::cards::Target;
use zvtes::crypt::CRYPT;
use zvtes::decision::{Choice, DecisionRequest};
use zvtes::draws::Interrupt;
use zvtes::game::Engine;
use zvtes::library::LIBRARY;
use zvtes::state::{MinionKind, UncontrolledSlot, CREATED};

/// One decision of a replay, by what was chosen.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Logged {
    pub seat: u8,
    /// The choice (the engine's `Choice`, written as Rust debug output),
    /// its hand positions and card table indices set to 0 (those are
    /// `cards`), an uncontrolled-region position the copy's rank among
    /// that vampire's copies there.
    pub choice: String,
    /// The KRCG ids of the cards at those positions, in the order the
    /// choice names them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cards: Vec<u32>,
    /// A card played from those stored on a card in play (its game id),
    /// not from the hand: the same card, another option.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<u32>,
    /// Every option offered was this choice (copies alike in hand, each
    /// its own option): an engine offering them as one asks nothing.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub forced: bool,
}

impl Logged {
    /// The same choice by the same seat (`forced` aside).
    fn same(&self, o: &Logged) -> bool {
        self.seat == o.seat
            && self.choice == o.choice
            && self.cards == o.cards
            && self.from == o.from
    }
}

/// The option `index` of `req`, by what it is.
pub fn logged(e: &Engine, req: &DecisionRequest, index: usize) -> Logged {
    let mut l = key(e, req, &req.options[index]);
    l.forced = req.options.iter().all(|c| key(e, req, c).same(&l));
    l
}

/// `choice`, offered to `req.seat`, by what it is.
fn key(e: &Engine, req: &DecisionRequest, choice: &Choice) -> Logged {
    let seat = req.seat;
    let mut cards = Vec::new();
    let mut from = None;
    let hand = |i: u8| e.virtual_hand(seat)[i as usize].0;
    let lib = |c: u16| LIBRARY[c as usize].id;
    let crypt = |c: u16| CRYPT[c as usize].id;
    // A vampire in the uncontrolled region: its card, and which of that
    // card's copies there (counters on one, not the other).
    // The region a slot is in: the interrupt's seat's (Malgorzata names a
    // vampire of another Methuselah's), else the deciding seat's.
    let region_seat = match e.interrupt() {
        Some(
            Interrupt::NameUncontrolled { seat, .. } | Interrupt::UncontrolledBlood { seat, .. },
        ) => seat,
        // Solomon Batanea, Lázár Dobrescu (his first pick): the prey's
        // region.
        Some(
            Interrupt::PreySlotToPool { of, .. } | Interrupt::NameSlotPair { of, from: None, .. },
        ) => of,
        _ => seat,
    };
    // A created vampire banished there (Childe of the Revolution): no crypt
    // card (`CREATED`), its library card instead.
    let slot_id = |u: &UncontrolledSlot| {
        if u.card != CREATED {
            return crypt(u.card);
        }
        u.banished
            .as_ref()
            .and_then(|b| match e.state.minions[b.minion.index()].kind {
                MinionKind::Created { card, .. } => Some(lib(card)),
                _ => None,
            })
            .unwrap_or(0)
    };
    let uncontrolled = |i: &mut u8, cards: &mut Vec<u32>| {
        let region = &e.state.seats[region_seat.index()].uncontrolled;
        let id = slot_id(&region[*i as usize]);
        cards.push(id);
        *i = region[..*i as usize]
            .iter()
            .filter(|u| slot_id(u) == id)
            .count() as u8;
    };
    let target = |t: &mut Target, cards: &mut Vec<u32>| {
        if let Target::Uncontrolled(i) = t {
            uncontrolled(i, cards);
        }
    };
    let mut c = *choice;
    match &mut c {
        Choice::Play {
            card, target: t, ..
        } => {
            let (c, stored) = e.virtual_hand(seat)[*card as usize];
            cards.push(lib(c));
            from = stored.map(|s| s.0);
            *card = 0;
            target(t, &mut cards);
        }
        Choice::UseAbility { target: t, .. } => target(t, &mut cards),
        Choice::Discard(i) | Choice::Bottom(i) | Choice::BurnForVote { card: i, .. } => {
            cards.push(lib(hand(*i)));
            *i = 0;
        }
        // The Great Beast's Disciplines: a slot of the Discipline list, no
        // card.
        Choice::Slot(_) if matches!(e.interrupt(), Some(Interrupt::BeastDisciplines { .. })) => {}
        Choice::Slot(i)
        | Choice::Transfer { slot: i }
        | Choice::Withdraw { slot: i }
        | Choice::InfluenceOut { slot: i }
        | Choice::Merge { slot: i } => uncontrolled(i, &mut cards),
        Choice::FindCard(k)
        | Choice::TakeFrom { card: k, .. }
        | Choice::DrawStored { stored: k, .. }
        | Choice::EmployFromHand { card: k, .. } => {
            cards.push(lib(*k));
            *k = 0;
        }
        Choice::AshVampire(k) => {
            cards.push(crypt(*k));
            *k = 0;
        }
        _ => {}
    }
    Logged {
        seat: seat.0,
        choice: format!("{c:?}"),
        cards,
        from,
        forced: false,
    }
}

/// The option `l` logged among those `req` offers: `hint` (the index it
/// was logged at) if that option is the choice, else the first that is
/// (copies alike are the same choice), whatever its index.
pub fn resolve(
    e: &Engine,
    req: &DecisionRequest,
    l: &Logged,
    hint: Option<usize>,
) -> Option<usize> {
    if req.seat.0 != l.seat {
        return None;
    }
    let is = |c: &Choice| key(e, req, c).same(l);
    hint.filter(|&i| req.options.get(i).is_some_and(is))
        .or_else(|| req.options.iter().position(is))
}

/// Every option `req` offers is the same choice: answering it says
/// nothing (an engine offering copies alike apart, where the replay's
/// engine asked nothing).
pub fn moot(e: &Engine, req: &DecisionRequest) -> bool {
    let first = key(e, req, &req.options[0]);
    req.options.iter().all(|c| key(e, req, c).same(&first))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zvtes::crypt::Sect;
    use zvtes::decision::WindowLabel;
    use zvtes::ids::SeatId;
    use zvtes::state::{Banished, CardId, Minion};

    /// A created vampire banished to an uncontrolled region (Childe of the
    /// Revolution) has no crypt card: a transfer to it is logged by its
    /// library card, not by indexing the crypt (a panic).
    #[test]
    fn a_banished_created_vampire_is_logged_by_its_library_card() {
        let mut e = Engine::bare(2);
        let childe = LIBRARY
            .iter()
            .position(|c| c.name == "Childe of the Revolution")
            .unwrap() as u16;
        let m = zvtes::ids::MinionId(e.state.minions.len() as u32);
        let kind = MinionKind::Created {
            card: childe,
            id: CardId(0),
            clan: "Brujah",
            sect: Sect::Anarch,
            born: &zvtes::state::NEWBORN,
        };
        e.state.minions.push(Minion {
            in_play: false,
            ..Minion::new(SeatId(0), kind, 0)
        });
        e.state.seats[0].uncontrolled.push(UncontrolledSlot {
            card: CREATED,
            copy: Default::default(),
            counters: 1,
            banished: Some(Banished {
                minion: m,
                cards: Vec::new(),
            }),
        });
        let choice = Choice::Transfer { slot: 0 };
        let req = DecisionRequest::new(SeatId(0), WindowLabel::InfluenceTransfers, vec![choice]);
        assert_eq!(
            key(&e, &req, &choice).cards,
            vec![LIBRARY[childe as usize].id]
        );
    }
}
