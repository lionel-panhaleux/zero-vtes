//! Games from positions of recorded games (owner, 2026-10-04, road 2 of
//! the replay review: the network never meets the boards a rescue, a
//! diablerie or a burned drain pays on, so it never learns them). A start
//! replays one of the games (`data/records`) to a decision and hands every
//! seat to its player from there (`zvtes_py`'s pool: training tables
//! queued with a `start`).
//!
//! The replay fixes every library and crypt order: from one position,
//! every game would draw the same cards, a future to memorize. So at the
//! branch each seat's library and crypt are shuffled again from the
//! pool's seed; hands, the uncontrolled region and the table stay as they
//! are.

use zvtes::game::Engine;
use zvtes::rng::Rng;
use zvtes_api::record::GameRecord;

use crate::replay::Replay;

/// A game to start from: its deal, and how far this engine reads it.
pub struct Start {
    replay: Replay,
    /// The decisions this engine reads from it (to the record's end, a
    /// rules change since, or the game's end).
    pub readable: usize,
}

impl Start {
    /// A game record (JSON) as a start.
    pub fn new(json: &str) -> Result<Start, String> {
        let record: GameRecord = serde_json::from_str(json).map_err(|e| format!("record: {e}"))?;
        let replay = Replay::new(&record)?;
        if !matches!(replay.seats(), 2 | 4 | 5) {
            return Err("2, 4 or 5 seats (tournament tables, the two-player variant)".into());
        }
        let mut start = Start {
            replay,
            readable: 0,
        };
        start.readable = start.read(usize::MAX).1;
        Ok(start)
    }

    pub fn seats(&self) -> usize {
        self.replay.seats()
    }

    /// The game replayed to decision `at` (or as far as it reads), and the
    /// decisions read.
    fn read(&self, at: usize) -> (Engine, usize) {
        self.replay.read(at)
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
