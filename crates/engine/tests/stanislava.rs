//! Reign of Stanislava (25th Anniversary): the deck's new cards, appended
//! to the generated tables (`ADDED_BATCHES`), and the titles they bring
//! (archbishop, Inner Circle). Rulings settled in rules-qa Batch 49.
//! Windows with a single option are elided, so the scripts only stop where
//! a real decision exists.

use zvtes::cards::{self, AbilitySource, Effect, Layer, Scope, Target};
use zvtes::crypt::{Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::decks::Deck;
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, CardId, CardInPlay, UncontrolledSlot};

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

fn capacity(m: u16) -> i32 {
    CRYPT[m as usize].capacity as i32
}

fn play_at(card: u8, mode: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor,
        target,
    }
}

fn use_at(card: CardId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(card),
        ability,
        actor,
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

/// The political action card in hand slot 0 played by `caller`, voted
/// through with its card ballot (the others pass).
fn call_and_pass(e: &mut Engine, caller: MinionId) {
    pick(e, play(0, 0, caller));
    to_referendum(e);
    pick_when(e, Choice::CastCardVote { favor: true });
    close_polling(e);
}

// ---- Titles ----------------------------------------------------------------

// Inner Circle (4 votes, unique per clan) and archbishop (2 votes, a city's
// title: contests the prince and the baron of that city); the clan's
// justicar and Inner Circle member are two titles, no contest between them
// (rulebook "Vampiric Sects", "Other Vampire Sects").
#[test]
fn inner_circle_and_archbishop_titles_vote_and_contest() {
    let mut e = game(3);
    let stan = e.add_minion(SeatId(0), find_card("Stanislava"), 4, false);
    let xaviar = e.add_minion(SeatId(0), find_card("Xaviar"), 4, false);
    let hartmut = e.add_minion(SeatId(1), find_card("Hartmut Stover"), 4, false);
    assert_eq!(e.state.title_of(stan), Title::InnerCircle("Gangrel"));
    assert_eq!(e.state.title_votes_of(stan), 4);
    assert_eq!(e.state.title_of(xaviar), Title::Justicar("Gangrel"));
    assert_eq!(e.state.title_votes_of(xaviar), 3);
    assert!(e.state.title_rivals(stan).is_empty());
    assert!(e.state.title_rivals(xaviar).is_empty());
    assert_eq!(e.state.title_of(hartmut), Title::Archbishop("Berlin"));
    assert_eq!(e.state.title_votes_of(hartmut), 2);
    // Not Sabbat any more: no archbishop.
    e.state.minions[hartmut.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    assert_eq!(e.state.title_of(hartmut), Title::None);
    // Ingrid, Prince of Geneva, and an archbishop of Geneva would contest:
    // one city key.
    let ingrid = CRYPT[find_card("Ingrid Rossler") as usize].title;
    assert_eq!(
        zvtes::state::title_key(ingrid),
        zvtes::state::title_key(Title::Archbishop("Geneva"))
    );
}

// The Reign of Stanislava deck: the reviewed decklist, 12 crypt and 88
// library cards, every card defined.
#[test]
fn reign_of_stanislava_is_a_complete_anniversary_deck() {
    let deck = Deck::anniversary(0);
    assert_eq!(deck.name, "Reign of Stanislava (Twenty-Fifth Anniversary)");
    assert_eq!(deck.crypt_cards().len(), 12);
    assert_eq!(deck.library_cards().len(), 88);
    assert_eq!(deck.clan(), "Gangrel");
    for c in deck.crypt_cards() {
        assert!(cards::vampire(c).is_some(), "{}", CRYPT[c as usize].name);
    }
    for c in deck.library_cards() {
        assert!(cards::lookup(c).is_some(), "{c}");
    }
}

// ---- Zillah's Valley -------------------------------------------------------

// "Add 4 blood to a vampire in your uncontrolled region with capacity 8 or
// more."
#[test]
fn zillahs_valley_adds_4_blood_to_a_big_uncontrolled_vampire() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let big = CRYPT
        .iter()
        .position(|c| c.capacity >= 8)
        .expect("a big vampire") as u16;
    let small = CRYPT
        .iter()
        .position(|c| c.capacity < 8)
        .expect("a small vampire") as u16;
    for card in [small, big] {
        e.state.seats[0].uncontrolled.push(UncontrolledSlot {
            card,
            counters: 0,
            banished: None,
        });
    }
    hand(&mut e, 0, &["Zillah's Valley"]);
    e.at_master_phase();
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(!req
        .options
        .contains(&play_at(0, 0, None, Target::Uncontrolled(0))));
    pick(&mut e, play_at(0, 0, None, Target::Uncontrolled(1)));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled[1].counters, 4);
    assert_eq!(e.state.seats[0].pool, 5);
}

// ---- Political Stranglehold, Ancient Influence, Reins of Power -------------

// "Each Methuselah gains 3 pool for each vampire they control with capacity
// 8 or more": vampires in torpor count. Only one a game: a second copy is
// not offered.
#[test]
fn political_stranglehold_gains_3_per_big_vampire_once_a_game() {
    let mut e = game(3);
    let stan = e.add_minion(SeatId(0), find_card("Stanislava"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), find_card("Hartmut Stover"), 4, true);
    e.add_minion(SeatId(1), find_card("Mark Decker"), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 1), 4, false);
    let small = capacity(plain("Brujah", 1)) < 8;
    hand(
        &mut e,
        0,
        &["Political Stranglehold", "Political Stranglehold"],
    );
    call_and_pass(&mut e, stan);
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[..2], [13, 16]);
    assert_eq!(pool(&e)[2], if small { 10 } else { 13 });
    e.state.minions[stan.index()].locked = false;
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(plays(&req).is_empty(), "only one a game");
}

/// Seats 0-3: Stanislava (11) and a spare; Hartmut (10); Mark Decker (8);
/// Ingrid (9). Seats 2 and 3 at 4 pool.
fn four_gangrel() -> (Engine, [MinionId; 4]) {
    let mut e = game(4);
    let stan = e.add_minion(SeatId(0), find_card("Stanislava"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let hartmut = e.add_minion(SeatId(1), find_card("Hartmut Stover"), 4, false);
    let mark = e.add_minion(SeatId(2), find_card("Mark Decker"), 4, false);
    let ingrid = e.add_minion(SeatId(3), find_card("Ingrid Rossler"), 4, false);
    e.state.seats[2].pool = 4;
    e.state.seats[3].pool = 4;
    (e, [stan, hartmut, mark, ingrid])
}

/// Each seat's choice as the referendum passes, asked clockwise from the
/// acting Methuselah: `picks[i]` the vampire of seat `order[i]`, or none.
fn choose(e: &mut Engine, order: &[u8], picks: &[Option<MinionId>]) {
    for (&s, &m) in order.iter().zip(picks) {
        let req = expect(e, SeatId(s), WindowLabel::ReferendumTerms);
        assert_eq!(req.options[0], Choice::Pass);
        pick(e, m.map_or(Choice::Pass, Choice::ChooseMinion));
    }
}

// [ANK 20180813] "Applies a pool gain then a pool loss": the 5 pool burned
// in all cases, after the gain — a Methuselah at 4 pool choosing a vampire
// survives, one choosing none is ousted. The choices are made as it passes,
// clockwise from the acting Methuselah, in the open; none gains nothing
// (rules-qa 49.1). A locked vampire may be chosen (ready, not unlocked).
#[test]
fn ancient_influence_gains_the_chosen_capacity_then_burns_5() {
    let (mut e, [stan, hartmut, mark, _]) = four_gangrel();
    e.state.minions[hartmut.index()].locked = true;
    hand(&mut e, 0, &["Ancient Influence"]);
    call_and_pass(&mut e, stan);
    choose(
        &mut e,
        &[0, 1, 2, 3],
        &[Some(stan), Some(hartmut), Some(mark), None],
    );
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[3].ousted, "4 pool, nothing gained, 5 burned");
    assert_eq!(e.state.seats[2].half_vps, 2, "its prey ousted");
    // Seat 2 survives its 4 pool (gain first) and takes 6 for the oust.
    assert_eq!(pool(&e)[..3], [10 + 11 - 5, 10 + 10 - 5, 4 + 8 - 5 + 6]);
}

// [ANK 20180813] Reins of Power: "Each Methuselah gains 6 pool, then burns
// pool equal to the capacity of their predator's chosen vampire": the gain
// first; a predator choosing none burns nothing (rules-qa 49.1).
#[test]
fn reins_of_power_burns_the_capacity_the_predator_chose() {
    let (mut e, [stan, hartmut, mark, ingrid]) = four_gangrel();
    hand(&mut e, 0, &["Reins of Power"]);
    call_and_pass(&mut e, stan);
    choose(
        &mut e,
        &[0, 1, 2, 3],
        &[Some(stan), None, Some(mark), Some(ingrid)],
    );
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Seat 0's predator (3) chose Ingrid (9); seat 1's (0) Stanislava (11);
    // seat 2's (1) none; seat 3's (2) Mark Decker (8).
    assert_eq!(pool(&e), vec![10 + 6 - 9, 10 + 6 - 11, 4 + 6, 4 + 6 - 8]);
    let _ = hartmut;
}

// [LSJ 20000309] "If multiple ousts happen, players whose prey gets ousted
// get a victory point, even if they are being ousted themselves. Only the
// surviving ones get 6 pool from the oust of their prey (not of their
// grand-prey)": Ancient Influence and Reins of Power ousting seats 2 and 3
// at once.
#[test]
fn ancient_influence_and_reins_of_power_oust_at_once() {
    // Ancient Influence: seats 2 and 3 choose nothing, 4 - 5.
    let (mut e, [stan, hartmut, _, _]) = four_gangrel();
    hand(&mut e, 0, &["Ancient Influence"]);
    call_and_pass(&mut e, stan);
    choose(
        &mut e,
        &[0, 1, 2, 3],
        &[Some(stan), Some(hartmut), None, None],
    );
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[2].ousted && e.state.seats[3].ousted);
    // Seat 1 scores seat 2 (and its 6 pool); seat 2 scores seat 3 though
    // ousted; seat 0 is seat 3's predator, not seat 2's: nothing for it.
    let vps: Vec<u32> = e.state.seats.iter().map(|s| s.half_vps).collect();
    assert_eq!(vps, vec![0, 2, 2, 0]);
    assert_eq!(pool(&e)[..2], [16, 15 + 6]);

    // Reins of Power: seats 1 and 2 choose Hartmut (10) and Mark (8);
    // seat 3 at 1 pool.
    let (mut e, [stan, hartmut, mark, _]) = four_gangrel();
    e.state.seats[3].pool = 1;
    hand(&mut e, 0, &["Reins of Power"]);
    call_and_pass(&mut e, stan);
    choose(
        &mut e,
        &[0, 1, 2, 3],
        &[None, Some(hartmut), Some(mark), None],
    );
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[2].ousted && e.state.seats[3].ousted);
    let vps: Vec<u32> = e.state.seats.iter().map(|s| s.half_vps).collect();
    assert_eq!(vps, vec![0, 2, 2, 0]);
    assert_eq!(pool(&e)[..2], [16, 16 + 6]);
}

// [ANK 20210309-2] "Burning the card for a vote is not considered playing
// it": an Ancient Influence, a Political Stranglehold, a Reins of Power
// burned for a vote leaves the next copy playable.
#[test]
fn once_a_game_referendums_burned_for_a_vote_are_not_played() {
    for card in [
        "Ancient Influence",
        "Political Stranglehold",
        "Reins of Power",
    ] {
        let (mut e, [stan, ..]) = four_gangrel();
        hand(&mut e, 0, &["Kine Resources Contested"]);
        hand(&mut e, 1, &[card]);
        pick(&mut e, play(0, 0, stan));
        to_referendum(&mut e);
        pick_when(
            &mut e,
            Choice::BurnForVote {
                card: 0,
                favor: false,
            },
        );
        assert!(!e.state.once_played.contains(&find_library(card)), "{card}");
        assert!(e.state.seats[1].ash.contains(&find_library(card)));
    }
}

// ---- Permanent Vacation ----------------------------------------------------

// [LSJ 20100129] [ANK 20191228] "Cannot be used or played if the
// conditions for the terms of the referendum cannot be met": no ready ally,
// no Permanent Vacation. With one, it passes and the ally is removed from
// the game — not burned: its card goes out of the game, not to the ash
// heap.
#[test]
fn permanent_vacation_removes_a_ready_ally_from_the_game() {
    let mut e = game(3);
    let stan = e.add_minion(SeatId(0), find_card("Stanislava"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let deuce = e.add_ally(SeatId(1), find_library("Double Deuce"));
    e.state.minions[deuce.index()].in_torpor = true;
    hand(&mut e, 0, &["Permanent Vacation"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(plays(&req).is_empty(), "no ready ally");

    e.state.minions[deuce.index()].in_torpor = false;
    // The only ready ally: chosen without a decision.
    pick(&mut e, play(0, 0, stan));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[deuce.index()].in_play);
    let card = find_library("Double Deuce");
    assert!(e.state.seats[1].removed.contains(&card));
    assert!(!e.state.seats[1].ash.contains(&card));
}

// ---- Monastery of Shadows --------------------------------------------------

// "Unique location. +1 hand size. Lock to give a vampire with capacity 8 or
// more +1 stealth." Any acting vampire, another Methuselah's included: "you"
// is its controller (rules-qa 49.5); when stealth is needed.
#[test]
fn monastery_of_shadows_gives_any_big_acting_vampire_stealth() {
    let mut e = game(3);
    let stan = e.add_minion(SeatId(0), find_card("Stanislava"), 4, false);
    let small = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    assert!(capacity(plain("Brujah", 0)) < 8);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let monastery = in_play(&mut e, 2, "Monastery of Shadows", None);
    assert_eq!(e.state.hand_size(SeatId(2)), 8);
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
    let lend = use_at(monastery, 0, None, Target::None);
    // A small vampire: not offered.
    pick(&mut e, bleed(small));
    pick(&mut e, block(z));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, lend);
    e.state.minions[z.index()].locked = false;
    // Stanislava: seat 2 lends the stealth; the block fails.
    pick(&mut e, bleed(stan));
    pick(&mut e, block(z));
    let req = offering(&mut e, lend);
    assert_eq!(req.seat, SeatId(2));
    pick(&mut e, lend);
    // Both of seat 0's vampires locked: the turn passes.
    expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.cards[monastery.index()].locked);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
    assert_eq!(e.state.seats[1].pool, 10 - 3);
}

// ---- Forced March, Instantaneous Transformation ----------------------------

/// Seat 0: Stanislava (CEL FOR PRO ANI, 6 blood) and a spare; the others'
/// vampires locked (no block).
fn stanislava_alone() -> (Engine, MinionId) {
    let mut e = game(3);
    let stan = e.add_minion(SeatId(0), find_card("Stanislava"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    for s in [1, 2] {
        let m = e.add_minion(SeatId(s), plain("Brujah", s as usize), 4, false);
        e.state.minions[m.index()].locked = true;
    }
    (e, stan)
}

// [LSJ 20070411] Forced March: "Can be used to unlock if the action is
// successful, even if it has no effect due to lack of a suitable target";
// [LSJ 19981028] [ANK 20190425] "[cel][for] Is played after resolution, but
// still during the action". A hunt at full blood gains nothing and
// succeeds: Forced March unlocks, the action still under way. "A vampire
// can play only one Forced March each turn": the second copy is not
// offered after her next action, Instantaneous Transformation is.
#[test]
fn forced_march_unlocks_after_an_action_without_effect_once_a_turn() {
    let (mut e, stan) = stanislava_alone();
    e.state.minions[stan.index()].blood = 11;
    hand(
        &mut e,
        0,
        &[
            "Forced March",
            "Forced March",
            "Instantaneous Transformation",
        ],
    );
    pick(&mut e, action(stan, ActionKind::Hunt));
    let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    assert!(e.action().is_some(), "still during the action");
    assert!(req.options.contains(&play(0, 0, stan)));
    pick(&mut e, play(0, 0, stan));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[stan.index()].locked);
    assert_eq!(blood(&e, stan), 11);
    pick(&mut e, bleed(stan));
    let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    assert!(!plays(&req)
        .iter()
        .any(|p| matches!(p, Choice::Play { card: 0, .. })));
    assert!(req.options.contains(&play(1, 1, stan)), "{:?}", req.options);
}

// [PIB 20150915] [LSJ 19981028] [ANK 20190425] Instantaneous Transformation
// [CEL][PRO] "Is played after resolution, after all combats (if any) are
// handled, but still during the action"; [LSJ 20070411] "Can be played if
// the action is unblocked even if it has no effect due to lack of a
// suitable target". At superior, once each turn; the basic +1 stealth is
// not limited.
#[test]
fn instantaneous_transformation_unlocks_once_a_turn_at_superior() {
    let (mut e, stan) = stanislava_alone();
    e.state.minions[stan.index()].blood = 11;
    hand(
        &mut e,
        0,
        &[
            "Instantaneous Transformation",
            "Instantaneous Transformation",
        ],
    );
    pick(&mut e, action(stan, ActionKind::Hunt));
    let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    assert!(e.action().is_some(), "still during the action");
    assert!(req.options.contains(&play(0, 1, stan)));
    pick(&mut e, play(0, 1, stan));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[stan.index()].locked);
    pick(&mut e, bleed(stan));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play(0, 1, stan),
    );
}

// [ANK 20220218] "The after action resolution effect can be used if the
// action 'fizzles'": a Raptor employed for its blood by Stanislava down to
// 1 blood at resolution fizzles; Forced March and Instantaneous
// Transformation still unlock.
#[test]
fn forced_march_and_instantaneous_transformation_unlock_after_a_fizzled_action() {
    for (card, mode) in [("Forced March", 0), ("Instantaneous Transformation", 1)] {
        let mut e = game(3);
        let stan = e.add_minion(SeatId(0), find_card("Stanislava"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Brujah", 1), 5, false);
        let spy = find_library("Raven Spy");
        e.state.cards.push(CardInPlay {
            counters: 1,
            ..CardInPlay::new(spy, SeatId(1), Some(z), 0)
        });
        let id = CardId(e.state.cards.len() as u32 - 1);
        for effect in cards::lookup(spy).expect("definition").modes[0].effects {
            if let Effect::Layer(modifier) = effect {
                e.state.layers.push(Layer {
                    scope: Scope::InPlay,
                    modifier: *modifier,
                    source: id,
                });
            }
        }
        hand(&mut e, 0, &["Raptor", card]);
        pick(&mut e, play(0, 0, stan));
        expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
        e.state.minions[stan.index()].blood = 1;
        let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
        assert_eq!(
            e.action().and_then(|x| x.outcome),
            Some(zvtes::actions::Outcome::Fizzled)
        );
        // Raptor left the hand: the modifier is its first card.
        assert!(req.options.contains(&play(0, mode, stan)), "{card}");
        pick(&mut e, play(0, mode, stan));
        request(&mut e);
        assert!(!e.state.minions[stan.index()].locked, "{card}");
    }
}

// [CEL][FOR] "+1 stealth. After action resolution, if the action was
// successful, this vampire can burn 1 blood to unlock."
#[test]
fn forced_march_at_superior_gives_stealth_then_an_unlock_for_blood() {
    let mut e = game(3);
    let stan = e.add_minion(SeatId(0), find_card("Stanislava"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    hand(&mut e, 0, &["Forced March"]);
    pick(&mut e, bleed(stan));
    pick(&mut e, block(z));
    pick_when(&mut e, play(0, 1, stan));
    pick_when(&mut e, Choice::UnlockForBlood { minion: stan });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[stan.index()].locked);
    assert_eq!(blood(&e, stan), 5);
    assert_eq!(e.state.seats[1].pool, 10 - 3, "+2 bleed, unblocked");
}

// ---- Ingrid Rossler ----------------------------------------------------------

/// The transfers seat 0 can spend this influence phase on its slot `slot`.
fn transfers(e: &mut Engine, slot: u8) -> u8 {
    to_window(e, SeatId(0), WindowLabel::InfluenceTransfers);
    let mut n = 0;
    loop {
        // Spent out, the lone pass elides: the phase is over.
        let req = request(e);
        if req.window != WindowLabel::InfluenceTransfers
            || !req.options.contains(&Choice::Transfer { slot })
        {
            return n;
        }
        e.choose(index_of(&req, Choice::Transfer { slot }));
        n += 1;
    }
}

// "If Ingrid is ready at the start of your influence phase, you get +2
// transfers." [RTR 20180303] "When she is influenced out, her bonus
// transfers do not apply for the ongoing influence phase."
#[test]
fn ingrid_rossler_gives_2_transfers_if_ready_as_the_phase_starts() {
    let mut e = game(3);
    e.add_minion(SeatId(0), find_card("Ingrid Rossler"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let slot = {
        e.state.seats[0].uncontrolled.push(UncontrolledSlot {
            card: plain("Ventrue", 0),
            counters: 0,
            banished: None,
        });
        0
    };
    assert_eq!(transfers(&mut e, slot), 6);

    // Influenced out with the phase's first transfer: 3 more, no bonus.
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ingrid = find_card("Ingrid Rossler");
    e.state.seats[0].uncontrolled.push(UncontrolledSlot {
        card: plain("Ventrue", 0),
        counters: 0,
        banished: None,
    });
    e.state.seats[0].uncontrolled.push(UncontrolledSlot {
        card: ingrid,
        counters: capacity(ingrid) as u8 - 1,
        banished: None,
    });
    to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::Transfer { slot: 1 });
    pick(&mut e, Choice::InfluenceOut { slot: 1 });
    request(&mut e);
    assert!(e
        .state
        .minions
        .iter()
        .any(|m| m.in_play && m.crypt_card() == Some(ingrid)));
    assert_eq!(transfers(&mut e, 0), 3);
}

// ---- Mark Decker ---------------------------------------------------------------

// "Mark gets an additional +1 strength in combat with a werewolf. +1
// strength." Against Double Deuce (3 life), 3 damage burns it; against a
// vampire, 2.
#[test]
fn mark_decker_strikes_harder_against_a_werewolf() {
    let mut e = game(3);
    let mark = e.add_minion(SeatId(0), find_card("Mark Decker"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let deuce = e.add_ally(SeatId(1), find_library("Double Deuce"));
    pick(&mut e, bleed(mark));
    pick_when(&mut e, block(deuce));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[deuce.index()].in_play, "3 damage: burned");

    let mut e = game(3);
    let mark = e.add_minion(SeatId(0), find_card("Mark Decker"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    pick(&mut e, bleed(mark));
    pick(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 2);
}

// ---- Stanislava --------------------------------------------------------------

// "Allies cannot block her. +2 bleed."
#[test]
fn stanislava_cannot_be_blocked_by_allies_and_bleeds_for_3() {
    let mut e = game(3);
    let stan = e.add_minion(SeatId(0), find_card("Stanislava"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let deuce = e.add_ally(SeatId(1), find_library("Double Deuce"));
    pick(&mut e, bleed(stan));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(deuce));
    assert_eq!(e.state.seats[1].pool, 10 - 3);
}

// "Retainers employed by minions opposing Stanislava in combat lose their
// abilities": for that combat, their card text (rules-qa 49.3) — the
// Szlachta Bodyguard's prevention is not offered, the Raptor's hand-size
// penalty is gone; they stay retainers (the Bodyguard's life is there).
#[test]
fn stanislava_strips_the_opposing_retainers_of_their_text() {
    let mut e = game(3);
    let stan = e.add_minion(SeatId(0), find_card("Stanislava"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let guard = find_library("Szlachta Bodyguard");
    e.state.cards.push(CardInPlay {
        counters: 2,
        ..CardInPlay::new(guard, SeatId(1), Some(z), 0)
    });
    let guard = CardId(e.state.cards.len() as u32 - 1);
    // Her own retainer keeps its text.
    let raptor = find_library("Raptor");
    e.state.cards.push(CardInPlay {
        counters: 1,
        ..CardInPlay::new(raptor, SeatId(0), Some(stan), 1)
    });
    let raptor = CardId(e.state.cards.len() as u32 - 1);
    for effect in cards::lookup(find_library("Raptor")).expect("def").modes[1].effects {
        if let Effect::Layer(modifier) = effect {
            e.state.layers.push(Layer {
                scope: Scope::InPlay,
                modifier: *modifier,
                source: raptor,
            });
        }
    }
    // In combat with her: the guard's text is lost, her Raptor's is not.
    e.state.combatants = Some([stan, z]);
    assert!(e.state.text_suppressed(guard));
    assert!(!e.state.text_suppressed(raptor));
    assert_eq!(e.state.hand_size(SeatId(1)), 6, "her Raptor applies");
    e.state.combatants = None;
    // The combat itself: no prevention offered; the guard stays.
    let prevent = use_at(guard, 0, None, Target::None);
    pick(&mut e, bleed(stan));
    pick(&mut e, block(z));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, prevent);
    assert_eq!(blood(&e, z), 3, "1 damage, none prevented");
    assert!(e.state.cards[guard.index()].in_play, "still a retainer");
    assert!(!e.state.text_suppressed(guard), "only during that combat");
}

// ---- Hartmut Stover ----------------------------------------------------------

// "Any vampire casting votes or ballots against a referendum called by
// Hartmut burns 1 blood once results are tallied": not a cost — a vampire
// with no blood votes against and burns nothing; Hartmut too when he votes
// against his own; a Methuselah's ballot (the Edge) costs nothing (rules-qa
// 49.2).
#[test]
fn hartmut_stover_burns_1_blood_from_the_vampires_voting_against() {
    let mut e = game(3);
    let hartmut = e.add_minion(SeatId(0), find_card("Hartmut Stover"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ingrid = e.add_minion(SeatId(1), find_card("Ingrid Rossler"), 4, false);
    let mark = e.add_minion(SeatId(2), find_card("Mark Decker"), 0, false);
    e.state.seats[2].has_edge = true;
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, hartmut));
    to_referendum(&mut e);
    pick_when(&mut e, votes(hartmut, false));
    pick_when(&mut e, votes(ingrid, false));
    pick_when(&mut e, votes(mark, false));
    pick_when(&mut e, Choice::CastEdge { favor: false });
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, hartmut), 3, "against his own");
    assert_eq!(blood(&e, ingrid), 3);
    assert_eq!(blood(&e, mark), 0);
}

// "You can burn the Edge during your discard phase to gain 1 pool."
#[test]
fn hartmut_stover_burns_the_edge_for_1_pool_in_the_discard_phase() {
    let mut e = game(3);
    let hartmut = e.add_minion(SeatId(0), find_card("Hartmut Stover"), 4, true);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.seats[0].has_edge = true;
    let burn = Choice::UseAbility {
        source: AbilitySource::Vampire(hartmut),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    let req = offering(&mut e, burn);
    assert_eq!(req.seat, SeatId(0));
    pick(&mut e, burn);
    assert!(!e.state.seats[0].has_edge);
    assert_eq!(e.state.seats[0].pool, 11);
}

// ---- Xaviar (advanced) -------------------------------------------------------

// "You can unlock Xaviar after a referendum called by him passes." The
// advanced card's own text, keyed apart from a base Xaviar's.
#[test]
fn xaviar_unlocks_after_a_referendum_he_called_passes() {
    let card = find_card("Xaviar");
    assert!(CRYPT[card as usize].adv);
    assert!(cards::vampire(card).is_some_and(|d| d.adv));
    let mut e = game(3);
    let xaviar = e.add_minion(SeatId(0), card, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    let unlock = Choice::UseAbility {
        source: AbilitySource::Vampire(xaviar),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    call_and_pass(&mut e, xaviar);
    pick_when(&mut e, unlock);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[xaviar.index()].locked);
}

// ---- Fear of Mekhet ----------------------------------------------------------

// "Put this card on a ready justicar or Inner Circle member and lock them.
// During this vampire's unlock phase, they burn 5 blood. If they cannot
// burn the blood, they are burned; otherwise, their controller moves this
// card to another justicar or Inner Circle member, or burns this card if
// there are no others." [ANK 20210109] "Can be moved (not played) on a
// vampire in torpor." All or nothing; the move does not lock and may stay
// with the same Methuselah (rules-qa 49.4).
#[test]
fn fear_of_mekhet_bleeds_justicars_and_moves_on() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let stan = e.add_minion(SeatId(1), find_card("Stanislava"), 6, false);
    let xaviar = e.add_minion(SeatId(1), find_card("Xaviar"), 3, true);
    e.add_minion(SeatId(2), plain("Brujah", 2), 4, false);
    hand(&mut e, 0, &["Fear of Mekhet"]);
    e.at_master_phase();
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(
        !req.options
            .contains(&play_at(0, 0, None, Target::Minion(xaviar))),
        "ready only"
    );
    pick(&mut e, play_at(0, 0, None, Target::Minion(stan)));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[stan.index()].locked);
    // Seat 1's unlock phase: 5 blood, then the move — to Xaviar in torpor,
    // the only other one (a lone choice, elided).
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(blood(&e, stan), 1);
    let fear = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.in_play && c.card == find_library("Fear of Mekhet"))
            .expect("in play") as u32,
    );
    assert_eq!(e.state.cards[fear.index()].host, Some(xaviar));
    assert!(!e.state.minions[stan.index()].locked, "unlocked as usual");
    // Next time, 3 blood: Xaviar is burned (the card with him).
    pick(&mut e, Choice::Pass);
    to_minion_phase(&mut e, SeatId(2));
    to_minion_phase(&mut e, SeatId(0));
    to_minion_phase(&mut e, SeatId(1));
    assert!(!e.state.minions[xaviar.index()].in_play);
    assert!(!e.state.cards[fear.index()].in_play);
    assert_eq!(
        e.state.seats[0].ash.last(),
        Some(&find_library("Fear of Mekhet"))
    );
}

// ---- Enkil Cog ---------------------------------------------------------------

/// Seat 0: Stanislava (11) with an Enkil Cog on her, unlocked, and a spare;
/// seats 1 and 2: a locked vampire each (no block), at seat 1's minion
/// phase. Returns the Cog.
fn cog_table() -> (Engine, MinionId, CardId) {
    let mut e = game(3);
    let stan = e.add_minion(SeatId(0), find_card("Stanislava"), 6, false);
    let spare = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[spare.index()].locked = true;
    for s in [1, 2] {
        let m = e.add_minion(SeatId(s), plain("Brujah", s as usize), 4, false);
        e.state.minions[m.index()].locked = true;
    }
    let cog = in_play(&mut e, 0, "Enkil Cog", Some(stan));
    e.state.active = SeatId(1);
    (e, stan, cog)
}

// [PIB 20150915] [LSJ 19981028] [ANK 20190425] "Is played after
// resolution, after all combats (if any) are handled, but still during the
// action": after a successful bleed against the prey, on a vampire with
// capacity 10 or more; +1 bleed.
#[test]
fn enkil_cog_goes_on_a_big_vampire_after_a_bleed_against_the_prey() {
    let (mut e, stan) = stanislava_alone();
    let small = e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Enkil Cog", "Enkil Cog"]);
    // A small vampire's bleed: no Cog.
    pick(&mut e, bleed(small));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play(0, 0, small),
    );
    pick(&mut e, bleed(stan));
    let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    assert!(e.action().is_some(), "still during the action");
    pick(&mut e, play(0, 0, stan));
    request(&mut e);
    let cog = find_library("Enkil Cog");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == cog && c.host == Some(stan)));
    assert_eq!(e.state.seats[1].pool, 10 - 1 - 3);
    let _ = req;
}

// "During any Methuselah's minion phase, this vampire can lock this card to
// take an action": offered as another Methuselah declares the end of their
// minion phase; the action is the vampire's own, against its Methuselah's
// prey; then the impulse returns to the active Methuselah, whose phase goes
// on — ending it again, the locked Cog is not offered (rules-qa 49.6).
#[test]
fn enkil_cog_takes_an_action_as_another_methuselah_ends_their_minion_phase() {
    let (mut e, stan, cog) = cog_table();
    let lock = use_at(cog, 0, None, Target::None);
    // Seat 1's lone pass elides: the rotation comes to seat 0.
    let req = expect(&mut e, SeatId(0), WindowLabel::PhaseImpulse);
    assert!(req.options.contains(&lock));
    pick(&mut e, lock);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(req.options[0], Choice::Pass, "it may be let go");
    assert!(req.options.contains(&bleed(stan)));
    assert!(e.state.cards[cog.index()].locked);
    pick(&mut e, bleed(stan));
    // Back in seat 1's phase, which ends: nothing more for seat 0.
    never_offered_until(&mut e, SeatId(2), WindowLabel::MinionPhase, lock);
    assert!(e.state.minions[stan.index()].locked);
    // Seat 0's prey, seat 1: 1 + 2 (Stanislava) + 1 (the Cog).
    assert_eq!(e.state.seats[1].pool, 10 - 4);
}

// [LSJ 20081213-1] [ANK 20210124] "If locked to take an action and the
// action is canceled as it is played (not if it is canceled after), it
// stays locked and the minion can choose to take an action again or let the
// Cog effect be lost": Govern the Unaligned canceled by Direct
// Intervention, Stanislava acts again.
#[test]
fn enkil_cog_action_canceled_as_played_can_be_taken_again() {
    let (mut e, stan, cog) = cog_table();
    hand(&mut e, 0, &["Govern the Unaligned"]);
    // Out-of-turn: not the active Methuselah's (seat 1) on their turn.
    hand(&mut e, 2, &["Direct Intervention"]);
    pick(&mut e, use_at(cog, 0, None, Target::None));
    pick(&mut e, play(0, 0, stan));
    pick_when(&mut e, master_play(0));
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[cog.index()].locked, "the Cog stays locked");
    assert!(!e.state.minions[stan.index()].locked);
    assert!(req.options.contains(&bleed(stan)), "the action again");
    assert_eq!(req.options[0], Choice::Pass, "or let it go");
    pick(&mut e, Choice::Pass);
    assert!(!e.state.minions[stan.index()].locked);
}

// In its own Methuselah's minion phase, the vampire acts as usual: no Cog
// offered; nor while it is locked.
#[test]
fn enkil_cog_is_not_offered_in_its_own_phase_nor_to_a_locked_vampire() {
    let (mut e, stan, cog) = cog_table();
    let lock = use_at(cog, 0, None, Target::None);
    e.state.minions[stan.index()].locked = true;
    never_offered_until(&mut e, SeatId(2), WindowLabel::MinionPhase, lock);
    e.state.minions[stan.index()].locked = false;
    e.state.minions[stan.index()].blood = 6;
    to_minion_phase(&mut e, SeatId(0));
    pick(&mut e, Choice::Pass);
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, lock);
}

// The Cog's whole story across a turn: Stanislava bleeds her prey, takes
// the Cog, unlocks with Forced March; in her prey's turn, as they end their
// minion phase, she bleeds them again — her per-turn flags are the new
// turn's. The active Methuselah may block that action; her own Methuselah
// is never asked to block it (a bleed is directed at the prey).
#[test]
fn enkil_cog_vampire_bleeds_again_in_her_preys_turn_and_may_be_blocked() {
    let mut e = game(3);
    let stan = e.add_minion(SeatId(0), find_card("Stanislava"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.state.minions[z.index()].locked = true;
    e.add_minion(SeatId(2), plain("Brujah", 2), 4, false);
    hand(&mut e, 0, &["Enkil Cog", "Forced March"]);
    pick(&mut e, bleed(stan));
    pick_when(&mut e, play(0, 0, stan));
    pick_when(&mut e, play(0, 0, stan));
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[stan.index()].locked, "Forced March");
    assert_eq!(e.state.seats[1].pool, 10 - 3);
    let _ = req;
    pick(&mut e, Choice::Pass);
    // Seat 1's turn: they end their minion phase; the Cog is offered.
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, Choice::Pass);
    let cog = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.in_play && c.card == find_library("Enkil Cog"))
            .expect("in play") as u32,
    );
    let req = expect(&mut e, SeatId(0), WindowLabel::PhaseImpulse);
    pick(&mut e, use_at(cog, 0, None, Target::None));
    let req2 = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req2.options.contains(&bleed(stan)), "a new turn's bleed");
    let _ = req;
    pick(&mut e, bleed(stan));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(
        req.options.contains(&block(z)),
        "the active Methuselah blocks"
    );
    pick(&mut e, Choice::Pass);
    let own: Vec<MinionId> = e.state.minions_of(SeatId(0)).collect();
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::DeclareBlock { minion } if own.contains(minion)),
    );
    assert_eq!(e.state.seats[1].pool, 10 - 3 - 4);
}

// The acting Methuselah ousted during their Cog action, in another's turn:
// Fame on Stanislava, her controller at 3 pool, sent to torpor by the
// active Methuselah's blocker. The oust removes her mid-combat; the turn
// stays the active Methuselah's, who goes on.
#[test]
fn enkil_cog_holder_ousted_during_its_action_leaves_the_turn_going_on() {
    let (mut e, stan, cog) = cog_table();
    e.state.seats[0].pool = 3;
    e.state.minions[stan.index()].blood = 1;
    let blocker = e.add_minion(SeatId(1), find_card("Mark Decker"), 4, false);
    e.state.cards.push(CardInPlay::new(
        find_library("Fame"),
        SeatId(2),
        Some(stan),
        0,
    ));
    pick(&mut e, Choice::Pass);
    pick(&mut e, use_at(cog, 0, None, Target::None));
    pick(&mut e, bleed(stan));
    pick_when(&mut e, block(blocker));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].ousted, "Fame's 3 pool");
    assert!(!e.state.seats[1].ousted);
    assert!(e.action().is_none());
    let _ = req;
}
