//! What happened, told to one viewer (docs/protocol.md §9): the engine's
//! facts (`narrate.rs`) named by the viewer's copies. Every
//! fact is public; a card it names is shown, numbered as the viewer last
//! saw it (`Numbering::name`).

use crate::narrate::{ActionFact, Fact};
use zvtes::actions::Outcome;
use zvtes::cards::AbilitySource;
use zvtes::combat::{CombatNote, Range as R};
use zvtes::copies::{minion_copy, CopyId, Place};
use zvtes::crypt::CRYPT;
use zvtes::game::Engine;
use zvtes::ids::MinionId;
use zvtes::library::LIBRARY;
use zvtes::state::CardId;
use zvtes_api::event::{ActionOutcome, ActionRef, EventKind, ReferendumOutcome, StruckSide};
use zvtes_api::ids::{MinionKind, Range, Zone};
use zvtes_api::view::Card;

use crate::numbering::Decks;
use crate::view::{action_kind, minion_base, minion_face, strike, Viewer};

struct Namer<'a> {
    e: &'a Engine,
    decks: &'a Decks,
    viewer: &'a mut Viewer,
}

impl Namer<'_> {
    fn card(&mut self, copy: CopyId, card: u16, crypt: bool) -> Card {
        let id = if crypt {
            CRYPT[card as usize].id
        } else {
            LIBRARY[card as usize].id
        };
        let named = self.viewer.numbering.name(self.decks, copy, id, crypt);
        Card {
            copy: named.map(|(c, _)| c),
            id: Some(id),
            crypt,
            nth: named.map(|(_, n)| n),
            numbered: None,
        }
    }

    /// A minion entering or leaving play: its card, numbered by its name.
    fn minion_named(&mut self, m: MinionId) -> Card {
        let mut c = self.minion_card(m);
        let v = &self.e.state.minions[m.index()];
        if let Some(w) = c.copy {
            let token = self.minion_kind(m) == MinionKind::Token;
            c.numbered = self.viewer.names.label(w, &minion_base(v), token);
        }
        c
    }

    fn minion_card(&mut self, m: MinionId) -> Card {
        let st = &self.e.state;
        let (card, crypt) = minion_face(&st.minions[m.index()]);
        self.card(minion_copy(st, m.index()), card, crypt)
    }

    fn minion_kind(&self, m: MinionId) -> MinionKind {
        crate::view::minion_kind(&self.e.state.minions[m.index()])
    }

    fn minion(&mut self, m: MinionId) -> u32 {
        self.minion_card(m).copy.unwrap_or(u32::MAX)
    }

    fn in_play(&mut self, c: CardId) -> Card {
        let card = &self.e.state.cards[c.index()];
        self.card(*card.copy, card.card, false)
    }

    fn action(&mut self, a: ActionFact) -> ActionRef {
        ActionRef {
            kind: action_kind(a.kind),
            card: a.card.map(|(card, copy)| self.card(copy, card, false)),
            source: a.source.map(|s| match s {
                AbilitySource::Card(c) => self.in_play(c),
                AbilitySource::Vampire(m) => self.minion_card(m),
            }),
        }
    }

    fn event(&mut self, fact: Fact) -> Option<EventKind> {
        let st = &self.e.state;
        Some(match fact {
            Fact::Decided(_) => return None,
            Fact::Turn { turn, active } => EventKind::Turn {
                turn,
                active: active.0,
            },
            Fact::Draw { seat, n, crypt } => EventKind::Draw {
                seat: seat.0,
                n,
                crypt,
            },
            Fact::Revealed { seat, card, crypt } => EventKind::Revealed {
                seat: seat.0,
                card: self.card(CopyId::NONE, card, crypt),
            },
            Fact::Pool { seat, delta, pool } => EventKind::Pool {
                seat: seat.0,
                delta,
                pool,
            },
            Fact::Ousted { seat } => EventKind::Ousted { seat: seat.0 },
            Fact::Edge { seat } => EventKind::Edge {
                seat: seat.map(|s| s.0),
            },
            Fact::MinionEnters { minion } => EventKind::EntersPlay {
                card: self.minion_named(minion),
                seat: st.minions[minion.index()].owner.0,
                on: None,
                minion: Some(self.minion_kind(minion)),
                effect: false,
            },
            Fact::MinionLeaves {
                minion,
                contested,
                to,
            } => EventKind::LeavesPlay {
                card: self.minion_named(minion),
                to: to.and_then(zone),
                minion: Some(self.minion_kind(minion)),
                contested,
                effect: false,
            },
            Fact::Torpor { minion, enters } => EventKind::Torpor {
                minion: self.minion(minion),
                enters,
            },
            Fact::Blood {
                minion,
                delta,
                blood,
            } => EventKind::Blood {
                minion: self.minion(minion),
                delta,
                blood,
            },
            Fact::CardEnters { card, on } => EventKind::EntersPlay {
                card: self.in_play(card),
                seat: st.cards[card.index()].controller.0,
                on: on.map(|m| self.minion(m)),
                minion: None,
                effect: st.cards[card.index()].effect_only,
            },
            Fact::CardLeaves { card, to } => EventKind::LeavesPlay {
                card: self.in_play(card),
                to: to.and_then(zone),
                minion: None,
                contested: false,
                effect: st.cards[card.index()].effect_only,
            },
            Fact::CardMoves { card, on } => EventKind::Moves {
                card: self.in_play(card),
                on: on.map(|m| self.minion(m)),
                effect: st.cards[card.index()].effect_only,
            },
            Fact::BlockFails { minion } => EventKind::BlockFails {
                minion: self.minion(minion),
            },
            Fact::Strike {
                minion,
                strike: kind,
                source,
                told,
            } => {
                let card = source.map(|c| LIBRARY[st.cards[c.index()].card as usize].id);
                EventKind::StrikeDeclared {
                    minion: self.minion(minion),
                    strike: kind.map(|k| crate::view::told(strike(k, card), told)),
                }
            }
            Fact::ActionEnds {
                action,
                outcome,
                ousted,
                blocker,
            } => EventKind::ActionEnds {
                seat: action.seat.0,
                acting: self.minion(action.acting),
                action: self.action(action),
                outcome: match (ousted, outcome) {
                    (true, _) => ActionOutcome::Ousted,
                    (_, Some(Outcome::Successful)) => ActionOutcome::Succeeds,
                    (_, Some(Outcome::Blocked)) => ActionOutcome::Blocked,
                    (_, Some(Outcome::Failed)) => ActionOutcome::Fails,
                    (_, Some(Outcome::Fizzled)) => ActionOutcome::Fizzles,
                    (_, None) => ActionOutcome::Ends,
                },
                blocker: blocker.map(|m| self.minion(m)),
            },
            Fact::ReferendumEnds {
                passed,
                canceled,
                votes: (votes_for, votes_against),
                action,
                hunted,
            } => EventKind::ReferendumEnds {
                outcome: if canceled {
                    ReferendumOutcome::Canceled
                } else if passed {
                    ReferendumOutcome::Passes
                } else {
                    ReferendumOutcome::Fails
                },
                votes_for,
                votes_against,
                action: action.map(|a| self.action(a)),
                hunted: hunted.map(|m| self.minion(m)),
            },
            Fact::Combat(CombatNote::Begins { minions: [a, b] }) => EventKind::CombatBegins {
                minions: [self.minion(a), self.minion(b)],
            },
            Fact::Combat(CombatNote::Strikes {
                minions,
                round,
                pair,
                range,
                strikes,
                told,
                damage,
                strike_damage,
            }) => EventKind::Strikes {
                round,
                pair,
                range: match range {
                    R::Close => Range::Close,
                    R::Long => Range::Long,
                },
                sides: (0..2)
                    .map(|i| StruckSide {
                        minion: self.minion(minions[i]),
                        strike: strikes[i].map(|k| crate::view::told(strike(k, None), told[i])),
                        damage: damage[i],
                        strike_damage: strike_damage[i],
                    })
                    .collect(),
            },
            Fact::Combat(CombatNote::Ends { .. }) => EventKind::CombatEnds,
        })
    }
}

/// The zone a card went to, when a seat's.
fn zone(p: Place) -> Option<Zone> {
    Some(match p {
        Place::Zone(_, "crypt") => Zone::Crypt,
        Place::Zone(_, "library") => Zone::Library,
        Place::Zone(_, "hand") => Zone::Hand,
        Place::Zone(_, "ash" | "ash_crypt") => Zone::AshHeap,
        Place::Zone(_, "removed" | "removed_crypt") => Zone::Removed,
        Place::Uncontrolled(_) => Zone::Uncontrolled,
        _ => return None,
    })
}

impl Viewer {
    /// A fact as this viewer is told it (`None`: a decision taken, told
    /// from its answer), named after the view that took in its change.
    pub fn event(&mut self, e: &Engine, decks: &Decks, fact: Fact) -> Option<EventKind> {
        Namer {
            e,
            decks,
            viewer: self,
        }
        .event(fact)
    }
}
