//! Rulings as tests (DESIGN §7.1): every test here backs one or more rows
//! of `data/rulings-v5.csv` (`status: test:<fn>`), cites the ruling's
//! references in the comment above it, and plays the ruling out as a
//! decision script — a game transcript that stops only where a real
//! decision exists (single-option windows elide). Rows backed by tests in
//! the other files cite them the same way; `scripts/rulings_v5.py --check
//! data/rulings-v5.csv` keeps the ledger and the tests in sync.

use zvtes::actions::Outcome;
use zvtes::cards::AbilitySource;
use zvtes::cards::{Layer, Modifier, Scope, Target};
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, CardId, CardInPlay, Expiry, Release, TimedStat};

mod common;
use common::*;

// ---- Script helpers ---------------------------------------------------------

/// The fallback answer for a window the script does not care about: pass,
/// or hand strike, or the first option.
fn fallback(req: &DecisionRequest) -> Choice {
    if req.options.contains(&Choice::Pass) {
        Choice::Pass
    } else if req.options.contains(&Choice::HandStrike) {
        Choice::HandStrike
    } else {
        req.options[0]
    }
}

/// Answer requests with `policy` until it returns `None`; that request is
/// returned.
fn drive(
    e: &mut Engine,
    mut policy: impl FnMut(&Engine, &DecisionRequest) -> Option<Choice>,
) -> DecisionRequest {
    loop {
        let req = request(e);
        match policy(e, &req) {
            Some(c) => {
                let i = index_of(&req, c);
                e.choose(i);
            }
            None => return req,
        }
    }
}

/// A card played by the Methuselah (master cards, Archon Investigation).
fn master(card: u8, target: Target) -> Choice {
    Choice::Play {
        card,
        mode: 0,
        actor: None,
        target,
    }
}

fn ability(card: u32, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(CardId(card)),
        ability,
        actor: None,
        target,
    }
}

fn library(e: &mut Engine, seat: u8, card: &str, n: usize) {
    e.state.seats[seat as usize].library = vec![find_library(card); n].into();
}

/// A card already in play (as its play would have left it, layers apart).
fn in_play(e: &mut Engine, name: &str, owner: u8, host: Option<MinionId>) -> u32 {
    e.state
        .cards
        .push(CardInPlay::new(find_library(name), SeatId(owner), host, 0));
    e.state.cards.len() as u32 - 1
}

fn set_pool(e: &mut Engine, pool: i32) {
    for s in &mut e.state.seats {
        s.pool = pool;
    }
}

fn half_vps(e: &Engine) -> Vec<u32> {
    e.state.seats.iter().map(|s| s.half_vps).collect()
}

fn locked(e: &Engine, m: MinionId) -> bool {
    e.state.minions[m.index()].locked
}

fn in_torpor(e: &Engine, m: MinionId) -> bool {
    e.state.minions[m.index()].in_torpor
}

/// `n` seats at seat 0's minion phase, turn 4, 10 pool each.
fn game(n: usize) -> Engine {
    let mut e = Engine::bare(n);
    set_pool(&mut e, 10);
    e.state.turn = 4;
    e
}

/// 3 seats at seat 0's minion phase: seat 0 has a (DOM, 6 blood) and a
/// second vampire (so its minion phase stays a real decision once a is
/// locked); seat 1 x (FOR, 4); seat 2 z (PRO, 4).
fn bleed_table() -> (Engine, MinionId, MinionId, MinionId) {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("DOM", 0), 6, false);
    e.add_minion(SeatId(0), find_card("Farah Sarroub"), 6, false);
    let x = e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
    let z = e.add_minion(SeatId(2), find_card("Crystal Lynn"), 4, false);
    (e, a, x, z)
}

/// 3 seats at seat 0's minion phase. Seat 0: Alexa Draper (Camarilla
/// Prince, 2 votes, DOM PRE), Mateo García (capacity 2, so seat 0's
/// minion phase stays a real decision once Alexa is locked); seat 1: Kevin
/// Jackson (Camarilla Prince, 2 votes), Bret Stryker (capacity 4, CEL
/// PRE); seat 2: Lenny Burkhead (primogen, 1 vote, ANI).
struct Politics {
    e: Engine,
    alexa: MinionId,
    kevin: MinionId,
    bret: MinionId,
    lenny: MinionId,
}

fn politics() -> Politics {
    let mut e = game(3);
    let alexa = e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    e.add_minion(SeatId(0), find_card("Mateo García"), 2, false);
    let kevin = e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
    let bret = e.add_minion(SeatId(1), find_card("Bret Stryker"), 4, false);
    let lenny = e.add_minion(SeatId(2), find_card("Lenny Burkhead"), 4, false);
    Politics {
        e,
        alexa,
        kevin,
        bret,
        lenny,
    }
}

// ---- Cards in play, master cards, the unlock phase --------------------------

// [LSJ 20020201] Delaying Tactics "played on a referendum provided by a
// card in play (eg. Anarch Revolt) does not prevent referendums provided
// by another copy of the same card in play": the repeat ban is per copy.
#[test]
fn delaying_tactics_on_one_copys_referendum_leaves_the_other_copy() {
    let mut e = game(3);
    let alexa = e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    let kevin = e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
    let anita = e.add_minion(SeatId(1), find_card("Anita Wainwright"), 3, false);
    e.add_minion(SeatId(2), find_card("Lenny Burkhead"), 4, false);
    let first = in_play(&mut e, "Anarch Revolt", 0, None);
    let second = in_play(&mut e, "Anarch Revolt", 2, None);
    hand(&mut e, 0, &["Delaying Tactics"]);
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, granted(first, kevin));
    pick(&mut e, votes(kevin, true));
    pick(&mut e, Choice::Pass); // seat 2
    pick(&mut e, play(0, 0, alexa)); // Delaying Tactics
    let req = expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!locked(&e, kevin));
    // Kevin has taken his political action; Anita may still call the
    // other copy's referendum, not the canceled copy's.
    assert_eq!(abilities(&req), vec![granted(second, anita)]);
}

// [PIB 20121031] The Coven "can be locked during damage resolution, but
// not while healing damage or preventing destruction (you cannot take 2,
// lock The Coven, then take the rest)": the lock is offered in the
// prevention window, and the damage then lands at once.
#[test]
fn the_coven_can_feed_in_the_prevention_window_but_damage_lands_at_once() {
    let (mut e, _a, x, _z) = bleed_table();
    let p = e.add_minion(SeatId(0), vampire_with("POT", 0), 6, false);
    let coven = in_play(&mut e, "Coven, The", 1, None);
    e.state.minions[x.index()].blood = 0;
    hand(&mut e, 0, &["Roundhouse"]);
    pick(&mut e, bleed(p));
    // Every impulse is a real window for seat 1 while The Coven is
    // unlocked; x blocks, p strikes with Roundhouse [pot] (3 damage).
    drive(&mut e, |_, req| {
        if (req.seat, req.window) == (SeatId(1), WindowLabel::CombatPrevention) {
            return None;
        }
        if req.options.contains(&block(x)) {
            return Some(block(x));
        }
        if req.window == WindowLabel::CombatStrike && req.seat == SeatId(0) {
            return Some(play(0, 0, p));
        }
        Some(fallback(req))
    });
    let feed = ability(coven, 0, Target::Minion(x));
    pick(&mut e, feed);
    assert_eq!(blood(&e, x), 2);
    drive(&mut e, |_, req| {
        (req.window != WindowLabel::MinionPhase).then(|| fallback(req))
    });
    // 3 damage on 2 blood, resolved at once: torpor.
    assert!(in_torpor(&e, x));
    assert_eq!(blood(&e, x), 0);
}

// [RBK contested-cards] Unique trifle: "You still gain the trifle master
// phase action, even if this card contests when played."
#[test]
fn unique_trifle_contested_when_played_still_refunds_the_action() {
    let mut e = game(3);
    e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
    e.add_minion(SeatId(2), find_card("Lenny Burkhead"), 4, false);
    let mine = in_play(&mut e, "Coven, The", 0, None);
    hand(&mut e, 1, &["Coven, The", "Villein"]);
    to_window(&mut e, SeatId(1), WindowLabel::MasterPhase);
    pick(&mut e, master(0, Target::None));
    // Both copies contested; the trifle's action is back and the master
    // phase reopens on the Villein.
    let req = expect(&mut e, SeatId(1), WindowLabel::MasterPhase);
    assert!(e.state.cards[mine as usize].contested && e.state.cards[1].contested);
    assert_eq!(e.state.seats[1].mpa, 1);
    assert!(e.state.seats[1].trifle_used);
    assert_eq!(plays(&req).len(), 1);
}

// [RTR 20040501] Dreams of the Sphinx "cannot be used when a card is
// played to draw a card that would cancel it 'as it is played'";
// [LSJ 20061207] [LSJ 20061013] no effects but wakes and the cancel in the
// as-played window; [ANK 20200616] "can be used during an action, when the
// controller has the opportunity to play an effect".
#[test]
fn dreams_of_the_sphinx_is_usable_during_an_action_but_not_in_the_as_played_window() {
    let mut e = game(2);
    let alexa = e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
    e.at_master_phase();
    let dreams = in_play(&mut e, "Dreams of the Sphinx", 1, None);
    hand(&mut e, 0, &["Villein"]);
    hand(&mut e, 1, &["Sudden Reversal"]);
    library(&mut e, 1, "Bonding", 4);
    pick(&mut e, master(0, Target::Minion(alexa)));
    pick(&mut e, Choice::SetX(2));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert!(uses(&req, dreams).is_empty(), "{:?}", req.options);
    pick(&mut e, Choice::Pass);
    // Seat 0's bleed: the lock is offered to seat 1 as soon as it holds
    // the impulse.
    to_minion_phase(&mut e, SeatId(0));
    pick(&mut e, bleed(alexa));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionAsAnnounced);
    assert_eq!(uses(&req, dreams), vec![ability(dreams, 0, Target::None)]);
    pick(&mut e, ability(dreams, 0, Target::None));
    assert_eq!(e.state.seats[1].hand.len(), 5); // 1 + the 4 in the library
    assert!(e.state.cards[dreams as usize].locked);
}

// [LSJ 20100812] [LSJ 20020904-2] "The hand size bonus ... ends at the end
// of the discard phase ... Any discard phase actions ... must be handled
// first": no discard-down during the discard phase — the discard-down to
// the rulebook's hand size comes once the bonus has ended with the turn.
#[test]
fn dreams_of_the_sphinx_bonus_lasts_through_the_discard_phase() {
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
    e.at_master_phase();
    let dreams = in_play(&mut e, "Dreams of the Sphinx", 0, None);
    // Not all alike: discarding down stays a decision (one choice per
    // card).
    hand(&mut e, 0, &["Bonding"; 6]);
    hand(&mut e, 0, &["Villein"]);
    library(&mut e, 0, "Bonding", 4);
    pick(&mut e, ability(dreams, 0, Target::None));
    assert_eq!(e.state.seats[0].hand.len(), 9);
    let mut discard_phase = false;
    let req = drive(&mut e, |e, req| {
        if req.window == WindowLabel::DiscardPhase && req.seat == SeatId(0) {
            // Still 9 cards and the bonus in force during the phase.
            assert_eq!(
                e.state.seats[0]
                    .timed
                    .of(TimedStat::HandSize, Expiry::EndOfTurn),
                2
            );
            discard_phase = true;
        }
        (req.window != WindowLabel::DiscardDown).then(|| fallback(req))
    });
    assert!(discard_phase);
    assert_eq!(req.seat, SeatId(0));
    assert_eq!(e.state.active, SeatId(1));
    assert_eq!(
        e.state.seats[0]
            .timed
            .of(TimedStat::HandSize, Expiry::EndOfTurn),
        0
    );
    pick(&mut e, Choice::Discard(0));
    pick(&mut e, Choice::Discard(0));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.seats[0].hand.len(), 7);
}

// [RBK 2-card-types] "You cannot lock the card 'in response to' another
// copy being played (in order to use it before it becomes contested)."
#[test]
fn dreams_of_the_sphinx_cannot_be_locked_in_response_to_a_second_copy() {
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
    let mine = in_play(&mut e, "Dreams of the Sphinx", 0, None);
    hand(&mut e, 0, &["Sudden Reversal"]);
    hand(&mut e, 1, &["Dreams of the Sphinx"]);
    to_window(&mut e, SeatId(1), WindowLabel::MasterPhase);
    pick(&mut e, master(0, Target::None));
    // The as-played window: the cancel, not the lock.
    let req = expect(&mut e, SeatId(0), WindowLabel::CardAsPlayed);
    assert!(uses(&req, mine).is_empty());
    assert_eq!(plays(&req).len(), 1);
    pick(&mut e, Choice::Pass);
    let _ = request(&mut e);
    assert!(e.state.cards[mine as usize].contested && e.state.cards[1].contested);
}

// [ANK 20210817] A wake "can be played or used during the 'as a card is
// played' window, except in combat, or when there is no action": not in a
// master card's as-played window, yes in an action modifier's.
#[test]
fn wakes_in_the_as_played_window_only_during_an_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("DOM", 0), 6, false);
    let b = e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
    e.state.minions[b.index()].locked = true;
    e.at_master_phase();
    hand(&mut e, 0, &["Villein", "Bonding"]);
    hand(&mut e, 1, &["Eyes of Argus", "Sudden Reversal"]);
    pick(&mut e, master(0, Target::Minion(a)));
    pick(&mut e, Choice::SetX(2));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert_eq!(plays(&req), vec![master(1, Target::None)]);
    pick(&mut e, Choice::Pass);
    pick(&mut e, bleed(a));
    pick(&mut e, play(0, 0, a)); // Bonding [dom]
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert_eq!(plays(&req), vec![play(0, 1, b)]);
}

// [RTR 19941109] [ANK 20180612] Homunculus: "The discipline level used to
// recruit it decides which version it is (inferior or superior). This does
// not change once it is in play."
#[test]
fn homunculus_version_is_fixed_by_the_employ_level() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), find_card("Crystal Lynn"), 6, false);
    let q = e.add_minion(SeatId(0), vampire_with("PRO", 1), 6, false);
    e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
    hand(&mut e, 0, &["Homunculus", "Homunculus"]);
    pick(&mut e, play(0, 0, p)); // [pro]
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[0].counters, 1);
    pick(&mut e, play(0, 1, q)); // [PRO]
    decline_futile(&mut e);
    assert_eq!(e.state.cards[1].counters, 2);
    assert_eq!(e.state.cards[0].counters, 1);
}

// [ANK 20200629-2] [LSJ 20091208] [RTR 19951017] "Replace next turn": the
// card "is replaced before unlocking cards: other unlock effects cannot be
// ordered before" — the hand is full when the unlock effects window opens.
#[test]
fn old_friends_is_replaced_before_the_unlock_effects() {
    let (mut e, _a, _x, _z) = bleed_table();
    in_play(&mut e, "Barrens, The", 0, None);
    hand(&mut e, 0, &["Bonding"; 6]);
    library(&mut e, 0, "Bonding", 3);
    e.state.seats[0].held[Release::NextUnlockPhase] = 1;
    let req = drive(&mut e, |e, req| {
        let own = (req.seat, req.window) == (SeatId(0), WindowLabel::UnlockAbilities)
            && e.state.active == SeatId(0);
        (!own).then(|| fallback(req))
    });
    assert!(!abilities(&req).is_empty());
    assert_eq!(e.state.seats[0].hand.len(), 7);
    assert_eq!(e.state.seats[0].held[Release::NextUnlockPhase], 0);
}

// [LSJ 20061207] [LSJ 20061013] "Cards are not replaced during the 'as
// played' window."
#[test]
fn no_replacement_in_the_as_played_window() {
    let mut e = game(2);
    let alexa = e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
    e.at_master_phase();
    hand(&mut e, 0, &["Villein"; 7]);
    library(&mut e, 0, "Bonding", 3);
    hand(&mut e, 1, &["Sudden Reversal"]);
    pick(&mut e, master(0, Target::Minion(alexa)));
    pick(&mut e, Choice::SetX(2));
    expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert_eq!(e.state.seats[0].hand.len(), 6);
    pick(&mut e, Choice::Pass);
    expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert_eq!(e.state.seats[0].hand.len(), 7);
}

// [RTR 20001020] Sudden Reversal "can only cancel master cards": an action
// modifier's as-played window offers it nothing. (No card in the pool
// plays a master card other than from the hand, so that half is moot.)
#[test]
fn sudden_reversal_cancels_only_master_cards() {
    let (mut e, a, _x, _z) = bleed_table();
    hand(&mut e, 0, &["Bonding"]);
    hand(&mut e, 1, &["Sudden Reversal"]);
    pick(&mut e, bleed(a));
    pick(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert_ne!(req.window, WindowLabel::CardAsPlayed, "{req:?}");
}

// [PIB 20150522] Vessel "can be used when the vampire is in torpor".
#[test]
fn vessel_usable_with_the_host_in_torpor() {
    let (mut e, a, _x, _z) = bleed_table();
    let vessel = in_play(&mut e, "Vessel", 0, Some(a));
    e.state.minions[a.index()].in_torpor = true;
    let req = drive(&mut e, |e, req| {
        let own = (req.seat, req.window) == (SeatId(0), WindowLabel::UnlockAbilities)
            && e.state.active == SeatId(0);
        (!own).then(|| fallback(req))
    });
    let to_pool = ability(vessel, 0, Target::None);
    assert!(uses(&req, vessel).contains(&to_pool), "{:?}", req.options);
    pick(&mut e, to_pool);
    assert_eq!((blood(&e, a), pool(&e)[0]), (5, 11));
}

// ---- The bleed: modifiers, reactions, bounces --------------------------------

// [LSJ 20050422] "Only effects played 'when the action is/would be
// successful' can be played afterwards (eg. Deflection, Archon
// Investigation or Conditioning can only be used before)"; [LSJ 19980105]
// Archon Investigation and the bounces "must be played before action
// resolution": once the blocks-declined window closes the bleed lands.
#[test]
fn bleed_cards_are_not_playable_after_resolution() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("DOM", 0), 6, false);
    e.add_minion(SeatId(0), find_card("Farah Sarroub"), 6, false);
    e.add_minion(SeatId(1), find_card("Monica Giovanni"), 6, false);
    e.add_minion(SeatId(1), find_card("Chelsea Blake"), 6, false);
    e.add_minion(SeatId(2), find_card("Monica Giovanni"), 6, false);
    hand(&mut e, 0, &["Conditioning"]);
    hand(&mut e, 1, &["Deflection", "Archon Investigation"]);
    pick(&mut e, bleed(a));
    expect(&mut e, SeatId(0), WindowLabel::ActionBlock);
    pick(&mut e, Choice::Pass);
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, Choice::Pass);
    expect(&mut e, SeatId(0), WindowLabel::ActionBlocksDeclined);
    pick(&mut e, play(0, 1, a)); // [DOM] +3: bleed 4
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlocksDeclined);
    assert!(plays(&req).contains(&master(1, Target::None)));
    assert!(plays(&req).len() > 1);
    pick(&mut e, Choice::Pass);
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 6, 10]);
    assert_eq!(e.state.seats[1].hand.len(), 2);
}

// [LSJ 19980824] [RTR 19941109] Bonding "cannot be used to increase the
// stealth of a non-bleed action": on a hunt it is never offered.
#[test]
fn bonding_gives_no_stealth_to_a_non_bleed_action() {
    let (mut e, a, _x, _z) = bleed_table();
    // Room for the hunt's blood whatever a's capacity.
    e.state.minions[a.index()].blood = 5;
    hand(&mut e, 0, &["Bonding"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 6);
    assert_eq!(e.state.seats[0].hand.len(), 1);
}

// [ANK 20190425] An "as the action is announced" modifier "must be played
// ... before action modifiers that are not explicitly played 'as the action
// is announced' (e.g., Conditioning)".
#[test]
fn as_announced_modifiers_precede_other_modifiers() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Aintzane Arriaga"), 6, false); // DOM OBL POT
    e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
    hand(&mut e, 0, &["Consign to Oblivion", "Conditioning"]);
    pick(&mut e, bleed(a));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionAsAnnounced);
    assert_eq!(plays(&req), vec![play(0, 0, a), play(0, 1, a)]);
    pick(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionBlock);
    assert_eq!(plays(&req), vec![play(0, 0, a), play(0, 1, a)]);
}

// [ANK 20181208] "Replace the action card (if any) before playing" an "as
// the action is announced" modifier: the action card's replacement, drawn
// as the action is declared, may be the Consign to Oblivion played on it.
#[test]
fn the_action_cards_replacement_is_playable_as_the_action_is_announced() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Aintzane Arriaga"), 6, false); // DOM OBL POT
    e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    library(&mut e, 0, "Consign to Oblivion", 1);
    pick(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionAsAnnounced);
    assert_eq!(plays(&req), vec![play(0, 0, a), play(0, 1, a)]);
}

// [ANK 20181208] Seduction, The Sleeping Mind: "Replace the action card (if
// any) before playing it." Either one, drawn as the action card's
// replacement, is playable as the action is announced.
#[test]
fn an_as_announced_modifier_drawn_as_the_replacement_is_playable() {
    for card in ["Seduction", "Sleeping Mind, The"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Aintzane Arriaga"), 6, false); // DOM OBL POT
        e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
        hand(&mut e, 0, &["Kine Resources Contested"]);
        library(&mut e, 0, card, 1);
        pick(&mut e, play(0, 0, a));
        let req = expect(&mut e, SeatId(0), WindowLabel::ActionAsAnnounced);
        assert!(
            plays(&req)
                .iter()
                .any(|p| matches!(p, Choice::Play { card: 0, actor: Some(m), .. } if *m == a)),
            "{card}: {:?}",
            req.options
        );
    }
}

// ---- Replacement and hand size ----------------------------------------------

// [ANK 20180318] "Cards that are not replaced count against the hand
// size": with Old Friends held back until the unlock phase, Dreams of the
// Sphinx's +2 draws the hand up to 8, not 9.
#[test]
fn cards_not_replaced_count_against_the_hand_size() {
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
    e.at_master_phase();
    let dreams = in_play(&mut e, "Dreams of the Sphinx", 0, None);
    hand(&mut e, 0, &["Bonding"; 6]);
    e.state.seats[0].held[Release::NextUnlockPhase] = 1;
    library(&mut e, 0, "Bonding", 5);
    pick(&mut e, ability(dreams, 0, Target::None));
    assert_eq!(e.state.seats[0].hand.len(), 8);
    assert_eq!(e.state.seats[0].library.len(), 3);
}

// [ANK 20221028] Elder Library: "If there is another copy in play, it
// enters contest without modifying the hand size" — and the copy it
// contests gives no hand size either from then on (spec §1.13.3), so its
// controller discards down at once.
#[test]
fn elder_library_enters_contest_without_modifying_the_hand_size() {
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
    e.at_master_phase();
    let theirs = in_play(&mut e, "Elder Library", 1, None);
    e.state.layers.push(Layer {
        scope: Scope::InPlay,
        modifier: Modifier::HandSize(1),
        source: CardId(theirs),
    });
    hand(&mut e, 0, &["Elder Library"]);
    hand(&mut e, 0, &["Bonding"; 6]);
    library(&mut e, 0, "Bonding", 3);
    // Not all alike: discarding down stays a decision (one choice per
    // card).
    hand(&mut e, 1, &["Bonding"; 7]);
    hand(&mut e, 1, &["Villein"]);
    pick(&mut e, master(0, Target::None));
    let req = expect(&mut e, SeatId(1), WindowLabel::DiscardDown);
    assert_eq!(req.options.len(), 2);
    assert!(e.state.cards[theirs as usize].contested && e.state.cards[1].contested);
    // Seat 0 replaced up to 7, not 8.
    assert_eq!(e.state.seats[0].hand.len(), 7);
    assert_eq!(e.state.seats[0].library.len(), 2);
    pick(&mut e, Choice::Discard(0));
    let _ = request(&mut e);
    assert_eq!(e.state.seats[1].hand.len(), 7);
}

// [PIB 20121031] Visit from the Capuchin "cannot be played if you have
// another one, even if it has only one counter left".
#[test]
fn visit_from_the_capuchin_is_not_playable_beside_ones_own_copy() {
    let mut e = game(2);
    let alexa = e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
    e.at_master_phase();
    let mine = in_play(&mut e, "Visit from the Capuchin", 0, None);
    e.state.cards[mine as usize].counters = 1;
    e.state.layers.push(Layer {
        scope: Scope::InPlay,
        modifier: Modifier::HandSizePerCounter,
        source: CardId(mine),
    });
    hand(&mut e, 0, &["Visit from the Capuchin", "Villein"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert_eq!(plays(&req), vec![master(1, Target::Minion(alexa))]);
    // Without that copy, it is playable.
    e.state.cards[mine as usize].in_play = false;
    e.state.layers.clear();
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(plays(&req).contains(&master(0, Target::None)));
}

// [RTR 20180719] [ANK 20200207] [ANK 20210627] Mirror Walk [THA]: "If the
// action is blocked, it ends unsuccessfully immediately, no other action
// modifier or reaction can be played afterwards." [ANK 20191204] "The
// would be blocker is locked ... from Mirror Walk's effect." [ANK
// 20220116] [ANK 20211207] [ANK 20210627] [RTR 19991206] [LSJ 20091125]
// Triggered by block: "all 'if blocked' effects must be applied ... before
// any other card or effect can be used". No combat, and no card in its
// after-resolution window: the acting Methuselah's minion phase goes on;
// the card is replaced as the discard phase begins.
#[test]
fn mirror_walk_superior_ends_the_blocked_action_before_block_resolution() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("THA", 0), 6, false);
    e.add_minion(SeatId(0), find_card("Farah Sarroub"), 6, false);
    let b = e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
    hand(&mut e, 0, &["Mirror Walk"]);
    hand(&mut e, 0, &["Villein"; 6]);
    library(&mut e, 0, "Villein", 3);
    hand(&mut e, 1, &["Eyes of Argus"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(b));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionBlock);
    assert_eq!(plays(&req), vec![play(0, 0, a), play(0, 1, a)]);
    pick(&mut e, play(0, 1, a)); // [THA]: +1 stealth
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert_eq!(e.state.seats[0].hand.len(), 6);
    assert_eq!(e.state.seats[0].held[Release::NextDiscardPhase], 1);
    pick(&mut e, play(0, 0, b)); // Eyes of Argus [aus]: +2 intercept
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
    assert!(locked(&e, b));
    assert_eq!(blood(&e, b), 4);
    assert!(req.options.contains(&Choice::Pass));
    let req = drive(&mut e, |_, req| {
        ((req.seat, req.window) != (SeatId(0), WindowLabel::DiscardPhase)).then(|| fallback(req))
    });
    assert_eq!(req.seat, SeatId(0));
    assert_eq!(e.state.seats[0].hand.len(), 7);
    assert_eq!(e.state.seats[0].held[Release::NextDiscardPhase], 0);
    assert_eq!(e.state.seats[0].library.len(), 2);
}

// [ANK 20190104] [LSJ 19980212] "The canceled card has still been played.
// The same reaction or modifier cannot be played again by the same minion":
// Conditioning canceled by Direct Intervention, the second copy is not
// offered to that vampire for the rest of the action.
#[test]
fn a_canceled_modifier_cannot_be_played_again_by_the_same_minion() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("dom", 0), 4, false);
    e.add_minion(SeatId(0), find_card("Farah Sarroub"), 6, false);
    e.add_minion(SeatId(1), find_card("Ángel Guerrero"), 4, false);
    hand(&mut e, 0, &["Conditioning", "Conditioning"]);
    hand(&mut e, 1, &["Direct Intervention"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, master(0, Target::None)); // canceled
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == a),
    );
    assert_eq!(e.state.seats[0].hand, vec![find_library("Conditioning")]);
}

// [PIB 20121216] Mirror Walk [THA]: "If the action is blocked and it was
// leaving torpor, the blocker does not get an opportunity to diablerize."
#[test]
fn mirror_walk_on_a_blocked_leave_torpor_gives_no_diablerie_opportunity() {
    let mut e = game(2);
    let tor = e.add_minion(SeatId(0), vampire_with("THA", 0), 2, true);
    e.add_minion(SeatId(0), find_card("Farah Sarroub"), 6, false);
    let z = e.add_minion(SeatId(1), find_card("Ángel Guerrero"), 4, false);
    hand(&mut e, 0, &["Mirror Walk"]);
    hand(&mut e, 1, &["Cats' Guidance", "Instinctive Reaction"]);
    pick(&mut e, action(tor, ActionKind::LeaveTorpor));
    pick(&mut e, block(z));
    pick(&mut e, play(0, 1, z)); // +1 intercept: 1 against 1 stealth
    pick(&mut e, play(0, 1, tor)); // Mirror Walk [THA]
    pick(&mut e, play(0, 0, z)); // Instinctive Reaction [ani]: 2 against 2
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
    assert!(in_torpor(&e, tor) && e.state.minions[tor.index()].in_play);
    assert!(locked(&e, z));
}

// [ANK 20210817] A wake "can be played or used during the 'as a card is
// played' window, except in combat, or when there is no action". [TOM
// 19951129] "The minion need not attempt to block nor play further reaction
// cards". [ANK 20200629-2] [LSJ 20091208] [RTR 19951017] "Replace next
// turn": Wake with Evening's Freshness is replaced before its player's
// unlock phase — the hand stays one short until then.
#[test]
fn wake_with_evenings_freshness_wakes_and_waits_for_the_next_unlock_phase() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("DOM", 0), 6, false);
    let b = e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
    e.state.minions[b.index()].locked = true;
    e.at_master_phase();
    hand(&mut e, 0, &["Villein", "Bonding"]);
    hand(
        &mut e,
        1,
        &["Wake with Evening's Freshness", "Sudden Reversal"],
    );
    hand(&mut e, 1, &["Villein"; 5]);
    library(&mut e, 1, "Villein", 3);
    pick(&mut e, master(0, Target::Minion(a)));
    pick(&mut e, Choice::SetX(2));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert_eq!(plays(&req), vec![master(1, Target::None)]);
    pick(&mut e, Choice::Pass);
    pick(&mut e, bleed(a));
    pick(&mut e, play(0, 0, a)); // Bonding [dom]
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert_eq!(plays(&req), vec![play(0, 0, b)]);
    pick(&mut e, play(0, 0, b));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert_eq!(e.state.seats[1].hand.len(), 6);
    assert_eq!(e.state.seats[1].held[Release::NextUnlockPhase], 1);
    assert!(req.options.contains(&block(b)));
    pick(&mut e, Choice::Pass); // woken, need not block
    let _ = drive(&mut e, |e, req| {
        let theirs = (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase)
            && e.state.active == SeatId(1);
        if e.state.active == SeatId(0) {
            assert_eq!(e.state.seats[1].hand.len(), 6, "{req:?}");
        }
        (!theirs).then(|| fallback(req))
    });
    assert_eq!(e.state.seats[1].hand.len(), 7);
    assert_eq!(e.state.seats[1].held[Release::NextUnlockPhase], 0);
    assert_eq!(e.state.seats[1].library.len(), 2);
}

// [LSJ 20061213] A bounce "cannot be used before all blocks have been
// declined."
#[test]
fn deflection_waits_for_all_blocks_to_be_declined() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("DOM", 0), 6, false);
    let c = e.add_minion(SeatId(1), find_card("Chelsea Blake"), 6, false);
    e.add_minion(SeatId(2), find_card("Monica Giovanni"), 6, false);
    hand(&mut e, 1, &["Deflection"]);
    pick(&mut e, bleed(a));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(c)));
    assert!(plays(&req).is_empty());
    pick(&mut e, Choice::Pass);
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlocksDeclined);
    assert!(!plays(&req).is_empty());
}

// [RTR 19941109] [ANK 20190116] "If the action continues 'as if
// unblocked', all action modifiers and reaction cards are still in
// effect": Form of the Cobra's +1 stealth still keeps the predator off
// after Form of Mist.
#[test]
fn continuation_keeps_modifiers() {
    let mut e = game(3);
    let p = e.add_minion(SeatId(0), find_card("Crystal Lynn"), 6, false);
    let x = e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
    let z = e.add_minion(SeatId(2), find_card("Monica Giovanni"), 4, false);
    hand(&mut e, 0, &["Form of the Cobra", "Form of Mist"]);
    hand(&mut e, 1, &["Eyes of Argus"]);
    pick(&mut e, bleed(p));
    pick(&mut e, play(0, 1, p)); // Cobra [PRO]: +1 stealth as announced
    pick(&mut e, block(x));
    pick(&mut e, play(0, 0, x)); // Eyes of Argus [aus]: intercept
    pick(&mut e, play(0, 1, p)); // Form of Mist [PRO]: combat ends, continue
    pick(&mut e, Choice::ContinueAction);
    drive(&mut e, |_, req| {
        assert!(!req.options.contains(&block(z)), "stealth lost: {req:?}");
        (req.window != WindowLabel::MinionPhase).then(|| fallback(req))
    });
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
    assert_eq!(pool(&e), vec![10, 9, 10]);
}

// [LSJ 20070808-1] "For actions resulting from an action card, 'continue
// as if unblocked' moves the action card from the ash heap ... to limbo":
// a continued Constant Revolution enters play, nothing goes to the ash
// heap.
#[test]
fn continued_action_card_leaves_the_ash_heap() {
    let mut e = game(2);
    let crystal = e.add_minion(SeatId(0), find_card("Crystal Lynn"), 6, false); // Anarch, PRO
    e.add_minion(SeatId(0), find_card("Asmin Kobane"), 4, false);
    let x = e.add_minion(SeatId(1), find_card("Ángel Guerrero"), 4, false);
    hand(&mut e, 0, &["Constant Revolution", "Form of Mist"]);
    hand(&mut e, 1, &["Cats' Guidance"]);
    pick(&mut e, play(0, 0, crystal));
    pick(&mut e, block(x));
    pick(&mut e, play(0, 1, x)); // Cats' Guidance [ANI]: +1 intercept
    pick(&mut e, play(0, 1, crystal)); // Form of Mist [PRO]
    pick(&mut e, Choice::ContinueAction);
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
    let cr = e.state.cards.last().expect("Constant Revolution in play");
    assert_eq!(cr.card, find_library("Constant Revolution"));
    assert!(cr.present());
    assert_eq!(cr.counters, 1);
    // Only the combat card went to the ash heap.
    assert_eq!(e.state.seats[0].ash, vec![find_library("Form of Mist")]);
}

// ---- Combat ------------------------------------------------------------------

// [TOM 19960521] "The optional press can only be used during the current
// round": Flash, Hidden Strength, Immortal Grapple and Indomitability each
// bank a press that is offered in their round's press step and gone in
// the next round (reached on the opponent's press).
#[test]
fn optional_press_is_usable_only_in_its_round() {
    struct Case {
        card: &'static str,
        disc: &'static str,
        window: WindowLabel,
        // The opponent's way to a second round: Immortal Grapple's press,
        // or Flash [cel] as a press.
        z_card: &'static str,
        z_disc: &'static str,
        z_loss: u8,
    }
    let cases = [
        Case {
            card: "Flash",
            disc: "CEL",
            window: WindowLabel::CombatDetermineRange,
            z_card: "Flash",
            z_disc: "CEL",
            z_loss: 1, // round 1 at long range
        },
        Case {
            card: "Hidden Strength",
            disc: "FOR",
            window: WindowLabel::CombatPrevention,
            z_card: "Immortal Grapple",
            z_disc: "POT",
            z_loss: 2,
        },
        Case {
            card: "Indomitability",
            disc: "FOR",
            window: WindowLabel::CombatPrevention,
            z_card: "Immortal Grapple",
            z_disc: "POT",
            z_loss: 2,
        },
        Case {
            card: "Immortal Grapple",
            disc: "POT",
            window: WindowLabel::CombatBeforeStrikes,
            z_card: "Flash",
            z_disc: "CEL",
            z_loss: 2,
        },
    ];
    for case in cases {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with(case.disc, 0), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with(case.z_disc, 0), 4, false);
        hand(&mut e, 0, &[case.card]);
        hand(&mut e, 1, &[case.z_card]);
        pick(&mut e, bleed(a));
        pick(&mut e, block(z));
        // Seat 0's press windows: round 1 before z presses, round 1 again
        // after (presses alternate: a may press to end), and no third.
        let mut z_pressed = false;
        let mut a_press_windows = 0;
        drive(&mut e, |_, req| {
            if req.window == WindowLabel::MinionPhase {
                return None;
            }
            match (req.seat, req.window) {
                (SeatId(0), w) if w == case.window && req.options.contains(&play(0, 1, a)) => {
                    Some(play(0, 1, a))
                }
                (SeatId(0), WindowLabel::PlayTerms) => Some(Choice::SetX(0)),
                (SeatId(0), WindowLabel::CombatPress) => {
                    a_press_windows += 1;
                    assert!(
                        a_press_windows <= 2 && req.options.contains(&Choice::UsePress),
                        "{}: press window {} {:?}",
                        case.card,
                        a_press_windows,
                        req.options
                    );
                    Some(Choice::Pass)
                }
                (SeatId(1), WindowLabel::CombatBeforeStrikes)
                    if case.z_card == "Immortal Grapple"
                        && req.options.contains(&play(0, 1, z)) =>
                {
                    Some(play(0, 1, z))
                }
                (SeatId(1), WindowLabel::CombatPress) if !z_pressed => {
                    z_pressed = true;
                    if req.options.contains(&Choice::UsePress) {
                        Some(Choice::UsePress)
                    } else {
                        Some(play(0, 0, z))
                    }
                }
                _ => Some(fallback(req)),
            }
        });
        assert!(z_pressed, "{}: no second round", case.card);
        assert_eq!(a_press_windows, 2, "{}", case.card);
        assert_eq!(blood(&e, z), 4 - case.z_loss, "{}", case.card);
    }
}

// [LSJ 20001114] Immediate damage prevention "cannot be used if there is
// no damage to prevent": at long range the hand strikes do nothing, and
// neither Indomitability nor Soak gets a window.
#[test]
fn immediate_prevention_needs_damage() {
    for card in ["Indomitability", "Soak"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Farah Sarroub"), 4, false);
        let x = e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
        hand(&mut e, 0, &["Flash"]);
        hand(&mut e, 1, &[card]);
        pick(&mut e, bleed(a));
        pick(&mut e, block(x));
        pick(&mut e, play(0, 0, a)); // Flash [cel]: maneuver to long
        drive(&mut e, |_, req| {
            assert_ne!(req.window, WindowLabel::CombatPrevention, "{card}: {req:?}");
            (req.window != WindowLabel::MinionPhase).then(|| fallback(req))
        });
        assert_eq!((blood(&e, a), blood(&e, x)), (4, 4), "{card}");
        assert_eq!(e.state.seats[1].hand.len(), 1, "{card}");
    }
}

// [RTR 20020501] Majesty [PRE], Earth Meld [PRO]: "The vampire still
// unlocks if an effect continues the combat or begins a new combat."
#[test]
fn majesty_and_earth_meld_still_unlock_when_the_combat_continues() {
    for (card, disc, cost) in [("Majesty", "PRE", 1), ("Earth Meld", "PRO", 0)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Monica Giovanni"), 6, false);
        e.add_minion(SeatId(0), find_card("Asmin Kobane"), 4, false);
        let x = e.add_minion(SeatId(1), vampire_with(disc, 0), 6, false);
        hand(&mut e, 0, &["Telepathic Tracking"]);
        hand(&mut e, 1, &[card]);
        let x_start = blood(&e, x);
        pick(&mut e, bleed(a));
        pick(&mut e, block(x));
        pick(&mut e, play(0, 1, x)); // combat ends, unlock
        pick(&mut e, play(0, 1, a)); // Telepathic Tracking [AUS]: a new round
        expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!locked(&e, x), "{card}");
        // Round 2 was fought: hand strikes both ways (the card's cost and
        // Tracking's 1 blood paid).
        assert_eq!(
            (blood(&e, a), blood(&e, x)),
            (6 - 1 - 1, x_start - cost - 1),
            "{card}"
        );
    }
}

// [RTR 20041202] "If fewer points of (preventable) damage are being
// resolved, then the effect prevents all of those points."
#[test]
fn soak_prevents_all_of_fewer_points() {
    let (mut e, a, x, _z) = bleed_table();
    hand(&mut e, 1, &["Soak"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(x));
    pick(&mut e, play(0, 0, x)); // Soak [for]: prevent 2 of 1
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4);
    assert_eq!(blood(&e, a), 5);
}

// [ANK 20200318] [LSJ 20001114] "Unused prevention points can't be carried
// over unless optional": the point of Soak left after the first strike
// does not cover the additional strikes.
#[test]
fn soak_prevention_does_not_carry_over_to_the_next_strike() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Farah Sarroub"), 6, false);
    let x = e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
    hand(&mut e, 0, &["Blur"]);
    hand(&mut e, 1, &["Soak"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(x));
    drive(&mut e, |_, req| match (req.seat, req.window) {
        (SeatId(1), WindowLabel::CombatPrevention) if req.options.contains(&play(0, 0, x)) => {
            Some(play(0, 0, x))
        }
        (SeatId(0), WindowLabel::CombatAdditionalStrikes) => Some(play(0, 1, a)),
        (_, WindowLabel::MinionPhase) => None,
        _ => Some(fallback(req)),
    });
    // Strike 1 prevented; the two additional strikes land.
    assert_eq!(blood(&e, x), 2);
    assert_eq!(blood(&e, a), 6 - 1 - 1); // x's strike, Blur's cost
}

// [RTR 20001020] [LSJ 20001024] [RTR 20030519] Taste of Vitae "must be
// played after the presses are handled and can be played if the round ends
// prematurely": the opponent goes to torpor, the round is over, the card
// is still offered at its end.
#[test]
fn taste_of_vitae_after_the_opponent_goes_to_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("POT", 0), 4, false);
    let x = e.add_minion(SeatId(1), find_card("Monica Giovanni"), 1, false);
    hand(&mut e, 0, &["Roundhouse", "Taste of Vitae"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(x));
    pick(&mut e, play(0, 0, a)); // Roundhouse [pot]: 3 damage
    let req = expect(&mut e, SeatId(0), WindowLabel::CombatEndOfRound);
    assert!(in_torpor(&e, x));
    assert_eq!(plays(&req), vec![play(0, 0, a)]);
    pick(&mut e, play(0, 0, a));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1 + 1);
    assert_eq!(blood(&e, x), 0);
}

// [LSJ 20021113] [ANK 20191219] [ANK 20180910-1] Taste of Vitae "can be
// played before or after effects that are played at the end of the round
// or 'when the combat would end' (eg. Telepathic Tracking)".
#[test]
fn taste_of_vitae_before_or_after_telepathic_tracking() {
    for taste_first in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Monica Giovanni"), 6, false);
        let x = e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
        hand(&mut e, 0, &["Taste of Vitae", "Telepathic Tracking"]);
        pick(&mut e, bleed(a));
        pick(&mut e, block(x));
        // Round 1: hand strikes, 1 each; the press step (Tracking [aus] is
        // a press) declined; end of round: both cards.
        expect(&mut e, SeatId(0), WindowLabel::CombatPress);
        pick(&mut e, Choice::Pass);
        let req = expect(&mut e, SeatId(0), WindowLabel::CombatEndOfRound);
        assert_eq!(plays(&req), vec![play(0, 0, a), play(1, 1, a)]);
        if taste_first {
            pick(&mut e, play(0, 0, a));
            pick(&mut e, play(0, 1, a));
        } else {
            pick(&mut e, play(1, 1, a));
            pick(&mut e, play(0, 0, a));
        }
        drive(&mut e, |_, req| {
            (req.window != WindowLabel::MinionPhase).then(|| fallback(req))
        });
        assert_eq!(blood(&e, x), 4 - 2, "taste first: {taste_first}");
        assert_eq!(
            blood(&e, a),
            6 - 1 + 1 - 1 - 1,
            "taste first: {taste_first}"
        );
    }
}

// [RTR 20010711] Theft of Vitae "can target a minion with less blood or
// life than the amount stolen".
#[test]
fn theft_of_vitae_takes_what_there_is() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Trevon Parker"), 4, false); // THA, capacity 6
    e.add_minion(SeatId(0), find_card("Asmin Kobane"), 4, false);
    let x = e.add_minion(SeatId(1), find_card("Farah Sarroub"), 1, false);
    hand(&mut e, 0, &["Theft of Vitae"]);
    pick(&mut e, bleed(t));
    pick(&mut e, block(x));
    pick(&mut e, play(0, 1, t)); // [THA]: steal 2
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 0);
    assert!(!in_torpor(&e, x));
    assert_eq!(blood(&e, t), 4 + 1 - 1); // x's hand strike lands
}

// [ANK 20200123] [LSJ 20041027] "If both opponents play Theft of Vitae,
// they resolve simultaneously: blood amounts are adjusted by the total of
// gain and loss and no blood wear off" — both at capacity, nothing
// changes.
#[test]
fn simultaneous_thefts_of_vitae_net_out() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Rosalina Cortez"), 3, false); // THA, capacity 3
    e.add_minion(SeatId(0), find_card("Asmin Kobane"), 4, false);
    let d = e.add_minion(SeatId(1), find_card("Abraham DuSable"), 8, false); // THA, capacity 8
    hand(&mut e, 0, &["Theft of Vitae"]);
    hand(&mut e, 1, &["Theft of Vitae"]);
    pick(&mut e, bleed(r));
    pick(&mut e, block(d));
    pick(&mut e, play(0, 1, r));
    pick(&mut e, play(0, 1, d));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, r), blood(&e, d)), (3, 8));
}

// [LSJ 20100813] Torn Signpost sets the base strength and "any '+X
// strength' effect adds to it": Signpost [POT] (3) with Roundhouse [pot]
// (+2) deals 5.
#[test]
fn torn_signpost_sets_the_base_and_hand_strike_bonuses_add() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("POT", 0), 4, false);
    e.add_minion(SeatId(0), find_card("Asmin Kobane"), 4, false);
    let x = e.add_minion(SeatId(1), find_card("Monica Giovanni"), 6, false);
    hand(&mut e, 0, &["Torn Signpost", "Roundhouse"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(x));
    pick(&mut e, play(0, 1, a)); // Torn Signpost [POT] before range
    pick(&mut e, play(0, 0, a)); // Roundhouse [pot]
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 1);
}

// [RBK reaction-cards] Cats' Guidance "cannot be played if the vampire is
// in torpor, for example if he went into torpor during the combat resulting
// from the block".
#[test]
fn cats_guidance_not_playable_from_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("DOM", 0), 6, false);
    let x = e.add_minion(SeatId(1), find_card("Ángel Guerrero"), 0, false);
    hand(&mut e, 1, &["Cats' Guidance"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(x));
    // x takes the hand strike at 0 blood: torpor; no after-combat window
    // (a's turn ends: it was seat 0's only vampire).
    let req = request(&mut e);
    assert_ne!(req.window, WindowLabel::ActionAfterCombat, "{req:?}");
    assert!(in_torpor(&e, x));
    assert_eq!(e.state.seats[1].hand.len(), 1);
}

// ---- Politics ------------------------------------------------------------------

/// Alexa calls Ancilla Empowerment; `script` drives the polling from the
/// caller's first impulse until Delaying Tactics (seat 1, Bret) cancels
/// it; the referendum is then checked to have been canceled.
fn delaying_tactics_at(script: impl FnOnce(&mut Engine, MinionId, MinionId)) {
    let mut t = politics();
    hand(&mut t.e, 0, &["Ancilla Empowerment"]);
    hand(&mut t.e, 1, &["Delaying Tactics"]);
    pick(&mut t.e, play(0, 0, t.alexa));
    decline_futile(&mut t.e);
    let (alexa, kevin, bret) = (t.alexa, t.kevin, t.bret);
    script(&mut t.e, alexa, kevin);
    pick(&mut t.e, play(0, 0, bret));
    expect(&mut t.e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&t.e), vec![10, 10, 10]);
    assert!(!locked(&t.e, alexa));
    assert_eq!(
        t.e.state.seats[0].hand,
        vec![find_library("Ancilla Empowerment")]
    );
}

// [LSJ 19980130] Delaying Tactics "may be played before, during, or after
// votes and ballots have been cast."
#[test]
fn delaying_tactics_is_playable_before_during_and_after_votes() {
    // Before any vote: the caller passes, seat 1 plays it.
    delaying_tactics_at(|e, _, _| pick(e, Choice::Pass));
    // During: after the caller's votes.
    delaying_tactics_at(|e, alexa, _| pick(e, votes(alexa, true)));
    // After every vote: the caller's, Kevin's against, a full rotation.
    delaying_tactics_at(|e, alexa, kevin| {
        pick(e, votes(alexa, true));
        pick(e, votes(kevin, false));
        pick(e, Choice::Pass); // seat 2
        pick(e, Choice::Pass); // seat 0
    });
}

// [LSJ 20010209] After Delaying Tactics "the results are not tallied (eg.
// Scorn of Adonis or Bribes have no effect)".
#[test]
fn delaying_tactics_skips_the_tally() {
    let mut t = politics();
    let catalina =
        t.e.add_minion(SeatId(0), find_card("Catalina Vega"), 5, false); // Toreador
    hand(&mut t.e, 0, &["Ancilla Empowerment", "Scorn of Adonis"]);
    hand(&mut t.e, 1, &["Delaying Tactics"]);
    pick(&mut t.e, play(0, 0, catalina));
    pick(&mut t.e, votes(catalina, true));
    pick(&mut t.e, votes(t.kevin, false));
    pick(&mut t.e, Choice::Pass); // seat 2
    pick(&mut t.e, play(0, 0, catalina)); // Scorn of Adonis
    pick(&mut t.e, play(0, 0, t.bret)); // Delaying Tactics
    expect(&mut t.e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&t.e), vec![10, 10, 10]);
}

// [LSJ 20010209] Telepathic Vote Counting [aus] cancels: "the results are
// not tallied (eg. Scorn of Adonis or Bribes have no effect)".
#[test]
fn telepathic_vote_counting_skips_the_tally() {
    let mut t = politics();
    let catalina =
        t.e.add_minion(SeatId(0), find_card("Catalina Vega"), 5, false); // Toreador, AUS
    hand(
        &mut t.e,
        0,
        &[
            "Ancilla Empowerment",
            "Scorn of Adonis",
            "Telepathic Vote Counting",
        ],
    );
    pick(&mut t.e, play(0, 0, catalina));
    pick(&mut t.e, votes(catalina, true));
    pick(&mut t.e, votes(t.kevin, false));
    pick(&mut t.e, Choice::Pass); // seat 2
    pick(&mut t.e, play(0, 0, catalina)); // Scorn of Adonis
    pick(&mut t.e, Choice::Pass); // seat 2 (seat 1 has nothing left: elided)
    pick(&mut t.e, play(0, 0, catalina)); // Telepathic Vote Counting [aus]
    expect(&mut t.e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&t.e), vec![10, 10, 10]);
}

// [LSJ 19980130] Telepathic Vote Counting "may be played before, during,
// or after votes and ballots have been cast."
#[test]
fn telepathic_vote_counting_cancels_before_during_or_after_votes() {
    for stage in 0..3 {
        let mut t = politics();
        let catalina =
            t.e.add_minion(SeatId(0), find_card("Catalina Vega"), 5, false);
        hand(
            &mut t.e,
            0,
            &["Ancilla Empowerment", "Telepathic Vote Counting"],
        );
        pick(&mut t.e, play(0, 0, catalina));
        match stage {
            // Before any vote: the caller's first impulse.
            0 => {}
            // During: after the caller's votes, once the impulse comes back.
            1 => {
                pick(&mut t.e, votes(catalina, true));
                pick(&mut t.e, Choice::Pass); // seat 1
                pick(&mut t.e, Choice::Pass); // seat 2
            }
            // After every vote.
            _ => {
                pick(&mut t.e, votes(catalina, true));
                pick(&mut t.e, votes(t.kevin, false));
                pick(&mut t.e, votes(t.lenny, false));
            }
        }
        pick(&mut t.e, play(0, 0, catalina)); // [aus]: cancel
        expect(&mut t.e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&t.e), vec![10, 10, 10], "stage {stage}");
    }
}

// [RTR 19951110] Scorn of Adonis "may be played at any time during the
// voting process and, when votes are tallied, will cause the loss of pool
// to Methuselahs voting 'no' before it is played, as well as ... after";
// "each Scorn played may only cause the loss of one pool to a given
// Methuselah, regardless of how many 'no' votes that Methuselah casts".
#[test]
fn scorn_of_adonis_costs_one_pool_per_methuselah_however_many_votes() {
    let mut t = politics();
    let catalina =
        t.e.add_minion(SeatId(0), find_card("Catalina Vega"), 5, false);
    let alice = t.e.add_minion(SeatId(1), find_card("Alice Chen"), 5, false); // a second prince
    hand(&mut t.e, 0, &["Ancilla Empowerment", "Scorn of Adonis"]);
    pick(&mut t.e, play(0, 0, catalina));
    pick(&mut t.e, votes(catalina, true));
    pick(&mut t.e, votes(t.kevin, false)); // before the Scorn
    pick(&mut t.e, votes(t.lenny, false));
    pick(&mut t.e, play(0, 0, catalina)); // Scorn of Adonis
    drive(&mut t.e, |_, req| {
        if req.options.contains(&votes(alice, false)) {
            return Some(votes(alice, false)); // after the Scorn
        }
        (req.window != WindowLabel::MinionPhase).then(|| fallback(req))
    });
    assert_eq!(pool(&t.e), vec![10, 9, 9]);
}

// [LSJ 20000309] "If multiple ousts happen, players whose prey gets ousted
// get a victory point, even if they are being ousted themselves. Only the
// surviving ones get 6 pool from the oust of their prey (not of their
// grand-prey)": each pool-damaging referendum, ousting seats 1 and 2 at
// once in a four-player game.
#[test]
fn multiple_ousts_by_one_referendum() {
    for card in [
        "Anarchist Uprising",
        "Ancilla Empowerment",
        "Camarilla's Iron Fist",
        "Conservative Agitation",
        "Domain Challenge",
        "Kine Resources Contested",
        "Neonate Breach",
        "Reckless Agitation",
    ] {
        let mut e = game(4);
        let alexa = e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
        let saku = e.add_minion(SeatId(0), find_card("Saku Pihlajamäki"), 5, false); // Anarch baron
        let kevin = e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
        let lenny = e.add_minion(SeatId(2), find_card("Lenny Burkhead"), 4, false);
        let bret = e.add_minion(SeatId(3), find_card("Bret Stryker"), 4, false);
        for m in [kevin, lenny, bret] {
            e.state.minions[m.index()].locked = true; // no votes against, no blocks
        }
        e.state.seats[1].pool = 1;
        e.state.seats[2].pool = 1;
        hand(&mut e, 0, &[card]);
        let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let call = plays(&req)[0];
        let caller = match call {
            Choice::Play { actor: Some(m), .. } => m,
            _ => unreachable!(),
        };
        assert!(caller == alexa || caller == saku);
        pick(&mut e, call);
        drive(&mut e, |_, req| {
            if req.window == WindowLabel::MinionPhase {
                return None;
            }
            // Terms: points to seats 1 and 2 (as many as allowed), none to
            // the others; a chosen Methuselah is seat 0 first (Iron
            // Fist's gainer), then seats 1 and 2 (Neonate Breach's
            // victims).
            let allocs: Vec<(SeatId, u8)> = req
                .options
                .iter()
                .filter_map(|o| match o {
                    Choice::AllocateTo { seat, n } => Some((*seat, *n)),
                    _ => None,
                })
                .collect();
            if let Some(&(seat, _)) = allocs.first() {
                let n = if seat == SeatId(1) || seat == SeatId(2) {
                    allocs.iter().map(|a| a.1).max().unwrap()
                } else {
                    allocs.iter().map(|a| a.1).min().unwrap()
                };
                return Some(Choice::AllocateTo { seat, n });
            }
            for s in [0, 1, 2] {
                if req.options.contains(&Choice::ChooseSeat(SeatId(s))) {
                    return Some(Choice::ChooseSeat(SeatId(s)));
                }
            }
            if req.options.contains(&votes(caller, true)) {
                return Some(votes(caller, true));
            }
            if req.options.contains(&Choice::CastCardVote { favor: true }) {
                return Some(Choice::CastCardVote { favor: true });
            }
            Some(fallback(req))
        });
        assert!(e.state.seats[1].ousted && e.state.seats[2].ousted, "{card}");
        assert!(
            !e.state.seats[0].ousted && !e.state.seats[3].ousted,
            "{card}"
        );
        assert_eq!(half_vps(&e), vec![2, 2, 0, 0], "{card}");
        let p = pool(&e);
        assert!(p[0] >= 14, "{card}: seat 0 got no 6 pool: {p:?}");
        assert_eq!(p[1], 0, "{card}");
        assert!(p[3] <= 10, "{card}: seat 3 gained pool: {p:?}");
    }
}

// [LSJ 20000622] Kine Resources Contested and Conservative Agitation:
// "Cannot allocate all X points to a single Methuselah, but you can
// allocate points to yourself"; [LSJ 20000622] [LSJ 20100129] Reckless
// Agitation: "nor any point to yourself"; [ANK 20241106] [LSJ 20100129]
// [LSJ 20000622] Camarilla's Iron Fist: "nor any point to the Methuselah
// gaining pool"; [LSJ 20020819] "Can allocate more points than a
// Methuselah has pool."
#[test]
fn allocation_terms_cannot_go_all_to_one_and_may_exceed_pool() {
    for (card, total, self_allowed) in [
        ("Kine Resources Contested", 4u8, true),
        ("Conservative Agitation", 3, true),
        ("Reckless Agitation", 6, false),
        ("Camarilla's Iron Fist", 5, false),
    ] {
        let mut e = game(3);
        let alexa = e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
        let saku = e.add_minion(SeatId(0), find_card("Saku Pihlajamäki"), 5, false);
        let kevin = e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
        let lenny = e.add_minion(SeatId(2), find_card("Lenny Burkhead"), 4, false);
        for m in [kevin, lenny] {
            e.state.minions[m.index()].locked = true;
        }
        e.state.seats[1].pool = 1;
        hand(&mut e, 0, &[card]);
        let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let call = plays(&req)[0];
        let caller = match call {
            Choice::Play { actor: Some(m), .. } => m,
            _ => unreachable!(),
        };
        assert!(caller == alexa || caller == saku);
        pick(&mut e, call);
        let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
        if self_allowed {
            assert_eq!(
                req.options,
                (0..total).map(|n| alloc(0, n)).collect::<Vec<_>>(),
                "{card}"
            );
            pick(&mut e, alloc(0, 0));
            let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
            assert_eq!(
                req.options,
                (1..total).map(|n| alloc(1, n)).collect::<Vec<_>>(),
                "{card}"
            );
        } else {
            assert_eq!(
                req.options,
                (1..total).map(|n| alloc(1, n)).collect::<Vec<_>>(),
                "{card}"
            );
        }
        // Seat 1 has 1 pool and takes all but one point; the last goes to
        // seat 2 without a decision.
        pick(&mut e, alloc(1, total - 1));
        pick(&mut e, votes(caller, true));
        close_polling(&mut e);
        assert!(e.state.seats[1].ousted, "{card}");
        assert_eq!(pool(&e)[2], 9, "{card}");
    }
}

// [LSJ 20100129] [ANK 20191228] A political action "cannot be used or
// played if the conditions for the terms of the referendum cannot be met":
// with a single opponent Reckless Agitation and Camarilla's Iron Fist are
// masked, Parity Shift with nobody richer, Banishment with no younger
// vampire; each is offered once its terms are satisfiable.
#[test]
fn political_actions_with_unmeetable_terms_are_not_offered() {
    let mut e = game(2);
    let alexa = e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false); // capacity 8
                                                                              // Saku: an Anarch baron of capacity 8 — Reckless Agitation's legal
                                                                              // actor, not younger than anyone, no prince or Camarilla title.
    let saku = e.add_minion(SeatId(0), find_card("Saku Pihlajamäki"), 5, false);
    e.add_minion(SeatId(1), find_card("Abraham DuSable"), 5, false); // capacity 8
    e.state.seats[1].pool = 5;
    hand(
        &mut e,
        0,
        &[
            "Banishment",
            "Camarilla's Iron Fist",
            "Parity Shift",
            "Reckless Agitation",
        ],
    );
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(plays(&req).is_empty(), "{:?}", plays(&req));
    e.state.seats[1].pool = 20;
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(plays(&req), vec![play(2, 0, alexa)]); // Parity Shift
    e.add_minion(SeatId(1), find_card("Mateo García"), 2, false);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let mut offered = plays(&req);
    offered.sort_by_key(|o| format!("{o:?}"));
    let mut want = vec![play(0, 0, alexa), play(0, 0, saku), play(2, 0, alexa)];
    want.sort_by_key(|o| format!("{o:?}"));
    assert_eq!(offered, want);
}

// [ANK 20221019-3] Banishment "is not directed": prey and predator may
// both attempt to block (each holds the intercept for the political
// action's +1 stealth, rules-qa 6.2).
#[test]
fn banishment_is_undirected() {
    let mut t = politics();
    let trung =
        t.e.add_minion(SeatId(1), find_card("Trung Chau Pham"), 3, false); // ANI
    hand(&mut t.e, 0, &["Banishment"]);
    hand(&mut t.e, 1, &["Cats' Guidance"]);
    hand(&mut t.e, 2, &["Cats' Guidance"]);
    pick(&mut t.e, play(0, 0, t.alexa));
    let req = expect(&mut t.e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(trung)));
    pick(&mut t.e, Choice::Pass);
    let req = expect(&mut t.e, SeatId(2), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(t.lenny)));
}

// [ANK 20200408-2] [RTR 20000501] A banished vampire "is placed in their
// last permanent controller's uncontrolled region ... It is removed from
// the game when the controller is ousted."
#[test]
fn banished_vampire_is_removed_when_its_controller_is_ousted() {
    let mut t = politics();
    hand(&mut t.e, 0, &["Banishment", "Ancilla Empowerment"]);
    pick(&mut t.e, play(0, 0, t.alexa));
    pick(&mut t.e, Choice::ChooseMinion(t.bret));
    pick(&mut t.e, votes(t.alexa, true));
    close_polling(&mut t.e);
    expect(&mut t.e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(t.e.state.seats[1].uncontrolled.len(), 1);
    // Isolating the oust: Alexa freed by hand for a second referendum,
    // seat 1 down to its last pool.
    t.e.state.minions[t.alexa.index()].locked = false;
    t.e.state.minions[t.alexa.index()].political_this_turn = false;
    t.e.state.seats[1].pool = 1;
    pick(&mut t.e, play(0, 0, t.alexa));
    pick(&mut t.e, votes(t.alexa, true));
    close_polling(&mut t.e);
    assert!(t.e.state.seats[1].ousted);
    assert!(t.e.state.seats[1].uncontrolled.is_empty());
}

// [LSJ 20010809-3] [LSJ 20010809-2] [TOM 19960210] [RTR 20000501] A
// banished vampire "will remember all effects applied to them if they are
// influenced back. This includes gained or lost titles": a prince who
// yielded his title comes back without it.
#[test]
fn banished_vampire_remembers_a_lost_title() {
    let mut t = politics();
    t.e.state.minions[t.kevin.index()].title_lost = true;
    t.e.state.minions[t.kevin.index()].blood = 7; // capacity: fully funded
    hand(&mut t.e, 0, &["Banishment"]);
    pick(&mut t.e, play(0, 0, t.alexa));
    pick(&mut t.e, Choice::ChooseMinion(t.kevin));
    pick(&mut t.e, votes(t.alexa, true));
    close_polling(&mut t.e);
    assert!(!t.e.state.minions[t.kevin.index()].in_play);
    to_window(&mut t.e, SeatId(1), WindowLabel::InfluenceTransfers);
    pick(&mut t.e, Choice::InfluenceOut { slot: 0 });
    let k = &t.e.state.minions[t.kevin.index()];
    assert!(k.in_play && k.title_lost && !k.title_contested);
}

// [LSJ 20031115] Protected District "cannot be used by a minion who is
// voting in favor of the referendum".
#[test]
fn protected_district_cannot_be_used_by_a_minion_voting_in_favor() {
    let mut t = politics();
    hand(&mut t.e, 0, &["Ancilla Empowerment"]);
    hand(&mut t.e, 2, &["Protected District"]);
    pick(&mut t.e, play(0, 0, t.alexa));
    pick(&mut t.e, votes(t.alexa, true));
    pick(&mut t.e, Choice::Pass); // seat 1
    let req = expect(&mut t.e, SeatId(2), WindowLabel::ReferendumPolling);
    assert!(plays(&req).contains(&play(0, 0, t.lenny)));
    pick(&mut t.e, votes(t.lenny, true));
    let lenny = t.lenny;
    drive(&mut t.e, |_, req| {
        assert!(!plays(req).contains(&play(0, 0, lenny)), "{req:?}");
        (req.window != WindowLabel::MinionPhase).then(|| fallback(req))
    });
    assert_eq!(t.e.state.seats[2].hand.len(), 1);
}

// [RTR 19951110] Voter Captivation "is played after the referendum is
// resolved. If the effect of the referendum ousts the acting Methuselah,
// there is no time to play it to prevent the ousting."
#[test]
fn voter_captivation_has_no_window_when_the_referendum_ousts_the_caller() {
    let mut t = politics();
    t.e.state.seats[0].pool = 2;
    hand(&mut t.e, 0, &["Ancilla Empowerment", "Voter Captivation"]);
    pick(&mut t.e, play(0, 0, t.alexa));
    pick(&mut t.e, votes(t.alexa, true));
    close_polling(&mut t.e);
    let req = request(&mut t.e);
    assert!(t.e.state.seats[0].ousted);
    assert_eq!(req.seat, SeatId(1));
    // Seat 2 burned 1 for its minion and took the 6 pool of the oust.
    assert_eq!(pool(&t.e), vec![0, 8, 15]);
}

// ---- Slice 11e: titles and votes ---------------------------------------------

// [ANK 20190606-2] Power Structure "cannot be used during a Blood Hunt
// referendum, since there is no political action" — while Ventrue
// Headquarters, "during the polling step of any referendum", can.
#[test]
fn power_structure_is_not_usable_during_a_blood_hunt() {
    let mut e = Engine::bare(3);
    let alexa = e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    e.add_minion(SeatId(0), find_card("Braden White"), 4, false); // Lasombra
    let victim = e.add_minion(SeatId(1), find_card("Abraham DuSable"), 2, true);
    e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
    e.add_minion(SeatId(2), find_card("Lenny Burkhead"), 4, false);
    let ps = in_play(&mut e, "Power Structure", 0, None);
    let hq = in_play(&mut e, "Ventrue Headquarters", 1, None);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: alexa,
            action: ActionKind::Diablerie,
            target: Some(victim),
        },
    );
    pick(&mut e, Choice::Pass); // seat 1 declines to block
    let req = expect(&mut e, SeatId(0), WindowLabel::BloodHuntPolling);
    assert!(uses(&req, ps).is_empty(), "no political action");
    pick(&mut e, Choice::Pass);
    let req = expect(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
    assert_eq!(
        uses(&req, hq),
        vec![ability(hq, 0, Target::None), ability(hq, 1, Target::None)]
    );
}

// [LSJ 20060409] Allies who can play as a vampire: "If they takes
// aggravated damage from a card they plays as a vampire (eg. Burst of
// Sunlight), they lose one life but do not go to torpor." Vivienne Géroux
// plays Daring the Dawn [for] as an Anarch vampire: its 2 aggravated
// damage costs her 2 of her 3 life; she stays in play.
#[test]
fn vivienne_loses_life_to_her_own_daring_the_dawn() {
    let mut e = Engine::bare(2);
    let v = e.add_ally(SeatId(0), find_library("Vivienne Géroux"));
    e.state.minions[v.index()].blood = 3;
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Daring the Dawn"]);
    pick(&mut e, bleed(v));
    pick(&mut e, play(0, 0, v));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let m = &e.state.minions[v.index()];
    assert!(m.in_play && !m.in_torpor, "an ally does not go to torpor");
    assert_eq!(m.blood, 1, "2 aggravated damage, 2 life");
}

// ---- Rows whose mentioned card entered the pool later -----------------------

/// The `nth` vampire matching `f` with no printed text implemented (a plain
/// fixture).
fn crypt_where(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    zvtes::crypt::CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| f(c) && zvtes::cards::vampire(*i as u16).is_none())
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a plain vampire")
}

/// `m` gains `disc` (a free beast slot: lower case basic, upper superior).
fn grant(e: &mut Engine, m: MinionId, disc: &'static str) {
    let slots = &mut e.state.minions[m.index()].beast_disciplines;
    let free = slots.iter().position(Option::is_none).expect("a free slot");
    slots[free] = Some(disc);
}

/// A card already in play with `mode`'s layers (no play).
fn equipped(e: &mut Engine, name: &str, owner: u8, host: Option<MinionId>, mode: usize) -> u32 {
    let id = in_play(e, name, owner, host);
    let card = find_library(name);
    for effect in zvtes::cards::lookup(card).expect("definition").modes[mode].effects {
        if let zvtes::cards::Effect::Layer(modifier) = effect {
            e.state.layers.push(Layer {
                scope: Scope::InPlay,
                modifier: *modifier,
                source: CardId(id),
            });
        }
    }
    id
}

fn has(c: &zvtes::crypt::CryptCard, disc: &str) -> bool {
    c.disciplines.contains(&disc)
}

fn use_at(card: u32, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(CardId(card)),
        ability,
        actor,
        target,
    }
}

fn play_at(card: u8, mode: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target,
    }
}

// Concoction of Vitality [LSJ 20100402] "Can be used after declaring an
// action using a discipline (eg. Govern the Unaligned)": the Concoction
// burned as Govern the Unaligned's bleed is announced — no vampire blocks,
// the bleed at +2 all the same.
#[test]
fn concoction_of_vitality_burns_after_a_discipline_action_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("dom", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cv = in_play(&mut e, "Concoction of Vitality", 0, Some(a));
    hand(&mut e, 0, &["Govern the Unaligned"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, use_at(cv, 0, Some(a), Target::None));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(z));
    assert!(!e.state.cards[cv as usize].in_play);
    assert_eq!(pool(&e)[1], 10 - 3);
}

// Death Seeker [ANK 20180910-2] [ANK 20180910-3] "Can be used to cancel a
// card played at the end of a round (eg. Disarm) or 'when combat would
// end' (eg. Psyche!): it then counts as having been played during that
// ending round." The blocker's .44 Magnum outdamages the Salubri; its
// Disarm at the round's end is canceled (no torpor), and its Psyche! next:
// the Salubri's second Death Seeker is not offered — one this round. Then,
// alone, a Psyche! canceled: no second combat.
#[test]
fn death_seeker_cancels_disarm_and_psyche_at_the_ending_round() {
    for disarm in [true, false] {
        let mut e = game(2);
        // Adonai's text (+1 bleed; +1 strength against a Tremere) is inert.
        let a = e.add_minion(SeatId(0), find_card("Adonai"), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with("pot", 0), 6, false);
        grant(&mut e, z, "CEL");
        let magnum = in_play(&mut e, ".44 Magnum", 1, Some(z));
        hand(&mut e, 0, &["Death Seeker", "Death Seeker"]);
        if disarm {
            hand(&mut e, 1, &["Disarm", "Psyche!"]);
        } else {
            hand(&mut e, 1, &["Psyche!"]);
        }
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, ability(magnum, 0, Target::None));
        let seeker = play(0, 0, a);
        if disarm {
            let req = offering(&mut e, play(0, 0, z));
            assert_eq!(req.window, WindowLabel::CombatEndOfRound);
            pick(&mut e, play(0, 0, z));
            pick(&mut e, seeker);
            assert!(!in_torpor(&e, a), "Disarm canceled");
        }
        pick_when(&mut e, play(0, 1, z));
        if disarm {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, seeker);
        } else {
            pick(&mut e, seeker);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, a), 6 - 1 - 2, "one round: no second combat");
            assert!(e.state.seats[1].ash.contains(&find_library("Psyche!")));
        }
    }
}

// Dual Form [ANK 20170226] "[PRO] The disciplines are copied when the action
// resolves, including any additional disciplines (eg. Seal of Veddartha).
// Disciplines gained after the action are not copied." The Seal's two
// counters' Dominate copied; its next counters' Fortitude, after, not.
#[test]
fn dual_form_copies_the_levels_a_seal_of_veddartha_grants() {
    let mut e = game(2);
    let base = crypt_where(
        |c| {
            c.capacity >= 7
                && has(c, "PRO")
                && !c
                    .disciplines
                    .iter()
                    .any(|d| ["dom", "DOM", "for", "FOR"].contains(d))
        },
        0,
    );
    let a = e.add_minion(SeatId(0), base, 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let seal = equipped(&mut e, "Seal of Veddartha", 0, Some(a), 0);
    e.state.cards[seal as usize].counters = 2;
    hand(&mut e, 0, &["Dual Form"]);
    pick(&mut e, play_at(0, 1, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let copy = MinionId(e.state.minions.len() as u32 - 1);
    assert_eq!(e.state.discipline_level(copy, "dom"), 2, "the Seal's two");
    e.state.cards[seal as usize].counters = 4;
    assert_eq!(e.state.discipline_level(a, "for"), 2);
    assert_eq!(e.state.discipline_level(copy, "for"), 0, "gained after");
}

// Guardian Vigil [LSJ 20010814-2] [ANK 20181127] "All effects apply for the
// duration of the action. They still apply if the action 'continues as if
// unblocked' (eg. [PRO] Form of Mist) and, if [aus] was used, the maneuver
// is provided again." (The maneuver is [cel]'s today.) The Anarch blocks
// with all three Disciplines; Form of Mist [PRO] continues the bleed with
// +1 stealth; the Anarch, unlocked by [for], attempts again — its +1
// intercept reaches the stealth — and gets the maneuver in the new combat.
#[test]
fn guardian_vigil_lasts_through_a_continued_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("PRO", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let anarch = crypt_where(
        |c| {
            c.sect == Some(zvtes::crypt::Sect::Anarch)
                && !c
                    .disciplines
                    .iter()
                    .any(|d| ["aus", "AUS", "cel", "CEL", "for", "FOR"].contains(d))
        },
        0,
    );
    let z = e.add_minion(SeatId(1), anarch, 6, false);
    for d in ["aus", "cel", "for"] {
        grant(&mut e, z, d);
    }
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Form of Mist"]);
    hand(&mut e, 1, &["Guardian Vigil"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 6, z));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&Choice::UseManeuver));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, Choice::ContinueAction);
    assert!(!locked(&e, z), "[for]: unlocked");
    let req = offering(&mut e, block(z));
    assert!(e.block_can_reach(req.seat, z), "the +1 intercept still");
    pick(&mut e, block(z));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&Choice::UseManeuver), "again");
}

// Guardian Vigil [LSJ 20010813] [LSJ 20010819-2] "The maneuver can only be
// used if the block is successful, and only in the resulting (block-
// induced) combat. It does not carry over to a follow-up combat (eg. [CEL]
// Psyche!)." Unused in the block's combat, it is not offered in the
// combat Psyche! queues.
#[test]
fn guardian_vigils_maneuver_is_not_offered_after_psyche() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let anarch = crypt_where(
        |c| c.sect == Some(zvtes::crypt::Sect::Anarch) && has(c, "cel"),
        0,
    );
    let z = e.add_minion(SeatId(1), anarch, 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Psyche!"]);
    hand(&mut e, 1, &["Guardian Vigil"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, z));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&Choice::UseManeuver));
    pick_when(&mut e, play(0, 1, a));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::UseManeuver,
    );
    assert!(e.state.seats[0].ash.contains(&find_library("Psyche!")));
}

// Jake Washington [LSJ 20030520-2] "His 'starting life amount' is 1
// (Vagabond Mystic cannot give him more life)": the Mystic's lock offered
// on a hurt Double Deuce, never on Jake.
#[test]
fn vagabond_mystic_gives_jake_washington_no_life() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let mystic = e.add_ally(SeatId(0), find_library("Vagabond Mystic"));
    let deuce = e.add_ally(SeatId(0), find_library("Double Deuce"));
    e.state.minions[deuce.index()].blood = 1;
    hand(&mut e, 0, &["Jake Washington"]);
    e.at_master_phase();
    pick(&mut e, master(0, Target::None));
    // The Mystic's lock is offered from the master phase on.
    let id = e.state.minions[mystic.index()].ally_id().unwrap();
    let heal = |t| ability(id.0, 0, Target::Minion(t));
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let jake = MinionId(e.state.minions.len() as u32 - 1);
    assert_eq!(e.state.ally_body(jake).map(|b| b.life), Some(1));
    assert!(req.options.contains(&heal(deuce)));
    assert!(!req.options.contains(&heal(jake)));
}

// Loki's Gift [ANK 20220809] "[PRO] The blood burned is not affected by
// effects modifying the hunt amount (eg. Ariadne)": Ariadne's [PRO] hunt
// gains her 2, the Ventrue burns 1.
#[test]
fn lokis_gift_burns_one_blood_whatever_ariadnes_hunt() {
    let mut e = game(2);
    let ari = e.add_minion(SeatId(0), find_card("Ariadne"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Loki's Gift"]);
    pick(&mut e, play_at(0, 1, ari, Target::Minion(z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ari), 3 + 2, "hunt +1");
    assert_eq!(blood(&e, z), 4 - 1);
}

// The Meddling of Semsith [ANK 20180318] [ANK 20231229] "Cards that are not
// replaced count against the hand size (eg. successive combats against
// Raptor stack the non-replaced cards)." Seat 1, the Meddling's chosen,
// fights the Raptor [ANI]'s employer twice in one action (Psyche!): each
// combat takes 1 hand size (a discard down), each Target Vitals its
// vampire plays is held; drawn up between and after the combats to the
// hand size less the cards held — 5 in hand, 2 held.
#[test]
fn meddling_stacks_the_cards_held_over_successive_raptor_combats() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let raptor = equipped(&mut e, "Raptor", 0, Some(a), 1);
    e.state.cards[raptor as usize].counters = 1;
    let meddling = equipped(&mut e, "Meddling of Semsith, The", 0, None, 0);
    e.state.cards[meddling as usize].against = zvtes::ids::SeatSet::single(SeatId(1));
    hand(&mut e, 0, &["Psyche!"]);
    hand(&mut e, 1, &["Target Vitals", "Target Vitals"]);
    fill_hand(&mut e, 1);
    library(&mut e, 1, "Conceal", 5);
    let vitals = find_library("Target Vitals");
    let (mut discards, mut aims) = (0, 0);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    drive(&mut e, |e, req| {
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            return None;
        }
        if req.window == WindowLabel::DiscardDown {
            discards += 1;
            let hand = &e.state.seats[1].hand;
            let i = hand.iter().position(|&c| c != vitals).unwrap();
            return Some(Choice::Discard(i as u8));
        }
        let aim = play(0, 0, z);
        if req.options.contains(&aim) && e.state.seats[1].hand[0] == vitals {
            aims += 1;
            return Some(aim);
        }
        if req.options.contains(&play(0, 1, a)) {
            return Some(play(0, 1, a));
        }
        Some(fallback(req))
    });
    assert_eq!((discards, aims), (2, 2));
    assert_eq!(e.state.seats[1].held[Release::MinionPhaseEnd], 2);
    assert_eq!(e.state.seats[1].hand.len(), 7 - 2);
}

// Oubliette [LSJ 20080226] "[OBT] Does not inflict damage (eg. does not
// benefit from Target Vitals)": aimed with Target Vitals, the Oubliette
// [OBT] still burns 1 blood, not 3.
#[test]
fn target_vitals_adds_nothing_to_an_oubliette() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 4, false);
    grant(&mut e, a, "obt");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    hand(&mut e, 0, &["Target Vitals", "Oubliette"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    // The strike first, the aim on it (rules-qa 55.8).
    pick_when(&mut e, play(1, 1, a));
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Target Vitals")));
    assert_eq!(blood(&e, z), 6 - 1);
}

// Powerbase: Savannah [ANK 20230814] "Takes into account effects that change
// who is considered the acting vampire in combat (eg. Deep Song)": Deep
// Song [ANI]'s target is the acting minion of its combat — its Methuselah's
// Savannah ends the combat; the Deep Song player's own Savannah is not
// offered.
#[test]
fn powerbase_savannah_ends_a_deep_song_combat_for_its_target() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), vampire_with("ANI", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let ours = in_play(&mut e, "Powerbase: Savannah", 0, None);
    in_play(&mut e, "Arcane Library", 0, None);
    let theirs = in_play(&mut e, "Powerbase: Savannah", 1, None);
    let museum = in_play(&mut e, "Art Museum", 1, None);
    hand(&mut e, 0, &["Deep Song"]);
    pick(&mut e, play_at(0, 1, d, Target::Minion(t)));
    let end = ability(theirs, 0, Target::Card(CardId(museum)));
    let ours_offered = |o: &Choice| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if c.0 == ours);
    let req = drive(&mut e, |_, req| {
        assert!(!req.options.iter().any(ours_offered), "{:?}", req.options);
        (!req.options.contains(&end)).then(|| fallback(req))
    });
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::CombatBeforeRange)
    );
    pick(&mut e, end);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, d), blood(&e, t)), (4, 4), "no strikes");
}

// Pulled Fangs [LSJ 20021113] [ANK 20191219] [ANK 20180910-1] "Can be played
// before or after effects that are played at the end of the round or 'when
// the combat would end' (eg. [AUS] Telepathic Tracking). Can be played
// before or after [CEL] Psyche!": a Sawed-Off Shotgun outdamages the
// blocker; at the round's end, either card first, the other still offered.
#[test]
fn pulled_fangs_before_or_after_psyche_or_telepathic_tracking() {
    for other in ["Psyche!", "Telepathic Tracking"] {
        for fangs_first in [true, false] {
            let mut e = game(2);
            let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 6, false);
            grant(&mut e, a, "AUS");
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
            let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
            let gun = in_play(&mut e, "Sawed-Off Shotgun", 0, Some(a));
            hand(&mut e, 0, &["Pulled Fangs", other]);
            pick(&mut e, bleed(a));
            pick_when(&mut e, block(z));
            pick_when(&mut e, ability(gun, 0, Target::None));
            let (fangs, x) = (play(0, 0, a), play(1, 1, a));
            let req = until(&mut e, SeatId(0), WindowLabel::CombatEndOfRound);
            assert!(
                req.options.contains(&fangs) && req.options.contains(&x),
                "{other}"
            );
            let then = if fangs_first {
                e.choose(index_of(&req, fangs));
                play(0, 1, a)
            } else {
                e.choose(index_of(&req, x));
                fangs
            };
            let req = request(&mut e);
            assert!(
                req.options.contains(&then),
                "{other}, fangs first {fangs_first}"
            );
        }
    }
}

// Righteous Aura [ANK 20180511-1] "The card is burned before the vampire
// leaves the ready region (eg. it is burned if Banishment is used on the
// vampire it is on)": Bret banished, the Aura on him goes to the ash heap,
// not with him to the uncontrolled region.
#[test]
fn righteous_aura_burns_as_its_vampire_is_banished() {
    let mut t = politics();
    let aura = in_play(&mut t.e, "Righteous Aura", 1, Some(t.bret));
    // As he played it.
    t.e.state.cards[aura as usize].by = Some(t.bret);
    hand(&mut t.e, 0, &["Banishment"]);
    pick(&mut t.e, play(0, 0, t.alexa));
    pick(&mut t.e, Choice::ChooseMinion(t.bret));
    pick(&mut t.e, votes(t.alexa, true));
    close_polling(&mut t.e);
    expect(&mut t.e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!t.e.state.minions[t.bret.index()].in_play, "banished");
    assert!(!t.e.state.cards[aura as usize].in_play);
    assert!(t.e.state.seats[1]
        .ash
        .contains(&find_library("Righteous Aura")));
}

/// A Hermana Hambrienta Mayor (Mexico City Circle) for `seat`.
fn hermana(e: &mut Engine, seat: u8, blood: u8) -> MinionId {
    e.add_minion(
        SeatId(seat),
        find_card("Hermana Hambrienta Mayor"),
        blood,
        false,
    )
}

// Slake the Thirst [LSJ 20100206-2] "[san] Can be played multiple times on
// the same blood gain (eg. Taste of Vitae)": a Hermana's Taste of Vitae
// gains her the 1 blood her blocker lost; Slake played twice on that gain,
// one for each other Hermana.
#[test]
fn slake_the_thirst_twice_on_a_taste_of_vitae() {
    let mut e = game(2);
    let a = hermana(&mut e, 0, 3);
    let b = hermana(&mut e, 0, 2);
    let c = hermana(&mut e, 0, 2);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(
        &mut e,
        0,
        &["Taste of Vitae", "Slake the Thirst", "Slake the Thirst"],
    );
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    let req = offering(&mut e, play_at(0, 0, a, Target::Minion(b)));
    assert_eq!(blood(&e, a), 3 - 1 + 1);
    e.choose(index_of(&req, play_at(0, 0, a, Target::Minion(b))));
    pick_when(&mut e, play_at(0, 0, a, Target::Minion(c)));
    request(&mut e);
    assert_eq!((blood(&e, b), blood(&e, c)), (3, 3));
}

// Slake the Thirst [LSJ 20100325] "[SAN] Can be played at any time blood is
// gained during an action, including after action resolution (eg. Games of
// Instinct, Perfectionist)": the Hermana's Perfectionist gain after her
// unblocked bleed; Slake [SAN] on it, after resolution.
#[test]
fn slake_the_thirst_on_a_perfectionist_gain_after_resolution() {
    let mut e = game(2);
    let a = hermana(&mut e, 0, 2);
    grant(&mut e, a, "san");
    let b = hermana(&mut e, 0, 2);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let perf = in_play(&mut e, "Perfectionist", 0, Some(a));
    hand(&mut e, 0, &["Slake the Thirst"]);
    pick(&mut e, bleed(a));
    until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    pick(&mut e, ability(perf, 0, Target::None));
    assert_eq!(blood(&e, a), 3);
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    e.choose(index_of(&req, play_at(0, 2, a, Target::Minion(b))));
    request(&mut e);
    assert_eq!(blood(&e, b), 3);
}

fn vampire_ability(m: MinionId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: None,
        target: Target::None,
    }
}

// Andre LeRoux [LSJ 20081123] "It is used in the 'would/about to be
// successful' window like Spying Mission": an unblocked bleed by another of
// his Methuselah's vampires — Spying Mission [OBF] and his reduction offered
// in the one window.
#[test]
fn andre_leroux_is_offered_where_spying_mission_is() {
    let mut e = game(2);
    let andre = e.add_minion(SeatId(0), find_card("Andre LeRoux"), 4, false);
    let spy = e.add_minion(SeatId(0), vampire_with("OBF", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Spying Mission"]);
    pick(&mut e, bleed(spy));
    let req = offering(&mut e, play(0, 1, spy));
    assert!(
        req.options.contains(&vampire_ability(andre, 0)),
        "{:?}",
        req.options
    );
}

// Street Cred [LSJ 20021113] [ANK 20191219] [ANK 20180910-1] "Can be played
// before or after effects that are played at the end of the round or 'when
// the combat would end' (eg. [AUS] Telepathic Tracking)": the blocker's .44
// Magnum outdamages the hand strike; at the round's end, either card first,
// the other still offered.
#[test]
fn street_cred_before_or_after_telepathic_tracking() {
    for cred_first in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with("POT", 0), 6, false);
        grant(&mut e, z, "AUS");
        let cap =
            zvtes::crypt::CRYPT[e.state.minions[z.index()].crypt_card().unwrap() as usize].capacity;
        let young = crypt_where(|c| c.capacity < cap, 0);
        e.state.seats[1]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card: young,
                copy: Default::default(),
                counters: 1,
                banished: None,
            });
        let gun = in_play(&mut e, ".44 Magnum", 1, Some(z));
        hand(&mut e, 1, &["Street Cred", "Telepathic Tracking"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, ability(gun, 0, Target::None));
        let cred = play_at(0, 0, z, Target::Uncontrolled(0));
        let tracking = play(1, 1, z);
        let req = until(&mut e, SeatId(1), WindowLabel::CombatEndOfRound);
        assert!(req.options.contains(&cred) && req.options.contains(&tracking));
        let then = if cred_first {
            e.choose(index_of(&req, cred));
            play(0, 1, z)
        } else {
            e.choose(index_of(&req, tracking));
            cred
        };
        let req = request(&mut e);
        assert!(req.options.contains(&then), "cred first {cred_first}");
    }
}

// Asguresh [LSJ 20100206] [LSJ 20030224] "If his ability is used to cancel
// a limited effect (additional strike), then the limit is not triggered and
// another limited effect can be used": Blur canceled (unpaid), Lightning
// Reflexes still offered for the round's limited additional strike.
#[test]
fn asguresh_canceling_blur_leaves_the_limited_strike_unused() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    grant(&mut e, a, "cel");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Asguresh"), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Blur", "Lightning Reflexes"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(g));
    let req = offering(&mut e, play(0, 0, a));
    let before = blood(&e, a);
    e.choose(index_of(&req, play(0, 0, a)));
    pick_when(&mut e, vampire_ability(g, 0));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(blood(&e, a), before, "Blur unpaid");
    assert!(e.state.seats[0].ash.contains(&find_library("Blur")));
    e.choose(index_of(&req, play(0, 0, a)));
}

// Asguresh [RTR 20001020] "Can only cancel minion cards played from the hand
// in the normal fashion (eg. not weapons played via Disguised Weapon)": his
// cancel offered on the Disguised Weapon, declined; the .44 Magnum it equips
// is not offered to it.
#[test]
fn asguresh_cannot_cancel_the_weapon_a_disguised_weapon_equips() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Asguresh"), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Disguised Weapon", ".44 Magnum"]);
    let cancel = vampire_ability(g, 0);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(g));
    pick_when(&mut e, play(0, 0, a));
    let req = offering(&mut e, cancel);
    e.choose(index_of(&req, Choice::Pass));
    never_offered_until(&mut e, SeatId(0), WindowLabel::CombatStrike, cancel);
    let magnum = find_library(".44 Magnum");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == magnum && c.host == Some(a)));
}

// The Beast, The Leatherface of Detroit [LSJ 20050315] [LSJ 20050413-1] [LSJ
// 20050413-2] "He can equip with an equipment that is a location when in
// play, but not by playing the card normally as it is an action card. (eg.
// Using Vast Wealth would work)." Palatial Estate in hand is not his to
// play; Vast Wealth's equip action finds it in the library and he equips it.
#[test]
fn the_beast_equips_a_location_through_vast_wealth() {
    let mut e = game(2);
    let beast = e.add_minion(
        SeatId(0),
        find_card("Beast, The Leatherface of Detroit"),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let wealth = in_play(&mut e, "Vast Wealth", 0, Some(beast));
    hand(&mut e, 0, &["Palatial Estate"]);
    fill_hand(&mut e, 0);
    library(&mut e, 0, "Palatial Estate", 1);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!plays(&req)
        .iter()
        .any(|o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == beast)));
    pick(&mut e, use_at(wealth, 0, Some(beast), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let estate = find_library("Palatial Estate");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == estate && c.host == Some(beast)));
}

// The Capuchin [ANK 20200417] "If he burns, he looses all cards and counters
// on him, all cards and effects lose track of him and he is considered a
// new vampire for all game purposes (eg. The Rack does not target him
// anymore)": The Rack chose him; burned, he goes back to the uncontrolled
// region, is influenced out again — and The Rack feeds nobody at the next
// unlock phase.
#[test]
fn the_rack_loses_track_of_a_burned_capuchin() {
    let mut e = game(2);
    let capuchin = find_card("Capuchin, The");
    let c = e.add_minion(SeatId(0), capuchin, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let rack = in_play(&mut e, "Rack, The", 0, None);
    e.state.cards[rack as usize].chosen = vec![c];
    e.burn(c);
    e.state.seats[0].uncontrolled[0].counters = 11;
    to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    let back = MinionId(e.state.minions.len() as u32 - 1);
    let feed = ability(rack, 0, Target::None);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, feed);
    let req = request(&mut e);
    e.choose(index_of(&req, Choice::Pass));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, feed);
    let v = &e.state.minions[back.index()];
    assert!(
        v.in_play && v.crypt_card() == Some(capuchin),
        "a new Capuchin"
    );
}

// Gratiano [LSJ 20040519] "If Gratino looses (eg. Fall of the Sabbat) or
// changes his title, he still gets one ballot to cast in the Prisi
// Sub-Referendum": with Fall of the Sabbat in play, one ballot, not two.
#[test]
fn gratiano_keeps_one_ballot_under_fall_of_the_sabbat() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(1), find_card("Gratiano"), 4, false);
    assert_eq!(e.state.ballots(g), 2);
    equipped(&mut e, "Fall of the Sabbat", 0, None, 0);
    assert_eq!(e.state.ballots(g), 1);
}

// Leandro [LSJ 20100527] "When a Methuselah decides to get no transfer, he
// can still use other effects that put blood on his uncontrolled vampires
// (eg. [DOM] Govern the Unaligned, Powerbase: Montreal)": seat 1 loses its
// transfers to Leandro; in its influence phase, Powerbase: Montreal still
// adds its blood.
#[test]
fn leandro_leaves_powerbase_montreal_its_blood() {
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("Leandro"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let young = crypt_where(|c| c.capacity >= 4, 0);
    e.state.seats[1]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: young,
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    let montreal = equipped(&mut e, "Powerbase: Montreal", 1, None, 0);
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    e.choose(index_of(&req, Choice::Pass));
    request(&mut e);
    assert!(e.state.seats[1].no_transfers_next);
    let add = ability(montreal, 0, Target::Uncontrolled(0));
    pick_when(&mut e, add);
    request(&mut e);
    assert_eq!(e.state.seats[1].uncontrolled[0].counters, 1);
}

// New Blood [ANK 20190117-2] "Cannot choose to be part of the 'Inner Circle'
// and get the title. The Inner Circle title and a Blood Brothers circle
// are different things": influenced out, his choices are the Blood
// Brother circles (or none); no title comes with them.
#[test]
fn new_blood_chooses_no_inner_circle() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: find_card("New Blood"),
            copy: Default::default(),
            counters: 4,
            banished: None,
        });
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    e.choose(index_of(&req, Choice::InfluenceOut { slot: 0 }));
    let req = request(&mut e);
    let nb = MinionId(e.state.minions.len() as u32 - 1);
    assert!(req.options.iter().all(|o| match o {
        Choice::ChooseCircle(c) => !c.contains("Inner Circle"),
        _ => true,
    }));
    let circle = *req
        .options
        .iter()
        .find(|o| matches!(o, Choice::ChooseCircle(_)))
        .expect("a circle");
    e.choose(index_of(&req, circle));
    request(&mut e);
    assert_eq!(e.state.title_of(nb), zvtes::crypt::Title::None);
}

// Erebus Mask [ANK 20180719-3] "Once in play, can be equipped from the
// minion it's on (rulebook action) by another minion that does not meet
// the requirements": a Brujah takes it from The Capuchin.
#[test]
fn erebus_mask_is_taken_by_a_non_harbinger() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let c = e.add_minion(SeatId(0), find_card("Capuchin, The"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let mask = in_play(&mut e, "Erebus Mask", 0, Some(c));
    pick(&mut e, take_from(b, c));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[mask as usize].host, Some(b));
}

// Erebus Mask [LSJ 20100203-2] "The fact it is Unique has no bearing on the
// burn option. You cannot use the burn option if you meet the requirement,
// even if you already have a copy of the card in play": a Methuselah with
// a ready Harbinger and a Mask on it is not offered the discard; one with
// no Harbinger is.
#[test]
fn erebus_mask_burn_option_needs_no_harbinger() {
    for harbinger in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if harbinger {
            let c = e.add_minion(SeatId(1), find_card("Capuchin, The"), 4, false);
            in_play(&mut e, "Erebus Mask", 1, Some(c));
        }
        let mask = find_library("Erebus Mask");
        e.state.seats[1].hand = vec![mask].into();
        library(&mut e, 1, "Soak", 3);
        pick(&mut e, Choice::Pass);
        let found = loop {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                break false;
            }
            if req.seat == SeatId(1) && req.options.contains(&Choice::Discard(0)) {
                break true;
            }
            e.choose(index_of(&req, fallback(&req)));
        };
        assert_eq!(found, !harbinger, "harbinger {harbinger}");
    }
}

// Call the Great Beast [LSJ 20080604] "The action provided by this card is
// considered to be an action requiring the listed clan": its counter action
// is Nergal's while he is Baali — not under Derange (a Malkavian's clan).
#[test]
fn call_the_great_beast_needs_a_baali_host() {
    for deranged in [false, true] {
        let mut e = game(2);
        let n = e.add_minion(SeatId(0), find_card("Nergal"), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let call = in_play(&mut e, "Call the Great Beast", 0, Some(n));
        e.state.cards[call as usize].counters = 10;
        if deranged {
            let d = in_play(&mut e, "Derange", 1, Some(n));
            e.state.minions[n.index()].deranged =
                Some(("Malkavian", zvtes::crypt::Sect::Camarilla, CardId(d)));
        }
        let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            req.options
                .contains(&use_at(call, 0, Some(n), Target::None)),
            !deranged
        );
    }
}

/// Seat 1's locked `card` vampire, ready to unlock and block with
/// `reaction` (mode `mode`): the vampire and its choice.
fn locked_blocker(e: &mut Engine, card: u16, reaction: &str, mode: u8) -> (MinionId, Choice) {
    let z = e.add_minion(SeatId(1), card, 4, false);
    e.state.minions[z.index()].locked = true;
    hand(e, 1, &[reaction]);
    (z, play(0, mode, z))
}

// The unlocks and attempts to block [LSJ 20090514] "If the action lacks a
// suitable target because of the unlock (e.g. Ambush), the block attempt
// still happens, as does the resulting combat if the block is
// successful": Ambush at a locked vampire, which unlocks to block it —
// each card of the group; the block's combat follows.
#[test]
fn an_unlock_to_block_ambush_still_blocks() {
    use zvtes::crypt::{Sect, Title};
    type Setup = fn(&mut Engine) -> (MinionId, Choice);
    let cases: [(&str, Setup); 9] = [
        ("Black Sunrise", |e| {
            locked_blocker(e, vampire_with("QUI", 0), "Black Sunrise", 0)
        }),
        ("Eluding the Arms of Morpheus", |e| {
            locked_blocker(e, plain("Ventrue", 0), "Eluding the Arms of Morpheus", 0)
        }),
        ("Read the Winds", |e| {
            let (z, unlock) = locked_blocker(e, vampire_with("ANI", 0), "Read the Winds", 1);
            grant(e, z, "AUS");
            (z, unlock)
        }),
        ("Second Tradition: Domain", |e| {
            let prince = crypt_where(|c| matches!(c.title, Title::Prince(_)), 0);
            locked_blocker(e, prince, "Second Tradition: Domain", 1)
        }),
        ("Sense the Savage Way", |e| {
            // Keegan's text (unlock after his own Gangrel action) is inert.
            locked_blocker(e, find_card("Keegan"), "Sense the Savage Way", 1)
        }),
        ("Under Siege", |e| {
            let sabbat = crypt_where(|c| c.sect == Some(Sect::Sabbat) && c.capacity >= 4, 0);
            let z = e.add_minion(SeatId(1), sabbat, 4, false);
            e.state.minions[z.index()].locked = true;
            let siege = in_play(e, "Under Siege", 1, None);
            e.state.cards[siege as usize].counters = 3;
            (z, use_at(siege, 0, Some(z), Target::None))
        }),
        ("Eyes of the Wild", |e| {
            locked_blocker(e, plain("Gangrel", 0), "Eyes of the Wild", 0)
        }),
        ("Organized Resistance", |e| {
            let baron = crypt_where(|c| matches!(c.title, Title::Baron(_)), 0);
            let b = e.add_minion(SeatId(1), baron, 4, false);
            let anarch = crypt_where(
                |c| c.sect == Some(Sect::Anarch) && c.title == Title::None,
                0,
            );
            let z = e.add_minion(SeatId(1), anarch, 4, false);
            e.state.minions[z.index()].locked = true;
            hand(e, 1, &["Organized Resistance"]);
            (z, play_at(0, 1, b, Target::Minion(z)))
        }),
        ("One With the Land", |e| {
            let tzimisce = crypt_where(|c| c.clan == "Tzimisce" && has(c, "PRO"), 0);
            locked_blocker(e, tzimisce, "One With the Land", 1)
        }),
    ];
    for (name, setup) in cases {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let (z, unlock) = setup(&mut e);
        hand(&mut e, 0, &["Ambush"]);
        e.notes = Some(Vec::new());
        pick(&mut e, play_at(0, 0, a, Target::Minion(z)));
        pick_when(&mut e, unlock);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        // Unlocked, the target no longer suits Ambush: a combat is the
        // block's.
        assert!(
            !noted_combats(&mut e).is_empty(),
            "{name}: the block's combat"
        );
    }
}

// Smash and Grab [ANK 20220507] "[ani] If used on an ally and it burns as a
// result, this counts as a Ⓓ action burning an ally (eg. Trophies,
// Predator's Transformation)": the prey's 2-life ally burned by the action;
// Predator's Transformation [ABO] follows — the prey burns 2 pool.
#[test]
fn smash_and_grab_burning_an_ally_answers_predators_transformation() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ani", 0), 4, false);
    e.state.minions[a.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    grant(&mut e, a, "ABO");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Dark Web Procurer"));
    e.state.minions[ally.index()].blood = 2;
    hand(&mut e, 0, &["Smash and Grab", "Predator's Transformation"]);
    pick(&mut e, play_at(0, 0, a, Target::Minion(ally)));
    // The hand shifts down: Predator's Transformation is card 0, its
    // after-action [ABO] mode 3.
    let req = offering(&mut e, play(0, 3, a));
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    assert!(!e.state.minions[ally.index()].in_play, "burned");
    e.choose(index_of(&req, play(0, 3, a)));
    request(&mut e);
    assert_eq!(pool(&e)[1], 10 - 2);
}

// Jimmy Dunn [LSJ 19990719] [RTR 19991001] "If a second Jimmy Dunn is
// brought out during an action, the acting vampire is not considered to be
// 'burning' the first Jimmy": seat 0's vampire diablerizes seat 2's Soul
// Gem bearer; the Gem brings seat 2's crypt Jimmy out, the prey's Jimmy
// burns — not the diablerist's burn: no Predator's Transformation [ABO].
#[test]
fn a_jimmy_dunn_burned_for_a_second_is_not_burned_by_the_action() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    grant(&mut e, a, "ABO");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let jimmy = find_card("Jimmy Dunn");
    let first = e.add_minion(SeatId(1), jimmy, 4, false);
    let bearer = e.add_minion(SeatId(2), crypt_where(|c| c.capacity >= 6, 0), 2, true);
    e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    in_play(&mut e, "Soul Gem of Etrius", 2, Some(bearer));
    e.state.seats[2].crypt = vec![jimmy].into();
    hand(&mut e, 0, &["Predator's Transformation"]);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: ActionKind::Diablerie,
            target: Some(bearer),
        },
    );
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 3, a));
    assert!(!e.state.minions[first.index()].in_play, "the first burned");
    assert!(e
        .state
        .minions
        .iter()
        .any(|m| m.in_play && m.owner == SeatId(2) && m.name() == "Jimmy Dunn"));
}
