//! Decklists as clients hold them: KRCG card ids and copies (Succubus
//! Club's `DeckList = Record<KrcgId, number>`; KRCG deck JSON's
//! `cards[].{id, count}`), resolved against the engine's pool.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use zvtes::crypt::CRYPT;
use zvtes::decks::Deck;
use zvtes::library::LIBRARY;
use zvtes::state::{HAND_SIZE, UNCONTROLLED_START};

/// KRCG id (as a string, the JSON key) → copies.
pub type DeckList = BTreeMap<String, u8>;

/// A decklist resolved to engine indices: crypt and library as (CRYPT /
/// LIBRARY index, copies), in the list's id order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub crypt: Vec<(u16, u8)>,
    pub library: Vec<(u16, u8)>,
}

/// Why a decklist cannot be played.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum DeckError {
    /// Ids the engine does not define (not in the pool, or not card ids).
    Unsupported { ids: Vec<String> },
    /// Too few cards for the opening deal (4 crypt, 7 library).
    TooSmall { crypt: usize, library: usize },
}

/// Resolve a decklist against the engine's pool.
pub fn resolve(list: &DeckList) -> Result<Resolved, DeckError> {
    let mut out = Resolved {
        crypt: Vec::new(),
        library: Vec::new(),
    };
    let mut unsupported = Vec::new();
    for (id, &n) in list {
        if n == 0 {
            continue;
        }
        let Ok(k) = id.parse::<u32>() else {
            unsupported.push(id.clone());
            continue;
        };
        if let Some(i) = CRYPT.iter().position(|c| c.id == k) {
            out.crypt.push((i as u16, n));
        } else if let Some(i) = LIBRARY.iter().position(|c| c.id == k) {
            out.library.push((i as u16, n));
        } else {
            unsupported.push(id.clone());
        }
    }
    if !unsupported.is_empty() {
        return Err(DeckError::Unsupported { ids: unsupported });
    }
    let size = |l: &[(u16, u8)]| l.iter().map(|&(_, n)| n as usize).sum::<usize>();
    let (crypt, library) = (size(&out.crypt), size(&out.library));
    if crypt < UNCONTROLLED_START || library < HAND_SIZE {
        return Err(DeckError::TooSmall { crypt, library });
    }
    Ok(out)
}

/// The decklist of one of the engine's decks (precons, minis, TWDA).
pub fn list_of(deck: &Deck) -> DeckList {
    let crypt = deck.crypt.iter().map(|&(c, n)| (CRYPT[c as usize].id, n));
    let library = deck
        .library
        .iter()
        .map(|&(c, n)| (LIBRARY[c as usize].id, n));
    crypt
        .chain(library)
        .map(|(id, n)| (id.to_string(), n))
        .collect()
}

/// Every deck the engine ships, by name: the precons, the New Blood
/// minis, the anniversary decks, then the TWDA decks.
pub fn catalog() -> Vec<(&'static str, DeckList)> {
    Deck::all().map(|d| (d.name, list_of(&d))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shipped_deck_resolves_to_itself() {
        for deck in Deck::all() {
            let r = resolve(&list_of(&deck)).expect(deck.name);
            let mut crypt = deck.crypt.to_vec();
            let mut library = deck.library.to_vec();
            crypt.sort_by_key(|&(c, _)| CRYPT[c as usize].id);
            library.sort_by_key(|&(c, _)| LIBRARY[c as usize].id);
            let mut got = r.clone();
            got.crypt.sort_by_key(|&(c, _)| CRYPT[c as usize].id);
            got.library.sort_by_key(|&(c, _)| LIBRARY[c as usize].id);
            assert_eq!((got.crypt, got.library), (crypt, library), "{}", deck.name);
        }
    }

    #[test]
    fn unsupported_ids_are_named() {
        let mut list = list_of(&Deck::precon(0));
        list.insert("100000".into(), 1);
        list.insert("not-an-id".into(), 2);
        assert_eq!(
            resolve(&list),
            Err(DeckError::Unsupported {
                ids: vec!["100000".into(), "not-an-id".into()]
            })
        );
    }

    #[test]
    fn a_deck_too_small_to_deal_is_refused() {
        let list: DeckList = [(CRYPT[0].id.to_string(), 3), (LIBRARY[0].id.to_string(), 7)]
            .into_iter()
            .collect();
        assert_eq!(
            resolve(&list),
            Err(DeckError::TooSmall {
                crypt: 3,
                library: 7
            })
        );
    }
}
