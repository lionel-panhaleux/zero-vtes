//! Slice 9a: printed vampire text — plain stat modifiers
//! (`cards::vampires`), a vampire's whole text or the plain half of a
//! two-part one (rules-qa 13.1), read live by `GameState::printed_mod`
//! on bleeds, stealth, intercept, strength, votes and hand size. Windows
//! with a single option are elided, so the scripts only stop where a real
//! decision exists.

use zvtes::cards::{self, vampires::VAMPIRES, AbilitySource, Stat, Target};
use zvtes::crypt::{Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, CardId, CardInPlay, PrintedCtx};

fn request(e: &mut Engine) -> DecisionRequest {
    match e.run() {
        Step::Decision(req) => {
            if std::env::var("TRACE").is_ok() {
                eprintln!("REQ {:?} {:?} {:?}", req.seat, req.window, req.options);
            }
            req
        }
        Step::Over(_) => panic!("game over while expecting a decision"),
    }
}

fn pick(e: &mut Engine, want: Choice) {
    let req = request(e);
    let idx = req
        .options
        .iter()
        .position(|o| *o == want)
        .unwrap_or_else(|| {
            panic!(
                "option {want:?} not in {:?} ({:?})",
                req.options, req.window
            )
        });
    e.choose(idx);
}

/// Pass through every decision (hand strikes in combat) until `seat`'s
/// `window`.
fn until(e: &mut Engine, seat: SeatId, window: WindowLabel) -> DecisionRequest {
    loop {
        let req = request(e);
        if (req.seat, req.window) == (seat, window) {
            return req;
        }
        let i = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::Pass | Choice::HandStrike))
            .unwrap_or(0);
        e.choose(i);
    }
}

fn act(minion: MinionId, action: ActionKind) -> Choice {
    Choice::TakeAction {
        minion,
        action,
        target: None,
    }
}

/// The `nth` untitled V5 vampire of `clan` with no printed-text definition
/// and capacity at least 4.
fn plain(clan: &str, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.clan == clan
                && c.title == Title::None
                && c.capacity >= 4
                && cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no plain {clan} vampire"))
}

fn add(e: &mut Engine, seat: u8, name: &str) -> MinionId {
    e.add_minion(SeatId(seat), find_card(name), 4, false)
}

/// A library card in play, on `host` if any.
fn in_play(e: &mut Engine, name: &str, owner: u8, host: Option<MinionId>) -> usize {
    e.state.cards.push(CardInPlay {
        card: find_library(name),
        owner: SeatId(owner),
        controller: SeatId(owner),
        host,
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
        stored: Vec::new(),
    });
    e.state.cards.len() - 1
}

fn during(kind: ActionKind, directed: bool, opponent: Option<MinionId>) -> PrintedCtx {
    PrintedCtx {
        kind: Some(kind),
        directed: Some(directed),
        opponent,
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

#[test]
fn every_vampire_definition_names_a_v5_crypt_card() {
    for def in VAMPIRES {
        let card = find_card(def.name);
        assert_eq!(cards::vampire(card).map(|d| d.name), Some(def.name));
    }
    assert!(cards::vampire(find_card("Alexa Draper")).is_none());
}

// Catalina Vega's "+1 bleed": her bleed takes 2 pool.
#[test]
fn a_printed_bleed_bonus_adds_to_the_bleed_amount() {
    let mut e = bare();
    let catalina = add(&mut e, 0, "Catalina Vega");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Nosferatu", 0), 4, false);
    let before = e.state.seats[1].pool;
    pick(&mut e, act(catalina, ActionKind::Bleed));
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
    pick(&mut e, act(hunter, ActionKind::Hunt));
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
    pick(&mut e, act(ariane, ActionKind::Hunt));
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
    pick(&mut e, act(a, ActionKind::Bleed));
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
    pick(&mut e, act(brujah, ActionKind::Bleed));
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

// Rules-qa 13.1: a vampire lands with part of its text, the rest quoted in
// `incomplete` — the plain stat half of a two-part vampire applies.
#[test]
fn the_plain_half_of_a_two_part_vampire_applies_and_the_rest_is_named() {
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
    let incomplete = VAMPIRES.iter().filter(|d| d.incomplete.is_some()).count();
    assert_eq!(incomplete, 18);
    assert!(cards::vampire(find_card("Theo Bell")).is_some_and(|d| d.incomplete.is_some()));
    assert!(cards::vampire(find_card("Catalina Vega")).is_some_and(|d| d.incomplete.is_none()));
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

fn hand(e: &mut Engine, seat: u8, cards: &[&str]) {
    for c in cards {
        e.state.seats[seat as usize].hand.push(find_library(c));
    }
}

fn blood(e: &Engine, m: MinionId) -> u8 {
    e.state.minions[m.index()].blood
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
    pick(&mut e, act(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: abaddon });
    let req = until(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&maneuver));
    pick(&mut e, maneuver);
    let req = until(&mut e, SeatId(1), WindowLabel::CombatPress);
    assert!(req.options.contains(&press));
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
    pick(&mut e, act(abaddon, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: flavio });
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(req.options.contains(&dodge));
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
    pick(&mut e, act(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: phaibun });
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(req.options.contains(&dodge));
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
    pick(&mut e, act(a, ActionKind::Bleed));
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
    pick(&mut e, act(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: x });
    let req = until(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    assert!(req.options.contains(&prevent));
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
    pick(&mut e, act(a, ActionKind::Bleed));
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
    pick(&mut e, act(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: rowena });
    let req = until(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    assert!(req.options.contains(&prevent));
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
    pick(&mut e, act(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: chun });
    let req = until(&mut e, SeatId(1), WindowLabel::CombatBeforeStrikes);
    assert!(req.options.contains(&strike));
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
    pick(&mut e, act(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: kasim });
    let req = until(&mut e, SeatId(1), WindowLabel::CombatBeforeRange);
    assert!(req.options.contains(&ability));
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
    pick(&mut e, act(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: noluthando });
    until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    pick(&mut e, card_ability(gun, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, noluthando)), (4 - 3, 4 - 1));
}
