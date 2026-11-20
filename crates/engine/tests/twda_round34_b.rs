//! Round 34 (`ADDED_BATCHES`): Black Hand and Camarilla Edition, its
//! master, modifier and reaction cards, the misc cards and the vampires
//! from Akram to Etrius.
//! Windows with a single option are elided, so the scripts only stop where
//! a real decision exists.

use zvtes::cards::{AbilitySource, Lasting, Stat, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel, Zone};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, CardId, Expiry};

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

/// A fixture vampire of capacity 4 to 8, no printed stealth.
#[track_caller]
fn mid(nth: usize) -> u16 {
    textless(|c| (4..=8).contains(&c.capacity) && no_stealth(c), nth)
}

/// No printed stealth (`fixture_rank` 2): a block of it is not futile.
fn no_stealth(c: &zvtes::crypt::CryptCard) -> bool {
    let i = CRYPT
        .iter()
        .position(|x| x.id == c.id)
        .expect("in the crypt");
    fixture_rank(i as u16) != Some(2)
}

/// The card `card` of the hand played in `mode` by `actor` on `target`.
fn play_at(card: u8, mode: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor,
        target,
    }
}

/// Ability `ability` of the card in play `card`, by `actor`, on `target`.
fn use_at(card: CardId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(card),
        ability,
        actor,
        target,
    }
}

fn ash_has(e: &Engine, seat: usize, name: &str) -> bool {
    e.state.seats[seat].ash.contains(&find_library(name))
}

/// The card named `name` in play, if any (not an effect-only stand-in).
fn card_in_play(e: &Engine, name: &str) -> Option<CardId> {
    let card = find_library(name);
    e.state
        .cards
        .iter()
        .position(|c| c.card == card && c.in_play && !c.effect_only)
        .map(|i| CardId(i as u32))
}

/// The cards named `name` in play on `m` (not effect-only stand-ins).
fn on(e: &Engine, name: &str, m: MinionId) -> Vec<CardId> {
    let card = find_library(name);
    e.state
        .cards
        .iter()
        .enumerate()
        .filter(|(_, c)| c.card == card && c.in_play && !c.effect_only && c.host == Some(m))
        .map(|(i, _)| CardId(i as u32))
        .collect()
}

fn locked(e: &Engine, m: MinionId) -> bool {
    e.state.minions[m.index()].locked
}

// ---- Masquerade Endangered, Rötschreck ---------------------------------------

/// Masquerade Endangered on `m`, card 0 of the hand.
fn endangered(m: MinionId) -> Choice {
    play_at(0, 0, None, Target::Minion(m))
}

/// Masquerade Endangered: "Put this card on a vampire who successfully
/// hunts. This vampire does not gain any blood from the hunt. This vampire
/// doesn't unlock as normal during his or her unlock phase. During this
/// vampire's next unlock phase, burn this card." Seat 0's vampire hunts (2
/// -> 3 blood); seat 1 plays it after resolution: back to 2, the card itself
/// on it; seat 2 plays a second copy after the same action [TOM 19950921],
/// taking nothing more. In seat 0's next unlock phase the vampire stays
/// locked and both copies burn to their players' ash heaps.
#[test]
fn masquerade_endangered_takes_the_hunts_blood_after_it() {
    let mut e = game(3);
    let v = e.add_minion(SeatId(0), mid(0), 2, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    e.add_minion(SeatId(2), mid(3), 4, false);
    hand(&mut e, 1, &["Masquerade Endangered"]);
    hand(&mut e, 2, &["Masquerade Endangered"]);
    pick(&mut e, action(v, ActionKind::Hunt));
    let req = offering(&mut e, endangered(v));
    assert_eq!(req.seat, SeatId(1));
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    e.choose(index_of(&req, endangered(v)));
    let req = offering(&mut e, endangered(v));
    assert_eq!(req.seat, SeatId(2));
    assert_eq!(blood(&e, v), 2, "the hunt's blood back to the bank");
    assert_eq!(on(&e, "Masquerade Endangered", v).len(), 1);
    assert!(!ash_has(&e, 1, "Masquerade Endangered"));
    e.choose(index_of(&req, endangered(v)));
    request(&mut e);
    assert_eq!(blood(&e, v), 2, "the second copy takes nothing more");
    assert_eq!(on(&e, "Masquerade Endangered", v).len(), 2);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(locked(&e, v), "not unlocked as normal");
    assert!(on(&e, "Masquerade Endangered", v).is_empty());
    assert!(ash_has(&e, 1, "Masquerade Endangered"));
    assert!(ash_has(&e, 2, "Masquerade Endangered"));
}

/// "A hunt is not successful if no blood is gained" [LSJ 20050720-2]: a
/// diseased hunter's hunt gains nothing, Masquerade Endangered is never
/// offered; nor to the hunting vampire's own Methuselah (out-of-turn).
#[test]
fn masquerade_endangered_needs_a_hunt_that_gained_blood() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), mid(0), 2, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    e.state.minions[v.index()].disease = true;
    hand(&mut e, 1, &["Masquerade Endangered"]);
    pick(&mut e, action(v, ActionKind::Hunt));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, endangered(v));

    let mut e = game(2);
    let v = e.add_minion(SeatId(0), mid(0), 2, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    hand(&mut e, 0, &["Masquerade Endangered"]);
    pick(&mut e, action(v, ActionKind::Hunt));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, endangered(v));
    assert_eq!(blood(&e, v), 3);
}

/// Rötschreck played by seat 1 on seat 0's acting vampire `a`, facing seat
/// 1's aggravated strike (LSJ 20080821, rules-qa 94.1): "during this
/// vampire's next unlock phase, burn this card": still on `a` after seat
/// 1's unlock phase, burned in seat 0's (a fix: it burned in its player's).
#[test]
fn rotschreck_on_another_methuselahs_vampire_burns_in_its_controllers_unlock_phase() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let z = e.add_minion(SeatId(1), mid(2), 4, false);
    e.add_minion(SeatId(1), mid(3), 4, false);
    let bow = in_play(&mut e, 1, "Ivory Bow", Some(z));
    hand(&mut e, 1, &["Rötschreck"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(z));
    let rot = play_at(0, 0, None, Target::Minion(a));
    pick_when(&mut e, use_at(bow, 0, None, Target::None));
    pick_when(&mut e, rot);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(
        on(&e, "Rötschreck", a).len(),
        1,
        "not in its player's unlock phase"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(on(&e, "Rötschreck", a).is_empty());
    assert!(ash_has(&e, 1, "Rötschreck"));
}

// ---- Mistrust, Path of Death and the Soul -------------------------------------

/// A scarce vampire (every one has text: named).
fn scarce(nth: usize) -> u16 {
    find_card(["Le Dinh Tho", "Asguresh"][nth])
}

/// Mistrust: "Burn a scarce vampire in torpor or move a ready scarce vampire
/// to his or her controller's uncontrolled region, with all cards and
/// counters on him or her." Seat 1's torpid scarce vampire burned; its
/// locked ready scarce one moved to seat 1's uncontrolled region, its blood
/// kept and its equipment out of play; a non-scarce one never a target.
#[test]
fn mistrust_burns_a_torpid_scarce_vampire_or_banishes_a_ready_one() {
    let mut e = game(2);
    e.add_minion(SeatId(0), mid(0), 4, false);
    let t = e.add_minion(SeatId(1), scarce(0), 1, true);
    let r = e.add_minion(SeatId(1), scarce(1), 3, false);
    let plain_one = e.add_minion(SeatId(1), mid(1), 4, false);
    e.state.minions[r.index()].locked = true;
    in_play(&mut e, 1, ".44 Magnum", Some(r));
    hand(&mut e, 0, &["Mistrust", "Mistrust"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&play_at(0, 0, None, Target::Minion(plain_one))));
    assert!(!req
        .options
        .contains(&play_at(0, 1, None, Target::Minion(plain_one))));
    assert!(!req
        .options
        .contains(&play_at(0, 0, None, Target::Minion(r))));
    pick(&mut e, play_at(0, 0, None, Target::Minion(t)));
    request(&mut e);
    assert!(!e.state.minions[t.index()].in_play);
    assert!(e.state.seats[1]
        .ash_crypt
        .contains(&e.state.minions[t.index()].crypt_card().unwrap()));

    let mut e = game(2);
    e.add_minion(SeatId(0), mid(0), 4, false);
    let r = e.add_minion(SeatId(1), scarce(1), 3, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    e.state.minions[r.index()].locked = true;
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(r));
    hand(&mut e, 0, &["Mistrust"]);
    e.at_master_phase();
    pick(&mut e, play_at(0, 1, None, Target::Minion(r)));
    request(&mut e);
    assert!(!e.state.minions[r.index()].in_play, "out of play");
    let slot = e.state.seats[1].uncontrolled.last().expect("a slot");
    assert_eq!(slot.counters, 3, "its blood kept as counters");
    assert!(slot
        .banished
        .as_ref()
        .is_some_and(|b| b.cards.contains(&gun)));
    assert!(
        !e.state.cards[gun.index()].in_play,
        "its equipment out of play"
    );
    assert!(!ash_has(&e, 1, ".44 Magnum"));
}

/// Path of Death and the Soul: "When a minion controlled by another
/// Methuselah is burned, you may lock this card to search your library …,
/// ash heap or hand for a Master: Discipline card. Move 1 blood from the
/// blood bank and that card to a ready vampire you control." Seat 0's
/// Mistrust burns seat 1's torpid vampire: seat 0 locks the Path for its
/// vampire, takes Celerity from its ash heap onto it, +1 blood.
#[test]
fn path_of_death_and_the_soul_feeds_a_discipline_on_a_burn() {
    let mut e = game(2);
    let v = e.add_minion(
        SeatId(0),
        textless(|c| c.capacity >= 4 && !c.disciplines.contains(&"CEL"), 0),
        2,
        false,
    );
    e.add_minion(SeatId(0), mid(1), 4, false);
    let t = e.add_minion(SeatId(1), scarce(0), 1, true);
    e.add_minion(SeatId(1), mid(2), 4, false);
    let path = in_play(&mut e, 0, "Path of Death and the Soul", None);
    e.state.seats[0].ash.push(find_library("Celerity"));
    hand(&mut e, 0, &["Mistrust"]);
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::Minion(t)));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(req.options.contains(&Choice::Pass));
    pick(&mut e, Choice::ChooseMinion(v));
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    let celerity = Choice::TakeFrom {
        zone: Zone::AshHeap,
        card: find_library("Celerity"),
    };
    assert!(req.options.contains(&celerity));
    pick(&mut e, celerity);
    request(&mut e);
    assert_eq!(on(&e, "Celerity", v).len(), 1);
    assert_eq!(blood(&e, v), 3);
    assert!(e.state.cards[path.index()].locked);
}

/// Path of Death and the Soul: "You cannot choose a Discipline card that all
/// your vampires already have at superior." [LSJ 20031209] Its only
/// vampire has superior Celerity: Celerity not offered, Fortitude is. Its
/// own minion burned: no offer.
#[test]
fn path_of_death_and_the_soul_offers_no_card_its_vampire_has_superior() {
    let mut e = game(2);
    let v = e.add_minion(
        SeatId(0),
        textless(
            |c| {
                c.disciplines.contains(&"CEL")
                    && !c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("for"))
            },
            0,
        ),
        2,
        false,
    );
    let t = e.add_minion(SeatId(1), scarce(0), 1, true);
    e.add_minion(SeatId(1), mid(2), 4, false);
    in_play(&mut e, 0, "Path of Death and the Soul", None);
    for name in ["Celerity", "Fortitude"] {
        e.state.seats[0].ash.push(find_library(name));
    }
    hand(&mut e, 0, &["Mistrust"]);
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::Minion(t)));
    pick_when(&mut e, Choice::ChooseMinion(v));
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    let take = |name| Choice::TakeFrom {
        zone: Zone::AshHeap,
        card: find_library(name),
    };
    assert!(!req.options.contains(&take("Celerity")));
    assert!(req.options.contains(&take("Fortitude")));

    // Its controller's own minion burned: nothing offered.
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), mid(0), 2, false);
    let own = e.add_minion(SeatId(0), scarce(0), 1, true);
    e.add_minion(SeatId(1), mid(2), 4, false);
    in_play(&mut e, 0, "Path of Death and the Soul", None);
    hand(&mut e, 0, &["Mistrust"]);
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::Minion(own)));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::ChooseMinion(v),
    );
}

/// "If you fail to find a Discipline card, you still get the blood." [LSJ
/// 20031209] Nothing to find: the search passes, the blood moves, the
/// library shuffled.
#[test]
fn path_of_death_and_the_soul_gives_blood_when_the_search_fails() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), mid(0), 2, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let t = e.add_minion(SeatId(1), scarce(0), 1, true);
    e.add_minion(SeatId(1), mid(2), 4, false);
    in_play(&mut e, 0, "Path of Death and the Soul", None);
    e.state.seats[0].library.push(find_library("Celerity"));
    hand(&mut e, 0, &["Mistrust"]);
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::Minion(t)));
    pick_when(&mut e, Choice::ChooseMinion(v));
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    e.choose(index_of(&req, Choice::Pass));
    request(&mut e);
    assert_eq!(blood(&e, v), 3);
    assert!(on(&e, "Celerity", v).is_empty());
}

// ---- Path of Evil Revelations -------------------------------------------------

/// A fixture vampire made infernal.
fn infernal(e: &mut Engine, nth: usize) -> MinionId {
    let m = e.add_minion(SeatId(0), mid(nth + 4), 3, false);
    e.state.minions[m.index()].made_infernal = true;
    m
}

/// Path of Evil Revelations: "During your master phase, you can burn 1 pool
/// to unlock all infernal minions you control." Two locked infernal
/// vampires unlocked for 1 pool, a non-infernal one stays locked; not
/// offered again that phase, nor with no locked infernal minion (rules-qa
/// 48.7). Not the infernal unlock: Ruins of Charizel collects nothing [LSJ
/// 20050228-2].
#[test]
fn path_of_evil_revelations_unlocks_infernals_for_a_pool() {
    let mut e = game(2);
    let i1 = infernal(&mut e, 0);
    let i2 = infernal(&mut e, 1);
    e.add_minion(SeatId(1), mid(1), 4, false);
    let path = in_play(&mut e, 0, "Path of Evil Revelations", None);
    for m in [i1, i2] {
        e.state.minions[m.index()].locked = true;
    }
    // Their infernal unlock declined on the way.
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(locked(&e, i1) && locked(&e, i2));
    let before = pool(&e)[0];
    e.choose(index_of(&req, use_at(path, 0, None, Target::None)));
    let req = request(&mut e);
    assert!(!locked(&e, i1) && !locked(&e, i2));
    assert_eq!(pool(&e)[0], before - 1);
    assert!(!req.options.contains(&use_at(path, 0, None, Target::None)));
}

/// Path of Evil Revelations and The Textbook Damnation (rules-qa 125.207;
/// LSJ 20100421: "Methuselah chooses Path. Untap all infernals. Now
/// Textbook's effect is no longer available"): the Textbook's vampire
/// (infernal by it) locked as the master phase begins, the Path's unlock
/// used first: the phase passes with the Textbook kept.
#[test]
fn path_of_evil_revelations_saves_the_textbook_damnation() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(4), 4, false);
    e.add_minion(SeatId(0), mid(5), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    let book = in_play(&mut e, 0, "Textbook Damnation, The", Some(a));
    let path = in_play(&mut e, 0, "Path of Evil Revelations", None);
    hand(&mut e, 0, &["Dreams of the Sphinx"]);
    e.state.minions[a.index()].locked = true;
    e.at_master_phase();
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::Pass), "the burn due");
    e.choose(index_of(&req, use_at(path, 0, None, Target::None)));
    let req = request(&mut e);
    assert!(!locked(&e, a));
    e.choose(index_of(&req, Choice::Pass));
    to_minion_phase(&mut e, SeatId(0));
    assert!(e.state.cards[book.index()].in_play, "kept");
}

/// [LSJ 20050228-2] "Only works when unlocking minions using their Infernal
/// ability (eg. not with Path of Evil Revelations …)": seat 1's Ruins of
/// Charizel gets no counter of seat 0's pool.
#[test]
fn path_of_evil_revelations_pool_is_not_collected_by_ruins_of_charizel() {
    let mut e = game(2);
    let i1 = infernal(&mut e, 0);
    e.add_minion(SeatId(1), mid(1), 4, false);
    let path = in_play(&mut e, 0, "Path of Evil Revelations", None);
    let ruins = in_play(&mut e, 1, "Ruins of Charizel", None);
    e.state.minions[i1.index()].locked = true;
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let before = pool(&e);
    e.choose(index_of(&req, use_at(path, 0, None, Target::None)));
    request(&mut e);
    assert!(!locked(&e, i1));
    assert_eq!(e.state.cards[ruins.index()].counters, 0);
    assert_eq!(pool(&e), vec![before[0] - 1, before[1]]);
}

/// "Minions can burn this card as a Ⓓ action that inflicts 1 unpreventable
/// environmental damage on acting vampires." Seat 1's vampire burns seat
/// 0's Path: burned, 1 blood off the actor.
#[test]
fn path_of_evil_revelations_burns_with_damage_to_its_burner() {
    let mut e = game(2);
    e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let x = e.add_minion(SeatId(1), mid(2), 4, false);
    e.add_minion(SeatId(1), mid(3), 4, false);
    let path = in_play(&mut e, 0, "Path of Evil Revelations", None);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, use_at(path, 1, Some(x), Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(card_in_play(&e, "Path of Evil Revelations").is_none());
    assert!(ash_has(&e, 0, "Path of Evil Revelations"));
    assert_eq!(blood(&e, x), 3);
}

// ---- Powerbase: Berlin, Powerbase: Washington, D.C. ----------------------------

/// Powerbase: Berlin: "Any Ventrue you control may move 1 blood to this card
/// as a +1 stealth action." The blood from the Ventrue [LSJ 19970414]; an
/// action requiring Ventrue: a Brujah not offered it [LSJ 20080604].
#[test]
fn powerbase_berlin_takes_blood_from_the_acting_ventrue() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), mid(0), 4, false);
    let berlin = in_play(&mut e, 0, "Powerbase: Berlin", None);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&use_at(berlin, 1, Some(b), Target::None)));
    pick(&mut e, use_at(berlin, 1, Some(v), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, v), 3);
    assert_eq!(e.state.cards[berlin.index()].counters, 1);
}

/// "The two actions provided are not cross-restrictive. Each can be
/// performed once during the same turn by the same minion." [LSJ 20100909]
/// Seat 0's Ventrue moves blood to seat 1's... its own Berlin; seat 1's
/// Berlin is stolen by seat 0's same vampire the same turn: the referendum
/// passes, the card (its blood with it) seat 0's.
#[test]
fn powerbase_berlin_two_actions_are_not_cross_restrictive() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.state.minions[v.index()].granted_title = Some(Title::Prince("Paris"));
    let w = e.add_minion(SeatId(0), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(0), 4, false);
    let berlin = in_play(&mut e, 1, "Powerbase: Berlin", None);
    e.state.cards[berlin.index()].counters = 2;
    pick(&mut e, use_at(berlin, 3, Some(v), Target::None));
    to_referendum(&mut e);
    pick_when(&mut e, votes(v, true));
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.card_controller(berlin), SeatId(0), "stolen");
    // The same vampire, unlocked, takes the other action the same turn.
    e.state.minions[v.index()].locked = false;
    pick(&mut e, action(w, ActionKind::Hunt));
    pick_when(&mut e, use_at(berlin, 1, Some(v), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[berlin.index()].counters, 3);
    assert_eq!(blood(&e, v), 3);
}

/// Powerbase: Berlin's steal not offered to a Methuselah who controls
/// another Berlin: no voluntary self-contest (rules-qa 121.119, "as every
/// 'take this card' ability"; 90.4). Seat 0 with its own Berlin: seat 1's
/// not offered; without it, offered.
#[test]
fn powerbase_berlin_is_not_stolen_into_a_self_contest() {
    for own in [true, false] {
        let mut e = game(2);
        let v = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(0), mid(1), 4, false);
        e.add_minion(SeatId(1), mid(0), 4, false);
        if own {
            in_play(&mut e, 0, "Powerbase: Berlin", None);
        }
        let berlin = in_play(&mut e, 1, "Powerbase: Berlin", None);
        let req = request(&mut e);
        assert_eq!(
            req.options
                .contains(&use_at(berlin, 3, Some(v), Target::None)),
            !own,
            "own: {own}"
        );
    }
}

/// "You may lock this card and burn X blood from it to give a Ventrue
/// attempting to block a political action +X intercept." Seat 1's Ventrue
/// at no intercept against a +1 stealth political action: reachable (the
/// opportunity counted: `block_can_reach`); X = 1 burned, the card locked,
/// the block made.
#[test]
fn powerbase_berlin_burns_x_blood_for_ventrue_intercept() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    let berlin = in_play(&mut e, 1, "Powerbase: Berlin", None);
    e.state.cards[berlin.index()].counters = 3;
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, caller));
    let req = offering(&mut e, block(y));
    assert!(e.block_can_reach(req.seat, y));
    e.choose(index_of(&req, block(y)));
    let req = offering(&mut e, use_at(berlin, 2, None, Target::None));
    e.choose(index_of(&req, use_at(berlin, 2, None, Target::None)));
    e.notes = Some(Vec::new());
    pick_when(&mut e, set_x(1));
    request(&mut e);
    assert!(e.state.cards[berlin.index()].locked);
    assert_eq!(e.state.cards[berlin.index()].counters, 2);
    assert!(!noted_combats(&mut e).is_empty(), "blocked");
}

/// Powerbase: Washington, D.C.: "During your unlock phase, you may move up
/// to 3 pool to this card and add 1 blood from the blood bank for each pool
/// you move, or you may move 2 blood from this card to your pool." X = 3:
/// 3 pool, 6 counters, the other not offered that phase; next unlock
/// phase: 2 to the pool.
#[test]
fn powerbase_washington_dc_banks_pool_doubled_or_pays_out_two() {
    let mut e = game(2);
    e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    let dc = in_play(&mut e, 0, "Powerbase: Washington, D.C.", None);
    e.state.cards[dc.index()].counters = 1;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&use_at(dc, 1, None, Target::None)));
    e.choose(index_of(&req, use_at(dc, 0, None, Target::None)));
    let req = request(&mut e);
    assert!(req.options.contains(&set_x(0)), "up to 3: 0 too");
    assert!(!req.options.contains(&set_x(4)));
    e.choose(index_of(&req, set_x(3)));
    let req = request(&mut e);
    assert!(!req.options.contains(&use_at(dc, 1, None, Target::None)));
    assert_eq!(e.state.cards[dc.index()].counters, 7);
    assert_eq!(pool(&e)[0], 7);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick_when(&mut e, use_at(dc, 1, None, Target::None));
    request(&mut e);
    assert_eq!(e.state.cards[dc.index()].counters, 5);
    assert_eq!(pool(&e)[0], 9);
}

/// "A vampire controlled by another Methuselah may move all the blood on
/// this card to his or her controller's pool as a Ⓓ action."
#[test]
fn powerbase_washington_dc_is_emptied_by_another_methuselahs_action() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    let dc = in_play(&mut e, 1, "Powerbase: Washington, D.C.", None);
    e.state.cards[dc.index()].counters = 5;
    pick(&mut e, use_at(dc, 2, Some(v), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![15, 10]);
    assert_eq!(e.state.cards[dc.index()].counters, 0);
}

// ---- Investments, Remover, Sociopath ----------------------------------------

/// Short-Term Investment: 3 blood, 1 to the pool per master phase action,
/// burned at the last; Protracted Investment enters with 5.
#[test]
fn short_and_protracted_investments_pay_out_by_master_phase_actions() {
    let mut e = game(2);
    e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    hand(
        &mut e,
        0,
        &["Short-Term Investment", "Protracted Investment"],
    );
    e.at_master_phase();
    e.state.seats[0].mpa = 2;
    pick(&mut e, master_play(0));
    request(&mut e);
    let st = card_in_play(&e, "Short-Term Investment").expect("in play");
    assert_eq!(e.state.cards[st.index()].counters, 3);
    pick(&mut e, master_play(0));
    request(&mut e);
    let pi = card_in_play(&e, "Protracted Investment").expect("in play");
    assert_eq!(e.state.cards[pi.index()].counters, 5);
    e.state.cards[st.index()].counters = 1;
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let before = pool(&e)[0];
    e.choose(index_of(&req, use_at(st, 0, None, Target::None)));
    request(&mut e);
    assert_eq!(pool(&e)[0], before + 1);
    assert!(
        card_in_play(&e, "Short-Term Investment").is_none(),
        "the last counter"
    );
    assert!(ash_has(&e, 0, "Short-Term Investment"));
}

/// Remover: "Put this card on a Black Hand vampire. This vampire can lock
/// any card (including a minion) as a +1 stealth Ⓓ action. A vampire can
/// have only one Remover." The card itself on its vampire; not on a
/// non-Black Hand vampire, nor a second on one; its bearer locks seat 1's
/// minion, then seat 1's location.
#[test]
fn remover_lets_its_black_hand_vampire_lock_a_minion_or_a_card() {
    let mut e = game(2);
    let bh = e.add_minion(
        SeatId(0),
        textless(|c| c.black_hand && c.capacity >= 4, 0),
        4,
        false,
    );
    let other = e.add_minion(
        SeatId(0),
        textless(|c| !c.black_hand && c.capacity >= 4, 0),
        4,
        false,
    );
    let x = e.add_minion(SeatId(1), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    let loc = in_play(&mut e, 1, "Powerbase: Berlin", None);
    hand(&mut e, 0, &["Remover", "Remover"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&play_at(0, 0, None, Target::Minion(other))));
    pick(&mut e, play_at(0, 0, None, Target::Minion(bh)));
    let req = request(&mut e);
    assert!(
        !req.options
            .contains(&play_at(0, 0, None, Target::Minion(bh))),
        "only one"
    );
    let remover = on(&e, "Remover", bh)[0];
    assert!(!ash_has(&e, 0, "Remover"));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let lock_x = use_at(remover, 0, Some(bh), Target::Minion(x));
    assert!(req.options.contains(&lock_x));
    e.choose(index_of(&req, lock_x));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(locked(&e, x));
    e.state.minions[bh.index()].locked = false;
    let _ = req;
    pick(&mut e, use_at(remover, 1, Some(bh), Target::Card(loc)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[loc.index()].locked);
}

/// Sociopath: "Once per turn, when this vampire remains ready and burns a
/// minion in combat or sends a vampire to torpor in combat, he or she gains
/// 1 blood." Its Sabbat bearer sends the blocker to torpor: +1; a second
/// combat that turn: nothing more.
#[test]
fn sociopath_feeds_its_vampire_once_a_turn_on_a_combat_kill() {
    let sabbat = |n| {
        textless(
            |c| c.sect == Some(Sect::Sabbat) && c.capacity >= 4 && no_stealth(c),
            n,
        )
    };
    let fight_out = |e: &mut Engine| {
        let mut req = request(e);
        while e.combat().is_some() {
            e.choose(fallback(&req));
            req = request(e);
        }
    };
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), sabbat(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let x = e.add_minion(SeatId(1), mid(2), 0, false);
    let y = e.add_minion(SeatId(1), mid(3), 1, false);
    let z = e.add_minion(SeatId(1), mid(4), 1, false);
    in_play(&mut e, 0, "Sociopath", Some(s));
    // Strength 2: a 1-blood bleeder to torpor.
    e.state.minions[s.index()]
        .timed
        .push((Lasting::Strength(1), Expiry::Never, SeatId(0)));
    pick(&mut e, bleed(s));
    pick_when(&mut e, block(x));
    fight_out(&mut e);
    assert!(e.state.minions[x.index()].in_torpor);
    assert_eq!(blood(&e, s), 4, "4 - 1 + 1");
    // Seat 1's turn: a first kill feeds it, a second does not.
    e.state.minions[s.index()].locked = false;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(y));
    pick_when(&mut e, block(s));
    fight_out(&mut e);
    assert!(e.state.minions[y.index()].in_torpor);
    assert_eq!(blood(&e, s), 4, "a new turn");
    e.state.minions[s.index()].locked = false;
    pick_when(&mut e, bleed(z));
    pick_when(&mut e, block(s));
    fight_out(&mut e);
    assert!(e.state.minions[z.index()].in_torpor);
    assert_eq!(blood(&e, s), 3, "once per turn");

    // Both to torpor: it did not remain ready.
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), sabbat(0), 1, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let x = e.add_minion(SeatId(1), mid(2), 1, false);
    e.add_minion(SeatId(1), mid(3), 4, false);
    in_play(&mut e, 0, "Sociopath", Some(s));
    for m in [s, x] {
        e.state.minions[m.index()]
            .timed
            .push((Lasting::Strength(1), Expiry::Never, SeatId(0)));
    }
    pick(&mut e, bleed(s));
    pick_when(&mut e, block(x));
    fight_out(&mut e);
    assert!(e.state.minions[x.index()].in_torpor && e.state.minions[s.index()].in_torpor);
    assert_eq!(blood(&e, s), 0);
}

// ---- The Spawning Pool, Steam Tunnels, Unnatural Disaster ------------------------

/// The Spawning Pool: "During your unlock phase, you may move 1 blood from a
/// ready Nosferatu you control to this card. If a minion you control blocks
/// a bleed against you, you may lock this card during the second round of
/// the resulting combat to inflict 1 damage to the acting minion for each
/// blood on the Spawning Pool. This damage cannot be prevented." Not in
/// round 1; in round 2, 2 damage on the acting vampire, the card locked.
#[test]
fn spawning_pool_stores_nosferatu_blood_and_hits_in_round_two() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(2), 4, false);
    e.add_minion(SeatId(0), mid(3), 4, false);
    let n = e.add_minion(SeatId(1), plain("Nosferatu", 0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    e.state.minions[n.index()].gained_levels = vec!["cel"];
    let spawning = in_play(&mut e, 1, "Spawning Pool, The", None);
    e.state.cards[spawning.index()].counters = 1;
    hand(&mut e, 1, &["Flash"]);
    let hit = use_at(spawning, 1, None, Target::None);
    let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    e.choose(index_of(&req, use_at(spawning, 0, None, Target::Minion(n))));
    request(&mut e);
    assert_eq!(e.state.cards[spawning.index()].counters, 2);
    assert_eq!(blood(&e, n), 3);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(n));
    // Round 1: not offered; Flash's press takes the combat on.
    loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&hit), "round 1");
        if let Some(i) = req.options.iter().position(|o| *o == play(0, 0, n)) {
            if req.window == WindowLabel::CombatPress {
                e.choose(i);
                break;
            }
        }
        e.choose(fallback(&req));
    }
    let req = offering(&mut e, hit);
    assert_eq!(e.combat().expect("combat").round, 2);
    let before = blood(&e, a);
    e.choose(index_of(&req, hit));
    assert_eq!(blood(&e, a), before - 2, "unpreventable, at once");
    assert!(e.state.cards[spawning.index()].locked);
    assert_eq!(
        e.state.cards[spawning.index()].counters,
        2,
        "the blood stays"
    );
}

/// Steam Tunnels: "Lock this card during your master phase to look at the
/// top three cards in your prey's library." The three noted, the card
/// locked; with the prey's library empty, not offered.
#[test]
fn steam_tunnels_looks_at_the_preys_top_three() {
    let mut e = game(2);
    e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    let tunnels = in_play(&mut e, 0, "Steam Tunnels", None);
    for name in ["Celerity", "Fortitude", "Potence", "Dominate"] {
        e.state.seats[1].library.push(find_library(name));
    }
    fill_hand(&mut e, 1);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    e.notes = Some(Vec::new());
    e.choose(index_of(&req, use_at(tunnels, 0, None, Target::None)));
    request(&mut e);
    let looks = noted_looks(&mut e);
    assert_eq!(looks.len(), 1);
    assert_eq!(looks[0].seat, SeatId(1));
    assert_eq!(looks[0].cards.len(), 3);
    assert!(e.state.cards[tunnels.index()].locked);

    let mut e = game(2);
    e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    let tunnels = in_play(&mut e, 0, "Steam Tunnels", None);
    hand(&mut e, 0, &["Unnatural Disaster"]);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(!req
        .options
        .contains(&use_at(tunnels, 0, None, Target::None)));
}

/// Unnatural Disaster: "Burn a location." Another Methuselah's location to
/// its ash heap; not playable with none in play.
#[test]
fn unnatural_disaster_burns_any_location() {
    let mut e = game(2);
    e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    hand(&mut e, 0, &["Unnatural Disaster"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(plays(&req).is_empty());

    let mut e = game(2);
    e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    let loc = in_play(&mut e, 1, "Steam Tunnels", None);
    hand(&mut e, 0, &["Unnatural Disaster"]);
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::Card(loc)));
    request(&mut e);
    assert!(!e.state.cards[loc.index()].in_play);
    assert!(ash_has(&e, 1, "Steam Tunnels"));
}

// ---- Ventrue Directorate Assembly, Watchtower, Weeping Stone -----------------------

/// Ventrue Directorate Assembly: "Each ready Ventrue gets an additional vote
/// during political actions." An untitled Ventrue: 1 vote in a political
/// action's referendum; none outside one (`votes_held`; the blood hunt:
/// `ventrue_directorate_assembly_adds_no_vote_in_a_blood_hunt`).
#[test]
fn ventrue_directorate_assembly_adds_ventrue_votes_in_political_actions() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), mid(0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    in_play(&mut e, 1, "Ventrue Directorate Assembly", None);
    assert_eq!(e.state.votes_held(v), 0);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, caller));
    to_referendum(&mut e);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, v), 1);
}

/// Ventrue Directorate Assembly in a blood hunt: not a political action
/// (advanced rules §3.7.8), so an untitled ready Ventrue has no vote there;
/// its titled prince votes its title's 2 only.
#[test]
fn ventrue_directorate_assembly_adds_no_vote_in_a_blood_hunt() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 0), 1, true);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    in_play(&mut e, 1, "Ventrue Directorate Assembly", None);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: d,
            action: ActionKind::Diablerie,
            target: Some(z),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
    let r = e.referendum().expect("the blood hunt");
    assert_eq!(r.votes_of(&e.state, v), 0);
    assert_eq!(r.votes_of(&e.state, p), 2);
}

/// "This card may be burned by any minion as a Ⓓ action; Brujah get +1
/// stealth when attempting that action." Seat 1's Brujah burns it at +1
/// stealth.
#[test]
fn ventrue_directorate_assembly_is_burned_with_brujah_stealth() {
    let mut e = game(2);
    e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let vda = in_play(&mut e, 0, "Ventrue Directorate Assembly", None);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, use_at(vda, 0, Some(b), Target::None));
    request(&mut e);
    assert_eq!(e.action().expect("action").stealth(&e.state), 1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Ventrue Directorate Assembly"));
}

/// Watchtower: Chosen are Called: "You may use a master phase action to draw
/// three cards (discard down to your hand size afterward)." "You can draw
/// more cards than are in the library …; draw as much as you can." [LSJ
/// 20060620] Two cards in the library: both drawn.
#[test]
fn watchtower_chosen_are_called_draws_from_a_short_library() {
    let mut e = game(2);
    e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    let w = in_play(&mut e, 0, "Watchtower: Chosen are Called", None);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    for name in ["Celerity", "Fortitude"] {
        e.state.seats[0].library.push(find_library(name));
    }
    let hand_before = e.state.seats[0].hand.len();
    e.choose(index_of(&req, use_at(w, 0, None, Target::None)));
    request(&mut e);
    assert!(e.state.seats[0].library.is_empty());
    assert_eq!(e.state.seats[0].hand.len(), hand_before + 2);
}

/// Weeping Stone: "+1 hand size. During your unlock phase, a ready Black
/// Hand vampire you control can gain 1 blood."
#[test]
fn weeping_stone_feeds_a_black_hand_vampire_and_raises_hand_size() {
    let mut e = game(2);
    let bh = e.add_minion(
        SeatId(0),
        textless(|c| c.black_hand && c.capacity >= 4, 0),
        3,
        false,
    );
    e.add_minion(SeatId(1), mid(1), 4, false);
    let before = e.state.hand_size(SeatId(0));
    let stone = in_play(&mut e, 0, "Weeping Stone", None);
    assert_eq!(e.state.hand_size(SeatId(0)), before + 1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick_when(&mut e, use_at(stone, 0, None, Target::Minion(bh)));
    request(&mut e);
    assert_eq!(blood(&e, bh), 4);
}

/// "Any Banu Haqim can burn this card as a Ⓓ action that costs 1 pool."
/// An action requiring Banu Haqim [LSJ 20080604]: a Brujah not offered it;
/// the Banu Haqim's Methuselah pays 1 pool.
#[test]
fn weeping_stone_is_burned_only_by_a_banu_haqim() {
    let mut e = game(2);
    e.add_minion(SeatId(0), mid(0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let h = e.add_minion(SeatId(1), plain("Banu Haqim", 0), 4, false);
    let stone = in_play(&mut e, 0, "Weeping Stone", None);
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!req
        .options
        .contains(&use_at(stone, 1, Some(b), Target::None)));
    e.choose(index_of(&req, use_at(stone, 1, Some(h), Target::None)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Weeping Stone"));
    assert_eq!(pool(&e)[1], 9);
}

// ---- modifier_reaction: referendum fixtures -------------------------------------

/// Seat 0's Sabbat caller `c` (a primogen: 1 vote) calls Black Forest Base's
/// referendum ("gain 2 pool"); seat 1's two primogens `x`, `y`; the
/// referendum under way, nobody cast.
fn politics(levels: &[&'static str]) -> (Engine, MinionId, MinionId, MinionId) {
    politics_with(levels, cam(1))
}

/// A Camarilla fixture vampire of capacity 4 to 8, no printed stealth.
#[track_caller]
fn cam(nth: usize) -> u16 {
    textless(
        |c| c.sect == Some(Sect::Camarilla) && (4..=8).contains(&c.capacity) && no_stealth(c),
        nth,
    )
}

/// [`politics`] with `x` of the crypt card `x_card`.
fn politics_with(levels: &[&'static str], x_card: u16) -> (Engine, MinionId, MinionId, MinionId) {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), sabbat(0), 4, false);
    let d = e.add_minion(SeatId(0), cam(0), 4, false);
    voter(&mut e, d);
    let x = e.add_minion(SeatId(1), x_card, 4, false);
    let y = e.add_minion(SeatId(1), cam(2), 4, false);
    for m in [c, x, y] {
        voter(&mut e, m);
    }
    e.state.minions[c.index()].gained_levels = levels.to_vec();
    let base = in_play(&mut e, 0, "Black Forest Base", None);
    pick(&mut e, use_at(base, 0, Some(c), Target::None));
    to_referendum(&mut e);
    (e, c, x, y)
}

/// A 1-vote title for `m`: a bishop if Sabbat, else a primogen.
fn voter(e: &mut Engine, m: MinionId) {
    let sabbat = e.state.sect_of(m) == Some(Sect::Sabbat);
    e.state.minions[m.index()].granted_title = Some(if sabbat {
        Title::Bishop
    } else {
        Title::Primogen
    });
}

/// A Sabbat fixture vampire, no printed stealth.
#[track_caller]
fn sabbat(nth: usize) -> u16 {
    textless(
        |c| c.sect == Some(Sect::Sabbat) && !c.seraph && c.capacity >= 4 && no_stealth(c),
        nth,
    )
}

// ---- Animal Magnetism, Surprise Influence ----------------------------------------

/// Animal Magnetism: "[ANI][OBF] This vampire gains 3 votes." The calling
/// vampire's: 1 + 3 votes. Surprise Influence: "This reacting vampire
/// gains 2 votes": seat 1's primogen at 3.
#[test]
fn animal_magnetism_and_surprise_influence_add_votes() {
    let (mut e, c, x, _) = politics(&["ani", "ani", "obf", "obf"]);
    hand(&mut e, 0, &["Animal Magnetism"]);
    hand(&mut e, 1, &["Surprise Influence"]);
    pick_when(&mut e, play(0, 1, c));
    pick_when(&mut e, play(0, 0, x));
    request(&mut e);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, c), 4);
    assert_eq!(r.votes_of(&e.state, x), 3);
}

// ---- The Art of Memory -----------------------------------------------------------

/// A Black Hand fixture vampire with `levels`, capacity 4 or more.
fn black_hand(e: &mut Engine, seat: u8, nth: usize, levels: &[&'static str]) -> MinionId {
    let m = e.add_minion(
        SeatId(seat),
        textless(|c| c.black_hand && c.capacity >= 4 && no_stealth(c), nth),
        4,
        false,
    );
    e.state.minions[m.index()].gained_levels = levels.to_vec();
    m
}

/// The Art of Memory: "If the action is successful, move one card played by
/// this vampire during this action from your ash heap to your hand." After
/// the action resolved and its combat [LSJ 20031112]: Bum's Rush's combat,
/// Undead Strength struck in it back to hand (Bum's Rush offered too, left
/// in the ash heap); The Art of Memory removed from the game.
#[test]
fn the_art_of_memory_returns_a_modifier_after_the_action_and_its_combat() {
    let mut e = game(2);
    let b = black_hand(&mut e, 0, 0, &["pot"]);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let x = e.add_minion(SeatId(1), mid(2), 4, false);
    e.add_minion(SeatId(1), mid(3), 4, false);
    hand(
        &mut e,
        0,
        &["Bum's Rush", "Art of Memory, The", "Undead Strength"],
    );
    pick(&mut e, play_at(0, 0, Some(b), Target::Minion(x)));
    pick_when(&mut e, play(0, 0, b));
    pick_when(&mut e, play(0, 0, b));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(e.combat().is_none(), "after the combat");
    let strength = find_library("Undead Strength");
    assert!(req
        .options
        .contains(&Choice::FindCard(find_library("Bum's Rush"))));
    e.choose(index_of(&req, Choice::FindCard(strength)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].hand.contains(&strength));
    assert!(!ash_has(&e, 0, "Undead Strength"));
    assert!(ash_has(&e, 0, "Bum's Rush"));
    assert!(e.state.seats[0]
        .removed
        .contains(&find_library("Art of Memory, The")));
}

/// "Can be used to retrieve the action card itself" [LSJ 20041115]: Govern
/// the Unaligned's bleed, the card back in hand, never in the ash heap.
#[test]
fn the_art_of_memory_can_return_the_action_card() {
    let mut e = game(2);
    let b = black_hand(&mut e, 0, 0, &["dom"]);
    e.add_minion(SeatId(0), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    hand(
        &mut e,
        0,
        &["Govern the Unaligned", "Art of Memory, The", "Threats"],
    );
    let govern = find_library("Govern the Unaligned");
    pick(&mut e, play(0, 0, b));
    pick_when(&mut e, play(0, 0, b));
    pick_when(&mut e, play(0, 0, b));
    pick_when(&mut e, Choice::FindCard(govern));
    assert!(ash_has(&e, 0, "Threats"));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].hand.contains(&govern));
    assert!(!ash_has(&e, 0, "Govern the Unaligned"));
}

/// Blocked, the action is not successful: nothing returned; The Art of
/// Memory still removed.
#[test]
fn the_art_of_memory_returns_nothing_when_blocked() {
    let mut e = game(2);
    let b = black_hand(&mut e, 0, 0, &["obf"]);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let x = e.add_minion(SeatId(1), mid(2), 4, false);
    e.add_minion(SeatId(1), mid(3), 4, false);
    hand(&mut e, 0, &["Art of Memory, The"]);
    pick(&mut e, bleed(b));
    pick_when(&mut e, play(0, 0, b));
    pick_when(&mut e, block(x));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::FindCard(_))
    });
    assert!(e.state.seats[0]
        .removed
        .contains(&find_library("Art of Memory, The")));
}

// ---- Babble, Random Patterns, Watch Commander, Ministry --------------------------

/// Seat 0's `a` (+1 stealth for the turn) bleeds seat 1.
fn stealthy_bleed(e: &mut Engine, a: MinionId, n: u8) {
    e.state.minions[a.index()].timed.push((
        Lasting::Stealth(n as i8),
        Expiry::EndOfTurn,
        SeatId(0),
    ));
    pick(e, bleed(a));
}

/// Babble [dem]: "Give another ready minion +1 intercept": seat 1's `x`
/// attempts to block a +1 stealth bleed; seat 1's `y` gives it +1: blocked
/// (sequencing) [ANK 20200607]. [DEM]: `y` locks to unlock seat 1's
/// locked `z`.
#[test]
fn babble_gives_another_methuselahs_blocker_intercept() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let x = e.add_minion(SeatId(1), mid(2), 4, false);
    let y = e.add_minion(SeatId(1), mid(3), 4, false);
    e.state.minions[y.index()].gained_levels = vec!["dem"];
    hand(&mut e, 1, &["Babble"]);
    e.notes = Some(Vec::new());
    stealthy_bleed(&mut e, a, 1);
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, y));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!noted_combats(&mut e).is_empty(), "blocked");

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let y = e.add_minion(SeatId(1), mid(3), 4, false);
    let z = e.add_minion(SeatId(1), mid(4), 4, false);
    e.state.minions[y.index()].gained_levels = vec!["dem", "dem"];
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 1, &["Babble"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play_at(0, 1, Some(y), Target::Minion(z)));
    request(&mut e);
    assert!(locked(&e, y) && !locked(&e, z));
}

/// Random Patterns: "[aus][dem] +1 intercept" (a reaction, seat 1's
/// blocker reaches a +1 stealth bleed); "[AUS][DEM] Only usable if a minion
/// attempts to block. The blocking minion gets -1 intercept" (an action
/// modifier: the same block then falls short).
#[test]
fn random_patterns_reaction_and_superior_modifier() {
    for superior in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), mid(0), 4, false);
        e.add_minion(SeatId(0), mid(1), 4, false);
        let x = e.add_minion(SeatId(1), mid(2), 4, false);
        e.add_minion(SeatId(1), mid(3), 4, false);
        e.state.minions[x.index()].gained_levels = vec!["aus", "dem"];
        e.state.minions[a.index()].gained_levels = vec!["aus", "aus", "dem", "dem"];
        hand(&mut e, 1, &["Random Patterns"]);
        if superior {
            hand(&mut e, 0, &["Random Patterns"]);
        }
        e.notes = Some(Vec::new());
        stealthy_bleed(&mut e, a, 1);
        pick_when(&mut e, block(x));
        pick_when(&mut e, play(0, 0, x));
        if superior {
            pick_when(&mut e, play(0, 1, a));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            noted_combats(&mut e).is_empty(),
            superior,
            "superior {superior}"
        );
    }
}

/// Watch Commander: "Only usable when attempting to block a directed
/// action. +1 intercept." Against a +1 stealth bleed: offered; against an
/// undirected +1 stealth hunt... a hunt is not blocked by others: a +1
/// stealth equip action, never offered.
#[test]
fn watch_commander_needs_a_directed_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let w = black_hand(&mut e, 1, 0, &[]);
    e.add_minion(SeatId(1), mid(3), 4, false);
    hand(&mut e, 1, &["Watch Commander"]);
    e.notes = Some(Vec::new());
    stealthy_bleed(&mut e, a, 1);
    pick_when(&mut e, block(w));
    pick_when(&mut e, play(0, 0, w));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!noted_combats(&mut e).is_empty(), "blocked");

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let w = black_hand(&mut e, 1, 0, &[]);
    e.add_minion(SeatId(1), mid(3), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    hand(&mut e, 1, &["Watch Commander"]);
    e.state.minions[a.index()]
        .timed
        .push((Lasting::Stealth(1), Expiry::EndOfTurn, SeatId(0)));
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, block(w));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, w));
}

/// Ministry: "This vampire gets +2 intercept. If the acting vampire is
/// Sabbat, this vampire gets an additional +1 intercept. If this vampire
/// does not successfully block this action, he or she is locked after action
/// resolution." The +1 lost as the acting vampire turns Anarch with The Red
/// Question [ANK 20190416]: seat 1's Black Hand blocker at +3 against a +3
/// stealth Sabbat bleed, then +2: the block fails, the blocker locked after
/// resolution; Ministry replaced only after the action.
#[test]
fn ministry_loses_its_extra_intercept_when_the_acting_vampire_turns_anarch() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| c.sect == Some(Sect::Sabbat) && c.capacity >= 4 && no_stealth(c),
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(0), mid(1), 4, false);
    let r = black_hand(&mut e, 1, 0, &[]);
    e.add_minion(SeatId(1), mid(3), 4, false);
    hand(&mut e, 0, &["Red Question, The"]);
    hand(&mut e, 1, &["Ministry"]);
    fill_hand(&mut e, 1);
    e.state.seats[1].library = vec![find_library("Soak"); 6].into();
    e.notes = Some(Vec::new());
    stealthy_bleed(&mut e, a, 3);
    pick_when(&mut e, block(r));
    pick_when(&mut e, play(0, 0, r));
    assert_eq!(e.state.seats[1].library.len(), 6, "not replaced yet");
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(noted_combats(&mut e).is_empty(), "the block fell short");
    assert!(locked(&e, r));
    assert_eq!(
        e.state.seats[1].library.len(),
        5,
        "replaced after the action"
    );
}

// ---- Circumspect Revelation, Council of Seraphim ------------------------------------

/// Circumspect Revelation: "Only usable when a Sabbat vampire attempts to
/// block. The block attempt fails, and that Sabbat vampire cannot attempt to
/// block this action again." A Camarilla vampire's attempt: never offered.
#[test]
fn circumspect_revelation_fails_a_sabbat_vampires_block_attempt() {
    let sabbat = |n| {
        textless(
            |c| c.sect == Some(Sect::Sabbat) && c.capacity >= 4 && no_stealth(c),
            n,
        )
    };
    let camarilla = |n| {
        textless(
            |c| c.sect == Some(Sect::Camarilla) && c.capacity >= 4 && no_stealth(c),
            n,
        )
    };
    let mut e = game(2);
    let b = black_hand(&mut e, 0, 0, &[]);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let x = e.add_minion(SeatId(1), sabbat(0), 4, false);
    e.add_minion(SeatId(1), camarilla(0), 4, false);
    hand(&mut e, 0, &["Circumspect Revelation"]);
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, b));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(x));
    assert!(pool(&e)[1] < 10, "the bleed went through");

    let mut e = game(2);
    let b = black_hand(&mut e, 0, 0, &[]);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let y = e.add_minion(SeatId(1), camarilla(0), 4, false);
    e.add_minion(SeatId(1), sabbat(0), 4, false);
    hand(&mut e, 0, &["Circumspect Revelation"]);
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(y));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, b));
}

/// Council of Seraphim: a Sabbat vampire's attempt fails, not locked, and it
/// cannot block again; otherwise the blocking minion gets -1 intercept: a
/// Camarilla vampire's block of a 0-stealth bleed falls short.
#[test]
fn council_of_seraphim_fails_a_sabbat_attempt_and_weakens_another() {
    let seraph = |n| textless(|c| c.seraph && c.capacity >= 4 && no_stealth(c), n);
    let sabbat = |n| {
        textless(
            |c| c.sect == Some(Sect::Sabbat) && !c.seraph && c.capacity >= 4 && no_stealth(c),
            n,
        )
    };
    let camarilla = |n| {
        textless(
            |c| c.sect == Some(Sect::Camarilla) && c.capacity >= 4 && no_stealth(c),
            n,
        )
    };
    for sabbat_blocker in [true, false] {
        let mut e = game(2);
        let s = e.add_minion(SeatId(0), seraph(0), 4, false);
        e.add_minion(SeatId(0), mid(1), 4, false);
        let x = e.add_minion(
            SeatId(1),
            if sabbat_blocker {
                sabbat(0)
            } else {
                camarilla(0)
            },
            4,
            false,
        );
        e.add_minion(SeatId(1), mid(3), 4, false);
        hand(&mut e, 0, &["Council of Seraphim"]);
        e.notes = Some(Vec::new());
        pick(&mut e, bleed(s));
        pick_when(&mut e, block(x));
        pick_when(&mut e, play(0, 0, s));
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(x));
        assert!(noted_combats(&mut e).is_empty(), "sabbat {sabbat_blocker}");
        assert!(!locked(&e, x));
    }
}

/// "The burn option cannot be used if all your Seraph are in torpor." [LSJ
/// 20091204] Its player's only Seraph torpid: no burn-option discard; no
/// Seraph at all: offered.
#[test]
fn a_torpid_seraph_bars_council_of_seraphims_burn_option() {
    let seraph = textless(|c| c.seraph && c.capacity >= 4, 0);
    for (with_seraph, offered) in [(true, false), (false, true)] {
        let mut e = game(2);
        e.add_minion(SeatId(0), mid(0), 4, false);
        e.add_minion(SeatId(1), mid(1), 4, false);
        if with_seraph {
            e.add_minion(SeatId(1), seraph, 4, true);
        }
        e.state.seats[1].hand = vec![find_library("Council of Seraphim")].into();
        e.state.seats[1].library = vec![find_library("Soak"); 6].into();
        pick(&mut e, Choice::Pass);
        let found = loop {
            let req = request(&mut e);
            if req.window == WindowLabel::UnlockAbilities && req.seat == SeatId(1) {
                break req.options.contains(&Choice::Discard(0));
            }
            if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                break false;
            }
            e.choose(fallback(&req));
        };
        assert_eq!(found, offered, "seraph {with_seraph}");
    }
}

// ---- Disarming Presence, Treachery, Under the Skin ------------------------------------

/// Disarming Presence [pre]: "When a vampire votes in this referendum, lock
/// him or her." Locked as it casts [RTR 19970425], its votes still counted
/// [RTR 19941109]. [PRE]: its player's own vampires spared.
#[test]
fn disarming_presence_locks_each_vampire_as_it_votes() {
    for superior in [false, true] {
        let (mut e, c, x, _) = politics(if superior { &["pre", "pre"] } else { &["pre"] });
        let d = MinionId(1);
        hand(&mut e, 0, &["Disarming Presence"]);
        pick_when(&mut e, play(0, superior as u8, c));
        pick_when(&mut e, votes(d, true));
        assert_eq!(locked(&e, d), !superior, "superior {superior}");
        assert!(!locked(&e, x));
        pick_when(&mut e, votes(x, false));
        assert!(locked(&e, x), "locked as it casts");
        assert_eq!(cast_of(&e, x), Some((false, 1)), "counted");
    }
}

/// Treachery [pre]: "If the referendum passes, the controller of the vampire
/// calling the referendum burns 1 pool", after the effects [LSJ
/// 20090115-2]: Black Forest Base's +2, then -1; a failed one: nothing.
#[test]
fn treachery_burns_the_caller_after_the_referendum_effects() {
    for pass in [true, false] {
        let (mut e, c, x, y) = politics(&[]);
        e.state.minions[x.index()].gained_levels = vec!["pre"];
        hand(&mut e, 1, &["Treachery"]);
        pick_when(&mut e, play(0, 0, x));
        pick_when(&mut e, votes(c, true));
        if !pass {
            pick_when(&mut e, votes(y, false));
        }
        close_polling(&mut e);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            pool(&e)[0],
            if pass { 10 + 2 - 1 } else { 10 },
            "pass {pass}"
        );
    }
}

/// Treachery [PRE]: "any other Methuselahs who cast votes or ballots in
/// favor of the referendum burn 1 pool when the votes are tallied" [RTR
/// 20010711]: seat 2 cast in favor burns 1, seat 1 (Treachery's, no cast)
/// nothing, the caller only its [pre] pool after the +2.
#[test]
fn treachery_superior_burns_every_other_methuselah_in_favor() {
    let mut e = game(3);
    let c = e.add_minion(SeatId(0), sabbat(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let x = e.add_minion(SeatId(1), mid(2), 4, false);
    let z = e.add_minion(SeatId(2), mid(3), 4, false);
    e.state.minions[c.index()].granted_title = Some(Title::Bishop);
    e.state.minions[z.index()].granted_title = Some(Title::Primogen);
    e.state.minions[x.index()].gained_levels = vec!["pre", "pre"];
    let base = in_play(&mut e, 0, "Black Forest Base", None);
    hand(&mut e, 1, &["Treachery"]);
    pick(&mut e, use_at(base, 0, Some(c), Target::None));
    to_referendum(&mut e);
    pick_when(&mut e, play(0, 1, x));
    pick_when(&mut e, votes(c, true));
    pick_when(&mut e, votes(z, true));
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10 + 2 - 1, 10, 10 - 1]);
}

/// Treachery [PRE]'s own player in favor (rules-qa 126.189: "any other
/// Methuselahs" is other than the caller's controller): seat 1, which
/// played it, casts its primogen's vote in favor and burns 1 pool too.
#[test]
fn treachery_superior_burns_its_own_player_in_favor() {
    let mut e = game(3);
    let c = e.add_minion(SeatId(0), sabbat(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let x = e.add_minion(SeatId(1), mid(2), 4, false);
    e.add_minion(SeatId(2), mid(3), 4, false);
    e.state.minions[c.index()].granted_title = Some(Title::Bishop);
    e.state.minions[x.index()].granted_title = Some(Title::Primogen);
    e.state.minions[x.index()].gained_levels = vec!["pre", "pre"];
    let base = in_play(&mut e, 0, "Black Forest Base", None);
    hand(&mut e, 1, &["Treachery"]);
    pick(&mut e, use_at(base, 0, Some(c), Target::None));
    to_referendum(&mut e);
    pick_when(&mut e, play(0, 1, x));
    pick_when(&mut e, votes(c, true));
    pick_when(&mut e, votes(x, true));
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10 + 2 - 1, 10 - 1, 10]);
}

/// `m`'s standing cast in the referendum: (in favor, votes).
fn cast_of(e: &Engine, m: MinionId) -> Option<(bool, u8)> {
    let r = e.referendum().expect("referendum");
    r.records
        .iter()
        .find(|v| v.source == zvtes::referendum::Source::Vampire(m) && !v.canceled)
        .map(|v| (v.favor, r.n_of(&e.state, v) as u8))
}

/// Under the Skin [AUS][PRE]: "Choose a younger vampire. That vampire
/// abstains … and lock that vampire at the end of the action if the
/// referendum passes." Seat 1's younger primogen never offered a cast; the
/// referendum passes: locked as the action ends, unlocked before.
#[test]
fn under_the_skin_superior_abstains_and_locks_on_a_pass() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), sabbat(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let x = e.add_minion(
        SeatId(1),
        textless(
            |k| k.capacity < CRYPT[sabbat(0) as usize].capacity && no_stealth(k),
            0,
        ),
        3,
        false,
    );
    e.add_minion(SeatId(1), mid(3), 4, false);
    e.state.minions[c.index()].granted_title = Some(Title::Bishop);
    e.state.minions[x.index()].granted_title = Some(Title::Primogen);
    e.state.minions[c.index()].gained_levels = vec!["aus", "aus", "pre", "pre"];
    let base = in_play(&mut e, 0, "Black Forest Base", None);
    hand(&mut e, 0, &["Under the Skin"]);
    pick(&mut e, use_at(base, 0, Some(c), Target::None));
    to_referendum(&mut e);
    pick_when(&mut e, play_at(0, 1, Some(c), Target::Minion(x)));
    pick_when(&mut e, votes(c, true));
    assert!(!locked(&e, x));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::CastVotes { minion, .. } if *minion == x),
    );
    assert_eq!(pool(&e)[0], 12, "passed");
    assert!(locked(&e, x), "locked as the action ended");
}

// ---- Kindred Coercion -----------------------------------------------------------

/// A vampire of capacity 11, any title (fixture vampires run out there).
fn elder() -> u16 {
    fixture_vampire(|c| c.capacity == 11, 0)
}

/// Seat 0's Sabbat caller `c` and a second, older primogen `o` (capacity
/// 10); seat 1's primogens `x`, `y` (capacity at most 8); seat 2's reacting
/// vampire `k` (capacity 9 or the card given) with `levels`; Black Forest
/// Base's referendum under way.
fn coercion_table(
    k_card: Option<u16>,
    k_blood: u8,
    levels: &[&'static str],
) -> (Engine, [MinionId; 5]) {
    let mut e = game(3);
    let c = e.add_minion(SeatId(0), sabbat(0), 4, false);
    let o = e.add_minion(
        SeatId(0),
        fixture_vampire(|c| c.capacity == 10, 0),
        4,
        false,
    );
    let x = e.add_minion(
        SeatId(1),
        textless(|k| k.clan == "Ventrue antitribu" && k.capacity <= 8, 0),
        4,
        false,
    );
    let y = e.add_minion(SeatId(1), mid(3), 4, false);
    let k = e.add_minion(
        SeatId(2),
        k_card.unwrap_or_else(|| textless(|k| k.capacity == 9, 0)),
        k_blood,
        false,
    );
    for m in [c, o, x, y] {
        voter(&mut e, m);
    }
    e.state.minions[k.index()].gained_levels = levels.to_vec();
    let base = in_play(&mut e, 0, "Black Forest Base", None);
    hand(&mut e, 2, &["Kindred Coercion"]);
    pick(&mut e, use_at(base, 0, Some(c), Target::None));
    to_referendum(&mut e);
    (e, [c, o, x, y, k])
}

/// Kindred Coercion [dom], X = 2: two younger casts canceled, 2 blood paid;
/// the older vampire never a pick.
#[test]
fn kindred_coercion_makes_two_younger_vampires_abstain() {
    let (mut e, [_, o, x, y, k]) = coercion_table(None, 4, &["dom"]);
    pick_when(&mut e, votes(x, true));
    pick_when(&mut e, votes(y, true));
    pick_when(&mut e, play(0, 0, k));
    let req = until(&mut e, SeatId(2), WindowLabel::PlayTerms);
    assert!(!req.options.contains(&Choice::ChooseMinion(o)), "older");
    e.choose(index_of(&req, Choice::ChooseMinion(x)));
    pick(&mut e, Choice::ChooseMinion(y));
    pick(&mut e, Choice::Pass);
    let req = request(&mut e);
    let _ = req;
    assert_eq!(cast_of(&e, x), None);
    assert_eq!(cast_of(&e, y), None);
    assert_eq!(blood(&e, k), 2);
}

/// "[dom] Can force a vampire who has not voted yet to keep abstaining."
/// [RTR 19991001] The uncast pick is never offered a cast.
#[test]
fn kindred_coercion_keeps_an_uncast_vampire_abstaining() {
    let (mut e, [_, _, x, _, k]) = coercion_table(None, 4, &["dom"]);
    pick_when(&mut e, play(0, 0, k));
    pick_when(&mut e, Choice::ChooseMinion(x));
    pick(&mut e, Choice::Pass);
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::CastVotes { minion, .. } if *minion == x),
    );
}

/// "[DOM] Playable on an abstaining vampire: it does not force them to vote,
/// but restricts their choice if they do." [RTR 19991001] [LSJ 20030828]
/// The treatment chosen pick by pick (PIB 20131014): `x`'s cast in favor
/// turned against, uncast `y` restricted to against, `o`'s cast made to
/// abstain.
#[test]
fn kindred_coercion_superior_treats_each_pick_its_own_way() {
    let (mut e, [_, o, x, y, k]) = coercion_table(Some(elder()), 4, &["dom", "dom"]);
    pick_when(&mut e, votes(x, true));
    pick_when(&mut e, votes(o, true));
    pick_when(&mut e, play(0, 1, k));
    for m in [x, y, o] {
        pick(&mut e, Choice::ChooseMinion(m));
    }
    pick(&mut e, Choice::Pass);
    let side = |m, favor| Choice::CastVotes { minion: m, favor };
    pick_when(&mut e, side(x, false));
    pick_when(&mut e, side(y, false));
    let req = offering(&mut e, side(o, false));
    e.choose(index_of(&req, Choice::Pass));
    request(&mut e);
    assert_eq!(cast_of(&e, x), Some((false, 1)), "turned");
    assert_eq!(cast_of(&e, o), None, "abstains");
    let req = offering(&mut e, votes(y, false));
    assert!(!req.options.contains(&votes(y, true)), "restricted");
}

/// "Can choose any number for X for [aus] and [dom] reaction cards that cost
/// X (e.g. Kindred Coercion), and still pay no blood." [LSJ 20051117]
/// Helena (ADV), 1 blood, picks three, pays nothing.
#[test]
fn helena_plays_kindred_coercion_for_more_than_her_blood() {
    let helena = CRYPT
        .iter()
        .position(|c| c.name == "Helena" && c.adv)
        .expect("Helena (ADV)") as u16;
    let (mut e, [c, _, x, y, k]) = coercion_table(Some(helena), 1, &[]);
    pick_when(&mut e, play(0, 0, k));
    for m in [x, y, c] {
        pick(&mut e, Choice::ChooseMinion(m));
    }
    pick(&mut e, Choice::Pass);
    request(&mut e);
    assert_eq!(blood(&e, k), 1);
    let r = e.referendum().expect("referendum");
    for m in [x, y, c] {
        assert!(r.abstaining.contains(&m));
    }
}

// ---- Loyalist, Wrong and Crosswise ------------------------------------------------

/// Loyalist: "Usable by a locked vampire. This vampire gains 3 votes against
/// the referendum." Gaining them forces its cast against [LSJ 20031119]:
/// locked uncast `x`, 1 + 3 against. Not by a vampire voting in favor [LSJ
/// 20031115]: `y`, cast in favor, never offered it.
#[test]
fn loyalist_votes_three_against_from_a_locked_vampire() {
    let (mut e, c, x, y) = politics_with(&[], plain("Ventrue antitribu", 0));
    e.state.minions[x.index()].locked = true;
    hand(&mut e, 1, &["Loyalist", "Loyalist"]);
    pick_when(&mut e, votes(c, true));
    pick_when(&mut e, votes(y, true));
    pick_when(&mut e, play(0, 0, x));
    let req = offering(&mut e, votes(x, false));
    assert!(!req.options.contains(&votes(x, true)), "forced against");
    e.choose(index_of(&req, votes(x, false)));
    assert_eq!(cast_of(&e, x), Some((false, 4)));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, y));
    assert_eq!(pool(&e)[0], 10, "2 for, 4 against: failed");
}

/// "Cannot be used by a minion who is voting in favor of the referendum."
/// [LSJ 20031115]
#[test]
fn loyalist_is_not_offered_to_a_vampire_voting_in_favor() {
    let (mut e, _, x, _) = politics_with(&[], plain("Ventrue antitribu", 0));
    hand(&mut e, 1, &["Loyalist"]);
    pick_when(&mut e, votes(x, true));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
}

/// "If the acting vampire is Sabbat and the referendum fails, move up to 2
/// blood from the acting vampire to this vampire." The Sabbat caller's
/// referendum fails 1 to 4: 2 of its blood to Loyalist's vampire (0 to 2
/// offered).
#[test]
fn loyalist_drains_a_sabbat_caller_when_the_referendum_fails() {
    let (mut e, c, x, _) = politics_with(&[], plain("Ventrue antitribu", 0));
    e.state.minions[x.index()].blood = 2;
    hand(&mut e, 1, &["Loyalist"]);
    pick_when(&mut e, votes(c, true));
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, votes(x, false));
    close_polling(&mut e);
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert!(req.options.contains(&set_x(0)) && !req.options.contains(&set_x(3)));
    e.choose(index_of(&req, set_x(2)));
    request(&mut e);
    assert_eq!(blood(&e, c), 2);
    assert_eq!(blood(&e, x), 4);
}

/// "If a minion who has gained the votes provided against the referendum, is
/// subsequently forced to vote in favor, the provided votes are not counted
/// when votes are tallied until that minion is again forced to vote against
/// the referendum." [LSJ 20031115] Loyalist's 4 against; Kindred Coercion
/// [DOM] turns it in favor: 1, Loyalist's 3 dropped. Wrong and Crosswise's 4
/// likewise (a turn in favor exists: Kindred Coercion).
#[test]
fn loyalists_votes_drop_while_turned_in_favor() {
    for card in ["Loyalist", "Wrong and Crosswise"] {
        let (mut e, [_, _, x, _, k]) = coercion_table(None, 4, &["dom", "dom"]);
        e.state.minions[x.index()].gained_levels = vec!["dem", "dem"];
        hand(&mut e, 1, &[card]);
        let mode = u8::from(card == "Wrong and Crosswise");
        pick_when(&mut e, votes(x, false));
        pick_when(&mut e, play(0, mode, x));
        request(&mut e);
        assert_eq!(
            cast_of(&e, x),
            Some((false, if mode == 1 { 5 } else { 4 })),
            "{card}"
        );
        pick_when(&mut e, play(0, 1, k));
        pick(&mut e, Choice::ChooseMinion(x));
        pick(&mut e, Choice::Pass);
        pick_when(
            &mut e,
            Choice::CastVotes {
                minion: x,
                favor: true,
            },
        );
        request(&mut e);
        assert_eq!(cast_of(&e, x), Some((true, 1)), "{card}");
    }
}

// ---- Political Backlash, Pulling Strings, Scrying of Secrets ----------------------------

/// Political Backlash: "Only usable when a referendum fails. The controller
/// of the acting vampire burns 2 pool." Two copies by seat 1's two
/// vampires: 4; a passed referendum: not offered.
#[test]
fn political_backlash_burns_the_failed_callers_pool() {
    let (mut e, c, x, y) = politics(&[]);
    hand(&mut e, 1, &["Political Backlash", "Political Backlash"]);
    pick_when(&mut e, votes(c, true));
    pick_when(&mut e, votes(x, false));
    pick_when(&mut e, votes(y, false));
    close_polling(&mut e);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 0, y));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[0], 10 - 4);

    let (mut e, c, x, _) = politics(&[]);
    hand(&mut e, 1, &["Political Backlash"]);
    pick_when(&mut e, votes(c, true));
    close_polling(&mut e);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
}

/// Pulling Strings [dom]: "Force a younger vampire to abstain": its cast
/// canceled; an older one not offered. [DOM]: the older one too.
#[test]
fn pulling_strings_cancels_a_younger_vampires_votes() {
    for superior in [false, true] {
        let (mut e, [_, o, x, _, k]) =
            coercion_table(None, 4, if superior { &["dom", "dom"] } else { &["dom"] });
        hand(&mut e, 2, &["Pulling Strings"]);
        pick_when(&mut e, votes(x, true));
        pick_when(&mut e, votes(o, true));
        let req = offering(&mut e, play_at(1, 0, Some(k), Target::Minion(x)));
        let older = play_at(1, superior as u8, Some(k), Target::Minion(o));
        assert_eq!(
            req.options.contains(&older),
            superior,
            "superior {superior}"
        );
        e.choose(index_of(
            &req,
            play_at(1, superior as u8, Some(k), Target::Minion(x)),
        ));
        request(&mut e);
        assert_eq!(cast_of(&e, x), None);
    }
}

/// Scrying of Secrets [AUS]: after a successful bleed, the top 7 of the bled
/// library looked at, and one bleed action card among them discarded, the
/// player's choice: Enchant Kindred and Govern the Unaligned offered, not
/// Intimidation at depth 8, nor Conditioning (a modifier). [ANK 20190425] [ANK 20190113]: only
/// after-resolution effects follow it (played in that window); a blocked
/// bleed: never offered.
#[test]
fn scrying_of_secrets_is_played_after_a_successful_bleed_only() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["aus", "aus"];
    hand(&mut e, 0, &["Scrying of Secrets"]);
    // Top first: Conditioning (a modifier), Enchant Kindred, Soak, Govern
    // the Unaligned, Soak x3, Intimidation (8th).
    let top = [
        "Conditioning",
        "Enchant Kindred",
        "Soak",
        "Govern the Unaligned",
        "Soak",
        "Soak",
        "Soak",
        "Intimidation",
    ];
    e.state.seats[1].library = top
        .iter()
        .rev()
        .map(|n| find_library(n))
        .collect::<Vec<_>>()
        .into();
    fill_hand(&mut e, 1);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(a));
    let req = offering(&mut e, play(0, 1, a));
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    e.choose(index_of(&req, play(0, 1, a)));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    let mut want = vec![
        Choice::FindCard(find_library("Enchant Kindred")),
        Choice::FindCard(find_library("Govern the Unaligned")),
    ];
    want.sort_by_key(|c| match c {
        Choice::FindCard(k) => *k,
        _ => 0,
    });
    assert_eq!(req.options, want, "mandatory, bleed action cards only");
    e.choose(index_of(
        &req,
        Choice::FindCard(find_library("Govern the Unaligned")),
    ));
    request(&mut e);
    assert!(ash_has(&e, 1, "Govern the Unaligned"));
    assert!(!ash_has(&e, 1, "Enchant Kindred"));
    assert_eq!(noted_looks(&mut e)[0].cards.len(), 7);

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let x = e.add_minion(SeatId(1), mid(2), 4, false);
    e.add_minion(SeatId(1), mid(3), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["aus"];
    hand(&mut e, 0, &["Scrying of Secrets"]);
    e.state.seats[1].library = vec![find_library("Soak"); 6].into();
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, a));
}

// ---- misc: allies, retainers, equipment ------------------------------------------

/// The card in play that is ally `m`'s body.
fn body_of(e: &Engine, m: MinionId) -> CardId {
    CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.body == Some(m))
            .expect("an ally's card") as u32,
    )
}

/// Abyssal Hunter: "can inflict 1 damage on any minion as a Ⓓ action":
/// seat 1's vampire loses 1 blood, the action directed at seat 1; at seat
/// 1's 1-life ally, it burns: an ally burned by a directed action [ANK
/// 20220507].
#[test]
fn abyssal_hunter_wounds_any_minion_as_a_directed_action() {
    let mut e = game(2);
    let h = e.add_ally(SeatId(0), find_library("Abyssal Hunter"));
    e.add_minion(SeatId(0), mid(0), 4, false);
    let x = e.add_minion(SeatId(1), mid(1), 4, false);
    let stray = e.add_ally(SeatId(1), find_library("Underbridge Stray"));
    let card = body_of(&e, h);
    pick(&mut e, use_at(card, 0, Some(h), Target::Minion(x)));
    request(&mut e);
    assert_eq!(
        e.action().expect("action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 3);
    // Unlocked, the window built anew by another action.
    e.state.minions[h.index()].locked = false;
    pick(&mut e, action(MinionId(1), ActionKind::Hunt));
    pick_when(&mut e, use_at(card, 0, Some(h), Target::Minion(stray)));
    request(&mut e);
    assert_eq!(
        e.action().expect("action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[stray.index()].in_play, "burned");
    assert!(ash_has(&e, 1, "Underbridge Stray"));
}

/// "During your unlock phase, Abyssal Hunter inflicts 1 damage on a ready
/// vampire you control": its controller's only ready vampire loses 1 blood.
/// Carolina Vález, immune to damage from allies, loses none [ANK 20210228];
/// nor from his action.
#[test]
fn abyssal_hunter_wounds_its_own_vampire_each_unlock_phase() {
    for carolina in [false, true] {
        let mut e = game(2);
        let v = e.add_minion(
            SeatId(0),
            if carolina {
                find_card("Carolina Vález")
            } else {
                mid(0)
            },
            4,
            false,
        );
        e.add_ally(SeatId(0), find_library("Abyssal Hunter"));
        e.add_minion(SeatId(1), mid(1), 4, false);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            blood(&e, v),
            if carolina { 4 } else { 3 },
            "carolina {carolina}"
        );
    }
    let mut e = game(2);
    let h = e.add_ally(SeatId(0), find_library("Abyssal Hunter"));
    e.add_minion(SeatId(0), mid(0), 4, false);
    let c = e.add_minion(SeatId(1), find_card("Carolina Vález"), 4, false);
    let card = body_of(&e, h);
    pick(&mut e, use_at(card, 0, Some(h), Target::Minion(c)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, c), 4);
}

/// "Burn Abyssal Hunter if any ranged aggravated damage is inflicted on it
/// (even if prevented)": at close range [PIB 20110818], White Phosphorus
/// Grenade's 2R aggravated: burned; a .44 Magnum's ranged normal strike:
/// not; with Dragon's Breath Rounds' aggravated damage: burned (a fix: only
/// a strike printed ranged aggravated was read).
#[test]
fn abyssal_hunter_burns_to_ranged_aggravated_damage_even_prevented() {
    for (gun, rounds, burned) in [
        ("White Phosphorus Grenade", false, true),
        (".44 Magnum", false, false),
        (".44 Magnum", true, true),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), mid(0), 4, false);
        e.add_minion(SeatId(0), mid(1), 4, false);
        let h = e.add_ally(SeatId(1), find_library("Abyssal Hunter"));
        // Life enough to outlast the damage itself.
        e.state.minions[h.index()].blood = 9;
        e.add_minion(SeatId(1), mid(2), 4, false);
        let w = in_play(&mut e, 0, gun, Some(a));
        if rounds {
            hand(&mut e, 0, &["Dragon's Breath Rounds"]);
        }
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(h));
        pick_when(&mut e, use_at(w, 0, None, Target::None));
        if rounds {
            pick_when(&mut e, play(0, 0, a));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            !e.state.minions[h.index()].in_play,
            burned,
            "{gun} {rounds}"
        );
    }
}

/// Chalice of Kinship: "The priscus, cardinal or regent with this equipment
/// can give each other ready Sabbat vampire you control 1 blood": its
/// cardinal's other ready Sabbat vampire +1, a torpid one, a Camarilla one
/// and the bearer nothing. An action requiring the title [LSJ 20080604]: an
/// untitled bearer is not offered it.
#[test]
fn chalice_of_kinship_feeds_each_other_ready_sabbat_vampire() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), sabbat(0), 3, false);
    e.state.minions[c.index()].granted_title = Some(Title::Cardinal);
    let s2 = e.add_minion(SeatId(0), sabbat(1), 2, false);
    let s3 = e.add_minion(SeatId(0), sabbat(2), 2, true);
    let k = e.add_minion(SeatId(0), cam(0), 2, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    let ch = in_play(&mut e, 0, "Chalice of Kinship", Some(c));
    pick(&mut e, use_at(ch, 0, Some(c), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!([c, s2, s3, k].map(|m| blood(&e, m)), [3, 3, 2, 2]);

    let mut e = game(2);
    let u = e.add_minion(SeatId(0), sabbat(0), 3, false);
    e.add_minion(SeatId(0), sabbat(1), 2, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    let ch = in_play(&mut e, 0, "Chalice of Kinship", Some(u));
    let req = request(&mut e);
    assert!(!req.options.contains(&use_at(ch, 0, Some(u), Target::None)));
}

/// "The action provided by the equipment to the attached minion fizzles …
/// if the equipment is not attached to the acting minion when the action
/// resolves" [RTR 19960221]: the Chalice moved off during the action, no
/// blood.
#[test]
fn chalice_of_kinship_action_fizzles_without_the_chalice() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), sabbat(0), 3, false);
    e.state.minions[c.index()].granted_title = Some(Title::Cardinal);
    let s2 = e.add_minion(SeatId(0), sabbat(1), 2, false);
    e.add_minion(SeatId(1), mid(1), 4, false);
    let ch = in_play(&mut e, 0, "Chalice of Kinship", Some(c));
    pick(&mut e, use_at(ch, 0, Some(c), Target::None));
    request(&mut e);
    e.state.cards[ch.index()].host = Some(s2);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, s2), 2);
}

/// Corpse Balloon [vic]: its employer blocking a directed action gets a
/// maneuver in the first round; [VIC]: +1 intercept during directed
/// actions — the version fixed as employed [RTR 19941109] [ANK 20180612]:
/// at [vic] a +1 stealth bleed is out of reach, at [VIC] blocked. A ghoul is
/// no mortal [ANK 20200203-2] [LSJ 20060515] (Marijava Thuggee too).
#[test]
fn corpse_balloon_gives_its_blocking_employer_a_first_round_maneuver() {
    for superior in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), mid(0), 4, false);
        e.add_minion(SeatId(0), mid(1), 4, false);
        let y = e.add_minion(SeatId(1), mid(2), 4, false);
        e.add_minion(SeatId(1), mid(3), 4, false);
        in_play_mode(&mut e, 1, "Corpse Balloon", Some(y), superior as u8);
        stealthy_bleed(&mut e, a, u8::from(superior));
        pick_when(&mut e, block(y));
        let req = offering(&mut e, Choice::UseManeuver);
        assert_eq!(req.seat, SeatId(1), "superior {superior}");
    }
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let y = e.add_minion(SeatId(1), mid(2), 4, false);
    e.add_minion(SeatId(1), mid(3), 4, false);
    in_play_mode(&mut e, 1, "Corpse Balloon", Some(y), 0);
    stealthy_bleed(&mut e, a, 1);
    never_reaching_until(&mut e, SeatId(0), WindowLabel::MinionPhase, y);
    for name in ["Corpse Balloon", "Marijava Thuggee"] {
        assert!(!zvtes::library::LIBRARY[find_library(name) as usize]
            .labels
            .contains(&zvtes::library::Label::Mortal));
    }
    let t = e.add_ally(SeatId(1), find_library("Marijava Thuggee"));
    assert!(!e.state.is_mortal(t));
}

// ---- Weapons, vehicle, retainer, vanilla allies ---------------------------------------

/// Seat 0's `a` (strength 1, with `weapon`) bleeds; seat 1's `x` (4 blood)
/// blocks.
fn armed_bleed(weapons: &[&str]) -> (Engine, MinionId, MinionId, Vec<CardId>) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let x = e.add_minion(SeatId(1), mid(2), 4, false);
    e.add_minion(SeatId(1), mid(3), 4, false);
    let ws = weapons
        .iter()
        .map(|w| in_play(&mut e, 0, w, Some(a)))
        .collect();
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    (e, a, x, ws)
}

/// Gas-Powered Chainsaw: "3 damage each strike. This weapon is only usable
/// once each combat." A second copy allows a second use [ANK 20230316]:
/// round 1 with the first, round 2 (pressed) the second offered, the first
/// not. A melee weapon by its type line: Meat Cleaver's strength+1 at 2.
#[test]
fn gas_powered_chainsaw_strikes_once_each_combat_per_copy() {
    let (mut e, a, x, ws) = armed_bleed(&["Gas-Powered Chainsaw", "Gas-Powered Chainsaw"]);
    e.state.minions[a.index()].gained_levels = vec!["cel"];
    hand(&mut e, 0, &["Flash"]);
    pick_when(&mut e, use_at(ws[0], 0, None, Target::None));
    loop {
        let req = request(&mut e);
        if req.window == WindowLabel::CombatPress && req.options.contains(&play(0, 0, a)) {
            e.choose(index_of(&req, play(0, 0, a)));
            break;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(blood(&e, x), 1, "3 damage");
    let req = offering(&mut e, use_at(ws[1], 0, None, Target::None));
    assert!(!req.options.contains(&use_at(ws[0], 0, None, Target::None)));
    assert!(e.state.melee_weapon(ws[0]));

    let (mut e, _, x, ws) = armed_bleed(&["Meat Cleaver"]);
    pick_when(&mut e, use_at(ws[0], 0, None, Target::None));
    request(&mut e);
    assert_eq!(blood(&e, x), 2, "strength 1 + 1");
}

/// Gas-Powered Chainsaw, "Melee weapon": its strike a melee weapon strike
/// for every reader. Nuriel ("He inflicts +1 damage with melee weapons")
/// deals 3 + 1; Armor of Vitality [FOR] ("if any of the damage was from the
/// opposing minion's melee weapon, that weapon is destroyed") burns it.
#[test]
fn gas_powered_chainsaw_strikes_as_a_melee_weapon() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Nuriel"), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let x = e.add_minion(SeatId(1), mid(2), 4, false);
    e.add_minion(SeatId(1), mid(3), 4, false);
    let saw = in_play(&mut e, 0, "Gas-Powered Chainsaw", Some(a));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, use_at(saw, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 0, "3 + Nuriel's 1");

    let (mut e, _, x, ws) = armed_bleed(&["Gas-Powered Chainsaw"]);
    e.state.minions[x.index()].gained_levels = vec!["for", "for"];
    hand(&mut e, 1, &["Armor of Vitality"]);
    pick_when(&mut e, use_at(ws[0], 0, None, Target::None));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[ws[0].index()].in_play, "destroyed");
    assert!(ash_has(&e, 0, "Gas-Powered Chainsaw"));
}

/// Rowan Ring ("Melee weapon") striking: Baal's Bloody Talons ("Only usable
/// before resolution of a melee weapon's strike") offered on its torpor
/// strike, a legal no-op (rules-digest D14).
#[test]
fn rowan_ring_strike_is_a_melee_weapon_strike() {
    let (mut e, a, _, ws) = armed_bleed(&["Rowan Ring"]);
    e.state.minions[a.index()].gained_levels = vec!["qui"];
    hand(&mut e, 0, &["Baal's Bloody Talons"]);
    pick_when(&mut e, use_at(ws[0], 0, None, Target::None));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.window, WindowLabel::CombatBeforeResolution);
}

/// Damage added to a strike dealing none adds nothing (group "Adding damage
/// to strikes" [RTR 19960221]): Baal's Bloody Talons [QUI]'s +1 aggravated
/// on Rowan Ring's torpor strike, the opposing vampire to torpor untouched.
#[test]
fn damage_added_to_rowan_rings_torpor_strike_adds_nothing() {
    let (mut e, a, x, ws) = armed_bleed(&["Rowan Ring"]);
    e.state.minions[a.index()].gained_levels = vec!["qui", "qui"];
    hand(&mut e, 0, &["Baal's Bloody Talons"]);
    pick_when(&mut e, use_at(ws[0], 0, None, Target::None));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Baal's Bloody Talons"), "played");
    assert!(e.state.minions[x.index()].in_torpor);
    assert_eq!(blood(&e, x), 4, "no damage added");
}

/// Grenade: "3R damage as a strike. If Grenade is used at close range, the
/// minion with this weapon takes 1 damage. Burn after use." Its bearer's
/// damage environmental, normal [LSJ 19970801]: 1 blood off, the opponent 3;
/// burned.
#[test]
fn grenade_hurts_its_bearer_at_close_range_and_burns() {
    let (mut e, a, x, ws) = armed_bleed(&["Grenade"]);
    pick_when(&mut e, use_at(ws[0], 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 1);
    assert_eq!(
        blood(&e, a),
        4 - 1 - 1,
        "the blocker's hand strike, its own 1"
    );
    assert!(ash_has(&e, 0, "Grenade"));
}

/// Grenade dodged: the strike resolves without effect on the dodger (rules-qa
/// 122.214), what it does to its own striker still happens: the bearer's 1
/// at close range, burned after use. X untouched.
#[test]
fn grenade_dodged_still_burns_and_hurts_its_bearer() {
    let (mut e, a, x, ws) = armed_bleed(&["Grenade"]);
    hand(&mut e, 1, &["Dodge"]);
    pick_when(&mut e, use_at(ws[0], 0, None, Target::None));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Dodge"), "dodged");
    assert_eq!(blood(&e, x), 4);
    assert_eq!(blood(&e, a), 4 - 1);
    assert!(ash_has(&e, 0, "Grenade"));
}

/// "Does not burn nor inflict damage if combat ends before it resolves."
/// [LSJ 20001127-2] [LSJ 20010806-1] [LSJ 19981006] The blocker's combat
/// ends strike (Majesty) first: the Grenade still on, no damage.
#[test]
fn grenade_is_kept_when_combat_ends_first() {
    let (mut e, a, x, ws) = armed_bleed(&["Grenade"]);
    e.state.minions[x.index()].gained_levels = vec!["pre"];
    hand(&mut e, 1, &["Majesty"]);
    pick_when(&mut e, use_at(ws[0], 0, None, Target::None));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        [blood(&e, a), blood(&e, x)],
        [4, 4 - 1],
        "Majesty's cost only"
    );
    assert!(e.state.cards[ws[0].index()].in_play);
}

/// Hawg: "an optional press each combat. A minion may have only 1 vehicle":
/// a press offered; a Sport Bike not equipped by its bearer.
#[test]
fn hawg_gives_a_press_each_combat_and_is_one_vehicle() {
    let (mut e, a, _, _) = armed_bleed(&["Hawg"]);
    let req = offering(&mut e, Choice::UsePress);
    assert_eq!(req.seat, SeatId(0));
    let _ = a;
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    in_play(&mut e, 0, "Hawg", Some(a));
    hand(&mut e, 0, &["Sport Bike"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, a)));
}

/// Resplendent Protector: "The minion with this retainer may prevent 1
/// damage each combat": the blocker's hand strike prevented once.
#[test]
fn resplendent_protector_prevents_one_damage_each_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let x = e.add_minion(SeatId(1), mid(2), 4, false);
    e.add_minion(SeatId(1), mid(3), 4, false);
    let r = in_play(&mut e, 0, "Resplendent Protector", Some(a));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, use_at(r, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4);
}

/// The Knights: "Unique mortal with 2 life. 2 strength"; The Slashers: "3
/// life. 0 strength … may strike for 1R damage": recruited bodies; the
/// Slashers' 1R strike lands 1.
#[test]
fn round_34_vanilla_allies_enter_with_their_body() {
    let mut e = game(2);
    let k = e.add_ally(SeatId(0), find_library("Knights, The"));
    let s = e.add_ally(SeatId(0), find_library("Slashers, The"));
    assert_eq!(blood(&e, k), 2);
    assert_eq!(blood(&e, s), 3);
    let x = e.add_minion(SeatId(1), mid(2), 4, false);
    e.add_minion(SeatId(1), mid(3), 4, false);
    let card = body_of(&e, s);
    pick(&mut e, bleed(s));
    pick_when(&mut e, block(x));
    pick_when(&mut e, use_at(card, 0, None, Target::None));
    request(&mut e);
    assert_eq!(blood(&e, x), 3);
}

// ---- Marijava Thuggee ----------------------------------------------------------------

/// Marijava Thuggee: "Requires a ready Black Hand vampire. The thuggee can
/// play cards requiring basic Obfuscate as a vampire." A non-Black Hand
/// vampire not offered the recruit; recruited, he plays a basic [obf]
/// stealth card, never its [OBF] mode, though another grant of the same
/// Discipline … [ANK 20180111]: basic only.
#[test]
fn marijava_thuggee_needs_a_black_hand_recruiter_and_plays_obfuscate() {
    let mut e = game(2);
    let b = black_hand(&mut e, 0, 0, &[]);
    let o = e.add_minion(
        SeatId(0),
        textless(|c| !c.black_hand && c.capacity >= 4 && no_stealth(c), 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), mid(2), 4, false);
    hand(&mut e, 0, &["Marijava Thuggee"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, o)));
    assert!(req.options.contains(&play(0, 0, b)));
    let t = e.add_ally(SeatId(0), find_library("Marijava Thuggee"));
    let y = e.add_minion(SeatId(1), mid(3), 4, false);
    hand(&mut e, 0, &["Lost in Crowds"]);
    e.state.minions[t.index()].locked = false;
    let card = body_of(&e, t);
    pick(&mut e, use_at(card, 0, Some(t), Target::Minion(y)));
    pick_when(&mut e, block(y));
    let req = request(&mut e);
    assert!(req.options.contains(&play(1, 0, t)), "basic [obf]");
    assert!(!req.options.contains(&play(1, 1, t)), "never [OBF]");
}

/// "He can inflict 1 damage on a ready vampire as a Ⓓ action": seat 1's
/// ready vampire loses 1; a torpid one is no target.
#[test]
fn marijava_thuggee_wounds_a_ready_vampire_as_a_directed_action() {
    let mut e = game(2);
    black_hand(&mut e, 0, 0, &[]);
    let t = e.add_ally(SeatId(0), find_library("Marijava Thuggee"));
    let x = e.add_minion(SeatId(1), mid(2), 4, false);
    let z = e.add_minion(SeatId(1), mid(3), 2, true);
    let card = body_of(&e, t);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&use_at(card, 0, Some(t), Target::Minion(z))));
    e.choose(index_of(&req, use_at(card, 0, Some(t), Target::Minion(x))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 3);
}

/// "During your unlock phase, if the number of thuggees you control is
/// greater than the number of Black Hand vampires you control, lock all the
/// thuggees you control": two thuggees, one Black Hand vampire (torpid,
/// counted): both locked; one each: neither.
#[test]
fn marijava_thuggees_lock_when_they_outnumber_black_hand() {
    for two in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(0), mid(0), 4, false);
        let b = black_hand(&mut e, 0, 0, &[]);
        e.state.minions[b.index()].in_torpor = true;
        let t1 = e.add_ally(SeatId(0), find_library("Marijava Thuggee"));
        let t2 = two.then(|| e.add_ally(SeatId(0), find_library("Marijava Thuggee")));
        e.add_minion(SeatId(1), mid(2), 4, false);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(locked(&e, t1), two, "two {two}");
        if let Some(t2) = t2 {
            assert!(locked(&e, t2));
        }
    }
}

/// Marijava Thuggee with Leech (basic Potence): "Can be used to meet one
/// (but not more than one) of the normal Discipline requirements of a
/// multi-Discipline card" [LSJ 20011217] [LSJ 20090508]; "multiple similar
/// effects … cannot be used to meet the requirements of a multi-discipline
/// card" [LSJ 20020218]: Shadow Boxing ([obf][pot]) not offered, a basic
/// [obf] card is.
#[test]
fn marijava_thuggee_with_leech_cannot_play_shadow_boxing() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let t = e.add_ally(SeatId(1), find_library("Marijava Thuggee"));
    // A Black Hand vampire: the thuggee not locked in the unlock phase.
    black_hand(&mut e, 1, 0, &[]);
    hand(&mut e, 1, &["Leech", "Shadow Boxing", "Undead Strength"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, play_at(0, 0, Some(t), Target::Minion(a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(t));
    let mut potence = false;
    while e.combat().is_some() || !potence {
        let req = request(&mut e);
        assert!(
            !req.options
                .iter()
                .any(|p| matches!(p, Choice::Play { card: 0, .. })),
            "{:?}",
            req.options
        );
        if req.options.contains(&play(1, 0, t)) {
            potence = true;
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(potence, "Leech's basic Potence: a [pot] strike offered");
}

// ---- Inscription, Vial of Elder Vitae ----------------------------------------------

/// Seat 0's `a` (Blood Sorcery) equips its other vampire `b` with Inscription,
/// Blood Fury (1 blood) from hand inscribed: `a` pays it.
fn inscribed() -> (Engine, MinionId, MinionId, CardId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    let b = e.add_minion(SeatId(0), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    e.add_minion(SeatId(1), mid(3), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tha"];
    hand(&mut e, 0, &["Inscription", "Blood Fury"]);
    pick(&mut e, play_at(0, 0, Some(a), Target::Minion(b)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let ins = on(&e, "Inscription", b)[0];
    (e, a, b, ins)
}

/// Inscription: "Put this equipment on a ready vampire and put a card that
/// requires Blood Sorcery from your hand on it (…; this acting vampire pays
/// the cost of that card)." The equipper needs only [tha], the receiver none
/// [LSJ 20031116]; on another vampire still an equip action [ANK 20230408]
/// [LSJ 20050313]: the card itself on `b`, Blood Fury stored, its 1 blood
/// paid by `a`.
#[test]
fn inscription_stores_a_blood_sorcery_card_paid_by_the_actor() {
    let (e, a, b, ins) = inscribed();
    assert!(!e.state.cards[ins.index()].effect_only);
    assert!(!ash_has(&e, 0, "Inscription"));
    assert!(e.state.cards[ins.index()]
        .stored
        .contains(&find_library("Blood Fury")));
    assert_eq!(blood(&e, a), 3, "the acting vampire paid");
    assert_eq!(blood(&e, b), 4);
    let a_note = e.state.minions[a.index()].locked;
    assert!(a_note, "the equip action taken");
}

/// "The vampire with this equipment can burn this equipment to use the
/// basic Blood Sorcery ability of that card." `b`, no Blood Sorcery, in
/// combat at close range: Blood Fury's [tha] used from the Inscription, free,
/// its [THA] not offered; the blocker takes 1 + 1; Inscription and the card
/// in the ash heap. Only when the card could be played [PIB 20121014]; the
/// bearer meets every other requirement [ANK 20230822] [LSJ 20031116]
/// [ANK 20230816-2].
#[test]
fn inscription_bearer_uses_the_basic_ability_once() {
    let (mut e, _, b, ins) = inscribed();
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let x = MinionId(2);
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(x));
    let hand_len = e.state.seats[0].hand.len() as u8;
    let fury = |mode| play(hand_len, mode, b);
    let req = offering(&mut e, fury(0));
    assert!(!req.options.contains(&fury(1)), "the basic ability only");
    e.choose(index_of(&req, fury(0)));
    request(&mut e);
    let req_blood = blood(&e, b);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2);
    assert!(req_blood >= 3, "no cost paid");
    assert!(!e.state.cards[ins.index()].in_play);
    assert!(ash_has(&e, 0, "Inscription"));
    assert!(ash_has(&e, 0, "Blood Fury"));
}

/// "When burned, the card put on it is not considered to be played normally
/// (if it is an action modifier or reaction the vampire can still play the
/// same card themselves)" [ANK 20180627-1]: an inscribed Mirror Walk used for
/// stealth, by its acting bearer [PIB 20121014], then the hand's Mirror
/// Walk still offered the same action; "Do
/// not replace" ignored as it was stored and used [ANK 20180627-1]: the hand
/// drawn up.
#[test]
fn inscription_use_is_not_a_play() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let x = e.add_minion(SeatId(1), mid(2), 4, false);
    e.add_minion(SeatId(1), mid(3), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tha"];
    hand(&mut e, 0, &["Inscription", "Mirror Walk", "Mirror Walk"]);
    e.state.seats[0].library = vec![find_library("Soak"); 6].into();
    pick(&mut e, play_at(0, 0, Some(a), Target::Minion(a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let ins = on(&e, "Inscription", a)[0];
    assert!(e.state.cards[ins.index()]
        .stored
        .contains(&find_library("Mirror Walk")));
    assert_eq!(
        e.state.seats[0].hand.len(),
        7,
        "drawn up: no delay for the stored card"
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // +1 intercept: one Mirror Walk leaves the stealth still needed.
    in_play(&mut e, 1, "Sport Bike", Some(x));
    let hand_len = e.state.seats[0].hand.len() as u8;
    let walk_hand = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Mirror Walk"))
        .expect("in hand") as u8;
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(hand_len, 0, a));
    let req = offering(&mut e, play(walk_hand, 0, a));
    assert_eq!(req.seat, SeatId(0));
    assert!(!e.state.cards[ins.index()].in_play, "burned as it is used");
}

/// Inscription's use is no card play: [LSJ 20031203] (an inscribed card
/// against Watenda) "The effect cannot be canceled by effects that cancel
/// card plays. It is not a card play." (advanced rules §1.8.2). Blood Fury
/// used from the Inscription: seat 1's Direct Intervention ("Cancel a
/// minion card as it is played") never offered; from hand, it is.
#[test]
fn inscription_use_is_not_cancelable_as_played() {
    for inscribed_use in [true, false] {
        let (mut e, _, b, _) = inscribed();
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        hand(&mut e, 1, &["Direct Intervention"]);
        let cancel = e.state.seats[1].hand.len() as u8 - 1;
        let fury = if inscribed_use {
            play(e.state.seats[0].hand.len() as u8, 0, b)
        } else {
            e.state.minions[b.index()].gained_levels = vec!["tha"];
            hand(&mut e, 0, &["Blood Fury"]);
            play(e.state.seats[0].hand.len() as u8 - 1, 0, b)
        };
        let x = MinionId(2);
        pick(&mut e, bleed(b));
        pick_when(&mut e, block(x));
        pick_when(&mut e, fury);
        let mut offered = false;
        loop {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            offered |= req.seat == SeatId(1)
                && req
                    .options
                    .iter()
                    .any(|o| matches!(o, Choice::Play { card, .. } if *card == cancel));
            e.choose(fallback(&req));
        }
        assert_eq!(offered, !inscribed_use, "inscribed: {inscribed_use}");
    }
}

/// Vial of Elder Vitae: "burn this card to gain 1 level of any one
/// Discipline until your next unlock phase. The vampire cannot choose a
/// Discipline he or she already has at the superior level." A level gained
/// (Temporis; no capacity with it); Fortitude, superior, not offered; gone after its
/// controller's next unlock phase.
#[test]
fn vial_of_elder_vitae_lends_a_level_until_the_next_unlock() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["for", "for"];
    let vial = in_play(&mut e, 0, "Vial of Elder Vitae", Some(a));
    let cap = e.state.capacity_of(a);
    let before = e.state.discipline_level(a, "tem");
    pick(&mut e, use_at(vial, 0, None, Target::None));
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::ChooseDiscipline("for")));
    e.choose(index_of(&req, Choice::ChooseDiscipline("tem")));
    request(&mut e);
    assert_eq!(e.state.discipline_level(a, "tem"), before + 1);
    assert_eq!(e.state.capacity_of(a), cap);
    assert!(ash_has(&e, 0, "Vial of Elder Vitae"));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.discipline_level(a, "tem"),
        before + 1,
        "until its controller's unlock"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.discipline_level(a, "tem"), before);
}

/// "The option to place the card on another minion can only be used when
/// playing the card normally; for any other action or put into play effect,
/// the card is placed on the playing minion." [LSJ 20090824] [LSJ 20051121]
/// Magic of the Smith finds Inscription: on its actor, Blood Fury still
/// stored from hand and paid (LSJ 20091206).
#[test]
fn inscription_found_by_a_search_stays_on_the_actor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tha"];
    hand(&mut e, 0, &["Magic of the Smith", "Blood Fury"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Inscription"), find_library("Soak")].into();
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::FindCard(find_library("Inscription")));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let ins = on(&e, "Inscription", a);
    assert_eq!(ins.len(), 1);
    assert!(e.state.cards[ins[0].index()]
        .stored
        .contains(&find_library("Blood Fury")));
    assert_eq!(
        blood(&e, a),
        4 - 1 - 1,
        "Magic of the Smith's blood, Blood Fury's"
    );
}

/// An Inscription on another Methuselah's vampire is used by that vampire
/// (its controller's), the card on it its owner's: an inscribed Inscription
/// used to equip enters play its owner's, seat 0's; its equip blocked, it
/// burns to seat 0's ash heap (fuzz round 34a seeds 91, 218: it was the
/// user's, a card counted twice).
#[test]
fn an_inscribed_card_used_by_another_methuselah_stays_its_owners() {
    for blocked in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), mid(0), 4, false);
        let b = e.add_minion(SeatId(0), mid(1), 4, false);
        let x = e.add_minion(SeatId(1), mid(2), 4, false);
        let y = e.add_minion(SeatId(1), mid(3), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["tha"];
        hand(&mut e, 0, &["Inscription", "Inscription"]);
        pick(&mut e, play_at(0, 0, Some(a), Target::Minion(x)));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let first = on(&e, "Inscription", x)[0];
        assert_eq!(e.state.cards[first.index()].owner, SeatId(0));
        hand(&mut e, 1, &["Blood Fury", "Blood Fury"]);
        let stored = e.state.seats[1].hand.len() as u8;
        if blocked {
            in_play(&mut e, 0, "Sport Bike", Some(b));
        }
        pick(&mut e, play_at(stored, 0, Some(x), Target::Minion(y)));
        if blocked {
            assert!(offered_before(
                &mut e,
                block(b),
                SeatId(1),
                WindowLabel::MinionPhase
            ));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let now = on(&e, "Inscription", y);
        let inscriptions = |seat: usize| {
            e.state.seats[seat]
                .ash
                .iter()
                .filter(|&&c| c == find_library("Inscription"))
                .count()
        };
        assert_eq!(inscriptions(1), 0, "blocked: {blocked}");
        if blocked {
            assert!(now.is_empty());
            assert_eq!(inscriptions(0), 2);
        } else {
            assert_eq!(now.len(), 1, "{now:?}");
            assert_eq!(e.state.cards[now[0].index()].owner, SeatId(0));
            assert_eq!(inscriptions(0), 1, "the first burned as it was used");
        }
    }
}

/// A vampire's printed ability `ai` offered in `req`, by any actor.
fn vamp_use(req: &zvtes::decision::DecisionRequest, m: MinionId, ai: u8) -> Option<Choice> {
    req.options.iter().copied().find(|o| {
        matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), ability, .. }
            if *v == m && *ability == ai)
    })
}

/// Pass every window until vampire `m`'s ability `ai` is offered, and take it.
fn take_vamp_use(e: &mut Engine, m: MinionId, ai: u8) {
    for _ in 0..200 {
        let req = request(e);
        if let Some(c) = vamp_use(&req, m, ai) {
            e.choose(index_of(&req, c));
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("never offered");
}

/// Akram blocks seat 0's bleed; both survive the combat. "Once each turn
/// after completing combat, if Akram and the opposing minion are still
/// ready, Akram may burn 1 blood to begin another combat": 1 blood, a new
/// combat with the same pair; not again that turn [ANK 20180719-4].
#[test]
fn akram_begins_another_combat_after_his_own() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let k = e.add_minion(SeatId(1), find_card("Akram"), 6, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(k));
    take_vamp_use(&mut e, k, 0);
    assert_eq!(blood(&e, k), 6 - 1 - 1, "the first combat's 1, his cost 1");
    request(&mut e);
    // The second combat's hand strikes.
    assert_eq!([blood(&e, a), blood(&e, k)], [4 - 2, 6 - 3]);
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == k),
    );
}

/// "If his ability is used to start a new combat, effects that end combat
/// and then do something else after combat (include continuing the action
/// as if unblocked) are lost" [LSJ 19980109] [RTR 20020501]: Torrent [CEL]
/// ends the first combat; Akram's new combat: the bleed stays blocked.
#[test]
fn akram_loses_the_first_combats_continuation() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 6, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let k = e.add_minion(SeatId(1), find_card("Akram"), 6, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    hand(&mut e, 0, &["Torrent"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(k));
    pick_when(&mut e, play(0, 1, a));
    take_vamp_use(&mut e, k, 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10, "no continuation: blocked");
}

/// "His ability cannot be used to start a new combat if there is already a
/// pending combat queued." [RTR 20020501] Pocket Out of Time's combat queued
/// first: Akram's not offered. Nor with the opponent gone to torpor.
#[test]
fn akram_needs_no_queued_combat_and_a_ready_opponent() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 6, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let k = e.add_minion(SeatId(1), find_card("Akram"), 6, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tem", "tem"];
    hand(&mut e, 0, &["Pocket Out of Time"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(k));
    let pocket = offering(&mut e, play(0, 1, a));
    e.choose(index_of(&pocket, play(0, 1, a)));
    let req = request(&mut e);
    assert!(vamp_use(&req, k, 0).is_none(), "a combat queued");

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 1, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let k = e.add_minion(SeatId(1), find_card("Akram"), 6, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    // Strength 2: the 1-blood bleeder to torpor.
    e.state.minions[k.index()]
        .timed
        .push((Lasting::Strength(1), Expiry::Never, SeatId(1)));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(k));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == k),
    );
    assert!(e.state.minions[a.index()].in_torpor);
}

/// Amadeo: "+1 hunt": with no blood, his forced hunt gains 2. "The hunt
/// bonus applies to non-standard hunt actions, blood comes from the target
/// of the hunt." [RTR 20030519] [RTR 20180511] Abactor: 1 + 4 + 1.
#[test]
fn amadeo_adds_a_blood_to_every_hunt() {
    for abactor in [false, true] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card("Amadeo"), 0, false);
        e.add_minion(SeatId(0), mid(1), 4, false);
        e.add_minion(SeatId(1), find_card("Tara"), 4, false);
        if abactor {
            hand(&mut e, 0, &["Abactor"]);
            pick(&mut e, play(0, 0, m));
            expect(&mut e, SeatId(0), WindowLabel::BloodHuntPolling);
            assert_eq!(blood(&e, m), 1 + 4 + 1);
        } else {
            expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, m), 2);
        }
    }
}

/// Anatole: "you may look at the top 5 cards of your library and then
/// shuffle the top 5 cards of your library. +1 intercept." The bottom three
/// untouched and in order, the top five the same cards; once a master
/// phase; in torpor, never.
#[test]
fn anatole_looks_at_and_shuffles_the_top_five() {
    let names = [
        "Soak",
        "Deflection",
        "Celerity",
        "Fortitude",
        "Potence",
        "Dominate",
        "Presence",
        "Auspex",
    ];
    let mut e = game(2);
    let an = e.add_minion(
        SeatId(0),
        find_card("Anatole, Prophet of Gehenna"),
        4,
        false,
    );
    e.add_minion(SeatId(1), mid(2), 4, false);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = names
        .iter()
        .map(|n| find_library(n))
        .collect::<Vec<_>>()
        .into();
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    e.notes = Some(Vec::new());
    e.choose(index_of(&req, vamp_use(&req, an, 0).expect("offered")));
    let req = request(&mut e);
    assert!(vamp_use(&req, an, 0).is_none(), "once a master phase");
    let lib = e.state.seats[0].library.to_cards();
    let bottom: Vec<u16> = names[..3].iter().map(|n| find_library(n)).collect();
    assert_eq!(&lib[..3], &bottom[..]);
    let mut top = lib[3..].to_vec();
    let mut want: Vec<u16> = names[3..].iter().map(|n| find_library(n)).collect();
    top.sort_unstable();
    want.sort_unstable();
    assert_eq!(top, want);
    assert_eq!(noted_looks(&mut e)[0].cards.len(), 5);
    assert_eq!(
        e.state.printed_mod(an, Stat::Intercept, Default::default()),
        1
    );

    let mut e = game(2);
    let an = e.add_minion(SeatId(0), find_card("Anatole, Prophet of Gehenna"), 4, true);
    e.add_minion(SeatId(0), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    e.state.seats[0].library = vec![find_library("Soak"); 6].into();
    // A master to play: the master phase a real decision.
    hand(&mut e, 0, &["Path of Evil Revelations"]);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(vamp_use(&req, an, 0).is_none());
}

/// Aristotle de Laurent: "While Aristotle is ready, each Methuselah gets +1
/// hand size. +1 stealth": both seats at 8; in torpor, 7.
#[test]
fn aristotle_widens_every_hand_while_ready() {
    for torpor in [false, true] {
        let mut e = game(2);
        let ar = e.add_minion(SeatId(0), find_card("Aristotle de Laurent"), 4, torpor);
        e.add_minion(SeatId(1), mid(2), 4, false);
        let n = if torpor { 7 } else { 8 };
        assert_eq!(
            [e.state.hand_size(SeatId(0)), e.state.hand_size(SeatId(1))],
            [n, n]
        );
        assert_eq!(
            e.state.printed_mod(ar, Stat::Stealth, Default::default()),
            1
        );
    }
}

/// Bindusara: "search your library for an Arcane Library, Elder Library or
/// Fragment of the Book of Nod and put that card in play. (Pay cost as
/// normal.)" Its requirements ignored [ANK 20200810]: Arcane Library with
/// no Tremere, 2 pool paid, the card itself in play; not twice a turn even
/// for another card [ANK 20200810].
#[test]
fn bindusara_puts_a_library_in_play_paying_its_cost() {
    let mut e = game(2);
    let b = e.add_minion(
        SeatId(0),
        find_card("Bindusara, Historian of the Kindred"),
        6,
        false,
    );
    e.add_minion(SeatId(0), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = [
        "Deflection",
        "Arcane Library",
        "Elder Library",
        "Fragment of the Book of Nod",
        "Deflection",
    ]
    .iter()
    .map(|n| find_library(n))
    .collect::<Vec<_>>()
    .into();
    take_vamp_use(&mut e, b, 0);
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    for n in [
        "Arcane Library",
        "Elder Library",
        "Fragment of the Book of Nod",
    ] {
        assert!(
            req.options.contains(&Choice::FindCard(find_library(n))),
            "{n}"
        );
    }
    e.choose(index_of(
        &req,
        Choice::FindCard(find_library("Arcane Library")),
    ));
    let req = request(&mut e);
    assert_eq!(pool(&e)[0], 10 - 2);
    assert!(card_in_play(&e, "Arcane Library").is_some());
    assert!(!ash_has(&e, 0, "Arcane Library"));
    assert!(!e.state.seats[0]
        .library
        .contains(&find_library("Arcane Library")));
    e.state.minions[b.index()].locked = false;
    let _ = req;
    pick(&mut e, action(MinionId(1), ActionKind::Hunt));
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == b),
    );
}

/// Elder Library through Bindusara: 1 pool, its +1 hand size on; with one in
/// play already, not offered (rules-qa 7.1); at 1 pool, Arcane Library
/// (2) not offered.
#[test]
fn bindusaras_elder_library_widens_the_hand() {
    let mut e = game(2);
    let b = e.add_minion(
        SeatId(0),
        find_card("Bindusara, Historian of the Kindred"),
        6,
        false,
    );
    e.add_minion(SeatId(0), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![
        find_library("Arcane Library"),
        find_library("Elder Library"),
    ]
    .into();
    take_vamp_use(&mut e, b, 0);
    pick_when(&mut e, Choice::FindCard(find_library("Elder Library")));
    request(&mut e);
    assert_eq!(pool(&e)[0], 9);
    assert_eq!(e.state.hand_size(SeatId(0)), 8);

    let mut e = game(2);
    let b = e.add_minion(
        SeatId(0),
        find_card("Bindusara, Historian of the Kindred"),
        6,
        false,
    );
    e.add_minion(SeatId(0), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    in_play(&mut e, 0, "Elder Library", None);
    e.state.seats[0].pool = 1;
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![
        find_library("Arcane Library"),
        find_library("Elder Library"),
        find_library("Fragment of the Book of Nod"),
    ]
    .into();
    take_vamp_use(&mut e, b, 0);
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            Choice::FindCard(find_library("Fragment of the Book of Nod"))
        ]
    );
}

/// Cailean: "Each round of combat, before range is determined, Cailean can
/// set the range for that round." Long set: no other setter that round,
/// other pre-range effects still played [RTR 19970630] [ANK 20180720].
#[test]
fn cailean_sets_each_rounds_range() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), mid(0), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let c = e.add_minion(SeatId(1), find_card("Cailean"), 6, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(c));
    take_vamp_use(&mut e, c, 1);
    let cb = e.combat().expect("combat");
    assert_eq!(cb.range, zvtes::combat::Range::Long);
    assert!(cb.range_set, "the Determine Range step skipped");
}

/// The round's plain text: Bryan Van Duesen's +1 bleed, Calebos's +1
/// strength (a bishop); the text-only ones resolve to their crypt card
/// with no definition (Ana Rita Montaña, Artemis, Boy Toy, Brachah,
/// Brooke).
#[test]
fn the_round_34_vampires_ana_rita_to_calebos_carry_their_text() {
    let mut e = game(2);
    let br = e.add_minion(SeatId(0), find_card("Bryan Van Duesen"), 4, false);
    let ca = e.add_minion(SeatId(0), find_card("Calebos"), 4, false);
    assert_eq!(e.state.printed_mod(br, Stat::Bleed, Default::default()), 1);
    assert_eq!(
        e.state.printed_mod(ca, Stat::Strength, Default::default()),
        1
    );
    assert_eq!(e.state.title_of(ca), Title::Bishop);
    for (name, sect) in [
        ("Ana Rita Montaña", Sect::Sabbat),
        ("Artemis", Sect::Sabbat),
        ("Boy Toy", Sect::Sabbat),
        ("Brachah", Sect::Camarilla),
        ("Brooke", Sect::Sabbat),
    ] {
        let card = find_card(name);
        assert!(zvtes::cards::vampire(card).is_none(), "{name}");
        assert_eq!(CRYPT[card as usize].sect, Some(sect), "{name}");
        assert_eq!(CRYPT[card as usize].title, Title::None, "{name}");
    }
}

/// Pass every window until one offers a choice matching `want`; take it.
fn pick_matching(e: &mut Engine, want: impl Fn(&Choice) -> bool) {
    for _ in 0..200 {
        let req = request(e);
        if let Some(i) = req.options.iter().position(&want) {
            e.choose(i);
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("never offered");
}

/// Seat 0's Damaskenos and its own ready Ministry (3 blood); seat 1's ready
/// Ministry (4) and torpid one (3).
fn damaskenos_table() -> (Engine, MinionId, [MinionId; 3]) {
    let mut e = game(2);
    let d = e.add_minion(
        SeatId(0),
        find_card("Damaskenos, Herald of Leandro"),
        6,
        false,
    );
    let own = e.add_minion(SeatId(0), plain("Ministry", 0), 3, false);
    let m1 = e.add_minion(SeatId(1), plain("Ministry", 1), 4, false);
    let m2 = e.add_minion(SeatId(1), plain("Ministry", 2), 3, true);
    (e, d, [own, m1, m2])
}

/// Damaskenos: "If he becomes the Malkavian Justicar, each ready Ministry
/// burns 2 blood": Malkavian Justicar passed on him, uncontested: every
/// Methuselah's ready Ministry −2, the torpid one not.
#[test]
fn damaskenos_becoming_malkavian_justicar_burns_two_from_each_ready_ministry() {
    let (mut e, d, [own, m1, m2]) = damaskenos_table();
    hand(&mut e, 0, &["Malkavian Justicar"]);
    pick(&mut e, play(0, 0, d));
    pick_matching(&mut e, |o| {
        *o == Choice::CastCardVote { favor: true } || *o == votes(d, true)
    });
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.title_of(d), Title::Justicar("Malkavian"));
    assert_eq!([blood(&e, own), blood(&e, m1), blood(&e, m2)], [1, 2, 3]);
}

/// Damaskenos "becomes" a title holder as the title becomes effective
/// (rules-qa 126.61, 126.8; 76.3, rules-digest §3.15): a title gained
/// contested makes no holder: seat 1's vampire claims the Malkavian Justicar
/// title; Damaskenos's is contested, nothing burned; the rival yields in its
/// unlock phase: Damaskenos becomes the justicar, each ready Ministry −2.
/// (Cairo by the same rule: `HostGainsTitle` fires as the contest resolves.)
#[test]
fn damaskenos_burns_the_ministry_only_once_he_holds_the_title() {
    let (mut e, d, [own, m1, _]) = damaskenos_table();
    let h = e.add_minion(SeatId(1), mid(3), 4, false);
    e.state.minions[h.index()].granted_title = Some(Title::Justicar("Malkavian"));
    hand(&mut e, 0, &["Malkavian Justicar"]);
    pick(&mut e, play(0, 0, d));
    pick_matching(&mut e, |o| {
        *o == Choice::CastCardVote { favor: true } || *o == votes(d, true)
    });
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[d.index()].title_contested);
    assert_eq!(
        [blood(&e, own), blood(&e, m1)],
        [3, 4],
        "contested: no burn"
    );
    pick_when(&mut e, Choice::YieldContest(zvtes::game::Contest::Title(h)));
    request(&mut e);
    assert_eq!(
        [blood(&e, own), blood(&e, m1)],
        [3, 4],
        "not before it returns"
    );
    // Uncontested "during your next unlock phase": he holds it now.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[d.index()].title_contested);
    assert_eq!([blood(&e, own), blood(&e, m1)], [1, 2]);
}

/// Christanius Lionel: "While Christanius is ready, your hand size is
/// increased by 1": 8; in torpor, 7. Constanza Vinti: +2 bleed against a
/// Methuselah with a ready Ventrue (3 pool), not when it is torpid (1);
/// Elisabetta Romano: +1 against a ready Ministry.
#[test]
fn round_34_b_printed_hand_size_and_bleeds() {
    for torpor in [false, true] {
        let mut e = game(2);
        e.add_minion(
            SeatId(0),
            find_card("Christanius Lionel, The Mad Chronicler"),
            4,
            torpor,
        );
        assert_eq!(e.state.hand_size(SeatId(0)), if torpor { 7 } else { 8 });
    }
    for (name, clan, more) in [
        ("Constanza Vinti", "Ventrue", 2),
        ("Elisabetta Romano", "Ministry", 1),
    ] {
        for torpor in [false, true] {
            let mut e = game(2);
            let c = e.add_minion(SeatId(0), find_card(name), 6, false);
            e.add_minion(SeatId(0), mid(1), 4, false);
            e.add_minion(SeatId(1), plain(clan, 0), 4, torpor);
            pick(&mut e, bleed(c));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(
                pool(&e)[1],
                10 - 1 - if torpor { 0 } else { more },
                "{name} {torpor}"
            );
        }
    }
}

/// Doctor Streck: "After Doctor Streck successfully bleeds, the target
/// Methuselah discards 1 card at random": seat 1's ash heap +1 from its
/// hand, the hand drawn back up.
#[test]
fn doctor_streck_bleeds_a_random_discard() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Doctor Streck"), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    fill_hand(&mut e, 1);
    e.state.seats[1].library = vec![find_library("Soak"); 6].into();
    pick(&mut e, bleed(s));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].ash.len(), 1);
    assert!(ash_has(&e, 1, "Form of Mist"));
    assert_eq!(e.state.seats[1].hand.len(), 7);
    assert_eq!(e.state.seats[1].library.len(), 5);
}

/// Dominique (ADV): "During your unlock phase, you may burn a location you
/// control to gain X pool, where X is the cost of that location." Club
/// Zombie (4): +4; with Therbold Realty in play, a 4-pool location +3 (LSJ
/// 20091124); once a phase.
#[test]
fn dominique_burns_a_location_for_its_cost_in_pool() {
    let dom = CRYPT
        .iter()
        .position(|c| c.name == "Dominique" && c.adv)
        .expect("Dominique (ADV)") as u16;
    for therbold in [false, true] {
        let mut e = game(2);
        let d = e.add_minion(SeatId(0), dom, 4, false);
        e.add_minion(SeatId(1), mid(2), 4, false);
        let club = in_play(&mut e, 0, "Club Zombie", None);
        let other = in_play(&mut e, 0, "Kingston Penitentiary, Ontario", None);
        if therbold {
            in_play(&mut e, 0, "Therbold Realty", None);
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
        let burn = |c| Choice::UseAbility {
            source: AbilitySource::Vampire(d),
            ability: 0,
            actor: None,
            target: Target::Card(c),
        };
        let before = pool(&e)[0];
        e.choose(index_of(&req, burn(club)));
        let req = request(&mut e);
        assert_eq!(
            pool(&e)[0],
            before + if therbold { 3 } else { 4 },
            "therbold {therbold}"
        );
        assert!(ash_has(&e, 0, "Club Zombie"));
        assert!(!req.options.contains(&burn(other)), "once a phase");
    }
}

/// Dominique (ADV) and a blood-cost location: Therbold Realty's "Locations
/// cost you 1 less blood or pool" reduces Palatial Estate (2 blood) too
/// [LSJ 20080107] ("eg. Palatial Estate"), read now [LSJ 20091124]: +1.
#[test]
fn dominique_burns_palatial_estate_under_therbold_for_one() {
    let dom = CRYPT
        .iter()
        .position(|c| c.name == "Dominique" && c.adv)
        .expect("Dominique (ADV)") as u16;
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), dom, 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    let estate = in_play(&mut e, 0, "Palatial Estate", Some(d));
    in_play(&mut e, 0, "Therbold Realty", None);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let before = pool(&e)[0];
    e.choose(index_of(
        &req,
        Choice::UseAbility {
            source: AbilitySource::Vampire(d),
            ability: 0,
            actor: None,
            target: Target::Card(estate),
        },
    ));
    request(&mut e);
    assert_eq!(pool(&e)[0], before + 2 - 1);
    assert!(ash_has(&e, 0, "Palatial Estate"));
}

/// Bulscu with Therbold Realty in another Methuselah's play: "costing 3 or
/// less" reads the location's standing reduction (LSJ 20091124): Club Zombie
/// (4, now 3) offered — a fix (rules-qa 116.102 read the printed cost).
#[test]
fn bulscu_reads_therbold_realtys_reduction_of_a_location() {
    for therbold in [false, true] {
        let mut e = game(2);
        let base = CRYPT
            .iter()
            .position(|c| c.name == "Bulscu" && !c.adv)
            .expect("Bulscu") as u16;
        let b = e.add_minion(SeatId(0), base, 4, false);
        e.add_minion(SeatId(0), mid(1), 4, false);
        e.add_minion(SeatId(1), mid(2), 4, false);
        let club = in_play(&mut e, 1, "Club Zombie", None);
        if therbold {
            in_play(&mut e, 1, "Therbold Realty", None);
        }
        e.state.seats[0].has_edge = true;
        let req = request(&mut e);
        let at = Choice::UseAbility {
            source: AbilitySource::Vampire(b),
            ability: 0,
            actor: Some(b),
            target: Target::Card(club),
        };
        assert_eq!(req.options.contains(&at), therbold, "therbold {therbold}");
    }
}

/// Donatien: "As a Ⓓ action, Donatien may bleed any Methuselah who has
/// attempted an unsuccessful bleeding action since your last turn. +1
/// bleed." Seat 2's blocked bleed: seat 2 offered, seat 3 not; seat 2 loses
/// 2 (1 + 1).
#[test]
fn donatien_bleeds_a_methuselah_whose_bleed_was_blocked() {
    let mut e = game(4);
    let d = e.add_minion(SeatId(0), find_card("Donatien"), 6, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    e.add_minion(SeatId(1), mid(2), 4, false);
    let c = e.add_minion(SeatId(2), mid(3), 4, false);
    let blocker = e.add_minion(SeatId(3), mid(4), 4, false);
    e.add_minion(SeatId(3), mid(5), 4, false);
    e.add_minion(SeatId(2), mid(6), 4, false);
    let at = |s: u8| Choice::UseAbility {
        source: AbilitySource::Vampire(d),
        ability: 0,
        actor: Some(d),
        target: Target::Seat(SeatId(s)),
    };
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    pick(&mut e, bleed(c));
    pick_when(&mut e, block(blocker));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&at(3)));
    e.choose(index_of(&req, at(2)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[2], 10 - 2);
}

/// Dónal O'Connor (G2): "Any vampire blocking Dónal burns 1 blood (before
/// combat begins, if any)." At once on the block [RTR 19960530]: the
/// blocker's 1, then the combat's hand strike.
#[test]
fn donal_g2_burns_a_blocker_before_combat() {
    let donal = CRYPT
        .iter()
        .position(|c| c.name == "Dónal O'Connor" && c.group == 2)
        .expect("Dónal G2") as u16;
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), donal, 6, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let x = e.add_minion(SeatId(1), mid(2), 4, false);
    e.add_minion(SeatId(1), mid(3), 4, false);
    pick(&mut e, bleed(d));
    pick_when(&mut e, block(x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 1 - 1, "his burn, then the hand strike");
}

/// Dónal O'Connor (G2): [RTR 19960530] "The blocking vampire burns 1 blood
/// immediately upon successfully blocking Donal, regardless of whether
/// combat begins": an older blocker ends the action with Obedience [dom]
/// (no combat), and has still burned 1.
#[test]
fn donal_g2_blocker_burns_without_combat() {
    let donal = CRYPT
        .iter()
        .position(|c| c.name == "Dónal O'Connor" && c.group == 2)
        .expect("Dónal G2") as u16;
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), donal, 6, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let cap = CRYPT[donal as usize].capacity;
    let x = e.add_minion(
        SeatId(1),
        fixture_vampire(|c| c.capacity > cap && no_stealth(c), 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), mid(3), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["dom"];
    hand(&mut e, 1, &["Obedience"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(d));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(noted_combats(&mut e).is_empty(), "no combat");
    assert!(ash_has(&e, 1, "Obedience"));
    assert_eq!(blood(&e, x), 4 - 1);
}

/// "An empty blocking vampire may still block and will try and fail to burn
/// a blood if the block is successful." [TOM 19951215-1]
#[test]
fn donal_g2_empty_blocker_still_blocks() {
    let donal = CRYPT
        .iter()
        .position(|c| c.name == "Dónal O'Connor" && c.group == 2)
        .expect("Dónal G2") as u16;
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), donal, 6, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let x = e.add_minion(SeatId(1), mid(2), 1, false);
    e.add_minion(SeatId(1), mid(3), 4, false);
    e.state.minions[x.index()].blood = 0;
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(d));
    pick_when(&mut e, block(x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!noted_combats(&mut e).is_empty(), "blocked");
    assert_eq!(pool(&e)[1], 10, "no bleed");
}

/// Etrius: "may steal up to 3 blood from a Tremere vampire as a +1 stealth
/// Ⓓ action. +2 bleed": 3 chosen at resolution from seat 1's 4-blood
/// Tremere; never himself.
#[test]
fn etrius_steals_up_to_three_from_a_tremere() {
    let mut e = game(2);
    let et = e.add_minion(SeatId(0), find_card("Etrius"), 4, false);
    e.add_minion(SeatId(0), mid(1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Tremere", 0), 4, false);
    let steal = |m| Choice::UseAbility {
        source: AbilitySource::Vampire(et),
        ability: 0,
        actor: Some(et),
        target: Target::Minion(m),
    };
    let req = request(&mut e);
    assert!(!req.options.contains(&steal(et)));
    e.choose(index_of(&req, steal(t)));
    pick_when(&mut e, set_x(3));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!([blood(&e, t), blood(&e, et)], [1, 7]);
    assert_eq!(e.state.printed_mod(et, Stat::Bleed, Default::default()), 2);
}

/// Text-only: Charice Fontaigne, Daliyah, Dani (sect only), The Colonel
/// (Black Hand, a trait beside the sect [LSJ 20070322] [ANK 20180807]) have
/// no definition.
#[test]
fn round_34_b_text_only_vampires_carry_their_traits() {
    for (name, sect, black_hand) in [
        ("Charice Fontaigne", Sect::Sabbat, false),
        ("Daliyah", Sect::Camarilla, false),
        ("Dani", Sect::Sabbat, false),
        ("Colonel, The", Sect::Sabbat, true),
    ] {
        let card = find_card(name);
        assert!(zvtes::cards::vampire(card).is_none(), "{name}");
        assert_eq!(CRYPT[card as usize].sect, Some(sect));
        assert_eq!(CRYPT[card as usize].black_hand, black_hand, "{name}");
    }
}
