//! Round 34 (`ADDED_BATCHES`): Black Hand and Camarilla Edition, its
//! combat cards, the vampires from Makarios onward and the trait tests of
//! Ingram to Kurt and Laurent to Olivia. Windows with a single option are
//! elided, so the scripts only stop where a real decision exists.

use zvtes::cards::{AbilitySource, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
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

/// No printed stealth (`fixture_rank` 2): a younger blocker stops its bleed
/// (no untitled one of capacity 9 or more is left).
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
    let big = |c: &zvtes::crypt::CryptCard| c.capacity >= 7;
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

/// The ability `ability` of the weapon `w`, used by its bearer.
fn weapon(w: CardId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(w),
        ability,
        actor: None,
        target: Target::None,
    }
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

/// Any use of the card in play `c`.
fn uses_card(o: &Choice, c: CardId) -> bool {
    matches!(o, Choice::UseAbility { source: AbilitySource::Card(x), .. } if *x == c)
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

// ---- Combat cards -------------------------------------------------------------

/// Blood to Water: "Only usable at close range, before strike resolution.
/// Not usable during the first round of combat." [ANK 20200703] [LSJ
/// 20031008] "Must be played after range is chosen, can be played before
/// or after strikes are chosen": never in round 1; in round 2 offered
/// before strikes and again after both are chosen, not before range. [tha]
/// on a vampire: 3 blood burned.
#[test]
fn blood_to_water_waits_for_range_then_either_side_of_strikes() {
    let (mut e, a, x) = duel(&["tha", "pot"], &[]);
    hand(&mut e, 0, &["Blood to Water", "Relentless Pursuit"]);
    fight(&mut e, a, x);
    let btw = |o: &Choice| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == a);
    never_offered_where(&mut e, SeatId(0), WindowLabel::CombatPress, btw);
    to_round_two(&mut e, a, 1);
    let req = offering(&mut e, play(0, 1, a));
    assert_eq!(req.window, WindowLabel::CombatBeforeStrikes);
    e.choose(index_of(&req, Choice::Pass));
    let req = offering(&mut e, play(0, 1, a));
    assert_eq!(req.window, WindowLabel::CombatBeforeResolution);
    assert!(e.combat().expect("combat").sides[0].strike.is_some());
    e.choose(index_of(&req, play(0, 1, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 6 - 1 - 3 - 1, "two hand strikes and 3 burned");
}

/// Blood to Water [tha]: "If the opposing minion is an ally, he or she is
/// burned": the blocking ally to its owner's ash heap; [THA] ("Opposing
/// vampire burns 5 blood") not offered against it.
#[test]
fn blood_to_water_burns_an_opposing_ally() {
    let (mut e, a, _) = duel(&["tha", "tha", "pot"], &[]);
    let ally = e.add_ally(SeatId(1), find_library("Carlton Van Wyk"));
    e.state.minions[ally.index()].blood = 2;
    hand(&mut e, 0, &["Blood to Water", "Relentless Pursuit"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(ally));
    to_round_two(&mut e, a, 1);
    assert!(e.state.minions[ally.index()].in_play, "alive into round 2");
    let req = offering(&mut e, play(0, 0, a));
    assert!(!req.options.contains(&play(0, 2, a)), "{:?}", req.options);
    e.choose(index_of(&req, play(0, 0, a)));
    request(&mut e);
    assert!(!e.state.minions[ally.index()].in_play);
    assert!(ash_has(&e, 1, "Carlton Van Wyk"));
}

/// Blood to Water [THA]: "Opposing vampire burns 5 blood."
#[test]
fn blood_to_water_superior_burns_five() {
    let (mut e, a, x) = duel(&["tha", "tha", "pot"], &[]);
    hand(&mut e, 0, &["Blood to Water", "Relentless Pursuit"]);
    fight(&mut e, a, x);
    to_round_two(&mut e, a, 1);
    pick_when(&mut e, play(0, 2, a));
    request(&mut e);
    assert_eq!(blood(&e, x), 6 - 1 - 5);
}

/// Cauldron of Blood: "Not usable on the first round of combat. [tha]
/// Strike: hand strike at +2 damage." [TOM 19960225] "Additional damage
/// inherits all of the properties of the base damage": aggravated hands
/// (Mantle's lasting) make the +2 aggravated too; not offered in round 1.
#[test]
fn cauldron_of_blood_bonus_inherits_aggravated_hands() {
    let (mut e, a, x) = duel(&["tha", "pot"], &["for"]);
    hand(&mut e, 0, &["Cauldron of Blood", "Relentless Pursuit"]);
    hand(&mut e, 1, &["Skin of Rock"]);
    fight(&mut e, a, x);
    let cauldron =
        |o: &Choice| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == a);
    never_offered_where(&mut e, SeatId(0), WindowLabel::CombatPress, cauldron);
    to_round_two(&mut e, a, 1);
    // Aggravated from round 2 (round 1's would have sent X to torpor).
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::Combat,
        SeatId(0),
    ));
    pick_when(&mut e, play(0, 0, a));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (0, 3), "strength 1 + 2, all aggravated");
}

/// Eldritch Glimmer [THA]: "4R + X ranged damage", X blood burned as it is
/// played: X = 2 costs A its 1 blood and 2 more, 6 damage pending.
#[test]
fn eldritch_glimmer_burns_x_for_x_ranged_damage() {
    let (mut e, a, x) = duel(&["tha", "tha", "pot"], &["for"]);
    hand(&mut e, 0, &["Eldritch Glimmer", "Relentless Pursuit"]);
    hand(&mut e, 1, &["Skin of Rock"]);
    fight(&mut e, a, x);
    let glimmer = |o: &Choice| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == a);
    never_offered_where(&mut e, SeatId(0), WindowLabel::CombatPress, glimmer);
    to_round_two(&mut e, a, 1);
    pick_when(&mut e, play(0, 1, a));
    pick(&mut e, set_x(2));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (6, 0));
    assert_eq!(blood(&e, a), 6 - 1 - 1 - 1 - 2, "round 1, the press, 1 + X");
}

/// Fists of Death [POT]: "+2 strength for the remainder of the combat":
/// A's hand strike 3 in round 1 and again in round 2.
#[test]
fn fists_of_death_lasts_the_combat() {
    let (mut e, a, x) = duel(&["pot", "pot"], &["for"]);
    hand(&mut e, 0, &["Fists of Death", "Relentless Pursuit"]);
    hand(&mut e, 1, &["Skin of Rock"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (3, 0));
    to_round_two(&mut e, a, 0);
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (3, 0));
}

/// Glaser Rounds: [RTR 19941109] "Must wait until the second time a given
/// gun is used in a given combat to play it"; [TOM 19960225] the +2 the
/// gun's own damage; [RTR 19990105] "after strikes have been declared but
/// before they resolve": round 1's .44 Magnum strike offers none, round
/// 2's takes it before resolution, 4 damage pending.
#[test]
fn glaser_rounds_waits_for_the_guns_second_use() {
    let (mut e, a, x) = duel(&["pot"], &["for"]);
    let gun = on(&mut e, ".44 Magnum", a);
    hand(&mut e, 0, &["Glaser Rounds", "Relentless Pursuit"]);
    hand(&mut e, 1, &["Skin of Rock"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 0));
    let glaser = |o: &Choice| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == a);
    never_offered_where(&mut e, SeatId(0), WindowLabel::CombatPress, glaser);
    to_round_two(&mut e, a, 1);
    pick_when(&mut e, weapon(gun, 0));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.window, WindowLabel::CombatBeforeResolution);
    e.choose(index_of(&req, play(0, 0, a)));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (4, 0));
}

/// Glaser Rounds: a use of the gun is any use by its bearer (rules-qa
/// 124.122): its maneuver in round 1, then its strike, offers it in round
/// 1.
#[test]
fn glaser_rounds_counts_a_gun_maneuver_as_a_use() {
    let (mut e, a, x) = duel(&[], &[]);
    let gun = on(&mut e, ".44 Magnum", a);
    hand(&mut e, 0, &["Glaser Rounds"]);
    fight(&mut e, a, x);
    // The maneuver commits the strike to the gun (LSJ 20021028).
    pick_when(&mut e, weapon(gun, 1));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.window, WindowLabel::CombatBeforeResolution);
    assert_eq!(e.combat().expect("combat").round, 1);
}

/// Glaser Rounds: [LSJ 20020425] "Cannot be played on an opponent's
/// weapon"; [RTR 19990105] "after strikes have been declared": X, holding
/// it, is never offered it on A's gun strike.
#[test]
fn glaser_rounds_only_on_the_strikers_own_gun() {
    let (mut e, a, x) = duel(&[], &[]);
    let gun = on(&mut e, ".44 Magnum", a);
    hand(&mut e, 1, &["Glaser Rounds"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 1));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == x),
    );
    assert_eq!(blood(&e, x), 6 - 2, "the gun's strike at long range");
}

/// Lessons in the Steel [aus][for]: "Only usable when damage is
/// successfully inflicted on this vampire. Look at the opposing minion's
/// controller's hand, and this vampire gets an optional press this
/// round": after X's hand strike lands on A, A's player sees seat 1's hand
/// and A presses; once a round.
#[test]
fn lessons_in_the_steel_after_a_landed_hit_peeks_and_presses() {
    let (mut e, a, x) = duel(&["aus", "for"], &[]);
    hand(&mut e, 0, &["Lessons in the Steel", "Lessons in the Steel"]);
    hand(&mut e, 1, &["Dodge"]);
    fight(&mut e, a, x);
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(blood(&e, a), 6 - 1, "after the hit");
    e.choose(index_of(&req, play(0, 0, a)));
    let req = request(&mut e);
    assert!(e.state.peeks.contains(&(SeatId(0), SeatId(1))));
    assert!(!req.options.contains(&play(0, 0, a)), "once a round");
    let req = offering(&mut e, Choice::UsePress);
    assert_eq!(req.window, WindowLabel::CombatPress);
}

/// Lessons in the Steel: not when the hit was fully prevented (rules-qa
/// 116.44: "successfully inflicted").
#[test]
fn lessons_in_the_steel_needs_the_damage_to_land() {
    let (mut e, a, x) = duel(&["aus", "for"], &[]);
    hand(&mut e, 0, &["Lessons in the Steel", "Skin of Rock"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(1, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
    assert_eq!(blood(&e, a), 6);
}

/// Lessons in the Steel [AUS][FOR]: "and that Methuselah discards a card
/// of your choice": seat 0 picks seat 1's Dodge, to seat 1's ash heap.
#[test]
fn lessons_in_the_steel_superior_discards_the_chosen_card() {
    let (mut e, a, x) = duel(&["aus", "aus", "for", "for"], &[]);
    hand(&mut e, 0, &["Lessons in the Steel"]);
    hand(&mut e, 1, &["Dodge", "Form of Mist"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(0));
    e.choose(index_of(&req, Choice::FindCard(find_library("Dodge"))));
    request(&mut e);
    assert!(ash_has(&e, 1, "Dodge"));
    assert_eq!(e.state.seats[1].hand, vec![find_library("Form of Mist")]);
}

/// Mask Empathy [pre]: "Strike: ranged. Burn a retainer employed by the
/// opposing minion": X's Raven Spy, named with the strike, burned.
#[test]
fn mask_empathy_strike_burns_a_retainer() {
    let (mut e, a, x) = duel(&["pre"], &[]);
    let spy = on(&mut e, "Raven Spy", x);
    hand(&mut e, 0, &["Mask Empathy"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play_on(0, 0, a, Target::Card(spy)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[spy.index()].in_play);
    assert!(ash_has(&e, 1, "Raven Spy"));
}

/// Mask Empathy [pre] dodged: the rulebook's "Dodge": "it protects the
/// dodging minion and their possessions from the effects of the opposing
/// strike. Retainers are not protected, however." (owner, rules-qa
/// 113.86): X's Raven Spy burns through X's Dodge.
#[test]
fn mask_empathy_strike_burns_a_retainer_through_a_dodge() {
    let (mut e, a, x) = duel(&["pre"], &[]);
    let spy = on(&mut e, "Raven Spy", x);
    hand(&mut e, 0, &["Mask Empathy"]);
    hand(&mut e, 1, &["Dodge"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play_on(0, 0, a, Target::Card(spy)));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Dodge"), "dodged");
    assert!(!e.state.cards[spy.index()].in_play);
    assert!(ash_has(&e, 1, "Raven Spy"));
}

/// Morlock: "Morlock can strike, ranged: burn a retainer." Dodged, the
/// retainer still burns (rulebook "Dodge": "Retainers are not protected").
#[test]
fn morlock_burns_a_retainer_through_a_dodge() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Morlock"), 6, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let z = e.add_minion(SeatId(1), plain4(1), 6, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    let spy = on(&mut e, "Raven Spy", z);
    hand(&mut e, 1, &["Dodge"]);
    fight(&mut e, m, z);
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(m),
            ability: 0,
            actor: None,
            target: Target::Card(spy),
        },
    );
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Dodge"), "dodged");
    assert!(!e.state.cards[spy.index()].in_play);
}

/// Donnybrook [obf] ("burn equipment or retainer with first strike")
/// dodged: the retainer burns (rulebook "Dodge": "Retainers are not
/// protected"), the equipment, a possession, stays.
#[test]
fn donnybrook_burns_a_retainer_through_a_dodge_not_an_equipment() {
    for (name, burns) in [("Raven Spy", true), ("Ivory Bow", false)] {
        let (mut e, a, x) = duel(&["obf"], &[]);
        e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
        let card = on(&mut e, name, x);
        hand(&mut e, 0, &["Donnybrook"]);
        hand(&mut e, 1, &["Dodge"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play_on(0, 0, a, Target::Card(card)));
        pick_when(&mut e, play(0, 0, x));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(ash_has(&e, 1, "Dodge"), "{name}: dodged");
        assert_eq!(!e.state.cards[card.index()].in_play, burns, "{name}");
    }
}

/// Hálito Negro: "2R damage on the opposing minion and 1R damage on all
/// retainers on the opposing minion": dodged, the vampire is spared, its
/// Raven Spy (1 life) takes the 1 and burns (rulebook "Dodge").
#[test]
fn halito_negro_hits_the_retainers_through_a_dodge() {
    let mut e = game(2);
    let death = textless(
        |c| {
            c.path == Some("Death and the Soul")
                && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("obl"))
        },
        0,
    );
    let a = e.add_minion(SeatId(0), death, 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    let spy = on(&mut e, "Raven Spy", x);
    e.state.cards[spy.index()].counters = 1;
    hand(&mut e, 0, &["Hálito Negro"]);
    hand(&mut e, 1, &["Dodge"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Dodge"), "dodged");
    assert_eq!(blood(&e, x), 4, "the dodger spared");
    assert!(!e.state.cards[spy.index()].in_play, "its retainer is not");
}

/// Mask Empathy [PRE]: "Play before range is determined. This vampire
/// burns 1 blood to burn a retainer employed by the opposing minion."
/// [ANK 20210226] "The card cannot be played if the minion cannot afford
/// to burn the blood": not at 0 blood; at 1, the blood and the retainer.
#[test]
fn mask_empathy_superior_burns_a_retainer_for_a_blood() {
    for start in [0, 1] {
        let (mut e, a, x) = duel(&["pre", "pre"], &[]);
        let spy = on(&mut e, "Raven Spy", x);
        hand(&mut e, 0, &["Mask Empathy"]);
        fight(&mut e, a, x);
        e.state.minions[a.index()].blood = start;
        let superior = play_on(0, 1, a, Target::Card(spy));
        if start == 0 {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, superior);
            continue;
        }
        let req = offering(&mut e, superior);
        assert_eq!(req.window, WindowLabel::CombatBeforeRange);
        e.choose(index_of(&req, superior));
        request(&mut e);
        assert_eq!(blood(&e, a), 0);
        assert!(!e.state.cards[spy.index()].in_play);
    }
}

/// Seat 0's older A (capacity 9 or more) and seat 1's younger X (5 or
/// less), with their second vampires.
fn elder_and_younger(a_levels: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        fixture_vampire(|c| c.capacity >= 9 && no_stealth(c), 0),
        6,
        false,
    );
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 7, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity <= 5, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 7, 2), 4, false);
    e.state.minions[a.index()].gained_levels = a_levels.to_vec();
    (e, a, x)
}

/// Masque of Judas [aus][obf]: "Maneuver with an optional press." [TOM
/// 19960521] "The optional press can only be used during the current
/// round": unused in round 1 (A presses on with Relentless Pursuit), not
/// offered in round 2.
#[test]
fn masque_of_judas_press_only_this_round() {
    let (mut e, a, x) = elder_and_younger(&["aus", "obf", "pot"]);
    hand(
        &mut e,
        0,
        &[
            "Masque of Judas",
            "Relentless Pursuit",
            "Relentless Pursuit",
        ],
    );
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    let req = offering(&mut e, Choice::UsePress);
    assert!(req.options.contains(&play(0, 0, a)), "{:?}", req.options);
    to_round_two(&mut e, a, 0);
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.window, WindowLabel::CombatPress);
    assert!(
        !req.options.contains(&Choice::UsePress),
        "{:?}",
        req.options
    );
}

/// Masque of Judas: "Only usable against an ally or a younger vampire":
/// not against an older one.
#[test]
fn masque_of_judas_not_against_an_older_vampire() {
    let (mut e, a, x) = elder_and_younger(&[]);
    e.state.minions[x.index()].gained_levels = vec!["aus", "obf"];
    hand(&mut e, 1, &["Masque of Judas"]);
    fight(&mut e, a, x);
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == x),
    );
}

/// Masque of Judas [AUS][OBF]: "The opposing minion's controller discards
/// 2 cards at random. Those cards are not replaced until after combat":
/// seat 1's hand 2 short during the combat, drawn back after it.
#[test]
fn masque_of_judas_superior_holds_the_discards_until_combat_ends() {
    let (mut e, a, x) = elder_and_younger(&["aus", "aus", "obf", "obf", "pot"]);
    hand(&mut e, 0, &["Masque of Judas", "Relentless Pursuit"]);
    e.state.seats[1].library = vec![find_library("Form of Mist"); 12].into();
    fight(&mut e, a, x);
    let held = e.state.seats[1].hand.len();
    pick_when(&mut e, play(0, 1, a));
    offering(&mut e, play(0, 0, a));
    assert!(e.combat().is_some());
    assert_eq!(e.state.seats[1].hand.len(), held - 2);
    assert_eq!(e.state.seats[1].ash.len(), 2);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].hand.len(), held);
}

/// Personal Scourge [dem]: "This damage cannot be prevented by equipment or
/// cards requiring Fortitude": X's Flak Jacket and Skin of Rock [for] are
/// not offered against it and its 1 lands; against A's hand strike, both
/// are.
#[test]
fn personal_scourge_bars_equipment_and_fortitude_prevention() {
    for scourge in [true, false] {
        let (mut e, a, x) = duel(&["dem"], &["for"]);
        let flak = on(&mut e, "Flak Jacket", x);
        hand(&mut e, 0, &["Personal Scourge"]);
        hand(&mut e, 1, &["Skin of Rock"]);
        fight(&mut e, a, x);
        if !scourge {
            let req = offering(&mut e, play(0, 0, x));
            assert!(req.options.iter().any(|o| uses_card(o, flak)), "{req:?}");
            continue;
        }
        pick_when(&mut e, play(0, 0, a));
        never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
            *o == play(0, 0, x) || uses_card(o, flak)
        });
        assert_eq!(blood(&e, x), 6 - 1);
    }
}

/// Personal Scourge: [LSJ 20040928] "If the opponent dodges this strike,
/// his weapons are unaffected and do damage this round": X dodges (Side
/// Strike), then strikes again with its .44 Magnum (Quickness) for 2.
#[test]
fn personal_scourge_dodged_leaves_the_weapons() {
    let (mut e, a, x) = duel(&["dem"], &["cel"]);
    let gun = on(&mut e, ".44 Magnum", x);
    hand(&mut e, 0, &["Personal Scourge"]);
    hand(&mut e, 1, &["Side Strike", "Quickness"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, weapon(gun, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 6 - 2, "the gun's 2");
    assert_eq!(blood(&e, x), 6, "dodged");
}

/// [LSJ 19980216] "Does not protect the target from self-inflicted damage
/// from a weapon" (Personal Scourge, Blood Rage, Blood Fury, Soul Burn):
/// X's Zip Gun strike deals A nothing, its 1 environmental damage on X
/// still lands (rules-qa 64.10, 116.117).
#[test]
fn personal_scourge_leaves_the_zip_guns_own_damage() {
    for (card, levels, cost, dealt) in [
        ("Personal Scourge", "dem", 0, 1),
        ("Blood Rage", "tha", 0, 1),
        ("Blood Fury", "tha", 1, 2),
        ("Soul Burn", "tha", 1, 1),
    ] {
        let (mut e, a, x) = duel(&[levels], &[]);
        hand(&mut e, 0, &[card]);
        hand(&mut e, 1, &["Zip Gun"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, 0, x));
        request(&mut e);
        let zip = CardId(
            e.state
                .cards
                .iter()
                .position(|c| c.card == find_library("Zip Gun") && c.in_play)
                .expect("Zip Gun on X") as u32,
        );
        pick_when(&mut e, play(0, 0, a));
        pick_when(&mut e, weapon(zip, 0));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, a), 6 - cost, "{card}: the gun harmless");
        assert_eq!(blood(&e, x), 6 - dealt - 1, "{card}: the gun's own 1");
    }
}

/// Personal Scourge [DEM]: "with 1 optional press, only usable to end
/// combat" [TOM 19960521]: X presses on (Apportation [tha]), A ends the
/// combat with it.
#[test]
fn personal_scourge_superior_press_ends_combat() {
    let (mut e, a, x) = duel(&["dem", "dem"], &["tha"]);
    hand(&mut e, 0, &["Personal Scourge"]);
    hand(&mut e, 1, &["Apportation"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, x));
    let req = expect(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert!(req.options.contains(&Choice::UsePress), "{:?}", req.options);
    e.choose(index_of(&req, Choice::UsePress));
    request(&mut e);
    assert!(e.combat().is_none(), "ended in round 1");
}

/// Ritual of the Bitter Rose: [ANK 20201228] "Can be played after a
/// diablerie, before the blood hunt": A diablerizes X (torpid, 4 blood);
/// before the hunt, each ready vampire of seat 0 gains 4 (the victim's
/// blood before it was taken, LSJ 20020411), the torpid one nothing.
#[test]
fn ritual_of_the_bitter_rose_after_a_diablerie_feeds_the_victims_blood() {
    let mut e = game(2);
    let big = |c: &zvtes::crypt::CryptCard| c.capacity >= 9;
    let a = e.add_minion(SeatId(0), textless(big, 0), 1, false);
    let b = e.add_minion(SeatId(0), textless(big, 1), 1, false);
    let t = e.add_minion(SeatId(0), textless(big, 2), 1, true);
    let x = e.add_minion(
        SeatId(1),
        textless(|c| (4..=5).contains(&c.capacity), 0),
        4,
        true,
    );
    e.add_minion(SeatId(1), textless(big, 3), 4, false);
    hand(&mut e, 0, &["Ritual of the Bitter Rose"]);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: ActionKind::Diablerie,
            target: Some(x),
        },
    );
    let req = offering(&mut e, play(0, 0, a));
    assert!(e.referendum().is_none(), "before the hunt");
    e.choose(index_of(&req, play(0, 0, a)));
    request(&mut e);
    assert_eq!(
        blood(&e, a),
        1 + 4 + 4,
        "the diablerie's 4 and the Ritual's"
    );
    assert_eq!(blood(&e, b), 1 + 4);
    assert_eq!(blood(&e, t), 1, "torpid");
}

/// Ritual of the Bitter Rose in combat: [LSJ 19970410] "he will have zero
/// blood when he goes to the ash heap … Ritual will give zero": X, empty,
/// burned by Song in the Dark [ANI]'s 2 aggravated (wounded by the first,
/// burned by the second); still offered, a legal no-op.
#[test]
fn ritual_of_the_bitter_rose_gives_nothing_for_an_empty_burn() {
    let (mut e, a, x) = duel(&["ani", "ani"], &[]);
    hand(
        &mut e,
        0,
        &["Song in the Dark", "Ritual of the Bitter Rose"],
    );
    fight(&mut e, a, x);
    e.state.minions[x.index()].blood = 0;
    pick_when(&mut e, play(0, 1, a));
    let req = offering(&mut e, play(0, 1, a));
    assert_eq!(req.window, WindowLabel::CombatEndOfRound);
    assert!(!e.state.minions[x.index()].in_play, "burned");
    let before = blood(&e, a);
    e.choose(index_of(&req, play(0, 1, a)));
    request(&mut e);
    assert_eq!(blood(&e, a), before);
}

/// Ritual of the Bitter Rose on a vampire burned in combat with blood left
/// (not a diablerie): Touch of Oblivion [OBL] sends X (3 blood) to torpor,
/// Decapitate [POT] burns it instead, its 3 still on it: "the blood on
/// that vampire" as it burns (LSJ 19970410), 3 to each ready vampire of
/// seat 0.
#[test]
fn ritual_of_the_bitter_rose_pays_the_blood_of_a_vampire_burned_in_combat() {
    let (mut e, a, x) = duel(&["obl", "obl", "pot", "pot"], &[]);
    e.state.minions[x.index()].blood = 3;
    let b = e.state.minions_of(SeatId(0)).find(|&m| m != a).expect("b");
    e.state.minions[b.index()].blood = 1;
    hand(
        &mut e,
        0,
        &[
            "Touch of Oblivion",
            "Decapitate",
            "Ritual of the Bitter Rose",
        ],
    );
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 2, a));
    let req = offering(&mut e, play(0, 1, a));
    assert!(!e.state.minions[x.index()].in_play, "burned");
    assert_eq!(req.window, WindowLabel::CombatEndOfRound);
    e.choose(index_of(&req, play(0, 1, a)));
    request(&mut e);
    assert_eq!(blood(&e, b), 1 + 3);
}

/// Ritual of the Bitter Rose, two copies on one diablerie in combat
/// (rules-qa 126.164): Coma [dem] sends X to torpor with its 3 blood, Amaranth
/// diablerizes it; one copy after the diablerie, before the hunt, the
/// other at the round's end: each gives 3.
#[test]
fn ritual_of_the_bitter_rose_two_copies_on_one_diablerie_both_give() {
    let (mut e, a, x) = duel(&["dem"], &[]);
    e.state.minions[x.index()].blood = 3;
    let b = e.state.minions_of(SeatId(0)).find(|&m| m != a).expect("b");
    e.state.minions[b.index()].blood = 1;
    hand(
        &mut e,
        0,
        &[
            "Coma",
            "Amaranth",
            "Ritual of the Bitter Rose",
            "Ritual of the Bitter Rose",
        ],
    );
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    let req = offering(&mut e, play(0, 1, a));
    assert!(!e.state.minions[x.index()].in_play, "diablerized");
    assert!(e.combat().is_some() && e.referendum().is_none());
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    let b0 = blood(&e, b);
    e.choose(index_of(&req, play(0, 1, a)));
    request(&mut e);
    assert_eq!(blood(&e, b), b0 + 3);
    let req = offering(&mut e, play(0, 1, a));
    e.choose(index_of(&req, Choice::Pass));
    let req = offering(&mut e, play(0, 1, a));
    assert_eq!(req.window, WindowLabel::CombatEndOfRound);
    e.choose(index_of(&req, play(0, 1, a)));
    request(&mut e);
    assert_eq!(blood(&e, b), b0 + 6);
}

/// Shattering Blow: [RTR 19980928] "Cannot be used if there is no
/// equipment to destroy": not offered against a bare X; X's Flak Jacket,
/// named with the strike, burned.
#[test]
fn shattering_blow_needs_an_equipment_to_destroy() {
    for equipped in [false, true] {
        let (mut e, a, x) = duel(&["pot"], &[]);
        let flak = equipped.then(|| on(&mut e, "Flak Jacket", x));
        hand(&mut e, 0, &["Shattering Blow"]);
        fight(&mut e, a, x);
        let Some(flak) = flak else {
            never_offered_where(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == a),
            );
            continue;
        };
        pick_when(&mut e, play_on(0, 0, a, Target::Card(flak)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.cards[flak.index()].in_play);
        assert!(ash_has(&e, 1, "Flak Jacket"));
    }
}

/// Shattering Blow [POT]: "destroy equipment with first strike": X's .44
/// Magnum burned before its strike resolves, A untouched.
#[test]
fn shattering_blow_superior_burns_the_gun_before_it_fires() {
    let (mut e, a, x) = duel(&["pot", "pot"], &[]);
    let gun = on(&mut e, ".44 Magnum", x);
    hand(&mut e, 0, &["Shattering Blow"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play_on(0, 1, a, Target::Card(gun)));
    pick_when(&mut e, weapon(gun, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[gun.index()].in_play);
    assert_eq!(blood(&e, a), 6);
}

/// Shotgun Ritual [tha]: "Any strike requiring Blood Sorcery that is not
/// usable during the first round of combat can be played by this vampire
/// during the first round": Cauldron of Blood, Eldritch Glimmer and Walk
/// of Flame offered to A in round 1 after it, not without it; Blood to
/// Water (no strike) still not; X's Walk of Flame not.
#[test]
fn shotgun_ritual_lets_round_one_sorcery_strikes() {
    for shotgun in [true, false] {
        let (mut e, a, x) = duel(&["tha"], &["tha"]);
        hand(
            &mut e,
            0,
            &[
                "Shotgun Ritual",
                "Cauldron of Blood",
                "Eldritch Glimmer",
                "Walk of Flame",
                "Blood to Water",
            ],
        );
        hand(&mut e, 1, &["Walk of Flame"]);
        fight(&mut e, a, x);
        if !shotgun {
            never_offered_where(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                |o| matches!(o, Choice::Play { card: 1..=3, actor: Some(m), .. } if *m == a),
            );
            continue;
        }
        pick_when(&mut e, play(0, 0, a));
        let req = expect(&mut e, SeatId(0), WindowLabel::CombatStrike);
        for card in 0..3 {
            assert!(req.options.contains(&play(card, 0, a)), "{card}: {req:?}");
        }
        e.choose(index_of(&req, Choice::HandStrike));
        never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
            matches!(o, Choice::Play { card: 3, actor: Some(m), .. } if *m == a)
                || matches!(o, Choice::Play { actor: Some(m), .. } if *m == x)
        });
    }
}

/// Shotgun Ritual [THA]: "with an optional press to continue" [TOM
/// 19960521]: used in round 1, the combat goes on to round 2.
#[test]
fn shotgun_ritual_superior_continue_press_this_round() {
    let (mut e, a, x) = duel(&["tha", "tha"], &[]);
    hand(&mut e, 0, &["Shotgun Ritual", "Walk of Flame"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, Choice::UsePress);
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.window, WindowLabel::CombatStrike);
    assert_eq!(e.combat().expect("combat").round, 2);
}

/// Song in the Dark [ANI]: "Strike: 2 aggravated damage", a close-range
/// strike: 2 aggravated pending on X; at long range (X's .44 Magnum
/// maneuver), chosen, it deals nothing (rules-digest D23).
#[test]
fn song_in_the_dark_superior_is_aggravated_and_close_only() {
    let (mut e, a, x) = duel(&["ani", "ani"], &["for"]);
    hand(&mut e, 0, &["Song in the Dark"]);
    hand(&mut e, 1, &["Skin of Rock"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (0, 2));

    let (mut e, a, x) = duel(&["ani", "ani"], &[]);
    let gun = on(&mut e, ".44 Magnum", x);
    hand(&mut e, 0, &["Song in the Dark"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 1));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 6, "a close strike at long range: nothing");
}

/// A fixture vampire of capacity 4 or more, the `nth`.
fn plain4(nth: usize) -> u16 {
    textless(|c| c.capacity >= 4, nth)
}

/// Vampire `m`'s printed ability `ability`, used by `actor` at `target`.
fn printed_use(m: MinionId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor,
        target,
    }
}

/// Any use of vampire `m`'s printed ability `ability`.
fn uses_printed(o: &Choice, m: MinionId, ability: u8) -> bool {
    matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), ability: i, .. }
        if *v == m && *i == ability)
}

/// Laurent de Valois ("Camarilla."), Mariano Pomposo ("Sabbat: Black
/// Hand.") and Olivia ("Sabbat bishop."): no definition, their traits
/// generated; Mariano's Black Hand kept through a sect change ([LSJ
/// 20070322] [ANK 20180807]). Marino Reymundo Vásquez ("Sabbat Archbishop
/// of Barcelona: +1 bleed.") bleeds for 2.
#[test]
fn the_round_34_vampires_laurent_to_olivia_carry_their_printed_traits() {
    for name in ["Laurent de Valois", "Mariano Pomposo", "Olivia"] {
        assert!(zvtes::cards::vampire(find_card(name)).is_none(), "{name}");
    }
    let c = |n: &str| &CRYPT[find_card(n) as usize];
    assert_eq!(c("Laurent de Valois").sect, Some(Sect::Camarilla));
    assert_eq!(c("Laurent de Valois").title, Title::None);
    assert_eq!(c("Olivia").title, Title::Bishop);
    assert_eq!(
        c("Marino Reymundo Vásquez").title,
        Title::Archbishop("Barcelona")
    );
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mariano Pomposo"), 4, false);
    e.state.minions[m.index()].sect_override = Some(Sect::Anarch);
    assert!(e.state.is_black_hand(m), "kept through a sect change");
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Marino Reymundo Vásquez"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    pick(&mut e, bleed(m));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

/// Makarios: "As a Ⓓ action, Makarios can put a seduction counter on a
/// Ventrue or Brujah controlled by your prey": a Ventrue of his prey is a
/// target, a Brujah antitribu of it and a Ventrue of another Methuselah
/// are not; the counter put on, the action directed at the prey.
#[test]
fn makarios_seduces_a_ventrue_of_his_prey() {
    let mut e = game(3);
    let mak = e.add_minion(SeatId(0), find_card("Makarios, The Seducer"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let b = e.add_minion(
        SeatId(1),
        fixture_vampire(|c| c.clan == "Brujah antitribu" && c.capacity >= 4, 0),
        4,
        false,
    );
    let w = e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    let req = request(&mut e);
    let at = |t| printed_use(mak, 0, Some(mak), Target::Minion(t));
    assert!(req.options.contains(&at(v)), "{:?}", req.options);
    assert!(!req.options.contains(&at(b)) && !req.options.contains(&at(w)));
    e.choose(index_of(&req, at(v)));
    request(&mut e);
    assert_eq!(
        e.action().expect("action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[v.index()].seduction.0, 1);
}

/// Makarios: "Makarios cannot be blocked by a vampire with a seduction
/// counter. That vampire may burn the seduction counter as an action": the
/// seduced Ventrue is not offered a block of his bleed, the other is; in
/// its own turn it burns the counter (an undirected action of its own).
#[test]
fn a_seduced_vampire_cannot_block_makarios_until_it_burns_the_counter() {
    let mut e = game(2);
    let mak = e.add_minion(SeatId(0), find_card("Makarios, The Seducer"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let o = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[v.index()].seduction.0 = 1;
    pick(&mut e, bleed(mak));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(!req.options.contains(&block(v)), "{:?}", req.options);
    assert!(req.options.contains(&block(o)));
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2, "bleed 2");
    let burn = printed_use(mak, 1, Some(v), Target::None);
    pick_when(&mut e, burn);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[v.index()].seduction.0, 0);
}

/// Mateusz Gryzbowsky: "At long range, Mateusz's strikes inflict 1 less
/// damage. +2 strength": his .44 Magnum at close range 2, at long range
/// (its maneuver) 1; his hand strike 3. [LSJ 20040518] the strike's
/// amount.
#[test]
fn mateusz_strikes_one_less_at_long_range() {
    for (maneuver, want) in [(false, 2), (true, 1)] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card("Mateusz Gryzbowsky"), 4, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let x = e.add_minion(SeatId(1), plain4(1), 4, false);
        e.state.minions[x.index()].gained_levels = vec!["for"];
        e.add_minion(SeatId(1), plain4(2), 4, false);
        let gun = on(&mut e, ".44 Magnum", m);
        hand(&mut e, 1, &["Skin of Rock"]);
        fight(&mut e, m, x);
        if maneuver {
            pick_when(&mut e, weapon(gun, 1));
        } else {
            pick_when(&mut e, weapon(gun, 0));
        }
        offering(&mut e, play(0, 0, x));
        assert_eq!(pending(&e, 1), (want, 0), "maneuver: {maneuver}");
    }
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mateusz Gryzbowsky"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(1), plain4(2), 4, false);
    hand(&mut e, 1, &["Skin of Rock"]);
    fight(&mut e, m, x);
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (3, 0));
}

/// Nephandus's reduction and Mateusz's come off the whole strike, its ammo
/// in ([LSJ 20100519] [LSJ 20040518]): Mateusz's 1R Saturday-Night Special
/// loaded with Manstopper Rounds (+1) at long range on a blocking
/// Nephandus: 1 + 1 - 1 - 1, none.
#[test]
fn reductions_come_off_the_loaded_strike() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mateusz Gryzbowsky"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let gun = on(&mut e, "Saturday-Night Special", m);
    hand(&mut e, 0, &["Manstopper Rounds"]);
    let neph = e.add_ally(SeatId(1), find_library("Nephandus"));
    e.state.minions[neph.index()].blood = 3;
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(neph));
    pick_when(&mut e, weapon(gun, 1));
    pick_when(&mut e, play(0, 0, m));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, neph), 3, "none inflicted");
}

/// The first request offering vampire `m`'s printed ability `ability`, the
/// fallback taken until then; that use is chosen.
fn use_printed(e: &mut Engine, m: MinionId, ability: u8) {
    for _ in 0..300 {
        let req = request(e);
        if let Some(i) = req.options.iter().position(|o| uses_printed(o, m, ability)) {
            e.choose(i);
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("{m:?}'s ability {ability} never offered");
}

/// A Sabbat fixture vampire of capacity 4 or more, untitled, the `nth`.
fn sabbat4(nth: usize) -> u16 {
    textless(|c| c.sect == Some(Sect::Sabbat) && c.capacity >= 4, nth)
}

/// Matteus, Flesh Sculptor: "Matteus may burn 1 additional blood to play
/// an action card that requires an archbishop or cardinal": a priscus, he
/// calls Deploy the Hand ("Requires a ready archbishop, cardinal or
/// regent"). [LSJ 20090722] "When does Matteus burn the blood …? — When he
/// wants to play the action card": gone before any block is declared, kept
/// once the action is blocked (at -1 stealth, the political action's +1
/// off; the combat's hand strikes elided).
#[test]
fn matteus_calls_deploy_the_hand_for_a_blood_more() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Matteus, Flesh Sculptor"), 3, false);
    e.state.minions[m.index()].timed.push((
        zvtes::cards::Lasting::Stealth(-1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let b = e.add_minion(SeatId(1), plain4(1), 4, false);
    hand(&mut e, 0, &["Deploy the Hand"]);
    pick(&mut e, play(0, 0, m));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert_eq!(blood(&e, m), 3 - 1, "burned as he plays the card");
    e.choose(index_of(&req, block(b)));
    request(&mut e);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert_eq!(
        blood(&e, m),
        3 - 1 - 1,
        "kept once blocked, the hand strike's 1 off"
    );
}

/// Matteus: [LSJ 20020223] "Can Matteus burn a blood to play a card that
/// requires a Cardinal/Archbishop but that he would be able to play
/// anyway? … Yes … Once per card": an archbishop himself, he is asked;
/// paying burns the blood as he plays the card, passing does not.
#[test]
fn matteus_may_pay_for_a_title_he_holds() {
    for pay in [true, false] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card("Matteus, Flesh Sculptor"), 3, false);
        e.state.minions[m.index()].granted_title = Some(Title::Archbishop("Paris"));
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        hand(&mut e, 0, &["Deploy the Hand"]);
        pick(&mut e, play(0, 0, m));
        let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
        assert_eq!(req.options, vec![Choice::Pass, Choice::ChooseMinion(m)]);
        let choice = if pay {
            Choice::ChooseMinion(m)
        } else {
            Choice::Pass
        };
        e.choose(index_of(&req, choice));
        expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
        assert_eq!(blood(&e, m), 3 - u8::from(pay), "pay: {pay}");
    }
}

/// Matteus and an either/or requirement: Auto-da-fé ("Requires a cardinal,
/// regent or Inquisitor") offered to him, for the blood.
#[test]
fn matteus_meets_an_either_or_title_requirement() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Matteus, Flesh Sculptor"), 3, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    hand(&mut e, 0, &["Auto-da-fé"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, m)), "{:?}", req.options);
}

/// Matteus: "He may burn 1 blood to get +1 intercept against an
/// archbishop, cardinal or regent attempting a political action." [ANK
/// 20180307-2] "Can use his ability to get intercept multiple times during
/// the same action": an archbishop's Deploy the Hand at +2 stealth or
/// more, blocked for a blood a point.
#[test]
fn matteus_burns_twice_for_intercept_against_an_archbishops_political_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), sabbat4(0), 4, false);
    e.state.minions[a.index()].granted_title = Some(Title::Archbishop("Paris"));
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Stealth(2),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let m = e.add_minion(SeatId(1), find_card("Matteus, Flesh Sculptor"), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    hand(&mut e, 0, &["Deploy the Hand"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, block(m));
    let stealth = e.action().expect("action").stealth(&e.state);
    assert!(stealth >= 2, "{stealth}");
    for _ in 0..stealth {
        use_printed(&mut e, m, 0);
    }
    assert_eq!(blood(&e, m), 4 - stealth as u8, "once a point");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
}

/// Miguel Santo Domingo: "During a bleed action, Miguel can burn 1 blood
/// to give another acting Sabbat vampire you control +1 bleed." [ANK
/// 20231124] [LSJ 19970718] before resolution: locked, he gives a Sabbat
/// vampire's bleed +1 once; not on his own bleed.
#[test]
fn miguel_gives_another_acting_sabbat_vampire_a_bleed_once() {
    let mut e = game(2);
    let miguel = e.add_minion(SeatId(0), find_card("Miguel Santo Domingo"), 4, false);
    e.state.minions[miguel.index()].locked = true;
    let s = e.add_minion(SeatId(0), sabbat4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    pick(&mut e, bleed(s));
    use_printed(&mut e, miguel, 0);
    let req = request(&mut e);
    assert!(
        !req.options.iter().any(|o| uses_printed(o, miguel, 0)),
        "once an action"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, miguel), 3);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
    let mut e = game(2);
    let miguel = e.add_minion(SeatId(0), find_card("Miguel Santo Domingo"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    pick(&mut e, bleed(miguel));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_printed(o, miguel, 0)
    });
}

/// Miller Delmardigan: "Cards that require Celerity [cel] cost Miller one
/// less blood. +1 strength": Fleetness (1 blood) free; his hand strike 2.
#[test]
fn miller_pays_a_blood_less_for_celerity_cards() {
    let mut e = game(2);
    let m = e.add_minion(
        SeatId(0),
        find_card("Miller Delmardigan, Teacher of Bahari"),
        4,
        false,
    );
    e.state.minions[m.index()].gained_levels = vec!["cel"];
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    hand(&mut e, 0, &["Fleetness"]);
    pick(&mut e, play(0, 0, m));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, m), 4);
    let mut e = game(2);
    let m = e.add_minion(
        SeatId(0),
        find_card("Miller Delmardigan, Teacher of Bahari"),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(1), plain4(2), 4, false);
    hand(&mut e, 1, &["Skin of Rock"]);
    fight(&mut e, m, x);
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (2, 0));
}

/// Muaziz: "Blood hunts cannot be called on Muaziz. Muaziz gets +1 stealth
/// on each of her actions": her bleed at stealth 1; her diablerie calls no
/// blood hunt (seat 1 holding the Edge could vote in one), another
/// vampire's does.
#[test]
fn muaziz_diablerizes_with_no_blood_hunt_and_acts_at_plus_one_stealth() {
    let mut e = game(2);
    let mz = e.add_minion(
        SeatId(0),
        find_card("Muaziz, Archon of Ulugh Beg"),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    pick(&mut e, bleed(mz));
    request(&mut e);
    assert_eq!(e.action().expect("bleed").stealth(&e.state), 1);
    for muaziz in [true, false] {
        let mut e = game(2);
        let d = if muaziz {
            e.add_minion(
                SeatId(0),
                find_card("Muaziz, Archon of Ulugh Beg"),
                4,
                false,
            )
        } else {
            e.add_minion(SeatId(0), plain4(0), 4, false)
        };
        e.add_minion(SeatId(0), plain4(1), 4, false);
        let t = e.add_minion(SeatId(1), textless(|c| c.capacity <= 4, 0), 1, true);
        e.add_minion(SeatId(1), plain4(2), 4, false);
        e.state.seats[1].has_edge = true;
        pick(
            &mut e,
            Choice::TakeAction {
                minion: d,
                action: ActionKind::Diablerie,
                target: Some(t),
            },
        );
        let mut called = false;
        for _ in 0..50 {
            let req = request(&mut e);
            if req.window == WindowLabel::BloodHuntPolling {
                called = true;
                break;
            }
            if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                break;
            }
            e.choose(fallback(&req));
        }
        assert!(!e.state.minions[t.index()].in_play, "diablerized");
        assert_eq!(called, !muaziz, "muaziz: {muaziz}");
    }
}

/// Muaziz: [TOM 19960604] "Her ability does not apply to the Blood Hunt
/// card": a prince's Blood Hunt is put on her, the card itself in play.
#[test]
fn blood_hunt_card_still_lands_on_muaziz() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), plain4(0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    e.add_minion(SeatId(0), plain4(1), 4, false);
    let mz = e.add_minion(
        SeatId(1),
        find_card("Muaziz, Archon of Ulugh Beg"),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain4(2), 4, false);
    hand(&mut e, 0, &["Blood Hunt"]);
    pick(&mut e, play_on(0, 0, p, Target::Minion(mz)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let bh = find_library("Blood Hunt");
    let c = e
        .state
        .cards
        .iter()
        .find(|c| c.card == bh && c.host == Some(mz))
        .expect("on Muaziz");
    assert!(c.in_play && !c.effect_only);
    assert!(!ash_has(&e, 0, "Blood Hunt"));
}

/// Nahir: "you can use a master phase action and burn 1 pool to put 1
/// research counter on her. You get +1 hand size for each research
/// counter." [ANK 20191218] "Can use her ability multiple times during the
/// same master phase if there are multiple master phase actions"; "The
/// hand size is still +X if Nahir is in torpor."
#[test]
fn nahir_buys_research_counters_with_each_master_phase_action() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nahir"), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let size = e.state.hand_size(SeatId(0));
    e.at_master_phase();
    e.state.seats[0].mpa = 2;
    use_printed(&mut e, n, 0);
    use_printed(&mut e, n, 0);
    request(&mut e);
    assert_eq!(e.state.minions[n.index()].research.0, 2);
    assert_eq!(e.state.seats[0].pool, 10 - 2);
    assert_eq!(e.state.hand_size(SeatId(0)), size + 2);
    e.state.minions[n.index()].in_torpor = true;
    assert_eq!(e.state.hand_size(SeatId(0)), size + 2, "in torpor too");
}

/// Nakova: "If Nakova is burned in combat, the opposing minion takes 4
/// damage (damage not preventable)." [LSJ 19980427] during combat:
/// empty, she blocks A and is burned by Song in the Dark [ANI]'s 2
/// aggravated; A takes her hand strike's 1 and the 4.
#[test]
fn nakova_burned_in_combat_deals_four_unpreventable_damage() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        fixture_vampire(|c| c.capacity >= 9 && no_stealth(c), 0),
        9,
        false,
    );
    e.state.minions[a.index()].gained_levels = vec!["ani", "ani"];
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let nk = e.add_minion(
        SeatId(1),
        find_card("Nakova, Advocate of Golconda"),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain4(1), 4, false);
    hand(&mut e, 0, &["Song in the Dark"]);
    fight(&mut e, a, nk);
    e.state.minions[nk.index()].blood = 0;
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[nk.index()].in_play, "burned");
    assert_eq!(blood(&e, a), 9 - 2 - 1 - 4);
}

/// Nakova with her burn replaced: [LSJ 19980427] "since this happens
/// 'instead' of burning her, the opposing minion takes no damage". Seat 0's
/// Charigger, The Axe removes her from the game instead of her burn: A
/// takes none of the 4.
#[test]
fn nakova_replaced_burn_deals_nothing() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        fixture_vampire(|c| c.capacity >= 9 && no_stealth(c), 0),
        9,
        false,
    );
    e.state.minions[a.index()].gained_levels = vec!["ani", "ani"];
    e.add_minion(SeatId(0), plain4(0), 4, false);
    in_play(&mut e, 0, "Charigger, The Axe", Some(a));
    let nk = e.add_minion(
        SeatId(1),
        find_card("Nakova, Advocate of Golconda"),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain4(1), 4, false);
    hand(&mut e, 0, &["Song in the Dark"]);
    fight(&mut e, a, nk);
    e.state.minions[nk.index()].blood = 0;
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[nk.index()].in_play, "removed");
    assert!(!e.state.seats[1]
        .ash_crypt
        .contains(&find_card("Nakova, Advocate of Golconda")));
    assert_eq!(blood(&e, a), 9 - 2 - 1, "none of her 4");
}

/// Mina Grotius and a contest yield: "Whenever a vampire controlled by your
/// predator or prey is burned, Mina gains 3 blood": seat 1 (her prey)
/// yields its contested copy, a vampire no one controls, not burned from
/// play (advanced rules §1.13.3; rules-qa 124.94): she gains nothing.
#[test]
fn mina_grotius_gains_nothing_from_a_yielded_contest() {
    let mut e = game(3);
    let mina = e.add_minion(SeatId(0), find_card("Mina Grotius"), 2, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let unique = textless(|c| c.capacity >= 4, 3);
    let y = e.add_minion(SeatId(1), unique, 3, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let k = e.add_minion(SeatId(2), unique, 3, false);
    e.add_minion(SeatId(2), plain4(2), 4, false);
    for m in [y, k] {
        e.state.minions[m.index()].contested = true;
    }
    let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    let yielded = *req
        .options
        .iter()
        .find(|o| matches!(o, Choice::YieldContest(_)))
        .expect("the yield");
    e.choose(index_of(&req, yielded));
    request(&mut e);
    assert!(!e.state.minions[y.index()].in_play, "yielded");
    assert_eq!(blood(&e, mina), 2);
}

/// Olga Triminov: "Black Hand vampires can steal Olga as a Ⓓ action that
/// costs 1 pool." [LSJ 20070322] [ANK 20180807] the trait kept through a
/// sect change: seat 0's Black Hand vampire, made Anarch, takes her for
/// 1 pool; a vampire with no Black Hand is not offered it.
#[test]
fn olga_is_stolen_by_a_black_hand_vampire_for_a_pool() {
    let mut e = game(2);
    let bh = e.add_minion(
        SeatId(0),
        fixture_vampire(|c| c.black_hand && c.capacity >= 4, 0),
        4,
        false,
    );
    e.state.minions[bh.index()].sect_override = Some(Sect::Anarch);
    let other = e.add_minion(SeatId(0), plain4(0), 4, false);
    let olga = e.add_minion(SeatId(1), find_card("Olga Triminov"), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let req = request(&mut e);
    let steal = |m| printed_use(olga, 0, Some(m), Target::Minion(olga));
    assert!(req.options.contains(&steal(bh)), "{:?}", req.options);
    assert!(!req.options.contains(&steal(other)));
    e.choose(index_of(&req, steal(bh)));
    request(&mut e);
    assert_eq!(
        e.action().expect("steal").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[olga.index()].owner, SeatId(0));
    assert_eq!(e.state.seats[0].pool, 10 - 1);
}

/// Piotr Andreikov and Roger Farnsworth ("Sabbat: Black Hand."), Raziya
/// Samater and Regilio ("Camarilla."), Sabrina and Sister Evelyn
/// ("Sabbat."): no definition, their traits generated; the Black Hand kept
/// through a sect change ([LSJ 20070322] [ANK 20180807]).
#[test]
fn the_round_34_vampires_piotr_to_sister_evelyn_carry_their_printed_traits() {
    for (name, sect, black_hand) in [
        ("Piotr Andreikov", Sect::Sabbat, true),
        ("Roger Farnsworth", Sect::Sabbat, true),
        ("Raziya Samater", Sect::Camarilla, false),
        ("Regilio, The Seeker of Akhenaten", Sect::Camarilla, false),
        ("Sabrina", Sect::Sabbat, false),
        ("Sister Evelyn", Sect::Sabbat, false),
    ] {
        let card = find_card(name);
        assert!(zvtes::cards::vampire(card).is_none(), "{name}");
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), card, 2, false);
        assert_eq!(e.state.sect_of(m), Some(sect), "{name}");
        assert_eq!(e.state.title_of(m), Title::None, "{name}");
        e.state.minions[m.index()].sect_override = Some(Sect::Independent);
        assert_eq!(e.state.is_black_hand(m), black_hand, "{name}");
    }
}

/// Radeyah: "As a Ⓓ action, Radeyah can enter combat with any Ministry
/// controlled by another Methuselah": seat 1's Ministry, not its Brujah nor
/// her own Methuselah's Ministry.
#[test]
fn radeyah_enters_combat_with_another_methuselahs_ministry() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Radeyah"), 4, false);
    let ministry = |n| textless(|c| c.clan == "Ministry", n);
    let own = e.add_minion(SeatId(0), ministry(0), 4, false);
    let theirs = e.add_minion(SeatId(1), ministry(1), 4, false);
    let other = e.add_minion(SeatId(1), textless(|c| c.clan == "Brujah", 0), 4, false);
    let req = request(&mut e);
    let at = |m| printed_use(r, 0, Some(r), Target::Minion(m));
    assert!(req.options.contains(&at(theirs)), "{:?}", req.options);
    assert!(!req.options.contains(&at(other)), "not a Ministry");
    assert!(!req.options.contains(&at(own)), "her own Methuselah's");
    e.notes = Some(Vec::new());
    pick(&mut e, at(theirs));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let begins = noted_combats(&mut e).into_iter().find_map(|n| match n {
        zvtes::combat::CombatNote::Begins { minions } => Some(minions),
        _ => None,
    });
    assert_eq!(begins, Some([r, theirs]));
}

/// The pool seat 1 loses to `name`'s bleed, seat 1 controlling a vampire
/// of `clan`, torpid or not (or none).
fn bled_by(name: &str, clan: Option<(&str, bool)>) -> i32 {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card(name), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    if let Some((clan, torpid)) = clan {
        e.add_minion(
            SeatId(1),
            textless(|c| c.clan == clan && c.capacity >= 4, 0),
            4,
            torpid,
        );
    }
    pick(&mut e, bleed(v));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    10 - e.state.seats[1].pool
}

/// Ranjan Rishi, Rebekka, Sigrid Bekker: "+1 bleed when bleeding a
/// Methuselah who controls a ready <clan>": 2 against a ready one, 1
/// against a torpid one or none, 1 against its antitribu.
#[test]
fn ranjan_rebekka_and_sigrid_bleed_harder_against_a_ready_clan() {
    for (name, clan, anti) in [
        (
            "Ranjan Rishi, Camarilla Scholar",
            "Brujah",
            "Brujah antitribu",
        ),
        (
            "Rebekka, Chantry Elder of Munich",
            "Malkavian",
            "Malkavian antitribu",
        ),
        ("Sigrid Bekker", "Nosferatu", "Nosferatu antitribu"),
    ] {
        assert_eq!(bled_by(name, Some((clan, false))), 2, "{name}");
        assert_eq!(bled_by(name, Some((clan, true))), 1, "{name} torpid");
        assert_eq!(bled_by(name, None), 1, "{name} none");
        assert_eq!(bled_by(name, Some((anti, false))), 1, "{name} {anti}");
    }
}

/// Rebekka: "+1 stealth on each of her actions": her hunt at 2 (the
/// rulebook's 1 and hers), her bleed at 1.
#[test]
fn rebekka_sneaks_on_each_action() {
    for (kind, stealth) in [(ActionKind::Hunt, 2), (ActionKind::Bleed, 1)] {
        let mut e = game(2);
        let r = e.add_minion(
            SeatId(0),
            find_card("Rebekka, Chantry Elder of Munich"),
            2,
            false,
        );
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        pick(&mut e, action(r, kind));
        request(&mut e);
        assert_eq!(
            e.action().expect("action").stealth(&e.state),
            stealth,
            "{kind:?}"
        );
    }
}

/// Reverend Blackwood: "Damaging strikes made by Reverend Blackwood that
/// require Blood Sorcery [tha] inflict +1 damage": Burning Touch [tha]'s 1
/// becomes 2; his hand strike stays 1.
#[test]
fn reverend_blackwood_adds_one_to_a_blood_sorcery_strike() {
    for touch in [true, false] {
        let mut e = game(2);
        let b = e.add_minion(SeatId(0), find_card("Reverend Blackwood"), 4, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let x = e.add_minion(SeatId(1), plain4(1), 4, false);
        e.add_minion(SeatId(1), plain4(2), 4, false);
        let flak = on(&mut e, "Flak Jacket", x);
        hand(&mut e, 0, &["Burning Touch"]);
        fight(&mut e, b, x);
        if touch {
            pick_when(&mut e, play(0, 0, b));
        } else {
            pick_when(&mut e, Choice::HandStrike);
        }
        let req = offering_where(&mut e, |o| uses_card(o, flak));
        let _ = req;
        assert_eq!(
            pending(&e, 1),
            (if touch { 2 } else { 1 }, 0),
            "touch: {touch}"
        );
    }
}

/// Pass (or hand strike) until a request offers an option `f` matches;
/// that request is returned, unanswered.
fn offering_where(e: &mut Engine, f: impl Fn(&Choice) -> bool) -> zvtes::decision::DecisionRequest {
    for _ in 0..200 {
        let req = request(e);
        if req.options.iter().any(&f) {
            return req;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    panic!("never offered");
}

/// Rosa Martínez: "During your unlock phase, if another ready vampire you
/// control is a Regent, Rosa gains 1 blood. +1 bleed": with Sha-Ennu
/// (regent) ready, +1, Rosa torpid too; Sha-Ennu torpid, nothing.
#[test]
fn rosa_martinez_feeds_while_a_ready_regent_serves_her() {
    for (rosa_torpid, regent_torpid, gain) in
        [(false, false, 1), (true, false, 1), (false, true, 0)]
    {
        let mut e = game(2);
        let r = e.add_minion(SeatId(0), find_card("Rosa Martínez"), 2, rosa_torpid);
        e.add_minion(SeatId(0), find_card("Sha-Ennu"), 4, regent_torpid);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        to_minion_phase(&mut e, SeatId(1));
        to_minion_phase(&mut e, SeatId(0));
        assert_eq!(
            blood(&e, r),
            2 + gain,
            "Rosa torpid {rosa_torpid}, regent torpid {regent_torpid}"
        );
    }
    assert_eq!(bled_by("Rosa Martínez", None), 2);
}

/// Sela (advanced): "Sabbat Archbishop of Rome: Sela gets +1 stealth during
/// actions directed at a Methuselah (or a card they control) controlling a
/// ready bishop": her bleed of seat 1 (a ready bishop) at 1, of seat 2 (an
/// archbishop only) at 0, an action at both at 1; her printed title.
#[test]
fn sela_sneaks_at_a_methuselah_with_a_ready_bishop() {
    let sela = CRYPT
        .iter()
        .position(|c| c.name == "Sela" && c.adv)
        .expect("Sela") as u16;
    assert_eq!(CRYPT[sela as usize].title, Title::Archbishop("Rome"));
    for (prey_title, stealth) in [(Title::Bishop, 1), (Title::Archbishop("Paris"), 0)] {
        let mut e = game(2);
        let s = e.add_minion(SeatId(0), sela, 4, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let t = e.add_minion(SeatId(1), sabbat4(1), 4, false);
        e.state.minions[t.index()].granted_title = Some(prey_title);
        pick(&mut e, bleed(s));
        request(&mut e);
        assert_eq!(
            e.action().expect("bleed").stealth(&e.state),
            stealth,
            "{prey_title:?}"
        );
    }
    // Several Methuselahs targeted: any of them (PIB 20150526).
    let mut e = game(3);
    let s = e.add_minion(SeatId(0), sela, 4, false);
    let t = e.add_minion(SeatId(1), sabbat4(1), 4, false);
    e.state.minions[t.index()].granted_title = Some(Title::Bishop);
    e.add_minion(SeatId(2), sabbat4(2), 4, false);
    let at = |s: u8| zvtes::state::PrintedCtx {
        kind: Some(ActionKind::Bleed),
        directed: Some(true),
        target: Some(SeatId(s)),
        ..Default::default()
    };
    let stealth = |ctx| e.state.printed_mod(s, zvtes::cards::Stat::Stealth, ctx);
    assert_eq!(stealth(at(2)), 0);
    let both = zvtes::state::PrintedCtx {
        targets: zvtes::ids::SeatSet::single(SeatId(2)).with(SeatId(1)),
        ..at(2)
    };
    assert_eq!(stealth(both), 1);
}

/// Thelonius and Tock ("Sabbat."), Wah Chun-Yuen ("Sabbat: Black Hand."):
/// no definition, traits generated, the Black Hand kept through a sect
/// change ([LSJ 20070322] [ANK 20180807]); Soldat Black Hand; Teresita
/// ("Black Hand Seraph:") Black Hand and Seraph (the generator's fix).
#[test]
fn the_round_34_vampires_soldat_to_wah_carry_their_printed_traits() {
    for (name, black_hand) in [
        ("Thelonius", false),
        ("Tock", false),
        ("Wah Chun-Yuen", true),
    ] {
        let card = find_card(name);
        assert!(zvtes::cards::vampire(card).is_none(), "{name}");
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), card, 2, false);
        assert_eq!(e.state.sect_of(m), Some(Sect::Sabbat), "{name}");
        assert_eq!(e.state.title_of(m), Title::None, "{name}");
        e.state.minions[m.index()].sect_override = Some(Sect::Independent);
        assert_eq!(e.state.is_black_hand(m), black_hand, "{name}");
    }
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Soldat"), 4, false);
    let t = e.add_minion(SeatId(0), find_card("Teresita, The Godmother"), 4, false);
    for m in [s, t] {
        e.state.minions[m.index()].sect_override = Some(Sect::Independent);
        assert!(e.state.is_black_hand(m));
    }
    assert!(e.state.is_seraph(t));
}

/// Soldat: "You may burn the Edge to give any Black Hand vampire an
/// optional manuever." [PIB 20130306] only while holding the Edge: Soldat
/// out of the combat, his controller's Black Hand Wah Chun-Yuen (made
/// Independent: [LSJ 20070322] [ANK 20180807] the trait kept) blocked; the
/// Edge burned, Wah maneuvers in the round's determine-range step; the
/// blocker is no target; without the Edge, not offered.
#[test]
fn soldat_burns_the_edge_for_a_black_hand_combatants_maneuver() {
    for edge in [true, false] {
        let mut e = game(2);
        let s = e.add_minion(SeatId(0), find_card("Soldat"), 4, false);
        let w = e.add_minion(SeatId(0), find_card("Wah Chun-Yuen"), 5, false);
        e.state.minions[w.index()].sect_override = Some(Sect::Independent);
        let x = e.add_minion(SeatId(1), plain4(1), 4, false);
        e.add_minion(SeatId(1), plain4(2), 4, false);
        e.state.seats[0].has_edge = edge;
        fight(&mut e, w, x);
        let give = |t| printed_use(s, 0, Some(s), Target::Minion(t));
        if !edge {
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
                uses_printed(o, s, 0)
            });
            continue;
        }
        let req = offering_where(&mut e, |o| uses_printed(o, s, 0));
        let mine = req
            .options
            .iter()
            .copied()
            .find(|o| uses_printed(o, s, 0))
            .expect("offered");
        let target_of = |o: Choice| match o {
            Choice::UseAbility { target, .. } => target,
            _ => Target::None,
        };
        assert_eq!(target_of(mine), Target::Minion(w), "{:?}", req.options);
        assert!(!req
            .options
            .iter()
            .any(|o| uses_printed(o, s, 0) && target_of(*o) == Target::Minion(x)));
        let _ = give;
        e.choose(index_of(&req, mine));
        let req = offering(&mut e, Choice::UseManeuver);
        assert_eq!(req.window, WindowLabel::CombatDetermineRange);
        assert!(!e.state.seats[0].has_edge, "the Edge burned");
    }
}

/// Spiridonas: "Once each turn, Spiridonas can burn X+2 blood to get +X
/// bleed for the current action." [SFC 19960819] after the block attempts
/// are concluded: X up to 3 with 5 blood, 3 taken: 5 blood burned, 4 bled.
/// [ANK 20180719-4] once in each player's turn: not again in that turn.
#[test]
fn spiridonas_burns_x_plus_two_for_x_bleed_after_blocks() {
    let mut e = game(2);
    let sp = e.add_minion(SeatId(0), find_card("Spiridonas"), 5, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    pick(&mut e, bleed(sp));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    e.choose(index_of(&req, Choice::Pass));
    use_printed(&mut e, sp, 0);
    let req = request(&mut e);
    assert_eq!(req.options, (0..=3).map(Choice::SetX).collect::<Vec<_>>());
    e.choose(index_of(&req, Choice::SetX(3)));
    let req = request(&mut e);
    assert!(
        !req.options.iter().any(|o| uses_printed(o, sp, 0)),
        "once a turn"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, sp), 0);
    assert_eq!(e.state.seats[1].pool, 10 - 4);
}

/// Szechenyi Jolán: "If Jolán performs a successful recruit or employ
/// action, she can unlock during the next discard phase": after recruiting
/// Carlton Van Wyk she unlocks in that discard phase; after a hunt, not
/// offered.
#[test]
fn szechenyi_jolan_unlocks_after_a_successful_recruit() {
    for recruit in [true, false] {
        let mut e = game(2);
        let j = e.add_minion(
            SeatId(0),
            find_card("Szechenyi Jolán, Mother of Horrors"),
            4,
            false,
        );
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        hand(&mut e, 0, &["Carlton Van Wyk"]);
        if recruit {
            pick(&mut e, play(0, 0, j));
        } else {
            pick(&mut e, action(j, ActionKind::Hunt));
        }
        let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
        assert!(e.state.minions[j.index()].locked);
        let offered = req.options.iter().any(|o| uses_printed(o, j, 0));
        assert_eq!(offered, recruit, "recruit: {recruit}");
        if recruit {
            use_printed(&mut e, j, 0);
            request(&mut e);
            assert!(!e.state.minions[j.index()].locked);
        }
    }
}

/// The advanced Sébastien Goulet with the base merged into it, or not.
fn sebastien(e: &mut Engine, merged: bool) -> MinionId {
    let adv = CRYPT
        .iter()
        .position(|c| c.name == "Sébastien Goulet" && c.adv)
        .expect("advanced") as u16;
    let m = e.add_minion(SeatId(0), adv, 4, false);
    if merged {
        let base = CRYPT
            .iter()
            .position(|c| c.name == "Sébastien Goulet" && !c.adv)
            .expect("base") as u16;
        e.state.minions[m.index()].merged = Some(base);
    }
    m
}

/// Sébastien Goulet (advanced): "+1 stealth on recruit and employ actions.
/// [MERGED] He can steal an ally or retainer as a +1 stealth Ⓓ action":
/// unmerged, his recruit at stealth 2 and no steal; merged, he takes seat
/// 1's Raven Spy (+1 stealth), the base's discount kept.
#[test]
fn sebastien_goulet_adv_hires_stealthily_and_steals_merged() {
    let mut e = game(2);
    let s = sebastien(&mut e, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    let spy = on(&mut e, "Raven Spy", x);
    hand(&mut e, 0, &["Carlton Van Wyk"]);
    let req = request(&mut e);
    assert!(!req
        .options
        .iter()
        .any(|o| uses_printed(o, s, 1) || uses_printed(o, s, 2)));
    e.choose(index_of(&req, play(0, 0, s)));
    request(&mut e);
    assert_eq!(
        e.action().expect("recruit").stealth(&e.state),
        2,
        "the recruit's 1 and his"
    );
    let mut e = game(2);
    let s = sebastien(&mut e, true);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    let spy2 = on(&mut e, "Raven Spy", x);
    let _ = spy;
    let steal = printed_use(s, 2, Some(s), Target::Card(spy2));
    pick(&mut e, steal);
    request(&mut e);
    assert_eq!(e.action().expect("steal").stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = &e.state.cards[spy2.index()];
    assert_eq!(
        (c.host, e.state.card_controller(spy2)),
        (Some(s), SeatId(0))
    );
    assert_eq!(
        e.state
            .printed_mod(s, zvtes::cards::Stat::CostOff, Default::default()),
        0,
        "the discount reads the card played"
    );
}

/// Teresita: "+1 intercept against Camarilla vampires." [ANK 20190416]
/// "She gains or looses intercept if the acting vampire changes sect
/// during the action (eg. with The Red Question or Mask of a Thousand
/// Faces)": a Camarilla vampire's 1-stealth bleed blocked; made Anarch by
/// The Red Question after her attempt, it is not.
#[test]
fn teresita_intercepts_camarilla_live() {
    for question in [false, true] {
        let mut e = game(2);
        let v = e.add_minion(
            SeatId(0),
            textless(|c| c.sect == Some(Sect::Camarilla) && c.capacity >= 4, 0),
            4,
            false,
        );
        e.state.minions[v.index()].timed.push((
            zvtes::cards::Lasting::Stealth(1),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(0),
        ));
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let t = e.add_minion(SeatId(1), find_card("Teresita, The Godmother"), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        hand(&mut e, 0, &["Red Question, The"]);
        pick(&mut e, bleed(v));
        pick_when(&mut e, block(t));
        if question {
            pick_when(&mut e, play(0, 0, v));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let blocked = e.last_outcome == Some(zvtes::actions::Outcome::Blocked);
        assert_eq!(blocked, !question, "The Red Question: {question}");
    }
}

/// Terrell Harding: "Terrell can steal 1 blood from a younger vampire as a
/// +1 stealth Ⓓ action": a younger vampire of his prey gives 1 (stealth
/// 1); an older one is no target.
#[test]
fn terrell_harding_steals_a_blood_from_a_younger_vampire() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Terrell Harding"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity <= 5, 0), 3, false);
    let old = e.add_minion(SeatId(1), textless(|c| c.capacity >= 8, 0), 4, false);
    let req = request(&mut e);
    let at = |m| printed_use(t, 0, Some(t), Target::Minion(m));
    assert!(req.options.contains(&at(young)), "{:?}", req.options);
    assert!(!req.options.contains(&at(old)));
    e.choose(index_of(&req, at(young)));
    request(&mut e);
    assert_eq!(e.action().expect("steal").stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, t), blood(&e, young)), (5, 2));
}

/// Vasilis: "If Vasilis is ready, all Giovanni get +1 bleed when bleeding a
/// Methuselah who controls a ready Brujah": a Giovanni bleeds seat 1 with
/// a ready Brujah for 2; Vasilis torpid, 1; seat 1's Brujah antitribu
/// only, 1.
#[test]
fn vasilis_gives_giovanni_bleed_against_a_brujah_keeper() {
    for (torpid, clan, want) in [
        (false, "Brujah", 2),
        (true, "Brujah", 1),
        (false, "Brujah antitribu", 1),
    ] {
        let mut e = game(2);
        e.add_minion(
            SeatId(0),
            find_card("Vasilis, The Traitor of Don Cruez"),
            2,
            torpid,
        );
        let g = e.add_minion(
            SeatId(0),
            textless(|c| c.clan == "Giovanni" && c.capacity >= 4, 0),
            4,
            false,
        );
        e.add_minion(
            SeatId(1),
            textless(|c| c.clan == clan && c.capacity >= 4, 0),
            4,
            false,
        );
        pick(&mut e, bleed(g));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(10 - e.state.seats[1].pool, want, "{torpid} {clan}");
    }
}

/// Victoria: "During your master phase, you may burn the Edge to gain 2
/// pool if Victoria is ready": the Edge burned, 2 pool, the master phase
/// action kept; not with Victoria in torpor.
#[test]
fn victoria_burns_the_edge_for_two_pool_in_the_master_phase() {
    for torpid in [false, true] {
        let mut e = game(2);
        let v = e.add_minion(SeatId(0), find_card("Victoria"), 4, torpid);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        e.state.seats[0].has_edge = true;
        e.at_master_phase();
        let req = request(&mut e);
        let offered = req.options.iter().any(|o| uses_printed(o, v, 0));
        assert_eq!(offered, !torpid, "torpid: {torpid}");
        if torpid {
            continue;
        }
        let mpa = e.state.mpa_left(SeatId(0));
        use_printed(&mut e, v, 0);
        request(&mut e);
        assert_eq!(e.state.seats[0].pool, 12);
        assert!(!e.state.seats[0].has_edge);
        assert_eq!(e.state.mpa_left(SeatId(0)), mpa);
    }
}

/// Virgil: "Virgil can steal a retainer as a Ⓓ action. He can steal all of
/// the life from a retainer attached to him as a +1 stealth action." [LSJ
/// 20100909] "The two actions provided are not cross-restrictive. Each can
/// be performed once during the same turn by the same minion": he takes
/// seat 1's Raven Spy, then (unlocked) its life: 1 blood, the Spy burned.
#[test]
fn virgil_steals_a_retainer_and_drains_his_own() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Virgil"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    let spy = on(&mut e, "Raven Spy", x);
    e.state.cards[spy.index()].counters = 1;
    pick(&mut e, printed_use(v, 0, Some(v), Target::Card(spy)));
    request(&mut e);
    // Unlocked before the steal resolves: offered his second action.
    e.state.minions[v.index()].locked = false;
    pick_when(&mut e, printed_use(v, 1, Some(v), Target::Card(spy)));
    assert_eq!(e.state.cards[spy.index()].host, Some(v));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, v), 5);
    assert!(!e.state.cards[spy.index()].in_play);
    assert!(ash_has(&e, 1, "Raven Spy"));
}

/// Virgil and an Ilomba on him (rules-qa 126.197: Ilomba's replacement
/// applies, "net 0"): at capacity, the stolen life's burn of his blood comes
/// first, then the life becomes his blood — back at capacity; Ilomba stays.
#[test]
fn virgil_at_capacity_nets_zero_through_ilomba() {
    let mut e = game(2);
    let card = find_card("Virgil");
    let cap = CRYPT[card as usize].capacity;
    let v = e.add_minion(SeatId(0), card, cap, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let il = in_play(&mut e, 0, "Ilomba", Some(v));
    e.state.cards[il.index()].counters = 1;
    pick(&mut e, printed_use(v, 1, Some(v), Target::Card(il)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, v), cap, "burned 1, then gained 1");
    assert!(e.state.cards[il.index()].in_play, "Ilomba stays");
}

/// Volker: "Volker cannot attempt to block primogen": a primogen's bleed
/// offers him no block; an untitled vampire's does.
#[test]
fn volker_cannot_block_a_primogen() {
    for primogen in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(
            SeatId(0),
            textless(|c| c.sect == Some(Sect::Camarilla) && c.capacity >= 4, 0),
            4,
            false,
        );
        if primogen {
            e.state.minions[a.index()].granted_title = Some(Title::Primogen);
        }
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let v = e.add_minion(SeatId(1), find_card("Volker, The Puppet Prince"), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        pick(&mut e, bleed(a));
        let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
        assert_eq!(req.options.contains(&block(v)), !primogen, "{primogen}");
    }
}

/// Watenda's cancel (ability 0), offered as the opposing minion plays.
fn watenda_cancel(w: MinionId) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(w),
        ability: 0,
        actor: None,
        target: Target::None,
    }
}

/// Watenda: "Once each combat, Watenda can cancel the effect of a combat
/// card the opposing minion plays by burning an amount of blood equal to
/// the blood or pool cost of the card. If the card is a strike card, the
/// opposing minion chooses a new strike": A's Song in the Dark (2 blood)
/// canceled for 2 of Watenda's, A's 2 still paid, a strike chosen again;
/// once a combat.
#[test]
fn watenda_cancels_a_combat_card_for_its_cost_in_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain4(0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["ani"];
    e.add_minion(SeatId(0), plain4(1), 4, false);
    let w = e.add_minion(SeatId(1), find_card("Watenda"), 3, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    hand(&mut e, 0, &["Song in the Dark", "Song in the Dark"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(w));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, watenda_cancel(w));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(
        req.options.contains(&play(0, 0, a)),
        "another strike chosen"
    );
    assert_eq!(blood(&e, w), 3 - 2);
    assert_eq!(blood(&e, a), 6 - 2, "its cost paid");
    assert!(ash_has(&e, 0, "Song in the Dark"));
    pick(&mut e, play(0, 0, a));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        watenda_cancel(w),
    );
}

/// Watenda: [LSJ 20091124] "The cost he pays takes into account any cost
/// modification that applies for the opposing minion": Miller Delmardigan's
/// Blur ([cel], 1 blood) costs him 0, canceled for 0. [LSJ 20071015-2] "he
/// pays any pool cost in blood": Caseless Rounds (1 pool) for 1 blood, its
/// pool still paid.
#[test]
fn watenda_pays_a_modified_cost_and_pool_in_blood() {
    let mut e = game(2);
    let m = e.add_minion(
        SeatId(0),
        find_card("Miller Delmardigan, Teacher of Bahari"),
        4,
        false,
    );
    e.state.minions[m.index()].gained_levels = vec!["cel"];
    e.add_minion(SeatId(0), plain4(1), 4, false);
    let w = e.add_minion(SeatId(1), find_card("Watenda"), 3, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    hand(&mut e, 0, &["Blur"]);
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(w));
    pick_when(&mut e, play(0, 0, m));
    let req = offering(&mut e, watenda_cancel(w));
    let before = blood(&e, w);
    e.choose(index_of(&req, watenda_cancel(w)));
    request(&mut e);
    assert_eq!(blood(&e, w), before, "Blur costs Miller nothing");

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(0), plain4(1), 4, false);
    let gun = on(&mut e, ".44 Magnum", a);
    let w = e.add_minion(SeatId(1), find_card("Watenda"), 3, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    hand(&mut e, 0, &["Caseless Rounds"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(w));
    pick_when(&mut e, weapon(gun, 0));
    pick_when(&mut e, play(0, 0, a));
    pick(&mut e, watenda_cancel(w));
    assert_eq!(blood(&e, w), 3 - 1, "the pool cost in blood");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10 - 1, "its pool still paid");
}

/// Watenda: [ANK 20221021] "Can only use his ability when he is in combat
/// himself": another of his Methuselah's vampires blocks; the combat card
/// played against it offers him nothing.
#[test]
fn watenda_cannot_cancel_outside_his_own_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain4(0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["ani"];
    e.add_minion(SeatId(0), plain4(1), 4, false);
    let w = e.add_minion(SeatId(1), find_card("Watenda"), 3, false);
    let x = e.add_minion(SeatId(1), plain4(2), 4, false);
    hand(&mut e, 0, &["Song in the Dark"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        watenda_cancel(w),
    );
}

/// Watenda: [LSJ 20021007] "Can use his ability even if he's going to
/// torpor": empty, wounded by A's hand strike, he cancels A's Amaranth (no
/// cost): in torpor, not diablerized.
#[test]
fn watenda_cancels_amaranth_on_his_way_to_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 8, 0), 4, false);
    e.add_minion(SeatId(0), plain4(1), 4, false);
    let w = e.add_minion(SeatId(1), find_card("Watenda"), 1, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    hand(&mut e, 0, &["Amaranth"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(w));
    e.state.minions[w.index()].blood = 0;
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, watenda_cancel(w));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[w.index()];
    assert!(v.in_play && v.in_torpor, "in torpor");
    assert!(ash_has(&e, 0, "Amaranth"));
}

/// Wilhelm Waldburg: "If Wilhelm's title is contested, the cost of
/// contesting is increased by 1 blood for the other vampire(s)": seat 1's
/// rival Prince of Berlin with 1 blood may only yield, with 2 pays 2;
/// Wilhelm pays 1.
#[test]
fn wilhelm_waldburg_makes_his_rivals_pay_two_to_contest() {
    use zvtes::game::Contest;
    for rival_blood in [1, 2] {
        let mut e = game(2);
        let wh = e.add_minion(SeatId(0), find_card("Wilhelm Waldburg"), 4, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let r = e.add_minion(SeatId(1), plain4(1), rival_blood, false);
        e.state.minions[r.index()].granted_title = Some(Title::Prince("Berlin"));
        e.state.minions[wh.index()].title_contested = true;
        e.state.minions[r.index()].title_contested = true;
        assert_eq!(e.state.title_contest_cost(r), 2);
        assert_eq!(e.state.title_contest_cost(wh), 1);
        let pay = Choice::PayContest(Contest::Title(r));
        let offered = offered_before(&mut e, pay, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(offered, rival_blood == 2, "{rival_blood}");
        if offered {
            request(&mut e);
            assert_eq!(blood(&e, r), 0);
        }
    }
}

/// Yasmin the Black ("Sabbat. Black Hand."): no definition, the trait
/// generated; Yazid Tamari (base): "Sabbat: Black Hand. Seraph. +1 bleed."
/// ([LSJ 20070322] [ANK 20180807] Black Hand kept through a sect change.)
#[test]
fn yazid_tamari_bleeds_for_two() {
    assert!(zvtes::cards::vampire(find_card("Yasmin the Black")).is_none());
    let base = CRYPT
        .iter()
        .position(|c| c.name == "Yazid Tamari" && !c.adv)
        .expect("base") as u16;
    let mut e = game(2);
    let y = e.add_minion(SeatId(0), base, 4, false);
    let ya = e.add_minion(SeatId(0), find_card("Yasmin the Black"), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    for m in [y, ya] {
        e.state.minions[m.index()].sect_override = Some(Sect::Independent);
        assert!(e.state.is_black_hand(m));
    }
    assert!(e.state.is_seraph(y));
    e.state.minions[y.index()].sect_override = None;
    pick(&mut e, bleed(y));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

/// Yazid Tamari merged: [ANK 20180524] [LSJ 20031121-3] "[MERGED] Is
/// Anarch Black Hand Seraph": the advanced card on the base, Anarch, Black
/// Hand, Seraph (the base's), +1 bleed and +1 stealth on a directed
/// action.
#[test]
fn yazid_tamari_merged_is_an_anarch_black_hand_seraph() {
    let at = |adv: bool| {
        CRYPT
            .iter()
            .position(|c| c.name == "Yazid Tamari" && c.adv == adv)
            .expect("Yazid") as u16
    };
    let mut e = game(2);
    let y = e.add_minion(SeatId(0), at(true), 4, false);
    assert!(!e.state.is_seraph(y), "the advanced card alone");
    e.state.minions[y.index()].merged = Some(at(false));
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    assert_eq!(e.state.sect_of(y), Some(Sect::Anarch));
    assert!(e.state.is_black_hand(y) && e.state.is_seraph(y));
    pick(&mut e, bleed(y));
    request(&mut e);
    assert_eq!(e.action().expect("bleed").stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

/// Yitzak: "Yitzak may lock a younger vampire as a Ⓓ action. If successful,
/// that vampire does not unlock as normal during his or her next unlock
/// phase": seat 1's younger vampire locked, still locked past its next
/// unlock phase, unlocked the one after; an older one is no target.
#[test]
fn yitzak_locks_a_younger_vampire_that_then_skips_its_unlock() {
    let mut e = game(2);
    let y = e.add_minion(SeatId(0), find_card("Yitzak"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity <= 5, 0), 4, false);
    let old = e.add_minion(SeatId(1), textless(|c| c.capacity >= 8, 0), 4, false);
    let req = request(&mut e);
    let at = |m| printed_use(y, 0, Some(y), Target::Minion(m));
    assert!(req.options.contains(&at(young)), "{:?}", req.options);
    assert!(!req.options.contains(&at(old)));
    e.choose(index_of(&req, at(young)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[young.index()].locked, "skipped its unlock");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[young.index()].locked);
}

/// Yong-Sun, Harmonist (advanced): "+X bleed, where X is the number of
/// vampires in the target Methuselah's ash heap": two vampires and an
/// imbued there, a bleed of 3; merged, the base's crypt burn offered too.
#[test]
fn yong_sun_advanced_bleeds_for_each_vampire_in_the_targets_ash_heap() {
    let at = |adv: bool| {
        CRYPT
            .iter()
            .position(|c| c.name == "Yong-Sun, Harmonist" && c.adv == adv)
            .expect("Yong-Sun") as u16
    };
    let imbued = CRYPT.iter().position(|c| c.imbued).expect("an imbued") as u16;
    for merged in [false, true] {
        let mut e = game(2);
        let y = e.add_minion(SeatId(0), at(true), 4, false);
        if merged {
            e.state.minions[y.index()].merged = Some(at(false));
        }
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        for c in [plain4(2), plain4(3), imbued] {
            e.state.seats[1].ash_crypt.push(c);
        }
        let req = request(&mut e);
        let burn = req.options.iter().any(|o| uses_printed(o, y, 0));
        assert_eq!(burn, merged, "merged: {merged}");
        pick(&mut e, bleed(y));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10 - 3, "merged: {merged}");
    }
}

/// Ingram Frizer ("Sabbat. Black Hand."), Itzahk Levine ("Camarilla."),
/// Josef and Kurt Strauss ("Sabbat."), Judah ("Camarilla primogen."), Kij
/// Dansky ("Sabbat priscus."): no definition, traits generated; Ingram,
/// Jalan-Aajav ("Black Hand Seraph:", the generator's fix) and Kazimir
/// Savostin Black Hand, kept through a sect change ([LSJ 20070322] [ANK
/// 20180807]); Jalan and Kazimir Seraph.
#[test]
fn the_round_34_vampires_ingram_to_kurt_carry_their_printed_traits() {
    for (name, sect, title, black_hand) in [
        ("Ingram Frizer", Sect::Sabbat, Title::None, true),
        ("Itzahk Levine", Sect::Camarilla, Title::None, false),
        ("Josef", Sect::Sabbat, Title::None, false),
        ("Kurt Strauss", Sect::Sabbat, Title::None, false),
        ("Judah", Sect::Camarilla, Title::Primogen, false),
        ("Kij Dansky", Sect::Sabbat, Title::Priscus, false),
    ] {
        let card = find_card(name);
        assert!(zvtes::cards::vampire(card).is_none(), "{name}");
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), card, 2, false);
        assert_eq!(e.state.sect_of(m), Some(sect), "{name}");
        assert_eq!(e.state.title_of(m), title, "{name}");
        e.state.minions[m.index()].sect_override = Some(Sect::Independent);
        assert_eq!(e.state.is_black_hand(m), black_hand, "{name}");
    }
    let mut e = game(2);
    for name in ["Jalan-Aajav", "Kazimir Savostin"] {
        let m = e.add_minion(SeatId(0), find_card(name), 2, false);
        e.state.minions[m.index()].sect_override = Some(Sect::Independent);
        assert!(e.state.is_black_hand(m) && e.state.is_seraph(m), "{name}");
    }
}

/// Imogen: "Imogen can exchange one random card from the hand of your prey
/// or predator for one library card from that Methuselah's ash heap as a +1
/// stealth Ⓓ action": at her prey, naming its Form of Mist: its one hand
/// card (Dodge) to the ash heap, Form of Mist to its hand; her predator a
/// target too, her own Methuselah not; a heap of burned vampires only is no
/// target (LSJ 19990525).
#[test]
fn imogen_exchanges_a_random_card_of_her_prey_for_a_named_ash_card() {
    let mut e = game(3);
    let im = e.add_minion(SeatId(0), find_card("Imogen"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(2), plain4(2), 4, false);
    hand(&mut e, 1, &["Dodge"]);
    for c in ["Form of Mist", "Skin of Rock"] {
        e.state.seats[1].ash.push(find_library(c));
    }
    e.state.seats[2].ash.push(find_library("Dodge"));
    e.state.seats[0].ash.push(find_library("Dodge"));
    e.state.seats[0].ash_crypt.push(plain4(3));
    let at = |s: u8| printed_use(im, 0, Some(im), Target::Seat(SeatId(s)));
    let req = request(&mut e);
    assert!(req.options.contains(&at(1)) && req.options.contains(&at(2)));
    assert!(!req.options.contains(&at(0)));
    e.choose(index_of(&req, at(1)));
    pick(&mut e, Choice::FindCard(find_library("Form of Mist")));
    request(&mut e);
    assert_eq!(e.action().expect("exchange").stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].hand, vec![find_library("Form of Mist")]);
    assert!(ash_has(&e, 1, "Dodge") && ash_has(&e, 1, "Skin of Rock"));
    assert!(!ash_has(&e, 1, "Form of Mist"));

    let mut e = game(2);
    let im = e.add_minion(SeatId(0), find_card("Imogen"), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    e.state.seats[1].ash_crypt.push(plain4(3));
    let req = request(&mut e);
    assert!(
        !req.options.iter().any(|o| uses_printed(o, im, 0)),
        "{:?}",
        req.options
    );
}

/// Jalan-Aajav: "Once each round, Jalan may burn 1 blood to treat
/// aggravated damage as normal. He gets 1 optional press each combat": X's
/// aggravated hand strike taken as normal for a blood (in torpor without);
/// his press.
#[test]
fn jalan_aajav_takes_aggravated_damage_as_normal_for_a_blood() {
    for used in [true, false] {
        let mut e = game(2);
        let j = e.add_minion(SeatId(0), find_card("Jalan-Aajav"), 3, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let x = e.add_minion(SeatId(1), plain4(1), 4, false);
        e.add_minion(SeatId(1), plain4(2), 4, false);
        e.state.minions[x.index()].timed.push((
            zvtes::cards::Lasting::AggravatedHands,
            zvtes::state::Expiry::Combat,
            SeatId(1),
        ));
        assert_eq!(
            e.state
                .printed_mod(j, zvtes::cards::Stat::CombatPress, Default::default()),
            1
        );
        fight(&mut e, j, x);
        if used {
            use_printed(&mut e, j, 0);
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let v = &e.state.minions[j.index()];
        assert_eq!(v.in_torpor, !used, "used: {used}");
        if used {
            assert_eq!(blood(&e, j), 3 - 1 - 1);
        }
    }
}

/// Jalan-Aajav outside combat (Konrad Fleischer's template, owner, rules-qa
/// 122.96: no "in combat" printed): Sunrise Service's aggravated point,
/// his player asked as it would be dealt; treated as normal for 1 blood
/// (4 - 1 - 1, no torpor), else torpor with his 4.
#[test]
fn jalan_aajav_treats_sunrise_service_damage_as_normal() {
    for treat in [true, false] {
        let mut e = game(2);
        let j = e.add_minion(SeatId(0), find_card("Jalan-Aajav"), 4, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        hand(&mut e, 0, &["Sunrise Service"]);
        pick(&mut e, play(0, 0, j));
        let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        let as_normal = Choice::ResolvePrinted { minion: j };
        assert_eq!(req.options, vec![Choice::Pass, as_normal]);
        e.choose(index_of(&req, if treat { as_normal } else { Choice::Pass }));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let v = &e.state.minions[j.index()];
        assert_eq!(v.in_torpor, !treat, "treated as normal: {treat}");
        assert_eq!(v.blood, if treat { 4 - 1 - 1 } else { 4 });
    }
}

/// Jan Pieterzoon: "While Jan is ready, every Methuselah's hand size is
/// reduced by 1. +1 bleed": 6 for every seat, 7 with Jan in torpor.
#[test]
fn jan_pieterzoon_shrinks_every_hand_while_ready() {
    let base = CRYPT
        .iter()
        .position(|c| c.name == "Jan Pieterzoon" && !c.adv)
        .expect("base") as u16;
    let mut e = game(3);
    let j = e.add_minion(SeatId(1), base, 4, false);
    for s in 0..3 {
        assert_eq!(e.state.hand_size(SeatId(s)), 6);
    }
    e.state.minions[j.index()].in_torpor = true;
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
    assert_eq!(
        e.state
            .printed_mod(j, zvtes::cards::Stat::Bleed, Default::default()),
        1
    );
}

/// Jan Pieterzoon merged: [ANK 20180524] "[MERGED] Is Independent. If the
/// base version was the one in play, it changes sect as it is merged":
/// Independent, every hand 6 (the base's text), +3 votes with a Gehenna
/// card in play.
#[test]
fn jan_pieterzoon_merged_is_independent_with_three_votes_under_gehenna() {
    let at = |adv: bool| {
        CRYPT
            .iter()
            .position(|c| c.name == "Jan Pieterzoon" && c.adv == adv)
            .expect("Jan") as u16
    };
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), at(true), 4, false);
    e.state.minions[j.index()].merged = Some(at(false));
    assert_eq!(e.state.sect_of(j), Some(Sect::Independent));
    assert_eq!(e.state.hand_size(SeatId(1)), 6);
    let votes = |e: &Engine| {
        e.state
            .printed_mod(j, zvtes::cards::Stat::Votes, Default::default())
    };
    assert_eq!(votes(&e), 0);
    in_play(&mut e, 1, "Thirst", None);
    assert_eq!(votes(&e), 3);
}

/// Javier Montoya (base): "Javier burns 1 less blood when rescuing a
/// vampire in torpor. +1 bleed." [PIB 20110918] [LSJ 20020304-2] "The
/// discount applies to the action, regardless of who pays": a rescue of a
/// torpid vampire with 2 blood costs 1, paid by it. Merged: Anarch Baron
/// of Barcelona with the base's discount.
#[test]
fn javier_montoya_rescues_for_one_blood_whoever_pays() {
    let at = |adv: bool| {
        CRYPT
            .iter()
            .position(|c| c.name == "Javier Montoya" && c.adv == adv)
            .expect("Javier") as u16
    };
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), at(false), 1, false);
    let t = e.add_minion(SeatId(0), textless(|c| c.capacity > 6, 0), 2, true);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: j,
            action: ActionKind::Rescue,
            target: Some(t),
        },
    );
    let req = until(&mut e, SeatId(0), WindowLabel::RescueCost);
    assert!(!req.options.contains(&Choice::PayRescue { from_acting: 2 }));
    pick(&mut e, Choice::PayRescue { from_acting: 0 });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_torpor);
    assert_eq!((blood(&e, j), blood(&e, t)), (1, 1));

    let mut e = game(2);
    let m = e.add_minion(SeatId(0), at(true), 4, false);
    e.state.minions[m.index()].merged = Some(at(false));
    assert_eq!(e.state.sect_of(m), Some(Sect::Anarch));
    assert_eq!(e.state.title_of(m), Title::Baron("Barcelona"));
    assert_eq!(
        e.state
            .printed_mod(m, zvtes::cards::Stat::RescueDiscount, Default::default()),
        1
    );
}

/// Kazimir Savostin: "If Kazimir blocks, he gets 1 optional press during
/// the resulting combat": blocking, a press offered once; blocked as he
/// acts, none.
#[test]
fn kazimir_savostin_presses_only_in_the_combat_his_block_began() {
    for blocks in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(0), plain4(1), 4, false);
        let k = if blocks {
            let k = e.add_minion(SeatId(1), find_card("Kazimir Savostin"), 4, false);
            e.add_minion(SeatId(1), plain4(2), 4, false);
            fight(&mut e, a, k);
            k
        } else {
            let k = e.add_minion(SeatId(0), find_card("Kazimir Savostin"), 4, false);
            let x = e.add_minion(SeatId(1), plain4(2), 4, false);
            fight(&mut e, k, x);
            k
        };
        let offered = offered_before(
            &mut e,
            printed_use(k, 0, None, Target::None),
            SeatId(0),
            WindowLabel::MinionPhase,
        );
        assert_eq!(offered, blocks, "blocks: {blocks}");
    }
}

/// Kendrick: "If Kendrick is ready during your master phase, you may look
/// at a card at random from your prey's hand. If it is a political action
/// card, your prey discards it": Kine Resources Contested discarded and
/// replaced; a Dodge stays; once a master phase.
#[test]
fn kendrick_discards_a_political_action_card_he_sees() {
    for (card, discarded) in [("Kine Resources Contested", true), ("Dodge", false)] {
        let mut e = game(2);
        let k = e.add_minion(SeatId(0), find_card("Kendrick"), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        hand(&mut e, 1, &[card]);
        e.state.seats[1].library = vec![find_library("Form of Mist")].into();
        e.at_master_phase();
        use_printed(&mut e, k, 0);
        let req = request(&mut e);
        assert!(!req.options.iter().any(|o| uses_printed(o, k, 0)), "once");
        assert_eq!(ash_has(&e, 1, card), discarded, "{card}");
        let held = e.state.seats[1].hand.contains(&find_library(card));
        assert_eq!(held, !discarded, "{card}");
        if discarded {
            assert_eq!(e.state.seats[1].hand, vec![find_library("Form of Mist")]);
        }
    }
}

/// Klaus van der Veken: "Klaus may cancel the effects of one of your prey's
/// hunting grounds" (rules-qa 126.108: each use, spent): his prey's
/// Academic Hunting Ground used, canceled: no blood, not offered again that
/// phase, offered and canceled again the next turn; let resolve: +1; from
/// torpor too (LSJ 20030819).
#[test]
fn klaus_van_der_veken_cancels_his_preys_hunting_ground_use() {
    for (cancel, torpid) in [(true, false), (false, false), (true, true)] {
        let mut e = game(2);
        e.add_minion(SeatId(0), find_card("Klaus van der Veken"), 4, torpid);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let v = e.add_minion(SeatId(1), plain4(1), 2, false);
        let hg = in_play(&mut e, 1, "Academic Hunting Ground", None);
        let turns = if cancel && !torpid { 2 } else { 1 };
        for turn in 0..turns {
            let before = blood(&e, v);
            let req = offering_where(&mut e, |o| uses_card(o, hg));
            let usage = req
                .options
                .iter()
                .copied()
                .find(|o| {
                    uses_card(o, hg)
                        && matches!(o, Choice::UseAbility { target: Target::Minion(m), .. } if *m == v)
                })
                .expect("the ground's use");
            e.choose(index_of(&req, usage));
            let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
            let answer = if cancel {
                Choice::ResolveRider { card: hg.0 }
            } else {
                Choice::Pass
            };
            e.choose(index_of(&req, answer));
            let req = request(&mut e);
            assert!(!req.options.iter().any(|o| uses_card(o, hg)), "spent");
            let gain = u8::from(!cancel);
            assert_eq!(blood(&e, v), before + gain, "cancel: {cancel}, turn {turn}");
        }
    }
}
