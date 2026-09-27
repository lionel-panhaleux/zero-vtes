//! Scenario tests backing `docs/rules-qa.md` answers, named by the test ids
//! reserved there. Only the card-free subset lives here; ids that need
//! library cards (bounces, third-party intercept, political NRA, combat
//! steps) are P1.

use zvtes::cards::Target;
use zvtes::crypt::{Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library};

mod common;
use common::*;

fn act(minion: MinionId, action: ActionKind, target: Option<MinionId>) -> Choice {
    Choice::TakeAction {
        minion,
        action,
        target,
    }
}

/// Any Camarilla primogen: a 1-vote source.
fn primogen() -> u16 {
    CRYPT
        .iter()
        .position(|c| c.title == Title::Primogen)
        .expect("a primogen in the V5 pool") as u16
}

/// 3 seats; seat 0 has two ready untitled vampires (0, 1), seat 1 one (2).
fn table() -> Engine {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), find_card("Abraham DuSable"), 5, false);
    e.add_minion(SeatId(0), find_card("Abraham Mellon"), 5, false);
    e.add_minion(SeatId(1), find_card("Abaddon"), 5, false);
    e
}

/// Reach the blood-hunt polling window: seat 0's vampire 0 diablerises a
/// torpor vampire of seat 1, seat 1 declines to block.
fn blood_hunt(e: &mut Engine) {
    let victim = e.add_minion(SeatId(1), find_card("Alexa Draper"), 0, true);
    pick(e, act(MinionId(0), ActionKind::Diablerie, Some(victim)));
    pick(e, Choice::Pass);
}

// rules-qa 1.3 — a pass with no ongoing attempt is a final decline: nothing
// in the card-less pool reopens it, and the bleed resolves.
#[test]
fn block_decline_is_final() {
    let mut e = table();
    pick(&mut e, act(MinionId(0), ActionKind::Bleed, None));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::ActionBlock)
    );
    pick(&mut e, Choice::Pass);
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::MinionPhase)
    );
    assert_eq!(e.state.seats[1].pool, 29);
}

// rules-qa 1.4 — NRA triggers on resolution, not success: a blocked bleed
// still counts as the minion's one bleed this turn.
#[test]
fn nra_resolution_not_success() {
    let mut e = table();
    pick(&mut e, act(MinionId(0), ActionKind::Bleed, None));
    pick(
        &mut e,
        Choice::DeclareBlock {
            minion: MinionId(2),
        },
    );
    let _ = request(&mut e);
    assert_eq!(e.state.seats[1].pool, 30);
    assert!(e.state.minions[0].bled_this_turn);
}

// rules-qa 1.4 — one bleed per minion per turn (the political half of the
// cap needs cards: P1); other rulebook actions are unrestricted.
#[test]
fn nra_one_bleed_per_turn_hunt_repeatable() {
    let mut e = table();
    e.state.minions[0].blood = 2;
    pick(&mut e, act(MinionId(0), ActionKind::Bleed, None));
    pick(&mut e, Choice::Pass);
    // Unlock the acting minion as a card would; the per-turn cap must hold.
    e.state.minions[0].locked = false;
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&act(MinionId(0), ActionKind::Bleed, None)));
    assert!(req
        .options
        .contains(&act(MinionId(0), ActionKind::Hunt, None)));
    pick(&mut e, act(MinionId(0), ActionKind::Hunt, None));
    e.state.minions[0].locked = false;
    let req = request(&mut e);
    assert!(req
        .options
        .contains(&act(MinionId(0), ActionKind::Hunt, None)));
}

// rules-qa 1.1 — a source casts all its votes at once, in one commitment,
// and is then spent for the referendum.
#[test]
fn referendum_source_atomicity() {
    let mut e = table();
    let prince = e.add_minion(SeatId(1), find_card("Horst von Brühl"), 5, false); // 2 votes
    blood_hunt(&mut e);
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::BloodHuntPolling)
    );
    let casts: Vec<_> = req
        .options
        .iter()
        .filter(|o| matches!(o, Choice::CastVotes { .. }))
        .collect();
    assert_eq!(casts.len(), 2); // for / against, never a partial cast
    pick(
        &mut e,
        Choice::CastVotes {
            minion: prince,
            favor: false,
        },
    );
    // The referendum closes (no other source): 0 for, 2 against → fails.
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::MinionPhase);
    assert!(e.state.minions[0].in_play);
}

// rules-qa 1.1 — sources are independent commitments: a Methuselah may cast
// one source on one rotation and hold another for a later rotation.
#[test]
fn referendum_cross_rotation_sources() {
    let mut e = table();
    let first = e.add_minion(SeatId(1), find_card("Horst von Brühl"), 5, false); // Mannheim
    let second = e.add_minion(SeatId(1), find_card("Alice Chen"), 5, false); // Nairobi
    blood_hunt(&mut e);
    pick(
        &mut e,
        Choice::CastVotes {
            minion: first,
            favor: true,
        },
    );
    // Seats 2 and 0 have nothing (elided); the rotation returns to seat 1
    // with the second source still available — and the first one spent.
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::BloodHuntPolling)
    );
    assert!(req.options.contains(&Choice::CastVotes {
        minion: second,
        favor: true
    }));
    assert!(!req.options.contains(&Choice::CastVotes {
        minion: first,
        favor: true
    }));
    pick(
        &mut e,
        Choice::CastVotes {
            minion: second,
            favor: true,
        },
    );
    let _ = request(&mut e);
    assert!(!e.state.minions[0].in_play); // 4 for > 0 against
}

// rules-qa 1.1 — Methuselah-sourced ballots are independent of the
// Methuselah's vampire votes. The Edge is P0's only Methuselah ballot; the
// multi-ballot case (card discards) is P1.
#[test]
fn referendum_methuselah_ballots_independent() {
    let mut e = table();
    let p = e.add_minion(SeatId(1), primogen(), 5, false); // 1 vote
    e.state.seats[1].has_edge = true;
    blood_hunt(&mut e);
    pick(
        &mut e,
        Choice::CastVotes {
            minion: p,
            favor: true,
        },
    );
    pick(&mut e, Choice::CastEdge { favor: false });
    assert!(!e.state.seats[1].has_edge);
    let _ = request(&mut e);
    assert!(e.state.minions[0].in_play); // 1 for, 1 against: tie fails
}

// rules-qa 3.3 — an out-of-turn master card counts against the player's
// next master phase, even if canceled; no second out-of-turn card before it.
#[test]
fn out_of_turn_consumes_next_mpa() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), find_card("Abraham DuSable"), 5, false);
    e.add_minion(SeatId(0), find_card("Abraham Mellon"), 5, false);
    e.add_minion(SeatId(1), find_card("Abaddon"), 5, false);
    e.state.turn = 4;
    e.at_master_phase();
    let villein = find_library("Villein");
    let reversal = find_library("Sudden Reversal");
    e.state.seats[0].hand.push(villein);
    e.state.seats[1].hand.extend([reversal, villein]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(MinionId(0)),
        },
    );
    pick(&mut e, Choice::SetX(2));
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::None,
        },
    ); // seat 1 cancels
       // Seat 0's turn runs out; seat 1's master phase has no action left, so
       // its Villein is not offered and the phase elides.
    pick(&mut e, Choice::Pass); // seat 0 minion phase
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::MinionPhase)
    );
    assert_eq!(e.state.seats[1].mpa, 0);
    assert!(e.state.seats[1].out_of_turn.is_none());
}

#[test]
fn no_two_out_of_turn_between_turns() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), find_card("Abraham DuSable"), 5, false);
    e.add_minion(SeatId(0), find_card("Abraham Mellon"), 5, false);
    e.state.turn = 4;
    e.at_master_phase();
    e.state.seats[0].mpa = 2; // two masters this phase
    let villein = find_library("Villein");
    let reversal = find_library("Sudden Reversal");
    e.state.seats[0].hand.extend([villein, villein]);
    e.state.seats[1].hand.extend([reversal, reversal]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(MinionId(0)),
        },
    );
    pick(&mut e, Choice::SetX(2));
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::None,
        },
    ); // first cancel
       // Second Villein: seat 1 still holds a Sudden Reversal but may not play
       // it before its own master phase — the as-played window elides.
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(MinionId(0)),
        },
    );
    pick(&mut e, Choice::SetX(2));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::MinionPhase)
    );
    assert_eq!(e.state.minions[0].blood, 3);
    assert_eq!(e.state.seats[1].hand.len(), 1);
}
