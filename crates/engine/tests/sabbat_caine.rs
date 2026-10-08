//! Sabbat V5, the Path of Caine: the deck's new cards (`ADDED_BATCHES`, a
//! sixth batch). Readings in rules-qa Batch 54. Windows with a single
//! option are elided, so the scripts only stop where a real decision
//! exists.

use zvtes::cards::{self, AbilitySource, Effect, Layer, Scope, Stat, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
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

/// A card already in play for `seat`, on `host` if given, with mode
/// `mode`'s layers (no play): its id.
fn in_play_mode(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>, mode: u8) -> CardId {
    let card = find_library(name);
    e.state
        .cards
        .push(CardInPlay::new(card, SeatId(seat), host, mode));
    let id = CardId(e.state.cards.len() as u32 - 1);
    for effect in cards::lookup(card).expect("definition").modes[mode as usize].effects {
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

fn in_play(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>) -> CardId {
    in_play_mode(e, seat, name, host, 0)
}

fn card(name: &str) -> u16 {
    find_card(name)
}

/// The first crypt card meeting `f`.
fn vampire(f: impl Fn(&zvtes::crypt::CryptCard) -> bool) -> u16 {
    CRYPT.iter().position(f).expect("a vampire") as u16
}

/// An untitled vampire with `disc` and capacity in `caps`.
fn with(disc: &str, caps: std::ops::RangeInclusive<u8>) -> u16 {
    vampire(|c| {
        caps.contains(&c.capacity) && c.title == Title::None && c.disciplines.contains(&disc)
    })
}

const ALGIRDAS: &str = "Algirdas, The Solar Prophet";
const ANOUSHA: &str = "Anousha, The Maniac";
const KALYANI: &str = "Kalyani, Agent of Doom";
const SERHAT: &str = "Serhat Gunde, Combat Junkie";
const LAWBRINGER: &str = "Lawbringer";
const PRZEMYSLAW: &str = "Przemysław, War Beast";
const BERENGUELA: &str = "Berenguela, The Vicious";

/// Seat 0: `name` (4 blood) and a spare; seat 1 a blocker (4 blood).
fn table(name: &str) -> (Engine, MinionId, MinionId) {
    let mut e = game(3);
    // Blood above capacity would drain at once.
    let c = card(name);
    let m = e.add_minion(SeatId(0), c, 4.min(CRYPT[c as usize].capacity), false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    (e, m, z)
}

fn bleed_blocked(e: &mut Engine, a: MinionId, b: MinionId) {
    pick(e, bleed(a));
    pick(e, block(b));
}

fn locked(e: &Engine, m: MinionId) -> bool {
    e.state.minions[m.index()].locked
}

// ---- The deck --------------------------------------------------------------------

// The Path of Caine precon: 12 crypt and 77 library cards, every card
// defined but the vampires printing no text; the third Sabbat deck.
#[test]
fn the_path_of_caine_deck_is_complete() {
    let deck = Deck::sabbat(2);
    assert_eq!(deck.name, "Path of Caine (Sabbat V5)");
    assert_eq!(deck.crypt_cards().len(), 12);
    assert_eq!(deck.library_cards().len(), 77);
    assert_eq!(deck.path(), Some("Caine"));
    for c in deck.library_cards() {
        assert!(cards::lookup(c).is_some(), "{c}");
    }
}

// ---- Vampires --------------------------------------------------------------------

// Algirdas: "Strike cards cost opposing younger vampires +1 blood." [LSJ
// 20090529] "If an opponent then plays an effect that grants a strike …,
// they pay the additional cost." [LSJ 20090703] "Strikes are paid at
// declaration": a younger [cel] vampire's Side Strike costs it 1 blood;
// an older one's nothing.
#[test]
fn algirdas_makes_younger_vampires_pay_for_strike_cards() {
    for (cap, toll) in [(2..=5, 1), (8..=11, 0)] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), with("cel", cap), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let algirdas = e.add_minion(SeatId(1), card(ALGIRDAS), 4, false);
        hand(&mut e, 0, &["Side Strike"]);
        bleed_blocked(&mut e, a, algirdas);
        pick_when(&mut e, play(0, 0, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            blood(&e, a),
            4 - toll,
            "capacity {:?}",
            e.state.capacity_of(a)
        );
    }
}

// Anousha: "+1 strength"; Kalyani: "-1 stealth when hunting. +1 strength."
#[test]
fn anousha_and_kalyani_are_strong_kalyani_a_poor_hunter() {
    let mut e = game(3);
    let anousha = e.add_minion(SeatId(0), card(ANOUSHA), 4, false);
    let kalyani = e.add_minion(SeatId(0), card(KALYANI), 4, false);
    let d = PrintedCtx::default();
    assert_eq!(e.state.printed_mod(anousha, Stat::Strength, d), 1);
    assert_eq!(e.state.printed_mod(kalyani, Stat::Strength, d), 1);
    let on = |kind| PrintedCtx {
        kind: Some(kind),
        ..Default::default()
    };
    assert_eq!(
        e.state
            .printed_mod(kalyani, Stat::Stealth, on(ActionKind::Hunt)),
        -1
    );
    assert_eq!(
        e.state
            .printed_mod(kalyani, Stat::Stealth, on(ActionKind::Bleed)),
        0
    );
}

// Crossbreaker: "Once each round of combat, Crossbreaker can burn 1 blood
// to make the damage from his hand strikes aggravated that round": the
// blocker goes to torpor from an aggravated hand strike it cannot mend.
#[test]
fn crossbreaker_buys_aggravated_hand_strikes() {
    let (mut e, crossbreaker, z) = table("Crossbreaker");
    bleed_blocked(&mut e, crossbreaker, z);
    pick_when(&mut e, vampire_ability(crossbreaker, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, crossbreaker),
        4 - 1 - 1,
        "the blood, the blocker's strike"
    );
    assert!(e.state.minions[z.index()].in_torpor, "aggravated");
}

// Serhat Gunde: "1 optional press each combat."
#[test]
fn serhat_gunde_has_a_press_each_combat() {
    let (mut e, serhat, z) = table(SERHAT);
    bleed_blocked(&mut e, serhat, z);
    offering(&mut e, Choice::UsePress);
}

// ---- Library cards -----------------------------------------------------------------

// Abbot: +1 stealth action, the card on the Sabbat vampire, unlocked; +1
// intercept during actions directed at its controller. [LSJ 20061222]
// "The +1 intercept applies to actions directed at cards controlled by the
// controller. It does not apply to actions directed at other Methuselahs."
#[test]
fn abbot_guards_its_controller() {
    let (mut e, lawbringer, _) = table(LAWBRINGER);
    hand(&mut e, 0, &["Abbot"]);
    pick(&mut e, play(0, 0, lawbringer));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!locked(&e, lawbringer), "unlocked");
    let abbot = find_library("Abbot");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == abbot && c.host == Some(lawbringer)));

    // Directed at its controller: +1; at another Methuselah: none.
    let mut e = game(3);
    let bleeder = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 3), 4, false);
    let guard = e.add_minion(SeatId(1), card(LAWBRINGER), 3, false);
    in_play(&mut e, 1, "Abbot", Some(guard));
    pick(&mut e, bleed(bleeder));
    request(&mut e);
    let a = e.action().expect("action");
    assert_eq!(a.intercept_of(&e.state, guard), 1);
    let mut e = game(4);
    let bleeder = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 3), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let guard = e.add_minion(SeatId(3), card(LAWBRINGER), 3, false);
    in_play(&mut e, 3, "Abbot", Some(guard));
    pick(&mut e, bleed(bleeder));
    request(&mut e);
    let a = e.action().expect("action");
    assert_eq!(a.intercept_of(&e.state, guard), 0, "a bleed at another");
}

// Abuse of Power: enter combat with a minion; at its end, only one
// combatant ready, the opposing minion's controller burns 1 pool.
#[test]
fn abuse_of_power_costs_the_fallen_a_pool() {
    let (mut e, lawbringer, z) = table(LAWBRINGER);
    e.state.minions[z.index()].blood = 0;
    hand(&mut e, 0, &["Abuse of Power"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(lawbringer),
            target: Target::Minion(z),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor);
    assert_eq!(e.state.seats[1].pool, 9);
}

// Abuse of Power: the acting vampire the one not ready, its own
// controller burns the pool — the controller of the ready minion's
// opponent (owner, 2026-10-08: rules-qa 54.7).
#[test]
fn abuse_of_power_costs_the_acting_vampire_s_controller_when_it_falls() {
    let (mut e, lawbringer, z) = table(LAWBRINGER);
    e.state.minions[lawbringer.index()].blood = 1;
    // Depravity: +1 strength, a 2-damage hand strike.
    in_play(&mut e, 1, "Depravity", Some(z));
    hand(&mut e, 0, &["Abuse of Power"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(lawbringer),
            target: Target::Minion(z),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[lawbringer.index()].in_torpor);
    assert!(e.state.minions[z.index()].ready());
    assert_eq!(e.state.seats[0].pool, 9);
    assert_eq!(e.state.seats[1].pool, 10);
}

// Augury of Doom: in its controller's prey's unlock phase, 1 pool for each
// vampire of theirs in torpor; none, the card burns (rules-qa 54.2).
#[test]
fn augury_of_doom_drains_a_prey_with_vampires_in_torpor() {
    let (mut e, _, _) = table(LAWBRINGER);
    e.add_minion(SeatId(1), plain("Brujah", 2), 2, true);
    e.add_minion(SeatId(1), plain("Brujah", 3), 2, true);
    e.add_minion(SeatId(2), plain("Brujah", 1), 4, false);
    let augury = in_play(&mut e, 0, "Augury of Doom", None);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 8);
    assert!(e.state.cards[augury.index()].in_play);

    let (mut e, _, _) = table(LAWBRINGER);
    e.add_minion(SeatId(2), plain("Brujah", 2), 4, false);
    let augury = in_play(&mut e, 0, "Augury of Doom", None);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !e.state.cards[augury.index()].in_play,
        "no vampire in torpor"
    );
    assert_eq!(e.state.seats[1].pool, 10);
}

// Depravity: +1 strength; +1 stealth during diablerie actions; its vampire
// cannot recruit allies or employ retainers.
#[test]
fn depravity_strengthens_and_bars_hiring() {
    let (mut e, lawbringer, _) = table(LAWBRINGER);
    in_play(&mut e, 0, "Depravity", Some(lawbringer));
    assert_eq!(
        e.state
            .bearer_mod(lawbringer, Stat::Strength, ActionKind::Bleed),
        1
    );
    assert_eq!(
        e.state
            .bearer_mod(lawbringer, Stat::Stealth, ActionKind::Diablerie),
        1
    );
    assert_eq!(
        e.state
            .bearer_mod(lawbringer, Stat::Stealth, ActionKind::Bleed),
        0
    );
    hand(&mut e, 0, &["Twisted Bloodhound"]);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play(0, 0, lawbringer),
    );
}

// [ANK 20180612] [ANK 20211127] Dread Mastiff: "The damage is
// environmental, the retainer is the damage source." [RTR 19941109] [TOM
// 19951216] [PIB 20140324] [ANK 20200422] "Damage is inflicted even
// against a dodge": the blocker's Side Strike dodges the hand strike, not
// the retainer's damage.
#[test]
fn dread_mastiff_bites_at_close_range_even_through_a_dodge() {
    let (mut e, lawbringer, _) = table(LAWBRINGER);
    let z = e.add_minion(SeatId(1), with("cel", 4..=11), 4, false);
    in_play(&mut e, 0, "Dread Mastiff", Some(lawbringer));
    hand(&mut e, 1, &["Side Strike"]);
    bleed_blocked(&mut e, lawbringer, z);
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 3, "the Mastiff's 1");
}

// Ensnare a Beast [ani] or [cel]: +1 intercept; [ANI] or [CEL]: blocking,
// the acting minion's strike cards cost it +1 blood. [LSJ 20090529] [LSJ
// 20090703] Paid as declared, a strike granted later too.
#[test]
fn ensnare_a_beast_makes_the_acting_minion_pay_for_strikes() {
    // Verrix: +1 stealth on his bleeds, intercept needed.
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), card("Verrix, Naughty Boy"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let hunter = e.add_minion(SeatId(1), card(PRZEMYSLAW), 4, false);
    hand(&mut e, 0, &["Side Strike"]);
    hand(&mut e, 1, &["Ensnare a Beast"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(hunter));
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 1,
            actor: Some(hunter),
            target: Target::None,
        },
    );
    pick_when(&mut e, play(0, 0, a)); // Side Strike
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 3, "Side Strike's toll");
}

// Fiendish Tongue: +1 bleed; Anarchs get -1 intercept; a Tzimisce's
// successful bleed lets it burn 1 blood to unlock in the discard phase.
#[test]
fn fiendish_tongue_bleeds_and_unlocks_a_tzimisce_later() {
    let (mut e, lawbringer, _) = table(LAWBRINGER);
    hand(&mut e, 0, &["Fiendish Tongue", "Abbot"]);
    pick(&mut e, play(0, 0, lawbringer));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 8);
    assert!(locked(&e, lawbringer));
    let unlock = Choice::UnlockForBlood { minion: lawbringer };
    pick_when(&mut e, unlock);
    assert!(!locked(&e, lawbringer));
    assert_eq!(blood(&e, lawbringer), 2, "its 3, 1 for the unlock");

    // An Anarch blocker: -1 intercept.
    let mut e = game(3);
    let lawbringer = e.add_minion(SeatId(0), card(LAWBRINGER), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let anarch = vampire(|c| c.sect == Sect::Anarch && c.capacity >= 4 && c.title == Title::None);
    let z = e.add_minion(SeatId(1), anarch, 4, false);
    hand(&mut e, 0, &["Fiendish Tongue"]);
    pick(&mut e, play(0, 0, lawbringer));
    pick_when(&mut e, block(z));
    let a = e.action().expect("action");
    assert_eq!(a.intercept_of(&e.state, z), -1);

    // "This Tzimisce": the clan read again in the discard phase, only a
    // Tzimisce unlocking there (owner, 2026-10-08: rules-qa 54.5). No pool
    // card changes a clan: a Brujah armed by hand.
    let (mut e, _, _) = table(LAWBRINGER);
    let brujah = MinionId(1);
    e.state.minions[brujah.index()].locked = true;
    e.state.minions[brujah.index()].discard_unlock_for_blood = 1;
    hand(&mut e, 0, &["Abbot"]);
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        Choice::UnlockForBlood { minion: brujah },
    );
}

// Hunting the Quarry [cel] or [tha], combat about to end: on the opposing
// minion, its player keeping control; burned by their vampire to enter
// combat with it as a +1 stealth Ⓓ action. [ANK 20191219] "Effects that
// are played when a combat 'would end' (eg. Telepathic Tracking) cannot be
// played afterwards." [CEL] or [THA]: 1 blood for a new round instead.
// [RTR 20030519] "Must be played after presses": in the would-end window.
#[test]
fn hunting_the_quarry_marks_the_opposing_minion_or_extends_the_combat() {
    let (mut e, _, z) = table(LAWBRINGER);
    let hekla = e.add_minion(
        SeatId(0),
        card("Hekla \"Bonecrusher\" Jónasdóttir"),
        5,
        false,
    );
    hand(&mut e, 0, &["Hunting the Quarry", "Telepathic Tracking"]);
    bleed_blocked(&mut e, hekla, z);
    let req = offering(&mut e, play(0, 0, hekla));
    assert_eq!(
        req.window,
        WindowLabel::CombatEndOfRound,
        "after the presses"
    );
    pick(&mut e, play(0, 0, hekla));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play(0, 1, hekla),
    );
    let quarry = find_library("Hunting the Quarry");
    let on: Vec<CardId> = (0..e.state.cards.len() as u32)
        .map(CardId)
        .filter(|&c| {
            let x = &e.state.cards[c.index()];
            x.in_play && x.card == quarry && x.host == Some(z)
        })
        .collect();
    assert_eq!(on.len(), 1, "on the blocker");
    assert_eq!(e.state.card_controller(on[0]), SeatId(0));

    // Next turn of seat 0: the granted action.
    let (mut e, _, z) = table(LAWBRINGER);
    let hekla = e.add_minion(
        SeatId(0),
        card("Hekla \"Bonecrusher\" Jónasdóttir"),
        5,
        false,
    );
    let mark = in_play(&mut e, 0, "Hunting the Quarry", Some(z));
    let rush = use_at(mark, 0, Some(hekla), Target::Minion(z));
    pick(&mut e, rush);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[mark.index()].in_play, "burned to attempt");
    assert_eq!(blood(&e, z), 3, "the combat");

    // [CEL] or [THA]: a new round for 1 blood.
    let (mut e, _, z) = table(LAWBRINGER);
    let hekla = e.add_minion(
        SeatId(0),
        card("Hekla \"Bonecrusher\" Jónasdóttir"),
        5,
        false,
    );
    hand(&mut e, 0, &["Hunting the Quarry"]);
    bleed_blocked(&mut e, hekla, z);
    pick_when(&mut e, play(0, 1, hekla));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 2, "two rounds of hand strikes");
    assert_eq!(blood(&e, hekla), 5 - 1 - 2);
}

// Pillars Fall: enter combat with a vampire; at its end, the opposing
// vampire not ready, the card on the acting vampire (rules-qa 54.3): +1
// bleed.
#[test]
fn pillars_fall_stays_on_a_victor() {
    let (mut e, lawbringer, z) = table(LAWBRINGER);
    e.state.minions[z.index()].blood = 0;
    hand(&mut e, 0, &["Pillars Fall"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(lawbringer),
            target: Target::Minion(z),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let pillars = find_library("Pillars Fall");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == pillars && c.host == Some(lawbringer)));
    assert!(!e.state.seats[0].ash.contains(&pillars));
    assert_eq!(
        e.state
            .bearer_mod(lawbringer, Stat::Bleed, ActionKind::Bleed),
        1
    );
}

// Rage of Apedemak [ani] or [pot], before range: this combat, +1 hand
// size and +1 damage on the vampire's hand strikes ([ANI] or [POT]: +2).
#[test]
fn rage_of_apedemak_strengthens_hand_strikes() {
    let (mut e, loyiso, z) = table("Loyiso, The Ruthless");
    hand(&mut e, 0, &["Rage of Apedemak"]);
    hand(&mut e, 1, &["Immortal Grapple"]);
    bleed_blocked(&mut e, loyiso, z);
    pick_when(&mut e, play(0, 1, loyiso));
    offering(&mut e, play(0, 0, z));
    assert_eq!(e.state.hand_size(SeatId(0)), 8, "this combat");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 3, "1 + 2");
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
}

// Sentry Signal [ani] or [tha]: a locked vampire unlocks and attempts to
// block; [ANI] or [THA]: a bleed against you reduced by 3.
#[test]
fn sentry_signal_unlocks_a_blocker_or_reduces_a_bleed() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 3), 4, false);
    let sentry = e.add_minion(SeatId(1), card(LAWBRINGER), 3, false);
    e.state.minions[sentry.index()].locked = true;
    hand(&mut e, 1, &["Sentry Signal"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, sentry));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10, "blocked");

    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 3), 4, false);
    let sentry = e.add_minion(SeatId(1), card(PRZEMYSLAW), 3, false);
    e.state.minions[a.index()].blood = 4;
    hand(&mut e, 1, &["Sentry Signal"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, sentry));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10, "1 reduced by 3");
}

// Soulgrinder [pot] or [tha]: a hand strike at +2 damage. [LSJ 20050628]
// [POT] or [THA]: "The opponent cannot play prevention cards of that
// discipline (eg. for cycling)": no Indomitability.
#[test]
fn soulgrinder_hits_hard_past_fortitude() {
    let (mut e, berenguela, _) = table(BERENGUELA);
    let z = e.add_minion(SeatId(1), with("for", 5..=11), 5, false);
    hand(&mut e, 0, &["Soulgrinder"]);
    hand(&mut e, 1, &["Indomitability"]);
    bleed_blocked(&mut e, berenguela, z);
    pick_when(&mut e, play(0, 1, berenguela));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, z));
    assert_eq!(blood(&e, z), 5 - 3);
}

// Twisted Bloodhound: "The employer can enter combat with a minion as a Ⓓ
// action."
#[test]
fn twisted_bloodhound_lets_its_employer_rush() {
    let (mut e, lawbringer, z) = table(LAWBRINGER);
    let hound = in_play(&mut e, 0, "Twisted Bloodhound", Some(lawbringer));
    pick(
        &mut e,
        use_at(hound, 0, Some(lawbringer), Target::Minion(z)),
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 3, "a hand strike");
}

// Wall of Filth [pro] or [tha], before range: on the vampire, burned to
// prevent 2 non-aggravated damage in combat; [PRO] or [THA] any damage.
// [RTR 20041202] "If fewer points of (preventable) damage are being
// resolved, then the effect prevents all of those points." [ANK 20200318]
// [LSJ 20001114] "Unused prevention points can't be carried over": the
// card is burned for the 1 point.
#[test]
fn wall_of_filth_prevents_two_damage_once() {
    let (mut e, lawbringer, z) = table(LAWBRINGER);
    hand(&mut e, 0, &["Wall of Filth"]);
    bleed_blocked(&mut e, lawbringer, z);
    pick_when(&mut e, play(0, 0, lawbringer));
    request(&mut e);
    let wall = find_library("Wall of Filth");
    let id = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.in_play && c.card == wall && c.host == Some(lawbringer))
            .expect("on Lawbringer") as u32,
    );
    assert!(!e.state.cards[id.index()].effect_only, "the card itself");
    assert!(!e.state.seats[0].ash.contains(&wall), "not in the ash heap");
    pick_when(&mut e, use_at(id, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, lawbringer), 3, "the hand strike prevented");
    assert!(!e.state.cards[id.index()].in_play, "burned");
}

/// A Yawp Court table: seat 0's political actor (an Anarch, 5+ capacity);
/// seat 1 Yawp Court with Lawbringer (Sabbat) to fight it.
fn yawp_table(actor_blood: u8) -> (Engine, MinionId, MinionId, CardId) {
    let mut e = game(3);
    let anarch = vampire(|c| c.sect == Sect::Anarch && c.capacity >= 5 && c.title == Title::None);
    let actor = e.add_minion(SeatId(0), anarch, actor_blood, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let sabbat = e.add_minion(SeatId(1), card(PRZEMYSLAW), 4, false);
    let court = in_play(&mut e, 1, "Yawp Court", None);
    (e, actor, sabbat, court)
}

// Yawp Court: before the referendum of a successful political action, its
// controller locks it and a ready unlocked Sabbat vampire to enter combat
// with the acting vampire. [LSJ 20080201] "The action cost, if any, is paid
// before Yawp Court gets locked": Reckless Agitation's 2 blood first. [RTR
// 19970630] "Damage inflicted at the end of combat is environmental": the
// acting vampire still ready, the Sabbat vampire takes 2 damage and the
// referendum is held. [LSJ 20060902] "If the acting minion is not ready
// after the combat, then the action ends and no referendum is conducted."
#[test]
fn yawp_court_fights_a_political_actor_before_the_referendum() {
    let (mut e, actor, sabbat, court) = yawp_table(5);
    hand(&mut e, 0, &["Reckless Agitation"]);
    pick(&mut e, play(0, 0, actor));
    let fight = use_at(court, 0, Some(sabbat), Target::None);
    pick_when(&mut e, fight);
    assert_eq!(blood(&e, actor), 5 - 2, "the cost first");
    to_referendum(&mut e);
    assert!(e.state.cards[court.index()].locked);
    assert_eq!(blood(&e, actor), 5 - 2 - 1, "the hand strike");
    assert_eq!(
        blood(&e, sabbat),
        4 - 1 - 2,
        "a hand strike, 2 environmental"
    );

    // The acting vampire to torpor in the combat: no referendum.
    let (mut e, actor, sabbat, court) = yawp_table(2);
    hand(&mut e, 0, &["Reckless Agitation"]);
    pick(&mut e, play(0, 0, actor));
    pick_when(&mut e, use_at(court, 0, Some(sabbat), Target::None));
    loop {
        let req = request(&mut e);
        assert!(e.referendum().is_none(), "no referendum");
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(e.state.minions[actor.index()].in_torpor);
    assert_eq!(blood(&e, sabbat), 4 - 1, "no environmental damage");
}

// [ANK 20180910-1] [LSJ 20060903] Yawp Court: "The effect is applied after
// cards that are played … 'when combat would end'": Hekla's Hunting the
// Quarry [THA] starts a second round; the 2 damage comes after it.
#[test]
fn yawp_court_damage_comes_after_the_combat_continues() {
    let (mut e, actor, _, court) = yawp_table(5);
    let hekla = e.add_minion(
        SeatId(1),
        card("Hekla \"Bonecrusher\" Jónasdóttir"),
        5,
        false,
    );
    hand(&mut e, 0, &["Political Stranglehold"]);
    hand(&mut e, 1, &["Hunting the Quarry"]);
    pick(&mut e, play(0, 0, actor));
    pick_when(&mut e, use_at(court, 0, Some(hekla), Target::None));
    pick_when(&mut e, play(0, 1, hekla));
    to_referendum(&mut e);
    assert_eq!(blood(&e, actor), 5 - 2, "two rounds");
    assert_eq!(
        blood(&e, hekla),
        5 - 1 - 2 - 2,
        "the card, two strikes, 2 environmental"
    );
}

// [LSJ 20090325-1] Yawp Court: "If the acting vampire gets out of torpor
// immediately (eg. Warsaw Station), the action ends and no referendum is
// conducted": a Nosferatu sent to torpor in the combat, moved back to the
// ready region by Warsaw Station, still ends the action.
#[test]
fn yawp_court_ends_the_action_of_a_vampire_back_from_torpor() {
    let mut e = game(3);
    let nosferatu =
        vampire(|c| c.clan == "Nosferatu" && c.sect != Sect::Sabbat && c.title == Title::None);
    let actor = e.add_minion(SeatId(0), nosferatu, 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let sabbat = e.add_minion(SeatId(1), card(PRZEMYSLAW), 4, false);
    let court = in_play(&mut e, 1, "Yawp Court", None);
    let warsaw = in_play(&mut e, 0, "Warsaw Station", None);
    hand(&mut e, 0, &["Political Stranglehold"]);
    hand(&mut e, 1, &["Soulgrinder"]);
    pick(&mut e, play(0, 0, actor));
    pick_when(&mut e, use_at(court, 0, Some(sabbat), Target::None));
    // Soulgrinder's 3: torpor.
    pick_when(&mut e, play(0, 0, sabbat));
    let back = use_at(warsaw, 1, None, Target::Minion(actor));
    let mut used = false;
    loop {
        let req = request(&mut e);
        assert!(e.referendum().is_none(), "no referendum");
        if req.options.contains(&back) && !used {
            used = true;
            pick(&mut e, back);
            continue;
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(used, "Warsaw Station offered");
    assert!(!e.state.minions[actor.index()].in_torpor);
}
