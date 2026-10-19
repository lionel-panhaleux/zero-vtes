//! zero-vtes engine (DESIGN.md, docs/engine.md): a decision-request state
//! machine enforcing the VTES rules on the V5 card pool. Replay = (seed,
//! decision log).

/// The sum of the `n` of every `$v(n)` modifier among the layers `$layers`
/// (the others count 0).
macro_rules! sum_n {
    ($layers:expr, $v:path) => {
        $layers
            .map(|l| match l.modifier {
                $v(n) => n,
                _ => 0,
            })
            .sum()
    };
}

pub mod actions;
pub mod agent;
pub mod bots;
pub mod cards;
pub mod combat;
pub mod contests;
pub mod copies;
pub mod crypt;
pub mod decision;
pub mod decks;
pub mod draws;
pub mod game;
pub mod habits;
pub mod ids;
pub mod influence;
pub mod library;
pub mod notes;
pub mod obs;
pub mod ousting;
pub mod plays;
pub mod politics;
pub mod referendum;
pub mod rng;
pub mod state;
pub mod twda;
pub mod window;
