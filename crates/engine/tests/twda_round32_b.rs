//! Round 32 (`ADDED_BATCHES`): Legacies of Blood completed, its combat
//! cards and vampires.
//! Windows with a single option are elided, so the scripts only stop where
//! a real decision exists.

use zvtes::cards::{AbilitySource, Target};
use zvtes::combat::Range;
use zvtes::crypt::{Title, CRYPT};
use zvtes::decision::{Choice, WindowLabel};
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

/// The `nth` untitled fixture vampire matching `f` (`fixture_vampire`).
#[track_caller]
fn textless(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    fixture_vampire(|c| f(c) && c.title == Title::None, nth)
}

/// The crypt card `name` (not advanced).
#[track_caller]
fn find_card(name: &str) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && !c.adv)
        .unwrap_or_else(|| panic!("no {name}")) as u16
}

/// Seat 0's A and a second vampire, seat 1's X and a second one: fixture
/// vampires of capacity 4 to 8, 4 blood each, A with `a_levels` and X with
/// `x_levels` gained.
fn duel(a_levels: &[&'static str], x_levels: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity);
    let a = e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    e.add_minion(SeatId(0), textless(mid, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.add_minion(SeatId(1), textless(mid, 3), 4, false);
    e.state.minions[a.index()].gained_levels = a_levels.to_vec();
    e.state.minions[x.index()].gained_levels = x_levels.to_vec();
    (e, a, x)
}

/// As `duel`, A of capacity 8 or more, X of 4 (younger).
fn elder_duel(
    a_levels: &[&'static str],
    x_levels: &[&'static str],
) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 8, 0), 4, false);
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 4, false);
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 3),
        4,
        false,
    );
    e.state.minions[a.index()].gained_levels = a_levels.to_vec();
    e.state.minions[x.index()].gained_levels = x_levels.to_vec();
    (e, a, x)
}

/// X blocks A's hunt (+1 stealth) with Eagle's Sight [aus], put in X's
/// hand at slot 0 (the rest of the hand moved down one).
fn hunt_blocked(e: &mut Engine, x: MinionId) {
    e.state.seats[1]
        .hand
        .insert(0, find_library("Eagle's Sight"));
    if !e.state.minions[x.index()].gained_levels.contains(&"aus") {
        e.state.minions[x.index()].gained_levels.push("aus");
    }
    pick_when(e, block(x));
    pick_when(e, play(0, 0, x));
    request(e);
}

/// A bleeds, X blocks: their combat begins.
fn fight(e: &mut Engine, a: MinionId, x: MinionId) {
    pick(e, bleed(a));
    pick_when(e, block(x));
}

/// The card `name` put on `host` (no play), in its first mode: its id.
fn on(e: &mut Engine, name: &str, host: MinionId) -> CardId {
    let owner = e.state.minions[host.index()].owner;
    e.state
        .cards
        .push(CardInPlay::new(find_library(name), owner, Some(host), 0));
    CardId(e.state.cards.len() as u32 - 1)
}

/// The strike of the weapon `w` (its ability `ability`) by its bearer.
fn weapon(w: CardId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(w),
        ability,
        actor: None,
        target: Target::None,
    }
}

/// The card `card` of the hand played by `actor` in `mode` on `target`.
fn play_on(card: u8, mode: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target,
    }
}

fn ash_has(e: &Engine, seat: usize, name: &str) -> bool {
    e.state.seats[seat].ash.contains(&find_library(name))
}

fn torpid(e: &Engine, m: MinionId) -> bool {
    let v = &e.state.minions[m.index()];
    v.in_play && v.in_torpor
}

fn burned(e: &Engine, m: MinionId) -> bool {
    !e.state.minions[m.index()].in_play
}

// ---- Armor of Caine's Fury ---------------------------------------------------

/// Armor of Caine's Fury [VAL]: "this vampire may prevent 2 damage from
/// the opposing minion's strikes each round". Round 1, A's 1-damage hand
/// strike: one use prevents it all [RTR 20041202], the second point not
/// carried over [ANK 20200318] [LSJ 20001114]; A presses (Flow Within the
/// Mountain [tha]); round 2, A's Earth Swords [MYT] (3R): two uses, 1
/// lands — 2 a round, not 3.
#[test]
fn armor_of_caines_fury_prevents_strike_damage_each_round() {
    let (mut e, a, x) = duel(&["myt", "myt", "tha"], &["val", "val"]);
    hand(&mut e, 0, &["Earth Swords", "Flow Within the Mountain"]);
    hand(&mut e, 1, &["Armor of Caine's Fury"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, x));
    pick_when(&mut e, Choice::HandStrike);
    let req = offering(&mut e, Choice::UsePrevent);
    assert_eq!(req.seat, SeatId(1));
    e.choose(index_of(&req, Choice::UsePrevent));
    pick_when(&mut e, play(1, 0, a));
    pick_when(&mut e, play(0, 2, a));
    let req = offering(&mut e, Choice::UsePrevent);
    e.choose(index_of(&req, Choice::UsePrevent));
    pick(&mut e, Choice::UsePrevent);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, x),
        4 - 1 - 1,
        "the card's blood, round 2's third point"
    );
}

// ---- Ashes to Ashes -----------------------------------------------------------

/// Ashes to Ashes [THN]: "Prevent all damage. This vampire unlocks and goes
/// to torpor (ending combat) … and this vampire gains 2 blood". X, locked
/// by its block, prevents A's hand strike: unlocked, 1 + 2 blood, in
/// torpor, the combat over. Backs [LSJ 20001114]'s other half below.
#[test]
fn ashes_to_ashes_prevents_all_unlocks_and_sends_its_vampire_to_torpor() {
    let (mut e, a, x) = duel(&[], &["thn", "thn"]);
    e.state.minions[x.index()].blood = 1;
    hand(&mut e, 1, &["Ashes to Ashes"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(torpid(&e, x));
    assert!(!e.state.minions[x.index()].locked);
    assert_eq!(blood(&e, x), 1 + 2, "nothing mended, 2 gained");
    assert_eq!(blood(&e, a), 4 - 1, "X's own strike landed on A");
}

/// Ashes to Ashes [thn]: "Cannot be used if there is no damage to prevent"
/// [LSJ 20001114]: A's strike dodged (Dust to Dust [thn]), Ashes never
/// offered to X.
#[test]
fn ashes_to_ashes_needs_damage_to_prevent() {
    let (mut e, a, x) = duel(&[], &["thn"]);
    hand(&mut e, 1, &["Ashes to Ashes", "Dust to Dust"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(1, 1, x));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, x));
    assert!(!torpid(&e, x));
}

/// Ashes to Ashes [thn]: its torpor comes with the damage step, so an end
/// of combat card comes first [LSJ 20011214-4]: A's Decapitate [POT]
/// burns X instead, A's Amaranth diablerizes X; and Undying Tenacity
/// defers it, the unlock standing [LSJ 20020211] [LSJ 20021122]: X stays
/// ready to the combat's end.
#[test]
fn ashes_to_ashes_torpor_meets_decapitate_and_undying_tenacity() {
    let (mut e, a, x) = duel(&["pot", "pot"], &["thn", "for"]);
    hand(&mut e, 0, &["Decapitate"]);
    hand(&mut e, 1, &["Ashes to Ashes"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    pick_when(&mut e, play(0, 2, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(burned(&e, x), "Decapitated");

    let (mut e, a, x) = duel(&[], &["thn"]);
    hand(&mut e, 0, &["Amaranth"]);
    hand(&mut e, 1, &["Ashes to Ashes"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(burned(&e, x), "diablerized");

    let (mut e, a, x) = duel(&[], &["thn", "for"]);
    hand(&mut e, 1, &["Ashes to Ashes", "Undying Tenacity"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    let req = offering(&mut e, play(0, 0, x));
    assert_eq!(req.window, WindowLabel::CombatPrevention);
    e.choose(index_of(&req, play(0, 0, x)));
    assert!(!e.state.minions[x.index()].locked, "unlocked");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(torpid(&e, x), "in torpor once the combat ended");
}

/// Ashes to Ashes [for] against Amaranth: "the diablerie is unsuccessful.
/// Another Amaranth can be played" [LSJ 19970224] [LSJ 20011214-4]. X,
/// with no blood to mend A's hand strike, meets A's
/// Amaranth: X's [for] sends it to torpor, a second Amaranth is offered;
/// declined, X is in torpor, not diablerized.
#[test]
fn ashes_to_ashes_fortitude_foils_an_amaranth() {
    let (mut e, a, x) = duel(&[], &["for"]);
    e.state.minions[x.index()].blood = 0;
    hand(&mut e, 0, &["Amaranth", "Amaranth"]);
    hand(&mut e, 1, &["Ashes to Ashes"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, x));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.seat, SeatId(0), "another Amaranth offered");
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(torpid(&e, x), "in torpor, not diablerized");
}

/// Ashes to Ashes [for]: "Only usable by a vampire being burned; he or she
/// is sent into torpor instead." X with no blood takes 2 aggravated
/// damage (A's Devil-Channel: Hands [abo] and Basilisk's Touch [pot], X
/// younger): burned without it, in torpor with it.
#[test]
fn ashes_to_ashes_sends_a_vampire_burned_by_damage_to_torpor() {
    for with in [false, true] {
        let (mut e, a, x) = elder_duel(&["abo", "pot"], &["for"]);
        e.state.minions[x.index()].blood = 0;
        hand(&mut e, 0, &["Devil-Channel: Hands", "Basilisk's Touch"]);
        if with {
            hand(&mut e, 1, &["Ashes to Ashes"]);
        }
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, 1, a));
        pick_when(&mut e, play(0, 0, a));
        if with {
            pick_when(&mut e, play(0, 0, x));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(torpid(&e, x), with);
        assert_eq!(burned(&e, x), !with);
    }
}

// ---- Basilisk's Touch ---------------------------------------------------------

/// Basilisk's Touch [myt]/[MYT]: A's hand strike lands on the younger X,
/// which mends it, then goes to torpor, burning 1 more blood at [MYT]
/// (rules-qa 116.138: successfully inflicted). Prevented in full (Soak
/// [FOR]), nothing happens.
#[test]
fn basilisks_touch_sends_a_vampire_hit_by_a_hand_strike_to_torpor() {
    let mut left = Vec::new();
    for (mode, prevented) in [(1, false), (2, false), (2, true)] {
        let (mut e, a, x) = elder_duel(&["myt", "myt"], &["for", "for"]);
        hand(&mut e, 0, &["Basilisk's Touch"]);
        if prevented {
            hand(&mut e, 1, &["Soak"]);
        }
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, mode, a));
        if prevented {
            pick_when(&mut e, play(0, 1, x));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(torpid(&e, x), !prevented, "{mode} {prevented}");
        left.push(blood(&e, x));
    }
    assert_eq!(left[0], left[1] + 1, "[MYT]'s 1 blood more");
    assert_eq!(left[2], 4, "nothing landed");
}

/// Basilisk's Touch [myt]: its torpor opens the going-to-torpor step as a
/// damage torpor does: A's Decapitate [POT] is offered there and burns X
/// instead (owner, rules-qa 124.26).
#[test]
fn basilisks_touch_torpor_offers_decapitate() {
    let (mut e, a, x) = elder_duel(&["myt", "pot", "pot"], &[]);
    hand(&mut e, 0, &["Basilisk's Touch", "Decapitate"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 2, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(burned(&e, x));
}

/// Basilisk's Touch: "Only usable in combat with an ally or younger
/// vampire": not against an older X.
#[test]
fn basilisks_touch_is_not_offered_against_an_older_vampire() {
    let (mut e, x, a) = elder_duel(&[], &["myt", "pot"]);
    hand(&mut e, 1, &["Basilisk's Touch"]);
    pick(&mut e, bleed(x));
    pick_when(&mut e, block(a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

// ---- Burning Touch ------------------------------------------------------------

/// Burning Touch [val]: "If this action is blocked, the blocking minion
/// burns one blood or life before combat begins", among the "if blocked"
/// effects [ANK 20220116] [ANK 20211207] [ANK 20210627] [RTR 19991206]
/// [LSJ 20091125]: X blocks
/// A's bleed, burns 1, then takes A's hand strike. [tha]: "This damage
/// cannot be prevented by cards that require Fortitude": X's Soak [for]
/// is not offered once it lands (LSJ 20050628).
#[test]
fn burning_touch_burns_the_blocker_and_bars_fortitude_prevention() {
    let (mut e, a, x) = duel(&["val", "tha"], &["for"]);
    hand(&mut e, 0, &["Burning Touch", "Burning Touch"]);
    hand(&mut e, 1, &["Soak"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, block(x));
    request(&mut e);
    assert_eq!(blood(&e, x), 4 - 1, "burned before the combat");
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
    assert_eq!(blood(&e, x), 4 - 1 - 1);
}

/// Burning Touch [VAL]: "Only usable during a bleed action. +1 bleed
/// (limited)."
#[test]
fn burning_touch_superior_is_a_limited_bleed() {
    let (mut e, a, _) = duel(&["val", "val"], &[]);
    hand(&mut e, 0, &["Burning Touch"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 2, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

// ---- Chameleon's Colors ---------------------------------------------------------

/// Chameleon's Colors [SPI]: "Only usable at long range. Strike: combat
/// ends. If this vampire was blocked while performing an action other than
/// bleeding, the action continues as if unblocked." A's blocked hunt
/// continues ([ani] maneuver to long range first): A gains blood; a
/// blocked bleed does not (Morphean Blow's exclusion). Short range: not
/// offered.
#[test]
fn chameleons_colors_superior_continues_a_blocked_non_bleed_action() {
    for hunt in [true, false] {
        let (mut e, a, x) = duel(&["ani", "spi", "spi"], &[]);
        hand(&mut e, 0, &["Chameleon's Colors", "Chameleon's Colors"]);
        let act = if hunt {
            action(a, zvtes::decision::ActionKind::Hunt)
        } else {
            bleed(a)
        };
        pick(&mut e, act);
        if hunt {
            hunt_blocked(&mut e, x);
        } else {
            pick_when(&mut e, block(x));
        }
        pick_when(&mut e, play(0, 0, a));
        pick_when(&mut e, play(0, 2, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if hunt {
            assert_eq!(blood(&e, a), 4 - 2 + 1, "the hunt continued");
        } else {
            assert_eq!(e.state.seats[1].pool, 10, "the bleed did not");
        }
    }
    let (mut e, a, x) = duel(&["spi", "spi"], &["aus"]);
    hand(&mut e, 0, &["Chameleon's Colors"]);
    pick(&mut e, action(a, zvtes::decision::ActionKind::Hunt));
    hunt_blocked(&mut e, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 2, a));
}

// ---- Circle ---------------------------------------------------------------------

/// Circle: "Requires a ready minion with flight. Press. If another round of
/// combat occurs, this minion gets an optional maneuver during that
/// round." A flier presses, round 2 offers it a maneuver; a minion without
/// flight is never offered the card.
#[test]
fn circle_presses_and_grants_a_maneuver_next_round() {
    let mut e = game(2);
    let flier = fixture_vampire(|c| c.disciplines.contains(&"FLIGHT") && c.capacity >= 3, 0);
    let a = e.add_minion(SeatId(0), flier, 4, false);
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    let x = e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 2),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 3),
        4,
        false,
    );
    hand(&mut e, 0, &["Circle"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    let req = offering(&mut e, Choice::UseManeuver);
    assert_eq!(req.seat, SeatId(0));
    assert_eq!(req.window, WindowLabel::CombatDetermineRange);

    let (mut e, a, x) = duel(&[], &[]);
    hand(&mut e, 0, &["Circle"]);
    fight(&mut e, a, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

// ---- Collapse the Arches ----------------------------------------------------------

/// Collapse the Arches [VIS]: "Requires a ready vampire who has blocked a
/// directed action … Only usable at long range … 4R damage. This strike
/// cannot be dodged … with first strike." X, the blocker of A's bleed,
/// maneuvers to long range (Crawling Chamber [tha]) and strikes through
/// A's dodge (Dust to Dust [thn]) [LSJ 20030902-2] [LSJ 20060808-1]: 4 on
/// A. A, the acting
/// minion, is never offered its own copy.
#[test]
fn collapse_the_arches_only_for_the_blocker_and_through_a_dodge() {
    let (mut e, a, x) = duel(&["thn", "vis", "vis"], &["tha", "vis", "vis"]);
    hand(&mut e, 0, &["Dust to Dust", "Collapse the Arches"]);
    hand(&mut e, 1, &["Crawling Chamber", "Collapse the Arches"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 1, a));
    let req = offering(&mut e, play(0, 2, x));
    assert!(!req.options.contains(&play(1, 2, a)) && req.seat == SeatId(1));
    e.choose(index_of(&req, play(0, 2, x)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 0, "4 through the dodge");
}

/// Collapse the Arches: "Only usable in the resulting combat" (rules-qa
/// 122.74), of "a directed action": X, blocking a hunt (undirected), at
/// long range by Crawling Chamber [tha], is not offered it.
#[test]
fn collapse_the_arches_needs_a_blocked_directed_action() {
    let (mut e, a, x) = duel(&[], &["tha", "tha", "aus", "thn"]);
    hand(
        &mut e,
        1,
        &["Crawling Chamber", "Collapse the Arches", "Dust to Dust"],
    );
    pick(&mut e, action(a, zvtes::decision::ActionKind::Hunt));
    hunt_blocked(&mut e, x);
    pick_when(&mut e, play(0, 0, x));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert_eq!(e.combat().map(|c| c.range), Some(Range::Long));
    assert!(req.options.contains(&play(1, 1, x)), "Dust to Dust there");
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::Play { card: 0, .. })));
}

// ---- Crawling Chamber -------------------------------------------------------------

/// Crawling Chamber [VIS]: "Opposing minion takes 1R damage each round of
/// combat during the press step." Environmental [RTR 19970630],
/// preventable: X passes the strike's prevention and plays Soak [for] on
/// the press step's damage. Combat ended early (A's Flow Within the
/// Mountain [vis] strike): none [LSJ 20010312] [LSJ 20001027].
#[test]
fn crawling_chamber_damages_in_each_press_step_preventably() {
    let (mut e, a, x) = duel(&["vis", "vis"], &["for"]);
    hand(&mut e, 0, &["Crawling Chamber"]);
    hand(&mut e, 1, &["Soak"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    until(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    e.choose(0);
    let req = until(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    assert!(matches!(
        e.combat().map(|c| c.press_step_prevention),
        Some(true)
    ));
    e.choose(index_of(&req, play(0, 0, x)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 1, "the hand strike only");

    let (mut e, a, x) = duel(&["vis", "vis"], &[]);
    hand(&mut e, 0, &["Crawling Chamber", "Flow Within the Mountain"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4, "combat ended: no press step damage");
}

// ---- Creeping Infection ----------------------------------------------------------

/// Creeping Infection [THN]: the hand strike lands, the card itself goes on
/// X, still seat 0's; next turn, as A announces a bleed of seat 1, seat 0
/// burns it: X cannot block; the bleed +1 for the card on X... gone with
/// the burn, so 1.
#[test]
fn creeping_infection_goes_on_the_opposing_minion_and_bars_its_block() {
    let (mut e, a, x) = duel(&["thn", "thn"], &[]);
    hand(&mut e, 0, &["Creeping Infection"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = find_library("Creeping Infection");
    let id = (0..e.state.cards.len())
        .find(|&i| {
            let c = &e.state.cards[i];
            c.card == card && c.in_play && !c.effect_only && c.host == Some(x)
        })
        .expect("the card itself on X") as u32;
    assert!(!ash_has(&e, 0, "Creeping Infection"));
    assert_eq!(e.state.card_controller(CardId(id)), SeatId(0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a));
    let burn = Choice::UseAbility {
        source: AbilitySource::Card(CardId(id)),
        ability: 0,
        actor: Some(a),
        target: Target::None,
    };
    let req = offering(&mut e, burn);
    assert_eq!(req.window, WindowLabel::ActionAsAnnounced);
    e.choose(index_of(&req, burn));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(x));
    assert!(ash_has(&e, 0, "Creeping Infection"));
}

/// Creeping Infection [THN]: "this striking vampire gets +1 bleed when
/// bleeding this minion's controller"; dodged, the card stays in the ash
/// heap (rules-qa 21.1).
#[test]
fn creeping_infection_superior_bleeds_its_controller_for_one_more() {
    let (mut e, a, x) = duel(&["thn", "thn"], &[]);
    hand(&mut e, 0, &["Creeping Infection"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[x.index()].locked = true;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[x.index()].locked = true;
    let pool = e.state.seats[1].pool;
    pick(&mut e, bleed(a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, pool - 2);

    let (mut e, a, x) = duel(&["thn", "thn"], &["thn"]);
    hand(&mut e, 0, &["Creeping Infection"]);
    hand(&mut e, 1, &["Dust to Dust"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Creeping Infection"), "dodged: not put on");
}

// ---- Darkling Trickery ------------------------------------------------------------

/// Darkling Trickery [MYT]: A's .44 Magnum strike inflicts no damage on X,
/// and A takes 1R environmental damage in normal strike resolution beside
/// X's hand strike [RTR 19941109] [TOM 19951216] [PIB 20140324] [ANK
/// 20200422]; against a melee weapon (Baseball Bat) it is not offered.
#[test]
fn darkling_trickery_voids_a_gun_strike_and_damages_its_striker() {
    let (mut e, a, x) = duel(&[], &["myt", "myt"]);
    let gun = on(&mut e, ".44 Magnum", a);
    hand(&mut e, 1, &["Darkling Trickery"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 0));
    pick_when(&mut e, play(0, 2, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4, "the gun inflicted nothing");
    assert_eq!(blood(&e, a), 4 - 2, "X's hand strike and 1R");

    let (mut e, a, x) = duel(&[], &["myt", "myt"]);
    let bat = on(&mut e, "Baseball Bat", a);
    hand(&mut e, 1, &["Darkling Trickery"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(bat, 0));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 2, x));
}

// ---- The Death of My Conscience ---------------------------------------------------

/// The Death of My Conscience [POT]: X = 3, three cards picked and burned
/// to the ash heap, +6 strength; neither they nor the card are replaced
/// until after combat: A's hand of 7 is 3 while X prevents, 7 again once
/// the combat ended. A capacity-5 vampire is never offered it ("capacity
/// above 5").
#[test]
fn death_of_my_conscience_burns_cards_for_strength_and_holds_their_replacement() {
    let (mut e, a, x) = elder_duel(&["pot", "pot"], &["for"]);
    hand(&mut e, 0, &["Death of My Conscience, The"]);
    fill_hand(&mut e, 0);
    for _ in 0..4 {
        e.state.seats[0].library.push(find_library("Torrent"));
    }
    hand(&mut e, 1, &["Soak"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick(&mut e, set_x(3));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    assert!(req.options.contains(&play(0, 0, x)));
    assert_eq!(
        e.state.seats[0].hand.len(),
        7 - 1 - 3,
        "nothing replaced yet"
    );
    let mist = find_library("Form of Mist");
    assert_eq!(
        e.state.seats[0].ash.iter().filter(|&&c| c == mist).count(),
        3
    );
    e.choose(0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(torpid(&e, x), "a 7-or-more hand strike");
    assert_eq!(e.state.seats[0].hand.len(), 7, "replaced after combat");

    let mut e = game(2);
    let five = e.add_minion(SeatId(0), textless(|c| c.capacity == 5, 0), 4, false);
    e.state.minions[five.index()].gained_levels = vec!["pot"];
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    let x = e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 2),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 3),
        4,
        false,
    );
    hand(&mut e, 0, &["Death of My Conscience, The", "Form of Mist"]);
    fight(&mut e, five, x);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play(0, 0, five),
    );
}

// ---- Death of the Drum ------------------------------------------------------------

/// Death of the Drum [mel]: "Strike: 2R damage, only preventable by cards
/// requiring Fortitude [for] or Visceratika [vis]": landed at long range
/// (Chameleon's Colors [ani]), X's Soak [for] is offered, its Armor of
/// Caine's Fury [pre] is not, even to cycle [LSJ 20050628]; Soak prevents
/// both. And an allowance (Armor of Caine's Fury [val], played before
/// range) is barred under it (rules-qa Batch 124).
#[test]
fn death_of_the_drum_is_only_prevented_by_fortitude_or_visceratika() {
    let (mut e, a, x) = duel(&["ani", "mel"], &["for", "pre"]);
    hand(&mut e, 0, &["Chameleon's Colors", "Death of the Drum"]);
    hand(&mut e, 1, &["Soak", "Armor of Caine's Fury"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, a));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    assert!(req.options.contains(&play(0, 0, x)));
    assert!(!req.options.contains(&play(1, 0, x)), "a [pre] prevention");
    e.choose(index_of(&req, play(0, 0, x)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4);

    let (mut e, a, x) = duel(&["ani", "mel"], &["val"]);
    hand(&mut e, 0, &["Chameleon's Colors", "Death of the Drum"]);
    hand(&mut e, 1, &["Armor of Caine's Fury"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, a));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::UsePrevent,
    );
    assert_eq!(blood(&e, x), 4 - 1 - 2, "the Armor's blood, the 2R");
}

/// Seat 0's A with `a_levels`, seat 1's Salubri X with [for] and
/// Tranquility Shield [for] in hand ("This combat, this vampire can prevent
/// 1 damage each round"), and a second vampire each.
fn shielded_duel(a_levels: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity);
    let a = e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    e.add_minion(SeatId(0), textless(mid, 1), 4, false);
    let x = e.add_minion(
        SeatId(1),
        textless(|c| c.clan == "Salubri" && mid(c), 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.state.minions[a.index()].gained_levels = a_levels.to_vec();
    e.state.minions[x.index()].gained_levels = vec!["for"];
    hand(&mut e, 1, &["Tranquility Shield"]);
    (e, a, x)
}

/// Death of the Drum: "only preventable by cards requiring Fortitude [for]
/// or Visceratika [vis]": an allowance a [for] card granted (Tranquility
/// Shield) prevents its damage (the allowance records its card's
/// Discipline).
#[test]
fn death_of_the_drum_admits_a_fortitude_cards_allowance() {
    let (mut e, a, x) = shielded_duel(&["ani", "mel"]);
    hand(&mut e, 0, &["Chameleon's Colors", "Death of the Drum"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, a));
    let req = offering(&mut e, Choice::UsePrevent);
    assert_eq!(req.seat, SeatId(1));
}

/// Burning Touch [tha]: "This damage cannot be prevented by cards that
/// require Fortitude [for]": the allowance Tranquility Shield [for] granted
/// is barred that round (rules-qa 122.3), A's hand strike's damage and the
/// Touch's both taken; without the Touch, the allowance is offered.
#[test]
fn burning_touch_bars_a_fortitude_cards_allowance() {
    let (mut e, a, x) = shielded_duel(&["tha"]);
    hand(&mut e, 0, &["Burning Touch"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::UsePrevent,
    );
    assert_eq!(blood(&e, x), 4 - 1 - 1, "the Shield's blood, the Touch's 1");

    let (mut e, a, x) = shielded_duel(&[]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, Choice::UsePrevent);
}

// ---- Devil-Channel ----------------------------------------------------------------

/// Devil-Channel: Back [abo]: "This vampire burns 1 blood to get an
/// additional strike (limited) this round, only usable to make a hand
/// strike." A's Baseball Bat strikes first; the extra strike offers the
/// hand strike, not the bat. With no blood, not offered [ANK 20210226].
#[test]
fn devil_channel_back_burns_blood_for_a_hand_only_additional_strike() {
    let (mut e, a, x) = duel(&["abo", "thn"], &[]);
    let bat = on(&mut e, "Baseball Bat", a);
    hand(&mut e, 0, &["Devil-Channel: Back", "Creeping Infection"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(bat, 0));
    pick_when(&mut e, play(0, 1, a));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(req.options.contains(&Choice::HandStrike));
    assert!(req.options.contains(&play(0, 0, a)), "a hand strike card");
    assert!(!req.options.contains(&weapon(bat, 0)), "hand strikes only");
    e.choose(index_of(&req, Choice::HandStrike));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1 - 1, "its blood, X's hand strike");

    let (mut e, a, x) = duel(&["abo"], &[]);
    hand(&mut e, 0, &["Devil-Channel: Back"]);
    pick(&mut e, bleed(a));
    e.state.minions[a.index()].blood = 0;
    pick_when(&mut e, block(x));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
}

/// Devil-Channel: Hands [abo]: "For the remainder of the combat, this
/// vampire's hand damage is aggravated": X, 4 blood, goes to torpor from
/// A's 1-damage hand strike.
#[test]
fn devil_channel_hands_aggravates_hand_damage() {
    let (mut e, a, x) = duel(&["abo"], &[]);
    hand(&mut e, 0, &["Devil-Channel: Hands"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(torpid(&e, x));
}

// ---- Dust to Dust -----------------------------------------------------------------

/// Dust to Dust [THN]: "Strike: dodge, with an optional press … with an
/// optional maneuver": X's play at the maneuver takes the range long and
/// commits its dodge; the press is offered this round [TOM 19960521].
#[test]
fn dust_to_dust_superior_maneuvers_into_a_committed_dodge_and_presses() {
    let (mut e, a, x) = duel(&[], &["thn", "thn"]);
    hand(&mut e, 1, &["Dust to Dust"]);
    fight(&mut e, a, x);
    let req = until(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    e.choose(index_of(&req, play(0, 2, x)));
    let req = offering(&mut e, Choice::UsePress);
    assert_eq!(req.seat, SeatId(1));
    assert_eq!(e.combat().map(|c| c.range), Some(Range::Long));
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4, "nothing reached X");
}

// ---- Flow Within the Mountain -----------------------------------------------------

/// Flow Within the Mountain [VIS]: "Strike: combat ends. If this vampire
/// was blocked while performing an action, the action continues as if
/// unblocked." A's blocked bleed continues: seat 1 bled for 1.
#[test]
fn flow_within_the_mountain_superior_continues_a_blocked_action() {
    let (mut e, a, x) = duel(&["vis", "vis"], &[]);
    hand(&mut e, 0, &["Flow Within the Mountain"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1);
    assert_eq!(blood(&e, a), 4 - 1, "its cost, no damage");
}

// ---- Fractured Armament -------------------------------------------------------------

/// Fractured Armament: "Strike: destroy equipment" — not offered with no
/// equipment to destroy [RTR 19980928]; [pot] burns X's Leather Jacket and
/// no damage, not a damaging strike [LSJ 19970225]; [POT] burns it and
/// inflicts 1, a damaging strike [LSJ 19970225].
#[test]
fn fractured_armament_burns_an_equipment_with_its_damage_at_superior() {
    let (mut e, a, x) = duel(&["pot", "pot"], &[]);
    hand(&mut e, 0, &["Fractured Armament"]);
    fight(&mut e, a, x);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { card: 0, .. })
    });
    for mode in [0, 1] {
        let (mut e, a, x) = duel(&["pot", "pot"], &[]);
        let jacket = on(&mut e, "Leather Jacket", x);
        hand(&mut e, 0, &["Fractured Armament"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play_on(0, mode, a, Target::Card(jacket)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.cards[jacket.index()].in_play);
        assert!(ash_has(&e, 1, "Leather Jacket"));
        assert_eq!(blood(&e, x), 4 - mode);
    }
}

// ---- Brother's Blood ----------------------------------------------------------------

/// The `nth` option of the pending request matching `want`: chosen.
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

/// Is `o` the use of ability `k` of a card named `name`?
fn uses_card(e: &Engine, o: &Choice, name: &str, k: u8) -> bool {
    matches!(*o, Choice::UseAbility { source: AbilitySource::Card(c), ability, .. }
        if ability == k && e.state.cards[c.index()].card == find_library(name))
}

/// Karl and Ilse (Torrance, one circle) for seat 0, Ilse's blood `karl`,
/// a seat-1 vampire X.
fn torrance_fight(karl: u8, x_levels: &[&'static str]) -> (Engine, MinionId, MinionId, MinionId) {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Karl"), karl, false);
    let i = e.add_minion(SeatId(0), find_card("Ilse"), 6, false);
    let x = e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 2),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 3),
        4,
        false,
    );
    e.state.minions[x.index()].gained_levels = x_levels.to_vec();
    (e, k, i, x)
}

/// Brother's Blood [san]: "Can be played even if there is no damage yet"
/// [LSJ 20100527]: Ilse, out of the combat, plays it before range; Karl,
/// with no blood, takes X's hand strike: Ilse burns 1 to mend it, Karl
/// stays ready. The card is an effect-only card on Ilse for the combat,
/// gone after it (rules-qa 68.12).
#[test]
fn brothers_blood_mends_a_circle_members_damage_with_the_donors_blood() {
    let (mut e, i, k, x) = torrance_fight(4, &[]);
    hand(&mut e, 0, &["Brother's Blood"]);
    pick(&mut e, bleed(i));
    e.state.minions[i.index()].blood = 0;
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, k));
    let ee = &e as *const Engine;
    pick_matching(&mut e, |o| {
        uses_card(unsafe { &*ee }, o, "Brother's Blood", 0)
    });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!torpid(&e, i), "mended by Ilse");
    assert_eq!(blood(&e, k), 6 - 1);
    assert!(ash_has(&e, 0, "Brother's Blood"));
}

/// Brother's Blood [SAN] "prevent destruction" (rules-qa Batch 124: the
/// aggravated wound stands): X's Death of the Drum [MEL] (2R aggravated,
/// at long range by Chameleon's Colors [ani]) on Karl (1 blood): the first
/// point wounds her, Ilse pays the second; Karl in torpor with her blood.
/// The card stays on Ilse, not in the ash heap.
#[test]
fn brothers_blood_prevents_destruction_not_torpor() {
    let (mut e, i, k, x) = torrance_fight(1, &["ani", "mel", "mel"]);
    hand(&mut e, 0, &["Brother's Blood"]);
    hand(&mut e, 1, &["Chameleon's Colors", "Death of the Drum"]);
    fight(&mut e, i, x);
    pick_when(&mut e, play(0, 1, k));
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 2, x));
    let ee = &e as *const Engine;
    pick_matching(&mut e, |o| {
        uses_card(unsafe { &*ee }, o, "Brother's Blood", 1)
    });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(torpid(&e, i));
    assert_eq!(blood(&e, i), 1, "the second point Ilse's");
    assert_eq!(blood(&e, k), 6 - 1);
    let card = find_library("Brother's Blood");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == card && c.in_play && !c.effect_only && c.host == Some(k)));
    assert!(!ash_has(&e, 0, "Brother's Blood"));
}

/// Brother's Blood "prevent destruction" on a combatant wounded by normal
/// damage (rulebook "Aggravated damage": "For each point of aggravated
/// damage that is successfully inflicted on a wounded vampire, they must
/// burn 1 blood to prevent their destruction"): Karl, 0 blood, takes 2
/// normal (X's two Murder of Crows) and 1 aggravated (X's hand strike);
/// Ilse pays 1, Karl goes to torpor, not burned.
#[test]
fn brothers_blood_prevents_destruction_of_a_combatant_wounded_by_normal_damage() {
    let (mut e, i, k, x) = torrance_fight(4, &[]);
    for _ in 0..2 {
        let id = in_play(&mut e, 1, "Murder of Crows", Some(x));
        e.state.cards[id.index()].counters = 1;
    }
    e.state.minions[x.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    hand(&mut e, 0, &["Brother's Blood"]);
    pick(&mut e, bleed(i));
    e.state.minions[i.index()].blood = 0;
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 1, k));
    let ee = &e as *const Engine;
    pick_matching(&mut e, |o| {
        uses_card(unsafe { &*ee }, o, "Brother's Blood", 1)
    });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[i.index()].in_play, "not burned");
    assert!(torpid(&e, i), "the wound stands");
    assert_eq!(blood(&e, k), 6 - 1);
}

// ---- Free Fight ---------------------------------------------------------------------

/// Free Fight [san]: Karl, blocked, locks Ilse with it; before range Ilse
/// becomes the combatant (the combat goes on): X's hand strike lands on
/// Ilse, Karl out of it.
#[test]
fn free_fight_swaps_in_a_locked_member() {
    let (mut e, i, k, x) = torrance_fight(4, &[]);
    hand(&mut e, 0, &["Free Fight"]);
    fight(&mut e, i, x);
    pick_when(&mut e, play(0, 0, i));
    pick(&mut e, Choice::ChooseMinion(k));
    pick_when(&mut e, Choice::ChooseMinion(k));
    assert!(e.state.minions[k.index()].locked);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, k), 6 - 1, "Ilse struck");
    assert_eq!(blood(&e, i), 4, "Karl out of the combat");
}

/// Free Fight, the combatant leaving the ready region mid-round: Karl, with
/// no blood, declines the before-range swap and goes to torpor from X's
/// strike; Ilse takes his place where he left, after strike resolution: no
/// new round without a press ([san]); with [SAN]'s press she continues it
/// and takes X's round-2 strike (owner, rules-qa 124.89).
#[test]
fn free_fight_newcomer_goes_on_from_strike_resolution() {
    for superior in [false, true] {
        let (mut e, i, k, x) = torrance_fight(4, &[]);
        if superior {
            e.state.minions[i.index()].gained_levels = vec!["san", "san"];
        }
        hand(&mut e, 0, &["Free Fight"]);
        pick(&mut e, bleed(i));
        e.state.minions[i.index()].blood = 0;
        pick_when(&mut e, block(x));
        pick_when(&mut e, play(0, superior as u8, i));
        pick(&mut e, Choice::ChooseMinion(k));
        let req = offering(&mut e, Choice::ChooseMinion(k));
        e.choose(index_of(&req, Choice::Pass));
        let req = offering(&mut e, Choice::ChooseMinion(k));
        assert!(torpid(&e, i), "Karl left the ready region");
        e.choose(index_of(&req, Choice::ChooseMinion(k)));
        if superior {
            pick_when(&mut e, Choice::UsePress);
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(torpid(&e, i));
        assert_eq!(blood(&e, k), if superior { 6 - 1 } else { 6 }, "{superior}");
    }
}

/// Free Fight [SAN]: "each of these Blood Brothers gets an optional press":
/// Ilse, swapped in before range, has her own press that round (Karl's
/// banked on his side is lost with it).
#[test]
fn free_fight_superior_gives_the_swapped_in_brother_a_press() {
    let (mut e, i, k, x) = torrance_fight(4, &[]);
    e.state.minions[i.index()].gained_levels = vec!["san", "san"];
    hand(&mut e, 0, &["Free Fight"]);
    fight(&mut e, i, x);
    pick_when(&mut e, play(0, 1, i));
    pick(&mut e, Choice::ChooseMinion(k));
    pick_when(&mut e, Choice::ChooseMinion(k));
    let req = offering(&mut e, Choice::UsePress);
    assert_eq!(req.seat, SeatId(0));
    assert_eq!(
        e.combat()
            .expect("combat")
            .sides
            .iter()
            .find(|s| s.seat == SeatId(0))
            .map(|s| s.minion),
        Some(k)
    );
}

/// Free Fight [san]: a Brother swapped in brings its own "each combat"
/// bonuses, as one entering any combat does (no ruling says otherwise):
/// Ilse, employing a Dread Mastiff [ANI] ("the employer gets 1 optional
/// press each combat"), swapped in before range, has her press.
#[test]
fn free_fight_newcomer_brings_its_each_combat_press() {
    let (mut e, i, k, x) = torrance_fight(4, &[]);
    let id = in_play_mode(&mut e, 0, "Dread Mastiff", Some(k), 1);
    e.state.cards[id.index()].counters = 2;
    hand(&mut e, 0, &["Free Fight"]);
    fight(&mut e, i, x);
    pick_when(&mut e, play(0, 0, i));
    pick(&mut e, Choice::ChooseMinion(k));
    pick_when(&mut e, Choice::ChooseMinion(k));
    let req = offering(&mut e, Choice::UsePress);
    assert_eq!(req.seat, SeatId(0));
    assert_eq!(
        e.combat()
            .expect("combat")
            .sides
            .iter()
            .find(|s| s.seat == SeatId(0))
            .map(|s| s.minion),
        Some(k)
    );
}

/// Ashes to Ashes [for] by an ally playing Fortitude cards as a vampire
/// (Reanimated Corpse): "the ally is burned instead of going to torpor"
/// [ANK 20170412]; a card it plays as a vampire sending it to torpor burns
/// it [LSJ 20060409]. A's Earth Swords [MYT] (3R) on the 2-life corpse:
/// Ashes offered (a legal no-op), the corpse burned.
#[test]
fn ashes_to_ashes_burns_an_ally_anyway() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 2),
        4,
        false,
    );
    e.state.minions[a.index()].gained_levels = vec!["myt", "myt"];
    let corpse = e.add_ally_version(SeatId(1), find_library("Reanimated Corpse"), 1);
    hand(&mut e, 0, &["Earth Swords"]);
    hand(&mut e, 1, &["Ashes to Ashes"]);
    fight(&mut e, a, corpse);
    pick_when(&mut e, play(0, 2, a));
    let req = offering(&mut e, play(0, 0, corpse));
    assert_eq!(req.window, WindowLabel::CombatPrevention);
    e.choose(index_of(&req, play(0, 0, corpse)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(burned(&e, corpse));
}

/// Shadow Court Satyr using its stored Ashes to Ashes [thn] as a vampire
/// with 1 Thanatosis: "This vampire … goes to torpor": an ally, he burns
/// [LSJ 20060409].
#[test]
fn shadow_court_satyr_burns_using_ashes_to_ashes() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 2),
        4,
        false,
    );
    hand(&mut e, 1, &["Ashes to Ashes"]);
    let s = e.add_ally(SeatId(1), find_library("Shadow Court Satyr"));
    let req = expect(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    e.choose(index_of(
        &req,
        Choice::FindCard(find_library("Ashes to Ashes")),
    ));
    fight(&mut e, a, s);
    let req = offering(&mut e, play(0, 1, s));
    assert_eq!(req.window, WindowLabel::CombatPrevention);
    e.choose(index_of(&req, play(0, 1, s)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(burned(&e, s), "sent to torpor, an ally burns");
}

/// Increased Strength: "The additional damage is the same type as the
/// strike's damage" [RTR 19970630]: under Devil-Channel: Hands [abo] A's
/// Decapitate [pot] hand strike is aggravated, its +1 too: X's Soak [for]
/// (non-aggravated only) is never offered.
#[test]
fn increased_strength_adds_aggravated_damage_to_an_aggravated_strike() {
    let (mut e, a, x) = duel(&["pot", "abo"], &["for"]);
    hand(
        &mut e,
        0,
        &["Devil-Channel: Hands", "Increased Strength", "Decapitate"],
    );
    hand(&mut e, 1, &["Soak"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
    assert!(torpid(&e, x));
}

/// Free Fight against Amelia, The Blood Red Tears ("After combat with
/// Amelia, the opposing minion burns 1 blood or life"): "the last opposing
/// minion burns the blood" [ANK 20200420-1] — Ilse, swapped in, not Karl.
#[test]
fn free_fight_amelia_burns_the_last_opposing_minion() {
    let (mut e, k, i, _) = torrance_fight(4, &[]);
    let amelia = e.add_minion(
        SeatId(1),
        find_card("Amelia, The Blood Red Tears"),
        4,
        false,
    );
    hand(&mut e, 0, &["Free Fight"]);
    fight(&mut e, k, amelia);
    pick_when(&mut e, play(0, 0, k));
    pick(&mut e, Choice::ChooseMinion(i));
    pick_when(&mut e, Choice::ChooseMinion(i));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, k), 4, "Karl out of the combat");
    assert_eq!(
        blood(&e, i),
        6 - 1 - 1,
        "Amelia's strike and her burn on Ilse"
    );
}

/// Bliss cancelling Devil-Channel: Back [ABO] (a frenzy card granting a
/// limited additional strike): "If a limited effect is canceled …, the
/// limit is not triggered and another limited effect can be used" [LSJ
/// 20030224]: A's second Back is offered after the first is canceled.
#[test]
fn bliss_canceling_a_limited_frenzy_strike_leaves_the_limit_unused() {
    let (mut e, a, x) = duel(&["abo", "abo"], &["dom"]);
    hand(&mut e, 0, &["Devil-Channel: Back", "Devil-Channel: Back"]);
    hand(&mut e, 1, &["Bliss"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    pick_when(&mut e, play(0, 0, x));
    let req = offering(&mut e, play(0, 2, a));
    assert_eq!(req.window, WindowLabel::CombatAdditionalStrikes);
    assert!(ash_has(&e, 1, "Bliss") && ash_has(&e, 0, "Devil-Channel: Back"));
}

// ---- Round 32 combat cards, Invoking the Beast onward -----------------------

/// Pass (or hand strike) through every request, asserting none offers
/// `bad`, until one offers `want`: taken.
fn never_offered_before(e: &mut Engine, bad: Choice, want: Choice) {
    never_offered_where_before(e, |o| *o == bad, want);
}

/// `never_offered_before` for every option `bad` matches.
fn never_offered_where_before(e: &mut Engine, bad: impl Fn(&Choice) -> bool, want: Choice) {
    for _ in 0..200 {
        let req = request(e);
        assert!(
            !req.options.iter().any(&bad),
            "offered in {:?}: {:?}",
            req.window,
            req.options
        );
        if let Some(i) = req.options.iter().position(|o| *o == want) {
            e.choose(i);
            return;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    panic!("{want:?} never offered");
}

/// The card `name` in play on `host`, itself (not an effect-only stand-in).
fn on_minion(e: &Engine, name: &str, host: MinionId) -> bool {
    let card = find_library(name);
    e.state
        .cards
        .iter()
        .any(|c| c.card == card && c.in_play && !c.effect_only && c.host == Some(host))
}

/// Invoking the Beast [abo]/[ABO]: +1 strength for the remainder of combat,
/// "A vampire can play only one Invoking the Beast each combat": A's [ABO]
/// in round 1 (its press continues), the second copy not offered in round
/// 2; A's hand strikes deal 2 each round.
#[test]
fn invoking_the_beast_gives_strength_for_the_combat_once() {
    let (mut e, a, x) = duel(&["abo", "abo"], &[]);
    hand(&mut e, 0, &["Invoking the Beast", "Invoking the Beast"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    never_offered_before(&mut e, play(0, 1, a), Choice::UsePress);
    assert_eq!(blood(&e, x), 4 - 2, "round 1");
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
    assert_eq!(blood(&e, x), 0, "round 2: 2 more");
}

/// Invoking the Beast and Whistling Up the Beast [abo] [REFLEX]: "Cancel a
/// frenzy card played on this vampire as it is played" [LSJ 20051113]: X's
/// Terror Frenzy [ani] (a frenzy card on A) canceled, its cost paid [ANK
/// 20260216]: A is not restrained.
#[test]
fn invoking_the_beast_reflex_cancels_terror_frenzy_on_its_vampire() {
    for card in ["Invoking the Beast", "Whistling Up the Beast"] {
        let (mut e, a, x) = duel(&["abo"], &["ani"]);
        hand(&mut e, 0, &[card, "Invoking the Beast"]);
        hand(&mut e, 1, &["Terror Frenzy"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, 0, x));
        pick_when(&mut e, play(0, 0, a));
        let req = request(&mut e);
        assert!(req.options.contains(&play(0, 1, a)), "before range still");
        assert!(ash_has(&e, 1, "Terror Frenzy") && ash_has(&e, 0, card));
        let cost = zvtes::library::LIBRARY[find_library("Terror Frenzy") as usize].blood_cost;
        let paid = match cost {
            zvtes::library::Cost::Fixed(n) => n,
            _ => 0,
        };
        assert_eq!(blood(&e, x), 4 - paid, "the canceled card's cost paid");
        let c = e.combat().expect("combat");
        assert!(
            !c.sides[0].no_close_maneuver && !c.sides[0].no_continue_press,
            "{card}"
        );
    }
}

/// Leapfrog: [obf] "Maneuver, only usable to go to long range"; [SPI]
/// "Strike: combat ends": A's [obf] takes the range long, its [SPI] ends
/// the combat before X's hand strike.
#[test]
fn leapfrog_maneuvers_and_superior_ends_combat() {
    let (mut e, a, x) = duel(&["obf", "spi", "spi"], &[]);
    hand(&mut e, 0, &["Leapfrog", "Leapfrog"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(e.combat().expect("combat").range, Range::Long);
    pick_when(&mut e, play(0, 2, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4, 4), "no strike landed");
}

/// A duel against a capacity-8 X with 8 blood.
fn big_duel(a_levels: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity == 8, 0), 8, false);
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 3),
        4,
        false,
    );
    e.state.minions[a.index()].gained_levels = a_levels.to_vec();
    (e, a, x)
}

/// `big_duel` with a Saturday-Night Special (1R) on A.
fn gun_duel(a_levels: &[&'static str]) -> (Engine, MinionId, MinionId, CardId) {
    let (mut e, a, x) = big_duel(a_levels);
    let gun = on(&mut e, "Saturday-Night Special", a);
    (e, a, x, gun)
}

/// Manstopper Rounds: "The gun inflicts +1 damage each strike for the
/// remainder of this combat" [RTR 19990105]: A's Saturday-Night Special
/// (1R) deals 2 in round 1; A presses (Swiftness of the Stag [cel]); round
/// 2, the gun's strike deals 2 again, the ammo not played again.
#[test]
fn manstopper_rounds_add_one_damage_each_gun_strike_this_combat() {
    let (mut e, a, x, gun) = gun_duel(&["cel"]);
    hand(&mut e, 0, &["Manstopper Rounds", "Swiftness of the Stag"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 0));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, weapon(gun, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 8 - 2 - 2);
}

/// Manstopper Rounds: the extra damage "inherits the properties of the
/// base damage" [TOM 19960225]: gun damage, X's Kevlar Vest (a gun point a
/// use) prevents both points of the 1R gun's 2.
#[test]
fn manstopper_rounds_damage_is_gun_strike_damage() {
    let (mut e, a, x, gun) = gun_duel(&[]);
    let vest = on(&mut e, "Kevlar Vest", x);
    hand(&mut e, 0, &["Manstopper Rounds"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 0));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, weapon(vest, 0));
    let req = request(&mut e);
    e.choose(index_of(&req, weapon(vest, 0)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 8, "both gun points prevented");
}

/// Manstopper Rounds: "Can only be played on a gun the minion playing the
/// card is using" [LSJ 20020425]: X's .44 Magnum strike, A's ammo never
/// offered.
#[test]
fn manstopper_rounds_are_not_played_on_an_opponents_gun() {
    let (mut e, a, x) = duel(&[], &[]);
    let gun = on(&mut e, ".44 Magnum", x);
    hand(&mut e, 0, &["Manstopper Rounds"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 0));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
    assert_eq!(blood(&e, a), 4 - 2, "the Magnum's 2 alone");
}

/// Projectile [cel]: "This strike cannot be dodged": X's dodge (Vampiric
/// Speed [cel]) is played, to no effect [LSJ 20030902-2] [LSJ
/// 20060808-1]; [CEL] "with an additional strike (limited)": A's hand
/// strike after it lands too.
#[test]
fn projectile_strikes_through_a_dodge() {
    for (mode, dealt) in [(0, 1), (2, 2)] {
        let (mut e, a, x) = duel(&["cel", "cel"], &["cel"]);
        hand(&mut e, 0, &["Projectile"]);
        hand(&mut e, 1, &["Vampiric Speed"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, mode, a));
        pick_when(&mut e, play(0, 0, x));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, x), 4 - dealt, "mode {mode}");
        assert!(ash_has(&e, 1, "Vampiric Speed"));
    }
}

/// Projectile: "If the minion used a gun's maneuver, he can still use
/// Projectile to strike with that gun" [LSJ 20050304]: A's Saturday-Night
/// Special maneuver commits its strike; Projectile through the gun is
/// offered, its 1R mode not; X's dodge does nothing.
#[test]
fn projectile_fires_a_gun_committed_by_its_maneuver() {
    let (mut e, a, x, gun) = gun_duel(&["cel"]);
    e.state.minions[x.index()].gained_levels = vec!["cel"];
    hand(&mut e, 0, &["Projectile"]);
    hand(&mut e, 1, &["Vampiric Speed"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 1));
    let through = play_on(0, 1, a, Target::Card(gun));
    let req = offering(&mut e, through);
    assert!(
        !req.options.contains(&play(0, 0, a)),
        "committed to the gun"
    );
    e.choose(index_of(&req, through));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 8 - 1);
}

/// Projectile: "Cannot be used with a melee weapon" [LSJ 20050224-3]: A's
/// Baseball Bat is no target.
#[test]
fn projectile_takes_no_melee_weapon() {
    let (mut e, a, x) = duel(&["cel"], &[]);
    let bat = on(&mut e, "Baseball Bat", a);
    hand(&mut e, 0, &["Projectile"]);
    fight(&mut e, a, x);
    let req = offering(&mut e, play(0, 0, a));
    assert!(!req.options.contains(&play_on(0, 1, a, Target::Card(bat))));
}

/// Rigor Mortis [pre]: "The opposing minion cannot use any additional
/// strikes this round" — no card providing one is played [LSJ
/// 20011214-5]: X's Acrobatics [cel] (dodge, with an additional strike)
/// not offered in round 1; A's [thn] press continues, offered in round 2.
#[test]
fn rigor_mortis_bars_the_opponents_additional_strikes_this_round() {
    let (mut e, a, x) = duel(&["thn"], &["cel", "cel"]);
    hand(&mut e, 0, &["Rigor Mortis"]);
    hand(&mut e, 1, &["Acrobatics"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    never_offered_before(&mut e, play(0, 1, x), Choice::UsePress);
    let req = offering(&mut e, play(0, 1, x));
    assert_eq!(req.window, WindowLabel::CombatStrike);
}

/// Rigor Mortis [THN]: "once each round this combat, you may cancel a
/// maneuver used by the opposing minion": X's Leapfrog [spi] canceled, the
/// range close again; X maneuvers again with a second Leapfrog (rules-qa
/// Batch 124), no second cancel that round: long range.
#[test]
fn rigor_mortis_cancel_leaves_the_maneuvering_minion_free_to_maneuver_again() {
    let (mut e, a, x) = duel(&["thn", "thn"], &["spi"]);
    hand(&mut e, 0, &["Rigor Mortis"]);
    hand(&mut e, 1, &["Leapfrog", "Leapfrog"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    pick_when(&mut e, play(0, 1, x));
    let req = offering(&mut e, Choice::ChooseMinion(x));
    assert_eq!(req.seat, SeatId(0));
    assert_eq!(req.options, vec![Choice::Pass, Choice::ChooseMinion(x)]);
    e.choose(index_of(&req, Choice::ChooseMinion(x)));
    assert_eq!(e.combat().expect("combat").range, Range::Close);
    pick_when(&mut e, play(0, 1, x));
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::ChooseMinion(x)));
    assert_eq!(e.combat().expect("combat").range, Range::Long);
}

/// Rigor Mortis [THN], two copies: "once each round this combat" is each
/// copy's (D7, per copy): A's second copy, played in round 2 after a
/// press, gives a second cancel that round; X's two Leapfrogs [spi] both
/// canceled.
#[test]
fn rigor_mortis_two_superior_copies_cancel_twice_a_round() {
    let (mut e, a, x) = duel(&["thn", "thn"], &["spi"]);
    hand(&mut e, 0, &["Rigor Mortis", "Rigor Mortis"]);
    hand(&mut e, 1, &["Leapfrog", "Leapfrog"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    pick_when(&mut e, Choice::UsePress);
    pick_when(&mut e, play(0, 2, a));
    assert_eq!(e.combat().expect("combat").round, 2);
    for _ in 0..2 {
        pick_when(&mut e, play(0, 1, x));
        let req = offering(&mut e, Choice::ChooseMinion(x));
        assert_eq!(req.seat, SeatId(0));
        e.choose(index_of(&req, Choice::ChooseMinion(x)));
        assert_eq!(e.combat().expect("combat").range, Range::Close);
    }
}

/// Rigor Mortis [THN] canceling a gun's maneuver: "the strike is still
/// committed" [LSJ 20050228-3] [ANK 20230111]: X's .44 Magnum maneuver
/// canceled at close range, X's strike is still the gun's (2 on A, no
/// hand strike); Aid from Bats [ani]'s likewise.
#[test]
fn rigor_mortis_cancel_keeps_a_gun_maneuvers_strike_committed() {
    let (mut e, a, x) = duel(&["thn", "thn"], &["ani"]);
    let gun = on(&mut e, ".44 Magnum", x);
    hand(&mut e, 0, &["Rigor Mortis"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    pick_when(&mut e, weapon(gun, 1));
    pick_when(&mut e, Choice::ChooseMinion(x));
    assert_eq!(e.combat().expect("combat").range, Range::Close);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, a),
        4 - 1 - 2,
        "the card's blood and the Magnum's 2"
    );

    let (mut e, a, x) = duel(&["thn", "thn"], &["ani"]);
    hand(&mut e, 0, &["Rigor Mortis"]);
    hand(&mut e, 1, &["Aid from Bats"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, Choice::ChooseMinion(x));
    let c = e.combat().expect("combat");
    assert_eq!(c.range, Range::Close);
    assert!(c.sides[1].committed_card.is_some(), "still Aid from Bats'");
}

/// Rigor Mortis [THN] canceling a maneuver that provides a press: "the
/// press is also canceled" [ANK 20230114]: X's Aid from Bats [ANI]
/// (maneuver, with an optional press this round) canceled, no press left.
#[test]
fn rigor_mortis_cancel_takes_the_maneuvers_press() {
    let (mut e, a, x) = duel(&["thn", "thn"], &["ani", "ani"]);
    hand(&mut e, 0, &["Rigor Mortis"]);
    hand(&mut e, 1, &["Aid from Bats"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    pick_when(&mut e, play(0, 1, x));
    request(&mut e);
    assert_eq!(e.combat().expect("combat").sides[1].presses, 1);
    pick_when(&mut e, Choice::ChooseMinion(x));
    assert_eq!(e.combat().expect("combat").sides[1].presses, 0);
}

/// Riposte: "Not usable on the first round of combat"; [CEL] "Strike:
/// combat ends, and inflict 2 damage to the opposing minion once combat
/// ends if the range is close (damage not preventable)" [LSJ 20100204-2]
/// [LSJ 20031123]: A presses (Swiftness of the Stag [cel]); in round 2
/// Riposte ends the combat, X takes 2 after it, X's Soak [for] not
/// offered. Dodged, none [LSJ 19980526] [RTR 20041202]; at long range
/// (A's Leapfrog [obf]), none [LSJ 19970915].
#[test]
fn riposte_damages_after_combat_at_close_range_unless_dodged() {
    for case in ["close", "dodged", "long"] {
        let (mut e, a, x) = duel(&["cel", "cel", "obf"], &["for", "cel"]);
        hand(&mut e, 0, &["Riposte", "Swiftness of the Stag", "Leapfrog"]);
        hand(&mut e, 1, &["Soak", "Vampiric Speed"]);
        fight(&mut e, a, x);
        never_offered_before(&mut e, play(0, 1, a), play(1, 0, a));
        if case == "long" {
            pick_when(&mut e, play(1, 0, a));
        }
        pick_when(&mut e, play(0, 1, a));
        if case == "dodged" {
            pick_when(&mut e, play(1, 0, x));
        }
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
        let left = 4 - 1 - if case == "close" { 2 } else { 0 };
        assert_eq!(blood(&e, x), left, "{case}");
        assert_eq!(blood(&e, a), 4 - 1 - 1, "round 1's strike and the card");
    }
}

/// Riposte: "Is a damage dealing effect and can be modified by other
/// effects like Dam the Heart's River" [ANK 20170111] [PIB 20130319]: and
/// Target Vitals' aim: X, 8 blood, takes Riposte [cel]'s 1 + 1 + 2 after
/// the combat.
#[test]
fn riposte_damage_takes_target_vitals_and_dam_the_hearts_river() {
    let (mut e, a, x) = big_duel(&["cel", "qui"]);
    hand(
        &mut e,
        0,
        &[
            "Riposte",
            "Swiftness of the Stag",
            "Dam the Heart's River",
            "Target Vitals",
        ],
    );
    fight(&mut e, a, x);
    pick_when(&mut e, play(1, 0, a));
    pick_when(&mut e, play(1, 0, a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 8 - 1 - (1 + 1 + 2));
}

/// Riposte: "If the combat continues …, the after combat effect does not
/// happen" [RTR 20020501] [LSJ 19980109]: X's Telepathic Tracking [AUS]
/// starts a new round, Riposte's damage lost.
#[test]
fn riposte_does_nothing_if_the_combat_continues() {
    let (mut e, a, x) = duel(&["cel"], &["aus", "aus"]);
    hand(&mut e, 0, &["Riposte", "Swiftness of the Stag"]);
    hand(&mut e, 1, &["Telepathic Tracking"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(1, 0, a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, x),
        4 - 1 - 1 - 1,
        "rounds 1 and 3's hand strikes and the card's blood only"
    );
}

/// Stone Quills [vis]: "Strike: 2R damage, with an optional maneuver": A
/// plays it at the maneuver (long range), its strike committed: X takes 2,
/// X's hand strike misses.
#[test]
fn stone_quills_maneuvers_and_commits_its_ranged_strike() {
    let (mut e, a, x) = duel(&["vis"], &[]);
    hand(&mut e, 0, &["Stone Quills"]);
    fight(&mut e, a, x);
    let req = offering(&mut e, play(0, 1, a));
    assert_eq!(req.window, WindowLabel::CombatDetermineRange);
    e.choose(index_of(&req, play(0, 1, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2);
    assert_eq!(blood(&e, a), 4 - 1, "the card's blood, no strike on A");
}

/// Strength of the Bear: [ani] "+1 strength this round", [SPI] "for the
/// remainder of combat, with an optional press this round": both in round
/// 1 (3 damage), round 2 (2).
#[test]
fn strength_of_the_bear_adds_strength_for_the_round_or_combat() {
    let (mut e, a, x) = big_duel(&["ani", "spi", "spi"]);
    hand(&mut e, 0, &["Strength of the Bear", "Strength of the Bear"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 2, a));
    pick_when(&mut e, Choice::UsePress);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 8 - 3 - 2);
}

/// "This vampire" applies to an ally playing the card as a vampire (rules-qa
/// 121.179): Tunnel Runner ("may play cards that require Animalism as a
/// vampire") strikes for 2 with Strength of the Bear [ani].
#[test]
fn an_ally_playing_strength_of_the_bear_as_a_vampire_gets_its_strength() {
    let (mut e, _, x) = duel(&[], &[]);
    let runner = e.add_ally(SeatId(0), find_library("Tunnel Runner"));
    hand(&mut e, 0, &["Strength of the Bear"]);
    pick(&mut e, bleed(runner));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, runner));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2, "1 strength + 1");
}

/// Swiftness of the Stag: [SPI] "[ACTION MODIFIER] +1 stealth": A's bleed
/// gets past X's block attempt; [spi] "[COMBAT] Maneuver or press": A goes
/// to long range.
#[test]
fn swiftness_of_the_stag_presses_maneuvers_or_hides() {
    let (mut e, a, x) = duel(&["spi", "spi"], &[]);
    hand(&mut e, 0, &["Swiftness of the Stag"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 2, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 1, "unblocked");

    let (mut e, a, x) = duel(&["spi"], &[]);
    hand(&mut e, 0, &["Swiftness of the Stag"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        (blood(&e, a), blood(&e, x)),
        (4, 4),
        "no hand strike at long"
    );
}

/// Swoop: "Maneuver, only usable to go to close range, and this minion
/// gets +1 strength on his or her initial strike this round": X's Leapfrog
/// [spi] takes the range long, A swoops back: its initial hand strike
/// deals 2, its additional one (Torrent [cel]) 1.
#[test]
fn swoop_closes_and_strengthens_the_initial_strike_only() {
    let (mut e, _, x) = big_duel(&["cel"]);
    let flier = fixture_vampire(|c| c.disciplines.contains(&"FLIGHT") && c.capacity >= 3, 0);
    let a = e.add_minion(SeatId(0), flier, 4, false);
    e.state.minions[a.index()].gained_levels = vec!["cel"];
    e.state.minions[x.index()].gained_levels = vec!["spi"];
    hand(&mut e, 0, &["Swoop", "Torrent"]);
    hand(&mut e, 1, &["Leapfrog"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 8 - 2 - 1);
}

/// Swoop (SW-2): "only usable to go to close range": at close range, never
/// offered.
#[test]
fn swoop_is_not_played_at_close_range() {
    let (mut e, _, x) = big_duel(&[]);
    let flier = fixture_vampire(|c| c.disciplines.contains(&"FLIGHT") && c.capacity >= 3, 0);
    let a = e.add_minion(SeatId(0), flier, 4, false);
    hand(&mut e, 0, &["Swoop"]);
    fight(&mut e, a, x);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        *o == play(0, 0, a)
    });
}

/// "The opposing vampire" names no ally (rules-qa 70.16, D14's exception):
/// against a blocking ally, Whistling Up the Beast (WU-2) and Infection
/// [THN] (IN-7) are never offered, though A's hand strike lands.
#[test]
fn whistling_up_the_beast_and_infection_are_not_played_against_an_ally() {
    let (mut e, a, _) = duel(&["abo", "abo", "thn", "thn"], &[]);
    let ally = e.add_ally(SeatId(1), find_library("Renegade Garou"));
    hand(&mut e, 0, &["Whistling Up the Beast", "Infection"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(ally));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(*o, Choice::Play { card: 0, actor: Some(m), .. } if m == a) || *o == play(1, 5, a)
    });
    assert!(e.state.minions[ally.index()].blood < 3, "struck");
}

/// Withering [thn] at long range (WI-5): its hand strike does nothing
/// there (D23), so the card is placed on no one.
#[test]
fn withering_at_long_range_places_nothing() {
    let (mut e, a, x) = duel(&["thn", "spi"], &[]);
    hand(&mut e, 0, &["Leapfrog", "Withering"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!on_minion(&e, "Withering", x));
    assert!(ash_has(&e, 0, "Withering"));
    assert_eq!(blood(&e, x), 4);
}

/// Toreador's Bane: "Only usable in combat with a Toreador, Toreador
/// antitribu, ally or younger vampire": the younger X blocking the older A
/// cannot play it; the younger blocker of a Toreador can, and an elder
/// against a younger vampire.
#[test]
fn toreadors_bane_needs_a_toreador_ally_or_younger_foe() {
    let (mut e, a, x) = elder_duel(&[], &["pre"]);
    hand(&mut e, 1, &["Toreador's Bane"]);
    fight(&mut e, a, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));

    let mut e = game(2);
    let t = e.add_minion(
        SeatId(0),
        fixture_vampire(|c| c.clan == "Toreador" && c.capacity >= 9, 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    let y = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 4, false);
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 3),
        4,
        false,
    );
    e.state.minions[y.index()].gained_levels = vec!["pre"];
    hand(&mut e, 1, &["Toreador's Bane"]);
    fight(&mut e, t, y);
    let req = offering(&mut e, play(0, 0, y));
    assert_eq!(req.window, WindowLabel::CombatStrike);

    let (mut e, a, x) = elder_duel(&["pre"], &[]);
    hand(&mut e, 0, &["Toreador's Bane"]);
    fight(&mut e, a, x);
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.window, WindowLabel::CombatStrike);
}

/// Toreador's Bane [MEL]: "As [mel] above, but usable to continue a bleed":
/// A's blocked bleed goes on; [mel]'s does not.
#[test]
fn toreadors_bane_superior_continues_a_blocked_bleed() {
    for (mode, bled) in [(1, 0), (2, 1)] {
        let (mut e, a, x) = elder_duel(&["mel", "mel"], &[]);
        hand(&mut e, 0, &["Toreador's Bane"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, mode, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e)[1], 10 - bled, "mode {mode}");
        assert_eq!(blood(&e, x), 4, "no strike landed");
    }
}

/// Vampiric Speed [CEL]: "Strike: dodge, with an optional press": X dodges
/// A's hand strike and presses to continue.
#[test]
fn vampiric_speed_dodges_and_superior_presses() {
    let (mut e, a, x) = duel(&[], &["cel", "cel"]);
    hand(&mut e, 1, &["Vampiric Speed"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    let req = offering(&mut e, Choice::UsePress);
    assert_eq!(req.seat, SeatId(1));
    assert_eq!(blood(&e, x), 4, "dodged");
    e.choose(index_of(&req, Choice::UsePress));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        (blood(&e, a), blood(&e, x)),
        (4 - 1, 4 - 1),
        "round 2's strikes"
    );
}

/// Whistling Up the Beast [ABO]: "During this combat, the opposing vampire
/// cannot use maneuvers to maneuver to close range and cannot use presses
/// to continue combat … and this vampire gets an optional maneuver this
/// round": A goes long; X's Leapfrog [spi] and .44 Magnum maneuver never
/// offered, X's press (Swiftness of the Stag [cel]) neither; X's gun
/// strikes (equipment usable).
#[test]
fn whistling_up_the_beast_keeps_the_opponent_long_and_unpressed() {
    let (mut e, a, x) = duel(&["abo", "abo"], &["spi", "cel"]);
    let gun = on(&mut e, ".44 Magnum", x);
    hand(&mut e, 0, &["Whistling Up the Beast"]);
    hand(&mut e, 1, &["Leapfrog", "Swiftness of the Stag"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    pick_when(&mut e, Choice::UseManeuver);
    let barred = |o: &Choice| {
        *o == play(0, 1, x) || *o == weapon(gun, 1) || *o == play(1, 0, x) || *o == play(1, 1, x)
    };
    never_offered_where_before(&mut e, barred, weapon(gun, 0));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, barred);
    assert_eq!(blood(&e, a), 4 - 2, "X's gun at long range");
    assert_eq!(blood(&e, x), 4);
}

/// Tranquility Shield lifts a frenzy card's effects on its vampire
/// (rules-qa 70.16): after Whistling Up the Beast [abo], X presses to
/// continue (Swiftness of the Stag [cel]). Blight [NEC]'s bar on presses
/// to continue is no frenzy effect: it stands (fixed: the shield lifted
/// it).
#[test]
fn tranquility_shield_lifts_whistling_up_the_beast_not_blight() {
    for (card, levels, lifted) in [
        ("Whistling Up the Beast", &["abo"][..], true),
        ("Blight", &["nec", "nec"][..], false),
    ] {
        let (mut e, a, _) = duel(levels, &[]);
        let x = e.add_minion(
            SeatId(1),
            fixture_vampire(|c| c.clan == "Salubri" && (4..=8).contains(&c.capacity), 0),
            4,
            false,
        );
        e.state.minions[x.index()].gained_levels = vec!["for", "cel"];
        hand(&mut e, 0, &[card]);
        hand(&mut e, 1, &["Tranquility Shield", "Swiftness of the Stag"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, 1, a));
        pick_when(&mut e, play(0, 0, x));
        let offered = offered_before(&mut e, play(0, 0, x), SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(offered, lifted, "{card}");
    }
}

/// Withering [thn]: "Strike: hand strike. Place this card on the opposing
/// minion. The minion with this card has -1 strength" — not on the strike
/// resolving with it [LSJ 20020211]: X's hand strike deals 1 that round;
/// the card on X (itself), burned in its controller's next discard phase.
#[test]
fn withering_weakens_the_opposing_minion_until_its_discard_phase() {
    let (mut e, a, x) = duel(&["thn"], &[]);
    hand(&mut e, 0, &["Withering"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1, "X's strike at full strength");
    assert_eq!(blood(&e, x), 4 - 1);
    assert!(on_minion(&e, "Withering", x) && !ash_has(&e, 0, "Withering"));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!on_minion(&e, "Withering", x) && ash_has(&e, 0, "Withering"));
}

/// Withering [THN]: "the minion with this card cannot play cards that
/// require any Disciplines" (rules-digest D29): landed in round 1, X's
/// press card (Swiftness of the Stag [cel]) is not offered in its press
/// step; after [thn]'s, it is.
#[test]
fn withering_superior_bars_discipline_cards() {
    for (mode, barred) in [(1, false), (2, true)] {
        let (mut e, a, x) = duel(&["thn", "thn"], &["cel"]);
        hand(&mut e, 0, &["Withering"]);
        hand(&mut e, 1, &["Swiftness of the Stag"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, mode, a));
        let offered = offered_before(&mut e, play(0, 0, x), SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(offered, !barred, "mode {mode}");
    }
}

/// Seat 0's A (with `a_levels`) and a second vampire B; seat 1's ally
/// `ally` and its vampire V (`v_levels`), unlocked out of the combat. A
/// bleeds, the ally blocks.
fn ally_blocks(
    a_levels: &[&'static str],
    ally: &str,
    v_levels: &[&'static str],
) -> (Engine, MinionId, MinionId, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    let b = e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    let p = e.add_ally(SeatId(1), find_library(ally));
    let v = e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 2),
        4,
        false,
    );
    e.state.minions[a.index()].gained_levels = a_levels.to_vec();
    e.state.minions[v.index()].gained_levels = v_levels.to_vec();
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(p));
    (e, a, b, p, v)
}

/// Heaven's Gate [nec]: "Prevent 1 damage to an ally in combat", by "a
/// ready unlocked vampire not involved in the combat" — not without
/// damage pending [LSJ 20001114]: V saves its Political Ally (1 life) from
/// A's hand strike; locked, V is not offered it.
#[test]
fn heavens_gate_prevents_one_damage_to_an_ally_from_outside_the_combat() {
    let (mut e, _, _, p, v) = ally_blocks(&[], "Political Ally", &["nec"]);
    hand(&mut e, 1, &["Heaven's Gate"]);
    let gate = play_on(0, 0, v, Target::Minion(p));
    let req = offering(&mut e, gate);
    assert_eq!(req.window, WindowLabel::CombatPrevention);
    e.choose(index_of(&req, gate));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[p.index()].in_play);
    assert_eq!(blood(&e, v), 4 - 1, "the card's blood");

    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    let p = e.add_ally(SeatId(1), find_library("Political Ally"));
    let v = e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 2),
        4,
        false,
    );
    e.state.minions[v.index()].gained_levels = vec!["nec"];
    e.state.minions[v.index()].locked = true;
    hand(&mut e, 1, &["Heaven's Gate"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(p));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play_on(0, 0, v, Target::Minion(p)),
    );
    assert!(!e.state.minions[p.index()].in_play);
}

/// Heaven's Gate [obe]: "Only usable if an ally would be burned in combat.
/// Instead, they gain 1 life if they have no life, and they cannot enter
/// (or be involved in) combat, perform actions, or block this turn; end
/// combat": the Political Ally stays, 1 life, the combat over; this turn
/// B's Bum's Rush cannot target it [ANK 20220429], nor A's Psyche! [cel]
/// begin a new combat with it, nor can it block B's bleed; seat 1's turn,
/// it acts.
#[test]
fn heavens_gate_saves_a_burning_ally_and_sidelines_it_for_the_turn() {
    let (mut e, a, b, p, v) = ally_blocks(&["cel"], "Political Ally", &["obe"]);
    hand(&mut e, 0, &["Psyche!", "Bum's Rush"]);
    hand(&mut e, 1, &["Heaven's Gate"]);
    let gate = play_on(0, 1, v, Target::Minion(p));
    never_offered_before(&mut e, play(0, 0, a), gate);
    let req = request(&mut e);
    assert!(e.combat().is_none(), "combat ended");
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0, .. })),
        "no Psyche!"
    );
    assert_eq!(blood(&e, p), 1);
    assert!(e.state.minions[p.index()].in_play && e.state.minions[p.index()].recruited);
    assert!(
        !req.options.contains(&play_on(1, 0, b, Target::Minion(p))),
        "no Bum's Rush"
    );
    e.choose(index_of(&req, bleed(b)));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, block(p));
    let req = request(&mut e);
    assert!(req
        .options
        .iter()
        .any(|o| matches!(o, Choice::TakeAction { minion, .. } if *minion == p)));
}

/// Heaven's Gate [OBE]: "add 2 life if they have no life, not to exceed
/// their starting life": the Political Ally (1 life) gets 1, Saatet-ta (2)
/// gets 2.
#[test]
fn heavens_gate_superior_adds_two_life_capped_at_its_starting_life() {
    for (ally, life) in [("Political Ally", 1), ("Saatet-ta", 2)] {
        let mut e = game(2);
        let a = e.add_minion(
            SeatId(0),
            textless(|c| (4..=8).contains(&c.capacity), 0),
            4,
            false,
        );
        e.add_minion(
            SeatId(0),
            textless(|c| (4..=8).contains(&c.capacity), 1),
            4,
            false,
        );
        let p = e.add_ally(SeatId(1), find_library(ally));
        e.state.minions[p.index()].blood = 1;
        let v = e.add_minion(
            SeatId(1),
            textless(|c| (4..=8).contains(&c.capacity), 2),
            4,
            false,
        );
        e.state.minions[v.index()].gained_levels = vec!["obe", "obe"];
        hand(&mut e, 1, &["Heaven's Gate"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(p));
        pick_when(&mut e, play_on(0, 2, v, Target::Minion(p)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, p), life, "{ally}");
    }
}

/// Heaven's Gate [obe] "can be played before another replacement effect
/// triggers (eg. Charigger, The Axe)" [ANK 20230816]; Charigger's "other
/// replacements effect can be played before" [LSJ 20071003-2] [ANK
/// 20230816]:
/// seat 0's Giovanni with Charigger kills its prey's Political Ally; seat
/// 1 saves it: no soul counter, the ally in play.
#[test]
fn heavens_gate_saves_the_preys_ally_before_charigger() {
    let mut e = game(2);
    let g = e.add_minion(
        SeatId(0),
        textless(
            |c| {
                c.clan == "Giovanni"
                    && c.capacity >= 4
                    && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("nec"))
            },
            0,
        ),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    let charigger = on(&mut e, "Charigger, The Axe", g);
    let p = e.add_ally(SeatId(1), find_library("Political Ally"));
    let v = e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 2),
        4,
        false,
    );
    e.state.minions[v.index()].gained_levels = vec!["obe"];
    hand(&mut e, 1, &["Heaven's Gate"]);
    pick(&mut e, bleed(g));
    pick_when(&mut e, block(p));
    pick_when(&mut e, play_on(0, 1, v, Target::Minion(p)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[p.index()].in_play, "saved");
    assert_eq!(e.state.cards[charigger.index()].counters, 0);
}

/// Escaped Mental Patient: "If his strike resolves, he cannot be saved from
/// burning (with eg. Heaven's Gate …)" [LSJ 20070507] [LSJ 20021205]: he
/// burns as the combat ends, Heaven's Gate never offered.
#[test]
fn escaped_mental_patient_cannot_be_saved_by_heavens_gate() {
    let (mut e, _, _, p, v) = ally_blocks(&[], "Escaped Mental Patient", &["obe"]);
    e.state.minions[p.index()].blood = 3;
    hand(&mut e, 1, &["Heaven's Gate"]);
    let card = e.state.minions[p.index()].ally_id().expect("an ally");
    pick_when(&mut e, weapon(card, 0));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play_on(0, 1, v, Target::Minion(p)),
    );
    assert!(!e.state.minions[p.index()].in_play);
}

/// Infection [for]: "Prevent all damage from the opponent's strikes this
/// round" [LSJ 20001114]: A's hand strike and its additional one
/// (Torrent [cel]) prevented; X pays the card's 2 blood alone.
#[test]
fn infection_fortitude_prevents_all_strike_damage_this_round() {
    let (mut e, a, x) = duel(&["cel"], &["for"]);
    hand(&mut e, 0, &["Torrent"]);
    hand(&mut e, 1, &["Infection"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2);
}

/// Infection [thn]: "Prevent all damage to a ghoul (ally or retainer) in
/// combat. Usable by a vampire not involved in the combat": seat 1's
/// torpid V saves its Procurer (ghoul ally) from A's hand strike; a
/// Political Ally (mortal) is no target.
#[test]
fn infection_thaumaturgy_saves_a_ghoul_from_outside_the_combat() {
    for (ally, ghoul) in [("Procurer", true), ("Political Ally", false)] {
        let mut e = game(2);
        let a = e.add_minion(
            SeatId(0),
            textless(|c| (4..=8).contains(&c.capacity), 0),
            4,
            false,
        );
        e.add_minion(
            SeatId(0),
            textless(|c| (4..=8).contains(&c.capacity), 1),
            4,
            false,
        );
        let p = e.add_ally(SeatId(1), find_library(ally));
        let v = e.add_minion(
            SeatId(1),
            textless(|c| (4..=8).contains(&c.capacity), 2),
            4,
            true,
        );
        e.state.minions[v.index()].gained_levels = vec!["thn"];
        hand(&mut e, 1, &["Infection"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(p));
        let save = play_on(0, 3, v, Target::Minion(p));
        assert_eq!(
            offered_before(&mut e, save, SeatId(0), WindowLabel::MinionPhase),
            ghoul,
            "{ally}"
        );
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[p.index()].in_play, ghoul, "{ally}");
    }
}

/// Infection [THN]: "Only usable at the end of a round in which this
/// vampire successfully inflicted damage on the opposing vampire. Put this
/// card on the opposing vampire. The vampire with this card cannot block
/// the vampire playing this card" [RTR 20030519]: on X (itself, not an
/// effect-only stand-in); a turn later X cannot block A's bleed, can
/// block B's. A dodged strike (Vampiric Speed [cel]): never offered.
#[test]
fn infection_superior_bars_its_victim_from_blocking_its_player() {
    let (mut e, a, x) = duel(&["thn", "thn"], &[]);
    let b = e.state.minions_of(SeatId(0)).find(|&m| m != a).expect("B");
    hand(&mut e, 0, &["Infection"]);
    fight(&mut e, a, x);
    let req = offering(&mut e, play(0, 5, a));
    assert_eq!(req.window, WindowLabel::CombatEndOfRound);
    e.choose(index_of(&req, play(0, 5, a)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(on_minion(&e, "Infection", x) && !ash_has(&e, 0, "Infection"));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(!req.options.contains(&block(x)));
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(b));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(x)));

    let (mut e, a, x) = duel(&["thn", "thn"], &["cel"]);
    hand(&mut e, 0, &["Infection"]);
    hand(&mut e, 1, &["Vampiric Speed"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 5, a));
}

/// The card `name` on `host` as `by` played it in its first mode, its
/// layers pushed.
fn played_on(e: &mut Engine, name: &str, host: MinionId, by: MinionId) -> CardId {
    let owner = e.state.minions[host.index()].owner;
    let id = in_play(e, owner.0, name, Some(host));
    e.state.cards[id.index()].by = Some(by);
    id
}

/// A Salubri antitribu A and a second vampire of seat 0;
/// seat 1's X of `clan` (capacity 4 to 8) and a second vampire.
fn salubri_duel(clan: &str) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        fixture_vampire(|c| c.clan == "Salubri antitribu" && c.capacity >= 4, 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    let x = e.add_minion(
        SeatId(1),
        fixture_vampire(|c| c.clan == clan && (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 3),
        4,
        false,
    );
    (e, a, x)
}

/// Tracker's Mark: "Play when the opposing minion burns blood or life due
/// to damage. Put this card on the opposing minion. … A vampire can play
/// only one Tracker's Mark each combat": offered once A's hand strike
/// landed, not before; on X itself; the second copy not offered.
#[test]
fn trackers_mark_marks_a_minion_that_burned_blood_to_damage() {
    let (mut e, a, x) = salubri_duel("Ventrue");
    hand(&mut e, 0, &["Tracker's Mark", "Tracker's Mark"]);
    fight(&mut e, a, x);
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(
        req.window,
        WindowLabel::CombatAdditionalStrikes,
        "after the damage"
    );
    assert_eq!(blood(&e, x), 4 - 1);
    e.choose(index_of(&req, play(0, 0, a)));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
    assert!(on_minion(&e, "Tracker's Mark", x) && !ash_has(&e, 0, "Tracker's Mark"));
}

/// Tracker's Mark, "Play when the opposing minion burns blood or life due
/// to damage": in the window right after that damage resolution only
/// (owner, rules-qa 124.235): Weather Control's damage as the before-range
/// step ends, X burns 1: offered at range determination; passed there, not
/// in the before-strikes window that follows, again after A's hand strike
/// lands.
#[test]
fn trackers_mark_is_offered_right_after_the_damage_only() {
    let (mut e, a, x) = salubri_duel("Ventrue");
    e.state.minions[a.index()].gained_levels = vec!["tha"];
    hand(&mut e, 0, &["Weather Control", "Tracker's Mark"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.window, WindowLabel::CombatDetermineRange);
    assert_eq!(blood(&e, x), 4 - 1);
    e.choose(index_of(&req, Choice::Pass));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.window, WindowLabel::CombatAdditionalStrikes);
}

/// Tracker's Mark after the press step's damage, Crawling Chamber [VIS]'s
/// (owner, rules-qa 124.235: "a window of its own"): passed after A's
/// hand strike, offered again as X burns the press step's 1R, though the
/// press step has no untimed window; passed there, never later; taken,
/// on X.
#[test]
fn trackers_mark_is_offered_after_press_step_damage() {
    for take in [false, true] {
        let (mut e, a, x) = salubri_duel("Ventrue");
        e.state.minions[a.index()].gained_levels = vec!["vis", "vis"];
        hand(&mut e, 0, &["Crawling Chamber", "Tracker's Mark"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, 2, a));
        let req = offering(&mut e, play(0, 0, a));
        assert_eq!(req.window, WindowLabel::CombatAdditionalStrikes);
        e.choose(index_of(&req, Choice::Pass));
        let req = offering(&mut e, play(0, 0, a));
        assert_eq!(req.window, WindowLabel::CombatPress, "right after it");
        assert_eq!(blood(&e, x), 4 - 2);
        if take {
            e.choose(index_of(&req, play(0, 0, a)));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(on_minion(&e, "Tracker's Mark", x));
        } else {
            e.choose(index_of(&req, Choice::Pass));
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
        }
    }
}

/// Tracker's Mark "requires a Salubri antitribu" for its entry action [LSJ
/// 20080604]: A may enter combat with the Tremere it marked, not with a
/// Ventrue; A gets +1 intercept blocking the marked X (X's +1 stealth
/// bleed blocked), and X may burn the card as a +1 stealth action.
#[test]
fn trackers_mark_gives_its_salubri_intercept_and_a_rush_at_a_tremere() {
    for (clan, rush) in [("Tremere", true), ("Ventrue", false)] {
        let (mut e, a, x) = salubri_duel(clan);
        let mark = played_on(&mut e, "Tracker's Mark", x, a);
        let enter = Choice::UseAbility {
            source: AbilitySource::Card(mark),
            ability: 0,
            actor: Some(a),
            target: Target::Minion(x),
        };
        let req = request(&mut e);
        assert_eq!(req.options.contains(&enter), rush, "{clan}");
    }
    let (mut e, a, x) = salubri_duel("Ventrue");
    played_on(&mut e, "Tracker's Mark", x, a);
    e.state.minions[x.index()].gained_levels = vec!["spi", "spi"];
    hand(&mut e, 1, &["Swiftness of the Stag"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(x));
    pick_when(&mut e, block(a));
    pick_when(&mut e, play(0, 2, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[0], 10, "blocked at 1 against 1");
}

/// Tracker's Mark: "The minion with this card may burn it as a +1 stealth
/// action": X's action, the card to its owner's ash heap.
#[test]
fn trackers_mark_bearer_burns_it_as_an_action() {
    let (mut e, a, x) = salubri_duel("Ventrue");
    let mark = played_on(&mut e, "Tracker's Mark", x, a);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(mark),
            ability: 1,
            actor: Some(x),
            target: Target::None,
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[mark.index()].in_play);
    assert!(ash_has(&e, 0, "Tracker's Mark") || ash_has(&e, 1, "Tracker's Mark"));
}

/// Vitae Block [OBE]: "Put this card in play and move up to 3 blood from
/// the opposing vampire to this card. During your unlock phase, move all
/// the blood from this card to the opposing vampire and burn this card":
/// X (8 blood) keeps 5, the card (itself, in play) holds 3 past the
/// combat; seat 0's next unlock phase, X has them back, the card in the
/// ash heap.
#[test]
fn vitae_block_holds_the_opposing_blood_until_the_players_unlock_phase() {
    let (mut e, a, x) = big_duel(&["obe", "obe"]);
    hand(&mut e, 0, &["Vitae Block"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = find_library("Vitae Block");
    let held = |e: &Engine| {
        e.state
            .cards
            .iter()
            .find(|c| c.card == card && c.in_play && !c.effect_only)
            .map(|c| c.counters)
    };
    assert_eq!(held(&e), Some(3));
    assert_eq!(blood(&e, x), 8 - 3 - 1, "3 moved, A's hand strike");
    assert!(!ash_has(&e, 0, "Vitae Block"));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(held(&e), None);
    assert!(ash_has(&e, 0, "Vitae Block"));
    assert_eq!(blood(&e, x), 8 - 1);
}

/// Vitae Block [ani]: "This vampire gets 1 optional press this round"; "A
/// vampire can play only one Vitae Block each combat": the press
/// continues, the second copy never offered in round 2.
#[test]
fn vitae_block_basic_presses_once_per_combat() {
    let (mut e, a, x) = duel(&["ani"], &[]);
    hand(&mut e, 0, &["Vitae Block", "Vitae Block"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::UsePress);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
    assert_eq!(blood(&e, x), 4 - 2, "two rounds of strikes");
}

/// A flier A of seat 0 (`blood`), a second vampire; seat 1's X (with
/// `x_levels`) and a second one; A's hand Roll, `more`, Forms of Mist to 7;
/// 3 cards in A's library. A bleeds (its blood set then), X blocks.
fn roll_fight(blood: u8, more: &[&str], x_levels: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let flier = fixture_vampire(|c| c.disciplines.contains(&"FLIGHT") && c.capacity >= 3, 0);
    let a = e.add_minion(SeatId(0), flier, 3, false);
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    let x = e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 2),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 3),
        4,
        false,
    );
    e.state.minions[x.index()].gained_levels = x_levels.to_vec();
    let mut cards = vec!["Roll"];
    cards.extend(more);
    cards.extend(vec!["Form of Mist"; 7 - cards.len()]);
    hand(&mut e, 0, &cards);
    e.state.seats[0]
        .library
        .extend([find_library("Deflection"); 3]);
    pick(&mut e, bleed(a));
    e.state.minions[a.index()].blood = blood;
    pick_when(&mut e, block(x));
    (e, a, x)
}

/// Roll: "Do not replace until after combat. [FLIGHT] Maneuver. A minion
/// may play only one Roll each round. If this minion is still ready when
/// you would draw to replace this card, you may move this card from your
/// ash heap to your hand instead": A rolls to long range; after X's
/// Leapfrog [spi] its second Roll is not offered that round; after the
/// combat A takes the first back.
#[test]
fn roll_maneuvers_once_a_round_and_returns_to_hand_after_combat() {
    let roll = find_library("Roll");
    let (mut e, a, x) = roll_fight(3, &["Roll"], &["spi"]);
    hand(&mut e, 1, &["Leapfrog"]);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, x));
    never_offered_until(&mut e, SeatId(0), WindowLabel::Draw, play(0, 0, a));
    assert_eq!(e.state.seats[0].hand.len(), 6, "held until after combat");
    pick(&mut e, Choice::FindCard(roll));
    request(&mut e);
    assert_eq!(e.state.seats[0].hand.len(), 7);
    assert!(!ash_has(&e, 0, "Roll"));
}

/// Roll: "If this minion is still ready" — read as the replacement comes
/// due: A, sent to torpor by X's .44 Magnum at long range, draws instead.
#[test]
fn roll_is_drawn_for_when_its_minion_went_to_torpor() {
    let roll = find_library("Roll");
    let (mut e, a, x) = roll_fight(0, &[], &[]);
    let gun = on(&mut e, ".44 Magnum", x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, weapon(gun, 0));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::FindCard(roll),
    );
    assert!(torpid(&e, a));
    assert_eq!(e.state.seats[0].hand.len(), 7, "drawn");
    assert!(ash_has(&e, 0, "Roll"));
}

/// Roll's replacement edges, as Steely Tenacity's: out of the ash heap,
/// drawn for [RBK targeting-of-cards]; held by The Meddling of Semsith,
/// still taken back once the hold ends [LSJ 20100527]; canceled (Direct
/// Intervention), replaced at once [LSJ 20080630]; beside Visit from the
/// Capuchin's replacement, taken back by choice (rules-qa 106.14).
#[test]
fn roll_replacement_edges() {
    let roll = find_library("Roll");
    let (mut e, a, _) = roll_fight(3, &[], &[]);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::Draw);
    let ash = &mut e.state.seats[0].ash;
    let k = ash
        .iter()
        .position(|&c| c == roll)
        .expect("in the ash heap");
    ash.take(k);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::FindCard(roll),
    );
    assert_eq!(e.state.seats[0].hand.len(), 7, "drawn");

    let (mut e, a, _) = roll_fight(3, &[], &[]);
    let id = in_play(&mut e, 1, "Meddling of Semsith, The", None);
    e.state.cards[id.index()].against = zvtes::ids::SeatSet::single(SeatId(0));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].hand.len(), 6, "held");
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::Draw);
    assert!(req.options.contains(&Choice::FindCard(roll)));

    let (mut e, a, _) = roll_fight(3, &[], &[]);
    hand(&mut e, 1, &["Direct Intervention"]);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, master_play(0));
    request(&mut e);
    assert_eq!(e.state.seats[0].hand.len(), 7, "replaced at once");
    assert!(e.state.seats[0].retakes.is_empty());

    let (mut e, a, _) = roll_fight(3, &[], &[]);
    let id = in_play(&mut e, 0, "Visit from the Capuchin", None);
    e.state.cards[id.index()].counters = 1;
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::FindCard(roll));
    request(&mut e);
    assert_eq!(
        e.state.cards[id.index()].counters,
        1,
        "the Capuchin's not taken"
    );
    assert!(e.state.seats[0].hand.contains(&roll));
}

/// The card `name` in seat `seat`'s ash heap.
fn to_ash(e: &mut Engine, seat: usize, name: &str) {
    e.state.seats[seat].to_ash(find_library(name), zvtes::copies::CopyId::NONE);
}

/// The retainer `name` in play on `host`, and its life.
fn retainer_on(e: &Engine, name: &str, host: MinionId) -> Option<(u8, u8)> {
    let card = find_library(name);
    e.state
        .cards
        .iter()
        .find(|c| c.card == card && c.present() && c.host == Some(host))
        .map(|c| (c.mode, c.counters))
}

/// Summon Spirit Beast [ani]: "Discard X cards" — X chosen as played, 0
/// allowed [LSJ 20030502], paid in blood: A discards 2, replaced.
#[test]
fn summon_spirit_beast_discards_x_cards_for_x_blood() {
    let (mut e, a, x) = duel(&["ani"], &[]);
    hand(&mut e, 0, &["Summon Spirit Beast"]);
    fill_hand(&mut e, 0);
    e.state.seats[0]
        .library
        .extend([find_library("Deflection"); 5]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert!(req.options.contains(&set_x(0)), "{:?}", req.options);
    e.choose(index_of(&req, set_x(2)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 2 - 1, "X and X's hand strike");
    assert_eq!(
        e.state.seats[0].library.len(),
        5 - 3,
        "the card and both discards replaced"
    );
    let fom = find_library("Form of Mist");
    assert_eq!(
        e.state.seats[0].ash.iter().filter(|&&c| c == fom).count(),
        2
    );
}

/// Summon Spirit Beast [spi]: "Move X animal retainers from your ash heap
/// to this vampire, with life from the blood bank equal to their starting
/// amounts. After this minion phase ends, burn those retainers": Raven Spy
/// and Ilomba named (a Ghoul Retainer, no animal, never offered), put in
/// play on A — "Cards requiring a discipline come in play at the inferior
/// version" [RBK equip] [RBK recruit-ally] [RBK employ-retainer],
/// "Requirements do not apply" [LSJ 20100204] [LSJ 20040518-2] [LSJ
/// 20100302-1]: A has no Animalism, no Laibon; 2 blood paid; burned as
/// seat 0's minion phase ends — Ilomba's burn A's blood instead (its text:
/// "If Ilomba would burn a life (or would otherwise be burned), this minion
/// burns a blood or life instead").
#[test]
fn summon_spirit_beast_brings_animal_retainers_until_the_minion_phase_ends() {
    let (mut e, a, x) = duel(&["spi"], &[]);
    for name in ["Raven Spy", "Ilomba", "Ghoul Retainer"] {
        to_ash(&mut e, 0, name);
    }
    hand(&mut e, 0, &["Summon Spirit Beast"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&Choice::FindCard(find_library("Ghoul Retainer"))));
    e.choose(index_of(&req, Choice::FindCard(find_library("Raven Spy"))));
    pick(&mut e, Choice::FindCard(find_library("Ilomba")));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        retainer_on(&e, "Raven Spy", a),
        Some((0, 1)),
        "inferior, 1 life"
    );
    assert!(retainer_on(&e, "Ilomba", a).is_some());
    assert_eq!(blood(&e, a), 4 - 2 - 1, "X = 2, and X's hand strike");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(retainer_on(&e, "Raven Spy", a).is_none() && ash_has(&e, 0, "Raven Spy"));
    assert!(retainer_on(&e, "Ilomba", a).is_some(), "Ilomba's own text");
    assert_eq!(blood(&e, a), 4 - 2 - 1 - 1, "its burn A's blood instead");
}

/// Summon Spirit Beast [SPI]: "not burned until after the end of your next
/// minion phase": played as a reaction to seat 1's bleed, the Raven Spy
/// stays through seat 0's next minion phase, burned as it ends; played in
/// combat in seat 0's own minion phase, it stays a turn more.
#[test]
fn summon_spirit_beast_superior_keeps_them_through_your_next_minion_phase() {
    let (mut e, a, x) = duel(&["spi", "spi"], &[]);
    to_ash(&mut e, 0, "Raven Spy");
    hand(&mut e, 0, &["Summon Spirit Beast"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(x));
    pick_when(&mut e, play(0, 4, a));
    pick(&mut e, Choice::FindCard(find_library("Raven Spy")));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        retainer_on(&e, "Raven Spy", a).is_some(),
        "seat 0's next minion phase"
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(retainer_on(&e, "Raven Spy", a).is_none());

    let (mut e, a, x) = duel(&["spi", "spi"], &[]);
    to_ash(&mut e, 0, "Raven Spy");
    hand(&mut e, 0, &["Summon Spirit Beast"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 3, a));
    pick(&mut e, Choice::FindCard(find_library("Raven Spy")));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        retainer_on(&e, "Raven Spy", a).is_some(),
        "through the next minion phase"
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(retainer_on(&e, "Raven Spy", a).is_none());
}

/// Dog Pack "put on a minion after an opponent has declared a 'strike:
/// combat ends' (eg. using … Summon Spirit Beast), the opponent's strike
/// has no effect" [ANK 20190412]: X's Majesty [pre], then A's Summon
/// Spirit Beast [spi] brings Dog Pack: A's hand strike lands.
#[test]
fn summon_spirit_beast_brings_dog_pack_after_a_combat_ends_strike() {
    let (mut e, a, x) = duel(&["spi"], &["pre"]);
    to_ash(&mut e, 0, "Dog Pack");
    hand(&mut e, 0, &["Summon Spirit Beast"]);
    hand(&mut e, 1, &["Majesty"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    let req = offering(&mut e, play(0, 1, a));
    assert_eq!(req.window, WindowLabel::CombatBeforeResolution);
    e.choose(index_of(&req, play(0, 1, a)));
    pick(&mut e, Choice::FindCard(find_library("Dog Pack")));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, x),
        4 - 1 - 1,
        "Majesty's blood and A's hand strike"
    );
}

/// A duel whose X (seat 1) is an anarch holding Groundfighting.
fn anarch_x_duel(a_levels: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    let x = e.add_minion(
        SeatId(1),
        textless(
            |c| c.sect == Some(zvtes::crypt::Sect::Anarch) && (4..=8).contains(&c.capacity),
            0,
        ),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 3),
        4,
        false,
    );
    e.state.minions[a.index()].gained_levels = a_levels.to_vec();
    hand(&mut e, 1, &["Groundfighting"]);
    (e, a, x)
}

/// Groundfighting: "Cannot cancel a card that prevents additional strikes
/// or removes the ability to strike completely" [LSJ 20050221]: A's Rigor
/// Mortis [pre] is not canceled.
#[test]
fn groundfighting_cannot_cancel_rigor_mortis() {
    let (mut e, a, x) = anarch_x_duel(&["pre"]);
    hand(&mut e, 0, &["Rigor Mortis"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, x));
    assert!(ash_has(&e, 0, "Rigor Mortis"));
}

/// Groundfighting: "Cannot cancel a card restricting the use of
/// disciplines" [LSJ 20050221]: A's Withering [THN] is not canceled, it
/// lands on X.
#[test]
fn groundfighting_cannot_cancel_withering() {
    let (mut e, a, x) = anarch_x_duel(&["thn", "thn"]);
    hand(&mut e, 0, &["Withering"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, x));
    assert!(on_minion(&e, "Withering", x));
}

// ---- Round 32 vampires, Otieno to Zygodat -----------------------------------

/// The `nth` plain fixture vampire of capacity 4 to 8 (`textless`).
fn mid(nth: usize) -> u16 {
    textless(|c| (4..=8).contains(&c.capacity), nth)
}

/// The text-only vampires resolve to their crypt card, with no definition,
/// and carry their printed sect and title.
#[test]
fn the_text_only_vampires_otieno_to_zygodat_carry_their_printed_traits() {
    use zvtes::crypt::Sect::Laibon;
    let mut e = game(2);
    for (name, title) in [
        ("Panya, The Wicked", Title::None),
        ("Sanjo", Title::None),
        ("Sobayifa", Title::Magaji),
        ("Tsunda", Title::Magaji),
        ("Ubende", Title::Magaji),
        ("Urenna Bunu", Title::None),
    ] {
        let card = find_card(name);
        assert!(zvtes::cards::vampire(card).is_none(), "{name}: no text");
        let m = e.add_minion(SeatId(0), card, 1, false);
        assert_eq!(e.state.sect_of(m), Some(Laibon), "{name}");
        assert_eq!(e.state.title_of(m), title, "{name}");
    }
}

/// Otieno: "+1 bleed against a Methuselah who controls a ready Guruhi." A
/// locked Guruhi is ready: 2 pool; the same in torpor: 1.
#[test]
fn otieno_bleeds_more_against_a_ready_guruhi_controller() {
    for (torpor, lost) in [(false, 2), (true, 1)] {
        let mut e = game(2);
        let o = e.add_minion(SeatId(0), find_card("Otieno"), 4, false);
        e.add_minion(SeatId(0), mid(0), 4, false);
        let g = e.add_minion(
            SeatId(1),
            fixture_vampire(|c| c.clan == "Guruhi", 0),
            4,
            torpor,
        );
        e.state.minions[g.index()].locked = true;
        pick(&mut e, bleed(o));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10 - lost, "torpor: {torpor}");
    }
}

/// Rosemarie: "Combat cards that do not require Fortitude [for] cost
/// Rosemarie +1 blood." Dodge (no Discipline) costs her 1; Indomitability
/// [for] its printed 0, X's hand strike prevented.
#[test]
fn rosemarie_pays_more_for_combat_cards_without_fortitude() {
    for (card, mode, after) in [("Dodge", 0, 2), ("Indomitability", 0, 3)] {
        let mut e = game(2);
        let r = e.add_minion(SeatId(0), find_card("Rosemarie"), 3, false);
        e.add_minion(SeatId(0), mid(0), 4, false);
        let x = e.add_minion(SeatId(1), mid(1), 4, false);
        e.add_minion(SeatId(1), mid(2), 4, false);
        hand(&mut e, 0, &[card]);
        fight(&mut e, r, x);
        pick_when(&mut e, play(0, mode, r));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, r), after, "{card}");
        assert!(ash_has(&e, 0, card), "{card}");
    }
}

/// Shasa Abu Badr: "+1 bleed when bleeding with an action card that
/// requires Presence [pre]." Entrancement [pre] (+1): 3; the rulebook
/// bleed: 1.
#[test]
fn shasa_bleeds_more_with_a_presence_action_card() {
    for (card, lost) in [(true, 3), (false, 1)] {
        let mut e = game(2);
        let s = e.add_minion(SeatId(0), find_card("Shasa Abu Badr"), 4, false);
        e.add_minion(SeatId(0), mid(0), 4, false);
        hand(&mut e, 0, &["Entrancement"]);
        if card {
            pick(&mut e, play(0, 0, s));
        } else {
            pick(&mut e, bleed(s));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10 - lost, "card: {card}");
    }
}

/// Zhara: "The leave torpor action costs Zhara 1 less blood." With 1
/// blood in torpor: offered, and she leaves it with none.
#[test]
fn zhara_leaves_torpor_for_one_blood() {
    let mut e = game(2);
    let z = e.add_minion(SeatId(0), find_card("Zhara"), 1, true);
    e.add_minion(SeatId(0), mid(0), 4, false);
    pick(&mut e, action(z, zvtes::decision::ActionKind::LeaveTorpor));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!torpid(&e, z));
    assert_eq!(blood(&e, z), 0);
}

/// Yseult: "She cannot block vampires with capacity 5 or more." A
/// capacity-5 bleeder: no attempt offered; a capacity-4 one: offered.
#[test]
fn yseult_cannot_block_a_capacity_five_vampire() {
    for (cap, offered) in [(5, false), (4, true)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity == cap, 0), 4, false);
        e.add_minion(SeatId(0), mid(0), 4, false);
        let y = e.add_minion(SeatId(1), find_card("Yseult"), 4, false);
        e.add_minion(SeatId(1), mid(1), 4, false);
        pick(&mut e, bleed(a));
        assert_eq!(
            offered_before(&mut e, block(y), SeatId(0), WindowLabel::MinionPhase),
            offered,
            "capacity {cap}"
        );
    }
}

/// Yseult: "After you replace a card requiring Melpominee [mel] Yseult
/// played, you can draw 1 additional card." Toreador's Bane [mel] against a
/// younger vampire: the extra draw offered; its [pre] mode: none.
#[test]
fn yseult_draws_an_extra_card_after_a_melpominee_card() {
    for (mode, extra) in [(1, true), (0, false)] {
        let mut e = game(2);
        let y = e.add_minion(SeatId(0), find_card("Yseult"), 4, false);
        e.add_minion(SeatId(0), mid(0), 4, false);
        let x = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 4, false);
        e.add_minion(SeatId(1), mid(1), 4, false);
        hand(&mut e, 0, &["Toreador's Bane"]);
        fill_hand(&mut e, 0);
        e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
        fight(&mut e, y, x);
        pick_when(&mut e, play(0, mode, y));
        assert_eq!(
            offered_before(
                &mut e,
                Choice::DrawLibrary,
                SeatId(0),
                WindowLabel::MinionPhase
            ),
            extra,
            "mode {mode}"
        );
    }
}

/// Zygodat: "When Zygodat successfully bleeds a Methuselah, she may burn
/// the top two cards of that Methuselah's library." Two burned; not
/// offered after a blocked bleed.
#[test]
fn zygodat_burns_two_library_cards_after_a_successful_bleed() {
    let mut e = game(2);
    let z = e.add_minion(SeatId(0), find_card("Zygodat"), 4, false);
    e.add_minion(SeatId(0), mid(0), 4, false);
    fill_hand(&mut e, 1);
    e.state.seats[1].library = vec![find_library("Bonding"); 5].into();
    let burn = Choice::UseAbility {
        source: AbilitySource::Vampire(z),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick(&mut e, bleed(z));
    pick_when(&mut e, burn);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].library.len(), 3);
    assert_eq!(e.state.seats[1].ash.to_vec(), [find_library("Bonding"); 2]);
    // Blocked: never offered.
    let mut e = game(2);
    let z = e.add_minion(SeatId(0), find_card("Zygodat"), 4, false);
    e.add_minion(SeatId(0), mid(0), 4, false);
    let x = e.add_minion(SeatId(1), mid(1), 4, false);
    e.state.seats[1].library = vec![find_library("Bonding"); 5].into();
    fight(&mut e, z, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, burn);
}

/// Is `o` the use of `m`'s printed ability `k`?
fn uses_own(o: &Choice, m: MinionId, k: u8) -> bool {
    matches!(*o, Choice::UseAbility { source: AbilitySource::Vampire(v), ability, .. }
        if v == m && ability == k)
}

/// An uncontrolled slot of `card` with `counters`.
fn slot(card: u16, counters: u8) -> zvtes::state::UncontrolledSlot {
    zvtes::state::UncontrolledSlot {
        card,
        copy: zvtes::copies::Untracked(zvtes::copies::CopyId::NONE),
        counters,
        banished: None,
    }
}

/// Uchenna: "During your master phase, Uchenna may burn a blood to burn a
/// card from your hand. Draw up to your hand size afterward." The card to
/// the ash heap, the hand drawn back up, a blood burned; once that master
/// phase.
#[test]
fn uchenna_burns_a_blood_and_a_hand_card_in_the_master_phase() {
    let mut e = game(2);
    let u = e.add_minion(SeatId(0), find_card("Uchenna"), 4, false);
    e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    hand(&mut e, 0, &["Bonding"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Deflection"); 3].into();
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick_matching(&mut e, |o| uses_own(o, u, 0));
    pick(&mut e, Choice::Discard(0));
    request(&mut e);
    assert!(ash_has(&e, 0, "Bonding"));
    assert_eq!(blood(&e, u), 3);
    let s = &e.state.seats[0];
    assert_eq!(s.hand.len(), 7);
    assert!(s.hand.contains(&find_library("Deflection")));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_own(o, u, 0)
    });
}

/// Uchenna (UC-3): the blood is her cost: with none, not offered.
#[test]
fn uchenna_needs_a_blood() {
    let mut e = game(2);
    // In torpor (her ability is used from there too): no hunt to refill.
    let u = e.add_minion(SeatId(0), find_card("Uchenna"), 0, true);
    e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    hand(&mut e, 0, &["Bonding", "Minion Tap"]);
    fill_hand(&mut e, 0);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert_eq!(blood(&e, u), 0);
    assert!(!req.options.iter().any(|o| uses_own(o, u, 0)));
}

/// Ugadja: "may move up to 2 blood from himself to a younger Laibon in your
/// uncontrolled region as a +1 stealth action." A younger Laibon: 2 moved;
/// an older Laibon and a younger non-Laibon are no targets.
#[test]
fn ugadja_moves_blood_to_a_younger_uncontrolled_laibon() {
    use zvtes::crypt::Sect;
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Ugadja"), 6, false);
    e.add_minion(SeatId(0), mid(0), 4, false);
    let laibon = |c: &zvtes::crypt::CryptCard| c.sect == Some(Sect::Laibon);
    let older = zvtes::crypt::CRYPT
        .iter()
        .position(|c| laibon(c) && c.capacity >= 10 && c.name != "Ugadja")
        .expect("an older Laibon") as u16;
    let u = &mut e.state.seats[0].uncontrolled;
    u.push(slot(textless(|c| laibon(c) && c.capacity <= 6, 0), 0));
    u.push(slot(textless(|c| !laibon(c) && c.capacity <= 6, 0), 0));
    u.push(slot(older, 0));
    let req = request(&mut e);
    let at = |i: usize, k: u8| Choice::UseAbility {
        source: AbilitySource::Vampire(g),
        ability: k,
        actor: Some(g),
        target: Target::Uncontrolled(i as u8),
    };
    assert!(req.options.contains(&at(0, 1)));
    assert!(!req.options.contains(&at(1, 0)), "not a Laibon");
    assert!(!req.options.contains(&at(2, 0)), "not younger");
    pick(&mut e, at(0, 1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 2);
    assert_eq!(blood(&e, g), 4);
}

/// Ugadja's "up to 2", Zayyat's and Casino Reeds' "up to 3" include 0, a
/// legal no-op action (D14, rules-qa 68.5, 73.3; one mechanism): taken
/// with no blood moved, the mover locked by it.
#[test]
fn ugadja_moves_zero_blood_as_a_legal_no_op() {
    use zvtes::crypt::{CryptCard, Sect};
    type Fits = fn(&CryptCard) -> bool;
    let cases: [(&str, u8, Fits); 3] = [
        ("Ugadja", 2, |c| c.sect == Some(Sect::Laibon)),
        ("Zayyat, The Sandstorm", 4, |_| true),
        ("Casino Reeds", 3, |c| c.clan == "Nosferatu"),
    ];
    for (name, ability, fits) in cases {
        let mut e = game(2);
        let card = find_card(name);
        let g = e.add_minion(SeatId(0), card, 6, false);
        e.add_minion(SeatId(0), mid(0), 4, false);
        let cap = CRYPT[card as usize].capacity;
        e.state.seats[0]
            .uncontrolled
            .push(slot(textless(|c| fits(c) && c.capacity < cap, 0), 0));
        pick(
            &mut e,
            Choice::UseAbility {
                source: AbilitySource::Vampire(g),
                ability,
                actor: Some(g),
                target: Target::Uncontrolled(0),
            },
        );
        request(&mut e);
        assert_eq!(e.state.seats[0].uncontrolled[0].counters, 0, "{name}");
        assert_eq!(blood(&e, g), 6, "{name}");
        assert!(e.state.minions[g.index()].locked, "{name}");
    }
}

/// Prejudice: "Once each action, Prejudice may remove an ally or retainer
/// in your prey's ash heap from the game to get +2 bleed for the current
/// action." Used during his bleed, before resolution [ANK 20231124] [LSJ
/// 19970718]: one Ghoul Retainer removed (not burned), 3 pool; a second use
/// not offered with another still there.
#[test]
fn prejudice_removes_a_prey_ally_for_two_bleed() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), find_card("Prejudice"), 4, false);
    e.add_minion(SeatId(0), mid(0), 4, false);
    let ghoul = find_library("Ghoul Retainer");
    e.state.seats[1].ash = vec![ghoul, find_library("Bonding"), ghoul].into();
    pick(&mut e, bleed(p));
    // The Ghoul Retainer the only card offered: picked as the lone option.
    pick_matching(&mut e, |o| uses_own(o, p, 0));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_own(o, p, 0)
    });
    assert_eq!(e.state.seats[1].pool, 10 - 3);
    let s = &e.state.seats[1];
    assert_eq!(s.ash.iter().filter(|&&c| c == ghoul).count(), 1);
    assert_eq!(s.removed.to_vec(), [ghoul]);
}

/// Prejudice: "Can only use his ability when he is bleeding" [ANK
/// 20180514]: his hunt, never; his bleed with no ally or retainer in the
/// prey's ash heap, never (the removal his cost, D14).
#[test]
fn prejudice_cannot_boost_without_bleeding() {
    for hunt in [true, false] {
        let mut e = game(2);
        let p = e.add_minion(SeatId(0), find_card("Prejudice"), 4, false);
        e.add_minion(SeatId(0), mid(0), 4, false);
        let card = if hunt { "Ghoul Retainer" } else { "Bonding" };
        e.state.seats[1].ash = vec![find_library(card)].into();
        let act = if hunt {
            zvtes::decision::ActionKind::Hunt
        } else {
            zvtes::decision::ActionKind::Bleed
        };
        pick(&mut e, action(p, act));
        never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
            uses_own(o, p, 0)
        });
    }
}

/// Kine Resources Contested by `caller` (seat 0), its terms allocated, up
/// to its referendum.
fn kine(e: &mut Engine, caller: MinionId) {
    hand(e, 0, &["Kine Resources Contested"]);
    pick(e, play(0, 0, caller));
    kine_terms(e);
}

/// The Kine Resources Contested played: its terms allocated, up to its
/// referendum.
fn kine_terms(e: &mut Engine) {
    for _ in 0..200 {
        let req = request(e);
        if req.window != WindowLabel::ReferendumTerms {
            if e.referendum().is_some() {
                return;
            }
            e.choose(fallback(&req));
            continue;
        }
        let alloc = req
            .options
            .iter()
            .copied()
            .find(|o| matches!(o, Choice::AllocateTo { .. }))
            .expect("an allocation");
        e.choose(index_of(&req, alloc));
    }
}

/// Sayshila: "If Sayshila is ready during the polling step of any
/// referendum, she can lock to give another titled vampire you control +2
/// votes." Her controller's magaji gets 2 more votes, she locks; once.
#[test]
fn sayshila_locks_to_give_a_titled_vampire_two_votes() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), mid(0), 4, false);
    let s = e.add_minion(SeatId(0), find_card("Sayshila"), 4, false);
    let t = e.add_minion(SeatId(0), find_card("Sobayifa"), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    kine(&mut e, caller);
    let before = e.referendum().expect("referendum").votes_of(&e.state, t);
    let give = Choice::UseAbility {
        source: AbilitySource::Vampire(s),
        ability: 0,
        actor: None,
        target: Target::Minion(t),
    };
    pick_when(&mut e, give);
    let req = request(&mut e);
    assert!(e.state.minions[s.index()].locked);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, t), before + 2);
    assert!(!req.options.iter().any(|o| uses_own(o, s, 0)), "once");
}

/// Sayshila: "Minions in combat with her cannot press to continue combat."
/// X's Flow Within the Mountain press is never offered against her; against
/// another vampire it is.
#[test]
fn sayshila_bars_presses_to_continue() {
    for (sayshila, offered) in [(true, false), (false, true)] {
        let mut e = game(2);
        let a = if sayshila {
            e.add_minion(SeatId(0), find_card("Sayshila"), 4, false)
        } else {
            e.add_minion(SeatId(0), mid(0), 4, false)
        };
        e.add_minion(SeatId(0), mid(1), 4, false);
        let x = e.add_minion(SeatId(1), mid(2), 4, false);
        e.add_minion(SeatId(1), mid(3), 4, false);
        e.state.minions[x.index()].gained_levels = vec!["tha"];
        hand(&mut e, 1, &["Flow Within the Mountain"]);
        fight(&mut e, a, x);
        assert_eq!(
            offered_before(&mut e, play(0, 0, x), SeatId(0), WindowLabel::MinionPhase),
            offered,
            "Sayshila: {sayshila}"
        );
    }
}

/// Sayshila: a press a card provides her opponent still continues combat,
/// "the minion is not the one pressing" (LSJ 20040521): Frenzy played on
/// her opponent, two rounds, 2 blood lost to her hand strikes.
#[test]
fn sayshila_leaves_a_press_frenzy_provides() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Sayshila"), 6, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let x = e.add_minion(SeatId(1), mid(2), 6, false);
    e.add_minion(SeatId(1), mid(3), 4, false);
    hand(&mut e, 1, &["Frenzy"]);
    fight(&mut e, a, x);
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(x),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 6 - 2, "two rounds");
}

/// Seat 0's Wamukota (torpid if `torpor`), a Laibon L with [obf]
/// employing Ghoul Retainer, Cloak the Gathering in hand, and a third
/// vampire; the seats after it each a vampire with Mr. Winthrop (+1
/// intercept), [aus] and Eagle's Sight, and a second one. Returns (engine,
/// Wamukota, L, the other seats' vampires).
fn wamukota_table(seats: usize, torpor: bool) -> (Engine, MinionId, MinionId, Vec<MinionId>) {
    use zvtes::crypt::Sect;
    let mut e = game(seats);
    let w = e.add_minion(SeatId(0), find_card("Wamukota"), 4, torpor);
    let l = e.add_minion(
        SeatId(0),
        textless(
            |c| c.sect == Some(Sect::Laibon) && (4..=8).contains(&c.capacity),
            0,
        ),
        4,
        false,
    );
    e.state.minions[l.index()].gained_levels = vec!["obf"];
    e.add_minion(SeatId(0), mid(0), 4, false);
    hand(&mut e, 0, &["Ghoul Retainer", "Cloak the Gathering"]);
    let mut others = Vec::new();
    for s in 1..seats {
        let x = e.add_minion(SeatId(s as u8), mid(2 * s - 1), 4, false);
        e.add_minion(SeatId(s as u8), mid(2 * s), 4, false);
        e.state.minions[x.index()].gained_levels = vec!["aus"];
        played_on(&mut e, "Mr. Winthrop", x, x);
        hand(&mut e, s as u8, &["Eagle's Sight"]);
        others.push(x);
    }
    (e, w, l, others)
}

/// L's employ blocked by X: Cloak the Gathering (+1 stealth), then X's
/// Eagle's Sight (+1 intercept): 2 against 2.
fn employ_blocked(e: &mut Engine, l: MinionId, x: MinionId) {
    pick(e, play(0, 0, l));
    pick_when(e, block(x));
    pick_when(e, play(0, 0, l));
    pick_when(e, play(0, 0, x));
}

/// Wamukota: "If another Laibon you control is blocked while equipping,
/// recruiting, or employing, you can lock Wamukota before block resolution
/// to lock the blocking minion and have that Laibon continue the action as
/// if unblocked." No combat, X locked, Wamukota locked; L's Cloak the
/// Gathering still counts [RTR 19941109] [ANK 20190116]; the action card
/// resolves (the Ghoul Retainer on L) [LSJ 20070808-1].
#[test]
fn wamukota_locks_to_continue_a_laibons_blocked_employ() {
    let (mut e, w, l, xs) = wamukota_table(2, false);
    let x = xs[0];
    employ_blocked(&mut e, l, x);
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionBlocked);
    let lock = req
        .options
        .iter()
        .copied()
        .find(|o| uses_own(o, w, 0))
        .expect("Wamukota offered");
    e.choose(index_of(&req, lock));
    // Seat 1 blocked, it did not decline: asked again (futile).
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert_eq!(e.action().expect("continued").stealth(&e.state), 2);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[w.index()].locked && e.state.minions[x.index()].locked);
    assert_eq!(blood(&e, x), 4, "no combat");
    let ghoul = find_library("Ghoul Retainer");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == ghoul && c.host == Some(l)));
}

/// Wamukota: "Methuselah that had declined to block before do not get
/// another opportunity to block" [ANK 20190116]: seat 1 declined, seat 2
/// blocked; continued, seat 1 is not asked again.
#[test]
fn wamukota_continuing_keeps_the_declines() {
    let (mut e, w, l, xs) = wamukota_table(3, false);
    pick(&mut e, play(0, 0, l));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    e.choose(index_of(&req, Choice::Pass));
    pick_when(&mut e, block(xs[1]));
    pick_when(&mut e, play(0, 0, l));
    pick_when(&mut e, play(0, 0, xs[1]));
    pick_matching(&mut e, |o| uses_own(o, w, 0));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(xs[0]));
    assert!(e.state.minions[xs[1].index()].locked);
}

/// Wamukota: "His ability … can be used when he is in torpor" [LSJ
/// 20011022]; not for a bleed, nor his own action.
#[test]
fn wamukota_continues_from_torpor_but_not_a_bleed() {
    let (mut e, w, l, xs) = wamukota_table(2, true);
    employ_blocked(&mut e, l, xs[0]);
    pick_matching(&mut e, |o| uses_own(o, w, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[xs[0].index()].locked);
    assert_eq!(blood(&e, xs[0]), 4, "no combat");
    // A bleed blocked: not offered.
    let (mut e, w, l, xs) = wamukota_table(2, false);
    pick(&mut e, bleed(l));
    pick_when(&mut e, block(xs[0]));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_own(o, w, 0)
    });
}

/// Wamukota on a blocked Swarm employ: the Swarm went on its actor at the
/// block, out of the ash heap, so the action cannot continue [LSJ
/// 20070808-1]: one Swarm, on L, none on X (fuzz 32b seed 440 put a
/// second).
#[test]
fn wamukota_cannot_continue_a_swarm_already_on_its_actor() {
    let (mut e, w, l, xs) = wamukota_table(2, false);
    let x = xs[0];
    hand(&mut e, 0, &["Swarm"]);
    pick(
        &mut e,
        Choice::Play {
            card: 2,
            mode: 0,
            actor: Some(l),
            target: Target::Minion(x),
        },
    );
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(1, 0, l));
    pick_when(&mut e, play(0, 0, x));
    pick_matching(&mut e, |o| uses_own(o, w, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let swarm = find_library("Swarm");
    let hosts: Vec<_> = e
        .state
        .cards
        .iter()
        .filter(|c| c.in_play && c.card == swarm)
        .map(|c| c.host)
        .collect();
    assert_eq!(hosts, vec![Some(l)]);
}

/// Is `card` in `seat`'s uncontrolled region?
fn uncontrolled_has(e: &Engine, seat: usize, card: u16) -> bool {
    e.state.seats[seat]
        .uncontrolled
        .iter()
        .any(|u| u.card == card)
}

/// Yavu Matebo: "While ready, Yavu may burn 2 blood to cause an ally or
/// younger vampire's action to fail as it is announced. If he does so, Yavu
/// moves to the uncontrolled region." Seat 0's capacity-4 bleed fails, no
/// pool lost; Yavu (seat 1) with his 2 blood left in seat 1's uncontrolled
/// region. A capacity-8 bleeder (not younger): never offered.
#[test]
fn yavu_matebo_fails_a_younger_vampires_action_and_leaves() {
    for (cap, offered) in [(4, true), (8, false)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity == cap, 0), 4, false);
        e.add_minion(SeatId(0), mid(0), 4, false);
        let y = e.add_minion(SeatId(1), find_card("Yavu Matebo"), 4, false);
        e.add_minion(SeatId(1), mid(1), 4, false);
        pick(&mut e, bleed(a));
        if !offered {
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
                uses_own(o, y, 0)
            });
            continue;
        }
        pick_matching(&mut e, |o| uses_own(o, y, 0));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10, "failed");
        assert!(burned(&e, y));
        let u = &e.state.seats[1].uncontrolled;
        assert_eq!(
            u.iter().map(|s| (s.card, s.counters)).collect::<Vec<_>>(),
            [(find_card("Yavu Matebo"), 2)]
        );
    }
}

/// Yavu Matebo "moves to the uncontrolled region (after breaking any
/// temporary control effects on him)": seat 2 controlling him for now, he
/// goes to seat 1's (his permanent controller's) region.
#[test]
fn yavu_matebo_returns_home_from_temporary_control() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 4, 0), 4, false);
    e.add_minion(SeatId(0), mid(0), 4, false);
    let y = e.add_minion(SeatId(2), find_card("Yavu Matebo"), 4, false);
    e.state.minions[y.index()].temp_from = Some(SeatId(1));
    e.add_minion(SeatId(1), mid(1), 4, false);
    e.add_minion(SeatId(2), mid(2), 4, false);
    pick(&mut e, bleed(a));
    pick_matching(&mut e, |o| uses_own(o, y, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let yavu = find_card("Yavu Matebo");
    assert!(uncontrolled_has(&e, 1, yavu));
    assert!(!uncontrolled_has(&e, 2, yavu));
}

/// Banishment: "the chosen vampire is moved to the uncontrolled region
/// (breaking any temporary control effects)", its "last permanent
/// controller's" [ANK 20200408-2]: seat 0's temporarily controlled vampire
/// goes to seat 1's region, not seat 0's.
#[test]
fn banishment_sends_a_temporarily_controlled_vampire_home() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), find_card("Sobayifa"), 4, false);
    let t = e.add_minion(SeatId(0), textless(|c| c.capacity == 4, 0), 4, false);
    e.state.minions[t.index()].temp_from = Some(SeatId(1));
    e.add_minion(SeatId(1), mid(1), 4, false);
    hand(&mut e, 0, &["Banishment"]);
    pick(&mut e, play(0, 0, caller));
    pick_when(&mut e, Choice::ChooseMinion(t));
    pick_when(&mut e, votes(caller, true));
    close_polling(&mut e);
    let card = e.state.minions[t.index()]
        .crypt_card()
        .expect("a crypt vampire");
    assert!(burned(&e, t));
    assert!(uncontrolled_has(&e, 1, card));
    assert!(!uncontrolled_has(&e, 0, card));
}

/// Three seats: seat 0's bleeder A and a second vampire; seat 1 holding the
/// Edge, a locked vampire and Bonding in hand; Sennadurek (locked) for
/// `owner`, beside a second vampire. Returns (engine, A, Sennadurek).
fn sennadurek_table(owner: u8) -> (Engine, MinionId, MinionId) {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let z = e.add_minion(SeatId(1), mid(2), 4, false);
    e.state.minions[z.index()].locked = true;
    let s = e.add_minion(SeatId(owner), find_card("Sennadurek"), 4, false);
    e.state.minions[s.index()].locked = true;
    e.add_minion(SeatId(2), mid(3), 4, false);
    e.state.seats[1].has_edge = true;
    hand(&mut e, 1, &["Bonding"]);
    e.notes = Some(Vec::new());
    (e, a, s)
}

/// Sennadurek: "Whenever a Methuselah loses the Edge when it is not your
/// turn, Sennadurek unlocks, and you may look at that Methuselah's hand."
/// Seat 0 bleeds seat 1 and takes the Edge: Sennadurek (seat 2) unlocks and
/// his controller sees seat 1's hand; seat 0's own Sennadurek, on its turn,
/// stays locked.
#[test]
fn sennadurek_unlocks_and_looks_when_the_edge_moves_in_anothers_turn() {
    let (mut e, a, s) = sennadurek_table(2);
    pick(&mut e, bleed(a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].has_edge);
    assert!(!e.state.minions[s.index()].locked);
    let looks = noted_looks(&mut e);
    assert!(looks.iter().any(|l| l.viewer == SeatId(2)
        && l.seat == SeatId(1)
        && l.cards == [find_library("Bonding")]));
    // His controller's own turn: no unlock.
    let (mut e, a, s) = sennadurek_table(0);
    pick(&mut e, bleed(a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].has_edge);
    assert!(e.state.minions[s.index()].locked);
}

/// Sennadurek in torpor (SE-5): "Sennadurek unlocks" all the same (D25):
/// the Edge taken in seat 0's turn unlocks him there.
#[test]
fn sennadurek_unlocks_in_torpor() {
    let (mut e, a, s) = sennadurek_table(2);
    e.state.minions[s.index()].in_torpor = true;
    pick(&mut e, bleed(a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].has_edge);
    assert!(torpid(&e, s));
    assert!(!e.state.minions[s.index()].locked);
}

/// Sennadurek: the Edge burned for a vote in seat 0's referendum is lost
/// too: he unlocks.
#[test]
fn sennadurek_unlocks_when_the_edge_is_burned_for_a_vote() {
    let (mut e, a, s) = sennadurek_table(2);
    kine(&mut e, a);
    pick_when(&mut e, Choice::CastEdge { favor: false });
    assert!(!e.state.seats[1].has_edge);
    assert!(!e.state.minions[s.index()].locked);
}

/// Sennadurek (red reading: an Edge holder ousted loses it): seat 0's
/// Kine Resources Contested burns seat 1's last pool, the Edge with it,
/// nobody gaining it; Sennadurek (seat 2) unlocks.
#[test]
fn sennadurek_unlocks_when_the_edge_holder_is_ousted() {
    let (mut e, _, s) = sennadurek_table(2);
    let caller = e.add_minion(SeatId(0), find_card("Sobayifa"), 4, false);
    e.state.seats[1].pool = 1;
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, caller));
    for _ in 0..20 {
        let req = request(&mut e);
        if req.window != WindowLabel::ReferendumTerms {
            break;
        }
        let to_one = req
            .options
            .iter()
            .copied()
            .rfind(|o| matches!(o, Choice::AllocateTo { seat, .. } if *seat == SeatId(1)))
            .expect("an allocation to seat 1");
        e.choose(index_of(&req, to_one));
    }
    pick_when(&mut e, votes(caller, true));
    close_polling(&mut e);
    assert!(e.state.seats[1].ousted);
    assert!(!e.state.seats.iter().any(|s| s.has_edge));
    assert!(!e.state.minions[s.index()].locked);
}

/// Seat 0's Phagian and A (no [nec]), seat 1's X (no [nec]) and a second
/// vampire, each seat holding Cold Aura [nec] (1 blood).
fn phagian_table() -> (Engine, MinionId, MinionId, MinionId) {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), find_card("Phagian"), 4, false);
    let no_nec = |c: &zvtes::crypt::CryptCard| {
        (4..=8).contains(&c.capacity)
            && !c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("nec"))
    };
    let a = e.add_minion(SeatId(0), textless(no_nec, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(no_nec, 1), 4, false);
    e.add_minion(SeatId(1), textless(no_nec, 2), 4, false);
    hand(&mut e, 0, &["Cold Aura"]);
    hand(&mut e, 1, &["Cold Aura"]);
    (e, p, a, x)
}

/// Phagian: "Any vampire you control may play cards that require
/// Necromancy [nec] at the basic level, with the cost increased by 1
/// blood." A plays Cold Aura's [nec] mode for 2 blood, never its [NEC] one;
/// seat 1's X is offered none.
#[test]
fn phagian_lets_his_vampires_play_basic_necromancy_for_a_blood_more() {
    let (mut e, _, a, x) = phagian_table();
    fight(&mut e, a, x);
    let req = offering(&mut e, play(0, 0, a));
    assert!(!req.options.contains(&play(0, 1, a)), "basic only");
    e.choose(index_of(&req, play(0, 0, a)));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(*o, Choice::Play { actor: Some(m), .. } if m == x),
    );
    assert_eq!(
        blood(&e, a),
        4 - 2,
        "1 blood and 1 more, no damage at long range"
    );
}

/// Phagian's +1 is the card's cost, paid with it: an action card [nec]
/// (Ex Nihilo, 1 blood) costs A nothing as announced, 2 as it resolves.
#[test]
fn phagian_surcharge_is_paid_with_the_cards_cost() {
    let (mut e, _, a, x) = phagian_table();
    e.state.seats[0].hand.clear();
    hand(&mut e, 0, &["Ex Nihilo"]);
    pick(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert!(
        req.options.contains(&block(x)),
        "a block window, mid-action"
    );
    assert_eq!(blood(&e, a), 4, "nothing paid as announced");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 2);
    let ex = find_library("Ex Nihilo");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == ex && c.host == Some(a)));
}

/// Phagian beside an Infernal Familiar on A: its player picks which pays
/// (rules-qa 122.85). The Familiar picked: its counter, Cold Aura's own 1
/// blood; Phagian picked: 2 blood, no counter.
#[test]
fn phagian_and_an_infernal_familiar_ask_which_grant_pays() {
    for familiar in [true, false] {
        let (mut e, p, a, x) = phagian_table();
        let f = played_on(&mut e, "Infernal Familiar", a, a);
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, 0, a));
        let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
        let mut options = req.options.clone();
        options.sort_by_key(|o| format!("{o:?}"));
        assert_eq!(options, [Choice::ChooseCard(f.0), Choice::ChooseMinion(p)]);
        let pick_it = if familiar {
            Choice::ChooseCard(f.0)
        } else {
            Choice::ChooseMinion(p)
        };
        e.choose(index_of(&req, pick_it));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let (left, counters) = if familiar { (3, 1) } else { (2, 0) };
        assert_eq!(blood(&e, a), left, "familiar: {familiar}");
        assert_eq!(
            e.state.cards[f.index()].counters,
            counters,
            "familiar: {familiar}"
        );
    }
}

/// Seat 0's Zhenga (5 blood) and a second vampire, a hand of Carlton Van
/// Wyk (2 pool), Young Bloods (2 pool), Mr. Winthrop and fillers, a
/// library of Bonding; seat 1's locked vampire and a second one, Direct
/// Intervention in hand.
fn zhenga_table() -> (Engine, MinionId) {
    let mut e = game(2);
    let z = e.add_minion(SeatId(0), find_card("Zhenga"), 5, false);
    e.add_minion(SeatId(0), mid(0), 4, false);
    let x = e.add_minion(SeatId(1), mid(1), 4, false);
    e.state.minions[x.index()].locked = true;
    hand(
        &mut e,
        0,
        &["Carlton Van Wyk", "Young Bloods", "Mr. Winthrop"],
    );
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
    hand(&mut e, 1, &["Direct Intervention"]);
    (e, z)
}

fn hire(name: &str) -> Choice {
    Choice::EmployFromHand {
        card: find_library(name),
        mode: 0,
    }
}

/// Zhenga: "As Zhenga announces a recruit or employ action, she can burn X
/// blood. If the action is successful, she can recruit and employ up to X
/// additional allies and retainers from your hand (requirements and cost
/// apply as normal)." X = 2 burned as announced; Carlton Van Wyk recruited,
/// then Mr. Winthrop and Young Bloods in her player's order, each paid, the
/// hand replaced only after the last [LSJ 20070707] [LSJ 20060530-1].
#[test]
fn zhenga_burns_x_to_hire_x_more_after_a_successful_recruit() {
    let (mut e, z) = zhenga_table();
    pick(&mut e, play(0, 0, z));
    pick_matching(&mut e, |o| uses_own(o, z, 0));
    pick(&mut e, set_x(2));
    request(&mut e);
    assert_eq!(blood(&e, z), 5 - 2);
    pick_when(&mut e, hire("Mr. Winthrop"));
    let req = request(&mut e);
    assert_eq!(req.options, [Choice::Pass, hire("Young Bloods")]);
    assert_eq!(e.state.seats[0].hand.len(), 6, "not replaced in between");
    e.choose(index_of(&req, hire("Young Bloods")));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let s = &e.state.seats[0];
    assert_eq!(s.hand.len(), 7);
    assert_eq!(s.pool, 10 - 2 - 2);
    let in_play = |name: &str| {
        let c = find_library(name);
        e.state.cards.iter().any(|k| k.in_play && k.card == c)
    };
    assert!(in_play("Carlton Van Wyk") && in_play("Young Bloods") && in_play("Mr. Winthrop"));
}

/// Zhenga's hires pay their "cost as normal": a cost reduction is asked
/// for each, as for a searched ally (LSJ 20090920-2): The Shard, passed
/// for Carlton Van Wyk, takes Young Bloods' 2 pool.
#[test]
fn zhenga_hires_take_cost_reductions() {
    let (mut e, z) = zhenga_table();
    let shard = in_play(&mut e, 0, "Shard, London, The", None);
    e.state.seats[0].has_edge = true;
    pick(&mut e, play(0, 0, z));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    e.choose(index_of(&req, Choice::Pass));
    pick_matching(&mut e, |o| uses_own(o, z, 0));
    pick(&mut e, set_x(1));
    pick_when(&mut e, hire("Young Bloods"));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    e.choose(index_of(&req, Choice::ResolveRider { card: shard.0 }));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[shard.index()].locked);
    assert_eq!(e.state.seats[0].pool, 10 - 2, "Carlton's 2 only");
    let yb = find_library("Young Bloods");
    assert!(e.state.cards.iter().any(|k| k.in_play && k.card == yb));
}

/// Zhenga's additional allies "are not 'played in the normal way' (eg.
/// Direct Intervention cannot be used to counter them)" [LSJ 20060530-2]:
/// offered against Carlton Van Wyk, never against Young Bloods after.
#[test]
fn zhenga_hires_are_not_played_normally() {
    let (mut e, z) = zhenga_table();
    let intervention = Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::None,
    };
    pick(&mut e, play(0, 0, z));
    let req = offering(&mut e, intervention);
    assert_eq!(req.seat, SeatId(1), "against the action card");
    e.choose(index_of(&req, Choice::Pass));
    pick_matching(&mut e, |o| uses_own(o, z, 0));
    pick(&mut e, set_x(1));
    pick_when(&mut e, hire("Young Bloods"));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, intervention);
    let yb = find_library("Young Bloods");
    assert!(e.state.cards.iter().any(|k| k.in_play && k.card == yb));
}

/// Zhenga "does not work with Piper (not an action)" [LSJ 20090115-1]
/// [ANK 20170309]: Zhenga made Anarch recruits Carlton Van Wyk with Piper;
/// her ability is never offered.
#[test]
fn zhenga_is_not_offered_with_piper() {
    let (mut e, z) = zhenga_table();
    e.state.minions[z.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    let last = e.state.seats[0].hand.len() - 1;
    e.state.seats[0].hand.take(last);
    e.state.seats[0].hand.push(find_library("Piper"));
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick_when(
        &mut e,
        Choice::Play {
            card: 6,
            mode: 0,
            actor: Some(z),
            target: Target::None,
        },
    );
    pick_when(&mut e, hire("Carlton Van Wyk"));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_own(o, z, 0) || matches!(o, Choice::EmployFromHand { .. })
    });
    let c = find_library("Carlton Van Wyk");
    assert!(e.state.cards.iter().any(|k| k.in_play && k.card == c));
}

/// Zhenga's X is burned as announced, kept if the action is blocked; no
/// additional hire then.
#[test]
fn zhenga_blocked_keeps_no_hires() {
    let (mut e, z) = zhenga_table();
    let b = e.add_minion(SeatId(1), mid(2), 4, false);
    played_on(&mut e, "Mr. Winthrop", b, b);
    pick(&mut e, play(0, 0, z));
    pick_matching(&mut e, |o| uses_own(o, z, 0));
    pick(&mut e, set_x(2));
    pick_when(&mut e, block(b));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::EmployFromHand { .. })
    });
    assert!(blood(&e, z) <= 5 - 2, "the X burned");
    let yb = find_library("Young Bloods");
    assert!(!e.state.cards.iter().any(|k| k.in_play && k.card == yb));
}

// ---- Round 32 vampires, Agru Kabera onward ----------------------------------

/// Dolie, Hasani and Honorine Ateba ("Laibon."): no definition, Laibon.
#[test]
fn the_text_only_vampires_agru_kabera_onward_carry_their_printed_traits() {
    let mut e = game(2);
    for name in ["Dolie", "Hasani", "Honorine Ateba"] {
        let card = find_card(name);
        assert!(zvtes::cards::vampire(card).is_none(), "{name}: no text");
        let m = e.add_minion(SeatId(0), card, 1, false);
        assert_eq!(
            e.state.sect_of(m),
            Some(zvtes::crypt::Sect::Laibon),
            "{name}"
        );
        assert_eq!(e.state.title_of(m), Title::None, "{name}");
    }
}

/// Agru Kabera, Amavi: "Laibon: +1 bleed." A plain bleed: 2 pool.
#[test]
fn agru_kabera_and_amavi_bleed_for_two() {
    for name in ["Agru Kabera", "Amavi"] {
        let mut e = game(2);
        let v = e.add_minion(SeatId(0), find_card(name), 4, false);
        e.add_minion(SeatId(0), mid(0), 4, false);
        e.add_minion(SeatId(1), mid(1), 4, false);
        pick(&mut e, bleed(v));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10 - 2, "{name}");
    }
}

/// Dela Eden: "Laibon: +1 strength." Her hand strike: 2 damage. Bupe
/// Kuila: "Laibon: +1 intercept." Against a bleed made +1 stealth as his
/// attempt is made (Cloak the Gathering): his block succeeds, a plain
/// vampire's fails.
#[test]
fn bupe_kuila_dela_eden_printed_bonuses() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Dela Eden"), 4, false);
    e.add_minion(SeatId(0), mid(0), 4, false);
    let x = e.add_minion(SeatId(1), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    fight(&mut e, d, x);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2);
    for bupe in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), mid(0), 4, false);
        e.add_minion(SeatId(0), mid(1), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["obf"];
        let b = if bupe {
            e.add_minion(SeatId(1), find_card("Bupe Kuila"), 4, false)
        } else {
            e.add_minion(SeatId(1), mid(2), 4, false)
        };
        e.add_minion(SeatId(1), mid(3), 4, false);
        hand(&mut e, 0, &["Cloak the Gathering"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(b));
        pick_when(&mut e, play(0, 0, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.seats[1].pool,
            if bupe { 10 } else { 9 },
            "bupe {bupe}"
        );
    }
}

/// Antoinette Dubois: "Toreador and Toreador antitribu cannot block
/// Antoinette. -1 intercept." Neither is offered a block; a Ventrue is.
/// Her intercept: a plain bleed she attempts to block fails (0 - 1 < 0).
#[test]
fn antoinette_dubois_cannot_be_blocked_by_toreador() {
    for (clan, offered) in [
        ("Toreador", false),
        ("Toreador antitribu", false),
        ("Ventrue", true),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Antoinette Dubois"), 4, false);
        e.add_minion(SeatId(0), mid(0), 4, false);
        let x = e.add_minion(SeatId(1), fixture_vampire(|c| c.clan == clan, 0), 4, false);
        e.add_minion(SeatId(1), mid(1), 4, false);
        pick(&mut e, bleed(a));
        assert_eq!(
            offered_before(&mut e, block(x), SeatId(0), WindowLabel::MinionPhase),
            offered,
            "{clan}"
        );
    }
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let a = e.add_minion(SeatId(1), find_card("Antoinette Dubois"), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    pick(&mut e, bleed(x));
    pick_when(&mut e, block(a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "her attempt failed");
}

/// Gentha Shale: "+1 bleed when bleeding a Methuselah who controls a ready
/// Ministry." A ready Ministry: 2; in torpor: 1.
#[test]
fn gentha_shale_bleeds_harder_against_the_ministry() {
    for (torpor, lost) in [(false, 2), (true, 1)] {
        let mut e = game(2);
        let g = e.add_minion(SeatId(0), find_card("Gentha Shale"), 4, false);
        e.add_minion(SeatId(0), mid(0), 4, false);
        e.add_minion(
            SeatId(1),
            fixture_vampire(|c| c.clan == "Ministry", 0),
            4,
            torpor,
        );
        e.add_minion(
            SeatId(1),
            fixture_vampire(|c| c.clan == "Ventrue", 0),
            4,
            false,
        );
        pick(&mut e, bleed(g));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10 - lost, "torpor {torpor}");
    }
}

/// Batsheva: base "+1 strength in combat with a Tzimisce"; merged, the
/// advanced card's +1 against a Malkavian and the [MERGED] +1 (rules-qa
/// 109.28 corrected): her hand strike against each clan.
#[test]
fn batsheva_base_and_merged_strength() {
    let adv = CRYPT
        .iter()
        .position(|c| c.name == "Batsheva" && c.adv)
        .expect("advanced") as u16;
    for (merged, clan, damage) in [
        (false, "Tzimisce", 2),
        (false, "Malkavian", 1),
        (true, "Malkavian", 3),
        (true, "Tzimisce", 3),
        (true, "Ventrue", 2),
    ] {
        let mut e = game(2);
        let b = e.add_minion(SeatId(0), find_card("Batsheva"), 4, false);
        if merged {
            e.state.minions[b.index()].merged = Some(find_card("Batsheva"));
            e.state.minions[b.index()].kind = zvtes::state::MinionKind::Vampire(adv);
        }
        e.add_minion(SeatId(0), mid(0), 4, false);
        let x = e.add_minion(SeatId(1), fixture_vampire(|c| c.clan == clan, 0), 4, false);
        e.add_minion(SeatId(1), mid(1), 4, false);
        fight(&mut e, b, x);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, x), 4 - damage, "merged {merged} {clan}");
    }
}

/// Aimee Laroux: "You can burn 1 pool and use 4 transfers to give Aimee +1
/// level of Melpominee [mel]." With 8 transfers: twice, superior; each use
/// 1 pool and 4 transfers.
#[test]
fn aimee_laroux_buys_melpominee_with_pool_and_transfers() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Aimee Laroux"), 2, false);
    e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    e.state.seats[0].transfers_next = 4;
    pick(&mut e, Choice::Pass);
    pick_matching(&mut e, |o| uses_own(o, a, 0));
    assert_eq!(e.state.seats[0].pool, 10 - 1);
    assert_eq!(e.state.discipline_level(a, "mel"), 1);
    pick_matching(&mut e, |o| uses_own(o, a, 0));
    assert_eq!(e.state.seats[0].pool, 10 - 2);
    assert_eq!(e.state.discipline_level(a, "mel"), 2, "superior");
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        uses_own(o, a, 0)
    });
}

/// Aimee Laroux from torpor (AI-3, D25) and at superior (AI-4, a legal
/// no-op, offered: D14): bought twice from torpor, then offered again.
#[test]
fn aimee_laroux_buys_from_torpor_and_is_offered_at_superior() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Aimee Laroux"), 2, true);
    e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    e.state.seats[0].transfers_next = 8;
    pick(&mut e, Choice::Pass);
    pick_matching(&mut e, |o| uses_own(o, a, 0));
    pick_matching(&mut e, |o| uses_own(o, a, 0));
    assert_eq!(e.state.discipline_level(a, "mel"), 2, "superior");
    let req = request(&mut e);
    assert!(
        req.options.iter().any(|o| uses_own(o, a, 0)),
        "{:?}",
        req.options
    );
}

/// Aimee Laroux: "She cannot play reaction cards." Mob Rule (action
/// modifier and reaction): on her own referendum she plays it as an action
/// modifier; on another Methuselah's, never — a plain vampire there may
/// (rules-digest D18).
#[test]
fn aimee_laroux_plays_no_reaction_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Aimee Laroux"), 2, false);
    e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    kine(&mut e, a);
    hand(&mut e, 0, &["Mob Rule"]);
    assert!(offered_before(
        &mut e,
        play(0, 0, a),
        SeatId(0),
        WindowLabel::MinionPhase
    ));
    for aimee in [true, false] {
        let mut e = game(2);
        let caller = e.add_minion(SeatId(0), mid(0), 4, false);
        e.add_minion(SeatId(0), mid(1), 4, false);
        let r = if aimee {
            e.add_minion(SeatId(1), find_card("Aimee Laroux"), 2, false)
        } else {
            e.add_minion(SeatId(1), mid(2), 4, false)
        };
        kine(&mut e, caller);
        hand(&mut e, 1, &["Mob Rule"]);
        assert_eq!(
            offered_before(&mut e, play(0, 0, r), SeatId(0), WindowLabel::MinionPhase),
            !aimee,
            "aimee {aimee}"
        );
    }
}

/// Aiyana, The Wolfcatcher: "an optional press each combat when opposing a
/// werewolf, and she can burn a blood each round to prevent all damage a
/// werewolf inflicts on her that round." Double Deuce blocks her: the press
/// offered; its strike pending, she burns 1 and takes none. A vampire
/// blocking: neither offered.
#[test]
fn aiyana_presses_and_shrugs_off_a_werewolf() {
    for wolf in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Aiyana, The Wolfcatcher"), 3, false);
        e.add_minion(SeatId(0), mid(0), 4, false);
        let x = if wolf {
            e.add_ally(SeatId(1), find_library("Double Deuce"))
        } else {
            e.add_minion(SeatId(1), mid(1), 4, false)
        };
        e.add_minion(SeatId(1), mid(2), 4, false);
        fight(&mut e, a, x);
        if wolf {
            pick_matching(&mut e, |o| uses_own(o, a, 0));
            let req = offering(&mut e, Choice::UsePress);
            assert_eq!(req.seat, SeatId(0));
            e.choose(index_of(&req, Choice::Pass));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, a), 3 - 1, "the burn, no damage");
        } else {
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
                uses_own(o, a, 0) || *o == Choice::UsePress
            });
        }
    }
}

/// The Make the Misere modes (cel, obf, pot, then each larger set) offered
/// to `m`, made Anarch with [cel] gained, against a locked minion of seat 1.
fn misere_modes(name: &str) -> Vec<u8> {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card(name), 4, false);
    e.add_minion(SeatId(0), mid(0), 4, false);
    let x = e.add_minion(SeatId(1), mid(1), 4, false);
    e.state.minions[x.index()].locked = true;
    e.state.minions[m.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    e.state.minions[m.index()].gained_levels = vec!["cel"];
    hand(&mut e, 0, &["Make the Misere"]);
    let req = request(&mut e);
    let mut modes: Vec<u8> = req
        .options
        .iter()
        .filter_map(|o| match *o {
            Choice::Play {
                card: 0,
                mode,
                actor: Some(a),
                ..
            } if a == m => Some(mode),
            _ => None,
        })
        .collect();
    modes.dedup();
    modes
}

/// Al-Muntaquim, The Avenger: "He cannot use cards requiring Celerity
/// [cel]." A multi-Discipline card: its modes without [cel] only [LSJ
/// 20050322-2].
#[test]
fn al_muntaquim_shuns_celerity_modes() {
    assert_eq!(misere_modes("Al-Muntaquim, The Avenger"), [1, 2, 5]);
}

/// Al-Muntaquim: "During your unlock phase, you can burn a Caitiff you
/// control to give Al-Muntaquim +1 bleed until the end of the game." His
/// torpid Caitiff burned, his bleed 2 the next turns; once in the phase;
/// not offered without a Caitiff.
#[test]
fn al_muntaquim_burns_a_caitiff_for_bleed_for_good() {
    let mut e = game(2);
    let al = e.add_minion(SeatId(0), find_card("Al-Muntaquim, The Avenger"), 4, false);
    let c = e.add_minion(
        SeatId(0),
        fixture_vampire(|c| c.clan == "Caitiff", 0),
        2,
        true,
    );
    let c2 = e.add_minion(
        SeatId(0),
        fixture_vampire(|c| c.clan == "Caitiff", 1),
        2,
        false,
    );
    e.add_minion(SeatId(1), mid(1), 4, false);
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(al),
            ability: 0,
            actor: None,
            target: Target::Minion(c),
        },
    );
    assert!(burned(&e, c));
    assert!(e.state.seats[0]
        .ash_crypt
        .contains(&e.state.minions[c.index()].crypt_card().unwrap()));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_own(o, al, 0)
    });
    pick(&mut e, bleed(al));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
    let _ = c2;
    // No Caitiff: never offered.
    let mut e = game(2);
    let al = e.add_minion(SeatId(0), find_card("Al-Muntaquim, The Avenger"), 4, false);
    e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        uses_own(o, al, 0)
    });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
}

/// Al-Muntaquim: "After a Gehenna event is played, he can unlock." Another
/// Methuselah's Fueled by Heart's Blood: locked, he unlocks; unlocked, not
/// offered (rules-qa 48.7).
#[test]
fn al_muntaquim_unlocks_after_a_gehenna_event() {
    for locked in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(0), mid(0), 4, false);
        let al = e.add_minion(SeatId(1), find_card("Al-Muntaquim, The Avenger"), 4, false);
        e.add_minion(SeatId(1), mid(1), 4, false);
        e.state.minions[al.index()].locked = locked;
        hand(&mut e, 0, &["Fueled by Heart's Blood"]);
        to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
        pick(
            &mut e,
            Choice::Play {
                card: 0,
                mode: 0,
                actor: None,
                target: Target::None,
            },
        );
        let unlock = Choice::ResolvePrinted { minion: al };
        if locked {
            let req = offering(&mut e, unlock);
            assert_eq!(req.seat, SeatId(1));
            e.choose(index_of(&req, unlock));
            assert!(!e.state.minions[al.index()].locked);
        } else {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, unlock);
        }
    }
}

/// Ibn Khaldun, Scholar: "During your discard phase, Ibn Khaldun can burn 1
/// blood to unlock. He cannot use cards requiring Celerity [cel]." Locked
/// by his hunt: unlocked for 1 blood, then not offered (unlocked); a
/// multi-Discipline card's [cel] modes barred [LSJ 20050322-2].
#[test]
fn ibn_khaldun_unlocks_in_the_discard_phase_and_shuns_celerity() {
    assert_eq!(misere_modes("Ibn Khaldun, Scholar"), [2]);
    let mut e = game(2);
    let ibn = e.add_minion(SeatId(0), find_card("Ibn Khaldun, Scholar"), 2, false);
    e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    pick(&mut e, action(ibn, zvtes::decision::ActionKind::Hunt));
    pick_matching(&mut e, |o| uses_own(o, ibn, 0));
    assert!(!e.state.minions[ibn.index()].locked);
    assert_eq!(blood(&e, ibn), 2 + 1 - 1, "the hunt's, the burn");
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        uses_own(o, ibn, 0)
    });
}

/// Aren, Priest of Eshu: "+1 stealth on political actions. Any vampire
/// casting votes or ballots against a referendum called by Aren burns 1
/// blood when the results are tallied." His Kine Resources Contested: a
/// blocker's attempt (intercept 0) fails; seat 1's vampire voting against
/// burns 1 blood.
#[test]
fn aren_sneaks_politics_and_taxes_the_naysayers() {
    let mut e = game(2);
    let aren = e.add_minion(SeatId(0), find_card("Aren, Priest of Eshu"), 4, false);
    e.add_minion(SeatId(0), mid(0), 4, false);
    let x = e.add_minion(SeatId(1), find_card("Sobayifa"), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, aren));
    pick_when(&mut e, block(x));
    kine_terms(&mut e);
    pick_when(&mut e, votes(x, false));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 1, "voted against");
}

/// Ayo Igoli: "an additional vote for each ready titled vampire you
/// control" — two ready magajis, a torpid one not: 2 votes. "You may use a
/// discard phase action to move 2 blood from Ayo to a Laibon in your
/// uncontrolled region": a Laibon gets 2, a non-Laibon is no target; with
/// 1 blood, not offered (Menele's).
#[test]
fn ayo_igoli_counts_titled_votes_and_feeds_an_uncontrolled_laibon() {
    use zvtes::crypt::Sect;
    for ayo_blood in [3, 1] {
        let mut e = game(2);
        let ayo = e.add_minion(SeatId(0), find_card("Ayo Igoli"), ayo_blood, false);
        e.add_minion(SeatId(0), find_card("Sobayifa"), 4, false);
        e.add_minion(SeatId(0), find_card("Tsunda"), 4, false);
        e.add_minion(SeatId(0), find_card("Ubende"), 4, true);
        e.add_minion(SeatId(1), mid(1), 4, false);
        assert_eq!(e.state.votes_held(ayo), 2);
        let laibon = |c: &zvtes::crypt::CryptCard| c.sect == Some(Sect::Laibon);
        let u = &mut e.state.seats[0].uncontrolled;
        u.push(slot(textless(laibon, 0), 0));
        u.push(slot(textless(|c| !laibon(c), 0), 0));
        let at = |i: u8| Choice::UseAbility {
            source: AbilitySource::Vampire(ayo),
            ability: 0,
            actor: None,
            target: Target::Uncontrolled(i),
        };
        if ayo_blood == 3 {
            let req = to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
            assert!(!req.options.contains(&at(1)), "not a Laibon");
            pick(&mut e, at(0));
            assert_eq!(e.state.seats[0].uncontrolled[0].counters, 2);
            assert_eq!(blood(&e, ayo), 1);
        } else {
            never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
                uses_own(o, ayo, 0)
            });
        }
    }
}

/// Eze, The Demon Prince: "Once each turn when Eze successfully performs a
/// non-hunt action, he unlocks after resolving that action" [LSJ
/// 20051211]: mandatory, the once spent even when the action unlocks him
/// already; once each turn [ANK 20180719-4]. His bleed: unlocked, a second
/// action (Whispers from the Dead): locked. His hunt: locked. Divine Sign [nec]
/// (it unlocks him) then a bleed: locked.
#[test]
fn eze_unlocks_once_each_turn_after_a_non_hunt_success() {
    let table = || {
        let mut e = game(2);
        let eze = e.add_minion(SeatId(0), find_card("Eze, The Demon Prince"), 4, false);
        e.add_minion(SeatId(0), mid(0), 4, false);
        e.add_minion(SeatId(0), mid(1), 4, false);
        let x = e.add_minion(SeatId(1), mid(2), 4, false);
        (e, eze, x)
    };
    let (mut e, eze, _) = table();
    hand(&mut e, 0, &["Whispers from the Dead"]);
    e.state.seats[0].ash = vec![find_library("Bonding")].into();
    pick(&mut e, bleed(eze));
    to_minion_phase(&mut e, SeatId(0));
    assert!(!e.state.minions[eze.index()].locked, "the first: unlocked");
    pick(&mut e, play(0, 0, eze));
    to_minion_phase(&mut e, SeatId(0));
    assert!(e.state.minions[eze.index()].locked, "the second: locked");
    assert!(e.state.seats[0].hand.contains(&find_library("Bonding")));

    let (mut e, eze, _) = table();
    pick(&mut e, action(eze, zvtes::decision::ActionKind::Hunt));
    to_minion_phase(&mut e, SeatId(0));
    assert!(e.state.minions[eze.index()].locked, "a hunt");

    let (mut e, eze, x) = table();
    hand(&mut e, 0, &["Divine Sign"]);
    pick(&mut e, play_on(0, 0, eze, Target::Minion(x)));
    to_minion_phase(&mut e, SeatId(0));
    assert!(
        !e.state.minions[eze.index()].locked,
        "Divine Sign unlocked him"
    );
    pick(&mut e, bleed(eze));
    to_minion_phase(&mut e, SeatId(0));
    assert!(e.state.minions[eze.index()].locked, "the once spent");
}

/// Eze (EZ-5): a fizzled action is a successful one (ANK 20220218,
/// rules-qa 16.4): Whispers from the Dead's named card gone from the ash
/// heap by resolution, the action fizzles, Eze unlocks.
#[test]
fn eze_unlocks_after_a_fizzled_action() {
    let mut e = game(2);
    let eze = e.add_minion(SeatId(0), find_card("Eze, The Demon Prince"), 4, false);
    e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    hand(&mut e, 0, &["Whispers from the Dead"]);
    e.state.seats[0].ash = vec![find_library("Bonding")].into();
    pick(&mut e, play(0, 0, eze));
    request(&mut e);
    e.state.seats[0].ash.take(0);
    to_minion_phase(&mut e, SeatId(0));
    assert!(!e.state.seats[0].hand.contains(&find_library("Bonding")));
    assert!(!e.state.minions[eze.index()].locked, "fizzled: unlocked");
}

/// Eze: "Eze gets 1 additional vote": a bonus vote, counted by Political
/// Struggle and Leadership Vacuum [ANK 20211009] [LSJ 20040518-2] — his
/// magaji's 2 and 1.
#[test]
fn eze_bonus_vote_counts_for_political_struggle_and_leadership_vacuum() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let eze = e.add_minion(SeatId(1), find_card("Eze, The Demon Prince"), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    assert_eq!(e.state.votes_held(eze), 3);
    let ps = on(&mut e, "Political Struggle", eze);
    e.state.cards[ps.index()].owner = SeatId(0);
    e.state.cards[ps.index()].controller = SeatId(0);
    e.state.cards[ps.index()].chosen = vec![a];
    hand(&mut e, 0, &["Leadership Vacuum"]);
    e.burn(eze);
    assert_eq!(e.state.votes_held(a), 3, "Political Struggle's X");
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Seat(SeatId(1)),
        },
    );
    request(&mut e);
    let lv = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Leadership Vacuum"))
        .expect("in play");
    assert_eq!(e.state.cards[lv].fixed_x, 3, "Leadership Vacuum's X");
}

/// The `nth` plain Laibon fixture vampire.
fn laibon(nth: usize) -> u16 {
    textless(
        |c| c.sect == Some(zvtes::crypt::Sect::Laibon) && c.capacity <= 8,
        nth,
    )
}

/// Falhu Shibaba: "Any Laibon may take a Ⓓ action that costs 1 blood to
/// lock Falhu and cause him not to unlock as normal on his next unlock
/// phase." Another Methuselah's Laibon takes it, directed (Falhu's
/// controller is offered a block): 1 blood paid, Falhu locked, still
/// locked through his next unlock phase. A non-Laibon is never offered it;
/// his own controller's Laibon and Falhu himself are.
#[test]
fn falhu_shibaba_can_be_locked_down_by_any_laibon() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), laibon(0), 4, false);
    let n = e.add_minion(SeatId(0), mid(0), 4, false);
    let f = e.add_minion(SeatId(1), find_card("Falhu Shibaba"), 4, false);
    let y = e.add_minion(SeatId(1), mid(1), 4, false);
    let take = |m| Choice::UseAbility {
        source: AbilitySource::Vampire(f),
        ability: 0,
        actor: Some(m),
        target: Target::Minion(f),
    };
    let req = request(&mut e);
    assert!(!req.options.contains(&take(n)), "not a Laibon");
    pick(&mut e, take(l));
    let req = offering(&mut e, block(y));
    assert_eq!(req.seat, SeatId(1), "directed at Falhu's controller");
    e.choose(index_of(&req, Choice::Pass));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, l), 4 - 1);
    assert!(e.state.minions[f.index()].locked);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[f.index()].locked, "no normal unlock");
    let mut e = game(2);
    let f = e.add_minion(SeatId(0), find_card("Falhu Shibaba"), 4, false);
    let l = e.add_minion(SeatId(0), laibon(0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    let take = |m| Choice::UseAbility {
        source: AbilitySource::Vampire(f),
        ability: 0,
        actor: Some(m),
        target: Target::Minion(f),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&take(l)) && req.options.contains(&take(f)));
}

/// Ganhuru: "Ganhuru can prevent 1 non-aggravated damage each combat." X's
/// hand strike at +1 strength: 1 of 2 prevented. Aggravated damage he
/// treats as normal (Skin of Night [for]) is not preventable by it [LSJ
/// 20040812-2] [PIB 20130327].
#[test]
fn ganhuru_prevents_one_normal_damage_each_combat_never_aggravated() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Ganhuru"), 4, false);
    e.add_minion(SeatId(0), mid(0), 4, false);
    let x = e.add_minion(SeatId(1), find_card("Dela Eden"), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    fight(&mut e, g, x);
    pick_matching(&mut e, |o| uses_own(o, g, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, g), 4 - 1);

    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Ganhuru"), 4, false);
    e.add_minion(SeatId(0), mid(0), 4, false);
    let x = e.add_minion(SeatId(1), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["abo"];
    hand(&mut e, 0, &["Skin of Night"]);
    hand(&mut e, 1, &["Devil-Channel: Hands"]);
    fight(&mut e, g, x);
    pick_when(&mut e, play(0, 1, x));
    pick_when(&mut e, play(0, 0, g));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_own(o, g, 0)
    });
    assert_eq!(blood(&e, g), 4 - 1, "the treated damage mended");
}

/// Ganhuru: "If Ubende is ready, you gain a pool whenever Ganhuru
/// successfully bleeds your prey." Another Methuselah's ready Ubende: +1
/// pool; in torpor: none (`holds_for_host`'s `ReadyNamed`); the bleed
/// deflected to another Methuselah: none (rules-qa 50.3).
#[test]
fn ganhuru_gains_a_pool_bleeding_his_prey_while_ubende_is_ready() {
    for (torpor, deflected, gained) in [(false, false, 1), (true, false, 0), (false, true, 0)] {
        let mut e = game(3);
        let g = e.add_minion(SeatId(0), find_card("Ganhuru"), 4, false);
        e.add_minion(SeatId(0), mid(0), 4, false);
        let d = e.add_minion(SeatId(1), mid(1), 4, false);
        e.add_minion(SeatId(2), find_card("Ubende"), 4, torpor);
        e.add_minion(SeatId(2), mid(2), 4, false);
        if deflected {
            e.state.minions[d.index()].gained_levels = vec!["dom"];
            hand(&mut e, 1, &["Deflection"]);
        }
        pick(&mut e, bleed(g));
        if deflected {
            pick_when(&mut e, play_on(0, 0, d, Target::Seat(SeatId(2))));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.seats[0].pool,
            10 + gained,
            "torpor {torpor} deflected {deflected}"
        );
    }
}

/// Seat 0's Gualtiero Ghiberti, Giovanni G and G2 with [nec] [NEC], a
/// second vampire; seat 1's X.
fn gualtiero_table(x_ally: bool) -> (Engine, MinionId, MinionId, MinionId, MinionId) {
    let mut e = game(2);
    let gu = e.add_minion(SeatId(0), find_card("Gualtiero Ghiberti"), 4, false);
    let giovanni = |nth| fixture_vampire(|c| c.clan == "Giovanni" && c.title == Title::None, nth);
    let g = e.add_minion(SeatId(0), giovanni(0), 4, false);
    let g2 = e.add_minion(SeatId(0), giovanni(1), 4, false);
    for m in [g, g2] {
        e.state.minions[m.index()].gained_levels = vec!["nec", "nec"];
    }
    let x = if x_ally {
        e.add_ally(SeatId(1), find_library("Double Deuce"))
    } else {
        e.add_minion(SeatId(1), mid(0), 4, false)
    };
    e.add_minion(SeatId(1), mid(1), 4, false);
    e.state.seats[0].ash = vec![find_library("Bonding")].into();
    (e, gu, g, g2, x)
}

/// Gualtiero Ghiberti: "Once each turn as another Giovanni announces an
/// action … requiring Necromancy [nec], Gualtiero can burn 1 blood to give
/// that action +1 stealth, even if stealth is not yet needed". G's Whispers
/// from the Dead [nec]: as announced, Gualtiero burns 1; X's block fails,
/// the card comes back. Once each turn [ANK 20180719-4]: G2's Whispers
/// later that turn, not offered; a bleed, never.
#[test]
fn gualtiero_lends_stealth_to_a_giovanni_necromancy_action() {
    let (mut e, gu, g, g2, x) = gualtiero_table(false);
    hand(
        &mut e,
        0,
        &["Whispers from the Dead", "Whispers from the Dead"],
    );
    pick(&mut e, play(0, 0, g));
    pick_matching(&mut e, |o| uses_own(o, gu, 0));
    assert_eq!(blood(&e, gu), 4 - 1);
    pick_when(&mut e, block(x));
    to_minion_phase(&mut e, SeatId(0));
    assert!(e.state.seats[0].hand.contains(&find_library("Bonding")));
    assert!(!e.state.minions[x.index()].locked, "the attempt failed");
    e.state.seats[0].ash = vec![find_library("Bonding")].into();
    pick(&mut e, play(0, 0, g2));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_own(o, gu, 0) || uses_own(o, gu, 1)
    });
    let (mut e, gu, g, _, _) = gualtiero_table(false);
    pick(&mut e, bleed(g));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_own(o, gu, 0)
    });
}

/// Gualtiero Ghiberti: "… as another Giovanni announces … [a] strike
/// requiring Necromancy [nec], … increase the damage inflicted … by that
/// strike by 1." G's Breath of Thanatos [NEC] (1 aggravated) on Double
/// Deuce (3 life): from out of the combat, Gualtiero burns 1 and it lands
/// 2. Once each turn [ANK 20180719-4], whichever branch (rules-qa 62.32):
/// G2's Whispers later, no stealth offered.
#[test]
fn gualtiero_adds_damage_to_a_giovanni_necromancy_strike() {
    let (mut e, gu, g, g2, x) = gualtiero_table(true);
    hand(&mut e, 0, &["Breath of Thanatos", "Whispers from the Dead"]);
    fight(&mut e, g, x);
    pick_when(&mut e, play(0, 1, g));
    pick_matching(&mut e, |o| uses_own(o, gu, 1));
    assert_eq!(blood(&e, gu), 4 - 1);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, x), 3 - 2);
    pick(&mut e, play(0, 0, g2));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_own(o, gu, 0)
    });
}

/// Demdemeh (seat 0), a plain vampire, seat 1's; `retainers` on him: (name,
/// version, counters), each with its version's layers; a Gehenna event in
/// play if `gehenna`. Seat 0's unlock phase reached.
fn demdemeh_table(gehenna: bool, retainers: &[(&str, u8, u8)]) -> (Engine, MinionId, Vec<CardId>) {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Demdemeh"), 4, false);
    e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    let mut ids = Vec::new();
    for &(name, mode, counters) in retainers {
        let id = in_play_mode(&mut e, 0, name, Some(d), mode);
        e.state.cards[id.index()].counters = counters;
        ids.push(id);
    }
    if gehenna {
        e.state.cards.push(CardInPlay::new(
            find_library("Fall of the Sabbat"),
            SeatId(1),
            None,
            0,
        ));
    }
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    (e, d, ids)
}

/// The represented minion Demdemeh's retainer became.
fn demdemeh_ally(e: &Engine) -> MinionId {
    MinionId(
        e.state
            .minions
            .iter()
            .position(|m| {
                m.in_play && matches!(m.kind, zvtes::state::MinionKind::Represented { .. })
            })
            .expect("the ally") as u32,
    )
}

/// Demdemeh: "If any Gehenna events are in play during your unlock phase,
/// you can move an animal retainer on Demdemeh to your ready region; it
/// becomes an animal ally with 1 strength and 1 bleed, and +1 strength
/// while it has 3 or more life." A Wolf Companion [ANI] at 3 life: off
/// him, an animal ally of his controller's, 3 life, 1 bleed, 2 strength;
/// at 2 life, 1 strength. Once in the phase (a Dog Pack left); not offered
/// with no Gehenna event. Its text lost [LSJ 20051116-1]: its damage layer
/// no longer applies.
#[test]
fn demdemeh_turns_an_animal_retainer_into_an_ally() {
    let (mut e, d, ids) = demdemeh_table(true, &[("Wolf Companion", 1, 3), ("Dog Pack", 0, 1)]);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(d),
            ability: 0,
            actor: None,
            target: Target::Card(ids[0]),
        },
    );
    let w = demdemeh_ally(&e);
    let v = &e.state.minions[w.index()];
    assert!(!e.state.cards[ids[0].index()].in_play, "off Demdemeh");
    assert!(v.is_ally() && v.owner == SeatId(0) && !v.locked);
    assert!(e.state.has_subtype(w, zvtes::library::Label::Animal));
    assert_eq!(v.blood, 3, "its life counters");
    assert_eq!(e.state.current_bleed(w), 1);
    assert_eq!(e.state.base_strength(w) as i32 + e.state.lasting(w).1, 2);
    assert!(e
        .state
        .layers
        .iter()
        .filter(|l| l.source == ids[0])
        .all(|l| !e.state.layer_active(l)));
    e.state.minions[w.index()].blood = 2;
    assert_eq!(e.state.base_strength(w) as i32 + e.state.lasting(w).1, 1);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_own(o, d, 0)
    });
    let (mut e, d, _) = demdemeh_table(false, &[("Dog Pack", 0, 1)]);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_own(o, d, 0)
    });
}

/// Demdemeh's ally loses "any continuous effect affecting them (eg. bonus
/// from Tainted Spring)" [LSJ 20081213-3]: a Wolf Companion [ani] (2 life)
/// with Tainted Spring [ANI]'s additional life (`CardInPlay::extra_life`),
/// at 3 life counters: the ally keeps its 3 counters, its starting life its
/// printed 2, not 3.
#[test]
fn demdemehs_ally_loses_tainted_springs_additional_life() {
    let (mut e, d, ids) = demdemeh_table(true, &[("Wolf Companion", 0, 3)]);
    e.state.cards[ids[0].index()].extra_life = zvtes::state::Sparse(1);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(d),
            ability: 0,
            actor: None,
            target: Target::Card(ids[0]),
        },
    );
    let w = demdemeh_ally(&e);
    assert_eq!(e.state.minions[w.index()].blood, 3, "its life counters");
    assert_eq!(e.state.ally_body(w).map(|b| b.life), Some(2), "printed");
}

/// Demdemeh's ally: "keep their current life counters and remember their
/// original 'starting life' … The retainers' special abilities are lost,
/// as well as any continuous effect affecting them (eg. bonus from Tainted
/// Spring)" [LSJ 20051116-1] [LSJ 20081213-3]. A Wolf Companion [ani] (2
/// life) with a Tainted Spring boost's damage layer (`boost_retainer`'s),
/// at 2 life: its starting life its printed 2 — Vagabond Mystic finds it
/// unhurt at 2, hurt at 1 —, the boost's layer gone.
#[test]
fn demdemehs_ally_remembers_its_starting_life() {
    let (mut e, d, ids) = demdemeh_table(true, &[("Wolf Companion", 0, 2)]);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::RetainerDamageClose {
            n: 1,
            when: zvtes::cards::RetainerWhen::ALWAYS,
        },
        source: ids[0],
    });
    let mystic = e.add_ally(SeatId(0), find_library("Vagabond Mystic"));
    let id = e.state.minions[mystic.index()].ally_id().expect("card");
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(d),
            ability: 0,
            actor: None,
            target: Target::Card(ids[0]),
        },
    );
    let w = demdemeh_ally(&e);
    assert_eq!(e.state.ally_body(w).map(|b| b.life), Some(2));
    assert!(e
        .state
        .layers
        .iter()
        .filter(|l| l.source == ids[0])
        .all(|l| !e.state.layer_active(l)));
    let heal = Choice::UseAbility {
        source: AbilitySource::Card(id),
        ability: 0,
        actor: None,
        target: Target::Minion(w),
    };
    assert!(!request(&mut e).options.contains(&heal), "not hurt at 2");
    e.state.minions[w.index()].blood = 1;
    assert!(request(&mut e).options.contains(&heal), "hurt at 1");
}

/// Onaedo: "While /Otieno/ is ready, Onaedo gets +1 stealth during
/// diablerie actions." Any Methuselah's Otieno (rules-digest D11), now in
/// the pool: ready, +1; in torpor, none.
#[test]
fn onaedo_sneaks_into_diablerie_while_otieno_is_ready() {
    for (torpor, n) in [(false, 1), (true, 0)] {
        let mut e = game(2);
        let o = e.add_minion(SeatId(0), find_card("Onaedo"), 4, false);
        e.add_minion(SeatId(1), find_card("Otieno"), 4, torpor);
        let diablerie = zvtes::state::PrintedCtx {
            kind: Some(zvtes::decision::ActionKind::Diablerie),
            ..Default::default()
        };
        let stealth = e
            .state
            .printed_mod(o, zvtes::cards::Stat::Stealth, diablerie);
        assert_eq!(stealth, n, "torpor {torpor}");
    }
}
