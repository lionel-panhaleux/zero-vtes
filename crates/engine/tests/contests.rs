//! Slice 5c: library-card contests (rulebook "Contested Cards"; spec
//! §1.13) with the hunting grounds as tenants — unique locations with an
//! optional unlock-phase pick of a vampire. Windows with a single option
//! are elided, so the scripts only stop where a real decision exists.

use zvtes::cards::AbilitySource;
use zvtes::cards::Target;
use zvtes::decision::{Choice, WindowLabel};
use zvtes::game::{Contest, Engine};
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, CardId, CardInPlay};

mod common;
use common::*;

fn feed(card: u32, ability: u8, vampire: MinionId) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(CardId(card)),
        ability,
        actor: None,
        target: Target::Minion(vampire),
    }
}

fn in_play(e: &mut Engine, name: &str, owner: u8) -> u32 {
    e.state
        .cards
        .push(CardInPlay::new(find_library(name), SeatId(owner), None, 0));
    e.state.cards.len() as u32 - 1
}

/// 3 seats at seat 0's master phase, turn 4, 10 pool each. Seat 0: Alexa
/// Draper (Camarilla Prince, 5 blood) and Mateo García (capacity 2, 1
/// blood); seat 1: Kevin Jackson (Camarilla Prince, 5 blood); seat 2:
/// Lenny Burkhead (primogen, 4 blood, unnamed).
struct Table {
    e: Engine,
    alexa: MinionId,
    mateo: MinionId,
    kevin: MinionId,
}

fn table() -> Table {
    let mut e = Engine::bare(3);
    let alexa = e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    let mateo = e.add_minion(SeatId(0), find_card("Mateo García"), 1, false);
    let kevin = e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
    e.add_minion(SeatId(2), find_card("Lenny Burkhead"), 4, false);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.state.turn = 4;
    e.at_master_phase();
    Table {
        e,
        alexa,
        mateo,
        kevin,
    }
}

// A hunting ground (each is a clan card: Uptown is Ventrue): put in play
// (2 pool, no host); "during your unlock
// phase, a ready vampire you control can gain 1 blood" — the controller's
// pick, once per unlock phase per copy; "only one hunting ground each
// turn" per vampire (rules-qa 6.5): a second ground cannot feed the same
// vampire, and a vampire at capacity is still a legal pick.
#[test]
fn hunting_ground_feeds_one_vampire_per_ground_once_per_turn() {
    let Table {
        mut e,
        alexa,
        mateo,
        ..
    } = table();
    hand(&mut e, 0, &["Uptown Hunting Ground"]); // Ventrue: Alexa's clan
    pick(&mut e, master_play(0));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 8);
    assert!(e.state.cards[0].in_play && e.state.cards[0].host.is_none());
    let park = in_play(&mut e, "Park Hunting Ground", 0);
    // Seat 0's next unlock phase: both grounds, both vampires.
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert_eq!(
        abilities(&req),
        vec![
            feed(0, 0, alexa),
            feed(0, 0, mateo),
            feed(park, 0, alexa),
            feed(park, 0, mateo)
        ]
    );
    pick(&mut e, feed(0, 0, mateo));
    assert_eq!(blood(&e, mateo), 2);
    // The Zoo is spent; the Park cannot feed Mateo again this turn.
    let req = expect(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert_eq!(abilities(&req), vec![feed(park, 0, alexa)]);
    pick(&mut e, feed(park, 0, alexa));
    assert_eq!(blood(&e, alexa), 6);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
}

// A vampire at capacity is still offered (DESIGN §4.5 mask policy) and
// counts as fed.
#[test]
fn hunting_ground_at_capacity_is_a_legal_pick_that_still_feeds() {
    let Table {
        mut e,
        alexa,
        mateo,
        ..
    } = table();
    e.state.minions[mateo.index()].blood = 2;
    let zoo = in_play(&mut e, "Zoo Hunting Ground", 0);
    let park = in_play(&mut e, "Park Hunting Ground", 0);
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(abilities(&req).contains(&feed(zoo, 0, mateo)));
    pick(&mut e, feed(zoo, 0, mateo));
    assert_eq!(blood(&e, mateo), 2);
    let req = expect(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert_eq!(abilities(&req), vec![feed(park, 0, alexa)]);
}

// Papillon: playable only with a ready vampire holding a city title; its
// pick is a titled vampire, for 2 blood. Carfax Abbey: needs a ready
// Anarch to play; a first Anarch pick, and a second one only with a ready
// baron, each its own once-per-phase use.
#[test]
fn papillon_and_carfax_abbey_requirements_and_picks() {
    let Table {
        mut e,
        alexa,
        mateo,
        kevin,
        ..
    } = table();
    hand(&mut e, 0, &["Papillon", "Carfax Abbey"]);
    hand(&mut e, 1, &["Papillon"]);
    // Seat 0 has a prince: Papillon is offered; no Anarch: Carfax is not.
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let plays: Vec<u8> = req
        .options
        .iter()
        .filter_map(|o| match o {
            Choice::Play { card, .. } => Some(*card),
            _ => None,
        })
        .collect();
    assert_eq!(plays, vec![0]);
    pick(&mut e, master_play(0));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 7);
    // Seat 1 plays its Papillon too: both contested (face down, out of
    // play) — seat 0's copy feeds nobody at its next unlock phase.
    let req = to_window(&mut e, SeatId(1), WindowLabel::MasterPhase);
    assert!(req
        .options
        .iter()
        .any(|o| matches!(o, Choice::Play { card: 0, .. })));
    pick(&mut e, master_play(0));
    let _ = request(&mut e);
    assert!(e.state.cards[0].contested && e.state.cards[1].contested);
    // Seat 0 pays; the contested Papillon feeds nobody.
    to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    pick(&mut e, Choice::PayContest(Contest::Library(CardId(0))));
    let req = to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(abilities(&req).is_empty());
    let _ = (alexa, mateo, kevin);
}

// Two Methuselahs bring the same unique location into play: both copies
// are contested — out of play, no ability — until one yields (its copy
// burned) and the other's returns face up at its next unlock phase. The
// upkeep is 1 pool per unlock phase (rulebook "Contested Cards").
#[test]
fn unique_library_card_contest_pay_yield_and_return() {
    let Table { mut e, alexa, .. } = table();
    hand(&mut e, 0, &["Uptown Hunting Ground"]); // both seats are Ventrue
    hand(&mut e, 1, &["Uptown Hunting Ground"]);
    pick(&mut e, master_play(0));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[0].contested);
    // Seat 1's master phase: the second copy enters and both contest.
    to_window(&mut e, SeatId(1), WindowLabel::MasterPhase);
    pick(&mut e, master_play(0));
    let _ = request(&mut e);
    assert!(e.state.cards[0].contested && e.state.cards[1].contested);
    assert!(e.state.cards[0].in_play && e.state.cards[1].in_play);
    // Seat 0's unlock phase: pay 1 pool; no feeding while contested.
    let item = Contest::Library(CardId(0));
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert_eq!(
        req.options,
        vec![Choice::PayContest(item), Choice::YieldContest(item)]
    );
    pick(&mut e, Choice::PayContest(item));
    assert_eq!(e.state.seats[0].pool, 7);
    let req = to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::UseAbility { .. })));
    // Seat 1 yields: its copy is burned to its ash heap.
    to_window(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    pick(&mut e, Choice::YieldContest(Contest::Library(CardId(1))));
    assert!(!e.state.cards[1].in_play);
    assert_eq!(
        e.state.seats[1].ash,
        vec![find_library("Uptown Hunting Ground")]
    );
    // Seat 0's next unlock: no rival left, the copy returns and feeds.
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(!e.state.cards[0].contested);
    assert!(abilities(&req).contains(&feed(0, 0, alexa)));
}

// "You cannot voluntarily contest cards with yourself": a unique card is
// not playable while a copy is in play under this seat, contested or not
// (rules-qa 7.1); a different unique card is.
#[test]
fn unique_card_is_masked_while_a_copy_is_in_play_under_the_same_seat() {
    let Table { mut e, .. } = table();
    in_play(&mut e, "Uptown Hunting Ground", 0);
    // Uptown (Ventrue) masked by the copy in play; Society (Toreador,
    // Mateo's clan) offered.
    hand(
        &mut e,
        0,
        &["Uptown Hunting Ground", "Society Hunting Ground"],
    );
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let plays: Vec<u8> = req
        .options
        .iter()
        .filter_map(|o| match o {
            Choice::Play { card, .. } => Some(*card),
            _ => None,
        })
        .collect();
    assert_eq!(plays, vec![1]);
    e.state.cards[0].contested = true;
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let plays: Vec<u8> = req
        .options
        .iter()
        .filter_map(|o| match o {
            Choice::Play { card, .. } => Some(*card),
            _ => None,
        })
        .collect();
    assert_eq!(plays, vec![1]);
}

// Carfax Abbey's two picks: an Anarch first; the second only with a ready
// baron and for another Anarch (the first is fed this turn); each pick is
// its own once-per-phase use, the second offered once the first is made
// (the same options twice otherwise).
#[test]
fn carfax_abbey_second_pick_needs_a_baron_and_another_anarch() {
    let mut e = Engine::bare(2);
    let saku = e.add_minion(SeatId(0), find_card("Saku Pihlajamäki"), 3, false);
    let anita = e.add_minion(SeatId(0), find_card("Anita Wainwright"), 1, false);
    e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.state.turn = 4;
    e.at_master_phase();
    let carfax = in_play(&mut e, "Carfax Abbey", 0);
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert_eq!(
        abilities(&req),
        vec![feed(carfax, 0, saku), feed(carfax, 0, anita)]
    );
    pick(&mut e, feed(carfax, 0, anita));
    assert_eq!(blood(&e, anita), 2);
    // The first pick is spent and Anita fed: the second pick, Saku only.
    let req = expect(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert_eq!(abilities(&req), vec![feed(carfax, 1, saku)]);
    pick(&mut e, feed(carfax, 1, saku));
    assert_eq!(blood(&e, saku), 4);
    // Without a ready baron the second pick is never offered.
    e.state.minions[saku.index()].in_torpor = true;
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert_eq!(abilities(&req), vec![feed(carfax, 0, anita)]);
}

// The cards on a contested vampire are out of play with it (rulebook
// "Contested Cards", advanced rules §1.13.3, §6.4.1; rules-qa 7.3, 83.1):
// Baltimore Purge's unlock rider is not offered and Laptop Computer's +1
// bleed is off while it is contested; its rival yielded, it returns at its
// next unlock phase with them, the Purge resolving (its vampire to
// torpor).
#[test]
fn the_cards_on_a_contested_vampire_are_out_of_play_with_it() {
    let mut e = Engine::bare(3);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.state.turn = 4;
    let v = plain("Ventrue", 0);
    let m = e.add_minion(SeatId(0), v, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    let on = |e: &mut Engine, name: &str| {
        let card = find_library(name);
        e.state
            .cards
            .push(CardInPlay::new(card, SeatId(0), Some(m), 0));
        let id = CardId(e.state.cards.len() as u32 - 1);
        for effect in zvtes::cards::lookup(card).expect("defined").modes[0].effects {
            if let zvtes::cards::Effect::Layer(modifier) = effect {
                e.state.layers.push(zvtes::cards::Layer {
                    scope: zvtes::cards::Scope::InPlay,
                    modifier: *modifier,
                    source: id,
                });
            }
        }
        id
    };
    let purge = on(&mut e, "Baltimore Purge");
    let laptop = on(&mut e, "Laptop Computer");
    let rider = Choice::ResolveRider { card: purge.0 };
    let bleed = e.state.current_bleed(m);
    e.state.seats[1].uncontrolled = vec![zvtes::state::UncontrolledSlot {
        card: v,
        copy: Default::default(),
        counters: 10,
        banished: None,
    }];
    to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    request(&mut e);
    let rival = MinionId(
        e.state
            .minions
            .iter()
            .position(|o| o.owner == SeatId(1) && o.crypt_card() == Some(v))
            .expect("the rival") as u32,
    );
    assert!(e.state.minions[m.index()].contested);
    assert!(e.state.cards[laptop.index()].out_of_play);
    assert_eq!(
        e.state.current_bleed(m),
        bleed - 1,
        "the Laptop's bleed off"
    );
    // Seat 0's unlock phase: its contest's upkeep, no Purge.
    never_offered_where(&mut e, SeatId(0), WindowLabel::UnlockAbilities, |o| {
        *o == rider
    });
    pick(&mut e, Choice::PayContest(Contest::Card(m)));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, rider);
    assert!(!e.state.minions[m.index()].in_torpor);
    to_window(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    pick(&mut e, Choice::YieldContest(Contest::Card(rival)));
    // Seat 0's next unlock phase: back with its cards, the Purge resolving.
    for _ in 0..300 {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        let i = req
            .options
            .iter()
            .position(|o| *o == rider)
            .unwrap_or_else(|| fallback(&req));
        e.choose(i);
    }
    assert!(!e.state.minions[m.index()].contested);
    assert!(!e.state.cards[laptop.index()].out_of_play);
    assert_eq!(e.state.current_bleed(m), bleed, "the Laptop's bleed back");
    assert!(e.state.minions[m.index()].in_torpor, "the Purge resolved");
}

// A non-unique title card's contest is its vampire's title's (rules-qa
// 19.1), not a library contest: a rival claimant's vampire becoming
// contested, its Fee Stake out of play with it (83.1), leaves the other's
// Fee Stake contested with its title.
#[test]
fn a_rival_contested_with_its_title_card_leaves_the_title_contest_alone() {
    let mut e = Engine::bare(3);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.state.turn = 4;
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let bv = plain("Ventrue", 0);
    let b = e.add_minion(SeatId(1), bv, 6, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    let stake = find_library("Fee Stake: Boston");
    let mut on = |owner: u8, host: MinionId| {
        e.state
            .cards
            .push(CardInPlay::new(stake, SeatId(owner), Some(host), 0));
        CardId(e.state.cards.len() as u32 - 1)
    };
    let mine = on(0, a);
    let theirs = on(1, b);
    for (m, c) in [(a, mine), (b, theirs)] {
        e.state.minions[m.index()].title_contested = true;
        e.state.cards[c.index()].contested = true;
    }
    e.state.seats[2].uncontrolled = vec![zvtes::state::UncontrolledSlot {
        card: bv,
        copy: Default::default(),
        counters: 10,
        banished: None,
    }];
    for _ in 0..300 {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(2), WindowLabel::InfluenceTransfers) {
            break;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    // Asserted before seat 0's turn: alone in its contest, its title
    // returns then.
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    assert!(e.state.minions[b.index()].contested);
    assert!(e.state.cards[theirs.index()].out_of_play);
    assert!(e.state.minions[a.index()].title_contested);
    assert!(e.state.cards[mine.index()].contested, "still its title's");
}
