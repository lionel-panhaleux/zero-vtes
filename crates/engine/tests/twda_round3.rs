//! Legacy cards for TWDA decks, round 3 (`ADDED_BATCHES`, the thirteenth
//! batch, slice 16o): the owner's list — Abactor and the hunters, the
//! Celerity and Obfuscate combat cards, Concealed Weapon, Learjet and
//! Infernal Pursuit, Baltimore Purge, Renewed Vigor and the blood-cursed
//! Banu Haqim. Readings in rules-qa Batch 63. Windows with a single option
//! are elided, so the scripts only stop where a real decision exists.

use zvtes::actions::Outcome;
use zvtes::cards::{self, AbilitySource, Effect, Layer, Scope, Target};
use zvtes::crypt::CRYPT;
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
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

fn use_at(card: CardId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(card),
        ability,
        actor,
        target,
    }
}

/// A card already in play for `seat`, on `host` if given, with mode
/// `mode`'s layers (no play): its id.
fn in_play_mode(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>, mode: u8) -> CardId {
    let card = find_library(name);
    e.state
        .cards
        .push(CardInPlay::new(card, SeatId(seat), host, mode));
    let id = CardId(e.state.cards.len() as u32 - 1);
    for effect in cards::lookup(card).expect("definition").modes[mode as usize].effects {
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

fn in_play(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>) -> CardId {
    in_play_mode(e, seat, name, host, 0)
}

fn card_in_play(e: &Engine, name: &str) -> CardId {
    let card = find_library(name);
    CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.in_play && c.card == card)
            .expect("in play") as u32,
    )
}

/// A card in hand (index `i`) played by `actor` at `target`.
fn play_at(card: u8, mode: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target,
    }
}

fn library(e: &mut Engine, seat: u8, name: &str, n: usize) {
    for _ in 0..n {
        e.state.seats[seat as usize]
            .library
            .push(find_library(name));
    }
}

/// An untitled vampire with no definition of its own, of capacity `cap`,
/// not Lasombra (Baltimore Purge reads them).
fn of_capacity(cap: u8, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.capacity == cap
                && c.clan != "Lasombra"
                && c.title == zvtes::crypt::Title::None
                && cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a vampire of the capacity")
}

/// `m` gets `disc` at superior level (the fixture's Great Beast slot): no
/// plain vampire of the pool has [OBE] or [CHI].
fn superior(e: &mut Engine, m: MinionId, disc: &'static str) {
    let slots = &mut e.state.minions[m.index()].beast_disciplines;
    let free = slots.iter().position(Option::is_none).expect("a free slot");
    slots[free] = Some(disc);
}

fn hunt(m: MinionId) -> Choice {
    action(m, ActionKind::Hunt)
}

/// Pass every blood hunt polling decision (Tara's votes kept: it fails).
fn decline_blood_hunt(e: &mut Engine) {
    while request(e).window == WindowLabel::BloodHuntPolling {
        pick(e, Choice::Pass);
    }
}

// ---- Abactor ---------------------------------------------------------------------

// Abactor: "+1 stealth action. Hunt at +4 hunt. After successful
// resolution, a referendum is called to call a blood hunt on this vampire
// (independent of the action, just as if they had committed diablerie)."
// [LSJ 20091026] [LSJ 20100112] "The Blood Hunt referendum is still a Blood
// Hunt": the blood hunt polling opens. [LSJ 20100112] [PIB 20121028] [LSJ
// 20090722-2] [LSJ 20030618] "part of the action resolution": Heidelberg
// Castle ("Cannot be used during an action") is not offered during it, and
// Freak Drive ("after resolution") only once it is over.
#[test]
fn abactor_hunts_at_plus_four_then_calls_a_blood_hunt_inside_the_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("cel", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), find_card("Tara"), 4, false);
    let castle = in_play(&mut e, 0, "Heidelberg Castle, Germany", None);
    hand(&mut e, 0, &["Abactor", "Freak Drive"]);
    pick(&mut e, play(0, 0, a));
    let mut polled = false;
    loop {
        let req = request(&mut e);
        let castle_used = req.options.iter().any(|o| {
            matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == castle)
        });
        let freak_drive = req
            .options
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == a));
        if req.window == WindowLabel::BloodHuntPolling {
            polled = true;
            assert!(!castle_used, "no Heidelberg Castle during the blood hunt");
            assert!(!freak_drive, "Freak Drive after the blood hunt");
            assert_eq!(blood(&e, a), 1 + 1 + 4, "hunt at +4 hunt");
        }
        if freak_drive {
            assert!(polled, "the blood hunt came first");
            break;
        }
        assert!(req.window != WindowLabel::MinionPhase || !polled, "{req:?}");
        e.choose(fallback(&req));
    }
}

// [LSJ 20090411] Abactor: "'Successful resolution' means the action must
// not be blocked and the vampire must have gained blood. If the vampire
// cannot gain blood (eg. Vampiric Disease), the blood hunt is not called."
// With a disease counter, or at capacity: no blood hunt; blocked, none.
#[test]
fn abactor_calls_no_blood_hunt_without_blood_gained() {
    for case in ["disease", "full", "blocked"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 2, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with("AUS", 0), 4, false);
        match case {
            "disease" => e.state.minions[a.index()].disease = true,
            "full" => e.state.minions[a.index()].blood = e.state.capacity_of(a).unwrap(),
            _ => hand(&mut e, 1, &["Eagle's Sight"]),
        }
        hand(&mut e, 0, &["Abactor"]);
        pick(&mut e, play(0, 0, a));
        if case == "blocked" {
            pick_when(&mut e, block(z));
            pick_when(&mut e, play(0, 0, z));
        }
        loop {
            let req = request(&mut e);
            assert_ne!(req.window, WindowLabel::BloodHuntPolling, "{case}");
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            e.choose(fallback(&req));
        }
    }
}

// [LSJ 20090724] [ANK 20180129-1] Abactor: "This is not a diablerie.
// Neither Rebirth nor Carlton Van Wyk's ability can be used." The blood
// hunt fails; in its controller's discard phase, Carlton has no vampire to
// burn.
#[test]
fn abactor_is_no_diablerie_for_carlton_van_wyk() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), find_card("Tara"), 4, false);
    e.add_ally(SeatId(1), find_library("Carlton Van Wyk"));
    let carlton = card_in_play(&e, "Carlton Van Wyk");
    hand(&mut e, 0, &["Abactor"]);
    pick(&mut e, play(0, 0, a));
    decline_blood_hunt(&mut e);
    assert!(e.state.minions[a.index()].in_play);
    loop {
        let req = request(&mut e);
        assert!(
            !req.options.iter().any(|o| matches!(
                o,
                Choice::UseAbility { source: AbilitySource::Card(c), target: Target::Minion(_), .. }
                    if *c == carlton
            )),
            "{req:?}"
        );
        if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
}

// [RBK action-card-or-card-in-play] A special hunt: "A minion can only
// perform this special hunt once per turn." Unlocked after the first, the
// vampire is offered no second Abactor.
#[test]
fn abactor_is_performed_once_each_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), find_card("Tara"), 4, false);
    hand(&mut e, 0, &["Abactor", "Abactor"]);
    pick(&mut e, play(0, 0, a));
    decline_blood_hunt(&mut e);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&play(0, 0, a)), "locked");
    e.state.minions[a.index()].locked = false;
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, a)), "once each turn");
}

// [LSJ 20090606] [ANK 20190606-2] [LSJ 20060306] Week of Nightmares: "Other
// non-standard hunt actions are still permitted (e.g. Abactor). When
// performing such actions, their text apply: this card does not combine
// with them." A Ravnos, barred from hunting as normal, hunts with Abactor
// for 5 blood from the bank.
#[test]
fn abactor_is_a_hunt_week_of_nightmares_permits() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), plain("Ravnos", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), find_card("Tara"), 4, false);
    in_play(&mut e, 1, "Week of Nightmares", None);
    hand(&mut e, 0, &["Abactor"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&hunt(r)), "no hunt as normal");
    pick(&mut e, play(0, 0, r));
    expect(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
    assert_eq!(blood(&e, r), (1 + 5).min(e.state.capacity_of(r).unwrap()));
}

// [ANK 20190701] Rebirth: "Cannot be used following an Amaranth or an
// Abactor as they are not diablerie actions" — and no diablerie is
// recorded: the vampire is no diablerist for the next turns.
#[test]
fn abactor_records_no_diablerie() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), find_card("Tara"), 4, false);
    hand(&mut e, 0, &["Abactor"]);
    pick(&mut e, play(0, 0, a));
    decline_blood_hunt(&mut e);
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[a.index()];
    assert_eq!((v.diablerie_turn, v.diablerie_shield), (None, false));
}

// ---- Hunt bonuses ----------------------------------------------------------------

// [RTR 19980707] [LSJ 19980722] Aaron's Feeding Razor: "The effect is not
// optional." [RTR 20030519] [RTR 20180511] "The hunt bonus applies to
// non-standard hunt actions, blood comes from the target of the hunt." A
// hunt for 2; Abactor for 1 + 4 + 1.
#[test]
fn aarons_feeding_razor_adds_to_every_hunt() {
    for abactor in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_capacity(6, 0), 0, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), find_card("Tara"), 4, false);
        in_play(&mut e, 0, "Aaron's Feeding Razor", Some(a));
        // With no blood, it must hunt: a lone hunt is taken at once.
        if abactor {
            hand(&mut e, 0, &["Abactor"]);
            pick(&mut e, play(0, 0, a));
            expect(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
            assert_eq!(blood(&e, a), 1 + 4 + 1);
        } else {
            expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, a), 2);
        }
    }
}

// Hospital Food: "Master: unique location. Requires a ready anarch. Lock
// when an anarch announces a hunting action. If that action is successful,
// the anarch gains an additional blood." [RTR 20030519] [RTR 20180511] the
// bonus applies to non-standard hunts (Abactor: 1 + 4 + 1); [LSJ
// 20050720-2] "The action is successful even if no blood is gained": at
// capacity, the card locked, the hunt successful, nothing gained. The lock
// its controller's option (rules-qa 63.12).
#[test]
fn hospital_food_adds_a_blood_to_an_anarchs_hunt() {
    for case in ["hunt", "abactor", "full", "declined"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Vivian VI"), 0, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let food = in_play(&mut e, 0, "Hospital Food", None);
        let lock = use_at(food, 0, None, Target::None);
        if case == "full" {
            e.state.minions[a.index()].blood = e.state.capacity_of(a).unwrap();
        }
        if case == "abactor" {
            hand(&mut e, 0, &["Abactor"]);
            pick(&mut e, play(0, 0, a));
        } else if case == "full" {
            pick(&mut e, hunt(a));
        }
        // With no blood, a lone hunt is taken at once.
        let req = offering(&mut e, lock);
        if case == "declined" {
            e.choose(fallback(&req));
        } else {
            e.choose(index_of(&req, lock));
        }
        loop {
            let req = request(&mut e);
            if matches!(
                (req.seat, req.window),
                (SeatId(0), WindowLabel::MinionPhase) | (_, WindowLabel::BloodHuntPolling)
            ) {
                break;
            }
            e.choose(fallback(&req));
        }
        let cap = e.state.capacity_of(a).unwrap();
        let want = match case {
            "hunt" => 1 + 1,
            "abactor" => 1 + 4 + 1,
            "full" => cap,
            _ => 1,
        };
        assert_eq!(blood(&e, a), want, "{case}");
        assert_eq!(e.state.cards[food.index()].locked, case != "declined");
        if case == "full" {
            assert_eq!(e.last_outcome, Some(Outcome::Successful));
        }
    }
}

// ---- Gregory Winter ----------------------------------------------------------------

// Gregory Winter: "Gregory can steal 1 blood (becoming life) from a vampire
// controlled by another Methuselah as a +1 stealth Ⓓ action. He can burn a
// vampire in torpor to gain 2 life as a Ⓓ action. During your unlock
// phase, Gregory burns 1 life." [RTR 20010711] "Can use his action on a
// vampire with no blood." [LSJ 20100909] "The two actions provided are not
// cross-restrictive. Each can be performed once during the same turn by
// the same minion": his steal on an empty vampire, then, unlocked, his
// burn of a torpid one; his unlock phase costs him a life.
#[test]
fn gregory_winter_steals_burns_and_wanes() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let greg = e.add_ally(SeatId(0), find_library("Gregory Winter"));
    let id = card_in_play(&e, "Gregory Winter");
    let empty = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    let torpid = e.add_minion(SeatId(1), plain("Ventrue", 1), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
    pick(&mut e, use_at(id, 0, Some(greg), Target::Minion(empty)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, greg), blood(&e, empty)), (4, 0));
    e.state.minions[greg.index()].locked = false;
    pick(&mut e, use_at(id, 1, Some(greg), Target::Minion(torpid)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[torpid.index()].in_play);
    assert_eq!(blood(&e, greg), 6, "over his starting life");
    pick(&mut e, Choice::Pass);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, greg), 5);
}

// [ANK 20200203-2] [LSJ 20060515] "Ghouls are monsters, not mortal."
// Gregory, a ghoul, carries no Mortal label.
#[test]
fn gregory_winter_is_a_ghoul_not_a_mortal() {
    let labels = zvtes::library::LIBRARY[find_library("Gregory Winter") as usize].labels;
    assert!(!labels.contains(&zvtes::library::Label::Mortal));
    assert!(labels.contains(&zvtes::library::Label::Ghoul));
}

// ---- Baltimore Purge ---------------------------------------------------------------

// Baltimore Purge: "Requires a vampire. If this vampire is Ravnos, this is
// a +1 stealth action. Put this card on this vampire. During your unlock
// phase, burn this card and each other Methuselah chooses a ready vampire
// they control who is not older than this vampire; if you control a ready
// Lasombra, you choose the vampires instead; this vampire and the chosen
// vampires go to torpor." [LSJ 20050111] nothing between the picks and the
// torpor; [PIB 20121028] "not an action, you can choose a vampire with a
// Secure Haven"; [ANK 20210529] "If you control a ready Lasombra, the
// chosen vampires may not be older than the acting vampire."
#[test]
fn baltimore_purge_sends_its_vampire_and_the_chosen_to_torpor() {
    for lasombra in [false, true] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), of_capacity(5, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        if lasombra {
            e.add_minion(SeatId(0), plain("Lasombra", 0), 4, false);
        }
        let small = e.add_minion(SeatId(1), of_capacity(3, 0), 4, false);
        let equal = e.add_minion(SeatId(1), of_capacity(5, 1), 4, false);
        let big = e.add_minion(SeatId(1), of_capacity(6, 0), 4, false);
        let haven = e.add_minion(SeatId(2), of_capacity(4, 0), 4, false);
        let other = e.add_minion(SeatId(2), of_capacity(4, 1), 4, false);
        in_play(&mut e, 2, "Secure Haven", Some(haven));
        hand(&mut e, 0, &["Baltimore Purge"]);
        pick(&mut e, play(0, 0, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let purge = find_library("Baltimore Purge");
        assert!(e.state.cards.iter().any(|c| c.in_play && c.card == purge));
        pick(&mut e, Choice::Pass);
        // Seat 1's pick (or the Lasombra controller's): the vampires not
        // older than the purging one.
        let chooser = |s: u8| SeatId(if lasombra { 0 } else { s });
        let req = until(&mut e, chooser(1), WindowLabel::ResolutionChoice);
        assert_eq!(
            req.options,
            vec![Choice::ChooseMinion(small), Choice::ChooseMinion(equal)]
        );
        pick(&mut e, Choice::ChooseMinion(equal));
        let req = expect(&mut e, chooser(2), WindowLabel::ResolutionChoice);
        assert!(req.options.contains(&Choice::ChooseMinion(haven)));
        pick(&mut e, Choice::ChooseMinion(haven));
        request(&mut e);
        for m in [a, equal, haven] {
            assert!(e.state.minions[m.index()].in_torpor, "{m:?}");
        }
        for m in [small, big, other] {
            assert!(!e.state.minions[m.index()].in_torpor, "{m:?}");
        }
        assert!(!e.state.cards.iter().any(|c| c.in_play && c.card == purge));
    }
}

// ---- Renewed Vigor -------------------------------------------------------------------

// [PIB 20121210] Renewed Vigor: "[obe], [OBE]: Is undirected if you control
// the target, directed at the controller of the target otherwise." At four
// seats, at a torpid vampire of seat 2 (across the table), seat 2 may block;
// at one's own, seat 2 (neither prey nor predator) is offered no block.
#[test]
fn renewed_vigor_is_directed_at_another_methuselahs_vampire() {
    for own in [false, true] {
        let mut e = game(4);
        let a = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
        superior(&mut e, a, "obe");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let t = e.add_minion(SeatId(if own { 0 } else { 2 }), plain("Brujah", 1), 1, true);
        let z = e.add_minion(SeatId(2), vampire_with("aus", 0), 4, false);
        hand(&mut e, 0, &["Renewed Vigor"]);
        // An intercept card in hand: a block attempt is not provably vain.
        hand(&mut e, 2, &["Eagle's Sight"]);
        pick(&mut e, play_at(0, 1, a, Target::Minion(t)));
        let blocked = offered_before(&mut e, block(z), SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blocked, !own, "own {own}");
    }
}

// [ANK 20210124] [LSJ 20070411] [LSJ 20090514] "If the action is unblocked
// when it resolves and the target is no longer in torpor, the action
// 'fizzles' (cost, if any, is paid, and it is considered successful, but
// has no effect)": the target out of torpor before resolution, nothing
// happens to it.
// [LSJ 20011204-3] [PIB 20130704] "When played, a multi-Disciplines card
// counts as requiring the Discipline(s) being used": a [for]-only vampire
// is offered the [for] mode only.
#[test]
fn renewed_vigor_fizzles_on_a_target_out_of_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    superior(&mut e, a, "obe");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(0), plain("Brujah", 1), 1, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Renewed Vigor"]);
    pick(&mut e, play_at(0, 1, a, Target::Minion(t)));
    request(&mut e);
    e.state.minions[t.index()].in_torpor = false;
    e.state.minions[t.index()].locked = true;
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
    assert!(e.state.minions[t.index()].locked, "nothing done to it");

    let mut e = game(2);
    let f = e.add_minion(SeatId(0), vampire_with("for", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Renewed Vigor"]);
    let req = request(&mut e);
    let modes: Vec<u8> = req
        .options
        .iter()
        .filter_map(|o| match o {
            Choice::Play {
                card: 0,
                mode,
                actor: Some(m),
                ..
            } if *m == f => Some(*mode),
            _ => None,
        })
        .collect();
    let has_obe = e.state.discipline_level(f, "obe") > 0;
    assert!(modes.contains(&0));
    assert_eq!(modes.iter().any(|&m| m > 0), has_obe);
}

// Renewed Vigor [for]: "Put this card on this vampire. During your unlock
// phase, this vampire can burn this card to move from torpor to the ready
// region. A vampire can have only one Renewed Vigor." [OBE]: "Add enough
// blood to another vampire to reach full capacity."
#[test]
fn renewed_vigor_brings_its_bearer_out_of_torpor_or_fills_a_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("for", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Renewed Vigor", "Renewed Vigor"]);
    pick(&mut e, play(0, 0, a));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let vigor = card_in_play(&e, "Renewed Vigor");
    assert_eq!(e.state.cards[vigor.index()].host, Some(a));
    e.state.minions[a.index()].locked = false;
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, a)), "only one");
    e.state.minions[a.index()].in_torpor = true;
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, use_at(vigor, 0, None, Target::None));
    to_minion_phase(&mut e, SeatId(0));
    assert!(!e.state.minions[a.index()].in_torpor);
    assert!(!e.state.cards[vigor.index()].in_play);

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    superior(&mut e, a, "obe");
    let t = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Renewed Vigor"]);
    pick(&mut e, play_at(0, 4, a, Target::Minion(t)));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(Some(blood(&e, t)), e.state.capacity_of(t));
}

// ---- Illusory Resources --------------------------------------------------------------

// Illusory Resources: "[chi] Put this card in play with 1 counter. During
// your unlock phase, a ready vampire with Chimerstry you control can move 1
// blood to this card. You can burn X counters from this card to reduce a
// bleed against you by X. [CHI] As above, but this card comes into play
// with 2 counters." [CHI]: 2 counters; one burned cancels a bleed of 1.
#[test]
fn illusory_resources_banks_counters_to_reduce_a_bleed() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    superior(&mut e, c, "chi");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Illusory Resources"]);
    pick(&mut e, play(0, 1, c));
    to_minion_phase(&mut e, SeatId(0));
    let ir = card_in_play(&e, "Illusory Resources");
    assert_eq!(e.state.cards[ir.index()].counters, 2);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[z.index()].blood = 4;
    hand(&mut e, 1, &["Bonding"]);
    pick(&mut e, bleed(z));
    pick_when(&mut e, use_at(ir, 1, None, Target::None));
    pick(&mut e, Choice::SetX(1));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(pool(&e)[0], 10, "a bleed of 1, less 1");
    assert_eq!(e.state.cards[ir.index()].counters, 1);
}

// ---- Combat cards ----------------------------------------------------------------

/// `a` (seat 0) bleeds; `z` (seat 1) blocks: the combat begins.
fn blocked_bleed(e: &mut Engine, a: MinionId, z: MinionId) {
    pick(e, bleed(a));
    pick_when(e, block(z));
}

// Staredown: "[pre] Strike: dodge. [PRE] Strike: combat ends." The dodge
// keeps the hand strike off.
#[test]
fn staredown_dodges_or_ends_combat() {
    for mode in [0, 1] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("PRE", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Staredown"]);
        blocked_bleed(&mut e, a, z);
        pick_when(&mut e, play(0, mode, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, a), 4, "mode {mode}");
        assert_eq!(blood(&e, z), 4);
    }
}

// Desert Eagle: "Weapon, gun. Strike: 2R damage."
#[test]
fn desert_eagle_strikes_for_two_at_range() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let gun = in_play(&mut e, 0, "Desert Eagle", Some(a));
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, use_at(gun, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 2);
}

// Behind You!: "Only usable on the first round of combat. [obf] Maneuver.
// [OBF] Strike: dodge." Dodged in the first round; pressed into a second
// (Infernal Pursuit [cel]), not offered again.
#[test]
fn behind_you_is_only_usable_in_the_first_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("OBF", 0), 4, false);
    superior(&mut e, a, "cel");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(
        &mut e,
        0,
        &["Behind You!", "Behind You!", "Infernal Pursuit"],
    );
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(1, 0, a));
    assert_eq!(blood(&e, a), 4, "dodged");
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
    assert_eq!(blood(&e, a), 4 - 1, "the second round's hand strike");
}

// Preternatural Evasion: "[cel] Strike: dodge. [CEL] Strike: this vampire
// burns 1 blood to end combat." [ANK 20200703-2] "The blood is burned when
// playing the card"; [ANK 20210226] "The card cannot be played if the
// minion cannot afford to burn the blood", "The burn blood effect is not
// reduced by effects that reduce the cost of the card (eg. Adana de
// Sforza)."
#[test]
fn preternatural_evasion_burns_its_blood_as_it_is_played() {
    for adana in [false, true] {
        let mut e = game(2);
        let a = if adana {
            e.add_minion(SeatId(0), find_card("Adana de Sforza"), 3, false)
        } else {
            e.add_minion(SeatId(0), vampire_with("CEL", 0), 3, false)
        };
        if adana {
            superior(&mut e, a, "cel");
        }
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Preternatural Evasion"]);
        blocked_bleed(&mut e, a, z);
        pick_when(&mut e, play(0, 1, a));
        request(&mut e);
        assert_eq!(blood(&e, a), 2, "burned as it is played, adana {adana}");
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, z), 4);
    }
    // With no blood left in the combat: the dodge only.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Preternatural Evasion"]);
    blocked_bleed(&mut e, a, z);
    e.state.minions[a.index()].blood = 0;
    let req = offering(&mut e, play(0, 0, a));
    assert!(!req.options.contains(&play(0, 1, a)));
}

/// A plain vampire of seat 0 with superior Animalism and Fortitude (Alpha
/// Glint's combo), of capacity `cap`.
fn glinter(e: &mut Engine, cap: u8) -> MinionId {
    let a = e.add_minion(SeatId(0), of_capacity(cap, 0), 4, false);
    superior(e, a, "ani");
    superior(e, a, "for");
    a
}

// Alpha Glint: "Not usable in combat with an ally or an older vampire."
// Against a vampire of the same capacity, both modes; an older one or an
// ally, neither.
#[test]
fn alpha_glint_is_not_usable_against_an_older_vampire_or_an_ally() {
    for (opponent, usable) in [("same", true), ("older", false), ("ally", false)] {
        let mut e = game(2);
        let a = glinter(&mut e, 5);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = match opponent {
            "same" => e.add_minion(SeatId(1), of_capacity(5, 1), 4, false),
            "older" => e.add_minion(SeatId(1), of_capacity(6, 0), 4, false),
            _ => e.add_ally(SeatId(1), find_library("Carlton Van Wyk")),
        };
        hand(&mut e, 0, &["Alpha Glint"]);
        blocked_bleed(&mut e, a, z);
        let offered = offered_before(&mut e, play(0, 1, a), SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(offered, usable, "{opponent}");
    }
}

// [RTR 20001020] [LSJ 20001024] Alpha Glint: "Cards that are played at the
// end of round/combat can still be played (eg. Taste of Vitae, Disarm)."
// Its [ani][for] strike ends the combat (resolved before the hand strike);
// at the end of that round the blocker may still play Taste of Vitae.
#[test]
fn alpha_glint_leaves_the_end_of_round_open() {
    let mut e = game(2);
    let a = glinter(&mut e, 5);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), of_capacity(5, 1), 4, false);
    hand(&mut e, 0, &["Alpha Glint"]);
    hand(&mut e, 1, &["Taste of Vitae"]);
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    let req = offering(&mut e, play(0, 0, z));
    assert_eq!(req.window, WindowLabel::CombatEndOfRound);
}

// [LSJ 19971110] [ANK 20190725] Weather Control: "The damage is applied
// even if the combat ends prematurely (eg. by Mariel, Lady Thunder, Alpha
// Glint)." [tha] Weather Control played before range; Alpha Glint [ANI]
// [FOR] ends the combat there: both still take the 1 damage.
#[test]
fn alpha_glint_before_range_lets_weather_control_damage_land() {
    let mut e = game(2);
    let a = glinter(&mut e, 5);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("tha", 0), 4, false);
    hand(&mut e, 0, &["Alpha Glint"]);
    hand(&mut e, 1, &["Weather Control"]);
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        (blood(&e, a), blood(&e, z)),
        (4 - 1 - 1, 4 - 1),
        "the combo's 1 blood"
    );
}

// Torrent: "[cel] Additional strike (limited). [CEL] Strike: combat ends.
// If this vampire was blocked while performing an action, the action
// continues as if unblocked." [RTR 19941109] [ANK 20190116] "all action
// modifiers and reaction cards are still in effect": Conditioning's [dom]
// +2 bleed lands with the bleed of 1.
#[test]
fn torrent_continues_a_blocked_bleed_with_its_modifiers() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
    superior(&mut e, a, "dom");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    hand(&mut e, 0, &["Conditioning", "Torrent"]);
    hand(&mut e, 1, &["Eagle's Sight"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 3);
    assert_eq!(blood(&e, a), 4 - 1 - 2, "Conditioning's 1, Torrent's 2");
}

// [ANK 20190116] "any Methuselahs that have declined to block before do
// not get another opportunity to block." [LSJ 20070808-1] "continue as if
// unblocked moves the action card from the ash heap … to limbo": a
// blocked Abactor, continued, still hunts — seat 1, which declined, is
// not asked again.
#[test]
fn torrent_continues_an_action_card_without_new_blocks() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let y = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    let z = e.add_minion(SeatId(2), vampire_with("aus", 1), 4, false);
    hand(&mut e, 0, &["Abactor", "Torrent"]);
    hand(&mut e, 1, &["Eagle's Sight"]);
    hand(&mut e, 2, &["Eagle's Sight"]);
    pick(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(y)));
    e.choose(index_of(&req, Choice::Pass));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 1, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(y));
    // Torrent's 2 blood paid, then the hunt's 5.
    assert_eq!(blood(&e, a), 5.min(e.state.capacity_of(a).unwrap()));
    assert!(e.state.seats[0].ash.contains(&find_library("Abactor")));
}

// [RTR 19970630] "An action cannot continue 'as if unblocked' after a
// combat resulting from a successful action (e.g. Bum's Rush)": Torrent
// [CEL] in a rush's combat just ends it; the rush is not resolved again.
#[test]
fn torrent_does_not_continue_a_rush() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
    superior(&mut e, a, "pot");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Bum's Rush", "Torrent"]);
    pick(&mut e, play_at(0, 0, a, Target::Minion(z)));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
    assert_eq!(pool(&e), vec![10, 10]);
    assert!(e.state.minions[a.index()].locked);
}

// Renegade Garou: "Renegade Garou gets an additional strike (limited) each
// round and an optional maneuver each combat. He may enter combat with a
// minion controlled by another Methuselah as a +1 stealth Ⓓ action. If
// Renegade Garou has 2 or fewer life during your unlock phase, he gains 1
// life from the blood bank." [PIB 20121028] "The additional strike is
// mandatory": two hand strikes of 2 at the vampire he rushes; hit back
// for 2, he heals 1 at his next unlock phase.
#[test]
fn renegade_garou_strikes_twice_each_round_and_heals() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let g = e.add_ally(SeatId(0), find_library("Renegade Garou"));
    let id = card_in_play(&e, "Renegade Garou");
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick(&mut e, use_at(id, 1, Some(g), Target::Minion(z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 5 - 2 - 2);
    assert_eq!(blood(&e, g), 3 - 1, "the vampire's hand strike");
    pick(&mut e, Choice::Pass);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, g), 3);
}

// Focus the Blood: "Only usable before range is determined. A vampire can
// play only one Focus the Blood each combat. Put this card and 1 blood on
// this Assamite. This vampire can burn this card to reduce the cost of a
// combat card they play by 1 blood." Its blood in, a second copy not
// offered that combat; burned, Torrent [cel] costs 1.
#[test]
fn focus_the_blood_banks_a_blood_and_a_discount() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Banu Haqim", 0), 3, false);
    superior(&mut e, a, "cel");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(
        &mut e,
        0,
        &["Focus the Blood", "Focus the Blood", "Torrent"],
    );
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    let focus = card_in_play(&e, "Focus the Blood");
    assert_eq!(e.state.cards[focus.index()].host, Some(a));
    assert_eq!(
        e.state.layers.iter().filter(|l| l.source == focus).count(),
        1,
        "its cost-source layer, once (findings L10)"
    );
    assert_eq!(blood(&e, a), 4);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, a)), "one each combat");
    pick_when(&mut e, play(1, 0, a));
    pick_when(&mut e, Choice::ResolveRider { card: focus.0 });
    request(&mut e);
    assert_eq!(
        blood(&e, a),
        4 - 1 - 1,
        "the hand strike taken, Torrent's 1"
    );
    assert!(!e.state.cards[focus.index()].in_play);
}

// ---- Infernal Pursuit -------------------------------------------------------------

/// Seat 0's hand: `cards`, then Form of Mist up to its hand size; its
/// library: `n` Bonding.
fn stacked(e: &mut Engine, cards: &[&str], n: usize) {
    hand(e, 0, cards);
    fill_hand(e, 0);
    e.state.seats[0].library.clear();
    library(e, 0, "Bonding", n);
}

// Infernal Pursuit: "[CEL] For the remainder of the combat, each time you
// replace a card (including when you draw to replace this card), draw an
// additional card and discard down to your hand size." [TOM 19950924] "The
// card you discard need not be one of the cards that you just drew": a
// Form of Mist offered. [TOM 19950921-2] "When played multiple times, each
// time you replace a card, draw all additional cards at once then discard
// down to hand size": the second copy draws 1 + 2, two discarded.
#[test]
fn infernal_pursuit_draws_extra_cards_with_each_replacement() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    stacked(&mut e, &["Infernal Pursuit", "Infernal Pursuit"], 6);
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::DiscardDown);
    assert_eq!(e.state.seats[0].hand.len(), 8);
    let mist = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Form of Mist"))
        .unwrap() as u8;
    assert!(req.options.contains(&Choice::Discard(mist)), "any card");
    pick(&mut e, Choice::Discard(mist));
    pick_when(&mut e, play(0, 1, a));
    expect(&mut e, SeatId(0), WindowLabel::DiscardDown);
    assert_eq!(e.state.seats[0].hand.len(), 6 + 1 + 2);
    assert_eq!(e.state.seats[0].library.len(), 6 - 2 - 3);
    e.choose(0);
    e.choose(0);
    assert_eq!(e.state.seats[0].hand.len(), 7);
}

// [LSJ 20060620] "You can draw more cards than are in the library …; draw
// as much as you can": one card left, the replacement only.
#[test]
fn infernal_pursuit_draws_what_the_library_has() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    stacked(&mut e, &["Infernal Pursuit"], 1);
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    let req = request(&mut e);
    assert_ne!(req.window, WindowLabel::DiscardDown);
    assert_eq!(e.state.seats[0].hand.len(), 7);
    assert!(e.state.seats[0].library.is_empty());
}

// [LSJ 20021022] Infernal Pursuit [CEL]: "If canceled as it is played, its
// effect is not applied and only one card is drawn as a replacement." [ANK
// 20191211] "Once successfully played, if a subsequent combat card played
// by this minion gets canceled as it is played, an additional card is
// drawn for replacement." Direct Intervention cancels the first copy, or
// the second.
#[test]
fn infernal_pursuit_canceled_draws_no_extra_card_but_a_later_canceled_card_does() {
    for first in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        stacked(&mut e, &["Infernal Pursuit", "Infernal Pursuit"], 6);
        hand(&mut e, 1, &["Direct Intervention"]);
        blocked_bleed(&mut e, a, z);
        pick_when(&mut e, play(0, 1, a));
        if !first {
            let req = until(&mut e, SeatId(0), WindowLabel::DiscardDown);
            e.choose(req.options.len() - 1);
            pick_when(&mut e, play(0, 1, a));
        }
        let req = until(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
        e.choose(index_of(&req, master_play(0)));
        let req = request(&mut e);
        let library = e.state.seats[0].library.len();
        if first {
            assert_ne!(req.window, WindowLabel::DiscardDown);
            assert_eq!(library, 6 - 1, "the replacement only");
        } else {
            assert_eq!(req.window, WindowLabel::DiscardDown);
            assert_eq!(library, 6 - 2 - 2, "the canceled copy's extra card");
        }
    }
}

// [ANK 20180925-1] [LSJ 20100506] "If an effect would force you to replace
// more than 1 card (e.g. Nicomedes, Blood Weakens), replace and draw extra
// cards individually (i.e. one replacement card draw at a time), discarding
// each time X cards (where X is the number of extra cards you drew)."
// Under Blood Weakens, two Infernal Pursuits held back; the hand emptied,
// both replaced at once: each replacement draws its 2 extra cards and
// discards 2 before the next; the hand then drawn up.
#[test]
fn infernal_pursuit_replaces_blood_weakens_cards_one_at_a_time() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Blood Weakens", None);
    stacked(&mut e, &["Infernal Pursuit", "Infernal Pursuit"], 0);
    // Two kinds of card, alternating: each discard a real choice.
    for _ in 0..10 {
        library(&mut e, 0, "Bonding", 1);
        library(&mut e, 0, "Conditioning", 1);
    }
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    request(&mut e);
    assert_eq!(e.state.seats[0].library.len(), 20, "held back");
    e.state.seats[0].hand = vec![find_library("Infernal Pursuit")].into();
    pick_when(&mut e, play(0, 1, a));
    // Two copies (this one counting, "including when you draw to replace
    // this card"): the first replacement and its 2 extra cards, 2
    // discarded; then the second's.
    expect(&mut e, SeatId(0), WindowLabel::DiscardDown);
    assert_eq!(e.state.seats[0].hand.len(), 3);
    e.choose(0);
    e.choose(0);
    expect(&mut e, SeatId(0), WindowLabel::DiscardDown);
    assert_eq!(e.state.seats[0].hand.len(), 1 + 3);
    e.choose(0);
    e.choose(0);
    request(&mut e);
    assert_eq!(e.state.seats[0].hand.len(), 7, "then drawn up");
    assert_eq!(e.state.seats[0].library.len(), 20 - 3 - 3 - 5);
}

// ---- Learjet -------------------------------------------------------------------------

// Learjet: "Each time you draw to replace a card this minion played as an
// acting minion, you may draw an additional card (discard afterward)."
// [LSJ 20050224-4] "The additional replacement is optional, you get to see
// the normal replacement before you decide." [PIB 20110718] "If a card
// played by the minion with Learjet has a delayed replacement, the
// additional card is drawn when the replacement is drawn": under Blood
// Weakens, Conditioning's replacement and the offer wait for the end of
// the action.
#[test]
fn learjet_offers_an_extra_card_after_the_replacement() {
    for weakens in [false, true] {
        for take in [false, true] {
            let mut e = game(2);
            let a = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
            superior(&mut e, a, "dom");
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
            in_play(&mut e, 0, "Learjet", Some(a));
            if weakens {
                in_play(&mut e, 1, "Blood Weakens", None);
            }
            stacked(&mut e, &["Conditioning"], 4);
            pick(&mut e, bleed(a));
            pick_when(&mut e, play(0, 0, a));
            let req = until(&mut e, SeatId(0), WindowLabel::Draw);
            assert_eq!(req.options, vec![Choice::Pass, Choice::DrawLibrary]);
            assert_eq!(e.state.seats[0].hand.len(), 7, "the replacement seen");
            assert_eq!(
                pool(&e)[1],
                if weakens { 10 - 3 } else { 10 },
                "after the action, weakens {weakens}"
            );
            if take {
                pick(&mut e, Choice::DrawLibrary);
                expect(&mut e, SeatId(0), WindowLabel::DiscardDown);
                assert_eq!(e.state.seats[0].hand.len(), 8);
            } else {
                pick(&mut e, Choice::Pass);
                request(&mut e);
                assert_eq!(e.state.seats[0].hand.len(), 7);
            }
        }
    }
}

/// Seat 0's optional extra draws offered on the way, each declined, until
/// `stop` holds of a decision.
fn extra_draws_until(e: &mut Engine, stop: impl Fn(&Engine, &DecisionRequest) -> bool) -> usize {
    let mut n = 0;
    for _ in 0..500 {
        let req = request(e);
        if stop(e, &req) {
            return n;
        }
        if req.seat == SeatId(0) && req.options == [Choice::Pass, Choice::DrawLibrary] {
            n += 1;
            e.choose(index_of(&req, Choice::Pass));
        } else {
            e.choose(fallback(&req));
        }
    }
    panic!("never stopped");
}

/// Seat 0's minion phase, in its own turn.
fn minion_phase(e: &Engine, req: &DecisionRequest) -> bool {
    (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) && e.state.active == SeatId(0)
}

/// Learjet on seat 0's vampire with `disc`, a second minion keeping the
/// minion phase open, seat 1's vampire: the bearer and that vampire.
fn learjet_game(disc: &'static str) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    superior(&mut e, a, disc);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Learjet", Some(a));
    (e, a, z)
}

// The action card the bearer plays to take its action is replaced before
// the action, not during it: no extra card (owner, 2026-10-14: rules-qa
// 73.2). Govern the Unaligned's bleed, the hand drawn back to 7.
#[test]
fn learjet_offers_no_extra_card_for_the_action_card_itself() {
    let (mut e, a, _) = learjet_game("dom");
    stacked(&mut e, &["Govern the Unaligned"], 4);
    pick(&mut e, play(0, 0, a));
    assert_eq!(extra_draws_until(&mut e, minion_phase), 0);
    assert_eq!(e.state.seats[0].hand.len(), 7, "the replacement drawn");
    assert_eq!(e.state.seats[0].learjet_held.total(), 0);
}

// [PIB 20110718] "the additional card is drawn when the replacement is
// drawn", whatever the delay: Old Friends' at seat 0's next unlock phase.
#[test]
fn learjet_offers_its_extra_card_with_a_replacement_held_until_the_unlock_phase() {
    let (mut e, a, _) = learjet_game("obf");
    stacked(&mut e, &["Old Friends"], 4);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    assert_eq!(
        extra_draws_until(&mut e, minion_phase),
        0,
        "not replaced yet"
    );
    let turn = e.state.turn;
    let n = extra_draws_until(&mut e, |e, req| e.state.turn > turn && minion_phase(e, req));
    assert_eq!(
        n, 1,
        "with the replacement, as seat 0's unlock phase begins"
    );
    assert_eq!(e.state.seats[0].learjet_held.total(), 0);
}

// Mirror Walk's replacement at seat 0's next discard phase, the extra card
// with it.
#[test]
fn learjet_offers_its_extra_card_with_a_replacement_held_until_the_discard_phase() {
    let (mut e, a, z) = learjet_game("tha");
    stacked(&mut e, &["Mirror Walk"], 4);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    assert_eq!(
        extra_draws_until(&mut e, minion_phase),
        0,
        "not replaced yet"
    );
    let turn = e.state.turn;
    let n = extra_draws_until(&mut e, |e, _| e.state.turn > turn);
    assert_eq!(
        n, 1,
        "with the replacement, as seat 0's discard phase begins"
    );
    assert_eq!(e.state.seats[0].learjet_held.total(), 0);
}

// Blood Weakens: Conditioning's replacement held until the action ends or
// the hand empties; Cloak the Gathering empties it, both replaced then,
// each with its extra card.
#[test]
fn learjet_offers_the_extra_cards_of_replacements_drawn_as_the_hand_empties() {
    let (mut e, a, z) = learjet_game("dom");
    superior(&mut e, a, "obf");
    in_play(&mut e, 1, "Blood Weakens", None);
    // A hand size of 2: the two cards.
    e.state.layers.push(Layer {
        scope: Scope::InPlay,
        modifier: zvtes::cards::Modifier::HandSize(-5),
        source: card_in_play(&e, "Learjet"),
    });
    hand(&mut e, 0, &["Conditioning", "Cloak the Gathering"]);
    e.state.seats[0].library.clear();
    library(&mut e, 0, "Bonding", 10);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a)); // Conditioning, held
    request(&mut e);
    assert_eq!(e.state.seats[0].hand.len(), 1);
    assert_eq!(e.state.seats[0].learjet_held.total(), 1);
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a)); // Cloak the Gathering, the hand empty
    request(&mut e);
    assert_eq!(
        e.state.seats[0].hand.len(),
        2,
        "both replaced as the hand empties"
    );
    assert_eq!(e.state.seats[0].held.total(), 0);
    assert_eq!(extra_draws_until(&mut e, minion_phase), 2);
    assert_eq!(e.state.seats[0].learjet_held.total(), 0);
}

// The Meddling of Semsith holds the replacements Blood Weakens releases at
// the action's end until the minion phase ends: the extra card waits with
// them.
#[test]
fn learjet_extra_card_waits_with_a_replacement_held_again_by_the_meddling() {
    let (mut e, a, _) = learjet_game("dom");
    in_play(&mut e, 1, "Blood Weakens", None);
    let meddling = in_play(&mut e, 1, "Meddling of Semsith, The", None);
    e.state.cards[meddling.index()].against = zvtes::ids::SeatSet::single(SeatId(0));
    stacked(&mut e, &["Conditioning"], 4);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    assert_eq!(
        extra_draws_until(&mut e, minion_phase),
        0,
        "not replaced yet"
    );
    let turn = e.state.turn;
    let n = extra_draws_until(&mut e, |e, _| e.state.turn > turn);
    assert_eq!(n, 1, "as the minion phase ends");
    assert_eq!(e.state.seats[0].learjet_held.total(), 0);
}

// ---- Concealed Weapon --------------------------------------------------------------

fn concealed(a: MinionId) -> Choice {
    play(0, 0, a)
}

// Concealed Weapon: "Only usable before range is determined. This minion
// equips with a non-unique weapon card from your hand (requirements and
// cost apply as normal). The weapon cannot cost 3 or more pool or inflict
// (with a regular strike) aggravated damage or 4 or more damage." [RTR
// 19980623] the weapon's damage "against a generic opponent"; [LSJ
// 20020821] [LSJ 20020904] the minion's strength not counted: a Salubri
// may take Righteous Blade (strength+1), a .44 Magnum (2R, 2 pool) or a
// Desert Eagle — never an Assault Rifle (4R, 5 pool) nor a Flamethrower
// (aggravated); [LSJ 20100725] [LSJ 20100303] [ANK 20180817] requirements
// apply: not Righteous Blade for a Brujah. [LSJ 20100130] [ANK 20180925-2]
// [LSJ 19991207] named as it is played, still in hand until it resolves.
// [LSJ 20080815] not the equip action: the minion, unlocked, may still
// equip the Desert Eagle that turn.
#[test]
fn concealed_weapon_equips_a_small_weapon_from_hand() {
    for salubri in [true, false] {
        let mut e = game(2);
        let a = if salubri {
            e.add_minion(SeatId(0), plain("Salubri", 0), 4, false)
        } else {
            e.add_minion(SeatId(0), plain("Brujah", 2), 4, false)
        };
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(
            &mut e,
            0,
            &[
                "Concealed Weapon",
                ".44 Magnum",
                "Assault Rifle",
                "Flamethrower",
                "Righteous Blade",
                "Desert Eagle",
            ],
        );
        blocked_bleed(&mut e, a, z);
        pick_when(&mut e, concealed(a));
        let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
        let named = |n: &str| {
            req.options.contains(&Choice::EmployFromHand {
                card: find_library(n),
                mode: 0,
            })
        };
        assert!(named(".44 Magnum") && named("Desert Eagle"));
        assert!(!named("Assault Rifle") && !named("Flamethrower"));
        assert_eq!(named("Righteous Blade"), salubri);
        let magnum = find_library(".44 Magnum");
        pick(
            &mut e,
            Choice::EmployFromHand {
                card: magnum,
                mode: 0,
            },
        );
        assert!(
            e.state.seats[0].hand.contains(&magnum),
            "shown at resolution"
        );
        request(&mut e);
        assert!(e
            .state
            .cards
            .iter()
            .any(|c| c.card == magnum && c.host == Some(a) && c.in_play));
        assert!(!e.state.seats[0].hand.contains(&magnum));
        assert_eq!(pool(&e)[0], 10 - 2);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        e.state.minions[a.index()].locked = false;
        let eagle = e.state.seats[0]
            .hand
            .iter()
            .position(|&c| c == find_library("Desert Eagle"))
            .unwrap() as u8;
        let req = request(&mut e);
        assert!(req.options.contains(&play(eagle, 0, a)), "may still equip");
    }
}

// [LSJ 20050913] "Cannot select cards that are 'playable as if in hand'
// (e.g. from The Erciyes Fragments)": a .44 Magnum on the Fragments is no
// weapon for Concealed Weapon to equip.
#[test]
fn concealed_weapon_does_not_reach_the_erciyes_fragments() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let frag = in_play(&mut e, 0, "Erciyes Fragments, The", None);
    e.state.cards[frag.index()].stored = vec![find_library(".44 Magnum")].into();
    hand(&mut e, 0, &["Concealed Weapon"]);
    blocked_bleed(&mut e, a, z);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, concealed(a));
}

// [ANK 20230824] [RTR 20070707] "Can reduce the cost of an action card even
// if it is not played as an action (e.g. a weapon played with Concealed
// Weapon …)." "Cost reductions that are not referring to an action apply."
// Ravnos Cache's counters pay the .44 Magnum. [LSJ 20080125] "Paying from
// another source … does not reduce the cost as seen by Concealed Weapon":
// with 5 counters, the AK-47 (5 pool) is still not offered.
#[test]
fn concealed_weapon_takes_ravnos_cache_counters() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ravnos", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cache = in_play(&mut e, 0, "Ravnos Cache", None);
    e.state.cards[cache.index()].counters = 5;
    hand(&mut e, 0, &["Concealed Weapon", ".44 Magnum", "AK-47"]);
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, concealed(a));
    // The Magnum its only weapon: named at once; the Cache then.
    let req = request(&mut e);
    let ak = Choice::EmployFromHand {
        card: find_library("AK-47"),
        mode: 0,
    };
    assert!(!req.options.contains(&ak), "{req:?}");
    pick(&mut e, Choice::ResolveRider { card: cache.0 });
    let req = request(&mut e);
    if req.options.contains(&Choice::SetX(2)) {
        pick(&mut e, Choice::SetX(2));
        request(&mut e);
    }
    assert_eq!(pool(&e)[0], 10, "the counters paid");
    assert_eq!(e.state.cards[cache.index()].counters, 5 - 2);
    assert!(e.state.seats[0].hand.contains(&find_library("AK-47")));
}

// ---- The vampires ---------------------------------------------------------------------

// Wise Frog: "Wise Frog strikes with first strike in combat with a younger
// vampire. Blood cursed." Her hand strike sends a younger blocker with no
// blood to torpor before it strikes; an older one hits her back.
#[test]
fn wise_frog_strikes_first_against_a_younger_vampire() {
    for younger in [true, false] {
        let mut e = game(2);
        let frog = e.add_minion(SeatId(0), find_card("Wise Frog"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let cap = if younger { 3 } else { 6 };
        let z = e.add_minion(SeatId(1), of_capacity(cap, 0), 0, false);
        blocked_bleed(&mut e, frog, z);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[z.index()].in_torpor);
        assert_eq!(
            blood(&e, frog),
            if younger { 4 } else { 3 },
            "younger {younger}"
        );
    }
}

// Lisette Vizquel: "Lisette gets +1 intercept during diablerie actions."
#[test]
fn lisette_vizquel_intercepts_diablerie_actions() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lisette Vizquel"), 4, false);
    let ctx = |kind| zvtes::state::PrintedCtx {
        kind: Some(kind),
        ..Default::default()
    };
    let intercept = |e: &Engine, kind| {
        e.state
            .printed_mod(l, zvtes::cards::Stat::Intercept, ctx(kind))
    };
    assert_eq!(intercept(&e, ActionKind::Diablerie), 1);
    assert_eq!(intercept(&e, ActionKind::Bleed), 0);
}

// "Blood cursed." (Alu, Wise Frog; rulebook "Blood Cursed"): it cannot
// commit diablerie — no diablerie action at a vampire in torpor, where a
// plain vampire has one.
#[test]
fn blood_cursed_vampires_cannot_diablerize() {
    for name in ["Alu", "Wise Frog", ""] {
        let mut e = game(2);
        let a = if name.is_empty() {
            e.add_minion(SeatId(0), plain("Banu Haqim", 0), 4, false)
        } else {
            e.add_minion(SeatId(0), find_card(name), 4, false)
        };
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, true);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let req = request(&mut e);
        let diablerie = Choice::TakeAction {
            minion: a,
            action: ActionKind::Diablerie,
            target: Some(t),
        };
        assert_eq!(req.options.contains(&diablerie), name.is_empty(), "{name}");
    }
}

// ---- Confusion of the Eye ----------------------------------------------------------

// Confusion of the Eye: "[obf] Reduce a younger vampire's or an ally's
// bleed against you by 1." Against a younger vampire's bleed of 1: none
// lands; an older one's is not reduced.
#[test]
fn confusion_of_the_eye_reduces_a_younger_vampires_bleed() {
    for younger in [true, false] {
        let mut e = game(2);
        let cap = if younger { 3 } else { 6 };
        let a = e.add_minion(SeatId(0), of_capacity(cap, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), of_capacity(5, 0), 4, false);
        superior(&mut e, z, "obf");
        hand(&mut e, 1, &["Confusion of the Eye"]);
        pick(&mut e, bleed(a));
        if younger {
            let req = offering(&mut e, play(0, 0, z));
            e.choose(index_of(&req, play(0, 0, z)));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(pool(&e)[1], 10);
        } else {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, z));
        }
    }
}

// Confusion of the Eye [OBF]: "Only usable during a referendum before any
// votes and ballots are cast. … The acting vampire cannot cast any votes
// or ballots in this referendum. If the referendum requires a titled
// vampire, the referendum fails." Concordia's War of Ages ("Requires a
// titled Sabbat vampire"): she casts no vote, and with the Edge in favor
// the referendum still fails.
#[test]
fn confusion_of_the_eye_fails_a_titled_referendum() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Concordia"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), of_capacity(5, 0), 4, false);
    superior(&mut e, z, "obf");
    e.state.seats[0].has_edge = true;
    hand(&mut e, 0, &["War of Ages"]);
    hand(&mut e, 1, &["Confusion of the Eye"]);
    pick(&mut e, play(0, 0, c));
    pick_when(&mut e, play(0, 1, z));
    loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&votes(c, true)), "{req:?}");
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        let edge = Choice::CastEdge { favor: true };
        if req.options.contains(&edge) {
            pick(&mut e, edge);
        } else {
            e.choose(fallback(&req));
        }
    }
    let war = find_library("War of Ages");
    assert!(!e.state.cards.iter().any(|x| x.in_play && x.card == war));
}

// Owner, 2026-10-11 (rules-qa 64.12): as a political referendum's polling
// opens, the other Methuselahs may play the cards usable "before any votes"
// before the caller casts. Seat 0 calls Kine Resources Contested; seat 2,
// across the table, is offered Confusion of the Eye [OBF] first — then the
// caller's vampire casts nothing.
#[test]
fn confusion_of_the_eye_comes_before_the_callers_votes() {
    let mut e = game(4);
    let c = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let z = e.add_minion(SeatId(2), of_capacity(5, 0), 4, false);
    superior(&mut e, z, "obf");
    e.add_minion(SeatId(3), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    hand(&mut e, 2, &["Confusion of the Eye"]);
    pick(&mut e, play(0, 0, c));
    let req = loop {
        let req = request(&mut e);
        assert!(
            !req.options.contains(&votes(c, true)),
            "the caller first: {req:?}"
        );
        if req.options.contains(&play(0, 1, z)) {
            break req;
        }
        e.choose(fallback(&req));
    };
    assert_eq!(
        (req.seat, req.window),
        (SeatId(2), WindowLabel::ReferendumPolling)
    );
    e.choose(index_of(&req, play(0, 1, z)));
    loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&votes(c, true)), "{req:?}");
        if req.window != WindowLabel::ReferendumPolling {
            break;
        }
        e.choose(fallback(&req));
    }
}

// [PIB 20150105] [LSJ 19980107] "Cannot be used during a referendum that is
// automatically passing": after Día de los Muertos, the first Sabbat
// referendum passes with no polling step — Confusion of the Eye is never
// offered.
#[test]
fn confusion_of_the_eye_is_not_offered_on_an_automatic_pass() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Concordia"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), of_capacity(5, 0), 4, false);
    superior(&mut e, z, "obf");
    hand(&mut e, 0, &["War of Ages", "Día de los Muertos"]);
    hand(&mut e, 1, &["Confusion of the Eye"]);
    e.at_master_phase();
    pick_when(&mut e, master_play(1));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, c));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, z));
    let war = find_library("War of Ages");
    assert!(e.state.cards.iter().any(|x| x.in_play && x.card == war));
}

// ---- Abombwe -----------------------------------------------------------------------------

// Abombwe: "Put this card on a Laibon or on a vampire with Protean [pro]."
// [LSJ 20051116-2] "The requirements are ignored if the card is not played
// in the normal fashion": Childe of the Revolution's search puts it on the
// new childe (no Protean); played, only on a vampire with [pro].
#[test]
fn abombwe_needs_protean_only_when_played() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Atiena"), 5, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), vampire_with("pro", 0), 4, false);
    hand(&mut e, 0, &["Abombwe", "Childe of the Revolution"]);
    e.at_master_phase();
    let req = request(&mut e);
    let targets: Vec<Target> = req
        .options
        .iter()
        .filter_map(|o| match o {
            Choice::Play {
                card: 0, target, ..
            } => Some(*target),
            _ => None,
        })
        .collect();
    assert!(targets.contains(&Target::Minion(p)));
    assert!(!targets.contains(&Target::Minion(other)));
    pick(&mut e, Choice::Pass);
    let before = e.state.minions.len();
    pick_when(&mut e, play(1, 0, b));
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    let abombwe = Choice::TakeFrom {
        zone: zvtes::decision::Zone::Hand,
        card: find_library("Abombwe"),
    };
    assert!(req.options.contains(&abombwe));
    e.choose(index_of(&req, abombwe));
    request(&mut e);
    let childe = MinionId(before as u32);
    assert_eq!(e.state.discipline_level(childe, "abo"), 1);
    assert_eq!(e.state.capacity_of(childe), Some(2));
}

// [RTR 20020501] [LSJ 19980109] "If the combat continues or a new combat
// begins, the after combat effect does not happen (e.g. … Telepathic
// Tracking)": Torrent [CEL] ends the round; the blocker's Telepathic
// Tracking [AUS] continues the combat; the action stays blocked.
#[test]
fn torrent_is_lost_if_the_combat_continues() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("AUS", 0), 4, false);
    hand(&mut e, 0, &["Torrent"]);
    hand(&mut e, 1, &["Telepathic Tracking"]);
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatEndOfRound);
    e.choose(index_of(&req, play(0, 1, z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
    assert_eq!(pool(&e)[1], 10, "no bleed");
}

// [ANK 20180925-1] Infernal Pursuit [CEL]: "Effects that make you discard
// (eg. Nicomedes) are subject to Infernal Pursuit", one replacement at a
// time: the blocker's two combat cards discarded to cancel Target Vitals
// are replaced each with its extra card, one discarded each time (the
// first discard at hand size, not above it).
#[test]
fn infernal_pursuit_extras_follow_a_forced_discard() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("CEL", 0), 4, false);
    hand(&mut e, 0, &["Target Vitals"]);
    hand(
        &mut e,
        1,
        &["Infernal Pursuit", "Side Strike", "Glancing Blow"],
    );
    fill_hand(&mut e, 1);
    e.state.seats[1].library.clear();
    for _ in 0..10 {
        library(&mut e, 1, "Bonding", 1);
        library(&mut e, 1, "Conditioning", 1);
    }
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, play(0, 1, z));
    // Its own replacement and extra card: one discarded.
    let req = until(&mut e, SeatId(1), WindowLabel::DiscardDown);
    e.choose(req.options.len() - 1);
    // Its hand strike chosen at once (the only strike), the aim on top.
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::PayToCancel);
    let before = e.state.seats[1].library.len();
    let mut discards = 0;
    loop {
        let req = request(&mut e);
        match (req.seat, req.window) {
            (SeatId(1), WindowLabel::DiscardDown)
                if before - e.state.seats[1].library.len() > 0 =>
            {
                discards += 1;
                assert_eq!(
                    e.state.seats[1].hand.len(),
                    if discards == 1 { 7 } else { 8 },
                    "one replacement and its extra card at a time"
                );
                e.choose(req.options.len() - 1);
            }
            (SeatId(1), WindowLabel::DiscardDown | WindowLabel::ResolutionChoice) => {
                e.choose(0);
            }
            (SeatId(0), WindowLabel::MinionPhase) => break,
            _ => e.choose(fallback(&req)),
        }
        if discards == 2 {
            break;
        }
    }
    assert_eq!(before - e.state.seats[1].library.len(), 4);
}

// Hospital Food on another Methuselah's anarch (owner, 2026-10-11:
// rules-qa 63.12): seat 1's anarch hunts; seat 0, its controller, may lock
// it — the anarch gains the extra blood.
#[test]
fn hospital_food_feeds_another_methuselahs_anarch() {
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("Vivian VI"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let a = e.add_minion(SeatId(1), find_card("Atiena"), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let food = in_play(&mut e, 0, "Hospital Food", None);
    let lock = use_at(food, 0, None, Target::None);
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.choose(index_of(&req, hunt(a)));
    let req = offering(&mut e, lock);
    assert_eq!(req.seat, SeatId(0), "its controller's option");
    e.choose(index_of(&req, lock));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 1 + 1 + 1);
    assert!(e.state.cards[food.index()].locked);
}

// ---- Owner's list, round 4 (slice 16p) -------------------------------------------

// Ariadne: "Blood hunts cannot be called on Ariadne. +1 hunt." [RTR
// 20030519] [RTR 20180511] "The hunt bonus applies to non-standard hunt
// actions, blood comes from the target of the hunt": a hunt for 2; an
// Abactor for 1 + 4 + 1, and no blood hunt called on her (Tara's votes
// there to cast).
#[test]
fn ariadne_hunts_for_one_more_and_escapes_blood_hunts() {
    for abactor in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Ariadne"), 0, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), find_card("Tara"), 4, false);
        if abactor {
            hand(&mut e, 0, &["Abactor"]);
            pick(&mut e, play(0, 0, a));
        }
        // With no blood, a lone hunt is taken at once.
        loop {
            let req = request(&mut e);
            assert_ne!(req.window, WindowLabel::BloodHuntPolling);
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            e.choose(fallback(&req));
        }
        assert_eq!(blood(&e, a), if abactor { 1 + 4 + 1 } else { 1 + 1 });
        assert!(e.state.minions[a.index()].in_play);
    }
}

// Jacob Fermor: "While Jacob is ready, werewolves you control get +1
// bleed. Jacob gets an optional press each combat." Renegade Garou (0
// bleed) bleeds for 1 while Jacob is ready, none with Jacob in torpor;
// blocked, Jacob is offered his press.
#[test]
fn jacob_fermor_gives_werewolves_bleed_and_presses() {
    for torpid in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), find_card("Jacob Fermor"), 4, torpid);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let g = e.add_ally(SeatId(0), find_library("Renegade Garou"));
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        pick(&mut e, bleed(g));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            pool(&e)[1],
            if torpid { 10 } else { 10 - 1 },
            "torpid {torpid}"
        );
    }
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jacob Fermor"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    blocked_bleed(&mut e, j, z);
    offering(&mut e, Choice::UsePress);
}
