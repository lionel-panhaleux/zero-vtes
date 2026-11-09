//! Round 27 (`ADDED_BATCHES`, slice 16am): Secrets of New York and Noites
//! Brasileiras (owner, 2026-11-08). Readings in rules-qa Batch 112. Windows
//! with a single option are elided, so the scripts only stop where a real
//! decision exists.

use zvtes::cards::{AbilitySource, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_library, CardId, CardInPlay};

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

/// The `nth` untitled vampire matching `f` with no printed-text
/// definition.
fn textless(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            f(c) && c.title == Title::None
                && !c.imbued
                && zvtes::cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a textless vampire")
}

/// The untitled textless vampire `nth` of `sect`, capacity 4 or more.
fn of_sect(sect: Sect, nth: usize) -> u16 {
    textless(|c| c.sect == Some(sect) && c.capacity >= 4, nth)
}

/// The crypt card `name` (not advanced).
fn find_card(name: &str) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && !c.adv)
        .unwrap_or_else(|| panic!("no {name}")) as u16
}

/// A card already in play for `seat`, on `host` if given: its id.
#[allow(dead_code)]
fn in_play(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>) -> CardId {
    e.state
        .cards
        .push(CardInPlay::new(find_library(name), SeatId(seat), host, 0));
    CardId(e.state.cards.len() as u32 - 1)
}

/// Minion `m`'s printed ability `ability` aimed at `target`, used outside
/// its own action.
fn printed(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: None,
        target,
    }
}

/// Minion `m`'s printed action `ability` at `target`.
fn printed_action(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: Some(m),
        target,
    }
}

/// Pick the first option matching `want`, the fallback until then.
fn pick_matching(e: &mut Engine, want: impl Fn(&Choice) -> bool) {
    for _ in 0..200 {
        let req = request(e);
        if let Some(i) = req.options.iter().position(&want) {
            e.choose(i);
            return;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    panic!("never offered");
}

// ---- Printed stats ----------------------------------------------------------

/// Luana do Lago: "+1 hand size." Rey, o Mentor and Tereza Rossi: "+1
/// strength."
#[test]
fn luana_rey_and_tereza_print_their_stats() {
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("Luana do Lago"), 4, false);
    let r = e.add_minion(SeatId(0), find_card("Rey, o Mentor"), 4, false);
    let t = e.add_minion(SeatId(1), find_card("Tereza Rossi"), 4, false);
    assert_eq!(e.state.hand_size(SeatId(0)), 8);
    assert_eq!(e.state.hand_size(SeatId(1)), 7);
    let printed = |m| {
        e.state
            .printed_mod(m, zvtes::cards::Stat::Strength, Default::default())
    };
    assert_eq!((printed(r), printed(t)), (1, 1));
}

/// Durok: "+1 intercept during actions directed at a card you control": an
/// action at his Methuselah's minion; not a bleed of the Methuselah.
#[test]
fn durok_intercepts_actions_at_his_cards() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Thomas Arturo"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let d = e.add_minion(SeatId(1), find_card("Durok"), 4, false);
    let titled = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[titled.index()].granted_title = Some(Title::Primogen);
    pick_when(&mut e, printed_action(a, 0, Target::Minion(titled)));
    let act = e.action().expect("the lock");
    assert_eq!(act.intercept_of(&e.state, d), 1);
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let d = e.add_minion(SeatId(1), find_card("Durok"), 4, false);
    pick_when(&mut e, bleed(a));
    assert_eq!(e.action().expect("the bleed").intercept_of(&e.state, d), 0);
}

/// Mel: "Mel cannot block vampires with a city title": a prince, not a
/// primogen.
#[test]
fn mel_cannot_block_a_city_title() {
    for (title, blocks) in [(Title::Prince("Paris"), false), (Title::Primogen, true)] {
        let mut e = game(2);
        // A Camarilla vampire: a prince's title off-sect would be dormant.
        let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        e.state.minions[a.index()].granted_title = Some(title);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let m = e.add_minion(SeatId(1), find_card("Mel, a Guerreira Vibrante"), 4, false);
        pick_when(&mut e, bleed(a));
        let req = request(&mut e);
        assert_eq!(req.options.contains(&block(m)), blocks, "{title:?}");
    }
}

/// Dorinha: "Action cards cost Dorinha +1 blood": a political action card
/// costing nothing costs her 1 blood.
#[test]
fn dorinha_pays_more_for_action_cards() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Dorinha, a Confusa"), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, d));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, d), 0, "a free political action at +1 blood");
}

// ---- Abilities --------------------------------------------------------------

/// Addison Payne: "can play cards requiring a prince as if he were a
/// prince": a prince's reaction as he blocks (Victor Gerard's reading).
#[test]
fn addison_plays_a_princes_reaction() {
    let mut e = game(2);
    let ad = e.add_minion(SeatId(0), find_card("Addison Payne"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("obf", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Second Tradition: Domain"]);
    hand(&mut e, 1, &["Cloak the Gathering"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    pick_when(&mut e, block(ad));
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 0, ad));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10, "blocked");
}

/// Aisling Sturbridge (G6): "Once each turn, after Aisling performs a
/// successful action, she can discard a card requiring Blood Sorcery to
/// unlock." The G2 card's text is not hers.
#[test]
fn aisling_g6_discards_blood_sorcery_to_unlock() {
    let group = |g| {
        CRYPT
            .iter()
            .position(|c| c.name == "Aisling Sturbridge" && c.group == g)
            .unwrap() as u16
    };
    assert_eq!(zvtes::cards::vampire(group(2)).unwrap().printed.len(), 1);
    assert!(zvtes::cards::vampire(group(6)).unwrap().printed.is_empty());
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), group(6), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Theft of Vitae"]);
    pick_when(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, printed(a, 0, Target::None));
    request(&mut e);
    assert!(!e.state.minions[a.index()].locked);
    assert!(!e.state.seats[0]
        .hand
        .contains(&find_library("Theft of Vitae")));
}

/// Aríete: "can enter combat with a non-Sabbat vampire as a Ⓓ action";
/// Valerie Duval: "with a younger non-Camarilla vampire".
#[test]
fn ariete_and_valerie_enter_combat_with_their_targets() {
    let mut e = game(2);
    let ar = e.add_minion(SeatId(0), find_card("Aríete"), 4, false);
    let va = e.add_minion(SeatId(0), find_card("Valerie Duval"), 4, false);
    let cam = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 0), 4, false);
    let sab_card = of_sect(Sect::Sabbat, 0);
    let sab = e.add_minion(SeatId(1), sab_card, 4, false);
    let req = request(&mut e);
    assert!(req
        .options
        .contains(&printed_action(ar, 0, Target::Minion(cam))));
    assert!(!req
        .options
        .contains(&printed_action(ar, 0, Target::Minion(sab))));
    let sab_cap = CRYPT[sab_card as usize].capacity;
    assert_eq!(
        req.options
            .contains(&printed_action(va, 0, Target::Minion(sab))),
        sab_cap < 7
    );
    assert!(!req
        .options
        .contains(&printed_action(va, 0, Target::Minion(cam))));
}

/// Maria Quitéria: "1 optional additional strike (limited) each combat";
/// Tharmas: "Once each combat, … can burn 1 blood to get 1 optional
/// maneuver."
#[test]
fn maria_strikes_again_and_tharmas_maneuvers() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Maria Quitéria"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), find_card("Tharmas"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick_when(&mut e, bleed(m));
    pick_when(&mut e, block(t));
    pick_when(&mut e, printed(t, 0, Target::None));
    request(&mut e);
    assert_eq!(blood(&e, t), 3);
    pick_matching(&mut e, |o| *o == printed(m, 0, Target::None));
}

/// Thomas Arturo: "can lock a titled vampire controlled by another
/// Methuselah as a +1 stealth Ⓓ action that costs 1 blood."
#[test]
fn thomas_arturo_locks_a_titled_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Thomas Arturo"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let u = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[t.index()].granted_title = Some(Title::Prince("Paris"));
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&printed_action(a, 0, Target::Minion(u))));
    pick(&mut e, printed_action(a, 0, Target::Minion(t)));
    assert_eq!(e.action().expect("the lock").stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.index()].locked);
    assert_eq!(blood(&e, a), 3);
}

/// Torque: "Titled vampires can enter combat with him as a Ⓓ action": a
/// titled vampire of another Methuselah's, not an untitled one.
#[test]
fn titled_vampires_enter_combat_with_torque() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let u = e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Primogen);
    let t = e.add_minion(SeatId(1), find_card("Torque"), 4, false);
    let req = request(&mut e);
    let enter = |a: MinionId| Choice::UseAbility {
        source: AbilitySource::Vampire(t),
        ability: 1,
        actor: Some(a),
        target: Target::Minion(t),
    };
    assert!(req.options.contains(&enter(p)), "{:?}", req.options);
    assert!(!req.options.contains(&enter(u)));
}

/// Bruxa Branca: "Once each turn, Bruxa Branca can get +1 intercept for the
/// current action": as she attempts to block, once a turn.
#[test]
fn bruxa_branca_intercepts_once_a_turn() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Bruxa Branca"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("obf", 0), 4, false);
    let y = e.add_minion(SeatId(1), vampire_with("obf", 1), 4, false);
    hand(&mut e, 1, &["Cloak the Gathering", "Cloak the Gathering"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, printed(b, 0, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10, "blocked");
    // Unlocked again for the test: the turn's use is spent.
    e.state.minions[b.index()].locked = false;
    pick(&mut e, bleed(y));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, y));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        printed(b, 0, Target::None),
    );
}

/// Eduardo: "During the polling step of any referendum, if Eduardo is
/// ready, he can discard 1 card at random to get +2 votes." Once a
/// referendum.
#[test]
fn eduardo_discards_at_random_for_votes() {
    let mut e = game(2);
    let ed = e.add_minion(
        SeatId(0),
        find_card("Eduardo, o Oráculo Mascarado"),
        4,
        false,
    );
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested", "Dodge"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, alloc(0, 1));
    pick_when(&mut e, printed(ed, 0, Target::None));
    assert_eq!(e.referendum().expect("polling").bonus, vec![(ed, 2)]);
    assert!(!e.state.seats[0].hand.contains(&find_library("Dodge")));
    assert!(!request(&mut e)
        .options
        .contains(&printed(ed, 0, Target::None)));
}

/// Hope: "During a bleed action, Hope can burn 1 blood to get +1 stealth or
/// +1 bleed." One use a bleed: the stealth taken, the bleed no more.
#[test]
fn hope_burns_once_a_bleed_for_stealth_or_bleed() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hope"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(h));
    pick_when(&mut e, block(b));
    // The stealth taken: the bleed, its other way, is the same use.
    pick_when(&mut e, printed(h, 0, Target::None));
    assert!(!request(&mut e)
        .options
        .contains(&printed(h, 1, Target::None)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "unblocked, its bleed 1");
    assert_eq!(blood(&e, h), 3);
}

/// Kaiser: "Once each action, if Kaiser is ready, he can discard a card
/// requiring 1 or more Disciplines to reduce a bleed against you by 1."
#[test]
fn kaiser_discards_a_discipline_card_against_a_bleed() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Kaiser"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Theft of Vitae"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    pick_when(&mut e, printed(k, 0, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10, "reduced to 0");
    assert!(e.state.seats[0].hand.is_empty());
}

// ---- Library cards ----------------------------------------------------------

/// The ability `ability` of card `c` in play, no target.
fn card_ability(c: CardId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(c),
        ability,
        actor: None,
        target: Target::None,
    }
}

/// Backroom Deal: "[ACTION MODIFIER] Only usable by a ready primogen other
/// than the acting Camarilla vampire. The acting Camarilla vampire gets +1
/// stealth." Its first mode needs the acting minion a primogen.
#[test]
fn backroom_deal_lends_a_primogens_stealth() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let p = e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Primogen);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
    hand(&mut e, 0, &["Backroom Deal"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, a)), "not a primogen");
    pick(&mut e, play(0, 1, p));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "unblocked");
}

/// Double Spiral Corporation: "You can lock this location to discard a
/// card."
#[test]
fn double_spiral_corporation_locks_to_discard() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let c = in_play(&mut e, 0, "Double Spiral Corporation", None);
    hand(&mut e, 0, &["Dodge"]);
    pick_when(&mut e, card_ability(c, 0));
    request(&mut e);
    assert!(e.state.cards[c.index()].locked);
    assert!(e.state.seats[0].hand.is_empty());
}

/// Emboldened Councilor: "[REACTION] Only usable by a locked primogen. This
/// primogen unlocks and attempts to block."
#[test]
fn emboldened_councilor_unlocks_a_primogen_to_block() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Primogen);
    e.state.minions[p.index()].locked = true;
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 3), 4, false);
    hand(&mut e, 0, &["Emboldened Councilor"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[p.index()].locked = true;
    pick(&mut e, bleed(z));
    pick_when(&mut e, play(0, 1, p));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10, "blocked");
}

/// Lambreta: "If the bearer is acting, you can lock this vehicle to give a
/// blocking minion -1 intercept for the current action."
#[test]
fn lambreta_slows_a_blocker() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
    let l = in_play(&mut e, 0, "Lambreta", Some(a));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, card_ability(l, 0));
    assert!(e.state.cards[l.index()].locked);
    assert_eq!(e.action().expect("the bleed").intercept_of(&e.state, b), -1);
}

/// Meia Lua: "[cel][pot] Strike: dodge or strike: hand strike at +1
/// damage. [CEL][POT] Strike: dodge, with 1 additional strike: hand strike
/// at +2 damage (limited)." Three modes for a superior vampire.
#[test]
fn meia_lua_offers_its_three_strikes() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Rey, o Mentor"), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
    hand(&mut e, 0, &["Meia Lua"]);
    pick(&mut e, bleed(r));
    pick_when(&mut e, block(b));
    let req = loop {
        let req = request(&mut e);
        if req.window == WindowLabel::CombatStrike && req.seat == SeatId(0) {
            break req;
        }
        let i = fallback(&req);
        e.choose(i);
    };
    for mode in 0..3 {
        assert!(
            req.options.contains(&play(0, mode, r)),
            "{mode}: {:?}",
            req.options
        );
    }
}

/// Respected Representative: "Add 1 blood to a non-titled non-Caitiff
/// Camarilla vampire with capacity 4 or more you control and put this card
/// on them to represent the Camarilla title of primogen."
#[test]
fn respected_representative_makes_a_primogen() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 0), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Respected Representative"]);
    let at = |m| Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(m),
    };
    pick_when(&mut e, at(v));
    request(&mut e);
    assert_eq!(e.state.title_of(v), Title::Primogen);
    assert_eq!(blood(&e, v), 3);
}

// ---- Printed riders ---------------------------------------------------------

/// Caique: "After Caique enters combat, discard 1 card at random."
#[test]
fn caique_discards_as_he_enters_combat() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Caique"), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Dodge"]);
    pick(&mut e, bleed(c));
    pick_when(&mut e, block(b));
    request(&mut e);
    assert!(e.state.seats[0].hand.is_empty());
}

/// Padraic Conroy: "If Padraic blocks an older vampire, discard 1 card at
/// random before range is determined during the first round of the
/// resulting combat." Not blocking a younger one.
#[test]
fn padraic_discards_blocking_an_older_vampire() {
    for (older, discards) in [(true, true), (false, false)] {
        let mut e = game(2);
        // Tereza Rossi (capacity 8): her text only +1 strength.
        let card = if older {
            find_card("Tereza Rossi")
        } else {
            textless(|c| c.capacity <= 3, 0)
        };
        let a = e.add_minion(SeatId(0), card, 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let p = e.add_minion(SeatId(1), find_card("Padraic Conroy"), 4, false);
        hand(&mut e, 1, &["Dodge"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(p));
        request(&mut e);
        assert_eq!(e.state.seats[1].hand.is_empty(), discards, "older {older}");
    }
}

/// Hellene Panhard: "After a referendum called by Hellene fails, stun
/// her." Locked, and she skips her next unlock.
#[test]
fn hellene_is_stunned_after_her_referendum_fails() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hellene Panhard"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Justicar("Ventrue"));
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, h));
    pick_when(&mut e, alloc(0, 1));
    pick_when(&mut e, votes(p, false));
    close_polling(&mut e);
    request(&mut e);
    let v = &e.state.minions[h.index()];
    assert!(v.locked && v.skip_unlock);
}

/// Julia Sowinski: "If Julia successfully bleeds, you get +1 transfer this
/// turn": for the influence phase after her bleed.
#[test]
fn julia_gets_a_transfer_after_a_successful_bleed() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Julia Sowinski"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    pick(&mut e, bleed(j));
    // Taken at this turn's influence phase.
    pick_when(&mut e, bleed(b));
    assert_eq!(e.state.seats[0].transfers_next, 1);
}

/// Gianni D'Angelo: "Once each turn, Gianni can burn 1 blood to give
/// another Camarilla vampire you control +1 intercept": not to himself.
#[test]
fn gianni_lends_intercept_to_another_camarilla_vampire() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Gianni D'Angelo"), 4, false);
    let c = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("obf", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Cloak the Gathering"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    pick_when(&mut e, block(c));
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, printed(g, 0, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10, "blocked");
    assert_eq!(blood(&e, g), 3);
}

/// Katherine Wiese: "Once each turn, Katherine can burn 2 blood as she
/// announces a directed action to lock a younger vampire controlled by the
/// target Methuselah. +1 bleed."
#[test]
fn katherine_locks_a_younger_vampire_as_she_bleeds() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Katherine Wiese"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let y = e.add_minion(SeatId(1), textless(|c| c.capacity <= 5, 0), 4, false);
    pick(&mut e, bleed(k));
    pick_when(&mut e, printed(k, 0, Target::Minion(y)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[y.index()].locked);
    assert_eq!(blood(&e, k), 2);
    assert_eq!(e.state.seats[1].pool, 10 - 2, "unblocked, +1 bleed");
}

/// O Grande Arquiteto: "Once each turn, after O Grande Arquiteto performs a
/// successful directed action, he can put 1 corruption counter on a minion
/// controlled by the target Methuselah."
#[test]
fn o_grande_arquiteto_corrupts_after_a_directed_action() {
    let mut e = game(2);
    let o = e.add_minion(SeatId(0), find_card("O Grande Arquiteto"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[t.index()].locked = true;
    pick(&mut e, bleed(o));
    pick_when(&mut e, printed(o, 0, Target::Minion(t)));
    request(&mut e);
    assert_eq!(e.state.minions[t.index()].corruption_of(SeatId(0)), 1);
}

/// Carter Vanderweyden: "During referendums he calls, Carter gets +1 vote
/// for each other primogen you control": not in another's referendum.
#[test]
fn carter_gets_a_vote_per_other_primogen() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Carter Vanderweyden"), 4, false);
    let p = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Primogen);
    let votes = |e: &Engine, calling| {
        e.state.printed_mod(
            c,
            zvtes::cards::Stat::Votes,
            zvtes::state::PrintedCtx {
                calling: Some(calling),
                ..Default::default()
            },
        )
    };
    assert_eq!(votes(&e, c), 1);
    assert_eq!(votes(&e, p), 0);
}

/// Samira: "cannot cast votes or ballots against referendums called by a
/// prince": for, yes; against, not offered.
#[test]
fn samira_cannot_vote_against_a_prince() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Samira"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Kine Resources Contested"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, p));
    pick_when(&mut e, alloc(1, 1));
    let req = offering(&mut e, votes(s, true));
    assert!(!req.options.contains(&votes(s, false)), "{:?}", req.options);
}

/// O Encourado: "can prevent 1 aggravated damage each round": Blow Torch's
/// aggravated point prevented.
#[test]
fn o_encourado_prevents_aggravated_damage() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let o = e.add_minion(SeatId(1), find_card("O Encourado"), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let torch = in_play(&mut e, 0, "Blow Torch", Some(a));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(o));
    pick_matching(
        &mut e,
        |x| matches!(x, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == torch),
    );
    pick_when(&mut e, printed(o, 0, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[o.index()].in_torpor);
}

/// Qadir al-Asmai: "Once each game, during your unlock phase, Qadir can
/// equip with a weapon card from your hand": not again the next turn.
#[test]
fn qadir_equips_a_weapon_once_a_game() {
    let mut e = game(2);
    let q = e.add_minion(SeatId(0), find_card("Qadir al-Asmai"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &[".44 Magnum", ".44 Magnum"]);
    to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    pick_matching(
        &mut e,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == q),
    );
    request(&mut e);
    let on_q = e
        .state
        .cards
        .iter()
        .filter(|c| c.in_play && c.host == Some(q))
        .count();
    assert_eq!(on_q, 1, "one Magnum on Qadir");
    let qadirs = |o: &Choice| match o {
        Choice::UseAbility {
            source: AbilitySource::Vampire(v),
            ..
        } => *v == q,
        _ => false,
    };
    let uses = |e: &mut Engine| request(e).options.iter().any(qadirs);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(!uses(&mut e), "once each game");
}

/// Sydnelson: "If Sydnelson is ready, he can lock before range is
/// determined to end a combat involving two vampires controlled by other
/// Methuselahs."
#[test]
fn sydnelson_ends_a_combat_between_others() {
    let mut e = game(3);
    let s = e.add_minion(SeatId(0), find_card("Sydnelson"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let a = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let b = e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 3), 4, false);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_matching(
        &mut e,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == s),
    );
    request(&mut e);
    assert!(e.state.minions[s.index()].locked);
    assert_eq!((blood(&e, a), blood(&e, b)), (4, 4), "no strikes");
}

/// Vritra: "If a Methuselah controls more locations than you do, Vritra can
/// steal one of those locations as a Ⓓ action that costs 1 blood." Not
/// from a Methuselah with as many as his.
#[test]
fn vritra_steals_a_location_from_a_richer_methuselah() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Vritra"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let l = in_play(&mut e, 1, "Double Spiral Corporation", None);
    let own = in_play(&mut e, 0, "Barrens, The", None);
    let steal = printed_action(v, 0, Target::Card(l));
    assert!(!request(&mut e).options.contains(&steal), "one each");
    e.state.cards[own.index()].in_play = false;
    pick(&mut e, steal);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.card_controller(l), SeatId(0));
    assert_eq!(blood(&e, v), 3);
}

/// Tubarão: "During the polling step of any referendum, you can reveal the
/// top card of your crypt, then move it to the bottom; if it is an older
/// vampire, Tubarão gets +1 vote."
#[test]
fn tubarao_reveals_his_crypt_top_for_a_vote() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Tubarão"), 4, false);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let young = textless(|c| c.capacity <= 3, 0);
    let old = find_card("Tereza Rossi");
    e.state.seats[0].crypt.push(young);
    e.state.seats[0].crypt.push(old);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, alloc(0, 1));
    pick_when(&mut e, printed(t, 0, Target::None));
    assert_eq!(
        e.state.seats[0].crypt.last(),
        Some(&young),
        "the old one to the bottom"
    );
    let r = e.referendum().expect("polling");
    assert_eq!(r.bonus, vec![(t, 1)], "an older vampire: +1 vote");
    assert!(!request(&mut e)
        .options
        .contains(&printed(t, 0, Target::None)));
}

/// Kali: "Once each turn, if Kali is ready, you can discard 1 card at random
/// to reduce the cost of a location card or equipment card you play by 1
/// pool."
#[test]
fn kali_discards_at_random_to_cheapen_equipment() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Kali"), 4, false);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &[".44 Magnum", "Dodge"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::ResolvePrinted { minion: k });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10 - 1, "the Magnum's 2 pool less 1");
    assert!(e.state.seats[0].hand.is_empty(), "Dodge discarded");
}

/// Elie Sansaricq: "Once each combat, after Elie successfully inflicts any
/// damage on the opposing minion, put 1 corruption counter on that minion."
#[test]
fn elie_corrupts_the_minion_she_damages() {
    let mut e = game(2);
    let el = e.add_minion(SeatId(0), find_card("Elie Sansaricq"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick(&mut e, bleed(el));
    pick_when(&mut e, block(b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(blood(&e, b) < 4, "her hand strike landed");
    assert_eq!(e.state.minions[b.index()].corruption_of(SeatId(0)), 1);
}

/// Ability `ability` of card `c` in play aimed at minion `m`.
fn card_ability_at(c: CardId, ability: u8, m: MinionId) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(c),
        ability,
        actor: None,
        target: Target::Minion(m),
    }
}

/// Circulatory System Outlet: "During your unlock phase, a ready vampire
/// you control with no blood can gain 1 blood."
#[test]
fn circulatory_system_outlet_feeds_an_empty_vampire() {
    let mut e = game(2);
    let empty = e.add_minion(SeatId(0), plain("Brujah", 0), 0, false);
    // Locked: no starving hunt before the unlock phase.
    e.state.minions[empty.index()].locked = true;
    let fed = e.add_minion(SeatId(0), plain("Brujah", 1), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let c = in_play(&mut e, 0, "Circulatory System Outlet", None);
    let req = offering(&mut e, card_ability_at(c, 0, empty));
    assert!(!req.options.contains(&card_ability_at(c, 0, fed)));
    pick(&mut e, card_ability_at(c, 0, empty));
    request(&mut e);
    assert_eq!(blood(&e, empty), 1);
}

/// Palavra Sagrada: "During an action, you can lock a ready Caitiff you
/// control to give another vampire you control +1 stealth or +1
/// intercept."
#[test]
fn palavra_sagrada_locks_a_caitiff_for_stealth() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let c = e.add_minion(SeatId(0), find_card("Dorinha, a Confusa"), 1, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let p = in_play(&mut e, 0, "Palavra Sagrada, Rio de Janeiro", None);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    let req = request(&mut e);
    assert!(
        !req.options.contains(&card_ability_at(p, 0, a)),
        "not the acting"
    );
    pick(&mut e, card_ability_at(p, 0, c));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[c.index()].locked);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "unblocked");
}

/// Deadly Secrets [OBF]: "Force another vampire to abstain. After action
/// resolution, if the referendum was successful, you can draw 1 card."
#[test]
fn deadly_secrets_silences_and_draws_on_success() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Kaiser"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    hand(&mut e, 0, &["Kine Resources Contested", "Deadly Secrets"]);
    // A full hand and a library: the draw-up takes nothing, each played
    // card is replaced from the library.
    fill_hand(&mut e, 0);
    for _ in 0..10 {
        e.state.seats[0].library.push(find_library("Form of Mist"));
    }
    pick(&mut e, play(0, 0, k));
    pick_when(&mut e, alloc(0, 1));
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 1,
            actor: Some(k),
            target: Target::Minion(p),
        },
    );
    request(&mut e);
    let r = e.referendum().expect("polling");
    assert_eq!(r.abstaining, vec![p], "the prince abstains");
    pick_matching(&mut e, |o| *o == Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    // Two replacements and the draw.
    assert_eq!(e.state.seats[0].library.len(), 10 - 3);
}

/// St. Patrick's Cathedral: "lock this location to look at and reorder the
/// top 3 cards of your library. You can then move them to the bottom of
/// your library (in the same order)."
#[test]
fn st_patricks_reorders_then_bottoms_the_top_three() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let c = in_play(&mut e, 0, "St. Patrick's Cathedral, New York", None);
    // A full hand: the draw-up takes nothing from the library.
    fill_hand(&mut e, 0);
    let [w, x, y, z] = ["Dodge", "Deflection", "Conditioning", "Bum's Rush"].map(find_library);
    for card in [w, x, y, z] {
        e.state.seats[0].library.push(card);
    }
    pick_when(&mut e, card_ability(c, 0));
    // Top down: z, y, x kept so.
    pick(&mut e, Choice::FindCard(z));
    pick(&mut e, Choice::FindCard(y));
    pick(&mut e, Choice::ResolveRider { card: c.0 });
    let lib: Vec<u16> = e.state.seats[0].library.iter().copied().collect();
    assert_eq!(
        lib,
        vec![x, y, z, w],
        "the three under the fourth, same order"
    );
}

/// Academia de Campeões: "During your discard phase, you can lock this
/// location to move up to two combat cards requiring Potence from your ash
/// heap to the bottom of your library in any order": one, then stop.
#[test]
fn academia_moves_potence_combat_cards_to_the_library_bottom() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let c = in_play(&mut e, 0, "Academia de Campeões, São Paulo", None);
    fill_hand(&mut e, 0);
    let [grapple, dodge, top] = ["Immortal Grapple", "Dodge", "Deflection"].map(find_library);
    e.state.seats[0].library.push(top);
    for card in [grapple, grapple, dodge] {
        e.state.seats[0].ash.push(card);
    }
    let req = offering(&mut e, card_ability(c, 0));
    assert_eq!(req.window, WindowLabel::DiscardPhase);
    pick(&mut e, card_ability(c, 0));
    let req = request(&mut e);
    assert!(
        !req.options.contains(&Choice::FindCard(dodge)),
        "not Potence"
    );
    pick(&mut e, Choice::FindCard(grapple));
    pick(&mut e, Choice::Pass);
    let lib: Vec<u16> = e.state.seats[0].library.iter().copied().collect();
    assert_eq!(lib, vec![grapple, top]);
}

/// The Art Hole: "After a primogen you control performs a successful
/// action, you can lock this location to unlock that primogen."
#[test]
fn the_art_hole_unlocks_a_successful_primogen() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Primogen);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
    let art = in_play(&mut e, 0, "Art Hole, New York, The", None);
    pick(&mut e, action(p, ActionKind::Hunt));
    pick_when(&mut e, card_ability(art, 0));
    request(&mut e);
    assert!(!e.state.minions[p.index()].locked);
    assert!(e.state.cards[art.index()].locked);
}

/// The ally `name` in play, by its card.
fn ally_named(e: &Engine, name: &str) -> MinionId {
    e.state
        .minions
        .iter()
        .position(|x| x.in_play && x.name() == name)
        .map(|i| MinionId(i as u32))
        .expect("in play")
}

/// Cãoera: "This ally gets +1 bleed for each life above 2 it has": 0 bleed
/// at 2 life, 2 at 4.
#[test]
fn caoera_bleeds_for_its_life_above_two() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Cãoera"]);
    pick_when(&mut e, play(0, 0, n));
    to_minion_phase(&mut e, SeatId(1));
    let c = ally_named(&e, "Cãoera");
    assert_eq!(e.state.base_bleed(c), 0);
    e.state.minions[c.index()].blood = 4;
    assert_eq!(e.state.base_bleed(c), 2);
}

/// Vial of Zapathasura's Vitae: "This Ravnos gets +1 strength and +1 level
/// of the chosen Discipline. This equipment cannot be moved or stolen. If
/// this equipment is burned, send this vampire to torpor."
#[test]
fn the_vial_raises_a_discipline_and_its_burn_sends_to_torpor() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), plain("Ravnos", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let before = e.state.discipline_level(r, "obf");
    hand(&mut e, 0, &["Vial of Zapathasura's Vitae"]);
    pick_when(&mut e, play(0, 1, r));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.discipline_level(r, "obf"), before + 1);
    let v = (0..e.state.cards.len())
        .map(|i| CardId(i as u32))
        .find(|c| e.state.cards[c.index()].host == Some(r))
        .expect("on the Ravnos");
    assert!(e.state.pinned(v), "never moved or stolen");
    e.burn_card(v);
    assert!(e.state.minions[r.index()].in_torpor);
}

/// Ron Zertnert: "If another minion you control is in combat, he can lock
/// during the press step to inflict 1 unpreventable damage on the opposing
/// minion."
#[test]
fn ron_zertnert_hits_the_opponent_of_his_controllers_combatant() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ron = e.add_ally(SeatId(0), find_library("Ron Zertnert, S.A.D. Agent"));
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let card = (0..e.state.cards.len())
        .map(|i| CardId(i as u32))
        .find(|c| e.state.cards[c.index()].body == Some(ron))
        .expect("Ron's card");
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    let req = offering(&mut e, card_ability(card, 2));
    assert_eq!(req.window, WindowLabel::CombatPress);
    let before = blood(&e, b);
    pick(&mut e, card_ability(card, 2));
    assert!(e.state.minions[ron.index()].locked);
    assert_eq!(blood(&e, b), before - 1, "unpreventable, at once");
}

/// Destilaria Estrela da Paraíba: "During an action directed at you (or a
/// card you control), you can lock this location to give a Nosferatu you
/// control +1 intercept; if that Nosferatu blocks, you get +1 hand size
/// during the resulting combat."
#[test]
fn destilaria_gives_a_nosferatu_intercept_and_hand_size() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("obf", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let d = in_play(&mut e, 0, "Destilaria Estrela da Paraíba", None);
    hand(&mut e, 1, &["Cloak the Gathering"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    pick_when(&mut e, block(n));
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, card_ability(d, 0));
    // A strike card: the combat has a decision of seat 0's.
    hand(&mut e, 0, &["Dodge"]);
    until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(e.combat().is_some(), "blocked");
    assert_eq!(e.state.hand_size(SeatId(0)), 8);
}

/// Convento da Penha: "During a political action, any Methuselah can lock
/// this location to give an acting vampire they control +1 stealth; other
/// Methuselahs must give you 1 pool to use this effect."
#[test]
fn convento_da_penha_sells_stealth_for_a_pool() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let a = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    // Rinaldo Albizzi: +1 intercept, a political action's +1 stealth met.
    let b = e.add_minion(SeatId(0), find_card("Rinaldo Albizzi"), 4, false);
    let c = in_play(&mut e, 0, "Convento da Penha, Vila Velha", None);
    hand(&mut e, 1, &["Kine Resources Contested"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, card_ability(c, 1));
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 10 + 1);
    assert!(e.state.cards[c.index()].locked);
}

/// Hálito Negro: "Strike: 2R damage on the opposing minion and 1R damage on
/// all retainers on the opposing minion."
#[test]
fn halito_negro_hits_the_opposing_retainers() {
    let mut e = game(2);
    // The card requires the Path of Death and the Soul (rules-digest
    // §3.12).
    let death = textless(
        |c| {
            c.path == Some("Death and the Soul")
                && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("obl"))
        },
        0,
    );
    let a = e.add_minion(SeatId(0), death, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let spy = in_play(&mut e, 1, "Raven Spy", Some(b));
    e.state.cards[spy.index()].counters = 1;
    hand(&mut e, 0, &["Hálito Negro"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_matching(
        &mut e,
        |o| matches!(o, Choice::Play { card: 0, actor: Some(x), .. } if *x == a),
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[spy.index()].in_play, "the spy burned");
    assert_eq!(blood(&e, b), 4 - 2);
}

/// The card in play that is ally `m`.
fn ally_card(e: &Engine, m: MinionId) -> CardId {
    (0..e.state.cards.len())
        .map(|i| CardId(i as u32))
        .find(|c| e.state.cards[c.index()].body == Some(m))
        .expect("the ally's card")
}

/// Father Anthony: "Once each combat, Father Anthony can strike: dodge or
/// strike: stun the opposing minion": the vampire locked, its next unlock
/// skipped.
#[test]
fn father_anthony_stuns_the_opposing_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let fa = e.add_ally(SeatId(1), find_library("Father Anthony"));
    let card = ally_card(&e, fa);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(fa));
    pick_when(&mut e, card_ability(card, 1));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let v = &e.state.minions[a.index()];
    assert!(v.locked && v.skip_unlock);
}

/// Father Anthony: "While Father Anthony is in combat with a vampire, cards
/// requiring 1 or more Disciplines cost that vampire +1 blood."
#[test]
fn father_anthony_taxes_discipline_cards() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Rey, o Mentor"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let fa = e.add_ally(SeatId(1), find_library("Father Anthony"));
    hand(&mut e, 0, &["Meia Lua"]);
    pick(&mut e, bleed(r));
    pick_when(&mut e, block(fa));
    pick_when(&mut e, play(0, 1, r));
    request(&mut e);
    assert_eq!(blood(&e, r), 3, "Meia Lua costs Rey 1 blood");
}

/// Chave de Braço [POT]: "at the end of the round, if the range is close
/// and this vampire has successfully inflicted more damage than the
/// opposing minion, put this card on the opposing minion": the card itself,
/// out of the ash heap.
#[test]
fn chave_de_braco_goes_on_the_outdamaged_minion() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Rey, o Mentor"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Chave de Braço"]);
    pick(&mut e, bleed(r));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 1, r));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = find_library("Chave de Braço");
    let on = e
        .state
        .cards
        .iter()
        .find(|c| c.in_play && c.card == card)
        .expect("in play");
    assert_eq!(on.host, Some(b));
    assert!(!on.effect_only, "the card itself");
    assert!(!e.state.seats[0].ash.contains(&card));
}

/// Mulher de Branco: "Burn this ally unless you remove three cards in your
/// ash heap from the game. After a minion successfully bleeds you (for 1 or
/// more), you can shuffle this ally into your library to inflict 2 damage
/// on the acting minion."
#[test]
fn mulher_de_branco_pays_three_ash_cards_then_strikes_back() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), plain("Hecata", 0), 4, false);
    e.add_minion(SeatId(0), plain("Hecata", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let dodge = find_library("Dodge");
    for _ in 0..3 {
        e.state.seats[0].ash.push(dodge);
    }
    hand(&mut e, 0, &["Mulher de Branco"]);
    pick(&mut e, play(0, 0, h));
    // The first pick; the two after it are the only options left.
    pick_when(&mut e, Choice::FindCard(dodge));
    to_minion_phase(&mut e, SeatId(1));
    let w = ally_named(&e, "Mulher de Branco");
    assert!(e.state.seats[0].ash.is_empty(), "three removed");
    let card = ally_card(&e, w);
    pick(&mut e, bleed(z));
    pick_when(&mut e, card_ability(card, 0));
    request(&mut e);
    assert!(!e.state.minions[w.index()].in_play);
    // Shuffled in, then drawn by the draw-up: never burned.
    let card = find_library("Mulher de Branco");
    let seat = &e.state.seats[0];
    assert!(seat.library.contains(&card) || seat.hand.contains(&card));
    assert!(!seat.ash.contains(&card));
    assert_eq!(blood(&e, z), 2);
}

/// Bar Último Trago: "You can lock this location if a minion you control
/// successfully blocks; … during the first round of that combat, both
/// combatants take 1R environmental damage during normal strike
/// resolution."
#[test]
fn bar_ultimo_trago_hurts_both_combatants() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let bar = in_play(&mut e, 0, "Bar Último Trago, Campina Grande", None);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    pick_when(&mut e, block(b));
    pick_when(&mut e, card_ability(bar, 0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    // A hand strike and the environmental point each.
    assert_eq!((blood(&e, b), blood(&e, z)), (2, 2));
}

/// Iara: "you can lock Iara to give a blocking minion -1 intercept; if that
/// action is blocked, instead of resolving the block, Iara enters combat
/// with and locks the blocking minion": the acting vampire unhurt, Iara
/// in the combat.
#[test]
fn iara_takes_the_blocked_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let iara = e.add_ally(SeatId(0), find_library("Iara"));
    // Rinaldo Albizzi: +1 intercept, still blocking past Iara's -1.
    let b = e.add_minion(SeatId(1), find_card("Rinaldo Albizzi"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let card = ally_card(&e, iara);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, card_ability(card, 0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10, "blocked");
    assert_eq!(blood(&e, a), 4, "the acting vampire out of the combat");
    assert_eq!(blood(&e, iara), 3 - 1, "Iara struck");
}

/// Information Exchange: "Allocate 3 points among one or more Methuselahs
/// and choose one of those Methuselahs. Successful referendum means you
/// look at the chosen Methuselah's hand and discard up to one card of your
/// choice from it, and for each point allocated, each Methuselah burns 1
/// pool." The chosen one is one given points.
#[test]
fn information_exchange_discards_from_the_chosen_hand() {
    let mut e = game(3);
    let n = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Information Exchange"]);
    hand(&mut e, 1, &["Dodge"]);
    pick(&mut e, play(0, 0, n));
    // Seat by seat: none to seat 0, two to seat 1, the last to seat 2.
    pick(&mut e, alloc(0, 0));
    pick(&mut e, alloc(1, 2));
    let req = request(&mut e);
    assert!(
        req.options.contains(&Choice::ChooseSeat(SeatId(1))),
        "{req:?}"
    );
    assert!(
        !req.options.contains(&Choice::ChooseSeat(SeatId(0))),
        "no points"
    );
    pick(&mut e, Choice::ChooseSeat(SeatId(1)));
    pick_matching(&mut e, |o| {
        *o == Choice::CastCardVote { favor: true } || *o == votes(n, true)
    });
    close_polling(&mut e);
    pick_when(&mut e, Choice::FindCard(find_library("Dodge")));
    assert!(!e.state.seats[1].hand.contains(&find_library("Dodge")));
    assert_eq!(
        (e.state.seats[1].pool, e.state.seats[2].pool),
        (10 - 2, 10 - 1)
    );
}

/// Porto de Santos: "Gain 1 pool. Each Methuselah looks at up to 3 cards
/// from the top of their crypt and moves one of those cards to their
/// uncontrolled region and the others to the bottom of their crypt." Once
/// a game for its player.
#[test]
fn porto_de_santos_moves_a_crypt_card_for_everyone() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let crypt: Vec<u16> = (0..4).map(|k| textless(|c| c.capacity >= 4, k)).collect();
    for s in 0..2 {
        for &c in &crypt {
            e.state.seats[s].crypt.push(c);
        }
    }
    hand(&mut e, 0, &["Porto de Santos", "Porto de Santos"]);
    pick_when(&mut e, master_play(0));
    // The top three: crypt[1..4]; crypt[2] kept by both.
    pick(&mut e, Choice::AshVampire(crypt[2]));
    pick(&mut e, Choice::AshVampire(crypt[2]));
    for s in 0..2 {
        let seat = &e.state.seats[s];
        assert_eq!(seat.uncontrolled.len(), 1);
        assert_eq!(seat.uncontrolled[0].card, crypt[2]);
        let left: Vec<u16> = seat.crypt.iter().copied().collect();
        assert_eq!(
            left,
            vec![crypt[1], crypt[3], crypt[0]],
            "the others under the rest"
        );
    }
    assert_eq!(e.state.seats[0].pool, 11);
    // The next master phase: the other master card playable, not Porto.
    hand(&mut e, 0, &["Barrens, The"]);
    let req = to_window(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let porto = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Porto de Santos"))
        .expect("the second copy") as u8;
    let barrens = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Barrens, The"))
        .expect("in hand") as u8;
    assert!(!req.options.contains(&master_play(porto)), "one a game");
    assert!(req.options.contains(&master_play(barrens)), "{req:?}");
}

/// Aaron Fletcher: "Put up to three equipment cards from your hand and/or
/// ash heap on Aaron, face down … Minions you control can equip with one of
/// those cards (face up) as a +1 stealth equip action."
#[test]
fn aaron_fletcher_stores_equipment_and_lends_it() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let magnum = find_library(".44 Magnum");
    e.state.seats[0].ash.push(magnum);
    hand(
        &mut e,
        0,
        &["Aaron Fletcher, Silent Investor", ".44 Magnum"],
    );
    pick(&mut e, play(0, 0, v));
    // From the hand, then the ash heap (each emptied: no more to offer).
    pick_when(&mut e, Choice::FindCard(magnum));
    pick(&mut e, Choice::FindCard(magnum));
    let aaron = ally_named(&e, "Aaron Fletcher, Silent Investor");
    let card = ally_card(&e, aaron);
    assert_eq!(e.state.cards[card.index()].stored.len(), 2);
    assert!(e.state.seats[0].ash.is_empty());
    to_minion_phase(&mut e, SeatId(0));
    let w = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    let equip = Choice::UseAbility {
        source: AbilitySource::Card(card),
        ability: 0,
        actor: Some(w),
        target: Target::None,
    };
    let req = request(&mut e);
    assert!(req.options.contains(&equip), "{:?}", req.options);
    // Two copies alike: one choice, not two.
    let second = |o: &Choice| {
        matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: 1, .. }
            if *c == card)
    };
    assert!(!req.options.iter().any(second));
    pick(&mut e, equip);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let on_w = e
        .state
        .cards
        .iter()
        .filter(|c| c.in_play && c.host == Some(w))
        .count();
    assert_eq!(on_w, 1, "a Magnum on the vampire");
    assert_eq!(e.state.cards[card.index()].stored.len(), 1);
}

/// Convento da Penha's 1 pool from another Methuselah is a cost: paid with
/// their last pool, they are ousted at once (rules-digest D3).
#[test]
fn convento_da_penha_paid_with_the_last_pool_ousts() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let a = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let b = e.add_minion(SeatId(2), find_card("Rinaldo Albizzi"), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
    let c = in_play(&mut e, 0, "Convento da Penha, Vila Velha", None);
    hand(&mut e, 1, &["Kine Resources Contested"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.seats[1].pool = 1;
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, card_ability(c, 1));
    request(&mut e);
    assert!(e.state.seats[1].ousted);
    // The pool paid, and the oust's 6 to seat 1's predator.
    assert_eq!(e.state.seats[0].pool, 10 + 1 + 6);
}
