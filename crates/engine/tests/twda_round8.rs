//! Legacy cards for TWDA decks, round 8 (`ADDED_BATCHES`, the eighteenth
//! batch, slice 16t): Blood Brother circles — the cards of the ten Blood
//! Brother TWDA decks. Readings in rules-qa Batch 68. Windows with a single
//! option are elided, so the scripts only stop where a real decision
//! exists.

use zvtes::cards::{self, AbilitySource, Target};
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

fn play_at(card: u8, mode: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target,
    }
}

fn use_at(card: CardId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(card),
        ability,
        actor,
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

/// A vampire with no definition of its own, of capacity `cap` or more.
fn of_capacity_at_least(cap: u8) -> u16 {
    zvtes::crypt::CRYPT
        .iter()
        .enumerate()
        .find(|(i, c)| c.capacity >= cap && zvtes::cards::vampire(*i as u16).is_none())
        .map(|(i, _)| i as u16)
        .expect("a vampire of the capacity")
}

/// `a` (seat 0) bleeds; `z` (seat 1) blocks: the combat begins.
fn blocked_bleed(e: &mut Engine, a: MinionId, z: MinionId) {
    pick(e, bleed(a));
    pick_when(e, block(z));
}

/// A Hermana Hambrienta Mayor (Mexico City Circle) for `seat`, with `blood`.
fn hermana(e: &mut Engine, seat: u8, blood: u8) -> MinionId {
    e.add_minion(
        SeatId(seat),
        find_card("Hermana Hambrienta Mayor"),
        blood,
        false,
    )
}

/// A New Blood for `seat` who chose no circle: a circle of one.
fn new_blood(e: &mut Engine, seat: u8, blood: u8) -> MinionId {
    e.add_minion(SeatId(seat), find_card("New Blood"), blood, false)
}

// ---- Circles ------------------------------------------------------------------

// [LSJ 20020609] "A vampire with no circle designation is in his own unique
// circle. Vampires with the same name (eg. The Embrace, non-unique
// vampires) do not share a circle by default": two Hermanas share the
// printed Mexico City Circle; a New Blood who chose none shares no circle
// (not even with another New Blood); one who chose Mexico City does.
#[test]
fn a_blood_brother_with_no_circle_is_in_a_circle_of_one() {
    let mut e = game(2);
    let a = hermana(&mut e, 0, 4);
    let b = hermana(&mut e, 0, 4);
    let n1 = new_blood(&mut e, 0, 2);
    let n2 = new_blood(&mut e, 0, 2);
    let st = &e.state;
    assert!(st.same_circle(a, b));
    assert!(!st.same_circle(a, a), "another one");
    assert!(!st.same_circle(a, n1));
    assert!(!st.same_circle(n1, n2));
    e.state.minions[n1.index()].chosen_circle = Some("Mexico City");
    assert!(e.state.same_circle(a, n1));
    // Camaraderie [SAN] with New Blood alone beside the reacting Hermana:
    // not offered; Walk of Caine offers no other pick.
    e.state.minions[n1.index()].chosen_circle = None;
    superior(&mut e, a, "san");
    hand(&mut e, 0, &["Camaraderie"]);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    // Seat 0 is bled by its predator in a 2-seat game.
    e.state.minions[b.index()].locked = true;
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 1, a));
}

// ---- Step 1: Las hermanas de sangre -------------------------------------------

// Blood Buff: "Trifle. Put this card on a ready Blood Brother you control.
// This Blood Brother gets +1 level of Sanguinus [san]." On a Hermana
// (basic [san]): superior, her capacity unchanged; not on another
// Methuselah's Blood Brother nor on a vampire of another clan.
#[test]
fn blood_buff_raises_a_blood_brothers_sanguinus() {
    let mut e = game(2);
    let h = hermana(&mut e, 0, 4);
    let v = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let o = hermana(&mut e, 1, 4);
    hand(&mut e, 0, &["Blood Buff"]);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let on = |m| Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(m),
    };
    assert!(!req.options.contains(&on(v)));
    assert!(!req.options.contains(&on(o)));
    assert!(!e.state.has_discipline(h, "SAN"));
    e.choose(index_of(&req, on(h)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.has_discipline(h, "SAN"));
    assert_eq!(e.state.capacity_of(h), Some(4));
}

// Camaraderie: "[san] Reduce a bleed against you by 1. [SAN] Only usable if
// you control another ready unlocked Blood Brother of the same circle as
// this Blood Brother. Reduce a bleed against you by 3." Seat 1 bleeds seat
// 0 for 4: [SAN] beside an unlocked Hermana reduces it to 1; that Hermana
// locked, only [san] is offered.
#[test]
fn camaraderie_reduces_more_beside_the_circle() {
    for other_unlocked in [true, false] {
        let mut e = game(2);
        let a = hermana(&mut e, 0, 4);
        superior(&mut e, a, "san");
        let b = hermana(&mut e, 0, 4);
        e.state.minions[b.index()].locked = !other_unlocked;
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Camaraderie"]);
        e.state.minions[z.index()].bleed_bonus_turn = 3;
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        e.state.minions[z.index()].bleed_bonus_turn = 3;
        e.state.minions[b.index()].locked = !other_unlocked;
        pick(&mut e, bleed(z));
        let req = offering(&mut e, play(0, 0, a));
        assert_eq!(req.options.contains(&play(0, 1, a)), other_unlocked);
        let pool = e.state.seats[0].pool;
        e.choose(index_of(&req, play(0, other_unlocked as u8, a)));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let lost = pool - e.state.seats[0].pool;
        assert_eq!(lost, if other_unlocked { 1 } else { 3 });
    }
}

// Claiming the Body: "Requires a ready unlocked Blood Brother of the same
// circle as another one who has just been blocked, before block
// resolution. [san] Lock the blocking minion and end the action. [SAN] As
// above, and unlock the acting Blood Brother." [RTR 20180719] [ANK
// 20200207] "The action ends unsuccessfully immediately, no other action
// modifier or reaction can be played afterwards": a Hermana's bleed is
// blocked; the other Hermana plays it — the blocker locked, no combat, no
// bleed; at [SAN] the acting Hermana unlocked.
#[test]
fn claiming_the_body_ends_a_blocked_action() {
    for sup in [false, true] {
        let mut e = game(2);
        let a = hermana(&mut e, 0, 4);
        let b = hermana(&mut e, 0, 4);
        if sup {
            superior(&mut e, b, "san");
        }
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[z.index()].locked = false;
        hand(&mut e, 0, &["Claiming the Body", "Shell Game"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, sup as u8, b));
        // Nothing more is played: no Shell Game, no combat.
        let req = request(&mut e);
        assert_eq!(req.window, WindowLabel::MinionPhase, "{:?}", req.options);
        assert!(e.state.minions[z.index()].locked);
        assert_eq!(e.state.minions[a.index()].locked, !sup);
        assert_eq!(blood(&e, a), 4);
        assert_eq!(blood(&e, b), 3, "its blood cost");
        assert_eq!(pool(&e)[1], 10);
    }
}

// Hermana Hambrienta: "When moved to the ready region in your influence
// phase, this vampire burns 2 blood or is burned." [LSJ 20051126] "If
// influenced with more blood than her capacity, the blood drain off before
// her ability forces her to burn 2": each Hermana influenced out with 2
// counters over her capacity burns 2 of her capacity; the other choice
// burns her.
#[test]
fn a_hermana_burns_two_blood_as_she_enters_play() {
    for (name, capacity) in [
        ("Hermana Hambrienta Mayor", 4),
        ("Hermana Hambrienta Menor", 2),
    ] {
        for keep in [true, false] {
            let mut e = game(2);
            e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
            e.state.seats[0]
                .uncontrolled
                .push(zvtes::state::UncontrolledSlot {
                    card: find_card(name),
                    copy: Default::default(),
                    counters: capacity + 2,
                    banished: None,
                });
            let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
            e.choose(index_of(&req, Choice::InfluenceOut { slot: 0 }));
            let req = request(&mut e);
            let h = MinionId(e.state.minions.len() as u32 - 1);
            let pick = if keep {
                Choice::BurnBloodToAvoid
            } else {
                req.options
                    .iter()
                    .copied()
                    .find(|o| *o != Choice::BurnBloodToAvoid)
                    .expect("the burn")
            };
            e.choose(index_of(&req, pick));
            if keep {
                assert_eq!(blood(&e, h), capacity - 2, "{name}");
                assert!(e.state.minions[h.index()].in_play);
            } else {
                assert!(!e.state.minions[h.index()].in_play, "{name}");
            }
        }
    }
}

// Hive Mind: "+1 stealth action. [san] Unlock this Blood Brother and
// another ready Blood Brother of the same circle. Only one Hive Mind can
// be played at [san] each turn. [SAN] Add 3 blood to a Blood Brother of the
// same circle in your uncontrolled region." [san]: both Hermanas unlocked
// after the action, a second [san] not offered the same turn; [SAN]: 3
// blood on the uncontrolled Hermana, not on an uncontrolled vampire of
// another clan.
#[test]
fn hive_mind_unlocks_the_circle_or_feeds_it() {
    let mut e = game(2);
    let a = hermana(&mut e, 0, 4);
    let b = hermana(&mut e, 0, 4);
    let c = hermana(&mut e, 0, 4);
    superior(&mut e, c, "san");
    e.state.minions[b.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].uncontrolled = vec![
        zvtes::state::UncontrolledSlot {
            card: find_card("Hermana Hambrienta Menor"),
            copy: Default::default(),
            counters: 0,
            banished: None,
        },
        zvtes::state::UncontrolledSlot {
            card: plain("Ventrue", 2),
            copy: Default::default(),
            counters: 0,
            banished: None,
        },
    ];
    hand(&mut e, 0, &["Hive Mind", "Hive Mind", "Hive Mind"]);
    pick(&mut e, play_at(0, 0, a, Target::Minion(b)));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked);
    assert!(!e.state.minions[b.index()].locked);
    assert!(!req.options.contains(&play_at(0, 0, c, Target::Minion(a))));
    assert!(!req
        .options
        .contains(&play_at(0, 1, c, Target::Uncontrolled(1))));
    e.choose(index_of(&req, play_at(0, 1, c, Target::Uncontrolled(0))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 3);
}

// Shell Game: "Only usable when the action is announced. [san] Lock X other
// ready Blood Brothers you control of the same circle as this one to give
// this Blood Brother +X stealth (even if stealth is not yet needed). If the
// action is successful, you may unlock one of those Blood Brothers." A
// Hermana's bleed: the other two locked, +2 stealth past a +1 intercept
// blocker; successful, one of them unlocked.
#[test]
fn shell_game_locks_the_circle_for_stealth() {
    let mut e = game(2);
    let a = hermana(&mut e, 0, 4);
    let b = hermana(&mut e, 0, 4);
    let c = hermana(&mut e, 0, 4);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Shell Game"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    pick(&mut e, Choice::ChooseMinion(b));
    // The last candidate picked, the picks end.
    pick(&mut e, Choice::ChooseMinion(c));
    // +2 stealth: no block attempt is offered to the blocker with none.
    let _ = z;
    let req = offering(&mut e, Choice::UnlockForBlood { minion: b });
    assert!(e.state.minions[b.index()].locked);
    assert!(e.state.minions[c.index()].locked);
    assert!(req.options.contains(&Choice::UnlockForBlood { minion: c }));
    e.choose(index_of(&req, Choice::UnlockForBlood { minion: b }));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[b.index()].locked);
    assert!(e.state.minions[c.index()].locked, "one only");
    assert_eq!(pool(&e)[1], 10 - 1);
}

// Shell Game, Walk of Caine [ANK 20190425] "Must be played (as well as other
// "as the action is announced" action modifiers) before action modifiers
// that are not explicitly played "as the action is announced"" and [ANK
// 20181208] "Replace the action card (if any) before playing it": Zillah's
// Tears announced, its replacement is drawn as Shell Game is offered; a
// bleed's Walk of Caine is offered as it is announced, and no more once
// that window is passed.
#[test]
fn as_announced_circle_modifiers_come_first() {
    let mut e = game(2);
    let a = hermana(&mut e, 0, 4);
    let b = hermana(&mut e, 0, 4);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(
        &mut e,
        0,
        &[
            "Zillah's Tears",
            "Shell Game",
            "Walk of Caine",
            "Deflection",
            "Deflection",
            "Deflection",
            "Deflection",
        ],
    );
    e.state.seats[0].library = vec![find_library("Deflection")].into();
    pick(&mut e, play(0, 0, a));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.window, WindowLabel::ActionAsAnnounced);
    assert!(e.state.seats[0].library.is_empty(), "replaced");
    e.choose(index_of(&req, Choice::Pass));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.choose(index_of(&req, bleed(b)));
    let walk = play(1, 0, b);
    let req = offering(&mut e, walk);
    assert_eq!(req.window, WindowLabel::ActionAsAnnounced);
    e.choose(index_of(&req, Choice::Pass));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, walk);
}

// Slake the Thirst: "Only usable after this Blood Brother gains blood. [SAN]
// [ACTION MODIFIER] As above, but usable as an action modifier." [LSJ
// 20100325] "[SAN] Can be played at any time blood is gained during an
// action, including after action resolution": a Hermana hunts (+1 blood);
// after resolution Slake gives the other Hermana as much; before the hunt
// gained anything, not offered.
#[test]
fn slake_the_thirst_shares_a_hunt() {
    let mut e = game(2);
    let a = hermana(&mut e, 0, 2);
    superior(&mut e, a, "san");
    let b = hermana(&mut e, 0, 2);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Slake the Thirst"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    let req = offering(&mut e, play_at(0, 2, a, Target::Minion(b)));
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    assert_eq!(blood(&e, a), 3);
    e.choose(index_of(&req, play_at(0, 2, a, Target::Minion(b))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 3);
}

// Slake the Thirst [LSJ 20100206-2] "[san] Can be played multiple times on
// the same blood gain": a blocked Hermana steals 1 blood with Theft of
// Vitae (its [tha] the fixture's) as she takes 1 damage — a gain all the
// same; before the strike, Slake is not offered; as the strikes end,
// played once for each other Hermana on the same gain.
#[test]
fn slake_the_thirst_again_on_the_same_gain() {
    let mut e = game(2);
    let a = hermana(&mut e, 0, 2);
    superior(&mut e, a, "tha");
    let b = hermana(&mut e, 0, 2);
    let c = hermana(&mut e, 0, 2);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(
        &mut e,
        0,
        &["Theft of Vitae", "Slake the Thirst", "Slake the Thirst"],
    );
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, play(0, 0, a));
    assert!(!req.options.contains(&play_at(1, 0, a, Target::Minion(b))));
    e.choose(index_of(&req, play(0, 0, a)));
    let req = offering(&mut e, play_at(0, 0, a, Target::Minion(b)));
    // 1 stolen, 1 lost to the hand strike: a gain all the same.
    assert_eq!(blood(&e, a), 2);
    assert_eq!(req.window, WindowLabel::CombatAdditionalStrikes);
    e.choose(index_of(&req, play_at(0, 0, a, Target::Minion(b))));
    let req = offering(&mut e, play_at(0, 0, a, Target::Minion(c)));
    e.choose(index_of(&req, play_at(0, 0, a, Target::Minion(c))));
    request(&mut e);
    assert_eq!(blood(&e, b), 3);
    assert_eq!(blood(&e, c), 3);
}

// Walk of Caine: "[san] Only usable as a bleed action is announced. Choose
// X ready Blood Brothers you control with 1 or more blood of the same
// circle as this Blood Brother, including this Blood Brother. Each chosen
// Blood Brother burns 1 blood. +X bleed (limited)." The acting Hermana and
// another: each burns 1, the bleed is 3; a Hermana with no blood is not
// offered.
#[test]
fn walk_of_caine_bleeds_with_the_circle() {
    let mut e = game(2);
    let a = hermana(&mut e, 0, 4);
    let b = hermana(&mut e, 0, 4);
    let c = hermana(&mut e, 0, 1);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Walk of Caine"]);
    pick(&mut e, bleed(a));
    e.state.minions[c.index()].blood = 0;
    pick_when(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::ChooseMinion(c)));
    e.choose(index_of(&req, Choice::ChooseMinion(a)));
    pick(&mut e, Choice::ChooseMinion(b));
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 3);
    assert_eq!(blood(&e, b), 3);
    assert_eq!(pool(&e)[1], 10 - 3);
}

// Zillah's Tears: "+1 stealth action. Requires a ready Sabbat vampire. Put
// this card on this acting vampire and move 1 blood from the blood bank to
// this vampire. This Sabbat vampire can burn this card any time after this
// turn ends to unlock. A vampire can have only one Zillah's Tears." On the
// Hermana with 1 blood more; not usable the same turn; a second not
// playable on her; the next turn, burned to unlock her.
#[test]
fn zillahs_tears_unlocks_a_later_turn() {
    let mut e = game(2);
    let a = hermana(&mut e, 0, 2);
    hermana(&mut e, 0, 2);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Zillah's Tears", "Zillah's Tears"]);
    pick(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 3);
    let tears = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(e.state.cards[tears.index()].host, Some(a));
    assert!(!req.options.contains(&use_at(tears, 0, None, Target::None)));
    // Locked into the next turn: its controller burns it to unlock her.
    e.state.minions[a.index()].skip_unlock = true;
    let req = offering(&mut e, use_at(tears, 0, None, Target::None));
    assert!(e.state.turn > 4);
    e.choose(index_of(&req, use_at(tears, 0, None, Target::None)));
    request(&mut e);
    assert!(!e.state.minions[a.index()].locked);
    assert!(!e.state.cards[tears.index()].in_play);
    let _ = CardInPlay::new;
}

// Slake the Thirst [LSJ 20100203] "If multiple effects make you gain blood,
// even due to a single card, you choose on which effect to apply Slake the
// Thirst": a Hermana hunts (+1), then gains 2 more by another effect in
// the same window; Slake answers either, the other Hermana gaining 2.
#[test]
fn slake_the_thirst_answers_one_gain_of_several() {
    let mut e = game(2);
    let a = hermana(&mut e, 0, 1);
    superior(&mut e, a, "san");
    let b = hermana(&mut e, 0, 1);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Slake the Thirst"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    let slake = play_at(0, 2, a, Target::Minion(b));
    offering(&mut e, slake);
    // Another effect's gain, in the same window.
    e.state.gain_blood(a, 2);
    let req = offering(&mut e, slake);
    e.choose(index_of(&req, slake));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::PlayTerms);
    assert!(req.options.contains(&Choice::SetX(1)));
    e.choose(index_of(&req, Choice::SetX(2)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 3);
}

// ---- Step 2: Blood Brother Torrance Sabbat Tricks -----------------------------

fn torrance(e: &mut Engine, seat: u8, name: &str, blood: u8) -> MinionId {
    e.add_minion(SeatId(seat), find_card(name), blood, false)
}

// Jack: "You may lock Jack when you play a Master: location (unique or not)
// to reduce the cost of that location by 1 pool." [LSJ 20011022] "The
// ability can be used when in torpor": Jack in torpor locks as Channel 10
// (2 pool) is played, which costs 1; not offered for a non-location.
#[test]
fn jack_locks_for_a_cheaper_location() {
    let mut e = game(2);
    let jack = torrance(&mut e, 0, "Jack", 3);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Channel 10"]);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    e.state.minions[jack.index()].in_torpor = true;
    e.state.minions[jack.index()].locked = false;
    let pool = e.state.seats[0].pool;
    e.choose(index_of(&req, master_play(0)));
    let lock = Choice::ResolvePrinted { minion: jack };
    let req = request(&mut e);
    assert!(req.options.contains(&lock), "{:?}", req.options);
    e.choose(index_of(&req, lock));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, pool - 1);
    assert!(e.state.minions[jack.index()].locked);
}

// Mark: "Mark gets +1 intercept when attempting to block a bleed action."
// Seat 1's bleed at seat 0: his intercept is 1; seat 1's Zillah's Tears (a
// +1 stealth undirected action): his 0 intercept cannot reach it — the
// attempt is never offered.
#[test]
fn mark_intercepts_bleeds() {
    for bleeding in [true, false] {
        let mut e = game(2);
        let mark = torrance(&mut e, 0, "Mark", 3);
        e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        let z = hermana(&mut e, 1, 4);
        hermana(&mut e, 1, 4);
        hand(&mut e, 1, &["Zillah's Tears"]);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        if bleeding {
            pick(&mut e, bleed(z));
            let req = until(&mut e, SeatId(0), WindowLabel::ActionBlock);
            assert!(req.options.contains(&block(mark)));
            let i = e.action().expect("action").intercept_of(&e.state, mark);
            assert_eq!(i, 1);
        } else {
            pick(&mut e, play(0, 0, z));
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(mark));
        }
    }
}

// Octopod: "Requires a ready Blood Brother of the same circle as another
// one in combat. Usable by a vampire not involved in the combat. [san] Only
// usable before range is chosen. The other Blood Brother gets an additional
// strike (limited) each round. [SAN] … two additional strikes." Ilse's
// bleed is blocked: Karl gives her one more hand strike, two at [SAN] (his
// superior, the fixture's) — the blocker loses 2 or 3; a Hermana, of
// another circle, may not play it [LSJ 20020609].
#[test]
fn octopod_gives_strikes_each_round() {
    for sup in [false, true] {
        let mut e = game(2);
        let ilse = torrance(&mut e, 0, "Ilse", 6);
        let karl = torrance(&mut e, 0, "Karl", 6);
        if sup {
            superior(&mut e, karl, "san");
        }
        let h = hermana(&mut e, 0, 4);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
        hand(&mut e, 0, &["Octopod"]);
        blocked_bleed(&mut e, ilse, z);
        let req = offering(&mut e, play(0, sup as u8, karl));
        let before = blood(&e, z);
        assert!(!req.options.contains(&play(0, 0, h)));
        e.choose(index_of(&req, play(0, sup as u8, karl)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, z), before - 2 - sup as u8);
        assert_eq!(blood(&e, karl), 5, "its blood cost");
    }
}

// Oppugnant Night: "+1 stealth action. Only one Oppugnant Night can be
// played each turn. Put this card in play and unlock this Blood Brother.
// Each Blood Brother of this circle (including this one) can enter combat
// with a minion as a Ⓓ action. Burn this card at the end of this minion
// phase." [LSJ 20080604] "The action provided by this card is considered
// to be an action requiring the listed clan": Ilse plays it and unlocks;
// Karl and Ilse may rush, the Hermana (another circle, [LSJ 20020609]) and
// a Ventrue not; a second is not playable this turn; the card burns as the
// minion phase ends.
#[test]
fn oppugnant_night_lets_the_circle_rush() {
    let mut e = game(2);
    let ilse = torrance(&mut e, 0, "Ilse", 6);
    let karl = torrance(&mut e, 0, "Karl", 6);
    let h = hermana(&mut e, 0, 4);
    let v = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 1), 8, false);
    hand(&mut e, 0, &["Oppugnant Night", "Oppugnant Night"]);
    pick(&mut e, play(0, 0, ilse));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ilse.index()].locked);
    let night = CardId(e.state.cards.len() as u32 - 1);
    assert!(e.state.cards[night.index()].in_play);
    let rush = |m| Choice::UseAbility {
        source: AbilitySource::Card(night),
        ability: 0,
        actor: Some(m),
        target: Target::Minion(z),
    };
    assert!(req.options.contains(&rush(karl)));
    assert!(req.options.contains(&rush(ilse)));
    assert!(!req.options.contains(&rush(h)));
    assert!(!req.options.contains(&rush(v)));
    assert!(!req.options.contains(&play(0, 0, karl)), "one each turn");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[night.index()].in_play);
}

// Three's a Crowd: "Requires a ready unlocked Blood Brother of the same
// circle as another one you control in combat. Only usable before range is
// determined. The range this round is automatically close, and the Blood
// Brother in combat gets an optional press, only usable to continue
// combat." [TOM 19960521] "The optional press can only be used during the
// current round": Karl plays it for Ilse — close range, a press to
// continue banked; a locked Truman cannot play it, nor a vampire of another
// circle [LSJ 20020609] (the Hermana).
#[test]
fn threes_a_crowd_closes_and_presses() {
    let mut e = game(2);
    let ilse = torrance(&mut e, 0, "Ilse", 6);
    let karl = torrance(&mut e, 0, "Karl", 6);
    let truman = torrance(&mut e, 0, "Truman", 6);
    e.state.minions[truman.index()].locked = true;
    let h = hermana(&mut e, 0, 4);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    hand(&mut e, 0, &["Three's a Crowd"]);
    blocked_bleed(&mut e, ilse, z);
    let req = offering(&mut e, play(0, 0, karl));
    assert!(!req.options.contains(&play(0, 0, truman)));
    assert!(!req.options.contains(&play(0, 0, h)));
    e.choose(index_of(&req, play(0, 0, karl)));
    request(&mut e);
    let c = e.combat().expect("combat");
    let i = c.side_of_minion(ilse).expect("ilse fights");
    assert_eq!(c.range, zvtes::combat::Range::Close);
    assert_eq!(c.sides[i].continue_presses, 1);
}

// ---- Step 3: Lisbon (Coagulated Entity, Coordinate Attacks, Unwholesome Bond) --

// Coagulated Entity: "Choose X other ready Blood Brothers you control who
// belong to the same circle as this acting vampire. [san] Ⓓ Enter combat
// with a ready minion controlled by another Methuselah. This acting
// vampire has +X strength and +X capacity during this action. Before the
// combat begins, each chosen vampire must move 1 blood to the acting
// vampire or this action is canceled." Ilse picks Karl and Truman (not the
// Hermana, of another circle [LSJ 20020609]): 2 blood moved to her, +2
// strength — her hand strike does 3; with Truman emptied before the
// resolution, the action is canceled, no combat.
#[test]
fn coagulated_entity_feeds_the_rush() {
    for paid in [true, false] {
        let mut e = game(2);
        let ilse = torrance(&mut e, 0, "Ilse", 6);
        let karl = torrance(&mut e, 0, "Karl", 3);
        let truman = torrance(&mut e, 0, "Truman", 3);
        let h = hermana(&mut e, 0, 4);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
        hand(&mut e, 0, &["Coagulated Entity"]);
        pick(&mut e, play_at(0, 0, ilse, Target::Minion(z)));
        let req = request(&mut e);
        assert!(!req.options.contains(&Choice::ChooseMinion(h)));
        e.choose(index_of(&req, Choice::ChooseMinion(karl)));
        pick(&mut e, Choice::ChooseMinion(truman));
        if !paid {
            e.state.minions[truman.index()].blood = 0;
        }
        let before = blood(&e, z);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if paid {
            assert_eq!(blood(&e, karl), 2);
            assert_eq!(blood(&e, truman), 2);
            assert_eq!(before - blood(&e, z), 3, "+2 strength");
        } else {
            assert_eq!(blood(&e, karl), 3);
            assert_eq!(blood(&e, z), before, "no combat");
        }
    }
}

// Coagulated Entity [SAN] "this vampire may prevent up to X damage during
// that combat"; [RTR 20041202] "If fewer points of (preventable) damage are
// being resolved, then the effect prevents all of those points" and [ANK
// 20200318] [LSJ 20001114] "Unused prevention points can't be carried over unless
// optional ('can/may prevent')": X = 2 against a 1-damage hand strike, the
// point prevented, one left for later in the combat.
#[test]
fn coagulated_entity_prevents_its_x() {
    let mut e = game(2);
    let ilse = torrance(&mut e, 0, "Ilse", 3);
    superior(&mut e, ilse, "san");
    let karl = torrance(&mut e, 0, "Karl", 3);
    let truman = torrance(&mut e, 0, "Truman", 3);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    hand(&mut e, 0, &["Coagulated Entity"]);
    pick(&mut e, play_at(0, 1, ilse, Target::Minion(z)));
    pick(&mut e, Choice::ChooseMinion(karl));
    pick(&mut e, Choice::ChooseMinion(truman));
    let req = offering(&mut e, Choice::UsePrevent);
    e.choose(index_of(&req, Choice::UsePrevent));
    let c = e.combat().expect("combat");
    let i = c.side_of_minion(ilse).expect("ilse fights");
    assert_eq!(c.sides[i].combat_prevents, 1, "one kept");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ilse), 3 + 2, "the damage prevented");
}

// Coordinate Attacks: "Requires a ready unlocked Blood Brother of the same
// circle as another one you control in combat. [san] Only usable at the end
// of a round if the minion opposing the Blood Brother you control is still
// ready and combat is about to end. After this round, this Blood Brother
// locks and enters combat with the opposing minion. The first round of
// combat, the opposing minion cannot play any strike cards." [RTR 20030519]
// "Must be played after the presses are handled": Ilse's combat would end;
// Karl plays it (the Hermana, of another circle, may not [LSJ 20020609]),
// locks and fights the blocker, who may not use its Theft of Vitae that
// first round.
#[test]
fn coordinate_attacks_sends_the_next_one_in() {
    let mut e = game(2);
    let ilse = torrance(&mut e, 0, "Ilse", 6);
    let karl = torrance(&mut e, 0, "Karl", 6);
    let h = hermana(&mut e, 0, 4);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    superior(&mut e, z, "tha");
    hand(&mut e, 0, &["Coordinate Attacks", "Theft of Vitae"]);
    superior(&mut e, karl, "tha");
    hand(&mut e, 1, &["Theft of Vitae"]);
    blocked_bleed(&mut e, ilse, z);
    let req = offering(&mut e, play(0, 0, karl));
    assert_eq!(req.window, WindowLabel::CombatEndOfRound);
    assert!(!req.options.contains(&play(0, 0, h)));
    e.choose(index_of(&req, play(0, 0, karl)));
    // The new combat: Karl's strike decision, Theft of Vitae offered him.
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    let c = e.combat().expect("the new combat");
    assert!(c.side_of_minion(karl).is_some() && c.side_of_minion(z).is_some());
    assert!(e.state.minions[karl.index()].locked);
    assert!(req.options.contains(&play(0, 0, karl)));
    let zi = c.side_of_minion(z).expect("z");
    assert_eq!(c.sides[zi].no_strike_cards_round, Some(1));
}

// Unwholesome Bond: "+1 stealth action. [san] This vampire gains X+1 blood,
// where X is the number of ready, unlocked Blood Brothers of the same
// circle as the acting vampire. [SAN] Add 1 blood from the blood bank to
// each Blood Brother in your uncontrolled region of the same circle as the
// acting vampire." [san]: Karl and Truman unlocked, Mark locked, a Hermana
// (another circle [LSJ 20020609]): Ilse gains 3; [SAN], Truman's: the
// uncontrolled Jack gets 1, the uncontrolled Hermana none.
#[test]
fn unwholesome_bond_counts_the_circle() {
    let mut e = game(2);
    let ilse = torrance(&mut e, 0, "Ilse", 2);
    superior(&mut e, ilse, "san");
    torrance(&mut e, 0, "Karl", 4);
    let truman = torrance(&mut e, 0, "Truman", 4);
    let mark = torrance(&mut e, 0, "Mark", 4);
    e.state.minions[mark.index()].locked = true;
    hermana(&mut e, 0, 4);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].uncontrolled = vec![
        zvtes::state::UncontrolledSlot {
            card: find_card("Jack"),
            copy: Default::default(),
            counters: 0,
            banished: None,
        },
        zvtes::state::UncontrolledSlot {
            card: find_card("Hermana Hambrienta Menor"),
            copy: Default::default(),
            counters: 0,
            banished: None,
        },
    ];
    hand(&mut e, 0, &["Unwholesome Bond", "Unwholesome Bond"]);
    e.state.minions[mark.index()].locked = true;
    pick(&mut e, play(0, 0, ilse));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ilse), 2 + 3);
    // Truman ([SAN]) plays the other.
    e.choose(index_of(&req, play(0, 1, truman)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 1);
    assert_eq!(e.state.seats[0].uncontrolled[1].counters, 0);
}

// Unwholesome Bond [LSJ 20050927] [ANK 20211113] "Can be used by Angelo or
// New Blood on a uncontrolled vampire with no circle designation, even
// another Angelo or New Blood. Multiple uncontrolled Angelos or New Bloods
// are in distinct "circle of one" though, so you have to choose one": New
// Blood's [SAN] on one of two uncontrolled New Bloods; a Torrance vampire
// may not.
#[test]
fn unwholesome_bond_lets_new_blood_feed_a_circle_of_one() {
    let mut e = game(2);
    let n = new_blood(&mut e, 0, 3);
    superior(&mut e, n, "san");
    let ilse = torrance(&mut e, 0, "Ilse", 4);
    superior(&mut e, ilse, "san");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let nb = find_card("New Blood");
    e.state.seats[0].uncontrolled = vec![
        zvtes::state::UncontrolledSlot {
            card: nb,
            copy: Default::default(),
            counters: 0,
            banished: None,
        },
        zvtes::state::UncontrolledSlot {
            card: nb,
            copy: Default::default(),
            counters: 0,
            banished: None,
        },
    ];
    hand(&mut e, 0, &["Unwholesome Bond"]);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&play_at(0, 2, ilse, Target::Uncontrolled(0))));
    e.choose(index_of(&req, play_at(0, 2, n, Target::Uncontrolled(1))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 0);
    assert_eq!(e.state.seats[0].uncontrolled[1].counters, 1);
}

// ---- Step 4: Ambush, Gestalt, London Evening Star, Sanguine Instruction --------

// Ambush: "Ⓓ Enter combat with a locked minion. This acting minion gets 1
// optional maneuver during that combat." [ANK 20210124] [LSJ 20070411] [LSJ
// 20090514] "If the action is unblocked when it resolves and the target is
// unlocked, the action "fizzles" (cost, if any, is paid, and it is
// considered successful, but has no effect)": an unlocked minion is no
// target; the locked one unlocked before resolution, no combat.
#[test]
fn ambush_needs_a_locked_minion() {
    for unlocked_later in [false, true] {
        let mut e = game(2);
        let ilse = torrance(&mut e, 0, "Ilse", 6);
        torrance(&mut e, 0, "Karl", 6);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        e.state.minions[z.index()].locked = true;
        hand(&mut e, 0, &["Ambush"]);
        let req = request(&mut e);
        assert!(!req
            .options
            .contains(&play_at(0, 0, ilse, Target::Minion(y))));
        e.choose(index_of(&req, play_at(0, 0, ilse, Target::Minion(z))));
        if unlocked_later {
            e.state.minions[z.index()].locked = false;
        }
        let before = blood(&e, z);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, z) < before, !unlocked_later);
    }
}

// Gestalt: "Choose X ready Blood Brothers you control who belong to the same
// circle as this reacting vampire (including this vampire). Each chosen
// vampire must burn a blood, or this card has no effect. [san] +X
// intercept. [SAN] … and this vampire may play cards as if he or she
// possessed all of the chosen Blood Brothers' Disciplines for the remainder
// of the action." Against a +1 stealth Zillah's Tears, Ilse picks herself
// and Karl: each burns 1, +2 intercept — the block succeeds; at [SAN] she
// has Karl's superior Fortitude for the action. A Hermana, of another
// circle [LSJ 20020609], is not offered.
#[test]
fn gestalt_lends_the_circles_strength() {
    for sup in [false, true] {
        let mut e = game(2);
        let ilse = torrance(&mut e, 0, "Ilse", 6);
        let karl = torrance(&mut e, 0, "Karl", 6);
        let h = hermana(&mut e, 0, 4);
        let z = hermana(&mut e, 1, 4);
        hermana(&mut e, 1, 4);
        // A strike card, for a decision in the combat.
        superior(&mut e, ilse, "tha");
        hand(&mut e, 0, &["Gestalt", "Theft of Vitae"]);
        hand(&mut e, 1, &["Zillah's Tears"]);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        pick(&mut e, play(0, 0, z));
        pick_when(&mut e, block(ilse));
        let req = offering(&mut e, play(0, sup as u8, ilse));
        e.choose(index_of(&req, play(0, sup as u8, ilse)));
        let req = request(&mut e);
        assert!(!req.options.contains(&Choice::ChooseMinion(h)));
        e.choose(index_of(&req, Choice::ChooseMinion(ilse)));
        pick(&mut e, Choice::ChooseMinion(karl));
        until(&mut e, SeatId(0), WindowLabel::CombatStrike);
        assert!(e.combat().is_some(), "blocked");
        assert_eq!(blood(&e, ilse), 5);
        assert_eq!(blood(&e, karl), 5);
        assert_eq!(e.state.has_discipline(ilse, "FOR"), sup);
    }
}

// London Evening Star: "Lock to give any minion +1 intercept for the
// current action." [ANK 20200607] "You must gain the impulse to be able to
// use the reaction, or use the effect": with its controller's impulse in
// the block attempt, Ilse gets +1 intercept against a +1 stealth action —
// the block succeeds.
#[test]
fn london_evening_star_gives_intercept() {
    let mut e = game(2);
    let ilse = torrance(&mut e, 0, "Ilse", 6);
    torrance(&mut e, 0, "Karl", 6);
    let star = {
        let card = find_library("London Evening Star, Tabloid Newspaper");
        e.state
            .cards
            .push(CardInPlay::new(card, SeatId(0), None, 0));
        CardId(e.state.cards.len() as u32 - 1)
    };
    let z = hermana(&mut e, 1, 4);
    hermana(&mut e, 1, 4);
    // A strike card, for a decision in the combat.
    superior(&mut e, ilse, "tha");
    hand(&mut e, 0, &["Theft of Vitae"]);
    hand(&mut e, 1, &["Zillah's Tears"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, z));
    pick_when(&mut e, block(ilse));
    let lock = use_at(star, 0, None, Target::Minion(ilse));
    let req = offering(&mut e, lock);
    e.choose(index_of(&req, lock));
    until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(e.state.cards[star.index()].locked);
    assert!(e.combat().is_some(), "blocked");
}

// Sanguine Instruction: "+1 stealth action. Requires a ready vampire.
// Choose a Discipline this vampire has at superior. Put this card on a
// ready vampire of the same clan as this acting vampire. The vampire with
// this card gains one level of the chosen Discipline. The vampire's
// capacity increases by 1 … Cannot be placed on a vampire with the
// superior version of the Discipline." Ilse ([POT] [SAN]) on Karl ([POT]
// already): [SAN] the only choice; Karl gets it, capacity 7; not on a
// Ventrue (another clan).
#[test]
fn sanguine_instruction_teaches_a_discipline() {
    let mut e = game(2);
    let ilse = torrance(&mut e, 0, "Ilse", 6);
    let karl = torrance(&mut e, 0, "Karl", 6);
    let v = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Sanguine Instruction"]);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&play_at(0, 0, ilse, Target::Minion(v))));
    // [SAN] the only choice left: taken at once.
    e.choose(index_of(&req, play_at(0, 0, ilse, Target::Minion(karl))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.has_discipline(karl, "SAN"));
    assert_eq!(e.state.capacity_of(karl), Some(7));
}

// Ambush [LSJ 20090514] "If the action lacks a suitable target because of
// the unlock (e.g. Ambush), the block attempt still happens, as does the
// resulting combat if the block is successful": the locked target unlocks
// and attempts to block with Sense the Savage Way (its [ani] the
// fixture's; capacity 7) — it blocks, the combat follows.
#[test]
fn ambush_target_unlocking_to_block_still_blocks() {
    let mut e = game(2);
    let ilse = torrance(&mut e, 0, "Ilse", 6);
    torrance(&mut e, 0, "Karl", 6);
    superior(&mut e, ilse, "tha");
    let z = e.add_minion(SeatId(1), of_capacity_at_least(7), 4, false);
    e.state.minions[z.index()].locked = true;
    superior(&mut e, z, "ani");
    hand(&mut e, 0, &["Ambush", "Theft of Vitae"]);
    hand(&mut e, 1, &["Sense the Savage Way"]);
    pick(&mut e, play_at(0, 0, ilse, Target::Minion(z)));
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    let c = e.combat().expect("combat");
    assert!(c.side_of_minion(z).is_some());
    assert!(e.action().is_some_and(|a| a.blocked_by.contains(&z)));
}

// ---- Step 5: Il Glorioso Piano Quinquennale (Kiev circle) ---------------------

fn kiev(e: &mut Engine, seat: u8, name: &str, blood: u8) -> MinionId {
    e.add_minion(SeatId(seat), find_card(name), blood, false)
}

// Danylo: "Danylo can search your library for a location, reveal it, and
// move it to your hand as an action that costs 1 pool (shuffle and discard
// down afterward)." [LSJ 20080114] [ANK 20221210] "Cannot fetch equipments
// that are locations while in play (locquipments)": from a library of a
// Channel 10 and other cards, the master location comes to hand, 1 pool
// paid.
#[test]
fn danylo_fetches_a_location() {
    let mut e = game(2);
    let d = kiev(&mut e, 0, "Danylo", 4);
    kiev(&mut e, 0, "Kyrylo", 4);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].library = vec![
        find_library("Deflection"),
        find_library("Channel 10"),
        find_library("Deflection"),
    ]
    .into();
    let act = Choice::UseAbility {
        source: AbilitySource::Vampire(d),
        ability: 0,
        actor: Some(d),
        target: Target::None,
    };
    pick(&mut e, act);
    let ch = find_library("Channel 10");
    // The only location: taken at once.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].hand.contains(&ch));
    assert_eq!(pool(&e)[0], 10 - 1);
}

// Gavrylo: "Allies and younger vampires get -1 intercept against Gavrylo."
// His bleed (no stealth): an older blocker may attempt to block it; a
// younger one's -1 intercept cannot reach it — never offered.
#[test]
fn gavrylo_dulls_younger_blockers() {
    for younger in [true, false] {
        let mut e = game(2);
        let g = kiev(&mut e, 0, "Gavrylo", 4);
        kiev(&mut e, 0, "Kyrylo", 4);
        let card = zvtes::crypt::CRYPT
            .iter()
            .enumerate()
            .find(|(i, c)| {
                (if younger {
                    c.capacity < 4
                } else {
                    c.capacity > 4
                }) && zvtes::cards::vampire(*i as u16).is_none()
            })
            .map(|(i, _)| i as u16)
            .expect("a vampire");
        let z = e.add_minion(SeatId(1), card, 2, false);
        pick(&mut e, bleed(g));
        if younger {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(z));
        } else {
            let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
            assert!(req.options.contains(&block(z)));
        }
    }
}

// Pavlo: "During your unlock phase, you can move 1 blood from Pavlo to a
// ready Blood Brother of the Kiev circle." To Volo, not to a Torrance Ilse.
#[test]
fn pavlo_feeds_the_kiev_circle() {
    let mut e = game(2);
    let p = kiev(&mut e, 0, "Pavlo", 3);
    let v = kiev(&mut e, 0, "Volo", 1);
    let ilse = torrance(&mut e, 0, "Ilse", 1);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let give = |t| Choice::UseAbility {
        source: AbilitySource::Vampire(p),
        ability: 0,
        actor: None,
        target: Target::Minion(t),
    };
    let req = offering(&mut e, give(v));
    assert!(!req.options.contains(&give(ilse)));
    e.choose(index_of(&req, give(v)));
    request(&mut e);
    assert_eq!(blood(&e, v), 2);
    assert_eq!(blood(&e, p), 2);
}

// Volo: "Volo gets -1 stealth when hunting." A hunt (the rulebook's +1
// stealth): Kyrylo's cannot be blocked by a vampire with no intercept —
// never offered; Volo's, at 0 stealth, can.
#[test]
fn volo_hunts_with_less_stealth() {
    for volo in [true, false] {
        let mut e = game(2);
        let v = kiev(&mut e, 0, "Volo", 2);
        let k = kiev(&mut e, 0, "Kyrylo", 2);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        pick(&mut e, action(if volo { v } else { k }, ActionKind::Hunt));
        if volo {
            let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
            assert!(req.options.contains(&block(z)));
        } else {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(z));
        }
    }
}

// Dark Influences: "Out-of-turn. Cancel a minion card as it is played, its
// cost is not paid, and put this card in play. That card cannot be played
// again this turn. The next card played that would cancel another
// Methuselah's minion card as it is played is canceled, its cost is not
// paid, and this card is burned instead." Seat 1 cancels seat 0's Hive
// Mind: in play, a second Hive Mind not offered this turn; the next turn
// seat 0's Direct Intervention on seat 1's Zillah's Tears is canceled
// unpaid and the Dark Influences burns.
#[test]
fn dark_influences_cancels_and_shields() {
    let mut e = game(2);
    let a = hermana(&mut e, 0, 4);
    let b = hermana(&mut e, 0, 4);
    let z = hermana(&mut e, 1, 4);
    hermana(&mut e, 1, 4);
    hand(
        &mut e,
        0,
        &["Hive Mind", "Hive Mind", "Direct Intervention"],
    );
    hand(&mut e, 1, &["Dark Influences", "Zillah's Tears"]);
    e.state.minions[b.index()].locked = true;
    pick(&mut e, play_at(0, 0, a, Target::Minion(b)));
    pick_when(&mut e, master_play(0));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let di = CardId(e.state.cards.len() as u32 - 1);
    assert!(e.state.cards[di.index()].in_play);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0, .. })),
        "barred this turn"
    );
    let pool0 = pool(&e)[0];
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, z));
    let dint = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Direct Intervention"))
        .expect("in hand") as u8;
    pick_when(&mut e, master_play(dint));
    request(&mut e);
    assert!(!e.state.cards[di.index()].in_play, "burned instead");
    assert_eq!(pool(&e)[0], pool0, "the cancel unpaid");
}

// Loki's Gift: "+1 stealth hunt action. [pro] Hunt, and put this card on
// this vampire. This vampire can burn this card and 1 blood to get +1
// stealth. [PRO] Ⓓ As above, and burn 1 blood from any vampire." [ANK
// 20220809] "[PRO] The blood burned is not affected by effects modifying
// the hunt amount", and a special hunt "only once per turn" [RBK
// action-card-or-card-in-play]: Volo's [PRO] (its superior the fixture's)
// burns 1 of a Ventrue's blood and hunts; the card on him; no second Loki's
// Gift hunt this turn.
#[test]
fn lokis_gift_hunts_and_bites() {
    let mut e = game(2);
    let v = kiev(&mut e, 0, "Volo", 1);
    superior(&mut e, v, "pro");
    kiev(&mut e, 0, "Kyrylo", 4);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Loki's Gift", "Loki's Gift"]);
    let before = blood(&e, z);
    pick(&mut e, play_at(0, 1, v, Target::Minion(z)));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), before - 1);
    assert_eq!(blood(&e, v), 2, "hunted");
    let gift = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(e.state.cards[gift.index()].host, Some(v));
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == v)));
}

// Procurer: "Ghoul with 1 life. 1 strength, 0 bleed. Procurer may move 1
// blood from the blood bank to a ready vampire you control as a +2 stealth
// action." Recruited, then (next turn) its action gives a Hermana 1 blood.
#[test]
fn procurer_feeds_a_vampire() {
    let mut e = game(2);
    let a = hermana(&mut e, 0, 2);
    hermana(&mut e, 0, 2);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Procurer"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let p = MinionId(e.state.minions.len() as u32 - 1);
    assert!(e.state.minions[p.index()].is_ally());
    e.state.minions[p.index()].recruited = false;
    let act = Choice::UseAbility {
        source: AbilitySource::Card(e.state.minions[p.index()].ally_id().expect("ally")),
        ability: 0,
        actor: Some(p),
        target: Target::Minion(a),
    };
    let before = blood(&e, a);
    let req = offering(&mut e, act);
    e.choose(index_of(&req, act));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), before + 1);
}

// ---- Step 6: The Fratmen (Chicago circle) -------------------------------------

fn chicago(e: &mut Engine, seat: u8, name: &str, blood: u8) -> MinionId {
    e.add_minion(SeatId(seat), find_card(name), blood, false)
}

/// A vampire of capacity in `range` with no definition of its own.
fn of_capacity_in(range: std::ops::RangeInclusive<u8>) -> u16 {
    zvtes::crypt::CRYPT
        .iter()
        .enumerate()
        .find(|(i, c)| range.contains(&c.capacity) && zvtes::cards::vampire(*i as u16).is_none())
        .map(|(i, _)| i as u16)
        .expect("a vampire")
}

// Barry: "Barry gets +1 strength when in combat with an ally or a younger
// vampire." Blocked by a younger vampire, his hand strike does 2; by an
// older one, 1.
#[test]
fn barry_hits_younger_harder() {
    for younger in [true, false] {
        let mut e = game(2);
        let b = chicago(&mut e, 0, "Barry", 5);
        chicago(&mut e, 0, "Larry", 5);
        let card = of_capacity_in(if younger { 1..=4 } else { 6..=11 });
        let z = e.add_minion(SeatId(1), card, 1, false);
        e.state.minions[z.index()].blood = 4;
        blocked_bleed(&mut e, b, z);
        let before = blood(&e, z);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            before.saturating_sub(blood(&e, z)),
            if younger { 2 } else { 1 }
        );
    }
}

// Jerry: "During your master phase, you may move any amount of blood from
// the Blood Brothers in the Chicago Circle to any other Blood Brothers in
// this circle." One move of 2 blood from Barry to Larry (owner, rules-qa
// 68.10: a move is x blood between two of them), not to a Hermana.
#[test]
fn jerry_moves_the_circles_blood() {
    let mut e = game(2);
    let j = chicago(&mut e, 0, "Jerry", 3);
    let barry = chicago(&mut e, 0, "Barry", 4);
    let larry = chicago(&mut e, 0, "Larry", 1);
    let h = hermana(&mut e, 0, 1);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let use_jerry = Choice::UseAbility {
        source: AbilitySource::Vampire(j),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    let req = offering(&mut e, use_jerry);
    e.choose(index_of(&req, use_jerry));
    // One move: the source, the destination, the amount.
    pick(&mut e, Choice::ChooseMinion(barry));
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::ChooseMinion(h)));
    e.choose(index_of(&req, Choice::ChooseMinion(larry)));
    let req = request(&mut e);
    assert_eq!(req.options.len(), 4, "1 to 4 blood");
    e.choose(index_of(&req, Choice::SetX(2)));
    pick(&mut e, Choice::Pass);
    assert_eq!(blood(&e, barry), 2);
    assert_eq!(blood(&e, larry), 3);
}

// Larry: "Once each action, Larry can burn 1 blood to get +1 intercept."
// Against a +1 stealth Zillah's Tears: his blood for the intercept, the
// block succeeds.
#[test]
fn larry_burns_for_intercept() {
    let mut e = game(2);
    let larry = chicago(&mut e, 0, "Larry", 4);
    superior(&mut e, larry, "tha");
    chicago(&mut e, 0, "Barry", 4);
    let z = hermana(&mut e, 1, 4);
    hermana(&mut e, 1, 4);
    hand(&mut e, 0, &["Theft of Vitae"]);
    hand(&mut e, 1, &["Zillah's Tears"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, z));
    pick_when(&mut e, block(larry));
    let burn = Choice::UseAbility {
        source: AbilitySource::Vampire(larry),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    let req = offering(&mut e, burn);
    e.choose(index_of(&req, burn));
    until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert_eq!(blood(&e, larry), 3);
}

// Larry: "Once each action, Larry can burn 1 blood to get +1 intercept"
// — for his own block attempt: not for Barry's.
#[test]
fn larry_buys_no_intercept_for_another_blocker() {
    let mut e = game(2);
    let larry = chicago(&mut e, 0, "Larry", 4);
    let barry = chicago(&mut e, 0, "Barry", 4);
    let z = hermana(&mut e, 1, 4);
    hermana(&mut e, 1, 4);
    hand(&mut e, 1, &["Zillah's Tears"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, z));
    pick_when(&mut e, block(barry));
    let burn = Choice::UseAbility {
        source: AbilitySource::Vampire(larry),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, burn);
    assert_eq!(blood(&e, larry), 4);
}

// Terry: "Once each combat, Terry can burn 1 blood to get a press."
// [PIB 20150121] "Her ability to get a press can only be used during the
// press step": blocked, she presses for a second round with a blood.
#[test]
fn terry_presses_for_blood() {
    let mut e = game(2);
    let t = chicago(&mut e, 0, "Terry", 4);
    chicago(&mut e, 0, "Barry", 4);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    blocked_bleed(&mut e, t, z);
    let press = Choice::UseAbility {
        source: AbilitySource::Vampire(t),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    let req = offering(&mut e, press);
    let before = blood(&e, z);
    assert_eq!(req.window, WindowLabel::CombatPress);
    e.choose(index_of(&req, press));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Two rounds of hand strikes: 1 more after the press.
    assert_eq!(before - blood(&e, z), 1);
    assert_eq!(blood(&e, t), 4 - 1 - 2, "the press's blood, 2 hand strikes");
}

// Tom: "Cards that require Sanguinus [san] cost Tom an additional blood."
// His Hive Mind costs him 2.
#[test]
fn tom_pays_more_for_sanguinus() {
    let mut e = game(2);
    let tom = chicago(&mut e, 0, "Tom", 3);
    chicago(&mut e, 0, "Barry", 4);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Hive Mind"]);
    let barry = MinionId(1);
    e.state.minions[barry.index()].locked = true;
    pick(&mut e, play_at(0, 0, tom, Target::Minion(barry)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, tom), 1);
}

// Major Boon: "Only usable if another Methuselah is being bled by a minion
// you do not control and the bleed amount is 1 or more, after blocks are
// declined. Put this card in play. You burn pool for this bleed instead of
// the target Methuselah (and the bleed is still successful) … If you are
// about to be successfully bled, you can burn this card to have that
// Methuselah burn pool instead of you." [LSJ 19990218] [RTR 19970630]:
// Seat 0 bleeds seat 1 for 1; seat 2 takes it. Later seat 1 bleeds seat 2;
// seat 2 burns the boon and seat 1 pays its own bleed.
#[test]
fn major_boon_takes_a_bleed_and_returns_one() {
    let mut e = game(3);
    let a = hermana(&mut e, 0, 4);
    hermana(&mut e, 0, 4);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
    hand(&mut e, 2, &["Major Boon"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, master_play(0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 10, 9]);
    let boon = CardId(e.state.cards.len() as u32 - 1);
    assert!(e.state.cards[boon.index()].in_play);
    pick(&mut e, bleed(y));
    let burn = use_at(boon, 0, None, Target::None);
    pick_when(&mut e, burn);
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 1, "seat 1 pays its own bleed");
    assert_eq!(pool(&e)[2], 9);
    assert!(!e.state.cards[boon.index()].in_play);
}

// Rampage: "[pot] Ⓓ Burn any location. If you control the location, this
// is a +1 stealth action." Barry burns seat 1's Channel 10.
#[test]
fn rampage_burns_a_location() {
    let mut e = game(2);
    let b = chicago(&mut e, 0, "Barry", 4);
    chicago(&mut e, 0, "Larry", 4);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.cards.push(CardInPlay::new(
        find_library("Channel 10"),
        SeatId(1),
        None,
        0,
    ));
    let ch = CardId(e.state.cards.len() as u32 - 1);
    hand(&mut e, 0, &["Rampage"]);
    pick(&mut e, play_at(0, 0, b, Target::Card(ch)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[ch.index()].in_play);
}

// Skin of Rock: "[for] Prevent 1 damage." [LSJ 20001114] "Cannot be used if
// there is no damage to prevent" and [RTR 20041202] "If fewer points of
// (preventable) damage are being resolved, then the effect prevents all of
// those points": Barry blocked, the 1 damage he takes prevented.
#[test]
fn skin_of_rock_prevents() {
    let mut e = game(2);
    let b = chicago(&mut e, 0, "Barry", 4);
    chicago(&mut e, 0, "Larry", 4);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Skin of Rock"]);
    blocked_bleed(&mut e, b, z);
    let req = offering(&mut e, play(0, 0, b));
    assert_eq!(req.window, WindowLabel::CombatPrevention);
    e.choose(index_of(&req, play(0, 0, b)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 4);
}

// Major Boon [RTR 19970630] "Cannot be played in succession … a second
// cannot be played to Boon the first" and [LSJ 19990218] "Multiple Major
// Boons in play cannot be burned on the same bleed action - after the
// first is burned, "you" are no longer the Methuselah burning pool"; [LSJ
// 20090205] "Only modifiers and reactions that can be played "when the
// action/bleed would be successful" can be played after Major Boon is
// played or burned": seat 2 takes seat 0's bleed on seat 1 — its second
// Major Boon is not offered on it; with two of them in play, the first
// burned, the second is not.
#[test]
fn major_boons_do_not_stack() {
    let mut e = game(3);
    let a = hermana(&mut e, 0, 4);
    hermana(&mut e, 0, 4);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
    hand(&mut e, 2, &["Major Boon", "Major Boon"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, master_play(0));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, master_play(0));
    // A second boon in play, remembering seat 1 too.
    let first = CardId(e.state.cards.len() as u32 - 1);
    let mut copy = CardInPlay::new(find_library("Major Boon"), SeatId(2), None, 0);
    copy.against = e.state.cards[first.index()].against;
    e.state.cards.push(copy);
    let second = CardId(e.state.cards.len() as u32 - 1);
    pick(&mut e, bleed(y));
    pick_when(&mut e, use_at(first, 0, None, Target::None));
    never_offered_until(
        &mut e,
        SeatId(2),
        WindowLabel::MinionPhase,
        use_at(second, 0, None, Target::None),
    );
    assert!(e.state.cards[second.index()].in_play);
}

// Skin of Rock [ANK 20200318] [LSJ 20001114] "Unused prevention points
// can't be carried over unless optional": Terry's [FOR] (the fixture's)
// prevents round 1's single point; after her press, round 2's lands — the
// unused point is gone.
#[test]
fn skin_of_rock_does_not_carry_over() {
    let mut e = game(2);
    let t = chicago(&mut e, 0, "Terry", 4);
    superior(&mut e, t, "for");
    chicago(&mut e, 0, "Barry", 4);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Skin of Rock"]);
    blocked_bleed(&mut e, t, z);
    let req = offering(&mut e, play(0, 1, t));
    e.choose(index_of(&req, play(0, 1, t)));
    let press = Choice::UseAbility {
        source: AbilitySource::Vampire(t),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    let req = offering(&mut e, press);
    e.choose(index_of(&req, press));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, t),
        4 - 1 - 1,
        "the press's blood, round 2's damage"
    );
}

// ---- Step 7: Sorority Codex (Budapest circle) ---------------------------------

fn budapest(e: &mut Engine, seat: u8, name: &str, blood: u8) -> MinionId {
    e.add_minion(SeatId(seat), find_card(name), blood, false)
}

// Dorka: "Dorka can enter combat with a minion controlled by your predator
// or prey as a Ⓓ action." Seat 0 of three: its prey's minion and its
// predator's are targets.
#[test]
fn dorka_rushes_predator_or_prey() {
    let mut e = game(3);
    let d = budapest(&mut e, 0, "Dorka", 4);
    budapest(&mut e, 0, "Zizi", 3);
    let prey = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let pred = e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    let rush = |t| Choice::UseAbility {
        source: AbilitySource::Vampire(d),
        ability: 0,
        actor: Some(d),
        target: Target::Minion(t),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&rush(prey)));
    assert!(req.options.contains(&rush(pred)));
}

// Gabi: "A minion opposing Gabi cannot strike: dodge during the first round
// of combat." Eugene's once-each-combat dodge is not offered against her
// the first round; against Viri it is.
#[test]
fn gabi_bars_dodges_the_first_round() {
    for gabi in [true, false] {
        let mut e = game(2);
        let g = budapest(&mut e, 0, "Gabi", 4);
        let viri = budapest(&mut e, 0, "Viri", 3);
        let eugene = e.add_minion(SeatId(1), find_card("Eugene"), 4, false);
        blocked_bleed(&mut e, if gabi { g } else { viri }, eugene);
        let dodge = Choice::UseAbility {
            source: AbilitySource::Vampire(eugene),
            ability: 0,
            actor: None,
            target: Target::None,
        };
        if gabi {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, dodge);
        } else {
            offering(&mut e, dodge);
        }
    }
}

// Viri: "While /Zizi/ is in your ready region, Viri gets +1 strength."
// Zizi: "If /Viri/ is in your ready region, Zizi can lock to unlock another
// Blood Brother of the same circle." [LSJ 20010702] "If a minion who is
// attempting to block is locked, then that block will fail": Viri's hand
// strike does 2 beside Zizi; Zizi locks to unlock the locked Dorka.
#[test]
fn viri_and_zizi_work_together() {
    let mut e = game(2);
    let viri = budapest(&mut e, 0, "Viri", 3);
    let zizi = budapest(&mut e, 0, "Zizi", 3);
    let dorka = budapest(&mut e, 0, "Dorka", 3);
    e.state.minions[dorka.index()].locked = true;
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let wake = Choice::UseAbility {
        source: AbilitySource::Vampire(zizi),
        ability: 0,
        actor: None,
        target: Target::Minion(dorka),
    };
    let req = request(&mut e);
    e.choose(index_of(&req, wake));
    request(&mut e);
    assert!(!e.state.minions[dorka.index()].locked);
    assert!(e.state.minions[zizi.index()].locked);
    let before = blood(&e, z);
    blocked_bleed(&mut e, viri, z);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(before - blood(&e, z), 2);
}

// Zizi: "If /Viri/ is in your ready region, Zizi can lock …": Viri in
// torpor, no unlock; out of it, the unlock is there.
#[test]
fn zizi_unlocks_only_with_viri_ready() {
    let mut e = game(2);
    let viri = e.add_minion(SeatId(0), find_card("Viri"), 3, true);
    let zizi = budapest(&mut e, 0, "Zizi", 3);
    let dorka = budapest(&mut e, 0, "Dorka", 3);
    e.state.minions[dorka.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let wake = Choice::UseAbility {
        source: AbilitySource::Vampire(zizi),
        ability: 0,
        actor: None,
        target: Target::Minion(dorka),
    };
    assert!(!request(&mut e).options.contains(&wake));
    e.state.minions[viri.index()].in_torpor = false;
    assert!(request(&mut e).options.contains(&wake));
}

// "Your ready region" is Zizi's controller's (owner, 2026-10-14: rules-qa
// 73.7): a ready Viri of another Methuselah's unlocks nothing, nor does
// that Zizi give the other Viri +1 strength.
#[test]
fn zizi_needs_viri_in_her_own_ready_region() {
    let mut e = game(2);
    let zizi = budapest(&mut e, 0, "Zizi", 3);
    let dorka = budapest(&mut e, 0, "Dorka", 3);
    e.state.minions[dorka.index()].locked = true;
    let viri = e.add_minion(SeatId(1), find_card("Viri"), 3, false);
    let wake = Choice::UseAbility {
        source: AbilitySource::Vampire(zizi),
        ability: 0,
        actor: None,
        target: Target::Minion(dorka),
    };
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, wake);
    let strength = e
        .state
        .printed_mod(viri, cards::Stat::Strength, Default::default());
    assert_eq!(strength, 0, "Zizi not in Viri's ready region");
}

// Ági: "Cards requiring Sanguinus [san] cost Ági 1 fewer blood." Her Hive
// Mind costs her none.
#[test]
fn agi_pays_less_for_sanguinus() {
    let mut e = game(2);
    let agi = budapest(&mut e, 0, "Ági", 4);
    let viri = budapest(&mut e, 0, "Viri", 3);
    e.state.minions[viri.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Hive Mind"]);
    pick(&mut e, play_at(0, 0, agi, Target::Minion(viri)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, agi), 4);
}

// Redistribution: "+1 stealth action. [san] This acting vampire gains a
// blood. Then move any amount of blood and equipment among the Blood
// Brothers in this circle." [LSJ 20061109] "Cannot target an uncontrolled
// Blood Brother": Viri gains 1, then one move of 2 of it to Zizi and her
// Monocle to Dorka; a Hermana, of another circle [LSJ 20020609], is no part
// of it.
#[test]
fn redistribution_moves_blood_and_equipment() {
    let mut e = game(2);
    let viri = budapest(&mut e, 0, "Viri", 2);
    let zizi = budapest(&mut e, 0, "Zizi", 1);
    let dorka = budapest(&mut e, 0, "Dorka", 2);
    let h = hermana(&mut e, 0, 2);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let monocle = {
        let card = find_library("Monocle of Clarity");
        e.state
            .cards
            .push(CardInPlay::new(card, SeatId(0), Some(zizi), 0));
        CardId(e.state.cards.len() as u32 - 1)
    };
    hand(&mut e, 0, &["Redistribution"]);
    pick(&mut e, play(0, 0, viri));
    let req = offering(&mut e, Choice::ChooseMinion(viri));
    assert_eq!(blood(&e, viri), 3);
    assert!(!req.options.contains(&Choice::ChooseMinion(h)));
    e.choose(index_of(&req, Choice::ChooseMinion(viri)));
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::ChooseMinion(h)));
    e.choose(index_of(&req, Choice::ChooseMinion(zizi)));
    pick(&mut e, Choice::SetX(2));
    pick(&mut e, Choice::ChooseCard(monocle.0));
    pick(&mut e, Choice::ChooseMinion(dorka));
    pick(&mut e, Choice::Pass);
    assert_eq!(blood(&e, viri), 1);
    assert_eq!(blood(&e, zizi), 3);
    assert_eq!(e.state.cards[monocle.index()].host, Some(dorka));
}

// Robert Carter: "Unique ghoul with 1 life. During your unlock phase,
// Carter's employer burns 1 blood, or Carter is burned. The vampire with
// this retainer gets +2 bleed." Employed by Viri; her bleed is 3; at the
// unlock phase her blood keeps him, or he is burned.
#[test]
fn robert_carter_bleeds_for_blood() {
    for keep in [true, false] {
        let mut e = game(2);
        let viri = budapest(&mut e, 0, "Viri", 3);
        budapest(&mut e, 0, "Zizi", 3);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Robert Carter"]);
        pick(&mut e, play(0, 0, viri));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let carter = CardId(e.state.cards.len() as u32 - 1);
        assert_eq!(e.state.cards[carter.index()].host, Some(viri));
        let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        e.choose(index_of(
            &req,
            if keep {
                Choice::BurnBloodToAvoid
            } else {
                Choice::Pass
            },
        ));
        request(&mut e);
        assert_eq!(e.state.cards[carter.index()].in_play, keep);
        if keep {
            assert_eq!(bled(e, viri), 3);
        }
    }
}

/// The pool seat 1 loses to `m`'s bleed (seat 0 bleeding its prey).
fn bled(mut e: Engine, m: MinionId) -> i32 {
    let before = pool(&e)[1];
    pick(&mut e, bleed(m));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    before - pool(&e)[1]
}

// Monocle of Clarity: equipped, nothing more (its question is no game
// state).
#[test]
fn monocle_of_clarity_is_equipped() {
    let mut e = game(2);
    let viri = budapest(&mut e, 0, "Viri", 3);
    budapest(&mut e, 0, "Zizi", 3);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Monocle of Clarity"]);
    pick(&mut e, play(0, 0, viri));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let m = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(e.state.cards[m.index()].host, Some(viri));
}

// ---- Step 8: The Hungarian Coyote ---------------------------------------------

// Breath of the Dragon: "[vic] Strike: 1R aggravated damage. If this strike
// is used at close range, this striking vampire also takes 1 environmental
// aggravated damage." [LSJ 19970801] "Damage done to the striking minion is
// environmental": Viri's breath at close range — 1 aggravated to the
// blocker, 1 to her besides the blocker's hand strike: both in torpor.
#[test]
fn breath_of_the_dragon_burns_both() {
    let mut e = game(2);
    let viri = budapest(&mut e, 0, "Viri", 3);
    budapest(&mut e, 0, "Dorka", 3);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Breath of the Dragon"]);
    blocked_bleed(&mut e, viri, z);
    let before = blood(&e, z);
    let req = offering(&mut e, play(0, 0, viri));
    e.choose(index_of(&req, play(0, 0, viri)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Aggravated damage is not mended: both to torpor, the blocker's blood
    // kept; Viri's 1 for the card, 1 for the hand strike.
    assert!(e.state.minions[z.index()].in_torpor);
    assert_eq!(blood(&e, z), before);
    assert!(e.state.minions[viri.index()].in_torpor);
    assert_eq!(blood(&e, viri), 3 - 1 - 1);
}

// Chiropteran Marauder: "[VIC] Burn 1 blood to make the damage from this
// vampire's hand strikes aggravated this round." [ANK 20210226] "The burn
// blood effect is not reduced by effects that reduce the cost of the card
// (eg. Adana de Sforza)" and "The card cannot be played if the minion cannot
// afford to burn the blood"; [RTR 19960112] "must be used before strike
// resolution and damage prevention … Can be played after both strikes have
// been chosen": Ági (cards requiring [san] cost her less — not this blood)
// burns 1, her hand strike aggravated; with no blood, not offered.
#[test]
fn chiropteran_marauder_aggravates_hands() {
    for blood_left in [true, false] {
        let mut e = game(2);
        let agi = budapest(&mut e, 0, "Ági", 6);
        budapest(&mut e, 0, "Viri", 3);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Chiropteran Marauder"]);
        blocked_bleed(&mut e, agi, z);
        if blood_left {
            let req = offering(&mut e, play(0, 1, agi));
            e.choose(index_of(&req, play(0, 1, agi)));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            // Its blood, then the blocker's hand strike; hers aggravated.
            assert_eq!(blood(&e, agi), 6 - 1 - 1);
            assert!(e.state.minions[z.index()].in_torpor);
        } else {
            e.state.minions[agi.index()].blood = 0;
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, agi));
        }
    }
}

// Tribute to the Master: "Move 1 blood from each ready vampire you control
// to your pool." [ANK 20210717] "Can be played with no ready vampire": two
// Budapest vampires give 1 each; the one in torpor gives none.
#[test]
fn tribute_to_the_master_takes_a_blood_each() {
    let mut e = game(2);
    let viri = budapest(&mut e, 0, "Viri", 3);
    let zizi = budapest(&mut e, 0, "Zizi", 3);
    let dorka = budapest(&mut e, 0, "Dorka", 3);
    e.state.minions[dorka.index()].in_torpor = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Tribute to the Master"]);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    e.state.minions[dorka.index()].in_torpor = true;
    let pool0 = pool(&e)[0];
    e.choose(index_of(&req, master_play(0)));
    request(&mut e);
    assert_eq!(pool(&e)[0], pool0 + 2);
    assert_eq!(blood(&e, viri), 2);
    assert_eq!(blood(&e, zizi), 2);
    assert_eq!(blood(&e, dorka), 3);
}

// ---- Step 9: Museum Sisters ---------------------------------------------------

// Blood Tears of Kephran: "The vampire with this equipment may burn this
// card to prevent 2 points of damage in combat or to gain 2 blood (ignore
// excess blood)." [RTR 20041202] "If fewer points of (preventable) damage
// are being resolved, then the effect prevents all of those points" and
// [ANK 20200318] [LSJ 20001114] "Unused prevention points can't be carried
// over unless optional": on Viri, burned against a 1-damage hand strike —
// none taken; another burned for 2 blood, to her capacity.
#[test]
fn blood_tears_of_kephran_prevent_or_feed() {
    let mut e = game(2);
    let viri = budapest(&mut e, 0, "Viri", 3);
    budapest(&mut e, 0, "Zizi", 3);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let tears = |e: &mut Engine| {
        let card = find_library("Blood Tears of Kephran");
        e.state
            .cards
            .push(CardInPlay::new(card, SeatId(0), Some(viri), 0));
        CardId(e.state.cards.len() as u32 - 1)
    };
    let t = tears(&mut e);
    blocked_bleed(&mut e, viri, z);
    let prevent = use_at(t, 0, None, Target::None);
    let req = offering(&mut e, prevent);
    e.choose(index_of(&req, prevent));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, viri), 3);
    assert!(!e.state.cards[t.index()].in_play);
    let t2 = tears(&mut e);
    let feed = use_at(t2, 1, None, Target::None);
    let req = offering(&mut e, feed);
    e.choose(index_of(&req, feed));
    request(&mut e);
    assert_eq!(blood(&e, viri), 3, "capacity 3: the excess ignored");
}

// Concoction of Vitality: "When the vampire with this card announces an
// action, he or she may burn this card to prevent vampires from blocking
// that action. If he or she does so, this vampire cannot play cards
// requiring Disciplines until the action is complete." [LSJ 20100402] "Can
// be used after declaring an action using a discipline": Viri's bleed —
// the vampire blocker is not offered a block, her Walk of Caine (a
// Discipline card) not offered her.
#[test]
fn concoction_of_vitality_bars_vampire_blocks() {
    let mut e = game(2);
    let viri = budapest(&mut e, 0, "Viri", 3);
    budapest(&mut e, 0, "Zizi", 3);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cv = {
        let card = find_library("Concoction of Vitality");
        e.state
            .cards
            .push(CardInPlay::new(card, SeatId(0), Some(viri), 0));
        CardId(e.state.cards.len() as u32 - 1)
    };
    hand(&mut e, 0, &["Walk of Caine"]);
    pick(&mut e, bleed(viri));
    let burn = use_at(cv, 0, Some(viri), Target::None);
    let req = offering(&mut e, burn);
    e.choose(index_of(&req, burn));
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play(0, 0, viri)),
        "no Discipline card"
    );
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(z));
    assert!(!e.state.cards[cv.index()].in_play);
}

// NRA PAC: "Any minion who successfully performs an equip action unlocks at
// the end of the turn." [LSJ 20061218] "Does not affect equip actions
// performed prior to its arrival in play" and [LSJ 20080619] "The minions
// who performed an equip action when it was in play unlock at the end of
// turn regardless of whether NRA PAC is still in play or not": Viri equips
// the Monocle with the event in play — unlocked as the turn ends, the
// event burned meanwhile.
#[test]
fn nra_pac_unlocks_equippers() {
    let mut e = game(2);
    let viri = budapest(&mut e, 0, "Viri", 3);
    budapest(&mut e, 0, "Zizi", 3);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let pac = {
        let card = find_library("NRA PAC");
        let id = CardId(e.state.cards.len() as u32);
        e.state
            .cards
            .push(CardInPlay::new(card, SeatId(0), None, 0));
        e.state.layers.push(zvtes::cards::Layer {
            scope: zvtes::cards::Scope::InPlay,
            modifier: zvtes::cards::Modifier::EquipUnlocksEndOfTurn,
            source: id,
        });
        id
    };
    hand(&mut e, 0, &["Monocle of Clarity"]);
    pick(&mut e, play(0, 0, viri));
    request(&mut e);
    assert!(e.state.minions[viri.index()].unlock_end_of_turn);
    e.state.cards[pac.index()].in_play = false;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[viri.index()].locked);
}

// ---- Step 10: As carequinha ---------------------------------------------------

// Fortitude, Vicissitude: "Put this card on a vampire. This vampire gains 1
// level of <Discipline>. Capacity increases by 1 … Cannot be placed on a
// vampire with superior <Discipline>." On Viri ([vic] basic): superior
// Vicissitude, capacity 4; Fortitude on Zizi: [for], capacity 4; no
// Vicissitude on Gabi ([VIC] already).
#[test]
fn fortitude_and_vicissitude_raise_a_discipline() {
    let mut e = game(2);
    let viri = budapest(&mut e, 0, "Viri", 3);
    let zizi = budapest(&mut e, 0, "Zizi", 3);
    let gabi = budapest(&mut e, 0, "Gabi", 3);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Vicissitude", "Fortitude"]);
    let on = |card, m| Choice::Play {
        card,
        mode: 0,
        actor: None,
        target: Target::Minion(m),
    };
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(!req.options.contains(&on(0, gabi)));
    e.choose(index_of(&req, on(0, viri)));
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let k = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Fortitude"))
        .expect("Fortitude") as u8;
    e.choose(index_of(&req, on(k, zizi)));
    request(&mut e);
    assert!(e.state.has_discipline(viri, "VIC"));
    assert_eq!(e.state.capacity_of(viri), Some(4));
    assert!(e.state.has_discipline(zizi, "for"));
    assert_eq!(e.state.capacity_of(zizi), Some(4));
}

// Horrid Form: "Only usable before range is determined. [vic] This vampire
// gets +1 strength this combat. [VIC] As above, and this vampire can
// prevent 1 damage each round." Gabi's [VIC]: her hand strike does 2, the
// blocker's 1 to her prevented.
#[test]
fn horrid_form_strengthens_and_shields() {
    let mut e = game(2);
    let gabi = budapest(&mut e, 0, "Gabi", 4);
    budapest(&mut e, 0, "Viri", 3);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Horrid Form"]);
    blocked_bleed(&mut e, gabi, z);
    let before = blood(&e, z);
    let req = offering(&mut e, play(0, 1, gabi));
    e.choose(index_of(&req, play(0, 1, gabi)));
    let req = offering(&mut e, Choice::UsePrevent);
    e.choose(index_of(&req, Choice::UsePrevent));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(before - blood(&e, z), 2);
    assert_eq!(blood(&e, gabi), 4 - 1, "its blood cost only");
}

// Inner Essence: "Only usable before range is determined. A vampire can play
// only one Inner Essence each combat. [vic] Once this combat, this vampire
// can reduce the cost of a combat card they play by 1 blood. [VIC] As
// above, and this vampire gains 1 blood." Gabi's [VIC]: +1 blood; her
// Breath of the Dragon then costs her nothing, the card burned for it; a
// second Inner Essence not offered.
#[test]
fn inner_essence_discounts_a_combat_card() {
    let mut e = game(2);
    let gabi = budapest(&mut e, 0, "Gabi", 3);
    budapest(&mut e, 0, "Viri", 3);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(
        &mut e,
        0,
        &["Inner Essence", "Inner Essence", "Breath of the Dragon"],
    );
    blocked_bleed(&mut e, gabi, z);
    let req = offering(&mut e, play(0, 1, gabi));
    e.choose(index_of(&req, play(0, 1, gabi)));
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 1, gabi)), "one each combat");
    let essence = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(blood(&e, gabi), 3 + 1);
    let breath = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Breath of the Dragon"))
        .expect("in hand") as u8;
    let req = offering(&mut e, play(breath, 0, gabi));
    e.choose(index_of(&req, play(breath, 0, gabi)));
    // The discount announced with the card: the essence burned, the
    // breath's blood not paid.
    let off = Choice::ResolveRider { card: essence.0 };
    pick(&mut e, off);
    request(&mut e);
    assert!(!e.state.cards[essence.index()].in_play);
    // The breath free: only the blocker's hand strike burns her blood, her
    // own aggravated point sending her to torpor.
    assert_eq!(blood(&e, gabi), 4 - 1);
}

// Skin of Steel: "[for] Prevent all damage from the opponent's strike."
// [LSJ 20001114] "Cannot be used if there is no damage to prevent": Gabi
// blocked, the hand strike's damage prevented.
#[test]
fn skin_of_steel_stops_the_strike() {
    let mut e = game(2);
    let gabi = budapest(&mut e, 0, "Gabi", 4);
    budapest(&mut e, 0, "Viri", 3);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Skin of Steel"]);
    blocked_bleed(&mut e, gabi, z);
    let req = offering(&mut e, play(0, 0, gabi));
    assert_eq!(req.window, WindowLabel::CombatPrevention);
    e.choose(index_of(&req, play(0, 0, gabi)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, gabi), 4 - 1, "its blood cost only");
}

// New Blood (owner, rules-qa 68.1): "choose any circle" — a printed one, or
// his own circle of one. Choosing Torrance, he shares Ilse's circle;
// keeping his own, none.
#[test]
fn new_blood_chooses_a_circle_or_keeps_his_own() {
    for torrance in [true, false] {
        let mut e = game(2);
        let ilse = e.add_minion(SeatId(0), find_card("Ilse"), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.seats[0]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card: find_card("New Blood"),
                copy: Default::default(),
                counters: 4,
                banished: None,
            });
        let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
        e.choose(index_of(&req, Choice::InfluenceOut { slot: 0 }));
        let req = request(&mut e);
        assert!(req.options.contains(&Choice::Pass));
        let nb = MinionId(e.state.minions.len() as u32 - 1);
        e.choose(index_of(
            &req,
            if torrance {
                Choice::ChooseCircle("Torrance")
            } else {
                Choice::Pass
            },
        ));
        assert_eq!(e.state.same_circle(nb, ilse), torrance);
    }
}

// Slake the Thirst [ANK 20201228] "Can be played after a diablerie, before
// the blood hunt ([SAN] after a normal diablerie)" (owner, rules-qa 68.2):
// a Hermana diablerizes a torpid vampire; before the blood hunt, Slake gives
// the other Hermana as much as she gained.
#[test]
fn slake_the_thirst_after_a_diablerie() {
    let mut e = game(2);
    let a = hermana(&mut e, 0, 1);
    superior(&mut e, a, "san");
    let b = hermana(&mut e, 0, 1);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Slake the Thirst"]);
    let taken = blood(&e, z);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: ActionKind::Diablerie,
            target: Some(z),
        },
    );
    let slake = play_at(0, 2, a, Target::Minion(b));
    let req = offering(&mut e, slake);
    assert!(e.referendum().is_none(), "before the blood hunt");
    e.choose(index_of(&req, slake));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), (1 + taken).min(4));
}

// Slake the Thirst (owner, rules-qa 68.10): each move of Redistribution is a
// gain of its own, answered alone — Viri (1 blood) gains 1 (the card),
// then 2 from Zizi (to her capacity of 3: 1), then… — Slake offers each
// move's amount, not their sum.
#[test]
fn slake_the_thirst_answers_one_move() {
    let mut e = game(2);
    let viri = budapest(&mut e, 0, "Viri", 1);
    let zizi = budapest(&mut e, 0, "Zizi", 3);
    let dorka = budapest(&mut e, 0, "Dorka", 3);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Redistribution", "Slake the Thirst"]);
    superior(&mut e, viri, "san");
    pick(&mut e, play(0, 0, viri));
    for (from, n) in [(zizi, 2), (dorka, 1)] {
        pick(&mut e, Choice::ChooseMinion(from));
        pick(&mut e, Choice::ChooseMinion(viri));
        pick(&mut e, Choice::SetX(n));
    }
    pick(&mut e, Choice::Pass);
    let slake = play_at(0, 2, viri, Target::Minion(zizi));
    let req = offering(&mut e, slake);
    e.choose(index_of(&req, slake));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::PlayTerms);
    assert!(req.options.contains(&Choice::SetX(2)));
    assert!(!req.options.contains(&Choice::SetX(4)));
}

// The Shard, London [ANK 20230721] [ANK 20230621] "Can allow allies to play cards that
// otherwise cost blood (eg. Procurer), if they do not otherwise require a
// vampire": a Procurer recruits another Procurer (2 blood) with The Shard
// and the Edge, the Shard announced with it; without the Edge, not offered.
#[test]
fn the_shard_lets_an_ally_pay_a_blood_cost() {
    for edge in [true, false] {
        let mut e = game(2);
        let viri = budapest(&mut e, 0, "Viri", 3);
        budapest(&mut e, 0, "Zizi", 3);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let shard = in_play(&mut e, 0, "Shard, London, The", None);
        hand(&mut e, 0, &["Procurer", "Procurer"]);
        pick(&mut e, play(0, 0, viri));
        let _ = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let p = MinionId(e.state.minions.len() as u32 - 1);
        assert!(e.state.minions[p.index()].is_ally());
        e.state.minions[p.index()].recruited = false;
        e.state.seats[0].has_edge = edge;
        for s in 1..e.state.seats.len() {
            e.state.seats[s].has_edge = false;
        }
        let req = request(&mut e);
        let k = e.state.seats[0]
            .hand
            .iter()
            .position(|&c| c == find_library("Procurer"))
            .expect("in hand") as u8;
        assert_eq!(req.options.contains(&play(k, 0, p)), edge);
        if edge {
            e.choose(index_of(&req, play(k, 0, p)));
            // The Shard the only way to pay: locked with the card.
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(e.state.cards[shard.index()].locked);
            assert_eq!(
                e.state
                    .minions
                    .iter()
                    .filter(|m| m.present() && m.is_ally())
                    .count(),
                2
            );
        }
    }
}

// The Line [ANK 20181007] "Can reduce the cost in blood (not in pool) of any
// action (ally, retainer, equipment, political)": Viri recruits Procurer
// (2 blood) for 1, The Line locked.
#[test]
fn the_line_takes_a_blood_off_a_recruit() {
    let mut e = game(2);
    let viri = budapest(&mut e, 0, "Viri", 3);
    budapest(&mut e, 0, "Zizi", 3);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let line = in_play(&mut e, 0, "Line, The", None);
    hand(&mut e, 0, &["Procurer"]);
    pick(&mut e, play(0, 0, viri));
    // Used during the action, before its cost is paid.
    pick_when(&mut e, use_at(line, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, viri), 3 - 1);
    assert!(e.state.cards[line.index()].locked);
}

// Redistribution (owner, rules-qa 68.10): equipment moves as many as
// wanted, each a move of its own; blood once between two given Blood
// Brothers — Zizi's Monocle and Blood Tears both go to Dorka; after 1 blood
// from Viri to Zizi, Viri may not move blood to Zizi again (to Dorka she
// may).
#[test]
fn redistribution_moves_equipment_freely_and_blood_once_a_pair() {
    let mut e = game(2);
    let viri = budapest(&mut e, 0, "Viri", 3);
    let zizi = budapest(&mut e, 0, "Zizi", 1);
    let dorka = budapest(&mut e, 0, "Dorka", 1);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let monocle = in_play(&mut e, 0, "Monocle of Clarity", Some(zizi));
    let tears = in_play(&mut e, 0, "Blood Tears of Kephran", Some(zizi));
    hand(&mut e, 0, &["Redistribution"]);
    pick(&mut e, play(0, 0, viri));
    for c in [monocle, tears] {
        pick_when(&mut e, Choice::ChooseCard(c.0));
        pick(&mut e, Choice::ChooseMinion(dorka));
    }
    pick(&mut e, Choice::ChooseMinion(viri));
    pick(&mut e, Choice::ChooseMinion(zizi));
    pick(&mut e, Choice::SetX(1));
    pick(&mut e, Choice::ChooseMinion(viri));
    let req = request(&mut e);
    assert!(
        !req.options.contains(&Choice::ChooseMinion(zizi)),
        "{:?}",
        req.options
    );
    assert!(req.options.contains(&Choice::ChooseMinion(dorka)));
    assert_eq!(e.state.cards[monocle.index()].host, Some(dorka));
    assert_eq!(e.state.cards[tears.index()].host, Some(dorka));
}
