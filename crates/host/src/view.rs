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
    VoteSource,
};
use zvtes_api::option::{DisciplineLevel, Mode};
use zvtes_api::view::{
    ActionView, Allocated, Card, CombatSide, CombatView, InPlayView, Marker, MinionView,
    ReferendumView, SeatView, SlotView, Strike, TableView, Target, Terms, Title, Vote,
};

use crate::numbering::{Decks, Numbering, Seen};

/// One viewer's view of the game: its numbering kept across views.
#[derive(Clone, Debug, Default)]
pub struct Viewer {
    pub seat: Option<SeatId>,
    pub numbering: Numbering,
}

impl Viewer {
    pub fn new(seat: Option<SeatId>) -> Viewer {
        Viewer {
            seat,
            numbering: Numbering::default(),
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
        Build {
            e,
            st: &st,
            names,
            numbering: Some(&self.numbering),
            seen: RefCell::new(Vec::new()),
        }
        .table()
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
    /// `None`: the first pass, collecting.
    numbering: Option<&'a Numbering>,
    seen: RefCell<Vec<Seen>>,
}

impl Build<'_> {
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
            (Some(id), false) => match self.numbering {
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
            .and_then(|n| n.get(c))
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
                    .map(|s| CombatSide {
                        minion: self.minion_ref(s.minion),
                        strike: s.chosen.then_some(s.strike).flatten().map(|k| {
                            strike(
                                k,
                                s.strike_source
                                    .map(|c| LIBRARY[e.state.cards[c.index()].card as usize].id),
                            )
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
        let minions = |torpor: bool| {
            mine.iter()
                .filter(|&&i| st.minions[i].in_torpor == torpor)
                .map(|&i| self.minion(i))
                .collect()
        };
        SeatView {
            seat: s as u8,
            name: self.names.get(s).cloned().unwrap_or_default(),
            pool: seat.pool,
            vp_halves: seat.half_vps,
            ousted: seat.ousted,
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
                    card: self.card(*u.copy, u.card, true),
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
            ready: minions(false),
            torpor: minions(true),
            in_play: (0..st.cards.len())
                .filter(|&c| {
                    shown_card(st, c)
                        && st.cards[c].host.is_none()
                        && st.card_controller(CardId(c as u32)) == sid
                })
                .map(|c| self.in_play(c))
                .collect(),
            limbo: self.limbo(s),
        }
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
        ] {
            if n > 0 {
                markers.push(marker(kind, n));
            }
        }
        for (kind, on) in [
            (MarkerKind::Disease, m.disease),
            (MarkerKind::TitleContested, m.title_contested),
            (MarkerKind::Expelled, m.expelled),
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
            card: self.card(minion_copy(st, i), card, crypt),
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
        }
    }
}

pub(crate) fn minion_kind(m: &zvtes::state::Minion) -> MinionKind {
    match m.kind {
        Mk::Vampire(_) => MinionKind::Vampire,
        Mk::Ally { .. } => MinionKind::Ally,
        Mk::Created { .. } | Mk::GreatBeast { .. } | Mk::Fury { .. } => MinionKind::Created,
        Mk::Token { .. } => MinionKind::Token,
    }
}

/// A minion's face: its card, crypt or library.
pub(crate) fn minion_face(m: &zvtes::state::Minion) -> (u16, bool) {
    match m.kind {
        Mk::Vampire(c) => (c, true),
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

fn title(t: Ti) -> Option<Title> {
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
    };
    s
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
    let siblings: Vec<_> = (0..def.modes.len())
        .filter(|&i| i != mode && zvtes::cards::library_mode(card, i) == printed)
        .map(|i| &def.modes[i])
        .collect();
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
        ActionState::AfterCombat => ActionStep::AfterCombat,
        ActionState::AfterResolution => ActionStep::Resolved,
        ActionState::AfterDiablerie(..) => ActionStep::Diablerie,
    }
}
