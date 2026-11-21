//! Round 35 (`ADDED_BATCHES`): Bloodlines and Final Nights completed.
//! Windows with a single option are elided, so the scripts only stop where
//! a real decision exists.
#![allow(dead_code)]

use zvtes::cards::{AbilitySource, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_library, CardId, UncontrolledSlot};

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

/// The `nth` textless vampire of `clan` with a capacity in `caps`.
#[track_caller]
fn of_clan(clan: &str, caps: std::ops::RangeInclusive<u8>, nth: usize) -> u16 {
    textless(|c| c.clan == clan && caps.contains(&c.capacity), nth)
}

/// The `nth` textless vampire of `sect` with a capacity in `caps`.
#[track_caller]
fn of_sect(sect: Sect, caps: std::ops::RangeInclusive<u8>, nth: usize) -> u16 {
    textless(|c| c.sect == Some(sect) && caps.contains(&c.capacity), nth)
}

/// The crypt card `name` (not advanced).
#[track_caller]
fn find_card(name: &str) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && !c.adv)
        .unwrap_or_else(|| panic!("no {name}")) as u16
}

/// The card `name` entered play (itself, not a stand-in) on `host`, out of
/// every ash heap: its id.
#[track_caller]
fn placed_on(e: &Engine, name: &str, host: Option<MinionId>) -> CardId {
    let card = find_library(name);
    assert!(
        e.state.seats.iter().all(|s| !s.ash.contains(&card)),
        "{name} in an ash heap"
    );
    let i = e
        .state
        .cards
        .iter()
        .position(|c| c.card == card && c.in_play && !c.effect_only && c.host == host)
        .unwrap_or_else(|| panic!("{name} not in play on {host:?}"));
    CardId(i as u32)
}

fn ash_has(e: &Engine, seat: usize, name: &str) -> bool {
    e.state.seats[seat].ash.contains(&find_library(name))
}

/// The ability `ability` of the card in play `source`, used by `actor` on
/// `target`.
fn use_on(source: CardId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(source),
        ability,
        actor,
        target,
    }
}

/// A master card from hand slot `card` in `mode`, on `target`.
fn master_at(card: u8, mode: u8, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: None,
        target,
    }
}

/// A slot of `card` in `seat`'s uncontrolled region with `counters`.
fn uncontrolled(e: &mut Engine, seat: u8, card: u16, counters: u8) {
    e.state.seats[seat as usize]
        .uncontrolled
        .push(UncontrolledSlot {
            card,
            copy: Default::default(),
            counters,
            banished: None,
        });
}

/// The stealth of the action `choice` declares, read at its first window.
fn action_stealth(e: &mut Engine, choice: Choice) -> i32 {
    pick(e, choice);
    request(e);
    e.action().expect("an action").stealth(&e.state)
}

/// The Methuselah the action `choice` declares is directed at.
fn directed_at(e: &mut Engine, choice: Choice) -> Option<SeatId> {
    pick(e, choice);
    request(e);
    e.action().expect("an action").directed_seat(&e.state)
}

// ---- Master cards (master) --------------------------------------------------

/// Contract: "Choose an Banu Haqim and put this card on a ready minion. The
/// chosen Banu Haqim can enter combat with the attached minion as a +1
/// stealth Ⓓ action." Seat 0 puts it on seat 1's ready minion (a torpid
/// one not offered), choosing seat 0's Banu Haqim of any Methuselah's: the
/// card itself in play; the chosen's +1 stealth rush of the attached
/// minion.
#[test]
fn contract_chosen_banu_haqim_rushes_the_attached_minion() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), of_clan("Banu Haqim", 1..=11, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let m = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let torpid = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, true);
    let theirs = e.add_minion(SeatId(1), of_clan("Banu Haqim", 1..=11, 1), 4, false);
    hand(&mut e, 0, &["Contract"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&master_at(0, 0, Target::Minion(torpid))));
    pick(&mut e, master_at(0, 0, Target::Minion(m)));
    let req = request(&mut e);
    assert!(
        req.options.contains(&Choice::ChooseMinion(theirs)),
        "any Methuselah's"
    );
    pick(&mut e, Choice::ChooseMinion(b));
    request(&mut e);
    let c = placed_on(&e, "Contract", Some(m));
    assert_eq!(e.state.cards[c.index()].chosen, vec![b]);
    let rush = use_on(c, 0, Some(b), Target::Minion(m));
    offering(&mut e, rush);
    assert_eq!(action_stealth(&mut e, rush), 1);
}

/// Contract with no Banu Haqim in play to choose: not offered (it played
/// with no choice left the master phase stuck, found by the host sweep).
#[test]
fn contract_needs_a_banu_haqim_to_choose() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let m = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Contract"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(!req.options.contains(&master_at(0, 0, Target::Minion(m))));
}

/// Contract's action requires a Banu Haqim (LSJ 20080604): its chosen
/// turned to another clan is not offered it.
/// Ledger: [LSJ 20080604].
#[test]
fn contract_action_needs_the_chosen_to_be_a_banu_haqim() {
    for changed in [false, true] {
        let mut e = game(2);
        let b = e.add_minion(SeatId(0), of_clan("Banu Haqim", 1..=11, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let m = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let c = in_play(&mut e, 0, "Contract", Some(m));
        e.state.cards[c.index()].chosen = vec![b];
        if changed {
            e.state.minions[b.index()].clan_change = Some("Brujah");
        }
        let req = request(&mut e);
        let offered = req
            .options
            .contains(&use_on(c, 0, Some(b), Target::Minion(m)));
        assert_eq!(offered, !changed);
    }
}

/// Grand Temple of Set: "During your master phase, you can move 1 counter
/// from your pool to this card" — once a master phase; "Lock and burn X
/// counters from this card during your influence phase to steal a vampire
/// controlled by your prey with capacity less than X": X from one above
/// the target's capacity, a vampire as old as the counters not offered;
/// the vampire stolen for good, the counters left.
#[test]
fn grand_temple_of_set_banks_pool_then_steals_a_younger_vampire() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity == 3, 0), 2, false);
    let old = e.add_minion(SeatId(1), textless(|c| c.capacity == 6, 0), 4, false);
    let t = in_play(&mut e, 0, "Grand Temple of Set", None);
    hand(&mut e, 0, &["Ascendance"]);
    e.at_master_phase();
    pick(&mut e, ability(t.0, 0));
    let req = request(&mut e);
    assert!(!req.options.contains(&ability(t.0, 0)), "once a phase");
    assert_eq!(e.state.cards[t.index()].counters, 1);
    assert_eq!(e.state.seats[0].pool, 9);
    e.state.cards[t.index()].counters = 5;
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    let steal = |m| use_on(t, 1, None, Target::Minion(m));
    assert!(req.options.contains(&steal(young)));
    assert!(!req.options.contains(&steal(old)), "capacity 6, 5 counters");
    pick(&mut e, steal(young));
    let req = request(&mut e);
    assert_eq!(req.options, vec![set_x(4), set_x(5)]);
    pick(&mut e, set_x(4));
    request(&mut e);
    assert_eq!(e.state.minions[young.index()].owner, SeatId(0));
    let c = &e.state.cards[t.index()];
    assert!(c.locked);
    assert_eq!(c.counters, 1);
}

/// Grand Temple of Set: "Any vampire can burn this card as a Ⓓ action":
/// another Methuselah's, directed at its controller (rules-qa 6.8); the
/// card burns.
#[test]
fn grand_temple_of_set_is_burned_by_a_vampire() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    let t = in_play(&mut e, 2, "Grand Temple of Set", None);
    let burn = use_on(t, 2, Some(a), Target::None);
    assert_eq!(directed_at(&mut e, burn), Some(SeatId(2)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 2, "Grand Temple of Set"));
}

/// Heartblood of the Clan: "Any Banu Haqim can add 1 blood to this card as
/// an action": one of its controller's, at 0 stealth (LSJ 19991004),
/// undirected; another Methuselah's, directed at the controller. "During
/// your influence phase, you can move any amount of blood from this card to
/// an Banu Haqim in your uncontrolled region": X of its counters, once a
/// phase (LSJ 19970916), a non-Banu Haqim slot no target.
#[test]
fn heartblood_of_the_clan_fills_from_any_banu_haqim_and_feeds_the_uncontrolled() {
    let mut e = game(3);
    let own = e.add_minion(SeatId(0), of_clan("Banu Haqim", 1..=11, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let h = in_play(&mut e, 0, "Heartblood of the Clan", None);
    let add = use_on(h, 0, Some(own), Target::None);
    pick(&mut e, add);
    request(&mut e);
    let a = e.action().expect("an action");
    assert_eq!(a.stealth(&e.state), 0);
    assert_eq!(a.directed_seat(&e.state), None);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[h.index()].counters, 1, "from the bank");
    assert_eq!(blood(&e, own), 4);

    let mut e = game(3);
    let theirs = e.add_minion(SeatId(0), of_clan("Banu Haqim", 1..=11, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    let h = in_play(&mut e, 2, "Heartblood of the Clan", None);
    let add = use_on(h, 0, Some(theirs), Target::None);
    assert_eq!(directed_at(&mut e, add), Some(SeatId(2)));

    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    uncontrolled(&mut e, 0, plain("Brujah", 1), 1);
    uncontrolled(&mut e, 0, of_clan("Banu Haqim", 4..=11, 1), 1);
    let h = in_play(&mut e, 0, "Heartblood of the Clan", None);
    e.state.cards[h.index()].counters = 3;
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    assert!(!req
        .options
        .contains(&use_on(h, 1, None, Target::Uncontrolled(0))));
    pick(&mut e, use_on(h, 1, None, Target::Uncontrolled(1)));
    let req = request(&mut e);
    assert_eq!(req.options, vec![set_x(1), set_x(2), set_x(3)]);
    pick(&mut e, set_x(2));
    let req = request(&mut e);
    assert_eq!(e.state.seats[0].uncontrolled[1].counters, 3);
    assert_eq!(e.state.cards[h.index()].counters, 1);
    assert!(!req
        .options
        .contains(&use_on(h, 1, None, Target::Uncontrolled(1))));
}

/// Inbase Discotek, Frankfurt: "Lock to give a vampire who successfully
/// hunts an additional blood from the blood bank": another Methuselah's
/// vampire's hunt; a card's hunt too, from the bank, not the hunt's target
/// (RTR 20030519); a diseased hunter gains nothing, its hunt not successful
/// (LSJ 20050720-2).
/// Ledger: [RTR 20030519] [LSJ 20050720-2].
#[test]
fn inbase_discotek_adds_blood_after_any_successful_hunt_from_the_bank() {
    for diseased in [false, true] {
        let mut e = game(2);
        let v = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[v.index()].disease = diseased;
        let d = in_play(&mut e, 1, "Inbase Discotek, Frankfurt", None);
        pick(&mut e, action(v, ActionKind::Hunt));
        let lock = use_on(d, 0, None, Target::None);
        if diseased {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, lock);
            continue;
        }
        let req = offering(&mut e, lock);
        assert_eq!(req.seat, SeatId(1));
        pick(&mut e, lock);
        request(&mut e);
        assert_eq!(blood(&e, v), 3, "1 + the hunt + 1");
        assert!(e.state.cards[d.index()].locked);
    }
    // Abactor's hunt: blood stolen from a younger vampire, +1 from the bank.
    let mut e = game(2);
    let vc = of_sect(Sect::Sabbat, 5..=11, 0);
    let v = e.add_minion(SeatId(0), vc, 0, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    let d = in_play(&mut e, 1, "Inbase Discotek, Frankfurt", None);
    hand(&mut e, 0, &["Abactor"]);
    pick(&mut e, play(0, 0, v));
    let lock = use_on(d, 0, None, Target::None);
    pick_when(&mut e, lock);
    request(&mut e);
    assert!(e.state.cards[d.index()].locked);
}

/// The Mausoleum, Venice: "Lock during the polling step of a political
/// action to get +1 vote, with an additional +1 vote if the card named
/// Ventrue Headquarters is not in play." +2 without one; +1 with another
/// Methuselah's; never in a blood hunt (rules-digest D8: "political
/// action").
#[test]
fn the_mausoleum_venice_votes_two_unless_ventrue_headquarters_is_in_play() {
    for vhq in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        let m = in_play(&mut e, 0, "Mausoleum, Venice, The", None);
        if vhq {
            in_play(&mut e, 1, "Ventrue Headquarters", None);
        }
        hand(&mut e, 0, &["Kine Resources Contested"]);
        pick(&mut e, play(0, 0, a));
        let req = until(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
        let two = ability(m.0, 2);
        let one = ability(m.0, 0);
        assert_eq!(req.options.contains(&two), !vhq, "{vhq}");
        assert_eq!(req.options.contains(&one), vhq, "{vhq}");
        let want = if vhq { one } else { two };
        pick(&mut e, want);
        let r = e.referendum().expect("a referendum");
        assert_eq!(r.tally(&e.state).0, if vhq { 1 } else { 2 });
    }
}

/// The Mausoleum, Venice: not in a blood hunt's referendum.
#[test]
fn the_mausoleum_venice_is_not_used_in_a_blood_hunt() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 0), 1, true);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    let m = in_play(&mut e, 1, "Mausoleum, Venice, The", None);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: d,
            action: ActionKind::Diablerie,
            target: Some(z),
        },
    );
    let req = until(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
    assert!(!req.options.contains(&ability(m.0, 2)));
    assert!(!req.options.contains(&ability(m.0, 0)));
}

/// The Path of Typhon: "Cards requiring Serpentis [ser] cost Ministries 1
/// fewer blood": any Methuselah's Ministry, not another clan; "Any minion
/// can burn this card as a Ⓓ action that inflicts 1 unpreventable
/// environmental damage on acting vampires": the burner takes 1, the card
/// burned (rules-qa 84.25).
#[test]
fn the_path_of_typhon_discounts_serpentis_and_bites_its_burner() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), of_clan("Ministry", 4..=11, 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let p = in_play(&mut e, 1, "Path of Typhon, The", None);
    let temptation = find_library("Temptation");
    assert_eq!(e.state.printed_cost_off(m, temptation, 0).blood, 1);
    assert_eq!(e.state.printed_cost_off(b, temptation, 0).blood, 0);
    let burn = use_on(p, 0, Some(b), Target::None);
    pick(&mut e, burn);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Path of Typhon, The"));
    assert_eq!(blood(&e, b), 3);
    let _ = z;
}

/// Powerbase: Rome: "You can use a master phase action to add 1 counter to
/// this card": the action spent; "Any Giovanni you control can move up to
/// 2 blood from themselves to this card as a +1 stealth action": the
/// amount chosen as it resolves, 0 to 2 (rules-qa 111.13); "Lock and burn
/// X counters from this card during the polling step of a political
/// action to get +X votes".
#[test]
fn powerbase_rome_banks_counters_and_giovanni_blood_for_votes() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), of_clan("Giovanni", 4..=11, 0), 4, false);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let r = in_play(&mut e, 0, "Powerbase: Rome", None);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    e.at_master_phase();
    pick(&mut e, ability(r.0, 0));
    let req = request(&mut e);
    assert!(!req.options.contains(&ability(r.0, 0)), "one action spent");
    assert_eq!(e.state.cards[r.index()].counters, 1);
    let give = use_on(r, 1, Some(g), Target::None);
    assert_eq!(action_stealth(&mut e, give), 1);
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![set_x(0), set_x(1), set_x(2)]);
    pick(&mut e, set_x(2));
    request(&mut e);
    assert_eq!(blood(&e, g), 2);
    assert_eq!(e.state.cards[r.index()].counters, 3);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    pick(&mut e, ability(r.0, 2));
    pick(&mut e, set_x(2));
    let rf = e.referendum().expect("a referendum");
    assert_eq!(rf.tally(&e.state).0, 2);
    let c = &e.state.cards[r.index()];
    assert!(c.locked);
    assert_eq!(c.counters, 1);
}

/// Powerbase: Rome: "The two actions provided are not cross-restrictive.
/// Each can be performed once during the same turn by the same minion"
/// (LSJ 20100909): one Giovanni moves blood, then burns the counters; the
/// burn by another Methuselah's minion is directed at the controller.
/// Ledger: [LSJ 20100909].
#[test]
fn powerbase_rome_two_actions_are_not_cross_restrictive() {
    let mut e = game(3);
    let g = e.add_minion(SeatId(0), of_clan("Giovanni", 4..=11, 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let r = in_play(&mut e, 0, "Powerbase: Rome", None);
    pick(&mut e, use_on(r, 1, Some(g), Target::None));
    pick_when(&mut e, set_x(1));
    let burn = use_on(r, 4, Some(g), Target::None);
    let req = offering(&mut e, burn);
    assert_eq!(req.window, WindowLabel::MinionPhase);
    pick(&mut e, burn);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[r.index()].counters, 0);

    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    let r = in_play(&mut e, 2, "Powerbase: Rome", None);
    let burn = use_on(r, 4, Some(a), Target::None);
    assert_eq!(directed_at(&mut e, burn), Some(SeatId(2)));
}

/// Powerbase: Madrid: "Vampires controlled by other Methuselahs can burn
/// all the counters from this card as a Ⓓ action": directed at its
/// controller (rules-qa 6.8), not the acting Methuselah's prey or
/// predator only.
#[test]
fn powerbase_madrid_counter_burn_is_directed_at_its_controller() {
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    let p = in_play(&mut e, 2, "Powerbase: Madrid", None);
    let burn = use_on(p, 2, Some(a), Target::None);
    assert_eq!(directed_at(&mut e, burn), Some(SeatId(2)));
}

/// The Realm of the Black Sun: "If your prey gains 1 or more pool during
/// their master phase, you can gain 1 pool": once that phase however many
/// gains (rules-digest D7); not for the prey's gain in another phase.
#[test]
fn realm_of_the_black_sun_gains_once_per_prey_master_phase() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    let realm = in_play(&mut e, 2, "Realm of the Black Sun, The", None);
    e.state.seats[0].mpa_next = 1;
    hand(&mut e, 0, &["Ascendance", "Ascendance", "Ascendance"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    let gain = Choice::ResolveRider { card: realm.0 };
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(2));
    assert!(req.options.contains(&gain));
    pick(&mut e, gain);
    pick(&mut e, master_play(0));
    let req = request(&mut e);
    assert!(!req.options.contains(&gain), "once a master phase");
    assert_eq!(e.state.seats[2].pool, 11);
    // The prey's gain in its minion phase.
    pick(&mut e, Choice::Pass);
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, gain);
    assert_eq!(e.state.seats[2].pool, 11);
}

/// The Realm of the Black Sun: "Any minion can burn this card as a Ⓓ
/// action; titled vampires get -1 stealth during that action."
#[test]
fn realm_of_the_black_sun_burn_costs_titled_vampires_stealth() {
    for titled in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        if titled {
            e.state.minions[a.index()].granted_title = Some(Title::Prince("Paris"));
        }
        e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        let realm = in_play(&mut e, 1, "Realm of the Black Sun, The", None);
        let burn = use_on(realm, 0, Some(a), Target::None);
        assert_eq!(action_stealth(&mut e, burn), if titled { -1 } else { 0 });
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(ash_has(&e, 1, "Realm of the Black Sun, The"));
    }
}

/// The Stranger Among Us: "Search your crypt for a scarce vampire. Show the
/// card to all players and place it face-down in your uncontrolled
/// region": only the scarce ones offered, the crypt shuffled after (LSJ
/// 20020304-1).
/// Ledger: [LSJ 20020304-1].
#[test]
fn the_stranger_among_us_searches_a_scarce_vampire() {
    let scarce = CRYPT
        .iter()
        .position(|c| c.scarce && !c.imbued)
        .expect("a scarce vampire") as u16;
    let plain_ones: Vec<u16> = (0..12).map(|i| plain("Brujah", i)).collect();
    let mut shuffled = false;
    for seed in 0..4 {
        let mut e = Engine::bare_seeded(seed, 2);
        for s in &mut e.state.seats {
            s.pool = 10;
        }
        e.state.turn = 4;
        e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let mut order = plain_ones.clone();
        order.insert(6, scarce);
        for &c in &order {
            e.state.seats[0].crypt.put(c, zvtes::copies::CopyId::NONE);
        }
        let before = e.state.seats[0].crypt.to_cards();
        hand(&mut e, 0, &["Stranger Among Us, The", "Ascendance"]);
        e.at_master_phase();
        pick(&mut e, master_play(0));
        let req = request(&mut e);
        assert_eq!(req.options, vec![Choice::Pass, Choice::AshVampire(scarce)]);
        pick(&mut e, Choice::AshVampire(scarce));
        request(&mut e);
        let slot = e.state.seats[0].uncontrolled.last().expect("placed");
        assert_eq!((slot.card, slot.counters), (scarce, 0));
        let mut rest = before.clone();
        rest.retain(|&c| c != scarce);
        shuffled |= e.state.seats[0].crypt.to_cards() != rest;
    }
    assert!(shuffled, "the crypt shuffled");
}

/// Tomb of Rameses III: chosen as played; "For each blood counter you
/// transfer to the chosen vampire during your influence phase, move one
/// counter from the blood bank to the Tomb": only the chosen vampire's
/// transfers, three of four (the 1999 text; LSJ 20030312 "Card text is
/// correct"); "At the end of your influence phase, if the total number of
/// counters on the chosen vampire and on the Tomb equals or exceeds that
/// vampire's capacity, you may move the vampire to the ready region": 3 + 3,
/// with its own counters only (LSJ 20040419); the Tomb then burned with its
/// counters.
#[test]
fn tomb_of_rameses_iii_counts_transfers_to_its_vampire_and_brings_it_out() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let six = textless(|c| c.capacity == 6, 0);
    uncontrolled(&mut e, 0, six, 0);
    uncontrolled(&mut e, 0, plain("Ventrue", 1), 0);
    e.state.turn = 6;
    hand(&mut e, 0, &["Tomb of Rameses III", "Ascendance"]);
    e.at_master_phase();
    pick(&mut e, master_at(0, 0, Target::Uncontrolled(0)));
    request(&mut e);
    let t = placed_on(&e, "Tomb of Rameses III", None);
    assert_eq!(e.state.cards[t.index()].chosen_crypt, Some(six));
    until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    for slot in [0, 1, 0, 0] {
        pick(&mut e, Choice::Transfer { slot });
    }
    assert_eq!(e.state.cards[t.index()].counters, 3);
    pick(&mut e, Choice::Pass);
    let ready = Choice::ResolveRider { card: t.0 };
    let req = offering(&mut e, ready);
    assert_eq!(req.seat, SeatId(0));
    pick(&mut e, ready);
    request(&mut e);
    let m = e
        .state
        .minions
        .iter()
        .position(|v| v.in_play && v.crypt_card() == Some(six))
        .expect("ready");
    assert_eq!(e.state.minions[m].owner, SeatId(0));
    assert_eq!(e.state.minions[m].blood, 3, "its own counters");
    assert!(ash_has(&e, 0, "Tomb of Rameses III"));
    assert_eq!(e.state.seats[0].uncontrolled.len(), 1);
}

/// Tomb of Rameses III: below the capacity, no move offered; "Burn this
/// card (and the counters on it) when this vampire leaves the uncontrolled
/// region": influenced out normally, the Tomb burns at once.
#[test]
fn tomb_of_rameses_iii_burns_when_its_vampire_leaves_otherwise() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let six = textless(|c| c.capacity == 6, 0);
    let three = textless(|c| c.capacity == 3, 0);
    uncontrolled(&mut e, 0, six, 1);
    uncontrolled(&mut e, 0, three, 3);
    let t = in_play(&mut e, 0, "Tomb of Rameses III", None);
    e.state.cards[t.index()].chosen_crypt = Some(three);
    e.state.cards[t.index()].counters = 2;
    until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::Transfer { slot: 1 });
    assert_eq!(e.state.cards[t.index()].counters, 3);
    pick(&mut e, Choice::InfluenceOut { slot: 1 });
    request(&mut e);
    assert!(ash_has(&e, 0, "Tomb of Rameses III"));

    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    uncontrolled(&mut e, 0, six, 1);
    let t = in_play(&mut e, 0, "Tomb of Rameses III", None);
    e.state.cards[t.index()].chosen_crypt = Some(six);
    e.state.cards[t.index()].counters = 3;
    until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::Pass);
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        Choice::ResolveRider { card: t.0 },
    );
    assert!(e.state.cards[t.index()].in_play, "5 of 6");
}

/// Tomb of Rameses III: "When … the controller of this card changes, the
/// controller chooses a vampire in his or her uncontrolled region": taken
/// by another Methuselah, it chooses in that one's region, the old choice
/// gone and the counters kept.
#[test]
fn tomb_of_rameses_iii_rechooses_on_a_change_of_control() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let six = textless(|c| c.capacity == 6, 0);
    let three = textless(|c| c.capacity == 3, 0);
    let four = textless(|c| c.capacity == 4, 0);
    uncontrolled(&mut e, 1, six, 1);
    uncontrolled(&mut e, 0, three, 0);
    uncontrolled(&mut e, 0, four, 0);
    let t = in_play(&mut e, 1, "Tomb of Rameses III", None);
    e.state.cards[t.index()].chosen_crypt = Some(six);
    e.state.cards[t.index()].counters = 2;
    e.state.minions[a.index()].gained_levels = vec!["dom", "dom"];
    hand(&mut e, 0, &["Dominate Kine"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 1,
            actor: Some(a),
            target: Target::Card(t),
        },
    );
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Slot(0), Choice::Slot(1)]);
    pick(&mut e, Choice::Slot(1));
    request(&mut e);
    let c = &e.state.cards[t.index()];
    assert_eq!(c.chosen_crypt, Some(four));
    assert_eq!(c.counters, 2);
    assert!(c.in_play);
    assert_eq!(e.state.card_controller(t), SeatId(0));
}

/// Tower of London: "Lock after resolution of a successful action requiring
/// Necromancy [nec] to gain 1 pool": another Methuselah's [nec] action
/// card; not a blocked one.
#[test]
fn tower_of_london_gains_a_pool_after_a_successful_necromancy_action() {
    for blocked in [false, true] {
        let mut e = game(2);
        let n = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[n.index()].gained_levels = vec!["nec"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let t = in_play(&mut e, 1, "Tower of London", None);
        e.state.seats[0]
            .ash
            .put(find_library("Ascendance"), zvtes::copies::CopyId::NONE);
        hand(&mut e, 0, &["Whispers from the Dead"]);
        pick(&mut e, play(0, 0, n));
        let lock = use_on(t, 0, None, Target::None);
        if blocked {
            pick_when(&mut e, block(z));
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, lock);
            assert_eq!(e.state.seats[1].pool, 10);
            continue;
        }
        let req = offering(&mut e, lock);
        assert_eq!(req.seat, SeatId(1));
        pick(&mut e, lock);
        request(&mut e);
        assert_eq!(e.state.seats[1].pool, 11);
        assert!(e.state.cards[t.index()].locked);
    }
}

/// True Faith: "Put this card on a non-infernal mortal ally you control, or
/// burn 2 pool to put this card on a non-infernal minion you control": the
/// card itself on the ally for its cost; on a vampire for 2 pool more; an
/// infernal minion no target.
#[test]
fn true_faith_goes_on_a_mortal_ally_free_or_any_minion_for_two_pool() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let inf = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.state.minions[inf.index()].made_infernal = true;
    let ally = e.add_ally(SeatId(0), find_library("Neighborhood Watch Commander"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["True Faith", "Ascendance"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(req.options.contains(&master_at(0, 0, Target::Minion(ally))));
    assert!(!req.options.contains(&master_at(0, 0, Target::Minion(v))));
    assert!(req.options.contains(&master_at(0, 1, Target::Minion(v))));
    assert!(!req.options.contains(&master_at(0, 1, Target::Minion(inf))));
    pick(&mut e, master_at(0, 1, Target::Minion(v)));
    request(&mut e);
    placed_on(&e, "True Faith", Some(v));
    assert_eq!(e.state.seats[0].pool, 7, "1 + 2");

    let mut e = game(2);
    let ally = e.add_ally(SeatId(0), find_library("Neighborhood Watch Commander"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["True Faith", "Ascendance"]);
    e.at_master_phase();
    pick(&mut e, master_at(0, 0, Target::Minion(ally)));
    request(&mut e);
    placed_on(&e, "True Faith", Some(ally));
    assert_eq!(e.state.seats[0].pool, 9);
}

/// True Faith: "This minion cannot be the target of directed actions
/// requiring Dominate [dom] or Presence [pre]": Exile [pre] not offered on
/// its bearer, offered on it without.
#[test]
fn true_faith_bars_dominate_and_presence_directed_actions() {
    for faith in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Toreador", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["pre"];
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), textless(|c| c.capacity <= 2, 0), 2, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if faith {
            in_play(&mut e, 1, "True Faith", Some(z));
        }
        hand(&mut e, 0, &["Exile"]);
        let exile = Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::Minion(z),
        };
        let req = request(&mut e);
        assert_eq!(req.options.contains(&exile), !faith, "{faith}");
    }
}

/// True Faith: "In combat, damage this minion inflicts on infernal minions
/// is aggravated. Infernal minions cannot block or strike against this
/// minion." An infernal vampire's block of the bearer not offered, a
/// non-infernal one's is; blocked by the bearer, the infernal strikes not,
/// and the bearer's hand strike sends it to torpor.
#[test]
fn true_faith_aggravates_damage_on_infernals_who_cannot_block_or_strike_it() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let inf = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[inf.index()].made_infernal = true;
    let other = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 0, "True Faith", Some(b));
    pick(&mut e, bleed(b));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(!req.options.contains(&block(inf)));
    assert!(req.options.contains(&block(other)));

    let mut e = game(2);
    let inf = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.state.minions[inf.index()].made_infernal = true;
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    in_play(&mut e, 1, "True Faith", Some(b));
    pick(&mut e, bleed(inf));
    pick_when(&mut e, block(b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[inf.index()].in_torpor, "aggravated");
    assert_eq!(blood(&e, b), 4, "no strike against it");
}

/// True Faith on Alejandro Aguirre: "If a card transforms the damage he
/// inflicts to aggravated (eg. True Faith), this includes the damage from
/// his ability" (ANK 20210620): before range in another combat, the
/// infernal combatant takes 1 aggravated, the other 1 normal.
/// Ledger: [ANK 20210620].
#[test]
fn true_faith_aggravates_alejandro_aguirres_damage_on_an_infernal() {
    let mut e = game(2);
    let al = e.add_minion(SeatId(0), find_card("Alejandro Aguirre"), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let inf = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[inf.index()].made_infernal = true;
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 0, "True Faith", Some(al));
    pick_when(&mut e, bleed(b));
    pick_when(&mut e, block(inf));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(al),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[inf.index()].in_torpor, "aggravated");
    assert!(!e.state.minions[b.index()].in_torpor);
}

/// True Faith: "Burn this card if this minion becomes infernal", read live.
#[test]
fn true_faith_burns_when_its_minion_becomes_infernal() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let t = in_play(&mut e, 0, "True Faith", Some(b));
    request(&mut e);
    assert!(e.state.cards[t.index()].in_play);
    e.state.minions[b.index()].made_infernal = true;
    request(&mut e);
    assert!(!e.state.cards[t.index()].in_play);
    assert!(ash_has(&e, 0, "True Faith"));
}

/// The Crusader Sword: "Only usable by a minion with True Faith or an
/// imbued. Vampires opposing this minion with True Faith or this imbued
/// cannot use maneuvers to go to close range": a True Faith vampire's
/// strike with it, at strength + 1 aggravated.
#[test]
fn the_crusader_sword_strikes_for_a_true_faith_bearer() {
    for faith in [false, true] {
        let mut e = game(2);
        let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let sword = in_play(&mut e, 0, "Crusader Sword, The", Some(b));
        if faith {
            in_play(&mut e, 0, "True Faith", Some(b));
        }
        pick(&mut e, bleed(b));
        pick_when(&mut e, block(z));
        let strike = use_on(sword, 0, None, Target::None);
        if faith {
            pick_when(&mut e, strike);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(e.state.minions[z.index()].in_torpor, "aggravated");
        } else {
            never_offered_where(
                &mut e,
                SeatId(1),
                WindowLabel::MinionPhase,
                |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == sword),
            );
        }
    }
}

// ---- Allies, equipment and retainers (misc) ---------------------------------

/// A card from hand slot `card` played by `actor` in `mode` on `target`.
fn play_on(card: u8, mode: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target,
    }
}

/// The card in play that is ally `m`.
fn body(e: &Engine, m: MinionId) -> CardId {
    CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.body == Some(m))
            .expect("the ally's card") as u32,
    )
}

/// The card `name` in play, itself (not a stand-in), anywhere.
fn card_in_play(e: &Engine, name: &str) -> Option<CardId> {
    let card = find_library(name);
    e.state
        .cards
        .iter()
        .position(|c| c.card == card && c.in_play && !c.effect_only)
        .map(|i| CardId(i as u32))
}

/// Akhenaten: "Akhenaten can burn himself to burn a Ministry controlled by
/// your prey as a Ⓓ action": both burned as it succeeds, he shuffled into
/// his owner's library; blocked, neither (rules-qa 55.4).
#[test]
fn akhenaten_burns_himself_and_a_ministry_of_the_prey() {
    for blocked in [false, true] {
        let mut e = game(2);
        let akh = e.add_ally(SeatId(0), find_library("Akhenaten, The Sun Pharaoh"));
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let min = e.add_minion(SeatId(1), of_clan("Ministry", 1..=11, 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let other = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        fill_hand(&mut e, 0);
        let card = body(&e, akh);
        let not = use_on(card, 0, Some(akh), Target::Minion(other));
        let burn = use_on(card, 0, Some(akh), Target::Minion(min));
        let req = request(&mut e);
        assert!(!req.options.contains(&not), "not a Ministry");
        pick(&mut e, burn);
        if blocked {
            pick_when(&mut e, block(z));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let lib = e.state.seats[0]
            .library
            .contains(&find_library("Akhenaten, The Sun Pharaoh"));
        assert_eq!(e.state.minions[min.index()].in_play, blocked);
        assert_eq!(e.state.minions[akh.index()].in_play, blocked);
        assert_eq!(lib, !blocked, "shuffled into his library");
    }
}

/// Akhenaten: "In combat with a Ministry, any damage he inflicts is
/// aggravated": his hand strike sends a blocked Ministry to torpor; a
/// Brujah mends it.
#[test]
fn akhenaten_inflicts_aggravated_damage_on_a_ministry() {
    for (clan, torpor) in [("Ministry", true), ("Brujah", false)] {
        let mut e = game(2);
        let v = e.add_minion(SeatId(0), of_clan(clan, 4..=11, 0), 4, false);
        e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        let akh = e.add_ally(SeatId(1), find_library("Akhenaten, The Sun Pharaoh"));
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        pick(&mut e, bleed(v));
        pick_when(&mut e, block(akh));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[v.index()].in_torpor, torpor, "{clan}");
    }
}

/// The mummies (Akhenaten, Amam, Qetu): "If he is burned, shuffle him into
/// his owner's library", other burn effects still applying: Set's Curse
/// still enters play.
/// Ledger: [ANK 20200813].
#[test]
fn the_mummies_shuffle_into_the_library_when_burned() {
    for name in [
        "Akhenaten, The Sun Pharaoh",
        "Amam the Devourer",
        "Qetu the Evil Doer",
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let m = e.add_ally(SeatId(1), find_library(name));
        fill_hand(&mut e, 1);
        let library = e.state.seats[1].library.len();
        hand(&mut e, 0, &["Set's Curse"]);
        pick(&mut e, play_on(0, 0, a, Target::Minion(m)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.minions[m.index()].in_play, "{name}");
        assert!(e.state.seats[1].library.contains(&find_library(name)));
        assert_eq!(e.state.seats[1].library.len(), library + 1);
        assert!(!ash_has(&e, 1, name));
        assert!(card_in_play(&e, "Set's Curse").is_some(), "{name}");
    }
}

/// Amam the Devourer: "Amam can burn a vampire in torpor to gain 1 life as
/// a Ⓓ action": past his 3 life; "The two actions provided are not
/// cross-restrictive": unlocked in the same turn, his enter-combat action
/// still offered.
/// Ledger: [LSJ 20100909].
#[test]
fn amam_burns_a_torpid_vampire_and_enters_combat_the_same_turn() {
    let mut e = game(2);
    let amam = e.add_ally(SeatId(0), find_library("Amam the Devourer"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, true);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let card = body(&e, amam);
    pick(&mut e, use_on(card, 1, Some(amam), Target::Minion(t)));
    let rush = use_on(card, 0, Some(amam), Target::Minion(z));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_play, "burned");
    assert_eq!(blood(&e, amam), 4);
    assert!(!req.options.contains(&rush), "locked");
    e.state.minions[amam.index()].locked = false;
    let req = request(&mut e);
    assert!(req.options.contains(&rush), "not cross-restrictive");
}

/// Amam the Devourer: "If a minion opposing Amam in combat is burned, Amam
/// can gain 1 life": an ally at 1 life burned by his strike; burned with it,
/// nothing (LSJ 20080701).
#[test]
fn amam_may_gain_a_life_when_his_opponent_burns() {
    let mut e = game(2);
    let amam = e.add_ally(SeatId(0), find_library("Amam the Devourer"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Neighborhood Watch Commander"));
    e.state.minions[ally.index()].blood = 1;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = body(&e, amam);
    pick(&mut e, use_on(card, 0, Some(amam), Target::Minion(ally)));
    let gain = Choice::ResolvePrinted { minion: amam };
    offering(&mut e, gain);
    assert!(!e.state.minions[ally.index()].in_play);
    let before = blood(&e, amam);
    pick(&mut e, gain);
    request(&mut e);
    assert_eq!(blood(&e, amam), before + 1);
}

/// Ambrosius: "Ambrosius can move a retainer from your ash heap to a ready
/// vampire you control and put 3 pathos counters on that retainer as an
/// action. During your unlock phase, burn 1 pathos counter from that
/// retainer. Remove the retainer from the game if it has no pathos
/// counters." Raven Spy, named as announced, enters at its inferior
/// version, 1 life (LSJ 20080116), its vampire +1 intercept; after three of
/// its controller's unlock phases it is removed from the game, Ambrosius
/// gone too (rules-qa 74.2).
/// Ledger: "Cards requiring a discipline come in play at the inferior
/// version" [RBK equip] [RBK recruit-ally] [RBK employ-retainer].
#[test]
fn ambrosius_ferries_a_retainer_with_three_pathos_counters() {
    let mut e = game(2);
    let amb = e.add_ally(SeatId(0), find_library("Ambrosius, The Ferryman"));
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    fill_hand(&mut e, 0);
    for name in ["Raven Spy", "Zombie"] {
        e.state.seats[0]
            .ash
            .put(find_library(name), zvtes::copies::CopyId::NONE);
    }
    let card = body(&e, amb);
    pick(&mut e, use_on(card, 0, Some(amb), Target::Minion(v)));
    let req = request(&mut e);
    assert!(req
        .options
        .contains(&Choice::FindCard(find_library("Zombie"))));
    pick(&mut e, Choice::FindCard(find_library("Raven Spy")));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let spy = placed_on(&e, "Raven Spy", Some(v));
    let c = &e.state.cards[spy.index()];
    assert_eq!((c.mode, c.counters, c.pathos.0), (0, 1, 3));
    e.state.minions[amb.index()].in_play = false;
    for left in [2, 1] {
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.cards[spy.index()].pathos.0, left);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    }
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[spy.index()].in_play);
    assert!(e.state.seats[0]
        .removed
        .contains(&find_library("Raven Spy")));
}

/// Ambrosius: requirements do not apply to the retainer he moves (a
/// Giovanni's Zombie onto a Brujah).
/// Ledger: [LSJ 20100204] [LSJ 20040518-2] [LSJ 20100302-1].
#[test]
fn ambrosius_ferries_a_retainer_whatever_its_requirements() {
    let mut e = game(2);
    let amb = e.add_ally(SeatId(0), find_library("Ambrosius, The Ferryman"));
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    fill_hand(&mut e, 0);
    e.state.seats[0]
        .ash
        .put(find_library("Zombie"), zvtes::copies::CopyId::NONE);
    let card = body(&e, amb);
    pick(&mut e, use_on(card, 0, Some(amb), Target::Minion(v)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let z = placed_on(&e, "Zombie", Some(v));
    assert_eq!(e.state.cards[z.index()].pathos.0, 3);
}

/// Ambrosius: "Ambrosius is immune to non-aggravated damage": blocking, he
/// takes nothing of a hand strike.
/// Ledger: [LSJ 20010627] [LSJ 20010629] [LSJ 20090304-1].
#[test]
fn ambrosius_takes_no_normal_damage() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let amb = e.add_ally(SeatId(1), find_library("Ambrosius, The Ferryman"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(amb));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[amb.index()].in_play);
    assert_eq!(blood(&e, amb), 2);
}

/// The Ancestor's Talisman: "The bearer gets +1 intercept during actions
/// requiring Dominate [dom] or Presence [pre]": not optional, a bleed
/// played with a [dom] card +1, a plain bleed 0.
/// Ledger: [RTR 19980707] [LSJ 19980722].
#[test]
fn ancestors_talisman_intercepts_dominate_and_presence_actions() {
    for card in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["dom"];
        e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        in_play(&mut e, 1, "Ancestor's Talisman, The", Some(z));
        hand(&mut e, 0, &["Dominate Kine"]);
        if card {
            pick(&mut e, play(0, 0, a));
        } else {
            pick(&mut e, bleed(a));
        }
        request(&mut e);
        let act = e.action().expect("an action");
        assert_eq!(act.intercept_of(&e.state, z), i32::from(card), "{card}");
    }
}

/// The Ancestor's Talisman: equipped from its bearer by another minion not
/// meeting its requirements (clan equipment), it works for it.
/// Ledger: [ANK 20180719-3].
#[test]
fn ancestors_talisman_works_for_a_non_banu_haqim_bearer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dom"];
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let t = in_play(&mut e, 1, "Ancestor's Talisman, The", Some(y));
    e.state.set_host(t, z);
    hand(&mut e, 0, &["Dominate Kine"]);
    pick(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(e.action().expect("an action").intercept_of(&e.state, z), 1);
}

/// Baleful Doll: "[nec] Choose a vampire controlled by your prey. The
/// bearer can lock themselves and burn this equipment during their unlock
/// phase to burn 3 blood from the chosen vampire." The equip chooses the
/// prey's vampire (directed at the prey); the bearer, locked by acting,
/// unlocks first, then burns the Doll: locked, the chosen -3.
/// Ledger: [PIB 20151009].
#[test]
fn baleful_doll_burns_three_blood_from_the_preys_chosen_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Giovanni", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["nec"];
    e.add_minion(SeatId(0), plain("Giovanni", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Baleful Doll"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(z)));
    request(&mut e);
    assert_eq!(
        e.action().expect("an action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let doll = placed_on(&e, "Baleful Doll", Some(a));
    assert_eq!(e.state.cards[doll.index()].chosen, vec![z]);
    let burn = use_on(doll, 0, None, Target::None);
    let req = offering(&mut e, burn);
    assert_eq!(req.seat, SeatId(0));
    assert!(!e.state.minions[a.index()].locked, "unlocked first");
    let before = blood(&e, z);
    pick(&mut e, burn);
    request(&mut e);
    assert!(e.state.minions[a.index()].locked);
    assert!(ash_has(&e, 0, "Baleful Doll"));
    assert_eq!(blood(&e, z), before - 3);
}

/// Baleful Doll: [LSJ 20011105] "You can tap and burn it for no effect if
/// the chosen vampire is no longer in play, sure. The target need only
/// exist when the card is played": the chosen vampire burned, the Doll's
/// burn still offered, the bearer locked and the Doll burned for nothing.
#[test]
fn baleful_doll_burns_for_nothing_once_its_chosen_vampire_is_gone() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Giovanni", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["nec"];
    e.add_minion(SeatId(0), plain("Giovanni", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Baleful Doll"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(z)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let doll = placed_on(&e, "Baleful Doll", Some(a));
    e.burn(z);
    let burn = use_on(doll, 0, None, Target::None);
    let req = offering(&mut e, burn);
    assert_eq!(req.seat, SeatId(0));
    pick(&mut e, burn);
    request(&mut e);
    assert!(e.state.minions[a.index()].locked);
    assert!(ash_has(&e, 0, "Baleful Doll"));
}

/// Brigitte Gebauer: "Lock Brigitte as a minion you control announces a
/// bleed action to give them +1 bleed for that action. If that action is
/// blocked and combat occurs, Brigitte burns 1 life." Unblocked: +1 bleed,
/// she locked; blocked into combat: 3 to 2 life; the block's combat
/// canceled (Hard Case), none (LSJ 20010716). Usable the turn she arrives.
/// Ledger: [ANK 20180517].
#[test]
fn brigitte_gives_a_bleed_one_and_burns_a_life_if_combat_follows() {
    for case in ["unblocked", "combat", "canceled"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let b = e.add_ally(SeatId(0), find_library("Brigitte Gebauer"));
        e.state.minions[b.index()].entered_turn = zvtes::state::Unhashed(e.state.turn);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        if case == "canceled" {
            hand(&mut e, 1, &["Hard Case"]);
        }
        let lend = use_on(body(&e, b), 0, None, Target::None);
        pick(&mut e, bleed(a));
        pick_when(&mut e, lend);
        assert!(e.state.minions[b.index()].locked);
        match case {
            "unblocked" => {
                until(&mut e, SeatId(1), WindowLabel::MinionPhase);
                assert_eq!(e.state.seats[1].pool, 10 - 2);
            }
            "combat" => {
                pick_when(&mut e, block(z));
                until(&mut e, SeatId(1), WindowLabel::MinionPhase);
                assert_eq!(blood(&e, b), 2);
            }
            _ => {
                pick_when(&mut e, block(z));
                pick_when(
                    &mut e,
                    Choice::Play {
                        card: 0,
                        mode: 0,
                        actor: Some(z),
                        target: Target::None,
                    },
                );
                until(&mut e, SeatId(1), WindowLabel::MinionPhase);
                assert_eq!(blood(&e, b), 3);
            }
        }
    }
}

/// Brigitte Gebauer at 1 life: the block's combat begins and she burns, a
/// side effect of the action — not the acting minion burning her: Taking the
/// Skin: Minion never offered to the acting vampire.
/// Ledger: [RTR 19960124] [LSJ 20090922] [ANK 20181022] [ANK 20220916].
#[test]
fn brigitte_burning_her_last_life_is_no_burn_by_the_acting_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["abo"];
    let b = e.add_ally(SeatId(0), find_library("Brigitte Gebauer"));
    e.state.minions[b.index()].blood = 1;
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Taking the Skin: Minion"]);
    let lend = use_on(body(&e, b), 0, None, Target::None);
    pick(&mut e, bleed(a));
    pick_when(&mut e, lend);
    pick_when(&mut e, block(z));
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(x), .. } if *x == a),
    );
    assert!(!e.state.minions[b.index()].in_play, "burned");
}

/// Canopic Jar: "Choose a ready minion. The bearer gets +1 intercept
/// against the chosen minion. If the bearer blocks the chosen minion, the
/// bearer can burn this equipment before block resolution to cancel combat
/// and unlock." +1 against the chosen, 0 against another; blocking it, the
/// Jar burned: no combat, the action blocked, the bearer unlocked.
#[test]
fn canopic_jar_intercepts_the_chosen_minion_and_cancels_its_block_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let o = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let jar = in_play(&mut e, 1, "Canopic Jar", Some(z));
    e.state.cards[jar.index()].chosen = vec![a];
    pick(&mut e, bleed(a));
    request(&mut e);
    let act = e.action().expect("an action");
    assert_eq!(act.intercept_of(&e.state, z), 1);
    pick_when(&mut e, block(z));
    let burn = use_on(jar, 0, None, Target::None);
    pick_when(&mut e, burn);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert!(ash_has(&e, 1, "Canopic Jar"));
    assert!(!e.state.minions[z.index()].locked, "unlocked");
    assert_eq!((blood(&e, a), blood(&e, z)), (4, 4), "no combat");
    let _ = o;
}

/// Canopic Jar [SER]: "As above, and the chosen minion burns 1 blood or
/// life after this equipment is burned": the bearer's burn of it.
#[test]
fn canopic_jar_superior_burns_a_blood_from_the_chosen_when_burned() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let jar = in_play_mode(&mut e, 1, "Canopic Jar", Some(z), 1);
    e.state.cards[jar.index()].chosen = vec![a];
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, use_on(jar, 0, None, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 3);
}

/// Felix "Fix" Hessian: "Felix can burn a location as a +1 stealth Ⓓ
/// action that costs 1 pool": directed at the location's controller, the
/// pool paid as it resolves.
/// Ledger: [LSJ 20090324].
#[test]
fn felix_burns_a_location_for_a_pool() {
    let mut e = game(3);
    let f = e.add_ally(SeatId(0), find_library("Felix \"Fix\" Hessian"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    let loc = in_play(&mut e, 2, "Ventrue Headquarters", None);
    let burn = use_on(body(&e, f), 0, Some(f), Target::Card(loc));
    pick(&mut e, burn);
    request(&mut e);
    let act = e.action().expect("an action");
    assert_eq!(act.directed_seat(&e.state), Some(SeatId(2)));
    assert_eq!(act.stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 2, "Ventrue Headquarters"));
    assert_eq!(e.state.seats[0].pool, 9);
}

/// The Ghouls of Plaza Morería: "They can burn 1 blood from a ready vampire
/// and put 1 container counter on themselves as a +1 stealth Ⓓ action. Any
/// ready Banu Haqim can burn X container counters from them to gain X blood
/// as a +1 stealth action": X named as announced; another Methuselah's
/// Banu Haqim's action is directed at the Ghouls' controller; it requires a
/// Banu Haqim.
/// Ledger: [LSJ 20080604].
#[test]
fn ghouls_of_plaza_moreria_store_blood_for_a_banu_haqim() {
    let mut e = game(2);
    let g = e.add_ally(SeatId(0), find_library("Ghouls of Plaza Morería, The"));
    let b = e.add_minion(SeatId(0), of_clan("Banu Haqim", 4..=11, 0), 1, false);
    let br = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = body(&e, g);
    pick(&mut e, use_on(card, 2, Some(g), Target::Minion(z)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 3);
    assert_eq!(e.state.cards[card.index()].counters, 1);
    e.state.cards[card.index()].counters = 3;
    let take = use_on(card, 3, Some(b), Target::None);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req
        .options
        .contains(&use_on(card, 3, Some(br), Target::None)));
    pick(&mut e, take);
    let req = request(&mut e);
    assert_eq!(req.options, vec![set_x(1), set_x(2), set_x(3)]);
    pick(&mut e, set_x(2));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 3);
    assert_eq!(e.state.cards[card.index()].counters, 1);

    let mut e = game(3);
    let g = e.add_ally(SeatId(2), find_library("Ghouls of Plaza Morería, The"));
    let b = e.add_minion(SeatId(0), of_clan("Banu Haqim", 4..=11, 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = body(&e, g);
    e.state.cards[card.index()].counters = 1;
    pick(&mut e, use_on(card, 3, Some(b), Target::None));
    request(&mut e);
    assert_eq!(
        e.action().expect("an action").directed_seat(&e.state),
        Some(SeatId(2))
    );
}

/// The Ghouls of Plaza Morería: "Ghouls are monsters, not mortal".
/// Ledger: [ANK 20200203-2] [LSJ 20060515].
#[test]
fn ghouls_of_plaza_moreria_are_no_mortals() {
    let g = find_library("Ghouls of Plaza Morería, The");
    let labels = zvtes::library::LIBRARY[g as usize].labels;
    assert!(labels.contains(&zvtes::library::Label::Ghoul));
    assert!(!labels.contains(&zvtes::library::Label::Mortal));
}

/// Giuseppe, Gravedigger: "This vampire gets +1 stealth during actions
/// requiring Necromancy [nec]": a [nec] action card +1, a bleed 0.
#[test]
fn giuseppe_sneaks_his_vampires_necromancy_actions() {
    for nec in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Giovanni", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["nec"];
        e.add_minion(SeatId(0), plain("Giovanni", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        in_play(&mut e, 0, "Giuseppe, Gravedigger", Some(a));
        e.state.seats[0]
            .ash
            .put(find_library("Ascendance"), zvtes::copies::CopyId::NONE);
        hand(&mut e, 0, &["Whispers from the Dead"]);
        let choice = if nec { play(0, 0, a) } else { bleed(a) };
        assert_eq!(action_stealth(&mut e, choice), i32::from(nec), "{nec}");
    }
}

/// Leonardo, Mortician: "Leonardo can add 1 blood to a ready vampire as a
/// +1 stealth action": another Methuselah's, directed at them.
#[test]
fn leonardo_adds_a_blood_to_a_ready_vampire() {
    let mut e = game(2);
    let l = e.add_ally(SeatId(0), find_library("Leonardo, Mortician"));
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 2, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
    let card = body(&e, l);
    pick(&mut e, use_on(card, 0, Some(l), Target::Minion(z)));
    request(&mut e);
    assert_eq!(
        e.action().expect("an action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 3);
    let _ = v;
}

/// Masquer: "[nec] Masquer is immune to non-aggravated damage. The employer
/// gets +1 intercept. You can burn this retainer to give any minion +1
/// intercept. [NEC] As above, but Masquer has 2 life." Employed at each
/// level: its version's life; the employer +1 intercept.
/// Ledger: [RTR 19941109] [ANK 20180612].
#[test]
fn masquer_is_immune_and_gives_intercept() {
    for (levels, life) in [(vec!["nec"], 1), (vec!["nec", "nec"], 2)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Giovanni", 0), 4, false);
        e.state.minions[a.index()].gained_levels = levels.clone();
        e.add_minion(SeatId(0), plain("Giovanni", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Masquer"]);
        let mode = life - 1;
        pick(&mut e, play(0, mode, a));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let m = placed_on(&e, "Masquer", Some(a));
        assert_eq!(e.state.cards[m.index()].counters, life);
        assert_eq!(e.state.bearer_always(a, zvtes::cards::Stat::Intercept), 1);
    }
}

/// Masquer: "Immune means damage is not inflicted, even if non-preventable
/// or outside combat": its retainer layer marks it immune to non-aggravated
/// damage (`GameState::retainer_immune_non_aggravated`, read by every site
/// damaging a retainer).
/// Ledger: [LSJ 20010627] [LSJ 20010629] [LSJ 20090304-1].
#[test]
fn masquer_takes_no_normal_damage() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Giovanni", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let m = in_play(&mut e, 0, "Masquer", Some(a));
    assert!(e.state.retainer_immune_non_aggravated(m));
}

/// Masquer: "You can burn this retainer to give any minion +1 intercept":
/// another Methuselah's blocker short of the stealth reaches it.
#[test]
fn masquer_burns_to_give_another_blocker_intercept() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dom"];
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let g = e.add_minion(SeatId(2), plain("Giovanni", 0), 4, false);
    let m = in_play(&mut e, 2, "Masquer", Some(g));
    hand(&mut e, 0, &["Dominate Kine"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, block(z));
    let gift = use_on(m, 0, None, Target::None);
    let req = offering(&mut e, gift);
    assert_eq!(req.seat, SeatId(2));
    pick(&mut e, gift);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 2, "Masquer"));
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
}

/// Mummy's Tongue: "This vampire can burn this card during your master
/// phase to lock any vampire. The locked vampire does not unlock as normal
/// during their next unlock phase." An already locked vampire too (LSJ
/// 19980316): locked through its next unlock phase, unlocked after.
#[test]
fn mummys_tongue_stuns_any_vampire_in_the_master_phase() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[z.index()].locked = true;
    let t = in_play(&mut e, 0, "Mummy's Tongue", Some(a));
    hand(&mut e, 0, &["Ascendance"]);
    e.at_master_phase();
    pick(&mut e, use_on(t, 0, None, Target::Minion(z)));
    request(&mut e);
    assert!(ash_has(&e, 0, "Mummy's Tongue"));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].locked, "no unlock");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].locked);
}

/// Mummy's Tongue: "This vampire": an ally bearer (moved to it by an equip
/// from its bearer, clan equipment) cannot use it (LSJ 19990824).
/// Ledger: [ANK 20180719-3].
#[test]
fn mummys_tongue_is_not_offered_to_an_ally_bearer() {
    let mut e = game(2);
    let ally = e.add_ally(SeatId(0), find_library("Neighborhood Watch Commander"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let t = in_play(&mut e, 0, "Mummy's Tongue", Some(ally));
    hand(&mut e, 0, &["Ascendance"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(!req.options.contains(&use_on(t, 0, None, Target::Minion(z))));
}

/// Priestess of Sekhmet: "This Ministry gets +1 bleed against a Methuselah
/// who controls a ready Camarilla vampire. While the employer is bleeding,
/// Camarilla vampires get -1 intercept. A vampire can have only one
/// Priestess of Sekhmet."
#[test]
fn priestess_of_sekhmet_bleeds_camarilla_tables_and_strips_their_intercept() {
    for camarilla in [true, false] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), of_clan("Ministry", 4..=11, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = if camarilla {
            e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=11, 0), 4, false)
        } else {
            e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=11, 0), 4, false)
        };
        in_play(&mut e, 0, "Priestess of Sekhmet", Some(m));
        assert_eq!(
            e.state.bleed_against_mod(m, SeatId(1)),
            i32::from(camarilla)
        );
        pick(&mut e, bleed(m));
        request(&mut e);
        let act = e.action().expect("an action");
        assert_eq!(act.intercept_of(&e.state, z), -i32::from(camarilla));
    }
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), of_clan("Ministry", 4..=11, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Priestess of Sekhmet", Some(m));
    hand(&mut e, 0, &["Priestess of Sekhmet"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, m)), "only one");
}

/// Puppeteer: "[nec] Puppeteer can take control of an ally or a vampire
/// with capacity 4 or less until the end of your turn as a Ⓓ action": a
/// capacity-5 vampire not offered; taken, back at the end of the turn.
/// Ledger: [LSJ 20010119].
#[test]
fn puppeteer_borrows_a_small_vampire_until_the_turn_ends() {
    let mut e = game(2);
    let p = e.add_ally(SeatId(0), find_library("Puppeteer"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let small = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 4, false);
    let big = e.add_minion(SeatId(1), textless(|c| c.capacity == 5, 0), 4, false);
    let card = body(&e, p);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&use_on(card, 0, Some(p), Target::Minion(big))));
    pick(&mut e, use_on(card, 0, Some(p), Target::Minion(small)));
    request(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[small.index()].owner, SeatId(0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[small.index()].owner, SeatId(1));
}

/// Puppeteer: "If one is ousted while having temporary control of a minion,
/// the minion is removed from the game" and "removed from the game when he
/// should return to a Methuselah who has been ousted".
/// Ledger: [ANK 20181110] [LSJ 20081105].
#[test]
fn puppeteer_temporary_control_and_ousts() {
    let mut e = game(3);
    let p = e.add_ally(SeatId(0), find_library("Puppeteer"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let small = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    let card = body(&e, p);
    pick(&mut e, use_on(card, 0, Some(p), Target::Minion(small)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[small.index()].owner, SeatId(0));
    e.state.seats[1].ousted = true;
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert!(!e.state.minions[small.index()].in_play, "removed");
}

/// Puppeteer [NEC]: 2 life.
#[test]
fn puppeteer_superior_has_two_life() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Giovanni", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["nec", "nec"];
    e.add_minion(SeatId(0), plain("Giovanni", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Puppeteer"]);
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let p = e
        .state
        .minions
        .iter()
        .position(|v| v.in_play && v.is_ally())
        .expect("recruited");
    assert_eq!(e.state.minions[p].blood, 2);
}

/// Qetu the Evil Doer: "Qetu gets 1 optional press each combat, only usable
/// to end combat": not offered with no press standing; after the
/// opponent's press to continue (Flash), offered, and the combat ends after
/// one round (the blocked vampire hit once).
#[test]
fn qetu_presses_only_to_end_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["cel"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let q = e.add_ally(SeatId(1), find_library("Qetu the Evil Doer"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Flash"]);
    let press = use_on(body(&e, q), 0, None, Target::None);
    let flash = play(0, 0, a);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(q));
    loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&press), "no press standing");
        if req.window == WindowLabel::CombatPress && req.options.contains(&flash) {
            e.choose(index_of(&req, flash));
            break;
        }
        e.choose(fallback(&req));
    }
    pick_when(&mut e, press);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 3, "one round");
}

/// Smoke Grenade: "End combat as a strike, only usable at long range. Burn
/// after use": not offered at close range; at long range (Flash's
/// maneuver) it ends the combat and burns; the combat ended before strike
/// resolution (Rötschreck against a Flamethrower), it stays.
/// Ledger: [LSJ 20001127-2] [LSJ 20010806-1] [RTR 20020501] [LSJ 20040602].
#[test]
fn smoke_grenade_burns_unless_the_combat_ended_before_it() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let g = in_play(&mut e, 1, "Smoke Grenade", Some(z));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_on(g, 0, None, Target::None),
    );
    assert!(e.state.cards[g.index()].in_play, "close range");
    for rotschreck in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
        e.state.minions[a.index()].gained_levels = vec!["cel"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Flash"]);
        let flamer = in_play(&mut e, 0, "Flamethrower", Some(a));
        let g = in_play(&mut e, 1, "Smoke Grenade", Some(z));
        if rotschreck {
            hand(&mut e, 1, &["Rötschreck"]);
        }
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, 0, a));
        let strikes = [
            use_on(flamer, 0, None, Target::None),
            use_on(g, 0, None, Target::None),
        ];
        for s in strikes {
            pick_when(&mut e, s);
        }
        if rotschreck {
            pick_when(&mut e, master_at(0, 0, Target::Minion(z)));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.cards[g.index()].in_play, rotschreck, "{rotschreck}");
    }
}

/// Zombie: "Zombie inflicts 1 damage on the opposing minion each round of
/// combat during normal strike resolution at close range": environmental,
/// the retainer its source, through a dodge.
/// Ledger: [ANK 20180612] [ANK 20211127]; [RTR 19941109] [TOM 19951216]
/// [PIB 20140324] [ANK 20200422].
#[test]
fn zombie_bites_at_close_range_even_through_a_dodge() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Giovanni", 0), 4, false);
    e.add_minion(SeatId(0), plain("Giovanni", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["cel"];
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    in_play(&mut e, 0, "Zombie", Some(a));
    hand(&mut e, 1, &["Side Strike"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 3, "the Zombie's 1");
}

/// Zombie: "This vampire can burn this retainer to gain 2 blood as a +1
/// stealth action": +2 blood, the Zombie in the ash heap — a zombie
/// retainer burned: Cursed Abattoir's controller may add a counter (rules-qa
/// 55.6, corrected).
#[test]
fn zombie_burns_itself_for_two_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Giovanni", 0), 2, false);
    e.add_minion(SeatId(0), plain("Giovanni", 1), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let zom = in_play(&mut e, 0, "Zombie", Some(a));
    let abattoir = in_play(&mut e, 0, "Cursed Abattoir", None);
    let burn = use_on(zom, 0, Some(a), Target::None);
    assert_eq!(action_stealth(&mut e, burn), 1);
    let add = Choice::ResolveRider { card: abattoir.0 };
    pick_when(&mut e, add);
    request(&mut e);
    assert_eq!(blood(&e, a), 4);
    assert!(ash_has(&e, 0, "Zombie"));
    assert_eq!(e.state.cards[abattoir.index()].counters, 1);
}

/// Ashur-uballit: "Zombies … you employ get +1 starting life": a Zombie
/// retainer employed comes in with 3 (rules-qa 125.49, corrected).
#[test]
fn ashur_uballit_gives_an_employed_zombie_retainer_a_life() {
    let mut e = game(2);
    e.add_minion(
        SeatId(0),
        find_card("Ashur-uballit, Son of the Abyss"),
        4,
        false,
    );
    let a = e.add_minion(SeatId(0), plain("Giovanni", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Zombie"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let z = placed_on(&e, "Zombie", Some(a));
    assert_eq!(e.state.cards[z.index()].counters, 3);
}

// ---- Action cards (action_b) ------------------------------------------------

/// Khabar: Loyalty: "Add 2 blood to a younger Banu Haqim in your
/// uncontrolled region": an older one and a younger non-Banu Haqim not
/// offered.
#[test]
fn khabar_loyalty_feeds_a_younger_uncontrolled_banu_haqim() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Banu Haqim", 5..=6, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    uncontrolled(&mut e, 0, of_clan("Banu Haqim", 2..=4, 1), 0);
    uncontrolled(&mut e, 0, of_clan("Banu Haqim", 7..=11, 0), 0);
    uncontrolled(&mut e, 0, of_clan("Brujah", 2..=4, 0), 0);
    hand(&mut e, 0, &["Khabar: Loyalty"]);
    let at = |i| play_on(0, 0, a, Target::Uncontrolled(i));
    let req = request(&mut e);
    assert!(req.options.contains(&at(0)));
    assert!(!req.options.contains(&at(1)), "older");
    assert!(!req.options.contains(&at(2)), "no Banu Haqim");
    pick(&mut e, at(0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 2);
}

/// Lure of the Serpent: "[ser] Ⓓ Steal a retainer controlled by another
/// vampire. [SER] Ⓓ Steal an ally controlled by another Methuselah":
/// requirements do not apply to a card taken in play; no stealth.
/// Ledger: [TOM 19960226-1].
#[test]
fn lure_of_the_serpent_steals_a_retainer_or_an_ally_whatever_its_requirements() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["ser"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Giovanni", 0), 4, false);
    let zom = in_play(&mut e, 1, "Zombie", Some(z));
    hand(&mut e, 0, &["Lure of the Serpent"]);
    let steal = play_on(0, 0, a, Target::Card(zom));
    assert_eq!(action_stealth(&mut e, steal), 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.cards[zom.index()].host,
        Some(a),
        "a Giovanni retainer"
    );

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["ser", "ser"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Brigitte Gebauer"));
    hand(&mut e, 0, &["Lure of the Serpent"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(ally)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[ally.index()].owner, SeatId(0));
}

/// Madman's Quill [AUS][DEM]: on the prey's ready minion, the card itself;
/// "Vampires with Dementation [dem] get +1 bleed against the attached
/// minion's controller": a [dem] vampire's bleed of that Methuselah +1, a
/// non-[dem] one's none; a second one not offered while one is on a prey's
/// minion.
#[test]
fn madmans_quill_gives_dementation_vampires_bleed_against_its_minions_controller() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Malkavian", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["aus", "aus", "dem", "dem"];
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let z2 = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Madman's Quill", "Madman's Quill"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(z)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    placed_on(&e, "Madman's Quill", Some(z));
    assert_eq!(e.state.bleed_against_mod(a, SeatId(1)), 1);
    assert_eq!(e.state.bleed_against_mod(b, SeatId(1)), 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let req = request(&mut e);
    assert!(!req.options.contains(&play_on(0, 1, a, Target::Minion(z2))));
}

/// Madman's Quill: "Vampires can burn this card as an action that costs 2
/// blood": another Methuselah's vampire, directed at its controller, the 2
/// blood paid.
#[test]
fn madmans_quill_burns_for_two_blood_by_any_vampire() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    let q = in_play(&mut e, 2, "Madman's Quill", Some(z));
    let burn = use_on(q, 0, Some(a), Target::None);
    assert_eq!(directed_at(&mut e, burn), Some(SeatId(2)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 2, "Madman's Quill"));
    assert_eq!(blood(&e, a), 2);
}

/// Mark of Damnation: "Any minion in combat with that vampire gets +1
/// strength and gets one optional press to continue combat": the bearer's
/// blocker strikes for 2 ([SER]: 3) and holds a press to continue.
#[test]
fn mark_of_damnation_strengthens_and_presses_its_bearers_opponents() {
    for (mode, hit) in [(0, 2), (1, 3)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        in_play_mode(&mut e, 1, "Mark of Damnation", Some(a), mode);
        assert_eq!(e.state.strength_against(z, a), hit - 1);
        assert_eq!(e.state.continue_presses_vs(a), 1);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        // A press to continue, offered to the bearer's opponent.
        loop {
            let req = request(&mut e);
            if req.window == WindowLabel::CombatPress && req.options.contains(&Choice::UsePress) {
                assert_eq!(req.seat, SeatId(1));
                break;
            }
            e.choose(fallback(&req));
        }
        assert_eq!(blood(&e, a), 5 - hit as u8, "{mode}");
    }
}

/// Mark of Damnation: "The vampire with this card may burn this card as an
/// action."
#[test]
fn mark_of_damnation_bearer_burns_it_as_an_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let m = in_play(&mut e, 1, "Mark of Damnation", Some(a));
    pick(&mut e, use_on(m, 0, Some(a), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Mark of Damnation"));
}

/// Damage pending on side `side` of the combat under way: (normal,
/// aggravated).
fn pending(e: &Engine, side: usize) -> (u8, u8) {
    let p = e.combat().expect("combat").sides[side].pending;
    (p.normal, p.aggravated)
}

/// Seat 0's [for] bleeder (Relentless Reaper in hand) blocked by seat 1's
/// minion bearing `weapon`, seat 1 controlling `copies` Mass Reality
/// (mode `mode`); the weapon's strike chosen: the bleeder.
fn mass_reality_strike(weapon: &str, copies: usize, mode: u8) -> (Engine, MinionId) {
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let w = in_play(&mut e, 1, weapon, Some(z));
    for _ in 0..copies {
        in_play_mode(&mut e, 1, "Mass Reality", None, mode);
    }
    hand(&mut e, 0, &["Relentless Reaper"]);
    pick(&mut e, bleed(x));
    pick_when(&mut e, block(z));
    pick_when(&mut e, use_on(w, 0, None, Target::None));
    (e, x)
}

/// Mass Reality: "Minions you control inflict +1 damage with weapons each
/// strike": two copies stack, a .44 Magnum's 2 to 4; [CHI] +2.
/// Ledger: [LSJ 20020904].
#[test]
fn mass_reality_adds_to_its_controllers_weapon_strikes_and_stacks() {
    for (copies, mode, n) in [(0, 0, 2), (1, 0, 3), (2, 0, 4), (1, 1, 4)] {
        let (mut e, x) = mass_reality_strike(".44 Magnum", copies, mode);
        offering(&mut e, play(0, 0, x));
        assert_eq!(pending(&e, 0), (n, 0), "{copies} {mode}");
    }
}

/// Mass Reality: additional damage of the strike's kind, aggravated with a
/// Molotov Cocktail; none on a strike dealing none (a Flash Grenade).
/// Ledger: [RTR 19970630] [TOM 19960225]; [RTR 19960221].
#[test]
fn mass_reality_adds_aggravated_damage_and_none_to_a_flash_grenade() {
    let (mut e, x) = mass_reality_strike("Molotov Cocktail", 1, 0);
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 0), (0, 3));
    let (mut e, x) = mass_reality_strike("Flash Grenade", 2, 0);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
    assert_eq!(blood(&e, x), 4);
}

/// Patshiv: "Each ready unlocked Ravnos gains 1 blood": every Methuselah's,
/// the acting one (locked) and a locked one none, a torpid one none.
#[test]
fn patshiv_feeds_every_ready_unlocked_ravnos() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ravnos", 0), 2, false);
    let own = e.add_minion(SeatId(0), plain("Ravnos", 1), 2, false);
    let theirs = e.add_minion(SeatId(1), plain("Ravnos", 2), 2, false);
    let locked = e.add_minion(SeatId(1), plain("Ravnos", 3), 2, false);
    e.state.minions[locked.index()].locked = true;
    let torpid = e.add_minion(SeatId(1), plain("Ravnos", 4), 2, true);
    hand(&mut e, 0, &["Patshiv"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        [a, own, theirs, locked, torpid].map(|m| blood(&e, m)),
        [2, 3, 3, 2, 2]
    );
}

/// Phobia: "During your master phase, you can put 1 phobia counter on this
/// card" — once a phase; "If a minion you control blocks the attached
/// vampire, you can burn 5 phobia counters from this card before range is
/// determined to end combat": with 5, the controller's blocker's combat
/// ends before range, no damage; with 4, not offered.
#[test]
fn phobia_counts_master_phases_and_ends_its_blocked_bearers_combat() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let p = in_play(&mut e, 0, "Phobia", Some(z));
    hand(&mut e, 0, &["Ascendance"]);
    e.at_master_phase();
    pick(&mut e, ability(p.0, 0));
    let req = request(&mut e);
    assert!(!req.options.contains(&ability(p.0, 0)), "once");
    assert_eq!(e.state.cards[p.index()].counters, 1);
    for counters in [5, 4] {
        let mut e = game(2);
        let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let p = in_play(&mut e, 1, "Phobia", Some(b));
        e.state.cards[p.index()].counters = counters;
        pick(&mut e, bleed(b));
        pick_when(&mut e, block(z));
        let end = ability(p.0, 1);
        if counters == 4 {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, end);
            continue;
        }
        pick_when(&mut e, end);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!((blood(&e, b), blood(&e, z)), (4, 4), "no strike");
        assert_eq!(e.state.cards[p.index()].counters, 0);
    }
}

/// Reality [chi]: "This vampire cannot be the target of directed actions,
/// cannot perform actions except to burn this card, cannot block and cannot
/// cast votes or ballots. This vampire can burn this card as a +1 stealth
/// action." Another's Exile at it not offered, its own controller's
/// (undirected) still is (ANK 20200619); its only action the burn, at +1
/// stealth; no block.
#[test]
fn reality_bars_its_vampire_but_for_burning_it() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Toreador", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["pre"];
    let r = e.add_minion(SeatId(0), textless(|c| c.capacity <= 2, 0), 2, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity <= 2, 1), 2, false);
    let y = e.add_minion(SeatId(1), plain("Toreador", 1), 4, false);
    e.state.minions[y.index()].gained_levels = vec!["pre"];
    let card = in_play(&mut e, 0, "Reality", Some(r));
    hand(&mut e, 0, &["Exile"]);
    let req = request(&mut e);
    assert!(
        req.options.contains(&play_on(0, 0, a, Target::Minion(r))),
        "undirected"
    );
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::TakeAction { minion, .. } if *minion == r)));
    let burn = use_on(card, 0, Some(r), Target::None);
    assert!(req.options.contains(&burn));
    let _ = z;
    // Another Methuselah's directed action at it.
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let r = e.add_minion(SeatId(0), textless(|c| c.capacity <= 2, 0), 2, false);
    let y = e.add_minion(SeatId(1), plain("Toreador", 1), 4, false);
    e.state.minions[y.index()].gained_levels = vec!["pre"];
    in_play(&mut e, 0, "Reality", Some(r));
    hand(&mut e, 1, &["Exile"]);
    let exile = play_on(0, 0, y, Target::Minion(r));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, exile);
    // Blocked by no one: its own controller's bleed by another vampire.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let r = e.add_minion(SeatId(1), textless(|c| c.capacity <= 2, 0), 2, false);
    in_play(&mut e, 1, "Reality", Some(r));
    pick(&mut e, bleed(a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(r));
    // Its burn: +1 stealth.
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), textless(|c| c.capacity <= 2, 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = in_play(&mut e, 0, "Reality", Some(r));
    assert_eq!(
        action_stealth(&mut e, use_on(card, 0, Some(r), Target::None)),
        1
    );
}

/// Reality: two copies on one vampire bar every action, its burns too (each
/// copy bars all but burning itself; advanced rules §1.15, LSJ 20041103).
#[test]
fn reality_twice_leaves_its_vampire_no_action() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), textless(|c| c.capacity <= 2, 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let one = in_play(&mut e, 0, "Reality", Some(r));
    let two = in_play(&mut e, 0, "Reality", Some(r));
    let req = request(&mut e);
    for c in [one, two] {
        assert!(!req.options.contains(&use_on(c, 0, Some(r), Target::None)));
    }
}

/// Reality [CHI]: "Move a locked younger vampire to the uncontrolled
/// region"; its target unlocked by resolution, the action fizzles, nothing
/// paid (rules-qa 117.1).
/// Ledger: [ANK 20210124] [LSJ 20070411] [LSJ 20090514].
#[test]
fn reality_superior_banishes_a_locked_younger_vampire_or_fizzles() {
    for unlock in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 8, 0), 6, false);
        e.state.minions[a.index()].gained_levels = vec!["chi", "chi"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 0), 2, false);
        e.state.minions[z.index()].locked = true;
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Reality"]);
        pick(&mut e, play_on(0, 1, a, Target::Minion(z)));
        request(&mut e);
        if unlock {
            e.state.minions[z.index()].locked = false;
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[z.index()].in_play, unlock, "{unlock}");
        // A fizzle pays nothing (rules-qa 17.4, 117.1).
        assert_eq!(blood(&e, a), if unlock { 6 } else { 3 });
    }
}

/// Sensory Deprivation [chi]: "put this card on an ally. The attached
/// minion does not unlock as normal. Burn this card after this acting
/// vampire leaves the ready region." The card itself on the ally, which
/// stays locked through its unlock phase; the acting vampire (emptied by
/// the cost) sent to torpor by Amam's combat, the card burns.
#[test]
fn sensory_deprivation_holds_an_ally_until_its_vampire_leaves_ready() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 3, false);
    e.state.minions[a.index()].gained_levels = vec!["chi"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Neighborhood Watch Commander"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[ally.index()].locked = true;
    hand(&mut e, 0, &["Sensory Deprivation"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(ally)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let sd = placed_on(&e, "Sensory Deprivation", Some(ally));
    assert!(e.state.minions[ally.index()].locked, "not unlocked");
    assert_eq!(blood(&e, a), 0);
    // Amam, seat 1's, enters combat with it: 2 damage, torpor.
    let amam = e.add_ally(SeatId(1), find_library("Amam the Devourer"));
    let rush = use_on(body(&e, amam), 0, Some(amam), Target::Minion(a));
    pick(&mut e, rush);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].in_torpor);
    assert!(!e.state.cards[sd.index()].in_play);
    assert!(ash_has(&e, 0, "Sensory Deprivation"));
}

/// Sensory Deprivation: "Can target an unlocked minion, does not prevent a
/// minion to wake or unlock in a non-normal way": an unlocked vampire
/// ([CHI]) accepted; locked by its own bleed, Freak Drive unlocks it.
/// Ledger: [PIB 20150111].
#[test]
fn sensory_deprivation_leaves_wakes_and_freak_drive() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["chi", "chi"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Sensory Deprivation"]);
    pick(&mut e, play_on(0, 3, a, Target::Minion(z)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    placed_on(&e, "Sensory Deprivation", Some(z));
    hand(&mut e, 1, &["Freak Drive"]);
    pick(&mut e, bleed(z));
    assert!(e.state.minions[z.index()].locked);
    pick_when(&mut e, play(0, 0, z));
    request(&mut e);
    assert!(!e.state.minions[z.index()].locked, "Freak Drive");
}

/// Sensory Deprivation on Fata Amria's bearer: no burn instead of the
/// unlock it does not do as normal.
/// Ledger: [LSJ 20010814-3] [LSJ 20020109].
#[test]
fn sensory_deprivation_bars_fata_amrias_burn_at_unlock() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[x.index()].locked = true;
    let fata = in_play(&mut e, 0, "Fata Amria", Some(x));
    in_play_mode(&mut e, 0, "Sensory Deprivation", Some(x), 3);
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        Choice::BurnInPlay { card: fata.0 },
    );
    assert!(e.state.minions[x.index()].locked);
}

/// Sensory Deprivation on an infernal vampire: its controller may still pay
/// a pool to unlock it.
/// Ledger: [LSJ 20050114].
#[test]
fn sensory_deprivation_leaves_an_infernal_vampire_its_paid_unlock() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[x.index()].made_infernal = true;
    e.state.minions[x.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play_mode(&mut e, 0, "Sensory Deprivation", Some(x), 3);
    let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&Choice::UnlockInfernal { minion: x }));
}

/// Sensory Deprivation: "Burn a retainer": another Methuselah's, directed.
#[test]
fn sensory_deprivation_burns_a_retainer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["chi"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let spy = in_play(&mut e, 1, "Raven Spy", Some(z));
    hand(&mut e, 0, &["Sensory Deprivation"]);
    pick(&mut e, play_on(0, 0, a, Target::Card(spy)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Raven Spy"));
}

/// Sensory Overload: "Put this card on a ready minion; you still control
/// this card. The attached minion cannot perform actions or block. Burn
/// this card during their discard phase." The prey's minion: no action in
/// its turn, burned in its controller's discard phase, not in the player's.
#[test]
fn sensory_overload_bars_a_minion_until_its_controllers_discard_phase() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["chi"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Sensory Overload"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(z)));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let so = placed_on(&e, "Sensory Overload", Some(z));
    assert_eq!(e.state.card_controller(so), SeatId(0), "still yours");
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::TakeAction { minion, .. } if *minion == z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !e.state.cards[so.index()].in_play,
        "burned in their discard phase"
    );
}

/// Sensory Overload [CHI]: "burn this card during your unlock phase".
#[test]
fn sensory_overload_superior_lasts_until_your_unlock_phase() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let so = in_play_mode(&mut e, 0, "Sensory Overload", Some(z), 1);
    e.state.cards[so.index()].controller = SeatId(0);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.cards[so.index()].in_play, "not their discard phase");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[so.index()].in_play);
}

/// Sleight of Hand: "Look at another Methuselah's hand and discard all
/// equipment cards from it. [CHI] As above, and you can put one of the
/// non-location equipment cards discarded on this vampire": the equipment
/// discarded; [CHI] a location equipment not offered, the pick on the
/// vampire, its owner still the other Methuselah.
#[test]
fn sleight_of_hand_superior_puts_a_discarded_equipment_on_the_actor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ravnos", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["chi", "chi"];
    e.add_minion(SeatId(0), plain("Ravnos", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, &[".44 Magnum", "Ascendance", "Palatial Estate"]);
    e.state.seats[1].library = vec![find_library("Form of Mist"); 4].into();
    hand(&mut e, 0, &["Sleight of Hand"]);
    pick(&mut e, play_on(0, 1, a, Target::Seat(SeatId(1))));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    let magnum = find_library(".44 Magnum");
    assert!(ash_has(&e, 1, ".44 Magnum"));
    assert!(!e.state.seats[1].hand.contains(&magnum), "discarded");
    assert!(req.options.contains(&Choice::FindCard(magnum)));
    let estate = find_library("Palatial Estate");
    assert!(ash_has(&e, 1, "Palatial Estate"));
    assert!(
        !req.options.contains(&Choice::FindCard(estate)),
        "a location"
    );
    pick(&mut e, Choice::FindCard(magnum));
    request(&mut e);
    let g = placed_on(&e, ".44 Magnum", Some(a));
    assert_eq!(e.state.cards[g.index()].owner, SeatId(1));
    assert_eq!(blood(&e, a), 3, "Sleight's blood only");
}

/// Sleight of Hand is not an equip action at either level (owner,
/// 2026-11-20: the 2021 text no longer equips; TOM 19960130 predates it):
/// its [CHI] action is no equip action for the cards reading one.
#[test]
fn sleight_of_hand_is_not_an_equip_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ravnos", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["chi", "chi"];
    e.add_minion(SeatId(0), plain("Ravnos", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, &[".44 Magnum"]);
    hand(&mut e, 0, &["Sleight of Hand"]);
    pick(&mut e, play_on(0, 1, a, Target::Seat(SeatId(1))));
    request(&mut e);
    assert!(!e.action().expect("an action").is_equip_action(&e.state));
}

/// Soul Painting [aus][pre]: "Vampires with Auspex [aus] get +1 intercept
/// when attempting to block the vampire with this card": an Auspex blocker
/// +1, another none; [AUS][PRE]: every vampire; "A vampire can have only
/// one Soul Painting".
#[test]
fn soul_painting_gives_auspex_vampires_intercept_against_its_bearer() {
    for mode in [0, 1] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let aus = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[aus.index()].gained_levels = vec!["aus"];
        let not = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        in_play_mode(&mut e, 1, "Soul Painting", Some(a), mode);
        pick(&mut e, bleed(a));
        request(&mut e);
        let act = e.action().expect("an action");
        assert_eq!(act.intercept_of(&e.state, aus), 1, "{mode}");
        assert_eq!(act.intercept_of(&e.state, not), mode as i32, "{mode}");
    }
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Toreador", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["aus", "pre"];
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Soul Painting", Some(z));
    hand(&mut e, 0, &["Soul Painting"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play_on(0, 0, a, Target::Minion(z))),
        "only one"
    );
}

/// Soul Painting: "This vampire can burn this card as an action that costs
/// 1 blood."
#[test]
fn soul_painting_bearer_burns_it_for_a_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let p = in_play(&mut e, 1, "Soul Painting", Some(a));
    pick(&mut e, use_on(p, 0, Some(a), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Soul Painting"));
    assert_eq!(blood(&e, a), 3);
}

/// Summon Soul [nec]: "This vampire gains 2 blood."
#[test]
fn summon_soul_feeds_its_vampire_two_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Giovanni", 0), 1, false);
    e.state.minions[a.index()].gained_levels = vec!["nec"];
    e.add_minion(SeatId(0), plain("Giovanni", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Summon Soul"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 3);
}

/// Summon Soul [NEC]: "Remove this card from the game and shuffle up to two
/// library cards from your ash heap into your library": removed as it
/// resolves, Shroudsight cannot retrieve it; two picks shuffled in; blocked,
/// to the ash heap.
/// Ledger: [ANK 20170331].
#[test]
fn summon_soul_superior_leaves_the_game_beyond_shroudsight() {
    let soul = find_library("Summon Soul");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Giovanni", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["nec", "nec"];
    e.add_minion(SeatId(0), plain("Giovanni", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for c in ["Ascendance", "Conceal", "Dodge"] {
        e.state.seats[0]
            .ash
            .put(find_library(c), zvtes::copies::CopyId::NONE);
    }
    hand(&mut e, 0, &["Summon Soul", "Shroudsight"]);
    fill_hand(&mut e, 0);
    pick(&mut e, play(0, 1, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionAsAnnounced);
    let shroud = *req
        .options
        .iter()
        .find(|o| matches!(o, Choice::Play { mode: 0, .. }))
        .expect("Shroudsight offered");
    e.choose(index_of(&req, shroud));
    pick_when(&mut e, Choice::FindCard(find_library("Ascendance")));
    pick_when(&mut e, Choice::FindCard(find_library("Conceal")));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].removed.contains(&soul));
    assert!(!e.state.seats[0].library.contains(&soul));
    assert!(!ash_has(&e, 0, "Summon Soul"));
    // Shuffled in, then drawn up (the hand two short).
    for c in ["Ascendance", "Conceal"] {
        let c = find_library(c);
        let s = &e.state.seats[0];
        assert!(s.library.contains(&c) || s.hand.contains(&c), "{c}");
        assert!(!s.ash.contains(&c));
    }
    assert!(ash_has(&e, 0, "Dodge"));

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Giovanni", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["nec", "nec"];
    e.add_minion(SeatId(0), plain("Giovanni", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Raven Spy", Some(z));
    hand(&mut e, 0, &["Summon Soul"]);
    pick(&mut e, play(0, 1, a));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert!(ash_has(&e, 0, "Summon Soul"), "blocked");
}

/// Summon the Serpent: "Search your crypt for a vampire with Serpentis
/// [ser], reveal them, and move them to your uncontrolled region with 1
/// blood": only the crypt's Serpentis vampires offered.
#[test]
fn summon_the_serpent_finds_a_serpentis_vampire_with_a_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ministry", 4..=11, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ser = vampire_with("ser", 0);
    let none = plain("Brujah", 3);
    for c in [none, ser, none] {
        e.state.seats[0].crypt.put(c, zvtes::copies::CopyId::NONE);
    }
    hand(&mut e, 0, &["Summon the Serpent"]);
    pick(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Pass, Choice::AshVampire(ser)]);
    pick(&mut e, Choice::AshVampire(ser));
    request(&mut e);
    let slot = e.state.seats[0].uncontrolled.last().expect("moved");
    assert_eq!((slot.card, slot.counters), (ser, 1));
}

/// Thrones Crumble: "[ser] Ⓓ Burn 2 blood from a ready titled vampire.
/// [SER] As above, and lock the target vampire": an untitled one not
/// offered.
#[test]
fn thrones_crumble_bleeds_a_titled_vampire_and_superior_locks_it() {
    for mode in [0, 1] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["ser", "ser"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let p = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
        let u = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Thrones Crumble"]);
        let req = request(&mut e);
        assert!(!req
            .options
            .contains(&play_on(0, mode, a, Target::Minion(u))));
        pick(&mut e, play_on(0, mode, a, Target::Minion(p)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, p), 2);
        assert_eq!(e.state.minions[p.index()].locked, mode == 1);
    }
}

/// The Trick of the Danya: "Move 1 blood from an uncontrolled minion in
/// your prey's uncontrolled region to your pool or to a vampire in your
/// uncontrolled region": directed at the prey; both destinations.
#[test]
fn trick_of_the_danya_moves_a_prey_uncontrolled_blood_to_pool_or_vampire() {
    for to_pool in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Ravnos", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["chi"];
        e.add_minion(SeatId(0), plain("Ravnos", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        uncontrolled(&mut e, 1, plain("Ventrue", 2), 3);
        uncontrolled(&mut e, 0, plain("Ravnos", 2), 0);
        hand(&mut e, 0, &["Trick of the Danya, The"]);
        let choice = if to_pool {
            play_on(0, 0, a, Target::None)
        } else {
            play_on(0, 1, a, Target::Uncontrolled(0))
        };
        pick(&mut e, choice);
        request(&mut e);
        assert_eq!(
            e.action().expect("an action").directed_seat(&e.state),
            Some(SeatId(1))
        );
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].uncontrolled[0].counters, 2);
        if to_pool {
            assert_eq!(e.state.seats[0].pool, 11);
        } else {
            assert_eq!(e.state.seats[0].uncontrolled[0].counters, 1);
        }
    }
}

/// Violation of Trust: "Burn a contract on a minion you control": another
/// Methuselah's Contract on the player's minion, directed at that
/// Methuselah; a non-contract card not offered.
#[test]
fn violation_of_trust_burns_a_contract_on_an_own_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let m = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let c = in_play(&mut e, 1, "Contract", Some(m));
    let other = in_play(&mut e, 1, "True Faith", Some(m));
    hand(&mut e, 0, &["Violation of Trust"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play_on(0, 0, a, Target::Card(other))));
    pick(&mut e, play_on(0, 0, a, Target::Card(c)));
    request(&mut e);
    assert_eq!(
        e.action().expect("an action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Contract"));
}

// ---- Vampires (vamp_c) ------------------------------------------------------

/// The crypt card `name` at the `adv` level.
#[track_caller]
fn crypt_card(name: &str, adv: bool) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && c.adv == adv)
        .unwrap_or_else(|| panic!("no {name}")) as u16
}

/// The round's text-only vampires resolve to their crypt cards, no
/// definition, their traits generated: sect, blood cursed (Harika Guljan,
/// Fida'i), non-unique (Fida'i), priscus (Gisela, Gustav), 2 votes
/// (Ezmerelda), Flight and slave (Fidus).
#[test]
fn the_round_35_text_only_vampires_carry_their_printed_traits() {
    for name in [
        "Francesca Giovanni",
        "George Frederick",
        "Gloria Giovanni",
        "Harika Guljan",
        "Béatrice L'Angou",
        "Celine Chevalier",
        "Count Ormonde",
        "Doris McMillon",
    ] {
        let c = crypt_card(name, false);
        assert!(zvtes::cards::vampire(c).is_none(), "{name}");
    }
    assert_eq!(
        CRYPT[crypt_card("George Frederick", false) as usize].sect,
        Some(Sect::Camarilla)
    );
    assert!(CRYPT[crypt_card("Harika Guljan", false) as usize].blood_cursed);
    for (name, sect) in [
        ("Béatrice L'Angou", Some(Sect::Sabbat)),
        ("Doris McMillon", Some(Sect::Sabbat)),
        ("Celine Chevalier", Some(Sect::Independent)),
        ("Count Ormonde", Some(Sect::Independent)),
    ] {
        assert_eq!(CRYPT[crypt_card(name, false) as usize].sect, sect, "{name}");
    }
    let fidai = &CRYPT[crypt_card("Fida'i", false) as usize];
    assert!(fidai.blood_cursed && !fidai.unique);
    for name in ["Gisela Harden, The Winnower", "Gustav Mallenhous"] {
        assert_eq!(
            CRYPT[crypt_card(name, false) as usize].title,
            Title::Priscus
        );
    }
    assert_eq!(
        CRYPT[crypt_card("Ezmerelda", false) as usize].title,
        Title::Votes(2)
    );
    let fidus = &CRYPT[crypt_card("Fidus, The Shrunken Beast", false) as usize];
    assert_eq!(fidus.slave, Some("Tremere"));
    assert!(fidus.disciplines.contains(&"FLIGHT") || fidus.disciplines.contains(&"flight"));
}

/// Ezmerelda: "If Ezmerelda is ready during your unlock phase, you can
/// remove her from the game to gain 11 pool. +1 bleed." Ready: removed,
/// +11; in torpor never offered.
#[test]
fn ezmerelda_leaves_the_game_for_eleven_pool() {
    for torpor in [false, true] {
        let mut e = game(2);
        let z = e.add_minion(SeatId(0), crypt_card("Ezmerelda", false), 4, torpor);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        assert_eq!(
            e.state
                .printed_mod(z, zvtes::cards::Stat::Bleed, Default::default()),
            1
        );
        let leave = Choice::UseAbility {
            source: AbilitySource::Vampire(z),
            ability: 0,
            actor: None,
            target: Target::None,
        };
        pick(&mut e, Choice::Pass);
        if torpor {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, leave);
            continue;
        }
        pick_when(&mut e, leave);
        request(&mut e);
        assert!(!e.state.minions[z.index()].in_play);
        assert!(e.state.seats[0]
            .removed_crypt
            .contains(&crypt_card("Ezmerelda", false)));
        assert_eq!(e.state.seats[0].pool, 21);
    }
}

/// Ezmerelda made Anarch keeps her 2 votes (titled).
/// Ledger: [LSJ 20010714].
#[test]
fn round35_personal_titles_survive_a_sect_change() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(0), crypt_card("Ezmerelda", false), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].sect_override = Some(Sect::Anarch);
    assert_eq!(e.state.title_of(z), Title::Votes(2));
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, caller));
    pick(&mut e, alloc(0, 3));
    to_window(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, z), 2);
}

/// Ferox: "Ferox can enter combat with a Nosferatu controlled by another
/// Methuselah as a Ⓓ action. Younger vampires must burn 1 blood to attempt
/// to block Ferox. Ferox cannot commit diablerie." Offered at another's
/// Nosferatu, not a Ventrue nor his own Nosferatu; a younger blocker pays 1.
#[test]
fn ferox_rushes_a_nosferatu_and_tolls_younger_blockers() {
    let mut e = game(2);
    let f = e.add_minion(
        SeatId(0),
        crypt_card("Ferox, The Rock Lord", false),
        6,
        false,
    );
    let own = e.add_minion(SeatId(0), of_clan("Nosferatu", 1..=11, 0), 4, false);
    let nos = e.add_minion(SeatId(1), of_clan("Nosferatu", 1..=4, 1), 4, false);
    let ven = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert!(e.state.cannot_diablerize(f));
    let at = |m| Choice::UseAbility {
        source: AbilitySource::Vampire(f),
        ability: 0,
        actor: Some(f),
        target: Target::Minion(m),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&at(nos)));
    assert!(!req.options.contains(&at(ven)));
    assert!(!req.options.contains(&at(own)));
    pick(&mut e, bleed(f));
    let before = blood(&e, nos);
    pick_when(&mut e, block(nos));
    assert_eq!(blood(&e, nos), before - 1, "younger: the toll");
}

/// Merged Ferox: "[MERGED] Ferox gets +1 intercept when attempting to block
/// directed actions": +1 against a directed bleed; the base rush kept.
#[test]
fn ferox_merged_intercepts_directed_actions() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let f = e.add_minion(
        SeatId(1),
        crypt_card("Ferox, The Rock Lord", false),
        6,
        false,
    );
    e.state.minions[f.index()].merged = Some(crypt_card("Ferox, The Rock Lord", true));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(a));
    request(&mut e);
    assert_eq!(e.action().expect("an action").intercept_of(&e.state, f), 1);
    let d = zvtes::cards::printing(&e.state.minions[f.index()], zvtes::cards::Stat::Intercept);
    assert!(d.is_some_and(|d| d.adv && !d.abilities.is_empty()));
}

/// Fida'i: "Fida'i do not unlock as normal. During your unlock phase, any
/// ready Banu Haqim you control with capacity 7 or more can burn 1 blood to
/// unlock this Fida'i." Still locked after the unlock step; a capacity-7
/// Banu Haqim of his controller's pays, a smaller one no payer.
#[test]
fn fidai_stays_locked_unless_a_great_banu_haqim_pays() {
    let mut e = game(2);
    let fi = e.add_minion(SeatId(0), crypt_card("Fida'i", false), 2, false);
    e.state.minions[fi.index()].locked = true;
    let big = e.add_minion(SeatId(0), of_clan("Banu Haqim", 7..=11, 0), 4, false);
    let small = e.add_minion(SeatId(0), of_clan("Banu Haqim", 1..=6, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.active = SeatId(1);
    let use_by = |m| Choice::UseAbility {
        source: AbilitySource::Vampire(fi),
        ability: 0,
        actor: None,
        target: Target::Minion(m),
    };
    let req = to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let _ = req;
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(e.state.minions[fi.index()].locked, "not as normal");
    assert!(req.options.contains(&use_by(big)));
    assert!(!req.options.contains(&use_by(small)));
    pick(&mut e, use_by(big));
    request(&mut e);
    assert!(!e.state.minions[fi.index()].locked);
    assert_eq!(blood(&e, big), 3);
}

/// Fidus: "+1 stealth on undirected actions. -1 strength": a hunt at +1, a
/// bleed at 0; strength 0.
#[test]
fn fidus_sneaks_undirected_and_hits_softly() {
    for hunt in [true, false] {
        let mut e = game(2);
        let f = e.add_minion(
            SeatId(0),
            crypt_card("Fidus, The Shrunken Beast", false),
            1,
            false,
        );
        e.add_minion(SeatId(0), plain("Tremere", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let choice = if hunt {
            action(f, ActionKind::Hunt)
        } else {
            bleed(f)
        };
        assert_eq!(
            action_stealth(&mut e, choice),
            i32::from(hunt) + i32::from(hunt)
        );
        let strength = zvtes::cards::Stat::Strength;
        assert_eq!(e.state.printed_mod(f, strength, Default::default()), -1);
    }
}

/// Gabrin: "Cards requiring Chimerstry [chi] cost Gabrin 1 fewer blood. He
/// can lock an ally or a younger vampire as a +1 stealth Ⓓ action": a
/// younger vampire locked; an older one not offered; a [chi] card 1 blood
/// less.
#[test]
fn gabrin_locks_a_younger_vampire_cheaply() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), crypt_card("Gabrin", false), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let cap = CRYPT[crypt_card("Gabrin", false) as usize].capacity;
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity < cap, 0), 2, false);
    let old = e.add_minion(SeatId(1), textless(|c| c.capacity > cap, 0), 4, false);
    let lock = |m| Choice::UseAbility {
        source: AbilitySource::Vampire(g),
        ability: 0,
        actor: Some(g),
        target: Target::Minion(m),
    };
    assert_eq!(
        e.state
            .printed_cost_off(g, find_library("Sensory Overload"), 0)
            .blood,
        1
    );
    let req = request(&mut e);
    assert!(!req.options.contains(&lock(old)));
    assert_eq!(action_stealth(&mut e, lock(young)), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[young.index()].locked);
}

/// Gaspare Giovanni: "During your master phase, Gaspare can burn 2 blood to
/// move a vampire from your ash heap to the top of your crypt": named as
/// used, on top, 2 blood; no master phase action spent (LSJ 19970724).
#[test]
fn gaspare_returns_an_ash_vampire_to_the_crypt_top() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), crypt_card("Gaspare Giovanni", false), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let dead = plain("Brujah", 2);
    let other = plain("Brujah", 3);
    for c in [dead, other] {
        e.state.seats[0]
            .ash_crypt
            .put(c, zvtes::copies::CopyId::NONE);
    }
    e.state.seats[0]
        .crypt
        .put(plain("Ventrue", 3), zvtes::copies::CopyId::NONE);
    hand(&mut e, 0, &["Ascendance"]);
    e.at_master_phase();
    let use_it = Choice::UseAbility {
        source: AbilitySource::Vampire(g),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick(&mut e, use_it);
    pick(&mut e, Choice::AshVampire(dead));
    let req = request(&mut e);
    assert!(req.options.contains(&master_play(0)), "no MPA spent");
    assert_eq!(e.state.seats[0].crypt.to_cards().last(), Some(&dead));
    assert!(!e.state.seats[0].ash_crypt.contains(&dead));
    assert_eq!(blood(&e, g), 2);
}

/// Gaël Pilet: "Gaël can never have more than 1 intercept; any additional
/// intercept she gains is lost": Raven Spy's +1 and a printed +1 leave her
/// at 1.
#[test]
fn gael_pilet_never_intercepts_above_one() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let g = e.add_minion(SeatId(1), crypt_card("Gaël Pilet", false), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Raven Spy", Some(g));
    in_play(&mut e, 1, "Mr. Winthrop", Some(g));
    pick(&mut e, bleed(a));
    request(&mut e);
    assert_eq!(e.action().expect("an action").intercept_of(&e.state, g), 1);
}

/// Gaël Pilet's cap with a reduction (owner 2026-11-20, Q5: summed, then
/// capped): Nightshades [obt] (-1) on her, Raven Spy and Mr. Winthrop (+1
/// each): 1, not the literal 0.
#[test]
fn gael_pilet_sums_then_caps_with_nightshades() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        fixture_vampire(|c| c.capacity >= 7 && c.title == Title::None, 0),
        4,
        false,
    );
    e.state.minions[a.index()].gained_levels = vec!["obt"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let g = e.add_minion(SeatId(1), crypt_card("Gaël Pilet", false), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Raven Spy", Some(g));
    in_play(&mut e, 1, "Mr. Winthrop", Some(g));
    hand(&mut e, 0, &["Nightshades"]);
    pick(&mut e, bleed(a));
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::Minion(g),
        },
    );
    request(&mut e);
    assert!(
        !e.state.seats[0].hand.contains(&find_library("Nightshades")),
        "played"
    );
    assert_eq!(e.action().expect("an action").intercept_of(&e.state, g), 1);
}

/// Gillespi Giovanni: "+1 bleed against a Methuselah who controls a ready
/// Camarilla vampire", read live (rules-qa 109.37).
#[test]
fn gillespi_bleeds_harder_at_camarilla_holders() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), crypt_card("Gillespi Giovanni", false), 4, false);
    let cam = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 1..=11, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    pick(&mut e, bleed(g));
    request(&mut e);
    assert_eq!(e.action().expect("an action").bleed_amount(&e.state), 2);
    e.state.minions[cam.index()].in_torpor = true;
    assert_eq!(e.action().expect("an action").bleed_amount(&e.state), 1);
}

/// Gisela Harden: "Gisela can burn an uncontrolled minion in your prey's
/// uncontrolled region as a Ⓓ action that costs X blood, where X is the
/// amount of blood on that card, any blood on that card is returned to your
/// prey's pool": a card with more blood than hers not offered; the named
/// one burned to its owner's ash heap, her 2 blood to the prey's pool;
/// directed at the prey.
#[test]
fn gisela_burns_an_uncontrolled_vampire_for_its_blood() {
    let mut e = game(2);
    let g = e.add_minion(
        SeatId(0),
        crypt_card("Gisela Harden, The Winnower", false),
        3,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cheap = plain("Ventrue", 2);
    let dear = plain("Ventrue", 3);
    uncontrolled(&mut e, 1, cheap, 2);
    uncontrolled(&mut e, 1, dear, 4);
    uncontrolled(&mut e, 1, plain("Ventrue", 4), 3);
    let burn = Choice::UseAbility {
        source: AbilitySource::Vampire(g),
        ability: 0,
        actor: Some(g),
        target: Target::Seat(SeatId(1)),
    };
    pick(&mut e, burn);
    let req = request(&mut e);
    assert_eq!(
        req.options,
        vec![Choice::Slot(0), Choice::Slot(2)],
        "only what she can pay"
    );
    pick(&mut e, Choice::Slot(0));
    request(&mut e);
    assert_eq!(
        e.action().expect("an action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ash_crypt.contains(&cheap));
    assert_eq!(e.state.seats[1].uncontrolled.len(), 2);
    assert_eq!(e.state.seats[1].pool, 12);
    assert_eq!(blood(&e, g), 1);
}

/// Gisela Harden: "Gisela can burn an uncontrolled vamp with 0 blood on it
/// for free" (LSJ 20040608); the blood grown past hers by resolution, the
/// cost unpayable: she pays what she can, her 1 blood, and the action
/// fizzles, nothing burned (rules-qa 3.6, 117.1, 122.177).
#[test]
fn gisela_burns_an_empty_one_free_and_fizzles_when_the_blood_grew() {
    for grown in [false, true] {
        let mut e = game(2);
        let g = e.add_minion(
            SeatId(0),
            crypt_card("Gisela Harden, The Winnower", false),
            1,
            false,
        );
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let empty = plain("Ventrue", 2);
        uncontrolled(&mut e, 1, empty, 0);
        uncontrolled(&mut e, 1, plain("Ventrue", 3), 0);
        let burn = Choice::UseAbility {
            source: AbilitySource::Vampire(g),
            ability: 0,
            actor: Some(g),
            target: Target::Seat(SeatId(1)),
        };
        pick(&mut e, burn);
        pick(&mut e, Choice::Slot(0));
        request(&mut e);
        if grown {
            e.state.seats[1].uncontrolled[0].counters = 2;
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].ash_crypt.contains(&empty), !grown);
        assert_eq!(
            e.state.seats[1].uncontrolled[0].counters,
            if grown { 2 } else { 0 }
        );
        assert_eq!(
            blood(&e, g),
            if grown { 0 } else { 1 },
            "free, or what she can"
        );
    }
}

/// Gustav Mallenhous: "Gustav can remove a bishop's or archbishop's title
/// as a Ⓓ action. +1 bleed": another's archbishop untitled; a cardinal not
/// offered.
#[test]
fn gustav_mallenhous_strips_an_archbishop() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), crypt_card("Gustav Mallenhous", false), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let arch = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=11, 0), 4, false);
    e.state.minions[arch.index()].granted_title = Some(Title::Archbishop("Paris"));
    let card = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=11, 1), 4, false);
    e.state.minions[card.index()].granted_title = Some(Title::Cardinal);
    let strip = |m| Choice::UseAbility {
        source: AbilitySource::Vampire(g),
        ability: 0,
        actor: Some(g),
        target: Target::Minion(m),
    };
    let req = request(&mut e);
    assert!(!req.options.contains(&strip(card)));
    pick(&mut e, strip(arch));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.title_of(arch), Title::None);
}

/// Haakon Mortensen: "During your unlock phase, you can move 1 blood from
/// Haakon to any Gangrel or Gangrel antitribu in your uncontrolled region":
/// both clans offered, a Ventrue not.
#[test]
fn haakon_feeds_an_uncontrolled_gangrel_antitribu() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), crypt_card("Haakon Mortensen", false), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    uncontrolled(&mut e, 0, of_clan("Gangrel", 1..=11, 0), 0);
    uncontrolled(&mut e, 0, of_clan("Gangrel antitribu", 1..=11, 0), 0);
    uncontrolled(&mut e, 0, plain("Ventrue", 2), 0);
    let to = |i| Choice::UseAbility {
        source: AbilitySource::Vampire(h),
        ability: 0,
        actor: None,
        target: Target::Uncontrolled(i),
    };
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let req = if req.options.contains(&to(0)) {
        req
    } else {
        pick(&mut e, Choice::Pass);
        until(&mut e, SeatId(0), WindowLabel::UnlockAbilities)
    };
    assert!(req.options.contains(&to(0)) && req.options.contains(&to(1)));
    assert!(!req.options.contains(&to(2)));
    pick(&mut e, to(1));
    request(&mut e);
    assert_eq!(e.state.seats[0].uncontrolled[1].counters, 1);
    assert_eq!(blood(&e, h), 3);
}

/// Haakon Mortensen: "Haakon can take an action to move 1 blood from the
/// blood bank to any unlocked vampire": the target locked by resolution,
/// the action fizzles; unlocked, +1.
/// Ledger: [ANK 20210124] [LSJ 20070411] [LSJ 20090514].
#[test]
fn haakon_mortensen_fizzles_on_a_target_locked_by_resolution() {
    for locked in [false, true] {
        let mut e = game(2);
        let h = e.add_minion(SeatId(0), crypt_card("Haakon Mortensen", false), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
        pick(
            &mut e,
            Choice::UseAbility {
                source: AbilitySource::Vampire(h),
                ability: 1,
                actor: Some(h),
                target: Target::Minion(z),
            },
        );
        request(&mut e);
        e.state.minions[z.index()].locked = locked;
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, z), if locked { 2 } else { 3 }, "{locked}");
    }
}

/// Hadrian Garrick: "+1 strength."
#[test]
fn hadrian_garrick_strikes_harder() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), crypt_card("Hadrian Garrick", false), 4, false);
    let strength = zvtes::cards::Stat::Strength;
    assert_eq!(e.state.printed_mod(h, strength, Default::default()), 1);
}

// ---- Vampires (vamp_b) ------------------------------------------------------

/// The printed strength `m` gets in combat with `foe`.
fn strength_vs(e: &Engine, m: MinionId, foe: MinionId) -> i32 {
    let ctx = zvtes::state::PrintedCtx {
        opponent: Some(foe),
        ..Default::default()
    };
    e.state.printed_mod(m, zvtes::cards::Stat::Strength, ctx)
}

/// Blanche Hill: "Once each turn, Blanche can unlock after successfully
/// hunting": offered as the hunt ends, taken; a second hunt the same turn,
/// never.
#[test]
fn blanche_hill_may_unlock_after_a_successful_hunt() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), crypt_card("Blanche Hill", false), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    pick(&mut e, action(b, ActionKind::Hunt));
    pick_when(&mut e, Choice::ResolvePrinted { minion: b });
    request(&mut e);
    assert!(!e.state.minions[b.index()].locked);
    pick(&mut e, action(b, ActionKind::Hunt));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::ResolvePrinted { minion: b },
    );
    assert!(e.state.minions[b.index()].locked, "once each turn");
}

/// Blanche Hill: the first hunt's unlock declined, then unlocked another
/// way and hunting again under Vampiric Disease ("cannot gain blood by
/// hunting"): no successful hunt (rules-qa 59.4, 89.26), her unlock not
/// offered again for it.
#[test]
fn blanche_hill_no_unlock_after_a_second_hunt_gaining_nothing() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), crypt_card("Blanche Hill", false), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    pick(&mut e, action(b, ActionKind::Hunt));
    let req = offering(&mut e, Choice::ResolvePrinted { minion: b });
    e.choose(index_of(&req, Choice::Pass));
    request(&mut e);
    assert!(e.state.minions[b.index()].locked, "declined");
    e.state.minions[b.index()].locked = false;
    e.state.minions[b.index()].disease = true;
    pick(&mut e, action(b, ActionKind::Hunt));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::ResolvePrinted { minion: b },
    );
}

/// Carlotta Giovanni: Lenelle's exchange; both cards named as announced.
/// Ledger: [LSJ 20000222] [LSJ 20011023].
#[test]
fn carlotta_names_the_cards_she_exchanges_as_she_announces() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), crypt_card("Carlotta Giovanni", false), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    let kevlar = find_library("Kevlar Vest");
    let rats = find_library("Army of Rats");
    hand(&mut e, 0, &["Army of Rats", "Blood Doll"]);
    e.state.seats[0].ash.extend([kevlar]);
    let exchange = Choice::UseAbility {
        source: AbilitySource::Vampire(c),
        ability: 0,
        actor: Some(c),
        target: Target::None,
    };
    pick(&mut e, exchange);
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::ActionExchange);
    pick(&mut e, Choice::Discard(0));
    let a = e.action().expect("action");
    assert_eq!((a.exchange, a.stealth(&e.state)), (Some((kevlar, rats)), 1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].ash, vec![rats]);
    assert!(e.state.seats[0].hand.contains(&kevlar));
    assert_eq!(blood(&e, c), 3);
}

/// Chas Giovanni Tello: "As Chas announces an action, any other Methuselah
/// can discard one master card as an out-of-turn master to have the action
/// fail": the bleed fails, Chas still locked, the master in the user's ash
/// heap, their out-of-turn master spent; with one already spent, or to his
/// own Methuselah, never offered.
#[test]
fn chas_giovanni_tello_action_fails_for_a_discarded_master() {
    for spent in [false, true] {
        let mut e = game(2);
        let c = e.add_minion(
            SeatId(0),
            crypt_card("Chas Giovanni Tello", false),
            4,
            false,
        );
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
        hand(&mut e, 0, &["Blood Doll"]);
        hand(&mut e, 1, &["Blood Doll"]);
        if spent {
            e.state.seats[1].out_of_turn = Some(zvtes::state::OutOfTurn { trifle: false });
        }
        let fail = Choice::UseAbility {
            source: AbilitySource::Vampire(c),
            ability: 0,
            actor: None,
            target: Target::None,
        };
        pick(&mut e, bleed(c));
        if spent {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, fail);
            continue;
        }
        let req = request(&mut e);
        assert!(req.seat != SeatId(0) || !req.options.contains(&fail));
        pick_when(&mut e, fail);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[c.index()].locked);
        assert_eq!(e.state.seats[1].pool, 10, "failed");
        assert!(ash_has(&e, 1, "Blood Doll"));
        assert!(e.state.seats[1].out_of_turn.is_some());
    }
}

/// Chas Giovanni Tello: "He gets +1 strength in combat with a titled
/// vampire": a prince, not an untitled one.
#[test]
fn chas_giovanni_tello_strikes_harder_against_a_titled_vampire() {
    let mut e = game(2);
    let c = e.add_minion(
        SeatId(0),
        crypt_card("Chas Giovanni Tello", false),
        4,
        false,
    );
    let prince = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 1..=11, 0), 4, false);
    e.state.minions[prince.index()].granted_title = Some(Title::Prince("Paris"));
    let plain_one = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 1..=11, 1), 4, false);
    assert_eq!(strength_vs(&e, c, prince), 1);
    assert_eq!(strength_vs(&e, c, plain_one), 0);
}

/// Cynthia Ingold, Dedefra: "+1 strength in combat with a Camarilla
/// vampire"; Cynthia sterile.
#[test]
fn round_35_strength_against_camarilla_vampires() {
    let mut e = game(2);
    let cam = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 1..=11, 0), 4, false);
    let sab = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 1..=11, 0), 4, false);
    for name in ["Cynthia Ingold", "Dedefra"] {
        let v = e.add_minion(SeatId(0), crypt_card(name, false), 4, false);
        assert_eq!(strength_vs(&e, v, cam), 1, "{name}");
        assert_eq!(strength_vs(&e, v, sab), 0, "{name}");
    }
    assert!(CRYPT[crypt_card("Cynthia Ingold", false) as usize].sterile);
}

/// Céleste, The Voice of a Secret: "+1 bleed against Methuselahs who
/// control a ready Toreador. She gets -1 intercept during bleed actions."
#[test]
fn celeste_bleeds_harder_at_a_toreador_controller_and_intercepts_less_on_bleeds() {
    for toreador in [true, false] {
        let mut e = game(2);
        let c = e.add_minion(
            SeatId(0),
            crypt_card("Céleste, The Voice of a Secret", false),
            3,
            false,
        );
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let clan = if toreador { "Toreador" } else { "Ventrue" };
        e.add_minion(SeatId(1), of_clan(clan, 1..=11, 0), 4, false);
        pick(&mut e, bleed(c));
        request(&mut e);
        let a = e.action().expect("an action");
        assert_eq!(a.bleed_amount(&e.state), 1 + u8::from(toreador));
    }
    for (bleeds, intercept) in [(true, -1), (false, 0)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let c = e.add_minion(
            SeatId(1),
            crypt_card("Céleste, The Voice of a Secret", false),
            3,
            false,
        );
        pick(
            &mut e,
            if bleeds {
                bleed(a)
            } else {
                action(a, ActionKind::Hunt)
            },
        );
        request(&mut e);
        assert_eq!(
            e.action().expect("an action").intercept_of(&e.state, c),
            intercept
        );
    }
}

/// Danny Larkshill: "+1 strength in combat with a younger vampire"; Delilah
/// Monroe: "-1 intercept against older vampires".
#[test]
fn round_35_age_conditioned_vampires() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), crypt_card("Danny Larkshill", false), 4, false);
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity == 3, 0), 3, false);
    let old = e.add_minion(SeatId(1), textless(|c| c.capacity == 9, 0), 4, false);
    assert_eq!(strength_vs(&e, d, young), 1);
    assert_eq!(strength_vs(&e, d, old), 0);
    for (cap, intercept) in [(7, -1), (2, 0)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity == cap, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let dl = e.add_minion(SeatId(1), crypt_card("Delilah Monroe", false), 4, false);
        pick(&mut e, bleed(a));
        request(&mut e);
        assert_eq!(
            e.action().expect("an action").intercept_of(&e.state, dl),
            intercept
        );
    }
}

/// Darius Styx: "Darius can look at 1 random card from another
/// Methuselah's hand as a Ⓓ action; if it is an equipment or retainer card,
/// put it on Darius (ignore requirements and cost)": an equipment and a
/// retainer put on him, their owner's still, the hand replaced; a master
/// card stays; never at his own Methuselah.
#[test]
fn darius_styx_takes_an_equipment_from_another_hand() {
    for (name, taken) in [
        ("Laptop Computer", true),
        ("Mr. Winthrop", true),
        ("Blood Doll", false),
    ] {
        let mut e = game(2);
        let d = e.add_minion(SeatId(0), crypt_card("Darius Styx", false), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
        hand(&mut e, 1, &[name]);
        e.state.seats[1]
            .library
            .put(find_library("Army of Rats"), zvtes::copies::CopyId::NONE);
        let at = |s| Choice::UseAbility {
            source: AbilitySource::Vampire(d),
            ability: 0,
            actor: Some(d),
            target: Target::Seat(s),
        };
        let req = request(&mut e);
        assert!(!req.options.contains(&at(SeatId(0))));
        pick(&mut e, at(SeatId(1)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let held = card_in_play(&e, name);
        assert_eq!(held.is_some(), taken, "{name}");
        if let Some(id) = held {
            let c = &e.state.cards[id.index()];
            assert_eq!((c.host, c.owner), (Some(d), SeatId(1)));
            assert_eq!(blood(&e, d), 4, "no cost");
        }
        let replaced = find_library(if taken { "Army of Rats" } else { name });
        assert!(e.state.seats[1].hand.contains(&replaced), "{name}");
    }
}

/// Darius Styx: "If the equipment cannot be legally placed on the vampire
/// (e.g. a second vehicle or a second Laptop Computer), it is burned
/// without cost." Ledger: [TOM 19950407].
#[test]
fn darius_styx_burns_an_equipment_he_cannot_hold() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), crypt_card("Darius Styx", false), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    in_play(&mut e, 0, "Laptop Computer", Some(d));
    hand(&mut e, 1, &["Laptop Computer"]);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(d),
            ability: 0,
            actor: Some(d),
            target: Target::Seat(SeatId(1)),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Laptop Computer"));
}

/// Egothha: "During your unlock phase, if Egothha is ready, your prey burns
/// the top card of his or her library, and if that card is a master card,
/// Egothha gains 1 blood": a master burned, +1; a library card burned, none;
/// in torpor, nothing burned.
#[test]
fn egothha_burns_his_preys_library_top_and_feeds_on_a_master() {
    for (top, torpor, gain) in [
        ("Blood Doll", false, 1),
        ("Army of Rats", false, 0),
        ("Blood Doll", true, 0),
    ] {
        let mut e = game(2);
        let g = e.add_minion(SeatId(0), crypt_card("Egothha", false), 2, torpor);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.turn = 3;
        e.state.active = SeatId(1);
        until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
        fill_hand(&mut e, 1);
        e.state.seats[1]
            .library
            .put(find_library(top), zvtes::copies::CopyId::NONE);
        to_minion_phase(&mut e, SeatId(0));
        assert_eq!(ash_has(&e, 1, top), !torpor, "{top} {torpor}");
        assert_eq!(blood(&e, g), 2 + gain, "{top} {torpor}");
    }
}

/// Enzo Giovanni: "Enzo ignores the normal prey or predator restrictions
/// for blocking actions while a vampire controlled by another Methuselah is
/// performing a political action. He gets +1 intercept during political
/// actions not performed by a prince, justicar, or Inner Circle member":
/// across the table at a political action, +1 against an untitled caller,
/// none against a prince; never at a bleed of his prey's.
#[test]
fn enzo_giovanni_blocks_any_political_action_with_intercept() {
    for prince in [false, true] {
        let mut e = game(4);
        let caller = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=11, 0), 4, false);
        if prince {
            e.state.minions[caller.index()].granted_title = Some(Title::Prince("Paris"));
        }
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(
            SeatId(2),
            crypt_card("Enzo Giovanni, Pentex Board of Directors", false),
            4,
            false,
        );
        hand(&mut e, 0, &["Kine Resources Contested"]);
        pick(&mut e, play(0, 0, caller));
        pick_when(&mut e, block(z));
        let a = e.action().expect("an action");
        assert_eq!(a.intercept_of(&e.state, z), i32::from(!prince));
    }
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(
        SeatId(2),
        crypt_card("Enzo Giovanni, Pentex Board of Directors", false),
        4,
        false,
    );
    pick(&mut e, bleed(a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(z));
}

/// Merged Enzo Giovanni: "[MERGED] If a referendum fails, your prey burns 1
/// pool": his +4 against fails the referendum, his prey 1 pool less than
/// with the advanced card alone.
#[test]
fn enzo_giovanni_merged_makes_the_prey_pay_for_a_failed_referendum() {
    let pools: Vec<i32> = [false, true]
        .into_iter()
        .map(|merged| {
            let mut e = game(2);
            let caller = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
            e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
            let adv = crypt_card("Enzo Giovanni, Pentex Board of Directors", true);
            let base = crypt_card("Enzo Giovanni, Pentex Board of Directors", false);
            let z = e.add_minion(SeatId(1), if merged { base } else { adv }, 4, true);
            if merged {
                e.state.minions[z.index()].merged = Some(adv);
            }
            hand(&mut e, 0, &["Kine Resources Contested"]);
            pick(&mut e, play(0, 0, caller));
            pick_when(
                &mut e,
                Choice::UseAbility {
                    source: AbilitySource::Vampire(z),
                    ability: 0,
                    actor: None,
                    target: Target::None,
                },
            );
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, z), 3);
            e.state.seats[0].pool
        })
        .collect();
    assert_eq!(pools[0] - pools[1], 1);
}

/// Erinyi: "-1 strength in combat with a Gangrel antitribu."
#[test]
fn erinyi_weakens_against_a_gangrel_antitribu() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), crypt_card("Erinyi", false), 3, false);
    let anti = e.add_minion(SeatId(1), of_clan("Gangrel antitribu", 1..=11, 0), 4, false);
    let gangrel = e.add_minion(SeatId(1), of_clan("Gangrel", 1..=11, 0), 4, false);
    assert_eq!(strength_vs(&e, r, anti), -1);
    assert_eq!(strength_vs(&e, r, gangrel), 0);
}
