//! The Endless Dance (30th Anniversary): the deck's new cards, appended to
//! the generated tables (`ADDED_BATCHES`); readings in rules-qa Batch 51.
//! Windows with a single option are elided, so the scripts only stop where
//! a real decision exists.

use zvtes::cards::{self, AbilitySource, Effect, Layer, Scope, Target};
use zvtes::crypt::CRYPT;
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::decks::Deck;
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, CardId, CardInPlay};

mod common;
use common::*;

// ---- Helpers ----------------------------------------------------------------

/// `n` seats at seat 0's minion phase, turn 4, 10 pool each.
fn game(n: usize) -> Engine {
    let mut e = Engine::bare(n);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.state.turn = 4;
    e
}

fn pool(e: &Engine) -> Vec<i32> {
    e.state.seats.iter().map(|s| s.pool).collect()
}

fn use_at(card: CardId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(card),
        ability,
        actor,
        target,
    }
}

fn vampire_ability(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: None,
        target,
    }
}

/// A card already in play for `seat`, on `host` if given, with its first
/// mode's layers (no play): its id.
fn in_play(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>) -> CardId {
    let card = find_library(name);
    e.state
        .cards
        .push(CardInPlay::new(card, SeatId(seat), host, 0));
    let id = CardId(e.state.cards.len() as u32 - 1);
    for effect in cards::lookup(card).expect("definition").modes[0].effects {
        if let Effect::Layer(modifier) = effect {
            e.state.layers.push(Layer {
                scope: Scope::InPlay,
                modifier: *modifier,
                source: id,
            });
        }
    }
    id
}

/// Seat 0: `caller` and a spare; the others' given vampires.
fn table(caller: &str) -> (Engine, MinionId) {
    let mut e = game(3);
    let m = e.add_minion(SeatId(0), find_card(caller), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    (e, m)
}

// ---- The deck ----------------------------------------------------------------

// The Endless Dance: the reviewed decklist (the bundle minus its 20 extras),
// 12 crypt and 88 library cards, every card defined — but Kazuki Hanabira
// and Valentina Osako, who print no text.
#[test]
fn the_endless_dance_is_a_complete_anniversary_deck() {
    let deck = Deck::anniversary(1);
    assert_eq!(deck.name, "The Endless Dance (Thirtieth Anniversary)");
    assert_eq!(deck.crypt_cards().len(), 12);
    assert_eq!(deck.library_cards().len(), 88);
    assert_eq!(deck.clan(), "Toreador");
    for c in deck.crypt_cards() {
        let name = CRYPT[c as usize].name;
        let textless = ["Kazuki Hanabira", "Valentina Osako"].contains(&name);
        assert_eq!(cards::vampire(c).is_some(), !textless, "{name}");
    }
    for c in deck.library_cards() {
        assert!(cards::lookup(c).is_some(), "{c}");
    }
}

// ---- Vampires ------------------------------------------------------------------

// "During a referendum she calls, Diana can burn 1 blood to cancel a
// reaction card as it is played (cost is still paid). +1 bleed."
#[test]
fn diana_iadanza_cancels_a_reaction_in_a_referendum_she_calls() {
    let (mut e, diana) = table("Diana Iadanza");
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    hand(&mut e, 0, &["Political Stranglehold"]);
    hand(&mut e, 1, &["Delaying Tactics"]);
    pick(&mut e, play(0, 0, diana));
    to_referendum(&mut e);
    let tactics = find_library("Delaying Tactics");
    // Seat 1 plays Delaying Tactics; Diana cancels it.
    pick_when(&mut e, play(0, 0, z));
    let req = expect(&mut e, SeatId(0), WindowLabel::CardAsPlayed);
    assert!(req
        .options
        .contains(&vampire_ability(diana, 0, Target::None)));
    pick(&mut e, vampire_ability(diana, 0, Target::None));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    request(&mut e);
    assert_eq!(blood(&e, diana), 3);
    assert!(e.state.seats[1].ash.contains(&tactics));
    assert!(e
        .state
        .once_played
        .contains(&find_library("Political Stranglehold")));
}

// "If François is ready during your unlock phase, he can burn 2 blood to
// lock a younger vampire. +1 bleed."
#[test]
fn francois_villon_locks_a_younger_vampire_in_his_unlock_phase() {
    let (mut e, francois) = table("François Villon");
    let young = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 2), 4, false);
    let lock = vampire_ability(francois, 0, Target::Minion(young));
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(2));
    let req = offering(&mut e, lock);
    assert_eq!(req.seat, SeatId(0));
    pick(&mut e, lock);
    request(&mut e);
    assert_eq!(blood(&e, francois), 2);
    assert!(e.state.minions[young.index()].locked);
}

// "Methuselahs casting (including controlling a minion casting) votes or
// ballots against a referendum called by Marcos burn 1 pool once results
// are tallied": once each Methuselah, however many sources (rules-qa
// 51.1).
#[test]
fn marcos_belegrad_burns_1_pool_from_each_methuselah_voting_against() {
    let (mut e, marcos) = table("Marcos Belegrad");
    let ingrid = e.add_minion(SeatId(1), find_card("Ingrid Rossler"), 4, false);
    let mark = e.add_minion(SeatId(2), find_card("Mark Decker"), 4, false);
    e.state.seats[1].has_edge = true;
    hand(&mut e, 0, &["Political Stranglehold"]);
    pick(&mut e, play(0, 0, marcos));
    to_referendum(&mut e);
    pick_when(&mut e, votes(marcos, true));
    pick_when(&mut e, votes(ingrid, false));
    pick_when(&mut e, Choice::CastEdge { favor: false });
    pick_when(&mut e, votes(mark, false));
    close_polling(&mut e);
    request(&mut e);
    assert_eq!(pool(&e), vec![10, 9, 9], "failed 2 to 5: no gain");
}

// [LSJ 20011022] "The ability can be used when in torpor": Modius
// discards a master card for +1 vote in the polling step of any
// referendum.
#[test]
fn modius_discards_a_master_for_a_vote_even_in_torpor() {
    let (mut e, caller) = table("Ingrid Rossler");
    let modius = e.add_minion(SeatId(1), find_card("Modius"), 4, true);
    hand(&mut e, 0, &["Political Stranglehold"]);
    hand(&mut e, 1, &["Zillah's Valley"]);
    pick(&mut e, play(0, 0, caller));
    to_referendum(&mut e);
    pick_when(&mut e, vampire_ability(modius, 0, Target::None));
    // The lone master card to discard: chosen without a decision.
    request(&mut e);
    assert!(e.state.seats[1].hand.is_empty());
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Zillah's Valley")));
}

// [LSJ 20011022] "The ability can be used when in torpor": during his
// controller's unlock phase, Silvian takes 1 blood from a prince they
// control.
#[test]
fn silvian_popescu_takes_a_princes_blood_even_in_torpor() {
    let (mut e, francois) = table("François Villon");
    let silvian = e.add_minion(SeatId(0), find_card("Silvian Popescu"), 0, true);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 2), 4, false);
    let take = vampire_ability(silvian, 0, Target::Minion(francois));
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(2));
    pick_when(&mut e, take);
    request(&mut e);
    assert_eq!(blood(&e, silvian), 1);
    assert_eq!(blood(&e, francois), 3);
}

// ---- Library ---------------------------------------------------------------------

// [LSJ 20030502] Awe: "X can be zero, but not less than zero": [pre] +X+1
// votes at X = 0, [PRE] +2X+1 at X = 2.
#[test]
fn awe_adds_x_plus_1_votes_and_x_may_be_0() {
    for (mode, x, more) in [(0u8, 0u8, 1u8), (1, 2, 5)] {
        let (mut e, diana) = table("Diana Iadanza");
        e.state.minions[diana.index()].blood = 6;
        hand(&mut e, 0, &["Political Stranglehold", "Awe"]);
        pick(&mut e, play(0, 0, diana));
        to_referendum(&mut e);
        pick_when(&mut e, play(0, mode, diana));
        pick(&mut e, set_x(x));
        request(&mut e);
        let before = e
            .referendum()
            .expect("referendum")
            .votes_of(&e.state, diana);
        assert_eq!(before, 3 + more, "Justicar 3 + Awe");
        assert_eq!(blood(&e, diana), 6 - x);
    }
}

// [RTR 19960530] Business Pressure: "Each Methuselah must decide how much
// pool to burn for it during the resolution of the effect … there can be
// some give and take": one pool at a time, from its player clockwise, until
// all pass in succession; [PRE] its player gets 3 votes a pool, the others
// 2. [RTR 20040501] "Cannot be used before the terms are set": first
// offered in the polling step.
#[test]
fn business_pressure_burns_pool_one_at_a_time_for_votes() {
    let (mut e, diana) = table("Diana Iadanza");
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 2), 4, false);
    hand(
        &mut e,
        0,
        &["Kine Resources Contested", "Business Pressure"],
    );
    pick(&mut e, play(0, 0, diana));
    let pressure = play(0, 1, diana);
    let req = offering(&mut e, pressure);
    assert_eq!(
        req.window,
        WindowLabel::ReferendumPolling,
        "after the terms"
    );
    pick(&mut e, pressure);
    let burn = |favor| Choice::CastCardVote { favor };
    // Seat 0 burns for, seat 1 against, seat 2 passes, seat 0 passes, seat
    // 1 burns again; then everyone passes.
    for (seat, choice) in [
        (0, burn(true)),
        (1, burn(false)),
        (2, Choice::Pass),
        (0, Choice::Pass),
        (1, burn(false)),
        (2, Choice::Pass),
        (0, Choice::Pass),
        (1, Choice::Pass),
    ] {
        let req = expect(&mut e, SeatId(seat), WindowLabel::ReferendumPolling);
        assert!(req.options.contains(&choice), "{seat}: {:?}", req.options);
        pick(&mut e, choice);
    }
    let r = e.referendum().expect("referendum");
    assert!(r.pressure.is_none(), "the round is over");
    assert_eq!(r.tally(), (3, 4));
    assert_eq!(pool(&e)[..2], [9, 8]);
}

// [ANK 20251125] [LSJ 20050221-2] "A vampire can have net negative votes,
// but net negative votes do not subtract from the total when votes are
// tallied": Camarilla Conclave's -1 vote to each non-Camarilla vampire,
// twice on Hartmut's 2 votes already cast, then once more on nothing.
#[test]
fn camarilla_conclave_takes_votes_from_non_camarilla_vampires() {
    let (mut e, diana) = table("Diana Iadanza");
    let hartmut = e.add_minion(SeatId(1), find_card("Hartmut Stover"), 4, false);
    let conclave = in_play(&mut e, 0, "Camarilla Conclave", None);
    e.state.cards[conclave.index()].counters = 4;
    hand(&mut e, 0, &["Political Stranglehold"]);
    pick(&mut e, play(0, 0, diana));
    to_referendum(&mut e);
    pick_when(&mut e, votes(hartmut, false));
    let minus = use_at(conclave, 1, None, Target::None);
    for _ in 0..3 {
        pick_when(&mut e, minus);
    }
    let r = e.referendum().expect("referendum");
    assert_eq!(r.tally(), (0, 0), "Hartmut's 2 votes gone, no less than 0");
    assert_eq!(e.state.cards[conclave.index()].counters, 1);
    pick_when(&mut e, minus);
    request(&mut e);
    assert!(
        !e.state.cards[conclave.index()].in_play,
        "no counters: burned"
    );
}

// [ANK 20200525-2] Immortal Grapple "Is still considered played if it is
// canceled as it is played (eg. Charismatic Aura). A second one cannot be
// played in the same round as its card text forbids it." Charismatic
// Aura's blood is burned to cancel the grapple; [ANK 20210226] "The card
// cannot be played if the minion cannot afford to burn the blood."
#[test]
fn charismatic_aura_cancels_a_grapple_which_still_counts_as_played() {
    let (mut e, diana) = table("Diana Iadanza");
    let z = e.add_minion(SeatId(1), vampire_with("POT", 0), 4, false);
    hand(&mut e, 0, &["Charismatic Aura"]);
    hand(&mut e, 1, &["Immortal Grapple", "Immortal Grapple"]);
    let grapple = play(0, 0, z);
    let aura = play(0, 0, diana);
    pick(&mut e, bleed(diana));
    pick(&mut e, block(z));
    pick_when(&mut e, grapple);
    let req = expect(&mut e, SeatId(0), WindowLabel::CardAsPlayed);
    assert!(req.options.contains(&aura));
    pick(&mut e, aura);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, grapple);
    assert_eq!(e.state.seats[1].ash.len(), 1, "one grapple, still played");

    // No blood: no Charismatic Aura.
    let (mut e, diana) = table("Diana Iadanza");
    let z = e.add_minion(SeatId(1), vampire_with("POT", 0), 4, false);
    hand(&mut e, 0, &["Charismatic Aura"]);
    hand(&mut e, 1, &["Immortal Grapple"]);
    pick(&mut e, bleed(diana));
    pick(&mut e, block(z));
    e.state.minions[diana.index()].blood = 0;
    pick_when(&mut e, play(0, 0, z));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play(0, 0, diana),
    );
}

// [LSJ 20100129] [ANK 20191228] Confiscation "Cannot be used or played if
// the conditions for the terms of the referendum cannot be met": no
// equipment in play, none; with one, it passes and the equipment moves
// onto the chosen vampire.
#[test]
fn confiscation_moves_an_equipment_to_the_chosen_vampire() {
    let (mut e, francois) = table("François Villon");
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    hand(&mut e, 0, &["Confiscation"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(plays(&req).is_empty(), "no equipment");
    let magnum = in_play(&mut e, 1, ".44 Magnum", Some(z));
    pick(&mut e, play(0, 0, francois));
    // The lone equipment chosen without a decision; then the vampire.
    pick(&mut e, Choice::ChooseMinion(francois));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    request(&mut e);
    assert_eq!(e.state.cards[magnum.index()].host, Some(francois));
    assert_eq!(e.state.card_controller(magnum), SeatId(0));
}

// [LSJ 20011214-2] Loup "Can be used even if the stealth is not currently
// needed": +1 stealth to every Toreador; burned after a Toreador's
// successful action; only one a turn.
#[test]
fn loup_gives_toreador_stealth_until_a_successful_action() {
    let mut e = game(3);
    let kazuki = e.add_minion(SeatId(0), find_card("Kazuki Hanabira"), 2, false);
    let diana = e.add_minion(SeatId(0), find_card("Diana Iadanza"), 4, false);
    for s in [1, 2] {
        let m = e.add_minion(SeatId(s), plain("Brujah", s as usize), 4, false);
        e.state.minions[m.index()].locked = true;
    }
    hand(&mut e, 0, &["Loup", "Loup"]);
    pick(&mut e, bleed(kazuki));
    // Offered with no block attempt: stealth not needed.
    pick_when(&mut e, play(0, 0, kazuki));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let loup = find_library("Loup");
    assert!(e.state.cards.iter().any(|c| c.card == loup), "put in play");
    assert!(
        !e.state.cards.iter().any(|c| c.in_play && c.card == loup),
        "burned by the success"
    );
    assert!(e.state.seats[0].ash.contains(&loup));
    // Only one a turn.
    pick(&mut e, bleed(diana));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        play(0, 0, diana),
    );
}

// [LSJ 20010702] [RTR 19941006] [LSJ 19991025] [LSJ 20100527] "If a minion
// who is attempting to block is locked, then that block will fail": The
// Louvre locks the Toreador attempting to block. With the Prince of Paris
// (François), it locks any minion.
#[test]
fn the_louvre_locks_a_toreador_blocker_and_with_paris_any_minion() {
    let (mut e, a) = table("Ingrid Rossler");
    let toreador = e.add_minion(SeatId(1), find_card("Diana Iadanza"), 4, false);
    let louvre = in_play(&mut e, 0, "Louvre, Paris, The", None);
    let lock = use_at(louvre, 0, None, Target::Minion(toreador));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(toreador));
    pick_when(&mut e, lock);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[toreador.index()].locked);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));

    // Not a Toreador: only with the Prince of Paris.
    let (mut e, _) = table("Ingrid Rossler");
    let other = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let louvre = in_play(&mut e, 0, "Louvre, Paris, The", None);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req
        .options
        .contains(&use_at(louvre, 1, None, Target::Minion(other))));
    e.add_minion(SeatId(0), find_card("François Villon"), 4, false);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req
        .options
        .contains(&use_at(louvre, 1, None, Target::Minion(other))));
}

// "Choose one or more Methuselahs. Successful referendum means each chosen
// Methuselah chooses up to three library cards in their ash heap and
// shuffles them into their library. Remove this card from the game, even
// if the action or the referendum fails or is canceled."
#[test]
fn reinforcements_shuffles_ash_cards_back_and_leaves_the_game() {
    let (mut e, francois) = table("François Villon");
    e.state.seats[0].ash = vec![
        find_library("Awe"),
        find_library("Loup"),
        find_library("Majesty"),
    ];
    hand(&mut e, 0, &["Reinforcements"]);
    pick(&mut e, play(0, 0, francois));
    pick_when(&mut e, Choice::ChooseSeat(SeatId(0)));
    pick_when(&mut e, Choice::Pass);
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    pick_when(&mut e, Choice::FindCard(find_library("Awe")));
    pick_when(&mut e, Choice::FindCard(find_library("Majesty")));
    pick_when(&mut e, Choice::Pass);
    request(&mut e);
    let seat = &e.state.seats[0];
    assert_eq!(seat.ash, vec![find_library("Loup")]);
    assert!(seat.library.contains(&find_library("Awe")));
    assert!(seat.removed.contains(&find_library("Reinforcements")));

    // Canceled as it is played: removed all the same.
    let (mut e, francois) = table("François Villon");
    hand(&mut e, 0, &["Reinforcements"]);
    hand(&mut e, 1, &["Direct Intervention"]);
    pick(&mut e, play(0, 0, francois));
    pick_when(&mut e, master_play(0));
    request(&mut e);
    assert!(e.state.seats[0]
        .removed
        .contains(&find_library("Reinforcements")));
    assert!(!e.state.seats[0]
        .ash
        .contains(&find_library("Reinforcements")));
    let _ = ActionKind::Bleed;
}
