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

/// A decklist's key: a KRCG id, or a card's name as `GET /cards` gives
/// it (any case): a vampire whose name several carry takes KRCG's suffix,
/// its group and whether advanced (`Theo Bell (G2 ADV)`); bare, the base
/// vampire of its one group.
fn id_of(key: &str) -> Option<u32> {
    if let Ok(id) = key.parse::<u32>() {
        return Some(id);
    }
    let same = |a: &str, b: &str| a.to_lowercase() == b.to_lowercase();
    if let Some(c) = LIBRARY.iter().find(|c| same(c.name, key)) {
        return Some(c.id);
    }
    let (base, suffix) = match key.strip_suffix(')').and_then(|k| k.rsplit_once(" (")) {
        Some((base, suffix))
            if suffix
                .split(' ')
                .all(|t| same(t, "adv") || group(t).is_some()) =>
        {
            (base, Some(suffix))
        }
        _ => (key, None),
    };
    let named = CRYPT.iter().filter(|c| same(c.name, base));
    let found: Vec<u32> = match suffix {
        Some(suffix) => {
            let adv = suffix.split(' ').any(|t| same(t, "adv"));
            let g = suffix.split(' ').find_map(group);
            named
                .filter(|c| c.adv == adv && g.is_none_or(|g| c.group == g))
                .map(|c| c.id)
                .collect()
        }
        None => {
            let all: Vec<_> = named.collect();
            let base: Vec<u32> = all.iter().filter(|c| !c.adv).map(|c| c.id).collect();
            match base.is_empty() {
                true => all.iter().map(|c| c.id).collect(),
                false => base,
            }
        }
    };
    match found[..] {
        [id] => Some(id),
        _ => None,
    }
}

/// A suffix's group: `G2`.
fn group(token: &str) -> Option<u8> {
    token.strip_prefix(['G', 'g'])?.parse().ok()
}

/// The cards the engine plays (`GET /cards`): KRCG id and the name a
/// decklist may give for it, the crypt then the library.
pub fn cards() -> Vec<(u32, String)> {
    let crypt = CRYPT.iter().map(|c| {
        let mut suffix = Vec::new();
        if CRYPT.iter().any(|o| o.name == c.name && o.group != c.group) {
            suffix.push(format!("G{}", c.group));
        }
        if c.adv {
            suffix.push("ADV".to_string());
        }
        match suffix.is_empty() {
            true => (c.id, c.name.to_string()),
            false => (c.id, format!("{} ({})", c.name, suffix.join(" "))),
        }
    });
    let library = LIBRARY.iter().map(|c| (c.id, c.name.to_string()));
    crypt.chain(library).collect()
}

/// `seat`'s decklist keyed by KRCG ids, a card named twice (by id and by
/// name) one key with its copies summed: what a game and its record take,
/// so a name a later card shares does not change a recorded deck. A key
/// that is no card of the engine's pool is refused, each named.
pub fn by_id(seat: Seat, list: &DeckList) -> Result<DeckList, SpecError> {
    let mut out = DeckList::new();
    let mut unsupported = Vec::new();
    for (key, &n) in list {
        if n == 0 {
            continue;
        }
        match id_of(key) {
            Some(id) if CRYPT.iter().any(|c| c.id == id) || LIBRARY.iter().any(|c| c.id == id) => {
                let copies = out.entry(id.to_string()).or_default();
                *copies = copies.saturating_add(n);
            }
            _ => unsupported.push(key.clone()),
        }
    }
    if !unsupported.is_empty() {
        return Err(SpecError::Unsupported {
            seat,
            cards: unsupported,
        });
    }
    Ok(out)
}

/// Resolve `seat`'s decklist against the engine's pool: cards it does not
/// define named, a deck too small to deal (4 crypt, 7 library) or with a
/// card counted past 255 refused, not clamped.
pub fn resolve(seat: Seat, list: &DeckList) -> Result<Resolved, SpecError> {
    let list = by_id(seat, list)?;
    let mut out = Resolved {
        crypt: Vec::new(),
        library: Vec::new(),
    };
    let (mut crypt, mut library) = (0u32, 0u32);
    let mut over = false;
    for (id, &n) in &list {
        let k: u32 = id.parse().expect("by_id's keys");
        over |= n > u8::MAX as u32;
        let copies = n.min(u8::MAX as u32) as u8;
        if let Some(i) = CRYPT.iter().position(|c| c.id == k) {
            out.crypt.push((i as u16, copies));
            crypt = crypt.saturating_add(n);
        } else if let Some(i) = LIBRARY.iter().position(|c| c.id == k) {
            out.library.push((i as u16, copies));
            library = library.saturating_add(n);
        }
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

/// `seat`'s deck as a game of `seats` takes it, or the refusal a game
/// would make (`POST /decks/check` answers it for a deck alone).
pub fn playable(seat: Seat, list: &DeckList, seats: usize) -> Result<Resolved, SpecError> {
    let deck = resolve(seat, list)?;
    // VEKN's two-player variant: its decks only (rules-digest §18.5).
    if seats == 2 {
        two_player_legal(seat, &deck)?;
    }
    Ok(deck)
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

/// VEKN's two-player variant's decks (`data/two-player.json`, from
/// `scripts/two_player.py`): the cards it allows, its suggested lists.
#[derive(serde::Deserialize)]
struct TwoPlayer {
    allowed: Vec<Allowed>,
    decks: Vec<TwoPlayerList>,
}

#[derive(serde::Deserialize)]
struct Allowed {
    id: u32,
}

#[derive(serde::Deserialize)]
struct TwoPlayerList {
    name: String,
    cards: DeckList,
}

fn two_player() -> &'static TwoPlayer {
    static DATA: std::sync::OnceLock<TwoPlayer> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../../../data/two-player.json"))
            .expect("data/two-player.json")
    })
}

/// VEKN's suggested two-player decklists, by name (`GET
/// /catalog?seats=2`).
pub fn two_player_catalog() -> Vec<(String, DeckList)> {
    two_player()
        .decks
        .iter()
        .map(|d| (d.name.clone(), d.cards.clone()))
        .collect()
}

/// A deck resolved for a two-player game, within the variant's rules: a
/// library of 40 to 60, every card on its list; else the cards off it.
pub fn two_player_legal(seat: Seat, deck: &Resolved) -> Result<(), SpecError> {
    let allowed = &two_player().allowed;
    let ok = |id: u32| allowed.iter().any(|a| a.id == id);
    let ids = deck
        .crypt
        .iter()
        .map(|&(c, _)| CRYPT[c as usize].id)
        .chain(deck.library.iter().map(|&(c, _)| LIBRARY[c as usize].id));
    let cards: Vec<String> = ids.filter(|&id| !ok(id)).map(|id| id.to_string()).collect();
    let library: u32 = deck.library.iter().map(|&(_, n)| u32::from(n)).sum();
    if cards.is_empty() && (40..=60).contains(&library) {
        return Ok(());
    }
    Err(SpecError::TwoPlayerDeck {
        seat,
        cards,
        library,
    })
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

    /// VEKN's suggested two-player lists are within its rules, every one
    /// the engine plays (its Tzimisce list, playing Nesserian, neither on
    /// its own list nor in the pool, excluded by `scripts/two_player.py`).
    #[test]
    fn the_two_player_lists_are_legal() {
        let lists = two_player_catalog();
        assert_eq!(lists.len(), 17);
        for (name, list) in &lists {
            let r = resolve(0, list).expect(name);
            assert_eq!(two_player_legal(0, &r), Ok(()), "{name}");
        }
    }

    /// Every card resolves from the name `cards()` gives it, in any case;
    /// a vampire's name several carry, bare, is refused, not guessed.
    #[test]
    fn a_card_is_found_by_its_name() {
        for (id, name) in cards() {
            assert_eq!(id_of(&name), Some(id), "{name}");
            assert_eq!(id_of(&name.to_lowercase()), Some(id), "{name}");
        }
        let theo = |g: u8| {
            CRYPT
                .iter()
                .find(|c| c.name == "Theo Bell" && c.group == g)
                .unwrap()
                .id
        };
        assert_eq!(id_of("Theo Bell (G2)"), Some(theo(2)));
        assert_eq!(id_of("Theo Bell (G6)"), Some(theo(6)));
        assert_eq!(id_of("Theo Bell"), None);
        assert_eq!(id_of("Theo Bell (ADV)"), None);
    }

    /// A list by ids is its own `by_id` (records and goldens unchanged);
    /// one by names becomes it, a card named twice one key, its copies
    /// summed.
    #[test]
    fn a_list_by_names_is_keyed_by_ids() {
        let list = list_of(&Deck::precon(0));
        assert_eq!(by_id(0, &list), Ok(list.clone()));
        let names: std::collections::HashMap<u32, String> = cards().into_iter().collect();
        let mut named: DeckList = list
            .iter()
            .map(|(id, &n)| (names[&id.parse::<u32>().unwrap()].clone(), n))
            .collect();
        let (first, &n) = list.iter().next().unwrap();
        named.insert(first.clone(), 1);
        let mut want = list.clone();
        *want.get_mut(first).unwrap() = n + 1;
        assert_eq!(by_id(0, &named), Ok(want));
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
        list.insert(id.clone(), 300);
        assert!(matches!(resolve(0, &list), Err(SpecError::DeckSize { .. })));
        // Named twice, its copies summed past what a count holds: refused,
        // not wrapped to a few copies.
        let names: std::collections::HashMap<u32, String> = cards().into_iter().collect();
        list.insert(id.clone(), u32::MAX);
        list.insert(names[&id.parse::<u32>().unwrap()].clone(), 5);
        assert!(matches!(resolve(0, &list), Err(SpecError::DeckSize { .. })));
    }
}
