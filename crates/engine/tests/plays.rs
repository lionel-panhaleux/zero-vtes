//! Slice 1 of the effects framework: the master phase as an offer window,
//! terms at announcement, the as-played cancel window, cost layers, trifle
//! and out-of-turn accounting, the discard phase. Cards: Villein, Sudden
//! Reversal.

use zvtes::cards::{self, vampires::VAMPIRES, Target, Window};
use zvtes::decision::{Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::LIBRARY;
use zvtes::state::{find_card, find_library};

mod common;
use common::*;

fn play(card: u8, target: MinionId) -> Choice {
    Choice::Play {
        card,
        mode: 0,
        actor: None,
        target: Target::Minion(target),
    }
}

/// 3 seats at seat 0's master phase on turn 4. Seat 0 has two ready
/// vampires (0: blood 5, 1: blood 1); seat 1 has one (2).
fn table() -> Engine {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), find_card("Abraham DuSable"), 5, false);
    e.add_minion(SeatId(0), find_card("Abraham Mellon"), 1, false);
    e.add_minion(SeatId(1), find_card("Abaddon"), 5, false);
    e.state.turn = 4;
    e.at_master_phase();
    e
}

/// The whole V5 pool is defined: every library card, and every vampire
/// definition names a V5 crypt card that resolves back to it (a vampire
/// without printed text has none).
#[test]
fn every_card_is_defined_and_its_modes_match_the_reviewed_windows() {
    for def in VAMPIRES {
        let card = find_card(def.name);
        assert_eq!(cards::vampire(card).map(|d| d.name), Some(def.name));
    }
    assert!(cards::vampire(find_card("Alice Chen")).is_none());
    // Definition modes line up with the generated per-mode static half:
    // each maps to a library mode (`cards::library_mode`: the same index
    // unless a printed choice is split), every library mode is covered, and
    // the windows of the definition modes on one library mode are exactly
    // the reviewed matrix's (the matrix is authoritative).
    for (i, c) in LIBRARY.iter().enumerate() {
        let def = cards::lookup(i as u16).unwrap_or_else(|| panic!("{} undefined", c.name));
        let mapped: Vec<usize> = (0..def.modes.len())
            .map(|mi| cards::library_mode(i as u16, mi))
            .collect();
        for lm in 0..c.modes.len() {
            assert!(mapped.contains(&lm), "{} library mode {lm}", c.name);
        }
        for (lmi, lm) in c.modes.iter().enumerate() {
            let mut want: Vec<Window> = lm
                .windows
                .iter()
                .map(|w| Window::from_name(w).unwrap())
                .collect();
            let mut have: Vec<Window> = def
                .modes
                .iter()
                .zip(&mapped)
                .filter(|(_, &m)| m == lmi)
                .flat_map(|(dm, _)| dm.windows.iter().copied())
                .collect();
            want.sort_by_key(|w| *w as u8);
            have.sort_by_key(|w| *w as u8);
            have.dedup();
            assert_eq!(have, want, "{} mode {lmi}", c.name);
        }
    }
}

#[test]
fn villein_targets_terms_blood_move_layers_and_trifle() {
    let mut e = table();
    let villein = find_library("Villein");
    e.state.seats[0].hand.push(villein);
    // Targets: seat 0's vampires with any blood; not seat 1's.
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert_eq!(
        req.options,
        vec![Choice::Pass, play(0, MinionId(0)), play(0, MinionId(1))]
    );
    pick(&mut e, play(0, MinionId(0)));
    // Terms are announced at play: 2 to 5 (rules-qa 3.5).
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(
        req.options,
        vec![
            Choice::SetX(2),
            Choice::SetX(3),
            Choice::SetX(4),
            Choice::SetX(5)
        ]
    );
    pick(&mut e, Choice::SetX(4));
    // Nobody holds a cancel: the as-played window elides; the trifle refunds
    // the master phase action, but the hand is empty, so the master phase
    // elides too and the minion phase opens.
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[0].blood, 1);
    assert_eq!(e.state.seats[0].pool, 34);
    assert_eq!(e.state.seats[0].mpa, 1);
    assert!(e.state.seats[0].trifle_used);
    assert_eq!(e.state.cards.len(), 1);
    assert_eq!(e.state.cards[0].host, Some(MinionId(0)));
    assert_eq!(e.state.layers.len(), 2);
    assert!(e.state.seats[0].ash.is_empty());
}

// Rulings: [LSJ 20020620]; [ANK 20210714].
#[test]
fn villein_costs_one_more_on_a_vampire_that_already_has_one() {
    let mut e = table();
    let villein = find_library("Villein");
    e.state.seats[0].hand.extend([villein, villein]);
    pick(&mut e, play(0, MinionId(0)));
    pick(&mut e, Choice::SetX(2));
    // Trifle refunded: a second Villein this master phase. With 0 pool the
    // copy on the same vampire (cost 1) is masked out; another vampire is
    // free.
    expect(&mut e, SeatId(0), WindowLabel::MasterPhase); // first play settled
    e.state.seats[0].pool = 0;
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert_eq!(req.options, vec![Choice::Pass, play(0, MinionId(1))]);
    e.state.seats[0].pool = 1;
    expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, play(0, MinionId(0)));
    pick(&mut e, Choice::SetX(3));
    // Cost 1 paid, the 3 blood left taken (pool 1 - 1 + 3): the second
    // trifle gives nothing back this master phase.
    // (Drained to 0, the vampire's mandatory hunt is elided at once.)
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 3);
    assert!(e.state.minions[0].blood == 1 && e.state.minions[0].locked);
    assert_eq!(e.state.seats[0].mpa, 0);
}

// Rulings: [LSJ 20020620]; [LSJ 20100216].
#[test]
fn minion_tap_moves_any_amount_paying_its_villein_surcharge_as_pool_is_gained() {
    let mut e = table();
    let (villein, tap) = (find_library("Villein"), find_library("Minion Tap"));
    e.state.seats[0].hand.extend([villein, tap]);
    pick(&mut e, play(0, MinionId(1)));
    pick(&mut e, Choice::SetX(2));
    // Villein took the 1 blood it found; with 1 pool left, Minion Tap costs
    // 1 (Villein's surcharge) and is still offered: the cost is paid as the
    // pool is gained.
    expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    e.state.seats[0].pool = 1;
    pick(&mut e, play(0, MinionId(0)));
    // Any amount, 0 included, up to the 5 blood there.
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(req.options, (0..=5).map(Choice::SetX).collect::<Vec<_>>());
    pick(&mut e, Choice::SetX(4));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 4);
    assert_eq!(e.state.minions[0].blood, 1);
    assert_eq!(e.state.seats[0].ash, vec![tap]);
    // A vampire in torpor is one "you control" too.
    let mut e = table();
    let t = e.add_minion(SeatId(0), find_card("Alice Chen"), 2, true);
    e.state.seats[0].hand.push(tap);
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(req.options.contains(&play(0, t)));
}

// Rulings: [LSJ 20100216].
#[test]
fn minion_tap_takes_what_is_left_at_resolution() {
    let mut e = table();
    let tap = find_library("Minion Tap");
    e.state.seats[0].hand.push(tap);
    e.state.seats[1].hand.push(find_library("Sudden Reversal"));
    pick(&mut e, play(0, MinionId(0)));
    pick(&mut e, Choice::SetX(5));
    // In the as-played window the vampire loses blood: 3 are left to take.
    expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    e.state.minions[0].blood = 3;
    pick(&mut e, Choice::Pass);
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 33);
    // Drained to 0, its mandatory hunt ran at once.
    assert!(e.state.minions[0].blood == 1 && e.state.minions[0].locked);
}

// Rulings: [LSJ 20090626]; [ANK 20190104] [LSJ 19980212].
#[test]
fn sudden_reversal_cancels_a_master_and_its_cost_and_trifle_refund() {
    let mut e = table();
    e.state.seats[0].hand.push(find_library("Villein"));
    e.state.seats[1].hand.push(find_library("Sudden Reversal"));
    pick(&mut e, play(0, MinionId(0)));
    // The terms are announced as the card is played, before the as-played
    // window, so a cancel is decided knowing them (rules-qa 3.5).
    pick(&mut e, Choice::SetX(5));
    // As-played window: seat 1 may cancel (out of turn).
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            Choice::Play {
                card: 0,
                mode: 0,
                actor: None,
                target: Target::None
            }
        ]
    );
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::None,
        },
    );
    // The cancel's own as-played window has no takers; both settle.
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[0].blood, 5);
    assert_eq!(e.state.seats[0].pool, 30);
    assert_eq!(e.state.seats[0].ash, vec![find_library("Villein")]);
    assert_eq!(e.state.seats[1].ash, vec![find_library("Sudden Reversal")]);
    assert!(e.state.cards.is_empty() && e.state.layers.is_empty());
    // A canceled trifle refunds nothing; the master phase action is spent.
    assert_eq!(e.state.seats[0].mpa, 0);
    assert!(e.state.seats[1].out_of_turn.is_some());
}

#[test]
fn discard_phase_replaces_from_the_library() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), find_card("Abraham DuSable"), 5, false);
    e.add_minion(SeatId(0), find_card("Abraham Mellon"), 5, false);
    let a = find_library(".44 Magnum");
    let b = find_library("AK-47");
    e.state.seats[0].hand.push(a);
    e.state.seats[0].library.push(b);
    fill_hand(&mut e, 0);
    e.state.turn = 4;
    pick(&mut e, Choice::Pass); // minion phase; influence elides
    let req = expect(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    // The .44 Magnum, and the first filler card.
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::Discard(0), Choice::Discard(1)]
    );
    pick(&mut e, Choice::Discard(0));
    assert_eq!(e.state.seats[0].ash, vec![a]);
    assert_eq!(e.state.seats[0].hand.last(), Some(&b));
    // One discard phase action: the phase elides and the turn passes; the
    // two empty seats elide their whole turns.
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.turn, 7);
}

// "Do not replace as long as this card is in play" (The Uncoiling): the
// card not replaced counts against the hand size (spec §6.8.1), so with a
// full hand its player discards down at once; as the card leaves play,
// the replacement is drawn.
#[test]
fn a_card_held_while_in_play_counts_against_the_hand_size() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), find_card("Abraham DuSable"), 5, false);
    e.add_minion(SeatId(0), find_card("Abraham Mellon"), 5, false);
    let id = zvtes::state::CardId(e.state.cards.len() as u32);
    e.state.cards.push(zvtes::state::CardInPlay::new(
        find_library("Uncoiling, The"),
        SeatId(0),
        None,
        0,
    ));
    e.state.seats[0].held_in_play.push(id);
    let size = e.state.hand_size(SeatId(0));
    let mist = find_library("Form of Mist");
    e.state.seats[0].hand = vec![mist; size].into();
    e.state.seats[0].library = vec![find_library("AK-47"); 5].into();
    // All alike: the one discard-down choice applies by itself.
    request(&mut e);
    assert_eq!(e.state.seats[0].hand.len(), size - 1);
    assert_eq!(e.state.seats[0].ash, vec![mist]);
    e.burn_card(id);
    request(&mut e);
    assert_eq!(
        e.state.seats[0].hand.len(),
        size,
        "replaced as it leaves play"
    );
    assert_eq!(e.state.seats[0].library.len(), 4);
}
