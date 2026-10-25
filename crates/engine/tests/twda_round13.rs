//! Legacy cards for TWDA decks, round 13 (`ADDED_BATCHES`, slice 16y): the
//! 2019-2024 products completed set by set (First Blood, Sabbat
//! Preconstructed, Twenty-Fifth and Thirtieth Anniversary) and the TWDA
//! greedy's first step. Readings in rules-qa Batch 76. Windows with a
//! single option are elided, so the scripts only stop where a real
//! decision exists.

use zvtes::cards::{AbilitySource, Stat, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
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
            sect.is_none_or(|s| c.sect == Some(s))
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

fn pool(e: &Engine, seat: usize) -> i32 {
    e.state.seats[seat].pool
}

/// A master card played from hand at `target`.
fn master(card: u8, mode: u8, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: None,
        target,
    }
}

/// Through seat 0's next turn, taking every option `want` matches: how many.
fn next_turn_taking(e: &mut Engine, mut want: impl FnMut(&Choice) -> bool) -> usize {
    let start = e.state.turn;
    let mut n = 0;
    loop {
        let req = request(e);
        if req.seat == SeatId(0) && req.window == WindowLabel::MinionPhase && e.state.turn > start {
            return n;
        }
        if let Some(i) = req.options.iter().position(&mut want) {
            e.choose(i);
            n += 1;
            continue;
        }
        e.choose(fallback(&req));
    }
}

// ---- Chunk 1: Animalism, University Hunting Ground, Computer Hacking,
// ---- Aching Beauty, the two justicars, Thadius Zho, Black Metamorphosis

// Animalism: "Put this card on a vampire. This vampire gets +1 level of
// Animalism [ani] and +1 capacity. Cannot be put on a vampire with superior
// Animalism [ANI]."
#[test]
fn animalism_gives_a_level_and_capacity_not_past_superior() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_disc("", None, 0), 4, false);
    let sup = e.add_minion(SeatId(0), vampire_with("ANI", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Animalism"]);
    e.at_master_phase();
    let cap = e.state.capacity_of(a).unwrap();
    let req = request(&mut e);
    assert!(!req.options.contains(&master(0, 0, Target::Minion(sup))));
    pick(&mut e, master(0, 0, Target::Minion(a)));
    request(&mut e);
    assert_eq!(e.state.discipline_level(a, "ani"), 1);
    assert_eq!(e.state.capacity_of(a), Some(cap + 1));
}

// University Hunting Ground: "During your unlock phase, a ready vampire you
// control can gain 1 blood."
#[test]
fn university_hunting_ground_feeds_a_vampire() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let hg = in_play(&mut e, 0, "University Hunting Ground", None);
    let used = next_turn_taking(&mut e, |o| *o == use_at(hg, 0, None, Target::Minion(v)));
    assert_eq!((used, blood(&e, v)), (1, 2));
}

// Computer Hacking: "Ⓓ Bleed with +1 bleed."
#[test]
fn computer_hacking_bleeds_for_two() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Computer Hacking"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 10 - 2);
}

// Aching Beauty: "Put this card on a Toreador. If this Toreador is blocked,
// the blocking minion's controller burns 1 pool before block resolution."
// On another Methuselah's Toreador too; two copies, 2 pool [LSJ 19980225].
#[test]
fn aching_beauty_burns_the_blockers_pool_per_copy() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), plain("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let other = e.add_minion(SeatId(1), plain("Toreador", 1), 4, false);
    hand(&mut e, 0, &["Aching Beauty", "Aching Beauty"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(req.options.contains(&master(0, 0, Target::Minion(other))));
    assert!(!req.options.contains(&master(0, 0, Target::Minion(z))));
    pick(&mut e, master(0, 0, Target::Minion(t)));
    pick_when(&mut e, master(0, 0, Target::Minion(t)));
    pick_when(&mut e, bleed(t));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 10 - 2);
}

// Banu Haqim Justicar: "Choose a ready Camarilla Banu Haqim. Successful
// referendum means this card is put on the chosen Banu Haqim to represent
// the unique Camarilla title of Banu Haqim Justicar. During this
// referendum, each Banu Haqim gets +1 vote." Kasim Bayar's printed
// "Assamite Justicar" is that title (rules-digest §1, V5 clan names): the
// two are rivals.
#[test]
fn banu_haqim_justicar_rivals_kasim_bayar() {
    let mut e = game(2);
    let b = e.add_minion(
        SeatId(0),
        textless(
            |c| c.clan == "Banu Haqim" && c.sect == Some(Sect::Camarilla) && c.title == Title::None,
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let kasim = e.add_minion(SeatId(1), find_card("Kasim Bayar"), 6, false);
    assert_eq!(e.state.title_of(kasim), Title::Justicar("Banu Haqim"));
    hand(&mut e, 0, &["Banu Haqim Justicar"]);
    pick(&mut e, play(0, 0, b));
    // b the one choice (Kasim's controller claims the title already):
    // chosen without a decision.
    pick_when(&mut e, votes(b, true));
    close_polling(&mut e);
    request(&mut e);
    assert_eq!(e.state.claimed_title(b), Title::Justicar("Banu Haqim"));
    assert!(e.state.title_rivals(b).contains(&kasim));
    let contested = |m: MinionId| e.state.minions[m.index()].title_contested;
    assert!(contested(b) && contested(kasim), "the title contested");
}

// Lasombra Justicar: the same for a ready Camarilla Lasombra; a Sabbat
// Lasombra is not chosen.
#[test]
fn lasombra_justicar_titles_a_ready_camarilla_lasombra() {
    let mut e = game(2);
    let l = e.add_minion(
        SeatId(0),
        textless(
            |c| c.clan == "Lasombra" && c.sect == Some(Sect::Camarilla) && c.title == Title::None,
            0,
        ),
        4,
        false,
    );
    let sabbat = e.add_minion(
        SeatId(0),
        textless(
            |c| c.clan == "Lasombra" && c.sect == Some(Sect::Sabbat) && c.title == Title::None,
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Lasombra Justicar"]);
    // The one Camarilla Lasombra the only choice: elided.
    pick(&mut e, play(0, 0, l));
    pick_when(&mut e, votes(l, true));
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.title_of(l), Title::Justicar("Lasombra"));
    assert_eq!(e.state.title_of(sabbat), Title::None);
}

// Thadius Zho: "can strike: 2R damage. He gets 1 optional maneuver each
// combat. He can burn 1 blood from a vampire as a +1 stealth Ⓓ action."
#[test]
fn thadius_zho_burns_a_vampires_blood_with_stealth() {
    let mut e = game(2);
    let zho = e.add_ally(SeatId(0), find_library("Thadius Zho"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let act = Choice::UseAbility {
        source: AbilitySource::Card(e.state.minions[zho.index()].ally_id().expect("ally card")),
        ability: 2,
        actor: Some(zho),
        target: Target::Minion(z),
    };
    pick_when(&mut e, act);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 1);
}

// Black Metamorphosis: "+1 stealth action. [obt] Put this card on this
// vampire. This vampire gets 1 optional additional strike (limited) each
// round. A vampire can have only one." Its own vampire, a second not
// offered; the strike in its combat.
#[test]
fn black_metamorphosis_gives_an_additional_strike_each_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    hand(&mut e, 0, &["Black Metamorphosis", "Black Metamorphosis"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let bm = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.host == Some(a))
        .map(|i| CardId(i as u32))
        .expect("on its vampire");
    assert!(!e.state.cards[bm.index()].effect_only);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0, .. })),
        "only one"
    );
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, use_at(bm, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 6 - 2, "two hand strikes");
}

// ---- Chunk 2: vampires on existing mechanisms ------------------------------

fn crypt_group(name: &str, group: u8) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && c.group == group)
        .unwrap_or_else(|| panic!("{name} G{group}")) as u16
}

// Aidan Lyle, Charles Delmare: "gets 1 optional press each combat."
#[test]
fn aidan_and_charles_have_a_press_each_combat() {
    for name in ["Aidan Lyle", "Charles Delmare"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card(name), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        offering(&mut e, Choice::UsePress);
    }
}

// Antón de Concepción: "1 optional maneuver during the first round of
// combat."
#[test]
fn anton_maneuvers_in_the_first_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Antón de Concepción"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, own_use(a, 0, Target::None));
    assert_eq!(req.window, WindowLabel::CombatDetermineRange);
}

// Duality: "After you replace a card requiring Vicissitude [vic] Duality
// played, you can draw 1 additional card (discard down afterward)."
#[test]
fn duality_draws_again_after_a_vicissitude_card() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Duality"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    hand(&mut e, 0, &["Horrid Form"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
    pick(&mut e, bleed(d));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, d));
    offering(&mut e, Choice::DrawLibrary);
}

// Emily Carson: "During your discard phase, Emily can burn the Edge to
// unlock."
#[test]
fn emily_carson_burns_the_edge_to_unlock() {
    let mut e = game(2);
    let emily = e.add_minion(SeatId(0), find_card("Emily Carson"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.state.minions[emily.index()].locked = true;
    e.state.seats[0].has_edge = true;
    let req = offering(&mut e, own_use(emily, 0, Target::None));
    assert_eq!(req.window, WindowLabel::DiscardPhase);
    pick(&mut e, own_use(emily, 0, Target::None));
    assert!(!e.state.minions[emily.index()].locked);
    assert!(!e.state.seats[0].has_edge);
}

// Frondator: "Rescuing a vampire from torpor costs Frondator 1 fewer
// blood": 1 instead of 2.
// The action's cost, whoever pays [PIB 20110918] [LSJ 20020304-2].
#[test]
fn frondator_rescues_for_one_blood() {
    let mut e = game(2);
    let f = e.add_minion(SeatId(0), find_card("Frondator"), 3, false);
    let t = e.add_minion(SeatId(0), plain("Brujah", 1), 0, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: f,
            action: ActionKind::Rescue,
            target: Some(t),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_torpor);
    assert_eq!(blood(&e, f), 3 - 1);
}

// Gem Ghastly: "You and Gem can play cards requiring a Ventrue as if she
// were a Ventrue": her Methuselah plays Uptown Hunting Ground (requires a
// Ventrue); without her, no.
// As a Ventrue for that card's effects [LSJ 20101013] [ANK 20190228] [ANK
// 20200705-2] [LSJ 20070708] [RTR 20070707].
#[test]
fn gem_ghastly_lets_her_methuselah_play_ventrue_cards() {
    for with_gem in [true, false] {
        let mut e = game(2);
        let v = if with_gem {
            find_card("Gem Ghastly")
        } else {
            plain("Malkavian", 0)
        };
        e.add_minion(SeatId(0), v, 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
        hand(&mut e, 0, &["Uptown Hunting Ground"]);
        e.at_master_phase();
        let req = request(&mut e);
        let offered = req
            .options
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0, .. }));
        assert_eq!(offered, with_gem);
    }
}

// Jeremy "Wix" Wyzchovsky: "can bleed with +1 bleed as a Ⓓ action."
#[test]
fn jeremy_bleeds_for_two() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jeremy \"Wix\" Wyzchovsky"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    let act = |o: &Choice| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(m), ability: 0, .. } if *m == j);
    let req = request(&mut e);
    let i = req.options.iter().position(act).expect("his bleed");
    e.choose(i);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 10 - 2);
}

// Kateline Nadasdy: "During the polling step of a political action,
// Kateline can burn 1 blood to force a non-acting vampire to abstain." Not
// the acting vampire; once a referendum, locked too (ANK 20220705).
#[test]
fn kateline_forces_a_non_acting_vampire_to_abstain() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    let other = e.add_minion(SeatId(0), find_card("Karen Suadela"), 4, false);
    let k = e.add_minion(SeatId(1), find_card("Kateline Nadasdy"), 3, false);
    e.state.minions[k.index()].locked = true;
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, caller));
    pick(&mut e, alloc(0, 3));
    let force = |t: MinionId| own_use(k, 0, Target::Minion(t));
    let req = offering(&mut e, force(other));
    assert!(
        !req.options.contains(&force(caller)),
        "not the acting vampire"
    );
    pick(&mut e, force(other));
    assert_eq!(blood(&e, k), 2);
    assert!(e
        .referendum()
        .expect("referendum")
        .abstaining
        .contains(&other));
}

// L'Épuisette: "+1 bleed against a Methuselah who controls a ready
// Toreador."
#[test]
fn l_epuisette_bleeds_harder_against_a_toreador() {
    for toreador in [true, false] {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), find_card("L'Épuisette"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let v = if toreador {
            plain("Toreador", 0)
        } else {
            plain("Ventrue", 0)
        };
        e.add_minion(SeatId(1), v, 6, false);
        e.state.minions.last_mut().unwrap().locked = true;
        pick(&mut e, bleed(l));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e, 1), 10 - 1 - toreador as i32, "{toreador}");
    }
}

// Luca Italicus: "Once each round of combat after the first, Luca can
// strike: 2R damage": not in the first round; in the second, after the
// blocker's press (Aidan Lyle's).
#[test]
fn luca_italicus_strikes_at_range_after_the_first_round() {
    let mut e = game(2);
    let luca = e.add_minion(SeatId(0), find_card("Luca Italicus"), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Aidan Lyle"), 7, false);
    let strike = own_use(luca, 0, Target::None);
    pick(&mut e, bleed(luca));
    pick_when(&mut e, block(z));
    never_offered_until(&mut e, SeatId(1), WindowLabel::CombatPress, strike);
    pick(&mut e, Choice::UsePress);
    pick_when(&mut e, strike);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 7 - 1 - 2, "the hand strike, then 2R");
}

// Nikolaus Vermeulen (G2): "During your unlock phase, you may move 1 blood
// from Nikolaus to any Nosferatu in your uncontrolled region."
#[test]
fn nikolaus_g2_feeds_an_uncontrolled_nosferatu() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), crypt_group("Nikolaus Vermeulen", 2), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    let nos = textless(|c| c.clan == "Nosferatu", 0);
    let ven = plain("Ventrue", 1);
    for card in [nos, ven] {
        e.state.seats[0]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card,
                copy: Default::default(),
                counters: 0,
                banished: None,
            });
    }
    let feed = |i: u8| own_use(n, 0, Target::Uncontrolled(i));
    let mut seen_ventrue = false;
    let used = next_turn_taking(&mut e, |o| {
        seen_ventrue |= *o == feed(1);
        *o == feed(0)
    });
    assert!(!seen_ventrue, "a Nosferatu only");
    assert_eq!((used, blood(&e, n)), (1, 3));
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 1);
}

// Nikolaus Vermeulen (G6): "During your influence phase, you can burn the
// Edge to add 1 blood to a Nosferatu in your uncontrolled region."
// In torpor too [LSJ 20011022].
#[test]
fn nikolaus_g6_burns_the_edge_for_an_uncontrolled_nosferatu() {
    let mut e = game(2);
    // In torpor: usable there too [LSJ 20011022].
    let n = e.add_minion(SeatId(0), crypt_group("Nikolaus Vermeulen", 6), 4, true);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.state.seats[0].has_edge = true;
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: textless(|c| c.clan == "Nosferatu", 0),
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    let use_it = own_use(n, 0, Target::Uncontrolled(0));
    let req = offering(&mut e, use_it);
    assert_eq!(req.window, WindowLabel::InfluenceTransfers);
    pick(&mut e, use_it);
    assert!(!e.state.seats[0].has_edge);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 1);
    assert_eq!(blood(&e, n), 4, "from the bank");
}

// Selma the Repugnant: "+1 intercept when attempting to block a Nosferatu
// vampire": she blocks a Nosferatu's +1 stealth hunt, not a Ventrue's.
#[test]
fn selma_intercepts_nosferatu() {
    for (clan, blocked) in [("Nosferatu", true), ("Ventrue", false)] {
        let mut e = game(2);
        let selma = e.add_minion(SeatId(1), find_card("Selma the Repugnant"), 5, false);
        let a = e.add_minion(SeatId(0), plain(clan, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        pick(&mut e, action(a, ActionKind::Hunt));
        pick_when(&mut e, block(selma));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        // Blocked: no blood from the hunt, 1 lost to her hand strike.
        assert_eq!(blood(&e, a), if blocked { 4 - 1 } else { 4 + 1 }, "{clan}");
    }
}

// Una: "Combat cards requiring Fortitude [for] cost Una 1 fewer blood":
// Skin of Steel (1 blood) is free.
#[test]
fn una_plays_fortitude_combat_cards_for_less() {
    let mut e = game(2);
    let una = e.add_minion(SeatId(1), find_card("Una"), 6, false);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    hand(&mut e, 1, &["Skin of Steel"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(una));
    pick_when(&mut e, play(0, 0, una));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, una), 6, "free, the strike prevented");
}

// Lady Vadislava: "Once each round, Lady Vadislava can burn 1 blood to make
// the damage from her hand strikes aggravated that round."
// Before strike resolution and prevention [RTR 19960112].
#[test]
fn lady_vadislava_buys_aggravated_hand_strikes() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Lady Vadislava"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
    pick(&mut e, bleed(v));
    pick_when(&mut e, block(z));
    pick_when(&mut e, own_use(v, 0, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // A normal point would be mended; an aggravated one sends him to torpor.
    assert!(e.state.minions[z.index()].in_torpor, "aggravated");
    assert_eq!(blood(&e, z), 2);
}

// Lord Leopold Valdemar: "can strike, ranged: steal 2 blood or life
// (becoming blood)."
#[test]
fn lord_leopold_steals_two_blood() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lord Leopold Valdemar"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    pick(&mut e, bleed(l));
    pick_when(&mut e, block(z));
    pick_when(&mut e, own_use(l, 0, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, l), blood(&e, z)), (4 + 2 - 1, 6 - 2));
}

// Grimgroth: "While Grimgroth is ready, mage allies cost Tremere you
// control 2 fewer pool to recruit. +2 bleed": another Tremere recruits
// Thadius Zho (3 pool) for 1; not while Grimgroth is in torpor.
#[test]
fn grimgroth_discounts_mage_allies_for_his_tremere() {
    for ready in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(0), find_card("Grimgroth"), 6, !ready);
        let t = e.add_minion(SeatId(0), plain("Tremere", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
        hand(&mut e, 0, &["Thadius Zho"]);
        pick(&mut e, play(0, 0, t));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e, 0), 10 - if ready { 1 } else { 3 }, "{ready}");
    }
}

// Dónal O'Connor (G6): "During your minion phase, Dónal can burn 1 blood to
// unlock."
// In torpor too [LSJ 20011022].
#[test]
fn donal_g6_burns_blood_to_unlock() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), crypt_group("Dónal O'Connor", 6), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.state.minions[d.index()].locked = true;
    pick_when(&mut e, own_use(d, 0, Target::None));
    assert!(!e.state.minions[d.index()].locked);
    assert_eq!(blood(&e, d), 4);
    // In torpor too [LSJ 20011022].
    e.state.minions[d.index()].in_torpor = true;
    e.state.minions[d.index()].locked = true;
    e.state.turn_uses.clear();
    offering(&mut e, own_use(d, 0, Target::None));
}

// Murat: "Combat cards requiring Serpentis [ser] cost Followers of Set an
// additional blood when in combat with Murat" (the Ministry, V5): Donnybrook
// [ser] (free; steal 2 blood) costs a Ministry vampire 1 against him, 0
// against another. No V5 Ministry vampire has Serpentis: Marcel is lent it
// (the fixture's Discipline slot).
#[test]
fn murat_makes_serpentis_combat_cards_cost_the_ministry() {
    for murat in [true, false] {
        let mut e = game(2);
        // 1 blood: the steal stays under his capacity of 4.
        let m = e.add_minion(SeatId(0), find_card("Marcel"), 1, false);
        superior(&mut e, m, "SER");
        e.state.minions[m.index()].sect_override = Some(Sect::Anarch);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let v = if murat {
            find_card("Murat")
        } else {
            plain("Nosferatu", 0)
        };
        let z = e.add_minion(SeatId(1), v, 6, false);
        hand(&mut e, 0, &["Donnybrook"]);
        pick(&mut e, bleed(m));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, 2, m));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        // The cost, the 2 stolen, the hand strike taken.
        assert_eq!(blood(&e, m), 1 - murat as u8 + 2 - 1, "{murat}");
    }
}

// ---- Chunk 3: Foundation Exhibit, Bela, Troius, Molly MacDonald ------------

// Foundation Exhibit: "Each non-titled Toreador antitribu you control gets +1
// vote. Any vampire can burn this card as a Ⓓ action that costs 1 blood."
// Another Methuselah's vampire burns it, paying 1.
#[test]
fn foundation_exhibit_votes_for_untitled_toreador_antitribu() {
    let mut e = game(2);
    let untitled = e.add_minion(
        SeatId(0),
        textless(
            |c| c.clan == "Toreador antitribu" && c.title == Title::None,
            0,
        ),
        4,
        false,
    );
    let titled = e.add_minion(
        SeatId(0),
        textless(
            |c| c.clan == "Toreador antitribu" && c.title != Title::None,
            0,
        ),
        4,
        false,
    );
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let fe = in_play(&mut e, 0, "Foundation Exhibit", None);
    assert_eq!(
        (e.state.clan_votes(untitled), e.state.clan_votes(titled)),
        (1, 0)
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick_when(&mut e, use_at(fe, 0, Some(z), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[fe.index()].in_play, "burned");
    assert_eq!(blood(&e, z), 4 - 1);
}

// Bela: "Bela cannot perform political actions. -1 stealth."
#[test]
fn bela_takes_no_political_action() {
    let mut e = game(2);
    let bela = e.add_minion(SeatId(0), find_card("Bela"), 3, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, bela)));
    assert!(req.options.contains(&play(0, 0, other)));
}

// Troius: "cannot cast votes or ballots against blood hunt referendums":
// in favor only.
#[test]
fn troius_votes_for_a_blood_hunt_only() {
    let mut e = game(2);
    let diablerist = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let victim = e.add_minion(SeatId(1), plain("Brujah", 1), 2, true);
    let troius = e.add_minion(SeatId(1), find_card("Troius"), 6, false);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: diablerist,
            action: ActionKind::Diablerie,
            target: Some(victim),
        },
    );
    let req = until(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
    assert!(req.options.contains(&votes(troius, true)));
    assert!(!req.options.contains(&votes(troius, false)));
}

// Molly MacDonald: "+1 vote and +1 bleed if your prey controls a
// non-Camarilla vampire": a Sabbat one in torpor counts; an all-Camarilla
// prey, no.
#[test]
fn molly_bleeds_harder_if_the_prey_controls_a_non_camarilla_vampire() {
    for sabbat in [true, false] {
        let mut e = game(2);
        let molly = e.add_minion(SeatId(0), find_card("Molly MacDonald"), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), with_disc("", Some(Sect::Camarilla), 0), 4, false);
        if sabbat {
            e.add_minion(SeatId(1), with_disc("", Some(Sect::Sabbat), 0), 0, true);
        }
        pick(&mut e, bleed(molly));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e, 1), 10 - 1 - sabbat as i32, "{sabbat}");
        assert_eq!(stat(&e, molly, Stat::Votes), sabbat as i32);
    }
}

// ---- Chunk 3b: Entombment, Carolina Vález, Rutor, Vasily, João Bilé,
// ---- Thomas De Lutrius

// Entombment: "[obt] Strike: burn the opposing ally. [OBT] Strike: send the
// opposing vampire to torpor." [obt] burns a blocking ally; against a
// vampire it cannot be chosen, nor [OBT] against an ally (a strike naming
// the object it acts on cannot be chosen when none exists, advanced rules
// §4.3.3); [OBT] torpors a vampire, wounded [LSJ 20090622-2]. Close range
// only [LSJ 20000110] (CardEffect and Torpor strikes, rules-qa 21.1).
#[test]
fn entombment_burns_an_ally_or_torpors_a_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Aggressive Corpse"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    superior(&mut e, a, "OBT");
    hand(&mut e, 0, &["Entombment"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(ally));
    let req = offering(&mut e, play(0, 0, a));
    assert!(
        !req.options.contains(&play(0, 1, a)),
        "[OBT] names a vampire"
    );
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play, "burned");

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    superior(&mut e, a, "OBT");
    hand(&mut e, 0, &["Entombment"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, play(0, 1, a));
    assert!(!req.options.contains(&play(0, 0, a)), "[obt] names an ally");
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor, "[OBT]: to torpor");

    // At long range (the blocker's .44 Magnum maneuvers): no effect.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(z));
    superior(&mut e, a, "OBT");
    hand(&mut e, 0, &["Entombment"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, use_at(gun, 1, None, Target::None));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].in_torpor, "close range only");
}

// Carolina Vález: "immune to damage from allies and retainers": a blocking
// ally's 2-strength strike, a vampire's retainer's damage, none.
// Immune: not inflicted [LSJ 20010627] [LSJ 20010629] [LSJ 20090304-1].
#[test]
fn carolina_takes_no_damage_from_allies_and_retainers() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Carolina Vález"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Aggressive Corpse"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    pick(&mut e, bleed(c));
    pick_when(&mut e, block(ally));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, c), 6, "the ally's strike");

    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Carolina Vález"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    in_play(&mut e, 1, "Dread Mastiff", Some(z));
    pick(&mut e, bleed(c));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, c),
        6 - 1,
        "the hand strike only, not the mastiff's"
    );
}

// Rutor: "Tzimisce get -1 intercept against Rutor": a Tzimisce's block of
// his bleed fails (intercept -1 against no stealth); a Ventrue's holds.
#[test]
fn rutor_slips_past_tzimisce() {
    for (clan, blocked) in [("Tzimisce", false), ("Ventrue", true)] {
        let mut e = game(2);
        let r = e.add_minion(SeatId(0), find_card("Rutor"), 7, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain(clan, 0), 6, false);
        pick(&mut e, bleed(r));
        pick_when(&mut e, block(z));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e, 1), if blocked { 10 } else { 10 - 1 }, "{clan}");
    }
}

// Vasily: "Contesting a title costs Vasily +1 blood unless you have the
// Edge": 2 blood to keep the contest (1 blood is not enough); with the
// Edge, 1.
#[test]
fn vasily_pays_more_to_contest_without_the_edge() {
    use zvtes::game::Contest;
    for edge in [false, true] {
        let mut e = game(2);
        let v = e.add_minion(SeatId(0), find_card("Vasily"), 1, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let rival = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[rival.index()].granted_title = Some(Title::Prince("Prague"));
        e.state.minions[v.index()].title_contested = true;
        e.state.minions[rival.index()].title_contested = true;
        e.state.seats[0].has_edge = edge;
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        // Yield alone (no Edge, 1 blood) is elided.
        let pay = Choice::PayContest(Contest::Title(v));
        assert_eq!(
            offered_before(&mut e, pay, SeatId(0), WindowLabel::MinionPhase),
            edge,
            "{edge}"
        );
    }
}

// João Bilé: "While João has no title, he cannot block titled vampires. He
// gets +1 stealth during actions to put vampires in play."
#[test]
fn joao_bile_blocks_no_titled_vampire_and_embraces_quietly() {
    for titled in [true, false] {
        let mut e = game(2);
        let j = e.add_minion(SeatId(1), find_card("João Bilé"), 5, false);
        let pick_a = if titled {
            textless(
                |c| c.title != Title::None && c.sect == Some(Sect::Camarilla),
                0,
            )
        } else {
            plain("Brujah", 0)
        };
        let a = e.add_minion(SeatId(0), pick_a, 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        pick(&mut e, bleed(a));
        assert_eq!(
            offered_before(&mut e, block(j), SeatId(0), WindowLabel::MinionPhase),
            !titled,
            "{titled}"
        );
    }
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("João Bilé"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Embrace, The"]);
    pick(&mut e, play(0, 0, j));
    request(&mut e);
    let stealth = e.action().expect("the action").stealth(&e.state);
    assert_eq!(stealth, 2, "the card's +1 and his");
}

// Thomas De Lutrius: "Burn 1 pool after Thomas goes to torpor."
#[test]
fn thomas_de_lutrius_burns_a_pool_going_to_torpor() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Thomas De Lutrius"), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), with_disc("POT", None, 0), 6, false);
    hand(&mut e, 1, &["Torn Signpost"]);
    pick(&mut e, bleed(t));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.index()].in_torpor);
    assert_eq!(pool(&e, 0), 10 - 1);
}

// ---- Chunk 4: Hand of Conrad, …

// Hand of Conrad: "This vampire gets superior Potence [POT]. This vampire
// can move a vampire from your ash heap to your uncontrolled region as an
// action." Named as declared, moved at resolution.
#[test]
fn hand_of_conrad_brings_an_ash_vampire_back_to_the_uncontrolled_region() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let gone = plain("Ventrue", 1);
    e.state.seats[0].ash_crypt.put(gone, Default::default());
    let hand = in_play(&mut e, 0, "Hand of Conrad", Some(a));
    assert_eq!(e.state.discipline_level(a, "pot"), 2);
    pick(&mut e, use_at(hand, 0, Some(a), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].ash_crypt.is_empty());
    assert_eq!(
        e.state.seats[0].uncontrolled.last().map(|u| u.card),
        Some(gone)
    );
}

// Khobar Towers, Al-Khubar: "Lock during your unlock phase to remove a ready
// minion you control from the game and gain X pool, where X is their
// capacity or cost": a vampire for its capacity, an ally (Thadius Zho, 3
// pool) for its cost.
#[test]
fn khobar_towers_cashes_in_a_minion() {
    for ally in [false, true] {
        let mut e = game(2);
        let m = if ally {
            e.add_ally(SeatId(0), find_library("Thadius Zho"))
        } else {
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false)
        };
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let x = if ally {
            3
        } else {
            e.state.capacity_of(m).unwrap() as i32
        };
        let kt = in_play(&mut e, 0, "Khobar Towers, Al-Khubar", None);
        let used = next_turn_taking(&mut e, |o| *o == use_at(kt, 0, None, Target::Minion(m)));
        assert_eq!(used, 1);
        assert!(!e.state.minions[m.index()].in_play, "removed");
        assert_eq!(pool(&e, 0), 10 + x, "{ally}");
    }
}

// Epikasta Rigatos: "You can move an action card Epikasta played from your
// ash heap to your library after resolution (shuffle afterward)": Computer
// Hacking goes to the library, not the ash heap (with an empty library, the
// play's replacement draws it back); declined, to the ash heap.
// Her action's own card, played that action [ANK 20180628].
#[test]
fn epikasta_recycles_her_action_card() {
    for recycle in [true, false] {
        let mut e = game(2);
        let epi = e.add_minion(SeatId(0), find_card("Epikasta Rigatos"), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Computer Hacking"]);
        let hacking = find_library("Computer Hacking");
        pick(&mut e, play(0, 0, epi));
        let req = offering(&mut e, own_use(epi, 0, Target::None));
        assert_eq!(req.window, WindowLabel::ActionAfterResolution);
        if recycle {
            pick(&mut e, own_use(epi, 0, Target::None));
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let s = &e.state.seats[0];
        assert_eq!(s.ash.contains(&hacking), !recycle, "{recycle}");
        assert_eq!(
            s.library.contains(&hacking) || s.hand.contains(&hacking),
            recycle
        );
    }
}

// Camarilla Vitae Slave: "This vampire gets +1 capacity. During their master
// phase, they can choose a Discipline they have at inferior and get +1 level
// of the chosen Discipline until their next master phase": superior through
// the next unlock phase, gone as the next master phase begins (the printed
// text, owner: rules-qa 76.24).
// Kept with the retainer burned [LSJ 19970224].
#[test]
fn camarilla_vitae_slave_raises_capacity_and_a_discipline() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), with_disc("vic", None, 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cap = e.state.capacity_of(v).unwrap();
    let slave = in_play(&mut e, 0, "Camarilla Vitae Slave", Some(v));
    assert_eq!(e.state.capacity_of(v), Some(cap + 1));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    // Its one inferior Discipline: the choice elides.
    pick(&mut e, use_at(slave, 0, None, Target::None));
    request(&mut e);
    assert_eq!(e.state.discipline_level(v, "vic"), 2);
    // The retainer burned: the level stays [LSJ 19970224].
    e.burn_card(slave);
    assert_eq!(
        e.state.discipline_level(v, "vic"),
        2,
        "kept without the retainer"
    );
    // A decision in the next unlock phase (the Chainsaw's damage).
    in_play(&mut e, 0, "Talbot's Chainsaw", Some(b));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    offering(&mut e, Choice::ChooseMinion(b));
    assert_eq!(
        e.state.discipline_level(v, "vic"),
        2,
        "kept through the unlock phase"
    );
    pick(&mut e, Choice::ChooseMinion(b));
    // Nothing to do in that master phase (elided): gone by the minion phase.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.discipline_level(v, "vic"),
        1,
        "gone with the next master phase"
    );
}

// Calebros: "If Calebros is the Prince of New York, any older Camarilla
// vampire controlled by another Methuselah can steal the title from him as
// a Ⓓ action that Calebros cannot block." Offered to an older Camarilla
// vampire of another Methuselah only — not to a younger one nor a
// non-Camarilla one; he cannot block it, his controller's other vampire
// can; the title moves (re-read at resolution against the actor). Then, no
// longer the prince, he is no target.
#[test]
fn calebros_loses_his_title_to_an_older_camarilla_vampire() {
    let mut e = game(3);
    let camarilla = |min: u8, max: u8| {
        move |c: &zvtes::crypt::CryptCard| {
            c.sect == Some(Sect::Camarilla)
                && c.title == Title::None
                && (min..=max).contains(&c.capacity)
        }
    };
    let older = e.add_minion(SeatId(0), textless(camarilla(6, 11), 0), 6, false);
    let younger = e.add_minion(SeatId(0), textless(camarilla(1, 5), 0), 4, false);
    let sabbat = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Sabbat) && c.capacity >= 6, 0),
        6,
        false,
    );
    let cal = e.add_minion(SeatId(1), find_card("Calebros, The Martyr"), 5, false);
    let other = e.add_minion(SeatId(1), textless(camarilla(4, 11), 2), 6, false);
    assert_eq!(e.state.title_of(cal), Title::Prince("New York"));
    let steal = |m: MinionId| Choice::UseAbility {
        source: AbilitySource::Vampire(cal),
        ability: 0,
        actor: Some(m),
        target: Target::Minion(cal),
    };
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let offered: Vec<Choice> = abilities(&req);
    assert_eq!(offered, vec![steal(older)], "{offered:?}");
    assert!(!offered.contains(&steal(younger)) && !offered.contains(&steal(sabbat)));
    pick(&mut e, steal(older));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(other)), "{:?}", req.options);
    assert!(!req.options.contains(&block(cal)), "Calebros cannot block");
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.title_of(older), Title::Prince("New York"));
    assert_eq!(e.state.title_of(cal), Title::None);
    e.state.minions[younger.index()].locked = false;
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(abilities(&req).is_empty(), "no longer the prince");
}

// Dr. Morrow: "Morrow can put 1 graft counter on an ally as a Ⓓ action. An
// ally with any graft counters gets -1 stealth." Allies only; a second
// counter adds nothing (LSJ 20070306); the -1 is his text's, gone with him.
// [LSJ 20070306]; Black Hand, kept through a sect change [LSJ 20070322]
// [ANK 20180807].
#[test]
fn dr_morrow_grafts_an_ally_for_minus_one_stealth() {
    let mut e = game(3);
    let morrow = e.add_minion(SeatId(0), find_card("Dr. Morrow, The Skindoctor"), 5, false);
    e.add_minion(SeatId(0), with_disc("", None, 0), 5, false);
    let ally = e.add_ally(SeatId(1), find_library("Aggressive Corpse"));
    let vamp = e.add_minion(SeatId(1), with_disc("", None, 1), 5, false);
    let graft = |m: MinionId| printed(morrow, 0, Target::Minion(m));
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&graft(ally)));
    assert!(!req.options.contains(&graft(vamp)), "allies only");
    assert_eq!(stat(&e, ally, Stat::Stealth), 0);
    pick(&mut e, graft(ally));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[ally.index()].grafts, 1);
    assert_eq!(stat(&e, ally, Stat::Stealth), -1);
    e.state.minions[ally.index()].grafts = 2;
    assert_eq!(stat(&e, ally, Stat::Stealth), -1, "any counters, once");
    e.state.minions[morrow.index()].in_torpor = true;
    assert_eq!(stat(&e, ally, Stat::Stealth), -1, "in torpor too");
    assert!(e.state.is_black_hand(morrow));
    e.state.minions[morrow.index()].sect_override = Some(Sect::Camarilla);
    assert!(e.state.is_black_hand(morrow), "a trait, kept");
    e.burn(morrow);
    assert_eq!(stat(&e, ally, Stat::Stealth), 0, "his text, gone with him");
}

// Zane: "If Zane is not a prince, he can call a referendum to receive (and
// contest) the title held by a prince as a +1 stealth political action."
// No prince in play, no referendum [LSJ 20100129] [ANK 20191228]; never his
// own controller's (no contest with oneself). A prince chosen (one in
// torpor holds his title): passed, Zane holds the title, contested with
// its holder; a prince himself, he cannot call it again.
#[test]
fn zane_takes_and_contests_a_princes_title() {
    let mut e = game(3);
    let zane = e.add_minion(SeatId(0), find_card("Zane"), 5, false);
    e.add_minion(SeatId(0), with_disc("", None, 0), 5, false);
    e.add_minion(SeatId(1), with_disc("", None, 2), 5, false);
    e.add_minion(SeatId(2), with_disc("", None, 1), 5, false);
    // His own controller's prince: no choice (a contest with oneself).
    e.add_minion(
        SeatId(0),
        textless(|c| matches!(c.title, Title::Prince(_)), 0),
        4,
        false,
    );
    let call = printed(zane, 0, Target::None);
    assert!(
        !request(&mut e).options.contains(&call),
        "no prince to choose"
    );
    let donny = e.add_minion(SeatId(1), find_card("Donny Kowalczyk"), 0, true);
    assert_eq!(e.state.title_of(donny), Title::Prince("Pittsburgh"));
    pick(&mut e, call);
    // The one prince: chosen without a decision (a single option).
    pick_when(&mut e, votes(zane, true));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.minions[zane.index()].granted_title,
        Some(Title::Prince("Pittsburgh"))
    );
    assert!(e.state.minions[zane.index()].title_contested);
    assert!(e.state.minions[donny.index()].title_contested);
    e.state.minions[zane.index()].title_contested = false;
    e.state.minions[zane.index()].locked = false;
    e.state.minions[zane.index()].political_this_turn = false;
    assert!(!request(&mut e).options.contains(&call), "a prince now");
}

// Zane chooses no prince of a city his Methuselah claims (rules-qa 100.1:
// a contest with oneself, his own baron of Pittsburgh); another city's,
// he does.
#[test]
fn zane_chooses_no_prince_of_a_city_his_methuselah_claims() {
    for (city, offered) in [("Pittsburgh", false), ("Paris", true)] {
        let mut e = game(3);
        let zane = e.add_minion(SeatId(0), find_card("Zane"), 5, false);
        let baron = e.add_minion(SeatId(0), with_disc("", None, 0), 5, false);
        e.state.minions[baron.index()].granted_title = Some(Title::Baron(city));
        e.add_minion(SeatId(1), with_disc("", None, 2), 5, false);
        e.add_minion(SeatId(2), with_disc("", None, 1), 5, false);
        let donny = e.add_minion(SeatId(1), find_card("Donny Kowalczyk"), 0, true);
        assert_eq!(e.state.title_of(donny), Title::Prince("Pittsburgh"));
        let call = printed(zane, 0, Target::None);
        assert_eq!(request(&mut e).options.contains(&call), offered, "{city}");
    }
}

// Mustafa: "During the polling step of a political action, each older
// Toreador controlled by another Methuselah can move 1 blood from
// themselves to Mustafa to force him to cast his votes and ballots as that
// Toreador's controller chooses." An older Toreador of another Methuselah
// only, not a younger one; the side chosen; used again by another, the
// last use decides [LSJ 20090304-2], the votes gained since the cast
// kept (rules-qa 76.5); once each Toreador (D7).
#[test]
fn mustafa_casts_his_votes_as_an_older_toreador_chooses() {
    let mut e = game(3);
    let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    // Older than Mustafa (6): capacity 8 and 7, no vote text.
    let old0 = e.add_minion(SeatId(0), find_card("Catalina Vega"), 4, false);
    let mustafa = e.add_minion(SeatId(1), find_card("Mustafa, The Heir"), 4, false);
    let old2 = e.add_minion(SeatId(2), find_card("Tatiana Romanov"), 4, false);
    let young2 = e.add_minion(
        SeatId(2),
        textless(
            |c| c.clan == "Toreador" && c.title == Title::None && c.capacity <= 5,
            0,
        ),
        4,
        false,
    );
    hand(&mut e, 0, &["Kine Resources Contested"]);
    let palace = in_play(&mut e, 1, "Elysium: The Palace of Versailles", None);
    pick(&mut e, play(0, 0, caller));
    pick(&mut e, alloc(0, 3));
    let force = |m: MinionId, favor: bool| Choice::UseAbility {
        source: AbilitySource::Vampire(mustafa),
        ability: if favor { 0 } else { 1 },
        actor: Some(m),
        target: Target::Minion(mustafa),
    };
    let req = offering(&mut e, force(old2, false));
    assert!(
        !req.options.contains(&force(young2, false)),
        "not a younger one"
    );
    pick(&mut e, force(old2, false));
    assert_eq!(blood(&e, old2), 3);
    assert_eq!(blood(&e, mustafa), 5);
    let side = |e: &Engine| {
        let r = e.referendum().expect("referendum");
        r.records
            .iter()
            .find(|r| r.source == zvtes::referendum::Source::Vampire(mustafa))
            .map(|r| (r.favor, r.n))
    };
    assert_eq!(side(&e), Some((false, 2)), "his prince's votes, against");
    pick_when(&mut e, ability(palace.0, 0));
    assert_eq!(side(&e), Some((false, 3)), "a vote gained joins the cast");
    pick_when(&mut e, force(old0, true));
    assert_eq!(
        side(&e),
        Some((true, 3)),
        "the last use decides, the vote gained kept"
    );
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        force(old2, true),
    );
}

// Mustafa, turned after a card added votes to his cast: the votes the card
// gave stay with the cast made again (rules-qa 76.5).
#[test]
fn mustafa_turned_keeps_the_votes_a_card_added() {
    let mut e = game(2);
    let mustafa = e.add_minion(SeatId(0), find_card("Mustafa, The Heir"), 4, false);
    let old = e.add_minion(SeatId(1), find_card("Tatiana Romanov"), 4, false);
    hand(
        &mut e,
        0,
        &["Kine Resources Contested", "Bewitching Oration"],
    );
    pick(&mut e, play(0, 0, mustafa));
    pick_when(&mut e, votes(mustafa, true));
    pick_when(&mut e, play(0, 0, mustafa));
    let _ = request(&mut e); // the play settles
    let side = |e: &Engine| {
        let r = e.referendum().expect("referendum");
        r.records
            .iter()
            .find(|r| r.source == zvtes::referendum::Source::Vampire(mustafa))
            .map(|r| (r.favor, r.n))
    };
    assert_eq!(side(&e), Some((true, 4)), "his 2 and the Oration's 2");
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(mustafa),
            ability: 1,
            actor: Some(old),
            target: Target::Minion(mustafa),
        },
    );
    assert_eq!(side(&e), Some((false, 4)), "turned, the Oration's kept");
}

// Talbot's Chainsaw: "If the bearer is ready during your unlock phase, this
// weapon inflicts 3 unpreventable damage on a ready minion you control. The
// bearer cannot hunt. The bearer can enter combat with a minion as a +1
// stealth Ⓓ action. The bearer gets 1 press (mandatory), only usable to
// continue combat during the first round." No hunt offered; the action at
// another's minion; in the first round, no pass at the press step; at the
// next unlock phase, its controller picks the minion to take 3.
// Not bound to strike with it [LSJ 19971215]; the unlock damage the
// weapon's, the bearer may take it [LSJ 20090304-1] [LSJ 20100514]; its
// press used first, alone to continue [ANK 20200525-1].
#[test]
fn talbots_chainsaw_bars_hunts_presses_and_bites_its_controller() {
    let mut e = game(2);
    let bearer = e.add_minion(SeatId(0), with_disc("", None, 0), 6, false);
    let other = e.add_minion(SeatId(0), with_disc("", None, 1), 6, false);
    let foe = e.add_minion(SeatId(1), with_disc("", None, 2), 6, false);
    let saw = in_play(&mut e, 0, "Talbot's Chainsaw", Some(bearer));
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let hunt = |m: MinionId| Choice::TakeAction {
        minion: m,
        action: ActionKind::Hunt,
        target: None,
    };
    assert!(
        !req.options.contains(&hunt(bearer)),
        "the bearer cannot hunt"
    );
    assert!(req.options.contains(&hunt(other)));
    let enter = use_at(saw, 1, Some(bearer), Target::Minion(foe));
    assert!(req.options.contains(&enter));
    assert!(!req
        .options
        .contains(&use_at(saw, 1, Some(bearer), Target::Minion(other))));
    pick(&mut e, enter);
    assert_eq!(e.action().expect("action").stealth(&e.state), 1);
    // Unblocked: combat. Its prevention once each combat; the press, its
    // only option at the first round's press step (no pass), used without a
    // decision: a second round.
    let prevent = use_at(saw, 2, None, Target::None);
    // Entered with the Chainsaw, not bound to strike with it [LSJ 19971215].
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(req.options.contains(&Choice::HandStrike));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatPrevention);
    assert_eq!(e.combat().expect("combat").round, 1);
    assert!(req.options.contains(&prevent));
    pick(&mut e, prevent);
    until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert_eq!(e.combat().expect("combat").round, 2, "the mandatory press");
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, prevent);
    // Its controller's next unlock phase: a ready minion of theirs, chosen.
    let req = offering(&mut e, Choice::ChooseMinion(other));
    assert_eq!(req.seat, SeatId(0));
    assert!(
        req.options.contains(&Choice::ChooseMinion(bearer)),
        "the bearer too"
    );
    let before = blood(&e, other);
    pick(&mut e, Choice::ChooseMinion(other));
    assert_eq!(blood(&e, other), before.saturating_sub(3));
}

// The Chainsaw's action fizzles once the Chainsaw left the acting minion
// (RTR 19960221): no combat.
// [RTR 19960221]
#[test]
fn talbots_chainsaw_action_fizzles_without_the_chainsaw() {
    let mut e = game(2);
    let bearer = e.add_minion(SeatId(0), with_disc("", None, 0), 6, false);
    let other = e.add_minion(SeatId(0), with_disc("", None, 1), 6, false);
    let foe = e.add_minion(SeatId(1), with_disc("", None, 2), 6, false);
    let saw = in_play(&mut e, 0, "Talbot's Chainsaw", Some(bearer));
    pick(&mut e, use_at(saw, 1, Some(bearer), Target::Minion(foe)));
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    e.state.cards[saw.index()].host = Some(other);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.combat().is_none());
    assert_eq!(blood(&e, foe), 6, "no combat");
}

/// The Molotov Cocktail in play, if any.
fn molotov_in_play(e: &Engine) -> Option<CardId> {
    let card = find_library("Molotov Cocktail");
    e.state
        .cards
        .iter()
        .position(|c| c.card == card && c.in_play)
        .map(|i| CardId(i as u32))
}

fn ash_has(e: &Engine, seat: usize, name: &str) -> bool {
    e.state.seats[seat]
        .ash
        .iter()
        .any(|&c| c == find_library(name))
}

// Molotov Cocktail: "Ranged strike: put this card on this minion; it
// becomes a weapon equipment that can strike: 2R aggravated damage, not
// usable the round it is put in play. Burn after use or at the end of
// combat." Played as the strike, the card itself goes on the striker (not
// a stand-in, out of the ash heap); a press on (Aidan Lyle's), it strikes
// for 2 aggravated in round two and burns. Unused, it burns as the combat
// ends.
#[test]
fn molotov_cocktail_becomes_a_weapon_for_one_strike() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Aidan Lyle"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    hand(&mut e, 0, &["Molotov Cocktail"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::UsePress);
    let saw = molotov_in_play(&e).expect("on Aidan");
    let c = &e.state.cards[saw.index()];
    assert_eq!(c.host, Some(a));
    assert!(!c.effect_only, "the card itself");
    assert!(!ash_has(&e, 0, "Molotov Cocktail"));
    request(&mut e);
    assert_eq!(e.combat().expect("combat").round, 2);
    let before = e.state.minions[z.index()].blood;
    pick_when(&mut e, use_at(saw, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(molotov_in_play(&e).is_none(), "burned after use");
    assert!(ash_has(&e, 0, "Molotov Cocktail"));
    // 2 aggravated: the first sends him to torpor, the second burns 1.
    assert!(e.state.minions[z.index()].in_torpor);
    assert_eq!(e.state.minions[z.index()].blood, before - 1);

    // No press: put on, never used, burned as the combat ends.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    hand(&mut e, 0, &["Molotov Cocktail"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(molotov_in_play(&e).is_none(), "burned at the end of combat");
    assert!(ash_has(&e, 0, "Molotov Cocktail"));
}

// The opposing strike ends combat: the Cocktail's strike never resolves,
// the card is not put on the minion [ANK 20200203-1]; nor when dodged
// (advanced rules §4.6.2).
#[test]
fn molotov_cocktail_is_not_put_when_combat_ends_first() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("pre", 0), 6, false);
    hand(&mut e, 0, &["Molotov Cocktail"]);
    hand(&mut e, 1, &["Majesty"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let molotov = find_library("Molotov Cocktail");
    assert!(
        !e.state.cards.iter().any(|c| c.card == molotov),
        "never put on"
    );
    assert!(
        ash_has(&e, 0, "Molotov Cocktail"),
        "never left the ash heap"
    );

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("cel", 0), 6, false);
    hand(&mut e, 0, &["Molotov Cocktail"]);
    hand(&mut e, 1, &["Side Strike"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let molotov = find_library("Molotov Cocktail");
    assert!(
        !e.state.cards.iter().any(|c| c.card == molotov),
        "dodged: never put on"
    );
    assert!(ash_has(&e, 0, "Molotov Cocktail"));
    assert!(ash_has(&e, 1, "Side Strike"), "the dodge was played");
}

// "Not usable the round it is put in play": an additional strike that
// round (Side Strike [CEL]) is no Cocktail strike.
#[test]
fn molotov_cocktail_does_not_strike_the_round_it_is_put_in_play() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Aidan Lyle"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    superior(&mut e, a, "CEL");
    hand(&mut e, 0, &["Molotov Cocktail", "Side Strike"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::CombatAdditionalStrikes);
    let saw = molotov_in_play(&e).expect("on Aidan");
    pick(&mut e, play(0, 1, a));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::CombatPress,
        use_at(saw, 0, None, Target::None),
    );
    assert_eq!(e.combat().expect("combat").round, 1);
}

// Charnas the Imp: "Put Charnas on a minion (this is a +1 stealth Ⓓ action
// if the target minion is controlled by another Methuselah). Charnas
// inflicts 1 damage on the employer during their unlock phase. … If the
// employer is about to be burned, their controller can put Charnas on
// another minion." Put on another Methuselah's ally: directed (they may
// block), +1 stealth, theirs; their unlock phase, 1 damage burns the
// 1-life ally; they move Charnas on.
// An employ action whatever the target [ANK 20230408] [LSJ 20050313].
#[test]
fn charnas_the_imp_bites_its_employer_and_moves_on() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), plain("Tremere", 0), 6, false);
    e.add_minion(SeatId(0), plain("Tremere", 1), 6, false);
    let ally = e.add_ally(SeatId(1), find_library("Aggressive Corpse"));
    e.state.minions[ally.index()].blood = 1;
    let other = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    hand(&mut e, 0, &["Charnas the Imp"]);
    let employ = play_at(0, 0, t, Target::Minion(ally));
    pick(&mut e, employ);
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert_eq!(e.action().expect("action").stealth(&e.state), 1);
    // An employ action whatever its target [ANK 20230408] [LSJ 20050313].
    assert_eq!(e.action().expect("action").kind, ActionKind::Employ);
    assert!(req.options.contains(&block(other)), "directed at seat 1");
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = find_library("Charnas the Imp");
    let id = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.card == card && c.in_play)
            .expect("Charnas in play") as u32,
    );
    assert_eq!(e.state.cards[id.index()].host, Some(ally));
    assert_eq!(e.state.card_controller(id), SeatId(1));
    let req = offering(&mut e, Choice::ChooseMinion(other));
    assert_eq!(req.seat, SeatId(1));
    assert!(req.options.contains(&Choice::Pass), "can");
    assert!(
        !e.state.minions[ally.index()].in_play,
        "burned by its 1 damage"
    );
    pick(&mut e, Choice::ChooseMinion(other));
    assert_eq!(e.state.cards[id.index()].host, Some(other));
    assert!(e.state.cards[id.index()].in_play);
}

// Two Wrongs: "Play when a minion controlled by a Methuselah other than
// your predator is bleeding you after blocks are declined. That minion is
// now bleeding his or her prey. The next card that would change the
// target of this bleed is canceled as it is played." Seat 0 bleeds seat 1,
// Deflection sends it to seat 2 — not seat 2's predator's bleed (seat 1 is
// its predator, not seat 0): Two Wrongs turns it on seat 0's prey, seat 1;
// seat 1's second Deflection is canceled as played, its 1 blood paid [ANK
// 20260216]; seat 1 is bled. Played after blocks are declined [ANK
// 20211003] [LSJ 20061213], before resolution [LSJ 19980105]; the target
// changed, seat 1 may block again; after the canceled card, no new block
// [LSJ 20061211].
#[test]
fn two_wrongs_turns_a_bleed_and_shields_it() {
    let mut e = game(4);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let d = e.add_minion(SeatId(1), vampire_with("dom", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(3), plain("Ventrue", 1), 4, false);
    // A second vampire for the second copy (one each card per minion an
    // action).
    let d2 = e.add_minion(SeatId(1), vampire_with("dom", 1), 4, false);
    hand(&mut e, 1, &["Deflection", "Deflection"]);
    hand(&mut e, 2, &["Two Wrongs"]);
    pick(&mut e, bleed(b));
    pick_when(&mut e, play_at(0, 0, d, Target::Seat(SeatId(2))));
    let two = master(0, 0, Target::None);
    let req = offering(&mut e, two);
    assert_eq!(req.seat, SeatId(2));
    pick(&mut e, two);
    request(&mut e);
    assert_eq!(e.action().expect("action").bleed_target, Some(SeatId(1)));
    let again = play_at(0, 0, d2, Target::Seat(SeatId(3)));
    pick_when(&mut e, again);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Canceled as played: still seat 1's bleed, the card spent.
    assert_eq!(pool(&e, 1), 9, "seat 1 bled");
    assert_eq!(pool(&e, 2), 10);
    assert_eq!(pool(&e, 3), 10);
    let deflection = find_library("Deflection");
    let spent = e.state.seats[1]
        .ash
        .iter()
        .filter(|&&c| c == deflection)
        .count();
    assert_eq!(spent, 2);
    assert_eq!(blood(&e, d2), 4 - 1, "the canceled card's cost paid");
}

// The Deflection Two Wrongs cancels as it is played is in flight for a
// narrator (the table noted, `Engine::notes`) before it reaches the ash
// heap: a client is told it left the hand, then went to the ash heap.
#[test]
fn two_wrongs_canceled_card_is_noted_in_flight() {
    use zvtes::copies::Place;
    use zvtes::notes::Note;
    let mut e = game(4);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let d = e.add_minion(SeatId(1), vampire_with("dom", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(3), plain("Ventrue", 1), 4, false);
    let d2 = e.add_minion(SeatId(1), vampire_with("dom", 1), 4, false);
    hand(&mut e, 1, &["Deflection", "Deflection"]);
    hand(&mut e, 2, &["Two Wrongs"]);
    pick(&mut e, bleed(b));
    pick_when(&mut e, play_at(0, 0, d, Target::Seat(SeatId(2))));
    let two = master(0, 0, Target::None);
    offering(&mut e, two);
    pick(&mut e, two);
    request(&mut e);
    e.notes = Some(Vec::new());
    pick_when(&mut e, play_at(0, 0, d2, Target::Seat(SeatId(3))));
    let deflection = find_library("Deflection");
    let in_flight = e.notes.as_ref().expect("noting").iter().any(|n| match n {
        Note::Ledger { ledger, .. } => ledger
            .census
            .iter()
            .any(|&(_, card, p)| card == deflection && p == Place::InFlight(1)),
        _ => false,
    });
    assert!(in_flight, "the canceled Deflection never noted in flight");
}

// Form of Corruption: "If your prey gets the Edge anew, you can add 1
// counter to this card. If the number of counters on this card equals or
// exceeds the amount of blood on a vampire controlled by your prey during
// your master phase, you can burn this card to steal that vampire." The
// prey bleeds and takes the Edge: a counter; its 1-blood vampire stolen in
// the next master phase, not its 4-blood one; the card burned.
#[test]
fn form_of_corruption_steals_a_drained_vampire() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Ministry", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ministry", 1), 4, false);
    let bleeder = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let weak = e.add_minion(SeatId(1), plain("Ventrue", 1), 1, false);
    e.add_minion(SeatId(2), plain("Brujah", 0), 4, false);
    let form = in_play(&mut e, 0, "Form of Corruption", None);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(bleeder));
    let add = Choice::ResolveRider { card: form.0 };
    let req = offering(&mut e, add);
    assert_eq!(req.seat, SeatId(0));
    pick(&mut e, add);
    assert_eq!(e.state.cards[form.index()].counters, 1);
    let steal = |m: MinionId| use_at(form, 0, None, Target::Minion(m));
    let req = offering(&mut e, steal(weak));
    assert_eq!(req.seat, SeatId(0));
    assert!(
        !req.options.contains(&steal(bleeder)),
        "more blood than counters"
    );
    pick(&mut e, steal(weak));
    assert_eq!(e.state.minions[weak.index()].owner, SeatId(0));
    assert!(!e.state.cards[form.index()].in_play, "burned");
}

// Form of Corruption steals no vampire whose unique title its Methuselah
// claims (rules-qa 100.1: a contest with oneself); another city's prince
// is stolen.
#[test]
fn form_of_corruption_steals_no_title_its_methuselah_claims() {
    use zvtes::crypt::Title;
    for (city, offered) in [("Paris", false), ("Berlin", true)] {
        let mut e = game(3);
        let own = e.add_minion(SeatId(0), plain("Ministry", 0), 4, false);
        e.state.minions[own.index()].granted_title = Some(Title::Prince("Paris"));
        e.add_minion(SeatId(0), plain("Ministry", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let weak = e.add_minion(SeatId(1), plain("Ventrue", 1), 1, false);
        e.state.minions[weak.index()].granted_title = Some(Title::Prince(city));
        e.add_minion(SeatId(2), plain("Brujah", 0), 4, false);
        let form = in_play(&mut e, 0, "Form of Corruption", None);
        e.state.cards[form.index()].counters = 1;
        e.at_master_phase();
        let req = request(&mut e);
        let steal = use_at(form, 0, None, Target::Minion(weak));
        assert_eq!(
            req.options.contains(&steal),
            offered,
            "{city}: {:?}",
            req.options
        );
    }
}

// "Anew": the prey holding the Edge already, a bleed of theirs adds none.
#[test]
fn form_of_corruption_counts_only_a_new_edge() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Ministry", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ministry", 1), 4, false);
    let bleeder = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 1, false);
    e.add_minion(SeatId(2), plain("Brujah", 0), 4, false);
    let form = in_play(&mut e, 0, "Form of Corruption", None);
    e.state.seats[1].has_edge = true;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].has_edge);
    pick(&mut e, bleed(bleeder));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::ResolveRider { card: form.0 },
    );
    assert_eq!(e.state.cards[form.index()].counters, 0);
}

// Game of Malkav: "Each other Methuselah secretly chooses a number between
// 1 and 5 inclusive; you choose a number between 1 and 6 inclusive. …
// Each Methuselah gains the amount of pool they chose unless another
// Methuselah chose exactly 1 less than they did, in which case, they burn
// that amount of pool instead." Seat 1 picks 3, seat 2 picks 2, the player
// 4: seat 2 gains 2, seat 1 (2 is one less) burns 3, the player (3 is one
// less) burns 4; nothing moves before the last pick.
#[test]
fn game_of_malkav_pays_the_undercut_numbers() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Malkavian", 0), 4, false);
    e.add_minion(SeatId(0), plain("Malkavian", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Game of Malkav"]);
    let malkav = master(0, 0, Target::None);
    let req = offering(&mut e, malkav);
    assert_eq!(req.seat, SeatId(0));
    let before = [pool(&e, 0), pool(&e, 1), pool(&e, 2)];
    pick(&mut e, malkav);
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(1));
    assert_eq!(req.options, (1..=5).map(Choice::SetX).collect::<Vec<_>>());
    pick(&mut e, Choice::SetX(3));
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(2));
    pick(&mut e, Choice::SetX(2));
    assert_eq!([pool(&e, 0), pool(&e, 1), pool(&e, 2)], before, "not yet");
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(0));
    assert_eq!(req.options, (1..=6).map(Choice::SetX).collect::<Vec<_>>());
    pick(&mut e, Choice::SetX(4));
    assert_eq!(pool(&e, 2), before[2] + 2);
    assert_eq!(pool(&e, 1), before[1] - 3);
    assert_eq!(pool(&e, 0), before[0] - 4);
}

// Alastor: "Choose a ready Camarilla vampire. Successful referendum means
// you search your library for an equipment card and put this card and the
// equipment on the chosen vampire (ignore requirements; shuffle
// afterward); pay half the cost rounded down of the equipment. The
// attached vampire can enter combat with a vampire as a +1 stealth Ⓓ
// action. The attached vampire cannot commit diablerie." A justicar calls
// it on himself: AK-47 (5 pool) found for 2; both on him; then no
// diablerie for him, and his combat action is offered.
#[test]
fn alastor_equips_its_vampire_and_bars_diablerie() {
    let mut e = game(2);
    let j = e.add_minion(
        SeatId(0),
        textless(|c| matches!(c.title, Title::Justicar(_)), 0),
        6,
        false,
    );
    let foe = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let torpid = e.add_minion(SeatId(1), plain("Ventrue", 1), 2, true);
    hand(&mut e, 0, &["Alastor"]);
    // The AK-47 deep in the library, past the draws (the hand draws up).
    let filler = find_library("Deflection");
    for _ in 0..10 {
        e.state.seats[0].library.push(filler);
    }
    e.state.seats[0].library.push(find_library("AK-47"));
    for _ in 0..10 {
        e.state.seats[0].library.push(filler);
    }
    pick(&mut e, play(0, 0, j));
    pick_when(&mut e, Choice::ChooseMinion(j));
    pick_when(&mut e, votes(j, true));
    let ak = Choice::FindCard(find_library("AK-47"));
    let req = offering(&mut e, ak);
    assert_eq!(req.window, WindowLabel::LibrarySearch);
    assert!(req.options.contains(&Choice::Pass), "optional");
    let before = pool(&e, 0);
    pick(&mut e, ak);
    assert_eq!(pool(&e, 0), before - 2, "half of 5, rounded down");
    let on = |name: &str| {
        let card = find_library(name);
        e.state
            .cards
            .iter()
            .position(|c| c.card == card && c.in_play && c.host == Some(j))
            .map(|i| CardId(i as u32))
    };
    let alastor = on("Alastor").expect("Alastor on him");
    assert!(on("AK-47").is_some());
    e.state.minions[j.index()].locked = false;
    e.state.minions[j.index()].acted.clear();
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let diablerie = Choice::TakeAction {
        minion: j,
        action: ActionKind::Diablerie,
        target: Some(torpid),
    };
    assert!(!req.options.contains(&diablerie), "cannot commit diablerie");
    assert!(req
        .options
        .contains(&use_at(alastor, 0, Some(j), Target::Minion(foe))));
}

// The round's vampires whose text is their sect and title only (Mosfair's
// Black Hand too): nothing to define (no `VampireDef`), the generated
// traits as printed.
#[test]
fn the_text_only_vampires_carry_their_printed_traits() {
    use Sect::{Camarilla, Sabbat};
    let cases: [(&str, Sect, Title, bool); 18] = [
        ("Arthur Denholm", Camarilla, Title::None, false),
        ("Claus Wegener", Camarilla, Title::None, false),
        ("Gwendolyn Fleming", Camarilla, Title::None, false),
        ("Ilias cel Frumos", Sabbat, Title::None, false),
        ("Janine", Sabbat, Title::None, false),
        ("Jonathan Gursel", Sabbat, Title::None, false),
        ("Justine Chen, Innocent", Sabbat, Title::None, false),
        ("Klaus Konrecht", Sabbat, Title::Cardinal, false),
        ("Lectora", Sabbat, Title::None, false),
        ("Loonar", Sabbat, Title::None, false),
        ("Monique Kim", Sabbat, Title::None, false),
        ("Mosfair", Sabbat, Title::None, true),
        ("Muhsin Samir", Camarilla, Title::None, false),
        (
            "Sheldon, Lord of the Clog",
            Camarilla,
            Title::Justicar("Nosferatu"),
            false,
        ),
        ("America Johnson", Sabbat, Title::None, false),
        ("Conchita Morales", Sabbat, Title::None, false),
        ("Guggenheim", Sabbat, Title::None, false),
        ("Percival", Sabbat, Title::Archbishop("Guadalajara"), false),
    ];
    let mut e = game(2);
    for (name, sect, title, black_hand) in cases {
        let card = find_card(name);
        assert!(
            zvtes::cards::vampire(card).is_none(),
            "{name}: no text to define"
        );
        let m = e.add_minion(SeatId(0), card, 1, false);
        assert_eq!(e.state.sect_of(m), Some(sect), "{name}");
        assert_eq!(e.state.title_of(m), title, "{name}");
        assert_eq!(e.state.is_black_hand(m), black_hand, "{name}");
    }
}

// Banu Haqim Justicar beside one's own Kasim Bayar: no vampire of his
// controller's can be chosen (a title card never makes its holder contest
// themselves, advanced rules §1.13.1), Kasim neither, so the card cannot be
// played: its terms cannot be met [LSJ 20100129] [ANK 20191228].
#[test]
fn banu_haqim_justicar_never_contests_its_own_kasim() {
    let mut e = game(2);
    let b = e.add_minion(
        SeatId(0),
        textless(
            |c| c.clan == "Banu Haqim" && c.sect == Some(Sect::Camarilla) && c.title == Title::None,
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(0), find_card("Kasim Bayar"), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Banu Haqim Justicar"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, b)), "{:?}", req.options);
}

/// A justicar of seat 0's calls Alastor on himself, `cards` deep in the
/// library: the engine at the search, and the justicar.
fn alastor_search(cards: &[&str]) -> (Engine, MinionId) {
    let mut e = game(2);
    let j = e.add_minion(
        SeatId(0),
        textless(|c| matches!(c.title, Title::Justicar(_)), 0),
        6,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Alastor"]);
    let filler = find_library("Deflection");
    for _ in 0..10 {
        e.state.seats[0].library.push(filler);
    }
    for name in cards {
        e.state.seats[0].library.push(find_library(name));
    }
    for _ in 0..10 {
        e.state.seats[0].library.push(filler);
    }
    pick(&mut e, play(0, 0, j));
    pick_when(&mut e, votes(j, true));
    (e, j)
}

/// The card named `name` in play on `m`, if any.
fn on_minion(e: &Engine, name: &str, m: MinionId) -> Option<CardId> {
    let card = find_library(name);
    e.state
        .cards
        .iter()
        .position(|c| c.card == card && c.in_play && c.host == Some(m))
        .map(|i| CardId(i as u32))
}

// Alastor: "Finding equipment is optional. When no equipment is found,
// alastor is still attached." [LSJ 20050331-2]
#[test]
fn alastor_finding_equipment_is_optional() {
    let (mut e, j) = alastor_search(&["AK-47"]);
    let req = offering(&mut e, Choice::FindCard(find_library("AK-47")));
    assert!(req.options.contains(&Choice::Pass));
    pick(&mut e, Choice::Pass);
    assert!(on_minion(&e, "Alastor", j).is_some(), "attached anyway");
    assert!(on_minion(&e, "AK-47", j).is_none());
}

// Alastor: "If the weapon retrieved costs blood, that cost is paid by the
// vampire chosen by the terms." [LSJ 20040518] Zaire River Ferry, 2 blood:
// half, 1, from the chosen vampire, no pool.
#[test]
fn alastor_blood_cost_is_the_chosen_vampires() {
    let (mut e, j) = alastor_search(&["Zaire River Ferry"]);
    let ferry = Choice::FindCard(find_library("Zaire River Ferry"));
    offering(&mut e, ferry);
    let (pool_before, blood_before) = (pool(&e, 0), blood(&e, j));
    pick(&mut e, ferry);
    assert_eq!(blood(&e, j), blood_before - 1);
    assert_eq!(pool(&e, 0), pool_before);
    assert!(on_minion(&e, "Zaire River Ferry", j).is_some());
}

// Alastor puts the equipment on, it does not equip with it: what it does
// as it is equipped does not happen — Shilmulo Tarot stores no card [LSJ
// 20090506] — and its requirements do not apply: a Ravnos card on a
// justicar of another clan [LSJ 20100204] [LSJ 20040518-2] [LSJ
// 20100302-1]. The vampire is not locked by it [LSJ 20090415] [LSJ
// 20100119] (no equip action).
#[test]
fn alastor_puts_on_without_equipping() {
    let (mut e, j) = alastor_search(&["Shilmulo Tarot"]);
    let tarot = Choice::FindCard(find_library("Shilmulo Tarot"));
    offering(&mut e, tarot);
    pick(&mut e, tarot);
    let id = on_minion(&e, "Shilmulo Tarot", j).expect("on the justicar");
    assert!(
        e.state.cards[id.index()].stored.is_empty(),
        "no card stored"
    );
}

// Alastor with no vampire to choose — the justicar already carries one ("a
// vampire can have only one Alastor"), the other Camarilla vampire in
// torpor — is not playable: its terms cannot be met [LSJ 20100129] [ANK
// 20191228].
#[test]
fn alastor_needs_a_vampire_to_choose() {
    let mut e = game(2);
    let j = e.add_minion(
        SeatId(0),
        textless(|c| matches!(c.title, Title::Justicar(_)), 0),
        6,
        false,
    );
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Alastor"]);
    assert!(request(&mut e).options.contains(&play(0, 0, j)));
    in_play(&mut e, 0, "Alastor", Some(j));
    e.state.minions[z.index()].in_torpor = true;
    assert!(!request(&mut e).options.contains(&play(0, 0, j)));
}

// Lasombra Justicar with no ready Camarilla Lasombra: not playable [LSJ
// 20100129] [ANK 20191228].
#[test]
fn lasombra_justicar_needs_a_ready_camarilla_lasombra() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), with_disc("", Some(Sect::Sabbat), 0), 4, false);
    hand(&mut e, 0, &["Lasombra Justicar"]);
    assert!(!request(&mut e).options.contains(&play(0, 0, caller)));
    e.add_minion(
        SeatId(1),
        textless(
            |c| c.clan == "Lasombra" && c.sect == Some(Sect::Camarilla) && c.title == Title::None,
            0,
        ),
        4,
        false,
    );
    assert!(
        request(&mut e).options.contains(&play(0, 0, caller)),
        "one to choose"
    );
}

// Charnas the Imp on another Methuselah's vampire: that Methuselah ousted,
// Charnas is removed from the game with their cards, not burned [ANK
// 20180408].
#[test]
fn charnas_is_removed_with_its_ousted_controller() {
    let mut e = game(3);
    let t = e.add_minion(SeatId(0), plain("Tremere", 0), 6, false);
    let t2 = e.add_minion(SeatId(0), plain("Tremere", 1), 6, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(2), plain("Brujah", 0), 6, false);
    hand(&mut e, 0, &["Charnas the Imp"]);
    pick(&mut e, play_at(0, 0, t, Target::Minion(v)));
    pick_when(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let id = on_minion(&e, "Charnas the Imp", v).expect("on seat 1's vampire");
    // Seat 1 at 1 pool, bled out.
    e.state.seats[1].pool = 1;
    pick(&mut e, bleed(t2));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert!(!e.state.cards[id.index()].in_play);
    let charnas = find_library("Charnas the Imp");
    assert!(!e.state.seats[0].ash.contains(&charnas), "not burned");
}

// Kateline Nadasdy's abstention after the vampire voted cancels its votes
// [LSJ 20100311], a priscus's ballots too [LSJ 19971001]; usable while she
// is locked or in torpor, once each referendum [ANK 20220705] [LSJ
// 20100705] [LSJ 20011022] [LSJ 20100902-2] [ANK 20180307-2].
#[test]
fn kateline_cancels_votes_cast_from_torpor() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    let other = e.add_minion(
        SeatId(0),
        textless(|c| c.title == Title::Priscus, 0),
        4,
        false,
    );
    let k = e.add_minion(SeatId(1), find_card("Kateline Nadasdy"), 3, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, caller));
    pick(&mut e, alloc(0, 3));
    pick_when(&mut e, votes(other, true));
    let cast = |e: &Engine| {
        e.referendum()
            .expect("referendum")
            .records
            .iter()
            .any(|r| r.source == zvtes::referendum::Source::Vampire(other) && !r.canceled)
    };
    assert!(cast(&e));
    let ballots = e
        .referendum()
        .expect("referendum")
        .records
        .iter()
        .find(|r| r.source == zvtes::referendum::Source::Vampire(other))
        .map_or(0, |r| r.ballots);
    assert!(ballots > 0, "a priscus casts a ballot");
    let force = own_use(k, 0, Target::Minion(other));
    pick_when(&mut e, force);
    assert!(!cast(&e), "its cast votes canceled");
    assert_eq!(blood(&e, k), 2, "paid in torpor");
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        own_use(k, 0, Target::Minion(caller)),
    );
}

// Gem Ghastly's "as if Ventrue" holds for cards played normally only:
// Piper puts no Ventrue ally in play through her [LSJ 20050119]; a real
// Ventrue Anarch may.
#[test]
fn gem_ghastly_lends_piper_no_ventrue() {
    let piper_for = |by: u16| {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), by, 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        // An Anarch, as Piper requires.
        e.state.minions[m.index()].sect_override = Some(Sect::Anarch);
        hand(&mut e, 0, &["Piper", "Political Ally"]);
        (e, m)
    };
    let (mut e, gem) = piper_for(find_card("Gem Ghastly"));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == gem),
    );
    let (mut e, ven) = piper_for(plain("Ventrue", 1));
    offering(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(ven),
            target: Target::None,
        },
    );
}

// Hand of Conrad: its action fizzles once the Hand left the acting vampire
// [RTR 19960221]; its superior Potence is not the vampire's while an effect
// bars it from using equipment (Drawing Out the Beast, Conquer the Beast)
// [RTR 20010711].
#[test]
fn hand_of_conrad_fizzles_off_its_bearer_and_its_potence_needs_equipment() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0]
        .ash_crypt
        .put(plain("Ventrue", 1), Default::default());
    let conrad = in_play(&mut e, 0, "Hand of Conrad", Some(a));
    pick(&mut e, use_at(conrad, 0, Some(a), Target::None));
    request(&mut e);
    e.state.cards[conrad.index()].host = Some(b);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.seats[0].ash_crypt.is_empty(), "fizzled");

    let mut e = game(2);
    let x = e.add_minion(SeatId(0), vampire_with("ani", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("cel", 0), 6, false);
    hand(&mut e, 1, &["Side Strike"]);
    in_play(&mut e, 1, "Hand of Conrad", Some(z));
    assert_eq!(e.state.discipline_level(z, "pot"), 2);
    hand(&mut e, 0, &["Drawing Out the Beast"]);
    pick(&mut e, bleed(x));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, x));
    // Its dodge in hand: a strike decision mid-combat.
    until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert_eq!(
        e.state.discipline_level(z, "pot"),
        0,
        "barred from equipment"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.discipline_level(z, "pot"), 2, "after the combat");

    // Conquer the Beast: "this vampire cannot use equipment this combat".
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), vampire_with("ani", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("cel", 0), 6, false);
    hand(&mut e, 1, &["Side Strike"]);
    in_play(&mut e, 0, "Hand of Conrad", Some(x));
    hand(&mut e, 0, &["Conquer the Beast"]);
    pick(&mut e, bleed(x));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert_eq!(
        e.state.discipline_level(x, "pot"),
        0,
        "its own equipment barred"
    );
}

// The equipment bar lives with its combat (rules-qa 76.16), one cut short
// with the turn too: Fame ousts the acting Methuselah mid-combat, the turn
// passes, the barred blocker has its Hand of Conrad's Potence again.
#[test]
fn an_equipment_bar_ends_with_a_combat_cut_short_by_the_turn() {
    let mut e = game(3);
    let x = e.add_minion(SeatId(0), vampire_with("ani", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("cel", 0), 6, false);
    e.add_minion(SeatId(2), plain("Brujah", 2), 4, false);
    e.state.seats[0].pool = 3;
    in_play(&mut e, 1, "Fame", Some(x));
    in_play(&mut e, 1, "Hand of Conrad", Some(z));
    hand(&mut e, 0, &["Drawing Out the Beast"]);
    hand(&mut e, 1, &["Side Strike"]);
    pick(&mut e, bleed(x));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, x));
    // Its dodge in hand: a strike decision mid-combat.
    until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert_eq!(e.state.discipline_level(z, "pot"), 0, "barred");
    pick(&mut e, Choice::HandStrike);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].ousted, "the turn cut short");
    assert!(e.state.equipment_barred.is_empty());
    assert_eq!(e.state.discipline_level(z, "pot"), 2);
}

// Aching Beauty's burn is applied as the block is declared, before any
// card: the action then ended by Change of Target still cost the blocker
// [RTR 19991206] [ANK 20220116] [ANK 20211207] [ANK 20210627] [RTR
// 19991206] [LSJ 20091125].
#[test]
fn aching_beauty_burns_even_if_the_action_then_ends() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), plain("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Aching Beauty", Some(t));
    hand(&mut e, 0, &["Change of Target"]);
    pick(&mut e, bleed(t));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, t));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Change of Target")));
    assert_eq!(pool(&e, 1), 10 - 1);
    assert!(e.combat().is_none());
}

// Mask of a Thousand Faces after Aching Beauty burned the blocker's pool:
// an effect the masking vampire could not have used, so no mask [RTR
// 19980623] [RTR 20030519] [LSJ 20030520] [RTR 20041202]. Without Aching
// Beauty, the mask is offered.
#[test]
fn no_mask_after_aching_beauty_burns() {
    let blocked = |beauty: bool| {
        let mut e = game(2);
        let t = e.add_minion(SeatId(0), plain("Toreador", 0), 4, false);
        let o = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if beauty {
            in_play(&mut e, 0, "Aching Beauty", Some(t));
        }
        hand(&mut e, 0, &["Mask of a Thousand Faces"]);
        pick(&mut e, bleed(t));
        pick_when(&mut e, block(z));
        (e, play(0, 0, o))
    };
    let (mut e, mask) = blocked(false);
    offering(&mut e, mask);
    // Offered while the attempt is pending; not once the block lands and
    // Aching Beauty burns.
    let (mut e, mask) = blocked(true);
    // The burn lands as a request runs.
    while request(&mut e).options.contains(&mask) && pool(&e, 1) == 10 {
        let req = request(&mut e);
        e.choose(fallback(&req));
    }
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, mask);
    assert_eq!(pool(&e, 1), 10 - 1);
}

// Alastor ignores requirements, not "a minion can have only one vehicle"
// (never a second, by any route: rules-digest §3.17, rules-qa 70.20): a
// second Sport Bike is not offered to a vampire carrying one, an AK-47 is.
#[test]
fn alastor_puts_no_second_vehicle() {
    let bike = Choice::FindCard(find_library("Sport Bike"));
    let ak = Choice::FindCard(find_library("AK-47"));
    let (mut e, _) = alastor_search(&["Sport Bike", "AK-47"]);
    let req = offering(&mut e, ak);
    assert!(req.options.contains(&bike), "the control");
    let (mut e, j) = alastor_search(&["Sport Bike", "AK-47"]);
    in_play(&mut e, 0, "Sport Bike", Some(j));
    let req = offering(&mut e, ak);
    assert!(!req.options.contains(&bike), "only one vehicle");
}

// Coma: "Strike: send the opposing vampire to torpor": not chosen against
// an ally (a strike naming the kind it acts on, advanced rules §4.3.3;
// rules-qa 76.11); against a vampire, offered.
#[test]
fn coma_is_not_chosen_against_an_ally() {
    let blocked_by = |ally: bool| {
        let mut e = game(2);
        // Erebus with superior Dementation, as in twda_round7.
        let a = e.add_minion(SeatId(0), find_card("Erebus"), 6, false);
        superior(&mut e, a, "dem");
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let b = if ally {
            e.add_ally(SeatId(1), find_library("Aggressive Corpse"))
        } else {
            e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false)
        };
        e.add_minion(SeatId(1), plain("Ventrue", 1), 6, false);
        hand(&mut e, 0, &["Coma"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(b));
        (e, play(0, 0, a))
    };
    let (mut e, coma) = blocked_by(false);
    offering(&mut e, coma);
    let (mut e, coma) = blocked_by(true);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, coma);
}

// Epikasta's own action blocked: her card still goes to the library once
// the action is over (rules-qa 76.22: blocked too).
#[test]
fn epikasta_recycles_a_blocked_action_card() {
    let mut e = game(2);
    let epi = e.add_minion(SeatId(0), find_card("Epikasta Rigatos"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Computer Hacking"]);
    pick(&mut e, play(0, 0, epi));
    pick_when(&mut e, block(z));
    pick_when(&mut e, own_use(epi, 0, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let hacking = find_library("Computer Hacking");
    let s = &e.state.seats[0];
    assert!(!s.ash.contains(&hacking));
    assert!(s.library.contains(&hacking) || s.hand.contains(&hacking));
}
