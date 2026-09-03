//! zero-vtes engine (DESIGN.md): decision-request engine, the two window
//! primitives, turn/phase machine, the card-free rulebook actions on real V5
//! crypt cards (titles, contests), library zones and card plays (P1).
//! Replay = (seed, decision log).

pub mod actions;
pub mod agent;
pub mod contests;
pub mod crypt;
pub mod decision;
pub mod game;
pub mod ids;
pub mod influence;
pub mod library;
pub mod referendum;
pub mod rng;
pub mod state;
pub mod window;
