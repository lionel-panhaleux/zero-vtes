//! Observational equivalence of window elision (DESIGN §4.3, §7.2): an
//! auto-pass must look, from every other seat, exactly like a chosen pass —
//! no hidden-hand information may leak through what the others are offered
//! or see.
//!
//! Replaying with elision off would prove nothing: `Engine::run` applies an
//! elided window's lone option through the same `apply` as `choose`. The
//! property is about hidden information, so it is tested with a hand-swap
//! counterfactual. Random games from the opening and the fuzz tables
//! (`common::tables`) are played with `RandomLegal`. At every real request
//! (outside a comparison) to a seat X whose options are one
//! hand-independent option (a pass, the hand strike) plus plays from X's
//! hand (`survivor`), the engine is cloned and the clone's X gets as many
//! copies of a card X cannot play (a combat card, modifier or reaction
//! requiring a clan none of X's minions has: `fillers`, a few tried). The
//! clone must then elide X's window. The actual game takes the
//! hand-independent option; the clone follows the actual game's next
//! choices while requests go to other seats, and at every step the requests
//! must be equal (seat, window, options), and so must
//! `GameState::legal_view(Y)` for every Y ≠ X who cannot see X's hand, and
//! the action, combat and referendum under way. It stops at X's next real
//! request in either run or the game's end, or where X's hidden cards
//! legitimately become public (`Stop`). The discard phase and a discard
//! down are not sampled: any card can be discarded, no hand elides them.
//!
//! A forced move (a lone option that is not a pass) has no hand-swap
//! counterfactual: its elision is the shared `apply` itself.
//!
//! A failure names the seed, X and the sampled window, the step, Y and the
//! differing option or view field. `ELISION_STATS=1` prints the samples per
//! window; `SEED_FROM=n ELISION_TRACE=1` prints both runs' requests.

use std::collections::HashMap;

mod common;

use common::tables::{table, ALLY_SLICE, PRECON_SLICE, SLICE};
use zvtes::agent::RandomLegal;
use zvtes::decision::{Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::SeatId;
use zvtes::library::{CardType, LIBRARY};

const SEEDS: u64 = 100;
const DECISION_BUDGET: u64 = 200_000;

/// A choice that depends on the cards in the deciding seat's hand.
fn from_hand(e: &Engine, seat: SeatId, c: &Choice) -> bool {
    let hand = e.state.seats[seat.index()].hand.len();
    match c {
        // The virtual hand: the hand, then Fleshforge Chamber's cards.
        Choice::Play { card, .. } => (*card as usize) < hand,
        Choice::BurnForVote { .. }
        | Choice::Discard(_)
        | Choice::Bottom(_)
        | Choice::EmployFromHand { .. } => true,
        Choice::TakeFrom { zone, .. } => *zone == zvtes::decision::Zone::Hand,
        _ => false,
    }
}

/// The option a sampled request is answered with: the one option left
/// once the hand-dependent ones are gone, if exactly one is and the hand
/// is not empty. Not where any card can be discarded (the discard phase, a
/// discard down): no hand elides those.
fn survivor(e: &Engine, req: &DecisionRequest) -> Option<usize> {
    if e.state.seats[req.seat.index()].hand.is_empty()
        || req
            .options
            .iter()
            .any(|o| matches!(o, Choice::Discard(_) | Choice::Bottom(_)))
    {
        return None;
    }
    let rest: Vec<usize> = (0..req.options.len())
        .filter(|&i| !from_hand(e, req.seat, &req.options[i]))
        .collect();
    (rest.len() == 1).then(|| rest[0])
}

/// The library cards X cannot play, one of which fills X's hand: combat
/// cards, modifiers and reactions requiring a clan none of X's minions
/// has, those requiring Disciplines they lack too first.
fn fillers(e: &Engine, x: SeatId) -> Vec<u16> {
    let st = &e.state;
    let mine: Vec<zvtes::ids::MinionId> = (0..st.minions.len())
        .filter(|&i| st.minions[i].in_play && st.minions[i].owner == x)
        .map(|i| zvtes::ids::MinionId(i as u32))
        .collect();
    let clans: Vec<&str> = mine
        .iter()
        .filter_map(|m| st.minions[m.index()].clan())
        .collect();
    let cards: Vec<u16> = LIBRARY
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            zvtes::cards::lookup(*i as u16).is_some()
                && !c.clans.is_empty()
                && !c.disciplines.is_empty()
                && c.types.iter().all(|t| {
                    matches!(
                        t,
                        CardType::Combat | CardType::ActionModifier | CardType::Reaction
                    )
                })
                && !c.clans.iter().any(|k| clans.contains(k))
        })
        .map(|(i, _)| i as u16)
        .collect();
    // Those whose Disciplines X's minions lack first (at the fuzz tables
    // every seat has them all: the clan alone bars them there).
    let lacks = |&card: &u16| {
        LIBRARY[card as usize]
            .disciplines
            .iter()
            .all(|d| mine.iter().all(|&m| st.discipline_level(m, d) == 0))
    };
    let mut cards = cards;
    cards.sort_by_key(|c| !lacks(c));
    cards
}

/// Filler cards tried per sample: X's own options may still read one (a
/// card's intercept counted toward a block's reach, an ability discarding
/// a card), and X is then asked again.
const TRIES: usize = 4;

/// Coverage key of a sampled request: its window, whose turn or action it
/// is, and an out-of-turn master among its plays.
fn key(e: &Engine, req: &DecisionRequest) -> String {
    let x = req.seat;
    let role = match e.action() {
        Some(a) if a.seat == x => "acting",
        Some(_) => "other_action",
        None if e.state.active == x => "active",
        None => "other_turn",
    };
    let master = e.state.active != x
        && req.options.iter().any(|c| match c {
            Choice::Play { card, .. } if from_hand(e, x, c) => {
                let id = e.playable_card(x, *card);
                LIBRARY[id as usize].types.contains(&CardType::Master)
            }
            _ => false,
        });
    format!(
        "{:?}:{role}{}",
        req.window,
        if master { ":master" } else { "" }
    )
}

/// Why a comparison ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Stop {
    /// X's next real request (either run).
    Asked,
    Over,
    /// A card of X's became public in one run and not the same one in the
    /// other (discarded, removed, put in play): Y's look at a hand of one
    /// distinct card elides and discards it (Revelations [aus]).
    Revealed,
    /// Y looked at X's hand (Revelations [aus]) or plays against an open
    /// hand: Y's options list X's cards.
    Looked,
}

/// X's cards that are public: the ash heap, removed cards, the library
/// cards in play (a card played, discarded, burned or put in play shows).
fn public_of(e: &Engine, x: SeatId) -> (Vec<u16>, Vec<u16>, Vec<u16>) {
    let s = &e.state.seats[x.index()];
    let in_play = e
        .state
        .cards
        .iter()
        .filter(|c| c.owner == x)
        .map(|c| c.card)
        .collect();
    (s.ash.clone(), s.removed.clone(), in_play)
}

/// Y's options list X's hand: a look at it (Revelations [aus]).
fn looks_at(e: &Engine, req: &DecisionRequest, x: SeatId) -> bool {
    let hand = zvtes::state::distinct(&e.state.seats[x.index()].hand, |_| true);
    req.window == WindowLabel::ResolutionChoice
        && req.options.len() == hand.len()
        && req
            .options
            .iter()
            .zip(&hand)
            .all(|(o, &c)| *o == Choice::FindCard(c))
}

/// Where two pretty debug dumps first differ: the line, and the path of
/// enclosing fields down to it (`seats[3] > hand`).
fn first_diff(a: &str, b: &str) -> String {
    let (la, lb): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
    let i = la.iter().zip(&lb).take_while(|(x, y)| x == y).count();
    let indent = |l: &str| l.len() - l.trim_start().len();
    // The enclosing lines: each less indented than the one below it.
    let mut open = Vec::new();
    let mut depth = la.get(i).map_or(0, |l| indent(l));
    for k in (0..i).rev() {
        if indent(la[k]) < depth {
            open.push(k);
            depth = indent(la[k]);
        }
    }
    open.reverse();
    let mut path = Vec::new();
    for (n, &k) in open.iter().enumerate() {
        let field = la[k].trim().trim_end_matches(['{', '[', '(', ' ']);
        let field = field.trim_end_matches(':');
        // An element of a list: its index among its siblings.
        if n > 0 && la[open[n - 1]].trim_end().ends_with('[') {
            let d = indent(la[k]);
            let nth = (open[n - 1] + 1..k)
                .filter(|&j| indent(la[j]) == d && !la[j].trim().starts_with(['}', ']', ')']))
                .count();
            if let Some(last) = path.last_mut() {
                *last = format!("{last}[{nth}]");
            }
            continue;
        }
        if !field.is_empty() && !field.chars().next().unwrap().is_uppercase() {
            path.push(field.to_string());
        }
    }
    format!(
        "at {}: {} (actual) vs {} (swapped)",
        path.join(" > "),
        la.get(i).map_or("<end>", |l| l.trim()),
        lb.get(i).map_or("<end>", |l| l.trim()),
    )
}

/// A comparison under way: the clone with X's hand swapped and its step
/// pending against the actual game's.
struct Swap {
    clone: Engine,
    next: Step,
    x: SeatId,
    key: String,
    /// The actual game's decision number at the sample.
    at: usize,
    steps: usize,
}

#[derive(Default)]
struct Stats {
    /// Compared steps per sample key.
    compared: HashMap<String, usize>,
    /// Samples per key (with at least one compared step).
    samples: HashMap<String, usize>,
    stops: HashMap<Stop, usize>,
    qualifying: usize,
    /// Qualifying requests the filler did not elide (X asked again in the
    /// same window).
    no_elision: usize,
}

/// Compare the actual game's `step` with the clone's; `None` to go on.
fn compare(name: &str, seed: u64, e: &Engine, step: &Step, s: &Swap) -> Option<Stop> {
    let (x, c) = (s.x, &s.clone);
    let at = |why: &str| {
        format!(
            "{name} seed {seed}: elision of X = {x:?} at decision {} ({}) leaks, \
             step {} after it: {why}",
            s.at, s.key, s.steps
        )
    };
    let (ra, rc) = match (step, &s.next) {
        (Step::Over(a), Step::Over(b)) => {
            assert_eq!(a, b, "{}", at("the final VPs differ"));
            return Some(Stop::Over);
        }
        (Step::Decision(ra), Step::Decision(rc)) => (ra, rc),
        (Step::Decision(r), Step::Over(_)) if r.seat == x => return Some(Stop::Asked),
        (Step::Over(_), Step::Decision(r)) if r.seat == x => return Some(Stop::Asked),
        (a, b) => panic!(
            "{}",
            at(&format!(
                "one game is over, not the other (actual over: {}, swapped over: {})",
                matches!(a, Step::Over(_)),
                matches!(b, Step::Over(_))
            ))
        ),
    };
    if ra.seat == x || rc.seat == x {
        return Some(Stop::Asked);
    }
    if public_of(e, x) != public_of(c, x) {
        return Some(Stop::Revealed);
    }
    let y = ra.seat;
    let open = e.state.hand_visible_to(x, y) || c.state.hand_visible_to(x, y);
    if looks_at(e, ra, x)
        || looks_at(c, rc, x)
        || (open && (ra.window, &ra.options) != (rc.window, &rc.options))
    {
        return Some(Stop::Looked);
    }
    assert!(
        rc.seat == y,
        "{}",
        at(&format!(
            "the request goes to Y = {y:?} ({:?}) in the actual game, to {:?} ({:?}) \
             with the swapped hand",
            ra.window, rc.seat, rc.window
        ))
    );
    assert!(
        ra.window == rc.window,
        "{}",
        at(&format!(
            "Y = {y:?} is asked in {:?} (actual) vs {:?} (swapped)",
            ra.window, rc.window
        ))
    );
    if ra.options != rc.options {
        let i = ra
            .options
            .iter()
            .zip(&rc.options)
            .take_while(|(a, b)| a == b)
            .count();
        panic!(
            "{}",
            at(&format!(
                "Y = {y:?} in {:?} is offered {} options (actual) vs {} (swapped); \
                 option {i}: {:?} vs {:?}",
                ra.window,
                ra.options.len(),
                rc.options.len(),
                ra.options.get(i),
                rc.options.get(i)
            ))
        );
    }
    for v in 0..e.state.seats.len() {
        let v = SeatId(v as u8);
        if v == x || e.state.hand_visible_to(x, v) || c.state.hand_visible_to(x, v) {
            continue;
        }
        let (va, vc) = (e.state.legal_view(v), c.state.legal_view(v));
        if va != vc {
            panic!(
                "{}",
                at(&format!(
                    "legal_view(Y = {v:?}) differs, while {y:?} decides in {:?}, {}",
                    ra.window,
                    first_diff(&format!("{va:#?}"), &format!("{vc:#?}"))
                ))
            );
        }
    }
    let contexts = |e: &Engine| {
        [
            ("action", format!("{:#?}", e.action())),
            ("combat", format!("{:#?}", e.combat())),
            ("referendum", format!("{:#?}", e.referendum())),
        ]
    };
    for ((what, a), (_, b)) in contexts(e).into_iter().zip(contexts(c)) {
        if a != b {
            panic!(
                "{}",
                at(&format!(
                    "the {what} under way differs, while {y:?} decides in {:?}, {}",
                    ra.window,
                    first_diff(&a, &b)
                ))
            );
        }
    }
    None
}

/// One random game from `start(seed)`, comparing every sampled elision.
fn game(name: &str, seed: u64, start: &dyn Fn(u64) -> Engine, stats: &mut Stats) {
    let mut e = start(seed);
    let mut agent = RandomLegal::new(seed ^ 0x00c0_ffee);
    let mut swap: Option<Swap> = None;
    loop {
        assert!(
            e.decisions < DECISION_BUDGET,
            "decision budget exhausted (seed {seed})"
        );
        let step = e.run();
        if let Some(s) = &mut swap {
            if std::env::var("ELISION_TRACE").is_ok() {
                let show = |st: &Step| match st {
                    Step::Decision(r) => format!("{:?} {:?} {:?}", r.seat, r.window, r.options),
                    Step::Over(_) => "over".into(),
                };
                eprintln!("  actual: {}\n  swapped: {}", show(&step), show(&s.next));
            }
            if let Some(stop) = compare(name, seed, &e, &step, s) {
                *stats.stops.entry(stop).or_default() += 1;
                if s.steps > 0 {
                    *stats.samples.entry(s.key.clone()).or_default() += 1;
                    *stats.compared.entry(s.key.clone()).or_default() += s.steps;
                }
                swap = None;
            } else {
                s.steps += 1;
            }
        }
        let req = match step {
            Step::Over(_) => return,
            Step::Decision(req) => req,
        };
        let mut idx = None;
        if swap.is_none() {
            if let Some(pass) = survivor(&e, &req) {
                stats.qualifying += 1;
                let x = req.seat;
                let n = e.state.seats[x.index()].hand.len();
                for card in fillers(&e, x).into_iter().take(TRIES) {
                    let mut clone = e.clone();
                    clone.state.seats[x.index()].hand = vec![card; n];
                    let next = clone.run();
                    // X's window elided: the clone goes on to another
                    // request (X asked in a later window is a sample with
                    // nothing to compare).
                    let elided = match &next {
                        Step::Decision(r) => (r.seat, r.window) != (x, req.window),
                        Step::Over(_) => true,
                    };
                    if elided {
                        swap = Some(Swap {
                            clone,
                            next,
                            x,
                            key: key(&e, &req),
                            at: e.log.len() + 1,
                            steps: 0,
                        });
                        idx = Some(pass);
                        break;
                    }
                }
                if idx.is_none() {
                    stats.no_elision += 1;
                }
            }
        }
        let sampled = idx.is_some();
        if std::env::var("ELISION_TRACE").is_ok() && (sampled || swap.is_some()) {
            eprintln!(
                "#{} {:?} {:?} {:?} sampled={sampled}",
                e.log.len() + 1,
                req.seat,
                req.window,
                req.options
            );
        }
        let idx = idx.unwrap_or_else(|| agent.choose(&req));
        e.choose(idx);
        // The clone follows the actual game's choice (at the sample, its
        // own window was elided: its next step is already pending).
        if let Some(s) = swap.as_mut().filter(|_| !sampled) {
            s.clone.choose(idx);
            s.next = s.clone.run();
        }
    }
}

/// Plays every seed from `start`, then checks the sampled windows cover
/// `kinds`: key prefixes (`key`) of samples with a compared step.
/// `ELISION_STATS=1` prints the samples per key; `SEED_FROM=n` starts at
/// seed n and `ELISION_TRACE=1` prints both runs' requests.
fn sweep(name: &str, start: &dyn Fn(u64) -> Engine, kinds: &[&str]) {
    let mut stats = Stats::default();
    let from = std::env::var("SEED_FROM").ok().and_then(|v| v.parse().ok());
    for seed in from.unwrap_or(0)..SEEDS {
        game(name, seed, start, &mut stats);
    }
    let mut keys: Vec<_> = stats.compared.iter().collect();
    keys.sort();
    if std::env::var("ELISION_STATS").is_ok() {
        eprintln!(
            "{name}: {} qualifying, {} not eliding, {} samples, {} steps, stops {:?}",
            stats.qualifying,
            stats.no_elision,
            stats.samples.values().sum::<usize>(),
            stats.compared.values().sum::<usize>(),
            stats.stops
        );
        for &(k, n) in &keys {
            eprintln!("  {k}: {} samples, {n} steps", stats.samples[k]);
        }
    }
    if from.is_none() {
        let missing: Vec<&&str> = kinds
            .iter()
            .filter(|k| !keys.iter().any(|(key, _)| key.starts_with(**k)))
            .collect();
        assert!(
            missing.is_empty(),
            "{name}: no elision compared in {missing:?}; sampled: {:?}",
            keys.iter().map(|(k, _)| k).collect::<Vec<_>>()
        );
    }
}

// Coverage keys: `Window:role[:master]`, the role the sampled seat's
// (`acting`: the acting Methuselah, `other_action`: another during an
// action, `other_turn`, `active`), `:master` where its plays include an
// out-of-turn master.

/// Precon crypts and libraries: blocks and reactions, action modifiers,
/// combat (strike, maneuver, press), referendum polling, as-played cancels.
#[test]
fn elision_is_unobservable_from_the_opening() {
    sweep(
        "opening",
        &|seed| Engine::new(seed, 4),
        &[
            "ActionBlock:other_action",
            "ActionBlock:acting",
            "ActionBlocksDeclined:",
            "CombatStrike:",
            "CombatDetermineRange:",
            "CombatPress:",
            "ReferendumPolling:",
            "CardAsPlayed:",
            "MasterPhase:active",
        ],
    );
}

/// The midgame tables: Sudden Reversal against a master played on another's
/// turn, the blood hunt, torpor and ousts.
#[test]
fn elision_is_unobservable_at_midgame_tables() {
    sweep(
        "midgame",
        &|seed| table(seed, SLICE, &[]),
        &[
            "ActionBlock:other_action",
            "ActionBlock:acting",
            "CombatStrike:",
            "CombatDetermineRange:",
            "CombatPress:",
            "ReferendumPolling:",
            "BloodHuntPolling:",
            "CardAsPlayed:other_turn:master",
        ],
    );
}

/// The precon-card tables: Direct Intervention against a modifier or
/// reaction mid-action, Wash, the cancels.
#[test]
fn elision_is_unobservable_at_precon_tables() {
    sweep(
        "precon",
        &|seed| table(seed, PRECON_SLICE, &["OBF"]),
        &[
            "ActionBlock:other_action",
            "ActionBlock:acting",
            "CombatStrike:",
            "CombatDetermineRange:",
            "CombatPress:",
            "CardAsPlayed:other_action:master",
            "CardAsPlayed:other_turn:master",
        ],
    );
}

/// The ally tables: allies' and retainers' combats (Owl Companion's open
/// hand).
#[test]
fn elision_is_unobservable_at_ally_tables() {
    sweep(
        "allies",
        &|seed| table(seed, ALLY_SLICE, &[]),
        &["CombatStrike:", "CombatPrevention:", "ActionBlock:acting"],
    );
}
