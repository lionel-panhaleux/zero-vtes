//! A game behind the contract (`docs/protocol.md`): `Game::handle` takes a
//! seat's command and returns the messages for it, the bots answered in
//! between; `Game::record` is the game as recorded. Transport-free: the
//! server and the wasm module are thin shells over it.

pub mod describe;
mod events;
mod game;
pub mod ids;
mod legacy;
pub mod numbering;
pub mod view;

pub use game::{Bots, Clock, Game, SystemClock, BUILT_IN};

use zvtes_api::message::SpecError;
use zvtes_api::DeckList;

/// The precon decklists the engine plays: (name, list).
pub fn catalog() -> Vec<(String, DeckList)> {
    zvtes_protocol::deck::catalog()
        .into_iter()
        .map(|(name, list)| {
            let list = list.into_iter().map(|(k, v)| (k, v as u32)).collect();
            (name.to_string(), list)
        })
        .collect()
}

/// Whether the engine can play a decklist (as seat 0).
pub fn check_deck(list: &DeckList) -> Result<(), SpecError> {
    let list = list
        .iter()
        .map(|(k, &v)| (k.clone(), v.min(u8::MAX as u32) as u8))
        .collect();
    zvtes_protocol::deck::resolve(&list)
        .map(|_| ())
        .map_err(|e| game::deck_error(0, e))
}

/// The engine's version: the commit this was built from.
pub const ENGINE: &str = env!("ZVTES_ENGINE");
