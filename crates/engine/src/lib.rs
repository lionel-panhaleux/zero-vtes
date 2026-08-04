//! zero-vtes P0 skeleton (DESIGN.md §11): decision-request engine, the two
//! window primitives, turn/phase machine, vanilla actions, no cards.
//! Replay = (seed, decision log).

pub mod agent;
pub mod decision;
pub mod game;
pub mod ids;
pub mod rng;
pub mod state;
pub mod window;
