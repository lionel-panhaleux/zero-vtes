//! Round 34 (`ADDED_BATCHES`): Black Hand and Camarilla Edition completed.
//! Windows with a single option are elided, so the scripts only stop where
//! a real decision exists.
#![allow(dead_code)]

use zvtes::cards::{AbilitySource, Stat, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_library, CardId, PrintedCtx, UncontrolledSlot};

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

/// A master card from hand slot `card`, on `target`.
fn master_on(card: u8, target: Target) -> Choice {
    Choice::Play {
        card,
        mode: 0,
        actor: None,
        target,
    }
}

/// Pick the first option matching `want`, the fallback until then.
fn pick_matching(e: &mut Engine, want: impl Fn(&Choice) -> bool) {
    for _ in 0..200 {
        let req = request(e);
        if let Some(i) = req.options.iter().position(&want) {
            e.choose(i);
            return;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    panic!("never offered");
}

/// Seat 0's `a` plays the political action card in hand slot 0, chooses
/// `terms` once the action succeeds (a term with one option elided), and
/// wins its referendum.
fn pass_referendum(e: &mut Engine, a: MinionId, terms: &[Choice]) {
    pick(e, play(0, 0, a));
    for &t in terms {
        loop {
            let req = request(e);
            if req.window == WindowLabel::ReferendumTerms && req.options.contains(&t) {
                e.choose(index_of(&req, t));
                break;
            }
            if req.window == WindowLabel::ReferendumPolling {
                break;
            }
            e.choose(fallback(&req));
        }
    }
    pick_matching(e, |o| {
        *o == Choice::CastCardVote { favor: true } || *o == votes(a, true)
    });
    close_polling(e);
}

/// Seat 0's `a` plays the political action card in hand slot 0 with these
/// `terms`, and its referendum fails, no vote cast.
fn fail_referendum(e: &mut Engine, a: MinionId, terms: &[Choice]) {
    pick(e, play(0, 0, a));
    for &t in terms {
        pick_when(e, t);
    }
    to_referendum(e);
    close_polling(e);
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

/// Seat 0's vampire of `clan` diablerizes seat 1's torpid vampire, and the
/// blood hunt passes on seat 1's prince's vote: the hunted vampire, seat
/// 1's prince.
fn blood_hunt_on(clan: &str, black_hand: bool) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let d = e.add_minion(
        SeatId(0),
        textless(
            |c| c.clan == clan && c.black_hand == black_hand && c.capacity >= 5,
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=7, 1), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 0), 1, true);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    pick(
        &mut e,
        Choice::TakeAction {
            minion: d,
            action: ActionKind::Diablerie,
            target: Some(z),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
    pick(&mut e, votes(p, true));
    (e, d, p)
}

// ---- Master cards (master_a) ------------------------------------------------

/// Absolution of the Diabolist: with a ready justicar, its player cancels
/// a passed blood hunt on another Methuselah's vampire (any vampire): the
/// diablerist stays, the card in the ash heap, the out-of-turn slot spent.
/// Its justicar in torpor: not offered, the diablerist burned.
#[test]
fn absolution_cancels_a_blood_hunt_with_a_ready_justicar() {
    for ready in [true, false] {
        let (mut e, d, p) = blood_hunt_on("Lasombra", false);
        e.state.minions[p.index()].granted_title = Some(Title::Justicar("Ventrue"));
        e.state.minions[p.index()].in_torpor = !ready;
        hand(&mut e, 1, &["Absolution of the Diabolist"]);
        let cancel = master_on(0, Target::Minion(d));
        if !ready {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, cancel);
            assert!(!e.state.minions[d.index()].in_play, "burned");
            continue;
        }
        let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
        assert!(req.options.contains(&cancel), "{:?}", req.options);
        pick(&mut e, cancel);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[d.index()].in_play, "not burned");
        assert!(ash_has(&e, 1, "Absolution of the Diabolist"));
        assert!(e.state.seats[1].out_of_turn.is_some(), "spent");
    }
}

/// Ascendance: "Gain 1 pool."
#[test]
fn ascendance_gains_one_pool() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Ascendance", "Ascendance"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 11);
    assert!(ash_has(&e, 0, "Ascendance"));
}

/// Ebony Fox Hunt: its player cancels a passed blood hunt on their own
/// Black Hand vampire, in their own turn; another Black Hand diablerist's,
/// or a vampire not Black Hand: not offered.
#[test]
fn ebony_fox_hunt_cancels_a_hunt_on_own_black_hand() {
    let (mut e, d, _) = blood_hunt_on("Lasombra", true);
    hand(&mut e, 0, &["Ebony Fox Hunt"]);
    let cancel = master_on(0, Target::Minion(d));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(req.options.contains(&cancel), "{:?}", req.options);
    pick(&mut e, cancel);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[d.index()].in_play, "not burned");
    assert!(ash_has(&e, 0, "Ebony Fox Hunt"));
    for (seat, bh) in [(1, true), (0, false)] {
        let (mut e, d, _) = blood_hunt_on("Lasombra", bh);
        hand(&mut e, seat, &["Ebony Fox Hunt"]);
        never_offered_until(
            &mut e,
            SeatId(0),
            WindowLabel::MinionPhase,
            master_on(0, Target::Minion(d)),
        );
        assert!(!e.state.minions[d.index()].in_play, "burned");
    }
}

/// Dominion: the card itself on another Methuselah's Black Hand vampire,
/// not on one already bearing a Dominion; vampires burn 1 blood to attempt
/// to block it (a cost: one with no blood is offered no attempt), an ally
/// blocks free; once the bearer is no longer Black Hand, no toll (read
/// live, rules-digest D13, D31).
#[test]
fn dominion_tolls_vampire_blockers_not_allies() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let bh = textless(|c| c.black_hand && c.capacity >= 4, 0);
    let a = e.add_minion(SeatId(1), bh, 4, false);
    let other = e.add_minion(
        SeatId(1),
        textless(|c| c.black_hand && c.capacity >= 4, 1),
        4,
        false,
    );
    in_play(&mut e, 0, "Dominion", Some(other));
    hand(&mut e, 0, &["Dominion"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(
        !req.options.contains(&master_on(0, Target::Minion(other))),
        "one"
    );
    pick(&mut e, master_on(0, Target::Minion(a)));
    request(&mut e);
    placed_on(&e, "Dominion", Some(a));
    for blocker in ["vampire", "empty", "ally", "not black hand"] {
        let mut e = game(2);
        let card = if blocker == "not black hand" {
            textless(|c| !c.black_hand && c.capacity >= 4, 0)
        } else {
            bh
        };
        let a = e.add_minion(SeatId(0), card, 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        in_play(&mut e, 0, "Dominion", Some(a));
        let z = match blocker {
            "empty" => e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false),
            "ally" => e.add_ally(SeatId(1), find_library("Asanbonsam Ghoul")),
            _ => e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false),
        };
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let before = blood(&e, z);
        pick(&mut e, bleed(a));
        if blocker == "empty" {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(z));
            continue;
        }
        pick_when(&mut e, block(z));
        match blocker {
            "vampire" => assert_eq!(blood(&e, z), before - 1, "the toll"),
            "ally" => assert_eq!(blood(&e, z), before, "free"),
            _ => assert_eq!(blood(&e, z), before, "no longer Black Hand"),
        }
    }
}

/// The stealth of `a`'s action once it is declared at `kind` (target `t`),
/// read at the block window.
fn action_stealth(e: &mut Engine, choice: Choice) -> i32 {
    pick(e, choice);
    request(e);
    e.action().expect("an action").stealth(&e.state)
}

/// Carthage Remembered: a Brujah prince, another Methuselah's, bleeds at
/// +1 stealth, a Brujah justicar too; an untitled Brujah, a Ventrue prince
/// and the prince's hunt not.
#[test]
fn carthage_remembered_gives_brujah_princes_stealth_on_bleeds() {
    let cases: [(&str, Option<Title>, bool, i32); 5] = [
        ("Brujah", Some(Title::Prince("Paris")), true, 1),
        ("Brujah", Some(Title::Justicar("Brujah")), true, 1),
        ("Brujah", None, true, 0),
        ("Ventrue", Some(Title::Prince("Paris")), true, 0),
        ("Brujah", Some(Title::Prince("Paris")), false, 0),
    ];
    for (clan, title, bleeds, plus) in cases {
        let mut stealth = Vec::new();
        for carthage in [false, true] {
            let mut e = game(2);
            let cam = |c: &zvtes::crypt::CryptCard| {
                c.clan == clan && c.sect == Some(Sect::Camarilla) && c.capacity >= 4
            };
            let a = e.add_minion(SeatId(0), textless(cam, 0), 4, false);
            e.state.minions[a.index()].granted_title = title;
            assert_eq!(title.unwrap_or(Title::None), e.state.title_of(a));
            e.add_minion(SeatId(0), textless(cam, 1), 4, false);
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
            if carthage {
                in_play(&mut e, 1, "Carthage Remembered", None);
            }
            let choice = if bleeds {
                bleed(a)
            } else {
                action(a, ActionKind::Hunt)
            };
            stealth.push(action_stealth(&mut e, choice));
        }
        assert_eq!(stealth[1] - stealth[0], plus, "{clan} {title:?} {bleeds}");
    }
}

/// Carthage Remembered: any minion burns it as a Ⓓ action, a Ventrue or a
/// Malkavian at -1 stealth, a Toreador at none; passed, the card burns.
#[test]
fn carthage_remembered_burn_costs_ventrue_stealth() {
    for (clan, s) in [("Ventrue", -1), ("Malkavian", -1), ("Toreador", 0)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_clan(clan, 4..=8, 0), 4, false);
        e.add_minion(SeatId(0), of_clan(clan, 4..=8, 1), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        let c = in_play(&mut e, 1, "Carthage Remembered", None);
        let burn = use_on(c, 0, Some(a), Target::None);
        assert_eq!(action_stealth(&mut e, burn), s, "{clan}");
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(ash_has(&e, 1, "Carthage Remembered"), "{clan}");
    }
}

/// Communal Haven: Temple: a diablerie at its controller's torpid Sabbat
/// vampire: their Sabbat blocker gets +1 intercept, their Camarilla one
/// none; a bleed at them gives none; another Methuselah's Temple none.
#[test]
fn communal_haven_temple_helps_block_actions_at_own_sabbat() {
    for (owner, diablerie, plus) in [(1, true, 1), (1, false, 0), (0, true, 0)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let t = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=7, 0), 1, true);
        let s = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=7, 1), 4, false);
        let n = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=7, 0), 4, false);
        in_play(&mut e, owner, "Communal Haven: Temple", None);
        let choice = if diablerie {
            Choice::TakeAction {
                minion: a,
                action: ActionKind::Diablerie,
                target: Some(t),
            }
        } else {
            bleed(a)
        };
        pick(&mut e, choice);
        request(&mut e);
        let act = e.action().expect("an action");
        let base = act.intercept_of(&e.state, n);
        assert_eq!(base, 0);
        assert_eq!(act.intercept_of(&e.state, s), plus, "{owner} {diablerie}");
    }
}

/// Communal Haven: Temple: "A Methuselah may have only one": not offered
/// while its player has one in play.
#[test]
fn communal_haven_temple_one_per_methuselah() {
    for held in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        if held {
            in_play(&mut e, 0, "Communal Haven: Temple", None);
        }
        hand(&mut e, 0, &["Communal Haven: Temple"]);
        e.at_master_phase();
        let req = request(&mut e);
        assert_eq!(req.options.contains(&master_play(0)), !held);
    }
}

/// Guru: its Sabbat vampire calls The Final Nights and it passes: each
/// vampire burns 1 blood, then (LSJ 20090115-2: after the referendum's
/// effects) the caller gains 1 from Guru; a failed referendum gains none.
/// [LSJ 20090115-2]
#[test]
fn guru_gains_after_the_referendum_effects() {
    use zvtes::notes::{Cause, Source};
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=7, 0), 3, false);
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=7, 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let guru = in_play(&mut e, 0, "Guru", Some(a));
    hand(&mut e, 0, &["Final Nights, The"]);
    e.notes = Some(Vec::new());
    let before = e.table();
    pass_referendum(&mut e, a, &[]);
    request(&mut e);
    let on_a: Vec<_> = noted_changes(&mut e, before)
        .into_iter()
        .filter(|&(_, at, _)| at == Err(a))
        .collect();
    let gain = on_a
        .iter()
        .position(|&(c, _, d)| c == Some(Cause::Effect(Source::Card(guru))) && d == 1)
        .expect("Guru's blood");
    let burn = on_a
        .iter()
        .position(|&(_, _, d)| d == -1)
        .expect("the burn");
    assert!(burn < gain, "{on_a:?}");
    assert_eq!(blood(&e, a), 3);
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=7, 0), 3, false);
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=7, 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Guru", Some(a));
    hand(&mut e, 0, &["Final Nights, The"]);
    fail_referendum(&mut e, a, &[]);
    request(&mut e);
    assert_eq!(blood(&e, a), 2, "failed: the actor's 1, no gain");
}

/// Seat 1's Sabbat vampire bearing Creep Show (3 blood) blocks seat 0's
/// `x` bleeding, who plays the hand card `card` in its basic mode as its
/// strike: the Creep Show vampire.
fn creep_show_blocks(e: &mut Engine, x: MinionId, card: &str) -> MinionId {
    let c = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 5..=7, 0), 3, false);
    e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 5..=7, 1), 4, false);
    in_play(e, 1, "Creep Show", Some(c));
    hand(e, 0, &[card]);
    pick(e, bleed(x));
    pick_when(e, block(c));
    pick_when(e, play(0, 0, x));
    c
}

/// Creep Show: the opposing minion dodges (Side Strike [cel]): its vampire
/// gains 1 blood (its hand strike dodged, 3 + 1); once per turn: blocking
/// again that turn, dodged again, nothing.
#[test]
fn creep_show_gains_a_blood_when_the_opponent_dodges() {
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), vampire_with("cel", 0), 4, false);
    let y = e.add_minion(SeatId(0), vampire_with("cel", 1), 4, false);
    let c = creep_show_blocks(&mut e, x, "Side Strike");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, c), 4, "dodged: +1");
    assert_eq!(blood(&e, x), 4, "the hand strike dodged");
    e.state.minions[c.index()].locked = false;
    hand(&mut e, 0, &["Side Strike"]);
    pick(&mut e, bleed(y));
    pick_when(&mut e, block(c));
    pick_when(&mut e, play(0, 0, y));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, c), 4, "once per turn");
}

/// Creep Show: the opposing minion ends combat as a strike (Majesty
/// [pre]): its vampire gains 1 blood.
#[test]
fn creep_show_gains_when_the_opponent_ends_combat() {
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), vampire_with("pre", 0), 4, false);
    e.add_minion(SeatId(0), vampire_with("pre", 1), 4, false);
    let c = creep_show_blocks(&mut e, x, "Majesty");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, c), 4);
}

/// Illegal Search and Seizure: another Methuselah's aggravated weapon
/// (White Phosphorus Grenade, 1 pool) and a 5-pool one (AK-47) are
/// offered, a 2-pool 2-damage one (.44 Magnum) not; the chosen weapon
/// burns and its bearer burns 1 blood, no prevention asked. [RTR 19980623]
#[test]
fn illegal_search_burns_an_aggravated_weapon_and_damages_its_bearer() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let wp = in_play(&mut e, 1, "White Phosphorus Grenade", Some(b));
    let ak = in_play(&mut e, 1, "AK-47", Some(b));
    let magnum = in_play(&mut e, 1, ".44 Magnum", Some(b));
    hand(&mut e, 0, &["Illegal Search and Seizure"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(req.options.contains(&master_on(0, Target::Card(ak))));
    assert!(!req.options.contains(&master_on(0, Target::Card(magnum))));
    pick(&mut e, master_on(0, Target::Card(wp)));
    request(&mut e);
    assert!(!e.state.cards[wp.index()].in_play, "burned");
    assert!(ash_has(&e, 1, "White Phosphorus Grenade"));
    assert_eq!(blood(&e, b), 3, "1 damage, unprevented");
}

/// Illegal Search and Seizure: a weapon's cost as modified in play: a
/// 2-pool .44 Magnum under Centralized Background Check costs 3, and is
/// offered. [ANK 20181215] [LSJ 20080125] [LSJ 20040701]
#[test]
fn illegal_search_reads_a_weapons_cost_in_play() {
    for check in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let magnum = in_play(&mut e, 1, ".44 Magnum", Some(b));
        if check {
            in_play(&mut e, 1, "Centralized Background Check", None);
        }
        hand(&mut e, 0, &["Illegal Search and Seizure"]);
        e.at_master_phase();
        let req = request(&mut e);
        assert_eq!(
            req.options.contains(&master_on(0, Target::Card(magnum))),
            check
        );
    }
}

/// Bureaucratic Overload: another Methuselah's political action costs 1
/// more blood, paid as it resolves (The Final Nights: 3 - 1 - 1).
#[test]
fn bureaucratic_overload_makes_political_actions_cost_a_blood() {
    for overload in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=7, 0), 3, false);
        e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=7, 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if overload {
            in_play(&mut e, 1, "Bureaucratic Overload", None);
        }
        hand(&mut e, 0, &["Final Nights, The"]);
        pass_referendum(&mut e, a, &[]);
        request(&mut e);
        assert_eq!(blood(&e, a), 2 - u8::from(overload));
    }
}

/// Bureaucratic Overload: any vampire calls a referendum to burn it, a +1
/// stealth political action costing the blood too; passed, it burns.
#[test]
fn bureaucratic_overload_burned_by_referendum_costs_a_blood_too() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=7, 0), 3, false);
    e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=7, 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[a.index()].granted_title = Some(Title::Prince("Paris"));
    let b = in_play(&mut e, 1, "Bureaucratic Overload", None);
    let burn = use_on(b, 0, Some(a), Target::None);
    assert_eq!(action_stealth(&mut e, burn), 1);
    pick_matching(&mut e, |o| *o == votes(a, true));
    close_polling(&mut e);
    request(&mut e);
    assert!(ash_has(&e, 1, "Bureaucratic Overload"));
    assert_eq!(blood(&e, a), 2, "the surcharge");
}

/// Brothers Grimm: a counter during its controller's unlock phase only
/// (3 → 4 over a round); at the fifth it burns, its controller +5 pool.
#[test]
fn brothers_grimm_pays_five_pool_at_five_counters() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let g = in_play(&mut e, 0, "Brothers Grimm", None);
    e.state.cards[g.index()].counters = 3;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.cards[g.index()].counters,
        3,
        "not the others' phase"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[g.index()].counters, 4);
    let pool = e.state.seats[0].pool;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Brothers Grimm"));
    assert_eq!(e.state.seats[0].pool, pool + 5);
}

/// Brothers Grimm: another Methuselah takes it for a master phase action
/// and a master card discarded; its controller, holding a master card, may
/// keep it by discarding one: kept, the taker's costs still spent; not,
/// taken with its counters.
#[test]
fn brothers_grimm_taken_or_kept_by_a_master_discard() {
    for keep in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let g = in_play(&mut e, 0, "Brothers Grimm", None);
        e.state.cards[g.index()].counters = 2;
        hand(&mut e, 0, &["Ascendance"]);
        hand(&mut e, 1, &["Ascendance", "Dominion"]);
        let req = until(&mut e, SeatId(1), WindowLabel::MasterPhase);
        let take = use_on(g, 0, None, Target::None);
        assert!(req.options.contains(&take), "{:?}", req.options);
        e.choose(index_of(&req, take));
        pick_when(&mut e, Choice::Discard(1));
        let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        assert_eq!(req.options, [Choice::Pass, Choice::Discard(0)]);
        e.choose(usize::from(keep));
        request(&mut e);
        assert!(ash_has(&e, 1, "Dominion"), "the taker's discard");
        assert_eq!(ash_has(&e, 0, "Ascendance"), keep);
        let owner = if keep { 0 } else { 1 };
        assert_eq!(e.state.card_controller(g), SeatId(owner));
        assert_eq!(e.state.cards[g.index()].counters, 2);
        assert_eq!(e.state.seats[1].mpa, 0, "the master phase action spent");
    }
}

/// Seat 0 plays Brainwash on slot `slot` of seat 1's (its prey's)
/// uncontrolled region, holding `cards` (crypt card, counters): the card.
fn brainwash(e: &mut Engine, cards: &[(u16, u8)], slot: u8) -> CardId {
    for &(card, counters) in cards {
        uncontrolled(e, 1, card, counters);
    }
    hand(e, 0, &["Brainwash"]);
    e.at_master_phase();
    pick(e, master_play(0));
    if cards.len() > 1 {
        let req = expect(e, SeatId(0), WindowLabel::PlayTerms);
        assert_eq!(
            req.options,
            (0..cards.len() as u8).map(Choice::Slot).collect::<Vec<_>>(),
            "by position"
        );
        e.choose(slot as usize);
    }
    request(e);
    placed_on(e, "Brainwash", None)
}

/// Brainwash: the card itself on the named slot of the prey's
/// uncontrolled region; in the prey's influence phase no transfer to or
/// from it is offered, the other slot's are. [LSJ 20010211] [LSJ
/// 20040728]
#[test]
fn brainwash_goes_on_the_preys_uncontrolled_vampire_and_bars_transfers() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cards = [(plain("Toreador", 0), 2), (plain("Toreador", 1), 2)];
    let c = brainwash(&mut e, &cards, 1);
    assert_eq!(e.state.cards[c.index()].on_slot, Some((SeatId(1), 1)));
    let req = to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    for (slot, open) in [(0, true), (1, false)] {
        assert_eq!(req.options.contains(&Choice::Transfer { slot }), open);
        assert_eq!(req.options.contains(&Choice::Withdraw { slot }), open);
    }
}

/// Brainwash: the slot before it influenced out, the card follows its
/// vampire's position (LSJ 20040728); Govern the Unaligned [DOM] still
/// adds blood to it (only transfers barred), and at capacity it is
/// influenced out, Brainwash staying on it. [LSJ 20010211] [LSJ 20040728]
#[test]
fn brainwash_follows_its_vampire_into_play() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let g = e.add_minion(SeatId(1), vampire_with("DOM", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let first = plain("Toreador", 0);
    let cap = CRYPT[first as usize].capacity;
    let small = textless(|c| c.capacity == 3, 0);
    let c = brainwash(&mut e, &[(first, cap), (small, 0)], 1);
    let req = to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    e.choose(index_of(&req, Choice::InfluenceOut { slot: 0 }));
    assert_eq!(e.state.cards[c.index()].on_slot, Some((SeatId(1), 0)));
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::Transfer { slot: 0 }));
    hand(&mut e, 1, &["Govern the Unaligned"]);
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 1,
            actor: Some(g),
            target: Target::Uncontrolled(0),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.seats[1].uncontrolled[0].counters, 3,
        "Govern's blood"
    );
    let req = to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    e.choose(index_of(&req, Choice::InfluenceOut { slot: 0 }));
    let m = MinionId(e.state.minions.len() as u32 - 1);
    assert_eq!(e.state.minions[m.index()].crypt_card(), Some(small));
    let card = &e.state.cards[c.index()];
    assert_eq!(
        (card.in_play, card.host, card.on_slot),
        (true, Some(m), None)
    );
}

/// Brainwash: its vampire influenced out, then banished back by
/// Banishment: the bar on transfers resumes (its last clause). [LSJ
/// 20010211]
#[test]
fn brainwash_resumes_after_banishment() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 9, 0), 6, false);
    e.state.minions[a.index()].granted_title = Some(Title::Prince("Paris"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let small = textless(|c| c.capacity == 3, 0);
    let c = brainwash(&mut e, &[(small, 3)], 0);
    let req = to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    e.choose(index_of(&req, Choice::InfluenceOut { slot: 0 }));
    let m = MinionId(e.state.minions.len() as u32 - 1);
    assert_eq!(e.state.cards[c.index()].host, Some(m));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    hand(&mut e, 0, &["Banishment"]);
    pass_referendum(&mut e, a, &[Choice::ChooseMinion(m)]);
    request(&mut e);
    let slot = e.state.seats[1].uncontrolled.len() as u8 - 1;
    assert!(e.state.seats[1].uncontrolled[slot as usize]
        .banished
        .is_some());
    let req = to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    assert!(!req.options.contains(&Choice::Transfer { slot }));
    assert!(!req.options.contains(&Choice::Withdraw { slot }));
}

/// Brainwash on an advanced vampire: it still merges with its base in
/// play, the card following onto the merged vampire. [LSJ 20030422]
#[test]
fn brainwashed_advanced_vampire_still_merges() {
    let named = |adv: bool| {
        (0..CRYPT.len() as u16)
            .find(|&c| CRYPT[c as usize].name == "Appolonius" && CRYPT[c as usize].adv == adv)
            .expect("Appolonius")
    };
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ap = e.add_minion(SeatId(1), named(false), 5, false);
    let c = brainwash(&mut e, &[(named(true), 3)], 0);
    let req = to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    e.choose(index_of(&req, Choice::Merge { slot: 0 }));
    let card = &e.state.cards[c.index()];
    assert_eq!(
        (card.in_play, card.host, card.on_slot),
        (true, Some(ap), None)
    );
}

/// Brainwash: any minion burns it, a +1 stealth action, directed at its
/// controller: the prey's minion, here, the card to its owner's ash heap.
#[test]
fn brainwash_burned_by_a_directed_action() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let c = brainwash(&mut e, &[(plain("Toreador", 0), 2)], 0);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let burn = use_on(c, 0, Some(z), Target::None);
    assert_eq!(action_stealth(&mut e, burn), 1);
    let a = e.action().expect("the burn");
    assert_eq!(a.directed_seat(&e.state), Some(SeatId(0)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Brainwash"));
}

/// Brainwash bars Solomon Batanea's transfers from its slot ("no transfers
/// … from that card"): with only the brainwashed vampire holding blood, his
/// ability is not offered; another slot with blood, it is, that slot only.
#[test]
fn brainwash_bars_solomon_batanea_from_its_slot() {
    for other in [false, true] {
        let mut e = game(2);
        let s = e.add_minion(SeatId(0), find_card("Solomon Batanea"), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let mut cards = vec![(plain("Toreador", 0), 2)];
        if other {
            cards.push((plain("Toreador", 1), 2));
        }
        brainwash(&mut e, &cards, 0);
        uncontrolled(&mut e, 0, plain("Toreador", 2), 1);
        let req = to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
        let used = req.options.iter().position(|o| {
            matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == s)
        });
        assert_eq!(used.is_some(), other);
        if let Some(i) = used {
            e.choose(i);
            request(&mut e);
            assert_eq!(e.state.seats[1].uncontrolled[0].counters, 2, "barred");
            assert_eq!(e.state.seats[1].uncontrolled[1].counters, 1);
        }
    }
}

/// Cairo Int'l Airport: its controller's minion burns 1 blood on the
/// prey's uncontrolled minion named as the action is announced (slot 1 of
/// 3: 3 → 2, the others untouched), directed at the prey (their minion is
/// offered a block); an empty one named, a legal no-op.
#[test]
fn cairo_airport_burns_a_blood_on_the_preys_uncontrolled_vampire() {
    for counters in [3, 0] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let c = in_play(&mut e, 0, "Cairo Int'l Airport", None);
        for (i, k) in [2, counters, 2].into_iter().enumerate() {
            uncontrolled(&mut e, 1, plain("Toreador", i), k);
        }
        pick(&mut e, use_on(c, 0, Some(a), Target::Seat(SeatId(1))));
        let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        assert_eq!(req.options, (0..3).map(Choice::Slot).collect::<Vec<_>>());
        e.choose(1);
        let req = request(&mut e);
        assert_eq!(req.window, WindowLabel::ActionBlock);
        assert!(req.options.contains(&block(z)), "directed at the prey");
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let left: Vec<u8> = e.state.seats[1]
            .uncontrolled
            .iter()
            .map(|u| u.counters)
            .collect();
        assert_eq!(left, [2, counters.saturating_sub(1), 2]);
    }
}

/// Seat 0's locked capacity-8 vampire with `copies` Kindred Society Games
/// (3 blood), beside seat 1's two capacity-4 vampires locked if `younger`
/// (the first returned), an unlocked one and an older locked one, at seat
/// 0's unlock-phase window: the vampire, the copies, seat 1's first.
fn society_games(copies: usize, younger: bool) -> (Engine, MinionId, Vec<CardId>, MinionId) {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), textless(|c| c.capacity == 8, 0), 3, false);
    e.state.minions[v.index()].locked = true;
    e.add_minion(SeatId(0), textless(|c| c.capacity == 9, 0), 4, false);
    let y = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 2, false);
    let y2 = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 1), 2, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 2), 2, false);
    let old = e.add_minion(SeatId(1), textless(|c| c.capacity >= 9, 1), 2, false);
    let games = (0..copies)
        .map(|_| in_play(&mut e, 1, "Kindred Society Games", Some(v)))
        .collect();
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    for m in [y, y2] {
        e.state.minions[m.index()].locked = younger;
    }
    e.state.minions[old.index()].locked = true;
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    (e, v, games, y)
}

/// Kindred Society Games: its vampire does not unlock as normal; during
/// its controller's unlock phase it burns 1 blood to unlock, the card
/// staying. [RTR 19951017]
#[test]
fn kindred_society_games_pays_a_blood_to_unlock() {
    let (mut e, v, games, _) = society_games(1, true);
    assert!(e.state.minions[v.index()].locked, "not as normal");
    pick(&mut e, use_on(games[0], 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[v.index()].locked);
    assert_eq!(blood(&e, v), 2);
    assert_eq!(e.state.cards[games[0].index()].host, Some(v));
}

/// Kindred Society Games on a vampire already unlocked at its controller's
/// unlock phase (rules-qa 126.106): the 1-blood payment is offered, a legal
/// no-op on the lock that keeps the card from moving (D14); paid, the card
/// stays, the blood gone.
#[test]
fn kindred_society_games_unlocked_vampire_may_pay_to_keep_it() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), textless(|c| c.capacity == 8, 0), 3, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity == 9, 0), 4, false);
    let y = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 2, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 1), 2, false);
    let games = in_play(&mut e, 1, "Kindred Society Games", Some(v));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[y.index()].locked = true;
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(!e.state.minions[v.index()].locked);
    e.choose(index_of(&req, use_on(games, 0, None, Target::None)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[games.index()].host, Some(v), "kept");
    assert_eq!(blood(&e, v), 2);
}

/// Kindred Society Games unpaid: as the phase ends, its vampire's
/// controller moves it onto a younger locked vampire, any Methuselah's
/// (an older locked one or an unlocked younger one not offered), the
/// vampire staying locked; with none, it burns.
#[test]
fn kindred_society_games_moves_to_a_younger_locked_vampire_when_unpaid() {
    let (mut e, v, games, y) = society_games(1, true);
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    let y2 = MinionId(y.0 + 1);
    assert_eq!(
        req.options,
        [Choice::ChooseMinion(y), Choice::ChooseMinion(y2)]
    );
    e.choose(0);
    request(&mut e);
    assert_eq!(e.state.cards[games[0].index()].host, Some(y));
    assert!(e.state.minions[v.index()].locked);
    let (mut e, v, games, _) = society_games(1, false);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[games[0].index()].in_play);
    assert!(ash_has(&e, 1, "Kindred Society Games"));
    assert!(e.state.minions[v.index()].locked);
}

/// Kindred Society Games, two copies: each its own unlock (2 blood); one
/// paid, the other moved on: the vampire unlocks (rules-qa 126.105).
/// [RTR 19951017]
#[test]
fn kindred_society_games_two_copies_cost_two_blood() {
    let (mut e, v, games, _) = society_games(2, false);
    pick(&mut e, use_on(games[0], 0, None, Target::None));
    request(&mut e);
    assert!(e.state.minions[v.index()].locked, "one copy paid");
    pick(&mut e, use_on(games[1], 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[v.index()].locked);
    assert_eq!(blood(&e, v), 1);
    let (mut e, v, games, y) = society_games(2, true);
    pick(&mut e, use_on(games[0], 0, None, Target::None));
    pick_when(&mut e, Choice::ChooseMinion(y));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[games[1].index()].host, Some(y));
    assert!(!e.state.minions[v.index()].locked, "the copy left paid");
    assert_eq!(blood(&e, v), 2);
}

/// Forest of Shadows: its controller locks it as their Malkavian's block
/// attempt needs stealth: X blood burned for +X, X up to 1 above the
/// blocker's intercept (0: X is 1); the blocker's Eagle's Sight [aus] at
/// +1 intercept, the Forest unlocked is used again in the same action (TOM
/// 19960109), X 1 again; the stealth lasts the turn. [TOM 19960109]
#[test]
fn forest_of_shadows_stealth_capped_one_above_intercept() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Malkavian", 5..=8, 0), 5, false);
    e.add_minion(SeatId(0), of_clan("Malkavian", 5..=8, 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let f = in_play(&mut e, 0, "Forest of Shadows", None);
    hand(&mut e, 1, &["Eagle's Sight"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let lock = use_on(f, 0, None, Target::None);
    // X's one option, 1, elided.
    pick_when(&mut e, lock);
    request(&mut e);
    assert_eq!(e.action().unwrap().stealth(&e.state), 1);
    assert_eq!(blood(&e, a), 4);
    pick_when(&mut e, play(0, 0, z));
    e.state.cards[f.index()].locked = false;
    pick_when(&mut e, lock);
    request(&mut e);
    assert_eq!(e.action().unwrap().stealth(&e.state), 2);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 3);
    assert_eq!(e.state.lasting(a).0, 2, "until the end of the turn");
}

/// Three seats, seat 0's minion phase: its `for` vampire, seat 1 at 1
/// pool, seat 2 controlling Last Stand (when `stand`).
fn last_stand(stand: bool) -> (Engine, MinionId) {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("for", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    e.state.seats[1].pool = 1;
    if stand {
        in_play(&mut e, 2, "Last Stand", None);
    }
    (e, a)
}

/// Last Stand: seat 0 bleeds seat 1 out: after the action's resolution
/// plays (Freak Drive offered, LSJ 20090731) the turn ends and the card
/// burns; seat 1's predator, seat 0, takes the next turn (turn 5; without
/// it seat 2 plays turn 5). [LSJ 20090731]
#[test]
fn last_stand_ends_the_turn_after_the_action() {
    for stand in [true, false] {
        let (mut e, a) = last_stand(stand);
        hand(&mut e, 0, &["Freak Drive"]);
        pick(&mut e, bleed(a));
        let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
        assert!(e.state.seats[1].ousted);
        assert!(req.options.contains(&play(0, 0, a)), "Freak Drive");
        e.choose(index_of(&req, Choice::Pass));
        let req = request(&mut e);
        if stand {
            assert_eq!((req.seat, e.state.turn), (SeatId(0), 5));
            assert!(ash_has(&e, 2, "Last Stand"));
        } else {
            until(&mut e, SeatId(2), WindowLabel::MinionPhase);
            assert_eq!(e.state.turn, 5);
        }
    }
}

/// Last Stand: the cut turn's minion phase ends, its end effects run
/// (rules-qa 126.110): Tupdog burns after it (LSJ 20090731), two Thirsts
/// burn 1 blood from a capacity-1 vampire that did not hunt. [LSJ 20090731]
#[test]
fn last_stand_ends_the_minion_phase() {
    let (mut e, a) = last_stand(true);
    let tupdog = e.add_minion(SeatId(0), find_card("Tupdog"), 2, false);
    let small = e.add_minion(SeatId(0), textless(|c| c.capacity == 1, 0), 1, false);
    in_play(&mut e, 2, "Thirst", None);
    in_play(&mut e, 2, "Thirst", None);
    pick(&mut e, bleed(a));
    while e.state.turn < 5 {
        let req = request(&mut e);
        e.choose(fallback(&req));
    }
    assert!(!e.state.minions[tupdog.index()].in_play, "burned");
    assert_eq!(blood(&e, small), 0, "Thirst");
    assert_eq!(e.state.active, SeatId(0));
}

/// Last Stand: no discard phase in the cut turn (LSJ 20090731): a card
/// held until it (The Unmasking's) is still held at seat 0's next turn.
/// [LSJ 20090731]
#[test]
fn last_stand_skips_the_discard_phase() {
    use zvtes::state::Release;
    let (mut e, a) = last_stand(true);
    for _ in 0..3 {
        e.state.seats[0].library.push(find_library("Form of Mist"));
    }
    e.state.seats[0].held[Release::NextDiscardPhase] = 1;
    pick(&mut e, bleed(a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.turn, 5);
    assert_eq!(
        e.state.seats[0].held[Release::NextDiscardPhase],
        1,
        "not replaced"
    );
}

/// Four seats: seat 0's [dom] vampire bleeds seat 1 (1 pool) for 3 with
/// Govern the Unaligned; seat 1 plays Extremis Boon as it is being ousted
/// (-2): the card, the auction begun at seat 2.
fn extremis_boon() -> (Engine, CardId) {
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), vampire_with("dom", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    for s in 1..4 {
        e.add_minion(SeatId(s), plain("Ventrue", s as usize), 4, false);
    }
    e.state.seats[1].pool = 1;
    hand(&mut e, 0, &["Govern the Unaligned"]);
    hand(&mut e, 1, &["Extremis Boon"]);
    pick(&mut e, play(0, 0, a));
    let boon = master_on(0, Target::Seat(SeatId(1)));
    let req = offering(&mut e, boon);
    assert_eq!(
        (req.seat, req.options.clone()),
        (SeatId(1), vec![Choice::Pass, boon])
    );
    e.choose(1);
    let req = request(&mut e);
    assert_eq!(
        req.seat,
        SeatId(2),
        "clockwise from the target's left: {req:?}"
    );
    let card = (0..e.state.cards.len())
        .map(|i| CardId(i as u32))
        .find(|&c| e.state.cards[c.index()].card == find_library("Extremis Boon"))
        .expect("in play");
    (e, card)
}

/// Extremis Boon: the bids start at what keeps its player in (3, at -2);
/// seat 2 bids 4, seat 3 5, seat 0 and seat 2 pass (for good): seat 3 pays
/// 5 to seat 1 (-2 → 3), seat 2 nothing; the card in play against seat 3.
/// [LSJ 20031121-1] [ANK 20190827]
#[test]
fn extremis_boon_the_high_bid_keeps_its_player_in() {
    let (mut e, card) = extremis_boon();
    let req = request(&mut e);
    assert_eq!(req.options[0], Choice::Pass);
    assert_eq!(
        req.options[1],
        Choice::SetX(3),
        "the starting bid keeps them in"
    );
    pick(&mut e, Choice::SetX(4));
    let req = request(&mut e);
    assert_eq!((req.seat, req.options[1]), (SeatId(3), Choice::SetX(5)));
    pick(&mut e, Choice::SetX(5));
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(0));
    pick(&mut e, Choice::Pass);
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(2));
    pick(&mut e, Choice::Pass);
    request(&mut e);
    assert!(!e.state.seats[1].ousted);
    assert_eq!(pool(&e), vec![10, 3, 10, 5]);
    let c = &e.state.cards[card.index()];
    assert!(c.in_play && c.against.contains(SeatId(3)));
}

/// Extremis Boon: no bid, its player is ousted, the card played to no
/// effect (LSJ 20040601).
#[test]
fn extremis_boon_no_bid_ousts() {
    let (mut e, _) = extremis_boon();
    for _ in 0..3 {
        pick(&mut e, Choice::Pass);
    }
    request(&mut e);
    assert!(e.state.seats[1].ousted);
    assert!(!e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == find_library("Extremis Boon")));
}

/// Extremis Boon: the high bidder steals up to 2 pool from its player for
/// each master phase action; its player burns it giving them 6 pool in
/// their master phase.
#[test]
fn extremis_boon_steal_and_burn() {
    let (mut e, card) = extremis_boon();
    for b in [Choice::Pass, Choice::SetX(4), Choice::Pass] {
        pick(&mut e, b);
    }
    request(&mut e);
    assert!(e.state.cards[card.index()].against.contains(SeatId(3)));
    let req = until(&mut e, SeatId(3), WindowLabel::MasterPhase);
    let steal = use_on(card, 0, None, Target::None);
    e.choose(index_of(&req, steal));
    let req = expect(&mut e, SeatId(3), WindowLabel::PlayTerms);
    assert_eq!(req.options, [Choice::SetX(1), Choice::SetX(2)]);
    e.choose(1);
    request(&mut e);
    let after = pool(&e);
    assert_eq!((after[1], after[3]), (0, 6 + 2));
    assert!(e.state.seats[1].ousted, "stolen down to 0");
    let (mut e, card) = extremis_boon();
    for b in [Choice::Pass, Choice::SetX(4), Choice::Pass] {
        pick(&mut e, b);
    }
    request(&mut e);
    e.state.seats[1].pool = 10;
    let req = until(&mut e, SeatId(1), WindowLabel::MasterPhase);
    e.choose(index_of(&req, use_on(card, 1, None, Target::None)));
    request(&mut e);
    assert_eq!((pool(&e)[1], pool(&e)[3]), (4, 12));
    assert!(ash_has(&e, 1, "Extremis Boon"));
}

// ---- Action cards (action) --------------------------------------------------

/// The hand card at `card` played by `a` in `mode` at `target`.
fn act(card: u8, mode: u8, a: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(a),
        target,
    }
}

/// Blooding: the card itself on its Sabbat actor, +1 blood, Black Hand
/// while Sabbat (read live: become Anarch, not); a second not offered.
#[test]
fn blooding_makes_its_sabbat_vampire_black_hand_with_a_blood() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| c.sect == Some(Sect::Sabbat) && !c.black_hand && c.capacity >= 4,
            0,
        ),
        3,
        false,
    );
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=7, 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert!(!e.state.is_black_hand(a));
    hand(&mut e, 0, &["Blooding", "Blooding"]);
    pick(&mut e, act(0, 0, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Blooding", Some(a));
    assert_eq!(blood(&e, a), 4);
    assert!(e.state.is_black_hand(a));
    e.state.minions[a.index()].sect_override = Some(Sect::Camarilla);
    assert!(!e.state.is_black_hand(a), "not Sabbat: not Black Hand");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=7, 0), 3, false);
    let b = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=7, 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Blooding", Some(a));
    hand(&mut e, 0, &["Blooding"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&act(0, 0, a, Target::None)),
        "only one"
    );
    assert!(req.options.contains(&act(0, 0, b, Target::None)));
}

/// Bloodwork: +1 bleed, the pool burned capped at 2; +1 stealth only
/// against a Methuselah controlling a ready Sabbat vampire.
#[test]
fn bloodwork_sneaks_against_a_sabbat_prey_and_caps_at_two() {
    for sabbat in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(
            SeatId(0),
            textless(|c| c.black_hand && c.capacity >= 4, 0),
            4,
            false,
        );
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let sect = if sabbat {
            Sect::Sabbat
        } else {
            Sect::Camarilla
        };
        e.add_minion(SeatId(1), of_sect(sect, 4..=7, 0), 4, false);
        e.state.minions[a.index()].timed.push((
            zvtes::cards::Lasting::Bleed(3),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(0),
        ));
        hand(&mut e, 0, &["Bloodwork"]);
        assert_eq!(
            action_stealth(&mut e, act(0, 0, a, Target::None)),
            i32::from(sabbat)
        );
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 8, "capped at 2");
    }
}

/// Cat Burglary [CEL]: bleeds a Methuselah other than the prey, the pool
/// burned capped at 1.
#[test]
fn cat_burglary_bleeds_a_non_prey_for_one_at_most() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Bleed(2),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    hand(&mut e, 0, &["Cat Burglary"]);
    pick(&mut e, act(0, 1, a, Target::Seat(SeatId(2))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 10, 9]);
}

/// Celestial Harmony [aus]: the chosen Methuselah keeps one of three hand
/// cards hidden, the player sees the two others; [AUS] is directed at
/// every other Methuselah.
#[test]
fn celestial_harmony_shows_all_but_the_card_its_target_keeps() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("aus", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Celestial Harmony"]);
    hand(&mut e, 2, &["Dominion", "Ascendance", "Brainwash"]);
    e.notes = Some(Vec::new());
    pick(&mut e, act(0, 0, a, Target::Seat(SeatId(2))));
    let req = until(&mut e, SeatId(2), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, (0..3).map(Choice::Discard).collect::<Vec<_>>());
    e.choose(1);
    request(&mut e);
    let looks = noted_looks(&mut e);
    let look = looks.last().expect("a look");
    assert_eq!((look.viewer, look.seat), (SeatId(0), SeatId(2)));
    assert_eq!(
        look.cards,
        vec![find_library("Dominion"), find_library("Brainwash")]
    );
    assert_eq!(e.state.seats[2].hand.len(), 3, "shown, kept");
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("AUS", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Celestial Harmony"]);
    pick(&mut e, act(0, 1, a, Target::None));
    request(&mut e);
    let seats = e.action().expect("the action").directed_seats(&e.state);
    assert!(seats.contains(SeatId(1)) && seats.contains(SeatId(2)));
}

/// Chronicle of the Lost Tribe: in play, +1 hand size to its controller;
/// stolen by another Methuselah's capacity-5 vampire (a capacity-4 one is
/// not offered), the hand size goes with it.
#[test]
fn chronicle_of_the_lost_tribe_raises_hand_size_until_stolen() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.black_hand && c.capacity >= 4, 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let small = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 4, false);
    let big = e.add_minion(SeatId(1), textless(|c| c.capacity == 5, 0), 4, false);
    let (h0, h1) = (e.state.hand_size(SeatId(0)), e.state.hand_size(SeatId(1)));
    hand(&mut e, 0, &["Chronicle of the Lost Tribe"]);
    pick(&mut e, act(0, 0, a, Target::None));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let c = placed_on(&e, "Chronicle of the Lost Tribe", None);
    assert_eq!(e.state.hand_size(SeatId(0)), h0 + 1);
    assert!(!req
        .options
        .contains(&use_on(c, 0, Some(small), Target::None)));
    e.choose(index_of(&req, use_on(c, 0, Some(big), Target::None)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.card_controller(c), SeatId(1));
    assert_eq!(
        (e.state.hand_size(SeatId(0)), e.state.hand_size(SeatId(1))),
        (h0, h1 + 1)
    );
}

/// Seat 0's `a` plays Clan Impersonation (hand slot 0) choosing `clan`.
fn impersonate(e: &mut Engine, a: MinionId, clan: &'static str) {
    pick_when(e, act(0, 0, a, Target::None));
    pick_when(e, Choice::ChooseClan(clan));
}

/// Clan Impersonation: the card on its actor, of the chosen clan, its sect
/// kept (RTR 20201130); burned by its own action, back to its clan.
/// [RTR 20201130] [ANK 20200415]
#[test]
fn clan_impersonation_changes_clan_not_sect_and_reverts() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let sect = e.state.sect_of(a);
    hand(&mut e, 0, &["Clan Impersonation"]);
    impersonate(&mut e, a, "Toreador");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = placed_on(&e, "Clan Impersonation", Some(a));
    assert_eq!(e.state.minions[a.index()].clan(), Some("Toreador"));
    assert_eq!(e.state.sect_of(a), sect);
    e.state.minions[a.index()].locked = false;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick_when(&mut e, use_on(c, 0, Some(a), Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Clan Impersonation"));
    assert_eq!(e.state.minions[a.index()].clan(), Some("Brujah"));
}

/// Clan Impersonation beside Derange: the latest governs (LSJ 20021209):
/// impersonated after a Derange, the chosen clan; that card gone, Derange's
/// clan again; deranged after the impersonation, Derange's. [LSJ 20021209]
#[test]
fn clan_impersonation_and_derange_latest_wins() {
    // Derange first (its effect as played), then the impersonation.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let d = in_play(&mut e, 1, "Derange", Some(a));
    e.state.minions[a.index()].deranged = Some(("Ventrue", Sect::Camarilla, d));
    assert_eq!(e.state.minions[a.index()].clan(), Some("Ventrue"));
    hand(&mut e, 0, &["Clan Impersonation"]);
    impersonate(&mut e, a, "Toreador");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.minions[a.index()].clan(),
        Some("Toreador"),
        "latest"
    );
    let c = placed_on(&e, "Clan Impersonation", Some(a));
    e.state.minions[a.index()].locked = false;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick_when(&mut e, use_on(c, 0, Some(a), Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.minions[a.index()].clan(),
        Some("Ventrue"),
        "Derange's again"
    );
    // The impersonation first, Derange played after it.
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Brujah" && c.capacity <= 6, 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(
        SeatId(1),
        textless(|c| c.clan == "Malkavian" && c.capacity >= 8, 0),
        6,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let malk = e.state.minions[v.index()].clan();
    hand(&mut e, 0, &["Clan Impersonation"]);
    impersonate(&mut e, a, "Toreador");
    hand(&mut e, 1, &["Derange"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick_when(&mut e, act(0, 0, v, Target::Minion(a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].deranged.is_some());
    assert_eq!(e.state.minions[a.index()].clan(), malk, "latest");
}

/// Clan Impersonation keeps a slave its slave status (LSJ 20030511), and
/// idles a justicar's title off its clan (RTR 20201130, LSJ 20060904).
/// [LSJ 20030511] [RTR 20201130] [LSJ 20060904]
#[test]
fn clan_impersonation_keeps_slave_status_and_idles_a_justicar() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), plain("Gangrel", 0), 3, false);
    e.state.minions[t.index()].slave_of = Some("Tremere antitribu");
    let j = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.state.minions[j.index()].granted_title = Some(Title::Justicar("Ventrue"));
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let slave = e.state.slave_clan(t);
    assert!(slave.is_some());
    hand(&mut e, 0, &["Clan Impersonation", "Clan Impersonation"]);
    impersonate(&mut e, t, "Brujah");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[t.index()].clan(), Some("Brujah"));
    assert_eq!(e.state.slave_clan(t), slave);
    assert_eq!(e.state.title_of(j), Title::Justicar("Ventrue"));
    impersonate(&mut e, j, "Brujah");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.title_of(j), Title::None, "off its clan");
}

/// A lasting clan change idles a justicar's title as an override does ("If
/// the vampire changes clan to a clan inappropriate for his title, he loses
/// its benefit until his clan is appropriate"), and gives it back once the
/// clan fits again. [RTR 20201130] [LSJ 20060904]
#[test]
fn a_lasting_clan_change_idles_a_justicar_until_the_clan_fits() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.state.minions[j.index()].granted_title = Some(Title::Justicar("Ventrue"));
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.state.minions[j.index()].clan_change = Some("Brujah");
    assert_eq!(e.state.title_of(j), Title::None, "off its clan");
    e.state.minions[j.index()].clan_change = Some("Ventrue");
    assert_eq!(e.state.title_of(j), Title::Justicar("Ventrue"));
}

/// Clan Impersonation and the scarce toll: a scarce vampire impersonating
/// a Ventrue, banished, influenced back beside a Ventrue pays 3 (its
/// apparent clan while controlled). [LSJ 20011015]
#[test]
fn clan_impersonation_scarce_toll_reads_apparent_clan() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 9, 0), 6, false);
    e.state.minions[a.index()].granted_title = Some(Title::Prince("Paris"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let card = CRYPT
        .iter()
        .position(|c| {
            c.scarce && !c.adv && c.capacity <= 3 && c.clan != "Ventrue" && c.title == Title::None
        })
        .expect("a small scarce vampire") as u16;
    let t = e.add_minion(SeatId(1), card, 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Clan Impersonation"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    impersonate(&mut e, t, "Ventrue");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[t.index()].clan(), Some("Ventrue"));
    hand(&mut e, 0, &["Banishment"]);
    pass_referendum(&mut e, a, &[Choice::ChooseMinion(t)]);
    request(&mut e);
    assert!(!e.state.minions[t.index()].in_play, "banished");
    e.state.seats[1].uncontrolled[0].counters = CRYPT[card as usize].capacity;
    to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    let before = e.state.seats[1].pool;
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    request(&mut e);
    assert!(e.state.minions[t.index()].in_play, "back");
    assert_eq!(e.state.seats[1].pool, before - 3, "one Ventrue: 3 pool");
}

/// Malkavian Dementia's Malkavian, its clan changed meanwhile (Clan
/// Impersonation played by it, as its taker's): it still goes back at its
/// taker's next unlock phase. [ANK 20221208-2]
#[test]
fn malkavian_dementia_holds_when_its_malkavian_changes_clan() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let m = e.add_minion(SeatId(1), of_clan("Malkavian", 4..=8, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Malkavian Dementia", "Clan Impersonation"]);
    e.at_master_phase();
    pick(&mut e, master_on(0, Target::Minion(m)));
    request(&mut e);
    assert_eq!(e.state.minions[m.index()].owner, SeatId(0));
    impersonate(&mut e, m, "Brujah");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[m.index()].clan(), Some("Brujah"));
    assert_eq!(e.state.minions[m.index()].owner, SeatId(0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[m.index()].owner, SeatId(1), "back");
}

/// Cryptic Mission [tha]: another Methuselah's 1-life ally burned by its 1
/// damage is a Ⓓ action burning an ally (ANK 20220507): Predator's
/// Transformation [ABO] answers it. [ANK 20220507]
#[test]
fn cryptic_mission_burning_an_ally_feeds_predators_transformation() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tha", "abo", "abo"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Dark Web Procurer"));
    e.state.minions[ally.index()].blood = 1;
    hand(&mut e, 0, &["Cryptic Mission", "Predator's Transformation"]);
    pick(&mut e, act(0, 1, a, Target::Minion(ally)));
    let req = offering(&mut e, play(0, 3, a));
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    assert!(!e.state.minions[ally.index()].in_play, "burned");
    e.choose(index_of(&req, play(0, 3, a)));
    request(&mut e);
    assert_eq!(pool(&e)[1], 10 - 2);
}

/// Cryptic Mission [THA]: a vampire burns 1 blood, the acting vampire
/// gains 1.
#[test]
fn cryptic_mission_superior_burns_a_vampire_blood_and_feeds_the_actor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 3, false);
    e.state.minions[a.index()].gained_levels = vec!["tha", "tha"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Cryptic Mission"]);
    pick(&mut e, act(0, 3, a, Target::Minion(v)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, v)), (4, 3));
}

/// Deep Cover Agent: the card on another Methuselah's younger vampire,
/// still its player's; in their unlock phase they look at its controller's
/// hand; its bearer burns it by a Ⓓ action at the card's player.
#[test]
fn deep_cover_agent_looks_at_its_hosts_controllers_hand_each_unlock() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.seraph && c.capacity >= 8, 0),
        6,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), textless(|c| c.capacity <= 5, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Deep Cover Agent"]);
    hand(&mut e, 1, &["Dominion"]);
    pick(&mut e, act(0, 0, a, Target::Minion(v)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let c = placed_on(&e, "Deep Cover Agent", Some(v));
    assert_eq!(e.state.card_controller(c), SeatId(0), "still yours");
    let burn = use_on(c, 1, Some(v), Target::None);
    let req = request(&mut e);
    assert!(req.options.contains(&burn));
    e.notes = Some(Vec::new());
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    e.choose(index_of(&req, use_on(c, 0, None, Target::None)));
    request(&mut e);
    assert!(
        e.state.peeks.contains(&(SeatId(0), SeatId(1))),
        "the hand seen"
    );
}

/// Deep Cover Agent's burn option: its player controlling a ready Seraph,
/// not offered; with only a torpid one, offered, a copy of it in play
/// notwithstanding. [LSJ 20091203] [ANK 20240610] [LSJ 20100203-2]
#[test]
fn deep_cover_agent_burn_option_with_only_torpid_seraphs() {
    for torpid in [false, true] {
        let mut e = game(2);
        let sera = e.add_minion(SeatId(0), textless(|c| c.seraph, 0), 4, torpid);
        in_play(&mut e, 0, "Deep Cover Agent", Some(sera));
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Deep Cover Agent"]);
        let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
        assert_eq!(
            req.options.contains(&Choice::Discard(0)),
            torpid,
            "{torpid}"
        );
    }
}

/// Distraction [cel]: draws 5 from a library of 2, as many as it holds
/// (LSJ 20060620), then discards down; [CEL] locks the prey's minion.
/// [LSJ 20060620]
#[test]
fn distraction_draws_from_a_short_library_and_locks_with_superior() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("cel", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for _ in 0..2 {
        e.state.seats[0].library.push(find_library("Form of Mist"));
    }
    hand(&mut e, 0, &["Distraction"]);
    pick(&mut e, act(0, 0, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].library.is_empty(), "drew what it held");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Distraction"]);
    pick(&mut e, act(0, 1, a, Target::Minion(v)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[v.index()].locked);
}

/// Kaymakli Barrier: the card on another Methuselah's younger vampire,
/// still its player's; that vampire cannot block an undirected action (a
/// hunt), may block a directed one (a bleed), and its directed actions cost
/// 1 blood more.
#[test]
fn kaymakli_barrier_bars_undirected_blocks_and_taxes_directed_actions() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 8, 0), 6, false);
    let h = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), textless(|c| c.capacity <= 6, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Kaymakli Barrier"]);
    pick(&mut e, act(0, 0, a, Target::Minion(v)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = placed_on(&e, "Kaymakli Barrier", Some(v));
    assert_eq!(e.state.card_controller(c), SeatId(0));
    pick(&mut e, action(h, ActionKind::Hunt));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, block(v));
    e.state.minions[v.index()].locked = false;
    let before = blood(&e, v);
    pick(&mut e, bleed(v));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, v), before - 1, "the directed toll");
}

/// Mesmerize [dom]: steals an ally's equipment; [DOM] needs a younger
/// vampire's: with none, not offered (RTR 19980928). [RTR 19980928]
#[test]
fn mesmerize_steals_equipment_only_where_there_is_some() {
    let mut e = game(2);
    let dom = fixture_vampire(|c| c.disciplines.contains(&"DOM") && c.capacity <= 7, 0);
    let a = e.add_minion(SeatId(0), dom, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Dark Web Procurer"));
    let cap = CRYPT[dom as usize].capacity;
    let old = e.add_minion(SeatId(1), textless(|c| c.capacity > cap, 0), 4, false);
    let jacket = in_play(&mut e, 1, "Leather Jacket", Some(ally));
    let other = in_play(&mut e, 1, "Leather Jacket", Some(old));
    hand(&mut e, 0, &["Mesmerize"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&act(0, 1, a, Target::Card(other))),
        "not younger"
    );
    e.choose(index_of(&req, act(0, 0, a, Target::Card(jacket))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[jacket.index()].host, Some(a));
}

/// Mind of a Child [DEM]: on another Methuselah's vampire, it cannot play
/// Discipline cards and has 1 capacity less; any vampire burns it, a +1
/// stealth action.
#[test]
fn mind_of_a_child_bars_discipline_cards_and_burns_by_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dem", "dem"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), vampire_with("cel", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cap = e.state.capacity_of(v).unwrap();
    hand(&mut e, 0, &["Mind of a Child"]);
    hand(&mut e, 1, &["Distraction"]);
    pick(&mut e, act(0, 1, a, Target::Minion(v)));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let c = placed_on(&e, "Mind of a Child", Some(v));
    assert_eq!(e.state.capacity_of(v), Some(cap - 1));
    assert!(
        !req.options.contains(&act(0, 0, v, Target::None)),
        "no Discipline cards"
    );
    let burn = use_on(c, 0, Some(b), Target::None);
    assert_eq!(action_stealth(&mut e, burn), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Mind of a Child"));
}

/// Nosferatu Performance Art: a ready Toreador to torpor, +1 stealth for a
/// Nosferatu above 5 capacity only; another Methuselah's Toreador location
/// burned, directed at them. [LSJ 20090324]
#[test]
fn nosferatu_performance_art_torpors_a_toreador_with_stealth_when_old() {
    for (caps, s) in [(6..=11, 1), (4..=5, 0)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_clan("Nosferatu", caps, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let t = e.add_minion(SeatId(1), of_clan("Toreador", 4..=8, 0), 4, false);
        hand(&mut e, 0, &["Nosferatu Performance Art"]);
        assert_eq!(action_stealth(&mut e, act(0, 0, a, Target::Minion(t))), s);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[t.index()].in_torpor);
    }
}

/// Of Noble Blood: its primogen makes another Methuselah's untitled
/// Camarilla vampire of capacity 6 a primogen, the card on it (a directed
/// action, ANK 20190606); a capacity-5 one not offered. [ANK 20190606]
#[test]
fn of_noble_blood_makes_another_methuselahs_vampire_primogen_directed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    e.state.minions[a.index()].granted_title = Some(Title::Primogen);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let six = e.add_minion(
        SeatId(1),
        textless(
            |c| c.sect == Some(Sect::Camarilla) && c.capacity == 6 && c.clan != "Caitiff",
            0,
        ),
        4,
        false,
    );
    let five = e.add_minion(
        SeatId(1),
        textless(
            |c| c.sect == Some(Sect::Camarilla) && c.capacity == 5 && c.clan != "Caitiff",
            0,
        ),
        4,
        false,
    );
    hand(&mut e, 0, &["Of Noble Blood"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&act(0, 0, a, Target::Minion(five))));
    e.choose(index_of(&req, act(0, 0, a, Target::Minion(six))));
    request(&mut e);
    assert_eq!(
        e.action().expect("the action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Of Noble Blood", Some(six));
    assert_eq!(e.state.title_of(six), Title::Primogen);
}

/// Precognizant Mobility [aus]: unlocks another Methuselah's ally; [AUS]
/// the acting vampire itself.
#[test]
fn precognizant_mobility_unlocks_an_ally_or_with_superior_itself() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("aus", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Dark Web Procurer"));
    e.state.minions[ally.index()].locked = true;
    hand(&mut e, 0, &["Precognizant Mobility"]);
    pick(&mut e, act(0, 0, a, Target::Minion(ally)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].locked);
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("AUS", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Precognizant Mobility"]);
    pick(&mut e, act(0, 1, a, Target::Minion(a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked, "unlocked itself");
}

/// Reunion Kamut: 2 blood on a Black Hand vampire of its player's
/// uncontrolled region; one not Black Hand is not offered.
#[test]
fn reunion_kamut_adds_two_to_an_uncontrolled_black_hand() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.black_hand && c.capacity >= 4, 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    uncontrolled(&mut e, 0, plain("Toreador", 0), 0);
    uncontrolled(&mut e, 0, textless(|c| c.black_hand, 1), 0);
    hand(&mut e, 0, &["Reunion Kamut"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&act(0, 0, a, Target::Uncontrolled(0))));
    e.choose(index_of(&req, act(0, 0, a, Target::Uncontrolled(1))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled[1].counters, 2);
}

/// Speed of Thought [CEL]: a vampire with basic Celerity cannot block it,
/// one with superior may; the pool burned capped at 1.
#[test]
fn speed_of_thought_superior_bars_blockers_without_superior_celerity() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let basic = e.add_minion(SeatId(1), vampire_with("cel", 1), 4, false);
    let sup = e.add_minion(SeatId(1), vampire_with("CEL", 1), 4, false);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Bleed(2),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    hand(&mut e, 0, &["Speed of Thought"]);
    pick(&mut e, act(0, 1, a, Target::None));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(!req.options.contains(&block(basic)));
    assert!(req.options.contains(&block(sup)));
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9, "capped at 1");
}

/// Trainer [ani]: steals another Methuselah's animal retainer onto the
/// acting vampire.
#[test]
fn trainer_steals_an_animal_retainer_from_another_methuselah() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ani", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = in_play(&mut e, 1, "Wolf Companion", Some(v));
    hand(&mut e, 0, &["Trainer"]);
    pick(&mut e, act(0, 0, a, Target::Card(w)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[w.index()].host, Some(a));
}

/// Trainer [ANI]: on its acting vampire; as it acts, another minion's
/// animal retainer loses its text, in the combat too; its own does not.
#[test]
fn trainer_silences_other_animal_retainers_while_it_acts() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ANI", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let w = in_play(&mut e, 1, "Wolf Companion", Some(v));
    let own = in_play(&mut e, 0, "Wolf Companion", Some(a));
    hand(&mut e, 0, &["Trainer"]);
    pick(&mut e, act(0, 1, a, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    placed_on(&e, "Trainer", Some(a));
    assert!(!e.state.text_suppressed(w), "not acting");
    e.state.minions[a.index()].locked = false;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a));
    request(&mut e);
    assert!(e.state.text_suppressed(w), "acting");
    assert!(!e.state.text_suppressed(own), "its own");
}

/// Seat 0's Toreador `a` (4 blood) plays The Portrait at seat 1, whose
/// crypt top is `top`: `a`.
fn portrait(e: &mut Engine, top: u16) -> MinionId {
    let a = e.add_minion(SeatId(0), of_clan("Toreador", 4..=8, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1].crypt.push(top);
    hand(e, 0, &["Portrait, The"]);
    let i = e.state.seats[0].hand.len() as u8 - 1;
    pick(e, act(i, 0, a, Target::Seat(SeatId(1))));
    a
}

/// The Portrait, a capacity-8 card shown: its vampire unlocks and its next
/// action is the bleed of the prey, at +1 stealth and +1 bleed (LSJ
/// 20020509: a separate action), the card still on the crypt. [LSJ
/// 20020221] "This is a cardless bleed": Cat Burglary, a bleed card in
/// hand, is no way to take it (a second forced bleed, the other vampire's,
/// keeps the window); [LSJ 20030922] the bonus on that bleed only.
#[test]
fn the_portrait_unlocks_and_forces_a_cardless_boosted_bleed_on_a_mid_reveal() {
    let mut e = game(2);
    e.notes = Some(Vec::new());
    let top = textless(|c| c.capacity == 8, 0);
    hand(&mut e, 0, &["Cat Burglary"]);
    let a = portrait(&mut e, top);
    e.state.minions[a.index()].gained_levels = vec!["cel"];
    e.state.forced_bleeds.0.push((MinionId(1), SeatId(0)));
    let bleed0 = e.state.current_bleed(a);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked, "unlocked");
    assert!(req.options.contains(&bleed(a)), "{:?}", req.options);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == a)),
        "cardless: {:?}",
        req.options
    );
    e.choose(index_of(&req, bleed(a)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].crypt.last(), Some(&top), "shown only");
    let bleeds: Vec<_> = noted_actions(&mut e)
        .into_iter()
        .filter(|n| n.ended && n.action.acting == a && n.action.kind == ActionKind::Bleed)
        .collect();
    assert_eq!(bleeds.len(), 1);
    assert!(bleeds[0].action.card.is_none(), "cardless");
    assert_eq!(bleeds[0].action.stealth(&e.state), 1, "+1 stealth");
    let other = i32::from(e.state.current_bleed(MinionId(1)));
    assert_eq!(
        e.state.seats[1].pool,
        10 - other - i32::from(bleed0) - 1,
        "+1 bleed"
    );
    assert!(e.state.seats[0]
        .hand
        .contains(&find_library("Cat Burglary")));
}

/// The Portrait after its vampire bled this turn: the mid reveal only
/// unlocks it. [LSJ 20021118]
#[test]
fn the_portrait_after_a_bleed_only_unlocks() {
    let mut e = game(2);
    let top = textless(|c| c.capacity == 9, 0);
    let a = e.add_minion(SeatId(0), of_clan("Toreador", 4..=8, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1].crypt.push(top);
    e.state.minions[a.index()].bled_this_turn = true;
    hand(&mut e, 0, &["Portrait, The"]);
    pick(&mut e, act(0, 0, a, Target::Seat(SeatId(1))));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked);
    assert!(req.options.contains(&Choice::Pass), "no bleed due");
}

/// The Portrait, a capacity-4 Nosferatu shown: its vampire burns 2, then
/// gains 2; a capacity-11 card: burns 1.
#[test]
fn the_portrait_on_a_nosferatu_burns_and_gains() {
    let mut e = game(2);
    let a = portrait(&mut e, of_clan("Nosferatu", 3..=6, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1 - 2 + 2);
    let mut e = game(2);
    let big = CRYPT
        .iter()
        .position(|c| c.capacity == 11 && !c.imbued)
        .expect("a capacity-11 vampire") as u16;
    let a = portrait(&mut e, big);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1 - 1);
}

/// Seat 0's [obf]/[OBF] vampire (`superior`), beside another; seat 1 with
/// a vampire: the actor.
fn glimpse_actor(e: &mut Engine, superior: bool) -> MinionId {
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 7, 0), 6, false);
    e.state.minions[a.index()].gained_levels = if superior {
        vec!["obf", "obf"]
    } else {
        vec!["obf"]
    };
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    a
}

/// Memory's Fading Glimpse [obf]: the prey's uncontrolled vampire named as
/// played (slot 1 of 2) to the bottom of their crypt, its 3 blood to their
/// pool, the action directed at them; [OBF]: 1 of it to the actor.
#[test]
fn memorys_fading_glimpse_sends_a_prey_crypt_card_to_the_bottom_and_its_blood_to_pool() {
    for superior in [false, true] {
        let mut e = game(2);
        let a = glimpse_actor(&mut e, superior);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let gone = plain("Toreador", 1);
        uncontrolled(&mut e, 1, plain("Toreador", 0), 2);
        uncontrolled(&mut e, 1, gone, 3);
        e.state.seats[1].crypt.push(plain("Toreador", 2));
        hand(&mut e, 0, &["Memory's Fading Glimpse"]);
        pick(&mut e, act(0, u8::from(superior), a, Target::None));
        pick_when(&mut e, Choice::Slot(1));
        let req = request(&mut e);
        assert!(req.options.contains(&block(z)), "directed at the prey");
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].uncontrolled.len(), 1);
        assert_eq!(e.state.seats[1].crypt[0], gone, "the bottom");
        let to_actor = u8::from(superior);
        assert_eq!(e.state.seats[1].pool, 10 + 3 - i32::from(to_actor));
        assert_eq!(blood(&e, a), 6 - 3 + to_actor);
    }
}

/// Memory's Fading Glimpse [OBF] with its prey ousted before it resolves:
/// "your prey" is read as it resolves (rules-qa 124.42, 121.120) and the
/// card was named as played (D1): the named card is in no prey's region,
/// the action has no effect. Seat 1 (1 pool) attempts to block and is
/// ousted by its own Direct Intervention on the actor's Cloak the
/// Gathering: no blood to the actor (only the card's 3 paid), the card in
/// no crypt.
#[test]
fn memorys_fading_glimpse_fizzles_once_its_prey_is_ousted() {
    let mut e = game(3);
    let a = glimpse_actor(&mut e, true);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    let gone = plain("Toreador", 1);
    uncontrolled(&mut e, 1, plain("Toreador", 0), 2);
    uncontrolled(&mut e, 1, gone, 3);
    e.state.seats[1].pool = 1;
    hand(
        &mut e,
        0,
        &["Memory's Fading Glimpse", "Cloak the Gathering"],
    );
    hand(&mut e, 1, &["Direct Intervention"]);
    pick(&mut e, act(0, 1, a, Target::None));
    pick_when(&mut e, Choice::Slot(1));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::None,
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert_eq!(blood(&e, a), 6 - 3, "no blood to the actor");
    assert!(!e.state.seats.iter().any(|s| s.crypt.contains(&gone)));
}

/// Memory's Fading Glimpse on a banished vampire: the card it carried
/// burns (SFC 19960919); on a banished created vampire (The Embrace's):
/// burned, its card to the ash heap, nothing in the crypt (LSJ
/// 20041201-2; a library card never to a crypt, LSJ 20041201). [SFC
/// 19960919] [LSJ 20041201-2] [LSJ 20041201]
#[test]
fn memorys_fading_glimpse_burns_a_created_vampire() {
    let mut e = game(2);
    let a = glimpse_actor(&mut e, false);
    e.state.minions[a.index()].granted_title = Some(Title::Prince("Paris"));
    let s = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Embrace, The"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, act(0, 0, s, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let created = MinionId(e.state.minions.len() as u32 - 1);
    let jacket = in_play(&mut e, 1, "Leather Jacket", Some(created));
    hand(&mut e, 0, &["Banishment", "Memory's Fading Glimpse"]);
    pass_referendum(&mut e, a, &[Choice::ChooseMinion(created)]);
    request(&mut e);
    assert_eq!(e.state.seats[1].uncontrolled.len(), 1, "banished");
    e.state.minions[a.index()].locked = false;
    let crypt = e.state.seats[1].crypt.len();
    pick_when(&mut e, act(0, 0, a, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].uncontrolled.is_empty());
    assert_eq!(e.state.seats[1].crypt.len(), crypt, "no crypt card");
    assert!(ash_has(&e, 1, "Embrace, The"));
    assert!(ash_has(&e, 1, "Leather Jacket"), "the card on it burned");
    assert!(!e.state.cards[jacket.index()].in_play);
}

/// Memory's Fading Glimpse on a banished merged vampire: both its crypt
/// cards to the bottom, the advanced lowest. [LSJ 20030422]
#[test]
fn memorys_fading_glimpse_unmerges_a_banished_merged_vampire() {
    let named = |adv: bool| {
        (0..CRYPT.len() as u16)
            .find(|&c| CRYPT[c as usize].name == "Appolonius" && CRYPT[c as usize].adv == adv)
            .expect("Appolonius")
    };
    let mut e = game(2);
    let cap = CRYPT[named(false) as usize].capacity;
    let old = CRYPT
        .iter()
        .position(|c| c.capacity > cap && !c.imbued && c.sect == Some(Sect::Camarilla))
        .expect("an older vampire") as u16;
    let a = e.add_minion(SeatId(0), old, 8, false);
    e.state.minions[a.index()].gained_levels = vec!["obf"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].granted_title = Some(Title::Prince("Paris"));
    let ap = e.add_minion(SeatId(1), named(false), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    uncontrolled(&mut e, 1, named(true), 3);
    let req = to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    e.choose(index_of(&req, Choice::Merge { slot: 0 }));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[ap.index()].merged.is_some());
    hand(&mut e, 0, &["Banishment", "Memory's Fading Glimpse"]);
    pass_referendum(&mut e, a, &[Choice::ChooseMinion(ap)]);
    request(&mut e);
    e.state.minions[a.index()].locked = false;
    pick_when(&mut e, act(0, 0, a, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let crypt = &e.state.seats[1].crypt;
    assert!(crypt.len() >= 2);
    let (a0, a1) = (crypt[0], crypt[1]);
    assert_eq!([a0, a1].map(|c| CRYPT[c as usize].name), ["Appolonius"; 2]);
    assert!(CRYPT[a0 as usize].adv, "the advanced lowest");
}

/// Resilient Mind: as another Methuselah's vampire plays Mind of a Child
/// [dem] on its vampire, it burns 1 blood: the card has no effect on it
/// (not put on it, to the ash heap; LSJ 20040729's "would fall off").
#[test]
fn resilient_mind_spares_its_vampire_a_dementation_action_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dem"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let r = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let rm = in_play(&mut e, 1, "Resilient Mind", Some(r));
    hand(&mut e, 0, &["Mind of a Child"]);
    zvtes::copies::number_unnumbered(&mut e);
    pick(&mut e, act(0, 0, a, Target::Minion(r)));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    e.choose(index_of(&req, use_on(rm, 0, None, Target::None)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, r), 3, "its blood");
    assert!(ash_has(&e, 0, "Mind of a Child"), "fell off");
    assert!(!e
        .state
        .bears(r, zvtes::cards::Modifier::BearerCannotPlayDisciplines));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.immune_to.0.is_empty(), "ended with the turn");
}

/// Resilient Mind against a card with no copy (a fixture's): no immunity
/// taken (copies are never the game's identity), so a hand strike, which
/// has no copy either, still lands on its vampire.
#[test]
fn resilient_mind_leaves_a_hand_strike_landing() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dem"];
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let r = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let rm = in_play(&mut e, 1, "Resilient Mind", Some(r));
    hand(&mut e, 0, &["Mind of a Child"]);
    pick(&mut e, act(0, 0, a, Target::Minion(r)));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    e.choose(index_of(&req, use_on(rm, 0, None, Target::None)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, r), 3, "its blood");
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(r));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, r), 2, "the hand strike landed");
}

/// Resilient Mind on a retainer card: "an action … card" is every action
/// card type (rules-qa 60.4), offered on any such play (LSJ 20031125): as
/// another Methuselah's vampire employs Hatch the Viper [pro][ser], its use is
/// offered (a legal no-op).
#[test]
fn resilient_mind_is_offered_on_a_retainer_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["pro", "ser"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let r = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let rm = in_play(&mut e, 1, "Resilient Mind", Some(r));
    hand(&mut e, 0, &["Hatch the Viper"]);
    zvtes::copies::number_unnumbered(&mut e);
    let req = request(&mut e);
    let employ = *req
        .options
        .iter()
        .find(|o| matches!(o, Choice::Play { card: 0, actor: Some(x), .. } if *x == a))
        .expect("Hatch the Viper employed");
    e.choose(index_of(&req, employ));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert!(req.options.contains(&use_on(rm, 0, None, Target::None)));
}

/// Resilient Mind used as Form of Corruption [ser] is put into play: that
/// Form of Corruption later cannot steal its vampire (LSJ 20031223).
/// [LSJ 20031223]
#[test]
fn resilient_mind_protects_from_form_of_corruption() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["ser"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let r = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let rm = in_play(&mut e, 1, "Resilient Mind", Some(r));
    hand(&mut e, 0, &["Form of Corruption"]);
    zvtes::copies::number_unnumbered(&mut e);
    pick(&mut e, act(0, 0, a, Target::None));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    e.choose(index_of(&req, use_on(rm, 0, None, Target::None)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let foc = placed_on(&e, "Form of Corruption", None);
    e.state.cards[foc.index()].counters = 3;
    assert_eq!(blood(&e, r), 0);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    e.choose(index_of(&req, use_on(foc, 0, None, Target::Minion(r))));
    request(&mut e);
    assert_eq!(e.state.minions[r.index()].owner, SeatId(1), "not stolen");
}

/// Thin-Blooded Seer: played by a capacity-2 vampire, the card becomes a
/// 1-capacity clanless sterile vampire of its sect that must hunt this
/// turn; a 1-blood card costs it 2 (not offered at 1 blood, a 1-blood
/// vampire beside it offered it); in its unlock phase it locks to look at
/// the top two cards of a library. [PIB 20150917] [RTR 20070707]
#[test]
fn thin_blooded_seer_enters_clanless_sterile_and_pays_double_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 2, 0), 2, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let sect = e.state.sect_of(a);
    hand(&mut e, 0, &["Thin-Blooded Seer"]);
    pick(&mut e, act(0, 0, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let s = MinionId(e.state.minions.len() as u32 - 1);
    assert!(matches!(
        e.state.minions[s.index()].kind,
        zvtes::state::MinionKind::Created { .. }
    ));
    assert!(e.state.minions[s.index()].is_vampire());
    assert_eq!(e.state.minions[s.index()].clan(), None, "clanless");
    assert!(e.state.is_sterile(s));
    assert_eq!(e.state.sect_of(s), sect);
    assert_eq!(e.state.capacity_of(s), Some(1));
    fill_hand(&mut e, 1);
    for _ in 0..3 {
        e.state.seats[1].library.push(find_library("Form of Mist"));
    }
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let look = Choice::UseAbility {
        source: AbilitySource::Card(placed_on(&e, "Thin-Blooded Seer", None)),
        ability: 0,
        actor: None,
        target: Target::Seat(SeatId(1)),
    };
    e.choose(index_of(&req, look));
    request(&mut e);
    assert!(e.state.minions[s.index()].locked, "locked to look");
    e.state.minions[s.index()].locked = false;
    e.state.minions[s.index()].blood = 1;
    hand(&mut e, 0, &["Catacombs"]);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let catacombs = |m: MinionId| {
        req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { actor: Some(x), .. } if *x == m))
    };
    assert!(catacombs(b), "a 1-blood vampire may");
    assert!(!catacombs(s), "costs the Seer 2");
}

/// Seat 0's Thin-Blooded Seer, created by a capacity-2 vampire, made
/// Anarch, at 1 blood, unlocked, at seat 0's next minion phase.
fn anarch_seer(e: &mut Engine) -> MinionId {
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 2, 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(e, 0, &["Thin-Blooded Seer"]);
    pick(e, act(0, 0, a, Target::None));
    until(e, SeatId(1), WindowLabel::MinionPhase);
    let s = MinionId(e.state.minions.len() as u32 - 1);
    e.state.minions[s.index()].sect_override = Some(Sect::Anarch);
    s
}

/// Thin-Blooded Seer: "Cards that cost blood cost this vampire twice as much
/// blood to play": an X cost too, X chosen then doubled (rules-qa 70.17).
/// Monkey Wrench ("X must be 1, 2 or 3", X blood) at X = 1 costs it 2: at
/// 1 blood, not offered; a 1-blood Anarch beside it offered it.
#[test]
fn thin_blooded_seer_doubles_an_x_cost() {
    let mut e = game(2);
    let s = anarch_seer(&mut e);
    let o = MinionId(1);
    e.state.minions[o.index()].sect_override = Some(Sect::Anarch);
    hand(&mut e, 0, &["Monkey Wrench"]);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[s.index()].blood = 1;
    e.state.minions[o.index()].blood = 1;
    pick(&mut e, bleed(s));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, s));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(o));
    offering(&mut e, play(0, 0, o));
}

/// Thin-Blooded Seer and a reduction: [PIB 20150917] [RTR 20070707] "When
/// computing the cost of a card, multiplications and divisions are applied
/// first, followed by additions and subtractions." With The Shard's 2 off
/// (the Edge held), the 1-blood Seer is offered Catacombs (1: 2 - 2 = 0),
/// not Clan Impersonation (2: 4 - 2 = 2; halved order, (2 - 2) x 2 = 0);
/// Catacombs paid nothing, the Shard locked (its only payment, taken on
/// the way).
#[test]
fn thin_blooded_seer_doubles_before_reductions() {
    let mut e = game(2);
    let s = anarch_seer(&mut e);
    hand(&mut e, 0, &["Catacombs", "Clan Impersonation"]);
    let shard = in_play(&mut e, 0, "Shard, London, The", None);
    e.state.seats[0].has_edge = true;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[s.index()].blood = 1;
    let req = request(&mut e);
    let ours = |c: u8| {
        req.options.iter().any(
            |o| matches!(o, Choice::Play { card, actor: Some(x), .. } if *card == c && *x == s),
        )
    };
    assert!(ours(0), "Catacombs: {:?}", req.options);
    assert!(!ours(1), "Clan Impersonation");
    let catacombs = *req
        .options
        .iter()
        .find(|o| matches!(o, Choice::Play { card: 0, actor: Some(x), .. } if *x == s))
        .expect("Catacombs");
    e.choose(index_of(&req, catacombs));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.cards[shard.index()].locked, "the Shard used");
    assert_eq!(blood(&e, s), 1, "paid nothing");
}

/// Nosferatu Performance Art: another Methuselah's location requiring a
/// Toreador (Art Museum) burned, the action directed at them. [LSJ
/// 20090324]
#[test]
fn nosferatu_performance_art_burns_a_toreador_location_directed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Nosferatu", 4..=8, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let museum = in_play(&mut e, 1, "Art Museum", None);
    hand(&mut e, 0, &["Nosferatu Performance Art"]);
    pick(&mut e, act(0, 1, a, Target::Card(museum)));
    request(&mut e);
    assert_eq!(
        e.action().expect("the action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Art Museum"));
}

// ---- Political actions (political) -----------------------------------------

/// The effect-only stand-ins of `name` in play.
fn stand_ins(e: &Engine, name: &str) -> Vec<CardId> {
    let card = find_library(name);
    (0..e.state.cards.len())
        .filter(|&i| {
            let c = &e.state.cards[i];
            c.card == card && c.present() && c.effect_only
        })
        .map(|i| CardId(i as u32))
        .collect()
}

/// A seat-0 Camarilla vampire holding `title`, beside another seat-0
/// vampire (the minion phase not elided).
fn titled_caller(e: &mut Engine, title: Title) -> MinionId {
    let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    e.state.minions[a.index()].granted_title = Some(title);
    e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 1), 4, false);
    a
}

/// Camarilla Exemplary: another Methuselah's torpid Camarilla vampire may
/// be chosen; passed, a stand-in on the chosen one, the card in the ash
/// heap; a blocker with no blood still blocks (LSJ 19970707); chosen again,
/// the burns add (LSJ 19980225, 19970707). [LSJ 19970707] [LSJ 19980225]
#[test]
fn camarilla_exemplary_tolls_blockers_for_the_game() {
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Prince("Paris"));
    let torpid = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 2), 2, true);
    let t = MinionId(1);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    let c = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Camarilla Exemplary", "Camarilla Exemplary"]);
    pick(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    assert!(
        req.options.contains(&Choice::ChooseMinion(torpid)),
        "torpid too"
    );
    e.choose(index_of(&req, Choice::ChooseMinion(t)));
    pick_matching(&mut e, |o| {
        *o == Choice::CastCardVote { favor: true } || *o == votes(a, true)
    });
    close_polling(&mut e);
    request(&mut e);
    assert_eq!(stand_ins(&e, "Camarilla Exemplary").len(), 1);
    assert!(ash_has(&e, 0, "Camarilla Exemplary"));
    assert_eq!(e.state.blockers_of_burn(t), 1);
    pick_when(&mut e, bleed(t));
    e.notes = Some(Vec::new());
    pick_when(&mut e, block(b));
    request(&mut e);
    assert!(!noted_combats(&mut e).is_empty(), "blocked with no blood");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[a.index()].locked = false;
    e.state.minions[a.index()].political_this_turn = false;
    pass_referendum(&mut e, a, &[Choice::ChooseMinion(t)]);
    request(&mut e);
    assert_eq!(e.state.blockers_of_burn(t), 2, "the same vampire again");
    e.state.minions[t.index()].locked = false;
    e.state.minions[t.index()].bled_this_turn = false;
    pick_when(&mut e, bleed(t));
    pick_when(&mut e, block(c));
    request(&mut e);
    assert!(blood(&e, c) <= 2, "two burned");
}

/// Unleash Hell's Fury, a 0-blood vampire as it attempts to block, may
/// block a Camarilla Exemplary vampire. [LSJ 19970707] [ANK 20230226]
#[test]
fn unleash_hells_fury_blocks_a_camarilla_exemplary_vampire() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nergal"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let k = in_play(&mut e, 1, "Camarilla Exemplary", Some(z));
    e.state.cards[k.index()].effect_only = true;
    hand(&mut e, 0, &["Unleash Hell's Fury"]);
    pick(&mut e, play(0, 0, n));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let token = MinionId(
        e.state
            .minions
            .iter()
            .position(|m| matches!(m.kind, zvtes::state::MinionKind::Fury { .. }))
            .expect("its token") as u32,
    );
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, bleed(z));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(token)), "{:?}", req.options);
}

/// Command of the Harpies: a ready prince (Praxis Seizure: Boston's)
/// loses the title, its card burned; with only a torpid prince, not
/// played. [LSJ 20100129] [ANK 20191228]
#[test]
fn command_of_the_harpies_strips_a_praxis_seizure_prince() {
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Primogen);
    let p = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 2), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 1, "Praxis Seizure: Boston", Some(p));
    assert_eq!(e.state.title_of(p), Title::Prince("Boston"));
    hand(&mut e, 0, &["Command of the Harpies"]);
    pass_referendum(&mut e, a, &[Choice::ChooseMinion(p)]);
    request(&mut e);
    assert_eq!(e.state.title_of(p), Title::None);
    assert!(ash_has(&e, 1, "Praxis Seizure: Boston"));
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Primogen);
    let p = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 2), 4, true);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Command of the Harpies"]);
    assert!(!request(&mut e).options.contains(&play(0, 0, a)));
}

/// Crusade: Barcelona: the card on its Tremere antitribu caller, the
/// unique Archbishop of Barcelona title, the caller unlocked at the next
/// discard phase; Crusade: Brussels on a caller not Toreador antitribu:
/// still locked; Miami titles its caller.
#[test]
fn crusades_title_their_callers_and_unlock_the_clan() {
    for (card, clan, city, unlocks) in [
        ("Crusade: Barcelona", "Tremere antitribu", "Barcelona", true),
        ("Crusade: Brussels", "Tremere antitribu", "Brussels", false),
        ("Crusade: Miami", "Tremere antitribu", "Miami", false),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_clan(clan, 4..=8, 0), 4, false);
        e.state.minions[a.index()].granted_title = Some(Title::Bishop);
        e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=7, 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &[card]);
        pass_referendum(&mut e, a, &[]);
        request(&mut e);
        placed_on(&e, card, Some(a));
        assert_eq!(e.state.title_of(a), Title::Archbishop(city));
        to_minion_phase(&mut e, SeatId(1));
        assert_eq!(!e.state.minions[a.index()].locked, unlocks, "{card}");
    }
}

/// Praxis Seizure: Boston, Cleveland, Dallas, Seattle: the card on its
/// caller, its city's prince; two of one Methuselah's vampires holding
/// Boston and Dallas do not contest.
#[test]
fn praxis_seizures_title_their_callers_with_distinct_cities() {
    for (card, city) in [
        ("Praxis Seizure: Boston", "Boston"),
        ("Praxis Seizure: Cleveland", "Cleveland"),
        ("Praxis Seizure: Dallas", "Dallas"),
        ("Praxis Seizure: Seattle", "Seattle"),
    ] {
        let mut e = game(2);
        let a = titled_caller(&mut e, Title::Primogen);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &[card]);
        pass_referendum(&mut e, a, &[]);
        request(&mut e);
        placed_on(&e, card, Some(a));
        assert_eq!(e.state.title_of(a), Title::Prince(city));
    }
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Primogen);
    let b = MinionId(1);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Praxis Seizure: Boston", Some(b));
    hand(&mut e, 0, &["Praxis Seizure: Dallas"]);
    pass_referendum(&mut e, a, &[]);
    request(&mut e);
    assert_eq!(e.state.title_of(a), Title::Prince("Dallas"));
    assert_eq!(e.state.title_of(b), Title::Prince("Boston"));
}

/// Seat 0's titled caller with seat 1 holding `vp_halves` received on
/// `turn`.
fn lextalionis_game(turn: u32, halves: u32) -> (Engine, MinionId, Vec<MinionId>) {
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Prince("Paris"));
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, true);
    let inf = e.add_minion(
        SeatId(1),
        fixture_vampire(|c| c.infernal && c.capacity >= 4, 0),
        4,
        false,
    );
    e.state.seats[0].previous_turn = 2;
    e.state.seats[0].this_turn = 4;
    e.state.seats[1].vps_received.0.push((turn, halves));
    (e, a, vec![x, y, inf])
}

/// Lextalionis: a Methuselah who received a victory point since the
/// caller's last turn (turn 3, after its turn 2) is chosen: all their
/// minions lock, torpid too, and do not unlock at their next unlock phase;
/// the infernal one may still pay a pool (LSJ 20050114); the phase after,
/// they unlock. [LSJ 20020429] [LSJ 20050114]
#[test]
fn lextalionis_locks_a_recent_ouster_out_of_its_next_unlock() {
    let (mut e, a, ms) = lextalionis_game(3, 2);
    hand(&mut e, 0, &["Lextalionis"]);
    pass_referendum(&mut e, a, &[Choice::ChooseSeat(SeatId(1))]);
    request(&mut e);
    assert!(ms.iter().all(|m| e.state.minions[m.index()].locked));
    let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    assert!(
        req.options
            .iter()
            .any(|o| matches!(o, Choice::UnlockInfernal { minion } if *minion == ms[2])),
        "{:?}",
        req.options
    );
    assert!(e.state.minions[ms[0].index()].locked, "not as normal");
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    assert!(!e.state.minions[ms[0].index()].locked, "the phase after");
}

/// Lextalionis: no Methuselah received a whole point since the caller's
/// last turn (one on its turn 2, or a half since): not played. [LSJ
/// 20020429] [LSJ 20100129] [ANK 20191228]
#[test]
fn lextalionis_needs_a_victory_point_since_your_last_turn() {
    for (turn, halves) in [(2, 2), (3, 1)] {
        let (mut e, a, _) = lextalionis_game(turn, halves);
        hand(&mut e, 0, &["Lextalionis"]);
        assert!(
            !request(&mut e).options.contains(&play(0, 0, a)),
            "{turn} {halves}"
        );
    }
}

/// Masquerade Enforcement: passed by a prince, the card in play; a vampire
/// influenced out costs its Methuselah 1 more pool; a merge none (LSJ
/// 20030527); a Camarilla vampire burns it by a referendum. [LSJ 20030527]
#[test]
fn masquerade_enforcement_taxes_every_vampire_brought_out() {
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Prince("Paris"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Masquerade Enforcement"]);
    pass_referendum(&mut e, a, &[]);
    request(&mut e);
    let me = placed_on(&e, "Masquerade Enforcement", None);
    let v = plain("Toreador", 0);
    uncontrolled(&mut e, 1, v, CRYPT[v as usize].capacity);
    let req = to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    let before = e.state.seats[1].pool;
    e.choose(index_of(&req, Choice::InfluenceOut { slot: 0 }));
    request(&mut e);
    assert_eq!(e.state.seats[1].pool, before - 1);
    let _ = me;
}

/// Masquerade Enforcement spares a merge. [LSJ 20030527]
#[test]
fn masquerade_enforcement_spares_a_merge() {
    let named = |adv: bool| {
        (0..CRYPT.len() as u16)
            .find(|&c| CRYPT[c as usize].name == "Appolonius" && CRYPT[c as usize].adv == adv)
            .expect("Appolonius")
    };
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), named(false), 5, false);
    in_play(&mut e, 0, "Masquerade Enforcement", None);
    uncontrolled(&mut e, 1, named(true), 3);
    let req = to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    let before = e.state.seats[1].pool;
    e.choose(index_of(&req, Choice::Merge { slot: 0 }));
    request(&mut e);
    assert_eq!(e.state.seats[1].pool, before - 1, "the merge's 1 pool only");
}

/// Masquerade Enforcement: a Camarilla vampire, any Methuselah's, burns it
/// by a +1 stealth referendum (an action requiring a Camarilla vampire,
/// LSJ 20080604); not offered to a Sabbat one; not played without a
/// prince or justicar. [LSJ 20080604]
#[test]
fn masquerade_enforcement_burned_by_a_camarilla_vampire() {
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Prince("Paris"));
    let s = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=7, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let me = in_play(&mut e, 1, "Masquerade Enforcement", None);
    let req = request(&mut e);
    assert!(!req.options.contains(&use_on(me, 0, Some(s), Target::None)));
    let burn = use_on(me, 0, Some(a), Target::None);
    assert_eq!(action_stealth(&mut e, burn), 1);
    pick_matching(&mut e, |o| *o == votes(a, true));
    close_polling(&mut e);
    request(&mut e);
    assert!(ash_has(&e, 1, "Masquerade Enforcement"));
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Masquerade Enforcement"]);
    assert!(
        !request(&mut e).options.contains(&play(0, 0, a)),
        "untitled"
    );
}

/// Reversal of Fortunes: passed by seat 0 of four on turn 4, the turn
/// continues (LSJ 20001012), then seat 3 plays, then seat 2; the prey is
/// still seat 1 (the impulse: `reversal_of_fortunes_keeps_impulse_clockwise`).
#[test]
fn reversal_of_fortunes_turns_play_to_the_right() {
    let mut e = game(4);
    let a = titled_caller(&mut e, Title::Prince("Paris"));
    for s in 1..4 {
        e.add_minion(SeatId(s), plain("Ventrue", s as usize), 4, false);
    }
    hand(&mut e, 0, &["Reversal of Fortunes"]);
    pass_referendum(&mut e, a, &[]);
    let req = request(&mut e);
    assert_eq!(
        (req.seat, e.state.active),
        (SeatId(0), SeatId(0)),
        "still seat 0's turn"
    );
    assert_eq!(e.state.prey(SeatId(0)), SeatId(1));
    until(&mut e, SeatId(3), WindowLabel::MinionPhase);
    assert_eq!(e.state.turn, 5);
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert_eq!(e.state.turn, 6);
}

/// Reversal of Fortunes reverses the turn sequence only (rules-qa
/// 126.162): with the order reversed, seat 0's hunt is offered to its prey's
/// block (seat 1) before its predator's (seat 3), as clockwise; its bleed
/// ousting seat 1, Life Boon is asked of seat 2 before seat 3.
#[test]
fn reversal_of_fortunes_keeps_impulse_clockwise() {
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    for s in 1..4 {
        e.add_minion(SeatId(s), plain("Ventrue", s as usize), 4, false);
    }
    e.state.turn_reversed.0 = 1;
    pick(&mut e, action(a, ActionKind::Hunt));
    let mut asked = Vec::new();
    while e.action().is_some() {
        let req = request(&mut e);
        if req.window == WindowLabel::ActionBlock && !asked.contains(&req.seat) {
            asked.push(req.seat);
        }
        e.choose(fallback(&req));
    }
    assert_eq!(asked, vec![SeatId(1), SeatId(3)], "clockwise");

    let mut e = game(4);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    for s in 1..4 {
        e.add_minion(SeatId(s), plain("Ventrue", s as usize), 4, false);
    }
    e.state.turn_reversed.0 = 1;
    e.state.seats[1].pool = 1;
    hand(&mut e, 2, &["Life Boon"]);
    hand(&mut e, 3, &["Life Boon"]);
    pick(&mut e, bleed(a));
    let mut boons = Vec::new();
    for _ in 0..60 {
        let req = request(&mut e);
        if req.window == WindowLabel::MinionPhase && req.seat == SeatId(0) {
            break;
        }
        if req.options.iter().any(|o| {
            matches!(
                o,
                Choice::Play {
                    target: Target::Seat(SeatId(1)),
                    ..
                }
            )
        }) && !boons.contains(&req.seat)
        {
            boons.push(req.seat);
        }
        e.choose(fallback(&req));
    }
    assert_eq!(
        boons,
        vec![SeatId(2), SeatId(3)],
        "clockwise from the active"
    );
}

/// Reversal of Fortunes outlives its caller's oust (PIB 20121029); a
/// second copy passed restores the order.
#[test]
fn reversal_of_fortunes_toggles_and_outlives_its_caller() {
    let mut e = game(3);
    let a = titled_caller(&mut e, Title::Prince("Paris"));
    for s in 1..3 {
        e.add_minion(SeatId(s), plain("Ventrue", s as usize), 4, false);
    }
    e.state.turn_reversed.0 = 1;
    hand(&mut e, 0, &["Reversal of Fortunes"]);
    pass_referendum(&mut e, a, &[]);
    request(&mut e);
    assert_eq!(e.state.turn_reversed.0, 0, "restored");
    e.state.turn_reversed.0 = 1;
    e.state.seats[0].pool = 1;
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert_eq!(e.state.turn_reversed.0, 1);
}

/// Seraph: another Methuselah's 7-capacity Black Hand vampire chosen;
/// passed, the card itself on it, +2 blood, a Seraph; a Camarilla prince's
/// votes not counted (non-Sabbat).
#[test]
fn seraph_makes_a_black_hand_elder_a_seraph() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| c.black_hand && c.sect == Some(Sect::Sabbat) && c.capacity >= 4,
            0,
        ),
        4,
        false,
    );
    e.state.minions[a.index()].granted_title = Some(Title::Bishop);
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=7, 1), 4, false);
    let t = e.add_minion(
        SeatId(1),
        textless(|c| c.black_hand && !c.seraph && c.capacity >= 7, 0),
        3,
        false,
    );
    let p = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    assert!(!e.state.is_seraph(t));
    hand(&mut e, 0, &["Seraph"]);
    pick(&mut e, play(0, 0, a));
    // The one vampire to choose, elided.
    to_referendum(&mut e);
    let r = e.referendum().expect("a referendum");
    assert_eq!(r.terms.minions, [t]);
    assert_eq!(r.votes_of(&e.state, p), 0, "non-Sabbat");
    pick_matching(&mut e, |o| {
        *o == votes(a, true) || *o == Choice::CastCardVote { favor: true }
    });
    close_polling(&mut e);
    request(&mut e);
    let c = placed_on(&e, "Seraph", Some(t));
    assert!(e.state.is_seraph(t));
    assert_eq!(blood(&e, t), 5);
    let _ = c;
}

/// Seraph: only a Seraph or 5-capacity Black Hand vampires to choose: not
/// played. [LSJ 20100129] [ANK 20191228]
#[test]
fn seraph_needs_a_non_seraph_black_hand_above_five() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.black_hand && c.capacity == 5, 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        textless(|c| c.black_hand && c.capacity == 5, 1),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| c.seraph && c.capacity >= 6, 0),
        4,
        false,
    );
    hand(&mut e, 0, &["Seraph"]);
    assert!(!request(&mut e).options.contains(&play(0, 0, a)));
}

/// Watchtower: Greatest Fall: an untitled Seraph caller's 2 votes pass it,
/// the card in play, its pool paid; locked, 1 blood to a ready Sabbat
/// vampire of its player's; Four Ride Forth in play burns as it enters.
#[test]
fn watchtower_greatest_fall_gives_seraphs_two_votes_and_feeds_the_sabbat() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.seraph && c.sect == Some(Sect::Sabbat), 0),
        4,
        false,
    );
    let s = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=7, 1), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Watchtower: Four Ride Forth", None);
    hand(&mut e, 0, &["Watchtower: Greatest Fall"]);
    pass_referendum(&mut e, a, &[]);
    request(&mut e);
    let w = placed_on(&e, "Watchtower: Greatest Fall", None);
    assert_eq!(e.state.seats[0].pool, 9, "1 pool");
    assert!(
        ash_has(&e, 1, "Watchtower: Four Ride Forth"),
        "another watchtower"
    );
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let lock = use_on(w, 0, None, Target::Minion(s));
    pick_when(&mut e, lock);
    request(&mut e);
    let _ = req;
    assert_eq!(blood(&e, s), 4);
}

/// Whether `o` uses vampire `m`'s printed ability `ability`.
fn uses_vampire(o: &Choice, m: MinionId, ability: u8) -> bool {
    matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), ability: i, .. }
        if *v == m && *i == ability)
}

/// The crypt card `name`, the advanced one or the other.
fn card_adv(name: &str, adv: bool) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && c.adv == adv)
        .unwrap_or_else(|| panic!("no {name}")) as u16
}

/// The text-only vampires carry their generated traits: Greger Anderssen
/// prince of Stockholm, Ian Wallingford bishop, Ezekiel and Henri Lavenant
/// Black Hand (kept through a sect change: LSJ 20070322), Gwendolyn and
/// Harrod Inner Circle, Hannibal cardinal. [LSJ 20070322] [ANK 20180807]
#[test]
fn the_round_34_vampires_ezekiel_to_ian_carry_their_printed_traits() {
    for name in [
        "Ezekiel, Lord of Montreal",
        "Greger Anderssen",
        "Heinrick Schlempt",
        "Ian Wallingford",
    ] {
        assert!(zvtes::cards::vampire(find_card(name)).is_none(), "{name}");
    }
    let c = |n: &str| &CRYPT[find_card(n) as usize];
    assert_eq!(c("Greger Anderssen").title, Title::Prince("Stockholm"));
    assert_eq!(c("Ian Wallingford").title, Title::Bishop);
    assert_eq!(c("Gwendolyn").title, Title::InnerCircle("Brujah"));
    assert_eq!(c("Harrod").title, Title::InnerCircle("Nosferatu"));
    assert_eq!(c("Hannibal").title, Title::Cardinal);
    assert_eq!(c("Heinrick Schlempt").sect, Some(Sect::Sabbat));
    let mut e = game(2);
    for n in ["Ezekiel, Lord of Montreal", "Henri Lavenant"] {
        let m = e.add_minion(SeatId(0), find_card(n), 4, false);
        assert!(e.state.is_black_hand(m), "{n}");
        e.state.minions[m.index()].sect_override = Some(Sect::Anarch);
        assert!(e.state.is_black_hand(m), "{n}: kept");
    }
}

/// Goratrix (advanced): burns another Methuselah's Tremere antitribu, a
/// torpid one offered too, a Tremere not; the action directed at them.
#[test]
fn goratrix_advanced_burns_a_tremere_antitribu() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), card_adv("Goratrix", true), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), of_clan("Tremere antitribu", 3..=8, 0), 3, false);
    let torpid = e.add_minion(SeatId(1), of_clan("Tremere antitribu", 3..=8, 1), 1, true);
    let tremere = e.add_minion(SeatId(1), of_clan("Tremere", 3..=8, 0), 3, false);
    let req = request(&mut e);
    let burn = |m: MinionId| Choice::UseAbility {
        source: AbilitySource::Vampire(g),
        ability: 0,
        actor: Some(g),
        target: Target::Minion(m),
    };
    assert!(req.options.contains(&burn(torpid)));
    assert!(!req.options.contains(&burn(tremere)));
    e.choose(index_of(&req, burn(t)));
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(1), "directed: seat 1 may block");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_play, "burned");
}

/// Goratrix merged: Camarilla (ANK 20180524); once in his bleed, 1 blood
/// for +1 bleed, not twice. [ANK 20180524]
#[test]
fn goratrix_merged_is_camarilla_and_bleeds_more() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), card_adv("Goratrix", true), 6, false);
    e.state.minions[g.index()].merged = Some(card_adv("Goratrix", false));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert_eq!(e.state.sect_of(g), Some(Sect::Camarilla));
    let bleed0 = e.state.current_bleed(g);
    pick(&mut e, bleed(g));
    let more = Choice::UseAbility {
        source: AbilitySource::Vampire(g),
        ability: 2,
        actor: None,
        target: Target::None,
    };
    pick_when(&mut e, more);
    let req = request(&mut e);
    assert!(!req.options.contains(&more), "once each action");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - i32::from(bleed0) - 1);
}

/// Greta Kircher: seat 1's younger vampire bleeds her controller: once
/// that action she burns 1 blood to reduce the bleed by 1; an older
/// bleeder: not offered.
#[test]
fn greta_kircher_reduces_a_bleed_by_a_younger_vampire() {
    for younger in [true, false] {
        let mut e = game(2);
        let g = e.add_minion(SeatId(0), find_card("Greta Kircher"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let cap = CRYPT[find_card("Greta Kircher") as usize].capacity;
        let b = e.add_minion(
            SeatId(1),
            textless(|c| c.capacity != cap && (c.capacity < cap) == younger, 0),
            4,
            false,
        );
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        to_minion_phase(&mut e, SeatId(1));
        pick(&mut e, bleed(b));
        let used = offered_before(
            &mut e,
            Choice::UseAbility {
                source: AbilitySource::Vampire(g),
                ability: 0,
                actor: None,
                target: Target::None,
            },
            SeatId(1),
            WindowLabel::MinionPhase,
        );
        assert_eq!(used, younger);
    }
}

/// Gwendolyn and Harrod bleed for 3; Harrod in his controller's unlock
/// phase looks at a card of the prey's uncontrolled region he picks, the
/// look noted to his player; with the region empty, not offered.
#[test]
fn harrod_looks_at_a_card_in_his_preys_uncontrolled_region() {
    for n in ["Gwendolyn", "Harrod"] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card(n), 4, false);
        assert_eq!(e.state.current_bleed(m), 3, "{n}");
    }
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Harrod"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    uncontrolled(&mut e, 1, plain("Toreador", 0), 0);
    uncontrolled(&mut e, 1, plain("Toreador", 1), 0);
    e.notes = Some(Vec::new());
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let look = *req
        .options
        .iter()
        .find(|o| uses_vampire(o, h, 0))
        .expect("offered");
    e.choose(index_of(&req, look));
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, [Choice::Slot(0), Choice::Slot(1)]);
    e.choose(1);
    request(&mut e);
    let looks = noted_looks(&mut e);
    let l = looks.last().expect("a look");
    assert_eq!((l.viewer, l.seat), (SeatId(0), SeatId(1)));
    assert_eq!(l.cards, vec![plain("Toreador", 1)]);
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Harrod"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(
        !req.options.iter().any(|o| uses_vampire(o, h, 0)),
        "an empty region"
    );
}

/// Hannibal: steals up to 2 blood from another Methuselah's Sabbat vampire
/// (3 → 1), a Camarilla one not offered; merged, both the steal and the
/// advanced card's unlock.
#[test]
fn hannibal_steals_up_to_two_blood_from_a_sabbat_vampire() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), card_adv("Hannibal", false), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let s = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=7, 0), 3, false);
    let c = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=7, 0), 3, false);
    let steal = |m: MinionId| Choice::UseAbility {
        source: AbilitySource::Vampire(h),
        ability: 0,
        actor: Some(h),
        target: Target::Minion(m),
    };
    let req = request(&mut e);
    assert!(!req.options.contains(&steal(c)));
    e.choose(index_of(&req, steal(s)));
    pick_when(&mut e, Choice::SetX(2));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, s), blood(&e, h)), (1, 4));
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), card_adv("Hannibal", true), 4, false);
    e.state.minions[h.index()].merged = Some(card_adv("Hannibal", false));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let s = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=7, 0), 3, false);
    let req = request(&mut e);
    assert!(req.options.contains(&steal(s)), "merged: the steal");
}

/// Henri Lavenant: +1 hand size to his controller while in combat.
#[test]
fn henri_lavenant_holds_one_more_card_in_combat() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Henri Lavenant"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let base = e.state.hand_size(SeatId(0));
    hand(&mut e, 1, &["Side Strike"]);
    e.state.minions[z.index()].gained_levels = vec!["cel"];
    pick(&mut e, bleed(h));
    pick_when(&mut e, block(z));
    request(&mut e);
    assert!(e.combat().is_some());
    assert_eq!(e.state.hand_size(SeatId(0)), base + 1);
    assert_eq!(e.state.hand_size(SeatId(1)), base);
}

/// Henry Taylor: after the action whose combat he played a combat card in,
/// the card from the ash heap to the top of his library.
#[test]
fn henry_taylor_returns_a_combat_card_to_the_library_top() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Henry Taylor"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Immortal Grapple"]);
    // A full hand and a library: nothing drawn back.
    for _ in 0..6 {
        hand(&mut e, 0, &["Minion Tap"]);
        e.state.seats[0].library.push(find_library("Minion Tap"));
    }
    pick(&mut e, bleed(h));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, h));
    let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    let grapple = find_library("Immortal Grapple");
    let back = *req
        .options
        .iter()
        .find(|o| uses_vampire(o, h, 0))
        .expect("offered");
    e.choose(index_of(&req, back));
    let req = request(&mut e);
    if req.options.contains(&Choice::FindCard(grapple)) {
        e.choose(index_of(&req, Choice::FindCard(grapple)));
    }
    assert_eq!(e.state.seats[0].library.last(), Some(&grapple), "the top");
}

/// Ian Forestal: plays a card requiring basic Celerity he lacks (Side
/// Strike), not its superior mode, and does not have Celerity (LSJ
/// 19980303); a master: Discipline card on him (Celerity) gives the
/// superior level. [LSJ 19980303]
#[test]
fn ian_forestal_plays_a_missing_discipline_at_basic() {
    let mut e = game(2);
    let i = e.add_minion(SeatId(0), find_card("Ian Forestal"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Side Strike"]);
    pick(&mut e, bleed(i));
    pick_when(&mut e, block(z));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(req.options.contains(&play(0, 0, i)), "basic");
    assert!(!req.options.contains(&play(0, 1, i)), "not superior");
    assert!(!e.state.has_discipline(i, "cel"));
    let mut e = game(2);
    let i = e.add_minion(SeatId(0), find_card("Ian Forestal"), 6, false);
    let no_cel = |c: &zvtes::crypt::CryptCard| {
        c.capacity >= 4 && !c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("cel"))
    };
    let p = e.add_minion(SeatId(0), fixture_vampire(no_cel, 0), 4, false);
    in_play(&mut e, 0, "Celerity", Some(i));
    in_play(&mut e, 0, "Celerity", Some(p));
    assert_eq!(e.state.discipline_level(i, "cel"), 2);
    assert_eq!(e.state.discipline_level(p, "cel"), 1);
}

/// Ian Forestal: "Each Master: Discipline card placed on him gives him the
/// superior level": Celerity (a master: Discipline card) superior on him;
/// Blooding by the Code ("Master." only, a Valeren level) the basic level.
#[test]
fn ian_forestal_reads_only_master_discipline_cards() {
    let mut e = game(2);
    let i = e.add_minion(SeatId(0), find_card("Ian Forestal"), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Celerity", Some(i));
    in_play(&mut e, 0, "Blooding by the Code", Some(i));
    assert_eq!(e.state.discipline_level(i, "cel"), 2);
    assert_eq!(e.state.discipline_level(i, "val"), 1);
}

/// Ignazio Giovanni: enters combat with his prey's capacity-6 vampire, not
/// a capacity-7 one; +1 strength.
#[test]
fn ignazio_giovanni_rushes_a_small_vampire_of_his_prey() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Ignazio Giovanni"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let six = e.add_minion(SeatId(1), textless(|c| c.capacity == 6, 0), 4, false);
    let seven = e.add_minion(SeatId(1), textless(|c| c.capacity == 7, 0), 4, false);
    let rush = |m: MinionId| Choice::UseAbility {
        source: AbilitySource::Vampire(g),
        ability: 0,
        actor: Some(g),
        target: Target::Minion(m),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&rush(six)));
    assert!(!req.options.contains(&rush(seven)));
    assert_eq!(
        e.state
            .printed_mod(g, Stat::Strength, PrintedCtx::default()),
        1
    );
}

/// Ian Forestal: Show of Force ([pot][pre], both missing) not by his grant
/// alone, nor with an Infernal Familiar's beside it; with [pre] his own, his
/// grant meets [pot], chosen over the Familiar's, no counter added.
/// [LSJ 20011217] [LSJ 20090508] [LSJ 20020218] [RTR 20020501] [LSJ
/// 20100517]
#[test]
fn ian_forestal_meets_one_requirement_and_spares_a_familiar() {
    let setup = |pre: bool, familiar: bool| {
        let mut e = game(2);
        let i = e.add_minion(SeatId(0), find_card("Ian Forestal"), 6, false);
        e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if pre {
            e.state.minions[i.index()].gained_levels.push("pre");
        }
        let f = familiar.then(|| in_play(&mut e, 0, "Infernal Familiar", Some(i)));
        hand(&mut e, 0, &["Show of Force"]);
        (e, i, f)
    };
    for familiar in [false, true] {
        let (mut e, i, _) = setup(false, familiar);
        let req = request(&mut e);
        assert!(
            !req.options.contains(&play(0, 0, i)),
            "two missing, {familiar}"
        );
    }
    let (mut e, i, f) = setup(true, true);
    pick(&mut e, play(0, 0, i));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    let f = f.expect("familiar");
    assert_eq!(
        req.options,
        [Choice::ChooseCard(f.0), Choice::ChooseMinion(i)]
    );
    e.choose(index_of(&req, Choice::ChooseMinion(i)));
    request(&mut e);
    assert_eq!(e.state.cards[f.index()].counters, 0, "no counter");
}

/// Ian Forestal blocking plays Pack Alpha [ani] and employs its Raven Spy
/// [ani] (LSJ 20080205-2). [LSJ 20080205-2] [LSJ 20060417]
#[test]
fn ian_forestal_meets_the_requirement_of_a_card_his_card_brings_in() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let i = e.add_minion(SeatId(1), find_card("Ian Forestal"), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Pack Alpha", "Raven Spy"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(i));
    pick_when(&mut e, play(0, 0, i));
    request(&mut e);
    assert!(e.state.seats[1].hand.is_empty(), "the spy employed");
}

/// Henry Taylor: Rapid Change played as an action modifier (+1 stealth, the
/// block failing) is no combat card played: not eligible (LSJ 20031221).
/// [LSJ 20031221]
#[test]
fn henry_taylor_ignores_a_card_played_as_an_action_modifier() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Henry Taylor"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Rapid Change"]);
    pick(&mut e, bleed(h));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, h));
    for _ in 0..100 {
        let req = request(&mut e);
        assert!(!req.options.iter().any(|o| uses_vampire(o, h, 0)));
        if req.seat == SeatId(0) && req.window == WindowLabel::MinionPhase {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(ash_has(&e, 0, "Rapid Change"));
    assert_eq!(e.state.seats[1].pool, 10 - 1, "unblocked");
}
