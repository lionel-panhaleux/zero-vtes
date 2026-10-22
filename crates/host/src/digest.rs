//! A position's digest (docs/protocol.md §11): what a record's decision
//! was taken on, so a replay on a later engine stops where the game it
//! reads is no longer the game played, though every choice still reads.
//! Over KRCG ids only, in a canonical order, hashed by FNV-1a 64: stable
//! across engine versions while the position is the same. Frozen: what it
//! hashes changed, every record since 2.4.0 stops at its first decision,
//! so a new definition is read by the record's `protocol`, the old one
//! kept for the records before it.

use zvtes::crypt::CRYPT;
use zvtes::game::Engine;
use zvtes::library::LIBRARY;
use zvtes::state::MinionKind;

/// The digest of the position `e` decides on: 16 hex digits.
pub fn digest(e: &Engine) -> String {
    let st = &e.state;
    let library = |i: u16| LIBRARY[i as usize].id;
    let crypt = |i: u16| CRYPT[i as usize].id;
    let mut h = Fnv::new();
    for (s, seat) in st.seats.iter().enumerate() {
        h.tag(b'S');
        h.word(seat.pool as u32);
        h.list(sorted(seat.hand.to_cards().into_iter().map(library)));
        h.list(sorted(seat.ash.to_cards().into_iter().map(library)));
        h.list(sorted(seat.ash_crypt.to_cards().into_iter().map(crypt)));
        // Bottom first, as the pile holds them.
        h.list(seat.library.to_cards().into_iter().map(library));
        h.list(seat.crypt.to_cards().into_iter().map(crypt));
        let mut uncontrolled: Vec<[u32; 2]> = seat
            .uncontrolled
            .iter()
            .map(|u| [crate::describe::slot_id(e, u), u.counters as u32])
            .collect();
        uncontrolled.sort_unstable();
        h.list(uncontrolled.into_iter().flatten());
        let table = st
            .cards
            .iter()
            .filter(|c| c.in_play && c.host.is_none() && c.controller.index() == s)
            .map(|c| library(c.card));
        h.list(sorted(table));
        // Minions sorted whole: two copies of one vampire are alike by id.
        let mut minions: Vec<Vec<u32>> = st
            .minions
            .iter()
            .enumerate()
            .filter(|(_, m)| m.in_play && m.owner.index() == s)
            .map(|(i, m)| {
                let id = match m.kind {
                    MinionKind::Vampire(c) | MinionKind::Imbued(c) => crypt(c),
                    MinionKind::Ally { card, .. }
                    | MinionKind::Created { card, .. }
                    | MinionKind::GreatBeast { card, .. }
                    | MinionKind::Fury { card, .. }
                    | MinionKind::Token { card, .. } => library(card),
                };
                let on = st
                    .cards
                    .iter()
                    .filter(|c| c.in_play && c.host.is_some_and(|h| h.index() == i))
                    .map(|c| library(c.card));
                let mut v = vec![id, m.blood as u32, m.locked as u32, m.in_torpor as u32];
                v.extend(sorted(on));
                v
            })
            .collect();
        minions.sort_unstable();
        h.word(minions.len() as u32);
        for m in minions {
            h.list(m);
        }
    }
    format!("{:016x}", h.0)
}

fn sorted(ids: impl Iterator<Item = u32>) -> Vec<u32> {
    let mut v: Vec<u32> = ids.collect();
    v.sort_unstable();
    v
}

/// FNV-1a 64 (never std's hasher, which changes between Rust releases).
struct Fnv(u64);

impl Fnv {
    fn new() -> Fnv {
        Fnv(0xcbf2_9ce4_8422_2325)
    }

    fn byte(&mut self, b: u8) {
        self.0 = (self.0 ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
    }

    fn word(&mut self, w: u32) {
        for b in w.to_le_bytes() {
            self.byte(b);
        }
    }

    fn tag(&mut self, t: u8) {
        self.byte(t);
    }

    /// Length first: `[1, 2] []` and `[1] [2]` differ.
    fn list(&mut self, ids: impl IntoIterator<Item = u32>) {
        let v: Vec<u32> = ids.into_iter().collect();
        self.word(v.len() as u32);
        for w in v {
            self.word(w);
        }
    }
}
