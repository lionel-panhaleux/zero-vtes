use crate::decision::DecisionRequest;
use crate::rng::Rng;

/// Picks uniformly among the offered options — the P0 fuzzing driver.
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
