//! Legacy cards for TWDA decks, rounds 4 and 5 (`ADDED_BATCHES`, the
//! fourteenth and fifteenth batches, slices 16p–16q): the Gangrel decks'
//! missing cards — Anachronism, Bone Spur, The Kiss of Ra, Mind of the
//! Wilds, Ossian, The Path of Lilith, Predator's Transformation, Read the
//! Winds and Zip Gun. Readings in rules-qa Batch 64. Windows with a single
//! option are elided, so the scripts only stop where a real decision exists.

use zvtes::cards::{self, AbilitySource, Effect, Layer, Scope, Target};
use zvtes::crypt::{Sect, CRYPT};
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

fn card_in_play(e: &Engine, name: &str) -> CardId {
    let card = find_library(name);
    CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.in_play && c.card == card)
            .expect("in play") as u32,
    )
}

fn play_at(card: u8, mode: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target,
    }
}

/// An untitled vampire with no definition of its own, of capacity `cap`,
/// of `sect` if given.
fn of_capacity(cap: u8, sect: Option<Sect>, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.capacity == cap
                && sect.is_none_or(|s| c.sect == s)
                && c.title == zvtes::crypt::Title::None
                && cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a vampire of the capacity")
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

fn holds(e: &Engine, m: MinionId, name: &str) -> bool {
    let card = find_library(name);
    e.state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == card && c.host == Some(m))
}

// ---- Anachronism ------------------------------------------------------------------

// Anachronism: "Put this card on a vampire with capacity above 6. This
// vampire cannot have or use any ranged weapons except the Ivory Bow. (If
// he or she currently has any, they are burned.)" [LSJ 20060825] "All
// weapons that provide ranged strikes are considered 'ranged weapons'": the
// .44 Magnum burns, the Ivory Bow and a melee Femur of Toomler stay; a
// capacity-6 vampire is no target; a new .44 Magnum cannot be equipped.
#[test]
fn anachronism_burns_and_bars_ranged_weapons() {
    let mut e = game(2);
    let big = e.add_minion(SeatId(0), find_card("Vivian VI"), 4, false);
    let six = e.add_minion(SeatId(0), of_capacity(6, None, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for w in [".44 Magnum", "Ivory Bow", "Femur of Toomler"] {
        in_play(&mut e, 0, w, Some(big));
    }
    hand(&mut e, 0, &["Anachronism", ".44 Magnum"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(six),
    }));
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(big),
        },
    );
    request(&mut e);
    assert!(holds(&e, big, "Anachronism"));
    assert!(!holds(&e, big, ".44 Magnum"));
    assert!(holds(&e, big, "Ivory Bow") && holds(&e, big, "Femur of Toomler"));
    let req = to_minion_phase(&mut e, SeatId(0));
    assert!(!req.options.contains(&play(0, 0, big)), "no ranged weapon");
    assert!(req.options.contains(&play(0, 0, six)));
}

// ---- Bone Spur --------------------------------------------------------------------

// Bone Spur: "[pro] For the remainder of this round, this vampire's hand
// damage is aggravated." [RTR 19960112] "Must be played before strike
// resolution and damage prevention in order to affect the current strike.
// (can be played after both strikes have been chosen)": both hand strikes
// chosen, then Bone Spur — the blocker takes 1 aggravated damage and goes
// to torpor.
#[test]
fn bone_spur_aggravates_a_chosen_hand_strike() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pro", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Bone Spur"]);
    blocked_bleed(&mut e, a, z);
    // Both hand strikes (each the only strike) chosen at once; the card
    // before they resolve.
    loop {
        let req = request(&mut e);
        if req.window == WindowLabel::CombatBeforeResolution && req.options.contains(&play(0, 0, a))
        {
            e.choose(index_of(&req, play(0, 0, a)));
            break;
        }
        e.choose(fallback(&req));
    }
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor);
}

// Bone Spur [PRO]: "As above, but for the remainder of this combat." The
// first round's hand strike dodged (Staredown), a press into the second:
// its hand strike still aggravated — the blocker to torpor; [pro] only for
// its round, the second strike a plain one.
#[test]
fn bone_spur_superior_lasts_the_combat() {
    for superior_spur in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
        superior(&mut e, a, "pro");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with("pre", 0), 4, false);
        hand(&mut e, 0, &["Bone Spur", "Infernal Pursuit"]);
        hand(&mut e, 1, &["Staredown"]);
        blocked_bleed(&mut e, a, z);
        pick_when(&mut e, play(0, u8::from(superior_spur), a));
        pick_when(&mut e, play(0, 0, z));
        pick_when(&mut e, play(0, 0, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.minions[z.index()].in_torpor,
            superior_spur,
            "superior {superior_spur}"
        );
    }
}

// ---- The Kiss of Ra ------------------------------------------------------------------

// The Kiss of Ra: "Only usable when a vampire who does not have Fortitude
// attempts to block this acting minion. [for] The block attempt is
// canceled, the blocking vampire burns 2 blood, and the current action is
// ended (without combat). [FOR] As above, and send the blocking vampire to
// torpor." [RTR 20180719] "No reaction or modifier can be played
// afterwards: the action ends immediately." [ANK 20211015] [ANK 20210124]
// [LSJ 20090220] [LSJ 20070709] "The action has not reached resolution, it
// is effectively canceled. The same vampire can undertake the same action
// again": no bleed lands, the vampire (unlocked) may bleed again.
#[test]
fn the_kiss_of_ra_ends_the_action_on_a_block_attempt() {
    for superior_kiss in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("FOR", 0), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
        hand(&mut e, 0, &["Kiss of Ra, The"]);
        hand(&mut e, 1, &["Eagle's Sight"]);
        blocked_bleed(&mut e, a, z);
        pick(&mut e, play(0, u8::from(superior_kiss), a));
        let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e)[1], 10, "no bleed");
        assert_eq!(blood(&e, z), 4 - 2);
        assert_eq!(e.state.minions[z.index()].in_torpor, superior_kiss);
        assert_eq!(e.last_outcome, None, "no resolution");
        assert!(!req.options.contains(&bleed(a)), "locked");
        e.state.minions[a.index()].locked = false;
        let req = request(&mut e);
        assert!(req.options.contains(&bleed(a)), "the same action again");
    }
}

// [LSJ 20100210] Unleash Hell's Fury: "If The Kiss of Ra or Elder
// Impersonation are used, the block does not happen": the Fury (a vampire
// without Fortitude) attempts to block the bleed; the Kiss ends the action
// — no aggravated damage, no bleed.
#[test]
fn the_kiss_of_ra_stops_unleash_hells_fury() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nergal"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("FOR", 0), 5, false);
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
    hand(&mut e, 1, &["Kiss of Ra, The"]);
    pick(&mut e, bleed(z));
    pick_when(&mut e, block(token));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[z.index()].in_torpor,
        "the block did not happen"
    );
    assert_eq!(pool(&e)[0], 10 - 2, "no bleed (the Fury's 2 pool)");
}

// [ANK 20211015] [ANK 20210124] [LSJ 20090220] [LSJ 20070709] The Kiss of
// Ra: "The same vampire can undertake the same action again, but not if it
// was blocked then continued before (eg. [PRO] Form of Mist)." An Abactor
// blocked by seat 1, continued as if unblocked (Torrent [CEL]), then
// Kissed against seat 2's attempt: recorded as taken; Kissed with no block
// before, not.
#[test]
fn the_kiss_of_ra_after_a_continued_block_leaves_the_action_taken() {
    for continued in [true, false] {
        let mut e = game(3);
        // Torrent's 2 blood and the Kiss's 3.
        let a = e.add_minion(SeatId(0), of_capacity(6, None, 0), 6, false);
        superior(&mut e, a, "cel");
        superior(&mut e, a, "for");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let y = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
        let z = e.add_minion(SeatId(2), plain("Brujah", 2), 4, false);
        superior(&mut e, z, "aus");
        hand(&mut e, 0, &["Abactor", "Torrent", "Kiss of Ra, The"]);
        hand(&mut e, 1, &["Eagle's Sight"]);
        hand(&mut e, 2, &["Eagle's Sight"]);
        pick(&mut e, play(0, 0, a));
        if continued {
            pick_when(&mut e, block(y));
            pick_when(&mut e, play(0, 0, y));
            pick_when(&mut e, play(0, 1, a));
        }
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, 0, z));
        // The hand shifts down as Abactor (and Torrent) are played.
        let kiss = play(if continued { 0 } else { 1 }, 0, a);
        pick_when(&mut e, kiss);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let abactor = zvtes::state::ActionId::Card(find_library("Abactor"));
        assert_eq!(
            e.state.minions[a.index()].acted.contains(&abactor),
            continued,
            "continued {continued}"
        );
    }
}

// [ANK 20210124] The Kiss of Ra: "The action was still taken (eg. counts
// against … [OBF] Veil the Legions)": Veil the Legions [OBF] with X = 1;
// the next action, Kissed, takes its +1 stealth — none is left.
#[test]
fn the_kiss_of_ra_counts_against_veil_the_legions() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    let o = e.add_minion(SeatId(0), vampire_with("OBF", 0), 5, false);
    let a = e.add_minion(SeatId(0), vampire_with("FOR", 0), 5, false);
    let z = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    hand(&mut e, 0, &["Veil the Legions", "Kiss of Ra, The"]);
    hand(&mut e, 1, &["Eagle's Sight", "Eagle's Sight"]);
    pick(&mut e, bleed(b));
    pick(&mut e, block(z));
    pick_when(&mut e, play(0, 1, o));
    pick(&mut e, Choice::SetX(1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].veil_left, 1);
    e.state.minions[z.index()].locked = false;
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 0, a));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].veil_left, 0, "spent on the Kissed action");
}

// The Kiss of Ra against a vampire with Fortitude: not offered.
#[test]
fn the_kiss_of_ra_needs_a_blocker_without_fortitude() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("FOR", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("for", 0), 4, false);
    hand(&mut e, 0, &["Kiss of Ra, The"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

// [LSJ 19970325] The Kiss of Ra: "When used by an acting vampire in torpor,
// there is no opportunity to diablerize the acting vampire, as the action is
// ended without a successful block." Leaving torpor, blocked: the Kiss, and
// no torpor-block window; still in torpor.
#[test]
fn the_kiss_of_ra_leaves_no_diablerie_of_a_torpid_acting_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("FOR", 0), 4, true);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    hand(&mut e, 0, &["Kiss of Ra, The"]);
    hand(&mut e, 1, &["Eagle's Sight"]);
    pick(&mut e, action(a, ActionKind::LeaveTorpor));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    loop {
        let req = request(&mut e);
        assert_ne!(req.window, WindowLabel::TorporBlock);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(e.state.minions[a.index()].in_play);
    assert!(e.state.minions[a.index()].in_torpor);
}

// [ANK 20210124] The Kiss of Ra: "The action was still taken (eg. counts
// against Enkil Cog or [OBF] Veil the Legions)": the Methuselah's optional
// action is spent.
#[test]
fn the_kiss_of_ra_leaves_the_action_taken() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("FOR", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    hand(&mut e, 0, &["Kiss of Ra, The"]);
    hand(&mut e, 1, &["Eagle's Sight"]);
    blocked_bleed(&mut e, a, z);
    pick(&mut e, play(0, 0, a));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.optional_action);
}

// ---- Mind of the Wilds -------------------------------------------------------------

// Mind of the Wilds: "If this action is blocked, this vampire gets 1
// optional maneuver during the first round of the resulting combat and the
// blocking minion cannot strike: combat ends." [LSJ 20100308] "The blocking
// minion is prohibited to strike: combat ends for the duration of the
// action (during any round of any combat)": Staredown's [PRE] combat ends
// is not offered in the first round nor, pressed (Infernal Pursuit), in
// the second; the maneuver is offered in the first round only.
#[test]
fn mind_of_the_wilds_bars_combat_ends_for_the_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
    superior(&mut e, a, "ani");
    superior(&mut e, a, "aus");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("PRE", 0), 4, false);
    hand(&mut e, 0, &["Mind of the Wilds", "Infernal Pursuit"]);
    hand(&mut e, 1, &["Staredown", "Staredown"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, Choice::UseManeuver);
    assert_eq!(req.seat, SeatId(0));
    let mut round = 1;
    loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&play(0, 1, z)), "round {round}");
        assert!(!req.options.contains(&play(1, 1, z)), "round {round}");
        if req.options.contains(&play(0, 0, a)) && req.window == WindowLabel::CombatPress {
            e.choose(index_of(&req, play(0, 0, a)));
            round = 2;
            continue;
        }
        if round == 2 && req.seat == SeatId(0) {
            assert!(
                !req.options.contains(&Choice::UseManeuver),
                "first round only"
            );
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(round, 2, "pressed");
}

// ---- Ossian -------------------------------------------------------------------------

// Ossian: "In the first round of combat with a vampire who has played a
// card that requires Auspex during this action, that vampire cannot use
// any maneuvers or strikes. Ossian gains 1 life at the end of each round
// for each blood the opposing vampire used to heal damage or prevent
// destruction that round." [LSJ 20050201] "The Auspex special applies to
// whatever the current action is": Ossian blocks a bleed with Mind of the
// Wilds [ani][aus] played — its vampire neither maneuvers nor strikes;
// Ossian's 2 damage costs it 2 blood. [ANK 20171109] "His life total can
// exceed his starting life": 4 + 2. [RTR 20030519] "The life is gained
// after the presses are handled."
#[test]
fn ossian_freezes_an_auspex_vampire_and_feeds_on_its_healing() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    superior(&mut e, a, "ani");
    superior(&mut e, a, "aus");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let o = e.add_ally(SeatId(1), find_library("Ossian"));
    hand(&mut e, 0, &["Mind of the Wilds"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, block(o));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::UseManeuver,
    );
    assert_eq!(blood(&e, a), 4 - 2, "no strike back");
    assert_eq!(blood(&e, o), 4 + 2);
}

// [ANK 20210614] Ossian: "If no Auspex was used previously, an Auspex
// strike can be used (eg. Charismatic Aura)." A plain bleed blocked by
// Ossian: Psychic Assault [aus] strikes him for 1 (the card then played,
// it is no strike before it).
#[test]
fn ossian_lets_a_first_auspex_strike_land() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("aus", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let o = e.add_ally(SeatId(1), find_library("Ossian"));
    hand(&mut e, 0, &["Psychic Assault"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(o));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // 1R from the strike; then the 2 blood the vampire lost to his.
    assert_eq!(blood(&e, o), 4 - 1 + 2);
}

// [LSJ 20100827] Ossian: "If Auspex was used to block him, the blocker can
// use an effect to set the range, but not a Sniper Rifle as it would
// require him to strike with it." Ossian rushes a vampire, which blocks
// with Precognition [aus]: its Sniper Rifle's long range is not offered.
#[test]
fn ossian_bars_a_sniper_rifle_after_an_auspex_block() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let o = e.add_ally(SeatId(0), find_library("Ossian"));
    let id = card_in_play(&e, "Ossian");
    let z = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let rifle = in_play(&mut e, 1, "Sniper Rifle", Some(z));
    hand(&mut e, 1, &["Precognition"]);
    pick(&mut e, use_at(id, 0, Some(o), Target::Minion(z)));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_at(rifle, 1, None, Target::None),
    );
}

// ---- The Path of Lilith ---------------------------------------------------------------

// The Path of Lilith: "When a non-Camarilla vampire you control sends an
// opposing vampire to torpor in combat, you may lock this card to put a
// torture counter on the opposing vampire. During your unlock phase, each
// Methuselah who controls any vampires with torture counters burns 1 pool.
// Any minion may burn this card as a Ⓓ action. Burn all torture counters
// when this card leaves play." An Anarch's aggravated strike (Bone Spur)
// sends the blocker to torpor: the card locked, a counter on it; seat 1
// burns 1 pool in seat 0's unlock phase; the card burned, the counter
// gone. A Camarilla vampire's: no offer.
#[test]
fn the_path_of_lilith_tortures_and_drains() {
    for anarch in [true, false] {
        let mut e = game(2);
        let sect = if anarch {
            Sect::Anarch
        } else {
            Sect::Camarilla
        };
        let a = e.add_minion(SeatId(0), of_capacity(5, Some(sect), 0), 4, false);
        superior(&mut e, a, "pro");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let path = in_play(&mut e, 0, "Path of Lilith, The", None);
        hand(&mut e, 0, &["Bone Spur"]);
        blocked_bleed(&mut e, a, z);
        pick_when(&mut e, play(0, 0, a));
        let torture = Choice::ResolveRider { card: path.0 };
        if !anarch {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, torture);
            continue;
        }
        let req = offering(&mut e, torture);
        e.choose(index_of(&req, torture));
        assert_eq!(e.state.minions[z.index()].torture, 1);
        assert!(e.state.cards[path.index()].locked);
        to_minion_phase(&mut e, SeatId(0));
        pick(&mut e, Choice::Pass);
        to_minion_phase(&mut e, SeatId(0));
        assert_eq!(pool(&e)[1], 10 - 1, "seat 1's vampire tortured");
        assert_eq!(pool(&e)[0], 10);
        e.burn_card(path);
        request(&mut e);
        assert_eq!(e.state.minions[z.index()].torture, 0);
    }
}

// ---- Predator's Transformation ---------------------------------------------------

// Predator's Transformation: "[abo] [REFLEX] Cancel a frenzy card played
// on this vampire as it is played." [LSJ 20051113] "Any frenzy card that
// targets/selects/chooses/affects the minion when played, is considered
// being played on the minion": Terror Frenzy [ani], affecting the opposing
// minion, canceled by it; [ANK 20260216] "The cost for a canceled
// non-action-card is still paid": Terror Frenzy's blood.
#[test]
fn predators_transformation_cancels_a_frenzy_card_on_its_vampire() {
    let run = |frenzy: bool| {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("ani", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        superior(&mut e, z, "abo");
        if frenzy {
            hand(&mut e, 0, &["Terror Frenzy"]);
        }
        hand(&mut e, 1, &["Predator's Transformation"]);
        blocked_bleed(&mut e, a, z);
        if frenzy {
            pick_when(&mut e, play(0, 0, a));
            let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
            e.choose(index_of(&req, play(0, 0, z)));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let canceled = e.state.seats[1]
            .ash
            .contains(&find_library("Predator's Transformation"));
        (blood(&e, a), canceled)
    };
    let (plain_blood, _) = run(false);
    let (blood_after, canceled) = run(true);
    assert!(canceled);
    assert_eq!(blood_after, plain_blood - 1, "its cost still paid");
}

// [ANK 20190425] Predator's Transformation [abo]: "Cards and effects that
// are played 'as the action is announced' must be played before playing
// 'regular' action modifier or reaction cards." [ANK 20181208] "Replace the
// action card (if any) before playing it." +1 stealth as the bleed is
// announced, even if not needed: the hand already back to its size.
#[test]
fn predators_transformation_gives_stealth_as_the_action_is_announced() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    superior(&mut e, a, "abo");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Predator's Transformation"]);
    fill_hand(&mut e, 0);
    library(&mut e, 0, "Bonding", 3);
    pick(&mut e, bleed(a));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionAsAnnounced);
    e.choose(index_of(&req, play(0, 1, a)));
    request(&mut e);
    assert_eq!(e.state.seats[0].hand.len(), 7, "replaced as played");
}

// Predator's Transformation [ABO]: "Only usable when this vampire burns a
// minion controlled by your prey, either in combat or as a Ⓓ action. Your
// prey burns two pool." [ANK 20220507] "Any directed action that ends up
// burning an ally counts as a directed action burning an ally": a rush
// (Bum's Rush) burns the prey's ally in combat — seat 1 burns 2 pool.
#[test]
fn predators_transformation_superior_follows_a_burned_prey_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    superior(&mut e, a, "abo");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Carlton Van Wyk"));
    e.state.minions[ally.index()].blood = 1;
    hand(&mut e, 0, &["Bum's Rush", "Predator's Transformation"]);
    pick(&mut e, play_at(0, 0, a, Target::Minion(ally)));
    // The hand shifts down: Predator's Transformation is card 0.
    let req = offering(&mut e, play(0, 2, a));
    assert_eq!(req.window, WindowLabel::CombatEndOfRound);
    assert!(!e.state.minions[ally.index()].in_play);
    e.choose(index_of(&req, play(0, 2, a)));
    request(&mut e);
    assert_eq!(pool(&e)[1], 10 - 2);
}

// ---- Read the Winds ------------------------------------------------------------------

// Read the Winds: "Do not replace until after this action. [ani][aus] +1
// intercept. This vampire does not lock for the block." The block lands,
// its vampire still unlocked after it; the card replaced as the action
// ends.
#[test]
fn read_the_winds_blocks_without_locking() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    superior(&mut e, z, "ani");
    superior(&mut e, z, "aus");
    hand(&mut e, 0, &["Cloak the Gathering"]);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Read the Winds"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].locked, "no lock for the block");
    assert_eq!(pool(&e)[1], 10, "blocked");
}

// Read the Winds [ANI][AUS]: "Only usable by a locked vampire. This vampire
// unlocks and attempts to block with +1 intercept, even if intercept is not
// yet needed." A locked vampire blocks the bleed.
#[test]
fn read_the_winds_superior_unlocks_to_block() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    superior(&mut e, z, "ani");
    superior(&mut e, z, "aus");
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 1, &["Read the Winds"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10, "blocked");
    assert!(e.state.minions[z.index()].locked, "locked for the block");
}

// ---- Zip Gun -----------------------------------------------------------------------

// Zip Gun: "Before range is determined, put this card on this minion. …
// It does 1R damage each strike, with an optional maneuver each combat.
// Bearer takes 1 damage during strike resolution when striking with this
// gun, but only once each combat. This card is kept as normal equipment."
// [LSJ 19970801] "Damage done to the bearer is environmental": 1 from the
// first strike only; pressed into a second round (Infernal Pursuit), no
// more. The gun stays on its bearer.
#[test]
fn zip_gun_hurts_its_bearer_once_each_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("pre", 0), 6, false);
    hand(&mut e, 0, &["Zip Gun", "Infernal Pursuit"]);
    hand(&mut e, 1, &["Staredown", "Staredown"]);
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    let gun = card_in_play(&e, "Zip Gun");
    assert_eq!(e.state.cards[gun.index()].host, Some(a));
    // Round 1: the gun's strike; the blocker dodges.
    pick_when(&mut e, use_at(gun, 0, None, Target::None));
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 0, a));
    // Round 2: the gun again; the blocker dodges again.
    pick_when(&mut e, use_at(gun, 0, None, Target::None));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1, "once each combat");
    assert!(holds(&e, a, "Zip Gun"), "kept as equipment");
}

// [LSJ 20100310] Zip Gun: "No damage is done to the bearer if he does not
// strike with the gun (eg. if it is destroyed with first strike)": the
// bearer maneuvers to long range with it; the blocker's Canine Horde [ANI]
// burns it with first strike — its bearer takes nothing.
#[test]
fn zip_gun_destroyed_first_deals_its_bearer_nothing() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("ANI", 0), 4, false);
    let gun = in_play(&mut e, 0, "Zip Gun", Some(a));
    hand(&mut e, 1, &["Canine Horde"]);
    blocked_bleed(&mut e, a, z);
    // The gun's maneuver to long range (Canine Horde [ANI] is ranged),
    // committing its strike.
    pick_when(&mut e, use_at(gun, 1, None, Target::None));
    pick_when(&mut e, play_at(0, 1, z, Target::Card(gun)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[gun.index()].in_play);
    assert_eq!(blood(&e, a), 4, "no strike, no damage");
}

fn library(e: &mut Engine, seat: u8, name: &str, n: usize) {
    for _ in 0..n {
        e.state.seats[seat as usize]
            .library
            .push(find_library(name));
    }
}
