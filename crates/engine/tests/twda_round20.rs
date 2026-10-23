//! Legacy cards, round 20 (`ADDED_BATCHES`, slice 16af): the cards players
//! asked for (the server's /admin/unsupported of 2026-10-23: a Kiasyd
//! deck, a Lasombra one, Catacombs, Humanitas, Nocturn) and the two cards
//! of VEKN's two-player list outside the pool. Readings in rules-qa Batch
//! 89. Windows with a single option are elided, so the scripts only stop
//! where a real decision exists.

use zvtes::cards::{AbilitySource, Stat, Target};
use zvtes::crypt::CRYPT;
use zvtes::decision::{ActionKind, Choice, WindowLabel};
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
    e.notes = Some(Vec::new());
    e
}

/// A card already in play for `seat`, on `host` if given, with its first
/// mode's layers (no play): its id.
fn in_play(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>) -> CardId {
    let card = find_library(name);
    e.state
        .cards
        .push(CardInPlay::new(card, SeatId(seat), host, 0));
    let id = CardId(e.state.cards.len() as u32 - 1);
    for effect in zvtes::cards::lookup(card).expect("definition").modes[0].effects {
        if let zvtes::cards::Effect::Layer(modifier) = effect {
            e.state.layers.push(zvtes::cards::Layer {
                scope: zvtes::cards::Scope::InPlay,
                modifier: *modifier,
                source: id,
            });
        }
    }
    id
}

/// The `nth` vampire of `clan` with no printed-text definition.
fn of_clan(clan: &str, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.clan == clan
                && c.title == zvtes::crypt::Title::None
                && zvtes::cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no textless {clan}"))
}

/// A vampire's own action `ability` at `target`.
fn own_at(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: Some(m),
        target,
    }
}

/// `card`'s `ability` used (`actor` the user, if any).
fn use_on(source: CardId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(source),
        ability,
        actor,
        target,
    }
}

fn pass_to(e: &mut Engine, seat: u8, window: WindowLabel) -> zvtes::decision::DecisionRequest {
    until(e, SeatId(seat), window)
}

// ---- Vampires -----------------------------------------------------------------

// Andrew Emory, Dame Hollerton and Randel, The Coward print no text;
// Banjoko's Black Hand and Seraph are generated traits (LSJ 20070322, ANK
// 20180807): no definition for any of them.
#[test]
fn the_textless_round_20_vampires_have_no_definition_and_their_traits() {
    for name in [
        "Andrew Emory",
        "Dame Hollerton",
        "Randel, The Coward",
        "Banjoko",
    ] {
        let c = find_card(name);
        assert!(zvtes::cards::vampire(c).is_none(), "{name}");
    }
    let b = &CRYPT[find_card("Banjoko") as usize];
    assert!(b.black_hand && b.seraph);
    let a = &CRYPT[find_card("Andrew Emory") as usize];
    assert_eq!(a.title, zvtes::crypt::Title::Bishop);
}

// Isanwayen: "If Isanwayen is ready during your master phase, you can lock
// him to get +1 master phase action."
#[test]
fn isanwayen_locks_for_a_master_phase_action() {
    let mut e = game(2);
    let i = e.add_minion(SeatId(0), find_card("Isanwayen"), 6, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    e.at_master_phase();
    hand(&mut e, 0, &["Dodge"]);
    let before = e.state.seats[0].mpa;
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(i),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    assert!(e.state.minions[i.index()].locked);
    assert_eq!(e.state.seats[0].mpa, before + 1);
}

// Isanwayen: "Rescuing him from torpor costs 1 additional blood" — 3, read
// as the rescue resolves (rules-qa 48.8).
#[test]
fn rescuing_isanwayen_costs_one_more_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Pherydima"), 6, false);
    let i = e.add_minion(SeatId(0), find_card("Isanwayen"), 1, true);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: ActionKind::Rescue,
            target: Some(i),
        },
    );
    // Who pays is chosen as announced: up to all 3 from the rescuer.
    let req = expect(&mut e, SeatId(0), WindowLabel::RescueCost);
    assert_eq!(
        req.options,
        vec![
            Choice::PayRescue { from_acting: 2 },
            Choice::PayRescue { from_acting: 3 }
        ]
    );
    pick(&mut e, Choice::PayRescue { from_acting: 3 });
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert!(!e.state.minions[i.index()].in_torpor);
    assert_eq!((blood(&e, a), blood(&e, i)), (3, 1));
}

// The Arcadian: "can bleed with +1 bleed as a +1 stealth Ⓓ action that
// costs 1 blood."
#[test]
fn the_arcadian_bleeds_at_plus_one_for_a_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Arcadian, The"), 5, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    pick(&mut e, own_at(a, 0, Target::None));
    request(&mut e);
    assert_eq!(e.action().expect("action").kind, ActionKind::Bleed);
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4, "1 blood paid");
    assert_eq!(pool(&e)[1], 10 - 2, "bled for 2");
}

// Omme Enberbenight: "Cards that require Mytherceria [myt] cost Omme 1 less
// blood": a [myt] mode, not another Discipline's (rules-qa 38.4).
#[test]
fn omme_pays_less_for_mytherceria_cards() {
    let mut e = game(2);
    let o = e.add_minion(SeatId(0), find_card("Omme Enberbenight"), 7, false);
    let st = &e.state;
    let at = |card, mode| PrintedCtx {
        card: Some((find_library(card), mode)),
        ..PrintedCtx::default()
    };
    // Gremlins [myt] (mode 1) vs [dom] (mode 0).
    assert_eq!(st.printed_mod(o, Stat::BloodCostOff, at("Gremlins", 1)), 1);
    assert_eq!(st.printed_mod(o, Stat::BloodCostOff, at("Gremlins", 0)), 0);
}

// Pherydima: "During your discard phase, you may burn a pool to unlock
// Pherydima."
#[test]
fn pherydima_burns_a_pool_to_unlock_in_the_discard_phase() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), find_card("Pherydima"), 8, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    pick(&mut e, action(p, ActionKind::Hunt));
    let req = pass_to(&mut e, 0, WindowLabel::DiscardPhase);
    let u = own_at(p, 0, Target::None);
    let u = req
        .options
        .iter()
        .copied()
        .find(|o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(m), .. } if *m == p))
        .unwrap_or(u);
    e.choose(index_of(&req, u));
    request(&mut e);
    assert!(!e.state.minions[p.index()].locked);
    assert_eq!(pool(&e)[0], 9);
}

// Roderick Phillips March: "While Roderick is ready, your minions get an
// additional +1 stealth on equip actions": himself and the others, an
// equip from hand (rules-qa 78.28, 80.44); not while he is in torpor.
#[test]
fn roderick_gives_stealth_on_equip_actions() {
    for torpor in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), find_card("Roderick Phillips March"), 7, torpor);
        let a = e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Kevlar Vest"]);
        pick(&mut e, play(0, 0, a));
        request(&mut e);
        let st = &e.state;
        let act = e.action().expect("equip action");
        let ctx = act.printed_ctx(st, None);
        assert_eq!(
            st.printed_mod(a, Stat::Stealth, ctx),
            if torpor { 0 } else { 1 },
            "torpor: {torpor}"
        );
    }
}

// Quincy, The Trapper: "may burn an ally controlled by your predator or
// prey with two +1 stealth Ⓓ actions" — the same action twice in two
// different turns (ANK 20170202).
// Rulings: [ANK 20170202].
#[test]
fn quincy_burns_an_ally_with_two_actions_in_two_turns() {
    let mut e = game(3);
    let q = e.add_minion(SeatId(0), find_card("Quincy, The Trapper"), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Mylan Horseed"));
    e.add_minion(SeatId(2), of_clan("Toreador", 0), 4, false);
    pick(&mut e, own_at(q, 0, Target::Minion(ally)));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert!(
        e.state.minions[ally.index()].in_play,
        "one action is not enough"
    );
    // The next turn of seat 0: the second action burns it.
    pick(&mut e, Choice::Pass);
    to_minion_phase(&mut e, SeatId(0));
    pick(&mut e, own_at(q, 0, Target::Minion(ally)));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play, "burned");
}

// ---- Action modifiers, reactions, combat -----------------------------------------

// Stone Travel [myt] +1 stealth against a block, Aura Absorption [myt] +1
// intercept back: each only when needed (rules-qa 3.2); the block made.
// Rulings: [LSJ 20011204-3] [PIB 20130704].
#[test]
fn stone_travel_and_aura_absorption_trade_stealth_and_intercept() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Dame Hollerton"), 5, false);
    let b = e.add_minion(SeatId(1), find_card("Pherydima"), 8, false);
    hand(&mut e, 0, &["Stone Travel"]);
    hand(&mut e, 1, &["Aura Absorption"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(b));
    // No intercept yet: the reaction is not offered.
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 1, b)));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 1, b));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert!(
        noted_combats(&mut e)
            .iter()
            .any(|c| matches!(c, zvtes::combat::CombatNote::Begins { .. })),
        "blocked: combat"
    );
}

// Aura Absorption [aus], Stone Travel [obt]: the blood burned is no cost —
// the card not played without it (ANK 20210226).
// Rulings: [ANK 20210226].
#[test]
fn burn_blood_modes_need_the_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 1, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Stone Travel"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(b));
    e.state.minions[a.index()].blood = 0;
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 3, false);
    let b = e.add_minion(SeatId(1), vampire_with("aus", 0), 3, false);
    hand(&mut e, 0, &["Stone Travel"]);
    hand(&mut e, 1, &["Aura Absorption"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(b));
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(blood(&e, a), 2, "[obt] burned 1");
    // b has the blood: [aus] offered, the block made; with none, not.
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, b)));
    e.state.minions[b.index()].blood = 0;
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, b));
}

// Stone Travel [MYT]: blocked, the acting vampire may cancel the combat;
// the action stays blocked (no bleed), and its controller may also fight.
// Rulings: [LSJ 20011204-3] [PIB 20130704].
#[test]
fn stone_travel_superior_may_cancel_the_combat_after_a_block() {
    for cancel in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Pherydima"), 8, false);
        let b = e.add_minion(SeatId(1), find_card("Isanwayen"), 6, false);
        hand(&mut e, 0, &["Stone Travel"]);
        hand(&mut e, 1, &["Aura Absorption"]);
        pick(&mut e, bleed(a));
        pick(&mut e, block(b));
        pick_when(&mut e, play(0, 2, a));
        pick_when(&mut e, play(0, 1, b));
        let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        assert_eq!(req.options, vec![Choice::Pass, Choice::CancelCombat]);
        pick(
            &mut e,
            if cancel {
                Choice::CancelCombat
            } else {
                Choice::Pass
            },
        );
        pass_to(&mut e, 0, WindowLabel::MinionPhase);
        let fought = noted_combats(&mut e)
            .iter()
            .any(|c| matches!(c, zvtes::combat::CombatNote::Strikes { .. }));
        assert_eq!(fought, !cancel, "cancel: {cancel}");
        assert_eq!(pool(&e)[1], 10, "blocked: no bleed");
    }
}

// Fae Contortion [myt] [REFLEX]: a grapple card canceled as it is played,
// its cost unpaid; it still counts as played (ANK 20190104).
// Rulings: [ANK 20190104] [LSJ 19980212].
#[test]
fn fae_contortion_cancels_a_grapple_as_it_is_played() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Dame Hollerton"), 5, false);
    let z = e.add_minion(SeatId(1), vampire_with("POT", 0), 4, false);
    hand(&mut e, 0, &["Fae Contortion"]);
    hand(&mut e, 1, &["Immortal Grapple", "Immortal Grapple"]);
    let grapple = play(0, 0, z);
    pick(&mut e, bleed(a));
    pick(&mut e, block(z));
    pick_when(&mut e, grapple);
    let req = expect(&mut e, SeatId(0), WindowLabel::CardAsPlayed);
    assert!(req.options.contains(&play(0, 0, a)));
    pick(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, grapple);
    assert_eq!(e.state.seats[1].ash.len(), 1, "the grapple, still played");
}

// Fae Contortion: a maneuver at [obt] and [myt], one Discipline as played
// (LSJ 20011204-3); not a cancel of a non-grapple card.
// Rulings: [LSJ 20011204-3] [PIB 20130704].
#[test]
fn fae_contortion_maneuvers_and_cancels_only_grapples() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Dame Hollerton"), 5, false);
    let z = e.add_minion(SeatId(1), vampire_with("pot", 0), 4, false);
    hand(&mut e, 0, &["Fae Contortion"]);
    hand(&mut e, 1, &["Torn Signpost"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(z));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&play(0, 1, a)), "[obt] maneuver");
    assert!(req.options.contains(&play(0, 2, a)), "[myt] maneuver");
}

// Gremlins [myt]: +1 stealth Ⓓ burn an equipment, directed at its
// controller (LSJ 20090324); a melee weapon not a target; [dom] and [MYT]
// a bleed at +1.
// Rulings: [LSJ 20011204-3] [PIB 20130704] [LSJ 20090324].
#[test]
fn gremlins_burns_a_non_melee_equipment_or_bleeds() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Dame Hollerton"), 5, false);
    let t = e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    let vest = in_play(&mut e, 1, "Kevlar Vest", Some(t));
    let club = in_play(&mut e, 1, "Sport Bike", Some(t));
    let axe = in_play(&mut e, 1, ".44 Magnum", Some(t));
    let _ = (club, axe);
    hand(&mut e, 0, &["Gremlins"]);
    let at = |c: CardId| Choice::Play {
        card: 0,
        mode: 1,
        actor: Some(a),
        target: Target::Card(c),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&at(vest)));
    pick(&mut e, at(vest));
    request(&mut e);
    assert_eq!(
        e.action().expect("action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert!(!e.state.cards[vest.index()].in_play, "burned");
    // A melee weapon is not offered.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Dame Hollerton"), 5, false);
    let t = e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    let blade = in_play(&mut e, 1, "Kali's Fang", Some(t));
    hand(&mut e, 0, &["Gremlins"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::Play {
        card: 0,
        mode: 1,
        actor: Some(a),
        target: Target::Card(blade),
    }));
    // [dom]: a bleed at +1.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Dame Hollerton"), 5, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Gremlins"]);
    pick(&mut e, play(0, 0, a));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 8);
}

// ---- Masters and cards in play ---------------------------------------------------

// Chanjelin Ward: "Other Methuselahs' actions targeting this vampire cost
// an additional blood": a diablerie of the torpid vampire pays it at
// resolution (rules-qa 3.6).
// Rulings: [LSJ 20080809-2] [RTR 20081119].
#[test]
fn chanjelin_ward_makes_actions_at_its_vampire_cost_a_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Pherydima"), 3, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), find_card("Dame Hollerton"), 0, true);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Chanjelin Ward", Some(t));
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: ActionKind::Diablerie,
            target: Some(t),
        },
    );
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_play, "diablerized");
    assert_eq!(blood(&e, a), 2, "the Ward's blood paid");
}

// Chanjelin Ward: "This vampire can burn this card to cause an action
// targeting him or her to fail" — the action reached resolution (NRA),
// was not blocked (ANK 20211015, LSJ 20070219); in torpor too (D25).
// Rulings: [ANK 20210309-3] [ANK 20211015] [RTR 20180719].
#[test]
fn chanjelin_ward_burns_to_fail_an_action_at_its_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Pherydima"), 3, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), find_card("Dame Hollerton"), 0, true);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    let ward = in_play(&mut e, 1, "Chanjelin Ward", Some(t));
    let diablerie = Choice::TakeAction {
        minion: a,
        action: ActionKind::Diablerie,
        target: Some(t),
    };
    pick(&mut e, diablerie);
    let u = first_use(&mut e, AbilitySource::Card(ward), 0);
    pick(&mut e, u);
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert!(e.state.minions[t.index()].in_play, "not diablerized");
    assert!(!e.state.cards[ward.index()].in_play, "burned");
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
}

// Chanjelin Ward: not while a block is attempted (ANK 20260316, LSJ
// 20041022).
// Rulings: [ANK 20260316] [LSJ 20041022].
#[test]
fn chanjelin_ward_is_not_burned_during_a_block_attempt() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Pherydima"), 3, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), find_card("Dame Hollerton"), 0, true);
    let z = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    let ward = in_play(&mut e, 1, "Chanjelin Ward", Some(t));
    hand(&mut e, 1, &["Aura Absorption"]);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: ActionKind::Diablerie,
            target: Some(t),
        },
    );
    pick_when(&mut e, block(z));
    let req = request(&mut e);
    assert!(uses(&req, ward.0).is_empty(), "{:?}", req.options);
}

// Chanjelin Ward: an ally pays no blood (rules-qa 73.5): its action at the
// vampire is not offered (D3).
#[test]
fn chanjelin_ward_bars_an_allys_action_at_its_vampire() {
    for ward in [false, true] {
        let mut e = game(2);
        let m = e.add_ally(SeatId(0), find_library("Mylan Horseed"));
        e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
        let p = e.add_minion(SeatId(1), find_card("Pherydima"), 8, false);
        e.state.minions[p.index()].locked = true;
        e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
        if ward {
            in_play(&mut e, 1, "Chanjelin Ward", Some(p));
        }
        let card = CardId(
            e.state
                .cards
                .iter()
                .position(|c| c.body == Some(m))
                .expect("Mylan's card") as u32,
        );
        let req = request(&mut e);
        let offered = req.options.iter().any(|o| {
            matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), target: Target::Minion(t), .. }
                if *c == card && *t == p)
        });
        assert_eq!(offered, !ward, "ward: {ward}");
    }
}

/// The first offered use of `ability` of `source`.
fn first_use(e: &mut Engine, source: AbilitySource, ability: u8) -> Choice {
    for _ in 0..60 {
        let req = request(e);
        if let Some(o) = req.options.iter().find(|o| {
            matches!(o, Choice::UseAbility { source: s, ability: a, .. } if *s == source && *a == ability)
        }) {
            return *o;
        }
        e.choose(fallback(&req));
    }
    panic!("{source:?} ability {ability} never offered");
}

// Humanitas: on a ready vampire you control; rescuing, +1 stealth and 1
// less blood whoever pays (PIB 20110918); no diablerie.
// Rulings: [PIB 20110918] [LSJ 20020304-2].
#[test]
fn humanitas_helps_rescues_and_bars_diablerie() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Pherydima"), 6, false);
    let t = e.add_minion(SeatId(0), of_clan("Brujah", 0), 1, true);
    let x = e.add_minion(SeatId(1), of_clan("Ventrue", 0), 0, true);
    e.add_minion(SeatId(1), of_clan("Ventrue", 1), 4, false);
    in_play(&mut e, 0, "Humanitas", Some(a));
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::TakeAction {
        minion: a,
        action: ActionKind::Diablerie,
        target: Some(x),
    }));
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: ActionKind::Rescue,
            target: Some(t),
        },
    );
    request(&mut e);
    let st = &e.state;
    let act = e.action().expect("rescue");
    assert_eq!(st.bearer_mod(a, Stat::Stealth, act.kind), 1);
    let req = expect(&mut e, SeatId(0), WindowLabel::RescueCost);
    // 1 blood to pay: all of it from one or the other.
    assert!(req
        .options
        .iter()
        .all(|o| matches!(o, Choice::PayRescue { from_acting } if *from_acting <= 1)));
}

// Catacombs: a locquipment; "the vampire with this location gets +1
// stealth and burns 1 less blood" rescuing.
// Rulings: [PIB 20110918] [LSJ 20020304-2].
#[test]
fn catacombs_helps_its_vampire_rescue() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Pherydima"), 6, false);
    let t = e.add_minion(SeatId(0), of_clan("Brujah", 0), 1, true);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Catacombs"]);
    pick(&mut e, play(0, 0, a));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.host == Some(a) && c.card == find_library("Catacombs")));
    e.state.minions[a.index()].locked = false;
    e.state.minions[a.index()].acted.clear();
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: ActionKind::Rescue,
            target: Some(t),
        },
    );
    let req = expect(&mut e, SeatId(0), WindowLabel::RescueCost);
    assert!(req
        .options
        .iter()
        .all(|o| matches!(o, Choice::PayRescue { from_acting } if *from_acting <= 1)));
}

fn slot(card: u16) -> zvtes::state::UncontrolledSlot {
    zvtes::state::UncontrolledSlot {
        card,
        copy: Default::default(),
        counters: 0,
        banished: None,
    }
}

// Great Symposium: one Kiasyd searched to the uncontrolled region (ANK
// 20210304), then 3 blood from the bank among the Kiasyd there; the search
// may fail, the blood still distributed (LSJ 20100306); once a game.
// Rulings: [ANK 20210304] [LSJ 20100306].
#[test]
fn great_symposium_searches_a_kiasyd_and_distributes_three_blood() {
    for find in [true, false] {
        let mut e = game(2);
        // A master requiring Kiasyd: a ready Kiasyd controlled.
        e.add_minion(SeatId(0), find_card("Pherydima"), 4, false);
        e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
        e.state.seats[0].crypt.push(find_card("Dame Hollerton"));
        e.state.seats[0]
            .uncontrolled
            .push(slot(find_card("Isanwayen")));
        e.at_master_phase();
        hand(&mut e, 0, &["Great Symposium", "Great Symposium"]);
        pick(&mut e, master_play(0));
        let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        if find {
            pick(&mut e, Choice::AshVampire(find_card("Dame Hollerton")));
        } else {
            e.choose(index_of(&req, Choice::Pass));
        }
        for _ in 0..3 {
            let req = request(&mut e);
            if let Some(o) = req.options.iter().find(|o| matches!(o, Choice::Slot(_))) {
                e.choose(index_of(&req, *o));
            }
        }
        let st = &e.state.seats[0];
        assert_eq!(st.uncontrolled.len(), if find { 2 } else { 1 });
        let total: u8 = st.uncontrolled.iter().map(|u| u.counters).sum();
        assert_eq!(total, 3, "find: {find}");
        // Once a game: the second copy not offered.
        e.state.seats[0].mpa = 2;
        let req = request(&mut e);
        assert!(!req
            .options
            .iter()
            .any(|o| matches!(o, Choice::Play { actor: None, .. })));
    }
}

// Harvest Rites: a Sabbat vampire's +1 stealth action, put on it; "once
// each turn, when this vampire successfully hunts, they gain 1 additional
// blood".
// Rulings: [RTR 20030519] [RTR 20180511].
#[test]
fn harvest_rites_adds_a_blood_to_one_hunt_each_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Dame Hollerton"), 2, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Harvest Rites"]);
    pick(&mut e, play(0, 0, a));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    let rites = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Harvest Rites"))
        .expect("in play");
    assert_eq!(e.state.cards[rites].host, Some(a));
    assert_eq!(e.state.hunt_bonus(a), 1);
    pick(&mut e, Choice::Pass);
    to_minion_phase(&mut e, SeatId(0));
    let before = blood(&e, a);
    pick(&mut e, action(a, ActionKind::Hunt));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), before + 2, "1 + 1 additional");
    assert_eq!(e.state.hunt_bonus(a), 0, "spent this turn");
    // The next turn, a hunt at capacity: successful, its blood draining
    // off, the once spent (owner, rules-qa 89.1).
    pick(&mut e, Choice::Pass);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.hunt_bonus(a), 1, "the next turn");
    let cap = e.state.capacity_of(a).expect("capacity");
    e.state.minions[a.index()].blood = cap;
    pick(&mut e, action(a, ActionKind::Hunt));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), cap);
    assert_eq!(e.state.hunt_bonus(a), 0, "spent at capacity");
}

// Guarded Rubrics: requires a ready Black Hand vampire; its bleed at +2 as
// a Ⓓ action costing 1 blood; while the bearer is ready, Gehenna cards
// cost every Methuselah 1 more pool.
#[test]
fn guarded_rubrics_bleeds_and_taxes_gehenna_cards() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Banjoko"), 5, false);
    let x = e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Guarded Rubrics"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, b)));
    assert!(!req.options.contains(&play(0, 0, x)), "Black Hand only");
    let gehenna = zvtes::library::LIBRARY
        .iter()
        .position(|c| c.labels.contains(&zvtes::library::Label::Gehenna))
        .expect("a Gehenna card") as u16;
    let base = zvtes::cards::pool_cost(&e.state, SeatId(1), gehenna, Target::None);
    let rub = in_play(&mut e, 0, "Guarded Rubrics", Some(b));
    let st = &e.state;
    assert_eq!(
        zvtes::cards::pool_cost(st, SeatId(1), gehenna, Target::None),
        base + 1
    );
    e.state.minions[b.index()].in_torpor = true;
    let st = &e.state;
    assert_eq!(
        zvtes::cards::pool_cost(st, SeatId(1), gehenna, Target::None),
        base,
        "bearer not ready"
    );
    e.state.minions[b.index()].in_torpor = false;
    pick(&mut e, use_on(rub, 0, Some(b), Target::None));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 4);
    assert_eq!(pool(&e)[1], 10 - 3, "bleed 1 + 2");
}

// Watchtower: Four Ride Forth: requires a ready Seraph; during the
// discard phase, lock it to unlock any ready Black Hand vampire; burned
// as another watchtower enters play.
#[test]
fn watchtower_unlocks_a_black_hand_vampire_and_burns_for_another() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Banjoko"), 5, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    e.at_master_phase();
    hand(
        &mut e,
        0,
        &["Watchtower: Four Ride Forth", "Watchtower: Four Ride Forth"],
    );
    e.state.seats[0].mpa = 2;
    pick(&mut e, master_play(0));
    request(&mut e);
    let first = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Watchtower: Four Ride Forth"))
        .expect("in play");
    pick(&mut e, master_play(0));
    request(&mut e);
    assert!(!e.state.cards[first].in_play, "burned by the second");
    let second = CardId(
        e.state
            .cards
            .iter()
            .rposition(|c| c.in_play && c.card == find_library("Watchtower: Four Ride Forth"))
            .expect("the second") as u32,
    );
    to_minion_phase(&mut e, SeatId(0));
    pick(&mut e, action(b, ActionKind::Hunt));
    let req = pass_to(&mut e, 0, WindowLabel::DiscardPhase);
    let u = uses(&req, second.0);
    assert_eq!(u.len(), 1, "{:?}", req.options);
    pick(&mut e, u[0]);
    request(&mut e);
    assert!(!e.state.minions[b.index()].locked);
    // No ready Seraph: not playable.
    let mut e = game(2);
    e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    e.at_master_phase();
    hand(&mut e, 0, &["Watchtower: Four Ride Forth"]);
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
}

// Ravager: on any Kiasyd (rules-qa 33.3); "non-changelings cannot block
// this Kiasyd's undirected hunt actions" — its bleed still blockable.
#[test]
fn ravager_bars_non_changelings_from_blocking_its_kiasyds_hunt() {
    for hunt in [true, false] {
        let mut e = game(2);
        let k = e.add_minion(SeatId(0), find_card("Dame Hollerton"), 2, false);
        e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
        in_play(&mut e, 1, "Ravager", Some(k));
        pick(
            &mut e,
            action(
                k,
                if hunt {
                    ActionKind::Hunt
                } else {
                    ActionKind::Bleed
                },
            ),
        );
        let offered = offered_before(&mut e, block(z), SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(offered, !hunt, "hunt: {hunt}");
    }
}

// Song of Pan: the acting Kiasyd unlocked, the card in play; Kiasyd and
// changeling allies +1 bleed; in its controller's discard phase it burns
// and each of their Kiasyd that did not bleed successfully takes 1
// unpreventable damage.
#[test]
fn song_of_pan_raises_kiasyd_bleeds_and_hurts_those_that_did_not_bleed() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Dame Hollerton"), 3, false);
    let i = e.add_minion(SeatId(0), find_card("Isanwayen"), 4, false);
    let m = e.add_ally(SeatId(0), find_library("Mylan Horseed"));
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Song of Pan"]);
    pick(&mut e, play(0, 0, d));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert!(!e.state.minions[d.index()].locked, "unlocked");
    let song = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Song of Pan"))
        .expect("in play");
    let bleed_of = |e: &Engine, m| e.state.bearer_mod(m, Stat::Bleed, ActionKind::Bleed);
    assert_eq!(bleed_of(&e, i), 1);
    assert_eq!(bleed_of(&e, m), 1, "a changeling ally");
    pick(&mut e, bleed(i));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 2);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[song].in_play, "burned");
    assert_eq!(blood(&e, d), 3 - 1, "did not bleed");
    assert_eq!(blood(&e, i), 4, "bled");
    assert!(
        !e.state.minions[m.index()].in_play,
        "the changeling's 1 life"
    );
}

// Fee Stakes: Corte, Los Angeles: an Anarch of capacity 5 or more takes the
// Baron title (as Perth).
#[test]
fn fee_stakes_corte_and_los_angeles_make_a_baron() {
    for (card, city) in [
        ("Fee Stake: Corte", "Corte"),
        ("Fee Stake: Los Angeles", "Los Angeles"),
    ] {
        let mut e = game(2);
        let a = e.add_minion(
            SeatId(0),
            CRYPT
                .iter()
                .enumerate()
                .position(|(i, c)| {
                    c.sect == Some(zvtes::crypt::Sect::Anarch)
                        && c.capacity >= 5
                        && c.title == zvtes::crypt::Title::None
                        && zvtes::cards::vampire(i as u16).is_none()
                })
                .expect("an Anarch") as u16,
            5,
            false,
        );
        e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
        hand(&mut e, 0, &[card]);
        pick(&mut e, play(0, 0, a));
        pass_to(&mut e, 0, WindowLabel::MinionPhase);
        assert_eq!(e.state.title_of(a), zvtes::crypt::Title::Baron(city));
    }
}

// Winterlich: "While he is ready, changelings cost you 1 fewer pool" —
// Mylan Horseed, himself recruiting or another of his Methuselah's; the
// first clause inert (no changeling in the pool has requirements).
#[test]
fn winterlich_makes_changelings_cost_one_pool_less() {
    let mut e = game(2);
    let w = e.add_minion(SeatId(0), find_card("Winterlich"), 4, false);
    let x = e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    let ctx = PrintedCtx {
        card: Some((find_library("Mylan Horseed"), 0)),
        ..PrintedCtx::default()
    };
    let st = &e.state;
    assert_eq!(st.printed_mod(w, Stat::PoolCostOff, ctx), 1);
    assert_eq!(st.printed_mod(x, Stat::PoolCostOff, ctx), 1);
    let other = PrintedCtx {
        card: Some((find_library("Kevlar Vest"), 0)),
        ..PrintedCtx::default()
    };
    assert_eq!(st.printed_mod(x, Stat::PoolCostOff, other), 0);
}

// Black Forest Base: "Changeling allies can burn this location as a +1
// stealth Ⓓ action" — Mylan Horseed; a vampire not.
#[test]
fn black_forest_base_is_burned_by_a_changeling_ally() {
    let mut e = game(2);
    let m = e.add_ally(SeatId(0), find_library("Mylan Horseed"));
    let v = e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    let base = in_play(&mut e, 1, "Black Forest Base", None);
    let req = request(&mut e);
    let by = |actor| {
        req.options.iter().any(|o| {
            matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: 1, actor: a, .. }
                if *c == base && *a == Some(actor))
        })
    };
    assert!(by(m), "{:?}", req.options);
    assert!(!by(v));
    let u = *req
        .options
        .iter()
        .find(|o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: 1, .. } if *c == base))
        .expect("the burn");
    e.choose(index_of(&req, u));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert!(!e.state.cards[base.index()].in_play, "burned");
}

// ---- Allies ----------------------------------------------------------------------

/// The Nocturn just recruited for `seat`, if any.
fn nocturn_of(e: &Engine, seat: u8) -> Vec<MinionId> {
    e.state
        .minions
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            m.in_play
                && m.owner == SeatId(seat)
                && m.kind.ally_card() == Some(find_library("Nocturn"))
        })
        .map(|(i, _)| MinionId(i as u32))
        .collect()
}

// Nocturn [obt]: acts the turn it is recruited, its recruiter unlocked on
// its first successful recruit this turn; plays non-ally cards requiring
// basic [obt] as an independent vampire (LSJ 20020503) — not another
// Nocturn; burned in its controller's unlock phase.
#[test]
fn nocturn_acts_at_once_plays_obtenebration_and_burns_at_unlock() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Nocturn"]);
    pick(&mut e, play(0, 0, a));
    let req = pass_to(&mut e, 0, WindowLabel::MinionPhase);
    let n = nocturn_of(&e, 0)[0];
    assert!(!e.state.minions[a.index()].locked, "the recruiter unlocked");
    assert!(req.options.contains(&bleed(n)), "acts at once");
    let st = &e.state;
    assert!(zvtes::cards::plays_card_as_vampire(st, n, find_library("Shadow Play"), 0).is_some());
    assert!(zvtes::cards::plays_card_as_vampire(st, n, find_library("Nocturn"), 0).is_none());
    pick(&mut e, Choice::Pass);
    to_minion_phase(&mut e, SeatId(0));
    assert!(!e.state.minions[n.index()].in_play, "burned at unlock");
}

// Nocturn [OBT]: "this vampire can perform this action again this turn,
// with the cost increased by 1 blood (cumulative)": a second recruit
// costs 2, a third (after a second at [OBT]) 3; after one at [obt] no
// more.
#[test]
fn nocturn_superior_lets_its_recruiter_recruit_again_at_a_higher_cost() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Dame Hollerton"), 5, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Nocturn", "Nocturn", "Nocturn"]);
    pick(&mut e, play(0, 1, a));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4);
    // Unlocked (its first recruit) and allowed again: 2 blood.
    pick(&mut e, play(0, 1, a));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 2, "1 + 1 more");
    assert_eq!(nocturn_of(&e, 0).len(), 2);
    // A third (the second at [OBT]): 3 blood, more than it has.
    e.state.minions[a.index()].locked = false;
    e.state.minions[a.index()].blood = 3;
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, a)) || req.options.contains(&play(0, 1, a)));
    pick(&mut e, play(0, 0, a));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 0, "1 + 2 more");
    // After one at [obt]: no fourth.
    e.state.minions[a.index()].locked = false;
    e.state.minions[a.index()].blood = 5;
    hand(&mut e, 0, &["Nocturn"]);
    never_offered_until(&mut e, SeatId(0), WindowLabel::DiscardPhase, play(0, 0, a));
}

// Khobar Towers on Nocturn: the unlock-phase effects in the order its
// Methuselah wishes — Khobar used on Nocturn before it burns (ANK
// 20170625), its cost (1) gained.
// Rulings: [ANK 20170625].
#[test]
fn khobar_towers_takes_nocturn_before_it_burns() {
    let mut e = game(2);
    e.add_minion(SeatId(0), vampire_with("obt", 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    let n = e.add_ally(SeatId(0), find_library("Nocturn"));
    let nc = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.body == Some(n))
            .expect("Nocturn's card") as u32,
    );
    let k = in_play(&mut e, 0, "Khobar Towers, Al-Khubar", None);
    let _ = nc;
    pick(&mut e, Choice::Pass);
    let req = pass_to(&mut e, 0, WindowLabel::UnlockAbilities);
    let u = req
        .options
        .iter()
        .copied()
        .find(|o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), target: Target::Minion(t), .. } if *c == k && *t == n))
        .expect("Khobar on Nocturn");
    let before = pool(&e)[0];
    e.choose(index_of(&req, u));
    request(&mut e);
    assert_eq!(pool(&e)[0], before + 1);
    assert!(!e.state.minions[n.index()].in_play);
}

// ---- Goblinism ---------------------------------------------------------------------

// Goblinism [pot]: Ⓓ burn a location, directed at its controller (LSJ
// 20090324).
// Rulings: [LSJ 20090324].
#[test]
fn goblinism_burns_a_location() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    let loc = in_play(&mut e, 1, "Therbold Realty", None);
    hand(&mut e, 0, &["Goblinism"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::Card(loc),
        },
    );
    request(&mut e);
    assert_eq!(
        e.action().expect("action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert!(!e.state.cards[loc.index()].in_play);
}

// Goblinism [myt]: a location (a locquipment too) searched to hand, shown;
// [MYT]: a haven found may go on the vampire instead, its cost paid (one
// haven a vampire).
// Rulings: [LSJ 20011204-3] [PIB 20130704].
#[test]
fn goblinism_finds_a_location_and_may_put_a_haven_on() {
    // [myt]: to hand.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Dame Hollerton"), 5, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Goblinism"]);
    // Under the draw-up's cards (a hand of 7).
    e.state.seats[0].library.push(find_library("Catacombs"));
    for _ in 0..10 {
        e.state.seats[0].library.push(find_library("Dodge"));
    }
    pick(&mut e, play(0, 1, a));
    let req = offering(&mut e, Choice::FindCard(find_library("Catacombs")));
    assert!(!req
        .options
        .contains(&Choice::FindCard(find_library("Dodge"))));
    e.choose(index_of(&req, Choice::FindCard(find_library("Catacombs"))));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert!(e.state.seats[0]
        .hand
        .iter()
        .any(|&c| c == find_library("Catacombs")));
    // [MYT]: Secure Haven put on, 1 pool paid; or kept.
    for put in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Pherydima"), 6, false);
        e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Goblinism"]);
        let haven = find_library("Secure Haven");
        e.state.seats[0].library.push(haven);
        for _ in 0..10 {
            e.state.seats[0].library.push(find_library("Dodge"));
        }
        pick(&mut e, play(0, 2, a));
        pick_when(&mut e, Choice::FindCard(haven));
        let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        assert_eq!(req.options, vec![Choice::Pass, Choice::FindCard(haven)]);
        e.choose(index_of(
            &req,
            if put {
                Choice::FindCard(haven)
            } else {
                Choice::Pass
            },
        ));
        pass_to(&mut e, 0, WindowLabel::MinionPhase);
        let on = e
            .state
            .cards
            .iter()
            .any(|c| c.in_play && c.card == haven && c.host == Some(a) && !c.effect_only);
        assert_eq!(on, put);
        assert_eq!(e.state.seats[0].hand.contains(&haven), !put);
        assert_eq!(pool(&e)[0], if put { 9 } else { 10 });
        if put {
            assert_eq!(e.state.havens_on(a), 1, "its layer");
        }
    }
}

// Chanjelin Ward: on a vampire you control, only one a vampire (a mask
// as played).
#[test]
fn chanjelin_ward_goes_on_your_vampire_one_each() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Dame Hollerton"), 5, false);
    let w = e.add_minion(SeatId(0), find_card("Pherydima"), 5, false);
    let x = e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Chanjelin Ward", Some(w));
    e.at_master_phase();
    hand(&mut e, 0, &["Chanjelin Ward"]);
    let on = |t| Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(t),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&on(k)));
    assert!(!req.options.contains(&on(w)), "one a vampire");
    assert!(!req.options.contains(&on(x)), "yours only");
}

// "Successfully hunts" at capacity: the hunt gets its blood, which drains
// off, and is successful (owner, rules-qa 89.26): Eddie Gaines does not
// unlock as normal after one.
#[test]
fn a_hunt_at_capacity_is_a_successful_hunt() {
    let mut e = game(2);
    let eddie = e.add_minion(SeatId(0), find_card("Eddie Gaines"), 0, false);
    let cap = e.state.capacity_of(eddie).expect("capacity");
    e.state.minions[eddie.index()].blood = cap;
    e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    pick(&mut e, action(eddie, ActionKind::Hunt));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert_eq!(blood(&e, eddie), cap);
    assert!(
        e.state.minions[eddie.index()].skip_unlock,
        "a successful hunt"
    );
}

// "A minion can have only one haven" (Secure Haven, Zaire River Ferry):
// either card printing "Haven.", Goblinism [MYT]'s put-on too (rules-qa
// 89.10).
#[test]
fn one_haven_a_minion_whichever_it_is() {
    // Secure Haven not put on the Ferry's bearer.
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Banjoko"), 5, false);
    let o = e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Zaire River Ferry", Some(l));
    e.at_master_phase();
    hand(&mut e, 0, &["Secure Haven"]);
    let on = |t| Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(t),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&on(o)));
    assert!(!req.options.contains(&on(l)), "the Ferry's bearer");
    // The Ferry not equipped by a Secure Haven's vampire.
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Banjoko"), 5, false);
    // A Lasombra card: two Lasombra.
    let o = e.add_minion(SeatId(0), of_clan("Lasombra", 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Secure Haven", Some(h));
    hand(&mut e, 0, &["Zaire River Ferry"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, o)));
    assert!(!req.options.contains(&play(0, 0, h)), "a Secure Haven's");
    // Goblinism [MYT]: the haven found stays in hand, no question asked.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Pherydima"), 6, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Secure Haven", Some(a));
    hand(&mut e, 0, &["Goblinism"]);
    let ferry = find_library("Zaire River Ferry");
    e.state.seats[0].library.push(ferry);
    for _ in 0..10 {
        e.state.seats[0].library.push(find_library("Dodge"));
    }
    pick(&mut e, play(0, 2, a));
    pick_when(&mut e, Choice::FindCard(ferry));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::FindCard(ferry),
    );
    assert!(e.state.seats[0].hand.contains(&ferry));
}

// Harvest Rites on a hunt stealing its blood (Week of Nightmares' Ravnos
// hunt): the bonus from the target (RTR 20030519), the once spent.
#[test]
fn harvest_rites_is_spent_by_a_hunt_stealing_its_blood() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), of_clan("Ravnos", 0), 1, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), of_clan("Ravnos", 1), 4, false);
    let week = in_play(&mut e, 0, "Week of Nightmares", None);
    in_play(&mut e, 0, "Harvest Rites", Some(r));
    assert_eq!(e.state.hunt_bonus(r), 1);
    pick(&mut e, use_on(week, 0, Some(r), Target::Minion(t)));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert_eq!(blood(&e, r), 1 + 2, "1 + 1 from the target");
    assert_eq!(e.state.hunt_bonus(r), 0, "spent");
}

// An action that is not a hunt never "successfully hunts" (rules-qa 89.26):
// Putrescent Sustenance's blood is no hunt, so Festivo dello Estinto does
// not fill the Sabbat vampire to capacity.
#[test]
fn a_blood_gain_that_is_no_hunt_is_no_successful_hunt() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Abderrahim, Death's Hand"), 1, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Festivo dello Estinto", None);
    e.state.seats[1].ash.push(find_library("Mylan Horseed"));
    hand(&mut e, 0, &["Putrescent Sustenance"]);
    assert!(e.state.capacity_of(a).expect("capacity") > 3);
    pick(&mut e, play(0, 0, a));
    pass_to(&mut e, 0, WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 1 + 2, "the blood gained, no fill");
}
