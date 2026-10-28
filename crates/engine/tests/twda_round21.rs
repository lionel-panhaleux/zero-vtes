//! Legacy cards, round 21 (`ADDED_BATCHES`, slice 16ag): the last step
//! paying in recent TWDA decks (Faerie Wards, four Kiasyd) and Nights of
//! Reckoning completed. Readings in rules-qa Batch 102. Windows with a
//! single option are elided, so the scripts only stop where a real
//! decision exists.

use zvtes::cards::{AbilitySource, Stat, Target};
use zvtes::crypt::CRYPT;
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

/// The `nth` untitled vampire of `clan`.
fn of_clan(clan: &str, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(_, c)| c.clan == clan && c.title == zvtes::crypt::Title::None)
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no untitled {clan}"))
}

/// A vampire's own `ability` at `target`.
fn own_at(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: Some(m),
        target,
    }
}

/// A vampire's own ability used during an action (no actor named).
fn own(m: MinionId, ability: u8) -> Choice {
    own_on(m, ability, Target::None)
}

/// A vampire's own ability at `target`, no actor named.
fn own_on(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: None,
        target,
    }
}

/// Imbued `name` in play for `seat`, its cost as its life.
fn imbued(e: &mut Engine, seat: u8, name: &str) -> MinionId {
    let c = find_card(name);
    assert!(CRYPT[c as usize].imbued, "{name} is an imbued");
    e.add_minion(SeatId(seat), c, CRYPT[c as usize].capacity, false)
}

/// A card already in play for `seat`, on `host` if given: its id.
fn in_play(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>) -> CardId {
    e.state
        .cards
        .push(CardInPlay::new(find_library(name), SeatId(seat), host, 0));
    CardId(e.state.cards.len() as u32 - 1)
}

/// An equipment already on `host`, with its layers (no play).
fn equipped(e: &mut Engine, name: &str, host: MinionId) -> CardId {
    let seat = e.state.minions[host.index()].owner.0;
    let id = in_play(e, seat, name, Some(host));
    for effect in zvtes::cards::lookup(find_library(name))
        .expect("definition")
        .modes[0]
        .effects
    {
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

/// "Put this card on": the card itself in play, no stand-in, its ash heap
/// without it.
fn placed(e: &Engine, c: CardId) {
    let card = &e.state.cards[c.index()];
    assert!(card.in_play && !card.effect_only, "the card itself");
    assert!(
        !e.state.seats.iter().any(|s| s.ash.contains(&card.card)),
        "not in an ash heap"
    );
}

/// Hand card `card` played in `mode` by `actor` at `target`.
fn play_at(card: u8, mode: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor,
        target,
    }
}

/// `card`'s `ability` used by its controller at `target`.
fn use_on(card: CardId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(card),
        ability,
        actor: None,
        target,
    }
}

// ---- The Kiasyd ---------------------------------------------------------------

// Bartholomew: "If Bartholomew is unlocked at the beginning of your turn,
// you gain 1 pool from the blood bank." Read before the unlock phase
// unlocks him: locked as the turn begins, no pool.
#[test]
fn bartholomew_gains_a_pool_if_unlocked_as_the_turn_begins() {
    for locked in [false, true] {
        let mut e = game(2);
        let b = e.add_minion(SeatId(0), find_card("Bartholomew"), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        e.state.minions[b.index()].locked = locked;
        let before = e.state.seats[0].pool;
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.minions[b.index()].locked, "unlocked as normal");
        assert_eq!(e.state.seats[0].pool, before + i32::from(!locked));
    }
}

// Julia Prima: "Julia can bleed any Methuselah as a Ⓓ action. Once each
// action, Julia can burn 1 blood to get +1 bleed." Her predator bled, for
// 2 with the blood burned; the +1 once.
// Rulings: [ANK 20231124] [LSJ 19970718].
#[test]
fn julia_prima_bleeds_any_methuselah_and_burns_blood_for_bleed() {
    let mut e = game(3);
    let julia = e.add_minion(SeatId(0), find_card("Julia Prima"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    pick(&mut e, own_at(julia, 0, Target::Seat(SeatId(2))));
    pick_when(&mut e, own(julia, 1));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, own(julia, 1));
    assert_eq!(e.state.seats[2].pool, 10 - 2);
    assert_eq!(e.state.seats[1].pool, 10);
    assert_eq!(blood(&e, julia), 5 - 1);
}

// Kassiym Malikhair: "During your unlock phase, you can move 1 blood from
// Kassiym to any ready Pander." Another Methuselah's Pander.
#[test]
fn kassiym_malikhair_moves_a_blood_to_any_ready_pander() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Kassiym Malikhair"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let pander = e.add_minion(SeatId(1), of_clan("Pander", 0), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick_when(&mut e, own_on(k, 0, Target::Minion(pander)));
    request(&mut e);
    assert_eq!((blood(&e, k), blood(&e, pander)), (3, 3));
}

// Marconius: "Cards requiring Mytherceria [myt] cost Marconius 1 less
// blood. +1 strength." Faerie Wards [MYT] (2 blood) for 1; the
// acting minion does not unlock as normal after.
#[test]
fn marconius_pays_one_less_for_mytherceria_and_strikes_harder() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Marconius"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    assert_eq!(
        e.state
            .printed_mod(m, Stat::Strength, PrintedCtx::default()),
        1
    );
    hand(&mut e, 0, &["Faerie Wards"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    pick_when(&mut e, play(0, 2, m));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, m), 5 - 1, "2 blood, 1 off");
    assert!(e.state.minions[m.index()].locked);
    assert_eq!(e.state.seats[0].pool, 10, "the action failed");
    // Seat 1's next unlock phase: its bleeder stays locked.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].locked, "no unlock as normal");
}

// ---- Faerie Wards -------------------------------------------------------------

// Faerie Wards: "Only usable if an ally or younger vampire is taking an
// action directed at you (or a card you control). [aus] +2 intercept.
// [myt] The action fails. Lock this reacting vampire." Not against an
// older vampire; [aus] lets a vampire without intercept block a +1 stealth
// bleed; [myt] fails it, the reacting vampire locked, the acting one
// unlocking as normal.
// Rulings: [LSJ 20011204-3] [PIB 20130704].
#[test]
fn faerie_wards_intercepts_or_fails_a_younger_vampires_action() {
    // Julia Prima (capacity 7: aus, myt) reacting.
    let old = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| c.capacity >= 9 && zvtes::cards::vampire(i as u16).is_none())
        .expect("an elder") as u16;
    for (acting, mode, offered) in [(plain("Ventrue", 0), 1u8, true), (old, 1, false)] {
        let mut e = game(2);
        let r = e.add_minion(SeatId(0), find_card("Julia Prima"), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), acting, 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Faerie Wards"]);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        pick(&mut e, bleed(z));
        let fails = play(0, mode, r);
        if !offered {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, fails);
            continue;
        }
        pick_when(&mut e, fails);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[0].pool, 10, "failed");
        assert!(e.state.minions[r.index()].locked);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(!e.state.minions[z.index()].locked, "unlocked as normal");
    }
}

// ---- Nights of Reckoning ----------------------------------------------------

// Antithesis: "[jud] +3 stealth action. Ⓓ Put this card on a non-Sabbat
// vampire with capacity less than 8. The vampire with this card cannot
// hunt. If he or she must hunt, he or she locks and goes to torpor instead.
// Burn this card during this vampire's discard phase." Not on a Sabbat or
// a capacity 8 vampire; the bloodless one, made to hunt, locked and in
// torpor; the card burned in its controller's discard phase.
#[test]
fn antithesis_sends_a_vampire_that_must_hunt_to_torpor_locked() {
    let mut e = game(2);
    let jack = imbued(&mut e, 0, "Jack \"Hannibal137\" Harmon");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    // Pulled Fangs already on it (no lock): Antithesis's lock still holds.
    equipped(&mut e, "Pulled Fangs", v);
    let big = CRYPT
        .iter()
        .position(|c| c.capacity == 8 && c.sect == Some(zvtes::crypt::Sect::Camarilla))
        .expect("a capacity 8 Camarilla") as u16;
    let elder = e.add_minion(SeatId(1), big, 4, false);
    let sabbat = CRYPT
        .iter()
        .position(|c| c.capacity <= 7 && c.sect == Some(zvtes::crypt::Sect::Sabbat))
        .expect("a Sabbat vampire") as u16;
    let s = e.add_minion(SeatId(1), sabbat, 4, false);
    hand(&mut e, 0, &["Antithesis"]);
    let at = |m: MinionId| play_at(0, 0, Some(jack), Target::Minion(m));
    let req = request(&mut e);
    assert!(req.options.contains(&at(v)), "{:?}", req.options);
    assert!(!req.options.contains(&at(elder)));
    assert!(!req.options.contains(&at(s)));
    pick(&mut e, at(v));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = (0..e.state.cards.len())
        .map(|i| CardId(i as u32))
        .find(|&c| e.state.cards[c.index()].card == find_library("Antithesis"))
        .expect("Antithesis in play");
    assert_eq!(e.state.cards[card.index()].host, Some(v));
    placed(&e, card);
    let m = &e.state.minions[v.index()];
    assert!(m.in_torpor && m.locked, "made to hunt: locked, in torpor");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !e.state.cards[card.index()].in_play,
        "burned in its discard phase"
    );
}

// The Church of Vindicated Faith: "Requires a ready imbued. When an imbued
// successfully performs an action, lock this card to move 1 blood from the
// blood bank to an imbued in your uncontrolled region." After an imbued's
// successful bleed; not after a vampire's.
#[test]
fn the_church_feeds_an_uncontrolled_imbued_after_an_imbued_succeeds() {
    for by_imbued in [true, false] {
        let mut e = game(2);
        let i = imbued(&mut e, 0, "Jack \"Hannibal137\" Harmon");
        let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let church = in_play(&mut e, 0, "Church of Vindicated Faith, The", None);
        e.state.seats[0]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card: find_card("Xian \"DziDzat155\" Quan"),
                copy: Default::default(),
                counters: 1,
                banished: None,
            });
        let feed = use_on(church, 0, Target::Uncontrolled(0));
        pick(&mut e, bleed(if by_imbued { i } else { v }));
        if by_imbued {
            pick_when(&mut e, feed);
            request(&mut e);
            assert_eq!(e.state.seats[0].uncontrolled[0].counters, 2);
            assert!(e.state.cards[church.index()].locked);
        } else {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, feed);
        }
    }
}

// Break the Code: "Gehenna. Imbued get -1 intercept when attempting to
// block monsters. A ready monster can enter combat with an imbued as a Ⓓ
// action." An imbued's block of a vampire's bleed (0 stealth) fails with
// it in play; a ready vampire enters combat with another Methuselah's
// imbued.
#[test]
fn break_the_code_weakens_imbued_blocks_and_lets_monsters_attack_them() {
    for event in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let i = imbued(&mut e, 0, "Jack \"Hannibal137\" Harmon");
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        if event {
            in_play(&mut e, 0, "Break the Code", None);
            let layer = zvtes::cards::Layer {
                scope: zvtes::cards::Scope::InPlay,
                modifier: zvtes::cards::Modifier::ImbuedInterceptVsMonstersMod(-1),
                source: CardId(e.state.cards.len() as u32 - 1),
            };
            e.state.layers.push(layer);
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        pick(&mut e, bleed(z));
        pick_when(&mut e, block(i));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.seats[0].pool,
            if event { 10 - 1 } else { 10 },
            "event {event}"
        );
    }
    // The granted action: seat 0's vampire at seat 1's imbued.
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let i = imbued(&mut e, 1, "Jack \"Hannibal137\" Harmon");
    let code = in_play(&mut e, 1, "Break the Code", None);
    let attack = Choice::UseAbility {
        source: AbilitySource::Card(code),
        ability: 0,
        actor: Some(v),
        target: Target::Minion(i),
    };
    pick_when(&mut e, attack);
    until(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    let c = e.combat().expect("combat");
    assert_eq!((c.sides[0].minion, c.sides[1].minion), (v, i));
}

// Punish: "[ACTION] +1 stealth action. Ⓓ Put this card on a vampire. You
// still control this card. The vampire with this card cannot gain blood.
// Any blood he or she gains goes to the blood bank instead. Burn this card
// during your next unlock phase. [COMBAT] As [ACTION] above, but put this
// card on the opposing vampire as a hand strike (at strength damage)
// instead." The vampire hunting gains nothing; the card its player's,
// burned in their unlock phase; as a strike, on the opponent as it lands.
#[test]
fn punish_bars_a_vampire_from_gaining_blood_until_its_players_unlock() {
    let mut e = game(2);
    let marion = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Punish"]);
    pick(&mut e, play_at(0, 0, Some(marion), Target::Minion(v)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = (0..e.state.cards.len())
        .map(|i| CardId(i as u32))
        .find(|&c| e.state.cards[c.index()].card == find_library("Punish"))
        .expect("Punish in play");
    assert_eq!(e.state.cards[card.index()].host, Some(v));
    placed(&e, card);
    assert_eq!(e.state.card_controller(card), SeatId(0), "still yours");
    // Its forced hunt gains nothing.
    request(&mut e);
    assert_eq!(blood(&e, v), 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !e.state.cards[card.index()].in_play,
        "burned in your unlock phase"
    );
    // [COMBAT]: Marion blocked, Punish as her strike.
    let mut e = game(2);
    let marion = imbued(&mut e, 0, "Marion \"Teacher193\" Perks");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Punish"]);
    pick(&mut e, bleed(marion));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play_at(0, 1, Some(marion), Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 1, "a hand strike at strength");
    let on = e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.host == Some(z) && c.card == find_library("Punish"));
    assert!(on, "on the opposing vampire");
    let punish = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.in_play && c.host == Some(z))
            .expect("Punish") as u32,
    );
    placed(&e, punish);
}

// Bond: "[inn] +1 stealth action. Move an incapacitated imbued to his or
// her controller's ready region. That imbued gains a life (not to exceed
// his or her starting amount)." The life is the one gained leaving the
// region, not one more (LSJ 20091118).
// Rulings: [LSJ 20091118].
#[test]
fn bond_moves_an_incapacitated_imbued_back_with_a_life() {
    let mut e = game(2);
    let inez = imbued(&mut e, 0, "Inez \"Nurse216\" Villagrande");
    let jack = imbued(&mut e, 0, "Jack \"Hannibal137\" Harmon");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    {
        let v = &mut e.state.minions[jack.index()];
        v.incapacitated = true;
        v.blood = 0;
    }
    hand(&mut e, 0, &["Bond"]);
    pick(&mut e, play_at(0, 0, Some(inez), Target::Minion(jack)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let v = &e.state.minions[jack.index()];
    assert!(!v.incapacitated && v.ready());
    assert_eq!(v.blood, 1, "one life, as leaving the region gives");
}

// Augur: "[viz] [1 CONVICTION] +1 stealth action. Look at the hands of
// your predator and prey and at the top three cards of your library. This
// acting imbued unlocks at the end of the action." Not offered without a
// conviction; the one held named as announced and burned as it resolves
// (rules-qa 102.1); both hands and the library top looked at; unlocked.
#[test]
fn augur_spends_a_conviction_to_look_and_unlocks() {
    for conviction in [false, true] {
        let mut e = game(3);
        e.notes = Some(Vec::new());
        let paul = imbued(&mut e, 0, "Paul \"Sixofswords29\" Moreton");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
        let held = conviction.then(|| in_play(&mut e, 0, "React with Conviction", Some(paul)));
        hand(&mut e, 0, &["Augur"]);
        let augur = play_at(0, 0, Some(paul), Target::None);
        if !conviction {
            assert!(!request(&mut e).options.contains(&augur));
            continue;
        }
        pick(&mut e, augur);
        // Directed at the predator and the prey (owner, rules-qa 102.11).
        request(&mut e);
        let at = e
            .action()
            .expect("Augur under way")
            .directed_seats(&e.state);
        assert!(
            at.contains(SeatId(1)) && at.contains(SeatId(2)),
            "directed at both"
        );
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.cards[held.unwrap().index()].in_play, "burned");
        assert!(!e.state.minions[paul.index()].locked, "unlocked");
        let looked: Vec<(SeatId, bool)> = e
            .notes
            .as_ref()
            .unwrap()
            .iter()
            .filter_map(|n| match n {
                zvtes::notes::Note::Look(l) if l.viewer == SeatId(0) => Some((l.seat, l.library)),
                _ => None,
            })
            .collect();
        assert!(looked.contains(&(SeatId(1), false)), "{looked:?}");
        assert!(looked.contains(&(SeatId(2), false)), "{looked:?}");
        assert!(looked.contains(&(SeatId(0), true)), "{looked:?}");
    }
}

/// `n` conviction cards on imbued `m`.
fn convictions(e: &mut Engine, m: MinionId, n: usize) -> Vec<CardId> {
    let seat = e.state.minions[m.index()].owner.0;
    (0..n)
        .map(|_| in_play(e, seat, "React with Conviction", Some(m)))
        .collect()
}

/// Seat 0's imbued `name` bleeds, blocked by seat 1's vampire (4 blood):
/// the engine at the combat, the imbued and the vampire.
fn imbued_blocked(
    name: &str,
    convictions_n: usize,
    hand_cards: &[&str],
) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let i = imbued(&mut e, 0, name);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    convictions(&mut e, i, convictions_n);
    hand(&mut e, 0, hand_cards);
    pick(&mut e, bleed(i));
    pick_when(&mut e, block(z));
    (e, i, z)
}

// Muse of Flame: "[ven] +2 stealth action. Ⓓ If this action is successful,
// choose a monster controlled by your prey at random. This imbued enters
// combat with the chosen monster." The prey's one ready vampire.
#[test]
fn muse_of_flame_sends_the_imbued_into_combat_with_a_prey_monster() {
    let mut e = game(3);
    let j = imbued(&mut e, 0, "Jennifer \"Flame61\" Vidisania");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    hand(&mut e, 0, &["Muse of Flame"]);
    pick(&mut e, play_at(0, 0, Some(j), Target::Seat(SeatId(1))));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    // The hand strikes exchanged: 1 each.
    assert_eq!(blood(&e, z), 4 - 1);
    assert_eq!(
        blood(&e, j),
        CRYPT[find_card("Jennifer \"Flame61\" Vidisania") as usize].capacity - 1
    );
}

// Smite: "[ven] [3 CONVICTION] Strike: strength+1 aggravated ranged
// damage. Even if the strike is dodged, burn any electronic equipment on
// either combatant." Its 3 convictions burned; 2 aggravated damage sending
// the blocker to torpor; its IR Goggles burned.
#[test]
fn smite_strikes_aggravated_at_range_and_burns_electronics() {
    let mut e = game(2);
    let john = imbued(&mut e, 0, "John \"Cop90\" O'Malley");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let held = convictions(&mut e, john, 3);
    let goggles = in_play(&mut e, 1, "IR Goggles", Some(z));
    hand(&mut e, 0, &["Smite"]);
    pick(&mut e, bleed(john));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play_at(0, 0, Some(john), Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        held.iter().all(|c| !e.state.cards[c.index()].in_play),
        "3 convictions"
    );
    assert!(
        !e.state.cards[goggles.index()].in_play,
        "electronics burned"
    );
    // 2 aggravated damage: 1 blood burned, the other unhealable, torpor.
    assert!(e.state.minions[z.index()].in_torpor, "aggravated");
}

// Imprison: "[jud] [2 CONVICTION] Only usable before range is determined.
// For the remainder of combat, the opposing monster cannot maneuver or
// press to continue, and strikes other than strikes to end combat cost the
// opposing monster an additional blood or life." The blocker's hand strike
// costs it a blood.
// Rulings: [LSJ 20090703].
#[test]
fn imprison_tolls_the_opposing_monsters_strikes() {
    let (mut e, jack, z) = imbued_blocked("Jack \"Hannibal137\" Harmon", 2, &["Imprison"]);
    pick_when(&mut e, play_at(0, 0, Some(jack), Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, z),
        4 - 1 - 1,
        "its toll and the imbued's hand strike"
    );
}

// Inflict: "[mar] [1 CONVICTION] Only usable before range is determined.
// For the remainder of combat, the amount of damage inflicted on this
// imbued by hand strikes, melee weapon strikes, or retainers is also
// inflicted on the striking minion." The blocker's hand strike (1) hits it
// too.
#[test]
fn inflict_reflects_hand_strike_damage_on_the_striker() {
    let (mut e, pedro, z) = imbued_blocked("Pedro Cortez", 1, &["Inflict"]);
    pick_when(&mut e, play_at(0, 0, Some(pedro), Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    // Pedro Cortez: +1 strength.
    assert_eq!(
        blood(&e, z),
        4 - 2 - 1,
        "Pedro's strike and its own, reflected"
    );
    // An aggravated hand strike (Wolf Claws [pro]) reflected as normal
    // damage (owner, rules-qa 102.15): its 3 blood soak Pedro's 2 and the
    // reflected 1, no torpor.
    let mut e = game(2);
    let pedro = imbued(&mut e, 0, "Pedro Cortez");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let g = e.add_minion(SeatId(1), plain("Gangrel", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    convictions(&mut e, pedro, 1);
    hand(&mut e, 0, &["Inflict"]);
    hand(&mut e, 1, &["Wolf Claws"]);
    pick(&mut e, bleed(pedro));
    pick_when(&mut e, block(g));
    pick_when(&mut e, play_at(0, 0, Some(pedro), Target::None));
    pick_when(&mut e, play_at(0, 0, Some(g), Target::None));
    // Before its own turn, where a bloodless vampire hunts at once.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, g), 4 - 1 - 2 - 1);
    assert!(
        !e.state.minions[g.index()].in_torpor,
        "the reflection normal"
    );
}

// Determine: "[viz] [1 CONVICTION] Play when a monster controlled by your
// predator is bleeding you, after blocks are declined. Lock this reacting
// imbued. The monster is now bleeding your predator's predator. Or play
// when a monster controlled by your predator or prey plays an action card.
// Lock this reacting imbued and cancel that action card as it is played
// (no cost is paid, and the monster doesn't lock). That monster cannot
// play the same action card again this turn." The predator's bleed sent
// on; the prey's Bum's Rush canceled, its second copy barred to that
// vampire this turn.
// Rulings: [ANK 20190104] [LSJ 19980212]; [RBK cancel-a-card].
#[test]
fn determine_redirects_a_predators_bleed_or_cancels_an_action_card() {
    // The bleed: seat 2 (seat 0's predator) bleeds seat 0.
    let mut e = game(3);
    let paul = imbued(&mut e, 0, "Paul \"Sixofswords29\" Moreton");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let z = e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 1), 4, false);
    let held = convictions(&mut e, paul, 1);
    hand(&mut e, 0, &["Determine"]);
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    pick_when(&mut e, play_at(0, 0, Some(paul), Target::Seat(SeatId(1))));
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert_eq!((e.state.seats[0].pool, e.state.seats[1].pool), (10, 9));
    assert!(e.state.minions[paul.index()].locked);
    assert!(!e.state.cards[held[0].index()].in_play, "its conviction");
    // The action card: seat 1 (seat 0's prey) plays Bum's Rush.
    let mut e = game(3);
    let paul = imbued(&mut e, 0, "Paul \"Sixofswords29\" Moreton");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    convictions(&mut e, paul, 1);
    hand(&mut e, 0, &["Determine"]);
    hand(&mut e, 1, &["Bum's Rush", "Bum's Rush"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let rush = play_at(0, 0, Some(v), Target::Minion(paul));
    pick(&mut e, rush);
    pick_when(&mut e, play_at(0, 1, Some(paul), Target::None));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[v.index()].locked,
        "the monster does not lock"
    );
    assert!(e.state.minions[paul.index()].locked);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { actor: Some(a), .. } if *a == v)),
        "barred this turn: {:?}",
        req.options
    );
}

// The Rose Foundation: "Lock this location and burn a conviction [1
// CONVICTION] from a ready imbued you control to reduce a bleed against you
// by one or to gain two votes in a referendum." A bleed for 1, reduced to
// none; the imbued's conviction burned, the location locked.
#[test]
fn the_rose_foundation_spends_an_imbueds_conviction_against_a_bleed() {
    let mut e = game(2);
    let jack = imbued(&mut e, 0, "Jack \"Hannibal137\" Harmon");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let held = convictions(&mut e, jack, 1);
    let rose = in_play(&mut e, 0, "Rose Foundation, The", None);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    pick_when(&mut e, use_on(rose, 0, Target::Minion(jack)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10, "bled for none");
    assert!(!e.state.cards[held[0].index()].in_play);
    assert!(e.state.cards[rose.index()].locked);
}

// Vigil: The Thin Line: "Master: unique hunter-list. Put this card in
// play. Lock this card to give a Defender +1 intercept when a monster is
// acting." (Defender.) Jack, a Defender, blocks a +1 stealth hunt with it;
// played only with a ready Defender (the creed as a clan, rules-qa 102.2).
#[test]
fn vigil_gives_a_defender_intercept_against_a_monster() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Vigil: The Thin Line"]);
    e.at_master_phase();
    let vigil = play_at(0, 0, None, Target::None);
    assert!(!request(&mut e).options.contains(&vigil), "no Defender");
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let jack = imbued(&mut e, 0, "Jack \"Hannibal137\" Harmon");
    // Bloodless: its hunt (+1 stealth) taken at once on seat 1's turn.
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Vigil: The Thin Line"]);
    e.at_master_phase();
    pick(&mut e, vigil);
    request(&mut e);
    let card = (0..e.state.cards.len())
        .map(|i| CardId(i as u32))
        .find(|&c| e.state.cards[c.index()].card == find_library("Vigil: The Thin Line"))
        .expect("in play");
    let req = offering(&mut e, block(jack));
    assert_eq!(e.action().map(|a| a.acting), Some(z));
    e.choose(req.options.iter().position(|o| *o == block(jack)).unwrap());
    pick_when(&mut e, use_on(card, 0, Target::None));
    request(&mut e);
    assert!(e.state.cards[card.index()].locked);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    // Another Methuselah's Defender (D11): seat 2's Vigil for seat 0's
    // imbued blocking seat 1's hunt.
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let jack = imbued(&mut e, 0, "Jack \"Hannibal137\" Harmon");
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    let card = in_play(&mut e, 2, "Vigil: The Thin Line", None);
    let req = offering(&mut e, block(jack));
    assert_eq!(e.action().map(|a| a.acting), Some(z));
    e.choose(req.options.iter().position(|o| *o == block(jack)).unwrap());
    pick_when(&mut e, use_on(card, 0, Target::None));
    request(&mut e);
    assert!(e.state.cards[card.index()].locked);
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
}

// The Crusader Sword: "Unique melee weapon. Strength+1 aggravated damage
// each strike. Only usable by a minion with True Faith or an imbued.
// Vampires opposing this minion with True Faith or this imbued cannot use
// maneuvers to go to close range." The imbued's .44 Magnum takes the range
// long: the blocker's Pursuit back to close is barred by the Sword only.
#[test]
fn the_crusader_sword_bars_opposing_vampires_from_closing() {
    for sword in [false, true] {
        let mut e = game(2);
        let i = imbued(&mut e, 0, "Jack \"Hannibal137\" Harmon");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let magnum = in_play(&mut e, 0, ".44 Magnum", Some(i));
        if sword {
            equipped(&mut e, "Crusader Sword, The", i);
        }
        hand(&mut e, 1, &["Pursuit"]);
        pick(&mut e, bleed(i));
        pick_when(&mut e, block(z));
        pick_when(&mut e, use_on(magnum, 1, Target::None));
        let pursuit = play_at(0, 0, Some(z), Target::None);
        if sword {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, pursuit);
        } else {
            pick_when(&mut e, pursuit);
        }
    }
}

// Its strike: 2 aggravated damage from a 1-strength imbued, sending the
// blocker to torpor; a vampire bearing it is not offered the strike (only
// usable by an imbued; equipped by anyone, LSJ 20030607).
// Rulings: [LSJ 20030607].
#[test]
fn the_crusader_sword_strikes_aggravated_for_an_imbued_only() {
    let mut e = game(2);
    let i = imbued(&mut e, 0, "Jack \"Hannibal137\" Harmon");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let sword = equipped(&mut e, "Crusader Sword, The", i);
    let theirs = equipped(&mut e, "Crusader Sword, The", z);
    pick(&mut e, bleed(i));
    pick_when(&mut e, block(z));
    pick_when(&mut e, use_on(sword, 0, Target::None));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        use_on(theirs, 0, Target::None),
    );
    assert!(e.state.minions[z.index()].in_torpor, "aggravated");
}

// Living Wood Staff: "[X CONVICTION] Unique melee weapon. Strength damage
// each strike. Imbued inflict +X damage each strike with this weapon."
// (Avenger.) Equipped with X = 2: two convictions burned as it resolves,
// then a strike of 1 + 2 at a blocker the next turn.
#[test]
fn living_wood_staff_spends_x_convictions_for_x_more_damage() {
    let mut e = game(2);
    let john = imbued(&mut e, 0, "John \"Cop90\" O'Malley");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let held = convictions(&mut e, john, 3);
    hand(&mut e, 0, &["Living Wood Staff"]);
    pick(&mut e, play_at(0, 0, Some(john), Target::None));
    pick_when(&mut e, Choice::SetX(2));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let burned = held
        .iter()
        .filter(|c| !e.state.cards[c.index()].in_play)
        .count();
    assert_eq!(burned, 2, "X convictions");
    let staff = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(e.state.cards[staff.index()].fixed_x, 2);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(john));
    pick_when(&mut e, block(z));
    pick_when(&mut e, use_on(staff, 0, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 3);
}

// Cleave: "[ven] Choose a melee weapon this imbued has or spend 1
// conviction [1 CONVICTION] to put this card on this imbued to represent a
// melee weapon that does strength damage each strike. This melee weapon
// inflicts +1 additional damage this action. If the opposing minion is (or
// becomes) immune to non-aggravated damage, he or she loses that immunity
// for handling damage from this weapon. Burn the melee weapon at the end
// of the action." The Righteous Blade chosen: 1 + 1 + 1, burned as the
// action ends.
// Rulings: [LSJ 20060412].
#[test]
fn cleave_adds_a_damage_to_the_chosen_weapon_and_burns_it() {
    let mut e = game(2);
    let john = imbued(&mut e, 0, "John \"Cop90\" O'Malley");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let blade = equipped(&mut e, "Righteous Blade", john);
    hand(&mut e, 0, &["Cleave"]);
    pick(&mut e, bleed(john));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play_at(0, 0, Some(john), Target::Card(blade)));
    pick_when(&mut e, use_on(blade, 0, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 3);
    assert!(!e.state.cards[blade.index()].in_play, "burned");
}

// Living Wood Staff: "Using Cleave with this weapon will not burn this
// weapon." Its strike at X = 0 is 1 + 1, and it stays.
#[test]
fn cleave_spares_the_living_wood_staff() {
    let mut e = game(2);
    let john = imbued(&mut e, 0, "John \"Cop90\" O'Malley");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let staff = equipped(&mut e, "Living Wood Staff", john);
    hand(&mut e, 0, &["Cleave"]);
    pick(&mut e, bleed(john));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play_at(0, 0, Some(john), Target::Card(staff)));
    pick_when(&mut e, use_on(staff, 0, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 2);
    assert!(e.state.cards[staff.index()].in_play, "spared");
}

// Cleave put on as a weapon for a conviction: an equipment (LSJ
// 20060608), striking for strength + 1 past Tye Cooper's immunity to
// non-aggravated damage, burned as the action ends.
// Rulings: [LSJ 20060608].
#[test]
fn cleave_as_a_weapon_strikes_past_an_immunity_to_normal_damage() {
    let mut e = game(2);
    let john = imbued(&mut e, 0, "John \"Cop90\" O'Malley");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let tye = e.add_ally(SeatId(1), find_library("Tye Cooper"));
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let held = convictions(&mut e, john, 1);
    hand(&mut e, 0, &["Cleave"]);
    pick(&mut e, bleed(john));
    pick_when(&mut e, block(tye));
    pick_when(&mut e, play_at(0, 1, Some(john), Target::None));
    until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    let cleave = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.in_play && c.card == find_library("Cleave"))
            .expect("Cleave in play") as u32,
    );
    assert_eq!(
        e.state.cards[cleave.index()].host,
        Some(john),
        "on the imbued"
    );
    placed(&e, cleave);
    pick_when(&mut e, use_on(cleave, 0, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[held[0].index()].in_play, "its conviction");
    assert!(
        !e.state.minions[tye.index()].in_play,
        "burned past immunity"
    );
    assert!(!e.state.cards[cleave.index()].in_play, "burned");
}

// Expiate: "Requires an imbued with 5 conviction. Only usable before range
// is determined on the first round. If two consecutive rounds of combat
// occur at close range and this imbued remains ready, this imbued burns 5
// conviction during the press step of that second round to burn himself
// or herself and the opposing monster. Otherwise, this imbued burns 1
// conviction when combat ends." Chun Hei's Immortal Grapple [POT] presses
// into a second round at close range: both burned. With no press, the
// combat ends after one round: a conviction burned.
// Rulings: [LSJ 20010312] [LSJ 20001027].
#[test]
fn expiate_burns_both_after_two_close_rounds_or_a_conviction() {
    for press in [true, false] {
        let mut e = game(2);
        let pedro = imbued(&mut e, 0, "Pedro Cortez");
        // Life to last two rounds of Chun Hei's strikes.
        e.state.minions[pedro.index()].blood = 8;
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let chun = e.add_minion(SeatId(1), find_card("Chun Hei"), 6, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let held = convictions(&mut e, pedro, 5);
        hand(&mut e, 0, &["Expiate"]);
        hand(&mut e, 1, &["Immortal Grapple"]);
        pick(&mut e, bleed(pedro));
        pick_when(&mut e, block(chun));
        pick_when(&mut e, play_at(0, 0, Some(pedro), Target::None));
        if press {
            pick_when(&mut e, play_at(0, 1, Some(chun), Target::None));
            pick_when(&mut e, Choice::UsePress);
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let burned = held
            .iter()
            .filter(|c| !e.state.cards[c.index()].in_play)
            .count();
        if press {
            assert!(!e.state.minions[pedro.index()].in_play, "the imbued");
            assert!(!e.state.minions[chun.index()].in_play, "the monster");
            assert_eq!(burned, 5);
        } else {
            assert!(e.state.minions[pedro.index()].in_play);
            assert_eq!(burned, 1, "a conviction as the combat ends");
        }
    }
}

// Lock: "[def] This action is at +1 stealth if it is undirected. Put this
// card on any minion. Other Methuselahs' actions targeting this minion
// cost monsters an additional blood or life. If this minion is a monster,
// he or she burns a blood or life when he or she attempts an action or a
// block. This minion may burn this card as an action." Put on the prey's
// vampire, controlled by its Methuselah: its bleed burns it a blood, and
// the Brujah's Bum's Rush at it costs the Brujah one more.
// Rulings: [LSJ 20080809-2] [RTR 20081119].
#[test]
fn lock_taxes_its_minions_actions_and_actions_at_it() {
    let mut e = game(2);
    let jack = imbued(&mut e, 0, "Jack \"Hannibal137\" Harmon");
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Lock", "Bum's Rush"]);
    pick(&mut e, play_at(0, 0, Some(jack), Target::Minion(z)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let lock = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Lock"))
        .expect("Lock in play");
    assert_eq!(e.state.cards[lock].host, Some(z));
    placed(&e, CardId(lock as u32));
    pick(&mut e, bleed(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 1, "its own action");
    pick(&mut e, play_at(0, 0, Some(b), Target::Minion(z)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    // Lock's blood, then the hand strikes exchanged.
    assert_eq!(blood(&e, b), 4 - 1 - 1);
    assert_eq!(blood(&e, z), 4 - 1 - 1);
}

// Its block attempt: the locked vampire burns a blood as it attempts to
// block the Brujah's bleed.
#[test]
fn lock_burns_a_blood_as_its_minion_attempts_a_block() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    equipped(&mut e, "Lock", z);
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 1 - 1, "Lock's blood and the strike");
    assert_eq!(blood(&e, b), 4 - 1);
}

// Moise Kasavubu: "He may take a +1 stealth action to search your library
// (shuffle afterward) or hand for an equipment and move it to any ready
// imbued you control (that imbued must meet the requirements and pay the
// cost as normal)." The Crusader Sword from the hand, then from the
// library, each on the imbued for its pool.
#[test]
fn moise_kasavubu_equips_a_ready_imbued_from_library_or_hand() {
    for from_hand in [true, false] {
        let mut e = game(2);
        let jack = imbued(&mut e, 0, "Jack \"Hannibal137\" Harmon");
        let moise = e.add_ally(SeatId(0), find_library("Moise Kasavubu"));
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let sword = find_library("Crusader Sword, The");
        if from_hand {
            hand(&mut e, 0, &["Crusader Sword, The"]);
        } else {
            // Under a hand's worth of other cards: not drawn first.
            e.state.seats[0]
                .library
                .put(sword, zvtes::copies::CopyId::NONE);
            for _ in 0..8 {
                e.state.seats[0]
                    .library
                    .put(find_library("Pursuit"), zvtes::copies::CopyId::NONE);
            }
        }
        let pool = e.state.seats[0].pool;
        let found = if from_hand {
            Choice::EmployFromHand {
                card: sword,
                mode: 0,
            }
        } else {
            Choice::FindCard(sword)
        };
        let card = moise_card(&e, moise);
        pick(
            &mut e,
            Choice::UseAbility {
                source: AbilitySource::Card(card),
                ability: 1,
                actor: Some(moise),
                target: Target::Minion(jack),
            },
        );
        pick_when(&mut e, found);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let on = e
            .state
            .cards
            .iter()
            .any(|c| c.in_play && c.card == sword && c.host == Some(jack));
        assert!(on, "on the imbued");
        let cost = zvtes::library::LIBRARY[sword as usize].pool_cost;
        assert_eq!(
            e.state.seats[0].pool,
            pool - match cost {
                zvtes::library::Cost::Fixed(n) => i32::from(n),
                _ => 0,
            }
        );
    }
}

/// The card of ally `m`.
fn moise_card(e: &Engine, m: MinionId) -> CardId {
    CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.body == Some(m))
            .expect("the ally's card") as u32,
    )
}

// Unity: "Lock this card and burn 1 pool to move two cards that require an
// imbued (or a creed or a virtue) from your ash heap to the top of your
// library. You may then cut your library once." Smite and Cleave (virtue
// cards) on top, Pursuit not offered; then uncut, or cut two to the
// bottom.
// Rulings: [LSJ 20060413].
#[test]
fn unity_moves_two_imbued_cards_to_the_library_top_then_may_cut() {
    for cut in [false, true] {
        let mut e = game(2);
        imbued(&mut e, 0, "Jack \"Hannibal137\" Harmon");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let unity = in_play(&mut e, 0, "Unity", None);
        // Seat 0's minion phase reached first: no draw takes the piles.
        request(&mut e);
        let (smite, cleave, pursuit) = (
            find_library("Smite"),
            find_library("Cleave"),
            find_library("Pursuit"),
        );
        for c in [smite, pursuit, cleave] {
            e.state.seats[0].ash.put(c, zvtes::copies::CopyId::NONE);
        }
        for _ in 0..12 {
            e.state.seats[0]
                .library
                .put(pursuit, zvtes::copies::CopyId::NONE);
        }
        let pool = e.state.seats[0].pool;
        pick(&mut e, use_on(unity, 0, Target::None));
        let req = request(&mut e);
        assert!(!req.options.contains(&Choice::FindCard(pursuit)));
        pick(&mut e, Choice::FindCard(smite));
        // Cleave, the one left, elided.
        pick(&mut e, if cut { Choice::SetX(2) } else { Choice::Pass });
        assert_eq!(e.state.seats[0].pool, pool - 1);
        assert!(e.state.cards[unity.index()].locked);
        // Bottom first: cut, the two at the bottom; else on top.
        let library = e.state.seats[0].library.to_cards();
        let ends = if cut {
            library[..2].to_vec()
        } else {
            library[library.len() - 2..].to_vec()
        };
        assert_eq!(ends, vec![smite, cleave]);
        assert_eq!(e.state.seats[0].ash.to_cards(), vec![pursuit]);
    }
}

// Angel of Berlin: "Master: out-of-turn. Requires a ready imbued. Unlock an
// imbued, or equip a ready imbued you control with an equipment from your
// hand (requirements must be met; pay cost as normal). Not usable during
// combat." During the predator's bleed, seat 0 unlocks its locked imbued;
// on the next one, after the imbued's block, equips the other with The
// Crusader Sword from hand for its pool.
// Rulings: [LSJ 20060410].
#[test]
fn angel_of_berlin_unlocks_or_equips_an_imbued_out_of_turn() {
    let mut e = game(2);
    let jack = imbued(&mut e, 0, "Jack \"Hannibal137\" Harmon");
    let john = imbued(&mut e, 0, "John \"Cop90\" O'Malley");
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(
        &mut e,
        0,
        &["Angel of Berlin", "Angel of Berlin", "Crusader Sword, The"],
    );
    // Not on its own turn.
    let req = request(&mut e);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0 | 1, .. })),
        "{:?}",
        req.options
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[jack.index()].locked = true;
    pick(&mut e, bleed(z));
    pick_when(&mut e, play_at(0, 0, None, Target::Minion(jack)));
    request(&mut e);
    assert!(!e.state.minions[jack.index()].locked, "unlocked");
    // One out-of-turn master between its turns: the equip on the next.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    pick_when(&mut e, block(jack));
    let pool = e.state.seats[0].pool;
    let sword = find_library("Crusader Sword, The");
    let req = until(&mut e, SeatId(0), WindowLabel::ActionBlocked);
    let play = req
        .options
        .iter()
        .find(
            |o| matches!(o, Choice::Play { mode: 1, target: Target::Minion(t), .. } if *t == john),
        )
        .cloned()
        .expect("the equip offered after the block");
    // The Sword, the one equipment in hand, named by elision.
    pick(&mut e, play);
    request(&mut e);
    let on = e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == sword && c.host == Some(john));
    assert!(on, "equipped");
    assert!(e.state.seats[0].pool < pool, "its cost");
}

// Blood Cult Awareness Network: "Whenever there are any ready imbued in
// play, any action to put a vampire or ghoul in play or to add counters to
// an uncontrolled vampire gets -1 stealth and costs an additional blood or
// life." The Embrace (+1 stealth, 2 blood) at 0 stealth: unblocked, 3
// blood; the imbued's block with no intercept succeeds, and fails without
// the event.
#[test]
fn blood_cult_awareness_network_taxes_actions_making_vampires() {
    for (taxed, block_it) in [(true, false), (true, true), (false, true)] {
        let mut e = game(2);
        let jack = imbued(&mut e, 0, "Jack \"Hannibal137\" Harmon");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        if taxed {
            let bcan = in_play(&mut e, 0, "Blood Cult Awareness Network", None);
            e.state.layers.push(zvtes::cards::Layer {
                scope: zvtes::cards::Scope::InPlay,
                modifier: zvtes::cards::Modifier::VampireMakingTaxed,
                source: bcan,
            });
        }
        hand(&mut e, 1, &["Embrace, The"]);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let minions = e.state.minions.len();
        pick(&mut e, play_at(0, 0, Some(v), Target::None));
        if block_it {
            pick_when(&mut e, block(jack));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if taxed && block_it {
            assert_eq!(e.state.minions.len(), minions, "blocked at 0 stealth");
        } else {
            assert_eq!(e.state.minions.len(), minions + 1);
            assert_eq!(blood(&e, v), 4 - 2 - u8::from(taxed));
        }
    }
}

/// Blood Cult Awareness Network in play for `seat`, with its layer.
fn taxed(e: &mut Engine, seat: u8) {
    let bcan = in_play(e, seat, "Blood Cult Awareness Network", None);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::VampireMakingTaxed,
        source: bcan,
    });
}

/// The `nth` crypt vampire with no definition matching `f`.
fn crypt_vampire(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| f(c) && !c.imbued && zvtes::cards::vampire(*i as u16).is_none())
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a vampire matching the filter")
}

// Blood Cult Awareness Network on conditional counters (LSJ 20060414,
// ANK 20240118-2): Feast of the Soul's Secrets [DOM] aimed at a younger
// vampire in the uncontrolled region costs a blood more, its 2 counters
// added on the bleed's success.
// Rulings: [LSJ 20060414] [ANK 20240118-2]; [ANK 20240118-2] [LSJ 20060414].
#[test]
fn blood_cult_awareness_network_taxes_conditional_uncontrolled_counters() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), find_card("Djeneba"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    imbued(&mut e, 2, "Jack \"Hannibal137\" Harmon");
    taxed(&mut e, 2);
    let young = crypt_vampire(|c| c.capacity <= 4, 1);
    let region = &mut e.state.seats[0].uncontrolled;
    region.push(zvtes::state::UncontrolledSlot {
        card: young,
        copy: Default::default(),
        counters: 0,
        banished: None,
    });
    let slot = region.len() as u8 - 1;
    hand(&mut e, 0, &["Feast of the Soul's Secrets"]);
    pick(&mut e, play_at(0, 2, Some(a), Target::Uncontrolled(slot)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled[slot as usize].counters, 2);
    assert_eq!(blood(&e, a), 5 - 1 - 1, "its blood, and the tax");
}

// Web of Knives Recruit is no action to put a vampire in play (LSJ
// 20060616-2): untaxed, its acting vampire pays the same.
// Rulings: [LSJ 20060616-2].
#[test]
fn blood_cult_awareness_network_leaves_web_of_knives_recruit_alone() {
    let mut paid = Vec::new();
    for tax in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Banu Haqim", 0), 4, false);
        e.add_minion(SeatId(0), plain("Banu Haqim", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        imbued(&mut e, 1, "Jack \"Hannibal137\" Harmon");
        if tax {
            taxed(&mut e, 1);
        }
        hand(&mut e, 0, &["Web of Knives Recruit"]);
        pick(&mut e, play_at(0, 0, Some(a), Target::None));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(e.state.cards.iter().any(|c| c.training), "trained");
        paid.push(blood(&e, a));
    }
    assert_eq!(paid[0], paid[1]);
}

// Call the Great Beast's 11th counter is an action to put a vampire in play
// (LSJ 20060815): at 0 stealth, the imbued blocks it, no Great Beast.
// Rulings: [LSJ 20060815] [LSJ 20011212].
#[test]
fn blood_cult_awareness_network_taxes_the_great_beasts_eleventh_counter() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nergal"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let jack = imbued(&mut e, 1, "Jack \"Hannibal137\" Harmon");
    taxed(&mut e, 1);
    hand(&mut e, 0, &["Call the Great Beast"]);
    pick(&mut e, play_at(0, 0, Some(n), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let beast = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.in_play && c.card == find_library("Call the Great Beast"))
            .expect("on Nergal") as u32,
    );
    assert_eq!(e.state.cards[beast.index()].counters, 10);
    e.state.minions[n.index()].locked = false;
    e.state.minions[jack.index()].locked = false;
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(beast),
            ability: 0,
            actor: Some(n),
            target: Target::None,
        },
    );
    pick_when(&mut e, block(jack));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
}

// Raw Recruit's action "to remove that vampire from the game" puts a
// vampire in play (LSJ 20011212): at 0 stealth, the imbued blocks it.
// Rulings: [LSJ 20011212] [LSJ 20060815].
#[test]
fn blood_cult_awareness_network_taxes_raw_recruits_gargoyle() {
    let mut e = game(2);
    let tremere = e.add_minion(SeatId(0), find_card("Porphyry"), 4, false);
    let p = tremere;
    let victim = e.add_minion(
        SeatId(1),
        crypt_vampire(|c| c.clan == "Ventrue" && c.capacity == 6, 0),
        2,
        true,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let jack = imbued(&mut e, 1, "Jack \"Hannibal137\" Harmon");
    let porphyry_tremere = e.add_minion(
        SeatId(0),
        crypt_vampire(|c| c.clan == "Tremere" && (4..=8).contains(&c.capacity), 0),
        6,
        false,
    );
    taxed(&mut e, 1);
    hand(&mut e, 0, &["Raw Recruit"]);
    pick(&mut e, play_at(0, 0, Some(p), Target::Minion(victim)));
    let rr = loop {
        let req = request(&mut e);
        if let Some(i) = e
            .state
            .cards
            .iter()
            .position(|c| c.in_play && c.held_vampire.is_some())
        {
            break CardId(i as u32);
        }
        let i = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::Pass))
            .unwrap_or(0);
        e.choose(i);
    };
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[jack.index()].locked = false;
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(rr),
            ability: 0,
            actor: Some(porphyry_tremere),
            target: Target::None,
        },
    );
    pick_when(&mut e, block(jack));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
}

// Moise Kasavubu "Requires a ready imbued": a minion card's requirement is
// its recruiter's (rulebook): the imbued recruits him, the vampire not.
#[test]
fn moise_kasavubu_is_recruited_by_an_imbued() {
    let mut e = game(2);
    let jack = imbued(&mut e, 0, "Jack \"Hannibal137\" Harmon");
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Moise Kasavubu"]);
    let req = request(&mut e);
    assert!(req
        .options
        .contains(&play_at(0, 0, Some(jack), Target::None)));
    assert!(!req.options.contains(&play_at(0, 0, Some(b), Target::None)));
}

// Imprison twice: the tolls add up (Archon's, LSJ 19980225): the blocker's
// hand strike costs it 2 blood more.
#[test]
fn imprison_copies_add_their_tolls() {
    let (mut e, jack, z) =
        imbued_blocked("Jack \"Hannibal137\" Harmon", 4, &["Imprison", "Imprison"]);
    pick_when(&mut e, play_at(0, 0, Some(jack), Target::None));
    pick_when(&mut e, play_at(0, 0, Some(jack), Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 2 - 1, "two tolls and the imbued's strike");
}

// Cleave's +1 inherits the weapon's damage (TOM 19960225): on The Crusader
// Sword, 3 aggravated against its 2.
// Rulings: [TOM 19960225].
#[test]
fn cleaves_damage_inherits_the_weapons_aggravation() {
    let mut left = Vec::new();
    for cleave in [false, true] {
        let mut e = game(2);
        let john = imbued(&mut e, 0, "John \"Cop90\" O'Malley");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let sword = equipped(&mut e, "Crusader Sword, The", john);
        hand(&mut e, 0, &["Cleave"]);
        pick(&mut e, bleed(john));
        pick_when(&mut e, block(z));
        if cleave {
            pick_when(&mut e, play_at(0, 0, Some(john), Target::Card(sword)));
        }
        pick_when(&mut e, use_on(sword, 0, Target::None));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(e.state.minions[z.index()].in_torpor, "aggravated");
        left.push(blood(&e, z));
    }
    // One point more, part of the same aggravated hit.
    assert_eq!(left, vec![3, 2]);
}

// Living Wood Staff at X = 0 (LSJ 20030502), no conviction held; then taken
// by a vampire, no Avenger (ANK 20180719-3).
// Rulings: [ANK 20180719-3]; [LSJ 20030502].
#[test]
fn living_wood_staff_x_may_be_0_and_be_taken_by_a_non_avenger() {
    let mut e = game(2);
    let john = imbued(&mut e, 0, "John \"Cop90\" O'Malley");
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Living Wood Staff"]);
    pick(&mut e, play_at(0, 0, Some(john), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let staff = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.in_play && c.card == find_library("Living Wood Staff"))
            .expect("equipped") as u32,
    );
    assert_eq!(e.state.cards[staff.index()].fixed_x, 0);
    pick(&mut e, take_from(b, john));
    request(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[staff.index()].host, Some(b));
}

// Faerie Wards [MYT] on an infernal acting vampire: no normal unlock, but
// her controller may still pay a pool to unlock her (LSJ 20050114).
// Rulings: [LSJ 20050114].
#[test]
fn faerie_wards_superior_leaves_an_infernal_vampire_its_paid_unlock() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Winterlich"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let arishat = e.add_minion(SeatId(1), find_card("Arishat"), 3, false);
    hand(&mut e, 0, &["Faerie Wards"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(arishat));
    pick_when(&mut e, play_at(0, 2, Some(r), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10, "failed");
    let unlock = Choice::UnlockInfernal { minion: arishat };
    let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    assert!(e.state.minions[arishat.index()].locked);
    assert!(req.options.contains(&unlock), "{:?}", req.options);
    pick(&mut e, unlock);
    assert!(!e.state.minions[arishat.index()].locked);
}

// Faerie Wards [aus]: +2 intercept for a block that needs it; not once a
// block was made (ANK 20260316, LSJ 20041022).
// Rulings: [ANK 20260316] [LSJ 20041022].
#[test]
fn faerie_wards_intercept_and_none_after_a_block() {
    // [aus]: a +1 stealth bleed blocked.
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Julia Prima"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Faerie Wards"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[z.index()].next_action_stealth = 1;
    pick(&mut e, bleed(z));
    pick_when(&mut e, block(r));
    pick_when(&mut e, play_at(0, 0, Some(r), Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10, "blocked");
    // After a block made: never offered.
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Julia Prima"), 5, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Faerie Wards"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    pick_when(&mut e, block(b));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        play_at(0, 1, Some(r), Target::None),
    );
}

// Karen Suadela, a prince, takes no action an event provides (LSJ
// 20100121): Break the Code's attack on an imbued is not hers.
// Rulings: [LSJ 20100121] [LSJ 20081120-2].
#[test]
fn karen_suadela_takes_no_break_the_code_action() {
    let mut e = game(2);
    let karen = e.add_minion(SeatId(0), find_card("Karen Suadela"), 4, false);
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let i = imbued(&mut e, 1, "Jack \"Hannibal137\" Harmon");
    let code = in_play(&mut e, 1, "Break the Code", None);
    let attack = |m| Choice::UseAbility {
        source: AbilitySource::Card(code),
        ability: 0,
        actor: Some(m),
        target: Target::Minion(i),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&attack(v)));
    assert!(!req.options.contains(&attack(karen)), "{:?}", req.options);
}

// A virtue card printed without its icon requires its virtue (rules-qa
// 102.2): Smite [ven] is not Jack's (Defense, Judgment), however many
// convictions.
#[test]
fn a_virtue_card_requires_its_virtue() {
    let (mut e, jack, _) = imbued_blocked("Jack \"Hannibal137\" Harmon", 3, &["Smite"]);
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        play_at(0, 0, Some(jack), Target::None),
    );
}

// Paul "Sixofswords29" Moreton moving his Methuselah's last pool to an
// imbued in the uncontrolled region: ousted then, the turn passing (the
// fuzz: the game ended with three Methuselahs left).
#[test]
fn paul_moving_the_last_pool_ousts_and_passes_the_turn() {
    let mut e = game(3);
    let paul = imbued(&mut e, 0, "Paul \"Sixofswords29\" Moreton");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: find_card("Peter \"Outback295\" Rophail"),
            copy: Default::default(),
            counters: 1,
            banished: None,
        });
    until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    e.state.seats[0].pool = 1;
    pick_when(&mut e, own_on(paul, 0, Target::Uncontrolled(0)));
    let req = request(&mut e);
    assert!(e.state.seats[0].ousted);
    assert_ne!(req.seat, SeatId(0), "the turn passed");
}

// Expiate against a mortal: no opposing monster, so two close rounds burn
// nothing at the press step; a conviction burned as the combat ends
// (rules-qa 102.21). ECTU Operative blocks Travis, pressed on by Talbot's
// Chainsaw.
#[test]
fn expiate_against_a_mortal_only_costs_a_conviction() {
    let mut e = game(2);
    let travis = imbued(&mut e, 0, "Travis \"Traveler72\" Miller");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ectu = e.add_ally(SeatId(1), find_library("ECTU Operative"));
    e.state.minions[ectu.index()].blood = 6;
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    equipped(&mut e, "Talbot's Chainsaw", ectu);
    let held = convictions(&mut e, travis, 5);
    hand(&mut e, 0, &["Expiate"]);
    pick(&mut e, bleed(travis));
    pick_when(&mut e, block(ectu));
    pick_when(&mut e, play_at(0, 0, Some(travis), Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[travis.index()].in_play);
    assert!(
        e.state.minions[ectu.index()].in_play,
        "a mortal, not burned"
    );
    // Drained to its 3 life, then a strike each round.
    assert_eq!(blood(&e, ectu), 3 - 2, "two rounds fought");
    let burned = held
        .iter()
        .filter(|c| !e.state.cards[c.index()].in_play)
        .count();
    assert_eq!(burned, 1);
}

// Muse of Flame's "a monster controlled by your prey" includes a torpid one
// (D28; rules-qa 102.12): the prey's only vampire, in torpor, faces the
// imbued.
#[test]
fn muse_of_flame_may_pick_a_torpid_monster() {
    let mut e = game(3);
    let j = imbued(&mut e, 0, "Jennifer \"Flame61\" Vidisania");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    hand(&mut e, 0, &["Muse of Flame"]);
    pick(&mut e, play_at(0, 0, Some(j), Target::Seat(SeatId(1))));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(blood(&e, z) < 4, "struck in torpor");
}

// Rulings: [LSJ 20071017].
// Imprison's toll lets the opposing monster choose not to strike (rules-qa
// 102.14): the blocker's strike step offers a pass.
#[test]
fn imprison_lets_the_opposing_monster_decline_its_strike() {
    let (mut e, jack, _) = imbued_blocked("Jack \"Hannibal137\" Harmon", 2, &["Imprison"]);
    pick_when(&mut e, play_at(0, 0, Some(jack), Target::None));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(req.options.contains(&Choice::Pass), "{req:?}");
}

// Lock on an ally with its last life (rules-qa 102.22): it burns as it
// announces its bleed, the bleed then without effect.
#[test]
fn lock_burns_an_ally_announcing_with_its_last_life() {
    let mut e = game(2);
    let screamer = e.add_ally(SeatId(0), find_library("Screamer"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    equipped(&mut e, "Lock", screamer);
    pick(&mut e, bleed(screamer));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[screamer.index()].in_play, "burned");
    assert_eq!(e.state.seats[1].pool, 10, "no bleed");
}

// Living Wood Staff found by Moise Kasavubu, its cost paid as normal:
// [X CONVICTION] asked a conviction at a time (rules-qa 102.1): one burned,
// X = 1.
#[test]
fn living_wood_staff_found_by_moise_asks_its_x() {
    let mut e = game(2);
    let john = imbued(&mut e, 0, "John \"Cop90\" O'Malley");
    let moise = e.add_ally(SeatId(0), find_library("Moise Kasavubu"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let held = convictions(&mut e, john, 1);
    hand(&mut e, 0, &["Living Wood Staff"]);
    let card = moise_card(&e, moise);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(card),
            ability: 1,
            actor: Some(moise),
            target: Target::Minion(john),
        },
    );
    pick_when(
        &mut e,
        Choice::EmployFromHand {
            card: find_library("Living Wood Staff"),
            mode: 0,
        },
    );
    pick_when(&mut e, Choice::BurnInPlay { card: held[0].0 });
    request(&mut e);
    let staff = e
        .state
        .cards
        .iter()
        .find(|c| c.in_play && c.card == find_library("Living Wood Staff"))
        .expect("equipped");
    assert_eq!(staff.host, Some(john));
    assert_eq!(staff.fixed_x, 1);
    assert!(!e.state.cards[held[0].index()].in_play, "its conviction");
}

// Living Wood Staff equipped by Angel of Berlin from hand, its cost paid as
// normal: [X CONVICTION] asked a conviction at a time (rules-qa 102.1).
#[test]
fn living_wood_staff_equipped_by_angel_of_berlin_asks_its_x() {
    let mut e = game(2);
    let john = imbued(&mut e, 0, "John \"Cop90\" O'Malley");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let held = convictions(&mut e, john, 1);
    hand(&mut e, 0, &["Angel of Berlin", "Living Wood Staff"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    // The Staff, the one equipment in hand, named by elision.
    pick_when(&mut e, play_at(0, 1, None, Target::Minion(john)));
    pick_when(&mut e, Choice::BurnInPlay { card: held[0].0 });
    request(&mut e);
    let staff = e
        .state
        .cards
        .iter()
        .find(|c| c.in_play && c.card == find_library("Living Wood Staff"))
        .expect("equipped");
    assert_eq!(staff.host, Some(john));
    assert_eq!(staff.fixed_x, 1);
    assert!(!e.state.cards[held[0].index()].in_play, "its conviction");
}
