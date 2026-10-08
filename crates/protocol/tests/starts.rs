//! Starts (`zvtes_protocol::start`): one of the owner's games replayed to a
//! decision, its libraries and crypts shuffled again, played on from there.

use zvtes::agent::{play_out, Agent, RandomLegal};
use zvtes::game::{Engine, Step};
use zvtes_protocol::start::Start;

fn owner_game() -> Start {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/replays/zero-vtes-replay-2026-10-04T12-09-51-567Z.json"
    );
    Start::new(&std::fs::read_to_string(path).expect("the replay")).expect("a start")
}

fn sorted(mut v: Vec<u16>) -> Vec<u16> {
    v.sort_unstable();
    v
}

#[test]
fn a_start_reads_the_whole_game() {
    let start = owner_game();
    assert_eq!(start.seats(), 4);
    assert_eq!(start.readable, 369);
}

/// From one position, games draw different cards (the replay's order is
/// not kept), from the same hands, board and cards left.
#[test]
fn a_position_keeps_the_table_and_shuffles_the_decks() {
    let start = owner_game();
    let a = start.position(200, 1, 60);
    let b = start.position(200, 2, 60);
    let mut differ = false;
    for (x, y) in a.state.seats.iter().zip(&b.state.seats) {
        assert_eq!(x.hand, y.hand);
        assert_eq!(x.pool, y.pool);
        assert_eq!(sorted(x.library.clone()), sorted(y.library.clone()));
        assert_eq!(sorted(x.crypt.clone()), sorted(y.crypt.clone()));
        differ |= x.library != y.library || x.crypt != y.crypt;
    }
    assert!(differ, "seeds 1 and 2 shuffle alike");
    assert_eq!(a.state.turn, b.state.turn);
    assert!(a.time_limit >= a.state.turn + 20);
    // The same seed, the same position.
    let c = start.position(200, 1, 60);
    for (x, y) in a.state.seats.iter().zip(&c.state.seats) {
        assert_eq!(x.library, y.library);
    }
}

#[test]
fn a_position_plays_on_to_the_end() {
    let start = owner_game();
    for (k, at) in [80, 200, 360].into_iter().enumerate() {
        let mut e: Engine = start.position(at, k as u64, 90);
        assert!(matches!(e.run(), Step::Decision(_)), "decision {at}");
        let mut agents: Vec<Box<dyn Agent>> = (0..4)
            .map(|s| Box::new(RandomLegal::new(s)) as Box<dyn Agent>)
            .collect();
        let vps = play_out(&mut e, &mut agents);
        assert_eq!(vps.len(), 4);
    }
}

/// Every decision of whole games is a position that plays on: none lands
/// inside an effect the reshuffled library or crypt would break.
#[test]
fn every_position_of_whole_games_plays_on() {
    for file in [
        "zero-vtes-replay-2026-10-04T12-09-51-567Z.json",
        "zero-vtes-replay-2026-10-04T13-51-13-838Z.json",
    ] {
        let path = format!("{}/../../data/replays/{file}", env!("CARGO_MANIFEST_DIR"));
        let start =
            Start::new(&std::fs::read_to_string(path).expect("the replay")).expect("a start");
        for at in 0..start.readable {
            let mut e = start.position(at, at as u64, 60);
            for k in 0..200 {
                match e.run() {
                    Step::Decision(req) => {
                        assert!(
                            !req.options.is_empty(),
                            "{file} {at}: empty {:?}",
                            req.window
                        );
                        e.choose((at + k) % req.options.len());
                    }
                    Step::Over(_) => break,
                }
            }
        }
    }
}
