//! Agents: whatever answers the engine's decision requests (DESIGN §4.1).
//! `RandomLegal` is the fuzzing driver; the scripted reference bots are in
//! `bots`.

use crate::decision::DecisionRequest;
use crate::game::{Engine, Step};
use crate::rng::Rng;

/// A policy: the index of the option to take. It may read the engine,
/// but only what its seat's legal view shows (DESIGN §5): the bots'
/// honesty is tested by answering the same request from the redacted
/// state.
pub trait Agent {
    fn decide(&mut self, engine: &Engine, req: &DecisionRequest) -> usize;
}

/// Picks uniformly among the offered options — the P0 fuzzing driver.
#[derive(Clone)]
pub struct RandomLegal {
    pub rng: Rng,
}

impl RandomLegal {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Rng::new(seed),
        }
    }

    pub fn choose(&mut self, req: &DecisionRequest) -> usize {
        self.rng.below(req.options.len())
    }
}

impl Agent for RandomLegal {
    fn decide(&mut self, _: &Engine, req: &DecisionRequest) -> usize {
        self.choose(req)
    }
}

/// Play a game to its end, each seat answered by its agent; the final
/// VPs per seat, in half-point units.
pub fn play_out(engine: &mut Engine, agents: &mut [Box<dyn Agent>]) -> Vec<u32> {
    loop {
        match engine.run() {
            Step::Decision(req) => {
                let i = agents[req.seat.index()].decide(engine, &req);
                engine.choose(i);
            }
            Step::Over(vps) => return vps,
        }
    }
}
