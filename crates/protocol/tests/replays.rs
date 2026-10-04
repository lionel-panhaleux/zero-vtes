//! Replays by what was chosen (`zvtes_protocol::logged`): a game played by
//! bots, exported, reads back from its choices, from its indices alone (a
//! replay made before choices), and past a decision its engine asked that
//! a later one does not (`Logged::forced`).

use zvtes::decks::Deck;
use zvtes_protocol::deck::list_of;
use zvtes_protocol::logged::Logged;
use zvtes_protocol::{GameSpec, Out, Player, Replay, SeatSpec, Session};

fn spec(seed: u64) -> GameSpec {
    let decks: Vec<Deck> = Deck::all().collect();
    let styles = ["aggro_bleed", "wall", "random"];
    GameSpec {
        seed,
        time_limit: Some(30),
        seats: (0..4)
            .map(|i| SeatSpec {
                name: format!("P{}", i + 1),
                deck: list_of(&decks[(seed as usize * 5 + i * 3) % decks.len()]),
                player: Player::Bot {
                    style: styles[(seed as usize + i) % 3].into(),
                },
            })
            .collect(),
    }
}

/// A game played by bots to its end, as exported.
fn played(seed: u64) -> (Replay, Vec<f32>) {
    let mut s = Session::new(spec(seed)).expect("a valid spec");
    loop {
        if let Out::Over { vps, .. } = s.advance() {
            return (s.replay(), vps);
        }
    }
}

/// `replay` read back, every seat answered from it: its decisions'
/// indices, and the end.
fn read(replay: &Replay) -> Result<(Vec<usize>, Vec<f32>), String> {
    let mut spec = replay.spec.clone();
    for s in &mut spec.seats {
        s.player = Player::Human;
    }
    let mut s = Session::new(spec).expect("a valid spec");
    let mut reader = replay.reader();
    let mut log = Vec::new();
    loop {
        match s.advance() {
            Out::Decision { decision, .. } => {
                let req = s.request().expect("a decision is pending").clone();
                let i = reader
                    .next(s.engine(), &req)
                    .ok_or("the replay ends early")??;
                log.push(i);
                s.answer(decision, i).map_err(|e| format!("{e:?}"))?;
            }
            Out::Over { vps, .. } => return Ok((log, vps)),
            Out::Decided { .. } => unreachable!("every seat is answered"),
        }
    }
}

#[test]
fn a_game_reads_back_from_its_choices() {
    for seed in 0..6 {
        let (replay, vps) = played(seed);
        assert_eq!(replay.choices.len(), replay.log.len(), "seed {seed}");
        // The first option that is the choice: the logged index, or an
        // earlier copy alike of it — the same game either way.
        let (log, end) = read(&replay).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        assert_eq!(end, vps, "seed {seed}");
        assert_eq!(log.len(), replay.log.len(), "seed {seed}");
        // Through JSON, as the fork saves it.
        let json = serde_json::to_string(&replay).expect("a replay serializes");
        let back: Replay = serde_json::from_str(&json).expect("a replay");
        assert_eq!(back, replay, "seed {seed}");
    }
}

#[test]
fn a_replay_without_choices_reads_by_its_indices() {
    let (mut replay, vps) = played(1);
    let log = replay.log.clone();
    replay.choices.clear();
    let json = serde_json::to_string(&replay).expect("a replay serializes");
    assert!(
        !json.contains("choices"),
        "no choices written: the old format"
    );
    let (read_log, end) = read(&replay).expect("reads");
    assert_eq!((read_log, end), (log, vps.clone()));
    // Filled in from its indices, it reads by its choices.
    let (filled, stop) = replay.with_choices().expect("a valid spec");
    assert_eq!(stop, None);
    assert_eq!(read(&filled).expect("reads").1, vps);
}

#[test]
fn a_decision_this_engine_does_not_ask_is_passed_over() {
    let (mut replay, vps) = played(2);
    // A decision the replay's engine asked between two copies alike (a
    // discard), which this engine answers itself; and one a choice no
    // seat is offered, not forced: the game went another way.
    let at = replay.log.len() / 2;
    let seat = replay.choices[at].seat;
    let ghost = |forced| Logged {
        seat,
        choice: "Discard(0)".into(),
        cards: vec![0],
        from: None,
        forced,
    };
    let mut forced = replay.clone();
    forced.choices.insert(at, ghost(true));
    forced.log.insert(at, 0);
    assert_eq!(read(&forced).expect("reads").1, vps);
    replay.choices.insert(at, ghost(false));
    replay.log.insert(at, 0);
    let err = read(&replay).expect_err("an unknown choice stops the replay");
    assert!(err.contains("Discard(0)"), "{err}");
}

/// One of the owner's games (`data/replays`), read whole: a canary for a
/// change to how choices are written (the `Choice` debug output) — or a
/// rules fix this game met (the README then says where it stops).
#[test]
fn an_owner_game_reads_whole() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/replays/zero-vtes-replay-2026-10-04T06-45-40-714Z.json"
    );
    let replay: Replay = serde_json::from_str(&std::fs::read_to_string(path).expect("the replay"))
        .expect("a replay");
    let (log, vps) = read(&replay).expect("reads whole");
    assert_eq!(log.len(), replay.log.len());
    assert_eq!(vps, vec![4.0, 0.0, 0.0, 0.0]);
}
