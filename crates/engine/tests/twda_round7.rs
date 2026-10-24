//! Legacy cards for TWDA decks, round 7 (`ADDED_BATCHES`, the seventeenth
//! batch, slice 16s): the next hundred cards by TWDA importance — the
//! decks of eleven new archetypes. Readings in rules-qa Batch 67. Windows
//! with a single option are elided, so the scripts only stop where a real
//! decision exists.

use zvtes::cards::{self, AbilitySource, Effect, Layer, Scope, Stat, Target};
use zvtes::crypt::{Sect, CRYPT};
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

/// A vampire's ability granting an action: the vampire is its actor.
fn vamp_action(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: Some(m),
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

/// An untitled vampire with no definition of its own, of capacity `cap`,
/// of `sect` if given.
fn of_capacity(cap: u8, sect: Option<Sect>, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.capacity == cap
                && sect.is_none_or(|s| c.sect == Some(s))
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

/// Plays on to `seat`'s next minion phase (fallbacks), choosing every
/// option `want` matches: how many were chosen.
fn next_turn_taking(e: &mut Engine, seat: SeatId, want: impl Fn(&Choice) -> bool) -> usize {
    let start = e.state.turn;
    let mut n = 0;
    loop {
        let req = request(e);
        if req.seat == seat && req.window == WindowLabel::MinionPhase && e.state.turn > start {
            return n;
        }
        if let Some(i) = req.options.iter().position(&want) {
            e.choose(i);
            n += 1;
            continue;
        }
        e.choose(fallback(&req));
    }
}

/// The pool seat 1 loses to `m`'s bleed (seat 0 bleeding its prey).
fn bled(mut e: Engine, m: MinionId) -> i32 {
    pick(&mut e, bleed(m));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    10 - pool(&e)[1]
}

// ---- Step 1: Adonai, Aura Reading, Banshee Ironwail, Cornelius Ottavio,
// ---- Ozmo, Poacher's Hunting Ground, Quira, Sheila Mezarin, Smear Campaign

// Adonai: "can enter combat with any ready Camarilla vampire controlled by
// another Methuselah as a Ⓓ action. He gets +1 strength when in combat
// with a Tremere. +1 bleed." A Camarilla vampire is a target, a Sabbat one
// is not; +1 strength against a Tremere only.
#[test]
fn adonai_rushes_camarilla_vampires_and_hits_tremere_harder() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Adonai"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let cam = e.add_minion(
        SeatId(1),
        of_capacity(4, Some(Sect::Camarilla), 0),
        4,
        false,
    );
    let sab = e.add_minion(SeatId(1), of_capacity(4, Some(Sect::Sabbat), 0), 4, false);
    let tremere = e.add_minion(SeatId(1), plain("Tremere", 0), 4, false);
    let req = request(&mut e);
    assert!(req
        .options
        .contains(&vamp_action(a, 0, Target::Minion(cam))));
    assert!(!req
        .options
        .contains(&vamp_action(a, 0, Target::Minion(sab))));
    let strength = |e: &Engine, o| {
        e.state.printed_mod(
            a,
            Stat::Strength,
            PrintedCtx {
                opponent: Some(o),
                ..PrintedCtx::default()
            },
        )
    };
    assert_eq!((strength(&e, tremere), strength(&e, cam)), (1, 0));
    assert_eq!(bled(e, a), 2);
}

// Ozmo, Quira and Sheila Mezarin: "+1 bleed."
#[test]
fn ozmo_quira_and_sheila_mezarin_bleed_for_two() {
    for name in ["Ozmo", "Quira, The Bitch Queen", "Sheila Mezarin"] {
        let mut e = game(2);
        let v = e.add_minion(SeatId(0), find_card(name), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        assert_eq!(bled(e, v), 2, "{name}");
    }
}

// Cornelius Ottavio: "+1 bleed. Cornelius gets an additional +1 bleed when
// bleeding a Methuselah who controls a ready non-Camarilla vampire." Two
// against a Methuselah of Camarilla vampires only, three once a Sabbat
// vampire is ready there — none while it is in torpor.
#[test]
fn cornelius_ottavio_bleeds_harder_against_non_camarilla() {
    for (sabbat, torpid, want) in [(false, false, 2), (true, false, 3), (true, true, 2)] {
        let mut e = game(2);
        let c = e.add_minion(SeatId(0), find_card("Cornelius Ottavio"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(
            SeatId(1),
            of_capacity(4, Some(Sect::Camarilla), 0),
            4,
            false,
        );
        if sabbat {
            let s = e.add_minion(SeatId(1), of_capacity(4, Some(Sect::Sabbat), 0), 4, false);
            e.state.minions[s.index()].in_torpor = torpid;
        }
        assert_eq!(bled(e, c), want, "sabbat {sabbat}, torpid {torpid}");
    }
}

// Aura Reading: "Only usable before range is chosen. [aus] The opposing
// minion's controller plays with an open hand for the remainder of combat.
// [AUS] You get +2 hand size for the remainder of this combat. A vampire
// can play only one Aura Reading at superior each combat." Seat 1's hand
// opens to seat 2 until the combat ends; seat 0 holds 9 cards for the
// combat, a second [AUS] copy not offered.
#[test]
fn aura_reading_opens_the_hand_and_widens_its_own() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("aus", 0), 6, false);
    superior(&mut e, a, "aus");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Aura Reading", "Aura Reading", "Aura Reading"]);
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert!(e.state.hand_visible_to(SeatId(1), SeatId(2)));
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
    pick_when(&mut e, play(0, 1, a));
    request(&mut e);
    assert_eq!(e.state.hand_size(SeatId(0)), 9);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 1, a)), "one at superior");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.hand_visible_to(SeatId(1), SeatId(2)));
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
}

// Banshee Ironwail: "Strength+2 damage each strike. … Bearer gets -1
// stealth on each action. If bearer blocks an action, the acting minion
// burns 1 blood or life (before combat begins, if any)." [LSJ 19971215]
// "Triggering the burn blood effect does not force the minion to strike
// with the weapon." [ANK 20210627] "If a block is successful, the blood is
// burned before any other effect can be used": the acting vampire is down
// a blood as the combat begins; its bearer may strike with it or by hand.
#[test]
fn banshee_ironwail_bleeds_the_blocked_minion_before_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    let w = in_play(&mut e, 1, "Banshee Ironwail", Some(z));
    assert_eq!(e.state.bearer_mod(z, Stat::Stealth, ActionKind::Bleed), -1);
    blocked_bleed(&mut e, a, z);
    request(&mut e);
    assert_eq!(blood(&e, a), 3);
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(req.options.contains(&use_at(w, 0, None, Target::None)));
    assert!(req.options.contains(&Choice::HandStrike));
}

// [RBK resolve-the-action] "If an acting minion leaves the ready region
// before block resolution, the action ends immediately and the blocking
// minion is not locked." A 1-life ally bleeds into Banshee Ironwail: it
// burns, the action ends, the blocker stays unlocked.
#[test]
fn banshee_ironwail_burns_an_acting_ally_and_leaves_the_blocker_unlocked() {
    let mut e = game(2);
    let ally = e.add_ally(SeatId(0), find_library("Spectral Servitor"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    in_play(&mut e, 1, "Banshee Ironwail", Some(z));
    blocked_bleed(&mut e, ally, z);
    request(&mut e);
    assert!(!e.state.minions[ally.index()].in_play);
    assert!(!e.state.minions[z.index()].locked);
    assert!(e.combat().is_none());
}

// Poacher's Hunting Ground: "choose up to X ready anarchs you control who
// each gain 1 blood, where X is the number of non-derivative hunting
// grounds controlled by other Methuselahs." [ANK 20170430] A Trophy:
// Hunting Ground counts (not in the pool): any non-derivative one does,
// its own copy (derivative) does not. One hunting ground across the
// table: one anarch fed; two: both.
#[test]
fn poachers_hunting_ground_feeds_one_anarch_per_rival_hunting_ground() {
    for rivals in [1usize, 2] {
        let mut e = game(2);
        let x = e.add_minion(SeatId(0), of_capacity(5, Some(Sect::Anarch), 0), 1, false);
        let y = e.add_minion(SeatId(0), of_capacity(5, Some(Sect::Anarch), 1), 1, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let p = in_play(&mut e, 0, "Poacher's Hunting Ground", None);
        for name in ["Academic Hunting Ground", "Park Hunting Ground"]
            .iter()
            .take(rivals)
        {
            in_play(&mut e, 1, name, None);
        }
        let used = next_turn_taking(
            &mut e,
            SeatId(0),
            |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == p),
        );
        assert_eq!(used, rivals);
        assert_eq!(blood(&e, x) + blood(&e, y) - 2, rivals as u8);
    }
}

// Smear Campaign: "Lock during your unlock phase and choose a locked
// non-anarch minion controlled by your predator or prey to burn a card on
// that minion. Cannot be used to burn an equipment or a retainer. That
// minion's controller then takes control of this card." Seat 1's locked
// Ventrue holds an archetype and a weapon: the archetype burns, the weapon
// is no target; seat 1 then controls the locked Smear Campaign.
#[test]
fn smear_campaign_burns_a_card_and_changes_hands() {
    let mut e = game(2);
    e.add_minion(SeatId(0), of_capacity(5, Some(Sect::Anarch), 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let smear = in_play(&mut e, 0, "Smear Campaign", None);
    let rebel = in_play(&mut e, 1, "Rebel", Some(v));
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(v));
    // Locked through seat 1's turn, as by its own action.
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[v.index()].locked = true;
    let use_on = |c| use_at(smear, 0, None, Target::Card(c));
    let req = offering(&mut e, use_on(rebel));
    assert_eq!(req.seat, SeatId(0));
    assert!(!req.options.contains(&use_on(gun)));
    e.choose(index_of(&req, use_on(rebel)));
    request(&mut e);
    assert!(!e.state.cards[rebel.index()].present());
    assert!(e.state.cards[gun.index()].present());
    assert_eq!(e.state.card_controller(smear), SeatId(1));
    assert!(e.state.cards[smear.index()].locked);
}

// ---- Step 2: Babalawo Alafin, Breath of Thanatos, Disciple of Lazarus,
// ---- Emerald Legionnaire, Erebus, Erlik, Mina Grotius, Nicomedes,
// ---- Solomon Batanea, Trochomancy

/// A Harbinger of Skulls: Nicomedes (capacity 5), then Solomon Batanea.
fn harbinger(nth: usize) -> u16 {
    find_card(["Nicomedes", "Solomon Batanea"][nth])
}

fn slot(card: u16, counters: u8) -> zvtes::state::UncontrolledSlot {
    zvtes::state::UncontrolledSlot {
        card,
        copy: Default::default(),
        counters,
        banished: None,
    }
}

/// The arena index of the card whose body `m` is (an ally).
fn body_card(e: &Engine, m: MinionId) -> CardId {
    CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.body == Some(m))
            .expect("an ally's card") as u32,
    )
}

// Babalawo Alafin: "During your unlock phase, if Alafin is ready, you may
// discard two cards (draw afterward). If the two cards are the same, each
// other Methuselah's hand size is one card smaller until his or her next
// unlock phase." Two Deflections: seat 1 holds 6 until its own unlock
// phase; a Deflection and a Bum's Rush: 7.
#[test]
fn babalawo_alafin_shrinks_the_others_hands_with_a_pair() {
    for (second, want) in [(1u8, 6usize), (2, 7)] {
        let mut e = game(2);
        let b = e.add_minion(SeatId(0), find_card("Babalawo Alafin"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Deflection", "Deflection", "Bum's Rush"]);
        let req = offering(&mut e, vamp_use(b, 0));
        assert_eq!(req.window, WindowLabel::UnlockAbilities);
        e.choose(index_of(&req, vamp_use(b, 0)));
        pick(&mut e, Choice::Discard(0));
        pick(&mut e, Choice::Discard(second));
        assert_eq!(e.state.hand_size(SeatId(1)), want);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.hand_size(SeatId(1)), 7, "until its unlock phase");
    }
}

fn vamp_use(m: MinionId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: None,
        target: Target::None,
    }
}

// Breath of Thanatos: "[nec] [ACTION MODIFIER] Only usable during an
// action to recruit or employ a wraith. +2 stealth. [NEC] [COMBAT] Strike:
// 1 aggravated damage." Recruiting an Emerald Legionnaire (wraith) against
// a 1-intercept block: offered; on a bleed: not.
#[test]
fn breath_of_thanatos_hides_a_wraith_recruitment() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Erlik"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    in_play(&mut e, 1, "Sport Bike", Some(z));
    hand(&mut e, 0, &["Emerald Legionnaire", "Breath of Thanatos"]);
    pick(&mut e, play(0, 0, h));
    pick_when(&mut e, block(z));
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, h)), "{:?}", req.options);

    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Erlik"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    in_play(&mut e, 1, "Sport Bike", Some(z));
    hand(&mut e, 0, &["Breath of Thanatos"]);
    pick(&mut e, bleed(h));
    pick_when(&mut e, block(z));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, h));
}

// Breath of Thanatos [NEC]: "Strike: 1 aggravated damage": the blocker
// loses a blood at least.
#[test]
fn breath_of_thanatos_strikes_for_one_aggravated() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Erlik"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    hand(&mut e, 0, &["Breath of Thanatos"]);
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(blood(&e, z) <= 7);
}

// Disciple of Lazarus: "Add 2 blood to a younger Harbinger of Skulls in
// your uncontrolled region, or search your crypt for a Harbinger of
// Skulls, reveal them, and move them to your uncontrolled region (shuffle
// afterward)." Erlik feeds a younger Harbinger; then finds one in the
// crypt, a Ventrue not offered.
#[test]
fn disciple_of_lazarus_feeds_or_finds_a_harbinger() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Erlik"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let young = harbinger(0);
    assert!(CRYPT[young as usize].capacity < 9);
    e.state.seats[0].uncontrolled.push(slot(young, 0));
    hand(&mut e, 0, &["Disciple of Lazarus"]);
    pick(&mut e, play_at(0, 0, h, Target::Uncontrolled(0)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 2);

    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Erlik"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ventrue = plain("Ventrue", 1);
    e.state.seats[0].crypt = vec![ventrue, young].into();
    hand(&mut e, 0, &["Disciple of Lazarus"]);
    e.notes = Some(Vec::new());
    pick(&mut e, play_at(0, 1, h, Target::None));
    let req = offering(&mut e, Choice::AshVampire(young));
    assert!(!req.options.contains(&Choice::AshVampire(ventrue)));
    e.choose(index_of(&req, Choice::AshVampire(young)));
    request(&mut e);
    assert_eq!(e.state.seats[0].uncontrolled[0].card, young);
    assert_eq!(e.state.seats[0].crypt, vec![ventrue]);
    // The vampire found, revealed to all.
    let revealed = noted_reveals(&mut e);
    assert_eq!(
        revealed
            .iter()
            .map(|r| (r.card, r.crypt))
            .collect::<Vec<_>>(),
        vec![(young, true)]
    );
}

fn play_at(card: u8, mode: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target,
    }
}

// Emerald Legionnaire: "During your unlock phase, if you control fewer
// copies of Emerald Legionnaire than ready unique Harbinger of Skulls, you
// can remove 7 cards in your ash heap from the game to move an Emerald
// Legionnaire from your ash heap to the ready region with 2 life." [ANK
// 20180509] "It does not require a ready Harbinger, the pool cost of the
// new one is not paid, the new one can act this turn": one Legionnaire,
// two ready Harbingers — the copy in the ash heap stands, unlocked, 2 life,
// no pool paid, 7 cards removed. [ANK 20180511-1] "His ability cannot be
// used from the ash heap, he needs to be ready": with none in play,
// nothing is offered; with as many in play as Harbingers, neither.
#[test]
fn emerald_legionnaire_raises_another_from_the_ash_heap() {
    let mut e = game(2);
    e.add_minion(SeatId(0), harbinger(0), 4, false);
    e.add_minion(SeatId(0), harbinger(1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let el = e.add_ally(SeatId(0), find_library("Emerald Legionnaire"));
    let card = body_card(&e, el);
    let legionnaire = find_library("Emerald Legionnaire");
    e.state.seats[0].ash = vec![legionnaire].into();
    e.state.seats[0].ash.extend([find_library("Deflection"); 7]);
    let pool0 = pool(&e)[0];
    let used = next_turn_taking(&mut e, SeatId(0), |o| {
        *o == use_at(card, 0, None, Target::None)
    });
    assert_eq!(used, 1);
    let raised: Vec<&zvtes::state::Minion> = e
        .state
        .minions
        .iter()
        .filter(|m| m.in_play && m.kind.ally_card() == Some(legionnaire))
        .collect();
    assert_eq!(raised.len(), 2);
    assert!(raised.iter().all(|m| !m.locked && m.blood == 2));
    assert!(e.state.seats[0].ash.is_empty());
    assert_eq!(e.state.seats[0].removed.len(), 7);
    assert_eq!(pool(&e)[0], pool0);

    for (in_play, harbingers) in [(0usize, 1usize), (1, 1)] {
        let mut e = game(2);
        for k in 0..harbingers {
            e.add_minion(SeatId(0), harbinger(k), 4, false);
        }
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        for _ in 0..in_play {
            e.add_ally(SeatId(0), find_library("Emerald Legionnaire"));
        }
        e.state.seats[0].ash = vec![legionnaire].into();
        e.state.seats[0].ash.extend([find_library("Deflection"); 7]);
        let used = next_turn_taking(&mut e, SeatId(0), |o| {
            matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. }
                if e_card_is(*c, legionnaire))
        });
        assert_eq!(used, 0, "{in_play} in play, {harbingers} Harbinger");
    }
}

/// Every arena card here is the one named: a stand-in for the closure.
fn e_card_is(_c: CardId, _card: u16) -> bool {
    true
}

// Erebus: "Wraiths (allies and retainers) cost Erebus 1 fewer blood or
// pool." An Emerald Legionnaire (1 pool) costs him none; a mortal ally his
// full price.
#[test]
fn erebus_recruits_wraiths_cheaper() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Erebus"), 4, false);
    let wraith = e
        .state
        .printed_cost_off(m, find_library("Emerald Legionnaire"), 0);
    let mortal = e
        .state
        .printed_cost_off(m, find_library("Neighborhood Watch Commander"), 0);
    assert_eq!((wraith.either, mortal.either), (1, 0));
}

// Erlik: "As Erlik enters play, you can distribute up to 5 blood from him
// to any of your uncontrolled Harbingers of Skulls. +1 bleed. +1 hand
// size." [LSJ 20080526] [LSJ 20100902] [ANK 20180626-1] "His 'as he
// enters play' ability can be used before he enters contest (not after)": moved out with 8 blood, he gives
// 2 to the Harbinger beside him and stops; with a copy in play, he gives
// first, then the contest begins.
#[test]
fn erlik_feeds_the_harbingers_as_he_enters_play() {
    for contested in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if contested {
            e.add_minion(SeatId(1), find_card("Erlik"), 4, false);
        }
        let _ = z;
        let erlik = find_card("Erlik");
        e.state.seats[0].uncontrolled = vec![slot(erlik, 10), slot(harbinger(1), 0)];
        to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
        pick(&mut e, Choice::InfluenceOut { slot: 0 });
        let req = expect(&mut e, SeatId(0), WindowLabel::EntryEffect);
        assert!(req.options.contains(&Choice::Pass));
        e.choose(index_of(&req, Choice::Slot(0)));
        pick(&mut e, Choice::Slot(0));
        pick(&mut e, Choice::Pass);
        assert_eq!(e.state.seats[0].uncontrolled[0].counters, 2);
        let m = MinionId(
            e.state
                .minions
                .iter()
                .position(|v| v.owner == SeatId(0) && v.crypt_card() == Some(erlik))
                .expect("Erlik") as u32,
        );
        assert_eq!(blood(&e, m), 8);
        assert_eq!(e.state.minions[m.index()].contested, contested);
        // A contested Erlik's text is out of play.
        assert_eq!(e.state.hand_size(SeatId(0)), if contested { 7 } else { 8 });
    }
}

// Mina Grotius: "Whenever a vampire controlled by your predator or prey is
// burned, Mina gains 3 blood. Once each action, she can burn 1 blood to
// get +1 bleed." [ANK 20231124] [LSJ 19970718] "The ability to increase
// bleed can only be used during a bleed action before Action Resolution":
// her bleed for 2, once; seat 1's vampire burned (in torpor), she gains 3;
// her own Methuselah's, nothing.
#[test]
fn mina_grotius_feeds_on_burned_neighbors_and_bleeds_for_blood() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mina Grotius"), 2, false);
    let own = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.burn(own);
    assert_eq!(blood(&e, m), 2);
    e.burn(z);
    assert_eq!(blood(&e, m), 5);
    pick(&mut e, bleed(m));
    let req = offering(&mut e, vamp_use(m, 0));
    e.choose(index_of(&req, vamp_use(m, 0)));
    let req = request(&mut e);
    assert!(!req.options.contains(&vamp_use(m, 0)), "once each action");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, m), 4);
    assert_eq!(pool(&e)[1], 10 - 2);
}

// Nicomedes: "During your turn, Nicomedes can burn 1 blood to shuffle your
// hand into your library (draw up afterward)." Once each turn: two cards
// in hand go back, seven come out.
#[test]
fn nicomedes_reshuffles_his_hand_once_a_turn() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nicomedes"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Deflection", "Bum's Rush"]);
    e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
    pick(&mut e, vamp_use(n, 0));
    let req = request(&mut e);
    assert_eq!(e.state.seats[0].hand.len(), 7);
    assert!(e.state.seats[0].library.is_empty());
    assert_eq!(blood(&e, n), 3);
    assert!(!req.options.contains(&vamp_use(n, 0)), "once each turn");
}

// [ANK 20180925-1] [LSJ 20100506] Infernal Pursuit [CEL]: "If an effect
// would force you to replace more than 1 card (e.g. Nicomedes), replace and
// draw extra cards individually", and "Effects that make you discard (eg. Nicomedes)
// are subject to Infernal Pursuit": in combat, after Infernal Pursuit,
// Nicomedes reshuffles his hand: each card drawn brings its extra, a
// discard after each.
#[test]
fn nicomedes_reshuffle_draws_infernal_pursuit_extras_one_at_a_time() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nicomedes"), 6, false);
    superior(&mut e, n, "cel");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    hand(&mut e, 0, &["Infernal Pursuit", "Deflection", "Bum's Rush"]);
    e.state.seats[0].library = (0..30)
        .map(|i| find_library(["Bonding", "Conditioning"][i % 2]))
        .collect();
    blocked_bleed(&mut e, n, z);
    pick_when(&mut e, play(0, 1, n));
    let req = until(&mut e, SeatId(0), WindowLabel::DiscardDown);
    e.choose(req.options.len() - 1);
    let req = offering(&mut e, vamp_use(n, 0));
    e.choose(index_of(&req, vamp_use(n, 0)));
    let mut discards = 0;
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::DiscardDown) {
            discards += 1;
            assert!(e.state.seats[0].hand.len() <= 8, "one at a time");
            e.choose(req.options.len() - 1);
            continue;
        }
        break;
    }
    assert!(discards >= 2, "{discards}");
}

// Solomon Batanea: "If Solomon is ready, you may spend transfers to move
// blood from your prey's uncontrolled vampires to your prey's pool at a
// cost of two transfers for each blood moved." Two transfers: a blood from
// seat 1's uncontrolled vampire to its pool.
#[test]
fn solomon_batanea_drains_the_preys_uncontrolled_region() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Solomon Batanea"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1].uncontrolled = vec![slot(plain("Ventrue", 1), 3)];
    to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    pick(&mut e, vamp_use(s, 0));
    request(&mut e);
    assert_eq!(e.state.seats[1].uncontrolled[0].counters, 2);
    assert_eq!(pool(&e)[1], 11);
}

// Trochomancy: "[nec] Remove thirteen cards in the target Methuselah's ash
// heap from the game to get +1 bleed (limited)." [LSJ 20090710] "The cards
// to remove are chosen by the player playing it. There is no order to the
// ash heap." [PIB 20120525] "The cards are removed immediately." Thirteen
// cards in seat 1's ash heap: played, the bleed is for 2 and they are
// gone; twelve: not offered.
#[test]
fn trochomancy_burns_the_targets_ash_heap_for_bleed() {
    for (cards, offered) in [(13usize, true), (12, false)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Nicomedes"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.seats[1].ash = vec![find_library("Deflection"); cards].into();
        hand(&mut e, 0, &["Trochomancy"]);
        pick(&mut e, bleed(a));
        if !offered {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
            continue;
        }
        pick_when(&mut e, play(0, 0, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.seats[1].ash.is_empty());
        assert_eq!(e.state.seats[1].removed.len(), 13);
        assert_eq!(pool(&e)[1], 10 - 2);
    }
}

// ---- Step 3: Arika, Catherine du Bois, Le Dinh Tho, Lucinde, Marcus
// ---- Vitel, Queen Anne (G2), The Baron

// Arika: "If Arika is ready during your prey's unlock phase, your prey
// chooses which locations he or she keeps in play. For each location he
// or she controls, your prey burns 1 pool or burns the location. +2
// bleed." [ANK 20200629] "Arika's prey can use a location during her
// unlock phase before burning it." [ANK 20250121] "The effect only
// triggers once each unlock phase": seat 1 feeds a vampire from its
// hunting ground, then resolves Arika — the hunting ground burns, the
// other location costs 1 pool; nothing more is due.
#[test]
fn arika_taxes_her_preys_locations() {
    let mut e = game(2);
    let arika = e.add_minion(SeatId(0), find_card("Arika"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
    let hg = in_play(&mut e, 1, "Academic Hunting Ground", None);
    let other = in_play(&mut e, 1, "Park Hunting Ground", None);
    let feed = use_at(hg, 0, None, Target::Minion(z));
    let arika_due = Choice::ResolvePrinted { minion: arika };
    let req = offering(&mut e, arika_due);
    assert_eq!(req.seat, SeatId(1));
    assert!(req.options.contains(&feed));
    e.choose(index_of(&req, feed));
    // Arika's effect, the only thing left due.
    pick(&mut e, arika_due);
    let mut paid = false;
    loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&arika_due), "once");
        if req.options.contains(&Choice::PayToCancel) {
            // The first one burns, the second is paid for.
            let i = if paid {
                index_of(&req, Choice::PayToCancel)
            } else {
                index_of(&req, Choice::Pass)
            };
            paid = true;
            e.choose(i);
            continue;
        }
        break;
    }
    assert_eq!(blood(&e, z), 3, "fed first");
    let burned = [hg, other]
        .iter()
        .filter(|c| !e.state.cards[c.index()].present())
        .count();
    assert_eq!(burned, 1);
    assert_eq!(pool(&e)[1], 9);
}

// Arika: "+2 bleed."
#[test]
fn arika_bleeds_for_three() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Arika"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert_eq!(bled(e, a), 3);
}

// Le Dinh Tho: "Tho may look at another Methuselah's hand and discard one
// card from it as a +1 stealth Ⓓ action (that Methuselah draws back up to
// his or her hand size). Tho gets -1 stealth when hunting." [PIB
// 20121031] "When using his ability, you do not get to see the card drawn
// as replacement": the Bum's Rush discarded, seat 1 draws back unseen.
#[test]
fn le_dinh_tho_discards_from_another_hand() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Le Dinh Tho"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Deflection", "Bum's Rush"]);
    e.state.seats[1].library = vec![find_library("Bonding"); 3].into();
    let hunt = e.state.printed_mod(
        t,
        Stat::Stealth,
        PrintedCtx {
            kind: Some(ActionKind::Hunt),
            ..PrintedCtx::default()
        },
    );
    assert_eq!(hunt, -1);
    pick(&mut e, vamp_action(t, 0, Target::Seat(SeatId(1))));
    let rush = find_library("Bum's Rush");
    pick_when(&mut e, Choice::FindCard(rush));
    request(&mut e);
    assert!(e.state.seats[1].ash.contains(&rush));
    assert!(!e.state.seats[1].hand.contains(&rush));
    assert!(!e.state.hand_visible_to(SeatId(1), SeatId(0)), "unseen");
}

// Scarce: "When a Methuselah moves a scarce vampire from their uncontrolled
// region to their ready region during their influence phase, they must burn
// 3 pool for each vampire of the same clan they already control." [LSJ
// 20011015] "The scarce penalty considers the apparent clan while
// controlled": beside a created Nagaraja vampire (Childe of the
// Revolution's kind, its clan its own), Le Dinh Tho costs 3 pool.
#[test]
fn le_dinh_tho_is_scarce() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    let childe = find_library("Childe of the Revolution");
    e.state
        .cards
        .push(CardInPlay::new(childe, SeatId(0), None, 0));
    e.state.minions[n.index()].kind = zvtes::state::MinionKind::Created {
        card: childe,
        id: CardId(e.state.cards.len() as u32 - 1),
        clan: Some("Nagaraja"),
        sect: Sect::Independent,
        born: &zvtes::state::NEWBORN,
    };
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let tho = find_card("Le Dinh Tho");
    e.state.seats[0].uncontrolled = vec![slot(tho, CRYPT[tho as usize].capacity)];
    to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    let before = pool(&e)[0];
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    request(&mut e);
    assert_eq!(pool(&e)[0], before - 3);
}

// Lucinde: "can enter combat with any ready non-Camarilla vampire
// controlled by another Methuselah as a Ⓓ action. During your unlock
// phase, you can burn the Edge to give Lucinde 1 blood from the blood
// bank." A Sabbat vampire is a target, a Camarilla one is not; the Edge
// burned, she gains 1 blood.
#[test]
fn lucinde_hunts_non_camarilla_and_burns_the_edge() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lucinde, Alastor"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let cam = e.add_minion(
        SeatId(1),
        of_capacity(4, Some(Sect::Camarilla), 0),
        4,
        false,
    );
    let sab = e.add_minion(SeatId(1), of_capacity(4, Some(Sect::Sabbat), 0), 4, false);
    let req = request(&mut e);
    assert!(req
        .options
        .contains(&vamp_action(l, 0, Target::Minion(sab))));
    assert!(!req
        .options
        .contains(&vamp_action(l, 0, Target::Minion(cam))));
    e.state.seats[0].has_edge = true;
    let used = next_turn_taking(&mut e, SeatId(0), |o| *o == vamp_use(l, 1));
    assert_eq!(used, 1);
    assert!(!e.state.seats[0].has_edge);
    assert_eq!(blood(&e, l), 4);
}

// Marcus Vitel: "Marcus can burn a retainer as a +1 stealth Ⓓ action.
// While Marcus is ready and a prince, Sabbat vampires you control get +1
// bleed when bleeding a Methuselah who controls a ready Camarilla vampire."
// Seat 1's retainer burns; seat 0's Sabbat vampire bleeds for 2 against
// seat 1's ready Camarilla vampire, for 1 once Marcus is in torpor.
#[test]
fn marcus_vitel_burns_retainers_and_spurs_the_sabbat() {
    let mut e = game(2);
    let mv = e.add_minion(SeatId(0), find_card("Marcus Vitel"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(
        SeatId(1),
        of_capacity(4, Some(Sect::Camarilla), 0),
        4,
        false,
    );
    let r = in_play(&mut e, 1, "Owl Companion", Some(z));
    pick(&mut e, vamp_action(mv, 0, Target::Card(r)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[r.index()].present());

    for torpid in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), find_card("Marcus Vitel"), 4, torpid);
        let s = e.add_minion(SeatId(0), of_capacity(4, Some(Sect::Sabbat), 0), 4, false);
        e.add_minion(
            SeatId(1),
            of_capacity(4, Some(Sect::Camarilla), 0),
            4,
            false,
        );
        assert_eq!(bled(e, s), if torpid { 1 } else { 2 });
    }
}

// Queen Anne (G2): "Anne gets an additional +1 bleed when bleeding a
// Methuselah who controls a ready Tremere. +1 bleed." Two, three against a
// ready Tremere.
#[test]
fn queen_anne_g2_bleeds_harder_against_tremere() {
    let anne = CRYPT
        .iter()
        .position(|c| c.name == "Queen Anne" && c.group == 2)
        .expect("Queen Anne G2") as u16;
    for (tremere, want) in [(false, 2), (true, 3)] {
        let mut e = game(2);
        let q = e.add_minion(SeatId(0), anne, 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if tremere {
            e.add_minion(SeatId(1), plain("Tremere", 0), 4, false);
        }
        assert_eq!(bled(e, q), want);
    }
}

// The Baron: "The Baron has 2 votes (titled). During your unlock phase, if
// The Baron is ready, you may look at the hand of each Methuselah who
// controls a ready Giovanni. +1 bleed." [PIB 20151116] "Cannot play cards
// requiring a Baron." [LSJ 20010714] "Keeps their title and votes if they
// changes sects." His title is 2 votes, no baron's, kept as Sabbat; seat
// 1's hand opens to him while it has a ready Giovanni.
#[test]
fn the_baron_votes_twice_and_reads_the_giovanni_hands() {
    let mut e = game(3);
    let b = e.add_minion(SeatId(0), find_card("Baron, The"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), find_card("Giovanni del Georgio"), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    assert_eq!(e.state.title_of(b), zvtes::crypt::Title::Votes(2));
    assert!(!cards::TitleClass::Baron.matches(e.state.title_of(b)));
    e.state.minions[b.index()].sect_override = Some(Sect::Sabbat);
    assert_eq!(e.state.title_votes_of(b), 2);
    e.state.minions[b.index()].sect_override = None;
    let used = next_turn_taking(&mut e, SeatId(0), |o| *o == vamp_use(b, 0));
    assert_eq!(used, 1);
    assert!(e.state.hand_visible_to(SeatId(1), SeatId(0)));
    assert!(!e.state.hand_visible_to(SeatId(2), SeatId(0)));
}

// ---- Step 4: Beetleman, Céleste Lamontagne, Effie Lowery, Fish, Ludmijla
// ---- Rakoczy, Petra, Rurik Rakoczy, Sahana, Stick, Yuri Kerezenski

// Céleste Lamontagne: "Céleste can strike: combat ends in combat with a
// werewolf. She gets +1 intercept against Camarilla vampires whose
// controller controls any Gehenna cards." +1 against a Camarilla vampire
// once its Methuselah has Absimiliard's Army in play, none before nor
// against a Sabbat one.
#[test]
fn celeste_lamontagne_watches_gehenna_camarilla() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Céleste Lamontagne"), 4, false);
    let cam = e.add_minion(
        SeatId(1),
        of_capacity(4, Some(Sect::Camarilla), 0),
        4,
        false,
    );
    let sab = e.add_minion(SeatId(1), of_capacity(4, Some(Sect::Sabbat), 0), 4, false);
    let against = |e: &Engine, o| {
        e.state.printed_mod(
            c,
            Stat::Intercept,
            PrintedCtx {
                opponent: Some(o),
                ..PrintedCtx::default()
            },
        )
    };
    assert_eq!(against(&e, cam), 0);
    in_play(&mut e, 1, "Absimiliard's Army", None);
    assert_eq!((against(&e, cam), against(&e, sab)), (1, 0));
}

// Céleste Lamontagne's combat ends: offered in combat with a werewolf (a
// Renegade Garou bleeding into her), not with a vampire.
#[test]
fn celeste_lamontagne_ends_combat_with_a_werewolf() {
    let mut e = game(2);
    let g = e.add_ally(SeatId(0), find_library("Renegade Garou"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let c = e.add_minion(SeatId(1), find_card("Céleste Lamontagne"), 4, false);
    blocked_bleed(&mut e, g, c);
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(req.options.contains(&vamp_use(c, 0)));
}

// Ludmijla Rakoczy: "Cards requiring Animalism [ani] cost Ludmijla -1
// blood."
#[test]
fn ludmijla_rakoczy_pays_less_for_animalism() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Ludmijla Rakoczy"), 4, false);
    let ani = e
        .state
        .printed_cost_off(l, find_library("Aire of Elation"), 0);
    assert_eq!(ani.blood, 0, "Aire of Elation requires Presence");
    let card = (0..zvtes::library::LIBRARY.len() as u16)
        .find(|&c| {
            let l = &zvtes::library::LIBRARY[c as usize];
            l.disciplines == ["ani"] && l.blood_cost == zvtes::library::Cost::Fixed(1)
        })
        .expect("a 1-blood Animalism card");
    assert_eq!(e.state.printed_cost_off(l, card, 0).blood, 1);
}

// Petra: "A vampire burns 1 blood after using a press to continue combat
// with her." Rurik Rakoczy presses on against her: a blood more than
// against another Nosferatu of her strength.
#[test]
fn petra_taxes_a_press_to_continue() {
    let left = |name: Option<&str>| {
        let mut e = game(2);
        let r = e.add_minion(SeatId(0), find_card("Rurik Rakoczy"), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let card = name.map_or(plain("Nosferatu", 0), find_card);
        let p = e.add_minion(SeatId(1), card, 8, false);
        blocked_bleed(&mut e, r, p);
        pick_when(&mut e, vamp_use(r, 0));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        blood(&e, r)
    };
    assert_eq!(left(None) - left(Some("Petra")), 1);
}

// Petra: "Damage Petra inflicts on Baali is aggravated." Her hand strike
// on Arishat (Baali), as she blocks: the point wounds her — torpor, her 2
// blood kept (rulebook "Damage") — where a normal one is mended.
#[test]
fn petra_inflicts_aggravated_damage_on_baali() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), find_card("Petra"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), find_card("Arishat"), 2, false);
    blocked_bleed(&mut e, p, b);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[b.index()].in_torpor);
    assert_eq!(blood(&e, b), 2);
}

// Rötschreck answers Petra's hand strike on a Baali: aggravated at its
// source (ANK 20200130-1), Arishat's Methuselah may put it on her.
#[test]
fn rotschreck_answers_petras_strike_on_a_baali() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), find_card("Petra"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), find_card("Arishat"), 2, false);
    hand(&mut e, 1, &["Rötschreck"]);
    blocked_bleed(&mut e, p, b);
    let rot = Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(b),
    };
    assert!(offered_before(
        &mut e,
        rot,
        SeatId(0),
        WindowLabel::MinionPhase
    ));
}

// Rurik Rakoczy: "Rurik gets 1 optional press each combat. While /Ludmijla
// Rakoczy/ is ready, Rurik gets +1 bleed." One, two beside a ready
// Ludmijla (of another Methuselah's).
#[test]
fn rurik_rakoczy_bleeds_harder_beside_ludmijla() {
    for (ludmijla, want) in [(false, 1), (true, 2)] {
        let mut e = game(2);
        let r = e.add_minion(SeatId(0), find_card("Rurik Rakoczy"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if ludmijla {
            e.add_minion(SeatId(1), find_card("Ludmijla Rakoczy"), 4, false);
        }
        assert_eq!(bled(e, r), want);
    }
}

// Sterile (Effie Lowery, Sahana): "Sterile vampires cannot perform actions
// to put new vampires in play" — Childe of the Revolution's "non-sterile
// baron".
#[test]
fn sterile_vampires_are_flagged_for_childe_of_the_revolution() {
    let mut e = game(2);
    let effie = e.add_minion(SeatId(0), find_card("Effie Lowery"), 4, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let holds = |m| {
        cards::holds(
            cards::Requirement::ActorNotSterile,
            &e.state,
            SeatId(0),
            Some(m),
            None,
            None,
            None,
            None,
            false,
        )
    };
    assert!(!holds(effie));
    assert!(holds(other));
    assert!(CRYPT[find_card("Sahana") as usize].sterile);
}

// ---- Step 5: Coma, Confusion, Eyes of Chaos, Kindred Spirits, Mind
// ---- Tricks, Morel, Osric Vladislav, Persephone Tar-Anis

// Coma: "[DEM] Strike: send the opposing vampire to torpor, and the
// opposing vampire does not unlock as normal during their next unlock
// phase." [LSJ 20050114] "The 'does not unlock as normal' effect is
// redundant with being infernal. If the minion is infernal, his
// controller can still pay a pool to unlock him": Arishat (infernal),
// sent to torpor by it, may still be unlocked for a pool.
#[test]
fn coma_sends_to_torpor_and_holds_the_unlock() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Erebus"), 6, false);
    superior(&mut e, a, "dem");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Arishat"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Coma"]);
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor);
    assert!(e.state.minions[z.index()].skip_unlock);
    let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&Choice::UnlockInfernal { minion: z }));
}

// Coma [RBK resolve-strike] [LSJ 20000110]: "Only effective at close
// range." The blocker's .44 Magnum maneuvers to long range: Coma lands
// nothing.
#[test]
fn coma_needs_close_range() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Erebus"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 0, &["Coma"]);
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, use_at(gun, 1, None, Target::None));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].in_torpor);
}

// Confusion: "Only usable during a bleed action. [dem] +1 bleed (limited).
// [DEM] +1 stealth and +1 bleed (limited)." [ANK 20171023] [RTR 19941109]
// "[DEM] Can only be used during a bleed action": a bleed for 2; on a
// hunt, not offered.
#[test]
fn confusion_adds_to_a_bleed_only() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Erebus"), 4, false);
    superior(&mut e, a, "dem");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Confusion"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 2);

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Erebus"), 1, false);
    superior(&mut e, a, "dem");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Confusion"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
}

// Confusion [DEM] during a block attempt that needs stealth: offered once,
// never the same play twice.
#[test]
fn confusion_superior_is_offered_once_as_stealth_is_needed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Erebus"), 4, false);
    superior(&mut e, a, "dem");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Confusion"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, play(0, 1, a));
    let n = req
        .options
        .iter()
        .filter(|o| {
            matches!(
                o,
                Choice::Play {
                    card: 0,
                    mode: 1,
                    ..
                }
            )
        })
        .count();
    assert_eq!(n, 1, "the same play twice");
    assert!(!req.options.contains(&play(0, 2, a)));
}

// Eyes of Chaos: "[DEM] +2 bleed (limited)."
#[test]
fn eyes_of_chaos_bleeds_for_three() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Erebus"), 4, false);
    superior(&mut e, a, "dem");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Eyes of Chaos"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 3);
}

// Kindred Spirits: "[dem] Ⓓ Bleed any Methuselah. Gain 1 pool if the bleed
// is successful (for 1 or more). [DEM] As above, with +1 bleed." Seat 0
// bleeds seat 2 (not its prey) for 2 and gains 1 pool.
#[test]
fn kindred_spirits_bleeds_any_methuselah() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), find_card("Erebus"), 4, false);
    superior(&mut e, a, "dem");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Kindred Spirits"]);
    pick(&mut e, play_at(0, 1, a, Target::Seat(SeatId(2))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![11, 10, 8]);
}

// Mind Tricks: "[DEM] +1 stealth, with 1 optional maneuver or press during
// the resulting combat if this vampire is blocked." Blocked anyway, its
// vampire maneuvers: no press left.
#[test]
fn mind_tricks_gives_a_maneuver_or_a_press() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Erebus"), 4, false);
    superior(&mut e, a, "dem");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    in_play(&mut e, 1, "Sport Bike", Some(z));
    in_play(&mut e, 1, "Sport Bike", Some(z));
    hand(&mut e, 0, &["Mind Tricks"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&Choice::UseManeuver));
    e.choose(index_of(&req, Choice::UseManeuver));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::UsePress,
    );
}

// Osric Vladislav: "Osric can burn a location you control to get +1
// stealth for the current action." [LSJ 20020814] [ANK 20221229] "His
// ability can be used more than once during an action, but only if he is
// acting and stealth is needed": against a 1-intercept blocker, both his
// locations burn and the block fails.
#[test]
fn osric_vladislav_burns_locations_for_stealth() {
    let mut e = game(2);
    let o = e.add_minion(SeatId(0), find_card("Osric Vladislav"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    in_play(&mut e, 1, "Sport Bike", Some(z));
    let l1 = in_play(&mut e, 0, "Academic Hunting Ground", None);
    let l2 = in_play(&mut e, 0, "Park Hunting Ground", None);
    let burn = |c| Choice::UseAbility {
        source: AbilitySource::Vampire(o),
        ability: 0,
        actor: None,
        target: Target::Card(c),
    };
    pick(&mut e, bleed(o));
    pick_when(&mut e, block(z));
    pick_when(&mut e, burn(l1));
    pick_when(&mut e, burn(l2));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[l1.index()].present() && !e.state.cards[l2.index()].present());
    assert_eq!(pool(&e)[1], 9, "unblocked");
}

// Persephone Tar-Anis: "+1 bleed." Morel prints no text.
#[test]
fn persephone_tar_anis_bleeds_for_two() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), find_card("Persephone Tar-Anis"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert_eq!(bled(e, p), 2);
}

// ---- Step 6: the Giovanni toolbox — Accorri, Acquired Ventrue Assets,
// ---- Almodo, Call of the Hungry Dead, Diego, Don Michael Antonio, Donatello,
// ---- Eric Milliner, Fake Out, Gianmaria, Nunzio, Powerbase: Cape Verde,
// ---- Primo, Raphaela, Spectral Divination, Spiritual Guidance, Sudario
// ---- Refraction, Thomasso Ghiberti, Thrown Sewer Lid

fn giovanni(name: &str) -> u16 {
    find_card(name)
}

// Acquired Ventrue Assets: "Put this card in play with 1 blood for each
// Giovanni you control. During your master phase, you can move 1 blood from
// this card to a Giovanni you control. Burn this card if it has no blood.
// Any non-Ventrue minion can burn this card as a Ⓓ action." Two Giovanni:
// 2 blood; one moved this master phase; a Brujah may burn it, a Ventrue
// not.
#[test]
fn acquired_ventrue_assets_feeds_the_giovanni() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), giovanni("Primo Giovanni"), 2, false);
    e.add_minion(SeatId(0), giovanni("Nunzio Giovanni"), 2, false);
    let brujah = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ventrue = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Acquired Ventrue Assets"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    request(&mut e);
    let ava = card_named(&e, "Acquired Ventrue Assets");
    assert_eq!(e.state.cards[ava.index()].counters, 2);
    let feed = use_at(ava, 0, None, Target::Minion(g));
    let req = offering(&mut e, feed);
    e.choose(index_of(&req, feed));
    let req = request(&mut e);
    assert_eq!(blood(&e, g), 3);
    assert_eq!(e.state.cards[ava.index()].counters, 1);
    assert!(!req.options.contains(&feed), "once each master phase");
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let burn = |m| use_at(ava, 1, Some(m), Target::None);
    assert!(req.options.contains(&burn(brujah)));
    assert!(!req.options.contains(&burn(ventrue)));
}

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

// Call of the Hungry Dead: "[NEC] Burn 1 blood to have that attempt fail;
// the blocking minion cannot attempt to block this action again." [ANK
// 20210226] "The burn blood effect is not reduced by effects that reduce
// the cost of the card" and "The card cannot be played if the minion
// cannot afford to burn the blood": Erlik with blood burns 1, the block
// fails, the blocker does not try again; without blood, not offered.
#[test]
fn call_of_the_hungry_dead_fails_a_block_for_blood() {
    for blood_left in [4u8, 0] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Erlik"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Call of the Hungry Dead"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        e.state.minions[a.index()].blood = blood_left;
        if blood_left == 0 {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
            continue;
        }
        pick_when(&mut e, play(0, 1, a));
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(z));
        assert_eq!(blood(&e, a), blood_left - 1);
        assert_eq!(pool(&e)[1], 10 - 2, "unblocked");
    }
}

// Diego Giovanni: "Diego can burn a location as a Ⓓ action that costs 2
// blood." Seat 1's hunting ground burns, Diego pays 2; with 1 blood, not
// offered.
#[test]
fn diego_giovanni_burns_a_location_for_two_blood() {
    for start in [4u8, 1] {
        let mut e = game(2);
        let d = e.add_minion(SeatId(0), giovanni("Diego Giovanni"), start, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let l = in_play(&mut e, 1, "Academic Hunting Ground", None);
        let burn = vamp_action(d, 0, Target::Card(l));
        let req = request(&mut e);
        assert_eq!(req.options.contains(&burn), start >= 2);
        if start < 2 {
            continue;
        }
        e.choose(index_of(&req, burn));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.cards[l.index()].present());
        assert_eq!(blood(&e, d), start - 2);
    }
}

// Don Michael Antonio Giovanni: "2 votes (titled): Michael cannot strike:
// combat ends or press to end combat. He gets +1 strength on any round of
// combat after the first." [LSJ 20010714] "Keeps their title and votes if
// they changes sects." Barred from combat ends; +1 strength from the second
// round; his 2 votes kept as Sabbat.
#[test]
fn don_michael_fights_on() {
    let mut e = game(2);
    let m = e.add_minion(
        SeatId(0),
        giovanni("Don Michael Antonio Giovanni"),
        4,
        false,
    );
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert!(e.state.combat_ends_barred(m, z));
    let at = |e: &Engine, round| {
        e.state.printed_mod(
            m,
            Stat::Strength,
            PrintedCtx {
                round,
                ..PrintedCtx::default()
            },
        )
    };
    assert_eq!((at(&e, 1), at(&e, 2)), (0, 1));
    e.state.minions[m.index()].sect_override = Some(Sect::Sabbat);
    assert_eq!(e.state.title_votes_of(m), 2);
}

// Don Michael "cannot … press to end combat": Rurik presses on against
// him; Psyche! [cel] (a press) in Michael's hand is not offered that round
// — a Ventrue in his place is offered it.
#[test]
fn don_michael_cannot_press_to_end_combat() {
    for michael in [true, false] {
        let mut e = game(2);
        let r = e.add_minion(SeatId(0), find_card("Rurik Rakoczy"), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let card = if michael {
            giovanni("Don Michael Antonio Giovanni")
        } else {
            plain("Ventrue", 0)
        };
        let m = e.add_minion(SeatId(1), card, 8, false);
        superior(&mut e, m, "cel");
        hand(&mut e, 1, &["Psyche!"]);
        blocked_bleed(&mut e, r, m);
        pick_when(&mut e, vamp_use(r, 0));
        let req = request(&mut e);
        let round = e.combat().expect("combat").round;
        let offered = req.options.contains(&play(0, 0, m));
        assert_eq!(
            (offered, round),
            if michael { (true, 2) } else { (true, 1) }
        );
    }
}

// Donatello Giovanni: "During your prey's unlock phase, you can choose a
// vampire controlled by a Methuselah other than your predator. If the
// chosen vampire attempts to bleed before your next unlock phase, they move
// 1 blood to Donatello." Seat 1 (his prey) unlocks: its vampire chosen —
// seat 2's (his predator's) is not offered —; it bleeds, Donatello gains 1.
#[test]
fn donatello_giovanni_taxes_the_chosen_bleeder() {
    let mut e = game(3);
    let d = e.add_minion(SeatId(0), giovanni("Donatello Giovanni"), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let w = e.add_minion(SeatId(2), plain("Tremere", 0), 4, false);
    let choose = |m| Choice::UseAbility {
        source: AbilitySource::Vampire(d),
        ability: 0,
        actor: None,
        target: Target::Minion(m),
    };
    let req = offering(&mut e, choose(v));
    assert!(!req.options.contains(&choose(w)), "his predator's");
    e.choose(index_of(&req, choose(v)));
    pick_when(&mut e, bleed(v));
    request(&mut e);
    assert_eq!((blood(&e, d), blood(&e, v)), (3, 3));
}

// Fake Out: "Do not replace until after combat. Maneuver." Played, seat 0
// holds one card fewer until the combat ends.
#[test]
fn fake_out_is_replaced_after_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 0, &["Fake Out"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    if e.combat().is_some() {
        assert_eq!(e.state.seats[0].hand.len(), 6, "not replaced yet");
    }
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].hand.len(), 7);
}

// Powerbase: Cape Verde: "Lock during any Methuselah's master phase to move
// 1 blood from a ready Giovanni or Laibon you control to your pool or from
// your pool to a ready vampire you control." Seat 0 locks it in seat 1's
// master phase: a blood from its Giovanni to its pool.
#[test]
fn powerbase_cape_verde_works_in_any_master_phase() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), giovanni("Primo Giovanni"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cv = in_play(&mut e, 0, "Powerbase: Cape Verde", None);
    let to_pool = use_at(cv, 0, None, Target::Minion(g));
    let req = offering(&mut e, to_pool);
    assert_eq!((req.seat, e.state.active), (SeatId(0), SeatId(1)));
    e.choose(index_of(&req, to_pool));
    request(&mut e);
    assert_eq!((blood(&e, g), pool(&e)[0]), (2, 11));
    assert!(e.state.cards[cv.index()].locked);
}

// [LSJ 20080604] "The action provided by this card is considered to be an
// action requiring the listed clan/sect": Cape Verde's steal, by seat 1's
// Laibon (Fish), not by its Ventrue.
#[test]
fn powerbase_cape_verde_is_stolen_by_giovanni_or_laibon() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let fish = e.add_minion(SeatId(1), find_card("Fish"), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cv = in_play(&mut e, 0, "Powerbase: Cape Verde", None);
    let steal = |m| use_at(cv, 2, Some(m), Target::None);
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(req.options.contains(&steal(fish)));
    assert!(!req.options.contains(&steal(v)));
}

// Spectral Divination: "[NEC] [REACTION] +1 intercept." Erlik attempts
// to block a +1 stealth hunt: offered.
#[test]
fn spectral_divination_intercepts() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Erlik"), 6, false);
    hand(&mut e, 1, &["Spectral Divination"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, play(0, 1, z));
    assert_eq!(req.seat, SeatId(1));
}

// Spiritual Guidance: "Play a combat card from your ash heap as if from
// your hand. If that card would be burned, instead remove it from the
// game." The card is played as Spiritual Guidance resolves, the impulse
// not going around (owner, rules-qa 67.4): a Giovanni plays it as range is
// determined, a Fake Out in hand too; the ash heap's Fake Out, the one
// play it can make, is made at once (a single option), the hand's kept;
// the opponent's impulse comes after both, the Fake Out removed from the
// game.
#[test]
fn spiritual_guidance_plays_from_the_ash_heap() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), giovanni("Primo Giovanni"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 0, &["Spiritual Guidance", "Fake Out"]);
    let fake = find_library("Fake Out");
    e.state.seats[0].ash = vec![fake].into();
    blocked_bleed(&mut e, a, z);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    e.choose(index_of(&req, play(0, 0, a)));
    expect(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    assert!(e.state.seats[0].removed.contains(&fake));
    assert!(!e.state.seats[0].ash.contains(&fake));
    assert_eq!(e.state.seats[0].hand, vec![fake]);
    assert!(!e.state.seats[0].ash_combat_play);
}

// Spiritual Guidance playing Inner Essence ([vic] lent to Primo by another
// vampire): its discount alone stays on the vampire (rules-qa 68.12), the
// card itself removed from the game, not back in the ash heap.
#[test]
fn spiritual_guidance_removes_a_lasting_effects_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), giovanni("Primo Giovanni"), 4, false);
    let t = e.add_minion(SeatId(0), vampire_with("vic", 0), 4, false);
    assert!(e.state.has_discipline(t, "vic"));
    e.state.lent_disciplines.push((a, t));
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    hand(&mut e, 0, &["Spiritual Guidance"]);
    let essence = find_library("Inner Essence");
    e.state.seats[0].ash = vec![essence].into();
    blocked_bleed(&mut e, a, z);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    e.choose(index_of(&req, play(0, 0, a)));
    request(&mut e);
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == essence && c.effect_only && c.host == Some(a)));
    assert!(e.state.seats[0].removed.contains(&essence));
    assert!(!e.state.seats[0].ash.contains(&essence));
}

// Spiritual Guidance with two plays from the ash heap (Weighted Walking
// Stick, Trap), played before range: the next decision is its player's,
// those plays its only options — no pass, the hand's Trap not offered
// (rules-qa 67.4).
#[test]
fn spiritual_guidance_forces_a_choice_among_the_ash_heaps_plays() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), giovanni("Primo Giovanni"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 0, &["Spiritual Guidance", "Trap"]);
    e.state.seats[0].ash =
        vec![find_library("Weighted Walking Stick"), find_library("Trap")].into();
    blocked_bleed(&mut e, a, z);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    e.choose(index_of(&req, play(0, 0, a)));
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(0));
    assert!(
        req.options.contains(&play(1, 0, a)) && req.options.contains(&play(2, 0, a)),
        "{:?}",
        req.options
    );
    assert!(
        req.options
            .iter()
            .all(|o| matches!(o, Choice::Play { card: 1 | 2, .. })),
        "{:?}",
        req.options
    );
}

// Spiritual Guidance with no combat card playable from the ash heap: a
// legal no-op (rules-qa 17.4), nothing forced after it.
#[test]
fn spiritual_guidance_with_nothing_to_play_from_the_ash_heap() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), giovanni("Primo Giovanni"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 0, &["Spiritual Guidance"]);
    e.state.seats[0].ash.clear();
    blocked_bleed(&mut e, a, z);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    e.choose(index_of(&req, play(0, 0, a)));
    assert!(!e.state.seats[0].ash_combat_play);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
}

// Sudario Refraction: "Choose three library cards in your ash heap by
// name. Move them to the top of your library, then discard 3 cards at
// random (draw afterward)." [LSJ 20050422] [LSJ 20090722] "The chosen
// cards are named when the action is declared. The current Sudario cannot
// be part of them." [LSJ 20090722] [ANK 20230105] "Any copy of the named
// card can be fetched upon resolution": three named; another Sudario in the ash heap
// may be named, the one played not; the three come back.
#[test]
fn sudario_refraction_brings_three_back() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), giovanni("Primo Giovanni"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Sudario Refraction"]);
    let names = ["Bonding", "Conditioning", "Sudario Refraction"];
    e.state.seats[0].ash = names.iter().map(|n| find_library(n)).collect();
    e.state.seats[0].library = vec![find_library("Deflection"); 10].into();
    pick(&mut e, play(0, 0, a));
    // The third name, the only one left, is taken at once.
    for n in &names[..2] {
        pick(&mut e, Choice::FindCard(find_library(n)));
    }
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let back = |e: &Engine, n| {
        let c = find_library(n);
        e.state.seats[0].hand.contains(&c)
            || e.state.seats[0].library.contains(&c)
            || e.state.seats[0].ash.iter().filter(|&&x| x == c).count()
                > (n == "Sudario Refraction") as usize
    };
    for n in names {
        assert!(back(&e, n), "{n}");
    }
}

// Sudario Refraction [LSJ 20090722]: "If no copy of a named card is in the
// ash heap upon resolution, the effect still resolves for the other
// copies and the acting Methuselah discards 3 cards." [LSJ 20080612-2] "If
// the library is empty and the hand is not full, you draw the returned
// cards up to hand size before discarding at random": a named card gone
// by resolution; an empty library — the two returned drawn, then 3
// discarded at random.
#[test]
fn sudario_refraction_resolves_without_a_named_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), giovanni("Primo Giovanni"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(
        &mut e,
        0,
        &[
            "Sudario Refraction",
            "Deflection",
            "Deflection",
            "Deflection",
            "Deflection",
        ],
    );
    let names = ["Bonding", "Conditioning", "Eyes of Chaos"];
    e.state.seats[0].ash = names.iter().map(|n| find_library(n)).collect();
    e.state.seats[0].library.clear();
    pick(&mut e, play(0, 0, a));
    for n in &names[..2] {
        pick(&mut e, Choice::FindCard(find_library(n)));
    }
    // Eyes of Chaos leaves the ash heap before resolution.
    e.state.seats[0]
        .ash
        .retain(|&c| c != find_library("Eyes of Chaos"));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // 4 Deflections + 2 returned = 6, 3 discarded at random, none to draw.
    assert_eq!(e.state.seats[0].hand.len(), 3);
}

// Thomasso Ghiberti: "During your unlock phase, you can move 1 blood from a
// Laibon you control to Thomasso."
#[test]
fn thomasso_ghiberti_takes_a_laibons_blood() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), giovanni("Thomasso Ghiberti"), 1, false);
    let l = e.add_minion(SeatId(0), find_card("Fish"), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let take = Choice::UseAbility {
        source: AbilitySource::Vampire(t),
        ability: 0,
        actor: None,
        target: Target::Minion(l),
    };
    let used = next_turn_taking(&mut e, SeatId(0), |o| *o == take);
    assert_eq!(used, 1);
    assert_eq!((blood(&e, t), blood(&e, l)), (2, 2));
}

// Thrown Sewer Lid: "Only usable at long range. [pot] Strike: 3R damage."
// Offered once the blocker's gun takes the range to long, not at close.
#[test]
fn thrown_sewer_lid_needs_long_range() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 0, &["Thrown Sewer Lid"]);
    blocked_bleed(&mut e, a, z);
    pick_when(&mut e, use_at(gun, 1, None, Target::None));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(req.options.contains(&play(0, 0, a)));

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    hand(&mut e, 0, &["Thrown Sewer Lid"]);
    blocked_bleed(&mut e, a, z);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

// ---- Step 7: Andre LeRoux, Black Annis, Bleeding the Vine, Enkidu, Jackie
// ---- Therman, New Blood, Shadow Boxing; the Red List

// Andre LeRoux: "If a vampire you control would successfully bleed, you
// can reduce the bleed amount by 1 to give Andre +2 bleed this turn." [LSJ
// 20081123] "His ability can be used only once per successful bleed, but
// can be used multiple times on different bleeds during the same turn" and
// "It is used in the 'would/about to be successful' window like Spying
// Mission". [LSJ 20090205] "Only modifiers and reactions that can be
// played 'when the action/bleed would be successful' can be played after
// it." [LSJ 20081124-2] "He can use his ability on his own bleed to get
// (-1 +2) = +1 bleed": Ozmo's 2 becomes 1, then Andre's own 1 + 2 - 1 + 2
// = 4: seat 1 loses 5.
#[test]
fn andre_leroux_takes_bleed_from_his_allies() {
    let mut e = game(2);
    let andre = e.add_minion(SeatId(0), find_card("Andre LeRoux"), 4, false);
    let ozmo = e.add_minion(SeatId(0), find_card("Ozmo"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let use_it = vamp_use(andre, 0);
    pick(&mut e, bleed(ozmo));
    let req = offering(&mut e, use_it);
    assert_eq!(req.window, WindowLabel::ActionWouldBeSuccessful);
    e.choose(index_of(&req, use_it));
    let req = request(&mut e);
    assert!(!req.options.contains(&use_it), "once a bleed");
    pick_when(&mut e, bleed(andre));
    pick_when(&mut e, use_it);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 1 - 4);
}

// [PIB 20150612] Andre LeRoux: "His ability cannot be used after Spying
// Mission. If his ability reduces a bleed to zero, it is no longer
// successful and Spying Mission cannot be used afterwards": after a
// Spying Mission [OBF] on the bleed, he is not offered.
#[test]
fn andre_leroux_not_after_a_spying_mission() {
    let mut e = game(2);
    let andre = e.add_minion(SeatId(0), find_card("Andre LeRoux"), 4, false);
    let spy = e.add_minion(SeatId(0), vampire_with("OBF", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Spying Mission"]);
    pick(&mut e, bleed(spy));
    pick_when(&mut e, play(0, 1, spy));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        vamp_use(andre, 0),
    );
}

// Black Annis: "can enter combat with any minion controlled by your
// predator or prey as a Ⓓ action. Other non-hunt actions cost her an
// additional blood. +1 strength. +1 stealth." Her bleed costs 1 blood, her
// hunt and her own action nothing.
#[test]
fn black_annis_pays_for_other_actions() {
    for (kind, cost) in [(ActionKind::Bleed, 1u8), (ActionKind::Hunt, 0)] {
        let mut e = game(2);
        let b = e.add_minion(SeatId(0), find_card("Black Annis"), 3, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        pick(&mut e, action(b, kind));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let hunt = if kind == ActionKind::Hunt { 1 } else { 0 };
        assert_eq!(blood(&e, b), 3 - cost + hunt, "{kind:?}");
    }
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Black Annis"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, vamp_action(b, 0, Target::Minion(z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // No toll on her own action; the Ventrue's hand strike costs a blood.
    assert_eq!(blood(&e, b), 2, "her own action costs nothing more");
}

// Bleeding the Vine: "During your unlock phase, lock this card or burn 1
// pool. You may lock this card to cancel a non-out-of-turn master card
// played by another Methuselah as that card is played. That card has no
// effect (no cost is paid), and that Methuselah gains another master phase
// action." Seat 1's Poacher's Hunting Ground canceled: seat 1 may play
// again; seat 0's unlock phase: 1 pool instead of the lock.
#[test]
fn bleeding_the_vine_cancels_a_master_card() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let vine = in_play(&mut e, 0, "Bleeding the Vine", None);
    hand(
        &mut e,
        1,
        &["Poacher's Hunting Ground", "Poacher's Hunting Ground"],
    );
    let cancel = use_at(vine, 0, None, Target::None);
    pick_when(&mut e, master_play(0));
    let req = offering(&mut e, cancel);
    assert_eq!(req.seat, SeatId(0));
    e.choose(index_of(&req, cancel));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::MasterPhase)
    );
    assert!(e.state.cards[vine.index()].locked);
    assert!(
        req.options.contains(&master_play(0)),
        "another master phase action"
    );
    let ash = find_library("Poacher's Hunting Ground");
    assert!(e.state.seats[1].ash.contains(&ash));

    // Seat 0's unlock phase: burn 1 pool rather than lock it.
    let req = until(&mut e, SeatId(0), WindowLabel::RiderTerms);
    let before = pool(&e)[0];
    e.choose(index_of(&req, Choice::BurnPool));
    request(&mut e);
    assert_eq!(pool(&e)[0], before - 1);
    assert!(!e.state.cards[vine.index()].locked);
}

// Enkidu, The Noah: "Enkidu can enter combat with any minion as a Ⓓ
// action. If Enkidu successfully performs an action to employ a retainer,
// he unlocks at the end of the turn. He cannot have or use equipment. +2
// strength." [LSJ 20050315] [LSJ 20010210] "He cannot attempt an action to
// equip nor strike to steal an equipment": a .44 Magnum in hand offers no
// equip; Jackie Therman employed, he is unlocked as seat 1's turn begins.
#[test]
fn enkidu_bears_no_equipment_and_unlocks_after_employing() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Enkidu, The Noah"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &[".44 Magnum", "Jackie Therman"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, k)), "no equip");
    pick(&mut e, play(1, 0, k));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[k.index()].locked);
}

// [LSJ 19980831] Heidelberg Castle, Germany: "Cannot be used to put a card
// on a vampire who is prohibited to have it (eg. an equipment on Enkidu)":
// the .44 Magnum of seat 0's Brujah cannot move to Enkidu.
#[test]
fn heidelberg_castle_cannot_arm_enkidu() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Enkidu, The Noah"), 6, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, ".44 Magnum", Some(b));
    let castle = in_play(&mut e, 0, "Heidelberg Castle, Germany", None);
    let req = request(&mut e);
    let moves_to_enkidu = req.options.iter().any(|o| {
        matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), target: Target::Minion(t), .. }
            if *c == castle && *t == k)
    });
    let _ = moves_to_enkidu;
    assert!(!e.state.can_hold(k, find_library(".44 Magnum"), &[]));
}

// Jackie Therman: "This minion gets an optional maneuver each combat." Her
// employer maneuvers once in a combat.
#[test]
fn jackie_therman_gives_a_maneuver() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    let j = in_play(&mut e, 0, "Jackie Therman", Some(a));
    blocked_bleed(&mut e, a, z);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&use_at(j, 0, None, Target::None)));
}

// New Blood: "As New Blood enters play during your influence phase, choose
// any circle and draw 1 card from your crypt." [LSJ 20060620] "You can
// draw more cards than are in the library or crypt (even from an empty
// library or crypt)": moved out, a crypt card comes to the uncontrolled
// region; from an empty crypt, nothing.
#[test]
fn new_blood_draws_a_crypt_card_as_he_enters() {
    for crypt in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let nb = find_card("New Blood");
        e.state.seats[0].uncontrolled = vec![slot(nb, CRYPT[nb as usize].capacity)];
        e.state.seats[0].crypt = if crypt {
            vec![plain("Tremere", 0)]
        } else {
            vec![]
        }
        .into();
        to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
        pick(&mut e, Choice::InfluenceOut { slot: 0 });
        request(&mut e);
        assert_eq!(e.state.seats[0].uncontrolled.len(), crypt as usize);
        assert!(e
            .state
            .minions
            .iter()
            .any(|m| m.in_play && m.crypt_card() == Some(nb)));
    }
}

// Shadow Boxing: "[OBF][POT] Only usable after a combat resulting from a
// block. Not usable if the blocking minion is ready. This vampire burns 1
// blood to continue the action as if unblocked." [ANK 20230314] "Is used
// after combat effectively ends", [ANK 20210226] "The burn blood effect is
// not reduced" and "The card cannot be played if the minion cannot afford
// to burn the blood": the blocker sent to torpor by Coma, the bleed goes
// on for a blood; with the blocker ready, not offered.
#[test]
fn shadow_boxing_continues_past_a_downed_blocker() {
    for downed in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Erebus"), 6, false);
        superior(&mut e, a, "obf");
        superior(&mut e, a, "pot");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
        hand(&mut e, 0, &["Coma", "Shadow Boxing"]);
        blocked_bleed(&mut e, a, z);
        if downed {
            pick_when(&mut e, play(0, 0, a));
            // Coma played, Shadow Boxing is the card left in hand.
            pick_when(&mut e, play(0, 2, a));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(pool(&e)[1], 10 - 1, "bled");
        } else {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(1, 2, a));
        }
    }
}

// Shadow Boxing [ANK 20200114]: "[OBF][POT] Cannot be used by a Slave
// that entered combat instead of the acting minion." Antonio d'Erlette
// (given superior [OBF][POT]: no Tremere antitribu of the pool has
// Obfuscate) bleeds, blocked by an empty vampire: Tupdog takes the combat and sends
// the blocker to torpor — Shadow Boxing not offered to Antonio; Antonio
// fighting himself, it is, and the bleed goes on.
#[test]
fn shadow_boxing_not_after_a_slave_took_the_combat() {
    for slave in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Antonio d'Erlette"), 4, false);
        superior(&mut e, a, "obf");
        superior(&mut e, a, "pot");
        let t = e.add_minion(SeatId(0), find_card("Tupdog"), 1, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
        hand(&mut e, 0, &["Shadow Boxing"]);
        blocked_bleed(&mut e, a, z);
        if slave {
            pick_when(
                &mut e,
                Choice::UseAbility {
                    source: AbilitySource::Vampire(t),
                    ability: 1,
                    actor: None,
                    target: Target::None,
                },
            );
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 2, a));
            assert!(e.state.minions[t.index()].locked);
            assert_eq!(pool(&e)[1], 10);
        } else {
            pick_when(&mut e, play(0, 2, a));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(pool(&e)[1], 10 - 1, "bled");
        }
        assert!(e.state.minions[z.index()].in_torpor);
    }
}

// The Red List (rulebook "Traits"): "Any Methuselah may use a master phase
// action to mark a Red List minion for the current turn. Any ready vampire
// they control may enter combat with a marked Red List minion as a +1
// stealth Ⓓ action that costs 1 blood. Each vampire can perform this action
// only once each turn." Seat 1 marks Enkidu, its vampire enters combat with
// him for a blood.
#[test]
fn the_red_list_marks_enkidu_for_a_rush() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Enkidu, The Noah"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Academic Hunting Ground"]);
    let req = until(&mut e, SeatId(1), WindowLabel::MasterPhase);
    assert!(req.options.contains(&Choice::ChooseMinion(k)));
    e.choose(index_of(&req, Choice::ChooseMinion(k)));
    let hunt = Choice::TakeAction {
        minion: v,
        action: ActionKind::RedListHunt,
        target: Some(k),
    };
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.choose(index_of(&req, hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // The combat fought: Enkidu took the Ventrue's 1, the Ventrue paid 1
    // blood and took his 3.
    assert_eq!(blood(&e, k), 5);
    assert!(blood(&e, v) == 0 || e.state.minions[v.index()].in_torpor);
}

// ---- Step 9: Blood Puppy, Eugene, Helena, Ira Rivers, Joaquin Murietta,
// ---- Khalu, Millicent Smith, Quicken Sight, Sideslip, Yvette

// Blood Puppy: "Move 3 blood from the blood bank to the Blood Puppy.
// During your unlock phase, you can move a blood from the Puppy to your
// pool, move a blood from the blood bank to the Puppy, or burn the Puppy
// to move all its blood to your pool." One of the three each unlock phase:
// a blood to the pool, then no other.
#[test]
fn blood_puppy_feeds_the_pool() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let p = in_play(&mut e, 0, "Blood Puppy", None);
    e.state.cards[p.index()].counters = 3;
    let used = next_turn_taking(
        &mut e,
        SeatId(0),
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: 0, .. } if *c == p),
    );
    assert_eq!(used, 1, "one of the three");
    assert_eq!(e.state.cards[p.index()].counters, 2);
    assert_eq!(pool(&e)[0], 11);
}

// Eugene: "Once each combat, Eugene can strike: dodge."
#[test]
fn eugene_dodges_once_a_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Eugene"), 8, false);
    blocked_bleed(&mut e, a, g);
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(req.options.contains(&vamp_use(g, 0)));
}

// Helena: "If Helena is locked and ready, she can burn a blood to be able
// to attempt to block and/or play reaction cards that require Auspex [aus]
// as if unlocked for the remainder of the action." Locked, she pays a
// blood and blocks seat 0's bleed.
#[test]
fn helena_blocks_while_locked() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let h = e.add_minion(SeatId(1), find_card("Helena"), 6, false);
    e.state.minions[h.index()].locked = true;
    pick(&mut e, bleed(a));
    pick_when(&mut e, vamp_use(h, 0));
    let req = offering(&mut e, block(h));
    assert_eq!(req.seat, SeatId(1));
    assert_eq!(blood(&e, h), 5);
}

// Ira Rivers: "Ira gets +1 intercept when attempting to block political
// actions. When Ira plays a card that requires Auspex [aus], you may draw
// an additional card from your library (discard down to your hand size
// afterward)." +1 against a political action, none against a bleed; his
// Quicken Sight offers the extra draw.
#[test]
fn ira_rivers_watches_politics_and_draws_on_auspex() {
    let mut e = game(2);
    let ira = e.add_minion(SeatId(1), find_card("Ira Rivers"), 4, false);
    let at = |e: &Engine, kind| {
        e.state.printed_mod(
            ira,
            Stat::Intercept,
            PrintedCtx {
                kind: Some(kind),
                ..PrintedCtx::default()
            },
        )
    };
    assert_eq!(
        (at(&e, ActionKind::Political), at(&e, ActionKind::Bleed)),
        (1, 0)
    );

    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    hand(&mut e, 1, &["Quicken Sight"]);
    fill_hand(&mut e, 1);
    e.state.seats[1].library = vec![find_library("Bonding"); 5].into();
    in_play(&mut e, 0, "Sport Bike", Some(a));
    let _ = a;
    e.state.minions[a.index()].blood = 4;
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, block(ira));
    pick_when(&mut e, play(0, 0, ira));
    let req = request(&mut e);
    assert!(
        req.options.contains(&Choice::DrawLibrary),
        "{:?}",
        req.options
    );
}

// Joaquin Murietta: "At long range, his strikes with guns are at +1
// damage." [RTR 19970630] [TOM 19960225] "Additional damage inherits all
// of the properties of the base damage." [RTR 19960221] "Adding damage to
// a strike that does not deal damage has no effect": his .44 Magnum at
// long range: 3 damage.
#[test]
fn joaquin_murietta_shoots_harder_at_long_range() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let j = e.add_minion(SeatId(1), find_card("Joaquin Murietta"), 8, false);
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(j));
    blocked_bleed(&mut e, a, j);
    // The gun's maneuver commits its strike (long range).
    pick_when(&mut e, use_at(gun, 1, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 3);
}

// Joaquin Murietta: "an optional additional strike (limited) in the first
// round of combat": offered then.
#[test]
fn joaquin_murietta_strikes_again_in_the_first_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let j = e.add_minion(SeatId(1), find_card("Joaquin Murietta"), 8, false);
    blocked_bleed(&mut e, a, j);
    let req = offering(&mut e, vamp_use(j, 0));
    assert_eq!(e.combat().expect("combat").round, 1);
    let _ = req;
}

// Khalu: "Khalu may burn 5 blood to cancel a minion card played by your
// predator or prey as it is played (no cost is paid)." [LSJ 20061207]
// "The cancelation must be played immediately": seat 0's Bum's Rush
// canceled, Khalu down 5 blood, no action.
#[test]
fn khalu_cancels_a_minion_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let k = e.add_minion(SeatId(1), find_card("Khalu"), 7, false);
    hand(&mut e, 0, &["Bum's Rush"]);
    pick(&mut e, play_at(0, 0, a, Target::Minion(k)));
    let cancel = Choice::UseAbility {
        source: AbilitySource::Vampire(k),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::CardAsPlayed);
    assert!(req.options.contains(&cancel), "{:?}", req.options);
    e.choose(index_of(&req, cancel));
    request(&mut e);
    assert_eq!(blood(&e, k), 2);
    assert!(e.action().is_none());
}

// Millicent Smith: "If any of your vampires are blocked, burn Millicent
// and your acting vampire (without combat). During your discard phase,
// your predator takes control of Millicent." [ANK 20220116] [ANK 20211207]
// [ANK 20210627] [RTR 19991206] [LSJ 20091125] "If the action is blocked,
// all effects that would be triggered by a successful block must be
// applied": blocked, both burn, no combat; unblocked to the
// discard phase, she goes to seat 1.
#[test]
fn millicent_smith_burns_the_blocked_hunter() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let m = in_play(&mut e, 0, "Millicent Smith, Puritan Vampire Hunter", None);
    blocked_bleed(&mut e, a, z);
    request(&mut e);
    assert!(!e.state.minions[a.index()].in_play);
    assert!(!e.state.cards[m.index()].present());
    assert!(e.combat().is_none());

    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let m = in_play(&mut e, 0, "Millicent Smith, Puritan Vampire Hunter", None);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.card_controller(m), SeatId(1));
}

// Quicken Sight: "[aus][cel] +1 intercept, with an optional maneuver in the
// resulting combat if this vampire successfully blocks." Ira blocks a
// stealthy hunt with it and maneuvers.
#[test]
fn quicken_sight_blocks_and_maneuvers() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ira = e.add_minion(SeatId(1), find_card("Ira Rivers"), 6, false);
    hand(&mut e, 1, &["Quicken Sight"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, block(ira));
    pick_when(&mut e, play(0, 0, ira));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&Choice::UseManeuver));
}

// Sideslip: "[CEL] Prevent 1 damage. A vampire can play only one Sideslip
// at superior each round." [LSJ 20001114] "Cannot be used if there is no
// damage to prevent": against a hand strike, one prevents, the second is
// not offered.
#[test]
fn sideslip_prevents_once_a_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Eugene"), 8, false);
    hand(&mut e, 1, &["Sideslip", "Sideslip"]);
    blocked_bleed(&mut e, a, g);
    pick_when(&mut e, play(0, 1, g));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, g));
}

// ---- Step 10: Abiku, Base Hunting Ground, Cesewayo, Homa, Massassi, Mbare
// ---- Market, Tatu Sawosa, Uzoma

// Abiku: "+1 stealth when hunting." Tatu Sawosa: "+1 strength."
#[test]
fn abiku_hunts_quietly_and_tatu_hits_hard() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Abiku"), 4, false);
    let t = e.add_minion(SeatId(0), find_card("Tatu Sawosa"), 4, false);
    let hunt = PrintedCtx {
        kind: Some(ActionKind::Hunt),
        ..PrintedCtx::default()
    };
    assert_eq!(e.state.printed_mod(a, Stat::Stealth, hunt), 1);
    assert_eq!(
        e.state
            .printed_mod(t, Stat::Strength, PrintedCtx::default()),
        1
    );
}

// Base Hunting Ground: "During your unlock phase, you may move 1 blood from
// the blood bank to a ready vampire you control." (Osebo.)
#[test]
fn base_hunting_ground_feeds_a_vampire() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Homa"), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let hg = in_play(&mut e, 0, "Base Hunting Ground", None);
    let used = next_turn_taking(&mut e, SeatId(0), |o| {
        *o == use_at(hg, 0, None, Target::Minion(v))
    });
    assert_eq!((used, blood(&e, v)), (1, 2));
}

// Cesewayo: "Once each action, Cesewayo may burn 1 blood to get +1
// intercept." Attempting to block a +1 stealth hunt: once.
#[test]
fn cesewayo_buys_intercept_once_an_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let c = e.add_minion(SeatId(1), find_card("Cesewayo"), 4, false);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, block(c));
    pick_when(&mut e, vamp_use(c, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Blocked: a blood for the intercept, another to the Brujah's hand
    // strike.
    assert_eq!(blood(&e, c), 2);
}

// Massassi: "Massassi can enter combat with a ready vampire as a Ⓓ action.
// If that action is successful, she gets an optional maneuver in the first
// round of the resulting combat."
#[test]
fn massassi_rushes_with_a_maneuver() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Massassi"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, vamp_action(m, 0, Target::Minion(z)));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&Choice::UseManeuver));
}

// Mbare Market, Harare: "Lock to add 1 blood from the blood bank to a
// Laibon in your uncontrolled region." Fish (Laibon) gets it, a Ventrue is
// no target.
#[test]
fn mbare_market_feeds_an_uncontrolled_laibon() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].uncontrolled = vec![slot(plain("Ventrue", 1), 0), slot(find_card("Fish"), 0)];
    let mk = in_play(&mut e, 0, "Mbare Market, Harare", None);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&use_at(mk, 0, None, Target::Uncontrolled(0))));
    e.choose(index_of(&req, use_at(mk, 0, None, Target::Uncontrolled(1))));
    request(&mut e);
    assert_eq!(e.state.seats[0].uncontrolled[1].counters, 1);
    assert!(e.state.cards[mk.index()].locked);
}

// ---- Step 11: Apache Jones, Cassandra Langely, Dementation, Deny, Eddie
// ---- Gaines, Effective Management, Jackie, Marta, Midget, Persia, The
// ---- Bitter and Sweet Story, Uncle George

// Dementation: "This vampire gets +1 level of Dementation [dem] and +1
// capacity." A Brujah plays Deny [dem] as a press once it has it.
#[test]
fn dementation_grants_the_discipline() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Dementation", Some(b));
    assert!(e.state.has_discipline(b, "dem"));
}

// Deny: "[dem] [COMBAT] Press. [DEM] [ACTION MODIFIER] +1 stealth." Erebus
// presses with it.
#[test]
fn deny_presses() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Erebus"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    hand(&mut e, 0, &["Deny"]);
    blocked_bleed(&mut e, a, z);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert!(req.options.contains(&play(0, 0, a)));
}

// Eddie Gaines: "If Eddie successfully hunts, he does not unlock as normal
// during his next unlock phase."
#[test]
fn eddie_gaines_naps_after_a_hunt() {
    let mut e = game(2);
    let ed = e.add_minion(SeatId(0), find_card("Eddie Gaines"), 0, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    // Bloodless, he must hunt: the hunt is taken at once.
    decline_futile(&mut e);
    assert_eq!(blood(&e, ed), 1, "hunted");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[ed.index()].locked);
}

// Effective Management: "Move the top card from your crypt to your
// uncontrolled region." [RTR 20000501] [LSJ 20070411-2] "Cannot be played
// when the target crypt is empty."
#[test]
fn effective_management_moves_the_crypt_top() {
    for crypt in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.seats[0].crypt = if crypt {
            vec![plain("Tremere", 0)]
        } else {
            vec![]
        }
        .into();
        hand(&mut e, 0, &["Effective Management"]);
        e.at_master_phase();
        let req = request(&mut e);
        assert_eq!(req.options.contains(&master_play(0)), crypt);
        if crypt {
            e.choose(index_of(&req, master_play(0)));
            request(&mut e);
            assert_eq!(e.state.seats[0].uncontrolled.len(), 1);
        }
    }
}

// Marta: "After Marta diablerizes an older vampire, she can gain 1 blood."
// An older victim: 1 more; a younger one: none.
#[test]
fn marta_feeds_on_elders() {
    for (cap, extra) in [(8u8, 1u8), (2, 0)] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card("Marta"), 1, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let victim = CRYPT
            .iter()
            .position(|c| c.capacity == cap && c.title == zvtes::crypt::Title::None)
            .expect("a victim") as u16;
        let v = e.add_minion(SeatId(1), victim, 1, true);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let diablerie = Choice::TakeAction {
            minion: m,
            action: ActionKind::Diablerie,
            target: Some(v),
        };
        pick(&mut e, diablerie);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, m), 1 + 1 + extra, "capacity {cap}");
    }
}

// Midget: "While Midget is acting or is in combat, animal retainers lose
// their abilities." In his combat his own Owl Companion opens no hand.
#[test]
fn midget_silences_animal_retainers() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Midget"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    in_play(&mut e, 0, "Owl Companion", Some(m));
    hand(&mut e, 1, &["Majesty"]);
    blocked_bleed(&mut e, m, z);
    until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(!e.state.hand_visible_to(SeatId(1), SeatId(0)));
}

// Persia: "Once each action, Persia can burn 1 blood to get +1 intercept."
#[test]
fn persia_buys_intercept() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(
        SeatId(1),
        find_card("Persia, The Beautiful Statue"),
        4,
        false,
    );
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, block(p));
    let req = request(&mut e);
    assert!(req.options.contains(&vamp_use(p, 0)));
}

// The Bitter and Sweet Story: "Each Methuselah gets +2 hand size for each
// victory point he or she has." [ANK 20230527-2] "When the controller is
// ousted, their predator does not draw cards": with 1 VP, 9 cards; the
// event's controller ousted, it goes, and the predator's victory point
// draws nothing.
#[test]
fn the_bitter_and_sweet_story_rewards_victory_points() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let v = e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 0, "Bitter and Sweet Story, The", None);
    e.state.seats[1].half_vps = 2;
    assert_eq!(e.state.hand_size(SeatId(1)), 9);
    // Seat 2 (seat 0's predator) ousts seat 0.
    e.state.seats[0].pool = 1;
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    pick(&mut e, bleed(v));
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].ousted);
    assert_eq!(e.state.seats[2].half_vps, 2);
    assert_eq!(e.state.hand_size(SeatId(2)), 7);
}
