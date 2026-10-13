//! The engine's card names and clans for the English renderer
//! (`zvtes_text::Cards`): what a client reads from its card database.

use std::collections::HashMap;
use std::sync::OnceLock;

use zvtes::crypt::CRYPT;
use zvtes::library::LIBRARY;

/// Card names by KRCG id (as printed), clan names by snake_case id.
pub struct EngineCards(HashMap<u32, String>, HashMap<String, &'static str>);

/// A card's name as printed: KRCG sorts "The Gate of Acheron" as "Gate of
/// Acheron, The" (only a trailing ", The": "Smiling Jack, The Anarch" is
/// printed so).
pub fn printed(name: &str) -> String {
    match name.strip_suffix(", The") {
        Some(rest) => format!("The {rest}"),
        None => name.to_string(),
    }
}

impl zvtes_text::Cards for EngineCards {
    fn name(&self, id: u32) -> Option<&str> {
        self.0.get(&id).map(String::as_str)
    }
    fn value(&self, id: &str) -> Option<&str> {
        self.1.get(id).copied()
    }
}

/// The engine's tables, built once.
pub fn cards() -> &'static EngineCards {
    static CARDS: OnceLock<EngineCards> = OnceLock::new();
    CARDS.get_or_init(|| {
        let crypt = CRYPT.iter().map(|c| (c.id, printed(c.name)));
        let library = LIBRARY.iter().map(|c| (c.id, printed(c.name)));
        let clans = CRYPT.iter().map(|c| (crate::view::snake(c.clan), c.clan));
        EngineCards(crypt.chain(library).collect(), clans.collect())
    })
}
