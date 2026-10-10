//! Legacy cards for TWDA decks (`ADDED_BATCHES`, from the eighth batch on):
//! the staples gating the most archetypes and the cheapest unlocks
//! (slice 16j). Readings in rules-qa Batch 57. Windows with a single option
//! are elided, so the scripts only stop where a real decision exists.

use zvtes::cards::{self, AbilitySource, Effect, Layer, Scope, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{Choice, WindowLabel};
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

/// A plain untitled vampire of `sect`, capacity 4 or more (`nth` one).
fn of_sect(sect: Sect, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.sect == sect
                && c.title == Title::None
                && c.capacity >= 4
                && cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a vampire of the sect")
}

// ---- Retainers ---------------------------------------------------------------

// J. S. Simmons, Esq. and Tasha Morgan: "Unique mortal with 1 life. The
// employer gets +1 bleed." Employed with an employ action, each makes its
// employer's bleed 2.
#[test]
fn j_s_simmons_and_tasha_morgan_give_their_employer_a_bleed() {
    for name in ["J. S. Simmons, Esq.", "Tasha Morgan"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        hand(&mut e, 0, &[name]);
        pick(&mut e, play(0, 0, a));
        to_minion_phase(&mut e, SeatId(0));
        e.state.minions[a.index()].locked = false;
        pick(&mut e, bleed(a));
        to_minion_phase(&mut e, SeatId(0));
        assert_eq!(pool(&e)[1], 10 - 2, "{name}");
    }
}

// ---- Alonso Petrodon -----------------------------------------------------------

// Alonso Petrodon: "Alonso inflicts 1 unpreventable damage on each anarch
// controlled by the target of the bleed after bleeding successfully. +1
// bleed." His bleed of 2 lands; each Anarch of the bled Methuselah takes 1
// — the torpid one too, its 1 blood burned (it stays in torpor) — the other
// vampires none.
#[test]
fn alonso_petrodon_damages_the_bled_methuselahs_anarchs() {
    let mut e = game(2);
    let alonso = e.add_minion(SeatId(0), find_card("Alonso Petrodon"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let anarch = e.add_minion(SeatId(1), of_sect(Sect::Anarch, 0), 3, false);
    let torpid = e.add_minion(SeatId(1), of_sect(Sect::Anarch, 1), 1, true);
    let other = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 0), 3, false);
    // No block: the anarch blocks only if asked; pass every window.
    pick(&mut e, bleed(alonso));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(pool(&e)[1], 10 - 2);
    assert_eq!(blood(&e, anarch), 2);
    assert_eq!(blood(&e, torpid), 0);
    assert_eq!(blood(&e, other), 3);
}

// ---- Maila ----------------------------------------------------------------------

// [ANK 20230122] Maila "Her ability can only be used once during each of
// your turns": she discards a political action card to give the acting
// vampire +1 stealth against a block; on a second action that turn, she
// cannot again.
#[test]
fn maila_gives_stealth_for_a_political_card_once_each_turn() {
    let mut e = game(2);
    let maila = e.add_minion(SeatId(0), find_card("Maila"), 4, false);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("AUS", 0), 4, false);
    hand(
        &mut e,
        0,
        &["Kine Resources Contested", "Kine Resources Contested"],
    );
    let stealth = Choice::UseAbility {
        source: AbilitySource::Vampire(maila),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick(&mut e, bleed(a));
    pick(&mut e, block(z));
    // Its discard elides: one card name to choose.
    pick_when(&mut e, stealth);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(pool(&e)[1], 10 - 1, "the block failed");
    assert_eq!(e.state.seats[0].hand.len(), 1);
    pick(&mut e, bleed(b));
    pick(&mut e, block(z));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, stealth);
    assert_eq!(e.state.seats[0].hand.len(), 1);
}

// ---- Gabriel Tremblay -------------------------------------------------------------

// Gabriel Tremblay: "Once each combat, Gabriel can discard a card requiring
// an Anarch to get +1 strength that combat. -1 stealth." Blocked, he
// discards Lam Into for a hand strike of 2; not twice that combat.
#[test]
fn gabriel_tremblay_discards_an_anarch_card_for_strength() {
    let mut e = game(2);
    let gabriel = e.add_minion(SeatId(0), find_card("Gabriel Tremblay"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    hand(&mut e, 0, &["Lam Into", "Lam Into"]);
    pick(&mut e, bleed(gabriel));
    pick(&mut e, block(z));
    pick_when(&mut e, vampire_ability(gabriel, 0));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(!req.options.contains(&vampire_ability(gabriel, 0)));
    pick(&mut e, Choice::HandStrike);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, z), 4 - 2);
}

// ---- Bum's Rush ------------------------------------------------------------------------

// Bum's Rush, blocked [LSJ 20001120] "Is not replaced if the action is
// blocked; it is replaced when the action ends, after all combats." and
// [LSJ 20101021] "The card is replaced after all "after resolution" cards
// and effects": through the block's combat the hand stays one short, made
// up once the action is over.
#[test]
fn bums_rush_is_replaced_as_its_action_ends() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Bum's Rush"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::Minion(x),
        },
    );
    e.state.seats[0].library = vec![find_library("Bum's Rush"); 10];
    // The blocker's Hidden Strength: a decision in the block's combat.
    hand(&mut e, 1, &["Hidden Strength"]);
    pick(&mut e, block(y));
    until(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    assert_eq!(e.state.seats[0].hand.len(), 0, "not replaced yet");
    assert_eq!(e.state.seats[0].held.action, 1);
    // Replaced as the action ends: drawn up to hand size.
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.seats[0].hand.len(), 7);
}

// Bum's Rush unblocked: the rush's combat, the acting minion with 1
// optional maneuver in it (to long range, the opponent's hand strike
// fails).
#[test]
fn bums_rush_enters_combat_with_a_maneuver() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Bum's Rush"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::Minion(x),
        },
    );
    let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(
        req.options.contains(&Choice::UseManeuver),
        "{:?}",
        req.options
    );
    pick(&mut e, Choice::UseManeuver);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(
        (blood(&e, a), blood(&e, x)),
        (4, 4),
        "no strike at long range"
    );
}

// ---- Mylan Horseed ------------------------------------------------------------------

/// The first capacity-8+ vampire of `sect`.
fn big(sect: Sect) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.sect == sect && c.capacity >= 8)
        .expect("a big vampire") as u16
}

/// 4 seats. Seat 0: Mylan Horseed and a spare; seat 2 (across the table)
/// a blocker with Mr. Winthrop, +1 intercept against Mylan's +1 stealth.
/// Mylan's unlock is his second ability (the first is the dodge).
fn mylan_table() -> (Engine, MinionId, CardId, MinionId) {
    let mut e = game(4);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(2), vampire_with("AUS", 0), 4, false);
    in_play(&mut e, 2, "Mr. Winthrop", Some(z));
    let mylan = e.add_ally(SeatId(0), find_library("Mylan Horseed"));
    let id = e.state.minions[mylan.index()].ally_id().expect("ally");
    (e, mylan, id, z)
}

// [PIB 20121028] Mylan Horseed "His action to unlock is undirected if the
// target is a vampire controlled by it's controller, directed otherwise":
// unlocking a locked capacity-8+ Sabbat vampire of his own, the
// Methuselah across the table cannot block; an Independent one of theirs,
// they can. A Camarilla vampire is no target.
#[test]
fn mylan_horseed_unlocks_a_big_non_camarilla_vampire() {
    let (mut e, mylan, id, z) = mylan_table();
    let own = e.add_minion(SeatId(0), find_card("Damian"), 4, false);
    let cam = e.add_minion(SeatId(2), big(Sect::Camarilla), 4, false);
    e.state.minions[own.index()].locked = true;
    e.state.minions[cam.index()].locked = true;
    let unlock = |t: MinionId| use_at(id, 1, Some(mylan), Target::Minion(t));
    let req = request(&mut e);
    assert!(req.options.contains(&unlock(own)), "{:?}", req.options);
    assert!(!req.options.contains(&unlock(cam)));
    pick(&mut e, unlock(own));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(z));
    assert!(!e.state.minions[own.index()].locked);

    let (mut e, mylan, id, z) = mylan_table();
    let theirs = e.add_minion(SeatId(2), find_card("Seraphina"), 4, false);
    e.state.minions[theirs.index()].locked = true;
    pick(&mut e, use_at(id, 1, Some(mylan), Target::Minion(theirs)));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        e.state.minions[theirs.index()].locked,
        "blocked: still locked"
    );
}

// Mylan Horseed "cannot block vampires": never offered against a vampire's
// bleed, offered against an ally's.
#[test]
fn mylan_horseed_cannot_block_vampires() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let mylan = e.add_ally(SeatId(1), find_library("Mylan Horseed"));
    pick(&mut e, bleed(a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(mylan));
}

// ---- Lam Into -----------------------------------------------------------------------

/// An Anarch with no printed-text definition having every Discipline of
/// `with` (any level) and none of `without`.
fn anarch(with: &[&str], without: &[&str]) -> u16 {
    let has = |c: &zvtes::crypt::CryptCard, d: &str| {
        c.disciplines.iter().any(|x| x.eq_ignore_ascii_case(d))
    };
    CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.sect == Sect::Anarch
                && c.capacity >= 4
                && cards::vampire(i as u16).is_none()
                && with.iter().all(|d| has(c, d))
                && !without.iter().any(|d| has(c, d))
        })
        .expect("an Anarch") as u16
}

/// A weapon already on `host` (no play; weapons carry no layers).
fn equip(e: &mut Engine, name: &str, host: MinionId) -> CardId {
    let owner = e.state.minions[host.index()].owner;
    e.state
        .cards
        .push(CardInPlay::new(find_library(name), owner, Some(host), 0));
    CardId(e.state.cards.len() as u32 - 1)
}

fn lam(mode: u8, a: MinionId, weapon: Option<CardId>) -> Choice {
    Choice::Play {
        card: 0,
        mode,
        actor: Some(a),
        target: weapon.map_or(Target::None, Target::Card),
    }
}

/// Seat 0: the Anarch `a` (6 blood) with Lam Into, a spare; seat 1: the
/// blocker X (6 blood), with Hidden Strength for a decision in the combat.
fn lam_table(a_card: u16) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), a_card, 6, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let x = e.add_minion(SeatId(1), vampire_with("FOR", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Lam Into"]);
    hand(&mut e, 1, &["Hidden Strength"]);
    (e, a, x)
}

// Lam Into, multi-Discipline [LSJ 20011204-3] [PIB 20130704] "When played, a
// multi-Disciplines card counts as requiring the Discipline(s) being used":
// a Potence Anarch without Fortitude or Vicissitude has the [pot] strikes
// only — the hand strike, or with a melee weapon on him, its strike; the
// [pot] hand strike lands at +2 (strength 1 + 2). Not replaced until after
// combat: held through it.
#[test]
fn lam_into_strikes_by_the_discipline_played() {
    let (mut e, a, x) = lam_table(anarch(&["pot"], &["for", "vic"]));
    let femur = equip(&mut e, "Femur of Toomler", a);
    e.state.seats[0].library = vec![find_library("Lam Into"); 10];
    pick(&mut e, bleed(a));
    pick(&mut e, block(x));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(req.options.contains(&lam(2, a, None)), "{:?}", req.options);
    assert!(req.options.contains(&lam(3, a, Some(femur))));
    for m in [0, 1, 4, 5] {
        assert!(
            !req.options.iter().any(|o| matches!(o,
            Choice::Play { card: 0, mode, .. } if *mode == m)),
            "mode {m}"
        );
    }
    pick(&mut e, lam(2, a, None));
    until(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    assert_eq!(e.state.seats[0].held.combat, 1, "held until after combat");
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 6 - 3);
    assert_eq!(e.state.seats[0].held.combat, 0);
}

// Lam Into [for] at the maneuver [LSJ 20021028] "The optional maneuver
// cannot be used if the strike cannot be used": played as the range is
// determined it maneuvers (to long) and commits the round's strike; the
// blocker's .44 Magnum maneuvers back to close, and the committed hand
// strike lands at +1 (X's own strike is the gun's). With strikes barred in the first round (One With the
// Land [PRO]), it is not offered at the maneuver.
#[test]
fn lam_into_fortitude_commits_its_strike_at_the_maneuver() {
    let (mut e, a, x) = lam_table(anarch(&["for"], &["pot", "vic"]));
    let gun = equip(&mut e, ".44 Magnum", x);
    pick(&mut e, bleed(a));
    pick(&mut e, block(x));
    pick_when(&mut e, lam(0, a, None));
    pick_when(&mut e, use_at(gun, 1, None, Target::None));
    // The committed strike elides: X's prevention is the next decision.
    until(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 6 - 2);

    let mut e = game(3);
    let a = e.add_minion(SeatId(0), anarch(&["for"], &["pot", "vic"]), 6, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    // A Tzimisce (the card's clan) with superior Protean.
    let tzimisce = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.clan == "Tzimisce"
                && c.disciplines.contains(&"PRO")
                && cards::vampire(i as u16).is_none()
        })
        .expect("a Tzimisce") as u16;
    let z = e.add_minion(SeatId(1), tzimisce, 6, false);
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 0, &["Lam Into"]);
    hand(&mut e, 1, &["One With the Land"]);
    pick(&mut e, bleed(a));
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 1,
            actor: Some(z),
            target: Target::None,
        },
    );
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, lam(0, a, None));
}

// Lam Into's melee weapon strike [RTR 19970630] [TOM 19960225] "Additional
// damage inherits all of the properties of the base damage": the [pot]
// strike with Righteous Blade is the Blade's — strength+1 at +2 — and its
// continue press comes with it.
#[test]
fn lam_into_uses_a_melee_weapon_strike_with_its_riders() {
    let (mut e, a, x) = lam_table(anarch(&["pot"], &["for", "vic"]));
    let blade = equip(&mut e, "Righteous Blade", a);
    pick(&mut e, bleed(a));
    pick(&mut e, block(x));
    pick_when(&mut e, lam(3, a, Some(blade)));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert!(req.options.contains(&Choice::UsePress), "{:?}", req.options);
    assert_eq!(blood(&e, x), 6 - 4);
}

// ---- Dragonbound -------------------------------------------------------------------

// Dragonbound: "During each Methuselah's discard phase, that Methuselah
// burns 1 pool for each vampire in torpor they control." Seat 1's event:
// in seat 0's discard phase the burn is due before the impulse passes — 1
// pool for each of seat 0's two vampires in torpor.
#[test]
fn dragonbound_burns_pool_for_each_vampire_in_torpor() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 2, true);
    e.add_minion(SeatId(0), plain("Brujah", 2), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let dragonbound = in_play(&mut e, 1, "Dragonbound", None);
    // A card in hand: the discard phase is a decision.
    hand(&mut e, 0, &["Minion Tap"]);
    let req = to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert!(
        !req.options.contains(&Choice::Pass),
        "due first: {:?}",
        req.options
    );
    pick(
        &mut e,
        Choice::ResolveRider {
            card: dragonbound.0,
        },
    );
    assert_eq!(pool(&e)[0], 10 - 2);
}

// Dragonbound (owner, 2026-10-09: rules-qa 57.5): played during a discard
// phase, it applies in that same phase, ordered among its mandatory
// effects — seat 0 plays it with two vampires in torpor and burns 2 pool
// before passing.
#[test]
fn dragonbound_applies_in_the_discard_phase_it_is_played_in() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 2, true);
    e.add_minion(SeatId(0), plain("Brujah", 2), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Dragonbound", "Minion Tap"]);
    let req = to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    let dragon = req
        .options
        .iter()
        .copied()
        .find(|o| matches!(o, Choice::Play { card: 0, .. }))
        .expect("Dragonbound playable");
    pick(&mut e, dragon);
    // The phase's only option left, the burn resolves at once — in this
    // discard phase, before seat 1's turn.
    request(&mut e);
    assert_eq!(pool(&e)[0], 10 - 2);
    assert_eq!(e.state.active, SeatId(1));
}

// ---- Scourge of the Enochians -------------------------------------------------------

// [ANK 20200508-1] Scourge of the Enochians "If the option to burn a vampire
// is used, the card goes to the predator immediately … If not, the
// predator must take the card when he has the opportunity": seat 0 burns
// a capacity-2 vampire (in torpor, the prey's) and the card is seat 2's
// (seat 0's predator) at once; a capacity-3 one is no target. Not used, it
// goes to the predator as seat 0's discard phase ends.
#[test]
fn scourge_of_the_enochians_burns_a_small_vampire_and_moves_on() {
    let small = find_card("Roy");
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let victim = e.add_minion(SeatId(1), small, 1, true);
    let big = e.add_minion(
        SeatId(1),
        CRYPT.iter().position(|c| c.capacity == 3).unwrap() as u16,
        1,
        false,
    );
    let scourge = in_play(&mut e, 0, "Scourge of the Enochians", None);
    hand(&mut e, 0, &["Minion Tap"]);
    let burn = |m: MinionId| use_at(scourge, 0, None, Target::Minion(m));
    let req = to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert!(req.options.contains(&burn(victim)), "{:?}", req.options);
    assert!(!req.options.contains(&burn(big)));
    pick(&mut e, burn(victim));
    assert!(!e.state.minions[victim.index()].in_play);
    assert_eq!(e.state.card_controller(scourge), SeatId(2));

    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), small, 1, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let scourge = in_play(&mut e, 0, "Scourge of the Enochians", None);
    hand(&mut e, 0, &["Minion Tap"]);
    to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    pick(&mut e, Choice::Pass);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.card_controller(scourge), SeatId(2));
}

// ---- Heidelberg Castle ----------------------------------------------------------------

fn heidelberg(id: CardId, from: MinionId) -> Choice {
    use_at(id, 0, None, Target::Minion(from))
}

// Heidelberg Castle [ANK 20170804] "Can be used to move more blood than the
// target can hold: the excess goes to the blood bank", and [ANK 20221103]
// Flaming Candle "Once in play, it can be moved (eg. Heidelberg Castle)":
// from A to B, 3 blood (B, one short of capacity, keeps 1 of it), then the
// Candle and the retainer; the castle is locked.
#[test]
fn heidelberg_castle_moves_blood_and_cards_between_two_vampires() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 3, false);
    let b_card = plain("Brujah", 1);
    let cap = CRYPT[b_card as usize].capacity;
    let b = e.add_minion(SeatId(0), b_card, cap - 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let candle = equip(&mut e, "Flaming Candle", a);
    let winthrop = in_play(&mut e, 0, "Mr. Winthrop", Some(a));
    let castle = in_play(&mut e, 0, "Heidelberg Castle, Germany", None);
    // B, the only other ready vampire, is chosen at once.
    pick(&mut e, heidelberg(castle, a));
    pick(&mut e, set_x(3));
    assert_eq!((blood(&e, a), blood(&e, b)), (0, cap));
    pick(&mut e, Choice::ChooseCard(candle.0));
    pick(&mut e, Choice::ChooseCard(winthrop.0));
    assert_eq!(e.state.cards[candle.index()].host, Some(b));
    assert_eq!(e.state.cards[winthrop.index()].host, Some(b));
    assert!(e.state.cards[castle.index()].locked);
}

// Heidelberg Castle [LSJ 19980831] "Cannot be used to put a card on a
// vampire who is prohibited to have it": A's Kevlar Vest is not offered to
// B, who has one ("a minion can have only one Kevlar Vest").
#[test]
fn heidelberg_castle_moves_no_card_its_receiver_cannot_have() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 3, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let vest = equip(&mut e, "Kevlar Vest", a);
    let candle = equip(&mut e, "Flaming Candle", a);
    equip(&mut e, "Kevlar Vest", b);
    let castle = in_play(&mut e, 0, "Heidelberg Castle, Germany", None);
    pick(&mut e, heidelberg(castle, a));
    pick(&mut e, set_x(0));
    let req = request(&mut e);
    assert!(
        req.options.contains(&Choice::ChooseCard(candle.0)),
        "{:?}",
        req.options
    );
    assert!(!req.options.contains(&Choice::ChooseCard(vest.0)));
}

// Heidelberg Castle "Cannot be used during an action": [LSJ 20010110] [LSJ
// 20050111] "May not be locked "in response" to an effect", [ANK 20171124]
// "Cannot be used before damage is inflicted after action resolution (eg.
// Daring the Dawn): this is still during the action", [LSJ 20090722-2]
// [LSJ 20030618] [LSJ 20100112] [PIB 20121028] the blood hunt is part of
// the action, [ANK 20170918-2] nor between an action's combats: A bleeds
// with Daring the Dawn [for] — blocks barred, its damage after resolution
// — and the castle is offered at no point of the action, nor to the
// other seat; once the action is over, it is.
#[test]
fn heidelberg_castle_waits_for_the_action_to_end() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("FOR", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 2), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let castle = in_play(&mut e, 0, "Heidelberg Castle, Germany", None);
    hand(&mut e, 0, &["Daring the Dawn"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    let mut seen = Vec::new();
    loop {
        let req = request(&mut e);
        if req.window == WindowLabel::MinionPhase {
            assert!(req.options.contains(&heidelberg(castle, b)));
            break;
        }
        seen.push(req.window);
        assert!(!uses(&req, castle.0).iter().any(|_| true), "{req:?}");
        let i = fallback(&req);
        e.choose(i);
    }
    assert!(blood(&e, a) < 4, "the after-resolution damage landed");
}

// Heidelberg Castle [ANK 20210608] "Can be used between two actions during
// another Methuselah's turn": as seat 1's minion phase ends, seat 0 can lock
// it.
#[test]
fn heidelberg_castle_is_used_on_another_methuselahs_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let castle = in_play(&mut e, 0, "Heidelberg Castle, Germany", None);
    to_minion_phase(&mut e, SeatId(1));
    let req = offering(&mut e, heidelberg(castle, a));
    assert_eq!(req.seat, SeatId(0));
    assert_eq!(e.state.active, SeatId(1));
}

// ---- Anarch Convert -------------------------------------------------------------------

/// Seat 0's next influence phase, Anarch Convert fully funded in its
/// uncontrolled region (slot 0), an empty crypt.
fn convert_table() -> (Engine, MinionId) {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: find_card("Anarch Convert"),
            counters: 1,
            banished: None,
        });
    to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    (e, v)
}

// [PIB 20121031] Anarch Convert "Can "convert" an already anarch vampire if
// he is not titled", and [LSJ 20060620] "You can draw more cards than are in
// the library or crypt (even from an empty library or crypt)": moved out,
// he is removed from the game to make a Camarilla vampire Anarch, then
// draws from the empty crypt (nothing). Another one makes that (now
// Anarch) vampire Anarch again and gains 1 pool.
#[test]
fn anarch_convert_makes_a_vampire_anarch_as_he_enters_play() {
    let (mut e, v) = convert_table();
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    let req = expect(&mut e, SeatId(0), WindowLabel::EntryEffect);
    assert!(req.options.contains(&Choice::Pass), "may keep him");
    pick(&mut e, Choice::ChooseMinion(v));
    pick(&mut e, Choice::DrawCrypt);
    assert_eq!(e.state.sect_of(v), Some(Sect::Anarch));
    assert_eq!(
        e.state.seats[0].removed_crypt,
        vec![find_card("Anarch Convert")]
    );
    assert!(e.state.seats[0].uncontrolled.is_empty());

    let (mut e, v) = convert_table();
    e.state.minions[v.index()].sect_override = Some(Sect::Anarch);
    let pool0 = pool(&e)[0];
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    pick(&mut e, Choice::ChooseMinion(v));
    pick(&mut e, Choice::Pass);
    assert_eq!(pool(&e)[0], pool0 + 1);
}

// Anarch Convert [LSJ 20080526] [RTR 20180303] [ANK 20211214] "His "as he
// enters play" ability can be used before he enters contest if there is
// another one in play. This is not self-contesting": with a Convert
// already in play, the second can be moved out — his removal then not
// optional — and no contest begins; the one in play is the vampire made
// Anarch. Kept, a Convert simply enters play.
#[test]
fn anarch_convert_beside_another_is_removed_before_any_contest() {
    let (mut e, _) = convert_table();
    let first = e.add_minion(SeatId(0), find_card("Anarch Convert"), 1, false);
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    let req = expect(&mut e, SeatId(0), WindowLabel::EntryEffect);
    assert!(!req.options.contains(&Choice::Pass), "{:?}", req.options);
    pick(&mut e, Choice::ChooseMinion(first));
    pick(&mut e, Choice::Pass);
    assert!(!e.state.minions[first.index()].contested);
    assert!(e.state.minions[first.index()].in_play);

    let (mut e, _) = convert_table();
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    pick(&mut e, Choice::Pass);
    let convert = find_card("Anarch Convert");
    assert!(e
        .state
        .minions
        .iter()
        .any(|m| m.in_play && m.crypt_card() == Some(convert)));
}

// ---- Become Anarch (slice 16k) ------------------------------------------------------

fn become_anarch(m: MinionId) -> Choice {
    action(m, zvtes::decision::ActionKind::BecomeAnarch)
}

// Advanced rules "Become Anarch": any ready untitled non-Anarch vampire, +1
// stealth, undirected; 2 blood, 1 if the controller controls another ready
// Anarch, paid at resolution — a titled vampire cannot; Brunhilde makes it
// free for her Methuselah's vampires.
#[test]
fn become_anarch_is_a_rulebook_action() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 0), 4, false);
    let titled = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = request(&mut e);
    assert!(req.options.contains(&become_anarch(v)));
    assert!(!req.options.contains(&become_anarch(titled)));
    pick(&mut e, become_anarch(v));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.sect_of(v), Some(Sect::Anarch));
    assert_eq!(blood(&e, v), 4 - 2);

    // Another ready Anarch: 1 blood.
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 0), 4, false);
    e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, become_anarch(v));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, v), 4 - 1);

    // Brunhilde: 0.
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 0), 4, false);
    e.add_minion(SeatId(0), find_card("Brunhilde"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, become_anarch(v));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, v), 4);
    assert_eq!(e.state.sect_of(v), Some(Sect::Anarch));
}

// ---- Elimelech, Reginald Moore, Valkyrie, The Medic --------------------------------

// Elimelech "strikes with first strike" (rulebook "First Strike"): his hand
// strike (strength 2) resolves first; the blocker, sent to torpor by it,
// strikes back not at all. Against a dodge, nothing resolves first.
#[test]
fn elimelech_strikes_first() {
    let mut e = game(2);
    let elimelech = e.add_minion(SeatId(0), find_card("Elimelech the Twice-Damned"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick(&mut e, bleed(elimelech));
    pick(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor);
    assert_eq!(blood(&e, elimelech), 6, "no strike back");
}

// Reginald Moore: "+1 strength in combat with a Tremere or Tremere
// antitribu. He cannot block vampires of those clans or take actions to
// enter combat with them." A Tremere's bleed: he is no blocker; his Bum's
// Rush cannot target a Tremere, a Ventrue it can.
#[test]
fn reginald_moore_shuns_the_tremere() {
    let mut e = game(2);
    let tremere = e.add_minion(SeatId(0), plain("Tremere", 0), 4, false);
    e.add_minion(SeatId(0), plain("Tremere", 1), 4, false);
    let reginald = e.add_minion(SeatId(1), find_card("Reginald Moore"), 4, false);
    pick(&mut e, bleed(tremere));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(reginald));

    let mut e = game(2);
    let reginald = e.add_minion(SeatId(0), find_card("Reginald Moore"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let tremere = e.add_minion(SeatId(1), plain("Tremere", 0), 4, false);
    let ventrue = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Bum's Rush"]);
    let rush = |t: MinionId| Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(reginald),
        target: Target::Minion(t),
    };
    let req = request(&mut e);
    assert!(!req.options.contains(&rush(tremere)));
    assert!(req.options.contains(&rush(ventrue)));
    pick(&mut e, rush(ventrue));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        e.state.minions[reginald.index()].locked,
        "the rush was taken"
    );
}

// Valkyrie: "cannot perform directed actions unless Brunhilde is ready.
// Non-unique. +1 strength." No bleed offered; Brunhilde ready (any
// Methuselah's), it is.
#[test]
fn valkyrie_bleeds_only_with_brunhilde_ready() {
    let mut e = game(2);
    let valkyrie = e.add_minion(SeatId(0), find_card("Valkyrie"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = request(&mut e);
    assert!(!req.options.contains(&bleed(valkyrie)));
    let mut e = game(2);
    let valkyrie = e.add_minion(SeatId(0), find_card("Valkyrie"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), find_card("Brunhilde"), 4, false);
    let req = request(&mut e);
    assert!(req.options.contains(&bleed(valkyrie)));
}

// The Medic: "During your turn, you can burn the Edge to add 1 blood to an
// Anarch you control." With the Edge, on its turn: an Anarch of its
// Methuselah's gains 1, the Edge is gone; a Camarilla vampire is no target.
#[test]
fn the_medic_burns_the_edge_for_an_anarchs_blood() {
    let mut e = game(2);
    let medic = e.add_minion(SeatId(0), find_card("Medic, The"), 4, false);
    let anarch = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 2, false);
    let cam = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 0), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].has_edge = true;
    let heal = |t: MinionId| Choice::UseAbility {
        source: AbilitySource::Vampire(medic),
        ability: 0,
        actor: None,
        target: Target::Minion(t),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&heal(anarch)), "{:?}", req.options);
    assert!(!req.options.contains(&heal(cam)));
    pick(&mut e, heal(anarch));
    assert_eq!(blood(&e, anarch), 3);
    assert!(!e.state.seats[0].has_edge);
}

// ---- Arishat, Scout Youngwood, Danielle Diron (slice 16k) ------------------------

fn host_ability(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: None,
        target,
    }
}

/// Seat 0's Kine Resources Contested called by `caller` in a 2-seat game
/// (seat 0 taking its 3 points, the rest elides), the polling step
/// reached with seat 0's impulse first.
fn kine_called(e: &mut Engine, caller: MinionId) {
    hand(e, 0, &["Kine Resources Contested"]);
    pick(e, play(0, 0, caller));
    pick(e, alloc(0, 3));
}

// Arishat [LSJ 20100311] "Her ability to force a vampire to abstain can
// cancel the votes and ballots of the vampire", [ANK 20220705] [LSJ
// 20100705] [LSJ 20011022] [LSJ 20100902-2] [ANK 20180307-2] "The ability can
// be used while locked or in torpor, only once per referendum": locked, she
// burns 1 blood to make a prince who cast 2 votes abstain — the votes are
// gone — and is not offered it twice. [LSJ 19971001] "Her ability applies
// to Priscus votes and ballots": a priscus who cast ballots abstains too.
#[test]
fn arishat_forces_a_vampire_to_abstain_once_a_referendum() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let arishat = e.add_minion(SeatId(1), find_card("Arishat"), 3, false);
    let prince = e.add_minion(SeatId(1), find_card("Karen Suadela"), 4, false);
    e.state.minions[arishat.index()].locked = true;
    kine_called(&mut e, caller);
    pick_when(&mut e, votes(prince, false));
    let force = |t: MinionId| host_ability(arishat, 0, Target::Minion(t));
    pick_when(&mut e, force(prince));
    assert_eq!(blood(&e, arishat), 2);
    let r = e.referendum().expect("referendum");
    assert!(r.abstaining.contains(&prince));
    let req = request(&mut e);
    assert!(!req.options.contains(&force(caller)), "once a referendum");
}

// Scout Youngwood [PIB 20150924-2] "Card with multiple types (eg.
// Reaction/Action Modifier) are considered of the type they are played as,
// not both": reaction cards cost her +1 blood — Scalpel Tongue played as a
// reaction (another's action) costs her 1, as her own action modifier 0.
// During her unlock phase she burns 2 blood to lock a younger vampire of
// her prey's.
#[test]
fn scout_youngwood_pays_more_for_reactions_and_locks_the_preys_young() {
    let mut e = game(2);
    let scout = e.add_minion(SeatId(0), find_card("Scout Youngwood"), 6, false);
    let young = e.add_minion(
        SeatId(1),
        CRYPT.iter().position(|c| c.capacity == 3).unwrap() as u16,
        2,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let _ = req;
    // Her next unlock phase: the lock.
    let lock = host_ability(scout, 0, Target::Minion(young));
    pick_when(&mut e, lock);
    assert!(e.state.minions[young.index()].locked);
    assert_eq!(blood(&e, scout), 6 - 2);
    // The surcharge, read on the played card.
    let card = find_library("Scalpel Tongue");
    let reacting = zvtes::state::PrintedCtx {
        card: Some((card, 0)),
        reacting: true,
        ..Default::default()
    };
    let acting = zvtes::state::PrintedCtx {
        card: Some((card, 0)),
        ..Default::default()
    };
    assert_eq!(
        e.state
            .printed_mod(scout, cards::Stat::BloodCostMore, reacting),
        1
    );
    assert_eq!(
        e.state
            .printed_mod(scout, cards::Stat::BloodCostMore, acting),
        0
    );

    // A reaction actually played: Party Out Of Bounds [obf] against a bleed,
    // free, costs her 1 blood (made Anarch, its requirement).
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let scout = e.add_minion(SeatId(1), find_card("Scout Youngwood"), 5, false);
    e.state.minions[scout.index()].sect_override = Some(Sect::Anarch);
    hand(&mut e, 1, &["Party Out Of Bounds"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, scout));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, scout), 5 - 1);
}

// Danielle Diron [ANK 20220705] "The ability can be used while locked, only
// once per referendum": locked, she burns 1 blood for +1 vote; not twice.
#[test]
fn danielle_diron_buys_a_vote_once_a_referendum() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let danielle = e.add_minion(SeatId(1), find_card("Danielle Diron"), 3, false);
    e.state.minions[danielle.index()].locked = true;
    kine_called(&mut e, caller);
    let vote = host_ability(danielle, 0, Target::None);
    pick_when(&mut e, vote);
    assert_eq!(blood(&e, danielle), 2);
    let req = request(&mut e);
    assert!(!req.options.contains(&vote));
}

// ---- Infernal, personal titles, Black Hand ------------------------------------------

// Rulebook "Infernal" and [LSJ 20050114] "The "does not unlock as normal"
// effect is redundant with being infernal. If the minion is infernal, his
// controller can still pay a pool to unlock him" (Mind Numb's stun, Puppet
// Master, Rötschreck): Arishat stays locked through the unlock, unless her
// Methuselah burns 1 pool — offered whatever keeps her from unlocking as
// normal.
#[test]
fn an_infernal_vampire_unlocks_for_a_pool() {
    for how in ["none", "stun", "Puppet Master", "Rötschreck"] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let arishat = e.add_minion(SeatId(1), find_card("Arishat"), 3, false);
        e.state.minions[arishat.index()].locked = true;
        match how {
            "stun" => e.state.minions[arishat.index()].skip_unlock = true,
            "none" => {}
            card => {
                let id = in_play(&mut e, 0, card, Some(arishat));
                let _ = id;
            }
        }
        let unlock = Choice::UnlockInfernal { minion: arishat };
        let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
        assert!(e.state.minions[arishat.index()].locked, "{how}");
        assert!(req.options.contains(&unlock), "{how}: {:?}", req.options);
        let pool1 = pool(&e)[1];
        pick(&mut e, unlock);
        assert!(!e.state.minions[arishat.index()].locked, "{how}");
        assert_eq!(pool(&e)[1], pool1 - 1);
    }
}

// Personal titles [LSJ 20010714] "Keeps their title and votes if they
// changes sects": Arishat's 1 vote (titled) and Scout Youngwood's 2 votes
// stay with them made Anarch.
#[test]
fn personal_titles_survive_a_sect_change() {
    let mut e = game(2);
    let arishat = e.add_minion(SeatId(0), find_card("Arishat"), 3, false);
    let scout = e.add_minion(SeatId(0), find_card("Scout Youngwood"), 3, false);
    for m in [arishat, scout] {
        e.state.minions[m.index()].sect_override = Some(Sect::Anarch);
    }
    assert_eq!(e.state.title_of(arishat), Title::Votes(1));
    assert_eq!(e.state.title_of(scout), Title::Votes(2));
    // A sect's title is lost off its sect (a prince made Independent).
    let tara = e.add_minion(SeatId(0), find_card("Tara"), 3, false);
    e.state.minions[tara.index()].sect_override = Some(Sect::Independent);
    assert_eq!(e.state.title_of(tara), Title::None);
    assert_eq!(e.state.title_of(arishat), Title::Votes(1));
}

// Black Hand [LSJ 20070322] [ANK 20180807] "Black Hand is not a title, it
// is a trait unrelated to any sect. The trait is not lost if the vampire
// changes sect": Fall of the Sabbat is "not usable if a ready vampire is
// Black Hand" — Elimelech, Sabbat or not, bars it.
#[test]
fn black_hand_is_a_trait_beside_the_sect() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let elimelech = e.add_minion(SeatId(1), find_card("Elimelech the Twice-Damned"), 4, false);
    e.state.minions[elimelech.index()].sect_override = Some(Sect::Independent);
    for name in ["Thirst", "Torpid Blood", "Blood Weakens"] {
        in_play(&mut e, 1, name, None);
    }
    hand(&mut e, 0, &["Fall of the Sabbat", "Minion Tap"]);
    let req = to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert!(
        !plays(&req)
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0, .. })),
        "{req:?}"
    );
    assert!(
        e.state.minions[elimelech.index()]
            .vampire()
            .unwrap()
            .black_hand
    );
}

// ---- Karen Suadela -----------------------------------------------------------------------

// Karen Suadela [LSJ 20100121] [LSJ 20081120-2] "She can't take actions
// provided by Gehenna events in play (The New Inquisition, Restricted
// Vitae). She keeps her title if Fall of the Camarilla is played … She
// doesn't become immune to blood hunts via Fueled by Heart's Blood": with
// The New Inquisition in play she is offered no referendum to burn it, a
// titled Ventrue is; under Fall of the Camarilla she is still Camarilla and
// a prince; Fueled by Heart's Blood bars no blood hunt on her.
#[test]
fn karen_suadela_takes_no_action_a_gehenna_event_provides() {
    let mut e = game(2);
    let karen = e.add_minion(SeatId(0), find_card("Karen Suadela"), 4, false);
    let tara = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let inq = in_play(&mut e, 1, "New Inquisition, The", None);
    let req = request(&mut e);
    let burn = |m: MinionId| use_at(inq, 1, Some(m), Target::None);
    assert!(req.options.contains(&burn(tara)), "{:?}", req.options);
    assert!(!req.options.contains(&burn(karen)));
    in_play(&mut e, 1, "Fall of the Camarilla", None);
    assert_eq!(e.state.sect_of(karen), Some(Sect::Camarilla));
    assert!(matches!(e.state.title_of(karen), Title::Prince(_)));
    assert_eq!(e.state.sect_of(tara), Some(Sect::Independent));
    assert!(e.state.gehenna_immune(karen));
}

// Karen Suadela [LSJ 20081120-2] [LSJ 20100121] "She can be chosen for the
// effects of a Gehenna event, but stays unaffected: … Recalled to the
// Founder, but doesn't burn. … Nightmares upon Nightmares, but she isn't
// locked if they don't. Blood Trade can't move blood from her, and blood
// moved to her go to the blood bank instead": The New Inquisition's damage
// chosen on her lands not; Blood Trade's giver cannot be her.
#[test]
fn karen_suadela_chosen_by_a_gehenna_event_stays_unaffected() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let karen = e.add_minion(SeatId(1), find_card("Karen Suadela"), 4, false);
    let other = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let inq = in_play(&mut e, 0, "New Inquisition, The", None);
    // The New Inquisition on seat 0's next unlock: Karen chosen, no damage.
    let hit = |t: MinionId| use_at(inq, 0, None, Target::Minion(t));
    pick_when(&mut e, hit(karen));
    assert_eq!(blood(&e, karen), 4);
    pick_when(&mut e, Choice::Pass);
    let _ = other;
    // Blood Trade, seat 1's: no move from Karen.
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let karen = e.add_minion(SeatId(1), find_card("Karen Suadela"), 4, false);
    let giver = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let trade = in_play(&mut e, 1, "Blood Trade", None);
    let req = offering(&mut e, use_at(trade, 0, None, Target::Minion(giver)));
    assert!(!req
        .options
        .contains(&use_at(trade, 0, None, Target::Minion(karen))));
}

// Karen Suadela [LSJ 20081120-2] [LSJ 20100121] "Her immunity does not
// extend to other things affected by events: … Her controller burns pool
// to Dragonbound while she is in Torpor": in torpor, she costs her
// Methuselah 1 pool to Dragonbound.
#[test]
fn karen_suadelas_immunity_stops_at_her() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), find_card("Karen Suadela"), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Dragonbound", None);
    hand(&mut e, 0, &["Minion Tap"]);
    to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    let req = request(&mut e);
    let due = req
        .options
        .iter()
        .find(|o| matches!(o, Choice::ResolveRider { .. }))
        .copied()
        .expect("Dragonbound due");
    pick(&mut e, due);
    assert_eq!(pool(&e)[0], 10 - 1);
}

// ---- Patsy, Unflinching Persistence ------------------------------------------------------

// Patsy [LSJ 20080603] [LSJ 20050128] "Counts as requiring an Anarch", and
// [LSJ 20100129] [ANK 20191228] "Cannot be used or played if the conditions
// for the terms of the referendum cannot be met": a baron with the Edge
// calls it on a prince — passed, the Edge is burned, the prince loses the
// title and its controller 2 pool. With no titled non-Anarch vampire, it
// is not offered.
#[test]
fn patsy_burns_the_edge_to_strip_a_title() {
    assert!(
        cards::CardFilter::HasLabel(zvtes::library::Label::RequiresAnarch)
            .matches(find_library("Patsy"))
    );
    let mut e = game(2);
    let baron = e.add_minion(SeatId(0), find_card("Jacques Rouge"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].has_edge = true;
    hand(&mut e, 0, &["Patsy"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play(0, 0, baron)),
        "no titled non-Anarch"
    );

    let mut e = game(2);
    let baron = e.add_minion(SeatId(0), find_card("Jacques Rouge"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let tara = e.add_minion(SeatId(1), find_card("Tara"), 4, false);
    e.state.seats[0].has_edge = true;
    hand(&mut e, 0, &["Patsy"]);
    // Tara is the only choice: the terms elide.
    pick(&mut e, play(0, 0, baron));
    pick_when(&mut e, votes(baron, true));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.title_of(tara), Title::None);
    assert!(!e.state.seats[0].has_edge);
    assert_eq!(pool(&e)[1], 10 - 2);
}

// Unflinching Persistence [LSJ 20001114] "Cannot be used if there is no
// damage to prevent": A's .44 Magnum strike, committed at long range, hits
// X, whose hand strike cannot reach — no damage on A, no [for] for it.
#[test]
fn unflinching_persistence_needs_damage_to_prevent() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("FOR", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let gun = equip(&mut e, ".44 Magnum", a);
    hand(&mut e, 0, &["Unflinching Persistence"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(x));
    pick_when(&mut e, use_at(gun, 1, None, Target::None));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
    assert_eq!(blood(&e, x), 4 - 2);
}

// ---- Mask of a Thousand Faces (slice 16k) -----------------------------------------

fn mask(mode: u8, by: MinionId) -> Choice {
    Choice::Play {
        card: 0,
        mode,
        actor: Some(by),
        target: Target::None,
    }
}

/// An Obfuscate vampire with no printed text, not of `clan` (any if
/// empty), the `nth`.
fn obf(nth: usize) -> u16 {
    vampire_with("OBF", nth)
}

// Mask of a Thousand Faces [LSJ 20010803] "Can be used after the action is
// blocked and before the block resolution", [LSJ 19990425] "Can only be
// played by the controller of the acting minion", [LSJ 19990106] "A minion
// attempting to block before is still attempting to block after": A's
// bleed blocked, B takes it over — A unlocks, B locks and fights the
// blocker. The blocker's Methuselah, with a Mask and an Obfuscate vampire,
// is never offered it.
#[test]
fn mask_of_a_thousand_faces_takes_over_a_blocked_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let b = e.add_minion(SeatId(0), obf(0), 5, false);
    let x = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let y = e.add_minion(SeatId(1), obf(1), 4, false);
    hand(&mut e, 0, &["Mask of a Thousand Faces"]);
    hand(&mut e, 1, &["Mask of a Thousand Faces"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    let req = until(&mut e, SeatId(0), WindowLabel::ActionBlocked);
    assert!(req.options.contains(&mask(0, b)), "{:?}", req.options);
    pick(&mut e, mask(0, b));
    // Resolved as the next decision is reached.
    request(&mut e);
    assert!(!e.state.minions[a.index()].locked);
    assert!(e.state.minions[b.index()].locked);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, mask(0, y));
    assert_eq!(blood(&e, a), 4);
    assert!(blood(&e, b) < 5, "B fought the blocker");
}

// Mask of a Thousand Faces [RTR 19951110] [LSJ 19971201] [RTR 20030519] [LSJ
// 20030521] [ANK 20210309] "The action remains the same. All action
// modifiers remain in effect, but inherent modifiers (a minion's inherent +1
// bleed for example) do not": Elimelech's bleed (2 with his +1) taken over
// by B bleeds for 1.
#[test]
fn mask_of_a_thousand_faces_leaves_the_inherent_modifiers() {
    let mut e = game(2);
    let elimelech = e.add_minion(SeatId(0), find_card("Elimelech the Twice-Damned"), 6, false);
    let b = e.add_minion(SeatId(0), obf(0), 5, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Mask of a Thousand Faces"]);
    pick(&mut e, bleed(elimelech));
    pick_when(&mut e, mask(0, b));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(pool(&e)[1], 10 - 1);
}

// Mask of a Thousand Faces [RTR 19980623] [RTR 20030519] [LSJ 20030520] [RTR
// 20041202] "Cannot be used to mask an action if the masking vampire is not
// capable of taking that action …", [LSJ 20020926] not an action an
// equipment (a card in play) grants, [LSJ 19980825] "Cannot be used during
// the resolution of an action, like during a referendum": a Nosferatu's
// SchreckNET is masked by another Nosferatu, not by a non-Nosferatu; the burn
// action Constant Revolution grants is masked by none; a referendum offers
// no Mask.
#[test]
fn mask_of_a_thousand_faces_needs_a_vampire_able_to_take_the_action() {
    let nosferatu = |nth: usize| {
        CRYPT
            .iter()
            .enumerate()
            .filter(|(i, c)| {
                c.clan == "Nosferatu"
                    && c.disciplines.contains(&"OBF")
                    && cards::vampire(*i as u16).is_none()
            })
            .nth(nth)
            .map(|(i, _)| i as u16)
            .expect("an Obfuscate Nosferatu")
    };
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), nosferatu(0), 4, false);
    let b = e.add_minion(SeatId(0), nosferatu(1), 4, false);
    let c = e.add_minion(
        SeatId(0),
        CRYPT
            .iter()
            .enumerate()
            .position(|(i, v)| {
                v.clan != "Nosferatu"
                    && v.disciplines.contains(&"OBF")
                    && cards::vampire(i as u16).is_none()
            })
            .expect("an Obfuscate non-Nosferatu") as u16,
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["SchreckNET", "Mask of a Thousand Faces"]);
    pick(&mut e, play(0, 0, a));
    let req = offering(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(b),
            target: Target::None,
        },
    );
    assert!(!req.options.contains(&Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(c),
        target: Target::None
    }));

    // A referendum: no Mask in it.
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    let b = e.add_minion(SeatId(0), obf(0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    kine_called(&mut e, caller);
    hand(&mut e, 0, &["Mask of a Thousand Faces"]);
    loop {
        let req = request(&mut e);
        if req.window == WindowLabel::MinionPhase {
            break;
        }
        if matches!(req.window, WindowLabel::ReferendumPolling) {
            assert!(!req
                .options
                .iter()
                .any(|o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == b)));
        }
        let i = fallback(&req);
        e.choose(i);
    }

    // A granted action (Constant Revolution's burn): no Mask.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), obf(0), 4, false);
    let b = e.add_minion(SeatId(0), obf(1), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let rev = in_play(&mut e, 1, "Constant Revolution", None);
    hand(&mut e, 0, &["Mask of a Thousand Faces"]);
    pick(&mut e, use_at(rev, 0, Some(a), Target::None));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, mask(0, b));
}

// Mask of a Thousand Faces [LSJ 20001113] "Can be used to take over a
// mandatory action that was not mandatory for the masker, but that he could
// perform (eg. a hunt)", [PIB 20150924-1] [PIB 20150930] "Can be used after
// blocks are declined …", [ANK 20200515] "Multiple copies can be played by
// different vampires on the same action": A with no blood must hunt; the
// blocks declined, B masks it, then C masks it from B; C hunts.
#[test]
fn mask_of_a_thousand_faces_takes_over_a_forced_hunt_twice() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 0, false);
    let b = e.add_minion(SeatId(0), obf(0), 3, false);
    let c = e.add_minion(SeatId(0), obf(1), 3, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    hand(
        &mut e,
        0,
        &["Mask of a Thousand Faces", "Mask of a Thousand Faces"],
    );
    // The forced hunt is its only option: declared at once.
    pick_when(&mut e, mask(0, b));
    pick_when(&mut e, mask(0, c));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, c), 3 + 1);
    // A, unlocked by the first Mask and still without blood, must hunt
    // again: its own hunt.
    assert_eq!((blood(&e, a), blood(&e, b)), (1, 3));
}

// Mask of a Thousand Faces and Haqim's Law: Retribution [ANK 20221102-2] "If
// used by a Banu Haqim and another takes the action over with Mask of a
// Thousand Faces, they cannot use it again": once per action per copy,
// whoever the acting minion becomes.
#[test]
fn haqims_law_is_spent_for_the_action_a_mask_takes_over() {
    let haqim = |nth: usize| {
        CRYPT
            .iter()
            .enumerate()
            .filter(|(i, c)| {
                c.clan == "Banu Haqim"
                    && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("obf"))
                    && cards::vampire(*i as u16).is_none()
            })
            .nth(nth)
            .map(|(i, _)| i as u16)
            .expect("a Banu Haqim with Obfuscate")
    };
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), haqim(0), 4, false);
    let b = e.add_minion(SeatId(0), haqim(1), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let law = in_play(&mut e, 0, "Haqim's Law: Retribution", None);
    hand(
        &mut e,
        0,
        &["Mask of a Thousand Faces", "Side Strike", "Side Strike"],
    );
    pick(&mut e, bleed(a));
    pick_when(&mut e, use_at(law, 0, Some(a), Target::None));
    pick_when(&mut e, mask(0, b));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_at(law, 0, Some(b), Target::None),
    );
    assert_eq!(pool(&e)[1], 10 - 2, "B's bleed of 1, the Law's +1");
}

// ---- Disguised Weapon, Hackerspace ---------------------------------------------------------

// Disguised Weapon [LSJ 20100130] [ANK 20180925-2] [LSJ 19991207] the weapon
// named as it is played, [LSJ 20100725] [LSJ 20100303] [ANK 20180817] its
// requirements applying, [ANK 20201229] not played by the vampire: blocked,
// before range A equips a .44 Magnum from hand, its 2 pool paid, and strikes
// with it. [LSJ 19980319] "If used to equip and contest a unique weapon
// already in play, neither weapon is available for use during the combat":
// equipping an Ivory Bow while X has one, both are contested.
#[test]
fn disguised_weapon_equips_a_weapon_from_hand_in_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Disguised Weapon", ".44 Magnum"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    let dw = Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(a),
        target: Target::None,
    };
    pick_when(&mut e, dw);
    let req = request(&mut e);
    if req.window == WindowLabel::PlayTerms {
        pick(&mut e, Choice::FindCard(find_library(".44 Magnum")));
    }
    request(&mut e);
    let gun = find_library(".44 Magnum");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == gun && c.host == Some(a) && c.in_play));
    assert_eq!(pool(&e)[0], 10 - 2);

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let fang = equip(&mut e, "Ivory Bow", x);
    hand(&mut e, 0, &["Disguised Weapon", "Ivory Bow"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, dw);
    let req = request(&mut e);
    if req.window == WindowLabel::PlayTerms {
        pick(&mut e, Choice::FindCard(find_library("Ivory Bow")));
    }
    request(&mut e);
    assert!(e.state.cards[fang.index()].contested);
}

// Hackerspace [ANK 20210410] "A counter is added when Anarch Convert enters
// play, before using his ability. If he uses his ability to make a
// non-anarch vampire anarch, a second counter is added": it enters with a
// counter for the Anarch in play; the Convert's entry adds one, his making
// a Camarilla vampire Anarch another; the counters then pay an Anarch's
// equip.
#[test]
fn hackerspace_counts_anarchs_arriving() {
    let mut e = game(2);
    e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 4, false);
    let cam = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Hackerspace"]);
    // Through seat 1's turn to seat 0's master phase.
    to_window(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, master_play(0));
    request(&mut e);
    let space = card_named(&e, "Hackerspace");
    assert_eq!(e.state.cards[space.index()].counters, 1);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: find_card("Anarch Convert"),
            counters: 1,
            banished: None,
        });
    to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    request(&mut e);
    assert_eq!(e.state.cards[space.index()].counters, 2);
    pick(&mut e, Choice::ChooseMinion(cam));
    assert_eq!(e.state.cards[space.index()].counters, 3);
}

/// The card in play of that name.
fn card_named(e: &Engine, name: &str) -> CardId {
    let card = find_library(name);
    CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.in_play && c.card == card)
            .expect("in play") as u32,
    )
}

// ---- Benjamin Rose, Dmitra Ilyanova, SchreckNET --------------------------------------------

fn printed(m: MinionId) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability: 0,
        actor: Some(m),
        target: Target::None,
    }
}

// Benjamin Rose: "can call a referendum to give you +1 hand size until the
// end of the game as a +1 stealth political action": passed, his
// Methuselah's hand size is 8, for the rest of the game.
#[test]
fn benjamin_rose_calls_for_a_hand_size_for_good() {
    let mut e = game(2);
    let benjamin = e.add_minion(SeatId(0), find_card("Benjamin Rose"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, printed(benjamin));
    pick_when(&mut e, votes(benjamin, true));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.hand_size(SeatId(0)), 8);
}

// Dmitra Ilyanova: "can call a referendum to distribute 4 blood from the
// blood bank among one or more ready vampires as a +1 stealth political
// action": 3 to her ally-to-be, 1 to herself, picked as the terms; passed,
// they gain them.
#[test]
fn dmitra_ilyanova_distributes_four_blood() {
    let mut e = game(2);
    let dmitra = e.add_minion(SeatId(0), find_card("Dmitra Ilyanova"), 4, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, printed(dmitra));
    for m in [other, other, other, dmitra] {
        pick_when(&mut e, Choice::ChooseMinion(m));
    }
    pick_when(&mut e, votes(dmitra, true));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, other), 2 + 3);
    assert_eq!(blood(&e, dmitra), 4 + 1);
}

// SchreckNET: "+1 stealth action. Unique. If this Nosferatu's capacity is 6
// or more, he or she unlocks. Put this card in play. Whenever a referendum
// succeeds, you may lock this card to look at any Methuselah's hand": a
// capacity-6+ Nosferatu plays it and unlocks; a passed referendum later,
// the card locks to open seat 1's hand to seat 0 for that action.
#[test]
fn schrecknet_unlocks_a_big_nosferatu_and_peeks_after_a_referendum() {
    let nosferatu = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.clan == "Nosferatu" && c.capacity >= 6 && cards::vampire(i as u16).is_none()
        })
        .expect("a big Nosferatu") as u16;
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), nosferatu, 4, false);
    let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["SchreckNET"]);
    hand(&mut e, 1, &["Minion Tap"]);
    pick(&mut e, play(0, 0, n));
    to_minion_phase(&mut e, SeatId(0));
    assert!(!e.state.minions[n.index()].locked);
    let net = card_named(&e, "SchreckNET");
    kine_called(&mut e, caller);
    pick_when(&mut e, votes(caller, true));
    pick_when(&mut e, use_at(net, 0, None, Target::Seat(SeatId(1))));
    assert!(e.state.hand_visible_to(SeatId(1), SeatId(0)));
    to_minion_phase(&mut e, SeatId(0));
    assert!(!e.state.hand_visible_to(SeatId(1), SeatId(0)));
    assert!(e.state.cards[net.index()].locked);
}

// ---- Become Anarch and Aline Gädeke, first strike and Soul Burn, Valkyrie --------------

// Aline Gädeke [ANK 20220814] "Can unlock after a vampire performs an
// action to become Anarch": another vampire of hers becomes Anarch; after
// resolution, locked Aline burns 1 blood to unlock.
#[test]
fn aline_gadeke_unlocks_after_a_vampire_becomes_anarch() {
    let mut e = game(2);
    let aline = e.add_minion(SeatId(0), find_card("Aline Gädeke"), 3, false);
    let v = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[aline.index()].locked = true;
    pick(&mut e, become_anarch(v));
    let unlock = Choice::UseAbility {
        source: AbilitySource::Vampire(aline),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick_when(&mut e, unlock);
    assert!(!e.state.minions[aline.index()].locked);
}

// First strike and Soul Burn [LSJ 20040928] "A weapon strike done with
// first strike is unaffected (unless Soul Burn is used with first strike
// too)", and Blood Fury alike: Elimelech's .44 Magnum strike lands first —
// the blocker's Soul Burn, after it, cannot keep it from inflicting its 2.
#[test]
fn a_first_strike_weapon_beats_soul_burn() {
    let mut e = game(2);
    let elimelech = e.add_minion(SeatId(0), find_card("Elimelech the Twice-Damned"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), vampire_with("tha", 0), 5, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let gun = equip(&mut e, ".44 Magnum", elimelech);
    hand(&mut e, 1, &["Soul Burn"]);
    pick(&mut e, bleed(elimelech));
    pick_when(&mut e, block(x));
    pick_when(&mut e, use_at(gun, 0, None, Target::None));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // The gun's 2, and Soul Burn's own blood.
    assert_eq!(blood(&e, x), 5 - 2 - 1);
}

// Valkyrie: "Non-unique": two of them in play, no contest.
#[test]
fn valkyrie_is_non_unique() {
    let mut e = game(2);
    let v1 = e.add_minion(SeatId(0), find_card("Valkyrie"), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: find_card("Valkyrie"),
            counters: 4,
            banished: None,
        });
    to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    let copies: Vec<_> = e
        .state
        .minions
        .iter()
        .filter(|m| m.in_play && m.name() == "Valkyrie")
        .collect();
    assert_eq!(copies.len(), 2);
    assert!(copies.iter().all(|m| !m.contested));
    let _ = v1;
}

// Mask of a Thousand Faces [ANK 20190117-1] "The masking vampire can play a
// modifier already played by the masked minion. He cannot use a limited
// effect (ie. bleed enhancer) if a limited effect has already been used on
// the action": A plays Cloak the Gathering; B, masking, can play another —
// the once-per-action limit is each minion's; a limited bleed stays the
// action's.
#[test]
fn mask_of_a_thousand_faces_replays_the_masked_minions_modifier() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), obf(0), 4, false);
    let b = e.add_minion(SeatId(0), obf(1), 5, false);
    let x = e.add_minion(SeatId(1), vampire_with("AUS", 0), 4, false);
    in_play(&mut e, 1, "Mr. Winthrop", Some(x));
    in_play(&mut e, 1, "Mr. Winthrop", Some(x));
    hand(
        &mut e,
        0,
        &[
            "Cloak the Gathering",
            "Mask of a Thousand Faces",
            "Cloak the Gathering",
        ],
    );
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, mask(0, b));
    // The other Cloak, now card 0 again: B, the acting minion, plays it.
    let req = offering(&mut e, play(0, 0, b));
    assert!(req.options.contains(&play(0, 0, b)));
}

// ---- Slice 16l: steps 10-15 ---------------------------------------------------------------

// Aksinya Daclau [LSJ 20000507] "Her ability can only be used after blocks
// are declined", [ANK 20180719-4] "Once each turn": seat 1 declines seat
// 0's bleed and discards a master to send it to seat 2; a second bleed that
// turn finds the ability spent.
#[test]
fn aksinya_daclau_redirects_a_bleed_once_each_turn() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let aksinya = e.add_minion(SeatId(1), find_card("Aksinya Daclau"), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Auspex", "Auspex"]);
    e.state.seats[1].library = vec![find_library("Trap")];
    let redirect = host_ability(aksinya, 0, Target::Seat(SeatId(2)));
    pick(&mut e, bleed(a));
    let req = to_window(&mut e, SeatId(1), WindowLabel::ActionBlocksDeclined);
    assert!(req.options.contains(&redirect), "{:?}", req.options);
    // Only to seat 2: not the acting Methuselah, not herself.
    assert!(!req
        .options
        .contains(&host_ability(aksinya, 0, Target::Seat(SeatId(0)))));
    pick(&mut e, redirect);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10);
    assert_eq!(pool(&e)[2], 9);
    assert_eq!(e.state.seats[1].ash, vec![find_library("Auspex")]);
    // Replaced at once (owner, 2026-10-09: rules-qa 59.16).
    assert_eq!(e.state.seats[1].hand.len(), 2);
    pick(&mut e, bleed(b));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, redirect);
}

// T.J. [ANK 20200508-2] "Must be unlocked with at least 2 blood", [LSJ
// 20011022] "The ability can be used when in torpor"; Gerald Windham [ANK
// 20190725] "His ability gives him votes but no title": T.J., in torpor,
// locks and burns 2 blood to cancel a referendum Tara (a Camarilla prince)
// called; Gerald's, called untitled, he cannot.
#[test]
fn tj_cancels_a_titled_camarilla_referendum_from_torpor() {
    let mut e = game(2);
    let tara = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let tj = e.add_minion(SeatId(1), find_card("T.J."), 3, false);
    e.state.minions[tj.index()].in_torpor = true;
    kine_called(&mut e, tara);
    let cancel = host_ability(tj, 0, Target::None);
    pick_when(&mut e, cancel);
    assert!(e.state.minions[tj.index()].locked);
    assert_eq!(blood(&e, tj), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 10], "canceled: no pool moved");

    let mut e = game(2);
    let gerald = e.add_minion(SeatId(0), find_card("Gerald Windham"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let tj = e.add_minion(SeatId(1), find_card("T.J."), 3, false);
    e.add_minion(SeatId(1), find_card("Tara"), 4, false);
    kine_called(&mut e, gerald);
    to_window(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    let r = e.referendum().expect("referendum");
    // Gerald: 1 vote for Tara, ready and titled, no title himself.
    assert_eq!(r.votes_of(&e.state, gerald), 1);
    assert_eq!(e.state.title_of(gerald), Title::None);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        host_ability(tj, 0, Target::None),
    );
}

// Fakir al Sidi [ANK 20220705] [LSJ 20100705] [LSJ 20011022] [LSJ
// 20100902-2] [ANK 20180307-2] "The ability can be used while locked or in
// torpor, only once per referendum": locked, he burns 3 blood before any
// vote to have the referendum fail.
#[test]
fn fakir_al_sidi_fails_a_referendum_while_locked() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let fakir = e.add_minion(SeatId(1), find_card("Fakir al Sidi"), 7, false);
    e.state.minions[fakir.index()].locked = true;
    kine_called(&mut e, caller);
    pick_when(&mut e, host_ability(fakir, 0, Target::None));
    assert_eq!(blood(&e, fakir), 4);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 10], "failed: no pool moved");
}

// Karl Schrekt [LSJ 20050128] "When considered a Justicar, he is also
// implicitly considered Camarilla", [LSJ 20101013] [ANK 20190228] [ANK
// 20200705-2] [LSJ 20070708] [RTR 20070707] "treated as such for all the
// effects of the card": Karl calls Parity Shift ("requires a prince or
// justicar") — made Anarch too, a justicar implying the Camarilla; [ANK
// 20231221] "When meeting a title requirement for a political action, the
// vampire does not get the title's votes": he casts his own 2, not a
// justicar's 3.
#[test]
fn karl_schrekt_plays_justicar_cards_with_his_own_votes() {
    let mut e = game(2);
    let karl = e.add_minion(SeatId(0), find_card("Karl Schrekt"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1].pool = 20;
    hand(&mut e, 0, &["Parity Shift"]);
    let shift = Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(karl),
        target: Target::None,
    };
    let req = request(&mut e);
    assert!(req.options.contains(&shift), "{:?}", req.options);
    pick(&mut e, shift);
    to_window(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, karl), 2);

    let mut e = game(2);
    let karl = e.add_minion(SeatId(0), find_card("Karl Schrekt"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[karl.index()].sect_override = Some(Sect::Anarch);
    e.state.seats[1].pool = 20;
    hand(&mut e, 0, &["Parity Shift"]);
    let req = request(&mut e);
    assert!(req.options.contains(&shift), "{:?}", req.options);
}

// Karl Schrekt [ANK 20230503] [LSJ 20050124] "Can meet a sect-specific
// title (e.g. Justicar) requirement even if the sect does not exist
// anymore (eg. Fall of the Camarilla)": with Fall of the Camarilla in
// play, he still calls Parity Shift.
#[test]
fn karl_schrekt_meets_a_justicar_requirement_after_the_fall() {
    let mut e = game(2);
    let karl = e.add_minion(SeatId(0), find_card("Karl Schrekt"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1].pool = 20;
    in_play(&mut e, 0, "Fall of the Camarilla", None);
    assert_eq!(
        e.state.sect_of(karl),
        Some(Sect::Independent),
        "no Camarilla"
    );
    hand(&mut e, 0, &["Parity Shift"]);
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(karl),
        target: Target::None,
    }));
}

// Karl Schrekt [LSJ 20010714] "Keeps their title and votes if they changes
// sects"; Randall [LSJ 20010714] alike: made Anarch, Karl keeps his 2 votes
// and Randall his 1. Camarilla's Iron Fist ("requires a titled Camarilla
// vampire"): Karl, met as a justicar, is a titled Camarilla vampire for
// the card (owner, 2026-10-09: rules-qa 59.14, over ANK 20250410) — he
// calls it, as Tara does.
#[test]
fn personal_vote_titles_survive_a_sect_change() {
    // Four seats: Iron Fist's terms need two Methuselahs besides the
    // chosen one.
    let mut e = game(4);
    let tara = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    let karl = e.add_minion(SeatId(0), find_card("Karl Schrekt"), 6, false);
    for s in 1..4 {
        e.add_minion(SeatId(s), plain("Ventrue", s as usize), 4, false);
    }
    e.state.minions[karl.index()].sect_override = Some(Sect::Anarch);
    hand(&mut e, 0, &["Camarilla's Iron Fist"]);
    let req = request(&mut e);
    let fist_by = |m: MinionId| {
        req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { actor: Some(a), .. } if *a == m))
    };
    assert!(fist_by(tara) && fist_by(karl));
    // Called by Karl, it reaches its referendum.
    let fist = req
        .options
        .iter()
        .copied()
        .find(|o| matches!(o, Choice::Play { actor: Some(a), .. } if *a == karl))
        .expect("Karl's Iron Fist");
    pick(&mut e, fist);
    to_referendum(&mut e);
    assert!(e.referendum().is_some());

    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    let karl = e.add_minion(SeatId(0), find_card("Karl Schrekt"), 6, false);
    let randall = e.add_minion(SeatId(0), find_card("Randall"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for m in [karl, randall] {
        e.state.minions[m.index()].sect_override = Some(Sect::Anarch);
    }
    kine_called(&mut e, caller);
    to_window(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, karl), 2);
    assert_eq!(r.votes_of(&e.state, randall), 1);
}

// Giovanni del Georgio [ANK 20180202] [LSJ 20010627] "His special is an
// undirected action", and [RBK action-card-or-card-in-play] "A minion can
// only perform this special hunt once per turn": two Bum's Rush in his prey's ash heap removed from the
// game for a 3-blood hunt; the action not offered again that turn.
#[test]
fn giovanni_del_georgio_hunts_on_a_pair_in_his_preys_ash_heap() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Giovanni del Georgio"), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let rush = find_library("Bum's Rush");
    e.state.seats[1].ash.extend([rush, rush, rush, rush]);
    let hunt = printed(g);
    let req = request(&mut e);
    assert!(req.options.contains(&hunt), "{:?}", req.options);
    // The one pair there: named at once.
    pick(&mut e, hunt);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, g), 2 + 3);
    assert_eq!(e.state.seats[1].ash.len(), 2);
    assert_eq!(e.state.seats[1].removed, vec![rush, rush]);
    e.state.minions[g.index()].locked = false;
    let req = request(&mut e);
    assert!(!req.options.contains(&hunt));
    // No pair in the prey's ash heap: not offered (rules-qa 17.4).
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Giovanni del Georgio"), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1].ash.extend([rush, find_library("Trap")]);
    let req = request(&mut e);
    assert!(!req.options.contains(&printed(g)));
}

// Mary Anne Blaire: "can add 2 blood to a vampire in your uncontrolled
// region as a +1 stealth action. While she is ready, Inner Circle members
// get -2 bleed": the blood added; Stanislava (+2 bleed) bleeds for 1, any
// Methuselah's, and for 3 again with Mary Anne in torpor.
#[test]
fn mary_anne_blaire_feeds_the_uncontrolled_and_weakens_the_inner_circle() {
    let mut e = game(2);
    let mary = e.add_minion(SeatId(0), find_card("Mary Anne Blaire"), 4, false);
    let stanislava = e.add_minion(SeatId(1), find_card("Stanislava"), 4, false);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: plain("Ventrue", 0),
            counters: 1,
            banished: None,
        });
    assert_eq!(e.state.current_bleed(stanislava), 1);
    let act = Choice::UseAbility {
        source: AbilitySource::Vampire(mary),
        ability: 0,
        actor: Some(mary),
        target: Target::Uncontrolled(0),
    };
    pick_when(&mut e, act);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 3);
    e.state.minions[mary.index()].in_torpor = true;
    assert_eq!(e.state.current_bleed(stanislava), 3);
}

// Vampiric Disease [ANK 20180702] "At the beginning of the unlock phase, an
// afflicted vampires unlocks and burns a blood", [ANK 20170227] [LSJ
// 20020408] "Unlocking an already unlocked vampire does not trigger the
// effect": of seat 1's two diseased vampires, the locked one burns 1 blood
// as it unlocks, the unlocked one none; at 0 blood, the counter burns.
#[test]
fn vampiric_disease_burns_blood_as_its_vampire_unlocks() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let locked = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let unlocked = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let dry = e.add_minion(SeatId(1), plain("Ventrue", 2), 1, false);
    for m in [locked, unlocked, dry] {
        e.state.minions[m.index()].disease = true;
    }
    e.state.minions[locked.index()].locked = true;
    e.state.minions[dry.index()].locked = true;
    e.state.minions[dry.index()].blood = 0;
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(blood(&e, locked), 3);
    assert_eq!(blood(&e, unlocked), 4);
    assert!(!e.state.minions[dry.index()].disease, "the counter burned");
    assert!(e.state.minions[locked.index()].disease);
}

// Vampiric Disease: played on a vampire, "a vampire with a disease counter
// cannot gain blood by hunting" — [LSJ 20090411] (Abactor) "a vampire that
// cannot gain blood has no successful resolution of a hunt" — and [ANK
// 20221210-2] "Any unlock, even one costing blood already, triggers the
// burning blood effect (eg. Freak Drive)": the diseased vampire hunts for
// nothing, then Freak Drive's unlock burns 1 blood.
#[test]
fn vampiric_disease_starves_the_hunt_and_burns_on_any_unlock() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), vampire_with("for", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Vampiric Disease", "Freak Drive"]);
    let req = to_window(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let disease = Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(v),
    };
    assert!(req.options.contains(&disease), "{:?}", req.options);
    pick(&mut e, disease);
    request(&mut e);
    assert!(e.state.minions[v.index()].disease);
    to_minion_phase(&mut e, SeatId(0));
    pick(&mut e, action(v, zvtes::decision::ActionKind::Hunt));
    pick_when(&mut e, play(0, 0, v));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // No blood hunted (1 for Freak Drive), 1 burned as he unlocked.
    assert_eq!(blood(&e, v), 3 - 1 - 1);
    assert!(!e.state.minions[v.index()].locked);
}

// Vampiric Disease [ANK 20210422] [LSJ 20040830] "The counter is placed
// before strikes are chosen (i.e. after range is determined, in the same
// step as Immortal Grapple)": the blocker of a diseased vampire has the
// counter as the before-strikes window opens, at close range.
#[test]
fn vampiric_disease_spreads_before_strikes_are_chosen() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("pot", 0), 4, false);
    e.state.minions[a.index()].disease = true;
    hand(&mut e, 1, &["Immortal Grapple"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    // Seat 1's Immortal Grapple window: the counter is already there.
    until(&mut e, SeatId(1), WindowLabel::CombatBeforeStrikes);
    assert!(e.state.minions[z.index()].disease);
}

// Vampiric Disease, Banishment [LSJ 20060213] [LSJ 20060908] "This is not
// unlocking (ie. for Anima Gathering, Vampiric Disease)": a diseased
// vampire banished comes back with its counter and its blood, no blood
// burned as it returns unlocked.
#[test]
fn a_banished_diseased_vampire_returns_without_unlocking() {
    let mut e = game(2);
    let tara = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    // Younger than Tara: Banishment's target.
    let z = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[z.index()].disease = true;
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 0, &["Banishment"]);
    pick(&mut e, play(0, 0, tara));
    pick(&mut e, Choice::ChooseMinion(z));
    pick_when(&mut e, votes(tara, true));
    close_polling(&mut e);
    assert!(!e.state.minions[z.index()].in_play);
    let cap = CRYPT[plain("Ventrue", 1) as usize].capacity;
    e.state.seats[1].uncontrolled[0].counters = cap;
    let req = to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    assert!(req.options.contains(&Choice::InfluenceOut { slot: 0 }));
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    request(&mut e);
    let v = &e.state.minions[z.index()];
    assert!(v.in_play && !v.locked && v.disease);
    assert_eq!(v.blood, cap);
}

// Justicar Retribution [RTR 19980623] "Counts the bleed amount that each
// vampire would have when attempting to bleed his prey": passed, it burns
// Stanislava (+2 bleed: 3) and leaves the 1-bleed vampires, the caller's
// own included.
#[test]
fn justicar_retribution_burns_the_big_bleeders() {
    let mut e = game(2);
    let tara = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    // In torpor, she still burns (owner, 2026-10-09: rules-qa 59.9).
    let stanislava = e.add_minion(SeatId(1), find_card("Stanislava"), 4, true);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 0, &["Justicar Retribution"]);
    pick(&mut e, play(0, 0, tara));
    pick_when(&mut e, votes(tara, true));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[stanislava.index()].in_play);
    assert!([tara, b, z]
        .iter()
        .all(|m| e.state.minions[m.index()].in_play));
}

// Randall [LSJ 20040812-2] [PIB 20130327] "Cannot be used to prevent
// aggravated damage": out of the combat, he burns 1 blood to prevent 1 of
// the hand strike on his Methuselah's Brujah; not the aggravated one Wolf
// Claws makes.
#[test]
fn randall_prevents_only_non_aggravated_damage() {
    for claws in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let randall = e.add_minion(SeatId(0), find_card("Randall"), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with("pro", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if claws {
            hand(&mut e, 1, &["Wolf Claws"]);
        }
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        if claws {
            pick_when(&mut e, play(0, 0, z));
        }
        let prevent = Choice::UseAbility {
            source: AbilitySource::Vampire(randall),
            ability: 0,
            actor: None,
            target: Target::Minion(a),
        };
        if claws {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, prevent);
        } else {
            pick_when(&mut e, prevent);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, randall), 3);
            assert_eq!(blood(&e, a), 4);
        }
    }
}

// Lodin: "Once each combat, Lodin can prevent 1 damage from an opposing
// minion's strike": blocked, he takes none of the blocker's hand strike.
#[test]
fn lodin_prevents_a_point_of_strike_damage() {
    let mut e = game(2);
    let lodin = e.add_minion(SeatId(0), find_card("Lodin (Olaf Holte)"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(lodin));
    pick_when(&mut e, block(z));
    pick_when(&mut e, host_ability(lodin, 0, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, lodin), 4);
}

// Trap [ANK 20200525-1] "The provided press is applied before any other
// press": played before range, it carries the combat on with no combatant
// pressing — a counter a round, from the 3 it is played with: four rounds
// of hand strikes, then it burns as the combat ends.
#[test]
fn trap_presses_to_continue_by_itself() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    hand(&mut e, 0, &["Trap"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Four rounds of the acting vampire's hand strikes.
    assert_eq!(blood(&e, z), 6 - 4);
    assert_eq!(e.state.seats[0].ash, vec![find_library("Trap")]);

    // A press only usable to continue (Conquer the Beast [ANI]) is not
    // usable while the Trap's stands — offered without it.
    for trap in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("ANI", 0), 8, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
        let cards: &[&str] = if trap {
            &["Trap", "Conquer the Beast"]
        } else {
            &["Conquer the Beast"]
        };
        hand(&mut e, 0, cards);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        if trap {
            pick_when(&mut e, play(0, 0, a));
        }
        pick_when(&mut e, play(0, 1, a));
        let press = offered_before(
            &mut e,
            Choice::UsePress,
            SeatId(0),
            WindowLabel::MinionPhase,
        );
        assert_eq!(press, !trap);
    }
}

// Conquer the Beast [ani]: "this card automatically burns 1 of those
// counters to give this vampire 1 mandatory press to continue combat",
// and "this vampire cannot use equipment this combat": four rounds, the
// vampire's .44 Magnum never offered.
#[test]
fn conquer_the_beast_forces_its_vampire_to_press_on() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ani", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    let gun = in_play(&mut e, 0, ".44 Magnum", Some(a));
    hand(&mut e, 0, &["Conquer the Beast"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_at(gun, 0, None, Target::None),
    );
    assert_eq!(blood(&e, z), 6 - 4);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Conquer the Beast")));
}

// Ephor [LSJ 20080604] "The action provided by this card is considered to
// be an action requiring the listed clan/sect/capacity/title": on Lodin,
// contesting Prince of Chicago with Kevin Jackson, the action makes Kevin
// yield; the card on a non-Ventrue grants none.
#[test]
fn ephor_forces_a_contested_title_to_be_yielded() {
    let mut e = game(2);
    let lodin = e.add_minion(SeatId(0), find_card("Lodin (Olaf Holte)"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let kevin = e.add_minion(SeatId(1), find_card("Kevin Jackson"), 4, false);
    for m in [lodin, kevin] {
        e.state.minions[m.index()].title_contested = true;
    }
    let ephor = in_play(&mut e, 0, "Ephor", Some(lodin));
    let yield_it = use_at(ephor, 0, Some(lodin), Target::Minion(kevin));
    let req = request(&mut e);
    assert!(req.options.contains(&yield_it), "{:?}", req.options);
    pick(&mut e, yield_it);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let k = &e.state.minions[kevin.index()];
    assert!(k.title_lost && !k.title_contested);

    let mut e = game(2);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let kevin = e.add_minion(SeatId(1), find_card("Kevin Jackson"), 4, false);
    e.state.minions[kevin.index()].title_contested = true;
    let ephor = in_play(&mut e, 0, "Ephor", Some(b));
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&use_at(ephor, 0, Some(b), Target::Minion(kevin))));
}

// Society of Leopold: "During his or her controller's unlock phase, the
// vampire with this card either burns 1 blood or is burned (the
// controller's choice)": seat 1 burns the blood; with none left, the
// vampire burns.
#[test]
fn society_of_leopold_bleeds_or_burns_its_vampire() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 0, "Society of Leopold", Some(z));
    let req = to_window(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Pass, Choice::BurnBloodToAvoid]);
    pick(&mut e, Choice::BurnBloodToAvoid);
    assert_eq!(blood(&e, z), 0);
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(0));
    to_minion_phase(&mut e, SeatId(1));
    assert!(!e.state.minions[z.index()].in_play, "burned, no blood left");
}

// Matasuntha: "After a minion in combat with Matasuntha leaves the ready
// region, you can gain 1 pool": her blocker, sent to torpor, gains her
// Methuselah a pool.
#[test]
fn matasuntha_gains_a_pool_as_her_opponent_falls() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Matasuntha"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(z));
    pick_when(&mut e, Choice::ResolvePrinted { minion: m });
    assert!(e.state.minions[z.index()].in_torpor);
    assert_eq!(pool(&e)[0], 11);
}

// The Signet of King Saul: "Vampires with capacity 8 or more cannot attempt
// to block the bearer": of seat 1's vampires, only the small one may.
#[test]
fn the_signet_of_king_saul_bars_big_blockers() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let small_card = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            (4..8).contains(&c.capacity)
                && c.title == Title::None
                && cards::vampire(i as u16).is_none()
        })
        .expect("a small plain vampire") as u16;
    let small = e.add_minion(SeatId(1), small_card, 4, false);
    let big = e.add_minion(SeatId(1), find_card("Stanislava"), 4, false);
    in_play(&mut e, 0, "Signet of King Saul, The", Some(a));
    pick(&mut e, bleed(a));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(small)));
    assert!(!req.options.contains(&block(big)));
}

// Inyanga: "Allies requiring a Gangrel cost Inyanga 1 fewer blood or pool":
// Double Deuce (2 pool) costs her Methuselah 1.
#[test]
fn inyanga_recruits_gangrel_allies_for_less() {
    let mut e = game(2);
    let inyanga = e.add_minion(SeatId(0), find_card("Inyanga"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Double Deuce"]);
    pick(&mut e, play(0, 0, inyanga));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[0], 9);
}

// Auspex (Master: Discipline): "This vampire gains 1 level of Auspex.
// Capacity increases by 1": on a vampire without Auspex.
#[test]
fn the_auspex_master_adds_a_level_and_capacity() {
    let mut e = game(2);
    let card = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.capacity >= 4
                && !c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("aus"))
                && cards::vampire(i as u16).is_none()
        })
        .expect("a vampire without Auspex") as u16;
    let v = e.add_minion(SeatId(0), card, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Auspex"]);
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(v),
        },
    );
    request(&mut e);
    assert!(e.state.has_discipline(v, "aus"));
    assert_eq!(
        e.state.capacity_of(v),
        Some(CRYPT[card as usize].capacity + 1)
    );
}

// Victor Donaldson: "If a card named Golconda: Inner Peace you play is
// canceled, you can move that card from your ash heap to your library
// (shuffle afterward)": seat 1 burns 2 pool to cancel it; Victor's
// Methuselah shuffles it back.
#[test]
fn victor_donaldson_shuffles_a_canceled_golconda_back() {
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("Victor Donaldson"), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Stanislava"), 4, false);
    hand(&mut e, 0, &["Golconda: Inner Peace"]);
    fill_hand(&mut e, 0);
    // Its replacement drawn: the hand is full as the card is shuffled back.
    e.state.seats[0].library = vec![find_library("Form of Mist")];
    e.at_master_phase();
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(z),
        },
    );
    pick_when(&mut e, Choice::PayToCancel);
    let req = to_window(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    let golconda = find_library("Golconda: Inner Peace");
    assert_eq!(e.state.seats[0].ash, vec![golconda]);
    pick(&mut e, req.options[1]);
    request(&mut e);
    assert!(e.state.seats[0].ash.is_empty());
    assert!(e.state.seats[0].library.contains(&golconda));
    assert!(e.state.minions[z.index()].in_play);
}

// Quick Meld [PRO]: "As above, and once this round, this vampire may burn
// 1 blood for an additional maneuver": the blocker maneuvers back; the
// acting vampire burns 1 blood to maneuver again.
#[test]
fn quick_meld_buys_an_extra_maneuver_for_a_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("PRO", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("pro", 0), 6, false);
    hand(&mut e, 0, &["Quick Meld"]);
    hand(&mut e, 1, &["Quick Meld"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, z));
    let before = blood(&e, a);
    let req = to_window(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(
        req.options.contains(&Choice::UseManeuver),
        "{:?}",
        req.options
    );
    pick(&mut e, Choice::UseManeuver);
    assert_eq!(blood(&e, a), before - 1);
}

// Society of Leopold: "The vampire with this card can move it to any other
// vampire as a +1 stealth Ⓓ action": seat 0's vampire moves it onto seat
// 1's, whose unlock phase then asks.
#[test]
fn society_of_leopold_moves_to_another_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let leopold = in_play(&mut e, 0, "Society of Leopold", Some(a));
    pick(&mut e, use_at(leopold, 0, Some(a), Target::Minion(z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[leopold.index()].host, Some(z));
    let req = to_window(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Pass, Choice::BurnBloodToAvoid]);
}

// Ephor: "This Ventrue gets +1 intercept and +1 vote": Lodin, Prince of
// Chicago (2 votes), casts 3.
#[test]
fn ephor_gives_its_ventrue_a_vote() {
    let mut e = game(2);
    let lodin = e.add_minion(SeatId(0), find_card("Lodin (Olaf Holte)"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Ephor", Some(lodin));
    kine_called(&mut e, lodin);
    to_window(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, lodin), 3);
}

// Karl Schrekt's justicar reading stays the cards' (G00073 "not treated as
// meeting the requirements for effects the card has from being in play"):
// no ability or rider of a card in play requires a justicar, so reading it
// in `holds` reaches card plays only — this guards that.
#[test]
fn no_in_play_effect_requires_a_justicar() {
    use zvtes::cards::{Requirement, TitleClass};
    let justicar = |r: &Requirement| matches!(r, Requirement::ActorTitleIn(cs) if cs.contains(&TitleClass::Justicar));
    for def in zvtes::cards::defs::DEFS.iter() {
        for m in def.abilities {
            assert!(!m.requires.iter().any(justicar), "{}", def.name);
        }
        for r in def.riders {
            assert!(!r.requires.iter().any(justicar), "{}", def.name);
        }
    }
}

// ---- Slice 16m: steps 16-20 ---------------------------------------------------------------

/// A crypt card by name and group (Theo Bell: G2 and G6).
fn crypt_of(name: &str, group: u8) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && c.group == group)
        .expect("a crypt card") as u16
}

fn seat_library(e: &mut Engine, seat: usize, names: &[&str]) {
    e.state.seats[seat].library = names.iter().map(|n| find_library(n)).collect();
}

// Theo Bell (G2): "If you control a ready prince or justicar, blood hunts
// cannot be called on Theo": with Tara (a prince) ready, barred; Tara in
// torpor, not. The G6 card keeps its own text.
#[test]
fn theo_bell_g2_is_shielded_by_a_ready_prince() {
    let mut e = game(2);
    let theo = e.add_minion(SeatId(0), crypt_of("Theo Bell", 2), 4, false);
    let tara = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert!(e.state.blood_hunt_barred(theo));
    e.state.minions[tara.index()].in_torpor = true;
    assert!(!e.state.blood_hunt_barred(theo));
    let g6 = e.add_minion(SeatId(1), crypt_of("Theo Bell", 6), 4, false);
    assert_eq!(
        zvtes::cards::vampire_of(&e.state.minions[g6.index()]).map(|d| d.group),
        Some(Some(6))
    );
}

// Juliet Parr: "After a referendum called by Juliet passes, you get +2 hand
// size until your next unlock phase": 9 after her Kine Resources
// Contested passes, 7 again as seat 0's next turn begins.
#[test]
fn juliet_parr_widens_the_hand_after_her_referendum() {
    let mut e = game(2);
    let juliet = e.add_minion(SeatId(0), find_card("Juliet Parr"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    kine_called(&mut e, juliet);
    pick_when(&mut e, votes(juliet, true));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.hand_size(SeatId(0)), 9);
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(
        e.state.hand_size(SeatId(0)),
        9,
        "until her next unlock phase"
    );
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
}

// Mistress Fanchon: "can search your library for a minion card, reveal it,
// and move it to your hand as a +1 stealth action": any card but a master
// or an event (owner, 2026-10-09: rules-qa 60.3).
#[test]
fn mistress_fanchon_searches_for_a_minion_card() {
    let mut e = game(2);
    let f = e.add_minion(SeatId(0), find_card("Mistress Fanchon"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    seat_library(&mut e, 0, &["Double Deuce", "Kevlar Vest", "Auspex"]);
    fill_hand(&mut e, 0);
    pick(&mut e, printed(f));
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    let deuce = find_library("Double Deuce");
    let kevlar = find_library("Kevlar Vest");
    let mut want = vec![Choice::FindCard(deuce), Choice::FindCard(kevlar)];
    want.sort_by_key(|c| match c {
        Choice::FindCard(x) => *x,
        _ => 0,
    });
    want.insert(0, Choice::Pass);
    assert_eq!(req.options, want, "the master excluded");
    pick(&mut e, Choice::FindCard(deuce));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].hand.contains(&deuce));
}

// The Beast: "cannot play action cards. He cannot have or use equipment or
// retainers" — [LSJ 20050315] [LSJ 20010210] no equip action — and "may
// enter combat with any ready minion controlled by another Methuselah".
#[test]
fn the_beast_plays_no_action_card_nor_equips() {
    let mut e = game(2);
    let beast = e.add_minion(
        SeatId(0),
        find_card("Beast, The Leatherface of Detroit"),
        4,
        false,
    );
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Bum's Rush", "Kevlar Vest"]);
    let req = request(&mut e);
    let by = |m: MinionId| {
        req.options
            .iter()
            .filter(|o| matches!(o, Choice::Play { actor: Some(a), .. } if *a == m))
            .count()
    };
    assert_eq!(by(beast), 0);
    assert!(by(other) >= 2);
    let rush = Choice::UseAbility {
        source: AbilitySource::Vampire(beast),
        ability: 0,
        actor: Some(beast),
        target: Target::Minion(z),
    };
    assert!(req.options.contains(&rush), "{:?}", req.options);
}

// Lord Tremere: "During your minion phase, you can lock or unlock another
// ready Tremere or Tremere antitribu" — once in it (rules-qa 58.7, 60.6):
// Mistress Fanchon locked, no second use that phase nor after it.
#[test]
fn lord_tremere_locks_and_unlocks_another_tremere() {
    let mut e = game(2);
    let lord = e.add_minion(SeatId(0), find_card("Lord Tremere"), 4, false);
    let f = e.add_minion(SeatId(0), find_card("Mistress Fanchon"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let toggle = host_ability(lord, 0, Target::Minion(f));
    pick(&mut e, toggle);
    assert!(e.state.minions[f.index()].locked);
    let req = request(&mut e);
    assert!(!req.options.contains(&toggle), "once each minion phase");
    // Nor later in the turn, out of the minion phase.
    pick(&mut e, Choice::Pass);
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, toggle);
}

// Nergal [ANK 20180719-4] [LSJ 20091021-2]: "Once each turn, Nergal can
// reduce the cost of a card he plays by 1 blood" — his choice of card
// (owner, 2026-10-09: rules-qa 60.7): taken on his first Aire of Elation,
// which costs nothing; the second that turn costs its 1 blood, no offer.
#[test]
fn nergal_takes_a_blood_off_once_each_turn() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nergal"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Aire of Elation", "Aire of Elation"]);
    pick(&mut e, bleed(n));
    pick_when(&mut e, play(0, 0, n));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    let take = Choice::ResolvePrinted { minion: n };
    assert!(req.options.contains(&take) && req.options.contains(&Choice::Pass));
    pick(&mut e, take);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, n), 6);
    e.state.minions[n.index()].locked = false;
    e.state.minions[n.index()].bled_this_turn = false;
    pick(&mut e, bleed(n));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, take);
}

// The Capuchin [ANK 20200417]: "If The Capuchin is burned, move him to his
// owner's uncontrolled region and you get +3 transfers at the start of
// your next influence phase": burned, he is in seat 0's uncontrolled
// region, a new card with no counter, and seat 0 has 3 more transfers.
#[test]
fn the_capuchin_returns_to_the_uncontrolled_region() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Capuchin, The"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.burn(c);
    let slot = &e.state.seats[0].uncontrolled[0];
    assert_eq!((slot.card, slot.counters), (find_card("Capuchin, The"), 0));
    assert!(!e.state.seats[0]
        .ash_crypt
        .contains(&find_card("Capuchin, The")));
    assert_eq!(e.state.seats[0].transfers_next, 3);
    to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    assert_eq!(e.state.seats[0].transfers_next, 0);
}

// Jimmy Dunn [RTR 20180303] "Can be influenced out if you already control
// one": the second comes out and the first burns, no contest.
#[test]
fn jimmy_dunn_burns_the_first_instead_of_contesting() {
    let mut e = game(2);
    let jimmy = find_card("Jimmy Dunn");
    let first = e.add_minion(SeatId(0), jimmy, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: jimmy,
            counters: 4,
            banished: None,
        });
    let req = to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    assert!(req.options.contains(&Choice::InfluenceOut { slot: 0 }));
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    request(&mut e);
    assert!(!e.state.minions[first.index()].in_play);
    let second = e
        .state
        .minions
        .iter()
        .filter(|m| m.in_play && m.name() == "Jimmy Dunn");
    assert!(second.clone().count() == 1 && second.clone().all(|m| !m.contested));
}

// Malgorzata: "may remove a vampire in your uncontrolled region from the
// game to add 4 blood from the blood bank to another vampire in your
// uncontrolled region as a +1 stealth action": both named as she acts.
#[test]
fn malgorzata_feeds_one_uncontrolled_vampire_with_another() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Malgorzata"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for card in [plain("Ventrue", 1), plain("Brujah", 0)] {
        e.state.seats[0]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card,
                counters: 1,
                banished: None,
            });
    }
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(m),
            ability: 0,
            actor: Some(m),
            target: Target::Uncontrolled(1),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let u = &e.state.seats[0].uncontrolled;
    assert_eq!(u.len(), 1);
    assert_eq!((u[0].card, u[0].counters), (plain("Brujah", 0), 5));
    assert_eq!(e.state.seats[0].removed_crypt, vec![plain("Ventrue", 1)]);
}

// Street Cred [pot]: "Only usable at the end of a round of combat in which
// this vampire successfully inflicted more damage than the opposing minion.
// Move 1 blood ... to a younger vampire in your uncontrolled region": the
// blocker's .44 Magnum (2) beats the hand strike (1); it feeds a vampire
// younger than itself — not than the acting one; [RTR 20030519] after the
// presses.
#[test]
fn street_cred_feeds_a_younger_uncontrolled_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("POT", 0), 6, false);
    let cap = CRYPT[e.state.minions[z.index()].crypt_card().unwrap() as usize].capacity;
    let young = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| c.capacity < cap && cards::vampire(i as u16).is_none())
        .expect("a younger vampire") as u16;
    e.state.seats[1]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: young,
            counters: 1,
            banished: None,
        });
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 1, &["Street Cred"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, use_at(gun, 0, None, Target::None));
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(z),
            target: Target::Uncontrolled(0),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].uncontrolled[0].counters, 2);
}

// Erebus Mask: "The Harbinger of Skulls with this equipment gets +1
// stealth": The Capuchin's bleed is a +1 stealth action.
#[test]
fn the_erebus_mask_gives_its_harbinger_stealth() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Capuchin, The"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Erebus Mask", Some(c));
    pick(&mut e, bleed(c));
    assert_eq!(e.action().expect("action").stealth(&e.state), 1);
}

// Under My Skin [THN]: "+1 stealth, and put this card on this vampire.
// During any action after this one, this vampire can burn this card to get
// +1 stealth ... for +2 stealth this action": The Capuchin keeps it after
// his first bleed (unblocked by Nadima's 1 intercept) and burns it for the
// stealth in his next one; [LSJ 20011204-3] [PIB 20130704] played at
// [THN], Obfuscate is not needed.
#[test]
fn under_my_skin_stays_for_a_later_action() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Capuchin, The"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let n = e.add_minion(SeatId(1), find_card("Nadima"), 4, false);
    hand(&mut e, 0, &["Under My Skin"]);
    pick(&mut e, bleed(c));
    pick_when(&mut e, block(n));
    pick_when(&mut e, play(0, 2, c));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let skin = card_named(&e, "Under My Skin");
    assert_eq!(e.state.cards[skin.index()].host, Some(c));
    e.state.minions[c.index()].locked = false;
    e.state.minions[c.index()].bled_this_turn = false;
    pick(&mut e, bleed(c));
    pick_when(&mut e, block(n));
    pick_when(&mut e, use_at(skin, 0, None, Target::None));
    request(&mut e);
    assert!(!e.state.cards[skin.index()].in_play);
}

// Crocodile's Tongue [aus]: "Only usable if an ally or younger vampire
// attempts to block. That attempt fails and the blocking minion's
// controller cannot attempt to block this action again. The blocking
// minion can burn 1 blood or life to cancel this card as it is played":
// declined, neither of seat 1's vampires may block; paid, the block goes on.
#[test]
fn crocodiles_tongue_bars_the_blockers_methuselah() {
    for pay in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Mistress Fanchon"), 6, false);
        let z1 = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let z2 = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Crocodile's Tongue"]);
        pick(&mut e, bleed(a));
        pick(&mut e, block(z1));
        pick_when(&mut e, play(0, 0, a));
        let req = to_window(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
        assert!(
            req.options.contains(&Choice::PayToCancel),
            "{:?}",
            req.options
        );
        if pay {
            pick(&mut e, Choice::PayToCancel);
            request(&mut e);
            assert_eq!(blood(&e, z1), 3);
            assert!(
                matches!(e.action().expect("action").state, zvtes::actions::ActionState::Attempt(m) if m == z1)
            );
        } else {
            pick(&mut e, Choice::Pass);
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(z2));
            assert_eq!(pool(&e)[1], 10 - 3, "unblocked: Fanchon's +2 bleed");
        }
    }
}

// Condemnation: Mute [DAI]: "Requires an infernal vampire. Put this card on
// a ready vampire. The attached vampire gets -3 votes. ... the attached
// vampire burns 1 blood after a referendum passes": Nergal puts it on
// Lodin (a prince, 2 votes): none left to cast; Tara's referendum passed
// costs him a blood. [PIB 20110802] "Only affects votes", and [ANK
// 20251125] [LSJ 20050221-2] net negative votes count as none.
#[test]
fn condemnation_mute_silences_and_bleeds_its_vampire() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nergal"), 6, false);
    let b = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    let tara = e.add_minion(SeatId(1), find_card("Lodin (Olaf Holte)"), 4, false);
    hand(&mut e, 0, &["Condemnation: Mute"]);
    let mute = Choice::Play {
        card: 0,
        mode: 2,
        actor: Some(n),
        target: Target::Minion(tara),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&mute), "{:?}", req.options);
    // Not by a vampire that is not infernal (Tara).
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::Play { mode: 1 | 2, actor: Some(a), .. } if *a == b)));
    pick(&mut e, mute);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.cards[card_named(&e, "Condemnation: Mute").index()].host,
        Some(tara)
    );
    kine_called(&mut e, b);
    to_window(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, tara), 0);
    pick_when(&mut e, votes(b, true));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, tara), 3);
}

// Condemnation: Mute [chi]: "Lock a younger vampire": Danielle Diron locks
// one — [LSJ 20011204-3] [PIB 20130704] played with Chimerstry, it requires
// Chimerstry only.
#[test]
fn condemnation_mute_chi_locks_a_younger_vampire() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Danielle Diron"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let cap = CRYPT[find_card("Danielle Diron") as usize].capacity;
    let young = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.capacity < cap && c.capacity >= 3 && cards::vampire(i as u16).is_none()
        })
        .expect("a younger vampire") as u16;
    let z = e.add_minion(SeatId(1), young, 3, false);
    hand(&mut e, 0, &["Condemnation: Mute"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(d),
            target: Target::Minion(z),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].locked);
}

// I am Legion [DAI][OBF] [LSJ 20100902-2]: "Can be used to gain a pool
// after a successful action, without cancelling an [aus] card": Nergal's
// bleed for 3 and a pool after it.
#[test]
fn i_am_legion_gains_a_pool_after_a_successful_action() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nergal"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["I am Legion"]);
    pick(&mut e, bleed(n));
    pick_when(&mut e, play(0, 3, n));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![11, 10 - 3]);
}

// My Kin Against the World: "Choose X younger vampires of the same clan as
// this acting vampire. If this referendum passes, unlock this acting
// vampire and the chosen vampires": X = 1, a locked younger clanmate
// chosen; both unlock.
#[test]
fn my_kin_against_the_world_unlocks_the_clan() {
    let mut e = game(2);
    let elder = find_card("Mistress Fanchon");
    let a = e.add_minion(SeatId(0), elder, 4, false);
    let f = e.add_minion(SeatId(0), find_card("Gerald Windham"), 4, false);
    e.state.minions[f.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["My Kin Against the World"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, set_x(1));
    // The one younger Tremere: chosen at once (a single option).
    pick_when(&mut e, votes(a, true));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked);
    assert!(!e.state.minions[f.index()].locked);
    assert_eq!(blood(&e, a), 3, "X = 1 blood");
}

// The Sargon Fragment [LSJ 20010627] "the card to retrieve must be named
// when declaring the action", [LSJ 20050228] "any copy of the named card in
// the ash heap can be fetched", and "This vampire gets superior
// Necromancy": the card named, it comes back for 1 blood.
#[test]
fn the_sargon_fragment_takes_back_a_named_card() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let frag = in_play(&mut e, 0, "Sargon Fragment, The", Some(v));
    assert!(e.state.has_discipline(v, "NEC"));
    let rush = find_library("Bum's Rush");
    e.state.seats[0].ash = vec![rush, find_library("Trap")];
    pick(&mut e, use_at(frag, 0, Some(v), Target::None));
    pick_when(&mut e, Choice::FindCard(rush));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].hand.contains(&rush));
    assert_eq!(blood(&e, v), 3);
    // Nothing in the ash heap: not offered.
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let frag = in_play(&mut e, 0, "Sargon Fragment, The", Some(v));
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&use_at(frag, 0, Some(v), Target::None)));
}

// Soul Gem of Etrius: "If this vampire is burned, draw the top card from
// your crypt. If it is younger, put the Soul Gem on him or her and move
// him or her to your ready region with blood ... equal to his or her
// capacity; otherwise, move that card to your uncontrolled region (and
// burn the Soul Gem)"; [LSJ 20100213] a new vampire; [RTR 20010711] [LSJ
// 20030918] the drawn vampire burned as it enters (a copy of one already
// there), the Gem draws again; [LSJ 20060620] an empty crypt, none drawn.
#[test]
fn the_soul_gem_brings_a_younger_vampire_out() {
    let big = find_card("Stanislava");
    let (young, other) = (plain("Ventrue", 0), plain("Brujah", 0));
    // Younger: in play with the Gem, full.
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), big, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let gem = in_play(&mut e, 0, "Soul Gem of Etrius", Some(v));
    e.state.seats[0].crypt = vec![young];
    e.burn(v);
    let host = e.state.cards[gem.index()].host.expect("on the new vampire");
    assert_eq!(e.state.minions[host.index()].crypt_card(), Some(young));
    assert_eq!(blood(&e, host), CRYPT[young as usize].capacity);
    // Not younger: to the uncontrolled region, the Gem burned.
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), young, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let gem = in_play(&mut e, 0, "Soul Gem of Etrius", Some(v));
    e.state.seats[0].crypt = vec![big];
    e.burn(v);
    assert!(!e.state.cards[gem.index()].in_play);
    assert_eq!(e.state.seats[0].uncontrolled[0].card, big);
    // A copy of one already in play: burned, the Gem draws again.
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), big, 4, false);
    e.add_minion(SeatId(0), young, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let gem = in_play(&mut e, 0, "Soul Gem of Etrius", Some(v));
    e.state.seats[0].crypt = vec![other, young];
    e.burn(v);
    let host = e.state.cards[gem.index()]
        .host
        .expect("on the third vampire");
    assert_eq!(e.state.minions[host.index()].crypt_card(), Some(other));
    assert!(e.state.seats[0].ash_crypt.contains(&young));
    // [LSJ 20060620] "draw as much as you can": an empty crypt, the Gem
    // burns.
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), big, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let gem = in_play(&mut e, 0, "Soul Gem of Etrius", Some(v));
    e.state.seats[0].crypt.clear();
    e.burn(v);
    assert!(!e.state.cards[gem.index()].in_play);
}

// The Erciyes Fragments: "Lock this card to move a library card from your
// prey's ash heap to this card ... You may play the card from the
// Fragments as if playing it from your hand ... When that card is burned,
// remove it from the game instead": a Bum's Rush taken from seat 1's ash
// heap, played by seat 0's vampire, ends removed from the game — its
// owner's (rules-qa 60.19) — [LSJ
// 20081126] not replaced.
#[test]
fn the_erciyes_fragments_play_a_card_from_the_preys_ash_heap() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let frag = in_play(&mut e, 0, "Erciyes Fragments, The", None);
    let rush = find_library("Bum's Rush");
    e.state.seats[1].ash = vec![rush];
    pick(&mut e, use_at(frag, 0, None, Target::None));
    request(&mut e);
    assert_eq!(e.state.cards[frag.index()].stored, vec![rush]);
    assert!(e.state.seats[1].ash.is_empty());
    assert!(e.state.cards[frag.index()].locked);
    let play = Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(a),
        target: Target::Minion(z),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&play), "{:?}", req.options);
    pick(&mut e, play);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[frag.index()].stored.is_empty());
    // Its owner's removed cards (owner, 2026-10-09: rules-qa 60.19).
    assert_eq!(e.state.seats[1].removed, vec![rush]);
    assert!(e.state.seats[0].removed.is_empty());
    assert!(!e.state.seats[0].ash.contains(&rush));
    assert!(e.state.seats[0].hand.is_empty(), "not replaced");
}

// Call the Great Beast: "Put this card with X ritual counters on this
// Baali, where X is this Baali's capacity. This Baali can put 1 ritual
// counter on this card as a +1 stealth action. If this card has 11 or more
// ritual counters, move this card to your ready region, burn this Baali
// and choose three Disciplines": Nergal (10) puts it on with 10, adds the
// 11th by action; he burns, the Great Beast stands — 9 blood, 4 strength,
// 3 bleed, infernal, the three chosen at superior.
#[test]
fn call_the_great_beast_rises_at_eleven_counters() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nergal"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Call the Great Beast"]);
    pick(&mut e, play(0, 0, n));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let beast = card_named(&e, "Call the Great Beast");
    assert_eq!(e.state.cards[beast.index()].host, Some(n));
    assert_eq!(e.state.cards[beast.index()].counters, 10);
    e.state.minions[n.index()].locked = false;
    pick(&mut e, use_at(beast, 0, Some(n), Target::None));
    for _ in 0..3 {
        let req = to_window(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        let pick_first = req.options[0];
        pick(&mut e, pick_first);
    }
    request(&mut e);
    assert!(!e.state.minions[n.index()].in_play, "Nergal burned");
    let m = e.state.cards[beast.index()].body.expect("the Great Beast");
    let v = &e.state.minions[m.index()];
    assert!(v.in_play && !v.locked);
    assert_eq!(v.blood, 9);
    assert_eq!(e.state.capacity_of(m), Some(9));
    assert_eq!((e.state.base_strength(m), e.state.base_bleed(m)), (4, 3));
    assert!(e.state.is_infernal(m));
    assert_eq!(e.state.discipline_level(m, "abo"), 2);
    assert_eq!(e.state.sect_of(m), Some(Sect::Independent));
}

// Unleash Hell's Fury: "This card can attempt to block an action directed at
// you ... and is considered a 9-capacity infernal vampire with +2
// intercept during that attempt; if successful, the acting minion takes 1
// unpreventable environmental aggravated damage and this card is burned":
// Nergal puts it in play; seat 1's bleed is blocked by it — the bleeder
// to torpor, no pool bled, the card in the ash heap, no combat — [RTR
// 19991206] the effect triggered by the block applies.
#[test]
fn unleash_hells_fury_blocks_and_burns_the_acting_minion() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nergal"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Unleash Hell's Fury"]);
    pick(&mut e, play(0, 0, n));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let fury = card_named(&e, "Unleash Hell's Fury");
    let token = MinionId(
        e.state
            .minions
            .iter()
            .position(|m| matches!(m.kind, zvtes::state::MinionKind::Fury { .. }))
            .expect("its token") as u32,
    );
    assert!(!e.state.minions[token.index()].in_play);
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, bleed(z));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(token)), "{:?}", req.options);
    pick(&mut e, block(token));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    // The aggravated damage: to torpor; no bleed (the card cost 2 pool).
    assert!(e.state.minions[z.index()].in_torpor);
    assert_eq!(pool(&e)[0], 10 - 2);
    assert!(!e.state.cards[fury.index()].in_play);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Unleash Hell's Fury")));
    assert!(!e.state.minions[token.index()].in_play);
    assert!(e.combat().is_none());
    // Not against an action directed elsewhere: seat 1's bleed of seat 0
    // is the only one in a 2-seat game, so check the prey's copy instead.
    let mut e = game(3);
    let n = e.add_minion(SeatId(1), find_card("Nergal"), 6, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 1), 4, false);
    let _ = n;
    let fury = in_play(&mut e, 2, "Unleash Hell's Fury", None);
    e.state.minions.push(zvtes::state::Minion {
        in_play: false,
        ..zvtes::state::Minion::new(
            SeatId(2),
            zvtes::state::MinionKind::Fury {
                card: find_library("Unleash Hell's Fury"),
                id: fury,
            },
            0,
        )
    });
    let token = MinionId(e.state.minions.len() as u32 - 1);
    pick(&mut e, bleed(a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(token));
}

// Call the Great Beast [LSJ 20101130] "You cannot play Call the Great
// Beast with an 11-capacity vampire if you already control a Call the
// Great Beast vampire" — nor, here, at all; [LSJ 20080604] its counter
// action requires a Baali.
#[test]
fn call_the_great_beast_waits_for_the_first_beast() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nergal"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = in_play(&mut e, 0, "Call the Great Beast", None);
    e.state.minions.push(zvtes::state::Minion::new(
        SeatId(0),
        zvtes::state::MinionKind::GreatBeast {
            card: find_library("Call the Great Beast"),
            id: card,
        },
        9,
    ));
    hand(&mut e, 0, &["Call the Great Beast"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, n)));
}

// Unleash Hell's Fury and The Signet of King Saul [ANK 20190501] "Unleash
// Hell's Fury cannot block the bearer": a 9-capacity vampire.
#[test]
fn unleash_hells_fury_cannot_block_the_signet_bearer() {
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("Nergal"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let fury = in_play(&mut e, 0, "Unleash Hell's Fury", None);
    e.state.minions.push(zvtes::state::Minion {
        in_play: false,
        ..zvtes::state::Minion::new(
            SeatId(0),
            zvtes::state::MinionKind::Fury {
                card: find_library("Unleash Hell's Fury"),
                id: fury,
            },
            0,
        )
    });
    let token = MinionId(e.state.minions.len() as u32 - 1);
    in_play(&mut e, 1, "Signet of King Saul, The", Some(z));
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, bleed(z));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, block(token));
}

// Soul Gem of Etrius [LSJ 20000113] "Does not trigger if the bearer is
// removed from the game (eg. Golconda: Inner Peace)", and [LSJ 20100527]
// "Still produces its full effect if the new younger vampire cannot use
// equipment (eg. Beast, The Leatherface of Detroit)".
#[test]
fn the_soul_gem_ignores_a_removal_and_suits_the_beast() {
    let big = find_card("Stanislava");
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), big, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let gem = in_play(&mut e, 1, "Soul Gem of Etrius", Some(v));
    e.state.seats[1].crypt = vec![plain("Ventrue", 0)];
    hand(&mut e, 0, &["Golconda: Inner Peace"]);
    e.at_master_phase();
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(v),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[v.index()].in_play);
    assert_eq!(e.state.seats[1].crypt.len(), 1, "no crypt card drawn");
    assert!(!e.state.cards[gem.index()].in_play);

    let mut e = game(2);
    let v = e.add_minion(SeatId(0), big, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let gem = in_play(&mut e, 0, "Soul Gem of Etrius", Some(v));
    let beast = find_card("Beast, The Leatherface of Detroit");
    e.state.seats[0].crypt = vec![beast];
    e.burn(v);
    let host = e.state.cards[gem.index()].host.expect("on the Beast");
    assert_eq!(e.state.minions[host.index()].crypt_card(), Some(beast));
}

// The Sargon Fragment [RTR 19960221] "The action provided by the equipment
// to the attached minion fizzles ... if the equipment is not attached to
// the acting minion when the action resolves unblocked": moved off before
// resolution, nothing comes back.
#[test]
fn the_sargon_fragment_action_fizzles_once_detached() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let w = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let frag = in_play(&mut e, 0, "Sargon Fragment, The", Some(v));
    let rush = find_library("Bum's Rush");
    e.state.seats[0].ash = vec![rush];
    pick(&mut e, use_at(frag, 0, Some(v), Target::None));
    request(&mut e);
    e.state.cards[frag.index()].host = Some(w);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.seats[0].hand.contains(&rush));
}

// Lord Tremere [LSJ 20011022] [LSJ 20100902-2] "His unlocking ability can
// be used when he is in torpor", and [LSJ 20010702] [RTR 19941006] [LSJ
// 19991025] [LSJ 20100527] "If a minion who is attempting to block is
// locked, then that block will fail": from torpor he unlocks Mistress
// Fanchon; then he locks Gerald Windham as Gerald attempts to block
// Fanchon's bleed — the bleed goes through.
#[test]
fn lord_tremere_works_from_torpor_and_stops_a_blocker() {
    let mut e = game(2);
    let lord = e.add_minion(SeatId(0), find_card("Lord Tremere"), 4, true);
    let f = e.add_minion(SeatId(0), find_card("Mistress Fanchon"), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Gerald Windham"), 4, false);
    e.state.minions[f.index()].locked = true;
    pick(&mut e, host_ability(lord, 0, Target::Minion(f)));
    assert!(!e.state.minions[f.index()].locked);
    // A new minion phase for the second use (once in each).
    e.state.turn_uses.clear();
    pick(&mut e, bleed(f));
    pick_when(&mut e, block(g));
    pick_when(&mut e, host_ability(lord, 0, Target::Minion(g)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 3, "unblocked: Fanchon's +2 bleed");
}

// The Capuchin [ANK 20200417] "If his controller is not his original owner
// and he burns, he goes to his original owner ... then to his owner's
// uncontrolled area. The Methuselah who was controlling him when he burned
// gets the additional transfers".
#[test]
fn the_capuchin_returns_to_his_owner_and_pays_his_controller() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(1), find_card("Capuchin, The"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[c.index()].origin = Some(SeatId(1));
    e.state.minions[c.index()].owner = SeatId(0);
    e.burn(c);
    assert_eq!(
        e.state.seats[1].uncontrolled[0].card,
        find_card("Capuchin, The")
    );
    assert_eq!(e.state.seats[0].transfers_next, 3);
    assert_eq!(e.state.seats[1].transfers_next, 0);
}

// The Erciyes Fragments [LSJ 20050913] (Disguised Weapon, Contraband, Pack
// Alpha) "Cannot select cards
// that are playable as if in hand (e.g. from The Erciyes Fragments)": a
// .44 Magnum on the Fragments is no weapon for Disguised Weapon to equip.
#[test]
fn the_erciyes_fragments_card_is_not_in_hand() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let frag = in_play(&mut e, 0, "Erciyes Fragments, The", None);
    e.state.cards[frag.index()].stored = vec![find_library(".44 Magnum")];
    hand(&mut e, 0, &["Disguised Weapon"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::None,
        },
    );
}

// I am Legion [dai][obf] [ANK 20190104] [LSJ 19980212] "The canceled card
// has still been played", [ANK 20260216] "The cost for a canceled
// non-action-card is still paid": Nergal cancels the Eagle's Sight of the
// vampire trying to block him; his +1 stealth holds and the bleed lands.
#[test]
fn i_am_legion_cancels_an_auspex_reaction() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nergal"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    hand(&mut e, 0, &["Cloak the Gathering", "I am Legion"]);
    hand(&mut e, 1, &["Eagle's Sight"]);
    pick(&mut e, bleed(n));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, n));
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 1, n));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Eagle's Sight")));
    assert_eq!(pool(&e)[1], 10 - 3, "Nergal's bleed for 3");
}

// The Capuchin: "+2 hand size" — influenced out, his Methuselah draws up
// to 9 at once (rulebook "Drawing Cards": "whenever an effect changes your
// hand size ..., immediately draw up"), not at the next replacement.
#[test]
fn the_capuchin_influenced_out_draws_up_at_once() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: find_card("Capuchin, The"),
            counters: 11,
            banished: None,
        });
    let villein = find_library("Villein");
    e.state.seats[0].hand = vec![villein; 7];
    e.state.seats[0].library = vec![villein; 10];
    to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    assert_eq!(e.state.seats[0].hand.len(), 7);
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    e.run();
    assert_eq!(e.state.hand_size(SeatId(0)), 9);
    assert_eq!(e.state.seats[0].hand.len(), 9);
    assert_eq!(e.state.seats[0].library.len(), 8);
}

// The Capuchin's 9 cards: a card played is out of hand until its
// replacement, never drawn for early — the hand stays at 9, no discard.
#[test]
fn the_capuchin_hand_stays_at_nine_through_a_play() {
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("Capuchin, The"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Dreams of the Sphinx"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Villein"); 10];
    assert_eq!(e.state.seats[0].hand.len(), 9);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::Discard(0),
    );
    assert_eq!(e.state.seats[0].hand.len(), 9);
    assert_eq!(e.state.seats[0].library.len(), 9, "one replacement");
}
