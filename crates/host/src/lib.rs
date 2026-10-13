//! A game behind the contract (`docs/protocol.md`): `Game::handle` takes a
//! seat's command and returns the messages for it, the bots answered in
//! between; `Game::record` is the game as recorded. Transport-free: the
//! server and the wasm module are thin shells over it.

pub mod cards;
pub mod deck;
pub mod describe;
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

pub use game::{Bots, Clock, Game, SystemClock, BUILT_IN};

use zvtes_api::message::SpecError;
use zvtes_api::DeckList;

/// The precon decklists the engine plays: (name, list).
pub fn catalog() -> Vec<(String, DeckList)> {
    deck::catalog()
}

/// Whether the engine can play a decklist (as seat 0).
pub fn check_deck(list: &DeckList) -> Result<(), SpecError> {
    deck::resolve(0, list).map(|_| ())
}

/// The engine's version: the commit this was built from.
pub const ENGINE: &str = env!("ZVTES_ENGINE");
