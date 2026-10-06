//! `TableView`: the table as one viewer may see it — the client-facing
//! projection of the engine's state (docs/human-play.md). Built from the
//! viewer's legal view (`GameState::legal_view`), never the engine's
//! layout serialized: clients depend on this shape only.

use serde::{Deserialize, Serialize};
use zvtes::actions::ActionState;
use zvtes::cards::Target;
use zvtes::crypt::{Title, CRYPT};
use zvtes::decision::ActionKind;
use zvtes::game::{Engine, Phase};
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::LIBRARY;
use zvtes::referendum::{RefStep, Source};
use zvtes::state::{CardId, GameState, MinionKind, HIDDEN};

use crate::track::{Copies, Face, Key, Layout, SeatZone, Tracker, ZoneId};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TableView {
    /// The viewing seat; `None`: every card shown (tests, replays).
    pub viewer: Option<u8>,
    /// The number of the next decision (the decisions answered so far).
    pub decision: u64,
    /// Game turn, 1-based.
    pub turn: u32,
    pub active: u8,
    pub phase: String,
    pub seats: Vec<SeatView>,
    pub action: Option<ActionView>,
    pub combat: Option<CombatView>,
    pub referendum: Option<ReferendumView>,
}

/// A physical card: its copy (stable across views) and, when the viewer
/// may know it, its KRCG id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardView {
    pub copy: u32,
    pub id: Option<u32>,
    /// A crypt card (else a library card), hidden or not.
    pub crypt: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeatView {
    pub seat: u8,
    pub pool: i32,
    /// Victory points (0.5 for surviving at the time limit).
    pub vps: f32,
    pub ousted: bool,
    pub edge: bool,
    pub prey: u8,
    pub predator: u8,
    /// Top last.
    pub crypt: Vec<CardView>,
    /// Top last.
    pub library: Vec<CardView>,
    pub hand: Vec<CardView>,
    pub ash_heap: Vec<CardView>,
    pub removed: Vec<CardView>,
    pub uncontrolled: Vec<SlotView>,
    /// Minions the seat controls, ready region then torpor.
    pub ready: Vec<MinionView>,
    pub torpor: Vec<MinionView>,
    /// Cards in play the seat controls that are on no minion.
    pub in_play: Vec<InPlayView>,
    /// Cards the seat is playing, not resolved yet.
    pub limbo: Vec<CardView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SlotView {
    pub card: CardView,
    pub counters: u8,
    /// A banished vampire, waiting to be influenced out again.
    pub banished: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MinionView {
    /// Arena index: what options name.
    pub minion: u32,
    pub card: CardView,
    /// "vampire", "ally", "created" (a library card as a vampire), "token".
    pub kind: String,
    pub name: String,
    /// Blood (a vampire) or life (an ally).
    pub blood: u8,
    pub capacity: Option<u8>,
    pub locked: bool,
    pub contested: bool,
    pub title: Option<String>,
    /// The Methuselah whose ash heap it goes to, when not its controller.
    pub owner: Option<u8>,
    pub cards: Vec<InPlayView>,
    /// Other public state, as short text (counters, marks).
    pub markers: Vec<String>,
    /// The corruption counters on it, all Methuselahs' (the markers say
    /// whose): what a client puts on the card.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub corruption: u8,
}

fn is_zero(n: &u8) -> bool {
    *n == 0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InPlayView {
    /// Arena index: what options name.
    pub card_id: u32,
    pub card: CardView,
    pub controller: u8,
    pub locked: bool,
    pub counters: u8,
    pub contested: bool,
    /// Cards stored on it (face down ones hidden to others), top last.
    pub stored: Vec<CardView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TargetView {
    Minion { minion: u32 },
    Seat { seat: u8 },
    Uncontrolled { slot: u8 },
    Card { card_id: u32 },
}

impl TargetView {
    pub fn of(t: Target) -> Option<TargetView> {
        match t {
            Target::None => None,
            Target::Minion(m) => Some(TargetView::Minion { minion: m.0 }),
            Target::Seat(s) => Some(TargetView::Seat { seat: s.0 }),
            Target::Uncontrolled(i) => Some(TargetView::Uncontrolled { slot: i }),
            Target::Card(c) => Some(TargetView::Card { card_id: c.0 }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionView {
    pub seat: u8,
    pub acting: u32,
    pub kind: String,
    /// The action card's KRCG id, for a card action.
    pub card: Option<u32>,
    pub target: Option<TargetView>,
    pub bleed_target: Option<u8>,
    /// The card in play granting the action (engine card id): a location's
    /// action, an equipment's.
    #[serde(default)]
    pub source: Option<u32>,
    /// Where it stands: "announced", "open" (blocks may be attempted),
    /// "attempt" (a block attempted), "declined" (no more blocks), "would
    /// succeed", "blocked", "after combat", "resolved".
    #[serde(default)]
    pub step: String,
    #[serde(default)]
    pub stealth: i32,
    /// The bleed amount, for a bleed.
    #[serde(default)]
    pub bleed: Option<u8>,
    /// The minion attempting the block, or that blocked, and its intercept.
    #[serde(default)]
    pub blocker: Option<u32>,
    #[serde(default)]
    pub intercept: Option<i32>,
    /// The Methuselahs who declined to block.
    #[serde(default)]
    pub declined: Vec<u8>,
    /// The action card's mode (its Disciplines and level) and X, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<crate::describe::ModeView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CombatView {
    pub minions: [u32; 2],
    pub round: u8,
    pub range: String,
    /// Each side (the acting minion's first): its strike once declared —
    /// the blocker chooses knowing the acting minion's — and the
    /// maneuvers and presses it has for the round.
    #[serde(default)]
    pub sides: Vec<CombatSideView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CombatSideView {
    pub minion: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strike: Option<String>,
    pub maneuvers: u8,
    pub presses: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReferendumView {
    pub caller: u8,
    pub calling: Option<u32>,
    pub votes_for: u32,
    pub votes_against: u32,
    /// The terms chosen so far: the Methuselah chosen, the points
    /// allocated (seat, points).
    pub chosen: Option<u8>,
    pub allocated: Vec<(u8, u8)>,
    /// The other terms chosen: Methuselahs, minions, a clan, cards in play
    /// (engine card ids) and the points they got.
    #[serde(default)]
    pub seats: Vec<u8>,
    #[serde(default)]
    pub minions: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clan: Option<String>,
    #[serde(default)]
    pub cards: Vec<u32>,
    #[serde(default)]
    pub card_allocated: Vec<(u32, u8)>,
    /// "terms", "polling", "shortfall".
    #[serde(default)]
    pub step: String,
    /// Every vote cast so far, in order.
    #[serde(default)]
    pub votes: Vec<VoteView>,
}

/// Votes cast: by whom, from what, which way, how many.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VoteView {
    pub seat: u8,
    /// "vampire" (`minion` casts them), "printed" (votes printed on
    /// `minion` it does not cast), "edge", "card" (a political action
    /// card's), "in_play" (a card in play's: `card_id`).
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minion: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_id: Option<u32>,
    pub favor: bool,
    pub n: u8,
    /// Canceled (an abstention): not tallied.
    pub canceled: bool,
}

pub fn phase_name(p: Phase) -> &'static str {
    match p {
        Phase::Unlock => "unlock",
        Phase::Master => "master",
        Phase::Minion => "minion",
        Phase::Influence => "influence",
        Phase::Discard => "discard",
        Phase::Over => "over",
    }
}

pub fn title_text(t: Title) -> Option<String> {
    match t {
        Title::None => None,
        Title::Primogen => Some("Primogen".into()),
        Title::Prince(c) => Some(format!("Prince of {c}")),
        Title::Baron(c) => Some(format!("Baron of {c}")),
        Title::Justicar(c) => Some(format!("{c} Justicar")),
        Title::InnerCircle(c) => Some(format!("{c} Inner Circle")),
        Title::Archbishop(c) => Some(format!("Archbishop of {c}")),
        Title::Independent { name, .. } => Some(name.into()),
    }
}

fn crypt_face(c: u16) -> Face {
    if c == HIDDEN {
        Face::HiddenCrypt
    } else {
        Face::Crypt(c)
    }
}

fn library_face(c: u16) -> Face {
    if c == HIDDEN {
        Face::HiddenLibrary
    } else {
        Face::Library(c)
    }
}

fn card_view(copy: u32, f: Face) -> CardView {
    let (id, crypt) = match f {
        Face::Crypt(c) => (Some(CRYPT[c as usize].id), true),
        Face::Library(c) => (Some(LIBRARY[c as usize].id), false),
        Face::HiddenCrypt => (None, true),
        Face::HiddenLibrary => (None, false),
    };
    CardView { copy, id, crypt }
}

/// A minion's arena key and face: an ally or created vampire is its card.
fn minion_key(m: MinionId, kind: MinionKind) -> (Key, Face) {
    match kind {
        MinionKind::Vampire(c) => (Key::Minion(m.0), Face::Crypt(c)),
        MinionKind::Ally { card, id, .. } | MinionKind::Created { card, id, .. } => {
            (Key::Card(id.0), Face::Library(card))
        }
        MinionKind::Token { card, .. } => (Key::Minion(m.0), Face::Library(card)),
    }
}

/// On the table: in play, or contested (face down, out of play, still there).
fn on_table(st: &GameState, m: usize) -> bool {
    st.minions[m].in_play
}

/// A library card in play shown as its own card: not an ally's or created
/// vampire's body (the minion shows it), not an effect whose card is in
/// the ash heap (rules-qa 21.1: a marker on its minion shows it).
fn shown_card(st: &GameState, i: usize) -> bool {
    let c = &st.cards[i];
    c.in_play && !c.effect_only && c.body.is_none() && !is_created_body(st, i)
}

fn is_created_body(st: &GameState, i: usize) -> bool {
    st.minions
        .iter()
        .any(|m| m.in_play && matches!(m.kind, MinionKind::Created { id, .. } if id.index() == i))
}

/// The seat's zones in the engine's lists (ash heap: library cards, then
/// crypt cards; likewise removed). `face_down`: a viewer's table — every
/// crypt and library shown face down, the viewer's own included (the legal
/// view gives a seat its own remaining cards, sorted, for the policy; a
/// player may not look through them).
fn zones(st: &GameState, s: usize, face_down: bool) -> [(SeatZone, Vec<Face>); 6] {
    let seat = &st.seats[s];
    let lib = |v: &[u16]| v.iter().map(|&c| library_face(c)).collect::<Vec<_>>();
    let cry = |v: &[u16]| v.iter().map(|&c| crypt_face(c)).collect::<Vec<_>>();
    let mut ash = lib(&seat.ash);
    ash.extend(cry(&seat.ash_crypt));
    let mut removed = lib(&seat.removed);
    removed.extend(cry(&seat.removed_crypt));
    [
        (
            SeatZone::Crypt,
            if face_down {
                vec![Face::HiddenCrypt; seat.crypt.len()]
            } else {
                cry(&seat.crypt)
            },
        ),
        (
            SeatZone::Library,
            if face_down {
                vec![Face::HiddenLibrary; seat.library.len()]
            } else {
                lib(&seat.library)
            },
        ),
        (SeatZone::Hand, lib(&seat.hand)),
        (SeatZone::AshHeap, ash),
        (SeatZone::Removed, removed),
        (
            SeatZone::Uncontrolled,
            seat.uncontrolled
                .iter()
                .map(|u| crypt_face(u.card))
                .collect(),
        ),
    ]
}

/// The cards each seat is playing: the plays under way, then the action's
/// card when not among them — until it enters play (a title card) or
/// returns to hand (Delaying Tactics).
fn limbo(e: &Engine) -> Vec<Vec<Face>> {
    let mut out = vec![Vec::new(); e.state.seats.len()];
    let flight = e.in_flight();
    for p in &flight {
        out[p.seat.index()].push(Face::Library(p.card));
    }
    let held = e
        .action()
        .filter(|a| !a.card_entered_play && !a.card_returned)
        .and_then(|a| a.card.as_ref());
    if let Some(p) = held {
        let listed = flight
            .iter()
            .any(|q| q.seat == p.seat && q.card == p.card && q.actor == p.actor);
        if !listed {
            out[p.seat.index()].push(Face::Library(p.card));
        }
    }
    out
}

fn layout(e: &Engine, st: &GameState, face_down: bool) -> Layout {
    let mut l = Layout::default();
    for (i, m) in st.minions.iter().enumerate() {
        if on_table(st, i) {
            let (k, f) = minion_key(MinionId(i as u32), m.kind);
            l.arena.insert(k, (f, m.owner.0));
        }
    }
    for (i, c) in st.cards.iter().enumerate() {
        if shown_card(st, i) {
            let seat = st.card_controller(CardId(i as u32)).0;
            l.arena
                .insert(Key::Card(i as u32), (Face::Library(c.card), seat));
            if !c.stored.is_empty() {
                let faces = c.stored.iter().map(|&x| library_face(x)).collect();
                l.zones.insert(ZoneId::Stored(i as u32), (faces, seat));
            }
        }
    }
    for s in 0..st.seats.len() {
        for (z, faces) in zones(st, s, face_down) {
            if !faces.is_empty() {
                l.zones.insert(ZoneId::Seat(s as u8, z), (faces, s as u8));
            }
        }
    }
    for (s, faces) in limbo(e).into_iter().enumerate() {
        if !faces.is_empty() {
            l.zones
                .insert(ZoneId::Seat(s as u8, SeatZone::Limbo), (faces, s as u8));
        }
    }
    l
}

/// The table as `viewer` sees it (`None`: everything), continuing
/// `tracker`'s copies — one tracker per viewer, fed every view in turn.
pub fn table_view(e: &Engine, viewer: Option<SeatId>, tracker: &mut Tracker) -> TableView {
    let st = match viewer {
        Some(v) => e.state.legal_view(v),
        None => e.state.clone(),
    };
    let lay = layout(e, &st, viewer.is_some());
    let copies = tracker.update(&lay).clone();
    let seats = (0..st.seats.len())
        .map(|s| seat_view(&st, &lay, &copies, s))
        .collect();
    TableView {
        viewer: viewer.map(|v| v.0),
        decision: e.log.len() as u64,
        turn: st.turn,
        active: st.active.0,
        phase: phase_name(e.phase()).into(),
        seats,
        action: e.action().map(|a| ActionView {
            seat: a.seat.0,
            acting: a.acting.0,
            kind: crate::describe::action_kind(a.kind).into(),
            card: a.card.as_ref().map(|p| LIBRARY[p.card as usize].id),
            // A granted action's target, or the action card's.
            target: TargetView::of(a.target)
                .or_else(|| a.card.as_ref().and_then(|p| TargetView::of(p.target))),
            bleed_target: a.bleed_target.map(|s| s.0),
            source: match a.source {
                Some(zvtes::cards::AbilitySource::Card(c)) => Some(c.0),
                _ => None,
            },
            step: action_step(a.state).into(),
            stealth: a.stealth(&e.state),
            bleed: (a.kind == ActionKind::Bleed).then(|| a.bleed_amount(&e.state)),
            blocker: blocker(a.state).map(|m| m.0),
            intercept: blocker(a.state).map(|m| a.intercept_of(&e.state, m)),
            declined: a.declined.iter().map(|s| s.0).collect(),
            mode: a
                .card
                .as_ref()
                .map(|p| crate::describe::mode_view(p.card, p.mode)),
            x: a.card.as_ref().map(|p| p.x).filter(|&x| x > 0),
        }),
        combat: e.combat().map(|c| CombatView {
            minions: [c.sides[0].minion.0, c.sides[1].minion.0],
            round: c.round,
            range: format!("{:?}", c.range).to_lowercase(),
            sides: c
                .sides
                .iter()
                .map(|s| CombatSideView {
                    minion: s.minion.0,
                    strike: crate::narrate::declared_strike(e, s),
                    maneuvers: s.maneuvers,
                    presses: s.presses,
                })
                .collect(),
        }),
        referendum: e.referendum().map(|r| {
            let tally = |favor: bool| {
                r.records
                    .iter()
                    .filter(|v| v.favor == favor && !v.canceled)
                    .map(|v| v.n as u32)
                    .sum()
            };
            ReferendumView {
                caller: r.caller.0,
                calling: r.calling.map(|m| m.0),
                votes_for: tally(true),
                votes_against: tally(false),
                chosen: r.terms.chosen.map(|s| s.0),
                allocated: r.terms.alloc.iter().map(|&(s, n)| (s.0, n)).collect(),
                seats: r.terms.seats.iter().map(|s| s.0).collect(),
                minions: r.terms.minions.iter().map(|m| m.0).collect(),
                clan: r.terms.clan.map(Into::into),
                cards: r.terms.cards.iter().map(|c| c.0).collect(),
                card_allocated: r.terms.card_alloc.iter().map(|&(c, n)| (c.0, n)).collect(),
                step: match r.step {
                    RefStep::Terms => "terms",
                    RefStep::Polling => "polling",
                    RefStep::Shortfall { .. } => "shortfall",
                    RefStep::Choose => "choose",
                }
                .into(),
                votes: r
                    .records
                    .iter()
                    .map(|v| {
                        let (source, minion, card_id) = match v.source {
                            Source::Vampire(m) => ("vampire", Some(m.0), None),
                            Source::Printed(m) => ("printed", Some(m.0), None),
                            Source::Edge(_) => ("edge", None, None),
                            Source::Card(_) => ("card", None, None),
                            Source::InPlay(c) => ("in_play", None, Some(c.0)),
                        };
                        VoteView {
                            seat: v.seat.0,
                            source: source.into(),
                            minion,
                            card_id,
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

fn action_step(s: ActionState) -> &'static str {
    match s {
        ActionState::AsAnnounced => "announced",
        ActionState::Open => "open",
        ActionState::Attempt(_) => "attempt",
        ActionState::Declined => "declined",
        ActionState::WouldSucceed => "would succeed",
        ActionState::Blocked(_) => "blocked",
        ActionState::AfterCombat => "after combat",
        ActionState::AfterResolution => "resolved",
    }
}

/// The minion attempting the block, or that blocked.
pub(crate) fn blocker(s: ActionState) -> Option<MinionId> {
    match s {
        ActionState::Attempt(m) | ActionState::Blocked(m) => Some(m),
        _ => None,
    }
}

fn zone_cards(lay: &Layout, copies: &Copies, z: ZoneId) -> Vec<CardView> {
    match lay.zones.get(&z) {
        Some((faces, _)) => faces
            .iter()
            .zip(copies.zone(z))
            .map(|(&f, &c)| card_view(c, f))
            .collect(),
        None => Vec::new(),
    }
}

fn in_play_view(st: &GameState, lay: &Layout, copies: &Copies, i: usize) -> InPlayView {
    let c = &st.cards[i];
    InPlayView {
        card_id: i as u32,
        card: card_view(copies.arena(Key::Card(i as u32)), Face::Library(c.card)),
        controller: st.card_controller(CardId(i as u32)).0,
        locked: c.locked,
        counters: c.counters,
        contested: c.contested,
        stored: zone_cards(lay, copies, ZoneId::Stored(i as u32)),
    }
}

fn minion_view(st: &GameState, lay: &Layout, copies: &Copies, i: usize) -> MinionView {
    let m = &st.minions[i];
    let id = MinionId(i as u32);
    let (key, face) = minion_key(id, m.kind);
    let kind = match m.kind {
        MinionKind::Vampire(_) => "vampire",
        MinionKind::Ally { .. } => "ally",
        MinionKind::Created { .. } => "created",
        MinionKind::Token { .. } => "token",
    };
    let cards = (0..st.cards.len())
        .filter(|&c| shown_card(st, c) && st.cards[c].host == Some(id))
        .map(|c| in_play_view(st, lay, copies, c))
        .collect();
    let mut markers = Vec::new();
    for &(seat, n) in &m.corruption {
        markers.push(format!("{n} corruption (seat {})", seat.0));
    }
    if m.hostage > 0 {
        markers.push(format!("{} hostage", m.hostage));
    }
    if m.nightmare > 0 {
        markers.push(format!("{} nightmare", m.nightmare));
    }
    // Lingering effects whose card is in the ash heap (rules-qa 21.1).
    for c in st
        .cards
        .iter()
        .filter(|c| c.in_play && c.effect_only && c.host == Some(id))
    {
        let name = LIBRARY[c.card as usize].name;
        if c.counters > 0 {
            markers.push(format!("{name}: {} counters", c.counters));
        } else {
            markers.push(format!("{name} (effect)"));
        }
    }
    if m.title_contested {
        markers.push("title contested".into());
    }
    if m.expelled {
        markers.push("expelled".into());
    }
    MinionView {
        minion: id.0,
        card: card_view(copies.arena(key), face),
        kind: kind.into(),
        name: m.name().into(),
        blood: m.blood,
        capacity: st.capacity_of(id),
        locked: m.locked,
        contested: m.contested,
        title: if m.is_vampire() {
            title_text(st.title_of(id))
        } else {
            None
        },
        owner: m.origin.filter(|&o| o != m.owner).map(|o| o.0),
        cards,
        markers,
        corruption: m.corruption.iter().map(|&(_, n)| n).sum(),
    }
}

fn seat_view(st: &GameState, lay: &Layout, copies: &Copies, s: usize) -> SeatView {
    let seat = &st.seats[s];
    let sid = SeatId(s as u8);
    let z = |k| zone_cards(lay, copies, ZoneId::Seat(s as u8, k));
    let uncontrolled = z(SeatZone::Uncontrolled)
        .into_iter()
        .zip(&seat.uncontrolled)
        .map(|(card, u)| SlotView {
            card,
            counters: u.counters,
            banished: u.banished.is_some(),
        })
        .collect();
    let mine: Vec<usize> = (0..st.minions.len())
        .filter(|&i| on_table(st, i) && st.minions[i].owner == sid)
        .collect();
    let minions = |torpor: bool| {
        mine.iter()
            .filter(|&&i| st.minions[i].in_torpor == torpor)
            .map(|&i| minion_view(st, lay, copies, i))
            .collect()
    };
    let in_play = (0..st.cards.len())
        .filter(|&c| {
            shown_card(st, c)
                && st.cards[c].host.is_none()
                && st.card_controller(CardId(c as u32)) == sid
        })
        .map(|c| in_play_view(st, lay, copies, c))
        .collect();
    SeatView {
        seat: s as u8,
        pool: seat.pool,
        vps: seat.half_vps as f32 / 2.0,
        ousted: seat.ousted,
        edge: seat.has_edge,
        prey: st.prey(sid).0,
        predator: st.predator(sid).0,
        crypt: z(SeatZone::Crypt),
        library: z(SeatZone::Library),
        hand: z(SeatZone::Hand),
        ash_heap: z(SeatZone::AshHeap),
        removed: z(SeatZone::Removed),
        uncontrolled,
        ready: minions(false),
        torpor: minions(true),
        in_play,
        limbo: z(SeatZone::Limbo),
    }
}

/// What a seat knows of the cards: how many of each card it sees (crypt
/// or library, KRCG id), wherever they are. It grows when something hidden
/// is revealed to it (a draw, a card turned face up, a random discard);
/// moving a known card leaves it as it was.
pub type Knowledge = std::collections::HashMap<(bool, u32), u32>;

pub fn knowledge(e: &Engine, seat: SeatId) -> Knowledge {
    fn walk(v: &serde_json::Value, k: &mut Knowledge) {
        match v {
            serde_json::Value::Object(o) => {
                if let (Some(_), Some(id), Some(crypt)) = (
                    o.get("copy"),
                    o.get("id").and_then(|i| i.as_u64()),
                    o.get("crypt").and_then(|c| c.as_bool()),
                ) {
                    *k.entry((crypt, id as u32)).or_default() += 1;
                }
                o.values().for_each(|v| walk(v, k));
            }
            serde_json::Value::Array(a) => a.iter().for_each(|v| walk(v, k)),
            _ => {}
        }
    }
    let view = table_view(e, Some(seat), &mut Tracker::default());
    let mut k = Knowledge::new();
    walk(
        &serde_json::to_value(&view).expect("views serialize"),
        &mut k,
    );
    k
}

/// `now` knows nothing `before` did not.
pub fn learned_nothing(before: &Knowledge, now: &Knowledge) -> bool {
    now.iter()
        .all(|(card, &n)| before.get(card).copied().unwrap_or(0) >= n)
}
