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
    assert!(cards::vampire(find_card("Alice Chen")).is_none());
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
    assert_eq!(incomplete, 9);
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

/// A card played from hand in its first mode.
fn play_card(card: u8, actor: MinionId) -> Choice {
    Choice::Play {
        card,
        mode: 0,
        actor: Some(actor),
        target: Target::None,
    }
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
    pick(&mut e, play_card(0, alexa));
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
    pick(&mut e, play_card(0, alexa));
    let req = until(&mut e, SeatId(1), WindowLabel::ReferendumPolling);
    assert!(req.options.contains(&vampire_ability(yewon, 0)));
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
    pick(&mut e, play_card(0, anne));
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
    pick(&mut e, act(hunter, ActionKind::Hunt));
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
    pick(&mut e, act(gwen, ActionKind::LeaveTorpor));
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
    pick(&mut e, act(larissa, ActionKind::Bleed));
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
    pick(&mut e, act(larissa, ActionKind::Hunt));
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
    pick(&mut e, act(abraham, ActionKind::Bleed));
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
    pick(&mut e, act(hunter, ActionKind::Hunt));
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
    pick(&mut e, act(nosferatu, ActionKind::Bleed));
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
    let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert!(req.options.contains(&vampire_ability(mora, 0)));
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
    pick(&mut e, act(x, ActionKind::Bleed));
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
        pick(&mut e, play_card(0, sybren));
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
    pick(&mut e, act(dario, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    pick(&mut e, vampire_ability(aline, 0));
    assert!(!e.state.minions[aline.index()].locked);
    assert_eq!(blood(&e, aline), 2);
    e.state.minions[aline.index()].locked = true;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, act(crow, ActionKind::Hunt));
    let again = until_counting(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        &[vampire_ability(aline, 0)],
    );
    assert_eq!(again, 0);

    let (mut e, aline) = setup();
    let dario = e.add_minion(SeatId(0), find_card("Dario Ziggler"), 2, true);
    pick(&mut e, act(dario, ActionKind::LeaveTorpor));
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
    pick(&mut e, act(alicia, ActionKind::Bleed));
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
    pick(&mut e, act(a, ActionKind::Bleed));
    until(&mut e, SeatId(1), WindowLabel::ActionAfterResolution);
    pick(&mut e, vampire_ability(ilonka, 0));
    let req = request(&mut e);
    assert_eq!(req.options, vec![Choice::SetX(0), Choice::SetX(1)]);
    pick(&mut e, Choice::SetX(1));
    assert_eq!(e.state.seats[0].hand.len(), 1);
    assert_eq!(e.state.seats[0].ash.len(), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, act(b, ActionKind::Bleed));
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
    pick(&mut e, act(richard, ActionKind::Bleed));
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
    pick(&mut e, act(marialena, ActionKind::Bleed));
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
    pick(&mut e, act(marialena, ActionKind::LeaveTorpor));
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
    pick(&mut e, act(a, ActionKind::Bleed));
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
        pick(&mut e, play_card(0, cedrick));
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
    pick(&mut e, act(leaver, ActionKind::LeaveTorpor));
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
        (e, gathii, blocker)
    };
    // A master card on top: 1 blood, whatever the block attempt.
    let (mut e, gathii, blocker) = setup("Dreams of the Sphinx");
    pick(&mut e, act(gathii, ActionKind::Bleed));
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
    pick(&mut e, act(gathii, ActionKind::Bleed));
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
    pick(&mut e, act(gathii, ActionKind::Bleed));
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
    pick(&mut e, act(gathii, ActionKind::LeaveTorpor));
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
    pick(&mut e, act(a, ActionKind::Bleed));
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
    pick(&mut e, act(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: x });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (3, 3));
}
