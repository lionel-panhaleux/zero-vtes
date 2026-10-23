//! A game behind the contract (`docs/protocol.md`): `Game::handle` takes a
//! seat's command and returns the messages for it, the bots answered in
//! between; `Game::record` is the game as recorded. Transport-free: the
//! server is a thin shell over it.

pub mod cards;
pub mod deck;
pub mod describe;
pub mod digest;
mod effects;
mod events;
mod game;
pub mod ids;
pub mod narrate;
pub mod numbering;
pub mod replay;
pub mod session;
pub mod start;
pub mod view;

#[cfg(test)]
mod passes_told;
#[cfg(test)]
mod targets_told;
#[cfg(test)]
mod terms_told;

pub use game::{Bots, Clock, Game, SystemClock, BUILT_IN};

use zvtes_api::DeckList;

/// The precon decklists the engine plays: (name, list).
pub fn catalog() -> Vec<(String, DeckList)> {
    deck::catalog()
}

/// VEKN's suggested two-player decklists: (name, list).
pub fn two_player_catalog() -> Vec<(String, DeckList)> {
    deck::two_player_catalog()
}

/// The engine's version: the commit this was built from.
pub const ENGINE: &str = env!("ZVTES_ENGINE");
