//! Helpers shared by the scenario tests: driving the decision loop, the
//! usual choices, and small fixture lookups. A test file's own helper of the
//! same name (a `table`, a seat-indexed `pool`) shadows the one here.
#![allow(dead_code)]

pub mod tables;

use zvtes::cards::{AbilitySource, Target};
use zvtes::crypt::CRYPT;
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_library, CardId};

// ---- Driving the decision loop -----------------------------------------------

/// The pending decision (`TRACE=1` prints each one).
pub fn request(e: &mut Engine) -> DecisionRequest {
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

/// The pending decision, asserted to be `seat`'s `window`; a window of
/// futile blocks only on the way is declined.
pub fn expect(e: &mut Engine, seat: SeatId, window: WindowLabel) -> DecisionRequest {
    let mut req = request(e);
    while (req.seat, req.window) != (seat, window) && e.futile_blocks(&req) {
        e.choose(index_of(&req, Choice::Pass));
        req = request(e);
    }
    assert_eq!((req.seat, req.window), (seat, window), "{:?}", req.options);
    req
}

pub fn index_of(req: &DecisionRequest, want: Choice) -> usize {
    req.options
        .iter()
        .position(|o| *o == want)
        .unwrap_or_else(|| {
            panic!(
                "option {want:?} not in {:?} ({:?})",
                req.options, req.window
            )
        })
}

/// Answer the pending decision with `want`, which must be offered; a
/// window of futile blocks only on the way is declined.
pub fn pick(e: &mut Engine, want: Choice) {
    let mut req = request(e);
    while !req.options.contains(&want) && e.futile_blocks(&req) {
        e.choose(index_of(&req, Choice::Pass));
        req = request(e);
    }
    e.choose(index_of(&req, want));
}

/// The first option of a request that is a pass (or a hand strike).
pub fn fallback(req: &DecisionRequest) -> usize {
    req.options
        .iter()
        .position(|o| matches!(o, Choice::Pass | Choice::HandStrike))
        .unwrap_or(0)
}

/// Pass (or hand strike) through every decision until `seat`'s `window`.
pub fn until(e: &mut Engine, seat: SeatId, window: WindowLabel) -> DecisionRequest {
    loop {
        let req = request(e);
        if (req.seat, req.window) == (seat, window) {
            return req;
        }
        let i = fallback(&req);
        e.choose(i);
    }
}

/// Pass every window until `seat`'s `window` opens (mandatory riders are
/// resolved in offer order on the way; single-option windows elide).
pub fn to_window(e: &mut Engine, seat: SeatId, window: WindowLabel) -> DecisionRequest {
    loop {
        let req = request(e);
        if (req.seat, req.window) == (seat, window) {
            return req;
        }
        assert!(
            matches!(req.options[0], Choice::Pass | Choice::ResolveRider { .. }),
            "{req:?}"
        );
        e.choose(0);
    }
}

/// Pass every window until `seat`'s minion phase opens.
pub fn to_minion_phase(e: &mut Engine, seat: SeatId) -> DecisionRequest {
    to_window(e, seat, WindowLabel::MinionPhase)
}

/// Pass (or hand strike) until `want` is offered, and take it.
pub fn pick_when(e: &mut Engine, want: Choice) {
    for _ in 0..200 {
        let req = request(e);
        if let Some(i) = req.options.iter().position(|o| *o == want) {
            e.choose(i);
            return;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    panic!("{want:?} never offered");
}

/// Pass (or hand strike) until a request offers `want`; that request is
/// returned, unanswered.
pub fn offering(e: &mut Engine, want: Choice) -> DecisionRequest {
    for _ in 0..200 {
        let req = request(e);
        if req.options.contains(&want) {
            return req;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    panic!("{want:?} never offered");
}

/// Pass (or hand strike) until `want` is offered — taken, true — or
/// `seat`'s `window` opens first — false.
pub fn offered_before(e: &mut Engine, want: Choice, seat: SeatId, window: WindowLabel) -> bool {
    loop {
        let req = request(e);
        if let Some(i) = req.options.iter().position(|o| *o == want) {
            e.choose(i);
            return true;
        }
        if (req.seat, req.window) == (seat, window) {
            return false;
        }
        let i = fallback(&req);
        e.choose(i);
    }
}

/// Pass (or hand strike) until `seat`'s `window`, asserting on the way no
/// request offers an option `bad` matches.
pub fn never_offered_where(
    e: &mut Engine,
    seat: SeatId,
    window: WindowLabel,
    bad: impl Fn(&Choice) -> bool,
) {
    loop {
        let req = request(e);
        assert!(
            !req.options.iter().any(&bad),
            "offered in {:?}: {:?}",
            req.window,
            req.options
        );
        if (req.seat, req.window) == (seat, window) {
            return;
        }
        let i = fallback(&req);
        e.choose(i);
    }
}

/// Pass (or hand strike) until `seat`'s `window`, asserting `choice` is
/// never offered on the way.
pub fn never_offered_until(e: &mut Engine, seat: SeatId, window: WindowLabel, choice: Choice) {
    never_offered_where(e, seat, window, |o| *o == choice);
}

/// Declines the pending windows of futile blocks only
/// (`Engine::futile_blocks`);
/// how many.
pub fn decline_futile(e: &mut Engine) -> usize {
    let mut n = 0;
    loop {
        let req = request(e);
        if !e.futile_blocks(&req) {
            return n;
        }
        e.choose(index_of(&req, Choice::Pass));
        n += 1;
    }
}

/// Pass (or hand strike) until `seat`'s `window`, asserting `m`'s block
/// attempt is offered on the way (rules-qa 73.8) and, wherever offered,
/// cannot reach the stealth.
pub fn never_reaching_until(e: &mut Engine, seat: SeatId, window: WindowLabel, m: MinionId) {
    let mut offered = false;
    loop {
        let req = request(e);
        if req.options.contains(&Choice::DeclareBlock { minion: m }) {
            assert!(!e.block_can_reach(req.seat, m), "{m:?} can reach");
            offered = true;
        }
        if (req.seat, req.window) == (seat, window) {
            assert!(offered, "{m:?}'s attempt never offered");
            return;
        }
        let i = fallback(&req);
        e.choose(i);
    }
}

/// Pass (or hand strike) until a referendum is under way.
pub fn to_referendum(e: &mut Engine) {
    for _ in 0..200 {
        let req = request(e);
        if e.referendum().is_some() {
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("no referendum called");
}

/// Everyone passes the polling step until it closes (futile blocks of
/// the political action declined first).
pub fn close_polling(e: &mut Engine) {
    decline_futile(e);
    loop {
        match e.run() {
            Step::Decision(req)
                if matches!(
                    req.window,
                    WindowLabel::ReferendumPolling | WindowLabel::BloodHuntPolling
                ) =>
            {
                let i = fallback(&req);
                e.choose(i);
            }
            _ => return,
        }
    }
}

// ---- Choices -----------------------------------------------------------------

/// A card from hand played by `actor` in `mode`, no target.
pub fn play(card: u8, mode: u8, actor: MinionId) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target: Target::None,
    }
}

/// A card from hand played by the Methuselah (no actor, no target).
pub fn master_play(card: u8) -> Choice {
    Choice::Play {
        card,
        mode: 0,
        actor: None,
        target: Target::None,
    }
}

pub fn action(minion: MinionId, action: ActionKind) -> Choice {
    Choice::TakeAction {
        minion,
        action,
        target: None,
    }
}

pub fn bleed(minion: MinionId) -> Choice {
    action(minion, ActionKind::Bleed)
}

pub fn take_from(minion: MinionId, giver: MinionId) -> Choice {
    Choice::TakeAction {
        minion,
        action: ActionKind::EquipFrom,
        target: Some(giver),
    }
}

pub fn block(minion: MinionId) -> Choice {
    Choice::DeclareBlock { minion }
}

pub fn votes(minion: MinionId, favor: bool) -> Choice {
    Choice::CastVotes { minion, favor }
}

pub fn alloc(seat: u8, n: u8) -> Choice {
    Choice::AllocateTo {
        seat: SeatId(seat),
        n,
    }
}

pub fn set_x(n: u8) -> Choice {
    Choice::SetX(n)
}

/// The action (ability 0) a card in play grants its bearer `actor`.
pub fn granted(card: u32, actor: MinionId) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(CardId(card)),
        ability: 0,
        actor: Some(actor),
        target: Target::None,
    }
}

/// The Methuselah's use of `ability` of a card in play.
pub fn ability(card: u32, ability: u8) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(CardId(card)),
        ability,
        actor: None,
        target: Target::None,
    }
}

/// The card plays offered.
pub fn plays(req: &DecisionRequest) -> Vec<Choice> {
    req.options
        .iter()
        .copied()
        .filter(|o| matches!(o, Choice::Play { .. }))
        .collect()
}

/// The abilities offered.
pub fn abilities(req: &DecisionRequest) -> Vec<Choice> {
    req.options
        .iter()
        .copied()
        .filter(|o| matches!(o, Choice::UseAbility { .. }))
        .collect()
}

/// The abilities offered of the card in play `card`.
pub fn uses(req: &DecisionRequest, card: u32) -> Vec<Choice> {
    req.options
        .iter()
        .copied()
        .filter(|o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(CardId(c)), .. } if *c == card))
        .collect()
}

// ---- Fixtures and state ------------------------------------------------------

pub fn hand(e: &mut Engine, seat: u8, cards: &[&str]) {
    for c in cards {
        e.state.seats[seat as usize].hand.push(find_library(c));
    }
}

pub fn blood(e: &Engine, m: MinionId) -> u8 {
    e.state.minions[m.index()].blood
}

pub fn pool(e: &Engine) -> Vec<i32> {
    e.state.seats.iter().map(|s| s.pool).collect()
}

/// The `nth` V5 vampire with `disc` and no printed text implemented (a plain
/// fixture: printed abilities would add decisions to the script).
pub fn vampire_with(disc: &str, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.disciplines.contains(&disc) && zvtes::cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no vampire with {disc}"))
}

/// The `nth` untitled V5 vampire of `clan` with no printed-text definition
/// and capacity at least 4.
pub fn plain(clan: &str, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.clan == clan
                && c.title == zvtes::crypt::Title::None
                && c.capacity >= 4
                && zvtes::cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no plain {clan} vampire"))
}

/// Pad `seat`'s hand up to its hand size with Form of Mist, a [pro] combat
/// card the plain fixture vampires cannot play: a short hand is drawn up at
/// once from the library (rulebook "Drawing Cards"), which would take the
/// cards a fixture prepared there.
pub fn fill_hand(e: &mut Engine, seat: u8) {
    let s = SeatId(seat);
    let filler = find_library("Form of Mist");
    while e.state.hand_load(s) < e.state.hand_size(s) {
        e.state.seats[seat as usize].hand.push(filler);
    }
}
