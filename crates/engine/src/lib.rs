//! zero-vtes engine (DESIGN.md): decision-request engine, the two window
//! primitives, turn/phase machine, the card-free rulebook actions on real V5
//! crypt cards (titles, contests), library zones and card plays (P1).
//! Replay = (seed, decision log).

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
pub mod cards;
pub mod combat;
pub mod contests;
pub mod crypt;
pub mod decision;
pub mod draws;
pub mod game;
pub mod ids;
pub mod influence;
pub mod library;
pub mod plays;
pub mod referendum;
pub mod rng;
pub mod state;
pub mod window;
