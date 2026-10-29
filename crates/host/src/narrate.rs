//! What happened between decisions: the engine runs every window nobody
//! can act in at once, so an action can resolve, a combat end or a
//! referendum close with no decision to tell. The engine notes it in
//! order (`zvtes::notes`), the table's ledger between its notes with
//! the cause of each change; the session tells the notes and the
//! difference of each two ledgers in a row as facts in the engine's ids
//! (the host names them for each viewer: `events.rs`): how an action or
//! a referendum ended, the phase and transfers, pool and blood moved,
//! minions and cards in and out of play, torpor, the Edge, locks,
//! counters, control, capacity and titles. What only a decision shows
//! (a block attempt given up, a strike declared) is read from its marks.
//! Every fact told is public. Computed state (the action's, the
//! combat's, the referendum's, markers, traits) is told by `Game::view`
//! from two views in a row.
//!
//! Every change is a fact; `told` marks the ones a log tells in words
//! (a decision's own effects are its option's to say, unless the option
//! does not: a rider, an ability).
//!
//! The marks read the engine alone (and its census for copies), not a
//! view.

use std::collections::{HashMap, HashSet};

use zvtes::actions::{ActionCtx, Outcome};
use zvtes::cards::{AbilitySource, StrikeKind};
use zvtes::combat::CombatNote;
use zvtes::copies::{minion_copy, CopyId, Place};
use zvtes::decision::ActionKind;
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::notes::{Cause, Ledger, Rule, Source};
use zvtes::referendum::RefKind;
use zvtes::state::CardId;

/// What happened, in the engine's ids.
#[derive(Clone, Debug, PartialEq)]
pub enum Fact {
    /// A decision taken here (the host tells it from its answer).
    Decided(u64),
    Turn {
        turn: u32,
        active: SeatId,
    },
    Phase(zvtes::game::Phase),
    Transfers(u8),
    Draw {
        seat: SeatId,
        n: u8,
        crypt: bool,
    },
    Pool {
        seat: SeatId,
        delta: i32,
        pool: i32,
        cause: Option<Cause>,
    },
    Ousted {
        seat: SeatId,
    },
    WithdrawalAnnounced {
        seat: SeatId,
    },
    WithdrawalBroken {
        seat: SeatId,
    },
    Withdrew {
        seat: SeatId,
    },
    /// The Edge's holder now (`None`: burned, no one holds it).
    Edge {
        seat: Option<SeatId>,
    },
    MinionEnters {
        minion: MinionId,
    },
    /// Burned (to `to`), or contested.
    MinionLeaves {
        minion: MinionId,
        contested: bool,
        to: Option<Place>,
    },
    Torpor {
        minion: MinionId,
        enters: bool,
    },
    Incapacitated {
        minion: MinionId,
        enters: bool,
    },
    Locked {
        what: Held,
        locked: bool,
    },
    Counters {
        what: Held,
        delta: i32,
        counters: u8,
        cause: Option<Cause>,
    },
    /// A minion or a card in play now controlled by `seat`.
    Controller {
        what: Held,
        seat: SeatId,
    },
    Capacity {
        minion: MinionId,
        capacity: Option<u8>,
    },
    /// `seat`'s hand seen by other Methuselahs (`Ledger::hands_seen`),
    /// before and now, and its cards as the ledger found them (a later
    /// move of the same step told after).
    HandSeen {
        seat: SeatId,
        seen: [u8; 2],
        cards: Vec<(CopyId, u16)>,
    },
    Title {
        minion: MinionId,
        title: zvtes::crypt::Title,
    },
    Blood {
        minion: MinionId,
        delta: i32,
        blood: u8,
        cause: Option<Cause>,
    },
    CardEnters {
        card: CardId,
        on: Option<MinionId>,
    },
    /// A card from a place to another (the census of two ledgers).
    Zoned {
        copy: CopyId,
        card: u16,
        crypt: bool,
        from: Place,
        to: Place,
        /// Each end as its ledger had it: who held it (a card's
        /// controller, a minion's), in torpor, on which minion.
        at: [Spot; 2],
        /// Its place in the uncontrolled region it leaves and in the one
        /// it goes to, the moves told before it applied.
        slots: [Option<u8>; 2],
    },
    /// To `to`, or contested: in its own contest, or out of play with
    /// its contested minion (rules-qa 83.1).
    CardLeaves {
        card: CardId,
        to: Option<Place>,
        contested: bool,
    },
    /// A card in play to another minion (or off one).
    Moves {
        card: CardId,
        on: Option<MinionId>,
    },
    BlockFails {
        minion: MinionId,
    },
    /// A strike declared (`None`: no strike), from a card or weapon, and
    /// what it was told to inflict as declared.
    Strike {
        minion: MinionId,
        strike: Option<StrikeKind>,
        source: Option<CardId>,
        told: Option<zvtes::combat::StrikeHit>,
    },
    /// An action announced (`Note::Action`).
    ActionStarts(ActionFact),
    /// Another minion now acting in the action under way.
    ActingChanges {
        seat: SeatId,
        minion: MinionId,
    },
    ActionEnds {
        action: ActionFact,
        /// `None`: ended otherwise (its acting minion gone).
        outcome: Option<Outcome>,
        /// Its Methuselah was ousted.
        ousted: bool,
        blocker: Option<MinionId>,
    },
    ReferendumEnds {
        passed: bool,
        canceled: bool,
        votes: (u32, u32),
        action: Option<ActionFact>,
        /// A blood hunt's diablerist.
        hunted: Option<MinionId>,
    },
    Combat(CombatNote),
    /// A crypt's or library's card revealed to all (`Note::Reveal`).
    Revealed {
        seat: SeatId,
        card: u16,
        crypt: bool,
    },
    /// A play resolved or canceled (`Note::Play`).
    Played(zvtes::game::PlayNote),
    /// A hand looked at, for its viewer only (`Note::Look`).
    Looked(zvtes::game::Look),
}

/// What a lock or counters are on: a minion, a card in play, an
/// uncontrolled vampire (by its copy).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Held {
    Minion(MinionId),
    Card(CardId),
    /// An uncontrolled vampire, by its place in its seat's region as the
    /// events have kept it when told (the ledger's then).
    Slot {
        seat: u8,
        slot: u8,
    },
}

/// An action as its end tells it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionFact {
    pub seat: SeatId,
    pub acting: MinionId,
    pub kind: ActionKind,
    /// The action card and its copy.
    pub card: Option<(u16, CopyId)>,
    /// What grants it: a card in play, another vampire.
    pub source: Option<AbilitySource>,
    /// The Methuselah bled, a bleed's.
    pub bleed_target: Option<SeatId>,
    /// Its target, else its card's.
    pub target: zvtes::cards::Target,
}

impl ActionFact {
    pub(crate) fn of(a: &ActionCtx) -> ActionFact {
        ActionFact {
            seat: a.seat,
            acting: a.acting,
            kind: a.kind,
            card: a.card.as_ref().map(|p| (p.card, *p.copy)),
            source: a.source.filter(|&s| s != AbilitySource::Vampire(a.acting)),
            bleed_target: a.bleed_target,
            target: match (a.target, &a.card) {
                (zvtes::cards::Target::None, Some(p)) => p.target,
                (t, _) => t,
            },
        }
    }
}

/// A fact, whether a log tells it in words, and the minions as it found
/// them.
#[derive(Clone, Debug, PartialEq)]
pub struct Told {
    pub fact: Fact,
    pub told: bool,
    pub faces: Faces,
}

impl Told {
    /// Its minions' faces set by `Session::observe`, for the batch.
    pub(crate) fn new(fact: Fact, told: bool) -> Told {
        Told {
            fact,
            told,
            faces: Faces::default(),
        }
    }
}

/// Each minion's copy and face (card, crypt) by `MinionId`, when a fact
/// was told: facts are named at the next view, and a merge before it
/// changes both (an event naming the merged copy would name a minion the
/// viewer never saw).
pub type Faces = std::sync::Arc<[(CopyId, u16, bool)]>;

pub(crate) fn faces(e: &Engine) -> Faces {
    (0..e.state.minions.len())
        .map(|i| {
            let (card, crypt) = crate::view::minion_face(&e.state.minions[i]);
            (minion_copy(&e.state, i), card, crypt)
        })
        .collect()
}

fn tell(out: &mut Vec<Told>, fact: Fact, told: bool) {
    out.push(Told::new(fact, told));
}

/// The table at a decision, as far as events tell it: the engine's
/// ledger, the action under way, and what only a decision shows (the
/// strikes declared).
#[derive(Clone, Debug)]
pub(crate) struct Mark {
    pub(crate) ledger: Ledger,
    pub(crate) action: Option<ActionFact>,
    /// The combat's strikes declared: per side, its minion and the round
    /// and pair it declared in, and what (`None`: not declared yet).
    strikes: Vec<(MinionId, u8, u8, Option<Declared>)>,
}

/// A strike declared: what, from which card, told as what damage.
#[derive(Clone, Copy, Debug)]
struct Declared {
    strike: Option<StrikeKind>,
    source: Option<CardId>,
    told: Option<zvtes::combat::StrikeHit>,
}

impl Mark {
    pub(crate) fn of(e: &Engine) -> Mark {
        Mark {
            ledger: e.table(),
            action: e.action().map(ActionFact::of),
            strikes: e
                .combat()
                .map(|c| {
                    c.sides
                        .iter()
                        .enumerate()
                        .map(|(i, s)| {
                            let declared = s.chosen.then(|| Declared {
                                strike: s.strike,
                                source: s.strike_source,
                                told: s
                                    .strike
                                    .filter(|_| s.strike_at.is_none())
                                    .and_then(|k| c.strike_told(&e.state, e.action(), i, k)),
                            });
                            (s.minion, c.round, c.pair, declared)
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    /// The strikes declared since (the acting minion's first: the
    /// blocker chooses knowing it), each told once.
    pub(crate) fn strikes(&self, now: &Mark, out: &mut Vec<Told>) {
        for &(m, round, pair, declared) in &now.strikes {
            let Some(d) = declared else { continue };
            let told = self
                .strikes
                .iter()
                .any(|&(b, r, p, s)| b == m && r == round && p == pair && s.is_some());
            if !told {
                let fact = Fact::Strike {
                    minion: m,
                    strike: d.strike,
                    source: d.source,
                    told: d.told,
                };
                tell(out, fact, true);
            }
        }
    }
}

/// A place's holder as a ledger had it: its seat (a card's controller,
/// its minion's on one; a minion's), a minion's region, the minion a
/// card is on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub seat: SeatId,
    pub region: crate::view::Region,
    pub on: Option<MinionId>,
    /// The seats that see a card there, a bit each (`GameState::legal_view`'s
    /// mask): none in a library or crypt; a hand's own and open viewers,
    /// as the ledger had them; an uncontrolled vampire's own seat, every
    /// seat's banished; a card stored face down, its card's controller
    /// (these two read from the table as it stands: a slot gone, a card
    /// changed hands since, is read so).
    pub shown: u8,
}

/// Every seat.
pub const ALL: u8 = u8::MAX;

impl Spot {
    /// Whether `viewer` sees a card there (every seat to a viewer of the
    /// whole table).
    pub fn seen_by(&self, viewer: Option<SeatId>) -> bool {
        viewer.is_none_or(|v| self.shown & 1 << v.0 != 0)
    }
}

fn spot(e: &Engine, l: &Ledger, p: Place, copy: CopyId) -> Spot {
    let st = &e.state;
    let minion = |m: MinionId| match l.minions.get(m.index()) {
        Some(m) => (
            m.controller,
            crate::view::Region::from_flags(m.torpor, m.incapacitated),
        ),
        None => (st.minions[m.index()].owner, crate::view::Region::Ready),
    };
    let stored = |c: u32| {
        (0..st.seats.len())
            .filter(|&v| st.stored_seen_by(CardId(c), SeatId(v as u8)))
            .fold(0u8, |m, v| m | 1 << v)
    };
    let shown = match p {
        Place::Zone(_, "library" | "crypt") => 0,
        Place::Zone(s, "hand") => l.hands_seen.get(s as usize).copied().unwrap_or(1 << s),
        Place::Uncontrolled(s) => {
            let slots = &st.seats[s as usize].uncontrolled;
            match slots
                .iter()
                .any(|u| *u.copy == copy && u.banished.is_some())
            {
                true => ALL,
                false => 1 << s,
            }
        }
        Place::Card { id, stored: true }
        | Place::Banished {
            stored: Some(id), ..
        } => stored(id),
        _ => ALL,
    };
    let at = |(seat, region): (SeatId, crate::view::Region), on| Spot {
        seat,
        region,
        on,
        shown,
    };
    match p {
        Place::Zone(s, _)
        | Place::Uncontrolled(s)
        | Place::InFlight(s)
        | Place::Training(s)
        | Place::Banished { seat: s, .. } => at((SeatId(s), crate::view::Region::Ready), None),
        Place::Minion { id, .. } => at(minion(MinionId(id)), None),
        Place::Card { id, .. } => {
            let c = CardId(id);
            match (
                l.cards.get(c.index()).copied().flatten(),
                e.state.cards[c.index()].body,
            ) {
                // A card in the ledger was no minion yet (one become a
                // minion since: Call the Great Beast).
                (Some(card), _) => match card.host {
                    Some(m) => at((minion(m).0, crate::view::Region::Ready), Some(m)),
                    None => at((card.controller, crate::view::Region::Ready), None),
                },
                (None, Some(m)) => at(minion(m), None),
                (None, None) => at(
                    (e.state.card_controller(c), crate::view::Region::Ready),
                    e.state.cards[c.index()].host,
                ),
            }
        }
    }
}

/// A census, one place a copy: mid-run, a card already in its zone is
/// still the action's in flight (its place, the zone).
fn one_place(census: &[(CopyId, u16, Place)]) -> Vec<(CopyId, u16, Place)> {
    let mut out: Vec<(CopyId, u16, Place)> = Vec::with_capacity(census.len());
    let mut at: HashMap<CopyId, usize> = HashMap::new();
    for &(c, card, p) in census {
        match at.get(&c) {
            Some(&i) if matches!(out[i].2, Place::InFlight(_)) => out[i] = (c, card, p),
            Some(_) => {}
            None => {
                at.insert(c, out.len());
                out.push((c, card, p));
            }
        }
    }
    out
}

/// Per seat, the copies in its library, crypt, hand and uncontrolled
/// region: one from the library in the hand next is a draw.
fn piles(l: &Ledger) -> [Vec<HashSet<CopyId>>; 4] {
    let n = l.pools.len();
    let mut piles = std::array::from_fn(|_| vec![HashSet::new(); n]);
    for &(copy, _, place) in &l.census {
        let (s, pile) = match place {
            Place::Zone(s, "library") => (s, 0),
            Place::Zone(s, "crypt") => (s, 1),
            Place::Zone(s, "hand") => (s, 2),
            Place::Uncontrolled(s) => (s, 3),
            _ => continue,
        };
        piles[pile][s as usize].insert(copy);
    }
    piles
}

/// What changed on the table from `before` to `after`, made by `cause`;
/// `told`: in words too (across the engine's run, or a decision whose
/// option does not tell its effects); `open`: the action under way, a
/// political referendum's; `seen`: each copy's last place, kept up to
/// date (a card resolving is in no place: it moves from where it was).
pub(crate) fn changes(
    e: &Engine,
    (before, after): (&Ledger, &Ledger),
    cause: Option<Cause>,
    told: bool,
    open: Option<ActionFact>,
    seen: &mut HashMap<CopyId, Place>,
    out: &mut Vec<Told>,
) {
    if after.turn.0 != before.turn.0 {
        let fact = Fact::Turn {
            turn: after.turn.0,
            active: after.turn.1,
        };
        tell(out, fact, true);
    }
    // The unlock phase is its turn's (`turn` tells it).
    if after.phase != before.phase && after.phase != zvtes::game::Phase::Unlock {
        tell(out, Fact::Phase(after.phase), false);
    }
    if let Some(left) = after
        .transfers
        .filter(|_| after.transfers != before.transfers)
    {
        tell(out, Fact::Transfers(left), false);
    }
    // A block attempt that ended with no block (short of the stealth,
    // or given up): the action goes on, or ends unblocked.
    if let Some(m) = before.attempting {
        if after.attempting != Some(m) && after.blocker != Some(m) {
            tell(out, Fact::BlockFails { minion: m }, true);
        }
    }
    if let (Some(r), None) = (&before.referendum, &after.referendum) {
        let (f, a) = r.votes;
        let political = r.kind == RefKind::Political;
        let fact = Fact::ReferendumEnds {
            // A political action's result, when the action outlives it.
            passed: after
                .passed
                .filter(|_| political)
                .unwrap_or(f > a && !r.forced_fail),
            canceled: r.canceled,
            votes: r.votes,
            action: open.filter(|_| political),
            hunted: match r.kind {
                RefKind::BloodHunt { diablerist } => Some(diablerist),
                RefKind::Political => None,
            },
        };
        tell(out, fact, true);
    }
    let (from, to) = (piles(before), piles(after));
    let drawn =
        |from: &[HashSet<CopyId>], to: &[HashSet<CopyId>], s: usize| match (from.get(s), to.get(s))
        {
            (Some(b), Some(a)) => a.iter().filter(|c| b.contains(c)).count(),
            _ => 0,
        };
    for s in 0..after.pools.len() {
        for (crypt, n) in [
            (false, drawn(&from[0], &to[2], s)),
            (true, drawn(&from[1], &to[3], s)),
        ] {
            if n == 0 {
                continue;
            }
            let fact = Fact::Draw {
                seat: SeatId(s as u8),
                n: n.min(255) as u8,
                crypt,
            };
            tell(out, fact, true);
        }
    }
    // Where a card that left play went, by its copy.
    let mut places: Option<HashMap<CopyId, Place>> = None;
    let mut place = |c: CopyId| {
        places
            .get_or_insert_with(|| after.census.iter().map(|&(c, _, p)| (c, p)).collect())
            .get(&c)
            .copied()
    };
    for (s, (&was, &now)) in before.pools.iter().zip(&after.pools).enumerate() {
        if was != now && !before.ousted[s] {
            let fact = Fact::Pool {
                seat: SeatId(s as u8),
                delta: now - was,
                pool: now,
                cause,
            };
            tell(out, fact, told);
        }
    }
    for (s, (&was, &now)) in before.withdrawal.iter().zip(&after.withdrawal).enumerate() {
        let seat = SeatId(s as u8);
        match (was, now) {
            ((_, false), (_, true)) => tell(out, Fact::Withdrew { seat }, told),
            ((false, _), (true, _)) => tell(out, Fact::WithdrawalAnnounced { seat }, told),
            ((true, _), (false, _)) => tell(out, Fact::WithdrawalBroken { seat }, told),
            _ => {}
        }
    }
    for (s, (&was, &now)) in before.ousted.iter().zip(&after.ousted).enumerate() {
        if now && !was && !after.withdrawal[s].1 {
            let fact = Fact::Ousted {
                seat: SeatId(s as u8),
            };
            tell(out, fact, told);
        }
    }
    // The Edge changing hands is told always: no option says it.
    if after.edge != before.edge {
        tell(out, Fact::Edge { seat: after.edge }, true);
    }
    // A Methuselah ousted takes their table along: told once.
    let gone = |s: SeatId| after.ousted[s.index()] && !before.ousted[s.index()];
    // The minions and cards come into play: their state told once they
    // are on the table (their moves below).
    let (mut minions_in, mut cards_in) = (Vec::new(), Vec::new());
    // Come into a contest at once (a second copy influenced out): told
    // contested once on the table.
    let mut contested_in = Vec::new();
    for (i, now) in after.minions.iter().enumerate() {
        let m = MinionId(i as u32);
        if gone(e.state.minions[i].owner) {
            continue;
        }
        let Some(b) = before.minions.get(i) else {
            if now.present {
                tell(out, Fact::MinionEnters { minion: m }, told);
                minions_in.push(m);
            } else if e.state.minions[i].contested {
                contested_in.push(m);
            }
            continue;
        };
        match (b.present, now.present) {
            (false, true) => {
                tell(out, Fact::MinionEnters { minion: m }, told);
                minions_in.push(m);
                continue;
            }
            (true, false) => {
                let fact = Fact::MinionLeaves {
                    minion: m,
                    contested: e.state.minions[i].contested,
                    to: place(minion_copy(&e.state, i)),
                };
                tell(out, fact, told);
                continue;
            }
            (false, false) => continue,
            (true, true) => {}
        }
        if now.torpor != b.torpor {
            let fact = Fact::Torpor {
                minion: m,
                enters: now.torpor,
            };
            tell(out, fact, told);
        }
        if now.incapacitated != b.incapacitated {
            let fact = Fact::Incapacitated {
                minion: m,
                enters: now.incapacitated,
            };
            tell(out, fact, told);
        }
        if now.blood != b.blood {
            let fact = Fact::Blood {
                minion: m,
                delta: i32::from(now.blood) - i32::from(b.blood),
                blood: now.blood,
                cause,
            };
            tell(out, fact, told);
        }
        if now.locked != b.locked {
            let what = Held::Minion(m);
            tell(
                out,
                Fact::Locked {
                    what,
                    locked: now.locked,
                },
                false,
            );
        }
        if now.controller != b.controller {
            let fact = Fact::Controller {
                what: Held::Minion(m),
                seat: now.controller,
            };
            tell(out, fact, false);
            // Its place another seat's: moved.
            let copy = minion_copy(&e.state, i);
            let found = after.census.iter().find(|&&(c, _, _)| c == copy);
            if let Some(&(copy, card, at)) = found.filter(|_| copy != CopyId::NONE) {
                let fact = Fact::Zoned {
                    copy,
                    card,
                    crypt: crate::view::minion_face(&e.state.minions[i]).1,
                    from: at,
                    to: at,
                    at: [spot(e, before, at, copy), spot(e, after, at, copy)],
                    slots: [None; 2],
                };
                tell(out, fact, false);
            }
        }
        if now.capacity != b.capacity {
            let fact = Fact::Capacity {
                minion: m,
                capacity: now.capacity,
            };
            tell(out, fact, false);
        }
        if now.title != b.title {
            let fact = Fact::Title {
                minion: m,
                title: now.title,
            };
            tell(out, fact, false);
        }
    }
    // An uncontrolled vampire's counters, by its copy (a transfer), told
    // at its place before the moves below.
    for &(_, copy, counters) in &after.slots {
        let was = before.slots.iter().find(|&&(_, c, _)| c == copy);
        if let Some(&(_, _, was)) = was.filter(|&&(_, _, n)| n != counters) {
            let fact = Fact::Counters {
                what: slot_of(&before.slots, copy),
                delta: i32::from(counters) - i32::from(was),
                counters,
                cause,
            };
            tell(out, fact, false);
        }
    }
    for (i, now) in after.cards.iter().enumerate() {
        if gone(e.state.cards[i].controller) {
            continue;
        }
        let c = CardId(i as u32);
        let was = before.cards.get(i).copied().flatten();
        match (was, *now) {
            (None, Some(now)) => {
                let fact = Fact::CardEnters {
                    card: c,
                    on: now.host,
                };
                tell(out, fact, told);
                cards_in.push(c);
            }
            // Become a minion (Call the Great Beast): it stays, its entry
            // told with the minions; moved from its place to the minion's
            // (the census names both by the card).
            (Some(_), None)
                if e.state.cards[i]
                    .body
                    .is_some_and(|m| e.state.minions[m.index()].present()) =>
            {
                let card = &e.state.cards[i];
                let at = Place::Card {
                    id: c.0,
                    stored: false,
                };
                let fact = Fact::Zoned {
                    copy: *card.copy,
                    card: card.card,
                    crypt: false,
                    from: at,
                    to: at,
                    at: [
                        spot(e, before, at, *card.copy),
                        spot(e, after, at, *card.copy),
                    ],
                    slots: [None; 2],
                };
                tell(out, fact, false);
            }
            (Some(_), None) => {
                let card = &e.state.cards[i];
                let fact = Fact::CardLeaves {
                    card: c,
                    to: place(*card.copy),
                    contested: card.in_play
                        && (card.contested
                            || card
                                .host
                                .is_some_and(|m| e.state.minions[m.index()].contested)),
                };
                tell(out, fact, told);
            }
            (Some(b), Some(now)) => {
                if b.host != now.host {
                    let fact = Fact::Moves {
                        card: c,
                        on: now.host,
                    };
                    tell(out, fact, told);
                }
                // Its place another (a minion, a controller): moved.
                if (b.host, b.controller) != (now.host, now.controller) {
                    let card = &e.state.cards[i];
                    let at = Place::Card {
                        id: c.0,
                        stored: false,
                    };
                    let fact = Fact::Zoned {
                        copy: *card.copy,
                        card: card.card,
                        crypt: false,
                        from: at,
                        to: at,
                        at: [
                            spot(e, before, at, *card.copy),
                            spot(e, after, at, *card.copy),
                        ],
                        slots: [None; 2],
                    };
                    tell(out, fact, false);
                }
                if b.controller != now.controller {
                    let fact = Fact::Controller {
                        what: Held::Card(c),
                        seat: now.controller,
                    };
                    tell(out, fact, false);
                }
                if b.locked != now.locked {
                    let what = Held::Card(c);
                    tell(
                        out,
                        Fact::Locked {
                            what,
                            locked: now.locked,
                        },
                        false,
                    );
                }
                if b.counters != now.counters {
                    let fact = Fact::Counters {
                        what: Held::Card(c),
                        delta: i32::from(now.counters) - i32::from(b.counters),
                        counters: now.counters,
                        cause,
                    };
                    tell(out, fact, false);
                }
            }
            _ => {}
        }
    }
    // Each card from a place to another, its seat not ousted since.
    let was: HashMap<CopyId, Place> = one_place(&before.census)
        .into_iter()
        .map(|(c, _, p)| (c, p))
        .collect();
    let seat = |p: Place| match p {
        Place::Zone(s, _) | Place::Uncontrolled(s) | Place::InFlight(s) => Some(SeatId(s)),
        _ => None,
    };
    // Each seat's uncontrolled region as the moves told so far leave it,
    // and each vampire's place in it after: a card told into the region
    // goes where the next view has it (Kaymakli Nightmares' redraw fills
    // the places the old ones left).
    let mut regions: HashMap<u8, Vec<CopyId>> = HashMap::new();
    for &(s, c, _) in &before.slots {
        regions.entry(s.0).or_default().push(c);
    }
    let mut rank: HashMap<CopyId, usize> = HashMap::new();
    for s in 0..after.ousted.len() as u8 {
        let region = after.slots.iter().filter(|&&(o, _, _)| o.0 == s);
        rank.extend(region.enumerate().map(|(i, &(_, c, _))| (c, i)));
    }
    for (copy, card, to) in one_place(&after.census) {
        let last = seen.insert(copy, to);
        let Some(from) = was
            .get(&copy)
            .copied()
            .or(last)
            .filter(|_| copy != CopyId::NONE)
        else {
            continue;
        };
        // An ousted Methuselah's places are out of the game (the view has
        // none): nothing moving among them is told, nor a card going with
        // a Methuselah ousted now (`ousted` tells it); one leaving a live
        // place for an earlier ousted owner's is (Disarm burned: gone from
        // the table, §9).
        if from == to
            || seat(from).is_some_and(|s| after.ousted[s.index()])
            || seat(to).is_some_and(gone)
        {
            continue;
        }
        // A crypt card's places: a crypt pile, the uncontrolled region,
        // a vampire (its own card's face: a token's is a library card).
        let crypt = [from, to].iter().any(|&p| match p {
            Place::Zone(_, "crypt" | "ash_crypt" | "removed_crypt") | Place::Uncontrolled(_) => {
                true
            }
            Place::Minion { merged: true, .. } => true,
            Place::Minion { id, .. } => crate::view::minion_face(&e.state.minions[id as usize]).1,
            _ => false,
        });
        let out_of = match from {
            Place::Uncontrolled(s) => {
                let region = regions.entry(s).or_default();
                region.iter().position(|&c| c == copy).map(|i| {
                    region.remove(i);
                    i as u8
                })
            }
            _ => None,
        };
        let into = match (to, rank.get(&copy)) {
            (Place::Uncontrolled(s), Some(&r)) => {
                let region = regions.entry(s).or_default();
                let next = region
                    .iter()
                    .position(|c| rank.get(c).is_some_and(|&q| q > r));
                let i = next.unwrap_or(region.len());
                region.insert(i, copy);
                Some(i as u8)
            }
            _ => None,
        };
        let fact = Fact::Zoned {
            copy,
            card,
            crypt,
            from,
            to,
            at: [spot(e, before, from, copy), spot(e, after, to, copy)],
            slots: [out_of, into],
        };
        tell(out, fact, false);
    }
    // A vampire moved within its region (a reshuffle's), from its place
    // as the moves left the region to the one it has now: the region's
    // order the events' too, which `counters` names places by.
    let mut seats: Vec<u8> = regions.keys().copied().collect();
    seats.sort();
    for s in seats {
        let now: Vec<CopyId> = after
            .slots
            .iter()
            .filter(|&&(o, _, _)| o.0 == s)
            .map(|&(_, c, _)| c)
            .collect();
        let region = regions.get_mut(&s).expect("a seat's region");
        for (i, &copy) in now.iter().enumerate() {
            let Some(j) = region.iter().position(|&c| c == copy).filter(|&j| j != i) else {
                continue;
            };
            let &(_, card, at) = after
                .census
                .iter()
                .find(|&&(c, _, _)| c == copy)
                .expect("an uncontrolled vampire in the census");
            region.remove(j);
            region.insert(i, copy);
            let fact = Fact::Zoned {
                copy,
                card,
                crypt: true,
                from: at,
                to: at,
                at: [spot(e, before, at, copy), spot(e, after, at, copy)],
                slots: [Some(j as u8), Some(i as u8)],
            };
            tell(out, fact, false);
        }
    }
    // A hand turned face up or down, told after the moves: the hand the
    // events built so far is the one it shows.
    for (s, (&was, &now)) in before.hands_seen.iter().zip(&after.hands_seen).enumerate() {
        if was != now && !after.ousted[s] {
            let seat = SeatId(s as u8);
            let cards = after
                .census
                .iter()
                .filter(|&&(_, _, p)| p == Place::Zone(s as u8, "hand"))
                .map(|&(c, card, _)| (c, card))
                .collect();
            let fact = Fact::HandSeen {
                seat,
                seen: [was, now],
                cards,
            };
            tell(out, fact, false);
        }
    }
    // An uncontrolled vampire come with counters (its slot new), told at
    // its place after the moves.
    for &(_, copy, counters) in &after.slots {
        let new = !before.slots.iter().any(|&(_, c, _)| c == copy);
        if new && counters > 0 {
            let fact = Fact::Counters {
                what: slot_of(&after.slots, copy),
                delta: i32::from(counters),
                counters,
                cause,
            };
            tell(out, fact, false);
        }
    }
    // A card come into a contest at once (a unique card put on a minion
    // while another copy is in play), told contested once on the table.
    let on_table = |copy: CopyId, census: &[(CopyId, u16, Place)]| {
        census
            .iter()
            .any(|&(k, _, p)| k == copy && matches!(p, Place::Card { .. }))
    };
    for (i, c) in e.state.cards.iter().enumerate() {
        let new = !matches!(was.get(&c.copy), Some(Place::Card { .. }));
        if c.in_play && c.contested && c.body.is_none() && new && on_table(*c.copy, &after.census) {
            let card = CardId(i as u32);
            let fact = Fact::CardLeaves {
                card,
                to: None,
                contested: true,
            };
            tell(out, fact, told);
        }
    }
    for &minion in &contested_in {
        let to = None;
        let fact = Fact::MinionLeaves {
            minion,
            contested: true,
            to,
        };
        tell(out, fact, told);
    }
    // What a minion come into play has beyond what it brought (an
    // uncontrolled vampire's counters, a returning minion's own blood,
    // else none; unlocked), and a card's (its place's holder's; a new
    // one no counters; unlocked). A minion's capacity, title, markers and
    // traits are the other half: `Game::view`, as the view has them.
    for m in minions_in.into_iter().chain(contested_in) {
        let now = after.minions[m.index()];
        let copy = minion_copy(&e.state, m.index());
        let slot = before.slots.iter().find(|&&(_, c, _)| c == copy);
        let own = before.minions.get(m.index()).map(|b| b.blood);
        let brought = slot.map(|&(_, _, n)| n).or(own).unwrap_or(0);
        if now.blood != brought {
            // Counters beyond its capacity drain as it comes in; blood or
            // life it starts with is its card's, or what put it in play.
            let cause = if now.blood < brought {
                Some(Cause::Effect(Source::Rule(Rule::Capacity)))
            } else {
                cause.or(Some(Cause::Effect(Source::Minion(m))))
            };
            let fact = Fact::Blood {
                minion: m,
                delta: i32::from(now.blood) - i32::from(brought),
                blood: now.blood,
                cause,
            };
            tell(out, fact, false);
        }
        if now.locked {
            let what = Held::Minion(m);
            tell(out, Fact::Locked { what, locked: true }, false);
        }
    }
    for c in cards_in {
        let Some(now) = after.cards[c.index()] else {
            continue;
        };
        // On a minion, controlled by another than the minion's.
        let holder = now.host.map(|m| after.minions[m.index()].controller);
        if holder.is_some_and(|h| h != now.controller) {
            let what = Held::Card(c);
            tell(
                out,
                Fact::Controller {
                    what,
                    seat: now.controller,
                },
                false,
            );
        }
        // Back from a contest, its counters its own still.
        let card = &e.state.cards[c.index()];
        let back = matches!(was.get(&card.copy), Some(Place::Card { .. }));
        if now.counters > 0 && !back {
            let fact = Fact::Counters {
                what: Held::Card(c),
                delta: i32::from(now.counters),
                counters: now.counters,
                cause,
            };
            tell(out, fact, false);
        }
        if now.locked {
            let what = Held::Card(c);
            tell(out, Fact::Locked { what, locked: true }, false);
        }
    }
}

/// `copy`'s place among its seat's uncontrolled vampires in `slots` (a
/// ledger's).
fn slot_of(slots: &[(SeatId, CopyId, u8)], copy: CopyId) -> Held {
    let (seat, _, _) = *slots
        .iter()
        .find(|&&(_, c, _)| c == copy)
        .expect("a slot of the ledger");
    let slot = slots
        .iter()
        .filter(|&&(s, _, _)| s == seat)
        .position(|&(_, c, _)| c == copy)
        .expect("its own seat's") as u8;
    Held::Slot { seat: seat.0, slot }
}

#[cfg(test)]
mod tests {
    use zvtes::cards::Target;
    use zvtes::combat::StrikeHit;
    use zvtes::crypt::CRYPT;
    use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
    use zvtes::game::{Engine, Step};
    use zvtes::ids::SeatId;
    use zvtes::state::{find_card, find_library, CardInPlay};

    use super::Mark;

    fn next(e: &mut Engine) -> DecisionRequest {
        match e.run() {
            Step::Decision(r) => r,
            Step::Over(_) => panic!("game over"),
        }
    }

    /// Passes (or the first option) until `done` holds of a decision.
    fn until(e: &mut Engine, done: impl Fn(&DecisionRequest) -> Option<usize>) {
        loop {
            let r = next(e);
            if let Some(i) = done(&r) {
                e.choose(i);
                return;
            }
            let pass = r.options.iter().position(|o| *o == Choice::Pass);
            e.choose(pass.unwrap_or(0));
        }
    }

    fn pick(e: &mut Engine, want: Choice) {
        until(e, |r| r.options.iter().position(|o| *o == want));
    }

    // A strike declared is marked with what it inflicts as declared (the
    // `strike_declared` event's `n`): Torn Signpost [POT] sets a strength of
    // 3, the hand strike for 3.
    /// The cards on a minion going into a contest are told leaving
    /// contested with it (rules-qa 83.1).
    #[test]
    fn a_card_on_a_contested_minion_is_told_leaving_contested() {
        let mut e = Engine::bare(2);
        let m = e.add_minion(SeatId(0), find_card("Farah Sarroub"), 4, false);
        e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
        let laptop = find_library("Laptop Computer");
        e.state
            .cards
            .push(CardInPlay::new(laptop, SeatId(0), Some(m), 0));
        let card = e.state.cards.len() - 1;
        let before = Mark::of(&e);
        e.state.minions[m.index()].contested = true;
        e.state.cards[card].out_of_play = true;
        let mut out = Vec::new();
        let after = Mark::of(&e).ledger;
        super::changes(
            &e,
            (&before.ledger, &after),
            None,
            true,
            None,
            &mut Default::default(),
            &mut out,
        );
        assert!(
            out.iter().any(|t| matches!(
                t.fact,
                super::Fact::CardLeaves {
                    contested: true,
                    ..
                }
            )),
            "{out:?}"
        );
    }

    /// A card's face told as its places showed it as it moved: drawn
    /// into a hand opened to the viewer only later, a back.
    #[test]
    fn a_card_drawn_before_its_hand_opens_is_told_a_back() {
        use zvtes::copies::CopyId;
        let mut e = Engine::bare(2);
        e.add_minion(SeatId(0), find_card("Farah Sarroub"), 4, false);
        e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
        let laptop = find_library("Laptop Computer");
        e.state.seats[1].library.put(laptop, CopyId(500));
        let before = Mark::of(&e).ledger;
        let top = e.state.seats[1].library.len() - 1;
        let (card, copy) = e.state.seats[1].library.take(top);
        e.state.seats[1].hand.put(card, copy);
        let after = Mark::of(&e).ledger;
        // Seat 0 looks at seat 1's hand afterwards.
        e.state.peeks.push((SeatId(0), SeatId(1)));
        let mut out = Vec::new();
        super::changes(
            &e,
            (&before, &after),
            None,
            true,
            None,
            &mut Default::default(),
            &mut out,
        );
        let t = out
            .into_iter()
            .find(|t| {
                matches!(
                    t.fact,
                    super::Fact::Zoned {
                        copy: CopyId(500),
                        ..
                    }
                )
            })
            .expect("the draw told");
        let t = super::Told {
            faces: super::faces(&e),
            ..t
        };
        let mut viewer = crate::view::Viewer::new(Some(SeatId(0)));
        let decks = crate::numbering::Decks::default();
        let told = viewer.event(&e, &decks, &t);
        assert!(
            matches!(&told, Some(zvtes_api::event::EventKind::CardMoves { card, .. }) if card.id.is_none() && card.copy.is_none()),
            "{told:?}"
        );
    }

    /// The facts of `steps` (each a change to the table, a ledger
    /// after it), told to seat 0 by `Viewer::tell` after a first view:
    /// both views and the events.
    fn told_to_seat_0(
        e: &mut Engine,
        steps: &[&dyn Fn(&mut Engine)],
    ) -> (
        [zvtes_api::view::TableView; 2],
        Vec<zvtes_api::event::EventKind>,
    ) {
        let mut viewer = crate::view::Viewer::new(Some(SeatId(0)));
        let first = viewer.view(e, &Default::default(), &[]);
        let (v, events) = told_to(&mut viewer, e, steps);
        ([first, v], events)
    }

    /// `steps` told to `viewer` as one step.
    fn told_to(
        viewer: &mut crate::view::Viewer,
        e: &mut Engine,
        steps: &[&dyn Fn(&mut Engine)],
    ) -> (zvtes_api::view::TableView, Vec<zvtes_api::event::EventKind>) {
        let decks = crate::numbering::Decks::default();
        let mut ledger = Mark::of(e).ledger;
        let (mut out, mut seen) = (Vec::new(), Default::default());
        for step in steps {
            step(e);
            let next = Mark::of(e).ledger;
            super::changes(e, (&ledger, &next), None, true, None, &mut seen, &mut out);
            ledger = next;
        }
        let faces = super::faces(e);
        let told = out
            .into_iter()
            .map(|t| super::Told {
                faces: faces.clone(),
                ..t
            })
            .collect();
        let (v, named) = viewer.tell(e, &decks, &[], told);
        (v, named.into_iter().filter_map(|(_, k)| k).collect())
    }

    /// A card in play going to another seat's hand: its leaving and its
    /// move name the number it had in sight.
    #[test]
    fn a_card_leaving_to_a_hidden_hand_keeps_its_number_in_every_event() {
        use zvtes::copies::CopyId;
        use zvtes_api::event::EventKind as K;
        let mut e = Engine::bare(2);
        e.add_minion(SeatId(0), find_card("Farah Sarroub"), 4, false);
        let m = e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
        let laptop = find_library("Laptop Computer");
        e.state.cards.push(CardInPlay::entering(
            laptop,
            CopyId(600),
            SeatId(1),
            Some(m),
            0,
        ));
        let i = e.state.cards.len() - 1;
        let back = |e: &mut Engine| {
            e.state.cards[i].in_play = false;
            e.state.seats[1].hand.put(laptop, CopyId(600));
        };
        let ([first, _], events) = told_to_seat_0(&mut e, &[&back]);
        let had = first.seats[1].ready[0].cards[0].card.copy;
        let copies: Vec<_> = events
            .iter()
            .filter_map(|k| match k {
                K::LeavesPlay { card, .. } | K::CardMoves { card, .. } => Some(card.copy),
                _ => None,
            })
            .collect();
        assert_eq!(copies, vec![had, had], "{events:?}");
        assert!(had.is_some());
    }

    /// A card stored face down on a banished vampire's card (Shilmulo
    /// Tarot) is seen by its card's controller alone.
    #[test]
    fn a_card_stored_face_down_on_a_banished_vampire_is_hidden() {
        use zvtes::copies::{CopyId, Place};
        let mut e = Engine::bare(2);
        let tarot = find_library("Shilmulo Tarot");
        e.state
            .cards
            .push(CardInPlay::new(tarot, SeatId(1), None, 0));
        let id = (e.state.cards.len() - 1) as u32;
        let l = Mark::of(&e).ledger;
        let at = Place::Banished {
            seat: 1,
            slot: CopyId(9),
            stored: Some(id),
        };
        let shown = super::spot(&e, &l, at, CopyId(10)).shown;
        assert_eq!(shown & 1, 0, "hidden from seat 0");
        assert_ne!(shown & 2, 0, "seen by its controller");
    }

    /// A minion taken by another Methuselah is told controlled by them.
    #[test]
    fn a_minion_taken_is_told_controlled() {
        let mut e = Engine::bare(2);
        let m = e.add_minion(SeatId(0), find_card("Farah Sarroub"), 4, false);
        e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
        let before = Mark::of(&e);
        e.state.minions[m.index()].owner = SeatId(1);
        let mut out = Vec::new();
        let after = Mark::of(&e).ledger;
        super::changes(
            &e,
            (&before.ledger, &after),
            None,
            true,
            None,
            &mut Default::default(),
            &mut out,
        );
        assert!(
            out.iter().any(|t| t.fact
                == super::Fact::Controller {
                    what: super::Held::Minion(m),
                    seat: SeatId(1),
                }),
            "{out:?}"
        );
    }

    #[test]
    fn a_declared_strike_is_marked_with_its_damage() {
        let pot = CRYPT
            .iter()
            .enumerate()
            .find(|(i, c)| {
                c.disciplines.contains(&"POT") && zvtes::cards::vampire(*i as u16).is_none()
            })
            .map(|(i, _)| i as u16)
            .expect("a vampire with POT");
        let mut e = Engine::bare(2);
        let a = e.add_minion(SeatId(0), pot, 4, false);
        e.add_minion(SeatId(0), find_card("Farah Sarroub"), 4, false);
        e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
        let z = e.add_minion(SeatId(1), find_card("Octane"), 4, false);
        e.state.turn = 4;
        // The blocker's weapon: a strike to choose, so its window stays.
        let femur = find_library("Femur of Toomler");
        e.state
            .cards
            .push(CardInPlay::new(femur, SeatId(1), Some(z), 0));
        e.state.seats[0].hand.push(find_library("Torn Signpost"));
        pick(
            &mut e,
            Choice::TakeAction {
                minion: a,
                action: ActionKind::Bleed,
                target: None,
            },
        );
        pick(&mut e, Choice::DeclareBlock { minion: z });
        pick(
            &mut e,
            Choice::Play {
                card: 0,
                mode: 1,
                actor: Some(a),
                target: Target::None,
            },
        );
        // Up to the blocker's strike choice, the acting minion's declared.
        loop {
            let r = next(&mut e);
            if (r.seat, r.window) == (SeatId(1), WindowLabel::CombatStrike) {
                break;
            }
            let pass = r.options.iter().position(|o| *o == Choice::Pass);
            e.choose(pass.unwrap_or(0));
        }
        let (_, _, _, declared) = Mark::of(&e).strikes[0];
        assert_eq!(
            declared.and_then(|d| d.told),
            Some(StrikeHit {
                n: 3,
                aggravated: false
            })
        );
    }
}
