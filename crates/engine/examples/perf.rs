//! The engine's speed against DESIGN §9's targets: random 4-seat games of
//! the precons, every seat a random legal agent, played to their end (most
//! to the turn limit). `cargo run -p zero-vtes-engine --release --example
//! perf -- [games] [batches]` (100 games, 5 batches by default): the
//! fastest batch's milliseconds a game and decisions a second, the least
//! disturbed by other work on the machine (decisions: those asked; the
//! elided ones are applied within them).

use std::time::Instant;

use zvtes::agent::{play_out, Agent, RandomLegal};
use zvtes::game::Engine;

fn main() {
    let mut args = std::env::args().skip(1).map(|a| a.parse().ok());
    let games: u64 = args.next().flatten().unwrap_or(100);
    let batches: u64 = args.next().flatten().unwrap_or(5);
    let (mut best, mut decisions, mut turns) = (f64::MAX, 0u64, 0u64);
    for _ in 0..batches {
        let (mut d, mut t) = (0u64, 0u64);
        let start = Instant::now();
        for seed in 0..games {
            let mut e = Engine::new(seed, 4);
            let mut agents: Vec<Box<dyn Agent>> = (0..4)
                .map(|s| Box::new(RandomLegal::new(seed * 4 + s)) as Box<dyn Agent>)
                .collect();
            play_out(&mut e, &mut agents);
            d += e.log.len() as u64;
            t += e.state.turn as u64;
        }
        best = best.min(start.elapsed().as_secs_f64());
        (decisions, turns) = (d, t);
    }
    println!(
        "{games} games a batch, {batches} batches: {:.1} turns and {:.0} decisions a game",
        turns as f64 / games as f64,
        decisions as f64 / games as f64
    );
    println!(
        "fastest batch: {:.2} ms a game, {:.0} decisions a second ({:.2} µs each)",
        best * 1e3 / games as f64,
        decisions as f64 / best,
        best * 1e6 / decisions as f64
    );
}
