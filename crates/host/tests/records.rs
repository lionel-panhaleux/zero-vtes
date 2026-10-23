//! The corpus of human games (`data/records`) reads whole on this engine,
//! each decision by its key, told (every option in English).

use zvtes_api::record::GameRecord;
use zvtes_host::replay::{Replay, Replayer, Step};

#[test]
fn every_record_reads_whole() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/records");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no records in {dir}");
    for path in &files {
        let record: GameRecord =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let mut r = Replayer::new(&record).unwrap();
        let mut read = 0;
        let over = loop {
            match r.step() {
                Step::Decision(d) => {
                    assert!(
                        d.options.iter().all(|t| !t.is_empty()),
                        "{path:?}: an option untold at {}",
                        d.decision
                    );
                    if d.chosen.is_none() {
                        break None;
                    }
                    read += 1;
                    r.take();
                }
                Step::Over { vp_halves, .. } => break Some(vp_halves),
                Step::Stopped { why, .. } => panic!("{path:?}: stops after {read}: {why}"),
            }
        };
        // The recorded decisions an undo did not take back.
        let decisions = Replay::new(&record).unwrap().decisions.len();
        // Each of them read: taken, or a pass the block window an engine
        // before rules-qa 82.1 reopened recorded, read past.
        assert!(read <= decisions, "{path:?}");
        assert_eq!(r.position(), decisions, "{path:?}");
        assert_eq!(
            over,
            record.result.as_ref().map(|r| r.vp_halves.clone()),
            "{path:?}"
        );
    }
}

/// A record written by a later host reads on this one (docs/protocol.md
/// §14): an entry type, an end reason and a refusal it does not know read
/// as `unknown`, and the game replays whole past them.
#[test]
fn a_later_hosts_record_reads() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/records/zero-vtes-replay-2026-10-04T12-09-51-567Z.json"
    );
    let mut v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let entries = v["entries"].as_array_mut().unwrap();
    entries.insert(
        3,
        serde_json::json!({"type": "from_the_future", "seat": 0, "note": 1}),
    );
    entries.insert(
        5,
        serde_json::json!({"type": "refused", "seat": 0, "at": 0, "error": {"error": "brand_new", "x": 2}}),
    );
    entries.insert(
        6,
        serde_json::json!({"type": "refused", "seat": 0, "at": 0, "error": {"error": "allocation", "reason": "brand_new"}}),
    );
    // A decision answered by a later host's kind of answerer.
    let at = entries
        .iter()
        .position(|e| e["type"] == "decision")
        .unwrap();
    entries[at]["by"] = serde_json::json!("oracle");
    v["result"]["reason"] = serde_json::json!({"type": "comet_strike", "seat": 2});
    let record: GameRecord = serde_json::from_value(v).expect("a later host's record parses");
    assert!(record.entries.contains(&zvtes_api::record::Entry::Unknown));
    assert!(record.entries.iter().any(|e| matches!(
        e,
        zvtes_api::record::Entry::Refused {
            error: zvtes_api::message::Error::Unknown,
            ..
        }
    )));
    assert_eq!(
        record.result.as_ref().unwrap().reason,
        zvtes_api::message::EndReason::Unknown
    );
    assert!(record.entries.iter().any(|e| matches!(
        e,
        zvtes_api::record::Entry::Decision {
            by: zvtes_api::record::By::Unknown,
            ..
        }
    )));
    assert!(record.entries.iter().any(|e| matches!(
        e,
        zvtes_api::record::Entry::Refused {
            error: zvtes_api::message::Error::Allocation {
                reason: zvtes_api::message::AllocationError::Unknown
            },
            ..
        }
    )));
    let replay = zvtes_host::replay::Replay::new(&record).unwrap();
    let decisions = record
        .entries
        .iter()
        .filter(|e| matches!(e, zvtes_api::record::Entry::Decision { .. }))
        .count();
    // Consumed: the passes a reopened block window recorded (before
    // rules-qa 82.1) are read past, not taken.
    assert_eq!(
        replay.read_counting(usize::MAX).2,
        decisions,
        "read past the unknown"
    );
}

/// A key written before its kind's fields grew (docs/protocol.md §11: a
/// bleed's before it named its seat, a transfer's before its `slot`, a
/// strike's before its `actor`) reads by the fields it has: a record from
/// before 2.5.0, its indices made useless, replays whole by its keys.
#[test]
fn an_older_key_reads_by_the_fields_it_has() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/records/zero-vtes-replay-2026-10-04T12-09-51-567Z.json"
    );
    let mut v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut bleeds = 0;
    for e in v["entries"].as_array_mut().unwrap() {
        if e["type"] == "decision" {
            e["option"] = serde_json::json!(9999);
            bleeds += usize::from(e["choice"]["action"] == "bleed");
        }
    }
    assert!(bleeds > 0, "no bleed recorded");
    let record: GameRecord = serde_json::from_value(v).unwrap();
    let mut r = Replayer::new(&record).unwrap();
    let mut read = 0;
    loop {
        match r.step() {
            Step::Decision(d) if d.chosen.is_some() => {
                read += 1;
                r.take();
            }
            Step::Decision(_) | Step::Over { .. } => break,
            Step::Stopped { why, .. } => panic!("stops after {read}: {why}"),
        }
    }
    assert!(read > 100, "{read} read");
}

/// The corpus's 2.4.0 record carries its decisions' position digests and
/// the replay checks them, so `every_record_reads_whole` stops an engine
/// change that moves its hidden state, not only one that changes its
/// options (the mechanism: `tests/game.rs`,
/// `a_record_stops_where_its_position_differs`).
#[test]
fn the_corpus_digests_are_checked() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/records/d8ec2c01edab010d.json"
    );
    let mut v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let record: GameRecord = serde_json::from_value(v.clone()).unwrap();
    let kept = Replay::new(&record).unwrap().decisions;
    assert!(
        kept.iter().all(|d| d.digest.is_some()),
        "every decision has its digest"
    );
    // A decision past the undos, as the replay reads them.
    let at = kept.len() - 20;
    v["entries"][kept[at].entry]["digest"] = serde_json::json!("0000000000000000");
    let moved: GameRecord = serde_json::from_value(v).unwrap();
    let mut r = Replayer::new(&moved).unwrap();
    let why = loop {
        match r.step() {
            Step::Decision(d) => {
                assert!(d.chosen.is_some(), "the game reads to its end otherwise");
                r.take();
            }
            Step::Over { .. } => panic!("a moved position read whole"),
            Step::Stopped { why, .. } => break why,
        }
    };
    assert_eq!(
        r.position(),
        at,
        "stopped at the decision whose digest differs"
    );
    assert_eq!(
        why,
        format!(
            "decision {}: the position differs from the record's",
            kept[at].decision
        )
    );
}
