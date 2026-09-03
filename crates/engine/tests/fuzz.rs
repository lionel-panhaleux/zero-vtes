//! Random-legal fuzzing with invariants and replay determinism (DESIGN §7):
//! every game terminates within a decision budget, invariants hold at every
//! decision point, and (seed, decision log) resimulation reproduces the exact
//! final state hash. Two starting positions: the rulebook opening, and a
//! seeded mid-game position that reaches the torpor / referendum / contest /
//! oust / card-play paths random play from the opening rarely or never gets
//! to.

use std::collections::HashSet;

use zvtes::agent::RandomLegal;
use zvtes::crypt::{CRYPT, PRECONS};
use zvtes::decision::{ActionKind, Choice};
use zvtes::game::{Engine, Step};
use zvtes::ids::SeatId;
use zvtes::rng::Rng;
use zvtes::state::{find_card, find_library, UncontrolledSlot, HAND_SIZE};

const SEEDS: u64 = 100;
const DECISION_BUDGET: u64 = 200_000;

/// Library-card conservation baseline per seat (library + hand + ash +
/// cards in play), fixed at the start of a game.
fn zone_totals(e: &Engine) -> Vec<usize> {
    e.state
        .seats
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let in_play = e
                .state
                .cards
                .iter()
                .filter(|c| c.owner.index() == i)
                .count();
            s.library.len() + s.hand.len() + s.ash.len() + in_play
        })
        .collect()
}

fn invariants(e: &Engine, totals: &[usize], seed: u64) {
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
                slot.counters <= CRYPT[slot.card as usize].capacity,
                "overfilled uncontrolled slot (seed {seed})"
            );
        }
        assert!(
            s.hand.len() <= HAND_SIZE,
            "hand over size at seat {i} (seed {seed})"
        );
    }
    let now = zone_totals(e);
    for (i, (a, b)) in now.iter().zip(totals).enumerate() {
        // Pending plays are out of every zone until they settle: allow a
        // couple of cards in flight per seat (a play and its cancel).
        assert!(
            *a <= *b && *a + 2 >= *b,
            "library cards not conserved at seat {i}: {a} vs {b} (seed {seed})"
        );
    }
    assert!(edges <= 1, "multiple Edges (seed {seed})");
    for (i, m) in st.minions.iter().enumerate() {
        assert!(m.blood <= m.capacity(), "blood over capacity (seed {seed})");
        if m.in_play {
            assert!(
                !st.seats[m.owner.index()].ousted,
                "minion of an ousted seat in play (seed {seed})"
            );
            // Uniqueness: a seat never has two copies of a vampire in play.
            let dup =
                st.minions.iter().enumerate().any(|(j, o)| {
                    j != i && o.owner == m.owner && o.in_play && o.name() == m.name()
                });
            assert!(!dup, "seat holds two copies of {} (seed {seed})", m.name());
        }
        if m.title_lost {
            assert!(
                !m.title_contested,
                "lost title still contested (seed {seed})"
            );
        }
    }
    // 1 VP per oust, +1 for the last survivor, or 0.5 per survivor at time:
    // never more than one VP per seat, in half-points.
    let half_vps: u32 = st.seats.iter().map(|s| s.half_vps).sum();
    assert!(
        half_vps <= 2 * st.seats.len() as u32,
        "more VPs than seats (seed {seed})"
    );
}

/// Coverage key of a chosen option: which kind of decision it was.
fn kind(c: &Choice) -> &'static str {
    match c {
        Choice::Pass => "pass",
        Choice::BurnEdge => "burn_edge",
        Choice::PayContest => "pay_contest",
        Choice::YieldContest => "yield_contest",
        Choice::Play { .. } => "play",
        Choice::SetX(_) => "set_x",
        Choice::Discard(_) => "discard",
        Choice::TakeAction { action, .. } => match action {
            ActionKind::Bleed => "bleed",
            ActionKind::Hunt => "hunt",
            ActionKind::LeaveTorpor => "leave_torpor",
            ActionKind::Rescue => "rescue",
            ActionKind::Diablerie => "diablerie",
        },
        Choice::DeclareBlock { .. } => "declare_block",
        Choice::PayRescue { .. } => "pay_rescue",
        Choice::CastVotes { .. } => "cast_votes",
        Choice::CastEdge { .. } => "cast_edge",
        Choice::Transfer { .. } => "transfer",
        Choice::Withdraw { .. } => "withdraw",
        Choice::DrawCrypt => "draw_crypt",
        Choice::InfluenceOut { .. } => "influence_out",
    }
}

/// Opening position: the rulebook start, 4 seats, random precon crypts and
/// libraries.
fn opening(seed: u64) -> Engine {
    Engine::new(seed, 4)
}

/// Mid-game position: low pools, ready vampires at assorted blood (some at
/// 0, so they can be wounded into torpor when they block), one torpor
/// vampire per seat at assorted blood, a short crypt, fully-funded
/// uncontrolled copies of one shared vampire (card contests) and of the
/// Mannheim prince/baron pair (title contests), the Edge, and the slice-1
/// cards in every hand (master phase, terms, as-played cancels). Random
/// play from the opening never reaches torpor in the card-less pool (a
/// vampire is only ever unlocked at 0 blood on its own turn, where it must
/// hunt), nor an oust; this fixture exercises those paths. Deterministic in
/// `seed`, so replay rebuilds the same start.
fn midgame(seed: u64) -> Engine {
    let mut e = Engine::new(seed, 4);
    let mut rng = Rng::new(seed ^ 0x5eed);
    let shared = find_card("Alice Chen");
    let city = [find_card("Horst von Brühl"), find_card("Aline Gädeke")];
    let villein = find_library("Villein");
    let reversal = find_library("Sudden Reversal");
    for s in 0..4u8 {
        let seat = SeatId(s);
        // Distinct cards for this seat: its precon's list, minus the fixture
        // cards, so no seat ever starts with two copies of a vampire.
        let (_, list) = PRECONS[(seed as usize + s as usize) % PRECONS.len()];
        let mut pool: Vec<u16> = list
            .iter()
            .map(|&(c, _)| c)
            .filter(|c| *c != shared && !city.contains(c))
            .collect();
        rng.shuffle(&mut pool);
        e.state.seats[s as usize].pool = 2 + rng.below(8) as i32;
        for in_torpor in [false, false, true].iter().skip(rng.below(2)) {
            let card = pool.pop().unwrap();
            let blood = (rng.below(4) as u8).min(CRYPT[card as usize].capacity);
            e.add_minion(seat, card, blood, *in_torpor);
        }
        e.state.seats[s as usize].crypt = pool.split_off(pool.len() - 3);
        e.state.seats[s as usize].uncontrolled = vec![
            UncontrolledSlot {
                card: shared,
                counters: CRYPT[shared as usize].capacity,
            },
            UncontrolledSlot {
                card: city[s as usize % 2],
                counters: CRYPT[city[s as usize % 2] as usize].capacity,
            },
        ];
        let hand = &mut e.state.seats[s as usize].hand;
        hand.truncate(HAND_SIZE - 3);
        hand.extend([villein, villein, reversal]);
    }
    e.state.seats[rng.below(4)].has_edge = true;
    e
}

fn play(seed: u64, start: fn(u64) -> Engine) -> (Engine, Vec<u32>, HashSet<&'static str>) {
    let mut e = start(seed);
    let totals = zone_totals(&e);
    let mut agent = RandomLegal::new(seed ^ 0x00c0_ffee);
    let mut chosen = HashSet::new();
    loop {
        assert!(
            e.decisions < DECISION_BUDGET,
            "decision budget exhausted (seed {seed})"
        );
        match e.run() {
            Step::Decision(req) => {
                invariants(&e, &totals, seed);
                let idx = agent.choose(&req);
                chosen.insert(kind(&req.options[idx]));
                e.choose(idx);
            }
            Step::Over(vps) => return (e, vps, chosen),
        }
    }
}

fn fuzz(start: fn(u64) -> Engine) -> HashSet<&'static str> {
    let mut seen = HashSet::new();
    for seed in 0..SEEDS {
        let (played, vps, chosen) = play(seed, start);
        seen.extend(chosen);
        let hash = played.state.hash();

        // Replay: the recorded decision log reproduces the identical game.
        let mut replayed = start(seed);
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
    seen
}

/// Coverage: random play must actually take every listed kind of decision,
/// or the invariants are not being exercised there.
fn covers(seen: &HashSet<&str>, kinds: &[&str]) {
    for k in kinds {
        assert!(
            seen.contains(k),
            "decision kind {k:?} never taken by random play"
        );
    }
}

#[test]
fn random_games_from_the_opening_terminate_hold_invariants_and_replay() {
    let seen = fuzz(opening);
    covers(
        &seen,
        &[
            "pass",
            "burn_edge",
            "bleed",
            "hunt",
            "declare_block",
            "transfer",
            "withdraw",
            "draw_crypt",
            "influence_out",
            "discard",
        ],
    );
}

#[test]
fn random_games_from_midgame_reach_torpor_referendums_contests_plays_and_ousts() {
    let seen = fuzz(midgame);
    covers(
        &seen,
        &[
            "leave_torpor",
            "rescue",
            "pay_rescue",
            "diablerie",
            "cast_votes",
            "cast_edge",
            "pay_contest",
            "yield_contest",
            "play",
            "set_x",
        ],
    );
    // Ousts and game wins happen from here: at least one game must end
    // before the time limit with a full VP awarded.
    let ousted = (0..SEEDS).any(|seed| {
        let (e, vps, _) = play(seed, midgame);
        e.state.seats.iter().any(|s| s.ousted) && vps.iter().any(|&v| v >= 2)
    });
    assert!(ousted, "no oust in {SEEDS} mid-game random games");
}
