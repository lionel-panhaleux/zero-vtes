//! Slice 8c: library search — Magic of the Smith's search at resolution for
//! an equipment the vampire can equip, Vast Wealth's first equipment found
//! working down from the top (requirements and cost as normal, shuffle
//! afterward). Windows with a single option are elided, so the scripts
//! only stop where a real decision exists.

use zvtes::cards::AbilitySource;
use zvtes::cards::{self, Effect, Layer, Scope, Target};
use zvtes::crypt::CRYPT;
use zvtes::decision::{Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, CardId, CardInPlay};

fn request(e: &mut Engine) -> DecisionRequest {
    match e.run() {
        Step::Decision(req) => {
            if std::env::var("TRACE").is_ok() {
                eprintln!("REQ {:?} {:?} {:?}", req.seat, req.window, req.options);
            }
            req
        }
        Step::Over(_) => panic!("game over while expecting a decision"),
    }
}

fn expect(e: &mut Engine, seat: SeatId, window: WindowLabel) -> DecisionRequest {
    let req = request(e);
    assert_eq!((req.seat, req.window), (seat, window), "{:?}", req.options);
    req
}

fn pick(e: &mut Engine, want: Choice) {
    let req = request(e);
    let idx = req
        .options
        .iter()
        .position(|o| *o == want)
        .unwrap_or_else(|| {
            panic!(
                "option {want:?} not in {:?} ({:?})",
                req.options, req.window
            )
        });
    e.choose(idx);
}

/// A card already on `host`, with its layers (no play).
fn put_on(e: &mut Engine, name: &str, host: MinionId) -> u32 {
    let card = find_library(name);
    let owner = e.state.minions[host.index()].owner;
    e.state.cards.push(CardInPlay {
        card,
        owner,
        controller: owner,
        host: Some(host),
        body: None,
        mode: 0,
        by: None,
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
        stored: Vec::new(),
        effect_only: false,
    });
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
    id.0
}

/// Seat 0's library, bottom first.
fn library(e: &mut Engine, cards: &[&str]) {
    e.state.seats[0].library = cards.iter().map(|c| find_library(c)).collect();
}

fn granted(card: u32, actor: MinionId) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(CardId(card)),
        ability: 0,
        actor: Some(actor),
        target: Target::None,
    }
}

/// The `nth` V5 vampire with superior `disc`.
fn vampire_with(disc: &str, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.disciplines.contains(&disc) && zvtes::cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no vampire with {disc}"))
}

fn in_play(e: &Engine, name: &str) -> Option<MinionId> {
    let card = find_library(name);
    e.state
        .cards
        .iter()
        .find(|c| c.card == card && c.in_play)
        .and_then(|c| c.host)
}

/// 3 seats. Seat 0: T (THA, not Ravnos) and B, 4 blood each, 4 pool; seat
/// 1: X; seat 2 empty.
fn table() -> (Engine, MinionId, MinionId) {
    let mut e = Engine::bare(3);
    let t_card = vampire_with("THA", 0);
    assert_ne!(CRYPT[t_card as usize].clan, "Ravnos");
    let t = e.add_minion(SeatId(0), t_card, 4, false);
    e.add_minion(SeatId(0), find_card("Farah Sarroub"), 4, false);
    let x = e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
    e.state.seats[0].pool = 4;
    (e, t, x)
}

// Magic of the Smith declares nothing: at resolution the vampire picks an
// equipment it can equip and pay for — not a second vehicle, not a clan
// equipment of another clan, not a pool cost above the pool — or none.
// Rulings: [RBK search].
#[test]
fn magic_of_the_smith_searches_at_resolution_for_an_equipment_the_vampire_can_equip() {
    let (mut e, t, _) = table();
    put_on(&mut e, "Unlicensed Taxicab", t);
    library(
        &mut e,
        &[
            "Sport Bike",
            "Treasured Samadji",
            "Assault Rifle",
            "Kevlar Vest",
            "Conditioning",
        ],
    );
    // A full hand: playing the card draws only its replacement (the top
    // card) before the action resolves.
    e.state.seats[0]
        .hand
        .push(find_library("Magic of the Smith"));
    for _ in 0..6 {
        e.state.seats[0].hand.push(find_library("Deflection"));
    }
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(t),
            target: Target::None,
        },
    );
    let kevlar = find_library("Kevlar Vest");
    let req = expect(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    assert_eq!(req.options, vec![Choice::Pass, Choice::FindCard(kevlar)]);
    pick(&mut e, Choice::FindCard(kevlar));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(in_play(&e, "Kevlar Vest"), Some(t));
    assert_eq!(e.state.seats[0].pool, 4 - 1);
    assert_eq!(e.state.minions[t.index()].blood, 4 - 1);
    // 5 cards, less the replacement drawn and the Kevlar Vest found.
    assert_eq!(e.state.seats[0].library.len(), 5 - 1 - 1);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Magic of the Smith")));
}

// Vast Wealth: the first equipment found from the top is a Ravnos clan
// equipment the vampire cannot equip — burned without cost; the equipment
// below it stays in the library.
// Rulings: [TOM 19950407].
#[test]
fn vast_wealth_burns_a_first_find_it_cannot_equip() {
    let (mut e, t, _) = table();
    let wealth = put_on(&mut e, "Vast Wealth", t);
    library(&mut e, &["Sport Bike", "Treasured Samadji", "Conditioning"]);
    pick(&mut e, granted(wealth, t));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Treasured Samadji")));
    assert_eq!(in_play(&e, "Treasured Samadji"), None);
    assert_eq!(e.state.seats[0].library.len(), 2);
    assert_eq!(e.state.seats[0].pool, 4);
}

// Vast Wealth: a second Laptop Computer cannot be placed on the vampire, so
// it is burned without cost.
// Rulings: [TOM 19950407].
#[test]
fn vast_wealth_burns_a_second_laptop_computer_without_cost() {
    let (mut e, t, _) = table();
    let wealth = put_on(&mut e, "Vast Wealth", t);
    put_on(&mut e, "Laptop Computer", t);
    library(&mut e, &["Conditioning", "Laptop Computer"]);
    let pool = e.state.seats[0].pool;
    pick(&mut e, granted(wealth, t));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Laptop Computer")));
    assert_eq!(e.state.seats[0].pool, pool);
}

// Vast Wealth: paying for the equipment found is not optional — an Assault
// Rifle's 5 pool ousts a Methuselah with 3.
// Rulings: [RTR 19941109] [RTR 20010711].
#[test]
fn vast_wealth_pays_a_pool_cost_that_ousts() {
    let (mut e, t, _) = table();
    let wealth = put_on(&mut e, "Vast Wealth", t);
    library(&mut e, &["Assault Rifle"]);
    e.state.seats[0].pool = 3;
    pick(&mut e, granted(wealth, t));
    request(&mut e);
    assert!(e.state.seats[0].ousted);
}

// Vast Wealth: a blocked action searches nothing.
// Rulings: [TOM 19951107].
#[test]
fn vast_wealth_searches_only_if_its_action_succeeds() {
    let (mut e, t, x) = table();
    let wealth = put_on(&mut e, "Vast Wealth", t);
    // The bike's +1 intercept meets the action's +1 stealth.
    put_on(&mut e, "Sport Bike", x);
    library(&mut e, &["Kevlar Vest", "Conditioning"]);
    pick(&mut e, granted(wealth, t));
    pick(&mut e, Choice::DeclareBlock { minion: x });
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(in_play(&e, "Kevlar Vest"), None);
    assert_eq!(
        e.state.seats[0].library,
        vec![find_library("Kevlar Vest"), find_library("Conditioning")]
    );
}
