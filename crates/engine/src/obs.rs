//! Observations (DESIGN §5): a decision request as the policy sees it —
//! entity tokens and option rows, from what the deciding seat's legal view
//! shows. Every token and option has integer ids (for embeddings and
//! pointers) and float features; widths are fixed, counts are not (the
//! batching layer pads). Pointers are token indices + 1 (0 = none), so the
//! policy scores options against the tokens they name (pointer-network
//! style): a new card is a new vocabulary id, never a new output.
//!
//! The encoder reads only what the viewer may know: seats' public zones as
//! sizes, the viewer's own hand, what remains of their own library and
//! crypt (contents, never the order), every minion and card in play, the
//! viewer's own face-down uncontrolled vampires (others' as `HIDDEN`, a
//! banished one excepted), the action's public fields (never the card it
//! names from hand), the combat and referendum under way. `tests/obs.rs`
//! pins it: the encoding of the full state equals that of its legal view.
//!
//! Two versions ([`Version`]): a trained network keeps the one it was
//! trained on. `V2` adds what `V1` lacks: the ash heaps' contents (public:
//! each seat's deck as it is revealed) and the table's memory, the last
//! public choices (`Engine::history`). `V3` (owner, 2026-10-05: lighter
//! training) reads the same, in fewer tokens: the viewer's library and
//! crypt as a separate list (`Obs::deck_ids`, pooled by the network, out of
//! attention), the other Methuselahs' face-down uncontrolled vampires as
//! their seat's count and counters, and the transfers left in the
//! influence phase.

use crate::cards::{AbilitySource, Target};
use crate::crypt::{Title, CRYPT};
use crate::decision::{ActionKind, Choice, DecisionRequest, WindowLabel, Zone};
use crate::game::{Contest, Engine};
use crate::ids::{MinionId, SeatId};
use crate::library::LIBRARY;
use crate::state::{CardId, GameState, MinionKind, HIDDEN};

/// Token and option id widths.
pub const TOK_IDS: usize = 4;
pub const OPT_IDS: usize = 5;
/// Token and option feature widths.
pub const TOK_F: usize = 64;
pub const OPT_F: usize = 24;

/// Token types (`ids[0]`).
pub const T_SEAT: i32 = 1;
pub const T_MINION: i32 = 2;
pub const T_HAND: i32 = 3;
pub const T_CARD: i32 = 4;
pub const T_SLOT: i32 = 5;
pub const T_CONTEXT: i32 = 6;
/// A card remaining in the viewer's own library or crypt, with its copies.
pub const T_DECK: i32 = 7;
/// The political referendum under way: the card calling it (the political
/// action card, or the card in play granting the action), the votes cast
/// so far; a second one names a chosen clan.
pub const T_REFERENDUM: i32 = 8;

/// A card in a seat's ash heap, with its copies (`V2`).
pub const T_ASH: i32 = 9;
/// A public choice (`Engine::history`, `V2`): `T_EVENT` + its choice kind
/// (`opt_ids[0]`), a token type per kind.
pub const T_EVENT: i32 = 10;

/// The encoding's version: what a network was trained to read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Version {
    /// Every network trained before 2026-10-03.
    #[default]
    V1,
    /// `V1` plus the ash heaps' contents and the last public choices.
    V2,
    /// `V2` lighter: the deck out of the tokens (`Obs::deck_ids`), the
    /// face-down uncontrolled vampires as counts, the transfers left.
    V3,
}

/// The vocabulary of `ids[1]`: 0 none, then the crypt, the library, a
/// hidden card, the windows (context token), the clans (a referendum's
/// clan choice).
pub const V_CRYPT: i32 = 1;
pub const V_LIBRARY: i32 = V_CRYPT + CRYPT.len() as i32;
pub const V_HIDDEN: i32 = V_LIBRARY + LIBRARY.len() as i32;
pub const V_WINDOW: i32 = V_HIDDEN + 1;
pub const N_WINDOWS: usize = WindowLabel::CombatEndOfRound as usize + 1;
pub const V_CLAN: i32 = V_WINDOW + N_WINDOWS as i32;
pub const N_ACTION_KINDS: usize = ActionKind::Ability as usize + 1;

/// Seats at most (relative-seat one-hots).
const SEATS: usize = 6;

// Token feature offsets.
const F_REL: usize = 0; // relative seat one-hot (the seat, or the controller)
const F_POOL: usize = 6;
const F_OUSTED: usize = 7;
const F_VPS: usize = 8;
const F_EDGE: usize = 9;
const F_HAND: usize = 10;
const F_LIBRARY: usize = 11;
const F_CRYPT: usize = 12;
const F_ASH: usize = 13;
const F_ACTIVE: usize = 14;
const F_PREY: usize = 15;
const F_PREDATOR: usize = 16;
const F_BLOOD: usize = 17; // a minion's blood or life, a card's or slot's counters, a deck card's copies
const F_CAPACITY: usize = 18;
const F_LOCKED: usize = 19;
const F_TORPOR: usize = 20;
const F_CONTESTED: usize = 21;
const F_TITLED: usize = 22;
const F_ALLY: usize = 23;
const F_BLED: usize = 24;
const F_POLITICAL: usize = 25;
const F_RECRUITED: usize = 26;
const F_EXPELLED: usize = 27;
const F_CORRUPTION: usize = 28;
const F_MARKERS: usize = 29;
const F_MODE: usize = 30;
const F_POSITION: usize = 31;
const F_TURN: usize = 32;
const F_MPA: usize = 33;
const F_HELD: usize = 34;
const F_UNCONTROLLED: usize = 35;
const F_ACTION: usize = 36; // action kind one-hot (context)
const F_LONG: usize = F_ACTION + N_ACTION_KINDS;
const F_ROUND: usize = F_LONG + 1;
const F_POLITICAL_REF: usize = F_ROUND + 1;
const F_BLOOD_HUNT: usize = F_POLITICAL_REF + 1;
const F_STEALTH: usize = F_BLOOD_HUNT + 1;
const F_BLEED_AMOUNT: usize = F_STEALTH + 1;
const F_CALLER: usize = F_BLEED_AMOUNT + 1; // referendum caller, relative one-hot
                                            // Slots of the context token's action one-hot, reused by the other token
                                            // types (they never carry it): the referendum's terms on the seat, minion
                                            // or card tokens they name (points allocated to a seat or card, a
                                            // Methuselah, minion or card chosen); the votes cast so far on the
                                            // referendum token.
const F_TERM_POINTS: usize = F_ACTION;
const F_TERM_CHOSEN: usize = F_ACTION + 1;
const F_VOTES_FOR: usize = F_ACTION + 2;
const F_VOTES_AGAINST: usize = F_ACTION + 3;
const _: () = assert!(F_VOTES_AGAINST < F_LONG);
// An event token's features: the seat that chose (`F_REL`), how far back
// (choices, game turns), then the option's features.
const E_AGE: usize = SEATS;
const E_TURNS: usize = E_AGE + 1;
const E_OPT: usize = E_TURNS + 1;
const _: () = assert!(E_OPT + OPT_F <= TOK_F);

// Option feature offsets.
const O_N: usize = 0;
const O_FAVOR: usize = 1;
const O_MODE: usize = 2;
const O_ZONE: usize = 3;
const O_TITLE: usize = 4;
const O_ACTION: usize = 5; // action kind one-hot
const _: () = assert!(O_ACTION + N_ACTION_KINDS <= OPT_F);

/// One observation: tokens and options, row by row; with `V3`, the
/// viewer's library and crypt apart (`T_DECK` rows the network pools into
/// the context token, nothing points at them), empty before.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Obs {
    pub tok_ids: Vec<[i32; TOK_IDS]>,
    pub tok_f: Vec<[f32; TOK_F]>,
    pub opt_ids: Vec<[i32; OPT_IDS]>,
    pub opt_f: Vec<[f32; OPT_F]>,
    pub deck_ids: Vec<[i32; TOK_IDS]>,
    pub deck_f: Vec<[f32; TOK_F]>,
}

/// Token indices of the entities, for pointers (index + 1; 0 = none).
#[derive(Clone)]
struct Index {
    seats: Vec<i32>,
    minions: Vec<i32>,
    cards: Vec<i32>,
    hand: Vec<i32>,
    /// The viewer's uncontrolled slots.
    slots: Vec<i32>,
    /// Every seat's uncontrolled slots (events name another seat's).
    seat_slots: Vec<Vec<i32>>,
}

fn at(v: &[i32], i: usize) -> i32 {
    v.get(i).copied().unwrap_or(0)
}

impl Index {
    fn minion(&self, m: MinionId) -> i32 {
        at(&self.minions, m.index())
    }
    fn card(&self, c: CardId) -> i32 {
        at(&self.cards, c.index())
    }
    fn seat(&self, s: SeatId) -> i32 {
        at(&self.seats, s.index())
    }
    fn target(&self, t: Target) -> i32 {
        match t {
            Target::None => 0,
            Target::Minion(m) => self.minion(m),
            Target::Seat(s) => self.seat(s),
            Target::Uncontrolled(i) => at(&self.slots, i as usize),
            Target::Card(c) => self.card(c),
        }
    }
    fn source(&self, s: AbilitySource) -> i32 {
        match s {
            AbilitySource::Card(c) => self.card(c),
            AbilitySource::Vampire(m) => self.minion(m),
        }
    }
}

fn crypt_id(card: u16) -> i32 {
    if card == HIDDEN {
        V_HIDDEN
    } else {
        assert!((card as usize) < CRYPT.len(), "crypt card {card}");
        V_CRYPT + card as i32
    }
}

fn library_id(card: u16) -> i32 {
    if card == HIDDEN {
        V_HIDDEN
    } else {
        assert!((card as usize) < LIBRARY.len(), "library card {card}");
        V_LIBRARY + card as i32
    }
}

fn minion_id(kind: MinionKind) -> i32 {
    match kind {
        MinionKind::Vampire(c) => crypt_id(c),
        MinionKind::Ally { card, .. }
        | MinionKind::Created { card, .. }
        | MinionKind::Token { card, .. } => library_id(card),
    }
}

/// The viewer-relative seat: 0 the viewer, 1 the next seat clockwise, …
fn rel(st: &GameState, viewer: SeatId, s: SeatId) -> usize {
    let n = st.seats.len();
    (s.index() + n - viewer.index()) % n
}

fn flag(b: bool) -> f32 {
    b as u8 as f32
}

/// Encode `req` as `req.seat` sees it.
pub fn encode(e: &Engine, req: &DecisionRequest, v: Version) -> Obs {
    encode_as(e, req, v, false)
}

/// The omniscient critic's encoding (training only, DESIGN §5 "asymmetric
/// critic"): `encode`'s tokens plus every other Methuselah's hand (hand
/// tokens of their relative seat) and face-down uncontrolled vampires face
/// up. The policy never sees it.
pub fn encode_critic(e: &Engine, req: &DecisionRequest, v: Version) -> Obs {
    encode_as(e, req, v, true)
}

fn encode_as(e: &Engine, req: &DecisionRequest, v: Version, omniscient: bool) -> Obs {
    let st = &e.state;
    let viewer = req.seat;
    let clans = crate::referendum::clans();
    let political = e
        .referendum()
        .filter(|r| matches!(r.kind, crate::referendum::RefKind::Political));
    let terms = political.map(|r| &r.terms);
    let card_v = |c: CardId| library_id(st.cards[c.index()].card);
    let mut o = Obs::default();
    let mut ix = Index {
        seats: vec![0; st.seats.len()],
        minions: vec![0; st.minions.len()],
        cards: vec![0; st.cards.len()],
        hand: Vec::new(),
        slots: Vec::new(),
        seat_slots: vec![Vec::new(); st.seats.len()],
    };
    let push = |o: &mut Obs, ids: [i32; TOK_IDS], f: [f32; TOK_F]| {
        o.tok_ids.push(ids);
        o.tok_f.push(f);
        o.tok_ids.len() as i32
    };

    // Seats, in relative order.
    for k in 0..st.seats.len() {
        let s = SeatId(((viewer.index() + k) % st.seats.len()) as u8);
        let seat = &st.seats[s.index()];
        let mut f = [0.0; TOK_F];
        f[F_REL + k.min(SEATS - 1)] = 1.0;
        f[F_POOL] = seat.pool as f32 / 30.0;
        f[F_OUSTED] = flag(seat.ousted);
        f[F_VPS] = seat.half_vps as f32 / 10.0;
        f[F_EDGE] = flag(seat.has_edge);
        f[F_HAND] = seat.hand.len() as f32 / 10.0;
        f[F_LIBRARY] = seat.library.len() as f32 / 80.0;
        f[F_CRYPT] = seat.crypt.len() as f32 / 12.0;
        f[F_ASH] = (seat.ash.len() + seat.ash_crypt.len()) as f32 / 40.0;
        f[F_ACTIVE] = flag(st.active == s);
        f[F_PREY] = flag(!seat.ousted && s != viewer && st.prey(viewer) == s);
        f[F_PREDATOR] = flag(!seat.ousted && s != viewer && st.predator(viewer) == s);
        f[F_TURN] = st.turn as f32 / 100.0;
        f[F_MPA] = (seat.mpa + seat.dpa) as f32 / 3.0;
        f[F_HELD] = seat.held.total() as f32 / 5.0;
        f[F_UNCONTROLLED] = seat.uncontrolled.len() as f32 / 6.0;
        if v == Version::V3 {
            // The face-down vampires' counters (public), their slots no
            // tokens.
            let hidden: u32 = (seat.uncontrolled.iter())
                .filter(|slot| !(omniscient || s == viewer || slot.banished.is_some()))
                .map(|slot| slot.counters as u32)
                .sum();
            f[F_BLOOD] = hidden as f32 / 20.0;
        }
        if let Some(t) = terms {
            let points: u32 = (t.alloc.iter().chain(&t.realloc))
                .filter(|&&(to, _)| to == s)
                .map(|&(_, n)| n as u32)
                .sum();
            f[F_TERM_POINTS] = points as f32 / 5.0;
            f[F_TERM_CHOSEN] = flag(t.chosen == Some(s) || t.seats.contains(&s));
        }
        ix.seats[s.index()] = push(&mut o, [T_SEAT, 0, 0, 0], f);
    }

    // Minions in play (contested ones included: face down, identity public).
    for (i, m) in st.minions.iter().enumerate() {
        if !m.in_play {
            continue;
        }
        let id = MinionId(i as u32);
        let mut f = [0.0; TOK_F];
        f[F_REL + rel(st, viewer, m.owner).min(SEATS - 1)] = 1.0;
        f[F_BLOOD] = m.blood as f32 / 10.0;
        f[F_CAPACITY] = st.capacity_of(id).unwrap_or(0) as f32 / 11.0;
        f[F_LOCKED] = flag(m.locked);
        f[F_TORPOR] = flag(m.in_torpor);
        f[F_CONTESTED] = flag(m.contested || m.title_contested);
        f[F_TITLED] = flag(m.is_vampire() && st.title_of(id) != Title::None);
        f[F_ALLY] = flag(!m.is_vampire());
        f[F_TERM_CHOSEN] = flag(terms.is_some_and(|t| t.minions.contains(&id)));
        f[F_BLED] = flag(m.bled_this_turn);
        f[F_POLITICAL] = flag(m.political_this_turn);
        f[F_RECRUITED] = flag(m.recruited);
        f[F_EXPELLED] = flag(m.expelled);
        f[F_CORRUPTION] = m.corruption.iter().map(|&(_, n)| n as f32).sum::<f32>() / 5.0;
        f[F_MARKERS] = (m.hostage + m.nightmare) as f32 / 5.0;
        ix.minions[i] = push(&mut o, [T_MINION, minion_id(m.kind), 0, 0], f);
    }

    // The viewer's hand, as `Choice::Play` indexes it (with Fleshforge
    // Chamber's cards).
    for (k, (card, _)) in e.virtual_hand(viewer).into_iter().enumerate() {
        let mut f = [0.0; TOK_F];
        f[F_REL] = 1.0;
        f[F_POSITION] = k as f32 / 10.0;
        let t = push(&mut o, [T_HAND, library_id(card), 0, 0], f);
        ix.hand.push(t);
    }
    if omniscient {
        for s in 0..st.seats.len() {
            let seat = SeatId(s as u8);
            if seat == viewer {
                continue;
            }
            for (k, &card) in st.seats[s].hand.iter().enumerate() {
                let mut f = [0.0; TOK_F];
                f[F_REL + rel(st, viewer, seat).min(SEATS - 1)] = 1.0;
                f[F_POSITION] = k as f32 / 10.0;
                push(&mut o, [T_HAND, library_id(card), 0, 0], f);
            }
        }
    }

    // Cards in play, then their hosts as pointers (a host is a minion,
    // indexed above).
    for (i, c) in st.cards.iter().enumerate() {
        if !c.in_play {
            continue;
        }
        let id = CardId(i as u32);
        let mut f = [0.0; TOK_F];
        f[F_REL + rel(st, viewer, st.card_controller(id)).min(SEATS - 1)] = 1.0;
        f[F_BLOOD] = c.counters as f32 / 10.0;
        f[F_LOCKED] = flag(c.locked);
        f[F_CONTESTED] = flag(c.contested);
        f[F_MODE] = c.mode as f32;
        f[F_MARKERS] = flag(c.effect_only);
        if let Some(t) = terms {
            f[F_TERM_CHOSEN] = flag(t.cards.contains(&id));
            let points: u32 = (t.card_alloc.iter())
                .filter(|&&(to, _)| to == id)
                .map(|&(_, n)| n as u32)
                .sum();
            f[F_TERM_POINTS] = points as f32 / 5.0;
        }
        let host = c.bearer().map_or(0, |m| ix.minion(m));
        ix.cards[i] = push(&mut o, [T_CARD, library_id(c.card), host, 0], f);
    }

    // Uncontrolled regions: the viewer's vampires face up to them, the
    // others' face down but a banished one.
    for s in 0..st.seats.len() {
        let seat = SeatId(s as u8);
        for (k, slot) in st.seats[s].uncontrolled.iter().enumerate() {
            let seen = omniscient || seat == viewer || slot.banished.is_some();
            if v == Version::V3 && !seen {
                ix.seat_slots[s].push(0);
                continue;
            }
            let mut f = [0.0; TOK_F];
            f[F_REL + rel(st, viewer, seat).min(SEATS - 1)] = 1.0;
            f[F_BLOOD] = slot.counters as f32 / 10.0;
            f[F_POSITION] = k as f32 / 10.0;
            if seen && slot.card != HIDDEN {
                f[F_CAPACITY] = st.slot_vampire(seat, k).0 as f32 / 11.0;
            }
            // A banished vampire is its arena entry (a created one has no
            // crypt card: `CREATED`).
            let card = match &slot.banished {
                Some(b) => minion_id(st.minions[b.minion.index()].kind),
                None if seen => crypt_id(slot.card),
                None => V_HIDDEN,
            };
            let t = push(&mut o, [T_SLOT, card, 0, 0], f);
            if seat == viewer {
                ix.slots.push(t);
            }
            ix.seat_slots[s].push(t);
        }
    }

    // What remains of the viewer's own library and crypt: a token per
    // card, with its copies (the order is never read).
    let seat = &st.seats[viewer.index()];
    for (cards, crypt) in [(&seat.library, false), (&seat.crypt, true)] {
        let mut left = std::collections::BTreeMap::new();
        for &c in cards {
            *left.entry(c).or_insert(0u32) += 1;
        }
        for (c, n) in left {
            let mut f = [0.0; TOK_F];
            f[F_REL] = 1.0;
            f[F_BLOOD] = n as f32 / 10.0;
            let id = if crypt {
                f[F_CAPACITY] = CRYPT[c as usize].capacity as f32 / 11.0;
                crypt_id(c)
            } else {
                library_id(c)
            };
            if v == Version::V3 {
                o.deck_ids.push([T_DECK, id, 0, 0]);
                o.deck_f.push(f);
            } else {
                push(&mut o, [T_DECK, id, 0, 0], f);
            }
        }
    }

    // The political referendum: its card and the votes so far (the terms
    // are on the tokens they name, above).
    if let Some(r) = political {
        {
            let card = e.action().map_or(0, |a| match (a.card, a.source) {
                (Some(play), _) => library_id(play.card),
                (None, Some(crate::cards::AbilitySource::Card(c))) => card_v(c),
                (None, Some(crate::cards::AbilitySource::Vampire(m))) => {
                    minion_id(st.minions[m.index()].kind)
                }
                (None, None) => 0,
            });
            let (yes, no) = r.tally();
            let mut f = [0.0; TOK_F];
            f[F_VOTES_FOR] = yes as f32 / 5.0;
            f[F_VOTES_AGAINST] = no as f32 / 5.0;
            push(&mut o, [T_REFERENDUM, card, 0, 0], f);
            if let Some(clan) = r.terms.clan {
                let k = clans.iter().position(|&c| c == clan).expect("a clan");
                push(
                    &mut o,
                    [T_REFERENDUM, V_CLAN + k as i32, 0, 0],
                    [0.0; TOK_F],
                );
            }
        }
    }

    if v != Version::V1 {
        // The ash heaps: a token per seat and card, with its copies.
        for s in 0..st.seats.len() {
            let seat = &st.seats[s];
            let rel = rel(st, viewer, SeatId(s as u8)).min(SEATS - 1);
            for (cards, crypt) in [(&seat.ash, false), (&seat.ash_crypt, true)] {
                let mut heap = std::collections::BTreeMap::new();
                for &c in cards {
                    *heap.entry(c).or_insert(0u32) += 1;
                }
                for (c, n) in heap {
                    let mut f = [0.0; TOK_F];
                    f[F_REL + rel] = 1.0;
                    f[F_BLOOD] = n as f32 / 4.0;
                    let id = if crypt { crypt_id(c) } else { library_id(c) };
                    push(&mut o, [T_ASH, id, 0, 0], f);
                }
            }
        }
        // The last public choices, oldest first: each as the option it
        // was, pointing at the tokens it named that are still here.
        // Two pointers: the actor (else the card or minion the choice
        // names) and the target.
        let n = e.history.len();
        let mut evix = ix.clone();
        evix.hand.clear();
        for (k, ev) in e.history.iter().enumerate() {
            evix.slots.clone_from(&ix.seat_slots[ev.seat.index()]);
            let (ids, of) = option(e, ev.seat, &evix, &clans, &ev.choice, ev.card);
            let mut f = [0.0; TOK_F];
            f[F_REL + rel(st, viewer, ev.seat).min(SEATS - 1)] = 1.0;
            f[E_AGE] = (n - k) as f32 / crate::game::HISTORY as f32;
            f[E_TURNS] = st.turn.saturating_sub(ev.turn) as f32 / 10.0;
            f[E_OPT..E_OPT + OPT_F].copy_from_slice(&of);
            let a = if ids[3] != 0 { ids[3] } else { ids[2] };
            push(&mut o, [T_EVENT + ids[0], ids[1], a, ids[4]], f);
        }
    }

    // The context: window, action, combat, referendum.
    let mut f = [0.0; TOK_F];
    let (mut a_ptr, mut b_ptr) = (0, 0);
    f[F_TURN] = st.turn as f32 / 100.0;
    if let (Version::V3, crate::game::Flow::Influence { left }) = (v, &e.flow) {
        f[F_MPA] = *left as f32 / 4.0;
    }
    if let Some(a) = e.action() {
        f[F_ACTION + a.kind as usize] = 1.0;
        if let Some(t) = a.bleed_target {
            f[F_REL + rel(st, viewer, t).min(SEATS - 1)] = 1.0;
        }
        f[F_STEALTH] = a.stealth(st) as f32 / 5.0;
        f[F_BLEED_AMOUNT] = a.bleed_amount(st) as f32 / 5.0;
        a_ptr = ix.minion(a.acting);
    }
    if let Some(c) = e.combat() {
        f[F_LONG] = flag(c.range == crate::combat::Range::Long);
        f[F_ROUND] = c.round as f32 / 5.0;
        a_ptr = ix.minion(c.sides[0].minion);
        b_ptr = ix.minion(c.sides[1].minion);
    }
    if let Some(r) = e.referendum() {
        f[F_CALLER + rel(st, viewer, r.caller).min(SEATS - 1)] = 1.0;
        match r.kind {
            crate::referendum::RefKind::Political => f[F_POLITICAL_REF] = 1.0,
            crate::referendum::RefKind::BloodHunt { diablerist } => {
                f[F_BLOOD_HUNT] = 1.0;
                b_ptr = ix.minion(diablerist);
            }
        }
        if let Some(m) = r.calling {
            b_ptr = ix.minion(m);
        }
    }
    let window = V_WINDOW + req.window as i32;
    push(&mut o, [T_CONTEXT, window, a_ptr, b_ptr], f);

    for choice in &req.options {
        let (ids, f) = option(e, viewer, &ix, &clans, choice, None);
        o.opt_ids.push(ids);
        o.opt_f.push(f);
    }
    o
}

/// Choice variant ids (`opt_ids[0]`) are 1-based: `N_CHOICE_KINDS` counts the
/// padding 0.
pub const N_CHOICE_KINDS: usize = 59;

/// `choice` as `viewer` would be offered it; `played`: the card its hand
/// index named, for a past choice (`Event::card`).
fn option(
    e: &Engine,
    viewer: SeatId,
    ix: &Index,
    clans: &[&str],
    choice: &Choice,
    played: Option<u16>,
) -> ([i32; OPT_IDS], [f32; OPT_F]) {
    let mut f = [0.0; OPT_F];
    let hand = |i: u8| at(&ix.hand, i as usize);
    let slot = |i: u8| at(&ix.slots, i as usize);
    let hand_card = |i: u8| library_id(played.unwrap_or_else(|| e.playable_card(viewer, i)));
    // A referenced card or minion also carries its identity: it may have
    // left play (a rider pending as its card leaves), naming no token.
    let card_v = |c: u32| library_id(e.state.cards[c as usize].card);
    let minion_v = |m: MinionId| minion_id(e.state.minions[m.index()].kind);
    let mut n = |x: u8| f[O_N] = x as f32 / 10.0;
    // [kind, vocabulary, pointer a, pointer b, pointer c]
    let ids: [i32; OPT_IDS] = match *choice {
        Choice::Pass => [1, 0, 0, 0, 0],
        Choice::BurnEdge => [2, 0, 0, 0, 0],
        Choice::PayContest(c) | Choice::YieldContest(c) => {
            let k = if matches!(choice, Choice::PayContest(_)) {
                3
            } else {
                4
            };
            let (v, p, title) = match c {
                Contest::Card(m) => (minion_v(m), ix.minion(m), false),
                Contest::Title(m) => (minion_v(m), ix.minion(m), true),
                Contest::Library(id) => (card_v(id.0), ix.card(id), false),
            };
            f[O_TITLE] = flag(title);
            [k, v, p, 0, 0]
        }
        Choice::Play {
            card,
            mode,
            actor,
            target,
        } => {
            f[O_MODE] = mode as f32;
            let actor = actor.map_or(0, |m| ix.minion(m));
            [5, hand_card(card), hand(card), actor, ix.target(target)]
        }
        Choice::UseAbility {
            source,
            ability,
            actor,
            target,
        } => {
            f[O_MODE] = ability as f32;
            let vocab = match source {
                AbilitySource::Card(c) => library_id(e.state.cards[c.index()].card),
                AbilitySource::Vampire(m) => minion_id(e.state.minions[m.index()].kind),
            };
            let actor = actor.map_or(0, |m| ix.minion(m));
            [6, vocab, ix.source(source), actor, ix.target(target)]
        }
        Choice::ResolveRider { card } => [7, card_v(card), ix.card(CardId(card)), 0, 0],
        Choice::ResolvePrinted { minion } => [8, minion_v(minion), ix.minion(minion), 0, 0],
        Choice::BurnInPlay { card } => [9, card_v(card), ix.card(CardId(card)), 0, 0],
        Choice::BurnPool => [10, 0, 0, 0, 0],
        Choice::BurnBloodFrom { minion } => [11, minion_v(minion), ix.minion(minion), 0, 0],
        Choice::SetX(x) => {
            n(x);
            [12, 0, 0, 0, 0]
        }
        Choice::Discard(i) => [13, hand_card(i), hand(i), 0, 0],
        Choice::FindCard(c) => [14, library_id(c), 0, 0, 0],
        Choice::TakeFrom { zone, card } => {
            f[O_ZONE] = match zone {
                Zone::Library => 0.0,
                Zone::Hand => 0.5,
                Zone::AshHeap => 1.0,
            };
            [15, library_id(card), 0, 0, 0]
        }
        Choice::Slot(i) => [16, 0, slot(i), 0, 0],
        Choice::AshVampire(c) => [17, crypt_id(c), 0, 0, 0],
        Choice::DrawLibrary => [18, 0, 0, 0, 0],
        Choice::DrawStored { card, stored } => {
            [19, library_id(stored), ix.card(CardId(card)), 0, 0]
        }
        Choice::Bottom(i) => [20, hand_card(i), hand(i), 0, 0],
        Choice::TakeEquipment { card } => [21, card_v(card), ix.card(CardId(card)), 0, 0],
        Choice::TakeAction {
            minion,
            action,
            target,
        } => {
            f[O_ACTION + action as usize] = 1.0;
            let t = target.map_or(0, |m| ix.minion(m));
            [22, minion_v(minion), ix.minion(minion), 0, t]
        }
        Choice::DeclareBlock { minion } => [23, minion_v(minion), ix.minion(minion), 0, 0],
        Choice::PayRescue { from_acting } => {
            n(from_acting);
            [24, 0, 0, 0, 0]
        }
        Choice::CastVotes { minion, favor } => {
            f[O_FAVOR] = flag(favor);
            [25, minion_v(minion), ix.minion(minion), 0, 0]
        }
        Choice::CastEdge { favor } => {
            f[O_FAVOR] = flag(favor);
            [26, 0, 0, 0, 0]
        }
        Choice::CastCardVote { favor } => {
            f[O_FAVOR] = flag(favor);
            [27, 0, 0, 0, 0]
        }
        Choice::BurnForVote { card, favor } => {
            f[O_FAVOR] = flag(favor);
            [28, hand_card(card), hand(card), 0, 0]
        }
        Choice::AllocateTo { seat, n: k } => {
            n(k);
            [29, 0, ix.seat(seat), 0, 0]
        }
        Choice::ChooseSeat(s) => [30, 0, ix.seat(s), 0, 0],
        Choice::ChooseMinion(m) => [31, minion_v(m), ix.minion(m), 0, 0],
        Choice::ChooseClan(c) => {
            let k = clans.iter().position(|&x| x == c).unwrap_or(0) as i32;
            [32, V_CLAN + k, 0, 0, 0]
        }
        Choice::ChooseCard(c) => [33, card_v(c), ix.card(CardId(c)), 0, 0],
        Choice::AllocateToCard { card, n: k } => {
            n(k);
            [34, card_v(card), ix.card(CardId(card)), 0, 0]
        }
        Choice::Transfer { slot: i } => [35, 0, slot(i), 0, 0],
        Choice::Withdraw { slot: i } => [36, 0, slot(i), 0, 0],
        Choice::DrawCrypt => [37, 0, 0, 0, 0],
        Choice::InfluenceOut { slot: i } => [38, 0, slot(i), 0, 0],
        Choice::HandStrike => [39, 0, 0, 0, 0],
        Choice::UseManeuver => [40, 0, 0, 0, 0],
        Choice::UsePress => [41, 0, 0, 0, 0],
        Choice::UsePrevent => [42, 0, 0, 0, 0],
        Choice::PayBlockToll => [43, 0, 0, 0, 0],
        Choice::PayToCancel => [44, 0, 0, 0, 0],
        Choice::StrikeCombatEnds => [45, 0, 0, 0, 0],
        Choice::RoundStrike { at } => match at {
            Some(c) => [46, card_v(c.0), ix.card(c), 0, 0],
            None => [46, 0, 0, 0, 0],
        },
        Choice::TakeDamageAfterResolution => [47, 0, 0, 0, 0],
        Choice::CommittedStrike => [48, 0, 0, 0, 0],
        Choice::BurnWeaponStrike { weapon } => [49, card_v(weapon), ix.card(CardId(weapon)), 0, 0],
        Choice::EmployFromHand { card, mode } => {
            f[O_MODE] = mode as f32;
            [50, library_id(card), 0, 0, 0]
        }
        Choice::BurnBloodForIntercept { minion } => [51, minion_v(minion), ix.minion(minion), 0, 0],
        Choice::UnlockForBlood { minion } => [52, minion_v(minion), ix.minion(minion), 0, 0],
        Choice::ContinueAction => [53, 0, 0, 0, 0],
        Choice::GainStrength => [54, 0, 0, 0, 0],
        Choice::BurnBloodToAvoid => [55, 0, 0, 0, 0],
        Choice::BurnCorruptionToFail { minion } => [56, minion_v(minion), ix.minion(minion), 0, 0],
        Choice::BurnCorruptionToUnlock { minion } => {
            [57, minion_v(minion), ix.minion(minion), 0, 0]
        }
        Choice::Diablerise => [58, 0, 0, 0, 0],
    };
    (ids, f)
}

/// The encoding's shape, for the Python side (never hardcode a width):
/// id and feature widths, vocabulary and kind counts, token types.
/// A stable key per vocabulary id (the id is its index): `crypt:<KRCG
/// id>`, `library:<KRCG id>`, `window:<label index>`, `clan:<name>`. A
/// trained network's embedding rows follow their key when the pool grows
/// and the ids shift (`train/model.py`, `remap_vocab`).
pub fn vocab_keys() -> Vec<String> {
    let mut keys = vec!["none".to_string()];
    keys.extend(CRYPT.iter().map(|c| format!("crypt:{}", c.id)));
    keys.extend(LIBRARY.iter().map(|c| format!("library:{}", c.id)));
    keys.push("hidden".into());
    keys.extend((0..N_WINDOWS).map(|w| format!("window:{w}")));
    keys.extend(
        crate::referendum::clans()
            .into_iter()
            .map(|c| format!("clan:{c}")),
    );
    debug_assert_eq!(
        keys.len(),
        V_CLAN as usize + crate::referendum::clans().len()
    );
    keys
}

pub fn schema_json() -> String {
    let vocab = V_CLAN as usize + crate::referendum::clans().len();
    format!(
        "{{\"tok_ids\": {TOK_IDS}, \"tok_f\": {TOK_F}, \"opt_ids\": {OPT_IDS}, \
         \"opt_f\": {OPT_F}, \"vocab\": {vocab}, \"choice_kinds\": {N_CHOICE_KINDS}, \
         \"token_types\": {}, \"windows\": {N_WINDOWS}, \"action_kinds\": {N_ACTION_KINDS}}}",
        T_EVENT + N_CHOICE_KINDS as i32
    )
}
