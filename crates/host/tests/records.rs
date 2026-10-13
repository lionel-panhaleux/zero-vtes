//! The corpus of human games (`data/records`) reads whole on this engine,
//! each decision by its key, told (every option in English).

use zvtes_api::record::GameRecord;
use zvtes_host::replay::{Replayer, Step};

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
        let decisions = record
            .entries
            .iter()
            .filter(|e| matches!(e, zvtes_api::record::Entry::Decision { .. }))
            .count();
        assert_eq!(read, decisions, "{path:?}");
        assert_eq!(
            over,
            record.result.as_ref().map(|r| r.vp_halves.clone()),
            "{path:?}"
        );
    }
}
