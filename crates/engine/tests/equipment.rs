//! Slice 8a: the equip action and equipment on a minion — equipping from
//! hand (rulebook "Equip": +1 stealth, undirected, the cost paid at
//! resolution) and from another minion (free, the equipment announced with
//! the action), "a minion can have only one vehicle", and what equipment
//! gives its bearer: stealth, bleed and intercept, a strike, a granted
//! bleed, an "if blocked" burn. Windows with a single option are elided, so
//! the scripts only stop where a real decision exists.

use zvtes::actions::Outcome;
use zvtes::cards::AbilitySource;
use zvtes::cards::{self, Effect, Layer, Scope, Target};
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
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

/// Pass (or hand strike) through every decision until `seat`'s `window`.
fn until(e: &mut Engine, seat: SeatId, window: WindowLabel) -> DecisionRequest {
    loop {
        let req = request(e);
        if (req.seat, req.window) == (seat, window) {
            return req;
        }
        let i = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::Pass | Choice::HandStrike))
            .unwrap_or(0);
        e.choose(i);
    }
}

fn hand(e: &mut Engine, seat: u8, cards: &[&str]) {
    for c in cards {
        e.state.seats[seat as usize].hand.push(find_library(c));
    }
}

/// An equipment already on `host`, with its layers (no play).
fn equip(e: &mut Engine, name: &str, host: MinionId) -> u32 {
    let card = find_library(name);
    let owner = e.state.minions[host.index()].owner;
    e.state.cards.push(CardInPlay {
        card,
        owner,
        controller: owner,
        host: Some(host),
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

fn play(card: u8, actor: MinionId) -> Choice {
    Choice::Play {
        card,
        mode: 0,
        actor: Some(actor),
        target: Target::None,
    }
}

fn action(minion: MinionId, action: ActionKind) -> Choice {
    Choice::TakeAction {
        minion,
        action,
        target: None,
    }
}

fn take_from(minion: MinionId, giver: MinionId) -> Choice {
    Choice::TakeAction {
        minion,
        action: ActionKind::EquipFrom,
        target: Some(giver),
    }
}

fn granted(card: u32, actor: MinionId) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(CardId(card)),
        ability: 0,
        actor: Some(actor),
        target: Target::None,
    }
}

fn host_of(e: &Engine, card: u32) -> Option<MinionId> {
    let c = &e.state.cards[card as usize];
    c.in_play.then_some(c.host).flatten()
}

#[test]
fn equip_from_hand_is_an_undirected_plus_one_stealth_action_paid_at_resolution() {
    let mut e = Engine::bare(3);
    let horst = e.add_minion(SeatId(0), find_card("Horst von Brühl"), 3, false);
    let kevin = e.add_minion(SeatId(1), find_card("Kevin Jackson"), 3, false);
    // Sport Bike's +1 intercept lets the prey's vampire meet the +1 stealth.
    equip(&mut e, "Sport Bike", kevin);
    hand(&mut e, 0, &["Sport Bike"]);
    let pool = e.state.seats[0].pool;
    pick(&mut e, play(0, horst));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req
        .options
        .contains(&Choice::DeclareBlock { minion: kevin }));
    assert_eq!(e.state.seats[0].pool, pool, "the cost waits for resolution");
    assert!(e.state.seats[0].hand.is_empty());
    pick(&mut e, Choice::Pass);
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, pool - 1);
    let bike = e
        .state
        .cards
        .iter()
        .position(|c| c.card == find_library("Sport Bike") && c.owner == SeatId(0))
        .expect("the bike entered play") as u32;
    assert_eq!(host_of(&e, bike), Some(horst));
}

#[test]
fn unlicensed_taxicab_gives_stealth_during_hunt_actions() {
    let mut e = Engine::bare(3);
    let horst = e.add_minion(SeatId(0), find_card("Horst von Brühl"), 3, false);
    let kevin = e.add_minion(SeatId(1), find_card("Kevin Jackson"), 3, false);
    equip(&mut e, "Unlicensed Taxicab", horst);
    equip(&mut e, "Sport Bike", kevin);
    pick(&mut e, action(horst, ActionKind::Hunt));
    // +2 stealth against 1 intercept: no block attempt is offered.
    let req = request(&mut e);
    assert_ne!(req.window, WindowLabel::ActionBlock, "{req:?}");
    assert_eq!(e.state.minions[horst.index()].blood, 4);
}

#[test]
fn a_minion_can_have_only_one_vehicle() {
    let mut e = Engine::bare(3);
    let horst = e.add_minion(SeatId(0), find_card("Horst von Brühl"), 3, false);
    let alice = e.add_minion(SeatId(0), find_card("Alice Chen"), 3, false);
    equip(&mut e, "Sport Bike", horst);
    let taxi = equip(&mut e, "Unlicensed Taxicab", alice);
    let codex = equip(&mut e, "Codex of the Edenic Groundskeepers", alice);
    hand(&mut e, 0, &["Stolen Police Cruiser"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&play(0, horst)));
    assert!(!req.options.contains(&play(0, alice)));
    // Alice cannot take the bike; Horst can take the Codex but not the
    // Taxicab, so the equipment terms elide.
    assert!(!req.options.contains(&take_from(alice, horst)));
    pick(&mut e, take_from(horst, alice));
    request(&mut e);
    assert_eq!(host_of(&e, codex), Some(horst));
    assert_eq!(host_of(&e, taxi), Some(alice));
}

/// Seat 0: Horst von Brühl and Alice Chen holding an Unlicensed Taxicab and
/// a Codex; seat 1: Kevin Jackson on a Sport Bike (1 intercept).
fn taxicab_on_alice() -> (Engine, MinionId, MinionId, MinionId, u32, u32) {
    let mut e = Engine::bare(3);
    let horst = e.add_minion(SeatId(0), find_card("Horst von Brühl"), 3, false);
    let alice = e.add_minion(SeatId(0), find_card("Alice Chen"), 3, false);
    let kevin = e.add_minion(SeatId(1), find_card("Kevin Jackson"), 3, false);
    let taxi = equip(&mut e, "Unlicensed Taxicab", alice);
    let codex = equip(&mut e, "Codex of the Edenic Groundskeepers", alice);
    equip(&mut e, "Sport Bike", kevin);
    (e, horst, alice, kevin, taxi, codex)
}

// Rulings: [LSJ 20060222].
#[test]
fn equip_from_another_minion_counts_each_equipments_own_stealth() {
    let (mut e, horst, alice, _, taxi, codex) = taxicab_on_alice();
    let pool = e.state.seats[0].pool;
    pick(&mut e, take_from(horst, alice));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionTerms);
    assert_eq!(
        req.options,
        vec![
            Choice::TakeEquipment { card: taxi },
            Choice::TakeEquipment { card: codex }
        ]
    );
    pick(&mut e, Choice::TakeEquipment { card: taxi });
    pick(&mut e, Choice::TakeEquipment { card: codex });
    // +1 stealth, +1 for the Taxicab: the bike's intercept falls short.
    let req = request(&mut e);
    assert_ne!(req.window, WindowLabel::ActionBlock, "{req:?}");
    assert_eq!(host_of(&e, taxi), Some(horst));
    assert_eq!(host_of(&e, codex), Some(horst));
    assert_eq!(e.state.seats[0].pool, pool, "free");
}

#[test]
fn a_blocked_equip_from_another_minion_leaves_the_equipment() {
    let (mut e, horst, alice, kevin, taxi, codex) = taxicab_on_alice();
    pick(&mut e, take_from(horst, alice));
    pick(&mut e, Choice::TakeEquipment { card: codex });
    // The Codex alone: +1 stealth, which the bike meets.
    pick(&mut e, Choice::Pass);
    pick(&mut e, Choice::DeclareBlock { minion: kevin });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
    assert_eq!(host_of(&e, codex), Some(alice));
    assert_eq!(host_of(&e, taxi), Some(alice));
}

/// Seat 0: Hafthor Thorsteinsson (Ravnos) with an Unlicensed Taxicab and a
/// Treasured Samadji; seat 1: `blocker`.
fn hafthor_blocked_by(blocker: &str) -> (Engine, MinionId, MinionId, u32) {
    let mut e = Engine::bare(3);
    let hafthor = e.add_minion(SeatId(0), find_card("Hafthor Thorsteinsson"), 4, false);
    let b = e.add_minion(SeatId(1), find_card(blocker), 4, false);
    let taxi = equip(&mut e, "Unlicensed Taxicab", hafthor);
    equip(&mut e, "Treasured Samadji", hafthor);
    pick(&mut e, action(hafthor, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: b });
    (e, hafthor, b, taxi)
}

// Rulings: [ANK 20220116] [ANK 20211207] [ANK 20210627] [RTR 19991206] [LSJ 20091125].
#[test]
fn unlicensed_taxicab_burns_as_a_prince_blocks_before_anything_else() {
    let (mut e, _, _, taxi) = hafthor_blocked_by("Kevin Jackson");
    // The first decision of the combat: the Taxicab is already gone.
    expect(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(!e.state.cards[taxi as usize].in_play);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Unlicensed Taxicab")));

    let (mut e, _, _, taxi) = hafthor_blocked_by("Octane");
    expect(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(
        e.state.cards[taxi as usize].in_play,
        "Octane holds no title"
    );
}

#[test]
fn treasured_samadji_ravnos_dodges_once_each_combat_and_bleeds_one_more() {
    let (mut e, hafthor, octane, _) = hafthor_blocked_by("Octane");
    let samadji = Choice::UseAbility {
        source: AbilitySource::Card(CardId(1)),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick(&mut e, samadji);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[hafthor.index()].blood, 4, "dodged");
    assert_eq!(e.state.minions[octane.index()].blood, 4);

    let mut e = Engine::bare(3);
    let hafthor = e.add_minion(SeatId(0), find_card("Hafthor Thorsteinsson"), 4, false);
    equip(&mut e, "Treasured Samadji", hafthor);
    let pool = e.state.seats[1].pool;
    pick(&mut e, action(hafthor, ActionKind::Bleed));
    request(&mut e);
    assert_eq!(e.state.seats[1].pool, pool - 2);
}

// Rulings: [ANK 20180719-3].
#[test]
fn treasured_samadji_taken_by_a_non_ravnos_gives_nothing() {
    let mut e = Engine::bare(3);
    let hafthor = e.add_minion(SeatId(0), find_card("Hafthor Thorsteinsson"), 4, false);
    let horst = e.add_minion(SeatId(0), find_card("Horst von Brühl"), 4, false);
    let samadji = equip(&mut e, "Treasured Samadji", hafthor);
    // A clan equipment in play: no requirement checked on the rulebook
    // action.
    pick(&mut e, take_from(horst, hafthor));
    request(&mut e);
    assert_eq!(host_of(&e, samadji), Some(horst));

    // Not Ravnos: no +1 bleed (rules-qa 9.1).
    let mut e = Engine::bare(3);
    let horst = e.add_minion(SeatId(0), find_card("Horst von Brühl"), 4, false);
    equip(&mut e, "Treasured Samadji", horst);
    let pool = e.state.seats[1].pool;
    pick(&mut e, action(horst, ActionKind::Bleed));
    request(&mut e);
    assert_eq!(e.state.seats[1].pool, pool - 1);
}

#[test]
fn codex_grants_a_bleed_costing_one_blood_with_three_more_against_no_ready_unlocked_minion() {
    let mut e = Engine::bare(3);
    let horst = e.add_minion(SeatId(0), find_card("Horst von Brühl"), 3, false);
    let kevin = e.add_minion(SeatId(1), find_card("Kevin Jackson"), 3, false);
    e.state.minions[kevin.index()].locked = true;
    let codex = equip(&mut e, "Codex of the Edenic Groundskeepers", horst);
    let pool = e.state.seats[1].pool;
    pick(&mut e, granted(codex, horst));
    request(&mut e);
    assert_eq!(e.state.minions[horst.index()].blood, 2);
    assert_eq!(e.state.seats[1].pool, pool - 4);

    // A ready unlocked minion at the target: the +3 is off, and it may
    // block the bleed (no stealth).
    let mut e = Engine::bare(3);
    let horst = e.add_minion(SeatId(0), find_card("Horst von Brühl"), 3, false);
    e.add_minion(SeatId(0), find_card("Alice Chen"), 3, false);
    let kevin = e.add_minion(SeatId(1), find_card("Kevin Jackson"), 3, false);
    let codex = equip(&mut e, "Codex of the Edenic Groundskeepers", horst);
    let pool = e.state.seats[1].pool;
    pick(&mut e, granted(codex, horst));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req
        .options
        .contains(&Choice::DeclareBlock { minion: kevin }));
    pick(&mut e, Choice::Pass);
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, pool - 1);
    // One bleed a turn: unlocked again, Horst has neither bleed left.
    e.state.minions[horst.index()].locked = false;
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&action(horst, ActionKind::Bleed)));
    assert!(!req.options.contains(&granted(codex, horst)));
    assert!(req.options.contains(&action(horst, ActionKind::Hunt)));
}

#[test]
fn stolen_police_cruiser_anarch_bleeds_one_more_and_younger_vampires_intercept_less() {
    let mut e = Engine::bare(3);
    let aline = e.add_minion(SeatId(0), find_card("Aline Gädeke"), 3, false);
    equip(&mut e, "Stolen Police Cruiser", aline);
    let pool = e.state.seats[1].pool;
    pick(&mut e, action(aline, ActionKind::Bleed));
    request(&mut e);
    assert_eq!(e.state.seats[1].pool, pool - 2);

    // Horst (capacity 6) is younger than Aline (7), Kevin is not: on a
    // Sport Bike each, only Kevin meets the hunt's +1 stealth.
    let mut e = Engine::bare(3);
    let aline = e.add_minion(SeatId(0), find_card("Aline Gädeke"), 3, false);
    let horst = e.add_minion(SeatId(1), find_card("Horst von Brühl"), 3, false);
    let kevin = e.add_minion(SeatId(1), find_card("Kevin Jackson"), 3, false);
    equip(&mut e, "Stolen Police Cruiser", aline);
    equip(&mut e, "Sport Bike", horst);
    equip(&mut e, "Sport Bike", kevin);
    pick(&mut e, action(aline, ActionKind::Hunt));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req
        .options
        .contains(&Choice::DeclareBlock { minion: kevin }));
    assert!(!req
        .options
        .contains(&Choice::DeclareBlock { minion: horst }));
}

#[test]
fn stolen_police_cruiser_burned_locks_its_anarch_through_their_next_unlock_phase() {
    let mut e = Engine::bare(3);
    let alice = e.add_minion(SeatId(0), find_card("Alice Chen"), 3, false);
    let aline = e.add_minion(SeatId(1), find_card("Aline Gädeke"), 3, false);
    let octane = e.add_minion(SeatId(1), find_card("Octane"), 3, false);
    let cruiser = equip(&mut e, "Stolen Police Cruiser", aline);
    let pool = e.state.seats[0].pool;
    pick(&mut e, granted(cruiser, alice));
    // Directed at the Cruiser's controller: their vampires may block.
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req
        .options
        .contains(&Choice::DeclareBlock { minion: octane }));
    pick(&mut e, Choice::Pass);
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, pool - 1);
    assert!(!e.state.cards[cruiser as usize].in_play);
    assert!(e.state.minions[aline.index()].locked);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        e.state.minions[aline.index()].locked,
        "did not unlock as normal"
    );
    assert!(!e.state.minions[aline.index()].skip_unlock);
}

#[test]
fn bowl_of_convergence_burns_blood_once_per_action_only_while_attempting_to_block() {
    let mut e = Engine::bare(3);
    let horst = e.add_minion(SeatId(0), find_card("Horst von Brühl"), 3, false);
    e.add_minion(SeatId(0), find_card("Alice Chen"), 3, false);
    // Alexander Silverson has superior Auspex: +1 intercept from the Bowl,
    // and its burn for 1 more.
    let alex = e.add_minion(SeatId(1), find_card("Alexander Silverson"), 3, false);
    equip(&mut e, "Unlicensed Taxicab", horst);
    let bowl = equip(&mut e, "Bowl of Convergence", alex);
    let burn = Choice::UseAbility {
        source: AbilitySource::Card(CardId(bowl)),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    // A +2 stealth hunt: the attempt is offered, reaching 2 with the burn.
    pick(&mut e, action(horst, ActionKind::Hunt));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&Choice::DeclareBlock { minion: alex }));
    assert!(!req.options.contains(&burn), "no attempt yet");
    pick(&mut e, Choice::DeclareBlock { minion: alex });
    pick(&mut e, burn);
    // Once per action: no second burn, so the rest of the block loop and
    // the combat elide; the burn's +1 made the block (2 against 2).
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
    // 3 blood, 1 burned for the Bowl, 1 to Horst's hand strike.
    assert_eq!(e.state.minions[alex.index()].blood, 1);
}

// A locquipment is only a location once in play (spec §1.3.1): no minion
// takes it from another with an equip action.
#[test]
fn a_locquipment_in_play_is_not_taken_from_another_minion() {
    let mut e = Engine::bare(2);
    let horst = e.add_minion(SeatId(0), find_card("Horst von Brühl"), 3, false);
    let alice = e.add_minion(SeatId(0), find_card("Kevin Jackson"), 3, false);
    e.add_minion(SeatId(1), find_card("Kevin Jackson"), 3, false);
    e.state.cards.push(CardInPlay {
        card: find_library("Living Manse"),
        owner: SeatId(0),
        controller: SeatId(0),
        host: Some(alice),
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
        stored: Vec::new(),
    });
    let req = request(&mut e);
    assert!(!req.options.contains(&take_from(horst, alice)));
    equip(&mut e, "Kevlar Vest", alice);
    let req = request(&mut e);
    assert!(req.options.contains(&take_from(horst, alice)));
}
