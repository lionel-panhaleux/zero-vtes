//! Observations (`zvtes::obs`, DESIGN §5): the encoding of a request reads
//! only what the deciding seat may know — it equals the encoding of the
//! same request from the seat's legal view — and every pointer names a
//! token or carries an identity. Games from the opening (bots and random seats) and the fuzz
//! tables (`common::tables`, random play). `OBS_STATS=1` prints the largest
//! token and option counts met. `V2` is `V1` plus its ash heap and event
//! tokens, and the history it reads holds only public choices. `V3` is
//! `V2` with its deck tokens apart and the others' face-down uncontrolled
//! vampires gone, pointers following.

mod common;

use common::tables::{table, ALLY_SLICE, PRECON_SLICE, SLICE};
use zvtes::agent::{Agent, RandomLegal};
use zvtes::bots::{Scripted, Style};
use zvtes::decision::Choice;
use zvtes::game::{Engine, Step};
use zvtes::obs::{
    encode, encode_critic, Obs, Version, T_ASH, T_CONTEXT, T_DECK, T_EVENT, T_HAND, T_REFERENDUM,
    T_SEAT, T_SLOT, V_CRYPT, V_HIDDEN, V_LIBRARY,
};

const DECISION_BUDGET: u64 = 100_000;

/// Options whose first pointer must name a token.
fn names_a_token(c: &Choice) -> bool {
    !matches!(
        c,
        Choice::Pass
            | Choice::BurnEdge
            | Choice::BurnPool
            | Choice::SetX(_)
            | Choice::FindCard(_)
            | Choice::TakeFrom { .. }
            | Choice::AshVampire(_)
            | Choice::DrawLibrary
            | Choice::PayRescue { .. }
            | Choice::CastEdge { .. }
            | Choice::CastCardVote { .. }
            | Choice::ChooseClan(_)
            | Choice::DrawCrypt
            | Choice::HandStrike
            | Choice::UseManeuver
            | Choice::UsePress
            | Choice::UsePrevent
            | Choice::PayBlockToll
            | Choice::PayToCancel
            | Choice::StrikeCombatEnds
            | Choice::RoundStrike { at: None }
            | Choice::TakeDamageAfterResolution
            | Choice::CommittedStrike
            | Choice::EmployFromHand { .. }
            | Choice::ContinueAction
            | Choice::GainStrength
            | Choice::BurnBloodToAvoid
            | Choice::Diablerise
    )
}

fn check(o: &Obs, req: &zvtes::decision::DecisionRequest, what: &str) {
    let n = o.tok_ids.len() as i32;
    for t in &o.tok_ids {
        assert!(
            t[2..].iter().all(|&p| (0..=n).contains(&p)),
            "{what}: token {t:?}"
        );
    }
    assert_eq!(o.opt_ids.len(), req.options.len());
    for (ids, c) in o.opt_ids.iter().zip(&req.options) {
        assert!(
            ids[2..].iter().all(|&p| (0..=n).contains(&p)),
            "{what}: {c:?}"
        );
        // A card or minion out of play names no token but keeps its
        // identity.
        assert!(
            !names_a_token(c) || ids[2] > 0 || ids[1] > 0,
            "{what}: {c:?} names nothing"
        );
        if let Choice::Play { actor: Some(_), .. } = c {
            assert!(ids[3] > 0, "{what}: {c:?} actor");
        }
    }
}

#[derive(Default)]
struct Stats {
    tokens: usize,
    tokens_v2: usize,
    tokens_v3: usize,
    options: usize,
    samples: usize,
}

/// `V2` without its ash heap and event tokens is `V1`: they come after
/// every token a pointer names, before the context token.
fn extends_v1(v1: &Obs, v2: &Obs, what: &str) {
    let added = |t: &[i32; 4]| t[0] == T_ASH || t[0] >= T_EVENT;
    let keep: Vec<usize> = (0..v2.tok_ids.len())
        .filter(|&i| !added(&v2.tok_ids[i]))
        .collect();
    let first = v2
        .tok_ids
        .iter()
        .position(added)
        .unwrap_or(v2.tok_ids.len());
    assert!(
        keep.iter().all(|&i| i < first || i == v2.tok_ids.len() - 1),
        "{what}: added tokens before the context"
    );
    let ids: Vec<_> = keep.iter().map(|&i| v2.tok_ids[i]).collect();
    let f: Vec<_> = keep.iter().map(|&i| v2.tok_f[i]).collect();
    let n = v1.tok_ids.len();
    assert!(ids[..n - 1] == v1.tok_ids[..n - 1], "{what}: tokens");
    assert!(ids[n - 1] == v1.tok_ids[n - 1], "{what}: context");
    assert!(f == v1.tok_f, "{what}: features");
    assert!(
        v2.opt_ids == v1.opt_ids && v2.opt_f == v1.opt_f,
        "{what}: options"
    );
}

/// `V3` is `V2` with its deck tokens in `deck_ids` and its face-down
/// slots gone (none with `omniscient`), pointers to them 0 and the others
/// following; the same features but the seat and context tokens' (the
/// face-down counters, the transfers left).
fn lightens_v2(v2: &Obs, v3: &Obs, omniscient: bool, what: &str) {
    let deck: Vec<usize> = (0..v2.tok_ids.len())
        .filter(|&i| v2.tok_ids[i][0] == T_DECK)
        .collect();
    assert!(
        deck.iter()
            .map(|&i| v2.tok_ids[i])
            .eq(v3.deck_ids.iter().copied())
            && deck
                .iter()
                .map(|&i| v2.tok_f[i])
                .eq(v3.deck_f.iter().copied()),
        "{what}: deck"
    );
    let gone = |t: &[i32; 4]| t[0] == T_DECK || (t[0] == T_SLOT && t[1] == V_HIDDEN);
    assert!(
        !omniscient
            || v2
                .tok_ids
                .iter()
                .all(|t| !(t[0] == T_SLOT && t[1] == V_HIDDEN))
    );
    let mut map = vec![0i32; v2.tok_ids.len() + 1];
    let mut k = 0;
    for (i, t) in v2.tok_ids.iter().enumerate() {
        if !gone(t) {
            k += 1;
            map[i + 1] = k;
        }
    }
    assert_eq!(k as usize, v3.tok_ids.len(), "{what}: token count");
    let moved = |ids: &[i32]| -> Vec<i32> {
        ids.iter()
            .enumerate()
            .map(|(j, &p)| if j >= 2 { map[p as usize] } else { p })
            .collect()
    };
    let kept = (0..v2.tok_ids.len()).filter(|&i| !gone(&v2.tok_ids[i]));
    for (i, j) in kept.zip(0..) {
        let (a, b) = (v2.tok_ids[i], v3.tok_ids[j]);
        // An event's pointer may have named a face-down slot: now 0.
        assert!(moved(&a) == b, "{what}: token {a:?} → {b:?}");
        if a[0] != T_SEAT && a[0] != T_CONTEXT {
            assert!(v2.tok_f[i] == v3.tok_f[j], "{what}: features of {a:?}");
        }
    }
    for (a, b) in v2.opt_ids.iter().zip(&v3.opt_ids) {
        assert!(moved(a) == b.to_vec(), "{what}: option {a:?} → {b:?}");
    }
    assert!(v2.opt_f == v3.opt_f, "{what}: option features");
}

fn game(what: &str, mut e: Engine, agents: &mut [Box<dyn Agent>], stats: &mut Stats) {
    while let Step::Decision(req) = e.run() {
        assert!(e.decisions < DECISION_BUDGET, "{what}: decision budget");
        let seeing = encode(&e, &req, Version::V1);
        let seeing_v2 = encode(&e, &req, Version::V2);
        let what = format!("{what}, decision {}", e.decisions);
        check(&seeing, &req, &what);
        check(&seeing_v2, &req, &what);
        extends_v1(&seeing, &seeing_v2, &what);
        let seeing_v3 = encode(&e, &req, Version::V3);
        check(&seeing_v3, &req, &what);
        lightens_v2(&seeing_v2, &seeing_v3, false, &what);
        lightens_v2(
            &encode_critic(&e, &req, Version::V2),
            &encode_critic(&e, &req, Version::V3),
            true,
            &what,
        );
        stats.tokens_v3 = stats.tokens_v3.max(seeing_v3.tok_ids.len());
        if e.decisions.is_multiple_of(2) {
            let mut view = e.clone();
            view.state = e.state.legal_view(req.seat);
            assert!(
                encode(&view, &req, Version::V1) == seeing,
                "{what}: {:?}",
                req.window
            );
            assert!(encode(&view, &req, Version::V3) == seeing_v3, "{what}: V3");
            assert!(
                encode(&view, &req, Version::V2) == seeing_v2,
                "{what}: {:?}",
                req.window
            );
            stats.samples += 1;
        }
        stats.tokens = stats.tokens.max(seeing.tok_ids.len());
        stats.tokens_v2 = stats.tokens_v2.max(seeing_v2.tok_ids.len());
        stats.options = stats.options.max(seeing.opt_ids.len());
        let i = agents[req.seat.index()].decide(&e, &req);
        // The history keeps the public choices only, a played card by
        // its identity.
        let before = e.history.clone();
        let choice = req.options[i];
        let played = match choice {
            Choice::Play { card, .. }
            | Choice::Discard(card)
            | Choice::BurnForVote { card, .. } => Some(e.playable_card(req.seat, card)),
            _ => None,
        };
        e.choose(i);
        if zvtes::game::public_choice(&choice) {
            let ev = e.history.back().expect("an event");
            assert!(
                ev.seat == req.seat && ev.choice == choice && ev.card == played,
                "{what}: {ev:?}"
            );
        } else {
            assert!(e.history == before, "{what}: {choice:?} recorded");
        }
    }
}

fn random(seed: u64, n: usize) -> Vec<Box<dyn Agent>> {
    (0..n)
        .map(|s| -> Box<dyn Agent> { Box::new(RandomLegal::new(seed * 10 + s as u64)) })
        .collect()
}

#[test]
fn observations_read_only_the_legal_view() {
    let mut stats = Stats::default();
    for seed in 0..12u64 {
        // Precon games: two bots, two random seats.
        let e = Engine::new(seed, 4);
        let mut agents = random(seed, 4);
        agents[0] = Box::new(Scripted::new(Style::AggroBleed, seed));
        agents[2] = Box::new(Scripted::new(Style::Wall, seed));
        game(&format!("precon {seed}"), e, &mut agents, &mut stats);
    }
    for seed in 0..6u64 {
        for (name, e) in [
            ("midgame", table(seed, SLICE, &[])),
            ("precon table", table(seed, PRECON_SLICE, &["OBF"])),
            ("ally table", table(seed, ALLY_SLICE, &[])),
        ] {
            let n = e.state.seats.len();
            game(
                &format!("{name} {seed}"),
                e,
                &mut random(seed, n),
                &mut stats,
            );
        }
    }
    if std::env::var("OBS_STATS").is_ok() {
        eprintln!(
            "{} samples; at most {} tokens ({} in V2, {} in V3), {} options",
            stats.samples, stats.tokens, stats.tokens_v2, stats.tokens_v3, stats.options
        );
    }
}

/// A banished created vampire (Childe of the Revolution) sits in its
/// uncontrolled slot with no crypt card (`CREATED`): its token is its card's
/// identity, the same to every seat.
#[test]
fn a_banished_created_vampire_is_its_card() {
    use zvtes::crypt::Sect;
    use zvtes::ids::{MinionId, SeatId};
    use zvtes::obs::{T_SLOT, V_LIBRARY};
    use zvtes::state::{
        find_card, find_library, Banished, CardId, Minion, MinionKind, UncontrolledSlot, CREATED,
    };
    let mut e = Engine::bare(2);
    e.add_minion(SeatId(0), find_card("Nayarana"), 3, false);
    e.add_minion(SeatId(0), find_card("Mithras"), 3, false);
    let childe = find_library("Childe of the Revolution");
    let kind = MinionKind::Created {
        card: childe,
        id: CardId(0),
        clan: "Brujah",
        sect: Sect::Anarch,
        born: &zvtes::state::NEWBORN,
    };
    e.state.minions.push(Minion {
        in_play: false,
        ..Minion::new(SeatId(1), kind, 1)
    });
    e.state.seats[1].uncontrolled.push(UncontrolledSlot {
        card: CREATED,
        counters: 1,
        banished: Some(Banished {
            minion: MinionId(e.state.minions.len() as u32 - 1),
            cards: Vec::new(),
        }),
    });
    let Step::Decision(req) = e.run() else {
        panic!("a decision")
    };
    let o = encode(&e, &req, Version::V1);
    let slot = o.tok_ids.iter().find(|t| t[0] == T_SLOT).expect("the slot");
    assert_eq!(slot[1], V_LIBRARY + childe as i32);
}

/// The critic's encoding adds every other hand and turns the others'
/// face-down uncontrolled vampires face up; the policy's is its prefix
/// otherwise unchanged in count of visible tokens.
#[test]
fn the_critic_sees_every_hand_and_uncontrolled_vampire() {
    let mut e = Engine::new(3, 4);
    let Step::Decision(req) = e.run() else {
        panic!("a decision")
    };
    let (seen, all) = (
        encode(&e, &req, Version::V1),
        encode_critic(&e, &req, Version::V1),
    );
    let hands = |o: &Obs| o.tok_ids.iter().filter(|t| t[0] == T_HAND).count();
    let others: usize = e
        .state
        .seats
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != req.seat.index())
        .map(|(_, s)| s.hand.len())
        .sum();
    assert_eq!(hands(&all), hands(&seen) + others);
    let hidden_slots = |o: &Obs| {
        o.tok_ids
            .iter()
            .filter(|t| t[0] == T_SLOT && t[1] == V_HIDDEN)
            .count()
    };
    assert!(hidden_slots(&seen) > 0);
    assert_eq!(hidden_slots(&all), 0);
}

/// The viewer's own library and crypt are encoded by contents — a token
/// per card with its copies — and never by order; no other seat's are.
#[test]
fn the_viewer_sees_what_remains_of_their_library_and_crypt() {
    let mut e = Engine::new(3, 4);
    let Step::Decision(req) = e.run() else {
        panic!("a decision")
    };
    let seen = encode(&e, &req, Version::V1);
    let seat = &e.state.seats[req.seat.index()];
    let mut expected = std::collections::BTreeMap::new();
    for &c in &seat.library {
        *expected.entry(V_LIBRARY + c as i32).or_insert(0u32) += 1;
    }
    for &c in &seat.crypt {
        *expected.entry(V_CRYPT + c as i32).or_insert(0u32) += 1;
    }
    let deck: std::collections::BTreeMap<i32, u32> = seen
        .tok_ids
        .iter()
        .zip(&seen.tok_f)
        .filter(|(t, _)| t[0] == T_DECK)
        .map(|(t, f)| (t[1], (f[17] * 10.0).round() as u32))
        .collect();
    assert!(!expected.is_empty());
    assert_eq!(deck, expected);
    // Another order of the same cards: the same observation.
    let seat = &mut e.state.seats[req.seat.index()];
    seat.library.reverse();
    seat.crypt.reverse();
    assert!(encode(&e, &req, Version::V1) == seen);
}

/// A political referendum names its card, its terms (points on the seat
/// tokens) and the votes cast so far: a voter can weigh what passes.
#[test]
fn a_referendum_shows_its_card_terms_and_votes() {
    use common::{alloc, expect, hand, pick, play, votes};
    use zvtes::decision::WindowLabel;
    use zvtes::ids::SeatId;
    use zvtes::state::{find_card, find_library};
    let mut e = Engine::bare(3);
    let alexa = e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    e.add_minion(SeatId(0), find_card("Mateo García"), 2, false);
    let kevin = e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
    e.add_minion(SeatId(2), find_card("Lenny Burkhead"), 4, false);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.state.turn = 4;
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, alexa));
    expect(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    pick(&mut e, alloc(0, 0));
    expect(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    pick(&mut e, alloc(1, 3));
    expect(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    pick(&mut e, votes(alexa, true));
    // Seat 1 polls next: 3 points on it, 1 on seat 2, 2 votes for so far.
    let req = loop {
        let Step::Decision(r) = e.run() else {
            panic!("a decision")
        };
        if r.seat == SeatId(1) && r.options.contains(&votes(kevin, false)) {
            break r;
        }
        e.choose(0);
    };
    let o = encode(&e, &req, Version::V1);
    let refs: Vec<_> = (0..o.tok_ids.len())
        .filter(|&i| o.tok_ids[i][0] == T_REFERENDUM)
        .collect();
    assert_eq!(refs.len(), 1);
    let kine = find_library("Kine Resources Contested") as i32;
    assert_eq!(o.tok_ids[refs[0]][1], V_LIBRARY + kine);
    let f = &o.tok_f[refs[0]];
    assert!(
        f.iter().any(|&x| (x - 2.0 / 5.0).abs() < 1e-6),
        "2 votes for"
    );
    // Seat tokens in relative order from seat 1: itself 3 points, seat 2
    // 1 point, seat 0 none.
    let seats: Vec<_> = (0..o.tok_ids.len())
        .filter(|&i| o.tok_ids[i][0] == T_SEAT)
        .collect();
    let (me, next, caller) = (&o.tok_f[seats[0]], &o.tok_f[seats[1]], &o.tok_f[seats[2]]);
    let diff: Vec<usize> = (0..me.len())
        .filter(|&j| me[j] == 3.0 / 5.0 && next[j] == 1.0 / 5.0 && caller[j] == 0.0)
        .collect();
    assert!(!diff.is_empty(), "the allocation is on the seat tokens");
}
