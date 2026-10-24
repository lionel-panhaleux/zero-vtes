//! The table as a seat sees it, from the engine (docs/protocol.md §6):
//! the engine's legal view (hidden cards blanked, their copies too),
//! copies renamed per viewer (`numbering`), every field data. Built in two
//! passes: the first collects the copies in sight, the numbering updates,
//! the second builds with wire numbers.

use std::cell::RefCell;

use zvtes::actions::ActionState;
use zvtes::cards::{AbilitySource, StrikeKind as Sk, Target as T};
use zvtes::combat::Range as R;
pub use zvtes::copies::minion_copy;
use zvtes::copies::CopyId;
use zvtes::crypt::{Title as Ti, CRYPT};
use zvtes::decision::ActionKind as Ak;
use zvtes::game::{Engine, Phase as P};
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::LIBRARY;
use zvtes::referendum::{RefStep, Source};
use zvtes::state::{CardId, GameState, MinionKind as Mk, HIDDEN};
use zvtes_api::ids::{
    ActionKind, ActionStep, MarkerKind, MinionKind, Phase, Range, ReferendumStep, StrikeKind,
    Trait, VoteSource,
};
use zvtes_api::option::{DisciplineLevel, Mode};
use zvtes_api::view::{
    ActionView, Allocated, Card, CombatSide, CombatView, InPlayView, Marker, MinionView,
    ReferendumView, SeatView, SlotView, Strike, TableView, Target, Terms, Title, Vote,
};

use crate::numbering::{Decks, MinionNames, Numbering, Seen};

/// One viewer's view of the game: its numbering kept across views.
#[derive(Clone, Debug, Default)]
pub struct Viewer {
    pub seat: Option<SeatId>,
    pub numbering: Numbering,
    pub names: MinionNames,
}

impl Viewer {
    pub fn new(seat: Option<SeatId>) -> Viewer {
        Viewer {
            seat,
            numbering: Numbering::default(),
            names: MinionNames::default(),
        }
    }

    /// The table now, the numbering brought up to date.
    pub fn view(&mut self, e: &Engine, decks: &Decks, names: &[String]) -> TableView {
        let st = match self.seat {
            Some(v) => e.state.legal_view(v),
            None => e.state.clone(),
        };
        let probe = Build {
            e,
            st: &st,
            names,
            numbering: None,
            seen: RefCell::new(Vec::new()),
        };
        probe.table();
        let seen = probe.seen.into_inner();
        self.numbering.update(decks, &seen);
        let minion_names = RefCell::new(std::mem::take(&mut self.names));
        let v = Build {
            e,
            st: &st,
            names,
            numbering: Some((&self.numbering, &minion_names)),
            seen: RefCell::new(Vec::new()),
        }
        .table();
        self.names = minion_names.into_inner();
        v
    }

    /// A minion's or a card in play's wire copy (options and events name
    /// them so), as of the last view.
    pub fn minion(&self, e: &Engine, m: MinionId) -> Option<u32> {
        self.numbering
            .get(minion_copy(&e.state, m.index()))
            .map(|(c, _)| c)
    }

    pub fn card(&self, e: &Engine, c: CardId) -> Option<u32> {
        self.numbering
            .get(*e.state.cards[c.index()].copy)
            .map(|(c, _)| c)
    }
}

struct Build<'a> {
    e: &'a Engine,
    st: &'a GameState,
    names: &'a [String],
    /// `None`: the first pass, collecting; then the numbering, and the
    /// minions' names numbered in table order.
    numbering: Option<(&'a Numbering, &'a RefCell<MinionNames>)>,
    seen: RefCell<Vec<Seen>>,
}

impl Build<'_> {
    /// An uncontrolled vampire's card: its crypt card, or a banished
    /// created vampire's library card.
    fn slot_card(&self, u: &zvtes::state::UncontrolledSlot) -> Card {
        let (card, crypt) = crate::describe::slot_face(self.e, u);
        self.card(*u.copy, card, crypt)
    }

    /// A card: its face if the viewer sees it, its copy if numbered.
    fn card(&self, copy: CopyId, card: u16, crypt: bool) -> Card {
        let id = (card != HIDDEN).then(|| {
            if crypt {
                CRYPT[card as usize].id
            } else {
                LIBRARY[card as usize].id
            }
        });
        let (copy, nth) = match (id, copy == CopyId::NONE) {
            (Some(id), false) => match self.numbering.map(|(n, _)| n) {
                None => {
                    self.seen.borrow_mut().push((copy, id, crypt));
                    (None, None)
                }
                Some(n) => match n.get(copy) {
                    Some((w, nth)) => (Some(w), Some(nth)),
                    None => (None, None),
                },
            },
            _ => (None, None),
        };
        Card {
            copy,
            id,
            crypt,
            nth,
            numbered: None,
        }
    }

    fn pile(&self, p: &zvtes::copies::Pile, crypt: bool) -> Vec<Card> {
        p.pairs()
            .map(|(c, copy)| self.card(copy, c, crypt))
            .collect()
    }

    /// A pile the viewer never looks through (libraries, crypts): backs.
    fn backs(&self, n: usize, crypt: bool) -> Vec<Card> {
        (0..n)
            .map(|_| Card {
                copy: None,
                id: None,
                crypt,
                nth: None,
                numbered: None,
            })
            .collect()
    }

    fn minion_ref(&self, m: MinionId) -> u32 {
        self.wire(minion_copy(self.st, m.index()))
    }

    fn card_ref(&self, c: CardId) -> u32 {
        self.wire(*self.st.cards[c.index()].copy)
    }

    fn wire(&self, c: CopyId) -> u32 {
        self.numbering
            .and_then(|(n, _)| n.get(c))
            .map_or(u32::MAX, |(w, _)| w)
    }

    fn table(&self) -> TableView {
        let e = self.e;
        let st = self.st;
        TableView {
            viewer: None,
            decision: e.log.len() as u64,
            turn: st.turn,
            active: st.active.0,
            phase: phase(e.phase()),
            seats: (0..st.seats.len()).map(|s| self.seat(s)).collect(),
            action: e.action().map(|a| {
                let blocker = match a.state {
                    ActionState::Attempt(m) | ActionState::Blocked(m) => Some(m),
                    _ => None,
                };
                ActionView {
                    seat: a.seat.0,
                    acting: self.minion_ref(a.acting),
                    kind: action_kind(a.kind),
                    step: action_step(a.state),
                    card: a.card.as_ref().map(|p| self.card(*p.copy, p.card, false)),
                    mode: a.card.as_ref().map(|p| mode(p.card, p.mode)),
                    x: a.card.as_ref().map(|p| p.x).filter(|&x| x > 0),
                    source: match a.source {
                        Some(AbilitySource::Card(c)) => Some(self.card_ref(c)),
                        _ => None,
                    },
                    target: self
                        .target(a.target)
                        .or_else(|| a.card.as_ref().and_then(|p| self.target(p.target))),
                    bleed_target: a.bleed_target.map(|s| s.0),
                    bleed: (a.kind == Ak::Bleed).then(|| a.bleed_amount(&e.state)),
                    stealth: a.stealth(&e.state),
                    blocker: blocker.map(|m| self.minion_ref(m)),
                    intercept: blocker.map(|m| a.intercept_of(&e.state, m)),
                    declined: a.declined.iter().map(|s| s.0).collect(),
                    declining: a.declining(&e.state),
                }
            }),
            combat: e.combat().map(|c| CombatView {
                minions: [
                    self.minion_ref(c.sides[0].minion),
                    self.minion_ref(c.sides[1].minion),
                ],
                round: c.round,
                range: Some(match c.range {
                    R::Close => Range::Close,
                    R::Long => Range::Long,
                }),
                sides: c
                    .sides
                    .iter()
                    .enumerate()
                    .map(|(i, s)| CombatSide {
                        minion: self.minion_ref(s.minion),
                        strike: s.chosen.then_some(s.strike).flatten().map(|k| {
                            let card = s
                                .strike_source
                                .map(|c| LIBRARY[e.state.cards[c.index()].card as usize].id);
                            let hit = s
                                .strike_at
                                .is_none()
                                .then(|| c.strike_told(&e.state, e.action(), i, k));
                            told(strike(k, card), hit.flatten())
                        }),
                        maneuvers: s.maneuvers,
                        presses: s.presses,
                    })
                    .collect(),
            }),
            transfers: e.transfers_left(),
            referendum: e.referendum().map(|r| {
                let tally = |favor: bool| {
                    r.records
                        .iter()
                        .filter(|v| v.favor == favor && !v.canceled)
                        .map(|v| v.n as u32)
                        .sum()
                };
                let mut seats: Vec<u8> = r.terms.chosen.iter().map(|s| s.0).collect();
                for s in &r.terms.seats {
                    if !seats.contains(&s.0) {
                        seats.push(s.0);
                    }
                }
                let mut allocated: Vec<Allocated> = r
                    .terms
                    .alloc
                    .iter()
                    .map(|&(s, points)| Allocated {
                        to: Target::Seat { seat: s.0 },
                        points,
                    })
                    .collect();
                allocated.extend(r.terms.card_alloc.iter().map(|&(c, points)| Allocated {
                    to: Target::Card {
                        copy: self.card_ref(c),
                    },
                    points,
                }));
                ReferendumView {
                    caller: r.caller.0,
                    step: match r.step {
                        RefStep::Terms => ReferendumStep::Terms,
                        RefStep::Polling => ReferendumStep::Polling,
                        RefStep::Shortfall { .. } => ReferendumStep::Shortfall,
                        RefStep::Choose => ReferendumStep::Choose,
                    },
                    calling: r.calling.map(|m| self.minion_ref(m)),
                    card: e
                        .action()
                        .and_then(|a| a.card.as_ref())
                        .map(|p| LIBRARY[p.card as usize].id),
                    votes_for: tally(true),
                    votes_against: tally(false),
                    terms: Terms {
                        seats,
                        cards: r
                            .terms
                            .minions
                            .iter()
                            .map(|&m| self.minion_ref(m))
                            .chain(r.terms.cards.iter().map(|&c| self.card_ref(c)))
                            .collect(),
                        clan: r.terms.clan.map(|c| snake(&String::from(c))),
                        allocated,
                        pending: r.terms.pending.iter().map(|s| s.0).collect(),
                    },
                    votes: r
                        .records
                        .iter()
                        .map(|v| {
                            let (source, copy) = match v.source {
                                Source::Vampire(m) => {
                                    (VoteSource::Vampire, Some(self.minion_ref(m)))
                                }
                                Source::Printed(m) => {
                                    (VoteSource::Printed, Some(self.minion_ref(m)))
                                }
                                Source::Edge(_) => (VoteSource::Edge, None),
                                Source::Card(_) => (VoteSource::Card, None),
                                Source::InPlay(c) => (VoteSource::InPlay, Some(self.card_ref(c))),
                                Source::Pool(_) => (VoteSource::Pool, None),
                            };
                            Vote {
                                seat: v.seat.0,
                                source,
                                copy,
                                favor: v.favor,
                                n: v.n,
                                canceled: v.canceled,
                            }
                        })
                        .collect(),
                }
            }),
        }
    }

    fn target(&self, t: T) -> Option<Target> {
        Some(match t {
            T::None => return None,
            T::Minion(m) => Target::Card {
                copy: self.minion_ref(m),
            },
            T::Seat(s) => Target::Seat { seat: s.0 },
            // An uncontrolled vampire, by its position in its owner's
            // region: named by its copy when the viewer sees it.
            T::Uncontrolled(_) => return None,
            T::Card(c) => Target::Card {
                copy: self.card_ref(c),
            },
        })
    }

    fn seat(&self, s: usize) -> SeatView {
        let st = self.st;
        let seat = &st.seats[s];
        let sid = SeatId(s as u8);
        let mut ash = self.pile(&seat.ash, false);
        ash.extend(self.pile(&seat.ash_crypt, true));
        let mut removed = self.pile(&seat.removed, false);
        removed.extend(self.pile(&seat.removed_crypt, true));
        let mine: Vec<usize> = (0..st.minions.len())
            .filter(|&i| st.minions[i].in_play && st.minions[i].owner == sid)
            .collect();
        let minions = |region: Region| {
            mine.iter()
                .filter(|&&i| Region::of(&st.minions[i]) == region)
                .map(|&i| self.minion(i))
                .collect()
        };
        let mut v = SeatView {
            seat: s as u8,
            name: self.names.get(s).cloned().unwrap_or_default(),
            pool: seat.pool,
            vp_halves: seat.half_vps,
            ousted: seat.ousted,
            withdrawing: seat.withdrawal == zvtes::state::Withdrawal::Announced,
            withdrew: seat.withdrew,
            edge: seat.has_edge,
            prey: st.prey(sid).0,
            predator: st.predator(sid).0,
            crypt: self.backs(seat.crypt.len(), true),
            library: self.backs(seat.library.len(), false),
            hand: self.pile(&seat.hand, false),
            ash_heap: ash,
            removed,
            uncontrolled: seat
                .uncontrolled
                .iter()
                .map(|u| SlotView {
                    card: self.slot_card(u),
                    counters: u.counters,
                    banished: u.banished.is_some(),
                    cards: u.banished.as_ref().map_or_else(Vec::new, |b| {
                        b.cards
                            .iter()
                            .flat_map(|&c| {
                                let c = &st.cards[c.index()];
                                std::iter::once(self.card(*c.copy, c.card, false))
                                    .chain(self.pile(&c.stored, false))
                            })
                            .collect()
                    }),
                })
                .collect(),
            training: (0..st.cards.len())
                .filter(|&c| {
                    let card = &st.cards[c];
                    card.training && !card.in_play && card.controller == sid
                })
                .map(|c| self.in_play(c))
                .collect(),
            ready: minions(Region::Ready),
            torpor: minions(Region::Torpor),
            incapacitated: minions(Region::Incapacitated),
            in_play: (0..st.cards.len())
                .filter(|&c| {
                    shown_card(st, c)
                        && st.cards[c].host.is_none()
                        && st.card_controller(CardId(c as u32)) == sid
                })
                .map(|c| self.in_play(c))
                .collect(),
            limbo: self.limbo(s),
        };
        // An ousted Methuselah's cards are out of the game, in no place
        // (the engine keeps its crypt cards in its ash heap and removed).
        if seat.ousted {
            v = SeatView {
                crypt: vec![],
                library: vec![],
                hand: vec![],
                ash_heap: vec![],
                removed: vec![],
                uncontrolled: vec![],
                training: vec![],
                ready: vec![],
                torpor: vec![],
                incapacitated: vec![],
                in_play: vec![],
                limbo: vec![],
                ..v
            };
        }
        v
    }

    /// The cards the seat is playing, then the action's card while it is
    /// held (until it enters play or returns to hand).
    fn limbo(&self, s: usize) -> Vec<Card> {
        let e = self.e;
        let flight = e.in_flight();
        let mut out: Vec<Card> = flight
            .iter()
            .filter(|p| p.seat.index() == s)
            .map(|p| self.card(*p.copy, p.card, false))
            .collect();
        let held = e
            .action()
            .filter(|a| !a.card_entered_play && !a.card_returned)
            .and_then(|a| a.card.as_ref())
            .filter(|p| p.seat.index() == s);
        if let Some(p) = held {
            if !flight
                .iter()
                .any(|q| q.seat == p.seat && q.card == p.card && q.actor == p.actor)
            {
                out.push(self.card(*p.copy, p.card, false));
            }
        }
        out
    }

    fn in_play(&self, i: usize) -> InPlayView {
        let st = self.st;
        let c = &st.cards[i];
        InPlayView {
            card: self.card(*c.copy, c.card, false),
            controller: st.card_controller(CardId(i as u32)).0,
            locked: c.locked,
            counters: c.counters,
            contested: c.contested,
            stored: self.pile(&c.stored, false),
        }
    }

    /// A minion's card, numbered by its name in the second pass.
    fn minion_card(
        &self,
        m: &zvtes::state::Minion,
        copy: CopyId,
        card: u16,
        crypt: bool,
        kind: MinionKind,
    ) -> Card {
        let mut c = self.card(copy, card, crypt);
        if let (Some((_, names)), Some(w)) = (self.numbering, c.copy) {
            c.numbered = names
                .borrow_mut()
                .label(w, &minion_base(m), kind == MinionKind::Token);
        }
        c
    }

    fn minion(&self, i: usize) -> MinionView {
        let st = self.st;
        let m = &st.minions[i];
        let id = MinionId(i as u32);
        let (card, crypt) = minion_face(m);
        let kind = minion_kind(m);
        let marker = |kind, n: u8| Marker {
            kind,
            seat: None,
            n: Some(n),
            card: None,
            text: None,
        };
        let mut markers: Vec<Marker> = m
            .corruption
            .iter()
            .map(|&(s, n)| Marker {
                seat: Some(s.0),
                ..marker(MarkerKind::Corruption, n)
            })
            .collect();
        for (kind, n) in [
            (MarkerKind::Hostage, m.hostage),
            (MarkerKind::Nightmare, m.nightmare),
            (MarkerKind::Torture, m.torture),
            (MarkerKind::Pact, m.pact),
            (MarkerKind::Taint, m.taint),
        ] {
            if n > 0 {
                markers.push(marker(kind, n));
            }
        }
        for &(s, _) in st.red_list_marks.iter().filter(|&&(_, t)| t == id) {
            markers.push(Marker {
                seat: Some(s.0),
                n: None,
                ..marker(MarkerKind::RedListMark, 0)
            });
        }
        for (kind, on) in [
            (MarkerKind::Disease, m.disease),
            (MarkerKind::TitleContested, m.title_contested),
            (MarkerKind::Expelled, m.expelled),
            (MarkerKind::OutOfPlay, m.out_of_play),
            (MarkerKind::Contested, m.contested_in_play),
        ] {
            if on {
                markers.push(Marker {
                    n: None,
                    ..marker(kind, 0)
                });
            }
        }
        // Lasting effects whose card is in the ash heap (rules-qa 21.1).
        for c in st
            .cards
            .iter()
            .filter(|c| c.in_play && c.effect_only && c.host == Some(id))
        {
            markers.push(Marker {
                card: Some(LIBRARY[c.card as usize].id),
                n: (c.counters > 0).then_some(c.counters),
                ..marker(MarkerKind::Effect, 0)
            });
        }
        MinionView {
            card: self.minion_card(m, minion_copy(st, i), card, crypt, kind),
            kind,
            name: crate::cards::printed(m.name()),
            blood: m.blood,
            capacity: st.capacity_of(id),
            locked: m.locked,
            contested: m.contested,
            title: if m.is_vampire() {
                title(st.title_of(id))
            } else {
                None
            },
            owner: m.origin.filter(|&o| o != m.owner).map(|o| o.0),
            cards: (0..st.cards.len())
                .filter(|&c| shown_card(st, c) && st.cards[c].host == Some(id))
                .map(|c| self.in_play(c))
                .collect(),
            markers,
            traits: traits(st, id),
        }
    }
}

/// A minion's traits, read live.
fn traits(st: &zvtes::state::GameState, m: MinionId) -> Vec<Trait> {
    let printed =
        |f: fn(&zvtes::crypt::CryptCard) -> bool| st.minions[m.index()].vampire().is_some_and(f);
    [
        (st.is_black_hand(m), Trait::BlackHand),
        (st.is_infernal(m), Trait::Infernal),
        (st.is_red_list(m), Trait::RedList),
        (st.blood_cursed(m), Trait::BloodCursed),
        (printed(|c| c.scarce), Trait::Scarce),
        (printed(|c| c.sterile), Trait::Sterile),
        (st.is_seraph(m), Trait::Seraph),
    ]
    .into_iter()
    .filter_map(|(has, t)| has.then_some(t))
    .chain(path(st.minions[m.index()].path()))
    .collect()
}

/// The trait of a Sabbat path, by its name (`SABBAT_PATHS`).
fn path(name: Option<&str>) -> Option<Trait> {
    let [caine, cathari, death, power] = zvtes::state::SABBAT_PATHS;
    Some(match name? {
        n if n == caine => Trait::PathOfCaine,
        n if n == cathari => Trait::PathOfCathari,
        n if n == death => Trait::PathOfDeathAndTheSoul,
        n if n == power => Trait::PathOfPowerAndTheInnerVoice,
        other => unreachable!("not a Sabbat path: {other}"),
    })
}

/// The region a minion in play is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    Ready,
    Torpor,
    /// An imbued with no life (rulebook "Appendix: Imbued Rules" 5).
    Incapacitated,
}

impl Region {
    pub fn of(m: &zvtes::state::Minion) -> Region {
        Region::from_flags(m.in_torpor, m.incapacitated)
    }

    pub fn from_flags(torpor: bool, incapacitated: bool) -> Region {
        if incapacitated {
            Region::Incapacitated
        } else if torpor {
            Region::Torpor
        } else {
            Region::Ready
        }
    }

    pub fn zone(self) -> zvtes_api::ids::Zone {
        match self {
            Region::Ready => zvtes_api::ids::Zone::Ready,
            Region::Torpor => zvtes_api::ids::Zone::Torpor,
            Region::Incapacitated => zvtes_api::ids::Zone::Incapacitated,
        }
    }
}

pub(crate) fn minion_kind(m: &zvtes::state::Minion) -> MinionKind {
    match m.kind {
        Mk::Vampire(_) => MinionKind::Vampire,
        Mk::Imbued(_) => MinionKind::Imbued,
        Mk::Ally { .. } => MinionKind::Ally,
        Mk::Created { .. } | Mk::GreatBeast { .. } | Mk::Fury { .. } => MinionKind::Created,
        Mk::Token { .. } => MinionKind::Token,
    }
}

/// A minion's face: its card, crypt or library.
pub(crate) fn minion_face(m: &zvtes::state::Minion) -> (u16, bool) {
    match m.kind {
        Mk::Vampire(c) | Mk::Imbued(c) => (c, true),
        Mk::Ally { card, .. }
        | Mk::Created { card, .. }
        | Mk::GreatBeast { card, .. }
        | Mk::Fury { card, .. }
        | Mk::Token { card, .. } => (card, false),
    }
}

/// A library card in play shown as its own card: not an ally's or created
/// vampire's body (the minion shows it), not an effect whose card is in
/// the ash heap (a marker on its minion shows it).
fn shown_card(st: &GameState, i: usize) -> bool {
    let c = &st.cards[i];
    c.in_play
        && !c.effect_only
        && c.body.is_none()
        && !st.minions.iter().any(|m| {
            m.in_play
                && matches!(m.kind, Mk::Created { id, .. }
                    | Mk::GreatBeast { id, .. }
                    | Mk::Fury { id, .. } if id.index() == i)
        })
}

pub fn snake(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .replace(['\'', '’'], "")
        .replace([' ', '-'], "_")
}

/// A title as a snake_case id for an option's `detail`: `primogen`,
/// `prince_of_paris`, `justicar_of_ventrue`.
pub(crate) fn title_detail(t: Ti) -> String {
    match title(t) {
        Some(Title {
            id, of: Some(of), ..
        }) => format!("{id}_of_{}", snake(&of)),
        Some(Title { id, .. }) => id,
        None => "none".into(),
    }
}

pub(crate) fn title(t: Ti) -> Option<Title> {
    let make = |id: &str, of: Option<&str>, votes: Option<u8>| {
        Some(Title {
            id: id.into(),
            of: of.map(Into::into),
            votes,
        })
    };
    match t {
        Ti::None => None,
        Ti::Primogen => make("primogen", None, None),
        Ti::Prince(c) => make("prince", Some(c), None),
        Ti::Baron(c) => make("baron", Some(c), None),
        Ti::Justicar(c) => make("justicar", Some(c), None),
        Ti::InnerCircle(c) => make("inner_circle", Some(c), None),
        Ti::Archbishop(c) => make("archbishop", Some(c), None),
        Ti::Bishop => make("bishop", None, None),
        Ti::Cardinal => make("cardinal", None, None),
        Ti::Priscus => make("priscus", None, None),
        Ti::Regent => make("regent", None, None),
        Ti::Magaji => make("magaji", None, None),
        Ti::Independent { name, votes } => make("independent", Some(name), Some(votes)),
        Ti::Laibon { name, votes } => make("laibon", Some(name), Some(votes)),
        Ti::Votes(n) => make("votes", None, Some(n)),
    }
}

pub(crate) fn strike(k: Sk, card: Option<u32>) -> Strike {
    let mut s = Strike {
        kind: StrikeKind::Unknown,
        n: None,
        ranged: false,
        aggravated: false,
        card,
        resolved: None,
        text: None,
    };
    s.kind = match k {
        Sk::Hand { bonus } => {
            s.n = Some(1 + bonus);
            StrikeKind::Hand
        }
        Sk::Melee { aggravated, .. } => {
            s.aggravated = aggravated;
            StrikeKind::Melee
        }
        Sk::Dodge => StrikeKind::Dodge,
        Sk::CombatEnds { .. } => StrikeKind::CombatEnds,
        Sk::Damage {
            n,
            ranged,
            aggravated,
        } => {
            s.n = Some(n);
            s.ranged = ranged;
            s.aggravated = aggravated;
            StrikeKind::Damage
        }
        Sk::StealBlood { n, ranged } => {
            s.n = Some(n);
            s.ranged = ranged;
            StrikeKind::StealBlood
        }
        Sk::TorporOrBurn => StrikeKind::TorporOrBurn,
        Sk::Torpor { .. } => StrikeKind::Torpor,
        Sk::CardEffect { .. } => StrikeKind::CardEffect,
        Sk::BurnWeapon => StrikeKind::BurnWeapon,
        Sk::StealWeapon => StrikeKind::StealWeapon,
        Sk::BurnEquipment => StrikeKind::BurnEquipment,
        Sk::RangedBurnEquipment => {
            s.ranged = true;
            StrikeKind::RangedBurnEquipment
        }
        Sk::Corrupt => StrikeKind::Corrupt,
        // Earthshock: a ranged damage strike, its amount told live.
        Sk::RangedStrength { .. } => {
            s.ranged = true;
            StrikeKind::Damage
        }
        Sk::BurnLife { n } => {
            s.n = Some(n);
            s.ranged = true;
            StrikeKind::BurnLife
        }
    };
    s
}

/// A strike with the damage the engine says it inflicts now, when it is a
/// damage strike in a live combat (`CombatCtx::strike_told`).
pub(crate) fn told(mut s: Strike, hit: Option<zvtes::combat::StrikeHit>) -> Strike {
    if let Some(h) = hit {
        s.n = Some(h.n);
        s.aggravated = h.aggravated;
    }
    s
}

/// A minion's name as numbering reads it: its printed name, every ghoul
/// token alike.
pub(crate) fn minion_base(m: &zvtes::state::Minion) -> String {
    match minion_kind(m) {
        MinionKind::Token => "ghoul token".into(),
        _ => crate::cards::printed(m.name()),
    }
}

/// The card types a mode's tags may name (a multi-type card's line).
const CARD_TYPES: [(&str, &str); 6] = [
    ("ACTION", "action"),
    ("ACTIONMODIFIER", "action_modifier"),
    ("REACTION", "reaction"),
    ("COMBAT", "combat"),
    ("REFLEX", "reflex"),
    ("POLITICAL", "political_action"),
];

/// A play's mode: its printed line's Disciplines (level by case) and type.
pub fn mode(card: u16, mode: u8) -> Mode {
    let printed = zvtes::cards::lookup(card)
        .filter(|def| (mode as usize) < def.modes.len())
        .map(|_| zvtes::cards::library_mode(card, mode as usize));
    let tags = printed
        .and_then(|m| LIBRARY[card as usize].modes.get(m))
        .map_or(&[][..], |m| m.tags);
    let mut out = Mode {
        index: mode,
        line: printed.unwrap_or(mode as usize) as u8,
        disciplines: vec![],
        card_type: None,
        apart: apart(card, mode as usize),
    };
    for t in tags {
        match CARD_TYPES.iter().find(|(tag, _)| tag == t) {
            Some((_, id)) => out.card_type = Some((*id).into()),
            None => out.disciplines.push(DisciplineLevel {
                discipline: t.to_lowercase(),
                superior: !t.chars().any(|c| c.is_lowercase()),
            }),
        }
    }
    out
}

/// What sets a mode apart from the other modes its printed line splits
/// into: its effects that not all of them have ("gain 3 blood" against
/// "gain 2 blood and unlock").
fn apart(card: u16, mode: usize) -> Vec<zvtes_api::option::Effect> {
    let Some(def) = zvtes::cards::lookup(card) else {
        return Vec::new();
    };
    let Some(own) = def.modes.get(mode) else {
        return Vec::new();
    };
    let printed = zvtes::cards::library_mode(card, mode);
    let siblings = (0..def.modes.len())
        .filter(|&i| i != mode && zvtes::cards::library_mode(card, i) == printed)
        .map(|i| &def.modes[i]);
    apart_from(own, siblings)
}

/// `own`'s effects that not all of `siblings` have, curated for no one in
/// particular (alike on every option of `own`); none without siblings.
pub(crate) fn apart_from<'a>(
    own: &zvtes::cards::ModeDef,
    siblings: impl Iterator<Item = &'a zvtes::cards::ModeDef>,
) -> Vec<zvtes_api::option::Effect> {
    let siblings: Vec<_> = siblings.collect();
    if siblings.is_empty() {
        return Vec::new();
    }
    let effects: Vec<_> = own
        .effects
        .iter()
        .filter(|e| siblings.iter().any(|m| !m.effects.contains(e)))
        .copied()
        .collect();
    crate::effects::curated(&effects, crate::effects::Whom::default())
}

pub fn phase(p: P) -> Phase {
    match p {
        P::Unlock => Phase::Unlock,
        P::Master => Phase::Master,
        P::Minion => Phase::Minion,
        P::Influence => Phase::Influence,
        P::Discard => Phase::Discard,
        P::Over => Phase::Over,
    }
}

pub fn action_kind(k: Ak) -> ActionKind {
    match k {
        Ak::Bleed => ActionKind::Bleed,
        Ak::Hunt => ActionKind::Hunt,
        Ak::LeaveTorpor => ActionKind::LeaveTorpor,
        Ak::Rescue => ActionKind::Rescue,
        Ak::Diablerie => ActionKind::Diablerie,
        Ak::Employ => ActionKind::Employ,
        Ak::Recruit => ActionKind::Recruit,
        Ak::Political => ActionKind::Political,
        Ak::Card => ActionKind::Card,
        Ak::BurnCard => ActionKind::BurnCard,
        Ak::StealCard => ActionKind::StealCard,
        Ak::Equip => ActionKind::Equip,
        Ak::EquipFrom => ActionKind::EquipFrom,
        Ak::EquipFound => ActionKind::EquipFound,
        Ak::Ability => ActionKind::Ability,
        Ak::BecomeAnarch => ActionKind::BecomeAnarch,
        Ak::RedListHunt => ActionKind::RedListHunt,
        Ak::Power => ActionKind::Power,
        Ak::BurnImbued => ActionKind::BurnImbued,
    }
}

fn action_step(s: ActionState) -> ActionStep {
    match s {
        ActionState::AsAnnounced => ActionStep::Announced,
        ActionState::Open => ActionStep::Open,
        ActionState::Attempt(_) => ActionStep::Attempt,
        ActionState::Declined => ActionStep::Declined,
        ActionState::WouldSucceed => ActionStep::WouldSucceed,
        ActionState::Blocked(_) => ActionStep::Blocked,
        // Between Psyche!'s combats: the first one is over.
        ActionState::AfterCombat | ActionState::QueuedCombat => ActionStep::AfterCombat,
        ActionState::AfterResolution => ActionStep::Resolved,
        ActionState::AfterDiablerie(..) => ActionStep::Diablerie,
    }
}

#[cfg(test)]
mod tests {
    use zvtes::crypt::CRYPT;
    use zvtes::game::Engine;
    use zvtes::ids::SeatId;
    use zvtes_api::ids::Trait;

    use super::Viewer;
    use crate::numbering::Decks;

    /// The first crypt card matching `f`.
    fn crypt(f: impl Fn(&zvtes::crypt::CryptCard) -> bool) -> u16 {
        CRYPT
            .iter()
            .position(f)
            .map(|i| i as u16)
            .expect("a vampire")
    }

    // Minions of one name told apart (docs/protocol.md §3): the second and
    // third of a name numbered in table order, whatever their seat, kept in
    // a later view. Markers are data: a hostage's count, a lasting effect's
    // card.
    #[test]
    fn minions_of_one_name_are_numbered_in_the_view() {
        let mut e = Engine::bare(2);
        let plain = crypt(|c| !c.unique);
        let first = e.add_minion(SeatId(0), plain, 4, false);
        e.add_minion(SeatId(1), plain, 4, false);
        e.add_minion(SeatId(1), plain, 4, false);
        for (i, m) in e.state.minions.iter_mut().enumerate() {
            m.copy = zvtes::copies::Untracked(zvtes::copies::CopyId(i as u16));
        }
        e.state.minions[first.index()].hostage = 2;
        let fame = zvtes::state::find_library("Fame");
        let mut effect = zvtes::state::CardInPlay::new(fame, SeatId(1), Some(first), 0);
        effect.effect_only = true;
        e.state.cards.push(effect);
        let names = vec!["a".to_string(), "b".to_string()];
        let decks = Decks::dealt(&e);
        let mut viewer = Viewer::new(Some(SeatId(1)));
        let numbered = |v: &zvtes_api::view::TableView| -> Vec<Option<u8>> {
            v.seats
                .iter()
                .flat_map(|s| s.ready.iter().map(|m| m.card.numbered))
                .collect()
        };
        let v = viewer.view(&e, &decks, &names);
        assert!(v.seats[0].ready[0].card.copy.is_some(), "numbered copies");
        assert_eq!(numbered(&v), [None, Some(2), Some(3)]);
        assert_eq!(
            v.seats[0].ready[0]
                .markers
                .iter()
                .map(|k| (k.kind, k.n, k.card))
                .collect::<Vec<_>>(),
            [
                (zvtes_api::ids::MarkerKind::Hostage, Some(2), None),
                (
                    zvtes_api::ids::MarkerKind::Effect,
                    None,
                    Some(zvtes::library::LIBRARY[fame as usize].id)
                )
            ]
        );
        let v = viewer.view(&e, &decks, &names);
        assert_eq!(numbered(&v), [None, Some(2), Some(3)], "kept");
    }

    /// A vampire's Sabbat path is a trait: printed, or followed since
    /// (Black Chantry 2025-06-12; rules-qa 97.3).
    #[test]
    fn a_vampires_path_is_a_trait() {
        let mut e = Engine::bare(2);
        e.add_minion(
            SeatId(0),
            zvtes::state::find_card("Aaradhya, The Callous Tyrant"),
            4,
            false,
        );
        let hartmut = e.add_minion(
            SeatId(0),
            zvtes::state::find_card("Hartmut Stover"),
            4,
            false,
        );
        let names = vec!["a".to_string(), "b".to_string()];
        let traits = |e: &Engine| {
            let v = Viewer::new(Some(SeatId(1))).view(e, &Decks::dealt(e), &names);
            let r = &v.seats[0].ready;
            (r[0].traits.clone(), r[1].traits.clone())
        };
        assert_eq!(
            traits(&e),
            (vec![Trait::PathOfPowerAndTheInnerVoice], vec![])
        );
        e.state.minions[hartmut.index()].path_override = Some("Cathari");
        assert_eq!(traits(&e).1, vec![Trait::PathOfCathari]);
    }

    // A minion's traits in every viewer's view, read live: a printed Black
    // Hand vampire, one become Black Hand (Mateusz Kazimirski's way), an
    // infernal one, a Red List one (marked this turn by seat 1: the mark
    // shown), a Sterile one, a Blood Cursed one in torpor; a plain one none.
    #[test]
    fn a_minions_traits_are_in_the_view() {
        let mut e = Engine::bare(2);
        let only = |t: fn(&zvtes::crypt::CryptCard) -> bool| {
            crypt(move |c| {
                let all = [
                    c.black_hand,
                    c.infernal,
                    c.red_list,
                    c.blood_cursed,
                    c.scarce,
                    c.sterile,
                    c.seraph,
                ];
                t(c) && all.iter().filter(|&&b| b).count() == 1
            })
        };
        e.add_minion(SeatId(0), only(|c| c.black_hand), 4, false);
        let plain = crypt(|c| {
            !(c.black_hand
                || c.infernal
                || c.red_list
                || c.blood_cursed
                || c.scarce
                || c.sterile
                || c.seraph)
        });
        let gained = e.add_minion(SeatId(0), plain, 4, false);
        e.state.minions[gained.index()].gained_black_hand = true;
        e.add_minion(SeatId(0), crypt(|c| c.infernal), 4, false);
        let red = e.add_minion(SeatId(0), crypt(|c| c.red_list), 4, false);
        e.state.red_list_marks.push((SeatId(1), red));
        e.add_minion(SeatId(0), only(|c| c.sterile), 4, false);
        e.add_minion(SeatId(0), only(|c| c.blood_cursed), 0, true);
        e.add_minion(SeatId(1), plain, 4, false);
        let names = vec!["a".to_string(), "b".to_string()];
        let v = Viewer::new(Some(SeatId(1))).view(&e, &Decks::dealt(&e), &names);
        let ready = &v.seats[0].ready;
        assert_eq!(ready[0].traits, vec![Trait::BlackHand]);
        assert_eq!(ready[1].traits, vec![Trait::BlackHand], "gained");
        assert!(ready[2].traits.contains(&Trait::Infernal));
        assert!(ready[3].traits.contains(&Trait::RedList));
        let mark = ready[3]
            .markers
            .iter()
            .find(|m| m.kind == zvtes_api::ids::MarkerKind::RedListMark);
        assert_eq!(mark.and_then(|m| m.seat), Some(1), "marked by seat 1");
        assert_eq!(ready[4].traits, vec![Trait::Sterile]);
        assert_eq!(v.seats[0].torpor[0].traits, vec![Trait::BloodCursed]);
        assert!(v.seats[1].ready[0].traits.is_empty());
    }
}
