//! Games from positions of the owner's games (owner, 2026-10-04,
//! road 2 of the replay review: the network never meets the boards a
//! rescue, a diablerie or a burned drain pays on, so it never learns
//! them). A start replays one of the games (`data/replays`) to a decision
//! and hands every seat to its player from there (`zvtes_py`'s pool:
//! training tables queued with a `start`).
//!
//! The replay fixes every library and crypt order: from one position,
//! every game would draw the same cards, a future to memorize. So at the
//! branch each seat's library and crypt are shuffled again from the
//! pool's seed; hands, the uncontrolled region and the table stay as they
//! are.

use crate::deck::{resolve, Resolved};
use crate::Replay;
use zvtes::game::{Engine, Step};
use zvtes::rng::Rng;

/// A game to start from: its deal, and how far this engine reads it.
pub struct Start {
    replay: Replay,
    decks: Vec<Resolved>,
    /// The decisions this engine reads from it (to the log's end, a rules
    /// change since, or the game's end).
    pub readable: usize,
}

impl Start {
    /// A replay (the fork's JSON) as a start.
    pub fn new(json: &str) -> Result<Start, String> {
        let replay: Replay = serde_json::from_str(json).map_err(|e| format!("replay: {e}"))?;
        let decks = replay
            .spec
            .seats
            .iter()
            .map(|s| resolve(&s.deck))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("{e:?}"))?;
        if !(4..=5).contains(&decks.len()) {
            return Err("4 or 5 seats (tournament tables)".into());
        }
        let mut start = Start {
            replay,
            decks,
            readable: 0,
        };
        start.readable = start.read(usize::MAX).1;
        Ok(start)
    }

    pub fn seats(&self) -> usize {
        self.decks.len()
    }

    /// The game replayed to decision `at` (or as far as it reads), and the
    /// decisions read.
    fn read(&self, at: usize) -> (Engine, usize) {
        let lists: Vec<_> = self
            .decks
            .iter()
            .map(|d| (d.crypt.as_slice(), d.library.as_slice()))
            .collect();
        let mut e = Engine::with_lists(self.replay.spec.seed, &lists);
        if let Some(t) = self.replay.spec.time_limit {
            e.time_limit = t;
        }
        let mut reader = self.replay.reader();
        let mut n = 0;
        while n < at {
            let Step::Decision(req) = e.run() else { break };
            match reader.next(&e, &req) {
                Some(Ok(i)) => e.choose(i),
                _ => break,
            }
            n += 1;
        }
        (e, n)
    }

    /// A position: the game at decision `at` (under `readable`), its
    /// libraries and crypts shuffled from `seed`, and its time limit at
    /// least `time_limit`, the turn it is at plus 20.
    pub fn position(&self, at: usize, seed: u64, time_limit: u32) -> Engine {
        let (mut e, _) = self.read(at);
        let mut rng = Rng::new(seed);
        for s in &mut e.state.seats {
            s.library.shuffle(&mut rng);
            s.crypt.shuffle(&mut rng);
        }
        e.time_limit = time_limit.max(e.state.turn + 20);
        e
    }
}
