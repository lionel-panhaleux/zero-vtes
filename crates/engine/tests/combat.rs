//! Slice 3 of the effects framework: combat (rulebook "Combat"; spec §4).
//! The card-less machine first — a block leads to one round of hand
//! strikes at close range — then the combat cards, step by step. Windows
//! with a single option are elided, so the scripts only stop where a real
//! decision exists.

use zvtes::actions::Outcome;
use zvtes::cards::Target;
use zvtes::crypt::CRYPT;
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::{MinionId, SeatId};

fn request(e: &mut Engine) -> DecisionRequest {
    match e.run() {
        Step::Decision(req) => {
            if std::env::var("TRACE").is_ok() {
                eprintln!("REQ {:?} {:?} {:?}", req.seat, req.window, req.options);
            }
            req
        }
        Step::Over(_) => panic!("game over while expecting a decision"),
    }
}

fn expect(e: &mut Engine, seat: SeatId, window: WindowLabel) -> DecisionRequest {
    let req = request(e);
    assert_eq!((req.seat, req.window), (seat, window), "{:?}", req.options);
    req
}

fn pick(e: &mut Engine, want: Choice) {
    let req = request(e);
    let idx = req
        .options
        .iter()
        .position(|o| *o == want)
        .unwrap_or_else(|| panic!("option {want:?} not in {:?}", req.options));
    e.choose(idx);
}

fn bleed(minion: MinionId) -> Choice {
    Choice::TakeAction {
        minion,
        action: ActionKind::Bleed,
        target: None,
    }
}

fn block(minion: MinionId) -> Choice {
    Choice::DeclareBlock { minion }
}

#[allow(dead_code)]
fn play(card: u8, mode: u8, actor: MinionId) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target: Target::None,
    }
}

#[allow(dead_code)]
fn plays(req: &DecisionRequest) -> Vec<Choice> {
    req.options
        .iter()
        .copied()
        .filter(|o| matches!(o, Choice::Play { .. }))
        .collect()
}

/// The `nth` V5 vampire with superior `disc`.
fn vampire_with(disc: &str, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(_, c)| c.disciplines.contains(&disc))
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no vampire with {disc}"))
}

fn blood(e: &Engine, m: MinionId) -> u8 {
    e.state.minions[m.index()].blood
}

/// 2 seats at seat 0's minion phase, turn 4. Seat 0: A (POT), B (CEL);
/// seat 1: X (FOR), Y (PRO). 4 blood each.
struct Table {
    e: Engine,
    a: MinionId,
    b: MinionId,
    x: MinionId,
    y: MinionId,
}

fn table() -> Table {
    let mut e = Engine::bare(2);
    let a = e.add_minion(SeatId(0), vampire_with("POT", 0), 4, false);
    let b = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
    let x = e.add_minion(SeatId(1), vampire_with("FOR", 0), 4, false);
    let y = e.add_minion(SeatId(1), vampire_with("PRO", 0), 4, false);
    e.state.turn = 4;
    Table { e, a, b, x, y }
}

/// Bleed with `acting`, blocked by `blocker`; every window up to the
/// block's combat elides with no cards in hand.
fn blocked_bleed(e: &mut Engine, acting: MinionId, blocker: MinionId) {
    pick(e, bleed(acting));
    pick(e, block(blocker));
}

// Rulebook "Combat": a block leads to one round at close range; with no
// cards, both minions hand strike for 1 (default strength), the blocker
// stays locked, and the action ended blocked.
#[test]
fn card_less_combat_is_one_round_of_hand_strikes() {
    let mut t = table();
    let e = &mut t.e;
    blocked_bleed(e, t.a, t.x);
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(e, t.a), blood(e, t.x)), (3, 3));
    assert!(e.state.minions[t.x.index()].locked);
    assert!(e.state.minions[t.a.index()].locked);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
    assert_eq!(e.state.seats[1].pool, 30);
}

// Rulebook "Damage Resolution": damage that cannot be mended wounds; the
// vampire goes to torpor and the combat ends.
#[test]
fn unmended_damage_sends_the_blocker_to_torpor() {
    let mut t = table();
    let e = &mut t.e;
    e.state.minions[t.x.index()].blood = 0;
    blocked_bleed(e, t.a, t.x);
    expect(e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.x.index()].in_torpor);
    assert_eq!(blood(e, t.a), 3);
    let _ = (t.b, t.y);
}
