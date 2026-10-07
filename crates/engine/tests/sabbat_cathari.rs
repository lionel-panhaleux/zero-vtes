//! Sabbat V5, the Path of Cathari: the deck's new cards (`ADDED_BATCHES`,
//! a fifth batch). Readings in rules-qa Batch 53. Windows with a single
//! option are elided, so the scripts only stop where a real decision
//! exists.

use zvtes::cards::{self, AbilitySource, Effect, Layer, Scope, Stat, Target};
use zvtes::crypt::{Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::decks::Deck;
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

fn card(name: &str) -> u16 {
    find_card(name)
}

/// The first crypt card meeting `f`.
fn vampire(f: impl Fn(&zvtes::crypt::CryptCard) -> bool) -> u16 {
    CRYPT.iter().position(f).expect("a vampire") as u16
}

/// An untitled vampire with `disc` and capacity 4 or more.
fn with(disc: &str) -> u16 {
    vampire(|c| c.capacity >= 4 && c.title == Title::None && c.disciplines.contains(&disc))
}

const AELSWITH: &str = "Aelswith, The Irresistible";
const AGNIESZKA: &str = "Agnieszka, Tempter of Legions";
const JURGEN: &str = "Jürgen, The Libertine";
const OSVALDO: &str = "Osvaldo Kühnemann";
const VERRIX: &str = "Verrix, Naughty Boy";

/// Seat 0: `name` (4 blood) and a spare; seat 1 a blocker (4 blood).
fn table(name: &str) -> (Engine, MinionId, MinionId) {
    let mut e = game(3);
    let m = e.add_minion(SeatId(0), card(name), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    (e, m, z)
}

fn bleed_blocked(e: &mut Engine, a: MinionId, b: MinionId) {
    pick(e, bleed(a));
    pick(e, block(b));
}

// ---- The deck ------------------------------------------------------------------

// The Path of Cathari precon: 12 crypt and 77 library cards, every card
// defined but the vampires printing no text; the second Sabbat deck, after
// the Path of Power (decks in the order they were completed).
#[test]
fn the_path_of_cathari_deck_is_complete() {
    let deck = Deck::sabbat(1);
    assert_eq!(deck.name, "Path of Cathari (Sabbat V5)");
    assert_eq!(
        Deck::sabbat(0).name,
        "Path of Power and the Inner Voice (Sabbat V5)"
    );
    assert_eq!(deck.crypt_cards().len(), 12);
    assert_eq!(deck.library_cards().len(), 77);
    assert_eq!(deck.path(), Some("Cathari"));
    for c in deck.library_cards() {
        assert!(cards::lookup(c).is_some(), "{c}");
    }
}

// ---- Vampires --------------------------------------------------------------------

// Aelswith and Terrifying Visage: "If this vampire is blocked, the blocking
// minion's controller burns 1 pool before block resolution." [ANK
// 20220116] [ANK 20211207] [ANK 20210627] [RTR 19991206] [LSJ 20091125]
// "When blocked, all 'if blocked' effects must be applied, ordered by the
// acting Methuselah, before any other card or effect can be used": both
// apply as the block is declared, before any window. [LSJ 20011022] "The
// passive ability applies even if the vampire is in torpor": blocked
// leaving torpor, the blocker's controller pays too.
#[test]
fn aelswith_blocked_costs_the_blockers_controller_a_pool() {
    let (mut e, aelswith, z) = table(AELSWITH);
    bleed_blocked(&mut e, aelswith, z);
    request(&mut e);
    assert_eq!(e.state.seats[1].pool, 9);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9, "blocked: no bleed");

    // With Terrifying Visage on her: 2.
    let (mut e, aelswith, z) = table(AELSWITH);
    in_play(&mut e, 0, "Terrifying Visage", Some(aelswith));
    bleed_blocked(&mut e, aelswith, z);
    request(&mut e);
    assert_eq!(e.state.seats[1].pool, 8);

    // In torpor, leaving it: an [aus] blocker with Eagle's Sight.
    let mut e = game(3);
    let aelswith = e.add_minion(SeatId(0), card(AELSWITH), 4, true);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), with("aus"), 4, false);
    hand(&mut e, 1, &["Eagle's Sight"]);
    pick(&mut e, action(aelswith, ActionKind::LeaveTorpor));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9);
}

// The pool burned ousts the blocker's controller: their minion leaves
// play with them, the action is blocked with no combat (rules-qa 53.1).
#[test]
fn a_blockers_controller_ousted_by_aelswith_leaves_no_combat() {
    let (mut e, aelswith, z) = table(AELSWITH);
    e.add_minion(SeatId(2), plain("Brujah", 2), 4, false);
    e.state.seats[1].pool = 1;
    bleed_blocked(&mut e, aelswith, z);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert!(!e.state.minions[z.index()].in_play);
    assert_eq!(blood(&e, aelswith), 4, "no combat");
    assert!(e.state.minions[aelswith.index()].locked);
}

// "Agnieszka can burn 1 blood to strike: combat ends. +1 bleed."
#[test]
fn agnieszka_bleeds_harder_and_buys_a_combat_ends_strike() {
    let (mut e, agnieszka, z) = table(AGNIESZKA);
    pick(&mut e, bleed(agnieszka));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 8);

    let (mut e, agnieszka, z2) = table(AGNIESZKA);
    let _ = z;
    bleed_blocked(&mut e, agnieszka, z2);
    pick_when(&mut e, vampire_ability(agnieszka, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, agnieszka), 3, "the strike's blood, no damage");
    assert_eq!(blood(&e, z2), 4, "combat ended before damage");
}

// [LSJ 20011022] Devorah: "While you have 5 or fewer pool, Devorah gets +1
// stealth" — in torpor too.
#[test]
fn devorah_gets_stealth_while_her_controller_is_low_even_in_torpor() {
    let mut e = game(3);
    let devorah = e.add_minion(SeatId(0), card("Devorah"), 4, true);
    let ctx = PrintedCtx::default();
    assert_eq!(e.state.printed_mod(devorah, Stat::Stealth, ctx), 0);
    e.state.seats[0].pool = 5;
    assert_eq!(e.state.printed_mod(devorah, Stat::Stealth, ctx), 1);
}

// Fotini: "After Fotini successfully bleeds (for 1 or more), you get +1
// hand size until your next discard phase": 8 cards drawn up, back to 7
// as the discard phase begins (rules-qa 53.4).
#[test]
fn fotini_gives_a_hand_size_until_the_discard_phase() {
    let (mut e, fotini, _) = table("Fotini Katsikaris");
    hand(&mut e, 0, &["Enthrall"; 7]);
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
    pick(&mut e, bleed(fotini));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.hand_size(SeatId(0)), 8);
    until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
}

// Gostoso: "After Gostoso successfully bleeds your prey (for 1 or more),
// he can gain 1 blood."
#[test]
fn gostoso_gains_a_blood_after_bleeding_his_prey() {
    let (mut e, gostoso, _) = table("Gostoso");
    pick(&mut e, bleed(gostoso));
    pick_when(&mut e, vampire_ability(gostoso, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, gostoso), 5);
}

// [LSJ 20011022] Jürgen: "Minions must burn 1 blood or life to attempt to
// block Jürgen" — in torpor too: leaving it, the [aus] blocker pays.
#[test]
fn jurgens_blockers_burn_a_blood_even_in_torpor() {
    let (mut e, jurgen, z) = table(JURGEN);
    bleed_blocked(&mut e, jurgen, z);
    assert_eq!(blood(&e, z), 3, "paid as the attempt is declared");

    let mut e = game(3);
    let jurgen = e.add_minion(SeatId(0), card(JURGEN), 4, true);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), with("aus"), 4, false);
    hand(&mut e, 1, &["Eagle's Sight"]);
    pick(&mut e, action(jurgen, ActionKind::LeaveTorpor));
    pick_when(&mut e, block(z));
    assert_eq!(blood(&e, z), 3);
}

// Osvaldo: "+1 bleed against a Methuselah controlling a Toreador or
// Ventrue."
#[test]
fn osvaldo_bleeds_harder_against_a_toreador_or_ventrue_controller() {
    let (mut e, osvaldo, _) = table(OSVALDO);
    pick(&mut e, bleed(osvaldo));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9);

    let (mut e, osvaldo, _) = table(OSVALDO);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    pick(&mut e, bleed(osvaldo));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.seats[1].pool, 8,
        "a Ventrue in torpor is controlled"
    );
}

// Verrix: "+1 stealth during bleed actions" — not a hunt.
#[test]
fn verrix_has_stealth_on_bleeds_only() {
    let (e, verrix, _) = table(VERRIX);
    let on = |kind| PrintedCtx {
        kind: Some(kind),
        ..Default::default()
    };
    assert_eq!(
        e.state
            .printed_mod(verrix, Stat::Stealth, on(ActionKind::Bleed)),
        1
    );
    assert_eq!(
        e.state
            .printed_mod(verrix, Stat::Stealth, on(ActionKind::Hunt)),
        0
    );
}

// ---- Library cards -----------------------------------------------------------------

// Enthrall [OBF] or [PRE]: a bleed with +1 bleed; successful, its vampire
// can burn 1 blood to draw a crypt card — [LSJ 20060620] "You can draw
// more cards than are in the library or crypt (even from an empty library
// or crypt); draw as many as possible": an empty crypt, the blood for
// nothing.
#[test]
fn enthrall_bleeds_and_buys_a_crypt_card() {
    for empty in [false, true] {
        let (mut e, glissando, _) = table(VERRIX);
        if empty {
            e.state.seats[0].crypt.clear();
        } else {
            e.state.seats[0].crypt.push(card("Ximena"));
        }
        let before = e.state.seats[0].uncontrolled.len();
        hand(&mut e, 0, &["Enthrall"]);
        pick(&mut e, play(0, 1, glissando));
        pick_when(&mut e, Choice::DrawCrypt);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 8, "+1 bleed");
        assert_eq!(blood(&e, glissando), 3);
        assert_eq!(
            e.state.seats[0].uncontrolled.len(),
            before + usize::from(!empty)
        );
    }
}

// Fever Pitch [FOR] or [PRE]: +1 bleed (limited); successful, the card on
// the vampire (rules-qa 53.5); on a later bleed of theirs, burned to have
// a block attempt fail — the blocker not offered again.
#[test]
fn fever_pitch_stays_to_foil_a_later_block() {
    let (mut e, devorah, z) = table("Devorah");
    e.state.minions[devorah.index()].blood = 5;
    hand(&mut e, 0, &["Fever Pitch"]);
    let fever = find_library("Fever Pitch");
    pick(&mut e, bleed(devorah));
    pick_when(&mut e, play(0, 1, devorah));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 8);
    let on: Vec<CardId> = (0..e.state.cards.len() as u32)
        .map(CardId)
        .filter(|&c| {
            let x = &e.state.cards[c.index()];
            x.in_play && x.card == fever && x.host == Some(devorah)
        })
        .collect();
    assert_eq!(on.len(), 1, "on Devorah");
    let id = on[0];

    // Next bleed: blocked, the card burned to fail it.
    e.state.minions[devorah.index()].locked = false;
    e.state.minions[devorah.index()].bled_this_turn = false;
    e.state.minions[devorah.index()].acted.clear();
    bleed_blocked(&mut e, devorah, z);
    pick_when(&mut e, use_at(id, 0, None, Target::None));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(z));
    assert!(!e.state.cards[id.index()].in_play);
    assert_eq!(e.state.seats[1].pool, 8 - 1);
}

// Forward Momentum: a counter after a Cathari vampire's successful bleed,
// one off after an unsuccessful one (blocked: rules-qa 53.6); 2 burned to
// unlock that vampire after its action.
#[test]
fn forward_momentum_counts_bleeds_and_unlocks() {
    let (mut e, devorah, z) = table("Devorah");
    let fm = in_play(&mut e, 0, "Forward Momentum", None);
    e.state.cards[fm.index()].counters = 1;
    pick(&mut e, bleed(devorah));
    let unlock = use_at(fm, 0, None, Target::None);
    pick_when(&mut e, unlock);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[fm.index()].counters, 0, "1 + 1 - 2");
    assert!(!e.state.minions[devorah.index()].locked);

    // Blocked: one off.
    e.state.cards[fm.index()].counters = 1;
    e.state.minions[devorah.index()].bled_this_turn = false;
    e.state.minions[devorah.index()].acted.clear();
    bleed_blocked(&mut e, devorah, z);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, unlock);
    assert_eq!(e.state.cards[fm.index()].counters, 0);
}

// Hedonism [for] or [pre]: played by another ready unlocked vampire of the
// acting Methuselah as a minion attempts to block — the attempt fails,
// both lock, and a combat between them is queued. [ANK 20170918-2] "The
// new combat is still part of the action": it begins once the block step
// is over (rules-qa 53.3), and the bleed then resolves. [LSJ 20021121]
// "Other effects that queue a combat cannot be used until the new combat
// begins": a second Hedonism [pre] is not offered against the next
// attempt, its [PRE] (no combat) is.
#[test]
fn hedonism_fails_an_attempt_and_queues_a_combat_in_the_action() {
    let (mut e, bleeder, z) = table("Ximena");
    let devorah = e.add_minion(SeatId(0), card("Devorah"), 4, false);
    let y = e.add_minion(SeatId(1), plain("Brujah", 2), 4, false);
    let gostoso = e.add_minion(SeatId(0), card("Gostoso"), 4, false);
    hand(&mut e, 0, &["Hedonism", "Hedonism"]);
    bleed_blocked(&mut e, bleeder, z);
    pick_when(&mut e, play(0, 0, devorah));
    request(&mut e);
    assert!(e.state.minions[z.index()].locked && e.state.minions[devorah.index()].locked);
    // The next attempt: Gostoso's Hedonism queues no second combat.
    pick_when(&mut e, block(y));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionBlock);
    assert!(
        req.options.contains(&play(0, 1, gostoso)),
        "[PRE]: no queue"
    );
    assert!(
        !req.options.contains(&play(0, 0, gostoso)),
        "one queued at a time"
    );

    // Unblocked after the failed attempt: the queued combat, then the
    // bleed.
    let (mut e, bleeder, z) = table("Ximena");
    let devorah = e.add_minion(SeatId(0), card("Devorah"), 4, false);
    hand(&mut e, 0, &["Hedonism"]);
    bleed_blocked(&mut e, bleeder, z);
    pick_when(&mut e, play(0, 0, devorah));
    // Its hand strikes only: the combat elides.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        (blood(&e, devorah), blood(&e, z)),
        (3, 3),
        "the queued combat"
    );
    assert_eq!(e.state.seats[1].pool, 9, "the bleed resolved after it");
}

// [LSJ 20090826] Hedonism: "Combat does not occur if the action ends
// prematurely": another minion blocks, Change of Target ends the action
// before block resolution — no combat at all. [FOR] or [PRE]: no combat
// queued.
#[test]
fn hedonisms_combat_is_lost_if_the_action_ends_first() {
    let (mut e, bleeder, z) = table("Ximena");
    let devorah = e.add_minion(SeatId(0), card("Devorah"), 4, false);
    let y = e.add_minion(SeatId(1), plain("Brujah", 2), 4, false);
    hand(&mut e, 0, &["Hedonism", "Change of Target"]);
    bleed_blocked(&mut e, bleeder, z);
    pick_when(&mut e, play(0, 0, devorah));
    pick_when(&mut e, block(y));
    pick_when(&mut e, play(0, 0, bleeder));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |_| false);
    assert_eq!((blood(&e, devorah), blood(&e, z), blood(&e, y)), (4, 4, 4));

    // [FOR] or [PRE]: both locked, no combat.
    let (mut e, bleeder, z) = table("Ximena");
    let devorah = e.add_minion(SeatId(0), card("Devorah"), 4, false);
    hand(&mut e, 0, &["Hedonism"]);
    bleed_blocked(&mut e, bleeder, z);
    pick_when(&mut e, play(0, 1, devorah));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].locked && e.state.minions[devorah.index()].locked);
    assert_eq!((blood(&e, devorah), blood(&e, z)), (4, 4));
    assert_eq!(e.state.seats[1].pool, 9);
}

// The Hungry Coyote: "Sabbat vampires you control get +1 hunt": 2 blood
// for a Sabbat vampire, 1 for another sect's. [RTR 20030519] [RTR
// 20180511] "The hunt bonus applies to non-standard hunt actions, blood
// comes from the target of the hunt": Restricted Vitae's steal takes 2.
#[test]
fn the_hungry_coyote_feeds_sabbat_hunts() {
    let (mut e, devorah, _) = table("Devorah");
    let spare = e
        .state
        .minions_of(SeatId(0))
        .find(|&m| m != devorah)
        .expect("spare");
    in_play(&mut e, 0, "Hungry Coyote, The", None);
    e.state.minions[devorah.index()].blood = 1;
    pick(&mut e, action(devorah, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, devorah), 3);
    e.state.minions[spare.index()].blood = 1;
    pick(&mut e, action(spare, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, spare), 2, "not Sabbat");

    let (mut e, devorah, _) = table("Devorah");
    in_play(&mut e, 0, "Hungry Coyote, The", None);
    let vitae = in_play(&mut e, 2, "Restricted Vitae", None);
    let older = e.add_minion(SeatId(2), card(AELSWITH), 4, false);
    let _ = older;
    let young = e.add_minion(SeatId(1), card("Cyan Sin"), 2, false);
    e.state.minions[devorah.index()].blood = 1;
    e.state.minions[devorah.index()].must_hunt = true;
    pick_when(
        &mut e,
        use_at(vitae, 0, Some(devorah), Target::Minion(young)),
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, young), 0);
    assert_eq!(blood(&e, devorah), 3);
}

// Kiss of Cathari [OBF] or [PRE]: combat ends; after it, at close range,
// the opposing minion is stunned (locked, not unlocking next time). [LSJ
// 20070124] "If the strike is dodged, the opposing minion is not stunned."
// [LSJ 20040601] "If the opponent unlocks before combat ends (eg. Majesty),
// the effect locks them again after it ends." [RTR 20020501] [LSJ
// 19980109] "If the combat continues or a new combat begins, the after
// combat effect does not happen": Telepathic Tracking.
#[test]
fn kiss_of_cathari_stuns_after_combat() {
    let (mut e, ximena, z) = table(AGNIESZKA);
    hand(&mut e, 0, &["Kiss of Cathari"]);
    bleed_blocked(&mut e, ximena, z);
    pick_when(&mut e, play(0, 1, ximena));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[z.index()];
    assert!(v.locked && v.skip_unlock, "stunned");

    // Dodged: nothing.
    let (mut e, ximena, _) = table(AGNIESZKA);
    let z = e.add_minion(SeatId(1), with("cel"), 4, false);
    hand(&mut e, 0, &["Kiss of Cathari"]);
    hand(&mut e, 1, &["Side Strike"]);
    bleed_blocked(&mut e, ximena, z);
    pick_when(&mut e, play(0, 1, ximena));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].skip_unlock);

    // Majesty unlocks its vampire: locked again.
    let (mut e, ximena, _) = table(AGNIESZKA);
    let z = e.add_minion(SeatId(1), with("pre"), 4, false);
    hand(&mut e, 0, &["Kiss of Cathari"]);
    hand(&mut e, 1, &["Majesty"]);
    bleed_blocked(&mut e, ximena, z);
    pick_when(&mut e, play(0, 1, ximena));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[z.index()];
    assert!(v.locked && v.skip_unlock, "locked again");

    // Combat continues: nothing.
    let (mut e, ximena, _) = table(AGNIESZKA);
    let z = e.add_minion(SeatId(1), with("AUS"), 5, false);
    hand(&mut e, 0, &["Kiss of Cathari"]);
    hand(&mut e, 1, &["Telepathic Tracking"]);
    bleed_blocked(&mut e, ximena, z);
    pick_when(&mut e, play(0, 1, ximena));
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].skip_unlock);
}

// Seeds of Terror [obf] or [pre], as the action is announced: minions burn
// 1 blood to attempt to block — an ally has none (rules-qa 53.2). [OBF] or
// [PRE]: the chosen vampire cannot block.
#[test]
fn seeds_of_terror_makes_blockers_pay_blood() {
    let (mut e, ximena, z) = table("Ximena");
    let deuce = e.add_ally(SeatId(1), find_library("Double Deuce"));
    hand(&mut e, 0, &["Seeds of Terror"]);
    pick(&mut e, bleed(ximena));
    pick_when(&mut e, play(0, 0, ximena));
    let req = offering(&mut e, block(z));
    assert!(
        !req.options.contains(&block(deuce)),
        "an ally pays no blood"
    );
    pick(&mut e, block(z));
    assert_eq!(blood(&e, z), 3);

    let (mut e, ximena, z) = table(AGNIESZKA);
    let y = e.add_minion(SeatId(1), plain("Brujah", 2), 4, false);
    hand(&mut e, 0, &["Seeds of Terror"]);
    pick(&mut e, bleed(ximena));
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 1,
            actor: Some(ximena),
            target: Target::Minion(z),
        },
    );
    let req = offering(&mut e, block(y));
    assert!(!req.options.contains(&block(z)));
}

// Subversion [cel] or [pre]: +1 stealth; [CEL] or [PRE]: +1 bleed
// (limited), or +1 stealth and +1 bleed (limited) — the stealth only
// when needed.
#[test]
fn subversion_adds_stealth_or_bleed() {
    let (mut e, osvaldo, _) = table(OSVALDO);
    hand(&mut e, 0, &["Subversion"]);
    pick(&mut e, bleed(osvaldo));
    pick_when(&mut e, play(0, 1, osvaldo));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 8);
}

// Swift Cover [cel] or [obf]: strike: dodge; [CEL] or [OBF]: +1 stealth.
#[test]
fn swift_cover_dodges_or_gives_stealth() {
    let mut e = game(3);
    let bleeder = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 3), 4, false);
    let glissando = e.add_minion(SeatId(1), card("Glissando"), 4, false);
    hand(&mut e, 1, &["Swift Cover"]);
    bleed_blocked(&mut e, bleeder, glissando);
    pick_when(&mut e, play(0, 0, glissando));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, glissando), 4, "dodged");

    // [CEL]: +1 stealth on Glissando's own bleed, the block failing.
    let mut e = game(3);
    let glissando = e.add_minion(SeatId(0), card("Glissando"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    hand(&mut e, 0, &["Swift Cover"]);
    bleed_blocked(&mut e, glissando, z);
    pick_when(&mut e, play(0, 1, glissando));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9, "unblocked");
}

// Terrifying Visage: put on a ready vampire following the Path of Cathari
// — any Methuselah's (rules-qa 53.7), not one on another path; one per
// vampire.
#[test]
fn terrifying_visage_goes_on_a_ready_cathari_vampire() {
    let mut e = game(3);
    let devorah = e.add_minion(SeatId(0), card("Devorah"), 4, false);
    let other = e.add_minion(SeatId(1), card("Ximena"), 3, false);
    let sleeping = e.add_minion(SeatId(1), card("Gostoso"), 3, true);
    let power = e.add_minion(SeatId(0), card("Concordia"), 4, false);
    hand(&mut e, 0, &["Terrifying Visage"]);
    e.at_master_phase();
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let on = |m| Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(m),
    };
    assert!(req.options.contains(&on(devorah)));
    assert!(req.options.contains(&on(other)), "another's");
    assert!(!req.options.contains(&on(sleeping)), "in torpor");
    assert!(!req.options.contains(&on(power)), "another path");
}
