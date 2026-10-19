//! Legacy cards for TWDA decks, round 9 (`ADDED_BATCHES`, the nineteenth
//! batch, slice 16u): Shadow Play, The Embrace and the Appolonius merge —
//! the Lasombra, Toreador antitribu and Ravnos TWDA decks and Popó.
//! Readings in rules-qa Batch 69. Windows with a single option are elided,
//! so the scripts only stop where a real decision exists.

use zvtes::actions::Outcome;
use zvtes::cards::{AbilitySource, Target};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, CardId, CardInPlay, Expiry, TimedStat};

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

/// A vampire's own ability `ability` used by itself.
fn printed(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: Some(m),
        target,
    }
}

/// A vampire's own ability `ability` used outside its actions (no actor).
fn own_use(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: None,
        target,
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

/// `m` gets `disc` at superior level (the fixture's Great Beast slot).
fn superior(e: &mut Engine, m: MinionId, disc: &'static str) {
    let slots = &mut e.state.minions[m.index()].beast_disciplines;
    let free = slots.iter().position(Option::is_none).expect("a free slot");
    slots[free] = Some(disc);
}

/// `a` (seat 0) bleeds; `z` (seat 1) blocks: the combat begins.
fn blocked_bleed(e: &mut Engine, a: MinionId, z: MinionId) {
    pick(e, bleed(a));
    pick_when(e, block(z));
}

// ---- Obtenebration ------------------------------------------------------------

// Shadow Play: "[obt] +1 stealth. [OBT] +2 stealth." Shroud of Night: "[obt]
// +1 stealth, not usable during a bleed action. [OBT] +1 stealth." Against a
// bleed's block attempt, Shadow Play is offered, Shroud of Night at
// inferior is not; Shadow Play's +1 makes the block fail.
#[test]
fn shroud_of_night_at_inferior_skips_bleeds() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Shroud of Night", "Shadow Play"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, play(1, 0, a));
    assert!(!req.options.contains(&play(0, 0, a)));
    e.choose(index_of(&req, play(1, 0, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 1, "unblocked");
}

// [LSJ 20031121-2] Tenebrous Form [OBT]: "minions without Obtenebration
// [obt] must burn 1 blood to attempt to block" — "A minion previously
// attempting to block can resign his attempt and pay no blood." A new
// attempt pays.
#[test]
fn tenebrous_form_makes_an_attempt_pay_or_resign() {
    for pay in [false, true] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 4, false);
        superior(&mut e, a, "obt");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
        hand(&mut e, 0, &["Tenebrous Form"]);
        // An intercept card keeps a new attempt reachable.
        hand(&mut e, 1, &["Enhanced Senses"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, 1, a));
        let req = expect(&mut e, SeatId(1), WindowLabel::BlockToll);
        assert_eq!(req.options, vec![Choice::PayBlockToll, Choice::Pass]);
        e.choose(if pay { 0 } else { 1 });
        assert_eq!(blood(&e, z), if pay { 4 - 1 } else { 4 });
        if !pay {
            pick_when(&mut e, block(z));
            request(&mut e);
            assert_eq!(blood(&e, z), 4 - 1);
        }
    }
}

// [LSJ 19970707] [ANK 20230226] Unleash Hell's Fury: "It is a Vampire with 0
// blood while attempting to block … It cannot 'burn x blood to attempt to
// block' (eg Tenebrous Form)": seat 0's Fury attempts to block Montano's
// bleed; Tenebrous Form [OBT] ends the attempt — the bleed lands.
#[test]
fn tenebrous_form_stops_unleash_hells_fury() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nergal"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("obt", 0), 4, false);
    superior(&mut e, z, "obt");
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Unleash Hell's Fury"]);
    pick(&mut e, play(0, 0, n));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let token = MinionId(
        e.state
            .minions
            .iter()
            .position(|m| matches!(m.kind, zvtes::state::MinionKind::Fury { .. }))
            .expect("its token") as u32,
    );
    to_minion_phase(&mut e, SeatId(1));
    hand(&mut e, 1, &["Tenebrous Form"]);
    pick(&mut e, bleed(z));
    pick_when(&mut e, block(token));
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[0], 10 - 2 - 1, "the Fury's 2 pool, the bleed's 1");
}

// Blanket of Night [OBT]: "Only usable by a ready vampire other than the
// acting minion if a minion attempts to block. Lock this modifying vampire
// to have that attempt fail; the blocking minion cannot attempt to block
// this action again." [LSJ 19990425] "Can only be played by the controller
// of the acting minion." [ANK 20200515] "Multiple copies can be played by
// different minions on the same action": two attempts fail, each to
// another of seat 0's vampires; seat 1's [OBT] vampire is never offered it.
#[test]
fn blanket_of_night_fails_attempts_from_other_vampires() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), vampire_with("obt", 0), 4, false);
    superior(&mut e, b, "obt");
    let c = e.add_minion(SeatId(0), vampire_with("obt", 1), 4, false);
    superior(&mut e, c, "obt");
    // A minion phase with more than a Pass left, so it does not elide.
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let y = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    let w = e.add_minion(SeatId(1), vampire_with("obt", 2), 4, false);
    superior(&mut e, w, "obt");
    let z = e.add_minion(SeatId(2), vampire_with("aus", 1), 4, false);
    hand(&mut e, 0, &["Blanket of Night", "Blanket of Night"]);
    hand(&mut e, 1, &["Eagle's Sight", "Blanket of Night"]);
    hand(&mut e, 2, &["Eagle's Sight"]);
    // A hunt: undirected, both other Methuselahs may block.
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, block(y));
    let req = offering(&mut e, play(0, 1, b));
    assert!(
        !req.options.contains(&play(0, 1, a)),
        "not the acting minion"
    );
    e.choose(index_of(&req, play(0, 1, b)));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, play(0, 1, c));
    assert!(!req.options.contains(&play(0, 1, b)), "b is locked");
    e.choose(index_of(&req, play(0, 1, c)));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(1, 1, w));
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
    let locked: Vec<bool> = e.state.minions.iter().map(|m| m.locked).collect();
    assert!(
        e.state.minions[b.index()].locked && e.state.minions[c.index()].locked,
        "{locked:?}"
    );
}

// Oubliette: "[obt] Strike: combat ends. [OBT] As above, and the opposing
// minion burns 1 blood or life after combat ends (even at long range)."
// [LSJ 20080226] "Does not inflict damage": no prevention reaches it.
// [LSJ 20031123] "Any blood loss occurs after combat": the combat is over
// as it burns. [LSJ 19980526] [RTR 20041202] "If the strike is dodged, no
// blood is burned."
#[test]
fn oubliette_burns_after_combat_unless_dodged() {
    for dodge in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 4, false);
        superior(&mut e, a, "obt");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with("pro", 0), 4, false);
        hand(&mut e, 0, &["Oubliette"]);
        hand(&mut e, 1, &["Form of Mist"]);
        blocked_bleed(&mut e, a, z);
        until(&mut e, SeatId(0), WindowLabel::CombatStrike);
        pick(&mut e, play(0, 1, a));
        let req = expect(&mut e, SeatId(1), WindowLabel::CombatStrike);
        if dodge {
            e.choose(index_of(&req, play(0, 0, z)));
        } else {
            e.choose(index_of(&req, Choice::HandStrike));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, z), if dodge { 4 } else { 4 - 1 });
    }
}

// [RTR 20020501] Oubliette [OBT]: "If the combat continues or a new combat
// begins, no blood is burned." [RTR 20020501] [LSJ 19980109] "If the combat
// continues …, the after combat effect does not happen": Telepathic
// Tracking [AUS] starts a new round; no burn after.
#[test]
fn oubliette_burns_nothing_if_the_combat_continues() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 4, false);
    superior(&mut e, a, "obt");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
    hand(&mut e, 0, &["Oubliette"]);
    hand(&mut e, 1, &["Telepathic Tracking"]);
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Telepathic Tracking's 1, round 2's hand strike: no Oubliette burn.
    assert_eq!(blood(&e, z), 4 - 1 - 1);
}

// Shadow Body [OBT]: "if this vampire was blocked while performing a
// non-bleed non-political action, the action continues as if unblocked."
// [ANK 20190116] "any Methuselahs that have declined to block before do
// not get another opportunity to block." [RTR 19941109] [ANK 20190116]
// "all action modifiers and reaction cards are still in effect".
// [LSJ 20070808-1] the action card goes back from the ash heap: a blocked
// Art Scam, continued, gains its 2 pool; seat 1, which declined, is not
// asked again. A bleed blocked just ends the combat.
#[test]
fn shadow_body_continues_a_non_bleed_action() {
    let mut e = game(3);
    let t = e.add_minion(SeatId(0), find_card("Lolita"), 3, false);
    superior(&mut e, t, "obt");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let y = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    let z = e.add_minion(SeatId(2), vampire_with("aus", 1), 4, false);
    hand(&mut e, 0, &["Art Scam", "Shadow Body"]);
    hand(&mut e, 1, &["Eagle's Sight"]);
    hand(&mut e, 2, &["Eagle's Sight"]);
    pick(&mut e, play(0, 0, t));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(y)));
    e.choose(index_of(&req, Choice::Pass));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 1, t));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(y));
    assert_eq!(pool(&e)[0], 10 + 2, "Art Scam resolved");
    assert!(e.state.seats[0].ash.contains(&find_library("Art Scam")));
}

// Shadow Body [OBT]: "a non-bleed non-political action" — a bleed blocked
// is not continued: Shadow Body [OBT] just ends the combat.
#[test]
fn shadow_body_does_not_continue_a_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 4, false);
    superior(&mut e, a, "obt");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Shadow Body"]);
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
    assert_eq!(pool(&e)[1], 10);
}

/// A vampire with no definition of its own, of capacity `cap` exactly, the
/// `nth` of them.
fn of_capacity(cap: u8, nth: usize) -> u16 {
    zvtes::crypt::CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| c.capacity == cap && zvtes::cards::vampire(*i as u16).is_none())
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a vampire of the capacity")
}

// Shroud of Absence [DOM][OBT]: "As above, and this vampire can strike:
// combat ends during the first round of the resulting combat if they are
// blocked." Blocked after its +1 stealth, the vampire is offered the
// combat-ends strike in the first round. A combo card: Ignacio's Dominate
// and Obtenebration, both superior.
#[test]
fn shroud_of_absence_gives_a_combat_ends_strike_if_blocked() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Ignacio, The Black Priest"), 4, false);
    superior(&mut e, a, "dom");
    superior(&mut e, a, "obt");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    hand(&mut e, 0, &["Shroud of Absence"]);
    hand(&mut e, 1, &["Eagle's Sight"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, z));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(req.options.contains(&Choice::StrikeCombatEnds));
    e.choose(index_of(&req, Choice::StrikeCombatEnds));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, z)), (4, 4), "no damage");
}

// Metro Underground: "During your discard phase, you may lock this card and
// burn 1 pool to unlock a vampire you control."
#[test]
fn metro_underground_unlocks_a_vampire_in_the_discard_phase() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let metro = in_play(&mut e, 0, "Metro Underground", None);
    pick(&mut e, action(a, ActionKind::Hunt));
    let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    let unlock = use_at(metro, 0, None, Target::Minion(a));
    e.choose(index_of(&req, unlock));
    assert!(!e.state.minions[a.index()].locked);
    assert!(e.state.cards[metro.index()].locked);
    assert_eq!(pool(&e)[0], 10 - 1);
}

// [PIB 20130221] Gurchon Hall: "move 1 blood from the blood bank to each of
// the two oldest ready vampires you control" — "You need two ready vampires
// to gain blood." The oldest (capacity 7) gains; the two 5s tie for the
// second: their controller's pick (rules-qa 69.1). With one ready vampire
// left, the hall is not offered.
#[test]
fn gurchon_hall_feeds_the_two_oldest_ready_vampires() {
    let mut e = game(2);
    let old = e.add_minion(SeatId(0), of_capacity(7, 0), 2, false);
    let x = e.add_minion(SeatId(0), of_capacity(5, 0), 2, false);
    let y = e.add_minion(SeatId(0), of_capacity(5, 1), 2, false);
    let young = e.add_minion(SeatId(0), of_capacity(3, 0), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let hall = in_play(&mut e, 0, "Gurchon Hall", None);
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    e.choose(index_of(&req, use_at(hall, 0, None, Target::None)));
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![Choice::ChooseMinion(x), Choice::ChooseMinion(y)]
    );
    e.choose(1);
    assert_eq!(
        [old, x, y, young].map(|m| blood(&e, m)),
        [3, 2, 3, 2],
        "the oldest and the 7 picked"
    );
    // Next turn: three vampires in torpor, one ready — not offered.
    for m in [old, x, y] {
        e.state.minions[m.index()].in_torpor = true;
    }
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_at(hall, 0, None, Target::None),
    );
}

// [LSJ 20010326] Alvaro: "After resolution of any political action, if
// Alvaro abstained (even in torpor), he can gain 2 blood" — "He gains blood
// if he casts no vote, even if he has lost his title or is in torpor": in
// torpor, he gains 2 after Autarkis Persecution's referendum, failed.
#[test]
fn alvaro_gains_blood_after_a_referendum_he_abstained_from() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let alvaro = e.add_minion(
        SeatId(1),
        find_card("Alvaro, The Scion of Angelica"),
        3,
        true,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Autarkis Persecution"]);
    pick(&mut e, play(0, 0, caller));
    to_referendum(&mut e);
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, alvaro), 3 + 2);
}

// Ambrosio Luis Monçada: "Monçada cannot block. Actions directed at him cost
// 1 additional pool." [LSJ 20030214-2] "Actions from minions of the same
// controller targeting him are undirected, so they are not affected": Anka
// rushes him — seat 0 pays 1 pool as the rush resolves; his own Methuselah
// rescues him from torpor for nothing more.
#[test]
fn ambrosio_taxes_directed_actions_and_cannot_block() {
    let mut e = game(2);
    let anka = e.add_minion(SeatId(0), find_card("Anka, Priestess of Thorns"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let amb = e.add_minion(
        SeatId(1),
        find_card("Ambrosio Luis Monçada, Plenipotentiary"),
        4,
        false,
    );
    let other = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(anka));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(amb));
    assert_eq!(pool(&e), vec![10, 10 - 1]);
    e.state.minions[anka.index()].locked = false;
    e.state.minions[anka.index()].acted.clear();
    e.state.minions[anka.index()].bled_this_turn = false;
    pick(&mut e, printed(anka, 0, Target::Minion(amb)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[0], 10 - 1, "the rush's pool");
    // Seat 1 rescues its own Ambrosio: undirected, untaxed.
    e.state.minions[amb.index()].in_torpor = true;
    e.state.minions[amb.index()].blood = 1;
    to_minion_phase(&mut e, SeatId(1));
    let p = pool(&e)[1];
    pick(
        &mut e,
        Choice::TakeAction {
            minion: other,
            action: ActionKind::Rescue,
            target: Some(amb),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[amb.index()].in_torpor);
    assert_eq!(pool(&e)[1], p);
}

/// Ambrosio's toll ousting the acting Methuselah ends the action, noted
/// (`Engine::action_notes`): the turn passes with it, or the game ends.
#[test]
fn ambrosio_s_toll_ousting_its_payer_ends_the_action() {
    for seats in [3, 2] {
        let mut e = game(seats);
        e.action_notes = Some(Vec::new());
        let anka = e.add_minion(SeatId(0), find_card("Anka, Priestess of Thorns"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let amb = e.add_minion(
            SeatId(1),
            find_card("Ambrosio Luis Monçada, Plenipotentiary"),
            4,
            false,
        );
        e.state.seats[0].pool = 1;
        pick(&mut e, printed(anka, 0, Target::Minion(amb)));
        let over = loop {
            match e.run() {
                zvtes::game::Step::Over(_) => break true,
                zvtes::game::Step::Decision(_) if e.state.seats[0].ousted => break false,
                zvtes::game::Step::Decision(req) => e.choose(fallback(&req)),
            }
        };
        assert!(e.state.seats[0].ousted, "{seats} seats");
        assert_eq!(over, seats == 2);
        assert!(e.action().is_none(), "{seats} seats: the action ended");
        let notes = e.action_notes.take().expect("noted");
        let told: Vec<_> = notes.iter().map(|n| (n.ended, n.ousted)).collect();
        assert_eq!(told, [(false, false), (true, true)], "{seats} seats");
    }
}

// Ambrosio: "If he is ready during your discard phase, you can unlock
// another ready Lasombra."
#[test]
fn ambrosio_unlocks_another_lasombra_in_the_discard_phase() {
    let mut e = game(2);
    let amb = e.add_minion(
        SeatId(0),
        find_card("Ambrosio Luis Monçada, Plenipotentiary"),
        4,
        false,
    );
    let las = e.add_minion(SeatId(0), find_card("Ramiro"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, action(las, ActionKind::Hunt));
    let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    e.choose(index_of(&req, own_use(amb, 0, Target::Minion(las))));
    assert!(!e.state.minions[las.index()].locked);
}

// Antonio Delgado: "If Antonio performs a political action whose referendum
// passes, he can unlock during the discard phase."
#[test]
fn antonio_delgado_unlocks_after_his_referendum_passes() {
    let mut e = game(2);
    let antonio = e.add_minion(SeatId(0), find_card("Antonio Delgado"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Autarkis Persecution"]);
    pick(&mut e, play(0, 0, antonio));
    to_referendum(&mut e);
    pick_when(&mut e, votes(antonio, true));
    close_polling(&mut e);
    let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    e.choose(index_of(&req, own_use(antonio, 0, Target::None)));
    assert!(!e.state.minions[antonio.index()].locked);
}

// [ANK 20231124] [LSJ 19970718] Francisco Domingo de Polonia: "Once each
// turn, Francisco can burn 1 blood to get +1 bleed for the current action"
// — "can only be used during a bleed action before Action Resolution".
#[test]
fn francisco_burns_blood_for_bleed_once_a_turn() {
    let mut e = game(2);
    let f = e.add_minion(
        SeatId(0),
        find_card("Francisco Domingo de Polonia"),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(f));
    pick_when(&mut e, own_use(f, 0, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 2);
    assert_eq!(blood(&e, f), 4 - 1);
}

// Gratiano: "Sabbat priscus: Gratiano gets +1 ballot." [LSJ 20051113-2]
// "The vampire's votes (if any) and ballots must be cast on the same side":
// his two ballots carry the prisci block against one priscus's ballot.
// [LSJ 20040519] "If Gratiano loses or changes his title, he still gets
// one ballot": untitled, he still casts one.
#[test]
fn gratiano_casts_an_extra_prisci_ballot() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Gratiano"), 4, false);
    assert_eq!(e.state.ballots(g), 2);
    e.state.minions[g.index()].title_lost = true;
    assert_eq!(e.state.ballots(g), 1, "the printed ballot");
    e.state.minions[g.index()].title_lost = false;
    hand(&mut e, 0, &["Autarkis Persecution"]);
    pick(&mut e, play(0, 0, caller));
    to_referendum(&mut e);
    pick_when(&mut e, votes(g, false));
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Against: the prisci block's 3; for: nothing — no pool gained.
    assert_eq!(pool(&e), vec![10, 10]);
}

/// The printed stat of `m` for an action of `kind` (no opponent).
fn stat_on(e: &Engine, m: MinionId, stat: zvtes::cards::Stat, kind: ActionKind) -> i32 {
    e.state.printed_mod(
        m,
        stat,
        zvtes::state::PrintedCtx {
            kind: Some(kind),
            ..Default::default()
        },
    )
}

// Royce: "-1 stealth when bleeding." Jost Werner: "+1 stealth." Zöe: "does
// not get the usual +1 stealth when hunting." Khalil Ravana: "+1 stealth
// during diablerie actions." Richter: "+1 strength."
#[test]
fn printed_stealth_and_strength_of_the_round_9_vampires() {
    use zvtes::cards::Stat;
    let mut e = game(2);
    let royce = e.add_minion(SeatId(0), find_card("Royce"), 1, false);
    let jost = e.add_minion(SeatId(0), find_card("Jost Werner"), 4, false);
    let zoe = e.add_minion(SeatId(0), find_card("Zöe"), 3, false);
    let khalil = e.add_minion(SeatId(0), find_card("Khalil Ravana"), 4, false);
    let richter = e.add_minion(
        SeatId(0),
        find_card("Richter, The Templar of Du Mont"),
        4,
        false,
    );
    assert_eq!(stat_on(&e, royce, Stat::Stealth, ActionKind::Bleed), -1);
    assert_eq!(stat_on(&e, royce, Stat::Stealth, ActionKind::Hunt), 0);
    assert_eq!(stat_on(&e, jost, Stat::Stealth, ActionKind::Bleed), 1);
    assert_eq!(stat_on(&e, zoe, Stat::Stealth, ActionKind::Hunt), -1);
    assert_eq!(stat_on(&e, zoe, Stat::Stealth, ActionKind::Bleed), 0);
    assert_eq!(stat_on(&e, khalil, Stat::Stealth, ActionKind::Diablerie), 1);
    assert_eq!(stat_on(&e, khalil, Stat::Stealth, ActionKind::Bleed), 0);
    assert_eq!(
        e.state
            .printed_mod(richter, Stat::Strength, Default::default()),
        1
    );
    // Zöe's hunt: 1 - 1, a block with no intercept succeeds.
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, action(zoe, ActionKind::Hunt));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
}

// [LSJ 20010111] Angelica: "During an action, Angelica can discard X master
// cards to get +X intercept" — "Can only use her ability once per action if
// she needs intercept, but she can get more than she needs." [LSJ 19981005]
// "The cards must all be burned at once": two masters discarded together
// against a +1 stealth bleed, +2 intercept; the block succeeds.
#[test]
fn angelica_discards_masters_for_intercept() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ang = e.add_minion(SeatId(1), find_card("Angelica, The Canonicus"), 4, false);
    hand(&mut e, 0, &["Shadow Play"]);
    hand(
        &mut e,
        1,
        &["Metro Underground", "Gird Minions", "Eagle's Sight"],
    );
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(ang));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, own_use(ang, 0, Target::None));
    pick(&mut e, Choice::Discard(0));
    pick(&mut e, Choice::Discard(1));
    let req = request(&mut e);
    assert!(
        !req.options.iter().any(|o| matches!(o, Choice::Discard(_))),
        "no master left to pick"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
    let ash = &e.state.seats[1].ash;
    assert!(ash.contains(&find_library("Metro Underground")));
    assert!(ash.contains(&find_library("Gird Minions")));
}

// Ayelea: "Ayelea may give you control of a Tremere antitribu until the end
// of your next turn as a Ⓓ action." Through seat 0's next turn, back at its
// end (rules-qa 69.3).
#[test]
fn ayelea_borrows_a_tremere_antitribu_until_the_end_of_her_next_turn() {
    let mut e = game(2);
    let ay = e.add_minion(SeatId(0), find_card("Ayelea, The Manipulator"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let tc = zvtes::crypt::CRYPT
        .iter()
        .position(|c| c.clan == "Tremere antitribu")
        .expect("a Tremere antitribu") as u16;
    let t = e.add_minion(SeatId(1), tc, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, printed(ay, 0, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[t.index()].owner, SeatId(0));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.minions[t.index()].owner, SeatId(0), "still hers");
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.minions[t.index()].owner, SeatId(0));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.minions[t.index()].owner, SeatId(1), "back");
}

// Marciana Giovanni: "During an action, you can lock her and choose a
// non-Sabbat vampire to allow that vampire to burn 1 blood to get +1
// intercept." [LSJ 20100604-2] [LSJ 20100611] "The beneficiary … can
// choose to burn a blood for intercept for the duration of the action."
// [LSJ 20010813-2] "Her ability to give intercept can be used when she is
// in torpor." In torpor, she lets her Methuselah's blocker burn for +1
// intercept against a +2 stealth bleed, once (rules-qa 79.2): the bleed
// goes through.
#[test]
fn marciana_lets_a_vampire_burn_blood_for_intercept() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 4, false);
    superior(&mut e, a, "obt");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let m = e.add_minion(
        SeatId(1),
        find_card("Marciana Giovanni, Investigator"),
        2,
        true,
    );
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    hand(&mut e, 0, &["Shadow Play"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, own_use(m, 0, Target::Minion(z)));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    let burn = Choice::BurnBloodForIntercept { minion: z };
    pick_when(&mut e, burn);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, burn);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
    assert_eq!(blood(&e, z), 5 - 1);
    assert!(e.state.minions[m.index()].locked);
}

// [LSJ 20100527] Marciana: "She can lock to grant herself the ability to
// burn blood for intercept. She would need a wake or unlock effect to
// attempt to block." [LSJ 20010702] [RTR 19941006] [LSJ 19991025] [LSJ
// 20100527] "If a minion who is attempting to block is locked, then that
// block will fail": attempting, she locks for her own grant — the attempt
// fails.
#[test]
fn marciana_locking_during_her_own_attempt_fails_it() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let m = e.add_minion(
        SeatId(1),
        find_card("Marciana Giovanni, Investigator"),
        2,
        false,
    );
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(m));
    pick_when(&mut e, own_use(m, 0, Target::Minion(m)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
}

// Marciana: "Bleed actions cost Marciana 1 additional blood."
#[test]
fn marciana_pays_a_blood_to_bleed() {
    let mut e = game(2);
    let m = e.add_minion(
        SeatId(0),
        find_card("Marciana Giovanni, Investigator"),
        2,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(m));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, m), pool(&e)[1]), (1, 9));
}

// Montano: "Cards requiring Obtenebration [obt] cost Montano 1 fewer blood.
// During an action, he can burn 1 blood to give a minion you control +1
// stealth." [ANK 20220705] [LSJ 20100705] "His ability to provide stealth
// can be used while locked or in torpor": in torpor, he gives the bleeder
// +1 stealth past a block; Tenebrous Form costs him nothing. [LSJ 20010714]
// "Keeps their title and votes if they changes sects": made Anarch, still 2
// votes.
#[test]
fn montano_lends_stealth_from_torpor_and_pays_less_for_obtenebration() {
    use zvtes::cards::Stat;
    let mut e = game(2);
    let mon = e.add_minion(SeatId(0), find_card("Montano"), 4, true);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ctx = zvtes::state::PrintedCtx {
        card: Some((find_library("Tenebrous Form"), 0)),
        ..Default::default()
    };
    assert_eq!(e.state.printed_mod(mon, Stat::BloodCostOff, ctx), 1);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, own_use(mon, 0, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
    assert_eq!(blood(&e, mon), 3);
    e.state.minions[mon.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    assert_eq!(e.state.title_of(mon), zvtes::crypt::Title::Votes(2));
}

// [LSJ 19990215] Lázár Dobrescu: "Lázár can move 1 blood from a crypt card
// in your prey's uncontrolled region to a vampire in your uncontrolled
// region as a Ⓓ action" — "Cannot use his special ability if his
// controller has no vampires in her uncontrolled region."
#[test]
fn lazar_moves_blood_between_uncontrolled_regions() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lázár Dobrescu"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let slot = |card| zvtes::state::UncontrolledSlot {
        card,
        copy: Default::default(),
        counters: 2,
        banished: None,
    };
    e.state.seats[1]
        .uncontrolled
        .push(slot(plain("Ventrue", 1)));
    let act = printed(l, 0, Target::Seat(SeatId(1)));
    assert!(!request(&mut e).options.contains(&act), "none of his own");
    e.state.seats[0].uncontrolled.push(slot(plain("Brujah", 2)));
    pick(&mut e, act);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].uncontrolled[0].counters, 1);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 3);
}

// Spleen: "If Spleen is blocked by an older vampire, he burns 1 blood before
// block resolution." Blocked by a capacity-4 vampire: 1 blood; by a
// capacity-2 one: none.
#[test]
fn spleen_burns_blood_when_an_older_vampire_blocks() {
    for (cap, burn) in [(4, 1), (2, 0)] {
        let mut e = game(2);
        let s = e.add_minion(SeatId(0), find_card("Spleen, Georgio Gutterpunk"), 2, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), of_capacity(cap, 0), cap, false);
        pick(&mut e, bleed(s));
        pick_when(&mut e, block(z));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        // The combat's hand strikes: 1 each (Spleen's last blood).
        assert_eq!(blood(&e, s), (2u8 - burn).saturating_sub(1));
    }
}

// [RTR 19970630] Shadow Body [OBT]: "An action cannot continue 'as if
// unblocked' after a combat resulting from a successful action (e.g. Bum's
// Rush)": Anka's rush at a Lasombra, Shadow Body [OBT] in its combat just
// ends it.
#[test]
fn shadow_body_does_not_continue_a_rush() {
    let mut e = game(2);
    let anka = e.add_minion(SeatId(0), find_card("Anka, Priestess of Thorns"), 4, false);
    superior(&mut e, anka, "obt");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let las = e.add_minion(SeatId(1), find_card("Ramiro"), 4, false);
    hand(&mut e, 0, &["Shadow Body"]);
    let rush = printed(anka, 0, Target::Minion(las));
    let req = request(&mut e);
    assert!(req.options.contains(&rush));
    let nolas = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert!(!request(&mut e)
        .options
        .contains(&printed(anka, 0, Target::Minion(nolas))));
    pick(&mut e, rush);
    pick_when(&mut e, play(0, 1, anka));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
    assert!(e.state.minions[anka.index()].locked);
    assert_eq!(
        (blood(&e, anka), blood(&e, las)),
        (4 - 1, 4),
        "Shadow Body's 1 blood"
    );
}

// ---- The Embrace, Toreador antitribu, Ravnos ---------------------------------

// The Embrace: "+1 stealth action. Requires a non-sterile vampire. Put this
// card in play. It becomes a 1-capacity non-unique vampire of the same clan
// and sect as the acting vampire, and must hunt this turn." No Discipline
// master search (Childe of the Revolution's): a Toreador antitribu Sabbat
// childe with no blood, which hunts at once. A sterile vampire is not
// offered it.
#[test]
fn the_embrace_makes_a_childe_of_the_sires_clan_and_sect() {
    let mut e = game(2);
    let sire = e.add_minion(SeatId(0), find_card("Lolita"), 3, false);
    let sterile = e.add_minion(SeatId(0), find_card("Effie Lowery"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Embrace, The"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, sterile)), "sterile");
    let before = e.state.minions.len();
    pick(&mut e, play(0, 0, sire));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let childe = MinionId(before as u32);
    let v = &e.state.minions[childe.index()];
    assert_eq!(v.clan(), Some("Toreador antitribu"));
    assert_eq!(e.state.sect_of(childe), Some(zvtes::crypt::Sect::Sabbat));
    assert_eq!(e.state.capacity_of(childe), Some(1));
    assert!(v.locked, "it hunted at once");
    assert_eq!(v.blood, 1);
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.body == Some(childe) && c.in_play));
}

// [LSJ 20091012] [LSJ 20020609] Hermana Hambrienta: "If she creates a vampire
// (eg. The Embrace), they are not Mexico Circle, they are each in their own
// individual circle of one." [LSJ 20020609] the circle of one. [LSJ
// 20020115] "Is not a vampire in the ash heap": diablerized, its card goes
// to the ash heap as the library card.
#[test]
fn an_embraced_blood_brother_is_in_a_circle_of_one() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hermana Hambrienta Mayor"), 4, false);
    let h2 = e.add_minion(SeatId(0), find_card("Hermana Hambrienta Menor"), 4, false);
    let d = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Embrace, The"]);
    let before = e.state.minions.len();
    pick(&mut e, play(0, 0, h));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let childe = MinionId(before as u32);
    assert_eq!(
        e.state.minions[childe.index()].clan(),
        Some("Blood Brother")
    );
    assert!(e.state.same_circle(h, h2));
    assert!(!e.state.same_circle(h, childe));
    to_minion_phase(&mut e, SeatId(1));
    e.state.minions[childe.index()].in_torpor = true;
    pick(
        &mut e,
        Choice::TakeAction {
            minion: d,
            action: ActionKind::Diablerie,
            target: Some(childe),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[childe.index()].in_play);
    assert!(
        e.state.seats[0].ash_crypt.is_empty(),
        "no vampire in the ash heap"
    );
    assert!(e.state.seats[0].ash.contains(&find_library("Embrace, The")));
}

// [RTR 19990712] The Embrace: "If moved to the uncontrolled region (eg. by
// Banishment), it stays a vampire and can be influenced."
#[test]
fn a_banished_embraced_vampire_can_be_influenced_again() {
    let mut e = game(2);
    let sire = e.add_minion(SeatId(0), find_card("Lolita"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Embrace, The"]);
    let before = e.state.minions.len();
    pick(&mut e, play(0, 0, sire));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let childe = MinionId(before as u32);
    // Banished (Banishment's move, by hand), with 1 blood as a counter.
    let card = e
        .state
        .cards
        .iter()
        .position(|c| c.body == Some(childe))
        .expect("its card");
    e.state.minions[childe.index()].in_play = false;
    e.state.minions[childe.index()].blood = 0;
    e.state.cards[card].in_play = false;
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: zvtes::state::CREATED,
            copy: Default::default(),
            counters: 1,
            banished: Some(zvtes::state::Banished {
                minion: childe,
                cards: vec![CardId(card as u32)],
            }),
        });
    let slot = e.state.seats[0].uncontrolled.len() as u8 - 1;
    until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::InfluenceOut { slot });
    request(&mut e);
    let v = &e.state.minions[childe.index()];
    assert!(v.in_play && v.is_vampire());
    assert_eq!(e.state.capacity_of(childe), Some(1));
}

// [LSJ 20060616-2] Web of Knives Recruit: "It is not an action to put a
// vampire in play": in the uncontrolled region with 3 training counters,
// out of play; one burns each unlock phase; the last burned, it moves to
// the ready region as a 3-capacity Independent Banu Haqim with [cel],
// [obf], [pot], [qui] and 3 blood, blood cursed.
#[test]
fn web_of_knives_recruit_trains_into_a_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Banu Haqim", 0), 4, false);
    e.add_minion(SeatId(0), plain("Banu Haqim", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Web of Knives Recruit"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let id = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.training)
            .expect("in training") as u32,
    );
    assert!(!e.state.cards[id.index()].in_play);
    assert_eq!(e.state.cards[id.index()].counters, 3);
    let minions = e.state.minions.len();
    for left in [2, 1] {
        to_minion_phase(&mut e, SeatId(1));
        to_minion_phase(&mut e, SeatId(0));
        assert_eq!(e.state.cards[id.index()].counters, left);
    }
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.minions.len(), minions + 1);
    let m = MinionId(minions as u32);
    let v = &e.state.minions[m.index()];
    assert_eq!((v.clan(), v.blood), (Some("Banu Haqim"), 3));
    assert_eq!(e.state.capacity_of(m), Some(3));
    assert_eq!(e.state.sect_of(m), Some(zvtes::crypt::Sect::Independent));
    assert!(e.state.has_discipline(m, "qui") && !e.state.has_discipline(m, "QUI"));
    assert!(e.state.blood_cursed(m));
    assert!(e.state.cards[id.index()].in_play && !e.state.cards[id.index()].training);
}

// Web of Knives Recruit: "You can burn training counters from no more than
// two Web of Knives Recruit cards each unlock phase": three in training,
// two burn, the player's picks.
#[test]
fn web_of_knives_recruits_train_two_at_a_time() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Banu Haqim", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ids: Vec<CardId> = (0..3)
        .map(|_| {
            let mut c = CardInPlay::new(find_library("Web of Knives Recruit"), SeatId(0), None, 0);
            c.in_play = false;
            c.training = true;
            c.counters = 3;
            e.state.cards.push(c);
            CardId(e.state.cards.len() as u32 - 1)
        })
        .collect();
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    for id in &ids {
        assert!(req.options.contains(&Choice::ResolveRider { card: id.0 }));
    }
    e.choose(index_of(&req, Choice::ResolveRider { card: ids[2].0 }));
    pick(&mut e, Choice::ResolveRider { card: ids[0].0 });
    to_minion_phase(&mut e, SeatId(0));
    let left: Vec<u8> = ids
        .iter()
        .map(|c| e.state.cards[c.index()].counters)
        .collect();
    assert_eq!(left, vec![2, 3, 2]);
}

// Art Scam: "+1 stealth action. Gain 2 pool." Fetish Club Hunting Ground:
// a ready vampire gains 1 blood in the unlock phase. Palla Grande: "Put
// this card in play with 3 counters. Toreador antitribu get +1 bleed.
// During your discard phase, burn 1 counter from this card. Burn this card
// if it has no counters."
#[test]
fn palla_grande_gives_toreador_antitribu_bleed_for_three_turns() {
    use zvtes::cards::Stat;
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Lolita"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let other = e.add_minion(SeatId(1), find_card("Carter"), 2, false);
    hand(&mut e, 0, &["Palla Grande"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    request(&mut e);
    let id = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(e.state.cards[id.index()].counters, 3);
    let bleeds = |e: &Engine| (e.state.current_bleed(t), e.state.current_bleed(other));
    assert_eq!(bleeds(&e), (2, 2), "every Toreador antitribu");
    let _ = Stat::Bleed;
    for left in [2, 1] {
        to_minion_phase(&mut e, SeatId(1));
        assert_eq!(e.state.cards[id.index()].counters, left);
        to_minion_phase(&mut e, SeatId(0));
    }
    to_minion_phase(&mut e, SeatId(1));
    assert!(!e.state.cards[id.index()].in_play, "burned");
    assert_eq!(bleeds(&e), (1, 1));
}

// Gird Minions: "Move any number of blood counters from your pool to 1 or
// more vampires you control." Each up to its capacity; one move, then
// another to a second vampire.
#[test]
fn gird_minions_moves_pool_to_vampires() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_capacity(5, 0), 2, false);
    let b = e.add_minion(SeatId(0), of_capacity(3, 0), 1, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Gird Minions"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    pick(&mut e, Choice::ChooseMinion(a));
    let req = request(&mut e);
    assert_eq!(req.options.len(), 3, "up to its capacity");
    pick(&mut e, Choice::SetX(3));
    pick(&mut e, Choice::ChooseMinion(b));
    pick(&mut e, Choice::SetX(2));
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::ChooseMinion(a)), "full");
    e.choose(index_of(&req, Choice::Pass));
    assert_eq!((blood(&e, a), blood(&e, b)), (5, 3));
    assert_eq!(pool(&e)[0], 10 - 5);
}

// [LSJ 20070411-2] [RTR 20000501] Mozambique Allure: "If this vampire is
// Laibon, draw 1 card from your crypt. Otherwise, this vampire becomes
// Laibon, and they unlock if they are … Ravnos" — "Cannot be played if the
// vampire is Laibon and your crypt is empty." A Ravnos becomes Laibon and
// unlocks; then, Laibon, it draws a crypt card; with an empty crypt it is
// not offered.
#[test]
fn mozambique_allure_makes_a_laibon_or_draws_a_crypt_card() {
    use zvtes::crypt::Sect;
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Ashlesha"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Mozambique Allure", "Mozambique Allure"]);
    pick(&mut e, play(0, 0, r));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.sect_of(r), Some(Sect::Laibon));
    assert!(!e.state.minions[r.index()].locked, "a Ravnos unlocks");
    e.state.minions[r.index()].acted.clear();
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, r)), "Laibon, crypt empty");
    e.state.seats[0].crypt.push(plain("Ventrue", 1));
    pick(&mut e, play(0, 0, r));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled.len(), 1);
}

// Sunset Strip, Hollywood: "Lock to give all vampires +1 stealth on hunt
// actions until your next unlock phase. Lock to reduce the cost to recruit
// a mortal ally or employ a mortal retainer by 1 blood or pool." [ANK
// 20230620] [ANK 20170605] [LSJ 20090705] from the announcement; [LSJ
// 20090705] [LSJ 20030917] [ANK 20230721] announced with the card: ECTU
// Operative costs 1 pool. [ANK 20230824] [RTR 20070707] as for an ally not
// recruited as an action: the same cost path The Shard takes there
// (rules-qa 36.5).
#[test]
fn sunset_strip_lowers_a_mortal_recruit() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let strip = in_play(&mut e, 0, "Sunset Strip, Hollywood", None);
    hand(&mut e, 0, &["ECTU Operative"]);
    pick(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert!(req.options.contains(&Choice::Pass));
    e.choose(index_of(&req, Choice::ResolveRider { card: strip.0 }));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[strip.index()].locked);
    assert_eq!(pool(&e)[0], 10 - 1);
    assert!(e.state.minions.iter().any(|m| m.is_ally()));
}

// Sunset Strip, Hollywood: "+1 stealth on hunt actions" for every
// Methuselah's vampires, until its controller's next unlock phase: seat
// 1's hunt at 2 stealth is out of reach of seat 0's 1 intercept.
#[test]
fn sunset_strip_gives_every_hunt_stealth_until_the_next_unlock() {
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), vampire_with("aus", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let h = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let strip = in_play(&mut e, 0, "Sunset Strip, Hollywood", None);
    hand(&mut e, 0, &["Eagle's Sight"]);
    pick(&mut e, use_at(strip, 0, None, Target::None));
    assert_eq!(
        e.state.seats[0]
            .timed
            .of(TimedStat::HuntStealth, Expiry::UnlockPhase),
        1
    );
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, action(h, ActionKind::Hunt));
    never_reaching_until(&mut e, SeatId(1), WindowLabel::MinionPhase, x);
    assert_eq!(blood(&e, h), 4 + 1);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(
        e.state.seats[0]
            .timed
            .of(TimedStat::HuntStealth, Expiry::UnlockPhase),
        0
    );
}

// Sunset Strip's "+1 stealth on hunt actions": Abactor's hunt too (rules-qa
// 72.3). Seat 1's Abactor at 1 + 1 stealth is out of reach of seat 0's
// Eagle's Sight (1 intercept); without the lock, within it.
#[test]
fn sunset_strip_gives_a_card_hunt_stealth() {
    for locked in [false, true] {
        let mut e = game(2);
        let x = e.add_minion(SeatId(0), vampire_with("aus", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let h = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let strip = in_play(&mut e, 0, "Sunset Strip, Hollywood", None);
        hand(&mut e, 0, &["Eagle's Sight"]);
        hand(&mut e, 1, &["Abactor"]);
        if locked {
            pick(&mut e, use_at(strip, 0, None, Target::None));
        }
        to_minion_phase(&mut e, SeatId(1));
        pick(&mut e, play(0, 0, h));
        let req = offering(&mut e, block(x));
        assert_eq!(e.block_can_reach(req.seat, x), !locked);
    }
}

// ---- Referendums ----------------------------------------------------------------

// Autarkis Persecution: "Successful referendum means each Methuselah gains
// 1 pool for each minion he or she controls." Allies and vampires in
// torpor count.
#[test]
fn autarkis_persecution_pays_each_methuselah_per_minion() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, true);
    e.add_ally(SeatId(0), find_library("Political Ally"));
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Autarkis Persecution"]);
    pick(&mut e, play(0, 0, caller));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10 + 3, 10 + 1]);
}

// The Eldest Are Kholo: "[ACTION MODIFIER] Each Laibon Ravnos gets +2
// votes. [REACTION] Each Ravnos gets +1 vote." Every Methuselah's (rules-qa
// 69.2): the caller's Laibon Ravnos and seat 1's both get 2.
#[test]
fn the_eldest_are_kholo_gives_every_laibon_ravnos_votes() {
    use zvtes::crypt::Sect;
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Ashlesha"), 3, false);
    e.state.minions[r.index()].sect_override = Some(Sect::Laibon);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let theirs = e.add_minion(SeatId(1), find_card("Sasha Miklos"), 2, false);
    e.state.minions[theirs.index()].sect_override = Some(Sect::Laibon);
    hand(
        &mut e,
        0,
        &["Autarkis Persecution", "Eldest Are Kholo, The"],
    );
    pick(&mut e, play(0, 0, r));
    pick_when(&mut e, play(0, 0, r));
    let req = offering(&mut e, votes(r, true));
    assert!(req.options.contains(&votes(r, true)));
    e.choose(index_of(&req, votes(r, true)));
    let req = offering(&mut e, votes(theirs, false));
    e.choose(index_of(&req, votes(theirs, false)));
    let ctx = e.referendum().expect("the referendum");
    assert_eq!(ctx.tally(&e.state), (2, 2));
}

// Cryptic Rider: "Only usable on a successful referendum. The next
// referendum a vampire you control calls this turn passes automatically."
// [LSJ 19980107] "During the referendum that passes automatically, no
// 'during a political action' and 'during a referendum' effect can be
// played": The Eldest Are Kholo is not offered in it. [PIB 20150105] [LSJ
// 19980107] The Eldest Are Kholo, Poison Pill: "Cannot be used during a
// referendum that is automatically passing": seat 1's Pill, offered in the
// first, is not in the second. [PIB 20150105] [LSJ
// 19980107] "Can be used after a referendum that is automatically passing":
// a second Cryptic Rider is offered after it.
#[test]
fn cryptic_rider_passes_the_next_referendum() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Ashlesha"), 3, false);
    let b = e.add_minion(SeatId(0), find_card("Sasha Miklos"), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(
        &mut e,
        0,
        &[
            "Autarkis Persecution",
            "Autarkis Persecution",
            "Cryptic Rider",
            "Cryptic Rider",
            "Eldest Are Kholo, The",
        ],
    );
    hand(&mut e, 1, &["Poison Pill"]);
    pick(&mut e, play(0, 0, a));
    let req = offering(&mut e, play(0, 0, p));
    e.choose(index_of(&req, Choice::Pass));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    // After its resolution, passed: Cryptic Rider (1 blood).
    pick_when(&mut e, play(1, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10 + 3, 10 + 1]);
    // Sasha's referendum passes with no polling step — no Kholo, no Pill,
    // no vote.
    pick(&mut e, play(0, 0, b));
    let req = loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&play(0, 0, p)), "no Poison Pill");
        if req.options.contains(&play(0, 0, b)) {
            break req;
        }
        e.choose(fallback(&req));
    };
    assert!(req.options.contains(&play(0, 0, b)), "the second Rider");
    assert!(!req.options.contains(&play(1, 0, b)), "no Kholo");
    assert_eq!(pool(&e), vec![10 + 3 + 3, 10 + 1 + 1]);
}

// [ANK 20180719-2] Poison Pill: "If the referendum passes and the effect of
// the referendum causes you to lose pool, the controller of the acting
// vampire loses the same amount" — "Any decrease in pool … is considered
// losing pool." [ANK 20180813] [LSJ 20041025] "If the effects of the
// referendum results in both gain and loss (eg. Ancient Influence), only
// the loss is counted, not the difference." [ANK 20180813] the gain, then
// the 5 burned: seat 0 loses seat 1's 5 too.
#[test]
fn poison_pill_hands_the_loss_back_to_the_caller() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), of_capacity(5, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let r = e.add_minion(SeatId(1), of_capacity(6, 0), 4, false);
    hand(&mut e, 0, &["Ancient Influence"]);
    hand(&mut e, 1, &["Poison Pill"]);
    pick(&mut e, play(0, 0, caller));
    pick_when(&mut e, play(0, 0, r));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    pick(&mut e, Choice::ChooseMinion(caller));
    pick(&mut e, Choice::ChooseMinion(r));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10 + 5 - 5 - 5, 10 + 6 - 5]);
}

// ---- Popó: Bollix, Flames of Insurrection, the Appolonius merge ------------------

/// Appolonius, base (`false`) or advanced (`true`): his CRYPT index.
fn appolonius(adv: bool) -> u16 {
    (0..zvtes::crypt::CRYPT.len() as u16)
        .find(|&c| {
            let v = &zvtes::crypt::CRYPT[c as usize];
            v.name == "Appolonius" && v.adv == adv
        })
        .expect("Appolonius")
}

/// `name` on `host`, as equipment: its id.
fn equip(e: &mut Engine, name: &str, host: MinionId) -> CardId {
    let seat = e.state.minions[host.index()].owner.0;
    in_play(e, seat, name, Some(host))
}

// [ANK 20230823] Bollix [tha]: "Does not protect from weapons if played at
// long range": a Sniper Rifle's blocker sets the range long; the Anarch's
// hand strike does not land, the rifle's 2 damage does.
#[test]
fn bollix_does_not_stop_weapons_at_long_range() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("tha", 0), 5, false);
    e.state.minions[a.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let rifle = equip(&mut e, "Sniper Rifle", z);
    hand(&mut e, 0, &["Bollix"]);
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, use_at(rifle, 1, None, Target::None));
    pick_when(&mut e, play(0, 2, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, z)), (5 - 2, 4));
}

// Bollix [pre]: "Cancel an aim, frenzy, or grapple card as it is played by
// the opposing minion, and its cost is not paid": Immortal Grapple
// canceled, to the ash heap.
#[test]
fn bollix_cancels_a_grapple_as_it_is_played() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pre", 0), 4, false);
    e.state.minions[a.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("pot", 0), 4, false);
    hand(&mut e, 0, &["Bollix"]);
    hand(&mut e, 1, &["Immortal Grapple"]);
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, play(0, 0, z));
    let req = expect(&mut e, SeatId(0), WindowLabel::CardAsPlayed);
    e.choose(index_of(&req, play(0, 1, a)));
    request(&mut e);
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Immortal Grapple")));
}

// [ANK 20180627-2] Flames of Insurrection: "When a minion opposing an
// anarch in combat leaves the ready region, the controller of that anarch
// may move a counter from the bank to a vampire in his or her uncontrolled
// region" — "If two Anarchs are in combat and both go to torpor, both
// controllers can benefit from the effect." Seat 1 controls the Flames;
// both Anarchs fall: seat 0 takes the counter, seat 1 declines.
#[test]
fn flames_of_insurrection_feeds_the_anarchs_uncontrolled_region() {
    use zvtes::crypt::Sect;
    let mut e = game(2);
    // Empty, it hunts; both fall to the round's hand strikes.
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 0, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("aus", 0), 0, false);
    e.state.minions[z.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Eagle's Sight"]);
    in_play(&mut e, 1, "Flames of Insurrection", None);
    let slot = |card| zvtes::state::UncontrolledSlot {
        card,
        copy: Default::default(),
        counters: 1,
        banished: None,
    };
    e.state.seats[0].uncontrolled.push(slot(plain("Brujah", 2)));
    e.state.seats[1]
        .uncontrolled
        .push(slot(plain("Ventrue", 2)));
    // Its mandatory hunt, declared at once.
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    let mut offers = Vec::new();
    for _ in 0..2 {
        let req = offering(&mut e, Choice::Slot(0));
        offers.push(req.seat);
        let pick_slot = req.seat == SeatId(0);
        e.choose(index_of(
            &req,
            if pick_slot {
                Choice::Slot(0)
            } else {
                Choice::Pass
            },
        ));
    }
    offers.sort();
    assert_eq!(offers, vec![SeatId(0), SeatId(1)]);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 2);
    assert_eq!(e.state.seats[1].uncontrolled[0].counters, 1);
}

// Rulebook "Advanced": "if you already control the advanced or the
// associated regular 'base' vampire and the other version of the vampire
// is in your uncontrolled region, then you can spend 4 transfers and 1
// pool counter to move the vampire card … The counters … on the vampire
// coming from the uncontrolled region are burned … When merged, the text
// on the base card still applies … The advanced card applies in full …
// the advanced card has precedence." Appolonius merged: Anarch, Baron of
// London (his [MERGED] title), the base card's press and bleed, the
// advanced card's rush; influencing the other version out is not offered.
#[test]
fn appolonius_merges_in_the_influence_phase() {
    use zvtes::cards::Stat;
    use zvtes::crypt::{Sect, Title};
    let mut e = game(2);
    let base = appolonius(false);
    let adv = appolonius(true);
    let ap = e.add_minion(SeatId(0), base, 5, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: adv,
            copy: Default::default(),
            counters: 3,
            banished: None,
        });
    let req = to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    assert!(!req.options.contains(&Choice::InfluenceOut { slot: 0 }));
    e.choose(index_of(&req, Choice::Merge { slot: 0 }));
    assert!(
        e.state.seats[0].uncontrolled.is_empty(),
        "its counters burned"
    );
    assert_eq!(pool(&e)[0], 10 - 1);
    let v = &e.state.minions[ap.index()];
    assert_eq!(
        (v.crypt_card(), v.merged, v.blood),
        (Some(adv), Some(base), 5)
    );
    assert_eq!(e.state.sect_of(ap), Some(Sect::Anarch));
    assert_eq!(e.state.title_of(ap), Title::Baron("London"));
    assert_eq!(
        e.state
            .printed_mod(ap, Stat::CombatPress, Default::default()),
        1
    );
    to_minion_phase(&mut e, SeatId(0));
    let them = MinionId(1);
    let req = request(&mut e);
    assert!(
        req.options.contains(&printed(ap, 0, Target::Minion(them))),
        "the rush"
    );
}

// Appolonius (advanced): "+1 bleed for each titled vampire controlled by
// the target Methuselah": 2 titled (one in torpor): 1 + 2; merged, the base
// card's +1 too.
#[test]
fn appolonius_bleeds_for_each_titled_vampire_of_his_target() {
    let adv = appolonius(true);
    for merged in [false, true] {
        let mut e = game(2);
        let ap = e.add_minion(SeatId(0), adv, 5, false);
        if merged {
            e.state.minions[ap.index()].merged = Some(appolonius(false));
        }
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), find_card("Gratiano"), 4, false);
        e.add_minion(SeatId(1), find_card("Antonio Delgado"), 4, true);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        pick(&mut e, bleed(ap));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e)[1], 10 - 3 - i32::from(merged));
    }
}

// Legacy: "+2 stealth action. Requires an unmerged advanced vampire.
// Unlock this vampire, search your crypt (shuffle afterward), uncontrolled
// region, and/or ash heap for their base version, and merge them with it."
// The base found in the crypt: merged, unlocked. Burned later, both cards
// go to the ash heap.
#[test]
fn legacy_merges_an_advanced_vampire_with_its_base() {
    let adv = appolonius(true);
    let base = appolonius(false);
    let mut e = game(2);
    let ap = e.add_minion(SeatId(0), adv, 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let d = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.seats[0].crypt = vec![plain("Brujah", 1), base, plain("Brujah", 2)].into();
    hand(&mut e, 0, &["Legacy"]);
    pick(&mut e, play(0, 0, ap));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(req.options.contains(&Choice::Pass));
    e.choose(index_of(&req, Choice::AshVampire(base)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[ap.index()];
    assert_eq!(v.merged, Some(base));
    assert!(!v.locked);
    assert_eq!(e.state.seats[0].crypt.len(), 2);
    // Diablerized: both cards to the ash heap.
    to_minion_phase(&mut e, SeatId(1));
    e.state.minions[ap.index()].in_torpor = true;
    pick(
        &mut e,
        Choice::TakeAction {
            minion: d,
            action: ActionKind::Diablerie,
            target: Some(ap),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let ash = &e.state.seats[0].ash_crypt;
    assert!(ash.contains(&adv) && ash.contains(&base));
}

// [RTR 20020501] [LSJ 19980109] Shadow Body [OBT]: "If the combat continues
// or a new combat begins, the after combat effect does not happen":
// Telepathic Tracking [AUS] starts a new round; the blocked Art Scam is
// not continued.
#[test]
fn shadow_body_does_not_continue_if_the_combat_continues() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Lolita"), 3, false);
    superior(&mut e, t, "obt");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
    hand(&mut e, 0, &["Art Scam", "Shadow Body"]);
    hand(&mut e, 1, &["Eagle's Sight", "Telepathic Tracking"]);
    pick(&mut e, play(0, 0, t));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 1, t));
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
    assert_eq!(pool(&e)[0], 10, "no Art Scam");
}

// Tenebrous Form [OBT] beside Where the Veil Thins [OBL]: each its toll —
// a minion with neither Discipline burns 2 blood to attempt to block, the
// second as the second card is played (rules-qa 69.12).
#[test]
fn tenebrous_form_and_where_the_veil_thins_add_their_tolls() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 4, false);
    superior(&mut e, a, "obt");
    superior(&mut e, a, "obl");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    hand(&mut e, 0, &["Tenebrous Form", "Where the Veil Thins"]);
    hand(&mut e, 1, &["Enhanced Senses", "Eagle's Sight"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    pick(&mut e, Choice::PayBlockToll);
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 1, a));
    pick(&mut e, Choice::PayBlockToll);
    assert_eq!(blood(&e, z), 4 - 2);
}
