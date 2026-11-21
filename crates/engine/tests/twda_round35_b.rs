//! Round 35 (`ADDED_BATCHES`): Bloodlines and Final Nights, track B: the
//! combat cards, the modifiers and reactions, the action cards from Abjure
//! onward and the vampires from Kemintiri onward. Windows with a single
//! option are elided, so the scripts only stop where a real decision
//! exists.

use zvtes::cards::Target;
use zvtes::crypt::{Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
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

/// The `nth` untitled fixture vampire matching `f` (`fixture_vampire`).
#[track_caller]
fn textless(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    fixture_vampire(|c| f(c) && c.title == Title::None, nth)
}

/// No printed stealth (`fixture_rank` 2): a block of it is not futile.
fn no_stealth(c: &zvtes::crypt::CryptCard) -> bool {
    let i = CRYPT
        .iter()
        .position(|x| x.id == c.id)
        .expect("in the crypt");
    fixture_rank(i as u16) != Some(2)
}

/// Seat 0's A (6 blood) and a second vampire, seat 1's X (6 blood) and a
/// second one: fixture vampires of capacity 7 or more, A with `a_levels`
/// and X with `x_levels` gained.
fn duel(a_levels: &[&'static str], x_levels: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let big = |c: &zvtes::crypt::CryptCard| c.capacity >= 7 && no_stealth(c);
    let a = e.add_minion(SeatId(0), textless(big, 0), 6, false);
    e.add_minion(SeatId(0), textless(big, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(big, 2), 6, false);
    e.add_minion(SeatId(1), textless(big, 3), 4, false);
    e.state.minions[a.index()].gained_levels = a_levels.to_vec();
    e.state.minions[x.index()].gained_levels = x_levels.to_vec();
    (e, a, x)
}

/// A bleeds, X blocks: their combat begins.
fn fight(e: &mut Engine, a: MinionId, x: MinionId) {
    pick(e, bleed(a));
    pick_when(e, block(x));
}

/// The card `name` of `host`'s controller put on `host` (no play).
fn on(e: &mut Engine, name: &str, host: MinionId) -> CardId {
    let owner = e.state.minions[host.index()].owner;
    e.state
        .cards
        .push(CardInPlay::new(find_library(name), owner, Some(host), 0));
    CardId(e.state.cards.len() as u32 - 1)
}

/// The cards named `name` in play on `m` (not effect-only stand-ins).
fn placed(e: &Engine, name: &str, m: MinionId) -> Vec<CardId> {
    let card = find_library(name);
    e.state
        .cards
        .iter()
        .enumerate()
        .filter(|(_, c)| c.card == card && c.in_play && !c.effect_only && c.host == Some(m))
        .map(|(i, _)| CardId(i as u32))
        .collect()
}

/// The hand card `card` played by `actor` in `mode` on `target`.
fn play_on(card: u8, mode: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target,
    }
}

/// The damage pending on combat side `side`: (normal, aggravated).
fn pending(e: &Engine, side: usize) -> (u8, u8) {
    let p = e.combat().expect("combat").sides[side].pending;
    (p.normal, p.aggravated)
}

fn ash_has(e: &Engine, seat: usize, name: &str) -> bool {
    e.state.seats[seat].ash.contains(&find_library(name))
}

/// Round 1 played out with hand strikes, A pressing on with Relentless
/// Pursuit (its hand card `rp`, [pot]): round 2's first request.
fn to_round_two(e: &mut Engine, a: MinionId, rp: u8) {
    pick_when(e, play(rp, 0, a));
    loop {
        let req = request(e);
        if e.combat().expect("combat").round == 2 {
            return;
        }
        e.choose(fallback(&req));
    }
}

fn is_play_of(o: &Choice, card: u8, m: MinionId) -> bool {
    matches!(o, Choice::Play { card: c, actor: Some(x), .. } if *c == card && *x == m)
}

// ---- Combat cards (combat_a) ---------------------------------------------------

/// Blessing of Durga Syn [CHI]: "Strike: steal equipment, with first
/// strike": any equipment, not only a weapon: X's Leather Jacket moves to
/// A as the strike resolves, X's own strike (a dodge) too late.
#[test]
fn blessing_of_durga_syn_steals_any_equipment_with_first_strike() {
    let (mut e, a, x) = duel(&["chi", "chi"], &[]);
    let jacket = on(&mut e, "Leather Jacket", x);
    hand(&mut e, 0, &["Blessing of Durga Syn"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play_on(0, 1, a, Target::Card(jacket)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[jacket.index()].host, Some(a));
    assert_eq!(blood(&e, a), 6 - 2 - 1, "2 blood, X's hand strike");
}

/// "Cannot be used if there is no equipment to steal." [RTR 19980928]
#[test]
fn blessing_of_durga_syn_not_offered_without_opposing_equipment() {
    let (mut e, a, x) = duel(&["chi", "chi"], &[]);
    hand(&mut e, 0, &["Blessing of Durga Syn"]);
    fight(&mut e, a, x);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        is_play_of(o, 0, a)
    });
}

/// Blood Agony [qui]: "Not usable during the first round of combat";
/// "must be used before strike resolution and damage prevention in order to
/// affect the current strike. Can be played after both strikes have been
/// chosen" [RTR 19960112]: never in round 1; in round 2 after both strikes
/// are chosen, A's hand strike lands aggravated.
#[test]
fn blood_agony_after_strikes_chosen_aggravates_the_current_hand_strike() {
    let (mut e, a, x) = duel(&["qui", "pot"], &["for"]);
    hand(&mut e, 0, &["Blood Agony", "Relentless Pursuit"]);
    hand(&mut e, 1, &["Skin of Rock"]);
    fight(&mut e, a, x);
    let agony = |o: &Choice| is_play_of(o, 0, a);
    never_offered_where(&mut e, SeatId(0), WindowLabel::CombatPress, agony);
    to_round_two(&mut e, a, 1);
    loop {
        let req = offering(&mut e, play(0, 0, a));
        if req.window == WindowLabel::CombatBeforeResolution {
            assert!(e.combat().expect("combat").sides[1].strike.is_some());
            e.choose(index_of(&req, play(0, 0, a)));
            break;
        }
        e.choose(index_of(&req, Choice::Pass));
    }
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (0, 1), "A's hand strike aggravated");
}

/// Blood Agony [qui]: "next hand or melee weapon strike": spent by it,
/// round 3's hand strike normal again; [QUI] keeps them aggravated all
/// combat.
#[test]
fn blood_agony_inferior_spends_on_one_strike_superior_lasts() {
    for (mode, third) in [(0u8, (1u8, 0u8)), (1, (0, 1))] {
        let (mut e, a, x) = duel(&["qui", "qui", "pot"], &["for"]);
        hand(
            &mut e,
            0,
            &["Blood Agony", "Relentless Pursuit", "Relentless Pursuit"],
        );
        hand(&mut e, 1, &["Skin of Rock", "Skin of Rock"]);
        fight(&mut e, a, x);
        to_round_two(&mut e, a, 1);
        pick_when(&mut e, play(0, mode, a));
        offering(&mut e, play(0, 0, x));
        assert_eq!(pending(&e, 1), (0, 1));
        pick(&mut e, play(0, 0, x));
        pick_when(&mut e, play(0, 0, a));
        loop {
            let req = request(&mut e);
            if e.combat().expect("combat").round == 3 {
                break;
            }
            e.choose(fallback(&req));
        }
        offering(&mut e, play(0, 0, x));
        assert_eq!(pending(&e, 1), third, "mode {mode}");
    }
}

/// Blood Sweat [QUI], Exuding Blood [qui]: "Only usable at long range":
/// not offered at close range; at long range (Fake Out), unpreventable: X's
/// Skin of Rock never offered against it.
#[test]
fn blood_sweat_and_exuding_blood_strike_unpreventably_at_long_range() {
    for (name, mode, n) in [("Blood Sweat", 1u8, 3u8), ("Exuding Blood", 0, 1)] {
        let (mut e, a, x) = duel(&["qui", "qui"], &["for"]);
        hand(&mut e, 0, &[name]);
        fight(&mut e, a, x);
        never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
            is_play_of(o, 0, a)
        });

        let (mut e, a, x) = duel(&["qui", "qui"], &["for"]);
        hand(&mut e, 0, &["Fake Out", name]);
        hand(&mut e, 1, &["Skin of Rock"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, 0, a));
        pick_when(&mut e, play(0, mode, a));
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
        assert_eq!(blood(&e, x), 6 - n, "{name}");
    }
}

/// Coagulate Blood [qui]: "Strike: 1R damage. If the opposing minion is an
/// ally, this strike inflicts +1 damage": 2 life off a blocking ally, 1
/// blood off a vampire.
#[test]
fn coagulate_blood_hits_an_ally_harder() {
    let (mut e, a, x) = duel(&["qui"], &[]);
    hand(&mut e, 0, &["Coagulate Blood"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 6 - 1);

    let (mut e, a, _) = duel(&["qui"], &[]);
    let ally = e.add_ally(SeatId(1), find_library("Carlton Van Wyk"));
    e.state.minions[ally.index()].blood = 3;
    hand(&mut e, 0, &["Coagulate Blood"]);
    fight(&mut e, a, ally);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ally), 3 - 2, "1 + 1 against an ally");
}

/// Coagulate Blood: "If the opposing minion is an ally, this strike
/// inflicts +1 damage", aimed at the ally's retainer at long range too (the
/// clause keyed to the opposing minion: advanced rules §4.2.3, Earthshock):
/// City Star Taxi blocks and moves to long range; A's Coagulate Blood at
/// its 3-life Corpse Minion takes 2.
#[test]
fn coagulate_blood_adds_one_at_an_allys_retainer() {
    let (mut e, a, _) = duel(&["qui"], &[]);
    let taxi = e.add_ally(SeatId(1), find_library("City Star Taxi"));
    let taxi_card = e.state.minions[taxi.index()]
        .ally_id()
        .expect("an ally card");
    let corpse = on(&mut e, "Corpse Minion", taxi);
    e.state.cards[corpse.index()].counters = 3;
    hand(&mut e, 0, &["Coagulate Blood"]);
    fight(&mut e, a, taxi);
    until(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    pick(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(taxi_card),
            ability: 1,
            actor: None,
            target: Target::None,
        },
    );
    until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::Card(corpse),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[corpse.index()].counters, 3 - 2, "1 + 1");
}

/// Disease [QUI]: "Only usable before range is determined. The opposing
/// minion gets -2 strength this combat": X (strength 1) strikes for none
/// in round 1 and again in round 2.
#[test]
fn disease_weakens_the_opponent_all_combat() {
    let (mut e, a, x) = duel(&["qui", "qui", "pot"], &[]);
    hand(&mut e, 0, &["Disease", "Relentless Pursuit"]);
    fight(&mut e, a, x);
    let req = offering(&mut e, play(0, 1, a));
    assert_eq!(req.window, WindowLabel::CombatBeforeRange);
    e.choose(index_of(&req, play(0, 1, a)));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, a),
        6 - 1 - 1,
        "the cards' costs, X's strikes nothing"
    );
    assert_eq!(blood(&e, x), 6 - 2);
}

/// Far Fatuus [CHI]: "Strike: 2R damage; only usable at long range ...
/// and prevent all damage from opponent's strikes for the remainder of this
/// round": not at close range; at long range X's Coagulate Blood (1R)
/// prevented, A's 2 lands.
#[test]
fn far_fatuus_superior_prevents_the_opposing_strikes_this_round() {
    let (mut e, a, x) = duel(&["chi", "chi"], &[]);
    hand(&mut e, 0, &["Far Fatuus"]);
    fight(&mut e, a, x);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        is_play_of(o, 0, a)
    });

    let (mut e, a, x) = duel(&["chi", "chi"], &["qui"]);
    hand(&mut e, 0, &["Fake Out", "Far Fatuus"]);
    hand(&mut e, 1, &["Coagulate Blood"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 6 - 2, "only the cost");
    assert_eq!(blood(&e, x), 6 - 2);
}

/// Blood of the Cobra [QUI]: "Play a non-ranged strike that requires
/// Quietus [qui] from your hand and use that strike as if it were ranged":
/// "the hand or melee weapon portion of the strike is still only effective
/// at close range" [RTR 20010711]. At long range (Fake Out), Scorpion's
/// Touch played through it: no hand damage on X, the card itself on X (not
/// a stand-in, out of the ash heap), its 1 blood paid.
#[test]
fn blood_of_the_cobra_scorpions_touch_at_long_range_lands_without_damage() {
    let (mut e, a, x) = duel(&["qui", "qui"], &[]);
    hand(
        &mut e,
        0,
        &["Fake Out", "Blood of the Cobra", "Scorpion's Touch"],
    );
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, a));
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(0));
    assert!(
        req.options
            .iter()
            .all(|o| matches!(o, Choice::Play { card: 0, .. })),
        "only the strike to play: {:?}",
        req.options
    );
    e.choose(index_of(&req, play(0, 0, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(placed(&e, "Scorpion's Touch", x).len(), 1);
    assert!(!ash_has(&e, 0, "Scorpion's Touch"));
    assert_eq!(blood(&e, x), 6, "no hand damage at long range");
}

/// Blood of the Cobra [QUI] names an object there may be none of
/// (rules-digest D14): no non-ranged Quietus strike in hand, not offered;
/// [qui] (1R damage) still is.
#[test]
fn blood_of_the_cobra_superior_needs_a_quietus_strike_in_hand() {
    let (mut e, a, x) = duel(&["qui", "qui"], &[]);
    hand(&mut e, 0, &["Blood of the Cobra", "Taste of Death"]);
    fight(&mut e, a, x);
    let req = offering(&mut e, play(0, 0, a));
    assert!(!req.options.contains(&play(0, 1, a)), "{:?}", req.options);
    e.choose(index_of(&req, play(0, 0, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 6 - 1);
}

/// Bond with the Mountain [vis]: "Strike: combat ends. Unlock both
/// combatants." "The unlock effect takes place before the end of combat"
/// [RTR 19970630]: X ends the combat, the bleeding A unlocked; [VIS]
/// ("Unlock this vampire"): A stays locked.
#[test]
fn bond_with_the_mountain_ends_combat_unlocking_both() {
    for (mode, a_unlocked) in [(1u8, true), (2, false)] {
        let (mut e, a, x) = duel(&[], &["vis", "vis"]);
        hand(&mut e, 1, &["Bond with the Mountain"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, mode, x));
        request(&mut e);
        assert!(e.combat().is_none(), "combat over");
        assert!(!e.state.minions[x.index()].locked);
        assert_eq!(
            !e.state.minions[a.index()].locked,
            a_unlocked,
            "mode {mode}"
        );
    }
}

/// Bond with the Mountain [tha]: "Strike: dodge, with an optional press":
/// A's hand strike dodged, X may press.
#[test]
fn bond_with_the_mountain_dodges_with_a_press() {
    let (mut e, a, x) = duel(&[], &["tha"]);
    hand(&mut e, 1, &["Bond with the Mountain"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatPress);
    assert!(req.options.len() > 1, "{:?}", req.options);
    assert_eq!(blood(&e, x), 6 - 1, "the cost, A's strike dodged");
}

/// Seat 0's A (4 blood) and a second vampire; seat 1's X, with Animalism
/// (`ani` levels), in torpor with 2 blood, locked, and a second vampire;
/// seat 1 holds Crimson Fury.
fn torpid_animalism(ani: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity);
    e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 8, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(mid, 1), 2, true);
    e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.state.minions[x.index()].gained_levels = ani.to_vec();
    e.state.minions[x.index()].locked = true;
    hand(&mut e, 1, &["Crimson Fury"]);
    (e, a, x)
}

fn diablerize(a: MinionId, x: MinionId) -> Choice {
    Choice::TakeAction {
        minion: a,
        action: ActionKind::Diablerie,
        target: Some(x),
    }
}

/// Crimson Fury [ani]: "Only usable when this vampire is being
/// diablerized. Usable by a locked vampire. Usable by a vampire in torpor.
/// The diablerie is canceled and the diablerist burns 1 blood." The locked
/// torpid X answers before the diablerie's steps (rules-qa 122.1): it stays
/// in torpor, its 1 blood paid; A burns 1; no diablerie made (rules-qa
/// 121.20).
#[test]
fn crimson_fury_cancels_a_diablerie_action() {
    let (mut e, a, x) = torpid_animalism(&["ani"]);
    pick(&mut e, diablerize(a, x));
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Pass, play(0, 0, x)]);
    e.choose(1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[x.index()];
    assert!(v.in_play && v.in_torpor);
    assert_eq!(v.blood, 2 - 1);
    assert_eq!(blood(&e, a), 4 - 1);
    assert!(e.state.minions[a.index()].diableries.0.is_empty());
    assert!(ash_has(&e, 1, "Crimson Fury"));
}

/// Crimson Fury [ANI]: "Burn both the diablerist and this vampire. (The
/// diablerie is still considered successful.)" A burned at once, X burned
/// once as the diablerie (LSJ 20011008): nothing gained, both crypt cards
/// in their ash heaps.
#[test]
fn crimson_fury_superior_burns_both_the_diablerie_still_made() {
    let (mut e, a, x) = torpid_animalism(&["ani", "ani"]);
    let (a_card, x_card) = (
        e.state.minions[a.index()].crypt_card().expect("vampire"),
        e.state.minions[x.index()].crypt_card().expect("vampire"),
    );
    pick(&mut e, diablerize(a, x));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].in_play);
    assert!(!e.state.minions[x.index()].in_play);
    assert!(e.state.seats[0].ash_crypt.contains(&a_card));
    assert!(e.state.seats[1].ash_crypt.contains(&x_card));
}

/// Crimson Fury [ANI]: the diablerist's burn is asked like any minion's
/// (rules-qa 122.1), not by diablerie: A diablerizes the torpid X, X's
/// Crimson Fury [ANI] burns A, and seat 1's Nagaraja is offered Consume the
/// Dead ("by any means except diablerie"): A removed from the game, not
/// burned, the Nagaraja +1 blood. (A's own Abandoning the Flesh is a
/// reaction in its own action: never offered.)
#[test]
fn crimson_fury_superior_burn_of_the_diablerist_is_asked() {
    let (mut e, a, x) = torpid_animalism(&["ani", "ani"]);
    let n = e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 3),
        2,
        false,
    );
    e.state.minions[n.index()].clan_change = Some("Nagaraja");
    let a_card = e.state.minions[a.index()].crypt_card().expect("vampire");
    hand(&mut e, 1, &["Consume the Dead"]);
    pick(&mut e, diablerize(a, x));
    pick_when(&mut e, play(0, 1, x));
    let consume = play(0, 2, n);
    let req = offering(&mut e, consume);
    assert_eq!(req.seat, SeatId(1));
    pick(&mut e, consume);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].in_play);
    assert!(e.state.seats[0].removed_crypt.contains(&a_card));
    assert!(!e.state.seats[0].ash_crypt.contains(&a_card));
    assert_eq!(blood(&e, n), 3);
}

/// Crimson Fury's moment is a diablerie's: a torpid X sent there by a
/// plain combat burn... none: X burned by Amaranth's absence. A blocked
/// leave-torpor's diablerie is its own Methuselah's action: no reaction
/// (LSJ 20011220).
#[test]
fn crimson_fury_not_in_its_own_leave_torpor_block() {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity) && no_stealth(c);
    let x = e.add_minion(SeatId(0), textless(mid, 0), 2, true);
    e.add_minion(SeatId(0), textless(mid, 1), 4, false);
    let b = e.add_minion(SeatId(1), textless(|c| c.capacity >= 9, 0), 4, false);
    e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["ani"];
    hand(&mut e, 0, &["Crimson Fury"]);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: x,
            action: ActionKind::LeaveTorpor,
            target: None,
        },
    );
    pick_when(&mut e, block(b));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, x));
}

/// Dagon's Call [qui]: "hand strike, and the opposing minion takes 1
/// unpreventable environmental damage during the press step each round this
/// combat (only effective if this strike was made at close range)": X takes
/// A's hand 1 and 1 in round 1's press step; A presses on, round 2 the
/// same: 4 in all.
#[test]
fn dagons_call_burns_each_press_step_until_the_combat_ends() {
    let (mut e, a, x) = duel(&["qui", "pot"], &[]);
    hand(&mut e, 0, &["Dagon's Call", "Relentless Pursuit"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 6 - 4);
}

/// "You are correct" (a dodge ignores the damage during press) [LSJ
/// 20010612]: X's Vampiric Speed dodges Dagon's Call: no press damage.
#[test]
fn dagons_call_dodged_does_nothing_in_the_press_step() {
    let (mut e, a, x) = duel(&["qui"], &["cel"]);
    hand(&mut e, 0, &["Dagon's Call"]);
    hand(&mut e, 1, &["Vampiric Speed"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 6, "no hand damage, no press damage");
}

/// "Both the strike and the environmental damage would get boosted by Dam
/// the Heart's River" [PIB 20150520]: X takes 1 + 1 from the hand strike
/// and 1 + 1 in the press step (a fix: the press step's damage took no
/// Dam).
#[test]
fn dagons_call_press_damage_takes_dam_the_hearts_river() {
    let (mut e, a, x) = duel(&["qui"], &[]);
    hand(&mut e, 0, &["Dam the Heart's River", "Dagon's Call"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 6 - 4);
}

/// Dagon's Call [QUI]: "this hand strike is at +1 damage", which "inherits
/// all of the properties of the base damage" [TOM 19960225]: under Blood
/// Agony [QUI] both points aggravated; "only one Dagon's Call each combat":
/// a second copy not offered in round 2.
#[test]
fn dagons_call_superior_bonus_is_the_hand_strikes_and_once_a_combat() {
    let (mut e, a, x) = duel(&["qui", "qui", "pot"], &["for"]);
    hand(
        &mut e,
        0,
        &["Dagon's Call", "Dagon's Call", "Relentless Pursuit"],
    );
    hand(&mut e, 1, &["Skin of Rock", "Skin of Rock"]);
    fight(&mut e, a, x);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::Combat,
        SeatId(0),
    ));
    pick_when(&mut e, play(0, 1, a));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (0, 2), "1 + 1, all aggravated");
    pick(&mut e, play(0, 1, x));
    to_round_two(&mut e, a, 1);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        is_play_of(o, 0, a)
    });
}

/// Seat 0's young A (capacity 3 to 5, Presence `a_levels`) and a second
/// vampire; seat 1's old X (capacity 8 or more, Dominate and Fortitude
/// `x_levels`) and a second one; seat 1 holds Denial of Aphrodite's Favor.
fn denial(a_levels: &[&'static str], x_levels: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity) && no_stealth(c);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| (3..=5).contains(&c.capacity) && no_stealth(c), 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    let x = e.add_minion(
        SeatId(1),
        textless(|c| c.capacity >= 8 && no_stealth(c), 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), textless(mid, 1), 4, false);
    e.state.minions[a.index()].gained_levels = a_levels.to_vec();
    e.state.minions[x.index()].gained_levels = x_levels.to_vec();
    hand(&mut e, 1, &["Denial of Aphrodite's Favor"]);
    (e, a, x)
}

/// The play of A's hand card `card`, in the first mode offered.
fn first_play(e: &mut Engine, card: u8, a: MinionId) -> Choice {
    let req = request(e);
    *req.options
        .iter()
        .find(|o| is_play_of(o, card, a))
        .unwrap_or_else(|| panic!("no play of {card}: {:?}", req.options))
}

/// Denial of Aphrodite's Favor [dom][for]: "Cancel an action card a younger
/// vampire plays that requires Presence [pre] as it is played. No cost is
/// paid. Do not lock the acting vampire; that vampire cannot attempt the
/// same action again this turn." [RBK cancel-a-card]: the card's text over
/// "free to attempt the same action again". A's Intimidation canceled: A
/// unlocked, no action, the card in its ash heap, a second Intimidation not
/// offered to A this turn, a bleed still is; Denial to seat 1's ash heap.
#[test]
fn denial_of_aphrodites_favor_cancels_a_younger_presence_action_card() {
    let (mut e, a, x) = denial(&["pre"], &["dom", "for"]);
    hand(&mut e, 0, &["Intimidation", "Intimidation"]);
    let p = first_play(&mut e, 0, a);
    pick(&mut e, p);
    let deny = play(0, 0, x);
    let req = offering(&mut e, deny);
    assert_eq!(req.window, WindowLabel::CardAsPlayed);
    e.choose(index_of(&req, deny));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::MinionPhase)
    );
    assert!(!e.state.minions[a.index()].locked, "not locked");
    assert!(ash_has(&e, 0, "Intimidation"));
    assert!(ash_has(&e, 1, "Denial of Aphrodite's Favor"));
    assert_eq!(e.state.seats[0].pool, 10, "no cost");
    assert!(
        !req.options.iter().any(|o| is_play_of(o, 0, a)),
        "{:?}",
        req.options
    );
    assert!(req.options.contains(&bleed(a)));
}

/// Denial of Aphrodite's Favor [DOM][FOR]: "Cancel an action modifier or
/// combat card a younger vampire plays that requires Presence": A's
/// Majesty canceled as played; [dom][for] alone offers no combat cancel.
#[test]
fn denial_of_aphrodites_favor_superior_cancels_a_presence_combat_card() {
    let (mut e, a, x) = denial(&["pre"], &["dom", "dom", "for", "for"]);
    hand(&mut e, 0, &["Majesty"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, a));
    let req = offering(&mut e, play(0, 1, x));
    assert_eq!(req.window, WindowLabel::CardAsPlayed);
    e.choose(index_of(&req, play(0, 1, x)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Majesty"));
    assert_eq!(blood(&e, x), 4 - 1, "Majesty canceled, A's hand strike");

    let (mut e, a, x) = denial(&["pre"], &["dom", "for"]);
    hand(&mut e, 0, &["Majesty"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4, "Majesty ended the combat");
}

/// "Younger": by capacity, a vampire only (rules-digest D30): an older A's
/// Intimidation offers no Denial.
#[test]
fn denial_of_aphrodites_favor_not_against_an_older_vampire() {
    let mut e = game(2);
    let old = |c: &zvtes::crypt::CryptCard| c.capacity >= 7 && no_stealth(c);
    let young = |c: &zvtes::crypt::CryptCard| (3..=5).contains(&c.capacity) && no_stealth(c);
    let a = e.add_minion(SeatId(0), textless(old, 0), 4, false);
    e.add_minion(SeatId(0), textless(young, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(young, 1), 4, false);
    e.add_minion(SeatId(1), textless(young, 2), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["pre"];
    e.state.minions[x.index()].gained_levels = vec!["dom", "for"];
    hand(&mut e, 0, &["Intimidation"]);
    hand(&mut e, 1, &["Denial of Aphrodite's Favor"]);
    let p = first_play(&mut e, 0, a);
    pick(&mut e, p);
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, x));
}

/// Draught of the Soul [AUS][QUI] on a diablerie: "can be played before or
/// after getting a Discipline card (if any), but must be played before the
/// Blood Hunt" [RTR 19991206]. A diablerizes seat 1's torpid X and plays it
/// after the diablerie: +1 blood, the card itself on A, +1 intercept, +1
/// bleed against seat 1 only.
#[test]
fn draught_of_the_soul_after_a_diablerie_action_before_the_hunt() {
    let mut e = game(3);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity);
    e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 8, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(mid, 1), 2, true);
    e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.add_minion(SeatId(2), textless(mid, 3), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["aus", "aus", "qui", "qui"];
    hand(&mut e, 0, &["Draught of the Soul"]);
    pick(&mut e, diablerize(a, x));
    let req = offering(&mut e, play(0, 4, a));
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    e.choose(index_of(&req, play(0, 4, a)));
    request(&mut e);
    assert_eq!(blood(&e, a), 4 + 2 + 1);
    assert_eq!(placed(&e, "Draught of the Soul", a).len(), 1);
    assert!(!ash_has(&e, 0, "Draught of the Soul"));
    assert_eq!(e.state.bearer_always(a, zvtes::cards::Stat::Intercept), 1);
    assert_eq!(e.state.bleed_against_mod(a, SeatId(1)), 1);
    assert_eq!(e.state.bleed_against_mod(a, SeatId(2)), 0);
}

/// "If a minion is burned in combat, his opponent his always considered to
/// have burned him" [LSJ 20090922]: A's hand strike sends the empty X down,
/// Decapitate [POT] burns it instead; at the round's end A plays Draught of
/// the Soul [qui] as a combat card: +1 blood.
#[test]
fn draught_of_the_soul_after_burning_the_blocker_in_combat() {
    let (mut e, a, x) = duel(&["pot", "pot", "qui"], &[]);
    e.state.minions[x.index()].blood = 0;
    hand(&mut e, 0, &["Decapitate", "Draught of the Soul"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    let req = offering(&mut e, play(0, 1, a));
    assert!(!e.state.minions[x.index()].in_play, "burned");
    let before = blood(&e, a);
    assert_eq!(req.window, WindowLabel::CombatEndOfRound);
    e.choose(index_of(&req, play(0, 1, a)));
    request(&mut e);
    assert_eq!(blood(&e, a), before + 1);
}

/// Elemental Stoicism [for]: "Prevent 1 non-aggravated damage": "cannot be
/// used to prevent aggravated damage, even if the minion treats them as
/// normal damage" [LSJ 20040812-2] [PIB 20130327]; against A's aggravated
/// hand strike, X's copy not offered.
#[test]
fn elemental_stoicism_cannot_prevent_aggravated_damage() {
    let (mut e, a, x) = duel(&[], &["for"]);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::Combat,
        SeatId(0),
    ));
    hand(&mut e, 1, &["Elemental Stoicism"]);
    fight(&mut e, a, x);
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, x));
}

/// Elemental Stoicism [FOR][obf]: "+1 stealth action. Put this card on this
/// acting vampire. The vampire with this card treats aggravated damage done
/// in combat as normal damage. Burn this card if the vampire goes to
/// torpor." A's action puts the card itself on A; then in combat X's
/// aggravated hand strike lands on A as normal damage; sent to torpor, the
/// card burns.
#[test]
fn elemental_stoicism_action_puts_it_on_the_vampire_in_combat_only() {
    let (mut e, a, x) = duel(&["for", "for", "obf"], &[]);
    hand(&mut e, 0, &["Elemental Stoicism"]);
    let p = first_play(&mut e, 0, a);
    pick(&mut e, p);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = placed(&e, "Elemental Stoicism", a);
    assert_eq!(card.len(), 1);
    assert!(!ash_has(&e, 0, "Elemental Stoicism"));

    let (mut e, a, x2) = (e, a, x);
    e.state.minions[x2.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::Combat,
        SeatId(1),
    ));
    e.state.minions[a.index()].locked = false;
    e.state.minions[a.index()].blood = 0;
    pick(&mut e, bleed(x2));
    pick_when(&mut e, block(a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let v = &e.state.minions[a.index()];
    assert!(
        v.in_play && v.in_torpor,
        "normal damage: torpor, not burned"
    );
    assert!(placed(&e, "Elemental Stoicism", a).is_empty());
    assert!(ash_has(&e, 0, "Elemental Stoicism"));
}

/// Eruption of Vitae [QUI]: "this vampire can burn X blood and randomly
/// discard X cards to inflict X additional non-aggravated damage this
/// strike": round 2, at long range, X up to the one other card in hand
/// (rules-digest D3), X = 1: A burns 1, the card discarded; X takes 2
/// aggravated and 1 normal. Not offered in round 1.
#[test]
fn eruption_of_vitae_superior_burns_and_discards_for_normal_damage() {
    let (mut e, a, x) = duel(&["qui", "qui", "pot"], &["for"]);
    hand(
        &mut e,
        0,
        &[
            "Eruption of Vitae",
            "Relentless Pursuit",
            "Fake Out",
            "Skin of Rock",
        ],
    );
    hand(&mut e, 1, &["Skin of Rock"]);
    fight(&mut e, a, x);
    let eruption = |o: &Choice| is_play_of(o, 0, a);
    never_offered_where(&mut e, SeatId(0), WindowLabel::CombatPress, eruption);
    to_round_two(&mut e, a, 1);
    pick_when(&mut e, play(1, 0, a));
    pick_when(&mut e, play(0, 1, a));
    let before = blood(&e, a);
    let req = request(&mut e);
    assert_eq!(
        req.options,
        vec![set_x(0), set_x(1)],
        "X up to the other card"
    );
    e.choose(index_of(&req, set_x(1)));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (1, 2));
    assert_eq!(blood(&e, a), before - 1 - 1, "its cost and X");
    assert!(e.state.seats[0].hand.is_empty());
    assert_eq!(
        e.state.seats[0].ash.len(),
        4,
        "Relentless Pursuit, Fake Out, the discard, the card itself"
    );
}

// ---- Combat cards (combat_b) ---------------------------------------------------

/// Form of the Serpent: "[ser] [ACTION MODIFIER] +1 stealth. [SER]
/// [COMBAT] Maneuver." [SER] in combat: A maneuvers to long range, its
/// Exuding Blood ("only usable at long range") offered.
#[test]
fn form_of_the_serpent_superior_maneuvers() {
    let (mut e, a, x) = duel(&["ser", "ser", "qui"], &[]);
    hand(&mut e, 0, &["Form of the Serpent", "Exuding Blood"]);
    fight(&mut e, a, x);
    let req = offering(&mut e, play(0, 1, a));
    assert_eq!(req.window, WindowLabel::CombatDetermineRange);
    e.choose(index_of(&req, play(0, 1, a)));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(
        req.window,
        WindowLabel::CombatStrike,
        "long range: Exuding Blood"
    );
}

/// `seat`'s library, top last (set once the combat began: a bare seat
/// draws its hand up from it).
fn library(e: &mut Engine, seat: usize, cards: &[&str]) {
    e.state.seats[seat].library = cards
        .iter()
        .map(|c| find_library(c))
        .collect::<Vec<_>>()
        .into();
}

/// Horrid Reality [chi]: "Equip this vampire with the first weapon you find
/// in your library (working down from the top ...). Do not pay the cost
/// for this weapon. At the end of this combat, burn the weapon if it is
/// still in play." A's .44 Magnum, under a Fake Out, on A, no pool paid;
/// as the combat ends, in the ash heap.
#[test]
fn horrid_reality_equips_the_first_weapon_and_burns_it_after_combat() {
    let (mut e, a, x) = duel(&["chi"], &[]);
    hand(&mut e, 0, &["Horrid Reality"]);
    fight(&mut e, a, x);
    library(&mut e, 0, &["Skin of Rock", ".44 Magnum", "Fake Out"]);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(placed(&e, ".44 Magnum", a).len(), 1);
    assert_eq!(e.state.seats[0].pool, 10, "no cost paid for the weapon");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(placed(&e, ".44 Magnum", a).is_empty());
    assert!(ash_has(&e, 0, ".44 Magnum"));
}

/// Horrid Reality [CHI]: "shuffle the equipment into your library at the
/// end of combat if it is still in play"; "The weapon is burned or returned
/// to the library if it is still in play, even if it is not on the minion
/// anymore" [LSJ 20001019]: X steals it with Blessing of Durga Syn, it
/// goes back to A's library all the same.
#[test]
fn horrid_reality_fate_follows_the_weapon_off_its_minion() {
    let (mut e, a, x) = duel(&["chi", "chi"], &["chi"]);
    hand(&mut e, 0, &["Horrid Reality"]);
    hand(&mut e, 1, &["Blessing of Durga Syn"]);
    fight(&mut e, a, x);
    library(&mut e, 0, &[".44 Magnum"]);
    pick_when(&mut e, play(0, 1, a));
    request(&mut e);
    let gun = placed(&e, ".44 Magnum", a)[0];
    pick_when(&mut e, play_on(0, 0, x, Target::Card(gun)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[gun.index()].in_play);
    let magnum = find_library(".44 Magnum");
    let seat = &e.state.seats[0];
    assert!(
        seat.library.contains(&magnum) || seat.hand.contains(&magnum),
        "back, drawn or not"
    );
    assert!(!ash_has(&e, 0, ".44 Magnum"));
}

/// "If the weapon retrieved has requirements that the vampire does not
/// meet, then it is burned" [RTR 20010711]: Sword of Nuln (Tremere) to
/// the ash heap; "If no weapon can be found, the cost is still paid" [TOM
/// 19951212-1].
#[test]
fn horrid_reality_burns_a_weapon_whose_requirements_fail_and_costs_without_one() {
    let (mut e, a, x) = duel(&["chi"], &[]);
    assert_ne!(e.state.minions[a.index()].clan(), Some("Tremere"));
    hand(&mut e, 0, &["Horrid Reality"]);
    fight(&mut e, a, x);
    library(&mut e, 0, &["Sword of Nuln"]);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert!(placed(&e, "Sword of Nuln", a).is_empty());
    assert!(ash_has(&e, 0, "Sword of Nuln"));

    let (mut e, a, x) = duel(&["chi"], &[]);
    hand(&mut e, 0, &["Horrid Reality"]);
    fight(&mut e, a, x);
    library(&mut e, 0, &["Skin of Rock"]);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(
        blood(&e, a),
        6 - 2 - 1,
        "paid, nothing found; X's hand strike"
    );
    assert!(e.state.cards.iter().all(|c| !c.in_play), "nothing equipped");
}

/// A bare crypt fixture: seat 0's crypt holds `cards`, bottom first.
fn crypt(e: &mut Engine, seat: usize, cards: &[u16]) {
    e.state.seats[seat].crypt = cards.to_vec().into();
}

/// The minion in play of crypt card `card` for `seat`, newest.
fn minion_of(e: &Engine, seat: usize, card: u16) -> Option<MinionId> {
    (0..e.state.minions.len())
        .rev()
        .map(|i| MinionId(i as u32))
        .find(|&m| {
            let v = &e.state.minions[m.index()];
            v.owner == SeatId(seat as u8) && v.in_play && v.crypt_card() == Some(card)
        })
}

/// Illusions of the Kindred [chi]: "Combat ends. Move the bottom card from
/// your crypt to your ready region ... gains blood equal to half their
/// capacity rounded down. That vampire and the opposing minion enter
/// combat. Remove that minion from the game at the end of combat." The
/// bottom vampire V enters with half its capacity, fights X, and is
/// removed from the game as that combat ends.
#[test]
fn illusions_of_the_kindred_fights_with_the_bottom_crypt_card() {
    let (mut e, a, x) = duel(&["chi"], &["for"]);
    let v_card = textless(|c| c.capacity >= 6 && no_stealth(c), 6);
    let top = textless(|c| c.capacity >= 6 && no_stealth(c), 7);
    crypt(&mut e, 0, &[v_card, top]);
    hand(&mut e, 0, &["Illusions of the Kindred"]);
    hand(&mut e, 1, &["Skin of Rock"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    let v = minion_of(&e, 0, v_card).expect("the illusion in play");
    assert_eq!(blood(&e, v), CRYPT[v_card as usize].capacity / 2);
    let sides = e
        .combat()
        .expect("the illusion's combat")
        .sides
        .each_ref()
        .map(|s| s.minion);
    assert!(sides.contains(&v) && sides.contains(&x), "{sides:?}");
    assert_eq!(e.state.seats[0].crypt.len(), 1, "the bottom card taken");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[v.index()].in_play);
    assert!(e.state.seats[0].removed_crypt.contains(&v_card));
}

/// "Cannot be played when the target crypt is empty" [RTR 20000501] [LSJ
/// 20070411-2].
#[test]
fn illusions_not_playable_with_an_empty_crypt() {
    let (mut e, a, x) = duel(&["chi"], &[]);
    hand(&mut e, 0, &["Illusions of the Kindred"]);
    fight(&mut e, a, x);
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        is_play_of(o, 0, a)
    });
}

/// "Queues a combat. No other combat can be queued afterwards (eg. Psyche!
/// cannot be used)" [LSJ 19971003]; "Cannot be used if there is already
/// another combat queued" [RTR 20020501]: X's Psyche! never offered between
/// the two combats.
#[test]
fn illusions_bars_psyche() {
    let (mut e, a, x) = duel(&["chi"], &["cel", "cel"]);
    let v_card = textless(|c| c.capacity >= 6 && no_stealth(c), 6);
    crypt(&mut e, 0, &[v_card]);
    hand(&mut e, 0, &["Illusions of the Kindred"]);
    hand(&mut e, 1, &["Psyche!"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&play(0, 1, x)), "{:?}", req.options);
        let v = minion_of(&e, 0, v_card);
        if v.is_some_and(|v| {
            e.combat()
                .is_some_and(|c| c.sides.iter().any(|s| s.minion == v))
        }) {
            break;
        }
        e.choose(fallback(&req));
    }
}

/// [LSJ 20031008] "Psyche! cannot be played at either time ... the vampire
/// that is removed from the game is not ready": at the end of the
/// illusion's own combat, X's Psyche! is never offered.
#[test]
fn illusions_vampire_cannot_psyche_in_its_own_combat() {
    let (mut e, a, x) = duel(&["chi"], &["cel", "cel"]);
    let v_card = textless(|c| c.capacity >= 6 && no_stealth(c), 6);
    crypt(&mut e, 0, &[v_card]);
    hand(&mut e, 0, &["Illusions of the Kindred"]);
    hand(&mut e, 1, &["Psyche!"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    loop {
        let req = request(&mut e);
        let v = minion_of(&e, 0, v_card);
        if v.is_some_and(|v| {
            e.combat()
                .is_some_and(|c| c.sides.iter().any(|s| s.minion == v))
        }) {
            break;
        }
        e.choose(fallback(&req));
    }
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        *o == play(0, 1, x)
    });
}

/// "If the combat continues nonetheless (eg. Telepathic Tracking), the card
/// has no effect (no crypt card moved, no combat)" [RTR 20020501].
#[test]
fn illusions_lost_when_telepathic_tracking_continues_the_combat() {
    let (mut e, a, x) = duel(&["chi"], &["aus", "for"]);
    let v_card = textless(|c| c.capacity >= 6 && no_stealth(c), 6);
    crypt(&mut e, 0, &[v_card]);
    hand(&mut e, 0, &["Illusions of the Kindred"]);
    hand(&mut e, 1, &["Telepathic Tracking", "Skin of Rock"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, x));
    request(&mut e);
    let c = e.combat().expect("the combat goes on");
    assert_eq!(c.sides.each_ref().map(|s| s.minion), [a, x]);
    assert_eq!(e.state.seats[0].crypt.len(), 1, "no crypt card moved");
    assert!(minion_of(&e, 0, v_card).is_none());
}

/// "If the vampire playing it goes to torpor (eg. Outside the Hourglass),
/// the new combat still occurs" [ANK 20180104]; Outside the Hourglass's
/// damage "is applied even if the combat ends prematurely" [ANK 20180104]
/// [LSJ 19971110]: X's [TEM] damages A before range, A's Illusions (its 2
/// blood paid) ends the combat: A to torpor, the illusion fights X.
#[test]
fn illusions_combat_occurs_after_outside_the_hourglass_torpors_the_player() {
    let (mut e, a, x) = duel(&["chi"], &["tem", "tem", "for"]);
    e.state.minions[a.index()].blood = 3;
    let v_card = textless(|c| c.capacity >= 6 && no_stealth(c), 6);
    crypt(&mut e, 0, &[v_card]);
    hand(&mut e, 0, &["Illusions of the Kindred"]);
    hand(&mut e, 1, &["Outside the Hourglass", "Skin of Rock"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 3, x));
    pick_when(&mut e, play(0, 0, a));
    let mut seen = false;
    for _ in 0..40 {
        let req = request(&mut e);
        if let Some(v) = minion_of(&e, 0, v_card) {
            if e.combat()
                .is_some_and(|c| c.sides.iter().any(|s| s.minion == v))
            {
                seen = true;
                break;
            }
        }
        e.choose(fallback(&req));
    }
    assert!(seen, "the illusion's combat");
    assert!(e.state.minions[a.index()].in_torpor);
}

/// "If the illusionary vampire does not enter combat, then they are
/// removed from the game at the end of the current action" [LSJ 19991110]:
/// A's own Outside the Hourglass sends the empty X to torpor before range,
/// then Illusions: the illusion stays out of combat until the action ends.
#[test]
fn illusions_vampire_unfought_leaves_with_the_action() {
    let (mut e, a, x) = duel(&["chi", "tem", "tem"], &[]);
    e.state.minions[x.index()].blood = 1;
    let v_card = textless(|c| c.capacity >= 6 && no_stealth(c), 6);
    crypt(&mut e, 0, &[v_card]);
    hand(
        &mut e,
        0,
        &["Outside the Hourglass", "Illusions of the Kindred"],
    );
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 3, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
    assert!(e.state.seats[0].removed_crypt.contains(&v_card));
}

/// "If the illusionary vampire is burned in combat ..., they stay in the
/// ash heap" [PIB 20110810]: the illusion, emptied, sent down by X's hand
/// strike and burned by Decapitate [POT]: in the ash heap, not removed.
#[test]
fn illusions_vampire_burned_stays_in_the_ash_heap() {
    let (mut e, a, x) = duel(&["chi"], &["pot", "pot"]);
    let v_card = textless(|c| c.capacity >= 6 && no_stealth(c), 6);
    crypt(&mut e, 0, &[v_card]);
    hand(&mut e, 0, &["Illusions of the Kindred"]);
    hand(&mut e, 1, &["Decapitate"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    let v = minion_of(&e, 0, v_card).expect("the illusion");
    e.state.minions[v.index()].blood = 0;
    pick_when(&mut e, play(0, 2, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].ash_crypt.contains(&v_card));
    assert!(!e.state.seats[0].removed_crypt.contains(&v_card));
}

/// "Even if they have the same name as a vampire in play, the illusionary
/// vampire is considered different for all purposes" [LSJ 20100213] [LSJ
/// 19991110]; "can use the votes ... even if there is another instance of
/// the title in play" [LSJ 20030419]: a titled vampire's copy as the
/// illusion: neither contests the card nor the title; and Jimmy Dunn's copy
/// already in play is not burned by an illusory Jimmy.
#[test]
fn illusions_vampire_does_not_contest_its_copy_or_title() {
    let titled = fixture_vampire(|c| c.title != Title::None && c.capacity >= 6, 0);
    let jimmy = zvtes::state::find_card("Jimmy Dunn");
    for card in [titled, jimmy] {
        let (mut e, a, x) = duel(&["chi"], &["for"]);
        let other = e.add_minion(SeatId(1), card, 3, false);
        crypt(&mut e, 0, &[card]);
        hand(&mut e, 0, &["Illusions of the Kindred"]);
        hand(&mut e, 1, &["Skin of Rock"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, 0, a));
        request(&mut e);
        let v = minion_of(&e, 0, card).expect("the illusion");
        for m in [v, other] {
            let w = &e.state.minions[m.index()];
            assert!(
                w.in_play && !w.contested && !w.title_contested,
                "{}",
                w.name()
            );
        }
    }
}

/// "If the card drawn is an imbued, the imbued would briefly come into
/// play, become incapacitated, and then be removed" [LSJ 20060323].
#[test]
fn illusions_imbued_is_removed_from_the_game() {
    let (mut e, a, x) = duel(&["chi"], &[]);
    let imbued = CRYPT.iter().position(|c| c.imbued).expect("an imbued") as u16;
    crypt(&mut e, 0, &[imbued]);
    hand(&mut e, 0, &["Illusions of the Kindred"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].removed_crypt.contains(&imbued));
}

/// Imposing Phantasm [chi]: "+1 strength this combat. After combat ends, if
/// the opposing minion is ready, the opposing minion gains blood or life
/// equal to the amount they lost to damage this combat": X loses 2 to A's
/// strength-2 hand strike and gets them back; "The opposing minion get
/// their blood back even if they changed controller" [ANK 20220525]: X
/// passed to seat 0 meanwhile, still paid.
#[test]
fn imposing_phantasm_gives_back_what_the_opponent_lost_whoever_controls_it() {
    for moved in [false, true] {
        let (mut e, a, x) = duel(&["chi"], &[]);
        hand(&mut e, 0, &["Imposing Phantasm"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, 0, a));
        if moved {
            e.state.minions[x.index()].owner = SeatId(0);
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, x), 6 - 2 + 2, "moved: {moved}");
        assert_eq!(blood(&e, a), 6 - 1 - 1, "its cost, X's hand strike");
    }
}

/// Imposing Phantasm [CHI]: "As above, with 1 additional strike (limited)
/// this round": A's strength-2 hand strike lands twice in round 1.
#[test]
fn imposing_phantasm_superior_strikes_twice() {
    let (mut e, a, x) = duel(&["chi", "chi"], &["for"]);
    hand(&mut e, 0, &["Imposing Phantasm"]);
    hand(&mut e, 1, &["Skin of Rock", "Skin of Rock"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    let req = offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (2, 0));
    e.choose(index_of(&req, Choice::Pass));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (2, 0), "the additional strike");
}

/// King of the Mountain [for]: "Prevent all damage from the opposing
/// minion's strike": A's hand strike prevented.
#[test]
fn king_of_the_mountain_prevents_a_strike() {
    let (mut e, a, x) = duel(&[], &["for"]);
    hand(&mut e, 1, &["King of the Mountain"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 6 - 2, "its cost only");
}

/// King of the Mountain [FOR]: "If the opposing minion inflicts damage
/// with a hand strike this round (even if the damage is prevented), he or
/// she takes an equal amount of damage during strike resolution as well";
/// "Damage dealt to the opponent is environmental" [RTR 19970630] [LSJ
/// 19970801]: X prevents A's hand strike with Skin of Rock, A takes 1 back
/// beside X's own hand strike.
#[test]
fn king_of_the_mountain_superior_reflects_hand_damage_prevented_or_not() {
    let (mut e, a, x) = duel(&[], &["for", "for"]);
    hand(&mut e, 1, &["King of the Mountain", "Skin of Rock"]);
    fight(&mut e, a, x);
    let req = offering(&mut e, play(0, 1, x));
    assert_eq!(req.window, WindowLabel::CombatBeforeStrikes);
    e.choose(index_of(&req, play(0, 1, x)));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 6 - 1 - 1, "X's hand strike and its own back");
    assert_eq!(blood(&e, x), 6 - 2, "the card's cost, A's damage prevented");
}

/// King of the Mountain [FOR], two copies in one round: each reflects (no
/// printed limit; copies add, rules-qa 126.41, LSJ 19970707): A takes X's
/// hand strike and its own 1 twice.
#[test]
fn king_of_the_mountain_superior_two_copies_reflect_twice() {
    for copies in [1usize, 2] {
        let (mut e, a, x) = duel(&[], &["for", "for"]);
        hand(&mut e, 1, &vec!["King of the Mountain"; copies]);
        fight(&mut e, a, x);
        for _ in 0..copies {
            pick_when(&mut e, play(0, 1, x));
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, a), 6 - 1 - copies as u8, "{copies}");
    }
}

/// King of the Mountain [FOR]: a hand strike its vampire is immune to is
/// still inflicted (rules-qa 116.44, 116.138) and reflected: Carolina Vález
/// (immune to allies' damage) bleeds, the 2-strength Aggressive Corpse
/// blocks; her King of the Mountain deals its 2 back, with her hand strike
/// 3 of its 3 life: burned; without the card, 2 life left.
#[test]
fn king_of_the_mountain_superior_reflects_a_hand_strike_its_vampire_is_immune_to() {
    for king in [false, true] {
        let mut e = game(2);
        let c = e.add_minion(SeatId(0), find_card("Carolina Vález"), 6, false);
        e.state.minions[c.index()].gained_levels = vec!["for", "for"];
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
        let ally = e.add_ally(SeatId(1), find_library("Aggressive Corpse"));
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 6, false);
        if king {
            hand(&mut e, 0, &["King of the Mountain"]);
        }
        pick(&mut e, bleed(c));
        pick_when(&mut e, block(ally));
        if king {
            pick_when(&mut e, play(0, 1, c));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            blood(&e, c),
            if king { 6 - 2 } else { 6 },
            "immune; the card's cost"
        );
        assert_eq!(e.state.minions[ally.index()].in_play, !king, "{king}");
        if !king {
            assert_eq!(blood(&e, ally), 2);
        }
    }
}

/// Putrefaction [thn]: "Strike: 1 damage. Put this card on the opposing
/// minion. The minion with this card has -1 stealth": the card itself on X,
/// out of the ash heap, X at -1 stealth.
#[test]
fn putrefaction_inferior_puts_itself_on_the_opponent() {
    let (mut e, a, x) = duel(&["thn"], &[]);
    hand(&mut e, 0, &["Putrefaction"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(placed(&e, "Putrefaction", x).len(), 1);
    assert!(!ash_has(&e, 0, "Putrefaction"));
    assert_eq!(e.state.bearer_always(x, zvtes::cards::Stat::Stealth), -1);
    assert_eq!(blood(&e, x), 6 - 1);
}

/// Putrefaction: "may choose not to unlock as normal and burn this card
/// during his or her unlock phase": "It can still be burned if the vampire
/// is already prevented from unlocking as normal by another effect" [RTR
/// 20070707]: X, held from unlocking, offered the burn; burned, X stays
/// locked.
#[test]
fn putrefaction_burns_by_forgoing_unlock_even_when_held() {
    let (mut e, a, x) = duel(&[], &[]);
    let rot = in_play_mode(&mut e, 0, "Putrefaction", Some(x), 1);
    e.state.cards[rot.index()].controller = SeatId(1);
    let _ = a;
    e.state.minions[x.index()].locked = true;
    e.state.minions[x.index()].skip_unlock = true;
    let burn = Choice::BurnInPlay { card: rot.0 };
    let req = offering(&mut e, burn);
    e.choose(index_of(&req, burn));
    request(&mut e);
    assert!(!e.state.cards[rot.index()].in_play);
    assert!(e.state.minions[x.index()].locked, "held still");
}

/// Putrefaction [THN]: "this minion takes 1 unpreventable damage when he or
/// she strikes in combat or takes an action"; "Damage is environmental"
/// [RTR 19970630]: X bleeds, 1 damage; blocked, strikes, 1 more.
#[test]
fn putrefaction_superior_damages_its_bearer_striking_and_acting() {
    let (mut e, a, x) = duel(&[], &[]);
    in_play_mode(&mut e, 1, "Putrefaction", Some(x), 2);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let before = blood(&e, x);
    pick(&mut e, bleed(x));
    pick_when(&mut e, block(a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, x),
        before - 1 - 1 - 1,
        "acting, striking, A's hand"
    );
}

/// Putrefaction [THN]: "The minion with this card may choose not to strike";
/// "You can't use the option to not strike, if you are commited to a strike
/// from another effect (eg. .44 Magnum)" [LSJ 20090528-2-2]: X offered to
/// pass its strike; committed by its gun's maneuver, its strike elides and
/// hits.
#[test]
fn putrefaction_superior_bearer_may_decline_to_strike_unless_committed() {
    let (mut e, a, x) = duel(&[], &[]);
    in_play_mode(&mut e, 1, "Putrefaction", Some(x), 2);
    fight(&mut e, a, x);
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(req.options.contains(&Choice::Pass), "{:?}", req.options);

    let (mut e, a, x) = duel(&[], &[]);
    in_play_mode(&mut e, 1, "Putrefaction", Some(x), 2);
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(x));
    fight(&mut e, a, x);
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(gun),
            ability: 1,
            actor: None,
            target: Target::None,
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 6 - 2, "the committed gun strike");
}

/// Save Face: "Only usable by an unlocked vampire not involved in the
/// current combat ... Choose a younger vampire of the same clan who is
/// involved in the combat. The chosen vampire gets a mandatory press, only
/// usable to continue combat. If the chosen combatant is still ready after
/// the combat ends, he or she gains 1 blood": "Can be used by a vampire in
/// torpor" [LSJ 20020416]; "by a minion of a Methuselah not involved"
/// [ANK 20181101]; "applied after combat ends" [LSJ 20071116]. Seat 2's
/// torpid elder plays it for A: A presses on (its only option, elided),
/// round 2 fought, A +1 blood after.
#[test]
fn save_face_forces_a_continue_press_and_pays_a_blood() {
    let mut e = game(3);
    let young = |c: &zvtes::crypt::CryptCard| {
        c.clan == "Brujah" && (4..=6).contains(&c.capacity) && no_stealth(c)
    };
    let old = |c: &zvtes::crypt::CryptCard| c.clan == "Brujah" && c.capacity >= 8;
    let a = e.add_minion(SeatId(0), textless(young, 0), 4, false);
    e.add_minion(SeatId(0), textless(young, 1), 4, false);
    let x = e.add_minion(
        SeatId(1),
        textless(|c| c.capacity >= 4 && no_stealth(c), 0),
        6,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| c.capacity >= 4 && no_stealth(c), 1),
        4,
        false,
    );
    let s = e.add_minion(SeatId(2), textless(old, 0), 2, true);
    e.add_minion(SeatId(2), textless(|c| c.capacity >= 4, 2), 4, false);
    hand(&mut e, 2, &["Save Face"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    let face = play_on(0, 0, s, Target::Minion(a));
    let req = offering(&mut e, face);
    assert_eq!(req.window, WindowLabel::CombatPress);
    e.choose(index_of(&req, face));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 6 - 2, "a second round: the press used");
    assert_eq!(
        blood(&e, a),
        4 - 1 - 1 + 1,
        "two rounds of X's hand, +1 after"
    );
}

/// Shadow Parasite: "Only usable at long range. [obt] Strike: 3R damage."
#[test]
fn shadow_parasite_strikes_only_at_long_range() {
    let (mut e, a, x) = duel(&["obt"], &[]);
    hand(&mut e, 0, &["Shadow Parasite"]);
    fight(&mut e, a, x);
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        is_play_of(o, 0, a)
    });

    let (mut e, a, x) = duel(&["obt"], &[]);
    hand(&mut e, 0, &["Fake Out", "Shadow Parasite"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 6 - 3);
}

/// Soul Stealing [nec] on a diablerie: "can be played before or after
/// getting a Discipline card (if any), but must be played before the Blood
/// Hunt" [RTR 19991206]: A diablerizes its prey's torpid X, then gains half
/// X's capacity.
#[test]
fn soul_stealing_on_a_diablerie_of_the_preys_vampire() {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (5..=8).contains(&c.capacity);
    e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 8, 1), 4, false);
    let x_card = textless(mid, 1);
    let x = e.add_minion(SeatId(1), x_card, 2, true);
    e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["nec"];
    hand(&mut e, 0, &["Soul Stealing"]);
    pick(&mut e, diablerize(a, x));
    let req = offering(&mut e, play(0, 0, a));
    e.choose(index_of(&req, play(0, 0, a)));
    request(&mut e);
    assert_eq!(blood(&e, a), 4 + 2 + CRYPT[x_card as usize].capacity / 2);
}

/// Soul Stealing [NEC] in combat: "If a minion is burned in combat, his
/// opponent his always considered to have burned him" [LSJ 20090922]: A's
/// Decapitate burns its prey's X; at the round's end, half X's capacity.
#[test]
fn soul_stealing_in_combat_after_burning_the_preys_vampire() {
    let (mut e, a, x) = duel(&["pot", "pot", "nec", "nec"], &[]);
    let cap = e.state.capacity_by(x, SeatId(0)).expect("a vampire");
    e.state.minions[x.index()].blood = 0;
    hand(&mut e, 0, &["Decapitate", "Soul Stealing"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    let req = offering(&mut e, play(0, 2, a));
    assert_eq!(req.window, WindowLabel::CombatEndOfRound);
    let before = blood(&e, a);
    e.choose(index_of(&req, play(0, 2, a)));
    request(&mut e);
    assert_eq!(blood(&e, a), before + cap / 2);
}

/// Tongue of the Serpent [SER]: "Strike: steal 2 blood"; "Can target a
/// minion with less blood than the amount stolen" [RTR 20010711]: X with
/// 1 blood gives what it has.
#[test]
fn tongue_of_the_serpent_steals_from_a_minion_with_less_blood() {
    let (mut e, a, x) = duel(&["ser", "ser"], &[]);
    e.state.minions[x.index()].blood = 1;
    e.state.minions[a.index()].blood = 3;
    hand(&mut e, 0, &["Tongue of the Serpent"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    request(&mut e);
    assert_eq!(blood(&e, a), 3 + 1 - 1, "X's 1, X's hand strike");
}

/// Torment the Soul: "Not usable during the first round of combat. [NEC]
/// Strike: 1R aggravated damage."
#[test]
fn torment_the_soul_not_in_the_first_round() {
    let (mut e, a, x) = duel(&["nec", "nec", "pot"], &["for"]);
    hand(&mut e, 0, &["Torment the Soul", "Relentless Pursuit"]);
    hand(&mut e, 1, &["Skin of Rock"]);
    fight(&mut e, a, x);
    let torment = |o: &Choice| is_play_of(o, 0, a);
    never_offered_where(&mut e, SeatId(0), WindowLabel::CombatPress, torment);
    to_round_two(&mut e, a, 1);
    pick_when(&mut e, play(0, 1, a));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (0, 1));
}

/// Tortured Confession: "Only usable when the opposing vampire goes into
/// torpor. Not usable by a vampire being burned or going to torpor. Combat
/// ends. You may look at the opposing vampire's controller's hand": X sent
/// down by A's hand strike, A looks at seat 1's hand, the torpor lands, the
/// combat over. "Not usable by vampires burned in combat" [RTR 19950622]:
/// A going down too, not offered.
#[test]
fn tortured_confession_looks_at_the_hand_and_torpor_lands() {
    let (mut e, a, x) = duel(&[], &[]);
    e.state.minions[x.index()].blood = 0;
    hand(&mut e, 0, &["Tortured Confession"]);
    hand(&mut e, 1, &["Skin of Rock"]);
    e.notes = Some(Vec::new());
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
    assert!(noted_looks(&mut e)
        .iter()
        .any(|l| l.viewer == SeatId(0) && l.seat == SeatId(1)));

    let (mut e, a, x) = duel(&[], &[]);
    e.state.minions[x.index()].blood = 0;
    hand(&mut e, 0, &["Tortured Confession"]);
    fight(&mut e, a, x);
    e.state.minions[a.index()].blood = 0;
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, a));
}

/// Vascular Explosion [qui]: "Strike: 2 unpreventable damage. If the
/// opposing minion is an ally, this strike inflicts +1 unpreventable
/// damage": 3 life off an ally, its prevention never offered.
#[test]
fn vascular_explosion_hits_an_ally_harder_unpreventably() {
    let (mut e, a, _) = duel(&["qui"], &[]);
    let ally = e.add_ally(SeatId(1), find_library("Carlton Van Wyk"));
    e.state.minions[ally.index()].blood = 4;
    hand(&mut e, 0, &["Vascular Explosion"]);
    fight(&mut e, a, ally);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ally), 4 - 3);
}

/// Wave of Lethargy [qui]: "Cards providing maneuvers and/or presses cost
/// the opposing vampire 1 additional blood to play this round": X's Fake
/// Out costs 1; [QUI] a strike card providing a maneuver too, Aid from
/// Bats, +1 once ("If an opponent then plays an effect that grants a
/// strike, they pay the additional cost" [LSJ 20090529]; "Strikes are paid
/// at declaration" [LSJ 20090703]).
#[test]
fn wave_of_lethargy_taxes_maneuver_and_strike_cards_once() {
    let (mut e, a, x) = duel(&["qui"], &[]);
    hand(&mut e, 0, &["Wave of Lethargy"]);
    hand(&mut e, 1, &["Fake Out"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, x),
        6 - 1,
        "Fake Out's 1, A's hand strike at long range none"
    );

    let (mut e, a, x) = duel(&["qui", "qui"], &["ani"]);
    hand(&mut e, 0, &["Wave of Lethargy"]);
    hand(&mut e, 1, &["Aid from Bats"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 6 - 1, "Aid from Bats: +1, not +2");
}

/// "The effect is resolved fully (including the blood gain) before
/// resolving other effects tied to the illusionary vampire entering play
/// (eg. Erlik's ability)" [ANK 20240724]: Erlik, the illusion, enters with
/// half his capacity and gives 2 of it to the Harbinger in seat 0's
/// uncontrolled region as he enters.
#[test]
fn illusions_vampire_gains_blood_before_erlik_enters() {
    let (mut e, a, x) = duel(&["chi"], &["for"]);
    let erlik = zvtes::state::find_card("Erlik");
    crypt(&mut e, 0, &[erlik]);
    e.state.seats[0].uncontrolled = vec![zvtes::state::UncontrolledSlot {
        card: zvtes::state::find_card("Nicomedes"),
        copy: Default::default(),
        counters: 0,
        banished: None,
    }];
    hand(&mut e, 0, &["Illusions of the Kindred"]);
    hand(&mut e, 1, &["Skin of Rock"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::EntryEffect);
    e.choose(index_of(&req, Choice::Slot(0)));
    pick(&mut e, Choice::Slot(0));
    pick(&mut e, Choice::Pass);
    let half = CRYPT[erlik as usize].capacity / 2;
    let m = minion_of(&e, 0, erlik).expect("Erlik, the illusion");
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 2);
    assert_eq!(blood(&e, m), half - 2);
}

// ---- Modifiers and reactions (modifier_reaction) --------------------------------

/// The `nth` untitled fixture vampire of `sect`, capacity 4 to 8, no
/// printed stealth.
#[track_caller]
fn of_sect(sect: zvtes::crypt::Sect, nth: usize) -> u16 {
    textless(
        |c| c.sect == Some(sect) && (4..=8).contains(&c.capacity) && no_stealth(c),
        nth,
    )
}

/// Banner of Neutrality: "Requires an Independent or Anarch vampire. Only
/// usable if a Camarilla or Sabbat vampire is bleeding you. Reduce the
/// bleed amount by 1": seat 1's Anarch against seat 0's Camarilla bleed of
/// 1: no pool lost; against an Anarch's bleed, not offered.
#[test]
fn banner_of_neutrality_reduces_a_camarilla_vampires_bleed() {
    use zvtes::crypt::Sect;
    for (acting, offered) in [(Sect::Camarilla, true), (Sect::Anarch, false)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_sect(acting, 0), 4, false);
        e.add_minion(SeatId(0), of_sect(acting, 1), 4, false);
        let r = e.add_minion(SeatId(1), of_sect(Sect::Anarch, 2), 4, false);
        e.add_minion(SeatId(1), of_sect(Sect::Anarch, 3), 4, false);
        hand(&mut e, 1, &["Banner of Neutrality"]);
        pick(&mut e, bleed(a));
        if offered {
            pick_when(&mut e, play(0, 0, r));
            until(&mut e, SeatId(1), WindowLabel::MinionPhase);
            assert_eq!(e.state.seats[1].pool, 10);
        } else {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, r));
            assert_eq!(e.state.seats[1].pool, 10 - 1);
        }
    }
}

/// Venetian Conference: "Only usable if a Camarilla vampire is acting. +2
/// intercept" (requires a Giovanni): the Camarilla A's +1 stealth action
/// (Draba) at stealth 2, X blocks with its +2; an Independent acting, never
/// offered.
#[test]
fn venetian_conference_gives_two_intercept_against_a_camarilla_vampire() {
    use zvtes::crypt::Sect;
    for sect in [Sect::Camarilla, Sect::Independent] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_sect(sect, 0), 4, false);
        e.add_minion(SeatId(0), of_sect(sect, 1), 4, false);
        let giovanni = |c: &zvtes::crypt::CryptCard| {
            c.clan == "Giovanni" && (4..=8).contains(&c.capacity) && no_stealth(c)
        };
        let x = e.add_minion(SeatId(1), textless(giovanni, 0), 4, false);
        e.add_minion(SeatId(1), textless(giovanni, 1), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["chi"];
        // Stealth 2 with Draba's: +1 intercept would not reach it.
        e.state.minions[a.index()].timed.push((
            zvtes::cards::Lasting::Stealth(1),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(0),
        ));
        hand(&mut e, 0, &["Draba"]);
        hand(&mut e, 1, &["Venetian Conference"]);
        pick(&mut e, play_on(0, 0, a, Target::Minion(x)));
        pick_when(&mut e, block(x));
        if sect == Sect::Independent {
            never_offered_where(
                &mut e,
                SeatId(1),
                WindowLabel::MinionPhase,
                |o| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == x),
            );
            continue;
        }
        let req = offering(&mut e, play(0, 0, x));
        e.choose(index_of(&req, play(0, 0, x)));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(
            e.last_outcome,
            Some(zvtes::actions::Outcome::Blocked),
            "2 against 2"
        );
    }
}

/// Clan Loyalty: "Only usable when this vampire is successfully blocked by a
/// vampire of the same clan ... Cancel the block and combat. The action
/// continues as normal, and no vampires of that clan may block the acting
/// vampire for the remainder of the turn"; "The blocking minion is not
/// locked" [ANK 20180321]: X unlocked, no combat, the action open again; X's
/// other Brujah cannot block.
#[test]
fn clan_loyalty_cancels_a_same_clan_block_and_bars_the_clan() {
    let mut e = game(2);
    let brujah = |c: &zvtes::crypt::CryptCard| {
        c.clan == "Brujah" && (4..=8).contains(&c.capacity) && no_stealth(c)
    };
    let a = e.add_minion(SeatId(0), textless(brujah, 0), 4, false);
    e.add_minion(SeatId(0), textless(brujah, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(brujah, 2), 4, false);
    let y = e.add_minion(SeatId(1), textless(brujah, 3), 4, false);
    hand(&mut e, 0, &["Clan Loyalty"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.window, WindowLabel::ActionBlocked);
    e.choose(index_of(&req, play(0, 0, a)));
    request(&mut e);
    assert!(e.combat().is_none());
    assert!(!e.state.minions[x.index()].locked, "not locked");
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, block(y));
    assert_eq!(e.state.seats[1].pool, 10 - 1, "the bleed lands");
}

/// Draba [chi]: "+1 stealth action. Ⓓ Put this card on a ready minion; you
/// still control this card": the card itself on seat 1's X, seat 0's.
#[test]
fn draba_action_puts_the_card_on_a_minion_and_its_player_keeps_it() {
    let (mut e, a, x) = duel(&["chi"], &[]);
    hand(&mut e, 0, &["Draba"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(x)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = placed(&e, "Draba", x);
    assert_eq!(card.len(), 1);
    assert!(!ash_has(&e, 0, "Draba"));
    assert_eq!(e.state.card_controller(card[0]), SeatId(0));
}

/// Draba: "Burn this card to reduce the attached minion's stealth to 0";
/// "You must gain the impulse to be able to use the reaction, or use the
/// effect" [ANK 20200607]: seat 2's Draba on A; A's +1 stealth action at
/// seat 1: seat 2 burns it, stealth 0.
#[test]
fn draba_burned_by_a_third_party_strips_the_attached_minions_stealth() {
    let mut e = game(3);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity) && no_stealth(c);
    let a = e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    e.add_minion(SeatId(0), textless(mid, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.add_minion(SeatId(1), textless(mid, 3), 4, false);
    e.add_minion(SeatId(2), textless(mid, 4), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["chi"];
    let draba = in_play(&mut e, 2, "Draba", Some(a));
    e.state.cards[draba.index()].controller = SeatId(2);
    hand(&mut e, 0, &["Draba"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(x)));
    let burn = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(draba),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    let req = offering(&mut e, burn);
    assert_eq!(req.seat, SeatId(2));
    e.choose(index_of(&req, burn));
    let ac = e.action().expect("the action");
    assert_eq!(ac.stealth(&e.state), 0);
    assert!(!e.state.cards[draba.index()].in_play);
}

/// Extortion [ser]: "Put this card on the acting minion. The attached
/// minion burns 1 additional blood to unlock during their unlock phase";
/// "Is played after resolution, after all combats" [PIB 20150915] [LSJ
/// 19981028] [ANK 20190425]: X
/// blocks A and plays it after the combat; at seat 0's next unlock, A
/// burns 1 to unlock.
#[test]
fn extortion_goes_on_the_acting_minion_and_taxes_its_unlock() {
    let (mut e, a, x) = duel(&[], &["ser"]);
    hand(&mut e, 1, &["Extortion"]);
    fight(&mut e, a, x);
    let req = offering(&mut e, play(0, 0, x));
    assert_eq!(req.window, WindowLabel::ActionAfterCombat);
    e.choose(index_of(&req, play(0, 0, x)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(placed(&e, "Extortion", a).len(), 1);
    assert!(!ash_has(&e, 1, "Extortion"));
    let before = blood(&e, a);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked);
    assert_eq!(blood(&e, a), before - 1);
}

/// Extortion [ser] on an empty vampire: no blood to unlock, it stays locked
/// (LSJ 20020604, LSJ 20011004); the Edge burns every Extortion its
/// player has (LSJ 20010716).
#[test]
fn extortion_holds_an_empty_vampire_and_the_edge_burns_them_all() {
    let (mut e, a, _) = duel(&[], &[]);
    let one = in_play(&mut e, 1, "Extortion", Some(a));
    let two = in_play(&mut e, 1, "Extortion", Some(a));
    for c in [one, two] {
        e.state.cards[c.index()].controller = SeatId(0);
    }
    e.state.minions[a.index()].blood = 0;
    e.state.minions[a.index()].locked = true;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].locked, "no blood to unlock");
    e.state.seats[0].has_edge = true;
    let burn = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(one),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    let req = request(&mut e);
    assert!(req.options.contains(&burn), "{:?}", req.options);
    e.choose(index_of(&req, burn));
    request(&mut e);
    assert!(placed(&e, "Extortion", a).is_empty(), "both burned");
    assert!(!e.state.seats[0].has_edge);
}

/// Extortion [SER]: "Put this card in play under the control of the acting
/// minion's controller. The controller of this card burns 1 pool during
/// their unlock phase."
#[test]
fn extortion_superior_burns_its_new_controllers_pool_each_unlock() {
    let (mut e, a, x) = duel(&[], &["ser", "ser"]);
    hand(&mut e, 1, &["Extortion"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = (0..e.state.cards.len())
        .map(|i| CardId(i as u32))
        .find(|&c| {
            e.state.cards[c.index()].in_play
                && e.state.cards[c.index()].card == find_library("Extortion")
        })
        .expect("in play");
    assert_eq!(e.state.card_controller(card), SeatId(0));
    let pool = e.state.seats[0].pool;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, pool - 1);
}

/// Iron Heart [pot][pre]: "Cancel an action modifier that requires Presence
/// [pre] as it is played ... No cost is paid"; "Cannot be used to cancel a
/// card that could be played with [pre] (or [dom]) but has not been (eg.
/// Virtuosa)" [PIB 20130704]: Virtuosa [pre] canceled; played with [mel],
/// not.
#[test]
fn iron_heart_cancels_only_a_modifier_played_with_presence() {
    for (levels, offered) in [(vec!["pre"], true), (vec!["mel"], false)] {
        let (mut e, a, x) = duel(&[], &["pot", "pre"]);
        e.state.minions[a.index()].gained_levels = levels.clone();
        hand(&mut e, 0, &["Virtuosa"]);
        hand(&mut e, 1, &["Iron Heart"]);
        pick(&mut e, bleed(a));
        if offered {
            pick_when(&mut e, block(x));
        }
        play_when(&mut e, 0, a);
        let iron = play(0, 0, x);
        if offered {
            let req = offering(&mut e, iron);
            assert_eq!(req.window, WindowLabel::CardAsPlayed);
            e.choose(index_of(&req, iron));
            request(&mut e);
            assert!(ash_has(&e, 0, "Virtuosa"));
        } else {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, iron);
        }
    }
}

/// Iron Heart: "an action modifier": a dual action modifier / reaction
/// card is one only when the acting minion plays it (rules-digest D18):
/// Hall of Hades' Court [tem] played as a reaction by the blocker's side is
/// no action modifier, not canceled.
#[test]
fn iron_heart_does_not_cancel_a_dual_card_played_as_a_reaction() {
    let mut e = Engine::bare(2);
    let _ = &mut e;
    let lib = zvtes::library::LIBRARY
        .iter()
        .position(|c| c.name == "Madrigal")
        .expect("Madrigal");
    let types = zvtes::library::LIBRARY[lib].types;
    assert!(types.len() > 1);
    let st = &e.state;
    assert_eq!(
        st.played_as(None, lib as u16, 0),
        Some(zvtes::library::CardType::Reaction),
        "by a minion not acting"
    );
}

/// Malleable Visage [VIC]: "Only usable by a ready unlocked vampire other
/// than the acting minion ... Instead of resolving the block, unlock the
/// acting minion, and this vampire and the blocking minion lock and enter
/// combat (the action is still blocked)"; "Does not change the acting
/// minion" [LSJ 20011023] [RTR 20020501]; "Requires a ready, unlocked
/// vampire" [RTR 20010711]: B, unlocked, fights X, A unlocked; B locked:
/// not offered.
#[test]
fn malleable_visage_takes_the_blocked_vampires_combat() {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity) && no_stealth(c);
    let a = e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    let b = e.add_minion(SeatId(0), textless(mid, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.add_minion(SeatId(1), textless(mid, 3), 4, false);
    e.state.minions[b.index()].gained_levels = vec!["vic", "vic"];
    e.state.minions[x.index()].gained_levels = vec!["for"];
    hand(&mut e, 0, &["Malleable Visage"]);
    hand(&mut e, 1, &["Skin of Rock"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    let visage = play(0, 1, b);
    let req = offering(&mut e, visage);
    assert_eq!(req.window, WindowLabel::ActionBlocked);
    e.choose(index_of(&req, visage));
    request(&mut e);
    let c = e.combat().expect("B's combat");
    assert!(c.sides.iter().any(|s| s.minion == b) && c.sides.iter().any(|s| s.minion == x));
    assert!(!e.state.minions[a.index()].locked, "A unlocked");
    assert_eq!(e.action().expect("the action").acting, a);

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    let b = e.add_minion(SeatId(0), textless(mid, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.add_minion(SeatId(1), textless(mid, 3), 4, false);
    e.state.minions[b.index()].gained_levels = vec!["vic", "vic"];
    e.state.minions[b.index()].locked = true;
    hand(&mut e, 0, &["Malleable Visage"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 1, b));
}

/// Red Herring [chi]: "Unlock this vampire, and the action ends
/// (unsuccessfully). If an action card was used to perform the action,
/// return it to its owner's hand ... Vampires you control cannot perform
/// the same action again this turn"; reached resolution, NRA [LSJ 20070709]
/// [ANK 20221011-2]; "Actions without card (provided by the rulebook) are
/// not the same as actions provided by cards" [LSJ 20060522] [LSJ
/// 20060824] [LSJ 20080725] [LSJ 20090617]; ended at once, nothing more
/// played [RTR 20180719] [ANK 20200207]: A's
/// Intimidation blocked, Red Herring: A unlocked, the card back in hand,
/// neither A nor B offered Intimidation again, B may still bleed.
#[test]
fn red_herring_ends_the_blocked_action_and_bars_the_card_for_own_vampires() {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity) && no_stealth(c);
    let a = e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    let b = e.add_minion(SeatId(0), textless(mid, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.add_minion(SeatId(1), textless(mid, 3), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["chi", "pre"];
    e.state.minions[b.index()].gained_levels = vec!["pre"];
    hand(&mut e, 0, &["Intimidation", "Red Herring"]);
    let p = first_play(&mut e, 0, a);
    pick(&mut e, p);
    pick_when(&mut e, block(x));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.window, WindowLabel::ActionBlocked);
    e.choose(index_of(&req, play(0, 0, a)));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::MinionPhase)
    );
    assert!(!e.state.minions[a.index()].locked);
    assert!(e.state.seats[0]
        .hand
        .contains(&find_library("Intimidation")));
    assert!(
        !req.options.iter().any(|o| is_play_of(o, 1, a)
            || is_play_of(o, 1, b)
            || is_play_of(o, 0, a)
            || is_play_of(o, 0, b)),
        "{:?}",
        req.options
    );
    assert!(req.options.contains(&bleed(b)));
}

/// Set's Call [ser]: "This action is successful (and the cost is paid), but
/// this vampire steals the retainer and combat does not occur";
/// "Requirements do not apply when taking control of a card already in
/// play" [TOM 19960226-1]: A employs a Ghoul Retainer (+1 stealth), X
/// blocks with Eagle's Sight and plays it: the retainer on X, no combat.
#[test]
fn sets_call_steals_an_employed_retainer() {
    let (mut e, a, x) = duel(&[], &["ser", "aus"]);
    hand(&mut e, 0, &["Ghoul Retainer"]);
    hand(&mut e, 1, &["Set's Call", "Eagle's Sight"]);
    let p = first_play(&mut e, 0, a);
    pick(&mut e, p);
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(1, 0, x));
    let req = offering(&mut e, play(0, 0, x));
    assert_eq!(req.window, WindowLabel::ActionBlocked);
    e.choose(index_of(&req, play(0, 0, x)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(placed(&e, "Ghoul Retainer", x).len(), 1);
    assert!(placed(&e, "Ghoul Retainer", a).is_empty());
    assert_eq!(blood(&e, x), 6, "no combat");
}

/// Set's Call [SER] on Cry Wolf: the blocker locks as a blocker does
/// (rulebook), and the action is still blocked for a reader of one [LSJ
/// 20090901] (Cry Wolf: "If the action is blocked, the acting vampire goes
/// to torpor"): X steals the werewolf and is locked; A goes to torpor.
#[test]
fn sets_call_blocker_locks_and_cry_wolf_still_sees_a_block() {
    let (mut e, a, x) = duel(&[], &["ser", "ser", "aus"]);
    e.state.minions[a.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    hand(&mut e, 0, &["Cry Wolf"]);
    hand(&mut e, 1, &["Set's Call", "Eagle's Sight"]);
    let p = first_play(&mut e, 0, a);
    pick(&mut e, p);
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(1, 0, x));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let wolf = find_library("Cry Wolf");
    assert!(
        e.state
            .minions
            .iter()
            .any(|v| v.in_play && v.owner == SeatId(1) && v.kind.ally_card() == Some(wolf)),
        "stolen"
    );
    assert!(e.state.minions[x.index()].locked, "the blocker locked");
    assert!(
        e.state.minions[a.index()].in_torpor,
        "blocked, for Cry Wolf"
    );
}

/// Set's Call [SER]: "this vampire steals the ally"; "The acting vampire's
/// controller is the one who makes the choices pertaining to the ally
/// entry" [LSJ 20030701-2]: A recruits Carlton Van Wyk, X steals it: the
/// ally X's Methuselah's.
#[test]
fn sets_call_superior_steals_the_recruited_ally() {
    let (mut e, a, x) = duel(&[], &["ser", "ser", "aus"]);
    hand(&mut e, 0, &["Carlton Van Wyk"]);
    hand(&mut e, 1, &["Set's Call", "Eagle's Sight"]);
    let p = first_play(&mut e, 0, a);
    pick(&mut e, p);
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(1, 0, x));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let carlton = find_library("Carlton Van Wyk");
    let ally = (0..e.state.minions.len())
        .map(|i| MinionId(i as u32))
        .find(|&m| {
            e.state.minions[m.index()].in_play
                && e.state.minions[m.index()].kind.ally_card() == Some(carlton)
        })
        .expect("the ally in play");
    assert_eq!(e.state.minions[ally.index()].owner, SeatId(1));
}

/// Shilmulo Deception [chi]: "This action is blocked with this vampire as
/// the blocking minion instead": seat 1's Y takes X's block, locks, fights
/// A; X stays locked.
#[test]
fn shilmulo_deception_makes_this_vampire_the_blocker() {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity) && no_stealth(c);
    let a = e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    e.add_minion(SeatId(0), textless(mid, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    let y = e.add_minion(SeatId(1), textless(mid, 3), 4, false);
    e.state.minions[y.index()].gained_levels = vec!["chi", "for"];
    hand(&mut e, 1, &["Shilmulo Deception", "Skin of Rock"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, y));
    request(&mut e);
    let c = e.combat().expect("the combat");
    assert!(c.sides.iter().any(|s| s.minion == y) && !c.sides.iter().any(|s| s.minion == x));
    assert!(e.state.minions[y.index()].locked);
    assert!(
        e.state.minions[x.index()].locked,
        "the first blocker stays locked"
    );
}

/// Shroud Mastery [NEC]: "Only usable as an action to recruit or employ a
/// wraith is announced. Unlock this vampire after action resolution if this
/// action is successful"; "played before regular modifiers" [ANK
/// 20190425]; "Replace the action card before playing it" [ANK 20181208]:
/// A Hecata recruits Screamer, a wraith: unlocked after.
#[test]
fn shroud_mastery_superior_unlocks_after_a_successful_wraith_recruit() {
    let mut e = game(2);
    let hecata = |c: &zvtes::crypt::CryptCard| {
        c.clan == "Hecata" && (4..=8).contains(&c.capacity) && no_stealth(c)
    };
    let a = e.add_minion(SeatId(0), textless(hecata, 0), 4, false);
    e.add_minion(SeatId(0), textless(hecata, 1), 4, false);
    e.add_minion(
        SeatId(1),
        textless(|c| c.capacity >= 4 && no_stealth(c), 2),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| c.capacity >= 4 && no_stealth(c), 3),
        4,
        false,
    );
    e.state.minions[a.index()].gained_levels = vec!["nec", "nec"];
    hand(&mut e, 0, &["Screamer", "Shroud Mastery"]);
    let p = first_play(&mut e, 0, a);
    pick(&mut e, p);
    let req = offering(&mut e, play(0, 1, a));
    assert_eq!(req.window, WindowLabel::ActionAsAnnounced);
    e.choose(index_of(&req, play(0, 1, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked);
}

/// Succulent Vitae [qui]: "Only usable if this vampire is about to
/// successfully hunt. Put this card on this vampire. This vampire can burn
/// this card to reduce the cost of a card requiring Quietus [qui] they play
/// by 1 blood"; "A hunt is not successful if no blood is gained" [LSJ
/// 20050720-2]: the card itself on A after its hunt.
#[test]
fn succulent_vitae_goes_on_the_hunter() {
    let (mut e, a, _) = duel(&["qui"], &[]);
    e.state.minions[a.index()].blood = 2;
    hand(&mut e, 0, &["Succulent Vitae"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(placed(&e, "Succulent Vitae", a).len(), 1);
    assert!(!ash_has(&e, 0, "Succulent Vitae"));

    let (mut e, a, _) = duel(&["qui"], &[]);
    e.state.minions[a.index()].blood = 2;
    e.state.minions[a.index()].disease = true;
    hand(&mut e, 0, &["Succulent Vitae"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, a));
}

/// Veil of Silence [QUI]: "This vampire gets First Strike with hand and
/// melee weapon strikes during the first round ... and the acting minion
/// cannot dodge during the first round": A's Vampiric Speed not offered in
/// round 1.
#[test]
fn veil_of_silence_superior_bars_the_acting_minions_dodge_in_the_first_round() {
    let (mut e, a, x) = duel(&["cel"], &["qui", "qui", "for"]);
    hand(&mut e, 0, &["Vampiric Speed"]);
    hand(&mut e, 1, &["Veil of Silence", "Skin of Rock"]);
    fight(&mut e, a, x);
    let req = offering(&mut e, play(0, 1, x));
    assert_eq!(req.window, WindowLabel::ActionBlocked);
    e.choose(index_of(&req, play(0, 1, x)));
    request(&mut e);
    let c = e.combat().expect("combat");
    let ix = c.sides.iter().position(|s| s.minion == x).expect("X");
    assert!(c.sides[ix].hand_melee_first_round);
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, a));
}

/// Venenation [ser]: "Put 1 corruption counter on the blocking minion. If
/// the number of your corruption counters on the minion equals or exceeds
/// their capacity or cost, you can burn those counters to steal the
/// blocking minion (combat does not occur)"; "the action is still blocked,
/// the blocking minion locks. Mask of a Thousand Faces cannot be used to
/// continue it" [ANK 20181003] [LSJ 20010803] [ANK 20220127]: a 1-pool
/// ally blocks; stolen, no combat, Mask never offered after.
#[test]
fn venenation_corrupts_the_blocker_and_steals_it_at_capacity() {
    let (mut e, a, _) = duel(&["ser"], &[]);
    e.state.minions[a.index()].gained_levels = vec!["ser", "obf"];
    let cheap = zvtes::library::LIBRARY
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.types.contains(&zvtes::library::CardType::Ally)
                && c.pool_cost == zvtes::library::Cost::Fixed(1)
                && zvtes::cards::lookup(i as u16).is_some()
        })
        .expect("a 1-pool ally") as u16;
    let ally = e.add_ally(SeatId(1), cheap);
    hand(&mut e, 0, &["Venenation", "Mask of a Thousand Faces"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(ally));
    pick_when(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert!(e.state.minions[ally.index()].corruption_of(SeatId(0)) >= 1);
    e.choose(index_of(&req, Choice::ChooseMinion(ally)));
    let mask = |o: &Choice| matches!(o, Choice::Play { card: 0, .. });
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, mask);
    assert_eq!(e.state.minions[ally.index()].owner, SeatId(0), "stolen");
    assert_eq!(e.state.seats[1].pool, 10, "still blocked");
}

/// The 1-pool ally a Venenation test steals.
fn one_pool_ally() -> u16 {
    zvtes::library::LIBRARY
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.types.contains(&zvtes::library::CardType::Ally)
                && c.pool_cost == zvtes::library::Cost::Fixed(1)
                && zvtes::cards::lookup(i as u16).is_some()
        })
        .expect("a 1-pool ally") as u16
}

/// Venenation's steal: "an effect can be used to continue the action as if
/// unblocked (eg. Crypt's Sons, Momentary Delay)" [ANK 20181003] [LSJ
/// 20010803] [ANK 20220127]: the blocking ally stolen, A's Crypt's Sons
/// (1 life burned) or Momentary Delay continues the bleed, seat 1 bled.
#[test]
fn venenation_steal_then_crypts_sons_or_momentary_delay_continues() {
    for sons in [true, false] {
        let (mut e, a, _) = duel(&["ser"], &[]);
        let ally = e.add_ally(SeatId(1), one_pool_ally());
        let continue_it = if sons {
            let c = in_play(&mut e, 0, "Crypt's Sons", Some(a));
            e.state.cards[c.index()].counters = 3;
            hand(&mut e, 0, &["Venenation"]);
            Choice::UseAbility {
                source: zvtes::cards::AbilitySource::Card(c),
                ability: 0,
                actor: None,
                target: Target::None,
            }
        } else {
            hand(&mut e, 0, &["Venenation", "Momentary Delay"]);
            play(0, 0, a)
        };
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(ally));
        pick_when(&mut e, play(0, 0, a));
        let req = request(&mut e);
        e.choose(index_of(&req, Choice::ChooseMinion(ally)));
        assert_eq!(e.state.minions[ally.index()].owner, SeatId(0), "stolen");
        pick_when(&mut e, continue_it);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10 - 1, "{sons}: the bleed lands");
    }
}

/// Venenation: "Modifiers that are played before block resolution like
/// Change of Target can be played after Venenation" [ANK 20190524] [LSJ
/// 20020126]; "can be played both before and after" [LSJ 20010803]: no
/// steal (a vampire blocks), the window still open: Change of Target
/// offered.
#[test]
fn venenation_then_change_of_target_before_block_resolution() {
    let (mut e, a, x) = duel(&["ser"], &[]);
    hand(&mut e, 0, &["Venenation", "Change of Target"]);
    fight_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::ActionBlocked);
    assert!(
        req.options.iter().any(|o| is_play_of(o, 0, a)),
        "Change of Target: {:?}",
        req.options
    );
    assert_eq!(e.state.minions[x.index()].corruption_of(SeatId(0)), 1);
}

/// Pass until a play of hand card `card` by `m` is offered, and take the
/// first one.
fn play_when(e: &mut Engine, card: u8, m: MinionId) {
    for _ in 0..200 {
        let req = request(e);
        if let Some(i) = req.options.iter().position(|o| is_play_of(o, card, m)) {
            e.choose(i);
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("no play of {card} by {m:?}");
}

/// A bleeds, X blocks (no combat yet).
fn fight_blocked(e: &mut Engine, a: MinionId, x: MinionId) {
    pick(e, bleed(a));
    pick_when(e, block(x));
}

/// Shroud Mastery [NEC] and The Summoning: "Is not an action to recruit a
/// specific type of ally" [ANK 20250526]: not offered as it is announced.
#[test]
fn shroud_mastery_is_not_offered_on_the_summoning() {
    let mut e = game(2);
    let hecata = |c: &zvtes::crypt::CryptCard| {
        c.clan == "Hecata" && (4..=8).contains(&c.capacity) && no_stealth(c)
    };
    let a = e.add_minion(SeatId(0), textless(hecata, 0), 4, false);
    e.add_minion(SeatId(0), textless(hecata, 1), 4, false);
    e.add_minion(
        SeatId(1),
        textless(|c| c.capacity >= 4 && no_stealth(c), 2),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| c.capacity >= 4 && no_stealth(c), 3),
        4,
        false,
    );
    e.state.minions[a.index()].gained_levels = vec!["pre", "nec", "nec"];
    hand(&mut e, 0, &["Summoning, The", "Shroud Mastery"]);
    library(&mut e, 0, &["Screamer"; 8]);
    let p = first_play(&mut e, 0, a);
    pick(&mut e, p);
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 1, a));
}

/// Venenation and Nakhthorheb: "If the block succeeds and the blocking
/// minion gets a corruption counter (eg. Venenation), Nakhthorheb is still
/// blocked" [LSJ 20071021]: the bleed never lands.
#[test]
fn nakhthorheb_stays_blocked_after_venenation() {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (6..=8).contains(&c.capacity) && no_stealth(c);
    let n = e.add_minion(SeatId(0), zvtes::state::find_card("Nakhthorheb"), 6, false);
    e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(mid, 1), 6, false);
    e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.state.minions[n.index()].gained_levels = vec!["ser"];
    e.state.minions[x.index()].gained_levels = vec!["for"];
    hand(&mut e, 0, &["Venenation"]);
    hand(&mut e, 1, &["Skin of Rock"]);
    pick(&mut e, bleed(n));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, n));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[x.index()].corruption_of(SeatId(0)), 1);
    assert_eq!(e.state.seats[1].pool, 10, "still blocked: no bleed");
}

// ---- Action cards (action_a) ----------------------------------------------------

/// Seat 0's A (capacity 4 to 5, `levels`) and a second vampire, seat 1's X
/// (capacity 6 to 8) and a second one, 6 blood each.
fn actors(levels: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let small = |c: &zvtes::crypt::CryptCard| (4..=5).contains(&c.capacity) && no_stealth(c);
    let mid = |c: &zvtes::crypt::CryptCard| (6..=8).contains(&c.capacity) && no_stealth(c);
    let a = e.add_minion(SeatId(0), textless(small, 0), 6, false);
    e.add_minion(SeatId(0), textless(small, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(mid, 0), 6, false);
    e.add_minion(SeatId(1), textless(mid, 1), 4, false);
    e.state.minions[a.index()].gained_levels = levels.to_vec();
    (e, a, x)
}

/// Atonement: "Put this card on this vampire. This vampire gets +1
/// intercept and does not lock for blocking a vampire the same age or
/// younger": the card itself on A, +1 intercept; blocking a younger X's
/// bleed, A stays unlocked.
#[test]
fn atonement_blocker_stays_unlocked_against_a_vampire_not_older() {
    let (mut e, a, _) = actors(&[]);
    hand(&mut e, 0, &["Atonement"]);
    let p = first_play(&mut e, 0, a);
    pick(&mut e, p);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(placed(&e, "Atonement", a).len(), 1);
    assert_eq!(e.state.bearer_always(a, zvtes::cards::Stat::Intercept), 1);
    let young = e.add_minion(
        SeatId(1),
        textless(|c| c.capacity <= 3 && no_stealth(c), 0),
        3,
        false,
    );
    e.state.minions[a.index()].locked = false;
    pick(&mut e, bleed(young));
    pick_when(&mut e, block(a));
    request(&mut e);
    assert!(
        !e.state.minions[a.index()].locked,
        "not locked for the block"
    );
}

/// Blackmail [OBF]: "The vampire with this card cannot block this acting
/// vampire": X, Blackmailed by A, cannot block A's next bleed.
#[test]
fn blackmail_bars_blocking_its_player() {
    let (mut e, a, x) = actors(&["obf", "obf"]);
    hand(&mut e, 0, &["Blackmail"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(x)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(placed(&e, "Blackmail", x).len(), 1);
    assert!(!ash_has(&e, 0, "Blackmail"));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(x));
}

/// Blood Clots: "The attached minion cannot gain blood or life (instead,
/// any blood or life this minion would gain goes to the blood bank)"; "Any
/// effect that gives a vampire more blood counts as gaining blood" [ANK
/// 20221019-2]: the clotted X hunts, gains nothing.
#[test]
fn blood_clots_stops_the_attached_minions_gains() {
    let (mut e, a, x) = actors(&["qui"]);
    hand(&mut e, 0, &["Blood Clots"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(x)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(placed(&e, "Blood Clots", x).len(), 1);
    pick(&mut e, action(x, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 6, "the hunt's blood to the bank");
}

/// Condemnation: Betrayed [dai]: "Bleed actions cost the attached minion +1
/// blood (allies cannot pay blood costs)": an ally with it is offered no
/// bleed; the mode played needs only its Discipline (LSJ 20011204-3).
#[test]
fn condemnation_betrayed_bars_an_allys_bleed() {
    let (mut e, _, _) = actors(&[]);
    let ally = e.add_ally(SeatId(0), find_library("Carlton Van Wyk"));
    let card = in_play_mode(&mut e, 0, "Condemnation: Betrayed", Some(ally), 1);
    let _ = card;
    let req = request(&mut e);
    assert!(!req.options.contains(&bleed(ally)), "{:?}", req.options);
}

/// Condemnation: Doomed [DAI]: "The attached minion gets -1 stealth ... if
/// the attached minion is blocked, their controller burns 1 pool before
/// block resolution" [ANK 20220116] [ANK 20211207] [ANK 20210627] [RTR
/// 19991206] [LSJ 20091125]: A blocked, seat 0 burns 1.
#[test]
fn condemnation_doomed_costs_a_pool_when_blocked() {
    let (mut e, a, x) = actors(&[]);
    in_play_mode(&mut e, 1, "Condemnation: Doomed", Some(a), 2);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 10 - 1);
}

/// Condemnation: Languid [DAI]: "-1 strength ... cannot press in combat":
/// A, strength 0, offered no press with Relentless Pursuit in hand.
#[test]
fn condemnation_languid_weakens_and_bars_presses() {
    let (mut e, a, x) = actors(&["pot"]);
    in_play_mode(&mut e, 1, "Condemnation: Languid", Some(a), 2);
    hand(&mut e, 0, &["Relentless Pursuit"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, a));
    assert_eq!(blood(&e, x), 6, "strength 0");
}

/// Corruption: "Put 1 corruption counter on a minion controlled by your
/// prey ... at their capacity or cost, you can burn those counters to steal
/// the target minion"; The Diamond Thunderbolt stops only the steal, the
/// counters burned [LSJ 20070222-2]: the counter put.
#[test]
fn corruption_corrupts_a_prey_minion() {
    let mut e = game(2);
    let ministry = |c: &zvtes::crypt::CryptCard| {
        c.clan == "Ministry" && (4..=8).contains(&c.capacity) && no_stealth(c)
    };
    let a = e.add_minion(SeatId(0), textless(ministry, 0), 4, false);
    e.add_minion(SeatId(0), textless(ministry, 1), 4, false);
    let x = e.add_minion(
        SeatId(1),
        textless(|c| c.capacity >= 6 && no_stealth(c), 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| c.capacity >= 6 && no_stealth(c), 1),
        4,
        false,
    );
    hand(&mut e, 0, &["Corruption"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(x)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[x.index()].corruption_of(SeatId(0)), 1);
}

/// Dismemberment of Osiris [SER]: "Burn a titled vampire in torpor (this is
/// not diablerie). Each ready vampire of the same clan as the titled
/// vampire burns 1 blood ... the titled vampire's controller burns 1 pool".
#[test]
fn dismemberment_of_osiris_burns_a_torpid_titled_vampire_and_bleeds_its_clan() {
    let (mut e, a, _) = actors(&["ser", "ser"]);
    let titled = fixture_vampire(|c| c.title != Title::None && c.capacity >= 6, 0);
    let t = e.add_minion(SeatId(1), titled, 2, true);
    let clan = CRYPT[titled as usize].clan;
    let mate = fixture_vampire(
        |c| c.clan == clan && c.title == Title::None && c.capacity >= 4,
        0,
    );
    let m = e.add_minion(SeatId(1), mate, 4, false);
    hand(&mut e, 0, &["Dismemberment of Osiris"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(t)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ash_crypt.contains(&titled));
    assert_eq!(blood(&e, m), 4 - 1);
    assert_eq!(e.state.seats[1].pool, 10 - 1);
}

/// Edge Vitiation [ser]: "Only usable if your prey has the Edge. Your prey
/// burns 1 pool and burns the Edge."
#[test]
fn edge_vitiation_burns_the_preys_edge_and_a_pool() {
    let (mut e, a, _) = actors(&["ser"]);
    e.state.seats[1].has_edge = true;
    hand(&mut e, 0, &["Edge Vitiation"]);
    pick(&mut e, play_on(0, 0, a, Target::Seat(SeatId(1))));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.seats[1].has_edge);
    assert_eq!(e.state.seats[1].pool, 10 - 1);
}

/// Edge Vitiation [SER] against Leverage: the bleed's Edge meets two
/// replacements, the acting Methuselah orders them (advanced rules §2.4):
/// Leverage first, "If you would get the Edge, it is burned instead", and
/// nobody gets it; with no Leverage, seat 1 gets the Edge seat 0's
/// successful bleed would have given.
#[test]
fn edge_vitiation_superior_loses_to_the_acting_bleeders_leverage() {
    for leverage in [true, false] {
        let (mut e, a, _) = actors(&[]);
        in_play_mode(&mut e, 1, "Edge Vitiation", None, 1);
        if leverage {
            e.state.seats[0].has_edge = true;
            hand(&mut e, 0, &["Leverage"]);
        }
        pick(&mut e, bleed(a));
        if leverage {
            pick_when(&mut e, play(0, 0, a));
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(!e.state.seats[0].has_edge, "{leverage}");
        assert_eq!(e.state.seats[1].has_edge, !leverage, "{leverage}");
        assert_eq!(e.state.seats[1].pool, 10 - if leverage { 2 } else { 1 });
    }
}

/// Erosion: "The attached minion gets a strength of 0"; "Erosion resets a
/// minion's base strength. Any modifiers ... are applied to the new base"
/// [LSJ 19971211-2]: X's base strength 0.
#[test]
fn erosion_resets_strength_to_zero() {
    let (mut e, a, _) = actors(&["qui"]);
    let plain_x = (0..CRYPT.len() as u16)
        .find(|&i| {
            fixture_rank(i) == Some(0)
                && CRYPT[i as usize].capacity >= 6
                && !CRYPT[i as usize].imbued
        })
        .expect("a vampire with no text");
    let x = e.add_minion(SeatId(1), plain_x, 6, false);
    let _ = a;
    in_play(&mut e, 0, "Erosion", Some(x));
    assert_eq!(e.state.base_strength(x), 0);
}

/// Erosion: "cannot use melee weapons", every effect of one barred, its
/// passive ones too (rules-qa 124.252): X's Sword of Troile gives it no
/// superior Potence under Erosion; without Erosion, it does.
#[test]
fn erosion_bars_a_melee_weapons_passive_effects() {
    for eroded in [true, false] {
        let (mut e, _, _) = actors(&["qui"]);
        let x = e.add_minion(
            SeatId(0),
            textless(
                |c| c.capacity >= 6 && !c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("pot")),
                2,
            ),
            6,
            false,
        );
        in_play(&mut e, 0, "Sword of Troile", Some(x));
        if eroded {
            in_play(&mut e, 1, "Erosion", Some(x));
        }
        assert_eq!(
            e.state.discipline_level(x, "pot"),
            if eroded { 0 } else { 2 },
            "{eroded}"
        );
    }
}

/// Erosion: "cannot use melee weapons": the eroded X blocking A's bleed is
/// never offered its Baseball Bat's strike; without Erosion, it is.
#[test]
fn erosion_bars_a_melee_weapons_strike() {
    // Talbot's Chainsaw prints no "melee": a weapon is a melee weapon only
    // if its card says so (owner Q6, Tom Wylie 19940912), Erosion lets it be.
    for (weapon, eroded) in [
        ("Baseball Bat", true),
        ("Baseball Bat", false),
        ("Talbot's Chainsaw", true),
    ] {
        let (mut e, a, x) = actors(&[]);
        let bat = in_play(&mut e, 1, weapon, Some(x));
        if eroded {
            in_play(&mut e, 0, "Erosion", Some(x));
        }
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(x));
        let strikes = |o: &Choice| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), .. } if *c == bat);
        let mut offered = false;
        for _ in 0..100 {
            let req = request(&mut e);
            offered |= req.options.iter().any(strikes);
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            e.choose(fallback(&req));
        }
        assert_eq!(
            offered,
            !eroded || weapon != "Baseball Bat",
            "{weapon} {eroded}"
        );
    }
}

/// Feline Saboteur [ANI][OBF]: "discards 1 card at random ... and burns the
/// top 4 cards of his or her library"; "discards, then burn 4 cards, then
/// replace" [ANK 20180725]: the replacement is the library's fifth card.
#[test]
fn feline_saboteur_superior_discards_burns_four_then_replaces() {
    let (mut e, a, _) = actors(&["ani", "ani", "obf", "obf"]);
    hand(&mut e, 0, &["Feline Saboteur"]);
    hand(&mut e, 1, &["Skin of Rock"; 7]);
    let lib = [
        "Fake Out",
        "Blood Agony",
        "Disease",
        "Dagon's Call",
        "Draba",
    ];
    e.state.seats[1].library = lib
        .iter()
        .map(|c| find_library(c))
        .collect::<Vec<_>>()
        .into();
    pick(&mut e, play_on(0, 1, a, Target::Seat(SeatId(1))));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Skin of Rock"), "the discard");
    assert_eq!(e.state.seats[1].hand.len(), 7);
    for burned in ["Blood Agony", "Disease", "Dagon's Call", "Draba"] {
        assert!(ash_has(&e, 1, burned), "{burned}");
    }
    assert!(e.state.seats[1].hand.contains(&find_library("Fake Out")));
}

/// Fleecing the Gaje: "Move ... 1 blood from a younger vampire controlled
/// by another Methuselah to this Ravnos."
#[test]
fn fleecing_the_gaje_takes_a_younger_vampires_blood() {
    let mut e = game(2);
    let ravnos =
        |c: &zvtes::crypt::CryptCard| c.clan == "Ravnos" && c.capacity >= 7 && no_stealth(c);
    let r = e.add_minion(SeatId(0), textless(ravnos, 0), 4, false);
    e.add_minion(SeatId(0), textless(ravnos, 1), 4, false);
    let y = e.add_minion(
        SeatId(1),
        textless(|c| c.capacity <= 4 && no_stealth(c), 0),
        3,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| c.capacity <= 4 && no_stealth(c), 1),
        3,
        false,
    );
    hand(&mut e, 0, &["Fleecing the Gaje"]);
    pick(&mut e, play_on(0, 2, r, Target::Minion(y)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, r), blood(&e, y)), (5, 2));
}

/// Haunt [NEC]: "Burn a location that does not require Giovanni to play";
/// "The action is directed at the Methuselah controlling the location"
/// [LSJ 20090324].
#[test]
fn haunt_superior_burns_a_location_directed_at_its_controller() {
    let (mut e, a, _) = actors(&["nec", "nec"]);
    let loc = zvtes::library::LIBRARY
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.labels.contains(&zvtes::library::Label::Location)
                && !c.clans.contains(&"Giovanni")
                && zvtes::cards::lookup(i as u16).is_some()
                && !c.labels.contains(&zvtes::library::Label::Unique)
        })
        .expect("a location") as u16;
    let name = zvtes::library::LIBRARY[loc as usize].name;
    let l = in_play(&mut e, 1, name, None);
    hand(&mut e, 0, &["Haunt"]);
    pick(&mut e, play_on(0, 1, a, Target::Card(l)));
    request(&mut e);
    assert_eq!(
        e.action().expect("the action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[l.index()].in_play);
}

/// Jar the Soul [NEC]: "Lock a ready minion ... and burn 1 blood or life";
/// "Does not damage vampires: an empty vampire will simply attempt to burn
/// blood and fail" [TOM 19951212-2]: X, empty, locked, still ready.
#[test]
fn jar_the_soul_superior_burns_without_damage() {
    let (mut e, a, x) = actors(&["nec", "nec"]);
    e.state.minions[x.index()].blood = 0;
    hand(&mut e, 0, &["Jar the Soul"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(x)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let v = &e.state.minions[x.index()];
    assert!(v.ready() && !v.in_torpor);
}

/// Jar the Soul [NEC] on a 1-life ally: "this counts as a Ⓓ action burning
/// an ally" (Predator's Transformation) [ANK 20220507]: the prey's ally
/// burned, among the action's burns.
#[test]
fn jar_the_soul_burning_a_prey_ally_counts_as_this_action_burning_it() {
    let (mut e, a, _) = actors(&["nec", "nec"]);
    let ally = e.add_ally(SeatId(1), find_library("Carlton Van Wyk"));
    e.state.minions[ally.index()].blood = 1;
    hand(&mut e, 0, &["Jar the Soul"]);
    e.notes = Some(Vec::new());
    pick(&mut e, play_on(0, 1, a, Target::Minion(ally)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play);
    let note = noted_actions(&mut e)
        .into_iter()
        .find(|n| n.action.acting == a && n.ended)
        .expect("the action");
    assert!(note
        .action
        .burned_owners
        .iter()
        .any(|&(s, ..)| s == SeatId(1)));
}

/// Daemonic Possession [NEC]: "Choose a minion that was controlled by
/// another Methuselah who has been burned since your last turn. Move the
/// chosen minion from their owner's ash heap to your ready region with 1
/// blood"; "since your last turn" [LSJ 20020429]; undirected [LSJ
/// 20010627] [ANK 20180202]: seat 1's vampire burned this turn, A takes it.
#[test]
fn daemonic_possession_takes_a_burned_vampire() {
    let (mut e, a, _) = actors(&["nec", "nec"]);
    let victim = textless(|c| c.capacity == 3, 0);
    let v = e.add_minion(SeatId(1), victim, 1, false);
    zvtes::copies::number_unnumbered(&mut e);
    e.burn(v);
    hand(&mut e, 0, &["Daemonic Possession"]);
    pick(&mut e, play(0, 1, a));
    request(&mut e);
    assert_eq!(e.action().expect("action").directed_seat(&e.state), None);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let m = minion_of(&e, 0, victim).expect("in seat 0's ready region");
    assert_eq!(blood(&e, m), 1);
}

/// Daemonic Possession: "a minion that was controlled by another
/// Methuselah" when it burned, read from the controller at the burn
/// [LSJ 20081014] [LSJ 20010717] [ANK 20220922]: seat 0's own vampire,
/// burned while seat 1 controlled it, lies in seat 0's own ash heap and is
/// taken; seat 1's vampire burned while seat 0 controlled it is not
/// offered (the card unplayable).
#[test]
fn daemonic_possession_reads_the_controller_at_the_burn() {
    for own_heap in [true, false] {
        let (mut e, a, _) = actors(&["nec", "nec"]);
        let victim = textless(|c| c.capacity == 3, 0);
        let (controller, owner) = if own_heap { (1, 0) } else { (0, 1) };
        let v = e.add_minion(SeatId(controller), victim, 1, false);
        e.state.minions[v.index()].origin = Some(SeatId(owner));
        zvtes::copies::number_unnumbered(&mut e);
        e.burn(v);
        assert!(e.state.seats[owner as usize].ash_crypt.contains(&victim));
        hand(&mut e, 0, &["Daemonic Possession"]);
        let req = request(&mut e);
        assert_eq!(req.options.contains(&play(0, 1, a)), own_heap, "{own_heap}");
        if own_heap {
            pick(&mut e, play(0, 1, a));
            until(&mut e, SeatId(1), WindowLabel::MinionPhase);
            let m = minion_of(&e, 0, victim).expect("in seat 0's ready region");
            assert_eq!(blood(&e, m), 1);
        }
    }
}

/// Grasp the Ghostly [nec]: "Move a non-unique non-location equipment card
/// from another Methuselah's ash heap to this minion and put 3 pathos
/// counters ... Remove the equipment from the game if it has no pathos
/// counters": A gets seat 1's .44 Magnum; three unlock phases later it is
/// removed from the game. "If the equipment is burned before ..., it goes
/// to its owner's ash heap" [LSJ 20060209]; undirected [LSJ 20010627]
/// [ANK 20180202].
#[test]
fn grasp_the_ghostly_moves_an_equipment_that_fades_in_three_unlocks() {
    let (mut e, a, _) = actors(&["nec"]);
    e.state.seats[1]
        .ash
        .put(find_library(".44 Magnum"), Default::default());
    hand(&mut e, 0, &["Grasp the Ghostly"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let gun = placed(&e, ".44 Magnum", a);
    assert_eq!(gun.len(), 1);
    for _ in 0..3 {
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    }
    assert!(!e.state.cards[gun[0].index()].in_play);
    assert!(e.state.seats[1]
        .removed
        .contains(&find_library(".44 Magnum")));
}

/// Guard Duty: "Put this card on the acting minion and choose a location.
/// If the chosen location is the target of a directed action (and you
/// control the location), this minion can burn 1 blood or burn this card to
/// unlock and attempt to block with +1 intercept"; the location chosen the
/// action's target (rules-qa 35.1); not if it cannot attempt [LSJ
/// 20010714-2], the restrictions kept [ANK 20180623]: A guards its own
/// location, then a +1
/// stealth action at it by seat 1: A, locked, unlocks and blocks.
#[test]
fn guard_duty_unlocks_to_block_an_action_at_the_chosen_location() {
    let (mut e, a, x) = actors(&[]);
    let loc = zvtes::library::LIBRARY
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.labels.contains(&zvtes::library::Label::Location)
                && !c.clans.contains(&"Giovanni")
                && zvtes::cards::lookup(i as u16).is_some()
                && !c.labels.contains(&zvtes::library::Label::Unique)
        })
        .expect("a location") as u16;
    let name = zvtes::library::LIBRARY[loc as usize].name;
    let l = in_play(&mut e, 0, name, None);
    hand(&mut e, 0, &["Guard Duty"]);
    pick(&mut e, play_on(0, 0, a, Target::Card(l)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let guard = placed(&e, "Guard Duty", a);
    assert_eq!(guard.len(), 1);
    assert!(e.state.minions[a.index()].locked);
    e.state.minions[x.index()].gained_levels = vec!["nec", "nec"];
    hand(&mut e, 1, &["Haunt"]);
    pick(&mut e, play_on(0, 1, x, Target::Card(l)));
    let use_it = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(guard[0]),
        ability: 1,
        actor: None,
        target: Target::None,
    };
    let req = offering(&mut e, use_it);
    e.choose(index_of(&req, use_it));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert!(e.state.cards[l.index()].in_play, "the location still there");
}

/// Guard Duty on another Methuselah's location: choosing it is targeting
/// it (rules-qa 35.1, LSJ 20031010), so the action is directed at its
/// controller (D12); legal there, the card does nothing for A (it needs
/// "you control the location").
#[test]
fn guard_duty_at_anothers_location_is_directed_at_them() {
    let (mut e, a, _) = actors(&[]);
    let loc = zvtes::library::LIBRARY
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.labels.contains(&zvtes::library::Label::Location)
                && !c.clans.contains(&"Giovanni")
                && zvtes::cards::lookup(i as u16).is_some()
                && !c.labels.contains(&zvtes::library::Label::Unique)
        })
        .expect("a location") as u16;
    let name = zvtes::library::LIBRARY[loc as usize].name;
    let l = in_play(&mut e, 1, name, None);
    hand(&mut e, 0, &["Guard Duty"]);
    pick(&mut e, play_on(0, 0, a, Target::Card(l)));
    request(&mut e);
    assert_eq!(
        e.action().expect("the action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(placed(&e, "Guard Duty", a).len(), 1, "legal there");
}

/// Corruption and React with Conviction: "If used to cancel a Corruption
/// action, then the action is canceled in its entirety and the corruption
/// counters are not put" [LSJ 20070222-1]: seat 1's imbued burns its React
/// with Conviction against A's Corruption: no counter.
#[test]
fn react_with_conviction_cancels_a_corruption_whole() {
    let mut e = game(2);
    let ministry = |c: &zvtes::crypt::CryptCard| {
        c.clan == "Ministry" && (4..=8).contains(&c.capacity) && no_stealth(c)
    };
    let a = e.add_minion(SeatId(0), textless(ministry, 0), 4, false);
    e.add_minion(SeatId(0), textless(ministry, 1), 4, false);
    let m = e.add_minion(
        SeatId(1),
        zvtes::state::find_card("Marion \"Teacher193\" Perks"),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| c.capacity >= 6 && no_stealth(c), 1),
        4,
        false,
    );
    let rc = in_play(&mut e, 1, "React with Conviction", Some(m));
    hand(&mut e, 0, &["Corruption"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(m)));
    let u = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(rc),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    let req = offering(&mut e, u);
    e.choose(index_of(&req, u));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[m.index()].corruption_of(SeatId(0)), 0);
    assert!(!e.state.cards[rc.index()].in_play, "burned");
}

// ---- Vampires (vamp_e) ----------------------------------------------------------

/// The crypt card `name`, advanced or not (`find_card` returns the first
/// row of a name: Kemintiri's advanced card).
fn crypt_adv(name: &str, adv: bool) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && c.adv == adv)
        .unwrap_or_else(|| panic!("no {name}")) as u16
}

/// Vampire `src`'s ability `ability`, by `actor`, at `target`.
fn vuse(src: MinionId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(src),
        ability,
        actor,
        target,
    }
}

/// Seat 0's merged Kemintiri (the advanced card over the base), 10 blood,
/// and a second vampire.
fn merged_kemintiri(e: &mut Engine) -> MinionId {
    let k = e.add_minion(SeatId(0), crypt_adv("Kemintiri", true), 10, false);
    e.state.minions[k.index()].merged = Some(crypt_adv("Kemintiri", false));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    k
}

/// Merged Kemintiri's justicar political action: "she is not seen as such
/// by other cards ... during the referendum (eg. another vampire can't play
/// Promise of 1528)" [LSJ 20050128-2] [ANK 20221011-4] [RTR 20070707]: her
/// Anathema choosing seat 1's ready Giovanni, Promise of 1528 never offered
/// in the polling; a Camarilla prince's Anathema on the same Giovanni,
/// offered.
#[test]
fn kemintiri_merged_anathema_is_no_camarilla_action_for_promise_of_1528() {
    for merged in [true, false] {
        let mut e = game(2);
        let caller = if merged {
            merged_kemintiri(&mut e)
        } else {
            let p = e.add_minion(SeatId(0), plain("Ventrue", 0), 6, false);
            e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
            p
        };
        let g = e.add_minion(SeatId(1), plain("Giovanni", 0), 4, false);
        e.add_minion(SeatId(1), plain("Gangrel", 0), 4, false);
        hand(&mut e, 0, &["Anathema"]);
        hand(&mut e, 1, &["Promise of 1528"]);
        pick(
            &mut e,
            Choice::Play {
                card: 0,
                mode: 0,
                actor: Some(caller),
                target: Target::None,
            },
        );
        pick_when(&mut e, Choice::ChooseMinion(g));
        let mut polled = false;
        let mut offered = false;
        for _ in 0..200 {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                break;
            }
            if req.window == WindowLabel::ReferendumPolling {
                polled = true;
                offered |= req
                    .options
                    .iter()
                    .any(|o| matches!(o, Choice::Play { card: 0, actor: Some(a), .. } if *a == g));
            }
            e.choose(fallback(&req));
        }
        assert!(polled, "{merged}");
        assert_eq!(offered, !merged, "{merged}");
    }
}

/// Kemintiri (base): "+1 bleed and +1 stealth when bleeding a Methuselah
/// who controls a Ventrue": a torpid Ventrue counts (rules-qa 53.8), a
/// Ventrue antitribu does not.
#[test]
fn kemintiri_bleeds_harder_against_a_ventrue_keeper() {
    for (clan, bonus) in [("Ventrue", 1), ("Ventrue antitribu", 0)] {
        let mut e = game(2);
        let k = e.add_minion(SeatId(0), crypt_adv("Kemintiri", false), 10, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain(clan, 0), 2, true);
        e.add_minion(SeatId(1), plain("Gangrel", 0), 4, false);
        pick(&mut e, bleed(k));
        request(&mut e);
        assert_eq!(e.action().unwrap().stealth(&e.state), bonus, "{clan}");
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10 - 1 - bonus, "{clan}");
    }
}

/// Kemintiri: "Any Ventrue can enter combat with her as a Ⓓ action" — an
/// action requiring a Ventrue [LSJ 20080604]: another Methuselah's
/// Ventrue rushes her, a Brujah may not; "She can enter combat with a
/// Ventrue as a +1 stealth Ⓓ action": at a Ventrue, not at a Gangrel.
#[test]
fn kemintiri_is_rushed_by_another_methuselahs_ventrue() {
    let mut e = game(2);
    let ven = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let k = e.add_minion(SeatId(1), crypt_adv("Kemintiri", false), 10, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let rush = |a| vuse(k, 0, Some(a), Target::Minion(k));
    let req = request(&mut e);
    assert!(req.options.contains(&rush(ven)), "{:?}", req.options);
    assert!(!req.options.contains(&rush(other)));
    pick(&mut e, rush(ven));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ven), 3, "in combat with her");

    let mut e = game(2);
    let k = e.add_minion(SeatId(0), crypt_adv("Kemintiri", false), 10, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let ven = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let gan = e.add_minion(SeatId(1), plain("Gangrel", 0), 4, false);
    let rush = |t| vuse(k, 1, Some(k), Target::Minion(t));
    let req = request(&mut e);
    assert!(req.options.contains(&rush(ven)), "{:?}", req.options);
    assert!(!req.options.contains(&rush(gan)));
    pick(&mut e, rush(ven));
    request(&mut e);
    assert_eq!(e.action().unwrap().stealth(&e.state), 1);
}

/// Merged Kemintiri: base and advanced text with the [MERGED] text
/// (rules-qa 111.19): she recruits Political Ally (requires a Ventrue) and
/// calls Parity Shift (a prince or justicar), "treated as such for all the
/// effects of that card" [LSJ 20101013] [ANK 20190228] [ANK 20200705-2]
/// [LSJ 20070708] [RTR 20070707]; casting her own 3 votes, not a
/// justicar's [ANK 20231221], none from Elysium: The Palace of Versailles
/// [LSJ 20050128-2] [ANK 20221011-4] [RTR 20070707]; minion cards only [LSJ
/// 20050721]; unmerged, none of it.
#[test]
fn kemintiri_merged_plays_ventrue_and_justicar_minion_cards() {
    let mut e = game(2);
    let k = merged_kemintiri(&mut e);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1].pool = 20;
    hand(&mut e, 0, &["Parity Shift", "Political Ally"]);
    let p = |card| Choice::Play {
        card,
        mode: 0,
        actor: Some(k),
        target: Target::None,
    };
    let req = request(&mut e);
    assert!(req.options.contains(&p(0)), "{:?}", req.options);
    assert!(req.options.contains(&p(1)), "{:?}", req.options);
    assert_eq!(e.state.title_of(k), Title::Votes(3));
    let elysium = in_play(&mut e, 0, "Elysium: The Palace of Versailles", None);
    pick(&mut e, p(0));
    to_window(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    pick_when(&mut e, ability(elysium.0, 0));
    request(&mut e);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, k), 3, "no titled Camarilla vampire");

    let mut e = game(2);
    let k = e.add_minion(SeatId(0), crypt_adv("Kemintiri", true), 10, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Parity Shift", "Political Ally"]);
    let req = request(&mut e);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { actor: Some(a), .. } if *a == k)),
        "unmerged: {:?}",
        req.options
    );
}

/// Merged Kemintiri and Closed Session ("Non-Camarilla vampires cannot cast
/// votes"): "When considered a Justicar, she is also implicitly considered
/// Camarilla" [LSJ 20050128]; she calls it and votes, "not seen as such by
/// other cards or effects" [LSJ 20050128-2] [ANK 20221011-4] [RTR
/// 20070707]: Independent throughout.
#[test]
fn kemintiri_merged_calls_closed_session_and_votes() {
    let mut e = game(2);
    let k = merged_kemintiri(&mut e);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested", "Closed Session"]);
    pick(&mut e, play(0, 0, k));
    pick(&mut e, alloc(0, 3));
    pick_when(&mut e, play(0, 0, k));
    let req = to_window(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    assert!(req.options.contains(&votes(k, true)), "{:?}", req.options);
    assert_eq!(e.state.sect_of(k), Some(zvtes::crypt::Sect::Independent));
}

/// Merged Kemintiri: "a justicar title" only where a requirement names one
/// (rules-qa 59.14): not Camarilla's Iron Fist ("a titled Camarilla
/// vampire"), Parity Shift yes; "Can meet a sect-specific title ...
/// requirement even if the sect does not exist anymore" [ANK 20230503]
/// [LSJ 20050124]: Parity Shift with Fall of the Camarilla in play.
#[test]
fn kemintiri_merged_meets_justicar_lists_only() {
    let mut e = game(4);
    let k = merged_kemintiri(&mut e);
    for s in 1..4 {
        e.add_minion(SeatId(s), plain("Ventrue", s as usize), 4, false);
    }
    e.state.seats[1].pool = 20;
    hand(&mut e, 0, &["Camarilla's Iron Fist", "Parity Shift"]);
    let req = request(&mut e);
    let plays = |card: u8| {
        req.options.iter().any(
            |o| matches!(o, Choice::Play { card: c, actor: Some(a), .. } if *c == card && *a == k),
        )
    };
    assert!(!plays(0), "{:?}", req.options);
    assert!(plays(1), "{:?}", req.options);

    let mut e = game(2);
    let k = merged_kemintiri(&mut e);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1].pool = 20;
    in_play(&mut e, 0, "Fall of the Camarilla", None);
    hand(&mut e, 0, &["Parity Shift"]);
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(k),
        target: Target::None,
    }));
}

/// Merged Kemintiri: "Keeps their title and votes if they changes sects"
/// [LSJ 20010714]: made Anarch, 3 votes; Kephamos, "2 votes (titled)",
/// keeps his 2 [LSJ 20010714]. Her Ventrue is no help for Piper, not
/// played normally [LSJ 20050119].
#[test]
fn kemintiri_merged_keeps_her_votes_and_lends_piper_no_ventrue() {
    let mut e = game(2);
    let k = merged_kemintiri(&mut e);
    let kep = e.add_minion(
        SeatId(0),
        find_card("Kephamos, High Priest of Marrakech"),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for m in [k, kep] {
        e.state.minions[m.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    }
    assert_eq!(e.state.title_of(k), Title::Votes(3));
    assert_eq!(e.state.title_of(kep), Title::Votes(2));
    hand(&mut e, 0, &["Piper", "Political Ally"]);
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == k),
    );
}

/// Kephamos: "+1 bleed".
#[test]
fn kephamos_bleeds_for_two() {
    let mut e = game(2);
    let kep = e.add_minion(
        SeatId(0),
        find_card("Kephamos, High Priest of Marrakech"),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    pick(&mut e, bleed(kep));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

/// Khay'tall: "+1 stealth during political actions. Vampires with any
/// corruption counters cannot cast votes or ballots against referendums
/// called by Khay'tall"; a counter gained after casting voids the cast
/// [PIB 20141026-2]: the corrupted voter casts only in favor; the clean one
/// against, voided once corrupted.
#[test]
fn khaytall_bars_corrupted_votes_against_his_referendum() {
    let mut e = game(2);
    let kh = e.add_minion(SeatId(0), find_card("Khay'tall, Snake of Eden"), 9, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let dirty = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let clean = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    for m in [dirty, clean] {
        e.state.minions[m.index()].granted_title = Some(Title::Votes(1));
    }
    e.state.minions[dirty.index()].corruption = vec![(SeatId(0), 1)];
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, kh));
    request(&mut e);
    assert_eq!(
        e.action().unwrap().stealth(&e.state),
        2,
        "the rulebook's +1 and his"
    );
    pick(&mut e, alloc(0, 3));
    let req = to_window(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    let _ = req;
    let req = offering(&mut e, votes(clean, false));
    assert!(
        !req.options.contains(&votes(dirty, false)),
        "{:?}",
        req.options
    );
    assert!(req.options.contains(&votes(dirty, true)));
    e.choose(index_of(&req, votes(clean, false)));
    request(&mut e);
    assert_eq!(e.referendum().unwrap().votes_of(&e.state, clean), 1);
    let against = e.referendum().unwrap().tally(&e.state).1;
    e.state.minions[clean.index()].corruption = vec![(SeatId(0), 1)];
    assert_eq!(
        e.referendum().unwrap().tally(&e.state).1,
        against - 1,
        "voided"
    );
}

/// Kostantin: "can steal an equipment as a +1 stealth Ⓓ action": seat 1's
/// .44 Magnum moves to him; his own minion's is no target.
#[test]
fn kostantin_steals_an_equipment_at_plus_one_stealth() {
    let mut e = game(2);
    let k = e.add_minion(
        SeatId(0),
        find_card("Kostantin, Baro of the Caravan"),
        9,
        false,
    );
    let mine = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Gangrel", 0), 4, false);
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(z));
    let own_gun = in_play(&mut e, 0, ".44 Magnum", Some(mine));
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&vuse(k, 0, Some(k), Target::Card(own_gun))));
    pick(&mut e, vuse(k, 0, Some(k), Target::Card(gun)));
    request(&mut e);
    assert_eq!(e.action().unwrap().stealth(&e.state), 1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[gun.index()].host, Some(k));
}

/// Krassimir: "Once each turn, if Krassimir is blocked, he can unlock and
/// end the action before block resolution": no combat, the action reached
/// resolution (NRA) [LSJ 20011202-2], nothing more played [RTR 20180719]
/// [ANK 20200207]; once in a player's turn [ANK 20180719-4]: blocked again,
/// not offered.
#[test]
fn krassimir_ends_a_blocked_action_once_each_turn() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Krassimir"), 7, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let z2 = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Skin of Rock"]);
    pick(&mut e, bleed(k));
    pick_when(&mut e, block(z));
    let end = vuse(k, 0, None, Target::None);
    pick_when(&mut e, end);
    let req = request(&mut e);
    assert!(e.combat().is_none(), "no combat");
    assert!(!e.state.minions[k.index()].locked, "unlocked");
    assert_eq!(e.state.seats[1].pool, 10);
    assert!(!req.options.contains(&bleed(k)), "the bleed spent (NRA)");
    pick(&mut e, action(k, ActionKind::Hunt));
    pick_when(&mut e, block(z2));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, end);
}

/// Krassimir: "He cannot use cards requiring Celerity [cel]"; "Can use a
/// multi-discipline card with a potential (unused) Celerity [cel] effect"
/// [LSJ 20050322-2]: Line Brawl's [cel] mode barred, its [pot] mode not.
/// Scarce generated.
#[test]
fn krassimir_shuns_celerity_modes() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Krassimir"), 7, false);
    let brawl = find_library("Line Brawl");
    let mode = |e: &Engine, m: u8| {
        e.state.printed_mod(
            k,
            zvtes::cards::Stat::CannotPlay,
            zvtes::state::PrintedCtx {
                card: Some((brawl, m)),
                ..Default::default()
            },
        )
    };
    assert_eq!(
        (mode(&e, 0), mode(&e, 1)),
        (1, 0),
        "[cel] barred, [pot] not"
    );
    assert!(CRYPT[find_card("Krassimir") as usize].scarce);
    assert!(CRYPT[find_card("Matthias") as usize].scarce);
}

/// Lithrac: "+1 stealth when taking an action to enter combat with another
/// minion" (Bum's Rush); his bleed at 0.
#[test]
fn lithrac_gets_stealth_rushing() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lithrac"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Gangrel", 0), 4, false);
    hand(&mut e, 0, &["Bum's Rush"]);
    pick(&mut e, play_on(0, 0, l, Target::Minion(z)));
    request(&mut e);
    assert_eq!(e.action().unwrap().stealth(&e.state), 1);
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lithrac"), 5, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(l));
    request(&mut e);
    assert_eq!(e.action().unwrap().stealth(&e.state), 0);
}

/// Luma: "Tremere Slave" (rules-qa 62.14): no bleed without a ready
/// Tremere; with one, she bleeds.
#[test]
fn luma_serves_the_tremere() {
    let mut e = game(2);
    let luma = e.add_minion(SeatId(0), find_card("Luma, Stone Beauty"), 5, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = request(&mut e);
    assert!(!req.options.contains(&bleed(luma)));
    e.add_minion(SeatId(0), plain("Tremere", 0), 4, false);
    let req = request(&mut e);
    assert!(req.options.contains(&bleed(luma)));
}

/// Marianna: "While Marianna is ready, any other Methuselah controlling a
/// ready Giovanni burns 1 pool during their unlock phase": seat 1, with two
/// ready Giovanni, burns 1; with its Giovanni in torpor or Marianna in
/// torpor, none.
#[test]
fn marianna_taxes_other_methuselahs_ready_giovanni() {
    for (marianna_torpid, giovanni_torpid, loss) in
        [(false, false, 1), (false, true, 0), (true, false, 0)]
    {
        let mut e = game(2);
        e.add_minion(SeatId(0), find_card("Marianna"), 8, marianna_torpid);
        e.add_minion(SeatId(0), plain("Giovanni", 2), 4, false);
        e.add_minion(SeatId(1), plain("Giovanni", 0), 4, giovanni_torpid);
        e.add_minion(SeatId(1), plain("Giovanni", 1), 4, giovanni_torpid);
        e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.seats[1].pool,
            10 - loss,
            "{marianna_torpid} {giovanni_torpid}"
        );
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[0].pool, 10, "not her controller");
    }
}

/// Marianna: "+1 strength in combat with a Giovanni".
#[test]
fn marianna_strikes_harder_against_a_giovanni() {
    for (clan, damage) in [("Giovanni", 2), ("Ventrue", 1)] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card("Marianna"), 8, false);
        let z = e.add_minion(SeatId(1), plain(clan, 0), 6, false);
        pick(&mut e, bleed(m));
        pick_when(&mut e, block(z));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, z), 6 - damage, "{clan}");
    }
}

/// Mary the Black: "Once each round, Mary can burn 1 blood to make the
/// damage from her hand strikes aggravated that round", after both strikes
/// chosen [RTR 19960112]. Infernal generated.
#[test]
fn mary_the_black_buys_aggravated_hand_strikes() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Mary the Black"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
    pick(&mut e, bleed(v));
    pick_when(&mut e, block(z));
    pick_when(&mut e, vuse(v, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor, "aggravated");
    assert_eq!(blood(&e, v), 6 - 1 - 1, "the cost, the blocker's strike");
    assert!(CRYPT[find_card("Mary the Black") as usize].infernal);
}

/// Matthias: "While Matthias is ready, Baali and Ministries get -1 bleed
/// against you. ... +1 intercept": a Ministry's bleed of 1 against his
/// controller takes nothing, a Brujah's 1; Matthias in torpor, the
/// Ministry's 1; his intercept 1.
#[test]
fn matthias_softens_baali_and_ministry_bleeds() {
    for (clan, torpid, loss) in [
        ("Ministry", false, 0),
        ("Brujah", false, 1),
        ("Ministry", true, 1),
    ] {
        let mut e = game(2);
        let b = e.add_minion(SeatId(0), plain(clan, 0), 4, false);
        let m = e.add_minion(SeatId(1), find_card("Matthias"), 7, torpid);
        let _ = m;
        pick(&mut e, bleed(b));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10 - loss, "{clan} {torpid}");
    }
    // "+1 intercept": his against a Brujah's bleed, 1.
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let m = e.add_minion(SeatId(1), find_card("Matthias"), 7, false);
    pick(&mut e, bleed(b));
    request(&mut e);
    assert_eq!(e.action().expect("the bleed").intercept_of(&e.state, m), 1);
}

/// Melek: "Once each combat, Melek can strike: dodge. Blood cursed."
#[test]
fn melek_dodges_once_each_combat() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Melek"), 7, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(z));
    let dodge = vuse(m, 0, None, Target::None);
    pick_when(&mut e, dodge);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, m), 7, "dodged");
    assert!(CRYPT[find_card("Melek") as usize].blood_cursed);
}

/// The text-only vampires: Kervos and Max Lowell Sabbat; Khalil Anvari,
/// Lalitha and Mario Giovanni Independent; no definition.
#[test]
fn the_round_35_b_textless_vampires_have_no_definition() {
    for (name, sect) in [
        ("Kervos, The Lieutenant", zvtes::crypt::Sect::Sabbat),
        ("Max Lowell", zvtes::crypt::Sect::Sabbat),
        ("Khalil Anvari", zvtes::crypt::Sect::Independent),
        ("Lalitha", zvtes::crypt::Sect::Independent),
        ("Mario Giovanni", zvtes::crypt::Sect::Independent),
    ] {
        let c = find_card(name);
        assert!(zvtes::cards::vampire(c).is_none(), "{name}");
        assert_eq!(CRYPT[c as usize].sect, Some(sect), "{name}");
    }
}
