//! Round 33 (`ADDED_BATCHES`), track C: Kindred Most Wanted and Tenth
//! Anniversary vampires. Windows with a single option are elided, so the
//! scripts only stop where a real decision exists.

use zvtes::actions::Outcome;
use zvtes::cards::{AbilitySource, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, CardId};

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

/// The `nth` untitled fixture vampire matching `f` (`fixture_vampire`).
#[track_caller]
fn textless(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    fixture_vampire(|c| f(c) && c.title == Title::None, nth)
}

/// The crypt card `name` of group `group`.
fn crypt_card(name: &str, group: u8) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && c.group == group)
        .unwrap_or_else(|| panic!("no {name} G{group}")) as u16
}

/// Whether `o` uses vampire `m`'s printed ability `ability`.
fn uses_vampire(o: &Choice, m: MinionId, ability: u8) -> bool {
    matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), ability: i, .. }
        if *v == m && *i == ability)
}

/// The first offer of vampire `m`'s ability `ability`, the fallback taken
/// until then.
fn vampire_use(e: &mut Engine, m: MinionId, ability: u8) -> Choice {
    for _ in 0..300 {
        let req = request(e);
        if let Some(o) = req.options.iter().find(|o| uses_vampire(o, m, ability)) {
            return *o;
        }
        e.choose(fallback(&req));
    }
    panic!("{m:?}'s ability {ability} never offered");
}

/// Vampire `m`'s printed action `ability` at `target`.
fn vampire_action(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: Some(m),
        target,
    }
}

/// The ability `ability` of the card in play `c`, used by its Methuselah.
fn card_ability(c: CardId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(c),
        ability,
        actor: None,
        target: Target::None,
    }
}

/// `a` bleeds and `z` blocks.
fn bleed_blocked(e: &mut Engine, a: MinionId, z: MinionId) {
    pick(e, bleed(a));
    pick_when(e, block(z));
}

/// The damage pending on combat side `side`: (normal, aggravated).
fn pending(e: &Engine, side: usize) -> (u8, u8) {
    let p = e.combat().expect("combat").sides[side].pending;
    (p.normal, p.aggravated)
}

/// A master: Discipline card `name` of `seat` on `host`, its level with it
/// (its mode's layer).
fn master_on(e: &mut Engine, seat: u8, name: &str, host: MinionId) -> CardId {
    in_play(e, seat, name, Some(host))
}

/// The damage `card`'s hand strike puts on a [for] bleeder holding
/// Relentless Reaper it blocks, read as the bleeder may prevent it:
/// (normal, aggravated).
fn hand_strike_damage(card: u16) -> (u8, u8) {
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 2), 4, false);
    let a = e.add_minion(SeatId(1), card, 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 0), 4, false);
    hand(&mut e, 0, &["Relentless Reaper"]);
    bleed_blocked(&mut e, x, a);
    offering(&mut e, play(0, 0, x));
    pending(&e, 0)
}

// ---- Text-only vampires -------------------------------------------------------

/// Dieter Kleist, Dollface, Dorian Strack, Dr. John Casey, Dre, Duck,
/// Felicia Mostrom, Gideon Fontaine, Giuliano Vincenzi ("Camarilla."),
/// Emerson Bridges ("Camarilla Prince of Washington, DC."), Gilbert Duane
/// G1 ("Camarilla Prince of Miami."), Gitane St. Claire ("Camarilla
/// primogen."): no definition, their title generated.
#[test]
fn round33_c_textless_vampires_carry_only_generated_traits() {
    for (name, title) in [
        ("Dieter Kleist", Title::None),
        ("Dollface", Title::None),
        ("Dorian Strack", Title::None),
        ("Dr. John Casey", Title::None),
        ("Dre, Leader of the Cold Dawn", Title::None),
        ("Duck", Title::None),
        ("Felicia Mostrom", Title::None),
        ("Gideon Fontaine", Title::None),
        ("Giuliano Vincenzi", Title::None),
        ("Emerson Bridges", Title::Prince("Washington, DC")),
        ("Gilbert Duane", Title::Prince("Miami")),
        ("Gitane St. Claire", Title::Primogen),
    ] {
        let i = crypt_card(name, 1);
        assert!(zvtes::cards::vampire(i).is_none(), "{name} has text");
        assert_eq!(CRYPT[i as usize].title, title, "{name}");
        assert_eq!(CRYPT[i as usize].sect, Some(zvtes::crypt::Sect::Camarilla));
    }
}

/// Gilbert Duane G1 prints no text: the G6 card's influence-phase ability is
/// not his (the definition bound to group 6), the G6 card keeps it.
#[test]
fn gilbert_duane_g1_has_only_his_title() {
    let g6 = crypt_card("Gilbert Duane", 6);
    assert!(zvtes::cards::vampire(g6).is_some_and(|d| !d.abilities.is_empty()));
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), crypt_card("Gilbert Duane", 1), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    assert_eq!(e.state.title_of(g), Title::Prince("Miami"));
    // Through his next influence phase.
    for seat in [1, 0] {
        never_offered_where(&mut e, SeatId(seat), WindowLabel::MinionPhase, |o| {
            uses_vampire(o, g, 0)
        });
    }
}

// ---- Don Cruez -----------------------------------------------------------------

/// Don Cruez: "Once each combat, Don Cruez may burn 1 blood to get one
/// maneuver." Used as the range is determined, he goes to long range for 1
/// blood; maneuvered back to close, it is not offered again that combat.
#[test]
fn don_cruez_buys_one_maneuver_a_combat() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Don Cruez, The Idealist"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    hand(&mut e, 0, &["Fake Out"]);
    hand(&mut e, 1, &["Fake Out"]);
    bleed_blocked(&mut e, d, x);
    let u = vampire_use(&mut e, d, 0);
    pick(&mut e, u);
    request(&mut e);
    assert_eq!(blood(&e, d), 3, "1 blood");
    assert_eq!(
        e.combat().expect("combat").range,
        zvtes::combat::Range::Long
    );
    pick_when(&mut e, play(0, 0, x));
    // Back at close, Don may answer with his Fake Out, not his ability.
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::CombatDetermineRange)
    );
    assert_eq!(
        e.combat().expect("combat").range,
        zvtes::combat::Range::Close
    );
    assert!(req.options.contains(&play(0, 0, d)));
    assert!(
        !req.options.iter().any(|o| uses_vampire(o, d, 0)),
        "once each combat"
    );
}

// ---- Doyle Fincher -------------------------------------------------------------

/// Doyle Fincher: "During your discard phase, if any of your prey's ready
/// vampires are not at full capacity, move 1 blood from the blood bank to
/// one of those vampires and 1 blood from the blood bank to Doyle. +1
/// bleed." His bleed of 2; in the discard phase his controller picks among
/// the prey's ready vampires below capacity only (not the full one, not the
/// torpid one); the one picked and Doyle each gain 1.
#[test]
fn doyle_fincher_feeds_the_preys_hungry_vampire_and_himself() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Doyle Fincher"), 3, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let cap4 = |n| textless(|c| c.capacity == 4, n);
    let full = e.add_minion(SeatId(1), cap4(1), 4, false);
    let low = e.add_minion(SeatId(1), cap4(2), 2, false);
    let low2 = e.add_minion(SeatId(1), cap4(3), 1, false);
    let torpid = e.add_minion(SeatId(1), cap4(4), 1, true);
    pick(&mut e, bleed(d));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(e.state.seats[1].pool, 10 - 2, "+1 bleed");
    assert_eq!(
        req.options,
        vec![Choice::ChooseMinion(low), Choice::ChooseMinion(low2)]
    );
    assert_eq!(blood(&e, d), 4, "Doyle's 1");
    pick(&mut e, Choice::ChooseMinion(low));
    request(&mut e);
    assert_eq!(blood(&e, low), 3);
    assert_eq!(
        (blood(&e, full), blood(&e, low2), blood(&e, torpid)),
        (4, 1, 1)
    );
}

/// Doyle Fincher: the prey's ready vampires all at full capacity (a torpid
/// one below it does not count): no blood for anyone, Doyle included.
#[test]
fn doyle_fincher_does_nothing_when_the_prey_is_full() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Doyle Fincher"), 3, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let cap4 = |n| textless(|c| c.capacity == 4, n);
    let full = e.add_minion(SeatId(1), cap4(1), 4, false);
    let torpid = e.add_minion(SeatId(1), cap4(2), 1, true);
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::ChooseMinion(_))
    });
    assert_eq!(
        (blood(&e, d), blood(&e, full), blood(&e, torpid)),
        (3, 4, 1)
    );
}

// ---- Dr. Jest -----------------------------------------------------------------

/// Dr. Jest at seat 0, seat 1 holding 7 Form of Mist over 3 Ancilla
/// Empowerment in its library.
fn dr_jest(torpor: bool, locked: bool) -> (Engine, MinionId) {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Dr. Jest"), 2, torpor);
    e.state.minions[j.index()].locked = locked;
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    fill_hand(&mut e, 1);
    for _ in 0..3 {
        e.state.seats[1]
            .library
            .push(find_library("Ancilla Empowerment"));
    }
    (e, j)
}

/// Dr. Jest: "If Dr. Jest is unlocked at the start of your turn, then
/// during your unlock phase, you may make your prey discard one card at
/// random from his or her hand (and draw back up to his or her hand size
/// afterward)." [LSJ 20011022] "The ability can be used when in torpor":
/// in torpor, at his next unlock phase, the prey discards one of its Form
/// of Mist and draws back to 7.
#[test]
fn dr_jest_makes_the_prey_discard_from_torpor() {
    let (mut e, j) = dr_jest(true, false);
    let u = vampire_use(&mut e, j, 0);
    assert_eq!(e.state.active, SeatId(0));
    assert!(matches!(
        u,
        Choice::UseAbility {
            target: Target::Seat(SeatId(1)),
            ..
        }
    ));
    pick(&mut e, u);
    request(&mut e);
    let mist = find_library("Form of Mist");
    let ash = &e.state.seats[1].ash;
    assert_eq!((ash.len(), ash.contains(&mist)), (1, true));
    let hand = &e.state.seats[1].hand;
    assert_eq!(hand.len(), 7, "drawn back up");
    assert!(hand.contains(&find_library("Ancilla Empowerment")));
}

/// Dr. Jest locked as his turn begins: he unlocks in the unlock phase, his
/// ability not offered that turn.
#[test]
fn dr_jest_needs_to_be_unlocked_as_the_turn_begins() {
    let (mut e, j) = dr_jest(false, true);
    pick(&mut e, Choice::Pass);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, j, 0)
    });
    assert_eq!(e.state.active, SeatId(0));
    assert!(!e.state.minions[j.index()].locked, "unlocked");
    assert!(e.state.seats[1].ash.is_empty());
}

// ---- Dylan, Eliott Sinclair, Gillian Krader --------------------------------------

/// Dylan: "Dylan gets +1 stealth when bleeding. +1 strength." A bleed of
/// stealth 1, a hunt of its own +1 only; his hand strike deals 2. Red List and infernal
/// generated.
#[test]
fn dylan_sneaks_on_bleeds_only_and_strikes_for_two() {
    let dylan = find_card("Dylan");
    assert!(CRYPT[dylan as usize].red_list && CRYPT[dylan as usize].infernal);
    assert_eq!(hand_strike_damage(dylan), (2, 0));
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), dylan, 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    pick(&mut e, bleed(d));
    request(&mut e);
    assert_eq!(e.action().expect("bleed").stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[d.index()].locked = false;
    pick(&mut e, action(d, ActionKind::Hunt));
    request(&mut e);
    assert_eq!(
        e.action().expect("hunt").stealth(&e.state),
        1,
        "the hunt's own"
    );
}

/// Eliott Sinclair: "Camarilla primogen: +1 strength." His hand strike
/// deals 2; a primogen.
#[test]
fn eliott_sinclair_strikes_for_two() {
    let el = find_card("Eliott Sinclair, Virtuoso Thespian");
    assert_eq!(CRYPT[el as usize].title, Title::Primogen);
    assert_eq!(hand_strike_damage(el), (2, 0));
    assert_eq!(hand_strike_damage(textless(|c| c.capacity >= 4, 3)), (1, 0));
}

/// Gillian Krader: "Gillian cannot perform political actions." Ancilla
/// Empowerment is not offered to her, offered to another vampire.
#[test]
fn gillian_krader_calls_no_referendum() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Gillian Krader"), 2, false);
    let o = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, g)));
    assert!(req.options.contains(&play(0, 0, o)));
}

// ---- Ebanezer Roush ------------------------------------------------------------

/// Ebanezer Roush: "Ebanezer may enter combat with a vampire of capacity
/// below 4 controlled by your predator or prey as a +2 stealth Ⓓ action."
/// At 4 seats: the prey's and the predator's capacity-3 vampires offered,
/// not the prey's capacity 4 one, nor the cross-table capacity-2 one; the
/// action of stealth 2, directed at the prey; unblocked, the combat with
/// the target.
#[test]
fn ebanezer_roush_attacks_small_vampires_of_his_neighbours() {
    let mut e = game(4);
    let eb = e.add_minion(SeatId(0), find_card("Ebanezer Roush"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let small = e.add_minion(SeatId(1), textless(|c| c.capacity == 3, 0), 3, false);
    e.state.minions[small.index()].gained_levels = vec!["cel"];
    hand(&mut e, 1, &["Side Strike"]);
    let big = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 4, false);
    let far = e.add_minion(SeatId(2), textless(|c| c.capacity == 2, 0), 2, false);
    let pred = e.add_minion(SeatId(3), textless(|c| c.capacity == 3, 1), 3, false);
    let req = request(&mut e);
    let at = |m| vampire_action(eb, 0, Target::Minion(m));
    assert!(req.options.contains(&at(small)));
    assert!(req.options.contains(&at(pred)));
    assert!(!req.options.contains(&at(big)), "capacity 4");
    assert!(!req.options.contains(&at(far)), "not a neighbour");
    pick(&mut e, at(small));
    request(&mut e);
    let a = e.action().expect("action");
    assert_eq!(a.stealth(&e.state), 2);
    assert_eq!(a.directed_seat(&e.state), Some(SeatId(1)));
    until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    let c = e.combat().expect("combat");
    assert_eq!([c.sides[0].minion, c.sides[1].minion], [eb, small]);
}

// ---- Elihu --------------------------------------------------------------------

/// Elihu bearing Femur of Toomler, blocked by a [cel][for] vampire holding
/// Side Strike and Relentless Reaper.
fn elihu() -> (Engine, MinionId, MinionId, CardId) {
    let mut e = game(2);
    let el = e.add_minion(SeatId(0), find_card("Elihu"), 6, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["cel", "for"];
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    let w = in_play(&mut e, 0, "Femur of Toomler", Some(el));
    hand(&mut e, 1, &["Side Strike", "Relentless Reaper"]);
    (e, el, z, w)
}

/// Elihu: "Elihu inflicts +2 damage with melee weapons. His strikes with
/// melee weapons cannot be dodged." [RTR 19970630] [TOM 19960225]
/// "Additional damage inherits all of the properties of the base damage":
/// his Femur of Toomler strike lands through a dodge (LSJ 20030902-2), 1
/// strength + 1 + 2, all aggravated.
#[test]
fn elihu_strikes_through_a_dodge_with_a_melee_weapon() {
    let (mut e, el, z, w) = elihu();
    assert!(CRYPT[find_card("Elihu") as usize].infernal);
    bleed_blocked(&mut e, el, z);
    pick_when(&mut e, card_ability(w, 0));
    pick_when(&mut e, play(0, 0, z));
    offering(&mut e, play(0, 0, z));
    assert_eq!(pending(&e, 1), (0, 4));
}

/// Elihu's hand strike is dodged: no damage on the dodging vampire.
#[test]
fn elihu_hand_strike_is_dodged() {
    let (mut e, el, z, _) = elihu();
    bleed_blocked(&mut e, el, z);
    pick_when(&mut e, Choice::HandStrike);
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4, "dodged");
    assert!(!e.state.minions[z.index()].in_torpor);
}

// ---- Ethan Locke --------------------------------------------------------------

/// Ethan Locke: "As a Ⓓ action, Ethan may steal a Discipline master card
/// from another vampire (moving the card to himself) if he does not already
/// possess the superior level of that Discipline." [LSJ 19990709] "When he
/// steals a master Discipline card, his controller becomes the controller
/// of the Discipline card": the prey's Celerity moves to him, under seat 0,
/// his Celerity superior, the victim's capacity down by 1.
#[test]
fn ethan_locke_takes_control_of_the_stolen_master_card() {
    let mut e = game(2);
    let et = e.add_minion(SeatId(0), find_card("Ethan Locke"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    let cel = master_on(&mut e, 1, "Celerity", z);
    assert_eq!(e.state.discipline_level(et, "cel"), 1);
    let cap = e.state.capacity_of(z).expect("a vampire");
    pick(&mut e, vampire_action(et, 0, Target::Card(cel)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[cel.index()].host, Some(et));
    assert_eq!(e.state.card_controller(cel), SeatId(0));
    assert_eq!(e.state.discipline_level(et, "cel"), 2);
    assert_eq!(e.state.discipline_level(z, "cel"), 0);
    assert_eq!(e.state.capacity_of(z), Some(cap - 1));
}

/// [LSJ 19970224] "Ethan's action is directed at the controller of the
/// skill card, not the controller of the vampire": a Celerity on seat 1's
/// vampire under seat 2's control: the action is directed at seat 2.
#[test]
fn ethan_locke_steal_is_directed_at_the_cards_controller() {
    let mut e = game(3);
    let et = e.add_minion(SeatId(0), find_card("Ethan Locke"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(2), textless(|c| c.capacity >= 4, 2), 4, false);
    let cel = master_on(&mut e, 2, "Celerity", z);
    assert_eq!(e.state.card_controller(cel), SeatId(2));
    pick(&mut e, vampire_action(et, 0, Target::Card(cel)));
    request(&mut e);
    assert_eq!(
        e.action().expect("action").directed_seat(&e.state),
        Some(SeatId(2))
    );
}

/// Ethan Locke: not a Discipline he has at superior (Thaumaturgy), nor his
/// own Methuselah's master card (rules-qa 90.4), nor a card on an ally
/// ("from another vampire").
#[test]
fn ethan_locke_cannot_steal_a_discipline_he_has_at_superior() {
    let mut e = game(2);
    let et = e.add_minion(SeatId(0), find_card("Ethan Locke"), 4, false);
    let own = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Screamer"));
    let tha = master_on(&mut e, 1, "Thaumaturgy", z);
    let mine = master_on(&mut e, 0, "Celerity", own);
    let on_ally = master_on(&mut e, 1, "Celerity", ally);
    let cel = master_on(&mut e, 1, "Celerity", z);
    let req = request(&mut e);
    let at = |c| vampire_action(et, 0, Target::Card(c));
    assert!(req.options.contains(&at(cel)));
    assert!(!req.options.contains(&at(tha)), "superior Thaumaturgy");
    assert!(!req.options.contains(&at(mine)), "his own Methuselah's");
    assert!(!req.options.contains(&at(on_ally)), "an ally's");
}

// ---- vamp_d: text-only vampires --------------------------------------------------

/// Hector Sosa, Ignatius, Jing Wei, Kallista, Master Sculptor, KoKo
/// ("Camarilla."), Jeffrey Mullins, Jesús Alcalá ("Sabbat."): no
/// definition, their sect generated.
#[test]
fn round33_c_vamp_d_textless_vampires_carry_only_their_sect() {
    use zvtes::crypt::Sect;
    for (name, group, sect) in [
        ("Hector Sosa", 1, Sect::Camarilla),
        ("Ignatius", 1, Sect::Camarilla),
        ("Jing Wei", 1, Sect::Camarilla),
        ("Kallista, Master Sculptor", 1, Sect::Camarilla),
        ("KoKo", 1, Sect::Camarilla),
        ("Jeffrey Mullins", 4, Sect::Sabbat),
        ("Jesús Alcalá", 3, Sect::Sabbat),
    ] {
        let i = crypt_card(name, group);
        assert!(zvtes::cards::vampire(i).is_none(), "{name} has text");
        assert_eq!(CRYPT[i as usize].title, Title::None, "{name}");
        assert_eq!(CRYPT[i as usize].sect, Some(sect), "{name}");
    }
}

// ---- Grendel the Worm-Eaten -------------------------------------------------------

/// Grendel: "While Grendel is ready, you may look at the top card of your
/// crypt." His controller's view keeps the crypt's top on top (the rest
/// sorted), locked too; another seat's view hides it; in torpor, sorted
/// again.
#[test]
fn grendel_shows_his_controller_the_crypt_top_while_ready() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Grendel the Worm-Eaten"), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 0), 4, false);
    let (high, low) = (
        textless(|c| c.capacity >= 4, 1),
        textless(|c| c.capacity >= 4, 2),
    );
    let (high, low) = (high.max(low), high.min(low));
    let crypt = &mut e.state.seats[0].crypt;
    crypt.push(low);
    crypt.push(high);
    crypt.push(low);
    let top = |e: &Engine, viewer: u8| {
        e.state.legal_view(SeatId(viewer)).seats[0]
            .crypt
            .last()
            .copied()
    };
    assert_eq!(top(&e, 0), Some(low), "the top seen");
    e.state.minions[g.index()].locked = true;
    assert_eq!(top(&e, 0), Some(low), "locked, still ready");
    assert_eq!(top(&e, 1), Some(zvtes::state::HIDDEN));
    e.state.minions[g.index()].in_torpor = true;
    assert_eq!(top(&e, 0), Some(high), "sorted: the top unknown");
}

// ---- Gunther, Beast Lord ----------------------------------------------------------

/// Gunther: "During your discard phase, Gunther may burn 1 blood to
/// unlock." [LSJ 20011022] "The ability can be used when in torpor":
/// locked in torpor, he unlocks for 1 blood, once in the phase.
#[test]
fn gunther_burns_a_blood_to_unlock_in_the_discard_phase_from_torpor() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Gunther, Beast Lord"), 3, true);
    e.state.minions[g.index()].locked = true;
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let u = vampire_use(&mut e, g, 0);
    assert_eq!(request(&mut e).window, WindowLabel::DiscardPhase);
    pick(&mut e, u);
    assert!(!e.state.minions[g.index()].locked);
    assert!(e.state.minions[g.index()].in_torpor);
    assert_eq!(blood(&e, g), 2);
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, g, 0)
    });
}

// ---- Hasina Kesi ----------------------------------------------------------------

/// Hasina Kesi: "Burn 1 pool each time Hasina goes into torpor." The
/// blocker's strength-2 hand strike sends her 1 blood to torpor: her
/// controller burns 1 pool.
#[test]
fn hasina_kesi_burns_a_pool_going_to_torpor() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hasina Kesi"), 1, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["pot"];
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    hand(&mut e, 1, &["Torn Signpost"]);
    bleed_blocked(&mut e, h, z);
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[h.index()].in_torpor);
    assert_eq!(e.state.seats[0].pool, 10 - 1);
    assert_eq!(e.state.seats[1].pool, 10);
}

// ---- Heather Florent ------------------------------------------------------------

/// Heather Florent of seat 1, in torpor, seat 1 holding the Edge; seat 0
/// passes its turn to seat 1's unlock phase, the Edge's pool taken.
fn heather(answer: Choice) -> (Engine, MinionId) {
    let mut e = game(2);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let h = e.add_minion(
        SeatId(1),
        find_card("Heather Florent, The Opportunist"),
        2,
        true,
    );
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.state.seats[1].has_edge = true;
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, Choice::BurnEdge);
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::ResolutionChoice)
    );
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::ResolvePrinted { minion: h }]
    );
    pick(&mut e, answer);
    request(&mut e);
    assert!(e.state.seats[1].has_edge, "kept");
    (e, h)
}

/// Heather Florent: "If you gain pool from the Edge, you may move it to
/// Heather (instead of putting it in your pool)." [LSJ 20011022] "The
/// ability can be used when in torpor": the Edge's pool to the torpid
/// Heather as blood, the pool unchanged; declined, the pool gains it.
#[test]
fn heather_florent_banks_the_edge_pool_on_herself() {
    let (e, h) = heather(Choice::ResolvePrinted {
        minion: MinionId(1),
    });
    assert_eq!(h, MinionId(1));
    assert_eq!((blood(&e, h), e.state.seats[1].pool), (3, 10));
    let (e, h) = heather(Choice::Pass);
    assert_eq!((blood(&e, h), e.state.seats[1].pool), (2, 11));
}

/// Heather Florent: pool gained by burning the Edge (Hartmut Stover's
/// discard-phase burn) is no pool "from the Edge": it goes to the pool, no
/// choice offered.
#[test]
fn heather_florent_takes_no_pool_from_a_burned_edge() {
    let mut e = game(2);
    let hs = e.add_minion(SeatId(0), find_card("Hartmut Stover"), 4, false);
    let h = e.add_minion(
        SeatId(0),
        find_card("Heather Florent, The Opportunist"),
        2,
        false,
    );
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 0), 4, false);
    e.state.seats[0].has_edge = true;
    let u = vampire_use(&mut e, hs, 0);
    pick(&mut e, u);
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::ResolvePrinted { minion: h }));
    assert_eq!((blood(&e, h), e.state.seats[0].pool), (2, 11));
    assert!(!e.state.seats[0].has_edge);
}

// ---- Helena Casimir, Kemintiri --------------------------------------------------

/// Helena Casimir: "Camarilla primogen: +1 bleed." Unblocked, 2 pool.
#[test]
fn helena_casimir_bleeds_for_two() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Helena Casimir"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    pick(&mut e, bleed(h));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
    assert_eq!(e.state.title_of(h), Title::Primogen);
}

/// Kemintiri (advanced): "Red List. +1 stealth." A bleed of stealth 1.
#[test]
fn kemintiri_advanced_gets_stealth_on_every_action() {
    let k = find_card("Kemintiri");
    assert!(CRYPT[k as usize].adv && CRYPT[k as usize].red_list);
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), k, 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    pick(&mut e, bleed(m));
    request(&mut e);
    assert_eq!(e.action().expect("bleed").stealth(&e.state), 1);
}

// ---- Igo the Hungry ----------------------------------------------------------------

/// Igo the Hungry: "Igo does not get the usual +1 stealth when hunting."
/// [PIB 20150820-2] "His ability only applies to the default rulebook hunt
/// action (eg. he gets Loki's Gift +1 stealth)": his rulebook hunt at 0,
/// his Loki's Gift [pro] hunt at 1.
#[test]
fn igo_hunts_at_zero_but_keeps_loki_gifts_stealth() {
    for (card, stealth) in [(None, 0), (Some("Loki's Gift"), 1)] {
        let mut e = game(2);
        let igo = e.add_minion(SeatId(0), find_card("Igo the Hungry"), 1, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        match card {
            Some(c) => {
                hand(&mut e, 0, &[c]);
                pick(&mut e, play(0, 0, igo));
            }
            None => pick(&mut e, action(igo, ActionKind::Hunt)),
        }
        request(&mut e);
        let a = e.action().expect("hunt");
        assert!(a.is_hunt());
        assert_eq!(a.stealth(&e.state), stealth, "{card:?}");
    }
}

// ---- Janey Pickman -------------------------------------------------------------------

/// Janey Pickman: "Once each round, Janey can burn 1 blood to make the
/// damage from her hand strikes aggravated for the current round." [RTR
/// 19960112] "Can be played after both strikes have been chosen": used
/// once both hand strikes are chosen, her strike's damage is aggravated;
/// not offered again that round.
#[test]
fn janey_pickman_aggravates_her_hand_strike_after_strikes_are_chosen() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Janey Pickman"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    hand(&mut e, 1, &["Relentless Reaper"]);
    bleed_blocked(&mut e, j, z);
    let chosen = |e: &Engine| {
        e.combat()
            .is_some_and(|c| c.sides.iter().all(|s| s.strike.is_some()))
    };
    let u = loop {
        let req = request(&mut e);
        match req.options.iter().find(|o| uses_vampire(o, j, 0)) {
            Some(&u) if chosen(&e) => break u,
            _ => e.choose(fallback(&req)),
        }
    };
    pick(&mut e, u);
    assert_eq!(blood(&e, j), 3);
    let req = offering(&mut e, play(0, 0, z));
    assert!(!req.options.iter().any(|o| uses_vampire(o, j, 0)));
    assert_eq!(pending(&e, 1), (0, 1));
}

// ---- Janni --------------------------------------------------------------------------

/// The damage Janni's strike puts on a [for] bleeder holding Relentless
/// Reaper she blocks, read as the bleeder may prevent it: (normal,
/// aggravated); `weapon` on her, its strike `strike` taken if any.
fn janni_strike(weapon: &str, strike: bool) -> (Engine, MinionId) {
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 2), 4, false);
    let j = e.add_minion(SeatId(1), find_card("Janni"), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 0), 4, false);
    let w = in_play(&mut e, 1, weapon, Some(j));
    hand(&mut e, 0, &["Relentless Reaper"]);
    bleed_blocked(&mut e, x, j);
    if strike {
        pick_when(&mut e, card_ability(w, 0));
    }
    (e, x)
}

/// Janni: "Janni inflicts +1 damage when striking with a weapon." Her .44
/// Magnum strikes for 3; her hand strike stays 1 (the weapon on her).
#[test]
fn janni_adds_one_to_a_weapon_strike_only() {
    let (mut e, x) = janni_strike(".44 Magnum", true);
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 0), (3, 0));
    assert_eq!(hand_strike_damage(find_card("Janni")), (1, 0));
}

/// [RTR 19970630] [TOM 19960225] "Additional damage inherits all of the
/// properties of the base damage": Janni's Molotov Cocktail strike, 2R
/// aggravated, deals 3 aggravated.
#[test]
fn janni_adds_aggravated_damage_to_a_molotov_strike() {
    let (mut e, x) = janni_strike("Molotov Cocktail", true);
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 0), (0, 3));
}

/// [RTR 19960221] "Adding damage to a strike that does not deal damage has
/// no effect": her Flash Grenade ends combat, no damage.
#[test]
fn janni_adds_nothing_to_a_flash_grenade() {
    let (mut e, x) = janni_strike("Flash Grenade", true);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
    assert_eq!(blood(&e, x), 4);
    assert!(!e.state.minions[x.index()].in_torpor);
}

/// Janni: "She can enter combat with a Ministry as a Ⓓ action": another
/// Methuselah's Ministry, not a non-Ministry, not her own Methuselah's;
/// blood cursed generated.
#[test]
fn janni_enters_combat_with_a_ministry() {
    let janni = find_card("Janni");
    assert!(CRYPT[janni as usize].blood_cursed);
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), janni, 4, false);
    let ministry = |n| textless(|c| c.clan == "Ministry", n);
    let own = e.add_minion(SeatId(0), ministry(0), 4, false);
    let theirs = e.add_minion(SeatId(1), ministry(1), 4, false);
    let other = e.add_minion(SeatId(1), textless(|c| c.clan == "Brujah", 0), 4, false);
    let req = request(&mut e);
    let at = |m| vampire_action(j, 0, Target::Minion(m));
    assert!(req.options.contains(&at(theirs)));
    assert!(!req.options.contains(&at(other)), "not a Ministry");
    assert!(!req.options.contains(&at(own)), "her own Methuselah's");
    e.notes = Some(Vec::new());
    pick(&mut e, at(theirs));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let begins = noted_combats(&mut e).into_iter().find_map(|n| match n {
        zvtes::combat::CombatNote::Begins { minions } => Some(minions),
        _ => None,
    });
    assert_eq!(begins, Some([j, theirs]));
}

// ---- Jayne Jonestown ------------------------------------------------------------------

/// Jayne Jonestown: "Jayne gets +1 stealth during hunt actions and -1
/// stealth during other actions." A hunt of stealth 2 (the rulebook's 1
/// and his), a bleed of -1. Red List generated.
#[test]
fn jayne_jonestown_hunts_with_stealth_and_acts_without() {
    let jayne = find_card("Jayne Jonestown");
    assert!(CRYPT[jayne as usize].red_list);
    for (kind, stealth) in [(ActionKind::Hunt, 2), (ActionKind::Bleed, -1)] {
        let mut e = game(2);
        let j = e.add_minion(SeatId(0), jayne, 1, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        pick(&mut e, action(j, kind));
        request(&mut e);
        assert_eq!(
            e.action().expect("action").stealth(&e.state),
            stealth,
            "{kind:?}"
        );
    }
}

/// Jayne Jonestown, ready or not, with a Society of Leopold on seat 1's
/// vampire; ready vampires of both seats, a torpid one of seat 1's.
fn jayne(torpor: bool) -> (Engine, [MinionId; 5], CardId) {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jayne Jonestown"), 2, torpor);
    let own = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let held = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let other = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    let torpid = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 3), 4, true);
    let sol = in_play(&mut e, 1, "Society of Leopold", Some(held));
    (e, [j, own, held, other, torpid], sol)
}

/// Jayne Jonestown: "During your unlock phase, if he is ready, you can
/// move the card named Society of Leopold (if any) to a ready vampire."
/// Seat 1's copy (kept by its vampire's blood in seat 1's unlock phase),
/// in his unlock phase: to any ready vampire but its own
/// (Jayne himself included), not a torpid one; moved, the same card in
/// play, its player's still.
#[test]
fn jayne_jonestown_moves_the_society_of_leopold_to_a_ready_vampire() {
    let (mut e, [j, own, held, other, torpid], sol) = jayne(false);
    // Seat 1's unlock phase: its vampire burns 1 blood to keep it.
    pick_when(&mut e, Choice::BurnBloodToAvoid);
    let u = vampire_use(&mut e, j, 0);
    assert_eq!(e.state.active, SeatId(0));
    assert_eq!(request(&mut e).window, WindowLabel::UnlockAbilities);
    assert!(matches!(u, Choice::UseAbility { target: Target::Card(c), .. } if c == sol));
    pick(&mut e, u);
    let req = request(&mut e);
    let to = |m| Choice::ChooseMinion(m);
    assert_eq!(req.options, vec![to(j), to(own), to(other)]);
    assert!(!req.options.contains(&to(held)) && !req.options.contains(&to(torpid)));
    pick(&mut e, to(other));
    let c = &e.state.cards[sol.index()];
    assert!(c.present() && c.host == Some(other));
    assert_eq!(e.state.card_controller(sol), SeatId(1));
    assert!(e.state.seats[1].ash.is_empty());
}

/// Jayne Jonestown in torpor: not offered.
#[test]
fn jayne_jonestown_moves_nothing_from_torpor() {
    let (mut e, [j, ..], _) = jayne(true);
    pick(&mut e, Choice::Pass);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, j, 0)
    });
    assert_eq!(e.state.active, SeatId(0));
}

// ---- Jazz Wentworth -----------------------------------------------------------------

/// Jazz Wentworth: "If the Edge is not controlled, Jazz may give you
/// control of it as a +1 stealth action." Nobody holding it: an action of
/// stealth 1, undirected; his Methuselah gets the Edge.
#[test]
fn jazz_wentworth_takes_the_uncontrolled_edge() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jazz Wentworth"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    pick(&mut e, vampire_action(j, 0, Target::None));
    request(&mut e);
    let a = e.action().expect("action");
    assert_eq!((a.stealth(&e.state), a.directed_seat(&e.state)), (1, None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].has_edge);
    assert!(!e.state.seats[1].has_edge);
}

/// Jazz Wentworth: the Edge held, by anyone: not offered.
#[test]
fn jazz_wentworth_is_not_offered_while_the_edge_is_held() {
    for holder in [0, 1] {
        let mut e = game(2);
        let j = e.add_minion(SeatId(0), find_card("Jazz Wentworth"), 4, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        e.state.seats[holder].has_edge = true;
        let req = request(&mut e);
        assert!(
            !req.options.contains(&vampire_action(j, 0, Target::None)),
            "seat {holder}"
        );
    }
}

// ---- Justine --------------------------------------------------------------------------

/// Justine: "Justine does not lock when she successfully blocks a vampire
/// with a capacity below 4." Blocking a capacity-3 vampire she stays
/// unlocked; a capacity-4 one, she locks.
#[test]
fn justine_stays_unlocked_blocking_a_small_vampire() {
    for (capacity, locked) in [(3, false), (4, true)] {
        let mut e = game(2);
        let x = e.add_minion(SeatId(0), textless(|c| c.capacity == capacity, 0), 3, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        let j = e.add_minion(SeatId(1), find_card("Justine, Elder of Dallas"), 6, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        bleed_blocked(&mut e, x, j);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.minions[j.index()].locked,
            locked,
            "capacity {capacity}"
        );
    }
}

// ---- Lazarus -------------------------------------------------------------------------

/// Lazarus, the base card or the advanced one.
fn lazarus(adv: bool) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == "Lazarus" && c.adv == adv)
        .expect("Lazarus") as u16
}

/// Lazarus: "Lazarus may inflict 1R damage as a strike." Maneuvered to
/// long range, his strike hits the blocker for 1, which it mends.
#[test]
fn lazarus_strikes_for_one_at_long_range() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), lazarus(false), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    hand(&mut e, 0, &["Fake Out"]);
    bleed_blocked(&mut e, l, x);
    pick_when(&mut e, play(0, 0, l));
    let u = vampire_use(&mut e, l, 0);
    assert_eq!(
        e.combat().expect("combat").range,
        zvtes::combat::Range::Long
    );
    pick(&mut e, u);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, x), blood(&e, l)), (3, 4));
}

/// Merged Lazarus: "[MERGED] Once each combat, Lazarus may burn 1 blood
/// to get one press, only usable to continue combat." [PIB 20150121] "His
/// ability to get a press can only be used during the press step": in the
/// press step, 1 blood, another round; not offered in the second round's.
#[test]
fn lazarus_merged_burns_a_blood_to_press_on() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), lazarus(true), 6, false);
    e.state.minions[l.index()].merged = Some(lazarus(false));
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 6, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    bleed_blocked(&mut e, l, x);
    let u = vampire_use(&mut e, l, 2);
    assert_eq!(request(&mut e).window, WindowLabel::CombatPress);
    let before = blood(&e, l);
    pick(&mut e, u);
    assert_eq!(blood(&e, l), before - 1);
    let mut rounds = vec![];
    loop {
        let req = request(&mut e);
        if let Some(c) = e.combat() {
            rounds.push(c.round);
        }
        assert!(!req.options.iter().any(|o| uses_vampire(o, l, 2)));
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(rounds.iter().max(), Some(&2), "a second round");
}

// ---- Lena Rowe -------------------------------------------------------------------------

/// Lena Rowe: "Lena cannot receive any title." Crusade: Chicago, a title
/// put on its acting vampire, is not offered to her, to another Sabbat
/// vampire it is.
#[test]
fn lena_rowe_cannot_be_given_a_title() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lena Rowe"), 3, false);
    let o = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(zvtes::crypt::Sect::Sabbat), 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    hand(&mut e, 0, &["Crusade: Chicago"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, l)));
    assert!(req.options.contains(&play(0, 0, o)));
    assert!(
        !e.state.may_gain_title(l, Title::Baron("Seattle")),
        "a baron's"
    );
}

// ---- Tereza Rostas ---------------------------------------------------------------------

/// Tereza Rostas at seat 0, 4 blood, the Edge with `edge`'s seat.
fn tereza(edge: Option<usize>) -> (Engine, MinionId) {
    let mut e = game(3);
    let t = e.add_minion(SeatId(0), find_card("Tereza Rostas"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(2), textless(|c| c.capacity >= 4, 2), 4, false);
    if let Some(s) = edge {
        e.state.seats[s].has_edge = true;
    }
    (e, t)
}

/// Tereza Rostas: "Tereza can steal the Edge as a Ⓓ action that costs 2
/// blood." [TOM 19951214-2] "She cannot steal the Edge if it is
/// uncontrolled": not offered then, nor while her Methuselah holds it;
/// held by seat 2, at seat 2, directed; she pays 2 and her Methuselah has
/// it.
#[test]
fn tereza_rostas_steals_the_edge() {
    for edge in [None, Some(0)] {
        let (mut e, t) = tereza(edge);
        let req = request(&mut e);
        assert!(
            !req.options.iter().any(|o| matches!(o,
                Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == t)),
            "{edge:?}"
        );
    }
    let (mut e, t) = tereza(Some(2));
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&vampire_action(t, 0, Target::Seat(SeatId(1)))));
    pick(&mut e, vampire_action(t, 0, Target::Seat(SeatId(2))));
    request(&mut e);
    assert_eq!(
        e.action().expect("action").directed_seat(&e.state),
        Some(SeatId(2))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].has_edge && !e.state.seats[2].has_edge);
    assert_eq!(blood(&e, t), 2);
}

/// [RTR 19980623] "If the target of the action burns the Edge during the
/// action, her controller still gets the Edge if the action succeeds"
/// (rules-qa 90.7): seat 2 burns it with Hand Intervention's ability
/// during the action; her Methuselah still gets it.
#[test]
fn tereza_rostas_steal_survives_the_edge_burned() {
    let (mut e, t) = tereza(Some(2));
    let k = in_play(&mut e, 2, "Hand Intervention", None);
    e.state.cards[k.index()].against = zvtes::ids::SeatSet::single(SeatId(2));
    pick(&mut e, vampire_action(t, 0, Target::Seat(SeatId(2))));
    let burn = card_ability(k, 0);
    let req = offering(&mut e, burn);
    assert_eq!(req.seat, SeatId(2));
    assert!(e.action().is_some(), "during the action");
    pick(&mut e, burn);
    request(&mut e);
    assert!(!e.state.seats[2].has_edge, "burned");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].has_edge);
    assert_eq!(blood(&e, t), 2);
}

// ---- Round 33, track C, vamp_e --------------------------------------------------

/// The crypt card `name`, the advanced one or the other.
fn card_adv(name: &str, adv: bool) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && c.adv == adv)
        .unwrap_or_else(|| panic!("no {name}")) as u16
}

/// A fixture vampire of capacity 4 or more, the `nth`.
fn plain4(nth: usize) -> u16 {
    textless(|c| c.capacity >= 4, nth)
}

/// Lupo ("Camarilla."), Marion French ("Independent."), Maria Stone G3
/// ("Sabbat."): no definition. Luccia Paciola ("+1 strength.") strikes for
/// 2; Lydia Van Cuelen ("+1 bleed.") bleeds for 2; Lucretia, Cess Queen
/// ("+1 stealth.") bleeds at stealth 1. Titles, votes, scarce, sterile and
/// infernal generated: Lucian a Justicar, Lucretia, Marty and Masika
/// primogens, Mata Hari 2 votes and Red List, Maureen 1 vote and infernal,
/// Lorrie scarce and sterile. [LSJ 20011015] [LSJ 20010714] (personal
/// titles: generated, kept whatever the sect).
#[test]
fn the_round_33_e_vampires_carry_their_printed_traits() {
    for name in ["Lupo", "Marion French"] {
        assert!(zvtes::cards::vampire(find_card(name)).is_none(), "{name}");
    }
    assert!(zvtes::cards::vampire(card_adv("Maria Stone", false)).is_none());
    let c = |n: &str| &CRYPT[find_card(n) as usize];
    assert_eq!(c("Lucian").title, Title::Justicar("Malkavian"));
    for n in ["Lucretia, Cess Queen", "Marty Lechtansi", "Miranda Sanova"] {
        assert_eq!(c(n).title, Title::Primogen, "{n}");
    }
    assert_eq!(
        CRYPT[card_adv("Masika", false) as usize].title,
        Title::Primogen
    );
    assert_eq!(c("Mata Hari").title, Title::Votes(2));
    assert!(c("Mata Hari").red_list);
    assert_eq!(c("Maureen, Dark Priestess").title, Title::Votes(1));
    assert!(c("Maureen, Dark Priestess").infernal);
    assert!(c("Lorrie Dunsirn").scarce && c("Lorrie Dunsirn").sterile);
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Maureen, Dark Priestess"), 4, false);
    e.state.minions[m.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    assert_eq!(
        e.state.title_of(m),
        Title::Votes(1),
        "kept through a sect change"
    );
    assert_eq!(hand_strike_damage(find_card("Luccia Paciola")), (2, 0));
    for (name, bleed_n, stealth) in [("Lydia Van Cuelen", 2, 0), ("Lucretia, Cess Queen", 1, 1)] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card(name), 4, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        pick(&mut e, bleed(m));
        request(&mut e);
        assert_eq!(
            e.action().expect("bleed").stealth(&e.state),
            stealth,
            "{name}"
        );
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10 - bleed_n, "{name}");
    }
}

// ---- Leon, Miranda Sanova --------------------------------------------------------

/// Leon: "Gangrel antitribu get -1 intercept when attempting to block
/// Leon." His stealth-0 bleed: a Gangrel antitribu's block fails (intercept
/// -1), a Ventrue's succeeds. Miranda Sanova's Brujah likewise, Miranda
/// in torpor leaving it (stealth 1), each blocker playing Enhanced Senses
/// [aus] (+1 intercept). [LSJ 20011022] "The passive ability applies even
/// if the vampire is in torpor."
#[test]
fn leon_and_miranda_lower_their_clans_blockers_intercept() {
    for (name, clan, torpor) in [
        ("Leon", "Gangrel antitribu", false),
        ("Miranda Sanova", "Brujah", true),
    ] {
        for (blocker, blocked) in [(clan, false), ("Ventrue", true)] {
            let mut e = game(2);
            let l = e.add_minion(SeatId(0), find_card(name), 3, torpor);
            e.add_minion(SeatId(0), plain4(0), 4, false);
            let z = e.add_minion(SeatId(1), plain(blocker, 0), 4, false);
            e.add_minion(SeatId(1), plain4(1), 4, false);
            let act = if torpor {
                e.state.minions[z.index()].gained_levels = vec!["aus"];
                hand(&mut e, 1, &["Enhanced Senses"]);
                action(l, ActionKind::LeaveTorpor)
            } else {
                bleed(l)
            };
            pick(&mut e, act);
            pick_when(&mut e, block(z));
            if torpor {
                pick_when(&mut e, play(0, 0, z));
            }
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            let done = if torpor {
                !e.state.minions[l.index()].in_torpor
            } else {
                e.state.seats[1].pool < 10
            };
            assert_eq!(done, !blocked, "{name} against {blocker}");
        }
    }
}

// ---- Lorrie Dunsirn --------------------------------------------------------------

/// Lorrie Dunsirn (4 blood) of seat 0 and a vampire beside her; seat 1's
/// blocker `z` with `zb` blood and another vampire.
fn lorrie(zb: u8) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lorrie Dunsirn"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 9, 0), zb, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    (e, l, z)
}

/// Lorrie Dunsirn: "She gets an additional strike (limited) each round and
/// a press each combat. … +1 strength." [LSJ 20051016] [LSJ 20051017] "Her
/// press and additional strike are mandatory." Blocked, each round she
/// strikes twice for 2, never offered to pass her additional strike; her
/// press continues the combat once, never offered to end it, and no
/// maneuver to long is offered her: the blocker takes 8 over two rounds.
#[test]
fn lorrie_dunsirn_strikes_twice_and_must_press_to_continue() {
    use zvtes::combat::CombatNote;
    let (mut e, l, z) = lorrie(9);
    hand(&mut e, 0, &["Fake Out"]);
    e.notes = Some(Vec::new());
    bleed_blocked(&mut e, l, z);
    loop {
        let req = request(&mut e);
        if req.seat == SeatId(0) {
            assert!(
                !req.options.contains(&play(0, 0, l)),
                "no maneuver to long: {:?}",
                req.window
            );
            if req.window == WindowLabel::CombatStrike {
                assert!(!req.options.contains(&Choice::Pass), "{:?}", req.options);
            }
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    let pairs: Vec<(u8, u8)> = noted_combats(&mut e)
        .into_iter()
        .filter_map(|n| match n {
            CombatNote::Strikes {
                round,
                pair,
                strike_damage,
                ..
            } if strike_damage[0] == 2 => Some((round, pair)),
            _ => None,
        })
        .collect();
    assert_eq!(pairs, vec![(1, 0), (1, 1), (2, 0), (2, 1)]);
    assert_eq!(blood(&e, z), 9 - 8);
}

/// Lorrie Dunsirn: "Non-hunt actions cost her an additional pool." Her
/// bleed costs her Methuselah 1 pool as it succeeds; blocked, nothing; her
/// hunt nothing.
#[test]
fn lorrie_dunsirn_pays_a_pool_for_non_hunt_actions() {
    let (mut e, l, _) = lorrie(4);
    pick(&mut e, bleed(l));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10 - 1, 10 - 1]);
    let (mut e, l, z) = lorrie(9);
    bleed_blocked(&mut e, l, z);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10);
    let (mut e, l, _) = lorrie(4);
    e.state.minions[l.index()].blood = 1;
    pick(&mut e, action(l, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10);
    assert_eq!(blood(&e, l), 2);
}

/// Lorrie Dunsirn: "Lorrie cannot play reaction cards, have equipment or
/// retainers." [LSJ 20050315] [LSJ 20010210] "She cannot attempt an action
/// to equip or employ, nor strike to steal an equipment or retainer." No
/// .44 Magnum equip offered her; locked, Forced Awakening offered the other
/// locked vampire only; blocked by a Magnum's bearer, Fast Hands [cel]
/// (steal weapon) never offered her.
#[test]
fn lorrie_dunsirn_cannot_equip_employ_or_react() {
    let (mut e, l, z) = lorrie(4);
    let o = MinionId(1);
    hand(&mut e, 0, &[".44 Magnum", "Forced Awakening"]);
    let by = |m: MinionId, card: u8| move |o: &Choice| matches!(o, Choice::Play { card: c, actor: Some(a), .. } if *c == card && *a == m);
    let req = request(&mut e);
    assert!(req.options.iter().any(by(o, 0)), "the other may equip");
    assert!(!req.options.iter().any(by(l, 0)));
    e.state.minions[l.index()].locked = true;
    e.state.minions[o.index()].locked = true;
    pick(&mut e, Choice::Pass);
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, bleed(z));
    let req = offering(&mut e, play(1, 0, o));
    assert!(!req.options.iter().any(by(l, 1)));

    let (mut e, l, z) = lorrie(4);
    in_play(&mut e, 1, ".44 Magnum", Some(z));
    e.state.minions[l.index()].gained_levels = vec!["cel"];
    hand(&mut e, 0, &["Fast Hands"]);
    bleed_blocked(&mut e, l, z);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, by(l, 0));
}

/// Lorrie Dunsirn: [LSJ 20050315] "She can equip with an equipment that is
/// a location when in play": Vast Wealth's action puts Palatial Estate on
/// her (Helen Fairchild's case).
#[test]
fn lorrie_dunsirn_equips_a_location_through_vast_wealth() {
    let (mut e, l, _) = lorrie(4);
    let wealth = in_play(&mut e, 0, "Vast Wealth", Some(l));
    hand(&mut e, 0, &["Palatial Estate"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Palatial Estate")].into();
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(wealth),
            ability: 0,
            actor: Some(l),
            target: Target::None,
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let estate = find_library("Palatial Estate");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == estate && c.host == Some(l)));
}

/// Seats run with the fallback until seat 0 is ousted (or the game ends).
fn until_seat0_ousted(e: &mut Engine) {
    for _ in 0..300 {
        match e.run() {
            zvtes::game::Step::Over(_) => return,
            zvtes::game::Step::Decision(_) if e.state.seats[0].ousted => return,
            zvtes::game::Step::Decision(req) => e.choose(fallback(&req)),
        }
    }
    panic!("seat 0 never ousted");
}

/// Lorrie Dunsirn: "Non-hunt actions cost her an additional pool." Her cost
/// and the bleed are simultaneous (rulebook 6.2.3; PIB 20130626-27,
/// https://usenet.krcg.org/t/vekn-50461/#m12, #m24, #m28: "You
/// simultaneously are ousted and oust your prey."; rules-qa 45.1): bleeding
/// for 1 with seat 0 and its prey both on 1 pool, both are ousted in one
/// batch — seat 0 scores its prey's victory point, no 6 pool; seat 3, its
/// predator, scores and takes 6.
#[test]
fn lorrie_dunsirn_bleed_ousts_both_at_once() {
    let mut e = game(4);
    let l = e.add_minion(SeatId(0), find_card("Lorrie Dunsirn"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    for s in 1..4 {
        e.add_minion(SeatId(s), plain4(s as usize), 4, false);
    }
    e.state.seats[0].pool = 1;
    e.state.seats[1].pool = 1;
    pick(&mut e, bleed(l));
    until_seat0_ousted(&mut e);
    assert!(e.state.seats[0].ousted, "her Methuselah, by the cost");
    assert!(e.state.seats[1].ousted, "the prey, by the bleed");
    assert_eq!(e.state.seats[0].half_vps, 2, "the prey's victory point");
    assert_eq!(e.state.seats[3].half_vps, 2);
    assert_eq!(e.state.seats[3].pool, 10 + 6);
    assert_eq!(e.state.seats[2].pool, 10);
}

/// Lorrie Dunsirn's pool on a fizzle for a cost no longer payable (rules-qa
/// 117.4: "on 3/ they are paid first … each as far as it goes"): no effect
/// follows, the last pool still ousts. Her Bum's Rush at François "Warden"
/// Loehr (1 blood more), her blood gone by resolution: seat 0 ousted, no
/// combat.
#[test]
fn lorrie_dunsirn_last_pool_on_a_fizzle_still_ousts() {
    let mut e = game(3);
    let l = e.add_minion(SeatId(0), find_card("Lorrie Dunsirn"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let f = e.add_minion(SeatId(1), find_card("François \"Warden\" Loehr"), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(2), plain4(2), 4, false);
    e.state.seats[0].pool = 1;
    hand(&mut e, 0, &["Bum's Rush"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(l),
            target: Target::Minion(f),
        },
    );
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    e.state.minions[l.index()].blood = 0;
    pick(&mut e, Choice::Pass);
    until_seat0_ousted(&mut e);
    assert!(e.state.seats[0].ousted);
    assert!(e.combat().is_none());
    assert_eq!(blood(&e, f), 4, "no combat");
}

/// Lorrie Dunsirn's last pool paid for an action whose effect gains pool
/// (rules-qa 45.1: "a pool the effect gains counting first"; PIB
/// 20130627, https://usenet.krcg.org/t/vekn-50461/#m25): Social Charm
/// [PRE] at 1 pool, the bleed successful: her cost 1, the pool gained 1,
/// seat 0 left on 1, not ousted.
#[test]
fn lorrie_dunsirn_last_pool_survives_a_pool_her_effect_gains() {
    let mut e = game(3);
    let l = e.add_minion(SeatId(0), find_card("Lorrie Dunsirn"), 4, false);
    e.state.minions[l.index()].gained_levels = vec!["pre", "pre"];
    for s in 0..3 {
        e.add_minion(SeatId(s), plain4(s as usize), 4, false);
    }
    e.state.seats[0].pool = 1;
    hand(&mut e, 0, &["Social Charm"]);
    pick(&mut e, play(0, 1, l));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.seats[0].ousted);
    assert_eq!(pool(&e), vec![1, 10 - 2, 10]);
}

/// Lorrie Dunsirn's last pool paid for a political action (rules-qa 45.1:
/// "the effect being to … call a referendum, the payer is ousted before
/// it"; vekn-50461 #m24): Kine Resources Contested at 1 pool ousts seat 0
/// at resolution, no referendum ever called.
#[test]
fn lorrie_dunsirn_last_pool_on_a_political_action_ousts_before_its_referendum() {
    let mut e = game(3);
    let l = e.add_minion(SeatId(0), find_card("Lorrie Dunsirn"), 4, false);
    for s in 0..3 {
        e.add_minion(SeatId(s), plain4(s as usize), 4, false);
    }
    e.state.seats[0].pool = 1;
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, l));
    for _ in 0..300 {
        assert!(e.referendum().is_none(), "no referendum called");
        match e.run() {
            zvtes::game::Step::Decision(_) if e.state.seats[0].ousted => break,
            zvtes::game::Step::Decision(req) => e.choose(fallback(&req)),
            zvtes::game::Step::Over(_) => panic!("game over"),
        }
    }
    assert!(e.state.seats[0].ousted);
}

// ---- Magdelena Schaefer, Mitru the Hunter ---------------------------------------

/// Magdelena Schaefer: "Strike cards that require Blood Sorcery [tha] cost
/// Magdalena an additional blood." Bleeding, blocked, she plays Theft of
/// Vitae [tha] as her strike: 1 blood, where a [tha] vampire without her
/// text pays none.
#[test]
fn magdelena_pays_a_blood_more_for_blood_sorcery_strikes() {
    let magdelena = find_card("Magdelena Schaefer");
    for (card, paid) in [(magdelena, 1), (vampire_with("THA", 0), 0)] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), card, 2, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let z = e.add_minion(SeatId(1), plain4(1), 4, false);
        e.state.minions[z.index()].gained_levels = vec!["pot"];
        e.add_minion(SeatId(1), plain4(2), 4, false);
        hand(&mut e, 0, &["Theft of Vitae"]);
        hand(&mut e, 1, &["Undead Strength"]);
        bleed_blocked(&mut e, m, z);
        pick_when(&mut e, play(0, 0, m));
        let req = request(&mut e);
        assert_eq!(
            (req.seat, req.window),
            (SeatId(1), WindowLabel::CombatStrike)
        );
        assert_eq!(blood(&e, m), 2 - paid, "{}", CRYPT[card as usize].name);
    }
}

/// Mitru the Hunter: "Cards that require Celerity [cel] cost Mitru 1 less
/// blood." Fleetness [cel] (1 blood) costs him none; Govern the Unaligned
/// [dom] its blood.
#[test]
fn mitru_pays_a_blood_less_for_celerity_cards() {
    for (card, mode, cost) in [("Fleetness", 0, 0), ("Govern the Unaligned", 0, 1)] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card("Mitru the Hunter"), 4, false);
        e.state.minions[m.index()].gained_levels = vec!["dom"];
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        hand(&mut e, 0, &[card]);
        pick(&mut e, play(0, mode, m));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, m), 4 - cost, "{card}");
        assert!(e.state.seats[1].pool < 10, "{card} bled");
    }
}

// ---- Marty Lechtansi --------------------------------------------------------------

/// Marty Lechtansi: "Once each combat, Marty may burn 1 blood to get one
/// press." [PIB 20150121] "His ability to get a press can only be used
/// during the press step": in the press step, 1 blood, a second round; not
/// offered again that combat.
#[test]
fn marty_lechtansi_buys_one_press_a_combat() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Marty Lechtansi"), 6, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 6, 0), 6, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    e.notes = Some(Vec::new());
    bleed_blocked(&mut e, m, x);
    let u = vampire_use(&mut e, m, 0);
    assert_eq!(request(&mut e).window, WindowLabel::CombatPress);
    let before = blood(&e, m);
    pick(&mut e, u);
    assert_eq!(blood(&e, m), before - 1);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, m, 0)
    });
    let rounds: Vec<u8> = noted_combats(&mut e)
        .into_iter()
        .filter_map(|n| match n {
            zvtes::combat::CombatNote::Strikes { round, .. } => Some(round),
            _ => None,
        })
        .collect();
    assert_eq!(rounds, vec![1, 2], "a second round, no third");
}

// ---- Masika, Maria Stone ------------------------------------------------------------

/// Masika (base): "During any Methuselah's discard phase, Masika may
/// unlock. +1 bleed." [LSJ 20011022] "The ability can be used when in
/// torpor." Locked in torpor, she unlocks in her own discard phase, once;
/// locked again, in the next Methuselah's discard phase too.
#[test]
fn masika_unlocks_in_any_discard_phase() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), card_adv("Masika", false), 4, true);
    e.state.minions[m.index()].locked = true;
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let u = vampire_use(&mut e, m, 0);
    assert_eq!(
        (request(&mut e).seat, request(&mut e).window),
        (SeatId(0), WindowLabel::DiscardPhase)
    );
    pick(&mut e, u);
    assert!(!e.state.minions[m.index()].locked);
    assert!(e.state.minions[m.index()].in_torpor);
    e.state.minions[m.index()].locked = true;
    let req = request(&mut e);
    assert!(!req.options.contains(&u), "once in the phase");
    pick(&mut e, Choice::Pass);
    let u = vampire_use(&mut e, m, 0);
    assert_eq!(e.state.active, SeatId(1));
    pick(&mut e, u);
    assert!(!e.state.minions[m.index()].locked);
}

/// Masika merged: her base card's +1 bleed and discard-phase unlock, the
/// advanced card's discard for intercept, and the [MERGED] title, Prince of
/// Lisbon.
#[test]
fn masika_merged_keeps_both_texts_and_is_prince_of_lisbon() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), card_adv("Masika", true), 4, false);
    e.state.minions[m.index()].merged = Some(card_adv("Masika", false));
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    assert_eq!(e.state.title_of(m), Title::Prince("Lisbon"));
    let d = zvtes::cards::vampire_of(&e.state.minions[m.index()]).expect("merged");
    assert_eq!(d.abilities.len(), 2);
    pick(&mut e, bleed(m));
    until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
    assert!(e.state.minions[m.index()].locked);
    let u = vampire_use(&mut e, m, 0);
    pick(&mut e, u);
    assert!(!e.state.minions[m.index()].locked);
}

/// Maria Stone merged: "[MERGED] Maria doesn't suffer -1 strength from her
/// special text, and she has superior Spiritus [SPI]." Her hand strike 1
/// (unmerged 0), Spiritus at 2 (unmerged 1).
#[test]
fn maria_stone_merged_strikes_unweakened_with_superior_spiritus() {
    let adv = card_adv("Maria Stone", true);
    for merged in [false, true] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), adv, 4, false);
        if merged {
            e.state.minions[m.index()].merged = Some(card_adv("Maria Stone", false));
        }
        assert_eq!(
            e.state.discipline_level(m, "spi"),
            1 + u8::from(merged),
            "{merged}"
        );
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let z = e.add_minion(SeatId(1), plain4(1), 4, false);
        pick(&mut e, bleed(m));
        pick_when(&mut e, block(z));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, z), 4 - u8::from(merged), "{merged}");
    }
}

// ---- Merrill Molitor --------------------------------------------------------------

/// Merrill Molitor (4 blood) bleeds, blocked by a [pro] vampire holding
/// Wolf Claws and `aim` (an aim card) in hand; Wolf Claws and the aim
/// played: Merrill offered his ability in his prevention step.
fn merrill(aim: Option<&str>) -> (Engine, MinionId) {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Merrill Molitor"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let z = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["pro"];
    e.add_minion(SeatId(1), plain4(2), 4, false);
    hand(&mut e, 1, &["Wolf Claws"]);
    if let Some(a) = aim {
        hand(&mut e, 1, &[a]);
    }
    bleed_blocked(&mut e, m, z);
    pick_when(&mut e, play(0, 0, z));
    if aim.is_some() {
        pick_when(&mut e, play(0, 0, z));
    }
    let u = vampire_use(&mut e, m, 0);
    pick(&mut e, u);
    (e, m)
}

/// Merrill Molitor: "Once each combat, Merrill may treat 1 aggravated
/// damage inflicted on him as normal damage." [LSJ 20030213] "The damage
/// he treats as normal is handled before any aggravated damage": Wolf
/// Claws' aggravated hand strike treated as normal, 1 blood burned, no
/// torpor; not offered again that combat.
#[test]
fn merrill_molitor_treats_one_aggravated_damage_as_normal_once_a_combat() {
    let (mut e, m) = merrill(None);
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, m, 0)
    });
    let v = &e.state.minions[m.index()];
    assert!(!v.in_torpor, "treated as normal");
    assert_eq!(v.blood, 3);
}

/// Merrill Molitor: [LSJ 20100527] "If an aggravated damage is inflicted
/// with Target Vitals, the additional damages are still aggravated": his
/// one point treated as normal (1 blood), Target Vitals' 2 more still
/// aggravated: to torpor, the second burning a blood (rulebook
/// "Aggravated damage").
#[test]
fn merrill_molitor_converts_one_of_target_vitals_aggravated_damage() {
    let (mut e, m) = merrill(Some("Target Vitals"));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let v = &e.state.minions[m.index()];
    assert!(v.in_torpor, "aggravated left");
    assert_eq!(v.blood, 4 - 1 - 1);
}

/// Merrill Molitor: "once each combat" frames his ability to combat
/// (Konrad Fleischer's "once each round" reaches out of it, rules-qa
/// 122.96): Rutor's Hand's aggravated point out of combat offers him no
/// choice; he goes to torpor.
#[test]
fn merrill_molitor_converts_no_damage_out_of_combat() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Merrill Molitor"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    hand(&mut e, 0, &["Rutor's Hand"]);
    pick(&mut e, play(0, 0, m));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        Choice::ResolvePrinted { minion: m },
    );
    assert!(e.state.minions[m.index()].in_torpor);
}

// ---- Maureen, Melissa Barton, Michael diCarlo ----------------------------------------

/// Maureen, Dark Priestess: "Once each turn, you can gain 1 pool after
/// Maureen performs a successful action requiring a Baali or Daimoinon
/// [dai]." Greater Curse [dai] bleeds: after it resolves, her Methuselah
/// gains 1 (and the card's 1); unlocked, her Contagion [dai] action is not
/// offered it that turn. [ANK 20180719-4] "Once each turn" means once
/// during each player's turn. A plain bleed never offers it.
#[test]
fn maureen_gains_a_pool_after_a_daimoinon_action_once_a_turn() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Maureen, Dark Priestess"), 6, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    hand(&mut e, 0, &["Greater Curse", "Contagion"]);
    pick(&mut e, play(0, 0, m));
    let u = vampire_use(&mut e, m, 0);
    assert_eq!(request(&mut e).window, WindowLabel::ActionAfterResolution);
    let before = e.state.seats[0].pool;
    pick(&mut e, u);
    assert_eq!(e.state.seats[0].pool, before + 1);
    assert_eq!(e.state.seats[1].pool, 10 - 1);
    e.state.minions[m.index()].locked = false;
    pick_when(&mut e, play(0, 0, m));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, m, 0)
    });
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Maureen, Dark Priestess"), 6, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    pick(&mut e, bleed(m));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, m, 0)
    });
}

/// Melissa Barton: "If any Methuselah burns the Edge for a vote, Melissa
/// gains 1 blood." [LSJ 20011022] "The passive ability applies even if the
/// vampire is in torpor." Torpid Melissa of seat 1: seat 0 burns the Edge
/// for a vote, she gains 1; another vote (the card's ballot) gives nothing.
#[test]
fn melissa_barton_gains_blood_when_the_edge_buys_a_vote() {
    for edge in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let mb = e.add_minion(SeatId(1), find_card("Melissa Barton"), 2, true);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        e.state.seats[0].has_edge = true;
        hand(&mut e, 0, &["Rumors of Gehenna"]);
        pick(&mut e, play(0, 0, a));
        pick_when(&mut e, Choice::ChooseSeat(SeatId(1)));
        pick_when(&mut e, Choice::Pass);
        to_referendum(&mut e);
        if edge {
            pick_when(&mut e, Choice::CastEdge { favor: true });
        } else {
            pick_when(&mut e, Choice::CastCardVote { favor: true });
        }
        assert_eq!(blood(&e, mb), 2 + u8::from(edge), "{edge}");
    }
}

/// Michael diCarlo: "If Michael performs a successful action to enter
/// combat with a minion, he can set the range for the first round of the
/// resulting combat to long." [RTR 19970630] [ANK 20180720] "When setting
/// the range for the round, no other effect can be used to reset the
/// range, but other pre-range effects can be played afterwards." Fleetness
/// [CEL] at a locked minion: before range he sets long, the step skipped
/// (his Fake Out never offered), his opponent's pre-range Dam the Heart's
/// River still played. Blocked, the block's combat never offers it.
#[test]
fn michael_dicarlo_sets_his_entered_combat_to_long() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Michael diCarlo"), 5, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.state.minions[x.index()].locked = true;
    e.state.minions[x.index()].gained_levels = vec!["qui"];
    hand(&mut e, 0, &["Fleetness", "Fake Out"]);
    hand(&mut e, 1, &["Dam the Heart's River"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 1,
            actor: Some(m),
            target: Target::Minion(x),
        },
    );
    let u = vampire_use(&mut e, m, 0);
    pick(&mut e, u);
    assert_eq!(
        e.combat().expect("combat").range,
        zvtes::combat::Range::Long
    );
    pick_when(&mut e, play(0, 0, x));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, m));

    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Michael diCarlo"), 5, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let z = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    hand(&mut e, 0, &["Fake Out"]);
    bleed_blocked(&mut e, m, z);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, m, 0)
    });
}

// ---- Louhi -------------------------------------------------------------------------

/// Three seats; Louhi (4 blood) for `seat`, a plain vampire each.
fn louhi(seat: u8) -> (Engine, MinionId, [MinionId; 3]) {
    let mut e = game(3);
    let l = e.add_minion(SeatId(seat), find_card("Louhi"), 4, false);
    let v = [0, 1, 2].map(|s| {
        let m = e.add_minion(SeatId(s), plain4(s as usize), 4, false);
        e.state.minions[m.index()].gained_levels = vec!["dom"];
        m
    });
    (e, l, v)
}

/// Louhi: "Once each turn, if Louhi is ready, she can burn 1 blood to
/// cancel an action card as it is played by a vampire controlled by your
/// predator (the acting minion does not lock)." Seat 0, her predator,
/// plays Govern the Unaligned: she cancels it for 1 blood, the card in the
/// ash heap, nothing paid, its vampire unlocked; it plays the second copy,
/// not offered her again. [ANK 20180719-4] once during each player's turn.
/// [RBK cancel-a-card] "If an action is canceled, it is not performed and
/// the minion is free to attempt the same action again." [ANK 20200813-2]
/// all action cards.
#[test]
fn louhi_cancels_her_predators_action_card_once_a_turn() {
    let (mut e, l, v) = louhi(1);
    hand(&mut e, 0, &["Govern the Unaligned", "Govern the Unaligned"]);
    pick(&mut e, play(0, 0, v[0]));
    let u = vampire_use(&mut e, l, 0);
    assert_eq!(request(&mut e).window, WindowLabel::CardAsPlayed);
    pick(&mut e, u);
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::MinionPhase)
    );
    assert_eq!(blood(&e, l), 3);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Govern the Unaligned")));
    assert!(!e.state.minions[v[0].index()].locked);
    assert_eq!((blood(&e, v[0]), e.state.seats[1].pool), (4, 10));
    pick(&mut e, play(0, 0, v[0]));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, l, 0)
    });
    assert_eq!(e.state.seats[1].pool, 10 - 3, "bled");
}

/// Louhi: her prey's vampire's action card is not hers to cancel.
#[test]
fn louhi_cancels_nothing_of_her_prey() {
    let (mut e, l, v) = louhi(0);
    hand(&mut e, 1, &["Govern the Unaligned"]);
    pick(&mut e, Choice::Pass);
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, play(0, 0, v[1]));
    never_offered_where(&mut e, SeatId(2), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, l, 0)
    });
    assert_eq!(e.state.seats[2].pool, 10 - 3);
}

/// Louhi: [ANK 20201229] "[PRE] The ally card itself cannot be cancelled
/// (eg. Louhi's ability cannot cancel it)": her predator's The Summoning
/// [PRE] may be canceled as played; let through, the ally it recruits is
/// never offered her. [RTR 20001020] "Cannot cancel cards that are not
/// played from the hand in the normal fashion."
#[test]
fn louhi_cannot_cancel_an_ally_put_in_play() {
    let (mut e, l, _) = louhi(1);
    let p = e.add_minion(SeatId(0), vampire_with("PRE", 0), 6, false);
    hand(&mut e, 0, &["Summoning, The"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = ["Political Ally", "Form of Mist", "Form of Mist"]
        .map(find_library)
        .to_vec()
        .into();
    let summon = Choice::Play {
        card: 0,
        mode: 1,
        actor: Some(p),
        target: Target::None,
    };
    pick(&mut e, summon);
    let u = vampire_use(&mut e, l, 0);
    assert_eq!(request(&mut e).window, WindowLabel::CardAsPlayed);
    pick(&mut e, Choice::Pass);
    let ally = find_library("Political Ally");
    let take = Choice::TakeFrom {
        zone: zvtes::decision::Zone::Library,
        card: ally,
    };
    let req = offering(&mut e, take);
    assert!(!req.options.contains(&u));
    e.choose(index_of(&req, take));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, l, 0)
    });
    assert!(e
        .state
        .minions
        .iter()
        .any(|m| m.in_play && m.kind.ally_card() == Some(ally)));
}

/// Louhi: [ANK 20201229] "The card is not "played" by the minion (e.g.
/// Louhi's ability cannot cancel it)": the Raven Spy her predator's
/// vampire employs through Trophy: Retainers is never offered her; the
/// same vampire's Raven Spy played from hand as an action is.
#[test]
fn louhi_cannot_cancel_a_retainer_trophy_retainers_employs() {
    let (mut e, l, _) = louhi(1);
    let a = e.add_minion(SeatId(0), vampire_with("ANI", 0), 4, false);
    let t = in_play(&mut e, 0, "Trophy: Retainers", Some(a));
    fill_hand(&mut e, 0);
    let spy = find_library("Raven Spy");
    e.state.seats[0].library.push(spy);
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    pick_when(&mut e, ability(t.0, 0));
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: spy,
        },
    );
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, l, 0)
    });
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == spy && c.host == Some(a)));
    e.state.seats[0].hand = vec![spy].into();
    pick(&mut e, play(0, 0, a));
    vampire_use(&mut e, l, 0);
}

// ---- Lucian -------------------------------------------------------------------------

/// Whether `o` is a use of card `c`'s ability.
fn uses_card(o: &Choice, c: CardId) -> bool {
    matches!(o, Choice::UseAbility { source: AbilitySource::Card(id), .. } if *id == c)
}

/// Lucian: "Lucian may steal equipment as a strike." [LSJ 19980206]
/// "Cannot attempt to steal a piece of equipment that he is prohibited from
/// having (eg. a second Leather Jacket)." His blocker bears a Leather
/// Jacket and a .44 Magnum, he a Leather Jacket: only the Magnum is a
/// target; it ends on him. [LSJ 19980206] [RBK steal-equipment] "The
/// stolen equipment cannot be used until the next round": his additional
/// strike (Lightning Reflexes) that round offers no Magnum strike; after
/// his press (Psyche!), the next round's does.
#[test]
fn lucian_steals_equipment_he_may_hold_unusable_this_round() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lucian"), 6, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 6, 0), 6, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    in_play(&mut e, 0, "Leather Jacket", Some(l));
    let jacket = in_play(&mut e, 1, "Leather Jacket", Some(z));
    let magnum = in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 0, &["Lightning Reflexes", "Psyche!"]);
    bleed_blocked(&mut e, l, z);
    let steal = |c| Choice::UseAbility {
        source: AbilitySource::Vampire(l),
        ability: 0,
        actor: None,
        target: Target::Card(c),
    };
    let req = offering(&mut e, steal(magnum));
    assert!(!req.options.contains(&steal(jacket)), "a second jacket");
    e.choose(index_of(&req, steal(magnum)));
    let req = offering(&mut e, play(0, 0, l));
    assert_eq!(e.state.cards[magnum.index()].host, Some(l));
    assert_eq!(e.combat().expect("combat").round, 1);
    e.choose(index_of(&req, play(0, 0, l)));
    loop {
        let req = request(&mut e);
        assert!(
            !req.options.iter().any(|o| uses_card(o, magnum)),
            "{:?}: {:?}",
            req.window,
            req.options
        );
        if req.options.contains(&play(0, 0, l)) {
            e.choose(index_of(&req, play(0, 0, l)));
            break;
        }
        e.choose(fallback(&req));
    }
    loop {
        let req = request(&mut e);
        if req.options.iter().any(|o| uses_card(o, magnum)) {
            assert_eq!(e.combat().expect("combat").round, 2);
            break;
        }
        e.choose(fallback(&req));
    }
}

// ---- Mariel, Lady Thunder -------------------------------------------------------

/// Mariel (seat 0, 4 blood, in torpor if `torpor`) beside a [tha] [qui]
/// vampire `a` (5 blood) bleeding; seat 1's `z` (5 blood) blocks. Each
/// combatant bears an Owl Companion with 2 life, `a`'s hand holding
/// `cards`, `z`'s Dam the Heart's River.
fn mariel(torpor: bool, cards: &[&str]) -> (Engine, MinionId, MinionId, MinionId, [CardId; 2]) {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mariel, Lady Thunder"), 4, torpor);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 5, 0), 5, false);
    e.state.minions[a.index()].gained_levels = vec!["tha", "for"];
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 5, 1), 5, false);
    e.state.minions[z.index()].gained_levels = vec!["qui"];
    e.add_minion(SeatId(1), plain4(2), 4, false);
    let owls = [(0, a), (1, z)].map(|(s, h)| {
        let o = in_play(&mut e, s, "Owl Companion", Some(h));
        e.state.cards[o.index()].counters = 2;
        o
    });
    hand(&mut e, 0, cards);
    hand(&mut e, 1, &["Dam the Heart's River"]);
    bleed_blocked(&mut e, a, z);
    (e, m, a, z, owls)
}

/// Mariel's ability.
fn mariel_use(m: MinionId) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability: 0,
        actor: None,
        target: Target::None,
    }
}

/// Mariel, Lady Thunder: "Before range is chosen in a combat that does not
/// involve Mariel, she may lock to end that combat and inflict 1
/// unpreventable damage on each combatant and any retainers they have
/// after combat ends." [RTR 19970630] "If she locks to end combat, the
/// damage is applied just after the combat ends": she locks, the combat
/// ends, each combatant loses 1 blood, each Owl Companion 1 life; [LSJ
/// 20100204-2] no combat card prevents it (Hidden Strength [for] never
/// offered); [LSJ 20031123] the blood lost after combat feeds no Taste of
/// Vitae (played at the round's end, it gains nothing). [LSJ 20011022] "The ability can be used when in torpor."
#[test]
fn mariel_locks_to_end_a_combat_and_damages_both_after() {
    for torpor in [false, true] {
        let (mut e, m, a, z, owls) = mariel(torpor, &["Hidden Strength", "Taste of Vitae"]);
        let u = vampire_use(&mut e, m, 0);
        assert_eq!(request(&mut e).window, WindowLabel::CombatBeforeRange);
        pick(&mut e, u);
        assert!(e.state.minions[m.index()].locked);
        // Taste of Vitae at the round's end, Hidden Strength never.
        let mut tasted = false;
        loop {
            let req = request(&mut e);
            assert!(!req.options.contains(&play(0, 0, a)), "{:?}", req.window);
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            match req.options.iter().position(|o| *o == play(1, 0, a)) {
                Some(i) => {
                    tasted = true;
                    e.choose(i)
                }
                None => e.choose(fallback(&req)),
            }
        }
        assert!(tasted);
        assert!(e.combat().is_none());
        assert_eq!((blood(&e, a), blood(&e, z)), (4, 4), "torpor {torpor}");
        assert_eq!(owls.map(|o| e.state.cards[o.index()].counters), [1, 1]);
    }
}

/// Mariel: [ANK 20170111] [PIB 20130319] "Is a damage dealing effect and
/// can be modified by other effects like Dam the Heart's River": the
/// blocker's Dam the Heart's River [qui] on `a`, then Mariel: `a` takes 2.
/// [LSJ 19971110] [ANK 20190725] "The damage is applied even if the combat
/// ends prematurely (eg. by Mariel …)": `a`'s Weather Control [tha]
/// played first, both combatants take its 1 too.
#[test]
fn mariels_damage_is_raised_by_dam_the_hearts_river() {
    let (mut e, m, a, z, _) = mariel(false, &[]);
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, mariel_use(m));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, z)), (5 - 2, 5 - 1));
    let (mut e, m, a, z, _) = mariel(false, &["Weather Control"]);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, mariel_use(m));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, z)), (5 - 2, 5 - 1 - 1));
}

// ---- Mata Hari ----------------------------------------------------------------------

/// Mata Hari (7 blood) and a Toreador for seat 0, two vampires for seat 1.
fn mata_hari() -> (Engine, MinionId, MinionId, MinionId) {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Mata Hari"), 7, false);
    let t = e.add_minion(SeatId(0), plain("Toreador", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    (e, h, t, x)
}

/// Whether `req` offers hand card `card` to `m` (any mode or target).
fn offers_card(req: &zvtes::decision::DecisionRequest, card: u8, m: Option<MinionId>) -> bool {
    req.options
        .iter()
        .any(|o| matches!(o, Choice::Play { card: c, actor, .. } if *c == card && *actor == m))
}

/// Mata Hari: "You and Mata Hari can play cards requiring a sect and/or
/// clan as if she were of that required sect and/or clan." [LSJ 20101013]
/// [ANK 20190228] [ANK 20200705-2] [LSJ 20070708] [RTR 20070707] "… For
/// example, playing Toreador Grand Ball as a Toreador: if chosen as the
/// locked Toreador, they unlock as normal": her Methuselah's Toreador
/// Grand Ball chooses her and the Toreador, she the locked one; she unlocks
/// as normal. [LSJ 20070708] [LSJ 20071109] "If a contract chooses her
/// when played (eg. Dirty Contract), she is 'chosen for the contract' … If
/// the contract choice is an effect of the card in play (eg. Clandestine
/// Contract), she's not chosen": her Clandestine Contract (Banu Haqim)
/// lands on X, not choosing her; her Methuselah's Dirty Contract chooses
/// her as its Samedi.
#[test]
fn mata_hari_plays_cards_requiring_any_sect_or_clan() {
    let (mut e, h, t, x) = mata_hari();
    hand(&mut e, 0, &["Toreador Grand Ball"]);
    e.at_master_phase();
    let ball = Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(h),
    };
    pick(&mut e, ball);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let grand_ball = find_library("Toreador Grand Ball");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == grand_ball && c.chosen == vec![h, t]));
    assert!(e.state.minions[h.index()].locked);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[h.index()].locked, "unlocks as normal");

    let (mut e, h, _, x2) = mata_hari();
    let _ = x;
    hand(&mut e, 0, &["Clandestine Contract"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(h),
            target: Target::Minion(x2),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let cc = find_library("Clandestine Contract");
    let on_x = e
        .state
        .cards
        .iter()
        .find(|c| c.in_play && c.card == cc && c.host == Some(x2))
        .expect("the contract on X");
    assert!(on_x.chosen.is_empty(), "not chosen by the card in play");

    let (mut e, h, _, _) = mata_hari();
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Dirty Contract"]);
    e.at_master_phase();
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(v),
        },
    );
    // The only Samedi to choose: the terms elide.
    request(&mut e);
    let dc = find_library("Dirty Contract");
    assert!(
        e.state
            .cards
            .iter()
            .any(|c| c.in_play && c.card == dc && c.chosen == vec![h]),
        "chosen as it is played"
    );
}

/// Mata Hari: [LSJ 20050526] "When playing a card requiring a sect, her
/// title makes her a titled vampire of that sect (eg. she can play Hand
/// Intervention)": Hand Intervention (a titled Sabbat) offered her, a
/// Toreador not. [LSJ 20010714] "Keeps their title and votes if they
/// changes sects": anarch, still 2 votes.
#[test]
fn mata_hari_calls_hand_intervention_as_titled_sabbat() {
    let (mut e, h, t, _) = mata_hari();
    hand(&mut e, 0, &["Hand Intervention"]);
    let req = request(&mut e);
    assert!(offers_card(&req, 0, Some(h)));
    assert!(!offers_card(&req, 0, Some(t)));
    e.state.minions[h.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    assert_eq!(e.state.title_of(h), Title::Votes(2));
}

/// Mata Hari: [LSJ 20110611] "Orun and Aye cards cannot be placed on her as
/// long as she's not a Laibon": Orun offered on a Laibon, never on her.
/// [ANK 20220718] Infamous Insurgent "does not require an anarch so it
/// cannot be played on vampires that can fake requirements for you (eg.
/// Mata Hari)": not on her (Red List, not Anarch).
#[test]
fn mata_hari_takes_no_orun() {
    let (mut e, h, _, _) = mata_hari();
    let l = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(zvtes::crypt::Sect::Laibon), 0),
        4,
        false,
    );
    let g = e.add_minion(SeatId(0), find_card("Genina, The Red Poet"), 8, false);
    e.state.minions[g.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    hand(&mut e, 0, &["Orun", "Infamous Insurgent"]);
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let on = |card: u8, m: MinionId| {
        req.options.iter().any(|o| {
            matches!(o,
            Choice::Play { card: c, target: Target::Minion(t), .. } if *c == card && *t == m)
        })
    };
    assert!(on(0, l) && !on(0, h));
    assert!(on(1, g) && !on(1, h));
}

/// Mata Hari: [ANK 20180723] "If her capacity is above 10 when playing Call
/// the Great Beast, she burns and it is put into play. If her capacity is
/// less, the card is put on her with tokens equal to her capacity, but she
/// will not be able to add counters to it later on": as a Baali, the card
/// on her with 7 counters; its counter action never offered her.
#[test]
fn mata_hari_plays_call_the_great_beast_as_a_baali() {
    let (mut e, h, _, _) = mata_hari();
    hand(&mut e, 0, &["Call the Great Beast"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(h),
            target: Target::None,
        },
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let beast = find_library("Call the Great Beast");
    let c = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == beast && c.host == Some(h))
        .expect("on Mata Hari");
    assert_eq!(e.state.cards[c].counters, 7);
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(id), .. } if id.index() == c),
    );
    let req = request(&mut e);
    assert!(!req.options.iter().any(|o| matches!(o,
        Choice::UseAbility { source: AbilitySource::Card(id), .. } if id.index() == c)));
}

/// Mata Hari: [LSJ 20050225] "You can still use the burn options of cards
/// with a requirement she could meet when you play them, but does not meet
/// while in play": Sight Beyond Sight (Salubri), its burn option offered in
/// the unlock phase.
#[test]
fn mata_hari_leaves_burn_options_open() {
    let (mut e, _, _, _) = mata_hari();
    hand(&mut e, 0, &["Sight Beyond Sight"]);
    for _ in 0..2 {
        e.state.seats[0]
            .library
            .put(find_library("Leverage"), zvtes::copies::CopyId::NONE);
    }
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&Choice::Discard(0)));
}

/// Mata Hari: [LSJ 20050119] "The ability cannot be used to fulfill card
/// requirements if they are not played normally, like when using Concealed
/// Weapon for an equipment or Piper or The Summoning for an ally": Piper
/// (an Anarch's) naming Political Ally (Ventrue) not offered her, a
/// clanless ally offered; Concealed Weapon naming Righteous Blade (Salubri)
/// never offered her, a .44 Magnum offered.
#[test]
fn mata_hari_meets_no_requirement_for_a_card_played_by_another() {
    for (ally, offered) in [("Political Ally", false), ("Vagabond Mystic", true)] {
        let (mut e, h, _, _) = mata_hari();
        e.state.minions[h.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
        hand(&mut e, 0, &["Piper", ally]);
        e.at_master_phase();
        let req = request(&mut e);
        assert_eq!(offers_card(&req, 0, Some(h)), offered, "{ally}");
    }
    let (mut e, h, _, _) = mata_hari();
    hand(&mut e, 0, &["Political Ally"]);
    let req = request(&mut e);
    assert!(offers_card(&req, 0, Some(h)), "from hand, hers to recruit");
    for (weapon, offered) in [("Righteous Blade", false), (".44 Magnum", true)] {
        let (mut e, h, _, x) = mata_hari();
        hand(&mut e, 0, &["Concealed Weapon", weapon]);
        pick(&mut e, bleed(h));
        pick_when(&mut e, block(x));
        let mut seen = false;
        loop {
            let req = request(&mut e);
            seen |= offers_card(&req, 0, Some(h));
            if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                break;
            }
            e.choose(fallback(&req));
        }
        assert_eq!(seen, offered, "{weapon}");
    }
}

/// Mata Hari: [LSJ 20070301] "If the vampire has an ability to play
/// Discipline-requiring cards 'as if' … the vampire may use that ability
/// even while he is treated as an ally. … Similarly for Mata Hari's
/// ability": she plays The Grandest Trick (Kiasyd) on her bleed, treated as
/// an ally, then Spoils of War (Brujah antitribu) after it succeeds: her
/// Methuselah gains its pool.
#[test]
fn mata_hari_plays_as_a_clan_while_treated_as_an_ally() {
    let (mut e, h, _, _) = mata_hari();
    hand(&mut e, 0, &["Grandest Trick, The", "Spoils of War"]);
    pick(&mut e, bleed(h));
    pick_when(&mut e, play(0, 0, h));
    let req = offering(&mut e, play(0, 0, h));
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    assert_eq!(e.state.seats[1].pool, 10 - 1);
    let before = e.state.seats[0].pool;
    e.choose(index_of(&req, play(0, 0, h)));
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, before + 1);
}

// ---- Round 33, track C, vamp_f --------------------------------------------------

/// Monique and Pieter ("Sabbat."), Normal and Ramiel DuPre ("Camarilla."),
/// Reverend Djoser Jones ("Independent."): no definition. Quinton McDonnell
/// ("Camarilla primogen: +1 strength.") strikes for 2. Titles, votes,
/// scarce, sterile, infernal and Red List generated: Quinton and Natasha
/// primogens, Rake Prince of Atlanta, Neferu 2 votes ([LSJ 20010714]
/// "Keeps their title and votes if they changes sects"), Pariah scarce and
/// sterile, Petaniqua infernal and Red List, Rabbat Red List.
#[test]
fn the_round_33_f_vampires_carry_their_printed_traits() {
    for name in [
        "Monique",
        "Pieter",
        "Normal",
        "Ramiel DuPre",
        "Reverend Djoser Jones",
    ] {
        assert!(zvtes::cards::vampire(find_card(name)).is_none(), "{name}");
    }
    let c = |n: &str| &CRYPT[find_card(n) as usize];
    for n in ["Quinton McDonnell", "Natasha Volfchek"] {
        assert_eq!(c(n).title, Title::Primogen, "{n}");
    }
    assert_eq!(c("Rake").title, Title::Prince("Atlanta"));
    assert_eq!(c("Neferu").title, Title::Votes(2));
    assert!(c("Pariah").scarce && c("Pariah").sterile);
    for adv in [false, true] {
        let p = &CRYPT[card_adv("Petaniqua", adv) as usize];
        assert!(p.infernal && p.red_list, "{adv}");
    }
    assert!(c("Rabbat, The Sewer Goddess").red_list);
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Neferu"), 4, false);
    e.state.minions[n.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    assert_eq!(e.state.title_of(n), Title::Votes(2), "kept");
    assert_eq!(hand_strike_damage(find_card("Quinton McDonnell")), (2, 0));
}

/// Natasha Volfchek: "+1 stealth on political actions. +1 bleed."
/// Political: +1 stealth, a bleed none; her bleed 2.
#[test]
fn natasha_volfchek_gets_stealth_on_political_actions() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Natasha Volfchek"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let stealth = |e: &Engine, kind| {
        e.state.printed_mod(
            n,
            zvtes::cards::Stat::Stealth,
            zvtes::state::PrintedCtx {
                kind: Some(kind),
                ..Default::default()
            },
        )
    };
    assert_eq!(
        (
            stealth(&e, ActionKind::Political),
            stealth(&e, ActionKind::Bleed)
        ),
        (1, 0)
    );
    pick(&mut e, bleed(n));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

/// Nik: "Brujah get +1 intercept when attempting to block Nik." His bleed:
/// a Brujah blocker at +1, a Toreador at 0; [LSJ 20011022] "The passive
/// ability applies even if the vampire is in torpor": read so in torpor.
#[test]
fn nik_gives_brujah_intercept_from_torpor() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nik"), 1, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let brujah = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let toreador = e.add_minion(SeatId(1), plain("Toreador", 0), 4, false);
    pick(&mut e, bleed(n));
    request(&mut e);
    let a = e.action().expect("bleed");
    assert_eq!(a.intercept_of(&e.state, brujah), 1);
    assert_eq!(a.intercept_of(&e.state, toreador), 0);
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nik"), 1, true);
    let brujah = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let against = e.state.printed_mod(
        n,
        zvtes::cards::Stat::InterceptAgainst,
        zvtes::state::PrintedCtx {
            opponent: Some(brujah),
            ..Default::default()
        },
    );
    assert_eq!(against, 1, "in torpor");
}

// ---- Neferu ---------------------------------------------------------------------

/// Seat 0's Tara calls Kine Resources Contested, carrying a corruption
/// counter of seat 1's; seat 1's Neferu (in torpor with `torpor`, 3
/// blood), and a titled vampire of seat 0 with no corruption.
fn neferu(torpor: bool) -> (Engine, MinionId, MinionId, MinionId) {
    let mut e = game(2);
    let tara = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.state.minions[tara.index()].add_corruption(SeatId(1), 1);
    let clean = e.add_minion(
        SeatId(0),
        fixture_vampire(|c| c.title == Title::Primogen, 0),
        4,
        false,
    );
    let n = e.add_minion(SeatId(1), find_card("Neferu"), 3, torpor);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, tara));
    (e, n, tara, clean)
}

/// Neferu's use, `favor` or against, at `t`.
fn neferu_at(n: MinionId, favor: bool, t: MinionId) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(n),
        ability: if favor { 0 } else { 1 },
        actor: None,
        target: Target::Minion(t),
    }
}

/// The side of `m`'s standing cast in the referendum, if any.
fn cast_of(e: &Engine, m: MinionId) -> Option<bool> {
    e.referendum()
        .expect("referendum")
        .records
        .iter()
        .find_map(|r| {
            (r.source == zvtes::referendum::Source::Vampire(m) && !r.canceled).then_some(r.favor)
        })
}

/// Neferu: "During the polling step of any referendum, Neferu can burn 1
/// blood to change the votes and ballots of a vampire with any of your
/// corruption counters to votes and ballots of your choice." [ANK
/// 20220705] [LSJ 20100705] [LSJ 20011022] [LSJ 20100902-2] [ANK
/// 20180307-2] "The ability can be used while locked or in torpor, only
/// once per referendum": from torpor, Tara's votes cast in favor turned
/// against, 1 blood; not offered again; a titled vampire with no corruption
/// counter is no target.
#[test]
fn neferu_turns_a_corrupted_vampires_votes_once_from_torpor() {
    let (mut e, n, tara, clean) = neferu(true);
    pick_when(&mut e, votes(tara, true));
    let req = offering(&mut e, neferu_at(n, false, tara));
    assert!(!req.options.contains(&neferu_at(n, false, clean)));
    e.choose(index_of(&req, neferu_at(n, false, tara)));
    request(&mut e);
    assert_eq!(cast_of(&e, tara), Some(false));
    assert_eq!(blood(&e, n), 2);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, n, 0) || uses_vampire(o, n, 1)
    });
}

/// Neferu: [ANK 20190731] "Cannot use her ability to force an abstaining
/// vampire to vote": Tara, changed in favor before she casts, has cast
/// nothing and may only cast in favor.
#[test]
fn neferu_does_not_force_an_abstaining_vampire() {
    let (mut e, n, tara, _) = neferu(false);
    pick_when(&mut e, neferu_at(n, true, tara));
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    assert_eq!(cast_of(&e, tara), None, "not compelled");
    assert!(req.options.contains(&votes(tara, true)));
    assert!(!req.options.contains(&votes(tara, false)));
}

// ---- Pariah ---------------------------------------------------------------------

/// Seat 0's Pariah (in torpor with `torpor`) and another vampire, seat 1's
/// vampire, `cards` in seat 0's hand over a stocked library; seat 0's
/// master phase begun.
fn pariah(torpor: bool, cards: &[&str]) -> (Engine, MinionId) {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), find_card("Pariah"), 4, torpor);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    for _ in 0..5 {
        e.state.seats[0].library.push(find_library("Deflection"));
    }
    hand(&mut e, 0, cards);
    e.at_master_phase();
    (e, p)
}

/// Pariah: "During your master phase, discard a master card or lock
/// Pariah." With a master card: lock or discard it (no other card offered),
/// the discard replaced at once, Pariah unlocked; with none: locked. [LSJ
/// 20011022] [LSJ 20100902-2] "His ability still applies when in torpor":
/// the due comes in torpor too.
#[test]
fn pariah_discards_a_master_card_or_locks_each_master_phase() {
    let (mut e, p) = pariah(false, &["Dreams of the Sphinx", "Deflection"]);
    pick_when(&mut e, Choice::ResolvePrinted { minion: p });
    let req = request(&mut e);
    assert_eq!(req.options, vec![Choice::Pass, Choice::Discard(0)]);
    e.choose(index_of(&req, Choice::Discard(0)));
    let dreams = find_library("Dreams of the Sphinx");
    assert!(e.state.seats[0].ash.contains(&dreams));
    assert_eq!(e.state.seats[0].library.len(), 5 - 5, "replaced: drawn up");
    assert!(!e.state.seats[0].hand.contains(&dreams));
    assert!(!e.state.minions[p.index()].locked);
    let (mut e, p) = pariah(false, &["Deflection"]);
    request(&mut e);
    assert!(e.state.minions[p.index()].locked, "no master card: locked");
    let (mut e, p) = pariah(true, &["Dreams of the Sphinx"]);
    let req = offering(&mut e, Choice::ResolvePrinted { minion: p });
    assert_eq!(req.seat, SeatId(0));
}

/// Pariah: "Pariah cannot take undirected actions other than hunting. He
/// can enter combat with any minion as a Ⓓ action." His bleed, his rush and
/// the hunt are offered; a political action, an equip, an employ are not;
/// [ANK 20180202] "Can use card-provided undirected hunt actions (eg.
/// Vulture's Buffet)": offered.
#[test]
fn pariah_takes_only_directed_actions_and_hunts() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), find_card("Pariah"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.state.seats[1].ash.push(find_library("Ghoul Retainer"));
    hand(
        &mut e,
        0,
        &[
            "Vulture's Buffet",
            "Kine Resources Contested",
            "Leather Jacket",
            "Ghoul Retainer",
        ],
    );
    let req = request(&mut e);
    for want in [
        bleed(p),
        action(p, ActionKind::Hunt),
        vampire_action(p, 0, Target::Minion(x)),
        play(0, 0, p),
    ] {
        assert!(req.options.contains(&want), "{want:?}: {:?}", req.options);
    }
    for card in 1..4 {
        assert!(
            !req.options.iter().any(|o| matches!(o,
                Choice::Play { card: c, actor: Some(a), .. } if *c == card && *a == p)),
            "card {card}: {:?}",
            req.options
        );
    }
}

/// Pariah in torpor: the leave-torpor action is undirected, not offered
/// (RED reading: the text read literally, "His ability still applies when
/// in torpor", LSJ 20011022); another torpid vampire's is.
#[test]
fn pariah_cannot_leave_torpor_by_his_own_action() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), find_card("Pariah"), 4, true);
    let o = e.add_minion(SeatId(0), plain4(0), 4, true);
    e.add_minion(SeatId(0), plain4(2), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let req = request(&mut e);
    assert!(!req.options.contains(&action(p, ActionKind::LeaveTorpor)));
    assert!(req.options.contains(&action(o, ActionKind::LeaveTorpor)));
}

// ---- Patrizia Giovanni ----------------------------------------------------------

/// Seat 0's Patrizia (5 blood), `levels` gained, `ash` in seat 0's ash
/// heap, `hand_cards` in hand; another vampire each.
fn patrizia(
    levels: &[&'static str],
    ash: &[&str],
    hand_cards: &[&str],
) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let p = e.add_minion(
        SeatId(0),
        find_card("Patrizia Giovanni, Collector of Secrets"),
        5,
        false,
    );
    e.state.minions[p.index()].gained_levels = levels.to_vec();
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let z = e.add_minion(SeatId(1), plain4(1), 4, false);
    for c in ash {
        e.state.seats[0].ash.push(find_library(c));
    }
    hand(&mut e, 0, hand_cards);
    (e, p, z)
}

/// The retainers named `name` on `m`.
fn retainers_on(e: &Engine, name: &str, m: MinionId) -> Vec<CardId> {
    let card = find_library(name);
    (0..e.state.cards.len() as u32)
        .map(CardId)
        .filter(|&c| {
            let x = &e.state.cards[c.index()];
            x.present() && x.card == card && x.host == Some(m)
        })
        .collect()
}

/// Patrizia: "Patrizia can employ a retainer from your ash heap as an
/// action whose cost is increased by 1 blood (requirements and cost apply
/// as normal)." [LSJ 20010627] "the card to retrieve must be announced
/// when declaring the action": named among the retainers she may employ
/// (Crypt's Sons, requiring an Anarch, and Twisted Bloodhound, a follower
/// of Caine, not offered); [TOM 19960130] an employ action. Unblocked:
/// Ghoul Retainer on her, its 2 pool paid, her blood 1 less; blocked: left
/// in the ash heap, nothing paid.
#[test]
fn patrizia_employs_a_retainer_from_the_ash_heap_for_a_blood_more() {
    for blocked in [false, true] {
        let ash = [
            "Ghoul Retainer",
            "Mr. Winthrop",
            "Crypt's Sons",
            "Twisted Bloodhound",
        ];
        let (mut e, p, z) = patrizia(&[], &ash, &[]);
        pick(&mut e, vampire_action(p, 0, Target::None));
        let req = request(&mut e);
        let named = |n: &str| Choice::FindCard(find_library(n));
        let mut offered = req.options.clone();
        offered.sort_by_key(|o| format!("{o:?}"));
        let mut want = vec![named("Ghoul Retainer"), named("Mr. Winthrop")];
        want.sort_by_key(|o| format!("{o:?}"));
        assert_eq!(offered, want);
        e.choose(index_of(&req, named("Ghoul Retainer")));
        request(&mut e);
        assert!(e.action().expect("her action").is_employ_action(&e.state));
        if blocked {
            pick_when(&mut e, block(z));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let hound = find_library("Ghoul Retainer");
        assert_eq!(
            retainers_on(&e, "Ghoul Retainer", p).len(),
            usize::from(!blocked)
        );
        assert_eq!(e.state.seats[0].ash.contains(&hound), blocked);
        if !blocked {
            assert_eq!(e.state.seats[0].pool, 10 - 2);
            assert_eq!(blood(&e, p), 5 - 1);
        } else {
            assert_eq!(e.state.seats[0].pool, 10);
        }
    }
}

/// Patrizia: [LSJ 20080803] "When choosing a card whose effect depend on
/// Discipline level, the vampire decides in respect to his disciplines the
/// normal way": with superior Animalism, the Raven Spy named at either
/// version, her player's choice as it is named (rules-qa 125.155), at 1 or
/// 2 life, its 1 blood and her 1 paid.
#[test]
fn patrizia_employs_the_version_her_player_chooses() {
    let card = find_library("Raven Spy");
    for mode in [0, 1] {
        let (mut e, p, _) = patrizia(&["ani", "ani"], &["Raven Spy", "Mr. Winthrop"], &[]);
        pick(&mut e, vampire_action(p, 0, Target::None));
        pick_when(&mut e, Choice::FindCard(card));
        let req = request(&mut e);
        assert_eq!(
            req.options,
            vec![
                Choice::Version { card, mode: 0 },
                Choice::Version { card, mode: 1 }
            ]
        );
        e.choose(index_of(&req, Choice::Version { card, mode }));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let spy = retainers_on(&e, "Raven Spy", p);
        assert_eq!(spy.len(), 1);
        assert_eq!(e.state.cards[spy[0].index()].counters, 1 + mode);
        assert_eq!(blood(&e, p), 5 - 2);
    }
}

/// Patrizia: [ANK 20180817] "Does not prevent the acting vampire from
/// playing the equipment/retainer/ally directly this turn if he unlocks
/// (not the same action)": unlocked after her ash employ, Ghoul Retainer
/// from hand offered.
#[test]
fn patrizia_may_employ_from_hand_after_her_ash_employ() {
    let (mut e, p, _) = patrizia(
        &[],
        &["Ghoul Retainer", "Mr. Winthrop"],
        &["Ghoul Retainer"],
    );
    pick(&mut e, vampire_action(p, 0, Target::None));
    pick_when(&mut e, Choice::FindCard(find_library("Ghoul Retainer")));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(retainers_on(&e, "Ghoul Retainer", p).len(), 1);
    e.state.minions[p.index()].locked = false;
    assert!(request(&mut e).options.contains(&play(0, 0, p)));
}

// ---- Paul Forrest ---------------------------------------------------------------

/// Paul Forrest: "During a bleed action, Paul can burn 1 blood to give the
/// acting Ravnos you control +1 bleed." [ANK 20220705] [LSJ 20100705]
/// "can be used while locked or in torpor": from torpor, another Ravnos's
/// bleed at 2, Paul 1 blood less; [ANK 20231124] [LSJ 19970718] "only
/// during a bleed action before Action Resolution"; a Brujah's bleed: never
/// offered.
#[test]
fn paul_forrest_gives_an_acting_ravnos_bleed_from_torpor() {
    for (clan, offered) in [("Ravnos", true), ("Brujah", false)] {
        let mut e = game(2);
        let paul = e.add_minion(SeatId(0), find_card("Paul Forrest, False Prophet"), 3, true);
        let a = e.add_minion(SeatId(0), plain(clan, 0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        pick(&mut e, bleed(a));
        if offered {
            let u = vampire_use(&mut e, paul, 0);
            assert!(e.action().is_some(), "before resolution");
            pick(&mut e, u);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.seats[1].pool, 10 - 2);
            assert_eq!(blood(&e, paul), 2);
        } else {
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
                uses_vampire(o, paul, 0)
            });
            assert_eq!(e.state.seats[1].pool, 10 - 1);
        }
    }
}

// ---- Petaniqua --------------------------------------------------------------------

/// Petaniqua (`adv`, merged with the base with `merged`) bleeds, seat 1's
/// X blocks.
fn petaniqua(adv: bool, merged: bool) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), card_adv("Petaniqua", adv), 6, false);
    if merged {
        e.state.minions[p.index()].merged = Some(card_adv("Petaniqua", false));
    }
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 6, 0), 6, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    (e, p, x)
}

/// Petaniqua: "Once each round of combat after the first, minions in combat
/// with Petaniqua take 2 damage before range is determined." [RTR
/// 19970630] environmental, [ANK 20170930] mended with the round's
/// pre-range damage: round 1, X untouched as range is determined; she
/// presses (Relentless Pursuit [pot]); round 2, X 2 blood less as range is
/// determined (its round-1 hand strike damage aside), Petaniqua none. Fake
/// Out in hand opens the range step.
#[test]
fn petaniqua_damages_her_opponent_before_range_after_the_first_round() {
    let (mut e, p, x) = petaniqua(false, false);
    e.state.minions[p.index()].gained_levels = vec!["pot"];
    hand(&mut e, 0, &["Relentless Pursuit", "Fake Out"]);
    bleed_blocked(&mut e, p, x);
    expect(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert_eq!(blood(&e, x), 6, "round 1: none");
    let req = offering(&mut e, play(0, 0, p));
    let (bx, bp) = (blood(&e, x), blood(&e, p));
    e.choose(index_of(&req, play(0, 0, p)));
    expect(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert_eq!(e.combat().expect("round 2").round, 2);
    assert_eq!(
        (blood(&e, x), blood(&e, p)),
        (bx - 2, bp - 1),
        "her press's 1"
    );
}

/// Petaniqua: "Allies and retainers cost her -1 blood or pool": Ghoul
/// Retainer (2 pool) for 1 from hand.
#[test]
fn petaniqua_pays_one_less_for_allies_and_retainers() {
    let (mut e, p, _) = petaniqua(false, false);
    hand(&mut e, 0, &["Ghoul Retainer"]);
    pick(&mut e, play(0, 0, p));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(retainers_on(&e, "Ghoul Retainer", p).len(), 1);
    assert_eq!(e.state.seats[0].pool, 10 - 1);
}

/// Petaniqua (advanced): "Once each combat, Petaniqua can burn 1 blood to
/// get 1 press." [PIB 20150121] "can only be used during the press step".
#[test]
fn petaniqua_advanced_presses_once_in_the_press_step() {
    let (mut e, p, x) = petaniqua(true, false);
    bleed_blocked(&mut e, p, x);
    let mut seen = 0;
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        if let Some(i) = req.options.iter().position(|o| uses_vampire(o, p, 0)) {
            assert_eq!(req.window, WindowLabel::CombatPress);
            seen += 1;
            e.choose(i);
            continue;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(seen, 1, "once each combat");
}

/// Petaniqua (advanced): "She can recruit a werewolf ally as if she met the
/// requirements of playing that card." Cry Wolf (a ready anarch) and
/// Remnant of the Endless Storm (a magaji) offered her, not an Independent
/// without her text; [LSJ 20101013] [ANK 20190228] [ANK 20200705-2] [LSJ
/// 20070708] [RTR 20070707] treated so for all the card's effects:
/// her Cry Wolf recruit blocked (X's Enhanced Senses [aus] against its +1
/// stealth), she goes to torpor (the acting anarch).
#[test]
fn petaniqua_recruits_a_werewolf_as_if_she_met_its_requirements() {
    let (mut e, p, x) = petaniqua(true, false);
    e.state.seats[0].pool = 20;
    let other = e.add_minion(
        SeatId(0),
        fixture_vampire(
            |c| c.title == Title::None && c.sect == Some(zvtes::crypt::Sect::Independent),
            0,
        ),
        4,
        false,
    );
    hand(&mut e, 0, &["Cry Wolf", "Remnant of the Endless Storm"]);
    let req = request(&mut e);
    for card in [0, 1] {
        assert!(
            req.options.contains(&play(card, 0, p)),
            "{card}: {:?}",
            req.options
        );
        assert!(!req.options.contains(&play(card, 0, other)), "{card}");
    }
    e.state.minions[x.index()].gained_levels = vec!["aus"];
    hand(&mut e, 1, &["Enhanced Senses"]);
    pick(&mut e, play(0, 0, p));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[p.index()].in_torpor);
}

/// Merged Petaniqua: [ANK 20180524] "[MERGED] Is Independent": Independent,
/// both texts: +1 bleed (bleed 2), +1 strength, the press, the round-2
/// damage before range.
#[test]
fn petaniqua_merged_keeps_both_texts() {
    let (mut e, p, x) = petaniqua(true, true);
    assert_eq!(e.state.sect_of(p), Some(zvtes::crypt::Sect::Independent));
    let d = zvtes::cards::vampire_of(&e.state.minions[p.index()]).expect("merged");
    assert_eq!(d.abilities.len(), 1);
    let stat = |s| e.state.printed_mod(p, s, Default::default());
    use zvtes::cards::Stat;
    assert_eq!(
        (
            stat(Stat::Bleed),
            stat(Stat::Strength),
            stat(Stat::EnvironmentalBeforeRangeAfterFirstRound)
        ),
        (1, 1, 2)
    );
    assert!(e.state.recruits_as_if(p, find_library("Cry Wolf")));
    pick(&mut e, bleed(p));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
    let _ = x;
}

/// Vlad Tepes and Petaniqua by The Summoning [PRE]: [LSJ 20050119] "The
/// ability cannot be used to fulfill card requirements if they are not
/// played normally (… The Summoning for an ally)": Cry Wolf (a ready
/// anarch) is not offered from the library, Vagabond Mystic is.
#[test]
fn vlad_tepes_and_petaniqua_meet_no_requirement_of_a_summoned_ally() {
    for name in ["Vlad Tepes", "Petaniqua"] {
        let mut e = game(2);
        let card = if name == "Petaniqua" {
            card_adv(name, true)
        } else {
            find_card(name)
        };
        let v = e.add_minion(SeatId(0), card, 4, false);
        e.state.minions[v.index()].gained_levels = vec!["pre", "pre"];
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        let [mystic, wolf] = ["Vagabond Mystic", "Cry Wolf"].map(find_library);
        for c in [mystic, wolf, find_library("Deflection")] {
            e.state.seats[0].library.push(c);
        }
        hand(&mut e, 0, &["Summoning, The"]);
        fill_hand(&mut e, 0);
        pick(&mut e, play(0, 1, v));
        let take = |card| Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card,
        };
        let req = offering(&mut e, take(mystic));
        assert!(!req.options.contains(&take(wolf)), "{name}");
    }
}

// ---- Petru Sipos, Rafaele Giovanni ----------------------------------------------

/// Petru Sipos: "Petru can steal 1 blood from a vampire in torpor as a Ⓓ
/// action." Seat 1's torpid vampire 1 less, Petru 1 more; a ready vampire
/// no target; the target rescued before resolution: no effect.
#[test]
fn petru_sipos_steals_a_blood_from_a_torpid_vampire() {
    for rescued in [false, true] {
        let mut e = game(2);
        let p = e.add_minion(SeatId(0), find_card("Petru Sipos"), 3, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let t = e.add_minion(SeatId(1), plain4(1), 3, true);
        let r = e.add_minion(SeatId(1), plain4(2), 4, false);
        let req = request(&mut e);
        assert!(!req
            .options
            .contains(&vampire_action(p, 0, Target::Minion(r))));
        e.choose(index_of(&req, vampire_action(p, 0, Target::Minion(t))));
        request(&mut e);
        if rescued {
            e.state.minions[t.index()].in_torpor = false;
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let moved = u8::from(!rescued);
        assert_eq!(
            (blood(&e, p), blood(&e, t)),
            (3 + moved, 3 - moved),
            "{rescued}"
        );
    }
}

/// Rafaele Giovanni: "Rafaele can bleed your predator as a Ⓓ action. +1
/// bleed." Three players: his bleed at seat 2 (his predator), 2 pool.
#[test]
fn rafaele_bleeds_his_predator() {
    let mut e = game(3);
    let r = e.add_minion(SeatId(0), find_card("Rafaele Giovanni"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(2), plain4(2), 4, false);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&vampire_action(r, 0, Target::Seat(SeatId(1)))));
    e.choose(index_of(
        &req,
        vampire_action(r, 0, Target::Seat(SeatId(2))),
    ));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 10, 8]);
}

// ---- Rabbat -----------------------------------------------------------------------

/// Seat 0's Rabbat (`b` blood, capacity 7) and another vampire; seat 1's
/// vampire X and `ally` (an ally) if any.
fn rabbat(b: u8) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Rabbat, The Sewer Goddess"), b, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    (e, r, x)
}

/// Rabbat: "If she hunts, you may move 1 of the blood she gains to your
/// pool." [LSJ 20050331] "may be used even when she is at full capacity.
/// One blood goes to your pool directly": at 7, the pool +1, her blood 7;
/// declined at 5: her blood 6, the pool unchanged.
#[test]
fn rabbat_moves_a_hunted_blood_to_the_pool_even_at_capacity() {
    for (b, take) in [(7, true), (5, false)] {
        let (mut e, r, _) = rabbat(b);
        pick(&mut e, action(r, ActionKind::Hunt));
        let req = offering(&mut e, Choice::ResolvePrinted { minion: r });
        assert!(req.options.contains(&Choice::Pass));
        e.choose(index_of(
            &req,
            if take {
                Choice::ResolvePrinted { minion: r }
            } else {
                Choice::Pass
            },
        ));
        request(&mut e);
        assert_eq!(e.state.seats[0].pool, 10 + i32::from(take), "{b}");
        assert_eq!(blood(&e, r), if take { 7 } else { 6 }, "{b}");
    }
}

/// Seat 1's turn, its `rusher` entering combat with Rabbat.
fn rushed(e: &mut Engine, r: MinionId) {
    e.state.turn = 5;
    e.state.active = SeatId(1);
    let req = request(e);
    let rush = req
        .options
        .iter()
        .position(|o| matches!(o, Choice::UseAbility { target: Target::Minion(t), .. } if *t == r))
        .expect("a rush at Rabbat");
    e.choose(rush);
}

/// Rabbat: "Rabbat may send a vampire to torpor or burn an ally as a
/// strike." Pariah rushing her: to torpor; [LSJ 20090622-2] "that vampire
/// is wounded (if they play Undying Tenacity, they go to torpor after
/// combat)": ready at the press step (Relentless Pursuit in hand opens
/// it), in torpor after. Cry Wolf (an ally)
/// rushing her: burned.
#[test]
fn rabbat_strikes_to_send_a_vampire_to_torpor_or_burn_an_ally() {
    let (mut e, r, _) = rabbat(5);
    let x = e.add_minion(SeatId(1), find_card("Pariah"), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["for"];
    hand(&mut e, 1, &["Undying Tenacity", "Relentless Pursuit"]);
    rushed(&mut e, r);
    let u = vampire_use(&mut e, r, 0);
    pick(&mut e, u);
    pick_when(&mut e, play(0, 0, x));
    expect(&mut e, SeatId(1), WindowLabel::CombatPress);
    assert!(!e.state.minions[x.index()].in_torpor);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
    let (mut e, r, _) = rabbat(5);
    let wolf = e.add_ally(SeatId(1), find_library("Cry Wolf"));
    rushed(&mut e, r);
    let u = vampire_use(&mut e, r, 0);
    pick(&mut e, u);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[wolf.index()].present(), "burned");
}

/// Rabbat: "She cannot take directed actions or block actions that aren't
/// directed at her or at a card on her." No bleed nor Red List hunt (a
/// directed action) offered her, her hunt is; X's bleed at her Methuselah: no block attempt; Pariah's rush at
/// her or Ethan Locke's steal of the master Discipline card on her: offered;
/// a rush at her Methuselah's other vampire: not.
#[test]
fn rabbat_blocks_only_actions_at_her_or_her_cards() {
    let (mut e, r, x) = rabbat(5);
    e.state.minions[x.index()].made_red_list = true;
    e.state.red_list_marks.push((SeatId(0), x));
    let red_list = |m| Choice::TakeAction {
        minion: m,
        action: ActionKind::RedListHunt,
        target: Some(x),
    };
    let req = request(&mut e);
    assert!(!req.options.contains(&bleed(r)));
    assert!(!req.options.contains(&red_list(r)), "directed");
    assert!(req.options.contains(&red_list(MinionId(1))));
    assert!(req.options.contains(&action(r, ActionKind::Hunt)));
    let attacker = |name: &str| {
        let (mut e, r, _) = rabbat(5);
        let other = MinionId(1);
        let a = e.add_minion(SeatId(1), find_card(name), 6, false);
        let card = master_on(&mut e, 0, "Auspex", r);
        e.state.turn = 5;
        e.state.active = SeatId(1);
        (e, r, other, a, card)
    };
    let (mut e, r, _, a, _) = attacker("Pariah");
    e.state.seats[1].pool = 10;
    pick(&mut e, bleed(a));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, block(r));
    let (mut e, r, other, a, _) = attacker("Pariah");
    pick(&mut e, vampire_action(a, 0, Target::Minion(other)));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, block(r));
    let (mut e, r, _, a, _) = attacker("Pariah");
    pick(&mut e, vampire_action(a, 0, Target::Minion(r)));
    offering(&mut e, block(r));
    let (mut e, r, _, a, card) = attacker("Ethan Locke");
    pick(&mut e, vampire_action(a, 0, Target::Card(card)));
    offering(&mut e, block(r));
}

// ---- Rake, Ransam, Rex --------------------------------------------------------------

/// Rake: "+1 strength in combat with Ventrue"; Ransam, The Old Man: "+1
/// strength in combat with a Tremere." Clan read live: their clan +1,
/// another clan 0, the antitribu 0.
#[test]
fn rake_and_ransam_are_stronger_against_their_clan() {
    for (name, clan, anti) in [
        ("Rake", "Ventrue", "Ventrue antitribu"),
        ("Ransam, The Old Man", "Tremere", "Tremere antitribu"),
    ] {
        let mut e = game(2);
        let v = e.add_minion(SeatId(0), find_card(name), 4, false);
        let strength = |e: &mut Engine, c: &str| {
            let o = e.add_minion(SeatId(1), plain(c, 0), 4, false);
            e.state.printed_mod(
                v,
                zvtes::cards::Stat::Strength,
                zvtes::state::PrintedCtx {
                    opponent: Some(o),
                    ..Default::default()
                },
            )
        };
        assert_eq!(
            (
                strength(&mut e, clan),
                strength(&mut e, "Brujah"),
                strength(&mut e, anti)
            ),
            (1, 0, 0),
            "{name}"
        );
    }
}

/// Rex, The Necronomist: "Combat cards cost Rex +1 blood." Dodge (free)
/// costs him 1; [PIB 20150924-2] Swallowed by the Night as a combat card +1,
/// as an action modifier none.
#[test]
fn rex_pays_more_for_combat_cards() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Rex, The Necronomist"), 3, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    let swallowed = find_library("Swallowed by the Night");
    let cost = |e: &Engine, mode| {
        e.state.printed_mod(
            r,
            zvtes::cards::Stat::BloodCostMore,
            zvtes::state::PrintedCtx {
                card: Some((swallowed, mode)),
                ..Default::default()
            },
        )
    };
    assert_eq!((cost(&e, 0), cost(&e, 1)), (0, 1));
    hand(&mut e, 0, &["Dodge"]);
    bleed_blocked(&mut e, r, x);
    pick_when(&mut e, play(0, 0, r));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, r), 3 - 1);
}

// ---- Round 33, track C, vamp_h ----------------------------------------------------

/// Timothy Crowley ("Camarilla Prince of Dallas."), Uma Hatch, Violette
/// Prentiss, Vliam Andor, Yuri, The Talon and Zebulon ("Camarilla."): no
/// definition, their sect and title generated.
#[test]
fn round33_c_vamp_h_textless_vampires_carry_only_their_sect_and_title() {
    use zvtes::crypt::Sect;
    for (name, title) in [
        ("Timothy Crowley", Title::Prince("Dallas")),
        ("Uma Hatch", Title::None),
        ("Violette Prentiss", Title::None),
        ("Vliam Andor", Title::None),
        ("Yuri, The Talon", Title::None),
        ("Zebulon", Title::None),
    ] {
        let i = crypt_card(name, 1);
        assert!(zvtes::cards::vampire(i).is_none(), "{name} has text");
        assert_eq!(CRYPT[i as usize].title, title, "{name}");
        assert_eq!(CRYPT[i as usize].sect, Some(Sect::Camarilla), "{name}");
    }
}

// ---- Tiberius, The Scandalmonger --------------------------------------------------

/// Tiberius: "If Tiberius is ready during your unlock phase, you may look
/// at the top card in your prey's library." Used, his Methuselah looks at
/// the prey's top card; in torpor, never offered.
#[test]
fn tiberius_looks_at_his_preys_library_top() {
    for torpor in [false, true] {
        let mut e = game(2);
        let t = e.add_minion(
            SeatId(0),
            find_card("Tiberius, The Scandalmonger"),
            4,
            torpor,
        );
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        fill_hand(&mut e, 1);
        let top = find_library("Lost in Crowds");
        e.state.seats[1].library.push(find_library("Conditioning"));
        e.state.seats[1].library.push(top);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        if torpor {
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
                uses_vampire(o, t, 0)
            });
            continue;
        }
        let u = vampire_use(&mut e, t, 0);
        e.notes = Some(Vec::new());
        pick(&mut e, u);
        request(&mut e);
        let looks = noted_looks(&mut e);
        assert!(
            looks.iter().any(|l| l.viewer == SeatId(0)
                && l.seat == SeatId(1)
                && l.cards == vec![top]
                && l.zone == zvtes::game::LookedZone::Library),
            "{looks:?}"
        );
    }
}

/// A look at a library's top is not offered with that library empty
/// (rules-qa 122.139: "a card naming an object there is none of", D14):
/// every library empty, neither Tiberius's look at his prey's, Kurt
/// Densch's at his own, Weirding Stone's at the active Methuselah's nor
/// Sweeper's at each Methuselah's is offered around the table; a card put
/// in the prey's library, Tiberius's, Sweeper's and (in that Methuselah's
/// unlock phase) the Stone's are, Kurt's still not.
#[test]
fn library_looks_need_a_card_there() {
    use zvtes::crypt::Sect;
    let mut e = game(3);
    let t = e.add_minion(
        SeatId(0),
        find_card("Tiberius, The Scandalmonger"),
        4,
        false,
    );
    let k = e.add_minion(SeatId(0), find_card("Kurt Densch"), 4, false);
    let an = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Anarch), 0),
        4,
        false,
    );
    let w = e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(2), plain4(2), 4, false);
    let stone = in_play(&mut e, 0, "Weirding Stone", Some(w));
    let sweeper = in_play(&mut e, 0, "Sweeper", Some(an));
    for s in 0..3 {
        fill_hand(&mut e, s);
    }
    let look = |o: &Choice, of: &[MinionId], cards: &[CardId]| match o {
        Choice::UseAbility {
            source: AbilitySource::Vampire(v),
            ..
        } => of.contains(v),
        Choice::UseAbility {
            source: AbilitySource::Card(c),
            ..
        } => cards.contains(c),
        _ => false,
    };
    let every = |o: &Choice| look(o, &[t, k], &[stone, sweeper]);
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, every);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, every);
    e.state.seats[1].library.push(find_library("Form of Mist"));
    let req = request(&mut e);
    e.choose(fallback(&req));
    let mut seen = Vec::new();
    for _ in 0..300 {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase)
            && e.state.active == SeatId(0)
        {
            break;
        }
        assert!(
            !req.options.iter().any(|o| look(o, &[k], &[])),
            "Kurt's: {req:?}"
        );
        if req.options.iter().any(|x| look(x, &[t], &[])) {
            seen.push("Tiberius");
        }
        for (c, who) in [(stone, "Stone"), (sweeper, "Sweeper")] {
            if req.options.iter().any(|x| look(x, &[], &[c])) {
                seen.push(who);
            }
        }
        e.choose(fallback(&req));
    }
    seen.dedup();
    assert_eq!(seen, vec!["Stone", "Tiberius", "Sweeper"]);
}

// ---- Tura Vaughn ----------------------------------------------------------------------

/// Tura Vaughn: "Tura gets +1 intercept when attempting to block other
/// Brujah." A bleeder at +1 stealth (Lost in Crowds): a Brujah blocked, a
/// Ventrue not.
#[test]
fn tura_vaughn_blocks_brujah_with_more_intercept() {
    for (clan, blocked) in [("Brujah", true), ("Ventrue", false)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain(clan, 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["obf"];
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let t = e.add_minion(SeatId(1), find_card("Tura Vaughn"), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        hand(&mut e, 0, &["Lost in Crowds"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(t));
        pick_when(&mut e, play(0, 0, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool == 10, blocked, "{clan}");
    }
}

// ---- Tusk, The Talebearer -------------------------------------------------------------

/// Tusk: "While Tusk is ready, you may look at the top card in your
/// library." His controller's view keeps the library's top on top, locked
/// too; another seat's view hides it; in torpor, sorted again.
#[test]
fn tusk_shows_his_controller_the_library_top_while_ready() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), card_adv("Tusk, The Talebearer", false), 4, false);
    e.add_minion(SeatId(1), plain4(0), 4, false);
    let (a, b) = (find_library("Conditioning"), find_library("Lost in Crowds"));
    let (high, low) = (a.max(b), a.min(b));
    let library = &mut e.state.seats[0].library;
    library.push(low);
    library.push(high);
    library.push(low);
    let top = |e: &Engine, viewer: u8| {
        e.state.legal_view(SeatId(viewer)).seats[0]
            .library
            .last()
            .copied()
    };
    assert_eq!(top(&e, 0), Some(low), "the top seen");
    e.state.minions[t.index()].locked = true;
    assert_eq!(top(&e, 0), Some(low), "locked, still ready");
    assert_eq!(top(&e, 1), Some(zvtes::state::HIDDEN));
    e.state.minions[t.index()].in_torpor = true;
    assert_eq!(top(&e, 0), Some(high), "sorted: the top unknown");
}

/// Merged Tusk: the base card's look at the top while ready and the
/// advanced card's burn of the top card as his action is announced.
#[test]
fn tusk_merged_keeps_both_texts() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), card_adv("Tusk, The Talebearer", true), 4, false);
    e.state.minions[t.index()].merged = Some(card_adv("Tusk, The Talebearer", false));
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let (a, b) = (find_library("Conditioning"), find_library("Lost in Crowds"));
    fill_hand(&mut e, 0);
    let library = &mut e.state.seats[0].library;
    library.push(a.max(b));
    library.push(a.min(b));
    assert!(e.state.library_top_seen_by(SeatId(0), SeatId(0)));
    pick(&mut e, bleed(t));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionAsAnnounced);
    let u = *req
        .options
        .iter()
        .find(|o| uses_vampire(o, t, 0))
        .expect("the burn");
    e.choose(index_of(&req, u));
    request(&mut e);
    assert_eq!(e.state.seats[0].library.len(), 1, "the top burned");
}

// ---- Ulugh Beg, The Watcher -----------------------------------------------------------

/// Ulugh Beg: "If Ulugh is ready during your discard phase, you get an
/// additional discard phase action." Two discards in the phase while he
/// is ready, one with him in torpor.
#[test]
fn ulugh_beg_gives_a_discard_phase_action_while_ready() {
    for (torpor, n) in [(false, 2), (true, 1)] {
        let mut e = game(2);
        e.add_minion(SeatId(0), find_card("Ulugh Beg, The Watcher"), 4, torpor);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        fill_hand(&mut e, 0);
        let mut discards = 0;
        loop {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                break;
            }
            if req.seat == SeatId(0) && req.options.contains(&Choice::Discard(0)) {
                discards += 1;
                pick(&mut e, Choice::Discard(0));
            } else {
                e.choose(fallback(&req));
            }
        }
        assert_eq!(discards, n, "torpor {torpor}");
    }
}

// ---- Uriah Winter ---------------------------------------------------------------------

/// Seat 0's Uriah Winter (locked, or in torpor), seat 0's pool 20, its
/// prey's `prey_pool`; seat 0's next unlock phase passed.
fn uriah(torpor: bool, prey_pool: i32) -> (Engine, MinionId) {
    let mut e = game(2);
    let u = e.add_minion(SeatId(0), find_card("Uriah Winter"), 1, torpor);
    e.state.minions[u.index()].locked = true;
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    e.state.seats[0].pool = 20;
    e.state.seats[1].pool = prey_pool;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    (e, u)
}

/// Uriah Winter: "If your prey has more pool than you do at the beginning
/// of your unlock phase, he or she takes control of Uriah during your
/// unlock phase." A richer prey takes him, unlocked; an equal one does not.
#[test]
fn uriah_winter_defects_to_a_richer_prey() {
    let (e, u) = uriah(false, 25);
    let m = &e.state.minions[u.index()];
    assert_eq!(m.owner, SeatId(1));
    assert!(!m.locked && m.in_play);
    let (e, u) = uriah(false, 20);
    assert_eq!(e.state.minions[u.index()].owner, SeatId(0));
}

/// Uriah Winter in torpor: [RTR 19941109] "Can still defect if in torpor,
/// and will remain in torpor if he does so." [LSJ 20011022] "The passive
/// ability applies even if the vampire is in torpor."
#[test]
fn uriah_winter_defects_from_torpor() {
    let (e, u) = uriah(true, 25);
    let m = &e.state.minions[u.index()];
    assert_eq!(m.owner, SeatId(1));
    assert!(m.in_torpor);
}

// ---- Valerius Maior, Hell's Fool --------------------------------------------------

/// Valerius Maior (base, 4 blood) bleeds holding `cards`, blocked by a
/// 6-blood vampire; the requests seat 0 gets to strike in, its first
/// strike `first` (a hand index played by its [THA] mode, or the hand
/// strike), each later one the first Theft of Vitae offered.
fn valerius_strikes(first: Option<u8>) -> Vec<Vec<Choice>> {
    let mut e = game(2);
    let v = e.add_minion(
        SeatId(0),
        card_adv("Valerius Maior, Hell's Fool", false),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 7, 0), 6, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    hand(
        &mut e,
        0,
        &["Theft of Vitae", "Theft of Vitae", "Theft of Vitae"],
    );
    bleed_blocked(&mut e, v, z);
    let mut seen = Vec::new();
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            return seen;
        }
        if req.seat == SeatId(0) && req.window == WindowLabel::CombatStrike {
            let theft = req
                .options
                .iter()
                .find(|o| matches!(o, Choice::Play { mode: 1, .. }))
                .copied();
            let pick_ = match (seen.is_empty(), first, theft) {
                (true, Some(_), Some(t)) | (false, _, Some(t)) => t,
                _ => req.options[fallback(&req)],
            };
            seen.push(req.options.clone());
            e.choose(index_of(&req, pick_));
            continue;
        }
        e.choose(fallback(&req));
    }
}

/// Valerius Maior: "If Valerius strikes with a strike that requires Blood
/// Sorcery [tha], he gains an optional additional strike (limited), only
/// usable to strike with a strike that requires Blood Sorcery." Theft of
/// Vitae [THA] struck, a second strike follows offering only Theft of
/// Vitae and a pass; struck with it, no third (the limit spent).
#[test]
fn valerius_maior_gains_a_blood_sorcery_strike() {
    let seen = valerius_strikes(Some(0));
    assert_eq!(seen.len(), 2, "{seen:?}");
    let second = &seen[1];
    assert!(second.contains(&Choice::Pass), "{second:?}");
    assert!(
        second
            .iter()
            .all(|o| matches!(o, Choice::Pass | Choice::Play { .. })),
        "{second:?}"
    );
    assert!(second.iter().any(|o| matches!(o, Choice::Play { .. })));
}

/// Valerius Maior's hand strike requires no Blood Sorcery: no additional
/// strike.
#[test]
fn valerius_maior_hand_strike_grants_nothing() {
    assert_eq!(valerius_strikes(None).len(), 1);
}

/// Valerius Maior (advanced): "If Valerius attempts to block, the acting
/// minion cannot play action modifier or combat cards that require
/// Chimerstry [chi] or Obfuscate [obf]." [LSJ 20050607-1] [LSJ 20080818]
/// "The prohibition to play certain cards applies from the moment he
/// attempts to block until the end of the action, even if he fails to
/// block." After his attempt Lost in Crowds [obf] is not offered, Earth
/// Control [pro] is, and the attempt fails: Lost in Crowds never offered
/// to the action's end. Another blocker's attempt: Lost in Crowds offered.
#[test]
fn valerius_maior_advanced_bars_chimerstry_and_obfuscate_after_his_attempt() {
    for valerius in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain4(0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["obf", "pro"];
        e.add_minion(SeatId(0), plain4(1), 4, false);
        let blocker = if valerius {
            card_adv("Valerius Maior, Hell's Fool", true)
        } else {
            plain4(2)
        };
        let v = e.add_minion(SeatId(1), blocker, 4, false);
        e.add_minion(SeatId(1), plain4(3), 4, false);
        hand(&mut e, 0, &["Lost in Crowds", "Earth Control"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(v));
        let req = offering(&mut e, play(1, 0, a));
        assert_eq!(
            req.options.contains(&play(0, 0, a)),
            !valerius,
            "{:?}",
            req.options
        );
        if !valerius {
            continue;
        }
        e.choose(index_of(&req, play(1, 0, a)));
        never_offered_where(
            &mut e,
            SeatId(0),
            WindowLabel::MinionPhase,
            |o| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == a),
        );
        assert_eq!(e.state.seats[1].pool, 10 - 1, "the attempt failed");
    }
}

/// Valerius Maior (advanced) blocks: the acting minion's Apparition [chi]
/// is never offered in the combat; blocked by another vampire, it is.
#[test]
fn valerius_maior_advanced_bars_combat_cards_in_his_block() {
    for (valerius, offered) in [(true, false), (false, true)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain4(0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["chi"];
        e.add_minion(SeatId(0), plain4(1), 4, false);
        let blocker = if valerius {
            card_adv("Valerius Maior, Hell's Fool", true)
        } else {
            plain4(2)
        };
        let z = e.add_minion(SeatId(1), blocker, 4, false);
        e.add_minion(SeatId(1), plain4(3), 4, false);
        hand(&mut e, 0, &["Apparition"]);
        bleed_blocked(&mut e, a, z);
        assert_eq!(
            offered_before(&mut e, play(0, 0, a), SeatId(0), WindowLabel::MinionPhase),
            offered,
            "Valerius {valerius}"
        );
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10, "blocked");
    }
}

/// Merged Valerius Maior: "[MERGED] Independent: Valerius becomes
/// non-infernal and non-Red List as he merges. While merged, his capacity
/// is reduced by 2." Independent, neither infernal nor Red List, capacity
/// 5, his 7 blood drained to 5; the base card's strike and the advanced
/// card's bar both his.
#[test]
fn valerius_maior_merged_is_independent_and_neither_infernal_nor_red_list() {
    use zvtes::crypt::Sect;
    let mut e = game(2);
    let adv = card_adv("Valerius Maior, Hell's Fool", true);
    let v = e.add_minion(SeatId(0), adv, 7, false);
    e.add_minion(SeatId(1), plain4(0), 4, false);
    assert!(e.state.is_infernal(v) && e.state.is_red_list(v));
    assert_eq!(e.state.sect_of(v), Some(Sect::Sabbat));
    e.state.minions[v.index()].merged = Some(card_adv("Valerius Maior, Hell's Fool", false));
    request(&mut e);
    assert_eq!(e.state.sect_of(v), Some(Sect::Independent));
    assert!(!e.state.is_infernal(v));
    assert!(!e.state.unlocks_as_infernal(v));
    assert!(!e.state.is_red_list(v));
    assert_eq!(e.state.capacity_of(v), Some(5));
    assert_eq!(blood(&e, v), 5);
    let d = zvtes::cards::vampire_of(&e.state.minions[v.index()]).expect("merged");
    assert_eq!((d.printed.len(), d.riders.len()), (2, 1));
}

// ---- Vardar Vardarian -----------------------------------------------------------------

/// Vardar Vardarian: "Ministries get -1 strength in combat with Vardar."
/// A Ministry's hand strike on him deals nothing, a Brujah's 1. Blood
/// cursed generated.
#[test]
fn vardar_vardarian_weakens_ministry_opponents() {
    let vardar = find_card("Vardar Vardarian");
    assert!(CRYPT[vardar as usize].blood_cursed);
    for (clan, lost) in [("Ministry", 0), ("Brujah", 1)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain(clan, 0), 4, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let v = e.add_minion(SeatId(1), vardar, 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        bleed_blocked(&mut e, a, v);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, v), 4 - lost, "{clan}");
    }
}

// ---- Wynn ---------------------------------------------------------------------------

/// Wynn: "Wynn may enter combat with a vampire controlled by your predator
/// or prey as a +1 stealth Ⓓ action." At a table of four: the prey's and
/// the predator's ready vampires offered, the cross-table one and a torpid
/// one not; the action at stealth 1, the combat with the target.
#[test]
fn wynn_rushes_a_neighbours_vampire_with_stealth() {
    let mut e = game(4);
    let w = e.add_minion(SeatId(0), find_card("Wynn"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let prey = e.add_minion(SeatId(1), plain4(1), 4, false);
    let torpid = e.add_minion(SeatId(1), plain4(2), 4, true);
    let across = e.add_minion(SeatId(2), plain4(3), 4, false);
    let pred = e.add_minion(SeatId(3), plain4(4), 4, false);
    let req = request(&mut e);
    let at = |m| vampire_action(w, 0, Target::Minion(m));
    assert!(req.options.contains(&at(prey)));
    assert!(req.options.contains(&at(pred)));
    assert!(!req.options.contains(&at(across)));
    assert!(!req.options.contains(&at(torpid)));
    e.notes = Some(Vec::new());
    pick(&mut e, at(prey));
    request(&mut e);
    assert_eq!(e.action().expect("rush").stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let begins = noted_combats(&mut e).into_iter().find_map(|n| match n {
        zvtes::combat::CombatNote::Begins { minions } => Some(minions),
        _ => None,
    });
    assert_eq!(begins, Some([w, prey]));
}

// ---- Zack North, Dancin' Dana ------------------------------------------------------

/// Zack North and Dancin' Dana: "Camarilla: +1 strength." A hand strike
/// for 2.
#[test]
fn zack_north_and_dancin_dana_strike_for_two() {
    for name in ["Zack North", "Dancin' Dana"] {
        assert_eq!(hand_strike_damage(card_adv(name, false)), (2, 0), "{name}");
    }
}

/// Merged Zack North: [ANK 20180524] "[MERGED] Is Anarch. If the base
/// version was the one in play, it changes sect as it is merged." Anarch,
/// his hand strike for 2; [RTR 19960112] the aggravating burn "can be
/// played after both strikes have been chosen": used then, his strike's
/// damage aggravated, not offered again that round.
#[test]
fn zack_north_merged_is_anarch_with_aggravated_hands() {
    use zvtes::crypt::Sect;
    let mut e = game(2);
    let z = e.add_minion(SeatId(0), card_adv("Zack North", true), 4, false);
    e.state.minions[z.index()].merged = Some(card_adv("Zack North", false));
    assert_eq!(e.state.sect_of(z), Some(Sect::Anarch));
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(1), plain4(2), 4, false);
    hand(&mut e, 1, &["Relentless Reaper"]);
    bleed_blocked(&mut e, z, x);
    let chosen = |e: &Engine| {
        e.combat()
            .is_some_and(|c| c.sides.iter().all(|s| s.strike.is_some()))
    };
    let u = loop {
        let req = request(&mut e);
        match req.options.iter().find(|o| uses_vampire(o, z, 0)) {
            Some(&u) if chosen(&e) => break u,
            _ => e.choose(fallback(&req)),
        }
    };
    pick(&mut e, u);
    assert_eq!(blood(&e, z), 3);
    let req = offering(&mut e, play(0, 0, x));
    assert!(!req.options.iter().any(|o| uses_vampire(o, z, 0)));
    assert_eq!(pending(&e, 1), (0, 2));
}

/// Merged Dancin' Dana: [ANK 20180524] "[MERGED] Is Anarch." and "While
/// Dancin' Dana is in combat, you get +1 hand size.": Anarch, hand size 7
/// out of combat, 8 while she fights.
#[test]
fn dancin_dana_merged_is_anarch_and_holds_more_in_combat() {
    use zvtes::crypt::Sect;
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), card_adv("Dancin' Dana", true), 4, false);
    e.state.minions[d.index()].merged = Some(card_adv("Dancin' Dana", false));
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    assert_eq!(e.state.sect_of(d), Some(Sect::Anarch));
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
    hand(&mut e, 0, &["Fake Out"]);
    bleed_blocked(&mut e, d, x);
    offering(&mut e, play(0, 0, d));
    assert!(e.combat().is_some());
    assert_eq!(e.state.hand_size(SeatId(0)), 8);
    assert_eq!(e.state.hand_size(SeatId(1)), 7);
}

// ---- Count Germaine ---------------------------------------------------------------

/// Count Germaine (advanced): "Germaine gets +2 bleed against a Methuselah
/// who controls a titled non-Independent, non-Anarch vampire." A torpid
/// Camarilla primogen: bleed 3; an Anarch baron or an untitled Camarilla
/// vampire: 1. Red List generated.
#[test]
fn count_germaine_advanced_bleeds_more_against_a_titled_sect_vampire() {
    use zvtes::crypt::Sect;
    let germaine = card_adv("Count Germaine", true);
    assert!(CRYPT[germaine as usize].red_list);
    let primogen = fixture_vampire(
        |c| c.title == Title::Primogen && c.sect == Some(Sect::Camarilla),
        0,
    );
    let baron = fixture_vampire(
        |c| matches!(c.title, Title::Baron(_)) && c.sect == Some(Sect::Anarch),
        0,
    );
    let untitled = fixture_vampire(
        |c| c.title == Title::None && c.sect == Some(Sect::Camarilla) && c.capacity >= 4,
        0,
    );
    for (card, torpor, bled) in [(primogen, true, 3), (baron, false, 1), (untitled, false, 1)] {
        let mut e = game(2);
        let g = e.add_minion(SeatId(0), germaine, 4, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), card, 4, torpor);
        pick(&mut e, bleed(g));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.seats[1].pool,
            10 - bled,
            "{}",
            CRYPT[card as usize].name
        );
    }
}

/// Merged Count Germaine: Anarch ("[MERGED] Anarch:", generated), the base
/// card's weapons bar and Toreador rush, the advanced card's titled bleed,
/// and "[MERGED] Germaine gets 1 optional press each combat."
#[test]
fn count_germaine_merged_keeps_both_cards_text() {
    use zvtes::cards::Stat;
    use zvtes::crypt::Sect;
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), card_adv("Count Germaine", true), 4, false);
    let press = |e: &Engine| {
        e.state
            .printed_mod(g, Stat::CombatPress, Default::default())
    };
    assert_eq!(
        (e.state.sect_of(g), press(&e)),
        (Some(Sect::Independent), 0)
    );
    e.state.minions[g.index()].merged = Some(card_adv("Count Germaine", false));
    assert_eq!((e.state.sect_of(g), press(&e)), (Some(Sect::Anarch), 1));
    assert_eq!(
        e.state
            .printed_mod(g, Stat::OpponentsNoWeapons, Default::default()),
        1
    );
    let d = zvtes::cards::vampire_of(&e.state.minions[g.index()]).expect("merged");
    assert_eq!((d.printed.len(), d.abilities.len()), (3, 1));
}

// ---- Tariq, The Silent ------------------------------------------------------------

/// Tariq, The Silent: "While Tariq is controlled, he gets -4 capacity."
/// [ANK 20190522] "His capacity is reduced immediately when he get
/// controlled": in play with 7 blood, capacity 3, the excess drained. Black
/// Hand and Red List generated; [LSJ 20070322] [ANK 20180807] "The trait is
/// not lost if the vampire changes sect."
#[test]
fn tariq_loses_four_capacity_as_he_is_controlled() {
    let tariq = find_card("Tariq, The Silent");
    assert!(CRYPT[tariq as usize].red_list && CRYPT[tariq as usize].capacity == 7);
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), tariq, 7, false);
    e.add_minion(SeatId(1), plain4(0), 4, false);
    request(&mut e);
    assert_eq!(e.state.capacity_of(t), Some(3));
    assert_eq!(blood(&e, t), 3);
    e.state.minions[t.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    assert!(e.state.is_black_hand(t));
}

/// Tariq: "He can strike, ranged: steal 1 blood or life (becoming blood)."
/// At 2 blood he steals 1 from the blocker, taking its hand strike.
#[test]
fn tariq_steals_a_blood_at_range() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Tariq, The Silent"), 2, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    bleed_blocked(&mut e, t, x);
    let u = vampire_use(&mut e, t, 0);
    pick(&mut e, u);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, t), blood(&e, x)), (2 + 1 - 1, 4 - 1));
}

// ---- Merged sects -----------------------------------------------------------------

/// A merged vampire takes the sect its [MERGED] text prints over the
/// advanced card's (ANK 20180524): Jeremy MacNeil ("[MERGED] Anarch Baron
/// of Los Angeles.") merged through Gift of Experience is an Anarch baron,
/// his anarch token burned, not Independent.
#[test]
fn jeremy_macneil_merged_is_an_anarch_baron() {
    use zvtes::crypt::Sect;
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), card_adv("Jeremy MacNeil", false), 4, false);
    e.state.minions[j.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(1), plain4(0), 4, false);
    e.state.seats[0].crypt.put(
        card_adv("Jeremy MacNeil", true),
        zvtes::copies::CopyId::NONE,
    );
    hand(&mut e, 0, &["Gift of Experience"]);
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(j),
        },
    );
    request(&mut e);
    let m = &e.state.minions[j.index()];
    assert!(m.merged.is_some());
    assert_eq!(m.sect_override, None, "the token gone");
    assert_eq!(e.state.sect_of(j), Some(Sect::Anarch));
    assert_eq!(e.state.title_of(j), Title::Baron("Los Angeles"));
}

// ---- Fix pass helpers --------------------------------------------------------

/// Whether a combat began since the notes were turned on.
fn combat_began(e: &mut Engine) -> bool {
    noted_combats(e)
        .into_iter()
        .any(|n| matches!(n, zvtes::combat::CombatNote::Begins { .. }))
}

// ---- Don Cruez -----------------------------------------------------------------

/// Don Cruez: "Once each combat, Don Cruez may burn 1 blood to get one
/// maneuver." The blood a cost (rules-digest D3): with none, never offered
/// in his combat; "once each combat" (D32, rules-qa 14.2): used in a first
/// combat, a second combat the same turn offers it again (1 blood more).
#[test]
fn don_cruez_maneuver_needs_a_blood_and_comes_back_each_combat() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Don Cruez, The Idealist"), 1, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    bleed_blocked(&mut e, d, x);
    e.state.minions[d.index()].blood = 0;
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, d, 0)
    });

    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Don Cruez, The Idealist"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 6, false);
    let y = e.add_minion(SeatId(1), plain4(2), 6, false);
    bleed_blocked(&mut e, d, x);
    let u = vampire_use(&mut e, d, 0);
    pick(&mut e, u);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let spent = blood(&e, d);
    assert!(spent < 4, "the first combat's blood");
    let v = &mut e.state.minions[d.index()];
    v.locked = false;
    v.acted.clear();
    v.bled_this_turn = false;
    bleed_blocked(&mut e, d, y);
    let u = vampire_use(&mut e, d, 0);
    let c = e.combat().expect("the second combat");
    assert!(c.sides.iter().any(|s| s.minion == y));
    pick(&mut e, u);
    assert_eq!(blood(&e, d), spent - 1, "1 blood again");
}

// ---- Doyle Fincher -------------------------------------------------------------

/// Doyle Fincher: "During your discard phase, if any of your prey's ready
/// vampires are not at full capacity, move 1 blood from the blood bank to
/// one of those vampires and 1 blood from the blood bank to Doyle." No
/// "if ready" on Doyle: in torpor too (rules-qa 16.5, 111.43; D25); Doyle
/// at full capacity, the prey's vampire still gains and Doyle's blood
/// drains off as usual (41.8).
#[test]
fn doyle_fincher_feeds_from_torpor_and_at_full_capacity() {
    let cap = zvtes::crypt::CRYPT[find_card("Doyle Fincher") as usize].capacity;
    for (torpor, start) in [(true, 1), (false, cap)] {
        let mut e = game(2);
        let d = e.add_minion(SeatId(0), find_card("Doyle Fincher"), start, torpor);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let low = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 1), 2, false);
        pick(&mut e, Choice::Pass);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, low), 3, "torpor {torpor}: the prey's vampire");
        assert_eq!(
            blood(&e, d),
            (start + 1).min(cap),
            "torpor {torpor}: Doyle's, capped"
        );
    }
}

// ---- Dr. Jest -----------------------------------------------------------------

/// Dr. Jest: "you may make your prey discard one card at random from his or
/// her hand (and draw back up to his or her hand size afterward)." The prey
/// with an empty hand: still offered, a legal no-op (D14, rules-qa 17.4):
/// nothing discarded.
#[test]
fn dr_jest_is_offered_against_an_empty_hand() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Dr. Jest"), 2, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.seats[1].hand.clear();
    let u = vampire_use(&mut e, j, 0);
    assert_eq!(e.state.active, SeatId(0));
    assert!(e.state.seats[1].hand.is_empty());
    pick(&mut e, u);
    request(&mut e);
    assert!(e.state.seats[1].ash.is_empty(), "nothing discarded");
}

// ---- Ebanezer Roush ------------------------------------------------------------

/// Ebanezer Roush: "Ebanezer may enter combat with a vampire of capacity
/// below 4 …". Re-read at resolution (D1, D13): the target's capacity
/// raised to 4 during the action (a Celerity master card put on it), the
/// action fizzles: no combat.
#[test]
fn ebanezer_roush_fizzles_once_the_target_grows_to_four() {
    let mut e = game(2);
    let eb = e.add_minion(SeatId(0), find_card("Ebanezer Roush"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let small = e.add_minion(SeatId(1), textless(|c| c.capacity == 3, 0), 3, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    e.notes = Some(Vec::new());
    pick(&mut e, vampire_action(eb, 0, Target::Minion(small)));
    request(&mut e);
    in_play(&mut e, 1, "Celerity", Some(small));
    assert_eq!(e.state.capacity_of(small), Some(4));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!combat_began(&mut e), "no combat");
    assert_eq!(e.last_outcome, Some(Outcome::Fizzled));
}

// ---- Elihu --------------------------------------------------------------------

/// The damage pending on a [cel][for] blocker of Elihu's who dodges (Side
/// Strike) his strike `ability` with `weapon`, read as the blocker may
/// prevent it (Relentless Reaper).
fn elihu_strike_through_dodge(weapon: &str, ability: Option<u8>) -> (u8, u8) {
    let mut e = game(2);
    let el = e.add_minion(SeatId(0), find_card("Elihu"), 6, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let z = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["cel", "for"];
    e.add_minion(SeatId(1), plain4(2), 4, false);
    let w = in_play(&mut e, 0, weapon, Some(el));
    hand(&mut e, 1, &["Side Strike", "Relentless Reaper"]);
    bleed_blocked(&mut e, el, z);
    match ability {
        Some(i) => pick_when(&mut e, card_ability(w, i)),
        None => pick_when(&mut e, Choice::HandStrike),
    }
    pick_when(&mut e, play(0, 0, z));
    offering(&mut e, play(0, 0, z));
    pending(&e, 1)
}

/// Elihu: "His strikes with melee weapons cannot be dodged." A melee
/// weapon's ranged strike is a melee weapon strike (LSJ 20051220, rules-qa
/// 124.128: Kerrie; Dagger alike), and a hand strike with Bundi is one too
/// (LSJ 20071020, LSJ 20090114): each lands through the dodge, Elihu's +2
/// on it.
#[test]
fn elihu_melee_weapons_ranged_and_hand_strikes_cannot_be_dodged() {
    assert_eq!(elihu_strike_through_dodge("Dagger", Some(1)), (3, 0));
    assert_eq!(elihu_strike_through_dodge("Kerrie", Some(1)), (3, 0));
    assert_eq!(elihu_strike_through_dodge("Bundi", Some(0)), (4, 0));
}

// ---- Ethan Locke --------------------------------------------------------------

/// Ethan Locke: "steal a Discipline master card from another vampire …
/// if he does not already possess the superior level of that Discipline."
/// A torpid vampire's card is taken ("from another vampire", D4); Ethan
/// gaining the superior level before resolution, the steal fizzles (D1):
/// the card stays.
#[test]
fn ethan_locke_steals_from_torpor_and_fizzles_once_superior() {
    for torpid in [true, false] {
        let mut e = game(2);
        let et = e.add_minion(SeatId(0), find_card("Ethan Locke"), 4, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let z = e.add_minion(SeatId(1), plain4(1), 4, torpid);
        e.add_minion(SeatId(1), plain4(2), 4, false);
        let cel = in_play(&mut e, 1, "Celerity", Some(z));
        pick(&mut e, vampire_action(et, 0, Target::Card(cel)));
        request(&mut e);
        if !torpid {
            e.state.minions[et.index()].gained_levels.push("cel");
            assert_eq!(e.state.discipline_level(et, "cel"), 2);
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let host = e.state.cards[cel.index()].host;
        if torpid {
            assert_eq!(host, Some(et), "taken from torpor");
        } else {
            assert_eq!(host, Some(z), "fizzled");
            assert_eq!(e.last_outcome, Some(Outcome::Fizzled));
        }
    }
}

// ---- Heather Florent ------------------------------------------------------------

/// Heather Florent: "If you gain pool from the Edge, you may move it to
/// Heather." At capacity: still offered, a legal no-op (D14, rules-qa
/// 17.4), the blood draining off as any gain (41.8): Heather stays at
/// capacity, the pool not gained.
#[test]
fn heather_florent_at_capacity_still_takes_the_edge_pool() {
    let card = find_card("Heather Florent, The Opportunist");
    let cap = zvtes::crypt::CRYPT[card as usize].capacity;
    let mut e = game(2);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let h = e.add_minion(SeatId(1), card, cap, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    e.state.seats[1].has_edge = true;
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, Choice::BurnEdge);
    let req = request(&mut e);
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::ResolvePrinted { minion: h }]
    );
    pick(&mut e, Choice::ResolvePrinted { minion: h });
    request(&mut e);
    assert_eq!((blood(&e, h), e.state.seats[1].pool), (cap, 10));
}

// ---- Janni --------------------------------------------------------------------

/// The damage Janni's strike `ability` with `weapon` puts on a [for]
/// bleeder holding Relentless Reaper she blocks: (normal, aggravated).
fn janni_damage(weapon: &str, ability: u8) -> (u8, u8) {
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), plain4(1), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(0), plain4(2), 4, false);
    let j = e.add_minion(SeatId(1), find_card("Janni"), 4, false);
    e.add_minion(SeatId(1), plain4(0), 4, false);
    let w = in_play(&mut e, 1, weapon, Some(j));
    hand(&mut e, 0, &["Relentless Reaper"]);
    bleed_blocked(&mut e, x, j);
    pick_when(&mut e, card_ability(w, ability));
    offering(&mut e, play(0, 0, x));
    pending(&e, 0)
}

/// Janni: "Janni inflicts +1 damage when striking with a weapon." Bundi's
/// hand strike is "both a hand strike and a melee weapon strike" (its
/// text; LSJ 20071020): 1 strength +1 +1 = 3.
#[test]
fn janni_adds_one_to_bundis_hand_strike() {
    assert_eq!(janni_damage("Bundi", 0), (3, 0));
}

// ---- Jayne Jonestown ------------------------------------------------------------------

/// Jayne Jonestown moves seat 1's Society of Leopold onto seat 0's own
/// vampire in seat 0's unlock phase: its rider ("During his or her
/// controller's unlock phase, the vampire with this card either burns 1
/// blood or is burned") is due that same phase, its pending riders read as
/// listed (D10): the vampire burns 1 blood before the minion phase.
#[test]
fn jayne_jonestown_moved_society_bites_its_new_bearer_that_phase() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jayne Jonestown"), 2, false);
    let own = e.add_minion(SeatId(0), plain4(0), 4, false);
    let held = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    let sol = in_play(&mut e, 1, "Society of Leopold", Some(held));
    pick_when(&mut e, Choice::BurnBloodToAvoid);
    let u = vampire_use(&mut e, j, 0);
    assert_eq!(e.state.active, SeatId(0));
    let turn = e.state.turn;
    pick(&mut e, u);
    pick(&mut e, Choice::ChooseMinion(own));
    assert_eq!(e.state.cards[sol.index()].host, Some(own));
    pick_when(&mut e, Choice::BurnBloodToAvoid);
    assert_eq!(
        (e.state.active, e.state.turn),
        (SeatId(0), turn),
        "the same unlock phase"
    );
    assert_eq!(blood(&e, own), 3);
    assert!(e.state.minions[own.index()].in_play);
}

// ---- Justine --------------------------------------------------------------------------

/// Justine: "Justine does not lock when she successfully blocks a vampire
/// with a capacity below 4." An ally has no capacity: blocking one, she
/// locks (D30).
#[test]
fn justine_locks_blocking_an_ally() {
    let mut e = game(2);
    let x = e.add_ally(SeatId(0), find_library("Screamer"));
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let j = e.add_minion(SeatId(1), find_card("Justine, Elder of Dallas"), 6, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    bleed_blocked(&mut e, x, j);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[j.index()].locked);
}

// ---- Lena Rowe -------------------------------------------------------------------------

/// Lena Rowe: "Lena cannot receive any title." No "if ready": in torpor
/// too (D25).
#[test]
fn lena_rowe_takes_no_title_in_torpor() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lena Rowe"), 1, true);
    let o = e.add_minion(SeatId(0), plain4(0), 1, true);
    assert!(!e
        .state
        .may_gain_title(l, zvtes::crypt::Title::Baron("Seattle")));
    assert!(e
        .state
        .may_gain_title(o, zvtes::crypt::Title::Baron("Seattle")));
}

// ---- Tereza Rostas ---------------------------------------------------------------------

/// Tereza Rostas: "Tereza can steal the Edge as a Ⓓ action". The Edge
/// passed from the target to a third Methuselah during the action (no pool
/// card does it; set by hand): she takes it from whoever holds it, its
/// target read as declared (RTR 19980623, rules-qa 90.7).
#[test]
fn tereza_rostas_takes_the_edge_from_its_new_holder() {
    let mut e = game(3);
    let t = e.add_minion(SeatId(0), find_card("Tereza Rostas"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(2), plain4(2), 4, false);
    e.state.seats[2].has_edge = true;
    pick(&mut e, vampire_action(t, 0, Target::Seat(SeatId(2))));
    request(&mut e);
    e.state.seats[2].has_edge = false;
    e.state.seats[1].has_edge = true;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].has_edge);
    assert!(!e.state.seats[1].has_edge && !e.state.seats[2].has_edge);
    assert_eq!(blood(&e, t), 2);
}

// ---- Louhi -------------------------------------------------------------------------

/// Louhi: "Once each turn, if Louhi is ready, she can burn 1 blood to
/// cancel an action card as it is played by a vampire controlled by your
/// predator." "If ready" (rules-digest D28): locked, her predator's Govern
/// the Unaligned is hers to cancel; in torpor, never offered, the bleed
/// resolving.
#[test]
fn louhi_cancels_only_while_ready() {
    for torpor in [false, true] {
        let mut e = game(3);
        let l = e.add_minion(SeatId(1), find_card("Louhi"), 4, torpor);
        e.state.minions[l.index()].locked = !torpor;
        let v = [0, 1, 2].map(|s| {
            let m = e.add_minion(SeatId(s), plain4(s as usize), 4, false);
            e.state.minions[m.index()].gained_levels = vec!["dom"];
            m
        });
        hand(&mut e, 0, &["Govern the Unaligned"]);
        pick(&mut e, play(0, 0, v[0]));
        if torpor {
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
                uses_vampire(o, l, 0)
            });
            assert_eq!(e.state.seats[1].pool, 10 - 3, "bled");
        } else {
            let u = vampire_use(&mut e, l, 0);
            pick(&mut e, u);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, l), 3, "locked, she cancels");
            assert_eq!(e.state.seats[1].pool, 10);
        }
    }
}

// ---- Lucian -------------------------------------------------------------------------

/// Lucian: "Lucian may steal equipment as a strike." Advanced rules §4.4.1
/// "A strike's damage is fixed when strike resolution begins": his blocker
/// strikes with the .44 Magnum (2R) he steals in the same resolution — the
/// Magnum ends on Lucian and its 2 damage still lands on him. A location
/// that is equipment (Palatial Estate; advanced rules §3.17 "a location is
/// not equipment" once in play as a location) is never a target of his
/// steal.
#[test]
fn lucian_s_steal_leaves_the_stolen_weapon_s_strike_resolving() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lucian"), 6, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 6, 0), 6, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let magnum = in_play(&mut e, 1, ".44 Magnum", Some(z));
    let estate = in_play(&mut e, 1, "Palatial Estate", Some(z));
    bleed_blocked(&mut e, l, z);
    let steal = |c| Choice::UseAbility {
        source: AbilitySource::Vampire(l),
        ability: 0,
        actor: None,
        target: Target::Card(c),
    };
    let req = offering(&mut e, steal(magnum));
    assert!(!req.options.contains(&steal(estate)), "a location");
    e.choose(index_of(&req, steal(magnum)));
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(1));
    let shot = req
        .options
        .iter()
        .position(|o| uses_card(o, magnum))
        .expect("the Magnum strike");
    let before = blood(&e, l);
    e.choose(shot);
    loop {
        let req = request(&mut e);
        if e.combat().is_none_or(|c| c.round > 1) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(e.state.cards[magnum.index()].host, Some(l), "stolen");
    assert_eq!(blood(&e, l), before - 2, "the Magnum's 2 still land");
}

// ---- Magdelena Schaefer -------------------------------------------------------------

/// Magdelena Schaefer: "Strike cards that require Blood Sorcery [tha] cost
/// Magdalena an additional blood." (rules-qa 42.2: "'strike cards' are
/// cards played in a strike mode", 38.4: the mode played.) Made Anarch,
/// blocked: Diversion's [tha] strike played at the determine-range step for
/// its maneuver costs her 1 blood; Apportation's [THA] maneuver (no strike
/// mode) nothing.
#[test]
fn magdelena_pays_for_a_strike_card_played_for_its_maneuver() {
    for (card, mode, paid) in [("Diversion", 2, 1), ("Apportation", 1, 0)] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card("Magdelena Schaefer"), 2, false);
        e.state.minions[m.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let z = e.add_minion(SeatId(1), plain4(1), 4, false);
        e.add_minion(SeatId(1), plain4(2), 4, false);
        hand(&mut e, 0, &[card]);
        hand(&mut e, 1, &["Dodge"]);
        bleed_blocked(&mut e, m, z);
        let req = to_window(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
        e.choose(index_of(&req, play(0, mode, m)));
        let req = request(&mut e);
        assert_eq!(req.window, WindowLabel::CombatStrike, "{card}");
        assert_eq!(blood(&e, m), 2 - paid, "{card}");
    }
}

// ---- Mariel, Lady Thunder -------------------------------------------------------

/// Mariel: "… she may lock to end that combat and inflict 1 unpreventable
/// damage on each combatant and any retainers they have after combat
/// ends." [RTR 20020501] (ledger 3939) "If the combat continues or a new
/// combat begins, the after combat effect does not happen": as she ends
/// it, the bleeder's Psyche! [CEL] queues a new combat; its hand strikes
/// take 1 blood each, Mariel's damage none, the Owl Companions untouched.
#[test]
fn mariel_s_damage_is_lost_to_a_queued_combat() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mariel, Lady Thunder"), 4, false);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 5, 0), 5, false);
    e.state.minions[a.index()].gained_levels = vec!["cel", "cel"];
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 5, 1), 5, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    let owls = [(0, a), (1, z)].map(|(s, h)| {
        let o = in_play(&mut e, s, "Owl Companion", Some(h));
        e.state.cards[o.index()].counters = 2;
        o
    });
    hand(&mut e, 0, &["Psyche!"]);
    bleed_blocked(&mut e, a, z);
    let u = vampire_use(&mut e, m, 0);
    pick(&mut e, u);
    let psyche = play(0, 1, a);
    let mut queued = false;
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        match req.options.iter().position(|o| *o == psyche) {
            Some(i) => {
                queued = true;
                e.choose(i)
            }
            None => e.choose(fallback(&req)),
        }
    }
    assert!(queued, "Psyche! offered as her end comes");
    assert_eq!(
        (blood(&e, a), blood(&e, z)),
        (5 - 1, 5 - 1),
        "hand strikes only"
    );
    assert_eq!(owls.map(|o| e.state.cards[o.index()].counters), [2, 2]);
}

// ---- Michael diCarlo ------------------------------------------------------------

/// Michael diCarlo: "… he can set the range for the first round of the
/// resulting combat to long." rules-qa 84.1: "once a range is set no effect
/// resets it that round … an earlier setter has priority (PIB 20120214)".
/// His Fleetness [CEL] rush; he passes the before-range window, his target
/// sets close with Brick by Brick [VIS]: the range stays close, his setter
/// never offered again.
#[test]
fn michael_dicarlo_yields_to_an_earlier_range_setter() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Michael diCarlo"), 5, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.state.minions[x.index()].locked = true;
    e.state.minions[x.index()].gained_levels = vec!["vis", "vis"];
    hand(&mut e, 0, &["Fleetness"]);
    hand(&mut e, 1, &["Brick by Brick"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 1,
            actor: Some(m),
            target: Target::Minion(x),
        },
    );
    vampire_use(&mut e, m, 0);
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, play(0, 2, x));
    assert_eq!(
        e.combat().expect("combat").range,
        zvtes::combat::Range::Close
    );
    loop {
        let req = request(&mut e);
        assert!(
            !req.options.iter().any(|o| uses_vampire(o, m, 0)),
            "{req:?}"
        );
        if e.combat().is_none_or(|c| c.round > 1) {
            break;
        }
        e.choose(fallback(&req));
    }
}

// ---- Pariah ---------------------------------------------------------------------

/// Pariah: "During your master phase, discard a master card or lock
/// Pariah." Already locked, the lock is still his controller's choice, a
/// legal no-op (rules-digest D14): offered beside the discard, chosen, the
/// master card kept, Pariah locked.
#[test]
fn pariah_may_choose_the_lock_while_locked() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), find_card("Pariah"), 4, false);
    e.state.minions[p.index()].locked = true;
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    for _ in 0..5 {
        e.state.seats[0].library.push(find_library("Deflection"));
    }
    hand(&mut e, 0, &["Dreams of the Sphinx"]);
    e.at_master_phase();
    pick_when(&mut e, Choice::ResolvePrinted { minion: p });
    let req = request(&mut e);
    assert_eq!(req.options, vec![Choice::Pass, Choice::Discard(0)]);
    e.choose(index_of(&req, Choice::Pass));
    request(&mut e);
    assert!(e.state.minions[p.index()].locked);
    assert!(e.state.seats[0]
        .hand
        .contains(&find_library("Dreams of the Sphinx")));
}

// ---- Patrizia Giovanni ----------------------------------------------------------

/// Seat 0's Patrizia (5 blood), `ash` in seat 0's ash heap; another
/// vampire each.
fn patrizia_ash(ash: &[&str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let p = e.add_minion(
        SeatId(0),
        find_card("Patrizia Giovanni, Collector of Secrets"),
        5,
        false,
    );
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let z = e.add_minion(SeatId(1), plain4(1), 4, false);
    for c in ash {
        e.state.seats[0].ash.push(find_library(c));
    }
    (e, p, z)
}

/// Patrizia: "Patrizia can employ a retainer from your ash heap as an
/// action whose cost is increased by 1 blood." The named retainer gone
/// from the ash heap by resolution: no effect, her blood still paid
/// (rules-qa 41.6, Hel-Blá: "the ally gone by resolution: no effect (blood
/// paid)"); blocked: her blood unpaid — the block's combat costs her the
/// blocker's hand strike only (5 - 1), the retainer left in the ash heap.
#[test]
fn patrizia_pays_for_a_retainer_gone_and_nothing_blocked() {
    for blocked in [false, true] {
        let (mut e, p, z) = patrizia_ash(&["Ghoul Retainer", "Mr. Winthrop"]);
        let ghoul = find_library("Ghoul Retainer");
        pick(&mut e, vampire_action(p, 0, Target::None));
        pick_when(&mut e, Choice::FindCard(ghoul));
        request(&mut e);
        if blocked {
            pick_when(&mut e, block(z));
        } else {
            e.state.seats[0].ash.retain(|&c| c != ghoul);
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(retainers_on(&e, "Ghoul Retainer", p).is_empty());
        assert_eq!(e.state.seats[0].pool, 10, "the retainer's pool unpaid");
        assert_eq!(blood(&e, p), 5 - 1, "blocked {blocked}");
        assert_eq!(e.state.seats[0].ash.contains(&ghoul), blocked);
    }
}

/// Patrizia: her ash employ asks no optional cost reduction (amber, the
/// implementer's reading; printed reductions apply): Sunset Strip,
/// Hollywood ("Lock to reduce the cost to … employ a mortal retainer by 1
/// blood or pool") is never offered on her Mr. Winthrop, her 1 blood paid,
/// the Strip left unlocked.
#[test]
fn patrizia_s_ash_employ_asks_no_optional_reduction() {
    let (mut e, p, _) = patrizia_ash(&["Mr. Winthrop", "Ghoul Retainer"]);
    let strip = in_play(&mut e, 0, "Sunset Strip, Hollywood", None);
    pick(&mut e, vampire_action(p, 0, Target::None));
    pick_when(&mut e, Choice::FindCard(find_library("Mr. Winthrop")));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: 1, .. } if *c == strip)
            || *o == Choice::ResolveRider { card: strip.0 }
    });
    assert_eq!(retainers_on(&e, "Mr. Winthrop", p).len(), 1);
    assert_eq!(blood(&e, p), 5 - 1);
    assert!(!e.state.cards[strip.index()].locked);
}

// ---- Rafaele Giovanni -----------------------------------------------------------

/// Rafaele Giovanni: "Rafaele can bleed your predator as a Ⓓ action." The
/// predator is the seat named as the action is declared: seat 2 ousted
/// mid-action by its own Direct Intervention's last pool (canceling his
/// Spying Mission [OBF]; LSJ 20050607 "handle the oust before resolving
/// the bleed"), the bleed fizzles (rules-qa 24.1) — seat 1, seat 0's
/// predator now, takes the oust's 6 pool and is not bled.
#[test]
fn rafaele_s_bleed_stays_on_the_predator_declared() {
    let mut e = game(3);
    let r = e.add_minion(SeatId(0), find_card("Rafaele Giovanni"), 4, false);
    e.state.minions[r.index()].gained_levels = vec!["obf"];
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(2), plain4(2), 4, false);
    e.state.seats[2].pool = 1;
    hand(&mut e, 0, &["Spying Mission"]);
    hand(&mut e, 2, &["Direct Intervention", "Deflection"]);
    pick(&mut e, vampire_action(r, 0, Target::Seat(SeatId(2))));
    pick_when(&mut e, play(0, 1, r));
    pick_when(&mut e, master_play(0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[2].ousted);
    assert_eq!(e.state.seats[1].pool, 10 + 6, "not bled");
    assert!(!e.state.seats[0].has_edge, "no successful bleed");
}

// ---- Rex, The Necronomist -------------------------------------------------------

/// Rex: "Combat cards cost Rex +1 blood." A free combat card costs him 1,
/// offered only with it (rules-qa 124.197, Rosemarie: "a 0-cost card costs
/// her 1 blood, offered only with it"): blocked, his last blood gone by
/// then, his Dodge is never offered.
#[test]
fn rex_is_not_offered_a_free_combat_card_without_blood() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Rex, The Necronomist"), 1, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    hand(&mut e, 0, &["Dodge"]);
    // The blocker's Dodge keeps its strike a decision.
    hand(&mut e, 1, &["Dodge"]);
    bleed_blocked(&mut e, r, x);
    e.state.minions[r.index()].blood = 0;
    let mut fought = false;
    loop {
        let req = request(&mut e);
        fought |= e.combat().is_some();
        assert!(!req.options.contains(&play(0, 0, r)), "{:?}", req.window);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(fought, "blocked");
    assert!(e.state.seats[0].hand.contains(&find_library("Dodge")));
}

// ---- Petaniqua ----------------------------------------------------------------------

/// Petaniqua: "Allies and retainers cost her -1 blood or pool." A
/// blood-cost ally: Neighborhood Watch Commander (2 blood) recruited for 1.
#[test]
fn petaniqua_pays_one_blood_less_for_a_blood_cost_ally() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), card_adv("Petaniqua", false), 6, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 6, 0), 6, false);
    hand(&mut e, 0, &["Neighborhood Watch Commander"]);
    pick(&mut e, play(0, 0, p));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions.iter().any(|m| m.in_play && m.is_ally()));
    assert_eq!(blood(&e, p), 6 - 1);
    assert_eq!(e.state.seats[0].pool, 10);
}

/// A fixture vampire of capacity 4 or more printing neither Obfuscate nor
/// Presence, the `nth`.
fn plain4_no_obf_pre(nth: usize) -> u16 {
    textless(
        |c| {
            c.capacity >= 4
                && c.disciplines
                    .iter()
                    .all(|d| !["obf", "pre"].iter().any(|w| d.eq_ignore_ascii_case(w)))
        },
        nth,
    )
}

// ---- Valerius Maior, Hell's Fool --------------------------------------------

/// Valerius Maior (base, 4 blood) bleeds holding three Theft of Vitae,
/// blocked by a 6-blood vampire holding `blocker` cards (played as its
/// first strike when offered); each of seat 0's strikes is Theft of Vitae
/// [THA]. Seat 0's strike requests and the blocker's blood after combat.
fn valerius_thefts(blocker: &[&str]) -> (usize, u8) {
    let mut e = game(2);
    let v = e.add_minion(
        SeatId(0),
        card_adv("Valerius Maior, Hell's Fool", false),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain4_no_obf_pre(0), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 7, 0), 6, false);
    e.add_minion(SeatId(1), plain4_no_obf_pre(1), 4, false);
    hand(
        &mut e,
        0,
        &["Theft of Vitae", "Theft of Vitae", "Theft of Vitae"],
    );
    hand(&mut e, 1, blocker);
    bleed_blocked(&mut e, v, z);
    let mut strikes = 0;
    let mut dodged = false;
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            return (strikes, e.state.minions[z.index()].blood);
        }
        if req.window == WindowLabel::CombatStrike {
            let card = |o: &Choice| {
                matches!(
                    o,
                    Choice::Play { mode: 1, .. } | Choice::Play { card: 0, .. }
                )
            };
            if req.seat == SeatId(0) {
                strikes += 1;
                let theft = *req
                    .options
                    .iter()
                    .find(|o| matches!(o, Choice::Play { mode: 1, .. }))
                    .expect("a Theft of Vitae [THA]");
                e.choose(index_of(&req, theft));
                continue;
            }
            if !dodged {
                if let Some(d) = req.options.iter().find(|o| card(o)).copied() {
                    dodged = true;
                    e.choose(index_of(&req, d));
                    continue;
                }
            }
        }
        e.choose(fallback(&req));
    }
}

/// Valerius Maior: "If Valerius strikes with a strike that requires Blood
/// Sorcery [tha], he gains an optional additional strike (limited), only
/// usable to strike with a strike that requires Blood Sorcery." He strikes
/// with it, dodged or not (combat.rs: "after a strike card his requirements
/// take, dodged or not"; rules-qa 111.43, D29): his first Theft of Vitae
/// [THA] dodged (Dodge), the additional strike still follows and its 2
/// blood land; undodged, both land.
#[test]
fn valerius_maior_gains_his_strike_after_a_dodged_blood_sorcery_strike() {
    assert_eq!(valerius_thefts(&[]), (2, 6 - 2 - 2));
    assert_eq!(valerius_thefts(&["Dodge"]), (2, 6 - 2));
}

/// Valerius Maior (advanced): "If Valerius attempts to block, the acting
/// minion cannot play action modifier or combat cards that require
/// Chimerstry [chi] or Obfuscate [obf]." [LSJ 20011204-3] A card of either
/// Discipline requires the one used (D29; Arnulf's filter, rules-qa
/// 121.7): blocked by him, a Cathari with [obf] and [pre] is offered Kiss
/// of Cathari ("[obf] or [pre] Strike: combat ends."), played with [pre];
/// one with [obf] alone is not.
#[test]
fn valerius_maior_advanced_bars_an_either_discipline_card_by_the_one_used() {
    for (levels, offered) in [(vec!["obf", "pre"], true), (vec!["obf"], false)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain4_no_obf_pre(0), 4, false);
        {
            let m = &mut e.state.minions[a.index()];
            m.gained_levels = levels.clone();
            m.sect_override = Some(Sect::Sabbat);
            m.path_override = Some("Cathari");
        }
        e.add_minion(SeatId(0), plain4_no_obf_pre(1), 4, false);
        let v = e.add_minion(
            SeatId(1),
            card_adv("Valerius Maior, Hell's Fool", true),
            4,
            false,
        );
        e.add_minion(SeatId(1), plain4_no_obf_pre(2), 4, false);
        hand(&mut e, 0, &["Kiss of Cathari"]);
        bleed_blocked(&mut e, a, v);
        assert_eq!(
            offered_before(&mut e, play(0, 0, a), SeatId(0), WindowLabel::MinionPhase),
            offered,
            "{levels:?}"
        );
    }
}

/// Merged Valerius Maior: "[MERGED] Independent: Valerius becomes
/// non-infernal and non-Red List as he merges." A Red List mark taken on
/// the advanced Valerius before the merge (rulebook "Red List": "enter
/// combat with a marked Red List minion") leads nowhere once he merges,
/// read live (owner rule; the readings' "moot"): the hunt is offered
/// before, not after; announced before the merge and merged by
/// resolution, it fizzles — no combat, its blood not paid (rules-qa 117.3).
#[test]
fn valerius_maior_merged_moots_a_red_list_mark() {
    let adv = card_adv("Valerius Maior, Hell's Fool", true);
    let base = card_adv("Valerius Maior, Hell's Fool", false);
    let setup = || {
        let mut e = game(2);
        let h = e.add_minion(SeatId(0), plain4_no_obf_pre(0), 4, false);
        e.add_minion(SeatId(0), plain4_no_obf_pre(1), 4, false);
        let v = e.add_minion(SeatId(1), adv, 4, false);
        e.add_minion(SeatId(1), plain4_no_obf_pre(2), 4, false);
        e.state.red_list_marks.push((SeatId(0), v));
        (e, h, v)
    };
    let hunt = |h, v| Choice::TakeAction {
        minion: h,
        action: ActionKind::RedListHunt,
        target: Some(v),
    };
    let (mut e, h, v) = setup();
    assert!(
        request(&mut e).options.contains(&hunt(h, v)),
        "marked, Red List"
    );
    e.state.minions[v.index()].merged = Some(base);
    assert!(!e.state.is_red_list(v));
    assert!(!request(&mut e).options.contains(&hunt(h, v)), "merged");

    let (mut e, h, v) = setup();
    e.notes = Some(Vec::new());
    pick(&mut e, hunt(h, v));
    e.state.minions[v.index()].merged = Some(base);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(noted_combats(&mut e).is_empty(), "no combat");
    assert_eq!(blood(&e, h), 4, "fizzled for its target: nothing paid");
}

// ---- Tariq, The Silent ------------------------------------------------------

/// Tariq, The Silent: "While Tariq is controlled, he gets -4 capacity."
/// [ANK 20190522] "His capacity is reduced immediately when he get
/// controlled": uncontrolled he holds 7 counters (his printed 7); influenced
/// out, he enters play controlled at capacity 3, the 4 above drained.
#[test]
fn tariq_influenced_out_drains_to_his_controlled_capacity() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain4_no_obf_pre(0), 4, false);
    e.add_minion(SeatId(1), plain4_no_obf_pre(1), 4, false);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: find_card("Tariq, The Silent"),
            copy: Default::default(),
            counters: 7,
            banished: None,
        });
    to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    request(&mut e);
    let t = MinionId(e.state.minions.len() as u32 - 1);
    assert_eq!(
        e.state.minions[t.index()].crypt_card(),
        Some(find_card("Tariq, The Silent"))
    );
    assert!(e.state.minions[t.index()].in_play);
    assert_eq!(e.state.capacity_of(t), Some(3));
    assert_eq!(blood(&e, t), 3);
}

/// Tariq: "He can strike, ranged: steal 1 blood or life (becoming blood)."
/// Blocked by Political Ally ("mortal with 1 life. 0 strength"), he steals
/// its life: the ally burned, Tariq 1 blood more.
#[test]
fn tariq_steals_an_allys_life() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Tariq, The Silent"), 2, false);
    e.add_minion(SeatId(0), plain4_no_obf_pre(0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    e.add_minion(SeatId(1), plain4_no_obf_pre(1), 4, false);
    bleed_blocked(&mut e, t, ally);
    let u = loop {
        let req = request(&mut e);
        if let Some(o) = req.options.iter().find(|o| {
            matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(v), ability: 0, .. } if *v == t)
        }) {
            break *o;
        }
        e.choose(fallback(&req));
    };
    pick(&mut e, u);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].present(), "its life stolen");
    assert_eq!(blood(&e, t), 2 + 1);
}

// ---- Tura Vaughn -------------------------------------------------------------

/// Tura Vaughn: "Tura gets +1 intercept when attempting to block other
/// Brujah." The clan only (rules-qa 80.2's reading: "with a Malkavian (the
/// clan only)"): a Brujah antitribu bleeding at +1 stealth (Lost in
/// Crowds) is not blocked.
#[test]
fn tura_vaughn_gets_nothing_against_brujah_antitribu() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah antitribu", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obf"];
    e.add_minion(SeatId(0), plain4_no_obf_pre(0), 4, false);
    let t = e.add_minion(SeatId(1), find_card("Tura Vaughn"), 4, false);
    e.add_minion(SeatId(1), plain4_no_obf_pre(1), 4, false);
    hand(&mut e, 0, &["Lost in Crowds"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(t));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "not blocked");
}

// ---- Count Germaine ----------------------------------------------------------

/// Count Germaine (advanced): "Germaine gets +2 bleed against a Methuselah
/// who controls a titled non-Independent, non-Anarch vampire." The title is
/// the effective one (`title_of`; RTR 20180511 "Now looks for
/// non-Independant non-Anarch"): a Camarilla prince whose title is
/// contested, or a Camarilla primogen turned Sabbat (the title not of its
/// sect), counts for nothing (bleed 1); an "n votes (titled)" vampire
/// counts by its sect — Independent 1, made Camarilla 3. Merged (both
/// cards' text, 111.19), Germaine keeps the +2 against a Camarilla primogen.
#[test]
fn count_germaine_reads_the_effective_title() {
    let primogen = fixture_vampire(
        |c| c.title == Title::Primogen && c.sect == Some(Sect::Camarilla),
        0,
    );
    let prince = fixture_vampire(
        |c| matches!(c.title, Title::Prince(_)) && c.sect == Some(Sect::Camarilla),
        0,
    );
    let votes = fixture_vampire(
        |c| matches!(c.title, Title::Votes(_)) && c.sect == Some(Sect::Independent),
        0,
    );
    let cases: [(u16, Option<Sect>, bool, bool, i32); 6] = [
        (prince, None, true, false, 1),
        (prince, None, false, false, 3),
        (primogen, Some(Sect::Sabbat), false, false, 1),
        (votes, None, false, false, 1),
        (votes, Some(Sect::Camarilla), false, false, 3),
        (primogen, None, false, true, 3),
    ];
    for (card, sect, contested, merged, bled) in cases {
        let mut e = game(2);
        let g = e.add_minion(SeatId(0), card_adv("Count Germaine", true), 4, false);
        if merged {
            e.state.minions[g.index()].merged = Some(card_adv("Count Germaine", false));
        }
        e.add_minion(SeatId(0), plain4_no_obf_pre(0), 4, false);
        let x = e.add_minion(SeatId(1), card, 4, false);
        e.state.minions[x.index()].sect_override = sect;
        e.state.minions[x.index()].title_contested = contested;
        pick(&mut e, bleed(g));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.seats[1].pool,
            10 - bled,
            "{} sect {sect:?} contested {contested} merged {merged}",
            CRYPT[card as usize].name
        );
    }
}
