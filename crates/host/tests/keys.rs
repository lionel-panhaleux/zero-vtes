//! Every option of every decision has its own key (docs/protocol.md §7):
//! what lets a record read by choice on a later engine. Random games on
//! the shipped decks, every seat answered at random, keys by the engine's
//! copies (as records name them).

use std::collections::HashMap;

use zvtes::agent::RandomLegal;
use zvtes::game::{Engine, Step};
use zvtes_host::describe::{key, EngineNames};

fn engine(seed: u64) -> Engine {
    let decks = zvtes_protocol::deck::catalog();
    let resolved: Vec<_> = (0..4)
        .map(|i| {
            zvtes_protocol::deck::resolve(&decks[(seed as usize + i * 3) % decks.len()].1)
                .expect("a shipped deck")
        })
        .collect();
    let lists: Vec<_> = resolved
        .iter()
        .map(|d| (d.crypt.as_slice(), d.library.as_slice()))
        .collect();
    Engine::with_lists(seed, &lists)
}

#[test]
fn two_options_of_a_decision_never_share_a_key() {
    let seeds: u64 = std::env::var("HOST_SEEDS").map_or(60, |s| s.parse().unwrap());
    let mut decisions = 0usize;
    let mut by_kind: HashMap<String, usize> = HashMap::new();
    let mut twice: HashMap<String, u64> = HashMap::new();
    for seed in 1..=seeds {
        let mut e = engine(seed);
        let mut agent = RandomLegal::new(seed);
        for _ in 0..20_000 {
            match e.run() {
                Step::Over(_) => break,
                Step::Decision(req) => {
                    decisions += 1;
                    let keys: Vec<_> = req
                        .options
                        .iter()
                        .map(|c| key(&e, &req, c, &EngineNames))
                        .collect();
                    for (i, a) in keys.iter().enumerate() {
                        *by_kind.entry(format!("{:?}", a.kind)).or_default() += 1;
                        for (j, b) in keys.iter().enumerate().skip(i + 1) {
                            if req.options[i] == req.options[j] {
                                // The engine offering one choice twice: not
                                // the key's to tell apart (reported).
                                let name = |c: &zvtes::decision::Choice| match c {
                                    zvtes::decision::Choice::UseAbility {
                                        source: zvtes::cards::AbilitySource::Vampire(m),
                                        ..
                                    } => e.state.minions[m.index()].name().to_string(),
                                    other => format!("{other:?}"),
                                };
                                twice
                                    .entry(format!("{:?} {}", req.window, name(&req.options[i])))
                                    .or_insert(seed);
                                continue;
                            }
                            assert!(
                                a != b,
                                "seed {seed}, {:?}: options {i} {:?} and {j} {:?} share {a:?}",
                                req.window,
                                req.options[i],
                                req.options[j]
                            );
                        }
                    }
                    let pick = agent.choose(&req);
                    e.choose(pick);
                }
            }
        }
    }
    assert!(decisions > 10_000, "{decisions} decisions");
    eprintln!("{decisions} decisions, {} option kinds", by_kind.len());
    for (what, seed) in &twice {
        eprintln!("offered twice: {what} (first at seed {seed})");
    }
}
