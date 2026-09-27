//! Decklists: the 14 full-size V5 precons (the v1 training roster, DESIGN
//! §3) and the 14 New Blood mini-starters, joined from the generated crypt
//! and library tables.

use crate::crypt::{MINI_PRECONS, PRECONS};
use crate::library::{MINI_LIBRARIES, PRECON_LIBRARIES};

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

    /// Every decklist: the precons, then the minis.
    pub fn all() -> impl Iterator<Item = Deck> {
        (0..PRECONS.len())
            .map(Deck::precon)
            .chain((0..MINI_PRECONS.len()).map(Deck::mini))
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

fn expand(list: &[(u16, u8)]) -> Vec<u16> {
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
            let full = !deck.name.contains("New Blood");
            assert_eq!(crypt.len(), if full { 12 } else { 6 }, "{}", deck.name);
            let sizes = if full { 77..=77 } else { 48..=49 };
            assert!(sizes.contains(&library.len()), "{}", deck.name);
        }
    }
}
