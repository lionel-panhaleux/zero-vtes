//! Slice 8b: weapons in combat — a weapon's strike chosen by its bearer,
//! guns' maneuvers committing the initial strike (rulebook "Determine
//! Range", "Choose Strike"), melee weapons (not hand strikes), AK-47's
//! additional strike, Sniper Rifle's range set. Windows with a single option
//! are elided, so the scripts only stop where a real decision exists.

use zvtes::cards::AbilitySource;
use zvtes::cards::Target;
use zvtes::crypt::CRYPT;
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

/// Pass through every decision until `seat`'s `window`.
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

/// A weapon already on `host` (no play; weapons carry no layers).
fn equip(e: &mut Engine, name: &str, host: MinionId) -> u32 {
    let owner = e.state.minions[host.index()].owner;
    e.state.cards.push(CardInPlay {
        card: find_library(name),
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
        against: Default::default(),
        entered: 0,
        chosen: Vec::new(),
    });
    CardId(e.state.cards.len() as u32 - 1).0
}

fn bleed(minion: MinionId) -> Choice {
    Choice::TakeAction {
        minion,
        action: ActionKind::Bleed,
        target: None,
    }
}

fn play(card: u8, mode: u8, actor: MinionId) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target: Target::None,
    }
}

/// A weapon's ability: its strike is `0`, its maneuver or range set `1`.
fn ability(card: u32, ability: u8) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(CardId(card)),
        ability,
        actor: None,
        target: Target::None,
    }
}

fn take_from(minion: MinionId, giver: MinionId) -> Choice {
    Choice::TakeAction {
        minion,
        action: ActionKind::EquipFrom,
        target: Some(giver),
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

fn blood(e: &Engine, m: MinionId) -> u8 {
    e.state.minions[m.index()].blood
}

/// 2 seats at seat 0's minion phase, turn 4. Seat 0: A (POT), B (CEL);
/// seat 1: X (FOR), Y (PRO). 4 blood each.
struct Table {
    e: Engine,
    a: MinionId,
    b: MinionId,
    x: MinionId,
}

fn table() -> Table {
    let mut e = Engine::bare(2);
    let a = e.add_minion(SeatId(0), vampire_with("POT", 0), 4, false);
    let b = e.add_minion(SeatId(0), find_card("Farah Sarroub"), 4, false);
    let x = e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
    e.add_minion(SeatId(1), find_card("Crystal Lynn"), 4, false);
    e.state.turn = 4;
    Table { e, a, b, x }
}

/// A bleeds, X blocks.
fn blocked_bleed(t: &mut Table) {
    pick(&mut t.e, bleed(t.a));
    pick(&mut t.e, Choice::DeclareBlock { minion: t.x });
}

// Rulebook "Determine Range": using the maneuver from a weapon chooses the
// strike — X's strike is the .44 Magnum's (its choice elides, the
// Flamethrower and the hand strike are not offered), 2R at long range.
#[test]
fn a_gun_maneuver_commits_the_initial_strike_to_that_gun() {
    let mut t = table();
    let magnum = equip(&mut t.e, ".44 Magnum", t.x);
    equip(&mut t.e, "Flamethrower", t.x);
    blocked_bleed(&mut t);
    let e = &mut t.e;
    let req = expect(e, SeatId(1), WindowLabel::CombatDetermineRange);
    assert_eq!(req.options, vec![Choice::Pass, ability(magnum, 1)]);
    pick(e, ability(magnum, 1));
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(e, t.a), blood(e, t.x)), (4 - 2, 4));
}

// Spec §4.3.1: a minion committed to a strike an effect then bars gets no
// strike at all. Both combatants maneuver with a .44 Magnum (back to close
// range), then Immortal Grapple [pot] bars non-hand strikes: neither
// strikes, and neither falls back to a hand strike.
// Rulings: [TOM 19951217].
#[test]
fn a_committed_strike_barred_by_immortal_grapple_leaves_no_strike() {
    let mut t = table();
    let a_gun = equip(&mut t.e, ".44 Magnum", t.a);
    let x_gun = equip(&mut t.e, ".44 Magnum", t.x);
    hand(&mut t.e, 0, &["Immortal Grapple"]);
    blocked_bleed(&mut t);
    let e = &mut t.e;
    pick(e, ability(a_gun, 1));
    // A cannot maneuver again (its gun's once each combat, and not twice in
    // a row): X's maneuver is the only one left.
    let req = expect(e, SeatId(1), WindowLabel::CombatDetermineRange);
    assert_eq!(req.options, vec![Choice::Pass, ability(x_gun, 1)]);
    pick(e, ability(x_gun, 1));
    pick(e, play(0, 0, t.a));
    let a_blood = blood(e, t.a);
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(e, t.a), blood(e, t.x)), (a_blood, 4));
}

// A melee weapon's strike is not a hand strike: under Immortal Grapple [pot]
// Kali's Fang is masked out and X hand strikes (1 damage, not strength+1
// aggravated).
#[test]
fn a_melee_weapon_is_barred_by_immortal_grapple_while_hand_strikes_stay() {
    let mut t = table();
    equip(&mut t.e, "Kali's Fang", t.x);
    hand(&mut t.e, 0, &["Immortal Grapple"]);
    blocked_bleed(&mut t);
    let e = &mut t.e;
    pick(e, play(0, 0, t.a));
    let a_blood = blood(e, t.a);
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.a.index()].ready());
    assert_eq!((blood(e, t.a), blood(e, t.x)), (a_blood - 1, 4 - 1));
}

// Sniper Rifle: the blocker sets the first round's range to long before
// range is determined — the determine range step is skipped (A's .44
// Magnum maneuver is never offered) and X's strike is the rifle's.
// Rulings: [RTR 19970630] [ANK 20180720].
#[test]
fn sniper_rifle_sets_long_range_skipping_determine_range_and_commits_the_strike() {
    let mut t = table();
    let a_gun = equip(&mut t.e, ".44 Magnum", t.a);
    let rifle = equip(&mut t.e, "Sniper Rifle", t.x);
    equip(&mut t.e, "Kali's Fang", t.x);
    blocked_bleed(&mut t);
    let e = &mut t.e;
    let req = expect(e, SeatId(1), WindowLabel::CombatBeforeRange);
    assert_eq!(req.options, vec![Choice::Pass, ability(rifle, 1)]);
    pick(e, ability(rifle, 1));
    let req = expect(e, SeatId(0), WindowLabel::CombatStrike);
    assert_eq!(req.options, vec![Choice::HandStrike, ability(a_gun, 0)]);
    pick(e, ability(a_gun, 0));
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(e, t.a), blood(e, t.x)), (4 - 2, 4 - 2));
}

// AK-47: after X strikes with it, 1 optional additional strike (limited)
// only usable with the gun — the gun's strike or none; the second gun
// strike gets no further one (one limited source a round).
#[test]
fn ak_47_gives_one_optional_additional_strike_only_with_the_gun() {
    let mut t = table();
    let ak = equip(&mut t.e, "AK-47", t.x);
    equip(&mut t.e, "Kali's Fang", t.x);
    blocked_bleed(&mut t);
    let e = &mut t.e;
    pick(e, Choice::Pass);
    let req = expect(e, SeatId(1), WindowLabel::CombatStrike);
    assert_eq!(
        req.options,
        vec![Choice::HandStrike, ability(ak, 0), ability(ak + 1, 0)]
    );
    pick(e, ability(ak, 0));
    let req = expect(e, SeatId(1), WindowLabel::CombatStrike);
    assert_eq!(req.options, vec![Choice::Pass, ability(ak, 0)]);
    pick(e, ability(ak, 0));
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(e, t.a), blood(e, t.x)), (4 - 2 - 2, 4 - 1));
}

// A clan weapon: equipping from hand needs the clan; taken from another
// minion it needs none, and the non-Tzimisce bearer strikes with it for
// strength+1 aggravated — X is wounded by the first point, burns 1 blood
// for the second, and goes to torpor.
// Rulings: [ANK 20180719-3].
#[test]
fn femur_of_toomler_taken_by_a_non_tzimisce_strikes_strength_plus_one_aggravated() {
    let mut t = table();
    assert_ne!(CRYPT[vampire_with("POT", 0) as usize].clan, "Tzimisce");
    let femur = equip(&mut t.e, "Femur of Toomler", t.b);
    hand(&mut t.e, 0, &["Femur of Toomler"]);
    let e = &mut t.e;
    let req = expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&play(0, 0, t.a)));
    pick(e, take_from(t.a, t.b));
    // Seat 0's next turn: A unlocked, bearing the Femur.
    pick(e, Choice::Pass);
    until(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[femur as usize].host, Some(t.a));
    pick(e, bleed(t.a));
    pick(e, Choice::DeclareBlock { minion: t.x });
    let req = expect(e, SeatId(0), WindowLabel::CombatStrike);
    assert_eq!(req.options, vec![Choice::HandStrike, ability(femur, 0)]);
    pick(e, ability(femur, 0));
    request(e);
    assert!(!e.state.minions[t.x.index()].ready());
    assert_eq!(blood(e, t.x), 4 - 1);
}

// Kevlar Vest's gun branch: part of the 2 points used against one AK-47
// strike, the rest kept and used against the gun's additional strike —
// one point a use, never against other damage (rules-qa 10.3).
// Rulings: [LSJ 20081210]; [RTR 20041202]; [ANK 20200318] [LSJ 20001114].
#[test]
fn kevlar_vest_spends_its_gun_points_over_separate_gun_strikes() {
    let mut t = table();
    let vest = equip(&mut t.e, "Kevlar Vest", t.a);
    let ak = equip(&mut t.e, "AK-47", t.x);
    blocked_bleed(&mut t);
    let e = &mut t.e;
    pick(e, Choice::Pass);
    pick(e, ability(ak, 0));
    // 2 gun damage on A (X takes A's hand strike): only the gun branch.
    let req = expect(e, SeatId(0), WindowLabel::CombatPrevention);
    assert_eq!(req.options, vec![Choice::Pass, ability(vest, 0)]);
    pick(e, ability(vest, 0));
    let req = expect(e, SeatId(0), WindowLabel::CombatPrevention);
    assert_eq!(req.options, vec![Choice::Pass, ability(vest, 0)]);
    pick(e, Choice::Pass);
    // The additional strike: the last point.
    pick(e, ability(ak, 0));
    let req = expect(e, SeatId(0), WindowLabel::CombatPrevention);
    assert_eq!(req.options, vec![Choice::Pass, ability(vest, 0)]);
    pick(e, ability(vest, 0));
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(e, t.a), blood(e, t.x)), (4 - 1 - 1, 4 - 1));
}

// Kevlar Vest's branch is chosen once each combat: its 1 point used against
// a hand strike, the later gun strike (Blur [cel]'s additional strike) finds
// nothing left.
#[test]
fn kevlar_vest_chooses_gun_or_other_once_each_combat() {
    let mut t = table();
    let z =
        t.e.add_minion(SeatId(1), find_card("Asmin Kobane"), 4, false);
    let vest = equip(&mut t.e, "Kevlar Vest", t.a);
    let ak = equip(&mut t.e, "AK-47", z);
    hand(&mut t.e, 1, &["Blur"]);
    let e = &mut t.e;
    pick(e, bleed(t.a));
    pick(e, Choice::DeclareBlock { minion: z });
    pick(e, Choice::Pass);
    pick(e, Choice::Pass);
    pick(e, Choice::HandStrike);
    // Z keeps Blur for after the pair.
    pick(e, Choice::Pass);
    let req = expect(e, SeatId(0), WindowLabel::CombatPrevention);
    assert_eq!(req.options, vec![Choice::Pass, ability(vest, 1)]);
    pick(e, ability(vest, 1));
    pick(e, play(0, 0, z));
    pick(e, ability(ak, 0));
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(e, t.a), 4 - 2);
}

// Righteous Blade: the press to continue is had as the strike is chosen and
// kept though Side Strike [cel] dodges it; the Blade's strike in the second
// round gives no second press (once each combat, rules-qa 10.4).
#[test]
fn righteous_blade_press_survives_a_dodge_once_each_combat() {
    let mut t = table();
    let blade = equip(&mut t.e, "Righteous Blade", t.x);
    hand(&mut t.e, 0, &["Side Strike"]);
    let e = &mut t.e;
    pick(e, bleed(t.b));
    pick(e, Choice::DeclareBlock { minion: t.x });
    // Before strikes: Side Strike's additional-strike mode is not wanted.
    pick(e, Choice::Pass);
    pick(e, play(0, 0, t.b));
    let b_blood = blood(e, t.b);
    pick(e, ability(blade, 0));
    let req = expect(e, SeatId(1), WindowLabel::CombatPress);
    assert_eq!(req.options, vec![Choice::Pass, Choice::UsePress]);
    pick(e, Choice::UsePress);
    // Round 2: the Blade again, strength+1 on B; no press this time.
    pick(e, ability(blade, 0));
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(e, t.b), blood(e, t.x)), (b_blood - 2, 4 - 1));
}
