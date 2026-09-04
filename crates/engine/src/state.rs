use crate::cards::Layer;
use crate::crypt::{CryptCard, Title, CRYPT};
use crate::ids::{MinionId, SeatId};
use crate::library::LIBRARY;

pub const START_POOL: i32 = 30;
pub const UNCONTROLLED_START: usize = 4;
pub const HAND_SIZE: usize = 7;

/// CRYPT index of the vampire with this printed name (test helper).
pub fn find_card(name: &str) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name)
        .unwrap_or_else(|| panic!("no V5 crypt card named {name:?}")) as u16
}

/// LIBRARY index of the library card with this printed name (test helper).
pub fn find_library(name: &str) -> u16 {
    LIBRARY
        .iter()
        .position(|c| c.name == name)
        .unwrap_or_else(|| panic!("no V5 library card named {name:?}")) as u16
}

/// Contest identity of a title (rulebook "Contested Titles" / "Vampiric
/// Sects"): prince, baron (and archbishop) of one city contest each other;
/// each clan's justicar is unique; primogen is not unique.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleKey {
    City(&'static str),
    Justicar(&'static str),
}

pub fn title_key(t: Title) -> Option<TitleKey> {
    match t {
        Title::Prince(c) | Title::Baron(c) => Some(TitleKey::City(c)),
        Title::Justicar(k) => Some(TitleKey::Justicar(k)),
        Title::None | Title::Primogen => None,
    }
}

pub fn title_votes(t: Title) -> u8 {
    match t {
        Title::None => 0,
        Title::Primogen => 1,
        Title::Prince(_) | Title::Baron(_) => 2,
        Title::Justicar(_) => 3,
    }
}

/// A vampire in play. `contested`: the card is contested — face down and out
/// of play, blood kept (rulebook "Contested Cards"). `title_contested` /
/// `title_lost`: the printed title is under contest / was yielded for the
/// rest of the game. `bled_this_turn` is the NRA special cap (rules-qa 1.4).
#[derive(Clone, Debug)]
pub struct Minion {
    pub owner: SeatId,
    pub card: u16,
    pub blood: u8,
    pub locked: bool,
    pub in_torpor: bool,
    pub in_play: bool,
    pub contested: bool,
    pub title_contested: bool,
    pub title_lost: bool,
    pub bled_this_turn: bool,
    /// One political action per minion per turn (rulebook "Political
    /// Action"; NRA on resolution, rules-qa 1.4).
    pub political_this_turn: bool,
    /// Expulsion: cannot play reaction cards, block or cast votes or
    /// ballots this turn.
    pub expelled: bool,
    /// Action cards this minion acted with this turn (NRA name identity,
    /// rules-qa 1.4).
    pub acted: Vec<u16>,
}

impl Minion {
    pub fn card(&self) -> &'static CryptCard {
        &CRYPT[self.card as usize]
    }

    pub fn name(&self) -> &'static str {
        self.card().name
    }

    pub fn capacity(&self) -> u8 {
        self.card().capacity
    }

    /// Controlled and present: in play and not contested (a contested copy
    /// is out of play until the contest ends).
    pub fn present(&self) -> bool {
        self.in_play && !self.contested
    }

    /// Ready: present and not in torpor (locked or not).
    pub fn ready(&self) -> bool {
        self.present() && !self.in_torpor
    }

    /// Effective title: none while contested, yielded, or out of play.
    pub fn title(&self) -> Title {
        if self.title_lost || self.title_contested || !self.present() {
            Title::None
        } else {
            self.card().title
        }
    }

    /// Votes this vampire can cast right now (title votes need a ready,
    /// uncontested holder; rulebook "Gaining Votes").
    pub fn votes(&self) -> u8 {
        if self.ready() {
            title_votes(self.title())
        } else {
            0
        }
    }
}

/// A library card in play (a master on a vampire, a location, …).
#[derive(Clone, Debug)]
pub struct CardInPlay {
    pub card: u16,
    pub owner: SeatId,
    /// Minion the card is on, if any.
    pub host: Option<MinionId>,
    pub in_play: bool,
    pub locked: bool,
    pub counters: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardId(pub u32);

impl CardId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Debug)]
pub struct UncontrolledSlot {
    pub card: u16,
    pub counters: u8,
    /// A banished vampire (Banishment): the arena entry it left, with its
    /// cards face down on it; it comes back as that entry when influenced
    /// out again (rules-qa 5.7).
    pub banished: Option<MinionId>,
}

/// An out-of-turn master card played since the seat's last master phase
/// (rulebook "Out-of-turn"; rules-qa 3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutOfTurn {
    pub trifle: bool,
}

#[derive(Clone, Debug)]
pub struct Seat {
    pub pool: i32,
    pub ousted: bool,
    /// Victory points in half-point units (tournament rules 3.7: 0.5 VP for
    /// surviving at the time limit).
    pub half_vps: u32,
    pub has_edge: bool,
    /// Remaining crypt cards (CRYPT indices), top at the end.
    pub crypt: Vec<u16>,
    pub uncontrolled: Vec<UncontrolledSlot>,
    /// Library (LIBRARY indices), top at the end; hand; ash heap.
    pub library: Vec<u16>,
    pub hand: Vec<u16>,
    pub ash: Vec<u16>,
    /// Master phase actions left in the current master phase.
    pub mpa: u8,
    /// One additional master phase action from trifles per master phase.
    pub trifle_used: bool,
    /// Out-of-turn master card played since the last master phase.
    pub out_of_turn: Option<OutOfTurn>,
    /// Discard phase actions left in the current discard phase.
    pub dpa: u8,
    pub turns_taken: u32,
    /// Cards played with "do not replace until your next unlock phase":
    /// the hand stays that many short until then.
    pub delayed_draws: u8,
    /// Political action card names the seat's minions cannot perform again
    /// this turn (Delaying Tactics).
    pub forbidden_political: Vec<&'static str>,
}

#[derive(Clone, Debug)]
pub struct GameState {
    pub seats: Vec<Seat>,
    /// Arena: minions are never removed, only marked out of play.
    pub minions: Vec<Minion>,
    /// Arena of library cards in play.
    pub cards: Vec<CardInPlay>,
    /// Continuous effects (DESIGN §4.5 scope layers).
    pub layers: Vec<Layer>,
    pub active: SeatId,
    /// Game turn counter, 1-based once the first turn starts.
    pub turn: u32,
}

impl GameState {
    pub fn living(&self) -> usize {
        self.seats.iter().filter(|s| !s.ousted).count()
    }

    /// Living seats clockwise starting at `from` (inclusive).
    pub fn clockwise_from(&self, from: SeatId) -> Vec<SeatId> {
        let n = self.seats.len();
        (0..n)
            .map(|k| SeatId(((from.index() + k) % n) as u8))
            .filter(|s| !self.seats[s.index()].ousted)
            .collect()
    }

    /// Next unousted seat clockwise. Returns `s` itself if it is the sole
    /// survivor.
    pub fn prey(&self, s: SeatId) -> SeatId {
        let n = self.seats.len();
        let mut i = (s.index() + 1) % n;
        while self.seats[i].ousted && i != s.index() {
            i = (i + 1) % n;
        }
        SeatId(i as u8)
    }

    pub fn predator(&self, s: SeatId) -> SeatId {
        let n = self.seats.len();
        let mut i = (s.index() + n - 1) % n;
        while self.seats[i].ousted && i != s.index() {
            i = (i + n - 1) % n;
        }
        SeatId(i as u8)
    }

    /// Present (in play, uncontested) minions of a seat, arena order.
    pub fn minions_of(&self, s: SeatId) -> impl Iterator<Item = MinionId> + '_ {
        self.minions
            .iter()
            .enumerate()
            .filter(move |(_, m)| m.owner == s && m.present())
            .map(|(i, _)| MinionId(i as u32))
    }

    /// Ready and unlocked: eligible to act normally or to attempt a block.
    pub fn ready_unlocked(&self, s: SeatId) -> Vec<MinionId> {
        self.minions_of(s)
            .filter(|m| {
                let m = &self.minions[m.index()];
                !m.locked && !m.in_torpor
            })
            .collect()
    }

    /// Vampires in torpor, any controller — rescue and diablerie targets.
    pub fn in_torpor(&self) -> Vec<MinionId> {
        self.minions
            .iter()
            .enumerate()
            .filter(|(_, m)| m.present() && m.in_torpor)
            .map(|(i, _)| MinionId(i as u32))
            .collect()
    }

    /// Does `s` already have a copy of this vampire in play, in any state
    /// (rulebook: no voluntary self-contest)?
    pub fn controls_copy(&self, s: SeatId, card: u16) -> bool {
        let name = CRYPT[card as usize].name;
        self.minions
            .iter()
            .any(|m| m.owner == s && m.in_play && m.name() == name)
    }

    /// Other seats' in-play copies of `m`'s vampire (contest rivals).
    pub fn card_rivals(&self, m: MinionId) -> Vec<MinionId> {
        let me = &self.minions[m.index()];
        self.minions
            .iter()
            .enumerate()
            .filter(|(i, o)| {
                *i != m.index()
                    && o.owner != me.owner
                    && o.in_play
                    && !self.seats[o.owner.index()].ousted
                    && o.name() == me.name()
            })
            .map(|(i, _)| MinionId(i as u32))
            .collect()
    }

    /// Other present vampires (any seat) claiming a title that contests
    /// `m`'s printed title.
    pub fn title_rivals(&self, m: MinionId) -> Vec<MinionId> {
        let me = &self.minions[m.index()];
        let Some(key) = title_key(me.card().title) else {
            return Vec::new();
        };
        self.minions
            .iter()
            .enumerate()
            .filter(|(i, o)| {
                *i != m.index()
                    && o.present()
                    && !o.title_lost
                    && !self.seats[o.owner.index()].ousted
                    && title_key(o.card().title) == Some(key)
            })
            .map(|(i, _)| MinionId(i as u32))
            .collect()
    }

    /// FNV-1a over a canonical serialization; used for replay equivalence.
    /// Every field that can influence a later decision must be mixed in.
    pub fn hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |x: u64| {
            h ^= x;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        };
        mix(self.active.0 as u64);
        mix(self.turn as u64);
        for s in &self.seats {
            mix(s.pool as u64);
            mix(s.ousted as u64);
            mix(s.half_vps as u64);
            mix(s.has_edge as u64);
            mix(s.turns_taken as u64);
            mix(s.mpa as u64);
            mix(s.trifle_used as u64);
            mix(s.dpa as u64);
            mix(s.delayed_draws as u64);
            mix(s.forbidden_political.len() as u64);
            for n in &s.forbidden_political {
                mix(n.len() as u64);
            }
            mix(match s.out_of_turn {
                None => 0,
                Some(OutOfTurn { trifle }) => 1 + trifle as u64,
            });
            for zone in [
                &s.crypt,
                &s.uncontrolled_cards(),
                &s.library,
                &s.hand,
                &s.ash,
            ] {
                mix(zone.len() as u64);
                for &c in zone.iter() {
                    mix(c as u64);
                }
            }
            for u in &s.uncontrolled {
                mix(u.counters as u64);
                mix(u.banished.map_or(0, |m| m.0 as u64 + 1));
            }
        }
        mix(self.minions.len() as u64);
        for m in &self.minions {
            mix(m.owner.0 as u64);
            mix(m.card as u64);
            mix(m.blood as u64);
            mix(m.locked as u64);
            mix(m.in_torpor as u64);
            mix(m.in_play as u64);
            mix(m.contested as u64);
            mix(m.title_contested as u64);
            mix(m.title_lost as u64);
            mix(m.bled_this_turn as u64);
            mix(m.political_this_turn as u64);
            mix(m.expelled as u64);
            mix(m.acted.len() as u64);
            for &c in &m.acted {
                mix(c as u64);
            }
        }
        mix(self.cards.len() as u64);
        for c in &self.cards {
            mix(c.card as u64);
            mix(c.owner.0 as u64);
            mix(c.host.map(|m| m.0 as u64 + 1).unwrap_or(0));
            mix(c.in_play as u64);
            mix(c.locked as u64);
            mix(c.counters as u64);
        }
        mix(self.layers.len() as u64);
        for l in &self.layers {
            mix(l.hash());
        }
        h
    }
}

impl Seat {
    fn uncontrolled_cards(&self) -> Vec<u16> {
        self.uncontrolled.iter().map(|u| u.card).collect()
    }
}
