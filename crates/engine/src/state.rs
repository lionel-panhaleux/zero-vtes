use crate::ids::{MinionId, SeatId};

pub const START_POOL: i32 = 30;
pub const CRYPT_SIZE: usize = 12;
pub const UNCONTROLLED_TARGET: usize = 4;

/// P0 minion: a vanilla vampire — capacity and blood, no disciplines, no
/// cards. `bled_this_turn` is the NRA special cap (rules-qa 1.4: one bleed
/// per minion per turn, set on resolution, blocked included).
#[derive(Clone, Debug)]
pub struct Minion {
    pub owner: SeatId,
    pub capacity: u8,
    pub blood: u8,
    pub locked: bool,
    pub in_torpor: bool,
    pub in_play: bool,
    pub bled_this_turn: bool,
}

#[derive(Clone, Debug)]
pub struct UncontrolledSlot {
    pub capacity: u8,
    pub counters: u8,
}

#[derive(Clone, Debug)]
pub struct Seat {
    pub pool: i32,
    pub ousted: bool,
    pub vps: u32,
    pub has_edge: bool,
    /// Remaining crypt cards (capacities only in P0), top at the end.
    pub crypt: Vec<u8>,
    pub uncontrolled: Vec<UncontrolledSlot>,
    pub turns_taken: u32,
}

#[derive(Clone, Debug)]
pub struct GameState {
    pub seats: Vec<Seat>,
    /// Arena: minions are never removed, only marked out of play.
    pub minions: Vec<Minion>,
    pub active: SeatId,
    pub turn: u32,
}

impl GameState {
    pub fn living(&self) -> usize {
        self.seats.iter().filter(|s| !s.ousted).count()
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

    pub fn minions_of(&self, s: SeatId) -> impl Iterator<Item = MinionId> + '_ {
        self.minions
            .iter()
            .enumerate()
            .filter(move |(_, m)| m.owner == s && m.in_play)
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

    /// FNV-1a over a canonical serialization; used for replay equivalence.
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
            mix(s.vps as u64);
            mix(s.has_edge as u64);
            mix(s.turns_taken as u64);
            mix(s.crypt.len() as u64);
            for &c in &s.crypt {
                mix(c as u64);
            }
            mix(s.uncontrolled.len() as u64);
            for u in &s.uncontrolled {
                mix(u.capacity as u64);
                mix(u.counters as u64);
            }
        }
        mix(self.minions.len() as u64);
        for m in &self.minions {
            mix(m.owner.0 as u64);
            mix(m.capacity as u64);
            mix(m.blood as u64);
            mix(m.locked as u64);
            mix(m.in_torpor as u64);
            mix(m.in_play as u64);
            mix(m.bled_this_turn as u64);
        }
        h
    }
}
