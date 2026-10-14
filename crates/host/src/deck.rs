//! Decklists as clients hold them: KRCG card ids and copies (Succubus
//! Club's `DeckList = Record<KrcgId, number>`; KRCG deck JSON's
//! `cards[].{id, count}`), resolved against the engine's pool.

use zvtes::crypt::CRYPT;
use zvtes::decks::Deck;
use zvtes::library::LIBRARY;
use zvtes::state::{HAND_SIZE, UNCONTROLLED_START};
use zvtes_api::message::SpecError;
use zvtes_api::{DeckList, Seat};

/// A decklist resolved to engine indices: crypt and library as (CRYPT /
/// LIBRARY index, copies), in the list's id order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub crypt: Vec<(u16, u8)>,
    pub library: Vec<(u16, u8)>,
}

/// Resolve `seat`'s decklist against the engine's pool: ids it does not
/// define named, a deck too small to deal (4 crypt, 7 library) or with a
/// card counted past 255 refused, not clamped.
pub fn resolve(seat: Seat, list: &DeckList) -> Result<Resolved, SpecError> {
    let mut out = Resolved {
        crypt: Vec::new(),
        library: Vec::new(),
    };
    let mut unsupported = Vec::new();
    let (mut crypt, mut library) = (0u32, 0u32);
    let mut over = false;
    for (id, &n) in list {
        if n == 0 {
            continue;
        }
        let Ok(k) = id.parse::<u32>() else {
            unsupported.push(id.clone());
            continue;
        };
        over |= n > u8::MAX as u32;
        let copies = n.min(u8::MAX as u32) as u8;
        if let Some(i) = CRYPT.iter().position(|c| c.id == k) {
            out.crypt.push((i as u16, copies));
            crypt += n;
        } else if let Some(i) = LIBRARY.iter().position(|c| c.id == k) {
            out.library.push((i as u16, copies));
            library += n;
        } else {
            unsupported.push(id.clone());
        }
    }
    if !unsupported.is_empty() {
        return Err(SpecError::Unsupported {
            seat,
            cards: unsupported,
        });
    }
    if over || (crypt as usize) < UNCONTROLLED_START || (library as usize) < HAND_SIZE {
        return Err(SpecError::DeckSize {
            seat,
            crypt,
            library,
        });
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
        .map(|(id, n)| (id.to_string(), n as u32))
        .collect()
}

/// Every deck the engine ships, by name: the precons, the New Blood
/// minis, the anniversary decks, then the TWDA decks.
pub fn catalog() -> Vec<(String, DeckList)> {
    Deck::all()
        .map(|d| (d.name.to_string(), list_of(&d)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shipped_deck_resolves_to_itself() {
        for deck in Deck::all() {
            let r = resolve(0, &list_of(&deck)).expect(deck.name);
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
            resolve(1, &list),
            Err(SpecError::Unsupported {
                seat: 1,
                cards: vec!["100000".into(), "not-an-id".into()]
            })
        );
    }

    #[test]
    fn a_deck_too_small_to_deal_is_refused() {
        let list: DeckList = [(CRYPT[0].id.to_string(), 3), (LIBRARY[0].id.to_string(), 7)]
            .into_iter()
            .collect();
        assert_eq!(
            resolve(0, &list),
            Err(SpecError::DeckSize {
                seat: 0,
                crypt: 3,
                library: 7
            })
        );
    }

    /// A count past what the engine holds is refused, not clamped.
    #[test]
    fn a_count_past_255_is_refused() {
        let mut list = list_of(&Deck::precon(0));
        let (id, _) = list
            .iter()
            .next_back()
            .map(|(k, v)| (k.clone(), *v))
            .unwrap();
        list.insert(id, 300);
        assert!(matches!(resolve(0, &list), Err(SpecError::DeckSize { .. })));
    }
}
