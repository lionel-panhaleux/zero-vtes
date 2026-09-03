//! zero-vtes P0 skeleton (DESIGN.md §11): decision-request engine, the two
//! window primitives, turn/phase machine, the card-free rulebook actions on
//! real V5 crypt cards (titles, contests), no library cards.
//! Replay = (seed, decision log).

pub mod agent;
pub mod crypt;
pub mod decision;
pub mod game;
pub mod ids;
pub mod library;
pub mod rng;
pub mod state;
pub mod window;
