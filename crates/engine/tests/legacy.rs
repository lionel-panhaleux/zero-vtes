//! Slice 16a: the owner's legacy additions to the pool, cards that make
//! tournament-winning decks (TWDA) playable — appended to the generated
//! tables so the V5 indices stay put. Windows with a single option are
//! elided, so the scripts only stop where a real decision exists.

use zvtes::cards::{self, AbilitySource, Effect, Layer, Scope, Target};
use zvtes::crypt::{CryptCard, Sect, Title, CRYPT};
use zvtes::decision::{Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{
    find_card, find_library, CardId, CardInPlay, Expiry, TimedStat, UncontrolledSlot,
};

mod common;
use common::*;

// ---- Helpers ----------------------------------------------------------------

/// The `nth` vampire with no printed-text definition matching `f`.
fn vampire(f: impl Fn(&CryptCard) -> bool, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| f(c) && cards::vampire(*i as u16).is_none())
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a vampire matching the filter")
}

/// The `nth` untitled vampire of capacity 5 or more, of `clan` (any if
/// empty), with every Discipline of `tags` (lower case: any level).
fn plain(clan: &str, tags: &[&str], nth: usize) -> u16 {
    vampire(
        |c| {
            (clan.is_empty() || c.clan == clan)
                && tags.iter().all(|t| {
                    c.disciplines
                        .iter()
                        .any(|d| d.eq_ignore_ascii_case(t) && (t.to_lowercase() == *t || d == t))
                })
                && c.title == Title::None
                && c.capacity >= 5
        },
        nth,
    )
}

fn play_at(card: u8, mode: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor,
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

fn pool(e: &Engine, seat: u8) -> i32 {
    e.state.seats[seat as usize].pool
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

fn uncontrolled(e: &mut Engine, seat: u8, card: u16) -> u8 {
    let region = &mut e.state.seats[seat as usize].uncontrolled;
    region.push(UncontrolledSlot {
        card,
        copy: Default::default(),
        counters: 0,
        banished: None,
    });
    region.len() as u8 - 1
}

// ---- Ennoia's Theater --------------------------------------------------------

// Ennoia's Theater: "Unique location. Lock to get +1 hand size this turn.
// Lock during your influence phase to get +1 transfer." Gangrel or Gangrel
// antitribu: one clan is enough (ANK 20190203); the transfer at any moment
// of the phase, after the others are spent too (RTR 20180303).
// Rulings: [ANK 20190203]; [RTR 20180303].
#[test]
fn ennoias_theater_locks_for_a_transfer_or_a_hand_size() {
    // No Gangrel: never offered.
    let mut e = Engine::bare(3);
    e.state.turn = 4;
    e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", &[], 1), 4, false);
    hand(&mut e, 0, &["Ennoia's Theater"]);
    e.at_master_phase();
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, master_play(0));

    let mut e = Engine::bare(3);
    e.state.turn = 4;
    e.add_minion(SeatId(0), plain("Gangrel", &[], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    let slot = uncontrolled(&mut e, 0, plain("Ventrue", &[], 0));
    hand(&mut e, 0, &["Ennoia's Theater"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    // The four transfers spent, then the lock: one more.
    to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    let theater = card_named(&e, "Ennoia's Theater");
    for _ in 0..4 {
        pick(&mut e, Choice::Transfer { slot });
    }
    let req = expect(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    assert!(!req.options.contains(&Choice::Transfer { slot }));
    pick(&mut e, ability(theater.0, 1));
    // Locked: no hand size this turn.
    let req = expect(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    assert_eq!(req.options, vec![Choice::Pass, Choice::Transfer { slot }]);
    pick(&mut e, Choice::Transfer { slot });
    assert_eq!(e.state.seats[0].uncontrolled[slot as usize].counters, 5);

    // The other lock, untimed: +1 hand size this turn.
    let mut e = Engine::bare(3);
    e.state.turn = 4;
    e.add_minion(SeatId(0), plain("Gangrel", &[], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    let theater = in_play(&mut e, 0, "Ennoia's Theater", None);
    pick(&mut e, ability(theater.0, 0));
    assert_eq!(
        e.state.seats[0]
            .timed
            .of(TimedStat::HandSize, Expiry::EndOfTurn),
        1
    );
    assert!(e.state.cards[theater.index()].locked);
}

// ---- Donnybrook ----------------------------------------------------------------

/// Seat 0's `a` (with `tags`, made Anarch) and a second vampire; seat 1's
/// `z`.
fn anarch_blocked(tags: &[&str]) -> (Engine, MinionId, MinionId) {
    let mut e = Engine::bare(2);
    e.state.turn = 4;
    let a = e.add_minion(
        SeatId(0),
        vampire(|c| c.capacity >= 5 && tags.iter().all(|t| has(c, t)), 0),
        4,
        false,
    );
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", &[], 0), 4, false);
    (e, a, z)
}

// Donnybrook: "Requires an Anarch. Only usable at close range. [obf]
// Strike: burn equipment or retainer with first strike. [pro] Strike: hand
// strike, aggravated. [ser] Strike: steal 2 blood or life (becoming
// blood)." Played, it requires the Discipline used (LSJ 20011204-3); [ser]
// may target a minion with less than 2 (RTR 20010711). With first strike
// the weapon burns before its strike resolves (rulebook "First Strike").
// Rulings: [RTR 20010711]; [LSJ 20011204-3] [PIB 20130704].
#[test]
fn donnybrook_burns_a_weapon_first_strikes_aggravated_or_steals() {
    // [obf]: the blocker's weapon burned before it strikes.
    let mut e = Engine::bare(2);
    e.state.turn = 4;
    // A vampire with Obfuscate, neither Protean nor Serpentis, made
    // Anarch.
    let a = e.add_minion(
        SeatId(0),
        vampire(
            |c| c.capacity >= 5 && has(c, "obf") && !has(c, "pro") && !has(c, "ser"),
            0,
        ),
        4,
        false,
    );
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", &[], 0), 4, false);
    let bow = in_play(&mut e, 1, "Ivory Bow", Some(z));
    hand(&mut e, 0, &["Donnybrook"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(z));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    let burn = play_at(0, 0, Some(a), Target::Card(bow));
    assert!(req.options.contains(&burn));
    // Only the Discipline it has: no [pro] nor [ser] mode.
    assert!(!plays(&req)
        .iter()
        .any(|p| matches!(p, Choice::Play { mode: 1 | 2, .. })));
    pick(&mut e, burn);
    let req = expect(&mut e, SeatId(1), WindowLabel::CombatStrike);
    let shoot = use_at(bow, 0, None, Target::None);
    assert!(req.options.contains(&shoot), "{:?}", req.options);
    pick(&mut e, shoot);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[bow.index()].in_play, "burned");
    assert_eq!(blood(&e, a), 4, "the bow's strike never resolved");

    // [pro]: an aggravated hand strike.
    let (mut e, a, z) = anarch_blocked(&["pro"]);
    hand(&mut e, 0, &["Donnybrook"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(z));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    let hit = play_at(0, 1, Some(a), Target::None);
    assert_eq!(
        plays(&req),
        vec![hit],
        "the aggravated hand strike only: no card to burn"
    );
    e.state.minions[z.index()].blood = 0;
    pick(&mut e, hit);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor);

    // [ser]: 2 blood stolen (Nadima, made Anarch, prints Serpentis).
    let mut e = Engine::bare(2);
    e.state.turn = 4;
    let nadima = CRYPT
        .iter()
        .position(|c| c.name == "Nadima")
        .expect("Nadima") as u16;
    let a = e.add_minion(SeatId(0), nadima, 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", &[], 0), 4, false);
    hand(&mut e, 0, &["Donnybrook"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(z));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    let steal = play_at(0, 2, Some(a), Target::None);
    assert!(req.options.contains(&steal), "{:?}", req.options);
    pick(&mut e, steal);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 2, "2 blood stolen");
}

// First strike and a dodge: the dodge protects from it (rulebook "First
// Strike"), so the weapon stays.
#[test]
fn a_dodge_protects_from_a_first_strike() {
    let (mut e, a, _) = anarch_blocked(&["obf"]);
    let z = e.add_minion(
        SeatId(1),
        vampire(|c| c.capacity >= 5 && c.disciplines.contains(&"OBL"), 0),
        4,
        false,
    );
    let bow = in_play(&mut e, 1, "Ivory Bow", Some(z));
    hand(&mut e, 0, &["Donnybrook"]);
    hand(&mut e, 1, &["Arms of Ahriman"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(z));
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Card(bow)));
    // Arms of Ahriman [OBL]: strike: dodge.
    pick_when(&mut e, play_at(0, 1, Some(z), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[bow.index()].in_play, "dodged: the bow stays");
}

// ---- Hell-for-Leather ----------------------------------------------------------

// Hell-for-Leather: "Requires an Anarch. Only one Hell-for-Leather can be
// played at a given Discipline each combat. [ani] Strike: dodge, with 1
// additional strike (limited). [cel] Additional strike that does not
// count against the limit. [obt] Only usable if this Anarch is ready and
// the opposing minion is not. The opposing minion's controller burns 2
// pool." [cel] before or after a limited additional strike (LSJ
// 20001206); played, it requires the Discipline used (LSJ 20011204-3).
// Rulings: [LSJ 20001206] [ANK 20211124]; [LSJ 20011204-3] [PIB 20130704].
#[test]
fn hell_for_leather_dodges_strikes_again_and_burns_pool_once_each_discipline() {
    // An Anarch with Animalism and Celerity (made Anarch).
    let mut e = Engine::bare(2);
    e.state.turn = 4;
    let a = e.add_minion(
        SeatId(0),
        vampire(|c| has(c, "ani") && has(c, "cel"), 0),
        4,
        false,
    );
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", &[], 0), 4, false);
    hand(
        &mut e,
        0,
        &["Hell-for-Leather", "Hell-for-Leather", "Hell-for-Leather"],
    );
    pick(&mut e, bleed(a));
    pick(&mut e, block(z));
    // [ani]: dodge, with a limited additional strike.
    pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
    // The additional strike step: [cel] (not against the limit) offered;
    // a second [ani] is not (one per Discipline each combat).
    let cel = play_at(0, 1, Some(a), Target::None);
    let req = offering(&mut e, cel);
    assert!(!plays(&req)
        .iter()
        .any(|p| matches!(p, Choice::Play { mode: 0, .. })));
    pick(&mut e, cel);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4, "dodged");
    assert_eq!(blood(&e, z), 4 - 2, "two additional hand strikes");

    // [obt]: once the blocker is not ready (to torpor this round), its
    // controller burns 2 pool; only if this Anarch is ready.
    let mut e = Engine::bare(2);
    e.state.turn = 4;
    // Alicia Cortez's text (directed actions at her Methuselah) is inert
    // in this combat.
    let a = e.add_minion(SeatId(0), find_card("Alicia Cortez"), 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", &[], 0), 0, false);
    hand(&mut e, 0, &["Hell-for-Leather"]);
    let obt = play_at(0, 2, Some(a), Target::None);
    pick(&mut e, bleed(a));
    pick(&mut e, block(z));
    // Not while both are ready: first offered at the end of the round,
    // the blocker sent to torpor by the hand strike.
    let req = offering(&mut e, obt);
    assert_eq!(req.window, WindowLabel::CombatEndOfRound);
    assert!(e.state.minions[z.index()].in_torpor);
    pick(&mut e, obt);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 30 - 2);
}

// Hell-for-Leather [obt]: "Only usable if this Anarch is ready and the
// opposing minion is not": a blocking ally burned by the round's hand strike
// is not ready (rules-qa 44.1, "sent to torpor or burned"; 75.3), its
// controller burns 2 pool.
#[test]
fn hell_for_leather_obt_burns_pool_against_a_burned_minion() {
    let mut e = Engine::bare(2);
    e.state.turn = 4;
    let a = e.add_minion(SeatId(0), find_card("Alicia Cortez"), 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    let z = e.add_ally(SeatId(1), find_library("Bone Shambler"));
    e.add_minion(SeatId(1), plain("Ventrue", &[], 0), 4, false);
    hand(&mut e, 0, &["Hell-for-Leather"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(z));
    pick_when(&mut e, play_at(0, 2, Some(a), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].in_play, "burned");
    assert_eq!(pool(&e, 1), 30 - 2);
}

fn has(c: &CryptCard, tag: &str) -> bool {
    c.disciplines.iter().any(|d| d.eq_ignore_ascii_case(tag))
}

// ---- Murmur of the False Will ---------------------------------------------------

// Murmur of the False Will: "[dom] [ACTION MODIFIER] Only usable during a
// bleed action. +1 bleed (limited). [DOM] [REACTION] Only usable if a
// younger vampire is bleeding you, after blocks are declined. Lock this
// reacting vampire. Change the target of the bleed to another Methuselah
// other than the acting vampire's controller." As the other redirects:
// after blocks are declined, blocks failed on stealth included (ANK
// 20211003), before "would be successful" cards (LSJ 19980105), to a valid
// target (PIB 20130711); not before all blocks are declined (LSJ
// 20061213); the new target may block (LSJ 20061211).
// Rulings: [ANK 20211003]; [LSJ 19980105]; [PIB 20130711]; [LSJ 20061213]; [LSJ 20061211].
#[test]
fn murmur_of_the_false_will_bleeds_or_redirects_a_younger_bleeder() {
    // [dom]: +1 bleed.
    let mut e = Engine::bare(3);
    e.state.turn = 4;
    let a = e.add_minion(
        SeatId(0),
        vampire(|c| c.capacity >= 5 && has(c, "dom"), 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    hand(&mut e, 0, &["Murmur of the False Will"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 30 - 2);

    // [DOM]: only against a younger bleeder, after blocks are declined.
    for young in [true, false] {
        let mut e = Engine::bare(3);
        e.state.turn = 4;
        // Hel-Blá's text (a printed action of its own) is inert as it
        // bleeds.
        let acting = if young {
            vampire(|c| c.capacity == 4 && c.title == Title::None, 0)
        } else {
            find_card("Hel-Blá")
        };
        let a = e.add_minion(SeatId(0), acting, 3, false);
        e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
        let r = e.add_minion(
            SeatId(1),
            vampire(
                |c| (5..=8).contains(&c.capacity) && c.disciplines.contains(&"DOM"),
                0,
            ),
            4,
            false,
        );
        e.add_minion(SeatId(2), plain("Ventrue", &[], 0), 4, false);
        hand(&mut e, 1, &["Murmur of the False Will"]);
        pick(&mut e, bleed(a));
        let redirect = play_at(0, 1, Some(r), Target::Seat(SeatId(2)));
        if young {
            let req = offering(&mut e, redirect);
            assert_eq!(req.window, WindowLabel::ActionBlocksDeclined);
            // Only to seat 2: not the acting Methuselah, nor oneself.
            assert_eq!(plays(&req), vec![redirect]);
            pick(&mut e, redirect);
            // Seat 2 may block the redirected bleed.
            let req = expect(&mut e, SeatId(2), WindowLabel::ActionBlock);
            assert!(req.options.len() > 1);
            pick(&mut e, Choice::Pass);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!((pool(&e, 1), pool(&e, 2)), (30, 29));
            assert!(e.state.minions[r.index()].locked);
        } else {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, redirect);
        }
    }
}

// ---- Saatet-ta ----------------------------------------------------------------

// Saatet-ta: "During an action, Saatet-ta can lock to give a Follower of Set
// you control +1 stealth, +1 intercept, or +1 bleed. If she is burned,
// shuffle her into her owner's library." The bleed only during a bleed
// action, before it resolves (ANK 20231124, LSJ 19970718); usable the turn
// she is recruited (ANK 20180517); shuffled when burned, burn effects
// still applying (ANK 20200813); not when discarded (RTR 20010711).
// Rulings: [ANK 20231124] [LSJ 19970718]; [ANK 20180517]; [ANK 20200813]; [RTR 20010711].
#[test]
fn saatet_ta_locks_for_a_follower_and_shuffles_back_when_burned() {
    let mut e = Engine::bare(3);
    e.state.turn = 4;
    let a = e.add_minion(SeatId(0), plain("Ministry", &[], 0), 4, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    // Keeps seat 0's minion phase a real decision.
    e.add_minion(SeatId(0), plain("Brujah", &[], 1), 4, false);
    let s = e.add_ally(SeatId(0), find_library("Saatet-ta"));
    // As if recruited this turn: still usable.
    e.state.minions[s.index()].recruited = true;
    let card = match e.state.minions[s.index()].kind {
        zvtes::state::MinionKind::Ally { id, .. } => id,
        _ => unreachable!(),
    };
    // Not for a non-Follower's bleed.
    pick(&mut e, bleed(other));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == card),
    );
    // The Follower's bleed: +1.
    pick(&mut e, bleed(a));
    pick_when(&mut e, use_at(card, 2, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[s.index()].locked);
    assert_eq!(pool(&e, 1), 30 - 1 - 2);
    // Burned: into the library, not the ash heap.
    let library = e.state.seats[0].library.len();
    e.burn(s);
    assert_eq!(e.state.seats[0].library.len(), library + 1);
    assert!(e.state.seats[0]
        .library
        .contains(&find_library("Saatet-ta")));
    assert!(!e.state.seats[0].ash.contains(&find_library("Saatet-ta")));
}

// ---- Carlton Van Wyk ------------------------------------------------------------

// Carlton Van Wyk: "Carlton can strike: 1R damage. Once each combat, he can
// strike: dodge. Carlton gets +1 intercept against vampires. During your
// discard phase, you can burn Carlton to burn a vampire who has committed
// diablerie since your last turn." Since the end of your last turn, the
// current one included (LSJ 20020429); the turn he is recruited too (ANK
// 20180517); Abactor is no diablerie (LSJ 20090724: not in the pool).
// Rulings: [LSJ 20020429]; [ANK 20180517].
#[test]
fn carlton_van_wyk_burns_a_recent_diablerist() {
    let mut e = Engine::bare(3);
    e.state.turn = 4;
    e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    let carlton = e.add_ally(SeatId(0), find_library("Carlton Van Wyk"));
    e.state.minions[carlton.index()].recruited = true;
    let recent = e.add_minion(SeatId(1), plain("Ventrue", &[], 0), 4, false);
    let old = e.add_minion(SeatId(1), plain("Ventrue", &[], 1), 4, false);
    let this = e.state.seats[0].this_turn;
    e.state.minions[recent.index()].diablerie_turn = Some(this);
    e.state.minions[old.index()].diablerie_turn = Some(e.state.seats[0].previous_turn);
    let card = match e.state.minions[carlton.index()].kind {
        zvtes::state::MinionKind::Ally { id, .. } => id,
        _ => unreachable!(),
    };
    let burn = |t| use_at(card, 2, None, Target::Minion(t));
    let req = to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert!(req.options.contains(&burn(recent)));
    assert!(
        !req.options.contains(&burn(old)),
        "before your last turn ended"
    );
    pick(&mut e, burn(recent));
    request(&mut e);
    assert!(!e.state.minions[recent.index()].in_play);
    assert!(!e.state.minions[carlton.index()].in_play);
}

// Carlton's +1 intercept against vampires: enough to block a vampire's
// hunt (+1 stealth), not an ally's action.
#[test]
fn carlton_van_wyk_intercepts_vampires() {
    let mut e = Engine::bare(3);
    e.state.turn = 4;
    // A vampire with no blood: its hunt (+1 stealth) is taken at once.
    e.add_minion(SeatId(0), plain("Brujah", &[], 0), 0, false);
    e.add_minion(SeatId(0), plain("Brujah", &[], 1), 4, false);
    let carlton = e.add_ally(SeatId(2), find_library("Carlton Van Wyk"));
    let req = expect(&mut e, SeatId(2), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(carlton)));
}

// ---- Jake Washington --------------------------------------------------------------

// Jake Washington: "Unique. Put this card in play; it becomes a unique
// mortal ally with 1 life, 0 strength and 0 bleed. During your unlock
// phase, you can burn Jake to add up to 4 blood to a ready vampire with no
// blood." Not an ally in the ash heap (LSJ 20071014); his starting life
// is 1 (LSJ 20030520-2).
// Rulings: [LSJ 20071014] [LSJ 20020115] [LSJ 20030128]; [LSJ 20030520-2].
#[test]
fn jake_washington_becomes_an_ally_and_burns_to_give_blood() {
    let mut e = Engine::bare(2);
    e.state.turn = 4;
    let empty = e.add_minion(SeatId(0), plain("Brujah", &[], 0), 0, false);
    let full = e.add_minion(SeatId(0), plain("Brujah", &[], 1), 2, false);
    hand(&mut e, 0, &["Jake Washington"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    request(&mut e);
    let jake = card_named(&e, "Jake Washington");
    let m = e.state.cards[jake.index()].body.expect("an ally");
    assert_eq!(blood(&e, m), 1, "1 life");
    // Vagabond Mystic cannot give him more life: at his starting life.
    assert_eq!(e.state.ally_body(m).map(|b| b.life), Some(1));
    // A vampire with no blood hunts this turn: emptied again before the
    // next unlock phase, where Jake burns to add up to 4 to it.
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[empty.index()].blood = 0;
    let give = use_at(jake, 0, None, Target::Minion(empty));
    let req = offering(&mut e, give);
    assert_eq!(req.window, WindowLabel::UnlockAbilities);
    assert!(!req
        .options
        .contains(&use_at(jake, 0, None, Target::Minion(full))));
    pick(&mut e, give);
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(req.options, (1..=4).map(Choice::SetX).collect::<Vec<_>>());
    pick(&mut e, Choice::SetX(4));
    request(&mut e);
    assert_eq!(blood(&e, empty), 4);
    assert!(!e.state.minions[m.index()].in_play);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Jake Washington")));
    // No ally in the ash heap.
    assert!(!cards::ash_ally(find_library("Jake Washington"), &[]));
}

// ---- Gilbert Duane --------------------------------------------------------------

// Gilbert Duane: "Anarch Baron of Miami: During your influence phase,
// Gilbert can burn 1 blood to reveal and remove the top card of your crypt
// from the game. If it is a Malkavian, a non-titled vampire you control
// becomes Anarch." (No ruling.)
#[test]
fn gilbert_duane_removes_his_crypts_top_card_and_makes_an_anarch() {
    for malkavian in [true, false] {
        let mut e = Engine::bare(2);
        e.state.turn = 4;
        let g = e.add_minion(SeatId(0), find_card("Gilbert Duane"), 5, false);
        let v = e.add_minion(SeatId(0), plain("Ventrue", &[], 0), 4, false);
        let w = e.add_minion(SeatId(0), plain("Ventrue", &[], 1), 4, false);
        let top = plain(if malkavian { "Malkavian" } else { "Brujah" }, &[], 0);
        e.state.seats[0].crypt = vec![top].into();
        let use_it = Choice::UseAbility {
            source: AbilitySource::Vampire(g),
            ability: 0,
            actor: None,
            target: Target::None,
        };
        let req = to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
        assert!(req.options.contains(&use_it));
        pick(&mut e, use_it);
        if malkavian {
            // Gilbert himself is titled: the other vampire is the pick.
            let req = request(&mut e);
            assert_eq!(
                req.options,
                vec![Choice::ChooseMinion(v), Choice::ChooseMinion(w)]
            );
            e.choose(0);
            request(&mut e);
            assert_eq!(e.state.sect_of(v), Some(Sect::Anarch));
        } else {
            request(&mut e);
            assert_eq!(e.state.sect_of(v), Some(Sect::Camarilla));
        }
        assert_eq!(blood(&e, g), 4);
        assert!(e.state.seats[0].crypt.is_empty());
        assert_eq!(e.state.seats[0].removed_crypt, vec![top]);
    }
}

// ---- Powerbase: Savannah ----------------------------------------------------------

// Powerbase: Savannah: "You may lock any other unique location you control
// to end combat involving an acting vampire you control before range is
// chosen." The acting vampire is the one considered acting, Deep Song's
// target included (ANK 20230814).
// Rulings: [ANK 20230814].
#[test]
fn powerbase_savannah_locks_a_location_to_end_its_acting_vampires_combat() {
    let mut e = Engine::bare(2);
    e.state.turn = 4;
    let a = e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", &[], 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", &[], 0), 4, false);
    let savannah = in_play(&mut e, 0, "Powerbase: Savannah", None);
    let library = in_play(&mut e, 0, "Arcane Library", None);
    // The blocker's side has one too: not its acting vampire, not offered.
    let theirs = in_play(&mut e, 1, "Powerbase: Savannah", None);
    in_play(&mut e, 1, "Art Museum", None);
    pick(&mut e, bleed(a));
    pick(&mut e, block(z));
    let end = use_at(savannah, 0, None, Target::Card(library));
    let req = offering(&mut e, end);
    assert_eq!(req.window, WindowLabel::CombatBeforeRange);
    // Not itself.
    assert!(!req
        .options
        .contains(&use_at(savannah, 0, None, Target::Card(savannah))));
    pick(&mut e, end);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: 0, .. }
            if *c == theirs)
    });
    assert_eq!((blood(&e, a), blood(&e, z)), (4, 4), "no strikes");
    assert!(e.state.cards[library.index()].locked);
}

// ---- Ashur Tablets ----------------------------------------------------------------

// Ashur Tablets: "Only one Ashur Tablets can be played each turn. Put this
// card in play. If you control three copies of this card, remove all
// copies in play (even controlled by other Methuselahs) from the game to
// gain 3 pool and choose up to thirteen library cards from your ash heap;
// move one of the chosen cards to your hand and shuffle the others into
// your library." Playable with an empty ash heap (PIB 20130119); the cards
// chosen as the third copy enters play (LSJ 20091030); the hand card shown
// (LSJ 20081129: public, a hand move).
// Rulings: [PIB 20130119]; [LSJ 20091030]; [LSJ 20081129].
#[test]
fn ashur_tablets_third_copy_recovers_the_ash_heap() {
    let mut e = Engine::bare(2);
    e.state.turn = 4;
    e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", &[], 1), 4, false);
    in_play(&mut e, 0, "Ashur Tablets", None);
    in_play(&mut e, 0, "Ashur Tablets", None);
    in_play(&mut e, 1, "Ashur Tablets", None);
    let (bonding, conditioning) = (find_library("Bonding"), find_library("Conditioning"));
    e.state.seats[0].ash = vec![bonding, bonding, conditioning].into();
    hand(&mut e, 0, &["Ashur Tablets", "Ashur Tablets"]);
    e.state.seats[0].mpa = 2;
    e.at_master_phase();
    pick(&mut e, master_play(0));
    // The picks: the hand card first.
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            Choice::FindCard(bonding),
            Choice::FindCard(conditioning)
        ]
    );
    e.choose(index_of(&req, Choice::FindCard(conditioning)));
    pick(&mut e, Choice::FindCard(bonding));
    pick(&mut e, Choice::Pass);
    let req = request(&mut e);
    // All copies removed, 3 pool; the second Tablets not playable this turn.
    let tablets = find_library("Ashur Tablets");
    assert!(!e.state.cards.iter().any(|c| c.in_play && c.card == tablets));
    assert_eq!(
        e.state.seats[0]
            .removed
            .iter()
            .filter(|&&c| c == tablets)
            .count(),
        3
    );
    assert_eq!(e.state.seats[1].removed, vec![tablets]);
    assert_eq!(pool(&e, 0), 33);
    assert!(e.state.seats[0].hand.contains(&conditioning));
    assert_eq!(e.state.seats[0].ash, vec![bonding]);
    // Shuffled into the library (the Tablets' replacement may draw it).
    let seat = &e.state.seats[0];
    assert!(seat.library.contains(&bonding) || seat.hand.contains(&bonding));
    assert!(!plays(&req).iter().any(|p| matches!(p, Choice::Play { .. })));
}

// With an empty ash heap it is still played (PIB 20130119): no pick.
#[test]
fn ashur_tablets_plays_with_an_empty_ash_heap() {
    let mut e = Engine::bare(2);
    e.state.turn = 4;
    e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", &[], 1), 4, false);
    in_play(&mut e, 0, "Ashur Tablets", None);
    in_play(&mut e, 0, "Ashur Tablets", None);
    hand(&mut e, 0, &["Ashur Tablets"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 0), 33);
}

// ---- Rötschreck -------------------------------------------------------------------

/// Seat 0's `a` (with an Ivory Bow: 1R aggravated) bleeds, blocked by
/// seat 1's `z`, who holds Rötschreck.
fn bow_bleed() -> (Engine, MinionId, MinionId, CardId) {
    bow_bleed_with(plain("Brujah", &[], 0), plain("Ventrue", &[], 0))
}

fn bow_bleed_with(acting: u16, blocker: u16) -> (Engine, MinionId, MinionId, CardId) {
    let mut e = Engine::bare(2);
    e.state.turn = 4;
    let a = e.add_minion(SeatId(0), acting, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", &[], 1), 4, false);
    let z = e.add_minion(SeatId(1), blocker, 4, false);
    // Keeps seat 1's minion phase a real decision.
    e.add_minion(SeatId(1), plain("Ventrue", &[], 1), 4, false);
    let bow = in_play(&mut e, 0, "Ivory Bow", Some(a));
    hand(&mut e, 1, &["Rötschreck"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(z));
    (e, a, z, bow)
}

// Rötschreck: "Put this card on a vampire when an opposing minion attempts
// to inflict aggravated damage on him or her, whether the damage would be
// successfully inflicted or not. Combat ends. This vampire is locked and
// sent to torpor. This vampire does not unlock as normal. During this
// vampire's next unlock phase, burn this card." On the vampire facing it
// (RTR 19980623), after strikes are declared (LSJ 19990217), dodged or not
// (LSJ 20011005); nothing resolves, no prevention (LSJ 19990217); both
// still ready as combat ends (LSJ 20070907); the unlock ban ends with the
// card (ANK 20191025); Soul Burn or Blood Fury does not stop it (ANK
// 20170519: the weapon damage is nullified only as it resolves).
// Rulings: [RTR 19980623]; [LSJ 19990217] [RTR 20041202] [LSJ 20100119]; [LSJ 20011005] [ANK 20211102]; [LSJ 19990217] [LSJ 20020715] [LSJ 20060803]; [LSJ 20070907] [LSJ 20020402]; [ANK 20191025]; [ANK 20170519].
#[test]
fn rotschreck_ends_combat_against_an_aggravated_strike() {
    let (mut e, a, z, bow) = bow_bleed();
    let rot = play_at(0, 0, None, Target::Minion(z));
    // Not before both strikes are chosen.
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(!req.options.contains(&rot));
    pick(&mut e, use_at(bow, 0, None, Target::None));
    // z dodges: still playable.
    let req = offering(&mut e, rot);
    assert_eq!(req.window, WindowLabel::CombatBeforeResolution);
    assert_eq!(req.seat, SeatId(1));
    pick(&mut e, rot);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Nothing resolved: no damage either way; z locked, in torpor, the
    // card on it.
    assert_eq!(blood(&e, a), 4);
    assert_eq!(blood(&e, z), 4);
    assert!(e.state.minions[z.index()].in_torpor);
    assert!(e.state.minions[z.index()].locked);
    let card = card_named(&e, "Rötschreck");
    assert_eq!(e.state.cards[card.index()].host, Some(z));
    // Out of turn: counted against seat 1's next master phase.
    assert!(e.state.seats[1].out_of_turn.is_some());
    // Seat 1's unlock phase: z does not unlock; the card burns.
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].locked);
    assert!(!e.state.cards[card.index()].in_play);
}

// Not against a strike that inflicts no aggravated damage (a hand strike),
// nor against non-strike damage (ANK 20220114), nor at a range the damage
// is not effective at (RTR 19980928).
// Rulings: [ANK 20220114]; [RTR 19980928] [ANK 20211102]; [ANK 20200130-1].
#[test]
fn rotschreck_needs_an_effective_aggravated_strike() {
    let (mut e, _, z, _) = bow_bleed();
    let rot = play_at(0, 0, None, Target::Minion(z));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, rot);
}

// Rötschreck under Dawn Operation ("all damage inflicted on vampires during
// the resulting combat is aggravated"): it answers a hand strike, not
// Psychic Assault ("any effect which would make damage from this strike
// aggravated leaves the damage normal instead"), aggravated at its source
// (ANK 20200130-1).
#[test]
fn rotschreck_under_dawn_operation_answers_hands_not_psychic_assault() {
    for assault in [false, true] {
        let mut e = Engine::bare(2);
        e.state.turn = 4;
        let a = e.add_minion(SeatId(0), plain("", &["FOR", "aus"], 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", &[], 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", &[], 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", &[], 1), 4, false);
        let cards: &[&str] = if assault {
            &["Dawn Operation", "Psychic Assault"]
        } else {
            &["Dawn Operation"]
        };
        hand(&mut e, 0, cards);
        hand(&mut e, 1, &["Rötschreck"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, play(0, 1, a)); // Dawn Operation [FOR]
        pick_when(&mut e, block(z));
        if assault {
            pick_when(&mut e, play(0, 0, a)); // Psychic Assault [aus]
        }
        let rot = play_at(0, 0, None, Target::Minion(z));
        let offered = offered_before(&mut e, rot, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(offered, !assault, "Psychic Assault: {assault}");
    }
}

// Rötschreck after an opposing weapon strike and a Blood Fury: the weapon
// damage is nullified only as the strikes resolve, so the aggravated
// strike still triggers it (ANK 20170519).
// Rulings: [ANK 20170519].
#[test]
fn rotschreck_answers_a_weapon_strike_blood_fury_would_nullify() {
    let (mut e, _, z, bow) = bow_bleed_with(
        plain("Brujah", &[], 0),
        vampire(|c| c.capacity >= 5 && has(c, "tha"), 0),
    );
    hand(&mut e, 1, &["Blood Fury"]);
    pick_when(&mut e, use_at(bow, 0, None, Target::None));
    pick_when(&mut e, play_at(1, 0, Some(z), Target::None));
    let rot = play_at(0, 0, None, Target::Minion(z));
    let req = offering(&mut e, rot);
    assert_eq!(req.window, WindowLabel::CombatBeforeResolution);
    pick(&mut e, rot);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor);
}

// A combat continued after Rötschreck ended it (Telepathic Tracking
// [AUS]): the card stays on the vampire, who is not sent to torpor, and
// still does not unlock as normal (RTR 20020501).
// Rulings: [RTR 20020501] [LSJ 20071009].
#[test]
fn rotschreck_in_a_continued_combat_sends_nobody_to_torpor() {
    let (mut e, a, z, bow) = bow_bleed_with(
        vampire(|c| c.capacity >= 5 && c.disciplines.contains(&"AUS"), 0),
        plain("Ventrue", &[], 0),
    );
    hand(&mut e, 0, &["Telepathic Tracking"]);
    pick_when(&mut e, use_at(bow, 0, None, Target::None));
    pick_when(&mut e, play_at(0, 0, None, Target::Minion(z)));
    // Combat would end: a new round instead.
    pick_when(&mut e, play_at(0, 1, Some(a), Target::None));
    // The new round's strikes (its earlier windows empty, elided).
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::CombatStrike);
    // (Locked already by its block.)
    assert!(!e.state.minions[z.index()].in_torpor);
    let card = card_named(&e, "Rötschreck");
    assert_eq!(e.state.cards[card.index()].host, Some(z));
    assert!(e.state.does_not_unlock(z));
}

// ---- Life Boon ----------------------------------------------------------------------

fn boon_on(e: &mut Engine, by: u8, target: u8) -> CardId {
    let id = in_play(e, by, "Life Boon", None);
    e.state.cards[id.index()].against = zvtes::ids::SeatSet::single(SeatId(target));
    id
}

fn half(e: &Engine) -> Vec<u32> {
    e.state.seats.iter().map(|s| s.half_vps).collect()
}

/// 3 seats; seat 0 bleeds seat 1 for 3, seat 1 at 1 pool; seat 2 holds a
/// Life Boon.
fn boon_bleed() -> (Engine, MinionId) {
    let mut e = Engine::bare(3);
    e.state.turn = 4;
    let a = e.add_minion(
        SeatId(0),
        vampire(|c| c.capacity >= 5 && c.disciplines.contains(&"DOM"), 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", &[], 1), 4, false);
    e.state.seats[1].pool = 1;
    e.state.seats[2].pool = 10;
    hand(&mut e, 0, &["Conditioning"]);
    hand(&mut e, 2, &["Life Boon"]);
    (e, a)
}

// Life Boon: "Out-of-turn. Boon. Give pool to a Methuselah with no pool to
// keep them in the game, and put this card in play." The pool given must
// make good what they owe (RTR 19970630, ANK 20230817-2): bled 4 at 1
// pool, at least 4 (to 1). Out of turn: counted against the next master
// phase.
// Rulings: [RTR 19970630] [ANK 20230817-2].
#[test]
fn life_boon_keeps_a_methuselah_in_the_game() {
    let (mut e, a) = boon_bleed();
    pick(&mut e, bleed(a));
    // [DOM] Conditioning: +3 bleed, 4 in all against 1 pool.
    pick_when(&mut e, play_at(0, 1, Some(a), Target::None));
    let save = play_at(0, 0, None, Target::Seat(SeatId(1)));
    let req = offering(&mut e, save);
    assert_eq!(req.seat, SeatId(2));
    assert_eq!(req.options, vec![Choice::Pass, save]);
    assert_eq!(pool(&e, 1), 1 - 4);
    pick(&mut e, save);
    let req = expect(&mut e, SeatId(2), WindowLabel::PlayTerms);
    assert_eq!(req.options, (4..=9).map(Choice::SetX).collect::<Vec<_>>());
    pick(&mut e, Choice::SetX(5));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::MinionPhase)
    );
    assert!(!e.state.seats[1].ousted);
    assert_eq!((pool(&e, 1), pool(&e, 2)), (-3 + 5, 10 - 5));
    assert_eq!(half(&e), vec![0, 0, 0], "nobody ousted");
    let boon = card_named(&e, "Life Boon");
    assert!(e.state.cards[boon.index()].against.contains(SeatId(1)));
    assert!(e.state.seats[2].out_of_turn.is_some());
}

// Declined: the oust applies — the predator scores and gains 6 pool.
#[test]
fn a_declined_life_boon_lets_the_oust_apply() {
    let (mut e, a) = boon_bleed();
    pick(&mut e, bleed(a));
    pick_when(&mut e, play_at(0, 1, Some(a), Target::None));
    let req = offering(&mut e, play_at(0, 0, None, Target::Seat(SeatId(1))));
    assert_eq!(req.seat, SeatId(2));
    pick(&mut e, Choice::Pass);
    request(&mut e);
    assert!(e.state.seats[1].ousted);
    assert_eq!(half(&e), vec![2, 0, 0]);
    assert!(e.state.seats[2].hand.contains(&find_library("Life Boon")));
}

// No Boon on one's own turn (out-of-turn), none without the pool to give
// (1 kept): the oust applies at once, no decision.
#[test]
fn life_boon_needs_another_turn_and_the_pool_to_give() {
    let (mut e, a) = boon_bleed();
    e.state.seats[2].pool = 4; // at most 3 to give: not the 4 owed
    pick(&mut e, bleed(a));
    pick_when(&mut e, play_at(0, 1, Some(a), Target::None));
    let req = request(&mut e);
    assert!(e.state.seats[1].ousted);
    assert_eq!(req.window, WindowLabel::MinionPhase);
}

// The Boon player gains the victory points the target gains, a series of
// Boons one after another (RTR 19951017), half points too (RTR 19991206),
// burned after 1 VP; the target still gains the 6 pool.
// Rulings: [RTR 19951017]; [RTR 19991206].
#[test]
fn life_boons_take_the_targets_victory_points_in_series() {
    let mut e = Engine::bare(4);
    e.state.turn = 4;
    let a = e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", &[], 1), 4, false);
    e.state.seats[1].pool = 1;
    let first = boon_on(&mut e, 2, 0);
    let second = boon_on(&mut e, 3, 0);
    pick(&mut e, bleed(a));
    request(&mut e);
    assert!(e.state.seats[1].ousted);
    assert_eq!(half(&e), vec![0, 0, 2, 0], "the first Boon's player");
    assert!(!e.state.cards[first.index()].in_play, "burned at 1 VP");
    assert!(e.state.cards[second.index()].in_play);
    assert_eq!(pool(&e, 0), 30 + 6);
    // The time limit: seat 0's half point goes to the second Boon.
    e.time_limit = e.state.turn;
    until_over(&mut e);
    // (Seats 2 and 3 survive too: their own half points.)
    assert_eq!(half(&e), vec![0, 0, 2 + 1, 1 + 1]);
    assert_eq!(e.state.cards[second.index()].counters, 1);
}

fn until_over(e: &mut Engine) {
    for _ in 0..500 {
        match e.run() {
            zvtes::game::Step::Over(_) => return,
            zvtes::game::Step::Decision(req) => {
                let i = fallback(&req);
                e.choose(i);
            }
        }
    }
    panic!("the game did not end");
}

// A Boon on one's predator: ousted by them, one gains nothing from it —
// the VP is theirs (RTR 20000501, reversing RTR 19990712).
// Rulings: [RTR 20000501] [LSJ 20020620-2].
#[test]
fn a_life_boon_on_your_predator_gives_nothing_as_you_are_ousted() {
    let mut e = Engine::bare(3);
    e.state.turn = 4;
    let a = e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", &[], 1), 4, false);
    e.state.seats[1].pool = 1;
    boon_on(&mut e, 1, 0);
    pick(&mut e, bleed(a));
    request(&mut e);
    assert!(e.state.seats[1].ousted);
    assert_eq!(half(&e), vec![2, 0, 0]);
}

// "Until you are being ousted" (rules-qa 45.5, owner): the Boon player's
// own oust gives it nothing — neither the VP for ousting it nor the last
// Methuselah standing's VP, both the saved Methuselah's.
#[test]
fn a_life_boon_player_ousted_last_gains_no_last_standing_vp() {
    let mut e = Engine::bare(2);
    e.state.turn = 4;
    let a = e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", &[], 1), 4, false);
    e.state.seats[1].pool = 1;
    boon_on(&mut e, 1, 0);
    pick(&mut e, bleed(a));
    until_over(&mut e);
    assert!(e.state.seats[1].ousted);
    assert_eq!(
        half(&e),
        vec![2 + 2, 0],
        "the oust and the last-standing VP"
    );
}

// Both the target and its prey ousted by one effect (Kine Resources
// Contested): the Boon saves the target, whose VP for its prey goes to the
// Boon player; the target keeps the 6 pool (LSJ 20021029).
// Rulings: [LSJ 20021029].
#[test]
fn a_life_boon_saved_target_scores_its_simultaneous_prey_for_the_boon_player() {
    let mut e = Engine::bare(4);
    e.state.turn = 4;
    let alexa = e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    e.add_minion(SeatId(0), find_card("Mateo García"), 2, false);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.state.seats[1].pool = 2;
    e.state.seats[2].pool = 2;
    hand(&mut e, 0, &["Kine Resources Contested"]);
    hand(&mut e, 3, &["Life Boon"]);
    pick(&mut e, play(0, 0, alexa));
    pick(&mut e, alloc(0, 0));
    pick(&mut e, alloc(1, 2));
    pick(&mut e, alloc(2, 2));
    pick_when(&mut e, votes(alexa, true));
    close_polling(&mut e);
    // Seats 1 and 2 at 0: seat 3 may save either; it saves seat 1.
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(3));
    let save = |t| play_at(0, 0, None, Target::Seat(SeatId(t)));
    assert_eq!(req.options, vec![Choice::Pass, save(1), save(2)]);
    e.choose(index_of(&req, save(1)));
    pick(&mut e, Choice::SetX(1));
    request(&mut e);
    assert!(!e.state.seats[1].ousted);
    assert!(e.state.seats[2].ousted);
    assert_eq!(half(&e), vec![0, 0, 0, 2], "seat 1's VP, the Boon player's");
    assert_eq!(pool(&e, 1), 1 + 6);
}

// "During each of their unlock phases, you can steal 1 pool from that
// Methuselah."
#[test]
fn life_boon_steals_a_pool_in_the_targets_unlock_phase() {
    let mut e = Engine::bare(2);
    e.state.turn = 4;
    e.add_minion(SeatId(0), plain("Brujah", &[], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", &[], 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", &[], 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", &[], 1), 4, false);
    let boon = boon_on(&mut e, 1, 0);
    let steal = use_at(boon, 0, None, Target::None);
    // Seat 0's next unlock phase.
    let req = offering(&mut e, steal);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::UnlockAbilities)
    );
    assert_eq!(e.state.active, SeatId(0));
    let (p0, p1) = (pool(&e, 0), pool(&e, 1));
    pick(&mut e, steal);
    request(&mut e);
    assert_eq!((pool(&e, 0), pool(&e, 1)), (p0 - 1, p1 + 1));
}

// Parity Shift's pool can pay for the Life Boon saving the Methuselah it
// takes from (LSJ 20100527): the gain lands before the oust.
// Rulings: [LSJ 20100527].
#[test]
fn parity_shift_pool_pays_for_a_life_boon() {
    let mut e = Engine::bare(3);
    e.state.turn = 4;
    let alexa = e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    e.add_minion(SeatId(0), find_card("Mateo García"), 2, false);
    e.state.seats[0].pool = 1;
    e.state.seats[1].pool = 3;
    e.state.seats[2].pool = 1;
    hand(&mut e, 0, &["Parity Shift"]);
    hand(&mut e, 2, &["Life Boon"]);
    pick(&mut e, play(0, 0, alexa));
    pick(&mut e, alloc(0, 1));
    pick_when(&mut e, votes(alexa, true));
    close_polling(&mut e);
    // Seat 1 at 0; seat 2 now at 3 pool, 1 to give and 2 to keep.
    let save = play_at(0, 0, None, Target::Seat(SeatId(1)));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.options.clone()),
        (SeatId(2), vec![Choice::Pass, save])
    );
    e.choose(1);
    let req = expect(&mut e, SeatId(2), WindowLabel::PlayTerms);
    assert_eq!(req.options, vec![Choice::SetX(1), Choice::SetX(2)]);
    pick(&mut e, Choice::SetX(1));
    request(&mut e);
    assert!(!e.state.seats[1].ousted);
    assert_eq!((pool(&e, 0), pool(&e, 1), pool(&e, 2)), (2, 1, 2));
}
