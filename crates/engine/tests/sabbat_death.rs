//! Sabbat V5, the Path of Death and the Soul: the deck's new cards
//! (`ADDED_BATCHES`, a seventh batch). Readings in rules-qa Batch 55.
//! Windows with a single option are elided, so the scripts only stop where
//! a real decision exists.

use zvtes::cards::{self, AbilitySource, Effect, Layer, Scope, Stat, Target};
use zvtes::crypt::{Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::decks::Deck;
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, CardId, CardInPlay, PrintedCtx};

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

fn vampire_ability(m: MinionId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: None,
        target: Target::None,
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

fn card(name: &str) -> u16 {
    find_card(name)
}

/// The first crypt card meeting `f`.
fn vampire(f: impl Fn(&zvtes::crypt::CryptCard) -> bool) -> u16 {
    CRYPT.iter().position(f).expect("a vampire") as u16
}

fn with(disc: &str, caps: std::ops::RangeInclusive<u8>) -> u16 {
    vampire(|c| {
        caps.contains(&c.capacity) && c.title == Title::None && c.disciplines.contains(&disc)
    })
}

/// The ally card `name`'s in-play card id for minion `m`.
fn body_card(e: &Engine, m: MinionId) -> CardId {
    CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.body == Some(m))
            .expect("the ally's card") as u32,
    )
}

const ABDERRAHIM: &str = "Abderrahim, Death's Hand";
const ASHUR: &str = "Ashur-uballit, Son of the Abyss";
const EULOGIO: &str = "Eulogio Sánchez de los Reyes";
const HULDU: &str = "Huldu, The Desecrator";
const SAKURA: &str = "Sakura, The Merciless";
const ZBIGNIEW: &str = "Zbigniew, Hunter of Mekhet";
const AITANA: &str = "Aitana, The Unholy";

/// Seat 0: `name` (4 blood, capacity-capped) and a spare; seat 1 a blocker.
fn table(name: &str) -> (Engine, MinionId, MinionId) {
    let mut e = game(3);
    let c = card(name);
    let m = e.add_minion(SeatId(0), c, 4.min(CRYPT[c as usize].capacity), false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    (e, m, z)
}

fn bleed_blocked(e: &mut Engine, a: MinionId, b: MinionId) {
    pick(e, bleed(a));
    pick(e, block(b));
}

// ---- The deck ----------------------------------------------------------------------

// The Path of Death and the Soul precon: 12 crypt and 77 library cards,
// every card defined but the vampires printing no text; the fourth Sabbat
// deck.
#[test]
fn the_path_of_death_and_the_soul_deck_is_complete() {
    let deck = Deck::sabbat(3);
    assert_eq!(deck.name, "Path of Death (Sabbat V5)");
    assert_eq!(deck.crypt_cards().len(), 12);
    assert_eq!(deck.library_cards().len(), 77);
    assert_eq!(deck.path(), Some("Death and the Soul"));
    for c in deck.library_cards() {
        assert!(cards::lookup(c).is_some(), "{c}");
    }
}

// ---- Vampires --------------------------------------------------------------------------

// [LSJ 20011022] Abderrahim "The ability can be used when in torpor": he
// burns 1 blood to give his zombie ally +1 stealth against a block.
#[test]
fn abderrahim_gives_an_ally_stealth_even_from_torpor() {
    let mut e = game(3);
    let abderrahim = e.add_minion(SeatId(0), card(ABDERRAHIM), 4, true);
    let drone = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    pick(&mut e, bleed(drone));
    pick_when(&mut e, block(z));
    pick_when(&mut e, vampire_ability(abderrahim, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, abderrahim), 3);
    assert_eq!(e.state.seats[1].pool, 9, "unblocked");
}

// Anxo Vilela: +1 strength while his Methuselah has the Edge.
#[test]
fn anxo_is_strong_with_the_edge() {
    let mut e = game(3);
    let anxo = e.add_minion(SeatId(0), card("Anxo Vilela"), 4, false);
    let d = PrintedCtx::default();
    assert_eq!(e.state.printed_mod(anxo, Stat::Strength, d), 0);
    e.state.seats[0].has_edge = true;
    assert_eq!(e.state.printed_mod(anxo, Stat::Strength, d), 1);
}

// Ashur-uballit: while ready, zombies recruited get +1 starting life (a
// Gravebound Drone with 3); +1 bleed.
#[test]
fn ashur_uballit_raises_tougher_zombies() {
    let (mut e, ashur, _) = table(ASHUR);
    let zbig = e.add_minion(SeatId(0), card(ZBIGNIEW), 4, false);
    hand(&mut e, 0, &["Gravebound Drone"]);
    pick(&mut e, play(0, 0, zbig));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let drone = e
        .state
        .minions
        .iter()
        .position(|m| m.is_ally() && m.in_play)
        .map(|i| MinionId(i as u32))
        .expect("the drone");
    assert_eq!(e.state.minions[drone.index()].blood, 3);
    assert_eq!(
        e.state
            .printed_mod(ashur, Stat::Bleed, PrintedCtx::default()),
        1
    );
}

// Gravebound Drone [OBL]: "during a bleed action, this ally can burn 1
// life from another copy of this ally you control to get +1 bleed" — the
// acting Drone, once in the bleed (the "during" template, rules-qa 6.6),
// however many other copies there are (owner, 2026-10-09, from play: the
// acting Drone burned two others for a bleed of 3).
#[test]
fn gravebound_drone_burns_another_copy_once_per_bleed() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let drone = find_library("Gravebound Drone");
    let a = e.add_ally_version(SeatId(0), drone, 1);
    let b = e.add_ally_version(SeatId(0), drone, 1);
    let c = e.add_ally_version(SeatId(0), drone, 1);
    let id = e.state.minions[a.index()].ally_id().unwrap();
    pick(&mut e, bleed(a));
    pick_when(&mut e, use_at(id, 1, None, Target::Minion(b)));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_at(id, 1, None, Target::Minion(c)),
    );
    assert_eq!(e.state.seats[1].pool, 10 - 2, "1 bleed +1, once");
    assert_eq!(blood(&e, b), 1);
    assert_eq!(blood(&e, c), 2);
}

// Eulogio: "look at and reorder the top 5 cards of your library, and
// unlock" as a +1 stealth action costing 1 blood (rules-qa 55.3).
#[test]
fn eulogio_reorders_the_top_of_his_library() {
    let (mut e, eulogio, _) = table(EULOGIO);
    let names = [
        "Glancing Blow",
        "Morbidity",
        "Obedience",
        "Heartrender",
        "Abbot",
    ];
    e.state.seats[0].library = names.iter().map(|n| find_library(n)).collect();
    fill_hand(&mut e, 0);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(eulogio),
            ability: 0,
            actor: Some(eulogio),
            target: Target::None,
        },
    );
    // From the top down: Glancing Blow first.
    pick_when(&mut e, Choice::FindCard(find_library("Glancing Blow")));
    pick_when(&mut e, Choice::FindCard(find_library("Obedience")));
    pick_when(&mut e, Choice::FindCard(find_library("Abbot")));
    pick_when(&mut e, Choice::FindCard(find_library("Morbidity")));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let lib = &e.state.seats[0].library;
    let order: Vec<u16> = lib.iter().rev().copied().collect();
    let want: Vec<u16> = [
        "Glancing Blow",
        "Obedience",
        "Abbot",
        "Morbidity",
        "Heartrender",
    ]
    .iter()
    .map(|n| find_library(n))
    .collect();
    assert_eq!(order, want);
    assert!(!e.state.minions[eulogio.index()].locked);
    assert_eq!(blood(&e, eulogio), 3);

    // The cost is paid at resolution, as for every action: blocked, not
    // paid (owner, 2026-10-08: rules-qa 55.3).
    let (mut e, eulogio, _) = table(EULOGIO);
    // An [aus] blocker, Eagle's Sight's +1 intercept against the stealth.
    let z = e.add_minion(SeatId(2), with("aus", 4..=6), 4, false);
    hand(&mut e, 2, &["Eagle's Sight"]);
    e.state.seats[0].library = names.iter().map(|n| find_library(n)).collect();
    fill_hand(&mut e, 0);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(eulogio),
            ability: 0,
            actor: Some(eulogio),
            target: Target::None,
        },
    );
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    let fought = 4 - blood(&e, z);
    assert_eq!(blood(&e, eulogio), 4 - fought, "only the combat's damage");
}

// Frau Schädel: +1 stealth during recruit and employ actions.
#[test]
fn frau_schadel_hires_stealthily() {
    let (e, frau, _) = table("Frau Schädel");
    let on = |kind| PrintedCtx {
        kind: Some(kind),
        ..Default::default()
    };
    assert_eq!(
        e.state
            .printed_mod(frau, Stat::Stealth, on(ActionKind::Recruit)),
        1
    );
    assert_eq!(
        e.state
            .printed_mod(frau, Stat::Stealth, on(ActionKind::Employ)),
        1
    );
    assert_eq!(
        e.state
            .printed_mod(frau, Stat::Stealth, on(ActionKind::Bleed)),
        0
    );
}

// Huldu: once each combat involving another minion of hers, ready, she can
// burn 1 blood to prevent 1 damage to it.
#[test]
fn huldu_shields_another_minion_in_combat() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let huldu = e.add_minion(SeatId(0), card(HULDU), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, vampire_ability(huldu, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4, "the hand strike prevented");
    assert_eq!(blood(&e, huldu), 3);
}

// Huldu: "another minion" of hers in combat — not her own combat.
#[test]
fn huldu_does_not_shield_herself() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let huldu = e.add_minion(SeatId(0), card(HULDU), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    bleed_blocked(&mut e, huldu, z);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        vampire_ability(huldu, 0),
    );
    assert!(blood(&e, huldu) < 4, "the hand strike landed");
}

// Sakura: on her turn, after a successful action requiring the Path of
// Death and the Soul (Putrescent Sustenance), 1 blood to unlock.
#[test]
fn sakura_unlocks_after_a_path_action() {
    let (mut e, sakura, _) = table(SAKURA);
    e.state.seats[1].ash.push(find_library("Heartrender"));
    hand(&mut e, 0, &["Putrescent Sustenance"]);
    pick(&mut e, play(0, 0, sakura));
    pick_when(&mut e, vampire_ability(sakura, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[sakura.index()].locked);
    assert_eq!(blood(&e, sakura), 4 + 2 - 1);
}

// ---- Library cards -----------------------------------------------------------------------

// Burial Site Hunting Ground: in the unlock phase, a ready Path of Death
// vampire gains 1 blood — or a wraith or zombie ally 1 life, not above its
// starting life.
#[test]
fn burial_site_feeds_the_path_or_mends_the_dead() {
    let mut e = game(3);
    let zbig = e.add_minion(SeatId(0), card(ZBIGNIEW), 2, false);
    let drone = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    e.state.minions[drone.index()].blood = 1;
    let site = in_play(&mut e, 0, "Burial Site Hunting Ground", None);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 2), 4, false);
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(req
        .options
        .contains(&use_at(site, 0, None, Target::Minion(zbig))));
    pick(&mut e, use_at(site, 1, None, Target::Minion(drone)));
    request(&mut e);
    assert_eq!(e.state.minions[drone.index()].blood, 2, "its starting life");
}

// Cursed Abattoir: a counter after a zombie of its controller's burns —
// "you can", their choice (owner, 2026-10-08: rules-qa 55.6); one burned
// during a zombie ally's bleed for +1 bleed.
#[test]
fn cursed_abattoir_counts_the_fallen_zombies() {
    let mut e = game(3);
    e.add_minion(SeatId(0), card(ZBIGNIEW), 4, false);
    let drone = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    let other = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    let abattoir = in_play(&mut e, 0, "Cursed Abattoir", None);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    // The other drone, at 1 life, burned in combat.
    e.state.minions[other.index()].blood = 1;
    let mut passed = e.clone();
    bleed_blocked(&mut e, other, z);
    pick_when(&mut e, Choice::ResolveRider { card: abattoir.0 });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[other.index()].in_play);
    assert_eq!(e.state.cards[abattoir.index()].counters, 1);
    // Declined: none.
    bleed_blocked(&mut passed, other, z);
    let req = until(&mut passed, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::ResolveRider { card: abattoir.0 }]
    );
    until(&mut passed, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(passed.state.cards[abattoir.index()].counters, 0);
    pick(&mut e, bleed(drone));
    pick_when(&mut e, use_at(abattoir, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
    assert_eq!(e.state.cards[abattoir.index()].counters, 0);
}

// Dance of the Dead: once each combat, a counter for a maneuver or press
// to a wraith or zombie ally; burned with its last counter.
#[test]
fn dance_of_the_dead_moves_the_dead_once_a_combat() {
    for counters in [2, 1] {
        let mut e = game(3);
        e.add_minion(SeatId(0), card(ZBIGNIEW), 4, false);
        let drone = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
        let dance = in_play(&mut e, 0, "Dance of the Dead", None);
        e.state.cards[dance.index()].counters = counters;
        let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        bleed_blocked(&mut e, drone, z);
        pick_when(&mut e, use_at(dance, 0, None, Target::None));
        if counters == 1 {
            // Its last counter: the card burns.
            assert!(!e.state.cards[dance.index()].in_play);
            continue;
        }
        assert_eq!(e.state.cards[dance.index()].counters, 1);
        // Once each combat: neither ability again in this one.
        let req = request(&mut e);
        assert_eq!(req.window, WindowLabel::CombatDetermineRange);
        for ability in 0..2 {
            assert!(!req
                .options
                .contains(&use_at(dance, ability, None, Target::None)));
        }
    }
}

// Fiorella: locks to give another wraith or zombie ally +1 stealth; [ANK
// 20180517] "They can lock to use their ability the turn they where
// recruited."
#[test]
fn fiorella_lends_stealth_to_the_dead_the_turn_she_arrives() {
    let mut e = game(3);
    e.add_minion(SeatId(0), card(ZBIGNIEW), 4, false);
    let drone = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    let fiorella = e.add_ally(SeatId(0), find_library("Fiorella, Empty One"));
    e.state.minions[fiorella.index()].recruited = true;
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let lend = use_at(body_card(&e, fiorella), 0, None, Target::None);
    pick(&mut e, bleed(drone));
    pick_when(&mut e, block(z));
    pick_when(&mut e, lend);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[fiorella.index()].locked);
    assert_eq!(e.state.seats[1].pool, 9, "unblocked");
}

// Freakish Conglomeration: enters combat as a Ⓓ action; burns 1 life in
// its controller's unlock phase.
#[test]
fn freakish_conglomeration_rushes_and_rots() {
    let mut e = game(3);
    e.add_minion(SeatId(0), card(ZBIGNIEW), 4, false);
    let freak = e.add_ally(SeatId(0), find_library("Freakish Conglomeration"));
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let rush = Choice::UseAbility {
        source: AbilitySource::Card(body_card(&e, freak)),
        ability: 0,
        actor: Some(freak),
        target: Target::Minion(z),
    };
    pick(&mut e, rush);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 3, "3 strength");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.minions[freak.index()].blood,
        3 - 1 - 1,
        "the hand strike, its unlock phase"
    );
}

// Gifts From Hereafter [DOM] or [OBL]: another vampire gives the acting
// zombie ally +1 bleed, then burns 1 blood to unlock it (rules-qa 55.5).
#[test]
fn gifts_from_hereafter_bleed_and_unlock_a_zombie() {
    let mut e = game(3);
    let zbig = e.add_minion(SeatId(0), card(ZBIGNIEW), 4, false);
    let el = e.add_minion(SeatId(0), card("El Monseñor"), 4, false);
    let drone = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let _ = zbig;
    hand(&mut e, 0, &["Gifts From Hereafter"]);
    pick(&mut e, bleed(drone));
    pick_when(&mut e, play(0, 1, el));
    pick_when(&mut e, Choice::UnlockForBlood { minion: drone });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 8);
    assert!(!e.state.minions[drone.index()].locked);
    assert_eq!(blood(&e, el), 3);
}

// Glancing Blow: prevents 1 damage from the opposing strike. [LSJ 20001114]
// "Cannot be used if there is no damage to prevent": not against a dodge.
#[test]
fn glancing_blow_prevents_a_strikes_damage() {
    let (mut e, aitana, z) = table(AITANA);
    hand(&mut e, 0, &["Glancing Blow"]);
    bleed_blocked(&mut e, aitana, z);
    pick_when(&mut e, play(0, 0, aitana));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, aitana), 4);

    let (mut e, aitana, _) = table(AITANA);
    let z = e.add_minion(SeatId(1), with("cel", 4..=11), 4, false);
    hand(&mut e, 0, &["Glancing Blow"]);
    hand(&mut e, 1, &["Side Strike"]);
    bleed_blocked(&mut e, aitana, z);
    pick_when(&mut e, play(0, 0, z));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play(0, 0, aitana),
    );
}

// Gravebound Drone: no equipment; [obl] 1 life to prevent 1 damage to
// another copy in combat; [OBL] during its bleed, 1 life of another copy
// for +1 bleed.
#[test]
fn gravebound_drones_shield_and_feed_each_other() {
    let mut e = game(3);
    e.add_minion(SeatId(0), card(ZBIGNIEW), 4, false);
    let a = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    let b = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    assert!(e.state.no_equipment(a));
    let cb = body_card(&e, b);
    bleed_blocked(&mut e, a, z);
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(cb),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[a.index()].blood, 2, "prevented");
    assert_eq!(e.state.minions[b.index()].blood, 1);

    // [OBL]: the superior version burns the other's life for the bleed.
    let mut e = game(3);
    e.add_minion(SeatId(0), card(ZBIGNIEW), 4, false);
    let a = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    let b = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let ca = body_card(&e, a);
    e.state.cards[ca.index()].mode = 1;
    pick(&mut e, bleed(a));
    pick_when(&mut e, use_at(ca, 1, None, Target::Minion(b)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 8);
    assert_eq!(e.state.minions[b.index()].blood, 1);
}

// Heartrender: removes itself from the game to burn a non-wraith non-zombie
// ally as a +1 stealth Ⓓ action (rules-qa 55.4).
#[test]
fn heartrender_trades_itself_for_an_ally() {
    let mut e = game(3);
    e.add_minion(SeatId(0), card(ZBIGNIEW), 4, false);
    let heart = e.add_ally(SeatId(0), find_library("Heartrender"));
    let deuce = e.add_ally(SeatId(1), find_library("Double Deuce"));
    let strike = Choice::UseAbility {
        source: AbilitySource::Card(body_card(&e, heart)),
        ability: 0,
        actor: Some(heart),
        target: Target::Minion(deuce),
    };
    pick(&mut e, strike);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[deuce.index()].in_play);
    assert!(!e.state.minions[heart.index()].in_play);
    assert!(e.state.seats[0]
        .removed
        .contains(&find_library("Heartrender")));
}

// Morbidity [obl] or [tha]: up to 2 of the opposing vampire's blood on the
// card in play, back to it as the combat ends and the card burns (rules-qa
// 55.2).
#[test]
fn morbidity_holds_blood_for_the_combat() {
    let (mut e, aitana, z) = table(AITANA);
    hand(&mut e, 0, &["Morbidity"]);
    hand(&mut e, 1, &["Immortal Grapple"]);
    bleed_blocked(&mut e, aitana, z);
    pick_when(&mut e, play(0, 0, aitana));
    // The blocker's next decision, in the combat.
    offering(&mut e, play(0, 0, z));
    let morbidity = find_library("Morbidity");
    let held: Vec<u8> = e
        .state
        .cards
        .iter()
        .filter(|c| c.in_play && c.card == morbidity)
        .map(|c| c.counters)
        .collect();
    assert_eq!(held, vec![2]);
    // The card itself in play, not the ash heap (rules-qa 21.1).
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == morbidity && !c.effect_only));
    assert!(!e.state.seats[0].ash.contains(&morbidity));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 1, "its 2 back, a hand strike");
    assert!(!e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == morbidity));
}

// Obedience [dom]: an older blocker about to enter combat with the acting
// vampire unlocks it and ends the action. [LSJ 20070709] "The action has
// reached resolution, so is subject to NRA." [RTR 20180719] [ANK 20170105]
// [ANK 20200207] "The action ends unsuccessfully immediately, combat does
// not happen." [DOM]: the acting vampire stays locked.
#[test]
fn obedience_ends_a_younger_vampires_action() {
    for (mode, unlocked) in [(0u8, true), (1, false)] {
        let mut e = game(3);
        let young = e.add_minion(SeatId(0), with("cel", 4..=4), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let old = e.add_minion(SeatId(1), with("DOM", 8..=11), 4, false);
        hand(&mut e, 1, &["Obedience"]);
        bleed_blocked(&mut e, young, old);
        pick_when(&mut e, play(0, mode, old));
        let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, young), 4, "no combat");
        assert_eq!(e.state.seats[1].pool, 10, "unsuccessful");
        assert_eq!(!e.state.minions[young.index()].locked, unlocked);
        if unlocked {
            assert!(!req.options.contains(&bleed(young)), "NRA");
        }
        assert!(
            !e.state.minions[old.index()].locked,
            "not locked by blocking"
        );
    }
}

// [LSJ 19991025] Obedience "can be played by an unlocked older vampire to
// avoid a new combat queued after a preceding combat, if the opponent is
// still the acting vampire" (rules-qa 88.10): Lawbringer's rush, the older
// target declining Obedience before the first combat; Psyche! [CEL] queues
// a second; Obedience ends the action before it.
#[test]
fn obedience_avoids_the_combat_psyche_queues() {
    let mut e = game(3);
    let young = e.add_minion(SeatId(0), card("Lawbringer"), 3, false);
    let slots = &mut e.state.minions[young.index()].beast_disciplines;
    slots[slots.iter().position(Option::is_none).expect("a free slot")] = Some("CEL");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let old = e.add_minion(SeatId(1), with("DOM", 8..=11), 6, false);
    hand(&mut e, 0, &["Abuse of Power", "Psyche!"]);
    hand(&mut e, 1, &["Obedience"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(young),
            target: Target::Minion(old),
        },
    );
    let req = offering(&mut e, play(0, 1, young));
    assert!(e.combat().is_some(), "the first combat");
    e.choose(index_of(&req, play(0, 1, young)));
    let req = offering(&mut e, play(0, 2, old));
    assert!(e.combat().is_none(), "between the combats");
    e.choose(index_of(&req, play(0, 2, old)));
    assert!(e.combat().is_none(), "no second combat");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
    assert!(!e.state.minions[young.index()].locked, "[dom]: unlocked");
}

// Obedience against the combat the action itself begins (Abuse of Power's
// "enter combat with a minion", by Lawbringer of the Path of Caine): played by the target once blocks are
// declined, by an unlocked (or woken) vampire — a reaction (owner,
// 2026-10-08: rules-qa 55.7). The action ends unsuccessfully, no combat;
// [DOM] leaves the acting vampire locked.
#[test]
fn obedience_ends_an_action_entering_combat_with_an_older_vampire() {
    for (mode, unlocked) in [(2u8, true), (3, false)] {
        let mut e = game(3);
        let young = e.add_minion(SeatId(0), card("Lawbringer"), 3, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let old = e.add_minion(SeatId(1), with("DOM", 8..=11), 4, false);
        hand(&mut e, 0, &["Abuse of Power"]);
        hand(&mut e, 1, &["Obedience"]);
        pick(
            &mut e,
            Choice::Play {
                card: 0,
                mode: 0,
                actor: Some(young),
                target: Target::Minion(old),
            },
        );
        pick_when(&mut e, play(0, mode, old));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
        assert_eq!((blood(&e, young), blood(&e, old)), (3, 4), "no combat");
        assert_eq!(e.state.seats[1].pool, 10, "no pool burned");
        assert_eq!(!e.state.minions[young.index()].locked, unlocked);
        assert!(!e.state.minions[old.index()].locked);
        // Ended unblocked by the target's card: its seat's hindrance.
        use zvtes::profile::Count;
        assert_eq!(e.profiles[1].get(Count::Hindrances), 1);
    }

    // A locked target cannot play it, a reaction; neither can a vampire
    // younger than the acting one.
    for (cap, locked) in [(8..=11, true), (2..=3, false)] {
        let mut e = game(3);
        let young = e.add_minion(SeatId(0), card("Lawbringer"), 3, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let other = e.add_minion(SeatId(1), with("dom", cap), 2, false);
        e.state.minions[other.index()].locked = locked;
        hand(&mut e, 0, &["Abuse of Power"]);
        hand(&mut e, 1, &["Obedience"]);
        pick(
            &mut e,
            Choice::Play {
                card: 0,
                mode: 0,
                actor: Some(young),
                target: Target::Minion(other),
            },
        );
        for mode in [2, 3] {
            assert!(!request(&mut e).options.contains(&play(0, mode, other)));
        }
        never_offered_until(
            &mut e,
            SeatId(0),
            WindowLabel::MinionPhase,
            play(0, 2, other),
        );
    }
}

// Deep Song [ANI]: [ANK 20200702] "The lock and enter combat effects are
// simultaneous. Both are lost if the action ends before that (eg.
// Obedience)": the target ends the action, neither locked nor in combat.
#[test]
fn obedience_ends_deep_song_before_its_lock_and_combat() {
    let mut e = game(3);
    let young = e.add_minion(SeatId(0), with("ANI", 1..=5), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let old = e.add_minion(SeatId(1), with("DOM", 8..=11), 4, false);
    hand(&mut e, 0, &["Deep Song"]);
    hand(&mut e, 1, &["Obedience"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 1,
            actor: Some(young),
            target: Target::Minion(old),
        },
    );
    pick_when(&mut e, play(0, 2, old));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
    assert!(!e.state.minions[old.index()].locked, "the lock lost");
    assert_eq!((blood(&e, young), blood(&e, old)), (4, 4), "no combat");
}

// [ANK 20210131] Crypt's Sons: "If used to cancel combat, reactions cards
// pertaining to that combat (eg. Obedience) cannot be used afterwards":
// continued as if unblocked, no Obedience.
#[test]
fn obedience_is_not_played_after_crypts_sons_continues_the_action() {
    let mut e = game(3);
    let anarch = vampire(|c| {
        c.sect == Some(zvtes::crypt::Sect::Anarch) && c.capacity <= 4 && c.title == Title::None
    });
    let young = e.add_minion(SeatId(0), anarch, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let sons = in_play(&mut e, 0, "Crypt's Sons", Some(young));
    e.state.cards[sons.index()].counters = 3;
    let old = e.add_minion(SeatId(1), with("DOM", 8..=11), 4, false);
    hand(&mut e, 1, &["Obedience"]);
    bleed_blocked(&mut e, young, old);
    pick_when(&mut e, use_at(sons, 0, None, Target::None));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, old));
}

// Putrescent Sustenance [obl] or [tha]: an ash heap ally out of the game
// for 2 blood, or 2 life to a zombie ally.
#[test]
fn putrescent_sustenance_feeds_on_the_dead() {
    let (mut e, zbig, _) = table(ZBIGNIEW);
    let drone = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    e.state.seats[1].ash.push(find_library("Double Deuce"));
    hand(&mut e, 0, &["Putrescent Sustenance"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 1,
            actor: Some(zbig),
            target: Target::Minion(drone),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[drone.index()].blood, 4);
    assert!(e.state.seats[1]
        .removed
        .contains(&find_library("Double Deuce")));
}

// Target Vitals: an aim, played as the strike is chosen, in the strike
// phase [LSJ 20080409]: the hand strike landing, +2 damage of its kind,
// aggravated with Crossbreaker's [RTR 19970630] [TOM 19960225] [LSJ
// 20100527], and no press this round (rules-qa 55.1). [RTR 19960221] [PIB
// 20130319] On a dodge, no effect.
#[test]
fn target_vitals_adds_damage_to_a_landing_strike() {
    let (mut e, aitana, z) = table(AITANA);
    hand(&mut e, 0, &["Target Vitals"]);
    bleed_blocked(&mut e, aitana, z);
    let req = offering(&mut e, play(0, 0, aitana));
    assert_eq!(req.window, WindowLabel::CombatStrike);
    pick(&mut e, play(0, 0, aitana));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 3);

    // Crossbreaker's aggravated hands: the +2 aggravated too.
    let (mut e, crossbreaker, z) = table("Crossbreaker");
    hand(&mut e, 0, &["Target Vitals"]);
    bleed_blocked(&mut e, crossbreaker, z);
    pick_when(&mut e, vampire_ability(crossbreaker, 0));
    pick_when(&mut e, play(0, 0, crossbreaker));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor, "3 aggravated");

    // Dodged: nothing.
    let (mut e, aitana, _) = table(AITANA);
    let z = e.add_minion(SeatId(1), with("cel", 4..=11), 4, false);
    hand(&mut e, 0, &["Target Vitals"]);
    hand(&mut e, 1, &["Side Strike"]);
    bleed_blocked(&mut e, aitana, z);
    pick_when(&mut e, play(0, 0, aitana));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4);
}

// Target Vitals: "They can discard two combat cards to cancel this card as
// it is played." [PIB 20121031] "If it is canceled, any blood cost (eg.
// Terror Frenzy) has to be paid": Terror Frenzy [ANI] makes it cost 1
// blood, paid, the card canceled.
#[test]
fn target_vitals_is_canceled_for_two_combat_cards_its_cost_paid() {
    let (mut e, aitana, _) = table(AITANA);
    let z = e.add_minion(SeatId(1), with("ANI", 4..=11), 5, false);
    hand(&mut e, 0, &["Target Vitals"]);
    hand(
        &mut e,
        1,
        &["Terror Frenzy", "Side Strike", "Glancing Blow"],
    );
    bleed_blocked(&mut e, aitana, z);
    pick_when(&mut e, play(0, 1, z)); // Terror Frenzy [ANI]
    pick_when(&mut e, play(0, 0, aitana));
    pick_when(&mut e, Choice::PayToCancel);
    request(&mut e);
    assert_eq!(e.state.seats[1].hand.len(), 0, "the two combat cards");
    assert_eq!(blood(&e, aitana), 4 - 1, "its cost");
}

// Wall Street Night: during an undirected action, locked to give a minion
// of its controller +1 intercept; its investment action is inert (no
// investment card in the pool).
#[test]
fn wall_street_night_gives_intercept_on_undirected_actions() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 3), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let paper = in_play(&mut e, 1, "Wall Street Night, Financial Newspaper", None);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, block(z));
    pick_when(&mut e, use_at(paper, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[paper.index()].locked);
    assert_eq!(blood(&e, a), 3, "blocked: a hand strike, no hunt");
}
