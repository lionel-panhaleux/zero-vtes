//! Legacy cards for TWDA decks, round 2 (`ADDED_BATCHES`, the twelfth
//! batch, slice 16n): Nephandus/Tupdog, The unnamed, Lutz Inner Circle,
//! Goratrix, the Toreador G1/G2 wall and the Ventrue grinder. Readings in
//! rules-qa Batch 62. Windows with a single option are elided, so the
//! scripts only stop where a real decision exists.

use zvtes::cards::{self, AbilitySource, Effect, Layer, Scope, Target};
use zvtes::crypt::{Sect, CRYPT};
use zvtes::decision::{Choice, WindowLabel, Zone};
use zvtes::game::{Contest, Engine};
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

fn vampire_ability(m: MinionId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: None,
        target: Target::None,
    }
}

fn vampire_ability_at(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: None,
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

/// A card already in play for `seat`, on `host` if given, with mode
/// `mode`'s layers (no play): its id.
fn in_play_mode(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>, mode: u8) -> CardId {
    let card = find_library(name);
    e.state
        .cards
        .push(CardInPlay::new(card, SeatId(seat), host, mode));
    let id = CardId(e.state.cards.len() as u32 - 1);
    for effect in cards::lookup(card).expect("definition").modes[mode as usize].effects {
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

fn in_play(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>) -> CardId {
    in_play_mode(e, seat, name, host, 0)
}

/// A card in hand (index `i`) played by `actor` at `target`.
fn play_at(card: u8, mode: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target,
    }
}

fn library(e: &mut Engine, seat: u8, name: &str, n: usize) {
    for _ in 0..n {
        e.state.seats[seat as usize]
            .library
            .push(find_library(name));
    }
}

// ---- Adana de Sforza ------------------------------------------------------------

// [PIB 20150924-2] Adana de Sforza: "Card with multiple types (eg.
// Combat/Action Modifier) are considered of the type they are played as, not
// both." Rapid Change (1 blood): played as a combat card ([PRO] strike:
// combat ends) it costs her 0; as an action modifier ([pro] +1 stealth) the
// full 1.
#[test]
fn adana_de_sforza_reduces_a_card_played_as_a_combat_card_only() {
    // As an action modifier, against Neighbor John's attempt (+1
    // intercept): the full cost.
    let mut e = game(2);
    let adana = e.add_minion(SeatId(0), find_card("Adana de Sforza"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let john = e.add_minion(SeatId(1), find_card("Neighbor John"), 4, false);
    hand(&mut e, 0, &["Rapid Change"]);
    pick(&mut e, bleed(adana));
    pick(&mut e, block(john));
    pick_when(&mut e, play(0, 0, adana));
    // Stealth 1 against intercept 1: blocked; John's hand strike takes 1.
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, adana), 5 - 1 - 1);
    // As a combat card: free.
    let mut e = game(2);
    let adana = e.add_minion(SeatId(0), find_card("Adana de Sforza"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("AUS", 0), 4, false);
    hand(&mut e, 0, &["Rapid Change"]);
    pick(&mut e, bleed(adana));
    pick(&mut e, block(z));
    pick_when(&mut e, play(0, 1, adana));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, adana), 5);
}

// [ANK 20210226] Charismatic Aura, Forced March: "The burn blood effect is
// not reduced by effects that reduce the cost of the card (eg. Adana de
// Sforza)." Adana's Charismatic Aura still burns its blood to cancel the
// grapple.
#[test]
fn adana_de_sforza_does_not_reduce_a_burn_blood_effect() {
    let mut e = game(2);
    let adana = e.add_minion(SeatId(0), find_card("Adana de Sforza"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("POT", 0), 4, false);
    hand(&mut e, 0, &["Charismatic Aura"]);
    hand(&mut e, 1, &["Immortal Grapple"]);
    pick(&mut e, bleed(adana));
    pick(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    pick(&mut e, play(0, 0, adana));
    // The blood burned, then the blocker's hand strike lands.
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, adana), 5 - 1 - 1);
}

// ---- Unmada, Anson ---------------------------------------------------------------

// Unmada: "While Unmada is ready, you get +1 discard phase action at the
// start of your discard phase, and +1 hand size." Ready, hand size 8; in
// torpor, 7.
#[test]
fn unmada_gives_hand_size_while_ready() {
    let mut e = game(2);
    let unmada = e.add_minion(SeatId(0), find_card("Unmada"), 5, false);
    assert_eq!(e.state.hand_size(SeatId(0)), 8);
    assert_eq!(e.state.printed_discard_actions(SeatId(0)), 1);
    e.state.minions[unmada.index()].in_torpor = true;
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
    assert_eq!(e.state.printed_discard_actions(SeatId(0)), 0);
}

// Anson: "If Anson is ready during your master phase, you get two master
// phase actions (instead of one)." Two masters played in one phase; in
// torpor, one.
#[test]
fn anson_gives_two_master_phase_actions() {
    for torpid in [false, true] {
        let mut e = game(2);
        let anson = e.add_minion(SeatId(0), find_card("Anson"), 5, torpid);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        let _ = anson;
        e.at_master_phase();
        hand(&mut e, 0, &["Anarch Revolt", "Elder Library"]);
        let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
        assert!(req.options.contains(&master_play(0)));
        e.choose(index_of(&req, master_play(0)));
        let req = request(&mut e);
        let second = req.window == WindowLabel::MasterPhase
            && req.options.iter().any(|o| matches!(o, Choice::Play { .. }));
        assert_eq!(second, !torpid);
    }
}

// ---- Alexandra -------------------------------------------------------------------

// [TOM 19960214] Alexandra "Can use her unlock ability at any point during
// the turn, including during a political action." [LSJ 20011022] "The
// ability can be used when in torpor." From torpor, she unlocks another
// Toreador during her own bleed; once that turn.
#[test]
fn alexandra_unlocks_another_toreador_during_an_action_even_from_torpor() {
    let mut e = game(2);
    let alexandra = e.add_minion(SeatId(0), find_card("Alexandra"), 5, true);
    let t = e.add_minion(SeatId(0), plain("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let unlock = vampire_ability_at(alexandra, 0, Target::Minion(t));
    pick(&mut e, bleed(t));
    pick_when(&mut e, unlock);
    assert!(!e.state.minions[t.index()].locked);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, unlock);
}

// [LSJ 20010702] [RTR 19941006] [LSJ 19991025] [LSJ 20100527] "If a minion
// who is attempting to block is locked, then that block will fail, unless
// they woke earlier during the action." Alexandra locks the Toreador
// attempting to block her bleed: the attempt fails, the bleed lands.
#[test]
fn alexandra_locks_a_toreador_attempting_to_block() {
    let mut e = game(2);
    let alexandra = e.add_minion(SeatId(0), find_card("Alexandra"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Toreador", 0), 4, false);
    pick(&mut e, bleed(alexandra));
    pick_when(&mut e, block(z));
    pick_when(&mut e, vampire_ability_at(alexandra, 0, Target::Minion(z)));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(pool(&e)[1], 10 - 3, "Alexandra bleeds for 3");
}

// ---- Rafael de Corazon ---------------------------------------------------------------

// Rafael de Corazon: "Any vampire contesting Rafael's title must yield
// during their unlock phase." Alexandra, Toreador Inner Circle too, may
// only yield; Rafael himself pays or yields.
#[test]
fn rafael_de_corazons_title_rivals_must_yield() {
    let mut e = game(2);
    let alexandra = e.add_minion(SeatId(0), find_card("Alexandra"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let rafael = e.add_minion(SeatId(1), find_card("Rafael de Corazon"), 5, false);
    for m in [alexandra, rafael] {
        e.state.minions[m.index()].title_contested = true;
    }
    // Rafael's own upkeep: pay or yield.
    let item = Contest::Title(rafael);
    let req = to_window(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    assert!(
        req.options.contains(&Choice::PayContest(item)),
        "{:?}",
        req.options
    );
    pick(&mut e, Choice::PayContest(item));
    // Alexandra's: yield only, at once.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let a = &e.state.minions[alexandra.index()];
    assert!(a.title_lost && !a.title_contested);
    assert!(!e.state.minions[rafael.index()].title_lost);
}

// ---- Liquidation ---------------------------------------------------------------------

// [LSJ 20070901] Liquidation "Is not playable if you have less than 7 cards
// in your library." With 7, it burns them all for 3 pool, and is not
// replaced until the discard phase.
#[test]
fn liquidation_needs_seven_library_cards() {
    for n in [6, 7] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        e.at_master_phase();
        hand(&mut e, 0, &["Liquidation"]);
        // The hand is drawn up to 7 first.
        library(&mut e, 0, "Bonding", n + 6);
        // Six cards: nothing to play, the master phase elides.
        let req = request(&mut e);
        assert_eq!(req.options.contains(&master_play(0)), n == 7);
        if n == 7 {
            e.choose(index_of(&req, master_play(0)));
            let _ = request(&mut e);
            assert_eq!(e.state.seats[0].pool, 13);
            assert_eq!(e.state.seats[0].library.len(), 0);
            assert_eq!(e.state.seats[0].ash.len(), 8);
            assert_eq!(e.state.seats[0].hand.len(), 6, "not replaced yet");
        }
    }
}

// ---- Combat vampires ---------------------------------------------------------------

/// The `nth` vampire of capacity 4 or less with no printed text.
fn small(nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| c.capacity <= 4 && cards::vampire(*i as u16).is_none())
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a small vampire")
}

/// The CRYPT index of the vampire `name` of group `g`.
fn of_group(name: &str, g: u8) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && c.group == g)
        .expect("that version") as u16
}

// [ANK 20190701] Goratrix "Can steal 2 blood or life." Blocked by
// Nephandus, he steals the ally's 2 life with his ranged strike: Nephandus
// burns, Goratrix gains 2.
#[test]
fn goratrix_steals_life_from_an_ally() {
    let mut e = game(2);
    let goratrix = e.add_minion(SeatId(0), find_card("Goratrix"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let neph = e.add_ally(SeatId(1), find_library("Nephandus"));
    pick(&mut e, bleed(goratrix));
    pick(&mut e, block(neph));
    pick_when(&mut e, vampire_ability(goratrix, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[neph.index()].in_play);
    assert_eq!(blood(&e, goratrix), 6);
}

// Omaya: "Omaya may prevent one damage each combat." Blocked by a vampire,
// she prevents its hand strike's 1 damage, once.
#[test]
fn omaya_prevents_one_damage_each_combat() {
    let mut e = game(2);
    let omaya = e.add_minion(SeatId(0), find_card("Omaya"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    pick(&mut e, bleed(omaya));
    pick(&mut e, block(z));
    pick_when(&mut e, vampire_ability(omaya, 0));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, omaya), 4);
}

// The unnamed: "The unnamed can burn 1 blood to strike: 2R aggravated
// damage. After it successfully bleeds, you can gain 2 pool." Its bleed
// lands: 2 pool, once.
#[test]
fn the_unnamed_gains_two_pool_after_a_successful_bleed() {
    let mut e = game(2);
    let unnamed = e.add_minion(SeatId(0), find_card("unnamed, The"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let gain = vampire_ability(unnamed, 1);
    pick(&mut e, bleed(unnamed));
    pick_when(&mut e, gain);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, gain);
    assert_eq!(pool(&e), vec![12, 9]);
}

// The unnamed's strike: 1 blood, 2 aggravated damage at long range too.
#[test]
fn the_unnamed_strikes_for_two_aggravated() {
    let mut e = game(2);
    let unnamed = e.add_minion(SeatId(0), find_card("unnamed, The"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    pick(&mut e, bleed(unnamed));
    pick(&mut e, block(z));
    pick_when(&mut e, vampire_ability(unnamed, 0));
    to_minion_phase(&mut e, SeatId(0));
    // Aggravated damage: the blocker goes to torpor; its hand strike took
    // 1 from the unnamed, who paid 1.
    assert!(e.state.minions[z.index()].in_torpor);
    assert_eq!(blood(&e, unnamed), 4 - 1 - 1);
}

// ---- Printed actions ---------------------------------------------------------------

// François Villon (G2): "François may steal up to 2 blood from a
// younger vampire as a +1 stealth Ⓓ action." The amount chosen at
// resolution (rules-qa 62.4): 1 or 2 of the younger vampire's 3.
#[test]
fn francois_villon_g2_steals_blood_from_a_younger_vampire() {
    for n in [1, 2] {
        let mut e = game(2);
        let francois = e.add_minion(SeatId(0), of_group("François Villon", 2), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Brujah", 1), 3, false);
        pick(
            &mut e,
            Choice::UseAbility {
                source: AbilitySource::Vampire(francois),
                ability: 0,
                actor: Some(francois),
                target: Target::Minion(z),
            },
        );
        let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        assert_eq!(req.options, vec![Choice::SetX(1), Choice::SetX(2)]);
        pick(&mut e, Choice::SetX(n));
        to_minion_phase(&mut e, SeatId(0));
        assert_eq!(blood(&e, z), 3 - n);
        assert_eq!(blood(&e, francois), 4 + n);
    }
}

// Victoria Ash: "Victoria can lock a younger ready vampire as a +1 stealth
// Ⓓ action."
#[test]
fn victoria_ash_locks_a_younger_vampire() {
    let mut e = game(2);
    let victoria = e.add_minion(SeatId(0), find_card("Victoria Ash"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), small(0), 3, false);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(victoria),
            ability: 0,
            actor: Some(victoria),
            target: Target::Minion(z),
        },
    );
    to_minion_phase(&mut e, SeatId(0));
    assert!(e.state.minions[z.index()].locked);
}

// ---- Lutz, Marcellus -----------------------------------------------------------------

/// Seat 0's `caller` plays Kine Resources Contested: `mine` points to seat
/// 0, the rest to seat 1 then seat 2; every vote in favor.
fn kine_resources(e: &mut Engine, caller: MinionId, mine: u8, prey: u8) {
    hand(e, 0, &["Kine Resources Contested"]);
    pick(e, play(0, 0, caller));
    pick(e, alloc(0, mine));
    if mine + prey < 4 {
        pick(e, alloc(1, prey));
    }
    let req = request(e);
    if req.options.contains(&votes(caller, true)) {
        pick(e, votes(caller, true));
    }
    close_polling(e);
}

// [ANK 20220808] Lutz von Hohenzollern: "His ability is applied after the
// referendum effects are applied, including any oust and pool gained from
// it." and "His ability is not applied if his controller is ousted by the
// referendum effects (reversal of previous rulings)." The prey burns 1
// more after the allocated points; with Lutz's controller ousted by them,
// none.
#[test]
fn lutz_burns_his_preys_pool_after_a_passed_referendum() {
    let mut e = game(3);
    let lutz = e.add_minion(SeatId(0), find_card("Lutz von Hohenzollern"), 5, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 1), 4, false);
    kine_resources(&mut e, lutz, 0, 3);
    assert_eq!(pool(&e), vec![10, 10 - 3 - 1, 10 - 1]);

    let mut e = game(3);
    let lutz = e.add_minion(SeatId(0), find_card("Lutz von Hohenzollern"), 5, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 1), 4, false);
    e.state.seats[0].pool = 2;
    kine_resources(&mut e, lutz, 2, 1);
    let _ = request(&mut e);
    assert!(e.state.seats[0].ousted);
    assert_eq!(e.state.seats[1].pool, 10 - 1);
}

// Marcellus: "If Marcellus is ready, political actions cost Nosferatu 1
// additional blood." A Nosferatu's Kine Resources Contested costs 1 blood;
// another clan's nothing.
#[test]
fn marcellus_makes_nosferatu_political_actions_cost_blood() {
    for (clan, cost) in [("Nosferatu", 1), ("Brujah", 0)] {
        let mut e = game(3);
        let caller = e.add_minion(SeatId(0), plain(clan, 0), 4, false);
        e.add_minion(SeatId(1), find_card("Marcellus"), 4, false);
        e.add_minion(SeatId(2), plain("Brujah", 1), 4, false);
        kine_resources(&mut e, caller, 0, 3);
        assert_eq!(blood(&e, caller), 4 - cost, "{clan}");
    }
}

// ---- Unlock phase abilities -------------------------------------------------------------

// [LSJ 20040127] Owain Evans: "His ability can only be used once each
// unlock, even if control of Owain changes during that unlock phase."
// [LSJ 20100723] "His ability can only be used when his controller has the
// opportunity to play effects. The acting player has to apply all mandatory
// effects (eg. lose pool to Anarch Revolt) before passing the opportunity
// to play effects to another player." In seat 1's unlock phase, seat 1
// first burns the pool Contagion takes, then seat 0 moves a bank blood to
// seat 1's vampire, once.
#[test]
fn owain_evans_feeds_the_active_methuselahs_vampire_once() {
    let mut e = game(2);
    let owain = e.add_minion(SeatId(0), find_card("Owain Evans, The Wanderer"), 4, false);
    let unnamed = e.add_minion(SeatId(0), find_card("unnamed, The"), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 3, false);
    in_play(&mut e, 0, "Contagion", Some(unnamed));
    let feed = vampire_ability_at(owain, 0, Target::Minion(z));
    pick(&mut e, Choice::Pass);
    let req = offering(&mut e, feed);
    assert_eq!(req.seat, SeatId(0));
    assert_eq!(e.state.seats[1].pool, 9, "the mandatory burn first");
    e.choose(index_of(&req, feed));
    let _ = request(&mut e);
    assert_eq!(blood(&e, z), 4);
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, feed);
}

// Keith Moody: "Your prey can burn the Edge during your unlock phase to
// burn 1 blood from Keith." Seat 1, seat 0's prey, holds the Edge and burns
// it in seat 0's unlock phase: Keith loses 1 blood.
#[test]
fn keith_moodys_prey_burns_the_edge_to_burn_his_blood() {
    let mut e = game(2);
    let keith = e.add_minion(SeatId(0), find_card("Keith Moody"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.state.seats[1].has_edge = true;
    let burn = vampire_ability(keith, 0);
    // Seat 0's turn ends; seat 1's passes; seat 0's unlock phase.
    let req = offering(&mut e, burn);
    assert_eq!(req.seat, SeatId(1));
    pick(&mut e, burn);
    assert_eq!(blood(&e, keith), 2);
    assert!(!e.state.seats[1].has_edge);
}

// ---- Blackhorse Tanner ----------------------------------------------------------------

// [ANK 20180719-4] Blackhorse Tanner: "'Once each turn' means once during
// each player's turn." The replacement for his master goes to the bottom
// and another card is drawn; the next draw that turn is not offered; in
// seat 1's turn, his reaction's replacement is offered again.
#[test]
fn blackhorse_tanner_redraws_once_in_each_players_turn() {
    let mut e = game(2);
    let bh = e.add_minion(SeatId(0), find_card("Blackhorse Tanner"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 2), 4, false);
    e.at_master_phase();
    hand(
        &mut e,
        0,
        &["Anarch Revolt", "Conditioning", "Eyes of Argus"],
    );
    fill_hand(&mut e, 0);
    library(&mut e, 0, "Bonding", 3);
    library(&mut e, 0, "Deflection", 1);
    library(&mut e, 0, "Govern the Unaligned", 1); // the top
    pick(&mut e, master_play(0));
    let req = expect(&mut e, SeatId(0), WindowLabel::Draw);
    assert_eq!(req.options, vec![Choice::Pass, Choice::DrawLibrary]);
    pick(&mut e, Choice::DrawLibrary);
    let seat = &e.state.seats[0];
    assert_eq!(seat.library[0], find_library("Govern the Unaligned"));
    assert!(seat.hand.contains(&find_library("Deflection")));
    // His bleed with Conditioning: its replacement is not offered.
    pick(&mut e, bleed(bh));
    pick_when(&mut e, play(0, 0, bh));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        Choice::DrawLibrary,
    );
    // Seat 1 bleeds: Blackhorse, locked, wakes with Eyes of Argus; its
    // replacement is offered again.
    let z = e.state.minions_of(SeatId(1)).next().unwrap();
    pick(&mut e, bleed(z));
    let eyes = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Eyes of Argus"))
        .unwrap() as u8;
    pick_when(&mut e, play(eyes, 1, bh));
    let req = offering(&mut e, Choice::DrawLibrary);
    assert_eq!((req.seat, req.window), (SeatId(0), WindowLabel::Draw));
}

// ---- Psychic Assault --------------------------------------------------------------------

// [LSJ 20070322] [ANK 20180807] "Black Hand is not a title, it is a trait
// unrelated to any sect. The trait is not lost if the vampire changes
// sect." Psychic Assault [aus] by Blackhorse Tanner, Black Hand: its damage
// is unpreventable — Omaya's prevention is not offered —, even once he is
// made Anarch; by a vampire with no Black Hand, Omaya prevents.
#[test]
fn psychic_assault_by_a_black_hand_vampire_is_unpreventable() {
    for black_hand in [true, false] {
        let mut e = game(2);
        let omaya = e.add_minion(SeatId(0), find_card("Omaya"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = if black_hand {
            let bh = e.add_minion(SeatId(1), find_card("Blackhorse Tanner"), 4, false);
            e.state.minions[bh.index()].sect_override = Some(Sect::Anarch);
            bh
        } else {
            e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false)
        };
        hand(&mut e, 1, &["Psychic Assault"]);
        let prevent = vampire_ability(omaya, 0);
        pick(&mut e, bleed(omaya));
        pick(&mut e, block(z));
        pick_when(&mut e, play(0, 0, z));
        let offered = offered_before(&mut e, prevent, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(offered, !black_hand);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, omaya), if black_hand { 3 } else { 4 });
    }
}

// ---- Anneke ------------------------------------------------------------------------------

// [RTR 20020501] [LSJ 20041203] Anneke: "Her ability only affects the rule
// restricting who gets to block an action. She must still meet all other
// requirements to block. In particular, Blood Bond, Day Operation,
// Seduction and prior decision not to block are not circumvented." At a
// four-seat table, seat 0 bleeds seat 1: Anneke, at seat 3 (neither target
// nor neighbour of the bleed's), may block; her Methuselah's decline stands;
// Seduction keeps her from blocking.
#[test]
fn anneke_blocks_outside_the_normal_restrictions_and_only_those() {
    // She blocks.
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 2), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 3), 4, false);
    let anneke = e.add_minion(SeatId(3), find_card("Anneke"), 4, false);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(anneke));
    assert_eq!(e.state.seats[1].pool, 10, "blocked");
    // Seduction: chosen as announced, she cannot block.
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), vampire_with("DOM", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 2), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 3), 4, false);
    let anneke = e.add_minion(SeatId(3), find_card("Anneke"), 4, false);
    hand(&mut e, 0, &["Seduction"]);
    pick(&mut e, bleed(a));
    pick(&mut e, play_at(0, 1, a, Target::Minion(anneke)));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(anneke));
    assert_eq!(e.state.seats[1].pool, 9);
    // Her Methuselah passed: no later attempt.
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 2), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 3), 4, false);
    let anneke = e.add_minion(SeatId(3), find_card("Anneke"), 4, false);
    pick(&mut e, bleed(a));
    let req = offering(&mut e, block(anneke));
    assert_eq!(req.seat, SeatId(3));
    e.choose(index_of(&req, Choice::Pass));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(anneke));
}

// ---- Antonio d'Erlette --------------------------------------------------------------------

// [LSJ 20090920-2] [RTR 20070707] Antonio d'Erlette: "Cost reductions
// apply (eg. Powerbase: Tshwane), except if they require an action (eg.
// Charisma)." With Charisma on him, the Nephandus he fetches from the
// library for 4 transfers still costs 2 pool; it is put in play, not
// recruited: it may act this turn.
#[test]
fn antonio_derlette_puts_a_mage_ally_in_play_for_transfers() {
    let mut e = game(2);
    let antonio = e.add_minion(SeatId(0), find_card("Antonio d'Erlette"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    in_play(&mut e, 0, "Charisma", Some(antonio));
    fill_hand(&mut e, 0);
    library(&mut e, 0, "Bonding", 3);
    library(&mut e, 0, "Nephandus", 1);
    library(&mut e, 0, "Bonding", 3);
    let fetch = vampire_ability(antonio, 0);
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, fetch);
    let req = expect(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    let neph = Choice::TakeFrom {
        zone: Zone::Library,
        card: find_library("Nephandus"),
    };
    assert_eq!(req.options, vec![Choice::Pass, neph]);
    pick(&mut e, neph);
    assert_eq!(e.state.seats[0].pool, 8);
    let n = e
        .state
        .minions
        .iter()
        .position(|m| m.in_play && m.name() == "Nephandus")
        .expect("in play");
    assert!(!e.state.minions[n].recruited);
    assert_eq!(e.state.minions[n].blood, 2);
    assert_eq!(e.state.seats[0].library.len(), 6);
}

/// Pass the referendum under way: the Edge, the caller's votes, its card's.
fn pass_referendum(e: &mut Engine, caller: MinionId) {
    e.state.seats[e.state.minions[caller.index()].owner.index()].has_edge = true;
    for _ in 0..200 {
        let req = request(e);
        let yes = [
            votes(caller, true),
            Choice::CastCardVote { favor: true },
            Choice::CastEdge { favor: true },
        ];
        if let Some(i) = req.options.iter().position(|o| yes.contains(o)) {
            e.choose(i);
            close_polling(e);
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("no vote");
}

// ---- Tupdog --------------------------------------------------------------------------

// Tupdog: "After the end of your minion phase, burn Tupdog and move the top
// card from your crypt to your uncontrolled region." Both Tupdogs of seat
// 0 burn, one in torpor too; two crypt cards come out.
#[test]
fn tupdogs_burn_after_their_minion_phase_and_bring_crypt_cards_out() {
    let mut e = game(2);
    let t1 = e.add_minion(SeatId(0), find_card("Tupdog"), 1, false);
    let t2 = e.add_minion(SeatId(0), find_card("Tupdog"), 0, true);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.state.seats[0].crypt = vec![
        plain("Ventrue", 0),
        plain("Ventrue", 1),
        plain("Ventrue", 2),
    ]
    .into();
    let before = e.state.seats[0].uncontrolled.len();
    pick(&mut e, Choice::Pass);
    let _ = request(&mut e);
    assert!(!e.state.minions[t1.index()].in_play);
    assert!(!e.state.minions[t2.index()].in_play);
    assert_eq!(e.state.seats[0].uncontrolled.len(), before + 2);
    assert_eq!(e.state.seats[0].crypt, vec![plain("Ventrue", 0)]);
}

// Tupdog, "Tremere antitribu slave" (rulebook "Slave"): no directed action
// — his rush — while his controller controls no ready Tremere antitribu;
// with Ember Wright ready, he may.
#[test]
fn tupdog_takes_no_directed_action_without_a_ready_tremere_antitribu() {
    for master in [false, true] {
        let mut e = game(2);
        let t = e.add_minion(SeatId(0), find_card("Tupdog"), 1, false);
        if master {
            e.add_minion(SeatId(0), find_card("Ember Wright"), 3, false);
        } else {
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        }
        let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        let rush = Choice::UseAbility {
            source: AbilitySource::Vampire(t),
            ability: 0,
            actor: Some(t),
            target: Target::Minion(z),
        };
        let req = request(&mut e);
        assert_eq!(req.options.contains(&rush), master);
        assert_eq!(req.options.contains(&bleed(t)), master);
    }
}

// Rulebook "Slave": "if a minion of that clan controlled by the same
// Methuselah is blocked, the controller can lock the slave to cancel the
// combat, unlock the acting vampire and have the slave enter combat with
// the blocker instead." Saiz's bleed is blocked: Tupdog fights the
// blocker, Saiz unlocked.
#[test]
fn tupdog_takes_the_combat_of_a_blocked_tremere_antitribu() {
    let mut e = game(2);
    let saiz = e.add_minion(SeatId(0), find_card("Saiz"), 3, false);
    let t = e.add_minion(SeatId(0), find_card("Tupdog"), 1, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    pick(&mut e, bleed(saiz));
    pick(&mut e, block(z));
    pick_when(&mut e, vampire_ability(t, 1));
    let _ = request(&mut e);
    // Tupdog fought the blocker (each hand strike 1); Saiz is unlocked and
    // untouched.
    assert!(!e.state.minions[saiz.index()].locked);
    assert_eq!(blood(&e, saiz), 3);
    assert!(e.state.minions[t.index()].locked);
    assert_eq!(blood(&e, z), 3);
}

// [ANK 20200115] Sniper Rifle: "If a Slave locks to enter combat instead of
// a blocked minion, the opponent cannot use the Rifle to set the range": the
// blocker's rifle sets it against Saiz, not against Tupdog.
#[test]
fn sniper_rifle_cannot_set_the_range_against_a_slave() {
    for slave in [false, true] {
        let mut e = game(2);
        let saiz = e.add_minion(SeatId(0), find_card("Saiz"), 3, false);
        let t = e.add_minion(SeatId(0), find_card("Tupdog"), 1, false);
        let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        let rifle = in_play(&mut e, 1, "Sniper Rifle", Some(z));
        let set_range = use_at(rifle, 1, None, Target::None);
        pick(&mut e, bleed(saiz));
        pick(&mut e, block(z));
        if slave {
            pick_when(&mut e, vampire_ability(t, 1));
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, set_range);
            assert!(e.state.minions[t.index()].locked, "Tupdog fought");
        } else {
            pick_when(&mut e, set_range);
        }
    }
}

// ---- Canine Horde ------------------------------------------------------------------------

// [RTR 19980928] Canine Horde "[ANI] Cannot be used if there is no
// equipment to destroy." Against a blocker with a Kevlar Vest: offered at
// long range, the vest burned with first strike; against one without, only
// [ani] is.
#[test]
fn canine_horde_superior_needs_an_equipment_to_destroy() {
    for armed in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("ANI", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        let vest = armed.then(|| in_play(&mut e, 1, "Kevlar Vest", Some(z)));
        hand(&mut e, 0, &["Canine Horde"]);
        pick(&mut e, bleed(a));
        pick(&mut e, block(z));
        let req = offering(&mut e, play(0, 0, a));
        let burn = vest.map(|v| play_at(0, 1, a, Target::Card(v)));
        assert_eq!(
            req.options
                .iter()
                .any(|o| matches!(o, Choice::Play { mode: 1, .. })),
            armed
        );
        if let Some(burn) = burn {
            e.choose(index_of(&req, burn));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(!e.state.cards[vest.unwrap().index()].in_play);
        }
    }
}

// ---- Ruins of Ceoris ---------------------------------------------------------------------

// [LSJ 20080114] Ruins of Ceoris: "Is an equipment while in hand, library
// or ash heap, both equipment and location when played (eg. for
// cost-related effects), and just a location while in play." [LSJ
// 20090324] "An action targeting it (eg. Conceal) is directed at the
// Methuselah controlling it, not the minion it is on." In play it is no
// equipment — Canine Horde [ANI] finds nothing to destroy —, Rewilding
// burns it as a location, its controller paying.
#[test]
fn ruins_of_ceoris_in_play_is_a_location_not_an_equipment() {
    assert!(!cards::is_equipment_in_play(find_library(
        "Ruins of Ceoris"
    )));
    let mut e = game(3);
    let g = e.add_minion(SeatId(0), plain("Gangrel", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Saiz"), 3, false);
    e.add_minion(SeatId(2), plain("Brujah", 1), 4, false);
    let ruins = in_play(&mut e, 1, "Ruins of Ceoris", Some(z));
    hand(&mut e, 0, &["Rewilding"]);
    pick(&mut e, play_at(0, 0, g, Target::Card(ruins)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[ruins.index()].in_play);
    assert_eq!(e.state.seats[1].pool, 8);
}

// [RTR 19960112] Ruins of Ceoris "May be put on any minion if the
// controller changes. If the new controller has no minions, it is burned."
// Disputed Territory gives it to seat 2, with no minion: burned.
#[test]
fn ruins_of_ceoris_given_to_a_methuselah_without_minions_burns() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), find_card("Victoria Ash"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    let y = e.add_minion(SeatId(1), find_card("Saiz"), 3, false);
    let ruins = in_play(&mut e, 1, "Ruins of Ceoris", Some(y));
    hand(&mut e, 0, &["Disputed Territory"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::ChooseSeat(SeatId(2)));
    pass_referendum(&mut e, a);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[ruins.index()].in_play);
}

// Ruins of Ceoris: "The Tremere antitribu with this location gets an
// optional maneuver or press each combat." Saiz, blocked, may maneuver or
// press, once over the combat.
#[test]
fn ruins_of_ceoris_gives_a_maneuver_or_a_press() {
    let mut e = game(2);
    let saiz = e.add_minion(SeatId(0), find_card("Saiz"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let ruins = in_play(&mut e, 0, "Ruins of Ceoris", Some(saiz));
    pick(&mut e, bleed(saiz));
    pick(&mut e, block(z));
    let maneuver = use_at(ruins, 0, None, Target::None);
    let press = use_at(ruins, 1, None, Target::None);
    pick_when(&mut e, maneuver);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, press);
}

// ---- The Unmasking ---------------------------------------------------------------------------

// [LSJ 20081120-2] [LSJ 20100121] Karen Suadela: "Her immunity does not
// extend to other things affected by events: Allies still benefit from The
// Unmasking intercept when trying to block her." Against a plain vampire's
// +1 stealth political action, Nephandus (no intercept of his own) can
// reach the stealth only with The Unmasking in play; against Karen's,
// Gehenna-immune, he still has its +1 intercept.
#[test]
fn the_unmasking_gives_allies_intercept_against_vampires() {
    for (karen, unmasking) in [(false, false), (false, true), (true, true)] {
        let mut e = game(2);
        let a = if karen {
            e.add_minion(SeatId(0), find_card("Karen Suadela"), 4, false)
        } else {
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false)
        };
        e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        let neph = e.add_ally(SeatId(1), find_library("Nephandus"));
        if unmasking {
            in_play(&mut e, 1, "Unmasking, The", None);
        }
        hand(&mut e, 0, &["Kine Resources Contested"]);
        pick(&mut e, play(0, 0, a));
        let _ = request(&mut e);
        let a = e.action().expect("the political action");
        assert_eq!(
            a.intercept_of(&e.state, neph),
            i32::from(unmasking),
            "karen {karen}"
        );
        if !karen {
            let req = offering(&mut e, block(neph));
            assert_eq!(e.block_can_reach(req.seat, neph), unmasking);
        }
    }
}

// ---- Hunting grounds and powerbases -------------------------------------------------------------

// Corporate Hunting Ground: "During your unlock phase, you may move 1 blood
// from the bank to a ready vampire you control."
#[test]
fn corporate_hunting_ground_feeds_a_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let hg = in_play(&mut e, 0, "Corporate Hunting Ground", None);
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, use_at(hg, 0, None, Target::Minion(a)));
    assert_eq!(blood(&e, a), 3);
}

// Powerbase: Barranquilla: "Put X blood on this card when it is played,
// where X is the capacity of a ready Sabbat vampire you control. During
// your unlock phase, you may move 1 blood from this card to your pool.
// Any vampire may burn this location as a Ⓓ action. Titled vampires get +1
// stealth on that action." Played for Jefferson Foster (6): 6 counters;
// one moved to the pool; seat 1's titled vampire may burn it.
#[test]
fn powerbase_barranquilla_banks_a_sabbat_vampires_capacity() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jefferson Foster"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), find_card("Victoria Ash"), 4, false);
    e.at_master_phase();
    hand(&mut e, 0, &["Powerbase: Barranquilla"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(j),
        },
    );
    let _ = request(&mut e);
    let pb = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(e.state.cards[pb.index()].counters, 6);
    // Seat 1's turn: Victoria's burn, +1 stealth for her title, read live.
    let burn = use_at(pb, 1, Some(v), Target::None);
    let req = offering(&mut e, burn);
    assert_eq!(req.seat, SeatId(1));
    // Seat 0's next unlock phase: 1 blood to the pool.
    e.choose(index_of(&req, Choice::Pass));
    pick_when(&mut e, use_at(pb, 0, None, Target::None));
    let _ = request(&mut e);
    assert_eq!(e.state.seats[0].pool, 10 - 1 + 1, "its 1 pool cost, 1 back");
    assert_eq!(e.state.cards[pb.index()].counters, 5);
}

// ---- Nephandus -------------------------------------------------------------------------------

/// Seat 0's `a` bleeds; seat 1's Nephandus (life `life`) blocks: in combat.
fn nephandus_blocks(e: &mut Engine, a: MinionId, life: u8) -> MinionId {
    let neph = e.add_ally(SeatId(1), find_library("Nephandus"));
    e.state.minions[neph.index()].blood = life;
    pick(e, bleed(a));
    pick(e, block(neph));
    neph
}

fn seat0(e: &mut Engine) -> MinionId {
    e.add_minion(SeatId(0), plain("Brujah", 2), 4, false)
}

/// The card in play that is `name`'s, the first.
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

// [ANK 20171109] Nephandus: "His life total can exceed his starting life
// capacity (it is the case for all allies)." He burns a vampire in torpor
// as a Ⓓ action: 3 life.
#[test]
fn nephandus_burns_a_torpid_vampire_to_gain_life_over_his_starting_life() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 2, true);
    e.add_minion(SeatId(1), plain("Brujah", 2), 4, false);
    let neph = e.add_ally(SeatId(0), find_library("Nephandus"));
    let id = card_in_play(&e, "Nephandus");
    pick(&mut e, use_at(id, 2, Some(neph), Target::Minion(z)));
    to_minion_phase(&mut e, SeatId(0));
    assert!(!e.state.minions[z.index()].in_play);
    assert_eq!(blood(&e, neph), 3);
}

// [LSJ 20100519] Nephandus: "Damage reduction applies to unpreventable
// damage." Psychic Assault by a Black Hand vampire, unpreventable: its 1
// damage reduced to none. [LSJ 20100519] Weighted Walking Stick: "If the
// damage is reduced (eg. Nephandus) or not inflicted (eg. Blood Fury), the
// corresponding token is not burned (contrary to prevented damage)": its
// strength+1 reduced to 1, one counter burned.
#[test]
fn nephandus_reduces_unpreventable_damage_and_weapon_damage() {
    let mut e = game(2);
    let bh = e.add_minion(SeatId(0), find_card("Blackhorse Tanner"), 4, false);
    seat0(&mut e);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    hand(&mut e, 0, &["Psychic Assault"]);
    fill_hand(&mut e, 0);
    let neph = nephandus_blocks(&mut e, bh, 2);
    pick_when(&mut e, play(0, 0, bh));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, neph), 2);

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    seat0(&mut e);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    hand(&mut e, 0, &["Weighted Walking Stick"]);
    let neph = nephandus_blocks(&mut e, a, 4);
    pick_when(&mut e, play(0, 0, a));
    let _ = request(&mut e);
    let stick = card_in_play(&e, "Weighted Walking Stick");
    pick_when(&mut e, use_at(stick, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, neph), 4 - 1);
    assert_eq!(e.state.cards[stick.index()].counters, 5 - 1);
}

// [LSJ 20080630] [ANK 20170111] Nephandus: "Damage reduction applies to
// damage done outside of or after combat if it comes from a strike (eg.
// Catatonic Fear)." Catatonic Fear [PRE]'s 1 damage after combat: none.
#[test]
fn nephandus_reduces_a_strikes_damage_after_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("PRE", 0), 4, false);
    seat0(&mut e);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    hand(&mut e, 0, &["Catatonic Fear"]);
    let neph = nephandus_blocks(&mut e, a, 2);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, neph), 2);
}

// [PIB 20111017] [LSJ 20010626] Nephandus: "Damage reduction does not apply
// to environmental damage (eg. Carrion Crows)." The crows' 1: taken; the
// hand strike's 1: reduced to none.
#[test]
fn nephandus_does_not_reduce_environmental_damage() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ani", 0), 4, false);
    seat0(&mut e);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    hand(&mut e, 0, &["Carrion Crows"]);
    let neph = nephandus_blocks(&mut e, a, 2);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, neph), 1);
}

// [LSJ 20091123-2] Nephandus: "Additional damage provided by Aim cards are
// not reduced separately." and "Damage done by a weapon is reduced if it
// is wielded by the opposing minion." Femur of Toomler, strength+1: 2
// reduced to 1; with Target Vitals, +2 more, not reduced again: 3.
#[test]
fn nephandus_reduces_a_weapon_strike_once_aims_included() {
    for aim in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Tzimisce", 0), 4, false);
        seat0(&mut e);
        e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        let femur = in_play(&mut e, 0, "Femur of Toomler", Some(a));
        if aim {
            hand(&mut e, 0, &["Target Vitals"]);
        }
        let neph = nephandus_blocks(&mut e, a, 5);
        // The strike chosen, then the aim on top (rules-qa 62.1).
        pick_when(&mut e, use_at(femur, 0, None, Target::None));
        if aim {
            pick_when(&mut e, play(0, 0, a));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, neph), if aim { 5 - 3 } else { 5 - 1 });
    }
}

// Nephandus: "Nephandus can strike: 1R damage. Nephandus gets 1 optional
// press each combat." Blocking, he strikes for 1 at range and presses
// once.
#[test]
fn nephandus_strikes_at_range_and_presses_once() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    seat0(&mut e);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let neph = nephandus_blocks(&mut e, a, 2);
    let id = card_in_play(&e, "Nephandus");
    pick_when(&mut e, use_at(id, 0, None, Target::None));
    pick_when(&mut e, use_at(id, 1, None, Target::None));
    let _ = neph;
    // The pressed round: another strike, the press spent.
    pick_when(&mut e, use_at(id, 0, None, Target::None));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_at(id, 1, None, Target::None),
    );
    assert_eq!(blood(&e, a), 4 - 2);
}

// ---- Veneficti ------------------------------------------------------------------------------

// [LSJ 20091203] Veneficti: "The burn option can be used if all your
// infernal vampires are in torpor." [LSJ 20100203-2] "The fact it is
// Unique has no bearing on the burn option. You cannot use the burn option
// if you meet the requirement, even if you already have a copy of the card
// in play." With The unnamed in torpor, discarded in the unlock phase;
// ready, not — a Veneficti in play changes nothing.
#[test]
fn veneficti_is_burnable_while_the_infernal_vampires_are_in_torpor() {
    for (torpid, copy) in [(true, false), (false, false), (false, true)] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), find_card("unnamed, The"), 4, torpid);
        e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        if copy {
            e.add_ally(SeatId(1), find_library("Veneficti"));
        }
        let v = find_library("Veneficti");
        e.state.seats[1].hand = vec![v].into();
        fill_hand(&mut e, 1);
        e.state.seats[1].library = vec![find_library("Bonding"); 2].into();
        pick(&mut e, Choice::Pass);
        let offered = offered_before(
            &mut e,
            Choice::Discard(0),
            SeatId(1),
            WindowLabel::MinionPhase,
        );
        assert_eq!(offered, torpid, "torpid {torpid}, copy {copy}");
    }
}

// Veneficti: "He cannot be the target of a directed action performed by an
// ally or a vampire with capacity 6 or less." Victoria Ash (6) cannot aim
// her lock at him... she aims at vampires only; Goratrix's rush? The
// rulebook rush of a capacity 4 vampire with Bum's Rush: not at him; a
// capacity 10's: yes.
#[test]
fn veneficti_cannot_be_targeted_by_small_vampires() {
    for big in [false, true] {
        let mut e = game(2);
        let a = if big {
            e.add_minion(SeatId(0), find_card("Goratrix"), 4, false)
        } else {
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false)
        };
        assert_eq!(e.state.capacity_of(a).unwrap() > 6, big);
        e.add_minion(SeatId(0), plain("Brujah", 3), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        let v = e.add_ally(SeatId(1), find_library("Veneficti"));
        hand(&mut e, 0, &["Bum's Rush"]);
        let req = request(&mut e);
        assert_eq!(
            req.options.contains(&play_at(0, 0, a, Target::Minion(v))),
            big,
            "big {big}"
        );
    }
}

// ---- CrimethInc. ----------------------------------------------------------------------------

// [LSJ 20070411] CrimethInc.: "Can be played if the action is unblocked
// even if it has no effect due to lack of a suitable target." [PIB
// 20150915] [LSJ 19981028] [ANK 20190425] "Is played after resolution,
// after all combats (if any) are handled, but still during the action."
// [LSJ 20011204-3] [PIB 20130704] "When played, a multi-Disciplines card
// counts as requiring the Discipline(s) being used." Elisha Tucker
// (Anarch, [pro], no [qui] nor [tha]) plays Make the Misere at a locked
// minion unlocked before resolution: it fizzles, still successful;
// CrimethInc. [pro] unlocks her, only its [pro] mode offered.
#[test]
fn crimethinc_unlocks_after_an_anarch_action_without_effect() {
    let mut e = game(2);
    let elisha = e.add_minion(SeatId(0), find_card("Elisha Tucker"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 0, &["Make the Misere", "CrimethInc."]);
    pick(&mut e, play_at(0, 0, elisha, Target::Minion(z)));
    e.state.minions[z.index()].locked = false;
    let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    assert!(e.action().is_some(), "still during the action");
    let crimethinc: Vec<u8> = req
        .options
        .iter()
        .filter_map(|o| match o {
            Choice::Play { card: 0, mode, .. } => Some(*mode),
            _ => None,
        })
        .collect();
    assert_eq!(crimethinc, vec![0], "[pro] only");
    pick(&mut e, play(0, 0, elisha));
    to_minion_phase(&mut e, SeatId(0));
    assert!(!e.state.minions[elisha.index()].locked);
}

// [LSJ 20080603] [LSJ 20050128] CrimethInc.: "An action requiring a Baron
// (eg. Open War) is an action requiring an Anarch." Elisha Tucker, baron,
// puts Open War in play: CrimethInc. unlocks her after.
#[test]
fn crimethinc_follows_an_action_requiring_a_baron() {
    let mut e = game(2);
    let elisha = e.add_minion(SeatId(0), find_card("Elisha Tucker"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    hand(&mut e, 0, &["Open War", "CrimethInc."]);
    pick(&mut e, play(0, 0, elisha));
    pick_when(&mut e, play(0, 0, elisha));
    to_minion_phase(&mut e, SeatId(0));
    assert!(!e.state.minions[elisha.index()].locked);
}

// CrimethInc.: a recruit, employ or equip of a card requiring an Anarch is
// an action requiring an Anarch (owner, rules-qa 62.21). Elisha employs
// Crypt's Sons, a retainer requiring an Anarch: CrimethInc. [pro] unlocks
// her after.
#[test]
fn crimethinc_follows_an_employ_of_a_card_requiring_an_anarch() {
    let mut e = game(2);
    let elisha = e.add_minion(SeatId(0), find_card("Elisha Tucker"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    hand(&mut e, 0, &["Crypt's Sons", "CrimethInc."]);
    pick(&mut e, play(0, 0, elisha));
    pick_when(&mut e, play(0, 0, elisha));
    to_minion_phase(&mut e, SeatId(0));
    assert!(!e.state.minions[elisha.index()].locked);
}

// CrimethInc. [tha]: "Put this card in play. During your minion phase, you
// can burn 1 pool to unlock a ready Anarch you control." Once in the
// phase, for a locked Anarch.
#[test]
fn crimethinc_in_play_unlocks_an_anarch_for_pool_once_a_phase() {
    let mut e = game(2);
    let elisha = e.add_minion(SeatId(0), find_card("Elisha Tucker"), 5, false);
    let other = e.add_minion(SeatId(0), find_card("Elisha Tucker"), 5, false);
    e.state.minions[other.index()].kind =
        zvtes::state::MinionKind::Vampire(find_card("Casey Snyder"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    let c = in_play_mode(&mut e, 0, "CrimethInc.", None, 2);
    e.state.minions[elisha.index()].locked = true;
    e.state.minions[other.index()].locked = true;
    let unlock = use_at(c, 0, None, Target::Minion(elisha));
    pick(&mut e, unlock);
    assert_eq!(e.state.seats[0].pool, 9);
    let _ = request(&mut e);
    assert!(!e.state.minions[elisha.index()].locked);
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        use_at(c, 0, None, Target::Minion(other)),
    );
}

// ---- Memory Rift --------------------------------------------------------------------------------

// [ANK 20190425] Memory Rift "Must be played (as well as other 'as the
// action is announced' action modifiers) before action modifiers that are
// not explicitly played 'as the action is announced'." [ANK 20181208]
// "Replace the action card (if any) before playing it." [LSJ 20011204-3]
// [PIB 20130704] Multi-Discipline: one play per set of Disciplines used.
// Elisha Tucker ([obf], [pre], no [dem]) bleeds: Memory Rift drawn as the
// replacement of her Line Brawl is offered as the action is announced, in
// its [obf], [pre] and [obf][pre] modes; once the block window opens, no
// longer. [obf]: +1 stealth before any stealth is needed, played.
#[test]
fn memory_rift_is_played_as_the_action_is_announced() {
    let mut e = game(2);
    let elisha = e.add_minion(SeatId(0), find_card("Elisha Tucker"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    e.state.seats[0].hand = vec![find_library("Line Brawl")].into();
    fill_hand(&mut e, 0);
    library(&mut e, 0, "Memory Rift", 1);
    let brawl = Choice::Play {
        card: 0,
        mode: 2,
        actor: Some(elisha),
        target: Target::Seat(SeatId(1)),
    };
    let req = request(&mut e);
    let brawl = if req.options.contains(&brawl) {
        brawl
    } else {
        *req.options
            .iter()
            .find(|o| matches!(o, Choice::Play { card: 0, actor: Some(a), .. } if *a == elisha))
            .expect("Line Brawl")
    };
    e.choose(index_of(&req, brawl));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionAsAnnounced);
    let rift = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Memory Rift"))
        .unwrap() as u8;
    let modes: Vec<u8> = req
        .options
        .iter()
        .filter_map(|o| match o {
            Choice::Play { card, mode, .. } if *card == rift => Some(*mode),
            _ => None,
        })
        .collect();
    assert_eq!(modes, vec![1, 2, 5]);
    e.choose(index_of(&req, play(rift, 1, elisha)));
    to_minion_phase(&mut e, SeatId(0));
    assert!(e.state.seats[0].ash.contains(&find_library("Memory Rift")));
}

// Memory Rift [dem]: "Choose a titled vampire or a younger vampire. The
// chosen vampire cannot block this action." [pre]: "If this action is
// blocked, this Anarch can strike: combat ends during the first round."
#[test]
fn memory_rift_bars_a_blocker_or_ends_the_combat() {
    // [dem]: the chosen titled vampire cannot block.
    let mut e = game(2);
    let medic = e.add_minion(SeatId(0), find_card("Medic, The"), 5, false);
    e.state.minions[medic.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let prince = e.add_minion(SeatId(1), find_card("Victoria Ash"), 5, false);
    hand(&mut e, 0, &["Memory Rift"]);
    pick(&mut e, bleed(medic));
    pick(&mut e, play_at(0, 0, medic, Target::Minion(prince)));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(prince));
    // [pre]: blocked, Elisha may strike: combat ends.
    let mut e = game(2);
    let elisha = e.add_minion(SeatId(0), find_card("Elisha Tucker"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    hand(&mut e, 0, &["Memory Rift"]);
    pick(&mut e, bleed(elisha));
    pick(&mut e, play(0, 2, elisha));
    pick(&mut e, block(z));
    pick_when(&mut e, Choice::StrikeCombatEnds);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!((blood(&e, elisha), blood(&e, z)), (5, 5));
}

// ---- Sense the Sin ---------------------------------------------------------------------------------

// [LSJ 20011204-3] [PIB 20130704] Sense the Sin: one Discipline per play,
// the card requiring the one used — The unnamed ([DAI], no [aus]) plays
// only its [DAI] mode: +2 bleed (limited), and minions with corruption
// counters get -1 intercept this action.
#[test]
fn sense_the_sin_superior_bleeds_and_lowers_corrupted_intercept() {
    // [DAI]: +2 bleed.
    let mut e = game(2);
    let unnamed = e.add_minion(SeatId(0), find_card("unnamed, The"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    e.state.minions[z.index()].add_corruption(SeatId(0), 1);
    hand(&mut e, 0, &["Sense the Sin"]);
    pick(&mut e, bleed(unnamed));
    let req = offering(&mut e, play(0, 2, unnamed));
    assert!(
        !req.options.contains(&play(0, 0, unnamed)) && !req.options.contains(&play(0, 1, unnamed))
    );
    e.choose(index_of(&req, play(0, 2, unnamed)));
    let _ = request(&mut e);
    let a = e.action().unwrap();
    assert_eq!(a.intercept_of(&e.state, z), -1, "corrupted: -1");
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(pool(&e)[1], 10 - 3);
}

// ---- Contagion ---------------------------------------------------------------------------------------

// Contagion [dai]: "+1 stealth action. Requires an infernal vampire. Put
// this card on this vampire. During each other Methuselah's unlock phase,
// they burn 1 pool if this vampire is ready." The card itself on The
// unnamed (rules-qa 47.1); seat 1 burns 1 in its unlock phase.
#[test]
fn contagion_drains_the_other_methuselahs_in_their_unlock_phase() {
    let mut e = game(2);
    let unnamed = e.add_minion(SeatId(0), find_card("unnamed, The"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    hand(&mut e, 0, &["Contagion"]);
    pick(&mut e, play(0, 0, unnamed));
    to_minion_phase(&mut e, SeatId(0));
    let c = card_in_play(&e, "Contagion");
    assert!(!e.state.cards[c.index()].effect_only);
    assert_eq!(e.state.cards[c.index()].host, Some(unnamed));
    assert!(!e.state.seats[0].ash.contains(&find_library("Contagion")));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(pool(&e), vec![10, 9]);
}

// [ANK 20200909] [LSJ 20040130] Contagion [DAI]: "Stealing the minion is
// part of the strike resolution, it does not happen if the strike is not
// selected and it can be dodged." The unnamed blocked by a vampire already
// holding capacity-1 of seat 0's counters: the strike adds one, the steal
// is offered and taken, the combat ends; dodged with Side Strike, no
// counter and no steal.
#[test]
fn contagion_superior_steals_a_corrupted_minion_unless_dodged() {
    for dodge in [false, true] {
        let mut e = game(2);
        let unnamed = e.add_minion(SeatId(0), find_card("unnamed, The"), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with("cel", 0), 5, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 5, false);
        let cap = e.state.capacity_of(z).unwrap();
        e.state.minions[z.index()].add_corruption(SeatId(0), cap - 1);
        let c = in_play_mode(&mut e, 0, "Contagion", Some(unnamed), 1);
        if dodge {
            hand(&mut e, 1, &["Side Strike"]);
        }
        pick(&mut e, bleed(unnamed));
        pick(&mut e, block(z));
        pick_when(&mut e, use_at(c, 0, None, Target::None));
        if dodge {
            pick_when(&mut e, play(0, 0, z));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.minions[z.index()].corruption_of(SeatId(0)), cap - 1);
            assert_eq!(e.state.minions[z.index()].owner, SeatId(1));
        } else {
            pick_when(&mut e, Choice::ChooseMinion(z));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.minions[z.index()].owner, SeatId(0));
            assert_eq!(e.state.minions[z.index()].corruption_of(SeatId(0)), 0);
        }
    }
}

// Anson: the second master phase action read live (rules-qa 62.5): ready,
// two masters; sent to torpor after the first, the second is lost.
#[test]
fn ansons_second_master_phase_action_is_read_live() {
    let mut e = game(2);
    let anson = e.add_minion(SeatId(0), find_card("Anson"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.at_master_phase();
    hand(&mut e, 0, &["Anarch Revolt", "Elder Library"]);
    pick(&mut e, master_play(0));
    let _ = request(&mut e);
    assert_eq!(e.state.mpa_left(SeatId(0)), 1);
    e.state.minions[anson.index()].in_torpor = true;
    assert_eq!(e.state.mpa_left(SeatId(0)), 0);
}

// Powerbase: Barranquilla's titled stealth, read live on the acting
// vampire: Victoria Ash's burn (+1 stealth) is out of seat 0's reach; an
// untitled vampire's is within it.
#[test]
fn powerbase_barranquilla_burn_has_stealth_for_a_titled_vampire() {
    for titled in [true, false] {
        let mut e = game(2);
        let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
        let v = if titled {
            e.add_minion(SeatId(1), find_card("Victoria Ash"), 4, false)
        } else {
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false)
        };
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let pb = in_play(&mut e, 0, "Powerbase: Barranquilla", None);
        e.state.cards[pb.index()].counters = 3;
        pick_when(&mut e, use_at(pb, 1, Some(v), Target::None));
        let req = offering(&mut e, block(b));
        assert_eq!(e.block_can_reach(req.seat, b), !titled, "titled {titled}");
    }
}

// ---- Herald of Topheth ---------------------------------------------------------------------

// Veneficti: "He can steal a demon ally or burn a demon retainer as a Ⓓ
// action." Herald of Topheth, a demon ally, stolen.
#[test]
fn veneficti_steals_a_demon_ally() {
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("unnamed, The"), 4, false);
    let ven = e.add_ally(SeatId(0), find_library("Veneficti"));
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let herald = e.add_ally(SeatId(1), find_library("Herald of Topheth"));
    let v = card_in_play(&e, "Veneficti");
    pick(&mut e, use_at(v, 0, Some(ven), Target::Minion(herald)));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.minions[herald.index()].owner, SeatId(0));
}

// Herald of Topheth: "Infernal. … If it has 4 or fewer life during your
// unlock phase, it gains 1 life." It does not unlock as normal — its
// controller may burn 1 pool to unlock it —, and it heals 1 life.
#[test]
fn herald_of_topheth_is_infernal_and_heals() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 2), 4, false);
    let herald = e.add_ally(SeatId(1), find_library("Herald of Topheth"));
    e.state.minions[herald.index()].locked = true;
    e.state.minions[herald.index()].blood = 3;
    pick(&mut e, Choice::Pass);
    let unlock = Choice::UnlockInfernal { minion: herald };
    let req = offering(&mut e, unlock);
    assert_eq!(req.seat, SeatId(1));
    assert!(e.state.minions[herald.index()].locked, "not as normal");
    e.choose(index_of(&req, unlock));
    to_minion_phase(&mut e, SeatId(1));
    assert!(!e.state.minions[herald.index()].locked);
    assert_eq!(e.state.seats[1].pool, 9);
    assert_eq!(blood(&e, herald), 4);
}

// [LSJ 20020503] [LSJ 20060126] Allies without a specified sect: "The ally
// counts as 'independent' when playing cards as a vampire." Herald of
// Topheth, meeting [pot], never plays Line Brawl ("Requires an Anarch").
#[test]
fn herald_of_topheth_plays_as_an_independent_vampire() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let herald = e.add_ally(SeatId(0), find_library("Herald of Topheth"));
    hand(&mut e, 0, &["Line Brawl"]);
    let req = request(&mut e);
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::Play { actor: Some(a), .. } if *a == herald)));
}
