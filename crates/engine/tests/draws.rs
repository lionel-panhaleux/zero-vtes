//! Slice 8c-ii: cards stored on cards and draw alternatives — Black Market
//! Cache's search and burn, Shilmulo Tarot's stored cards, one treatment
//! per replacement beside Visit from the Capuchin, Heart of Nizchetus' draw
//! and bottom move. Windows with a single option are elided, so the scripts
//! only stop where a real decision exists.

use zvtes::cards::{self, Effect, Layer, Scope, Target};
use zvtes::crypt::CRYPT;
use zvtes::decision::{Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Contest, Engine, Step};
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_library, CardId, CardInPlay};

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

/// Pass every window until `seat`'s `window` opens as a real decision.
fn to_window(e: &mut Engine, seat: SeatId, window: WindowLabel) -> DecisionRequest {
    loop {
        let req = request(e);
        if (req.seat, req.window) == (seat, window) {
            return req;
        }
        assert!(matches!(req.options[0], Choice::Pass), "{req:?}");
        e.choose(0);
    }
}

/// A card already in play on `host` (or hostless), with its layers (no
/// play).
fn put(e: &mut Engine, name: &str, owner: u8, host: Option<MinionId>) -> u32 {
    let card = find_library(name);
    e.state.cards.push(CardInPlay {
        card,
        owner: SeatId(owner),
        controller: SeatId(owner),
        host,
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
        stored: Vec::new(),
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

fn cards(names: &[&str]) -> Vec<u16> {
    names.iter().map(|c| find_library(c)).collect()
}

/// Seat 0's library, bottom first.
fn library(e: &mut Engine, names: &[&str]) {
    e.state.seats[0].library = cards(names);
}

fn hand(e: &mut Engine, names: &[&str]) {
    e.state.seats[0].hand = cards(names);
}

fn sorted(mut v: Vec<u16>) -> Vec<u16> {
    v.sort_unstable();
    v
}

fn use_ability(card: u32) -> Choice {
    Choice::UseAbility {
        card,
        ability: 0,
        actor: None,
        target: Target::None,
    }
}

fn play_on(target: MinionId) -> Choice {
    Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(target),
    }
}

fn stored(card: u32, name: &str) -> Choice {
    Choice::DrawStored {
        card,
        stored: find_library(name),
    }
}

/// The `nth` V5 vampire of `clan`.
fn of_clan(clan: &str, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(_, c)| c.clan == clan)
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no {clan} vampire"))
}

/// 3 seats, seat 0 active with 10 pool: R (Ravnos) and B (Brujah), 4 blood
/// each; nobody else has a minion.
fn table() -> (Engine, MinionId, MinionId) {
    let mut e = Engine::bare(3);
    let r = e.add_minion(SeatId(0), of_clan("Ravnos", 0), 4, false);
    let b = e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    e.state.seats[0].pool = 10;
    (e, r, b)
}

// Black Market Cache is replaced first — here by the top Kevlar Vest
// (rules-qa 12.4) — then the non-unique equipment left are chosen one at a
// time (a unique one is not offered) and the library shuffled; a later
// draw may take one of them (rules-qa 9.4, 11.3).
#[test]
fn black_market_cache_is_replaced_before_storing_non_unique_equipment() {
    let (mut e, _, b) = table();
    e.at_master_phase();
    e.state.seats[0].mpa = 2;
    let mut h = vec!["Black Market Cache", "Vast Wealth"];
    h.extend(["Deflection"; 5]);
    hand(&mut e, &h);
    library(
        &mut e,
        &[
            "Conditioning",
            "Heart of Nizchetus",
            "Sport Bike",
            "Kevlar Vest",
            "Kevlar Vest",
        ],
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
    let (kevlar, bike) = (find_library("Kevlar Vest"), find_library("Sport Bike"));
    let req = expect(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    // The replacement already drew one Kevlar Vest.
    assert!(e.state.seats[0].hand.contains(&kevlar));
    let mut found = [Choice::FindCard(kevlar), Choice::FindCard(bike)];
    found.sort_by_key(|c| match c {
        Choice::FindCard(k) => *k,
        _ => unreachable!(),
    });
    assert_eq!(req.options[0], Choice::Pass);
    assert_eq!(req.options[1..], found[..]);
    pick(&mut e, Choice::FindCard(kevlar));
    let req = expect(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    assert_eq!(req.options, vec![Choice::Pass, Choice::FindCard(bike)]);
    pick(&mut e, Choice::Pass);
    let cache = 0;
    assert!(e.state.cards[0].in_play && !e.state.cards[0].contested);
    assert_eq!(e.state.cards[0].stored, vec![kevlar]);
    assert_eq!(e.state.seats[0].hand.len(), 7);
    assert_eq!(e.state.seats[0].library.len(), 5 - 2);
    // Vast Wealth's replacement may be the Cache's card.
    let wealth = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Vast Wealth"))
        .unwrap() as u8;
    pick(
        &mut e,
        Choice::Play {
            card: wealth,
            mode: 0,
            actor: None,
            target: Target::Minion(b),
        },
    );
    let req = expect(&mut e, SeatId(0), WindowLabel::Draw);
    assert_eq!(
        req.options,
        vec![Choice::DrawLibrary, stored(cache, "Kevlar Vest")]
    );
}

// Black Market Cache: a search finding nothing leaves the location without
// cards — burned at once (rules-qa 11.3); the replacement is drawn as usual.
#[test]
fn black_market_cache_finding_nothing_is_burned_at_once() {
    let (mut e, _, _) = table();
    e.at_master_phase();
    let mut h = vec!["Black Market Cache"];
    h.extend(["Deflection"; 6]);
    hand(&mut e, &h);
    library(&mut e, &["Conditioning", "Heart of Nizchetus"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::None,
        },
    );
    request(&mut e);
    assert!(!e.state.cards[0].in_play);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Black Market Cache")));
    assert_eq!(e.state.seats[0].hand.len(), 7);
}

// Black Market Cache: its last card drawn, the location is burned.
#[test]
fn black_market_cache_is_burned_as_its_last_card_is_drawn() {
    let (mut e, _, b) = table();
    let cache = put(&mut e, "Black Market Cache", 0, None);
    e.state.cards[cache as usize].stored = cards(&["Kevlar Vest"]);
    e.at_master_phase();
    let mut h = vec!["Vast Wealth"];
    h.extend(["Deflection"; 6]);
    hand(&mut e, &h);
    library(&mut e, &["Conditioning"]);
    pick(&mut e, play_on(b));
    let req = expect(&mut e, SeatId(0), WindowLabel::Draw);
    assert_eq!(
        req.options,
        vec![Choice::DrawLibrary, stored(cache, "Kevlar Vest")]
    );
    pick(&mut e, stored(cache, "Kevlar Vest"));
    request(&mut e);
    assert!(!e.state.cards[cache as usize].in_play);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Black Market Cache")));
    assert_eq!(e.state.seats[0].library, cards(&["Conditioning"]));
}

// Beside Visit from the Capuchin, each replacement takes one treatment:
// the Capuchin's counter or a card from a draw alternative — not the top
// card, the Capuchin's "instead" standing otherwise.
// Rulings: [ANK 20200905].
#[test]
fn visit_from_the_capuchin_or_a_draw_alternative_is_one_treatment_per_replacement() {
    let (mut e, _, b) = table();
    let capuchin = put(&mut e, "Visit from the Capuchin", 0, None);
    e.state.cards[capuchin as usize].counters = 2;
    let cache = put(&mut e, "Black Market Cache", 0, None);
    e.state.cards[cache as usize].stored = cards(&["Kevlar Vest"]);
    e.at_master_phase();
    // Hand size 9 with two counters: a full hand.
    let mut h = vec!["Vast Wealth"];
    h.extend(["Deflection"; 8]);
    hand(&mut e, &h);
    library(&mut e, &["Conditioning", "Deflection"]);
    pick(&mut e, play_on(b));
    let req = expect(&mut e, SeatId(0), WindowLabel::Draw);
    assert_eq!(
        req.options,
        vec![
            Choice::ResolveRider { card: capuchin },
            stored(cache, "Kevlar Vest")
        ]
    );
    pick(&mut e, Choice::ResolveRider { card: capuchin });
    request(&mut e);
    assert_eq!(e.state.cards[capuchin as usize].counters, 1);
    assert_eq!(e.state.seats[0].hand.len(), 8);
    assert_eq!(e.state.seats[0].library.len(), 2);
    assert_eq!(e.state.cards[cache as usize].stored.len(), 1);
}

// Shilmulo Tarot equipped by an effect (Vast Wealth's find) takes the top
// two cards of the library as it is equipped — before it enters contest
// with another Methuselah's copy.
// Rulings: [LSJ 20090506] [ANK 20221028] [LSJ 20090428-1].
#[test]
fn shilmulo_tarot_equipped_by_a_search_takes_its_cards_before_entering_contest() {
    let (mut e, r, _) = table();
    let y = e.add_minion(SeatId(1), of_clan("Ravnos", 1), 4, false);
    e.state.minions[y.index()].locked = true;
    let theirs = put(&mut e, "Shilmulo Tarot", 1, Some(y));
    let wealth = put(&mut e, "Vast Wealth", 0, Some(r));
    library(&mut e, &["Conditioning", "Deflection", "Shilmulo Tarot"]);
    pick(
        &mut e,
        Choice::UseAbility {
            card: wealth,
            ability: 0,
            actor: Some(r),
            target: Target::None,
        },
    );
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let tarot = e.state.cards.len() - 1;
    let c = &e.state.cards[tarot];
    assert_eq!((c.card, c.host), (find_library("Shilmulo Tarot"), Some(r)));
    assert_eq!(
        sorted(c.stored.clone()),
        sorted(cards(&["Conditioning", "Deflection"]))
    );
    assert!(c.contested && e.state.cards[theirs as usize].contested);
    assert!(e.state.seats[0].library.is_empty());
}

// Shilmulo Tarot: "while this Ravnos is ready" — its cards are offered for
// a draw while its Ravnos bearer is ready; not while it is in torpor, nor
// once a bearer that is not Ravnos holds it (read on the current bearer,
// rules-qa 9.1).
// Rulings: [ANK 20180719-3].
#[test]
fn shilmulo_tarot_offers_its_cards_only_while_a_ready_ravnos_bears_it() {
    let (mut e, r, b) = table();
    let tarot = put(&mut e, "Shilmulo Tarot", 0, Some(r));
    e.state.cards[tarot as usize].stored = cards(&["Kevlar Vest"]);
    e.at_master_phase();
    e.state.seats[0].mpa = 3;
    let mut h = vec!["Vast Wealth", "Vast Wealth", "Vast Wealth"];
    h.extend(["Deflection"; 4]);
    hand(&mut e, &h);
    library(&mut e, &["Assault Rifle", "Conditioning", "Sport Bike"]);
    pick(&mut e, play_on(b));
    let req = expect(&mut e, SeatId(0), WindowLabel::Draw);
    assert_eq!(
        req.options,
        vec![Choice::DrawLibrary, stored(tarot, "Kevlar Vest")]
    );
    pick(&mut e, Choice::DrawLibrary);
    // In torpor: the replacement is drawn from the library.
    e.state.minions[r.index()].in_torpor = true;
    pick(&mut e, play_on(b));
    request(&mut e);
    assert_eq!(e.state.seats[0].library, cards(&["Assault Rifle"]));
    // On a ready Brujah: likewise.
    e.state.minions[r.index()].in_torpor = false;
    e.state.cards[tarot as usize].host = Some(b);
    pick(&mut e, play_on(b));
    request(&mut e);
    assert!(e.state.seats[0].library.is_empty());
    assert_eq!(
        e.state.cards[tarot as usize].stored,
        cards(&["Kevlar Vest"])
    );
}

// Shilmulo Tarot: "during your unlock phase, you can move the top card of
// your library to this equipment" — once per unlock phase, adding to the
// cards already on it (no limit).
// Rulings: [PIB 20110712-2].
#[test]
fn shilmulo_tarot_moves_the_top_card_once_each_unlock_phase() {
    let (mut e, r, _) = table();
    let tarot = put(&mut e, "Shilmulo Tarot", 0, Some(r));
    e.state.cards[tarot as usize].stored = cards(&["Kevlar Vest", "Sport Bike"]);
    library(&mut e, &["Conditioning", "Deflection"]);
    // A full hand (no draw on the way round); the Edge keeps the unlock
    // window open once the Tarot is used.
    hand(&mut e, &["Deflection"; 7]);
    e.state.seats[0].has_edge = true;
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&use_ability(tarot)));
    pick(&mut e, use_ability(tarot));
    assert_eq!(
        e.state.cards[tarot as usize].stored,
        cards(&["Kevlar Vest", "Sport Bike", "Deflection"])
    );
    assert_eq!(e.state.seats[0].library, cards(&["Conditioning"]));
    let req = expect(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(!req.options.contains(&use_ability(tarot)));
}

// A Shilmulo Tarot leaving play burns the cards on it: yielded in contest,
// it goes to the ash heap with them.
// Rulings: [LSJ 20021128].
#[test]
fn a_shilmulo_tarot_leaving_play_burns_the_cards_on_it() {
    let (mut e, r, _) = table();
    let y = e.add_minion(SeatId(1), of_clan("Ravnos", 1), 4, false);
    let tarot = put(&mut e, "Shilmulo Tarot", 0, Some(r));
    let theirs = put(&mut e, "Shilmulo Tarot", 1, Some(y));
    e.state.cards[tarot as usize].stored = cards(&["Kevlar Vest", "Sport Bike"]);
    e.state.cards[tarot as usize].contested = true;
    e.state.cards[theirs as usize].contested = true;
    // Seat 2's turn first: seat 0's unlock phase comes before seat 1's.
    e.state.active = SeatId(2);
    let item = Contest::Library(CardId(tarot));
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&Choice::YieldContest(item)));
    pick(&mut e, Choice::YieldContest(item));
    assert!(!e.state.cards[tarot as usize].in_play);
    assert!(e.state.cards[tarot as usize].stored.is_empty());
    assert_eq!(
        sorted(e.state.seats[0].ash.clone()),
        sorted(cards(&["Shilmulo Tarot", "Kevlar Vest", "Sport Bike"]))
    );
}

// Heart of Nizchetus: the number is picked as the ability is used (1 to 3)
// and capped by the library — 3 picked, 2 in the library: 2 drawn, with no
// discard-down before 2 are moved back to the bottom, one at a time, the
// last one chosen the bottom card (rules-qa 9.5, 11.1).
#[test]
fn heart_of_nizchetus_draws_the_number_picked_capped_by_the_library_then_as_many_to_the_bottom() {
    let (mut e, r, _) = table();
    let heart = put(&mut e, "Heart of Nizchetus", 0, Some(r));
    hand(&mut e, &["Deflection"; 7]);
    library(&mut e, &["Kevlar Vest", "Sport Bike"]);
    // The Edge keeps the unlock window open once the Heart is used.
    e.state.seats[0].has_edge = true;
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&use_ability(heart)));
    pick(&mut e, use_ability(heart));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(
        req.options,
        vec![Choice::SetX(1), Choice::SetX(2), Choice::SetX(3)]
    );
    pick(&mut e, Choice::SetX(3));
    let req = expect(&mut e, SeatId(0), WindowLabel::BottomCards);
    assert_eq!(req.options.len(), 9);
    // Drawn: Sport Bike (the top) then Kevlar Vest, at hand indices 7, 8.
    let h = &e.state.seats[0].hand;
    assert_eq!(h[7..], cards(&["Sport Bike", "Kevlar Vest"])[..]);
    pick(&mut e, Choice::Bottom(8));
    pick(&mut e, Choice::Bottom(7));
    let req = expect(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(!req.options.contains(&use_ability(heart)));
    assert_eq!(e.state.seats[0].hand, cards(&["Deflection"; 7]));
    assert_eq!(
        e.state.seats[0].library,
        cards(&["Sport Bike", "Kevlar Vest"])
    );
    assert!(e.state.seats[0].ash.is_empty());
}
