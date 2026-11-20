//! Round 33 (`ADDED_BATCHES`): Kindred Most Wanted and Tenth Anniversary
//! completed. Windows with a single option are elided, so the scripts only
//! stop where a real decision exists.
#![allow(dead_code)]

use zvtes::cards::Target;
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{Choice, WindowLabel};
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

/// A seat-0 Camarilla vampire holding `title`, beside another seat-0
/// vampire (the minion phase not elided).
fn titled_caller(e: &mut Engine, title: Title) -> MinionId {
    let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    e.state.minions[a.index()].granted_title = Some(title);
    e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 1), 4, false);
    a
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

fn ash_crypt_has(e: &Engine, seat: usize, card: u16) -> bool {
    e.state.seats[seat].ash_crypt.contains(&card)
}

// ---- Political actions ----------------------------------------------------

/// The Final Nights: "Successful referendum means each vampire burns 1
/// blood": every vampire in play, any Methuselah's, the caller's own and
/// those in torpor too (rules-qa 59.9).
#[test]
fn the_final_nights_bleeds_every_vampire_torpor_too() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    let b = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 1), 4, true);
    let c = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    let d = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 1), 3, true);
    hand(&mut e, 0, &["Final Nights, The"]);
    pass_referendum(&mut e, a, &[]);
    request(&mut e);
    assert_eq!(
        [a, b, c, d].map(|m| blood(&e, m)),
        [3, 3, 3, 2],
        "each vampire burned 1"
    );
}

/// The Final Nights: "If this referendum fails, the acting vampire burns 1
/// blood": held and failed (no vote cast), the acting vampire alone; passed,
/// not it beyond the others' 1 (rules-qa 43.1: a fail rider).
#[test]
fn the_final_nights_failed_costs_the_actor_a_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    let b = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 1), 4, false);
    let c = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    hand(&mut e, 0, &["Final Nights, The"]);
    fail_referendum(&mut e, a, &[]);
    request(&mut e);
    assert_eq!([a, b, c].map(|m| blood(&e, m)), [3, 4, 4]);
}

/// Gangrel Justicar: the card itself on the chosen Camarilla Gangrel,
/// another Methuselah's, its unique Camarilla title; in its referendum each
/// Gangrel gets 1 extra vote (the chosen one, untitled, 1 vote).
#[test]
fn gangrel_justicar_titles_a_camarilla_gangrel_with_clan_votes() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 1), 4, false);
    let g = e.add_minion(
        SeatId(1),
        textless(
            |c| c.clan == "Gangrel" && c.sect == Some(Sect::Camarilla),
            0,
        ),
        4,
        false,
    );
    hand(&mut e, 0, &["Gangrel Justicar"]);
    pick(&mut e, play(0, 0, a));
    // The one Gangrel chosen, elided.
    let req = until(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    let r = e.referendum().expect("a referendum");
    assert_eq!(r.votes_of(&e.state, g), 1, "the Gangrel's extra vote");
    e.choose(index_of(&req, Choice::CastCardVote { favor: true }));
    close_polling(&mut e);
    request(&mut e);
    placed_on(&e, "Gangrel Justicar", Some(g));
    assert_eq!(e.state.title_of(g), Title::Justicar("Gangrel"));
}

/// Gangrel Justicar is not played with no ready Camarilla Gangrel: a
/// Gangrel antitribu, a torpid Gangrel are none. [LSJ 20100129] [ANK
/// 20191228]
#[test]
fn gangrel_justicar_needs_a_ready_camarilla_gangrel() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 1), 4, false);
    e.add_minion(
        SeatId(1),
        textless(|c| c.clan == "Gangrel antitribu", 0),
        4,
        false,
    );
    let g = e.add_minion(
        SeatId(1),
        textless(
            |c| c.clan == "Gangrel" && c.sect == Some(Sect::Camarilla),
            0,
        ),
        4,
        true,
    );
    hand(&mut e, 0, &["Gangrel Justicar"]);
    assert!(!request(&mut e).options.contains(&play(0, 0, a)));
    // Ready, the window built anew at seat 0's next minion phase.
    e.state.minions[g.index()].in_torpor = false;
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&play(0, 0, a)));
}

/// Ravnos Acceptance: passed, the card in play and each Methuselah, from
/// the caller clockwise, picks of their Ravnos in play (torpor too) which
/// become Camarilla, one at a time until a pass; the others keep their
/// sect; a non-Ravnos is not offered (rules-digest D11, D28).
#[test]
fn ravnos_acceptance_lets_each_ravnos_turn_camarilla() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    let r0 = e.add_minion(SeatId(0), of_clan("Ravnos", 3..=8, 0), 4, false);
    let r1 = e.add_minion(SeatId(1), of_clan("Ravnos", 3..=8, 1), 3, true);
    let r2 = e.add_minion(SeatId(1), of_clan("Ravnos", 3..=8, 2), 3, false);
    let x = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 1), 4, false);
    let before = e.state.sect_of(r2);
    hand(&mut e, 0, &["Ravnos Acceptance"]);
    pass_referendum(&mut e, a, &[]);
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    assert_eq!(req.options, [Choice::Pass, Choice::ChooseMinion(r0)]);
    e.choose(1);
    let req = expect(&mut e, SeatId(1), WindowLabel::ReferendumTerms);
    assert_eq!(
        req.options,
        [
            Choice::Pass,
            Choice::ChooseMinion(r1),
            Choice::ChooseMinion(r2)
        ]
    );
    assert!(!req.options.contains(&Choice::ChooseMinion(x)));
    e.choose(index_of(&req, Choice::ChooseMinion(r1)));
    pick(&mut e, Choice::Pass);
    request(&mut e);
    placed_on(&e, "Ravnos Acceptance", None);
    assert_eq!(e.state.sect_of(r0), Some(Sect::Camarilla));
    assert_eq!(e.state.sect_of(r1), Some(Sect::Camarilla));
    assert_eq!(e.state.sect_of(r2), before);
    assert_ne!(before, Some(Sect::Camarilla));
}

/// Ravnos Acceptance in play: "As any Ravnos enters play, they can choose
/// to become Camarilla": a Ravnos influenced by another Methuselah is
/// offered it once, two copies in play notwithstanding; a non-Ravnos is not.
#[test]
fn ravnos_acceptance_offers_camarilla_to_an_entering_ravnos() {
    for ravnos in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
        e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 1), 4, false);
        e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 2), 4, false);
        in_play(&mut e, 0, "Ravnos Acceptance", None);
        in_play(&mut e, 0, "Ravnos Acceptance", None);
        let card = if ravnos {
            of_clan("Ravnos", 3..=8, 0)
        } else {
            of_sect(Sect::Sabbat, 4..=8, 3)
        };
        uncontrolled(&mut e, 1, card, 10);
        to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
        pick(&mut e, Choice::InfluenceOut { slot: 0 });
        let m = MinionId(e.state.minions.len() as u32 - 1);
        assert_eq!(e.state.minions[m.index()].crypt_card(), Some(card));
        let join = Choice::ChooseMinion(m);
        if !ravnos {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, join);
            continue;
        }
        let req = expect(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
        assert_eq!(req.options, [Choice::Pass, join]);
        e.choose(1);
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, join);
        assert_eq!(e.state.sect_of(m), Some(Sect::Camarilla));
    }
}

/// Giovanni Acceptance, Invitation Accepted, Revocation of Tyre: as Ravnos
/// Acceptance for the clan each names, exactly: Giovanni is not Hecata
/// (rules-qa 121.54).
#[test]
fn giovanni_acceptance_skips_hecata() {
    for (card, clan, other) in [
        ("Giovanni Acceptance", "Giovanni", "Hecata"),
        ("Invitation Accepted", "Ministry", "Ravnos"),
        ("Revocation of Tyre", "Banu Haqim", "Ravnos"),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
        let own = e.add_minion(
            SeatId(0),
            textless(|c| c.clan == clan && c.sect != Some(Sect::Camarilla), 0),
            4,
            false,
        );
        let not = e.add_minion(SeatId(0), of_clan(other, 3..=8, 0), 4, false);
        e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 1), 4, false);
        hand(&mut e, 0, &[card]);
        pass_referendum(&mut e, a, &[]);
        let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
        assert_eq!(
            req.options,
            [Choice::Pass, Choice::ChooseMinion(own)],
            "{card}"
        );
        assert!(!req.options.contains(&Choice::ChooseMinion(not)));
        e.choose(1);
        request(&mut e);
        placed_on(&e, card, None);
        assert_eq!(e.state.sect_of(own), Some(Sect::Camarilla), "{card}");
    }
}

/// Might of the Camarilla: "each Methuselah burns a vampire (of his or her
/// choosing) in his or her uncontrolled region": the caller's too, an
/// imbued none (rules-qa 87.8), a single vampire taken without asking; to
/// the owner's ash heap, the other slots kept, no blood hunt (rules-qa
/// 113.52). Not playable by an untitled vampire.
#[test]
fn might_of_the_camarilla_burns_one_uncontrolled_vampire_each() {
    let imbued = (0..CRYPT.len() as u16)
        .find(|&c| CRYPT[c as usize].imbued)
        .expect("an imbued");
    let (v0, v1, v2) = (
        of_sect(Sect::Sabbat, 4..=8, 0),
        of_sect(Sect::Sabbat, 4..=8, 1),
        of_sect(Sect::Sabbat, 4..=8, 2),
    );
    let mut e = game(3);
    let u = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 2), 4, false);
    let a = titled_caller(&mut e, Title::Justicar("Ventrue"));
    e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 3), 4, false);
    e.add_minion(SeatId(2), of_sect(Sect::Sabbat, 4..=8, 4), 4, false);
    uncontrolled(&mut e, 0, v0, 2);
    uncontrolled(&mut e, 0, imbued, 1);
    uncontrolled(&mut e, 1, v1, 3);
    uncontrolled(&mut e, 1, v2, 1);
    // Seat 2's one vampire, banished with an equipment on it: burned with it.
    let v3 = of_sect(Sect::Sabbat, 4..=8, 5);
    let b = e.add_minion(SeatId(2), v3, 0, false);
    let gear = in_play(&mut e, 2, "Flak Jacket", Some(b));
    e.state.minions[b.index()].in_play = false;
    e.state.cards[gear.index()].in_play = false;
    e.state.seats[2].uncontrolled.push(UncontrolledSlot {
        card: v3,
        copy: Default::default(),
        counters: 2,
        banished: Some(zvtes::state::Banished {
            minion: b,
            cards: vec![gear],
        }),
    });
    hand(&mut e, 0, &["Might of the Camarilla"]);
    assert!(
        !request(&mut e).options.contains(&play(0, 0, u)),
        "untitled"
    );
    pass_referendum(&mut e, a, &[]);
    let req = expect(&mut e, SeatId(1), WindowLabel::ReferendumTerms);
    assert_eq!(req.options, [Choice::Slot(0), Choice::Slot(1)], "no pass");
    e.choose(1);
    request(&mut e);
    let cards = |s: usize| -> Vec<u16> {
        e.state.seats[s]
            .uncontrolled
            .iter()
            .map(|x| x.card)
            .collect()
    };
    assert_eq!(cards(0), [imbued]);
    assert_eq!(cards(1), [v1]);
    assert!(ash_crypt_has(&e, 0, v0) && ash_crypt_has(&e, 1, v2));
    assert!(e.state.seats[2].uncontrolled.is_empty() && ash_crypt_has(&e, 2, v3));
    assert!(e.state.seats[2].ash.contains(&find_library("Flak Jacket")));
    assert!(e.referendum().is_none(), "no blood hunt");
}

/// The Peace of Khetamon: another Methuselah's vampire in torpor to their
/// uncontrolled region, its blood as counters and its equipment out of play
/// on it; influenced again, it comes back ready and unlocked, holding the
/// equipment (rules-qa 5.7).
#[test]
fn the_peace_of_khetamon_banishes_a_torpid_vampire_with_its_cards() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 1), 4, false);
    e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    let card = of_sect(Sect::Camarilla, 4..=8, 1);
    let t = e.add_minion(SeatId(1), card, 3, true);
    let gear = in_play(&mut e, 1, "Flak Jacket", Some(t));
    hand(&mut e, 0, &["Peace of Khetamon, The"]);
    pass_referendum(&mut e, a, &[Choice::ChooseMinion(t)]);
    request(&mut e);
    let region = &e.state.seats[1].uncontrolled;
    assert_eq!(region.len(), 1);
    assert_eq!((region[0].card, region[0].counters), (card, 3));
    let banished = region[0].banished.as_ref().expect("banished");
    assert_eq!(banished.cards, [gear]);
    assert!(!e.state.minions[t.index()].in_play);
    assert!(!e.state.cards[gear.index()].in_play);
    e.state.seats[1].uncontrolled[0].counters = CRYPT[card as usize].capacity;
    to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    request(&mut e);
    let v = &e.state.minions[t.index()];
    assert!(v.ready() && !v.locked && !v.in_torpor);
    assert!(e.state.cards[gear.index()].in_play);
    assert_eq!(e.state.cards[gear.index()].host, Some(t));
}

/// The Peace of Khetamon needs a vampire in torpor: not played with none
/// [LSJ 20100129] [ANK 20191228]; one rescued from torpor during the
/// polling is no longer a legal choice as it passes: it stays (rules-digest
/// D1; Banishment re-checked alike).
#[test]
fn the_peace_of_khetamon_needs_a_vampire_in_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 1), 4, false);
    let t = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    hand(&mut e, 0, &["Peace of Khetamon, The"]);
    assert!(!request(&mut e).options.contains(&play(0, 0, a)));

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 1), 4, false);
    e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 1), 4, false);
    let _ = t;
    let t = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 0), 4, true);
    e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 2), 4, true);
    hand(&mut e, 0, &["Peace of Khetamon, The"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::ChooseMinion(t));
    expect(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    e.state.minions[t.index()].in_torpor = false;
    pick_matching(&mut e, |o| *o == Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    request(&mut e);
    assert!(e.state.minions[t.index()].in_play);
    assert!(e.state.seats[1].uncontrolled.is_empty());
}

/// Praxis Seizure: Barcelona on a Camarilla Tremere: the card on it, its
/// capacity +1 while it holds the title; María del Toro, Prince of
/// Barcelona, entering contests the title: the capacity back to printed.
/// [TOM 19960210]
#[test]
fn praxis_seizure_barcelona_raises_a_tremere_princes_capacity() {
    let mut e = game(2);
    let tremere = textless(
        |c| c.clan == "Tremere" && c.sect == Some(Sect::Camarilla) && (4..=8).contains(&c.capacity),
        0,
    );
    let a = e.add_minion(SeatId(0), tremere, 4, false);
    e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    let printed = CRYPT[tremere as usize].capacity;
    hand(&mut e, 0, &["Praxis Seizure: Barcelona"]);
    pass_referendum(&mut e, a, &[]);
    request(&mut e);
    placed_on(&e, "Praxis Seizure: Barcelona", Some(a));
    assert_eq!(e.state.title_of(a), Title::Prince("Barcelona"));
    assert_eq!(e.state.capacity_of(a), Some(printed + 1));
    let m = e.add_minion(SeatId(1), find_card("María del Toro"), 4, false);
    e.stage_contests(m);
    assert_eq!(e.state.title_of(a), Title::None, "contested");
    assert_eq!(e.state.capacity_of(a), Some(printed));
}

/// Praxis Seizure: Rome: a non-Brujah prince gets no capacity; a Brujah
/// prince +1, lost as Tryphosa, Prince of Rome, contests it. [TOM
/// 19960210]
#[test]
fn praxis_seizure_rome_gives_a_non_brujah_no_capacity() {
    for brujah in [false, true] {
        let mut e = game(2);
        let card = textless(
            |c| {
                (c.clan == "Brujah") == brujah
                    && c.sect == Some(Sect::Camarilla)
                    && (4..=8).contains(&c.capacity)
            },
            0,
        );
        let a = e.add_minion(SeatId(0), card, 4, false);
        e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 1), 4, false);
        e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
        let printed = CRYPT[card as usize].capacity;
        hand(&mut e, 0, &["Praxis Seizure: Rome"]);
        pass_referendum(&mut e, a, &[]);
        request(&mut e);
        placed_on(&e, "Praxis Seizure: Rome", Some(a));
        assert_eq!(e.state.title_of(a), Title::Prince("Rome"));
        assert_eq!(
            e.state.capacity_of(a),
            Some(printed + u8::from(brujah)),
            "brujah {brujah}"
        );
        if brujah {
            let m = e.add_minion(SeatId(1), find_card("Tryphosa"), 4, false);
            e.stage_contests(m);
            assert_eq!(e.state.capacity_of(a), Some(printed));
        }
    }
}

/// Sabbat Threat: each pass gives the chosen Methuselah a threat counter,
/// on one effect-only stand-in of theirs; a third leaves two ("never more
/// than two", still chosen: rules-digest D4); at that Methuselah's unlock
/// phase they burn 1 pool per counter, each phase. The caller may choose
/// none but others; any Methuselah may be chosen.
#[test]
fn sabbat_threat_burns_pool_per_counter_each_unlock() {
    let mut e = game(2);
    let mut callers = Vec::new();
    for (i, city) in ["Paris", "Berlin", "London"].into_iter().enumerate() {
        let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, i), 4, false);
        e.state.minions[a.index()].granted_title = Some(Title::Prince(city));
        callers.push(a);
    }
    e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 1), 4, false);
    for (n, &a) in callers.iter().enumerate() {
        hand(&mut e, 0, &["Sabbat Threat"]);
        pass_referendum(&mut e, a, &[Choice::ChooseSeat(SeatId(1)), Choice::Pass]);
        let counters = stand_ins(&e, "Sabbat Threat");
        assert_eq!(counters.len(), 1, "one stand-in");
        let k = &e.state.cards[counters[0].index()];
        assert_eq!((k.controller, k.host), (SeatId(1), None));
        assert!(k.against.contains(SeatId(1)) && !k.against.contains(SeatId(0)));
        assert_eq!(k.counters, (n as u8 + 1).min(2));
    }
    for _ in 0..2 {
        let before = e.state.seats[1].pool;
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, before - 2);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    }
}

/// Sabbat Threat: "You can gain pool from the Edge before burning it to
/// remove the threat counters, and lose no pool to the threat counters."
/// [TOM 19951214-1] At their unlock phase the chosen Methuselah takes the
/// Edge's pool, then burns the Edge: the counters gone, no pool burned.
#[test]
fn sabbat_threat_edge_burned_after_its_pool_clears_the_counters() {
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Prince("Paris"));
    e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 1), 4, false);
    hand(&mut e, 0, &["Sabbat Threat"]);
    pass_referendum(&mut e, a, &[Choice::ChooseSeat(SeatId(1)), Choice::Pass]);
    let k = stand_ins(&e, "Sabbat Threat")[0];
    e.state.seats[1].has_edge = true;
    let before = e.state.seats[1].pool;
    let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&Choice::ResolveRider { card: k.0 }));
    e.choose(index_of(&req, Choice::BurnEdge));
    pick(&mut e, ability(k.0, 0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.seats[1].has_edge);
    assert!(!e.state.cards[k.index()].in_play);
    assert_eq!(e.state.seats[1].pool, before + 1);
}

/// Sacrifice: a Sabbat vampire of capacity 8 or more chooses a vampire of
/// capacity 6 or less of its clan, any Methuselah's, torpor too; one of
/// another clan is not offered; passed, the chosen burns. With no such
/// vampire it is not played. [LSJ 20100129] [ANK 20191228]
#[test]
fn sacrifice_burns_a_smaller_vampire_of_the_actors_clan() {
    let big = textless(|c| c.sect == Some(Sect::Sabbat) && c.capacity >= 8, 0);
    let clan = CRYPT[big as usize].clan;
    let small = |nth| {
        textless(
            |c| c.capacity <= 6 && c.clan != clan && c.sect == Some(Sect::Sabbat),
            nth,
        )
    };
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), big, 8, false);
    e.add_minion(SeatId(0), small(0), 4, false);
    let other = e.add_minion(SeatId(1), small(1), 4, false);
    hand(&mut e, 0, &["Sacrifice"]);
    assert!(!request(&mut e).options.contains(&play(0, 0, a)), "none");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), big, 8, false);
    e.add_minion(SeatId(0), small(0), 4, false);
    let t = e.add_minion(SeatId(1), small(2), 3, true);
    let r = e.add_minion(SeatId(1), small(3), 3, false);
    e.state.minions[t.index()].clan_change = Some(clan);
    e.state.minions[r.index()].clan_change = Some(clan);
    let _ = other;
    let other = e.add_minion(SeatId(1), small(1), 4, false);
    hand(&mut e, 0, &["Sacrifice"]);
    pick(&mut e, play(0, 0, a));
    let req = loop {
        let req = request(&mut e);
        if req.window == WindowLabel::ReferendumTerms {
            break req;
        }
        e.choose(fallback(&req));
    };
    assert!(req.options.contains(&Choice::ChooseMinion(t)));
    assert!(req.options.contains(&Choice::ChooseMinion(r)));
    assert!(!req.options.contains(&Choice::ChooseMinion(other)));
    e.choose(index_of(&req, Choice::ChooseMinion(t)));
    pick_matching(&mut e, |o| {
        *o == Choice::CastCardVote { favor: true } || *o == votes(a, true)
    });
    close_polling(&mut e);
    request(&mut e);
    assert!(!e.state.minions[t.index()].in_play, "burned");
    assert!(e.state.minions[r.index()].in_play);
}

/// Sacrifice by Vidal Jarbeaux (Toreador, capacity 8, Camarilla), meeting
/// the Sabbat requirement once: only Toreador vampires are offered, never
/// one of a clan he could meet "as if". [LSJ 20100526]
#[test]
fn sacrifice_by_vidal_matches_only_his_own_clan() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Vidal Jarbeaux"), 6, false);
    e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    let toreador = e.add_minion(SeatId(1), of_clan("Toreador", 3..=6, 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Toreador", 3..=6, 1), 4, false);
    let other = e.add_minion(
        SeatId(1),
        textless(|c| c.clan != "Toreador" && (3..=6).contains(&c.capacity), 0),
        4,
        false,
    );
    hand(&mut e, 0, &["Sacrifice"]);
    pick(&mut e, play(0, 0, v));
    let req = loop {
        let req = request(&mut e);
        if req.window == WindowLabel::ReferendumTerms {
            break req;
        }
        e.choose(fallback(&req));
    };
    assert!(req.options.contains(&Choice::ChooseMinion(toreador)));
    assert!(!req.options.contains(&Choice::ChooseMinion(other)));
}

/// Trumped-Up Charges on another Methuselah's ally (Ossian's own Red List
/// counted): that ally's controller gets 2 votes per Red List minion in
/// play, here 2, cast against; passed, the card itself on the ally, which
/// is Red List. An untitled vampire cannot play it. The Red List counted
/// again through the polling (owner, 2026-11-19): a minion made Red List
/// after the cast adds 2 against, and the referendum fails.
#[test]
fn trumped_up_charges_red_lists_a_minion_with_votes_per_red_list() {
    for more in [false, true] {
        let mut e = game(3);
        let u = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 2), 4, false);
        let a = titled_caller(&mut e, Title::Prince("Paris"));
        let ally = e.add_ally(SeatId(1), find_library("Gravebound Drone"));
        e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
        let s = e.add_minion(SeatId(2), of_sect(Sect::Sabbat, 4..=8, 1), 4, false);
        let ossian = e.add_ally(SeatId(2), find_library("Ossian"));
        assert!(e.state.is_red_list(ossian), "Ossian's printed Red List");
        assert!(!e.state.is_red_list(ally));
        hand(&mut e, 0, &["Trumped-Up Charges"]);
        assert!(
            !request(&mut e).options.contains(&play(0, 0, u)),
            "untitled"
        );
        pick(&mut e, play(0, 0, a));
        pick_when(&mut e, Choice::ChooseMinion(ally));
        let cast = Choice::CastGrantedVotes { favor: false };
        loop {
            let req = request(&mut e);
            if req.options.contains(&cast) {
                assert_eq!(req.seat, SeatId(1));
                e.choose(index_of(&req, cast));
                break;
            }
            assert!(
                !req.options
                    .iter()
                    .any(|o| matches!(o, Choice::CastGrantedVotes { .. })),
                "only the chosen minion's controller"
            );
            let i = if req.options.contains(&votes(a, true)) {
                index_of(&req, votes(a, true))
            } else {
                fallback(&req)
            };
            e.choose(i);
        }
        let r = e.referendum().expect("the referendum");
        let granted = r
            .records
            .iter()
            .find(|v| v.source == zvtes::referendum::Source::Granted(SeatId(1)))
            .expect("the granted votes");
        assert_eq!((granted.n, granted.favor), (2, false));
        if more {
            e.state.minions[s.index()].made_red_list = true;
            assert_eq!(running(&e).1, 4);
        }
        // Prince 2 + the card's 1 against 2: pass with the card's vote;
        // against 4, it fails.
        pick_matching(&mut e, |o| *o == Choice::CastCardVote { favor: true });
        close_polling(&mut e);
        request(&mut e);
        if more {
            assert!(!e.state.is_red_list(ally));
            assert!(e.state.seats[0]
                .ash
                .contains(&find_library("Trumped-Up Charges")));
        } else {
            placed_on(&e, "Trumped-Up Charges", Some(ally));
            assert!(e.state.is_red_list(ally));
        }
    }
}

/// Trumped-Up Charges' granted burn: a titled vampire, any Methuselah's,
/// calls it as a +1 stealth political action (LSJ 20080604: an action
/// requiring a title); passed, the card burns and its minion is no longer
/// Red List. Not offered to an untitled vampire.
#[test]
fn trumped_up_charges_burned_lifts_the_red_list() {
    let mut e = game(2);
    let u = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 2), 4, false);
    let a = titled_caller(&mut e, Title::Prince("Paris"));
    let t = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 1), 4, false);
    let card = in_play(&mut e, 1, "Trumped-Up Charges", Some(t));
    assert!(e.state.is_red_list(t));
    let req = request(&mut e);
    assert!(req.options.contains(&granted(card.0, a)));
    assert!(!req.options.contains(&granted(card.0, u)), "untitled");
    e.choose(index_of(&req, granted(card.0, a)));
    pick_matching(&mut e, |o| *o == votes(a, true));
    close_polling(&mut e);
    request(&mut e);
    assert!(!e.state.cards[card.index()].in_play);
    assert!(!e.state.is_red_list(t));
}

/// Priority Shift on a minion Red List by Trumped-Up Charges: no longer Red
/// List, the card burned ("burn this card if the minion is no longer Red
/// List"); on a printed Red List vampire, lifted for good. Not played by a
/// Red List or untitled vampire.
#[test]
fn priority_shift_lifts_the_red_list_and_burns_trumped_up_charges() {
    let red = textless(|c| c.red_list && (4..=8).contains(&c.capacity), 0);
    let mut e = game(2);
    let u = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 2), 4, false);
    let a = titled_caller(&mut e, Title::Prince("Paris"));
    let t = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    let p = e.add_minion(SeatId(1), red, 4, false);
    let card = in_play(&mut e, 1, "Trumped-Up Charges", Some(t));
    hand(&mut e, 0, &["Priority Shift", "Priority Shift"]);
    let shift = |actor, at| Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(actor),
        target: Target::Minion(at),
    };
    let req = request(&mut e);
    assert!(!req.options.contains(&shift(u, t)), "untitled");
    assert!(req.options.contains(&shift(a, p)), "printed Red List");
    e.choose(index_of(&req, shift(a, t)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.is_red_list(t));
    assert!(!e.state.cards[card.index()].in_play, "burned");
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Trumped-Up Charges")));
    assert!(e.state.is_red_list(p));
    let b = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 3), 4, false);
    e.state.minions[b.index()].granted_title = Some(Title::Prince("Berlin"));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.choose(index_of(&req, shift(b, p)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.is_red_list(p), "a printed Red List lifted");
}

/// Keystone Kine [obf] burning a Red List ally (made so: Public Enemy's
/// "minion", an ally too): "counts as a Ⓓ action burning the ally (eg.
/// for trophies)", so its vampire's controller may award a trophy. [LSJ
/// 20080608]
#[test]
fn keystone_kine_burning_a_red_list_ally_earns_a_trophy() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 3..=8, 0), 3, false);
    e.state.minions[a.index()].gained_levels = vec!["obf", "dom"];
    e.add_minion(SeatId(0), of_sect(Sect::Anarch, 3..=8, 1), 4, false);
    e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Carlton Van Wyk"));
    e.state.minions[ally.index()].made_red_list = true;
    hand(
        &mut e,
        0,
        &["Keystone Kine", "Conditioning", "Trophy: Chosen"],
    );
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 2,
            actor: Some(a),
            target: Target::None,
        },
    );
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::ChooseMinion(ally));
    let trophy = Choice::TakeFrom {
        zone: zvtes::decision::Zone::Hand,
        card: find_library("Trophy: Chosen"),
    };
    let req = request(&mut e);
    assert!(!e.state.minions[ally.index()].in_play);
    assert_eq!(req.seat, SeatId(0));
    assert!(req.options.contains(&trophy), "{req:?}");
}

// ---- Allies, retainers and equipment (misc) -------------------------------

/// Seat 0's A and a second vampire, seat 1's X and a second one: textless
/// vampires of capacity 4 to 8, 4 blood each, with these levels gained.
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

/// A bleeds, X blocks: their combat begins.
fn bleed_blocked(e: &mut Engine, a: MinionId, x: MinionId) {
    pick(e, bleed(a));
    pick_when(e, block(x));
}

/// `name` in play on `host`, for its controller, with its first mode's
/// layers.
fn equipped(e: &mut Engine, name: &str, host: MinionId) -> CardId {
    let seat = e.state.minions[host.index()].owner.0;
    in_play_mode(e, seat, name, Some(host), 0)
}

/// The bearer's (or the ally's) use of ability `ability` of card `w`.
fn use_card(w: CardId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(w),
        ability,
        actor: None,
        target: Target::None,
    }
}

/// The card of ally `m`.
fn ally_card(e: &Engine, m: MinionId) -> CardId {
    let i = e
        .state
        .cards
        .iter()
        .position(|c| c.body == Some(m) && c.present())
        .expect("the ally's card");
    CardId(i as u32)
}

/// Hand card `card` played by `actor` in mode 0 at minion `t`.
fn play_at(card: u8, actor: MinionId, t: MinionId) -> Choice {
    Choice::Play {
        card,
        mode: 0,
        actor: Some(actor),
        target: Target::Minion(t),
    }
}

/// +1 stealth on `m`'s actions (a stand-in card's layer).
fn stealthy(e: &mut Engine, m: MinionId) {
    bonus(e, m, zvtes::cards::Stat::Stealth);
}

/// +1 of `stat` on `m` (a stand-in card's layer).
fn bonus(e: &mut Engine, m: MinionId, stat: zvtes::cards::Stat) {
    let seat = e.state.minions[m.index()].owner.0;
    let id = in_play(e, seat, "Leather Jacket", Some(m));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::Bearer {
            stat,
            n: 1,
            during: &[],
            who: zvtes::cards::Bearer::Any,
        },
        source: id,
    });
}

/// Submachine Gun: "3R damage each strike, with an optional maneuver each
/// combat": its strike deals 3 at close range too.
#[test]
fn submachine_gun_strikes_for_3r() {
    let (mut e, a, x) = duel(&[], &[]);
    let gun = equipped(&mut e, "Submachine Gun", a);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, use_card(gun, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 3);
}

/// D'habi Revenant: "This Baali gets +1 bleed": a Baali bearer bleeds for
/// 2, another clan's bearer for 1 (read live, rules-qa 9.1); only a Baali
/// may employ it (its clan requirement).
#[test]
fn dhabi_revenant_gives_a_baali_bleed() {
    for (clan, bled) in [("Baali", 2), ("Brujah", 1)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        // Every Baali vampire has a definition: a Brujah made Baali.
        if clan == "Baali" {
            e.state.minions[a.index()].clan_change = Some("Baali");
        }
        e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        in_play(&mut e, 0, "D'habi Revenant", Some(a));
        hand(&mut e, 0, &["D'habi Revenant"]);
        let req = request(&mut e);
        assert_eq!(
            req.options.contains(&play(0, 0, a)),
            clan == "Baali",
            "{clan}"
        );
        pick(&mut e, bleed(a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10 - bled, "{clan}");
    }
}

/// Hellhound: "cannot take actions" (none offered, a block still is);
/// "If the Hellhound has only 1 life during your unlock phase, it gains 1
/// life": 1 → 2, 2 stays 2.
#[test]
fn hellhound_takes_no_action_blocks_and_heals_from_one_life() {
    for (life, after) in [(1, 2), (2, 2)] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let h = e.add_ally(SeatId(1), find_library("Hellhound"));
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[h.index()].blood = life;
        let req = to_minion_phase(&mut e, SeatId(1));
        assert_eq!(blood(&e, h), after);
        assert!(
            !req.options.iter().any(|o| matches!(o,
                Choice::TakeAction { minion, .. } if *minion == h)),
            "{:?}",
            req.options
        );
    }
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let h = e.add_ally(SeatId(1), find_library("Hellhound"));
    pick(&mut e, bleed(a));
    offering(&mut e, block(h));
}

/// Harzomatuili: recruited by a ready cardinal or regent only; once during
/// a directed action he burns 1 life for +1 intercept (Member of the
/// Entourage's shape, any directed action: rules-qa 13.2), not during an
/// undirected one; his hand strikes are aggravated; "fewer than 4 life
/// during your unlock phase": 3 → 4.
#[test]
fn harzomatuili_needs_a_cardinal_or_regent_and_burns_life_for_intercept() {
    for (title, offered) in [(Title::None, false), (Title::Cardinal, true)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
        e.state.minions[a.index()].granted_title = (title != Title::None).then_some(title);
        e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 1), 4, false);
        hand(&mut e, 0, &["Harzomatuili"]);
        let req = request(&mut e);
        assert_eq!(req.options.contains(&play(0, 0, a)), offered, "{title:?}");
    }
    // A bleed at his controller: +1 intercept once, for a life.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    stealthy(&mut e, a);
    let h = e.add_ally(SeatId(1), find_library("Harzomatuili"));
    let card = ally_card(&e, h);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(h));
    pick_when(&mut e, use_card(card, 1));
    assert_eq!(blood(&e, h), 3);
    let req = request(&mut e);
    assert!(!req.options.contains(&use_card(card, 1)), "once");
    // Undirected (a hunt): not offered.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    stealthy(&mut e, a);
    let h = e.add_ally(SeatId(1), find_library("Harzomatuili"));
    let card = ally_card(&e, h);
    pick(&mut e, action(a, zvtes::decision::ActionKind::Hunt));
    pick_when(&mut e, block(h));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_card(card, 1),
    );
    // Aggravated hand strikes; unlock-phase regeneration.
    let (mut e, a, _) = duel(&[], &[]);
    let h = e.add_ally(SeatId(1), find_library("Harzomatuili"));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(h));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        e.state.minions[a.index()].in_torpor,
        "2 aggravated: none left"
    );
    assert_eq!(blood(&e, h), 4 - 1);
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(blood(&e, h), 4);
}

/// The Textbook Damnation: its vampire has superior Daimoinon, is
/// infernal, pays 2 less blood for [dai] cards; an ally bearer gets none
/// of it (rules-digest D16). Its vampire locked as its controller's master
/// phase begins burns it.
#[test]
fn textbook_damnation_makes_its_vampire_infernal_and_burns_when_locked() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let book = equipped(&mut e, "Textbook Damnation, The", a);
    assert!(e.state.has_discipline(a, "DAI"));
    assert!(e.state.is_infernal(a));
    let ally = e.add_ally(SeatId(0), find_library("Hellhound"));
    let other = equipped(&mut e, "Textbook Damnation, The", ally);
    assert!(!e.state.is_infernal(ally) && !e.state.has_discipline(ally, "dai"));
    e.state.cards[other.index()].in_play = false;
    // Infernal: unlocked for a pool as the turn begins, it stays; left
    // locked into the master phase, it burns.
    e.state.minions[a.index()].locked = true;
    pick_when(&mut e, Choice::UnlockInfernal { minion: a });
    to_minion_phase(&mut e, SeatId(0));
    assert!(e.state.cards[book.index()].in_play, "unlocked");
    e.state.minions[a.index()].locked = true;
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(0));
    assert!(e.state.minions[a.index()].locked);
    assert!(
        !e.state.cards[book.index()].in_play,
        "locked in its master phase"
    );
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Textbook Damnation, The")));
}

/// The Textbook Damnation's burn is ordered by its Methuselah among the
/// master phase's effects (LSJ 20100421; LSJ 20050412): its vampire locked
/// after the phase began, the phase cannot be passed with the Textbook
/// kept; locked as the phase began and unlocked in it before the burn
/// resolves, it keeps the Textbook.
#[test]
fn textbook_damnation_burns_with_its_vampire_locked_in_the_master_phase() {
    let setup = |locked: bool| {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let book = equipped(&mut e, "Textbook Damnation, The", a);
        e.state.minions[a.index()].locked = locked;
        hand(&mut e, 0, &["Dreams of the Sphinx"]);
        e.at_master_phase();
        (e, a, book)
    };
    // Locked after the phase began: no pass until the burn resolves.
    let (mut e, a, book) = setup(false);
    assert!(request(&mut e).options.contains(&Choice::Pass));
    e.state.minions[a.index()].locked = true;
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::Pass));
    pick(&mut e, Choice::ResolveRider { card: book.0 });
    assert!(!e.state.cards[book.index()].in_play);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Textbook Damnation, The")));
    // Locked as it began, unlocked before the burn: kept.
    let (mut e, a, book) = setup(true);
    assert!(request(&mut e)
        .options
        .contains(&Choice::ResolveRider { card: book.0 }));
    e.state.minions[a.index()].locked = false;
    pick(&mut e, Choice::Pass);
    to_minion_phase(&mut e, SeatId(0));
    assert!(e.state.cards[book.index()].in_play);
}

/// The Textbook Damnation on Herald of Topheth (an ally playing [dai] cards
/// as a vampire): no -2 blood on its [dai] cards (LSJ 20101230: "Herald is
/// not 'this vampire'"; rules-qa 121.179); a vampire bearer's [dai]
/// Conflagration costs 2 less.
#[test]
fn textbook_damnation_gives_herald_of_topheth_nothing() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let h = e.add_ally(SeatId(0), find_library("Herald of Topheth"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    equipped(&mut e, "Textbook Damnation, The", a);
    equipped(&mut e, "Textbook Damnation, The", h);
    let conflagration = find_library("Conflagration");
    assert_eq!(e.state.printed_cost_off(a, conflagration, 1).blood, 2);
    assert_eq!(e.state.printed_cost_off(h, conflagration, 1).blood, 0);
}

/// The mode of `name` carrying a `SectBound` layer.
fn sect_bound_mode(name: &str) -> u8 {
    let def = zvtes::cards::lookup(find_library(name)).expect("definition");
    def.modes
        .iter()
        .position(|m| {
            m.effects.iter().any(|e| {
                matches!(
                    e,
                    zvtes::cards::Effect::Layer(zvtes::cards::Modifier::SectBound(_))
                )
            })
        })
        .expect("a sect-bound mode") as u8
}

/// Writ of Acceptance: "considered a Camarilla vampire", not optional (RTR
/// 19980707); an override over the underlying sect, the most recent taking
/// precedence, staying over an actual change (LSJ 20100811, ANK 20190619):
/// a vampire made Anarch, then under a Writ, is Camarilla; a Derange
/// arriving later wins (its actor's sect); the Derange gone, the Writ
/// again; the Writ gone, Anarch. An ally bearer is no vampire: nothing
/// (rules-digest D16).
// Rulings: [RTR 19980707] [LSJ 20100811] [ANK 20190619].
#[test]
fn writ_of_acceptance_makes_its_vampire_camarilla_over_a_sect_change() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 1), 4, false);
    e.state.minions[v.index()].sect_override = Some(Sect::Anarch);
    let writ = equipped(&mut e, "Writ of Acceptance", v);
    assert_eq!(e.state.sect_of(v), Some(Sect::Camarilla));
    // A Derange played later (by a Sabbat vampire): its sect wins.
    let d = in_play(&mut e, 1, "Derange", Some(v));
    e.state.minions[v.index()].deranged = Some(("Malkavian antitribu", Sect::Sabbat, d));
    e.state.minions[v.index()]
        .considered
        .0
        .push((Sect::Camarilla, writ));
    e.state.minions[v.index()]
        .considered
        .0
        .push((Sect::Sabbat, d));
    assert_eq!(e.state.sect_of(v), Some(Sect::Sabbat));
    e.state.cards[d.index()].in_play = false;
    e.state.minions[v.index()].deranged = None;
    assert_eq!(e.state.sect_of(v), Some(Sect::Camarilla), "the Writ again");
    e.state.cards[writ.index()].in_play = false;
    assert_eq!(e.state.sect_of(v), Some(Sect::Anarch), "the actual sect");
    let ally = e.add_ally(SeatId(0), find_library("Hellhound"));
    equipped(&mut e, "Writ of Acceptance", ally);
    assert_eq!(e.state.sect_of(ally), None);
}

/// "A temporary effect overriding the sect has precedence and if it does,
/// this card burns (eg. Writ of Acceptance)" [LSJ 20100811]: Field
/// Training, The Red Question, Galaric's Legacy and Go Anarch on an Anarch
/// burn as a Writ arrives on it; and "If the target changes sect to a sect
/// inappropriate for his title, he loses its benefit until his sect is
/// appropriate" [LSJ 20060904]: a Sabbat bishop under a Writ has no title.
#[test]
fn writ_of_acceptance_burns_the_sect_cards_and_idles_a_sabbat_title() {
    for name in [
        "Field Training",
        "Red Question, The",
        "Galaric's Legacy",
        "Go Anarch",
    ] {
        let mut e = game(2);
        let v = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 4..=8, 0), 4, false);
        e.add_minion(SeatId(0), of_sect(Sect::Anarch, 4..=8, 1), 4, false);
        e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
        let card = in_play_mode(&mut e, 0, name, Some(v), sect_bound_mode(name));
        request(&mut e);
        assert!(e.state.cards[card.index()].in_play, "{name}: kept");
        equipped(&mut e, "Writ of Acceptance", v);
        request(&mut e);
        assert!(!e.state.cards[card.index()].in_play, "{name}: burned");
    }
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    e.state.minions[v.index()].granted_title = Some(Title::Bishop);
    assert_eq!(e.state.title_of(v), Title::Bishop);
    let writ = equipped(&mut e, "Writ of Acceptance", v);
    assert_eq!(e.state.title_of(v), Title::None, "idle");
    e.state.cards[writ.index()].in_play = false;
    assert_eq!(e.state.title_of(v), Title::Bishop, "back");
}

/// A Ravnos under a Writ of Acceptance is only considered Camarilla: Ravnos
/// Acceptance's join, an actual sect change outliving the Writ, is still
/// offered it (rules-qa 111.32; LSJ 20100811).
#[test]
fn ravnos_acceptance_still_offered_to_a_ravnos_under_a_writ() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    let r = e.add_minion(SeatId(0), of_clan("Ravnos", 3..=8, 0), 4, false);
    e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 1), 4, false);
    let writ = equipped(&mut e, "Writ of Acceptance", r);
    assert_eq!(e.state.sect_of(r), Some(Sect::Camarilla));
    hand(&mut e, 0, &["Ravnos Acceptance"]);
    pass_referendum(&mut e, a, &[]);
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    assert_eq!(req.options, [Choice::Pass, Choice::ChooseMinion(r)]);
    e.choose(1);
    request(&mut e);
    e.state.cards[writ.index()].in_play = false;
    assert_eq!(e.state.sect_of(r), Some(Sect::Camarilla), "for good");
}

/// Seat 0's A (4 blood) bleeds and seat 1's ally `name` (with `life`,
/// `cards` in seat 1's hand) blocks.
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

/// Samuel Haight: "Once each action, Samuel may burn a life to get +1
/// bleed", during his bleed before resolution (ANK 20231124, LSJ
/// 19970718): a life for a bleed of 1, not twice; no bleed card bleeds
/// more. "If Samuel has less than 4 life during your unlock phase, he
/// gains 1 life."
// Rulings: [ANK 20231124] [LSJ 19970718].
#[test]
fn samuel_haight_burns_life_for_bleed_once_each_action() {
    let mut e = game(2);
    let s = e.add_ally(SeatId(0), find_library("Samuel Haight"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = ally_card(&e, s);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&use_card(card, 0)),
        "not before acting"
    );
    pick(&mut e, bleed(s));
    pick_when(&mut e, use_card(card, 0));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_card(card, 0),
    );
    assert_eq!(e.state.seats[1].pool, 10 - 1);
    assert_eq!(blood(&e, s), 3);
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, s), 4, "regenerated");
}

/// Samuel Haight: "He may steal equipment as a strike": the opposing
/// vampire's weapon moves to him, at close range; against an unequipped
/// minion the strike is not offered (RTR 19980928).
// Rulings: [RTR 19980928].
#[test]
fn samuel_haight_steals_equipment_as_a_strike() {
    let (mut e, a, s) = ally_blocks("Samuel Haight", 4, &[]);
    let card = ally_card(&e, s);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o,
            Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), ability: 1, .. }
                if *c == card)
    });
    assert!(e.state.minions[a.index()].in_play);
    let (mut e, a2, s) = ally_blocks("Samuel Haight", 4, &[]);
    let card = ally_card(&e, s);
    let magnum = in_play(&mut e, 0, ".44 Magnum", Some(a2));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(card),
            ability: 1,
            actor: None,
            target: Target::Card(magnum),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[magnum.index()].host, Some(s), "stolen");
}

/// Samuel Haight: "Any vampire may call a referendum to make Samuel become
/// Red List as a +1 stealth political action": another Methuselah's prince
/// calls it on their turn; passed, Samuel is Red List.
#[test]
fn samuel_haight_referendum_puts_him_on_the_red_list() {
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Prince("Paris"));
    let s = e.add_ally(SeatId(1), find_library("Samuel Haight"));
    e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    let card = ally_card(&e, s);
    assert!(!e.state.is_red_list(s));
    pick(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(card),
            ability: 2,
            actor: Some(a),
            target: Target::None,
        },
    );
    pick_when(&mut e, votes(a, true));
    close_polling(&mut e);
    request(&mut e);
    assert!(e.state.is_red_list(s));
}

/// Samuel Haight plays cards requiring basic Blood Sorcery as a vampire:
/// Burst of Sunlight [tha] offered, its [THA] not (ANK 20180111); he does
/// not have the Discipline (LSJ 19980303); its aggravated damage to him is
/// a life lost, no torpor (LSJ 20060409).
// Rulings: [ANK 20180111] [LSJ 19980303] [LSJ 20060409].
#[test]
fn samuel_haight_plays_basic_blood_sorcery_taking_its_damage_as_life() {
    let (mut e, a, s) = ally_blocks("Samuel Haight", 4, &["Burst of Sunlight"]);
    assert!(!e.state.has_discipline(s, "tha"));
    let req = offering(&mut e, play(0, 0, s));
    assert!(!req.options.contains(&play(0, 1, s)), "basic only");
    e.choose(index_of(&req, play(0, 0, s)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].in_torpor);
    let v = &e.state.minions[s.index()];
    assert!(v.in_play && !v.in_torpor);
    assert_eq!(v.blood, 4 - 1 - 1);
}

/// Spiritual Protector on A: the vampire blocking A cannot use its
/// equipment (its Submachine Gun's strike) in the resulting combat; with
/// no block (A's Bum's Rush on it) the gun is used (TOM 19951208-2).
// Rulings: [TOM 19951208-2].
#[test]
fn spiritual_protector_bars_the_blockers_equipment() {
    for blocked in [true, false] {
        let (mut e, a, x) = duel(&["pot"], &[]);
        let sp = equipped(&mut e, "Spiritual Protector", a);
        e.state.cards[sp.index()].counters = 1;
        let gun = equipped(&mut e, "Submachine Gun", x);
        if blocked {
            bleed_blocked(&mut e, a, x);
            never_offered_until(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                use_card(gun, 0),
            );
            assert!(e.state.minions[x.index()].in_play);
        } else {
            hand(&mut e, 0, &["Bum's Rush"]);
            pick(&mut e, play_at(0, a, x));
            pick_when(&mut e, use_card(gun, 0));
        }
    }
}

/// Spiritual Protector "is immune to non-aggravated damage": "damage is not
/// inflicted, even if non-preventable or outside combat" (LSJ 20010627,
/// LSJ 20010629, LSJ 20090304-1): Target Retainer's normal strike leaves
/// its one life; an aggravated one burns it.
// Rulings: [LSJ 20010627] [LSJ 20010629] [LSJ 20090304-1].
#[test]
fn spiritual_protector_takes_no_normal_damage() {
    for aggravated in [false, true] {
        let mut e = game(2);
        let g = e.add_minion(SeatId(0), plain("Giovanni", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let sp = equipped(&mut e, "Spiritual Protector", g);
        e.state.cards[sp.index()].counters = 1;
        let a = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        e.state.minions[g.index()].locked = false;
        if aggravated {
            e.state.minions[a.index()].timed.push((
                zvtes::cards::Lasting::AggravatedHands,
                zvtes::state::Expiry::EndOfTurn,
                SeatId(1),
            ));
        }
        hand(&mut e, 1, &["Target Retainer"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(g));
        pick_when(
            &mut e,
            Choice::Play {
                card: 0,
                mode: 0,
                actor: Some(a),
                target: Target::Card(sp),
            },
        );
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.cards[sp.index()].present(),
            !aggravated,
            "{aggravated}"
        );
    }
}

/// Orb of Ulain: "This ally cannot be the target of directed actions
/// requiring … Dominate [dom], Presence [pre] …": Far Mastery's and
/// Entrancement's superior modes are not offered at its ally, offered at
/// another ally.
#[test]
fn orb_of_ulain_shields_its_ally_from_directed_dominate_and_presence() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), vampire_with("DOM", 0), 4, false);
    let w = e.add_minion(SeatId(0), vampire_with("PRE", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    let x = e.add_ally(SeatId(1), find_library("Political Ally"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    equipped(&mut e, "Orb of Ulain", ally);
    hand(&mut e, 0, &["Far Mastery", "Entrancement"]);
    let req = request(&mut e);
    let at = |card: u8, by: MinionId, t: MinionId| Choice::Play {
        card,
        mode: 1,
        actor: Some(by),
        target: Target::Minion(t),
    };
    assert!(!req.options.contains(&at(0, v, ally)) && !req.options.contains(&at(1, w, ally)));
    assert!(req.options.contains(&at(0, v, x)), "{:?}", req.options);
    assert!(req.options.contains(&at(1, w, x)), "{:?}", req.options);
}

/// Orb of Ulain: "Reaction cards requiring any of those Disciplines cost 1
/// additional blood while this ally is acting": X's Eagle's Sight [aus]
/// against the Orb's ally costs a blood; against a vampire bearing it,
/// nothing.
#[test]
fn orb_of_ulain_taxes_reactions_while_its_ally_acts() {
    let mut left = Vec::new();
    for ally in [true, false] {
        let mut e = game(2);
        let a = if ally {
            e.add_ally(SeatId(0), find_library("Political Ally"))
        } else {
            e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false)
        };
        e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        e.state.minions[z.index()].gained_levels = vec!["aus"];
        e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        equipped(&mut e, "Orb of Ulain", a);
        stealthy(&mut e, a);
        hand(&mut e, 1, &["Eagle's Sight"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, 0, z));
        request(&mut e);
        left.push(blood(&e, z));
    }
    assert_eq!(left, [3, 4]);
}

/// Polaris Coach: "During your unlock phase, move 1 blood from this vampire
/// to the Polaris Coach or burn the Polaris Coach": the blood moves onto it
/// as a counter, its controller's choice; with no blood to move, it burns.
#[test]
fn polaris_coach_feeds_on_its_vampire_or_burns() {
    for (b, kept) in [(3, true), (0, false)] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let x = e.add_minion(SeatId(1), plain("Ventrue", 0), b, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let coach = equipped(&mut e, "Polaris Coach", x);
        if kept {
            pick_when(&mut e, Choice::BurnBloodToAvoid);
            assert_eq!(e.state.cards[coach.index()].counters, 1);
            assert_eq!(blood(&e, x), b - 1);
        } else {
            to_minion_phase(&mut e, SeatId(1));
            assert!(!e.state.cards[coach.index()].present(), "burned");
        }
    }
}

/// Polaris Coach: "While this vampire is acting, he or she may burn one
/// counter … to get +1 stealth": as often as counters and need allow
/// (rules-digest D7).
#[test]
fn polaris_coach_counters_buy_stealth() {
    let (mut e, a, x) = duel(&[], &[]);
    let coach = equipped(&mut e, "Polaris Coach", a);
    e.state.cards[coach.index()].counters = 2;
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, use_card(coach, 0));
    assert_eq!(e.state.cards[coach.index()].counters, 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "unblocked");
}

/// Polaris Coach: "During undirected actions and actions that are not
/// directed at this vampire, he or she cannot block or play reaction
/// cards": a bleed at its Methuselah is not blockable by it, nor its
/// reaction played; a directed action targeting it (Bum's Rush) is.
#[test]
fn polaris_coach_bars_blocks_and_reactions_off_its_own_target() {
    for coach in [true, false] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let x = e.add_minion(SeatId(1), vampire_with("dom", 0), 4, false);
        e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
        if coach {
            equipped(&mut e, "Polaris Coach", x);
        }
        hand(&mut e, 1, &["Deflection"]);
        pick(&mut e, bleed(a));
        let deflect = Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(x),
            target: Target::Seat(SeatId(2)),
        };
        if coach {
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
                *o == block(x) || *o == deflect
            });
        } else {
            offering(&mut e, deflect);
        }
    }
    let (mut e, a, x) = duel(&["pot"], &["aus"]);
    equipped(&mut e, "Polaris Coach", x);
    stealthy(&mut e, a);
    hand(&mut e, 0, &["Bum's Rush"]);
    hand(&mut e, 1, &["Eagle's Sight"]);
    pick(&mut e, play_at(0, a, x));
    pick_when(&mut e, block(x));
    offering(&mut e, play(0, 0, x));
}

/// Polaris Coach: "A minion may have only one haven and only one
/// vehicle": not equipped on a vampire holding Body Bag, nor on one
/// holding Sport Bike.
#[test]
fn polaris_coach_is_one_haven_and_one_vehicle() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let c = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    equipped(&mut e, "Body Bag", a);
    equipped(&mut e, "Sport Bike", c);
    hand(&mut e, 0, &["Polaris Coach"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, a)));
    assert!(!req.options.contains(&play(0, 0, c)));
    assert!(req.options.contains(&play(0, 0, b)));
}

/// Powder of Rigidity: "If the action to equip with the Powder is
/// successful, unlock the acting minion during the next discard phase": its
/// equipper, locked by the action, is unlocked before the next Methuselah's
/// turn.
#[test]
fn powder_of_rigidity_equip_unlocks_at_discard() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Powder of Rigidity"]);
    pick(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].locked);
    placed_on(&e, "Powder of Rigidity", Some(a));
    drop(req);
    to_minion_phase(&mut e, SeatId(1));
    assert!(!e.state.minions[a.index()].locked, "unlocked at discard");
}

/// Powder of Rigidity burned before range: the opposing minion cannot play
/// cards requiring Protean for the rest of the combat (X's Wolf Claws not
/// offered); a werewolf opponent (Samuel Haight) gets -2 strength, its hand
/// strike dealing 1.
#[test]
fn powder_of_rigidity_bars_protean_and_weakens_a_werewolf() {
    let (mut e, a, x) = duel(&[], &["pro"]);
    let powder = equipped(&mut e, "Powder of Rigidity", a);
    hand(&mut e, 1, &["Wolf Claws"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, use_card(powder, 0));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Powder of Rigidity")));
    let (mut e, a, x) = duel(&[], &["pro"]);
    hand(&mut e, 1, &["Wolf Claws"]);
    bleed_blocked(&mut e, a, x);
    offering(&mut e, play(0, 0, x));
    let (mut e, a, s) = ally_blocks("Samuel Haight", 4, &[]);
    let powder = equipped(&mut e, "Powder of Rigidity", a);
    pick_when(&mut e, use_card(powder, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1, "3 strength - 2");
    assert_eq!(blood(&e, s), 4 - 1);
}

/// Ghoul Escort: "When this vampire is blocked, he or she may burn this
/// retainer and unlock instead of entering combat. (This does not unlock
/// the blocker.)": the Escort burned, A unlocked, X still locked, no
/// combat, the bleed blocked.
#[test]
fn ghoul_escort_burns_to_escape_the_blocks_combat() {
    let (mut e, a, x) = duel(&[], &[]);
    let escort = equipped(&mut e, "Ghoul Escort", a);
    e.state.cards[escort.index()].counters = 4;
    e.notes = Some(Vec::new());
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, Choice::CancelCombat);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].ash.contains(&find_library("Ghoul Escort")));
    assert!(!e.state.minions[a.index()].locked, "unlocked");
    assert!(
        e.state.minions[x.index()].locked,
        "the blocker stays locked"
    );
    assert_eq!((blood(&e, a), blood(&e, x)), (4, 4), "no combat");
    assert_eq!(e.state.seats[1].pool, 10, "blocked");
}

/// Ghoul Escort: "Cannot be used if blocked when leaving torpor, as no
/// combat would happen." [ANK 20181122-1]
#[test]
fn ghoul_escort_not_offered_when_leaving_torpor_is_blocked() {
    let (mut e, a, x) = duel(&[], &[]);
    e.state.minions[a.index()].in_torpor = true;
    let escort = equipped(&mut e, "Ghoul Escort", a);
    e.state.cards[escort.index()].counters = 4;
    pick(&mut e, action(a, zvtes::decision::ActionKind::LeaveTorpor));
    pick_when(&mut e, block(x));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::CancelCombat,
    );
    assert!(e.state.cards[escort.index()].present());
}

/// Caiaphas Smith: "Any vampire blocking Caiaphas is burned after the
/// combat (if any)": X blocks his equip action and, the combat over, is
/// burned to its controller's ash heap; an ally blocking him is not.
/// "Caiaphas cannot bleed": no bleed offered him.
#[test]
fn caiaphas_burns_the_vampire_blocking_him_and_cannot_bleed() {
    let mut e = game(2);
    let c = e.add_ally(SeatId(0), find_library("Caiaphas Smith"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = e.state.minions[x.index()].crypt_card().expect("crypt card");
    let req = request(&mut e);
    assert!(!req.options.contains(&bleed(c)));
    bonus(&mut e, x, zvtes::cards::Stat::Intercept);
    hand(&mut e, 0, &["Leather Jacket"]);
    e.notes = Some(Vec::new());
    pick(&mut e, play(0, 0, c));
    pick_when(&mut e, block(x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!noted_combats(&mut e).is_empty(), "a combat");
    assert!(!e.state.minions[x.index()].in_play, "burned");
    assert!(e.state.seats[1].ash_crypt.contains(&card));
    let mut e = game(2);
    let c = e.add_ally(SeatId(0), find_library("Caiaphas Smith"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Hellhound"));
    e.state.minions[ally.index()].blood = 4;
    bonus(&mut e, ally, zvtes::cards::Stat::Intercept);
    hand(&mut e, 0, &["Leather Jacket"]);
    pick(&mut e, play(0, 0, c));
    pick_when(&mut e, block(ally));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[ally.index()].in_play, "an ally: not burned");
}

/// Caiaphas Smith: "If he is unlocked at the start of your turn, your
/// predator takes control of him", read before anything unlocks: left
/// unlocked, he passes to seat 0's predator (seat 2) for good; locked, he
/// stays.
#[test]
fn caiaphas_left_unlocked_goes_to_the_predator() {
    for locked in [false, true] {
        let mut e = game(3);
        let c = e.add_ally(SeatId(0), find_library("Caiaphas Smith"));
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
        to_minion_phase(&mut e, SeatId(1));
        e.state.minions[c.index()].locked = locked;
        to_minion_phase(&mut e, SeatId(0));
        let owner = e.state.minions[c.index()].owner;
        assert_eq!(
            owner,
            if locked { SeatId(0) } else { SeatId(2) },
            "{locked}"
        );
    }
}

/// Caiaphas Smith, a red reading ("(if any)", rules-qa Batch 125): X's
/// block stands but Ohoyo Hopoksia takes its combat (rules-qa 124.154): X
/// fought no combat and still burns, as the action ends.
#[test]
fn caiaphas_burns_a_blocker_whose_combat_never_happened() {
    let mut e = game(2);
    let c = e.add_ally(SeatId(0), find_library("Caiaphas Smith"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_ally(SeatId(1), find_library("Ohoyo Hopoksia (Bastet)"));
    bonus(&mut e, x, zvtes::cards::Stat::Intercept);
    hand(&mut e, 0, &["Leather Jacket"]);
    pick(&mut e, play(0, 0, c));
    pick_when(&mut e, block(x));
    let req = expect(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    let i = req
        .options
        .iter()
        .position(|o| matches!(o, Choice::ResolveRider { .. }))
        .expect("Ohoyo's stand-in");
    e.choose(i);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[x.index()].in_play, "burned");
    assert_eq!(blood(&e, x), 0);
}

/// Seat 0's Deviki Prasanta and a ready Banu Haqim (with `levels`), at
/// seat 0's next master phase; `ash` in its ash heap, `hand` in hand.
fn deviki(levels: &[&'static str], ash: &[&str], hand_: &[&str]) -> (Engine, MinionId, CardId) {
    let mut e = game(2);
    let d = e.add_ally(SeatId(0), find_library("Deviki Prasanta"));
    let h = e.add_minion(SeatId(0), of_clan("Banu Haqim", 3..=8, 0), 4, false);
    e.state.minions[h.index()].gained_levels = levels.to_vec();
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for c in ash {
        e.state.seats[0].ash.push(find_library(c));
    }
    hand(&mut e, 0, hand_);
    to_minion_phase(&mut e, SeatId(1));
    to_window(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let card = ally_card(&e, d);
    (e, h, card)
}

/// Deviki's use on Banu Haqim `h`.
fn deviki_on(card: CardId, h: MinionId) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(card),
        ability: 0,
        actor: None,
        target: Target::Minion(h),
    }
}

/// Deviki Prasanta: locked in her master phase, she puts a master
/// Discipline card from the ash heap (or library) on a ready Banu Haqim —
/// put, not played, it still gives its Discipline (Agent of Power: LSJ
/// 20071001-1, LSJ 20100422); the hand's copy is not offered.
// Rulings: [LSJ 20071001-1] [LSJ 20100422].
#[test]
fn deviki_puts_agent_of_power_from_the_ash_heap_on_a_banu_haqim() {
    let (mut e, h, card) = deviki(&[], &["Agent of Power"], &["Agent of Power"]);
    pick(&mut e, deviki_on(card, h));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::LibrarySearch);
    assert_eq!(
        req.options,
        [
            Choice::Pass,
            Choice::TakeFrom {
                zone: zvtes::decision::Zone::AshHeap,
                card: find_library("Agent of Power"),
            }
        ]
    );
    e.choose(1);
    pick_when(&mut e, Choice::ChooseDiscipline("pre"));
    placed_on(&e, "Agent of Power", Some(h));
    assert!(e.state.has_discipline(h, "pre"));
    let d = e.state.cards[card.index()].body.expect("Deviki");
    assert!(e.state.minions[d.index()].locked, "Deviki locked");
}

/// Deviki Prasanta: "Cannot put a Discipline card on a vampire if it could
/// normally not be put on him (eg. because he already has the superior
/// level of that discipline)" [ANK 20200320]: Presence not offered for a
/// Banu Haqim with superior Presence, offered for one with none.
#[test]
fn deviki_cannot_put_a_discipline_already_superior() {
    let presence = Choice::TakeFrom {
        zone: zvtes::decision::Zone::AshHeap,
        card: find_library("Presence"),
    };
    for (levels, offered) in [(&["pre", "pre"][..], false), (&[][..], true)] {
        let (mut e, h, card) = deviki(levels, &["Presence"], &[]);
        pick(&mut e, deviki_on(card, h));
        let req = request(&mut e);
        assert_eq!(req.options.contains(&presence), offered, "{levels:?}");
    }
}

/// Incriminating Videotape: "Choose a minion your prey controls. The
/// chosen minion cannot block the minion with this equipment." Chosen as
/// played, it never blocks its bearer, another of seat 1's does; moved to
/// another bearer, it bars blocking the new one and may block the old
/// (TOM 19960114, LSJ 20020514).
// Rulings: [TOM 19960114] [LSJ 20020514].
#[test]
fn incriminating_videotape_bars_the_chosen_minion_from_blocking() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Incriminating Videotape"]);
    pick(&mut e, play_at(0, a, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let tape = placed_on(&e, "Incriminating Videotape", Some(a));
    assert_eq!(e.state.cards[tape.index()].chosen, [x]);
    assert!(e.state.cannot_block(x, a, true) && !e.state.cannot_block(y, a, true));
    e.state.cards[tape.index()].host = Some(b);
    assert!(!e.state.cannot_block(x, a, true) && e.state.cannot_block(x, b, true));
    let mut e2 = game(2);
    let a2 = e2.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e2.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x2 = e2.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let y2 = e2.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let tape2 = equipped(&mut e2, "Incriminating Videotape", a2);
    e2.state.cards[tape2.index()].chosen = vec![x2];
    pick(&mut e2, bleed(a2));
    let req = expect(&mut e2, SeatId(1), WindowLabel::ActionBlock);
    assert!(!req.options.contains(&block(x2)) && req.options.contains(&block(y2)));
}

/// Incriminating Videotape: "If it is not equipped but merely put into
/// play (eg. Alastor), no minion is chosen. One is if it is equipped (eg.
/// Magic of the Smith)." [LSJ 20090506] Found by Vast Wealth's equip, its
/// Methuselah chooses one of the prey's minions.
#[test]
fn incriminating_videotape_chosen_when_equipped_by_a_search() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let wealth = equipped(&mut e, "Vast Wealth", a);
    fill_hand(&mut e, 0);
    e.state.seats[0]
        .library
        .push(find_library("Incriminating Videotape"));
    pick(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(wealth),
            ability: 0,
            actor: Some(a),
            target: Target::None,
        },
    );
    let req = offering(&mut e, Choice::ChooseMinion(y));
    assert!(req.options.contains(&Choice::ChooseMinion(x)));
    e.choose(index_of(&req, Choice::ChooseMinion(y)));
    request(&mut e);
    let tape = placed_on(&e, "Incriminating Videotape", Some(a));
    assert_eq!(e.state.cards[tape.index()].chosen, [y]);
}

/// Bloodstone: "Put this equipment on any minion (this is a +1 stealth Ⓓ
/// action if the minion is controlled by another Methuselah)": on seat 1's
/// X, the equip is directed at seat 1, still an equip action (ANK
/// 20230408, LSJ 20050313); the card enters play on X. On A's own other
/// minion, undirected.
// Rulings: [ANK 20230408] [LSJ 20050313].
#[test]
fn bloodstone_goes_on_another_methuselahs_minion_directed() {
    for other in [true, false] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Bloodstone"]);
        let t = if other { x } else { b };
        pick(&mut e, play_at(0, a, t));
        request(&mut e);
        let act = e.action().expect("the equip");
        assert_eq!(act.kind, zvtes::decision::ActionKind::Equip);
        assert_eq!(act.directed_seat(&e.state), other.then_some(SeatId(1)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let stone = placed_on(&e, "Bloodstone", Some(t));
        assert_eq!(e.state.cards[stone.index()].by, Some(a));
    }
}

/// Bloodstone: "The option to place the card on another minion can only be
/// used when playing the card normally; for any other action or put into
/// play effect, the card is placed on the playing minion." [LSJ 20090824]
/// [LSJ 20051121] Found by Vast Wealth's equip, it goes on the acting
/// vampire.
#[test]
fn bloodstone_found_by_a_search_stays_on_the_actor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let wealth = equipped(&mut e, "Vast Wealth", a);
    fill_hand(&mut e, 0);
    e.state.seats[0].library.push(find_library("Bloodstone"));
    pick(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(wealth),
            ability: 0,
            actor: Some(a),
            target: Target::None,
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Bloodstone", Some(a));
}

/// Bloodstone: "This acting vampire gets +2 intercept when attempting to
/// block the bearer": A, which put it on X, reaches X's +2 stealth bleed;
/// an ally that put it there gets nothing ("vampire": rules-digest D16).
#[test]
fn bloodstone_gives_its_equipper_intercept_against_the_bearer() {
    for vampire in [true, false] {
        let mut e = game(2);
        let a = if vampire {
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false)
        } else {
            e.add_ally(SeatId(0), find_library("Political Ally"))
        };
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let stone = equipped(&mut e, "Bloodstone", x);
        e.state.cards[stone.index()].by = Some(a);
        stealthy(&mut e, x);
        stealthy(&mut e, x);
        to_minion_phase(&mut e, SeatId(1));
        pick(&mut e, bleed(x));
        let req = request(&mut e);
        assert_eq!(req.window, WindowLabel::ActionBlock);
        assert_eq!(e.block_can_reach(SeatId(0), a), vampire, "{vampire}");
    }
}

/// "Ghouls are monsters, not mortal." [ANK 20200203-2] [LSJ 20060515]
/// Deviki Prasanta and the Hellhound are no mortal allies (Caiaphas Smith,
/// a mortal, is); D'habi Revenant and Ghoul Escort are ghoul retainers, no
/// mortal ones.
#[test]
fn round_33_ghouls_are_no_mortals() {
    let mut e = game(2);
    for (name, mortal) in [
        ("Deviki Prasanta", false),
        ("Hellhound", false),
        ("Caiaphas Smith", true),
    ] {
        let m = e.add_ally(SeatId(0), find_library(name));
        assert_eq!(e.state.is_mortal(m), mortal, "{name}");
    }
    use zvtes::library::{Label, LIBRARY};
    for name in ["D'habi Revenant", "Ghoul Escort"] {
        let labels = LIBRARY[find_library(name) as usize].labels;
        assert!(
            labels.contains(&Label::Ghoul) && !labels.contains(&Label::Mortal),
            "{name}"
        );
    }
}

/// .44 Magnum: "Provides only one maneuver each combat, even if the bearer
/// changes." [LSJ 19980302-2] A uses its maneuver against Samuel Haight;
/// the gun then moving to Samuel (as his steal strike moves it), its
/// maneuver is never offered him in that combat.
#[test]
fn a_guns_maneuver_used_stays_used_when_the_gun_changes_bearer() {
    let (mut e, a, s) = ally_blocks("Samuel Haight", 4, &[]);
    let magnum = equipped(&mut e, ".44 Magnum", a);
    pick_when(&mut e, use_card(magnum, 1));
    e.state.cards[magnum.index()].host = Some(s);
    e.state.cards[magnum.index()].controller = SeatId(1);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_card(magnum, 1),
    );
}

/// Incriminating Videotape put in play by Alastor: "no minion is chosen"
/// [LSJ 20090506]; it bars no one.
#[test]
fn incriminating_videotape_put_by_alastor_chooses_no_one() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    e.state.minions[j.index()].granted_title = Some(Title::Justicar("Ventrue"));
    let other = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Alastor"]);
    fill_hand(&mut e, 0);
    for c in ["Form of Mist", "Incriminating Videotape", "Form of Mist"] {
        e.state.seats[0].library.push(find_library(c));
    }
    pick(&mut e, play(0, 0, j));
    pick_matching(&mut e, |o| *o == Choice::ChooseMinion(other));
    pick_matching(&mut e, |o| {
        matches!(
            o,
            Choice::CastVotes { favor: true, .. } | Choice::CastCardVote { favor: true }
        )
    });
    close_polling(&mut e);
    pick_when(
        &mut e,
        Choice::FindCard(find_library("Incriminating Videotape")),
    );
    request(&mut e);
    let tape = placed_on(&e, "Incriminating Videotape", Some(other));
    assert!(e.state.cards[tape.index()].chosen.is_empty());
    assert!(!e.state.cannot_block(x, other, true));
}

// ---- Master cards -------------------------------------------------------------

/// A master card in hand slot `card` played on `target`.
fn master_at(card: u8, target: Target) -> Choice {
    Choice::Play {
        card,
        mode: 0,
        actor: None,
        target,
    }
}

/// The ability `ability` of the card in play `source`, used by `actor` on
/// `target`.
fn use_on(source: CardId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(source),
        ability,
        actor,
        target,
    }
}

fn ash_has(e: &Engine, seat: usize, name: &str) -> bool {
    e.state.seats[seat].ash.contains(&find_library(name))
}

/// Bay and Howl: "Move 1 blood from each ready Gangrel antitribu you
/// control to a Gangrel antitribu in your uncontrolled region." Two ready
/// Gangrel antitribu (one locked) give 1 each, a torpid one and a Gangrel
/// nothing; the uncontrolled one goes past its capacity (rules-qa 28.3);
/// another clan's slot is no target.
#[test]
fn bay_and_howl_drains_ready_gangrel_antitribu_into_the_uncontrolled() {
    let mut e = game(2);
    let ga = |n| of_clan("Gangrel antitribu", 3..=11, n);
    let a = e.add_minion(SeatId(0), ga(0), 3, false);
    let b = e.add_minion(SeatId(0), ga(1), 3, false);
    e.state.minions[b.index()].locked = true;
    let t = e.add_minion(SeatId(0), ga(2), 3, true);
    let g = e.add_minion(SeatId(0), plain("Gangrel", 0), 3, false);
    uncontrolled(&mut e, 0, plain("Brujah", 0), 1);
    let cap = CRYPT[ga(3) as usize].capacity;
    uncontrolled(&mut e, 0, ga(3), cap);
    hand(&mut e, 0, &["Bay and Howl"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(!req.options.contains(&master_at(0, Target::Uncontrolled(0))));
    pick(&mut e, master_at(0, Target::Uncontrolled(1)));
    request(&mut e);
    assert_eq!([a, b, t, g].map(|m| blood(&e, m)), [2, 2, 3, 3]);
    assert_eq!(e.state.seats[0].uncontrolled[1].counters, cap + 2);
}

/// Campground Hunting Ground: "During your unlock phase, you may move 1
/// blood from the blood bank to a ready vampire you control." One ready
/// vampire fed; fed, it is no pick for another hunting ground that turn
/// (rules-qa 6.5).
#[test]
fn campground_hunting_ground_feeds_one_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Gangrel antitribu", 4..=11, 0), 1, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let c = in_play(&mut e, 0, "Campground Hunting Ground", None);
    let h = in_play(&mut e, 0, "Academic Hunting Ground", None);
    let feed = use_on(c, 0, None, Target::Minion(a));
    pick_when(&mut e, feed);
    let req = request(&mut e);
    assert_eq!(blood(&e, a), 2);
    assert!(!req.options.contains(&use_on(h, 0, None, Target::Minion(a))));
}

/// City Gangrel Connections: "You get 1 additional vote during each
/// political action." Its controller's own vote in a political action's
/// polling, once (either side, one use between them); the blood hunt's
/// referendum is not a political action (rules-qa 122.206).
#[test]
fn city_gangrel_connections_adds_one_vote_in_political_actions() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 1), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let c = in_play(&mut e, 1, "City Gangrel Connections", None);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, a));
    to_referendum(&mut e);
    pick_when(&mut e, ability(c.0, 1));
    let req = request(&mut e);
    assert!(!req.options.contains(&ability(c.0, 0)));
    assert!(!req.options.contains(&ability(c.0, 1)));
    assert_eq!(e.referendum().expect("referendum").tally(&e.state).1, 1);
}

/// City Gangrel Connections: a blood hunt is no political action, its vote
/// not offered there (rules-qa 122.206; ANK 20190606-2's reading).
#[test]
fn city_gangrel_connections_gives_no_blood_hunt_vote() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 5..=8, 0), 4, false);
    e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 1), 4, false);
    let v = e.add_minion(SeatId(1), of_clan("Brujah", 1..=3, 0), 1, true);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.state.seats[1].has_edge = true;
    let c = in_play(&mut e, 1, "City Gangrel Connections", None);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: zvtes::decision::ActionKind::Diablerie,
            target: Some(v),
        },
    );
    let req = until(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
    assert!(uses(&req, c.0).is_empty());
}

/// `n` distinct combat cards (fillers for hands and libraries).
fn fillers(n: usize) -> Vec<u16> {
    let out: Vec<u16> = zvtes::library::LIBRARY
        .iter()
        .enumerate()
        .filter(|(_, c)| c.types == [zvtes::library::CardType::Combat])
        .map(|(i, _)| i as u16)
        .take(n)
        .collect();
    assert_eq!(out.len(), n);
    out
}

/// Deal with the Devil: "Discard your hand and draw a new one. Do not
/// replace this card until after you discard your hand." The six others
/// discarded, a new hand of seven drawn: the card's own replacement in it
/// (rules-qa 122.40).
#[test]
fn deal_with_the_devil_discards_and_redraws_the_hand() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cards = fillers(16);
    hand(&mut e, 0, &["Deal with the Devil"]);
    for &c in &cards[..6] {
        e.state.seats[0].hand.push(c);
    }
    for &c in &cards[6..] {
        e.state.seats[0].library.push(c);
    }
    e.at_master_phase();
    pick(&mut e, master_play(0));
    request(&mut e);
    let s = &e.state.seats[0];
    assert!(
        cards[..6].iter().all(|c| s.ash.contains(c)),
        "the hand discarded"
    );
    assert!(s.ash.contains(&find_library("Deal with the Devil")));
    assert_eq!(s.hand.len(), 7);
    assert!(s.hand.iter().all(|c| cards[6..].contains(c)), "a new hand");
    assert_eq!(s.library.len(), 3);
}

/// Deal with the Devil with a short library: "You can draw more cards than
/// are in the library … draw as much as you can." [LSJ 20060620]
#[test]
fn deal_with_the_devil_draws_what_the_library_has() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cards = fillers(9);
    hand(&mut e, 0, &["Deal with the Devil"]);
    for &c in &cards[..6] {
        e.state.seats[0].hand.push(c);
    }
    for &c in &cards[6..] {
        e.state.seats[0].library.push(c);
    }
    e.at_master_phase();
    pick(&mut e, master_play(0));
    request(&mut e);
    let s = &e.state.seats[0];
    assert_eq!(s.hand.len(), 3);
    assert!(s.library.is_empty());
}

/// Kingston Penitentiary: its controller feeds a ready vampire of theirs in
/// their unlock phase, no lock; another Methuselah, in their own unlock
/// phase, gives the controller 1 pool, locks it and feeds one of theirs;
/// locked, a third Methuselah cannot.
#[test]
fn kingston_penitentiary_feeds_its_controller_and_others_for_a_pool() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let c = e.add_minion(SeatId(2), plain("Tremere", 0), 1, false);
    e.add_minion(SeatId(2), plain("Tremere", 1), 4, false);
    let k = in_play(&mut e, 0, "Kingston Penitentiary, Ontario", None);
    let others = |m| use_on(k, 1, None, Target::Minion(m));
    pick_when(&mut e, others(b));
    request(&mut e);
    assert_eq!(blood(&e, b), 2);
    assert_eq!(pool(&e), vec![11, 9, 10]);
    assert!(e.state.cards[k.index()].locked);
    never_offered_until(&mut e, SeatId(2), WindowLabel::MinionPhase, others(c));
    // Its controller's own use: no lock needed, it unlocks first anyway.
    e.state.cards[k.index()].locked = true;
    pick_when(&mut e, use_on(k, 0, None, Target::Minion(a)));
    request(&mut e);
    assert_eq!(blood(&e, a), 2);
}

/// Threestar Cab Company: "Lock during your master phase to move a blood
/// from the blood bank to this card. Lock during your unlock phase to move 1
/// blood from this card to your pool or to a ready vampire you control."
/// No master phase action spent; then a vampire fed, 1 blood off the card.
#[test]
fn threestar_cab_company_banks_then_feeds() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let t = in_play(&mut e, 0, "Threestar Cab Company", None);
    e.state.cards[t.index()].counters = 1;
    e.at_master_phase();
    pick(&mut e, ability(t.0, 0));
    assert_eq!(e.state.cards[t.index()].counters, 2);
    assert!(e.state.cards[t.index()].locked);
    assert_eq!(e.state.mpa_left(SeatId(0)), 1, "no master phase action");
    pick_when(&mut e, use_on(t, 2, None, Target::Minion(a)));
    request(&mut e);
    assert_eq!(blood(&e, a), 2);
    assert_eq!(e.state.cards[t.index()].counters, 1);
    assert!(e.state.minions[a.index()].hunting_ground_fed);
}

/// A trophy in play for seat `seat`, not awarded yet (no layer).
fn unawarded(e: &mut Engine, seat: u8, name: &str) -> CardId {
    e.state.cards.push(zvtes::state::CardInPlay::new(
        find_library(name),
        SeatId(seat),
        None,
        0,
    ));
    CardId(e.state.cards.len() as u32 - 1)
}

/// The card `name` in play, wherever.
fn card_in_play(e: &Engine, name: &str) -> CardId {
    let card = find_library(name);
    (0..e.state.cards.len())
        .map(|i| CardId(i as u32))
        .find(|c| e.state.cards[c.index()].card == card && e.state.cards[c.index()].in_play)
        .expect("in play")
}

/// Seat 0's vampire A, seat 1's torpid Red List vampire, seat 1 holding the
/// Edge (a blood hunt then asks): A's diablerie declared.
fn red_list_kill() -> (Engine, MinionId) {
    let red = textless(|c| !c.red_list && (1..=4).contains(&c.capacity), 0);
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), red, 2, true);
    e.state.minions[v.index()].made_red_list = true;
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.seats[1].has_edge = true;
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: zvtes::decision::ActionKind::Diablerie,
            target: Some(v),
        },
    );
    (e, a)
}

/// Whether a blood hunt polling opens before seat 0's next minion phase.
fn blood_hunt_called(e: &mut Engine) -> bool {
    loop {
        let req = request(e);
        if req.window == WindowLabel::BloodHuntPolling {
            return true;
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            return false;
        }
        e.choose(fallback(&req));
    }
}

/// Trophy: Diablerie: "+2 stealth when attempting diablerie" (a bleed gets
/// none); retrieved from hand for the Red List victim it bars that blood
/// hunt and burns at the action's end [LSJ 20050222].
#[test]
fn trophy_diablerie_awarded_before_the_blood_hunt_bars_it_and_burns() {
    let (mut e, a) = red_list_kill();
    assert_eq!(e.action().expect("diablerie").stealth(&e.state), 0);
    hand(&mut e, 0, &["Trophy: Diablerie"]);
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Hand,
            card: find_library("Trophy: Diablerie"),
        },
    );
    let t = card_in_play(&e, "Trophy: Diablerie");
    assert_eq!(e.state.cards[t.index()].host, Some(a));
    assert!(!blood_hunt_called(&mut e), "no blood hunt on its vampire");
    assert!(!e.state.cards[t.index()].in_play);
    assert!(ash_has(&e, 0, "Trophy: Diablerie"));
    // Without it, the blood hunt is called.
    let (mut e, _) = red_list_kill();
    assert!(blood_hunt_called(&mut e));
    // On the vampire: +2 stealth on its diablerie, none on a bleed.
    let (mut e, a) = red_list_kill();
    in_play(&mut e, 0, "Trophy: Diablerie", Some(a));
    assert_eq!(e.action().expect("diablerie").stealth(&e.state), 2);
}

/// Trophy: Diablerie on Abactor's hunter: "The Blood Hunt referendum is
/// still a Blood Hunt: … Trophy: Diablerie would prevent the referendum."
/// [LSJ 20091026] [LSJ 20100112] No diablerie: the trophy stays.
#[test]
fn abactor_calls_no_blood_hunt_on_a_trophy_diablerie_bearer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1].has_edge = true;
    let t = in_play(&mut e, 0, "Trophy: Diablerie", Some(a));
    hand(&mut e, 0, &["Abactor"]);
    pick(&mut e, play(0, 0, a));
    assert!(!blood_hunt_called(&mut e));
    assert!(e.state.cards[t.index()].in_play, "no diablerie: kept");
}

/// Trophy: Domain: "+1 intercept. Once each action, this vampire can burn 1
/// blood to unlock and attempt to block." Its controller uses it [ANK
/// 20210309-3] for its locked vampire; once each action.
#[test]
fn trophy_domain_intercepts_and_unlocks_to_block() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let d = in_play(&mut e, 1, "Trophy: Domain", Some(x));
    assert_eq!(e.state.bearer_always(x, zvtes::cards::Stat::Intercept), 1);
    e.state.minions[x.index()].locked = true;
    pick(&mut e, bleed(a));
    let req = loop {
        let req = request(&mut e);
        if !uses(&req, d.0).is_empty() {
            break req;
        }
        e.choose(fallback(&req));
    };
    assert_eq!(req.seat, SeatId(1));
    e.choose(index_of(&req, uses(&req, d.0)[0]));
    assert!(!e.state.minions[x.index()].locked);
    assert_eq!(blood(&e, x), 2);
    let a_ = e.action().expect("bleed");
    assert!(matches!(a_.state, zvtes::actions::ActionState::Attempt(m) if m == x));
}

/// Trophy: Hunting Ground: "During your unlock phase, the vampire with this
/// card gains 2 blood from the blood bank. A vampire can gain blood from
/// only one hunting ground card each turn." Fed first by another hunting
/// ground, it gains nothing from this one (rules-qa 6.5); unawarded, it
/// feeds no one.
#[test]
fn trophy_hunting_ground_feeds_its_vampire_two() {
    for fed_first in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        in_play(&mut e, 0, "Trophy: Hunting Ground", Some(a));
        unawarded(&mut e, 0, "Trophy: Hunting Ground");
        let h = in_play(&mut e, 0, "Academic Hunting Ground", None);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        if fed_first {
            pick_when(&mut e, use_on(h, 0, None, Target::Minion(a)));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, a), if fed_first { 2 } else { 3 }, "{fed_first}");
    }
}

/// Poacher's Hunting Ground counts a Trophy: Hunting Ground: "Is a
/// non-derivative hunting ground (ie. taken into account by Poacher's
/// Hunting Ground), even if not active yet." [ANK 20170430] [LSJ 20080610]
/// One unawarded on seat 1's side: one anarch fed, not two.
#[test]
fn trophy_hunting_ground_counts_for_poachers_even_unawarded() {
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 4..=8, 0), 1, false);
    let y = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 4..=8, 1), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let p = in_play(&mut e, 0, "Poacher's Hunting Ground", None);
    unawarded(&mut e, 1, "Trophy: Hunting Ground");
    let req = offering(&mut e, use_on(p, 0, None, Target::Minion(x)));
    assert!(!req.options.contains(&use_on(p, 1, None, Target::Minion(y))));
    e.choose(index_of(&req, use_on(p, 0, None, Target::Minion(x))));
    let req = request(&mut e);
    assert!(!req.options.contains(&use_on(p, 1, None, Target::Minion(y))));
    assert_eq!(blood(&e, x), 2);
}

/// Trophy: Safe Passage: "While the vampire with this card is ready, he or
/// she cannot be the target of other Methuselahs' actions." Seat 1's
/// directed actions at it not offered; in torpor, its diablerie is.
#[test]
fn trophy_safe_passage_shields_its_ready_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 1, "Trophy: Safe Passage", Some(x));
    let y = e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
    hand(&mut e, 0, &["Bum's Rush"]);
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(a),
        target: Target::Minion(y),
    }));
    let at_x = |o: &Choice| match o {
        Choice::Play { target, .. } => *target == Target::Minion(x),
        Choice::TakeAction { target, .. } => *target == Some(x),
        _ => false,
    };
    assert!(!req.options.iter().any(at_x), "{:?}", req.options);
    e.state.minions[x.index()].in_torpor = true;
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&Choice::TakeAction {
        minion: a,
        action: zvtes::decision::ActionKind::Diablerie,
        target: Some(x),
    }));
}

/// The trophy `name` searched from hand for the Red List kill: its id.
fn award_from_hand(e: &mut Engine, name: &str) -> CardId {
    hand(e, 0, &[name]);
    pick_when(
        e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Hand,
            card: find_library(name),
        },
    );
    card_in_play(e, name)
}

/// Trophy: Clan Respect: "When this card enters play, choose a clan." Played
/// in the master phase, the clan chosen then, no effect yet; searched for
/// an award, chosen as it lands (D1).
#[test]
fn trophy_clan_respect_chooses_its_clan_as_it_enters_play() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Trophy: Clan Respect"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    pick(&mut e, Choice::ChooseClan("Tremere"));
    request(&mut e);
    let t = card_in_play(&e, "Trophy: Clan Respect");
    assert_eq!(e.state.cards[t.index()].chosen_clan.0, Some("Tremere"));
    assert!(e.state.cards[t.index()].host.is_none());
    let (mut e, a) = red_list_kill();
    let t = award_from_hand(&mut e, "Trophy: Clan Respect");
    pick(&mut e, Choice::ChooseClan("Ventrue"));
    assert_eq!(e.state.cards[t.index()].host, Some(a));
    assert_eq!(e.state.cards[t.index()].chosen_clan.0, Some("Ventrue"));
}

/// Trophy: Clan Respect: "Vampires of the chosen clan cannot block the
/// vampire with this card." Ventrue chosen: seat 1's Ventrue cannot block
/// its vampire's bleed, its Tremere can.
#[test]
fn trophy_clan_respect_bars_blocks_by_its_clan() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("Tremere", 0), 4, false);
    let c = in_play(&mut e, 0, "Trophy: Clan Respect", Some(a));
    e.state.cards[c.index()].chosen_clan = zvtes::state::SparseOption(Some("Ventrue"));
    pick(&mut e, bleed(a));
    let req = offering(&mut e, block(t));
    assert!(!req.options.contains(&block(v)));
}

/// Trophy: Discipline: "choose a Discipline. The vampire with this card has
/// the chosen Discipline at the superior level." Awarded, Obfuscate chosen:
/// its vampire has it superior; no capacity with it.
#[test]
fn trophy_discipline_gives_the_chosen_discipline_at_superior() {
    let (mut e, a) = red_list_kill();
    let cap = e.state.capacity_of(a);
    let t = award_from_hand(&mut e, "Trophy: Discipline");
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::ChooseDiscipline("obf")));
    e.choose(index_of(&req, Choice::ChooseDiscipline("obf")));
    assert_eq!(e.state.cards[t.index()].host, Some(a));
    assert_eq!(e.state.discipline_level(a, "obf"), 2);
    assert_eq!(e.state.capacity_of(a), cap);
}

/// Trophy: Wealth: five counters as it is placed on a vampire (none put in
/// play in the master phase); they pay its vampire's equipment, the card
/// burned with its last.
#[test]
fn trophy_wealth_pays_its_vampires_equipment_and_burns_empty() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Trophy: Wealth"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    request(&mut e);
    assert_eq!(
        e.state.cards[card_in_play(&e, "Trophy: Wealth").index()].counters,
        0
    );
    let (mut e, _) = red_list_kill();
    let w = award_from_hand(&mut e, "Trophy: Wealth");
    assert_eq!(e.state.cards[w.index()].counters, 5);
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = in_play(&mut e, 0, "Trophy: Wealth", Some(a));
    e.state.cards[w.index()].counters = 2;
    hand(&mut e, 0, &[".44 Magnum"]);
    let pool0 = pool(&e)[0];
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::ResolveRider { card: w.0 });
    let req = request(&mut e);
    if req.options.contains(&Choice::SetX(2)) {
        e.choose(index_of(&req, Choice::SetX(2)));
    }
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[0], pool0, "the counters paid the 2 pool");
    assert!(!e.state.cards[w.index()].in_play, "burned with its last");
    assert!(ash_has(&e, 0, "Trophy: Wealth"));
}

/// Trophy: Retainers: in its controller's unlock phase, its vampire employs
/// a retainer from the library or hand, not as an action, its vampire
/// unlocked and free to act [LSJ 20080815]; the version among those its
/// Disciplines meet [LSJ 20080803], its player's choice (rules-qa 125.17);
/// the cost paid as normal.
#[test]
fn trophy_retainers_employs_by_its_vampires_disciplines() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ANI", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let t = in_play(&mut e, 0, "Trophy: Retainers", Some(a));
    fill_hand(&mut e, 0);
    e.state.seats[0].library.push(find_library("Raven Spy"));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick_when(&mut e, ability(t.0, 0));
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: find_library("Raven Spy"),
        },
    );
    pick(
        &mut e,
        Choice::Version {
            card: find_library("Raven Spy"),
            mode: 1,
        },
    );
    let spy = card_in_play(&e, "Raven Spy");
    assert_eq!(e.state.cards[spy.index()].host, Some(a));
    assert_eq!(
        e.state.cards[spy.index()].mode,
        1,
        "[ANI]: the superior version chosen"
    );
    assert_eq!(blood(&e, a), 3, "its cost paid");
    assert!(!e.state.minions[a.index()].locked, "no action");
}

/// Trophy: Progeny awarded to a Brujah: instead, its vampire's controller's
/// non-unique 2-capacity vampire of that clan and sect, 2 blood from the
/// bank, a Discipline master searched for it; burned, it is a library card
/// in the ash heap [LSJ 20020115].
#[test]
fn trophy_progeny_becomes_a_two_capacity_childe() {
    let (mut e, a) = red_list_kill();
    fill_hand(&mut e, 0);
    e.state.seats[0].library.push(find_library("Potence"));
    let t = award_from_hand(&mut e, "Trophy: Progeny");
    let c = &e.state.cards[t.index()];
    assert!(c.host.is_none());
    let m = c.body.expect("a vampire");
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: find_library("Potence"),
        },
    );
    request(&mut e);
    let v = &e.state.minions[m.index()];
    assert!(v.in_play && v.is_vampire());
    assert_eq!(v.owner, SeatId(0));
    assert_eq!(v.clan(), e.state.minions[a.index()].clan());
    assert_eq!(e.state.sect_of(m), e.state.sect_of(a));
    assert_eq!(v.blood, 2);
    assert_eq!(
        e.state.capacity_of(m),
        Some(3),
        "2, and its Discipline card"
    );
    assert_eq!(e.state.discipline_level(m, "pot"), 1);
    e.burn(m);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Trophy: Progeny")));
    assert!(e.state.seats[0].ash_crypt.is_empty());
}

/// Trophy: Progeny on a sterile vampire: moved as a trophy, nothing else.
#[test]
fn trophy_progeny_on_a_sterile_vampire_is_an_inert_trophy() {
    let (mut e, a) = red_list_kill();
    e.state.minions[a.index()].made_sterile = zvtes::state::Sparse(1);
    let n = e.state.minions.len();
    let t = award_from_hand(&mut e, "Trophy: Progeny");
    assert_eq!(e.state.cards[t.index()].host, Some(a));
    assert_eq!(e.state.minions.len(), n, "no vampire");
}

/// A prince for seat `seat`, with `blood`.
fn prince(e: &mut Engine, seat: u8, nth: usize, blood: u8) -> MinionId {
    let m = e.add_minion(
        SeatId(seat),
        of_sect(Sect::Camarilla, 4..=8, nth),
        blood,
        false,
    );
    e.state.minions[m.index()].granted_title = Some(Title::Prince("Paris"));
    m
}

/// The Treatment: on a prince that successfully bled seat 0 since its last
/// turn [LSJ 20020429], not one whose bleed came before; the card itself on
/// it; each unlock 1 unpreventable damage; its prince can burn it as a Ⓓ
/// action, requiring the title [LSJ 20080604].
#[test]
fn the_treatment_damages_its_prince_on_each_unlock() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Ravnos", 0), 4, false);
    let p = prince(&mut e, 1, 0, 4);
    let q = prince(&mut e, 1, 1, 4);
    e.state.seats[0].previous_turn = 2;
    e.state.minions[p.index()].bleeds.push((3, SeatId(0), 1));
    e.state.minions[q.index()].bleeds.push((2, SeatId(0), 1));
    hand(&mut e, 0, &["Treatment, The"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(!req.options.contains(&master_at(0, Target::Minion(q))));
    pick(&mut e, master_at(0, Target::Minion(p)));
    request(&mut e);
    let t = placed_on(&e, "Treatment, The", Some(p));
    e.state.minions[p.index()].locked = true;
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, p), 3, "unlocked: 1 damage");
    assert!(req.options.contains(&granted(t.0, p)));
    e.choose(index_of(&req, granted(t.0, p)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[t.index()].in_play);
}

/// The Treatment: "Will cease functioning if the vampire it is on loses his
/// Prince title. It will resume working if the vampire becomes a Prince
/// again." [LSJ 19980209] Untitled: no damage, nor the burn action, an
/// action requiring the title [LSJ 20080604]; titled again, the damage.
#[test]
fn the_treatment_sleeps_while_its_prince_title_is_lost() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = prince(&mut e, 1, 0, 4);
    prince(&mut e, 1, 1, 4);
    let t = in_play(&mut e, 0, "Treatment, The", Some(p));
    e.state.minions[p.index()].granted_title = None;
    e.state.minions[p.index()].locked = true;
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, p), 4);
    assert!(!req.options.contains(&granted(t.0, p)));
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[p.index()].locked = true;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, p), 3);
}

/// The Treatment: "Does nothing to the target Prince if the Prince is in
/// Torpor with zero blood." [TOM 19960423]
#[test]
fn the_treatment_spares_an_empty_torpid_prince() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = prince(&mut e, 1, 0, 0);
    prince(&mut e, 1, 1, 4);
    e.state.minions[p.index()].in_torpor = true;
    e.state.minions[p.index()].locked = true;
    in_play(&mut e, 0, "Treatment, The", Some(p));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let v = &e.state.minions[p.index()];
    assert!(v.in_play && v.in_torpor && !v.locked);
    assert_eq!(v.blood, 0);
}

/// Insurance Scam: "During your turn, you may lock this card and burn X
/// locations you control to gain X pool." Two (one on a minion): +2 pool,
/// both burned, the card locked, not used again this turn.
#[test]
fn insurance_scam_burns_locations_for_pool() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let s = in_play(&mut e, 0, "Insurance Scam", None);
    let h = in_play(&mut e, 0, "Academic Hunting Ground", None);
    let haven = in_play(&mut e, 0, "Secure Haven", Some(a));
    in_play(&mut e, 1, "Asylum Hunting Ground", None);
    e.at_master_phase();
    pick(&mut e, ability(s.0, 0));
    let req = request(&mut e);
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            Choice::ChooseCard(h.0),
            Choice::ChooseCard(haven.0)
        ]
    );
    e.choose(1);
    pick(&mut e, Choice::ChooseCard(haven.0));
    request(&mut e);
    assert_eq!(pool(&e)[0], 12);
    assert!(ash_has(&e, 0, "Academic Hunting Ground") && ash_has(&e, 0, "Secure Haven"));
    assert!(e.state.cards[s.index()].locked);
    let req = request(&mut e);
    assert!(uses(&req, s.0).is_empty());
}

/// Not to Be: out of turn, seat 0 cancels seat 1's event as it is played
/// (its cost unpaid), the card put in play: seat 0's next master phase has
/// one action (+1, less its out-of-turn play), its discard phase none, the
/// card burned there.
#[test]
fn not_to_be_cancels_an_event_and_shifts_phase_actions() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Not to Be", "Deal with the Devil"]);
    hand(&mut e, 1, &["Urban Jungle"]);
    until(&mut e, SeatId(1), WindowLabel::DiscardPhase);
    pick(&mut e, master_play(0));
    pick(&mut e, master_play(0));
    request(&mut e);
    assert!(e.state.seats[1].ash.contains(&find_library("Urban Jungle")));
    let n = placed_on(&e, "Not to Be", None);
    assert_eq!(pool(&e)[0], 9, "its own pool");
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert_eq!(e.state.mpa_left(SeatId(0)), 1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].dpa, 0, "no discard phase action");
    assert!(!e.state.cards[n.index()].in_play);
    assert!(ash_has(&e, 0, "Not to Be"));
}

/// Ruins of Charizel: the pool paid to unlock an infernal minion by the
/// trait goes onto it; Metro Underground's pool does not ("Only works when
/// unlocking minions using their Infernal ability", LSJ 20050228-2); its
/// counters reduce a bleed against its controller; burned in the
/// predator's discard phase.
#[test]
fn ruins_of_charizel_banks_infernal_unlocks_and_reduces_a_bleed() {
    let mut e = game(2);
    let inf = e.add_minion(SeatId(0), textless(|c| c.infernal, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let r = in_play(&mut e, 0, "Ruins of Charizel", None);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[inf.index()].locked = true;
    pick_when(&mut e, Choice::UnlockInfernal { minion: inf });
    assert_eq!(e.state.cards[r.index()].counters, 1);
    assert_eq!(pool(&e)[0], 9, "the pool still paid");
    // Seat 1 bleeds for 1: X=1 reduces it to nothing.
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(b));
    pick_when(&mut e, ability(r.0, 0));
    let req = request(&mut e);
    if req.options.contains(&Choice::SetX(1)) {
        e.choose(index_of(&req, Choice::SetX(1)));
    }
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[0], 9, "bleed reduced");
    e.state.cards[r.index()].counters = 2;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.cards[r.index()].counters,
        0,
        "the predator's discard phase"
    );
}

/// Ruins of Charizel: Metro Underground's unlock is no infernal trait's:
/// its pool to the bank, no counter. [LSJ 20050228-2]
#[test]
fn ruins_of_charizel_takes_no_metro_underground_pool() {
    let mut e = game(2);
    let inf = e.add_minion(SeatId(0), textless(|c| c.infernal, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let r = in_play(&mut e, 0, "Ruins of Charizel", None);
    let m = in_play(&mut e, 0, "Metro Underground", None);
    e.state.minions[inf.index()].locked = true;
    pick_when(&mut e, use_on(m, 0, None, Target::Minion(inf)));
    request(&mut e);
    assert!(!e.state.minions[inf.index()].locked);
    assert_eq!(pool(&e)[0], 9);
    assert_eq!(e.state.cards[r.index()].counters, 0);
}

/// Glass Walker Pact: in its controller's unlock phase, the top library card
/// burned: 1 blood from any ready locked vampire to a ready Giovanni,
/// either Methuselah's (D11); a master burned, the Pact burns as well and
/// the blood still moves; another Methuselah's werewolf burns it.
#[test]
fn glass_walker_pact_moves_blood_and_burns_on_a_master_or_a_werewolf() {
    for top in ["Form of Mist", "Deal with the Devil"] {
        let mut e = game(2);
        let g = e.add_minion(SeatId(0), plain("Giovanni", 0), 1, false);
        let g2 = e.add_minion(SeatId(1), plain("Giovanni", 1), 1, false);
        let l = e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
        let p = in_play(&mut e, 0, "Glass Walker Pact", None);
        fill_hand(&mut e, 0);
        e.state.seats[0].library.push(find_library(top));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        e.state.minions[l.index()].locked = true;
        pick_when(&mut e, use_on(p, 0, None, Target::Minion(l)));
        let req = request(&mut e);
        assert!(
            req.options.contains(&Choice::ChooseMinion(g2)),
            "any Giovanni"
        );
        e.choose(index_of(&req, Choice::ChooseMinion(g)));
        request(&mut e);
        assert_eq!((blood(&e, l), blood(&e, g)), (2, 2), "{top}");
        assert!(e.state.seats[0].ash.contains(&find_library(top)));
        assert_eq!(
            e.state.cards[p.index()].in_play,
            top == "Form of Mist",
            "{top}"
        );
    }
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Giovanni", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
    let p = in_play(&mut e, 0, "Glass Walker Pact", None);
    e.add_ally(SeatId(1), find_library("Tunnel Runner"));
    request(&mut e);
    assert!(!e.state.cards[p.index()].in_play, "a werewolf of another's");
}

/// Warning Sirens, out of turn: on seat 0's vampire the directed action
/// targets, attempting to block short of the stealth, +1 intercept (D15):
/// the block made. Never on its player's own turn.
#[test]
fn warning_sirens_gives_a_targeted_blocker_intercept() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Warning Sirens"]);
    hand(&mut e, 1, &["Bum's Rush"]);
    let own = request(&mut e);
    assert!(plays(&own).is_empty(), "never on its player's own turn");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[b.index()].next_action_stealth = 1;
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(b),
            target: Target::Minion(x),
        },
    );
    pick_when(&mut e, block(x));
    pick_when(&mut e, master_at(0, Target::Minion(x)));
    request(&mut e);
    assert!(ash_has(&e, 0, "Warning Sirens"));
    assert!(e.state.minions[x.index()].locked, "the block made");
}

/// Warning Sirens, out of turn, on a minion in combat: "That minion gets an
/// optional strike: dodge this round."
#[test]
fn warning_sirens_gives_a_combatant_a_dodge() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Warning Sirens"]);
    hand(&mut e, 1, &["Bum's Rush"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(b),
            target: Target::Minion(x),
        },
    );
    let dodge_mode = Choice::Play {
        card: 0,
        mode: 1,
        actor: None,
        target: Target::Minion(x),
    };
    pick_when(&mut e, dodge_mode);
    let req = loop {
        let req = request(&mut e);
        if req.options.contains(&Choice::RoundStrike { at: None }) {
            break req;
        }
        assert!(e.combat().is_some(), "the dodge offered in this combat");
        e.choose(fallback(&req));
    };
    assert_eq!(req.seat, SeatId(0));
    e.choose(index_of(&req, Choice::RoundStrike { at: None }));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4, "dodged");
}

/// Seat 0 plays Betrayer on its uncontrolled copy of seat 1's vampire `v`
/// (slot 0): the card's id.
fn betray(e: &mut Engine, v: MinionId) -> CardId {
    let card = e.state.minions[v.index()]
        .crypt_card()
        .expect("a crypt vampire");
    uncontrolled(e, 0, card, 1);
    let i = e.state.seats[0].hand.len() as u8;
    hand(e, 0, &["Betrayer"]);
    e.at_master_phase();
    let slot = e.state.seats[0].uncontrolled.len() as u8 - 1;
    pick(e, master_at(i, Target::Uncontrolled(slot)));
    request(e);
    placed_on(e, "Betrayer", None)
}

/// Betrayer: "The affected Methuselah is named when the card is played"
/// [RTR 19980928]; "You select an uncontrolled vampire when you play the
/// card … you still have to choose just one of them" [LSJ 19990419]: two
/// of seat 1's vampires in seat 0's uncontrolled region, one chosen, its
/// slot; a vampire of seat 0's own is no target.
#[test]
fn betrayer_names_its_methuselah_as_played() {
    let mut e = game(3);
    let own = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), plain("Tremere", 0), 4, false);
    let own_card = e.state.minions[own.index()].crypt_card().unwrap();
    uncontrolled(&mut e, 0, own_card, 1);
    let w_card = e.state.minions[w.index()].crypt_card().unwrap();
    uncontrolled(&mut e, 0, w_card, 1);
    let b = betray(&mut e, v);
    let c = &e.state.cards[b.index()];
    assert_eq!(c.chosen, vec![v]);
    assert!(c.against.contains(SeatId(1)) && !c.against.contains(SeatId(2)));
    let mut e2 = game(2);
    let own = e2.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e2.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let own_card = e2.state.minions[own.index()].crypt_card().unwrap();
    uncontrolled(&mut e2, 0, own_card, 1);
    hand(&mut e2, 0, &["Betrayer"]);
    e2.at_master_phase();
    let req = request(&mut e2);
    assert!(plays(&req).is_empty(), "{req:?}");
}

/// Betrayer: "Each of their turns, the person subjected to Betrayer can
/// name a vampire. If the guess is correct, Betrayer is burned." [RTR
/// 19941109] Seat 1 burns 1 pool each unlock phase, then may burn another
/// to guess among its vampires: wrong, the pool only; right, the card
/// burns.
#[test]
fn betrayer_burns_a_pool_each_unlock_and_a_right_guess_burns_it() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let b = betray(&mut e, v);
    let req = offering(&mut e, Choice::ChooseMinion(w));
    assert_eq!(req.seat, SeatId(1));
    assert_eq!(pool(&e)[1], 9, "the burn first");
    assert!(req.options.contains(&Choice::Pass) && req.options.contains(&Choice::ChooseMinion(v)));
    e.choose(index_of(&req, Choice::ChooseMinion(w)));
    request(&mut e);
    assert_eq!(pool(&e)[1], 8, "1 more for the wrong guess");
    assert!(e.state.cards[b.index()].in_play);
    pick_when(&mut e, Choice::ChooseMinion(v));
    request(&mut e);
    assert_eq!(pool(&e)[1], 6);
    assert!(!e.state.cards[b.index()].in_play);
    assert!(ash_has(&e, 0, "Betrayer"));
}

/// Betrayer: "a player can be damaged by multiple Betrayers … would have to
/// pay 1 pool for each Betrayer in order to burn them, even if the same
/// vampire is the betrayer in each case" [RTR 19941109]: two copies, 2 pool
/// burned, each its own guess.
#[test]
fn betrayer_copies_each_burn_and_each_take_a_guess() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let b1 = betray(&mut e, v);
    let b2 = in_play(&mut e, 0, "Betrayer", None);
    e.state.cards[b2.index()].chosen = vec![v];
    e.state.cards[b2.index()].against = zvtes::ids::SeatSet::single(SeatId(1));
    pick_when(&mut e, Choice::ChooseMinion(v));
    pick_when(&mut e, Choice::ChooseMinion(v));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 2 - 2);
    assert!(!e.state.cards[b1.index()].in_play && !e.state.cards[b2.index()].in_play);
}

/// Betrayer: "If the vampire chosen for Betrayer is burned, so is Betrayer.
/// If the vampire chosen for Betrayer becomes contested, Betrayer is
/// nullified until the contest is resolved. If a player takes control of
/// the vampire that is the target of the betrayer, that player takes the
/// pool loss." [RTR 19941109]
#[test]
fn betrayer_burns_with_its_vampire_sleeps_while_contested_and_follows_control() {
    // Control taken: the new controller pays, the old one no more.
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), plain("Tremere", 0), 4, false);
    let b = betray(&mut e, v);
    e.state.minions[v.index()].owner = SeatId(2);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1..], [10, 9]);
    assert!(e.state.cards[b.index()].against.contains(SeatId(2)));
    // Contested: no burn that unlock phase.
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    betray(&mut e, v);
    e.state.minions[v.index()].contested = true;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10, "nullified");
    // Burned: the card burns with it.
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let b = betray(&mut e, v);
    e.burn(v);
    assert!(!e.state.cards[b.index()].in_play);
    assert!(ash_has(&e, 0, "Betrayer"));
}

/// Betrayer: the vampire chosen is hidden from every Methuselah but its
/// player (owner's reading asked, rules-qa Batch 125): seat 1's legal view
/// of the card shows none, seat 0's shows it; the named Methuselah is
/// public; the guess reads only that.
#[test]
fn betrayer_hides_its_vampire_from_the_victim() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let b = betray(&mut e, v);
    let theirs = e.state.legal_view(SeatId(1));
    assert!(theirs.cards[b.index()].chosen.is_empty());
    assert!(theirs.cards[b.index()].against.contains(SeatId(1)));
    assert_eq!(
        e.state.legal_view(SeatId(0)).cards[b.index()].chosen,
        vec![v]
    );
}

// ---- Modifiers and reactions (modifier_reaction) --------------------------

/// Seat 0's acting `a` (capacity in `acts`, 3 blood) and a second vampire;
/// seat 1's Baali `x` (capacity in `blocks`) and a second vampire.
fn baali_blocks(
    acts: std::ops::RangeInclusive<u8>,
    blocks: std::ops::RangeInclusive<u8>,
) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let not_baali = |c: &zvtes::crypt::CryptCard| !["Baali", "Brujah", "Ventrue"].contains(&c.clan);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| not_baali(c) && acts.contains(&c.capacity), 0),
        3,
        false,
    );
    e.add_minion(
        SeatId(0),
        textless(|c| not_baali(c) && (4..=11).contains(&c.capacity), 1),
        4,
        false,
    );
    let x = e.add_minion(SeatId(1), of_clan("Ventrue", blocks, 0), 4, false);
    // Every Baali vampire has a definition: a Ventrue made Baali.
    e.state.minions[x.index()].clan_change = Some("Baali");
    e.add_minion(SeatId(1), of_clan("Brujah", 4..=11, 0), 4, false);
    (e, a, x)
}

/// Bloodstorm of Chorazin: "Lock this Baali. This Baali inflicts 1
/// unpreventable damage on the acting minion and each retainer on the
/// acting minion. The action ends." The action ends unsuccessfully at
/// once, nothing more played (RTR 20180719, ANK 20200207): no combat, no
/// pool bled, the retainer of 1 life burned to its owner's ash heap; an
/// older acting vampire is no target ("an ally or younger vampire").
/// Ledger: [RTR 20180719] [ANK 20200207].
#[test]
fn bloodstorm_of_chorazin_damages_the_acting_minion_and_its_retainers_and_ends_the_action() {
    let (mut e, a, x) = baali_blocks(2..=4, 6..=11);
    let r = in_play(&mut e, 0, "Ghoul Retainer", Some(a));
    e.state.cards[r.index()].counters = 1;
    hand(&mut e, 1, &["Bloodstorm of Chorazin"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, x));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::MinionPhase)
    );
    assert!(e.action().is_none(), "ended at once");
    assert_eq!(blood(&e, a), 3 - 1);
    assert!(!e.state.cards[r.index()].in_play);
    assert!(ash_has(&e, 0, "Ghoul Retainer"));
    assert_eq!(pool(&e)[1], 10);
    assert!(e.state.minions[x.index()].locked);

    let baali = of_clan("Ventrue", 6..=11, 0);
    let older = CRYPT[baali as usize].capacity + 1..=11;
    let (mut e, a, x) = baali_blocks(older, 6..=11);
    hand(&mut e, 1, &["Bloodstorm of Chorazin"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
}

/// Ilomba: "If damage is done simultaneously to Ilomba and its owner, the
/// controller chooses the order of resolution" (LSJ 20090617-2): Bloodstorm
/// of Chorazin damages both at once; with 1 blood, Ilomba's point first
/// leaves the vampire empty for its own damage (torpor), its own first
/// leaves none for Ilomba (burned).
/// Ledger: [LSJ 20090617-2].
#[test]
fn bloodstorm_lets_ilombas_controller_order_its_damage() {
    for ilomba_first in [true, false] {
        let (mut e, a, x) = baali_blocks(2..=4, 6..=11);
        e.state.minions[a.index()].blood = 1;
        let il = in_play(&mut e, 0, "Ilomba", Some(a));
        hand(&mut e, 1, &["Bloodstorm of Chorazin"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(x));
        pick_when(&mut e, play(0, 0, x));
        let req = request(&mut e);
        assert_eq!(req.seat, SeatId(0));
        assert_eq!(
            req.options,
            vec![Choice::ChooseMinion(a), Choice::ChooseCard(il.0)]
        );
        let first = if ilomba_first {
            Choice::ChooseCard(il.0)
        } else {
            Choice::ChooseMinion(a)
        };
        pick(&mut e, first);
        let v = &e.state.minions[a.index()];
        if ilomba_first {
            assert!(v.in_play && v.in_torpor, "Ilomba first: to torpor");
        } else {
            assert!(!v.in_play, "the vampire first: burned");
        }
    }
}

/// Dirty Little Secrets: "For each pool this Nosferatu antitribu bleeds
/// from the target Methuselah, burn one card from the top of that
/// Methuselah's library": a bleed for 2 burns 2. "If the Methuselah you
/// are bleeding does not burn any pool due to another effect, then she
/// does not burn any cards from her library either" (LSJ 19970224): Major
/// Boon's player pays, the library untouched.
/// Ledger: [LSJ 19970224].
#[test]
fn dirty_little_secrets_burns_a_library_card_per_pool_bled() {
    for boon in [false, true] {
        let mut e = game(3);
        let a = e.add_minion(
            SeatId(0),
            of_clan("Nosferatu antitribu", 4..=11, 0),
            4,
            false,
        );
        e.add_minion(
            SeatId(0),
            of_clan("Nosferatu antitribu", 4..=11, 1),
            4,
            false,
        );
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
        bonus(&mut e, a, zvtes::cards::Stat::Bleed);
        fill_hand(&mut e, 1);
        for _ in 0..5 {
            e.state.seats[1].library.push(find_library("Deflection"));
        }
        hand(&mut e, 0, &["Dirty Little Secrets"]);
        if boon {
            hand(&mut e, 2, &["Major Boon"]);
        }
        pick(&mut e, bleed(a));
        pick_when(&mut e, play(0, 0, a));
        if boon {
            pick_when(&mut e, master_play(0));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let lib = e.state.seats[1].library.len();
        if boon {
            assert_eq!(pool(&e), vec![10, 10, 8]);
            assert_eq!(lib, 5, "nothing burned");
        } else {
            assert_eq!(pool(&e)[1], 8);
            assert_eq!(lib, 3);
            assert_eq!(e.state.seats[1].ash.len(), 2);
        }
    }
}

/// Dr. Marisa Fletcher, CDC: "During your unlock phase, add two counters
/// to this card from the blood bank. When a vampire with capacity less
/// than X is blocked while hunting, where X is the number of counters on
/// this card, burn that vampire and all the counters on this card." Any
/// Methuselah's vampire; with the "if blocked" effects, before any card
/// or combat (ANK 20220116): burned, no combat, the event stays; a
/// capacity-X hunter and a small bleeder are spared.
/// Ledger: [ANK 20220116] [ANK 20211207] [ANK 20210627] [RTR 19991206] [LSJ 20091125].
#[test]
fn dr_marisa_fletcher_burns_a_small_vampire_blocked_hunting() {
    // (acting capacity, its action, burned)
    for (caps, hunt, burned) in [
        (2..=3, true, true),
        (4..=4, true, false),
        (2..=3, false, false),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_clan("Ventrue", caps, 0), 2, false);
        e.add_minion(SeatId(0), of_clan("Ventrue", 5..=11, 1), 4, false);
        let x = e.add_minion(SeatId(1), of_clan("Brujah", 5..=11, 0), 4, false);
        e.add_minion(SeatId(1), of_clan("Brujah", 5..=11, 1), 4, false);
        let ev = in_play(&mut e, 1, "Dr. Marisa Fletcher, CDC", None);
        e.state.cards[ev.index()].counters = 4;
        bonus(&mut e, x, zvtes::cards::Stat::Intercept);
        let act = if hunt {
            action(a, zvtes::decision::ActionKind::Hunt)
        } else {
            bleed(a)
        };
        pick(&mut e, act);
        pick_when(&mut e, block(x));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(!e.state.minions[a.index()].in_play, burned);
        assert!(e.state.cards[ev.index()].in_play);
        assert_eq!(
            e.state.cards[ev.index()].counters,
            if burned { 0 } else { 4 }
        );
        if burned {
            assert_eq!(blood(&e, x), 4, "no combat");
        }
    }
    // Two counters in each of its controller's unlock phases.
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let ev = in_play(&mut e, 1, "Dr. Marisa Fletcher, CDC", None);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[ev.index()].counters, 2);
}

/// Esteem: "Only usable at the end of a successful action directed at the
/// Methuselah with the edge. You gain the edge." After a successful
/// directed lock (Steal the Mind [dom]) at seat 1, holding the Edge, seat
/// 0 takes it; after its successful bleed the Edge already moved, so no
/// Methuselah it is directed at holds it (read as played).
#[test]
fn esteem_takes_the_edge_after_a_successful_action_at_its_holder() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let y = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dom"];
    e.state.seats[1].has_edge = true;
    hand(&mut e, 0, &["Steal the Mind", "Esteem"]);
    pick(&mut e, play_at(0, a, y));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].has_edge && !e.state.seats[1].has_edge);
    assert!(e.state.minions[y.index()].locked);

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.state.seats[1].has_edge = true;
    hand(&mut e, 0, &["Esteem"]);
    pick(&mut e, bleed(a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
    assert!(e.state.seats[0].has_edge, "the bleed's");
}

/// Seat 0's Gangrel antitribu `g1` bleeds and `g2` stands by, beside a
/// third vampire; seat 1's `x`, Saku Pihlajamäki ("1 optional maneuver
/// each combat"), 6 blood, blocks.
fn alpha_pack() -> (Engine, MinionId, MinionId, MinionId) {
    let mut e = game(2);
    let g1 = e.add_minion(SeatId(0), of_clan("Gangrel antitribu", 4..=11, 0), 4, false);
    let g2 = e.add_minion(SeatId(0), of_clan("Gangrel antitribu", 4..=11, 1), 4, false);
    e.add_minion(SeatId(0), of_clan("Ventrue", 4..=11, 0), 4, false);
    let x = e.add_minion(SeatId(1), find_card("Saku Pihlajamäki"), 6, false);
    e.add_minion(SeatId(1), of_clan("Brujah", 4..=11, 1), 4, false);
    (e, g1, g2, x)
}

/// `m`'s own printed ability.
fn printed_use(m: MinionId) -> impl Fn(&Choice) -> bool {
    move |o| {
        matches!(o, Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Vampire(v), ..
        } if *v == m)
    }
}

/// Follow the Alpha: "… when combat involving another Gangrel antitribu
/// you control ends and the opposing minion is still ready. Lock this
/// Gangrel antitribu. This Gangrel antitribu enters combat with the
/// opposing minion. The first round of this new combat, the opposing
/// minion cannot use maneuvers." Its gun's maneuver offered in the first
/// combat, none in the new one's first round.
#[test]
fn follow_the_alpha_takes_over_the_combat_and_bars_maneuvers() {
    let (mut e, g1, g2, x) = alpha_pack();
    hand(&mut e, 0, &["Follow the Alpha"]);
    bleed_blocked(&mut e, g1, x);
    let req = request(&mut e);
    assert!(req.options.iter().any(printed_use(x)), "{req:?}");
    pick_when(&mut e, play(0, 0, g2));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, printed_use(x));
    assert!(e.state.minions[g2.index()].locked);
    assert_eq!(blood(&e, g2), 4 - 1, "the new combat's hand strike");
    assert_eq!(blood(&e, x), 6 - 1 - 1, "a hand strike in each combat");
}

/// Follow the Alpha: "Effects that end combat and then do something else
/// after combat … will be lost, except for unlock effects" (LSJ 19980109,
/// RTR 20020501): Legacy of Power's torpor after the combat it ended is
/// lost to the new combat, and "If the combat continues or a new combat
/// begins …, the vampires do not go into torpor" (RTR 20020501, LSJ
/// 20021126).
/// Ledger: [LSJ 19980109] [RTR 20020501].
#[test]
fn follow_the_alpha_loses_the_after_combat_effects() {
    let (mut e, g1, g2, x) = alpha_pack();
    let p = legacy_prince(&mut e, 1);
    hand(&mut e, 0, &["Follow the Alpha"]);
    hand(&mut e, 1, &["Legacy of Power"]);
    bleed_blocked(&mut e, g1, x);
    pick_when(&mut e, play(0, 0, p));
    pick_when(&mut e, play(0, 0, g2));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[g1.index()].in_torpor);
    assert!(!e.state.minions[x.index()].in_torpor);
    assert!(e.state.minions[p.index()].locked);
}

/// Follow the Alpha: "Cannot be used to start a new combat if there is
/// already a pending combat queued" (RTR 20020501): Psyche! [CEL] played
/// first in the combat's end, it is not offered there.
/// Ledger: [RTR 20020501].
#[test]
fn follow_the_alpha_is_not_offered_with_a_combat_queued() {
    let (mut e, g1, g2, x) = alpha_pack();
    e.state.minions[g1.index()].gained_levels = vec!["cel", "cel"];
    hand(&mut e, 0, &["Psyche!", "Follow the Alpha"]);
    bleed_blocked(&mut e, g1, x);
    let req = offering(&mut e, play(0, 1, g1));
    assert!(req.options.contains(&play(1, 0, g2)), "offered before");
    e.choose(index_of(&req, play(0, 1, g1)));
    loop {
        let req = request(&mut e);
        if req.window != WindowLabel::CombatEndOfRound {
            break;
        }
        assert!(!req.options.contains(&play(0, 0, g2)), "{req:?}");
        e.choose(fallback(&req));
    }
}

/// A ready Camarilla vampire of `seat`'s, prince of a city, 4 blood.
fn legacy_prince(e: &mut Engine, seat: u8) -> MinionId {
    let p = e.add_minion(SeatId(seat), of_sect(Sect::Camarilla, 4..=11, 5), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Chicago"));
    p
}

/// Legacy of Power: "Lock this reacting vampire and end combat. Each of
/// the vampires involved in that combat goes to torpor." Played during
/// combat, as it begins (ANK 20200109): seat 1's prince ends the combat of
/// its blocker; both vampires to torpor, no damage dealt, 3 blood paid. "May
/// be used if one of your vampires is in combat with an ally. In this
/// case, your vampire goes into torpor and the ally is unaffected" (RTR
/// 19960112).
/// Ledger: [RTR 19960112]; [ANK 20200109].
#[test]
fn legacy_of_power_ends_the_combat_and_sends_both_vampires_to_torpor() {
    for ally in [false, true] {
        let mut e = game(2);
        let a = if ally {
            let r = e.add_ally(SeatId(0), find_library("47th Street Royals"));
            e.add_minion(SeatId(0), of_clan("Ventrue", 4..=11, 0), 4, false);
            r
        } else {
            e.add_minion(SeatId(0), of_clan("Ventrue", 4..=11, 0), 4, false)
        };
        e.add_minion(SeatId(0), of_clan("Ventrue", 4..=11, 1), 4, false);
        let x = e.add_minion(SeatId(1), of_clan("Brujah", 4..=11, 0), 4, false);
        let p = legacy_prince(&mut e, 1);
        hand(&mut e, 1, &["Legacy of Power"]);
        bleed_blocked(&mut e, a, x);
        pick_when(&mut e, play(0, 0, p));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let (va, vx) = (&e.state.minions[a.index()], &e.state.minions[x.index()]);
        assert!(vx.in_torpor && blood(&e, x) == 4);
        if ally {
            assert!(
                va.in_play && !va.in_torpor && va.blood == 2,
                "the ally untouched"
            );
        } else {
            assert!(va.in_torpor && va.blood == 4);
        }
        assert!(e.state.minions[p.index()].locked);
        assert_eq!(blood(&e, p), 4 - 3);
    }
}

/// Legacy of Power: "If the combat continues or a new combat begins (eg.
/// Psyche!), the vampires do not go into torpor" (RTR 20020501, LSJ
/// 20021126): Psyche! [CEL] played as the ended combat would end.
/// Ledger: [RTR 20020501] [LSJ 20021126].
#[test]
fn legacy_of_power_loses_its_torpor_to_a_new_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ventrue", 4..=11, 0), 4, false);
    e.add_minion(SeatId(0), of_clan("Ventrue", 4..=11, 1), 4, false);
    let x = e.add_minion(SeatId(1), of_clan("Brujah", 4..=11, 0), 4, false);
    let p = legacy_prince(&mut e, 1);
    e.state.minions[a.index()].gained_levels = vec!["cel", "cel"];
    hand(&mut e, 0, &["Psyche!"]);
    hand(&mut e, 1, &["Legacy of Power"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, p));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].in_torpor);
    assert!(!e.state.minions[x.index()].in_torpor);
    assert!(e.state.minions[p.index()].locked);
}

/// Nightshades: "[obt] Only usable as the action is announced. Choose an
/// ally or younger vampire. The chosen minion gets -1 intercept. [OBT] As
/// above, but usable if a minion attempts to block." The younger blocker's
/// 1 intercept against the bleed's 1 stealth: with the card it falls short
/// (the bleed lands), without it the block is made.
#[test]
fn nightshades_lowers_the_chosen_younger_vampires_intercept() {
    for mode in [None, Some(0), Some(1)] {
        let mut e = game(2);
        let a = e.add_minion(
            SeatId(0),
            textless(|c| (6..=11).contains(&c.capacity), 0),
            4,
            false,
        );
        e.add_minion(
            SeatId(0),
            textless(|c| (6..=11).contains(&c.capacity), 1),
            4,
            false,
        );
        let x = e.add_minion(
            SeatId(1),
            textless(|c| (2..=4).contains(&c.capacity), 2),
            4,
            false,
        );
        e.add_minion(
            SeatId(1),
            textless(|c| (2..=4).contains(&c.capacity), 3),
            4,
            false,
        );
        e.state.minions[a.index()].gained_levels = vec!["obt", "obt"];
        stealthy(&mut e, a);
        bonus(&mut e, x, zvtes::cards::Stat::Intercept);
        hand(&mut e, 0, &["Nightshades"]);
        pick(&mut e, bleed(a));
        let at = |mode| Choice::Play {
            card: 0,
            mode,
            actor: Some(a),
            target: Target::Minion(x),
        };
        match mode {
            Some(0) => pick(&mut e, at(0)),
            Some(_) => {
                pick_when(&mut e, block(x));
                pick_when(&mut e, at(1));
            }
            None => pick_when(&mut e, block(x)),
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e)[1], if mode.is_some() { 9 } else { 10 }, "{mode:?}");
    }
}

/// Permanency [chi]: "Only usable after this vampire plays an action
/// modifier card requiring Chimerstry [chi] that is not canceled as it is
/// played. Move that action modifier card from your ash heap to your hand
/// (discard down afterward)." Fata Morgana [CHI] resolved, Permanency
/// brings it back; with no such modifier played it is not offered.
#[test]
fn permanency_returns_the_chimerstry_modifier_just_played() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["chi", "chi"];
    hand(&mut e, 0, &["Fata Morgana", "Permanency"]);
    pick(&mut e, bleed(a));
    let req = offering(&mut e, play(0, 1, a));
    assert!(
        !req.options.contains(&play(1, 0, a)),
        "no modifier played yet"
    );
    e.choose(index_of(&req, play(0, 1, a)));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let fata = find_library("Fata Morgana");
    assert!(e.state.seats[0].hand.contains(&fata));
    assert!(!e.state.seats[0].ash.contains(&fata));
    assert_eq!(pool(&e)[1], 10 - 2, "the bleed for 2");
    assert!(ash_has(&e, 0, "Permanency"));
}

/// Permanency [CHI]: "Only usable if an action card requiring Chimerstry
/// is put in play. Put this card on the action card. If the attached card
/// would be burned, instead return it to your hand." Fatuus Mastery [chi]
/// put on its vampire, Permanency on it; the vampire burned, Fatuus
/// Mastery goes to the hand, Permanency (on it as it leaves play) to the
/// ash heap.
#[test]
fn permanency_superior_returns_its_action_card_to_hand_instead_of_burning() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["chi", "chi"];
    hand(&mut e, 0, &["Fatuus Mastery", "Permanency"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let fatuus = placed_on(&e, "Fatuus Mastery", Some(a));
    let perm = placed_on(&e, "Permanency", Some(a));
    assert_eq!(e.state.cards[perm.index()].on_card, Some(fatuus));
    e.burn(a);
    assert!(e.state.seats[0]
        .hand
        .contains(&find_library("Fatuus Mastery")));
    assert!(!ash_has(&e, 0, "Fatuus Mastery"));
    assert!(ash_has(&e, 0, "Permanency"));
}

/// Seat 0's `a` (capacity 6+, 4 blood) bleeds, beside a second vampire;
/// seat 1's younger `x` blocks, beside a second one.
fn younger_blocks(levels: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| (6..=11).contains(&c.capacity), 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        textless(|c| (6..=11).contains(&c.capacity), 1),
        4,
        false,
    );
    let x = e.add_minion(
        SeatId(1),
        textless(|c| (2..=4).contains(&c.capacity), 2),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| (2..=4).contains(&c.capacity), 3),
        4,
        false,
    );
    e.state.minions[a.index()].gained_levels = levels.to_vec();
    (e, a, x)
}

/// Psychomachia [pre]: "Unlock this vampire, and the action ends
/// (unsuccessfully)." The blocker is not locked (LSJ 20030902-1); the
/// action reached resolution, NRA (RTR 20020501, LSJ 20070709): the
/// unlocked vampire is offered no second bleed; 1 blood paid.
/// Ledger: [RTR 20020501] [LSJ 20070709].
#[test]
fn psychomachia_presence_ends_the_blocked_action_unlocked() {
    let (mut e, a, x) = younger_blocks(&["pre"]);
    hand(&mut e, 0, &["Psychomachia"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::MinionPhase)
    );
    assert!(!e.state.minions[a.index()].locked);
    assert!(!e.state.minions[x.index()].locked, "the blocker not locked");
    assert!(!req.options.contains(&bleed(a)), "NRA");
    assert_eq!(blood(&e, a), 4 - 1);
    assert_eq!(pool(&e)[1], 10);
}

/// Psychomachia [dai]: "The block fails, and the action continues. The
/// blocking minion cannot attempt to block this action again." [DAI]: "…
/// and the blocking minion takes 1 unpreventable environmental damage."
/// The bleed lands, the blocker unlocked (LSJ 20030902-1), never offered
/// the block again; an older blocker is no target.
/// Ledger: [LSJ 20030902-1].
#[test]
fn psychomachia_daimoinon_fails_the_block_and_hurts_the_blocker() {
    for superior in [false, true] {
        let (mut e, a, x) = younger_blocks(if superior { &["dai", "dai"] } else { &["dai"] });
        hand(&mut e, 0, &["Psychomachia"]);
        bleed_blocked(&mut e, a, x);
        let before = blood(&e, x);
        pick_when(&mut e, play(0, if superior { 2 } else { 1 }, a));
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(x));
        assert_eq!(pool(&e)[1], 9, "the action continued");
        assert!(!e.state.minions[x.index()].locked);
        assert_eq!(blood(&e, x), before - u8::from(superior));
    }
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| (2..=4).contains(&c.capacity), 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        textless(|c| (2..=4).contains(&c.capacity), 1),
        4,
        false,
    );
    let x = e.add_minion(
        SeatId(1),
        textless(|c| (6..=11).contains(&c.capacity), 2),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| (6..=11).contains(&c.capacity), 3),
        4,
        false,
    );
    e.state.minions[a.index()].gained_levels = vec!["dai"];
    hand(&mut e, 0, &["Psychomachia"]);
    bleed_blocked(&mut e, a, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
}

/// Purification [qui][ACTION]: "+1 stealth action. Add 1 life to an ally
/// who has fewer life than their starting life." The player's Royals at 1
/// of 2 life heal to 2.
#[test]
fn purification_heals_a_hurt_ally() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let r = e.add_ally(SeatId(0), find_library("47th Street Royals"));
    e.state.minions[r.index()].blood = 1;
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["qui"];
    hand(&mut e, 0, &["Purification"]);
    pick(&mut e, play_at(0, a, r));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, r), 2);
}

/// Purification [QUI][REACTION]: "Only usable during an action requiring
/// Dominate or Presence against a minion you control. The action fails."
/// Steal the Mind [dom]'s lock at seat 1's vampire fails: not locked, not
/// blocked (LSJ 20070219); a bleed (no Discipline) is no target.
#[test]
fn purification_fails_a_dominate_action_at_your_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let y = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let q = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dom"];
    e.state.minions[q.index()].gained_levels = vec!["qui", "qui"];
    hand(&mut e, 0, &["Steal the Mind", "Steal the Mind"]);
    hand(&mut e, 1, &["Purification"]);
    pick(&mut e, play_at(0, a, y));
    pick_when(&mut e, play(0, 1, q));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::MinionPhase)
    );
    assert!(!e.state.minions[y.index()].locked, "failed");
    assert!(e.state.seats[1].hand.is_empty());

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let q = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.state.minions[q.index()].gained_levels = vec!["qui", "qui"];
    hand(&mut e, 1, &["Purification"]);
    pick(&mut e, bleed(a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, q));
}

/// Rebirth: "Requires a Red List minion. Only usable when a diablerie
/// action is successful. No blood hunt may be called, and this vampire
/// unlocks. A vampire can play only one Rebirth each turn." A Red List
/// diablerist plays it after its diablerie: no blood hunt (seat 1's Edge
/// would have passed it), unlocked; its second diablerie this turn gets no
/// second Rebirth; a vampire off the Red List gets none.
#[test]
fn rebirth_bars_the_blood_hunt_and_unlocks_the_red_list_diablerist() {
    for red in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 2, false);
        e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        let t = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 0), 0, true);
        let t2 = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 1), 0, true);
        e.state.minions[a.index()].made_red_list = red;
        e.state.seats[1].has_edge = true;
        hand(&mut e, 0, &["Rebirth", "Rebirth"]);
        let diablerie = |t| Choice::TakeAction {
            minion: a,
            action: zvtes::decision::ActionKind::Diablerie,
            target: Some(t),
        };
        pick(&mut e, diablerie(t));
        if !red {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
            continue;
        }
        pick_when(&mut e, play(0, 0, a));
        let req = request(&mut e);
        assert_eq!(
            (req.seat, req.window),
            (SeatId(0), WindowLabel::MinionPhase)
        );
        assert!(e.state.minions[a.index()].in_play, "no blood hunt");
        assert!(!e.state.minions[a.index()].locked);
        assert!(!e.state.minions[t.index()].in_play, "diablerized");
        pick(&mut e, diablerie(t2));
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
    }
}

/// Rebirth: "Cannot be used following an Amaranth or an Abactor as they
/// are not diablerie actions" (ANK 20190701; LSJ 20090724, ANK
/// 20180129-1): neither offers it to a Red List vampire.
/// Ledger: [LSJ 20090724] [ANK 20180129-1]; [ANK 20190701].
#[test]
fn rebirth_is_not_offered_after_amaranth_or_abactor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), textless(|c| c.capacity <= 4, 0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[a.index()].made_red_list = true;
    hand(&mut e, 0, &["Amaranth", "Rebirth"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, a));
    assert!(
        !e.state.minions[b.index()].present(),
        "diablerized in combat"
    );

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), find_card("Tara"), 4, false);
    e.state.minions[a.index()].made_red_list = true;
    hand(&mut e, 0, &["Abactor", "Rebirth"]);
    pick(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, a));
}

/// Sins of the Cauchemar [AUS]: "Put this card on the acting vampire. You
/// still control this card. This reacting vampire gets +1 intercept when
/// the vampire with this card attempts to bleed you." With no block
/// attempt, by several minions on one action (LSJ 19990611): two copies
/// on the bleeder, each player's vampire reaching its 1 stealth, a third
/// not; [aus] +1 intercept.
/// Ledger: [LSJ 19990611].
#[test]
fn sins_of_the_cauchemar_gives_its_player_intercept_against_the_bearers_bleeds() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let r1 = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let r2 = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let r3 = e.add_minion(SeatId(1), plain("Brujah", 2), 4, false);
    for r in [r1, r2] {
        e.state.minions[r.index()].gained_levels = vec!["aus", "aus"];
    }
    stealthy(&mut e, a);
    hand(
        &mut e,
        1,
        &["Sins of the Cauchemar", "Sins of the Cauchemar"],
    );
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, r1));
    pick_when(&mut e, play(0, 1, r2));
    let req = offering(&mut e, block(r1));
    assert!(e.block_can_reach(req.seat, r1) && e.block_can_reach(req.seat, r2));
    assert!(!e.block_can_reach(req.seat, r3));
    let on: Vec<_> = e
        .state
        .cards
        .iter()
        .filter(|c| c.card == find_library("Sins of the Cauchemar") && c.in_play && !c.effect_only)
        .collect();
    assert_eq!(on.len(), 2);
    assert!(on.iter().all(|c| c.host == Some(a)));
    assert_eq!(
        e.state
            .card_controller(placed_on(&e, "Sins of the Cauchemar", Some(a))),
        SeatId(1)
    );
    assert_eq!(blood(&e, r1), 4 - 1);
}

/// Sins of the Cauchemar: "The vampire with this card can burn this card as
/// a Ⓓ action." Its bearer, not another vampire of its player's.
#[test]
fn sins_of_the_cauchemar_is_burned_by_its_bearers_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let sins = in_play_mode(&mut e, 1, "Sins of the Cauchemar", Some(a), 1);
    let req = request(&mut e);
    assert!(!req.options.contains(&granted(sins.0, b)));
    pick(&mut e, granted(sins.0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[sins.index()].in_play);
    assert!(ash_has(&e, 1, "Sins of the Cauchemar"));
}

/// Tainted Vitae: "Only usable if a vampire controlled by your predator is
/// about to successfully hunt. The hunting vampire gets +2 hunt this action
/// and put this card on them; you still control this card. The attached
/// vampire must burn 1 additional blood to announce a bleed action (if it
/// is not canceled as played) against you." A hunt for 3; the bearer's
/// next bleed of seat 1 burns 1 blood as announced, an empty bearer is
/// offered none; a hunt gaining nothing (diseased) is not successful (LSJ
/// 20050720-2): not offered.
/// Ledger: [LSJ 20050720-2].
#[test]
fn tainted_vitae_feeds_the_predators_hunter_and_taxes_its_bleeds() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ventrue", 6..=11, 0), 1, false);
    e.add_minion(SeatId(0), of_clan("Ventrue", 4..=11, 1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ministry", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ministry", 1), 4, false);
    hand(&mut e, 1, &["Tainted Vitae"]);
    pick(&mut e, action(a, zvtes::decision::ActionKind::Hunt));
    let req = offering(&mut e, play(0, 0, v));
    assert_eq!(req.window, WindowLabel::ActionBlocksDeclined);
    e.choose(index_of(&req, play(0, 0, v)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 1 + 3);
    assert_eq!(blood(&e, v), 4 - 2);
    let tv = placed_on(&e, "Tainted Vitae", Some(a));
    assert_eq!(e.state.card_controller(tv), SeatId(1));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a));
    request(&mut e);
    assert_eq!(blood(&e, a), 4 - 1, "burned to announce");
    // Two copies, 2 blood to announce: with 1, no bleed offered.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ventrue", 6..=11, 0), 1, false);
    e.add_minion(SeatId(0), of_clan("Ventrue", 4..=11, 1), 4, false);
    e.add_minion(SeatId(1), plain("Ministry", 0), 4, false);
    in_play(&mut e, 1, "Tainted Vitae", Some(a));
    in_play(&mut e, 1, "Tainted Vitae", Some(a));
    let req = request(&mut e);
    assert!(!req.options.contains(&bleed(a)), "{req:?}");

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ventrue", 6..=11, 0), 1, false);
    e.add_minion(SeatId(0), of_clan("Ventrue", 4..=11, 1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ministry", 0), 4, false);
    e.state.minions[a.index()].disease = true;
    hand(&mut e, 1, &["Tainted Vitae"]);
    pick(&mut e, action(a, zvtes::decision::ActionKind::Hunt));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, v));
}

/// Undying Thirst: "Only usable if this Baali blocks, before block
/// resolution. Put this card on the acting vampire." Seat 1's Baali blocks
/// and puts it on the bleeder, controlled by its bearer's Methuselah.
#[test]
fn undying_thirst_goes_on_the_blocked_vampire() {
    let (mut e, a, x) = baali_blocks(2..=4, 6..=11);
    hand(&mut e, 1, &["Undying Thirst"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, x));
    request(&mut e);
    let ut = placed_on(&e, "Undying Thirst", Some(a));
    assert_eq!(e.state.card_controller(ut), SeatId(0));
}

/// Undying Thirst: "If there are any vampires in torpor that the attached
/// vampire can diablerize, they cannot perform any action other than
/// diablerie (although they are free not to perform any action). If they
/// must hunt, they can choose to diablerize a vampire instead." With a
/// torpid vampire, its only actions are diableries; with none, a bleed
/// again. Empty, "he must either hunt or diablerize, this is a mandatory
/// action" (LSJ 20081213-2). "Has no effect on a vampire that cannot
/// commit diablerie (eg. because of Humanitas)" (LSJ 20050324-2).
/// Ledger: [LSJ 20081213-2]; [LSJ 20050324-2].
#[test]
fn undying_thirst_leaves_only_diablerie_while_a_victim_lies_in_torpor() {
    let acts =
        |req: &zvtes::decision::DecisionRequest, m: MinionId| -> Vec<zvtes::decision::ActionKind> {
            req.options
                .iter()
                .filter_map(|o| match *o {
                    Choice::TakeAction { minion, action, .. } if minion == m => Some(action),
                    Choice::Play { actor: Some(x), .. } if x == m => {
                        Some(zvtes::decision::ActionKind::Card)
                    }
                    _ => None,
                })
                .collect()
        };
    use zvtes::decision::ActionKind as K;
    // (torpid victim, blood, Humanitas)
    for (victim, blood_left, humane) in [
        (true, 4, false),
        (false, 4, false),
        (true, 0, false),
        (true, 4, true),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Ventrue", 0), blood_left, false);
        e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        if victim {
            e.add_minion(SeatId(1), plain("Brujah", 1), 1, true);
        }
        in_play(&mut e, 0, "Undying Thirst", Some(a));
        if humane {
            in_play(&mut e, 0, "Humanitas", Some(a));
        }
        hand(&mut e, 0, &["Govern the Unaligned"]);
        e.state.minions[a.index()].gained_levels = vec!["dom"];
        let req = request(&mut e);
        let mut got = acts(&req, a);
        got.sort_by_key(|k| *k as u8);
        got.dedup();
        let case = format!("victim {victim}, blood {blood_left}, Humanitas {humane}");
        match (victim, blood_left, humane) {
            (true, 0, _) => assert_eq!(got, vec![K::Hunt, K::Diablerie], "{case}"),
            (true, _, false) => assert_eq!(got, vec![K::Diablerie], "{case}"),
            _ => {
                assert!(
                    got.contains(&K::Bleed) && got.contains(&K::Card),
                    "{case}: {got:?}"
                );
                assert!(!got.contains(&K::Diablerie), "{case}: {got:?}");
            }
        }
    }
}

/// Veiled Sight [chi]: "Only usable by a blocking vampire. Reduce the
/// acting minion's stealth to 0." A blocking vampire is one attempting to
/// block (rules-qa 121.123; owner, 2026-11-19): not offered before X's
/// attempt, nor ever to a third Methuselah's vampire that cannot block the
/// bleed; played by X attempting, the 1-stealth bleed is blocked with no
/// intercept; without it, the attempt falls short and the bleed lands.
#[test]
fn veiled_sight_zeroes_stealth_for_the_vampire_attempting_to_block() {
    for played in [true, false] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
        let x = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(2), plain("Tremere", 0), 4, false);
        e.state.minions[x.index()].gained_levels = vec!["chi"];
        e.state.minions[z.index()].gained_levels = vec!["chi"];
        stealthy(&mut e, a);
        hand(&mut e, 1, &["Veiled Sight"]);
        hand(&mut e, 2, &["Veiled Sight"]);
        pick(&mut e, bleed(a));
        let req = offering(&mut e, block(x));
        assert!(!req.options.contains(&play(0, 0, x)), "no attempt yet");
        e.choose(index_of(&req, block(x)));
        if played {
            pick_when(&mut e, play(0, 0, x));
        }
        let by_z = |o: &Choice| matches!(o, Choice::Play { actor: Some(m), .. } if *m == z);
        never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, by_z);
        assert_eq!(pool(&e)[1], if played { 10 } else { 9 });
    }
}

/// Veiled Sight [CHI]: "… this blocking vampire can strike: steal 1 blood
/// or life (becoming blood) during the resulting combat if they block."
/// Usable once, on any round (RTR 19960112); on a minion with no blood
/// (RTR 20010711), stealing nothing; not ranged (TOM 19951215-2): at long
/// range it steals nothing.
/// Ledger: [RTR 20010711]; [TOM 19951215-2].
#[test]
fn veiled_sight_superior_steals_once_in_the_block_combat() {
    let steal = Choice::RoundStrike { at: None };
    for (acting_blood, long) in [(3, false), (0, false), (3, true)] {
        let mut e = game(2);
        // Saku Pihlajamäki: "1 optional maneuver each combat".
        let a = if long {
            e.add_minion(SeatId(0), find_card("Saku Pihlajamäki"), 4, false)
        } else {
            e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false)
        };
        e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
        let x = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        e.state.minions[x.index()].gained_levels = vec!["chi", "chi"];
        hand(&mut e, 1, &["Veiled Sight"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(x));
        pick_when(&mut e, play(0, 1, x));
        e.state.minions[a.index()].blood = acting_blood;
        if long {
            pick_matching(
                &mut e,
                |o| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(v), .. } if *v == a),
            );
            offered_before(&mut e, steal, SeatId(0), WindowLabel::MinionPhase);
        } else {
            pick_when(&mut e, steal);
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        // Its cost, then the hand strike landing at close range.
        let (hit, stolen) = if long {
            (0, 0)
        } else {
            (1, u8::from(acting_blood > 0))
        };
        assert_eq!(blood(&e, x), 4 - 2 - hit + stolen, "{acting_blood} {long}");
        assert_eq!(blood(&e, a), acting_blood - stolen, "{acting_blood} {long}");
    }
    // Unused in the first round, offered in the second, Abaddon pressing
    // ("1 optional press each combat"); used, no more.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let x = e.add_minion(SeatId(1), find_card("Abaddon"), 6, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["chi", "chi"];
    hand(&mut e, 1, &["Veiled Sight"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 1, x));
    let req = offering(&mut e, steal);
    e.choose(index_of(&req, Choice::HandStrike));
    pick_matching(
        &mut e,
        |o| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(v), ability: 1, .. } if *v == x),
    );
    pick_when(&mut e, steal);
    assert_eq!(e.combat().map(|c| c.round), Some(2));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, steal);
    assert_eq!(blood(&e, a), 4 - 1 - 1, "a hand strike, then the steal");
}

/// Weigh the Heart [aus][ser]: "Only usable during a bleed action. +1 bleed
/// (limited)." [AUS][SER]: "Only usable as a directed action is announced.
/// After action resolution, if this action was successful, put 1
/// corruption counter on a minion controlled by (any of) the target
/// Methuselah(s)." The minion chosen then, one of the target's only (ANK
/// 20180515): seat 1's two, not seat 0's or seat 2's; a blocked bleed puts
/// none.
/// Ledger: [ANK 20180515].
#[test]
fn weigh_the_heart_corrupts_a_targets_minion_after_a_successful_action() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["aus", "ser"];
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Weigh the Heart"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 2);

    for blocked in [false, true] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["aus", "aus", "ser", "ser"];
        let y = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(2), plain("Gangrel", 0), 4, false);
        hand(&mut e, 0, &["Weigh the Heart"]);
        pick(&mut e, bleed(a));
        pick(&mut e, play(0, 1, a));
        if blocked {
            pick_when(&mut e, block(y));
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
                matches!(o, Choice::ChooseMinion(_))
            });
            assert!(e.state.minions.iter().all(|v| v.corruption.is_empty()));
            continue;
        }
        let req = offering(&mut e, Choice::ChooseMinion(z));
        assert_eq!(req.seat, SeatId(0));
        assert_eq!(
            req.options,
            vec![Choice::ChooseMinion(y), Choice::ChooseMinion(z)]
        );
        e.choose(index_of(&req, Choice::ChooseMinion(z)));
        assert_eq!(e.state.minions[z.index()].corruption, vec![(SeatId(0), 1)]);
        assert_eq!(pool(&e)[1], 10 - 1);
    }
}

/// Ilomba under Weather Control [THA]'s second round: "If Ilomba would burn
/// a life …, this minion burns a blood or life instead", each point of the
/// 2 (ANK 20180525): its host pays 1 then 2 (fix: one point only before).
#[test]
fn ilombas_host_pays_each_point_of_weather_control() {
    let mut e = game(2);
    let big = |c: &zvtes::crypt::CryptCard| c.capacity >= 9;
    let a = e.add_minion(SeatId(0), fixture_vampire(big, 0), 9, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let z = e.add_minion(SeatId(1), fixture_vampire(big, 1), 11, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tha", "tha"];
    in_play(&mut e, 1, "Ilomba", Some(z));
    request(&mut e);
    let zs = blood(&e, z);
    hand(&mut e, 0, &["Weather Control", "Apportation"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, z),
        zs - (1 + 2) - 2 - (1 + 2),
        "weather, hand strikes, Ilomba"
    );
}

// ---- Per-count votes read through the polling (owner, 2026-11-19) ----------

/// The running tally of the referendum under way.
fn running(e: &Engine) -> (u32, u32) {
    e.referendum().expect("the referendum").tally(&e.state)
}

/// Seat 0's `caller` calls Political Stranglehold; seat 0 makes `uses`
/// then `caller` casts for; `change` then moves what a per-count source
/// counts: the votes for move by `by`, read again.
fn live_votes(
    e: &mut Engine,
    caller: MinionId,
    uses: &[Choice],
    change: impl FnOnce(&mut Engine),
    by: i32,
) {
    hand(e, 0, &["Political Stranglehold"]);
    pick(e, play(0, 0, caller));
    to_referendum(e);
    for &u in uses {
        pick_when(e, u);
    }
    pick_when(e, votes(caller, true));
    request(e);
    let before = running(e).0 as i32;
    change(e);
    assert_eq!(running(e).0 as i32, before + by);
}

/// A vampire's own ability (`AbilitySource::Vampire`).
fn own_ability(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(m),
        ability,
        actor: None,
        target,
    }
}

/// Aisha az-Zahra's +1 vote per ready Anarch, used before she casts: an
/// Anarch of hers sent to torpor after the cast takes one off (owner,
/// 2026-11-19: per-count votes read live through the polling; corrects
/// rules-qa line 322's "as it resolves").
#[test]
fn aisha_az_zahra_counts_her_anarchs_again_at_the_tally() {
    let mut e = game(2);
    let aisha = e.add_minion(SeatId(0), find_card("Aisha az-Zahra"), 4, false);
    let anita = e.add_minion(SeatId(0), find_card("Anita Wainwright"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    live_votes(
        &mut e,
        aisha,
        &[own_ability(aisha, 0, Target::None)],
        |e| e.state.minions[anita.index()].in_torpor = true,
        -1,
    );
}

/// Powerbase: Madrid's +1 vote per counter to a titled Sabbat vampire: a
/// counter added after the cast adds a vote (owner, 2026-11-19).
#[test]
fn powerbase_madrid_counts_its_counters_again_at_the_tally() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    e.state.minions[v.index()].granted_title = Some(Title::Bishop);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let madrid = in_play(&mut e, 0, "Powerbase: Madrid", None);
    e.state.cards[madrid.index()].counters = 2;
    let lock = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(madrid),
        ability: 1,
        actor: None,
        target: Target::Minion(v),
    };
    live_votes(
        &mut e,
        v,
        &[lock],
        |e| e.state.cards[madrid.index()].counters = 3,
        1,
    );
}

/// Bastille Opera House: +1 vote per ready Daughter of Cacophony, read
/// again (owner, 2026-11-19, overturning rules-qa 124.27's "counted as the
/// lock resolves"): one torpid as it is locked (1 vote), back out of
/// torpor by the tally (2); none ready then, 0.
#[test]
fn bastille_opera_house_counts_its_daughters_again_at_the_tally() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.state.minions[a.index()].granted_title = Some(Title::Primogen);
    let doc = |e: &mut Engine, nth| {
        let m = e.add_minion(SeatId(0), plain("Toreador", nth), 4, false);
        e.state.minions[m.index()].clan_change = Some("Daughter of Cacophony");
        m
    };
    let (d1, d2) = (doc(&mut e, 0), doc(&mut e, 1));
    e.state.minions[d2.index()].in_torpor = true;
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let b = in_play(&mut e, 0, "Bastille Opera House", None);
    let per = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(b),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    live_votes(
        &mut e,
        a,
        &[per],
        |e| e.state.minions[d2.index()].in_torpor = false,
        1,
    );
    let (favor, _) = running(&e);
    e.state.minions[d1.index()].in_torpor = true;
    e.state.minions[d2.index()].in_torpor = true;
    assert_eq!(running(&e).0, favor - 2);
}

/// Ayo Igoli's printed vote per ready titled vampire of his: a magaji of
/// his sent to torpor after he cast takes one off (owner, 2026-11-19).
#[test]
fn ayo_igoli_counts_his_titled_vampires_again_at_the_tally() {
    let mut e = game(2);
    let ayo = e.add_minion(SeatId(0), find_card("Ayo Igoli"), 4, false);
    e.add_minion(SeatId(0), find_card("Sobayifa"), 4, false);
    let t = e.add_minion(SeatId(0), find_card("Tsunda"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    live_votes(
        &mut e,
        ayo,
        &[],
        |e| e.state.minions[t.index()].in_torpor = true,
        -1,
    );
}

/// Gerald Windham's printed vote per ready titled vampire of the other
/// Methuselahs: another titled one after he cast adds one (owner,
/// 2026-11-19).
#[test]
fn gerald_windham_counts_the_titled_vampires_again_at_the_tally() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Gerald Windham"), 4, false);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    let q = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    live_votes(
        &mut e,
        g,
        &[],
        |e| e.state.minions[q.index()].granted_title = Some(Title::Primogen),
        1,
    );
}

/// Carter Vanderweyden's printed vote per other primogen of his, in the
/// referendum he calls: one losing the title after he cast takes one off
/// (owner, 2026-11-19).
#[test]
fn carter_vanderweyden_counts_his_primogens_again_at_the_tally() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Carter Vanderweyden"), 4, false);
    let p = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Primogen);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    live_votes(
        &mut e,
        c,
        &[],
        |e| e.state.minions[p.index()].granted_title = None,
        -1,
    );
}

/// Xeper's printed vote per ready unique infernal vampire of his: Giotto
/// Verducci sent to torpor after he cast takes one off (owner,
/// 2026-11-19).
#[test]
fn xeper_counts_his_infernals_again_at_the_tally() {
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), find_card("Xeper, Sultan of Lepers"), 5, false);
    let g = e.add_minion(SeatId(0), find_card("Giotto Verducci"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    live_votes(
        &mut e,
        x,
        &[],
        |e| e.state.minions[g.index()].in_torpor = true,
        -1,
    );
}

// ---- Readings the round's grading found untested ----------------------------

/// The Final Nights canceled by Delaying Tactics: no tally, so neither its
/// pass (each vampire 1 blood) nor its fail rider (the acting vampire's 1)
/// applies (LSJ 20010209; rules-qa 43.1).
#[test]
fn the_final_nights_canceled_burns_nothing() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    let b = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 1), 4, false);
    let z = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    hand(&mut e, 0, &["Final Nights, The"]);
    hand(&mut e, 1, &["Delaying Tactics"]);
    pick(&mut e, play(0, 0, a));
    to_referendum(&mut e);
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Delaying Tactics")));
    assert_eq!(
        [a, b, z].map(|m| blood(&e, m)),
        [4, 4, 3],
        "Delaying Tactics' 1 blood only"
    );
}

/// Gangrel Justicar: a Gangrel only considered Camarilla (an Anarch under a
/// Writ of Acceptance) is a Camarilla Gangrel to choose (the effective
/// sect, rules-qa 111.54); without the Writ the card is not played.
#[test]
fn gangrel_justicar_chooses_a_gangrel_considered_camarilla() {
    for writ in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
        e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 1), 4, false);
        let g = e.add_minion(
            SeatId(1),
            textless(|c| c.clan == "Gangrel" && c.sect == Some(Sect::Anarch), 0),
            4,
            false,
        );
        if writ {
            equipped(&mut e, "Writ of Acceptance", g);
        }
        hand(&mut e, 0, &["Gangrel Justicar"]);
        assert_eq!(request(&mut e).options.contains(&play(0, 0, a)), writ);
        if writ {
            pass_referendum(&mut e, a, &[]);
            request(&mut e);
            placed_on(&e, "Gangrel Justicar", Some(g));
            assert_eq!(e.state.title_of(g), Title::Justicar("Gangrel"));
        }
    }
}

/// Ravnos Acceptance: a Sabbat bishop Ravnos turning Camarilla keeps the
/// title but loses its benefit (advanced rules §5.8.2: `title_of`'s sect
/// fit). The card in play leaves with its ousted controller (rules-digest
/// D5).
#[test]
fn ravnos_acceptance_idles_an_off_sect_title_and_leaves_with_its_ousted_player() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    let r = e.add_minion(SeatId(0), of_clan("Ravnos", 3..=8, 0), 4, false);
    e.state.minions[r.index()].sect_override = Some(Sect::Sabbat);
    e.state.minions[r.index()].granted_title = Some(Title::Bishop);
    e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 1), 4, false);
    assert_eq!(e.state.title_of(r), Title::Bishop);
    hand(&mut e, 0, &["Ravnos Acceptance"]);
    pass_referendum(&mut e, a, &[]);
    pick_when(&mut e, Choice::ChooseMinion(r));
    request(&mut e);
    assert_eq!(e.state.sect_of(r), Some(Sect::Camarilla));
    assert_eq!(e.state.title_of(r), Title::None);
    // Seat 1's copy, seat 1 ousted by a bleed.
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    stealthy(&mut e, a);
    let card = in_play(&mut e, 1, "Ravnos Acceptance", None);
    e.state.seats[1].pool = 1;
    pick(&mut e, bleed(a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert!(!e.state.cards[card.index()].in_play);
}

/// Praxis Seizure: Barcelona's +1 capacity counts for every reader (a
/// vampire of the printed capacity is now younger, rules-digest D30);
/// raised, no blood is filled; lost to a contest, the excess drains.
#[test]
fn praxis_seizure_capacity_counts_everywhere_and_drains_when_lost() {
    let mut e = game(2);
    let tremere = textless(
        |c| c.clan == "Tremere" && c.sect == Some(Sect::Camarilla) && (4..=8).contains(&c.capacity),
        0,
    );
    let printed = CRYPT[tremere as usize].capacity;
    let a = e.add_minion(SeatId(0), tremere, printed, false);
    let peer = e.add_minion(
        SeatId(0),
        textless(|c| c.capacity == printed && c.clan != "Tremere", 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    assert!(!e.state.younger_than(peer, a));
    hand(&mut e, 0, &["Praxis Seizure: Barcelona"]);
    pass_referendum(&mut e, a, &[]);
    request(&mut e);
    assert!(e.state.younger_than(peer, a));
    assert_eq!(blood(&e, a), printed, "nothing filled");
    e.state.minions[a.index()].blood = printed + 1;
    let m = e.add_minion(SeatId(1), find_card("María del Toro"), 4, false);
    e.stage_contests(m);
    request(&mut e);
    assert_eq!(blood(&e, a), printed, "the excess drained");
}

/// Sabbat Threat: the caller may choose themselves (rules-qa 70.1); the
/// Edge burned outside the unlock phase (their minion phase) clears the
/// counters too.
#[test]
fn sabbat_threat_on_its_caller_cleared_by_the_edge_in_the_minion_phase() {
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Prince("Paris"));
    e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    hand(&mut e, 0, &["Sabbat Threat"]);
    pass_referendum(&mut e, a, &[Choice::ChooseSeat(SeatId(0)), Choice::Pass]);
    let k = stand_ins(&e, "Sabbat Threat")[0];
    assert_eq!(e.state.cards[k.index()].controller, SeatId(0));
    assert_eq!(e.state.cards[k.index()].counters, 1);
    e.state.seats[0].has_edge = true;
    pick_when(&mut e, ability(k.0, 0));
    request(&mut e);
    assert!(!e.state.seats[0].has_edge);
    assert!(!e.state.cards[k.index()].in_play);
}

/// Sacrifice: the chosen no longer of the acting vampire's clan at the
/// passage is no legal choice: not burned (rules-digest D1).
#[test]
fn sacrifice_spares_a_chosen_no_longer_legal() {
    let big = textless(|c| c.sect == Some(Sect::Sabbat) && c.capacity >= 8, 0);
    let clan = CRYPT[big as usize].clan;
    let small = |nth| {
        textless(
            |c| c.capacity <= 6 && c.clan != clan && c.sect == Some(Sect::Sabbat),
            nth,
        )
    };
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), big, 8, false);
    e.add_minion(SeatId(0), small(0), 4, false);
    let t = e.add_minion(SeatId(1), small(1), 3, false);
    e.state.minions[t.index()].clan_change = Some(clan);
    hand(&mut e, 0, &["Sacrifice"]);
    pick(&mut e, play(0, 0, a));
    // Its one choice, elided.
    to_referendum(&mut e);
    assert_eq!(e.referendum().expect("referendum").terms.minions, [t]);
    e.state.minions[t.index()].clan_change = None;
    pick_matching(&mut e, |o| {
        *o == Choice::CastCardVote { favor: true } || *o == votes(a, true)
    });
    close_polling(&mut e);
    request(&mut e);
    assert!(e.state.minions[t.index()].in_play);
}

/// Trumped-Up Charges' granted burn referendum: no card was played, so no
/// political-card vote (rules-qa 6.2).
#[test]
fn trumped_up_charges_burn_referendum_has_no_card_vote() {
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Prince("Paris"));
    let t = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 1), 4, false);
    let card = in_play(&mut e, 1, "Trumped-Up Charges", Some(t));
    pick(&mut e, granted(card.0, a));
    let req = offering(&mut e, votes(a, true));
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::CastCardVote { .. })));
}

/// Priority Shift: directed at another Methuselah's Red List minion, not at
/// one's own (rules-qa 20.1); a printed Red List lifted, a card making it
/// Red List again does (the texts); its target no longer Red List as it
/// resolves, the action fizzles (rules-digest D1).
#[test]
fn priority_shift_direction_relisting_and_fizzle() {
    let red = textless(|c| c.red_list && (4..=8).contains(&c.capacity), 0);
    let shift = |actor, at| Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(actor),
        target: Target::Minion(at),
    };
    for (own, seat) in [(true, 0u8), (false, 1)] {
        let mut e = game(2);
        let a = titled_caller(&mut e, Title::Prince("Paris"));
        let p = e.add_minion(SeatId(seat), red, 4, false);
        e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
        hand(&mut e, 0, &["Priority Shift"]);
        pick(&mut e, shift(a, p));
        request(&mut e);
        let directed = e.action().expect("action").directed_seat(&e.state);
        assert_eq!(directed, (!own).then_some(SeatId(1)), "own {own}");
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(!e.state.is_red_list(p));
        in_play(&mut e, 1, "Trumped-Up Charges", Some(p));
        assert!(e.state.is_red_list(p), "Red List again");
    }
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Prince("Paris"));
    let t = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=8, 0), 4, false);
    e.state.minions[t.index()].made_red_list = true;
    hand(&mut e, 0, &["Priority Shift"]);
    pick(&mut e, shift(a, t));
    request(&mut e);
    e.state.minions[t.index()].made_red_list = false;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
}

/// Samuel Haight paying his last life for +1 bleed: he burns, and the bleed
/// fails with its acting minion gone.
#[test]
fn samuel_haight_paying_his_last_life_burns_and_the_bleed_fails() {
    let mut e = game(2);
    let s = e.add_ally(SeatId(0), find_library("Samuel Haight"));
    e.state.minions[s.index()].blood = 1;
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = ally_card(&e, s);
    pick(&mut e, bleed(s));
    pick_when(&mut e, use_card(card, 0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[s.index()].in_play);
    assert_eq!(e.state.seats[1].pool, 10);
}

/// Harzomatuili paying his last life for +1 intercept: he burns, and the
/// block attempt fails: the bleed goes through.
#[test]
fn harzomatuili_paying_his_last_life_burns_and_the_block_fails() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    stealthy(&mut e, a);
    let h = e.add_ally(SeatId(1), find_library("Harzomatuili"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[h.index()].blood = 1;
    let card = ally_card(&e, h);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(h));
    pick_when(&mut e, use_card(card, 1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[h.index()].in_play);
    assert_eq!(e.state.seats[1].pool, 10 - 1);
}

/// Bloodstone: two copies on X put there by A: +4 intercept against it, no
/// limit (the text): A reaches X's 3-stealth bleed with both, not with one.
#[test]
fn bloodstone_two_copies_give_four_intercept() {
    for copies in [1, 2] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        for _ in 0..copies {
            let stone = equipped(&mut e, "Bloodstone", x);
            e.state.cards[stone.index()].by = Some(a);
        }
        for _ in 0..3 {
            stealthy(&mut e, x);
        }
        to_minion_phase(&mut e, SeatId(1));
        pick(&mut e, bleed(x));
        request(&mut e);
        assert_eq!(e.block_can_reach(SeatId(0), a), copies == 2, "{copies}");
    }
}

/// Bloodstone played at X, X gone before it resolves: the action's target
/// re-checked (rules-digest D1), it fizzles and the card is not in play.
#[test]
fn bloodstone_fizzles_with_its_recipient_gone() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Bloodstone"]);
    pick(&mut e, play_at(0, a, x));
    request(&mut e);
    e.burn(x);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert!(e
        .state
        .cards
        .iter()
        .all(|c| !(c.in_play && c.card == find_library("Bloodstone"))));
    assert!(ash_has(&e, 0, "Bloodstone"));
}

/// Caiaphas Smith in the combat his blocker X starts: his 1R strike and
/// his optional maneuver are offered; X sent to torpor in it is still
/// burned after it, and so is X when Caiaphas burns in the combat (LSJ
/// 20041008).
#[test]
fn caiaphas_strikes_at_range_and_his_blocker_burns_in_any_case() {
    for caiaphas_dies in [false, true] {
        let mut e = game(2);
        let c = e.add_ally(SeatId(0), find_library("Caiaphas Smith"));
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        if caiaphas_dies {
            e.state.minions[c.index()].blood = 1;
        }
        let card = ally_card(&e, c);
        bonus(&mut e, x, zvtes::cards::Stat::Intercept);
        hand(&mut e, 0, &["Leather Jacket"]);
        pick(&mut e, play(0, 0, c));
        pick_when(&mut e, block(x));
        if !caiaphas_dies {
            pick_when(&mut e, use_card(card, 1));
            pick_when(&mut e, use_card(card, 0));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[c.index()].in_play, !caiaphas_dies);
        assert!(!e.state.minions[x.index()].in_play, "burned");
    }
}

/// Ghoul Escort: employed by a vampire only (an ally is not offered it);
/// its cancel offered only for the block's resulting combat (Ohoyo
/// Hopoksia's stand-in combat is none, rules-qa 124.154) and only on a
/// vampire bearer (an ally bearing it is not, rules-digest D16).
#[test]
fn ghoul_escort_needs_a_vampire_and_the_blocks_own_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ally = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Ghoul Escort"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, a)));
    assert!(!req.options.contains(&play(0, 0, ally)));
    // Ohoyo's combat.
    let (mut e, a, x) = duel(&[], &[]);
    let escort = equipped(&mut e, "Ghoul Escort", a);
    e.state.cards[escort.index()].counters = 4;
    e.add_ally(SeatId(1), find_library("Ohoyo Hopoksia (Bastet)"));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    let req = expect(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    let i = req
        .options
        .iter()
        .position(|o| matches!(o, Choice::ResolveRider { .. }))
        .expect("Ohoyo's stand-in");
    e.choose(i);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::CancelCombat,
    );
    // An ally bearer.
    let (mut e, _, x) = duel(&[], &[]);
    let ally = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    let escort = equipped(&mut e, "Ghoul Escort", ally);
    e.state.cards[escort.index()].counters = 4;
    bleed_blocked(&mut e, ally, x);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::CancelCombat,
    );
}

/// Incriminating Videotape: its prey controlling no minion, nothing to
/// choose: not played (rules-qa 17.4 masks an illegal choice only).
#[test]
fn incriminating_videotape_not_played_with_a_minionless_prey() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Incriminating Videotape"]);
    let req = request(&mut e);
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::Play { card: 0, .. })));
}

/// Polaris Coach on an ally (rules-digest D16): no stealth for its
/// counters, no bar on blocking, and with no vampire to feed it, it burns
/// at its controller's unlock phase.
#[test]
fn polaris_coach_on_an_ally_gives_nothing_and_burns() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Gravebound Drone"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let coach = equipped(&mut e, "Polaris Coach", ally);
    e.state.cards[coach.index()].counters = 2;
    pick(&mut e, bleed(a));
    offering(&mut e, block(ally));
    let mut e2 = game(2);
    e2.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ally2 = e2.add_ally(SeatId(0), find_library("Gravebound Drone"));
    e2.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let coach2 = equipped(&mut e2, "Polaris Coach", ally2);
    e2.state.cards[coach2.index()].counters = 2;
    pick(&mut e2, bleed(ally2));
    never_offered_until(
        &mut e2,
        SeatId(1),
        WindowLabel::MinionPhase,
        use_card(coach2, 0),
    );
    to_minion_phase(&mut e2, SeatId(0));
    assert!(!e2.state.cards[coach2.index()].present(), "burned");
}

/// Powder of Rigidity's bar joins Pseudo-Blindness [CHI]'s (the bars a
/// union, `Side::barred_disciplines`): A barred from Chimerstry and
/// Obfuscate by X's block, then from Protean by X's burned Powder, plays
/// neither Amria [chi] nor Wolf Claws [pro].
#[test]
fn powder_of_rigidity_bar_joins_pseudo_blindness() {
    let (mut e, a, x) = duel(&["obf", "chi", "pro"], &["chi", "chi"]);
    let powder = equipped(&mut e, "Powder of Rigidity", x);
    hand(&mut e, 0, &["Elder Impersonation", "Amria", "Wolf Claws"]);
    hand(&mut e, 1, &["Pseudo-Blindness"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, x));
    pick_when(&mut e, use_card(powder, 0));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == a),
    );
}

/// Bay and Howl: a ready Gangrel antitribu with no blood moves nothing (a
/// legal no-op for it); the other's 1 still moves.
#[test]
fn bay_and_howl_moves_nothing_from_an_empty_gangrel_antitribu() {
    let mut e = game(2);
    let ga = |n| of_clan("Gangrel antitribu", 3..=11, n);
    let a = e.add_minion(SeatId(0), ga(0), 0, false);
    let b = e.add_minion(SeatId(0), ga(1), 3, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    uncontrolled(&mut e, 0, ga(2), 1);
    hand(&mut e, 0, &["Bay and Howl"]);
    e.at_master_phase();
    pick(&mut e, master_at(0, Target::Uncontrolled(0)));
    request(&mut e);
    assert_eq!([a, b].map(|m| blood(&e, m)), [0, 2]);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 2);
}

/// Betrayer (the red, built as is: the chosen is the in-play vampire, RTR
/// 19941109): its player influences out their own copy, contesting it
/// (Betrayer nullified), then yields: their copy burned, Betrayer goes on
/// and the named Methuselah burns again. A torpid chosen still costs its
/// controller the pool.
#[test]
fn betrayer_goes_on_when_its_player_yields_the_contest() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let card = e.state.minions[v.index()].crypt_card().expect("crypt card");
    let b = betray(&mut e, v);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 9);
    e.state.seats[0].uncontrolled.pop();
    let m = e.add_minion(SeatId(0), card, 4, false);
    e.stage_contests(m);
    assert!(e.state.minions[v.index()].contested);
    to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    pick(&mut e, Choice::YieldContest(zvtes::game::Contest::Card(m)));
    request(&mut e);
    assert!(!e.state.minions[m.index()].in_play);
    // Seat 1's copy returns in its unlock phase (its burn ordered first,
    // still nullified), Betrayer active again from then on.
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[v.index()].contested);
    assert_eq!(pool(&e)[1], 9);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 8);
    assert!(e.state.cards[b.index()].in_play);
    // A torpid chosen.
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    betray(&mut e, v);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 9);
}

/// City Gangrel Connections in Veles' Hunt's referendum, a political
/// action (rules-qa 116.88): its vote offered there.
#[test]
fn city_gangrel_connections_votes_in_veles_hunt() {
    let mut e = game(2);
    let g = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Gangrel" && c.capacity == 6, 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let y = e.add_minion(
        SeatId(1),
        textless(|c| c.sect == Some(Sect::Camarilla) && c.capacity == 4, 0),
        4,
        false,
    );
    let c = in_play(&mut e, 1, "City Gangrel Connections", None);
    hand(&mut e, 0, &["Veles' Hunt"]);
    pick(&mut e, play(0, 0, g));
    // Its one choice, elided.
    to_referendum(&mut e);
    assert_eq!(e.referendum().expect("referendum").terms.minions, [y]);
    pick_when(&mut e, ability(c.0, 1));
    assert_eq!(running(&e).1, 1);
}

/// Deal with the Devil canceled as played (Sudden Reversal): nothing
/// discarded or drawn but its own replacement, as any canceled card.
#[test]
fn deal_with_the_devil_canceled_is_replaced_normally() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cards = fillers(16);
    hand(&mut e, 0, &["Deal with the Devil"]);
    for &c in &cards[..6] {
        e.state.seats[0].hand.push(c);
    }
    for &c in &cards[6..] {
        e.state.seats[0].library.push(c);
    }
    hand(&mut e, 1, &["Sudden Reversal"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    pick_when(&mut e, master_play(0));
    request(&mut e);
    let s = &e.state.seats[0];
    assert!(s.ash.contains(&find_library("Deal with the Devil")));
    assert!(cards[..6].iter().all(|c| s.hand.contains(c)), "kept");
    assert_eq!(s.hand.len(), 7);
    assert_eq!(s.library.len(), 9);
}

/// Glass Walker Pact: a Hecata is no Giovanni (rules-qa 121.54), the
/// locked source no recipient of its own blood; with no other ready
/// Giovanni, the use still burns the library's top card and moves nothing.
#[test]
fn glass_walker_pact_needs_another_ready_giovanni() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), plain("Giovanni", 0), 1, false);
    let h = e.add_minion(SeatId(0), plain("Hecata", 0), 1, false);
    let l = e.add_minion(SeatId(1), plain("Giovanni", 1), 3, false);
    let p = in_play(&mut e, 0, "Glass Walker Pact", None);
    fill_hand(&mut e, 0);
    e.state.seats[0].library.push(find_library("Form of Mist"));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[l.index()].locked = true;
    pick_when(&mut e, use_on(p, 0, None, Target::Minion(l)));
    // Its one recipient, elided: neither the Hecata nor the source.
    request(&mut e);
    assert_eq!([g, h, l].map(|m| blood(&e, m)), [2, 1, 2]);
    // Only the source a Giovanni, the other torpid.
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Giovanni", 0), 1, true);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let l = e.add_minion(SeatId(1), plain("Giovanni", 1), 3, false);
    let p = in_play(&mut e, 0, "Glass Walker Pact", None);
    fill_hand(&mut e, 0);
    e.state.seats[0].library.push(find_library("Form of Mist"));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[l.index()].locked = true;
    pick_when(&mut e, use_on(p, 0, None, Target::Minion(l)));
    request(&mut e);
    assert_eq!(blood(&e, l), 3);
    assert!(ash_has(&e, 0, "Form of Mist"));
}

/// Kingston Penitentiary: another Methuselah ousted by its 1 pool feeds
/// nothing.
#[test]
fn kingston_penitentiary_payer_ousted_feeds_nothing() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    e.add_minion(SeatId(2), plain("Tremere", 0), 4, false);
    e.state.seats[1].pool = 1;
    let k = in_play(&mut e, 0, "Kingston Penitentiary, Ontario", None);
    pick_when(&mut e, use_on(k, 1, None, Target::Minion(b)));
    request(&mut e);
    assert!(e.state.seats[1].ousted);
    assert_eq!(blood(&e, b), 1);
}

/// The Treatment: the bleed need not have been made as a prince (the text
/// reads the prince now): an untitled vampire's bleed, then a prince.
#[test]
fn the_treatment_counts_a_bleed_made_before_the_title() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Ravnos", 0), 4, false);
    let p = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    stealthy(&mut e, p);
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, bleed(p));
    request(&mut e);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    hand(&mut e, 0, &["Treatment, The"]);
    let req = to_window(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(e.state.seats[0].pool < 10, "bled");
    assert!(req.options.contains(&master_at(0, Target::Minion(p))));
}

/// Trophy: Diablerie retrieved for a Red List vampire diablerized in combat
/// by Amaranth: on the diablerist, then burned at the end of the enclosing
/// bleed (LSJ 20050222), no blood hunt.
#[test]
fn trophy_diablerie_by_amaranth_burns_at_the_actions_end() {
    let red = textless(|c| !c.red_list && (1..=4).contains(&c.capacity), 0);
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), red, 0, false);
    e.state.minions[v.index()].made_red_list = true;
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.seats[1].has_edge = true;
    hand(&mut e, 0, &["Amaranth", "Trophy: Diablerie"]);
    bleed_blocked(&mut e, a, v);
    pick_when(&mut e, play(0, 0, a));
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Hand,
            card: find_library("Trophy: Diablerie"),
        },
    );
    let t = card_in_play(&e, "Trophy: Diablerie");
    assert_eq!(e.state.cards[t.index()].host, Some(a));
    assert!(!blood_hunt_called(&mut e));
    assert!(!e.state.cards[t.index()].in_play);
    assert!(ash_has(&e, 0, "Trophy: Diablerie"));
}

/// Trophy: Retainers: a torpid vampire employs too (its unlock-phase use
/// reads no readiness).
#[test]
fn trophy_retainers_employs_from_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ANI", 0), 4, true);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let t = in_play(&mut e, 0, "Trophy: Retainers", Some(a));
    fill_hand(&mut e, 0);
    e.state.seats[0].library.push(find_library("Raven Spy"));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick_when(&mut e, ability(t.0, 0));
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: find_library("Raven Spy"),
        },
    );
    pick(
        &mut e,
        Choice::Version {
            card: find_library("Raven Spy"),
            mode: 1,
        },
    );
    let spy = card_in_play(&e, "Raven Spy");
    assert_eq!(e.state.cards[spy.index()].host, Some(a));
}

/// Trophy: Safe Passage shields its vampire, not the cards on it: Soul
/// Feasting at the Nar-Sheptha it bears is offered.
#[test]
fn trophy_safe_passage_leaves_the_cards_on_its_vampire_targetable() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.capacity >= 6 && c.disciplines.contains(&"nec"), 0),
        1,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Trophy: Safe Passage", Some(x));
    let nar = in_play(&mut e, 1, "Nar-Sheptha", Some(x));
    e.state.cards[nar.index()].counters = 1;
    hand(&mut e, 0, &["Soul Feasting"]);
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::Play {
        card: 0,
        mode: 2,
        actor: Some(a),
        target: Target::Card(nar),
    }));
}

/// Trophy: Wealth already in play, unawarded, moved onto the Red List
/// diablerist (the award's other path): five counters as it lands.
#[test]
fn trophy_wealth_moved_from_play_gets_five_counters() {
    let (mut e, a) = red_list_kill();
    let w = unawarded(&mut e, 0, "Trophy: Wealth");
    pick_when(&mut e, Choice::ResolveRider { card: w.0 });
    request(&mut e);
    assert_eq!(e.state.cards[w.index()].host, Some(a));
    assert_eq!(e.state.cards[w.index()].counters, 5);
}

/// Weather Control [tha] with an Ilomba on the opposing vampire (1 blood):
/// the damage at once resolved Ilomba first, no choice asked: the host pays
/// Ilomba's point and goes to torpor for its own, not burned.
#[test]
fn weather_control_resolves_ilomba_first() {
    let mut e = game(2);
    let big = |c: &zvtes::crypt::CryptCard| c.capacity >= 9;
    let a = e.add_minion(SeatId(0), fixture_vampire(big, 0), 9, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tha"];
    let il = in_play(&mut e, 1, "Ilomba", Some(z));
    request(&mut e);
    e.state.minions[z.index()].blood = 1;
    hand(&mut e, 0, &["Weather Control"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::ChooseCard(il.0),
    );
    let v = &e.state.minions[z.index()];
    assert!(v.in_play && v.in_torpor);
}

/// Dirty Little Secrets on a bleed Deflected to seat 2: the library burned
/// is the new target's, a card per pool it bled.
#[test]
fn dirty_little_secrets_follows_a_redirected_bleed() {
    let mut e = game(3);
    let a = e.add_minion(
        SeatId(0),
        of_clan("Nosferatu antitribu", 4..=11, 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        of_clan("Nosferatu antitribu", 4..=11, 1),
        4,
        false,
    );
    let x = e.add_minion(SeatId(1), vampire_with("dom", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    bonus(&mut e, a, zvtes::cards::Stat::Bleed);
    for s in [1, 2] {
        fill_hand(&mut e, s);
        for _ in 0..5 {
            e.state.seats[s as usize]
                .library
                .push(find_library("Deflection"));
        }
    }
    e.state.seats[1].hand.clear();
    hand(&mut e, 1, &["Deflection"]);
    let lib1 = e.state.seats[1].library.len();
    hand(&mut e, 0, &["Dirty Little Secrets"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(x),
            target: Target::Seat(SeatId(2)),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[2], 8);
    assert_eq!(e.state.seats[2].library.len(), 3);
    assert_eq!(e.state.seats[2].ash.len(), 2);
    // Seat 1's library only drawn from (its hand refilled), none burned.
    assert!(e.state.seats[1].library.len() <= lib1);
    assert_eq!(
        e.state.seats[1].ash.to_cards(),
        [find_library("Deflection")]
    );
}

/// Dr. Marisa Fletcher and Millicent Smith on one blocked hunt: both apply
/// (LSJ 20050428): Millicent and the hunter burned, Marisa's counters too.
#[test]
fn dr_marisa_fletcher_and_millicent_smith_both_apply() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ventrue", 2..=3, 0), 2, false);
    e.add_minion(SeatId(0), of_clan("Ventrue", 5..=11, 1), 4, false);
    let x = e.add_minion(SeatId(1), of_clan("Brujah", 5..=11, 0), 4, false);
    e.add_minion(SeatId(1), of_clan("Brujah", 5..=11, 1), 4, false);
    let ev = in_play(&mut e, 1, "Dr. Marisa Fletcher, CDC", None);
    e.state.cards[ev.index()].counters = 4;
    let m = in_play(&mut e, 0, "Millicent Smith, Puritan Vampire Hunter", None);
    bonus(&mut e, x, zvtes::cards::Stat::Intercept);
    pick(&mut e, action(a, zvtes::decision::ActionKind::Hunt));
    pick_when(&mut e, block(x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].in_play);
    assert!(!e.state.cards[m.index()].in_play);
    assert_eq!(e.state.cards[ev.index()].counters, 0);
}

/// Sins of the Cauchemar [aus]: a plain +1 intercept, the card to the ash
/// heap, nothing put on the acting vampire.
#[test]
fn sins_of_the_cauchemar_basic_is_plain_intercept() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let r = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.state.minions[r.index()].gained_levels = vec!["aus"];
    stealthy(&mut e, a);
    hand(&mut e, 1, &["Sins of the Cauchemar"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(r));
    pick_when(&mut e, play(0, 0, r));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10, "blocked");
    assert!(ash_has(&e, 1, "Sins of the Cauchemar"));
}

/// Incriminating Videotape: the chosen minion is the equip's target, so
/// the equip is directed at the prey (owner, 2026-11-19; rules-digest
/// §3.4): only the prey's minions may block it, not the predator's. Found
/// by Vast Wealth, the equip is Vast Wealth's own undirected action, the
/// minion chosen as it resolves.
#[test]
fn incriminating_videotape_equip_is_directed_at_the_prey() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let z = e.add_minion(SeatId(2), plain("Tremere", 0), 4, false);
    bonus(&mut e, y, zvtes::cards::Stat::Intercept);
    bonus(&mut e, z, zvtes::cards::Stat::Intercept);
    hand(&mut e, 0, &["Incriminating Videotape"]);
    pick(&mut e, play_at(0, a, x));
    request(&mut e);
    let act = e.action().expect("the equip");
    assert_eq!(act.directed_seat(&e.state), Some(SeatId(1)));
    let req = offering(&mut e, block(y));
    assert_eq!(req.seat, SeatId(1));
    e.choose(index_of(&req, Choice::Pass));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(z));
    // By a search: undirected.
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Tremere", 0), 4, false);
    let wealth = equipped(&mut e, "Vast Wealth", a);
    fill_hand(&mut e, 0);
    e.state.seats[0]
        .library
        .push(find_library("Incriminating Videotape"));
    pick(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(wealth),
            ability: 0,
            actor: Some(a),
            target: Target::None,
        },
    );
    request(&mut e);
    let act = e.action().expect("Vast Wealth's equip");
    assert_eq!(act.directed_seat(&e.state), None);
}

/// Rules-qa 77.23 (corrected): Samuel Haight, a werewolf, deals damage
/// other than his strikes' through the cards he plays as a vampire:
/// Riposte [cel]'s after-combat damage on Selena is aggravated ("Damage
/// inflicted on Selena by a werewolf"): he steals her gun as round 1's
/// strike, presses (Swiftness of the Stag [cel]), ends round 2 with
/// Riposte; her 1 damage after combat cannot be mended, she goes to torpor.
// Rulings: [LSJ 20100204-2] [ANK 20170111].
#[test]
fn samuel_haights_riposte_is_aggravated_on_selena() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Selena"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let gun = in_play(&mut e, 0, ".44 Magnum", Some(s));
    let sam = e.add_ally(SeatId(1), find_library("Samuel Haight"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Riposte", "Swiftness of the Stag"]);
    let card = ally_card(&e, sam);
    let steal = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(card),
        ability: 1,
        actor: None,
        target: Target::Card(gun),
    };
    pick(&mut e, bleed(s));
    pick_when(&mut e, block(sam));
    for want in [steal, play(1, 0, sam), play(0, 0, sam)] {
        pick_when(&mut e, want);
    }
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[s.index()].in_torpor);
}

/// LSJ 20011217: an "X or Y" grant meets one Discipline of a combo card,
/// never both (rules-qa 125.246): Grey Thorne ("Celerity [cel] or Potence
/// [pot]", ANK 20180731-1) never plays Meia Lua [cel][pot]; Talaq's "[qui]
/// or [tha]" is read so too, no pool combo card needing both.
// Rulings: [LSJ 20011217] [ANK 20180731-1].
#[test]
fn an_or_grant_never_plays_a_combo_card_by_both() {
    let mut e = game(2);
    e.add_minion(SeatId(0), of_sect(Sect::Anarch, 4..=7, 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let g = e.add_ally(SeatId(0), find_library("Grey Thorne"));
    hand(&mut e, 0, &["Meia Lua"]);
    pick(&mut e, bleed(g));
    pick_when(&mut e, block(z));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == g),
    );
}
