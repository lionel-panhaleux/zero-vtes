//! Round 33 (`ADDED_BATCHES`): Kindred Most Wanted and Tenth Anniversary
//! completed, track B: its combat cards.
//! Windows with a single option are elided, so the scripts only stop where
//! a real decision exists.

use zvtes::cards::{Lasting, Stat, Target};
use zvtes::combat::Range;
use zvtes::crypt::Title;
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_library, CardId, Expiry};

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

/// A bleeds, X blocks: their combat begins.
fn fight(e: &mut Engine, a: MinionId, x: MinionId) {
    pick(e, bleed(a));
    pick_when(e, block(x));
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

fn range(e: &Engine) -> Range {
    e.combat().expect("combat").range
}

/// The card named `name` in play, if any (not an effect-only stand-in).
fn card_in_play(e: &Engine, name: &str) -> Option<CardId> {
    let card = find_library(name);
    e.state
        .cards
        .iter()
        .position(|c| c.card == card && c.in_play && !c.effect_only)
        .map(|i| CardId(i as u32))
}

// ---- Backflip ---------------------------------------------------------------

/// Backflip: "Only usable at long range. Strike: dodge, with an optional
/// press." Close range: not offered. Long range (A's Flash [cel]
/// maneuver): A dodges; the press is offered in round 1, left unused while
/// X presses on (Nimble Feet), and gone in round 2 [TOM 19960521].
#[test]
fn backflip_dodges_at_long_range_with_a_press_this_round() {
    let (mut e, a, x) = duel(&[], &[]);
    hand(&mut e, 0, &["Backflip"]);
    fight(&mut e, a, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));

    let (mut e, a, x) = duel(&["cel"], &["cel"]);
    hand(&mut e, 0, &["Backflip", "Flash"]);
    hand(&mut e, 1, &["Nimble Feet"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(1, 0, a));
    request(&mut e);
    assert_eq!(range(&e), Range::Long);
    pick_when(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert!(req.options.contains(&Choice::UsePress), "round 1's press");
    e.choose(index_of(&req, Choice::Pass));
    pick_when(&mut e, play(0, 0, x));
    while e.combat().is_some_and(|c| c.round == 1) {
        let req = request(&mut e);
        e.choose(fallback(&req));
    }
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::UsePress,
    );
}

// ---- Dead Hand --------------------------------------------------------------

/// Dead Hand [nec]: "Strike: hand strike at +1 damage." "Additional damage
/// inherits all of the properties of the base damage." [TOM 19960225]: A's
/// hands aggravated, X faces 2 aggravated damage, read by X's Ignore the
/// Searing Flames [ser] (offered only for an aggravated strike) and
/// burned as 2 normal.
#[test]
fn dead_hand_hand_strike_inherits_aggravated_hands() {
    let (mut e, a, x) = duel(&["nec"], &["ser"]);
    e.state.minions[a.index()]
        .timed
        .push((Lasting::AggravatedHands, Expiry::Combat, SeatId(0)));
    hand(&mut e, 0, &["Dead Hand"]);
    hand(&mut e, 1, &["Ignore the Searing Flames"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2, "2 aggravated taken as normal");
    assert!(!torpid(&e, x));
}

/// Dead Hand [NEC]: "the damage from this strike cannot be prevented by
/// cards requiring Fortitude [for]": "The opponent cannot play prevention
/// cards of that discipline (eg. for cycling)." [LSJ 20050628] X's Soak
/// [for] is never offered against the landed strike; its 2 damage lands.
#[test]
fn dead_hand_superior_bars_fortitude_prevention() {
    let (mut e, a, x) = duel(&["nec", "nec"], &["for"]);
    hand(&mut e, 0, &["Dead Hand"]);
    hand(&mut e, 1, &["Soak"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
    assert_eq!(blood(&e, x), 4 - 2);
}

// ---- Gleam of Red Eyes ------------------------------------------------------

/// Gleam of Red Eyes: "[pro] Press. [PRO] Maneuver." [PRO] in determine
/// range: the range long, no hand strike lands; [pro] in the press step: a
/// second round, at close range, both hand strikes landing.
#[test]
fn gleam_of_red_eyes_presses_or_maneuvers() {
    let (mut e, a, x) = duel(&["pro", "pro"], &[]);
    hand(&mut e, 0, &["Gleam of Red Eyes", "Gleam of Red Eyes"]);
    fight(&mut e, a, x);
    let req = expect(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(
        !req.options.contains(&play(0, 0, a)),
        "no press before range"
    );
    e.choose(index_of(&req, play(0, 1, a)));
    request(&mut e);
    assert_eq!(range(&e), Range::Long);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 1, 4 - 1), "round 2 only");
}

// ---- Vanish from the Mind's Eye ---------------------------------------------

/// Vanish from the Mind's Eye: "[obf] Press, only usable to end combat.
/// [OBF] Press." [obf] is not offered to continue, only against X's press
/// to continue (Nimble Feet); [OBF] continues.
#[test]
fn vanish_from_the_minds_eye_ends_combat_only_at_basic() {
    let (mut e, a, x) = duel(&["obf"], &[]);
    hand(&mut e, 0, &["Vanish from the Mind's Eye"]);
    fight(&mut e, a, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));

    let (mut e, a, x) = duel(&["obf"], &["cel"]);
    hand(&mut e, 0, &["Vanish from the Mind's Eye"]);
    hand(&mut e, 1, &["Nimble Feet"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 1, 4 - 1), "one round");

    let (mut e, a, x) = duel(&["obf", "obf"], &[]);
    hand(&mut e, 0, &["Vanish from the Mind's Eye"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 2, 4 - 2), "two rounds");
}

/// Drive the game to seat 0's minion phase: each request answered with the
/// first of `prefer` it offers, else a pass (or hand strike); `check` sees
/// every request on the way with the combat's round (0 out of combat).
fn drive(
    e: &mut Engine,
    prefer: &[Choice],
    mut check: impl FnMut(&zvtes::decision::DecisionRequest, u8),
) {
    loop {
        let req = request(e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            return;
        }
        check(&req, e.combat().map_or(0, |c| c.round));
        let i = prefer
            .iter()
            .find_map(|p| req.options.iter().position(|o| o == p))
            .unwrap_or_else(|| fallback(&req));
        e.choose(i);
    }
}

// ---- Leathery Hide ----------------------------------------------------------

/// Leathery Hide: "Prevent four non-aggravated damage from the opposing
/// minion's strike." A's 5-damage hand strike: X, a Gangrel antitribu,
/// prevents 4, 1 lands. "Cannot be used to prevent aggravated damage, even
/// if the minion treats them as normal damage (eg. Skin of Night)." [LSJ
/// 20040812-2] [PIB 20130327]: A's hands aggravated, X under Skin of Night
/// [for]: never offered.
#[test]
fn leathery_hide_prevents_normal_strike_damage_never_aggravated() {
    let (mut e, a, _) = duel(&[], &[]);
    let x = e.add_minion(SeatId(1), plain("Gangrel antitribu", 0), 4, false);
    e.state.minions[a.index()]
        .timed
        .push((Lasting::Strength(4), Expiry::Combat, SeatId(0)));
    hand(&mut e, 1, &["Leathery Hide"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 1);

    let (mut e, a, _) = duel(&[], &[]);
    let x = e.add_minion(SeatId(1), plain("Gangrel antitribu", 0), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["for"];
    e.state.minions[a.index()]
        .timed
        .push((Lasting::AggravatedHands, Expiry::Combat, SeatId(0)));
    hand(&mut e, 1, &["Leathery Hide", "Skin of Night"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(1, 0, x));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
    assert_eq!(blood(&e, x), 4 - 1, "the aggravated point taken as normal");
}

// ---- Skin of the Adder ------------------------------------------------------

/// Skin of the Adder [SER]: "This vampire can prevent 1 damage each round
/// … and this vampire's hand strikes inflict +1 damage this combat." A
/// prevents X's hand strike and deals 2; a second copy is not offered
/// ("only one … each combat").
#[test]
fn skin_of_the_adder_prevents_each_round_and_adds_hand_damage() {
    let (mut e, a, x) = duel(&["ser", "ser"], &[]);
    hand(&mut e, 0, &["Skin of the Adder", "Skin of the Adder"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    let mut second = false;
    drive(&mut e, &[Choice::UsePrevent], |req, _| {
        second |= req.seat == SeatId(0)
            && (req.options.contains(&play(0, 0, a)) || req.options.contains(&play(0, 1, a)));
    });
    assert!(!second, "once each combat");
    assert_eq!(blood(&e, a), 4 - 1, "its cost; X's strike prevented");
    assert_eq!(blood(&e, x), 4 - 2);
}

// ---- Silence of Death -------------------------------------------------------

/// Silence of Death [qui]: "This vampire gets first strike with hand and
/// melee-weapon strikes this round." A's hand strike sends X (no blood) to
/// torpor first: X's strike never resolves. A's ranged Earth Swords [myt]
/// gets none: X's hand strike lands.
#[test]
fn silence_of_death_first_strike_only_with_hand_or_melee() {
    let (mut e, a, x) = duel(&["qui"], &[]);
    e.state.minions[x.index()].blood = 0;
    hand(&mut e, 0, &["Silence of Death"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(torpid(&e, x));
    assert_eq!(blood(&e, a), 4 - 1, "its cost only");

    let (mut e, a, x) = duel(&["qui", "myt"], &[]);
    e.state.minions[x.index()].blood = 0;
    hand(&mut e, 0, &["Silence of Death", "Earth Swords"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(torpid(&e, x));
    assert_eq!(blood(&e, a), 4 - 1 - 1, "its cost and X's strike");
}

// ---- Immense Size -----------------------------------------------------------

/// Immense Size [VIC]: "The opposing minion cannot play grapple cards …
/// this combat … with 1 optional press." X's Immortal Grapple is never
/// offered, in round 1 or round 2 (A's press used to continue, round 1
/// only [TOM 19960521]).
#[test]
fn immense_size_bars_opposing_grapples_for_the_combat() {
    let (mut e, a, x) = duel(&["vic", "vic"], &["pot"]);
    hand(&mut e, 0, &["Immense Size"]);
    hand(&mut e, 1, &["Immortal Grapple"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    let mut rounds = 0;
    let mut presses = Vec::new();
    drive(&mut e, &[Choice::UsePress], |req, round| {
        rounds = rounds.max(round);
        assert!(!req.options.contains(&play(0, 0, x)), "{req:?}");
        if req.seat == SeatId(0) && req.options.contains(&Choice::UsePress) {
            presses.push(round);
        }
    });
    assert_eq!((rounds, presses), (1, vec![1]), "round 2's windows elided");
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 2, 4 - 2), "two rounds");
}

// ---- Grasp of the Python ----------------------------------------------------

/// Grasp of the Python [SER]: "if another round of combat occurs, that
/// round is at close range (skip the determine range step for that round)
/// and strikes that are not hand strikes cannot be used that round (by
/// either combatant)." A's grapple strike, its press used to continue
/// (round 1 only: none to end, [TOM 19960521]); round 2: no determine
/// range, so X's Gleam of Red Eyes [PRO] maneuver is never offered
/// [RTR 19970630] [ANK 20180720], nor A's or X's Earth Swords.
#[test]
fn grasp_of_the_python_superior_fixes_next_round_close_and_hand_only() {
    let (mut e, a, x) = duel(&["ser", "ser", "myt"], &["pro", "pro", "myt"]);
    hand(&mut e, 0, &["Grasp of the Python", "Earth Swords"]);
    hand(&mut e, 1, &["Gleam of Red Eyes", "Earth Swords"]);
    fight(&mut e, a, x);
    let req = expect(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    e.choose(index_of(&req, Choice::Pass));
    pick_when(&mut e, play(0, 1, a));
    let mut rounds = 0;
    drive(&mut e, &[Choice::UsePress], |req, round| {
        rounds = rounds.max(round);
        if round == 2 {
            assert_ne!(req.window, WindowLabel::CombatDetermineRange);
            assert!(!req.options.contains(&play(1, 0, a)), "{req:?}");
            assert!(!req.options.contains(&play(1, 0, x)), "{req:?}");
        }
    });
    assert_eq!(rounds, 2);
    assert_eq!(blood(&e, a), 4 - 1 - 1 - 1, "its cost and two hand strikes");
}

/// Grasp of the Python [SER], dodged: "if another round of combat occurs,
/// that round is at close range … and strikes that are not hand strikes
/// cannot be used that round (by either combatant)": X's Side Strike dodge
/// does not free it, "It applies to the round more than to a minion"
/// (owner, 2026-11-19; a Rules Director ruling desirable). A presses on;
/// round 2: no determine range, neither's Earth Swords offered.
#[test]
fn grasp_of_the_python_dodged_still_binds_both_next_round() {
    let (mut e, a, x) = duel(&["ser", "ser", "myt"], &["cel", "myt"]);
    hand(&mut e, 0, &["Grasp of the Python", "Earth Swords"]);
    hand(&mut e, 1, &["Side Strike", "Earth Swords"]);
    e.notes = Some(Vec::new());
    fight(&mut e, a, x);
    drive(
        &mut e,
        &[play(0, 1, a), play(0, 0, x), Choice::UsePress],
        |req, round| {
            if round == 2 {
                assert_ne!(req.window, WindowLabel::CombatDetermineRange);
                assert!(!req.options.contains(&play(0, 0, a)), "{req:?}");
                assert!(!req.options.contains(&play(0, 0, x)), "{req:?}");
            }
        },
    );
    assert!(ash_has(&e, 1, "Side Strike"));
    assert_eq!(strike_ranges(&mut e), [Range::Close, Range::Close]);
    assert_eq!(blood(&e, a), 4 - 1 - 1, "its cost and X's round 2 hand");
}

/// Grasp of the Python [SER]: "If another effect was already setting that
/// round's range (e.g. Neutral Guard), that effect has priority." [PIB
/// 20120214] Charge of the Buffalo [spi] makes the first round close, before
/// A's Neutral Guard [obe] sets every round long: X's grapple presses on,
/// and the second round is long.
#[test]
fn grasp_of_the_python_yields_the_next_range_to_neutral_guard() {
    let (mut e, a, x) = duel(&["obe", "spi"], &["ser", "ser"]);
    hand(&mut e, 0, &["Charge of the Buffalo", "Neutral Guard"]);
    hand(&mut e, 1, &["Grasp of the Python"]);
    e.notes = Some(Vec::new());
    pick(&mut e, play_on(0, 1, a, Target::Minion(x)));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 1, x));
    pick_when(&mut e, Choice::UsePress);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(strike_ranges(&mut e), [Range::Close, Range::Long]);
}

/// The range of each strike pair noted.
fn strike_ranges(e: &mut Engine) -> Vec<Range> {
    noted_combats(e)
        .into_iter()
        .filter_map(|n| match n {
            zvtes::combat::CombatNote::Strikes { range, .. } => Some(range),
            _ => None,
        })
        .collect()
}

// ---- Ignore the Searing Flames ----------------------------------------------

/// Ignore the Searing Flames [ser]: "Only usable if the opposing minion's
/// strike inflicts aggravated damage on this vampire. This vampire treats
/// all aggravated damage from that strike as normal damage." A's 2
/// aggravated hand damage: X burns 2 blood, out of torpor; the points stay
/// aggravated for prevention, so X's Leathery Hide is never offered (LSJ
/// 20040812-2). Unplayed, X goes to torpor.
#[test]
fn ignore_the_searing_flames_takes_the_strikes_aggravated_damage_as_normal() {
    for played in [true, false] {
        let (mut e, a, _) = duel(&[], &[]);
        let x = e.add_minion(SeatId(1), plain("Gangrel antitribu", 0), 4, false);
        e.state.minions[x.index()].gained_levels = vec!["ser"];
        let m = &mut e.state.minions[a.index()];
        m.timed
            .push((Lasting::AggravatedHands, Expiry::Combat, SeatId(0)));
        m.timed
            .push((Lasting::Strength(1), Expiry::Combat, SeatId(0)));
        hand(&mut e, 1, &["Ignore the Searing Flames", "Leathery Hide"]);
        fight(&mut e, a, x);
        if played {
            pick_when(&mut e, play(0, 0, x));
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
            assert_eq!(blood(&e, x), 4 - 2);
            assert!(!torpid(&e, x));
        } else {
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(torpid(&e, x));
        }
    }
}

/// Ignore the Searing Flames [dai]: "Prevent all aggravated damage from the
/// opposing minion's strike." "Cannot be used if there is no damage to
/// prevent." [LSJ 20001114]: against a normal hand strike, never offered;
/// against 2 aggravated, all prevented.
#[test]
fn ignore_the_searing_flames_prevents_only_aggravated_strike_damage() {
    let (mut e, a, x) = duel(&[], &["dai"]);
    hand(&mut e, 1, &["Ignore the Searing Flames"]);
    fight(&mut e, a, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, x));
    assert_eq!(blood(&e, x), 4 - 1);

    let (mut e, a, x) = duel(&[], &["dai"]);
    let m = &mut e.state.minions[a.index()];
    m.timed
        .push((Lasting::AggravatedHands, Expiry::Combat, SeatId(0)));
    m.timed
        .push((Lasting::Strength(1), Expiry::Combat, SeatId(0)));
    hand(&mut e, 1, &["Ignore the Searing Flames"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4);
    assert!(!torpid(&e, x));
}

/// Ignore the Searing Flames [DAI]: "Burn 1 blood to be immune to
/// aggravated damage this round." "Immune means damage is not inflicted,
/// even if non-preventable or outside combat." [LSJ 20010627] [LSJ
/// 20010629] [LSJ 20090304-1]: A's 2 aggravated hand damage and X's own
/// Breath of the Dragon [vic]'s 1 aggravated at close range, none landing;
/// X burned its 1 blood.
#[test]
fn ignore_the_searing_flames_superior_immunity_through_every_source() {
    let (mut e, a, x) = duel(&[], &["dai", "dai", "vic"]);
    let m = &mut e.state.minions[a.index()];
    m.timed
        .push((Lasting::AggravatedHands, Expiry::Combat, SeatId(0)));
    m.timed
        .push((Lasting::Strength(1), Expiry::Combat, SeatId(0)));
    hand(
        &mut e,
        1,
        &["Ignore the Searing Flames", "Breath of the Dragon"],
    );
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, x));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 1 - 1, "the burn and the Breath's cost");
    assert!(!torpid(&e, x));
    assert!(torpid(&e, a), "A took the Breath's aggravated damage");
}

/// Ignore the Searing Flames [DAI]: "The card cannot be played if the
/// minion cannot afford to burn the blood." [ANK 20210226]: X with no
/// blood is never offered it.
#[test]
fn ignore_the_searing_flames_superior_needs_the_blood() {
    let (mut e, a, x) = duel(&[], &["dai", "dai"]);
    e.state.minions[x.index()].blood = 0;
    hand(&mut e, 1, &["Ignore the Searing Flames"]);
    fight(&mut e, a, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 2, x));
}

// ---- Mercy for Seth ---------------------------------------------------------

/// Mercy for Seth [nec]: "Strike: put this card on the opposing minion …
/// During the attached minion's unlock phase, they take 1 unpreventable
/// environmental damage. They can burn this card as a +1 stealth action."
/// The card itself goes on X (out of the ash heap, rules-qa 47.1); X
/// loses 1 blood at its unlock; X burns it with a +1 stealth action, to
/// its owner's ash heap.
#[test]
fn mercy_for_seth_goes_on_the_opposing_vampire_and_hurts_each_unlock() {
    let (mut e, a, x) = duel(&["nec"], &[]);
    hand(&mut e, 0, &["Mercy for Seth"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let mercy = card_in_play(&e, "Mercy for Seth").expect("in play");
    assert_eq!(e.state.cards[mercy.index()].host, Some(x));
    assert!(!ash_has(&e, 0, "Mercy for Seth"));
    assert_eq!(blood(&e, x), 4, "no damage at [nec]");
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 1, "its unlock phase");
    let burn = uses(&req, mercy.0)[0];
    e.choose(index_of(&req, burn));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(card_in_play(&e, "Mercy for Seth").is_none());
    assert!(ash_has(&e, 0, "Mercy for Seth"), "its owner's ash heap");
}

/// Mercy for Seth: "If the attached minion is a mortal, they are burned."
/// Loyal Street Gang, a mortal ally, blocks and is struck: burned, the
/// card with it. A dodge (Read Intentions [aus]): nothing put on, the card
/// in the ash heap (rules-qa 21.1).
#[test]
fn mercy_for_seth_burns_a_mortal_ally() {
    let (mut e, a, _) = duel(&["nec"], &[]);
    let gang = e.add_ally(SeatId(1), find_library("Loyal Street Gang"));
    hand(&mut e, 0, &["Mercy for Seth"]);
    fight(&mut e, a, gang);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(burned(&e, gang));
    assert!(card_in_play(&e, "Mercy for Seth").is_none());

    let (mut e, a, x) = duel(&["nec"], &["aus", "aus"]);
    hand(&mut e, 0, &["Mercy for Seth"]);
    hand(&mut e, 1, &["Read Intentions"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(card_in_play(&e, "Mercy for Seth").is_none());
    assert!(ash_has(&e, 0, "Mercy for Seth"));
}

/// Mercy for Seth [NEC]: "As above, and this strike inflicts 1 damage on
/// the opposing minion." X loses 1 blood, the card on it.
#[test]
fn mercy_for_seth_superior_inflicts_one_damage() {
    let (mut e, a, x) = duel(&["nec", "nec"], &[]);
    hand(&mut e, 0, &["Mercy for Seth"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 1);
    let mercy = card_in_play(&e, "Mercy for Seth").expect("in play");
    assert_eq!(e.state.cards[mercy.index()].host, Some(x));
}

// ---- Rapid Thought ----------------------------------------------------------

/// Rapid Thought [CEL]: "Only usable during the choose-strike step, and
/// only if this vampire would choose his or her strike first. Instead, the
/// opposing minion chooses his or her strike first." A, choosing first,
/// plays it: X's choice comes next, its own Rapid Thought not offered (it
/// no longer would choose first after A); then A's.
#[test]
fn rapid_thought_superior_makes_the_opponent_choose_first() {
    let (mut e, a, x) = duel(&["cel", "cel", "myt"], &["cel", "cel", "myt"]);
    hand(&mut e, 0, &["Rapid Thought", "Earth Swords"]);
    hand(&mut e, 1, &["Rapid Thought", "Earth Swords"]);
    fight(&mut e, a, x);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    e.choose(index_of(&req, play(0, 1, a)));
    let req = expect(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(!req.options.contains(&play(0, 1, x)), "{:?}", req.options);
    e.choose(index_of(&req, Choice::HandStrike));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    e.choose(index_of(&req, Choice::HandStrike));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 1, 4 - 1));
}

/// Rapid Thought [cel]: "Maneuver or press." A maneuvers to long range,
/// then presses for a second round with the other copy.
#[test]
fn rapid_thought_maneuvers_or_presses() {
    let (mut e, a, x) = duel(&["cel"], &[]);
    hand(&mut e, 0, &["Rapid Thought", "Rapid Thought"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(range(&e), Range::Long);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 1, 4 - 1), "round 2 only");
}

// ---- Shared Strength --------------------------------------------------------

/// Shared Strength: "Only usable by a ready vampire not involved in combat.
/// [for] Choose a vampire in combat. For the remainder of combat, that
/// vampire may play cards that require Fortitude [for] as if he or she has
/// the basic level of Fortitude. [FOR] As above, but … superior." B, out
/// of the combat, lends it to A: after [for], Soak [for] is offered to A
/// and not [FOR]; after [FOR], both. A has no Fortitude for any other
/// reader.
#[test]
fn shared_strength_lets_a_combatant_play_fortitude() {
    for superior in [false, true] {
        let (mut e, a, x) = duel(&[], &[]);
        let b = MinionId(1);
        e.state.minions[b.index()].gained_levels = vec!["for", "for"];
        hand(&mut e, 0, &["Shared Strength", "Soak"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play_on(0, u8::from(superior), b, Target::Minion(a)));
        assert_eq!(e.state.discipline_level(a, "for"), 0);
        let req = until(&mut e, SeatId(0), WindowLabel::CombatPrevention);
        assert!(req.options.contains(&play(0, 0, a)));
        assert_eq!(req.options.contains(&play(0, 1, a)), superior);
    }
}

// ---- Tranquility ------------------------------------------------------------

/// Tranquility [DOM]: "Cancel a frenzy card as it is played (no cost is
/// paid) … and the target of the frenzy card gets an optional press, only
/// usable to end combat." X's Terror Frenzy on A, canceled by B out of the
/// combat: X keeps its blood; A, offered the press to end only once X
/// presses on, ends the combat after one round.
#[test]
fn tranquility_cancels_a_frenzy_card_and_presses_its_target() {
    let (mut e, a, x) = duel(&[], &["ani", "cel"]);
    let b = MinionId(1);
    e.state.minions[b.index()].gained_levels = vec!["dom", "dom"];
    hand(&mut e, 0, &["Tranquility"]);
    hand(&mut e, 1, &["Terror Frenzy", "Nimble Feet"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 1, b));
    pick_when(&mut e, play(0, 0, x));
    let req = expect(&mut e, SeatId(0), WindowLabel::CombatPress);
    e.choose(index_of(&req, Choice::UsePress));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 1, "no cost; A's strike");
    assert_eq!(blood(&e, a), 4 - 1, "one round");
    assert!(ash_has(&e, 1, "Terror Frenzy"));
}

// ---- Fear of the Void Below -------------------------------------------------

/// Fear of the Void Below [dai]: "This combat, each round defaults to long
/// range, and this vampire gets 1 optional press each round, only usable
/// to end combat." Round 1 long, A's press offered only against X's press
/// to continue (Nimble Feet), passed; round 2 long again until X's
/// maneuver (Gleam of Red Eyes [PRO]) closes it; A's press again, used.
#[test]
fn fear_of_the_void_below_defaults_every_round_to_long_with_an_ending_press() {
    let (mut e, a, x) = duel(&["dai"], &["cel", "pro", "pro"]);
    hand(&mut e, 0, &["Fear of the Void Below"]);
    hand(
        &mut e,
        1,
        &["Nimble Feet", "Nimble Feet", "Gleam of Red Eyes"],
    );
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    request(&mut e);
    assert_eq!(range(&e), Range::Long);
    pick_when(&mut e, play(0, 0, x));
    let req = expect(&mut e, SeatId(0), WindowLabel::CombatPress);
    e.choose(index_of(&req, Choice::Pass));
    let req = expect(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    assert_eq!(
        (e.combat().expect("combat").round, range(&e)),
        (2, Range::Long)
    );
    e.choose(index_of(&req, play(1, 1, x)));
    pick_when(&mut e, play(0, 0, x));
    let req = expect(&mut e, SeatId(0), WindowLabel::CombatPress);
    e.choose(index_of(&req, Choice::UsePress));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1 - 1, "its cost and round 2's strike");
    assert_eq!(blood(&e, x), 4 - 1, "round 2's strike only");
}

/// Fear of the Void Below [dai]: "Affects the range immediately"; "the
/// range default is long (e.g. [dai] Fear of the Void Below), [TEM]
/// Outside the Hourglass triggers the Flamethrower" [LSJ 20100709]: A's
/// Improvised Flamethrower burns, A takes its aggravated damage.
#[test]
fn fear_of_the_void_below_defaults_long_and_triggers_the_flamethrower() {
    let (mut e, a, x) = duel(&["dai"], &["tem", "tem"]);
    let flame = in_play(&mut e, 0, "Improvised Flamethrower", Some(a));
    hand(&mut e, 0, &["Fear of the Void Below"]);
    hand(&mut e, 1, &["Outside the Hourglass"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 3, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[flame.index()].in_play, "burned");
    assert!(torpid(&e, a), "its aggravated damage");
}

/// Fear of the Void Below [DAI]: "the opposing minion's controller discards
/// 1 card at random each round during normal strike resolution." X dodges
/// (Read Intentions [aus]): one of its two Form of Mist goes to the ash
/// heap all the same. X's strike: combat ends (Bliss [DOM]) resolves first:
/// no discard.
#[test]
fn fear_of_the_void_below_superior_discards_at_random_through_a_dodge() {
    let (mut e, a, x) = duel(&["dai", "dai"], &["aus", "aus"]);
    hand(&mut e, 0, &["Fear of the Void Below"]);
    hand(
        &mut e,
        1,
        &["Read Intentions", "Form of Mist", "Form of Mist"],
    );
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].hand.len(), 1);
    let mist = find_library("Form of Mist");
    assert_eq!(
        e.state.seats[1].ash.iter().filter(|&&c| c == mist).count(),
        1
    );

    let (mut e, a, x) = duel(&["dai", "dai"], &["dom", "dom"]);
    hand(&mut e, 0, &["Fear of the Void Below"]);
    hand(&mut e, 1, &["Bliss", "Form of Mist"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    pick_when(&mut e, play(0, 3, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].hand.len(), 1, "no discard");
}

/// Fear of the Void Below: "A vampire can play only one Fear of the Void
/// Below each combat." [dai] played before range: the other copy's [obf]
/// maneuver is never offered.
#[test]
fn fear_of_the_void_below_once_each_combat() {
    let (mut e, a, x) = duel(&["dai", "obf"], &[]);
    hand(
        &mut e,
        0,
        &["Fear of the Void Below", "Fear of the Void Below"],
    );
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

// ---- Optional presses -------------------------------------------------------

/// "The optional press can only be used during the current round." [TOM
/// 19960521] Immense Size [VIC], Grasp of the Python [ser] (to continue),
/// Tranquility [DOM] (to end, on the canceled Terror Frenzy's target): A's
/// press offered in round 1, left unused while X presses on twice (Nimble
/// Feet), never offered in rounds 2 and 3.
#[test]
fn round_33_optional_presses_are_this_rounds_only() {
    for card in ["Immense Size", "Grasp of the Python", "Tranquility"] {
        let (mut e, a, x) = duel(&["vic", "vic", "ser"], &["cel", "ani"]);
        let b = MinionId(1);
        e.state.minions[b.index()].gained_levels = vec!["dom", "dom"];
        hand(&mut e, 0, &[card]);
        hand(&mut e, 1, &["Nimble Feet", "Nimble Feet", "Terror Frenzy"]);
        fight(&mut e, a, x);
        match card {
            "Immense Size" => pick_when(&mut e, play(0, 1, a)),
            "Grasp of the Python" => pick_when(&mut e, play(0, 0, a)),
            _ => {
                pick_when(&mut e, play(2, 0, x));
                pick_when(&mut e, play(0, 1, b));
            }
        }
        let mut offered = false;
        drive(&mut e, &[play(0, 0, x)], |req, round| {
            if req.seat == SeatId(0) && req.options.contains(&Choice::UsePress) {
                assert_eq!(round, 1, "{card}");
                offered = true;
            }
        });
        assert!(offered, "{card}");
        assert_eq!(blood(&e, x), 4 - 3, "{card}: three rounds");
    }
}

/// Fear of the Void Below [dai]: "Affects the range immediately (eg.
/// Outside the Hourglass and Alpha Glint could be played, but not Disarm
/// because the range is long)." [LSJ 20100709] A's Alpha Glint [ANI][FOR]
/// still ends the combat before range after it. X's Disarm [pot], its 2R
/// Earth Swords [myt] outdoing A's hand strike: offered at the default
/// close range, never once Fear made it long.
#[test]
fn fear_of_the_void_below_affects_the_range_immediately() {
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
    e.state.minions[a.index()].gained_levels = vec!["dai", "ani", "ani", "for", "for"];
    hand(&mut e, 0, &["Fear of the Void Below", "Alpha Glint"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 1 - 1, 4), "no strike");

    for fear in [false, true] {
        let (mut e, a, x) = duel(&["dai"], &["myt", "pot"]);
        hand(&mut e, 0, &["Fear of the Void Below"]);
        hand(&mut e, 1, &["Disarm", "Earth Swords"]);
        fight(&mut e, a, x);
        if fear {
            pick_when(&mut e, play(0, 1, a));
        }
        pick_when(&mut e, play(1, 1, x));
        let disarm = play(0, 0, x);
        let offered = offered_before(&mut e, disarm, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(offered, !fear);
    }
}

/// Blessed Blade [val] cancels Grasp of the Python, a grapple, as it is
/// played: "The cost for a canceled non-action-card is still paid" [ANK
/// 20260216]. X's 1 blood is burned, its strike then a hand strike.
#[test]
fn blessed_blade_cancels_grasp_of_the_python_its_cost_paid() {
    let (mut e, a, x) = duel(&["val"], &["ser"]);
    hand(&mut e, 0, &["Blessed Blade"]);
    hand(&mut e, 1, &["Grasp of the Python"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Grasp of the Python"));
    assert_eq!(blood(&e, x), 4 - 1 - 1, "its cost and A's hand strike");
    assert_eq!(blood(&e, a), 4 - 1, "X's hand strike");
}

/// Ignore the Searing Flames [ser]'s points land as normal: X, with the
/// blood to mend them, is not going into torpor, so A's Decapitate [POT]
/// ("only usable when the opposing vampire is going into torpor") is never
/// offered; unplayed, it is (the as-normal reading shared by the
/// going-to-torpor step and the damage, `Side::landing`).
#[test]
fn searing_flames_as_normal_damage_is_not_going_to_torpor() {
    for played in [true, false] {
        let (mut e, a, x) = duel(&["pot", "pot"], &["ser"]);
        let m = &mut e.state.minions[a.index()];
        m.timed
            .push((Lasting::AggravatedHands, Expiry::Combat, SeatId(0)));
        m.timed
            .push((Lasting::Strength(1), Expiry::Combat, SeatId(0)));
        hand(&mut e, 0, &["Decapitate"]);
        hand(&mut e, 1, &["Ignore the Searing Flames"]);
        fight(&mut e, a, x);
        if played {
            pick_when(&mut e, play(0, 0, x));
        }
        let offered = offered_before(&mut e, play(0, 2, a), SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(offered, !played);
    }
}

// ==== Action cards ===========================================================

// ---- Helpers (action cards) -------------------------------------------------

/// Seat 0's A and B, seat 1's X and Y: fixture vampires of capacity 4 to 8
/// matching `f`, 4 blood each.
fn four(f: impl Fn(&zvtes::crypt::CryptCard) -> bool + Copy) -> (Engine, [MinionId; 4]) {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity) && f(c);
    let a = e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    let b = e.add_minion(SeatId(0), textless(mid, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    let y = e.add_minion(SeatId(1), textless(mid, 3), 4, false);
    (e, [a, b, x, y])
}

/// Card `card` of the hand played by `actor` in `mode` at minion `t`.
fn play_at(card: u8, mode: u8, actor: MinionId, t: MinionId) -> Choice {
    play_on(card, mode, actor, Target::Minion(t))
}

/// The card `name` entered play (itself, not a stand-in) on `host`, out of
/// every ash heap: its id.
#[track_caller]
fn placed_on(e: &Engine, name: &str, host: Option<MinionId>) -> CardId {
    let card = find_library(name);
    assert!(
        e.state.seats.iter().all(|s| !s.ash.contains(&card)),
        "{name} in an ash heap"
    );
    let i = e
        .state
        .cards
        .iter()
        .position(|c| c.card == card && c.in_play && !c.effect_only && c.host == host)
        .unwrap_or_else(|| panic!("{name} not in play on {host:?}"));
    CardId(i as u32)
}

/// The uses offered of the card in play `card`.
fn card_uses(req: &zvtes::decision::DecisionRequest, card: CardId) -> Vec<Choice> {
    uses(req, card.0)
}

// ---- Blood Bond -------------------------------------------------------------

/// Blood Bond: "Put this card on any vampire. The vampire with this card
/// cannot block this acting vampire." A bonds X: X may still block B's
/// bleed, never A's next one (Y may).
#[test]
fn blood_bond_bars_its_vampire_from_blocking_the_acting_vampire() {
    let (mut e, [a, b, x, y]) = four(|_| true);
    hand(&mut e, 0, &["Blood Bond"]);
    pick(&mut e, play_at(0, 0, a, x));
    pick_when(&mut e, bleed(b));
    placed_on(&e, "Blood Bond", Some(x));
    let req = offering(&mut e, block(x));
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a));
    let req = offering(&mut e, block(y));
    assert!(!req.options.contains(&block(x)), "X bonded to A");
}

/// "[AUS] Only affects the rule restricting who gets to block an action
/// … Blood Bond … not circumvented." [RTR 19950413] [RTR 20020501] (the
/// same rule for Falcon's Eye, [LSJ 20020112], and Anneke, [LSJ 20041203]):
/// seat 2's bonded X is never
/// offered Eagle's Sight [AUS] against A's bleed of seat 1; Z is.
// Rulings: [RTR 19950413] [LSJ 20020112] [RTR 20020501]; [RTR 20020501]
// [LSJ 20041203].
#[test]
fn blood_bond_is_not_circumvented_by_eagles_sight() {
    let mut e = game(3);
    let aus = |n| {
        fixture_vampire(
            |c| c.disciplines.contains(&"AUS") && c.title == Title::None,
            n,
        )
    };
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, true);
    let x = e.add_minion(SeatId(2), aus(0), 4, false);
    let z = e.add_minion(SeatId(2), aus(1), 4, false);
    let bond = in_play(&mut e, 2, "Blood Bond", Some(x));
    e.state.cards[bond.index()].by = Some(a);
    hand(&mut e, 2, &["Eagle's Sight"]);
    pick(&mut e, bleed(a));
    let req = until(&mut e, SeatId(2), WindowLabel::ActionBlock);
    assert!(req.options.contains(&play(0, 1, z)));
    assert!(!req.options.contains(&play(0, 1, x)), "bonded to A");
}

// ---- Ritual Challenge -------------------------------------------------------

/// Ritual Challenge: "If this action succeeds, put this card on the acting
/// Gangrel. This vampire gets +1 strength. A vampire may have only one
/// Ritual Challenge card." The card on A itself; put on after the play, the
/// second copy still played by A, to the ash heap, A's strength +1 only
/// (rules-qa 99.2, D14).
#[test]
fn ritual_challenge_gives_its_gangrel_a_strength() {
    let (mut e, [a, ..]) = four(|c| c.clan == "Gangrel");
    let before = e.state.bearer_always(a, Stat::Strength);
    hand(&mut e, 0, &["Ritual Challenge", "Ritual Challenge"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    placed_on(&e, "Ritual Challenge", Some(a));
    assert_eq!(e.state.bearer_always(a, Stat::Strength), before + 1);
    to_minion_phase(&mut e, SeatId(0));
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Ritual Challenge"), "the second not put on");
    let card = find_library("Ritual Challenge");
    let on_a = e
        .state
        .cards
        .iter()
        .filter(|c| c.card == card && c.in_play && c.host == Some(a))
        .count();
    assert_eq!(on_a, 1);
    assert_eq!(e.state.bearer_always(a, Stat::Strength), before + 1);
}

// ---- Heart of Darkness ------------------------------------------------------

/// Heart of Darkness [SER]: X's aggravated hands hit A for 1 normal; A's
/// prevention used once that combat. "Any minion can burn this card as a
/// Ⓓ action": X burns it, to A's controller's ash heap.
#[test]
fn heart_of_darkness_takes_aggravated_as_normal_and_burns_by_action() {
    let (mut e, a, x) = duel(&["ser", "ser"], &[]);
    hand(&mut e, 0, &["Heart of Darkness"]);
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let heart = placed_on(&e, "Heart of Darkness", Some(a));
    assert_eq!(blood(&e, a), 4 - 2);
    // X attacks through a bleed A blocks; X's hands are aggravated.
    e.state.minions[x.index()]
        .timed
        .push((Lasting::AggravatedHands, Expiry::Combat, SeatId(1)));
    e.state.minions[a.index()].locked = false;
    pick(&mut e, bleed(x));
    pick_when(&mut e, block(a));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatPrevention);
    assert_eq!(card_uses(&req, heart).len(), 1);
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!torpid(&e, a), "aggravated taken as normal");
    assert_eq!(blood(&e, a), 4 - 2 - 1);
    // Y burns it.
    let y = zvtes::ids::MinionId(3);
    let burn = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(heart),
        ability: 1,
        actor: Some(y),
        target: Target::None,
    };
    pick(&mut e, burn);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Heart of Darkness"));
}

// ---- Concordance ------------------------------------------------------------

/// Concordance [ser]: "Bleed. If more than 1 pool is bled with this action,
/// ignore the excess." A bleed for 2 burns 1.
#[test]
fn concordance_bleeds_for_one_at_most() {
    let (mut e, [a, _, x, y]) = four(|_| true);
    e.state.minions[a.index()].gained_levels = vec!["ser"];
    e.state.minions[x.index()].in_torpor = true;
    e.state.minions[y.index()].in_torpor = true;
    e.state.minions[a.index()]
        .timed
        .push((Lasting::Bleed(1), Expiry::EndOfTurn, SeatId(0)));
    hand(&mut e, 0, &["Concordance"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 1, "the excess ignored");
}

/// Concordance [DAI] on an infernal A: +1 strength, -1 stealth, flight;
/// "Once played, it continues to apply even if the vampire stops being
/// Infernal" [LSJ 20090207]: A no longer infernal keeps them. "During your
/// influence phase, this vampire can burn 1 blood to unlock."
#[test]
fn concordance_superior_stays_after_the_vampire_stops_being_infernal() {
    let (mut e, [a, ..]) = four(|_| true);
    e.state.minions[a.index()].gained_levels = vec!["dai", "dai"];
    e.state.minions[a.index()].made_infernal = true;
    let before = e.state.bearer_always(a, Stat::Strength);
    hand(&mut e, 0, &["Concordance"]);
    pick(&mut e, play(0, 2, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Concordance", Some(a));
    assert_eq!(blood(&e, a), 4 - 1);
    // Infernal: A stays locked through its unlock phase (the pool not
    // burned), and unlocks for a blood in the influence phase.
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    let used = card_uses(&req, card);
    assert_eq!(used.len(), 1, "{:?}", req.options);
    e.choose(index_of(&req, used[0]));
    request(&mut e);
    assert!(!e.state.minions[a.index()].locked);
    assert_eq!(blood(&e, a), 4 - 1 - 1);
    e.state.minions[a.index()].made_infernal = false;
    assert_eq!(e.state.bearer_always(a, Stat::Strength), before + 1);
    assert_eq!(e.state.bearer_always(a, Stat::Stealth), -1);
    assert!(e.state.has_discipline(a, "flight"));
}

/// Concordance [DAI] by a vampire infernal through The Textbook Damnation
/// [LSJ 20090207]: the Textbook burned, the vampire no longer infernal
/// keeps Concordance's +1 strength, -1 stealth and flight.
#[test]
fn concordance_stays_after_the_textbook_damnation_burns() {
    let (mut e, [a, ..]) = four(|_| true);
    let book = in_play(&mut e, 0, "Textbook Damnation, The", Some(a));
    assert!(e.state.is_infernal(a) && e.state.has_discipline(a, "DAI"));
    let before = e.state.bearer_always(a, Stat::Strength);
    hand(&mut e, 0, &["Concordance"]);
    pick(&mut e, play(0, 2, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    placed_on(&e, "Concordance", Some(a));
    e.state.cards[book.index()].in_play = false;
    assert!(!e.state.is_infernal(a));
    assert_eq!(e.state.bearer_always(a, Stat::Strength), before + 1);
    assert_eq!(e.state.bearer_always(a, Stat::Stealth), -1);
    assert!(e.state.has_discipline(a, "flight"));
}

// ---- Consecration Rites -----------------------------------------------------

/// Consecration Rites: "Put this card on a location you control. You may
/// burn this card to cause an action directed at the location to fail."
/// X's Conceal [OBF] at the location: burned by its controller before any
/// block attempt, the action fails; the location stays.
#[test]
fn consecration_rites_fails_an_action_at_its_location() {
    let (mut e, [a, _, x, _]) = four(|_| true);
    e.state.minions[a.index()].granted_title = Some(Title::Archbishop("Paris"));
    e.state.minions[a.index()].sect_override = Some(zvtes::crypt::Sect::Sabbat);
    e.state.minions[x.index()].gained_levels = vec!["obf", "obf"];
    let place = in_play(&mut e, 0, "Rack, The", None);
    hand(&mut e, 0, &["Consecration Rites"]);
    hand(&mut e, 1, &["Conceal"]);
    pick(&mut e, play_on(0, 0, a, Target::Card(place)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let rites = e
        .state
        .cards
        .iter()
        .position(|c| c.card == find_library("Consecration Rites") && c.in_play && !c.effect_only)
        .map(|i| CardId(i as u32))
        .expect("on the location");
    assert_eq!(e.state.cards[rites.index()].on_card, Some(place));
    pick(&mut e, play_on(0, 1, x, Target::Card(place)));
    let req = offering(&mut e, ability(rites.0, 0));
    e.choose(index_of(&req, ability(rites.0, 0)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.cards[place.index()].in_play, "the action failed");
    assert!(ash_has(&e, 0, "Consecration Rites"));
}

// ---- Soul Decoration --------------------------------------------------------

/// Soul Decoration [OBF][VIC]: "During an action this vampire performs, they
/// can cancel a card requiring Auspex [aus] as it is played, and its cost
/// is not paid." Once each action: X's Eagle's Sight canceled, Y's not
/// cancelable; the bleed goes on.
#[test]
fn soul_decoration_cancels_an_auspex_card_once_each_action() {
    let (mut e, [a, _, x, y]) = four(|_| true);
    e.state.minions[a.index()].gained_levels = vec!["obf", "obf", "vic", "vic"];
    e.state.minions[x.index()].gained_levels = vec!["aus"];
    e.state.minions[y.index()].gained_levels = vec!["aus"];
    hand(&mut e, 0, &["Soul Decoration"]);
    hand(&mut e, 1, &["Eagle's Sight", "Eagle's Sight"]);
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Soul Decoration", Some(a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[a.index()]
        .timed
        .push((Lasting::Stealth(2), Expiry::EndOfTurn, SeatId(0)));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, x));
    let req = expect(&mut e, SeatId(0), WindowLabel::CardAsPlayed);
    let cancel = card_uses(&req, card);
    assert_eq!(cancel.len(), 1);
    e.choose(index_of(&req, cancel[0]));
    request(&mut e);
    assert!(ash_has(&e, 1, "Eagle's Sight"));
    pick_when(&mut e, block(y));
    pick_when(&mut e, play(0, 0, y));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        card_uses_choice(o, card)
    });
    assert_eq!(pool(&e)[1], 10 - 1);
}

fn card_uses_choice(o: &Choice, card: CardId) -> bool {
    matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), .. } if *c == card)
}

// ---- Fire Dance -------------------------------------------------------------

/// Fire Dance: "Once each combat, the vampire with this card may treat 1
/// aggravated damage inflicted on him or her as normal damage." X's
/// aggravated hands, 2 rounds: the first round's point as normal, the
/// second's aggravated (A sent to torpor with no blood).
#[test]
fn fire_dance_turns_one_aggravated_point_each_combat() {
    let (mut e, a, x) = duel(&[], &[]);
    e.state.minions[a.index()].sect_override = Some(zvtes::crypt::Sect::Sabbat);
    let dance = in_play(&mut e, 0, "Fire Dance", Some(a));
    e.state.minions[x.index()]
        .timed
        .push((Lasting::AggravatedHands, Expiry::Combat, SeatId(1)));
    e.state.minions[x.index()].gained_levels = vec!["cel"];
    hand(&mut e, 1, &["Nimble Feet"]);
    fight(&mut e, a, x);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatPrevention);
    let used = card_uses(&req, dance);
    assert_eq!(used.len(), 1);
    e.choose(index_of(&req, used[0]));
    request(&mut e);
    assert_eq!(blood(&e, a), 4 - 1, "as normal");
    // Round 2 by X's press: no second use.
    pick_when(&mut e, play(0, 1, x));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        card_uses_choice(o, dance)
    });
    assert!(torpid(&e, a), "round 2's aggravated point");
}

/// Fire Dance: "The target vampire is chosen when the action is announced."
/// [RTR 19970425] The Sabbat target in the play itself; a second copy not
/// offered at it ("only one").
#[test]
fn fire_dance_target_is_chosen_as_announced() {
    let (mut e, [a, b, ..]) = four(|_| true);
    e.state.minions[a.index()].granted_title = Some(Title::Priscus);
    for m in [a, b] {
        e.state.minions[m.index()].sect_override = Some(zvtes::crypt::Sect::Sabbat);
    }
    hand(&mut e, 0, &["Fire Dance", "Fire Dance"]);
    pick(&mut e, play_at(0, 0, a, b));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    placed_on(&e, "Fire Dance", Some(b));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&play_at(0, 0, a, a)));
    assert!(!req.options.contains(&play_at(0, 0, a, b)), "only one");
}

// ---- Red List ---------------------------------------------------------------

/// Red List: "Choose a ready ally or a ready younger vampire. The chosen
/// minion becomes Red List. That minion may not attempt to block this
/// action." X never offered a block of it, Y is; X Red List after.
#[test]
fn red_list_marks_a_younger_vampire_who_cannot_block_it() {
    let mut e = game(2);
    let cam = |c: &zvtes::crypt::CryptCard| c.sect == Some(zvtes::crypt::Sect::Camarilla);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| cam(c) && c.capacity >= 8, 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(
        SeatId(1),
        textless(|c| (4..=6).contains(&c.capacity), 1),
        4,
        false,
    );
    let y = e.add_minion(
        SeatId(1),
        textless(|c| (4..=6).contains(&c.capacity), 2),
        4,
        false,
    );
    e.state.minions[a.index()].granted_title = Some(Title::Primogen);
    hand(&mut e, 0, &["Red List"]);
    pick(&mut e, play_at(0, 0, a, x));
    let req = offering(&mut e, block(y));
    assert!(!req.options.contains(&block(x)));
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.is_red_list(x));
}

// ---- Tumnimos ---------------------------------------------------------------

/// Tumnimos: "it becomes a 2-capacity (non-unique) Ravnos of the same sect
/// as the acting Ravnos with basic Chimerstry [chi], and must hunt this
/// turn. You can put a master Discipline card from your hand on this new
/// vampire."
#[test]
fn tumnimos_becomes_a_young_ravnos_who_must_hunt() {
    let (mut e, [a, ..]) = four(|c| c.clan == "Ravnos" && c.capacity >= 5);
    hand(&mut e, 0, &["Tumnimos"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let new = zvtes::ids::MinionId(e.state.minions.len() as u32 - 1);
    let v = &e.state.minions[new.index()];
    assert!(v.in_play && v.is_vampire());
    assert_eq!(e.state.capacity_of(new), Some(2));
    assert_eq!(v.clan(), Some("Ravnos"));
    assert_eq!(e.state.sect_of(new), e.state.sect_of(a));
    assert!(e.state.has_discipline(new, "chi"));
    // It hunted in seat 0's minion phase, offered nothing else.
    assert!(e.state.minions[new.index()].hunted_turn.is_some());
    assert_eq!(blood(&e, new), 1);
    assert_eq!(blood(&e, a), 4 - 2);
}

// ---- Covenant of Blood ------------------------------------------------------

/// Covenant of Blood: a non-Camarilla vampire enters combat with a
/// Camarilla one; the same vampire burns the card the same turn ("The two
/// actions provided are not cross-restrictive" [LSJ 20100909]); a
/// Camarilla vampire's burn is at -1 stealth.
#[test]
fn covenant_of_blood_lets_non_camarilla_attack_camarilla() {
    use zvtes::crypt::Sect;
    let (mut e, [a, b, x, y]) = four(|_| true);
    for (m, s) in [
        (a, Sect::Sabbat),
        (b, Sect::Camarilla),
        (x, Sect::Camarilla),
        (y, Sect::Camarilla),
    ] {
        e.state.minions[m.index()].sect_override = Some(s);
    }
    let cov = in_play(&mut e, 0, "Covenant of Blood", None);
    let attack = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(cov),
        ability: 0,
        actor: Some(a),
        target: Target::Minion(x),
    };
    let burn_by = |m| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(cov),
        ability: 1,
        actor: Some(m),
        target: Target::None,
    };
    let req = request(&mut e);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::UseAbility { ability: 0, actor: Some(m), .. } if *m == b)),
        "B is Camarilla"
    );
    pick(&mut e, attack);
    // A acts again this turn (unlocked by hand): its burn still offered.
    e.state.minions[a.index()].locked = false;
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (3, 3), "hand strikes traded");
    assert!(req.options.contains(&burn_by(a)));
    e.choose(index_of(&req, burn_by(b)));
    request(&mut e);
    assert_eq!(e.action().expect("action").stealth(&e.state), -1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Covenant of Blood"));
}

// ---- Rastacourere -----------------------------------------------------------

/// Rastacourere on X, a prince: its title worth 1 vote less, -1 stealth on
/// its political actions, capacity -1.
#[test]
fn rastacourere_costs_a_prince_a_vote_a_stealth_and_a_capacity() {
    let (mut e, [a, _, x, _]) = four(|c| c.sect == Some(zvtes::crypt::Sect::Camarilla));
    e.state.minions[x.index()].granted_title = Some(Title::Prince("Paris"));
    let cap = e.state.capacity_of(x).unwrap();
    hand(&mut e, 0, &["Rastacourere"]);
    pick(&mut e, play_at(0, 0, a, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    placed_on(&e, "Rastacourere", Some(x));
    assert_eq!(pool(&e)[0], 10 - 1);
    assert_eq!(e.state.capacity_of(x), Some(cap - 1));
    hand(&mut e, 1, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, x));
    request(&mut e);
    use zvtes::decision::ActionKind::{Bleed, Political};
    assert_eq!(e.action().expect("action").kind, Political);
    assert_eq!(e.state.bearer_mod(x, Stat::Stealth, Political), -1);
    assert_eq!(e.state.bearer_mod(x, Stat::Stealth, Bleed), 0);
    to_referendum(&mut e);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, x), 2 - 1);
}

/// Rastacourere on a prince at full capacity: the blood above the reduced
/// capacity drains off by the rules (ANK 20180719-1, Barrenness).
#[test]
fn rastacourere_drains_a_full_prince_to_his_reduced_capacity() {
    let (mut e, [a, _, x, _]) = four(|c| c.sect == Some(zvtes::crypt::Sect::Camarilla));
    e.state.minions[x.index()].granted_title = Some(Title::Prince("Paris"));
    let cap = e.state.capacity_of(x).unwrap();
    e.state.minions[x.index()].blood = cap;
    hand(&mut e, 0, &["Rastacourere"]);
    pick(&mut e, play_at(0, 0, a, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    placed_on(&e, "Rastacourere", Some(x));
    assert_eq!(e.state.minions[x.index()].blood, cap - 1);
}

/// Rastacourere: "Does not affect the Prisci sub-referendum." [LSJ
/// 20041202] [ANK 20180307-1] A priscus with it still casts its ballot.
#[test]
fn rastacourere_spares_the_prisci_ballot() {
    let (mut e, [_, _, x, _]) = four(|_| true);
    e.state.minions[x.index()].granted_title = Some(Title::Priscus);
    e.state.minions[x.index()].sect_override = Some(zvtes::crypt::Sect::Sabbat);
    let ballots = e.state.ballots(x);
    in_play(&mut e, 0, "Rastacourere", Some(x));
    assert!(ballots > 0);
    assert_eq!(e.state.ballots(x), ballots);
}

/// The crypt card `name` (not advanced).
#[track_caller]
fn crypt_card(name: &str) -> u16 {
    zvtes::crypt::CRYPT
        .iter()
        .position(|c| c.name == name && !c.adv)
        .unwrap_or_else(|| panic!("no {name}")) as u16
}

/// The side of `m`'s standing cast, if any.
fn cast_side(e: &Engine, m: MinionId) -> Option<bool> {
    e.referendum()
        .expect("referendum")
        .records
        .iter()
        .find_map(|r| {
            (r.source == zvtes::referendum::Source::Vampire(m) && !r.canceled).then_some(r.favor)
        })
}

/// `caller` plays Kine Resources Contested (hand slot 0) up to the polling.
fn kine_polling(e: &mut Engine, caller: MinionId) {
    pick(e, play(0, 0, caller));
    loop {
        let req = request(e);
        if req.window == WindowLabel::ReferendumPolling {
            return;
        }
        e.choose(fallback(&req));
    }
}

// ---- Revelation of the Sire -------------------------------------------------

/// Revelation of the Sire: a younger Caitiff "is now considered to be a
/// member of the same clan as the acting vampire. He or she is no longer
/// considered Caitiff. His or her capacity increases by 1." The card burned
/// (by hand): Caitiff again, its capacity back.
#[test]
fn revelation_of_the_sire_adopts_a_younger_caitiff_into_the_clan() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Brujah" && c.capacity >= 7, 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(
        SeatId(1),
        textless(|c| c.clan == "Caitiff" && c.capacity <= 5, 0),
        2,
        false,
    );
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let cap = e.state.capacity_of(x).unwrap();
    hand(&mut e, 0, &["Revelation of the Sire"]);
    pick(&mut e, play_at(0, 0, a, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Revelation of the Sire", Some(x));
    assert_eq!(e.state.minions[x.index()].clan(), Some("Brujah"));
    assert_eq!(e.state.capacity_of(x), Some(cap + 1));
    e.burn_card(card);
    assert_eq!(e.state.minions[x.index()].clan(), Some("Caitiff"));
    assert_eq!(e.state.capacity_of(x), Some(cap));
}

// ---- Anarch Secession -------------------------------------------------------

/// Anarch Secession by a Camarilla prince of Chicago: "This vampire becomes
/// Anarch … put this card on him or her as a title card to represent the
/// title of baron of the same city. Minions without titles cannot block
/// this action." Untitled X never offered a block, titled Y is; A an
/// Anarch baron of Chicago, the card itself his title card.
#[test]
fn anarch_secession_makes_a_prince_baron_of_his_city() {
    use zvtes::crypt::Sect;
    let (mut e, [a, _, x, y]) = four(|c| c.sect == Some(Sect::Camarilla));
    e.state.minions[a.index()].granted_title = Some(Title::Prince("Chicago"));
    e.state.minions[y.index()].granted_title = Some(Title::Primogen);
    hand(&mut e, 0, &["Anarch Secession"]);
    pick(&mut e, play(0, 0, a));
    let req = offering(&mut e, block(y));
    assert!(!req.options.contains(&block(x)), "untitled");
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Anarch Secession", Some(a));
    assert_eq!(e.state.sect_of(a), Some(Sect::Anarch));
    assert_eq!(
        e.state.title_card_on(a),
        Some((card, Title::Baron("Chicago")))
    );
    assert_eq!(e.state.title_of(a), Title::Baron("Chicago"));
}

/// Anarch Secession by a justicar: no city title, Anarch, the card to the
/// ash heap, the Camarilla title dormant off-sect.
#[test]
fn anarch_secession_from_a_justicar_leaves_no_title_card() {
    use zvtes::crypt::Sect;
    let (mut e, [a, ..]) = four(|c| c.sect == Some(Sect::Camarilla));
    let clan = e.state.minions[a.index()].clan().unwrap();
    e.state.minions[a.index()].granted_title = Some(Title::Justicar(clan));
    hand(&mut e, 0, &["Anarch Secession"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.sect_of(a), Some(Sect::Anarch));
    assert!(ash_has(&e, 0, "Anarch Secession"));
    assert_eq!(e.state.title_card_on(a), None);
    assert_eq!(e.state.title_of(a), Title::None);
}

/// "When meeting a requirement, the controller can choose how he meets it
/// (eg. he can play Anarch Secession as a Prince of any city, even a
/// made-up one)." [PIB 20150306] [PIB 20150307] Vlad Tepes, untitled, plays
/// it: an Anarch baron of a made-up city.
#[test]
fn vlad_tepes_secedes_as_prince_of_a_city_he_chooses() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), crypt_card("Vlad Tepes"), 6, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, true);
    hand(&mut e, 0, &["Anarch Secession"]);
    pick(&mut e, play(0, 0, v));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Anarch Secession", Some(v));
    assert_eq!(e.state.sect_of(v), Some(zvtes::crypt::Sect::Anarch));
    assert_eq!(
        e.state.title_card_on(v),
        Some((card, Title::Baron(zvtes::cards::MADE_UP_CITY)))
    );
}

/// [PIB 20150306] [PIB 20150307]: "any city": Vlad Tepes may take a city
/// held in play. X prince and Y baron of Chicago, Z prince of Paris: one
/// option a city (X for Chicago, Z for Paris) beside the made-up one;
/// Chicago chosen, Vlad baron of Chicago, his title contested with X's.
#[test]
fn vlad_tepes_secedes_as_prince_of_a_city_in_play() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), crypt_card("Vlad Tepes"), 6, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let y = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 3), 4, false);
    e.state.minions[x.index()].granted_title = Some(Title::Prince("Chicago"));
    e.state.minions[y.index()].granted_title = Some(Title::Baron("Chicago"));
    e.state.minions[z.index()].granted_title = Some(Title::Prince("Paris"));
    hand(&mut e, 0, &["Anarch Secession"]);
    pick(&mut e, play(0, 0, v));
    let req = offering(&mut e, Choice::ChooseMinion(x));
    let mut options = req.options.clone();
    options.sort_by_key(|o| format!("{o:?}"));
    let mut want = vec![
        Choice::Pass,
        Choice::ChooseMinion(x),
        Choice::ChooseMinion(z),
    ];
    want.sort_by_key(|o| format!("{o:?}"));
    assert_eq!(options, want);
    e.choose(index_of(&req, Choice::ChooseMinion(x)));
    let card = placed_on(&e, "Anarch Secession", Some(v));
    assert_eq!(
        e.state.title_card_on(v),
        Some((card, Title::Baron("Chicago")))
    );
    assert!(e.state.minions[v.index()].title_contested);
}

/// CrimethInc.: "Only usable after resolution of a successful action
/// requiring an Anarch or making this vampire Anarch": Anarch Secession's
/// "This vampire becomes Anarch" (rules-digest §3.12, LSJ 20080603). A, a
/// Camarilla prince with Presence, secedes; CrimethInc. [pro] unlocks him.
#[test]
fn crimethinc_follows_anarch_secession() {
    use zvtes::crypt::Sect;
    let (mut e, [a, ..]) = four(|c| c.sect == Some(Sect::Camarilla));
    e.state.minions[a.index()].granted_title = Some(Title::Prince("Chicago"));
    e.state.minions[a.index()].gained_levels = vec!["pro"];
    hand(&mut e, 0, &["Anarch Secession", "CrimethInc."]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.sect_of(a), Some(Sect::Anarch));
    assert!(!e.state.minions[a.index()].locked);
    assert!(ash_has(&e, 0, "CrimethInc."));
}

// ---- Distant Friend ---------------------------------------------------------

/// Seat 0's A (dom, pre) and B; seat 1's prince X and Y; Distant Friend
/// (mode `mode`) played by A on X, the turn passed to seat 1's minion
/// phase: the card in play.
fn distant_friend(mode: u8) -> (Engine, MinionId, MinionId, CardId) {
    let (mut e, [a, _, x, _]) = four(|c| c.sect == Some(zvtes::crypt::Sect::Camarilla));
    let level = if mode == 0 {
        vec!["dom", "pre"]
    } else {
        vec!["dom", "dom", "pre", "pre"]
    };
    e.state.minions[a.index()].gained_levels = level;
    e.state.minions[x.index()].granted_title = Some(Title::Prince("Paris"));
    hand(&mut e, 0, &["Distant Friend"]);
    pick(&mut e, play_at(0, mode, a, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Distant Friend", None);
    assert_eq!(e.state.cards[card.index()].chosen, vec![x]);
    (e, a, x, card)
}

/// Distant Friend: "If this action is successful, unlock the acting vampire
/// at the end of the turn. You may burn this card during a referendum to
/// change the votes and ballots of the chosen vampire to votes and ballots
/// of your choice." X casts in favor of its own Kine Resources Contested;
/// seat 0 burns the card: X's cast against.
#[test]
fn distant_friend_turns_its_vampires_votes_and_unlocks_the_actor() {
    let (mut e, a, x, card) = distant_friend(0);
    assert!(
        !e.state.minions[a.index()].locked,
        "unlocked at the end of the turn"
    );
    hand(&mut e, 1, &["Kine Resources Contested"]);
    kine_polling(&mut e, x);
    pick_when(&mut e, votes(x, true));
    pick_when(&mut e, ability(card.0, 1));
    request(&mut e);
    assert_eq!(cast_side(&e, x), Some(false));
    assert!(ash_has(&e, 0, "Distant Friend"));
}

/// Distant Friend [DOM][PRE]: "you can force the chosen vampire to cast his
/// or her votes and ballots when you burn this card." X, not cast yet,
/// forced to cast in favor.
#[test]
fn distant_friend_superior_forces_the_cast() {
    let (mut e, _, x, card) = distant_friend(1);
    hand(&mut e, 1, &["Kine Resources Contested"]);
    kine_polling(&mut e, x);
    pick_when(&mut e, ability(card.0, 2));
    request(&mut e);
    assert_eq!(cast_side(&e, x), Some(true));
}

/// Distant Friend: "Cannot be burned during a referendum that is
/// automatically passing." [PIB 20150105] [LSJ 19980107] After Día de los
/// Muertos, the first Sabbat referendum passes with no polling step: the
/// burn never offered.
#[test]
fn distant_friend_is_not_burned_in_an_automatically_passing_referendum() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), crypt_card("Concordia"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let card = in_play(&mut e, 1, "Distant Friend", None);
    e.state.cards[card.index()].chosen = vec![c];
    hand(&mut e, 0, &["War of Ages", "Día de los Muertos"]);
    e.at_master_phase();
    pick_when(&mut e, master_play(1));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, c));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        card_uses_choice(o, card)
    });
    let war = find_library("War of Ages");
    assert!(e.state.cards.iter().any(|x| x.in_play && x.card == war));
}

// ---- Consignment to Duat ----------------------------------------------------

/// Consignment to Duat [ser]: "Can target a vampire with no blood." [RTR
/// 20010711] X empty: the steal offered, nothing taken.
#[test]
fn consignment_to_duat_steals_from_an_empty_vampire() {
    let (mut e, [a, _, x, _]) = four(|_| true);
    e.state.minions[a.index()].gained_levels = vec!["ser"];
    e.state.minions[x.index()].blood = 0;
    hand(&mut e, 0, &["Consignment to Duat"]);
    pick(&mut e, play_at(0, 0, a, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1, "its cost paid, nothing gained");
    assert!(ash_has(&e, 0, "Consignment to Duat"));
}

/// Consignment to Duat [ser]: "If it burns an ally as a result, this counts
/// as a Ⓓ action burning an ally (eg. Trophies, Predator's
/// Transformation)." [ANK 20220507] The prey's 1-life ally burned;
/// Predator's Transformation [ABO] answers it.
#[test]
fn consignment_to_duat_burning_an_ally_is_a_directed_burn() {
    let (mut e, [a, ..]) = four(|_| true);
    e.state.minions[a.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    e.state.minions[a.index()].gained_levels = vec!["ser", "abo", "abo"];
    let ally = e.add_ally(SeatId(1), find_library("Dark Web Procurer"));
    e.state.minions[ally.index()].blood = 1;
    hand(
        &mut e,
        0,
        &["Consignment to Duat", "Predator's Transformation"],
    );
    pick(&mut e, play_at(0, 0, a, ally));
    let req = offering(&mut e, play(0, 3, a));
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    assert!(!e.state.minions[ally.index()].in_play, "burned");
    e.choose(index_of(&req, play(0, 3, a)));
    request(&mut e);
    assert_eq!(pool(&e)[1], 10 - 2);
}

/// Consignment to Duat [SER]: "During the attached minion's unlock phase,
/// they burn 1 blood or life. If they cannot, they go to torpor." X burns
/// its one blood; empty at its next unlock, to torpor. The card itself on
/// X.
#[test]
fn consignment_to_duat_drains_each_unlock_then_torpor() {
    let (mut e, [a, _, x, _]) = four(|_| true);
    e.state.minions[a.index()].gained_levels = vec!["ser", "ser"];
    e.state.minions[x.index()].blood = 2;
    hand(&mut e, 0, &["Consignment to Duat"]);
    pick(&mut e, play_at(0, 1, a, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Consignment to Duat", Some(x));
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 1);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[x.index()].blood = 0;
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(torpid(&e, x));
}

/// Consignment to Duat [SER]: "Burn this card after this acting minion
/// enters combat": A, its player, blocked by X next turn.
#[test]
fn consignment_to_duat_burns_when_its_acting_minion_enters_combat() {
    let (mut e, [a, _, x, _]) = four(|_| true);
    e.state.minions[a.index()].gained_levels = vec!["ser", "ser"];
    let card = in_play_mode(&mut e, 0, "Consignment to Duat", Some(x), 1);
    e.state.cards[card.index()].by = Some(a);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    assert!(e.state.cards[card.index()].in_play, "not before the combat");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[card.index()].in_play);
    assert!(ash_has(&e, 0, "Consignment to Duat"));
}

// ---- Mind of a Killer -------------------------------------------------------

/// Mind of a Killer [dem] on X: +1 strength; "Burn this card after a combat
/// involving them ends": B's bleed, X blocks, the card burned.
#[test]
fn mind_of_a_killer_burns_after_its_minions_combat() {
    let (mut e, [a, b, x, _]) = four(|_| true);
    e.state.minions[a.index()].gained_levels = vec!["dem"];
    hand(&mut e, 0, &["Mind of a Killer"]);
    pick(&mut e, play_at(0, 0, a, x));
    pick_when(&mut e, bleed(b));
    placed_on(&e, "Mind of a Killer", Some(x));
    assert_eq!(e.state.bearer_always(x, Stat::Strength), 1);
    pick_when(&mut e, block(x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Mind of a Killer"));
    assert_eq!(e.state.bearer_always(x, Stat::Strength), 0);
}

/// Mind of a Killer [DEM]: "During their discard phase, burn this card,
/// and their controller must inflict 2 unpreventable environmental damage
/// on another ready minion they control … lock the attached minion after
/// this card is burned." In seat 1's discard phase: Y takes 2, X locked.
#[test]
fn mind_of_a_killer_superior_wounds_another_minion_and_locks_in_the_discard_phase() {
    let (mut e, [a, _, x, y]) = four(|_| true);
    e.state.minions[a.index()].gained_levels = vec!["dem", "dem"];
    hand(&mut e, 0, &["Mind of a Killer"]);
    pick(&mut e, play_at(0, 1, a, x));
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[x.index()].locked);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Mind of a Killer"));
    assert_eq!(blood(&e, y), 4 - 2);
    assert_eq!(blood(&e, x), 4);
    // X locked in seat 1's discard phase, unlocked since in no unlock
    // phase of seat 1's: still locked now.
    assert!(e.state.minions[x.index()].locked);
}

/// Mind of a Killer [DEM]: "lock the attached minion after this card is
/// burned": any burn of it (owner, 2026-11-19). On seat 1's ally, Y's
/// Cleansing Ritual [obe] burns it in seat 1's minion phase: the ally
/// locked; [dem] on another ally, not.
#[test]
fn mind_of_a_killer_superior_locks_when_burned_by_another_card() {
    for superior in [true, false] {
        let (mut e, [a, _, _, y]) = four(|_| true);
        e.state.minions[a.index()].gained_levels = vec!["dem", "dem"];
        e.state.minions[y.index()].gained_levels = vec!["obe"];
        let ally = e.add_ally(SeatId(1), find_library("Dark Web Procurer"));
        hand(&mut e, 0, &["Mind of a Killer"]);
        pick(&mut e, play_at(0, u8::from(superior), a, ally));
        to_minion_phase(&mut e, SeatId(1));
        let card = placed_on(&e, "Mind of a Killer", Some(ally));
        assert!(!e.state.minions[ally.index()].locked);
        hand(&mut e, 1, &["Cleansing Ritual"]);
        pick(&mut e, play_on(0, 1, y, Target::Card(card)));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(ash_has(&e, 0, "Mind of a Killer"));
        assert_eq!(e.state.minions[ally.index()].locked, superior);
    }
}

// ---- The Name Forgotten -----------------------------------------------------

/// The Name Forgotten: "If the vampire with this card is burned, he or she
/// is removed from the game instead, and all Methuselahs remove all other
/// vampires with the same name in their crypts, uncontrolled regions and
/// ash heaps from the game." "Each Methuselah must find as many copies of
/// the vampire as possible when searching." [LSJ 20050611]
#[test]
fn the_name_forgotten_removes_every_copy_when_its_vampire_burns() {
    let (mut e, [a, _, x, _]) = four(|c| c.group != 0);
    e.state.minions[a.index()].gained_levels = vec!["tha"];
    let c = match e.state.minions[x.index()].kind {
        zvtes::state::MinionKind::Vampire(c) => c,
        _ => unreachable!(),
    };
    e.state.seats[0].crypt.push(c);
    e.state.seats[0].crypt.push(c);
    e.state.seats[1].ash_crypt.push(c);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: c,
            copy: zvtes::copies::Untracked(zvtes::copies::CopyId::NONE),
            counters: 2,
            banished: None,
        });
    hand(&mut e, 0, &["Name Forgotten, The"]);
    pick(&mut e, play_at(0, 0, a, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    placed_on(&e, "Name Forgotten, The", Some(x));
    e.burn(x);
    request(&mut e);
    assert!(!e.state.minions[x.index()].in_play);
    let removed = |s: usize| {
        e.state.seats[s]
            .removed_crypt
            .to_cards()
            .iter()
            .filter(|&&k| k == c)
            .count()
    };
    assert_eq!((removed(0), removed(1)), (3, 2), "every copy removed");
    assert!(!e.state.seats[1].ash_crypt.contains(&c), "not burned");
    assert!(!e.state.seats[0].crypt.contains(&c));
    assert!(e.state.seats[0].uncontrolled.iter().all(|s| s.card != c));
}

// ---- Inquisition ------------------------------------------------------------

/// Inquisition: "Choose one or more bishops. Each of the chosen bishops
/// loses his or her title and takes 2 unpreventable damage. The controllers
/// of the chosen bishops may attempt to block": bishops of seats 1 and 2
/// (across the table: seat 2 neither prey nor predator) chosen, seat 2
/// offered a block; both untitled, 2 blood less. With no bishop in play,
/// not offered.
#[test]
fn inquisition_strips_and_wounds_bishops_of_two_methuselahs() {
    use zvtes::crypt::Sect;
    let sab = |c: &zvtes::crypt::CryptCard| c.sect == Some(Sect::Sabbat) && c.capacity >= 4;
    let table = || {
        let mut e = game(4);
        let a = e.add_minion(SeatId(0), textless(sab, 0), 4, false);
        e.add_minion(SeatId(0), textless(sab, 1), 4, false);
        let x = e.add_minion(SeatId(1), textless(sab, 2), 4, false);
        let z = e.add_minion(SeatId(2), textless(sab, 3), 4, false);
        e.add_minion(SeatId(3), textless(sab, 4), 4, true);
        hand(&mut e, 0, &["Inquisition"]);
        (e, a, x, z)
    };
    let (mut e, a, ..) = table();
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, a)), "no bishop to choose");
    let (mut e, a, x, z) = table();
    for m in [x, z] {
        e.state.minions[m.index()].granted_title = Some(Title::Bishop);
    }
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::ChooseMinion(x));
    pick(&mut e, Choice::ChooseMinion(z));
    let req = offering(&mut e, block(z));
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    for m in [x, z] {
        assert_eq!(e.state.title_of(m), Title::None);
        assert_eq!(blood(&e, m), 4 - 2);
    }
}

// ---- Shared Nightmare -------------------------------------------------------

/// Shared Nightmare [chi]: "Choose one or two allies or younger vampires
/// controlled by your predator and/or prey. The chosen minions cannot
/// block or cast votes or ballots this turn." Seat 1's prince X and seat
/// 2's Z chosen: X never offered a block of B's bleed (W is), nor a vote
/// in B's referendum.
#[test]
fn shared_nightmare_silences_two_minions_for_the_turn() {
    use zvtes::crypt::Sect;
    let cam = |c: &zvtes::crypt::CryptCard| c.sect == Some(Sect::Camarilla);
    let mut e = game(3);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| !cam(c) && c.capacity >= 9, 0),
        6,
        false,
    );
    let b = e.add_minion(
        SeatId(0),
        textless(|c| cam(c) && c.capacity >= 4, 0),
        4,
        false,
    );
    let x = e.add_minion(
        SeatId(1),
        textless(|c| cam(c) && (4..=6).contains(&c.capacity), 1),
        4,
        false,
    );
    let w = e.add_minion(
        SeatId(1),
        textless(|c| cam(c) && (4..=6).contains(&c.capacity), 2),
        4,
        false,
    );
    let z = e.add_minion(
        SeatId(2),
        textless(|c| cam(c) && (4..=6).contains(&c.capacity), 3),
        4,
        false,
    );
    e.state.minions[a.index()].gained_levels = vec!["chi"];
    e.state.minions[x.index()].granted_title = Some(Title::Prince("Paris"));
    hand(&mut e, 0, &["Shared Nightmare", "Kine Resources Contested"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::ChooseMinion(x));
    pick(&mut e, Choice::ChooseMinion(z));
    pick_when(&mut e, bleed(b));
    assert_eq!(blood(&e, a), 6 - 3);
    let req = offering(&mut e, block(w));
    assert!(!req.options.contains(&block(x)));
    e.choose(index_of(&req, Choice::Pass));
    let _ = z;
    // A referendum the same turn: X casts nothing. (B is locked: A's
    // second action unlocked by hand.)
    e.state.minions[a.index()].locked = false;
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, a));
    to_referendum(&mut e);
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::CastVotes { minion, .. } if *minion == x),
    );
}

/// Shared Nightmare [CHI]: "As above, but choose three allies or younger
/// vampires": exactly three (owner, 2026-11-19: "card text is just three
/// (not lower) - but the vampire can use the inferior effect if it wants
/// less"). Two eligible: [CHI] not offered, [chi] is; two allies of seat
/// 1's more: [CHI] offered, no pass before the third pick.
#[test]
fn shared_nightmare_superior_picks_exactly_three() {
    let (mut e, [a, _, x, y]) = four(|c| c.capacity <= 7);
    e.state.minions[a.index()].gained_levels = vec!["chi", "chi"];
    e.state.minions[a.index()].kind =
        zvtes::state::MinionKind::Vampire(textless(|c| c.capacity >= 10, 0));
    hand(&mut e, 0, &["Shared Nightmare"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, a)));
    assert!(!req.options.contains(&play(0, 1, a)), "two eligible");
    let ally = e.add_ally(SeatId(1), find_library("Dark Web Procurer"));
    let other = e.add_ally(SeatId(1), find_library("Dark Web Procurer"));
    pick(&mut e, play(0, 1, a));
    pick(&mut e, Choice::ChooseMinion(x));
    pick(&mut e, Choice::ChooseMinion(y));
    let req = request(&mut e);
    let mut options = req.options.clone();
    options.sort_by_key(|o| format!("{o:?}"));
    assert_eq!(
        options,
        vec![Choice::ChooseMinion(ally), Choice::ChooseMinion(other)]
    );
    pick(&mut e, Choice::ChooseMinion(ally));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Shared Nightmare"));
    assert_eq!(blood(&e, a), 4 - 3);
}

// ---- Lifeless Tongues -------------------------------------------------------

/// Lifeless Tongues [NEC]: the vampire of seat 1's ash heap removed, the
/// card in play under seat 1's control; A's controller looks at seat 1's
/// hand in seat 0's unlock phase and in seat 1's ([NEC]: any unlock
/// phase); any minion burns it as a +1 stealth action.
#[test]
fn lifeless_tongues_lets_the_actor_see_the_heap_owners_hand_each_unlock() {
    let (mut e, [a, _, x, _]) = four(|_| true);
    e.state.minions[a.index()].gained_levels = vec!["nec", "nec"];
    let dead = textless(|c| c.capacity >= 4, 9);
    e.state.seats[1].ash_crypt.push(dead);
    hand(&mut e, 0, &["Lifeless Tongues"]);
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = e
        .state
        .cards
        .iter()
        .position(|c| c.card == find_library("Lifeless Tongues") && c.in_play)
        .map(|i| CardId(i as u32))
        .expect("in play");
    assert_eq!(e.state.card_controller(card), SeatId(1));
    assert!(e.state.seats[1].removed_crypt.contains(&dead));
    // Seat 1's next unlock phase is past; seat 0's own next.
    let look = ability(card.0, 1);
    pick_when(&mut e, look);
    request(&mut e);
    assert!(e.state.peeks.contains(&(SeatId(0), SeatId(1))));
    // The burn: X's +1 stealth action.
    let burn = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(card),
        ability: 2,
        actor: Some(x),
        target: Target::None,
    };
    pick_when(&mut e, burn);
    request(&mut e);
    assert_eq!(
        e.action().expect("action").kind,
        zvtes::decision::ActionKind::BurnCard
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Lifeless Tongues"));
}

// ---- Bujo -------------------------------------------------------------------

/// Seat 0's Ravnos A (and B), seat 1's X and Y; `heap` in seat 1's ash heap,
/// `own` in seat 0's hand after Bujo.
fn bujo_table(heap: &[&str], own: &[&str]) -> (Engine, MinionId, MinionId) {
    let (mut e, [a, _, x, _]) = four(|c| c.clan == "Ravnos");
    for c in heap {
        e.state.seats[1].ash.push(find_library(c));
    }
    hand(&mut e, 0, &["Bujo"]);
    hand(&mut e, 0, own);
    (e, a, x)
}

/// Bujo: the referendum of seat 1's Kine Resources Contested called, the
/// card removed from the game, A at +3 votes; "The chosen action card does
/// not provide its +1 Vote … Another Political Action card can be
/// discarded by the acting Methuselah to gain +1 vote." [LSJ 20080219]
#[test]
fn bujo_calls_a_referendum_from_another_ash_heap_and_removes_it() {
    let (mut e, a, _) = bujo_table(&["Kine Resources Contested"], &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, a));
    to_referendum(&mut e);
    let krc = find_library("Kine Resources Contested");
    assert!(e.state.seats[1].removed.contains(&krc));
    assert!(!e.state.seats[1].ash.contains(&krc));
    assert_eq!(e.referendum().unwrap().votes_of(&e.state, a), 3);
    let req = offering(&mut e, votes(a, true));
    assert!(!req.options.contains(&Choice::CastCardVote { favor: true }));
    assert!(
        req.options
            .iter()
            .any(|o| matches!(o, Choice::BurnForVote { .. })),
        "{:?}",
        req.options
    );
}

/// Bujo: "When Delaying Tactics is played during the referendum called by
/// Bujo, the card is not returned to anyone's hand." [LSJ 20100527] (the
/// same rule for Delaying Tactics' row, [LSJ 20041130] [LSJ 20100527]):
/// Bujo to the ash heap, the named card removed.
#[test]
fn bujo_referendum_delaying_tactics_returns_nothing() {
    let (mut e, a, x) = bujo_table(&["Kine Resources Contested"], &[]);
    hand(&mut e, 1, &["Delaying Tactics"]);
    pick(&mut e, play(0, 0, a));
    to_referendum(&mut e);
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let (bujo, krc) = (
        find_library("Bujo"),
        find_library("Kine Resources Contested"),
    );
    assert!(!e.state.seats[0].hand.contains(&bujo));
    assert!(e.state.seats[0].ash.contains(&bujo));
    assert!(e.state.seats[1].removed.contains(&krc));
    assert!(e.state.seats.iter().all(|s| !s.hand.contains(&krc)));
}

// ---- Shepherd's Innocence ---------------------------------------------------

/// Shepherd's Innocence [ani]: X's Raven Spy taken, onto B, chosen as the
/// action is announced; its card seat 0's.
#[test]
fn shepherds_innocence_takes_a_retainer_onto_a_chosen_vampire() {
    let (mut e, [a, b, x, _]) = four(|_| true);
    e.state.minions[a.index()].gained_levels = vec!["ani"];
    let spy = in_play(&mut e, 1, "Raven Spy", Some(x));
    hand(&mut e, 0, &["Shepherd's Innocence"]);
    pick(&mut e, play_on(0, 1, a, Target::Card(spy)));
    pick(&mut e, Choice::ChooseMinion(b));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[spy.index()].host, Some(b));
    assert_eq!(e.state.card_controller(spy), SeatId(0));
}

/// Shepherd's Innocence [ANI]: "Is directed at all Methuselahs controlling a
/// card requiring Animalism" [RBK who-may-attempt-to-block]: seat 2, across
/// the table, offered a block. "Retainers and equipments you control are
/// also affected. You must put them on vampires you control (… if the card
/// is on an ally, it must move)." [LSJ 20080803-2] [LSJ 19981102] Every
/// Raven Spy ends on A, the one on seat 0's ally moved.
#[test]
fn shepherds_innocence_superior_takes_all_and_moves_own_off_allies() {
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let ally = e.add_ally(SeatId(0), find_library("Dark Web Procurer"));
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let z = e.add_minion(SeatId(2), textless(|c| c.capacity >= 4, 2), 4, false);
    e.add_minion(SeatId(3), textless(|c| c.capacity >= 4, 3), 4, true);
    e.state.minions[a.index()].gained_levels = vec!["ani", "ani"];
    let spies = [
        in_play(&mut e, 0, "Raven Spy", Some(ally)),
        in_play(&mut e, 1, "Raven Spy", Some(x)),
        in_play(&mut e, 2, "Raven Spy", Some(z)),
    ];
    hand(&mut e, 0, &["Shepherd's Innocence"]);
    pick(&mut e, play(0, 3, a));
    let req = offering(&mut e, block(z));
    e.choose(index_of(&req, Choice::Pass));
    for _ in 0..50 {
        let req = request(&mut e);
        if req.seat == SeatId(1) && req.window == WindowLabel::MinionPhase {
            break;
        }
        match req
            .options
            .iter()
            .position(|o| *o == Choice::ChooseMinion(a))
        {
            Some(i) => e.choose(i),
            None => e.choose(fallback(&req)),
        }
    }
    for s in spies {
        assert_eq!(e.state.cards[s.index()].host, Some(a), "{s:?}");
        assert_eq!(e.state.card_controller(s), SeatId(0));
    }
}

// ---- Undue Influence --------------------------------------------------------

/// Seat 0's Anarch A (and B), seat 1's X and Y; A with `levels`.
fn undue(levels: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let (mut e, [a, b, x, _]) = four(|_| true);
    for m in [a, b] {
        e.state.minions[m.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    }
    e.state.minions[a.index()].gained_levels = levels.to_vec();
    (e, a, x)
}

fn slot(e: &mut Engine, seat: usize, card: u16, counters: u8) {
    e.state.seats[seat]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card,
            copy: zvtes::copies::Untracked(zvtes::copies::CopyId::NONE),
            counters,
            banished: None,
        });
}

/// Undue Influence: "Cannot be played with no vampire in the uncontrolled
/// region." [LSJ 20060623] [qui]: with one at capacity less 1, played: it
/// reaches its capacity and comes into play.
#[test]
fn undue_influence_quietus_brings_a_full_vampire_into_play() {
    let (mut e, a, _) = undue(&["qui"]);
    hand(&mut e, 0, &["Undue Influence"]);
    let req = request(&mut e);
    assert!(!plays(&req)
        .iter()
        .any(|o| matches!(o, Choice::Play { card: 0, .. })));
    let (mut e, a2, _) = undue(&["qui"]);
    let _ = a;
    let v = textless(|c| c.capacity == 4, 7);
    slot(&mut e, 0, v, 3);
    hand(&mut e, 0, &["Undue Influence"]);
    pick(&mut e, play_on(0, 2, a2, Target::Uncontrolled(0)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].uncontrolled.is_empty());
    assert!(e
        .state
        .minions
        .iter()
        .any(|m| m.in_play && m.kind == zvtes::state::MinionKind::Vampire(v) && m.blood == 4));
}

/// Undue Influence [pre]: "Bleed. If the bleed is successful, add 1 blood
/// to the chosen vampire."
#[test]
fn undue_influence_presence_adds_blood_on_a_successful_bleed() {
    let (mut e, a, x) = undue(&["pre"]);
    e.state.minions[x.index()].in_torpor = true;
    e.state.minions[3].in_torpor = true;
    slot(&mut e, 0, textless(|c| c.capacity >= 6, 7), 1);
    hand(&mut e, 0, &["Undue Influence"]);
    pick(&mut e, play_on(0, 1, a, Target::Uncontrolled(0)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 1);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 2);
}

/// Undue Influence [chi]: 1 blood from seat 1's uncontrolled vampire to A's
/// chosen one, the action directed at seat 1 (its minions offered a block).
#[test]
fn undue_influence_chimerstry_moves_blood_from_another_region() {
    let (mut e, a, x) = undue(&["chi"]);
    slot(&mut e, 0, textless(|c| c.capacity >= 6, 7), 1);
    slot(&mut e, 1, textless(|c| c.capacity >= 6, 8), 2);
    hand(&mut e, 0, &["Undue Influence"]);
    // The only other region and its only vampire: the picks elide.
    pick(&mut e, play_on(0, 0, a, Target::Uncontrolled(0)));
    let req = offering(&mut e, block(x));
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 2);
    assert_eq!(e.state.seats[1].uncontrolled[0].counters, 1);
}

/// Undue Influence: "Is affected by Blood Cult Awareness Network" [ANK
/// 20240118-2] [LSJ 20060414] (BCAN's own row: conditional counters too):
/// [pre] with BCAN in play and a ready imbued, -1 stealth. Backs BCAN's
/// mention row too: [LSJ 20060414] [ANK 20240118-2].
#[test]
fn undue_influence_is_taxed_by_bcan() {
    let (mut e, a, _) = undue(&["pre"]);
    slot(&mut e, 0, textless(|c| c.capacity >= 6, 7), 1);
    let imbued = zvtes::crypt::CRYPT.iter().position(|c| c.imbued).unwrap() as u16;
    e.add_minion(SeatId(1), imbued, 3, false);
    in_play(&mut e, 1, "Blood Cult Awareness Network", None);
    hand(&mut e, 0, &["Undue Influence"]);
    pick(&mut e, play_on(0, 1, a, Target::Uncontrolled(0)));
    request(&mut e);
    let act = e.action().expect("action");
    assert!(act.makes_vampires(&e.state));
    // The card's +1 in its base, BCAN's -1 read live.
    assert_eq!(act.stealth(&e.state), act.base_stealth as i32 - 1);
}

/// Public Trust [PRE] and Break the Bonds [pre]: "Is affected by Blood
/// Cult Awareness Network" [ANK 20240118-2] [LSJ 20060414]: their blood to
/// an uncontrolled vampire on a successful bleed, -1 stealth with BCAN in
/// play and a ready imbued; Public Trust [pre], no blood added, untaxed.
#[test]
fn public_trust_and_break_the_bonds_presence_are_taxed_by_bcan() {
    for (card, levels, mode, taxed) in [
        ("Public Trust", &["pre", "pre"][..], 1, true),
        ("Public Trust", &["pre"][..], 0, false),
        ("Break the Bonds", &["pre"][..], 2, true),
    ] {
        let (mut e, a, _) = undue(levels);
        slot(&mut e, 0, textless(|c| c.capacity >= 6, 7), 1);
        let imbued = zvtes::crypt::CRYPT.iter().position(|c| c.imbued).unwrap() as u16;
        e.add_minion(SeatId(1), imbued, 3, false);
        in_play(&mut e, 1, "Blood Cult Awareness Network", None);
        hand(&mut e, 0, &[card]);
        let target = if taxed {
            Target::Uncontrolled(0)
        } else {
            Target::None
        };
        pick(&mut e, play_on(0, mode, a, target));
        request(&mut e);
        let act = e.action().expect("action");
        assert_eq!(act.makes_vampires(&e.state), taxed, "{card} {mode}");
        assert_eq!(
            act.stealth(&e.state),
            act.base_stealth as i32 - i32::from(taxed),
            "{card} {mode}"
        );
    }
}

/// Bujo: "Requirements apply but do not count as requirements for the
/// action (eg. you cannot use CrimethInc. if the chosen card requires an
/// Anarch)." [LSJ 20100725] [LSJ 20100303] [ANK 20180817] The Anarch
/// Ravnos calls seat 1's Free States Rant: CrimethInc. [pro] never offered
/// after.
#[test]
fn bujo_chosen_card_requirements_do_not_open_crimethinc() {
    let (mut e, a, _) = bujo_table(&["Free States Rant"], &["CrimethInc."]);
    e.state.minions[a.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    e.state.minions[a.index()].gained_levels = vec!["pro"];
    pick(&mut e, play(0, 0, a));
    to_referendum(&mut e);
    assert!(e.state.seats[1]
        .removed
        .contains(&find_library("Free States Rant")));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, a));
}

// ==== Vampires, Adrianne to Bianca, and Phillipe Rigaud ======================

// ---- Helpers (vampires) -----------------------------------------------------

/// The crypt card `name`, advanced.
#[track_caller]
fn adv_card(name: &str) -> u16 {
    zvtes::crypt::CRYPT
        .iter()
        .position(|c| c.name == name && c.adv)
        .unwrap_or_else(|| panic!("no advanced {name}")) as u16
}

/// Is `o` a use of vampire `m`'s printed ability `ability`, whoever acts?
fn uses_printed(o: &Choice, m: MinionId, ability: u8) -> bool {
    matches!(o, Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(v),
        ability: a,
        ..
    } if *v == m && *a == ability)
}

/// Pass (or hand strike) until a use of vampire `m`'s printed ability
/// `ability` is offered, and take it.
#[track_caller]
fn pick_use(e: &mut Engine, m: MinionId, ability: u8) {
    for _ in 0..200 {
        let req = request(e);
        if let Some(i) = req.options.iter().position(|o| uses_printed(o, m, ability)) {
            e.choose(i);
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("{m:?}'s ability {ability} never offered");
}

/// A fixture vampire of `clan`, capacity in `caps` (the `nth`).
#[track_caller]
fn of_clan(clan: &str, caps: std::ops::RangeInclusive<u8>, nth: usize) -> u16 {
    fixture_vampire(|c| c.clan == clan && caps.contains(&c.capacity), nth)
}

/// What `m`'s bleed of seat 1 costs it, in pool, unblocked.
fn bleeds_for(e: &mut Engine, m: MinionId) -> i32 {
    let before = e.state.seats[1].pool;
    pick(e, bleed(m));
    until(e, SeatId(0), WindowLabel::MinionPhase);
    before - e.state.seats[1].pool
}

// ---- Text-only vampires -----------------------------------------------------

/// Agrippina, Aleph, Anastasia Grey, Angel, Badger, Bear Paw ("Camarilla.")
/// and Anvil ("Camarilla primogen."): no definition, their sect and title
/// generated.
#[test]
fn round33_b_textless_vampires_carry_only_generated_traits() {
    for (name, title) in [
        ("Agrippina", Title::None),
        ("Aleph", Title::None),
        ("Anastasia Grey", Title::None),
        ("Angel", Title::None),
        ("Badger", Title::None),
        ("Bear Paw", Title::None),
        ("Anvil", Title::Primogen),
    ] {
        let i = crypt_card(name);
        assert!(zvtes::cards::vampire(i).is_none(), "{name} has text");
        let c = &zvtes::crypt::CRYPT[i as usize];
        assert_eq!(c.title, title, "{name}");
        assert_eq!(c.sect, Some(zvtes::crypt::Sect::Camarilla), "{name}");
    }
}

// ---- Plain bonuses ----------------------------------------------------------

/// Adrianne ("Camarilla: +1 bleed.") and Ankh-sen-Sutekh ("Independent: +1
/// bleed."): an unblocked bleed for 2.
#[test]
fn adrianne_and_ankh_sen_sutekh_bleed_for_two() {
    for name in ["Adrianne", "Ankh-sen-Sutekh"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), crypt_card(name), 4, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        assert_eq!(bleeds_for(&mut e, a), 2, "{name}");
    }
}

/// Angus the Unruled ("+1 strength."), Bianca ("+1 strength."): their hand
/// strike deals 2. Basilia ("Damage from Basilia's hand strikes is
/// aggravated."): her hand strike's 1 aggravated is not mended, the blocker
/// to torpor with its blood kept.
#[test]
fn angus_bianca_strike_for_two_and_basilia_strikes_aggravated() {
    for name in ["Angus the Unruled", "Bianca"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), crypt_card(name), 4, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
        fight(&mut e, a, x);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, x), 2, "{name}");
    }
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), crypt_card("Basilia"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    fight(&mut e, b, x);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(torpid(&e, x), "aggravated damage is not mended");
    assert_eq!(blood(&e, x), 4, "nothing burned to mend it");
}

// ---- Alexis Sorokin ---------------------------------------------------------

/// Alexis Sorokin: "She can steal an equipment as a Ⓓ action." Another
/// Methuselah's equipment moves to her; her own Methuselah's is never
/// offered.
#[test]
fn alexis_sorokin_steals_another_methuselahs_equipment() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), crypt_card("Alexis Sorokin"), 4, false);
    let b = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    let own = in_play(&mut e, 0, ".44 Magnum", Some(b));
    let theirs = in_play(&mut e, 1, ".44 Magnum", Some(x));
    let req = request(&mut e);
    let steal = |c: CardId| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(a),
        ability: 1,
        actor: Some(a),
        target: Target::Card(c),
    };
    assert!(!req.options.contains(&steal(own)));
    pick(&mut e, steal(theirs));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[theirs.index()].host, Some(a));
    assert_eq!(e.state.cards[own.index()].host, Some(b));
}

/// Alexis Sorokin: "She gets +1 bleed against a Methuselah who controls a
/// ready Toreador." 2 against a ready Toreador's Methuselah, 1 when that
/// Toreador is in torpor.
#[test]
fn alexis_sorokin_bleeds_more_against_a_ready_toreador() {
    for ready in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), crypt_card("Alexis Sorokin"), 4, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        e.add_minion(SeatId(1), of_clan("Toreador", 3..=8, 0), 4, !ready);
        e.add_minion(SeatId(1), of_clan("Brujah", 3..=8, 0), 4, false);
        assert_eq!(bleeds_for(&mut e, a), 1 + i32::from(ready), "ready {ready}");
    }
}

/// Alexis Sorokin: "Once each combat, Alexis can strike: dodge." Offered as
/// her strike, the blocker's hand strike dodged.
#[test]
fn alexis_sorokin_dodges_once_each_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), crypt_card("Alexis Sorokin"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    fight(&mut e, a, x);
    pick_use(&mut e, a, 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4, "the hand strike dodged");
    assert_eq!(blood(&e, x), 4);
}

// ---- Cannot block undirected actions ----------------------------------------

/// Allonzo Montoya, Benedict Giovanni, Agent, Phillipe Rigaud: "cannot
/// block undirected actions." Seat 1's vampire is offered no block of a
/// hunt (undirected), its companion is; it is offered one of a bleed of
/// its Methuselah. Directedness is the action's own (LSJ 20080809).
#[test]
fn round33_vampires_cannot_block_undirected_actions() {
    for name in [
        "Allonzo Montoya",
        "Benedict Giovanni, Agent",
        "Phillipe Rigaud",
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        let b = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
        let v = e.add_minion(SeatId(1), crypt_card(name), 4, false);
        let y = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
        pick(&mut e, action(a, zvtes::decision::ActionKind::Hunt));
        let req = offering(&mut e, block(y));
        assert!(!req.options.contains(&block(v)), "{name} blocks a hunt");
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        pick(&mut e, bleed(b));
        offering(&mut e, block(v));
    }
}

/// Allonzo Montoya: "Allonzo gets +1 stealth on actions that require
/// Serpentis [ser]." Concordance's [ser] bleed (+1 stealth) at 2; his
/// rulebook bleed at 0.
#[test]
fn allonzo_montoya_sneaks_serpentis_actions() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), crypt_card("Allonzo Montoya"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    hand(&mut e, 0, &["Concordance"]);
    pick(&mut e, play_on(0, 0, a, Target::None));
    request(&mut e);
    let act = e.action().expect("action");
    assert_eq!(act.stealth(&e.state), 2);
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), crypt_card("Allonzo Montoya"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    pick(&mut e, bleed(a));
    request(&mut e);
    assert_eq!(e.action().expect("action").stealth(&e.state), 0);
}

// ---- Benedict Giovanni, Agent ----------------------------------------------

/// Benedict Giovanni, Agent: "During your unlock phase, any Giovanni older
/// than the oldest ready Giovanni you control can burn 1 blood to steal
/// Benedict." In his controller's unlock phase, seat 1's Giovanni of
/// capacity 7 steals him (1 blood) where seat 1's of capacity 6 (equal, not
/// older) is not offered; a ready Giovanni of 8 of his controller's bars
/// the 7 (Martino della Passaglia; Diego Giovanni of 8, his text a
/// location burn).
#[test]
fn benedict_giovanni_is_stolen_by_an_older_giovanni() {
    for elder in [false, true] {
        let mut e = game(2);
        let ben = e.add_minion(SeatId(0), crypt_card("Benedict Giovanni, Agent"), 4, false);
        if elder {
            e.add_minion(SeatId(0), crypt_card("Diego Giovanni"), 4, false);
        } else {
            e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        }
        let seven = e.add_minion(SeatId(1), crypt_card("Martino della Passaglia"), 4, false);
        let six = e.add_minion(SeatId(1), of_clan("Giovanni", 6..=6, 0), 4, false);
        let by = |m: MinionId| {
            move |o: &Choice| {
                matches!(o, Choice::UseAbility { actor: Some(a), .. } if *a == m)
                    && uses_printed(o, ben, 0)
            }
        };
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let mut stolen = false;
        loop {
            let req = request(&mut e);
            assert!(!req.options.iter().any(by(six)), "equal age");
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            match req.options.iter().position(by(seven)) {
                Some(i) => {
                    e.choose(i);
                    stolen = true;
                }
                None => e.choose(fallback(&req)),
            }
        }
        assert_eq!(stolen, !elder, "elder {elder}");
        assert_eq!(e.state.minions[ben.index()].owner, SeatId(u8::from(!elder)));
        assert_eq!(blood(&e, seven), 4 - u8::from(!elder));
    }
}

// ---- Phillipe Rigaud --------------------------------------------------------

/// Phillipe Rigaud: "When any older vampires are in torpor that Phillipe may
/// diablerize, he must attempt a diablerie action (unless he must hunt)."
/// With an older torpid vampire his minion phase offers only his
/// diableries (no end of phase, no other action); "As long as there is an
/// opportunity for diablerie, he must attempt it, even if he did so already
/// and unlocked." [ANK 20211010]: unlocked after one, forced again; "If he
/// cannot commit diablerie, then his card text does not force him to"
/// [LSJ 20050514]: a younger torpid vampire only, or empty (he must hunt),
/// free.
#[test]
fn phillipe_rigaud_must_diablerize_an_older_torpid_vampire() {
    use zvtes::decision::ActionKind::{Bleed, Diablerie, Hunt};
    let diablerie = |m: MinionId, t: MinionId| Choice::TakeAction {
        minion: m,
        action: Diablerie,
        target: Some(t),
    };
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), crypt_card("Phillipe Rigaud"), 4, false);
    let b = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let old1 = e.add_minion(SeatId(1), textless(|c| c.capacity >= 8, 0), 2, true);
    let old2 = e.add_minion(SeatId(1), textless(|c| c.capacity >= 8, 1), 2, true);
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 0), 1, true);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::Pass));
    assert!(!req.options.contains(&action(b, Bleed)));
    assert!(!req.options.contains(&action(p, Bleed)));
    // Any diablerie discharges it, the younger vampire's too.
    for t in [old1, old2, young] {
        assert!(req.options.contains(&diablerie(p, t)), "{t:?}");
    }
    assert_eq!(
        e.dues(&req),
        vec![zvtes::actions::Due::Action(p)],
        "the phase waits on him"
    );
    pick(&mut e, diablerie(p, old1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[old1.index()].in_play, "diablerized");
    assert!(
        request(&mut e).options.contains(&Choice::Pass),
        "locked: free"
    );
    e.state.minions[p.index()].locked = false;
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::Pass), "unlocked: due again");
    assert!(req.options.contains(&diablerie(p, old2)));

    // A younger torpid vampire only: free.
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), crypt_card("Phillipe Rigaud"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 0), 1, true);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::Pass));
    assert!(req.options.contains(&diablerie(p, young)));
    assert!(req.options.contains(&action(p, Bleed)));

    // Empty, he must hunt instead: the hunt, alone, taken.
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), crypt_card("Phillipe Rigaud"), 0, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 8, 0), 2, true);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    request(&mut e);
    let act = e.action().expect("his hunt");
    assert_eq!((act.acting, act.kind), (p, Hunt));
}

/// Phillipe Rigaud's due beside another vampire's hunt: both in its
/// player's order (rules-qa 17.2), his diablerie offered with the hunt.
#[test]
fn phillipe_rigaud_diablerie_is_due_beside_a_hunt() {
    use zvtes::decision::ActionKind::{Diablerie, Hunt};
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), crypt_card("Phillipe Rigaud"), 4, false);
    let h = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 0, false);
    let old = e.add_minion(SeatId(1), textless(|c| c.capacity >= 8, 0), 2, true);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    let req = request(&mut e);
    assert!(req.options.contains(&action(h, Hunt)));
    assert!(req.options.contains(&Choice::TakeAction {
        minion: p,
        action: Diablerie,
        target: Some(old),
    }));
    assert!(!req.options.contains(&Choice::Pass));
}

/// Phillipe Rigaud (red reading): his own Methuselah's older torpid vampire
/// is one he "may diablerize": forced too.
#[test]
fn phillipe_must_diablerize_even_his_own_methuselahs_older_vampire() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), crypt_card("Phillipe Rigaud"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let own = e.add_minion(SeatId(0), textless(|c| c.capacity >= 8, 0), 2, true);
    let theirs = e.add_minion(SeatId(1), textless(|c| c.capacity >= 8, 1), 2, true);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::Pass));
    for t in [own, theirs] {
        assert!(req.options.contains(&Choice::TakeAction {
            minion: p,
            action: zvtes::decision::ActionKind::Diablerie,
            target: Some(t),
        }));
    }
}

// ---- Amaravati --------------------------------------------------------------

/// Amaravati: "During the polling step of a political action, Amaravati can
/// burn 2 blood to give each ready Banu Haqim you control +1 vote." "The
/// ability can be used while locked or in torpor, only once per
/// referendum." [ANK 20220705] [LSJ 20100705] [LSJ 20011022] [LSJ
/// 20100902-2] [ANK 20180307-2]: from torpor, 2 blood burned, seat 0's two
/// ready untitled Banu Haqim cast 1 each, not offered again; seat 1's Banu
/// Haqim gets none.
#[test]
fn amaravati_gives_each_banu_haqim_a_vote_from_torpor_once() {
    let mut e = game(2);
    let am = e.add_minion(SeatId(0), crypt_card("Amaravati"), 3, true);
    let b1 = e.add_minion(SeatId(0), of_clan("Banu Haqim", 3..=11, 0), 4, false);
    let b2 = e.add_minion(SeatId(0), of_clan("Banu Haqim", 3..=11, 1), 4, false);
    let theirs = e.add_minion(SeatId(1), of_clan("Banu Haqim", 3..=11, 2), 4, false);
    e.state.seats[1].has_edge = true;
    hand(&mut e, 0, &["Political Stranglehold"]);
    pick(&mut e, play(0, 0, b1));
    to_referendum(&mut e);
    pick_use(&mut e, am, 0);
    assert_eq!(blood(&e, am), 1);
    pick_when(&mut e, votes(b1, true));
    pick_when(&mut e, votes(b2, true));
    assert_eq!(tally_now(&e), (2, 0));
    while e.referendum().is_some() {
        let req = request(&mut e);
        assert!(!req.options.iter().any(|o| uses_printed(o, am, 0)), "once");
        assert!(
            !req.options
                .iter()
                .any(|o| *o == votes(theirs, true) || *o == votes(theirs, false)),
            "another Methuselah's Banu Haqim"
        );
        e.choose(fallback(&req));
    }
}

/// The referendum under way's tally now.
fn tally_now(e: &Engine) -> (u32, u32) {
    e.referendum().expect("referendum").tally(&e.state)
}

// ---- Ambrogino Giovanni -----------------------------------------------------

/// Ambrogino Giovanni (advanced): "During your unlock phase, Ambrogino can
/// remove seven cards in your ash heap from the game to gain 2 blood. …
/// +1 bleed." Eight cards: seven removed, 2 blood; six: never offered; his
/// bleed for 2.
#[test]
fn ambrogino_giovanni_trades_seven_ash_cards_for_two_blood() {
    for cards in [8usize, 6] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), adv_card("Ambrogino Giovanni"), 4, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        e.state.seats[0].ash = vec![find_library("Roundhouse"); cards].into();
        assert_eq!(bleeds_for(&mut e, a), 2);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let mut used = false;
        loop {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            match req.options.iter().position(|o| uses_printed(o, a, 0)) {
                Some(i) => {
                    assert_eq!(req.window, WindowLabel::UnlockAbilities);
                    e.choose(i);
                    used = true;
                }
                None => e.choose(fallback(&req)),
            }
        }
        assert_eq!(used, cards >= 7, "{cards}");
        let s = &e.state.seats[0];
        if used {
            assert_eq!((s.ash.len(), s.removed.len()), (1, 7));
            assert_eq!(blood(&e, a), 6);
        } else {
            assert_eq!((s.ash.len(), s.removed.len()), (6, 0));
            assert_eq!(blood(&e, a), 4);
        }
    }
}

// ---- Andreas, The Bard of Crete ---------------------------------------------

/// Andreas, The Bard of Crete: "Once each turn, Andreas may burn 1 blood to
/// get +1 intercept for the current action." "\"Once each turn\" means once
/// during each player's turn." [ANK 20180719-4] Used on a hunt (+1
/// stealth), not again on the next hunt that turn, again in seat 0's next
/// turn.
#[test]
fn andreas_burns_for_intercept_once_each_turn() {
    use zvtes::decision::ActionKind::Hunt;
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let b = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    let n = e.add_minion(
        SeatId(1),
        crypt_card("Andreas, The Bard of Crete"),
        6,
        false,
    );
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    pick(&mut e, action(a, Hunt));
    pick_when(&mut e, block(n));
    pick_use(&mut e, n, 0);
    assert_eq!(blood(&e, n), 5);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[n.index()].locked = false;
    pick(&mut e, action(b, Hunt));
    pick_when(&mut e, block(n));
    let req = request(&mut e);
    assert!(
        !req.options.iter().any(|o| uses_printed(o, n, 0)),
        "this turn"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[n.index()].locked = false;
    e.state.minions[a.index()].locked = false;
    pick(&mut e, action(a, Hunt));
    pick_when(&mut e, block(n));
    pick_use(&mut e, n, 0);
}

// ---- Astrid Thomas ----------------------------------------------------------

/// Seat 0's Astrid Thomas and a vampire of capacity 8 or more (Political
/// Stranglehold's 3 pool), seat 1's titled Tremere T
/// (primogen, 1 vote), Scalpel Tongue's player R ([cel][pre]) and the
/// Edge; Astrid calls Political Stranglehold.
fn astrid_table() -> (Engine, MinionId, MinionId, MinionId) {
    let mut e = game(2);
    let astrid = e.add_minion(SeatId(0), crypt_card("Astrid Thomas"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 8, 0), 4, false);
    let t = e.add_minion(SeatId(1), of_clan("Tremere", 3..=11, 0), 4, false);
    e.state.minions[t.index()].granted_title = Some(Title::Primogen);
    let r = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.state.minions[r.index()].gained_levels = vec!["cel", "pre"];
    e.state.seats[1].has_edge = true;
    hand(&mut e, 0, &["Political Stranglehold"]);
    pick(&mut e, play(0, 0, astrid));
    to_referendum(&mut e);
    (e, astrid, t, r)
}

/// Whether Political Stranglehold passed, its polling closed: seat 0's 3
/// pool gained.
fn passes(e: &mut Engine) -> bool {
    let before = e.state.seats[0].pool;
    close_polling(e);
    request(e);
    e.state.seats[0].pool == before + 3
}

/// Astrid Thomas: "When results are tallied during a referendum, any
/// Tremere who are not abstaining cast votes and ballots with Astrid
/// (unless Astrid abstains)." "Her ability triggers when votes are
/// tallied." [RTR 20001020] Seat 1's Tremere's 1 against counts for her at
/// the tally: 2-0, passed (a 1-1 tie fails).
#[test]
fn astrid_thomas_draws_casting_tremere_to_her_side_at_the_tally() {
    let (mut e, astrid, t, _) = astrid_table();
    pick_when(&mut e, votes(astrid, true));
    pick_when(&mut e, votes(t, false));
    assert_eq!(tally_now(&e), (1, 1), "cast as cast until the tally");
    assert!(passes(&mut e));
}

/// "Tremere who have not yet voted can choose to abstain." [RTR 19941006]
/// The Tremere passes and never casts: Astrid's 1 in favor against seat
/// 1's Edge, a tie, the referendum fails (drawn, it would pass 2-1).
#[test]
fn a_tremere_that_never_casts_escapes_astrid_thomas() {
    let (mut e, astrid, t, _) = astrid_table();
    pick_when(&mut e, votes(astrid, true));
    pick_when(&mut e, Choice::CastEdge { favor: false });
    loop {
        let req = request(&mut e);
        if req.options.contains(&votes(t, true)) {
            e.choose(index_of(&req, Choice::Pass));
            break;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(cast_side(&e, t), None);
    assert!(!passes(&mut e));
}

/// "If her votes are canceled, the other Tremere votes are unaffected by
/// her ability." [RTR 19960530] Scalpel Tongue cancels Astrid's 1 in
/// favor: the Tremere's 1 against stands, the referendum fails.
#[test]
fn astrid_thomas_canceled_moves_no_tremere() {
    let (mut e, astrid, t, r) = astrid_table();
    pick_when(&mut e, votes(astrid, true));
    pick_when(&mut e, votes(t, false));
    hand(&mut e, 1, &["Scalpel Tongue"]);
    pick_when(&mut e, play_on(0, 0, r, Target::Minion(astrid)));
    assert!(!passes(&mut e));
}

/// Protected District: "If a minion who has gained the votes provided
/// against the referendum, is subsequently forced to vote in favor, the
/// provided votes are not counted when votes are tallied" [LSJ 20031115];
/// advanced rules §3.7.6.3. Seat 1's primogen Tremere T and its 3 Protected
/// District votes, in both orders: Astrid moves T's 1 to her side, the 3
/// uncounted. Cast then the card, against the Edge and R's 2 votes: 2-3,
/// fails (the 3 moved too: 5-3). The card then the cast, against the Edge
/// alone: 2-1, passes (T kept against: 1-5).
#[test]
fn astrid_thomas_silences_a_tremere_s_protected_district_votes() {
    for cast_first in [true, false] {
        let (mut e, astrid, t, r) = astrid_table();
        hand(&mut e, 1, &["Protected District"]);
        if cast_first {
            e.state.minions[r.index()].granted_title = Some(Title::Votes(2));
        }
        pick_when(&mut e, votes(astrid, true));
        pick_when(&mut e, Choice::CastEdge { favor: false });
        if cast_first {
            pick_when(&mut e, votes(r, false));
            pick_when(&mut e, votes(t, false));
            pick_when(&mut e, play(0, 0, t));
            request(&mut e);
            assert_eq!(tally_now(&e), (1, 7));
            assert!(!passes(&mut e), "cast first");
        } else {
            pick_when(&mut e, play(0, 0, t));
            pick_when(&mut e, votes(t, false));
            request(&mut e);
            assert_eq!(tally_now(&e), (1, 5));
            assert!(passes(&mut e), "card first");
        }
    }
}

/// Astrid Thomas over Kindred Manipulation ("Change the votes and ballots
/// of a younger vampire to votes and ballots of your choice."), in either
/// order: hers, applied at the tally, is the last used (owner, 2026-11-19;
/// "Whoever uses it last has the final say on his votes" [LSJ
/// 20090304-2]; advanced rules §3.7.6.3). Seat 0's older Ventrue
/// antitribu K calls Political Stranglehold and turns seat 1's primogen
/// Tremere T against, before T casts (T may then cast against only) or
/// after its cast in favor: T's 1 is Astrid's at the tally, 2-0, passed
/// (T kept against: 1-1, fails).
#[test]
fn astrid_thomas_overrides_kindred_manipulation_at_the_tally() {
    for before_cast in [true, false] {
        let mut e = game(2);
        let k = e.add_minion(SeatId(0), of_clan("Ventrue antitribu", 9..=11, 0), 4, false);
        let astrid = e.add_minion(SeatId(0), crypt_card("Astrid Thomas"), 4, false);
        let t = e.add_minion(SeatId(1), of_clan("Tremere", 3..=8, 0), 4, false);
        e.state.minions[t.index()].granted_title = Some(Title::Primogen);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        assert!(e.state.younger_by(t, k, SeatId(0)), "T younger than K");
        hand(
            &mut e,
            0,
            &["Political Stranglehold", "Kindred Manipulation"],
        );
        pick(&mut e, play(0, 0, k));
        to_referendum(&mut e);
        pick_when(&mut e, votes(astrid, true));
        if before_cast {
            pick_when(&mut e, play_at(0, 1, k, t));
            let req = expect(&mut e, SeatId(1), WindowLabel::ReferendumPolling);
            assert!(!req.options.contains(&votes(t, true)), "set against");
            pick_when(&mut e, votes(t, false));
        } else {
            pick_when(&mut e, votes(t, true));
            pick_when(&mut e, play_at(0, 1, k, t));
        }
        request(&mut e);
        assert_eq!(cast_side(&e, t), Some(false), "changed against");
        assert_eq!(tally_now(&e), (1, 1), "cast as cast until the tally");
        let pool = e.state.seats[0].pool;
        close_polling(&mut e);
        request(&mut e);
        assert!(e.state.seats[0].pool > pool, "before cast: {before_cast}");
    }
}

// ---- Azaneal ----------------------------------------------------------------

/// Azaneal: "Actions to put vampires in play cost Azaneal -1 blood or
/// pool." The Embrace (2 blood) costs him 1, the vampire made; Art Scam
/// (pool) or any card putting none in play is not reduced.
#[test]
fn azaneal_embraces_for_one_blood() {
    let mut e = game(2);
    let az = e.add_minion(SeatId(0), crypt_card("Azaneal"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    hand(&mut e, 0, &["Embrace, The"]);
    let minions = e.state.minions.len();
    pick(&mut e, play_on(0, 0, az, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions.len(), minions + 1);
    assert_eq!(blood(&e, az), 3);
    let off = |card: &str| {
        e.state.printed_mod(
            az,
            Stat::CostOff,
            zvtes::state::PrintedCtx {
                card: Some((find_library(card), 0)),
                ..Default::default()
            },
        )
    };
    assert_eq!((off("Embrace, The"), off("Art Scam")), (1, 0));
}

// ---- Basir ------------------------------------------------------------------

/// Master card 0 of the hand played on minion `m`.
fn master_on(m: MinionId) -> Choice {
    Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(m),
    }
}

/// Basir: "Basir gains 1 blood after you use a master phase action to put a
/// master Discipline card on him." Seat 1's Presence on him: nothing; his
/// controller's: capacity 3, 2 blood.
#[test]
fn basir_gains_blood_from_his_own_master_discipline_card() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), crypt_card("Basir"), 1, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    hand(&mut e, 1, &["Presence"]);
    hand(&mut e, 0, &["Presence"]);
    until(&mut e, SeatId(1), WindowLabel::MasterPhase);
    pick(&mut e, master_on(b));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!((e.state.capacity_of(b), blood(&e, b)), (Some(2), 1));
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, master_on(b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((e.state.capacity_of(b), blood(&e, b)), (Some(3), 2));
}

/// Basir: "He cannot block or play reaction cards." Seat 1's companion is
/// offered the block of a bleed and Deflection; Basir neither, [dom] given.
#[test]
fn basir_cannot_block_or_react() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    let b = e.add_minion(SeatId(1), crypt_card("Basir"), 1, false);
    let y = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    e.add_minion(SeatId(2), textless(|c| c.capacity >= 4, 3), 4, false);
    for m in [b, y] {
        e.state.minions[m.index()].gained_levels = vec!["dom"];
    }
    hand(&mut e, 1, &["Deflection"]);
    pick(&mut e, bleed(a));
    let req = offering(&mut e, block(y));
    assert!(!req.options.contains(&block(b)));
    let to = Target::Seat(SeatId(2));
    let req = offering(&mut e, play_on(0, 0, y, to));
    assert!(!req.options.contains(&play_on(0, 0, b, to)));
}

// ---- Text-only vampires (B to D) ---------------------------------------------

/// Black Lotus, Devyn ("Sabbat."), Brazil, Camille Devereux, The Raven,
/// Colin Flynn, Courtland Leighton, Demetrius Slater, Didi Meyers
/// ("Camarilla."), Cardano ("Camarilla Prince of Boston."), Chester DuBois
/// ("Camarilla primogen."), Cristobal Ghiberti, Cristofero Giovanni
/// ("Independent."): no definition, their sect and title generated.
#[test]
fn round33_b_textless_vampires_b_to_d_carry_only_generated_traits() {
    use zvtes::crypt::Sect;
    for (name, sect, title) in [
        ("Black Lotus", Sect::Sabbat, Title::None),
        ("Devyn", Sect::Sabbat, Title::None),
        ("Brazil", Sect::Camarilla, Title::None),
        ("Camille Devereux, The Raven", Sect::Camarilla, Title::None),
        ("Colin Flynn", Sect::Camarilla, Title::None),
        ("Courtland Leighton", Sect::Camarilla, Title::None),
        ("Demetrius Slater", Sect::Camarilla, Title::None),
        ("Didi Meyers", Sect::Camarilla, Title::None),
        ("Cardano", Sect::Camarilla, Title::Prince("Boston")),
        ("Chester DuBois", Sect::Camarilla, Title::Primogen),
        ("Cristobal Ghiberti", Sect::Independent, Title::None),
        ("Cristofero Giovanni", Sect::Independent, Title::None),
    ] {
        let i = crypt_card(name);
        assert!(zvtes::cards::vampire(i).is_none(), "{name} has text");
        let c = &zvtes::crypt::CRYPT[i as usize];
        assert_eq!((c.sect, c.title), (Some(sect), title), "{name}");
    }
}

// ---- Black Cat --------------------------------------------------------------

/// Black Cat: "Equipment costs Black Cat 1 less pool (but never less than 0
/// pool)." A .44 Magnum (2 pool) from hand costs her 1.
#[test]
fn black_cat_equips_for_one_pool_less() {
    let mut e = game(2);
    let cat = e.add_minion(SeatId(0), crypt_card("Black Cat"), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 0), 4, false);
    hand(&mut e, 0, &[".44 Magnum"]);
    pick(&mut e, play(0, 0, cat));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let gun = card_in_play(&e, ".44 Magnum").expect("equipped");
    assert_eq!(e.state.cards[gun.index()].host, Some(cat));
    assert_eq!(e.state.seats[0].pool, 10 - 1);
}

/// Concealed Weapon: "The weapon cannot cost 3 or more pool": "Cost
/// modifications that are not limited to cards "played" are considered
/// before checking for card cost (eg. Black Cat can equip with a Combat
/// Shotgun …)" [LSJ 20040701] [LSJ 20080125] [ANK 20181215]. Blocked, Black
/// Cat takes the Shotgun (3 pool) for 2; another vampire is offered no
/// Concealed Weapon with only the Shotgun in hand.
#[test]
fn black_cat_finds_a_combat_shotgun_with_concealed_weapon() {
    for cat in [true, false] {
        let mut e = game(2);
        let a = if cat {
            e.add_minion(SeatId(0), crypt_card("Black Cat"), 4, false)
        } else {
            e.add_minion(SeatId(0), of_clan("Brujah", 4..=8, 0), 4, false)
        };
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
        hand(&mut e, 0, &["Concealed Weapon", "Combat Shotgun"]);
        fight(&mut e, a, x);
        if !cat {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
            continue;
        }
        pick_when(&mut e, play(0, 0, a));
        request(&mut e);
        let gun = card_in_play(&e, "Combat Shotgun").expect("equipped");
        assert_eq!(e.state.cards[gun.index()].host, Some(a));
        assert_eq!(e.state.seats[0].pool, 10 - 2, "3 pool, 1 less");
    }
}

/// Black Cat: "The reduction applies for other effects that check the
/// "cost" of equipment on her." "If she is equipped with something and that
/// equipment is transferred to another minion, then it regains its normal
/// pool value. Conversely, if a piece of equipment moves onto her, its
/// effective pool cost is lowered by 1." [RTR 19941109] Bulscu steals
/// equipment "costing 3 or fewer": a Flamethrower (4 pool) on her costs 3,
/// a target; one on her companion is not. Swapped, the other one is.
#[test]
fn bulscu_reads_the_cost_of_equipment_on_black_cat() {
    for swapped in [false, true] {
        let mut e = game(2);
        let b = e.add_minion(SeatId(0), crypt_card("Bulscu"), 4, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        e.state.seats[0].has_edge = true;
        let cat = e.add_minion(SeatId(1), crypt_card("Black Cat"), 4, false);
        let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        for m in [cat, z] {
            e.state.minions[m.index()].locked = true;
        }
        let first = in_play(&mut e, 1, "Flamethrower", Some(cat));
        let second = in_play(&mut e, 1, "Flamethrower", Some(z));
        if swapped {
            e.state.cards[first.index()].host = Some(z);
            e.state.cards[second.index()].host = Some(cat);
        }
        let (on_cat, on_other) = if swapped {
            (second, first)
        } else {
            (first, second)
        };
        let at = |c: CardId| Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Vampire(b),
            ability: 1,
            actor: Some(b),
            target: Target::Card(c),
        };
        let req = request(&mut e);
        assert!(!req.options.contains(&at(on_other)), "swapped {swapped}");
        e.choose(index_of(&req, at(on_cat)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.cards[on_cat.index()].host, Some(b));
    }
}

// ---- Cagliostro, The Grand Copht --------------------------------------------

/// Cagliostro: "If Cagliostro is ready at the start of your discard phase,
/// you get +1 discard phase action." Ayse Dhanial ("you can use a discard
/// phase action to add 1 blood to her") gains 2 with him ready, 1 with him
/// in torpor.
#[test]
fn cagliostro_gives_a_discard_phase_action_if_ready() {
    for (torpor, gained) in [(false, 2), (true, 1)] {
        let mut e = game(2);
        e.add_minion(
            SeatId(0),
            crypt_card("Cagliostro, The Grand Copht"),
            4,
            torpor,
        );
        let ayse = e.add_minion(SeatId(0), crypt_card("Ayse Dhanial"), 1, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 0), 4, false);
        loop {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                break;
            }
            match req.options.iter().position(|o| uses_printed(o, ayse, 0)) {
                Some(i) => e.choose(i),
                None => e.choose(fallback(&req)),
            }
        }
        assert_eq!(blood(&e, ayse), 1 + gained, "torpor {torpor}");
    }
}

/// Cagliostro: "If he is blocked and combat occurs, he can put 1 corruption
/// counter on the blocking minion after that combat ends." After the
/// block's combat, the blocker is the one target (its companion is not);
/// chosen, it carries 1 of seat 0's corruption counters. An unblocked bleed
/// offers nothing.
#[test]
fn cagliostro_corrupts_his_blocker_after_the_combat() {
    let mut e = game(2);
    let c = e.add_minion(
        SeatId(0),
        crypt_card("Cagliostro, The Grand Copht"),
        4,
        false,
    );
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let y = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    let at = |t: MinionId| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(c),
        ability: 0,
        actor: None,
        target: Target::Minion(t),
    };
    fight(&mut e, c, x);
    let req = offering(&mut e, at(x));
    assert!(e.combat().is_none(), "after the combat");
    assert!(!req.options.contains(&at(y)));
    e.choose(index_of(&req, at(x)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[x.index()].corruption_of(SeatId(0)), 1);

    let mut e = game(2);
    let c = e.add_minion(
        SeatId(0),
        crypt_card("Cagliostro, The Grand Copht"),
        4,
        false,
    );
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    pick(&mut e, bleed(c));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_printed(o, c, 0)
    });
}

// ---- Callirus ---------------------------------------------------------------

/// Callirus: "During your unlock phase, Callirus can burn 1 blood to burn
/// one card on him." His own .44 Magnum and seat 1's Shell Break on him are
/// both offered, an effect-only stand-in on him (rules-qa 21.1) and a card
/// on another minion are not; the Shell Break chosen goes to seat 1's ash
/// heap, Callirus at 1 blood. In torpor too; at 0 blood, not offered.
#[test]
fn callirus_burns_a_card_on_him_for_a_blood() {
    for (torpor, blood_on) in [(false, 2), (true, 2), (false, 0)] {
        let mut e = game(2);
        let c = e.add_minion(SeatId(0), crypt_card("Callirus"), blood_on, torpor);
        let o = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        let gun = in_play(&mut e, 0, ".44 Magnum", Some(c));
        let shell = in_play(&mut e, 1, "Shell Break", Some(c));
        let stand_in = in_play(&mut e, 1, "Shell Break", Some(c));
        e.state.cards[stand_in.index()].effect_only = true;
        let other = in_play(&mut e, 0, ".44 Magnum", Some(o));
        let at = |t: CardId| Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Vampire(c),
            ability: 0,
            actor: None,
            target: Target::Card(t),
        };
        if blood_on == 0 {
            until(&mut e, SeatId(1), WindowLabel::MinionPhase);
            // Empty, he hunted in seat 0's turn: emptied again.
            e.state.minions[c.index()].blood = 0;
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
                uses_printed(o, c, 0)
            });
            continue;
        }
        let req = offering(&mut e, at(shell));
        assert_eq!(req.window, WindowLabel::UnlockAbilities);
        assert!(req.options.contains(&at(gun)));
        assert!(!req.options.contains(&at(stand_in)), "a stand-in");
        assert!(!req.options.contains(&at(other)));
        e.choose(index_of(&req, at(shell)));
        assert!(!e.state.cards[shell.index()].in_play);
        assert!(ash_has(&e, 1, "Shell Break"), "torpor {torpor}");
        assert_eq!(blood(&e, c), 1);
    }
}

// ---- Carmine Giovanni -------------------------------------------------------

/// Carmine Giovanni: "Wraiths come into play with 1 additional life." "Her
/// additional life ability applies … when she is in torpor." [LSJ 20011022]
/// [LSJ 20100902-2] With seat 1's Carmine in torpor, seat 0 recruits a
/// Screamer (wraith with 1 life; Hecata) at 2 life and employs a
/// Nar-Sheptha (wraith with 1 life; Banu Haqim) at 2; a Political Ally
/// keeps its printed life.
#[test]
fn carmine_gives_wraiths_an_extra_life() {
    for carmine in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_clan("Hecata", 4..=8, 0), 4, false);
        let b = e.add_minion(SeatId(0), of_clan("Banu Haqim", 4..=8, 0), 4, false);
        if carmine {
            e.add_minion(SeatId(1), crypt_card("Carmine Giovanni"), 4, true);
        }
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
        hand(&mut e, 0, &["Screamer", "Nar-Sheptha"]);
        pick(&mut e, play(0, 0, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let screamer = e.state.minions.len() - 1;
        assert_eq!(e.state.minions[screamer].blood, 1 + u8::from(carmine));
        pick(&mut e, play(0, 0, b));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let nar = card_in_play(&e, "Nar-Sheptha").expect("employed");
        assert_eq!(e.state.cards[nar.index()].counters, 1 + u8::from(carmine));
        let ally = e.add_ally(SeatId(0), find_library("Political Ally"));
        let printed = zvtes::library::LIBRARY[find_library("Political Ally") as usize]
            .body
            .expect("an ally")
            .life;
        assert_eq!(blood(&e, ally), printed);
    }
}

/// Carmine Giovanni: wraiths "come into play with 1 additional life": their
/// starting life, fixed as they enter (rules-qa 124.147; advanced rules
/// §5.5.2: "later change does not move it"). Renewed Vigor [obe] ("Add
/// enough life to an ally or retainer to reach their starting life") on a
/// Nar-Sheptha: employed with Carmine in play, down to 1, Carmine burned
/// since, back to 2; employed before Carmine entered play, kept at 1.
#[test]
fn carmine_s_extra_life_is_a_wraith_s_starting_life() {
    for carmine_first in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        let b = e.add_minion(SeatId(0), of_clan("Banu Haqim", 4..=8, 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["obe"];
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
        let crypt = crypt_card("Carmine Giovanni");
        let carmine = carmine_first.then(|| e.add_minion(SeatId(1), crypt, 4, true));
        hand(&mut e, 0, &["Nar-Sheptha", "Renewed Vigor"]);
        pick(&mut e, play(0, 0, b));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let nar = card_in_play(&e, "Nar-Sheptha").expect("employed");
        assert_eq!(
            e.state.cards[nar.index()].counters,
            1 + u8::from(carmine_first)
        );
        match carmine {
            Some(c) => {
                e.state.cards[nar.index()].counters = 1;
                e.burn(c);
            }
            None => {
                e.add_minion(SeatId(1), crypt, 4, true);
            }
        }
        pick(&mut e, play_on(0, 3, a, Target::Card(nar)));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(ash_has(&e, 0, "Renewed Vigor"));
        assert_eq!(
            e.state.cards[nar.index()].counters,
            1 + u8::from(carmine_first),
            "Carmine first {carmine_first}"
        );
    }
}

/// Carmine Giovanni: "Carmine can lock to give any ally +1 stealth", from
/// torpor too [LSJ 20011022] [LSJ 20100902-2]. Seat 0's Screamer bleeds,
/// seat 1's untouched vampire blocks: Carmine, in torpor, locks, the block fails,
/// the bleed lands. A vampire's bleed is offered nothing.
#[test]
fn carmine_locks_to_sneak_an_acting_ally() {
    let mut e = game(2);
    let carmine = e.add_minion(SeatId(0), crypt_card("Carmine Giovanni"), 4, true);
    let v = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let s = e.add_ally(SeatId(0), find_library("Screamer"));
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let y = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    pick(&mut e, bleed(v));
    pick_when(&mut e, block(x));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_printed(o, carmine, 0)
    });
    pick(&mut e, bleed(s));
    pick_when(&mut e, block(y));
    pick_use(&mut e, carmine, 0);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[carmine.index()].locked);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "the Screamer's bleed");
}

/// Carmine Giovanni (group Personal titles): "Keeps their title and votes
/// if they changes sects." [LSJ 20010714] Made Anarch, still "2 votes
/// (titled)".
#[test]
fn carmine_keeps_her_votes_through_a_sect_change() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), crypt_card("Carmine Giovanni"), 4, false);
    e.state.minions[c.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    assert_eq!(e.state.title_of(c), Title::Votes(2));
}

// ---- Cassandra, Magus Prime -------------------------------------------------

/// Cassandra: "If Cassandra is ready, your hand size is increased by one. +1
/// strength." Hand size 8 while ready, 7 in torpor; her hand strike deals 2.
#[test]
fn cassandra_widens_the_hand_while_ready_and_strikes_for_two() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), crypt_card("Cassandra, Magus Prime"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    assert_eq!(e.state.hand_size(SeatId(0)), 8);
    fight(&mut e, c, x);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2);
    e.state.minions[c.index()].in_torpor = true;
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
}

// ---- Crusher ----------------------------------------------------------------

/// Crusher: "Once each combat, Crusher may dodge as a strike that costs 1
/// blood. +1 strength." Round 1: his dodge, 1 blood, the blocker's hand
/// strike dodged; the blocker presses (Nimble Feet [cel]); round 2: no
/// dodge offered, his hand strike deals 2.
#[test]
fn crusher_dodges_for_a_blood_once_each_combat() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), crypt_card("Crusher"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["cel"];
    hand(&mut e, 1, &["Nimble Feet"]);
    fight(&mut e, c, x);
    pick_use(&mut e, c, 0);
    pick_when(&mut e, play(0, 0, x));
    assert_eq!((blood(&e, c), blood(&e, x)), (4 - 1, 4), "dodged");
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_printed(o, c, 0)
    });
    assert_eq!((blood(&e, c), blood(&e, x)), (4 - 1 - 1, 4 - 2));
}

// ---- Democritus -------------------------------------------------------------

/// A vampire of seat 0's and its copy of seat 1's, contested; Democritus
/// with seat 0, in torpor if `torpor`.
fn democritus_contest(torpor: bool) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    e.add_minion(SeatId(0), crypt_card("Democritus"), 4, torpor);
    let card = textless(|c| c.capacity >= 4, 0);
    let own = e.add_minion(SeatId(0), card, 3, false);
    let other = e.add_minion(SeatId(1), card, 3, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    for m in [own, other] {
        e.state.minions[m.index()].contested = true;
    }
    (e, own, other)
}

/// Democritus: "If Democritus is ready, any Methuselah contesting a card
/// with you burns 1 extra pool to avoid yielding that card." Seat 1 keeps
/// its copy for 2 pool; seat 0, his controller, for 1. Democritus in
/// torpor: 1.
#[test]
fn democritus_makes_rivals_pay_two_pool_to_keep_contesting() {
    use zvtes::game::Contest;
    for torpor in [false, true] {
        let (mut e, own, other) = democritus_contest(torpor);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        pick(&mut e, Choice::Pass);
        pick_when(&mut e, Choice::PayContest(Contest::Card(other)));
        assert_eq!(e.state.seats[1].pool, 10 - 1 - u8::from(!torpor) as i32);
        pick_when(&mut e, Choice::PayContest(Contest::Card(own)));
        assert_eq!(e.state.seats[0].pool, 10 - 1, "torpor {torpor}");
    }
}

/// Democritus: a rival with 1 pool cannot pay the 2 (a payment the payer
/// does not have is not offered, rules-digest D3): its copy yielded,
/// burned.
#[test]
fn democritus_bars_a_payment_short_of_his_toll() {
    use zvtes::game::Contest;
    let (mut e, _, other) = democritus_contest(false);
    e.state.seats[1].pool = 1;
    pick(&mut e, Choice::Pass);
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        Choice::PayContest(Contest::Card(other)),
    );
    assert!(!e.state.minions[other.index()].in_play, "yielded");
    assert_eq!(e.state.seats[1].pool, 1);
}

/// Democritus: "Does not affect the cost to contest titles, since it is
/// paid in vampire blood, not pool." [RTR 19950509] Seat 1's vampire made a
/// Ventrue Justicar contests his title: it keeps it for 1 blood, its pool
/// untouched.
#[test]
fn democritus_leaves_title_contests_at_one_blood() {
    use zvtes::game::Contest;
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), crypt_card("Democritus"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let rival = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 1, false);
    e.state.minions[rival.index()].granted_title = Some(Title::Justicar("Ventrue"));
    for m in [d, rival] {
        e.state.minions[m.index()].title_contested = true;
    }
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, Choice::PayContest(Contest::Title(rival)));
    assert_eq!((blood(&e, rival), e.state.seats[1].pool), (0, 10));
}

/// Democritus beside Olugbenga ("you can use the blood on that vampire or on
/// Olugbenga as pool to pay for that contest"): seat 1 owes 2, paid one at
/// a time, 1 blood from Olugbenga, then 1 pool.
#[test]
fn democritus_toll_is_paid_unit_by_unit_with_olugbenga() {
    use zvtes::game::Contest;
    let (mut e, _, other) = democritus_contest(false);
    let o = e.add_minion(SeatId(1), crypt_card("Olugbenga"), 4, false);
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, Choice::PayContest(Contest::Card(other)));
    pick(&mut e, Choice::BurnBloodFrom { minion: o });
    let req = request(&mut e);
    assert!(
        req.options.contains(&Choice::BurnBloodFrom { minion: o }),
        "a second unit"
    );
    e.choose(index_of(&req, Choice::BurnPool));
    assert_eq!((blood(&e, o), e.state.seats[1].pool), (4 - 1, 10 - 1));
    assert!(e.state.minions[other.index()].in_play);
}

// ---- Text-only vampires (R to T) ---------------------------------------------

/// Ricki Van Demsi, Roland Bishop, Roland Loussarian, Roman Alexander,
/// Roreca Quaid, Rufina Soledad, Sammy, Sarah Cobbler, Thomas Thorne
/// ("Camarilla."), Sir Walter Nash ("Camarilla Prince of Chicago."), Sahira
/// Siraj ("Independent."), Shane Grimald ("Sabbat."), Skryta Zyleta
/// ("Sabbat. Black Hand."): no definition, their sect, title and Black Hand
/// generated; Skryta's Black Hand kept through a sect change [LSJ 20070322]
/// [ANK 20180807].
#[test]
fn round33_b_textless_vampires_r_to_t_carry_only_generated_traits() {
    use zvtes::crypt::Sect;
    for (name, sect, title) in [
        ("Ricki Van Demsi", Sect::Camarilla, Title::None),
        ("Roland Bishop", Sect::Camarilla, Title::None),
        ("Roland Loussarian", Sect::Camarilla, Title::None),
        ("Roman Alexander", Sect::Camarilla, Title::None),
        ("Roreca Quaid", Sect::Camarilla, Title::None),
        ("Rufina Soledad", Sect::Camarilla, Title::None),
        ("Sammy", Sect::Camarilla, Title::None),
        ("Sarah Cobbler", Sect::Camarilla, Title::None),
        ("Thomas Thorne", Sect::Camarilla, Title::None),
        ("Sir Walter Nash", Sect::Camarilla, Title::Prince("Chicago")),
        ("Sahira Siraj", Sect::Independent, Title::None),
        ("Shane Grimald", Sect::Sabbat, Title::None),
        ("Skryta Zyleta", Sect::Sabbat, Title::None),
    ] {
        let i = crypt_card(name);
        assert!(zvtes::cards::vampire(i).is_none(), "{name} has text");
        let c = &zvtes::crypt::CRYPT[i as usize];
        assert_eq!((c.sect, c.title), (Some(sect), title), "{name}");
        assert_eq!(c.black_hand, name == "Skryta Zyleta", "{name}");
    }
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), crypt_card("Skryta Zyleta"), 4, false);
    e.state.minions[s.index()].sect_override = Some(Sect::Independent);
    assert!(e.state.is_black_hand(s), "a trait, not a sect's");
}

// ---- Roxanne, Rectrix of the 13th Floor --------------------------------------

/// Is `o` vampire `m`'s printed ability `ability` at minion `t`?
fn uses_at(o: &Choice, m: MinionId, ability: u8, t: MinionId) -> bool {
    uses_printed(o, m, ability)
        && matches!(o, Choice::UseAbility { target: Target::Minion(x), .. } if *x == t)
}

/// Roxanne: "Camarilla primogen: Roxanne may enter combat with a vampire of
/// capacity below 4 controlled by your predator or prey as a +1 stealth Ⓓ
/// action." Four seats: the prey's and the predator's capacity-3 vampires
/// offered, not the prey's capacity-4 one nor the cross-table seat's; at
/// the prey's: stealth 1, unblocked, Roxanne and it exchange hand strikes.
#[test]
fn roxanne_enters_combat_with_a_small_vampire_of_her_prey() {
    let mut e = game(4);
    let r = e.add_minion(
        SeatId(0),
        crypt_card("Roxanne, Rectrix of the 13th Floor"),
        4,
        false,
    );
    let small = |n| textless(|c| c.capacity == 3, n);
    let prey = e.add_minion(SeatId(1), small(0), 3, false);
    let big = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 4, false);
    let across = e.add_minion(SeatId(2), small(1), 3, false);
    let pred = e.add_minion(SeatId(3), small(2), 3, false);
    let req = request(&mut e);
    let offered = |t| req.options.iter().any(|o| uses_at(o, r, 0, t));
    assert!(offered(prey) && offered(pred));
    assert!(!offered(big), "capacity 4");
    assert!(!offered(across), "not a neighbour's");
    let at = *req.options.iter().find(|o| uses_at(o, r, 0, prey)).unwrap();
    e.choose(index_of(&req, at));
    request(&mut e);
    assert_eq!(e.action().expect("her action").stealth(&e.state), 1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(
        (blood(&e, r), blood(&e, prey)),
        (3, 2),
        "their hand strikes"
    );
}

// ---- Sabine Lafitte -----------------------------------------------------------

/// Through seat 0's next turn, taking every option `want` matches: how many.
fn next_turn_taking(e: &mut Engine, mut want: impl FnMut(&Choice) -> bool) -> usize {
    let start = e.state.turn;
    let mut n = 0;
    loop {
        let req = request(e);
        if req.seat == SeatId(0) && req.window == WindowLabel::MinionPhase && e.state.turn > start {
            return n;
        }
        let wanted: Vec<bool> = req.options.iter().map(&mut want).collect();
        if let Some(i) = wanted.iter().position(|&w| w) {
            e.choose(i);
            n += 1;
            continue;
        }
        e.choose(fallback(&req));
    }
}

/// Sabine Lafitte: "During your unlock phase, you may move 1 blood from
/// Sabine to any controlled Tremere." "The ability can be used when in
/// torpor." [LSJ 20011022] From torpor, to seat 1's Tremere: any
/// Methuselah's, a torpid own Tremere and herself offered too, not a
/// Ventrue; once.
#[test]
fn sabine_lafitte_feeds_a_tremere_from_torpor() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), crypt_card("Sabine Lafitte"), 3, true);
    let own = e.add_minion(SeatId(0), of_clan("Tremere", 4..=11, 0), 1, true);
    let ven = e.add_minion(SeatId(0), of_clan("Ventrue", 4..=11, 0), 1, false);
    let theirs = e.add_minion(SeatId(1), of_clan("Tremere", 4..=11, 1), 1, false);
    let feed = |o: &Choice, t| uses_at(o, s, 0, t);
    let (mut seen_own, mut seen_ven, mut seen_self) = (false, false, false);
    let used = next_turn_taking(&mut e, |o| {
        seen_own |= feed(o, own);
        seen_ven |= feed(o, ven);
        seen_self |= feed(o, s);
        feed(o, theirs)
    });
    assert!(seen_own && seen_self, "a torpid Tremere, herself");
    assert!(!seen_ven, "a Tremere only");
    assert_eq!((used, blood(&e, s), blood(&e, theirs)), (1, 2, 2));
}

// ---- Sebastian Marley ---------------------------------------------------------

/// Sebastian Marley: "Camarilla: +1 strength." His hand strike deals 2.
#[test]
fn sebastian_marley_strikes_for_two() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), crypt_card("Sebastian Marley"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    fight(&mut e, a, x);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 2);
}

// ---- Sylvester Simms ----------------------------------------------------------

/// Sylvester Simms: "If Sylvester goes into torpor, each Methuselah burns 1
/// pool." Blocked by Sebastian Marley's 2 damage with 1 blood: to torpor,
/// every seat's pool -1, his controller's too.
#[test]
fn sylvester_simms_going_to_torpor_burns_every_pool() {
    let mut e = game(3);
    let s = e.add_minion(SeatId(0), crypt_card("Sylvester Simms"), 1, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), crypt_card("Sebastian Marley"), 4, false);
    e.add_minion(SeatId(2), textless(|c| c.capacity >= 4, 1), 4, false);
    fight(&mut e, s, x);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(torpid(&e, s));
    assert_eq!(pool(&e), [9, 9, 9]);
}

// ---- Tatiana Stepanova, Alastor ------------------------------------------------

/// Tatiana Stepanova: "You and Tatiana can play cards requiring a Brujah or
/// a Gangrel as if she were of that required clan." [LSJ 20101013] [ANK
/// 20190228] [ANK 20200705-2] [LSJ 20070708] [RTR 20070707] "…not treated
/// as meeting the requirements for effects the card has from being in
/// play": she recruits 47th Street Royals (Brujah) and her Methuselah
/// plays Brujah Debate (requires a Brujah) with only her; in its master
/// phase she is no Brujah to lock, and her hand strike gets no +1.
#[test]
fn tatiana_stepanova_plays_brujah_and_gangrel_cards() {
    let mut e = game(2);
    let t = e.add_minion(
        SeatId(0),
        crypt_card("Tatiana Stepanova, Alastor"),
        4,
        false,
    );
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 0), 4, false);
    hand(&mut e, 0, &["47th Street Royals"]);
    offering(&mut e, play(0, 0, t));

    let mut e = game(2);
    let t = e.add_minion(
        SeatId(0),
        crypt_card("Tatiana Stepanova, Alastor"),
        4,
        false,
    );
    let x = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=11, 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=11, 1), 4, false);
    hand(&mut e, 0, &["Brujah Debate"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(card_in_play(&e, "Brujah Debate").is_some());
    assert!(!e.state.minions[t.index()].locked, "not locked as a Brujah");
    e.choose(index_of(&req, bleed(t)));
    pick_when(&mut e, block(x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 3, "no +1 strength");
}

/// Tatiana Stepanova: "The ability cannot be used to fulfill card
/// requirements if they are not played normally, like when using … The
/// Summoning for an ally." [LSJ 20050119] Her The Summoning [PRE] offers
/// Vagabond Mystic, not 47th Street Royals (Brujah).
#[test]
fn tatiana_stepanova_summons_no_brujah_ally() {
    let mut e = game(2);
    let t = e.add_minion(
        SeatId(0),
        crypt_card("Tatiana Stepanova, Alastor"),
        4,
        false,
    );
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let [royals, mystic] = ["47th Street Royals", "Vagabond Mystic"].map(find_library);
    for c in [royals, mystic, find_library("Form of Mist")] {
        e.state.seats[0].library.push(c);
    }
    hand(&mut e, 0, &["Summoning, The"]);
    fill_hand(&mut e, 0);
    pick(&mut e, play(0, 1, t));
    let take = |card| Choice::TakeFrom {
        zone: zvtes::decision::Zone::Library,
        card,
    };
    let req = offering(&mut e, take(mystic));
    assert!(!req.options.contains(&take(royals)), "not played normally");
}

// ---- Sonja Blue ---------------------------------------------------------------

fn sonja() -> u16 {
    crypt_card("Sonja Blue")
}

/// `card` in `seat`'s uncontrolled region with `counters`.
fn uncontrolled(e: &mut Engine, seat: usize, card: u16, counters: u8) {
    e.state.seats[seat]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card,
            copy: Default::default(),
            counters,
            banished: None,
        });
}

/// The minion of `card` most recently put in play.
fn last_of(e: &Engine, card: u16) -> MinionId {
    let i = e
        .state
        .minions
        .iter()
        .rposition(|m| m.kind == zvtes::state::MinionKind::Vampire(card))
        .expect("in play");
    MinionId(i as u32)
}

/// Sonja Blue: "Gain 4 pool when Sonja is moved to the ready region in
/// your influence phase." "Some effects check to see if a minion entered
/// play 'during' an influence phase … not Undue Influence" [RTR
/// 20080809]: influenced out, +4; brought out by Undue Influence [qui] in
/// the minion phase, nothing.
#[test]
fn sonja_blue_gains_four_pool_influenced_out() {
    let cap = zvtes::crypt::CRYPT[sonja() as usize].capacity;
    let mut e = game(2);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    uncontrolled(&mut e, 0, sonja(), cap);
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    let before = e.state.seats[0].pool;
    e.choose(index_of(&req, Choice::InfluenceOut { slot: 0 }));
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, before + 4);

    let (mut e, a, _) = undue(&["qui"]);
    slot(&mut e, 0, sonja(), cap - 1);
    hand(&mut e, 0, &["Undue Influence"]);
    pick(&mut e, play_on(0, 2, a, Target::Uncontrolled(0)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[last_of(&e, sonja()).index()].in_play);
    assert_eq!(e.state.seats[0].pool, 10, "not in the influence phase");
}

/// Hermana Hambrienta: "When moved to the ready region in your influence
/// phase, this vampire burns 2 blood or is burned." Brought out by Undue
/// Influence [qui] in the minion phase: no burn asked [RTR 20080809]
/// (rules-qa 68.3).
#[test]
fn hermana_brought_out_in_the_minion_phase_burns_nothing() {
    let menor = crypt_card("Hermana Hambrienta Menor");
    let (mut e, a, _) = undue(&["qui"]);
    slot(&mut e, 0, menor, 1);
    hand(&mut e, 0, &["Undue Influence"]);
    pick(&mut e, play_on(0, 2, a, Target::Uncontrolled(0)));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        Choice::BurnBloodToAvoid,
    );
    let h = last_of(&e, menor);
    assert!(e.state.minions[h.index()].in_play);
    assert_eq!(blood(&e, h), 2);
}

/// Sonja Blue: "When contesting her, the new copy provides 4 pool, then
/// the old copy is removed from the game (for leaving play due to
/// contest). The new copy remains face down, out-of-play, ready to be
/// turned face up during her next unlock phase." [LSJ 20070829-2] Seat 0
/// influences out a copy seat 1 controls: +4; seat 1's removed, not in its
/// ash heap; seat 0's contested, then in play at its next turn, uncontested.
#[test]
fn sonja_blue_contested_removes_the_copy_in_play() {
    let cap = zvtes::crypt::CRYPT[sonja() as usize].capacity;
    let mut e = game(3);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let theirs = e.add_minion(SeatId(1), sonja(), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(2), textless(|c| c.capacity >= 4, 2), 4, false);
    uncontrolled(&mut e, 0, sonja(), cap);
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    let before = e.state.seats[0].pool;
    e.choose(index_of(&req, Choice::InfluenceOut { slot: 0 }));
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, before + 4);
    assert!(!e.state.minions[theirs.index()].in_play);
    assert!(e.state.seats[1].removed_crypt.contains(&sonja()));
    assert!(!e.state.seats[1].ash_crypt.contains(&sonja()));
    let ours = last_of(&e, sonja());
    assert!(e.state.minions[ours.index()].contested);
    let pool = e.state.seats[0].pool;
    next_turn_taking(&mut e, |_| false);
    let v = &e.state.minions[ours.index()];
    assert!(v.present() && !v.contested);
    assert!(e.state.seats[0].pool >= pool, "no contest paid");
}

/// Sonja Blue: "She may block as an ally (but remains a vampire in
/// combat)." "'block as an ally' only affects the 'who can block this
/// action' check." [LSJ 20050224-1] A hunt made unblockable by vampires
/// (Beast Meld [ANI][PRO]): Sonja offered, another vampire not; an action allies
/// cannot block (Visions of Gehenna [pre]): Sonja offered, as a vampire.
#[test]
fn sonja_blue_blocks_as_an_ally_or_a_vampire() {
    let table = |card: u16, levels: &[&'static str]| {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), card, 4, false);
        e.state.minions[a.index()].gained_levels = levels.to_vec();
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        let s = e.add_minion(SeatId(1), sonja(), 4, false);
        let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        (e, a, s, x)
    };
    let meld = fixture_vampire(
        |c| ["ANI", "PRO"].iter().all(|d| c.disciplines.contains(d)),
        0,
    );
    let (mut e, a, s, x) = table(meld, &[]);
    hand(&mut e, 0, &["Beast Meld"]);
    pick(&mut e, action(a, zvtes::decision::ActionKind::Hunt));
    pick_when(&mut e, play(0, 1, a));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(s)));
    assert!(!req.options.contains(&block(x)), "vampires cannot block");

    let (mut e, a, s, _) = table(of_clan("Ravnos", 4..=11, 0), &["pre"]);
    hand(&mut e, 0, &["Visions of Gehenna"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(s)), "allies cannot block");
}

/// Sonja Blue: "The Unmasking will not give her intercept." [LSJ 20060503]
/// A vampire's political action with The Unmasking in play: an ally's
/// intercept 1, Sonja's 0.
#[test]
fn sonja_blue_gets_no_unmasking_intercept() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    let s = e.add_minion(SeatId(1), sonja(), 4, false);
    let neph = e.add_ally(SeatId(1), find_library("Nephandus"));
    in_play(&mut e, 1, "Unmasking, The", None);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, a));
    request(&mut e);
    let act = e.action().expect("the political action");
    assert_eq!(act.intercept_of(&e.state, neph), 1);
    assert_eq!(act.intercept_of(&e.state, s), 0);
}

/// Sonja Blue: "Sonja may remove a vampire's title as a Ⓓ action." Seat
/// 1's torpid Sir Walter Nash (Prince of Chicago): directed at seat 1, his
/// title gone at resolution.
#[test]
fn sonja_blue_removes_a_title() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), sonja(), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let nash = e.add_minion(SeatId(1), crypt_card("Sir Walter Nash"), 2, true);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let req = request(&mut e);
    let at = *req
        .options
        .iter()
        .find(|o| uses_at(o, s, 0, nash))
        .expect("a titled vampire, torpid too");
    e.choose(index_of(&req, at));
    request(&mut e);
    let act = e.action().expect("her action");
    assert_eq!(act.directed_seat(&e.state), Some(SeatId(1)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.title_of(nash), Title::None);
}

/// Sonja Blue: "During your prey's discard phase, your predator may burn 1
/// pool to move Sonja to his or her ready region." Three seats, seat 0's
/// Sonja in torpor: offered to seat 2 (seat 0's predator) in seat 1's
/// discard phase only; taken, seat 2 -1 pool, Sonja seat 2's and ready.
#[test]
fn sonja_blue_taken_by_the_predator_from_torpor() {
    let mut e = game(3);
    let s = e.add_minion(SeatId(0), sonja(), 2, true);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(2), textless(|c| c.capacity >= 4, 2), 4, false);
    for _ in 0..400 {
        let req = request(&mut e);
        if let Some(i) = req.options.iter().position(|o| uses_printed(o, s, 1)) {
            assert_eq!(req.seat, SeatId(2));
            assert_eq!(
                (e.state.active, e.phase()),
                (SeatId(1), zvtes::game::Phase::Discard)
            );
            e.choose(i);
            request(&mut e);
            let v = &e.state.minions[s.index()];
            assert_eq!((v.owner, v.in_torpor), (SeatId(2), false));
            assert_eq!(e.state.seats[2].pool, 9);
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("never offered");
}

/// Sonja Blue: "If Sonja leaves play, remove her from the game." Burned, the
/// burn happened first (advanced rules §1.10.1): diablerized in torpor, she
/// is removed, not in the ash heap; Mina Grotius still gains 3 blood for a
/// neighbour's vampire burned.
#[test]
fn sonja_blue_burned_is_removed_from_the_game() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let mina = e.add_minion(SeatId(0), crypt_card("Mina Grotius"), 1, false);
    let s = e.add_minion(SeatId(1), sonja(), 2, true);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: zvtes::decision::ActionKind::Diablerie,
            target: Some(s),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[s.index()].in_play);
    assert!(e.state.seats[1].removed_crypt.contains(&sonja()));
    assert!(!e.state.seats[1].ash_crypt.contains(&sonja()));
    assert_eq!(blood(&e, mina), 4, "her burn noticed");
}

// ==== Fix pass: combat readings =============================================

/// The strike of the weapon `w` (its ability `ability`) by its bearer.
fn cb_weapon(w: zvtes::state::CardId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(w),
        ability,
        actor: None,
        target: Target::None,
    }
}

// ---- Backflip ---------------------------------------------------------------

/// Backflip: "Only usable at long range. Strike: dodge, with an optional
/// press." No Discipline or clan: an ally plays it. A maneuvers to long
/// range (Flash [cel]) and strikes with Earth Swords [myt] (2R); seat 1's
/// Loyal Street Gang dodges with Backflip, its 2 life untouched.
#[test]
fn backflip_is_played_by_an_ally() {
    let (mut e, a, _) = duel(&["cel", "myt"], &[]);
    let gang = e.add_ally(SeatId(1), find_library("Loyal Street Gang"));
    hand(&mut e, 0, &["Flash", "Earth Swords"]);
    hand(&mut e, 1, &["Backflip"]);
    fight(&mut e, a, gang);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(range(&e), Range::Long);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, gang));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Backflip"));
    assert_eq!(blood(&e, gang), 2, "dodged");
}

// ---- Dead Hand / Shared Strength --------------------------------------------

/// Dead Hand [NEC]: "the damage from this strike cannot be prevented by
/// cards requiring Fortitude [for]"; "The opponent cannot play prevention
/// cards of that discipline (eg. for cycling)." [LSJ 20050628]. A [for]
/// card played through Shared Strength still requires Fortitude ("When
/// played, a multi-Disciplines card counts as requiring the Discipline(s)
/// being used", LSJ 20011204-3; rules-qa 38.4). Seat 1's Y lends X [for]:
/// against A's Dead Hand [nec], X's Soak is offered; against [NEC], never.
#[test]
fn dead_hand_superior_bars_a_fortitude_card_lent_by_shared_strength() {
    for superior in [false, true] {
        let (mut e, a, x) = duel(&["nec", "nec"], &[]);
        let y = MinionId(3);
        e.state.minions[y.index()].gained_levels = vec!["for"];
        hand(&mut e, 0, &["Dead Hand"]);
        hand(&mut e, 1, &["Shared Strength", "Soak"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play_on(0, 0, y, Target::Minion(x)));
        pick_when(&mut e, play(0, u8::from(superior), a));
        let soak = play(0, 0, x);
        if superior {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, soak);
            assert_eq!(blood(&e, x), 4 - 2);
        } else {
            let req = offering(&mut e, soak);
            assert_eq!(req.window, WindowLabel::CombatPrevention);
        }
    }
}

// ---- Shared Strength --------------------------------------------------------

/// Shared Strength: "Only usable by a ready vampire not involved in combat.
/// [for] Choose a vampire in combat." Any Methuselah's vampire out of the
/// combat (Bliss: "Can be played by a minion of a Methuselah not involved
/// in the current combat." ANK 20181101), either combatant chosen: seat
/// 2's W lends A [for] (A's Soak offered); seat 0's B lends the opposing
/// X [for] (X's Soak offered).
#[test]
fn shared_strength_lends_from_any_methuselah_to_either_combatant() {
    for opposing in [false, true] {
        let mut e = game(3);
        let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity);
        let a = e.add_minion(SeatId(0), textless(mid, 0), 4, false);
        let b = e.add_minion(SeatId(0), textless(mid, 1), 4, false);
        let x = e.add_minion(SeatId(1), textless(mid, 2), 4, false);
        e.add_minion(SeatId(1), textless(mid, 3), 4, false);
        let w = e.add_minion(SeatId(2), textless(mid, 4), 4, false);
        let (lender, seat, to, soak_seat) = if opposing { (b, 0, x, 1) } else { (w, 2, a, 0) };
        e.state.minions[lender.index()].gained_levels = vec!["for"];
        hand(&mut e, seat, &["Shared Strength"]);
        hand(&mut e, soak_seat, &["Soak"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play_on(0, 0, lender, Target::Minion(to)));
        let req = offering(&mut e, play(0, 0, to));
        assert_eq!(
            req.window,
            WindowLabel::CombatPrevention,
            "opposing {opposing}"
        );
        assert!(ash_has(&e, seat as usize, "Shared Strength"));
    }
}

/// Shared Strength: "For the remainder of combat": B's [for] lent to A,
/// A's Soak offered in that combat (passed); next, X bleeds and A blocks:
/// Soak never offered to A.
#[test]
fn shared_strength_ends_with_the_combat() {
    let (mut e, a, x) = duel(&[], &[]);
    let b = MinionId(1);
    e.state.minions[b.index()].gained_levels = vec!["for"];
    hand(&mut e, 0, &["Shared Strength", "Soak"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play_on(0, 0, b, Target::Minion(a)));
    let req = offering(&mut e, play(0, 0, a));
    e.choose(index_of(&req, Choice::Pass));
    to_minion_phase(&mut e, SeatId(1));
    e.state.minions[a.index()].locked = false;
    e.state.minions[x.index()].locked = false;
    pick(&mut e, bleed(x));
    pick_when(&mut e, block(a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
    assert_eq!(e.state.seats[0].hand, vec![find_library("Soak")]);
}

/// Shared Strength lends Fortitude to "play cards that require Fortitude":
/// a combo card's other Discipline stays the vampire's own; "Can be used
/// to meet one (but not more than one) of the normal Discipline
/// requirements of a multi-Discipline card" [LSJ 20011217] (Bestow Vigor's
/// group). Sympathetic Agony [chi][for]: A with Chimerstry, lent [for],
/// is offered it; A without, lent [for], never.
#[test]
fn shared_strength_meets_one_missing_requirement_of_a_combo_card() {
    for chi in [true, false] {
        let levels: &[&'static str] = if chi { &["chi"] } else { &[] };
        let (mut e, a, x) = duel(levels, &[]);
        let b = MinionId(1);
        e.state.minions[b.index()].gained_levels = vec!["for"];
        hand(&mut e, 0, &["Shared Strength", "Sympathetic Agony"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play_on(0, 0, b, Target::Minion(a)));
        let agony = play(0, 0, a);
        assert_eq!(
            offered_before(&mut e, agony, SeatId(0), WindowLabel::MinionPhase),
            chi,
            "chi {chi}"
        );
    }
}

// ---- Fear of the Void Below -------------------------------------------------

/// Fear of the Void Below [dai]: "each round defaults to long range"; a
/// round's range set by another effect wins: "Range is assumed to be at
/// default (usually close, unless e.g. [dai] Fear of the Void Below was
/// played), unless range was set by another effect" [LSJ 20100709]
/// (ledger 5019); PIB 20120214. A's Fear, X's Gleam of Red Eyes [PRO]
/// maneuver to close, A's Grasp of the Python [SER] (next round close,
/// "skip the determine range step"), its press used: round 2 close, not
/// long.
#[test]
fn fear_of_the_void_below_yields_to_a_set_range() {
    let (mut e, a, x) = duel(&["dai", "ser", "ser"], &["pro", "pro"]);
    hand(
        &mut e,
        0,
        &["Fear of the Void Below", "Grasp of the Python"],
    );
    hand(&mut e, 1, &["Gleam of Red Eyes"]);
    e.notes = Some(Vec::new());
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    request(&mut e);
    assert_eq!(range(&e), Range::Long);
    pick_when(&mut e, play(0, 1, x));
    pick_when(&mut e, play(0, 1, a));
    drive(&mut e, &[Choice::UsePress], |req, round| {
        if round == 2 {
            assert_ne!(req.window, WindowLabel::CombatDetermineRange);
        }
    });
    assert_eq!(strike_ranges(&mut e), [Range::Close, Range::Close]);
}

// ---- Grasp of the Python ----------------------------------------------------

/// Grasp of the Python [SER]: "If another effect was already setting that
/// round's range (e.g. Neutral Guard), that effect has priority." [PIB
/// 20120214]; the hand-strike-only clause is separate ("and strikes that
/// are not hand strikes cannot be used that round (by either combatant)"):
/// round 2 long by A's Neutral Guard [obe], A's Earth Swords [myt] (2R)
/// never offered in it.
#[test]
fn grasp_of_the_python_bars_non_hand_strikes_in_a_round_set_long() {
    let (mut e, a, x) = duel(&["obe", "spi", "myt"], &["ser", "ser"]);
    hand(
        &mut e,
        0,
        &["Charge of the Buffalo", "Neutral Guard", "Earth Swords"],
    );
    hand(&mut e, 1, &["Grasp of the Python"]);
    e.notes = Some(Vec::new());
    pick(&mut e, play_on(0, 1, a, Target::Minion(x)));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 1, x));
    drive(&mut e, &[Choice::UsePress], |req, round| {
        if round == 2 {
            assert!(!req.options.contains(&play(0, 1, a)), "{req:?}");
        }
    });
    assert_eq!(strike_ranges(&mut e), [Range::Close, Range::Long]);
    assert!(!ash_has(&e, 0, "Earth Swords"));
}

// ---- Ignore the Searing Flames ----------------------------------------------

/// Ignore the Searing Flames [DAI]: "Burn 1 blood to be immune to
/// aggravated damage this round", aggravated damage the vampire treats as
/// normal too: "No. Immunity means you ignore the aggravated damage." (LSJ
/// 2005-03-03, https://usenet.krcg.org/t/iRsXfkv5vvc/#m5); advanced rules
/// §4.5.5. X under Skin of Night [for] (aggravated as normal), immune: A's
/// 2 aggravated hand damage costs X nothing (only the burn); without
/// [DAI], 2 blood.
#[test]
fn ignore_the_searing_flames_superior_immunity_covers_aggravated_as_normal() {
    for immune in [true, false] {
        let (mut e, a, x) = duel(&[], &["dai", "dai", "for"]);
        let m = &mut e.state.minions[a.index()];
        m.timed
            .push((Lasting::AggravatedHands, Expiry::Combat, SeatId(0)));
        m.timed
            .push((Lasting::Strength(1), Expiry::Combat, SeatId(0)));
        hand(&mut e, 1, &["Skin of Night", "Ignore the Searing Flames"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, 0, x));
        if immune {
            pick_when(&mut e, play(0, 2, x));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let lost = if immune { 1 } else { 2 };
        assert_eq!(blood(&e, x), 4 - lost, "immune {immune}");
    }
}

// ---- Immense Size -----------------------------------------------------------

/// Immense Size [vic]: "The opposing minion cannot play grapple cards …
/// this combat": a grapple already played stands (advanced rules §2.5.10).
/// X's Grasp of the Python [SER] in round 1, its press used; A's Immense
/// Size [vic] before round 2's strikes: round 2 still close, A's Earth
/// Swords [myt] still barred.
#[test]
fn immense_size_leaves_a_grapple_in_force_standing() {
    let (mut e, a, x) = duel(&["vic", "myt"], &["ser", "ser"]);
    hand(&mut e, 0, &["Immense Size", "Earth Swords"]);
    hand(&mut e, 1, &["Grasp of the Python"]);
    e.notes = Some(Vec::new());
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    let mut sized = false;
    drive(&mut e, &[Choice::UsePress, play(0, 0, a)], |req, round| {
        if round == 2 {
            sized |= req.options.contains(&play(0, 0, a));
            assert!(!req.options.contains(&play(1, 1, a)), "{req:?}");
            assert!(!req.options.contains(&play(0, 1, a)), "{req:?}");
        }
    });
    assert!(sized, "Immense Size offered in round 2");
    assert!(ash_has(&e, 0, "Immense Size"));
    assert_eq!(strike_ranges(&mut e), [Range::Close, Range::Close]);
}

// ---- Mercy for Seth ---------------------------------------------------------

/// Mercy for Seth: "During the attached minion's unlock phase, they take 1
/// unpreventable environmental damage." No "only one": two copies on X,
/// 2 blood at its unlock phase.
#[test]
fn mercy_for_seth_copies_stack() {
    let (mut e, _, x) = duel(&[], &[]);
    in_play(&mut e, 0, "Mercy for Seth", Some(x));
    in_play(&mut e, 0, "Mercy for Seth", Some(x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2);
}

/// Mercy for Seth: "They can burn this card as a +1 stealth action": a
/// torpid bearer takes no such action (rulebook: a vampire in torpor takes
/// only the action to leave torpor). X in torpor: its unlock damage, no
/// burn offered.
#[test]
fn mercy_for_seth_is_not_burned_from_torpor() {
    let (mut e, _, _) = duel(&[], &[]);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 5), 3, true);
    let mercy = in_play(&mut e, 0, "Mercy for Seth", Some(x));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 3 - 1);
    assert!(uses(&req, mercy.0).is_empty(), "{:?}", req.options);
}

// ---- Rapid Thought ----------------------------------------------------------

/// Rapid Thought [CEL]: "only if this vampire would choose his or her
/// strike first. Instead, the opposing minion chooses his or her strike
/// first." A strike committed at the maneuver is still chosen in the
/// choose-strike step (advanced rules §4.3.1): A's .44 Magnum maneuver
/// (long range) commits its strike; A is still offered Rapid Thought
/// beside it, plays it: X chooses first (A not chosen yet), then A's
/// committed 2R lands.
#[test]
fn rapid_thought_superior_follows_a_committed_strike() {
    let (mut e, a, x) = duel(&["cel", "cel"], &["pot"]);
    let gun = in_play(&mut e, 0, ".44 Magnum", Some(a));
    hand(&mut e, 0, &["Rapid Thought"]);
    hand(&mut e, 1, &["Earth Swords"]);
    fight(&mut e, a, x);
    pick_when(&mut e, cb_weapon(gun, 1));
    request(&mut e);
    assert_eq!(range(&e), Range::Long);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    e.choose(index_of(&req, play(0, 1, a)));
    let req = expect(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(!e.combat().expect("combat").sides[0].chosen, "X first");
    e.choose(index_of(&req, Choice::HandStrike));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Rapid Thought"));
    assert_eq!((blood(&e, a), blood(&e, x)), (4, 4 - 2));
}

/// Rapid Thought [CEL] beside a card committed at the maneuver: A, an
/// Anarch, plays Lam Into [for] for its maneuver, its strike committed
/// (LSJ 20021028); A is still offered Rapid Thought beside the committed
/// strike, and plays it (advanced rules §4.3.1).
#[test]
fn rapid_thought_superior_follows_a_strike_card_committed_at_the_maneuver() {
    let (mut e, a, x) = duel(&["cel", "cel", "for"], &[]);
    e.state.minions[a.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    hand(&mut e, 0, &["Lam Into", "Rapid Thought"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(req.options.contains(&Choice::CommittedStrike), "{req:?}");
    e.choose(index_of(&req, play(0, 1, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Rapid Thought"));
    assert!(ash_has(&e, 0, "Lam Into"));
}

// ---- Silence of Death -------------------------------------------------------

/// Silence of Death: "[QUI] As above, with an optional maneuver." A's
/// [QUI] before range, its maneuver taken: the strikes at long range;
/// [qui] gives none, the strikes at close range.
#[test]
fn silence_of_death_superior_gives_an_optional_maneuver() {
    for superior in [true, false] {
        let (mut e, a, x) = duel(&["qui", "qui"], &[]);
        hand(&mut e, 0, &["Silence of Death"]);
        e.notes = Some(Vec::new());
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, u8::from(superior), a));
        let maneuvered = offered_before(
            &mut e,
            Choice::UseManeuver,
            SeatId(0),
            WindowLabel::MinionPhase,
        );
        assert_eq!(maneuvered, superior);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let want = if superior { Range::Long } else { Range::Close };
        assert_eq!(strike_ranges(&mut e), [want]);
    }
}

// ---- Skin of the Adder ------------------------------------------------------

/// Skin of the Adder [ser]: "This vampire can prevent 1 damage each
/// round": any damage, not only a strike's. X's Outside the Hourglass
/// [TEM] ("This vampire inflicts 2 damage on the opposing minion", before
/// range) on A: A prevents 1 of the 2 before choosing any strike.
#[test]
fn skin_of_the_adder_prevents_damage_that_is_no_strike() {
    let (mut e, a, x) = duel(&["ser", "myt"], &["tem", "tem"]);
    hand(&mut e, 0, &["Skin of the Adder", "Earth Swords"]);
    hand(&mut e, 1, &["Outside the Hourglass"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 3, x));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::CombatPrevention);
    e.choose(index_of(&req, Choice::UsePrevent));
    expect(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert_eq!(blood(&e, a), 4 - 1 - 1, "its cost and 1 of the 2");
}

// ==== Fix pass: action readings =============================================

/// Four seats, seat `n`'s fixture vampire (capacity 4 or more) each, 4
/// blood: [seat 0's, seat 1's, seat 2's, seat 3's].
fn ab_four_seats() -> (Engine, [MinionId; 4]) {
    let mut e = game(4);
    let mut out = [MinionId(0); 4];
    for (s, m) in out.iter_mut().enumerate() {
        *m = e.add_minion(SeatId(s as u8), textless(|c| c.capacity >= 4, s), 4, false);
    }
    (e, out)
}

// ---- Anarch Secession -------------------------------------------------------

/// Persona Non Grata: "This vampire cannot gain a non-Anarch title" — a
/// baron's is an Anarch title (`GameState::may_gain_title`). Vlad Tepes
/// under it plays Anarch Secession as a prince ([PIB 20150306]): he still
/// becomes baron, the card his title card.
#[test]
fn anarch_secession_baron_title_is_not_barred_by_persona_non_grata() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), crypt_card("Vlad Tepes"), 6, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, true);
    in_play(&mut e, 1, "Persona Non Grata", Some(v));
    hand(&mut e, 0, &["Anarch Secession"]);
    pick(&mut e, play(0, 0, v));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Anarch Secession", Some(v));
    assert_eq!(
        e.state.title_card_on(v),
        Some((card, Title::Baron(zvtes::cards::MADE_UP_CITY)))
    );
}

// ---- Concordance ------------------------------------------------------------

/// Concordance [dai] twice on one vampire (no "only one" printed): both
/// apply, +2 strength and -2 stealth (rules-digest D4).
#[test]
fn concordance_two_copies_both_apply() {
    let (mut e, [a, ..]) = four(|_| true);
    let before = e.state.bearer_always(a, Stat::Strength);
    in_play_mode(&mut e, 0, "Concordance", Some(a), 1);
    in_play_mode(&mut e, 0, "Concordance", Some(a), 1);
    assert_eq!(e.state.bearer_always(a, Stat::Strength), before + 2);
    assert_eq!(e.state.bearer_always(a, Stat::Stealth), -2);
}

// ---- Consignment to Duat ----------------------------------------------------

/// Consignment to Duat [SER] on seat 1's ally: "During the attached
/// minion's unlock phase, they burn 1 blood or life. If they cannot, they
/// go to torpor." Its 2 life: 1 at seat 1's next unlock, burned at the
/// following one (an ally always "can": its last life burned), the card
/// with it.
#[test]
fn consignment_to_duat_drains_an_ally_to_its_burn() {
    let (mut e, [a, ..]) = four(|_| true);
    e.state.minions[a.index()].gained_levels = vec!["ser", "ser"];
    let ally = e.add_ally(SeatId(1), find_library("Dark Web Procurer"));
    e.state.minions[ally.index()].blood = 2;
    hand(&mut e, 0, &["Consignment to Duat"]);
    pick(&mut e, play_at(0, 1, a, ally));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Consignment to Duat", Some(ally));
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ally), 1);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(burned(&e, ally));
    assert!(ash_has(&e, 1, "Dark Web Procurer"));
    assert!(ash_has(&e, 0, "Consignment to Duat"));
}

// ---- Covenant of Blood ------------------------------------------------------

/// Covenant of Blood: "enter combat with any Camarilla vampire controlled
/// by another Methuselah": a ready one (rules-qa 85.6: a torpid vampire is
/// no combat target). Torpid X never offered, ready Y is.
#[test]
fn covenant_of_blood_offers_no_torpid_target() {
    use zvtes::crypt::Sect;
    let (mut e, [a, b, x, y]) = four(|_| true);
    for (m, s) in [
        (a, Sect::Sabbat),
        (b, Sect::Camarilla),
        (x, Sect::Camarilla),
        (y, Sect::Camarilla),
    ] {
        e.state.minions[m.index()].sect_override = Some(s);
    }
    e.state.minions[x.index()].in_torpor = true;
    let cov = in_play(&mut e, 0, "Covenant of Blood", None);
    let attack = |t| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(cov),
        ability: 0,
        actor: Some(a),
        target: Target::Minion(t),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&attack(y)));
    assert!(!req.options.contains(&attack(x)), "torpid");
}

// ---- Distant Friend ---------------------------------------------------------

/// Distant Friend: "burn this card during a referendum to change the votes
/// and ballots of the chosen vampire": its chosen vampire burned, the burn
/// never offered (a legal no-op otherwise, D14; `ChosenInPlay`).
#[test]
fn distant_friend_is_not_burned_once_its_vampire_is_gone() {
    let (mut e, [a, _, x, y]) = four(|c| c.sect == Some(zvtes::crypt::Sect::Camarilla));
    e.state.minions[a.index()].gained_levels = vec!["dom", "pre"];
    hand(&mut e, 0, &["Distant Friend"]);
    pick(&mut e, play_at(0, 0, a, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Distant Friend", None);
    e.burn(x);
    hand(&mut e, 1, &["Kine Resources Contested"]);
    kine_polling(&mut e, y);
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        ability(card.0, 1),
    );
    assert!(e.state.cards[card.index()].in_play);
}

// ---- Fire Dance -------------------------------------------------------------

/// Fire Dance: "Is directed or not based on the controller of the target,
/// as always." [RTR 19980707] Four seats: A puts it on seat 2's Sabbat Z
/// (neither prey nor predator): seat 2 offered a block; on its own B: Z
/// never offered one.
#[test]
fn fire_dance_is_directed_at_another_s_target() {
    for own in [false, true] {
        let (mut e, [a, _, z, _]) = ab_four_seats();
        let b = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 7), 4, false);
        e.state.minions[a.index()].granted_title = Some(Title::Priscus);
        for m in [a, b, z] {
            e.state.minions[m.index()].sect_override = Some(zvtes::crypt::Sect::Sabbat);
        }
        hand(&mut e, 0, &["Fire Dance"]);
        pick(&mut e, play_at(0, 0, a, if own { b } else { z }));
        if own {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, block(z));
        } else {
            let req = offering(&mut e, block(z));
            e.choose(index_of(&req, Choice::Pass));
        }
    }
}

// ---- Heart of Darkness ------------------------------------------------------

/// Heart of Darkness [SER] twice on A (no "only one" printed): "this
/// vampire can prevent 1 damage each combat" from each. X's 2 aggravated
/// hand damage, taken as normal: both preventions offered, both used, A
/// keeps its blood.
#[test]
fn heart_of_darkness_two_copies_prevent_twice() {
    let (mut e, a, x) = duel(&[], &[]);
    let h1 = in_play_mode(&mut e, 0, "Heart of Darkness", Some(a), 1);
    let h2 = in_play_mode(&mut e, 0, "Heart of Darkness", Some(a), 1);
    for l in [Lasting::AggravatedHands, Lasting::Strength(1)] {
        e.state.minions[x.index()]
            .timed
            .push((l, Expiry::Combat, SeatId(1)));
    }
    fight(&mut e, a, x);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatPrevention);
    assert_eq!(card_uses(&req, h1).len(), 1);
    assert_eq!(card_uses(&req, h2).len(), 1);
    e.choose(index_of(&req, card_uses(&req, h1)[0]));
    let req = request(&mut e);
    let second = card_uses(&req, h2);
    assert_eq!(second.len(), 1, "{:?}", req.options);
    e.choose(index_of(&req, second[0]));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!torpid(&e, a));
    assert_eq!(blood(&e, a), 4);
}

// ---- Inquisition ------------------------------------------------------------

/// Inquisition: "Choose one or more bishops": a torpid bishop too ("bishops"
/// unqualified); at resolution a pick no longer a bishop is spared (the
/// chosen are "bishops", read then: D13). X torpid, picked: untitled, 2
/// damage; Z's title lost before resolution: untouched.
#[test]
fn inquisition_picks_a_torpid_bishop_and_spares_one_no_longer_a_bishop() {
    use zvtes::crypt::Sect;
    let sab = |c: &zvtes::crypt::CryptCard| c.sect == Some(Sect::Sabbat) && c.capacity >= 4;
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), textless(sab, 0), 4, false);
    e.add_minion(SeatId(0), textless(sab, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(sab, 2), 4, true);
    e.add_minion(SeatId(1), textless(sab, 5), 4, false);
    let z = e.add_minion(SeatId(2), textless(sab, 3), 4, false);
    e.add_minion(SeatId(3), textless(sab, 4), 4, true);
    for m in [x, z] {
        e.state.minions[m.index()].granted_title = Some(Title::Bishop);
    }
    hand(&mut e, 0, &["Inquisition"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::ChooseMinion(x));
    pick(&mut e, Choice::ChooseMinion(z));
    let req = offering(&mut e, block(z));
    e.state.minions[z.index()].granted_title = None;
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[x.index()].granted_title, None);
    assert_eq!(blood(&e, x), 4 - 2);
    assert_eq!(blood(&e, z), 4);
}

// ---- Lifeless Tongues -------------------------------------------------------

/// Lifeless Tongues: "Actions targeting an ash heap are undirected"
/// (rules-qa 17.2; Ⓓ indicative, 20.1). Four seats: a vampire of seat 2's
/// ash heap (neither prey nor predator): seat 2's Z never offered a block;
/// the card in play under seat 2's control.
#[test]
fn lifeless_tongues_is_undirected() {
    let (mut e, [a, _, z, _]) = ab_four_seats();
    e.state.minions[a.index()].gained_levels = vec!["nec"];
    let dead = textless(|c| c.capacity >= 4, 9);
    e.state.seats[2].ash_crypt.push(dead);
    hand(&mut e, 0, &["Lifeless Tongues"]);
    pick(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, block(z));
    let card = card_in_play(&e, "Lifeless Tongues").expect("in play");
    assert_eq!(e.state.card_controller(card), SeatId(2));
}

// ---- The Name Forgotten -----------------------------------------------------

/// The Name Forgotten on a torpid vampire X diablerized by A: its burn is
/// replaced (removed instead), so the diablerie is unsuccessful (rules-qa
/// 121.20: "the diablerist gets nothing, no blood hunt"): X removed, A's
/// blood unchanged, no diablerie recorded, A still in play.
#[test]
fn the_name_forgotten_makes_a_diablerie_unsuccessful() {
    let (mut e, [a, _, x, _]) = four(|c| c.group != 0);
    e.state.minions[x.index()].in_torpor = true;
    let c = e.state.minions[x.index()].crypt_card().expect("a vampire");
    in_play(&mut e, 1, "Name Forgotten, The", Some(x));
    let before = blood(&e, a);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: zvtes::decision::ActionKind::Diablerie,
            target: Some(x),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[x.index()].in_play);
    assert!(e.state.seats[1].removed_crypt.contains(&c));
    assert!(!e.state.seats[1].ash_crypt.contains(&c));
    assert_eq!(blood(&e, a), before, "the diablerist gets nothing");
    assert!(e.state.minions[a.index()].diableries.0.is_empty());
    assert!(e.state.minions[a.index()].in_play, "no blood hunt");
}

// ---- Revelation of the Sire -------------------------------------------------

/// Revelation of the Sire: "a member of the same clan as the acting
/// vampire": the clan fixed as it resolves (`CardInPlay::clan_change`).
/// The actor's own clan later changed, then the actor burned: X stays
/// Brujah.
#[test]
fn revelation_of_the_sire_clan_is_fixed_at_resolution() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Brujah" && c.capacity >= 7, 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(
        SeatId(1),
        textless(|c| c.clan == "Caitiff" && c.capacity <= 5, 0),
        2,
        false,
    );
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    hand(&mut e, 0, &["Revelation of the Sire"]);
    pick(&mut e, play_at(0, 0, a, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    placed_on(&e, "Revelation of the Sire", Some(x));
    e.state.minions[a.index()].kind =
        zvtes::state::MinionKind::Vampire(textless(|c| c.clan == "Toreador", 0));
    assert_eq!(e.state.minions[a.index()].clan(), Some("Toreador"));
    assert_eq!(e.state.minions[x.index()].clan(), Some("Brujah"));
    e.burn(a);
    assert_eq!(e.state.minions[x.index()].clan(), Some("Brujah"));
}

// ---- Shepherd's Innocence ---------------------------------------------------

/// Shepherd's Innocence [ani] at seat 1's Underbridge Stray, an ally
/// requiring Animalism: "Take control of a card in play that requires
/// Animalism": the ally changes hands with its card (PIB 20121028).
#[test]
fn shepherds_innocence_takes_an_ally_requiring_animalism() {
    let (mut e, [a, ..]) = four(|_| true);
    e.state.minions[a.index()].gained_levels = vec!["ani"];
    let stray = e.add_ally(SeatId(1), find_library("Underbridge Stray"));
    let card = e.state.minions[stray.index()].ally_id().expect("its card");
    hand(&mut e, 0, &["Shepherd's Innocence"]);
    pick(&mut e, play_on(0, 2, a, Target::Card(card)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions_of(SeatId(0)).any(|m| m == stray));
    assert_eq!(e.state.card_controller(card), SeatId(0));
}

/// Shepherd's Innocence [ani] at Tier of Souls [ANI] on seat 1's X (neither
/// equipment nor retainer): the card's controller changes, its control in
/// play still its host's (`GameState::card_controller`; amber, a gap).
#[test]
fn shepherds_innocence_on_a_card_on_another_s_vampire_changes_its_controller_only() {
    let (mut e, [a, _, x, _]) = four(|_| true);
    e.state.minions[a.index()].gained_levels = vec!["ani"];
    let tier = in_play_mode(&mut e, 1, "Tier of Souls", Some(x), 1);
    hand(&mut e, 0, &["Shepherd's Innocence"]);
    pick(&mut e, play_on(0, 2, a, Target::Card(tier)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[tier.index()].host, Some(x));
    assert_eq!(e.state.cards[tier.index()].controller, SeatId(0));
    assert_eq!(e.state.card_controller(tier), SeatId(1));
}

// ---- Tumnimos ---------------------------------------------------------------

/// Tumnimos: "You can put a master Discipline card from your hand on this
/// new vampire": one Celerity of two, chosen, on it (played by Tumnimos,
/// not in the normal fashion: no cost); the other kept.
#[test]
fn tumnimos_puts_a_master_discipline_from_hand_on_the_new_vampire() {
    let (mut e, [a, ..]) = four(|c| c.clan == "Ravnos" && c.capacity >= 5);
    hand(&mut e, 0, &["Tumnimos", "Celerity", "Celerity"]);
    let cel = find_library("Celerity");
    pick(&mut e, play(0, 0, a));
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Hand,
            card: cel,
        },
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let new = MinionId(e.state.minions.len() as u32 - 1);
    placed_on(&e, "Celerity", Some(new));
    assert!(e.state.has_discipline(new, "cel"));
    assert_eq!(
        e.state.seats[0].hand.iter().filter(|&&c| c == cel).count(),
        1
    );
    assert_eq!(pool(&e)[0], 10);
}

// ---- Undue Influence --------------------------------------------------------

/// Undue Influence [chi]: "Move 1 blood from a crypt card in any
/// uncontrolled region to the chosen vampire": an empty source a legal
/// no-op (D14): played, nothing moves.
#[test]
fn undue_influence_chimerstry_from_an_empty_vampire_moves_nothing() {
    let (mut e, [a, b, ..]) = four(|_| true);
    for m in [a, b] {
        e.state.minions[m.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    }
    e.state.minions[a.index()].gained_levels = vec!["chi"];
    for (seat, n, counters) in [(0, 7, 1), (1, 8, 0)] {
        e.state.seats[seat]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card: textless(|c| c.capacity >= 6, n),
                copy: zvtes::copies::Untracked(zvtes::copies::CopyId::NONE),
                counters,
                banished: None,
            });
    }
    hand(&mut e, 0, &["Undue Influence"]);
    pick(&mut e, play_on(0, 0, a, Target::Uncontrolled(0)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Undue Influence"));
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 1);
    assert_eq!(e.state.seats[1].uncontrolled[0].counters, 0);
}

// ---- Bujo -------------------------------------------------------------------

/// Bujo: "this Ravnos must meet the requirements of that political
/// action", its terms too: "Cannot be used or played if the conditions for
/// the terms of the referendum cannot be met (e.g. no legal selection …)"
/// [LSJ 20100129] [ANK 20191228] (read as the Ravnos could call it from
/// hand: `others_ash_referendums`). Seat 1's Permanent Vacation ("Choose a
/// ready ally") with no ally in play: Bujo not offered; an ally: offered.
#[test]
fn bujo_is_not_offered_for_a_referendum_with_no_legal_selection() {
    for ally in [false, true] {
        let (mut e, a, _) = bujo_table(&["Permanent Vacation"], &[]);
        if ally {
            e.add_ally(SeatId(1), find_library("Dark Web Procurer"));
        }
        let req = request(&mut e);
        assert_eq!(req.options.contains(&play(0, 0, a)), ally, "ally {ally}");
    }
}

/// Bujo on a political card of several modes: the first mode the Ravnos
/// could play calling a referendum (amber, no ruling). Seat 1's Lily
/// Prelude, A with superior Melpominee: its [mel] referendum, the points
/// allocated, no Methuselah chosen ([MEL]'s term never asked).
#[test]
fn bujo_calls_a_multi_mode_card_s_first_referendum() {
    let (mut e, a, _) = bujo_table(&["Lily Prelude"], &[]);
    e.state.minions[a.index()].gained_levels = vec!["mel", "mel"];
    pick(&mut e, play(0, 0, a));
    let mut allocated = false;
    loop {
        let req = request(&mut e);
        if req.window == WindowLabel::ReferendumPolling {
            break;
        }
        assert!(
            !req.options
                .iter()
                .any(|o| matches!(o, Choice::ChooseSeat(_))),
            "{req:?}"
        );
        allocated |= req
            .options
            .iter()
            .any(|o| matches!(o, Choice::AllocateTo { .. }));
        e.choose(fallback(&req));
    }
    assert!(allocated);
    assert!(e.state.seats[1]
        .removed
        .contains(&find_library("Lily Prelude")));
}

// ==== Fix pass: vampire readings ============================================

/// Seat 0's `diablerist` diablerizes seat 1's torpid `victim`.
fn vb_diablerize(e: &mut Engine, diablerist: MinionId, victim: MinionId) {
    pick(
        e,
        Choice::TakeAction {
            minion: diablerist,
            action: ActionKind::Diablerie,
            target: Some(victim),
        },
    );
}

/// The combat rounds noted (one strike pair each).
fn vb_rounds(e: &mut Engine) -> usize {
    noted_combats(e)
        .into_iter()
        .filter(|n| matches!(n, zvtes::combat::CombatNote::Strikes { .. }))
        .count()
}

// ==== vamp_a ==================================================================

// ---- Alexis Sorokin ---------------------------------------------------------

/// Alexis Sorokin: "Once each combat, Alexis can strike: dodge." Round 1:
/// her dodge against X's Grasp of the Python [ser], whose press X uses to
/// continue; round 2 (no determine range): her dodge never offered.
#[test]
fn alexis_sorokin_dodges_only_once_each_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), crypt_card("Alexis Sorokin"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["ser"];
    hand(&mut e, 1, &["Grasp of the Python"]);
    e.notes = Some(Vec::new());
    fight(&mut e, a, x);
    pick_use(&mut e, a, 0);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, Choice::UsePress);
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        if e.combat().is_some_and(|c| c.round == 2) {
            assert!(
                !req.options.iter().any(|o| uses_printed(o, a, 0)),
                "{req:?}"
            );
        }
        e.choose(fallback(&req));
    }
    assert_eq!(vb_rounds(&mut e), 2);
}

/// Alexis Sorokin: "She can steal an equipment as a Ⓓ action": an action
/// of her own, not an equip action (no `IsEquipAction` reader sees it).
#[test]
fn alexis_sorokin_steal_is_not_an_equip_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), crypt_card("Alexis Sorokin"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    let theirs = in_play(&mut e, 1, ".44 Magnum", Some(x));
    pick(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Vampire(a),
            ability: 1,
            actor: Some(a),
            target: Target::Card(theirs),
        },
    );
    request(&mut e);
    let act = e.action().expect("her action");
    assert_eq!(act.kind, ActionKind::Ability);
    assert!(!act.is_equip_action(&e.state));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[theirs.index()].host, Some(a));
}

// ---- Allonzo Montoya --------------------------------------------------------

/// Allonzo Montoya: "Allonzo gets +1 stealth on actions that require
/// Serpentis [ser]": his employ of Hatch the Viper ([pro][ser]) at 1 more
/// stealth than the same employ by another vampire.
#[test]
fn allonzo_montoya_sneaks_a_serpentis_employ() {
    let mut base = None;
    for allonzo in [false, true] {
        let mut e = game(2);
        let card = if allonzo {
            crypt_card("Allonzo Montoya")
        } else {
            textless(|c| c.capacity >= 4, 0)
        };
        let a = e.add_minion(SeatId(0), card, 4, false);
        e.state.minions[a.index()].gained_levels = vec!["pro", "ser"];
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
        hand(&mut e, 0, &["Hatch the Viper"]);
        pick(&mut e, play(0, 0, a));
        request(&mut e);
        let act = e.action().expect("the employ");
        assert_eq!(act.kind, ActionKind::Employ);
        let stealth = act.stealth(&e.state);
        match base {
            None => base = Some(stealth),
            Some(b) => assert_eq!(stealth, b + 1),
        }
    }
}

// ---- Amaravati --------------------------------------------------------------

/// Amaravati: "During the polling step of a political action": never in a
/// blood hunt's polling (adv §3.7.8.2: only "political action" excludes
/// blood hunts). Seat 0's diablerie, seat 1's prince Tara a voter: the
/// blood hunt polled, Amaravati never offered.
#[test]
fn amaravati_is_not_used_in_a_blood_hunt() {
    let mut e = game(2);
    let am = e.add_minion(SeatId(0), crypt_card("Amaravati"), 3, false);
    let d = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), crypt_card("Tara"), 4, false);
    let v = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 2, true);
    vb_diablerize(&mut e, d, v);
    let mut polled = false;
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
            break;
        }
        polled |= req.window == WindowLabel::BloodHuntPolling;
        assert!(
            !req.options.iter().any(|o| uses_printed(o, am, 0)),
            "{req:?}"
        );
        e.choose(fallback(&req));
    }
    assert!(polled, "a blood hunt polled");
}

// ---- Ambrogino Giovanni -----------------------------------------------------

/// Ambrogino Giovanni (advanced): "remove seven cards in your ash heap from
/// the game to gain 2 blood": at 8 blood of 9, he reaches his capacity, no
/// more.
#[test]
fn ambrogino_giovanni_gains_up_to_his_capacity() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), adv_card("Ambrogino Giovanni"), 8, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.state.seats[0].ash = vec![find_library("Roundhouse"); 8].into();
    let used = next_turn_taking(&mut e, |o| uses_printed(o, a, 0));
    assert_eq!(used, 1);
    assert_eq!(e.state.seats[0].removed.len(), 7);
    assert_eq!(e.state.capacity_of(a), Some(9));
    assert_eq!(blood(&e, a), 9);
}

// ---- Astrid Thomas ----------------------------------------------------------

/// Astrid Thomas: "When results are tallied during a referendum": a blood
/// hunt too (adv §3.7.8.2). Seat 0's diablerist D; Astrid in favor, seat
/// 1's primogen Tremere against: drawn to her side, 2-0, D burned (1-1
/// would fail).
#[test]
fn astrid_thomas_draws_tremere_in_a_blood_hunt() {
    let mut e = game(2);
    let astrid = e.add_minion(SeatId(0), crypt_card("Astrid Thomas"), 4, false);
    let d = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let t = e.add_minion(SeatId(1), of_clan("Tremere", 3..=11, 0), 4, false);
    e.state.minions[t.index()].granted_title = Some(Title::Primogen);
    let v = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 2, true);
    vb_diablerize(&mut e, d, v);
    pick_when(&mut e, votes(astrid, true));
    pick_when(&mut e, votes(t, false));
    assert_eq!(tally_now(&e), (1, 1));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[d.index()].in_play, "the blood hunt passed");
}

/// Astrid Thomas: "any Tremere … cast votes and ballots with Astrid": a
/// Tremere priscus's ballot (0 votes, the prisci block's 3) cast against
/// moves to her side with its record: Political Stranglehold passes 4-0
/// (1-3 as cast).
#[test]
fn astrid_thomas_draws_a_tremere_priscus_ballot() {
    let mut e = game(2);
    let astrid = e.add_minion(SeatId(0), crypt_card("Astrid Thomas"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 8, 0), 4, false);
    let t = e.add_minion(SeatId(1), of_clan("Tremere", 3..=11, 0), 4, false);
    e.state.minions[t.index()].sect_override = Some(zvtes::crypt::Sect::Sabbat);
    e.state.minions[t.index()].granted_title = Some(Title::Priscus);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    hand(&mut e, 0, &["Political Stranglehold"]);
    pick(&mut e, play(0, 0, astrid));
    to_referendum(&mut e);
    pick_when(&mut e, votes(astrid, true));
    pick_when(&mut e, votes(t, false));
    assert_eq!(tally_now(&e), (1, 3), "cast as cast until the tally");
    let before = e.state.seats[0].pool;
    close_polling(&mut e);
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, before + 3, "passed");
}

// ---- Basir ------------------------------------------------------------------

/// Basir: "Basir gains 1 blood after you use a master phase action to put a
/// master Discipline card on him." His controller's Presence canceled by
/// seat 1's Sudden Reversal: never put on him, no blood.
#[test]
fn basir_gains_nothing_from_a_canceled_master_discipline_card() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), crypt_card("Basir"), 1, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    hand(&mut e, 0, &["Presence"]);
    hand(&mut e, 1, &["Sudden Reversal"]);
    to_window(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, master_on(b));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    let cancel = req
        .options
        .iter()
        .position(|o| matches!(o, Choice::Play { card: 0, .. }))
        .expect("Sudden Reversal");
    e.choose(cancel);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Presence"));
    assert_eq!((e.state.capacity_of(b), blood(&e, b)), (Some(1), 1));
}

/// Basir: "Basir gains 1 blood after you use a master phase action to put a
/// master Discipline card on him": after the card is on him, within the
/// capacity it gives (at 1 blood of capacity 1: capacity 2, 2 blood). A
/// trifle (Agent of Power) still spends the master phase action it is
/// played with (rulebook "Trifle"); in torpor too (D25).
#[test]
fn basir_gains_within_the_capacity_the_card_gives() {
    for (torpor, card) in [(false, "Agent of Power"), (true, "Presence")] {
        let mut e = game(2);
        let b = e.add_minion(SeatId(0), crypt_card("Basir"), 1, torpor);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        hand(&mut e, 0, &[card]);
        to_window(&mut e, SeatId(0), WindowLabel::MasterPhase);
        pick(&mut e, master_on(b));
        if card == "Agent of Power" {
            pick_when(&mut e, Choice::ChooseDiscipline("dom"));
        }
        request(&mut e);
        assert_eq!(torpid(&e, b), torpor);
        assert_eq!(
            (e.state.capacity_of(b), blood(&e, b)),
            (Some(2), 2),
            "{card}"
        );
    }
}

// ---- Benedict Giovanni, Agent ----------------------------------------------

/// Benedict Giovanni, Agent: "During your unlock phase, any Giovanni older
/// than the oldest ready Giovanni you control can burn 1 blood to steal
/// Benedict." Benedict in torpor, the stealer (Martino della Passaglia,
/// capacity 7) in torpor too: stolen, Martino 1 blood burned, Benedict
/// still in torpor.
#[test]
fn benedict_giovanni_is_stolen_in_torpor_by_a_torpid_giovanni() {
    let mut e = game(2);
    let ben = e.add_minion(SeatId(0), crypt_card("Benedict Giovanni, Agent"), 2, true);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let seven = e.add_minion(SeatId(1), crypt_card("Martino della Passaglia"), 4, true);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let used = next_turn_taking(&mut e, |o| {
        matches!(o, Choice::UseAbility { actor: Some(a), .. } if *a == seven)
            && uses_printed(o, ben, 0)
    });
    assert_eq!(used, 1);
    assert_eq!(e.state.minions[ben.index()].owner, SeatId(1));
    assert!(torpid(&e, ben));
    assert_eq!(blood(&e, seven), 3);
}

// ==== vamp_b ==================================================================

// ---- Cagliostro, The Grand Copht --------------------------------------------

/// Cagliostro: "If he is blocked and combat occurs, he can put 1 corruption
/// counter on the blocking minion after that combat ends." Burned in the
/// combat (seat 1's 1-life ally): nothing to target, never offered. Sent to
/// torpor by the combat (Sebastian Marley's 2): offered, from torpor (D25).
/// The combat ended at once by X's Majesty ("Strike: combat ends"): still a
/// combat, offered.
#[test]
fn cagliostro_corrupts_after_any_combat_from_the_block() {
    for case in ["burned", "torpor", "majesty"] {
        let mut e = game(2);
        let blood0 = if case == "torpor" { 1 } else { 4 };
        let c = e.add_minion(
            SeatId(0),
            crypt_card("Cagliostro, The Grand Copht"),
            blood0,
            false,
        );
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        let x = match case {
            "burned" => e.add_ally(SeatId(1), find_library("Dark Web Procurer")),
            "torpor" => e.add_minion(SeatId(1), crypt_card("Sebastian Marley"), 4, false),
            _ => e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false),
        };
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
        if case == "majesty" {
            e.state.minions[x.index()].gained_levels = vec!["pre"];
            hand(&mut e, 1, &["Majesty"]);
        }
        let at = Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Vampire(c),
            ability: 0,
            actor: None,
            target: Target::Minion(x),
        };
        fight(&mut e, c, x);
        if case == "majesty" {
            pick_when(&mut e, play(0, 0, x));
        }
        if case == "burned" {
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
                uses_printed(o, c, 0)
            });
            assert!(!e.state.minions[x.index()].in_play, "burned in the combat");
            continue;
        }
        let req = offering(&mut e, at);
        assert!(e.combat().is_none(), "after the combat");
        assert_eq!(torpid(&e, c), case == "torpor", "{case}");
        e.choose(index_of(&req, at));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.minions[x.index()].corruption_of(SeatId(0)),
            1,
            "{case}"
        );
    }
}

// ---- Carmine Giovanni -------------------------------------------------------

/// Carmine Giovanni: "Wraiths come into play with 1 additional life":
/// Khazar's Diary's represented wraith ally ("a wraith ally with 2 life")
/// enters with 3, seat 1's Carmine in torpor.
#[test]
fn carmine_gives_khazars_diary_wraith_an_extra_life() {
    for carmine in [true, false] {
        let mut e = game(3);
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
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
        if carmine {
            e.add_minion(SeatId(1), crypt_card("Carmine Giovanni"), 4, true);
        }
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
        e.add_minion(SeatId(2), textless(|c| c.capacity >= 4, 3), 4, false);
        let dead = textless(|c| c.capacity >= 4, 4);
        e.state.seats[2].ash_crypt = vec![dead].into();
        let d = in_play(&mut e, 0, "Khazar's Diary (Endless Night)", None);
        e.state.cards[d.index()].counters = 7;
        pick(&mut e, granted(d.0, g));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let w = e
            .state
            .minions
            .iter()
            .position(|m| {
                m.in_play
                    && matches!(m.kind, zvtes::state::MinionKind::Represented { card, .. }
                        if card == dead)
            })
            .map(|i| MinionId(i as u32))
            .expect("the wraith");
        assert_eq!(blood(&e, w), 2 + u8::from(carmine), "Carmine {carmine}");
    }
}

/// Carmine Giovanni: "Wraiths come into play with 1 additional life": a
/// Shell Break that "becomes a wraith retainer with 2 life" was already in
/// play, it does not come into play: 2 life with Carmine in play (red,
/// with the owner: recommended yes, as built).
#[test]
fn carmine_spares_a_shell_break_becoming_a_wraith() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), of_clan("Giovanni", 4..=11, 0), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(0), crypt_card("Carmine Giovanni"), 4, true);
    let victim = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    hand(&mut e, 0, &["Shell Break"]);
    pick(&mut e, play_on(0, 0, g, Target::Minion(victim)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let shell = find_library("Shell Break");
    let id = e
        .state
        .cards
        .iter()
        .position(|c| c.card == shell && c.in_play)
        .map(|i| CardId(i as u32))
        .expect("on the victim");
    e.burn(victim);
    assert_eq!(e.state.cards[id.index()].host, Some(g));
    assert_eq!(e.state.cards[id.index()].counters, 2);
}

/// Carmine Giovanni: "Carmine can lock to give any ally +1 stealth": any
/// Methuselah's (D11). Three seats: seat 0's Screamer bleeds, seat 1's
/// vampire blocks; seat 2's Carmine locks for it, the block fails.
#[test]
fn carmine_sneaks_another_methuselahs_acting_ally() {
    let mut e = game(3);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let s = e.add_ally(SeatId(0), find_library("Screamer"));
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let y = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    let carmine = e.add_minion(SeatId(2), crypt_card("Carmine Giovanni"), 4, false);
    e.add_minion(SeatId(2), textless(|c| c.capacity >= 4, 3), 4, false);
    pick(&mut e, bleed(s));
    pick_when(&mut e, block(y));
    let req = loop {
        let req = request(&mut e);
        if req.options.iter().any(|o| uses_printed(o, carmine, 0)) {
            break req;
        }
        e.choose(fallback(&req));
    };
    assert_eq!(req.seat, SeatId(2));
    pick_use(&mut e, carmine, 0);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[carmine.index()].locked);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "the Screamer's bleed");
}

// ==== vamp_g ==================================================================

// ---- Roxanne, Rectrix of the 13th Floor --------------------------------------

/// Roxanne: "Roxanne may enter combat with a vampire of capacity below 4
/// controlled by your predator or prey as a +1 stealth Ⓓ action." A torpid
/// capacity-3 vampire is not offered (a ready one is); the capacity read
/// again at resolution: the target's capacity 4 by then, the action
/// fizzles, no combat.
#[test]
fn roxanne_targets_a_ready_small_vampire_still_small_at_resolution() {
    let mut e = game(2);
    let r = e.add_minion(
        SeatId(0),
        crypt_card("Roxanne, Rectrix of the 13th Floor"),
        4,
        false,
    );
    let small = |n| textless(|c| c.capacity == 3, n);
    let t = e.add_minion(SeatId(1), small(0), 3, false);
    let asleep = e.add_minion(SeatId(1), small(1), 3, true);
    let req = request(&mut e);
    assert!(
        !req.options.iter().any(|o| uses_at(o, r, 0, asleep)),
        "torpid"
    );
    let at = *req
        .options
        .iter()
        .find(|o| uses_at(o, r, 0, t))
        .expect("a ready one");
    e.choose(index_of(&req, at));
    e.state.minions[t.index()].kind =
        zvtes::state::MinionKind::Vampire(textless(|c| c.capacity == 4, 0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, r), blood(&e, t)), (4, 3), "no combat");
}

// ---- Sabine Lafitte -----------------------------------------------------------

/// Sabine Lafitte: "move 1 blood from Sabine to any controlled Tremere": to
/// a Tremere at capacity, the excess drains (rules-qa 41.8): Sabine 1 less,
/// the Tremere still at its capacity.
#[test]
fn sabine_lafitte_blood_above_capacity_drains() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), crypt_card("Sabine Lafitte"), 3, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let t = of_clan("Tremere", 4..=11, 1);
    let cap = zvtes::crypt::CRYPT[t as usize].capacity;
    let theirs = e.add_minion(SeatId(1), t, cap, false);
    let used = next_turn_taking(&mut e, |o| uses_at(o, s, 0, theirs));
    assert_eq!(used, 1);
    assert_eq!((blood(&e, s), blood(&e, theirs)), (2, cap));
}

// ---- Sonja Blue ---------------------------------------------------------------

/// Sonja Blue: "Sonja may remove a vampire's title as a Ⓓ action": her own
/// Methuselah's titled vampire too, undirected (rules-digest 20.1: Ⓓ only
/// indicative); a title card's title: the card (Praxis Seizure: Chicago)
/// burned, the prince untitled.
#[test]
fn sonja_blue_removes_her_own_methuselahs_title_card() {
    use zvtes::crypt::Sect;
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), sonja(), 4, false);
    let p = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Camarilla) && c.capacity >= 4, 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    in_play(&mut e, 0, "Praxis Seizure: Chicago", Some(p));
    assert_eq!(e.state.title_of(p), Title::Prince("Chicago"));
    let req = request(&mut e);
    let at = *req
        .options
        .iter()
        .find(|o| uses_at(o, s, 0, p))
        .expect("her own Methuselah's");
    e.choose(index_of(&req, at));
    request(&mut e);
    let act = e.action().expect("her action");
    assert_eq!(act.directed_seat(&e.state), None);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.title_of(p), Title::None);
    assert!(ash_has(&e, 0, "Praxis Seizure: Chicago"));
}

/// Sonja Blue: "your predator may burn 1 pool to move Sonja to his or her
/// ready region"; The Diamond Thunderbolt: "Control of that vampire does
/// not change, and that vampire gains 1 blood"; "The cost of the change of
/// control effect is still paid" [LSJ 20040518]. Seat 0 plays it: Sonja
/// stays seat 0's, in torpor, 1 blood more; seat 2's pool still burned.
#[test]
fn sonja_blue_kept_by_the_diamond_thunderbolt() {
    let mut e = game(3);
    let s = e.add_minion(SeatId(0), sonja(), 2, true);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(2), textless(|c| c.capacity >= 4, 2), 4, false);
    hand(&mut e, 0, &["Diamond Thunderbolt, The"]);
    let take = loop {
        let req = request(&mut e);
        if let Some(i) = req.options.iter().position(|o| uses_printed(o, s, 1)) {
            break i;
        }
        e.choose(fallback(&req));
    };
    e.choose(take);
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(0));
    let thunder = req
        .options
        .iter()
        .position(|o| matches!(o, Choice::Play { card: 0, .. }))
        .expect("The Diamond Thunderbolt");
    e.choose(thunder);
    request(&mut e);
    let v = &e.state.minions[s.index()];
    assert_eq!((v.owner, v.in_torpor, v.blood), (SeatId(0), true, 3));
    assert_eq!(e.state.seats[2].pool, 9);
}

/// Sonja Blue: "If Sonja leaves play, remove her from the game." Banished
/// (Banishment: "moved to the uncontrolled region"): removed, not in the
/// uncontrolled region.
#[test]
fn sonja_blue_banished_is_removed_from_the_game() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 9, 0), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    let s = e.add_minion(SeatId(1), sonja(), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    e.state.minions[a.index()].granted_title = Some(Title::Votes(3));
    hand(&mut e, 0, &["Banishment"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::ChooseMinion(s));
    to_referendum(&mut e);
    pick_when(&mut e, votes(a, true));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[s.index()].in_play);
    assert!(e.state.seats[1].removed_crypt.contains(&sonja()));
    assert!(e.state.seats[1]
        .uncontrolled
        .iter()
        .all(|u| u.card != sonja()));
}

/// Sonja Blue: put out of play by her own Descent into Darkness [obt]:
/// removed from the game, the Descent burned with her (advanced rules
/// §1.10.1).
#[test]
fn sonja_blue_descending_into_darkness_is_removed() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), sonja(), 4, false);
    e.state.minions[s.index()].gained_levels = vec!["obt"];
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    hand(&mut e, 0, &["Descent into Darkness"]);
    pick(&mut e, play_on(0, 0, s, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[s.index()].in_play);
    assert!(e.state.seats[0].removed_crypt.contains(&sonja()));
    assert!(ash_has(&e, 0, "Descent into Darkness"));
}

/// Sonja Blue in VEKN's two-player variant (`Format::TwoPlayer`): a
/// contested crypt card stays in play (rules-digest §18.5, rules-qa 91.2),
/// so she does not leave: seat 0 influences a copy of seat 1's Sonja; both
/// in play, none removed.
#[test]
fn sonja_blue_contested_in_two_players_stays() {
    let cap = zvtes::crypt::CRYPT[sonja() as usize].capacity;
    let mut e = game(2);
    e.state.format = zvtes::state::Format::TwoPlayer;
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let theirs = e.add_minion(SeatId(1), sonja(), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    uncontrolled(&mut e, 0, sonja(), cap);
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    e.choose(index_of(&req, Choice::InfluenceOut { slot: 0 }));
    request(&mut e);
    assert!(e.state.minions[theirs.index()].in_play);
    assert!(!e.state.seats[1].removed_crypt.contains(&sonja()));
    assert!(e.state.minions[last_of(&e, sonja()).index()].in_play);
}

// ---- Sylvester Simms ----------------------------------------------------------

/// Sylvester Simms: "If Sylvester goes into torpor, each Methuselah burns 1
/// pool": by a card too. His bleed with Day Operation [for] ("The acting
/// vampire goes to torpor after resolving the action"): seat 1 bled 1 and
/// burns 1, seats 0 and 2 burn 1.
#[test]
fn sylvester_simms_sent_to_torpor_by_a_card_burns_every_pool() {
    let mut e = game(3);
    let s = e.add_minion(SeatId(0), crypt_card("Sylvester Simms"), 4, false);
    e.state.minions[s.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(2), textless(|c| c.capacity >= 4, 2), 4, false);
    hand(&mut e, 0, &["Day Operation"]);
    pick(&mut e, bleed(s));
    pick_when(&mut e, play(0, 0, s));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(torpid(&e, s));
    assert_eq!(pool(&e), [9, 8, 9]);
}

/// Sylvester Simms: a torpor averted is no torpor: X's Garrote strike would
/// send him to torpor, X burns the Garrote to burn him instead ("This is
/// not considered diablerie"): no pool burned.
#[test]
fn sylvester_simms_burned_instead_of_torpor_burns_no_pool() {
    let mut e = game(3);
    let s = e.add_minion(SeatId(0), crypt_card("Sylvester Simms"), 1, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), crypt_card("Sebastian Marley"), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(2), textless(|c| c.capacity >= 4, 2), 4, false);
    let garrote = in_play(&mut e, 1, "Garrote", Some(x));
    fight(&mut e, s, x);
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        let i = req
            .options
            .iter()
            .position(|o| {
                matches!(o, Choice::UseAbility {
                    source: zvtes::cards::AbilitySource::Card(c), ..
                } if *c == garrote)
            })
            .unwrap_or_else(|| fallback(&req));
        e.choose(i);
    }
    assert!(!e.state.minions[s.index()].in_play, "burned");
    assert_eq!(pool(&e), [10, 10, 10]);
}
