//! Card copies (docs/online-plan.md "Copies"): each physical card's number,
//! dealt in decklist order (independent of the seed) and carried through
//! every zone. Bookkeeping for clients and records, never part of the
//! game's identity: a `Pile` hashes its card ids alone, `Untracked` nothing.

use std::hash::{Hash, Hasher};
use std::ops::Deref;

use crate::game::Engine;
use crate::rng::Rng;
use crate::state::{GameState, MinionKind, CREATED};

/// Test fixture: number every card a fixture put in place without a copy
/// (`Pile::push`, `add_minion`, slots and cards in play built by hand), so
/// a game from a fixture table follows its copies too.
#[cfg(feature = "fixtures")]
pub fn number_unnumbered(e: &mut Engine) {
    let st = &mut e.state;
    let counter = &mut st.copy_counter.0;
    let mut fresh = |c: &mut CopyId| {
        if *c == CopyId::NONE {
            *c = counter.deal();
        }
    };
    for s in &mut st.seats {
        for pile in [
            &mut s.crypt,
            &mut s.library,
            &mut s.hand,
            &mut s.ash,
            &mut s.ash_crypt,
            &mut s.removed,
            &mut s.removed_crypt,
        ] {
            pile.copies.iter_mut().for_each(&mut fresh);
        }
        for slot in &mut s.uncontrolled {
            if slot.card != CREATED {
                fresh(&mut slot.copy.0);
            }
        }
    }
    for c in &mut st.cards {
        if !c.effect_only {
            fresh(&mut c.copy.0);
        }
        c.stored.copies.iter_mut().for_each(&mut fresh);
        if let Some((_, copy, _)) = c.held_vampire.as_mut() {
            fresh(&mut copy.0);
        }
    }
    for m in &mut st.minions {
        if matches!(m.kind, MinionKind::Vampire(_) | MinionKind::Token { .. }) {
            fresh(&mut m.copy.0);
        }
        if m.merged.is_some() {
            fresh(&mut m.merged_copy.0);
        }
    }
}

/// Where a card copy is, for `census`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// A seat's zone: crypt, library, hand, ash heap (and its crypt
    /// cards), removed cards (and vampires).
    Zone(u8, &'static str),
    Uncontrolled(u8),
    /// A card in play, by its arena index; `stored`: a card stored on it
    /// (or on a training card).
    Card {
        id: u32,
        stored: bool,
    },
    /// A crypt vampire or a token in play, by its arena index; `merged`:
    /// its base card.
    Minion {
        id: u32,
        merged: bool,
    },
    /// A card a Methuselah is playing (announced, in its as-played window,
    /// or the action under way's card).
    InFlight(u8),
    /// Out of play on a banished vampire in `seat`'s uncontrolled region
    /// (Banishment: it stays with it), `slot` that vampire's copy;
    /// `stored`: stored on that card (its arena index), face down perhaps.
    Banished {
        seat: u8,
        slot: CopyId,
        stored: Option<u32>,
    },
    /// In `seat`'s uncontrolled region with training counters (Web of
    /// Knives Recruit), out of play.
    Training(u8),
}

/// Every card in the game with its copy and where it is (`NONE` for an
/// unnumbered fixture card); an ousted Methuselah's cards are in none
/// (rulebook "Ending the Game": removed from every zone).
pub fn census(e: &Engine) -> Vec<(CopyId, u16, Place)> {
    let st = &e.state;
    let mut out = Vec::new();
    let mut add = |copy: CopyId, card: u16, place: Place| out.push((copy, card, place));
    let mut banished = Vec::new();
    for (i, s) in st.seats.iter().enumerate() {
        let i = i as u8;
        for (name, pile) in [
            ("crypt", &s.crypt),
            ("library", &s.library),
            ("hand", &s.hand),
            ("ash", &s.ash),
            ("ash_crypt", &s.ash_crypt),
            ("removed", &s.removed),
            ("removed_crypt", &s.removed_crypt),
        ] {
            for (card, copy) in pile.pairs() {
                add(copy, card, Place::Zone(i, name));
            }
        }
        for slot in &s.uncontrolled {
            if slot.card != CREATED {
                add(*slot.copy, slot.card, Place::Uncontrolled(i));
            }
            if let Some(b) = &slot.banished {
                banished.extend(b.cards.iter().map(|&c| (c.0, i, *slot.copy)));
            }
        }
    }
    for (id, c) in st.cards.iter().enumerate() {
        let id = id as u32;
        let away = banished.iter().find(|&&(b, _, _)| b == id);
        let (at, stored) = match away {
            Some(&(_, seat, slot)) => (
                Place::Banished {
                    seat,
                    slot,
                    stored: None,
                },
                Place::Banished {
                    seat,
                    slot,
                    stored: Some(id),
                },
            ),
            None if c.training => (
                Place::Training(c.controller.0),
                Place::Card { id, stored: true },
            ),
            None => (
                Place::Card { id, stored: false },
                Place::Card { id, stored: true },
            ),
        };
        if (c.in_play || c.training || away.is_some()) && !c.effect_only {
            add(*c.copy, c.card, at);
        }
        for (card, copy) in c.stored.pairs() {
            add(copy, card, stored);
        }
        if let Some((card, copy, _)) = c.held_vampire {
            add(copy.0, card, stored);
        }
    }
    for (id, m) in st.minions.iter().enumerate() {
        let id = id as u32;
        if !m.in_play {
            continue;
        }
        match m.kind {
            MinionKind::Vampire(card) | MinionKind::Token { card, .. } => {
                add(*m.copy, card, Place::Minion { id, merged: false });
            }
            _ => {}
        }
        if let Some(base) = m.merged {
            add(*m.merged_copy, base, Place::Minion { id, merged: true });
        }
    }
    for p in e.in_flight() {
        add(*p.copy, p.card, Place::InFlight(p.seat.0));
    }
    // The action's card, until it enters play or goes back to the hand.
    if let Some(a) = e
        .action()
        .filter(|a| !a.card_entered_play && !a.card_returned)
    {
        if let Some(p) = &a.card {
            add(*p.copy, p.card, Place::InFlight(p.seat.0));
        }
    }
    out
}

/// A minion's card copy: a vampire's or a token's own, else its card in
/// play's (an ally, a created vampire, the Great Beast, a Fury).
pub fn minion_copy(st: &GameState, i: usize) -> CopyId {
    let m = &st.minions[i];
    match m.kind {
        MinionKind::Ally { id, .. }
        | MinionKind::Created { id, .. }
        | MinionKind::GreatBeast { id, .. }
        | MinionKind::Fury { id, .. } => *st.cards[id.index()].copy,
        _ => *m.copy,
    }
}

/// A card copy's number; `NONE` (the default) for a card with none (a
/// test fixture's).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CopyId(pub u16);

impl CopyId {
    pub const NONE: CopyId = CopyId(u16::MAX);
}

impl Default for CopyId {
    fn default() -> Self {
        CopyId::NONE
    }
}

/// A value kept beside the game's identity: always equal, never hashed.
#[derive(Clone, Copy, Debug, Default)]
pub struct Untracked<T>(pub T);

impl<T> PartialEq for Untracked<T> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl<T> Eq for Untracked<T> {}

impl<T> Hash for Untracked<T> {
    fn hash<H: Hasher>(&self, _: &mut H) {}
}

impl<T> Deref for Untracked<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

/// A zone's cards (CRYPT or LIBRARY indices, top at the end) with their
/// copies. Reads go through the card ids (`Deref` to `[u16]`); every move
/// takes and puts a card with its copy.
#[derive(Clone, Debug, Default)]
pub struct Pile {
    cards: Vec<u16>,
    copies: Vec<CopyId>,
}

impl PartialEq for Pile {
    fn eq(&self, o: &Self) -> bool {
        self.cards == o.cards
    }
}

impl Eq for Pile {}

impl PartialEq<Vec<u16>> for Pile {
    /// As its card ids.
    fn eq(&self, o: &Vec<u16>) -> bool {
        &self.cards == o
    }
}

impl Hash for Pile {
    /// As the card ids alone (the hash a `Vec<u16>` zone had).
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.cards.hash(h)
    }
}

impl Deref for Pile {
    type Target = [u16];
    fn deref(&self) -> &[u16] {
        &self.cards
    }
}

impl Pile {
    pub fn new() -> Pile {
        Pile::default()
    }

    /// The copy of the card at `i`.
    pub fn copy(&self, i: usize) -> CopyId {
        self.copies[i]
    }

    pub fn copies(&self) -> &[CopyId] {
        &self.copies
    }

    /// The cards with their copies, bottom first.
    pub fn pairs(&self) -> impl DoubleEndedIterator<Item = (u16, CopyId)> + '_ {
        self.cards.iter().copied().zip(self.copies.iter().copied())
    }

    /// The card ids, owned (a zone's contents read as a list).
    pub fn to_cards(&self) -> Vec<u16> {
        self.cards.clone()
    }

    /// Put a card on top.
    pub fn put(&mut self, card: u16, copy: CopyId) {
        self.cards.push(card);
        self.copies.push(copy);
    }

    /// Put a card at `i` (0: the bottom).
    pub fn put_at(&mut self, i: usize, card: u16, copy: CopyId) {
        self.cards.insert(i, card);
        self.copies.insert(i, copy);
    }

    /// Take the card at `i`.
    pub fn take(&mut self, i: usize) -> (u16, CopyId) {
        (self.cards.remove(i), self.copies.remove(i))
    }

    /// Take the card at `i`, the top card taking its place.
    pub fn swap_take(&mut self, i: usize) -> (u16, CopyId) {
        (self.cards.swap_remove(i), self.copies.swap_remove(i))
    }

    /// Take the top card.
    pub fn take_top(&mut self) -> Option<(u16, CopyId)> {
        let card = self.cards.pop()?;
        Some((card, self.copies.pop().expect("a copy per card")))
    }

    /// Take the first card equal to `card` (from the bottom).
    pub fn take_card(&mut self, card: u16) -> Option<(u16, CopyId)> {
        let i = self.cards.iter().position(|&c| c == card)?;
        Some(self.take(i))
    }

    /// Take every card, bottom first.
    pub fn take_all(&mut self) -> Vec<(u16, CopyId)> {
        let cards = std::mem::take(&mut self.cards);
        let copies = std::mem::take(&mut self.copies);
        cards.into_iter().zip(copies).collect()
    }

    /// Take the cards from `at` up (the top `len - at`), bottom first.
    pub fn take_from(&mut self, at: usize) -> Pile {
        Pile {
            cards: self.cards.split_off(at),
            copies: self.copies.split_off(at),
        }
    }

    /// Take the bottom `n` cards, bottom first.
    pub fn take_bottom(&mut self, n: usize) -> Vec<(u16, CopyId)> {
        let cards = self.cards.drain(..n);
        cards.zip(self.copies.drain(..n)).collect()
    }

    /// Take the cards `keep` rejects, bottom first; the rest stay in order.
    pub fn take_unless(&mut self, mut keep: impl FnMut(u16) -> bool) -> Vec<(u16, CopyId)> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < self.cards.len() {
            if keep(self.cards[i]) {
                i += 1;
            } else {
                out.push(self.take(i));
            }
        }
        out
    }

    /// Put cards on top, in order.
    pub fn put_all(&mut self, cards: impl IntoIterator<Item = (u16, CopyId)>) {
        for (card, copy) in cards {
            self.put(card, copy);
        }
    }

    pub fn clear(&mut self) {
        self.cards.clear();
        self.copies.clear();
    }

    /// Shuffle, copies following their cards: the same random draws as
    /// shuffling the card ids alone.
    pub fn shuffle(&mut self, rng: &mut Rng) {
        let mut order: Vec<usize> = (0..self.cards.len()).collect();
        rng.shuffle(&mut order);
        self.cards = order.iter().map(|&i| self.cards[i]).collect();
        self.copies = order.iter().map(|&i| self.copies[i]).collect();
    }

    /// Reorder by `key`, stably, copies following their cards.
    pub fn sort_by_key<K: Ord>(&mut self, mut key: impl FnMut(u16) -> K) {
        let mut pairs: Vec<_> = self.pairs().collect();
        pairs.sort_by_key(|&(c, _)| key(c));
        self.cards = pairs.iter().map(|p| p.0).collect();
        self.copies = pairs.iter().map(|p| p.1).collect();
    }

    /// Change the card at `i`, keeping its copy (a redacted view's
    /// hidden card).
    pub fn set_card(&mut self, i: usize, card: u16) {
        self.cards[i] = card;
    }

    /// Set every copy (a redacted view: the copies a viewer may not follow).
    pub fn set_copies(&mut self, copy: CopyId) {
        self.copies.fill(copy);
    }

    /// Test fixture: put a card with no copy on top.
    #[cfg(feature = "fixtures")]
    pub fn push(&mut self, card: u16) {
        self.put(card, CopyId::NONE);
    }

    /// Test fixture: reverse the order, copies following their cards.
    #[cfg(feature = "fixtures")]
    pub fn reverse(&mut self) {
        self.cards.reverse();
        self.copies.reverse();
    }

    /// Test fixture: keep the bottom `n` cards.
    #[cfg(feature = "fixtures")]
    pub fn truncate(&mut self, n: usize) {
        self.cards.truncate(n);
        self.copies.truncate(n);
    }

    /// Test fixture: keep the cards `keep` accepts.
    #[cfg(feature = "fixtures")]
    pub fn retain(&mut self, mut keep: impl FnMut(&u16) -> bool) {
        self.take_unless(|c| keep(&c));
    }

    /// Test fixture: put a card with no copy at `i`.
    #[cfg(feature = "fixtures")]
    pub fn insert(&mut self, i: usize, card: u16) {
        self.put_at(i, card, CopyId::NONE);
    }
}

#[cfg(feature = "fixtures")]
impl Extend<u16> for Pile {
    /// Test fixture: put cards with no copy on top.
    fn extend<I: IntoIterator<Item = u16>>(&mut self, cards: I) {
        for c in cards {
            self.push(c);
        }
    }
}

#[cfg(feature = "fixtures")]
impl From<Vec<u16>> for Pile {
    /// Test fixture: cards with no copy.
    fn from(cards: Vec<u16>) -> Pile {
        let copies = vec![CopyId::NONE; cards.len()];
        Pile { cards, copies }
    }
}

#[cfg(feature = "fixtures")]
impl FromIterator<u16> for Pile {
    /// Test fixture: cards with no copy.
    fn from_iter<I: IntoIterator<Item = u16>>(cards: I) -> Pile {
        Pile::from(cards.into_iter().collect::<Vec<_>>())
    }
}

impl FromIterator<(u16, CopyId)> for Pile {
    fn from_iter<I: IntoIterator<Item = (u16, CopyId)>>(pairs: I) -> Pile {
        let mut p = Pile::new();
        p.put_all(pairs);
        p
    }
}

impl IntoIterator for Pile {
    type Item = (u16, CopyId);
    type IntoIter = std::iter::Zip<std::vec::IntoIter<u16>, std::vec::IntoIter<CopyId>>;
    /// The cards with their copies, bottom first.
    fn into_iter(self) -> Self::IntoIter {
        self.cards.into_iter().zip(self.copies)
    }
}

impl<'a> IntoIterator for &'a Pile {
    type Item = &'a u16;
    type IntoIter = std::slice::Iter<'a, u16>;
    fn into_iter(self) -> Self::IntoIter {
        self.cards.iter()
    }
}

/// Copies dealt in order from a counter: the decklists' cards first, then
/// whatever the game creates.
#[derive(Clone, Copy, Debug, Default)]
pub struct CopyCounter(pub u16);

impl CopyCounter {
    /// The next copy number.
    pub fn deal(&mut self) -> CopyId {
        let c = CopyId(self.0);
        self.0 += 1;
        c
    }

    /// The cards, numbered in their order.
    pub fn number(&mut self, cards: Vec<u16>) -> Pile {
        cards.into_iter().map(|c| (c, self.deal())).collect()
    }
}
