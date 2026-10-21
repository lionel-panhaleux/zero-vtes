//! What happened, told to one viewer (docs/protocol.md §9): the engine's
//! facts (`narrate.rs`) named by the viewer's copies. Every
//! fact is public; a card it names is shown, numbered as the viewer last
//! saw it (`Numbering::name`).

use crate::narrate::{ActionFact, Fact, Held, Spot, Told};
use zvtes::actions::Outcome;
use zvtes::cards::AbilitySource;
use zvtes::combat::{CombatNote, Range as R};
use zvtes::copies::{CopyId, Place};
use zvtes::crypt::CRYPT;
use zvtes::game::Engine;
use zvtes::ids::MinionId;
use zvtes::library::LIBRARY;
use zvtes::state::CardId;
use zvtes_api::event::{
    ActionOutcome, ActionRef, EventKind, ReferendumOutcome, StruckSide, ZoneRef,
};
use zvtes_api::ids::{MinionKind, Range, Zone};
use zvtes_api::option::SlotRef;
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

    fn sees(&self, at: Spot) -> bool {
        at.seen_by(self.viewer.seat)
    }

    /// A place as a seat's zone, held as the fact found it (`Spot`): a
    /// stored card's, the card it is on; a card on a minion's, the
    /// minion; a banished vampire's cards', it.
    fn zone_ref(&mut self, p: Place, at: Spot) -> Option<ZoneRef> {
        let st = &self.e.state;
        let minion = if at.torpor { Zone::Torpor } else { Zone::Ready };
        let (zone, on) = match p {
            Place::Zone(..) | Place::Uncontrolled(_) => (zone(p)?, None),
            Place::InFlight(_) => (Zone::Limbo, None),
            Place::Training(_) => (Zone::Training, None),
            // With the banished vampire (a created one's slot unnumbered).
            Place::Banished { slot, .. } => {
                let on = self.viewer.numbering.get(slot).map(|(w, _)| w);
                (Zone::Uncontrolled, on)
            }
            Place::Card { id, stored: true } => {
                (Zone::Stored, Some(self.in_play(CardId(id)).copy?))
            }
            // An ally: a minion.
            Place::Card { id, .. } if st.cards[id as usize].body.is_some() => (minion, None),
            Place::Card { .. } => (Zone::InPlay, at.on.map(|m| self.minion(m))),
            // A merged vampire's base card: on the minion it is now.
            Place::Minion { id, merged: true } => (minion, Some(self.minion(MinionId(id)))),
            Place::Minion { .. } => (minion, None),
        };
        Some(ZoneRef {
            seat: at.seat.0,
            zone,
            on,
        })
    }

    /// A minion's or a card in play's copy.
    fn copy(&mut self, h: Held) -> Option<u32> {
        match h {
            Held::Minion(m) => Some(self.minion(m)),
            Held::Card(c) => self.in_play(c).copy,
            Held::Slot(_) => unreachable!("a lock or control is a minion's or a card's"),
        }
    }

    /// What counters are on: its copy, or an uncontrolled
    /// vampire's place (none once it left the region: its copy then, a
    /// minion).
    fn held(&mut self, h: Held) -> Option<(Option<u32>, Option<SlotRef>)> {
        let st = &self.e.state;
        Some(match h {
            Held::Minion(_) => unreachable!("counters are a card's or a slot's"),
            Held::Card(c) => (Some(self.in_play(c).copy?), None),
            Held::Slot(copy) => {
                let slot = st.seats.iter().enumerate().find_map(|(s, seat)| {
                    let i = seat.uncontrolled.iter().position(|u| *u.copy == copy)?;
                    Some(SlotRef {
                        seat: s as u8,
                        slot: i as u8,
                    })
                });
                match slot {
                    Some(slot) => (None, Some(slot)),
                    None => {
                        let i = self.faces.iter().position(|&(c, _, _)| c == copy)?;
                        (Some(self.minion(MinionId(i as u32))), None)
                    }
                }
            }
        })
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
            Fact::Phase(p) => EventKind::Phase {
                phase: crate::view::phase(p),
            },
            Fact::Transfers(left) => EventKind::Transfers { left },
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
            Fact::Locked { what, locked } => EventKind::Locked {
                copy: self.copy(what)?,
                locked,
            },
            Fact::Counters {
                what,
                delta,
                counters,
                cause,
            } => {
                let (copy, slot) = self.held(what)?;
                EventKind::Counters {
                    copy,
                    slot,
                    delta,
                    counters,
                    cause: cause.map(|c| self.cause(c)),
                }
            }
            Fact::Controller { what, seat } => EventKind::Controller {
                copy: self.copy(what)?,
                seat: seat.0,
            },
            Fact::Capacity { minion, capacity } => EventKind::Capacity {
                minion: self.minion(minion),
                capacity,
            },
            Fact::Title { minion, title } => EventKind::Title {
                minion: self.minion(minion),
                title: crate::view::title(title),
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
            Fact::Zoned {
                copy,
                card,
                crypt,
                from,
                to,
                at: [was, now],
            } => {
                let card = if self.sees(was) || self.sees(now) {
                    self.card(copy, card, crypt)
                } else {
                    Card {
                        copy: None,
                        id: None,
                        crypt,
                        nth: None,
                        numbered: None,
                    }
                };
                EventKind::CardMoves {
                    card,
                    from: self.zone_ref(from, was)?,
                    to: self.zone_ref(to, now)?,
                }
            }
            Fact::Moves { card, on } => EventKind::Moves {
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
    /// The table now and what happened since, named for this viewer as
    /// the view numbers it.
    pub fn tell(
        &mut self,
        e: &Engine,
        decks: &Decks,
        names: &[String],
        told: Vec<Told>,
    ) -> (zvtes_api::view::TableView, Vec<(Told, Option<EventKind>)>) {
        let v = self.view(e, decks, names);
        let named = told
            .into_iter()
            .map(|t| {
                let kind = self.event(e, decks, &t);
                (t, kind)
            })
            .collect();
        (v, named)
    }

    /// A fact as this viewer is told it (`None`: a decision taken, told
    /// from its answer), named after the view that took in its change.
    pub(crate) fn event(&mut self, e: &Engine, decks: &Decks, t: &Told) -> Option<EventKind> {
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
