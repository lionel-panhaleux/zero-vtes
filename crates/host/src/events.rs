//! What happened, told to one viewer (docs/protocol.md §9): the engine's
//! facts (`narrate.rs`) named by the viewer's copies. Every
//! fact is public; a card it names is shown, numbered as the viewer last
//! saw it (`Numbering::name`).

use crate::narrate::{ActionFact, Fact, Told};
use zvtes::actions::Outcome;
use zvtes::cards::AbilitySource;
use zvtes::combat::{CombatNote, Range as R};
use zvtes::copies::{CopyId, Place};
use zvtes::crypt::CRYPT;
use zvtes::game::Engine;
use zvtes::ids::MinionId;
use zvtes::library::LIBRARY;
use zvtes::state::CardId;
use zvtes_api::event::{ActionOutcome, ActionRef, EventKind, ReferendumOutcome, StruckSide};
use zvtes_api::ids::{MinionKind, Range, Zone};
use zvtes_api::view::Card;

use crate::numbering::Decks;
use crate::view::{action_kind, minion_base, strike, Viewer};

struct Namer<'a> {
    e: &'a Engine,
    decks: &'a Decks,
    viewer: &'a mut Viewer,
    /// The minions as the fact found them (`Told::faces`).
    faces: &'a [(CopyId, u16, bool)],
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
        let (copy, card, crypt) = self.faces[m.index()];
        self.card(copy, card, crypt)
    }

    fn minion_kind(&self, m: MinionId) -> MinionKind {
        crate::view::minion_kind(&self.e.state.minions[m.index()])
    }

    fn minion(&mut self, m: MinionId) -> u32 {
        self.minion_card(m).copy.unwrap_or(u32::MAX)
    }

    fn cause(&mut self, c: zvtes::notes::Cause) -> zvtes_api::event::Cause {
        use zvtes::notes::{Cause as C, Rule, Source};
        let (kind, source) = match c {
            C::Cost(s) => (zvtes_api::event::CauseKind::Cost, s),
            C::Effect(s) => (zvtes_api::event::CauseKind::Effect, s),
        };
        let (card, rule) = match source {
            Source::Played { card, copy } => (Some(self.card(copy, card, false)), None),
            Source::Card(c) => (Some(self.in_play(c)), None),
            Source::Minion(m) => (Some(self.minion_named(m)), None),
            Source::Rule(r) => {
                use zvtes_api::event::CauseRule as R;
                let r = match r {
                    Rule::Strikes => R::Strikes,
                    Rule::Transfer => R::Transfer,
                    Rule::Oust => R::Oust,
                    Rule::Edge => R::Edge,
                    Rule::Contest => R::Contest,
                    Rule::Disease => R::Disease,
                    Rule::Diablerie => R::Diablerie,
                    Rule::Capacity => R::Capacity,
                    Rule::Vote => R::Vote,
                };
                (None, Some(r))
            }
        };
        zvtes_api::event::Cause { kind, card, rule }
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
            Fact::Looked(l) => {
                if self.viewer.seat != Some(l.viewer) {
                    return None;
                }
                EventKind::Looked {
                    seat: l.seat.0,
                    cards: l
                        .cards
                        .iter()
                        .map(|&c| self.card(CopyId::NONE, c, false))
                        .collect(),
                }
            }
            Fact::Played(p) if p.canceled => EventKind::Canceled {
                seat: p.seat.0,
                card: self.card(p.copy, p.card, false),
            },
            Fact::Played(p) => {
                let card = self.card(p.copy, p.card, false);
                // An uncontrolled target is not named: a hidden region, its
                // slot perhaps moved by now.
                let target = match p.target {
                    zvtes::cards::Target::Minion(m) => Some(self.minion(m)),
                    zvtes::cards::Target::Card(c) => self.in_play(c).copy,
                    _ => None,
                };
                let whom = crate::effects::Whom {
                    target,
                    host: None,
                    actor: p.actor.map(|m| self.minion(m)),
                    card: card.copy,
                };
                // Pool and blood (life) it moves at once: told by their
                // events, the card their cause.
                let effects = zvtes::cards::lookup(p.card)
                    .and_then(|d| d.modes.get(p.mode as usize))
                    .filter(|_| !p.no_effect)
                    .map_or(Vec::new(), |m| crate::effects::curated(m.effects, whom))
                    .into_iter()
                    .filter(|e| !(e.when.is_none() && told_by_events(e.kind)))
                    .collect();
                EventKind::Resolved {
                    seat: p.seat.0,
                    card,
                    effects,
                }
            }
            Fact::Pool {
                seat,
                delta,
                pool,
                cause,
            } => EventKind::Pool {
                seat: seat.0,
                delta,
                pool,
                cause: cause.map(|c| self.cause(c)),
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
                cause,
            } => EventKind::Blood {
                minion: self.minion(minion),
                delta,
                blood,
                cause: cause.map(|c| self.cause(c)),
            },
            Fact::CardEnters { card, on } => EventKind::EntersPlay {
                card: self.in_play(card),
                seat: st.cards[card.index()].controller.0,
                on: on.map(|m| self.minion(m)),
                minion: None,
                effect: st.cards[card.index()].effect_only,
            },
            Fact::CardLeaves {
                card,
                to,
                contested,
            } => EventKind::LeavesPlay {
                card: self.in_play(card),
                to: to.and_then(zone),
                minion: None,
                contested,
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
            Fact::ActionStarts(action) => EventKind::ActionStarts {
                seat: action.seat.0,
                acting: self.minion_named(action.acting),
                minion: self.minion_kind(action.acting),
                action: self.action(action),
            },
            Fact::ActingChanges { seat, minion } => EventKind::ActingChanges {
                seat: seat.0,
                acting: self.minion_named(minion),
                minion: self.minion_kind(minion),
            },
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
    pub fn event(&mut self, e: &Engine, decks: &Decks, t: &Told) -> Option<EventKind> {
        Namer {
            e,
            decks,
            viewer: self,
            faces: &t.faces,
        }
        .event(t.fact.clone())
    }
}

/// An effect's kind whose change an event tells (`pool`, `blood`).
fn told_by_events(k: zvtes_api::option::EffectKind) -> bool {
    use zvtes_api::option::EffectKind as K;
    matches!(k, K::Pool | K::Blood | K::Life)
}
