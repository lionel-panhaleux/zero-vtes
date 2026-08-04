//! Random-legal fuzzing with invariants and replay determinism (DESIGN §7):
//! every game terminates within a decision budget, invariants hold at every
//! decision point, and (seed, decision log) resimulation reproduces the exact
//! final state hash.

use zvtes::agent::RandomLegal;
use zvtes::game::{Engine, Step};

const SEEDS: u64 = 100;
const DECISION_BUDGET: u64 = 200_000;

fn invariants(e: &Engine, seed: u64) {
    let st = &e.state;
    assert!(
        !st.seats[st.active.index()].ousted,
        "active seat is ousted (seed {seed})"
    );
    let mut edges = 0;
    for (i, s) in st.seats.iter().enumerate() {
        assert!(s.pool >= 0, "negative pool at seat {i} (seed {seed})");
        if s.ousted {
            assert_eq!(s.pool, 0, "ousted seat {i} kept pool (seed {seed})");
            assert!(!s.has_edge, "ousted seat {i} kept the Edge (seed {seed})");
        }
        if s.has_edge {
            edges += 1;
        }
        for slot in &s.uncontrolled {
            assert!(
                slot.counters <= slot.capacity,
                "overfilled uncontrolled slot (seed {seed})"
            );
        }
    }
    assert!(edges <= 1, "multiple Edges (seed {seed})");
    for m in &st.minions {
        assert!(m.blood <= m.capacity, "blood over capacity (seed {seed})");
        if m.in_play {
            assert!(
                !st.seats[m.owner.index()].ousted,
                "minion of an ousted seat in play (seed {seed})"
            );
        }
    }
    let vps: u32 = st.seats.iter().map(|s| s.vps).sum();
    assert!(
        vps <= st.seats.len() as u32,
        "more VPs than seats (seed {seed})"
    );
}

fn play(seed: u64) -> (Engine, Vec<u32>) {
    let mut e = Engine::new(seed, 4);
    let mut agent = RandomLegal::new(seed ^ 0x00c0_ffee);
    loop {
        assert!(
            e.decisions < DECISION_BUDGET,
            "decision budget exhausted (seed {seed})"
        );
        match e.run() {
            Step::Decision(req) => {
                invariants(&e, seed);
                let idx = agent.choose(&req);
                e.choose(idx);
            }
            Step::Over(vps) => return (e, vps),
        }
    }
}

#[test]
fn random_games_terminate_hold_invariants_and_replay() {
    for seed in 0..SEEDS {
        let (played, vps) = play(seed);
        let hash = played.state.hash();

        // Replay: the recorded decision log reproduces the identical game.
        let mut replayed = Engine::new(seed, 4);
        let mut cursor = 0;
        let replayed_vps = loop {
            match replayed.run() {
                Step::Decision(_) => {
                    replayed.choose(played.log[cursor]);
                    cursor += 1;
                }
                Step::Over(v) => break v,
            }
        };
        assert_eq!(
            cursor,
            played.log.len(),
            "log length mismatch (seed {seed})"
        );
        assert_eq!(replayed_vps, vps, "VP mismatch on replay (seed {seed})");
        assert_eq!(
            replayed.state.hash(),
            hash,
            "state hash mismatch on replay (seed {seed})"
        );
    }
}
