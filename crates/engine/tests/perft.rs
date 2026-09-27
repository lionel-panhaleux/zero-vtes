//! Perft-style counts (DESIGN §7.5): the number of real-decision sequences
//! to a fixed depth from scripted positions, with a per-window breakdown
//! (decision nodes, options offered), pinned in `tests/golden/perft.txt`.
//! A legality regression changes an option list, hence a count; the
//! breakdown names the window. Single-option windows elide inside `run()`
//! and never branch; a game over before the depth is one leaf.
//! `GOLDEN_BLESS=1 cargo test --release --test perft` rewrites the pins.
//! The `midgame` positions are reached by random play from a seed: it is a
//! canary (any change in their first decisions moves the position), the
//! scripted ones localize.

use std::collections::HashMap;
use std::time::Instant;

use zvtes::agent::RandomLegal;
use zvtes::crypt::CRYPT;
use zvtes::decision::WindowLabel;
use zvtes::game::{Engine, Step};
use zvtes::ids::SeatId;
use zvtes::state::find_card;

mod common;
use common::*;

#[derive(Default)]
struct Count {
    /// Sequences of `depth` real decisions (or fewer, ending the game).
    leaves: u64,
    /// Options taken (`Engine::choose` calls): the work done.
    steps: u64,
    /// Per window: decision nodes, options offered.
    windows: HashMap<WindowLabel, (u64, u64)>,
}

fn perft(mut e: Engine, depth: u32, c: &mut Count) {
    if depth == 0 {
        c.leaves += 1;
        return;
    }
    let n = match e.run() {
        Step::Over(_) => {
            c.leaves += 1;
            return;
        }
        Step::Decision(req) => {
            let w = c.windows.entry(req.window).or_default();
            w.0 += 1;
            w.1 += req.options.len() as u64;
            assert!(!req.options.is_empty(), "empty request at {:?}", req.window);
            req.options.len()
        }
    };
    c.steps += n as u64;
    for i in 0..n - 1 {
        let mut b = e.clone();
        b.choose(i);
        perft(b, depth - 1, c);
    }
    e.choose(n - 1);
    perft(e, depth - 1, c);
}

/// The `nth` V5 vampire without printed text having every one of `discs`
/// (at either level).
fn having(discs: &[&str], nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            zvtes::cards::vampire(*i as u16).is_none()
                && discs
                    .iter()
                    .all(|d| c.disciplines.iter().any(|cd| cd.eq_ignore_ascii_case(d)))
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no plain vampire with {discs:?}"))
}

/// The rulebook opening, 4 seats.
fn opening(seed: u64) -> Engine {
    Engine::new(seed, 4)
}

/// Random play from the opening of `seed`, `n` real decisions in.
fn midgame(seed: u64, n: usize) -> Engine {
    let mut e = Engine::new(seed, 4);
    let mut agent = RandomLegal::new(seed);
    for _ in 0..n {
        let req = request(&mut e);
        let i = agent.choose(&req);
        e.choose(i);
    }
    e
}

/// Seat 0's minion phase, turn 4, 3 seats: stealth and bleed modifiers
/// against intercept, bounce and wake reactions.
fn action() -> Engine {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), having(&["obf", "dom"], 0), 5, false);
    e.add_minion(SeatId(1), having(&["aus", "ani"], 0), 4, false);
    e.add_minion(SeatId(2), having(&["dom", "aus"], 0), 4, false);
    hand(
        &mut e,
        0,
        &[
            "Cloak the Gathering",
            "Spying Mission",
            "Conditioning",
            "Seduction",
            "Threats",
        ],
    );
    hand(
        &mut e,
        1,
        &["Eagle's Sight", "Enhanced Senses", "Cats' Guidance"],
    );
    hand(
        &mut e,
        2,
        &[
            "Deflection",
            "Eyes of Argus",
            "Wake with Evening's Freshness",
        ],
    );
    e.state.turn = 4;
    e
}

/// The first combat decision of a bleed blocked by a PRE vampire: strikes,
/// maneuvers, presses, prevention and X terms on both sides.
fn combat() -> Engine {
    let mut e = Engine::bare(2);
    let a = e.add_minion(SeatId(0), vampire_with("POT", 0), 4, false);
    e.add_minion(SeatId(0), find_card("Farah Sarroub"), 4, false);
    e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Elisha Tucker"), 6, false);
    hand(
        &mut e,
        0,
        &["Immortal Grapple", "Roundhouse", "Torn Signpost", "Flash"],
    );
    hand(&mut e, 1, &["Majesty", "Flash", "Hidden Strength", "Soak"]);
    e.state.turn = 4;
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    e
}

/// A political action called: terms (a stepwise allocation), then polling
/// with a prince's votes, the calling card's, and vote cards.
fn referendum() -> Engine {
    let mut e = Engine::bare(3);
    let alexa = e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    e.add_minion(SeatId(0), find_card("Mateo García"), 2, false);
    e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
    e.add_minion(SeatId(1), find_card("Bret Stryker"), 4, false);
    e.add_minion(SeatId(2), find_card("Lenny Burkhead"), 4, false);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    hand(
        &mut e,
        0,
        &["Kine Resources Contested", "Bewitching Oration"],
    );
    hand(&mut e, 1, &["Scalpel Tongue"]);
    hand(&mut e, 2, &["Delaying Tactics"]);
    e.state.turn = 4;
    pick(&mut e, play(0, 0, alexa));
    e
}

/// Seat 0's master phase: masters played, cancelled (Sudden Reversal) and
/// answered out of turn.
fn masters() -> Engine {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), find_card("Abraham DuSable"), 5, false);
    e.add_minion(SeatId(0), find_card("Abraham Mellon"), 1, false);
    e.add_minion(SeatId(1), find_card("Abaddon"), 5, false);
    e.add_minion(SeatId(2), find_card("Alexa Draper"), 3, false);
    hand(
        &mut e,
        0,
        &[
            "Villein",
            "Blood Doll",
            "Vessel",
            "Dreams of the Sphinx",
            "Potence",
        ],
    );
    hand(&mut e, 1, &["Sudden Reversal"]);
    hand(&mut e, 2, &["Direct Intervention"]);
    e.state.turn = 4;
    e.at_master_phase();
    e
}

/// The pinned positions and depths.
fn positions() -> Vec<(&'static str, Engine, u32)> {
    vec![
        ("opening/1", opening(1), 5),
        ("midgame/3", midgame(3, 150), 6),
        ("midgame/5", midgame(5, 300), 4),
        ("action", action(), 7),
        ("combat", combat(), 7),
        ("referendum", referendum(), 6),
        ("masters", masters(), 6),
    ]
}

fn render(name: &str, depth: u32, c: &Count) -> String {
    let mut out = format!("{name} depth {depth}: {} leaves\n", c.leaves);
    let mut windows: Vec<(String, (u64, u64))> = c
        .windows
        .iter()
        .map(|(w, n)| (format!("{w:?}"), *n))
        .collect();
    windows.sort();
    for (w, (nodes, options)) in windows {
        out.push_str(&format!("  {w} {nodes} nodes {options} options\n"));
    }
    out
}

#[test]
fn perft_counts_match_the_pinned_ones() {
    let mut got = String::from("# position depth: leaves / window nodes options\n");
    for (name, e, depth) in positions() {
        let mut c = Count::default();
        perft(e, depth, &mut c);
        got.push_str(&render(name, depth, &c));
    }
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/perft.txt");
    let bless = "GOLDEN_BLESS=1 cargo test --release --test perft";
    if std::env::var("GOLDEN_BLESS").is_ok() {
        std::fs::write(path, &got).unwrap();
        return;
    }
    let Ok(want) = std::fs::read_to_string(path) else {
        panic!("no pinned perft counts {path}; re-bless: {bless}");
    };
    let mut position = "";
    for (g, w) in got.lines().zip(want.lines()) {
        if !g.starts_with(' ') {
            position = g;
        }
        assert!(
            g == w,
            "perft diverges under {position:?}\n  pinned: {w}\n  now:    {g}\n\
             if the legality change is intended, re-bless: {bless}"
        );
    }
    assert_eq!(
        got.lines().count(),
        want.lines().count(),
        "perft output length differs; re-bless: {bless}"
    );
}

/// The perft speed baseline (DESIGN §7.5), one depth deeper:
/// `cargo test --release --test perft -- --ignored --nocapture`.
#[test]
#[ignore]
fn perft_speed() {
    let (mut steps, mut secs) = (0, 0.0);
    for (name, e, depth) in positions() {
        let mut c = Count::default();
        let t = Instant::now();
        perft(e, depth + 1, &mut c);
        let dt = t.elapsed().as_secs_f64();
        eprintln!(
            "{name} depth {}: {} leaves, {} steps, {:.0} steps/s",
            depth + 1,
            c.leaves,
            c.steps,
            c.steps as f64 / dt
        );
        steps += c.steps;
        secs += dt;
    }
    eprintln!(
        "total: {steps} steps in {secs:.2}s, {:.0} steps/s",
        steps as f64 / secs
    );
}
