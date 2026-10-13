//! The contract in English: events (and options, A7) rendered from their
//! data alone, with the names a client knows — card names by KRCG id
//! (`Cards`), seats and minions from the views and events it was sent
//! (`Names`). What the host sends with the `text` setting is this.

mod effect;
mod event;
mod option;

use std::collections::HashMap;

use zvtes_api::event::EventKind;
use zvtes_api::ids::{CopyId, KrcgId, MinionKind, Seat};
use zvtes_api::view::{ActionView, Card, ReferendumView, TableView};

pub use effect::{label, part, parts};
pub use event::{event, strike};
pub use option::option;

/// Card names by KRCG id: the client's card database.
pub trait Cards {
    fn name(&self, id: KrcgId) -> Option<&str>;
    /// A clan, Discipline or circle by its snake_case id, as printed
    /// ("gangrel_antitribu": "Gangrel antitribu"); `None`: title case.
    fn value(&self, id: &str) -> Option<&str> {
        let _ = id;
        None
    }
}

/// What a client knows of the table's names: seats, and every minion by
/// its copy as views and events showed it (a copy is never reused).
pub struct Names<'a> {
    cards: &'a dyn Cards,
    seats: Vec<String>,
    /// Each seat's prey, as the last view showed.
    prey: Vec<Seat>,
    minions: HashMap<CopyId, String>,
    /// Cards on the table by copy, as the table names them: a card on a
    /// minion with it ("Luciano's Feral Hound"), an uncontrolled vampire.
    table: HashMap<CopyId, String>,
    /// The minion each card on one is on.
    hosts: HashMap<CopyId, CopyId>,
    /// Each card on the table's own name (without the minion it is on).
    own: HashMap<CopyId, String>,
    /// The seat controlling each minion, as the last view showed.
    controllers: HashMap<CopyId, Seat>,
    /// The action under way, as the last view showed.
    action: Option<ActionView>,
    /// The referendum under way, as the last view showed.
    referendum: Option<ReferendumView>,
}

impl<'a> Names<'a> {
    pub fn new(cards: &'a dyn Cards) -> Names<'a> {
        Names {
            cards,
            seats: Vec::new(),
            prey: Vec::new(),
            minions: HashMap::new(),
            table: HashMap::new(),
            hosts: HashMap::new(),
            own: HashMap::new(),
            controllers: HashMap::new(),
            action: None,
            referendum: None,
        }
    }

    /// Learn the names a view shows.
    pub fn see(&mut self, v: &TableView) {
        self.seats = v.seats.iter().map(|s| s.name.clone()).collect();
        self.prey = v.seats.iter().map(|s| s.prey).collect();
        self.action = v.action.clone();
        self.referendum = v.referendum.clone();
        for (seat, s) in v.seats.iter().enumerate() {
            for m in s.ready.iter().chain(&s.torpor) {
                let name = minion_name(m.kind, &m.name);
                for c in &m.cards {
                    if let Some(copy) = c.card.copy {
                        let card = self.card(&c.card);
                        let owner = match m.kind {
                            MinionKind::Token => "ghoul token".to_string(),
                            _ => name.clone(),
                        };
                        self.table.insert(copy, format!("{owner}'s {card}"));
                        self.own.insert(copy, card.clone());
                        if let Some(h) = m.card.copy {
                            self.hosts.insert(copy, h);
                        }
                    }
                }
                if let Some(c) = m.card.copy {
                    self.minions.insert(c, name);
                    self.controllers.insert(c, seat as Seat);
                }
            }
            for c in &s.in_play {
                if let Some(copy) = c.card.copy {
                    let card = self.card(&c.card);
                    self.own.insert(copy, card.clone());
                    self.table.insert(copy, card);
                }
            }
            for (i, u) in s.uncontrolled.iter().enumerate() {
                if let Some(copy) = u.card.copy {
                    let name = match u.card.id {
                        Some(_) => self.card(&u.card),
                        None => format!("uncontrolled vampire #{}", i + 1),
                    };
                    self.table.insert(copy, name);
                }
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

    /// The action under way, as the last view showed.
    pub fn action(&self) -> Option<&ActionView> {
        self.action.as_ref()
    }

    /// The referendum under way, as the last view showed.
    pub fn referendum(&self) -> Option<&ReferendumView> {
        self.referendum.as_ref()
    }

    /// The seat controlling a minion, as the last view showed.
    pub fn controller(&self, minion: CopyId) -> Option<Seat> {
        self.controllers.get(&minion).copied()
    }

    /// The minion a card is on, as the last view showed.
    pub fn host(&self, card: CopyId) -> Option<CopyId> {
        self.hosts.get(&card).copied()
    }

    pub fn prey(&self, s: Seat) -> Option<Seat> {
        self.prey.get(s as usize).copied()
    }

    /// A card on the table or a minion, by copy, as the table names it.
    pub fn copy(&self, c: CopyId) -> String {
        self.minions
            .get(&c)
            .or(self.table.get(&c))
            .cloned()
            .unwrap_or_else(|| "a card".into())
    }

    /// A minion as an option names it (a token without its article).
    pub fn minion_bare(&self, c: CopyId) -> String {
        let name = self.minion(c);
        match name.strip_prefix("a ghoul token") {
            Some("") => "ghoul token".into(),
            _ => name,
        }
    }

    /// A card on the table by its own name (not the minion it is on).
    pub fn own(&self, c: CopyId) -> String {
        self.own.get(&c).cloned().unwrap_or_else(|| self.copy(c))
    }

    /// A card or minion by copy as an option names it.
    pub fn copy_bare(&self, c: CopyId) -> String {
        match self.minions.contains_key(&c) {
            true => self.minion_bare(c),
            false => self.copy(c),
        }
    }

    /// A clan, a Discipline, a circle by its snake_case id, as printed.
    pub fn value(&self, v: &str) -> String {
        if let Some(name) = self.cards.value(v) {
            return name.into();
        }
        v.split('_')
            .map(|w| {
                let mut c = w.chars();
                c.next()
                    .map(|f| f.to_uppercase().chain(c).collect::<String>())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" ")
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
