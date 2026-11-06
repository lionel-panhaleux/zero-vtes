//! Round 25 (`ADDED_BATCHES`, slice 16ak): Ebony Kingdom and Blood Shadowed
//! Court completed (owner, 2026-11-03). Readings in rules-qa Batch 109.
//! Windows with a single option are elided, so the scripts only stop where a
//! real decision exists.

use zvtes::cards::Target;
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, CardId, CardInPlay};

mod common;
use common::*;

// ---- Helpers --------------------------------------------------------------

/// `n` seats at seat 0's minion phase, turn 4, 10 pool each.
fn game(n: usize) -> Engine {
    let mut e = Engine::bare(n);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.state.turn = 4;
    e
}

/// The `nth` untitled vampire of `sect` with no printed-text definition,
/// capacity 4 or more.
fn of_sect(sect: Sect, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.sect == Some(sect)
                && c.title == Title::None
                && c.capacity >= 4
                && !c.imbued
                && zvtes::cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no {sect:?} vampire"))
}

/// The first untitled vampire of `sect` with Discipline `disc` (as
/// printed) and no printed-text definition.
fn of_sect_with(sect: Sect, disc: &str) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .find(|(i, c)| {
            c.sect == Some(sect)
                && c.title == Title::None
                && !c.imbued
                && c.capacity >= 4
                && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case(disc))
                && zvtes::cards::vampire(*i as u16).is_none()
        })
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no {sect:?} with {disc}"))
}

/// A card already in play for `seat`, on `host` if given: its id.
fn in_play(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>) -> CardId {
    e.state
        .cards
        .push(CardInPlay::new(find_library(name), SeatId(seat), host, 0));
    CardId(e.state.cards.len() as u32 - 1)
}

/// As `in_play`, with the layers its first mode puts in play.
fn in_play_with_layers(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>) -> CardId {
    let id = in_play(e, seat, name, host);
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

/// The cards `name` in play on `host`.
fn on(e: &Engine, name: &str, host: MinionId) -> Vec<CardId> {
    let card = find_library(name);
    (0..e.state.cards.len())
        .map(|i| CardId(i as u32))
        .filter(|c| {
            let x = &e.state.cards[c.index()];
            x.in_play && x.card == card && x.host == Some(host)
        })
        .collect()
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

/// Seat 0 plays its hand's `name` on `m` in its master phase.
fn master_on(e: &mut Engine, name: &str, m: MinionId) {
    let card = find_library(name);
    let i = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == card)
        .expect("in hand") as u8;
    pick_when(e, play_at(i, 0, None, Target::Minion(m)));
    request(e);
}

// ---- Orun and Aye ----------------------------------------------------------

/// Orun: "For non-Orun cards played by Methuselahs other than this Laibon's
/// controller, this Laibon is considered to have 2 additional capacity. A
/// Laibon gets an additional vote for every three Orun he or she has." The
/// capacity stacks, 2 per Orun; the votes count every three altogether
/// (LSJ 20051204); its own controller's cards read the printed capacity.
// Rulings: [LSJ 20051204] [LSJ 20060407] [LSJ 20090623].
#[test]
fn orun_raises_capacity_for_others_and_votes_by_three() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cap = e.state.capacity_of(l).unwrap();
    hand(&mut e, 0, &["Orun", "Orun", "Orun"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    master_on(&mut e, "Orun", l);
    let first = on(&e, "Orun", l);
    assert_eq!(first.len(), 1);
    let c = &e.state.cards[first[0].index()];
    assert!(c.in_play && !c.effect_only, "the card itself");
    assert_eq!(e.state.capacity_by(l, SeatId(1)), Some(cap + 2));
    assert_eq!(e.state.capacity_by(l, SeatId(0)), Some(cap), "its own");
    assert_eq!(e.state.capacity_of(l), Some(cap));
    master_on(&mut e, "Orun", l);
    assert_eq!(e.state.capacity_by(l, SeatId(1)), Some(cap + 4));
    let votes = e.state.host_votes(l);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    master_on(&mut e, "Orun", l);
    assert_eq!(on(&e, "Orun", l).len(), 3);
    assert_eq!(e.state.host_votes(l), votes + 1, "one vote for three");
}

/// Orun, Aye: "Burn this card if this Laibon has more Orun and Aye than his
/// or her capacity": every one triggers, the Laibon's controller picks which
/// burns, and the rest stay as the condition is false again (red, rules-qa
/// 109.2). Not a Laibon, nothing applies (ANK 20221208).
// Rulings: [ANK 20221208].
#[test]
fn orun_and_aye_over_capacity_the_controller_picks_which_burns() {
    for keep_orun in [true, false] {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), find_card("Abu Nuwasi"), 3, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        assert_eq!(e.state.capacity_of(l), Some(3));
        hand(&mut e, 0, &["Orun", "Aye", "Orun", "Aye"]);
        until(&mut e, SeatId(0), WindowLabel::MasterPhase);
        master_on(&mut e, "Orun", l);
        master_on(&mut e, "Aye", l);
        until(&mut e, SeatId(0), WindowLabel::MasterPhase);
        master_on(&mut e, "Orun", l);
        assert_eq!(on(&e, "Orun", l).len() + on(&e, "Aye", l).len(), 3);
        master_on(&mut e, "Aye", l);
        let req = request(&mut e);
        assert_eq!(req.seat, SeatId(0), "the Laibon's controller");
        let burn: Vec<_> = req
            .options
            .iter()
            .filter_map(|o| match *o {
                Choice::BurnInPlay { card } => Some(e.state.cards[card as usize].card),
                _ => None,
            })
            .collect();
        assert_eq!(burn.len(), 2, "one option a kind: {:?}", req.options);
        let gone = if keep_orun { "Aye" } else { "Orun" };
        e.choose(
            req.options
                .iter()
                .position(|o| {
                    matches!(*o, Choice::BurnInPlay { card }
                        if e.state.cards[card as usize].card == find_library(gone))
                })
                .unwrap(),
        );
        request(&mut e);
        let (orun, aye) = (on(&e, "Orun", l).len(), on(&e, "Aye", l).len());
        assert_eq!((orun, aye), if keep_orun { (2, 1) } else { (1, 2) });
        assert!(e.state.seats[0].ash.contains(&find_library(gone)));
        // Not a Laibon: no burn, no capacity for others.
        e.state.minions[l.index()].sect_override = Some(Sect::Anarch);
        let cap = e.state.capacity_of(l);
        assert_eq!(e.state.capacity_by(l, SeatId(1)), cap);
    }
}

/// Pallid ("burn this card if this vampire has an Aye") burns as an Aye
/// takes its Laibon over capacity; the controller's choice of Orun or Aye
/// is asked once (rules-qa 109.2).
#[test]
fn pallid_burns_apart_from_the_orun_and_aye_choice() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Abu Nuwasi"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let pallid = in_play_with_layers(&mut e, 1, "Pallid", Some(l));
    for _ in 0..3 {
        in_play_with_layers(&mut e, 0, "Orun", Some(l));
    }
    hand(&mut e, 0, &["Aye"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    master_on(&mut e, "Aye", l);
    let req = request(&mut e);
    assert!(!e.state.cards[pallid.index()].in_play, "Pallid burned");
    let burns = req
        .options
        .iter()
        .filter(|o| matches!(o, Choice::BurnInPlay { .. }))
        .count();
    assert_eq!((req.seat, burns), (SeatId(0), 2), "{:?}", req.options);
}

/// Aye: a first-strike equipment burn counts too (rules-qa 109.6): the
/// Laibon's Canine Horde burns the blocker's Kevlar Vest, one Aye of two
/// burns.
#[test]
fn aye_burns_one_when_its_laibons_first_strike_burns_equipment() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect_with(Sect::Laibon, "ANI"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let vest = in_play(&mut e, 1, "Kevlar Vest", Some(z));
    in_play_with_layers(&mut e, 0, "Aye", Some(l));
    in_play_with_layers(&mut e, 0, "Aye", Some(l));
    hand(&mut e, 0, &["Canine Horde"]);
    pick_when(&mut e, bleed(l));
    pick_when(&mut e, block(z));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    let horde = plays(&req)
        .into_iter()
        .find(|o| matches!(o, Choice::Play { target: Target::Card(c), .. } if *c == vest))
        .expect("Canine Horde at the vest");
    pick(&mut e, horde);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[vest.index()].in_play, "burned");
    assert_eq!(on(&e, "Aye", l).len(), 1);
}

/// Aye counts the Laibon's own burns only (rules-qa 109.6): Waxen Poetica,
/// burned after its use against him, burns no Aye.
#[test]
fn aye_ignores_an_equipment_burned_by_its_own_effect() {
    let l_card = (0..CRYPT.len() as u16)
        .find(|&c| {
            let v = &CRYPT[c as usize];
            v.sect == Some(Sect::Laibon)
                && !v.disciplines.iter().any(|d| d.eq_ignore_ascii_case("cel"))
                && !v.adv
                && zvtes::cards::vampire(c).is_none()
        })
        .expect("a Laibon without Celerity");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let l = e.add_minion(SeatId(1), l_card, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play_with_layers(&mut e, 1, "Aye", Some(l));
    in_play_with_layers(&mut e, 1, "Aye", Some(l));
    let w = in_play(&mut e, 0, "Waxen Poetica", Some(a));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(l));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(w),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[w.index()].in_play, "burned after use");
    assert_eq!(on(&e, "Aye", l).len(), 2);
}

/// Orun: "If this Laibon successfully bleeds for more than 2 …, he or she
/// burns one Orun": one, however many (LSJ 20090603); a bleed for 1 none.
// Rulings: [LSJ 20090603].
#[test]
fn orun_burns_one_after_a_bleed_for_more_than_two() {
    for (more, left) in [(2i8, 1usize), (0, 2)] {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[v.index()].locked = true;
        hand(&mut e, 0, &["Orun", "Orun"]);
        until(&mut e, SeatId(0), WindowLabel::MasterPhase);
        master_on(&mut e, "Orun", l);
        master_on(&mut e, "Orun", l);
        e.state.minions[l.index()].timed.push((
            zvtes::cards::Lasting::Bleed(more),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(0),
        ));
        let pool = e.state.seats[1].pool;
        pick_when(&mut e, bleed(l));
        to_minion_phase(&mut e, SeatId(0));
        assert_eq!(e.state.seats[1].pool, pool - 1 - more as i32);
        assert_eq!(on(&e, "Orun", l).len(), left, "bleed +{more}");
    }
}

/// Aye: "This Laibon may lock this card to cancel a Frenzy card played on
/// him or her as it is played": Terror Frenzy, played on the opposing
/// minion (LSJ 20051113), canceled; the card still played (ANK 20190104).
// Rulings: [LSJ 20051113] [ANK 20190104].
#[test]
fn aye_cancels_a_frenzy_card_played_on_its_laibon() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ani = vampire_with("ANI", 0);
    let b = e.add_minion(SeatId(1), ani, 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let aye = in_play(&mut e, 0, "Aye", Some(l));
    hand(&mut e, 1, &["Terror Frenzy"]);
    pick_when(&mut e, bleed(l));
    pick_when(&mut e, block(b));
    let frenzy = play_at(0, 0, Some(b), Target::None);
    pick_when(&mut e, frenzy);
    let cancel = ability(aye.0, 0);
    pick_when(&mut e, cancel);
    request(&mut e);
    assert!(e.state.cards[aye.index()].locked);
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Terror Frenzy")));
}

/// Aye: "This Laibon may burn a blood and lock three of his or her Aye to
/// be able to play reaction cards and attempt to block as if unlocked for
/// the current action." Locked, three Aye: he blocks; two Aye, no.
#[test]
fn aye_lets_a_locked_laibon_block_for_three_locked_aye() {
    for n in [3, 2] {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let a = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let ayes: Vec<CardId> = (0..n).map(|_| in_play(&mut e, 0, "Aye", Some(l))).collect();
        to_minion_phase(&mut e, SeatId(1));
        e.state.minions[l.index()].locked = true;
        pick(&mut e, bleed(a));
        let grant = ability(ayes[0].0, 1);
        let req = request(&mut e);
        if n == 2 {
            assert!(!req.options.contains(&grant), "two Aye only");
            continue;
        }
        e.choose(index_of(&req, grant));
        assert!(ayes.iter().all(|c| e.state.cards[c.index()].locked));
        assert_eq!(blood(&e, l), 3);
        pick_when(&mut e, block(l));
    }
}

/// Aye: "If this Laibon burns a minion … in play, he or she burns one
/// Aye": a diablerie burns the victim, one Aye of two burns.
#[test]
fn aye_burns_one_when_its_laibon_burns_a_minion() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Aye", "Aye"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    master_on(&mut e, "Aye", l);
    master_on(&mut e, "Aye", l);
    assert_eq!(on(&e, "Aye", l).len(), 2);
    pick_when(
        &mut e,
        Choice::TakeAction {
            minion: l,
            action: ActionKind::Diablerie,
            target: Some(v),
        },
    );
    to_minion_phase(&mut e, SeatId(1));
    assert!(!e.state.minions[v.index()].in_play);
    assert_eq!(on(&e, "Aye", l).len(), 1);
}

/// Aye: any burn the Laibon's effects cause counts, a minion he burns while
/// blocking included (red, rules-qa 109.6): blocking an ally's bleed, his
/// strike burns it, one Aye of two burns, its equipment going with it no
/// second.
#[test]
fn aye_burns_one_when_its_blocking_laibon_burns_an_ally() {
    let mut e = game(2);
    let ally = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let l = e.add_minion(SeatId(1), of_sect(Sect::Laibon, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[l.index()].timed.push((
        zvtes::cards::Lasting::Strength(5),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    in_play_with_layers(&mut e, 1, "Aye", Some(l));
    in_play_with_layers(&mut e, 1, "Aye", Some(l));
    // Its equipment burns with it: no second Aye.
    in_play(&mut e, 0, "Kevlar Vest", Some(ally));
    pick_when(&mut e, bleed(ally));
    pick_when(&mut e, block(l));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play, "burned");
    assert_eq!(on(&e, "Aye", l).len(), 1);
}

/// Aye: equipment his strike destroys counts too (red, rules-qa 109.6):
/// Heroic Might's "strike: burn equipment" burns the blocker's Kevlar Vest,
/// one Aye of two burns.
#[test]
fn aye_burns_one_when_its_laibons_strike_burns_equipment() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect_with(Sect::Laibon, "POT"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let might = in_play(&mut e, 0, "Heroic Might", Some(l));
    let vest = in_play(&mut e, 1, "Kevlar Vest", Some(z));
    in_play_with_layers(&mut e, 0, "Aye", Some(l));
    in_play_with_layers(&mut e, 0, "Aye", Some(l));
    pick_when(&mut e, bleed(l));
    pick_when(&mut e, block(z));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(might),
            ability: 0,
            actor: None,
            target: Target::Card(vest),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[vest.index()].in_play, "burned");
    assert_eq!(on(&e, "Aye", l).len(), 1);
}

// ---- Bamba ------------------------------------------------------------------

/// Bamba: "it becomes a 1-capacity non-unique Laibon of the same clan and
/// cannot act this turn. If the acting Laibon is a magaji, you may search
/// your library, hand and ash heap for a master: Discipline card to play on
/// this vampire … and you may move 1 blood from that magaji to this
/// Laibon." A magaji: the search and the blood; not one, neither.
// Rulings: [LSJ 20090528-1].
#[test]
fn bamba_becomes_a_laibon_and_a_magaji_gives_it_more() {
    for magaji in [true, false] {
        let mut e = game(2);
        let sire = if magaji {
            find_card("Mamadou Keita")
        } else {
            of_sect(Sect::Laibon, 0)
        };
        let l = e.add_minion(SeatId(0), sire, 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[v.index()].locked = true;
        hand(&mut e, 0, &["Bamba", "Potence"]);
        pick_when(&mut e, play_at(0, 0, Some(l), Target::None));
        let req = request(&mut e);
        let bamba = e
            .state
            .minions
            .iter()
            .position(|m| m.in_play && m.name() == "Bamba")
            .map(|i| MinionId(i as u32))
            .expect("in play");
        assert_eq!(e.state.sect_of(bamba), Some(Sect::Laibon));
        assert_eq!(
            e.state.minions[bamba.index()].clan(),
            e.state.minions[l.index()].clan()
        );
        assert_eq!(e.state.capacity_of(bamba), Some(1));
        if !magaji {
            assert_ne!(req.window, WindowLabel::LibrarySearch);
            assert_eq!(blood(&e, bamba), 0, "it cannot act, so no hunt");
            continue;
        }
        assert_eq!(req.window, WindowLabel::LibrarySearch);
        let potence = find_library("Potence");
        let found = Choice::TakeFrom {
            zone: zvtes::decision::Zone::Hand,
            card: potence,
        };
        e.choose(index_of(&req, found));
        let req = request(&mut e);
        e.choose(index_of(&req, Choice::ResolvePrinted { minion: l }));
        request(&mut e);
        assert_eq!(blood(&e, bamba), 1);
        assert_eq!(blood(&e, l), 3 - 1, "the action's blood and the move");
        assert_eq!(e.state.capacity_of(bamba), Some(2), "the master's +1");
    }
}

// ---- Bestial Vengeance ----------------------------------------------------

/// Bestial Vengeance [ANI]: "Choose a retainer on this Laibon. This combat,
/// … the opposing minion takes 1 damage during strike resolution if the
/// retainer is ready … [ANI] As above, but for 2 damage." Environmental
/// (RTR 19970630): 2 more damage while the retainer stays, none once it is
/// gone; one each combat.
// Rulings: [RTR 19970630].
#[test]
fn bestial_vengeance_hurts_while_the_retainer_stays() {
    let mut lost = Vec::new();
    for stays in [true, false] {
        let mut e = game(2);
        let fish = e.add_minion(SeatId(0), find_card("Fish"), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let b = e.add_minion(SeatId(1), vampire_with("pot", 0), 8, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let raven = in_play(&mut e, 0, "Raven Spy", Some(fish));
        hand(&mut e, 0, &["Bestial Vengeance", "Bestial Vengeance"]);
        hand(&mut e, 1, &["Immortal Grapple"]);
        pick_when(&mut e, bleed(fish));
        pick_when(&mut e, block(b));
        let before = blood(&e, b);
        let superior = play_at(0, 1, Some(fish), Target::Card(raven));
        pick_when(&mut e, superior);
        let req = request(&mut e);
        assert_eq!(
            (req.seat, req.window),
            (SeatId(1), WindowLabel::CombatBeforeStrikes)
        );
        if !stays {
            e.state.cards[raven.index()].in_play = false;
        }
        e.choose(index_of(&req, Choice::Pass));
        let req = request(&mut e);
        assert!(
            !plays(&req)
                .iter()
                .any(|p| matches!(p, Choice::Play { actor: Some(a), .. } if *a == fish)),
            "one each combat"
        );
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        lost.push(before - blood(&e, b));
    }
    assert_eq!(lost[0], lost[1] + 2, "{lost:?}");
}

// ---- Blood Shield ---------------------------------------------------------

/// A plain vampire of `clan` with capacity in `caps`.
fn plain_cap(clan: &str, caps: std::ops::RangeInclusive<u8>) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .find(|(i, c)| {
            (clan.is_empty() || c.clan == clan)
                && !c.imbued
                && caps.contains(&c.capacity)
                && c.title == Title::None
                && !c.adv
                && zvtes::cards::vampire(*i as u16).is_none()
        })
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no {clan} of {caps:?}"))
}

/// Blood Shield: "may prevent 1 damage from an opposing minion's strike
/// each round. If used to prevent damage from a hand strike made by a
/// vampire with capacity 6 or less, that vampire burns 2 blood": at once
/// (LSJ 20090528-1); a capacity 7 striker burns none.
// Rulings: [LSJ 20090528-1].
#[test]
fn blood_shield_prevents_and_burns_a_small_hand_striker() {
    for (caps, burned) in [(4..=6, 2u8), (7..=11, 0)] {
        // Any clan: the cap is what counts.
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let b = e.add_minion(SeatId(1), plain_cap("", caps), 6, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let shield = in_play(&mut e, 0, "Blood Shield", Some(a));
        pick_when(&mut e, bleed(a));
        pick_when(&mut e, block(b));
        pick_when(&mut e, ability(shield.0, 0));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, a), 4, "its 1 damage prevented");
        assert_eq!(blood(&e, b), 6 - 1 - burned);
    }
}

// ---- Brutal Influence -----------------------------------------------------

/// Brutal Influence [pot]: "Bleed at +1 bleed. When the action is
/// announced, this vampire may lock X Orun on him or her to get an
/// additional +X bleed." Two Orun locked: a bleed for 4.
#[test]
fn brutal_influence_locks_orun_for_more_bleed() {
    let mut e = game(2);
    let fish = e.add_minion(SeatId(0), find_card("Fish"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[v.index()].locked = true;
    let oruns = [
        in_play(&mut e, 0, "Orun", Some(fish)),
        in_play(&mut e, 0, "Orun", Some(fish)),
    ];
    hand(&mut e, 0, &["Brutal Influence"]);
    let pool = e.state.seats[1].pool;
    pick_when(&mut e, play_at(0, 0, Some(fish), Target::None));
    let req = request(&mut e);
    e.choose(index_of(&req, set_x(2)));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.seats[1].pool, pool - 4);
    assert!(oruns.iter().all(|c| e.state.cards[c.index()].locked));
}

/// Brutal Influence [POT]: "Lock X Orun on this vampire to move X blood
/// from the bank to a younger Laibon in your uncontrolled region." One
/// Orun locked, one blood on Abu Nuwasi (capacity 3, younger than Fish).
#[test]
fn brutal_influence_locks_orun_for_a_younger_laibons_blood() {
    let mut e = game(2);
    let fish = e.add_minion(SeatId(0), find_card("Fish"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[v.index()].locked = true;
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: find_card("Abu Nuwasi"),
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    let orun = in_play(&mut e, 0, "Orun", Some(fish));
    hand(&mut e, 0, &["Brutal Influence"]);
    pick_when(&mut e, play_at(0, 1, Some(fish), Target::Uncontrolled(0)));
    let req = request(&mut e);
    e.choose(index_of(&req, set_x(1)));
    to_minion_phase(&mut e, SeatId(0));
    assert!(e.state.cards[orun.index()].locked);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 1);
}

// ---- Despiral ---------------------------------------------------------------

/// Despiral: "Put this card on this Ishtarri and put 1 counter on this
/// card. This Ishtarri gets +X bleed, where X is the number of counters on
/// this card. This Ishtarri can add a counter to this card as a +1 stealth
/// action that costs X blood." One counter: +1 bleed; the action next turn
/// for 1 blood: two, +2 bleed.
// Rulings: [LSJ 20080604].
#[test]
fn despiral_grows_its_bleed_by_its_counters() {
    let mut e = game(2);
    let ish = e.add_minion(SeatId(0), plain("Ishtarri", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[v.index()].locked = true;
    let base = e.state.current_bleed(ish);
    hand(&mut e, 0, &["Despiral"]);
    pick_when(&mut e, play_at(0, 0, Some(ish), Target::None));
    request(&mut e);
    let d = on(&e, "Despiral", ish);
    assert_eq!(d.len(), 1, "on him");
    assert_eq!(e.state.cards[d[0].index()].counters, 1);
    assert_eq!(e.state.current_bleed(ish), base + 1);
    assert_eq!(blood(&e, ish), 2, "the card's blood");
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(0));
    let grow = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(d[0]),
        ability: 0,
        actor: Some(ish),
        target: Target::None,
    };
    pick_when(&mut e, grow);
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.cards[d[0].index()].counters, 2);
    assert_eq!(blood(&e, ish), 1, "X = 1 blood");
    assert_eq!(e.state.current_bleed(ish), base + 2);
}

// ---- Devil-Channel: Feet ---------------------------------------------------

/// Devil-Channel: Feet [ABO]: "Frenzy (this vampire) … Strike: dodge with
/// an optional press. [ABO] As above, and this vampire locks an Aye on him
/// or her to unlock." Blocked as he bleeds, he dodges, and an Aye locked
/// unlocks him.
#[test]
fn devil_channel_feet_locks_an_aye_to_unlock() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Socrate Cidibe"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let aye = in_play(&mut e, 0, "Aye", Some(s));
    hand(&mut e, 0, &["Devil-Channel: Feet"]);
    pick_when(&mut e, bleed(s));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play_at(0, 2, Some(s), Target::None));
    request(&mut e);
    assert!(e.state.cards[aye.index()].locked);
    assert!(!e.state.minions[s.index()].locked, "unlocked");
}

// ---- Dusk Work --------------------------------------------------------------

/// Dusk Work: "Requires a Laibon with three or more Aye. Play after
/// resolving an action (successfully or not). Lock three Aye on this Laibon
/// to unlock him or her." Three Aye: all locked, he unlocks; two, not
/// offered.
// Rulings: [PIB 20150915].
#[test]
fn dusk_work_locks_three_aye_to_unlock() {
    for n in [3, 2] {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[v.index()].locked = true;
        let ayes: Vec<CardId> = (0..n).map(|_| in_play(&mut e, 0, "Aye", Some(l))).collect();
        hand(&mut e, 0, &["Dusk Work"]);
        let dusk = play_at(0, 0, Some(l), Target::None);
        pick_when(&mut e, bleed(l));
        if n == 2 {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, dusk);
            continue;
        }
        pick_when(&mut e, dusk);
        request(&mut e);
        assert!(ayes.iter().all(|c| e.state.cards[c.index()].locked));
        assert!(!e.state.minions[l.index()].locked);
    }
}

// ---- Edge of the World ------------------------------------------------------

/// Edge of the World: "Only usable if you have gained a victory point
/// during this action. You gain 4 pool. If this Guruhi is acting, he or she
/// unlocks and gains enough blood from the bank to reach full capacity."
/// The prey ousted by the bleed: the card after resolution.
#[test]
fn edge_of_the_world_rewards_a_victory_point() {
    let mut e = game(3);
    let g = e.add_minion(SeatId(0), plain("Guruhi", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[v.index()].locked = true;
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    e.state.seats[1].pool = 1;
    hand(&mut e, 0, &["Edge of the World"]);
    let edge = play_at(0, 0, Some(g), Target::None);
    pick_when(&mut e, bleed(g));
    let pool = e.state.seats[0].pool;
    pick_when(&mut e, edge);
    request(&mut e);
    assert!(e.state.seats[1].ousted);
    assert_eq!(e.state.seats[0].pool, pool + 6 + 4, "the oust's 6, then 4");
    assert!(!e.state.minions[g.index()].locked);
    assert_eq!(Some(blood(&e, g)), e.state.capacity_of(g));
}

// ---- Exile ------------------------------------------------------------------

/// Exile [pre]: "Lock an ally or younger vampire and unlock this acting
/// vampire." [PRE]: "Lock X Orun on this vampire to send a vampire or
/// imbued with capacity or cost 2X or less to his or her owner's
/// uncontrolled region": two Orun, a capacity 4 vampire sent.
// Rulings: [ANK 20221208].
#[test]
fn exile_locks_or_sends_away_by_its_orun() {
    // [pre]
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), vampire_with("PRE", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain_cap("Ventrue", 1..=3), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Exile"]);
    pick_when(&mut e, play_at(0, 0, Some(p), Target::Minion(y)));
    to_minion_phase(&mut e, SeatId(0));
    assert!(e.state.minions[y.index()].locked);
    assert!(!e.state.minions[p.index()].locked, "unlocked");
    // [PRE]
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), vampire_with("PRE", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain_cap("Ventrue", 4..=4), 2, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[v.index()].locked = true;
    let oruns = [
        in_play(&mut e, 0, "Orun", Some(p)),
        in_play(&mut e, 0, "Orun", Some(p)),
    ];
    hand(&mut e, 0, &["Exile"]);
    pick_when(&mut e, play_at(0, 1, Some(p), Target::Minion(y)));
    // X: 2 the only choice (half of 4 up to two Orun), elided.
    to_minion_phase(&mut e, SeatId(0));
    assert!(!e.state.minions[y.index()].in_play, "sent away");
    assert_eq!(e.state.seats[1].uncontrolled.len(), 1);
    assert!(oruns.iter().all(|c| e.state.cards[c.index()].locked));
}

// ---- Familial Bond ----------------------------------------------------------

/// Familial Bond: "+1 intercept. If this block fails, the acting minion may
/// choose to make the action fail. If the action succeeds, this Laibon may
/// lock (after resolution) to enter combat with the acting minion." The
/// block short of a +2 stealth bleed: made to fail, no pool lost; let
/// through, the Laibon locks to fight.
// Rulings: [ANK 20200329] [LSJ 20070219].
#[test]
fn familial_bond_fails_the_action_or_brings_combat() {
    for fail in [true, false] {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let a = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Familial Bond"]);
        to_minion_phase(&mut e, SeatId(1));
        e.state.minions[a.index()].timed.push((
            zvtes::cards::Lasting::Stealth(2),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(1),
        ));
        let pool = e.state.seats[0].pool;
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(l));
        pick_when(&mut e, play_at(0, 0, Some(l), Target::None));
        let req = offering(&mut e, Choice::ResolvePrinted { minion: a });
        assert_eq!(req.seat, SeatId(1), "the acting Methuselah chooses");
        if fail {
            e.choose(index_of(&req, Choice::ResolvePrinted { minion: a }));
            to_minion_phase(&mut e, SeatId(0));
            assert_eq!(e.state.seats[0].pool, pool, "failed: no bleed");
            continue;
        }
        e.choose(index_of(&req, Choice::Pass));
        let req = offering(&mut e, Choice::ResolvePrinted { minion: l });
        assert_eq!(req.seat, SeatId(0));
        e.choose(index_of(&req, Choice::ResolvePrinted { minion: l }));
        request(&mut e);
        assert!(blood(&e, l) < 4 && blood(&e, a) < 4, "they fought");
        assert!(e.state.minions[l.index()].locked);
        assert!(e.state.seats[0].pool < pool, "bled");
    }
}

// ---- Guinea-Bissau Carnival -------------------------------------------------

/// Guinea-Bissau Carnival: "Each Laibon gets +X stealth when hunting and
/// gains X+1 additional blood from the bank if successful, where X is the
/// number of Aye he or she has. Burn this card during your next unlock
/// phase." Two Aye: a hunt for 1 + 3; the card gone at the next unlock.
#[test]
fn guinea_bissau_carnival_feeds_laibon_by_their_aye() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[v.index()].locked = true;
    in_play(&mut e, 0, "Aye", Some(l));
    in_play(&mut e, 0, "Aye", Some(l));
    hand(&mut e, 0, &["Guinea-Bissau Carnival"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick_when(&mut e, play_at(0, 0, None, Target::None));
    pick_when(&mut e, action(l, ActionKind::Hunt));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(blood(&e, l), 1 + 1 + 3);
    let carnival = find_library("Guinea-Bissau Carnival");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == carnival));
    to_minion_phase(&mut e, SeatId(0));
    assert!(!e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == carnival));
    assert!(e.state.seats[0].ash.contains(&carnival));
}

// ---- Hiding in the Open -----------------------------------------------------

/// A prince with no printed-text definition.
fn prince() -> u16 {
    (0..CRYPT.len() as u16)
        .find(|&c| {
            matches!(CRYPT[c as usize].title, Title::Prince(_))
                && zvtes::cards::vampire(c).is_none()
                && CRYPT[c as usize].capacity >= 4
        })
        .expect("a prince")
}

/// Hiding in the Open: "If this referendum succeeds, put this card on the
/// acting Laibon. This Laibon gets +1 bleed. Cards which require a
/// non-Laibon title cannot be played on this Laibon nor while this Laibon
/// is acting." On him: +1 bleed, and a prince's Second Tradition: Domain
/// is not offered against his bleed; without it, it is.
#[test]
fn hiding_in_the_open_bars_other_titles_cards() {
    for hiding in [true, false] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card("Mamadou Keita"), 7, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let p = e.add_minion(SeatId(1), prince(), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let base = e.state.current_bleed(m);
        if hiding {
            hand(&mut e, 0, &["Hiding in the Open"]);
            pick(&mut e, play(0, 0, m));
            pick_when(&mut e, votes(m, true));
            close_polling(&mut e);
            to_minion_phase(&mut e, SeatId(1));
            assert_eq!(on(&e, "Hiding in the Open", m).len(), 1);
            assert_eq!(e.state.current_bleed(m), base + 1);
            to_minion_phase(&mut e, SeatId(0));
        }
        e.state.minions[m.index()].timed.push((
            zvtes::cards::Lasting::Stealth(1),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(0),
        ));
        hand(&mut e, 1, &["Second Tradition: Domain"]);
        pick_when(&mut e, bleed(m));
        pick_when(&mut e, block(p));
        let domain = play_at(0, 0, Some(p), Target::None);
        if hiding {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, domain);
        } else {
            pick_when(&mut e, domain);
        }
    }
}

// ---- Ilomba -----------------------------------------------------------------

/// Ilomba: "If Ilomba would burn a life (or would otherwise be burned),
/// this minion burns a blood or life instead. If he or she cannot, he or
/// she is burned." Put on seat 1's vampire, smashed for 2 life: its host
/// burns 2 blood, Ilomba stays (ANK 20180525); a host with 1 blood burns.
// Rulings: [ANK 20180525] [LSJ 20090824].
#[test]
fn ilomba_takes_its_life_from_its_host() {
    for (host_blood, burned) in [(4u8, false), (1, true)] {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
        let an = e.add_minion(SeatId(0), of_sect_with(Sect::Anarch, "ani"), 4, false);
        let v = e.add_minion(SeatId(1), plain("Ventrue", 0), host_blood, false);
        let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        e.state.minions[w.index()].locked = true;
        hand(&mut e, 0, &["Ilomba", "Smash and Grab"]);
        pick_when(&mut e, play_at(0, 0, Some(l), Target::Minion(v)));
        let req = request(&mut e);
        if req.window == WindowLabel::ActionBlock {
            e.choose(index_of(&req, Choice::Pass));
            request(&mut e);
        }
        let ilomba = on(&e, "Ilomba", v);
        assert_eq!(ilomba.len(), 1, "on seat 1's vampire");
        e.state.minions[v.index()].locked = true;
        let hand_i = e.state.seats[0]
            .hand
            .iter()
            .position(|&c| c == find_library("Smash and Grab"))
            .unwrap() as u8;
        pick_when(
            &mut e,
            play_at(hand_i, 1, Some(an), Target::Card(ilomba[0])),
        );
        to_minion_phase(&mut e, SeatId(1));
        if burned {
            assert!(!e.state.minions[v.index()].in_play, "no blood: burned");
        } else {
            assert_eq!(blood(&e, v), host_blood - 2);
            assert!(e.state.cards[ilomba[0].index()].in_play, "Ilomba stays");
        }
    }
}

// ---- Impundulu --------------------------------------------------------------

/// The card in play of ally `m`.
fn ally_card(e: &Engine, m: MinionId) -> CardId {
    CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.body == Some(m))
            .expect("its card") as u32,
    )
}

/// Impundulu: "He may steal 1 blood or life from a minion as a +1 stealth
/// Ⓓ action." From seat 1's vampire: 1 blood moved to his life.
// Rulings: [RTR 20010711].
#[test]
fn impundulu_steals_blood_for_life() {
    let mut e = game(2);
    let imp = e.add_ally(SeatId(0), find_library("Impundulu"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[w.index()].locked = true;
    let life = blood(&e, imp);
    let steal = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(ally_card(&e, imp)),
        ability: 2,
        actor: Some(imp),
        target: Target::Minion(v),
    };
    pick_when(&mut e, steal);
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(blood(&e, v), 2);
    assert_eq!(blood(&e, imp), life + 1);
}

// ---- Invoke Poison Glands ---------------------------------------------------

/// Invoke Poison Glands [ABO]: "strike: hand strike at +X damage, where X
/// is the number of Orun on this vampire." Two Orun: 1 + 2 damage.
// Rulings: [TOM 19960225].
#[test]
fn invoke_poison_glands_strikes_harder_by_orun() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Socrate Cidibe"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 0, "Orun", Some(s));
    in_play(&mut e, 0, "Orun", Some(s));
    hand(&mut e, 0, &["Invoke Poison Glands"]);
    pick_when(&mut e, bleed(s));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play_at(0, 3, Some(s), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 6 - 3);
}

// ---- Ishtarri Warlord -------------------------------------------------------

/// Ishtarri Warlord: "Put this card on an Ishtarri you control. This
/// Ishtarri gets an optional maneuver or press each combat." On him; its
/// maneuver offered before range.
#[test]
fn ishtarri_warlord_goes_on_an_ishtarri() {
    let mut e = game(2);
    let ish = e.add_minion(SeatId(0), plain("Ishtarri", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Ishtarri Warlord"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    master_on(&mut e, "Ishtarri Warlord", ish);
    let w = on(&e, "Ishtarri Warlord", ish);
    assert_eq!(w.len(), 1);
    let c = &e.state.cards[w[0].index()];
    assert!(c.in_play && !c.effect_only);
    pick_when(&mut e, bleed(ish));
    pick_when(&mut e, block(b));
    pick_when(&mut e, ability(w[0].0, 0));
}

// ---- Jua Vema ---------------------------------------------------------------

/// Jua Vema [FOR]: "This vampire treats aggravated damage as normal damage
/// … for the remainder of the action if you lock an Aye on him or her."
/// An aggravated hand strike: 1 blood lost, ready, the Aye locked; without
/// the card, torpor.
#[test]
fn jua_vema_takes_aggravated_damage_as_normal() {
    for played in [true, false] {
        let mut e = game(2);
        let s = e.add_minion(SeatId(0), find_card("Socrate Cidibe"), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        e.state.minions[b.index()].timed.push((
            zvtes::cards::Lasting::AggravatedHands,
            zvtes::state::Expiry::EndOfTurn,
            SeatId(1),
        ));
        let aye = in_play(&mut e, 0, "Aye", Some(s));
        hand(&mut e, 0, &["Jua Vema"]);
        pick_when(&mut e, bleed(s));
        pick_when(&mut e, block(b));
        if played {
            pick_when(&mut e, play_at(0, 1, Some(s), Target::None));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let v = &e.state.minions[s.index()];
        assert_eq!(v.in_torpor, !played);
        if played {
            assert_eq!(v.blood, 4);
            assert!(e.state.cards[aye.index()].locked);
        }
    }
}

// ---- Kuta -------------------------------------------------------------------

/// Kuta: "Choose a younger vampire with capacity 6 or less. If this
/// referendum succeeds, put this card on that vampire. Directed actions
/// cost this vampire an additional blood. This vampire cannot block older
/// vampires." On it: its bleed costs 1 blood; it cannot block the older
/// caller.
#[test]
fn kuta_taxes_and_bars_a_younger_vampire() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mamadou Keita"), 7, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain_cap("Ventrue", 4..=6), 4, false);
    let other = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Kuta"]);
    pick(&mut e, play(0, 0, m));
    pick_when(&mut e, Choice::ChooseMinion(y));
    pick_when(&mut e, votes(m, true));
    close_polling(&mut e);
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(on(&e, "Kuta", y).len(), 1);
    let before = blood(&e, y);
    pick(&mut e, bleed(y));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, y), before - 1, "the directed action's toll");
    e.state.minions[y.index()].locked = false;
    e.state.minions[other.index()].locked = false;
    pick(&mut e, bleed(m));
    let req = request(&mut e);
    assert!(req.options.contains(&block(other)));
    assert!(!req.options.contains(&block(y)), "older: no block");
}

// ---- Make an Example --------------------------------------------------------

/// Make an Example: "Inflict 2 unpreventable damage on both the acting
/// Laibon and the blocking minion and end the action (without combat)."
/// Both take 2; no combat, no bleed.
// Rulings: [RTR 20180719] [ANK 20200207].
#[test]
fn make_an_example_hurts_both_and_ends_the_action() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
    let g = e.add_minion(SeatId(0), plain("Guruhi", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Make an Example"]);
    let pool = e.state.seats[1].pool;
    pick_when(&mut e, bleed(l));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play_at(0, 0, Some(g), Target::None));
    request(&mut e);
    assert_eq!(blood(&e, l), 2);
    assert_eq!(blood(&e, b), 2);
    assert_eq!(e.state.seats[1].pool, pool, "no bleed");
    assert!(e.action().is_none() || e.combat().is_none());
}

// ---- Mapatano Utando --------------------------------------------------------

/// Mapatano Utando: "You may lock this card to reduce a bleed against you
/// by 1. During your influence phase, you may lock this card and burn the
/// Edge to gain 4 transfers." A bleed for 1 against seat 0: none lost; its
/// next influence phase, the Edge for 4 transfers.
#[test]
fn mapatano_utando_reduces_a_bleed_and_trades_the_edge() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let a = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let card = in_play(&mut e, 0, "Mapatano Utando", None);
    to_minion_phase(&mut e, SeatId(1));
    let pool = e.state.seats[0].pool;
    pick(&mut e, bleed(a));
    pick_when(&mut e, ability(card.0, 0));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.seats[0].pool, pool, "reduced to 0");
    e.state.seats[0].has_edge = true;
    e.state.cards[card.index()].locked = false;
    to_minion_phase(&mut e, SeatId(1));
    let req = offering(&mut e, ability(card.0, 1));
    let left = e.transfers_left().expect("the influence phase");
    e.choose(index_of(&req, ability(card.0, 1)));
    assert_eq!(e.transfers_left(), Some(left + 4), "4 transfers more");
    request(&mut e);
    assert!(!e.state.seats[0].has_edge, "the Edge burned");
}

// ---- Mundane ----------------------------------------------------------------

/// Mundane: "Put this card on a Laibon with no Orun and no Mundane. … Burn
/// this card if this vampire gets an Orun. Alternatively, when a minion
/// with no Orun plays an action card requiring a Discipline or Virtue,
/// play this card to your ash heap as an out-of-turn master to cancel that
/// card as it is played." Out of turn: Entrancement canceled. On a Laibon:
/// burned as an Orun arrives.
// Rulings: [ANK 20200813-2].
#[test]
fn mundane_cancels_a_discipline_action_or_taxes_a_laibon() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
    let p = e.add_minion(SeatId(1), vampire_with("pre", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Mundane", "Mundane", "Orun"]);
    hand(&mut e, 1, &["Entrancement"]);
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, play(0, 0, p));
    let mundane = play_at(0, 1, None, Target::None);
    pick_when(&mut e, mundane);
    request(&mut e);
    assert!(e.state.seats[1].ash.contains(&find_library("Entrancement")));
    assert!(e.state.seats[0].ash.contains(&find_library("Mundane")));
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    master_on(&mut e, "Mundane", l);
    assert_eq!(on(&e, "Mundane", l).len(), 1);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    master_on(&mut e, "Orun", l);
    assert!(on(&e, "Mundane", l).is_empty(), "burned for the Orun");
}

// ---- Neebi ------------------------------------------------------------------

/// Neebi: "Requires a Laibon with three or more Aye. … Set the range for
/// this round. Skip the determine range step this round." Long range set.
// Rulings: [RTR 19970630].
#[test]
fn neebi_sets_the_range() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    for _ in 0..3 {
        in_play(&mut e, 0, "Aye", Some(l));
    }
    hand(&mut e, 0, &["Neebi"]);
    e.notes = Some(Vec::new());
    pick_when(&mut e, bleed(l));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play_at(0, 1, Some(l), Target::None));
    request(&mut e);
    let ranges: Vec<_> = noted_combats(&mut e)
        .into_iter()
        .filter_map(|n| match n {
            zvtes::combat::CombatNote::Strikes { range, .. } => Some(range),
            _ => None,
        })
        .collect();
    assert_eq!(ranges, vec![zvtes::combat::Range::Long]);
}

// ---- Nkishi -----------------------------------------------------------------

/// Nkishi: "You may use a master phase action to search your library or
/// ash heap for an Aye or an Orun and put that card on this Laibon." An
/// Orun from the ash heap onto him, its capacity bonus with it.
#[test]
fn nkishi_fetches_an_orun_onto_its_laibon() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let nkishi = in_play(&mut e, 0, "Nkishi", Some(l));
    let orun = find_library("Orun");
    e.state.seats[0].ash.put(orun, Default::default());
    let cap = e.state.capacity_of(l).unwrap();
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick_when(&mut e, ability(nkishi.0, 0));
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::AshHeap,
            card: orun,
        },
    );
    request(&mut e);
    assert_eq!(on(&e, "Orun", l).len(), 1);
    assert_eq!(
        e.state.capacity_by(l, SeatId(1)),
        Some(cap + 2),
        "its layers"
    );
}

// ---- Pallid -----------------------------------------------------------------

/// Pallid: "when a vampire with no Aye or a zombie is acting, play this
/// card to your ash heap as an out-of-turn master to give a vampire you
/// control +1 intercept." A +1 stealth bleed against seat 0: Pallid, then
/// a block that reaches it.
#[test]
fn pallid_gives_intercept_out_of_turn() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let a = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Pallid"]);
    to_minion_phase(&mut e, SeatId(1));
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Stealth(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    let pool = e.state.seats[0].pool;
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(d));
    pick_when(&mut e, play_at(0, 1, None, Target::Minion(d)));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.seats[0].pool, pool, "blocked");
    assert!(e.state.seats[0].ash.contains(&find_library("Pallid")));
}

// ---- Reliquary: Trinket -----------------------------------------------------

/// Reliquary: Trinket: "If the bearer is a ready Laibon, you get +1 hand
/// size. This Laibon may discard up to 3 cards from your hand as a +1
/// stealth action." Equipped: +1 hand size; its action next turn, 2 cards
/// discarded (an empty library replaces none).
// Rulings: [LSJ 20080604].
#[test]
fn reliquary_trinket_grows_the_hand_and_discards() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[v.index()].locked = true;
    let size = e.state.hand_size(SeatId(0));
    hand(
        &mut e,
        0,
        &["Reliquary: Trinket", "Bonding", "Bonding", "Bonding"],
    );
    pick_when(&mut e, play_at(0, 0, Some(l), Target::None));
    request(&mut e);
    let t = on(&e, "Reliquary: Trinket", l);
    assert_eq!(t.len(), 1);
    assert_eq!(e.state.hand_size(SeatId(0)), size + 1);
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(0));
    let held = e.state.seats[0].hand.len();
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(t[0]),
            ability: 0,
            actor: Some(l),
            target: Target::None,
        },
    );
    pick_when(&mut e, Choice::Discard(0));
    pick_when(&mut e, Choice::Discard(0));
    let req = request(&mut e);
    e.choose(index_of(&req, Choice::Pass));
    request(&mut e);
    assert_eq!(e.state.seats[0].hand.len(), held - 2);
}

// ---- Remnant of the Endless Storm -------------------------------------------

/// Remnant of the Endless Storm: "+1 strength for each life it has. …
/// enter combat with any minion as a Ⓓ action. If any effect does more than
/// 2 damage to the Remnant, ignore the excess. If the Remnant has 4 or
/// fewer life during your unlock phase, it gains 1 life." Recruited: 6
/// strength; a 3-damage strike does 2; 1 life back at the unlock phase.
#[test]
fn remnant_of_the_endless_storm_caps_damage_and_heals() {
    let mut e = game(2);
    e.state.seats[0].pool = 20;
    let m = e.add_minion(SeatId(0), find_card("Mamadou Keita"), 7, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[v.index()].locked = true;
    e.state.minions[w.index()].locked = true;
    hand(&mut e, 0, &["Remnant of the Endless Storm"]);
    pick_when(&mut e, play_at(0, 0, Some(m), Target::None));
    to_minion_phase(&mut e, SeatId(1));
    let r = e
        .state
        .minions
        .iter()
        .position(|x| x.in_play && x.name() == "Remnant of the Endless Storm")
        .map(|i| MinionId(i as u32))
        .expect("recruited");
    assert_eq!(e.state.base_strength(r), 1 + 5);
    to_minion_phase(&mut e, SeatId(0));
    e.state.minions[v.index()].locked = false;
    e.state.minions[v.index()].timed.push((
        zvtes::cards::Lasting::Strength(2),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    let fight = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(ally_card(&e, r)),
        ability: 0,
        actor: Some(r),
        target: Target::Minion(v),
    };
    pick_when(&mut e, fight);
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(blood(&e, r), 5 - 2, "3 damage capped at 2");
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, r), 4, "1 life at the unlock phase");
}

// ---- Savannah Runner --------------------------------------------------------

/// Savannah Runner [CEL]: "Lock … an Aye on him or her to unlock another
/// ready Laibon. Not usable by a blocking Laibon." During seat 1's bleed,
/// a locked Laibon unlocked by another's Aye.
#[test]
fn savannah_runner_unlocks_another_laibon() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), of_sect_with(Sect::Laibon, "CEL"), 4, false);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
    let a = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let aye = in_play(&mut e, 0, "Aye", Some(r));
    hand(&mut e, 0, &["Savannah Runner"]);
    to_minion_phase(&mut e, SeatId(1));
    e.state.minions[l.index()].locked = true;
    pick(&mut e, bleed(a));
    pick_when(&mut e, play_at(0, 2, Some(r), Target::Minion(l)));
    request(&mut e);
    assert!(!e.state.minions[l.index()].locked);
    assert!(e.state.cards[aye.index()].locked);
    assert!(
        !e.state.minions[r.index()].locked,
        "the Aye locked, not him"
    );
}

// ---- Sense Vibrations -------------------------------------------------------

/// Sense Vibrations: "This Laibon with Auspex may lock this card during a
/// referendum to get 1 additional vote." His own Hiding in the Open, no
/// other vote: passed with it, failed without.
#[test]
fn sense_vibrations_votes_once() {
    for vibrate in [true, false] {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), of_sect_with(Sect::Laibon, "aus"), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let sv = in_play(&mut e, 0, "Sense Vibrations", Some(l));
        hand(&mut e, 0, &["Hiding in the Open"]);
        pick(&mut e, play(0, 0, l));
        if vibrate {
            pick_when(&mut e, ability(sv.0, 0));
            pick_when(&mut e, votes(l, true));
        }
        close_polling(&mut e);
        to_minion_phase(&mut e, SeatId(1));
        assert_eq!(on(&e, "Hiding in the Open", l).len(), vibrate as usize);
    }
}

// ---- Unholy Radiance, Well-Marked -------------------------------------------

/// Unholy Radiance: "Requires a Laibon with three or more Orun. Only usable
/// during a referendum. … Gain 4 votes." His Hiding in the Open passes on
/// them.
#[test]
fn unholy_radiance_gives_four_votes() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for _ in 0..3 {
        in_play(&mut e, 0, "Orun", Some(l));
    }
    hand(&mut e, 0, &["Hiding in the Open", "Unholy Radiance"]);
    pick(&mut e, play(0, 0, l));
    pick_when(&mut e, play_at(0, 0, Some(l), Target::None));
    pick_when(&mut e, votes(l, true));
    close_polling(&mut e);
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(on(&e, "Hiding in the Open", l).len(), 1);
}

/// Well-Marked: "Put this card on this Laibon. If his or her capacity is 5
/// or more, he or she unlocks." On a capacity 5+ Laibon: unlocked.
#[test]
fn well_marked_unlocks_a_big_laibon() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Fish"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[v.index()].locked = true;
    hand(&mut e, 0, &["Well-Marked"]);
    pick_when(&mut e, play_at(0, 0, Some(l), Target::None));
    request(&mut e);
    assert_eq!(on(&e, "Well-Marked", l).len(), 1);
    assert!(!e.state.minions[l.index()].locked);
}

// ---- Supernatural Resistance ------------------------------------------------

/// Supernatural Resistance: "Lock an Orun on this Laibon to cancel an
/// action card targeting this Laibon … Not usable to cancel a card that
/// doesn't require a Discipline or that requires Celerity or Potence."
/// Seat 1's Exile [pre] at him: canceled, an Orun locked.
// Rulings: [ANK 20200813-2].
#[test]
fn supernatural_resistance_cancels_an_action_at_its_laibon() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), vampire_with("PRE", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let oruns: Vec<CardId> = (0..3)
        .map(|_| in_play(&mut e, 0, "Orun", Some(l)))
        .collect();
    hand(&mut e, 0, &["Supernatural Resistance"]);
    hand(&mut e, 1, &["Exile"]);
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, play_at(0, 0, Some(p), Target::Minion(l)));
    pick_when(&mut e, play_at(0, 0, Some(l), Target::None));
    request(&mut e);
    assert_eq!(
        oruns
            .iter()
            .filter(|c| e.state.cards[c.index()].locked)
            .count(),
        1
    );
    assert!(e.state.seats[1].ash.contains(&find_library("Exile")));
    assert!(!e.state.minions[l.index()].locked, "the lock canceled");
}

// ---- Taking the Skin: Vulture -----------------------------------------------

/// Taking the Skin: Vulture [abo]: "+1 intercept, and put this card on this
/// vampire. This vampire has flight [FLIGHT]. Burn this card during your
/// next unlock phase." Blocking a +1 stealth bleed: on him, flying; gone at
/// his next unlock phase.
#[test]
fn taking_the_skin_vulture_flies_until_the_unlock_phase() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), vampire_with("abo", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let a = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Taking the Skin: Vulture"]);
    to_minion_phase(&mut e, SeatId(1));
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Stealth(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(d));
    pick_when(&mut e, play_at(0, 1, Some(d), Target::None));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(on(&e, "Taking the Skin: Vulture", d).len(), 1);
    assert!(e.state.discipline_level(d, "flight") > 0, "flight");
    to_minion_phase(&mut e, SeatId(0));
    assert!(on(&e, "Taking the Skin: Vulture", d).is_empty(), "burned");
}

// ---- Taming the Beast -------------------------------------------------------

/// Taming the Beast [abo]: "Strike: hand strike at +1 damage." Blocked as
/// he bleeds, his strike does 1 + 1.
// Rulings: [TOM 19960225].
#[test]
fn taming_the_beast_strikes_at_plus_one() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Socrate Cidibe"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Taming the Beast"]);
    pick_when(&mut e, bleed(s));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play_at(0, 1, Some(s), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 6 - 2);
}

// ---- Transcendent Laibon ----------------------------------------------------

/// Transcendent Laibon: "Allies and younger vampires must burn 1 blood or
/// life to attempt to block this Laibon … Burn this card if this minion …
/// has fewer than three Aye or three Orun." A younger blocker burns 1
/// blood; an Aye gone, the card burns.
#[test]
fn transcendent_laibon_taxes_small_blockers_and_needs_its_cards() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Fish"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain_cap("Ventrue", 1..=4), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let ayes: Vec<CardId> = (0..3).map(|_| in_play(&mut e, 0, "Aye", Some(l))).collect();
    for _ in 0..3 {
        in_play(&mut e, 0, "Orun", Some(l));
    }
    hand(&mut e, 0, &["Transcendent Laibon"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    master_on(&mut e, "Transcendent Laibon", l);
    assert_eq!(on(&e, "Transcendent Laibon", l).len(), 1);
    pick_when(&mut e, bleed(l));
    pick_when(&mut e, block(y));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, y), 3 - 1 - 1, "the toll, then 1 damage");
    e.state.cards[ayes[0].index()].in_play = false;
    request(&mut e);
    assert!(on(&e, "Transcendent Laibon", l).is_empty(), "two Aye left");
}

// ---- Tunnel Runner ----------------------------------------------------------

/// Tunnel Runner: "A vampire stealing life from the Runner takes 1 damage
/// for each point stolen." Tier of Souls [ani] on it: the stealer 1 blood
/// lower than against another ally.
// Rulings: [RTR 19970630].
#[test]
fn tunnel_runner_hurts_who_steals_its_life() {
    let mut ends = Vec::new();
    for name in ["Tunnel Runner", "Gravebound Drone"] {
        let mut e = game(2);
        let s = e.add_minion(SeatId(0), vampire_with("ani", 0), 2, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let ally = e.add_ally(SeatId(1), find_library(name));
        let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        e.state.minions[w.index()].locked = true;
        e.state.minions[ally.index()].locked = true;
        let life = blood(&e, ally);
        hand(&mut e, 0, &["Tier of Souls"]);
        pick_when(&mut e, play_at(0, 0, Some(s), Target::Minion(ally)));
        request(&mut e);
        assert_eq!(blood(&e, ally), life - 1);
        ends.push(blood(&e, s));
    }
    assert_eq!(ends[0] + 1, ends[1], "{ends:?}");
}

// ---- Vampires (group 5) -----------------------------------------------------

/// Aisata Swanou: "During your master phase, you may draw a card and move a
/// card from your hand to either your ash heap or the top of your library."
/// The drawn Bonding back on top.
#[test]
fn aisata_swanou_draws_and_stacks_her_library() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Aisata Swanou"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let bonding = find_library("Bonding");
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    e.state.seats[0].library = vec![find_library("Deflection"), bonding].into();
    pick_when(&mut e, own_ability(a, 0));
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Hand,
            card: bonding,
        },
    );
    // Before the hand draws back up.
    assert!(!e.state.seats[0].hand.contains(&bonding));
    assert_eq!(e.state.seats[0].library.last().copied(), Some(bonding));
}

/// A vampire's own granted action, no target.
fn own_action(m: MinionId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(m),
        ability,
        actor: Some(m),
        target: Target::None,
    }
}

/// A vampire's own ability, no target.
fn own_ability(m: MinionId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(m),
        ability,
        actor: None,
        target: Target::None,
    }
}

/// Alan Sovereign: his investment text inert (no investment card in the
/// pool): nothing to use.
#[test]
fn alan_sovereign_has_nothing_to_use() {
    let card = find_card("Alan Sovereign");
    let def = zvtes::cards::vampire(card).expect("defined");
    assert!(def.abilities.is_empty() && def.printed.is_empty() && def.riders.is_empty());
}

/// Alejandro Aguirre: "he may lock before range is determined in any combat
/// that does not involve him to inflict 1 unpreventable damage on each
/// combatant."
#[test]
fn alejandro_aguirre_hurts_both_combatants() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Alejandro Aguirre"), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.notes = Some(Vec::new());
    let before = e.table();
    pick_when(&mut e, bleed(b));
    pick_when(&mut e, block(v));
    pick_when(&mut e, own_ability(a, 0));
    request(&mut e);
    assert!(e.state.minions[a.index()].locked);
    let hurt: Vec<_> = noted_changes(&mut e, before)
        .into_iter()
        .filter(|(c, _, n)| *n == -1 && matches!(c, Some(zvtes::notes::Cause::Effect(zvtes::notes::Source::Minion(x))) if *x == a))
        .collect();
    assert_eq!(hurt.len(), 2, "{hurt:?}");
}

/// Alfred Benezri: "-1 strength in combat with an ally."
#[test]
fn alfred_benezri_is_weaker_against_an_ally() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Alfred Benezri"), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Gravebound Drone"));
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let vs = |o| zvtes::state::PrintedCtx {
        opponent: Some(o),
        ..Default::default()
    };
    assert_eq!(
        e.state
            .printed_mod(a, zvtes::cards::Stat::Strength, vs(ally)),
        -1
    );
    assert_eq!(
        e.state.printed_mod(a, zvtes::cards::Stat::Strength, vs(v)),
        0
    );
}

/// Alonzo Guillen: "During your unlock phase and your discard phase, if
/// Alonzo is ready, you may burn the Edge to look at another Methuselah's
/// hand."
#[test]
fn alonzo_guillen_trades_the_edge_for_a_look() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Alonzo Guillen"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].has_edge = true;
    to_minion_phase(&mut e, SeatId(1));
    let look = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(a),
        ability: 0,
        actor: None,
        target: Target::Seat(SeatId(1)),
    };
    pick_when(&mut e, look);
    request(&mut e);
    assert!(!e.state.seats[0].has_edge);
}

/// Amelia: "After combat with Amelia, the opposing minion burns 1 blood or
/// life."
#[test]
fn amelia_drains_her_combat_opponent() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        find_card("Amelia, The Blood Red Tears"),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 6 - 1 - 1, "her strike, then the burn");
}

/// Anastasz di Zagreb: "If there are any other justicars ready, Anastasz
/// gets 1 fewer vote from his justicar title. Anastasz may steal 1 blood as
/// a ranged strike."
#[test]
fn anastasz_di_zagreb_shares_justice() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Anastasz di Zagreb"), 4, false);
    let votes = e
        .state
        .printed_mod(a, zvtes::cards::Stat::Votes, Default::default());
    let j = (0..CRYPT.len() as u16)
        .find(|&c| {
            matches!(CRYPT[c as usize].title, Title::Justicar(_))
                && CRYPT[c as usize].name != "Anastasz di Zagreb"
        })
        .expect("another justicar");
    e.add_minion(SeatId(1), j, 4, false);
    assert_eq!(
        e.state
            .printed_mod(a, zvtes::cards::Stat::Votes, Default::default()),
        votes - 1
    );
}

/// Annabelle Triabell (G3): "Annabelle can give Toreador vampires +1 bleed
/// for the remainder of the turn as a +1 stealth action that costs 1 blood.
/// If that action is successful, Annabelle unlocks." Another Toreador
/// bleeds at +1; she unlocks; next turn, back to normal.
// Rulings: [LSJ 20030811].
#[test]
fn annabelle_triabell_g3_rallies_the_toreador() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        find_card_group("Annabelle Triabell", 3),
        4,
        false,
    );
    let t = e.add_minion(SeatId(0), plain("Toreador", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[v.index()].locked = true;
    let base = e.state.current_bleed(t);
    pick_when(&mut e, own_action(a, 0));
    request(&mut e);
    assert!(!e.state.minions[a.index()].locked, "successful: unlocked");
    assert_eq!(blood(&e, a), 3);
    assert_eq!(e.state.current_bleed(t), base + 1);
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.current_bleed(t), base);
}

/// The crypt card of `name` in `group`.
fn find_card_group(name: &str, group: u8) -> u16 {
    (0..CRYPT.len() as u16)
        .find(|&c| {
            let v = &CRYPT[c as usize];
            v.name == name && v.group == group && !v.adv
        })
        .expect("in the crypt")
}

/// Antonio Veradas: "During your discard phase, Antonio may burn 1 blood
/// to unlock."
#[test]
fn antonio_veradas_unlocks_in_the_discard_phase() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Antonio Veradas"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[v.index()].locked = true;
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, own_ability(a, 0));
    request(&mut e);
    assert!(!e.state.minions[a.index()].locked);
}

/// Arriette Sylla: "If Arriette has no Orun, she gets -1 bleed."
#[test]
fn arriette_sylla_bleeds_less_without_orun() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Arriette Sylla"), 3, false);
    let without = e.state.current_bleed(a);
    in_play(&mut e, 0, "Orun", Some(a));
    assert_eq!(e.state.current_bleed(a), without + 1);
}

// ---- Vampires (group 6) -----------------------------------------------------

/// A printed stat of `m` against `opponent`.
fn vs(e: &Engine, m: MinionId, stat: zvtes::cards::Stat, opponent: MinionId) -> i32 {
    e.state.printed_mod(
        m,
        stat,
        zvtes::state::PrintedCtx {
            opponent: Some(opponent),
            ..Default::default()
        },
    )
}

/// Barth: "Vampires of capacity 3 or less controlled by your prey or
/// predator get -1 stealth when Barth is ready and unlocked."
#[test]
fn barth_slows_small_vampires_of_his_neighbors() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Barth"), 4, false);
    let small = e.add_minion(SeatId(1), plain_cap("Ventrue", 1..=3), 2, false);
    let big = e.add_minion(SeatId(1), plain_cap("Ventrue", 5..=11), 4, false);
    let stealth = |e: &Engine, m| {
        e.state
            .printed_mod(m, zvtes::cards::Stat::Stealth, Default::default())
    };
    assert_eq!(stealth(&e, small), -1);
    assert_eq!(stealth(&e, big), 0);
    e.state.minions[b.index()].locked = true;
    assert_eq!(stealth(&e, small), 0, "locked: none");
}

/// Batsheva (advanced): "+1 strength in combat with a Malkavian."
#[test]
fn batsheva_is_stronger_against_a_malkavian() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card_adv("Batsheva"), 4, false);
    let m = e.add_minion(SeatId(1), plain("Malkavian", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert_eq!(vs(&e, b, zvtes::cards::Stat::Strength, m), 1);
    assert_eq!(vs(&e, b, zvtes::cards::Stat::Strength, v), 0);
}

/// The advanced crypt card of `name`.
fn find_card_adv(name: &str) -> u16 {
    (0..CRYPT.len() as u16)
        .find(|&c| CRYPT[c as usize].name == name && CRYPT[c as usize].adv)
        .expect("advanced")
}

/// Casino Reeds: "Casino may move up to 3 blood from himself to a younger
/// Nosferatu in your uncontrolled region as an action. Casino inflicts +1
/// damage with ranged strikes."
#[test]
fn casino_reeds_feeds_a_younger_nosferatu() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Casino Reeds"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[v.index()].locked = true;
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: plain_cap("Nosferatu", 1..=4),
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    assert_eq!(
        e.state.printed_mod(
            c,
            zvtes::cards::Stat::RangedStrikeDamage,
            Default::default()
        ),
        1
    );
    let feed = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(c),
        ability: 0,
        actor: Some(c),
        target: Target::Uncontrolled(0),
    };
    pick_when(&mut e, feed);
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 3);
    assert_eq!(blood(&e, c), 6 - 3);
}

/// Cesewayo (advanced): "During an action directed at you, Cesewayo may
/// lock an Aye on him to unlock." Once each such action (rules-digest D7):
/// locked again, a second Aye does not unlock him.
#[test]
fn cesewayo_unlocks_for_an_aye() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card_adv("Cesewayo"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let a = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let aye = in_play(&mut e, 0, "Aye", Some(c));
    in_play(&mut e, 0, "Aye", Some(c));
    to_minion_phase(&mut e, SeatId(1));
    e.state.minions[c.index()].locked = true;
    pick(&mut e, bleed(a));
    pick_when(&mut e, own_ability(c, 0));
    request(&mut e);
    assert!(!e.state.minions[c.index()].locked);
    assert!(e.state.cards[aye.index()].locked);
    e.state.minions[c.index()].locked = true;
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        own_ability(c, 0),
    );
}

/// Cock Robin: "Once per turn, when Cock Robin successfully performs an
/// action that requires Animalism [ani], he unlocks at the end of the
/// action." Tier of Souls [ani]: unlocked after it.
#[test]
fn cock_robin_unlocks_after_an_animalism_action() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Cock Robin"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[w.index()].locked = true;
    e.state.minions[v.index()].locked = true;
    hand(&mut e, 0, &["Tier of Souls"]);
    pick_when(&mut e, play_at(0, 0, Some(r), Target::Minion(v)));
    request(&mut e);
    assert!(!e.state.minions[r.index()].locked);
}

/// David Morgan: "+1 intercept when attempting to block a vampire of
/// capacity 2 or less."
#[test]
fn david_morgan_watches_small_vampires() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("David Morgan, The Scourge"), 4, false);
    let small = e.add_minion(SeatId(1), plain_cap("Ventrue", 1..=2), 2, false);
    let big = e.add_minion(SeatId(1), plain_cap("Ventrue", 4..=11), 4, false);
    assert_eq!(vs(&e, d, zvtes::cards::Stat::Intercept, small), 1);
    assert_eq!(vs(&e, d, zvtes::cards::Stat::Intercept, big), 0);
}

/// Edith Blount: "If Enid Blount is in your ready region, Edith gets an
/// optional maneuver and an optional press each combat."
#[test]
fn edith_blount_maneuvers_beside_enid() {
    let mut e = game(2);
    let ed = e.add_minion(SeatId(0), find_card("Edith Blount"), 4, false);
    e.add_minion(SeatId(0), find_card("Enid Blount"), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick_when(&mut e, bleed(ed));
    pick_when(&mut e, block(b));
    pick_when(&mut e, own_ability(ed, 0));
}

/// Edward Neally: "If Edward is ready, you can use a master phase action
/// to increase your hand size by 1 card for the remainder of the turn."
#[test]
fn edward_neally_widens_the_hand() {
    let mut e = game(2);
    let ed = e.add_minion(SeatId(0), find_card("Edward Neally"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let size = e.state.hand_size(SeatId(0));
    pick_when(&mut e, own_ability(ed, 0));
    request(&mut e);
    assert_eq!(e.state.hand_size(SeatId(0)), size + 1);
}

/// Ellison Humboldt: "Princes cannot block Ellison."
#[test]
fn ellison_humboldt_slips_past_princes() {
    let mut e = game(2);
    let el = e.add_minion(SeatId(0), find_card("Ellison Humboldt"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), prince(), 4, false);
    let o = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick_when(&mut e, bleed(el));
    let req = request(&mut e);
    assert!(req.options.contains(&block(o)));
    assert!(!req.options.contains(&block(p)));
}

/// Enid Blount: "If Edith Blount is in your ready region, she may burn a
/// blood to give Enid +1 stealth for the current action." Blocked at 0
/// against 0: Edith's blood, and the block fails.
#[test]
fn enid_blount_slips_away_on_ediths_blood() {
    let mut e = game(2);
    let en = e.add_minion(SeatId(0), find_card("Enid Blount"), 4, false);
    let ed = e.add_minion(SeatId(0), find_card("Edith Blount"), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let pool = e.state.seats[1].pool;
    pick_when(&mut e, bleed(en));
    pick_when(&mut e, block(b));
    pick_when(&mut e, own_ability(en, 0));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(blood(&e, ed), 3);
    assert!(e.state.seats[1].pool < pool, "unblocked");
}

// ---- Vampires (group 7) -----------------------------------------------------

/// Erichtho: "Actions to recruit or employ mages cost Erichtho 1 less pool
/// or blood."
#[test]
fn erichtho_recruits_mages_cheaper() {
    let mut e = game(2);
    let er = e.add_minion(SeatId(0), find_card("Erichtho"), 4, false);
    let mage = (0..zvtes::library::LIBRARY.len() as u16)
        .find(|&c| {
            zvtes::library::LIBRARY[c as usize]
                .labels
                .contains(&zvtes::library::Label::Mage)
        })
        .expect("a mage");
    let ctx = zvtes::state::PrintedCtx {
        card: Some((mage, 0)),
        ..Default::default()
    };
    assert_eq!(
        e.state
            .printed_mod(er, zvtes::cards::Stat::ActionRecruitCostOff, ctx),
        1
    );
}

/// Esau: "When Esau declares a bleed action, he may burn 1 blood to flip a
/// coin. If it's heads, Esau gets an additional +1 bleed for that action.
/// +1 bleed." Burned: a bleed for 2 or 3.
#[test]
fn esau_flips_for_more_bleed() {
    let mut e = game(2);
    let es = e.add_minion(SeatId(0), find_card("Esau"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[v.index()].locked = true;
    let pool = e.state.seats[1].pool;
    pick_when(&mut e, bleed(es));
    pick_when(&mut e, own_ability(es, 0));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(blood(&e, es), 3);
    let lost = pool - e.state.seats[1].pool;
    assert!((2..=3).contains(&lost), "{lost}");
}

/// Fleurdumal: "If Fleurdumal is blocked, she may burn 1 blood before
/// combat to force the opposing minion's controller to play with an open
/// hand during the resulting combat."
#[test]
fn fleurdumal_opens_the_blockers_hand() {
    let mut e = game(2);
    let f = e.add_minion(SeatId(0), find_card("Fleurdumal"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("pot", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Immortal Grapple"]);
    pick_when(&mut e, bleed(f));
    pick_when(&mut e, block(b));
    pick_when(&mut e, own_ability(f, 0));
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(1));
    assert!(e.state.seats[1].combat_open_hand);
    assert_eq!(blood(&e, f), 3);
}

/// Gemini: "During your unlock phase, if Gemini is ready, you may move a
/// blood from a minion in your prey's uncontrolled region to his or her
/// pool."
#[test]
fn gemini_drains_the_preys_uncontrolled_region() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Gemini"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: plain("Ventrue", 1),
            copy: Default::default(),
            counters: 2,
            banished: None,
        });
    to_minion_phase(&mut e, SeatId(1));
    let pool = e.state.seats[1].pool;
    pick_when(&mut e, own_ability(g, 0));
    request(&mut e);
    assert_eq!(e.state.seats[1].uncontrolled[0].counters, 1);
    assert_eq!(e.state.seats[1].pool, pool + 1);
}

/// Gengis (G3): "Any Brujah controlled by another Methuselah can take a Ⓓ
/// action to unlock Gengis and take control of him until the end of the
/// turn. Gengis cannot block that action."
// Rulings: [LSJ 20080604].
#[test]
fn gengis_g3_goes_over_to_a_brujah() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card_group("Gengis", 3), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let br = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    to_minion_phase(&mut e, SeatId(1));
    e.state.minions[g.index()].locked = true;
    let take = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(g),
        ability: 0,
        actor: Some(br),
        target: Target::Minion(g),
    };
    pick(&mut e, take);
    let req = request(&mut e);
    assert!(!req.options.contains(&block(g)), "not he");
    to_minion_phase(&mut e, SeatId(1));
    let v = &e.state.minions[g.index()];
    assert_eq!(v.owner, SeatId(1));
    assert!(!v.locked);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(
        e.state.minions[g.index()].owner,
        SeatId(0),
        "back at the turn's end"
    );
}

/// Gerard Rafin: "During your unlock phase, if Gerard is ready, you may look
/// at 1 card at random from your prey's hand."
#[test]
fn gerard_rafin_looks_at_a_random_card() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Gerard Rafin"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Bonding", "Deflection"]);
    to_minion_phase(&mut e, SeatId(1));
    e.notes = Some(Vec::new());
    pick_when(&mut e, own_ability(g, 0));
    request(&mut e);
    let looks = noted_looks(&mut e);
    assert_eq!(looks.len(), 1);
    assert_eq!(looks[0].cards.len(), 1);
    assert_eq!(looks[0].seat, SeatId(1));
}

/// Gracis Nostinus: "If a younger vampire attempts to block Gracis and
/// fails, lock that vampire at the end of the action."
#[test]
fn gracis_nostinus_locks_younger_failed_blockers() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Gracis Nostinus"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain_cap("Ventrue", 1..=3), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[g.index()].timed.push((
        zvtes::cards::Lasting::Stealth(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    pick_when(&mut e, bleed(g));
    pick_when(&mut e, block(y));
    let mut req = request(&mut e);
    while req.window == WindowLabel::ActionBlock {
        e.choose(index_of(&req, Choice::Pass));
        req = request(&mut e);
    }
    assert_eq!(req.window, WindowLabel::MinionPhase, "the action over");
    assert!(e.state.minions[y.index()].locked);
}

/// Horatio Ballard: "If Horatio doesn't have a title, he can call a
/// referendum to become the Prince of Chicago as a +1 stealth political
/// action." Passed: Prince of Chicago.
#[test]
fn horatio_ballard_becomes_prince_of_chicago() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Horatio Ballard"), 4, false);
    let m = e.add_minion(SeatId(0), find_card("Mamadou Keita"), 7, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[m.index()].locked = true;
    pick_when(&mut e, own_action(h, 0));
    pick_when(&mut e, votes(m, true));
    close_polling(&mut e);
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.title_of(h), Title::Prince("Chicago"));
}

/// Hrothulf: "If you control the Edge, Hrothulf can burn it to enter combat
/// with a ready minion controlled by another Methuselah as a Ⓓ action."
#[test]
fn hrothulf_burns_the_edge_to_fight() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hrothulf"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[w.index()].locked = true;
    e.state.seats[0].has_edge = true;
    let fight = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(h),
        ability: 0,
        actor: Some(h),
        target: Target::Minion(v),
    };
    pick_when(&mut e, fight);
    to_minion_phase(&mut e, SeatId(1));
    assert!(!e.state.seats[0].has_edge);
    assert!(blood(&e, v) < 4, "they fought");
}

/// Ilyana Ravidovich: "Once each action, Ilyana may burn 1 blood to get +1
/// intercept." Blocking a +1 stealth bleed: she reaches it.
#[test]
fn ilyana_ravidovich_burns_for_intercept() {
    let mut e = game(2);
    let i = e.add_minion(SeatId(0), find_card("Ilyana Ravidovich"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let a = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    to_minion_phase(&mut e, SeatId(1));
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Stealth(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    let pool = e.state.seats[0].pool;
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(i));
    pick_when(&mut e, own_ability(i, 0));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.seats[0].pool, pool, "blocked");
}

// ---- Vampires (group 8) -----------------------------------------------------

/// Ozzy: "During your unlock phase, roll a 6-sided die. On a 1 or 2, Ozzy
/// gets +1 stealth, on a 5 or 6, Ozzy gets -1 bleed and +1 strength. This
/// effect lasts until your next unlock phase." One of the three outcomes,
/// gone by the next.
#[test]
fn ozzy_rolls_a_die_each_unlock_phase() {
    let mut e = game(2);
    let o = e.add_minion(
        SeatId(0),
        find_card("J. Oswald \"Ozzy\" Hyde-White"),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(0));
    let lasting: Vec<_> = e.state.minions[o.index()]
        .timed
        .iter()
        .map(|t| t.0)
        .collect();
    use zvtes::cards::Lasting as L;
    assert!(
        lasting.is_empty()
            || lasting == [L::Stealth(1)]
            || lasting == [L::Bleed(-1), L::Strength(1)],
        "{lasting:?}"
    );
}

/// Jara Drory: "During your unlock phase, you may look at 1 card at random
/// from your predator's hand, and Jara may burn 1 blood to exchange that
/// card with a card from your predator's ash heap."
#[test]
fn jara_drory_swaps_a_card_of_her_predator() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jara Drory"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    to_minion_phase(&mut e, SeatId(1));
    hand(&mut e, 1, &["Bonding"]);
    e.state.seats[1]
        .ash
        .put(find_library("Deflection"), Default::default());
    pick_when(&mut e, own_ability(j, 0));
    pick_when(&mut e, Choice::FindCard(find_library("Deflection")));
    request(&mut e);
    assert!(e.state.seats[1].hand.contains(&find_library("Deflection")));
    assert!(e.state.seats[1].ash.contains(&find_library("Bonding")));
    assert_eq!(blood(&e, j), 3);
}

/// Jaroslav Pascek: "He can inflict 1 damage on each of your prey's Sabbat
/// vampires as a Ⓓ action."
#[test]
fn jaroslav_pascek_hurts_the_preys_sabbat() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jaroslav Pascek"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let s1 = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 0), 4, false);
    let s2 = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 1), 4, false);
    e.state.minions[s1.index()].locked = true;
    e.state.minions[s2.index()].locked = true;
    let act = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(j),
        ability: 0,
        actor: Some(j),
        target: Target::Seat(SeatId(1)),
    };
    pick_when(&mut e, act);
    request(&mut e);
    assert_eq!((blood(&e, s1), blood(&e, s2)), (3, 3));
}

/// Jason: "If Jason is blocked, the blocking vampire burns 1 blood after
/// the resulting combat."
#[test]
fn jason_drains_his_blocker() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jason, The World's Voice"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick_when(&mut e, bleed(j));
    pick_when(&mut e, block(b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 6 - 1 - 1, "his strike, then the burn");
}

/// Jibade el-Bahrawi (advanced): "he may burn a blood and lock X Aye on him
/// to prevent X damage to any ally or retainer in combat." His ally's 1
/// damage prevented, an Aye locked.
#[test]
fn jibade_shields_an_ally_with_his_aye() {
    let mut e = game(2);
    let jb = e.add_minion(SeatId(0), find_card_adv("Jibade el-Bahrawi"), 5, false);
    let ally = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let aye = in_play(&mut e, 0, "Aye", Some(jb));
    let life = blood(&e, ally);
    pick_when(&mut e, bleed(ally));
    pick_when(&mut e, block(b));
    let shield = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(jb),
        ability: 1,
        actor: None,
        target: Target::Minion(ally),
    };
    pick_when(&mut e, shield);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ally), life, "prevented");
    assert!(e.state.cards[aye.index()].locked);
    assert_eq!(blood(&e, jb), 4);
}

/// Joshua Tarnopolski: "Allies and retainers cost Joshua 1 less … He gets
/// +2 strength in combat with a Sabbat vampire."
#[test]
fn joshua_tarnopolski_recruits_cheaper_and_hates_the_sabbat() {
    let mut e = game(2);
    let jo = e.add_minion(SeatId(0), find_card("Joshua Tarnopolski"), 4, false);
    let s = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 0), 4, false);
    let c = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert_eq!(vs(&e, jo, zvtes::cards::Stat::Strength, s), 2);
    assert_eq!(vs(&e, jo, zvtes::cards::Stat::Strength, c), 0);
    let ctx = zvtes::state::PrintedCtx {
        card: Some((find_library("Gravebound Drone"), 0)),
        ..Default::default()
    };
    assert_eq!(e.state.printed_mod(jo, zvtes::cards::Stat::CostOff, ctx), 1);
}

/// Julio Martinez: "Julio may burn a blood to get +1 intercept when
/// attempting to block a Camarilla vampire." Blocking a +1 stealth
/// Camarilla bleed: he reaches it.
#[test]
fn julio_martinez_burns_to_block_the_camarilla() {
    let mut e = game(2);
    let ju = e.add_minion(SeatId(0), find_card("Julio Martinez"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let a = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    to_minion_phase(&mut e, SeatId(1));
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Stealth(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    let pool = e.state.seats[0].pool;
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(ju));
    pick_when(&mut e, own_ability(ju, 0));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.seats[0].pool, pool, "blocked");
}

/// Kamiri wa Itherero (advanced): "Kamiri can inflict 2 unpreventable
/// damage on a minion as a Ⓓ action."
#[test]
fn kamiri_wa_itherero_damages_a_minion() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card_adv("Kamiri wa Itherero"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[v.index()].locked = true;
    let hit = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(k),
        ability: 0,
        actor: Some(k),
        target: Target::Minion(v),
    };
    pick_when(&mut e, hit);
    request(&mut e);
    assert_eq!(blood(&e, v), 2);
}

/// Katarina Kornfeld: "If another ready vampire you control is a prince,
/// Katarina gets +1 stealth."
#[test]
fn katarina_kornfeld_hides_behind_a_prince() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Katarina Kornfeld"), 4, false);
    let st = |e: &Engine| {
        e.state
            .printed_mod(k, zvtes::cards::Stat::Stealth, Default::default())
    };
    assert_eq!(st(&e), 0);
    e.add_minion(SeatId(0), prince(), 4, false);
    assert_eq!(st(&e), 1);
}

/// Khalid: "you may use a master phase action and discard a card that
/// requires Obfuscate [obf] from your hand to gain a pool."
#[test]
fn khalid_trades_an_obfuscate_card_for_pool() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Khalid"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let obf = (0..zvtes::library::LIBRARY.len() as u16)
        .find(|&c| {
            zvtes::library::LIBRARY[c as usize].disciplines == ["obf"]
                && zvtes::cards::lookup(c).is_some()
        })
        .expect("an Obfuscate card");
    e.state.seats[0].hand.put(obf, Default::default());
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let pool = e.state.seats[0].pool;
    pick_when(&mut e, own_ability(k, 0));
    // The one Obfuscate card: its discard elided.
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, pool + 1);
    assert!(e.state.seats[0].ash.contains(&obf));
}

// ---- Vampires, group 9 ------------------------------------------------------

/// Kurt Densch: "During your unlock phase, you may look at the top card of
/// your library, and Kurt may burn 1 blood to move that card to the bottom
/// of your library. +1 intercept." The move asked after the look.
#[test]
fn kurt_densch_moves_the_looked_card_to_the_bottom() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Kurt Densch"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    fill_hand(&mut e, 0);
    for name in ["Deflection", "Conditioning", "Govern the Unaligned"] {
        e.state.seats[0].library.push(find_library(name));
    }
    assert_eq!(
        e.state
            .printed_mod(k, zvtes::cards::Stat::Intercept, Default::default()),
        1
    );
    pick_when(&mut e, own_ability(k, 0));
    pick(&mut e, Choice::ResolvePrinted { minion: k });
    let library = e.state.seats[0].library.to_cards();
    assert_eq!(library[0], find_library("Govern the Unaligned"));
    assert_eq!(library[2], find_library("Conditioning"), "the new top");
    assert_eq!(blood(&e, k), 3);
}

/// Kyoko Shinsegawa: "Kyoko cannot hunt as normal. She may hunt by
/// stealing a blood from another ready vampire as a +1 stealth Ⓓ action."
/// No rulebook hunt; the steal is her hunt, the one an empty Kyoko must
/// take.
// Rulings: [LSJ 20050720-2] [RTR 20010711] [LSJ 20100915]. [RTR 20180511]
// [RTR 20030519]. [LSJ 19970224]. [RBK action-card-or-card-in-play].
#[test]
fn kyoko_hunts_by_stealing_blood() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Kyoko Shinsegawa"), 0, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[w.index()].locked = true;
    let hunt = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(k),
        ability: 0,
        actor: Some(k),
        target: Target::Minion(v),
    };
    let req = request(&mut e);
    assert!(!req.options.contains(&action(k, ActionKind::Hunt)));
    assert!(req.options.contains(&hunt), "the hunt she must take");
    pick(&mut e, hunt);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, k), 1);
    assert_eq!(blood(&e, v), 3);
    // Once each turn: unlocked again, not offered.
    e.state.minions[k.index()].locked = false;
    assert!(!request(&mut e).options.contains(&hunt));
}

/// Lille Haake: "+1 bleed when bleeding a Methuselah who controls a ready
/// Sabbat vampire. During your master phase, you may use a master phase
/// action to discard a card from your hand." Bled for 2; the card
/// discarded and replaced.
#[test]
fn lille_haake_bleeds_sabbat_holders_and_discards() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lille Haake"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let s = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[s.index()].locked = true;
    pick(&mut e, bleed(l));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.seats[1].pool, 10 - 2);
    hand(&mut e, 0, &["Deflection"]);
    for _ in 0..3 {
        e.state.seats[0].library.push(find_library("Conditioning"));
    }
    pick_when(&mut e, own_ability(l, 0));
    pick(&mut e, Choice::Discard(0));
    request(&mut e);
    assert!(e.state.seats[0].ash.contains(&find_library("Deflection")));
    assert!(!e.state.seats[0].hand.contains(&find_library("Deflection")));
}

/// Luanda Magere: "+1 strength for each Orun he has." Two Orun: +2.
#[test]
fn luanda_magere_is_strong_per_orun() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Luanda Magere"), 4, false);
    let strength = |e: &Engine| {
        e.state
            .printed_mod(l, zvtes::cards::Stat::Strength, Default::default())
    };
    assert_eq!(strength(&e), 0);
    in_play(&mut e, 0, "Orun", Some(l));
    in_play(&mut e, 0, "Orun", Some(l));
    assert_eq!(strength(&e), 2);
}

/// Lucas Halton: "Once each turn, when Lucas blocks an action, he may burn
/// a blood instead of locking for the successful block." Unlocked for 1
/// blood; not offered at his second block that turn.
// Rulings: [ANK 20180719-4].
#[test]
fn lucas_halton_burns_blood_instead_of_locking_once_a_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let l = e.add_minion(SeatId(1), find_card("Lucas Halton"), 10, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(l));
    pick(&mut e, Choice::ResolvePrinted { minion: l });
    assert!(!e.state.minions[l.index()].locked);
    assert_eq!(blood(&e, l), 9);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(l));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::UnlockAbilities,
        Choice::ResolvePrinted { minion: l },
    );
}

/// Lucian, the Perfect: "Strikes against Lucian cost an additional blood or
/// life. A minion opposing Lucian may choose not to strike. +3 bleed." The
/// blocker may pass its strike; its hand strike costs it 1 blood.
// Rulings: [LSJ 20090703].
#[test]
fn lucian_taxes_strikes_against_him() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lucian, the Perfect"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    assert_eq!(
        e.state
            .printed_mod(l, zvtes::cards::Stat::Bleed, Default::default()),
        3
    );
    pick(&mut e, bleed(l));
    pick_when(&mut e, block(b));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(
        req.options.contains(&Choice::Pass),
        "may choose not to strike"
    );
    e.choose(index_of(&req, Choice::HandStrike));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(blood(&e, b), 4 - 1 - 1, "the toll, then Lucian's hand");
}

/// Lucina: "+1 strength in combat with a vampire whose capacity is 4 or
/// less. During your unlock phase, if you control the Edge, Lucina gains 1
/// blood."
#[test]
fn lucina_strikes_the_young_and_feeds_on_the_edge() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lucina"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let small = e.add_minion(SeatId(1), plain_cap("Ventrue", 1..=4), 2, false);
    let big = e.add_minion(SeatId(1), plain_cap("Ventrue", 5..=11), 4, false);
    assert_eq!(vs(&e, l, zvtes::cards::Stat::Strength, small), 1);
    assert_eq!(vs(&e, l, zvtes::cards::Stat::Strength, big), 0);
    e.state.seats[0].has_edge = true;
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, l), 4);
    e.state.seats[0].has_edge = false;
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, l), 4, "no Edge, no blood");
}

/// Madame Guil: "+1 bleed."
#[test]
fn madame_guil_bleeds_for_one_more() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Madame Guil"), 4, false);
    assert_eq!(
        e.state
            .printed_mod(g, zvtes::cards::Stat::Bleed, Default::default()),
        1
    );
}

/// Mamadou Keita: "Mamadou may call a referendum to do 1 unpreventable
/// damage to a ready minion as a +1 stealth political action." Passed: the
/// chosen vampire loses 1 blood.
#[test]
fn mamadou_keita_damages_a_ready_minion_by_referendum() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mamadou Keita"), 7, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick(&mut e, own_action(m, 0));
    pick_when(&mut e, Choice::ChooseMinion(v));
    pick_when(&mut e, votes(m, true));
    close_polling(&mut e);
    assert_eq!(blood(&e, v), 3);
}

/// Marlena: "Marlena does not lock when performing a recruit ally action.
/// She can perform only 1 recruit ally action each turn." Unlocked after
/// recruiting; no second recruit that turn.
#[test]
fn marlena_recruits_unlocked_once_a_turn() {
    let mut e = game(2);
    e.state.seats[0].pool = 20;
    let m = e.add_minion(SeatId(0), find_card("Marlena"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Gregory Winter", "Carlton Van Wyk"]);
    pick(&mut e, play_at(0, 0, Some(m), Target::None));
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(a), .. } if *a == m),
    );
    assert!(!e.state.minions[m.index()].locked);
    assert!(e
        .state
        .minions
        .iter()
        .any(|x| x.in_play && x.name() == "Gregory Winter"));
}

// ---- Vampires, group 10 -----------------------------------------------------

/// Maxwell: "Once each turn, Maxwell can burn a blood to get +1 stealth on
/// an action that requires Presence [pre]." Enchant Kindred's bleed, Kurt
/// Densch's intercept: the stealth bought; never on a rulebook bleed.
// Rulings: [ANK 20180719-4].
#[test]
fn maxwell_buys_stealth_on_presence_actions() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Maxwell"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let k = e.add_minion(SeatId(1), find_card("Kurt Densch"), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[w.index()].locked = true;
    hand(&mut e, 0, &["Enchant Kindred"]);
    let req = request(&mut e);
    let enchant = plays(&req)
        .into_iter()
        .find(|o| matches!(o, Choice::Play { card: 0, mode: 0, actor: Some(a), .. } if *a == m))
        .expect("the bleed");
    pick(&mut e, enchant);
    pick_when(&mut e, block(k));
    pick_when(&mut e, own_ability(m, 0));
    assert_eq!(blood(&e, m), 3);

    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Maxwell"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let k = e.add_minion(SeatId(1), find_card("Kurt Densch"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(k));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        own_ability(m, 0),
    );
}

/// Maxwell: "If he doesn't have a title, he can call a referendum to become
/// the Prince of Chicago as a +1 stealth political action." Passed: Prince
/// of Chicago; once Sabbat, not offered (LSJ 20030202) — Horatio Ballard's
/// same text alike.
// Rulings: [LSJ 20030202].
#[test]
fn maxwell_becomes_prince_of_chicago_while_camarilla() {
    let mut e = game(2);
    let mx = e.add_minion(SeatId(0), find_card("Maxwell"), 4, false);
    let m = e.add_minion(SeatId(0), find_card("Mamadou Keita"), 7, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[m.index()].locked = true;
    let call = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(mx),
        ability: 1,
        actor: Some(mx),
        target: Target::None,
    };
    e.state.minions[mx.index()].sect_override = Some(Sect::Sabbat);
    assert!(!request(&mut e).options.contains(&call), "not Camarilla");
    e.state.minions[mx.index()].sect_override = None;
    pick(&mut e, call);
    pick_when(&mut e, votes(m, true));
    close_polling(&mut e);
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.title_of(mx), Title::Prince("Chicago"));
}

/// Menele: "During your unlock phase, you may move 2 blood from Menele to
/// a younger vampire in your uncontrolled region. +1 bleed." One of
/// capacity 10 or more there is not younger.
#[test]
fn menele_feeds_a_younger_uncontrolled_vampire() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card_group("Menele", 3), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let not_younger = (0..CRYPT.len() as u16)
        .find(|&c| {
            let v = &CRYPT[c as usize];
            v.capacity >= 10 && !v.imbued
        })
        .expect("a capacity 10 or more vampire");
    for card in [plain_cap("", 2..=9), not_younger] {
        e.state.seats[0]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card,
                copy: Default::default(),
                counters: 0,
                banished: None,
            });
    }
    let feed = |t: u8| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(m),
        ability: 0,
        actor: None,
        target: Target::Uncontrolled(t),
    };
    let req = offering(&mut e, feed(0));
    assert!(!req.options.contains(&feed(1)), "not younger");
    pick(&mut e, feed(0));
    request(&mut e);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 2);
    assert_eq!(blood(&e, m), 4);
}

/// Nana Buruku: "While Nana is ready, you get two master phase actions
/// (instead of one) each master phase and +1 hand size." In torpor,
/// neither.
#[test]
fn nana_buruku_gives_two_master_phase_actions_and_a_card() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nana Buruku"), 4, false);
    let size = e.state.hand_size(SeatId(0));
    let two = |e: &Engine| {
        e.state.printed_mod(
            n,
            zvtes::cards::Stat::TwoMasterPhaseActions,
            Default::default(),
        )
    };
    assert_eq!(two(&e), 1);
    e.state.minions[n.index()].in_torpor = true;
    assert_eq!(two(&e), 0);
    assert_eq!(e.state.hand_size(SeatId(0)), size - 1);
}

/// Ngozi Ekwensu: "Once each combat, Ngozi may lock an Orun on her to make
/// damage from her hand strikes aggravated for the current round." The
/// Orun locked; 2 aggravated damage burn an empty blocker (normal damage
/// would only send it to torpor).
// Rulings: [RTR 19960112].
#[test]
fn ngozi_ekwensu_locks_an_orun_for_aggravated_hands() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Ngozi Ekwensu"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let orun = in_play(&mut e, 0, "Orun", Some(n));
    e.state.minions[n.index()].timed.push((
        zvtes::cards::Lasting::Strength(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    pick(&mut e, bleed(n));
    pick_when(&mut e, block(b));
    pick_when(&mut e, own_ability(n, 0));
    to_minion_phase(&mut e, SeatId(1));
    assert!(e.state.cards[orun.index()].locked);
    assert!(!e.state.minions[b.index()].in_play, "burned");
}

/// Oliver Thrace: "Vampires opposing Oliver in combat may not end combat as
/// a strike. Once each action, he may burn a card that requires Blood
/// Sorcery [tha] from your hand to get +1 bleed." Allies may; the card to
/// the ash heap; bled for 2.
// Rulings: [ANK 20231124] [LSJ 19970718].
#[test]
fn oliver_thrace_bars_combat_ends_and_burns_blood_sorcery_to_bleed() {
    let mut e = game(2);
    let o = e.add_minion(SeatId(0), find_card("Oliver Thrace"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Gregory Winter"));
    let stat = zvtes::cards::Stat::NoCombatEnds;
    assert_eq!(vs(&e, v, stat, o), 1);
    assert_eq!(vs(&e, ally, stat, o), 0);
    e.state.minions[v.index()].locked = true;
    e.state.minions[ally.index()].locked = true;
    hand(&mut e, 0, &["Blood Rage"]);
    for _ in 0..3 {
        e.state.seats[0].library.push(find_library("Conditioning"));
    }
    pick(&mut e, bleed(o));
    pick_when(&mut e, own_ability(o, 0));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.seats[1].pool, 10 - 2);
    assert!(e.state.seats[0].ash.contains(&find_library("Blood Rage")));
}

/// Pieter van Dorn: "Pieter may burn a blood from a ready Tremere as a Ⓓ
/// action."
#[test]
fn pieter_van_dorn_burns_a_tremeres_blood() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), find_card("Pieter van Dorn"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("Tremere", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[w.index()].locked = true;
    e.state.minions[t.index()].locked = true;
    pick(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Vampire(p),
            ability: 0,
            actor: Some(p),
            target: Target::Minion(t),
        },
    );
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(blood(&e, t), 3);
}

/// Rachel Brandywine: "If Rachel is ready, at the end of your discard
/// phase, you may place cards you discard during that discard phase in
/// your library (shuffle afterward). +1 bleed." The discard, redrawn,
/// then back in the library.
// Rulings: [RTR 20030519].
#[test]
fn rachel_brandywine_shuffles_the_discards_back() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Rachel Brandywine"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Deflection"]);
    fill_hand(&mut e, 0);
    for _ in 0..3 {
        e.state.seats[0].library.push(find_library("Conditioning"));
    }
    until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    pick(&mut e, Choice::Discard(0));
    pick_when(&mut e, Choice::ResolvePrinted { minion: r });
    request(&mut e);
    let deflection = find_library("Deflection");
    assert!(!e.state.seats[0].ash.contains(&deflection));
    assert!(e.state.seats[0].library.to_cards().contains(&deflection));
    assert_eq!(e.state.active, SeatId(1), "the turn over");
}

/// Ruth McGinley: "Ruth gets first strike when striking with a gun." Her
/// .44 Magnum sends the 1-blood blocker to torpor before its hand strike.
#[test]
fn ruth_mcginley_shoots_first() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Ruth McGinley"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let gun = in_play(&mut e, 0, ".44 Magnum", Some(r));
    pick(&mut e, bleed(r));
    pick_when(&mut e, block(b));
    let shoot = |o: &Choice| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), ability: 0, .. } if *c == gun);
    let mut req = request(&mut e);
    for _ in 0..50 {
        if req.options.iter().any(shoot) {
            break;
        }
        e.choose(fallback(&req));
        req = request(&mut e);
    }
    let i = req.options.iter().position(shoot).expect("the gun");
    e.choose(i);
    to_minion_phase(&mut e, SeatId(1));
    assert!(e.state.minions[b.index()].in_torpor);
    assert_eq!(blood(&e, r), 4, "no strike back");
}

/// Sébastien Goulet: "Allies and retainers cost Sébastien 1 less blood or
/// pool. Once each action, he can burn 1 blood to get +1 bleed." Gregory
/// Winter for 1 pool; the next turn's bleed for 2.
// Rulings: [ANK 20231124] [LSJ 19970718].
#[test]
fn sebastien_goulet_recruits_cheaper_and_bleeds_harder() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Sébastien Goulet"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[v.index()].locked = true;
    e.state.minions[w.index()].locked = true;
    let cost = match zvtes::library::LIBRARY[find_library("Gregory Winter") as usize].pool_cost {
        zvtes::library::Cost::Fixed(n) => n as i32,
        _ => unreachable!(),
    };
    hand(&mut e, 0, &["Gregory Winter"]);
    pick(&mut e, play_at(0, 0, Some(s), Target::None));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.seats[0].pool, 10 - (cost - 1));
    to_minion_phase(&mut e, SeatId(0));
    e.state.minions[v.index()].locked = true;
    e.state.minions[w.index()].locked = true;
    pick(&mut e, bleed(s));
    pick_when(&mut e, own_ability(s, 0));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

/// Sir Ralph Hamilton: "Ralph gets +1 strength in combat with a younger
/// Camarilla vampire. Followers of Set get +1 bleed when bleeding Ralph's
/// controller." The Ministry's +1 against his controller only.
#[test]
fn sir_ralph_hamilton_fights_young_camarilla_and_draws_the_ministry() {
    let mut e = game(3);
    let r = e.add_minion(SeatId(0), find_card("Sir Ralph Hamilton"), 4, false);
    let ralph = CRYPT[find_card("Sir Ralph Hamilton") as usize].capacity;
    let young = (0..CRYPT.len() as u16)
        .find(|&c| {
            let v = &CRYPT[c as usize];
            v.sect == Some(Sect::Camarilla)
                && v.capacity < ralph
                && !v.adv
                && !v.imbued
                && zvtes::cards::vampire(c).is_none()
        })
        .expect("a younger Camarilla vampire");
    let young = e.add_minion(SeatId(1), young, 4, false);
    let sabbat = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 0), 4, false);
    let set = e.add_minion(SeatId(1), plain("Ministry", 0), 4, false);
    let stat = zvtes::cards::Stat::Strength;
    assert_eq!(vs(&e, r, stat, young), 1);
    assert_eq!(vs(&e, r, stat, sabbat), 0);
    let bleeding = |s: u8| zvtes::state::PrintedCtx {
        kind: Some(ActionKind::Bleed),
        target: Some(SeatId(s)),
        ..Default::default()
    };
    let bleed_mod = |s: u8| {
        e.state
            .printed_mod(set, zvtes::cards::Stat::Bleed, bleeding(s))
    };
    assert_eq!(bleed_mod(0), 1);
    assert_eq!(bleed_mod(2), 0);
}

// ---- Vampires, group 11 -----------------------------------------------------

/// Suzanne Kadim: "+1 bleed."
#[test]
fn suzanne_kadim_bleeds_for_one_more() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Suzanne Kadim"), 4, false);
    assert_eq!(
        e.state
            .printed_mod(s, zvtes::cards::Stat::Bleed, Default::default()),
        1
    );
}

/// Tammy Walenski: "Tammy gets 1 optional maneuver each combat."
#[test]
fn tammy_walenski_maneuvers_once_a_combat() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Tammy Walenski"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick(&mut e, bleed(t));
    pick_when(&mut e, block(b));
    pick_when(&mut e, own_ability(t, 0));
}

/// Tobias Smith: "Tobias can give any other non-titled Sabbat vampire you
/// control with capacity 6 or more the title of bishop as an action."
#[test]
fn tobias_smith_makes_a_bishop() {
    let big = (0..CRYPT.len() as u16)
        .find(|&c| {
            let v = &CRYPT[c as usize];
            v.sect == Some(Sect::Sabbat)
                && v.title == Title::None
                && v.capacity >= 6
                && !v.adv
                && !v.imbued
                && zvtes::cards::vampire(c).is_none()
        })
        .expect("a big Sabbat vampire");
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Tobias Smith"), 4, false);
    let b = e.add_minion(SeatId(0), big, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let give = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(t),
        ability: 0,
        actor: Some(t),
        target: Target::Minion(b),
    };
    pick(&mut e, give);
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.title_of(b), Title::Bishop);
}

/// Tyler: "When Tyler diablerizes a vampire, she unlocks and gains a blood
/// from the blood bank." Unlocked after the diablerie, its 2 blood and 1
/// more.
#[test]
fn tyler_unlocks_and_feeds_on_diablerie() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Tyler"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: t,
            action: ActionKind::Diablerie,
            target: Some(v),
        },
    );
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].locked);
    assert_eq!(blood(&e, t), 4 + 2 + 1);
}

/// Tyler: "Once per turn, she may burn a blood to get +1 bleed or an
/// additional vote." Bled for 2; that turn, no vote bought in Mamadou's
/// referendum. Another turn, the vote bought.
// Rulings: [ANK 20231124] [LSJ 19970718].
#[test]
fn tyler_burns_once_a_turn_for_bleed_or_a_vote() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Tyler"), 4, false);
    let m = e.add_minion(SeatId(0), find_card("Mamadou Keita"), 7, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[v.index()].locked = true;
    e.state.minions[w.index()].locked = true;
    pick(&mut e, bleed(t));
    pick_when(&mut e, own_ability(t, 0));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
    assert_eq!(blood(&e, t), 3);
    pick(&mut e, own_action(m, 0));
    pick_when(&mut e, Choice::ChooseMinion(v));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        own_ability(t, 1),
    );

    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Tyler"), 4, false);
    let m = e.add_minion(SeatId(0), find_card("Mamadou Keita"), 7, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick(&mut e, own_action(m, 0));
    pick_when(&mut e, Choice::ChooseMinion(v));
    pick_when(&mut e, own_ability(t, 1));
    assert_eq!(blood(&e, t), 3);
}

/// Umdava: "You may use a master phase action to search your library or
/// ash heap for an Aye or Orun to place on a ready Laibon." Another
/// Methuselah's Laibon gets the Orun from the library.
#[test]
fn umdava_places_an_orun_on_any_ready_laibon() {
    let mut e = game(2);
    let u = e.add_minion(SeatId(0), find_card("Umdava"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let l = e.add_minion(SeatId(1), of_sect(Sect::Laibon, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let orun = find_library("Orun");
    fill_hand(&mut e, 0);
    for c in [
        find_library("Conditioning"),
        orun,
        find_library("Deflection"),
    ] {
        e.state.seats[0].library.push(c);
    }
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Vampire(u),
            ability: 0,
            actor: None,
            target: Target::Minion(l),
        },
    );
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: orun,
        },
    );
    request(&mut e);
    assert_eq!(on(&e, "Orun", l).len(), 1);
}

/// Undele: "Undele may call a referendum to move two minion cards from your
/// ash heap to your hand as a +1 stealth political action (discard
/// afterward)." With one there, not offered; with two, both to the hand.
// Rulings: [LSJ 20100917] [LSJ 20100129] [ANK 20191228].
#[test]
fn undele_brings_two_minion_cards_back() {
    let mut e = game(2);
    let u = e.add_minion(SeatId(0), find_card("Undele"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let (gregory, carlton) = (
        find_library("Gregory Winter"),
        find_library("Carlton Van Wyk"),
    );
    e.state.seats[0].ash.put(gregory, Default::default());
    e.state.seats[0]
        .ash
        .put(find_library("Orun"), Default::default());
    assert!(
        !request(&mut e).options.contains(&own_action(u, 0)),
        "one minion card"
    );
    e.state.seats[0].ash.put(carlton, Default::default());
    pick(&mut e, own_action(u, 0));
    pick_when(&mut e, Choice::FindCard(gregory));
    pick_when(&mut e, votes(u, true));
    close_polling(&mut e);
    let hand = &e.state.seats[0].hand;
    assert!(hand.contains(&gregory) && hand.contains(&carlton));
    assert_eq!(e.state.seats[0].ash.len(), 1);
}

/// Victorine Lafourcade: "When a vampire of capacity 4 or less is bleeding
/// you, you may burn the Edge to cause the action to fail. Victorine gets
/// an optional press each combat." The small bleeder fails; a big one,
/// not offered.
#[test]
fn victorine_lafourcade_burns_the_edge_against_small_bleeders() {
    let mut e = game(2);
    let small = e.add_minion(SeatId(0), plain_cap("Brujah", 1..=4), 4, false);
    let big = e.add_minion(SeatId(0), plain_cap("Brujah", 5..=11), 4, false);
    let v = e.add_minion(SeatId(1), find_card("Victorine Lafourcade"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.seats[1].has_edge = true;
    pick(&mut e, bleed(big));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        own_ability(v, 0),
    );
    // The big one's bleed took the Edge.
    e.state.seats[0].has_edge = false;
    e.state.seats[1].has_edge = true;
    let pool = e.state.seats[1].pool;
    pick(&mut e, bleed(small));
    pick_when(&mut e, own_ability(v, 0));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.seats[1].pool, pool, "the bleed failed");
    assert!(!e.state.seats[1].has_edge);
}

/// Virstania, The Great Mother: "If Virstania is ready, any ready slave
/// Gargoyle may remove his or her slave status and become an Independent
/// vampire as a +1 stealth action, and non-slave Gargoyles can bleed at +1
/// bleed as a Ⓓ action that costs a blood." Verbruch freed, then his bleed
/// for 2 at 1 blood.
// Rulings: [LSJ 20100909] [LSJ 20080604].
#[test]
fn virstania_frees_gargoyles_and_lets_them_bleed() {
    let mut e = game(2);
    let vi = e.add_minion(
        SeatId(0),
        find_card("Virstania, The Great Mother"),
        4,
        false,
    );
    let g = e.add_minion(SeatId(0), find_card("Verbruch"), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[v.index()].locked = true;
    e.state.minions[w.index()].locked = true;
    let grant = |ability: u8| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(vi),
        ability,
        actor: Some(g),
        target: Target::None,
    };
    let req = request(&mut e);
    assert!(!req.options.contains(&grant(1)), "a slave");
    pick(&mut e, grant(0));
    to_minion_phase(&mut e, SeatId(1));
    assert!(e.state.slave_clan(g).is_none());
    assert_eq!(e.state.sect_of(g), Some(Sect::Independent));
    to_minion_phase(&mut e, SeatId(0));
    e.state.minions[v.index()].locked = true;
    e.state.minions[w.index()].locked = true;
    pick_when(&mut e, grant(1));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.seats[1].pool, 10 - 2);
    assert_eq!(blood(&e, g), 3);
}

/// William Biltmore: "Once each action, William can burn 1 blood to get +1
/// stealth. +1 bleed." Against Kurt Densch's intercept.
#[test]
fn william_biltmore_buys_stealth() {
    let mut e = game(2);
    let w = e.add_minion(SeatId(0), find_card("William Biltmore"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let k = e.add_minion(SeatId(1), find_card("Kurt Densch"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    assert_eq!(
        e.state
            .printed_mod(w, zvtes::cards::Stat::Bleed, Default::default()),
        1
    );
    pick(&mut e, bleed(w));
    pick_when(&mut e, block(k));
    pick_when(&mut e, own_ability(w, 0));
    assert_eq!(blood(&e, w), 3);
}

// ---- The ledger's further rulings -------------------------------------------

/// Aye off a Laibon does nothing (ANK 20221208): three locked-for Aye on a
/// locked vampire turned Anarch, no "as if unlocked".
// Rulings: [ANK 20221208].
#[test]
fn aye_does_nothing_off_a_laibon() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let a = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let ayes: Vec<CardId> = (0..3).map(|_| in_play(&mut e, 0, "Aye", Some(l))).collect();
    to_minion_phase(&mut e, SeatId(1));
    e.state.minions[l.index()].locked = true;
    e.state.minions[l.index()].sect_override = Some(Sect::Anarch);
    pick(&mut e, bleed(a));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        ability(ayes[0].0, 1),
    );
}

/// Bestial Vengeance: its damage at long range (LSJ 20090610), the retainer
/// shot that round by the opposing gun all the same (LSJ 20090616).
// Rulings: [LSJ 20090610] [LSJ 20090616].
#[test]
fn bestial_vengeance_hurts_at_long_range_past_the_retainers_burning() {
    let mut e = game(2);
    let fish = e.add_minion(SeatId(0), find_card("Fish"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let raven = in_play(&mut e, 0, "Raven Spy", Some(fish));
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(b));
    hand(&mut e, 0, &["Bestial Vengeance", "Bestial Vengeance"]);
    pick_when(&mut e, bleed(fish));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play_at(0, 1, Some(fish), Target::Card(raven)));
    let gun_use = |o: &Choice, ab: u8| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), ability, .. } if *c == gun && *ability == ab);
    let at_raven = |o: &Choice| format!("{o:?}").contains(&format!("CardId({})", raven.0));
    let mut shot = false;
    for _ in 0..60 {
        let req = request(&mut e);
        if req.window == WindowLabel::MinionPhase {
            break;
        }
        let pick_i = if req.seat == SeatId(1) {
            req.options.iter().position(|o| gun_use(o, 1)).or_else(|| {
                req.options
                    .iter()
                    .position(|o| at_raven(o) && !matches!(o, Choice::Play { .. }))
                    .inspect(|_| shot = true)
            })
        } else {
            None
        };
        e.choose(pick_i.unwrap_or_else(|| fallback(&req)));
    }
    assert!(shot, "the gun aimed at the retainer");
    assert!(!e.state.cards[raven.index()].in_play, "the retainer shot");
    assert_eq!(blood(&e, b), 4 - 2, "Bestial Vengeance's 2, at long range");
}

/// Familial Bond: a locked Laibon cannot lock to fight (PIB 20121107).
// Rulings: [PIB 20121107].
#[test]
fn familial_bond_needs_an_unlocked_laibon_to_fight() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let a = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Familial Bond"]);
    to_minion_phase(&mut e, SeatId(1));
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Stealth(2),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(l));
    pick_when(&mut e, play_at(0, 0, Some(l), Target::None));
    let req = offering(&mut e, Choice::ResolvePrinted { minion: a });
    e.state.minions[l.index()].locked = true;
    e.choose(index_of(&req, Choice::Pass));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        Choice::ResolvePrinted { minion: l },
    );
}

/// Impundulu on his controller's own minion: undirected (ANK 20180917).
// Rulings: [ANK 20180917].
#[test]
fn impundulu_on_his_own_minion_is_undirected() {
    let mut e = game(2);
    let imp = e.add_ally(SeatId(0), find_library("Impundulu"));
    let own = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let steal = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(ally_card(&e, imp)),
        ability: 2,
        actor: Some(imp),
        target: Target::Minion(own),
    };
    pick_when(&mut e, steal);
    request(&mut e);
    let a = e.action().expect("the action");
    assert_eq!(a.directed_seat(&e.state), None);
}

/// Make an Example: by a locked Guruhi (PIB 20150720, RTR 19941109), not
/// one in torpor (RTR 19970306).
// Rulings: [PIB 20150720] [RTR 19941109]. [PIB 20150720] [RTR 19970306].
#[test]
fn make_an_example_from_a_locked_guruhi_not_a_torpid_one() {
    for torpor in [false, true] {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
        let g = e.add_minion(SeatId(0), plain("Guruhi", 0), 4, torpor);
        e.state.minions[g.index()].locked = !torpor;
        let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Make an Example"]);
        pick_when(&mut e, bleed(l));
        pick_when(&mut e, block(b));
        let example = play_at(0, 0, Some(g), Target::None);
        if torpor {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, example);
        } else {
            pick_when(&mut e, example);
            request(&mut e);
            assert_eq!(blood(&e, b), 2);
        }
    }
}

/// Orun's capacity, for another Methuselah's cards, read by Aranthebes in
/// play before it (LSJ 20051110), and over Wormwood's ceiling (LSJ
/// 20051205).
// Rulings: [LSJ 20051110] [LSJ 20051205].
#[test]
fn orun_capacity_reaches_cards_already_in_play() {
    let four = (0..CRYPT.len() as u16)
        .find(|&c| {
            let v = &CRYPT[c as usize];
            v.sect == Some(Sect::Laibon)
                && v.capacity == 4
                && v.title == Title::None
                && !v.adv
                && zvtes::cards::vampire(c).is_none()
        })
        .expect("a capacity 4 Laibon");
    let bled = |orun: bool| {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), four, 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        e.state.minions[v.index()].locked = true;
        let ar = in_play(&mut e, 1, "Aranthebes, The Immortal", None);
        e.state.layers.push(zvtes::cards::Layer {
            scope: zvtes::cards::Scope::InPlay,
            modifier: zvtes::cards::Modifier::YoungBleedAgainstController { max: 4, n: -1 },
            source: ar,
        });
        if orun {
            hand(&mut e, 0, &["Orun"]);
            until(&mut e, SeatId(0), WindowLabel::MasterPhase);
            master_on(&mut e, "Orun", l);
        }
        let pool = e.state.seats[1].pool;
        pick_when(&mut e, bleed(l));
        to_minion_phase(&mut e, SeatId(1));
        pool - e.state.seats[1].pool
    };
    assert_eq!(bled(false), 0, "Aranthebes' -1");
    assert_eq!(bled(true), 1, "capacity 6 for it");

    let mut e = game(2);
    let l = e.add_minion(SeatId(0), four, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Orun"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    master_on(&mut e, "Orun", l);
    let w = in_play(&mut e, 1, "Wormwood", None);
    e.state.cards[w.index()].counters = 3;
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::CapacityCapByCounters,
        source: w,
    });
    assert_eq!(e.state.capacity_of(l), Some(3));
    assert_eq!(e.state.capacity_by(l, SeatId(1)), Some(5));
}

/// Orun: a rush directed at a vampire burns one Orun after its combat
/// (ANK 20240809).
// Rulings: [ANK 20240809].
#[test]
fn orun_burns_after_a_rushs_combat() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), vampire_with("PRE", 0), 6, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[w.index()].locked = true;
    hand(&mut e, 0, &["Orun", "Bum's Rush"]);
    hand(&mut e, 1, &["Catatonic Fear"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    master_on(&mut e, "Orun", l);
    let rush = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Bum's Rush"))
        .expect("in hand") as u8;
    pick_when(&mut e, play_at(rush, 0, Some(l), Target::Minion(v)));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert_eq!(on(&e, "Orun", l).len(), 1, "still there in combat");
    e.choose(index_of(&req, Choice::HandStrike));
    to_minion_phase(&mut e, SeatId(1));
    assert!(on(&e, "Orun", l).is_empty(), "burned after combat");
}

/// Savannah Runner [cel]: not without the blood it burns (ANK 20210226).
// Rulings: [ANK 20210226].
#[test]
fn savannah_runner_needs_the_blood_it_burns() {
    for start in [0u8, 1] {
        let mut e = game(2);
        let r = e.add_minion(SeatId(0), of_sect_with(Sect::Laibon, "cel"), start, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let a = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Savannah Runner"]);
        to_minion_phase(&mut e, SeatId(1));
        e.state.minions[a.index()].timed.push((
            zvtes::cards::Lasting::Stealth(1),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(1),
        ));
        pick(&mut e, bleed(a));
        let runner = play_at(0, 0, Some(r), Target::None);
        if start == 0 {
            // Its only way to the intercept unpayable: no block.
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, block(r));
        } else {
            pick_when(&mut e, block(r));
            pick_when(&mut e, runner);
            request(&mut e);
            assert_eq!(blood(&e, r), 0);
        }
    }
}

/// Lucian, the Perfect: a strike committed by .44 Magnum's maneuver is made
/// (LSJ 20090528-2-2); one set by Sniper Rifle may still be declined (LSJ
/// 20071017); a strike at a card on him pays nothing (LSJ 20090528-2);
/// Catatonic Fear, damaging after combat, pays the toll (LSJ 20031118).
// Rulings: [LSJ 20090528-2-2] [LSJ 20071017] [LSJ 20090528-2] [LSJ 20031118].
// [LSJ 20070829-1] [ANK 20161025].
#[test]
fn lucian_taxes_every_strike_at_him_but_not_at_his_cards() {
    // The strike options of the blocker opposing Lucian.
    fn blocker_strikes(
        disc: &str,
        weapon: Option<&str>,
        cards: &[&str],
        gear: bool,
    ) -> Option<(Engine, MinionId, DecisionRequest)> {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), find_card("Lucian, the Perfect"), 6, false);
        // No damage from him: the blocker's blood tells its costs alone.
        let strength = e.state.base_strength(l) as i8;
        e.state.minions[l.index()].timed.push((
            zvtes::cards::Lasting::Strength(-strength),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(0),
        ));
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let b = e.add_minion(SeatId(1), vampire_with(disc, 0), 6, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        if gear {
            in_play(&mut e, 0, "Weighted Walking Stick", Some(l));
        }
        if let Some(w) = weapon {
            in_play(&mut e, 1, w, Some(b));
        }
        hand(&mut e, 1, cards);
        pick(&mut e, bleed(l));
        pick_when(&mut e, block(b));
        // The blocker takes its weapon's range and commitment where offered.
        for _ in 0..30 {
            let req = request(&mut e);
            if req.seat == SeatId(1) && req.window == WindowLabel::CombatStrike {
                return Some((e, b, req));
            }
            if req.window == WindowLabel::MinionPhase {
                let _ = l;
                return None;
            }
            let commit = req.options.iter().position(|o| {
                req.seat == SeatId(1)
                    && matches!(
                        o,
                        Choice::UseAbility {
                            source: zvtes::cards::AbilitySource::Card(_),
                            ability: 1,
                            ..
                        }
                    )
            });
            let i = commit.unwrap_or_else(|| fallback(&req));
            e.choose(i);
        }
        panic!("no strike choice");
    }
    // The committed strike its one option: no choice, no declining.
    assert!(blocker_strikes("PRE", Some(".44 Magnum"), &[], false).is_none());
    let (_, _, req) = blocker_strikes("PRE", Some("Sniper Rifle"), &[], false).expect("a choice");
    assert!(req.options.contains(&Choice::Pass), "Sniper Rifle's range");
    let (e, b, _) = blocker_strikes("PRE", None, &["Catatonic Fear"], false).expect("a choice");
    let cost = match zvtes::library::LIBRARY[find_library("Catatonic Fear") as usize].blood_cost {
        zvtes::library::Cost::Fixed(n) => n,
        _ => 0,
    };
    let mut e = e;
    let fear = plays(&request(&mut e))
        .into_iter()
        .find(|o| matches!(o, Choice::Play { mode: 1, .. }))
        .expect("Catatonic Fear [PRE]");
    let before = blood(&e, b);
    pick(&mut e, fear);
    request(&mut e);
    assert_eq!(blood(&e, b), before - cost - 1, "its cost and the toll");
    // A combat-ends strike and a dodge are strikes against him too (ANK
    // 20161025).
    for (disc, card) in [("PRE", "Majesty"), ("CEL", "Side Strike")] {
        let (mut e, b, req) = blocker_strikes(disc, None, &[card], false).expect("a choice");
        let cost = match zvtes::library::LIBRARY[find_library(card) as usize].blood_cost {
            zvtes::library::Cost::Fixed(n) => n,
            _ => 0,
        };
        let strike = plays(&req)
            .into_iter()
            .find(|o| matches!(o, Choice::Play { mode: 0, .. }))
            .expect(card);
        let before = blood(&e, b);
        pick(&mut e, strike);
        request(&mut e);
        assert_eq!(
            blood(&e, b),
            before - cost - 1,
            "{card}: its cost and the toll"
        );
    }
    let (mut e, b, req) = blocker_strikes("ANI", None, &["Canine Horde"], true).expect("a choice");
    let at_stick = plays(&req)
        .into_iter()
        .find(|o| {
            matches!(
                o,
                Choice::Play {
                    target: Target::Card(_),
                    ..
                }
            )
        })
        .expect("Canine Horde at the stick");
    let before = blood(&e, b);
    pick(&mut e, at_stick);
    request(&mut e);
    assert_eq!(blood(&e, b), before, "no toll on a strike at his card");
}

/// Supernatural Resistance on a strike card: canceled, the opposing minion
/// chooses a strike again (LSJ 20090818).
// Rulings: [LSJ 20090818].
#[test]
fn supernatural_resistance_cancels_a_strike_card_and_the_strike_is_chosen_again() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("PRE", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    for _ in 0..3 {
        in_play(&mut e, 0, "Orun", Some(l));
    }
    hand(&mut e, 0, &["Supernatural Resistance"]);
    hand(&mut e, 1, &["Catatonic Fear", "Catatonic Fear"]);
    pick(&mut e, bleed(l));
    pick_when(&mut e, block(b));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    let fear = plays(&req)
        .into_iter()
        .find(|o| matches!(o, Choice::Play { mode: 0, .. }))
        .expect("Catatonic Fear");
    e.choose(index_of(&req, fear));
    let req = request(&mut e);
    let resist = plays(&req)
        .into_iter()
        .find(|o| matches!(o, Choice::Play { actor: Some(a), .. } if *a == l))
        .expect("Supernatural Resistance offered");
    e.choose(index_of(&req, resist));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::CombatStrike),
        "a strike chosen again"
    );
}

/// Transcendent Laibon's burn option: only with no ready Laibon with three
/// Aye and three Orun (LSJ 20110611-2), its uniqueness aside (LSJ
/// 20100203-2).
// Rulings: [LSJ 20110611-2] [LSJ 20100203-2].
#[test]
fn transcendent_laibons_burn_option_reads_the_aye_and_orun() {
    for cards in [false, true] {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 0), 6, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if cards {
            for _ in 0..3 {
                in_play(&mut e, 0, "Aye", Some(l));
                in_play(&mut e, 0, "Orun", Some(l));
            }
        }
        hand(&mut e, 0, &["Transcendent Laibon"]);
        to_minion_phase(&mut e, SeatId(1));
        if cards {
            never_offered_until(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                Choice::Discard(0),
            );
        } else {
            let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
            assert!(
                req.options.contains(&Choice::Discard(0)),
                "{:?}",
                req.options
            );
        }
    }
}

/// Amelia: after combat in torpor too (ANK 20200420-1); each combat of the
/// action (Psyche!'s second one): two burns.
// Rulings: [ANK 20200420-1].
#[test]
fn amelia_drains_from_torpor_and_after_each_combat() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        find_card("Amelia, The Blood Red Tears"),
        1,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[b.index()].timed.push((
        zvtes::cards::Lasting::Strength(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].in_torpor);
    assert_eq!(blood(&e, b), 6 - 1 - 1, "her strike, then the burn");

    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        find_card("Amelia, The Blood Red Tears"),
        6,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let cel = vampire_with("CEL", 0);
    let start = CRYPT[cel as usize].capacity;
    let b = e.add_minion(SeatId(1), cel, start, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.notes = Some(Vec::new());
    hand(&mut e, 1, &["Psyche!"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play_at(0, 1, Some(b), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let begins = noted_combats(&mut e)
        .into_iter()
        .filter(|n| matches!(n, zvtes::combat::CombatNote::Begins { .. }))
        .count();
    assert_eq!(begins, 2, "two combats");
    let strength = e.state.base_strength(a) as u8;
    assert_eq!(blood(&e, b), start - 2 * strength - 2, "a burn after each");
}

/// Enid Blount: Edith's blood for her stealth, several times an action
/// (LSJ 20040617): twice against Kurt Densch's +1 intercept.
// Rulings: [LSJ 20040617].
#[test]
fn enid_blount_takes_ediths_stealth_twice() {
    let mut e = game(2);
    let en = e.add_minion(SeatId(0), find_card("Enid Blount"), 4, false);
    let ed = e.add_minion(SeatId(0), find_card("Edith Blount"), 4, false);
    let k = e.add_minion(SeatId(1), find_card("Kurt Densch"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let pool = e.state.seats[1].pool;
    pick_when(&mut e, bleed(en));
    pick_when(&mut e, block(k));
    pick_when(&mut e, own_ability(en, 0));
    pick_when(&mut e, own_ability(en, 0));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(blood(&e, ed), 2);
    assert!(e.state.seats[1].pool < pool, "unblocked");
}

/// Edward Neally: a hand size each master phase action spent on it (LSJ
/// 20020814): two with Nana Buruku's two.
// Rulings: [LSJ 20020814].
#[test]
fn edward_neally_widens_the_hand_twice() {
    let mut e = game(2);
    let ed = e.add_minion(SeatId(0), find_card("Edward Neally"), 4, false);
    e.add_minion(SeatId(0), find_card("Nana Buruku"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let size = e.state.hand_size(SeatId(0));
    pick_when(&mut e, own_ability(ed, 0));
    pick_when(&mut e, own_ability(ed, 0));
    request(&mut e);
    assert_eq!(e.state.hand_size(SeatId(0)), size + 2);
}

/// Ellison Humboldt: once each referendum (ANK 20180307-2), either way.
// Rulings: [ANK 20180307-2].
#[test]
fn ellison_humboldt_forces_a_prince_once_a_referendum() {
    let mut e = game(2);
    let el = e.add_minion(SeatId(0), find_card("Ellison Humboldt"), 4, false);
    let m = e.add_minion(SeatId(0), find_card("Mamadou Keita"), 7, false);
    let p = e.add_minion(SeatId(1), prince(), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick(&mut e, own_action(m, 0));
    pick_when(&mut e, Choice::ChooseMinion(v));
    let force = |ability: u8| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(el),
        ability,
        actor: None,
        target: Target::Minion(p),
    };
    pick_when(&mut e, force(0));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        *o == force(0) || *o == force(1)
    });
}

/// Horatio Ballard: not once he is no longer Camarilla (LSJ 20030202).
// Rulings: [LSJ 20030202].
#[test]
fn horatio_ballard_calls_only_while_camarilla() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Horatio Ballard"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[h.index()].sect_override = Some(Sect::Sabbat);
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        own_action(h, 0),
    );
}

/// Hrothulf: the Edge burned only if the action succeeds (LSJ 20030519):
/// blocked, he keeps it.
// Rulings: [LSJ 20030519].
#[test]
fn hrothulf_keeps_the_edge_when_blocked() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hrothulf"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.seats[0].has_edge = true;
    let fight = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(h),
        ability: 0,
        actor: Some(h),
        target: Target::Minion(v),
    };
    pick_when(&mut e, fight);
    pick_when(&mut e, block(w));
    to_minion_phase(&mut e, SeatId(1));
    assert!(e.state.seats[0].has_edge);
}

/// Jibade el-Bahrawi (advanced): several times a combat (LSJ 20040617):
/// one Aye, then the other, against two damage.
// Rulings: [LSJ 20040617].
#[test]
fn jibade_shields_twice_in_a_combat() {
    let mut e = game(2);
    let jb = e.add_minion(SeatId(0), find_card_adv("Jibade el-Bahrawi"), 5, false);
    let ally = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[b.index()].timed.push((
        zvtes::cards::Lasting::Strength(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    let ayes = [
        in_play(&mut e, 0, "Aye", Some(jb)),
        in_play(&mut e, 0, "Aye", Some(jb)),
    ];
    let life = blood(&e, ally);
    pick_when(&mut e, bleed(ally));
    pick_when(&mut e, block(b));
    let shield = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(jb),
        ability: 1,
        actor: None,
        target: Target::Minion(ally),
    };
    pick_when(&mut e, shield);
    pick_when(&mut e, set_x(1));
    pick_when(&mut e, shield);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ally), life, "both prevented");
    assert!(ayes.iter().all(|c| e.state.cards[c.index()].locked));
}

/// Julio Martinez: several times an action (LSJ 20040617): twice against a
/// +2 stealth Camarilla bleed.
// Rulings: [LSJ 20040617].
#[test]
fn julio_martinez_burns_twice_to_block() {
    let mut e = game(2);
    let ju = e.add_minion(SeatId(0), find_card("Julio Martinez"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let a = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    to_minion_phase(&mut e, SeatId(1));
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Stealth(2),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    let pool = e.state.seats[0].pool;
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(ju));
    pick_when(&mut e, own_ability(ju, 0));
    pick_when(&mut e, own_ability(ju, 0));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.seats[0].pool, pool, "blocked");
    assert!(blood(&e, ju) <= 2, "two blood burned");
}

/// Sébastien Goulet: his reduction on an ally Piper brings in (ANK
/// 20210913, LSJ 20090116); Erichtho's, keyed to the action, not (LSJ
/// 20090115-1).
// Rulings: [ANK 20210913] [LSJ 20090116]. [LSJ 20090115-1] [ANK 20170309].
#[test]
fn piper_takes_sebastiens_reduction_not_erichthos() {
    let cost = |c: u16| match zvtes::library::LIBRARY[c as usize].pool_cost {
        zvtes::library::Cost::Fixed(n) => n as i32,
        _ => 0,
    };
    for (who, card, off) in [
        ("Sébastien Goulet", find_library("Gregory Winter"), 1),
        ("Erichtho", find_library("Thadius Zho"), 0),
    ] {
        let mut e = game(2);
        e.state.seats[0].pool = 20;
        let v = e.add_minion(SeatId(0), find_card(who), 4, false);
        e.state.minions[v.index()].sect_override = Some(Sect::Anarch);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Piper"]);
        e.state.seats[0].hand.push(card);
        until(&mut e, SeatId(0), WindowLabel::MasterPhase);
        let before = e.state.seats[0].pool;
        pick_when(&mut e, play_at(0, 0, Some(v), Target::None));
        let pick_card = Choice::TakeFrom {
            zone: zvtes::decision::Zone::Hand,
            card,
        };
        let req = request(&mut e);
        if req.options.contains(&pick_card) {
            e.choose(index_of(&req, pick_card));
        }
        request(&mut e);
        assert_eq!(
            e.state.seats[0].pool,
            before - cost(find_library("Piper")) - (cost(card) - off),
            "{who}: Piper's pool, the ally's"
        );
    }
}

/// Tunnel Runner: the damage for its stolen life lands with the strikes,
/// one Soak preventing both (ANK 20200517-2): Theft of Vitae's 1 stolen,
/// its 1 hand damage and 1 for the theft prevented.
// Rulings: [ANK 20200517-2].
#[test]
fn tunnel_runners_theft_damage_is_soaked_with_the_strike() {
    let thief = (0..CRYPT.len() as u16)
        .find(|&c| {
            let v = &CRYPT[c as usize];
            let has = |d: &str| v.disciplines.iter().any(|x| x.eq_ignore_ascii_case(d));
            has("tha")
                && has("for")
                && v.title == Title::None
                && v.capacity >= 4
                && !v.adv
                && !v.imbued
                && zvtes::cards::vampire(c).is_none()
        })
        .expect("a vampire with Blood Sorcery and Fortitude");
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), thief, 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let tr = e.add_ally(SeatId(1), find_library("Tunnel Runner"));
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Theft of Vitae", "Soak"]);
    pick(&mut e, bleed(s));
    pick_when(&mut e, block(tr));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    let theft = plays(&req)
        .into_iter()
        .find(|o| {
            matches!(
                o,
                Choice::Play {
                    card: 0,
                    mode: 0,
                    ..
                }
            )
        })
        .expect("Theft of Vitae");
    e.choose(index_of(&req, theft));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatPrevention);
    let soak = plays(&req)
        .into_iter()
        .find(|o| matches!(o, Choice::Play { mode: 0, .. }))
        .expect("Soak");
    e.choose(index_of(&req, soak));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, s), 3 + 1, "the stolen life, no damage");
}

/// Brutal Influence: a vampire no longer Laibon may lock its Orun, and none
/// burns for the bleed (ANK 20221208): two Orun locked for X 2, the bleed
/// for 4, both still on him.
// Rulings: [ANK 20221208].
#[test]
fn brutal_influence_off_a_laibon_burns_no_orun() {
    let mut e = game(2);
    let fish = e.add_minion(SeatId(0), find_card("Fish"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[v.index()].locked = true;
    hand(&mut e, 0, &["Orun", "Orun", "Brutal Influence"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    master_on(&mut e, "Orun", fish);
    master_on(&mut e, "Orun", fish);
    e.state.minions[fish.index()].sect_override = Some(Sect::Anarch);
    let pool = e.state.seats[1].pool;
    let bi = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Brutal Influence"))
        .expect("in hand") as u8;
    pick_when(&mut e, play_at(bi, 0, Some(fish), Target::None));
    let req = request(&mut e);
    e.choose(index_of(&req, set_x(2)));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.seats[1].pool, pool - 4);
    assert_eq!(on(&e, "Orun", fish).len(), 2, "none burned");
}

/// Orun's +2 capacity, for "cards played by Methuselahs other than this
/// Laibon's controller", is read by another Methuselah's vampire's ability
/// requirement too (red, rules-qa 109.3): Apolonia Czarnecki (capacity 7)
/// steals from a capacity 6 Laibon, not once it holds an Orun (8 to her).
#[test]
fn orun_counts_for_a_vampires_ability_requirement() {
    let six = (0..CRYPT.len() as u16)
        .find(|&c| {
            let v = &CRYPT[c as usize];
            v.sect == Some(Sect::Laibon)
                && v.capacity == 6
                && v.title == Title::None
                && !v.adv
                && zvtes::cards::vampire(c).is_none()
        })
        .expect("a capacity 6 Laibon");
    assert_eq!(CRYPT[find_card("Apolonia Czarnecki") as usize].capacity, 7);
    for orun in [false, true] {
        let mut e = game(2);
        let ap = e.add_minion(SeatId(1), find_card("Apolonia Czarnecki"), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let l = e.add_minion(SeatId(0), six, 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        if orun {
            hand(&mut e, 0, &["Orun"]);
            until(&mut e, SeatId(0), WindowLabel::MasterPhase);
            master_on(&mut e, "Orun", l);
        }
        let steal = Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Vampire(ap),
            ability: 0,
            actor: Some(ap),
            target: Target::Minion(l),
        };
        let req = to_minion_phase(&mut e, SeatId(1));
        assert_eq!(req.options.contains(&steal), !orun, "orun: {orun}");
    }
}

/// A vampire is a played card: its ability reads an Orun'ed Laibon as the
/// Methuselah who played it does, whoever controls it now (red, rules-qa
/// 109.3; LSJ 20090205, 20090930). Apolonia, played by the Laibon's
/// controller and held by another, still sees the Laibon at 6 and steals.
// Rulings: [LSJ 20090205], [LSJ 20090930].
#[test]
fn a_stolen_vampires_ability_reads_orun_as_its_player() {
    let six = (0..CRYPT.len() as u16)
        .find(|&c| {
            let v = &CRYPT[c as usize];
            v.sect == Some(Sect::Laibon)
                && v.capacity == 6
                && v.title == Title::None
                && !v.adv
                && zvtes::cards::vampire(c).is_none()
        })
        .expect("a capacity 6 Laibon");
    let mut e = game(2);
    let ap = e.add_minion(SeatId(0), find_card("Apolonia Czarnecki"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let l = e.add_minion(SeatId(0), six, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Orun"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    master_on(&mut e, "Orun", l);
    let v = &mut e.state.minions[ap.index()];
    v.owner = SeatId(1);
    v.temp_from = Some(SeatId(0));
    let steal = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(ap),
        ability: 0,
        actor: Some(ap),
        target: Target::Minion(l),
    };
    let req = to_minion_phase(&mut e, SeatId(1));
    assert!(req.options.contains(&steal), "6 to her player");
    // Still 6 to her player as the action resolves: the blood moves.
    for m in e.state.minions.iter_mut().filter(|m| m.owner == SeatId(0)) {
        m.locked = true;
    }
    let (before, hers) = (blood(&e, l), blood(&e, ap));
    pick(&mut e, steal);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, l), blood(&e, ap)), (before - 1, hers + 1));
}

/// Abderrahim's "younger vampire you control": "you" his controller, the
/// age as his player reads it (rules-qa 109.3): stolen from the Laibon's
/// rival, he sees the thief's Orun'ed Laibon two older, younger no more.
#[test]
fn a_stolen_abderrahim_reads_orun_as_his_player() {
    let abd = find_card("Abderrahim, Death's Hand");
    let cap = CRYPT[abd as usize].capacity;
    let younger = (0..CRYPT.len() as u16)
        .find(|&c| {
            let v = &CRYPT[c as usize];
            v.capacity == cap - 1 && !v.adv && zvtes::cards::vampire(c).is_none()
        })
        .expect("a vampire one younger");
    for orun in [false, true] {
        let mut e = game(2);
        let b = e.add_minion(SeatId(0), vampire_with("AUS", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let a = e.add_minion(SeatId(0), abd, 4, false);
        let l = e.add_minion(SeatId(1), younger, 4, false);
        e.state.minions[l.index()].sect_override = Some(Sect::Laibon);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let v = &mut e.state.minions[a.index()];
        v.owner = SeatId(1);
        v.temp_from = Some(SeatId(0));
        if orun {
            in_play_with_layers(&mut e, 1, "Orun", Some(l));
        }
        let give = |o: &Choice| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(s), .. } if *s == a);
        pick_when(&mut e, bleed(l));
        let mut offered = false;
        for _ in 0..12 {
            let req = request(&mut e);
            if req.options.iter().any(give) {
                offered = true;
                break;
            }
            if req.window == WindowLabel::ActionBlock && req.seat == SeatId(0) {
                e.choose(index_of(&req, block(b)));
                continue;
            }
            if req.window == WindowLabel::MinionPhase {
                break;
            }
            e.choose(0);
        }
        assert_eq!(offered, !orun, "orun: {orun}");
    }
}

/// Transcendent Laibon reads a blocker's capacity as its controller does:
/// another Methuselah's capacity 4 Laibon with an Orun is 6 to it, no
/// younger than Fish, and blocks with no toll (LSJ 20051110).
// Rulings: [LSJ 20051110].
#[test]
fn transcendent_laibon_reads_an_orun_blockers_capacity() {
    let four = (0..CRYPT.len() as u16)
        .find(|&c| {
            let v = &CRYPT[c as usize];
            v.sect == Some(Sect::Laibon)
                && v.capacity == 4
                && v.title == Title::None
                && !v.adv
                && zvtes::cards::vampire(c).is_none()
        })
        .expect("a capacity 4 Laibon");
    for orun in [false, true] {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), find_card("Fish"), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let y = e.add_minion(SeatId(1), four, 3, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        for _ in 0..3 {
            in_play(&mut e, 0, "Aye", Some(l));
            in_play(&mut e, 0, "Orun", Some(l));
        }
        if orun {
            in_play_with_layers(&mut e, 1, "Orun", Some(y));
        }
        hand(&mut e, 0, &["Transcendent Laibon"]);
        until(&mut e, SeatId(0), WindowLabel::MasterPhase);
        master_on(&mut e, "Transcendent Laibon", l);
        pick_when(&mut e, bleed(l));
        // The toll paid as the block is declared, before any combat.
        pick_when(&mut e, block(y));
        assert_eq!(blood(&e, y), if orun { 3 } else { 2 }, "orun: {orun}");
    }
}

/// Kuta reads "older" as its player reads capacities (Orun's "cards played
/// by", rules-qa 109.50): seat 0's Kuta on seat 1's capacity 6 Laibon with
/// an Orun makes it 8 to seat 0, so it blocks seat 0's capacity 7 vampire;
/// without the Orun it may not.
// Rulings: [LSJ 20051110].
#[test]
fn kuta_reads_older_as_its_player() {
    let six = (0..CRYPT.len() as u16)
        .find(|&c| {
            let v = &CRYPT[c as usize];
            v.sect == Some(Sect::Laibon)
                && v.capacity == 6
                && v.title == Title::None
                && !v.adv
                && zvtes::cards::vampire(c).is_none()
        })
        .expect("a capacity 6 Laibon");
    for orun in [false, true] {
        let mut e = game(2);
        let w = e.add_minion(SeatId(0), plain_cap("Brujah", 7..=7), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let y = e.add_minion(SeatId(1), six, 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        in_play_with_layers(&mut e, 0, "Kuta", Some(y));
        if orun {
            in_play_with_layers(&mut e, 1, "Orun", Some(y));
        }
        pick(&mut e, bleed(w));
        let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
        assert_eq!(req.options.contains(&block(y)), orun, "orun: {orun}");
    }
}

/// Orun reaches a card already in the pool: seat 0's Blind Spot reads seat
/// 1's capacity 4 Laibon with an Orun as 6, no younger than its capacity 5
/// vampire, so not a choice; without the Orun it is (rules-qa 109.1).
// Rulings: [LSJ 20051110].
#[test]
fn blind_spot_reads_an_orun_laibons_capacity() {
    let four = (0..CRYPT.len() as u16)
        .find(|&c| {
            let v = &CRYPT[c as usize];
            v.sect == Some(Sect::Laibon)
                && v.capacity == 4
                && v.title == Title::None
                && !v.adv
                && zvtes::cards::vampire(c).is_none()
        })
        .expect("a capacity 4 Laibon");
    for orun in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain_cap("", 5..=5), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let y = e.add_minion(SeatId(1), four, 4, false);
        let small = e.add_minion(SeatId(1), plain_cap("Ventrue", 1..=2), 2, false);
        // An ally, always a choice: the choice stays a decision.
        e.add_ally(SeatId(1), find_library("Gregory Winter"));
        if orun {
            in_play_with_layers(&mut e, 1, "Orun", Some(y));
        }
        hand(&mut e, 0, &["Blind Spot"]);
        until(&mut e, SeatId(0), WindowLabel::MasterPhase);
        pick(&mut e, play_at(0, 0, None, Target::Minion(a)));
        let req = offering(&mut e, Choice::ChooseMinion(small));
        assert_eq!(
            req.options.contains(&Choice::ChooseMinion(y)),
            !orun,
            "orun: {orun}"
        );
    }
}
