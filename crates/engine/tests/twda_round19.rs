//! Legacy cards, round 19 (`ADDED_BATCHES`, slice 16ae): the Imbued, their
//! powers and convictions, and the rules they bring (rulebook "Appendix:
//! Imbued Rules"). Readings in rules-qa Batch 87. Windows with a single
//! option are elided, so the scripts only stop where a real decision
//! exists.

use zvtes::cards::Target;
use zvtes::crypt::CRYPT;
use zvtes::decision::{Choice, WindowLabel, Zone};
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

/// Imbued `name` in play for `seat`, its cost as its life.
fn imbued(e: &mut Engine, seat: u8, name: &str) -> MinionId {
    let c = find_card(name);
    assert!(CRYPT[c as usize].imbued, "{name} is an imbued");
    e.add_minion(SeatId(seat), c, CRYPT[c as usize].capacity, false)
}

/// Library card `name` in play on minion `host`, for `host`'s controller.
fn on(e: &mut Engine, host: MinionId, name: &str) -> CardId {
    let seat = e.state.minions[host.index()].owner;
    e.state
        .cards
        .push(CardInPlay::new(find_library(name), seat, Some(host), 0));
    CardId(e.state.cards.len() as u32 - 1)
}

/// The names of the conviction cards on `m`.
fn convictions(e: &Engine, m: MinionId) -> usize {
    e.state.convictions(m).len()
}

// ---- The imbued as a minion -------------------------------------------------

// Rulebook "Appendix: Imbued Rules" 1: "An imbued is considered a mortal
// ally, not a vampire. Imbued have 1 strength and 1 bleed, by default.
// Their cost is also their starting life."
#[test]
fn an_imbued_is_a_mortal_ally_its_cost_its_life() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    let v = &e.state.minions[m.index()];
    assert!(v.is_ally() && v.is_imbued() && !v.is_vampire());
    assert!(e.state.is_mortal(m) && !e.state.is_monster(m));
    let body = e.state.ally_body(m).expect("an ally's body");
    assert_eq!((body.life, body.strength, body.bleed), (4, 1, 1));
    assert_eq!(e.state.capacity_of(m), None);
    assert_eq!(e.state.sect_of(m), None);
}

// Rulebook "Appendix: Imbued Rules" 2: "When an imbued enters play with no
// conviction, they may gain 1 conviction from your library, hand or ash
// heap." Influenced out with its cost in counters, it chooses one from
// hand; Liz "doesn't gain a conviction as normal when entering play".
#[test]
fn an_imbued_influenced_out_gains_a_conviction() {
    for liz in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let name = if liz {
            "Liz \"Ticket312\" Thornton"
        } else {
            "Marion \"Teacher193\" Perks"
        };
        let card = find_card(name);
        let cost = CRYPT[card as usize].capacity;
        e.state.seats[1].uncontrolled = vec![UncontrolledSlot {
            card,
            copy: Default::default(),
            counters: cost,
            banished: None,
        }];
        hand(&mut e, 1, &["Second Sight"]);
        to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
        pick(&mut e, Choice::InfluenceOut { slot: 0 });
        let m = MinionId(e.state.minions.len() as u32 - 1);
        assert!(e.state.minions[m.index()].is_imbued());
        assert_eq!(blood(&e, m), cost, "its cost as its life");
        let from_hand = Choice::TakeFrom {
            zone: Zone::Hand,
            card: find_library("Second Sight"),
        };
        if liz {
            assert!(!request(&mut e).options.contains(&from_hand));
            assert_eq!(convictions(&e, m), 0);
        } else {
            pick(&mut e, from_hand);
            assert_eq!(convictions(&e, m), 1);
        }
    }
}

// Rulebook "Appendix: Imbued Rules" 2: "During your unlock phase, you may
// play 1 conviction on each of your imbued. You may play these conviction
// cards from your hand or from your ash heap." One each imbued, the second
// from the ash heap on the other one.
#[test]
fn convictions_are_played_in_the_unlock_phase_one_each_imbued() {
    let mut e = game(2);
    let a = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    let b = imbued(&mut e, 0, "Pedro Cortez");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Second Sight"]);
    e.state.seats[0].ash = vec![find_library("Strike with Conviction")].into();
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let on_a = Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(a),
    };
    assert!(req.options.contains(&on_a), "{:?}", req.options);
    pick(&mut e, on_a);
    let req = request(&mut e);
    // The hand is empty now: the ash heap's conviction is the first card.
    let ash_on = |m| Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(m),
    };
    assert!(!req.options.contains(&ash_on(a)), "one on each imbued");
    pick(&mut e, ash_on(b));
    request(&mut e);
    assert_eq!((convictions(&e, a), convictions(&e, b)), (1, 1));
    assert!(e.state.seats[0].ash.is_empty());
    for m in [a, b] {
        let c = e.state.convictions(m)[0];
        assert!(!e.state.cards[c.index()].effect_only, "the card itself");
    }
}

// Rulebook "Appendix: Imbued Rules" 2: "Each imbued has a limit of 5
// conviction. Any conviction gained above five is burned instead."
#[test]
fn a_sixth_conviction_is_burned() {
    let mut e = game(2);
    let a = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for _ in 0..5 {
        on(&mut e, a, "Second Sight");
    }
    hand(&mut e, 0, &["Second Sight"]);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(a),
        },
    );
    request(&mut e);
    assert_eq!(convictions(&e, a), 5);
    assert_eq!(
        e.state.seats[0].ash.to_vec(),
        vec![find_library("Second Sight")]
    );
}

// ---- Incapacitation ---------------------------------------------------------

// Rulebook "Appendix: Imbued Rules" 5: "When an imbued has no life, they are
// placed in the incapacitated region (controlled but not ready)"; "an
// imbued may leave the incapacitated region and gain a life … by burning 3
// conviction during their unlock phase".
#[test]
fn an_imbued_with_no_life_is_incapacitated_and_recovers_for_three_convictions() {
    let mut e = game(2);
    let a = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for _ in 0..3 {
        on(&mut e, a, "Second Sight");
    }
    e.state.minions[a.index()].blood = 1;
    e.damage(a, 1);
    let v = &e.state.minions[a.index()];
    assert!(v.in_play && v.incapacitated && !v.ready() && !v.in_torpor);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let recover = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(a),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    assert!(req.options.contains(&recover), "{:?}", req.options);
    pick(&mut e, recover);
    request(&mut e);
    let v = &e.state.minions[a.index()];
    assert!(v.ready() && !v.incapacitated);
    assert_eq!(v.blood, 1);
    assert_eq!(convictions(&e, a), 0);
}

// ---- Powers -----------------------------------------------------------------

/// The ability uses offered of card `card` in play.
// Rulebook "Appendix: Imbued Rules" 3: "An imbued may get a power as a +1
// stealth action, like equipment … The imbued unlocks if the action is
// successful. An imbued may not have two copies of the same power."
// Discern requires justice, Marion's.
#[test]
fn a_power_is_gained_by_a_plus_one_stealth_action_the_imbued_unlocking() {
    let mut e = game(2);
    let a = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Discern", "Discern"]);
    pick(&mut e, play(0, 0, a));
    let req = to_minion_phase(&mut e, SeatId(0));
    let discern = find_library("Discern");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == discern && c.host == Some(a) && c.in_play && !c.effect_only));
    assert!(!e.state.seats[0].ash.contains(&discern), "the card itself");
    assert!(!e.state.minions[a.index()].locked, "unlocked on success");
    assert!(!req.options.contains(&play(0, 0, a)), "no second copy");
}

// Discern: "[REACTION] +1 intercept. Only usable when a monster is acting."
// Used as a reaction (LSJ 20070403): the power locked, the imbued unlocked;
// the stealth a Cloak the Gathering adds needing it.
// Rulings: [LSJ 20070403] [LSJ 20070413].
#[test]
fn discern_gives_intercept_against_an_acting_monster() {
    let mut e = game(2);
    let a = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), vampire_with("obf", 0), 4, false);
    let d = on(&mut e, a, "Discern");
    hand(&mut e, 1, &["Cloak the Gathering"]);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(v));
    pick_when(&mut e, block(a));

    pick_when(&mut e, play(0, 0, v));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionBlock);
    let use_it = uses(&req, d.0)[0];
    e.choose(index_of(&req, use_it));
    request(&mut e);
    assert!(e.state.cards[d.index()].locked, "the power locked");
    assert_eq!(pool(&e)[0], 10, "blocked: not bled");
}

// Rulebook "Appendix: Imbued Rules" 5: "Any minion may burn an incapacitated
// imbued and take their equipment as a (D) action. If that action is
// successful, then each ready imbued may burn 1 conviction to inflict 1
// unpreventable damage on that acting minion."
#[test]
fn an_incapacitated_imbued_is_burned_by_an_action_and_avenged() {
    let mut e = game(2);
    let a = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    let b = imbued(&mut e, 0, "Pedro Cortez");
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    on(&mut e, b, "Second Sight");
    e.state.minions[a.index()].blood = 1;
    e.damage(a, 1);
    assert!(e.state.minions[a.index()].incapacitated);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: v,
            action: zvtes::decision::ActionKind::BurnImbued,
            target: Some(a),
        },
    );
    let ss = e.state.convictions(b)[0];
    pick_when(&mut e, Choice::BurnInPlay { card: ss.0 });
    request(&mut e);
    assert!(!e.state.minions[a.index()].in_play, "burned");
    assert_eq!(convictions(&e, b), 0);
    assert_eq!(blood(&e, v), 4 - 1, "avenged");
}

// Travis: "… as a +1 stealth action that costs 1 conviction [1
// CONVICTION]." "Must announce which conviction he burns as part of the
// terms of the action" (RTR 20111202): named as the action is announced,
// burned at resolution; gone by then, the action fizzles.
// Rulings: [RTR 20111202].
#[test]
fn travis_names_his_conviction_and_pays_it_at_resolution() {
    for lost in [false, true] {
        let mut e = game(2);
        let t = imbued(&mut e, 0, "Travis \"Traveler72\" Miller");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let ss = on(&mut e, t, "Second Sight");
        let sw = on(&mut e, t, "Strike with Conviction");
        let pedro = find_card("Pedro Cortez");
        e.state.seats[0].uncontrolled = vec![UncontrolledSlot {
            card: pedro,
            copy: Default::default(),
            counters: 0,
            banished: None,
        }];
        let act = Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Vampire(t),
            ability: 0,
            actor: Some(t),
            target: Target::Uncontrolled(0),
        };
        pick(&mut e, act);
        let req = expect(&mut e, SeatId(0), WindowLabel::ActionTerms);
        assert!(req.options.contains(&Choice::BurnInPlay { card: ss.0 }));
        pick(&mut e, Choice::BurnInPlay { card: sw.0 });
        if lost {
            e.state.cards[sw.index()].in_play = false;
        }
        to_minion_phase(&mut e, SeatId(0));
        let counters = e.state.seats[0].uncontrolled[0].counters;
        assert_eq!(counters, if lost { 0 } else { 2 });
        assert!(e.state.cards[ss.index()].in_play, "the other one kept");
        if !lost {
            assert!(!e.state.cards[sw.index()].in_play, "the named one burned");
        }
    }
}

// ---- The imbued's printed text ------------------------------------------------

// Béatrice: "During your unlock phase, another ready imbued you control
// takes 1 unpreventable damage. +1 bleed." The only other one is chosen.
#[test]
fn beatrice_damages_another_of_her_imbued_each_unlock_phase() {
    let mut e = game(2);
    let b = imbued(&mut e, 0, "Béatrice \"Oracle171\" Tremblay");
    let m = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert_eq!(e.state.current_bleed(b), 2);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!((blood(&e, b), blood(&e, m)), (3, 4 - 1));
}

// Earl: "+1 stealth on actions other than actions to enter combat. During
// your unlock phase, if you control more than two ready imbued, Earl burns
// 1 conviction." Three with him: one burned.
#[test]
fn earl_burns_a_conviction_with_more_than_two_ready_imbued() {
    for three in [false, true] {
        let mut e = game(2);
        let earl = imbued(&mut e, 0, "Earl \"Shaka74\" Deams");
        imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
        if three {
            imbued(&mut e, 0, "Pedro Cortez");
        }
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        on(&mut e, earl, "Second Sight");
        let hunt = zvtes::state::PrintedCtx {
            kind: Some(zvtes::decision::ActionKind::Bleed),
            ..Default::default()
        };
        assert_eq!(
            e.state.printed_mod(earl, zvtes::cards::Stat::Stealth, hunt),
            1
        );
        to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
        to_minion_phase(&mut e, SeatId(0));
        assert_eq!(convictions(&e, earl), usize::from(!three));
    }
}

// François: "Other Methuselahs' actions that target François cost an
// additional blood or life." A rush at him costs the vampire 1 blood more,
// paid at resolution.
#[test]
fn actions_at_francois_cost_another_blood() {
    let mut e = game(2);
    let f = imbued(&mut e, 0, "François \"Warden\" Loehr");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Bum's Rush"]);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(v),
            target: Target::Minion(f),
        },
    );
    let before = blood(&e, v);
    // Past the blocks declined, to the combat: paid by then.
    for _ in 0..20 {
        if e.state.combatants.is_some() {
            break;
        }
        let req = request(&mut e);
        e.choose(fallback(&req));
    }
    assert!(e.state.combatants.is_some(), "the rush's combat");
    assert_eq!(before - blood(&e, v), 1, "the toll paid as it resolves");
}

// Jennifer: "Weapons cost Jennifer 1 less pool or blood. +1 bleed. -1
// stealth."
#[test]
fn jennifer_pays_a_pool_less_for_weapons() {
    let mut e = game(2);
    let j = imbued(&mut e, 0, "Jennifer \"Flame61\" Vidisania");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert_eq!(e.state.current_bleed(j), 2);
    hand(&mut e, 0, &[".44 Magnum"]);
    pick(&mut e, play(0, 0, j));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(pool(&e)[0], 10 - (2 - 1));
}

// Pedro: "+1 strength. Pedro cannot maneuver to long range …": at close
// range, his granted maneuver (Jack's own is not his) is not offered.
#[test]
fn pedro_cannot_maneuver_to_long_range() {
    let mut e = game(2);
    let p = imbued(&mut e, 0, "Pedro Cortez");
    assert_eq!(
        e.state
            .printed_mod(p, zvtes::cards::Stat::Strength, Default::default()),
        1
    );
    assert_eq!(
        e.state
            .printed_mod(p, zvtes::cards::Stat::NoManeuverToLong, Default::default()),
        1
    );
}

// Paul: "During your influence phase, you may move a counter from your pool
// to an imbued in your uncontrolled region." Not to a vampire there.
#[test]
fn paul_moves_a_pool_counter_to_an_uncontrolled_imbued() {
    let mut e = game(2);
    let paul = imbued(&mut e, 0, "Paul \"Sixofswords29\" Moreton");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].uncontrolled = vec![
        UncontrolledSlot {
            card: plain("Brujah", 1),
            copy: Default::default(),
            counters: 0,
            banished: None,
        },
        UncontrolledSlot {
            card: find_card("Pedro Cortez"),
            copy: Default::default(),
            counters: 0,
            banished: None,
        },
    ];
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    let to = |i| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(paul),
        ability: 0,
        actor: None,
        target: Target::Uncontrolled(i),
    };
    assert!(
        !req.options.contains(&to(0)),
        "not to a vampire: {:?}",
        req.options
    );
    pick(&mut e, to(1));
    assert_eq!(e.state.seats[0].uncontrolled[1].counters, 1);
    assert_eq!(pool(&e)[0], 10 - 1);
}

// Leaf: "During your discard phase, Leaf may lock to gain a conviction from
// your library (shuffle afterward), hand or ash heap."
#[test]
fn leaf_locks_to_gain_a_conviction_from_the_library() {
    let mut e = game(2);
    let leaf = imbued(&mut e, 0, "Leaf \"Potter116\" Pankowski");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Second Sight")].into();
    let lock = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(leaf),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick_when(&mut e, lock);
    pick(
        &mut e,
        Choice::TakeFrom {
            zone: Zone::Library,
            card: find_library("Second Sight"),
        },
    );
    request(&mut e);
    assert!(e.state.minions[leaf.index()].locked);
    assert_eq!(convictions(&e, leaf), 1);
}

// Anna: "Anna may move 2 blood from the blood bank to any vampire as a +1
// stealth action. During your unlock phase, you may look at the top three
// cards of your library." Another Methuselah's vampire: directed.
#[test]
fn anna_gives_two_blood_from_the_bank_to_any_vampire() {
    let mut e = game(2);
    let anna = imbued(&mut e, 0, "Anna \"Dictatrix11\" Suljic");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    let act = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(anna),
        ability: 0,
        actor: Some(anna),
        target: Target::Minion(v),
    };
    pick(&mut e, act);
    assert!(e
        .action()
        .is_some_and(|a| a.directed_seat(&e.state) == Some(SeatId(1))));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, v), 3);
}

// Inez: "When Inez enters play, you may search your library (shuffle
// afterward) or hand for a power that requires innocence and put it on
// her." Hide (innocence) from the library; Discern (justice) not offered.
// Rulings: [LSJ 20080526].
#[test]
fn inez_searches_a_power_requiring_innocence_as_she_enters() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = find_card("Inez \"Nurse216\" Villagrande");
    e.state.seats[1].uncontrolled = vec![UncontrolledSlot {
        card,
        copy: Default::default(),
        counters: 3,
        banished: None,
    }];
    fill_hand(&mut e, 1);
    e.state.seats[1].library = vec![find_library("Hide"), find_library("Discern")].into();
    to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    let inez = MinionId(e.state.minions.len() as u32 - 1);
    let req = request(&mut e);
    let discern = Choice::TakeFrom {
        zone: Zone::Library,
        card: find_library("Discern"),
    };
    assert!(!req.options.contains(&discern));
    pick(
        &mut e,
        Choice::TakeFrom {
            zone: Zone::Library,
            card: find_library("Hide"),
        },
    );
    request(&mut e);
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.host == Some(inez) && c.card == find_library("Hide")));
}

// Jennie: "Any incapacitated imbued may move to the ready region and gain a
// life … by burning 2 conviction during any Methuselah's unlock phase."
// Another Methuselah's imbued, in Jennie's controller's unlock phase.
#[test]
fn jennie_lets_any_incapacitated_imbued_recover_for_two_convictions() {
    let mut e = game(2);
    imbued(&mut e, 0, "Jennie \"Cassie247\" Orne");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let m = imbued(&mut e, 1, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    on(&mut e, m, "Second Sight");
    on(&mut e, m, "Second Sight");
    e.state.minions[m.index()].blood = 1;
    e.damage(m, 1);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let recover = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(m),
        ability: 1,
        actor: None,
        target: Target::None,
    };
    pick_when(&mut e, recover);
    request(&mut e);
    assert!(e.state.minions[m.index()].ready());
    assert_eq!((blood(&e, m), convictions(&e, m)), (1, 0));
}

// John: "John may enter combat with a monster as a Ⓓ action that costs 1
// conviction." Not a mortal ally.
#[test]
fn john_rushes_a_monster_for_a_conviction() {
    let mut e = game(2);
    let john = imbued(&mut e, 0, "John \"Cop90\" O'Malley");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let mortal = imbued(&mut e, 1, "Marion \"Teacher193\" Perks");
    on(&mut e, john, "Second Sight");
    let rush = |t| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(john),
        ability: 0,
        actor: Some(john),
        target: Target::Minion(t),
    };
    let req = request(&mut e);
    assert!(
        !req.options.contains(&rush(mortal)),
        "a mortal is no monster"
    );
    e.notes = Some(Vec::new());
    pick(&mut e, rush(v));
    to_minion_phase(&mut e, SeatId(0));
    assert!(!noted_combats(&mut e).is_empty(), "the rush's combat");
    assert_eq!(convictions(&e, john), 0, "paid at resolution");
}

// Maman: "During a referendum, Maman may burn a conviction to get 3 votes
// against the referendum."
#[test]
fn maman_buys_three_votes_against_with_a_conviction() {
    let mut e = game(2);
    let maman = imbued(&mut e, 1, "Maman Boumba");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    on(&mut e, maman, "Second Sight");
    let anarch = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.sect == Some(zvtes::crypt::Sect::Anarch)
                && c.title == zvtes::crypt::Title::None
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("an anarch") as u16;
    let a = e.add_minion(SeatId(0), anarch, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Anarchist Uprising"]);
    pick(&mut e, play(0, 0, a));
    let votes = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(maman),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick_when(&mut e, votes);
    let r = e.referendum().expect("referendum");
    assert!(r.bonus.contains(&(maman, 3)) && r.forced_against.contains(&maman));
    request(&mut e);
    assert_eq!(convictions(&e, maman), 0);
}

// Hide: "[REACTION] [1 CONVICTION] Only usable during a monster's action
// directed at this imbued. The action fails." The rush fails: no combat.
// Rulings: [LSJ 20070403] [LSJ 20070413] [LSJ 20070219].
#[test]
fn hide_fails_a_monsters_action_at_its_imbued() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Inez \"Nurse216\" Villagrande");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let hide = on(&mut e, m, "Hide");
    on(&mut e, m, "Second Sight");
    hand(&mut e, 1, &["Bum's Rush"]);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(v),
            target: Target::Minion(m),
        },
    );
    let req = offering(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(hide),
            ability: 1,
            actor: None,
            target: Target::None,
        },
    );
    let use_it = uses(&req, hide.0)[0];
    e.choose(index_of(&req, use_it));
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, m), 3, "no combat");
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
}

// Champion: "[REACTION] [2 CONVICTION] … a monster controlled by another
// Methuselah is taking an action directed at you or an imbued … The action
// fails and the acting monster enters combat with this imbued instead."
// Its bleed at seat 0 fails; the vampire fights Champion's imbued.
// Rulings: [LSJ 20070403] [LSJ 20070413] [LSJ 20070217] [LSJ 20070219].
#[test]
fn champion_turns_a_monsters_action_into_a_combat_with_its_imbued() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Jack \"Hannibal137\" Harmon");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let ch = on(&mut e, m, "Champion");
    on(&mut e, m, "Second Sight");
    on(&mut e, m, "Second Sight");
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(v));
    let req = offering(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(ch),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    let use_it = uses(&req, ch.0)[0];
    e.choose(index_of(&req, use_it));
    request(&mut e);
    assert_eq!(e.state.combatants, Some([v, m]));
    assert_eq!(convictions(&e, m), 0);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[0], 10, "not bled");
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
}

// Illuminate: "All imbued get +1 intercept when blocking monsters (not
// cumulative with any other Illuminate in play, nor with The Unmasking). Any
// monster may enter combat with this imbued as a Ⓓ action."
#[test]
fn illuminate_gives_imbued_intercept_and_invites_a_rush() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Inez \"Nurse216\" Villagrande");
    let other = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Illuminate"]);
    pick(&mut e, play(0, 0, m));
    to_minion_phase(&mut e, SeatId(0));
    let il = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(e.state.cards[il.index()].host, Some(m));
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let rush = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(il),
        ability: 0,
        actor: Some(v),
        target: Target::Minion(m),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&rush), "{:?}", req.options);
    pick(&mut e, bleed(v));
    let a = e.action().expect("the bleed").clone();
    assert_eq!(a.intercept_of(&e.state, other), 1, "a monster acting");
}

// ---- In combat ----------------------------------------------------------------

/// Seat 1's vampire `v` bleeds seat 0, blocked by `b`: into the combat.
fn blocked_by(e: &mut Engine, v: MinionId, b: MinionId) {
    to_window(e, SeatId(1), WindowLabel::MinionPhase);
    pick(e, bleed(v));
    pick_when(e, block(b));
}

/// A table for combat: seat 0's imbued `name` and a vampire, seat 1's two
/// vampires (the first returned).
fn combat_table(name: &str) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let m = imbued(&mut e, 0, name);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    (e, m, v)
}

/// The first offered use of the ability `ability` of a card in play or of a
/// minion's printed text.
fn first_use(e: &mut Engine, source: zvtes::cards::AbilitySource, ability: u8) -> Choice {
    for _ in 0..60 {
        let req = request(e);
        if let Some(o) = req.options.iter().find(|o| {
            matches!(o, Choice::UseAbility { source: s, ability: a, .. } if *s == source && *a == ability)
        }) {
            return *o;
        }
        e.choose(fallback(&req));
    }
    panic!("{source:?} ability {ability} never offered");
}

// Jack: "Jack gets an optional maneuver on the first round of combat."
#[test]
fn jack_gets_a_maneuver_in_the_first_round() {
    let (mut e, jack, v) = combat_table("Jack \"Hannibal137\" Harmon");
    blocked_by(&mut e, v, jack);
    let m = first_use(&mut e, zvtes::cards::AbilitySource::Vampire(jack), 0);
    pick(&mut e, m);
    assert_eq!(
        e.combat().map(|c| c.range),
        Some(zvtes::combat::Range::Long),
        "maneuvered"
    );
}

// Peter: "While Peter is in combat with a monster, he may burn a conviction
// to get a maneuver."
#[test]
fn peter_burns_a_conviction_to_maneuver_against_a_monster() {
    let (mut e, peter, v) = combat_table("Peter \"Outback295\" Rophail");
    on(&mut e, peter, "Second Sight");
    blocked_by(&mut e, v, peter);
    let m = first_use(&mut e, zvtes::cards::AbilitySource::Vampire(peter), 0);
    pick(&mut e, m);
    assert_eq!(
        e.combat().map(|c| c.range),
        Some(zvtes::combat::Range::Long)
    );
    request(&mut e);
    assert_eq!(convictions(&e, peter), 0);
}

// Shame: "[COMBAT] [2 CONVICTION] Only usable before strikes are chosen.
// Burn this card. The opposing monster takes an amount of damage equal to
// his or her strength, then combat ends." A strength-1 vampire with 5
// blood: 4 left; the card burned.
// Rulings: [RTR 19970630].
#[test]
fn shame_turns_a_monsters_strength_on_it_and_ends_combat() {
    let (mut e, m, v) = combat_table("Leaf \"Potter116\" Pankowski");
    let shame = on(&mut e, m, "Shame");
    on(&mut e, m, "Second Sight");
    on(&mut e, m, "Second Sight");
    blocked_by(&mut e, v, m);
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Card(shame), 0);
    pick(&mut e, u);
    request(&mut e);
    assert_eq!(blood(&e, v), 5 - 1);
    assert!(!e.state.cards[shame.index()].in_play, "burned");
    assert_eq!(e.state.combatants, None, "combat ended");
}

// Surge: "Put three surge counters on this imbued. This imbued may burn any
// number of surge counters to prevent that amount of non-aggravated damage.
// Burn all surge counters … at the end of the action."
#[test]
fn surge_counters_prevent_damage_and_go_with_the_action() {
    let (mut e, m, v) = combat_table("Jennifer \"Flame61\" Vidisania");
    let surge = on(&mut e, m, "Surge");
    on(&mut e, m, "Second Sight");
    blocked_by(&mut e, v, m);
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Card(surge), 0);
    pick(&mut e, u);
    request(&mut e);
    assert_eq!(e.state.cards[surge.index()].counters, 3);
    let prevent = first_use(&mut e, zvtes::cards::AbilitySource::Card(surge), 2);
    pick(&mut e, prevent);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, m), 4, "the hand strike prevented");
    assert_eq!(
        e.state.cards[surge.index()].counters,
        0,
        "burned with the action"
    );
}

// Abjure: "[COMBAT] Lock this imbued before range is determined to end a
// combat between a monster and a mortal. If the mortal is a minion other
// than this imbued, you may move a conviction to this imbued from your hand
// or ash heap." Another imbued of its player's blocking (the blocker locked,
// it cannot use its own): the power and the imbued locked, a conviction
// gained.
#[test]
fn abjure_ends_a_combat_between_a_monster_and_a_mortal() {
    let (mut e, m, v) = combat_table("Erick \"Shophet125\" Franco");
    let other = imbued(&mut e, 0, "Jennifer \"Flame61\" Vidisania");
    let abjure = on(&mut e, other, "Abjure");
    hand(&mut e, 0, &["Second Sight"]);
    blocked_by(&mut e, v, m);
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Card(abjure), 0);
    pick(&mut e, u);
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: Zone::Hand,
            card: find_library("Second Sight"),
        },
    );
    request(&mut e);
    assert!(e.state.cards[abjure.index()].locked);
    assert!(e.state.minions[other.index()].locked);
    assert_eq!(convictions(&e, other), 1);
    assert_eq!((blood(&e, m), blood(&e, v)), (4, 5), "no strike");
}

// Foresee: "[COMBAT] [1 CONVICTION] … first round of combat resulting from a
// block. Look at the opposing minion's controller's hand. You may then
// choose to cancel combat. If you do, and this imbued was blocking, the
// action continues as if unblocked."
// Rulings: [RTR 19941109] [ANK 20190116].
#[test]
fn foresee_looks_and_cancels_the_combat_the_bleed_going_on() {
    let (mut e, m, v) = combat_table("Earl \"Shaka74\" Deams");
    let fs = on(&mut e, m, "Foresee");
    on(&mut e, m, "Second Sight");
    hand(&mut e, 1, &["Govern the Unaligned"]);
    blocked_by(&mut e, v, m);
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Card(fs), 0);
    pick(&mut e, u);
    assert!(e.state.hand_visible_to(SeatId(1), SeatId(0)));
    pick(&mut e, Choice::CancelCombat);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[0], 10 - 1, "bled, as if unblocked");
}

// Strike with Conviction: "Burn to make a hand or weapon strike against a
// monster at +1 damage."
#[test]
fn strike_with_conviction_hits_a_monster_harder() {
    let (mut e, m, v) = combat_table("Marion \"Teacher193\" Perks");
    let sw = on(&mut e, m, "Strike with Conviction");
    blocked_by(&mut e, v, m);
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Card(sw), 0);
    pick(&mut e, u);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(blood(&e, v) <= 5 - 2, "2 damage");
    assert!(!e.state.cards[sw.index()].in_play);
}

// Marion: "Marion gets an optional press each combat."
#[test]
fn marion_gets_a_press_each_combat() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    assert_eq!(
        e.state
            .printed_mod(m, zvtes::cards::Stat::CombatPress, Default::default()),
        1
    );
}

// ---- More of the imbued, the powers and convictions ---------------------------

// Xian: "If Xian successfully blocks a monster and is still ready at the end
// of the action, he may gain a conviction from your hand or ash heap."
#[test]
fn xian_gains_a_conviction_after_blocking_a_monster() {
    let (mut e, xian, v) = combat_table("Xian \"DziDzat155\" Quan");
    e.state.seats[0].ash = vec![find_library("Second Sight")].into();
    blocked_by(&mut e, v, xian);
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Vampire(xian), 0);
    pick(&mut e, u);
    pick(
        &mut e,
        Choice::TakeFrom {
            zone: Zone::AshHeap,
            card: find_library("Second Sight"),
        },
    );
    request(&mut e);
    assert_eq!(convictions(&e, xian), 1);
}

// React with Conviction: "When an effect that would change control of this
// imbued is played or announced, you may burn this card to cancel that
// effect … No cost is paid." Far Mastery [DOM] stealing it: canceled, the
// thief still unlocked (LSJ 20060425).
// Rulings: [LSJ 20060425] [ANK 20200813-2].
#[test]
fn react_with_conviction_cancels_a_steal_of_its_imbued() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), vampire_with("DOM", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let rc = on(&mut e, m, "React with Conviction");
    hand(&mut e, 1, &["Far Mastery"]);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 1,
            actor: Some(v),
            target: Target::Minion(m),
        },
    );
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Card(rc), 0);
    pick(&mut e, u);
    request(&mut e);
    assert_eq!(e.state.minions[m.index()].owner, SeatId(0), "not stolen");
    assert!(!e.state.cards[rc.index()].in_play, "burned");
}

// Second Sight: "Burn when a monster is acting to give this imbued +1
// intercept for that action." Against a +1 stealth bleed.
#[test]
fn second_sight_gives_intercept_against_a_monster() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), vampire_with("obf", 0), 4, false);
    let ss = on(&mut e, m, "Second Sight");
    hand(&mut e, 1, &["Cloak the Gathering"]);
    blocked_by(&mut e, v, m);
    pick_when(&mut e, play(0, 0, v));
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Card(ss), 0);
    pick(&mut e, u);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[0], 10, "blocked");
    assert!(!e.state.cards[ss.index()].in_play);
}

// Strike with Conviction: "Burn to bleed at +1 bleed as a Ⓓ action." "If
// the action to bleed is blocked or canceled, Strike with Conviction is not
// burned" (LSJ 20060421): burned as the bleed resolves.
#[test]
fn strike_with_conviction_bleeds_and_burns_only_as_it_resolves() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let sw = on(&mut e, m, "Strike with Conviction");
    let bleed_it = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(sw),
        ability: 2,
        actor: Some(m),
        target: Target::None,
    };
    pick(&mut e, bleed_it);
    assert!(
        e.state.cards[sw.index()].in_play,
        "kept while the bleed is on"
    );
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(pool(&e)[1], 10 - 2, "1 + 1 bleed");
    assert!(!e.state.cards[sw.index()].in_play, "burned as it resolved");
}

// Vigilance: "[ACTION MODIFIER] Only usable at the end of a successful
// action (after resolving the action). Unlock this imbued."
// Rulings: [ANK 20190425].
#[test]
fn vigilance_unlocks_its_imbued_after_a_successful_action() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let vig = on(&mut e, m, "Vigilance");
    pick(&mut e, bleed(m));
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Card(vig), 0);
    pick(&mut e, u);
    request(&mut e);
    assert!(!e.state.minions[m.index()].locked);
    assert!(e.state.cards[vig.index()].locked);
}

// Rejuvenate: "During your unlock phase, if this imbued has fewer life than
// his or her starting amount, he or she gains 1 life."
#[test]
fn rejuvenate_heals_its_imbued_each_unlock_phase() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    on(&mut e, m, "Rejuvenate");
    e.state.minions[m.index()].blood = 2;
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, m), 3);
}

// Respire: "[ACTION] Ⓓ Inflict 1 damage on a vampire with capacity less than
// 6. If this action is successful, you may move 1 conviction from your hand
// or ash heap to this imbued."
#[test]
fn respire_damages_a_small_vampire_and_gains_a_conviction() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Leaf \"Potter116\" Pankowski");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let small = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            !c.imbued
                && c.capacity <= 5
                && c.title == zvtes::crypt::Title::None
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("a small vampire") as u16;
    let cap = CRYPT[small as usize].capacity;
    let v = e.add_minion(SeatId(1), small, cap, false);
    let r = on(&mut e, m, "Respire");
    hand(&mut e, 0, &["Second Sight"]);
    let act = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(r),
        ability: 0,
        actor: Some(m),
        target: Target::Minion(v),
    };
    pick(&mut e, act);
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: Zone::Hand,
            card: find_library("Second Sight"),
        },
    );
    request(&mut e);
    assert_eq!(blood(&e, v), cap - 1);
    assert_eq!(convictions(&e, m), 1);
}

// Inspire: "[ACTION] [1 CONVICTION] +1 stealth action. … move the conviction
// you paid for this action from your ash heap to any imbued in play."
#[test]
fn inspire_moves_the_conviction_it_paid_to_another_imbued() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Inez \"Nurse216\" Villagrande");
    let other = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ins = on(&mut e, m, "Inspire");
    on(&mut e, m, "Second Sight");
    let act = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(ins),
        ability: 1,
        actor: Some(m),
        target: Target::Minion(other),
    };
    pick(&mut e, act);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!((convictions(&e, m), convictions(&e, other)), (0, 1));
}

// Project: "[ACTION MODIFIER] [1 CONVICTION] Only usable as the action is
// announced. Vampires with capacity less than 7 and monster allies who cost
// less than 4 pool cannot block this action."
#[test]
fn project_keeps_small_vampires_from_blocking() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Maman Boumba");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let small = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let p = on(&mut e, m, "Project");
    on(&mut e, m, "Second Sight");
    pick(&mut e, bleed(m));
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Card(p), 0);
    pick(&mut e, u);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(small));
    assert_eq!(pool(&e)[1], 10 - 1);
}

// Illuminate: "Burn this card during your next unlock phase or if this
// imbued leaves the ready region." Incapacitated: burned.
#[test]
fn illuminate_burns_as_its_imbued_is_incapacitated() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Inez \"Nurse216\" Villagrande");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let il = on(&mut e, m, "Illuminate");
    e.state.minions[m.index()].blood = 1;
    e.damage(m, 1);
    request(&mut e);
    assert!(!e.state.cards[il.index()].in_play);
}

// Lupe: "If Lupe has a vehicle, once each action she may burn a conviction
// to get +1 stealth or +1 intercept."
#[test]
fn lupe_with_a_vehicle_buys_stealth_with_a_conviction() {
    let mut e = game(2);
    let lupe = imbued(&mut e, 0, "Lupe \"Cabbie22\" Droin");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    on(&mut e, lupe, "Sport Bike");
    on(&mut e, lupe, "Second Sight");
    pick(&mut e, bleed(lupe));
    pick_when(&mut e, block(b));
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Vampire(lupe), 0);
    pick(&mut e, u);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(pool(&e)[1], 10 - 1, "unblocked");
    assert_eq!(convictions(&e, lupe), 0);
}

// Erick: "If Erick successfully enters combat with a monster as an action,
// he may gain a conviction from your hand or ash heap (before combat
// begins)." Bum's Rush at a vampire.
#[test]
fn erick_gains_a_conviction_rushing_a_monster() {
    let mut e = game(2);
    let erick = imbued(&mut e, 0, "Erick \"Shophet125\" Franco");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Bum's Rush", "Second Sight"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(erick),
            target: Target::Minion(v),
        },
    );
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Vampire(erick), 0);
    pick(&mut e, u);
    pick(
        &mut e,
        Choice::TakeFrom {
            zone: Zone::Hand,
            card: find_library("Second Sight"),
        },
    );
    request(&mut e);
    assert_eq!(convictions(&e, erick), 1);
}

// Donate: "[ACTION MODIFIER] [1 CONVICTION] Only usable during a bleed
// action by a ready imbued other than the acting mortal. Burn this card to
// give the acting mortal you control +1 bleed (limited)."
// Rulings: [ANK 20231124] [LSJ 19970718].
#[test]
fn donate_gives_another_mortal_a_bleed() {
    let mut e = game(2);
    let d = imbued(&mut e, 0, "Travis \"Traveler72\" Miller");
    let m = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let don = on(&mut e, d, "Donate");
    on(&mut e, d, "Second Sight");
    pick(&mut e, bleed(m));
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Card(don), 0);
    pick(&mut e, u);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(pool(&e)[1], 10 - 2);
    assert!(!e.state.cards[don.index()].in_play, "burned");
    assert_eq!(convictions(&e, d), 0);
}

// ---- Cards from earlier rounds meeting the imbued -------------------------------

// Rulebook "Appendix: Imbued Rules", card rulings: Lázár Dobrescu "targeting
// unknown uncontrolled card OK, but the recipient must be a vampire". An
// imbued alone in his controller's region: no action (LSJ 19990215); the
// prey's imbued a source.
#[test]
fn lazar_takes_from_an_imbued_but_gives_to_vampires_only() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lázár Dobrescu"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let slot = |card| UncontrolledSlot {
        card,
        copy: Default::default(),
        counters: 2,
        banished: None,
    };
    e.state.seats[1]
        .uncontrolled
        .push(slot(find_card("Pedro Cortez")));
    e.state.seats[0]
        .uncontrolled
        .push(slot(find_card("Marion \"Teacher193\" Perks")));
    let act = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(l),
        ability: 0,
        actor: Some(l),
        target: Target::Seat(SeatId(1)),
    };
    assert!(
        !request(&mut e).options.contains(&act),
        "no vampire of his own"
    );
    e.state.seats[0].uncontrolled.push(slot(plain("Brujah", 2)));
    pick(&mut e, act);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.seats[1].uncontrolled[0].counters, 1,
        "from the imbued"
    );
    assert_eq!(
        (
            e.state.seats[0].uncontrolled[0].counters,
            e.state.seats[0].uncontrolled[1].counters
        ),
        (2, 3),
        "to the vampire"
    );
}

// The Soul Gem of Etrius "can retrieve an imbued, and will even put them in
// play if they are 'younger' than the bearer, but they get no blood or
// life" (rulebook "Appendix: Imbued Rules", card rulings): incapacitated at
// once.
// Rulings: [LSJ 20060323].
#[test]
fn the_soul_gem_brings_an_imbued_in_with_no_life() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), vampire_with("pot", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cap = e.state.capacity_of(b).expect("a vampire");
    let small = CRYPT
        .iter()
        .position(|c| c.imbued && c.capacity < cap)
        .expect("a younger imbued") as u16;
    e.state.seats[0].crypt = vec![small].into();
    on(&mut e, b, "Soul Gem of Etrius");
    e.burn(b);
    request(&mut e);
    let m = MinionId(e.state.minions.len() as u32 - 1);
    let v = &e.state.minions[m.index()];
    assert!(v.is_imbued() && v.in_play && v.incapacitated && v.blood == 0);
}

// ---- The rulings on the powers --------------------------------------------------

// [LSJ 20081016] Abjure "can be used by an incapacitated Imbued".
#[test]
fn abjure_is_used_by_an_incapacitated_imbued() {
    let (mut e, m, v) = combat_table("Marion \"Teacher193\" Perks");
    let other = imbued(&mut e, 0, "Jennifer \"Flame61\" Vidisania");
    let abjure = on(&mut e, other, "Abjure");
    e.state.minions[other.index()].blood = 1;
    e.damage(other, 1);
    assert!(e.state.minions[other.index()].incapacitated);
    blocked_by(&mut e, v, m);
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Card(abjure), 0);
    pick(&mut e, u);
    request(&mut e);
    assert!(e.state.cards[abjure.index()].locked);
}

// [ANK 20260316] Champion "cannot be used while a block is being
// attempted": the bleed blocked, the attempt under way, not offered.
// Rulings: [ANK 20260316] [LSJ 20041022].
#[test]
fn champion_is_not_used_during_a_block_attempt() {
    let (mut e, m, v) = combat_table("Jack \"Hannibal137\" Harmon");
    let ch = on(&mut e, m, "Champion");
    on(&mut e, m, "Second Sight");
    on(&mut e, m, "Second Sight");
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    hand(&mut e, 1, &["Cloak the Gathering"]);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(v));
    pick_when(&mut e, block(b));
    let req = request(&mut e);
    assert!(uses(&req, ch.0).is_empty(), "{:?}", req.options);
}

// [LSJ 20060421] "If the action to bleed is blocked …, Strike with
// Conviction is not burned."
// Rulings: [LSJ 20060421] [RTR 20111202].
#[test]
fn strike_with_convictions_blocked_bleed_keeps_it() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let sw = on(&mut e, m, "Strike with Conviction");
    pick(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(sw),
            ability: 2,
            actor: Some(m),
            target: Target::None,
        },
    );
    pick_when(&mut e, block(b));
    pick_when(&mut e, Choice::HandStrike);
    to_minion_phase(&mut e, SeatId(0));
    assert!(e.state.cards[sw.index()].in_play, "not burned");
    assert_eq!(pool(&e)[1], 10, "blocked");
}

// [LSJ 20040812-2] Surge "cannot be used to prevent aggravated damage":
// with counters and only aggravated damage pending, not offered.
// Rulings: [LSJ 20040812-2] [PIB 20130327].
#[test]
fn surge_prevents_no_aggravated_damage() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Jennifer \"Flame61\" Vidisania");
    let surge = on(&mut e, m, "Surge");
    e.state.cards[surge.index()].counters = 3;
    let def = zvtes::cards::lookup(find_library("Surge")).expect("defined");
    assert!(def.abilities[2]
        .requires
        .contains(&zvtes::cards::Requirement::NonAggravatedDamagePending));
}

// [RTR 20180303] Paul: "When he is influenced out, his ability can be used
// during the ongoing influence phase."
#[test]
fn paul_moves_a_counter_in_the_influence_phase_he_arrives() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let paul = find_card("Paul \"Sixofswords29\" Moreton");
    e.state.seats[1].uncontrolled = vec![
        UncontrolledSlot {
            card: paul,
            copy: Default::default(),
            counters: 4,
            banished: None,
        },
        UncontrolledSlot {
            card: find_card("Pedro Cortez"),
            copy: Default::default(),
            counters: 0,
            banished: None,
        },
    ];
    to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    let p = MinionId(e.state.minions.len() as u32 - 1);
    let to = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(p),
        ability: 0,
        actor: None,
        target: Target::Uncontrolled(0),
    };
    pick_when(&mut e, to);
    assert_eq!(e.state.seats[1].uncontrolled[0].counters, 1);
}

// Vigilance: "[REACTION] [1 CONVICTION] Only usable by a locked imbued.
// Unlock this imbued." During another Methuselah's action (LSJ 20070403).
// Rulings: [LSJ 20070403] [LSJ 20070413].
#[test]
fn vigilance_unlocks_a_locked_imbued_as_a_reaction() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let vig = on(&mut e, m, "Vigilance");
    on(&mut e, m, "Second Sight");
    e.state.minions[m.index()].locked = true;
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(v));
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Card(vig), 1);
    pick(&mut e, u);
    request(&mut e);
    assert!(!e.state.minions[m.index()].locked);
    assert_eq!(convictions(&e, m), 0);
}

// ---- Ledger rows met by the imbued ----------------------------------------------

/// Seat 0's imbued `name` at 1 life bleeds, blocked by seat 1's vampire:
/// its hand strike takes the last life.
fn last_life_table(name: &str) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let m = imbued(&mut e, 0, name);
    e.state.minions[m.index()].blood = 1;
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    (e, m, z)
}

/// Into the combat of `last_life_table`, and out of the action.
fn lose_last_life(e: &mut Engine, m: MinionId, z: MinionId) {
    pick(e, bleed(m));
    pick_when(e, block(z));
    until(e, SeatId(1), WindowLabel::MinionPhase);
}

// The Book of Going Forth by Night: [ANK 20220816] "Does not apply to
// Imbued getting incapacitated." Its last life lost to the bearer: the
// imbued incapacitated, not stolen.
// Rulings: [ANK 20220816].
#[test]
fn the_book_of_going_forth_by_night_takes_no_incapacitated_imbued() {
    let (mut e, m, z) = last_life_table("Marion \"Teacher193\" Perks");
    e.state.cards.push(CardInPlay::new(
        find_library("Book of Going Forth by Night, The"),
        SeatId(1),
        Some(z),
        0,
    ));
    lose_last_life(&mut e, m, z);
    let v = &e.state.minions[m.index()];
    assert!(v.incapacitated && v.in_play);
    assert_eq!(v.owner, SeatId(0), "not stolen");
}

// Tension in the Ranks: [LSJ 20060409-2] "Pool is not paid when an imbued
// is incapacitated."
// Rulings: [LSJ 20060409-2].
#[test]
fn tension_in_the_ranks_costs_nothing_for_an_incapacitated_imbued() {
    let (mut e, m, z) = last_life_table("Marion \"Teacher193\" Perks");
    e.state.cards.push(CardInPlay::new(
        find_library("Tension in the Ranks"),
        SeatId(1),
        None,
        0,
    ));
    lose_last_life(&mut e, m, z);
    assert!(e.state.minions[m.index()].incapacitated);
    assert_eq!(pool(&e)[0], 10);
}

// Dreams of the Sphinx: [LSJ 20070516] "Cannot move blood to Imbued in the
// uncontrolled region." An imbued alone there: no target; a vampire beside
// it is one.
// Rulings: [LSJ 20070516].
#[test]
fn dreams_of_the_sphinx_adds_no_blood_to_an_uncontrolled_imbued() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.cards.push(CardInPlay::new(
        find_library("Dreams of the Sphinx"),
        SeatId(0),
        None,
        0,
    ));
    let dreams = CardId(e.state.cards.len() as u32 - 1);
    let slot = |card| UncontrolledSlot {
        card,
        copy: Default::default(),
        counters: 0,
        banished: None,
    };
    e.state.seats[0]
        .uncontrolled
        .push(slot(find_card("Pedro Cortez")));
    e.state.seats[0].uncontrolled.push(slot(plain("Brujah", 2)));
    let to = |i| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(dreams),
        ability: 2,
        actor: None,
        target: Target::Uncontrolled(i),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&to(1)), "{:?}", req.options);
    assert!(!req.options.contains(&to(0)), "the imbued");
}

// Kindred Segregation: [LSJ 20060409] "Imbued can be kept by paying 0
// pool, as they have a \"pool cost to recruit\" of zero."
// Rulings: [LSJ 20060409].
#[test]
fn kindred_segregation_keeps_an_imbued_for_nothing() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let m = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    hand(&mut e, 0, &["Kindred Segregation"]);
    let card = (e.state.seats[0].hand.len() - 1) as u8;
    pick(&mut e, play(card, 0, a));
    to_referendum(&mut e);
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    pick_when(&mut e, Choice::ChooseMinion(m));
    request(&mut e);
    assert!(e.state.minions[m.index()].in_play, "kept");
    assert_eq!(pool(&e)[0], 10, "for nothing");
}

// Champion: [LSJ 20070214-2] "The action fails: it might be continued (eg.
// Form of Mist) but still fails anyway." The bleeder's Form of Mist [PRO]
// ends Champion's combat and continues the action: it fails, no pool bled.
// Rulings: [LSJ 20070214-2].
#[test]
fn champions_failed_action_continued_still_fails() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Jack \"Hannibal137\" Harmon");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), vampire_with("PRO", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let ch = on(&mut e, m, "Champion");
    on(&mut e, m, "Second Sight");
    on(&mut e, m, "Second Sight");
    hand(&mut e, 1, &["Form of Mist"]);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(v));
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Card(ch), 0);
    pick(&mut e, u);
    pick_when(&mut e, play(0, 1, v));
    pick_when(&mut e, Choice::ContinueAction);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
    assert_eq!(pool(&e)[0], 10, "not bled");
}

/// Seat 1's vampire rushes seat 0's imbued holding `power` (and two
/// convictions): the power's reaction fails the rush, the rusher unlocked
/// again after. Returns the engine and the rusher.
fn rush_failed_by(power: &str, ability: u8) -> (Engine, MinionId) {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Inez \"Nurse216\" Villagrande");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let p = on(&mut e, m, power);
    on(&mut e, m, "Second Sight");
    on(&mut e, m, "Second Sight");
    hand(&mut e, 1, &["Bum's Rush", "Bum's Rush"]);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(v),
            target: Target::Minion(m),
        },
    );
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Card(p), ability);
    pick(&mut e, u);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
    e.state.minions[v.index()].locked = false;
    (e, v)
}

// Hide and Champion, "Fail an Action": [ANK 20211015] [RTR 20180719] "If the
// action fails, the action has reached resolution, so is subject to NRA
// (Non Repeatable Action) rules." The rusher, unlocked again, cannot rush
// with the second Bum's Rush. Hide's own row: [RTR 20180719] [ANK 20211015].
// Rulings: [ANK 20211015] [RTR 20180719].
#[test]
fn an_action_hide_or_champion_fails_is_spent() {
    for (power, ability) in [("Hide", 1), ("Champion", 0)] {
        let (mut e, v) = rush_failed_by(power, ability);
        let req = request(&mut e);
        let again = req
            .options
            .iter()
            .any(|o| matches!(o, Choice::Play { actor: Some(a), .. } if *a == v));
        assert!(!again, "{power}: NRA, {:?}", req.options);
    }
}

// Hide, "Fail an action (after 'as announced')": [ANK 20260316] [LSJ
// 20041022] "Cannot be used while a block is being attempted, or has
// already been successful." Another of its player's minions attempting to
// block the rush: not offered.
// Rulings: [ANK 20260316] [LSJ 20041022].
#[test]
fn hide_is_not_used_during_a_block_attempt() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Inez \"Nurse216\" Villagrande");
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let hide = on(&mut e, m, "Hide");
    on(&mut e, m, "Second Sight");
    hand(&mut e, 1, &["Bum's Rush", "Cloak the Gathering"]);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(v),
            target: Target::Minion(m),
        },
    );
    pick_when(&mut e, block(b));
    let req = request(&mut e);
    assert!(uses(&req, hide.0).is_empty(), "{:?}", req.options);
}

// Surge and Donate: [LSJ 20070403] [LSJ 20070413] "[REACTION] Can be used
// only when reactions can be used (unlocked or with a wake effect). It
// does not count as playing a reaction card." Surge's reaction during
// another Methuselah's bleed: not for a locked imbued; unlocked, used, the
// power stays in play (no card played).
// Rulings: [LSJ 20070403] [LSJ 20070413].
#[test]
fn surges_reaction_needs_an_imbued_able_to_react() {
    for locked in [true, false] {
        let mut e = game(2);
        let m = imbued(&mut e, 0, "Jennifer \"Flame61\" Vidisania");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let surge = on(&mut e, m, "Surge");
        on(&mut e, m, "Second Sight");
        e.state.minions[m.index()].locked = locked;
        hand(&mut e, 1, &["Cloak the Gathering"]);
        to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
        pick(&mut e, bleed(v));
        let react = |o: &Choice| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), ability: 1, .. } if *c == surge);
        if locked {
            for _ in 0..40 {
                let req = request(&mut e);
                assert!(!req.options.iter().any(react), "locked: {:?}", req.options);
                if e.action().is_none() {
                    break;
                }
                e.choose(fallback(&req));
            }
        } else {
            let u = first_use(&mut e, zvtes::cards::AbilitySource::Card(surge), 1);
            pick(&mut e, u);
            request(&mut e);
            assert_eq!(e.state.cards[surge.index()].counters, 3);
            assert!(e.state.cards[surge.index()].in_play, "no card played");
        }
    }
}

// Strike with Conviction, "Adding damage to strikes": [RTR 19970630] [TOM
// 19960225] "Additional damage inherits all of the properties of the base
// damage." Jack maneuvers to long range and strikes with his .44 Magnum
// (2R) at +1: 3 damage at long range, the added one ranged too; the
// vampire's hand strike lands nothing there.
// Rulings: [RTR 19970630] [TOM 19960225].
#[test]
fn strike_with_convictions_added_damage_is_the_weapons() {
    let (mut e, jack, v) = combat_table("Jack \"Hannibal137\" Harmon");
    let gun = on(&mut e, jack, ".44 Magnum");
    let sw = on(&mut e, jack, "Strike with Conviction");
    blocked_by(&mut e, v, jack);
    let away = first_use(&mut e, zvtes::cards::AbilitySource::Vampire(jack), 0);
    pick(&mut e, away);
    let u = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(sw),
        ability: 1,
        actor: None,
        target: Target::Card(gun),
    };
    pick_when(&mut e, u);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(
        (blood(&e, v), blood(&e, jack)),
        (5 - 3, 4),
        "his life untouched"
    );
    assert!(!e.state.cards[sw.index()].in_play, "burned");
}

// Foresee, "Action continue as if unblocked": [ANK 20190116] "If the action
// continues \"as if unblocked\", any Methuselahs that have declined to block
// before do not get another opportunity to block." Seat 1's hunt: its prey
// declines, its predator's imbued blocks (Second Sight for the hunt's
// stealth) and cancels the combat with
// Foresee: the prey is not asked again.
// Rulings: [ANK 20190116].
#[test]
fn foresees_continued_action_asks_no_decliner_again() {
    let mut e = game(3);
    let m = imbued(&mut e, 0, "Earl \"Shaka74\" Deams");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let x = e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    let fs = on(&mut e, m, "Foresee");
    let ss = on(&mut e, m, "Second Sight");
    on(&mut e, m, "Second Sight");
    hand(&mut e, 1, &["Govern the Unaligned"]);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, action(v, zvtes::decision::ActionKind::Hunt));
    let req = expect(&mut e, SeatId(2), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(x)));
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, block(m));
    // The hunt's +1 stealth met.
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Card(ss), 0);
    pick(&mut e, u);
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Card(fs), 0);
    pick(&mut e, u);
    pick(&mut e, Choice::CancelCombat);
    for _ in 0..40 {
        let req = request(&mut e);
        if e.last_outcome.is_some() {
            break;
        }
        assert!(
            !req.options.contains(&block(x)),
            "asked again: {:?}",
            req.options
        );
        e.choose(fallback(&req));
    }
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
}

// Waiting Game: "each ally burns 1 life" — an imbued at its last life is
// incapacitated, not burned; one incapacitated has none to burn (rulebook
// "Appendix: Imbued Rules" 5; rules-qa 87.12).
#[test]
fn waiting_game_incapacitates_an_imbued_and_spares_an_incapacitated_one() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let a = imbued(&mut e, 1, "Marion \"Teacher193\" Perks");
    let b = imbued(&mut e, 1, "Pedro Cortez");
    e.state.minions[a.index()].blood = 1;
    e.state.minions[b.index()].blood = 1;
    e.damage(b, 1);
    assert!(e.state.minions[b.index()].incapacitated);
    let card = find_library("Waiting Game");
    e.state
        .cards
        .push(CardInPlay::new(card, SeatId(1), None, 10));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    for m in [a, b] {
        let v = &e.state.minions[m.index()];
        assert!(v.in_play && v.incapacitated, "{m:?} not burned");
    }
}

// Donate's [REACTION]: "+1 intercept to the attempting mortal you control"
// — another imbued's power reaches a block its own intercept cannot: the
// attempt is not futile, and made with Donate it blocks the hunt's +1
// stealth (engine.md §3: third-party intercept counted by the reach).
#[test]
fn donate_makes_another_mortals_block_reachable() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    let d = imbued(&mut e, 0, "Travis \"Traveler72\" Miller");
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let don = on(&mut e, d, "Donate");
    on(&mut e, d, "React with Conviction");
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, action(v, zvtes::decision::ActionKind::Hunt));
    expect(&mut e, SeatId(0), WindowLabel::ActionBlock);
    assert!(
        e.block_can_reach(SeatId(0), m),
        "Donate's intercept counted"
    );
    pick(&mut e, block(m));
    let u = first_use(&mut e, zvtes::cards::AbilitySource::Card(don), 1);
    pick(&mut e, u);
    request(&mut e);
    assert_eq!(e.state.combatants, Some([v, m]), "blocked");
}

// Rejuvenate's unlock-phase life is gained incapacitated too, the imbued
// staying there: it leaves only by its own rule (rules-qa 87.43, the
// owner).
#[test]
fn rejuvenate_gives_life_to_an_incapacitated_imbued_left_there() {
    let mut e = game(2);
    let m = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    on(&mut e, m, "Rejuvenate");
    e.state.minions[m.index()].blood = 1;
    e.damage(m, 1);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    to_minion_phase(&mut e, SeatId(0));
    let v = &e.state.minions[m.index()];
    assert!(
        v.incapacitated && v.blood == 1,
        "{:?}",
        (v.incapacitated, v.blood)
    );
}

// Strike with Conviction's weapon strike is the weapon's own, its
// requirements read (rules-qa 87.41): at close range a Sniper Rifle's
// ("only usable at long range") is not offered, a .44 Magnum's is.
#[test]
fn strike_with_conviction_reads_the_weapons_own_requirements() {
    let (mut e, m, v) = combat_table("Marion \"Teacher193\" Perks");
    let rifle = on(&mut e, m, "Sniper Rifle");
    let gun = on(&mut e, m, ".44 Magnum");
    let sw = on(&mut e, m, "Strike with Conviction");
    blocked_by(&mut e, v, m);
    let with = |w| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(sw),
        ability: 1,
        actor: None,
        target: Target::Card(w),
    };
    let req = offering(&mut e, with(gun));
    assert!(!req.options.contains(&with(rifle)), "{:?}", req.options);
}

// A strike card's "use a melee weapon strike" is a strike that is not a
// hand strike: under Immortal Grapple's "only hand strikes this round",
// Lucky Blow keeps its hand strike, not its weapon one (rules-qa 87.41's
// one routine, its bars).
#[test]
fn lucky_blows_weapon_strike_is_barred_under_immortal_grapple() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("POT", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let baton = on(&mut e, z, "Argent Baton");
    hand(&mut e, 0, &["Immortal Grapple"]);
    hand(&mut e, 1, &["Lucky Blow"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    let weapon = Choice::Play {
        card: 0,
        mode: 1,
        actor: Some(z),
        target: Target::Card(baton),
    };
    assert!(req.options.contains(&play(0, 0, z)), "{:?}", req.options);
    assert!(!req.options.contains(&weapon), "{:?}", req.options);
}

// A gun's maneuver commits its strike (LSJ 20090528-2-2); Strike with
// Conviction may still make that gun's strike at +1, as Target Vitals aims
// it (rules-qa 87.42, the owner): to long range with the .44 Magnum's own
// maneuver, then 3 damage there.
#[test]
fn strike_with_conviction_strikes_with_the_gun_its_maneuver_committed() {
    let (mut e, m, v) = combat_table("Marion \"Teacher193\" Perks");
    let gun = on(&mut e, m, ".44 Magnum");
    let sw = on(&mut e, m, "Strike with Conviction");
    blocked_by(&mut e, v, m);
    let away = first_use(&mut e, zvtes::cards::AbilitySource::Card(gun), 1);
    pick(&mut e, away);
    let u = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(sw),
        ability: 1,
        actor: None,
        target: Target::Card(gun),
    };
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(req.options.contains(&u), "{:?}", req.options);
    pick(&mut e, u);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, v), 5 - 3);
}
