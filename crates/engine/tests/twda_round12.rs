//! Legacy cards for TWDA decks, round 12 (`ADDED_BATCHES`, the
//! twenty-second batch, slice 16x): the next cards bringing 2025-2026 TWDA
//! decks into the kept corpus. Readings in rules-qa Batch 74. Windows with
//! a single option are elided, so the scripts only stop where a real
//! decision exists.

use zvtes::cards::{AbilitySource, Stat, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{Choice, WindowLabel};
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

/// A card play by `actor` at `target`.
fn play_at(card: u8, mode: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target,
    }
}

/// The `nth` plain vampire (no printed text, untitled, capacity 4 or more)
/// with `disc` (any, if empty), of `sect` if given.
fn with_disc(disc: &str, sect: Option<Sect>, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            sect.is_none_or(|s| c.sect == s)
                && (disc.is_empty() || c.disciplines.contains(&disc))
                && c.title == Title::None
                && c.capacity >= 4
                && zvtes::cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no plain vampire with {disc}"))
}

fn stat(e: &Engine, m: MinionId, s: Stat) -> i32 {
    e.state.printed_mod(m, s, PrintedCtx::default())
}

/// The `nth` vampire with no printed text matching `f`.
fn textless(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| f(c) && zvtes::cards::vampire(*i as u16).is_none())
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a textless vampire")
}

/// A library of `n` Conceals for `seat`, to draw from.
fn library(e: &mut Engine, seat: usize, n: usize) {
    e.state.seats[seat].library = vec![find_library("Conceal"); n].into();
}

// ---- Equipment and combat cards ------------------------------------------------

// Sawed-Off Shotgun: "Weapon, gun. 3R damage each strike, only usable once
// each combat." [ANK 20230316] "A second copy allows a second use in the
// same combat": after the first copy's strike, Blur's additional strike
// offers the second copy only.
#[test]
fn sawed_off_shotgun_strikes_once_each_combat_per_copy() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), with_disc("cel", None, 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let first = in_play(&mut e, 1, "Sawed-Off Shotgun", Some(x));
    let second = in_play(&mut e, 1, "Sawed-Off Shotgun", Some(x));
    hand(&mut e, 1, &["Blur"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    let shot = |c: CardId| use_at(c, 0, None, Target::None);
    pick_when(&mut e, shot(first));
    pick_when(&mut e, play(0, 0, x));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(req.options.contains(&shot(second)));
    assert!(!req.options.contains(&shot(first)), "once each combat");
    pick(&mut e, shot(second));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].ready() || blood(&e, a) == 0);
}

// Flak Jacket: "The minion with this equipment may prevent 1 damage each
// combat." Against the first hand strike; the additional strike's damage
// lands.
#[test]
fn flak_jacket_prevents_one_damage_each_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), with_disc("cel", None, 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let jacket = in_play(&mut e, 0, "Flak Jacket", Some(a));
    hand(&mut e, 1, &["Blur"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    let prevent = use_at(jacket, 0, None, Target::None);
    pick_when(&mut e, prevent);
    pick_when(&mut e, play(0, 0, x));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, prevent);
    assert_eq!(blood(&e, a), 4 - 1, "the second strike lands");
}

// Lucky Blow: "Do not replace until after combat. Strike: hand strike or use
// a melee weapon strike. This strike is at +1 damage." A hand strike for
// 2; the card is replaced as the combat ends, not before.
#[test]
fn lucky_blow_strikes_for_one_more_and_is_replaced_after_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Lucky Blow"]);
    library(&mut e, 0, 10);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    let full = e.state.hand_size(SeatId(0));
    pick_when(&mut e, play(0, 0, a));
    assert_eq!(e.state.hand_load(SeatId(0)), full - 1, "not yet replaced");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2);
    assert_eq!(e.state.hand_load(SeatId(0)), full);
}

// Lucky Blow with a melee weapon: [RTR 19970630] [TOM 19960225] "Additional
// damage inherits all of the properties of the base damage." A Righteous
// Blade's 2 aggravated at +1.
#[test]
fn lucky_blow_adds_to_a_melee_weapon_strike() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let blade = in_play(&mut e, 0, "Righteous Blade", Some(a));
    hand(&mut e, 0, &["Lucky Blow"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play_at(0, 1, a, Target::Card(blade)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 6 - 3);
}

// ---- Actions --------------------------------------------------------------------

// Arson: "Do not replace until after the action. Ⓓ Burn a location."
// [LSJ 20090324] directed at the location's controller: across a 4-player
// table, only that Methuselah may block. [LSJ 20001120] replaced when the
// action ends.
#[test]
fn arson_burns_a_location_directed_at_its_controller() {
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(3), plain("Toreador", 0), 4, false);
    let site = in_play(&mut e, 2, "Information Highway", None);
    hand(&mut e, 0, &["Arson"]);
    library(&mut e, 0, 10);
    let full = e.state.hand_size(SeatId(0));
    pick(&mut e, play_at(0, 0, a, Target::Card(site)));
    assert_eq!(e.state.hand_load(SeatId(0)), full - 1, "not yet replaced");
    request(&mut e);
    let directed = e.action().expect("action").directed_seats(&e.state);
    assert!(directed.contains(SeatId(2)) && !directed.contains(SeatId(1)));
    let mut blockers = Vec::new();
    loop {
        let req = request(&mut e);
        if req.window != WindowLabel::ActionBlock {
            break;
        }
        blockers.push(req.seat);
        e.choose(fallback(&req));
    }
    assert_eq!(blockers, vec![SeatId(2)]);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[site.index()].in_play);
    assert_eq!(e.state.hand_load(SeatId(0)), full);
}

// Loss [qui]: "Ⓓ Burn an equipment or location on a minion controlled by
// your predator or prey." [LSJ 20090324] "directed against the Methuselah
// controlling the equipment or location": the prey's Flak Jacket, not the
// one across the table.
#[test]
fn loss_burns_an_equipment_of_the_predator_or_prey() {
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    superior(&mut e, a, "qui");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let q = e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(3), plain("Toreador", 0), 4, false);
    let near = in_play(&mut e, 1, "Flak Jacket", Some(p));
    let far = in_play(&mut e, 2, "Flak Jacket", Some(q));
    hand(&mut e, 0, &["Loss"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play_at(0, 0, a, Target::Card(far))));
    pick(&mut e, play_at(0, 0, a, Target::Card(near)));
    request(&mut e);
    let directed = e.action().expect("action").directed_seats(&e.state);
    assert!(directed.contains(SeatId(1)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[near.index()].in_play);
}

// Loss [QUI]: "Ⓓ Bleed with +2 bleed."
#[test]
fn loss_superior_bleeds_for_two_more() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    superior(&mut e, a, "qui");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 0, &["Loss"]);
    pick(&mut e, play_at(0, 1, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 3);
}

// ---- Action modifiers and reactions ------------------------------------------

/// A seat 0 vampire of capacity 10 with superior `disc`, and a
/// younger seat 1 vampire: (game, acting, younger).
fn older_and_younger(disc: &'static str) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    // Thucimia, capacity 10: her text (a bleed trade) is inert here.
    let old = find_card("Thucimia");
    let a = e.add_minion(SeatId(0), old, 4, false);
    superior(&mut e, a, disc);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let young = CRYPT
        .iter()
        .enumerate()
        .find(|(i, c)| {
            c.capacity <= 4 && c.title == Title::None && zvtes::cards::vampire(*i as u16).is_none()
        })
        .map(|(i, _)| i as u16)
        .expect("a young vampire");
    let z = e.add_minion(SeatId(1), young, 4, false);
    (e, a, z)
}

// Deed the Heart's Desire [qui]: "Only usable as the action is announced.
// Choose a younger vampire. The chosen vampire cannot block this action."
#[test]
fn deed_the_hearts_desire_bars_a_younger_vampire_from_blocking() {
    let (mut e, a, z) = older_and_younger("qui");
    hand(&mut e, 0, &["Deed the Heart's Desire"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play_at(0, 0, a, Target::Minion(z)));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(z));
    assert_eq!(pool(&e)[1], 10 - 1);
}

// Deed the Heart's Desire [QUI]: "As above, and the chosen vampire cannot
// play reaction cards during this action."
#[test]
fn deed_the_hearts_desire_superior_bars_reactions_too() {
    let (mut e, a, z) = older_and_younger("qui");
    superior(&mut e, z, "aus");
    hand(&mut e, 0, &["Deed the Heart's Desire"]);
    hand(&mut e, 1, &["Telepathic Misdirection"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play_at(0, 1, a, Target::Minion(z)));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, z));
    assert_eq!(pool(&e)[1], 10 - 1);
}

// Eluding the Arms of Morpheus: "Only usable by a locked vampire. This
// vampire unlocks and attempts to block." The block locks it again.
#[test]
fn eluding_the_arms_of_morpheus_unlocks_to_block() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 1, &["Eluding the Arms of Morpheus"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10, "blocked");
    assert_eq!(blood(&e, z), 4 - 1 - 1, "its 1 blood, then A's hand strike");
}

// Eluding the Arms of Morpheus: [ANK 20180623] "Does not ignore the normal
// prey, predator or target restrictions": a locked vampire across a
// 4-player table never gets it against a bleed of another.
#[test]
fn eluding_the_arms_of_morpheus_keeps_the_block_restrictions() {
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let z = e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(3), plain("Toreador", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 2, &["Eluding the Arms of Morpheus"]);
    pick(&mut e, bleed(a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, z));
}

// Guardian Vigil: "Requires an Anarch. More than one Discipline can be used
// to play this card. This Anarch attempts to block. … [for] This Anarch
// does not lock for blocking this action." [ANK 20180926] played before the
// block succeeds: the Anarch blocks and stays unlocked.
// Rulings: [LSJ 20011204-3] [PIB 20130704].
#[test]
fn guardian_vigil_fortitude_blocks_without_locking() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), with_disc("", Some(Sect::Anarch), 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    superior(&mut e, z, "for");
    hand(&mut e, 1, &["Guardian Vigil"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 2, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10, "blocked");
    assert!(!e.state.minions[z.index()].locked, "no lock for the block");
}

// Guardian Vigil: "Requires an Anarch": a non-Anarch never gets it.
#[test]
fn guardian_vigil_needs_an_anarch() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), with_disc("", Some(Sect::Camarilla), 0), 4, false);
    superior(&mut e, z, "for");
    hand(&mut e, 1, &["Guardian Vigil"]);
    pick(&mut e, bleed(a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 2, z));
}

// ---- Locations ----------------------------------------------------------------------

// Palatial Estate: "During your master phase, the vampire with this location
// gains 1 blood." Mandatory, in its controller's master phase.
#[test]
fn palatial_estate_gives_its_vampire_a_blood_each_master_phase() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 1, "Palatial Estate", Some(z));
    // The master phase's only option: elided.
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 2 + 1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 2 + 1, "not in another's master phase");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 2 + 2, "again in the next one");
}

// Island of Yiaros: "Lock to give a striking vampire you control +X strength
// for that strike …, where X is the number of votes the vampire has (e.g.,
// a prince gets +2 strength)." A prince's hand strike for 3.
#[test]
fn island_of_yiaros_adds_a_strikers_votes_to_its_strength() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let prince = CRYPT
        .iter()
        .enumerate()
        .find(|(i, c)| {
            matches!(c.title, Title::Prince(_)) && zvtes::cards::vampire(*i as u16).is_none()
        })
        .map(|(i, _)| i as u16)
        .expect("a prince");
    let z = e.add_minion(SeatId(1), prince, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let island = in_play(&mut e, 1, "Island of Yiaros", None);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, use_at(island, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 3);
    assert!(e.state.cards[island.index()].locked);
}

// ---- Vampires --------------------------------------------------------------------

// Alessandro Garcia (G4): "Sabbat: Alessandro gets +1 strength in combat
// with a Lasombra."
#[test]
fn alessandro_garcia_strikes_harder_against_a_lasombra() {
    let mut e = game(2);
    let al = e.add_minion(SeatId(0), find_card("Alessandro Garcia"), 4, false);
    let l = e.add_minion(SeatId(1), plain("Lasombra", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let vs = |o: MinionId| {
        e.state.printed_mod(
            al,
            Stat::Strength,
            PrintedCtx {
                opponent: Some(o),
                ..Default::default()
            },
        )
    };
    assert_eq!((vs(l), vs(v)), (1, 0));
}

// Laika (G4): "Sabbat: Laika can burn 1 blood to strike: 2R damage."
#[test]
fn laika_burns_a_blood_to_strike_for_two() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let l = e.add_minion(SeatId(1), find_card("Laika"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(l));
    pick_when(&mut e, own_use(l, 0, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 2);
    assert_eq!(blood(&e, l), 4 - 1 - 1, "the blood, then A's hand strike");
}

// Xipe Totec (G5): "Sabbat Archbishop of Guatemala City: Xipe can steal an
// ally or retainer as a +1 stealth Ⓓ action that costs 1 blood. +1
// strength." [TOM 19960226-1] requirements do not apply to a card taken in
// play: another Methuselah's ally becomes his controller's.
#[test]
fn xipe_totec_steals_an_ally_for_a_blood() {
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), find_card("Xipe Totec"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    assert_eq!(stat(&e, x, Stat::Strength), 1);
    pick(&mut e, printed(x, 0, Target::Minion(ally)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[ally.index()].owner, SeatId(0));
    assert_eq!(blood(&e, x), 4 - 1);
}

// Luc (G4): "Sabbat: Luc gets +1 stealth during diablerie actions. He can
// give you +3 hand size until your next unlock phase as a +1 stealth
// action."
#[test]
fn luc_gives_three_hand_size_until_the_next_unlock_phase() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Luc"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    library(&mut e, 0, 20);
    let base = e.state.hand_size(SeatId(0));
    pick(&mut e, printed(l, 0, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.hand_size(SeatId(0)), base + 3);
    assert_eq!(e.state.hand_load(SeatId(0)), base + 3, "drawn up");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.hand_size(SeatId(0)), base, "until the unlock phase");
}

// Thucimia (G4): "During a bleed action, Thucimia can give you -2 hand size
// until your next unlock phase to get +1 bleed. … +1 hand size." Once each
// bleed (rules-digest D7), before resolution [ANK 20231124] [LSJ
// 19970718].
#[test]
fn thucimia_trades_two_hand_size_for_a_bleed_once_each_bleed() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Thucimia"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let base = e.state.hand_size(SeatId(0));
    assert_eq!(stat(&e, t, Stat::HandSize), 1);
    pick(&mut e, bleed(t));
    let trade = own_use(t, 0, Target::None);
    pick_when(&mut e, trade);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, trade);
    assert_eq!(pool(&e)[1], 10 - 2);
    assert_eq!(e.state.hand_size(SeatId(0)), base - 2);
}

// ---- Lasting bonuses ------------------------------------------------------------

// Dive into Madness [dem]: "+1 stealth action. This vampire gets superior
// Dementation [DEM] until the end of the turn and unlocks." Superior for
// the rest of the turn only.
#[test]
fn dive_into_madness_gives_superior_dementation_until_the_end_of_the_turn() {
    let mut e = game(2);
    let d = e.add_minion(
        SeatId(0),
        textless(|c| c.disciplines.contains(&"dem"), 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Dive into Madness"]);
    assert!(!e.state.has_discipline(d, "DEM"));
    pick(&mut e, play_at(0, 0, d, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.has_discipline(d, "DEM"));
    assert!(!e.state.minions[d.index()].locked, "unlocked");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !e.state.has_discipline(d, "DEM"),
        "until the end of the turn"
    );
}

// Dive into Madness [DEM]: "Put this card on this vampire. This vampire gets
// +1 bleed." The card itself in play, +1 bleed.
// Rulings: [PIB 20130311].
#[test]
fn dive_into_madness_superior_goes_on_the_vampire_for_a_bleed() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), textless(|c| c.capacity <= 5, 0), 4, false);
    superior(&mut e, d, "dem");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Dive into Madness"]);
    let base = e.state.current_bleed(d);
    let req = request(&mut e);
    assert!(
        req.options.contains(&play_at(0, 0, d, Target::None)),
        "[dem] at [DEM] too"
    );
    pick(&mut e, play_at(0, 1, d, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let dive = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.host == Some(d))
        .map(|i| CardId(i as u32))
        .expect("on the vampire");
    assert!(!e.state.cards[dive.index()].effect_only);
    assert!(!e.state.seats[0]
        .ash
        .contains(&find_library("Dive into Madness")));
    assert_eq!(e.state.current_bleed(d), base + 1);
}

// Dive into Madness [DEM]: "Older vampires can burn this card as a Ⓓ
// action. A vampire can have no more than two Dive into Madness." With two
// on him, no third; an older vampire may burn one, a younger one may not.
#[test]
fn dive_into_madness_caps_at_two_and_older_vampires_burn_it() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), textless(|c| c.capacity == 5, 0), 4, false);
    superior(&mut e, d, "dem");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let first = in_play(&mut e, 0, "Dive into Madness", Some(d));
    in_play(&mut e, 0, "Dive into Madness", Some(d));
    hand(&mut e, 0, &["Dive into Madness"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play_at(0, 0, d, Target::None)));
    assert!(
        !req.options.contains(&play_at(0, 1, d, Target::None)),
        "no third"
    );
    // Another Methuselah's vampires: the older one may burn it.
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let old = e.add_minion(SeatId(0), textless(|c| c.capacity >= 7, 0), 4, false);
    let young = e.add_minion(SeatId(0), textless(|c| c.capacity <= 4, 0), 4, false);
    let d = e.add_minion(SeatId(1), textless(|c| c.capacity == 5, 0), 4, false);
    let dive = in_play(&mut e, 1, "Dive into Madness", Some(d));
    let req = request(&mut e);
    assert!(req
        .options
        .contains(&use_at(dive, 0, Some(old), Target::None)));
    assert!(!req
        .options
        .contains(&use_at(dive, 0, Some(young), Target::None)));
    let _ = first;
}

// Gear Up [pre]: "This Anarch unlocks and gets +1 stealth until the end of
// the turn." His next action this turn at +1 stealth; gone next turn.
#[test]
fn gear_up_presence_gives_stealth_until_the_end_of_the_turn() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), with_disc("", Some(Sect::Anarch), 0), 4, false);
    superior(&mut e, g, "pre");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Gear Up"]);
    pick(&mut e, play_at(0, 1, g, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[g.index()].locked);
    assert_eq!(e.state.lasting(g).0, 1);
    // The bleed at stealth 1: seat 1's vampire, 0 intercept, cannot reach.
    pick(&mut e, bleed(g));
    never_reaching_until(&mut e, SeatId(1), WindowLabel::MinionPhase, z);
    assert_eq!(e.state.lasting(g).0, 0, "until the end of the turn");
}

// Gear Up [vic]: "This Anarch unlocks and gets +1 strength until your
// unlock phase." Through the other Methuselah's turn; gone in his own
// unlock phase.
// Rulings: [LSJ 20011204-3] [PIB 20130704].
#[test]
fn gear_up_vicissitude_gives_strength_until_your_unlock_phase() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), with_disc("", Some(Sect::Anarch), 0), 4, false);
    superior(&mut e, g, "vic");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Gear Up"]);
    pick(&mut e, play_at(0, 2, g, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.lasting(g).1, 1, "through the next turn");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.lasting(g).1, 0, "until your unlock phase");
}

// Gear Up [nec]: "This Anarch unlocks. Move a card from your ash heap to
// your hand." [LSJ 20010627] "The card to retrieve must be announced when
// declaring the action": named as it is played, fetched on success.
#[test]
fn gear_up_necromancy_fetches_the_card_named_as_it_is_played() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), with_disc("", Some(Sect::Anarch), 0), 4, false);
    superior(&mut e, g, "nec");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Gear Up"]);
    let (conceal, deflection) = (find_library("Conceal"), find_library("Deflection"));
    e.state.seats[0].ash = vec![conceal, deflection].into();
    pick(&mut e, play_at(0, 0, g, Target::None));
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::FindCard(conceal)));
    e.choose(index_of(&req, Choice::FindCard(deflection)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].hand.contains(&deflection));
    assert!(e.state.seats[0].ash.contains(&conceal));
    assert!(!e.state.minions[g.index()].locked);
}

// ---- Action modifiers and combat cards, part 2 ------------------------------------

// Acheron Vortex: "Minions without Necromancy [nec] or Obtenebration [obt]
// get -1 intercept." [LSJ 20020612] any time during the action, no block
// attempt needed.
#[test]
fn acheron_vortex_lowers_intercept_of_minions_without_nec_or_obt() {
    let mut e = game(2);
    let harbinger = CRYPT
        .iter()
        .position(|c| c.clan == "Harbinger of Skulls")
        .expect("a Harbinger") as u16;
    let a = e.add_minion(SeatId(0), harbinger, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let n = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    superior(&mut e, n, "obt");
    hand(&mut e, 0, &["Acheron Vortex"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    let mods = |e: &Engine, m| zvtes::actions::ActionCtx::intercept_mods(&e.state, m);
    assert_eq!((mods(&e, z), mods(&e, n)), (-1, 0));
}

// Repulsion [OBE]: "Put this card on this vampire. This vampire gets +1
// stealth. Burn this card if this vampire is in torpor. This vampire cannot
// have or play another Repulsion." [LSJ 20011214-2] usable though stealth is
// not needed: on him, +1 stealth to his next action; no second Repulsion.
#[test]
fn repulsion_superior_stays_on_the_vampire_for_stealth() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    superior(&mut e, r, "obe");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 0, &["Repulsion", "Repulsion"]);
    pick(&mut e, bleed(r));
    pick_when(&mut e, play(0, 2, r));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let rep = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.host == Some(r))
        .expect("on the vampire");
    assert!(!e.state.cards[rep].effect_only);
    assert!(!e.state.seats[0].ash.contains(&find_library("Repulsion")));
    // His next action, next turn: the card in play gives +1; a second
    // Repulsion is never offered.
    let repulsions = |req: &zvtes::decision::DecisionRequest| {
        plays(req)
            .into_iter()
            .filter(|c| matches!(c, Choice::Play { actor: Some(m), .. } if *m == r))
            .count()
    };
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(r));
    let req = request(&mut e);
    assert_eq!(repulsions(&req), 0, "no other Repulsion");
    never_reaching_until(&mut e, SeatId(1), WindowLabel::MinionPhase, z);
}

// Repulsion [pre]: "Only usable as the action is announced. If this action is
// blocked, this vampire gets 1 optional maneuver, only usable to get to long
// range, during the resulting combat." At close range, a maneuver to long.
// Rulings: [LSJ 20011204-3] [PIB 20130704].
#[test]
fn repulsion_presence_gives_a_maneuver_to_long_range_if_blocked() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), with_disc("pre", None, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Repulsion"]);
    pick(&mut e, bleed(r));
    pick_when(&mut e, play(0, 0, r));
    pick_when(&mut e, block(z));
    pick_when(&mut e, Choice::UseManeuver);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // At long range, both hand strikes miss.
    assert_eq!((blood(&e, r), blood(&e, z)), (4, 4));
}

// Anesthetic Touch [OBE]: "Strike: hand strike. Combat ends immediately after
// the resolution of this strike … with first strike." [LSJ 20011210-1] a
// hand strike, combat ending after its resolution: the opposing hand strike,
// held back by the first strike, never resolves.
#[test]
fn anesthetic_touch_superior_ends_combat_before_the_other_strike() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    superior(&mut e, z, "obe");
    hand(&mut e, 1, &["Anesthetic Touch"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 2, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, z)), (4 - 1, 4));
}

// Anesthetic Touch [obe]: [LSJ 20011210-2] "A dodge won't prevent combat
// from ending after strike resolution": dodged by Side Strike, the combat
// still ends — no press offered to the acting vampire's Pursuit.
// [LSJ 20011210-3] "the damage can be prevented or healed as normal".
#[test]
fn anesthetic_touch_ends_combat_though_dodged() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_disc("cel", None, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    superior(&mut e, z, "obe");
    hand(&mut e, 0, &["Side Strike"]);
    hand(&mut e, 1, &["Anesthetic Touch"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.combat().is_none());
    assert_eq!(blood(&e, a), 4, "dodged");
}

// Harass: "Ⓓ Enter combat with a vampire who has 3 or fewer blood or with a
// locked minion." An unlocked vampire with 4 blood is no target; one with
// 3, or a locked one, is.
#[test]
fn harass_targets_a_low_blood_vampire_or_a_locked_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let full = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let low = e.add_minion(SeatId(1), plain("Ventrue", 1), 3, false);
    let locked = e.add_minion(SeatId(1), plain("Toreador", 0), 5, false);
    e.state.minions[locked.index()].locked = true;
    hand(&mut e, 0, &["Harass"]);
    let req = request(&mut e);
    let at = |m| play_at(0, 0, a, Target::Minion(m));
    assert!(!req.options.contains(&at(full)));
    assert!(req.options.contains(&at(low)));
    assert!(req.options.contains(&at(locked)));
}

// Harass: "This acting minion gets 1 optional press, only usable to continue
// combat, during that combat." The press continues into a second round.
#[test]
fn harass_gives_a_press_to_continue() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Harass"]);
    pick(&mut e, play_at(0, 0, a, Target::Minion(z)));
    pick_when(&mut e, Choice::UsePress);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 3 - 2, "two rounds of hand strikes");
}

// Tainted Spring [ANI]: "Only usable as an action to recruit or employ an
// animal is announced. If this action is successful, the animal gets +1
// strength (for an ally) … and the animal gets 1 additional starting life."
// Underbridge Stray, 0 strength and 1 life, enters with 1 and 2.
#[test]
fn tainted_spring_superior_strengthens_a_recruited_animal_ally() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    superior(&mut e, a, "ani");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Underbridge Stray", "Tainted Spring"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let stray = e
        .state
        .minions
        .iter()
        .position(|m| m.in_play && m.is_ally())
        .map(|i| MinionId(i as u32))
        .expect("the ally");
    let body = e.state.ally_body(stray).expect("an ally");
    assert_eq!((body.strength, body.life), (1, 2));
    assert_eq!(blood(&e, stray), 2, "its life");
}

// Tainted Spring [ani]: "… or inflicts +1 damage (for damage-dealing
// retainers)." [TOM 19960225] the extra damage as the base damage: Murder
// of Crows inflicts 2 each round.
#[test]
fn tainted_spring_adds_to_a_retainers_damage() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    superior(&mut e, a, "ani");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Murder of Crows", "Tainted Spring"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.retainer_damage(a), 2);
    let crows = e
        .state
        .cards
        .iter()
        .find(|c| c.in_play && c.host == Some(a))
        .expect("the retainer");
    assert_eq!(crows.counters, 1, "no extra life at [ani]");
}

// Divine Sign [NEC]: "Ⓓ Choose any minion, put this card on this acting
// minion, and unlock this acting minion. This minion gets +2 intercept
// against the chosen minion. The chosen minion can burn this card as a Ⓓ
// action. … if this minion blocks the chosen minion, their controller plays
// with an open hand during the resulting combat." The card itself on the
// acting vampire, which unlocks; next turn the chosen vampire's bleed meets
// +2 intercept and, blocked, an open hand; only it may burn the card.
#[test]
fn divine_sign_superior_guards_against_the_chosen_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    superior(&mut e, a, "nec");
    superior(&mut e, a, "cel");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    // Side Strike: a decision for A inside the combat.
    hand(&mut e, 0, &["Divine Sign", "Side Strike"]);
    pick(&mut e, play_at(0, 1, a, Target::Minion(z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let sign = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.host == Some(a))
        .map(|i| CardId(i as u32))
        .expect("on the acting vampire");
    assert!(!e.state.cards[sign.index()].effect_only);
    assert!(!e.state.seats[0].ash.contains(&find_library("Divine Sign")));
    assert_eq!(e.state.cards[sign.index()].chosen, vec![z]);
    assert!(!e.state.minions[a.index()].locked, "unlocked");
    // Seat 1's turn: only the chosen vampire may burn the card.
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(req
        .options
        .contains(&use_at(sign, 0, Some(z), Target::None)));
    assert!(!req
        .options
        .contains(&use_at(sign, 0, Some(y), Target::None)));
    pick(&mut e, bleed(z));
    request(&mut e);
    let act = e.action().expect("action");
    assert_eq!(act.host_intercept(&e.state, a), 2);
    pick_when(&mut e, block(a));
    request(&mut e);
    assert!(e.combat().is_some());
    assert!(e.state.hand_visible_to(SeatId(1), SeatId(0)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !e.state.hand_visible_to(SeatId(1), SeatId(0)),
        "that combat only"
    );
}

// Pulled Fangs: "Only usable at close range at the end of a round during
// which this minion successfully inflicted more damage than the opposing
// vampire. … Put this card on the opposing vampire; this minion inflicts 1
// damage on them. The attached vampire cannot hunt; if they must hunt, they
// go to torpor instead." [TOM 19950304] the minions' damage counted: 3 from
// a Sawed-Off Shotgun against 1; the card itself on the blocker, its 1
// damage to 0 blood; at 0 blood in its controller's minion phase, the
// vampire must hunt and goes to torpor instead.
// Rulings: [TOM 19950304] [LSJ 20010812]; [RTR 20030519].
#[test]
fn pulled_fangs_goes_on_the_loser_and_sends_a_starving_bearer_to_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let gun = in_play(&mut e, 0, "Sawed-Off Shotgun", Some(a));
    hand(&mut e, 0, &["Pulled Fangs"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, use_at(gun, 0, None, Target::None));
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    let fangs = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.host == Some(z))
        .map(|i| CardId(i as u32))
        .expect("on the opposing vampire");
    assert!(!e.state.cards[fangs.index()].effect_only);
    assert!(!e.state.seats[0].ash.contains(&find_library("Pulled Fangs")));
    assert_eq!(blood(&e, z), 0, "3 from the gun, then 1");
    assert!(!e.state.minions[z.index()].in_torpor);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        e.state.minions[z.index()].in_torpor,
        "must hunt: torpor instead"
    );
}

// Pulled Fangs: "Any vampire(s) can burn this card with two +1 stealth
// actions." [ANK 20170202-2] "A total of two actions must be performed (by
// whomever) to remove it": the first leaves it, the second, by another
// vampire, burns it. Meanwhile its bearer is offered no hunt.
#[test]
fn pulled_fangs_burns_after_two_actions_and_bars_the_hunt() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let d = e.add_minion(SeatId(0), plain("Brujah", 2), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let fangs = in_play(&mut e, 1, "Pulled Fangs", Some(d));
    let hunt = Choice::TakeAction {
        minion: d,
        action: zvtes::decision::ActionKind::Hunt,
        target: None,
    };
    let req = request(&mut e);
    assert!(!req.options.contains(&hunt), "cannot hunt");
    pick(&mut e, use_at(fangs, 0, Some(a), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        e.state.cards[fangs.index()].in_play,
        "one action is not enough"
    );
    assert_eq!(e.state.cards[fangs.index()].counters, 1);
    pick(&mut e, use_at(fangs, 0, Some(b), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[fangs.index()].in_play);
    let req = request(&mut e);
    assert!(req.options.contains(&hunt), "free to hunt again");
}

// Clandestine Contract: "+1 stealth action. Contract. Requires an Assamite
// with capacity 5 or more. Ⓓ Enter combat with a minion and put this card
// on the target minion. This Assamite is chosen for this contract. This
// Assamite can enter combat with the attached minion as a +1 stealth Ⓓ
// action." [ANK 20220910] two different actions: on the target with the
// acting Assamite chosen; next turn only he gets the granted action; an
// Assamite of capacity 4 cannot play it.
#[test]
fn clandestine_contract_chooses_the_acting_assamite() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Banu Haqim" && c.capacity >= 5, 0),
        4,
        false,
    );
    let small = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Banu Haqim" && c.capacity < 5, 0),
        4,
        false,
    );
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Clandestine Contract"]);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&play_at(0, 0, small, Target::Minion(z))));
    pick(&mut e, play_at(0, 0, a, Target::Minion(z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let contract = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.host == Some(z))
        .map(|i| CardId(i as u32))
        .expect("on the target");
    assert!(!e.state.cards[contract.index()].effect_only);
    assert!(!e.state.seats[0]
        .ash
        .contains(&find_library("Clandestine Contract")));
    assert_eq!(e.state.cards[contract.index()].chosen, vec![a]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req
        .options
        .contains(&use_at(contract, 0, Some(a), Target::Minion(z))));
    assert!(!req
        .options
        .contains(&use_at(contract, 0, Some(small), Target::Minion(z))));
}

// Clandestine Contract with The Black Throne: "You can lock this card after
// a minion with a contract for which an Assamite you control is chosen
// leaves the ready region to gain 1 pool." Any contract: the chosen
// Assamite's own combat sends the attached vampire to torpor; the lock is
// offered, for 1 pool.
#[test]
fn clandestine_contract_feeds_the_black_throne() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Banu Haqim" && c.capacity >= 5, 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[z.index()].blood = 0;
    let contract = in_play(&mut e, 0, "Clandestine Contract", Some(z));
    e.state.cards[contract.index()].chosen = vec![a];
    let throne = in_play(&mut e, 0, "Black Throne, The", None);
    pick(&mut e, use_at(contract, 0, Some(a), Target::Minion(z)));
    let lock = Choice::ResolveRider { card: throne.0 };
    pick_when(&mut e, lock);
    request(&mut e);
    assert!(e.state.minions[z.index()].in_torpor);
    assert!(e.state.cards[throne.index()].locked);
    assert_eq!(pool(&e)[0], 10 + 1);
}

// The Book of Going Forth by Night: "This vampire gets superior Serpentis
// [SER]. If an ally opposing this vampire in combat would be burned,
// instead steal and lock the ally; if the ally has lost their last life,
// they gain 1 life." An Underbridge Stray hit for its last life by the
// bearer's hand strike changes sides, locked, with 1 life.
// Rulings: [TOM 19960226-1].
#[test]
fn the_book_of_going_forth_by_night_steals_an_ally_about_to_burn() {
    let mut e = game(2);
    let stray = e.add_ally(SeatId(0), find_library("Underbridge Stray"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 1, "Book of Going Forth by Night, The", Some(z));
    assert!(e.state.has_discipline(z, "SER"));
    pick(&mut e, bleed(stray));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[stray.index()];
    assert!(v.in_play, "not burned");
    assert_eq!(v.owner, SeatId(1));
    assert!(v.locked);
    assert_eq!(v.blood, 1);
}

// ---- Vampires, part 2 -------------------------------------------------------------

// Asguresh (G5): "During a combat, Asguresh can cancel a combat card played
// by the opposing minion as it is played, and its cost is not paid."
// [LSJ 20100206] "If his ability is used to cancel a strike, another strike
// is chosen": the Side Strike canceled, a strike chosen again; once each
// combat (rules-digest D7): the second Side Strike stands.
#[test]
fn asguresh_cancels_a_combat_card_once_each_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    superior(&mut e, a, "cel");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Asguresh"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Side Strike", "Side Strike"]);
    let cancel = Choice::UseAbility {
        source: AbilitySource::Vampire(g),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(g));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, cancel);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(
        req.options.contains(&play(0, 0, a)),
        "another strike chosen"
    );
    pick(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, cancel);
}

// Adhiambo (G4): "Laibon: Adhiambo gets +1 stealth during actions to equip
// with a weapon card from your hand and actions to enter combat with a
// minion." Not during a bleed.
#[test]
fn adhiambo_gets_stealth_equipping_weapons_and_entering_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Adhiambo"), 4, false);
    let with = |kind, weapon, combat| zvtes::state::PrintedCtx {
        kind: Some(kind),
        equip_weapon: weapon,
        enters_combat: combat,
        ..Default::default()
    };
    use zvtes::decision::ActionKind as K;
    let s = |e: &Engine, c| e.state.printed_mod(a, Stat::Stealth, c);
    assert_eq!(s(&e, with(K::Equip, true, false)), 1);
    assert_eq!(s(&e, with(K::Ability, false, true)), 1);
    assert_eq!(s(&e, with(K::Bleed, false, false)), 0);
}

// Tyler McGill (G5): "Camarilla: In a referendum, Tyler gets +1 vote after a
// Methuselah burns the Edge for a vote. He cannot block Nosferatu." Seat 0
// burns the Edge for a vote; untitled Tyler then casts 1.
#[test]
fn tyler_mcgill_votes_after_the_edge_is_burned_for_a_vote() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), find_card("Tyler McGill"), 4, false);
    e.state.seats[0].has_edge = true;
    hand(&mut e, 0, &["Rumors of Gehenna"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::ChooseSeat(SeatId(1)));
    pick_when(&mut e, Choice::Pass);
    to_referendum(&mut e);
    pick_when(&mut e, Choice::CastEdge { favor: true });
    pick_when(&mut e, votes(t, false));
    let r = e.referendum().expect("referendum");
    assert!(r.records.iter().any(|v| v.n == 1 && !v.favor));
}

// Tyler McGill: "He cannot block Nosferatu." Not a Nosferatu's bleed; a
// Ventrue's, yes.
#[test]
fn tyler_mcgill_cannot_block_nosferatu() {
    for (clan, blocks) in [("Nosferatu", false), ("Ventrue", true)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain(clan, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let t = e.add_minion(SeatId(1), find_card("Tyler McGill"), 4, false);
        e.add_minion(SeatId(1), plain("Toreador", 0), 4, false);
        pick(&mut e, bleed(a));
        let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
        assert_eq!(req.options.contains(&block(t)), blocks, "{clan}");
    }
}

// Saulot, The Wanderer (G4): "Damage Baali inflict on Saulot is reduced to
// 0. … +1 bleed. +1 strength." A Baali's hand strike does nothing to him;
// his own strikes for 2.
#[test]
fn saulot_takes_no_damage_from_baali() {
    let mut e = game(2);
    let baali = CRYPT
        .iter()
        .position(|c| c.clan == "Baali")
        .expect("a Baali") as u16;
    let b = e.add_minion(SeatId(0), baali, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let s = e.add_minion(SeatId(1), find_card("Saulot, The Wanderer"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert_eq!(stat(&e, s, Stat::Bleed), 1);
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(s));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, s), 4);
    assert_eq!(blood(&e, b), 4 - 2);
}

// Saulot, The Wanderer: "During your unlock phase, you can move 2 blood from
// him to a vampire in play or in your uncontrolled region." To an
// uncontrolled vampire.
#[test]
fn saulot_moves_two_blood_to_an_uncontrolled_vampire() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let s = e.add_minion(SeatId(1), find_card("Saulot, The Wanderer"), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: plain("Ventrue", 1),
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    pick_when(&mut e, own_use(s, 1, Target::Uncontrolled(0)));
    request(&mut e);
    assert_eq!(blood(&e, s), 6 - 2);
    assert_eq!(e.state.seats[1].uncontrolled[0].counters, 2);
}

// Paulo de Castille (G4): "After Paulo enters play during your influence
// phase, you can move 1 blood from him to a Lasombra in your uncontrolled
// region." Influenced with 4, 1 moves to the Lasombra beside him.
#[test]
fn paulo_de_castille_moves_a_blood_to_an_uncontrolled_lasombra() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let slot = |card| zvtes::state::UncontrolledSlot {
        card,
        copy: Default::default(),
        counters: 0,
        banished: None,
    };
    let paulo = CRYPT
        .iter()
        .position(|c| c.name == "Paulo de Castille")
        .expect("Paulo") as u16;
    e.state.seats[0].uncontrolled = vec![slot(paulo), slot(plain("Lasombra", 0))];
    e.state.seats[0].uncontrolled[0].counters = 4;
    let req = to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    let out = req
        .options
        .iter()
        .copied()
        .find(|c| matches!(c, Choice::InfluenceOut { slot: 0 }))
        .expect("move Paulo out");
    e.choose(index_of(&req, out));
    let req = request(&mut e);
    let to_lasombra = req
        .options
        .iter()
        .copied()
        .find(|c| *c != Choice::Pass)
        .expect("the move offered");
    e.choose(index_of(&req, to_lasombra));
    request(&mut e);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 1);
}

// Leila Monroe (G4): "Sabbat: Leila can change the sect of an Anarch with no
// blood to Sabbat as a +1 stealth Ⓓ action." An Anarch with blood is no
// target; one with none becomes Sabbat.
#[test]
fn leila_monroe_makes_an_empty_anarch_sabbat() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Leila Monroe"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let full = e.add_minion(SeatId(1), with_disc("", Some(Sect::Anarch), 0), 2, false);
    let empty = e.add_minion(SeatId(1), with_disc("", Some(Sect::Anarch), 1), 0, false);
    let req = request(&mut e);
    assert!(!req.options.contains(&printed(l, 0, Target::Minion(full))));
    pick(&mut e, printed(l, 0, Target::Minion(empty)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.sect_of(empty), Some(Sect::Sabbat));
}

// Mateusz Kazimirski, Biker of Blood (G6): "If Mateusz is Sabbat and not
// Black Hand, he can become Black Hand as a +1 stealth action that costs 1
// blood." [LSJ 20070322] [ANK 20180807] a trait, not lost with a sect
// change; no second time.
#[test]
fn mateusz_becomes_black_hand_for_a_blood() {
    let mut e = game(2);
    let m = e.add_minion(
        SeatId(0),
        find_card("Mateusz Kazimirski, Biker of Blood"),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert!(!e.state.is_black_hand(m));
    pick(&mut e, printed(m, 0, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.is_black_hand(m));
    assert_eq!(blood(&e, m), 4 - 1);
    e.state.minions[m.index()].sect_override = Some(Sect::Camarilla);
    assert!(e.state.is_black_hand(m), "kept through a sect change");
}

// Lucinde, Alastor (G7): "During your master phase, Lucinde can burn 1 blood
// to choose a vampire; this turn, Camarilla vampires you control can enter
// combat with the chosen vampire as a Ⓓ action. +1 bleed." [LSJ 20011022]
// in torpor too: chosen from torpor, the other Camarilla vampire takes the
// action; next turn, no longer.
#[test]
fn lucinde_g7_lets_camarilla_vampires_attack_her_chosen_vampire() {
    let mut e = game(2);
    let lucinde = CRYPT
        .iter()
        .position(|c| c.name == "Lucinde, Alastor" && c.group == 7)
        .expect("Lucinde G7") as u16;
    let l = e.add_minion(SeatId(1), lucinde, 4, true);
    let c = e.add_minion(SeatId(1), with_disc("", Some(Sect::Camarilla), 0), 4, false);
    let z = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let choose = own_use(l, 0, Target::Minion(z));
    pick_when(&mut e, choose);
    let attack = Choice::UseAbility {
        source: AbilitySource::Vampire(l),
        ability: 1,
        actor: Some(c),
        target: Target::Minion(z),
    };
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(req.options.contains(&attack));
    assert_eq!(blood(&e, l), 4 - 1);
    pick(&mut e, attack);
    request(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&attack), "this turn only");
}

// ---- Further rulings -----------------------------------------------------------

// Anesthetic Touch [OBE]: [LSJ 20011210-3] "Does not end combat until after
// strike resolution, so the damage can be prevented or healed as normal":
// the acting vampire's Flak Jacket prevents the Touch's damage.
#[test]
fn anesthetic_touch_damage_is_prevented_as_usual() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    superior(&mut e, z, "obe");
    let jacket = in_play(&mut e, 0, "Flak Jacket", Some(a));
    hand(&mut e, 1, &["Anesthetic Touch"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 2, z));
    pick_when(&mut e, use_at(jacket, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, z)), (4, 4));
}

// Hide the Mind [obf] against Anesthetic Touch: [PIB 20130704] "Cannot be
// used to cancel a card that could be played with [aus] but has not been
// (eg. Anesthetic Touch played with [obe])"; [LSJ 20011204-3] [PIB
// 20130704] played, a multi-Discipline card requires the Discipline used:
// the [aus] dodge is canceled, the [obe] strike is not.
#[test]
fn hide_the_mind_cancels_anesthetic_touch_only_played_with_auspex() {
    for (mode, cancels) in [(0u8, true), (1u8, false)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        superior(&mut e, a, "obf");
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        superior(&mut e, z, "aus");
        superior(&mut e, z, "obe");
        hand(&mut e, 0, &["Hide the Mind"]);
        hand(&mut e, 1, &["Anesthetic Touch"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, mode, z));
        let req = request(&mut e);
        assert_eq!(req.options.contains(&play(0, 0, a)), cancels, "mode {mode}");
    }
}

// Guardian Vigil [cel]: "This Anarch gets 1 optional maneuver during the
// resulting combat if they block." [LSJ 20010813] [LSJ 20010819-2] in the
// block's combat: at close range, the maneuver offered.
#[test]
fn guardian_vigil_celerity_maneuvers_in_the_blocks_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), with_disc("", Some(Sect::Anarch), 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    superior(&mut e, z, "cel");
    hand(&mut e, 1, &["Guardian Vigil"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, z));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&Choice::UseManeuver));
}

// Harass: [LSJ 20070411] [LSJ 20090514] "If the action is not blocked and
// the target has now more then 3 blood and is unlocked, the action
// 'fizzles'": the target fed to 4 before resolution, no combat.
#[test]
fn harass_fizzles_on_a_target_now_out_of_reach() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Harass"]);
    pick(&mut e, play_at(0, 0, a, Target::Minion(z)));
    e.state.minions[z.index()].blood = 4;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, z)), (4, 4), "no combat");
}

// Asguresh: [LSJ 20011023] [LSJ 20080630] "If a canceled card had a 'Do Not
// Replace Until' clause on it …, that clause is canceled as well and the
// card is replaced normally": a canceled Lucky Blow is replaced at once.
#[test]
fn asguresh_cancels_lucky_blow_replaced_at_once() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    superior(&mut e, a, "cel");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Asguresh"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    // Side Strike: a strike still to choose, the combat paused.
    hand(&mut e, 0, &["Lucky Blow", "Side Strike"]);
    library(&mut e, 0, 10);
    let full = e.state.hand_size(SeatId(0));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(g));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, own_use(g, 0, Target::None));
    request(&mut e);
    assert!(e.combat().is_some());
    assert_eq!(e.state.hand_load(SeatId(0)), full, "replaced at once");
}

// Island of Yiaros: [RTR 19970630] "Provides no additional strength to a
// Priscus for his ballots": a priscus's votes held are none.
#[test]
fn island_of_yiaros_gives_a_priscus_nothing() {
    let mut e = game(2);
    let priscus = CRYPT
        .iter()
        .enumerate()
        .find(|(i, c)| c.title == Title::Priscus && zvtes::cards::vampire(*i as u16).is_none())
        .map(|(i, _)| i as u16)
        .expect("a priscus");
    let p = e.add_minion(SeatId(1), priscus, 4, false);
    assert_eq!(e.state.votes_held(p), 0);
}

// Deed the Heart's Desire [QUI]: [ANK 20231028] "An opponent can play a
// reaction to wake as the card is played, before its effects apply": the
// chosen locked vampire may wake in the card's as-played window.
#[test]
fn deed_the_hearts_desire_superior_leaves_a_wake_as_it_is_played() {
    let (mut e, a, z) = older_and_younger("qui");
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 0, &["Deed the Heart's Desire"]);
    hand(&mut e, 1, &["Wake with Evening's Freshness"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play_at(0, 1, a, Target::Minion(z)));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::CardAsPlayed);
    assert!(req.options.contains(&play(0, 0, z)));
}

// Pulled Fangs: its 1 damage is not strike damage but may be prevented
// (owner, 2026-10-15: rules-qa 74.1): the attached vampire's Flak Jacket
// prevents it at the end of the round.
#[test]
fn pulled_fangs_damage_may_be_prevented() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let gun = in_play(&mut e, 0, "Sawed-Off Shotgun", Some(a));
    let jacket = in_play(&mut e, 1, "Flak Jacket", Some(z));
    hand(&mut e, 0, &["Pulled Fangs"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, use_at(gun, 0, None, Target::None));
    // The jacket kept for the Fangs, not the gun's 3.
    let prevent = use_at(jacket, 0, None, Target::None);
    let req = until(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    e.choose(index_of(&req, Choice::Pass));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, prevent);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 3, "the 1 prevented");
}

// Pulled Fangs after Body Flare burned the opposing vampire: no vampire is
// left to put it on, it is not offered (rules-qa 75.1); the burned vampire
// is not sent to torpor.
#[test]
fn pulled_fangs_is_not_offered_against_a_burned_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("PRO", 1), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Body Flare", "Pulled Fangs"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
    let m = &e.state.minions[z.index()];
    assert!(!m.in_play && !m.in_torpor, "burned, not in torpor");
}

// Pulled Fangs after an aggravated strike sent the opposing vampire to
// torpor with blood left: owner (2026-10-15, rules-qa 74.13) "Pulled Fang's
// damage must land on them even in torpor": he mends it with 1 blood.
#[test]
fn pulled_fangs_lands_on_a_vampire_sent_to_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("PRO", 1), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Body Flare", "Pulled Fangs"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert!(e.state.cards.iter().any(|c| c.in_play && c.host == Some(z)));
    assert!(e.state.minions[z.index()].in_torpor);
    assert_eq!(
        blood(&e, z),
        4 - 1 - 1,
        "Body Flare's second point, then the 1"
    );
}

// Lucinde, Alastor (G7): "this turn, Camarilla vampires you control can
// enter combat with the chosen vampire" — owner (2026-10-15, rules-qa
// 74.2): "triggered effects do not disappear with their source": Lucinde
// burned after choosing, the grant still stands this turn.
#[test]
fn lucinde_g7_grant_outlives_her() {
    let mut e = game(2);
    let lucinde = CRYPT
        .iter()
        .position(|c| c.name == "Lucinde, Alastor" && c.group == 7)
        .expect("Lucinde G7") as u16;
    let l = e.add_minion(SeatId(1), lucinde, 4, false);
    let c = e.add_minion(SeatId(1), with_disc("", Some(Sect::Camarilla), 0), 4, false);
    let z = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    pick_when(&mut e, own_use(l, 0, Target::Minion(z)));
    e.state.minions[l.index()].in_play = false;
    let attack = Choice::UseAbility {
        source: AbilitySource::Vampire(l),
        ability: 1,
        actor: Some(c),
        target: Target::Minion(z),
    };
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(req.options.contains(&attack));
    assert_eq!(
        req.options.iter().filter(|&&o| o == attack).count(),
        1,
        "offered once"
    );
    pick(&mut e, attack);
    request(&mut e);
    assert_eq!(e.action().map(|a| a.acting), Some(c), "the action declared");
}
