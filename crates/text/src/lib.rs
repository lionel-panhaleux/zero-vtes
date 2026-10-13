//! The contract in English: events (and options, A7) rendered from their
//! data alone, with the names a client knows — card names by KRCG id
//! (`Cards`), seats and minions from the views and events it was sent
//! (`Names`). What the host sends with the `text` setting is this.

mod event;

use std::collections::HashMap;

use zvtes_api::event::EventKind;
use zvtes_api::ids::{CopyId, KrcgId, MinionKind, Seat};
use zvtes_api::view::{Card, TableView};

pub use event::{event, strike};

/// Card names by KRCG id: the client's card database.
pub trait Cards {
    fn name(&self, id: KrcgId) -> Option<&str>;
}

/// What a client knows of the table's names: seats, and every minion by
/// its copy as views and events showed it (a copy is never reused).
pub struct Names<'a> {
    cards: &'a dyn Cards,
    seats: Vec<String>,
    minions: HashMap<CopyId, String>,
}

impl<'a> Names<'a> {
    pub fn new(cards: &'a dyn Cards) -> Names<'a> {
        Names {
            cards,
            seats: Vec::new(),
            minions: HashMap::new(),
        }
    }

    /// Learn the names a view shows.
    pub fn see(&mut self, v: &TableView) {
        self.seats = v.seats.iter().map(|s| s.name.clone()).collect();
        for m in v.seats.iter().flat_map(|s| s.ready.iter().chain(&s.torpor)) {
            if let Some(c) = m.card.copy {
                let name = minion_name(m.kind, &m.name);
                self.minions.insert(c, name);
            }
        }
    }

    /// Learn the names an event shows (a minion come into play before a
    /// view shows it).
    pub fn hear(&mut self, e: &EventKind) {
        if let EventKind::EntersPlay {
            card,
            minion: Some(kind),
            ..
        }
        | EventKind::LeavesPlay {
            card,
            minion: Some(kind),
            ..
        } = e
        {
            if let Some(c) = card.copy {
                let name = minion_name(*kind, &self.card(card));
                self.minions.entry(c).or_insert(name);
            }
        }
    }

    pub fn seat(&self, s: Seat) -> String {
        self.seats
            .get(s as usize)
            .cloned()
            .unwrap_or_else(|| format!("Seat {}", s + 1))
    }

    pub fn minion(&self, c: CopyId) -> String {
        self.minions
            .get(&c)
            .cloned()
            .unwrap_or_else(|| "a minion".into())
    }

    /// Whether a card is a minion's, one the views or events showed.
    pub fn is_minion(&self, c: &Card) -> bool {
        c.copy.is_some_and(|c| self.minions.contains_key(&c))
    }

    /// A card's name (a back: "a card").
    pub fn card(&self, c: &Card) -> String {
        self.card_id(c.id)
    }

    pub fn card_id(&self, id: Option<KrcgId>) -> String {
        id.and_then(|id| self.cards.name(id))
            .map_or_else(|| "a card".into(), str::to_string)
    }
}

/// A minion as the table names it: a token is nameless.
fn minion_name(kind: MinionKind, name: &str) -> String {
    match kind {
        MinionKind::Token => "a ghoul token".into(),
        _ => name.into(),
    }
}
