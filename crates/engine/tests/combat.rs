//! Slice 3 of the effects framework: combat (rulebook "Combat"; spec §4).
//! The card-less machine first — a block leads to one round of hand
//! strikes at close range — then the combat cards, step by step. Windows
//! with a single option are elided, so the scripts only stop where a real
//! decision exists.

use zvtes::actions::Outcome;
use zvtes::cards::Target;
use zvtes::crypt::CRYPT;
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library};

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
        .unwrap_or_else(|| panic!("option {want:?} not in {:?}", req.options));
    e.choose(idx);
}

fn bleed(minion: MinionId) -> Choice {
    Choice::TakeAction {
        minion,
        action: ActionKind::Bleed,
        target: None,
    }
}

fn block(minion: MinionId) -> Choice {
    Choice::DeclareBlock { minion }
}

#[allow(dead_code)]
fn play(card: u8, mode: u8, actor: MinionId) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target: Target::None,
    }
}

#[allow(dead_code)]
fn plays(req: &DecisionRequest) -> Vec<Choice> {
    req.options
        .iter()
        .copied()
        .filter(|o| matches!(o, Choice::Play { .. }))
        .collect()
}

/// The `nth` V5 vampire with superior `disc`.
fn vampire_with(disc: &str, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(_, c)| c.disciplines.contains(&disc))
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no vampire with {disc}"))
}

fn blood(e: &Engine, m: MinionId) -> u8 {
    e.state.minions[m.index()].blood
}

fn hand(e: &mut Engine, seat: u8, cards: &[&str]) {
    for c in cards {
        e.state.seats[seat as usize].hand.push(find_library(c));
    }
}

fn set_x(n: u8) -> Choice {
    Choice::SetX(n)
}

/// 2 seats at seat 0's minion phase, turn 4. Seat 0: A (POT), B (CEL);
/// seat 1: X (FOR), Y (PRO). 4 blood each.
struct Table {
    e: Engine,
    a: MinionId,
    b: MinionId,
    x: MinionId,
    y: MinionId,
}

fn table() -> Table {
    let mut e = Engine::bare(2);
    let a = e.add_minion(SeatId(0), vampire_with("POT", 0), 4, false);
    let b = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
    let x = e.add_minion(SeatId(1), vampire_with("FOR", 0), 4, false);
    let y = e.add_minion(SeatId(1), vampire_with("PRO", 0), 4, false);
    e.state.turn = 4;
    Table { e, a, b, x, y }
}

/// Bleed with `acting`, blocked by `blocker`; every window up to the
/// block's combat elides with no cards in hand.
fn blocked_bleed(e: &mut Engine, acting: MinionId, blocker: MinionId) {
    pick(e, bleed(acting));
    pick(e, block(blocker));
}

// Rulebook "Combat": a block leads to one round at close range; with no
// cards, both minions hand strike for 1 (default strength), the blocker
// stays locked, and the action ended blocked.
#[test]
fn card_less_combat_is_one_round_of_hand_strikes() {
    let mut t = table();
    let e = &mut t.e;
    blocked_bleed(e, t.a, t.x);
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(e, t.a), blood(e, t.x)), (3, 3));
    assert!(e.state.minions[t.x.index()].locked);
    assert!(e.state.minions[t.a.index()].locked);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
    assert_eq!(e.state.seats[1].pool, 30);
}

// Rulebook "Damage Resolution": damage that cannot be mended wounds; the
// vampire goes to torpor and the combat ends.
#[test]
fn unmended_damage_sends_the_blocker_to_torpor() {
    let mut t = table();
    let e = &mut t.e;
    e.state.minions[t.x.index()].blood = 0;
    blocked_bleed(e, t.a, t.x);
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.x.index()].in_torpor);
    assert_eq!(blood(e, t.a), 3);
    let _ = (t.b, t.y);
}

// rules-qa 1.2 — impulse-reset inside a combat step: the acting side has
// the first opportunity; after the opponent plays, the opportunity returns
// to the acting side at the same step, and the step closes on consecutive
// declines. Torn Signpost [POT] sets a strength of 3 (spec §4.4.1: replaces
// the base).
#[test]
fn combat_step_impulse_reset() {
    let mut t = table();
    let e = &mut t.e;
    let z = e.add_minion(SeatId(1), vampire_with("POT", 1), 4, false);
    hand(e, 0, &["Torn Signpost"]);
    hand(e, 1, &["Torn Signpost"]);
    blocked_bleed(e, t.a, z);
    // Before range: seat 0 first, declines; seat 1 plays; back to seat 0.
    let req = expect(e, SeatId(0), WindowLabel::CombatBeforeRange);
    assert_eq!(plays(&req), vec![play(0, 0, t.a), play(0, 1, t.a)]);
    pick(e, Choice::Pass);
    expect(e, SeatId(1), WindowLabel::CombatBeforeRange);
    pick(e, play(0, 1, z));
    let req = expect(e, SeatId(0), WindowLabel::CombatBeforeRange);
    assert_eq!(plays(&req).len(), 2);
    pick(e, play(0, 1, t.a));
    // Both at strength 3, hand strikes: 3 damage each way.
    pick(e, Choice::Pass);
    pick(e, Choice::Pass);
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(e, t.a), blood(e, z)), (1, 1));
}

// Rulebook "Determine Range": maneuvers alternate, a minion cannot
// maneuver twice in a row, the last uncanceled one sets the range. Hand
// strikes are ineffective at long range.
#[test]
fn maneuvers_alternate_and_long_range_voids_hand_strikes() {
    let mut t = table();
    let e = &mut t.e;
    let z = e.add_minion(SeatId(1), vampire_with("CEL", 1), 4, false);
    hand(e, 0, &["Flash", "Flash"]);
    hand(e, 1, &["Flash"]);
    blocked_bleed(e, t.b, z);
    // Determine range: seat 0 maneuvers to long; it cannot maneuver again
    // until seat 1 does.
    pick(e, play(0, 0, t.b));
    // Seat 0's window elides (a second maneuver in a row is not offered).
    let req = expect(e, SeatId(1), WindowLabel::CombatDetermineRange);
    assert_eq!(plays(&req), vec![play(0, 0, z), play(0, 1, z)]);
    pick(e, play(0, 0, z)); // back to close
    let req = expect(e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert_eq!(plays(&req), vec![play(0, 0, t.b), play(0, 1, t.b)]);
    pick(e, play(0, 0, t.b)); // long again; seat 1 has nothing left
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(e, t.b), blood(e, z)), (4, 4));
}

// Rulebook "Strike Effects": a strike: combat ends resolves first and
// alone; Majesty [PRE] unlocks the vampire before combat ends.
#[test]
fn strike_combat_ends_resolves_first_and_unlocks() {
    let mut t = table();
    let e = &mut t.e;
    let z = e.add_minion(SeatId(1), vampire_with("PRE", 0), 4, false);
    hand(e, 1, &["Majesty"]);
    blocked_bleed(e, t.a, z);
    // Seat 0 hand strikes (elided); seat 1 chooses Majesty [PRE].
    pick(e, play(0, 1, z));
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(e, t.a), blood(e, z)), (4, 3));
    assert!(!e.state.minions[z.index()].locked);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
}

// Rulebook "Dodge": the dodging minion is protected from the opposing
// strike (Roundhouse [POT]: hand strike at +3).
#[test]
fn dodge_voids_the_opposing_strike() {
    let mut t = table();
    let e = &mut t.e;
    let z = e.add_minion(SeatId(1), vampire_with("CEL", 1), 4, false);
    hand(e, 0, &["Roundhouse"]);
    hand(e, 1, &["Side Strike"]);
    blocked_bleed(e, t.a, z);
    // Before strikes: Z declines the limited additional strike.
    expect(e, SeatId(1), WindowLabel::CombatBeforeStrikes);
    pick(e, Choice::Pass);
    pick(e, play(0, 1, t.a));
    pick(e, play(0, 0, z));
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(e, t.a), blood(e, z)), (4, 4));
}

// Hidden Strength: X is the blood cost, chosen at play (rules-qa 3.5), up
// to the vampire's blood and possibly more than needed; prevents X+1. The
// card is not offered with no damage to prevent (the prevention window is
// elided for the undamaged side).
// Rulings: [LSJ 19970224]; [RTR 20041202] [PIB 20121202]; [LSJ 20001114] [LSJ 20030121]; [LSJ 20030502].
#[test]
fn hidden_strength_x_terms_and_prevention() {
    let mut t = table();
    let e = &mut t.e;
    hand(e, 1, &["Hidden Strength"]);
    hand(e, 0, &["Roundhouse"]);
    blocked_bleed(e, t.a, t.x);
    pick(e, play(0, 1, t.a)); // 4 damage
    let req = expect(e, SeatId(1), WindowLabel::CombatPrevention);
    assert_eq!(plays(&req), vec![play(0, 0, t.x), play(0, 1, t.x)]);
    pick(e, play(0, 0, t.x));
    let req = expect(e, SeatId(1), WindowLabel::PlayTerms);
    assert_eq!(req.options, (0..=4).map(set_x).collect::<Vec<_>>());
    pick(e, set_x(2)); // prevent 3 of 4, for 2 blood
                       // 1 damage left, nothing else to play: mended.
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(e, t.x), 1);
    assert!(!e.state.minions[t.x.index()].in_torpor);
    assert_eq!(blood(e, t.a), 3);
}

// Rulebook "Aggravated damage" and Soak rulings: Soak cannot prevent
// aggravated damage, so it is not offered against Body Flare; the first
// aggravated point wounds, each further one burns 1 blood.
// Rulings: [LSJ 20040812-2] [PIB 20130327].
#[test]
fn soak_cannot_prevent_aggravated_damage() {
    let mut t = table();
    let e = &mut t.e;
    hand(e, 1, &["Soak"]);
    hand(e, 0, &["Body Flare"]);
    let p = e.add_minion(SeatId(0), vampire_with("PRO", 1), 4, false);
    blocked_bleed(e, p, t.x);
    pick(e, play(0, 0, p)); // 2 aggravated, 2 blood
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.x.index()].in_torpor);
    assert_eq!(blood(e, t.x), 3);
    assert_eq!(blood(e, p), 4 - 2 - 1);
}

// Rulebook "Additional Strikes": one limited source per round — after
// Blur [CEL] (2 additional strikes) Side Strike [CEL] is no longer
// offered; the extra pairs are the striker's alone.
#[test]
fn additional_strikes_limited_once_per_round() {
    let mut t = table();
    let e = &mut t.e;
    hand(e, 0, &["Blur", "Side Strike"]);
    blocked_bleed(e, t.b, t.x);
    let req = expect(e, SeatId(0), WindowLabel::CombatBeforeStrikes);
    assert_eq!(plays(&req).len(), 3);
    pick(e, Choice::Pass);
    // B's strike (Side Strike's dodge is the alternative), then the
    // additional-strike window: Blur [CEL].
    pick(e, Choice::HandStrike);
    expect(e, SeatId(0), WindowLabel::CombatBeforeResolution);
    pick(e, Choice::Pass);
    let req = expect(e, SeatId(0), WindowLabel::CombatAdditionalStrikes);
    assert_eq!(plays(&req).len(), 3);
    pick(e, play(0, 1, t.b));
    // Side Strike [CEL] is masked out now (the window elides); two more
    // pairs in which B strikes alone (the dodge is its other choice) and
    // X has no strike.
    for _ in 0..2 {
        let req = expect(e, SeatId(0), WindowLabel::CombatStrike);
        assert_eq!(req.options, vec![Choice::HandStrike, play(0, 0, t.b)]);
        pick(e, Choice::HandStrike);
    }
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(e, t.b), blood(e, t.x)), (4 - 1 - 1, 4 - 3));
}

// Immortal Grapple [POT]: only hand strikes this round (Majesty masked
// out); the optional press continues to a second round, which is at close
// range with the determine range step skipped.
// Rulings: [RTR 19970630] [ANK 20180720].
#[test]
fn immortal_grapple_bars_non_hand_strikes_and_fixes_next_round_close() {
    let mut t = table();
    let e = &mut t.e;
    let z = e.add_minion(SeatId(1), vampire_with("PRE", 0), 4, false);
    hand(e, 0, &["Immortal Grapple"]);
    hand(e, 1, &["Majesty", "Flash"]);
    e.state.minions[z.index()].blood = 6;
    blocked_bleed(e, t.a, z);
    expect(e, SeatId(1), WindowLabel::CombatDetermineRange);
    pick(e, Choice::Pass);
    pick(e, play(0, 1, t.a));
    // Z's strike choice elides: hands only (Majesty masked out).
    // Press step: A's banked press continues.
    let req = expect(e, SeatId(0), WindowLabel::CombatPress);
    assert_eq!(req.options, vec![Choice::Pass, Choice::UsePress]);
    pick(e, Choice::UsePress);
    // Z may press to end with Flash [cel]; it declines.
    let req = expect(e, SeatId(1), WindowLabel::CombatPress);
    assert_eq!(plays(&req), vec![play(1, 0, z)]);
    pick(e, Choice::Pass);
    // Round 2: no determine range step (Flash's maneuver would have been
    // offered there); Majesty is back.
    let req = expect(e, SeatId(1), WindowLabel::CombatStrike);
    assert_eq!(plays(&req), vec![play(0, 0, z), play(0, 1, z)]);
    pick(e, play(0, 1, z));
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(e, t.a), blood(e, z)), (3, 6 - 1 - 1));
}

// Spec §4.9.1–4.9.2 — Telepathic Tracking [AUS] replaces the would-end
// event with a new round; the Form of Mist [PRO] after-combat continuation
// is lost when the combat continues.
// Rulings: [RTR 20020501] [LSJ 19980109]; [RTR 20030519].
#[test]
fn would_end_replacement_loses_the_after_combat_rider() {
    let mut t = table();
    let e = &mut t.e;
    let p = e.add_minion(SeatId(0), vampire_with("PRO", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("AUS", 0), 4, false);
    hand(e, 0, &["Form of Mist"]);
    hand(e, 1, &["Telepathic Tracking"]);
    blocked_bleed(e, p, z);
    pick(e, play(0, 1, p)); // strike: combat ends
                            // End of round: seat 0 first (nothing), seat 1 plays the replacement.
    let req = expect(e, SeatId(1), WindowLabel::CombatEndOfRound);
    assert_eq!(plays(&req), vec![play(0, 1, z)]);
    pick(e, play(0, 1, z));
    // Round 2: hand strikes; no continuation offered after combat.
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(e, p), blood(e, z)), (3, 4 - 1 - 1));
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
}

// rules-qa 3.4 — continuation keeps the declines: on a hunt (undirected,
// +1 stealth) the prey declines and the predator blocks with intercept;
// after Form of Mist [PRO] the action continues with +1 stealth, the prey
// gets no new attempt although it could now reach the stealth, and the
// blocker unlocked with Cats' Guidance [ani] in the after-combat window.
// Rulings: [RTR 19980623] [LSJ 20040219] [ANK 20221130]; [ANK 20190116].
#[test]
fn continuation_keeps_declines() {
    let mut e = Engine::bare(3);
    let p = e.add_minion(SeatId(0), vampire_with("PRO", 0), 4, false);
    let x = e.add_minion(SeatId(1), vampire_with("ANI", 0), 4, false);
    let z = e.add_minion(SeatId(2), vampire_with("ANI", 1), 4, false);
    e.state.turn = 4;
    hand(&mut e, 0, &["Form of Mist"]);
    hand(&mut e, 1, &["Cats' Guidance", "Cats' Guidance"]);
    hand(&mut e, 2, &["Cats' Guidance", "Cats' Guidance"]);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: p,
            action: ActionKind::Hunt,
            target: None,
        },
    );
    // Prey declines; predator attempts and reaches 1 intercept.
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(x)));
    pick(&mut e, Choice::Pass);
    pick(&mut e, block(z));
    pick(&mut e, play(0, 1, z));
    // Combat: P plays Form of Mist [PRO].
    pick(&mut e, play(0, 1, p));
    // After combat: P may continue; Z may unlock (Cats' Guidance [ani]).
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionAfterCombat);
    assert_eq!(req.options, vec![Choice::Pass, Choice::ContinueAction]);
    pick(&mut e, Choice::ContinueAction);
    let req = expect(&mut e, SeatId(2), WindowLabel::ActionAfterCombat);
    assert_eq!(plays(&req), vec![play(0, 0, z)]);
    pick(&mut e, play(0, 0, z));
    // Back in the block loop at stealth 2: the prey's decline stands (X
    // could reach 2 intercept, so a reset would offer the attempt), Z's
    // attempt is pruned (1 reachable) — the hunt resolves.
    loop {
        let req = request(&mut e);
        if e.last_outcome.is_some() {
            break;
        }
        assert!(
            !req.options.contains(&block(x)),
            "the prey got a new attempt: {:?}",
            req.options
        );
        pick(&mut e, Choice::Pass);
    }
    assert!(!e.state.minions[z.index()].locked);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
    assert_eq!(blood(&e, p), 4 - 1 + 1);
}

// rules-qa 2.2 — `COMBAT_UNTIMED`: Claws of the Dead [pro] is offered at
// every impulse before the strike resolution it affects, including after
// both strikes are chosen (rulings), and not in the prevention window; the
// hand strike becomes aggravated.
// Rulings: [RTR 19960112].
#[test]
fn combat_untimed_deadline_precedes_step() {
    let mut t = table();
    let e = &mut t.e;
    let p = e.add_minion(SeatId(0), vampire_with("PRO", 1), 4, false);
    hand(e, 0, &["Claws of the Dead"]);
    hand(e, 1, &["Indomitability"]);
    blocked_bleed(e, p, t.x);
    for w in [
        WindowLabel::CombatBeforeRange,
        WindowLabel::CombatDetermineRange,
        WindowLabel::CombatBeforeStrikes,
    ] {
        let req = expect(e, SeatId(0), w);
        assert!(plays(&req).contains(&play(0, 0, p)), "{w:?}");
        // The [PRO] maneuver only in determine range.
        assert_eq!(
            plays(&req).contains(&play(0, 1, p)),
            w == WindowLabel::CombatDetermineRange
        );
        pick(e, Choice::Pass);
    }
    let req = expect(e, SeatId(0), WindowLabel::CombatBeforeResolution);
    assert_eq!(plays(&req), vec![play(0, 0, p)]);
    pick(e, play(0, 0, p));
    // Prevention on P (1 normal from X): nothing to play. On X: 1
    // aggravated — Indomitability can prevent it.
    let req = expect(e, SeatId(1), WindowLabel::CombatPrevention);
    assert_eq!(plays(&req), vec![play(0, 0, t.x), play(0, 1, t.x)]);
    pick(e, Choice::Pass);
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.x.index()].in_torpor);
    assert_eq!(blood(e, t.x), 4);
    assert_eq!(blood(e, p), 4 - 1 - 1);
}

// Taste of Vitae: at the end of the round, blood equal to what the
// opposing vampire lost to damage this round, capped by capacity.
#[test]
fn taste_of_vitae_gains_blood_lost_by_opponent() {
    let mut t = table();
    let e = &mut t.e;
    hand(e, 0, &["Roundhouse", "Taste of Vitae"]);
    hand(e, 1, &["Taste of Vitae"]);
    blocked_bleed(e, t.a, t.x);
    pick(e, play(0, 0, t.a)); // +2: 3 damage
    let req = expect(e, SeatId(0), WindowLabel::CombatEndOfRound);
    assert_eq!(plays(&req), vec![play(0, 0, t.a)]);
    pick(e, play(0, 0, t.a));
    pick(e, play(0, 0, t.x));
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    let cap = e.state.minions[t.a.index()].capacity();
    assert_eq!(blood(e, t.a), (4 - 1 + 3).min(cap));
    assert_eq!(blood(e, t.x), 4 - 3 + 1);
}

// Theft of Vitae [THA]: a ranged strike works at long range where the
// hand strike does not; stolen blood moves before damage is mended.
#[test]
fn ranged_steal_at_long_range() {
    let mut t = table();
    let e = &mut t.e;
    let z = e.add_minion(SeatId(0), find_card("Kassandra Tassaki"), 2, false); // CEL THA
    hand(e, 0, &["Flash", "Theft of Vitae"]);
    blocked_bleed(e, z, t.x);
    pick(e, play(0, 0, z)); // maneuver to long
    pick(e, play(0, 1, z)); // steal 2
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(e, z), blood(e, t.x)), (4, 2));
}

// Rulebook "Leave Torpor": blocked, no combat — the blocking vampire may
// diablerise the acting vampire, and the blood hunt follows.
#[test]
fn blocked_leave_torpor_blocker_may_diablerise() {
    let mut t = table();
    let e = &mut t.e;
    let z = e.add_minion(SeatId(1), vampire_with("ANI", 0), 4, false);
    let tor = e.add_minion(SeatId(0), vampire_with("DOM", 0), 2, true);
    hand(e, 1, &["Cats' Guidance"]);
    pick(
        e,
        Choice::TakeAction {
            minion: tor,
            action: ActionKind::LeaveTorpor,
            target: None,
        },
    );
    pick(e, block(z));
    pick(e, play(0, 1, z)); // +1 intercept
    let req = expect(e, SeatId(1), WindowLabel::TorporBlock);
    assert_eq!(req.options, vec![Choice::Pass, Choice::Diablerise]);
    pick(e, Choice::Diablerise);
    assert!(!e.state.minions[tor.index()].in_play);
    assert_eq!(blood(e, z), 6.min(e.state.minions[z.index()].capacity()));
    // The blood hunt rotation starts at the diablerist's controller; the
    // hunt is inside the action (rules-qa 5.8), which ends blocked after it.
    expect(e, SeatId(1), WindowLabel::BloodHuntPolling);
    assert_eq!(e.last_outcome, None);
    pick(e, Choice::Pass);
    let _ = request(e);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
}

// Rulebook "Combat" / spec §4.1.3: only combat cards are playable during
// combat — a strike card's as-played window offers no wake to a locked
// vampire elsewhere (Eyes of Argus [AUS] lists CARD_AS_PLAYED), so the
// window elides.
#[test]
fn no_reaction_cards_during_combat() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), vampire_with("POT", 0), 4, false);
    let x = e.add_minion(SeatId(1), vampire_with("FOR", 0), 4, false);
    let w = e.add_minion(SeatId(2), vampire_with("AUS", 0), 4, false);
    e.state.minions[w.index()].locked = true;
    e.state.turn = 4;
    hand(&mut e, 0, &["Roundhouse"]);
    hand(&mut e, 2, &["Eyes of Argus"]);
    blocked_bleed(&mut e, a, x);
    // During the action the wake is offered to seat 2; it declines.
    while request(&mut e).window == WindowLabel::ActionBlock {
        expect(&mut e, SeatId(2), WindowLabel::ActionBlock);
        pick(&mut e, Choice::Pass);
    }
    pick(&mut e, play(0, 0, a));
    // No as-played wake for seat 2; the rest of seat 0's turn elides.
    expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[w.index()].locked);
    assert_eq!(blood(&e, x), 1);
}
