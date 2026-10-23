//! The seats' context (`zvtes_env::context`, DESIGN §5 "The seats'
//! context"): the outcomes a seat's profile counts (a bleed's pool, a
//! block that lands), and the Disciplines' fixed order. The public-only
//! reading and the choices counted are checked in `tests/obs.rs`.

#[path = "../../engine/tests/common/mod.rs"]
mod common;

use common::*;
use zvtes::game::Engine;
use zvtes::ids::SeatId;
use zvtes::profile::Count;
use zvtes::state::find_card;
use zvtes_env::context::{self, DISCIPLINES, LEN, PER_SEAT, POSITION, SLOTS};

/// The profile's `k` for the seat `rel` seats after the viewer.
fn slot(ctx: &[f32], rel: usize, k: Count) -> f32 {
    ctx[(1 + POSITION.len() + DISCIPLINES.len() + k as usize) * SLOTS + rel]
}

/// The features of the seat `rel` seats after the viewer.
fn seat(ctx: &[f32], rel: usize) -> Vec<f32> {
    (0..PER_SEAT).map(|f| ctx[f * SLOTS + rel]).collect()
}

/// The context's names as p16 first trained on them: a trained network
/// reads its columns by position, so these keep their order and a new
/// feature comes after them.
const NAMES: [&str; 60] = [
    "present",
    "pool",
    "ousted",
    "vps",
    "ready",
    "unlocked",
    "ready_blood",
    "torpor",
    "hand",
    "library",
    "edge",
    "lunge",
    "lunged",
    "prey_margin",
    "ready_capacity",
    "ready_equipment",
    "discipline:obf",
    "discipline:pre",
    "discipline:dom",
    "discipline:pot",
    "discipline:cel",
    "discipline:for",
    "discipline:tha",
    "discipline:qui",
    "discipline:aus",
    "discipline:pro",
    "discipline:obl",
    "discipline:ani",
    "discipline:chi",
    "discipline:dem",
    "discipline:abo",
    "discipline:nec",
    "discipline:obe",
    "discipline:vic",
    "discipline:obt",
    "discipline:san",
    "discipline:spi",
    "discipline:dai",
    "discipline:ser",
    "discipline:vis",
    "discipline:myt",
    "plays_action",
    "plays_modifier",
    "plays_reaction",
    "plays_combat",
    "plays_political",
    "plays_master",
    "plays_other",
    "bleeds",
    "hunts",
    "political_actions",
    "rushes",
    "builds",
    "other_actions",
    "blocks",
    "strikes",
    "blocks_landed",
    "pool_bled",
    "stealth_shown",
    "intercept_shown",
];

#[test]
fn the_names_keep_their_order() {
    let names = context::names();
    assert_eq!(names.len() * SLOTS, LEN);
    assert_eq!(
        names[..NAMES.len()],
        NAMES.map(String::from),
        "append, never insert"
    );
    // A Discipline the pool gains is a new feature at the end, not an
    // insertion into the Disciplines.
    for d in zvtes::state::disciplines() {
        assert!(
            names.contains(&format!("discipline:{d}")),
            "{d}: a feature for it at the end of context::names"
        );
    }
}

#[test]
fn a_bleed_counts_its_pool_and_a_block_that_lands() {
    // Seat 0 bleeds seat 1, which has no minion to block: 1 pool bled.
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), find_card("Madison"), 6, false);
    e.add_minion(SeatId(0), find_card("Crystal Lynn"), 6, false);
    e.state.turn = 4;
    let pool = e.state.seats[1].pool;
    to_minion_phase(&mut e, SeatId(0));
    pick(&mut e, bleed(a));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.seats[1].pool, pool - 1);
    let p = &e.profiles[0];
    assert!(p.get(Count::Bleeds) == 1 && p.get(Count::PoolBled) == 1);
    assert_eq!(e.profiles[1].get(Count::BlocksLanded), 0);
    let seen = context::features(&e, SeatId(0));
    assert!(slot(&seen, 0, Count::PoolBled) > 0.0 && slot(&seen, 1, Count::PoolBled) == 0.0);
    // From seat 1's seat, seat 0 sits two seats on (3 seats).
    assert_eq!(seat(&context::features(&e, SeatId(1)), 2), seat(&seen, 0));

    // Seat 1's minion blocks the next bleed: the block lands, no pool.
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), find_card("Madison"), 6, false);
    e.add_minion(SeatId(0), find_card("Crystal Lynn"), 6, false);
    let b = e.add_minion(SeatId(1), find_card("Donny Kowalczyk"), 6, false);
    e.state.turn = 4;
    let pool = e.state.seats[1].pool;
    to_minion_phase(&mut e, SeatId(0));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.seats[1].pool, pool);
    let p = &e.profiles[1];
    assert!(p.get(Count::Blocks) == 1 && p.get(Count::BlocksLanded) == 1);
    assert_eq!(e.profiles[0].get(Count::PoolBled), 0);
}
