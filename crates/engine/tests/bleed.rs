//! Slice 2 of the effects framework: the bleed with action modifiers and
//! reactions — the as-announced window, the block loop with stealth and
//! intercept cards (rulebook A/B/C, rules-qa 1.3, "only when needed" per
//! 3.2), wakes in the as-played window (2.4), bounces (Deflection), Archon
//! Investigation, Consign to Oblivion, the employ action and an unlock-phase
//! ability (Homunculus). Windows with a single option are elided, so the
//! scripts below only stop where a real decision exists.

use zvtes::cards::AbilitySource;
use zvtes::cards::Target;
use zvtes::decision::{Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::CardId;
use zvtes::state::{find_card, find_library, CardInPlay};

mod common;
use common::*;

fn redirect(mode: u8, actor: MinionId, to: SeatId) -> Choice {
    Choice::Play {
        card: 0,
        mode,
        actor: Some(actor),
        target: Target::Seat(to),
    }
}

/// 3 seats at seat 0's minion phase, turn 4, 6 blood each. Seat 0: A
/// (DOM), P (PRO), O (OBL). Seat 1: B (AUS, basic dom), C (DOM and AUS).
/// Seat 2: D.
struct Table {
    e: Engine,
    a: MinionId,
    p: MinionId,
    o: MinionId,
    b: MinionId,
    c: MinionId,
    d: MinionId,
}

fn table() -> Table {
    let mut e = Engine::bare(3);
    // No printed text: later definitions cannot shift these picks.
    let a = e.add_minion(SeatId(0), find_card("Madison"), 6, false); // DOM
    let p = e.add_minion(SeatId(0), find_card("Crystal Lynn"), 6, false); // PRO
    let o = e.add_minion(SeatId(0), find_card("Monica Giovanni"), 6, false); // AUS FOR OBL
    let b = e.add_minion(SeatId(1), find_card("Donny Kowalczyk"), 6, false); // AUS, basic dom
    let c = e.add_minion(SeatId(1), find_card("Gelasia Fotiou"), 6, false); // AUS DOM
    let d = e.add_minion(SeatId(2), find_card("Malachi"), 6, false); // AUS DOM FOR
    e.state.turn = 4;
    Table {
        e,
        a,
        p,
        o,
        b,
        c,
        d,
    }
}

// rules-qa 3.2 — stealth only when needed: Earth Control is masked with no
// attempt pending and offered once an attempt has intercept ≥ stealth. A
// failed attempt goes back to A (rulebook B.4); with no intercept in hand a
// new attempt is pruned as unreachable and the seat's pass is the final
// decline.
#[test]
fn stealth_only_when_needed() {
    let mut t = table();
    let e = &mut t.e;
    e.state.seats[0].hand.push(find_library("Earth Control"));
    pick(e, bleed(t.p));
    // No attempt yet: Earth Control is not offered, seat 0's window elides.
    let req = expect(e, SeatId(1), WindowLabel::ActionBlock);
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            Choice::DeclareBlock { minion: t.b },
            Choice::DeclareBlock { minion: t.c }
        ]
    );
    pick(e, Choice::DeclareBlock { minion: t.b });
    // Impulse back to seat 0: 0 intercept ≥ 0 stealth, stealth is needed.
    let req = expect(e, SeatId(0), WindowLabel::ActionBlock);
    assert_eq!(plays(&req), vec![play(0, 0, t.p), play(0, 1, t.p)]);
    pick(e, play(0, 1, t.p)); // +2 stealth, 1 blood
                              // Every remaining window elides: the attempt fails at closure, back to
                              // A, seat 1's new attempt is pruned (nothing reaches 2 intercept), C,
                              // resolution.
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 29);
    assert_eq!(e.state.minions[t.p.index()].blood, 5);
    assert!(e.state.seats[0].has_edge);
}

// Bonding [DOM] is "+1 stealth and +1 bleed": the stealth is the effect's,
// so the mode is not playable while no block attempt matches the current
// stealth — only [dom] is offered before an attempt; once B attempts at 0
// intercept both modes are, and the play beats the attempt.
// Rulings: [TOM 19951109].
#[test]
fn bonding_superior_needs_the_stealth_to_be_needed() {
    let mut t = table();
    let e = &mut t.e;
    e.state.seats[0].hand.push(find_library("Bonding"));
    pick(e, bleed(t.a));
    let req = expect(e, SeatId(0), WindowLabel::ActionBlock);
    assert_eq!(plays(&req), vec![play(0, 0, t.a)]);
    pick(e, Choice::Pass);
    pick(e, Choice::DeclareBlock { minion: t.b });
    let req = expect(e, SeatId(0), WindowLabel::ActionBlock);
    assert_eq!(plays(&req), vec![play(0, 0, t.a), play(0, 1, t.a)]);
    pick(e, play(0, 1, t.a));
    // Stealth 1, bleed 2; the attempt fails, nothing reaches 1 intercept.
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 28);
}

// rules-qa 3.2 — "even if stealth is not yet needed": Form of the Cobra
// [PRO] is offered as the action is announced with no attempt pending;
// then an intercept card is offered only while intercept is needed.
#[test]
fn stealth_waived_by_card_text() {
    let mut t = table();
    let e = &mut t.e;
    e.state.seats[0]
        .hand
        .push(find_library("Form of the Cobra"));
    e.state.seats[1].hand.push(find_library("Eyes of Argus"));
    pick(e, bleed(t.p));
    let req = expect(e, SeatId(0), WindowLabel::ActionAsAnnounced);
    assert_eq!(plays(&req), vec![play(0, 1, t.p)]);
    pick(e, play(0, 1, t.p));
    // Stealth 1; both B and C have Auspex, so Eyes of Argus makes 2
    // intercept reachable for either: both attempts are offered.
    let req = expect(e, SeatId(1), WindowLabel::ActionBlock);
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            Choice::DeclareBlock { minion: t.b },
            Choice::DeclareBlock { minion: t.c }
        ]
    );
    pick(e, Choice::DeclareBlock { minion: t.b });
    // Seat 0 has nothing (elided); intercept is needed for B: 1 > 0.
    let req = expect(e, SeatId(1), WindowLabel::ActionBlock);
    assert_eq!(plays(&req), vec![play(0, 0, t.b)]);
    pick(e, play(0, 0, t.b));
    // 2 ≥ 1 at closure: blocked; combat, both lose 1 blood.
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[t.p.index()].blood, 5);
    assert_eq!(e.state.minions[t.b.index()].blood, 5);
    assert!(e.state.minions[t.b.index()].locked);
    assert_eq!(e.state.seats[1].pool, 30);
}

// rules-qa 1.3 — an attempt persists until all pass; a failed attempt
// returns to A where the same Methuselah may attempt again; a pass in A
// with no attempt is the final decline.
#[test]
fn block_attempt_persists_until_all_pass() {
    let mut t = table();
    let e = &mut t.e;
    e.state.seats[0]
        .hand
        .push(find_library("Form of the Cobra"));
    e.state.seats[1].hand.push(find_library("Eyes of Argus"));
    pick(e, bleed(t.p));
    pick(e, play(0, 1, t.p)); // stealth 1 as announced
    pick(e, Choice::DeclareBlock { minion: t.b });
    pick(e, Choice::Pass); // seat 1 holds Eyes of Argus back
                           // Closure at 0 < 1: the attempt fails, back to A, where seat 1 (which
                           // declared rather than passed) is offered a new attempt.
    let req = expect(e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&Choice::DeclareBlock { minion: t.b }));
    pick(e, Choice::Pass); // final decline
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 29);
}

// rules-qa 15.2 — intercept gained during a failed attempt stays for the
// rest of the action: B's first Eyes of Argus falls short of P's two Earth
// Controls, and B's new attempt reaches 4 with the second card only because
// the first card's +2 is kept.
#[test]
fn intercept_from_a_failed_attempt_carries_into_a_new_one() {
    let mut t = table();
    let e = &mut t.e;
    e.state.seats[0]
        .hand
        .extend([find_library("Earth Control"), find_library("Earth Control")]);
    e.state.seats[1]
        .hand
        .extend([find_library("Eyes of Argus"), find_library("Eyes of Argus")]);
    pick(e, bleed(t.p));
    pick(e, Choice::DeclareBlock { minion: t.b });
    pick(e, play(0, 1, t.p)); // stealth 2
    pick(e, play(0, 0, t.b)); // +2 intercept
    pick(e, play(0, 1, t.p)); // stealth 4
    pick(e, Choice::Pass); // seat 1 holds the second Eyes of Argus back
                           // The attempt fails; B's kept +2 and the card in hand reach 4.
    let req = expect(e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&Choice::DeclareBlock { minion: t.b }));
    pick(e, Choice::DeclareBlock { minion: t.b });
    pick(e, play(0, 0, t.b));
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 30);
    assert!(e.state.minions[t.b.index()].locked);
}

// rules-qa 1.3 — only a change of target reopens block attempts: after seat
// 1 declines, its Deflection sends the bleed to seat 2, who may block; seat
// 1 stays out. Deflection rulings: after blocks are declined, to a valid
// target other than the acting controller and oneself.
// Rulings: [ANK 20211003]; [PIB 20130711].
#[test]
fn block_reopen_only_on_bounce() {
    let mut t = table();
    let e = &mut t.e;
    e.state.seats[1].hand.push(find_library("Deflection"));
    pick(e, bleed(t.a));
    pick(e, Choice::Pass); // seat 1 declines (final)
                           // State C: B (basic dom) and C (superior) may redirect, to seat 2 only.
    let req = expect(e, SeatId(1), WindowLabel::ActionBlocksDeclined);
    assert_eq!(
        plays(&req),
        vec![
            redirect(0, t.b, SeatId(2)),
            redirect(0, t.c, SeatId(2)),
            redirect(1, t.c, SeatId(2))
        ]
    );
    pick(e, redirect(0, t.c, SeatId(2)));
    // Once it resolves: C locked (basic Deflection), 1 blood paid; seat 2
    // is the target and may attempt with D; seat 1 may not.
    let req = expect(e, SeatId(2), WindowLabel::ActionBlock);
    assert!(e.state.minions[t.c.index()].locked);
    assert_eq!(e.state.minions[t.c.index()].blood, 5);
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::DeclareBlock { minion: t.d }]
    );
    pick(e, Choice::Pass);
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((e.state.seats[1].pool, e.state.seats[2].pool), (30, 29));
    assert!(e.state.seats[0].has_edge);
}

#[test]
fn archon_investigation_needs_a_bleed_of_four_and_burns_the_bleeder() {
    let mut t = table();
    let e = &mut t.e;
    e.state.seats[0].hand.push(find_library("Conditioning"));
    e.state.seats[1]
        .hand
        .push(find_library("Archon Investigation"));
    pick(e, bleed(t.a));
    let req = expect(e, SeatId(0), WindowLabel::ActionBlock);
    assert_eq!(plays(&req), vec![play(0, 0, t.a), play(0, 1, t.a)]);
    pick(e, Choice::Pass);
    pick(e, Choice::Pass); // seat 1 declines; bleed 1: Archon not offered in C
    let req = expect(e, SeatId(0), WindowLabel::ActionBlocksDeclined);
    assert_eq!(plays(&req), vec![play(0, 0, t.a), play(0, 1, t.a)]);
    pick(e, play(0, 1, t.a)); // [DOM] +3: bleed 4, 1 blood
    let archon = Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::None,
    };
    let req = expect(e, SeatId(1), WindowLabel::ActionBlocksDeclined);
    assert_eq!(plays(&req), vec![archon]);
    pick(e, archon);
    // Out of turn: 3 pool paid, the bleeder burned, the action fails.
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.a.index()].in_play);
    assert_eq!(e.state.seats[1].pool, 27);
    assert!(!e.state.seats[0].has_edge);
    assert!(e.state.seats[1].out_of_turn.is_some());
}

fn archon_sequence(t: &mut Table) -> Choice {
    let e = &mut t.e;
    e.state.seats[0].hand.push(find_library("Conditioning"));
    e.state.seats[1]
        .hand
        .push(find_library("Archon Investigation"));
    pick(e, bleed(t.a));
    pick(e, Choice::Pass);
    pick(e, Choice::Pass);
    pick(e, play(0, 1, t.a)); // [DOM] +3: bleed 4
    let archon = Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::None,
    };
    pick(e, archon);
    archon
}

// Tension in the Ranks in play: Archon Investigation burns a ready minion,
// so the bleeder's Methuselah burns 1 pool inside the play's resolution —
// at 1 pool they are ousted there, and the turn passes (no minion phase for
// them); their predator scores.
#[test]
fn archon_investigation_under_tension_in_the_ranks_ousts_the_bleeder_s_methuselah() {
    let mut t = table();
    t.e.state.seats[0].pool = 1;
    t.e.state.cards.push(CardInPlay {
        card: find_library("Tension in the Ranks"),
        owner: SeatId(2),
        controller: SeatId(2),
        host: None,
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
    archon_sequence(&mut t);
    let e = &mut t.e;
    let req = request(e);
    assert_eq!(req.seat, SeatId(1));
    assert!(e.state.seats[0].ousted);
    assert!(!e.state.minions[t.a.index()].in_play);
    assert_eq!(e.state.seats[2].half_vps, 2);
    assert_eq!(e.state.seats[2].pool, 36);
    assert_eq!(e.state.living(), 2);
}

// Archon Investigation at exactly 3 pool: its player pays down to 0 and is
// ousted by their own out-of-turn play; the active Methuselah's turn goes
// on (the failed action ends, their minion phase resumes) and they score
// the oust as the predator.
#[test]
fn archon_investigation_at_three_pool_ousts_its_player_and_the_turn_goes_on() {
    let mut t = table();
    t.e.state.seats[1].pool = 3;
    archon_sequence(&mut t);
    let e = &mut t.e;
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert!(!e.state.minions[t.a.index()].in_play);
    assert_eq!(e.state.seats[0].half_vps, 2);
    assert_eq!(e.state.seats[0].pool, 36);
    assert_eq!(e.state.living(), 2);
}

// rules-qa 2.4 — a wake is playable in the as-played window of a card
// played during the action, before that card's effect exists: Eyes of
// Argus wakes B before Consign to Oblivion raises reaction costs.
// Rulings: [TOM 19951129].
#[test]
fn wake_playable_in_as_played_window() {
    let mut t = table();
    let e = &mut t.e;
    e.state.minions[t.b.index()].locked = true;
    e.state.seats[0]
        .hand
        .push(find_library("Consign to Oblivion"));
    e.state.seats[1]
        .hand
        .extend([find_library("Eyes of Argus"), find_library("Deflection")]);
    pick(e, bleed(t.o));
    let req = expect(e, SeatId(0), WindowLabel::ActionAsAnnounced);
    assert_eq!(plays(&req), vec![play(0, 0, t.o), play(0, 1, t.o)]);
    pick(e, play(0, 0, t.o));
    // Consign's as-played window: seat 1's only play is the wake (B is
    // locked); not the intercept, not Deflection.
    let req = expect(e, SeatId(1), WindowLabel::CardAsPlayed);
    assert_eq!(plays(&req), vec![play(0, 1, t.b)]);
    assert!(e.state.layers.is_empty()); // Consign has not resolved
    pick(e, play(0, 1, t.b));
    // Both settle; state A: B, woken, may attempt to block.
    let req = expect(e, SeatId(1), WindowLabel::ActionBlock);
    assert_eq!(e.state.layers.len(), 1);
    assert!(req.options.contains(&Choice::DeclareBlock { minion: t.b }));
    pick(e, Choice::Pass); // declines
                           // C: Deflection now costs 2 blood; C (6 blood) and woken B may play it.
    let req = expect(e, SeatId(1), WindowLabel::ActionBlocksDeclined);
    assert_eq!(plays(&req).len(), 3);
}

#[test]
fn consign_raises_reaction_costs() {
    let mut t = table();
    let e = &mut t.e;
    e.state.minions[t.b.index()].blood = 1;
    e.state.minions[t.c.index()].blood = 1;
    e.state.seats[0]
        .hand
        .push(find_library("Consign to Oblivion"));
    e.state.seats[1].hand.push(find_library("Deflection"));
    pick(e, bleed(t.o));
    pick(e, play(0, 0, t.o));
    pick(e, Choice::Pass); // seat 1 declines
                           // Deflection costs 2 blood now; nobody can pay: C elides, bleed lands.
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 29);
    assert_eq!(e.state.seats[1].hand.len(), 1);
}

#[test]
fn as_played_window_wakes_and_cancels_only() {
    let mut t = table();
    let e = &mut t.e;
    e.state.seats[0]
        .hand
        .push(find_library("Consign to Oblivion"));
    e.state.seats[1]
        .hand
        .extend([find_library("Eyes of Argus"), find_library("Conditioning")]);
    pick(e, bleed(t.o));
    pick(e, play(0, 0, t.o));
    // B is unlocked: no wake to play; the intercept mode is not an
    // as-played play; Conditioning is a modifier. The as-played window
    // elides entirely and state A opens on seat 1's attempts.
    let req = expect(e, SeatId(1), WindowLabel::ActionBlock);
    assert!(plays(&req).is_empty());
}

#[test]
fn consign_superior_defers_reaction_replacement_to_the_end_of_the_action() {
    let mut t = table();
    let e = &mut t.e;
    e.state.seats[0]
        .hand
        .push(find_library("Consign to Oblivion"));
    e.state.seats[1].hand.push(find_library("Deflection"));
    e.state.seats[1].library.push(find_library("Villein"));
    pick(e, bleed(t.o));
    pick(e, play(0, 1, t.o)); // [OBL]
    pick(e, Choice::Pass); // seat 1 declines
    pick(e, redirect(1, t.c, SeatId(2)));
    // Deflection resolved (2 blood): not replaced yet.
    let req = expect(e, SeatId(2), WindowLabel::ActionBlock);
    assert!(req.options.contains(&Choice::DeclareBlock { minion: t.d }));
    assert_eq!(e.state.minions[t.c.index()].blood, 4);
    assert!(e.state.seats[1].hand.is_empty());
    pick(e, Choice::Pass); // seat 2 declines
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].hand, vec![find_library("Villein")]);
    assert!(e.state.layers.is_empty()); // action layers popped
    assert_eq!(e.state.seats[2].pool, 29);
}

#[test]
fn homunculus_employ_then_unlock_ability_and_nra_by_card_name() {
    let mut t = table();
    let e = &mut t.e;
    let homunculus = find_library("Homunculus");
    e.state.seats[0].hand.extend([homunculus, homunculus]);
    let req = expect(e, SeatId(0), WindowLabel::MinionPhase);
    // P (PRO) may employ at either level; A and O (no Protean) may not.
    let employ = |card, mode| play(card, mode, t.p);
    assert_eq!(
        plays(&req),
        vec![employ(0, 0), employ(0, 1), employ(1, 0), employ(1, 1)]
    );
    pick(e, employ(0, 1));
    // +1 stealth, undirected, no intercept anywhere: every window elides;
    // the retainer enters play with 2 life.
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards.len(), 1);
    assert_eq!(
        (e.state.cards[0].host, e.state.cards[0].counters),
        (Some(t.p), 2)
    );
    assert!(e.state.minions[t.p.index()].locked);
    // NRA: even unlocked, P cannot employ a second Homunculus this turn.
    e.state.minions[t.p.index()].locked = false;
    let req = expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert!(plays(&req).is_empty());
    e.state.minions[t.p.index()].locked = true;
    pick(e, Choice::Pass); // minion phase; influence elides
    pick(e, Choice::Pass); // discard phase
                           // Seat 1's unlock phase: seat 0 may burn 1 blood from P to unlock it.
    let req = expect(e, SeatId(0), WindowLabel::UnlockAbilities);
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            Choice::UseAbility {
                source: AbilitySource::Card(CardId(0)),
                ability: 0,
                actor: None,
                target: Target::None
            }
        ]
    );
    pick(
        e,
        Choice::UseAbility {
            source: AbilitySource::Card(CardId(0)),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    let _ = request(e);
    assert!(!e.state.minions[t.p.index()].locked);
    assert_eq!(e.state.minions[t.p.index()].blood, 5);
}

// [LSJ 20010817] [PIB 20131013] Archon Investigation "targets the bleeding
// vampire (eg. costs one more against Secure Haven)".
#[test]
fn archon_investigation_costs_one_more_against_a_secure_haven() {
    let mut t = table();
    let e = &mut t.e;
    e.state.cards.push(zvtes::state::CardInPlay {
        card: find_library("Secure Haven"),
        owner: SeatId(0),
        controller: SeatId(0),
        host: Some(t.a),
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
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::Haven,
        source: zvtes::state::CardId(e.state.cards.len() as u32 - 1),
    });
    archon_sequence(&mut t);
    let e = &mut t.e;
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.a.index()].in_play);
    assert_eq!(e.state.seats[1].pool, 26, "3 + 1");
}
