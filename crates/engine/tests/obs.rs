//! Observations (`zvtes::obs`, DESIGN §5): the encoding of a request reads
//! only what the deciding seat may know — it equals the encoding of the
//! same request from the seat's legal view — and every pointer names a
//! token or carries an identity. Games from the opening (bots and random seats) and the fuzz
//! tables (`common::tables`, random play). `OBS_STATS=1` prints the largest
//! token and option counts met.

mod common;

use common::tables::{table, ALLY_SLICE, PRECON_SLICE, SLICE};
use zvtes::agent::{Agent, RandomLegal};
use zvtes::bots::{Scripted, Style};
use zvtes::decision::Choice;
use zvtes::game::{Engine, Step};
use zvtes::obs::{encode, encode_critic, Obs, T_HAND, T_SLOT, V_HIDDEN};

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
    options: usize,
    samples: usize,
}

fn game(what: &str, mut e: Engine, agents: &mut [Box<dyn Agent>], stats: &mut Stats) {
    while let Step::Decision(req) = e.run() {
        assert!(e.decisions < DECISION_BUDGET, "{what}: decision budget");
        let seeing = encode(&e, &req);
        let what = format!("{what}, decision {}", e.decisions);
        check(&seeing, &req, &what);
        if e.decisions.is_multiple_of(2) {
            let mut view = e.clone();
            view.state = e.state.legal_view(req.seat);
            assert!(encode(&view, &req) == seeing, "{what}: {:?}", req.window);
            stats.samples += 1;
        }
        stats.tokens = stats.tokens.max(seeing.tok_ids.len());
        stats.options = stats.options.max(seeing.opt_ids.len());
        let i = agents[req.seat.index()].decide(&e, &req);
        e.choose(i);
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
            "{} samples; at most {} tokens, {} options",
            stats.samples, stats.tokens, stats.options
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
    let o = encode(&e, &req);
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
    let (seen, all) = (encode(&e, &req), encode_critic(&e, &req));
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
