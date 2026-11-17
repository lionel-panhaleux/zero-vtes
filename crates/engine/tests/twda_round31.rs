//! Round 31 (`ADDED_BATCHES`): Third Edition completed. Windows with a
//! single option are elided, so the scripts only stop where a real decision
//! exists.

use zvtes::cards::{AbilitySource, Stat, Target};
use zvtes::combat::Range;
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::{CardType, LIBRARY};
use zvtes::state::{find_library, CardId, CardInPlay};

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

/// The crypt card `name`, advanced or not.
#[track_caller]
fn crypt_card(name: &str, adv: bool) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && c.adv == adv)
        .unwrap_or_else(|| panic!("{name}")) as u16
}

/// A card already in play for `seat`, on `host` if given, in mode `mode`
/// with that mode's layers (no play): its id.
fn in_play_mode(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>, mode: u8) -> CardId {
    let card = find_library(name);
    e.state
        .cards
        .push(CardInPlay::new(card, SeatId(seat), host, mode));
    let id = CardId(e.state.cards.len() as u32 - 1);
    for effect in zvtes::cards::lookup(card).expect("definition").modes[mode as usize].effects {
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

/// [`in_play_mode`] in its first mode.
fn in_play(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>) -> CardId {
    in_play_mode(e, seat, name, host, 0)
}

/// The `nth` textless Sabbat vampire.
fn sabbat(nth: usize) -> u16 {
    textless(|c| c.sect == Some(Sect::Sabbat) && c.capacity <= 7, nth)
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

/// A seat-0 Sabbat vampire with `title`, beside another seat-0 vampire and
/// a seat-1 one (the minion phase not elided).
fn titled_caller(e: &mut Engine, title: Title) -> MinionId {
    let a = e.add_minion(SeatId(0), sabbat(0), 4, false);
    e.state.minions[a.index()].granted_title = Some(title);
    e.add_minion(SeatId(0), sabbat(1), 4, false);
    a
}

// ---- Political actions ---------------------------------------------------------

/// Auto-da-fé: "Requires a cardinal, regent or Inquisitor." A cardinal and
/// a vampire bearing Sabbat Inquisitor call it; an untitled Sabbat vampire
/// does not, Mercy, Knight Inquisitor neither (her name is no Inquisitor).
#[test]
fn auto_da_fe_needs_a_cardinal_regent_or_inquisitor() {
    let mut e = game(2);
    let c = titled_caller(&mut e, Title::Cardinal);
    let u = e.add_minion(SeatId(0), sabbat(2), 4, false);
    let q = e.add_minion(SeatId(0), sabbat(3), 4, false);
    in_play(&mut e, 0, "Sabbat Inquisitor", Some(q));
    let mercy = e.add_minion(SeatId(0), find_card("Mercy, Knight Inquisitor"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Auto-da-fé"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, c)));
    assert!(req.options.contains(&play(0, 0, q)));
    assert!(!req.options.contains(&play(0, 0, u)));
    assert!(!req.options.contains(&play(0, 0, mercy)));
}

/// Auto-da-fé passed on a capacity-5 vampire: its controller burns 3 pool
/// (8 - 5) and keeps it, or lets it burn; with less than 3 pool only the
/// burn is left (rules-digest D3).
#[test]
fn auto_da_fe_burns_pool_or_the_chosen_vampire() {
    let five = textless(|c| c.capacity == 5 && c.sect != Some(Sect::Sabbat), 0);
    for (pool, pay) in [(10, true), (10, false), (2, false)] {
        let mut e = game(2);
        let c = titled_caller(&mut e, Title::Cardinal);
        let v = e.add_minion(SeatId(1), five, 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.seats[1].pool = pool;
        hand(&mut e, 0, &["Auto-da-fé"]);
        pass_referendum(&mut e, c, &[Choice::ChooseMinion(v)]);
        if pool >= 3 {
            let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
            assert_eq!(req.options, vec![Choice::BurnPool, Choice::Pass]);
            pick(&mut e, if pay { Choice::BurnPool } else { Choice::Pass });
        }
        request(&mut e);
        assert_eq!(e.state.minions[v.index()].in_play, pay, "{pool} {pay}");
        assert_eq!(e.state.seats[1].pool, if pay { pool - 3 } else { pool });
        assert_eq!(
            e.state.seats[1].ash_crypt.contains(&five),
            !pay,
            "the vampire burned to its ash heap"
        );
    }
}

/// [LSJ 20100129] [ANK 20191228] Auto-da-fé, Excommunication, Sabbat
/// Inquisitor with no legal choice: not called. Every vampire of capacity
/// 8 or more; only a torpid archbishop; every Sabbat vampire an
/// Inquisitor already.
#[test]
fn political_actions_with_no_legal_choice_are_not_called() {
    // Auto-da-fé: capacity 8 and more only.
    let big = |n| {
        CRYPT
            .iter()
            .enumerate()
            .filter(|(_, c)| c.capacity >= 8 && c.title == Title::None && !c.imbued && !c.adv)
            .nth(n)
            .map(|(i, _)| i as u16)
            .expect("an untitled elder")
    };
    let mut e = game(2);
    let cardinal = CRYPT
        .iter()
        .position(|c| c.title == Title::Cardinal && c.capacity >= 8 && !c.adv)
        .expect("a printed cardinal of capacity 8 or more") as u16;
    let c = e.add_minion(SeatId(0), cardinal, 4, false);
    e.add_minion(SeatId(0), big(0), 4, false);
    e.add_minion(SeatId(1), big(1), 4, false);
    hand(&mut e, 0, &["Auto-da-fé"]);
    assert!(!request(&mut e).options.contains(&play(0, 0, c)));
    // Excommunication: the archbishop in torpor.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), sabbat(0), 4, false);
    e.add_minion(SeatId(0), sabbat(1), 4, false);
    let t = e.add_minion(SeatId(1), sabbat(2), 1, true);
    e.state.minions[t.index()].granted_title = Some(Title::Archbishop("Montreal"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Excommunication"]);
    assert!(!request(&mut e).options.contains(&play(0, 0, a)));
    // Sabbat Inquisitor: each Sabbat vampire bears one.
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Archbishop("Montreal"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for m in [a, MinionId(1)] {
        in_play(&mut e, 0, "Sabbat Inquisitor", Some(m));
    }
    hand(&mut e, 0, &["Sabbat Inquisitor"]);
    assert!(!request(&mut e).options.contains(&play(0, 0, a)));
}

/// Crusade: Detroit, New York, Philadelphia, Pittsburgh, Toronto: the card
/// itself on its Sabbat caller as that city's archbishop (rules-qa 85.14);
/// New York's beside Francisco Domingo de Polonia, its printed holder, the
/// two contest it (3.1).
#[test]
fn crusades_of_third_edition_title_their_sabbat_caller() {
    for (card, city) in [
        ("Crusade: Detroit", "Detroit"),
        ("Crusade: New York", "New York"),
        ("Crusade: Philadelphia", "Philadelphia"),
        ("Crusade: Pittsburgh", "Pittsburgh"),
        ("Crusade: Toronto", "Toronto"),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), sabbat(0), 4, false);
        e.add_minion(SeatId(0), sabbat(1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &[card]);
        pass_referendum(&mut e, a, &[]);
        request(&mut e);
        assert_eq!(e.state.title_of(a), Title::Archbishop(city), "{card}");
        placed_on(&e, card, Some(a));
    }
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), sabbat(0), 4, false);
    e.add_minion(SeatId(0), sabbat(1), 4, false);
    let f = e.add_minion(
        SeatId(1),
        find_card("Francisco Domingo de Polonia"),
        4,
        false,
    );
    hand(&mut e, 0, &["Crusade: New York"]);
    pass_referendum(&mut e, a, &[]);
    request(&mut e);
    assert!(e.state.minions[a.index()].title_contested);
    assert!(e.state.minions[f.index()].title_contested);
}

/// Crusade: Rome: "If this vampire is Brujah antitribu, they can unlock
/// during your next discard phase": any time in it, once (rules-qa 69.9,
/// 26.5; D7) — offered still after a discard, unlocked or left locked; a
/// Lasombra is offered nothing.
#[test]
fn crusade_rome_lets_a_brujah_antitribu_unlock_at_the_discard_phase() {
    for (clan, unlock) in [
        ("Brujah antitribu", true),
        ("Brujah antitribu", false),
        ("Lasombra", false),
    ] {
        let mut e = game(2);
        let a = e.add_minion(
            SeatId(0),
            textless(|c| c.sect == Some(Sect::Sabbat) && c.clan == clan, 0),
            4,
            false,
        );
        e.add_minion(SeatId(0), sabbat(1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Crusade: Rome"]);
        pass_referendum(&mut e, a, &[]);
        assert_eq!(e.state.title_of(a), Title::Archbishop("Rome"));
        // Cards in hand keep the discard phase from eliding.
        hand(&mut e, 0, &["Dodge", "Dodge"]);
        let asked = Choice::UnlockForBlood { minion: a };
        if clan == "Lasombra" {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, asked);
            assert!(e.state.minions[a.index()].locked);
            continue;
        }
        let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
        assert!(req.options.contains(&asked), "{req:?}");
        e.choose(index_of(&req, Choice::Discard(0)));
        let req = expect(&mut e, SeatId(0), WindowLabel::DiscardPhase);
        assert!(req.options.contains(&asked), "still, after a discard");
        if unlock {
            e.choose(index_of(&req, asked));
            assert!(!e.state.minions[a.index()].locked);
            let req = request(&mut e);
            assert!(!req.options.contains(&asked), "once");
        } else {
            e.choose(index_of(&req, Choice::Pass));
            request(&mut e);
            assert!(e.state.minions[a.index()].locked);
        }
    }
}

/// Deploy the Hand: seat 1 chosen gets a target counter (the card's
/// effect, seat 1's, against seat 1; the card in seat 0's ash heap); at each
/// of seat 1's unlock phases seat 1 picks a ready minion of theirs, a
/// torpid one not, which takes 2 unpreventable damage.
#[test]
fn deploy_the_hand_damages_a_ready_minion_each_unlock_phase() {
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Archbishop("Montreal"));
    let r = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let s = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 2), 1, true);
    hand(&mut e, 0, &["Deploy the Hand"]);
    pass_referendum(&mut e, a, &[Choice::ChooseSeat(SeatId(1)), Choice::Pass]);
    let counters = stand_ins(&e, "Deploy the Hand");
    assert_eq!(counters.len(), 1);
    let k = &e.state.cards[counters[0].index()];
    assert_eq!((k.controller, k.host), (SeatId(1), None));
    assert!(k.against.contains(SeatId(1)) && !k.against.contains(SeatId(0)));
    assert!(ash_has(&e, 0, "Deploy the Hand"));
    for _ in 0..2 {
        let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
        assert!(req.options.contains(&Choice::ChooseMinion(r)));
        assert!(req.options.contains(&Choice::ChooseMinion(s)));
        assert!(!req.options.contains(&Choice::ChooseMinion(t)));
        let before = blood(&e, r);
        pick(&mut e, Choice::ChooseMinion(r));
        request(&mut e);
        assert_eq!(blood(&e, r), before - 2);
        e.state.minions[r.index()].blood = 4;
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    }
}

/// Deploy the Hand's counter: its Methuselah burns the Edge to burn it,
/// nobody else; then no damage at their unlock phase. [LSJ 20100129]
/// [ANK 20191228] Every Methuselah with a counter: not called.
#[test]
fn deploy_the_hand_counter_burns_with_the_edge() {
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Archbishop("Montreal"));
    let r = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let s = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Deploy the Hand", "Deploy the Hand"]);
    pass_referendum(
        &mut e,
        a,
        &[Choice::ChooseSeat(SeatId(0)), Choice::ChooseSeat(SeatId(1))],
    );
    // Both chosen: the second copy is not called.
    let b = MinionId(1);
    assert!(!request(&mut e).options.contains(&play(0, 0, b)));
    let counters = stand_ins(&e, "Deploy the Hand");
    let theirs = *counters
        .iter()
        .find(|&&k| e.state.cards[k.index()].against.contains(SeatId(1)))
        .expect("seat 1's counter");
    // Seat 0 holds the Edge: its own counter's burn only.
    e.state.seats[0].has_edge = true;
    let burn = ability(theirs.0, 0);
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, burn);
    e.state.seats[0].has_edge = false;
    e.state.seats[1].has_edge = true;
    let req = offering(&mut e, burn);
    assert_eq!(req.seat, SeatId(1));
    pick(&mut e, burn);
    request(&mut e);
    assert!(!e.state.seats[1].has_edge);
    assert!(!e.state.cards[theirs.index()].in_play);
    let before = (blood(&e, r), blood(&e, s));
    for seat in [0, 1] {
        never_offered_where(&mut e, SeatId(seat), WindowLabel::MinionPhase, |o| {
            *o == Choice::ChooseMinion(r) || *o == Choice::ChooseMinion(s)
        });
    }
    assert_eq!((blood(&e, r), blood(&e, s)), before, "no damage since");
}

/// Dogs of War in play: any Brujah antitribu may enter combat with another
/// Methuselah's minion as a Ⓓ action, a Toreador antitribu not. [LSJ
/// 20100909] The rush and the burn referendum are not cross-restrictive:
/// after the rush, unlocked, the same vampire may call the burn, no longer
/// rush.
#[test]
fn dogs_of_war_rush_and_burn_are_not_cross_restrictive() {
    let mut e = game(2);
    let b = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Brujah antitribu", 0),
        5,
        false,
    );
    let t = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Toreador antitribu", 0),
        4,
        false,
    );
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let d = in_play(&mut e, 0, "Dogs of War", None);
    let rush = |m| use_on(d, 0, Some(m), Target::Minion(v));
    let req = request(&mut e);
    assert!(req.options.contains(&rush(b)));
    assert!(!req.options.contains(&rush(t)));
    pick(&mut e, rush(b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[b.index()].locked = false;
    let req = request(&mut e);
    assert!(!req.options.contains(&rush(b)), "once a turn");
    assert!(req.options.contains(&use_on(d, 2, Some(b), Target::None)));
}

/// Seat 0's `d` (of `clan`) diablerizes seat 1's torpid vampire, seat 1's
/// prince passes the blood hunt; seat 0 controls Dogs of War.
fn blood_hunt_on(clan: &str) -> (Engine, MinionId, CardId) {
    let mut e = game(2);
    let d = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == clan && c.sect == Some(Sect::Sabbat), 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), sabbat(1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, true);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    let dogs = in_play(&mut e, 0, "Dogs of War", None);
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
    (e, d, dogs)
}

/// Dogs of War: "You can burn this card to cancel a blood hunt called on a
/// Brujah antitribu": once the blood hunt passed, before the burn (LSJ
/// 20081203-2), by its controller, the diablerist's own Methuselah; the
/// card to the ash heap, the vampire not burned. A Lasombra diablerist:
/// not offered, burned.
#[test]
fn dogs_of_war_saves_a_brujah_antitribu_from_a_blood_hunt() {
    let (mut e, d, dogs) = blood_hunt_on("Brujah antitribu");
    let cancel = use_on(dogs, 1, None, Target::Minion(d));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Pass, cancel]);
    pick(&mut e, cancel);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[d.index()].in_play, "not burned");
    assert!(ash_has(&e, 0, "Dogs of War"));
    let (mut e, d, dogs) = blood_hunt_on("Lasombra");
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_on(dogs, 1, None, Target::Minion(d)),
    );
    assert!(!e.state.minions[d.index()].in_play, "burned");
}

/// Dogs of War's cancel of a Veles' Hunt on a Brujah antitribu: its
/// controller's, the hunted vampire's own Methuselah; canceled, "no more
/// Veles' Hunts can be played this game" (rules-qa 116.87).
#[test]
fn dogs_of_war_cancels_a_veles_hunt_barring_it() {
    let mut e = game(2);
    let gangrel = textless(|c| c.clan == "Gangrel" && c.capacity >= 5, 0);
    let cap = CRYPT[gangrel as usize].capacity;
    let g = e.add_minion(SeatId(0), gangrel, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let y = e.add_minion(
        SeatId(1),
        textless(|c| c.clan == "Brujah antitribu" && c.capacity < cap, 0),
        3,
        false,
    );
    e.add_minion(SeatId(1), textless(|c| c.capacity < cap, 0), 3, false);
    let dogs = in_play(&mut e, 1, "Dogs of War", None);
    hand(&mut e, 0, &["Veles' Hunt"]);
    pass_referendum(&mut e, g, &[Choice::ChooseMinion(y)]);
    let cancel = use_on(dogs, 1, None, Target::Minion(y));
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Pass, cancel]);
    pick(&mut e, cancel);
    request(&mut e);
    assert!(e.state.minions[y.index()].in_play, "not burned");
    assert!(ash_has(&e, 1, "Dogs of War"));
    assert!(e.state.once_played.contains(&find_library("Veles' Hunt")));
}

/// Excommunication: a printed archbishop loses the title for good; a
/// Crusade's archbishop loses it with the card burned (rules-qa 19.6).
#[test]
fn excommunication_strips_an_archbishops_title() {
    for crusade in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), sabbat(0), 4, false);
        e.add_minion(SeatId(0), sabbat(1), 4, false);
        let x = if crusade {
            let x = e.add_minion(SeatId(1), sabbat(2), 4, false);
            in_play(&mut e, 1, "Crusade: Chicago", Some(x));
            x
        } else {
            e.add_minion(
                SeatId(1),
                find_card("Francisco Domingo de Polonia"),
                4,
                false,
            )
        };
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        assert!(matches!(e.state.title_of(x), Title::Archbishop(_)));
        hand(&mut e, 0, &["Excommunication"]);
        pass_referendum(&mut e, a, &[Choice::ChooseMinion(x)]);
        request(&mut e);
        assert_eq!(e.state.title_of(x), Title::None, "crusade {crusade}");
        assert_eq!(ash_has(&e, 1, "Crusade: Chicago"), crusade);
    }
}

/// Hand Intervention: one Methuselah chosen, -1 hand size while the card is
/// in play (the card itself, against them); they alone may burn it by
/// burning the Edge, the size back. [LSJ 20100129] [ANK 20191228] A
/// Methuselah chosen by one in play is not choosable: none left, not
/// called.
#[test]
fn hand_intervention_lowers_the_chosen_hand_size_until_the_edge_burns_it() {
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Bishop);
    e.state.minions[1].granted_title = Some(Title::Bishop);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let size = e.state.hand_size(SeatId(1));
    hand(&mut e, 0, &["Hand Intervention", "Hand Intervention"]);
    pass_referendum(&mut e, a, &[Choice::ChooseSeat(SeatId(1))]);
    let hi = placed_on(&e, "Hand Intervention", None);
    assert_eq!(
        e.state.cards[hi.index()].against,
        zvtes::ids::SeatSet::single(SeatId(1))
    );
    assert_eq!(e.state.hand_size(SeatId(1)), size - 1);
    assert_eq!(e.state.hand_size(SeatId(0)), size);
    // The second copy: only seat 0 left to choose, the term elided.
    let b = MinionId(1);
    pick(&mut e, play(0, 0, b));
    never_offered_where(&mut e, SeatId(0), WindowLabel::ReferendumPolling, |o| {
        *o == Choice::ChooseSeat(SeatId(1))
    });
    pick_matching(&mut e, |o| *o == Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    assert_eq!(e.state.hand_size(SeatId(0)), size - 1);
    assert_eq!(e.state.hand_size(SeatId(1)), size - 1);
    // Seat 0, not chosen by the first, holds the Edge: not offered its burn.
    e.state.seats[0].has_edge = true;
    let burn = ability(hi.0, 0);
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, burn);
    e.state.seats[0].has_edge = false;
    e.state.seats[1].has_edge = true;
    let req = offering(&mut e, burn);
    assert_eq!(req.seat, SeatId(1));
    pick(&mut e, burn);
    request(&mut e);
    assert!(!e.state.seats[1].has_edge);
    assert!(ash_has(&e, 0, "Hand Intervention"));
    assert!(!e.state.cards[hi.index()].in_play);
    assert_eq!(e.state.hand_size(SeatId(1)), size);
}

/// Perpetual Care: each Methuselah burns 2 pool per vampire in torpor they
/// control, the caller too, a ready one not.
#[test]
fn perpetual_care_burns_two_pool_per_torpid_vampire() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 1, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 1, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 1, true);
    e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    hand(&mut e, 0, &["Perpetual Care"]);
    pass_referendum(&mut e, a, &[]);
    assert_eq!(pool(&e), vec![8, 6, 10]);
}

/// Rabble Razing: every vampire of capacity below 4, any Methuselah's, in
/// torpor too (rules-qa 59.9), burns 1 blood; capacity 4 untouched, an
/// empty one burns nothing.
#[test]
fn rabble_razing_bleeds_the_small_vampires() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 6, 0), 4, false);
    let three = e.add_minion(SeatId(0), textless(|c| c.capacity == 3, 0), 3, false);
    let two = e.add_minion(SeatId(1), textless(|c| c.capacity == 2, 0), 1, true);
    let four = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 4, false);
    let one = e.add_minion(SeatId(1), textless(|c| c.capacity == 1, 0), 0, false);
    hand(&mut e, 0, &["Rabble Razing"]);
    pass_referendum(&mut e, a, &[]);
    assert_eq!(
        [three, two, four, one, a].map(|m| blood(&e, m)),
        [2, 0, 4, 0, 4]
    );
}

/// Sabbat Inquisitor: the card itself on the chosen Sabbat vampire, +1
/// bleed; its Ⓓ action at seat 1's crypt burns a top vampire with Blood
/// Sorcery, leaves another; seen by its Methuselah alone. [LSJ 20100909]
/// Not cross-restrictive: after it, unlocked, the burn referendum is still
/// offered, the crypt action not.
#[test]
fn sabbat_inquisitor_burns_a_blood_sorcerer_off_the_crypt_top() {
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Archbishop("Montreal"));
    let q = MinionId(1);
    e.add_minion(SeatId(0), sabbat(2), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let bleed0 = e.state.current_bleed(q);
    hand(&mut e, 0, &["Sabbat Inquisitor"]);
    pass_referendum(&mut e, a, &[Choice::ChooseMinion(q)]);
    let si = placed_on(&e, "Sabbat Inquisitor", Some(q));
    assert_eq!(e.state.current_bleed(q), bleed0 + 1);
    let tha = textless(
        |c| c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("tha")),
        0,
    );
    let other = textless(
        |c| !c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("tha")),
        0,
    );
    for (top, burned) in [(other, false), (tha, true)] {
        e.state.seats[1].crypt.push(top);
        let n = e.state.seats[1].crypt.len();
        e.notes = Some(Vec::new());
        pick_when(&mut e, use_on(si, 0, Some(q), Target::Seat(SeatId(1))));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let looks = noted_looks(&mut e);
        assert!(looks.iter().any(|l| l.viewer == SeatId(0)
            && l.zone == zvtes::game::LookedZone::Crypt
            && l.cards == [top]));
        assert_eq!(e.state.seats[1].crypt.len(), if burned { n - 1 } else { n });
        assert_eq!(e.state.seats[1].ash_crypt.contains(&top), burned);
        e.state.minions[q.index()].locked = false;
        let req = request(&mut e);
        assert!(req.options.contains(&use_on(si, 1, Some(q), Target::None)));
        assert!(!req
            .options
            .contains(&use_on(si, 0, Some(q), Target::Seat(SeatId(1)))));
        if !burned {
            // Next turn, the same vampire again.
            until(&mut e, SeatId(1), WindowLabel::MinionPhase);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        }
    }
}

/// Sabbat Priest: the card to the ash heap, its effect on the chosen ready
/// Sabbat vampire for the game; a vampire attempting to block it burns 1
/// blood. [LSJ 19970707] One with no blood still blocks, burning what it
/// can.
#[test]
fn sabbat_priest_tolls_blockers_for_the_game() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), sabbat(0), 4, false);
    let t = e.add_minion(SeatId(0), sabbat(1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Sabbat Priest"]);
    pass_referendum(&mut e, a, &[Choice::ChooseMinion(t)]);
    let on: Vec<_> = stand_ins(&e, "Sabbat Priest")
        .into_iter()
        .filter(|k| e.state.cards[k.index()].host == Some(t))
        .collect();
    assert_eq!(on.len(), 1);
    assert!(ash_has(&e, 0, "Sabbat Priest"));
    assert_eq!(e.state.blockers_of_burn(t), 1);
    pick_when(&mut e, bleed(t));
    e.notes = Some(Vec::new());
    pick_when(&mut e, block(b));
    request(&mut e);
    assert!(!noted_combats(&mut e).is_empty(), "blocked");
}

/// Seat 0's `t` bleeds after `fixture` Sabbat Priests were put on it and
/// one more chosen it by referendum (`called`); seat 1's 4-blood vampire
/// blocks: its blood after the combat.
fn blocker_blood_after(fixture: usize, called: bool) -> u8 {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), sabbat(0), 4, false);
    let t = e.add_minion(SeatId(0), sabbat(1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    for _ in 0..fixture {
        let k = in_play(&mut e, 0, "Sabbat Priest", Some(t));
        e.state.cards[k.index()].effect_only = true;
    }
    if called {
        hand(&mut e, 0, &["Sabbat Priest"]);
        pass_referendum(&mut e, a, &[Choice::ChooseMinion(t)]);
        assert_eq!(stand_ins(&e, "Sabbat Priest").len(), fixture + 1);
    }
    pick_when(&mut e, bleed(t));
    pick_when(&mut e, block(b));
    request(&mut e);
    blood(&e, b)
}

/// [LSJ 19980225] [LSJ 19970707] Sabbat Priest chosen again on a vampire
/// with one already: a second copy; the burns add up (each blocker's
/// blood against the same combat without the card).
#[test]
fn sabbat_priest_copies_add_up() {
    let base = blocker_blood_after(0, false);
    assert_eq!(blocker_blood_after(0, true), base - 1);
    assert_eq!(blocker_blood_after(1, true), base - 2);
}

/// Templar: the card itself on the chosen Sabbat vampire; its +1 stealth
/// rush at another Methuselah's ready vampire, not an ally nor one in
/// torpor. [LSJ 20100909] After it, unlocked, the burn referendum still
/// offered; a Sabbat vampire passing it burns the card.
#[test]
fn templar_grants_a_rush_and_is_burned_by_a_sabbat_referendum() {
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Bishop);
    let t = MinionId(1);
    e.state.minions[t.index()].granted_title = Some(Title::Bishop);
    // A third keeps the minion phase from eliding.
    e.add_minion(SeatId(0), sabbat(2), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 1, true);
    hand(&mut e, 0, &["Templar"]);
    pass_referendum(&mut e, a, &[Choice::ChooseMinion(t)]);
    let tp = placed_on(&e, "Templar", Some(t));
    let rush = |at| use_on(tp, 0, Some(t), Target::Minion(at));
    let req = request(&mut e);
    assert!(req.options.contains(&rush(v)));
    assert!(!req.options.contains(&rush(w)));
    assert!(!req.options.contains(&rush(a)), "another Methuselah's");
    pick(&mut e, rush(v));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[t.index()].locked = false;
    let burn = use_on(tp, 1, Some(t), Target::None);
    let req = request(&mut e);
    assert!(!req.options.contains(&rush(v)));
    assert!(req.options.contains(&burn));
    pick(&mut e, burn);
    pick_matching(&mut e, |o| *o == votes(t, true));
    close_polling(&mut e);
    request(&mut e);
    assert!(!e.state.cards[tp.index()].in_play);
    assert!(ash_has(&e, 0, "Templar"));
}

/// [RTR 20030519] Two Templars on one vampire: each its own rush, the
/// second still offered after the first, unlocked.
#[test]
fn templar_copies_each_grant_their_rush() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), sabbat(0), 6, false);
    e.add_minion(SeatId(0), sabbat(1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let one = in_play(&mut e, 0, "Templar", Some(t));
    let two = in_play(&mut e, 0, "Templar", Some(t));
    pick(&mut e, use_on(one, 0, Some(t), Target::Minion(v)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[t.index()].locked = false;
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&use_on(one, 0, Some(t), Target::Minion(v))));
    assert!(req
        .options
        .contains(&use_on(two, 0, Some(t), Target::Minion(v))));
}

/// Tithings: "steal 1 pool from each Methuselah who does not control a
/// vampire with a capacity above 6": a torpid elder counts (rules-digest
/// D28); not called by a capacity-6 Sabbat vampire.
#[test]
fn tithings_taxes_the_methuselahs_without_an_elder() {
    let mut e = game(3);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Sabbat) && c.capacity >= 7, 0),
        4,
        false,
    );
    let six = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Sabbat) && c.capacity == 6, 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 7, 1), 1, true);
    e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 4, false);
    e.add_minion(SeatId(2), textless(|c| c.capacity == 4, 1), 4, false);
    hand(&mut e, 0, &["Tithings"]);
    assert!(!request(&mut e).options.contains(&play(0, 0, six)));
    pass_referendum(&mut e, a, &[]);
    assert_eq!(pool(&e), vec![11, 10, 9]);
}

/// Transfer of Power: every Methuselah richer than the caller as it
/// passes loses 1, all at once (spec §6.5.4): two at 21 against 20 both
/// pay, an equal one not.
#[test]
fn transfer_of_power_steals_from_the_richer_all_at_once() {
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    for s in 1..4 {
        e.add_minion(SeatId(s), plain("Ventrue", s as usize), 4, false);
    }
    for (s, p) in [20, 21, 21, 20].into_iter().enumerate() {
        e.state.seats[s].pool = p;
    }
    hand(&mut e, 0, &["Transfer of Power"]);
    pass_referendum(&mut e, a, &[]);
    assert_eq!(pool(&e), vec![22, 20, 20, 20]);
}

// ---- Master cards (master_a) ------------------------------------------------

/// A master card from hand slot `card`, on `target`.
fn master_on(card: u8, target: Target) -> Choice {
    Choice::Play {
        card,
        mode: 0,
        actor: None,
        target,
    }
}

/// Amusement Park Hunting Ground: "During your unlock phase, a ready
/// vampire you control can gain 1 blood. A vampire can gain blood from
/// only one hunting ground each turn." Once that phase; a second ground
/// not offered for the vampire it fed.
#[test]
fn amusement_park_hunting_ground_feeds_one_vampire_once() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let hg = in_play(&mut e, 0, "Amusement Park Hunting Ground", None);
    let park = in_play(&mut e, 0, "Park Hunting Ground", None);
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    pick(&mut e, use_on(hg, 0, None, Target::Minion(a)));
    assert_eq!(blood(&e, a), 5);
    let req = expect(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(uses(&req, hg.0).is_empty(), "once a phase");
    assert!(!req
        .options
        .contains(&use_on(park, 0, None, Target::Minion(a))));
    assert!(req
        .options
        .contains(&use_on(park, 0, None, Target::Minion(b))));
}

/// Gang Territory: "During your influence phase, you can lock this
/// location to add 1 blood to a Brujah antitribu in your uncontrolled
/// region." Not another clan's.
#[test]
fn gang_territory_adds_blood_to_an_uncontrolled_brujah_antitribu() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let slot = |card| zvtes::state::UncontrolledSlot {
        card,
        copy: Default::default(),
        counters: 0,
        banished: None,
    };
    e.state.seats[0].uncontrolled = vec![
        slot(textless(|c| c.clan == "Brujah antitribu", 0)),
        slot(textless(|c| c.clan == "Brujah", 0)),
    ];
    let g = in_play(&mut e, 0, "Gang Territory", None);
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    let add = |i| use_on(g, 0, None, Target::Uncontrolled(i));
    assert!(!req.options.contains(&add(1)), "not a Brujah antitribu");
    pick(&mut e, add(0));
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 1);
    assert!(e.state.cards[g.index()].locked);
}

/// Information Network: "Lock to give a Nosferatu antitribu you control +1
/// intercept." Seat 1's Nosferatu antitribu blocks a +1 stealth bleed
/// with it; a Nosferatu is not offered it.
#[test]
fn information_network_gives_a_blocking_nosferatu_antitribu_intercept() {
    for clan in ["Nosferatu antitribu", "Nosferatu"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let n = e.add_minion(SeatId(1), textless(|c| c.clan == clan, 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let net = in_play(&mut e, 1, "Information Network", None);
        hand(&mut e, 0, &["Lost in Crowds"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(n));
        pick_when(&mut e, play(0, 0, a));
        if clan == "Nosferatu" {
            never_offered_until(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                ability(net.0, 0),
            );
            continue;
        }
        pick_when(&mut e, ability(net.0, 0));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
        assert!(e.state.cards[net.index()].locked);
    }
}

/// The Path of Metamorphosis: "Cards requiring Vicissitude [vic] cost
/// Tzimisce -1 blood. Minions can burn this card as a Ⓓ action that
/// inflicts 1 unpreventable environmental damage on acting vampires."
#[test]
fn the_path_of_metamorphosis_discounts_vicissitude_and_bites_its_burner() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), textless(|c| c.clan == "Tzimisce", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let p = in_play(&mut e, 0, "Path of Metamorphosis, The", None);
    let card = find_library("Horrid Form");
    assert_eq!(e.state.printed_cost_off(t, card, 0).blood, 1);
    assert_eq!(
        e.state.printed_cost_off(z, card, 0).blood,
        0,
        "not a Tzimisce"
    );
    e.state.active = SeatId(1);
    let req = to_minion_phase(&mut e, SeatId(1));
    let burn = req
        .options
        .iter()
        .find(|o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), actor: Some(m), .. } if *c == p && *m == z))
        .copied()
        .expect("the burn offered");
    pick(&mut e, burn);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[p.index()].in_play);
    assert_eq!(blood(&e, z), 3);
}

/// Seat 0's Toreador antitribu beside another vampire, seat 1's Political
/// Ally (a Ventrue's) and vampire, The Art of Love in seat 0's hand.
fn art_of_love(seats: usize) -> (Engine, MinionId) {
    let mut e = game(seats);
    e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Toreador antitribu", 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Art of Love, The", "Deflection"]);
    (e, ally)
}

/// The Art of Love: "Take control of an ally controlled by another
/// Methuselah until the end of your turn." A Ventrue's ally taken by a
/// Methuselah with none (requirements do not apply: TOM 19960226-1),
/// still theirs in their discard phase, back as their turn ends (ANK
/// 20210313).
// Rulings: [TOM 19960226-1] [ANK 20210313].
#[test]
fn the_art_of_love_borrows_an_ally_until_the_turn_ends() {
    let (mut e, ally) = art_of_love(2);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, master_on(0, Target::Minion(ally)));
    request(&mut e);
    assert_eq!(e.state.minions[ally.index()].owner, SeatId(0));
    // The ally acts for its taker.
    pick_when(&mut e, bleed(ally));
    until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert_eq!(
        e.state.minions[ally.index()].owner,
        SeatId(0),
        "the discard phase"
    );
    assert_eq!(e.state.seats[1].pool, 10 - 3);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[ally.index()].owner, SeatId(1), "back");
}

/// The Art of Love takes no copy of a unique ally its player controls
/// (rules-qa 58.19, 121.33; no self-contest, 7.1): seat 0 with its own
/// Carlton Van Wyk is not offered seat 1's; seat 1's Political Ally is.
/// Both Carltons are set uncontested (the fixture: a copy entering play
/// contests them).
#[test]
fn the_art_of_love_takes_no_copy_of_an_owned_unique_ally() {
    let mut e = game(2);
    e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Toreador antitribu", 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let mine = e.add_ally(SeatId(0), find_library("Carlton Van Wyk"));
    let theirs = e.add_ally(SeatId(1), find_library("Carlton Van Wyk"));
    let other = e.add_ally(SeatId(1), find_library("Political Ally"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Art of Love, The"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    for m in [mine, theirs] {
        e.state.minions[m.index()].contested = false;
        let card = e.state.minions[m.index()].ally_id().expect("an ally");
        e.state.cards[card.index()].contested = false;
    }
    let req = request(&mut e);
    assert!(e.state.minions[theirs.index()].present());
    assert!(req.options.contains(&master_on(0, Target::Minion(other))));
    assert!(!req.options.contains(&master_on(0, Target::Minion(theirs))));
}

/// The Art of Love's ally lost meanwhile is not returned (ANK 20210313):
/// borrowed from seat 2, it bleeds seat 1, whose Pentweret steals it for
/// good; it stays seat 1's past seat 0's turn.
// Rulings: [ANK 20210313].
#[test]
fn the_art_of_love_ally_lost_meanwhile_is_not_returned() {
    let mut e = game(3);
    e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Toreador antitribu", 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), zvtes::state::find_card("Pentweret"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(2), find_library("Political Ally"));
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Art of Love, The"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, master_on(0, Target::Minion(ally)));
    pick_when(&mut e, bleed(ally));
    pick_when(&mut e, block(p));
    pick_matching(
        &mut e,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == p),
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let v = &e.state.minions[ally.index()];
    assert!(v.in_play);
    assert_eq!(v.owner, SeatId(1), "not returned to seat 2");
}

/// The Art of Love's ally is removed from the game when it should return
/// to an ousted Methuselah (ANK 20181110): it bleeds its own Methuselah
/// out; at the end of the turn it leaves play, to no ash heap. Its taker
/// ousted while holding it: removed too (ANK 20181110, LSJ 20081105).
// Rulings: [ANK 20181110] [LSJ 20081105]. [ANK 20181110].
#[test]
fn the_art_of_love_ally_removed_when_its_methuselah_is_ousted() {
    let (mut e, ally) = art_of_love(3);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, master_on(0, Target::Minion(ally)));
    request(&mut e);
    e.state.seats[1].pool = 3;
    pick_when(&mut e, bleed(ally));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert!(e.state.minions[ally.index()].in_play, "until the turn ends");
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[ally.index()].in_play,
        "removed, not returned"
    );
    assert!(!ash_has(&e, 1, "Political Ally"));
    // The taker ousted on its own turn (Rewilding burning its location's
    // 2 pool): the ally is removed with its minions.
    let (mut e, ally) = art_of_love(3);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, master_on(0, Target::Minion(ally)));
    request(&mut e);
    e.state.seats[0].pool = 2;
    let place = in_play(&mut e, 0, "Elysian Fields", None);
    hand(&mut e, 0, &["Rewilding"]);
    let g = e.add_minion(SeatId(0), plain("Gangrel", 0), 5, false);
    pick(
        &mut e,
        Choice::Play {
            card: 1,
            mode: 0,
            actor: Some(g),
            target: Target::Card(place),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].ousted);
    assert!(!e.state.minions[ally.index()].in_play);
    assert_eq!(
        e.state.minions[ally.index()].owner,
        SeatId(0),
        "never returned"
    );
}

/// Gangrel Conspiracy: "Cancel a blood hunt called on a Gangrel
/// antitribu. You may play this card during your turn." Once the blood
/// hunt passed (LSJ 20081203-2), by the diablerist's own Methuselah in
/// their turn: the vampire stays, the card to the ash heap, counted
/// against the next master phase (rules-qa 3.3). Not offered for a
/// Lasombra.
// Rulings: [LSJ 20081203-2].
#[test]
fn gangrel_conspiracy_cancels_a_blood_hunt_on_a_gangrel_antitribu() {
    let (mut e, d, _) = blood_hunt_on("Gangrel antitribu");
    hand(&mut e, 0, &["Gangrel Conspiracy"]);
    let cancel = master_on(0, Target::Minion(d));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(req.options.contains(&cancel), "{:?}", req.options);
    pick(&mut e, cancel);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[d.index()].in_play, "not burned");
    assert!(ash_has(&e, 0, "Gangrel Conspiracy"));
    assert!(e.state.seats[0].out_of_turn.is_some(), "spent");
    let (mut e, d, _) = blood_hunt_on("Lasombra");
    hand(&mut e, 0, &["Gangrel Conspiracy"]);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        master_on(0, Target::Minion(d)),
    );
    assert!(!e.state.minions[d.index()].in_play, "burned");
}

/// Gangrel Conspiracy canceling a Veles' Hunt on a Gangrel antitribu: the
/// referendum counts as canceled, "no more Veles' Hunts can be played this
/// game" (red reading; rules-qa 116.87, Dogs of War's `cancels: true`).
#[test]
fn gangrel_conspiracy_cancels_a_veles_hunt_barring_it() {
    let mut e = game(2);
    let gangrel = textless(|c| c.clan == "Gangrel" && c.capacity >= 5, 0);
    let cap = CRYPT[gangrel as usize].capacity;
    let g = e.add_minion(SeatId(0), gangrel, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let y = e.add_minion(
        SeatId(1),
        textless(|c| c.clan == "Gangrel antitribu" && c.capacity < cap, 0),
        3,
        false,
    );
    e.add_minion(SeatId(1), textless(|c| c.capacity < cap, 0), 3, false);
    hand(&mut e, 1, &["Gangrel Conspiracy"]);
    hand(&mut e, 0, &["Veles' Hunt"]);
    pass_referendum(&mut e, g, &[Choice::ChooseMinion(y)]);
    let cancel = master_on(0, Target::Minion(y));
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert!(req.options.contains(&cancel), "{:?}", req.options);
    e.choose(index_of(&req, cancel));
    request(&mut e);
    assert!(e.state.minions[y.index()].in_play, "not burned");
    assert!(ash_has(&e, 1, "Gangrel Conspiracy"));
    assert!(e.state.once_played.contains(&find_library("Veles' Hunt")));
}

/// Gangrel Conspiracy is never offered while the blood hunt is still
/// polled (LSJ 20081203-2: "after votes and ballots have been cast").
#[test]
fn gangrel_conspiracy_waits_for_the_blood_hunt_to_pass() {
    let mut e = game(2);
    let d = e.add_minion(
        SeatId(0),
        textless(
            |c| c.clan == "Gangrel antitribu" && c.sect == Some(Sect::Sabbat),
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(0), sabbat(1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, true);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    hand(&mut e, 0, &["Gangrel Conspiracy"]);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: d,
            action: ActionKind::Diablerie,
            target: Some(z),
        },
    );
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::BloodHuntPolling,
        master_on(0, Target::Minion(d)),
    );
}

/// Legacy of Caine's theft of `v` (its card `legacy`) from `t`.
fn theft(legacy: CardId, v: MinionId, t: MinionId) -> Choice {
    use_on(legacy, 0, Some(v), Target::Minion(t))
}

/// Seat 0's capacity-7+ vampire `v` with `b` blood bearing seat 1's Legacy
/// of Caine, seat 0's `w` (4 blood), seat 1's `x` (4 blood).
fn legacy_bearer(b: u8) -> (Engine, MinionId, MinionId, MinionId, CardId) {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), textless(|c| c.capacity == 7, 0), b, false);
    let w = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let legacy = in_play(&mut e, 1, "Legacy of Caine", Some(v));
    (e, v, w, x, legacy)
}

/// Legacy of Caine: "Put this card on a vampire with a capacity above 6
/// controlled by another Methuselah. This vampire cannot hunt as normal.
/// He or she may hunt by stealing 1 blood from another vampire as a Ⓓ
/// action." Played by seat 0 on seat 1's capacity-7 vampire (the card
/// itself on it); seat 1's minion phase: no rulebook hunt, the theft of
/// another vampire's blood, any Methuselah's, once each turn.
// Rulings: [RBK action-card-or-card-in-play] (group "Special Hunt action").
#[test]
fn legacy_of_caine_replaces_the_hunt_with_a_theft() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), textless(|c| c.capacity == 7, 0), 2, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let small = e.add_minion(SeatId(0), textless(|c| c.capacity == 4, 0), 2, false);
    hand(&mut e, 0, &["Legacy of Caine"]);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(
        !req.options.contains(&master_on(0, Target::Minion(small))),
        "own, small"
    );
    assert!(
        !req.options.contains(&master_on(0, Target::Minion(y))),
        "capacity"
    );
    pick(&mut e, master_on(0, Target::Minion(v)));
    request(&mut e);
    let legacy = placed_on(&e, "Legacy of Caine", Some(v));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !req.options.contains(&action(v, ActionKind::Hunt)),
        "no rulebook hunt"
    );
    assert!(
        req.options.contains(&theft(legacy, v, y)),
        "its own Methuselah's"
    );
    assert!(
        !req.options.contains(&theft(legacy, v, v)),
        "another vampire"
    );
    e.choose(index_of(&req, theft(legacy, v, a)));
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, v), 3);
    assert_eq!(blood(&e, a), 3);
    e.state.minions[v.index()].locked = false;
    assert!(
        !request(&mut e).options.contains(&theft(legacy, v, y)),
        "once a turn"
    );
}

/// Legacy of Caine's theft is capped at the target's blood, a bloodless
/// target legal and the hunt then gaining nothing (LSJ 20050720-2); at
/// capacity, the blood taken goes to the bank (LSJ 20010809-1); with no
/// other vampire it is not offered (LSJ 19970224); a hunt bonus takes
/// more from the target (RTR 20180511, Aaron's Feeding Razor).
// Rulings: [LSJ 20050720-2] [RTR 20010711] [LSJ 20100915]. [LSJ
// 20010809-1]. [LSJ 19970224]. [RTR 20180511] [RTR 20030519].
#[test]
fn legacy_of_caine_theft_is_capped_and_needs_another_vampire() {
    // A bloodless target.
    let (mut e, v, _, x, legacy) = legacy_bearer(2);
    e.state.minions[x.index()].blood = 0;
    pick(&mut e, theft(legacy, v, x));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, v), blood(&e, x)), (2, 0));
    // At capacity: the target loses it, the bank takes it.
    let (mut e, v, _, x, legacy) = legacy_bearer(7);
    pick(&mut e, theft(legacy, v, x));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, v), blood(&e, x)), (7, 3));
    // A hunt bonus: 2 from the target.
    let (mut e, v, _, x, legacy) = legacy_bearer(2);
    in_play(&mut e, 0, "Aaron's Feeding Razor", Some(v));
    pick(&mut e, theft(legacy, v, x));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, v), blood(&e, x)), (4, 2));
    // No other vampire: never offered.
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), textless(|c| c.capacity == 7, 0), 2, false);
    e.add_ally(SeatId(0), find_library("Political Ally"));
    e.add_ally(SeatId(1), find_library("Political Ally"));
    let legacy = in_play(&mut e, 1, "Legacy of Caine", Some(v));
    let req = request(&mut e);
    assert!(!req.options.iter().any(|o| matches!(o,
        Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == legacy)));
}

/// Legacy of Caine leaves the card hunts (Abactor) with their own text:
/// its theft does not combine with them (LSJ 20090606).
// Rulings: [LSJ 20090606] [ANK 20190606-2] [LSJ 20060306].
#[test]
fn legacy_of_caine_leaves_card_hunts() {
    let (mut e, v, _, _, legacy) = legacy_bearer(2);
    hand(&mut e, 0, &["Abactor"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, v)), "{:?}", req.options);
    assert!(!req.options.contains(&action(v, ActionKind::Hunt)));
    assert!(req.options.iter().any(|o| matches!(o,
        Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == legacy)));
}

/// The Damned: "Put this card on a vampire with a capacity below 5. The
/// vampire with this card burns 1 additional blood for each bleeding
/// action he or she successfully performs (after resolving the action)."
/// Played on seat 1's capacity-4 vampire (the card itself on it), not on a
/// capacity-5 one; seat 0's bearer bleeds: the burn after resolution, no
/// pass before it; a blocked bleed burns none.
#[test]
fn the_damned_burns_a_blood_after_each_successful_bleed() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let small = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 4, false);
    let big = e.add_minion(SeatId(1), textless(|c| c.capacity == 5, 0), 4, false);
    hand(&mut e, 0, &["Damned, The"]);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(!req.options.contains(&master_on(0, Target::Minion(big))));
    pick(&mut e, master_on(0, Target::Minion(small)));
    request(&mut e);
    placed_on(&e, "Damned, The", Some(small));
    for blocked in [false, true] {
        let mut e = game(2);
        let v = e.add_minion(SeatId(0), textless(|c| c.capacity == 4, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        in_play(&mut e, 1, "Damned, The", Some(v));
        pick(&mut e, bleed(v));
        if blocked {
            pick_when(&mut e, block(x));
            never_offered_until(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                Choice::TakeDamageAfterResolution,
            );
            continue;
        }
        // The burn alone owed after resolution: taken at once (elided).
        to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, v), 3);
        assert_eq!(e.state.seats[1].pool, 9);
    }
}

/// The Damned's burn is ordered by the acting Methuselah among the
/// after-resolution effects, other cards played first (ANK 20180206:
/// Freak Drive [for] before it).
// Rulings: [ANK 20180206].
#[test]
fn the_damned_burn_is_ordered_among_after_resolution_effects() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), textless(|c| c.capacity == 4, 0), 4, false);
    e.state.minions[v.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Damned, The", Some(v));
    hand(&mut e, 0, &["Freak Drive"]);
    pick(&mut e, bleed(v));
    let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    assert!(!req.options.contains(&Choice::Pass), "owed");
    assert!(req.options.contains(&Choice::TakeDamageAfterResolution));
    e.choose(index_of(&req, play(0, 0, v)));
    // Freak Drive resolved, the burn alone left: taken at once (elided).
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, v),
        4 - 1 - 1,
        "Freak Drive's blood, then the burn"
    );
    assert!(
        !e.state.minions[v.index()].locked,
        "unlocked by Freak Drive"
    );
}

/// Seat 0's Sabbat `d` (blood 3) given Danse Macabre, two other vampires
/// (the minion phase not elided), seat 1's vampire.
fn danse() -> (Engine, MinionId) {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), sabbat(0), 3, false);
    e.add_minion(SeatId(0), sabbat(1), 4, false);
    e.add_minion(SeatId(0), sabbat(2), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[d.index()].timed.push((
        zvtes::cards::Lasting::MayUnlockForBloodAfterSuccess {
            blood: 1,
            sect: Sect::Sabbat,
        },
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    (e, d)
}

/// Danse Macabre: "Choose a ready Sabbat vampire you control. Once this
/// turn, when the chosen Sabbat vampire performs a successful action, he
/// or she may burn a blood to unlock after resolving that action." Played
/// on one's own ready Sabbat vampire (not a Camarilla one); after its
/// successful bleed, unlocked for 1 blood; its second successful action
/// that turn: not offered.
#[test]
fn danse_macabre_unlocks_once_after_a_successful_action() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), sabbat(0), 3, false);
    let c = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Danse Macabre"]);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(
        !req.options.contains(&master_on(0, Target::Minion(c))),
        "not Sabbat"
    );
    pick(&mut e, master_on(0, Target::Minion(s)));
    request(&mut e);
    assert!(e.state.minions[s.index()]
        .timed
        .iter()
        .any(|&(l, ..)| matches!(
            l,
            zvtes::cards::Lasting::MayUnlockForBloodAfterSuccess { .. }
        )));
    let (mut e, d) = danse();
    let unlock = Choice::UnlockForBlood { minion: d };
    pick(&mut e, bleed(d));
    pick_when(&mut e, unlock);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[d.index()].locked);
    assert_eq!(blood(&e, d), 2);
    pick(&mut e, action(d, ActionKind::Hunt));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, unlock);
    assert!(e.state.minions[d.index()].locked, "once this turn");
}

/// Danse Macabre declined after one action waits for the next successful
/// one; a blocked action offers none.
#[test]
fn danse_macabre_declined_waits_for_the_next_success() {
    let (mut e, d) = danse();
    let unlock = Choice::UnlockForBlood { minion: d };
    pick(&mut e, bleed(d));
    let req = offering(&mut e, unlock);
    e.choose(index_of(&req, Choice::Pass));
    // Unlocked by hand before the next window is built (a second
    // vampire acts first), it hunts.
    let req = to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[d.index()].locked);
    e.state.minions[d.index()].locked = false;
    let other = e.state.minions_of(SeatId(0)).nth(1).unwrap();
    e.choose(index_of(&req, action(other, ActionKind::Hunt)));
    pick_when(&mut e, action(d, ActionKind::Hunt));
    pick_when(&mut e, unlock);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[d.index()].locked);
    let (mut e, d) = danse();
    let x = e.state.minions_of(SeatId(1)).next().unwrap();
    pick(&mut e, bleed(d));
    pick_when(&mut e, block(x));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::UnlockForBlood { minion: d },
    );
}

/// Club Zombie: "Lock to give a vampire you control +1 intercept for the
/// current action." Seat 1's vampire blocks a +1 stealth bleed with it;
/// an ally attempting is not offered it.
#[test]
fn club_zombie_locks_to_give_a_blocking_vampire_intercept() {
    for ally in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let b = if ally {
            e.add_ally(SeatId(1), find_library("Political Ally"))
        } else {
            e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false)
        };
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let club = in_play(&mut e, 1, "Club Zombie", None);
        hand(&mut e, 0, &["Lost in Crowds"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(b));
        pick_when(&mut e, play(0, 0, a));
        if ally {
            never_offered_until(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                ability(club.0, 0),
            );
            continue;
        }
        pick_when(&mut e, ability(club.0, 0));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
        assert!(e.state.cards[club.index()].locked);
    }
}

/// Club Zombie: "If Club Zombie is unlocked at the start of your turn, a
/// ready vampire you control gains 1 blood during your unlock phase."
/// Unlocked through the turn before: one of its controller's ready
/// vampires, chosen (no pass), gains 1; locked then (for an intercept in
/// another's turn), it unlocks and gives nothing that phase.
#[test]
fn club_zombie_feeds_a_vampire_only_if_unlocked_at_turn_start() {
    for locked in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let club = in_play(&mut e, 0, "Club Zombie", None);
        e.state.cards[club.index()].locked = locked;
        if locked {
            until(&mut e, SeatId(1), WindowLabel::MinionPhase);
            never_offered_until(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                Choice::ChooseMinion(a),
            );
            assert!(!e.state.cards[club.index()].locked, "unlocked as normal");
            assert_eq!(blood(&e, a), 4);
            continue;
        }
        let req = offering(&mut e, Choice::ChooseMinion(a));
        assert_eq!(req.seat, SeatId(0));
        assert!(!req.options.contains(&Choice::Pass), "mandatory");
        e.choose(index_of(&req, Choice::ChooseMinion(a)));
        assert_eq!(blood(&e, a), 5);
    }
}

/// A crypt card of `clan` with `disc` at basic, none of `lacks`.
fn of_clan_with(clan: &str, disc: &str, lacks: &[&str], nth: usize) -> u16 {
    textless(
        |c| {
            c.clan == clan
                && c.disciplines.contains(&disc)
                && lacks.iter().all(|l| {
                    !c.disciplines.contains(l)
                        && !c.disciplines.contains(&l.to_ascii_uppercase().as_str())
                })
        },
        nth,
    )
}

/// Haven Affinity: "Put this card on a Tzimisce. If this Tzimisce blocks,
/// they can play combat cards during the resulting combat as if they had
/// the superior level of all their Disciplines." Played on another
/// Methuselah's Tzimisce (the card itself on it); a basic-[vic] Tzimisce
/// blocking is offered Horrid Form's [VIC] mode; acting and blocked, only
/// [vic].
#[test]
fn haven_affinity_blocking_tzimisce_plays_superior_combat_cards() {
    // Its player needs a ready Tzimisce (its clan icon).
    let mut e = game(2);
    e.add_minion(SeatId(0), of_clan_with("Tzimisce", "vic", &[], 1), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), of_clan_with("Tzimisce", "vic", &[], 0), 4, false);
    hand(&mut e, 0, &["Haven Affinity"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, master_on(0, Target::Minion(t)));
    request(&mut e);
    placed_on(&e, "Haven Affinity", Some(t));
    for blocking in [true, false] {
        let mut e = game(2);
        let tz = of_clan_with("Tzimisce", "vic", &[], 0);
        let (actor, blocker) = if blocking {
            let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
            e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
            let t = e.add_minion(SeatId(1), tz, 4, false);
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
            hand(&mut e, 1, &["Horrid Form"]);
            in_play(&mut e, 0, "Haven Affinity", Some(t));
            (a, t)
        } else {
            let t = e.add_minion(SeatId(0), tz, 4, false);
            e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
            let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
            hand(&mut e, 0, &["Horrid Form"]);
            in_play(&mut e, 1, "Haven Affinity", Some(t));
            (t, b)
        };
        let t = if blocking { blocker } else { actor };
        let seat = e.state.minions[t.index()].owner;
        pick(&mut e, bleed(actor));
        pick_when(&mut e, block(blocker));
        let req = until(&mut e, seat, WindowLabel::CombatBeforeRange);
        assert!(req.options.contains(&play(0, 0, t)), "{:?}", req.options);
        assert_eq!(req.options.contains(&play(0, 1, t)), blocking, "[VIC]");
    }
}

/// Infernal Pact: "Put this card on a Tremere antitribu you control and
/// choose a Discipline. The vampire with this card can play cards that
/// require that Discipline as though he or she has the superior version
/// of that Discipline." Dominate chosen for a Tremere antitribu without
/// it: both Govern the Unaligned modes offered; Presence's superior still
/// barred; the card itself on it.
#[test]
fn infernal_pact_plays_the_chosen_discipline_at_superior() {
    let mut e = game(2);
    let tc = textless(
        |c| {
            c.clan == "Tremere antitribu"
                && !["dom", "DOM", "pre", "PRE"]
                    .iter()
                    .any(|d| c.disciplines.contains(d))
        },
        0,
    );
    let t = e.add_minion(SeatId(0), tc, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Infernal Pact", "Govern the Unaligned"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, master_on(0, Target::Minion(t)));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    let own = CRYPT[tc as usize]
        .disciplines
        .iter()
        .find(|d| d.chars().all(|c| c.is_ascii_uppercase()))
        .map(|d| d.to_ascii_lowercase());
    if let Some(d) = own {
        assert!(
            req.options
                .iter()
                .any(|o| matches!(o, Choice::ChooseDiscipline(x) if *x == d)),
            "one already superior: a legal no-op"
        );
    }
    e.choose(index_of(&req, Choice::ChooseDiscipline("dom")));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Infernal Pact", Some(t));
    assert!(req.options.contains(&play(0, 0, t)), "{:?}", req.options);
    let govern = find_library("Govern the Unaligned");
    assert!(
        zvtes::cards::has_disciplines(&e.state, t, govern, 1),
        "[DOM]"
    );
    let aire = find_library("Aire of Elation");
    assert!(
        !zvtes::cards::has_disciplines(&e.state, t, aire, 1),
        "[PRE]"
    );
    assert_eq!(e.state.discipline_level(t, "dom"), 0, "its level unchanged");
}

/// Two seats with full hands and 20-card libraries, seat 0 at its next
/// master phase with `card` in hand slot 0, a ready vampire of `clan`.
fn master_table(card: &str, clan: &str) -> Engine {
    let mut e = game(2);
    e.add_minion(SeatId(0), textless(|c| c.clan == clan, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let filler = find_library("Deflection");
    for s in 0..2 {
        for _ in 0..20 {
            e.state.seats[s].library.push(filler);
        }
    }
    hand(&mut e, 0, &[card]);
    fill_hand(&mut e, 0);
    fill_hand(&mut e, 1);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    e
}

/// `seat`'s secret pick, asked with the three signs.
#[track_caller]
fn sign(e: &mut Engine, seat: u8, n: u8) {
    let req = expect(e, SeatId(seat), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![set_x(1), set_x(2), set_x(3)]);
    e.choose(index_of(&req, set_x(n)));
}

/// Lupine Assault: "Do not replace until your influence phase. Each
/// Methuselah (including you) discards 5 cards of his or her choice, then
/// draws back up to his or her hand size." The player first, 5 chosen
/// each, hands drawn back up; the player's one short until its influence
/// phase.
#[test]
fn lupine_assault_each_methuselah_discards_five_and_draws_up() {
    let mut e = master_table("Lupine Assault", "Brujah antitribu");
    // A Deflection among seat 1's Forms of Mist: its 5 chosen, kept.
    e.state.seats[1].hand.take(0);
    hand(&mut e, 1, &["Deflection"]);
    pick(&mut e, master_play(0));
    // Seat 0's hand all alike: its discards elided.
    for _ in 0..5 {
        let req = expect(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
        let form = req
            .options
            .iter()
            .copied()
            .find(|&o| {
                matches!(o, Choice::Discard(i)
                    if e.state.seats[1].hand[i as usize] == find_library("Form of Mist"))
            })
            .expect("a Form of Mist");
        e.choose(index_of(&req, form));
    }
    request(&mut e);
    let form = find_library("Form of Mist");
    let count = |e: &Engine, s: usize| e.state.seats[s].ash.iter().filter(|&&c| c == form).count();
    assert_eq!((count(&e, 0), count(&e, 1)), (5, 5));
    assert!(
        e.state.seats[1].hand.contains(&find_library("Deflection")),
        "chosen"
    );
    assert_eq!(e.state.seats[0].hand.len(), 6, "not replaced yet");
    assert_eq!(e.state.seats[1].hand.len(), 7);
    // A Sabbat vampire's path offered: the influence phase not elided.
    until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    assert_eq!(
        e.state.seats[0].hand.len(),
        7,
        "replaced at the influence phase"
    );
}

/// Lupine Assault: "Only one Lupine Assault may be played in a game": the
/// other Methuselah's copy is never offered after it.
#[test]
fn lupine_assault_only_once_a_game() {
    let mut e = master_table("Lupine Assault", "Brujah antitribu");
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.seats[1].hand.take(0);
    hand(&mut e, 1, &["Lupine Assault"]);
    pick(&mut e, master_play(0));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { actor: None, .. })
    });
}

/// Cracking the Wall: "Play Rock-Paper-Scissors with another Methuselah.
/// If you tie, you each discard 1 card at random. Otherwise, the loser
/// discards their hand." Seat 1 picks first, hidden; scissors lose to
/// rock: seat 1's whole hand to the ash heap, drawn back up; the card
/// itself not replaced until its player's discard phase.
#[test]
fn cracking_the_wall_loser_discards_the_hand() {
    let mut e = master_table("Cracking the Wall", "Malkavian antitribu");
    pick(&mut e, master_on(0, Target::Seat(SeatId(1))));
    sign(&mut e, 1, 3);
    sign(&mut e, 0, 1);
    request(&mut e);
    let form = find_library("Form of Mist");
    let count = |e: &Engine, s: usize| e.state.seats[s].ash.iter().filter(|&&c| c == form).count();
    assert_eq!((count(&e, 0), count(&e, 1)), (0, 7));
    assert_eq!(e.state.seats[1].hand.len(), 7, "drawn back up");
    assert_eq!(e.state.seats[0].hand.len(), 6);
    until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert_eq!(
        e.state.seats[0].hand.len(),
        7,
        "replaced at the discard phase"
    );
}

/// Cracking the Wall's tie: each discards 1 card at random, replaced.
#[test]
fn cracking_the_wall_tie_discards_one_each_at_random() {
    let mut e = master_table("Cracking the Wall", "Malkavian antitribu");
    pick(&mut e, master_on(0, Target::Seat(SeatId(1))));
    sign(&mut e, 1, 2);
    sign(&mut e, 0, 2);
    request(&mut e);
    let form = find_library("Form of Mist");
    let count = |e: &Engine, s: usize| e.state.seats[s].ash.iter().filter(|&&c| c == form).count();
    assert_eq!((count(&e, 0), count(&e, 1)), (1, 1));
    assert_eq!(e.state.seats[0].hand.len(), 6);
    assert_eq!(e.state.seats[1].hand.len(), 7);
}

/// Malkavian Game: "Play Rock-Paper-Scissors with another Methuselah. The
/// winner steals 2 pool from the loser." Either way: paper beats rock.
#[test]
fn malkavian_game_winner_steals_two_pool() {
    for (mine, theirs, pool0) in [(2, 1, 12), (1, 2, 8)] {
        let mut e = master_table("Malkavian Game", "Malkavian antitribu");
        pick(&mut e, master_on(0, Target::Seat(SeatId(1))));
        sign(&mut e, 1, theirs);
        sign(&mut e, 0, mine);
        request(&mut e);
        assert_eq!(pool(&e), vec![pool0, 20 - pool0]);
    }
}

/// Malkavian Game: "Each time you tie, play again, but for 1 additional
/// pool." A tie, then seat 1 wins: 3 pool moved; a loser with less than
/// the stake loses what they have, ousted (rules-qa 116.54).
#[test]
fn malkavian_game_tie_raises_the_stake() {
    for short in [false, true] {
        let mut e = master_table("Malkavian Game", "Malkavian antitribu");
        if short {
            e.state.seats[0].pool = 2;
        }
        pick(&mut e, master_on(0, Target::Seat(SeatId(1))));
        sign(&mut e, 1, 1);
        sign(&mut e, 0, 1);
        sign(&mut e, 1, 2);
        sign(&mut e, 0, 1);
        if short {
            assert!(
                matches!(e.run(), zvtes::game::Step::Over(_)),
                "seat 1 alone"
            );
            assert!(e.state.seats[0].ousted, "its last pool lost");
            assert_eq!(
                e.state.seats[1].pool,
                10 + 2 + 6,
                "what it had, the oust's 6"
            );
        } else {
            request(&mut e);
            assert_eq!(pool(&e), vec![7, 13]);
        }
    }
}

/// Malkavian Game lost by its player with three at the table: ousted on its
/// own turn, the turn passes to the next Methuselah (fuzz 31b seed 210
/// asked the ousted seat's master phase on).
#[test]
fn malkavian_game_lost_by_its_player_passes_the_turn() {
    let mut e = game(3);
    e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Malkavian antitribu", 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Malkavian Game"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    e.state.seats[0].pool = 2;
    pick(&mut e, master_on(0, Target::Seat(SeatId(1))));
    sign(&mut e, 1, 2);
    sign(&mut e, 0, 1);
    // At once: a discard-down the oust causes is asked on the next turn.
    assert!(e.state.seats[0].ousted, "its last pool lost");
    assert_eq!(e.state.active, SeatId(1), "the turn passed");
    assert_eq!(request(&mut e).seat, SeatId(1));
}

/// Seat 0's vampire `a` bleeds seat 1, whose 1-life Political Ally blocks;
/// seat 1 holds Left for Dead: the ally would burn to `a`'s hand strike.
fn ally_left_for_dead() -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Left for Dead"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(ally));
    (e, a, ally)
}

/// Left for Dead: "Only usable when an ally would be burned in combat.
/// Combat ends. Put this card on that ally; that ally is not burned. Put 1
/// life on the ally from the blood bank if he or she has no life. Lock the
/// ally. This ally does not unlock as normal. … Burn this card during his
/// or her next unlock phase." Played by the ally's own Methuselah out of
/// turn: the ally in play with 1 life, locked, the card itself on it; its
/// next unlock phase it stays locked and the card burns; the one after, it
/// unlocks.
#[test]
fn left_for_dead_saves_an_ally_burned_in_combat() {
    let (mut e, _, ally) = ally_left_for_dead();
    let save = master_on(0, Target::Minion(ally));
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Pass, save]);
    e.choose(index_of(&req, save));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[ally.index()];
    assert!(v.in_play, "not burned");
    assert_eq!(v.blood, 1, "1 life from the bank");
    assert!(v.locked);
    let card = placed_on(&e, "Left for Dead", Some(ally));
    assert!(e.combat().is_none(), "combat ended");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        e.state.minions[ally.index()].locked,
        "does not unlock as normal"
    );
    assert!(
        !e.state.cards[card.index()].in_play,
        "burned in that unlock phase"
    );
    assert!(ash_has(&e, 1, "Left for Dead"));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[ally.index()].locked,
        "unlocks the turn after"
    );
}

/// Left for Dead declined: the ally burns as the damage would have had it;
/// not offered to the active Methuselah (out-of-turn).
#[test]
fn left_for_dead_declined_burns_the_ally() {
    let (mut e, _, ally) = ally_left_for_dead();
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play, "burned");
    assert!(ash_has(&e, 1, "Political Ally"));
    // Seat 0's own: never offered in its own turn.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Left for Dead"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(ally));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        master_on(0, Target::Minion(ally)),
    );
    assert!(!e.state.minions[ally.index()].in_play);
}

/// Left for Dead: "If he or she enters combat, he or she is burned." Its
/// ally, unlocked, blocks: burned as the combat begins, the combat over.
#[test]
fn left_for_dead_ally_entering_combat_is_burned() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Left for Dead", Some(ally));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(ally));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[ally.index()].in_play,
        "burned entering combat"
    );
    assert!(ash_has(&e, 1, "Political Ally"));
    assert_eq!(blood(&e, a), 4, "no strike landed");
}

/// A second Left for Dead on the first one's "If he or she enters combat,
/// he or she is burned": not offered (red reading: the burn is the card's
/// text as the combat begins, no burn in combat), the ally burned.
#[test]
fn left_for_dead_second_copy_not_offered_on_the_entering_burn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Left for Dead", Some(ally));
    hand(&mut e, 1, &["Left for Dead"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(ally));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        master_on(0, Target::Minion(ally)),
    );
    assert!(!e.state.minions[ally.index()].in_play);
    assert!(e.state.seats[1]
        .hand
        .contains(&find_library("Left for Dead")));
}

/// Left for Dead: "Effects to begin a new combat cannot be used on the Left
/// for Dead ally as he is not ready" (LSJ 20020509): Bum's Rush cannot
/// target it, another ally of the same Methuselah it can.
// Rulings: [LSJ 20020509].
#[test]
fn left_for_dead_ally_cannot_begin_a_new_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    let other = e.add_ally(SeatId(1), find_library("Political Ally"));
    in_play(&mut e, 1, "Left for Dead", Some(ally));
    hand(&mut e, 0, &["Bum's Rush"]);
    let rush = |t| Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(a),
        target: Target::Minion(t),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&rush(other)), "{:?}", req.options);
    assert!(!req.options.contains(&rush(ally)));
}

/// FBI Special Affairs Division "does not trigger if the ally is saved by
/// Left for Dead" (PIB 20121028): no counter, no damage to the acting
/// vampire; declined, it does.
// Rulings: [PIB 20121028].
#[test]
fn fbi_special_affairs_division_ignores_an_ally_left_for_dead() {
    for saved in [true, false] {
        let (mut e, a, ally) = ally_left_for_dead();
        let fbi = in_play(&mut e, 1, "FBI Special Affairs Division", None);
        let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
        let pick_ = if saved {
            master_on(0, Target::Minion(ally))
        } else {
            Choice::Pass
        };
        e.choose(index_of(&req, pick_));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[ally.index()].in_play, saved);
        let counters = e.state.cards[fbi.index()].counters;
        assert_eq!(
            (counters, blood(&e, a)),
            if saved { (0, 4) } else { (1, 2) }
        );
    }
}

/// The Art of Love on an incapacitated imbued (an ally): back to its
/// Methuselah as the turn ends, still incapacitated (LSJ 20010119).
// Rulings: [LSJ 20010119].
#[test]
fn the_art_of_love_returns_an_incapacitated_imbued_as_it_is() {
    let (mut e, _) = art_of_love(2);
    let imbued = CRYPT.iter().position(|c| c.imbued).expect("an imbued") as u16;
    let x = e.add_minion(SeatId(1), imbued, 3, false);
    e.state.minions[x.index()].incapacitated = true;
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, master_on(0, Target::Minion(x)));
    request(&mut e);
    assert_eq!(e.state.minions[x.index()].owner, SeatId(0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let v = &e.state.minions[x.index()];
    assert_eq!(v.owner, SeatId(1));
    assert!(v.in_play && v.incapacitated, "returned as it is");
}

// ---- Allies and retainers -------------------------------------------------------

/// The card of ally `m` in play (its own card, the ally its body).
fn ally_card(e: &Engine, m: MinionId) -> CardId {
    let i = e
        .state
        .cards
        .iter()
        .position(|c| c.body == Some(m) && c.present())
        .expect("the ally's card");
    CardId(i as u32)
}

/// Seat 0's Herald of Topheth (5 life, 3 strength) rushes seat 1's ally
/// `name`, given `life`; seat 1 also has a vampire. Herald and the ally.
fn herald_rushes(name: &str, life: u8) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let h = e.add_ally(SeatId(0), find_library("Herald of Topheth"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library(name));
    e.state.minions[ally.index()].blood = life;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let herald = ally_card(&e, h);
    pick(&mut e, use_on(herald, 0, Some(h), Target::Minion(ally)));
    (e, h, ally)
}

/// Black Spiral Buddy: "Damage from Black Spiral Buddy's hand strikes is
/// aggravated. Once each combat, Black Spiral Buddy gets an additional
/// strike (limited)." The additional strike is mandatory (PIB 20121028):
/// no decision declines it, two aggravated hand strikes land in the first
/// round; pressed into a second round, he strikes once.
// Rulings: [PIB 20121028].
#[test]
fn black_spiral_buddy_strikes_twice_once_each_combat() {
    let (mut e, h, buddy) = herald_rushes("Black Spiral Buddy", 10);
    // A press each combat for the Buddy: a second round.
    let card = ally_card(&e, buddy);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::HostCombatPress(1),
        source: card,
    });
    pick_when(&mut e, Choice::UsePress);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, h), 5 - 3, "2 + 1 hand strikes");
    assert_eq!(blood(&e, buddy), 10 - 3 - 3);
}

/// Black Spiral Buddy: "If Black Spiral Buddy has fewer than 3 life during
/// your unlock phase, he gains 1 life": at 2, 3; at 3, still 3.
#[test]
fn black_spiral_buddy_heals_below_three_life() {
    for (life, after) in [(2, 3), (3, 3)] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let b = e.add_ally(SeatId(1), find_library("Black Spiral Buddy"));
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[b.index()].blood = life;
        to_minion_phase(&mut e, SeatId(1));
        assert_eq!(blood(&e, b), after);
    }
}

/// Marijava Ghoul: "The minion with this retainer gets +1 stealth when
/// attempting an action that requires Presence [pre]": its bearer's [pre]
/// action card (Entrancement) one more stealth than without it, its plain
/// bleed none.
#[test]
fn marijava_ghoul_gives_stealth_on_presence_actions() {
    let stealth = |ghoul: bool, presence: bool| {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("pre", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if ghoul {
            in_play(&mut e, 0, "Marijava Ghoul", Some(a));
        }
        hand(&mut e, 0, &["Entrancement"]);
        let act = if presence { play(0, 0, a) } else { bleed(a) };
        pick(&mut e, act);
        request(&mut e);
        e.action().expect("action").stealth(&e.state)
    };
    assert_eq!(stealth(true, true), stealth(false, true) + 1, "Presence");
    assert_eq!(stealth(true, false), stealth(false, false), "a bleed");
}

/// Seat 0's capacity-6 vampire takes a political action (Rabble Razing, 1
/// stealth) or a bleed (none); seat 1 has a Political Antagonist, a
/// Ventrue antitribu and a Brujah. Actor, the Antagonist and both
/// vampires.
fn antagonist(political: bool) -> (Engine, MinionId, MinionId, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 6, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let pa = e.add_ally(SeatId(1), find_library("Political Antagonist"));
    let va = e.add_minion(
        SeatId(1),
        textless(|c| c.clan == "Ventrue antitribu", 0),
        4,
        false,
    );
    let b = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Rabble Razing"]);
    pick(&mut e, if political { play(0, 0, a) } else { bleed(a) });
    (e, a, pa, va, b)
}

/// Political Antagonist: "During a political action, the Antagonist may
/// lock to give a Ventrue antitribu +1 intercept." The Ventrue antitribu's
/// block of a 1-stealth political action reaches (the bots see it), the
/// Brujah's does not; used, the block succeeds and the
/// Antagonist is locked.
#[test]
fn political_antagonist_locks_for_a_blocking_ventrue_antitribu() {
    let (mut e, a, pa, va, b) = antagonist(true);
    let req = offering(&mut e, block(va));
    assert!(e.block_can_reach(req.seat, va), "the Antagonist's +1");
    assert!(!e.block_can_reach(req.seat, b), "no Ventrue antitribu");
    pick(&mut e, block(va));
    let card = ally_card(&e, pa);
    pick_when(&mut e, use_on(card, 0, None, Target::None));
    request(&mut e);
    assert!(e.state.minions[pa.index()].locked);
    assert!(e.state.minions[va.index()].locked, "blocked");
    assert!(e.state.minions[a.index()].locked);
    // A bleed: never offered.
    let (mut e, _, pa, va, _) = antagonist(false);
    pick(&mut e, block(va));
    let card = ally_card(&e, pa);
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        use_on(card, 0, None, Target::None),
    );
}

/// Seat 0's vampire bleeds; seat 1's ally `name`, at `life`, blocks with
/// `cards` in hand. The vampire and the ally, in combat.
fn ally_blocks(name: &str, life: u8, cards: &[&str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library(name));
    e.state.minions[ally.index()].blood = life;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, cards);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(ally));
    (e, a, ally)
}

/// Rafastio Ghoul: "can play cards requiring basic Blood Sorcery [tha] as a
/// vampire with a capacity of 3." Burst of Sunlight [tha] offered, its
/// [THA] not (ANK 20180111); its aggravated damage to him is a life lost,
/// no torpor (LSJ 20060409): 4 life, less its own 1 and the hand strike's
/// 1, still in play.
// Rulings: [ANK 20180111] [LSJ 20060409].
#[test]
fn rafastio_ghoul_takes_burst_of_sunlights_damage_as_life() {
    let (mut e, a, r) = ally_blocks("Rafastio Ghoul", 4, &["Burst of Sunlight"]);
    let req = offering(&mut e, play(0, 0, r));
    assert!(!req.options.contains(&play(0, 1, r)), "basic only");
    e.choose(index_of(&req, play(0, 0, r)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].in_torpor, "1R aggravated");
    let v = &e.state.minions[r.index()];
    assert!(v.in_play && !v.in_torpor);
    assert_eq!(v.blood, 4 - 1 - 1);
}

/// Talaq, The Immortal: "may play cards that require basic Quietus [qui] or
/// basic Blood Sorcery [tha] as a vampire": at long range (his maneuver),
/// Taste of Death [qui] and Burst of Sunlight [tha] offered at his strike,
/// beside his own 1R, neither
/// superior; he does not have them (LSJ 19980303: Tremere Convocation sees
/// no Blood Sorcery on him).
// Rulings: [LSJ 19980303] [ANK 20180111].
#[test]
fn talaq_plays_quietus_and_blood_sorcery_without_having_them() {
    let (mut e, _, t) = ally_blocks(
        "Talaq, The Immortal",
        3,
        &["Taste of Death", "Burst of Sunlight"],
    );
    let talaq = ally_card(&e, t);
    // His maneuver to long range: Taste of Death's.
    pick_when(&mut e, use_on(talaq, 1, None, Target::None));
    let req = offering(&mut e, play(0, 0, t));
    for p in [play(0, 0, t), play(1, 0, t)] {
        assert!(req.options.contains(&p), "{p:?}");
    }
    for p in [play(0, 1, t), play(1, 1, t)] {
        assert!(!req.options.contains(&p), "{p:?}");
    }
    assert!(req.options.contains(&use_on(talaq, 0, None, Target::None)));
    for d in ["qui", "tha"] {
        assert!(!zvtes::cards::holds_for_host(
            zvtes::cards::Requirement::HostDiscipline(d),
            &e.state,
            t
        ));
    }
}

/// Survivalist: "He can lock to give a Brujah antitribu you control 1
/// maneuver", the turn he is recruited too (ANK 20180517): offered for
/// his controller's blocked Brujah antitribu, used at once (range long:
/// neither hand strike lands), the Survivalist locked; not for a Brujah.
// Rulings: [ANK 20180517].
#[test]
fn survivalist_locks_to_give_a_brujah_antitribu_a_maneuver() {
    for (clan, offered) in [("Brujah antitribu", true), ("Brujah", false)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), textless(|c| c.clan == clan, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let s = e.add_ally(SeatId(0), find_library("Survivalist"));
        e.state.minions[s.index()].recruited = true;
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        pick(&mut e, bleed(a));
        let survivalist = ally_card(&e, s);
        let lock = use_on(survivalist, 1, None, Target::None);
        if offered {
            pick_when(&mut e, block(z));
            pick_when(&mut e, lock);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(e.state.minions[s.index()].locked);
            assert_eq!(
                [blood(&e, a), blood(&e, z)],
                [4, 4],
                "no hand strike at long range"
            );
        } else {
            pick_when(&mut e, block(z));
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, lock);
        }
    }
}

/// Seat 0's Succubus, a seat-0 vampire beside her, seat 1's vampire and
/// `target_ally` (if any). The Succubus and seat 1's minions.
fn succubus(target_ally: Option<&str>) -> (Engine, MinionId, MinionId, Option<MinionId>) {
    let mut e = game(2);
    let s = e.add_ally(SeatId(0), find_library("Succubus"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = target_ally.map(|n| e.add_ally(SeatId(1), find_library(n)));
    (e, s, z, ally)
}

/// Succubus: "may steal 1 blood or life (gaining a life) from a minion as a
/// +1 stealth Ⓓ action." A 1-life ally: stolen, burned by the Ⓓ action
/// (its Methuselah among the action's burned owners: ANK 20220507), the
/// Succubus at 4 life.
// Rulings: [ANK 20220507].
#[test]
fn succubus_steals_life_and_burns_an_empty_ally() {
    let (mut e, s, _, ally) = succubus(Some("Political Ally"));
    let ally = ally.expect("the ally");
    e.notes = Some(Vec::new());
    let card = ally_card(&e, s);
    pick(&mut e, use_on(card, 0, Some(s), Target::Minion(ally)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play);
    assert!(ash_has(&e, 1, "Political Ally"));
    assert_eq!(blood(&e, s), 4);
    let note = noted_actions(&mut e)
        .into_iter()
        .find(|n| n.action.acting == s && n.ended)
        .expect("the action");
    assert_eq!(note.action.burned_owners, vec![(SeatId(1), Title::None)]);
}

/// Succubus's action at a vampire with no blood: offered, successful,
/// nothing stolen (RTR 20010711).
// Rulings: [RTR 20010711].
#[test]
fn succubus_steals_from_an_empty_vampire() {
    let (mut e, s, z, _) = succubus(None);
    e.state.minions[z.index()].blood = 0;
    let card = ally_card(&e, s);
    pick(&mut e, use_on(card, 0, Some(s), Target::Minion(z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!([blood(&e, s), blood(&e, z)], [3, 0]);
    assert!(e.state.minions[s.index()].locked, "acted");
}

/// Succubus: "Once each turn, she may burn 1 life to get +1 bleed for the
/// current action": during her bleed, before resolution (ANK 20231124, LSJ
/// 19970718), once that turn (ANK 20180719-4): she bleeds 1 for her 1 life,
/// not offered again after resolution.
// Rulings: [ANK 20231124] [LSJ 19970718] [ANK 20180719-4].
#[test]
fn succubus_burns_life_for_bleed_once_each_turn() {
    let (mut e, s, _, _) = succubus(None);
    let card = ally_card(&e, s);
    let boost = use_on(card, 1, None, Target::None);
    pick(&mut e, bleed(s));
    pick_when(&mut e, boost);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, boost);
    assert_eq!(e.state.seats[1].pool, 10 - 1);
    assert_eq!(blood(&e, s), 2);
}

/// Succubus: "Damage from Succubus's hand strikes is aggravated": blocking
/// a vampire with blood, her 1 sends it to torpor.
#[test]
fn succubus_hand_strikes_are_aggravated() {
    let (mut e, a, _) = ally_blocks("Succubus", 3, &[]);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].in_torpor);
}

/// Wolf Companion: "the opposing minion takes 1 damage during strike
/// resolution (if the range is close)": environmental, its source the
/// retainer (ANK 20180612, ANK 20211127), through a dodge (RTR 19941109,
/// TOM 19951216, PIB 20140324, ANK 20200422): the blocker's Side Strike
/// dodges the hand strike, not the wolf's 1.
// Rulings: [ANK 20180612] [ANK 20211127] [RTR 19941109] [TOM 19951216]
// [PIB 20140324] [ANK 20200422].
#[test]
fn wolf_companion_bites_at_close_range_through_a_dodge() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("cel", 0), 4, false);
    in_play(&mut e, 0, "Wolf Companion", Some(a));
    hand(&mut e, 1, &["Side Strike"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 3, "the wolf's 1");
}

/// Wolf Companion [ANI]: "As above, but Wolf Companion has 3 life", the
/// version of the Discipline level used to employ it (RTR 19941109, ANK
/// 20180612): 3 life at [ANI], 2 at [ani].
// Rulings: [RTR 19941109] [ANK 20180612].
#[test]
fn wolf_companion_superior_has_three_life_fixed_at_employ() {
    for (level, mode, life) in [("ANI", 1, 3), ("ani", 0, 2)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with(level, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Wolf Companion"]);
        pick(&mut e, play(0, mode, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let wolf = placed_on(&e, "Wolf Companion", Some(a));
        assert_eq!(e.state.cards[wolf.index()].counters, life, "{level}");
    }
}

/// Seat 0's Young Bloods and a vampire; seat 1's vampires: a locked
/// capacity-7 one with 1 blood, a locked capacity-8 one, an unlocked
/// capacity-4 one. The Young Bloods and the three.
fn young_bloods() -> (Engine, MinionId, [MinionId; 3]) {
    let mut e = game(2);
    let y = e.add_ally(SeatId(0), find_library("Young Bloods"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let seven = e.add_minion(SeatId(1), textless(|c| c.capacity == 7, 0), 1, false);
    let eight = e.add_minion(SeatId(1), textless(|c| c.capacity == 8, 0), 4, false);
    let four = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 4, false);
    for m in [seven, eight] {
        e.state.minions[m.index()].locked = true;
    }
    (e, y, [seven, eight, four])
}

/// Young Bloods: "can burn 2 blood from a locked vampire with a capacity
/// less than 8 as a +1 stealth Ⓓ action": the locked capacity-7 vampire a
/// target, its 1 blood burned (LSJ 20070213); the capacity-8 one and the
/// unlocked one not targets.
// Rulings: [LSJ 20070213].
#[test]
fn young_bloods_burn_two_blood_from_a_locked_young_vampire() {
    let (mut e, y, [seven, eight, four]) = young_bloods();
    let card = ally_card(&e, y);
    let req = request(&mut e);
    for (m, ok) in [(seven, true), (eight, false), (four, false)] {
        let o = use_on(card, 0, Some(y), Target::Minion(m));
        assert_eq!(req.options.contains(&o), ok, "{m:?}");
    }
    pick(&mut e, use_on(card, 0, Some(y), Target::Minion(seven)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, seven), 0);
}

/// Young Bloods' target unlocked by resolution, unblocked: the action
/// fizzles (successful, nothing burned: ANK 20210124, LSJ 20070411, LSJ
/// 20090514).
// Rulings: [ANK 20210124] [LSJ 20070411] [LSJ 20090514].
#[test]
fn young_bloods_fizzle_on_a_target_unlocked_by_resolution() {
    let (mut e, y, [seven, ..]) = young_bloods();
    e.state.minions[seven.index()].blood = 3;
    e.notes = Some(Vec::new());
    let card = ally_card(&e, y);
    pick(&mut e, use_on(card, 0, Some(y), Target::Minion(seven)));
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    e.state.minions[seven.index()].locked = false;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, seven), 3);
    let note = noted_actions(&mut e)
        .into_iter()
        .find(|n| n.action.acting == y && n.ended)
        .expect("the action");
    assert_eq!(note.action.outcome, Some(zvtes::actions::Outcome::Fizzled));
}

/// Young Bloods burned by a strike: "If a vampire controlled by another
/// Methuselah burns the Young Bloods in combat …, he or she gains 2 blood",
/// after the strike's resolution (LSJ 20080212): their 2 sends the
/// 1-blood vampire to torpor (its 1 mended, 1 left), then it gains 2.
// Rulings: [LSJ 20080212].
#[test]
fn young_bloods_burned_by_a_strike_feed_the_vampire_after_resolution() {
    let (mut e, a, y) = ally_blocks("Young Bloods", 1, &[]);
    e.state.minions[a.index()].blood = 1;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[y.index()].in_play);
    let v = &e.state.minions[a.index()];
    assert!(v.in_torpor, "the strike not mended by the gain");
    assert_eq!(v.blood, 2);
}

/// Young Bloods burned as an action: Tier of Souls [ani] by another
/// Methuselah's vampire steals their last life, the action's own effect
/// (advanced rules §5.4.1): the vampire gains the life stolen and 2. Burned
/// by an ally's action (Succubus), nobody gains.
#[test]
fn young_bloods_burned_by_an_action_feed_the_actor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Gangrel", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let y = e.add_ally(SeatId(1), find_library("Young Bloods"));
    e.state.minions[y.index()].blood = 1;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Tier of Souls"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::Minion(y),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[y.index()].in_play);
    assert_eq!(blood(&e, a), 1 + 1 + 2);

    let (mut e, s, _, y) = succubus(Some("Young Bloods"));
    let y = y.expect("the Young Bloods");
    e.state.minions[y.index()].blood = 1;
    let card = ally_card(&e, s);
    pick(&mut e, use_on(card, 0, Some(s), Target::Minion(y)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[y.index()].in_play);
    assert_eq!(blood(&e, s), 4, "the life stolen only");
}

/// Seat 0's `a` (a vampire with `disc`) bleeds; seat 1's Escaped Mental
/// Patient, at 3 life, blocks with `cards` in seat 0's hand: `a`, the
/// patient and his strike.
fn patient_blocks(disc: &str, cards: &[&str]) -> (Engine, MinionId, MinionId, Choice) {
    patient_blocks_in(game(2), disc, cards)
}

/// [`patient_blocks`] in the engine `e`.
fn patient_blocks_in(
    mut e: Engine,
    disc: &str,
    cards: &[&str],
) -> (Engine, MinionId, MinionId, Choice) {
    let a = e.add_minion(SeatId(0), vampire_with(disc, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let p = e.add_ally(SeatId(1), find_library("Escaped Mental Patient"));
    e.state.minions[p.index()].blood = 3;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, cards);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(p));
    let strike = use_on(ally_card(&e, p), 0, None, Target::None);
    (e, a, p, strike)
}

/// Escaped Mental Patient: "This ally can strike: hand strike at +1
/// damage, aggravated; burn him at the end of combat if this strike has
/// resolved." His 2 aggravated send the bleeder to torpor; he burns as the
/// combat ends, an ally burned in combat by his opponent (advanced rules
/// §5.4.1): FBI Special Affairs Division counts him (LSJ 20100527).
// Rulings: [LSJ 20100527].
#[test]
fn escaped_mental_patient_burns_after_his_aggravated_strike_resolves() {
    let (mut e, a, p, strike) = patient_blocks("pot", &[]);
    let fbi = in_play(&mut e, 1, "FBI Special Affairs Division", None);
    pick_when(&mut e, strike);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].in_torpor, "2 aggravated");
    assert!(!e.state.minions[p.index()].in_play);
    assert!(ash_has(&e, 1, "Escaped Mental Patient"));
    assert_eq!(e.state.cards[fbi.index()].counters, 1);
}

/// Escaped Mental Patient's strike prevented in full (Skin of Steel [for]):
/// it resolved all the same, so he burns as the combat ends (advanced
/// rules §4.6.2: "Preventing the damage leaves the strike resolved").
#[test]
fn escaped_mental_patient_burns_though_his_damage_is_prevented() {
    let (mut e, a, p, strike) = patient_blocks("for", &["Skin of Steel"]);
    pick_when(&mut e, strike);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].in_torpor, "all prevented");
    assert!(!e.state.minions[p.index()].in_play);
    assert!(ash_has(&e, 1, "Escaped Mental Patient"));
}

/// Escaped Mental Patient: his strike or a normal hand strike, chosen at
/// strike declaration (ANK 20190717-2); the hand strike does not burn him
/// (LSJ 20001127-2, RTR 20020501: only his own effect).
// Rulings: [ANK 20190717-2] [LSJ 20001127-2] [RTR 20020501].
#[test]
fn escaped_mental_patient_normal_hand_strike_does_not_burn() {
    let (mut e, a, p, strike) = patient_blocks("pot", &[]);
    let req = offering(&mut e, strike);
    assert!(req.options.contains(&Choice::HandStrike));
    e.choose(index_of(&req, Choice::HandStrike));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[p.index()].in_play);
    assert_eq!(blood(&e, a), 4 - 1);
}

/// Escaped Mental Patient whose strike never resolves: the opposing
/// "strike: combat ends" (Force of Personality [pre]): he does not burn.
// Rulings: [LSJ 20001127-2] [LSJ 20010627] [LSJ 20070508].
#[test]
fn escaped_mental_patient_does_not_burn_when_combat_ends_before_his_strike() {
    let (mut e, a, p, strike) = patient_blocks("pre", &["Force of Personality"]);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, strike);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[p.index()].in_play);
    assert!(!e.state.minions[a.index()].in_torpor, "no damage");
}

/// Escaped Mental Patient's strike dodged (Side Strike [cel]) or of no
/// effect on the vampire (Eyes of Blades [AUS][CEL]): it resolved, without
/// damage, so he burns as the combat ends (owner, rules-qa 122.62).
#[test]
fn escaped_mental_patient_burns_though_his_strike_is_dodged() {
    for (levels, card, mode) in [
        (&["cel"][..], "Side Strike", 0),
        (&["aus", "aus", "cel", "cel"][..], "Eyes of Blades", 1),
    ] {
        let (mut e, a, p, strike) = patient_blocks("pot", &[card]);
        e.state.minions[a.index()].gained_levels = levels.to_vec();
        pick_when(&mut e, play(0, mode, a));
        pick_when(&mut e, strike);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.minions[p.index()].in_play, "{card}");
        assert!(ash_has(&e, 1, "Escaped Mental Patient"), "{card}");
        assert!(ash_has(&e, 0, card), "{card}: played");
        assert_eq!(blood(&e, a), 4, "{card}: no damage");
    }
}

/// Escaped Mental Patient against Gemini's Mirror [OBF]: tails, his strike
/// has no effect on the vampire, heads, it lands (2 aggravated: torpor);
/// both resolved, so he burns either way (owner, rules-qa 122.62). Over
/// seeds, both happen.
#[test]
fn escaped_mental_patient_burns_though_a_geminis_mirror_voids_his_strike() {
    let mut seen = [false; 2];
    for seed in 0..40 {
        let mut e = Engine::bare_seeded(seed, 2);
        for s in &mut e.state.seats {
            s.pool = 10;
        }
        e.state.turn = 4;
        let (mut e, a, p, strike) = patient_blocks_in(e, "pot", &["Gemini's Mirror"]);
        e.state.minions[a.index()].gained_levels = vec!["obf", "obf"];
        pick_when(&mut e, play(0, 1, a));
        pick_when(&mut e, strike);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.minions[p.index()].in_play, "seed {seed}");
        seen[usize::from(e.state.minions[a.index()].in_torpor)] = true;
    }
    assert_eq!(seen, [true, true]);
}

/// Escaped Mental Patient whose strike resolved: Left for Dead, in his
/// Methuselah's hand out of turn, is never offered for his burn as the
/// combat ends (LSJ 20070507, LSJ 20021205; advanced rules §5.4.3).
// Rulings: [LSJ 20070507] [LSJ 20021205].
#[test]
fn escaped_mental_patient_cannot_be_left_for_dead() {
    let (mut e, _, p, strike) = patient_blocks("pot", &[]);
    hand(&mut e, 1, &["Left for Dead"]);
    pick_when(&mut e, strike);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        master_on(0, Target::Minion(p)),
    );
    assert!(!e.state.minions[p.index()].in_play);
}

/// Seat 0's vampire bleeds seat 1 (library: Deflections under a Wake with
/// Evening's Freshness), `couriers` of seat 0 in play, recruited this turn,
/// the first `locked` ones locked. The Couriers.
fn couriers_on_a_bleed(couriers: usize, locked: usize) -> (Engine, Vec<MinionId>) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let cs: Vec<MinionId> = (0..couriers)
        .map(|i| {
            let c = e.add_ally(SeatId(0), find_library("Courier"));
            e.state.minions[c.index()].recruited = true;
            e.state.minions[c.index()].locked = i < locked;
            c
        })
        .collect();
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    fill_hand(&mut e, 1);
    for name in ["Deflection", "Deflection", "Wake with Evening's Freshness"] {
        e.state.seats[1].library.push(find_library(name));
    }
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(a));
    (e, cs)
}

/// Courier: "When a minion you control successfully bleeds, you may look at
/// the top card of the library of the Methuselah being bled, and you may
/// lock the Courier to discard that card", the turn he is recruited too
/// (ANK 20180517): after the bleed, its controller sees Wake with
/// Evening's Freshness, the Courier locks, the card in the ash heap.
// Rulings: [ANK 20180517].
#[test]
fn courier_looks_and_locks_to_discard_the_bled_library_top() {
    let (mut e, cs) = couriers_on_a_bleed(1, 0);
    let card = ally_card(&e, cs[0]);
    pick_when(&mut e, use_on(card, 0, None, Target::None));
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    let wake = find_library("Wake with Evening's Freshness");
    let looks = noted_looks(&mut e);
    assert!(looks
        .iter()
        .any(|l| l.viewer == SeatId(0) && l.seat == SeatId(1) && l.cards == [wake]));
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::ResolveRider { card: card.0 }]
    );
    e.choose(1);
    request(&mut e);
    assert!(e.state.minions[cs[0].index()].locked);
    assert!(ash_has(&e, 1, "Wake with Evening's Freshness"));
    assert_eq!(e.state.seats[1].library.len(), 2);
}

/// Two Couriers, one locked: each looks once this bleed (LSJ 20001102-2),
/// the locked one too, never to discard (LSJ 20001102: needs not be
/// unlocked to look); the unlocked one discards, the next card then on
/// top (in succession, LSJ 20001102).
// Rulings: [LSJ 20001102] [LSJ 20001102-2].
#[test]
fn couriers_look_even_locked_and_each_once_a_bleed() {
    let (mut e, cs) = couriers_on_a_bleed(2, 1);
    let (locked, free) = (ally_card(&e, cs[0]), ally_card(&e, cs[1]));
    let look = |c: CardId| use_on(c, 0, None, Target::None);
    let req = offering(&mut e, look(locked));
    assert!(req.options.contains(&look(free)));
    e.choose(index_of(&req, look(locked)));
    let req = request(&mut e);
    assert!(
        !req.options
            .contains(&Choice::ResolveRider { card: locked.0 }),
        "no discard for the locked Courier"
    );
    assert!(!req.options.contains(&look(locked)), "once a bleed");
    e.choose(index_of(&req, look(free)));
    pick(&mut e, Choice::ResolveRider { card: free.0 });
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        *o == look(locked) || *o == look(free)
    });
    assert_eq!(noted_looks(&mut e).len(), 2);
    assert!(ash_has(&e, 1, "Wake with Evening's Freshness"));
    assert_eq!(e.state.seats[1].library.len(), 2);
}

/// Courier: "gets -1 intercept when attempting to block a vampire": a
/// vampire's 0-stealth bleed cannot be reached.
#[test]
fn courier_blocks_vampires_at_minus_one_intercept() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let c = e.add_ally(SeatId(1), find_library("Courier"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(a));
    never_reaching_until(&mut e, SeatId(0), WindowLabel::MinionPhase, c);
}

/// Seat 0's Dauntain Black Magician and a torpid Toreador; seat 1's vampire
/// bearing a Potence master card and a Tremere antitribu. The Magician,
/// his card, the torpid vampire, the bearer, the Potence card and the
/// Tremere antitribu.
#[allow(clippy::type_complexity)]
fn dauntain() -> (
    Engine,
    MinionId,
    CardId,
    MinionId,
    MinionId,
    CardId,
    MinionId,
) {
    let mut e = game(2);
    let d = e.add_ally(SeatId(0), find_library("Dauntain Black Magician"));
    // Toreador: no superior Potence to bar the card (rules-qa 81.1).
    let t = e.add_minion(SeatId(0), plain("Toreador", 0), 2, true);
    e.add_minion(SeatId(0), plain("Toreador", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let potence = in_play(&mut e, 1, "Potence", Some(z));
    let ta = e.add_minion(
        SeatId(1),
        textless(|c| c.clan == "Tremere antitribu", 0),
        4,
        false,
    );
    let card = ally_card(&e, d);
    (e, d, card, t, z, potence, ta)
}

/// Dauntain Black Magician: "can move a master discipline card from any
/// vampire to a vampire you control as a Ⓓ action": the Potence on the
/// prey's vampire moves to his Methuselah's torpid vampire, chosen as the
/// action is announced (the card itself, its level with it).
#[test]
fn dauntain_moves_a_discipline_master_to_a_torpid_vampire_of_yours() {
    let (mut e, d, card, t, z, potence, _) = dauntain();
    assert_eq!(e.state.discipline_level(z, "pot"), 1);
    pick(&mut e, use_on(card, 0, Some(d), Target::Card(potence)));
    pick(&mut e, Choice::ChooseMinion(t));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(placed_on(&e, "Potence", Some(t)), potence);
    assert_eq!(e.state.discipline_level(z, "pot"), 0);
    assert!(e.state.discipline_level(t, "pot") >= 1);
}

/// Dauntain Black Magician's move keeps the master's "Cannot be put on a
/// vampire with superior Potence" (rules-qa 81.1, 122.48): his torpid
/// vampire with superior Potence is never offered as the receiver; the
/// card goes to his other vampire.
#[test]
fn dauntain_moves_no_master_onto_a_superior_vampire() {
    let (mut e, d, card, t, _, potence, _) = dauntain();
    e.state.minions[t.index()].gained_levels = vec!["pot", "pot"];
    pick(&mut e, use_on(card, 0, Some(d), Target::Card(potence)));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::ChooseMinion(t),
    );
    let on = e.state.cards[potence.index()].host.expect("moved");
    assert_ne!(on, t);
    assert_eq!(e.state.minions[on.index()].owner, SeatId(0));
}

/// Dauntain Black Magician: "can burn himself and a Tremere antitribu
/// controlled by your predator as a Ⓓ action": successful, both in their
/// ash heaps; blocked by the Tremere antitribu, neither burned.
#[test]
fn dauntain_burns_himself_and_a_predators_tremere_antitribu() {
    let (mut e, d, card, _, _, _, ta) = dauntain();
    pick(&mut e, use_on(card, 1, Some(d), Target::Minion(ta)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[d.index()].in_play);
    assert!(!e.state.minions[ta.index()].in_play);
    assert!(ash_has(&e, 0, "Dauntain Black Magician"));

    let (mut e, d, card, _, _, _, ta) = dauntain();
    pick(&mut e, use_on(card, 1, Some(d), Target::Minion(ta)));
    pick_when(&mut e, block(ta));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[d.index()].in_play, "blocked");
    assert!(e.state.minions[ta.index()].in_play);
}

/// Dauntain Black Magician's two actions are not cross-restrictive: after
/// the move, unlocked, he may burn the Tremere antitribu the same turn
/// (LSJ 20100909).
// Rulings: [LSJ 20100909].
#[test]
fn dauntain_two_actions_do_not_restrict_each_other() {
    let (mut e, d, card, t, _, potence, ta) = dauntain();
    pick(&mut e, use_on(card, 0, Some(d), Target::Card(potence)));
    pick(&mut e, Choice::ChooseMinion(t));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[d.index()].locked = false;
    let req = request(&mut e);
    assert!(req
        .options
        .contains(&use_on(card, 1, Some(d), Target::Minion(ta))));
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::UseAbility { ability: 0, actor: Some(m), .. } if *m == d)));
}

/// Hexaped: "may give a link counter to any Methuselah as a +1 stealth Ⓓ
/// action. Remove Hexaped from the game if that action succeeds. The
/// Tremere antitribu who recruited it gets +1 bleed and +1 stealth when
/// bleeding the Methuselah with that link counter." Recruited by a Tremere
/// antitribu, its action at the prey succeeds: the Hexaped removed from
/// the game, the counter on the prey; the recruiter's bleed there is at 1
/// stealth for 2, another vampire's at 0 for 1.
#[test]
fn hexaped_links_the_prey_for_its_recruiter() {
    let mut e = game(2);
    let r = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Tremere antitribu", 0),
        4,
        false,
    );
    let w = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    fill_hand(&mut e, 1);
    hand(&mut e, 0, &["Hexaped"]);
    pick(&mut e, play(0, 0, r));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let h = MinionId(e.state.minions.len() as u32 - 1);
    let card = ally_card(&e, h);
    assert_eq!(e.state.cards[card.index()].by, Some(r));
    e.state.minions[h.index()].recruited = false;
    pick(&mut e, use_on(card, 0, Some(h), Target::Seat(SeatId(1))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[h.index()].in_play);
    assert!(e.state.seats[0].removed.contains(&find_library("Hexaped")));
    assert!(!ash_has(&e, 0, "Hexaped"));
    assert_eq!(e.state.seats[1].links.0, vec![r]);
    e.state.minions[r.index()].locked = false;
    pick(&mut e, bleed(r));
    assert_eq!(e.action().expect("the bleed").stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
    pick(&mut e, bleed(w));
    assert_eq!(e.action().expect("the bleed").stealth(&e.state), 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2 - 1);
}

/// Hexaped's bonus "when bleeding the Methuselah with that link counter",
/// read on the bleed's current target (red reading R3, D13): the linked
/// prey deflects the bleed to seat 2, which has no counter — seat 2 loses
/// 1, not 2.
#[test]
fn hexaped_link_bonus_drops_with_a_redirected_bleed() {
    let mut e = game(3);
    let r = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Tremere antitribu", 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let d = e.add_minion(SeatId(1), vampire_with("dom", 0), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    e.state.seats[1].links.0 = vec![r];
    hand(&mut e, 1, &["Deflection"]);
    pick(&mut e, bleed(r));
    assert_eq!(e.action().expect("the bleed").stealth(&e.state), 1);
    pick_when(&mut e, play_on(0, 0, d, Target::Seat(SeatId(2))));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!((e.state.seats[1].pool, e.state.seats[2].pool), (10, 10 - 1));
}

/// A Hexaped put in play, not recruited (a fixture's, as Pressing Flesh's):
/// no recruiter, its action gives a counter naming no one (rules-digest
/// §3.7), the Hexaped still removed.
#[test]
fn hexaped_put_in_play_links_for_no_one() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let h = e.add_ally(SeatId(0), find_library("Hexaped"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = ally_card(&e, h);
    pick(&mut e, use_on(card, 0, Some(h), Target::Seat(SeatId(1))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[h.index()].in_play);
    assert!(e.state.seats[1].links.0.is_empty());
}

/// Two link counters naming one recruiter (two Hexapeds it recruited) on
/// one Methuselah: +2 stealth and +2 bleed, one each (a red reading, rules-qa
/// Batch 122).
#[test]
fn hexaped_links_stack_per_counter() {
    let mut e = game(2);
    let r = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Tremere antitribu", 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1].links.0 = vec![r, r];
    pick(&mut e, bleed(r));
    assert_eq!(e.action().expect("the bleed").stealth(&e.state), 2);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 3);
}

/// Corrupt Construction: "[vic] After this ally enters play, you can
/// remove from the game any number of ally, retainer or vampire cards from
/// your ash heap or hand; this ally gains 1 life for each card removed this
/// way. This ally gets +1 strength for each life it has." Recruited: the
/// heap's ally, retainer and vampire and the hand's ally offered, not the
/// heap's master nor the hand's action; three removed, then stopped: 4
/// life, the cards out of the game.
#[test]
fn corrupt_construction_removes_minion_cards_for_life() {
    use zvtes::decision::Zone;
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("vic", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    fill_hand(&mut e, 1);
    let [ally, retainer, master] = ["Political Ally", "Dread Mastiff", "Potence"].map(find_library);
    let vamp = plain("Brujah", 2);
    e.state.seats[0].ash = vec![ally, retainer, master].into();
    e.state.seats[0].ash_crypt = vec![vamp].into();
    hand(
        &mut e,
        0,
        &["Corrupt Construction", "Courier", "Entrancement"],
    );
    pick(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::EntryEffect);
    let ash = |card| Choice::TakeFrom {
        zone: Zone::AshHeap,
        card,
    };
    let courier = Choice::TakeFrom {
        zone: Zone::Hand,
        card: find_library("Courier"),
    };
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            ash(ally),
            ash(retainer),
            Choice::AshVampire(vamp),
            courier
        ]
    );
    for c in [ash(ally), Choice::AshVampire(vamp), courier] {
        pick(&mut e, c);
    }
    pick(&mut e, Choice::Pass);
    request(&mut e);
    let cc = MinionId(e.state.minions.len() as u32 - 1);
    assert_eq!(blood(&e, cc), 1 + 3);
    let s = &e.state.seats[0];
    assert!(s.removed.contains(&ally) && s.removed.contains(&find_library("Courier")));
    assert_eq!(s.removed_crypt, vec![vamp]);
    assert!(s.ash.contains(&retainer) && s.ash.contains(&master));
}

/// Corrupt Construction [VIC]: 2 life with nothing to remove; "During your
/// unlock phase, this ally burns 1 life": 1 at its next unlock phase,
/// burned at the one after.
#[test]
fn corrupt_construction_superior_has_two_life_and_wanes() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("VIC", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Corrupt Construction"]);
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let cc = MinionId(e.state.minions.len() as u32 - 1);
    assert_eq!(blood(&e, cc), 2);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, cc), 1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[cc.index()].in_play);
    assert!(ash_has(&e, 0, "Corrupt Construction"));
}

/// Corrupt Construction raised by Pressing Flesh: its entry offered again
/// (ANK 20170309-2), the card removed, no life gained (the zombie "cannot
/// gain life").
// Rulings: [ANK 20170309-2].
#[test]
fn pressing_flesh_raises_corrupt_construction_with_its_removal() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("thn", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let vamp = plain("Brujah", 1);
    hand(&mut e, 0, &["Pressing Flesh"]);
    let cc = e.add_ally(SeatId(1), find_library("Corrupt Construction"));
    e.burn(cc);
    assert!(ash_has(&e, 1, "Corrupt Construction"));
    e.state.seats[0].ash_crypt = vec![vamp].into();
    pick(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::EntryEffect);
    assert_eq!(req.options, vec![Choice::Pass, Choice::AshVampire(vamp)]);
    pick(&mut e, Choice::AshVampire(vamp));
    request(&mut e);
    assert_eq!(e.state.seats[0].removed_crypt, vec![vamp]);
    let raised = MinionId(e.state.minions.len() as u32 - 1);
    assert_eq!(blood(&e, raised), 1, "its starting life, no gain");
}

/// Third Edition's ghouls are monsters, not mortals (ANK 20200203-2, LSJ
/// 20060515): Corrupt Construction and Rafastio Ghoul; a Courier is.
/// Animal and ghoul retainers are no mortals either: Ritual Scalpel's "a
/// mortal (ally or retainer)" cuts Malajit Chandramouli, a mortal
/// retainer, never one of them.
// Rulings: [ANK 20200203-2] [LSJ 20060515].
#[test]
fn third_edition_ghouls_are_no_mortals() {
    let mut e = game(2);
    for (name, mortal) in [
        ("Corrupt Construction", false),
        ("Rafastio Ghoul", false),
        ("Courier", true),
    ] {
        let m = e.add_ally(SeatId(0), find_library(name));
        assert_eq!(e.state.is_mortal(m), mortal, "{name}");
    }
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Le Dinh Tho"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let scalpel = in_play(&mut e, 0, "Ritual Scalpel", Some(n));
    let cut = |r: CardId| Choice::UseAbility {
        source: AbilitySource::Card(scalpel),
        ability: 1,
        actor: Some(n),
        target: Target::Card(r),
    };
    let malajit = in_play(&mut e, 1, "Malajit Chandramouli", Some(x));
    let others: Vec<(&str, CardId)> = [
        "Dog Pack",
        "Ilomba",
        "Murder of Crows",
        "Owl Companion",
        "Raptor",
        "Raven Spy",
        "Wolf Companion",
        "Feral Hound",
        "Ghoul Retainer",
        "Marijava Ghoul",
        "Revenant",
    ]
    .into_iter()
    .map(|name| (name, in_play(&mut e, 1, name, Some(x))))
    .collect();
    let req = request(&mut e);
    assert!(req.options.contains(&cut(malajit)), "{req:?}");
    for (name, r) in others {
        assert!(!req.options.contains(&cut(r)), "{name}");
    }
}

/// Seat 0's vampire of `clan` with an Infernal Familiar (none, one or two)
/// and `cards` in hand; seat 1's vampire. The vampire and the Familiars.
fn familiar(clan: &str, n: usize, cards: &[&str]) -> (Engine, MinionId, Vec<CardId>) {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), plain(clan, 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    fill_hand(&mut e, 1);
    let fs = (0..n)
        .map(|_| in_play(&mut e, 0, "Infernal Familiar", Some(v)))
        .collect();
    hand(&mut e, 0, cards);
    (e, v, fs)
}

/// Infernal Familiar: "The vampire with this retainer can play a card that
/// requires a Discipline he or she does not have as if he or she had the
/// basic level of that Discipline; if he or she does so, place an
/// investment counter on the Infernal Familiar. If the number of investment
/// counters on the Familiar is greater than this vampire's capacity, burn
/// this vampire." A Brujah plays Govern the Unaligned [dom] (not [DOM]): a
/// counter; with as many counters as his capacity, the next play burns him.
#[test]
fn infernal_familiar_plays_a_missing_discipline_for_a_counter() {
    let (mut e, v, fs) = familiar("Brujah", 1, &["Govern the Unaligned"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, v)));
    assert!(!req.options.contains(&play(0, 1, v)), "basic only");
    pick(&mut e, play(0, 0, v));
    request(&mut e);
    assert_eq!(e.state.cards[fs[0].index()].counters, 1);

    let (mut e, v, fs) = familiar("Brujah", 1, &["Govern the Unaligned"]);
    let cap = e.state.capacity_of(v).expect("a vampire");
    e.state.cards[fs[0].index()].counters = cap;
    pick(&mut e, play(0, 0, v));
    request(&mut e);
    assert!(!e.state.minions[v.index()].in_play, "burned");
}

/// Infernal Familiar on an ally: "The vampire with this retainer" reads a
/// vampire only (D16): the ally is not offered Govern the Unaligned [dom],
/// no counter.
#[test]
fn infernal_familiar_gives_an_ally_nothing() {
    let (mut e, v, _) = familiar("Brujah", 0, &["Govern the Unaligned"]);
    let ally = e.add_ally(SeatId(0), find_library("Political Ally"));
    let f = in_play(&mut e, 0, "Infernal Familiar", Some(ally));
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play(0, 0, ally)),
        "{:?}",
        req.options
    );
    assert!(
        !req.options.contains(&play(0, 0, v)),
        "no Familiar, no [dom]"
    );
    assert_eq!(e.state.cards[f.index()].counters, 0);
}

/// Infernal Familiar: "A counter is added if the card is canceled as it is
/// played (eg. Direct Intervention)" (LSJ 20000424).
// Rulings: [LSJ 20000424].
#[test]
fn infernal_familiar_counts_a_canceled_play() {
    let (mut e, v, fs) = familiar("Brujah", 1, &["Govern the Unaligned"]);
    e.state.seats[1].hand.take(0);
    hand(&mut e, 1, &["Direct Intervention"]);
    let di = e.state.seats[1].hand.len() as u8 - 1;
    pick(&mut e, play(0, 0, v));
    let req = offering(&mut e, master_on(di, Target::None));
    assert_eq!(req.seat, SeatId(1));
    e.choose(index_of(&req, master_on(di, Target::None)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Direct Intervention"));
    assert!(e.action().is_none());
    assert_eq!(e.state.seats[1].pool, 10 - 1, "its own cost, no bleed");
    assert_eq!(e.state.cards[fs[0].index()].counters, 1);
}

/// Two Infernal Familiars: Show of Force [pot][pre] (a combo card) played
/// by a Toreador missing only Potence, one counter in all (RTR 20020501,
/// LSJ 20100517: none from the other); a Tremere missing both cannot play
/// it, two grants never meeting two requirements (LSJ 20011217, LSJ
/// 20090508, LSJ 20020218). Its player chooses which Familiar takes the
/// counter (owner, rules-qa 122.85), the one with more counters too; two
/// alike (as many counters) ask nothing.
// Rulings: [RTR 20020501] [LSJ 20100517] [LSJ 20011217] [LSJ 20090508]
// [LSJ 20020218].
#[test]
fn two_infernal_familiars_add_one_counter_and_meet_one_requirement() {
    let (mut e, t, _) = familiar("Tremere", 2, &["Show of Force"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, t)), "two missing");

    let (mut e, v, fs) = familiar("Toreador", 2, &["Show of Force"]);
    e.state.cards[fs[0].index()].counters = 1;
    pick(&mut e, play(0, 0, v));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::PlayTerms);
    assert_eq!(
        req.options,
        vec![Choice::ChooseCard(fs[0].0), Choice::ChooseCard(fs[1].0)]
    );
    e.choose(index_of(&req, Choice::ChooseCard(fs[0].0)));
    request(&mut e);
    let counters: Vec<u8> = fs
        .iter()
        .map(|f| e.state.cards[f.index()].counters)
        .collect();
    assert_eq!(counters, vec![2, 0], "on the one chosen");

    let (mut e, v, fs) = familiar("Toreador", 2, &["Show of Force"]);
    pick(&mut e, play(0, 0, v));
    let req = request(&mut e);
    assert_ne!(req.window, WindowLabel::PlayTerms, "alike: no choice");
    let counters: u8 = fs.iter().map(|f| e.state.cards[f.index()].counters).sum();
    assert_eq!(counters, 1);
}

/// Infernal Familiar: its vampire does not have the Discipline (LSJ
/// 19980303), before and after a play through it.
// Rulings: [LSJ 19980303].
#[test]
fn infernal_familiar_gives_no_discipline() {
    let (mut e, v, _) = familiar("Brujah", 1, &["Govern the Unaligned"]);
    pick(&mut e, play(0, 0, v));
    request(&mut e);
    assert!(!zvtes::cards::holds_for_host(
        zvtes::cards::Requirement::HostDiscipline("dom"),
        &e.state,
        v
    ));
}

/// Infernal Familiar's burn read live: counters equal to a capacity a card
/// raises by 1; that card burned, the vampire burns.
#[test]
fn infernal_familiar_burns_its_vampire_when_capacity_falls() {
    let (mut e, v, fs) = familiar("Brujah", 1, &[]);
    let raiser = in_play(&mut e, 0, "Potence", Some(v));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::HostCapacity(1),
        source: raiser,
    });
    let cap = e.state.capacity_of(v).expect("a vampire");
    e.state.cards[fs[0].index()].counters = cap;
    request(&mut e);
    assert!(e.state.minions[v.index()].in_play);
    e.burn_card(raiser);
    request(&mut e);
    assert!(!e.state.minions[v.index()].in_play, "burned");
}

/// Infernal Familiar meeting Animalism for Pack Alpha [ani] meets it for
/// the Raven Spy [ani] the card employs too (LSJ 20080205-2, LSJ
/// 20060417): one counter.
// Rulings: [LSJ 20080205-2] [LSJ 20060417].
#[test]
fn infernal_familiar_meets_animalism_for_pack_alpha_and_its_raven_spy() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Tremere", 0), 4, false);
    let f = in_play(&mut e, 1, "Infernal Familiar", Some(t));
    hand(&mut e, 1, &["Pack Alpha", "Raven Spy"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(t));
    pick_when(&mut e, play(0, 0, t));
    request(&mut e);
    let spy = find_library("Raven Spy");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == spy && c.present() && c.host == Some(t)));
    assert_eq!(e.state.cards[f.index()].counters, 1);
}

/// Seat 1's Shadow Court Satyr enters play with `stored` in hand, puts it
/// on him (Pass offered: "you can"), at 10 life; seat 2 holds Direct
/// Intervention; seat 0's Brujah (`cards` in hand) bleeds and he blocks. The
/// Brujah, the Satyr, his card and the stored card's index among the
/// cards seat 1 may play (after the hand).
fn satyr_blocks(stored: &str, cards: &[&str]) -> (Engine, MinionId, MinionId, CardId, u8) {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &[stored]);
    hand(&mut e, 2, &["Direct Intervention"]);
    hand(&mut e, 0, cards);
    let s = e.add_ally(SeatId(1), find_library("Shadow Court Satyr"));
    let card = ally_card(&e, s);
    let req = expect(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    let c = find_library(stored);
    assert_eq!(req.options, vec![Choice::Pass, Choice::FindCard(c)]);
    e.choose(1);
    request(&mut e);
    assert_eq!(e.state.cards[card.index()].stored.to_vec(), vec![c]);
    e.state.minions[s.index()].blood = 10;
    // Out of combat, never his to use (LSJ 20060502).
    assert!(!e
        .virtual_hand(SeatId(1))
        .iter()
        .any(|&(_, f)| f == Some(card)));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(s));
    // Seat 1's hand is empty: the stored card first.
    let at = e.state.seats[1].hand.len() as u8;
    (e, a, s, card, at)
}

/// Shadow Court Satyr: "When the Satyr enters play, you can put a combat
/// card from your hand on him. Once each combat, the Satyr can use the
/// ability of that card as if played from your hand." Roundhouse on him:
/// in combat, its [pot] offered, not its [POT] (ANK 20180111); used: a hand
/// strike at +2, no Direct Intervention offered (LSJ 20010716, ANK
/// 20221011), the card still on him, never in play (ANK 20190629) nor in
/// the ash heap; pressed into a second round, not offered again.
// Rulings: [LSJ 20021210-2] [LSJ 20060502] [LSJ 20010716] [ANK 20221011]
// [ANK 20190629] [ANK 20180111].
#[test]
fn shadow_court_satyr_uses_its_stored_combat_card_once_each_combat() {
    let (mut e, a, s, card, at) = satyr_blocks("Roundhouse", &[]);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::HostCombatPress(1),
        source: card,
    });
    let req = offering(&mut e, play(at, 0, s));
    assert!(!req.options.contains(&play(at, 1, s)), "basic only");
    e.choose(index_of(&req, play(at, 0, s)));
    let di = master_on(0, Target::None);
    never_offered_until(&mut e, SeatId(1), WindowLabel::CombatPress, di);
    pick(&mut e, Choice::UsePress);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(at, 0, s));
    assert!(!ash_has(&e, 2, "Direct Intervention"));
    assert_eq!(blood(&e, a), 4 - 3 - 1, "+2, then a plain hand strike");
    let roundhouse = find_library("Roundhouse");
    assert_eq!(
        e.state.cards[card.index()].stored.to_vec(),
        vec![roundhouse]
    );
    assert!(!ash_has(&e, 1, "Roundhouse"));
    assert!(!e
        .state
        .cards
        .iter()
        .any(|c| c.card == roundhouse && c.in_play));
}

/// Target Vitals on the Satyr: no card cancels his use as played, its own
/// text does — the opposing Methuselah may discard two combat cards to
/// cancel it (LSJ 20010716, ANK 20221011).
// Rulings: [LSJ 20010716] [ANK 20221011].
#[test]
fn shadow_court_satyr_stored_target_vitals_still_cancelable() {
    let (mut e, _, s, _, at) = satyr_blocks("Target Vitals", &["Roundhouse", "Roundhouse"]);
    pick_when(&mut e, play(at, 0, s));
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(0));
    assert!(
        req.options.contains(&Choice::PayToCancel),
        "{:?}",
        req.options
    );
}

/// Weighted Walking Stick on the Satyr, used: it goes on him as the card
/// itself, out of his store, no longer his to use (LSJ 20010716).
// Rulings: [LSJ 20010716].
#[test]
fn shadow_court_satyr_stored_walking_stick_leaves_the_storage() {
    let (mut e, _, s, card, at) = satyr_blocks("Weighted Walking Stick", &[]);
    pick_when(&mut e, play(at, 0, s));
    request(&mut e);
    placed_on(&e, "Weighted Walking Stick", Some(s));
    assert!(e.state.cards[card.index()].stored.is_empty());
}

/// Taste of Vitae on the Satyr: no Discipline, used "as a vampire" all the
/// same (LSJ 20080717): offered at the end of the round.
// Rulings: [LSJ 20080717].
#[test]
fn shadow_court_satyr_uses_a_card_without_discipline() {
    let (mut e, _, s, _, at) = satyr_blocks("Taste of Vitae", &[]);
    offering(&mut e, play(at, 0, s));
}

/// Amaranth on the Satyr: an ally cannot commit diablerie (RTR 19970630,
/// LSJ 20080717): his Roundhouse-less hand strikes send the Brujah to
/// torpor, Amaranth never offered.
// Rulings: [RTR 19970630] [LSJ 20080717].
#[test]
fn shadow_court_satyr_cannot_diablerize_with_amaranth() {
    let (mut e, a, s, _, at) = satyr_blocks("Amaranth", &[]);
    e.state.minions[s.index()].kind = match e.state.minions[s.index()].kind {
        zvtes::state::MinionKind::Ally { card, id, mut body } => {
            body.strength = 5;
            zvtes::state::MinionKind::Ally { card, id, body }
        }
        k => k,
    };
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(at, 0, s));
    assert!(e.state.minions[a.index()].in_torpor);
}

/// Shadow Court Satyr raised by Pressing Flesh: his entry offered again
/// (ANK 20170309-2).
// Rulings: [ANK 20170309-2].
#[test]
fn pressing_flesh_raises_a_satyr_with_its_store() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("thn", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let s = e.add_ally(SeatId(1), find_library("Shadow Court Satyr"));
    e.burn(s);
    hand(&mut e, 0, &["Pressing Flesh", "Roundhouse"]);
    pick(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::FindCard(find_library("Roundhouse"))]
    );
}

/// Body Flare [pro] (2 blood) on the Satyr at 2 life: his last life paid,
/// he burns and the combat ends before strike resolution (ANK 20180928-1):
/// the Brujah takes nothing.
// Rulings: [ANK 20180928-1].
#[test]
fn shadow_court_satyr_paying_its_last_life_for_a_strike_ends_combat() {
    let (mut e, a, s, _, at) = satyr_blocks("Body Flare", &[]);
    e.state.minions[s.index()].blood = 2;
    pick_when(&mut e, play(at, 0, s));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[s.index()].in_play);
    assert_eq!(blood(&e, a), 4);
    assert!(!e.state.minions[a.index()].in_torpor);
}

/// Burst of Sunlight [tha] on the Satyr: its aggravated damage to him a
/// life lost (LSJ 20060409): 10 life, less its own 1 and the hand
/// strike's 1.
// Rulings: [LSJ 20060409].
#[test]
fn shadow_court_satyr_takes_burst_of_sunlights_damage_as_life() {
    let (mut e, _, s, _, at) = satyr_blocks("Burst of Sunlight", &[]);
    pick_when(&mut e, play(at, 0, s));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[s.index()];
    assert!(v.in_play);
    assert_eq!(v.blood, 10 - 1 - 1);
}

/// Sanguine Entrapment [cel][qui] (a combo card) on the Satyr: never
/// usable, his use the basic level of one Discipline (advanced rules §5.5;
/// LSJ 20011217, LSJ 20020218).
// Rulings: [LSJ 20011217] [LSJ 20090508] [LSJ 20020218].
#[test]
fn shadow_court_satyr_cannot_use_a_combo_card() {
    let (mut e, _, s, _, at) = satyr_blocks("Sanguine Entrapment", &[]);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(at, 0, s));
}

/// Pack Alpha [ani] on the Satyr, used as a vampire with Animalism: the
/// Raven Spy [ani] it employs from his Methuselah's hand meets it too (LSJ
/// 20080205-2, LSJ 20060417), on the Satyr.
// Rulings: [LSJ 20080205-2] [LSJ 20060417].
#[test]
fn shadow_court_satyr_meets_animalism_for_pack_alpha_and_its_raven_spy() {
    let (mut e, _, s, _, at) = satyr_blocks("Pack Alpha", &[]);
    hand(&mut e, 1, &["Raven Spy"]);
    pick_when(&mut e, play(at + 1, 0, s));
    request(&mut e);
    let spy = find_library("Raven Spy");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == spy && c.present() && c.host == Some(s)));
}

/// Allies playing cards as vampires meet one Discipline of a combo card at
/// most (LSJ 20011217, LSJ 20090508), however many grants (LSJ 20020218):
/// Talaq and Rafastio Ghoul never play Hunter's Mark [cel][tha]; Talaq with
/// a [for] grant beside his own (Pressing Flesh's) never plays Blood
/// Tempering [for][qui].
// Rulings: [LSJ 20011217] [LSJ 20090508] [LSJ 20020218].
#[test]
fn allies_with_grants_never_play_a_combo_card_by_two() {
    for name in ["Talaq, The Immortal", "Rafastio Ghoul"] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let t = e.add_ally(SeatId(0), find_library(name));
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(
            &mut e,
            0,
            &["Hunter's Mark", "Blood Tempering", "Burst of Sunlight"],
        );
        if name.starts_with("Talaq") {
            let card = ally_card(&e, t);
            e.state.layers.push(zvtes::cards::Layer {
                scope: zvtes::cards::Scope::InPlay,
                modifier: zvtes::cards::Modifier::PlaysAsVampire(zvtes::cards::AsVampire {
                    disciplines: &["for"],
                    capacity: 1,
                    non_action: false,
                    non_ally: false,
                    sect: Sect::Independent,
                    strike_only: false,
                    once_per_combat: false,
                    combat_only: false,
                }),
                source: card,
            });
        }
        let req = request(&mut e);
        let by = |o: &Choice, k: u8| matches!(o, Choice::Play { card, actor: Some(m), .. } if *card == k && *m == t);
        assert!(!req.options.iter().any(|o| by(o, 0) || by(o, 1)), "{name}");
    }
}

/// Escaped Mental Patient's aggravated strike answered by Rötschreck (his
/// own Methuselah's, out of turn, on the bleeder): the combat ends before
/// his strike resolves, he does not burn (LSJ 20001127-2, LSJ 20010627,
/// LSJ 20070508).
// Rulings: [LSJ 20001127-2] [LSJ 20010627] [LSJ 20070508].
#[test]
fn escaped_mental_patient_survives_a_rotschreck() {
    let (mut e, a, p, strike) = patient_blocks("pot", &[]);
    hand(&mut e, 1, &["Rötschreck"]);
    pick_when(&mut e, strike);
    pick_when(&mut e, master_on(0, Target::Minion(a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[p.index()].in_play, "not burned");
    assert!(e.state.minions[a.index()].in_torpor, "Rötschreck's torpor");
}

// ---- Modifiers and reactions ------------------------------------------------

/// The `nth` textless Sabbat vampire of capacity 4 to 7 (4 blood kept).
fn sabbat4(nth: usize) -> u16 {
    textless(
        |c| c.sect == Some(Sect::Sabbat) && (4..=7).contains(&c.capacity),
        nth,
    )
}

/// The hand card `card` played in `mode` by `actor` at minion `t`.
fn play_at(card: u8, mode: u8, actor: MinionId, t: MinionId) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target: Target::Minion(t),
    }
}

/// The ranges of the strikes noted since notes were kept.
fn strike_ranges(e: &mut Engine) -> Vec<zvtes::combat::Range> {
    noted_combats(e)
        .into_iter()
        .filter_map(|n| match n {
            zvtes::combat::CombatNote::Strikes { range, .. } => Some(range),
            _ => None,
        })
        .collect()
}

/// Gang Tactics: "If the action is blocked, the resulting combat is at
/// close range. Other effects cannot change this; skip the determine range
/// step during that combat." No determine range step, every strike close;
/// the blocker's Cold Aura [nec], a later range setter, never offered (PIB
/// 20120214: the first setter has priority).
// Rulings: [RTR 19970630] [ANK 20180720] [PIB 20120214].
#[test]
fn gang_tactics_closes_every_round_of_the_block() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Nosferatu antitribu", 0), 4, false);
    e.add_minion(SeatId(0), plain("Nosferatu antitribu", 1), 4, false);
    let x = e.add_minion(SeatId(1), vampire_with("nec", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Gang Tactics"]);
    hand(&mut e, 1, &["Cold Aura"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(a));
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, block(x));
    let mut range_step = false;
    loop {
        let req = request(&mut e);
        range_step |= req.window == WindowLabel::CombatDetermineRange;
        assert!(!req.options.contains(&play(0, 0, x)), "Cold Aura");
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(!range_step);
    let ranges = strike_ranges(&mut e);
    assert!(!ranges.is_empty());
    assert!(ranges.iter().all(|r| *r == zvtes::combat::Range::Close));
}

/// Seat 0's Sabbat `a` hunts (+1 stealth) and plays Stealth Ritus on its
/// Sabbat `b` (+1): stealth 2. Seat 1's `x` attempts to block.
fn hunt_at_two_stealth(x_intercept: bool) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), sabbat4(0), 2, false);
    let b = e.add_minion(SeatId(0), sabbat4(1), 2, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    if x_intercept {
        in_play(&mut e, 1, "Raven Spy", Some(x));
    }
    hand(&mut e, 0, &["Stealth Ritus"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick(&mut e, play_at(0, 0, a, b));
    pick_when(&mut e, block(x));
    (e, a, x)
}

/// Legwork: "Do not replace until your next unlock phase. +1 intercept. Not
/// usable by a vampire with more than 0 intercept." Offered to a vampire at
/// 0 intercept, held until its next unlock phase; not to one at +1 (Raven
/// Spy), though it still needs intercept.
#[test]
fn legwork_gives_intercept_only_from_zero() {
    let (mut e, _, x) = hunt_at_two_stealth(false);
    e.state.seats[1].library = vec![find_library("Form of Mist"); 5].into();
    hand(&mut e, 1, &["Legwork"]);
    fill_hand(&mut e, 1);
    pick_when(&mut e, play(0, 0, x));
    request(&mut e);
    assert_eq!(
        e.action().expect("action").intercept_of(&e.state, x),
        1,
        "+1 intercept"
    );
    assert_eq!(
        e.state.seats[1].held[zvtes::state::Release::NextUnlockPhase],
        1
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.seats[1].held[zvtes::state::Release::NextUnlockPhase],
        0,
        "replaced at its next unlock phase"
    );

    let (mut e, _, x) = hunt_at_two_stealth(true);
    hand(&mut e, 1, &["Legwork"]);
    let short = e.action().expect("action");
    assert!(short.intercept_of(&e.state, x) < short.stealth(&e.state));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
}

/// Legwork (1 blood) is never an ally's (its blood cost; rulebook, allies):
/// seat 1's Political Ally attempting to block a 2-stealth hunt, short of
/// intercept, is not offered it.
#[test]
fn legwork_is_not_offered_to_an_ally() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), sabbat4(0), 2, false);
    let b = e.add_minion(SeatId(0), sabbat4(1), 2, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Stealth Ritus"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick(&mut e, play_at(0, 0, a, b));
    pick_when(&mut e, block(ally));
    hand(&mut e, 1, &["Legwork"]);
    let short = e.action().expect("action");
    assert!(short.intercept_of(&e.state, ally) < short.stealth(&e.state));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play(0, 0, ally),
    );
}

/// Stealth Ritus: "Choose another ready Sabbat vampire you control. The
/// chosen vampire burns 1 blood, or this card has no effect. This action
/// gets +1 stealth, even if stealth is not yet needed." Only `b` (another
/// Sabbat of seat 0) is offered, not the actor nor the Camarilla `c`; `a`
/// pays the card's 1, `b` (locked, still ready) burns 1, stealth 1 out of
/// the blocker's reach. With `b` empty: offered, no effect, the cost still
/// paid.
// Rulings: [ANK 20190425] [ANK 20181208].
#[test]
fn stealth_ritus_takes_a_blood_from_another_sabbat_vampire_for_stealth() {
    for b_blood in [2, 0] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), sabbat4(0), 4, false);
        let b = e.add_minion(SeatId(0), sabbat4(1), b_blood, false);
        // Locked, still ready (it would hunt empty).
        e.state.minions[b.index()].locked = true;
        let c = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        let x = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
        hand(&mut e, 0, &["Stealth Ritus"]);
        pick(&mut e, bleed(a));
        let req = expect(&mut e, SeatId(0), WindowLabel::ActionAsAnnounced);
        assert!(req.options.contains(&play_at(0, 0, a, b)));
        assert!(!req.options.contains(&play_at(0, 0, a, a)));
        assert!(!req.options.contains(&play_at(0, 0, a, c)));
        e.choose(index_of(&req, play_at(0, 0, a, b)));
        request(&mut e);
        let stealth = e.action().expect("action").stealth(&e.state);
        assert_eq!(blood(&e, a), 3, "its cost");
        if b_blood == 2 {
            assert_eq!((blood(&e, b), stealth), (1, 1));
            never_reaching_until(&mut e, SeatId(0), WindowLabel::MinionPhase, x);
        } else {
            assert_eq!((blood(&e, b), stealth), (0, 0), "no effect");
        }
    }
}

/// Seat 0's Sabbat `a` bleeds seat 1 at +1 stealth (Stealth Ritus on `b`);
/// seat 1's `x` (Obtenebration and Auspex) attempts to block, `cards` in
/// hand.
fn darksight_table(seats: usize, cards: &[&str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(seats);
    let a = e.add_minion(SeatId(0), sabbat4(0), 4, false);
    let b = e.add_minion(SeatId(0), sabbat4(1), 2, false);
    let x = e.add_minion(SeatId(1), vampire_with("OBT", 0), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["aus"];
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Stealth Ritus"]);
    hand(&mut e, 1, cards);
    pick(&mut e, bleed(a));
    pick(&mut e, play_at(0, 0, a, b));
    pick_when(&mut e, block(x));
    (e, a, x)
}

/// Darksight [OBT]: "if this vampire blocks, put this card on the acting
/// minion. The attached minion gets -1 stealth when bleeding this reacting
/// vampire's controller." The card itself on the acting `a`, controlled by
/// seat 0; `a`'s next bleed at seat 1 is at -1 stealth, none against seat
/// 2.
#[test]
fn darksight_superior_follows_a_blocked_minion_and_slows_its_bleeds() {
    let (mut e, a, x) = darksight_table(3, &["Darksight"]);
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Darksight", Some(a));
    assert_eq!(e.state.card_controller(card), SeatId(0));
    assert_eq!(e.state.bleed_stealth_against(a, SeatId(2)), 0);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a));
    request(&mut e);
    let act = e.action().expect("the bleed");
    assert_eq!(act.bleed_target, Some(SeatId(1)));
    assert_eq!(act.stealth(&e.state), -1);
}

/// Darksight [ANK 20220805] "Goes to the ash heap when played, if it has
/// been removed when the vampire blocks, it has no effect." The hunt at 2
/// stealth: [OBT] played first, taken out of the ash heap, then the
/// second +1 lands the block: nothing on the acting minion, no stand-in.
/// Melange [AUS] alike (the same placement).
// Rulings: [ANK 20220805].
#[test]
fn darksight_removed_before_the_block_has_no_effect() {
    for name in ["Darksight", "Melange"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), sabbat4(0), 2, false);
        let b = e.add_minion(SeatId(0), sabbat4(1), 2, false);
        let x = e.add_minion(SeatId(1), vampire_with("OBT", 0), 4, false);
        e.state.minions[x.index()].gained_levels = vec!["aus", "aus"];
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Stealth Ritus"]);
        let second = if name == "Darksight" {
            "Melange"
        } else {
            "Darksight"
        };
        hand(&mut e, 1, &[name, second]);
        pick(&mut e, action(a, ActionKind::Hunt));
        pick(&mut e, play_at(0, 0, a, b));
        pick_when(&mut e, block(x));
        pick_when(&mut e, play(0, 1, x));
        request(&mut e);
        let card = find_library(name);
        let ash = &mut e.state.seats[1].ash;
        let k = ash
            .iter()
            .position(|&c| c == card)
            .expect("in the ash heap");
        ash.take(k);
        pick_when(&mut e, play(0, 0, x));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(
            !e.state.cards.iter().any(|c| c.card == card && c.present()),
            "{name}: nothing in play"
        );
        assert!(e.state.minions[x.index()].locked, "{name}: it blocked");
    }
}

/// Darksight: "Any minion can burn this card as a +1 stealth action." Seat
/// 0's ally burns the Darksight seat 1 put on seat 0's vampire: the card in
/// its owner's ash heap.
#[test]
fn darksight_is_burned_by_any_minion_as_a_stealth_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), sabbat4(0), 4, false);
    let ally = e.add_ally(SeatId(0), find_library("Political Ally"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = in_play(&mut e, 1, "Darksight", Some(a));
    pick(&mut e, use_on(card, 0, Some(ally), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[card.index()].present());
    assert!(ash_has(&e, 1, "Darksight"));
}

/// Patterns in the Chaos: "Only usable after resolution of a successful
/// bleed. [dem] Look at the top card of the library of the target
/// Methuselah. [DEM] Look at the target Methuselah's hand." After a
/// successful bleed: [DEM] sees seat 1's hand, [dem] its library's top.
/// After a hunt: not offered.
#[test]
fn patterns_in_the_chaos_sees_the_bled_methuselahs_cards() {
    for mode in [0, 1] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("DEM", 0), 4, false);
        e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Patterns in the Chaos"]);
        fill_hand(&mut e, 1);
        e.state.seats[1].library = vec![find_library("Conceal"); 3].into();
        e.notes = Some(Vec::new());
        pick(&mut e, bleed(a));
        pick_when(&mut e, play(0, mode, a));
        request(&mut e);
        let looks = noted_looks(&mut e);
        let look = looks.last().expect("a look");
        assert_eq!((look.viewer, look.seat), (SeatId(0), SeatId(1)));
        if mode == 0 {
            assert_eq!(look.zone, zvtes::game::LookedZone::Library);
            assert_eq!(look.cards, vec![find_library("Conceal")]);
        } else {
            assert_eq!(look.zone, zvtes::game::LookedZone::Hand);
            assert_eq!(look.cards, e.state.seats[1].hand.to_vec(), "its hand");
        }
    }
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("DEM", 0), 2, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Patterns in the Chaos"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { card: 0, .. })
    });
}

/// Patterns in the Chaos with nothing to look at (D14; advanced rules
/// §1.6.5; rules-qa 78.20): [dem] is not offered against an empty
/// library, [DEM] still is; once the bleed ousted its target, neither.
#[test]
fn patterns_in_the_chaos_needs_a_library_and_a_seated_target() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("DEM", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Patterns in the Chaos"]);
    fill_hand(&mut e, 1);
    pick(&mut e, bleed(a));
    let req = offering(&mut e, play(0, 1, a));
    assert!(!req.options.contains(&play(0, 0, a)), "an empty library");

    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("DEM", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
    e.state.seats[1].pool = 1;
    e.state.seats[1].library = vec![find_library("Conceal"); 3].into();
    hand(&mut e, 0, &["Patterns in the Chaos"]);
    pick(&mut e, bleed(a));
    never_offered_where(&mut e, SeatId(2), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { card: 0, .. })
    });
    assert!(e.state.seats[1].ousted);
}

/// Hecate's library burn ("the top card of the target Methuselah's
/// library") is not offered against an empty library (D14, rules-qa
/// 78.20).
#[test]
fn hecate_burns_nothing_from_an_empty_library() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hecate"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    fill_hand(&mut e, 1);
    pick(&mut e, bleed(h));
    vc_never(&mut e, h, 0, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "bled");
}

/// Seat 0's Sabbat `a` (titled `a_title`) bleeds; seat 1's Sabbat `x`
/// (titled `x_title`) blocks with Cardinal Sin: Insubordination in hand;
/// the combat's hand strikes leave both ready.
fn insubordination(a_title: Title, x_title: Title) -> (Engine, MinionId, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), sabbat4(0), 4, false);
    e.state.minions[a.index()].granted_title = Some(a_title).filter(|t| *t != Title::None);
    e.add_minion(SeatId(0), sabbat4(1), 4, false);
    let x = e.add_minion(SeatId(1), sabbat4(2), 4, false);
    e.state.minions[x.index()].granted_title = Some(x_title).filter(|t| *t != Title::None);
    let y = e.add_minion(SeatId(1), sabbat4(3), 4, false);
    e.state.minions[y.index()].granted_title = Some(Title::Cardinal);
    hand(&mut e, 1, &["Cardinal Sin: Insubordination"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    (e, a, x, y)
}

/// Cardinal Sin: Insubordination: "Only usable after a combat involving
/// this vampire and a non-titled Sabbat vampire; only usable if the
/// opposing vampire is ready. Usable by a locked vampire. The opposing
/// vampire goes into torpor." The blocking archbishop, locked, sends the
/// untitled acting vampire to torpor; seat 1's cardinal, not in the
/// combat, is not offered it.
#[test]
fn cardinal_sin_insubordination_sends_an_untitled_sabbat_attacker_to_torpor() {
    let (mut e, a, x, y) = insubordination(Title::None, Title::Archbishop("Paris"));
    let req = offering(&mut e, play_at(0, 0, x, a));
    assert_eq!(req.window, WindowLabel::ActionAfterCombat);
    assert!(e.state.minions[x.index()].locked, "a locked vampire");
    assert!(
        !req.options.contains(&play_at(0, 0, y, a)),
        "not in the combat"
    );
    e.choose(index_of(&req, play_at(0, 0, x, a)));
    request(&mut e);
    assert!(e.state.minions[a.index()].in_torpor);
    assert!(ash_has(&e, 1, "Cardinal Sin: Insubordination"));
}

/// Cardinal Sin: Insubordination after an earlier combat of the action
/// (owner, rules-qa 122.28: any combat of the action): Devorah's Hedonism
/// fails X's attempt and queues their combat; archbishop Y blocks A, both
/// ready after; the queued combat comes last, and Y, in the action's
/// after-combat window, sends A to torpor.
#[test]
fn cardinal_sin_insubordination_after_an_earlier_combat_of_the_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), sabbat4(0), 4, false);
    let devorah = e.add_minion(SeatId(0), find_card("Devorah"), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let y = e.add_minion(SeatId(1), sabbat4(3), 4, false);
    e.state.minions[y.index()].granted_title = Some(Title::Archbishop("Paris"));
    hand(&mut e, 0, &["Hedonism"]);
    hand(&mut e, 1, &["Cardinal Sin: Insubordination"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, devorah));
    pick_when(&mut e, block(y));
    let req = offering(&mut e, play_at(0, 0, y, a));
    assert_eq!(req.window, WindowLabel::ActionAfterCombat);
    assert_eq!((blood(&e, a), blood(&e, devorah)), (3, 3), "both combats");
    e.choose(index_of(&req, play_at(0, 0, y, a)));
    request(&mut e);
    assert!(e.state.minions[a.index()].in_torpor);
    assert!(!e.state.minions[devorah.index()].in_torpor);
    assert!(ash_has(&e, 1, "Cardinal Sin: Insubordination"));
}

/// Cardinal Sin: Insubordination: not by an untitled or a bishop blocker,
/// not against a titled acting vampire (a bishop).
#[test]
fn cardinal_sin_insubordination_needs_its_titles() {
    for (a_title, x_title) in [
        (Title::None, Title::None),
        (Title::None, Title::Bishop),
        (Title::Bishop, Title::Archbishop("Paris")),
    ] {
        let (mut e, a, x, _) = insubordination(a_title, x_title);
        never_offered_until(
            &mut e,
            SeatId(0),
            WindowLabel::MinionPhase,
            play_at(0, 0, x, a),
        );
        assert!(!e.state.minions[a.index()].in_torpor);
    }
}

/// Seat 0's Dominate vampire `a` bleeds; seat 1's ally (Political Ally,
/// another vampire beside it) attempts to block; seat 0 holds Command.
fn command_table() -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("DOM", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Command"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(ally));
    (e, a, ally)
}

/// Command [dom]: "Only usable when an ally attempts to block. The attempt
/// fails and lock that ally. That ally cannot attempt to block this action
/// again." The ally locked, the bleed unblocked (seat 1 loses 1 pool), no
/// second attempt; not offered against a vampire's attempt.
#[test]
fn command_fails_an_allys_attempt_and_locks_it() {
    let (mut e, a, ally) = command_table();
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert!(e.state.minions[ally.index()].locked);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(ally));
    assert_eq!(pool(&e)[1], 10 - 1, "the bleed resolved");
    assert!(ash_has(&e, 0, "Command"));

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("DOM", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
    hand(&mut e, 0, &["Command"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { card: 0, .. })
    });
}

/// Command [DOM]: "As above, and put this card on the ally. This ally does
/// not unlock as normal. During this ally's next unlock phase, burn this
/// card." The card itself on the ally; at seat 1's next unlock phase the
/// ally stays locked and the card burns to seat 0's ash heap; the next
/// one, it unlocks.
#[test]
fn command_superior_holds_the_ally_through_its_next_unlock() {
    let (mut e, a, ally) = command_table();
    pick_when(&mut e, play(0, 1, a));
    request(&mut e);
    let card = placed_on(&e, "Command", Some(ally));
    assert_eq!(e.state.card_controller(card), SeatId(1));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        e.state.minions[ally.index()].locked,
        "not unlocked as normal"
    );
    assert!(!e.state.cards[card.index()].present());
    assert!(ash_has(&e, 0, "Command"));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].locked);
}

/// Seat 0's `bleeder` bleeds seat 1 unblocked; seat 1's Brujah antitribu
/// `y` holds Vendetta; seat 2 has a Brujah antitribu too.
fn vendetta_table(bleeder: u16) -> (Engine, MinionId, MinionId, MinionId) {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), bleeder, 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Brujah antitribu", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let z = e.add_minion(SeatId(2), plain("Brujah antitribu", 1), 4, false);
    hand(&mut e, 1, &["Vendetta"]);
    pick(&mut e, bleed(a));
    (e, a, y, z)
}

/// Vendetta: "Only usable after a minion successfully bleeds you for 3 or
/// more. Put this card on the acting minion." Stanislava (+2 bleed) bleeds
/// seat 1 for 3: the card itself on her, and a second copy by seat 1's
/// other Brujah antitribu too (no "only one"). A bleed for 1: not offered.
#[test]
fn vendetta_marks_a_minion_that_bled_you_for_three() {
    let (mut e, a, y, _) = vendetta_table(find_card("Stanislava"));
    let y2 = e.add_minion(SeatId(1), plain("Brujah antitribu", 2), 4, false);
    hand(&mut e, 1, &["Vendetta"]);
    pick_when(&mut e, play(0, 0, y));
    pick_when(&mut e, play(0, 0, y2));
    request(&mut e);
    assert_eq!(pool(&e)[1], 10 - 3);
    let card = placed_on(&e, "Vendetta", Some(a));
    assert_eq!(e.state.card_controller(card), SeatId(0));
    let vendetta = find_library("Vendetta");
    let on = e
        .state
        .cards
        .iter()
        .filter(|c| c.card == vendetta && c.present() && !c.effect_only && c.host == Some(a))
        .count();
    assert_eq!(on, 2, "both copies");

    let (mut e, _, y, _) = vendetta_table(plain("Ventrue", 2));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, y));
    assert_eq!(pool(&e)[1], 10 - 1);
}

/// Vendetta: "Brujah antitribu get +1 strength in combat with the attached
/// minion. Brujah antitribu can enter combat with the attached minion as a
/// Ⓓ action. If the attached minion is in torpor, burn this card." Seat
/// 2's Brujah antitribu rushes the marked vampire (seat 0's own is not
/// offered it); two Vendettas stack: its hand strike at 3 sends the
/// 2-blood vampire to torpor, the cards burn.
// Rulings: [LSJ 20080604].
#[test]
fn vendetta_lets_brujah_antitribu_rush_the_marked_minion_stronger() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 2, false);
    let own = e.add_minion(SeatId(0), plain("Brujah antitribu", 0), 4, false);
    let z = e.add_minion(SeatId(2), plain("Brujah antitribu", 1), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
    let v1 = in_play(&mut e, 1, "Vendetta", Some(a));
    let v2 = in_play(&mut e, 1, "Vendetta", Some(a));
    let rush = |card: CardId, m: MinionId| use_on(card, 0, Some(m), Target::Minion(a));
    let req = request(&mut e);
    assert!(
        !req.options.contains(&rush(v1, own)),
        "its own Methuselah's"
    );
    assert_eq!(e.state.strength_against(z, a), 2, "two Vendettas");
    assert_eq!(e.state.strength_against(a, z), 0, "Brujah antitribu only");
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    pick(&mut e, rush(v1, z));
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].in_torpor, "2 blood, 1+2 damage");
    assert!(!e.state.cards[v1.index()].present() && !e.state.cards[v2.index()].present());
    assert!(ash_has(&e, 1, "Vendetta"));
}

/// Seat 0's Vicissitude vampire recruits Vagabond Mystic and plays
/// Lobotomy in `mode` during the action.
fn lobotomy_recruit(mode: u8) -> (Engine, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("VIC", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Vagabond Mystic", "Lobotomy"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, mode, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let ally = MinionId(e.state.minions.len() as u32 - 1);
    assert!(e.state.minions[ally.index()].is_ally());
    (e, ally)
}

/// Lobotomy: "[vic] Put this card on the ally and lock the ally after
/// action resolution if this action is successful. … The attached ally
/// gets -1 bleed and cannot maneuver. [VIC] As above, but the ally is not
/// locked." The card itself on the recruited ally, locked at [vic] only;
/// its bleed down 1, no maneuver.
#[test]
fn lobotomy_goes_on_the_recruited_ally() {
    for (mode, locked) in [(0, true), (1, false)] {
        let (e, ally) = lobotomy_recruit(mode);
        placed_on(&e, "Lobotomy", Some(ally));
        assert_eq!(e.state.minions[ally.index()].locked, locked);
        assert_eq!(
            e.state
                .bearer_mod(ally, zvtes::cards::Stat::Bleed, ActionKind::Bleed),
            -1
        );
        assert!(e
            .state
            .bears(ally, zvtes::cards::Modifier::BearerNoManeuver));
    }
}

/// Lobotomy: "Actions requiring Dominate [dom] or Presence [pre] have no
/// effect on the attached ally" (red reading: offered, void at
/// resolution). Entrancement [PRE] at seat 1's Lobotomized ally: offered,
/// unblocked, the ally stays with seat 1; at an ally without it, stolen.
#[test]
fn lobotomy_makes_dom_and_pre_actions_void_on_the_ally() {
    for lobotomized in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("PRE", 0), 4, false);
        e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        let ally = e.add_ally(SeatId(1), find_library("Vagabond Mystic"));
        e.state.minions[ally.index()].locked = true;
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        if lobotomized {
            in_play(&mut e, 0, "Lobotomy", Some(ally));
        }
        hand(&mut e, 0, &["Entrancement"]);
        pick(&mut e, play_at(0, 1, a, ally));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let owner = e.state.minions[ally.index()].owner;
        assert_eq!(owner, if lobotomized { SeatId(1) } else { SeatId(0) });
        assert!(ash_has(&e, 0, "Entrancement"));
    }
}

/// Seat 0's `caller` plays Kine Resources Contested (hand slot 0) and the
/// referendum's terms are answered: the polling step is under way.
fn kine_polling(e: &mut Engine, caller: MinionId) {
    pick(e, play(0, 0, caller));
    loop {
        let req = request(e);
        if req.window == WindowLabel::ReferendumPolling {
            return;
        }
        e.choose(fallback(&req));
    }
}

/// The side of `m`'s standing cast, if any.
fn cast_side(e: &Engine, m: MinionId) -> Option<bool> {
    e.referendum()
        .expect("referendum")
        .records
        .iter()
        .find_map(|r| {
            (r.source == zvtes::referendum::Source::Vampire(m) && !r.canceled).then_some(r.favor)
        })
}

/// Cheval de Bataille: "Any vampire casting votes or ballots against this
/// referendum burns 1 blood when the results are tallied." [RTR 19951110]
/// Seat 1's bishop votes against before it is played, seat 2's after: each
/// burns 1; seat 2's empty bishop voting against burns nothing and stays;
/// the one voting in favor keeps its blood.
// Rulings: [RTR 19951110].
#[test]
fn cheval_de_bataille_burns_every_vampire_voting_against() {
    let mut e = game(3);
    let c = e.add_minion(SeatId(0), sabbat4(0), 4, false);
    e.state.minions[c.index()].granted_title = Some(Title::Bishop);
    e.add_minion(SeatId(0), sabbat4(1), 4, false);
    let v1 = e.add_minion(SeatId(1), sabbat4(2), 3, false);
    let f1 = e.add_minion(SeatId(1), sabbat4(3), 3, false);
    let v2 = e.add_minion(SeatId(2), sabbat4(4), 3, false);
    let v0 = e.add_minion(SeatId(2), sabbat4(5), 0, false);
    for m in [v1, f1, v2, v0] {
        e.state.minions[m.index()].granted_title = Some(Title::Bishop);
    }
    hand(
        &mut e,
        0,
        &["Kine Resources Contested", "Cheval de Bataille"],
    );
    kine_polling(&mut e, c);
    pick_when(&mut e, votes(v1, false));
    pick_when(&mut e, play(0, 0, c));
    pick_when(&mut e, votes(f1, true));
    pick_when(&mut e, votes(v2, false));
    pick_when(&mut e, votes(v0, false));
    close_polling(&mut e);
    request(&mut e);
    assert_eq!([v1, f1, v2, v0].map(|m| blood(&e, m)), [3 - 1, 3, 3 - 1, 0]);
    assert!(e.state.minions[v0.index()].in_play);
}

/// Cheval de Bataille: "Requires a ready titled Sabbat vampire." Not
/// offered to an untitled Sabbat caller nor a titled Camarilla one.
#[test]
fn cheval_de_bataille_needs_a_titled_sabbat_vampire() {
    for camarilla in [false, true] {
        let mut e = game(2);
        let c = if camarilla {
            e.add_minion(SeatId(0), find_card("Tara"), 4, false)
        } else {
            e.add_minion(SeatId(0), sabbat4(0), 4, false)
        };
        e.add_minion(SeatId(0), sabbat4(1), 4, false);
        let v = e.add_minion(SeatId(1), sabbat4(2), 3, false);
        e.state.minions[v.index()].granted_title = Some(Title::Bishop);
        hand(
            &mut e,
            0,
            &["Kine Resources Contested", "Cheval de Bataille"],
        );
        kine_polling(&mut e, c);
        never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
            matches!(o, Choice::Play { card: 0, .. })
        });
    }
}

/// De Sade (Ventrue antitribu, 9) calls Kine Resources Contested with
/// Kindred Manipulation in hand; seat 1's younger bishop `t`.
fn de_sade_calls() -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("De Sade"), 4, false);
    e.add_minion(SeatId(0), sabbat4(0), 4, false);
    let t = e.add_minion(SeatId(1), sabbat4(1), 4, false);
    e.state.minions[t.index()].granted_title = Some(Title::Bishop);
    e.add_minion(SeatId(1), sabbat4(2), 4, false);
    hand(
        &mut e,
        0,
        &["Kine Resources Contested", "Kindred Manipulation"],
    );
    kine_polling(&mut e, k);
    (e, k, t)
}

/// Kindred Manipulation: "Change the votes and ballots of a younger
/// vampire to votes and ballots of your choice." [RTR 19991001] [LSJ
/// 20030828] "Playable on an abstaining vampire: it does not force them to
/// vote, but restricts their choice if they do." Played "in favor" on `t`
/// before it cast: no cast made; `t` may cast in favor or pass, not
/// against.
// Rulings: [RTR 19991001] [LSJ 20030828].
#[test]
fn kindred_manipulation_restricts_an_uncast_vampire() {
    let (mut e, k, t) = de_sade_calls();
    pick_when(&mut e, play_at(0, 0, k, t));
    let req = expect(&mut e, SeatId(1), WindowLabel::ReferendumPolling);
    assert_eq!(cast_side(&e, t), None, "not compelled");
    assert!(req.options.contains(&votes(t, true)));
    assert!(req.options.contains(&Choice::Pass));
    assert!(!req.options.contains(&votes(t, false)));
}

/// Kindred Manipulation on a cast made: [ANK 20221019] (De Sade) "His
/// ability applies only to the first time a vampire casts his votes, not
/// if they change afterwards." `t` casts in favor, De Sade's Kindred
/// Manipulation turns it against: no toll.
// Rulings: [ANK 20221019].
#[test]
fn kindred_manipulation_turns_a_cast_without_de_sades_toll() {
    let (mut e, k, t) = de_sade_calls();
    pick_when(&mut e, votes(t, true));
    pick_when(&mut e, play_at(0, 1, k, t));
    request(&mut e);
    assert_eq!(cast_side(&e, t), Some(false));
    assert_eq!(blood(&e, t), 4, "no toll");
}

/// Michael Luther: "lock Michael during a referendum to change the votes
/// and ballots of a Camarilla vampire to votes and ballots of your
/// choice." A change compels no vampire that has not cast (Kindred
/// Manipulation's RTR 19991001; advanced rules §3.7.6.3): Tara, changed
/// against before she casts, has cast nothing and may only cast against.
#[test]
fn michael_luther_does_not_compel_an_uncast_vampire() {
    let mut e = game(2);
    let tara = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ml = e.add_minion(SeatId(1), find_card("Michael Luther"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    kine_polling(&mut e, tara);
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(ml),
            ability: 1,
            actor: None,
            target: Target::Minion(tara),
        },
    );
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    assert_eq!(cast_side(&e, tara), None, "not compelled");
    assert!(req.options.contains(&votes(tara, false)));
    assert!(!req.options.contains(&votes(tara, true)));
}

/// Inconnu Tutelage: "A Methuselah may spend four transfers and remove a
/// vampire in his or her uncontrolled region from the game to search for
/// any card in his or her library and put it in his or her hand." Seat 1's
/// event, used by seat 0: its second uncontrolled vampire removed, its 4
/// transfers spent, Deflection from its library in hand; an imbued there
/// is no target.
#[test]
fn inconnu_tutelage_trades_four_transfers_and_a_vampire_for_any_card() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let keep = plain("Brujah", 0);
    let gone = plain("Brujah", 1);
    let imbued = CRYPT.iter().position(|c| c.imbued).expect("an imbued") as u16;
    for card in [keep, gone, imbued] {
        e.state.seats[0]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card,
                copy: Default::default(),
                counters: 0,
                banished: None,
            });
    }
    fill_hand(&mut e, 0);
    let deflection = find_library("Deflection");
    e.state.seats[0].library =
        vec![find_library("Conceal"), deflection, find_library("Conceal")].into();
    let event = in_play(&mut e, 1, "Inconnu Tutelage", None);
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    assert!(!req
        .options
        .contains(&use_on(event, 0, None, Target::Uncontrolled(2))));
    e.choose(index_of(
        &req,
        use_on(event, 0, None, Target::Uncontrolled(1)),
    ));
    pick(&mut e, Choice::FindCard(deflection));
    request(&mut e);
    let s = &e.state.seats[0];
    assert!(s.hand.contains(&deflection));
    assert_eq!(s.library.len(), 2);
    assert_eq!(
        s.uncontrolled.iter().map(|u| u.card).collect::<Vec<_>>(),
        vec![keep, imbued]
    );
    assert_eq!(s.removed_crypt, vec![gone]);
}

/// Inconnu Tutelage "to search for any card in his or her library": not
/// offered with an empty library (rules-qa 80.9, 78.20; advanced rules
/// §1.6.5); offered with a card in it.
#[test]
fn inconnu_tutelage_needs_a_library_to_search() {
    for stocked in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        e.state.seats[0]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card: plain("Brujah", 0),
                copy: Default::default(),
                counters: 0,
                banished: None,
            });
        fill_hand(&mut e, 0);
        if stocked {
            e.state.seats[0].library = vec![find_library("Deflection")].into();
        }
        let event = in_play(&mut e, 1, "Inconnu Tutelage", None);
        let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
        let used = use_on(event, 0, None, Target::Uncontrolled(0));
        assert_eq!(req.options.contains(&used), stocked);
    }
}

/// Lam Into [vic]: "This strike is at +1 damage, with 1 optional press. If
/// another round of combat occurs, that round is at close range (skip the
/// determine range step for that round)." The press taken, round 2 is
/// close with no determine range step, the blocker's later range setter
/// (Cold Aura [nec]; Neebi by a Laibon with three Aye) not offered then
/// (PIB 20120214: the first setter has priority). Vicissitude vampires are
/// in the pool now (round 31); this one made Anarch.
// Rulings: [TOM 19960521] [RTR 19970630] [ANK 20180720] [PIB 20120214].
#[test]
fn lam_into_vicissitude_presses_into_a_close_round() {
    let vic = vampire_with("vic", 0);
    let laibon = textless(|c| c.sect == Some(Sect::Laibon) && c.capacity >= 4, 0);
    for neebi in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vic, 6, false);
        e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
        e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        let x = if neebi {
            let x = e.add_minion(SeatId(1), laibon, 6, false);
            for _ in 0..3 {
                in_play(&mut e, 1, "Aye", Some(x));
            }
            x
        } else {
            e.add_minion(SeatId(1), vampire_with("nec", 0), 6, false)
        };
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Lam Into"]);
        hand(&mut e, 1, &[if neebi { "Neebi" } else { "Cold Aura" }]);
        e.notes = Some(Vec::new());
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(x));
        pick_when(
            &mut e,
            Choice::Play {
                card: 0,
                mode: 4,
                actor: Some(a),
                target: Target::None,
            },
        );
        pick_when(&mut e, Choice::UsePress);
        loop {
            let req = request(&mut e);
            if e.combat().is_some_and(|c| c.round == 2) {
                assert_ne!(req.window, WindowLabel::CombatDetermineRange);
                assert!(
                    !req.options.iter().any(|o| matches!(o,
                        Choice::Play { actor: Some(m), .. } if *m == x)),
                    "{:?}",
                    req.options
                );
            }
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            e.choose(fallback(&req));
        }
        let close: Vec<_> = noted_combats(&mut e)
            .into_iter()
            .filter_map(|n| match n {
                zvtes::combat::CombatNote::Strikes {
                    round: 2, range, ..
                } => Some(range),
                _ => None,
            })
            .collect();
        assert!(!close.is_empty(), "pressed into round 2");
        assert!(close.iter().all(|r| *r == zvtes::combat::Range::Close));
    }
}

// ---- Track B (combat, equipment, vamp_c, vamp_a): helpers -------------------

/// A bleeds, X blocks: their combat begins.
fn bleed_blocked(e: &mut Engine, a: MinionId, x: MinionId) {
    pick(e, bleed(a));
    pick_when(e, block(x));
}

/// Seat 0's A and a second vampire, seat 1's X and a second one: plain
/// textless vampires of capacity 4 to 8, 4 blood each, A with `a_levels`
/// and X with `x_levels` gained.
fn duel(a_levels: &[&'static str], x_levels: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity);
    let a = e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    e.add_minion(SeatId(0), textless(mid, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.add_minion(SeatId(1), textless(mid, 3), 4, false);
    e.state.minions[a.index()].gained_levels = a_levels.to_vec();
    e.state.minions[x.index()].gained_levels = x_levels.to_vec();
    (e, a, x)
}

fn pending(e: &Engine, side: usize) -> (u8, u8) {
    let p = e.combat().expect("combat").sides[side].pending;
    (p.normal, p.aggravated)
}

/// +n strength until the end of the turn.
fn stronger(e: &mut Engine, m: MinionId, n: i8) {
    let seat = e.state.minions[m.index()].owner;
    e.state.minions[m.index()].timed.push((
        zvtes::cards::Lasting::Strength(n),
        zvtes::state::Expiry::EndOfTurn,
        seat,
    ));
}

fn in_torpor(e: &Engine, m: MinionId) -> bool {
    let v = &e.state.minions[m.index()];
    v.in_play && v.in_torpor
}

// ---- Acrobatics -------------------------------------------------------------

/// Acrobatics [CEL]: "Strike: dodge, with an additional strike (limited)."
/// X's hand strike dodged; A's additional hand strike lands.
#[test]
fn acrobatics_superior_dodges_then_strikes_again() {
    let (mut e, a, x) = duel(&["cel", "cel"], &[]);
    hand(&mut e, 0, &["Acrobatics"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1, "its cost; X's strike dodged");
    assert_eq!(blood(&e, x), 4 - 1, "the additional hand strike");
}

/// Acrobatics: "Cannot be played if the minion cannot play additional
/// strike cards, even for the dodge." [LSJ 20011214-5] Under Blight [NEC],
/// neither level is offered to A.
#[test]
fn acrobatics_is_barred_by_blight() {
    let (mut e, a, x) = duel(&["cel", "cel"], &["nec", "nec"]);
    hand(&mut e, 0, &["Acrobatics"]);
    hand(&mut e, 1, &["Blight"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == a),
    );
    assert!(e.state.seats[0].hand.contains(&find_library("Acrobatics")));
}

// ---- Backstab ---------------------------------------------------------------

/// Backstab [cel]: "This vampire gets first strike on his or her initial
/// strike this round." "Can be played at any time before strike
/// resolution, including after both minions have selected their strikes."
/// [LSJ 20000215] Both hand strikes chosen, X, the blocker, plays it: A,
/// at 1 blood against X's strength 2, goes to torpor before striking.
#[test]
fn backstab_after_strikes_are_chosen_gives_first_strike() {
    let (mut e, a, x) = duel(&[], &["cel"]);
    e.state.minions[a.index()].blood = 1;
    stronger(&mut e, x, 1);
    hand(&mut e, 1, &["Backstab"]);
    bleed_blocked(&mut e, a, x);
    let req = until(&mut e, SeatId(1), WindowLabel::CombatBeforeResolution);
    let c = e.combat().expect("combat");
    assert!(c.sides.iter().all(|s| s.chosen), "both strikes chosen");
    e.choose(index_of(&req, play(0, 0, x)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(in_torpor(&e, a));
    assert_eq!(blood(&e, x), 4 - 2, "its cost, A never struck");
}

/// Backstab: "Only usable by a blocking vampire. Only usable on the first
/// round of combat." The acting A is never offered it.
#[test]
fn backstab_is_the_blockers_only() {
    let (mut e, a, x) = duel(&["cel"], &[]);
    hand(&mut e, 0, &["Backstab"]);
    bleed_blocked(&mut e, a, x);
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == a),
    );
}

/// Backstab [CEL]: "As above, with an additional strike (limited)." The
/// additional strike has no first strike: A, at 1 blood, empty after X's
/// first strike, still strikes back as X's additional strike sends it to
/// torpor.
#[test]
fn backstab_superior_strikes_again_without_first_strike() {
    let (mut e, a, x) = duel(&["cel", "cel"], &["cel", "cel"]);
    e.state.minions[a.index()].blood = 1;
    hand(&mut e, 0, &["Pursuit"]);
    hand(&mut e, 1, &["Backstab"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(in_torpor(&e, a));
    assert_eq!(blood(&e, x), 4 - 2 - 1 - 1, "its cost; two hand strikes");
}

// ---- Body Arsenal -----------------------------------------------------------

/// Body Arsenal [vic]: "Damage from this vampire's hand strikes is
/// aggravated this round." "Can be played after both strikes are chosen."
/// [RTR 19960112] Played before resolution: X's pending damage is
/// aggravated.
#[test]
fn body_arsenal_after_strikes_aggravates_the_hand_strike() {
    let (mut e, a, x) = duel(&["vic"], &["for"]);
    hand(&mut e, 0, &["Body Arsenal"]);
    hand(&mut e, 1, &["Resilience"]);
    bleed_blocked(&mut e, a, x);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatBeforeResolution);
    e.choose(index_of(&req, play(0, 0, a)));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (0, 1));
}

/// Body Arsenal [VIC]: "As above, and this vampire gets +1 strength this
/// combat." Before range: X's pending damage 2, aggravated.
#[test]
fn body_arsenal_superior_adds_strength() {
    let (mut e, a, x) = duel(&["vic", "vic"], &["for"]);
    hand(&mut e, 0, &["Body Arsenal"]);
    hand(&mut e, 1, &["Resilience"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (0, 2));
}

// ---- Bonecraft and Nimble Feet --------------------------------------------

/// Bonecraft [vic]: "Strike: 1 damage and put this card on the opposing
/// minion." "The rest of the effect applies if the damage is prevented, it
/// does not if it is dodged." [LSJ 19990106-2] X prevents the 1 with
/// Resilience: the card itself on X.
#[test]
fn bonecraft_prevented_still_goes_on_the_opponent() {
    let (mut e, a, x) = duel(&["vic"], &["for"]);
    hand(&mut e, 0, &["Bonecraft"]);
    hand(&mut e, 1, &["Resilience"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4, "prevented");
    placed_on(&e, "Bonecraft", Some(x));
}

/// [LSJ 19990106-2] Dodged (Side Strike [cel]): the card in A's ash heap,
/// nothing on X.
#[test]
fn bonecraft_dodged_stays_in_the_ash_heap() {
    let (mut e, a, x) = duel(&["vic"], &["cel"]);
    hand(&mut e, 0, &["Bonecraft"]);
    hand(&mut e, 1, &["Side Strike"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Bonecraft"));
    assert!(e.state.cards.iter().all(|c| !c.in_play));
}

/// Bonecraft: "The attached minion gets -1 strength (after the current
/// strike resolution step)." Nimble Feet [cel]: "Press." X's hand strike
/// lands for 1 in round 1, for none in round 2 after A's press.
#[test]
fn bonecraft_takes_strength_from_the_next_strike_on() {
    let (mut e, a, x) = duel(&["vic", "cel"], &[]);
    hand(&mut e, 0, &["Bonecraft", "Nimble Feet"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert_eq!(blood(&e, a), 4 - 1 - 1, "its cost; X's 1");
    e.choose(index_of(&req, play(0, 0, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1 - 1, "X's strength 0 in round 2");
    assert_eq!(blood(&e, x), 4 - 1 - 1, "Bonecraft; A's hand strike");
}

/// Bonecraft [VIC]: "the attached minion gets -2 strength." "The attached
/// minion can burn this card as a +1 stealth action that costs 2 blood
/// (allies cannot pay blood costs)." On A: its hand strike deals nothing;
/// A takes the action, paying 2, the card to its owner's ash heap.
#[test]
fn bonecraft_host_burns_it_for_two_blood() {
    let (mut e, a, x) = duel(&[], &[]);
    in_play_mode(&mut e, 1, "Bonecraft", Some(a), 1);
    stronger(&mut e, a, 1);
    bleed_blocked(&mut e, a, x);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4, "strength 2 less 2");
    let (mut e, a, _) = duel(&[], &[]);
    let card = in_play_mode(&mut e, 1, "Bonecraft", Some(a), 0);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let burn = granted(card.0, a);
    assert!(req.options.contains(&burn), "{:?}", req.options);
    e.choose(index_of(&req, burn));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[card.index()].in_play);
    assert!(ash_has(&e, 1, "Bonecraft"));
    assert_eq!(blood(&e, a), 4 - 2);
}

/// Nimble Feet [CEL]: "Additional strike (limited)."
#[test]
fn nimble_feet_superior_strikes_again() {
    let (mut e, a, x) = duel(&["cel", "cel"], &[]);
    hand(&mut e, 0, &["Nimble Feet"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2, "two hand strikes");
    assert_eq!(blood(&e, a), 4 - 1);
}

// ---- Burning Wrath ----------------------------------------------------------

/// Burning Wrath: "All the damage is aggravated." [LSJ 20020122] [pot]:
/// strength 1 at +1, 2 aggravated; [POT]: 3.
#[test]
fn burning_wrath_is_all_aggravated() {
    for (mode, levels, want) in [(0, &["pot"][..], 2), (1, &["pot", "pot"][..], 3)] {
        let (mut e, a, x) = duel(levels, &["for"]);
        e.state.minions[a.index()].blood = 4;
        hand(&mut e, 0, &["Burning Wrath"]);
        hand(&mut e, 1, &["Resilience"]);
        bleed_blocked(&mut e, a, x);
        pick_when(&mut e, play(0, mode, a));
        offering(&mut e, play(0, 0, x));
        assert_eq!(pending(&e, 1), (0, want));
    }
}

// ---- Disengage --------------------------------------------------------------

/// Disengage: "Alternatively, burn 1 blood to cancel a grapple card (such
/// as Immortal Grapple or Mighty Grapple) as it is played (no cost is paid
/// for that card)." [LSJ 20061207] [LSJ 20061013] played at once, in the
/// as-played window; [ANK 20210226] the blood an effect. X's Mighty Grapple
/// [POT] canceled: A burns 1, X's strike a hand strike; Disengage held
/// until after combat ("Do not replace until after combat").
#[test]
fn disengage_cancels_an_opposing_grapple_for_a_blood() {
    let (mut e, a, x) = duel(&["for"], &["pot", "pot"]);
    hand(&mut e, 0, &["Disengage", "Resilience"]);
    hand(&mut e, 1, &["Mighty Grapple"]);
    bleed_blocked(&mut e, a, x);
    e.state.seats[0].library = vec![find_library("Pursuit"); 3].into();
    pick_when(&mut e, play(0, 1, x));
    let req = expect(&mut e, SeatId(0), WindowLabel::CardAsPlayed);
    e.choose(index_of(&req, play(0, 1, a)));
    let req = offering(&mut e, play(0, 0, a));
    assert!(ash_has(&e, 1, "Mighty Grapple"), "still played");
    assert_eq!(e.state.seats[0].hand.len(), 1, "not replaced in combat");
    assert_eq!(pending(&e, 0), (1, 0), "X's hand strike");
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1 - 1, "its blood; X's hand strike");
    assert_eq!(blood(&e, x), 4 - 1, "A's hand strike, no grapple");
}

/// [ANK 20190104] [LSJ 19980212] "The canceled card has still been
/// played": Immortal Grapple canceled, X's second copy is not offered that
/// round ("A vampire can play only one Immortal Grapple each round").
#[test]
fn disengage_canceled_grapple_still_counts_as_played() {
    let (mut e, a, x) = duel(&[], &["pot"]);
    hand(&mut e, 0, &["Disengage"]);
    hand(&mut e, 1, &["Immortal Grapple", "Immortal Grapple"]);
    let grapple = play(0, 0, x);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, grapple);
    pick(&mut e, play(0, 1, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, grapple);
    assert!(ash_has(&e, 1, "Immortal Grapple"));
}

/// [ANK 20210226] "The card cannot be played if the minion cannot afford
/// to burn the blood." A empty: no cancel offered.
#[test]
fn disengage_cannot_cancel_without_the_blood() {
    let (mut e, a, x) = duel(&[], &["pot", "pot"]);
    hand(&mut e, 0, &["Disengage"]);
    hand(&mut e, 1, &["Mighty Grapple"]);
    pick(&mut e, bleed(a));
    e.state.minions[a.index()].blood = 0;
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 1, x));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
    assert!(e.state.seats[0].hand.contains(&find_library("Disengage")));
}

/// Disengage: "Press, only usable to end combat." Not offered in A's own
/// press step; after X's press to continue (Nimble Feet), it ends the
/// combat: no second round.
#[test]
fn disengage_presses_only_to_end() {
    let (mut e, a, x) = duel(&[], &["cel"]);
    hand(&mut e, 0, &["Disengage"]);
    hand(&mut e, 1, &["Nimble Feet"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    let req = expect(&mut e, SeatId(0), WindowLabel::CombatPress);
    e.choose(index_of(&req, play(0, 0, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 1, 4 - 1), "one round");
}

// ---- Optional presses -------------------------------------------------------

/// "The optional press can only be used during the current round." [TOM
/// 19960521] Dissolution [PRO] ("Maneuver, with an optional press"),
/// Kraken's Kiss [VIC], Mighty Grapple [POT], Shadow Strike [OBT]: the
/// press left unused in round 1, X continuing (Nimble Feet), none offered
/// to A in round 2.
#[test]
fn optional_presses_are_this_rounds_only() {
    for (card, disc, mode) in [
        ("Dissolution", "pro", 1),
        ("Kraken's Kiss", "vic", 1),
        ("Mighty Grapple", "pot", 1),
        ("Shadow Strike", "obt", 1),
    ] {
        let (mut e, a, x) = duel(&[disc, disc], &["cel"]);
        hand(&mut e, 0, &[card]);
        hand(&mut e, 1, &["Nimble Feet"]);
        bleed_blocked(&mut e, a, x);
        pick_when(&mut e, play(0, mode, a));
        // Round 1: A's press passed, X's press played, A's answer passed.
        let req = until(&mut e, SeatId(0), WindowLabel::CombatPress);
        assert!(req.options.contains(&Choice::UsePress), "{card}");
        e.choose(index_of(&req, Choice::Pass));
        pick_when(&mut e, play(0, 0, x));
        while e.combat().is_some_and(|c| c.round == 1) {
            let req = request(&mut e);
            e.choose(fallback(&req));
        }
        never_offered_until(
            &mut e,
            SeatId(0),
            WindowLabel::MinionPhase,
            Choice::UsePress,
        );
    }
}

// ---- Drain Essence ----------------------------------------------------------

/// Drain Essence [tha]: "Not usable on the first round of combat. Strike:
/// ranged; steal 2 blood." "Can target a minion with less blood or life
/// than the amount stolen." [RTR 20010711] Not offered in round 1; in
/// round 2, X at 1 blood loses it to A.
#[test]
fn drain_essence_steals_what_there_is_after_the_first_round() {
    let (mut e, a, x) = duel(&["tha", "cel"], &[]);
    e.state.minions[x.index()].blood = 2;
    hand(&mut e, 0, &["Drain Essence", "Nimble Feet"]);
    bleed_blocked(&mut e, a, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::CombatPress, play(0, 0, a));
    pick(&mut e, play(1, 0, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 0);
    assert!(!e.state.minions[x.index()].in_torpor, "no damage");
    assert_eq!(
        blood(&e, a),
        4 - 1 - 1 - 1 + 1,
        "X's hand strikes; its cost"
    );
}

// ---- Kraken's Kiss ----------------------------------------------------------

/// Kraken's Kiss: "If this striking minion takes 4 or more damage this
/// combat, they go to torpor." "Does not count the damage inflicted before
/// the card was played." [LSJ 20010610] A takes 2 a round from X: played in
/// round 2, A is still ready after it (2 since); in round 3, 4 since, A
/// goes to torpor with blood left.
#[test]
fn krakens_kiss_counts_only_damage_after_its_play() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), fixture_vampire(|c| c.capacity >= 9, 0), 9, false);
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    let x = e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 2),
        4,
        false,
    );
    e.state.minions[a.index()].gained_levels = vec!["vic", "cel"];
    stronger(&mut e, x, 1);
    hand(&mut e, 0, &["Kraken's Kiss", "Nimble Feet", "Nimble Feet"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(1, 0, a));
    pick_when(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert!(!e.state.minions[a.index()].in_torpor);
    assert_eq!(blood(&e, a), 9 - 2 - 1 - 2, "2 since its play");
    e.choose(index_of(&req, play(0, 0, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(in_torpor(&e, a));
    assert_eq!(blood(&e, a), 4 - 2, "blood left");
}

/// Kraken's Kiss played by an ally (the Satyr's stored card, "as if played
/// from your hand"): "If this striking minion takes 4 or more damage this
/// combat, they go to torpor." An effect sending an ally to torpor burns it
/// (rulebook, allies; digest §14.5, LSJ 20060409): the Brujah's 4-strength
/// hand strike burns the Satyr, life left.
#[test]
fn krakens_kiss_burns_an_ally_that_plays_it() {
    let (mut e, a, s, _, at) = satyr_blocks("Kraken's Kiss", &[]);
    stronger(&mut e, a, 3);
    pick_when(&mut e, play(at, 0, s));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[s.index()].in_play, "burned");
    assert!(ash_has(&e, 1, "Shadow Court Satyr"));
}

/// Kraken's Kiss [VIC]: "once each round this combat, this vampire can
/// strike, ranged: steal 2 blood or life (becoming blood)." "The strike to
/// steal 2 blood can be used on a minion with less than 2 blood or life."
/// [RTR 20010711] In round 2 (its press), the steal takes X's last blood.
#[test]
fn krakens_kiss_superior_steals_once_each_round() {
    let (mut e, a, x) = duel(&["vic", "vic"], &[]);
    e.state.minions[x.index()].blood = 3;
    hand(&mut e, 0, &["Kraken's Kiss"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, Choice::UsePress);
    let steal = Choice::RoundStrike { at: None };
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(req.options.contains(&steal), "{:?}", req.options);
    e.choose(index_of(&req, steal));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 0);
    assert_eq!(
        blood(&e, a),
        4 - 1 - 1 - 1 + 1,
        "its cost; two hand strikes"
    );
}

// ---- Meld with the Land -----------------------------------------------------

/// Meld with the Land [vic]: "Strike: combat ends, and unlock the opposing
/// minion (at any range) before combat ends." "The unlock effect occurs
/// when the strike resolves, before the combat ends." [RTR 19970630] [RTR
/// 20020501] A's strike: X, locked by its block, unlocked; A stays locked.
#[test]
fn meld_with_the_land_unlocks_the_opposing_minion() {
    let (mut e, a, x) = duel(&["vic"], &[]);
    hand(&mut e, 0, &["Meld with the Land"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[x.index()].locked);
    assert!(e.state.minions[a.index()].locked);
    assert_eq!((blood(&e, a), blood(&e, x)), (4, 4));
}

/// "The opponent still unlocks if an effect continues the combat or begins
/// a new combat." [RTR 20020501] [LSJ 20030717] X's Meld unlocks A;
/// Telepathic Tracking [AUS] continues the combat: A stays unlocked. A
/// dodging opponent is not unlocked (a dodge's protection, LSJ 19980526).
#[test]
fn meld_with_the_land_unlock_survives_a_continued_combat() {
    let (mut e, a, x) = duel(&["aus", "aus"], &["vic"]);
    hand(&mut e, 0, &["Telepathic Tracking"]);
    hand(&mut e, 1, &["Meld with the Land"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 1, a));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked);
    assert_eq!(blood(&e, x), 4 - 1, "round 2 fought");

    let (mut e, a, x) = duel(&["cel"], &["vic"]);
    hand(&mut e, 0, &["Side Strike"]);
    hand(&mut e, 1, &["Meld with the Land"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, x));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].locked, "dodged");
}

// ---- Mighty Grapple ---------------------------------------------------------

/// Mighty Grapple [POT]: "use a melee weapon strike. This strike is at +1
/// damage." "Additional damage inherits all of the properties of the base
/// damage." [RTR 19970630] [TOM 19960225] The Femur of Toomler's
/// strength+1 aggravated, +1: 3 aggravated.
#[test]
fn mighty_grapple_superior_weapon_strike_inherits_its_kind() {
    let (mut e, a, x) = duel(&["pot", "pot"], &["for"]);
    e.state.cards.push(CardInPlay::new(
        find_library("Femur of Toomler"),
        SeatId(0),
        Some(a),
        0,
    ));
    let femur = CardId(e.state.cards.len() as u32 - 1);
    hand(&mut e, 0, &["Mighty Grapple"]);
    hand(&mut e, 1, &["Resilience"]);
    bleed_blocked(&mut e, a, x);
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 2,
            actor: Some(a),
            target: Target::Card(femur),
        },
    );
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (0, 3));
}

/// Mighty Grapple [pot]: "Press, only usable to continue combat." A's
/// press continues: a second round fought.
#[test]
fn mighty_grapple_basic_presses_to_continue() {
    let (mut e, a, x) = duel(&["pot"], &[]);
    hand(&mut e, 0, &["Mighty Grapple"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 2, 4 - 2), "two rounds");
}

// ---- Resilience -------------------------------------------------------------

/// Resilience [FOR]: "Prevent 3 non-aggravated damage." "Cannot be used to
/// prevent aggravated damage, even if the minion treats them as normal
/// damage." [LSJ 20040812-2] [PIB 20130327] Against Burning Wrath's 2
/// aggravated: [for] offered, [FOR] not.
#[test]
fn resilience_superior_prevents_normal_damage_only() {
    let (mut e, a, x) = duel(&["pot"], &["for", "for"]);
    hand(&mut e, 0, &["Burning Wrath"]);
    hand(&mut e, 1, &["Resilience"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    let req = offering(&mut e, play(0, 0, x));
    assert!(!req.options.contains(&play(0, 1, x)));
    e.choose(index_of(&req, play(0, 0, x)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(in_torpor(&e, x), "1 aggravated left");
    assert_eq!(blood(&e, x), 4, "aggravated damage is not mended");
}

/// "If fewer points of (preventable) damage are being resolved, then the
/// effect prevents all of those points." [RTR 20041202] "Unused prevention
/// points can't be carried over." [ANK 20200318] [LSJ 20001114] [FOR] on
/// A's 1: none left for A's additional strike (Nimble Feet [CEL]).
#[test]
fn resilience_superior_prevents_fewer_and_carries_nothing() {
    let (mut e, a, x) = duel(&["cel", "cel"], &["for", "for"]);
    hand(&mut e, 0, &["Nimble Feet"]);
    hand(&mut e, 1, &["Resilience"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    assert_eq!(blood(&e, x), 4, "all of the 1");
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 1, "the additional strike's 1");
}

/// "Cannot be used if there is no damage to prevent." [LSJ 20001114] A's
/// dodge: X never offered Resilience.
#[test]
fn resilience_needs_damage_to_prevent() {
    let (mut e, a, x) = duel(&["cel"], &["for"]);
    hand(&mut e, 0, &["Side Strike"]);
    hand(&mut e, 1, &["Resilience"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == x),
    );
}

// ---- Shadow Step ------------------------------------------------------------

/// Shadow Step [OBT]: "Only usable before range is determined. Burn 2
/// blood to set the range for this round." "The burn blood effect is not
/// reduced"; "cannot be played if the minion cannot afford to burn the
/// blood." [ANK 20210226] "Once the range is set, no other effect can be
/// used to reset the range that round", the determine range step skipped.
/// [RTR 19970630] [ANK 20180720] A sets long: X's maneuver never offered,
/// the hand strikes miss. At 1 blood, not offered.
#[test]
fn shadow_step_sets_the_range_for_two_blood() {
    let (mut e, a, x) = duel(&["obt", "obt"], &["cel"]);
    hand(&mut e, 0, &["Shadow Step"]);
    hand(&mut e, 1, &["Pursuit"]);
    bleed_blocked(&mut e, a, x);
    let req = expect(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    e.choose(index_of(&req, play(0, 2, a)));
    request(&mut e);
    assert_eq!(blood(&e, a), 4 - 2);
    assert_eq!(e.combat().expect("combat").range, Range::Long);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
    assert_eq!((blood(&e, a), blood(&e, x)), (2, 4), "out of reach");

    let (mut e, a, x) = duel(&["obt", "obt"], &[]);
    hand(&mut e, 0, &["Shadow Step"]);
    pick(&mut e, bleed(a));
    e.state.minions[a.index()].blood = 1;
    pick_when(&mut e, block(x));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { mode: 1 | 2, .. })
    });
}

/// "If another effect was already setting that round's range, that effect
/// has priority." [PIB 20120214] X sets close first: A's never offered.
#[test]
fn shadow_step_yields_to_a_range_already_set() {
    let (mut e, a, x) = duel(&["obt", "obt"], &["obt", "obt"]);
    hand(&mut e, 0, &["Shadow Step"]);
    hand(&mut e, 1, &["Shadow Step"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), mode: 1 | 2, .. } if *m == a),
    );
    assert_eq!(blood(&e, a), 4 - 1, "close: X's hand strike");
}

// ---- Shadow Strike and Skin Trap --------------------------------------------

/// Shadow Strike [obt]: "Strike: strength ranged damage, with 1 optional
/// maneuver." Played at the maneuver to long range, the strike committed:
/// X takes 1, A nothing.
#[test]
fn shadow_strike_maneuvers_and_commits_its_strike() {
    let (mut e, a, x) = duel(&["obt"], &[]);
    hand(&mut e, 0, &["Shadow Strike"]);
    bleed_blocked(&mut e, a, x);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    e.choose(index_of(&req, play(0, 0, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 1, 4 - 1), "its cost; 1R");
}

/// "The optional maneuver cannot be used if the strike cannot be used."
/// [LSJ 20021028] Skin Trap [VIC]: "the opposing minion cannot strike this
/// round unless they burn 1 blood immediately." A declines to burn: no
/// strike this round, so no Shadow Strike at the maneuver.
#[test]
fn shadow_strike_maneuver_needs_a_usable_strike() {
    let (mut e, a, x) = duel(&["obt"], &["vic", "vic"]);
    hand(&mut e, 0, &["Shadow Strike"]);
    hand(&mut e, 1, &["Skin Trap"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Pass, Choice::BurnBloodToAvoid]);
    e.choose(0);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
    assert_eq!(blood(&e, x), 4 - 1, "Skin Trap's cost, A never struck");
    assert_eq!(blood(&e, a), 4 - 1, "X's hand strike");
}

/// Skin Trap [VIC]: A burns its 1 blood: it strikes. [vic]: "The opposing
/// minion cannot dodge this round": X's Side Strike never offered.
#[test]
fn skin_trap_superior_burns_a_blood_to_strike_and_bars_the_dodge() {
    let (mut e, a, x) = duel(&["cel"], &["vic", "vic"]);
    hand(&mut e, 0, &["Side Strike"]);
    hand(&mut e, 1, &["Skin Trap"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    pick(&mut e, Choice::BurnBloodToAvoid);
    assert_eq!(blood(&e, a), 4 - 1);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
    assert_eq!(blood(&e, x), 4 - 1 - 1, "its cost; A's hand strike");
}

/// Skin Trap: "A vampire can play only one Skin Trap each round." A plays
/// one [vic]; its second copy is offered in neither mode that round (the
/// combat's only round).
#[test]
fn skin_trap_one_per_vampire_each_round() {
    let (mut e, a, x) = duel(&["vic", "vic"], &[]);
    hand(&mut e, 0, &["Skin Trap", "Skin Trap"]);
    bleed_blocked(&mut e, a, x);
    let req = expect(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    assert!(req.options.contains(&play(0, 1, a)));
    e.choose(index_of(&req, play(0, 0, a)));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == a),
    );
    assert!(e.state.seats[0].hand.contains(&find_library("Skin Trap")));
}

/// "(allies cannot burn blood)": an ally blocking, no question asked, no
/// strike this round.
#[test]
fn skin_trap_superior_bars_an_allys_strike() {
    let (mut e, a, _) = duel(&["vic", "vic"], &[]);
    let ally = e.add_ally(SeatId(1), find_library("47th Street Royals"));
    hand(&mut e, 0, &["Skin Trap"]);
    bleed_blocked(&mut e, a, ally);
    pick_when(&mut e, play(0, 1, a));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        *o == Choice::BurnBloodToAvoid
    });
    assert_eq!(blood(&e, a), 4 - 1, "its cost only");
}

// ---- Song of Serenity -------------------------------------------------------

/// Song of Serenity [ani]: "The opposing minion gets -1 strength this
/// round." X at strength 2: 1 in round 1, 2 in round 2 (A's press).
#[test]
fn song_of_serenity_weakens_for_the_round() {
    let (mut e, a, x) = duel(&["ani", "cel"], &[]);
    stronger(&mut e, x, 1);
    hand(&mut e, 0, &["Song of Serenity", "Nimble Feet"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1 - 2);
}

/// Song of Serenity [ANI]: "As above, but for the remainder of combat." "A
/// vampire may play only one Song of Serenity each combat": X at strength
/// 1 both rounds; A's second copy never offered in round 2.
#[test]
fn song_of_serenity_superior_lasts_the_combat_once() {
    let (mut e, a, x) = duel(&["ani", "ani", "cel"], &[]);
    stronger(&mut e, x, 1);
    hand(
        &mut e,
        0,
        &["Song of Serenity", "Song of Serenity", "Nimble Feet"],
    );
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(1, 0, a));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == a),
    );
    assert_eq!(blood(&e, a), 4 - 1 - 1);
}

// ---- Reality Mirror ---------------------------------------------------------

/// A bleeds, blocked by X, a Malkavian antitribu (Reality Mirror's clan)
/// with `x_levels`; seat 0's ash heap holds `ash`.
fn mirror_duel(x_levels: &[&'static str], ash: &[&str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity);
    let a = e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    e.add_minion(SeatId(0), textless(mid, 1), 4, false);
    let malk = textless(|c| mid(c) && c.clan == "Malkavian antitribu", 0);
    let x = e.add_minion(SeatId(1), malk, 4, false);
    e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.state.minions[x.index()].gained_levels = x_levels.to_vec();
    e.state.seats[0].ash = ash
        .iter()
        .map(|n| find_library(n))
        .collect::<Vec<_>>()
        .into();
    (e, a, x)
}

/// Reality Mirror: "Play a combat card from the opposing minion's
/// controller's ash heap as if from your hand. If the card requires a
/// Discipline this vampire does not have, you can play the card as if this
/// vampire had the basic level of 1 of the Discipline(s) required." X,
/// without Vicissitude nor Animalism, mirrors A's Body Arsenal: the [vic]
/// and [ani] modes of A's two cards offered (its next decision), no
/// superior one; Body Arsenal's cost paid, back in A's ash heap.
#[test]
fn reality_mirror_plays_an_opposing_ash_card_lending_a_basic_discipline() {
    let (mut e, a, x) = mirror_duel(&[], &["Body Arsenal", "Song of Serenity"]);
    hand(&mut e, 1, &["Reality Mirror"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::CombatBeforeRange)
    );
    assert_eq!(
        plays(&req),
        vec![play(0, 0, x), play(1, 0, x)],
        "basic modes only"
    );
    e.choose(index_of(&req, play(0, 0, x)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(in_torpor(&e, a), "X's aggravated hand strike");
    assert_eq!(blood(&e, x), 4 - 2 - 1, "Body Arsenal's cost; A's strike");
    assert!(ash_has(&e, 0, "Body Arsenal"), "its owner's");
    assert!(ash_has(&e, 1, "Reality Mirror"));
}

/// "Cannot be used to bypass restrictions (like Clan restriction or any
/// other play restriction)." [LSJ 20020426] A's Raking Talons (Gargoyle)
/// is no card X can play: Reality Mirror is never offered (owner,
/// 2026-11-17: rules-qa 122.215). "Only one Reality Mirror can be played
/// in a game": against A's Body Arsenal, X's second copy never offered.
#[test]
fn reality_mirror_keeps_clan_restrictions_and_is_once_a_game() {
    let (mut e, a, x) = mirror_duel(&[], &["Raking Talons"]);
    hand(&mut e, 1, &["Reality Mirror"]);
    bleed_blocked(&mut e, a, x);
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == x),
    );
    assert!(ash_has(&e, 0, "Raking Talons"));

    let (mut e, a, x) = mirror_duel(&[], &["Body Arsenal"]);
    hand(&mut e, 1, &["Reality Mirror", "Reality Mirror"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == x),
    );
    assert!(ash_has(&e, 0, "Body Arsenal"), "played, back to its owner");
    assert_eq!(e.state.seats[1].hand.len(), 1);
}

/// Reality Mirror is timed by the card it plays (owner, 2026-11-17:
/// rules-qa 122.155, 122.215): A's Burning Wrath [pot] in the ash heap, X
/// (no Potence) is first offered the Mirror as it chooses its strike, the
/// Wrath its strike, basic Potence lent: A takes 2 aggravated, to torpor.
#[test]
fn reality_mirror_mirrors_a_strike_card_as_its_strike() {
    let (mut e, a, x) = mirror_duel(&[], &["Burning Wrath"]);
    e.state.minions[x.index()].blood = 4;
    hand(&mut e, 1, &["Reality Mirror"]);
    bleed_blocked(&mut e, a, x);
    let req = offering(&mut e, play(0, 0, x));
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::CombatStrike)
    );
    assert!(
        req.options.contains(&Choice::HandStrike),
        "before the strike"
    );
    e.choose(index_of(&req, play(0, 0, x)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(in_torpor(&e, a), "2 aggravated");
    assert_eq!(blood(&e, x), 4 - 3 - 1, "the Wrath's cost; A's strike");
    assert!(ash_has(&e, 0, "Burning Wrath"), "its owner's");
}

/// "As if this vampire had the basic level of 1 of the Discipline(s)
/// required": X, with Obfuscate, mirrors Shadow Feint [cel][obf], Celerity
/// lent (beside Body Arsenal, for a decision).
#[test]
fn reality_mirror_lends_one_missing_discipline_of_a_combination() {
    let (mut e, a, x) = mirror_duel(&["obf"], &["Body Arsenal", "Shadow Feint"]);
    hand(&mut e, 1, &["Reality Mirror"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    let req = request(&mut e);
    assert_eq!(
        plays(&req),
        vec![play(0, 0, x), play(1, 0, x)],
        "{:?}",
        req.options
    );
}

/// Disengage: "A card played 'as if from hand' can be canceled 'as
/// played'." [LSJ 20060106] X mirrors A's Immortal Grapple (basic Potence
/// lent); A's Disengage cancels it, the grapple back in A's ash heap.
#[test]
fn disengage_cancels_a_grapple_played_as_if_from_hand() {
    let (mut e, a, x) = mirror_duel(&[], &["Immortal Grapple"]);
    hand(&mut e, 0, &["Disengage"]);
    hand(&mut e, 1, &["Reality Mirror"]);
    bleed_blocked(&mut e, a, x);
    let req = until(&mut e, SeatId(1), WindowLabel::CombatBeforeStrikes);
    e.choose(index_of(&req, play(0, 0, x)));
    // The grapple, X's one option, elided: its as-played window.
    let req = expect(&mut e, SeatId(0), WindowLabel::CardAsPlayed);
    e.choose(index_of(&req, play(0, 1, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Immortal Grapple"));
    assert_eq!(blood(&e, a), 4 - 1 - 1, "its blood; X's hand strike");
}

// ---- Abandoning the Flesh ---------------------------------------------------

/// Seat 0's A (4 blood) and a second vampire; seat 1's X, with Dementation,
/// in torpor with 2 blood, and a second vampire; seat 1 holds Abandoning
/// the Flesh.
fn torpid_dementation() -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 8, 0), 4, false);
    e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(mid, 1), 2, true);
    e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["dem"];
    hand(&mut e, 1, &["Abandoning the Flesh"]);
    (e, a, x)
}

fn diablerize(a: MinionId, x: MinionId) -> Choice {
    Choice::TakeAction {
        minion: a,
        action: ActionKind::Diablerie,
        target: Some(x),
    }
}

/// Abandoning the Flesh: "Only usable if this vampire would be burned.
/// Usable by a vampire in torpor. [dem] Instead, remove this vampire from
/// the game (diablerie, if any, is canceled) and put this card in play."
/// A diablerizes the torpid X: X's controller is asked before the
/// diablerie's steps (advanced rules §3.7.8) and plays it, as a reaction:
/// X removed, the card itself in play; the diablerie unsuccessful (rules-qa
/// 121.20): no blood, no diablerie recorded.
#[test]
fn abandoning_the_flesh_saves_a_torpid_vampire_from_diablerie() {
    let (mut e, a, x) = torpid_dementation();
    let x_card = e.state.minions[x.index()].crypt_card().expect("a vampire");
    pick(&mut e, diablerize(a, x));
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Pass, play(0, 0, x)]);
    e.choose(1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[x.index()].in_play);
    assert!(e.state.seats[1].removed_crypt.contains(&x_card));
    assert!(!e.state.seats[1].ash_crypt.contains(&x_card));
    let card = placed_on(&e, "Abandoning the Flesh", None);
    assert_eq!(e.state.card_controller(card), SeatId(1));
    assert_eq!(blood(&e, a), 4, "no blood taken");
    assert!(e.state.minions[a.index()].diableries.0.is_empty());
}

/// Declined, the diablerie goes on: X burned, its blood to A.
#[test]
fn abandoning_the_flesh_declined_lets_the_diablerie_happen() {
    let (mut e, a, x) = torpid_dementation();
    let x_card = e.state.minions[x.index()].crypt_card().expect("a vampire");
    pick(&mut e, diablerize(a, x));
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ash_crypt.contains(&x_card), "burned");
    assert_eq!(e.state.minions[a.index()].diableries.0.len(), 1);
    assert_eq!(blood(&e, a), 4 + 2);
}

/// "A Methuselah can have only one Abandoning the Flesh": one in play, a
/// second is not offered; the diablerie goes on.
#[test]
fn abandoning_the_flesh_one_per_methuselah() {
    let (mut e, a, x) = torpid_dementation();
    in_play_mode(&mut e, 1, "Abandoning the Flesh", None, 0);
    pick(&mut e, diablerize(a, x));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
    assert!(!e.state.minions[x.index()].in_play);
    assert_eq!(e.state.minions[a.index()].diableries.0.len(), 1);
}

/// A standing replacement applying (Charigger's) applies, the acting
/// Methuselah's choice (owner, rules-qa 122.2, 121.19): Abandoning the
/// Flesh is never offered; X removed by Charigger, a soul counter on it,
/// the diablerie unsuccessful (rules-qa 121.20), the card still in hand.
#[test]
fn charigger_takes_the_burn_before_abandoning_the_flesh() {
    let (mut e, a, x) = torpid_dementation();
    let x_card = e.state.minions[x.index()].crypt_card().expect("a vampire");
    let axe = in_play_mode(&mut e, 0, "Charigger, The Axe", Some(a), 0);
    pick(&mut e, diablerize(a, x));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
    assert!(!e.state.minions[x.index()].in_play);
    assert!(e.state.seats[1].removed_crypt.contains(&x_card));
    assert_eq!(e.state.cards[axe.index()].counters, 1);
    assert!(e.state.seats[1]
        .hand
        .contains(&find_library("Abandoning the Flesh")));
    assert!(e.state.minions[a.index()].diableries.0.is_empty());
}

/// "Will not trigger the Soul Gem of Etrius since the vampire is removed
/// from the game instead of being burned." [LSJ 20021010] X, empty,
/// blocking, burned by Burning Wrath's 2 aggravated, plays it as a combat
/// card: removed, the Gem draws nothing; the combat over.
#[test]
fn abandoning_the_flesh_in_combat_draws_no_soul_gem() {
    let (mut e, a, x) = duel(&["pot"], &["dem"]);
    e.state.minions[x.index()].blood = 0;
    in_play_mode(&mut e, 1, "Soul Gem of Etrius", Some(x), 0);
    e.state.seats[1].crypt = vec![textless(|c| (4..=8).contains(&c.capacity), 5)].into();
    hand(&mut e, 0, &["Burning Wrath"]);
    hand(&mut e, 1, &["Abandoning the Flesh"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    e.choose(index_of(&req, play(0, 0, x)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.combat().is_none());
    assert!(!e.state.minions[x.index()].in_play);
    assert_eq!(e.state.seats[1].crypt.len(), 1, "no Soul Gem draw");
    assert!(ash_has(&e, 1, "Soul Gem of Etrius"));
    placed_on(&e, "Abandoning the Flesh", None);
}

/// Two vampires burned together, each its controller's card asked: A
/// bleeds with 3 blood, X, empty, blocks with aggravated hands of
/// strength 2; A's Burning Wrath (3 blood) empties him; they burn in one
/// strike resolution. Each Methuselah holds Abandoning the Flesh and is asked for
/// their own vampire in turn, the second asked once the first answered.
/// Seat 0 declines (A burned), seat 1 plays it (X removed).
#[test]
fn abandoning_the_flesh_is_asked_for_each_vampire_burned_together() {
    let (mut e, a, x) = duel(&["pot", "dem"], &["dem"]);
    e.state.minions[a.index()].blood = 3;
    e.state.minions[x.index()].blood = 0;
    stronger(&mut e, x, 1);
    e.state.minions[x.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    let (a_card, x_card) = (
        e.state.minions[a.index()].crypt_card().expect("a vampire"),
        e.state.minions[x.index()].crypt_card().expect("a vampire"),
    );
    hand(&mut e, 0, &["Burning Wrath", "Abandoning the Flesh"]);
    hand(&mut e, 1, &["Abandoning the Flesh"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    let mut asked = Vec::new();
    for _ in 0..100 {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        if req.window == WindowLabel::ResolutionChoice {
            asked.push((req.seat, req.options.clone()));
            let pick = if req.seat == SeatId(1) {
                index_of(&req, play(0, 0, x))
            } else {
                index_of(&req, Choice::Pass)
            };
            e.choose(pick);
            continue;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(
        asked,
        vec![
            (SeatId(0), vec![Choice::Pass, play(0, 0, a)]),
            (SeatId(1), vec![Choice::Pass, play(0, 0, x)]),
        ]
    );
    assert!(e.state.seats[0].ash_crypt.contains(&a_card), "A burned");
    assert!(
        e.state.seats[1].removed_crypt.contains(&x_card),
        "X removed"
    );
    assert!(!e.state.seats[1].ash_crypt.contains(&x_card));
    let card = placed_on(&e, "Abandoning the Flesh", None);
    assert_eq!(e.state.card_controller(card), SeatId(1));
}

/// "Cannot be played when burned as a result of a Blood Hunt referendum."
/// [LSJ 20070417] X, the blocker, diablerizes A with Amaranth in the
/// combat; seat 0's prince passes the blood hunt: X, a combatant still,
/// burns, never offered the card.
#[test]
fn abandoning_the_flesh_is_not_offered_against_a_blood_hunt() {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity);
    let a = e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    let p = e.add_minion(SeatId(0), plain("Ventrue", 2), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    let x = e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.add_minion(SeatId(1), textless(mid, 3), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["dem"];
    e.state.minions[a.index()].blood = 1;
    stronger(&mut e, x, 1);
    let x_card = e.state.minions[x.index()].crypt_card().expect("a vampire");
    hand(&mut e, 1, &["Amaranth", "Abandoning the Flesh"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    let mut voted = false;
    for _ in 0..300 {
        let req = request(&mut e);
        assert!(
            !req.options
                .iter()
                .any(|o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == x)),
            "{req:?}"
        );
        if req.options.contains(&votes(p, true)) {
            voted = true;
            e.choose(index_of(&req, votes(p, true)));
            continue;
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(voted);
    assert!(e.state.seats[1].ash_crypt.contains(&x_card), "burned");
}

/// "You can lock this card during a bleed action to give an acting vampire
/// with Dementation +1 bleed." "The ability to increase bleed can only be
/// used during a bleed action before Action Resolution." [ANK 20231124]
/// [LSJ 19970718] A ([dem]) bleeds for 2, the card locked; the second
/// vampire, without Dementation, is never offered it.
#[test]
fn abandoning_the_flesh_lock_gives_an_acting_dementation_vampire_bleed() {
    let (mut e, a, _) = duel(&["dem"], &[]);
    let card = in_play_mode(&mut e, 0, "Abandoning the Flesh", None, 0);
    let lock = ability(card.0, 0);
    pick(&mut e, bleed(a));
    pick_when(&mut e, lock);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 2);
    assert!(e.state.cards[card.index()].locked);

    let (mut e, _, _) = duel(&[], &[]);
    let card = in_play_mode(&mut e, 0, "Abandoning the Flesh", None, 0);
    pick(&mut e, bleed(MinionId(0)));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        ability(card.0, 0),
    );
    assert_eq!(pool(&e)[1], 10 - 1);
}

/// Amaranth's diablerie in combat: A, the acting [dem] vampire going down,
/// asked before it (a combat card): removed; X takes no blood, no
/// diablerie recorded.
#[test]
fn abandoning_the_flesh_cancels_a_diablerie_in_combat() {
    let (mut e, a, x) = duel(&["dem"], &[]);
    e.state.minions[a.index()].blood = 1;
    stronger(&mut e, x, 1);
    hand(&mut e, 0, &["Abandoning the Flesh"]);
    hand(&mut e, 1, &["Amaranth"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    e.choose(index_of(&req, play(0, 0, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].in_play);
    assert!(e.state.minions[x.index()].diableries.0.is_empty());
    assert_eq!(blood(&e, x), 4 - 1, "A's hand strike; nothing taken");
    placed_on(&e, "Abandoning the Flesh", None);
}

/// Sacrificial Lamb [OBF] on another Methuselah's torpid vampire: a burn,
/// answered as a reaction: removed, the acting vampire gains nothing.
#[test]
fn abandoning_the_flesh_answers_sacrificial_lamb() {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity);
    let a = e.add_minion(SeatId(0), vampire_with("OBF", 0), 6, false);
    e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    let t = e.add_minion(SeatId(1), textless(mid, 1), 0, true);
    e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.state.minions[t.index()].gained_levels = vec!["dem"];
    hand(&mut e, 0, &["Sacrificial Lamb"]);
    hand(&mut e, 1, &["Abandoning the Flesh"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 1,
            actor: Some(a),
            target: Target::Minion(t),
        },
    );
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    e.choose(index_of(&req, play(0, 0, t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_play);
    let card = e.state.minions[t.index()].crypt_card().expect("a vampire");
    assert!(e.state.seats[1].removed_crypt.contains(&card));
    assert_eq!(blood(&e, a), 6 - 3, "its cost, no capacity gained");
}

// ---- Equipment --------------------------------------------------------------

/// `name` in play on `host`, for its controller, with its equip mode's
/// layers.
fn equipped(e: &mut Engine, name: &str, host: MinionId) -> CardId {
    let seat = e.state.minions[host.index()].owner.0;
    in_play_mode(e, seat, name, Some(host), 0)
}

/// The bearer's use of ability `ability` of the card in play `w`.
fn weapon(w: CardId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(w),
        ability,
        actor: None,
        target: Target::None,
    }
}

/// Black Gloves: "Strength+1 aggravated damage each strike. Bearer takes 1
/// damage during strike resolution when this weapon is used, but only once
/// each combat." "Damage done to the bearer is environmental." [LSJ
/// 19970801] A strikes X for 2 aggravated and takes 1 beside X's hand
/// strike; X prevents it (Skin of Steel), and A's Blur strike with the
/// Gloves hurts A no more.
#[test]
fn black_gloves_strike_aggravated_and_hurt_the_bearer_once_each_combat() {
    let (mut e, a, x) = duel(&["cel"], &["for"]);
    let gloves = equipped(&mut e, "Black Gloves", a);
    hand(&mut e, 0, &["Blur"]);
    hand(&mut e, 1, &["Skin of Steel"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, weapon(gloves, 0));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (0, 2), "strength+1 aggravated");
    assert_eq!(pending(&e, 0), (2, 0), "X's hand strike and the Gloves'");
    pick(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, weapon(gloves, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1 - 2, "Blur; 1 and 1, not 1 more");
    assert!(in_torpor(&e, x), "the second strike's 2 aggravated");
}

/// Brass Knuckles: "Strength+1 damage each strike, only usable once each
/// combat." "A second copy allows a second use in the same combat." [ANK
/// 20230316] After the first copy's strike, Blur's additional strike offers
/// the second copy, not the first.
#[test]
fn a_second_brass_knuckles_strikes_again_in_the_combat() {
    let (mut e, a, x) = duel(&["cel"], &[]);
    let k1 = equipped(&mut e, "Brass Knuckles", a);
    let k2 = equipped(&mut e, "Brass Knuckles", a);
    hand(&mut e, 0, &["Blur"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, weapon(k1, 0));
    pick_when(&mut e, play(0, 0, a));
    let req = offering(&mut e, weapon(k2, 0));
    assert!(!req.options.contains(&weapon(k1, 0)));
    e.choose(index_of(&req, weapon(k2, 0)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 0, "2 and 2 on 4 blood");
}

/// Combat Shotgun: "3R damage each strike, only usable once each round."
/// "A second copy allows a second use in the same round." [ANK 20230316]
/// After the first copy's strike, Blur's additional strike offers the
/// second copy, not the first.
#[test]
fn a_second_combat_shotgun_fires_again_in_the_round() {
    let (mut e, a, x) = duel(&["cel"], &[]);
    e.state.minions[x.index()].blood = 6;
    let s1 = equipped(&mut e, "Combat Shotgun", a);
    let s2 = equipped(&mut e, "Combat Shotgun", a);
    hand(&mut e, 0, &["Blur"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, weapon(s1, 0));
    pick_when(&mut e, play(0, 0, a));
    let req = offering(&mut e, weapon(s2, 0));
    assert!(!req.options.contains(&weapon(s1, 0)));
    e.choose(index_of(&req, weapon(s2, 0)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 0, "3 and 3 on 6 blood");
}

/// Concealed Weapon: "The weapon cannot cost 3 or more pool." "Cost
/// modifications that are not limited to cards 'played' are considered
/// before checking for card cost." [LSJ 20040701] [LSJ 20080125] [ANK
/// 20181215] Jennifer "Flame61" Vidisania ("Weapons cost Jennifer 1 less
/// pool or blood") equips a Combat Shotgun with it for 2 pool; a plain
/// vampire is not offered Concealed Weapon for it.
#[test]
fn concealed_weapon_puts_a_combat_shotgun_on_jennifer_vidisania() {
    let shotgun = find_library("Combat Shotgun");
    let (mut e, a, x) = duel(&[], &[]);
    hand(&mut e, 0, &["Concealed Weapon", "Combat Shotgun"]);
    bleed_blocked(&mut e, a, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));

    let mut e = game(2);
    let c = find_card("Jennifer \"Flame61\" Vidisania");
    let j = e.add_minion(SeatId(0), c, CRYPT[c as usize].capacity, false);
    e.add_minion(SeatId(0), of_clan("Ventrue", 4..=8, 0), 4, false);
    let x = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 1), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 2), 4, false);
    hand(&mut e, 0, &["Concealed Weapon", "Combat Shotgun"]);
    bleed_blocked(&mut e, j, x);
    // The Shotgun, the one card it may name: its terms elided.
    pick_when(&mut e, play(0, 0, j));
    let req = offering(&mut e, Choice::HandStrike);
    let gun = placed_on(&e, "Combat Shotgun", Some(j));
    assert!(req.options.contains(&weapon(gun, 0)));
    assert!(!e.state.seats[0].hand.contains(&shotgun));
    assert_eq!(e.state.seats[0].pool, 10 - 2, "3 pool, 1 less");
}

/// Codex of the Damned: "While the bearer has Blood Sorcery [tha], he or
/// she is considered to have superior Blood Sorcery [THA]." Basic, then
/// superior with it; nothing on a bearer without it; back to basic once
/// its bearer burns a Vial of Garou Blood ("cannot use other equipment
/// cards": rules-qa 76.16).
#[test]
fn codex_of_the_damned_raises_blood_sorcery_to_superior() {
    let (mut e, a, x) = duel(&["tha"], &["for"]);
    hand(&mut e, 1, &["Skin of Steel"]);
    let no_tha =
        |c: &zvtes::crypt::CryptCard| !c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("tha"));
    assert!(no_tha(
        &CRYPT[e.state.minions[a.index()].crypt_card().unwrap() as usize]
    ));
    assert!(no_tha(
        &CRYPT[e.state.minions[x.index()].crypt_card().unwrap() as usize]
    ));
    assert_eq!(e.state.discipline_level(a, "tha"), 1);
    equipped(&mut e, "Codex of the Damned", a);
    equipped(&mut e, "Codex of the Damned", x);
    assert!(e.state.has_discipline(a, "THA"));
    assert_eq!(e.state.discipline_level(x, "tha"), 0, "nothing to raise");
    let vial = equipped(&mut e, "Vial of Garou Blood", a);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, weapon(vial, 0));
    offering(&mut e, play(0, 0, x));
    assert_eq!(e.state.discipline_level(a, "tha"), 1, "barred");
}

/// Codex of the Damned: "Each time the bearer with Blood Sorcery commits
/// diablerie, each other ready vampire you control gains 1 blood, and each
/// one younger than the victim may receive a master: Discipline card from
/// your ash heap, hand, or library (shuffle afterward)." A (capacity 11)
/// diablerizes V (capacity 8): Y (capacity 4) gains 1 and takes Potence
/// from hand; Z (capacity 9) gains 1, no search; the torpid T nothing.
#[test]
fn codex_of_the_damned_feeds_and_teaches_after_diablerie() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 10, 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tha"];
    let y = e.add_minion(SeatId(0), textless(|c| c.capacity == 4, 0), 2, false);
    let z = e.add_minion(SeatId(0), textless(|c| c.capacity == 9, 0), 2, false);
    let t = e.add_minion(SeatId(0), textless(|c| c.capacity == 4, 1), 2, true);
    let v = e.add_minion(SeatId(1), textless(|c| c.capacity == 8, 0), 2, true);
    e.add_minion(SeatId(1), textless(|c| c.capacity == 5, 0), 4, false);
    equipped(&mut e, "Codex of the Damned", a);
    hand(&mut e, 0, &["Potence"]);
    e.state.seats[1].has_edge = true;
    pick(&mut e, diablerize(a, v));
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    let potence = Choice::TakeFrom {
        zone: zvtes::decision::Zone::Hand,
        card: find_library("Potence"),
    };
    assert!(req.options.contains(&Choice::Pass));
    e.choose(index_of(&req, potence));
    to_referendum(&mut e);
    assert_eq!((blood(&e, y), blood(&e, z), blood(&e, t)), (3, 3, 2));
    assert_eq!(blood(&e, a), 4 + 2, "the victim's");
    assert_eq!(e.state.discipline_level(y, "pot"), 1);
    assert!(!e.state.seats[0].hand.contains(&find_library("Potence")));
}

/// Codex of the Damned: "the bearer with Blood Sorcery": a bearer without
/// it gives nothing as it diablerizes.
#[test]
fn codex_without_blood_sorcery_gives_nothing_on_diablerie() {
    let mut e = game(2);
    let no_tha =
        |c: &zvtes::crypt::CryptCard| !c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("tha"));
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.capacity >= 10 && no_tha(c), 0),
        4,
        false,
    );
    let y = e.add_minion(SeatId(0), textless(|c| c.capacity == 4, 0), 2, false);
    let v = e.add_minion(SeatId(1), textless(|c| c.capacity == 8, 0), 2, true);
    e.add_minion(SeatId(1), textless(|c| c.capacity == 5, 0), 4, false);
    equipped(&mut e, "Codex of the Damned", a);
    hand(&mut e, 0, &["Potence"]);
    e.state.seats[1].has_edge = true;
    pick(&mut e, diablerize(a, v));
    for _ in 0..200 {
        let req = request(&mut e);
        assert!(!req
            .options
            .iter()
            .any(|o| matches!(o, Choice::TakeFrom { .. })));
        if e.referendum().is_some() {
            break;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(blood(&e, y), 2);
}

/// Enchanted Marionette: "This Malkavian antitribu gets +1 bleed and +1
/// stealth during bleed actions." "The effect is not optional." [RTR
/// 19980707] [LSJ 19980722] Its bleed has 1 stealth with no choice asked
/// and takes 2 pool. Starshell Grenade Launcher: "The minion with this
/// equipment may lock to give an acting minion -1 stealth for the current
/// action." "Can be played/used any time during the action before
/// resolution; a block attempt is not required." [LSJ 20020612] Sequencing
/// rules apply [ANK 20200607]: the prey's bearer, with no block attempt,
/// locks for -1 in the reaction window.
#[test]
fn enchanted_marionette_bleeds_for_two_with_stealth_starshell_takes_it() {
    let mut e = game(2);
    let m = e.add_minion(
        SeatId(0),
        of_clan("Malkavian antitribu", 4..=8, 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        of_clan("Malkavian antitribu", 4..=8, 1),
        4,
        false,
    );
    let x = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 1), 4, false);
    equipped(&mut e, "Enchanted Marionette", m);
    let starshell = equipped(&mut e, "Starshell Grenade Launcher", x);
    pick(&mut e, bleed(m));
    let req = offering(&mut e, weapon(starshell, 1));
    assert_eq!(req.seat, SeatId(1));
    assert_eq!(req.window, WindowLabel::ActionBlock);
    assert_eq!(e.action().expect("action").stealth(&e.state), 1);
    e.choose(index_of(&req, weapon(starshell, 1)));
    request(&mut e);
    assert!(e.state.minions[x.index()].locked);
    assert_eq!(e.action().expect("action").stealth(&e.state), 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

/// Enchanted Marionette, clan equipment: "Once in play, can be equipped
/// from the minion it's on (rulebook action) by another minion that does
/// not meet the requirements." [ANK 20180719-3] A Brujah takes it from the
/// Malkavian antitribu, then bleeds for 1 (rules-qa 9.1).
#[test]
fn enchanted_marionette_taken_by_another_clan_gives_nothing() {
    let mut e = game(2);
    let m = e.add_minion(
        SeatId(0),
        of_clan("Malkavian antitribu", 4..=8, 0),
        4,
        false,
    );
    let b = e.add_minion(SeatId(0), of_clan("Brujah", 4..=8, 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 0), 4, false);
    let marionette = equipped(&mut e, "Enchanted Marionette", m);
    pick(&mut e, take_from(b, m));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[marionette.index()].host, Some(b));

    let mut e = game(2);
    let b = e.add_minion(SeatId(0), of_clan("Brujah", 4..=8, 0), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=8, 1), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 0), 4, false);
    equipped(&mut e, "Enchanted Marionette", b);
    pick(&mut e, bleed(b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1);
}

/// Leather Jacket: "If the action to equip with the Leather Jacket is
/// successful, unlock the acting minion at the end of the turn. Bearer may
/// burn Leather Jacket to prevent all the damage from the opposing minion's
/// strike." "Cannot be used if there is no damage to prevent." [LSJ
/// 20001114] Equipped, A is unlocked as the turn ends. In combat the burn
/// waits for X's strike damage, then prevents all 3 of it.
#[test]
fn leather_jacket_unlocks_its_equipper_and_burns_to_prevent_a_strike() {
    let (mut e, a, _) = duel(&[], &[]);
    hand(&mut e, 0, &["Leather Jacket"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    placed_on(&e, "Leather Jacket", Some(a));
    assert!(
        !e.state.minions[a.index()].locked,
        "unlocked as the turn ended"
    );

    let (mut e, a, x) = duel(&[], &[]);
    stronger(&mut e, x, 2);
    let jacket = equipped(&mut e, "Leather Jacket", a);
    bleed_blocked(&mut e, a, x);
    let req = offering(&mut e, weapon(jacket, 0));
    assert_eq!(req.window, WindowLabel::CombatPrevention);
    assert_eq!(pending(&e, 0), (3, 0));
    e.choose(index_of(&req, weapon(jacket, 0)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4, "all 3 prevented");
    assert!(ash_has(&e, 0, "Leather Jacket"));

    // X at strength 0: no damage, no burn offered.
    let (mut e, a, x) = duel(&[], &[]);
    stronger(&mut e, x, -1);
    let jacket = equipped(&mut e, "Leather Jacket", a);
    bleed_blocked(&mut e, a, x);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        weapon(jacket, 0),
    );
    assert!(e.state.cards[jacket.index()].present());
}

/// Leather Jacket: "A minion may have only one Leather Jacket." Lucian:
/// "Cannot attempt to steal a piece of equipment that he is prohibited from
/// having (eg. a second Leather Jacket)." [LSJ 19980206] Tutu, holding a
/// Jacket, is not offered the torpid vampire's; without his own, he is.
#[test]
fn tutu_cannot_steal_a_second_leather_jacket() {
    for own in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(0), of_clan("Brujah", 4..=8, 0), 4, false);
        let x = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 0), 2, true);
        e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 1), 4, false);
        let jacket = equipped(&mut e, "Leather Jacket", x);
        let tutu = e.add_ally(SeatId(0), find_library("Tutu the Doubly Evil One"));
        if own {
            equipped(&mut e, "Leather Jacket", tutu);
        }
        let card = e.state.minions[tutu.index()].ally_id().expect("an ally");
        let req = request(&mut e);
        let steal = req.options.iter().any(|o| {
            matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), ability: 2, target: Target::Card(t), .. }
                if *c == card && *t == jacket)
        });
        assert_eq!(steal, !own, "holding one: {own}");
    }
}

/// Local 1111: "Requires a ready Sabbat vampire. … The Sabbat vampire with
/// this location may move 2 blood from the blood bank to any other ready
/// Sabbat vampire as a +1 stealth action." Seat 0's Sabbat bearer moves 2
/// blood to seat 2's Sabbat vampire across the table: the action directed
/// at seat 2, whose vampire may block (D12). Neither the bearer itself nor
/// a Camarilla vampire is a target; a Camarilla vampire cannot equip it.
#[test]
fn local_1111_feeds_another_sabbat_vampire() {
    use zvtes::crypt::Sect;
    let mut e = game(4);
    let s = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    let cam = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 1), 4, false);
    let t = e.add_minion(SeatId(2), of_sect(Sect::Sabbat, 4..=8, 1), 2, false);
    let b = e.add_minion(SeatId(2), of_sect(Sect::Sabbat, 4..=8, 2), 4, false);
    e.add_minion(SeatId(3), of_sect(Sect::Camarilla, 4..=8, 2), 4, false);
    let local = equipped(&mut e, "Local 1111", s);
    hand(&mut e, 0, &["Local 1111"]);
    let at = |m: MinionId| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(local),
        ability: 0,
        actor: Some(s),
        target: Target::Minion(m),
    };
    let req = request(&mut e);
    assert!(!req.options.contains(&at(s)));
    assert!(!req.options.contains(&at(cam)));
    assert!(!req.options.contains(&play(0, 0, cam)), "Camarilla");
    e.choose(index_of(&req, at(t)));
    let req = until(&mut e, SeatId(2), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(b)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, t), 4);
}

/// San Nicolás de los Servitas: "Actions directed at this Lasombra cost 1
/// additional pool." Bum's Rush at the Lasombra costs seat 0 a pool as it
/// resolves (Ambrosio's, LSJ 20030214-2); at a Ventrue holding it, nothing
/// (rules-qa 9.1).
#[test]
fn san_nicolas_taxes_actions_directed_at_its_lasombra() {
    for clan in ["Lasombra", "Ventrue"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_clan("Brujah", 4..=8, 0), 4, false);
        e.add_minion(SeatId(0), of_clan("Brujah", 4..=8, 1), 4, false);
        let l = e.add_minion(SeatId(1), of_clan(clan, 4..=8, 0), 4, false);
        e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 1), 4, false);
        equipped(&mut e, "San Nicolás de los Servitas", l);
        hand(&mut e, 0, &["Bum's Rush"]);
        pick(
            &mut e,
            Choice::Play {
                card: 0,
                mode: 0,
                actor: Some(a),
                target: Target::Minion(l),
            },
        );
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let tax = i32::from(clan == "Lasombra");
        assert_eq!(e.state.seats[0].pool, 10 - tax, "{clan}");
    }
}

/// San Nicolás de los Servitas: "If this location is burned, this Lasombra
/// takes 2 unpreventable environmental damage." Conceal [OBF] burns it:
/// its Lasombra burns 2 blood. "An action targeting it (eg. Conceal) is
/// directed at the Methuselah controlling it, not the minion it is on."
/// [LSJ 20090324] No action directed at the Lasombra: no pool more.
#[test]
fn san_nicolas_burned_damages_its_lasombra() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Brujah", 4..=8, 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obf", "obf"];
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=8, 1), 4, false);
    let l = e.add_minion(SeatId(1), of_clan("Lasombra", 4..=8, 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 0), 4, false);
    let sn = equipped(&mut e, "San Nicolás de los Servitas", l);
    hand(&mut e, 0, &["Conceal"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 1,
            actor: Some(a),
            target: Target::Card(sn),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "San Nicolás de los Servitas"));
    assert_eq!(blood(&e, l), 4 - 2);
    assert_eq!(e.state.seats[0].pool, 10);
}

/// San Nicolás de los Servitas: "This card cannot be contested. If a card
/// would contest this location, instead it is burned." The incoming copy
/// burns (LSJ 2006-10-16: "Any card that would contest this location is
/// immediately burned before contesting begins"). Seat 0's Lasombra equips
/// a copy while seat 1 has one: seat 0's burns, seat 1's stays
/// uncontested; seat 0's Lasombra takes its 2 (red reading, rules-qa 122).
#[test]
fn a_second_san_nicolas_burns_as_it_enters() {
    let mut e = game(2);
    let l0 = e.add_minion(SeatId(0), of_clan("Lasombra", 4..=8, 0), 6, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=8, 0), 4, false);
    let l1 = e.add_minion(SeatId(1), of_clan("Lasombra", 4..=8, 1), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 0), 4, false);
    let theirs = equipped(&mut e, "San Nicolás de los Servitas", l1);
    hand(&mut e, 0, &["San Nicolás de los Servitas"]);
    pick(&mut e, play(0, 0, l0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "San Nicolás de los Servitas"));
    let c = &e.state.cards[theirs.index()];
    assert!(c.in_play && !c.contested && c.host == Some(l1));
    assert_eq!(blood(&e, l0), 6 - 2 - 2, "its cost, then its 2");
}

/// Saturday-Night Special: "1R damage each strike, with an optional
/// maneuver each combat." Maneuvered to long range, it strikes X for 1
/// (X's hand strike out of range). "The optional maneuver cannot be used
/// if the strike cannot be used (eg. Hidden Lurker)." [LSJ 20021028] With
/// no strike in the first round (One With the Land [PRO]), neither is
/// offered.
#[test]
fn saturday_night_special_maneuvers_and_strikes_1r() {
    let (mut e, a, x) = duel(&[], &[]);
    let gun = equipped(&mut e, "Saturday-Night Special", a);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4, 4 - 1));

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Brujah", 4..=8, 0), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=8, 1), 4, false);
    let z = e.add_minion(SeatId(1), of_clan("Tzimisce", 4..=8, 0), 4, true);
    e.state.minions[z.index()].in_torpor = false;
    e.state.minions[z.index()].locked = true;
    e.state.minions[z.index()].gained_levels = vec!["pro", "pro"];
    let gun = equipped(&mut e, "Saturday-Night Special", a);
    hand(&mut e, 1, &["One With the Land"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, z));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::UseManeuver) || *o == weapon(gun, 0) || *o == weapon(gun, 1)
    });
    assert_eq!(e.state.seats[1].pool, 10, "blocked");
}

/// Sire's Index Finger: "The vampire with this equipment is immune to
/// frenzy cards." X's Terror Frenzy [ani] is not offered at the bearer;
/// the bearer's own Dam the Heart's River [qui] is, the opponent the one
/// frenzied (ANK 20220409).
#[test]
fn sires_index_finger_bars_frenzy_cards() {
    let (mut e, a, x) = duel(&["qui"], &["ani"]);
    equipped(&mut e, "Sire's Index Finger", a);
    hand(&mut e, 0, &["Dam the Heart's River"]);
    hand(&mut e, 1, &["Terror Frenzy"]);
    bleed_blocked(&mut e, a, x);
    let req = expect(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    assert!(
        req.options.contains(&play(0, 0, a)),
        "Dam the Heart's River"
    );
    e.choose(index_of(&req, Choice::Pass));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
    assert!(e.state.seats[1]
        .hand
        .contains(&find_library("Terror Frenzy")));
}

/// "Losing the use of the Finger won't make the vampire vulnerable to a
/// previously-played Frenzy card, and gaining it won't render him immune to
/// a previously-played Frenzy card." [LSJ 20040210] V, locked under its
/// Rötschreck, gains A's Finger by Rave: V still does not unlock at its
/// next unlock phase, the Rötschreck burned then, the Finger kept.
#[test]
fn sires_index_finger_gained_leaves_a_rotschreck_played_before() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(0), plain("Brujah", 1), 1, false);
    e.state.minions[v.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let finger = in_play(&mut e, 0, "Sire's Index Finger", Some(a));
    let rot = in_play(&mut e, 0, "Rötschreck", Some(v));
    hand(&mut e, 0, &["Rave"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::ChooseCard(finger.0));
    pick(&mut e, Choice::ChooseMinion(v));
    pick(&mut e, Choice::Pass);
    request(&mut e);
    assert_eq!(e.state.cards[finger.index()].host, Some(v));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[v.index()].locked, "Rötschreck still read");
    assert!(!e.state.cards[rot.index()].in_play, "burned at the unlock");
    assert_eq!(e.state.cards[finger.index()].host, Some(v));
    assert!(e.state.cards[finger.index()].in_play);
}

/// Sire's Index Finger under an equipment bar: "A vampire that cannot use
/// [Blade of Enoch] does not gain immunity to frenzy cards" [LSJ 20030607];
/// rules-qa 76.16. A burns his Vial of Garou Blood ("cannot use other
/// equipment cards for the remainder of combat"): X's Terror Frenzy is
/// offered at him then.
#[test]
fn sires_index_finger_shields_no_bearer_barred_from_equipment() {
    let (mut e, a, x) = duel(&[], &["ani"]);
    equipped(&mut e, "Sire's Index Finger", a);
    let vial = equipped(&mut e, "Vial of Garou Blood", a);
    hand(&mut e, 1, &["Terror Frenzy"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, weapon(vial, 0));
    let req = expect(&mut e, SeatId(1), WindowLabel::CombatBeforeRange);
    assert!(req.options.contains(&play(0, 0, x)), "Terror Frenzy at A");
}

/// The Underbridge Stray's unlock of `m` ("During an action directed at
/// you …"), offered in `req`.
fn stray_unlock(req: &zvtes::decision::DecisionRequest, stray: CardId, m: MinionId) -> Choice {
    *req.options
        .iter()
        .find(|o| {
            matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), ability: 1, target: Target::Minion(t), .. }
                if *c == stray && *t == m)
        })
        .expect("the Stray's unlock")
}

/// Starshell Grenade Launcher: "Can be used to reduce stealth multiple
/// times in the same action if the bearer manages to unlock." [LSJ
/// 20011023-2] Locked for -1, unlocked by an Underbridge Stray, locked
/// again: -2. "Using the Launcher to reduce stealth does not force the
/// minion to strike with it if he blocks." [LSJ 19971215] Locked, unlocked,
/// the bearer blocks and strikes with its hand.
#[test]
fn starshell_locks_again_after_an_unlock_and_commits_no_strike() {
    for blocks in [false, true] {
        let (mut e, a, x) = duel(&[], &[]);
        let starshell = equipped(&mut e, "Starshell Grenade Launcher", x);
        let stray = e.add_ally(SeatId(1), find_library("Underbridge Stray"));
        let stray = e.state.minions[stray.index()].ally_id().expect("an ally");
        pick(&mut e, bleed(a));
        pick_when(&mut e, weapon(starshell, 1));
        let req = request(&mut e);
        e.choose(index_of(&req, stray_unlock(&req, stray, x)));
        assert!(!e.state.minions[x.index()].locked);
        if blocks {
            // At close range its hand strike, the only one: elided.
            pick_when(&mut e, block(x));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!((blood(&e, a), blood(&e, x)), (4 - 1, 4 - 1), "hand strikes");
        } else {
            pick_when(&mut e, weapon(starshell, 1));
            request(&mut e);
            assert_eq!(e.action().expect("action").stealth_reduce, 2);
        }
    }
}

/// "If a minion who is attempting to block is locked, then that block will
/// fail, unless they woke earlier during the action." [LSJ 20010702] [RTR
/// 19941006] [LSJ 19991025] [LSJ 20100527] The Starshell's bearer attempts
/// to block, then locks for it: the attempt fails, the bleed lands.
#[test]
fn starshell_bearer_locking_during_its_block_attempt_fails_it() {
    let (mut e, a, x) = duel(&[], &[]);
    let starshell = equipped(&mut e, "Starshell Grenade Launcher", x);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, weapon(starshell, 1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].locked);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "unblocked");
    assert_eq!(blood(&e, a), 4, "no combat");
}

/// Starshell Grenade Launcher: "1R aggravated damage each strike, only
/// usable once per combat, and only usable at long range." Not at close;
/// at long range (Flash [cel]) 1 aggravated, X preventing it (Skin of
/// Steel); not again for Blur's additional strike, but a second copy is:
/// "A second copy allows a second use in the same combat." [ANK 20230316]
#[test]
fn starshell_strikes_once_each_combat_at_long_range() {
    let (mut e, a, x) = duel(&[], &[]);
    let starshell = equipped(&mut e, "Starshell Grenade Launcher", a);
    bleed_blocked(&mut e, a, x);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        weapon(starshell, 0),
    );

    for second in [false, true] {
        let (mut e, a, x) = duel(&["cel"], &["for"]);
        let starshell = equipped(&mut e, "Starshell Grenade Launcher", a);
        let other = second.then(|| equipped(&mut e, "Starshell Grenade Launcher", a));
        hand(&mut e, 0, &["Flash", "Blur"]);
        hand(&mut e, 1, &["Skin of Steel"]);
        bleed_blocked(&mut e, a, x);
        pick_when(&mut e, play(0, 0, a));
        pick_when(&mut e, weapon(starshell, 0));
        offering(&mut e, play(0, 0, x));
        assert_eq!(pending(&e, 1), (0, 1));
        pick(&mut e, play(0, 0, x));
        pick_when(&mut e, play(0, 0, a));
        match other {
            Some(s2) => {
                let req = offering(&mut e, weapon(s2, 0));
                assert!(!req.options.contains(&weapon(starshell, 0)));
            }
            None => never_offered_until(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                weapon(starshell, 0),
            ),
        }
    }
}

/// Sword of Judgment: "The bearer gets 1 optional additional strike
/// (limited) each round. Strike: strength+1 damage." "You get the
/// additional strike even if you don't use the weapon's strike" [LSJ
/// 20030213-2]: a hand strike, then the Sword's additional strike, the
/// Sword's 2 in it.
#[test]
fn sword_of_judgment_gives_an_additional_strike_after_a_hand_strike() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Brujah antitribu", 4..=8, 0), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah antitribu", 4..=8, 1), 4, false);
    let x = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 1), 4, false);
    let sword = equipped(&mut e, "Sword of Judgment", a);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, Choice::HandStrike);
    pick_when(&mut e, weapon(sword, 1));
    pick_when(&mut e, weapon(sword, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 1 - 2);
}

/// Sword of Judgment: "Only usable by a Brujah antitribu." Clan equipment
/// taken by another minion [ANK 20180719-3]: on a Brujah, neither its
/// strike nor its additional strike is offered (rules-qa 9.1).
#[test]
fn sword_of_judgment_taken_by_another_clan_is_unusable() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Brujah antitribu", 4..=8, 0), 4, false);
    let b = e.add_minion(SeatId(0), of_clan("Brujah", 4..=8, 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 0), 4, false);
    let sword = equipped(&mut e, "Sword of Judgment", a);
    pick(&mut e, take_from(b, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[sword.index()].host, Some(b));

    let mut e = game(2);
    let b = e.add_minion(SeatId(0), of_clan("Brujah", 4..=8, 0), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=8, 1), 4, false);
    let x = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 1), 4, false);
    let sword = equipped(&mut e, "Sword of Judgment", b);
    bleed_blocked(&mut e, b, x);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        *o == weapon(sword, 0) || *o == weapon(sword, 1)
    });
}

/// Vial of Garou Blood: "The vampire with this Vial may burn it before
/// range is chosen in a round of combat to get +1 strength and an
/// additional strike (limited) each round for the remainder of combat. If
/// he or she does so, he or she cannot use other equipment cards for the
/// remainder of combat." Burned: the .44 Magnum's maneuver and strike are
/// no longer offered; A's hand strikes twice at strength 2.
#[test]
fn vial_of_garou_blood_strengthens_and_bars_equipment() {
    let (mut e, a, x) = duel(&[], &[]);
    let vial = equipped(&mut e, "Vial of Garou Blood", a);
    let magnum = equipped(&mut e, ".44 Magnum", a);
    bleed_blocked(&mut e, a, x);
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::CombatBeforeRange);
    e.choose(index_of(&req, weapon(vial, 0)));
    assert!(ash_has(&e, 0, "Vial of Garou Blood"));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        *o == weapon(magnum, 0) || *o == weapon(magnum, 1)
    });
    assert_eq!(blood(&e, x), 4 - 2 - 2, "two hand strikes at strength 2");
    assert_eq!(blood(&e, a), 4 - 1);
}

/// White Phosphorus Grenade: "2R aggravated damage each strike. If White
/// Phosphorus Grenade is used at close range, the minion with this
/// equipment takes 1 aggravated damage. Burn after use." "Damage done to
/// the bearer is environmental." [LSJ 19970801] At close range X 2
/// aggravated, A X's 1 and its own 1 aggravated, the Grenade burned; at
/// long range (Flash [cel]) A takes nothing.
#[test]
fn white_phosphorus_grenade_burns_its_bearer_at_close_range() {
    let (mut e, a, x) = duel(&[], &["for"]);
    let grenade = equipped(&mut e, "White Phosphorus Grenade", a);
    hand(&mut e, 1, &["Skin of Steel"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, weapon(grenade, 0));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (0, 2));
    assert_eq!(pending(&e, 0), (1, 1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "White Phosphorus Grenade"));

    let (mut e, a, x) = duel(&["cel"], &[]);
    let grenade = equipped(&mut e, "White Phosphorus Grenade", a);
    hand(&mut e, 0, &["Flash"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, weapon(grenade, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4, "no damage at long range");
    assert!(in_torpor(&e, x));
    assert!(ash_has(&e, 0, "White Phosphorus Grenade"));
}

/// White Phosphorus Grenade: "Does not burn if combat ends before it
/// resolves (eg. if the opponent uses a 'strike: combat ends')." [LSJ
/// 20001127-2] [LSJ 20010806-1] [LSJ 19981006] X's Majesty [pre]: the
/// Grenade stays, its bearer unhurt.
#[test]
fn white_phosphorus_grenade_is_kept_when_combat_ends_first() {
    let (mut e, a, x) = duel(&[], &["pre"]);
    let grenade = equipped(&mut e, "White Phosphorus Grenade", a);
    hand(&mut e, 1, &["Majesty"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, weapon(grenade, 0));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[grenade.index()].present());
    assert_eq!(
        (blood(&e, a), blood(&e, x)),
        (4, 4 - 1),
        "Majesty's cost only"
    );
}

// ---- Vampires (vamp_c) --------------------------------------------------------

/// `m`'s printed ability `ability` offered in `req`, whatever its actor and
/// target.
fn vc_own(req: &zvtes::decision::DecisionRequest, m: MinionId, ability: u8) -> Option<Choice> {
    req.options.iter().copied().find(|o| {
        matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(v), ability: a, .. }
            if *v == m && *a == ability)
    })
}

/// Pass (or hand strike) until `m`'s printed ability `ability` is offered,
/// and use it.
#[track_caller]
fn vc_use_when(e: &mut Engine, m: MinionId, ability: u8) {
    for _ in 0..200 {
        let req = request(e);
        if let Some(c) = vc_own(&req, m, ability) {
            e.choose(index_of(&req, c));
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("{m:?}'s ability {ability} never offered");
}

/// Pass (or hand strike) until `seat`'s `window`, `m`'s printed ability
/// `ability` never offered on the way.
fn vc_never(e: &mut Engine, m: MinionId, ability: u8, seat: SeatId, window: WindowLabel) {
    never_offered_where(e, seat, window, |o| {
        matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(v), ability: a, .. }
            if *v == m && *a == ability)
    });
}

/// `m`'s printed action (ability 0) at `target`.
fn vc_at(m: MinionId, target: Target) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(m),
        ability: 0,
        actor: Some(m),
        target,
    }
}

/// The `nth` fixture vampire of capacity `cap`.
fn vc_cap(cap: u8, nth: usize) -> u16 {
    textless(|c| c.capacity == cap, nth)
}

/// The `nth` fixture vampire with a title of `title`, capacity 4 to 8.
fn vc_titled(title: Title, nth: usize) -> u16 {
    fixture_vampire(
        |c| c.title == title && !c.adv && (4..=8).contains(&c.capacity),
        nth,
    )
}

/// The `nth` fixture vampire of capacity 5 with no superior Discipline.
fn vc_inferior(nth: usize) -> u16 {
    textless(
        |c| {
            c.capacity == 5
                && c.disciplines
                    .iter()
                    .all(|d| d.chars().all(|ch| ch.is_ascii_lowercase()))
        },
        nth,
    )
}

/// The traits of round 31's vampires: the text-only fourteen (Patrick,
/// Rodrigo, Smash, Ulrike Rothbart, Axel Von Anders, Beauregard Krueller,
/// Bill Butler, Charlie Tyne, Droescher One-Eye, Esoara, Frère Marc:
/// "Sabbat."; Xendil Charmer: "Sabbat priscus."; Gabriel de Cambrai:
/// "Sabbat. Black Hand."; Guedado: "Laibon.") have no definition; each has
/// its generated sect and title, as do Little Willie, Louis de
/// Maisonneuve, Lucy Markowitz, Lula Burch and the base Melinda Galbraith
/// (a cardinal), Sabbat vampires printing no text beyond their sect and
/// title; Lord Aaron Wesley Wilkshire is Black Hand; Nostoket, Rico Loco,
/// Darvag, Dominique Santo Paulo and Gabriel de Cambrai keep Black Hand
/// under another sect [LSJ 20070322] [ANK 20180807].
#[test]
fn the_round_31_vampires_carry_their_printed_traits() {
    use zvtes::crypt::Sect::{Laibon, Sabbat};
    let text_only = [
        ("Patrick", Sabbat, Title::None),
        ("Rodrigo", Sabbat, Title::None),
        ("Smash", Sabbat, Title::None),
        ("Ulrike Rothbart", Sabbat, Title::None),
        ("Xendil Charmer", Sabbat, Title::Priscus),
        ("Axel Von Anders", Sabbat, Title::None),
        ("Beauregard Krueller", Sabbat, Title::None),
        ("Bill Butler", Sabbat, Title::None),
        ("Charlie Tyne", Sabbat, Title::None),
        ("Droescher One-Eye", Sabbat, Title::None),
        ("Esoara", Sabbat, Title::None),
        ("Frère Marc", Sabbat, Title::None),
        ("Gabriel de Cambrai", Sabbat, Title::None),
        ("Guedado", Laibon, Title::None),
    ];
    let mut e = game(2);
    for (name, sect, title) in text_only {
        let card = find_card(name);
        assert!(zvtes::cards::vampire(card).is_none(), "{name}: no text");
        let m = e.add_minion(SeatId(0), card, 1, false);
        assert_eq!(e.state.sect_of(m), Some(sect), "{name}");
        assert_eq!(e.state.title_of(m), title, "{name}");
        assert_eq!(
            e.state.is_black_hand(m),
            name == "Gabriel de Cambrai",
            "{name}"
        );
    }
    for (name, title) in [
        ("Little Willie", Title::None),
        ("Louis de Maisonneuve", Title::None),
        ("Lucy Markowitz", Title::None),
        ("Lula Burch", Title::None),
        ("Melinda Galbraith", Title::Cardinal),
    ] {
        let card = crypt_card(name, false);
        assert!(zvtes::cards::vampire(card).is_none(), "{name}: no text");
        let m = e.add_minion(SeatId(0), card, 1, false);
        assert_eq!(e.state.sect_of(m), Some(Sabbat), "{name}");
        assert_eq!(e.state.title_of(m), title, "{name}");
    }
    let aaron = e.add_minion(
        SeatId(0),
        find_card("Lord Aaron Wesley Wilkshire"),
        1,
        false,
    );
    assert!(e.state.is_black_hand(aaron));
    for (name, title) in [
        ("Radu Bistri", Title::Cardinal),
        ("Ryszard", Title::Bishop),
        ("Sha-Ennu", Title::Regent),
        ("Urraca", Title::Bishop),
        ("Ysador the Foul", Title::Cardinal),
        ("Eric Kressida", Title::Bishop),
        ("Gravitnir", Title::Priscus),
        ("Hektor", Title::Priscus),
    ] {
        let card = find_card(name);
        assert!(zvtes::cards::vampire(card).is_some(), "{name}");
        let m = e.add_minion(SeatId(1), card, 1, false);
        assert_eq!(e.state.title_of(m), title, "{name}");
    }
    for name in [
        "Mugur Sabau",
        "Terrifisto",
        "Armin Brenner",
        "Fabrizia Contreraz",
    ] {
        let m = e.add_minion(SeatId(1), find_card(name), 1, false);
        assert!(
            matches!(e.state.title_of(m), Title::Archbishop(_)),
            "{name}"
        );
    }
    for name in [
        "Nostoket",
        "Rico Loco",
        "Darvag, The Butcher of Rus",
        "Dominique Santo Paulo",
        "Gabriel de Cambrai",
    ] {
        let m = e.add_minion(SeatId(1), find_card(name), 1, false);
        e.state.minions[m.index()].sect_override = Some(zvtes::crypt::Sect::Independent);
        assert!(e.state.is_black_hand(m), "{name}");
    }
}

/// Morrow the Sage: "Once each action, Morrow may burn 2 blood to get +1
/// stealth." Lord Tremere's +1 intercept attempts the block: Morrow burns 2
/// blood for 1 stealth, once in the action (his Cloak the Gathering still
/// offered beside).
#[test]
fn morrow_buys_stealth_once_an_action_for_two_blood() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Morrow the Sage"), 5, false);
    hand(&mut e, 0, &["Cloak the Gathering"]);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let lord = e.add_minion(SeatId(1), find_card("Lord Tremere"), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(lord));
    vc_use_when(&mut e, m, 0);
    let req = request(&mut e);
    assert_eq!(blood(&e, m), 5 - 2);
    assert_eq!(e.action().expect("bleed").stealth(&e.state), 1);
    assert!(vc_own(&req, m, 0).is_none(), "once each action");
    assert!(req.options.contains(&play(0, 0, m)), "{:?}", req.options);
}

/// Mowgli: "cannot use maneuvers to maneuver to long range": at close range
/// his Pursuit is not offered; once the opposing minion maneuvered to long,
/// it is (to close). The regression of `Stat::NoManeuverToLong` read for
/// maneuver cards (Pedro Cortez's stat, once read for the granted
/// maneuver only).
#[test]
fn mowgli_cannot_maneuver_to_long_range_even_with_a_card() {
    let mut e = game(2);
    let mo = e.add_minion(SeatId(0), find_card("Mowgli"), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let x = e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 2), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["cel"];
    hand(&mut e, 0, &["Pursuit"]);
    hand(&mut e, 1, &["Pursuit"]);
    bleed_blocked(&mut e, mo, x);
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::CombatDetermineRange,
        play(0, 0, mo),
    );
    pick(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 0, mo));
}

/// Mowgli: "cannot use presses to end combat": the opposing minion pressed
/// to continue, his Boxed In (a press, to end now) is not offered: the
/// combat goes on to a second round, his card still in hand.
#[test]
fn mowgli_cannot_press_to_end_combat() {
    let mut e = game(2);
    let mo = e.add_minion(SeatId(0), find_card("Mowgli"), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let x = e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 2), 4, false);
    hand(&mut e, 0, &["Boxed In"]);
    hand(&mut e, 1, &["Boxed In"]);
    bleed_blocked(&mut e, mo, x);
    pick_when(&mut e, play(0, 0, x));
    request(&mut e);
    assert_eq!(e.combat().expect("combat").round, 2);
    assert_eq!(e.state.seats[0].hand[0], find_library("Boxed In"));
}

/// Mugur Sabau: "Each time Mugur commits diablerie, he gets an additional
/// +1 bleed." He diablerizes a torpid vampire (the blood hunt failing), then
/// bleeds for 2 next turn.
#[test]
fn mugur_bleeds_one_more_for_each_diablerie() {
    let mut e = game(2);
    let mu = e.add_minion(SeatId(0), find_card("Mugur Sabau"), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let x = e.add_minion(SeatId(1), vc_cap(5, 1), 2, true);
    e.add_minion(SeatId(1), vc_cap(5, 2), 4, false);
    pick(&mut e, diablerize(mu, x));
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[mu.index()].in_play, "the blood hunt failed");
    assert_eq!(e.state.minions[mu.index()].diableries.0.len(), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let before = e.state.seats[1].pool;
    pick(&mut e, bleed(mu));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, before - 2);
}

/// Nickolai, The Survivor: "Nickolai may gain 1 level of any Discipline
/// another of your ready vampires has at superior as a +1 stealth action
/// that costs 1 blood." The Disciplines named: those of his ready vampires'
/// at superior ([POT], [DOM]), not a torpid one's ([OBF]); [pot] named, he
/// has it at superior, 1 blood paid, the action at 1 stealth.
#[test]
fn nickolai_learns_a_discipline_his_ally_has_at_superior() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nickolai, The Survivor"), 4, false);
    let p = e.add_minion(SeatId(0), vc_inferior(0), 4, false);
    let d = e.add_minion(SeatId(0), vc_inferior(1), 4, false);
    let t = e.add_minion(SeatId(0), vc_inferior(2), 2, true);
    e.state.minions[p.index()].gained_levels = vec!["pot", "pot"];
    e.state.minions[d.index()].gained_levels = vec!["dom", "dom"];
    e.state.minions[t.index()].gained_levels = vec!["obf", "obf"];
    e.add_minion(SeatId(1), vc_cap(5, 3), 4, false);
    assert_eq!(e.state.discipline_level(n, "pot"), 1);
    pick(&mut e, vc_at(n, Target::None));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::ActionTerms);
    let mut named = req.options.clone();
    named.sort_by_key(|c| format!("{c:?}"));
    assert_eq!(
        named,
        vec![
            Choice::ChooseDiscipline("dom"),
            Choice::ChooseDiscipline("pot")
        ]
    );
    e.choose(index_of(&req, Choice::ChooseDiscipline("pot")));
    request(&mut e);
    assert_eq!(e.action().expect("action").stealth(&e.state), 1);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.discipline_level(n, "pot"), 2);
    assert_eq!(blood(&e, n), 4 - 1);
}

/// Nickolai: with no other ready vampire superior in a Discipline, the
/// action is not offered.
#[test]
fn nickolai_has_nothing_to_learn_alone() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nickolai, The Survivor"), 4, false);
    e.add_minion(SeatId(0), vc_inferior(0), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    let req = request(&mut e);
    assert!(vc_own(&req, n, 0).is_none(), "{:?}", req.options);
}

/// Nickolai: the vampire superior in the named Discipline sent to torpor
/// before the action resolves, no level is gained (D1); the blood paid.
#[test]
fn nickolai_gains_nothing_once_the_teacher_is_gone() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nickolai, The Survivor"), 4, false);
    let p = e.add_minion(SeatId(0), vc_inferior(0), 4, false);
    e.state.minions[p.index()].gained_levels = vec!["pot", "pot"];
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    pick(&mut e, vc_at(n, Target::None));
    request(&mut e);
    e.state.minions[p.index()].in_torpor = true;
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.discipline_level(n, "pot"), 1);
    assert_eq!(blood(&e, n), 4 - 1);
}

/// Nostoket: "may inflict 2 damage on a ready minion controlled by your
/// predator or prey as a Ⓓ action." Four seats: the prey's and the
/// predator's ready vampires are targets, a cross-table one and a torpid
/// one are not; the prey's takes 2 damage.
#[test]
fn nostoket_wounds_a_ready_minion_of_his_predator_or_prey() {
    let mut e = game(4);
    let no = e.add_minion(SeatId(0), find_card("Nostoket"), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let prey = e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    let torpid = e.add_minion(SeatId(1), vc_cap(5, 2), 2, true);
    let across = e.add_minion(SeatId(2), vc_cap(5, 3), 4, false);
    let pred = e.add_minion(SeatId(3), vc_cap(5, 4), 4, false);
    let req = request(&mut e);
    assert!(req.options.contains(&vc_at(no, Target::Minion(prey))));
    assert!(req.options.contains(&vc_at(no, Target::Minion(pred))));
    assert!(!req.options.contains(&vc_at(no, Target::Minion(torpid))));
    assert!(!req.options.contains(&vc_at(no, Target::Minion(across))));
    e.choose(index_of(&req, vc_at(no, Target::Minion(prey))));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, prey), 4 - 2);
    assert!(!e.state.minions[no.index()].in_torpor);
}

/// Radu Bistri: "If the card named Purchase Pact is in play during your
/// unlock phase, Radu can gain 1 blood." Another Methuselah's Purchase Pact:
/// offered once in his unlock phase, +1 blood.
#[test]
fn radu_gains_a_blood_while_purchase_pact_is_in_play() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Radu Bistri"), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    e.state.cards.push(CardInPlay::new(
        find_library("Purchase Pact"),
        SeatId(1),
        None,
        0,
    ));
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    vc_use_when(&mut e, r, 0);
    vc_never(&mut e, r, 0, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, r), 4 + 1);
}

/// Radu: without Purchase Pact in play, never offered.
#[test]
fn radu_gains_nothing_without_purchase_pact() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Radu Bistri"), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    vc_never(&mut e, r, 0, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, r), 4);
}

/// Rain: "+1 stealth on actions directed at a Methuselah who controls no
/// ready titled vampires." Her bleed: 1 stealth against an untitled prey,
/// none once the prey's primogen is ready, 1 again with it in torpor (read
/// live).
#[test]
fn rain_sneaks_on_methuselahs_without_ready_titled_vampires() {
    for (titled, torpid, want) in [(false, false, 1), (true, false, 0), (true, true, 1)] {
        let mut e = game(2);
        let r = e.add_minion(SeatId(0), find_card("Rain"), 4, false);
        e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
        e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
        if titled {
            e.add_minion(SeatId(1), vc_titled(Title::Primogen, 0), 2, torpid);
        }
        pick(&mut e, bleed(r));
        request(&mut e);
        assert_eq!(
            e.action().expect("bleed").stealth(&e.state),
            want,
            "titled {titled}, torpid {torpid}"
        );
    }
}

/// Raphael Catarari: "If Raphael's blood total is odd, he does not unlock as
/// normal." Locked with 3 blood he stays locked; with 4 he unlocks.
#[test]
fn raphael_unlocks_only_with_even_blood() {
    for (b, locked) in [(3, true), (4, false)] {
        let mut e = game(2);
        let r = e.add_minion(SeatId(0), find_card("Raphael Catarari"), b, false);
        e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
        e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
        e.state.minions[r.index()].locked = true;
        to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[r.index()].locked, locked, "{b} blood");
    }
}

/// Raphael: "Unlocking is the first thing to do in the unlock phase: if he
/// is odd, gaining one blood later in the unlock phase will not let him
/// 'unlock as normal' this turn." [LSJ 20060911] Locked with 3 blood, his
/// Vessel moves a pool to him in the unlock phase (4): still locked.
#[test]
fn raphael_stays_locked_though_even_later_in_the_unlock_phase() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Raphael Catarari"), 3, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    e.state.minions[r.index()].locked = true;
    let vessel = in_play_mode(&mut e, 0, "Vessel", Some(r), 0);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(vessel),
            ability: 1,
            actor: None,
            target: Target::None,
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, r), 4);
    assert!(e.state.minions[r.index()].locked);
}

/// Raphael: "You may use a master phase action to add a blood to him from
/// your pool or to burn a blood from him." Each spends a master phase
/// action: pool -1 and blood +1; then blood -1.
#[test]
fn raphael_gains_or_burns_a_blood_for_a_master_phase_action() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Raphael Catarari"), 2, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let pool = e.state.seats[0].pool;
    vc_use_when(&mut e, r, 0);
    request(&mut e);
    assert_eq!((blood(&e, r), e.state.seats[0].pool), (3, pool - 1));
    assert_eq!(e.state.mpa_left(SeatId(0)), 0);
    e.state.seats[0].mpa += 1;
    vc_use_when(&mut e, r, 1);
    request(&mut e);
    assert_eq!(blood(&e, r), 2);
    assert_eq!(e.state.mpa_left(SeatId(0)), 0);
}

/// Raphael at 0 blood: "to burn a blood from him" is still offered, a legal
/// no-op (D14; rules-qa 63.16, 17.4): the master phase action spent,
/// nothing burned.
#[test]
fn raphael_burns_nothing_at_zero_blood_for_a_master_phase_action() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Raphael Catarari"), 2, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    e.state.minions[r.index()].blood = 0;
    e.state.minions[r.index()].in_torpor = false;
    let pool = e.state.seats[0].pool;
    let req = request(&mut e);
    let burn = vc_own(&req, r, 1).expect("the burn offered at 0 blood");
    e.choose(index_of(&req, burn));
    request(&mut e);
    assert_eq!((blood(&e, r), e.state.seats[0].pool), (0, pool));
    assert!(e.state.minions[r.index()].in_play && !e.state.minions[r.index()].in_torpor);
    assert_eq!(e.state.mpa_left(SeatId(0)), 0);
}

/// Redbone McCray: "may enter combat with a younger Camarilla vampire as a
/// Ⓓ action." A capacity-4 Camarilla vampire is a target, a capacity-7 one
/// (older) and a capacity-4 Sabbat one are not; the combat follows.
#[test]
fn redbone_attacks_a_younger_camarilla_vampire() {
    use zvtes::crypt::Sect;
    let mut e = game(2);
    let rb = e.add_minion(SeatId(0), find_card("Redbone McCray"), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let young = e.add_minion(
        SeatId(1),
        textless(|c| c.sect == Some(Sect::Camarilla) && c.capacity == 4, 0),
        4,
        false,
    );
    let old = e.add_minion(
        SeatId(1),
        textless(|c| c.sect == Some(Sect::Camarilla) && c.capacity == 7, 0),
        4,
        false,
    );
    let sabbat = e.add_minion(
        SeatId(1),
        textless(|c| c.sect == Some(Sect::Sabbat) && c.capacity == 4, 0),
        4,
        false,
    );
    let req = request(&mut e);
    assert!(req.options.contains(&vc_at(rb, Target::Minion(young))));
    assert!(!req.options.contains(&vc_at(rb, Target::Minion(old))));
    assert!(!req.options.contains(&vc_at(rb, Target::Minion(sabbat))));
    e.choose(index_of(&req, vc_at(rb, Target::Minion(young))));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        (blood(&e, rb), blood(&e, young)),
        (4 - 1, 4 - 1),
        "their hand strikes"
    );
}

/// Rico Loco: "If Rico is ready during your master phase, move a Bomb from
/// your hand or ash heap to him (no cost) or he takes 1 damage." A due of the
/// master phase (no pass before it, rules-qa 33.1): both zones and the damage
/// offered; from hand, the Bomb itself on him, no blood or pool paid.
#[test]
fn rico_moves_a_bomb_onto_himself_each_master_phase() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Rico Loco"), 2, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    // Blood Doll: a master phase window with a choice besides the due.
    hand(&mut e, 0, &["Bomb", "Blood Doll"]);
    e.state.seats[0].ash = vec![find_library("Bomb")].into();
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(!req.options.contains(&Choice::Pass), "{:?}", req.options);
    e.choose(index_of(&req, Choice::ResolvePrinted { minion: r }));
    let req = request(&mut e);
    let bomb = find_library("Bomb");
    let from_hand = Choice::TakeFrom {
        zone: zvtes::decision::Zone::Hand,
        card: bomb,
    };
    let from_ash = Choice::TakeFrom {
        zone: zvtes::decision::Zone::AshHeap,
        card: bomb,
    };
    assert_eq!(req.options, vec![Choice::Pass, from_hand, from_ash]);
    let pool = e.state.seats[0].pool;
    e.choose(index_of(&req, from_hand));
    request(&mut e);
    let on = e
        .state
        .cards
        .iter()
        .find(|c| c.card == bomb && c.in_play)
        .expect("the Bomb in play");
    assert_eq!(on.host, Some(r));
    assert!(!on.effect_only);
    assert!(!e.state.seats[0].hand.contains(&bomb));
    assert!(ash_has(&e, 0, "Bomb"), "the other one stays");
    assert_eq!((blood(&e, r), e.state.seats[0].pool), (2, pool));
}

/// Rico Loco: "The damage he takes during the master phase from his
/// ability is environmental." [RTR 19970630] No Bomb in hand or ash heap:
/// he takes 1 damage out of combat (a blood), caused by his own text.
#[test]
fn rico_takes_a_bomb_or_a_wound_each_master_phase() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Rico Loco"), 2, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    hand(&mut e, 0, &["Blood Doll"]);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    e.notes = Some(Vec::new());
    let before = e.table();
    e.choose(index_of(&req, Choice::ResolvePrinted { minion: r }));
    request(&mut e);
    assert_eq!(blood(&e, r), 1);
    assert!(e.combat().is_none());
    let changes = noted_changes(&mut e, before);
    assert_eq!(
        changes,
        vec![(
            Some(zvtes::notes::Cause::Effect(zvtes::notes::Source::Minion(r))),
            Err(r),
            -1
        )]
    );
}

/// Rico Loco: "During your unlock phase, if Rico is ready and has a Bomb,
/// burn the Bomb and another ready minion you control takes 2 damage." The
/// Bomb to the ash heap, the other ready vampire 2 blood down (the torpid
/// one not chosen); Rico in torpor: nothing.
#[test]
fn rico_burns_his_bomb_to_wound_another_minion_in_the_unlock_phase() {
    for torpid in [false, true] {
        let mut e = game(2);
        let r = e.add_minion(SeatId(0), find_card("Rico Loco"), 2, torpid);
        let v = e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
        let t = e.add_minion(SeatId(0), vc_cap(5, 1), 2, true);
        e.add_minion(SeatId(1), vc_cap(5, 2), 4, false);
        let bomb = in_play_mode(&mut e, 0, "Bomb", Some(r), 0);
        hand(&mut e, 0, &["Blood Doll"]);
        to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
        to_window(&mut e, SeatId(0), WindowLabel::MasterPhase);
        assert_eq!(
            e.state.cards[bomb.index()].in_play,
            torpid,
            "torpid {torpid}"
        );
        assert_eq!(blood(&e, v), if torpid { 4 } else { 4 - 2 });
        assert_eq!(blood(&e, t), 2);
    }
}

/// Ryszard: "cannot block titled vampires." A primogen's bleed: no attempt
/// offered to him; an untitled vampire's: offered.
#[test]
fn ryszard_cannot_block_titled_vampires() {
    for titled in [true, false] {
        let mut e = game(2);
        let a = if titled {
            e.add_minion(SeatId(0), vc_titled(Title::Primogen, 0), 4, false)
        } else {
            e.add_minion(SeatId(0), vc_cap(5, 0), 4, false)
        };
        e.add_minion(SeatId(0), vc_cap(5, 1), 4, false);
        let ry = e.add_minion(SeatId(1), find_card("Ryszard"), 4, false);
        e.add_minion(SeatId(1), vc_cap(5, 2), 4, false);
        pick(&mut e, bleed(a));
        let req = request(&mut e);
        assert_eq!(req.options.contains(&block(ry)), !titled, "titled {titled}");
    }
}

/// Schuyler: "If Schuyler is blocked, your prey can move a library card from
/// their ash heap to the top of their library before block resolution." Three
/// seats, his predator (Lord Tremere, +1 intercept against the hunt's +1
/// stealth) blocks his hunt: his prey is asked, the card chosen on top of
/// their library, out of their ash heap.
#[test]
fn schuyler_blocked_lets_his_prey_restack_a_card() {
    let mut e = game(3);
    let s = e.add_minion(SeatId(0), find_card("Schuyler"), 1, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    let pred = e.add_minion(SeatId(2), find_card("Lord Tremere"), 4, false);
    let card = find_library("Pursuit");
    e.state.seats[1].ash = vec![card].into();
    // A full hand: the card stays on the library.
    fill_hand(&mut e, 1);
    pick(&mut e, action(s, ActionKind::Hunt));
    pick_when(&mut e, block(pred));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.options.clone()),
        (SeatId(1), vec![Choice::Pass, Choice::FindCard(card)])
    );
    e.choose(1);
    request(&mut e);
    assert_eq!(e.state.seats[1].library.last(), Some(&card));
    assert!(!e.state.seats[1].ash.contains(&card));
}

/// Servius Marius Pustula: "Each Methuselah gets +X hand size, where X is
/// the number of ready vampires with capacity above 7 he or she controls."
/// His Methuselah: him and a capacity-8 vampire, 9; the other: a ready
/// capacity-9, a torpid capacity-9 and a capacity-7, 8; Servius in torpor:
/// his Methuselah 8.
#[test]
fn servius_widens_each_hand_by_its_big_ready_vampires() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Servius Marius Pustula"), 4, false);
    e.add_minion(SeatId(0), vc_cap(8, 0), 4, false);
    e.add_minion(SeatId(1), vc_cap(9, 0), 4, false);
    e.add_minion(SeatId(1), vc_cap(9, 1), 2, true);
    e.add_minion(SeatId(1), vc_cap(7, 0), 4, false);
    assert_eq!(
        (e.state.hand_size(SeatId(0)), e.state.hand_size(SeatId(1))),
        (9, 8)
    );
    e.state.minions[s.index()].in_torpor = true;
    assert_eq!(e.state.hand_size(SeatId(0)), 8, "from torpor too");
}

/// Sha-Ennu: "Vampires with capacity 3 or less cannot block Sha-Ennu. +2
/// bleed." A capacity-3 vampire is not offered the attempt, a capacity-4
/// one is; the bleed takes 3 pool.
#[test]
fn sha_ennu_is_unblockable_by_small_vampires() {
    let mut e = game(2);
    let sh = e.add_minion(SeatId(0), find_card("Sha-Ennu"), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let small = e.add_minion(SeatId(1), vc_cap(3, 0), 3, false);
    let big = e.add_minion(SeatId(1), vc_cap(4, 0), 4, false);
    pick(&mut e, bleed(sh));
    let req = request(&mut e);
    assert!(req.options.contains(&block(big)));
    assert!(!req.options.contains(&block(small)));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 3);
}

/// Tears, The Dark Pierrot: "If Tears is blocked when attempting a bleed or
/// political action, he burns 1 blood (before combat begins, if any)."
/// Blocked bleeding: 1 blood burned before the combat's strikes; blocked
/// hunting: none.
#[test]
fn tears_burns_a_blood_when_blocked_bleeding_not_hunting() {
    for hunt in [false, true] {
        let mut e = game(2);
        let t = e.add_minion(SeatId(0), find_card("Tears, The Dark Pierrot"), 4, false);
        e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
        // +1 intercept: the hunt's +1 stealth reached.
        let x = e.add_minion(SeatId(1), find_card("Lord Tremere"), 4, false);
        e.add_minion(SeatId(1), vc_cap(5, 2), 4, false);
        // X's Pursuit keeps a window open in the combat.
        e.state.minions[x.index()].gained_levels = vec!["cel"];
        hand(&mut e, 1, &["Pursuit"]);
        let kind = if hunt {
            ActionKind::Hunt
        } else {
            ActionKind::Bleed
        };
        pick(&mut e, action(t, kind));
        pick_when(&mut e, block(x));
        let req = request(&mut e);
        assert!(e.combat().is_some(), "{req:?}");
        assert_eq!(blood(&e, t), if hunt { 4 } else { 4 - 1 }, "hunt {hunt}");
    }
}

/// Tears: blocked attempting a political action, he burns 1 blood too.
#[test]
fn tears_burns_a_blood_when_blocked_politicking() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Tears, The Dark Pierrot"), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let x = e.add_minion(SeatId(1), find_card("Lord Tremere"), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 2), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["cel"];
    hand(&mut e, 1, &["Pursuit"]);
    hand(&mut e, 0, &["Political Stranglehold"]);
    pick(&mut e, play(0, 0, t));
    pick_when(&mut e, block(x));
    request(&mut e);
    assert!(e.combat().is_some());
    assert_eq!(blood(&e, t), 4 - 1);
}

/// Terrifisto: "During a referendum called by Terrifisto, he can burn 1
/// blood once results are tallied to inflict 2 damage on a ready vampire who
/// cast votes or ballots against the referendum." His Political
/// Stranglehold: a primogen votes against, another in favor; at the tally,
/// before the referendum's effect, the one against offered (not the one in
/// favor); chosen: Terrifisto 1 blood, it 2 blood; then the effect.
#[test]
fn terrifisto_wounds_a_vampire_that_voted_against() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Terrifisto"), 5, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let against = e.add_minion(SeatId(1), vc_titled(Title::Primogen, 0), 4, false);
    let favor = e.add_minion(SeatId(1), vc_titled(Title::Primogen, 1), 4, false);
    hand(&mut e, 0, &["Political Stranglehold"]);
    pick(&mut e, play(0, 0, t));
    to_referendum(&mut e);
    pick_when(&mut e, votes(t, true));
    pick_when(&mut e, votes(against, false));
    pick_when(&mut e, votes(favor, true));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::ChooseMinion(against)]
    );
    let pool = e.state.seats[0].pool;
    e.choose(1);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, t), blood(&e, against)), (5 - 1, 4 - 2));
    assert_eq!(e.state.seats[0].pool, pool + 3, "passed: Terrifisto's 10");
}

/// Terrifisto: Pass, nothing burned nor inflicted; the referendum resolves.
#[test]
fn terrifisto_may_decline_his_tally_wound() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Terrifisto"), 5, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let against = e.add_minion(SeatId(1), vc_titled(Title::Primogen, 0), 4, false);
    hand(&mut e, 0, &["Political Stranglehold"]);
    pick(&mut e, play(0, 0, t));
    to_referendum(&mut e);
    pick_when(&mut e, votes(t, true));
    pick_when(&mut e, votes(against, false));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    let pool = e.state.seats[0].pool;
    e.choose(index_of(&req, Choice::Pass));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, t), blood(&e, against)), (5, 4));
    assert_eq!(e.state.seats[0].pool, pool + 3);
}

/// Urraca: "While you have the Edge, Urraca gets +1 bleed." With the Edge
/// her bleed takes 2 pool, without 1.
#[test]
fn urraca_bleeds_harder_with_the_edge() {
    for edge in [true, false] {
        let mut e = game(2);
        let u = e.add_minion(SeatId(0), find_card("Urraca"), 4, false);
        e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
        e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
        e.state.seats[0].has_edge = edge;
        pick(&mut e, bleed(u));
        to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.seats[1].pool,
            if edge { 8 } else { 9 },
            "edge {edge}"
        );
    }
}

/// Uta Kovacs: "can enter combat with a minion controlled by another
/// Methuselah as a Ⓓ action." An older vampire of the other Methuselah's is
/// a target, her own second vampire is not.
#[test]
fn uta_attacks_any_other_methuselahs_minion() {
    let mut e = game(2);
    let uta = e.add_minion(SeatId(0), find_card("Uta Kovacs"), 4, false);
    let own = e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let old = e.add_minion(SeatId(1), vc_cap(10, 0), 4, false);
    let req = request(&mut e);
    assert!(req.options.contains(&vc_at(uta, Target::Minion(old))));
    assert!(!req.options.contains(&vc_at(uta, Target::Minion(own))));
}

/// The blood a vampire of `name` (4 blood, [vis] gained) has after playing
/// Brick by Brick in `mode` at the maneuver of a blocked bleed (to long
/// range: no hand strike lands).
fn brick_by_brick_paid(name: &str, mode: u8) -> u8 {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card(name), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let x = e.add_minion(SeatId(1), vc_cap(9, 0), 6, false);
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["vis"];
    hand(&mut e, 0, &["Brick by Brick"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, mode, a));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    blood(&e, a)
}

/// Uta Kovacs: "Cards that require Blood Sorcery [tha] cost her 1 less
/// blood." Brick by Brick played at [tha] (a maneuver) is free; at [vis]
/// (its strike) it costs her its 1 blood: the cards played using [tha]
/// (rules-digest D29).
#[test]
fn uta_pays_one_less_blood_for_blood_sorcery() {
    assert_eq!(brick_by_brick_paid("Uta Kovacs", 0), 4);
    assert_eq!(brick_by_brick_paid("Uta Kovacs", 1), 3);
}

/// Lord Tremere: "Cards requiring Blood Sorcery [tha] cost Lord Tremere 1
/// fewer blood": the cards played using [tha] (rules-qa 60.5, D29), not a
/// card printing [tha] played at another Discipline. The regression of his
/// discount once read on the card's printed Disciplines.
#[test]
fn lord_tremere_discounts_only_cards_played_using_blood_sorcery() {
    assert_eq!(brick_by_brick_paid("Lord Tremere", 0), 4);
    assert_eq!(brick_by_brick_paid("Lord Tremere", 1), 3);
}

/// Ysador the Foul: "Non-infernal, non-demon minions opposing Ysador in
/// combat cannot use maneuvers to maneuver to close range." Ysador
/// maneuvers to long: a mundane opposing vampire's Pursuit is not offered;
/// an infernal one's is.
#[test]
fn ysador_keeps_mundane_opponents_at_long_range() {
    for infernal in [false, true] {
        let mut e = game(2);
        let y = e.add_minion(SeatId(0), find_card("Ysador the Foul"), 4, false);
        e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
        let x = e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
        e.add_minion(SeatId(1), vc_cap(5, 2), 4, false);
        e.state.minions[y.index()].gained_levels = vec!["cel"];
        e.state.minions[x.index()].gained_levels = vec!["cel"];
        e.state.minions[x.index()].made_infernal = infernal;
        hand(&mut e, 0, &["Pursuit"]);
        hand(&mut e, 1, &["Pursuit"]);
        bleed_blocked(&mut e, y, x);
        pick_when(&mut e, play(0, 0, y));
        let offered = offered_before(&mut e, play(0, 0, x), SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(offered, infernal, "infernal {infernal}");
    }
}

// ---- Vampires (vamp_a) --------------------------------------------------------

/// The advanced crypt card `name`.
fn va_adv(name: &str) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && c.adv)
        .unwrap_or_else(|| panic!("no {name} (ADV)")) as u16
}

/// Pass until `m`'s printed ability `ability` is offered: that request.
#[track_caller]
fn va_offered(e: &mut Engine, m: MinionId, ability: u8) -> zvtes::decision::DecisionRequest {
    for _ in 0..200 {
        let req = request(e);
        if vc_own(&req, m, ability).is_some() {
            return req;
        }
        e.choose(fallback(&req));
    }
    panic!("{m:?}'s ability {ability} never offered");
}

/// Seat 0's Kine Resources Contested, called by `caller`: 3 points on
/// `seat`, the least on the others; passing until its referendum.
fn va_kine(e: &mut Engine, caller: MinionId, seat: u8) {
    hand(e, 0, &["Kine Resources Contested"]);
    pick(e, play(0, 0, caller));
    for _ in 0..200 {
        let req = request(e);
        if req.window != WindowLabel::ReferendumTerms {
            if e.referendum().is_some() {
                return;
            }
            e.choose(fallback(&req));
            continue;
        }
        let (s, n) = req
            .options
            .iter()
            .filter_map(|o| match *o {
                Choice::AllocateTo { seat: s, n } => Some((s, n)),
                _ => None,
            })
            .min_by_key(|&(s, n)| if s.0 == seat { 3 - n.min(3) } else { n })
            .expect("an allocation");
        e.choose(index_of(&req, alloc(s.0, n)));
    }
    panic!("no referendum");
}

/// `m`'s printed modifier to `stat` with `opponent` opposing it.
fn va_against(e: &Engine, m: MinionId, stat: zvtes::cards::Stat, opponent: MinionId) -> i32 {
    e.state.printed_mod(
        m,
        stat,
        zvtes::state::PrintedCtx {
            opponent: Some(opponent),
            ..Default::default()
        },
    )
}

/// Armin Brenner: "After a referendum called by Armin passes, you can gain
/// 1 pool." "His ability is applied after the referendum effects are
/// applied, including any oust and pool gained from it." [LSJ
/// 20090115-2] His Kine Resources Contested ousts his prey: the oust's 6
/// pool already gained, he gains 1 more, once.
#[test]
fn armin_brenner_gains_a_pool_after_his_referendum_passes_and_its_oust() {
    let mut e = game(3);
    let armin = e.add_minion(SeatId(0), find_card("Armin Brenner"), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    e.add_minion(SeatId(2), vc_cap(5, 2), 4, false);
    e.state.seats[1].pool = 3;
    va_kine(&mut e, armin, 1);
    pick_when(&mut e, votes(armin, true));
    let req = va_offered(&mut e, armin, 0);
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    assert!(e.state.seats[1].ousted);
    let before = e.state.seats[0].pool;
    assert_eq!(before, 10 + 6, "the oust's pool already gained");
    e.choose(index_of(&req, vc_own(&req, armin, 0).expect("offered")));
    assert_eq!(e.state.seats[0].pool, before + 1);
    vc_never(&mut e, armin, 0, SeatId(2), WindowLabel::MinionPhase);
}

/// Armin: his referendum failing (no vote cast), nothing offered.
#[test]
fn armin_brenner_gains_nothing_from_a_failed_referendum() {
    let mut e = game(2);
    let armin = e.add_minion(SeatId(0), find_card("Armin Brenner"), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    va_kine(&mut e, armin, 1);
    vc_never(&mut e, armin, 0, SeatId(1), WindowLabel::MinionPhase);
}

/// Bela Kardoza: "During the polling step of any referendum, Bela can burn
/// X blood to get +X votes." "The ability can be used while locked, only
/// once per referendum." [ANK 20220705] Locked, in another Methuselah's
/// referendum, X offered from 0 to his 4 blood; 2 burned for 2 votes; not
/// offered again.
#[test]
fn bela_kardoza_burns_x_blood_for_x_votes_once_while_locked() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 1), 4, false);
    let bela = e.add_minion(SeatId(1), find_card("Bela Kardoza"), 4, false);
    e.state.minions[bela.index()].locked = true;
    va_kine(&mut e, caller, 0);
    vc_use_when(&mut e, bela, 0);
    let req = request(&mut e);
    let mut xs = req.options.clone();
    xs.sort_by_key(|c| format!("{c:?}"));
    assert_eq!(xs, (0..=4).map(set_x).collect::<Vec<_>>());
    e.choose(index_of(&req, set_x(2)));
    let req = request(&mut e);
    assert_eq!(blood(&e, bela), 4 - 2);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, bela), 2);
    assert!(vc_own(&req, bela, 0).is_none(), "once each referendum");
}

/// Caroline Bishops: "Caroline cannot block non-Camarilla vampires." A
/// Sabbat vampire's bleed: she cannot block it; a Camarilla vampire's: she
/// can.
#[test]
fn caroline_bishops_cannot_block_a_non_camarilla_vampire() {
    use zvtes::crypt::Sect;
    for (sect, blocks) in [(Sect::Sabbat, false), (Sect::Camarilla, true)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_sect(sect, 4..=8, 0), 4, false);
        e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
        let c = e.add_minion(SeatId(1), find_card("Caroline Bishops"), 3, false);
        e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
        pick(&mut e, bleed(a));
        let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
        assert_eq!(req.options.contains(&block(c)), blocks, "{sect:?}");
    }
}

/// Conrad Adoula: "While Conrad is acting, reaction cards cost younger
/// vampires 1 additional blood." "Card with multiple types (eg.
/// Reaction/Action Modifier) are considered of the type they are played
/// as, not both." [PIB 20150924-2] Only while he acts; a younger vampire's
/// Telepathic Counter and Form of the Bat played as a reaction cost 1
/// more; Form of the Bat as an action modifier, an older vampire's or an
/// ally's reaction, no more.
#[test]
fn conrad_adoula_taxes_younger_vampires_reactions_while_he_acts() {
    use zvtes::cards::Stat;
    let mut e = game(2);
    let conrad = e.add_minion(SeatId(0), find_card("Conrad Adoula"), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let young = e.add_minion(SeatId(1), vc_cap(4, 0), 4, false);
    let old = e.add_minion(SeatId(1), vc_cap(9, 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    let cost = |e: &Engine, m: MinionId, card: &str, reacting: bool| {
        e.state.printed_mod(
            m,
            Stat::BloodCostMore,
            zvtes::state::PrintedCtx {
                card: Some((find_library(card), 0)),
                reacting,
                ..Default::default()
            },
        )
    };
    assert_eq!(cost(&e, young, "Telepathic Counter", true), 0);
    pick(&mut e, bleed(conrad));
    request(&mut e);
    assert_eq!(cost(&e, young, "Telepathic Counter", true), 1);
    assert_eq!(cost(&e, young, "Form of the Bat", true), 1);
    assert_eq!(cost(&e, young, "Form of the Bat", false), 0);
    assert_eq!(cost(&e, old, "Telepathic Counter", true), 0);
    assert_eq!(cost(&e, ally, "Telepathic Counter", true), 0);
}

/// Count Vladimir Rustovitch: "After you replace a combat card Vladimir
/// played, you can draw 1 additional card." His Horrid Form replaced: the
/// extra draw offered.
#[test]
fn count_vladimir_draws_again_after_a_combat_card() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Count Vladimir Rustovitch"), 6, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let z = e.add_minion(SeatId(1), vc_cap(5, 1), 6, false);
    e.add_minion(SeatId(1), vc_cap(5, 2), 4, false);
    hand(&mut e, 0, &["Horrid Form"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
    bleed_blocked(&mut e, v, z);
    pick_when(&mut e, play(0, 0, v));
    offering(&mut e, Choice::DrawLibrary);
}

/// Count Vladimir: an action modifier he plays brings no extra draw.
#[test]
fn count_vladimir_draws_nothing_more_after_an_action_modifier() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Count Vladimir Rustovitch"), 6, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let z = e.add_minion(SeatId(1), vc_cap(5, 1), 6, false);
    e.add_minion(SeatId(1), vc_cap(5, 2), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["aus"];
    e.state.minions[v.index()].gained_levels = vec!["obf"];
    hand(&mut e, 0, &["Cloak the Gathering"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
    pick(&mut e, bleed(v));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, v));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::DrawLibrary,
    );
}

/// Count Vladimir: "He gets +1 strength in combat with a werewolf." Double
/// Deuce (a werewolf ally): +1; a vampire: none.
#[test]
fn count_vladimir_gets_strength_against_a_werewolf() {
    use zvtes::cards::Stat;
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Count Vladimir Rustovitch"), 6, false);
    let wolf = e.add_ally(SeatId(1), find_library("Double Deuce"));
    let z = e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    assert_eq!(va_against(&e, v, Stat::Strength, wolf), 1);
    assert_eq!(va_against(&e, v, Stat::Strength, z), 0);
}

/// Darvag, The Butcher of Rus: "Darvag can burn a location as a Ⓓ
/// action." Seat 1's location: the action directed at seat 1 (its block
/// window), the location burned to its owner's ash heap.
#[test]
fn darvag_burns_a_location_as_an_action() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Darvag, The Butcher of Rus"), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    let l = in_play_mode(&mut e, 1, "Academic Hunting Ground", None, 0);
    pick(&mut e, vc_at(d, Target::Card(l)));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[l.index()].present());
    assert!(ash_has(&e, 1, "Academic Hunting Ground"));
}

/// DeSalle: "During an action, each Sabbat vampire attempting to block
/// DeSalle can burn 1 blood to get +1 intercept. +1 stealth." His bleed at
/// 1 stealth: seat 1's Sabbat vampire (no intercept) can reach it, a
/// Camarilla one cannot; it attempts, burns 1 blood, and blocks.
#[test]
fn desalle_lets_a_sabbat_blocker_burn_blood_for_intercept() {
    use zvtes::crypt::Sect;
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("DeSalle"), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let z = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 3, false);
    let c = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 0), 3, false);
    pick(&mut e, bleed(d));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert_eq!(e.action().expect("bleed").stealth(&e.state), 1);
    assert!(e.block_can_reach(SeatId(1), z));
    assert!(!e.block_can_reach(SeatId(1), c));
    pick(&mut e, block(z));
    pick_when(&mut e, Choice::BurnBloodForIntercept { minion: z });
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert_eq!(
        blood(&e, z),
        3 - 1 - 1,
        "the burn, then DeSalle's hand strike"
    );
}

/// DeSalle: once each action: burned for, the attempt matched again by
/// his Cloak the Gathering, no second burn: the bleed succeeds.
#[test]
fn desalle_lets_each_sabbat_blocker_burn_once() {
    use zvtes::crypt::Sect;
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("DeSalle"), 4, false);
    e.state.minions[d.index()].gained_levels = vec!["obf"];
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let z = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 3, false);
    e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 0), 3, false);
    hand(&mut e, 0, &["Cloak the Gathering"]);
    pick(&mut e, bleed(d));
    pick_when(&mut e, block(z));
    let burn = Choice::BurnBloodForIntercept { minion: z };
    pick_when(&mut e, burn);
    pick_when(&mut e, play(0, 0, d));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, burn);
    assert_eq!(blood(&e, z), 3 - 1);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
}

/// Dominique Santo Paulo: "+1 strength in combat with Nosferatu or
/// Nosferatu antitribu": against either clan, not a Brujah.
#[test]
fn dominique_gets_strength_against_nosferatu_of_either_clan() {
    use zvtes::cards::Stat;
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Dominique Santo Paulo"), 4, false);
    let nos = e.add_minion(SeatId(1), of_clan("Nosferatu", 4..=8, 0), 4, false);
    let anti = e.add_minion(
        SeatId(1),
        of_clan("Nosferatu antitribu", 4..=8, 0),
        4,
        false,
    );
    let brujah = e.add_minion(SeatId(1), of_clan("Brujah", 4..=8, 0), 4, false);
    assert_eq!(va_against(&e, d, Stat::Strength, nos), 1);
    assert_eq!(va_against(&e, d, Stat::Strength, anti), 1);
    assert_eq!(va_against(&e, d, Stat::Strength, brujah), 0);
}

/// Dr. Julius Sutphen (ADV): "During a directed action, Julius can burn 1
/// blood to get +1 stealth." His bleed, Lord Tremere's +1 intercept
/// attempting: 1 blood for 1 stealth, once (his Cloak the Gathering still
/// offered beside).
#[test]
fn julius_sutphen_advanced_burns_for_stealth_on_a_directed_action() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), va_adv("Dr. Julius Sutphen"), 4, false);
    e.state.minions[j.index()].gained_levels = vec!["obf"];
    hand(&mut e, 0, &["Cloak the Gathering"]);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let lord = e.add_minion(SeatId(1), find_card("Lord Tremere"), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    pick(&mut e, bleed(j));
    pick_when(&mut e, block(lord));
    vc_use_when(&mut e, j, 0);
    let req = request(&mut e);
    assert_eq!(blood(&e, j), 4 - 1);
    assert_eq!(e.action().expect("bleed").stealth(&e.state), 1);
    assert!(vc_own(&req, j, 0).is_none(), "once each action");
}

/// Dr. Julius Sutphen (ADV): on his hunt (undirected), never offered.
#[test]
fn julius_sutphen_advanced_burns_nothing_on_an_undirected_action() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), va_adv("Dr. Julius Sutphen"), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let lord = e.add_minion(SeatId(1), find_card("Lord Tremere"), 4, false);
    e.state.minions[lord.index()].gained_levels = vec!["aus"];
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    pick(&mut e, action(j, ActionKind::Hunt));
    pick_when(&mut e, block(lord));
    vc_never(&mut e, j, 0, SeatId(1), WindowLabel::MinionPhase);
}

/// Merged Julius: the base card's toll on younger blockers, the advanced
/// card's stealth, its [MERGED] "Archbishop of Atlanta: Julius gets +1
/// strength in combat with a Camarilla vampire."
#[test]
fn julius_sutphen_merged_tolls_younger_blockers_and_fights_the_camarilla() {
    use zvtes::cards::Stat;
    use zvtes::crypt::Sect;
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), va_adv("Dr. Julius Sutphen"), 4, false);
    e.state.minions[j.index()].merged = Some(find_card("Dr. Julius Sutphen"));
    let def = zvtes::cards::vampire_of(&e.state.minions[j.index()]).expect("merged");
    assert_eq!((def.printed.len(), def.abilities.len()), (2, 1));
    let cam = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    let sab = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    let young = e.add_minion(SeatId(1), vc_cap(4, 0), 4, false);
    assert_eq!(va_against(&e, j, Stat::Strength, cam), 1);
    assert_eq!(va_against(&e, j, Stat::Strength, sab), 0);
    assert_eq!(va_against(&e, young, Stat::BlockToll, j), 1);
    assert_eq!(e.state.title_of(j), Title::Archbishop("Atlanta"));
}

/// Drusilla Euphemia: "Drusilla can take control of a ready Ventrue until
/// the end of the turn as a Ⓓ action that costs 2 blood." Another
/// Methuselah's ready Ventrue only (not a torpid one, a Ventrue antitribu
/// nor her own): taken, 2 blood paid, back as the turn ends.
#[test]
fn drusilla_takes_control_of_a_ready_ventrue_until_the_turn_ends() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Drusilla Euphemia"), 4, false);
    let own = e.add_minion(SeatId(0), of_clan("Ventrue", 4..=8, 0), 4, false);
    let v = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 1), 4, false);
    let torpid = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 2), 2, true);
    let anti = e.add_minion(SeatId(1), of_clan("Ventrue antitribu", 4..=8, 0), 4, false);
    let req = request(&mut e);
    assert!(req.options.contains(&vc_at(d, Target::Minion(v))));
    for m in [own, torpid, anti] {
        assert!(!req.options.contains(&vc_at(d, Target::Minion(m))), "{m:?}");
    }
    e.choose(index_of(&req, vc_at(d, Target::Minion(v))));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[v.index()].owner, SeatId(0));
    assert_eq!(blood(&e, d), 4 - 2);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[v.index()].owner, SeatId(1));
}

/// Eric Kressida: "If Eric is in torpor during any other Methuselah's
/// discard phase, you may burn the Edge to move him to your ready region."
/// Not in his own Methuselah's discard phase; in seat 1's: the Edge burned,
/// he is ready.
#[test]
fn eric_kressida_burns_the_edge_to_leave_torpor_in_another_discard_phase() {
    let mut e = game(2);
    let eric = e.add_minion(SeatId(0), find_card("Eric Kressida"), 2, true);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    e.state.seats[0].has_edge = true;
    vc_never(&mut e, eric, 0, SeatId(1), WindowLabel::MinionPhase);
    let req = va_offered(&mut e, eric, 0);
    assert_eq!(e.phase(), zvtes::game::Phase::Discard);
    assert_eq!(e.state.active, SeatId(1));
    e.choose(index_of(&req, vc_own(&req, eric, 0).expect("offered")));
    assert!(!in_torpor(&e, eric));
    assert!(!e.state.seats[0].has_edge);
}

/// Fabrizia Contreraz: "While your prey controls no ready Camarilla
/// vampires, Fabrizia gets -2 votes." "She only gets her votes if your prey
/// controls a ready Camarilla vampire." [PIB 20150713] Her prey's
/// Camarilla vampire torpid: none; ready: her 2. "A vampire can have net
/// negative votes, but net negative votes do not subtract from the total"
/// [ANK 20251125] [LSJ 20050221-2]: made Independent (her title lapses),
/// she has none, not fewer.
#[test]
fn fabrizia_votes_only_while_her_prey_has_a_ready_camarilla_vampire() {
    use zvtes::crypt::Sect;
    let mut e = game(3);
    let f = e.add_minion(SeatId(0), find_card("Fabrizia Contreraz"), 4, false);
    e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 0), 2, true);
    e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    e.add_minion(SeatId(2), of_sect(Sect::Camarilla, 4..=8, 1), 4, false);
    assert_eq!(e.state.votes_held(f), 0);
    let cam = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 2), 4, false);
    assert_eq!(e.state.votes_held(f), 2);
    e.state.minions[cam.index()].in_torpor = true;
    e.state.minions[f.index()].sect_override = Some(Sect::Independent);
    assert_eq!(e.state.votes_held(f), 0);
}

/// Father Juan Carlos: "During a bleed action, Father Juan Carlos can burn
/// 1 blood to get -1 bleed and +1 stealth." Not before a block attempt
/// needs it; seat 1's vampire attempting, he burns 1: the attempt fails,
/// the bleed takes 1 - 1 pool; once.
#[test]
fn father_juan_carlos_trades_a_bleed_for_stealth_when_needed() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Father Juan Carlos"), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let z = e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 2), 4, false);
    pick(&mut e, bleed(j));
    vc_never(&mut e, j, 0, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, block(z));
    vc_use_when(&mut e, j, 0);
    vc_never(&mut e, j, 0, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, j), 4 - 1);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
    assert_eq!(e.state.seats[1].pool, 10);
}

/// Gravitnir: "During the polling step of a political action, if Gravitnir
/// is ready, he can burn 1 blood to force the acting vampire to abstain."
/// "The ability can be used while locked, only once per referendum." [ANK
/// 20220705] Locked, after the caller voted: its votes canceled, it
/// abstains; not offered again.
#[test]
fn gravitnir_forces_the_acting_vampire_to_abstain_once_while_locked() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), vc_titled(Title::Primogen, 0), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Gravitnir"), 3, false);
    e.state.minions[g.index()].locked = true;
    va_kine(&mut e, caller, 1);
    pick_when(&mut e, votes(caller, true));
    vc_use_when(&mut e, g, 0);
    let req = request(&mut e);
    assert_eq!(blood(&e, g), 3 - 1);
    let r = e.referendum().expect("referendum");
    assert!(r.abstaining.contains(&caller));
    assert!(r
        .records
        .iter()
        .filter(|x| x.source == zvtes::referendum::Source::Vampire(caller))
        .all(|x| x.canceled));
    assert!(vc_own(&req, g, 0).is_none(), "once each referendum");
}

/// Gravitnir: in torpor ("if Gravitnir is ready"), never offered.
#[test]
fn gravitnir_forces_nothing_from_torpor() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), vc_titled(Title::Primogen, 0), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Gravitnir"), 3, true);
    e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    va_kine(&mut e, caller, 1);
    vc_never(&mut e, g, 0, SeatId(1), WindowLabel::MinionPhase);
}

/// Harold Zettler, Pentex Director: "Giovanni get +1 bleed against you."
/// A Giovanni bleeding his controller takes 2; a Hecata 1 (rules-qa
/// 121.54).
#[test]
fn harold_zettler_lets_giovanni_bleed_his_controller_harder() {
    for (clan, lost) in [("Giovanni", 2), ("Hecata", 1)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_clan(clan, 4..=8, 0), 4, false);
        e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
        e.add_minion(
            SeatId(1),
            find_card("Harold Zettler, Pentex Director"),
            4,
            true,
        );
        pick(&mut e, bleed(a));
        to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10 - lost, "{clan}");
    }
}

/// Hektor: "Damage from Hektor's hand strikes is aggravated. Baali get +1
/// bleed against you." His hand strike sends the blocker to torpor; a
/// Baali (Giotto Verducci) bleeding his controller takes 2.
#[test]
fn hektor_strikes_aggravated_and_draws_baali_bleeds() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hektor"), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    let z = e.add_minion(SeatId(1), vc_cap(5, 1), 4, false);
    e.add_minion(SeatId(1), vc_cap(5, 2), 4, false);
    bleed_blocked(&mut e, h, z);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(in_torpor(&e, z), "1 aggravated damage");

    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Giotto Verducci"), 4, false);
    e.add_minion(SeatId(0), vc_cap(5, 0), 4, false);
    e.add_minion(SeatId(1), find_card("Hektor"), 4, true);
    pick(&mut e, bleed(b));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

// ---- Track C (action_b, master_b, action_a, vamp_b): helpers ----------------

/// The `nth` fixture prince (2 votes).
#[track_caller]
fn textless_prince(nth: usize) -> u16 {
    fixture_vampire(|c| matches!(c.title, Title::Prince(_)), nth)
}

/// The card `card` of the hand played by `actor` in `mode` on `target`.
fn play_on(card: u8, mode: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target,
    }
}

/// Pass until the ability `ability` of `source` is offered, and take it.
fn pick_use_of(e: &mut Engine, source: CardId, ability: u8) {
    for _ in 0..200 {
        let req = request(e);
        if let Some(i) = req.options.iter().position(|o| {
            matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: a, .. }
                if *c == source && *a == ability)
        }) {
            e.choose(i);
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("ability {ability} of {source:?} never offered");
}

// ---- Action cards (action_b) ------------------------------------------------

/// Patronage: "Unlock a younger Toreador antitribu and this acting vampire.
/// You may play only one Patronage per turn." Another Methuselah's locked
/// younger Toreador antitribu is offered (not an older one); both unlocked,
/// 1 pool paid, the card in the ash heap; the second copy is not offered
/// that turn.
#[test]
fn patronage_unlocks_a_younger_toreador_antitribu_and_the_actor_once_a_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Toreador antitribu", 6..=9, 0), 4, false);
    let cap = CRYPT[e.state.minions[a.index()].crypt_card().unwrap() as usize].capacity;
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let young = e.add_minion(
        SeatId(1),
        of_clan("Toreador antitribu", 1..=cap - 1, 0),
        1,
        false,
    );
    e.state.minions[young.index()].locked = true;
    let old = e.add_minion(
        SeatId(1),
        of_clan("Toreador antitribu", cap..=11, 1),
        4,
        false,
    );
    hand(&mut e, 0, &["Patronage", "Patronage"]);
    let req = request(&mut e);
    assert!(req
        .options
        .contains(&play_on(0, 0, a, Target::Minion(young))));
    assert!(
        !req.options.contains(&play_on(0, 0, a, Target::Minion(old))),
        "not younger"
    );
    e.choose(index_of(&req, play_on(0, 0, a, Target::Minion(young))));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked, "the actor unlocked");
    assert!(
        !e.state.minions[young.index()].locked,
        "the target unlocked"
    );
    assert_eq!(e.state.seats[0].pool, 9);
    assert!(ash_has(&e, 0, "Patronage"));
    assert_eq!(e.state.seats[0].hand.len(), 1);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0, .. })),
        "one Patronage a turn: {:?}",
        req.options
    );
}

/// Political Struggle on another's prince (2 votes): the card itself on
/// him, the acting Lasombra chosen; next turn its rush is offered to that
/// vampire only; the prince sent to torpor in it, the card burns and the
/// Lasombra gets +2 votes for good.
/// [LSJ 19990119] The votes gained however the vampire goes down (with the
/// next test).
#[test]
fn political_struggle_rushes_and_gives_votes_when_its_vampire_goes_down() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Lasombra", 4..=11, 0), 4, false);
    let other = e.add_minion(SeatId(0), of_clan("Lasombra", 4..=11, 1), 4, false);
    let b = e.add_minion(SeatId(1), textless_prince(0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Political Struggle"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(b)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let ps = placed_on(&e, "Political Struggle", Some(b));
    assert_eq!(e.state.cards[ps.index()].chosen, vec![a]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let rush = use_on(ps, 0, Some(a), Target::Minion(b));
    assert!(req.options.contains(&rush), "{:?}", req.options);
    assert!(
        !req.options
            .contains(&use_on(ps, 0, Some(other), Target::Minion(b))),
        "the acting vampire's only"
    );
    assert_eq!(e.state.votes_held(a), 0);
    e.state.minions[b.index()].blood = 0;
    e.choose(index_of(&req, rush));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[b.index()].in_torpor);
    assert!(ash_has(&e, 0, "Political Struggle"));
    assert!(!e.state.cards[ps.index()].in_play);
    assert_eq!(e.state.votes_held(a), 2, "the prince's 2 votes");
}

/// Political Struggle: its vampire burned out of combat, by any effect:
/// the votes still gained, the card burned.
/// [LSJ 19990119] "The acting vampire gains X votes if the vampire with
/// this card is burned, no matter how that vampire is burned."
#[test]
fn political_struggle_gives_votes_however_its_vampire_burns() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Lasombra", 4..=11, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), textless_prince(0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ps = in_play(&mut e, 0, "Political Struggle", Some(b));
    e.state.cards[ps.index()].chosen = vec![a];
    e.burn(b);
    assert!(!e.state.minions[b.index()].in_play);
    assert!(ash_has(&e, 0, "Political Struggle"));
    assert_eq!(e.state.votes_held(a), 2);
}

/// Political Struggle: the attached prince diablerized by the acting
/// vampire's Amaranth in the rush; at the blood hunt the votes are already
/// the Lasombra's.
/// [LSJ 20050707] Burned by diablerie: the votes gained before the blood
/// hunt referendum.
#[test]
fn political_struggle_votes_come_before_the_blood_hunt() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Lasombra", 4..=11, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), textless_prince(0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1].has_edge = true;
    let ps = in_play(&mut e, 0, "Political Struggle", Some(b));
    e.state.cards[ps.index()].chosen = vec![a];
    hand(&mut e, 0, &["Amaranth"]);
    pick(&mut e, use_on(ps, 0, Some(a), Target::Minion(b)));
    pick_when(&mut e, play(0, 0, a));
    to_referendum(&mut e);
    assert!(!e.state.minions[b.index()].in_play, "diablerized");
    assert!(ash_has(&e, 0, "Political Struggle"));
    assert_eq!(e.state.votes_held(a), 2, "gained before the blood hunt");
}

/// Political Struggle's X: bonus votes counted (Legendary Vampire's +2),
/// conditional ones not (Aura of Invincibility's counters, Sundown's), a
/// priscus ballot not.
/// [ANK 20211009] [LSJ 20040518-2] Bonus votes yes, conditional votes no;
/// the mention rows of Legendary Vampire, Aura of Invincibility, Sundown.
/// [ANK 20211009] [LSJ 20041025] Priscus ballots not counted.
#[test]
fn political_struggle_counts_bonus_not_conditional_votes() {
    // (victim setup, expected X)
    for case in 0..4 {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_clan("Lasombra", 4..=11, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let (v, x) = match case {
            // Legendary Vampire: +2 votes on an untitled vampire.
            0 => {
                let v = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
                in_play(&mut e, 1, "Legendary Vampire", Some(v));
                (v, 2)
            }
            // Aura of Invincibility: votes in referendums its prince calls.
            1 => {
                let v = e.add_minion(SeatId(1), textless_prince(0), 4, false);
                let c = in_play(&mut e, 1, "Aura of Invincibility", Some(v));
                e.state.layers.push(zvtes::cards::Layer {
                    scope: zvtes::cards::Scope::InPlay,
                    modifier: zvtes::cards::Modifier::HostVotesPerCounterWhenCalling,
                    source: c,
                });
                e.state.cards[c.index()].counters = 2;
                (v, 2)
            }
            // Sundown: his conditional votes not, his title's yes.
            2 => {
                let base = (0..CRYPT.len() as u16)
                    .find(|&i| CRYPT[i as usize].name == "Sundown" && !CRYPT[i as usize].adv)
                    .expect("Sundown");
                let s = e.add_minion(SeatId(1), base, 4, false);
                e.state.minions[s.index()].granted_title = Some(Title::Prince("Paris"));
                (s, 2)
            }
            // A priscus: no votes, his ballot not counted.
            _ => {
                let p = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 2..=5, 0), 3, false);
                e.state.minions[p.index()].granted_title = Some(Title::Priscus);
                assert_eq!(e.state.ballots(p), 1);
                (p, 0)
            }
        };
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let ps = in_play(&mut e, 0, "Political Struggle", Some(v));
        e.state.cards[ps.index()].chosen = vec![a];
        e.burn(v);
        assert!(ash_has(&e, 0, "Political Struggle"), "case {case}");
        assert_eq!(e.state.votes_held(a), x, "case {case}");
    }
}

/// Political Struggle's votes are bonus votes the Lasombra casts in a
/// later referendum.
#[test]
fn political_struggle_votes_are_cast_in_referendums() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Lasombra", 4..=11, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), textless_prince(0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ps = in_play(&mut e, 0, "Political Struggle", Some(b));
    e.state.cards[ps.index()].chosen = vec![a];
    e.burn(b);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, a));
    to_referendum(&mut e);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, a), 2);
}

/// Pulse of the Canaille [aus]: every other Methuselah's hand visible to
/// the acting Methuselah for the rest of the turn, after a later action
/// too, a card drawn since included; not the next turn.
#[test]
fn pulse_of_the_canaille_shows_every_hand_until_the_turn_ends() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("aus", 0), 5, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Pulse of the Canaille"]);
    hand(&mut e, 1, &["Conceal"]);
    assert!(!e.state.hand_visible_to(SeatId(1), SeatId(0)));
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Pulse of the Canaille"));
    for s in [1, 2] {
        assert!(e.state.hand_visible_to(SeatId(s), SeatId(0)), "seat {s}");
    }
    assert!(
        !e.state.hand_visible_to(SeatId(0), SeatId(1)),
        "the actor's only"
    );
    pick(&mut e, action(other, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.seats[2].hand.push(find_library("Deflection"));
    assert!(
        e.state.hand_visible_to(SeatId(2), SeatId(0)),
        "after an action"
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !e.state.hand_visible_to(SeatId(1), SeatId(0)),
        "the turn over"
    );
}

/// Pulse of the Canaille [AUS]: the card itself on the acting vampire, +2
/// bleed; "a vampire can have only one": a second copy's [AUS] not offered
/// to it, its [aus] still.
#[test]
fn pulse_of_the_canaille_superior_goes_on_the_actor_for_two_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("AUS", 0), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(
        &mut e,
        0,
        &["Pulse of the Canaille", "Pulse of the Canaille"],
    );
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Pulse of the Canaille", Some(a));
    assert_eq!(e.state.bearer_mod(a, Stat::Bleed, ActionKind::Bleed), 2);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&play(0, 0, a)), "{:?}", req.options);
    assert!(!req.options.contains(&play(0, 1, a)), "only one");
}

/// Purity of the Beast: a rush at the prey's ally only (not a predator's,
/// not a vampire), not by an ally; +2 strength in that combat: a
/// 1-strength vampire's hand strike burns a 3-life ally.
#[test]
fn purity_of_the_beast_fights_a_preys_ally_at_plus_two_strength() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    assert_eq!(e.state.base_strength(a), 1);
    let own_ally = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    let prey_ally = e.add_ally(SeatId(1), find_library("Gravebound Drone"));
    e.state.minions[prey_ally.index()].blood = 3;
    let prey_vampire = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let pred_ally = e.add_ally(SeatId(2), find_library("Gravebound Drone"));
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Purity of the Beast"]);
    let req = request(&mut e);
    assert!(req
        .options
        .contains(&play_on(0, 0, a, Target::Minion(prey_ally))));
    for (actor, t) in [(a, pred_ally), (a, prey_vampire), (own_ally, prey_ally)] {
        assert!(
            !req.options
                .contains(&play_on(0, 0, actor, Target::Minion(t))),
            "{actor:?} at {t:?}"
        );
    }
    e.choose(index_of(&req, play_on(0, 0, a, Target::Minion(prey_ally))));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[prey_ally.index()].in_play, "3 damage");
    assert!(ash_has(&e, 1, "Gravebound Drone"));
}

/// Rave: from the acting vampire, 2 blood to one vampire, 1 to another
/// (locked), each offered once for blood; its weapon to the locked one.
/// Moves one at a time (rules-qa 68.10).
#[test]
fn rave_moves_blood_and_equipment_from_the_actor_to_several_vampires() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let r1 = e.add_minion(SeatId(0), plain("Brujah", 1), 1, false);
    let r2 = e.add_minion(SeatId(0), plain("Brujah", 2), 1, false);
    e.state.minions[r2.index()].locked = true;
    let torpid = e.add_minion(SeatId(0), plain("Brujah", 3), 1, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let hook = in_play(&mut e, 0, "Meat Hook", Some(a));
    hand(&mut e, 0, &["Rave"]);
    pick(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            Choice::ChooseMinion(a),
            Choice::ChooseCard(hook.0)
        ]
    );
    e.choose(index_of(&req, Choice::ChooseMinion(a)));
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::ChooseMinion(r1)));
    assert!(
        req.options.contains(&Choice::ChooseMinion(r2)),
        "locked too"
    );
    assert!(
        !req.options.contains(&Choice::ChooseMinion(torpid)),
        "ready"
    );
    e.choose(index_of(&req, Choice::ChooseMinion(r1)));
    pick(&mut e, set_x(2));
    pick(&mut e, Choice::ChooseMinion(a));
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::ChooseMinion(r1)), "once");
    e.choose(index_of(&req, Choice::ChooseMinion(r2)));
    pick(&mut e, set_x(1));
    pick(&mut e, Choice::ChooseCard(hook.0));
    pick(&mut e, Choice::ChooseMinion(r2));
    pick(&mut e, Choice::Pass);
    request(&mut e);
    assert_eq!((blood(&e, a), blood(&e, r1), blood(&e, r2)), (1, 3, 2));
    assert_eq!(e.state.cards[hook.index()].host, Some(r2));
    assert!(ash_has(&e, 0, "Rave"));
}

/// Rave's equipment only to a vampire able to hold it: a vehicle not to a
/// vampire with one ("a vampire may have only one vehicle").
#[test]
fn rave_moves_equipment_only_to_a_vampire_able_to_hold_it() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let r1 = e.add_minion(SeatId(0), plain("Brujah", 1), 1, false);
    let r2 = e.add_minion(SeatId(0), plain("Brujah", 2), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let bike = in_play(&mut e, 0, "Sport Bike", Some(a));
    in_play(&mut e, 0, "Unlicensed Taxicab", Some(r1));
    hand(&mut e, 0, &["Rave"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::ChooseCard(bike.0));
    let req = request(&mut e);
    assert_eq!(req.options, vec![Choice::Pass, Choice::ChooseMinion(r2)]);
}

/// Rave by an ally: "(or life)": its 2 life to a vampire, the ally burned
/// (rules-qa 73.5).
#[test]
fn rave_by_an_ally_moves_its_life_and_burns_at_none() {
    let mut e = game(2);
    let ally = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Rave"]);
    pick(&mut e, play(0, 0, ally));
    pick_when(&mut e, Choice::ChooseMinion(ally));
    pick(&mut e, Choice::ChooseMinion(v));
    pick(&mut e, set_x(2));
    request(&mut e);
    assert_eq!(blood(&e, v), 3);
    assert!(!e.state.minions[ally.index()].in_play, "burned");
    assert!(ash_has(&e, 0, "Gravebound Drone"));
}

/// Recruiting Party: each Ventrue antitribu in the acting Methuselah's
/// uncontrolled region gains 1 blood, not a Ventrue; not offered to a
/// non-Ventrue antitribu (its clan requirement); no stealth: a
/// 0-intercept block reaches it.
#[test]
fn recruiting_party_feeds_each_uncontrolled_ventrue_antitribu() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ventrue antitribu", 4..=11, 0), 4, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let slot = |card| zvtes::state::UncontrolledSlot {
        card,
        copy: Default::default(),
        counters: 0,
        banished: None,
    };
    e.state.seats[0].uncontrolled = vec![
        slot(of_clan("Ventrue antitribu", 1..=11, 1)),
        slot(of_clan("Ventrue", 1..=11, 0)),
        slot(of_clan("Ventrue antitribu", 1..=11, 2)),
    ];
    hand(&mut e, 0, &["Recruiting Party"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play(0, 0, other)),
        "Ventrue antitribu"
    );
    e.choose(index_of(&req, play(0, 0, a)));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(b)), "stealth 0");
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let counters: Vec<u8> = e.state.seats[0]
        .uncontrolled
        .iter()
        .map(|s| s.counters)
        .collect();
    assert_eq!(counters, vec![1, 0, 1]);
}

/// Recure of the Homeland by a locked torpid Tremere antitribu: out of
/// torpor, no blood paid.
/// [ANK 20181017] Not the default action: no blood paid to leave torpor.
#[test]
fn recure_of_the_homeland_leaves_torpor_locked_and_for_free() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Tremere antitribu", 4..=11, 0), 1, true);
    e.state.minions[a.index()].locked = true;
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Recure of the Homeland"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].in_torpor);
    assert_eq!(blood(&e, a), 1, "nothing paid");
    assert!(ash_has(&e, 0, "Recure of the Homeland"));
}

/// Recure of the Homeland blocked: no combat, the blocking vampire offered
/// the diablerie.
#[test]
fn recure_of_the_homeland_blocked_offers_the_diablerie() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Tremere antitribu", 4..=11, 0), 1, true);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Recure of the Homeland"]);
    hand(&mut e, 1, &["Spirit's Touch"]);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play_on(0, 0, b, Target::None));
    let req = until(&mut e, SeatId(1), WindowLabel::TorporBlock);
    assert_eq!(req.window, WindowLabel::TorporBlock);
    assert!(req.options.contains(&Choice::Diablerise));
    assert!(e.combat().is_none(), "no combat");
}

/// Recure of the Homeland, its vampire out of torpor before it resolves:
/// it fizzles.
/// [ANK 20220218] Only usable in torpor: leaving torpor before resolution
/// fizzles it.
#[test]
fn recure_of_the_homeland_fizzles_once_out_of_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Tremere antitribu", 4..=11, 0), 2, true);
    e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Recure of the Homeland"]);
    pick(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::ActionBlock);
    e.state.minions[a.index()].in_torpor = false;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
}

/// A torpor action card is a locked vampire's only if printed (Recure of the
/// Homeland: "Usable by a locked vampire"; Movement of the Slow Body: "Only
/// usable by a locked vampire in torpor"): Rapid Healing is not offered to
/// a locked torpid vampire, offered once it is unlocked (rulebook "Only
/// unlocked minions can perform actions").
#[test]
fn torpor_cards_need_an_unlocked_vampire_unless_printed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("for", 0), 2, true);
    e.state.minions[a.index()].locked = true;
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Rapid Healing"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, a)), "locked");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("for", 0), 2, true);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Rapid Healing"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, a)), "unlocked");
}

/// Restructure [dem] on another Methuselah's locked ally: the card on it; in
/// that Methuselah's unlock phase the ally stays locked, its controller
/// offered to burn 1 pool to unlock it.
#[test]
fn restructure_holds_an_ally_locked_until_its_controller_pays() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.capacity >= 4 && c.disciplines.contains(&"dem"), 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_ally(SeatId(1), find_library("Gravebound Drone"));
    e.state.minions[t.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Restructure"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Restructure", Some(t));
    assert_eq!(blood(&e, a), 3, "1 blood");
    let mut used = false;
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
            break;
        }
        if let Some(&u) = uses(&req, card.0).first() {
            assert_eq!(req.seat, SeatId(1), "the ally's controller");
            assert!(e.state.minions[t.index()].locked, "not unlocked as normal");
            pick(&mut e, u);
            used = true;
            continue;
        }
        e.choose(fallback(&req));
    }
    assert!(used);
    assert!(!e.state.minions[t.index()].locked);
    assert_eq!(e.state.seats[1].pool, 9);
}

/// Restructure [DEM]: another Methuselah's ally of a clan the acting
/// Methuselah has none of, stolen.
/// [TOM 19960226-1] Requirements do not apply when taking control of a card
/// already in play.
#[test]
fn restructure_superior_steals_an_ally_without_its_requirements() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("DEM", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let clan_ally = (0..LIBRARY.len() as u16)
        .find(|&c| {
            let l = &LIBRARY[c as usize];
            l.types == [CardType::Ally]
                && !l.clans.is_empty()
                && !l.clans.contains(&"Brujah")
                && zvtes::cards::lookup(c).is_some()
                && l.body.is_some()
        })
        .expect("a clan ally");
    let t = e.add_ally(SeatId(1), clan_ally);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Restructure"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[t.index()].owner, SeatId(0));
    assert!(ash_has(&e, 0, "Restructure"));
}

/// Shadow of the Beast [PRO]: the card itself on the acting vampire; in its
/// next combat a maneuver from the card (once) and a press; "only one": a
/// second copy not offered to it.
#[test]
fn shadow_of_the_beast_gives_a_maneuver_and_superior_a_press() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("PRO", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Shadow of the Beast", "Shadow of the Beast"]);
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Shadow of the Beast", Some(a));
    assert_eq!(blood(&e, a), 4, "1 blood");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&play(0, 0, a)), "only one");
    assert!(!req.options.contains(&play(0, 1, a)), "only one");
    e.choose(index_of(&req, bleed(a)));
    pick_when(&mut e, block(z));
    pick_use_of(&mut e, card, 0);
    let mut pressed = false;
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
            break;
        }
        assert!(uses(&req, card.0).is_empty(), "once each combat");
        if req.seat == SeatId(0) && req.options.contains(&Choice::UsePress) {
            pressed = true;
            e.choose(index_of(&req, Choice::UsePress));
            continue;
        }
        e.choose(fallback(&req));
    }
    assert!(pressed, "a press offered");
}

/// Sibyl's Tongue: X the copies in the actor's Methuselah's ash heap: two,
/// not offered to a 1-blood actor; the named card searched to hand, 2 blood
/// paid, the hand discarded down.
#[test]
fn sibyls_tongue_costs_a_blood_per_copy_in_the_ash_heap_and_fetches_the_named_card() {
    let mut e = game(2);
    let poor = e.add_minion(
        SeatId(0),
        of_clan("Malkavian antitribu", 3..=11, 0),
        1,
        false,
    );
    let a = e.add_minion(
        SeatId(0),
        of_clan("Malkavian antitribu", 3..=11, 1),
        3,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let tongue = find_library("Sibyl's Tongue");
    e.state.seats[0].ash = vec![tongue, tongue].into();
    hand(&mut e, 0, &["Sibyl's Tongue"]);
    fill_hand(&mut e, 0);
    let conceal = find_library("Conceal");
    e.state.seats[0].library = vec![
        find_library("Deflection"),
        conceal,
        find_library("Deflection"),
    ]
    .into();
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, poor)), "X = 2");
    e.choose(index_of(&req, play(0, 0, a)));
    pick(&mut e, Choice::FindCard(conceal));
    pick_when(&mut e, Choice::FindCard(conceal));
    until(&mut e, SeatId(0), WindowLabel::DiscardDown);
    assert!(e.state.seats[0].hand.contains(&conceal));
    assert!(!e.state.seats[0].library.contains(&conceal));
    assert_eq!(blood(&e, a), 1, "2 blood");
}

/// Sibyl's Tongue with none in the ash heap: free; a name not in the
/// library moves nothing.
#[test]
fn sibyls_tongue_is_free_with_none_in_the_ash_heap() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        of_clan("Malkavian antitribu", 3..=11, 0),
        2,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Sibyl's Tongue"]);
    e.state.seats[0].library = vec![find_library("Deflection")].into();
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::FindCard(find_library("Conceal")));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 2, "X = 0");
    assert!(!e.state.seats[0].hand.contains(&find_library("Conceal")));
    assert!(ash_has(&e, 0, "Sibyl's Tongue"));
}

/// Sunrise Service: the card itself on its Sabbat vampire, then 1
/// unpreventable aggravated damage, which a ready vampire cannot mend: to
/// torpor, the card with it; not offered to a non-Sabbat vampire.
#[test]
fn sunrise_service_wounds_its_sabbat_vampire_and_stays_on_it() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 0), 3, false);
    let cam = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=11, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Sunrise Service"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, cam)), "Sabbat");
    e.choose(index_of(&req, play(0, 0, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Sunrise Service", Some(a));
    assert!(e.state.minions[a.index()].in_torpor, "aggravated");
    assert_eq!(blood(&e, a), 3, "not mended");
}

/// Sunrise Service: burned as its vampire declares a bleed, the bleed is
/// unblockable: no block offered to an able blocker; the card in the ash
/// heap.
#[test]
fn sunrise_service_burns_to_make_an_action_unblockable() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = in_play(&mut e, 0, "Sunrise Service", Some(a));
    pick(&mut e, bleed(a));
    pick_use_of(&mut e, card, 0);
    assert!(ash_has(&e, 0, "Sunrise Service"));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, block(b));
    assert_eq!(e.state.seats[1].pool, 9, "bled");
}

/// Total Insanity [dem] on another Methuselah's younger vampire (not an
/// older one): the card on it, -1 stealth; a counter in each of its
/// controller's unlock phases (not the player's), burned at the third.
#[test]
fn total_insanity_dulls_stealth_and_burns_after_three_unlock_phases() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.capacity >= 7 && c.disciplines.contains(&"dem"), 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), textless(|c| c.capacity <= 4, 0), 4, false);
    let old = e.add_minion(SeatId(1), textless(|c| c.capacity >= 9, 0), 4, false);
    hand(&mut e, 0, &["Total Insanity"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play_on(0, 0, a, Target::Minion(old))),
        "younger"
    );
    e.choose(index_of(&req, play_on(0, 0, a, Target::Minion(t))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Total Insanity", Some(t));
    assert_eq!(e.state.bearer_mod(t, Stat::Stealth, ActionKind::Bleed), -1);
    for n in 1..=2 {
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.cards[card.index()].counters, n);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.cards[card.index()].counters, n, "not the player's");
    }
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[card.index()].in_play, "burned at 3");
    assert!(ash_has(&e, 0, "Total Insanity"));
}

/// Total Insanity [DEM] on an ally: its action cards never offered (another
/// minion's still), its bleed still is.
#[test]
fn total_insanity_superior_bars_action_cards() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("DEM", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Gravebound Drone"));
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Total Insanity"]);
    hand(&mut e, 1, &["Rave"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(ally)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Total Insanity", Some(ally));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&play(0, 0, ally)), "no action card");
    assert!(req.options.contains(&play(0, 0, v)));
    assert!(req.options.contains(&bleed(ally)));
}

/// War Party: an archbishop puts it on another's untitled vampire (not on a
/// cardinal; not played by an untitled vampire); a third Methuselah's
/// vampire is offered the rush and sends it to torpor; the bearer's own
/// Methuselah's vampires are not offered it.
#[test]
fn war_party_lets_any_vampire_rush_its_bearer() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 0), 4, false);
    e.state.minions[a.index()].granted_title = Some(Title::Archbishop("Paris"));
    let untitled = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    let own = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let cardinal = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=11, 2), 4, false);
    e.state.minions[cardinal.index()].granted_title = Some(Title::Cardinal);
    let w = e.add_minion(SeatId(2), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 1), 4, false);
    hand(&mut e, 0, &["War Party"]);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&play_on(0, 0, a, Target::Minion(cardinal))));
    assert!(!req
        .options
        .contains(&play_on(0, 0, untitled, Target::Minion(t))));
    e.choose(index_of(&req, play_on(0, 0, a, Target::Minion(t))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = placed_on(&e, "War Party", Some(t));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(uses(&req, card.0).is_empty(), "{:?}", req.options);
    let _ = own;
    let req = until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    let rush = use_on(card, 0, Some(w), Target::Minion(t));
    assert!(req.options.contains(&rush), "{:?}", req.options);
    e.state.minions[t.index()].blood = 0;
    e.choose(index_of(&req, rush));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.index()].in_torpor);
}

// ---- Master cards (master_b) ------------------------------------------------

/// The `nth` library card whose first mode requires Discipline `d`.
fn requiring(d: &str, nth: usize) -> u16 {
    LIBRARY
        .iter()
        .enumerate()
        .filter(|(_, c)| c.modes.first().is_some_and(|m| m.tags.contains(&d)))
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a card requiring it")
}

/// A path played at a master phase: `clan`'s cards requiring `disc` 1
/// blood cheaper, any Methuselah's, `other`'s not; another Methuselah's
/// vampire burns it as an action and takes 1 damage.
fn path_discounts_and_bites(name: &str, clan: &str, disc: &str, other: &str) {
    let mut e = game(2);
    let own = e.add_minion(SeatId(0), of_clan(clan, 1..=11, 0), 4, false);
    let theirs = e.add_minion(SeatId(1), of_clan(clan, 1..=11, 1), 4, false);
    let not = e.add_minion(SeatId(0), plain(other, 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = requiring(disc, 0);
    hand(&mut e, 0, &[name]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 9);
    let path = placed_on(&e, name, None);
    let off = |m| e.state.printed_cost_off(m, card, 0).blood;
    assert_eq!((off(own), off(theirs), off(not)), (1, 1, 0));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let burn = use_on(path, 0, Some(z), Target::None);
    assert!(req.options.contains(&burn), "{:?}", req.options);
    e.choose(index_of(&req, burn));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[path.index()].in_play);
    assert!(ash_has(&e, 0, name));
    assert_eq!(blood(&e, z), 3, "1 unpreventable damage");
}

/// The Path of Night: "Cards requiring Obtenebration [obt] cost Lasombra 1
/// fewer blood. Any minion can burn this card as a Ⓓ action that inflicts
/// 1 unpreventable environmental damage on acting vampires."
#[test]
fn the_path_of_night_discounts_obtenebration_and_bites_its_burner() {
    path_discounts_and_bites("Path of Night, The", "Lasombra", "obt", "Brujah");
}

/// The Path of the Feral Heart: "Cards that require Protean [pro] cost
/// Gangrel antitribu 1 less blood … if that minion is a vampire, he or she
/// takes 1 unpreventable damage when this card is burned." Not a Gangrel's.
#[test]
fn the_path_of_the_feral_heart_discounts_protean_for_gangrel_antitribu() {
    path_discounts_and_bites(
        "Path of the Feral Heart, The",
        "Gangrel antitribu",
        "pro",
        "Gangrel",
    );
}

/// Pentex™ Loves You!: locked during its controller's Sabbat vampire's
/// bleed to choose it, the vampire burns 1 blood for +1 bleed, once.
/// [ANK 20231124] [LSJ 19970718] "The ability to increase bleed can only
/// be used during a bleed action before Action Resolution": in a hunt the
/// chosen vampire is offered nothing.
#[test]
fn pentex_loves_you_locks_for_a_sabbat_bleeder_to_burn_for_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[b.index()].locked = true;
    let p = in_play(&mut e, 0, "Pentex(TM) Loves You!", None);
    let burn = use_on(p, 1, Some(a), Target::None);
    pick(&mut e, bleed(a));
    pick_when(&mut e, use_on(p, 0, None, Target::Minion(a)));
    assert!(e.state.cards[p.index()].locked);
    pick_when(&mut e, burn);
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, burn);
    assert_eq!(blood(&e, a), 3);
    assert_eq!(e.state.seats[1].pool, 8, "bled for 2");

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[b.index()].locked = true;
    let p = in_play(&mut e, 0, "Pentex(TM) Loves You!", None);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, use_on(p, 0, None, Target::Minion(a)));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        use_on(p, 1, Some(a), Target::None),
    );
}

/// Pentex™ Loves You!: "once during the current action": the vampire chosen
/// during another's action is offered nothing in its own next action, the
/// card still locked.
#[test]
fn pentex_loves_you_choice_lasts_the_action_only() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 0), 4, false);
    let c = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[b.index()].locked = true;
    let p = in_play(&mut e, 0, "Pentex(TM) Loves You!", None);
    pick(&mut e, bleed(a));
    pick_when(&mut e, use_on(p, 0, None, Target::Minion(c)));
    pick_when(&mut e, bleed(c));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        use_on(p, 1, Some(c), Target::None),
    );
    assert_eq!(e.state.seats[1].pool, 8, "two bleeds of 1");
}

/// Powerbase: Mexico City: played with 5 blood; each of its controller's
/// unlock phases moves 1 to their pool; the last one moved, it burns.
#[test]
fn powerbase_mexico_city_pays_a_blood_each_unlock_and_burns_empty() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Powerbase: Mexico City"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    request(&mut e);
    let pb = placed_on(&e, "Powerbase: Mexico City", None);
    assert_eq!(e.state.cards[pb.index()].counters, 5);
    assert_eq!(e.state.seats[0].pool, 8);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[pb.index()].counters, 5, "not the other's");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[pb.index()].counters, 4);
    assert_eq!(e.state.seats[0].pool, 9);
    e.state.cards[pb.index()].counters = 1;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10);
    assert!(!e.state.cards[pb.index()].in_play);
    assert!(ash_has(&e, 0, "Powerbase: Mexico City"));
}

/// Powerbase: Mexico City: another Methuselah's Sabbat vampire moves all its
/// blood to its controller's pool as a directed action, and it burns
/// empty; their non-Sabbat vampire is not offered the action.
/// [LSJ 20080604] The action requires a Sabbat vampire.
#[test]
fn powerbase_mexico_city_is_raided_by_another_sabbat_vampire() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let s = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=11, 0), 4, false);
    let cam = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let pb = in_play(&mut e, 0, "Powerbase: Mexico City", None);
    e.state.cards[pb.index()].counters = 3;
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&granted(pb.0, cam)), "Sabbat only");
    e.choose(index_of(&req, granted(pb.0, s)));
    let req = until(&mut e, SeatId(0), WindowLabel::ActionBlock);
    assert_eq!(
        e.action().and_then(|a| a.directed_seat(&e.state)),
        Some(SeatId(0))
    );
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 13);
    assert!(!e.state.cards[pb.index()].in_play);
    assert!(ash_has(&e, 0, "Powerbase: Mexico City"));
}

/// Purchase Pact: played only with a ready archbishop, priscus, cardinal or
/// regent; locked before range ends a combat between its controller's
/// Sabbat vampire and another's.
#[test]
fn purchase_pact_ends_a_sabbat_combat_before_range() {
    let mut e = game(2);
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Purchase Pact"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { .. })
    });

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 0), 4, false);
    e.state.minions[a.index()].granted_title = Some(Title::Archbishop("Paris"));
    let b = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=11, 1), 4, false);
    hand(&mut e, 0, &["Purchase Pact"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    request(&mut e);
    let pp = placed_on(&e, "Purchase Pact", None);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::CombatBeforeRange);
    e.choose(index_of(&req, use_on(pp, 0, None, Target::None)));
    request(&mut e);
    assert!(e.combat().is_none(), "combat ended");
    assert_eq!((blood(&e, a), blood(&e, b)), (4, 4), "no strikes");
    assert!(e.state.cards[pp.index()].locked);
}

/// Purchase Pact: any titled Sabbat vampire may call a referendum to burn it
/// as a +1 stealth political action, not an untitled one; passed, it burns.
/// [LSJ 20080604] The action requires a titled Sabbat vampire.
#[test]
fn purchase_pact_is_burned_by_a_titled_sabbat_referendum() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 0), 4, false);
    e.state.minions[a.index()].granted_title = Some(Title::Bishop);
    let u = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let pp = in_play(&mut e, 1, "Purchase Pact", None);
    let req = request(&mut e);
    assert!(!req.options.contains(&granted(pp.0, u)), "untitled");
    e.choose(index_of(&req, use_on(pp, 1, Some(a), Target::None)));
    assert_eq!(e.action().map(|x| x.stealth(&e.state)), Some(1));
    to_referendum(&mut e);
    pick_when(&mut e, votes(a, true));
    close_polling(&mut e);
    request(&mut e);
    assert!(!e.state.cards[pp.index()].in_play);
    assert!(ash_has(&e, 1, "Purchase Pact"));
}

/// Recruitment: "Search your crypt for a vampire. Show it to all players and
/// place it face down in your uncontrolled region." Any vampire, no blood.
#[test]
fn recruitment_moves_a_searched_vampire_to_the_uncontrolled_region() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let (v, w) = (plain("Toreador", 0), plain("Nosferatu", 0));
    e.state.seats[0].crypt.push(v);
    e.state.seats[0].crypt.push(w);
    hand(&mut e, 0, &["Recruitment"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::AshVampire(v)));
    assert!(req.options.contains(&Choice::AshVampire(w)));
    e.choose(index_of(&req, Choice::AshVampire(w)));
    request(&mut e);
    let slot = e.state.seats[0].uncontrolled.last().expect("placed");
    assert_eq!((slot.card, slot.counters), (w, 0));
    assert!(!e.state.seats[0].crypt.contains(&w));
    assert_eq!(e.state.seats[0].pool, 8);
}

/// Recruitment and Coroner's Contact search a crypt: not offered with an
/// empty one, nothing to search (advanced rules §1.6.5; rules-qa 78.20,
/// 80.9, 113.14); offered once it holds a vampire.
#[test]
fn recruitment_and_coroners_contact_need_a_crypt_to_search() {
    for stocked in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if stocked {
            e.state.seats[0].crypt.push(plain("Toreador", 0));
        }
        hand(&mut e, 0, &["Recruitment", "Coroner's Contact"]);
        e.at_master_phase();
        let req = request(&mut e);
        for card in [0, 1] {
            assert_eq!(req.options.contains(&master_play(card)), stocked, "{card}");
        }
    }
}

/// Sermon of Caine: from one ready vampire to the others, one move at a
/// time, each recipient once (rules-qa 68.10); a torpid one not; the excess
/// over capacity to the bank (rules-qa 57.7).
#[test]
fn sermon_of_caine_spreads_one_vampires_blood_to_the_others() {
    let mut e = game(2);
    let src = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let r1 = e.add_minion(SeatId(0), plain("Brujah", 1), 1, false);
    e.state.minions[r1.index()].locked = true;
    let r2 = e.add_minion(SeatId(0), plain("Brujah", 2), 0, false);
    let cap2 = CRYPT[e.state.minions[r2.index()].crypt_card().unwrap() as usize].capacity;
    e.state.minions[r2.index()].blood = cap2 - 1;
    let torpid = e.add_minion(SeatId(0), plain("Brujah", 3), 0, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Sermon of Caine"]);
    e.at_master_phase();
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(src),
        },
    );
    pick_when(&mut e, Choice::ChooseMinion(src));
    let req = request(&mut e);
    assert!(
        req.options.contains(&Choice::ChooseMinion(r1)),
        "locked too"
    );
    assert!(
        !req.options.contains(&Choice::ChooseMinion(torpid)),
        "ready"
    );
    e.choose(index_of(&req, Choice::ChooseMinion(r1)));
    pick(&mut e, set_x(1));
    pick(&mut e, Choice::ChooseMinion(src));
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::ChooseMinion(r1)), "once");
    e.choose(index_of(&req, Choice::ChooseMinion(r2)));
    pick(&mut e, set_x(2));
    request(&mut e);
    assert_eq!(
        (blood(&e, src), blood(&e, r1), blood(&e, r2)),
        (1, 2, cap2),
        "the excess to the bank"
    );
    assert!(ash_has(&e, 0, "Sermon of Caine"));
}

/// Sermon of Caine names a recipient: with one ready vampire (the other
/// torpid) it is not offered.
#[test]
fn sermon_of_caine_needs_another_ready_vampire() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 0, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Sermon of Caine"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { .. })
    });
}

/// Slave Auction: in play with a blood per Methuselah in the game (4); each
/// master phase action moves 1 to its controller's pool; empty, it burns.
#[test]
fn slave_auction_holds_a_blood_per_methuselah_and_pays_them_out() {
    let mut e = game(4);
    for s in 0..4 {
        e.add_minion(SeatId(s), plain("Brujah", s as usize), 4, false);
    }
    hand(&mut e, 0, &["Slave Auction"]);
    e.at_master_phase();
    e.state.seats[0].mpa = 3;
    pick(&mut e, master_play(0));
    request(&mut e);
    let sa = placed_on(&e, "Slave Auction", None);
    assert_eq!(e.state.cards[sa.index()].counters, 4);
    assert_eq!(e.state.seats[0].pool, 9);
    pick(&mut e, ability(sa.0, 0));
    request(&mut e);
    assert_eq!(e.state.cards[sa.index()].counters, 3);
    assert_eq!(e.state.seats[0].pool, 10);
    e.state.cards[sa.index()].counters = 1;
    pick(&mut e, ability(sa.0, 0));
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 11);
    assert!(!e.state.cards[sa.index()].in_play);
    assert!(ash_has(&e, 0, "Slave Auction"));
}

/// Spirit Summoning Chamber: locked, a ready Tremere antitribu burns 1
/// blood to search for a minion card requiring Blood Sorcery, shown, to
/// hand; a card without it not found. Not offered without a Tremere
/// antitribu with blood.
#[test]
fn spirit_summoning_chamber_burns_tremere_antitribu_blood_for_a_sorcery_card() {
    let tha: Vec<u16> = LIBRARY
        .iter()
        .enumerate()
        .filter(|(_, c)| c.types == [CardType::Action] && c.disciplines.contains(&"tha"))
        .map(|(i, _)| i as u16)
        .take(2)
        .collect();
    let dodge = find_library("Dodge");
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), of_clan("Tremere antitribu", 1..=11, 0), 0, false);
    e.state.minions[t.index()].locked = true;
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ssc = in_play(&mut e, 0, "Spirit Summoning Chamber", None);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&ability(ssc.0, 0)),
        "no blood to burn"
    );

    let mut e = game(2);
    let t = e.add_minion(SeatId(0), of_clan("Tremere antitribu", 3..=11, 0), 3, false);
    e.state.minions[t.index()].locked = true;
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = [dodge, tha[0], tha[1], dodge].iter().copied().collect();
    let ssc = in_play(&mut e, 0, "Spirit Summoning Chamber", None);
    pick(&mut e, ability(ssc.0, 0));
    let req = request(&mut e);
    assert!(
        req.options.contains(&Choice::FindCard(tha[1])),
        "{:?}",
        req.options
    );
    assert!(!req.options.contains(&Choice::FindCard(dodge)));
    e.choose(index_of(&req, Choice::FindCard(tha[1])));
    request(&mut e);
    assert!(e.state.seats[0].hand.contains(&tha[1]));
    assert_eq!(blood(&e, t), 2, "a locked one pays too");
    assert!(e.state.cards[ssc.index()].locked);
}

/// Spirit Summoning Chamber "to search your library": not offered with an
/// empty library (rules-qa 121.117, 78.20); offered with a card in it.
#[test]
fn spirit_summoning_chamber_needs_a_library_to_search() {
    for stocked in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), of_clan("Tremere antitribu", 3..=11, 0), 3, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        fill_hand(&mut e, 0);
        if stocked {
            e.state.seats[0].library = [find_library("Dodge")].iter().copied().collect();
        }
        let ssc = in_play(&mut e, 0, "Spirit Summoning Chamber", None);
        let req = request(&mut e);
        assert_eq!(req.options.contains(&ability(ssc.0, 0)), stocked);
    }
}

/// Strained Vitae Supply: the rulebook hunt has no stealth; a card's hunt
/// keeps its +1.
/// [PIB 20120204] "The effect only applies to the default hunt action
/// provided by the rulebook."
#[test]
fn strained_vitae_supply_strips_the_rulebook_hunt_stealth() {
    for (supply, abactor) in [(false, false), (true, false), (true, true)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_clan("Gangrel antitribu", 1..=11, 0), 1, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if supply {
            in_play(&mut e, 1, "Strained Vitae Supply", None);
        }
        if abactor {
            hand(&mut e, 0, &["Abactor"]);
            pick(&mut e, play(0, 0, a));
        } else {
            pick(&mut e, action(a, ActionKind::Hunt));
        }
        request(&mut e);
        let stealth = e.action().expect("the hunt").stealth(&e.state);
        assert_eq!(
            stealth,
            if supply && !abactor { 0 } else { 1 },
            "supply {supply}, abactor {abactor}"
        );
    }
}

/// Strained Vitae Supply: each Methuselah's master phase action puts 1 of
/// their pool on it; burned once its counters reach the Methuselahs in the
/// game, read live: an oust leaving 2 burns it at 2.
#[test]
fn strained_vitae_supply_burns_at_a_counter_per_methuselah() {
    let mut e = game(3);
    for s in 0..3 {
        e.add_minion(SeatId(s), plain("Brujah", s as usize), 4, false);
    }
    let sv = in_play(&mut e, 0, "Strained Vitae Supply", None);
    let req = until(&mut e, SeatId(1), WindowLabel::MasterPhase);
    e.choose(index_of(&req, ability(sv.0, 0)));
    let req = until(&mut e, SeatId(2), WindowLabel::MasterPhase);
    e.choose(index_of(&req, ability(sv.0, 0)));
    request(&mut e);
    assert_eq!(e.state.cards[sv.index()].counters, 2);
    assert_eq!((e.state.seats[1].pool, e.state.seats[2].pool), (9, 9));
    assert!(e.state.cards[sv.index()].in_play, "2 of 3");
    e.state.seats[2].pool = 0;
    e.state.seats[2].ousted = true;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[sv.index()].in_play, "2 of 2");
    assert!(ash_has(&e, 0, "Strained Vitae Supply"));
}

/// Talons of the Dead: on its player's Gangrel antitribu (not a Gangrel, not
/// another's), the card itself; a directed action costs him 1 blood more,
/// an undirected hunt none; his hand strikes aggravated.
#[test]
fn talons_of_the_dead_aggravates_hands_and_taxes_directed_actions() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), of_clan("Gangrel antitribu", 4..=11, 0), 4, false);
    let gangrel = e.add_minion(SeatId(0), plain("Gangrel", 0), 4, false);
    let theirs = e.add_minion(SeatId(1), of_clan("Gangrel antitribu", 1..=11, 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[theirs.index()].locked = true;
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 0, &["Talons of the Dead"]);
    e.at_master_phase();
    let on = |m| Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(m),
    };
    let req = request(&mut e);
    assert!(!req.options.contains(&on(gangrel)));
    assert!(!req.options.contains(&on(theirs)));
    e.choose(index_of(&req, on(g)));
    request(&mut e);
    let t = placed_on(&e, "Talons of the Dead", Some(g));
    assert!(e
        .state
        .borne(g)
        .any(|l| l.modifier == zvtes::cards::Modifier::BearerAggravatedHands && l.source == t));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.choose(index_of(&req, bleed(g)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, g), 3, "the toll");
    assert_eq!(e.state.seats[1].pool, 9);
}

/// Talons of the Dead: its vampire's controller burns it with a master phase
/// action.
#[test]
fn talons_of_the_dead_burns_for_a_master_phase_action() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), of_clan("Gangrel antitribu", 1..=11, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let t = in_play(&mut e, 0, "Talons of the Dead", Some(g));
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let burn = use_on(t, 0, Some(g), Target::None);
    let burn = if req.options.contains(&burn) {
        burn
    } else {
        use_on(t, 0, None, Target::None)
    };
    e.choose(index_of(&req, burn));
    request(&mut e);
    assert!(!e.state.cards[t.index()].in_play);
    assert!(ash_has(&e, 0, "Talons of the Dead"));
    assert_eq!(e.state.seats[0].mpa, 0, "its master phase action");
}

/// Toy Chest Test: a capacity 1–3 Sabbat vampire put in play since the end
/// of the player's last turn is burned: another's influenced in the
/// meantime is a target, one in play since that turn is not, nor Toy
/// ("Toy Chest Test cannot be played on him").
/// [LSJ 20020429] "Since your last turn" means between the end of your
/// last turn and now. Your last turn does not qualify, your current turn
/// does.
#[test]
fn toy_chest_test_burns_a_young_sabbat_vampire_put_in_play_since_last_turn() {
    let mut e = game(3);
    e.state.seats[0].this_turn = 4;
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 5..=11, 0), 4, false);
    let old = e.add_minion(SeatId(2), of_sect(Sect::Sabbat, 1..=3, 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Toy Chest Test"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let young = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 1..=3, 1), 1, false);
    let toy = e.add_minion(SeatId(1), find_card("Toy"), 2, false);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let on = |m| Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(m),
    };
    assert!(!req.options.contains(&on(old)), "in play since that turn");
    assert!(!req.options.contains(&on(toy)), "Toy: not on him");
    e.choose(index_of(&req, on(young)));
    request(&mut e);
    assert!(!e.state.minions[young.index()].in_play, "burned");
    assert!(ash_has(&e, 0, "Toy Chest Test"));

    // This turn qualifies.
    let mut e = game(2);
    e.state.seats[0].this_turn = 4;
    e.state.seats[0].previous_turn = 2;
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 5..=11, 0), 4, false);
    let now = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 1..=3, 0), 1, false);
    let four = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=4, 0), 1, false);
    hand(&mut e, 0, &["Toy Chest Test"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(req.options.contains(&on(now)), "{:?}", req.options);
    assert!(!req.options.contains(&on(four)), "capacity below 4");
}

/// Toy Chest Test: a vampire whose only contest is its title "has been
/// contested" no (red reading Q2: the card's contest only): still a
/// target.
#[test]
fn toy_chest_test_ignores_a_title_contest() {
    let mut e = game(2);
    e.state.seats[0].this_turn = 4;
    e.state.seats[0].previous_turn = 2;
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 5..=11, 0), 4, false);
    let t = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 1..=3, 0), 1, false);
    e.state.minions[t.index()].title_contested = true;
    hand(&mut e, 0, &["Toy Chest Test"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(t),
    }));
}

/// Toy Chest Test "requires a ready Sabbat vampire with a capacity above
/// 4": a capacity 4 one is not enough.
#[test]
fn toy_chest_test_needs_a_big_sabbat_vampire() {
    let mut e = game(2);
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=4, 0), 4, false);
    e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 1..=3, 0), 1, false);
    hand(&mut e, 0, &["Toy Chest Test"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { .. })
    });
}

/// Toy Chest Test "(not usable on a vampire that has been contested)": a
/// vampire contested since it entered play, its rival yielded, is spared
/// (rules-qa 122, red: contested since it last entered play).
#[test]
fn toy_chest_test_spares_a_contested_vampire() {
    let v = of_sect(Sect::Sabbat, 1..=3, 0);
    let mut e = game(3);
    e.state.seats[0].this_turn = 4;
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 5..=11, 0), 4, false);
    let rival = e.add_minion(SeatId(2), v, 3, false);
    e.add_minion(SeatId(2), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1].uncontrolled = vec![zvtes::state::UncontrolledSlot {
        card: v,
        copy: Default::default(),
        counters: 11,
        banished: None,
    }];
    hand(&mut e, 0, &["Toy Chest Test"]);
    to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    request(&mut e);
    let clean = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 1..=3, 1), 1, false);
    let m = MinionId(
        e.state
            .minions
            .iter()
            .position(|o| o.owner == SeatId(1) && o.crypt_card() == Some(v))
            .expect("influenced") as u32,
    );
    assert!(e.state.minions[m.index()].contested);
    to_window(&mut e, SeatId(2), WindowLabel::UnlockAbilities);
    pick(
        &mut e,
        Choice::YieldContest(zvtes::game::Contest::Card(rival)),
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[m.index()].contested, "uncontested now");
    // Seat 0's last turn taken as before both entries (turn 5).
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    e.state.seats[0].this_turn = 1;
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let on = |t| Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(t),
    };
    assert!(!req.options.contains(&on(m)), "{:?}", req.options);
    assert!(req.options.contains(&on(clean)), "{:?}", req.options);
}

/// Using the Advantage: in its controller's unlock phase, with the Edge,
/// locked for 1 pool; without the Edge not offered.
#[test]
fn using_the_advantage_locks_for_a_pool_with_the_edge() {
    for edge in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.seats[0].has_edge = edge;
        let u = in_play(&mut e, 0, "Using the Advantage", None);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let used = offered_before(&mut e, ability(u.0, 0), SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(used, edge);
        if edge {
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.seats[0].pool, 11);
            assert!(e.state.cards[u.index()].locked);
        }
    }
}

/// Ventrue Investment: a blood per Ventrue antitribu its player controls,
/// one in torpor too; an investment (Swiss Cut grows); a master phase
/// action moves each to the pool, the last burning it. Not played without
/// a ready Ventrue antitribu (its clan requirement), a torpid one only.
#[test]
fn ventrue_investment_holds_a_blood_per_ventrue_antitribu() {
    let mut e = game(2);
    e.add_minion(SeatId(0), of_clan("Ventrue antitribu", 1..=11, 0), 4, false);
    e.add_minion(SeatId(0), of_clan("Ventrue antitribu", 1..=11, 1), 0, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let swiss = in_play(&mut e, 1, "Swiss Cut", None);
    hand(&mut e, 0, &["Ventrue Investment"]);
    e.at_master_phase();
    e.state.seats[0].mpa = 3;
    pick(&mut e, master_play(0));
    request(&mut e);
    let vi = placed_on(&e, "Ventrue Investment", None);
    assert_eq!(e.state.cards[vi.index()].counters, 2);
    assert_eq!(e.state.cards[swiss.index()].counters, 1, "an investment");
    pick(&mut e, ability(vi.0, 0));
    request(&mut e);
    pick(&mut e, ability(vi.0, 0));
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 12);
    assert!(!e.state.cards[vi.index()].in_play);
    assert!(ash_has(&e, 0, "Ventrue Investment"));

    // Its clan requirement: a ready Ventrue antitribu, so never 0 blood.
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), of_clan("Ventrue antitribu", 1..=11, 0), 0, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Ventrue Investment"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { .. })
    });
}

/// Seat 0's `a` calls Ancilla Empowerment's referendum.
fn call_referendum(e: &mut Engine, a: MinionId) {
    hand(e, 0, &["Ancilla Empowerment"]);
    pick(e, play(0, 0, a));
    to_referendum(e);
}

/// Vox Senis in play: every Methuselah, one with no titled vampire too,
/// casts 1 vote of their own on either side, once a referendum.
#[test]
fn vox_senis_gives_every_methuselah_a_vote() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    let vox = in_play(&mut e, 0, "Vox Senis", None);
    call_referendum(&mut e, a);
    let mut used = Vec::new();
    loop {
        let req = request(&mut e);
        if !matches!(req.window, WindowLabel::ReferendumPolling) {
            break;
        }
        let vox_uses: Vec<usize> = (0..req.options.len())
            .filter(|&i| {
                matches!(req.options[i], Choice::UseAbility { source: AbilitySource::Card(c), .. }
                    if c == vox)
            })
            .collect();
        if used.contains(&req.seat) {
            assert!(vox_uses.is_empty(), "once: {:?}", req.options);
            e.choose(fallback(&req));
        } else {
            assert_eq!(vox_uses.len(), 2, "for or against: {:?}", req.options);
            used.push(req.seat);
            let favor = req.seat != SeatId(2);
            e.choose(index_of(
                &req,
                use_on(vox, u8::from(!favor), None, Target::None),
            ));
        }
    }
    used.sort();
    assert_eq!(used, vec![SeatId(0), SeatId(1), SeatId(2)]);
}

/// Vox Senis played out of turn in another's referendum, from hand to the
/// ash heap: 3 votes on its player's side, its next master phase action
/// spent; offered while its player's own Vox Senis is in play (it enters
/// no play); never on its player's own turn.
#[test]
fn vox_senis_out_of_turn_gains_three_votes() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    in_play(&mut e, 1, "Vox Senis", None);
    hand(&mut e, 1, &["Vox Senis"]);
    hand(&mut e, 0, &["Vox Senis"]);
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    pick(&mut e, play(1, 0, a));
    to_referendum(&mut e);
    let three = Choice::Play {
        card: 0,
        mode: 2,
        actor: None,
        target: Target::None,
    };
    let mut played = false;
    for _ in 0..50 {
        let req = request(&mut e);
        if req.seat == SeatId(0) {
            assert!(
                !req.options
                    .iter()
                    .any(|o| matches!(o, Choice::Play { mode: 1 | 2, .. })),
                "not on its own turn: {:?}",
                req.options
            );
        }
        if req.seat == SeatId(1) && req.options.contains(&three) {
            e.choose(index_of(&req, three));
            played = true;
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(played, "offered beside its own Vox Senis in play");
    request(&mut e);
    let rf = e.referendum().expect("referendum");
    assert!(rf
        .records
        .iter()
        .any(|r| r.source == zvtes::referendum::Source::Card(SeatId(1))
            && !r.favor
            && r.n == 3
            && !r.canceled));
    assert!(ash_has(&e, 1, "Vox Senis"));
    assert!(e.state.seats[1].hand.is_empty());
    assert!(
        e.state.seats[1].out_of_turn.is_some(),
        "its next master phase action"
    );
}

/// Vox Senis out of turn can be canceled as played (Sudden Reversal): no
/// votes, the card in its player's ash heap.
/// [LSJ 20031201] "May still be 'canceled as it is played' when used as an
/// out-of-turn master."
#[test]
fn vox_senis_out_of_turn_can_be_canceled_as_played() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    hand(&mut e, 1, &["Vox Senis"]);
    hand(&mut e, 2, &["Sudden Reversal"]);
    call_referendum(&mut e, a);
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 1,
            actor: None,
            target: Target::None,
        },
    );
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::CardAsPlayed);
    pick_when(&mut e, master_play(0));
    request(&mut e);
    let rf = e.referendum().expect("referendum");
    assert!(!rf
        .records
        .iter()
        .any(|r| r.source == zvtes::referendum::Source::Card(SeatId(1)) && !r.canceled));
    assert!(ash_has(&e, 1, "Vox Senis"));
    assert!(ash_has(&e, 2, "Sudden Reversal"));
}

/// Waste Management Operation: in its controller's discard phase, locked to
/// move a library card from the ash heap to the library's bottom; a crypt
/// card in the ash heap is not offered.
/// [LSJ 20041201] "Crypt cards cannot be put in the library and library
/// cards cannot be put in the crypt."
#[test]
fn waste_management_operation_recycles_an_ash_card_to_the_library_bottom() {
    let (dodge, soak) = (find_library("Dodge"), find_library("Soak"));
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let burned = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.burn(burned);
    e.state.seats[0].ash.put(dodge, Default::default());
    e.state.seats[0].ash.put(soak, Default::default());
    e.state.seats[0].library = [soak, soak].iter().copied().collect();
    fill_hand(&mut e, 0);
    let w = in_play(&mut e, 0, "Waste Management Operation", None);
    let _ = a;
    pick_when(&mut e, ability(w.0, 0));
    let req = request(&mut e);
    let finds: Vec<Choice> = req
        .options
        .iter()
        .copied()
        .filter(|o| matches!(o, Choice::FindCard(_) | Choice::AshVampire(_)))
        .collect();
    assert_eq!(
        finds,
        vec![Choice::FindCard(soak), Choice::FindCard(dodge)],
        "{:?}",
        req.options
    );
    e.choose(index_of(&req, Choice::FindCard(dodge)));
    request(&mut e);
    assert_eq!(e.state.seats[0].library[0], dodge, "the bottom");
    assert!(!e.state.seats[0].ash.contains(&dodge));
    assert!(e.state.cards[w.index()].locked);
}

/// Waste Management Operation into an empty library, the hand one short:
/// the card is drawn at once.
/// [ANK 20211201] "If the library is empty and the hand not full, you draw
/// immediately after using it."
#[test]
fn waste_management_operation_draws_at_once_into_an_empty_library() {
    let dodge = find_library("Dodge");
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].ash.put(dodge, Default::default());
    hand(&mut e, 0, &["Soak", "Soak", "Soak", "Soak", "Soak", "Soak"]);
    let w = in_play(&mut e, 0, "Waste Management Operation", None);
    pick_when(&mut e, ability(w.0, 0));
    request(&mut e);
    assert!(e.state.seats[0].library.is_empty());
    assert_eq!(e.state.seats[0].hand.len(), 7);
    assert!(e.state.seats[0].hand.contains(&dodge));
}

// ---- Action cards (action_a) ------------------------------------------------

/// A textless Sabbat vampire with a capacity in `caps`, made a priscus.
fn sabbat_priscus(
    e: &mut Engine,
    seat: u8,
    caps: std::ops::RangeInclusive<u8>,
    nth: usize,
    blood: u8,
) -> MinionId {
    let m = e.add_minion(SeatId(seat), of_sect(Sect::Sabbat, caps, nth), blood, false);
    e.state.minions[m.index()].granted_title = Some(Title::Priscus);
    m
}

/// Blood Feast: each ready Sabbat vampire its player controls gains 1 blood
/// as it resolves: the acting priscus and a locked one ("ready" is not
/// "unlocked", rules-digest D28), not a torpid one, a Camarilla one or
/// another Methuselah's; an untitled Sabbat vampire is not offered it.
#[test]
fn blood_feast_feeds_each_ready_sabbat_vampire() {
    let mut e = game(2);
    let a = sabbat_priscus(&mut e, 0, 5..=11, 0, 2);
    let untitled = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 1), 1, false);
    let locked = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 2), 1, false);
    e.state.minions[locked.index()].locked = true;
    let torpid = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 3), 1, true);
    let cam = e.add_minion(SeatId(0), plain("Ventrue", 0), 1, false);
    let other = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=11, 4), 1, false);
    hand(&mut e, 0, &["Blood Feast"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play(0, 0, untitled)),
        "a titled actor"
    );
    e.choose(index_of(&req, play(0, 0, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Blood Feast"));
    assert_eq!(blood(&e, a), 2, "1 paid, 1 gained");
    assert_eq!(blood(&e, untitled), 2);
    assert_eq!(blood(&e, locked), 2);
    assert_eq!(blood(&e, torpid), 1);
    assert_eq!(blood(&e, cam), 1);
    assert_eq!(blood(&e, other), 1);
}

/// Media Influence [pre]: a bleed at +2; [PRE]: each unlocked vampire of
/// its player gains 1 blood, a torpid unlocked one too (D28), not the
/// acting one (locked by acting), a locked one or another Methuselah's.
#[test]
fn media_influence_bleeds_or_feeds_the_unlocked() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pre", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Media Influence"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 3, "+2 bleed");

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("PRE", 0), 4, false);
    let free = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    let locked = e.add_minion(SeatId(0), plain("Brujah", 1), 1, false);
    e.state.minions[locked.index()].locked = true;
    let torpid = e.add_minion(SeatId(0), plain("Brujah", 2), 1, true);
    let other = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    hand(&mut e, 0, &["Media Influence"]);
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Media Influence"));
    assert_eq!(blood(&e, a), 3, "1 paid, none gained");
    assert_eq!(blood(&e, free), 2);
    assert_eq!(blood(&e, locked), 1);
    assert_eq!(blood(&e, torpid), 2);
    assert_eq!(blood(&e, other), 1);
}

/// Bloodbath on an untitled capacity-5 Sabbat vampire: 2 blood, the card
/// itself on it, +1 vote; titled, no vote; untitled again, the vote back.
/// A capacity-4 Sabbat vampire is no target.
/// [LSJ 19970224] No extra vote while it has a title, back once it loses it.
#[test]
fn bloodbath_feeds_and_votes_while_untitled() {
    let mut e = game(2);
    let a = sabbat_priscus(&mut e, 0, 5..=11, 0, 4);
    let t = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 5..=11, 1), 1, false);
    let small = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 1..=4, 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Bloodbath"]);
    let req = request(&mut e);
    assert!(
        !req.options
            .contains(&play_on(0, 0, a, Target::Minion(small))),
        "capacity above 4"
    );
    e.choose(index_of(&req, play_on(0, 0, a, Target::Minion(t))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Bloodbath", Some(t));
    assert_eq!(blood(&e, t), 3);
    assert_eq!(e.state.votes_held(t), 1);
    e.state.minions[t.index()].granted_title = Some(Title::Bishop);
    assert_eq!(e.state.votes_held(t), 1, "the bishop's vote only");
    e.state.minions[t.index()].granted_title = None;
    assert_eq!(e.state.votes_held(t), 1, "untitled again");
    e.state.minions[t.index()].granted_title = Some(Title::Cardinal);
    assert_eq!(e.state.votes_held(t), 3, "the cardinal's 3 only");
}

/// The Call: a younger vampire with Dementation in the player's
/// uncontrolled region gets 2 blood ([AUS][DEM]: 3); an older one with it
/// and a younger one without are no targets.
#[test]
fn the_call_feeds_a_younger_dementation_vampire_uncontrolled() {
    for (mode, gained) in [(0u8, 2u8), (1, 3)] {
        let mut e = game(2);
        let card = CRYPT
            .iter()
            .enumerate()
            .find(|(i, c)| {
                ["AUS", "DEM"].iter().all(|d| c.disciplines.contains(d))
                    && c.capacity >= 6
                    && zvtes::cards::vampire(*i as u16).is_none()
            })
            .map(|(i, _)| i as u16)
            .expect("an [aus][dem] vampire");
        let a = e.add_minion(SeatId(0), card, 4, false);
        let cap = CRYPT[card as usize].capacity;
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let with_dem = |young: bool, dem: bool| {
            CRYPT
                .iter()
                .position(|c| {
                    (c.capacity < cap) == young
                        && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("dem")) == dem
                })
                .expect("a vampire") as u16
        };
        let slot = |card| zvtes::state::UncontrolledSlot {
            card,
            copy: Default::default(),
            counters: 0,
            banished: None,
        };
        e.state.seats[0].uncontrolled = vec![
            slot(with_dem(true, true)),
            slot(with_dem(false, true)),
            slot(with_dem(true, false)),
        ];
        hand(&mut e, 0, &["Call, The"]);
        let req = request(&mut e);
        for i in [1, 2] {
            assert!(
                !req.options
                    .contains(&play_on(0, mode, a, Target::Uncontrolled(i))),
                "slot {i}"
            );
        }
        e.choose(index_of(&req, play_on(0, mode, a, Target::Uncontrolled(0))));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[0].uncontrolled[0].counters, gained);
        assert!(ash_has(&e, 0, "Call, The"));
    }
}

/// Cull the Herd: the other Methuselah's hand seen, its allies and
/// retainers discarded and replaced, the actor gaining 1 blood each
/// (capacity-capped); a master card kept.
#[test]
fn cull_the_herd_discards_allies_and_retainers_for_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=4, 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Cull the Herd"]);
    hand(
        &mut e,
        1,
        &[
            "Mr. Winthrop",
            "Carlton Van Wyk",
            "Ghoul Retainer",
            "Blood Doll",
        ],
    );
    fill_hand(&mut e, 1);
    e.state.seats[1].library = vec![find_library("Conceal"); 5].into();
    pick(&mut e, play_on(0, 0, a, Target::Seat(SeatId(1))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    for n in ["Mr. Winthrop", "Carlton Van Wyk", "Ghoul Retainer"] {
        assert!(ash_has(&e, 1, n), "{n}");
    }
    assert!(!ash_has(&e, 1, "Blood Doll"));
    let h = &e.state.seats[1].hand;
    assert_eq!(h.len(), 7, "replaced");
    assert_eq!(
        h.iter().filter(|&&c| c == find_library("Conceal")).count(),
        3
    );
    assert_eq!(blood(&e, a), 4, "1 paid, 3 gained up to capacity 4");
}

/// Martial Ritus: the card itself on a younger Sabbat vampire (+1
/// strength), the acting vampire unlocked; burned in its controller's next
/// unlock phase. An older Sabbat vampire and a younger Camarilla one are no
/// targets.
#[test]
fn martial_ritus_unlocks_its_actor_and_arms_a_younger_sabbat_until_unlock() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 8..=8, 0), 4, false);
    let young = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 1..=5, 0), 2, false);
    let old = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 9..=11, 1), 2, false);
    let cam = e.add_minion(SeatId(0), of_clan("Ventrue", 1..=5, 0), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cap = |m: MinionId| e.state.capacity_of(m).unwrap();
    assert!(cap(old) >= cap(a));
    hand(&mut e, 0, &["Martial Ritus"]);
    let req = request(&mut e);
    for t in [old, cam] {
        assert!(!req.options.contains(&play_on(0, 0, a, Target::Minion(t))));
    }
    let strength = e.state.bearer_mod(young, Stat::Strength, ActionKind::Bleed);
    e.choose(index_of(&req, play_on(0, 0, a, Target::Minion(young))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let id = placed_on(&e, "Martial Ritus", Some(young));
    assert!(!e.state.minions[a.index()].locked, "unlocked");
    assert_eq!(
        e.state.bearer_mod(young, Stat::Strength, ActionKind::Bleed),
        strength + 1
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        e.state.cards[id.index()].in_play,
        "the other's unlock phase"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[id.index()].in_play);
    assert!(ash_has(&e, 0, "Martial Ritus"));
}

/// Martial Ritus on another Methuselah's younger Sabbat vampire: directed
/// (seat 1 asked to block); the card is then seat 1's (7.5) and "your next
/// unlock phase" its controller's (red reading; 32.7, advanced rules
/// §6.2.5): it burns at seat 1's next unlock phase, before seat 0's.
#[test]
fn martial_ritus_on_another_methuselahs_vampire_burns_at_its_controllers_unlock() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 8..=8, 0), 4, false);
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 8..=8, 1), 4, false);
    let young = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 1..=5, 0), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Martial Ritus"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(young)));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::ActionBlock)
    );
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let id = placed_on(&e, "Martial Ritus", Some(young));
    assert_eq!(e.state.card_controller(id), SeatId(1));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[id.index()].in_play, "seat 1's unlock phase");
    assert!(ash_has(&e, 0, "Martial Ritus"), "its owner's ash heap");
}

/// Blood of the Sabbat: the card itself on another Sabbat vampire, which
/// gains 1 blood after its successful directed action (a bleed), none
/// after an undirected one (a hunt); a vampire with one is no target for
/// another ("only one"): a second priscus may put the second copy on A,
/// not on T.
#[test]
fn blood_of_the_sabbat_feeds_its_vampire_after_a_directed_success() {
    let mut e = game(2);
    let a = sabbat_priscus(&mut e, 0, 5..=11, 0, 4);
    let t = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 5..=11, 1), 2, false);
    let u = sabbat_priscus(&mut e, 0, 5..=11, 2, 4);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Blood of the Sabbat", "Blood of the Sabbat"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play_on(0, 0, a, Target::Minion(a))),
        "other than the acting vampire"
    );
    e.choose(index_of(&req, play_on(0, 0, a, Target::Minion(t))));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Blood of the Sabbat", Some(t));
    assert!(req.options.contains(&play_on(0, 0, u, Target::Minion(a))));
    assert!(
        !req.options.contains(&play_on(0, 0, u, Target::Minion(t))),
        "only one"
    );
    e.choose(index_of(&req, bleed(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9);
    assert_eq!(blood(&e, t), 3, "a directed success");
    e.state.minions[t.index()].locked = false;
    e.state.minions[t.index()].acted.clear();
    pick(&mut e, action(t, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, t), 4, "the hunt's only");
}

/// Hidden Pathways [obf]: the card itself in play; a second copy not
/// offered; another Methuselah's minion burns it as an action directed at
/// its controller.
#[test]
fn hidden_pathways_one_per_methuselah_and_burned_by_an_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    let b = e.add_minion(SeatId(0), vampire_with("obf", 1), 4, false);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Hidden Pathways", "Hidden Pathways"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, b)));
    e.choose(index_of(&req, play(0, 0, a)));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let hp = placed_on(&e, "Hidden Pathways", None);
    assert!(
        !req.options.contains(&play(0, 0, b)) && !req.options.contains(&play(0, 1, b)),
        "one per Methuselah: {:?}",
        req.options
    );
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let burn = use_on(hp, 0, Some(p), Target::None);
    e.choose(index_of(&req, burn));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::ActionBlock),
        "directed"
    );
    assert_eq!(e.action().expect("action").stealth(&e.state), 0);
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[hp.index()].in_play);
    assert!(ash_has(&e, 0, "Hidden Pathways"));
}

/// Hidden Pathways in play: +1 stealth on the player's minions' non-bleed
/// actions directed at the prey (a rush at the prey's minion), not on a
/// bleed of the prey, nor on a rush at the predator's minion.
#[test]
fn hidden_pathways_gives_stealth_against_the_prey_but_not_on_bleeds() {
    // (what the acting minion does, the stealth expected)
    for case in 0..3 {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let prey = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let pred = e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
        in_play(&mut e, 0, "Hidden Pathways", None);
        hand(&mut e, 0, &["Bum's Rush"]);
        let (choice, want) = match case {
            0 => (play_on(0, 0, a, Target::Minion(prey)), 1),
            1 => (bleed(a), 0),
            _ => (play_on(0, 0, a, Target::Minion(pred)), 0),
        };
        pick(&mut e, choice);
        request(&mut e);
        assert_eq!(
            e.action().expect("action").stealth(&e.state),
            want,
            "case {case}"
        );
    }
}

/// Esbat: 2 blood to another Methuselah's unlocked Sabbat vampire (the
/// action directed at them: they are asked to block), or 1 to each of two
/// (the second picked as played); a locked one is no target.
/// [ANK 20210124] [LSJ 20070411] [LSJ 20090514] The group row, with the
/// next test.
#[test]
fn esbat_moves_two_blood_or_one_to_each_of_two() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 0), 4, false);
    let a2 = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 1), 4, false);
    let own = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 5..=11, 2), 1, false);
    let t = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 5..=11, 3), 1, false);
    let locked = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 5..=11, 4), 1, false);
    e.state.minions[locked.index()].locked = true;
    let t2 = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 5..=11, 5), 1, false);
    hand(&mut e, 0, &["Esbat", "Esbat"]);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&play_on(0, 0, a, Target::Minion(locked))));
    e.choose(index_of(&req, play_on(0, 0, a, Target::Minion(t))));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::ActionBlock)
    );
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, t), 3);
    pick(&mut e, play_on(0, 1, a2, Target::Minion(t)));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::PlayTerms);
    assert!(!req.options.contains(&Choice::Pass), "two, not one");
    assert!(!req.options.contains(&Choice::ChooseMinion(locked)));
    assert!(req.options.contains(&Choice::ChooseMinion(t2)));
    e.choose(index_of(&req, Choice::ChooseMinion(own)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, t), 4);
    assert_eq!(blood(&e, own), 2);
    assert!(ash_has(&e, 0, "Esbat"));
}

/// Esbat on a vampire locked before it resolves: the action fizzles,
/// successful, no blood.
/// [ANK 20210124] [LSJ 20070411] [LSJ 20090514] "If the action is unblocked
/// when it resolves and the target is locked, the action fizzles."
#[test]
fn esbat_fizzles_on_a_target_locked_by_resolution() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 0), 4, false);
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 1), 4, false);
    let t = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 5..=11, 3), 1, false);
    hand(&mut e, 0, &["Esbat"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(t)));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::ActionBlock);
    e.state.minions[t.index()].locked = true;
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, t), 1, "no effect");
    assert!(ash_has(&e, 0, "Esbat"));
    assert!(e.state.turn_successes.contains(&a), "successful");
}

/// Bum's Rush by `a` on `b` (seat 1), the block declined: the combat.
fn rush(e: &mut Engine, a: MinionId, b: MinionId, card: u8) {
    pick(e, play_on(card, 0, a, Target::Minion(b)));
    let req = request(e);
    if req.window == WindowLabel::ActionBlock {
        e.choose(index_of(&req, Choice::Pass));
    }
}

/// Ablative Skin [for]: the card itself on the actor with 3 counters; in a
/// combat, a counter removed prevents 1 non-aggravated damage; the last one
/// gone, the card burns to the ash heap.
#[test]
fn ablative_skin_prevents_with_its_counters_and_burns_empty() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("for", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Ablative Skin", "Bum's Rush"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let id = placed_on(&e, "Ablative Skin", Some(a));
    assert_eq!(e.state.cards[id.index()].counters, 3);
    e.state.cards[id.index()].counters = 1;
    e.state.minions[a.index()].locked = false;
    e.state.minions[a.index()].acted.clear();
    rush(&mut e, a, b, 0);
    pick_use_of(&mut e, id, 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4, "the hand strike prevented");
    assert!(!e.state.cards[id.index()].in_play);
    assert!(ash_has(&e, 0, "Ablative Skin"));
}

/// Ablative Skin [for] cannot prevent aggravated damage (hand strikes made
/// aggravated), even treated as normal (Skin of Night played); [FOR] can.
/// [LSJ 20040812-2] [PIB 20130327] The group row "Prevent non-aggravated".
#[test]
fn ablative_skin_basic_cannot_prevent_aggravated() {
    for mode in [0u8, 1] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("FOR", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        e.state.minions[b.index()].timed.push((
            zvtes::cards::Lasting::AggravatedHands,
            zvtes::state::Expiry::EndOfTurn,
            SeatId(1),
        ));
        let id = in_play_mode(&mut e, 0, "Ablative Skin", Some(a), mode);
        e.state.cards[id.index()].counters = 3;
        hand(&mut e, 0, &["Bum's Rush", "Skin of Night"]);
        rush(&mut e, a, b, 0);
        if mode == 0 {
            // Its aggravated damage treated as normal: still aggravated for
            // Ablative Skin.
            pick_when(&mut e, play(0, 0, a));
            never_offered_where(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == id),
            );
            assert_eq!(e.state.cards[id.index()].counters, 3);
        } else {
            pick_use_of(&mut e, id, 1);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.cards[id.index()].counters, 2, "prevented");
        }
    }
}

/// Ablative Skin against Blood Rage's strike: not offered; the damage lands.
/// [LSJ 19990216] "Cannot be used to prevent damage that cannot be prevented
/// by cards that require Fortitude."
#[test]
fn ablative_skin_cannot_prevent_a_fortitude_proof_strike() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("for", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("tha", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let id = in_play(&mut e, 0, "Ablative Skin", Some(a));
    e.state.cards[id.index()].counters = 3;
    hand(&mut e, 0, &["Bum's Rush"]);
    hand(&mut e, 1, &["Blood Rage"]);
    rush(&mut e, a, b, 0);
    pick_when(&mut e, play(0, 0, b));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == id),
    );
    assert_eq!(e.state.cards[id.index()].counters, 3);
    assert!(blood(&e, a) < 4, "the damage landed");
}

/// Blessing of Chaos: the card itself on the actor; an opposing vampire in
/// its combat cannot play Majesty [pre] (it can without the card).
#[test]
fn blessing_of_chaos_bars_the_opposing_vampires_chaos_cards_in_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("dem", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Blessing of Chaos"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Blessing of Chaos", Some(a));
    for blessed in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("dem", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let b = e.add_minion(SeatId(1), vampire_with("pre", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        if blessed {
            in_play(&mut e, 0, "Blessing of Chaos", Some(a));
        }
        hand(&mut e, 0, &["Bum's Rush"]);
        hand(&mut e, 1, &["Majesty"]);
        rush(&mut e, a, b, 0);
        let majesty =
            |o: &Choice| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == b);
        if blessed {
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, majesty);
        } else {
            let req = offering(&mut e, play(0, 0, b));
            assert!(req.options.iter().any(majesty));
        }
    }
}

/// Blessing of Chaos: its vampire attempts to block and fails (Cloak the
/// Gathering's stealth): the acting vampire's Conditioning [dom] is no
/// longer offered for the rest of the action; with no attempt, it is.
/// [LSJ 20080818] From the attempt to the action's end, even if it fails.
#[test]
fn blessing_of_chaos_bars_the_acting_modifiers_once_its_vampire_attempted() {
    let actor = textless(
        |c| {
            c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("dom"))
                && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("obf"))
        },
        0,
    );
    for attempt in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("dem", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let b = e.add_minion(SeatId(1), actor, 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        in_play(&mut e, 0, "Blessing of Chaos", Some(a));
        hand(&mut e, 1, &["Conditioning", "Cloak the Gathering"]);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        pick(&mut e, bleed(b));
        let conditioning =
            |o: &Choice| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == b);
        if attempt {
            let req = request(&mut e);
            assert!(req.options.iter().any(conditioning), "before the attempt");
            e.choose(index_of(&req, Choice::Pass));
            pick(&mut e, block(a));
            pick_when(&mut e, play(1, 0, b));
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, conditioning);
            assert_eq!(e.state.seats[0].pool, 9, "the attempt failed: bled");
        } else {
            let req = offering(&mut e, play(0, 0, b));
            assert!(req.options.iter().any(conditioning));
        }
    }
}

/// Blessing of Chaos [DEM]: its vampire immune to directed actions
/// requiring the four Disciplines; [dem] not.
#[test]
fn blessing_of_chaos_superior_shields_from_chaos_actions() {
    for mode in [0u8, 1] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("DEM", 0), 4, false);
        in_play_mode(&mut e, 0, "Blessing of Chaos", Some(a), mode);
        for d in ["dem", "chi", "DOM", "pre"] {
            assert_eq!(e.state.immune_to_directed(a, &[d]), mode == 1, "{d}");
        }
        assert!(!e.state.immune_to_directed(a, &["pot"]));
    }
}

/// Is `o` a use of ability `ability` of `source`?
fn uses_of(o: &Choice, source: CardId, ability: u8) -> bool {
    matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: a, .. }
        if *c == source && *a == ability)
}

/// Lunatic Eruption on another Methuselah's ready vampire (directed: they
/// are asked to block): the card itself on it; in its controller's minion
/// phase, its rush on a minion of their prey is the only action offered
/// (`Engine::dues` names it), before any other.
/// [LSJ 20001127] The mandatory attack before any non-mandatory action.
#[test]
fn lunatic_eruption_forces_a_rush_on_the_prey_first() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("dem", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Lunatic Eruption"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(x)));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::ActionBlock)
    );
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let le = placed_on(&e, "Lunatic Eruption", Some(x));
    assert_eq!(e.state.card_controller(le), SeatId(1));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&Choice::Pass), "{:?}", req.options);
    assert!(!req.options.contains(&bleed(x)));
    assert!(!req.options.contains(&bleed(z)), "first, before any other");
    assert!(
        e.dues(&req).contains(&zvtes::actions::Due::Action(x)),
        "{:?}",
        e.dues(&req)
    );
    let rush = req
        .options
        .iter()
        .copied()
        .find(|o| uses_of(o, le, 0))
        .expect("the rush");
    assert!(
        matches!(rush, Choice::UseAbility { target: Target::Minion(m), .. } if e.state.minions[m.index()].owner == SeatId(0))
    );
    e.choose(index_of(&req, rush));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(req.options.contains(&bleed(z)), "free once taken");
}

/// Lunatic Eruption with no ready minion of the prey to fight: its vampire
/// is stuck, no action offered to it (red reading, advanced rules §3.9
/// "with one it cannot take, is stuck"; rules-qa 28.8).
#[test]
fn lunatic_eruption_with_no_ready_prey_minion_leaves_its_vampire_stuck() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("dem", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Lunatic Eruption"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(x)));
    pick_when(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Lunatic Eruption", Some(x));
    for m in [a, b] {
        e.state.minions[m.index()].in_torpor = true;
    }
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::TakeAction { minion, .. } | Choice::UseAbility { actor: Some(minion), .. } if *minion == x),
    );
}

/// Lunatic Eruption's vampire, its rush taken and unlocked, may act again:
/// a bleed offered, the rush not forced again.
/// [LSJ 20090226] [ANK 20200227] Entered combat and unlocked: free.
#[test]
fn lunatic_eruption_is_free_after_its_rush_and_an_unlock() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let le = in_play(&mut e, 0, "Lunatic Eruption", Some(x));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let rush = req
        .options
        .iter()
        .copied()
        .find(|o| uses_of(o, le, 0))
        .expect("rush");
    e.choose(index_of(&req, rush));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[x.index()].locked = false;
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = if req.options.contains(&bleed(x)) {
        req
    } else {
        // The unlock is read by the next window built.
        e.choose(index_of(&req, Choice::Pass));
        request(&mut e)
    };
    assert!(req.options.contains(&bleed(x)), "{:?}", req.options);
    assert!(
        !req.options.iter().any(|o| uses_of(o, le, 0)),
        "not again: once taken (an action of this copy)"
    );
}

/// Two Lunatic Eruptions on one minion: stuck, no action at all (its rushes,
/// its bleed); its Methuselah's other minion acts.
/// [LSJ 20041103] "If a minion has two of those, he is stuck and cannot
/// act."
#[test]
fn lunatic_eruption_two_copies_leave_the_minion_stuck() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 0, "Lunatic Eruption", Some(x));
    in_play(&mut e, 0, "Lunatic Eruption", Some(x));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(req.options.contains(&bleed(z)), "{:?}", req.options);
    assert!(
        !req.options.iter().any(|o| match *o {
            Choice::TakeAction { minion, .. } => minion == x,
            Choice::UseAbility { actor, .. } | Choice::Play { actor, .. } => actor == Some(x),
            _ => false,
        }),
        "{:?}",
        req.options
    );
}

/// Burning Lunatic Eruption: by a third Methuselah's minion, an action
/// directed at the bearer's controller; by that controller's own minion,
/// undirected.
/// [LSJ 20010328] Directed at the Methuselah controlling the minion it is
/// on, undirected for that Methuselah.
#[test]
fn lunatic_eruption_burn_is_directed_at_the_bearers_controller() {
    for own in [false, true] {
        let mut e = game(3);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let w = e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
        e.state.minions[x.index()].locked = true;
        let le = in_play(&mut e, 0, "Lunatic Eruption", Some(x));
        let (seat, by) = if own { (1, z) } else { (2, w) };
        let req = until(&mut e, SeatId(seat), WindowLabel::MinionPhase);
        let burn = req
            .options
            .iter()
            .copied()
            .find(|o| {
                uses_of(o, le, 1)
                    && matches!(o, Choice::UseAbility { actor: Some(m), .. } if *m == by)
            })
            .expect("the burn");
        e.choose(index_of(&req, burn));
        request(&mut e);
        let at = e.action().expect("action").directed_seat(&e.state);
        assert_eq!(at, (!own).then_some(SeatId(1)), "own {own}");
    }
}

/// Lunatic Eruption's vampire rushes, is unlocked, then burns the card as a
/// second action the same turn.
/// [LSJ 20100909] The two actions provided are not cross-restrictive.
#[test]
fn lunatic_eruption_burn_and_rush_are_two_actions() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let le = in_play(&mut e, 0, "Lunatic Eruption", Some(x));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let rush = req
        .options
        .iter()
        .copied()
        .find(|o| uses_of(o, le, 0))
        .expect("rush");
    e.choose(index_of(&req, rush));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[x.index()].locked = false;
    let burn = |o: &Choice| {
        uses_of(o, le, 1) && matches!(o, Choice::UseAbility { actor: Some(m), .. } if *m == x)
    };
    let req = offering_where(&mut e, burn);
    let i = req.options.iter().position(burn).expect("burn");
    e.choose(i);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[le.index()].in_play);
    assert!(ash_has(&e, 0, "Lunatic Eruption"));
}

/// Lunatic Eruption [DEM]: an ally burning it takes 1 unpreventable damage
/// ("the acting minion"), a life.
#[test]
fn lunatic_eruption_superior_burn_damages_the_acting_ally() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ally = e.add_ally(SeatId(0), find_library("Carlton Van Wyk"));
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[x.index()].locked = true;
    let le = in_play_mode(&mut e, 1, "Lunatic Eruption", Some(x), 1);
    let life = blood(&e, ally);
    let req = request(&mut e);
    let burn = req
        .options
        .iter()
        .copied()
        .find(|o| {
            uses_of(o, le, 2)
                && matches!(o, Choice::UseAbility { actor: Some(m), .. } if *m == ally)
        })
        .expect("the burn");
    e.choose(index_of(&req, burn));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[le.index()].in_play);
    assert_eq!(blood(&e, ally), life - 1);
}

/// Pass (or hand strike) until a request offers an option `want` matches.
fn offering_where(
    e: &mut Engine,
    want: impl Fn(&Choice) -> bool,
) -> zvtes::decision::DecisionRequest {
    for _ in 0..200 {
        let req = request(e);
        if req.options.iter().any(&want) {
            return req;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    panic!("never offered");
}

/// The first equipment card bearing (`with`) or lacking each of `labels`.
fn equipment_where(f: impl Fn(&[zvtes::library::Label]) -> bool) -> &'static str {
    LIBRARY
        .iter()
        .find(|c| c.types.contains(&CardType::Equipment) && f(c.labels))
        .map(|c| c.name)
        .expect("an equipment")
}

/// Bauble [vic]: the card itself on the acting vampire; a non-unique
/// non-location equipment searched from the library onto another of the
/// player's minions, its cost not paid (a .44 Magnum: no pool), a card
/// requiring a Discipline at its inferior version (Tinglestripe); a unique
/// equipment and a location equipment not offered; the vampire then offered
/// no action.
/// The group rows "Put card in play ignoring requirements": requirements do
/// not apply, X is 0 [LSJ 20100204] [LSJ 20040518-2] [LSJ 20100302-1];
/// inferior version [RBK equip].
#[test]
fn bauble_puts_a_found_equipment_on_another_minion_and_idles_its_vampire() {
    use zvtes::library::Label;
    let unique = equipment_where(|l| l.contains(&Label::Unique) && !l.contains(&Label::Location));
    let located = equipment_where(|l| l.contains(&Label::Location));
    for found in [".44 Magnum", "Tinglestripe"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("vic", 0), 4, false);
        let o = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Bauble"]);
        fill_hand(&mut e, 0);
        e.state.seats[0].library = [".44 Magnum", "Tinglestripe", unique, located]
            .iter()
            .map(|n| find_library(n))
            .collect::<Vec<_>>()
            .into();
        pick(&mut e, play_on(0, 0, a, Target::Minion(o)));
        let req = offering(&mut e, Choice::FindCard(find_library(found)));
        assert_eq!(req.window, WindowLabel::LibrarySearch);
        for n in [unique, located] {
            assert!(
                !req.options.contains(&Choice::FindCard(find_library(n))),
                "{n}"
            );
        }
        e.choose(index_of(&req, Choice::FindCard(find_library(found))));
        let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let bauble = placed_on(&e, "Bauble", Some(a));
        let eq = placed_on(&e, found, Some(o));
        assert_eq!(e.state.cards[bauble.index()].chosen_card.0, Some(eq));
        assert_eq!(e.state.cards[eq.index()].mode, 0, "inferior");
        assert_eq!(e.state.seats[0].pool, 10, "no cost paid");
        assert_eq!(blood(&e, a), 2, "Bauble's 2 blood");
        assert_eq!(e.state.seats[0].library.len(), 2, "one found, one drawn");
        e.state.minions[a.index()].locked = false;
        e.state.minions[a.index()].acted.clear();
        let _ = req;
        let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(
            !req.options
                .iter()
                .any(|o| matches!(o, Choice::TakeAction { minion, .. } if *minion == a)),
            "no actions: {:?}",
            req.options
        );
    }
}

/// Bauble [vic] and its .44 Magnum on another minion, in play.
fn bauble_fixture(e: &mut Engine, mode: u8) -> (MinionId, MinionId, CardId, CardId) {
    let a = e.add_minion(SeatId(0), vampire_with("VIC", 0), 4, false);
    let o = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let bauble = in_play_mode(e, 0, "Bauble", Some(a), mode);
    let gun = in_play(e, 0, ".44 Magnum", Some(o));
    e.state.cards[bauble.index()].chosen_card = zvtes::state::SparseOption(Some(gun));
    (a, o, bauble, gun)
}

/// Bauble [vic]: burned with its equipment during its player's unlock phase.
#[test]
fn bauble_burns_with_its_equipment_in_the_unlock_phase() {
    let mut e = game(2);
    let (_, o, bauble, gun) = bauble_fixture(&mut e, 0);
    let req = request(&mut e);
    assert!(
        !req.options
            .iter()
            .any(|c| uses_of(c, bauble, 0) || uses_of(c, bauble, 1)),
        "not now"
    );
    e.choose(index_of(&req, Choice::Pass));
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    e.choose(index_of(&req, use_on(bauble, 0, None, Target::None)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[bauble.index()].in_play && !e.state.cards[gun.index()].in_play);
    assert!(ash_has(&e, 0, "Bauble") && ash_has(&e, 0, ".44 Magnum"));
    assert!(e.state.minions[o.index()].in_play);
}

/// Bauble burned by another effect (its vampire burned): the equipment
/// stays.
/// [LSJ 20090113-1] "If it is burned by another mean than its effect, the
/// weapon is not burned."
#[test]
fn bauble_burned_otherwise_leaves_the_weapon() {
    let mut e = game(2);
    let (a, o, bauble, gun) = bauble_fixture(&mut e, 0);
    e.burn(a);
    assert!(!e.state.cards[bauble.index()].in_play);
    assert!(e.state.cards[gun.index()].in_play);
    assert_eq!(e.state.cards[gun.index()].host, Some(o));
}

/// Bauble with its equipment gone (its bearer burned) is not offered its
/// burn; a .44 Magnum back in play (another card in play) does not restore
/// it.
/// [RTR 20080808] Not burned if the equipment is not in play.
/// [LSJ 20040726] [ANK 20191031] Not if it was burned and came back.
#[test]
fn bauble_cannot_burn_without_its_equipment() {
    let mut e = game(2);
    let (_, o, bauble, _) = bauble_fixture(&mut e, 0);
    let o2 = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.burn(o);
    in_play(&mut e, 0, ".44 Magnum", Some(o2));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |c| {
        uses_of(c, bauble, 0)
    });
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |c| {
        uses_of(c, bauble, 0)
    });
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |c| {
        uses_of(c, bauble, 0)
    });
    assert!(e.state.cards[bauble.index()].in_play);
}

/// Bauble [VIC]: burned with its equipment at any time (its player's minion
/// phase).
#[test]
fn bauble_superior_burns_at_any_time() {
    let mut e = game(2);
    let (_, _, bauble, gun) = bauble_fixture(&mut e, 1);
    let req = request(&mut e);
    e.choose(index_of(&req, use_on(bauble, 2, None, Target::None)));
    request(&mut e);
    assert!(!e.state.cards[bauble.index()].in_play && !e.state.cards[gun.index()].in_play);
}

/// Bauble [vic], its equipment's bearer taken by another Methuselah: burned
/// with it at any later impulse while that bearer is theirs (read live).
/// Red reading (rules-qa batch 122): "after the minion with the chosen
/// equipment leaves the controlled region".
#[test]
fn bauble_basic_burns_once_its_equipment_bearer_is_taken() {
    let mut e = game(2);
    let (_, o, bauble, gun) = bauble_fixture(&mut e, 0);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let req = request(&mut e);
    assert!(
        !req.options.iter().any(|c| uses_of(c, bauble, 1)),
        "still the player's"
    );
    let i = req
        .options
        .iter()
        .position(|c| matches!(c, Choice::TakeAction { .. }))
        .expect("an action");
    e.state.minions[o.index()].owner = SeatId(1);
    e.choose(i);
    let req = offering_where(&mut e, |c| uses_of(c, bauble, 1));
    let i = req
        .options
        .iter()
        .position(|c| uses_of(c, bauble, 1))
        .unwrap();
    e.choose(i);
    request(&mut e);
    assert!(!e.state.cards[bauble.index()].in_play && !e.state.cards[gun.index()].in_play);
}

/// Monomancy's actor (seat 0) and its target (seat 1, with Majesty [pre]).
fn monomancy_game() -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let pre = textless(
        |c| {
            c.sect == Some(Sect::Sabbat)
                && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("pre"))
        },
        0,
    );
    let t = e.add_minion(SeatId(1), pre, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Monomancy"]);
    hand(&mut e, 1, &["Majesty"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(t)));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::ActionBlock)
    );
    e.choose(index_of(&req, Choice::Pass));
    (e, a, t)
}

/// Monomancy: its target ends the combat before range in the first round;
/// no strike; the card itself put on the target, -1 bleed.
/// [RTR 20020501] By Monomancy's own effect, the card put on.
#[test]
fn monomancy_target_may_end_combat_and_takes_the_card() {
    let (mut e, a, t) = monomancy_game();
    let req = offering(&mut e, Choice::EndCombat);
    assert_eq!(req.seat, SeatId(1));
    e.choose(index_of(&req, Choice::EndCombat));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Monomancy", Some(t));
    assert_eq!(e.state.bearer_mod(t, Stat::Bleed, ActionKind::Bleed), -1);
    assert_eq!((blood(&e, a), blood(&e, t)), (4, 4), "no strike");
    assert!(e.combat().is_none());
}

/// Monomancy: its target ends the combat with Majesty's strike instead: the
/// card goes to the acting Methuselah's ash heap, not on the target.
/// [RTR 20020501] "The card is not put on the target vampire if he or she
/// uses another effect than Monomancy to end combat."
#[test]
fn monomancy_not_put_on_when_combat_ends_otherwise() {
    let (mut e, _, t) = monomancy_game();
    let req = offering(&mut e, Choice::EndCombat);
    e.choose(index_of(&req, Choice::Pass));
    pick_when(&mut e, play(0, 0, t));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Majesty"));
    assert!(ash_has(&e, 0, "Monomancy"));
    assert!(!e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == find_library("Monomancy")));
    assert_eq!(e.state.bearer_mod(t, Stat::Bleed, ActionKind::Bleed), 0);
}

// ---- Vampires (vamp_b) ------------------------------------------------------

/// The abilities of vampire `m` offered.
fn vuses(req: &zvtes::decision::DecisionRequest, m: MinionId) -> Vec<Choice> {
    req.options
        .iter()
        .copied()
        .filter(|o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == m))
        .collect()
}

/// Pass (or hand strike) until vampire `m`'s ability `ability` is offered,
/// and take it.
fn pick_vuse(e: &mut Engine, m: MinionId, ability: u8) {
    for _ in 0..200 {
        let req = request(e);
        if let Some(i) = req.options.iter().position(|o| {
            matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), ability: a, .. }
                if *v == m && *a == ability)
        }) {
            e.choose(i);
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("ability {ability} of {m:?} never offered");
}

/// Vampire `src`'s ability `ability` used by `actor` at `target`.
fn vuse(src: MinionId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(src),
        ability,
        actor,
        target,
    }
}

/// Hukros: "During your unlock phase, you may move a card from your hand
/// to Hukros face down." Moved, the card is replaced (rules-qa 37.1), out
/// of the hand size; once each unlock phase, the next one's joining it.
/// [LSJ 20070320-2] "He can accumulate as many cards as he likes with his
/// special, at the rate of one each unlock."
#[test]
fn hukros_stashes_a_card_each_unlock_phase() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hukros"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cloak = find_library("Cloak the Gathering");
    let soak = find_library("Soak");
    e.state.seats[0].hand = vec![cloak, soak, soak, soak, soak, soak, soak].into();
    e.state.seats[0].library = vec![cloak; 10].into();
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert_eq!(vuses(&req, h).len(), 1, "{:?}", req.options);
    pick_vuse(&mut e, h, 0);
    pick(&mut e, Choice::Discard(0));
    assert_eq!(e.state.minions[h.index()].stored.0, vec![cloak]);
    assert_eq!(e.state.seats[0].hand.len(), 7, "replaced");
    assert_eq!(e.state.seats[0].library.len(), 9);
    assert_eq!(e.state.hand_load(SeatId(0)), 7, "not in the hand size");
    let req = request(&mut e);
    assert!(vuses(&req, h).is_empty(), "once: {:?}", req.options);
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert_eq!(vuses(&req, h).len(), 1);
    pick_vuse(&mut e, h, 0);
    pick(&mut e, Choice::Discard(0));
    assert_eq!(e.state.minions[h.index()].stored.0.len(), 2);
}

/// Hukros "may play that card as if from your hand": only he (LSJ
/// 2006-10-10: "only by Hukros"), not replaced once played (LSJ
/// 2006-10-04: "cards from Hukros's stash … are not replaced at all");
/// another of his Methuselah's minions is not offered it.
#[test]
fn hukros_alone_plays_his_card_unreplaced() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hukros"), 4, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[other.index()].beast_disciplines[0] = Some("obf");
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cloak = find_library("Cloak the Gathering");
    e.state.minions[h.index()]
        .stored
        .0
        .put(cloak, Default::default());
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Soak"); 5].into();
    // Another minion: not his card to play.
    pick(&mut e, bleed(other));
    pick_when(&mut e, block(z));
    let req = request(&mut e);
    assert!(
        !plays(&req).contains(&play(7, 0, other)),
        "{:?}",
        req.options
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    // Hukros: his card, the block then failing.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[other.index()].locked = true;
    let pool = e.state.seats[1].pool;
    pick(&mut e, bleed(h));
    pick_when(&mut e, block(z));
    pick(&mut e, play(7, 0, h));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[h.index()].stored.0.is_empty());
    assert!(ash_has(&e, 0, "Cloak the Gathering"));
    assert_eq!(e.state.seats[0].library.len(), 5, "not replaced");
    assert_eq!(e.state.seats[1].pool, pool - 1, "unblocked");
}

/// Hukros's cards: hidden to the other Methuselahs ("you may look at the
/// card at any time"), never in a hand; burned with him to the ash heap
/// (LSJ 2006-10-10: "it would burn if Hukros is burned").
#[test]
fn hukros_stash_is_hidden_and_burns_with_him() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hukros"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cards = [find_library("Cloak the Gathering"), find_library("Soak")];
    for c in cards {
        e.state.minions[h.index()]
            .stored
            .0
            .put(c, Default::default());
    }
    let mine = e.state.legal_view(SeatId(0));
    assert_eq!(mine.minions[h.index()].stored.0, cards.to_vec());
    let theirs = e.state.legal_view(SeatId(1));
    assert!(theirs.minions[h.index()]
        .stored
        .0
        .iter()
        .all(|&c| c == zvtes::state::HIDDEN));
    e.burn(h);
    assert!(e.state.minions[h.index()].stored.0.is_empty());
    assert!(ash_has(&e, 0, "Cloak the Gathering") && ash_has(&e, 0, "Soak"));
}

/// Humo: "If you also control Greensleeves and he is ready, Humo may burn
/// a blood to give Greensleeves +1 stealth once each action." Greensleeves
/// blocked: Humo's blood buys the stealth, once; another minion of his
/// Methuselah's acting is not offered it.
#[test]
fn humo_gives_greensleeves_stealth_once_each_action() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Greensleeves"), 4, false);
    let humo = e.add_minion(SeatId(0), find_card("Humo"), 4, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(other));
    pick_when(&mut e, block(z));
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == humo),
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let pool = e.state.seats[1].pool;
    let before = blood(&e, humo);
    pick(&mut e, bleed(g));
    pick_when(&mut e, block(z));
    pick_vuse(&mut e, humo, 0);
    assert_eq!(blood(&e, humo), before - 1);
    let req = request(&mut e);
    assert!(vuses(&req, humo).is_empty(), "once: {:?}", req.options);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, pool - 1, "the block failed");
}

/// Greensleeves: "If you also control Humo, Greensleeves may burn a blood
/// to give Humo +1 bleed during a bleed action." Humo now in the pool, the
/// text applies: on Humo's bleed, before resolution; not on another action.
/// [ANK 20231124] [LSJ 19970718] "The ability to increase bleed can only be
/// used during a bleed action before Action Resolution."
#[test]
fn greensleeves_gives_humo_a_bleed_before_resolution() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Greensleeves"), 4, false);
    let humo = e.add_minion(SeatId(0), find_card("Humo"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, action(humo, ActionKind::Hunt));
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == g),
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let before = blood(&e, g);
    pick(&mut e, bleed(humo));
    pick_vuse(&mut e, g, 0);
    assert_eq!(blood(&e, g), before - 1);
    let req = request(&mut e);
    assert!(vuses(&req, g).is_empty(), "{:?}", req.options);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2, "bled for 2");
}

/// Icarus: "When Icarus bleeds a Methuselah successfully, you may look at
/// that Methuselah's hand." The bled Methuselah's hand, looked at; a
/// blocked bleed gives no look.
#[test]
fn icarus_sees_the_bled_hand() {
    let mut e = game(2);
    let icarus = e.add_minion(SeatId(0), find_card("Icarus, The Manchurian"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Soak"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(icarus));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(noted_looks(&mut e).is_empty());
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[icarus.index()].locked = false;
    pick(&mut e, bleed(icarus));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let looks = noted_looks(&mut e);
    assert_eq!(looks.len(), 1, "{looks:?}");
    assert_eq!((looks[0].viewer, looks[0].seat), (SeatId(0), SeatId(1)));
    assert!(looks[0].cards.contains(&find_library("Soak")));
}

/// Isouda de Blaise: "You may burn the Edge during your discard phase to
/// unlock Isouda." Locked, with the Edge: unlocked, the Edge gone; without
/// it, not offered.
#[test]
fn isouda_burns_the_edge_to_unlock_in_the_discard_phase() {
    let mut e = game(2);
    let isouda = e.add_minion(SeatId(0), find_card("Isouda de Blaise"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[isouda.index()].locked = true;
    e.state.seats[0].has_edge = true;
    let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert_eq!(vuses(&req, isouda).len(), 1, "{:?}", req.options);
    pick_vuse(&mut e, isouda, 0);
    assert!(!e.state.minions[isouda.index()].locked);
    assert!(!e.state.seats[0].has_edge);
    // No Edge: not offered.
    let mut e = game(2);
    let isouda = e.add_minion(SeatId(0), find_card("Isouda de Blaise"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[isouda.index()].locked = true;
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == isouda),
    );
}

/// Jacques Molay: "Jacques can enter combat with an ally or younger vampire
/// as a Ⓓ action. +1 strength." A younger vampire offered, an older one
/// not; his hand strike deals 2.
#[test]
fn jacques_molay_rushes_a_younger_vampire_with_extra_strength() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jacques Molay"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let young = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    let older = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.capacity >= 9 && !c.imbued && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("a vampire of capacity 9 or more") as u16;
    let old = e.add_minion(SeatId(1), older, 4, false);
    let req = request(&mut e);
    assert!(req
        .options
        .contains(&vuse(j, 0, Some(j), Target::Minion(young))));
    assert!(!req
        .options
        .contains(&vuse(j, 0, Some(j), Target::Minion(old))));
    pick(&mut e, vuse(j, 0, Some(j), Target::Minion(young)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, young), 4 - 2);
}

/// Jaggedy Andy: "After Andy goes to torpor, discard 2 cards at random
/// (draw up afterward)." Two discarded and drawn for; with one card, that
/// one.
/// [ANK 20211010] "If he goes to torpor and their controller cannot
/// discard two cards, they discard as many as they can."
#[test]
fn jaggedy_andy_discards_two_at_random_after_torpor() {
    for (n, library) in [(7usize, 10usize), (1, 0)] {
        let mut e = game(2);
        let andy = e.add_minion(SeatId(0), find_card("Jaggedy Andy"), 1, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[z.index()].beast_disciplines[0] = Some("POT");
        hand(&mut e, 1, &["Torn Signpost"]);
        e.state.seats[0].hand = vec![find_library("Soak"); n].into();
        e.state.seats[0].library = vec![find_library("Form of Mist"); library].into();
        pick(&mut e, bleed(andy));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, 1, z));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(e.state.minions[andy.index()].in_torpor);
        let discarded = n.min(2);
        assert_eq!(
            e.state.seats[0]
                .ash
                .iter()
                .filter(|&&c| c == find_library("Soak"))
                .count(),
            discarded
        );
        assert_eq!(
            e.state.seats[0].library.len(),
            library.saturating_sub(discarded)
        );
    }
}

/// Jane Sims: "Non-combat cards cost Jane +1 blood." Swallowed by the
/// Night as an action modifier costs her 1 (its printed cost 0), as a
/// combat card nothing more.
/// [PIB 20150924-2] "Card with multiple types (eg. Combat/Action Modifier)
/// are considered of the type they are played as, not both."
#[test]
fn jane_sims_pays_more_for_non_combat_cards() {
    let mut e = game(2);
    let jane = e.add_minion(SeatId(0), find_card("Jane Sims"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[jane.index()].beast_disciplines[0] = Some("OBF");
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Swallowed by the Night"]);
    pick(&mut e, bleed(jane));
    pick_when(&mut e, block(z));
    pick(&mut e, play(0, 0, jane));
    request(&mut e);
    assert_eq!(blood(&e, jane), 3, "+1 as an action modifier");
    let swift = find_library("Swallowed by the Night");
    let cost = |mode| {
        e.state.printed_mod(
            jane,
            Stat::BloodCostMore,
            zvtes::state::PrintedCtx {
                card: Some((swift, mode)),
                ..Default::default()
            },
        )
    };
    assert_eq!((cost(0), cost(1)), (1, 0), "the combat mode, nothing more");
}

/// Joseph Cambridge: "Joseph gets an additional +1 stealth on political
/// actions." Political: +1; a bleed: none.
#[test]
fn joseph_cambridge_gets_stealth_on_political_actions() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Joseph Cambridge"), 4, false);
    let on = |kind| {
        e.state.printed_mod(
            j,
            Stat::Stealth,
            zvtes::state::PrintedCtx {
                kind: Some(kind),
                ..Default::default()
            },
        )
    };
    assert_eq!((on(ActionKind::Political), on(ActionKind::Bleed)), (1, 0));
}

/// The `nth` fixture vampire matching `f`, titles allowed.
#[track_caller]
fn bare_vampire(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    fixture_vampire(f, nth)
}

/// Kai Simmons: "Titled Sabbat vampires controlled by other Methuselahs can
/// call a referendum to steal Kai as a +1 stealth political action."
/// Another Methuselah's bishop calls it, passes it: Kai is theirs for
/// good. An untitled Sabbat vampire is not offered, nor a bishop of Kai's
/// own Methuselah.
/// [LSJ 20080604] The action provided by this card is an action requiring
/// the listed clan/sect/capacity/title.
#[test]
fn kai_simmons_is_stolen_by_another_methuselahs_titled_sabbat_vampire() {
    let mut e = game(2);
    let kai = e.add_minion(SeatId(0), find_card("Kai Simmons"), 1, false);
    let own = e.add_minion(
        SeatId(0),
        bare_vampire(|c| c.title == Title::Bishop, 0),
        4,
        false,
    );
    let bishop = e.add_minion(
        SeatId(1),
        bare_vampire(|c| c.title == Title::Bishop, 1),
        4,
        false,
    );
    let untitled = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=11, 0), 4, false);
    let steal = |m| vuse(kai, 0, Some(m), Target::None);
    let req = request(&mut e);
    assert!(!req.options.contains(&steal(own)), "his own Methuselah's");
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(req.options.contains(&steal(bishop)), "{:?}", req.options);
    assert!(!req.options.contains(&steal(untitled)));
    pick(&mut e, steal(bishop));
    pick_when(&mut e, votes(bishop, true));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let k = &e.state.minions[kai.index()];
    assert_eq!(k.owner, SeatId(1));
    assert_eq!(k.temp_from, None, "for good");
}

/// Konrad Fleischer: "Once each round, Konrad may treat 1 aggravated damage
/// as normal. He gets an optional press each combat." An aggravated hand
/// strike landing on him, treated as normal: 1 blood burned, no torpor.
#[test]
fn konrad_fleischer_takes_one_aggravated_as_normal_each_round() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Konrad Fleischer"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].beast_disciplines[0] = Some("pro");
    hand(&mut e, 1, &["Wolf Claws"]);
    assert_eq!(
        e.state
            .printed_mod(k, Stat::CombatPress, Default::default()),
        1
    );
    pick(&mut e, bleed(k));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    pick_vuse(&mut e, k, 0);
    let req = request(&mut e);
    assert!(vuses(&req, k).is_empty(), "{:?}", req.options);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let v = &e.state.minions[k.index()];
    assert!(!v.in_torpor, "treated as normal");
    assert_eq!(v.blood, 3);
}

/// Konrad Fleischer outside combat (owner, rules-qa 122.96): his Sunrise
/// Service's aggravated point, once per damaging effect, his player's
/// choice: treated as normal, 1 blood burned, no torpor; declined, he goes
/// to torpor, blood kept.
#[test]
fn konrad_fleischer_treats_sunrise_service_damage_as_normal() {
    for treat in [true, false] {
        let mut e = game(2);
        let k = e.add_minion(SeatId(0), find_card("Konrad Fleischer"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Sunrise Service"]);
        pick(&mut e, play(0, 0, k));
        let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        let as_normal = Choice::ResolvePrinted { minion: k };
        assert_eq!(req.options, vec![Choice::Pass, as_normal]);
        e.choose(index_of(&req, if treat { as_normal } else { Choice::Pass }));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let v = &e.state.minions[k.index()];
        assert_eq!(v.in_torpor, !treat, "treated as normal: {treat}");
        assert_eq!(v.blood, if treat { 3 } else { 4 });
    }
}

/// Lady Zara Slatikov: "Zara gets 1 optional press each combat. She can
/// enter combat with a Baali as a Ⓓ action." A Baali offered, another
/// vampire not.
#[test]
fn lady_zara_rushes_a_baali() {
    let mut e = game(2);
    let zara = e.add_minion(SeatId(0), find_card("Lady Zara Slatikov"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let baali = CRYPT
        .iter()
        .position(|c| c.clan == "Baali")
        .expect("a Baali") as u16;
    let baali = e.add_minion(SeatId(1), baali, 4, false);
    let other = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = request(&mut e);
    assert!(req
        .options
        .contains(&vuse(zara, 0, Some(zara), Target::Minion(baali))));
    assert!(!req
        .options
        .contains(&vuse(zara, 0, Some(zara), Target::Minion(other))));
    assert_eq!(
        e.state
            .printed_mod(zara, Stat::CombatPress, Default::default()),
        1
    );
}

/// Laszlo Mirac: "When Laszlo calls a referendum, he may choose a younger
/// vampire. That vampire cannot cast votes or ballots in that referendum.
/// +1 bleed." Before any vote, a younger prince chosen casts nothing; a
/// vampire as old as Laszlo (capacity 10) is not offered.
#[test]
fn laszlo_mirac_silences_a_younger_vampire_in_his_referendum() {
    let mut e = game(2);
    let laszlo = e.add_minion(SeatId(0), find_card("Laszlo Mirac"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let prince = e.add_minion(
        SeatId(1),
        bare_vampire(
            |c| matches!(c.title, Title::Prince(_)) && c.capacity < 10,
            0,
        ),
        4,
        false,
    );
    let older = e.add_minion(SeatId(1), bare_vampire(|c| c.capacity >= 10, 0), 4, false);
    assert_eq!(
        e.state.printed_mod(laszlo, Stat::Bleed, Default::default()),
        1
    );
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    pick(&mut e, play(0, 0, laszlo));
    to_referendum(&mut e);
    let req = request(&mut e);
    let silence = |t| vuse(laszlo, 0, None, Target::Minion(t));
    assert!(req.options.contains(&silence(prince)), "{:?}", req.options);
    assert!(!req.options.contains(&silence(older)));
    pick(&mut e, silence(prince));
    loop {
        let req = request(&mut e);
        if e.referendum().is_none() {
            break;
        }
        assert!(!req
            .options
            .iter()
            .any(|o| matches!(o, Choice::CastVotes { minion, .. } if *minion == prince)));
        e.choose(fallback(&req));
    }
}

/// Leo Washington: "An older vampire opposing Leo gets an optional maneuver
/// on the first round of combat." An older blocker is offered a maneuver
/// in the first round, with no card; a younger one is not.
#[test]
fn leo_washingtons_older_opponent_gets_a_first_round_maneuver() {
    for (cap, offered) in [(4..=11, true), (1..=1, false)] {
        let mut e = game(2);
        let leo = e.add_minion(SeatId(0), find_card("Leo Washington"), 2, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let opponent = bare_vampire(|c| cap.contains(&c.capacity), 0);
        let z = e.add_minion(SeatId(1), opponent, 1, false);
        pick(&mut e, bleed(leo));
        pick_when(&mut e, block(z));
        let seen = offered_before(
            &mut e,
            Choice::UseManeuver,
            SeatId(1),
            WindowLabel::MinionPhase,
        );
        assert_eq!(seen, offered);
    }
}

/// Leo Washington's "older" read live (D13; rules-qa 111.46, Hanna
/// Redmonds): the older blocker's first-round maneuver is withdrawn once
/// it is no longer older.
#[test]
fn leo_washington_reads_the_opponents_age_live() {
    let mut e = game(2);
    let leo = e.add_minion(SeatId(0), find_card("Leo Washington"), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(
        SeatId(1),
        bare_vampire(|c| (4..=11).contains(&c.capacity), 0),
        1,
        false,
    );
    pick(&mut e, bleed(leo));
    pick_when(&mut e, block(z));
    let req = to_window(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&Choice::UseManeuver));
    let down =
        e.state.capacity_of(z).expect("capacity") - e.state.capacity_of(leo).expect("capacity");
    let src = in_play(&mut e, 1, ".44 Magnum", Some(z));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::HostCapacityDown(down),
        source: src,
    });
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::UseManeuver), "{req:?}");
}

/// Lernean: "Once each round, Lernean may burn 1 blood to get an additional
/// strike (limited)."
#[test]
fn lernean_buys_a_limited_additional_strike_each_round() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lernean"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(l));
    pick_when(&mut e, block(z));
    pick_vuse(&mut e, l, 0);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, l), 4 - 1 - 1, "a blood, a hand strike taken");
    assert_eq!(blood(&e, z), 4 - 2, "two hand strikes");
}

/// Lord Aaron Wesley Wilkshire: "Frenzy cards played on Aaron cost +1 blood
/// or pool." Terror Frenzy played on him in combat: paid in pool (pool -1,
/// blood at its cost) or in blood (+1); with no blood beyond the cost,
/// the pool pays, unasked.
/// [LSJ 20060831] "The methusalem playing the frenzy card chooses if the
/// cost is paid in blood or pool."
#[test]
fn lord_aaron_frenzy_cards_cost_blood_or_pool() {
    for (blood_at, in_pool, asked) in [(4u8, true, true), (4, false, true), (1, true, false)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Gangrel", 0), blood_at, false);
        e.state.minions[a.index()].beast_disciplines = [Some("ani"), Some("for"), None];
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let aaron = e.add_minion(
            SeatId(1),
            find_card("Lord Aaron Wesley Wilkshire"),
            4,
            false,
        );
        // Soak keeps a window open before the strikes' damage lands.
        hand(&mut e, 0, &["Terror Frenzy", "Soak"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(aaron));
        pick_when(&mut e, play(0, 0, a));
        let req = request(&mut e);
        if asked {
            assert_eq!(req.window, WindowLabel::PlayTerms, "{req:?}");
            let pay = if in_pool {
                Choice::BurnPool
            } else {
                Choice::Pass
            };
            pick(&mut e, pay);
        }
        let req = request(&mut e);
        assert_eq!(req.window, WindowLabel::CombatPrevention, "{req:?}");
        let (pool, spent) = (e.state.seats[0].pool, blood_at - blood(&e, a));
        if in_pool {
            assert_eq!((pool, spent), (9, 1), "the toll in pool");
        } else {
            assert_eq!((pool, spent), (10, 2), "the toll in blood");
        }
    }
}

/// Lord Aaron chosen by Bloodlust's terms ("Choose X minions"): its player
/// still chooses blood or pool for the +1 (LSJ 20060831), asked once the
/// picks are made; X = 1 paid in blood as the action resolves.
#[test]
fn lord_aaron_picked_by_bloodlust_offers_blood_or_pool() {
    for in_pool in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("pre", 0), 3, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let aaron = e.add_minion(
            SeatId(1),
            find_card("Lord Aaron Wesley Wilkshire"),
            4,
            false,
        );
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Bloodlust"]);
        pick(&mut e, play(0, 0, a));
        pick(&mut e, Choice::ChooseMinion(aaron));
        pick(&mut e, Choice::Pass);
        let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
        assert!(req.options.contains(&Choice::BurnPool), "{req:?}");
        e.choose(index_of(
            &req,
            if in_pool {
                Choice::BurnPool
            } else {
                Choice::Pass
            },
        ));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(ash_has(&e, 0, "Bloodlust"));
        let (pool, spent) = (e.state.seats[0].pool, 3 - blood(&e, a));
        if in_pool {
            assert_eq!((pool, spent), (9, 1), "the toll in pool");
        } else {
            assert_eq!((pool, spent), (10, 2), "the toll in blood");
        }
    }
}

/// Lucubratio: "Once each turn, when Lucubratio blocks, he may discard a
/// card that requires Auspex [aus] from your hand instead of locking for
/// the block." The Auspex card discarded (and replaced): he stays unlocked;
/// a second block that turn locks him, unasked; the next Methuselah's
/// turn, offered again.
/// [ANK 20180719-4] "\"Once each turn\" means once during each player's
/// turn."
#[test]
fn lucubratio_discards_auspex_instead_of_locking_once_each_turn() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let a = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let luc = e.add_minion(SeatId(2), find_card("Lucubratio"), 4, false);
    hand(&mut e, 2, &["Eyes of Argus", "Soak"]);
    e.state.seats[2].library = vec![find_library("Soak"); 20].into();
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(luc));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::ResolutionChoice, "{req:?}");
    assert_eq!(req.options, vec![Choice::Pass, Choice::Discard(0)]);
    let library = e.state.seats[2].library.len();
    e.choose(1);
    assert!(!e.state.minions[luc.index()].locked);
    assert!(ash_has(&e, 2, "Eyes of Argus"));
    assert_eq!(e.state.seats[2].library.len(), library - 1, "replaced");
    hand(&mut e, 2, &["Eyes of Argus"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(luc));
    let req = request(&mut e);
    assert_ne!(req.window, WindowLabel::ResolutionChoice, "once each turn");
    assert!(e.state.minions[luc.index()].locked);
    // A later turn: offered again.
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(luc));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::ResolutionChoice, "{req:?}");
}

/// Luke Fellows: "If Luke bleeds successfully, he unlocks at the end of the
/// action." Bled for 1: unlocked; blocked: locked.
#[test]
fn luke_fellows_unlocks_after_a_successful_bleed() {
    let mut e = game(2);
    let luke = e.add_minion(SeatId(0), find_card("Luke Fellows"), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    pick(&mut e, bleed(luke));
    let req = request(&mut e);
    assert_eq!(e.state.seats[1].pool, 9);
    assert!(!e.state.minions[luke.index()].locked, "unlocked: {req:?}");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[z.index()].locked = false;
    pick(&mut e, bleed(luke));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[luke.index()].locked);
}

/// Margarite: "Any Tzimisce can enter combat with Margarite as a Ⓓ
/// action." Another Methuselah's Tzimisce offered (it enters combat), a
/// non-Tzimisce not.
/// [LSJ 20080604] An action requiring the listed clan.
#[test]
fn margarite_is_rushed_by_another_methuselahs_tzimisce() {
    let mut e = game(2);
    let tz = e.add_minion(SeatId(0), plain("Tzimisce", 0), 4, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let m = e.add_minion(SeatId(1), find_card("Margarite"), 1, false);
    let rush = |a| vuse(m, 0, Some(a), Target::Minion(m));
    let req = request(&mut e);
    assert!(req.options.contains(&rush(tz)), "{:?}", req.options);
    assert!(!req.options.contains(&rush(other)));
    pick(&mut e, rush(tz));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, tz), 3, "in combat with her");
}

/// A rush targets a ready minion (rules-qa 17.3): Margarite, Marcus Vitel
/// (advanced) and Giangaleazzo in torpor are offered to no rusher of their
/// clan; ready, they are.
#[test]
fn margarite_marcus_and_giangaleazzo_in_torpor_are_not_rushed() {
    for (name, adv, clan) in [
        ("Margarite", false, "Tzimisce"),
        ("Marcus Vitel", true, "Brujah"),
        ("Giangaleazzo, The Traitor", false, "Lasombra"),
    ] {
        for torpid in [false, true] {
            let mut e = game(2);
            let a = e.add_minion(SeatId(0), plain(clan, 0), 4, false);
            e.add_minion(SeatId(0), plain("Gangrel", 0), 4, false);
            let m = e.add_minion(SeatId(1), crypt_card(name, adv), 2, torpid);
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
            let req = request(&mut e);
            let rushed = req.options.iter().any(|o| {
                matches!(o, Choice::UseAbility { actor: Some(y), target: Target::Minion(t), .. }
                    if *y == a && *t == m)
            });
            assert_eq!(rushed, !torpid, "{name}, torpid {torpid}");
        }
    }
}

/// Mary Johnson: "Older vampires with Presence [pre] can steal Mary as a Ⓓ
/// action." An older Presence vampire takes her for good; one without
/// Presence is not offered.
#[test]
fn mary_johnson_is_stolen_by_an_older_presence_vampire() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), vampire_with("pre", 0), 4, false);
    let other = e.add_minion(SeatId(0), plain("Gangrel", 0), 4, false);
    let mary = e.add_minion(SeatId(1), find_card("Mary Johnson"), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let steal = |a| vuse(mary, 0, Some(a), Target::Minion(mary));
    let req = request(&mut e);
    assert!(req.options.contains(&steal(p)), "{:?}", req.options);
    assert!(!req.options.contains(&steal(other)));
    pick(&mut e, steal(p));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[mary.index()].owner, SeatId(0));
}

/// Melinda Galbraith (advanced): "During your unlock phase, Melinda can
/// lock a Sabbat vampire with capacity 6 or less." From torpor, another
/// Methuselah's capacity-5 Sabbat vampire locked; a capacity-7 one not
/// offered.
/// [LSJ 20011022] "The ability can be used when in torpor."
#[test]
fn melinda_galbraith_locks_a_small_sabbat_vampire_from_torpor() {
    let mut e = game(2);
    let mel = e.add_minion(SeatId(0), crypt_card("Melinda Galbraith", true), 4, true);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let small = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 5..=5, 0), 4, false);
    let big = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 7..=7, 0), 4, false);
    let lock = |t| vuse(mel, 0, None, Target::Minion(t));
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&lock(small)), "{:?}", req.options);
    assert!(!req.options.contains(&lock(big)));
    pick(&mut e, lock(small));
    assert!(e.state.minions[small.index()].locked);
}

/// Melinda Galbraith merged: "[MERGED] Sabbat regent: +2 bleed." Bleeds for
/// 3; regent.
#[test]
fn melinda_galbraith_merged_bleeds_for_three() {
    let mut e = game(2);
    let mel = e.add_minion(SeatId(0), crypt_card("Melinda Galbraith", true), 4, false);
    assert_eq!(
        e.state.printed_mod(mel, Stat::Bleed, Default::default()),
        0,
        "unmerged"
    );
    e.state.minions[mel.index()].merged = Some(crypt_card("Melinda Galbraith", false));
    assert_eq!(e.state.printed_mod(mel, Stat::Bleed, Default::default()), 2);
    assert_eq!(e.state.title_of(mel), Title::Regent);
}
