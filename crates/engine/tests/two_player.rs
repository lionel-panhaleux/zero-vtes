//! VEKN's two-player variant (`Format::TwoPlayer`, "Two-player variant for
//! Vampire: The Eternal Struggle", vekn.net): a game dealt with 2 seats
//! plays it; each rule beside its standard counterpart.

use zvtes::crypt::{Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::{Contest, Engine};
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{Format, UncontrolledSlot};

mod common;
use common::*;

// "The Methuselah who takes the first turn gets only 3 transfers on their
// first influence phase"; then 4. Four seats: 1, then 2 for the second
// player (rulebook "Influence Phase").
#[test]
fn two_player_transfers_are_3_then_4() {
    let mut e = Engine::new(7, 2);
    assert_eq!(e.state.format, Format::TwoPlayer);
    until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    assert_eq!(e.transfers_left(), Some(3));
    until(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    assert_eq!(e.transfers_left(), Some(4));
    let mut e = Engine::new(7, 4);
    assert_eq!(e.state.format, Format::Standard);
    until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    assert_eq!(e.transfers_left(), Some(1));
    until(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    assert_eq!(e.transfers_left(), Some(2));
}

/// Seat 0 holds `card` (MinionId(1), beside a second vampire); seat 1
/// influences a copy out on turn 4, in a two-player game.
fn contested_copy(card: u16) -> (Engine, MinionId) {
    let mut e = Engine::bare(2);
    e.state.format = Format::TwoPlayer;
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), card, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.seats[1].uncontrolled.push(UncontrolledSlot {
        card,
        copy: Default::default(),
        counters: CRYPT[card as usize].capacity,
        banished: None,
    });
    e.state.turn = 4;
    pick(&mut e, Choice::Pass); // seat 0's minion phase
    until(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    let copy = MinionId(e.state.minions.len() as u32 - 1);
    (e, copy)
}

// "Control of the card is being contested … (contested crypt cards are no
// longer out of play)": both copies in play and usable; 1 pool in each of
// the controller's unlock phases, or yielded and burned; the contest over
// once the others yield. The standard counterpart (face down, out of play):
// game.rs `card_contest_pay_yield_and_return`.
#[test]
fn a_contested_crypt_card_stays_in_play_its_upkeep_pool() {
    let (mut e, copy) = contested_copy(plain("Brujah", 0));
    let mine = MinionId(1);
    for m in [mine, copy] {
        let v = &e.state.minions[m.index()];
        assert!(v.contested_in_play && !v.contested && v.present(), "{m:?}");
    }
    let item = Contest::Card(mine);
    let req = expect(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert_eq!(
        req.options,
        vec![
            Choice::PayContest(item),
            Choice::YieldContest(item),
            Choice::WithdrawFromGame
        ]
    );
    let pool = e.state.seats[0].pool;
    pick(&mut e, Choice::PayContest(item));
    assert_eq!(e.state.seats[0].pool, pool - 1);
    // Used as normal: it acts.
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&action(mine, ActionKind::Hunt)));
    pick(&mut e, Choice::Pass);
    expect(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    pick(&mut e, Choice::YieldContest(Contest::Card(copy)));
    assert!(!e.state.minions[copy.index()].in_play, "yielded: burned");
    to_minion_phase(&mut e, SeatId(0));
    assert!(
        !e.state.minions[mine.index()].contested_in_play,
        "no rival left"
    );
}

/// Two unique princes of one city (their texts beside the point: only
/// their titles are read).
fn two_princes() -> (u16, u16) {
    let prince = |i: u16| {
        let c = &CRYPT[i as usize];
        c.unique && matches!(c.title, Title::Prince(_))
    };
    let all = 0..CRYPT.len() as u16;
    all.clone()
        .filter(|&a| prince(a))
        .flat_map(|a| all.clone().map(move |b| (a, b)))
        .find(|&(a, b)| {
            let (x, y) = (&CRYPT[a as usize], &CRYPT[b as usize]);
            prince(b) && x.title == y.title && x.name != y.name
        })
        .expect("two princes of one city")
}

// "If two contested vampires (two copies of the same vampire) have a title,
// their title is not contested."
#[test]
fn two_contested_copies_keep_their_title() {
    let (e, copy) = contested_copy(two_princes().0);
    for m in [MinionId(1), copy] {
        assert!(!e.state.minions[m.index()].title_contested, "{m:?}");
        assert_ne!(e.state.title_of(m), Title::None, "{m:?}");
    }
}

// Another vampire claiming that title contests it by the rulebook: the two
// copies and it, all three contesting (rules-qa 91.6).
#[test]
fn another_claimant_contests_both_copies_title() {
    let (prince, rival) = two_princes();
    let mut e = Engine::bare(2);
    e.state.format = Format::TwoPlayer;
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), prince, 4, false);
    let other = e.add_minion(SeatId(1), rival, 4, false);
    e.stage_contests(other);
    e.state.seats[1].uncontrolled.push(UncontrolledSlot {
        card: prince,
        copy: Default::default(),
        counters: CRYPT[prince as usize].capacity,
        banished: None,
    });
    e.state.turn = 4;
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    let copy = MinionId(e.state.minions.len() as u32 - 1);
    for m in [MinionId(1), other, copy] {
        assert!(e.state.minions[m.index()].title_contested, "{m:?}");
    }
}
