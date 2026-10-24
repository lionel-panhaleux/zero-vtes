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
    decline_futile(&mut e);
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
    expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
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

/// Pass until `seat`'s unlock-phase window, `n` times over (the first is
/// the next one).
fn to_unlock(e: &mut Engine, seat: SeatId, n: usize) -> zvtes::decision::DecisionRequest {
    let mut req = until(e, seat, WindowLabel::UnlockAbilities);
    for _ in 1..n {
        e.choose(fallback(&req));
        req = until(e, seat, WindowLabel::UnlockAbilities);
    }
    req
}

// rules-qa 95.3–95.5 — a withdrawal (library exhausted, turn begun short
// of a full hand, announced in the unlock phase) kept until the next
// unlock phase: out of the game with half a victory point (90.9), the
// predator gaining neither victory point nor pool (rulebook "Withdrawing
// from the Game").
#[test]
fn a_withdrawal_kept_a_round_scores_half_a_victory_point() {
    let mut e = table();
    e.add_minion(SeatId(2), find_card("Alexa Draper"), 5, false);
    let req = to_unlock(&mut e, SeatId(0), 1);
    assert!(req.options.contains(&Choice::WithdrawFromGame));
    pick(&mut e, Choice::WithdrawFromGame);
    // Announced once.
    assert!(!e.state.may_announce_withdrawal(SeatId(0)));
    assert!(!request(&mut e).options.contains(&Choice::WithdrawFromGame));
    let predator_pool = e.state.seats[2].pool;
    to_unlock(&mut e, SeatId(2), 1);
    let req = to_unlock(&mut e, SeatId(1), 1);
    assert_eq!(req.seat, SeatId(1));
    let s = &e.state.seats[0];
    assert!(s.withdrew && s.ousted);
    let vps: Vec<u32> = e.state.seats.iter().map(|s| s.half_vps).collect();
    assert_eq!(vps, [1, 0, 0]);
    assert_eq!(e.state.seats[2].pool, predator_pool);
}

// rules-qa 95.4 — pool lost before the next unlock phase breaks the
// withdrawal; it may be announced again then.
#[test]
fn a_withdrawal_fails_on_a_pool_loss() {
    let mut e = table();
    let bleeder = e.add_minion(SeatId(2), find_card("Alexa Draper"), 5, false);
    to_unlock(&mut e, SeatId(0), 1);
    pick(&mut e, Choice::WithdrawFromGame);
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    pick(&mut e, act(bleeder, ActionKind::Bleed, None));
    let req = to_unlock(&mut e, SeatId(0), 1);
    assert!(e.state.seats[0].pool < 30);
    assert!(!e.state.seats[0].withdrew);
    assert!(req.options.contains(&Choice::WithdrawFromGame));
}

// rules-qa 95.4 — one of the Methuselah's minions entering combat breaks
// the withdrawal.
#[test]
fn a_withdrawal_fails_on_combat() {
    let mut e = table();
    let bleeder = e.add_minion(SeatId(2), find_card("Alexa Draper"), 5, false);
    to_unlock(&mut e, SeatId(0), 1);
    pick(&mut e, Choice::WithdrawFromGame);
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    pick(&mut e, act(bleeder, ActionKind::Bleed, None));
    pick(&mut e, block(MinionId(0)));
    to_unlock(&mut e, SeatId(0), 1);
    assert!(!e.state.seats[0].withdrew);
}

// rules-qa 95.3 — no withdrawal with a full hand at the turn's start: the
// unlock phase offers none.
#[test]
fn no_withdrawal_with_a_full_hand() {
    let mut e = table();
    fill_hand(&mut e, 0);
    until(&mut e, SeatId(2), WindowLabel::UnlockAbilities);
    loop {
        let req = request(&mut e);
        if req.seat == SeatId(0) {
            assert!(!req.options.contains(&Choice::WithdrawFromGame));
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
}

// rules-qa 95.2, 95.4 — blood a vampire of the Methuselah spends, no
// combat (leaving torpor), breaks the withdrawal.
#[test]
fn a_withdrawal_fails_on_blood_spent() {
    let mut e = table();
    let torpid = e.add_minion(SeatId(0), find_card("Alexa Draper"), 4, true);
    to_unlock(&mut e, SeatId(0), 1);
    pick(&mut e, Choice::WithdrawFromGame);
    pick(&mut e, act(torpid, ActionKind::LeaveTorpor, None));
    let req = to_unlock(&mut e, SeatId(0), 1);
    assert!(e.state.minions[torpid.index()].blood < 4);
    assert!(!e.state.seats[0].withdrew);
    assert!(req.options.contains(&Choice::WithdrawFromGame));
}
