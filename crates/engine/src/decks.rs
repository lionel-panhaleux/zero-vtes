//! Decklists: the 14 full-size V5 precons (the v1 training roster, DESIGN
//! §3), the 14 New Blood mini-starters, joined from the generated crypt
//! and library tables, and the tournament-winning decks fully in the pool
//! (`twda.rs`).

use crate::crypt::{MINI_PRECONS, PRECONS};
use crate::library::{MINI_LIBRARIES, PRECON_LIBRARIES};
use crate::twda::TWDA;

/// Cards and copies: (CRYPT or LIBRARY index, copies).
pub type CardList<'a> = &'a [(u16, u8)];

/// A decklist: crypt and library as (CRYPT / LIBRARY index, copies).
#[derive(Clone, Copy, Debug)]
pub struct Deck {
    pub name: &'static str,
    pub crypt: &'static [(u16, u8)],
    pub library: &'static [(u16, u8)],
}

impl Deck {
    /// The full-size precon `i` (`PRECONS` order).
    pub fn precon(i: usize) -> Deck {
        Deck {
            name: PRECONS[i].0,
            crypt: PRECONS[i].1,
            library: PRECON_LIBRARIES[i].1,
        }
    }

    /// The New Blood mini-starter `i` (`MINI_PRECONS` order).
    pub fn mini(i: usize) -> Deck {
        Deck {
            name: MINI_PRECONS[i].0,
            crypt: MINI_PRECONS[i].1,
            library: MINI_LIBRARIES[i].1,
        }
    }

    /// The TWDA deck `i` (`TWDA` order).
    pub fn twda(i: usize) -> Deck {
        Deck {
            name: TWDA[i].0,
            crypt: TWDA[i].2,
            library: TWDA[i].3,
        }
    }

    /// Every decklist: the precons, the minis, then the TWDA decks.
    pub fn all() -> impl Iterator<Item = Deck> {
        (0..PRECONS.len())
            .map(Deck::precon)
            .chain((0..MINI_PRECONS.len()).map(Deck::mini))
            .chain((0..TWDA.len()).map(Deck::twda))
    }

    /// The share of crypt two decks have in common: shared copies over the
    /// smaller crypt. Tables avoid seating decks sharing more than 20%
    /// (owner, 2026-09-28: crippling contests; `crates/py`).
    pub fn crypt_share(&self, other: &Deck) -> f32 {
        let shared: u32 = self
            .crypt
            .iter()
            .filter_map(|&(c, n)| {
                other
                    .crypt
                    .iter()
                    .find(|&&(o, _)| o == c)
                    .map(|&(_, m)| n.min(m) as u32)
            })
            .sum();
        let size = |d: &Deck| d.crypt.iter().map(|&(_, n)| n as u32).sum::<u32>();
        shared as f32 / size(self).min(size(other)).max(1) as f32
    }

    /// The deck's clan: the clan with the most crypt copies (ties: the
    /// first in crypt order). Training draws a clan, then a deck of it.
    pub fn clan(&self) -> &'static str {
        let mut counts: Vec<(&'static str, u32)> = Vec::new();
        for &(c, n) in self.crypt {
            let clan = crate::crypt::CRYPT[c as usize].clan;
            match counts.iter_mut().find(|(k, _)| *k == clan) {
                Some((_, m)) => *m += n as u32,
                None => counts.push((clan, n as u32)),
            }
        }
        let best = counts.iter().map(|&(_, n)| n).max().unwrap_or(0);
        counts
            .into_iter()
            .find(|&(_, n)| n == best)
            .map_or("", |(k, _)| k)
    }

    /// The crypt as a list of cards, one entry per copy.
    pub fn crypt_cards(&self) -> Vec<u16> {
        expand(self.crypt)
    }

    /// The library as a list of cards, one entry per copy.
    pub fn library_cards(&self) -> Vec<u16> {
        expand(self.library)
    }
}

pub(crate) fn expand(list: &[(u16, u8)]) -> Vec<u16> {
    list.iter()
        .flat_map(|&(card, copies)| std::iter::repeat_n(card, copies as usize))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypt::CRYPT;
    use crate::library::LIBRARY;

    #[test]
    fn crypt_share_counts_copies_over_the_smaller_crypt() {
        let a = Deck::precon(0);
        assert_eq!(a.crypt_share(&a), 1.0);
        let b = Deck {
            name: "half",
            crypt: &[],
            library: &[],
        };
        assert_eq!(a.crypt_share(&b), 0.0);
        // Distinct precons of one clan share some vampires, others none.
        let shares: Vec<f32> = (1..14).map(|i| a.crypt_share(&Deck::precon(i))).collect();
        assert!(shares.iter().all(|&x| (0.0..=1.0).contains(&x)));
    }

    #[test]
    fn every_precon_is_its_own_clan() {
        let mut clans: Vec<&str> = (0..14).map(|i| Deck::precon(i).clan()).collect();
        clans.sort();
        clans.dedup();
        assert_eq!(clans.len(), 14);
    }

    #[test]
    fn decklists_are_complete() {
        assert_eq!(PRECONS.len(), 14);
        assert_eq!(MINI_PRECONS.len(), 14);
        for (i, (name, _)) in PRECON_LIBRARIES.iter().enumerate() {
            assert_eq!(*name, PRECONS[i].0);
        }
        for (i, (name, _)) in MINI_LIBRARIES.iter().enumerate() {
            assert_eq!(*name, MINI_PRECONS[i].0);
        }
        for deck in Deck::all() {
            let (crypt, library) = (deck.crypt_cards(), deck.library_cards());
            assert!(crypt.iter().all(|&c| (c as usize) < CRYPT.len()));
            assert!(library.iter().all(|&c| (c as usize) < LIBRARY.len()));
            let (crypt_sizes, sizes) = match deck.name {
                n if n.contains("New Blood") => (6..=6, 48..=49),
                n if PRECONS.iter().any(|p| p.0 == n) => (12..=12, 77..=77),
                // Tournament-legal (tournament rules: crypt 12+, library
                // 60–90).
                _ => (12..=40, 60..=90),
            };
            assert!(crypt_sizes.contains(&crypt.len()), "{}", deck.name);
            assert!(sizes.contains(&library.len()), "{}", deck.name);
        }
    }
}
