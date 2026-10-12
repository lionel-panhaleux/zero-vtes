//! Slice 9a: printed vampire text — plain stat modifiers
//! (`cards::vampires`), a vampire's whole text or the plain half of a
//! two-part one (rules-qa 13.1), read live by `GameState::printed_mod`
//! on bleeds, stealth, intercept, strength, votes and hand size. Windows
//! with a single option are elided, so the scripts only stop where a real
//! decision exists.

use zvtes::cards::{self, vampires::VAMPIRES, AbilitySource, Stat, Target};
use zvtes::crypt::CRYPT;
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, ActionId, CardId, CardInPlay, PrintedCtx};

mod common;
use common::*;

fn add(e: &mut Engine, seat: u8, name: &str) -> MinionId {
    e.add_minion(SeatId(seat), find_card(name), 4, false)
}

/// A library card in play, on `host` if any.
fn in_play(e: &mut Engine, name: &str, owner: u8, host: Option<MinionId>) -> usize {
    e.state
        .cards
        .push(CardInPlay::new(find_library(name), SeatId(owner), host, 0));
    e.state.cards.len() - 1
}

fn during(kind: ActionKind, directed: bool, opponent: Option<MinionId>) -> PrintedCtx {
    PrintedCtx {
        kind: Some(kind),
        directed: Some(directed),
        opponent,
        ..PrintedCtx::default()
    }
}

fn in_combat_with(opponent: MinionId) -> PrintedCtx {
    PrintedCtx {
        opponent: Some(opponent),
        ..PrintedCtx::default()
    }
}

/// 2 seats at seat 0's minion phase, turn 4.
fn bare() -> Engine {
    let mut e = Engine::bare(2);
    e.state.turn = 4;
    e
}

// Catalina Vega's "+1 bleed": her bleed takes 2 pool.
#[test]
fn a_printed_bleed_bonus_adds_to_the_bleed_amount() {
    let mut e = bare();
    let catalina = add(&mut e, 0, "Catalina Vega");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Nosferatu", 0), 4, false);
    let before = e.state.seats[1].pool;
    pick(&mut e, action(catalina, ActionKind::Bleed));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, before - 2);
}

// Hesha Ruhadze: +1 bleed for each unique equipment on him — not a
// non-unique one, nor a contested unique one.
#[test]
fn hesha_ruhadze_bleeds_one_more_per_uncontested_unique_equipment() {
    let mut e = bare();
    let hesha = add(&mut e, 0, "Hesha Ruhadze");
    let bleed = during(ActionKind::Bleed, true, None);
    assert_eq!(e.state.printed_mod(hesha, Stat::Bleed, bleed), 0);
    in_play(&mut e, "Kali's Fang", 0, Some(hesha));
    in_play(&mut e, "Ivory Bow", 0, Some(hesha));
    in_play(&mut e, ".44 Magnum", 0, Some(hesha));
    let femur = in_play(&mut e, "Femur of Toomler", 0, Some(hesha));
    e.state.cards[femur].contested = true;
    assert_eq!(e.state.printed_mod(hesha, Stat::Bleed, bleed), 2);
}

// A hunt is +1 stealth: Rinaldo Albizzi's +1 intercept makes his block
// attempt reachable, while a vampire with no intercept is not offered one.
#[test]
fn a_printed_intercept_bonus_opens_a_block_on_an_undirected_action() {
    let mut e = bare();
    let hunter = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    let rinaldo = add(&mut e, 1, "Rinaldo Albizzi");
    let other = e.add_minion(SeatId(1), plain("Nosferatu", 0), 4, false);
    pick(&mut e, action(hunter, ActionKind::Hunt));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req
        .options
        .contains(&Choice::DeclareBlock { minion: rinaldo }));
    assert!(!req
        .options
        .contains(&Choice::DeclareBlock { minion: other }));
}

// Ariane's "-1 stealth during undirected actions": her hunt has no
// stealth, so a vampire with no intercept may block it.
#[test]
fn ariane_hunts_with_no_stealth() {
    let mut e = bare();
    let ariane = e.add_minion(SeatId(0), find_card("Ariane"), 1, false);
    let other = e.add_minion(SeatId(1), plain("Nosferatu", 0), 4, false);
    pick(&mut e, action(ariane, ActionKind::Hunt));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req
        .options
        .contains(&Choice::DeclareBlock { minion: other }));
}

// Intercept read on the action performed and the acting minion.
#[test]
fn printed_intercept_reads_the_action_and_the_acting_minion() {
    let mut e = bare();
    let st = |e: &Engine, m, ctx| e.state.printed_mod(m, Stat::Intercept, ctx);
    let actor = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let bleed = during(ActionKind::Bleed, true, Some(actor));
    let hunt = during(ActionKind::Hunt, false, Some(actor));
    let political = during(ActionKind::Political, false, Some(actor));

    let castellan = add(&mut e, 1, "Castellan");
    assert_eq!((st(&e, castellan, bleed), st(&e, castellan, hunt)), (1, -1));
    let dowager = add(&mut e, 1, "Dowager, The");
    assert_eq!((st(&e, dowager, bleed), st(&e, dowager, hunt)), (1, 0));
    let martina = add(&mut e, 1, "Martina Srnankova");
    assert_eq!((st(&e, martina, bleed), st(&e, martina, political)), (1, 0));
    let oluwa = add(&mut e, 1, "Oluwafunmilayo");
    assert_eq!((st(&e, oluwa, political), st(&e, oluwa, bleed)), (1, 0));
    let hafthor = add(&mut e, 1, "Hafthor Thorsteinsson");
    assert_eq!(st(&e, hafthor, hunt), 1);

    // Branimira: while her Methuselah has 9 or fewer pool.
    let branimira = add(&mut e, 1, "Branimira");
    e.state.seats[1].pool = 10;
    assert_eq!(st(&e, branimira, bleed), 0);
    e.state.seats[1].pool = 9;
    assert_eq!(st(&e, branimira, bleed), 1);

    // Bret Stryker: -1 against a titled acting vampire; a contested title
    // is no title.
    let bret = add(&mut e, 1, "Bret Stryker");
    let alexa = add(&mut e, 0, "Alexa Draper");
    let lenny = add(&mut e, 0, "Lenny Burkhead"); // primogen
    assert_eq!(st(&e, bret, bleed), 0);
    assert_eq!(
        st(&e, bret, during(ActionKind::Bleed, true, Some(lenny))),
        -1
    );
    let by_alexa = during(ActionKind::Bleed, true, Some(alexa));
    assert_eq!(st(&e, bret, by_alexa), -1);
    e.state.minions[alexa.index()].title_contested = true;
    assert_eq!(st(&e, bret, by_alexa), 0);

    // Azucena (capacity 5): +1 against a younger Lasombra, -1 against an
    // older one, nothing against a younger non-Lasombra.
    let azucena = add(&mut e, 1, "Azucena");
    let braden = add(&mut e, 0, "Braden White"); // Lasombra, 4
    let kamaluddin = add(&mut e, 0, "Kamaluddin"); // Lasombra, 6
    let at = |m| during(ActionKind::Hunt, false, Some(m));
    assert_eq!(st(&e, azucena, at(braden)), 1);
    assert_eq!(st(&e, azucena, at(kamaluddin)), -1);
    assert_eq!(st(&e, azucena, at(actor)), 0);
}

// Massimiliano's "+1 strength": his hand strike does 2 damage.
#[test]
fn a_printed_strength_bonus_adds_to_hand_strike_damage() {
    let mut e = bare();
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let massimiliano = add(&mut e, 1, "Massimiliano");
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(
        &mut e,
        Choice::DeclareBlock {
            minion: massimiliano,
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let blood = |m: MinionId| e.state.minions[m.index()].blood;
    assert_eq!((blood(a), blood(massimiliano)), (4 - 2, 4 - 1));
}

// Kevin Jackson: +1 strength in combat with a Brujah, and the Brujah gets
// +1 strength in combat with him — no one else does.
#[test]
fn kevin_jackson_and_a_brujah_opposing_him_each_strike_one_harder() {
    let mut e = bare();
    let brujah = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let other_brujah = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let kevin = add(&mut e, 1, "Kevin Jackson");
    let ventrue = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let strength = |e: &Engine, m, o| e.state.printed_mod(m, Stat::Strength, in_combat_with(o));
    assert_eq!(strength(&e, kevin, ventrue), 0);
    assert_eq!(strength(&e, other_brujah, ventrue), 0);
    pick(&mut e, action(brujah, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: kevin });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let blood = |m: MinionId| e.state.minions[m.index()].blood;
    assert_eq!((blood(brujah), blood(kevin)), (4 - 2, 4 - 2));
}

// Mickey Wheeler (capacity 5): +1 strength against an older vampire only.
#[test]
fn mickey_wheeler_strikes_one_harder_against_an_older_vampire() {
    let mut e = bare();
    let mickey = add(&mut e, 0, "Mickey Wheeler");
    let older = add(&mut e, 1, "Kevin Jackson"); // 7
    let younger = add(&mut e, 1, "Braden White"); // 4
    let strength = |m| {
        e.state
            .printed_mod(mickey, Stat::Strength, in_combat_with(m))
    };
    assert_eq!((strength(older), strength(younger)), (1, 0));
}

// Weirich Waldburg: while he is ready, primogen his Methuselah controls
// get +1 stealth — not he, not other vampires, not another's primogen.
#[test]
fn weirich_waldburg_gives_his_primogen_stealth_while_ready() {
    let mut e = bare();
    let weirich = add(&mut e, 0, "Weirich Waldburg");
    let lenny = add(&mut e, 0, "Lenny Burkhead"); // primogen
    let plain_one = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let rival = add(&mut e, 1, "Rinaldo Albizzi"); // primogen
    let hunt = during(ActionKind::Hunt, false, None);
    let stealth = |e: &Engine, m| e.state.printed_mod(m, Stat::Stealth, hunt);
    assert_eq!(
        [lenny, weirich, plain_one, rival].map(|m| stealth(&e, m)),
        [1, 0, 0, 0]
    );
    e.state.minions[weirich.index()].in_torpor = true;
    assert_eq!(stealth(&e, lenny), 0);
}

// "While X is ready, you get +n hand size", lost in torpor or contest.
#[test]
fn printed_hand_size_holds_while_the_vampire_is_ready() {
    let mut e = bare();
    let andre = add(&mut e, 0, "André the Manipulator");
    let carmelita = add(&mut e, 0, "Carmelita Neillson");
    add(&mut e, 1, "André the Manipulator");
    assert_eq!(e.state.hand_size(SeatId(0)), 7 + 2 + 1);
    e.state.minions[andre.index()].in_torpor = true;
    assert_eq!(e.state.hand_size(SeatId(0)), 7 + 1);
    e.state.minions[carmelita.index()].contested = true;
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
}

// Neserian: -1 bleed while her Methuselah controls no location, +1 vote
// while they control one; a contested location is not in play.
#[test]
fn neserian_bleeds_less_without_a_location_and_votes_with_one() {
    let mut e = bare();
    let neserian = add(&mut e, 0, "Neserian");
    let bleed = during(ActionKind::Bleed, true, None);
    let read = |e: &Engine| {
        (
            e.state.printed_mod(neserian, Stat::Bleed, bleed),
            e.state
                .printed_mod(neserian, Stat::Votes, PrintedCtx::default()),
        )
    };
    assert_eq!(read(&e), (-1, 0));
    in_play(&mut e, "Elder Library", 1, None);
    assert_eq!(read(&e), (-1, 0));
    let library = in_play(&mut e, "Elder Library", 0, None);
    assert_eq!(read(&e), (0, 1));
    e.state.cards[library].contested = true;
    assert_eq!(read(&e), (-1, 0));
}

// A passive printed ability applies while its vampire is in torpor (LSJ
// 20011022, "Ability active in torpor"), not while it is contested: Rose
// Abawi's "+1 bleed while you control Mithras".
#[test]
fn printed_text_stays_in_torpor_and_is_lost_while_contested() {
    let mut e = bare();
    let rose = add(&mut e, 0, "Rose Abawi");
    let bleed = during(ActionKind::Bleed, true, None);
    assert_eq!(e.state.printed_mod(rose, Stat::Bleed, bleed), 0);
    let mithras = add(&mut e, 0, "Mithras");
    e.state.minions[mithras.index()].in_torpor = true;
    assert_eq!(e.state.printed_mod(rose, Stat::Bleed, bleed), 1);
    e.state.minions[mithras.index()].contested = true;
    assert_eq!(e.state.printed_mod(rose, Stat::Bleed, bleed), 0);
    let catalina = add(&mut e, 0, "Catalina Vega");
    e.state.minions[catalina.index()].contested = true;
    assert_eq!(e.state.printed_mod(catalina, Stat::Bleed, bleed), 0);
}

// Rules-qa 13.1: a vampire may land with part of its text, the rest quoted
// in `incomplete` — every vampire now has all of it; the plain stat halves
// of two-part vampires apply.
#[test]
fn plain_stat_halves_apply_and_every_vampire_has_all_its_text() {
    let mut e = bare();
    let mithras = add(&mut e, 0, "Mithras");
    let oskar = add(&mut e, 0, "Oskar Anasov");
    let roy = add(&mut e, 0, "Roy");
    let ragnar = add(&mut e, 0, "Ragnar Nordstrom"); // capacity 8
    let alexa = add(&mut e, 1, "Alexa Draper"); // prince, capacity 8
    let braden = add(&mut e, 1, "Braden White"); // untitled, capacity 4
    let bleed = during(ActionKind::Bleed, true, None);
    let st = &e.state;
    assert_eq!(st.printed_mod(mithras, Stat::Bleed, bleed), 2);
    assert_eq!(
        st.printed_mod(mithras, Stat::Strength, in_combat_with(braden)),
        1
    );
    assert_eq!(
        st.printed_mod(mithras, Stat::Votes, PrintedCtx::default()),
        2
    );
    assert_eq!(st.printed_mod(oskar, Stat::Stealth, bleed), 1);
    // Roy: +1 strength in combat with a titled vampire; Ragnar: with a
    // younger one.
    let strength = |m, o| st.printed_mod(m, Stat::Strength, in_combat_with(o));
    assert_eq!([alexa, braden].map(|o| strength(roy, o)), [1, 0]);
    assert_eq!([alexa, braden].map(|o| strength(ragnar, o)), [0, 1]);
    assert!(VAMPIRES.iter().all(|d| d.incomplete.is_none()));
}

// ---- Slice 9b: printed combat abilities --------------------------------

fn vampire_ability(m: MinionId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: None,
        target: Target::None,
    }
}

fn card_ability(card: usize, ability: u8) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(CardId(card as u32)),
        ability,
        actor: None,
        target: Target::None,
    }
}

/// As `until`, counting the decisions on the way that offered any of
/// `watched`.
fn until_counting(e: &mut Engine, seat: SeatId, window: WindowLabel, watched: &[Choice]) -> usize {
    let mut offered = 0;
    loop {
        let req = request(e);
        if (req.seat, req.window) == (seat, window) {
            return offered;
        }
        if req.options.iter().any(|o| watched.contains(o)) {
            offered += 1;
        }
        let i = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::Pass | Choice::HandStrike))
            .unwrap_or(0);
        e.choose(i);
    }
}

/// Seat 0: a plain Ventrue `a` about to bleed (and a second vampire, so the
/// turn goes on); seat 1: `name`, the blocker.
fn blocked_by(name: &str) -> (Engine, MinionId, MinionId) {
    let mut e = bare();
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let blocker = add(&mut e, 1, name);
    (e, a, blocker)
}

// Abaddon: "1 optional maneuver and 1 optional press each combat" — one use
// of each over the whole combat. He maneuvers to long range (the hand
// strikes miss) and presses to continue; the second round is at close
// range, neither offered again.
#[test]
fn abaddon_maneuvers_and_presses_once_each_combat() {
    let (mut e, a, abaddon) = blocked_by("Abaddon");
    let (maneuver, press) = (vampire_ability(abaddon, 0), vampire_ability(abaddon, 1));
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: abaddon });
    until(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    pick(&mut e, maneuver);
    until(&mut e, SeatId(1), WindowLabel::CombatPress);
    pick(&mut e, press);
    let again = until_counting(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        &[maneuver, press],
    );
    assert_eq!(again, 0);
    assert_eq!((blood(&e, a), blood(&e, abaddon)), (4 - 1, 4 - 1));
}

// Flávio Gonçalves: "Once each combat, Flávio can strike: dodge." Abaddon
// bleeds and presses to continue; the dodge is not offered in the second
// round.
#[test]
fn flavio_goncalves_dodges_once_each_combat() {
    let mut e = bare();
    let abaddon = add(&mut e, 0, "Abaddon");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let flavio = add(&mut e, 1, "Flávio Gonçalves");
    let dodge = vampire_ability(flavio, 0);
    pick(&mut e, action(abaddon, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: flavio });
    until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    pick(&mut e, dodge);
    until(&mut e, SeatId(0), WindowLabel::CombatPress);
    pick(&mut e, vampire_ability(abaddon, 1));
    let again = until_counting(&mut e, SeatId(0), WindowLabel::MinionPhase, &[dodge]);
    assert_eq!(again, 0);
    // Only the second round's hand strikes landed.
    assert_eq!((blood(&e, abaddon), blood(&e, flavio)), (4 - 1, 4 - 1));
}

// Phaibun: "Once each combat, Phaibun can discard a card at random to
// strike: dodge." — the discarded card is replaced; with an empty hand the
// dodge is not offered.
#[test]
fn phaibun_discards_a_card_at_random_to_dodge() {
    let (mut e, a, phaibun) = blocked_by("Phaibun");
    hand(&mut e, 1, &["Elder Library", "Elder Library"]);
    e.state.seats[1].library.push(find_library("Elder Library"));
    let dodge = vampire_ability(phaibun, 0);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: phaibun });
    until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    pick(&mut e, dodge);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let seat = &e.state.seats[1];
    assert_eq!(
        (seat.ash.len(), seat.hand.len(), seat.library.len()),
        (1, 2, 0)
    );
    assert_eq!((blood(&e, a), blood(&e, phaibun)), (4, 4));

    let (mut e, a, phaibun) = blocked_by("Phaibun");
    let dodge = vampire_ability(phaibun, 0);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: phaibun });
    assert_eq!(
        until_counting(&mut e, SeatId(0), WindowLabel::MinionPhase, &[dodge]),
        0
    );
}

// Opikun, in torpor, burns 1 blood to prevent up to 2 non-aggravated damage
// to the vampire his Methuselah blocks with.
// Rulings: [LSJ 20011022].
#[test]
fn opikun_in_torpor_prevents_damage_to_a_vampire_in_combat() {
    let mut e = bare();
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Nosferatu", 0), 4, false);
    let opikun = e.add_minion(SeatId(1), find_card("Opikun"), 2, true);
    let prevent = vampire_ability(opikun, 0);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: x });
    until(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    pick(&mut e, prevent);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        (blood(&e, a), blood(&e, x), blood(&e, opikun)),
        (4 - 1, 4, 1)
    );
}

// Printed prevention of non-aggravated damage (Rowena, Opikun) is not
// offered against aggravated damage: an Ivory Bow's aggravated strike
// sends Rowena to torpor.
// Rulings: [LSJ 20040812-2] [PIB 20130327].
#[test]
fn printed_non_aggravated_prevention_leaves_aggravated_damage() {
    let (mut e, a, rowena) = blocked_by("Rowena");
    let bow = in_play(&mut e, "Ivory Bow", 0, Some(a));
    let opikun = e.add_minion(SeatId(1), find_card("Opikun"), 2, true);
    let watched = [vampire_ability(rowena, 0), vampire_ability(opikun, 0)];
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: rowena });
    until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    pick(&mut e, card_ability(bow, 0));
    assert_eq!(
        until_counting(&mut e, SeatId(0), WindowLabel::MinionPhase, &watched),
        0
    );
    assert!(e.state.minions[rowena.index()].in_torpor);
    assert_eq!(blood(&e, opikun), 2);
}

// Rowena: "can prevent 1 non-aggravated damage each round".
#[test]
fn rowena_prevents_one_non_aggravated_damage() {
    let (mut e, a, rowena) = blocked_by("Rowena");
    let prevent = vampire_ability(rowena, 0);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: rowena });
    until(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    pick(&mut e, prevent);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, rowena)), (4 - 1, 4));
}

// Chun Hei: "Once each combat, Chun Hei can burn 1 blood to get 1
// additional strike that does not count against the limit. +1 strength."
#[test]
fn chun_hei_burns_blood_for_an_additional_strike() {
    let (mut e, a, chun) = blocked_by("Chun Hei");
    let strike = vampire_ability(chun, 0);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: chun });
    // Once the first pair is over (rules-qa 47.2).
    until(&mut e, SeatId(1), WindowLabel::CombatAdditionalStrikes);
    pick(&mut e, strike);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Two hand strikes of strength 2 on A; one of 1 on Chun Hei.
    assert_eq!((blood(&e, a), blood(&e, chun)), (0, 4 - 1 - 1));
}

// Kasim Bayar: "Once each combat, Kasim can discard a political action card
// before range is determined to get +2 strength that combat." — the only
// political action card in hand is the pick.
#[test]
fn kasim_bayar_discards_a_political_action_for_two_strength() {
    let (mut e, a, kasim) = blocked_by("Kasim Bayar");
    hand(&mut e, 1, &["Elder Library", "Kine Resources Contested"]);
    let ability = vampire_ability(kasim, 0);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: kasim });
    until(&mut e, SeatId(1), WindowLabel::CombatBeforeRange);
    pick(&mut e, ability);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, kasim)), (4 - 3, 4 - 1));
    assert_eq!(
        e.state.seats[1].ash,
        vec![find_library("Kine Resources Contested")]
    );
}

// Noluthando: "+1 damage with ranged strikes (even at close range)": his
// .44 Magnum does 3 at close range.
#[test]
fn noluthando_adds_one_damage_to_his_ranged_strikes() {
    let (mut e, a, noluthando) = blocked_by("Noluthando");
    let gun = in_play(&mut e, ".44 Magnum", 1, Some(noluthando));
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: noluthando });
    until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    pick(&mut e, card_ability(gun, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, noluthando)), (4 - 3, 4 - 1));
}

fn polls(req: &DecisionRequest) -> bool {
    matches!(
        req.window,
        WindowLabel::ReferendumPolling | WindowLabel::BloodHuntPolling
    )
}

// Alexa Draper's referendum ability: usable while she is locked (she called
// the referendum), once per referendum — a second Dominate card in hand buys
// no second vote; her +1 vote carries Ancilla Empowerment 3 to 2.
// Rulings: [ANK 20220705].
#[test]
fn alexa_draper_discards_a_dominate_card_for_a_vote_once_per_referendum() {
    let mut e = bare();
    let alexa = add(&mut e, 0, "Alexa Draper");
    add(&mut e, 0, "Mateo García");
    let kevin = add(&mut e, 1, "Kevin Jackson");
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    hand(
        &mut e,
        0,
        &["Ancilla Empowerment", "Conditioning", "Bonding"],
    );
    pick(&mut e, play(0, 0, alexa));
    until(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    assert!(e.state.minions[alexa.index()].locked);
    pick(&mut e, vampire_ability(alexa, 0));
    pick(&mut e, Choice::Discard(0));
    assert_eq!(e.state.seats[0].ash, vec![find_library("Conditioning")]);
    let (mut alexa_voted, mut kevin_voted) = (false, false);
    loop {
        let req = request(&mut e);
        if !polls(&req) {
            break;
        }
        assert!(!req.options.contains(&vampire_ability(alexa, 0)));
        let choice = if req.seat == SeatId(0) && !alexa_voted {
            alexa_voted = true;
            Choice::CastVotes {
                minion: alexa,
                favor: true,
            }
        } else if req.seat == SeatId(1) && !kevin_voted {
            kevin_voted = true;
            Choice::CastVotes {
                minion: kevin,
                favor: false,
            }
        } else {
            Choice::Pass
        };
        pick(&mut e, choice);
    }
    // 3 for, 2 against: each Methuselah burns 1 pool per minion.
    assert_eq!([e.state.seats[0].pool, e.state.seats[1].pool], [8, 9]);
}

// Yewon Ong's referendum ability is usable in torpor, once per referendum; a
// vampire in torpor casts no votes all the same.
// Rulings: [LSJ 20011022] [ANK 20220705].
#[test]
fn yewon_ong_uses_her_referendum_ability_in_torpor() {
    let mut e = bare();
    let alexa = add(&mut e, 0, "Alexa Draper");
    add(&mut e, 0, "Mateo García");
    let yewon = e.add_minion(SeatId(1), find_card("Yewon Ong"), 2, true);
    add(&mut e, 1, "Kevin Jackson");
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    hand(&mut e, 1, &["Arms of Ahriman", "Consign to Oblivion"]);
    pick(&mut e, play(0, 0, alexa));
    until(&mut e, SeatId(1), WindowLabel::ReferendumPolling);
    pick(&mut e, vampire_ability(yewon, 0));
    pick(&mut e, Choice::Discard(1));
    assert_eq!(
        e.state.seats[1].ash,
        vec![find_library("Consign to Oblivion")]
    );
    loop {
        let req = request(&mut e);
        if !polls(&req) {
            break;
        }
        assert!(!req.options.contains(&vampire_ability(yewon, 0)));
        assert!(!req
            .options
            .iter()
            .any(|o| matches!(o, Choice::CastVotes { minion, .. } if *minion == yewon)));
        pick(&mut e, Choice::Pass);
    }
}

/// Seat 0's Amelia Locke diablerizes seat 1's Arjun Shah. In the blood hunt,
/// Alice Chen (2 votes) and, with `edge`, the Edge vote for; Aisha az-Zahra
/// uses her ability whenever offered and casts against. Seat 1 also controls
/// a ready Anarch (Anita Wainwright) and one in torpor (Joseph Fischer).
/// Returns whether the diablerist is still in play, and Aisha's blood.
fn aisha_blood_hunt(edge: bool) -> (bool, u8) {
    let mut e = bare();
    let amelia = add(&mut e, 0, "Amelia Locke");
    let alice = add(&mut e, 0, "Alice Chen");
    let arjun = e.add_minion(SeatId(1), find_card("Arjun Shah"), 2, true);
    let aisha = add(&mut e, 1, "Aisha az-Zahra");
    add(&mut e, 1, "Anita Wainwright");
    e.add_minion(SeatId(1), find_card("Joseph Fischer"), 2, true);
    e.state.seats[0].has_edge = edge;
    pick(
        &mut e,
        Choice::TakeAction {
            minion: amelia,
            action: ActionKind::Diablerie,
            target: Some(arjun),
        },
    );
    let against = Choice::CastVotes {
        minion: aisha,
        favor: false,
    };
    let (mut alice_voted, mut edge_cast, mut aisha_voted) = (false, false, false);
    loop {
        let req = request(&mut e);
        match req.window {
            WindowLabel::BloodHuntPolling => {}
            WindowLabel::ActionBlock => {
                pick(&mut e, Choice::Pass);
                continue;
            }
            _ => break,
        }
        let choice = if req.seat == SeatId(0) && !alice_voted {
            alice_voted = true;
            Choice::CastVotes {
                minion: alice,
                favor: true,
            }
        } else if req.seat == SeatId(0) && edge && !edge_cast {
            edge_cast = true;
            Choice::CastEdge { favor: true }
        } else if req.options.contains(&vampire_ability(aisha, 0)) {
            vampire_ability(aisha, 0)
        } else if req.options.contains(&against) && !aisha_voted {
            aisha_voted = true;
            against
        } else {
            Choice::Pass
        };
        pick(&mut e, choice);
    }
    assert!(aisha_voted);
    (e.state.minions[amelia.index()].in_play, blood(&e, aisha))
}

// Aisha az-Zahra in a blood hunt ("any referendum"): +1 vote for each ready
// Anarch her controller controls — herself and Anita, not Joseph in torpor —
// once per referendum (1 blood). Her 2 against tie Alice Chen's 2 for, and
// the hunt fails; with the Edge also for, 3 for carry it.
// Rulings: [ANK 20220705].
#[test]
fn aisha_az_zahra_gets_a_vote_per_ready_anarch_in_a_blood_hunt() {
    assert_eq!(aisha_blood_hunt(false), (true, 3));
    assert_eq!(aisha_blood_hunt(true), (false, 3));
}

// Queen Anne, in the polling step of the referendum she called, burns 1
// blood to force a younger Camarilla vampire to abstain — any Methuselah's,
// in torpor too, not an Anarch — once per referendum; the abstaining prince
// casts no votes.
#[test]
fn queen_anne_forces_a_younger_camarilla_vampire_to_abstain() {
    let mut e = bare();
    let anne = add(&mut e, 0, "Queen Anne");
    let mateo = add(&mut e, 0, "Mateo García");
    let kevin = add(&mut e, 1, "Kevin Jackson");
    add(&mut e, 1, "Anita Wainwright");
    let arjun = e.add_minion(SeatId(1), find_card("Arjun Shah"), 2, true);
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    pick(&mut e, play(0, 0, anne));
    let req = until(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    let force = |t| Choice::UseAbility {
        source: AbilitySource::Vampire(anne),
        ability: 0,
        actor: None,
        target: Target::Minion(t),
    };
    let offered: Vec<Choice> = req
        .options
        .iter()
        .copied()
        .filter(|o| matches!(o, Choice::UseAbility { .. }))
        .collect();
    assert_eq!(offered, vec![force(mateo), force(kevin), force(arjun)]);
    pick(&mut e, force(kevin));
    assert_eq!(blood(&e, anne), 3);
    loop {
        let req = request(&mut e);
        if !polls(&req) {
            break;
        }
        assert!(!req
            .options
            .iter()
            .any(|o| matches!(o, Choice::UseAbility { .. })));
        assert!(!req
            .options
            .iter()
            .any(|o| matches!(o, Choice::CastVotes { minion, .. } if *minion == kevin)));
        pick(&mut e, Choice::Pass);
    }
}

// ---- Slice 9c: printed abilities during actions ------------------------

// Oskar Anasov, in torpor, burns 1 blood to give his controller's vampire
// attempting to block +1 intercept: his blood makes the block reachable,
// and once intercept is no longer needed he is not offered it again.
// Rulings: [LSJ 20011022].
#[test]
fn oskar_anasov_in_torpor_gives_intercept_to_a_blocker() {
    let mut e = bare();
    let hunter = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let blocker = e.add_minion(SeatId(1), plain("Nosferatu", 0), 4, false);
    let oskar = e.add_minion(SeatId(1), find_card("Oskar Anasov"), 2, true);
    pick(&mut e, action(hunter, ActionKind::Hunt));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req
        .options
        .contains(&Choice::DeclareBlock { minion: blocker }));
    pick(&mut e, Choice::DeclareBlock { minion: blocker });
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, vampire_ability(oskar, 0));
    assert_eq!(blood(&e, oskar), 1);
    let again = until_counting(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        &[vampire_ability(oskar, 0)],
    );
    assert_eq!(again, 0);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
}

// Gwenllian Arwyn, during the action she performs to leave torpor, burns 1
// blood to give the vampire attempting to block -1 intercept: Rinaldo
// Albizzi's +1 no longer meets the action's stealth, the attempt fails, and
// the -1 stays for the rest of the action — no second attempt is offered.
// Rulings: [LSJ 20011022].
#[test]
fn gwenllian_arwyn_lowers_a_blockers_intercept_as_she_leaves_torpor() {
    let mut e = bare();
    let gwen = e.add_minion(SeatId(0), find_card("Gwenllian Arwyn"), 4, true);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let rinaldo = add(&mut e, 1, "Rinaldo Albizzi");
    pick(&mut e, action(gwen, ActionKind::LeaveTorpor));
    pick(&mut e, Choice::DeclareBlock { minion: rinaldo });
    until(&mut e, SeatId(0), WindowLabel::ActionBlock);
    pick(&mut e, vampire_ability(gwen, 0));
    assert_eq!(blood(&e, gwen), 3);
    let watched = [
        vampire_ability(gwen, 0),
        Choice::DeclareBlock { minion: rinaldo },
    ];
    let again = until_counting(&mut e, SeatId(0), WindowLabel::MinionPhase, &watched);
    assert_eq!(again, 0);
    assert!(!e.state.minions[gwen.index()].in_torpor);
}

// Larissa Moreira discards a card requiring Animalism for +1 bleed during her
// bleed before resolution, once per action (rules-qa 15.2) — not during a
// hunt.
// Rulings: [ANK 20231124] [LSJ 19970718].
#[test]
fn larissa_moreira_discards_animalism_cards_for_bleed_during_a_bleed() {
    let setup = || {
        let mut e = bare();
        let larissa = add(&mut e, 0, "Larissa Moreira");
        e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        hand(&mut e, 0, &["Aid from Bats", "Army of Rats"]);
        (e, larissa)
    };
    let (mut e, larissa) = setup();
    let before = e.state.seats[1].pool;
    pick(&mut e, action(larissa, ActionKind::Bleed));
    until(&mut e, SeatId(0), WindowLabel::ActionBlock);
    pick(&mut e, vampire_ability(larissa, 0));
    pick(&mut e, Choice::Discard(0));
    let again = until_counting(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        &[vampire_ability(larissa, 0)],
    );
    assert_eq!(again, 0);
    assert_eq!(e.state.seats[0].ash.len(), 1);
    assert_eq!(e.state.seats[1].pool, before - 2);

    let (mut e, larissa) = setup();
    pick(&mut e, action(larissa, ActionKind::Hunt));
    let offered = until_counting(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        &[vampire_ability(larissa, 0)],
    );
    assert_eq!(offered, 0);
}

// Abraham DuSable discards cards requiring Blood Sorcery: +1 stealth on his
// bleed while stealth is needed — once per action (rules-qa 15.2), so
// Rinaldo Albizzi's +1 intercept still blocks — and +1 intercept on his own
// block attempt: the card in hand makes the block reachable.
#[test]
fn abraham_dusable_discards_blood_sorcery_cards_for_stealth_or_intercept() {
    let mut e = bare();
    let abraham = add(&mut e, 0, "Abraham DuSable");
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let rinaldo = add(&mut e, 1, "Rinaldo Albizzi");
    hand(&mut e, 0, &["Apportation", "Blood Fury", "Blood Rage"]);
    pick(&mut e, action(abraham, ActionKind::Bleed));
    // No block attempt yet: stealth is not needed.
    let offered = until_counting(
        &mut e,
        SeatId(1),
        WindowLabel::ActionBlock,
        &[vampire_ability(abraham, 0)],
    );
    assert_eq!(offered, 0);
    pick(&mut e, Choice::DeclareBlock { minion: rinaldo });
    until(&mut e, SeatId(0), WindowLabel::ActionBlock);
    pick(&mut e, vampire_ability(abraham, 0));
    pick(&mut e, Choice::Discard(0));
    let again = until_counting(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        &[vampire_ability(abraham, 0)],
    );
    assert_eq!(again, 0);
    assert_eq!(e.state.seats[0].ash.len(), 1);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));

    let mut e = bare();
    let hunter = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let abraham = add(&mut e, 1, "Abraham DuSable");
    hand(&mut e, 1, &["Apportation"]);
    pick(&mut e, action(hunter, ActionKind::Hunt));
    pick(&mut e, Choice::DeclareBlock { minion: abraham });
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, vampire_ability(abraham, 1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
}

// Daria, in torpor, locks to give the acting Nosferatu +1 stealth while it
// is needed; locked, she is not offered it again. A blocker with no
// intercept then no longer meets the bleed's stealth.
// Rulings: [LSJ 20011022].
#[test]
fn daria_in_torpor_locks_to_give_an_acting_nosferatu_stealth() {
    let mut e = bare();
    let nosferatu = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    let daria = e.add_minion(SeatId(0), find_card("Daria"), 2, true);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let blocker = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    pick(&mut e, action(nosferatu, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: blocker });
    until(&mut e, SeatId(0), WindowLabel::ActionBlock);
    pick(&mut e, vampire_ability(daria, 0));
    assert!(e.state.minions[daria.index()].locked);
    let again = until_counting(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        &[vampire_ability(daria, 0)],
    );
    assert_eq!(again, 0);
    assert_ne!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
}

// ---- Slice 9c: printed abilities in phases ------------------------------

fn use_on(m: MinionId, ability: u8, target: MinionId) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: None,
        target: Target::Minion(target),
    }
}

// Nonu Dis, ready in his controller's master phase, adds 1 blood to a ready
// Follower of Set (Ministry) they control once a master card has been played
// — himself included, not a vampire of another clan — once per master
// phase however many master cards are played.
// Rulings: [ANK 20220617].
#[test]
fn nonu_dis_adds_blood_once_per_master_phase_after_a_master_card() {
    let mut e = bare();
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let nonu = add(&mut e, 1, "Nonu Dis");
    let marcel = e.add_minion(SeatId(1), find_card("Marcel"), 2, false);
    let brujah = e.add_minion(SeatId(1), plain("Brujah", 0), 2, false);
    hand(&mut e, 1, &["Dreams of the Sphinx", "Dreams of the Sphinx"]);
    let req = until(&mut e, SeatId(1), WindowLabel::MasterPhase);
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::UseAbility { .. })));
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::None,
        },
    );
    let req = until(&mut e, SeatId(1), WindowLabel::MasterPhase);
    // (The Dreams of the Sphinx now in play offers its own lock.)
    let uses: Vec<Choice> = req
        .options
        .iter()
        .copied()
        .filter(|o| {
            matches!(
                o,
                Choice::UseAbility {
                    source: AbilitySource::Vampire(_),
                    ..
                }
            )
        })
        .collect();
    assert_eq!(uses, vec![use_on(nonu, 0, nonu), use_on(nonu, 0, marcel)]);
    pick(&mut e, use_on(nonu, 0, marcel));
    assert_eq!(blood(&e, marcel), 3);
    let watched = [
        use_on(nonu, 0, nonu),
        use_on(nonu, 0, marcel),
        use_on(nonu, 0, brujah),
    ];
    let again = until_counting(&mut e, SeatId(1), WindowLabel::MinionPhase, &watched);
    assert_eq!(again, 0);
}

// Mora, the Death Seer, ready in her controller's discard phase, moves a
// library card from the ash heap to the bottom of the library (one choice
// per card name), once per discard phase.
#[test]
fn mora_moves_an_ash_heap_card_to_the_library_bottom_once_per_discard_phase() {
    let mut e = bare();
    let mora = add(&mut e, 0, "Mora, the Death Seer");
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let (bonding, conditioning, villein) = (
        find_library("Bonding"),
        find_library("Conditioning"),
        find_library("Villein"),
    );
    e.state.seats[0].ash = vec![bonding, bonding, conditioning];
    e.state.seats[0].library = vec![villein];
    fill_hand(&mut e, 0);
    until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    pick(&mut e, vampire_ability(mora, 0));
    let req = request(&mut e);
    let mut names = vec![Choice::FindCard(bonding), Choice::FindCard(conditioning)];
    names.sort_by_key(|c| match c {
        Choice::FindCard(n) => *n,
        _ => 0,
    });
    assert_eq!(req.options, names);
    pick(&mut e, Choice::FindCard(conditioning));
    assert_eq!(e.state.seats[0].library, vec![conditioning, villein]);
    assert_eq!(e.state.seats[0].ash, vec![bonding, bonding]);
    let again = until_counting(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        &[vampire_ability(mora, 0)],
    );
    assert_eq!(again, 0);
}

// Sreelekha, ready as her controller's discard phase begins, gives a second
// discard phase action; Ayse Dhanial spends each on 1 blood. With Sreelekha
// in torpor, one use only.
#[test]
fn sreelekha_gives_a_discard_phase_action_that_ayse_dhanial_spends_on_blood() {
    for (torpor, gained) in [(false, 2), (true, 1)] {
        let mut e = bare();
        e.add_minion(SeatId(0), find_card("Sreelekha"), 4, torpor);
        let ayse = e.add_minion(SeatId(0), find_card("Ayse Dhanial"), 1, false);
        e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        // Use Ayse's ability whenever offered, pass otherwise.
        loop {
            let req = request(&mut e);
            if req.seat == SeatId(1) && req.window == WindowLabel::MinionPhase {
                break;
            }
            if req.options.contains(&vampire_ability(ayse, 0)) {
                pick(&mut e, vampire_ability(ayse, 0));
            } else {
                pick(&mut e, Choice::Pass);
            }
        }
        assert_eq!(blood(&e, ayse), 1 + gained, "Sreelekha in torpor: {torpor}");
    }
}

// Kalinda: during her controller's turn, burn the Edge to unlock her, once
// that turn (rules-qa 15.2) — not on another Methuselah's turn.
#[test]
fn kalinda_burns_the_edge_to_unlock_on_her_turn() {
    let mut e = bare();
    let kalinda = e.add_minion(SeatId(0), find_card("Kalinda"), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.state.minions[kalinda.index()].locked = true;
    e.state.seats[0].has_edge = true;
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::MinionPhase);
    pick(&mut e, vampire_ability(kalinda, 0));
    assert!(!e.state.minions[kalinda.index()].locked);
    assert!(!e.state.seats[0].has_edge);
    e.state.minions[kalinda.index()].locked = true;
    e.state.seats[0].has_edge = true;
    let req = request(&mut e);
    assert!(!req.options.contains(&vampire_ability(kalinda, 0)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[kalinda.index()].locked = true;
    e.state.seats[0].has_edge = true;
    pick(&mut e, action(x, ActionKind::Bleed));
    let req = until(&mut e, SeatId(0), WindowLabel::ActionBlock);
    assert!(!req.options.contains(&vampire_ability(kalinda, 0)));
}

// ---- Slice 9c-ii: after an action --------------------------------------

/// Pass through the polling step, `votes` cast by their controllers first,
/// and the political action card's vote if `card` names a side.
fn poll(e: &mut Engine, votes: &[(MinionId, bool)], card: Option<bool>) {
    let mut cast = vec![false; votes.len()];
    let mut card = card;
    loop {
        let req = request(e);
        if !polls(&req) {
            return;
        }
        if let Some(favor) = card {
            if req.options.contains(&Choice::CastCardVote { favor }) {
                card = None;
                pick(e, Choice::CastCardVote { favor });
                continue;
            }
        }
        let vote = votes
            .iter()
            .enumerate()
            .find(|(i, (m, _))| !cast[*i] && e.state.minions[m.index()].owner == req.seat)
            .map(|(i, v)| (i, *v));
        let choice = match vote {
            Some((i, (minion, favor))) => {
                cast[i] = true;
                Choice::CastVotes { minion, favor }
            }
            None => Choice::Pass,
        };
        pick(e, choice);
    }
}

// Sybren van Oosten can unlock after the referendum his political action
// called passes — not after it fails.
#[test]
fn sybren_van_oosten_unlocks_after_his_referendum_passes() {
    for passes in [true, false] {
        let mut e = bare();
        let sybren = add(&mut e, 0, "Sybren van Oosten");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let kevin = add(&mut e, 1, "Kevin Jackson");
        for s in &mut e.state.seats {
            s.pool = 10;
        }
        hand(&mut e, 0, &["Ancilla Empowerment"]);
        pick(&mut e, play(0, 0, sybren));
        until(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
        let mut votes = vec![(kevin, false)];
        if passes {
            votes.push((sybren, true));
        }
        poll(&mut e, &votes, Some(true));
        if passes {
            until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
            pick(&mut e, vampire_ability(sybren, 0));
            assert!(!e.state.minions[sybren.index()].locked);
        } else {
            let offered = until_counting(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                &[vampire_ability(sybren, 0)],
            );
            assert_eq!(offered, 0);
            assert!(e.state.minions[sybren.index()].locked);
        }
    }
}

// Queen Anne can unlock after her diablerie action (the blood hunt fails).
#[test]
fn queen_anne_unlocks_after_her_diablerie() {
    let mut e = bare();
    let anne = add(&mut e, 0, "Queen Anne");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let victim = e.add_minion(SeatId(1), plain("Brujah", 1), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: anne,
            action: ActionKind::Diablerie,
            target: Some(victim),
        },
    );
    let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    assert!(!req.options.contains(&vampire_ability(anne, 0)));
    pick(&mut e, vampire_ability(anne, 1));
    assert!(!e.state.minions[anne.index()].locked);
    assert!(!e.state.minions[victim.index()].in_play);
}

// Aline Gädeke, in torpor, burns 1 blood to unlock after another Anarch her
// controller controls performs a successful action, once each turn; a
// fizzled leave-torpor action counts.
// Rulings: [LSJ 20011022] [ANK 20220218].
#[test]
fn aline_gadeke_unlocks_after_another_anarchs_successful_action() {
    let setup = || {
        let mut e = bare();
        let aline = e.add_minion(SeatId(0), find_card("Aline Gädeke"), 3, true);
        e.state.minions[aline.index()].locked = true;
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        (e, aline)
    };
    let (mut e, aline) = setup();
    let dario = add(&mut e, 0, "Dario Ziggler");
    let crow = add(&mut e, 0, "Crow");
    pick(&mut e, action(dario, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    pick(&mut e, vampire_ability(aline, 0));
    assert!(!e.state.minions[aline.index()].locked);
    assert_eq!(blood(&e, aline), 2);
    e.state.minions[aline.index()].locked = true;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, action(crow, ActionKind::Hunt));
    let again = until_counting(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        &[vampire_ability(aline, 0)],
    );
    assert_eq!(again, 0);

    let (mut e, aline) = setup();
    let dario = e.add_minion(SeatId(0), find_card("Dario Ziggler"), 2, true);
    pick(&mut e, action(dario, ActionKind::LeaveTorpor));
    // Blood lost during the action: the leave-torpor fizzles.
    e.state.minions[dario.index()].blood = 1;
    let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    assert!(e.state.minions[dario.index()].in_torpor);
    assert!(req.options.contains(&vampire_ability(aline, 0)));
}

// Alicia Cortez, after her successful bleed, discards a political action
// card to lock a younger vampire of the bled Methuselah — a locked one is
// offered too, for the cycling (rules-qa 16.4).
#[test]
fn alicia_cortez_locks_a_younger_vampire_of_the_bled_methuselah() {
    let mut e = bare();
    let alicia = add(&mut e, 0, "Alicia Cortez");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let madison = add(&mut e, 1, "Madison");
    let amelia = add(&mut e, 1, "Amelia Locke");
    e.state.minions[amelia.index()].locked = true;
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    pick(&mut e, action(alicia, ActionKind::Bleed));
    let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    assert!(req.options.contains(&use_on(alicia, 0, madison)));
    assert!(req.options.contains(&use_on(alicia, 0, amelia)));
    // (The one political action card in hand: the discard elides.)
    pick(&mut e, use_on(alicia, 0, madison));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.seats[0].ash,
        vec![find_library("Ancilla Empowerment")]
    );
    assert!(e.state.minions[madison.index()].locked);
}

// Ilonka, ready after a successful bleed against her controller, looks at
// the bleeder's controller's hand and discards 1 card at random from it;
// once each turn.
#[test]
fn ilonka_discards_at_random_from_the_bleeders_hand_once_each_turn() {
    let mut e = bare();
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ilonka = add(&mut e, 1, "Ilonka");
    hand(&mut e, 0, &["Blood Rage", "Army of Rats"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    until(&mut e, SeatId(1), WindowLabel::ActionAfterResolution);
    assert!(!e.state.hand_visible_to(SeatId(0), SeatId(1)));
    pick(&mut e, vampire_ability(ilonka, 0));
    let req = request(&mut e);
    assert_eq!(req.options, vec![Choice::SetX(0), Choice::SetX(1)]);
    // She looked at the hand before choosing (owner play feedback).
    assert!(e.state.hand_visible_to(SeatId(0), SeatId(1)));
    pick(&mut e, Choice::SetX(1));
    assert_eq!(e.state.seats[0].hand.len(), 1);
    assert_eq!(e.state.seats[0].ash.len(), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, action(b, ActionKind::Bleed));
    let again = until_counting(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        &[vampire_ability(ilonka, 0)],
    );
    assert_eq!(again, 0);
}

// Richard de Worde, after his successful bleed, looks at the bled
// Methuselah's hand and burns 1 blood to discard the card he chooses from
// it; once per action (rules-qa 16.1).
#[test]
fn richard_de_worde_discards_a_card_from_the_bled_methuselahs_hand() {
    let mut e = bare();
    let richard = add(&mut e, 0, "Richard de Worde");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Blood Rage", "Army of Rats"]);
    pick(&mut e, action(richard, ActionKind::Bleed));
    until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    pick(&mut e, vampire_ability(richard, 0));
    let req = request(&mut e);
    let mut shown = vec![find_library("Blood Rage"), find_library("Army of Rats")];
    shown.sort_unstable();
    assert_eq!(
        req.options,
        shown.into_iter().map(Choice::FindCard).collect::<Vec<_>>()
    );
    pick(&mut e, Choice::FindCard(find_library("Army of Rats")));
    assert_eq!(e.state.seats[1].hand, vec![find_library("Blood Rage")]);
    assert_eq!(e.state.seats[1].ash, vec![find_library("Army of Rats")]);
    assert_eq!(blood(&e, richard), 3);
    let again = until_counting(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        &[vampire_ability(richard, 0)],
    );
    assert_eq!(again, 0);
}

// ---- Slice 9c-ii: vampire riders and side-aware votes ------------------

// Marialena burns 1 blood as she is blocked, before block resolution: on a
// bleed (then the hand strikes), and in torpor on a leave-torpor action
// (before the blocker's diablerie choice).
// Rulings: [LSJ 20011022].
#[test]
fn marialena_burns_a_blood_when_blocked() {
    let mut e = bare();
    let marialena = add(&mut e, 0, "Marialena");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let blocker = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, action(marialena, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: blocker });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        (blood(&e, marialena), blood(&e, blocker)),
        (4 - 1 - 1, 4 - 1)
    );

    let mut e = bare();
    let marialena = e.add_minion(SeatId(0), find_card("Marialena"), 3, true);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    // +1 intercept reaches the leave-torpor action's stealth.
    let blocker = add(&mut e, 1, "Rinaldo Albizzi");
    pick(&mut e, action(marialena, ActionKind::LeaveTorpor));
    pick(&mut e, Choice::DeclareBlock { minion: blocker });
    until(&mut e, SeatId(1), WindowLabel::TorporBlock);
    assert_eq!(blood(&e, marialena), 2);
}

// Egidia Arrú blocks a bleeding vampire with 1 blood: her hand strike (+1
// strength) sends it to torpor, and its controller burns 1 pool.
#[test]
fn egidia_arru_makes_a_minion_leaving_the_ready_region_cost_a_pool() {
    let mut e = bare();
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let egidia = add(&mut e, 1, "Egidia Arrú");
    let before = e.state.seats[0].pool;
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: egidia });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].in_torpor);
    assert_eq!(e.state.seats[0].pool, before - 1);
}

// Roy, unlocked during his controller's discard phase, is locked as it ends
// — in torpor too.
// Rulings: [LSJ 20011022].
#[test]
fn roy_locks_himself_if_unlocked_during_the_discard_phase() {
    for torpor in [false, true] {
        let mut e = bare();
        let roy = e.add_minion(SeatId(0), find_card("Roy"), 1, torpor);
        e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(e.state.minions[roy.index()].locked, "in torpor: {torpor}");
    }
}

// Cedrick Calhoun goes to torpor after his political action if its
// referendum fails — not if it passes.
#[test]
fn cedrick_calhoun_goes_to_torpor_if_his_referendum_fails() {
    for passes in [true, false] {
        let mut e = bare();
        let cedrick = add(&mut e, 0, "Cedrick Calhoun");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let kevin = add(&mut e, 1, "Kevin Jackson");
        for s in &mut e.state.seats {
            s.pool = 10;
        }
        hand(&mut e, 0, &["Ancilla Empowerment"]);
        pick(&mut e, play(0, 0, cedrick));
        until(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
        let votes = if passes { vec![] } else { vec![(kevin, false)] };
        poll(&mut e, &votes, Some(true));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[cedrick.index()].in_torpor, !passes);
    }
}

// Jason "Son" Newberry casts his 1 vote and 2 more against a blood hunt: 3
// against Kevin Jackson's 2 in favor, the diablerist is not burned.
#[test]
fn jason_son_newberry_gets_two_votes_against_a_blood_hunt() {
    let mut e = bare();
    let diablerist = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let victim = e.add_minion(SeatId(1), plain("Brujah", 1), 2, true);
    let jason = add(&mut e, 1, "Jason \"Son\" Newberry");
    let kevin = add(&mut e, 1, "Kevin Jackson");
    pick(
        &mut e,
        Choice::TakeAction {
            minion: diablerist,
            action: ActionKind::Diablerie,
            target: Some(victim),
        },
    );
    // (Seat 0 has nothing to cast: its polling turn elides.)
    until(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
    poll(&mut e, &[(kevin, true), (jason, false)], None);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[diablerist.index()].in_play);
}

// Styles Margs: the blood hunt called on him gets +2 votes in favor — with
// no vote cast it passes and he is burned (Queen Anne's diablerie, with no
// vote, is not).
#[test]
fn styles_margs_blood_hunt_gets_two_votes_in_favor() {
    let mut e = bare();
    let styles = add(&mut e, 0, "Styles Margs");
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let victim = e.add_minion(SeatId(1), plain("Brujah", 1), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: styles,
            action: ActionKind::Diablerie,
            target: Some(victim),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[styles.index()].in_play);
}

// Queen Anne blocks a leave-torpor action (Oskar Anasov's blood buys the
// intercept), diablerises the acting vampire, and unlocks after the blood
// hunt (rules-qa 16.2).
#[test]
fn queen_anne_unlocks_after_a_diablerie_as_the_blocker() {
    let mut e = bare();
    let leaver = e.add_minion(SeatId(0), plain("Brujah", 0), 4, true);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let anne = add(&mut e, 1, "Queen Anne");
    let oskar = add(&mut e, 1, "Oskar Anasov");
    pick(&mut e, action(leaver, ActionKind::LeaveTorpor));
    pick(&mut e, Choice::DeclareBlock { minion: anne });
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, vampire_ability(oskar, 0));
    until(&mut e, SeatId(1), WindowLabel::TorporBlock);
    pick(&mut e, Choice::Diablerise);
    until(&mut e, SeatId(1), WindowLabel::ActionAfterResolution);
    pick(&mut e, vampire_ability(anne, 1));
    assert!(!e.state.minions[anne.index()].locked);
    assert!(!e.state.minions[leaver.index()].in_play);
}

// Gathii reveals the top of his library during his action, once per action:
// a master card burns 1 blood, another card gives +1 stealth — but only
// while stealth is needed (ANK 20231031), so the reveal before any block
// attempt leaves the bleed at stealth 0; usable in torpor, on his
// leave-torpor action.
// Rulings: [ANK 20231031]; [LSJ 20011022].
#[test]
fn gathii_reveals_the_top_card_for_blood_or_stealth() {
    let setup = |top: &str| {
        let mut e = bare();
        let gathii = add(&mut e, 0, "Gathii");
        e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        let blocker = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        e.state.seats[0].library.push(find_library(top));
        fill_hand(&mut e, 0);
        (e, gathii, blocker)
    };
    // A master card on top: 1 blood, whatever the block attempt.
    let (mut e, gathii, blocker) = setup("Dreams of the Sphinx");
    pick(&mut e, action(gathii, ActionKind::Bleed));
    // (Gathii's reveal is offered before any attempt: seat 0's window
    // does not elide.)
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, Choice::DeclareBlock { minion: blocker });
    until(&mut e, SeatId(0), WindowLabel::ActionBlock);
    pick(&mut e, vampire_ability(gathii, 0));
    let again = until_counting(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        &[vampire_ability(gathii, 0)],
    );
    assert_eq!(again, 0);
    assert_eq!(blood(&e, gathii), 4 - 1 - 1); // the reveal, then the combat
    assert_eq!(e.state.seats[0].library.len(), 1);

    // Another card, with an attempt pending: +1 stealth, the attempt fails.
    let (mut e, gathii, blocker) = setup("Conditioning");
    let before = e.state.seats[1].pool;
    pick(&mut e, action(gathii, ActionKind::Bleed));
    // (Gathii's reveal is offered before any attempt: seat 0's window
    // does not elide.)
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, Choice::DeclareBlock { minion: blocker });
    until(&mut e, SeatId(0), WindowLabel::ActionBlock);
    pick(&mut e, vampire_ability(gathii, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, gathii), 4);
    assert_eq!(e.state.seats[1].pool, before - 1);

    // The same card revealed before any attempt: no stealth gained, so the
    // block still reaches the bleed.
    let (mut e, gathii, blocker) = setup("Conditioning");
    pick(&mut e, action(gathii, ActionKind::Bleed));
    pick(&mut e, vampire_ability(gathii, 0));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, Choice::DeclareBlock { minion: blocker });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));

    // In torpor, on his leave-torpor action: the reveal still burns his
    // blood on a master card.
    let mut e = bare();
    let gathii = e.add_minion(SeatId(0), find_card("Gathii"), 4, true);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.state.seats[0]
        .library
        .push(find_library("Dreams of the Sphinx"));
    fill_hand(&mut e, 0);
    pick(&mut e, action(gathii, ActionKind::LeaveTorpor));
    pick(&mut e, vampire_ability(gathii, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, gathii), 4 - 1 - 2); // the reveal, then the cost
    assert!(!e.state.minions[gathii.index()].in_torpor);
}

// Evan Klein: a coin flip on an action another Methuselah's minion directs
// at him — tails fails it at once, the acting minion locked. His own
// controller's actions on him are undirected and never flipped.
// Rulings: [LSJ 20030214-2].
#[test]
fn evan_klein_flips_a_coin_on_an_action_directed_at_him() {
    let (mut failed, mut resolved) = (0, 0);
    for seed in 0..8 {
        let mut e = Engine::bare_seeded(seed, 2);
        e.state.turn = 4;
        let d = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        let evan = e.add_minion(SeatId(1), find_card("Evan Klein"), 2, true);
        e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        pick(
            &mut e,
            Choice::TakeAction {
                minion: d,
                action: ActionKind::Diablerie,
                target: Some(evan),
            },
        );
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[d.index()].locked);
        if e.state.minions[evan.index()].in_play {
            failed += 1;
        } else {
            resolved += 1;
        }
    }
    assert!(failed > 0 && resolved > 0, "{failed} failed, {resolved} ok");

    // His controller's own rescue: undirected, never flipped.
    for seed in 0..8 {
        let mut e = Engine::bare_seeded(seed, 2);
        e.state.turn = 4;
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let evan = e.add_minion(SeatId(1), find_card("Evan Klein"), 2, true);
        let rescuer = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        pick(
            &mut e,
            Choice::TakeAction {
                minion: rescuer,
                action: ActionKind::Rescue,
                target: Some(evan),
            },
        );
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(
            !e.state.minions[evan.index()].in_torpor,
            "rescue failed at seed {seed}"
        );
    }
}

// Adrino Manauara's mandatory press: at the press step his controller
// cannot pass while it can be used to continue, so the press is taken (its
// window holds nothing else) and a second round follows — where the press,
// spent, leaves the step passable and the combat ends (rules-qa 14.1).
#[test]
fn adrino_manauara_must_use_his_press_to_continue() {
    let mut e = bare();
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let adrino = add(&mut e, 1, "Adrino Manauara");
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: adrino });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Two rounds of hand strikes, one damage each.
    assert_eq!((blood(&e, a), blood(&e, adrino)), (2, 2));
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));

    // A blocker without the press: one round only.
    let mut e = bare();
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: x });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (3, 3));
}

// ---- Slice 9d: printed actions ------------------------------------------

/// Vampire `m`'s printed action `ability`, at `target`.
fn printed_action(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: Some(m),
        target,
    }
}

fn library(e: &mut Engine, seat: u8, cards: &[&str]) {
    for c in cards {
        e.state.seats[seat as usize].library.push(find_library(c));
    }
}

// Sakhar searches the library for an equipment card as a +1 stealth action
// and moves it to hand (or finds none); the library is shuffled. His action
// is taken once a turn, even after he unlocks (spec §3.8.1).
#[test]
fn sakhar_searches_his_library_for_an_equipment_card() {
    let mut e = bare();
    let sakhar = add(&mut e, 0, "Sakhar");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    library(
        &mut e,
        0,
        &[".44 Magnum", "Blood Rage", "Kevlar Vest", ".44 Magnum"],
    );
    fill_hand(&mut e, 0);
    let magnum = find_library(".44 Magnum");
    let kevlar = find_library("Kevlar Vest");
    pick(&mut e, printed_action(sakhar, 0, Target::None));
    let a = e.action().expect("action");
    assert_eq!((a.kind, a.stealth(&e.state)), (ActionKind::Ability, 1));
    assert!(e.state.minions[sakhar.index()].locked);
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    let mut found = vec![magnum, kevlar];
    found.sort_unstable();
    let mut want = vec![Choice::Pass];
    want.extend(found.into_iter().map(Choice::FindCard));
    assert_eq!(req.options, want);
    pick(&mut e, Choice::FindCard(kevlar));
    assert!(e.state.seats[0].hand.contains(&kevlar));
    assert_eq!(e.state.seats[0].library.len(), 3);
    e.state.minions[sakhar.index()].locked = false;
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req
        .options
        .contains(&printed_action(sakhar, 0, Target::None)));
}

// Dominica finds a master archetype card, not another master card.
#[test]
fn dominica_searches_her_library_for_a_master_archetype_card() {
    let mut e = bare();
    let dominica = add(&mut e, 0, "Dominica");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    library(&mut e, 0, &["Blood Doll", "Rebel", "Kevlar Vest"]);
    fill_hand(&mut e, 0);
    pick(&mut e, printed_action(dominica, 0, Target::None));
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    let rebel = find_library("Rebel");
    assert_eq!(req.options, vec![Choice::Pass, Choice::FindCard(rebel)]);
    pick(&mut e, Choice::FindCard(rebel));
    assert_eq!(e.state.seats[0].hand.last(), Some(&rebel));
}

// Seraphina adds 2 blood to a minion her Methuselah controls — herself, or
// one in torpor — as a +1 stealth undirected action, up to its capacity.
#[test]
fn seraphina_adds_two_blood_to_a_minion_you_control() {
    let mut e = bare();
    let seraphina = add(&mut e, 0, "Seraphina");
    let torpid = e.add_minion(SeatId(0), plain("Brujah", 0), 0, true);
    let full = e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let theirs = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    let req = request(&mut e);
    for t in [seraphina, torpid, full] {
        assert!(req
            .options
            .contains(&printed_action(seraphina, 0, Target::Minion(t))));
    }
    assert!(!req
        .options
        .contains(&printed_action(seraphina, 0, Target::Minion(theirs))));
    pick(&mut e, printed_action(seraphina, 0, Target::Minion(torpid)));
    let a = e.action().expect("action");
    assert_eq!(a.directed_seat(&e.state), None);
    assert_eq!(a.stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, torpid), 2);

    let mut e = bare();
    let seraphina = add(&mut e, 0, "Seraphina");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cap = e.state.capacity_of(seraphina).unwrap();
    e.state.minions[seraphina.index()].blood = cap - 1;
    pick(
        &mut e,
        printed_action(seraphina, 0, Target::Minion(seraphina)),
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, seraphina), cap);
}

// Elen Kamjian must bleed with +1 bleed once her Methuselah controls a
// locked minion: before any other action — the only one on offer, so the
// window elides — and no other bleed (the Codex's) discharges it. Once
// taken, if she unlocks she is free to act.
// Rulings: [LSJ 20090226] [ANK 20211009-2].
#[test]
fn elen_kamjian_must_bleed_while_you_control_a_locked_minion() {
    let mut e = bare();
    let elen = add(&mut e, 0, "Elen Kamjian");
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    // The Codex grants her another bleed, which the obligation leaves out.
    let codex = in_play(&mut e, "Codex of the Edenic Groundskeepers", 0, Some(elen));
    let codex_bleed = Choice::UseAbility {
        source: AbilitySource::Card(CardId(codex as u32)),
        ability: 0,
        actor: Some(elen),
        target: Target::None,
    };
    let forced = printed_action(elen, 0, Target::None);
    // No locked minion yet: nothing is mandatory, her action not offered.
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::Pass));
    assert!(!req.options.contains(&forced));
    let pool = e.state.seats[1].pool;
    assert!(req.options.contains(&codex_bleed));
    pick(&mut e, action(other, ActionKind::Hunt));
    let offered = until_counting(&mut e, SeatId(1), WindowLabel::MinionPhase, &[codex_bleed]);
    assert_eq!(offered, 0);
    // Her +1 bleed, taken alone (the Codex's would take 1: a ready unlocked
    // minion stands against it).
    assert_eq!(e.state.seats[1].pool, pool - 2);

    // Discharged: unlocked again, she may take any action.
    let mut e = bare();
    let elen = add(&mut e, 0, "Elen Kamjian");
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[other.index()].locked = true;
    let v = &mut e.state.minions[elen.index()];
    v.acted.push(ActionId::Printed(elen, 0));
    v.bled_this_turn = true;
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::Pass));
    assert!(req.options.contains(&action(elen, ActionKind::Hunt)));
}

// Having bled before the obligation attaches, Elen is stuck: she takes no
// action at all, while the others act freely.
// Rulings: [ANK 20211009-2].
#[test]
fn elen_kamjian_is_stuck_if_she_bled_before_her_bleed_became_mandatory() {
    let mut e = bare();
    let elen = add(&mut e, 0, "Elen Kamjian");
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, action(elen, ActionKind::Bleed));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[elen.index()].locked = false;
    // The obligation attaches once another minion is locked.
    let third = e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.state.minions[third.index()].locked = true;
    let req = request(&mut e);
    assert!(req
        .options
        .iter()
        .all(|o| !matches!(o, Choice::TakeAction { minion, .. } if *minion == elen)));
    assert!(req.options.contains(&action(other, ActionKind::Hunt)));
    assert!(req.options.contains(&Choice::Pass));
}

// Lenelle exchanges a card from hand for a library card in the ash heap as
// a +1 stealth action costing 1 blood: both named as the action is
// announced, the ash card first, one option per card; exchanged on success
// only if both are still there. Not offered with an empty hand or ash heap.
// Rulings: [LSJ 20000222] [LSJ 20011023].
#[test]
fn lenelle_names_the_cards_she_exchanges_as_she_announces_the_action() {
    let setup = || {
        let mut e = bare();
        let lenelle = add(&mut e, 0, "Lenelle, Mambo of Birmingham");
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        (e, lenelle)
    };
    let kevlar = find_library("Kevlar Vest");
    let magnum = find_library(".44 Magnum");
    let rage = find_library("Blood Rage");
    let rats = find_library("Army of Rats");
    let (mut e, lenelle) = setup();
    hand(&mut e, 0, &["Blood Rage"]);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&printed_action(lenelle, 0, Target::None)));

    let (mut e, lenelle) = setup();
    hand(&mut e, 0, &["Blood Rage", "Army of Rats", "Blood Rage"]);
    e.state.seats[0].ash.extend([kevlar, magnum]);
    pick(&mut e, printed_action(lenelle, 0, Target::None));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::ActionExchange);
    let mut ash = vec![kevlar, magnum];
    ash.sort_unstable();
    assert_eq!(
        req.options,
        ash.into_iter().map(Choice::FindCard).collect::<Vec<_>>()
    );
    pick(&mut e, Choice::FindCard(kevlar));
    let req = request(&mut e);
    assert_eq!(req.options, vec![Choice::Discard(0), Choice::Discard(1)]);
    pick(&mut e, Choice::Discard(1));
    let a = e.action().expect("action");
    assert_eq!((a.exchange, a.stealth(&e.state)), (Some((kevlar, rats)), 1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let s = &e.state.seats[0];
    assert_eq!(s.ash, vec![magnum, rats]);
    let mut held = s.hand.clone();
    held.sort_unstable();
    let mut want = vec![rage, rage, kevlar];
    want.sort_unstable();
    assert_eq!(held, want);
    assert_eq!(blood(&e, lenelle), 3);

    // The named card left the hand before resolution: no exchange, the
    // action still successful and paid for.
    let (mut e, lenelle) = setup();
    hand(&mut e, 0, &["Blood Rage", "Army of Rats"]);
    e.state.seats[0].ash.extend([kevlar, magnum]);
    pick(&mut e, printed_action(lenelle, 0, Target::None));
    pick(&mut e, Choice::FindCard(magnum));
    pick(&mut e, Choice::Discard(0));
    e.state.seats[0].hand.retain(|&c| c != rage);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].ash, vec![kevlar, magnum]);
    assert_eq!(e.state.seats[0].hand, vec![rats]);
    assert_eq!(blood(&e, lenelle), 3);
}

// "Unless she must hunt": with no blood, Elen's rulebook hunt comes before
// her mandatory bleed.
#[test]
fn elen_kamjian_hunts_instead_when_she_has_no_blood() {
    let mut e = bare();
    let elen = e.add_minion(SeatId(0), find_card("Elen Kamjian"), 0, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[other.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    // The hunt is the only option: it elides.
    let pool = e.state.seats[1].pool;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, elen), 1);
    assert_eq!(e.state.seats[1].pool, pool);
}

// Another vampire must hunt while Elen Kamjian's bleed is mandatory: the
// acting Methuselah chooses the order of the two (rules-qa 17.2).
#[test]
fn a_rulebook_hunt_and_elens_bleed_are_ordered_by_their_methuselah() {
    let mut e = bare();
    let elen = add(&mut e, 0, "Elen Kamjian");
    let starving = e.add_minion(SeatId(0), plain("Brujah", 0), 0, false);
    let locked = e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.state.minions[locked.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = request(&mut e);
    assert_eq!(
        req.options,
        vec![
            action(starving, ActionKind::Hunt),
            printed_action(elen, 0, Target::None)
        ]
    );
}

// Theo Bell enters combat with another Methuselah's ready minion as a Ⓓ
// action: directed at its controller, who alone may block; unblocked, the
// combat is part of the action (spec §4.1.2), its after-combat window
// leading to the after-resolution one (rules-qa 17.3) — Aline Gädeke's
// unlock after an Anarch's successful action is offered there. Theo strikes
// first with his +1 strength.
#[test]
fn theo_bell_enters_combat_with_a_minion_as_a_directed_action() {
    let mut e = Engine::bare(3);
    e.state.turn = 4;
    let theo = add(&mut e, 0, "Theo Bell");
    let aline = e.add_minion(SeatId(0), find_card("Aline Gädeke"), 3, false);
    e.state.minions[aline.index()].locked = true;
    let torpid = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, true);
    let target = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let other = e.add_minion(SeatId(2), plain("Brujah", 0), 4, false);
    let req = request(&mut e);
    let at = |t| printed_action(theo, 0, Target::Minion(t));
    assert!(req.options.contains(&at(target)));
    assert!(req.options.contains(&at(other)));
    for t in [theo, aline, torpid] {
        assert!(!req.options.contains(&at(t)));
    }
    pick(&mut e, at(target));
    assert_eq!(
        e.action().expect("action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    let before = blood(&e, target);
    // No block offered to seat 2.
    until_windows(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);

    // Hand strikes: Theo's 2 damage, the Ventrue's 1.
    assert_eq!(blood(&e, target), before - 2);
    assert_eq!(blood(&e, theo), 3);
    pick(&mut e, vampire_ability(aline, 0));
    assert!(!e.state.minions[aline.index()].locked);
}

// Nathaniel Bordruff enters combat with a locked vampire only; if the
// target unlocks before resolution, the action is successful (its cost
// paid) but has no effect: no combat.
// Rulings: [ANK 20210124] [LSJ 20070411] [LSJ 20090514].
#[test]
fn nathaniel_bordruff_action_fizzles_if_the_target_unlocks() {
    let mut e = bare();
    let nathaniel = add(&mut e, 0, "Nathaniel Bordruff");
    let locked = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[locked.index()].locked = true;
    let unlocked = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let req = request(&mut e);
    let at = |t| printed_action(nathaniel, 0, Target::Minion(t));
    assert!(req.options.contains(&at(locked)));
    assert!(!req.options.contains(&at(unlocked)));
    pick(&mut e, at(locked));
    e.state.minions[locked.index()].locked = false;
    let counted = until_counting(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        &[Choice::HandStrike],
    );
    assert_eq!(counted, 0);
    assert_eq!(blood(&e, locked), 4);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
}

// Adisa enters combat with a younger vampire only; after a successful hunt
// — blood gained — she does not unlock in her next unlock phase; a hunt at
// full capacity gains none and is not one (spec §3.7.2.1).
#[test]
fn adisa_enters_combat_with_younger_vampires_and_stays_locked_after_hunting() {
    let mut e = bare();
    let adisa = e.add_minion(SeatId(0), find_card("Adisa"), 2, false);
    // (A second minion keeps the minion phase from eliding into the next
    // turn, whose unlock phase would spend the skip.)
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let cap = e.state.capacity_of(adisa).unwrap();
    let (younger, older): (Vec<u16>, Vec<u16>) = (0..CRYPT.len() as u16)
        .filter(|&c| cards::vampire(c).is_none() && CRYPT[c as usize].capacity != cap)
        .partition(|&c| CRYPT[c as usize].capacity < cap);
    let y = e.add_minion(SeatId(1), younger[0], 1, false);
    let o = e.add_minion(SeatId(1), older[0], 1, false);
    let req = request(&mut e);
    assert!(req
        .options
        .contains(&printed_action(adisa, 0, Target::Minion(y))));
    assert!(!req
        .options
        .contains(&printed_action(adisa, 0, Target::Minion(o))));
    pick(&mut e, action(adisa, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, adisa), 3);
    assert!(e.state.minions[adisa.index()].skip_unlock);

    let mut e = bare();
    let adisa = e.add_minion(SeatId(0), find_card("Adisa"), cap, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, action(adisa, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[adisa.index()].skip_unlock);
}

/// As `until`, returning the windows passed on the way (seat 2 never asked
/// to block, as `until` would answer Pass for it silently).
fn until_windows(e: &mut Engine, seat: SeatId, window: WindowLabel) -> Vec<WindowLabel> {
    let mut seen = Vec::new();
    loop {
        let req = request(e);
        if (req.seat, req.window) == (seat, window) {
            return seen;
        }
        assert!(
            !(req.window == WindowLabel::ActionBlock && req.seat == SeatId(2)),
            "seat 2 offered a block on an action directed at seat 1"
        );
        seen.push(req.window);
        let i = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::Pass | Choice::HandStrike))
            .unwrap_or(0);
        e.choose(i);
    }
}

// Entering combat with Evan Klein is an action directed at him: the coin
// flips as the as-announced window closes — tails, the action fails with
// no combat (Theo still locked); heads, the combat happens.
#[test]
fn evan_klein_flips_a_coin_on_a_minion_entering_combat_with_him() {
    let (mut failed, mut fought) = (0, 0);
    for seed in 0..8 {
        let mut e = Engine::bare_seeded(seed, 2);
        e.state.turn = 4;
        let theo = add(&mut e, 0, "Theo Bell");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let evan = e.add_minion(SeatId(1), find_card("Evan Klein"), 4, false);
        pick(&mut e, printed_action(theo, 0, Target::Minion(evan)));
        until_windows(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[theo.index()].locked);
        // (Hand strikes only: the strike windows elide; the combat shows
        // in Theo's 2 damage.)
        if blood(&e, evan) < 4 {
            fought += 1;
        } else {
            assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
            failed += 1;
        }
    }
    assert!(failed > 0 && fought > 0, "{failed} failed, {fought} fought");
}

// ---- Slice 9d-iv: card targets and steals -------------------------------

fn at_card(m: MinionId, card: usize) -> Choice {
    printed_action(m, 0, Target::Card(CardId(card as u32)))
}

// Saankaláxt steals an equipment for 1 blood: directed at its controller
// (not at the minion it is on, rulebook "Target"), undirected from a minion
// of his own Methuselah; not a second vehicle, not a locquipment (a
// location once in play, spec §1.3.1), not his own.
#[test]
fn saankalaxt_steals_an_equipment_but_not_a_second_vehicle() {
    let mut e = bare();
    let saan = add(&mut e, 0, "Saankaláxt");
    let friend = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let theirs = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let bike = in_play(&mut e, "Sport Bike", 0, Some(saan));
    let vest = in_play(&mut e, "Kevlar Vest", 0, Some(friend));
    let magnum = in_play(&mut e, ".44 Magnum", 1, Some(theirs));
    let cruiser = in_play(&mut e, "Stolen Police Cruiser", 1, Some(theirs));
    let manse = in_play(&mut e, "Living Manse", 1, Some(theirs));
    let req = request(&mut e);
    for (card, offered) in [
        (bike, false),
        (vest, true),
        (magnum, true),
        (cruiser, false),
        (manse, false),
    ] {
        assert_eq!(
            req.options.contains(&at_card(saan, card)),
            offered,
            "{card}"
        );
    }
    pick(&mut e, at_card(saan, magnum));
    let a = e.action().expect("action");
    assert_eq!(a.directed_seat(&e.state), Some(SeatId(1)));
    assert_eq!(a.directed_minion(), None);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[magnum].host, Some(saan));
    assert_eq!(e.state.card_controller(CardId(magnum as u32)), SeatId(0));
    assert_eq!(e.state.cards[magnum].owner, SeatId(1));
    assert_eq!(blood(&e, saan), 3);

    // From his own Methuselah's minion: undirected.
    let mut e = bare();
    let saan = add(&mut e, 0, "Saankaláxt");
    let friend = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let vest = in_play(&mut e, "Kevlar Vest", 0, Some(friend));
    pick(&mut e, at_card(saan, vest));
    assert_eq!(e.action().expect("action").directed_seat(&e.state), None);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[vest].host, Some(saan));
}

// Darius steals a non-electronic, non-vehicle, non-weapon equipment, a
// melee weapon being a weapon; the equipment gone by resolution, the action
// is successful with no effect (spec §3.4.1).
#[test]
fn darius_shirazi_steals_neither_vehicle_nor_weapon_and_fizzles_without_the_target() {
    let mut e = bare();
    let darius = add(&mut e, 0, "Darius Shirazi");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let theirs = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let vest = in_play(&mut e, "Kevlar Vest", 1, Some(theirs));
    let blade = in_play(&mut e, "Righteous Blade", 1, Some(theirs));
    let magnum = in_play(&mut e, ".44 Magnum", 1, Some(theirs));
    let bike = in_play(&mut e, "Sport Bike", 1, Some(theirs));
    let laptop = in_play(&mut e, "Laptop Computer", 1, Some(theirs));
    let phone = in_play(&mut e, "Camera Phone", 1, Some(theirs));
    let req = request(&mut e);
    for (card, offered) in [
        (vest, true),
        (blade, false),
        (magnum, false),
        (bike, false),
        (laptop, false),
        (phone, false),
    ] {
        assert_eq!(
            req.options.contains(&at_card(darius, card)),
            offered,
            "{card}"
        );
    }
    pick(&mut e, at_card(darius, vest));
    e.state.cards[vest].in_play = false;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[vest].host, Some(theirs));
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
}

// Aniel burns a card requiring a Discipline from another ready minion as a
// +1 stealth action costing 1 blood: not from a minion in torpor, not a
// card requiring none, not from himself.
#[test]
fn aniel_burns_a_card_requiring_a_discipline_from_another_ready_minion() {
    let mut e = bare();
    let aniel = add(&mut e, 0, "Aniel");
    let ready = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let torpid = e.add_minion(SeatId(1), plain("Ventrue", 1), 0, true);
    let target = in_play(&mut e, "Homunculus", 1, Some(ready));
    let asleep = in_play(&mut e, "Homunculus", 1, Some(torpid));
    let vest = in_play(&mut e, "Kevlar Vest", 1, Some(ready));
    let his = in_play(&mut e, "Homunculus", 0, Some(aniel));
    let req = request(&mut e);
    for (card, offered) in [(target, true), (asleep, false), (vest, false), (his, false)] {
        assert_eq!(
            req.options.contains(&at_card(aniel, card)),
            offered,
            "{card}"
        );
    }
    pick(&mut e, at_card(aniel, target));
    let a = e.action().expect("action");
    assert_eq!(
        (a.stealth(&e.state), a.directed_seat(&e.state)),
        (1, Some(SeatId(1)))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[target].in_play);
    assert!(e.state.seats[1].ash.contains(&find_library("Homunculus")));
    assert_eq!(blood(&e, aniel), 3);
}

// Apolonia steals 1 blood from a younger vampire — one with none is still a
// legal target, one of her own Methuselah's an undirected one.
#[test]
fn apolonia_czarnecki_steals_one_blood_from_a_younger_vampire() {
    let mut e = bare();
    let apolonia = add(&mut e, 0, "Apolonia Czarnecki");
    let cap = e.state.capacity_of(apolonia).unwrap();
    let younger: Vec<u16> = (0..CRYPT.len() as u16)
        .filter(|&c| cards::vampire(c).is_none() && CRYPT[c as usize].capacity < cap)
        .collect();
    let y = e.add_minion(SeatId(1), younger[0], 2, false);
    let dry = e.add_minion(SeatId(1), younger[1], 0, true);
    // No vampire older than her lacks printed text: Keegan's (unlock after
    // his own Gangrel action) does not come into play here.
    let o = add(&mut e, 1, "Keegan");
    e.state.minions[o.index()].blood = 2;
    let mine = e.add_minion(SeatId(0), younger[2], 2, false);
    let req = request(&mut e);
    let at = |t| printed_action(apolonia, 0, Target::Minion(t));
    for (t, offered) in [(y, true), (dry, true), (o, false), (mine, true)] {
        assert_eq!(req.options.contains(&at(t)), offered);
    }
    pick(&mut e, at(y));
    assert_eq!(
        e.action().expect("action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, apolonia), blood(&e, y)), (5, 1));

    let mut e = bare();
    let apolonia = add(&mut e, 0, "Apolonia Czarnecki");
    let mine = e.add_minion(SeatId(0), younger[2], 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, printed_action(apolonia, 0, Target::Minion(mine)));
    assert_eq!(e.action().expect("action").directed_seat(&e.state), None);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, apolonia), blood(&e, mine)), (5, 1));
}

// ---- Slice 12a: the last vampires' printed text -------------------------

/// A card from hand played by `actor` at `target`.
fn play_at(card: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode: 0,
        actor: Some(actor),
        target,
    }
}

// Keegan unlocks after his successful action requiring a Gangrel
// (Rewilding), once each turn: not after his next one (Dog Pack's employ).
#[test]
fn keegan_unlocks_after_a_successful_action_requiring_a_gangrel() {
    let mut e = bare();
    let keegan = add(&mut e, 0, "Keegan");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let place = in_play(&mut e, "Elysian Fields", 1, None);
    hand(&mut e, 0, &["Rewilding", "Dog Pack"]);
    pick(
        &mut e,
        play_at(0, keegan, Target::Card(CardId(place as u32))),
    );
    until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    assert!(!e.state.cards[place].in_play);
    pick(&mut e, vampire_ability(keegan, 0));
    assert!(!e.state.minions[keegan.index()].locked);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, keegan));
    let again = until_counting(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        &[vampire_ability(keegan, 0)],
    );
    assert_eq!(again, 0);
    assert!(e.state.minions[keegan.index()].locked);
}

// Keegan's and Anja's "successful action" includes a fizzled one: Keegan's
// Rewilding whose location is gone by resolution; a Hecata's recruit of
// Aggressive Corpse with too little blood left to pay for it.
// Rulings: [ANK 20220218].
#[test]
fn keegan_and_anja_unlock_after_a_fizzled_action() {
    let mut e = bare();
    let keegan = add(&mut e, 0, "Keegan");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let place = in_play(&mut e, "Elysian Fields", 1, None);
    hand(&mut e, 0, &["Rewilding"]);
    pick(
        &mut e,
        play_at(0, keegan, Target::Card(CardId(place as u32))),
    );
    e.state.cards[place].in_play = false;
    let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    assert!(req.options.contains(&vampire_ability(keegan, 0)));

    let mut e = bare();
    let anja = add(&mut e, 0, "\"Mother\" Anja Giovanni");
    let hecata = e.add_minion(SeatId(0), plain("Hecata", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Aggressive Corpse"]);
    pick(&mut e, play(0, 0, hecata));
    e.state.minions[hecata.index()].blood = 1;
    let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    assert!(e.state.minions.iter().all(|m| !m.is_ally()));
    assert!(req.options.contains(&vampire_ability(anja, 0)));
}

// "Mother" Anja Giovanni, ready, burns 1 blood to unlock another Hecata
// after its successful action requiring Hecata (a recruit of Aggressive
// Corpse), once each turn; never after its hunt.
#[test]
fn mother_anja_giovanni_unlocks_another_hecata_after_its_hecata_action() {
    let mut e = bare();
    let anja = add(&mut e, 0, "\"Mother\" Anja Giovanni");
    let hecata = e.add_minion(SeatId(0), plain("Hecata", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Aggressive Corpse", "Split the Veil"]);
    pick(&mut e, action(hecata, ActionKind::Hunt));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        vampire_ability(anja, 0),
    );
    e.state.minions[hecata.index()].locked = false;
    pick(&mut e, play(0, 0, hecata));
    until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    pick(&mut e, vampire_ability(anja, 0));
    assert!(!e.state.minions[hecata.index()].locked);
    assert_eq!(blood(&e, anja), 3);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, hecata));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        vampire_ability(anja, 0),
    );
}

fn rescue(minion: MinionId, target: MinionId) -> Choice {
    Choice::TakeAction {
        minion,
        action: ActionKind::Rescue,
        target: Some(target),
    }
}

/// A vampire with no printed text of the capacities `caps`.
fn plain_of(caps: impl Fn(u8) -> bool, nth: usize) -> u16 {
    (0..CRYPT.len() as u16)
        .filter(|&c| cards::vampire(c).is_none() && caps(CRYPT[c as usize].capacity))
        .nth(nth)
        .expect("a plain vampire")
}

// Adelino Da Silva rescues an older vampire for free — with 1 blood
// between them, the rescue is offered — but a younger one costs the full 2;
// Doc Martina rescues any vampire for 1, which the rescued vampire may pay:
// the discount is the action's, whoever pays.
// Rulings: [PIB 20110918] [LSJ 20020304-2].
#[test]
fn adelino_and_doc_martina_rescue_for_less_whoever_pays() {
    let mut e = bare();
    let adelino = e.add_minion(SeatId(0), find_card("Adelino Da Silva"), 1, false);
    let older = e.add_minion(SeatId(0), plain_of(|c| c > 5, 0), 0, true);
    let younger = e.add_minion(SeatId(0), plain_of(|c| c < 5, 0), 0, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = request(&mut e);
    assert!(req.options.contains(&rescue(adelino, older)));
    assert!(!req.options.contains(&rescue(adelino, younger)));
    pick(&mut e, rescue(adelino, older));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[older.index()].in_torpor);

    let mut e = bare();
    let doc = e.add_minion(SeatId(0), find_card("Doc Martina"), 1, false);
    let t = e.add_minion(SeatId(0), plain_of(|c| c > 6, 0), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, rescue(doc, t));
    let req = until(&mut e, SeatId(0), WindowLabel::RescueCost);
    assert!(!req.options.contains(&Choice::PayRescue { from_acting: 2 }));
    pick(&mut e, Choice::PayRescue { from_acting: 0 });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_torpor);
    assert_eq!((blood(&e, doc), blood(&e, t)), (1, 1));
}

// Sergio Bueno: a minion with his Methuselah's corruption counters burns 1
// blood to attempt to block him — none with no blood; a clean one attempts
// for free; an ally pays in life — its last one too: it burns and its
// attempt fails, the other ally may still attempt. From torpor too (his
// leave-torpor action).
// Rulings: [LSJ 20011022].
#[test]
fn sergio_bueno_makes_corrupted_minions_burn_blood_or_life_to_block_him() {
    let setup = |torpor: bool| {
        let mut e = bare();
        let sergio = e.add_minion(SeatId(0), find_card("Sergio Bueno"), 4, torpor);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        (e, sergio)
    };
    let (mut e, sergio) = setup(false);
    let dry = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    let corrupted = e.add_minion(SeatId(1), plain("Ventrue", 1), 2, false);
    let clean = e.add_minion(SeatId(1), plain("Ventrue", 2), 2, false);
    for m in [dry, corrupted] {
        e.state.minions[m.index()].add_corruption(SeatId(0), 1);
    }
    pick(&mut e, bleed(sergio));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(!req.options.contains(&block(dry)));
    assert!(req.options.contains(&block(clean)));
    pick(&mut e, block(corrupted));
    assert_eq!(blood(&e, corrupted), 1);

    let (mut e, sergio) = setup(false);
    let agent = e.add_ally(SeatId(1), find_library("ECTU Operative"));
    let hurt = e.add_ally(SeatId(1), find_library("ECTU Operative"));
    e.state.minions[hurt.index()].blood = 1;
    for m in [agent, hurt] {
        e.state.minions[m.index()].add_corruption(SeatId(0), 1);
    }
    let life = blood(&e, agent);
    pick(&mut e, bleed(sergio));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, block(hurt));
    assert!(!e.state.minions[hurt.index()].in_play);
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(!req.options.contains(&block(hurt)));
    pick(&mut e, block(agent));
    assert_eq!(blood(&e, agent), life - 1);

    let (mut e, sergio) = setup(true);
    // +1 intercept against the +1 stealth leave-torpor action.
    let corrupted = add(&mut e, 1, "Rinaldo Albizzi");
    e.state.minions[corrupted.index()].blood = 2;
    e.state.minions[corrupted.index()].add_corruption(SeatId(0), 1);
    pick(&mut e, action(sergio, ActionKind::LeaveTorpor));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, block(corrupted));
    assert_eq!(blood(&e, corrupted), 1);
}

// Alexander Silverson: a vampire burns 1 blood to cast votes against his
// referendum — not offered with no blood, free in favor; a Methuselah's
// ballot (the Edge) is not a vampire's and costs nothing.
#[test]
fn alexander_silverson_makes_votes_against_his_referendum_cost_a_blood() {
    let mut e = bare();
    let alexander = add(&mut e, 0, "Alexander Silverson");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let dry = e.add_minion(SeatId(1), find_card("Alice Chen"), 0, false);
    let prince = e.add_minion(SeatId(1), find_card("Andi Liu"), 2, false);
    e.state.seats[1].has_edge = true;
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    pick(&mut e, play(0, 0, alexander));
    let req = until(&mut e, SeatId(1), WindowLabel::ReferendumPolling);
    assert!(!req.options.contains(&votes(dry, false)));
    assert!(req.options.contains(&votes(dry, true)));
    pick(&mut e, votes(prince, false));
    assert_eq!(blood(&e, prince), 1);
    let req = until(&mut e, SeatId(1), WindowLabel::ReferendumPolling);
    assert!(req.options.contains(&Choice::CastEdge { favor: false }));
    pick(&mut e, Choice::CastEdge { favor: false });
    assert_eq!(blood(&e, prince), 1);
}

// Tommaso Sforza, ready and out of the combat, burns 1 blood before range is
// determined to end the combat of the zombie ally his Methuselah controls
// (Aggressive Corpse, blocking) — not a combat of his vampire, nor from
// torpor.
#[test]
fn tommaso_sforza_ends_the_combat_of_a_wraith_or_zombie_ally() {
    let use_it = |t: MinionId| vampire_ability(t, 0);
    let (mut e, a, corpse) = {
        let mut e = bare();
        let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let corpse = e.add_ally(SeatId(1), find_library("Aggressive Corpse"));
        (e, a, corpse)
    };
    let tommaso = add(&mut e, 1, "Tommaso Sforza");
    pick(&mut e, bleed(a));
    pick(&mut e, block(corpse));
    until(&mut e, SeatId(1), WindowLabel::CombatBeforeRange);
    let life = blood(&e, corpse);
    pick(&mut e, use_it(tommaso));
    // No strike: both unhurt.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, tommaso)), (4, 3));
    assert_eq!(blood(&e, corpse), life);

    for torpor in [false, true] {
        let (mut e, a, blocker) = blocked_by("Tommaso Sforza");
        let tommaso = blocker;
        let (e, a, blocker) = if torpor {
            let corpse = e.add_ally(SeatId(1), find_library("Aggressive Corpse"));
            e.state.minions[tommaso.index()].in_torpor = true;
            (e, a, corpse)
        } else {
            (e, a, blocker)
        };
        let mut e = e;
        pick(&mut e, bleed(a));
        pick(&mut e, block(blocker));
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, use_it(tommaso));
    }
}

// Roger de Camden, ready and out of the combat, discards a card requiring
// Oblivion to give his Methuselah's blocker a maneuver (to long range),
// once each turn.
#[test]
fn roger_de_camden_discards_an_oblivion_card_for_a_maneuver() {
    let (mut e, a, blocker) = blocked_by("Roger de Camden");
    let roger = blocker;
    let blocker = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let second = e.add_minion(SeatId(0), plain("Ventrue", 2), 4, false);
    hand(&mut e, 1, &["Shadow Sentinel", "Shadow Sentinel"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(blocker));
    until(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    // The two Shadow Sentinels alike: one choice, the discard elided.
    pick(&mut e, vampire_ability(roger, 0));
    request(&mut e);
    assert_eq!(e.state.seats[1].hand.len(), 1);
    // At long range the hand strikes miss.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, blocker)), (4, 4));
    pick(&mut e, bleed(second));
    pick(&mut e, block(roger));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        vampire_ability(roger, 0),
    );
}

/// Pass until `seat`'s own unlock phase offers it its unlock abilities.
fn own_unlock(e: &mut Engine, seat: SeatId) -> DecisionRequest {
    loop {
        let req = until(e, seat, WindowLabel::UnlockAbilities);
        if e.state.active == seat {
            return req;
        }
        let i = fallback(&req);
        e.choose(i);
    }
}

fn use_on_ally(m: MinionId, ability: u8, ally: MinionId) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: None,
        target: Target::Minion(ally),
    }
}

// Sri Sansa, in torpor, moves 1 blood or life between her and a mortal ally
// her Methuselah controls during his unlock phase — once each unlock phase,
// either way; not to an ally at its starting life (its cap), never with a
// zombie; to Sri at her capacity it drains off as usual; an ally giving its
// last life burns.
// Rulings: [LSJ 20011022].
#[test]
fn sri_sansa_moves_blood_or_life_with_a_mortal_ally_in_the_unlock_phase() {
    let mut e = bare();
    let sri = e.add_minion(SeatId(0), find_card("Sri Sansa"), 3, true);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let mystic = e.add_ally(SeatId(0), find_library("Vagabond Mystic"));
    let agent = e.add_ally(SeatId(0), find_library("ECTU Operative"));
    let corpse = e.add_ally(SeatId(0), find_library("Aggressive Corpse"));
    let full = blood(&e, mystic);
    e.state.minions[mystic.index()].blood = full - 1;
    e.state.minions[corpse.index()].blood -= 1;
    let req = own_unlock(&mut e, SeatId(0));
    let offered = |m, a, t| req.options.contains(&use_on_ally(m, a, t));
    assert!(offered(sri, 0, mystic) && offered(sri, 1, mystic));
    assert!(!offered(sri, 0, agent) && offered(sri, 1, agent));
    assert!(!offered(sri, 0, corpse) && !offered(sri, 1, corpse));
    pick(&mut e, use_on_ally(sri, 0, mystic));
    assert_eq!((blood(&e, sri), blood(&e, mystic)), (2, full));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(m), .. } if *m == sri),
    );

    // At her capacity the blood still moves, and drains off from her; an
    // ally's last life burns it.
    let cap = e.state.capacity_of(sri).unwrap();
    e.state.minions[sri.index()].blood = cap;
    e.state.minions[agent.index()].blood = 1;
    let req = own_unlock(&mut e, SeatId(0));
    assert!(req.options.contains(&use_on_ally(sri, 1, agent)));
    pick(&mut e, use_on_ally(sri, 1, agent));
    request(&mut e);
    assert_eq!(blood(&e, sri), cap);
    assert!(!e.state.minions[agent.index()].in_play);
}

// Luciano Carvalho, in torpor, moves an animal retainer (Raven Spy) from a
// vampire his Methuselah controls to another — himself included, not the
// one it is on — during the discard phase, once, both chosen as he uses it;
// a non-animal retainer (Revenant) is not offered.
// Rulings: [LSJ 20011022].
#[test]
fn luciano_carvalho_moves_an_animal_retainer_between_his_vampires() {
    let mut e = bare();
    let luciano = e.add_minion(SeatId(0), find_card("Luciano Carvalho"), 3, true);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let spy = in_play(&mut e, "Raven Spy", 0, Some(a));
    let ghoul = in_play(&mut e, "Revenant", 0, Some(a));
    let move_card = |c: usize| Choice::UseAbility {
        source: AbilitySource::Vampire(luciano),
        ability: 0,
        actor: None,
        target: Target::Card(CardId(c as u32)),
    };
    let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert!(!req.options.contains(&move_card(ghoul)));
    pick(&mut e, move_card(spy));
    // Where to, announced with the use.
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert!(!req.options.contains(&Choice::ChooseMinion(a)));
    assert!(req.options.contains(&Choice::ChooseMinion(luciano)));
    pick(&mut e, Choice::ChooseMinion(b));
    assert_eq!(e.state.cards[spy].host, Some(b));
    let req = request(&mut e);
    assert!(!req.options.contains(&move_card(spy)));
}

// Gnaeus Aemilius Augustinus, ready in his Methuselah's unlock phase: the
// prey chooses one of their ready minions (not a torpid one), which takes
// 1 unpreventable damage — ordered among the unlock phase's mandatory
// effects; nothing while he is in torpor.
#[test]
fn gnaeus_aemilius_augustinus_makes_the_prey_damage_a_ready_minion() {
    let mut e = bare();
    let gnaeus = add(&mut e, 0, "Gnaeus Aemilius Augustinus");
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
    let agent = e.add_ally(SeatId(1), find_library("ECTU Operative"));
    let torpid = e.add_minion(SeatId(1), plain("Ventrue", 1), 2, true);
    e.state.seats[0].has_edge = true;
    let req = own_unlock(&mut e, SeatId(0));
    assert!(req
        .options
        .contains(&Choice::ResolvePrinted { minion: gnaeus }));
    assert!(!req.options.contains(&Choice::Pass));
    pick(&mut e, Choice::ResolvePrinted { minion: gnaeus });
    let req = expect(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert!(req.options.contains(&Choice::ChooseMinion(agent)));
    assert!(!req.options.contains(&Choice::ChooseMinion(torpid)));
    assert!(!req.options.contains(&Choice::Pass));
    pick(&mut e, Choice::ChooseMinion(v));
    assert_eq!(blood(&e, v), 1);

    e.state.minions[gnaeus.index()].in_torpor = true;
    let req = own_unlock(&mut e, SeatId(0));
    assert!(!req
        .options
        .contains(&Choice::ResolvePrinted { minion: gnaeus }));
}

// The Guardian performs no directed action (a bleed, a rescue of another
// Methuselah's vampire) unless his Methuselah controls a ready Nosferatu
// older than him — not one in torpor; undirected ones stay (a hunt).
#[test]
fn the_guardian_performs_directed_actions_only_beside_an_older_nosferatu() {
    let older = (0..CRYPT.len() as u16)
        .find(|&c| {
            cards::vampire(c).is_none()
                && CRYPT[c as usize].clan == "Nosferatu"
                && CRYPT[c as usize].capacity > 4
        })
        .expect("a plain Nosferatu older than him");
    for (nosferatu, torpor, directed) in [
        (false, false, false),
        (true, true, false),
        (true, false, true),
    ] {
        let mut e = bare();
        let guardian = add(&mut e, 0, "Guardian, The");
        if nosferatu {
            e.add_minion(SeatId(0), older, 4, torpor);
        } else {
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        }
        let theirs = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, true);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let req = request(&mut e);
        assert_eq!(req.options.contains(&bleed(guardian)), directed);
        assert_eq!(req.options.contains(&rescue(guardian, theirs)), directed);
        assert!(req.options.contains(&action(guardian, ActionKind::Hunt)));
    }
}

// "A ready unique older Nosferatu": a created vampire is not unique — a
// Nosferatu Childe of the Revolution raised to capacity 5 (four Discipline
// master cards) does not free the Guardian's bleed.
#[test]
fn the_guardian_is_not_freed_by_a_created_older_nosferatu() {
    use zvtes::cards::{Layer, Modifier, Scope};
    use zvtes::state::{Minion, MinionKind};
    let mut e = bare();
    let guardian = add(&mut e, 0, "Guardian, The");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let childe = in_play(&mut e, "Childe of the Revolution", 0, None);
    e.state.minions.push(Minion::new(
        SeatId(0),
        MinionKind::Created {
            card: find_library("Childe of the Revolution"),
            id: CardId(childe as u32),
            clan: "Nosferatu",
            sect: zvtes::crypt::Sect::Anarch,
            born: &zvtes::state::NEWBORN,
        },
        1,
    ));
    let created = MinionId(e.state.minions.len() as u32 - 1);
    e.state.cards[childe].body = Some(created);
    for _ in 0..4 {
        let c = in_play(&mut e, "Celerity", 0, Some(created));
        e.state.layers.push(Layer {
            scope: Scope::InPlay,
            modifier: Modifier::DisciplineLevel("cel"),
            source: CardId(c as u32),
        });
    }
    assert_eq!(e.state.capacity_of(created), Some(5));
    let req = request(&mut e);
    assert!(!req.options.contains(&bleed(guardian)));
}

// The Guardian locks after a successful bleed against his Methuselah to
// enter combat with the bleeder, which acts first (the ledger row: "the
// opponent is the acting minion and plays their combat effects first");
// the after-resolution window opens again once the combat is over, and the
// action ends.
// Rulings: [ANK 20221102].
#[test]
fn the_guardian_enters_combat_with_a_successful_bleeder() {
    let mut e = bare();
    // Abaddon's maneuver makes his determine-range step a decision.
    let a = add(&mut e, 0, "Abaddon");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let guardian = add(&mut e, 1, "Guardian, The");
    let before = e.state.seats[1].pool;
    pick(&mut e, bleed(a));
    until(&mut e, SeatId(1), WindowLabel::ActionAfterResolution);
    assert_eq!(e.state.seats[1].pool, before - 1);
    pick(&mut e, vampire_ability(guardian, 0));
    assert!(e.state.minions[guardian.index()].locked);
    assert_eq!(e.state.combatants, Some([a, guardian]));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::CombatDetermineRange)
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.combatants, None);
    assert!(e.action().is_none());
    // Both struck for 1.
    assert_eq!((blood(&e, a), blood(&e, guardian)), (3, 3));
}

// Hel-Blá moves an ally requiring Hecata or Oblivion from her Methuselah's
// ash heap to the ready region as a +1 stealth action that costs 1 blood:
// named as the action is declared (Screamer or Bone Shambler, not ECTU
// Operative), put in play at resolution locked, at its starting life, not
// recruited (no requirement, no cost: it may act this turn once unlocked).
// Not offered with no such ally in the ash heap; the named ally gone by
// resolution, the action has no effect (its blood still paid).
// Rulings: [LSJ 20100204] [LSJ 20040518-2] [LSJ 20100302-1] [RBK equip] [RBK recruit-ally] [RBK employ-retainer].
#[test]
fn hel_bla_moves_a_hecata_or_oblivion_ally_from_the_ash_heap() {
    let mut e = bare();
    let hel = add(&mut e, 0, "Hel-Blá");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = request(&mut e);
    assert!(!req.options.contains(&printed_action(hel, 0, Target::None)));

    let mut e = bare();
    let hel = add(&mut e, 0, "Hel-Blá");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for c in ["Screamer", "Bone Shambler", "ECTU Operative"] {
        e.state.seats[0].ash.push(find_library(c));
    }
    let pool_before = e.state.seats[0].pool;
    pick(&mut e, printed_action(hel, 0, Target::None));
    // Named at once, before any block.
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    let shambler = find_library("Bone Shambler");
    assert!(req
        .options
        .contains(&Choice::FindCard(find_library("Screamer"))));
    assert!(!req
        .options
        .contains(&Choice::FindCard(find_library("ECTU Operative"))));
    pick(&mut e, Choice::FindCard(shambler));
    assert!(e.state.minions.iter().all(|m| !m.is_ally()));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let ally = e
        .state
        .minions
        .iter()
        .position(|m| m.in_play && m.is_ally())
        .map(|i| MinionId(i as u32))
        .expect("the ally in play");
    let v = &e.state.minions[ally.index()];
    assert!(v.locked && !v.recruited);
    assert_eq!(
        v.blood,
        zvtes::library::LIBRARY[shambler as usize]
            .body
            .unwrap()
            .life
    );
    assert!(!e.state.seats[0].ash.contains(&shambler));
    assert_eq!((blood(&e, hel), e.state.seats[0].pool), (3, pool_before));

    let mut e = bare();
    let hel = add(&mut e, 0, "Hel-Blá");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].ash.push(shambler);
    // The only one: named without a decision.
    pick(&mut e, printed_action(hel, 0, Target::None));
    e.state.seats[0].ash.clear();
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions.iter().all(|m| !m.is_ally()));
    assert_eq!(blood(&e, hel), 3);
}

// Anne-Marie Bourgeois equips a .44 Magnum (2 pool) for 1 pool.
#[test]
fn anne_marie_bourgeois_equips_for_a_pool_less() {
    let mut e = bare();
    let anne = add(&mut e, 0, "Anne-Marie Bourgeois, Inconnu Recruiter");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &[".44 Magnum"]);
    let before = e.state.seats[0].pool;
    pick(&mut e, play(0, 0, anne));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, before - 1);
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == find_library(".44 Magnum") && c.host == Some(anne)));
}

// Kuyén's animals cost 1 blood or pool less, one employed through Piper
// included: Raven Spy (1 blood) for nothing; then Dog Pack (2 pool) for 1.
// Rulings: [ANK 20210913] [LSJ 20090116].
#[test]
fn kuyen_pays_less_for_animals_through_piper_too() {
    let mut e = bare();
    let kuyen = add(&mut e, 0, "Kuyén");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Piper", "Raven Spy", "Dog Pack"]);
    e.at_master_phase();
    pick(&mut e, play(0, 0, kuyen));
    let spy = find_library("Raven Spy");
    pick(&mut e, Choice::EmployFromHand { card: spy, mode: 1 });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == spy && c.host == Some(kuyen)));
    assert_eq!(blood(&e, kuyen), 4);
    e.state.minions[kuyen.index()].locked = false;
    let before = e.state.seats[0].pool;
    let pack = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Dog Pack"))
        .unwrap() as u8;
    pick(&mut e, play(pack, 0, kuyen));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, before - 1);
}

// Máddji recruits War Ghoul, an ally requiring a Tzimisce (3 pool), for 2.
#[test]
fn maddji_pays_less_for_tzimisce_allies_and_retainers() {
    let mut e = bare();
    let maddji = add(&mut e, 0, "Máddji, Mistress of Szlachtas");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["War Ghoul"]);
    let before = e.state.seats[0].pool;
    pick(&mut e, play(0, 0, maddji));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, before - 2);
    assert!(e.state.minions.iter().any(|m| m.is_ally()));
}

// Roger de Camden recruits Aggressive Corpse, requiring Hecata (2 blood),
// for 1 blood.
#[test]
fn roger_de_camden_pays_a_blood_less_for_hecata_and_oblivion_cards() {
    let mut e = bare();
    let roger = add(&mut e, 0, "Roger de Camden");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Aggressive Corpse"]);
    pick(&mut e, play(0, 0, roger));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, roger), 3);
    assert!(e.state.minions.iter().any(|m| m.is_ally()));
    let st = &e.state;
    let off = st.printed_cost_off(roger, find_library("Touch of Oblivion"), 0);
    assert_eq!((off.pool, off.blood, off.either), (0, 1, 0));
    let off = st.printed_cost_off(roger, find_library(".44 Magnum"), 0);
    assert_eq!(off, Default::default());
}

// Abraham Mellon: +1 hand size while ready. He plays cards requiring a
// Tremere as one — only those he plays: a master card requiring a ready
// Tremere (Chantry) is not his Methuselah's to play with him.
#[test]
fn abraham_mellon_adds_hand_size_but_plays_no_tremere_master() {
    let mut e = bare();
    let abraham = add(&mut e, 0, "Abraham Mellon");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert_eq!(e.state.hand_size(SeatId(0)), 8);
    assert!(e.state.meets_clan(abraham, &["Tremere"], false));
    assert!(!e.state.meets_clan(abraham, &["Tremere"], true));
    hand(&mut e, 0, &["Chantry"]);
    e.at_master_phase();
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { actor: None, .. })
    });
    e.state.minions[abraham.index()].in_torpor = true;
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
}

// Lady Scarlett Churchill plays a card requiring a Toreador as one (Scorn
// of Adonis, calling a referendum), and her Methuselah plays Toreador Grand
// Ball with her as one of its two Toreador — she may be the one it locks
// (a Ventrue may not), but she unlocks as normal after: the card's effect
// in play does not take her for a Toreador.
// Rulings: [ANK 20220816-2] [LSJ 20101013] [ANK 20190228] [ANK 20200705-2] [LSJ 20070708] [RTR 20070707].
#[test]
fn lady_scarlett_churchill_plays_toreador_cards_as_a_toreador() {
    let mut e = bare();
    let lady = add(&mut e, 0, "Lady Scarlett Churchill");
    let toreador = e.add_minion(SeatId(0), plain("Toreador", 0), 4, false);
    let ventrue = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(
        &mut e,
        0,
        &[
            "Toreador Grand Ball",
            "Ancilla Empowerment",
            "Scorn of Adonis",
        ],
    );
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.at_master_phase();
    let ball = |t| Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(t),
    };
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(req.options.contains(&ball(toreador)));
    assert!(!req.options.contains(&ball(ventrue)));
    // The other Toreador, the only one left to choose (no Ventrue): the
    // terms elide.
    pick(&mut e, ball(lady));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[lady.index()].locked);
    let grand_ball = find_library("Toreador Grand Ball");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == grand_ball && c.chosen == vec![lady, toreador]));
    // Her own card requiring a Toreador, as she calls a referendum.
    pick(&mut e, play(0, 0, toreador));
    let req = until(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    assert!(req.options.iter().any(|o| matches!(o,
        Choice::Play { card: 0, actor: Some(m), .. } if *m == toreador)));
    close_polling(&mut e);
    e.state.minions[lady.index()].locked = false;
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    let calls = e.state.seats[0].hand.len() as u8 - 1;
    pick(&mut e, play(calls, 0, lady));
    let req = until(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    assert!(req.options.iter().any(|o| matches!(o,
        Choice::Play { card: 0, actor: Some(m), .. } if *m == lady)));
    close_polling(&mut e);
    // Locked by the Ball, she unlocks as normal.
    e.state.minions[lady.index()].locked = true;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[lady.index()].locked);
}

// A card another card puts in play is still played: Regina Blake's blood
// pays the Living Manse a Tzimisce's Vast Wealth finds (rules-qa 41.13).
#[test]
fn regina_blake_pays_for_a_location_found_by_a_search() {
    let mut e = bare();
    let regina = e.add_minion(SeatId(0), find_card("Regina Blake"), 3, false);
    let tzimisce = e.add_minion(SeatId(0), plain("Tzimisce", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let wealth = in_play(&mut e, "Vast Wealth", 0, Some(tzimisce));
    e.state.seats[0].library = vec![find_library("Living Manse")];
    fill_hand(&mut e, 0);
    pick(&mut e, granted(wealth as u32, tzimisce));
    let req = until(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert!(req.options.contains(&Choice::Pass));
    pick(&mut e, Choice::ChooseMinion(regina));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, tzimisce), blood(&e, regina)), (4, 2));
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == find_library("Living Manse") && c.host == Some(tzimisce)));
}

// Regina Blake, in torpor, pays 1 of a location's cost with her blood as
// her Methuselah plays it — optional, announced with the card: Elder
// Library's 1 pool; a locquipment played as equipment too (Living Manse's
// 1 blood, equipped by a Tzimisce).
// Rulings: [LSJ 20080107] [LSJ 20080114] [LSJ 20011022].
#[test]
fn regina_blake_pays_one_of_a_location_cost_with_her_blood() {
    let mut e = bare();
    let regina = e.add_minion(SeatId(0), find_card("Regina Blake"), 3, true);
    let tzimisce = e.add_minion(SeatId(0), plain("Tzimisce", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Elder Library", "Living Manse"]);
    let before = e.state.seats[0].pool;
    e.at_master_phase();
    pick(&mut e, master_play(0));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert!(req.options.contains(&Choice::Pass));
    pick(&mut e, Choice::ChooseMinion(regina));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((e.state.seats[0].pool, blood(&e, regina)), (before, 2));
    pick(&mut e, play(0, 0, tzimisce));
    pick(&mut e, Choice::ChooseMinion(regina));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, tzimisce), blood(&e, regina)), (4, 1));
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == find_library("Living Manse") && c.in_play));
}

// ---- Slice 12b: the last five vampires' text ----------------------------

// Mithras: a younger vampire burns 1 blood to attempt to block him — none
// with no blood — and to cast votes against his referendum (in favor free);
// an ally, himself and a Methuselah's ballot (the Edge) pay nothing. From
// torpor too: his leave-torpor action.
// Rulings: [LSJ 20011022].
#[test]
fn mithras_makes_younger_vampires_burn_blood_to_block_him_or_vote_against_him() {
    let setup = |torpor: bool| {
        let mut e = bare();
        let mithras = e.add_minion(SeatId(0), find_card("Mithras"), 4, torpor);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        (e, mithras)
    };
    let (mut e, mithras) = setup(false);
    let dry = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    let younger = e.add_minion(SeatId(1), plain("Ventrue", 1), 2, false);
    let agent = e.add_ally(SeatId(1), find_library("ECTU Operative"));
    let toll = |e: &Engine, m, stat| e.state.printed_mod(m, stat, in_combat_with(mithras));
    for stat in [Stat::BlockToll, Stat::VoteAgainstToll] {
        assert_eq!(
            [younger, agent, mithras].map(|m| toll(&e, m, stat)),
            [1, 0, 0]
        );
    }
    pick(&mut e, bleed(mithras));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(!req.options.contains(&block(dry)));
    assert!(req.options.contains(&block(agent)));
    pick(&mut e, block(younger));
    assert_eq!(blood(&e, younger), 1);

    let (mut e, mithras) = setup(false);
    let agent = e.add_ally(SeatId(1), find_library("ECTU Operative"));
    let life = blood(&e, agent);
    pick(&mut e, bleed(mithras));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, block(agent));
    assert_eq!(blood(&e, agent), life);

    let (mut e, mithras) = setup(true);
    // +1 intercept against the +1 stealth leave-torpor action.
    let younger = add(&mut e, 1, "Rinaldo Albizzi");
    e.state.minions[younger.index()].blood = 2;
    pick(&mut e, action(mithras, ActionKind::LeaveTorpor));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, block(younger));
    assert_eq!(blood(&e, younger), 1);

    let (mut e, mithras) = setup(false);
    let dry = e.add_minion(SeatId(1), find_card("Alice Chen"), 0, false);
    let prince = e.add_minion(SeatId(1), find_card("Andi Liu"), 2, false);
    e.state.seats[1].has_edge = true;
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    pick(&mut e, play(0, 0, mithras));
    let req = until(&mut e, SeatId(1), WindowLabel::ReferendumPolling);
    assert!(!req.options.contains(&votes(dry, false)));
    assert!(req.options.contains(&votes(dry, true)));
    pick(&mut e, votes(prince, false));
    assert_eq!(blood(&e, prince), 1);
    let req = until(&mut e, SeatId(1), WindowLabel::ReferendumPolling);
    assert!(req.options.contains(&Choice::CastEdge { favor: false }));
    pick(&mut e, Choice::CastEdge { favor: false });
    assert_eq!(blood(&e, prince), 1);
}

// Ragnar Nordstrom plays a card requiring Protean for 1 blood less, a
// combat card too (Body Flare's 2 blood: 1); a card played using another
// Discipline takes nothing off (Invigorate's [ani] mode, not its [pro] one).
#[test]
fn ragnar_nordstrom_pays_a_blood_less_for_protean_cards() {
    let mut e = bare();
    let ragnar = add(&mut e, 0, "Ragnar Nordstrom");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let blocker = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Body Flare"]);
    pick(&mut e, bleed(ragnar));
    pick(&mut e, block(blocker));
    pick_when(&mut e, play(0, 0, ragnar));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // 1 blood for Body Flare, 1 damage from the blocker's hand strike.
    assert_eq!(blood(&e, ragnar), 2);
    let st = &e.state;
    let invigorate = find_library("Invigorate");
    let off = |mode| st.printed_cost_off(ragnar, invigorate, mode).blood;
    assert_eq!([0, 2].map(off), [0, 1]);
    let off = st.printed_cost_off(ragnar, find_library("Roundhouse"), 0);
    assert_eq!(off, Default::default());
}

// Daria: a directed action costs her 1 blood more, payable as it is
// announced with the action's own cost, paid as it resolves — a plain bleed
// 1, a 0-cost directed card (Deep Song's bleed) 1, a 1-blood one
// (Revelations [aus] at her prey) 2, not offered at 1 blood; her undirected
// actions are not affected (Revelations [AUS], hunting). Unpayable by
// resolution, the action fizzles.
#[test]
fn daria_pays_a_blood_more_for_directed_actions() {
    let setup = |b: u8| {
        let mut e = bare();
        let daria = e.add_minion(SeatId(0), find_card("Daria"), b, false);
        e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        hand(&mut e, 0, &["Revelations", "Deep Song"]);
        (e, daria)
    };
    let at_prey = |daria| Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(daria),
        target: Target::Seat(SeatId(1)),
    };
    let (mut e, daria) = setup(1);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&bleed(daria)));
    assert!(req.options.contains(&play(1, 0, daria)));
    assert!(req.options.contains(&play(0, 1, daria)));
    assert!(!req.options.contains(&at_prey(daria)));
    let before = e.state.seats[1].pool;
    pick(&mut e, bleed(daria));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, daria), e.state.seats[1].pool), (0, before - 1));

    let (mut e, daria) = setup(1);
    pick(&mut e, play(1, 0, daria));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, daria), e.state.seats[1].pool), (0, before - 2));

    let (mut e, daria) = setup(2);
    pick(&mut e, at_prey(daria));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, daria), 0);
    assert_eq!(e.state.seats[1].hand.len(), 0);

    let (mut e, daria) = setup(1);
    pick(&mut e, play(0, 1, daria));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, daria), 0);

    let (mut e, daria) = setup(1);
    pick(&mut e, action(daria, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, daria), 2);

    // Her blood gone by resolution: the bleed fizzles, still her bleed of
    // the turn.
    let (mut e, daria) = setup(1);
    pick(&mut e, bleed(daria));
    e.state.minions[daria.index()].blood = 0;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, before);
    assert!(e.state.minions[daria.index()].bled_this_turn);
}

// Djeneba: the strike cards of the minion in combat with her cost it 1
// blood more — a dodge (Side Strike) too —, not offered unpayable; a
// maneuver card (Pursuit) is not a strike, her own strike cards (Voracious
// Vermin) cost nothing more; the same whether she blocks or acts. An ally
// pays in life (The Vozhd of Juiz de Fora, playing [ani] as a vampire), its
// last one too: it burns and the combat ends, no strike resolving.
// Rulings: [ANK 20180928-1].
#[test]
fn djeneba_makes_the_opposing_minions_strike_cards_cost_a_blood_or_life() {
    let blocked = |b: u8, cards: &[&str]| {
        let mut e = bare();
        let a = e.add_minion(SeatId(0), vampire_with("cel", 0), b, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let djeneba = add(&mut e, 1, "Djeneba");
        hand(&mut e, 0, cards);
        hand(&mut e, 1, &["Voracious Vermin"]);
        pick(&mut e, bleed(a));
        pick(&mut e, block(djeneba));
        (e, a, djeneba)
    };
    let (mut e, a, djeneba) = blocked(3, &["Pursuit", "Side Strike"]);
    pick_when(&mut e, play(0, 0, a));
    assert_eq!(blood(&e, a), 3);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, djeneba));
    // Both dodge: no damage.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, djeneba)), (2, 4));
    let st = &e.state;
    let toll = |m, o| st.printed_mod(m, Stat::StrikeCardToll, in_combat_with(o));
    assert_eq!((toll(a, djeneba), toll(djeneba, a)), (1, 0));

    let (mut e, a, _) = blocked(1, &["Side Strike"]);
    e.state.minions[a.index()].blood = 0;
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));

    let ally = |life: u8| {
        let mut e = bare();
        // Its entry burn takes the Tzimisce's Revenant.
        let t = e.add_minion(SeatId(0), plain("Tzimisce", 0), 4, false);
        in_play(&mut e, "Revenant", 0, Some(t));
        let vozhd = e.add_ally(SeatId(0), find_library("Vozhd of Juiz de Fora, The"));
        let djeneba = add(&mut e, 1, "Djeneba");
        request(&mut e);
        e.state.minions[vozhd.index()].blood = life;
        hand(&mut e, 0, &["Voracious Vermin"]);
        // Its "enter combat with a vampire" action (0 bleed: no bleed).
        let id = e.state.minions[vozhd.index()].ally_id().unwrap();
        pick(
            &mut e,
            Choice::UseAbility {
                source: AbilitySource::Card(id),
                ability: 0,
                actor: Some(vozhd),
                target: Target::Minion(djeneba),
            },
        );
        pick_when(&mut e, play(0, 0, vozhd));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        (e, vozhd, djeneba)
    };
    // Its dodge: Djeneba's hand strike has no effect.
    let (e, vozhd, _) = ally(2);
    assert_eq!(blood(&e, vozhd), 1);
    let (e, vozhd, djeneba) = ally(1);
    assert!(!e.state.minions[vozhd.index()].in_play);
    assert_eq!(blood(&e, djeneba), 4);
}

// Marchesa Liliana, during her own bleed, removes seven cards of her
// Methuselah's ash heap from the game, picked one at a time — burned
// vampires included — for an additional +1 bleed (with her own +1: 3
// pool); once per action however many cards are left (the "during"
// template); not with fewer than seven.
#[test]
fn marchesa_liliana_removes_seven_ash_cards_for_bleed_once_per_action() {
    let setup = |ash: &[&str]| {
        let mut e = bare();
        let liliana = add(&mut e, 0, "Marchesa Liliana");
        e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        e.state.seats[0].ash = ash.iter().map(|c| find_library(c)).collect();
        (e, liliana)
    };
    let mut ash = vec!["Roundhouse"; 8];
    ash.extend(["Pursuit"; 8]);
    let (mut e, liliana) = setup(&ash);
    let before = e.state.seats[1].pool;
    pick(&mut e, bleed(liliana));
    until(&mut e, SeatId(0), WindowLabel::ActionBlock);
    pick(&mut e, vampire_ability(liliana, 0));
    for _ in 0..6 {
        let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        assert_eq!(req.options.len(), 2);
        pick(&mut e, Choice::FindCard(find_library("Roundhouse")));
    }
    pick(&mut e, Choice::FindCard(find_library("Pursuit")));
    let again = until_counting(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        &[vampire_ability(liliana, 0)],
    );
    assert_eq!(again, 0);
    let s = &e.state.seats[0];
    assert_eq!((s.ash.len(), s.removed.len()), (9, 7));
    assert_eq!(e.state.seats[1].pool, before - 3);

    let (mut e, liliana) = setup(&["Roundhouse"; 6]);
    pick(&mut e, bleed(liliana));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        vampire_ability(liliana, 0),
    );
    assert_eq!(e.state.seats[1].pool, before - 2);

    // A burned vampire is an ash heap card too: 6 library cards and 1
    // vampire make seven, the vampire picked among them.
    let (mut e, liliana) = setup(&["Roundhouse"; 6]);
    let burned = plain("Nosferatu", 0);
    e.state.seats[0].ash_crypt.push(burned);
    pick(&mut e, bleed(liliana));
    until(&mut e, SeatId(0), WindowLabel::ActionBlock);
    pick(&mut e, vampire_ability(liliana, 0));
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(req
        .options
        .contains(&Choice::FindCard(find_library("Roundhouse"))));
    pick(&mut e, Choice::AshVampire(burned));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let s = &e.state.seats[0];
    assert!(s.ash.is_empty() && s.ash_crypt.is_empty());
    assert_eq!(
        (s.removed.len(), s.removed_crypt.clone()),
        (6, vec![burned])
    );
    assert_eq!(e.state.seats[1].pool, before - 3);
}
