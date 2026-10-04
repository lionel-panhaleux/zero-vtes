//! Protocol sweep: full games, one seat "human" (answered at random), the
//! others bots, over the shipped decks. Every message round-trips through
//! JSON; copies are unique in every view; a card in play keeps its copy;
//! every option offered to any seat is described; a narrated decision
//! shows no card the viewer may not know.

use std::collections::{BTreeSet, HashMap};

use zvtes::crypt::CRYPT;
use zvtes::decision::{Choice, WindowLabel};
use zvtes::decks::Deck;
use zvtes::game::{Engine, Step};
use zvtes::ids::SeatId;
use zvtes::library::LIBRARY;
use zvtes::referendum::RefStep;
use zvtes::rng::Rng;
use zvtes_protocol::deck::list_of;
use zvtes_protocol::describe::{kind, Describer};
use zvtes_protocol::session::AllocationView;
use zvtes_protocol::track::Tracker;
use zvtes_protocol::view::{table_view, CardView, TableView};
use zvtes_protocol::{AnswerError, GameSpec, Out, Player, SeatSpec, Session};

fn copies(v: &TableView) -> Vec<CardView> {
    let mut out = Vec::new();
    for s in &v.seats {
        out.extend(s.crypt.iter().chain(&s.library).chain(&s.hand));
        out.extend(s.ash_heap.iter().chain(&s.removed).chain(&s.limbo));
        out.extend(s.uncontrolled.iter().map(|u| u.card));
        for m in s.ready.iter().chain(&s.torpor) {
            out.push(m.card);
            for c in &m.cards {
                out.push(c.card);
                out.extend(&c.stored);
            }
        }
        for c in &s.in_play {
            out.push(c.card);
            out.extend(&c.stored);
        }
    }
    out
}

fn check_view(v: &TableView, arena: &mut HashMap<u32, u32>, decks: &[usize]) {
    let all = copies(v);
    // The votes listed add up to the tally; a blocker comes with its
    // intercept, a bleed with its amount.
    if let Some(r) = &v.referendum {
        let tally = |favor: bool| -> u32 {
            r.votes
                .iter()
                .filter(|x| x.favor == favor && !x.canceled)
                .map(|x| x.n as u32)
                .sum()
        };
        assert_eq!((tally(true), tally(false)), (r.votes_for, r.votes_against));
    }
    if let Some(a) = &v.action {
        assert_eq!(a.blocker.is_some(), a.intercept.is_some());
        assert_eq!(a.kind == "bleed", a.bleed.is_some());
    }
    // Crypts and libraries are face down, the viewer's own too.
    for s in &v.seats {
        assert!(s.crypt.iter().chain(&s.library).all(|c| c.id.is_none()));
    }
    // Every card of the surviving decks is shown, none twice (an ousted
    // Methuselah's cards leave the game unless another controls them).
    let low: usize = v
        .seats
        .iter()
        .filter(|s| !s.ousted)
        .map(|s| decks[s.seat as usize])
        .sum();
    let high: usize = decks.iter().sum();
    assert!(
        (low..=high).contains(&all.len()),
        "decision {}: {} cards shown, decks hold {low}..={high}",
        v.decision,
        all.len()
    );
    let set: BTreeSet<u32> = all.iter().map(|c| c.copy).collect();
    assert_eq!(
        set.len(),
        all.len(),
        "duplicate copies at decision {}",
        v.decision
    );
    // A card in play keeps its copy while in play.
    let mut now = HashMap::new();
    for s in &v.seats {
        for m in s.ready.iter().chain(&s.torpor) {
            for c in &m.cards {
                now.insert(c.card_id, c.card.copy);
            }
        }
        for c in &s.in_play {
            now.insert(c.card_id, c.card.copy);
        }
    }
    for (id, copy) in &now {
        if let Some(before) = arena.get(id) {
            assert_eq!(
                before, copy,
                "card in play {id} changed copy at {}",
                v.decision
            );
        }
    }
    *arena = now;
}

/// The omniscient view shows each card between the surviving decks'
/// copies and every deck's: an ousted Methuselah's cards leave the game,
/// except those another Methuselah controls (rulebook "Ousting").
fn check_ids(e: &Engine, tracker: &mut Tracker, decks: &[Deck]) {
    let v = table_view(e, None, tracker);
    let mut shown: HashMap<Option<u32>, usize> = HashMap::new();
    for c in copies(&v) {
        *shown.entry(c.id).or_default() += 1;
    }
    let mut low: HashMap<Option<u32>, usize> = HashMap::new();
    let mut high: HashMap<Option<u32>, usize> = HashMap::new();
    for (i, d) in decks.iter().enumerate() {
        let ids = d
            .crypt
            .iter()
            .map(|&(c, n)| (CRYPT[c as usize].id, n))
            .chain(d.library.iter().map(|&(c, n)| (LIBRARY[c as usize].id, n)));
        for (id, n) in ids {
            *high.entry(Some(id)).or_default() += n as usize;
            if !v.seats[i].ousted {
                *low.entry(Some(id)).or_default() += n as usize;
            }
        }
    }
    for (id, &n) in &shown {
        let hi = high.get(id).copied().unwrap_or(0);
        assert!(
            n <= hi,
            "decision {}: {id:?} shown {n} times, decks hold {hi}",
            v.decision
        );
    }
    for (id, &n) in &low {
        let got = shown.get(id).copied().unwrap_or(0);
        assert!(
            got >= n,
            "decision {}: {id:?} shown {got} times, surviving decks hold {n}",
            v.decision
        );
    }
}

fn roundtrip(out: &Out) {
    let json = serde_json::to_string(out).expect("serialize");
    let back: Out = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(&back, out);
}

/// The events narrate the ends: an action or a referendum seen at one
/// decision and gone at the next is told with the next one's message (the
/// first message about it). A combat is told begun, its strikes, then
/// ended.
struct Ends {
    /// The last decision seen: its number, an action and a referendum.
    last: Option<(u64, bool, bool)>,
    /// A combat told begun, not yet told ended; unknown right after an
    /// undo (the decision taken back may have begun one).
    in_combat: Option<bool>,
}

impl Default for Ends {
    fn default() -> Self {
        Ends {
            last: None,
            in_combat: Some(false),
        }
    }
}

impl Ends {
    fn see(&mut self, out: &Out, seed: u64) {
        let (decision, events, view) = match out {
            Out::Decision {
                decision,
                events,
                view,
                ..
            }
            | Out::Decided {
                decision,
                events,
                view,
                ..
            } => (*decision, events, view),
            Out::Over { events, view, .. } => (u64::MAX, events, view),
        };
        assert!(events.iter().all(|e| !e.is_empty()));
        // A combat is told begun, its strikes, then ended (a game may end
        // during one).
        for e in events {
            if e.starts_with("Combat: ") {
                assert!(
                    self.in_combat != Some(true),
                    "seed {seed}: a combat begun twice: {events:?}"
                );
                self.in_combat = Some(true);
            } else if e.starts_with("Round ") {
                assert!(
                    self.in_combat != Some(false),
                    "seed {seed}: strikes out of combat: {events:?}"
                );
            } else if e == "The combat ends" {
                assert!(
                    self.in_combat != Some(false),
                    "seed {seed}: an untold combat ended: {events:?}"
                );
                self.in_combat = Some(false);
            }
        }
        let (action, referendum) = (view.action.is_some(), view.referendum.is_some());
        if let Some((d, had_action, had_referendum)) = self.last {
            if decision == d {
                return;
            }
            let told = |words: &[&str]| events.iter().any(|e| words.iter().any(|w| e.contains(w)));
            if had_action && !action {
                assert!(
                    told(&[" succeeds", " is blocked", " fails", " fizzles", " ends"]),
                    "seed {seed} decision {decision}: an action's end untold: {events:?}"
                );
            }
            if had_referendum && !referendum {
                assert!(
                    told(&[" passes (", " fails (", " is canceled"]),
                    "seed {seed} decision {decision}: a referendum's end untold: {events:?}"
                );
            }
        }
        self.last = Some((decision, action, referendum));
    }
}

fn spec(seed: u64, decks: &[Deck]) -> GameSpec {
    let styles = ["aggro_bleed", "wall", "random"];
    GameSpec {
        seed,
        time_limit: Some(40),
        seats: decks
            .iter()
            .enumerate()
            .map(|(i, d)| SeatSpec {
                name: format!("P{}", i + 1),
                deck: list_of(d),
                player: if i == 0 {
                    Player::Human
                } else {
                    Player::Bot {
                        style: styles[(seed as usize + i) % 3].into(),
                    }
                },
            })
            .collect(),
    }
}

/// Random valid amounts for an allocation: 1 to `min_nonzero` recipients,
/// then the rest point by point within the caps.
fn amounts(a: &AllocationView, rng: &mut Rng) -> Vec<u8> {
    let n = a.recipients.len();
    let cap = |i: usize| a.recipients[i].cap.unwrap_or(u8::MAX);
    let mut out = vec![0u8; n];
    let mut left = a.points;
    while out.iter().filter(|&&x| x > 0).count() < a.min_nonzero as usize {
        let i = rng.below(n);
        if out[i] == 0 && cap(i) > 0 {
            out[i] = 1;
            left -= 1;
        }
    }
    while left > 0 {
        let i = rng.below(n);
        if out[i] < cap(i) {
            out[i] += 1;
            left -= 1;
        }
    }
    out
}

/// What the allocations in the engine's referendum are, in order: seats,
/// then cards, then a shortfall's re-allocation.
fn allocated(e: &Engine) -> Option<Vec<u8>> {
    let t = &e.referendum()?.terms;
    Some(
        t.alloc
            .iter()
            .map(|(_, n)| *n)
            .chain(t.card_alloc.iter().map(|(_, n)| *n))
            .chain(t.realloc.iter().map(|(_, n)| *n))
            .collect(),
    )
}

/// Describe every option of the pending request as its own seat sees it
/// (exercises every variant any seat is offered, bots' included).
fn describe_all(
    e: &Engine,
    trackers: &mut HashMap<u8, Tracker>,
    seen: &mut BTreeSet<&'static str>,
    prompts: &mut u32,
) {
    let mut probe = e.clone();
    let Step::Decision(req) = probe.run() else {
        return;
    };
    let tracker = trackers.entry(req.seat.0).or_default();
    let view = table_view(&probe, Some(req.seat), tracker);
    let d = Describer {
        e: &probe,
        seat: req.seat,
        window: req.window,
        view: &view,
        names: &[],
        narrate: false,
        offered: &req.options,
        about: None,
    };
    // A choice an effect asks as it resolves says what it is for.
    if probe.interrupt().is_some()
        && matches!(
            req.window,
            WindowLabel::ResolutionChoice | WindowLabel::EntryEffect
        )
    {
        assert!(d.prompt().is_some(), "{:?}", probe.interrupt());
        *prompts += 1;
    }
    for (i, c) in req.options.iter().enumerate() {
        let o = d.describe(i, c);
        assert!(!o.text.is_empty());
        // A play names its mode: the same as the engine's, with the tags
        // the text prints ("at superior [DOM]").
        if let Choice::Play { mode, .. } = *c {
            let m = o.mode.as_ref().expect("a play's mode");
            assert_eq!(m.index, mode);
            for t in &m.tags {
                assert!(o.text.contains(&format!("[{t}]")), "{}: {t}", o.text);
            }
        }
        seen.insert(kind(c));
    }
}

#[test]
fn an_unknown_bot_style_is_refused_up_front() {
    let decks = [Deck::precon(0), Deck::precon(1)];
    let mut sp = spec(0, &decks);
    sp.seats[1].player = Player::Bot {
        style: "netwrok".into(),
    };
    assert!(matches!(
        Session::new(sp.clone()),
        Err(zvtes_protocol::SpecError::Style { seat: 1, .. })
    ));
    assert!(Session::with_styles(sp, &["netwrok"]).is_ok());
}

/// A human answer, for the sweep's undo checks: the options it saw, and
/// its library and crypt sizes then.
type Answered = (Vec<String>, (usize, usize));

#[test]
fn sessions_over_the_shipped_decks() {
    let all: Vec<Deck> = Deck::all().collect();
    let seeds: u64 = std::env::var("PROTOCOL_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(24);
    let mut seen = BTreeSet::new();
    let mut prompts = 0;
    let mut undone = 0;
    let (mut allocations, mut refused, mut ordered, mut cast_by_order) = (0, 0, 0, 0);
    for seed in 0..seeds {
        let mut rng = Rng::new(seed);
        let decks: Vec<Deck> = (0..4).map(|_| all[rng.below(all.len())]).collect();
        let cards: Vec<usize> = decks
            .iter()
            .map(|d| {
                d.crypt
                    .iter()
                    .chain(d.library)
                    .map(|&(_, n)| n as usize)
                    .sum()
            })
            .collect();
        let mut s = Session::new(spec(seed, &decks)).expect("spec");
        let mut arena = HashMap::new();
        let mut trackers = HashMap::new();
        let mut omniscient = Tracker::default();
        let mut n = 0;
        // The side of the human's standing vote order, while it stands.
        let mut standing: Option<bool> = None;
        // Who took each logged decision; for the human's own answers, the
        // options it saw and its library and crypt sizes then (undo checks).
        let mut deciders: Vec<(u8, Option<Answered>)> = Vec::new();
        // The decision an undo takes back, asked again: its number and options.
        let mut again: Option<(u64, Vec<String>)> = None;
        let mut ends = Ends::default();
        loop {
            n += 1;
            assert!(n < 20_000, "seed {seed}: no end");
            describe_all(s.engine(), &mut trackers, &mut seen, &mut prompts);
            check_ids(s.engine(), &mut omniscient, &decks);
            assert_eq!(deciders.len(), s.engine().log.len(), "seed {seed}");
            let out = s.advance();
            roundtrip(&out);
            ends.see(&out, seed);
            if !s
                .engine()
                .referendum()
                .is_some_and(|r| r.step == RefStep::Polling)
            {
                standing = None;
            }
            match &out {
                Out::Decision {
                    decision,
                    options,
                    view,
                    allocation,
                    votes,
                    undo,
                    ..
                } => {
                    check_view(view, &mut arena, &cards);
                    let texts: Vec<String> = options.iter().map(|o| o.text.clone()).collect();
                    if let Some(asked) = again.take() {
                        assert_eq!(
                            asked,
                            (*decision, texts.clone()),
                            "seed {seed}: asked again"
                        );
                    }
                    let sizes = (view.seats[0].library.len(), view.seats[0].crypt.len());
                    // A standing order leaves no vote of its side to cast.
                    if let (Some(f), Some(v)) = (standing, votes) {
                        let left = if f { v.favor } else { v.against };
                        assert_eq!(left, 0, "seed {seed}: a vote left to the order");
                    }
                    // An allocation answered at once: refused when it does
                    // not add up (nothing changes), else applied as given.
                    if let Some(a) = allocation.as_ref().filter(|_| rng.below(2) == 0) {
                        let before = s.engine().log.len();
                        let mut bad = amounts(a, &mut rng);
                        bad[0] += 1;
                        assert!(matches!(
                            s.allocate(*decision, &bad),
                            Err(AnswerError::Allocation { .. })
                        ));
                        assert_eq!(s.engine().log.len(), before);
                        refused += 1;
                        let good = amounts(a, &mut rng);
                        let had = allocated(s.engine()).unwrap_or_default().len();
                        let outs = s.allocate(*decision, &good).expect("valid amounts");
                        assert!(!outs.is_empty());
                        for o in &outs {
                            roundtrip(o);
                            ends.see(o, seed);
                            assert!(matches!(o, Out::Decided { seat: 0, .. }));
                            deciders.push((0, None));
                        }
                        // Still in the referendum (terms, polling): as given.
                        if let Some(now) = allocated(s.engine()) {
                            if now.len() >= had + good.len() {
                                assert_eq!(now[had..had + good.len()], good[..], "seed {seed}");
                            }
                        }
                        allocations += 1;
                        continue;
                    }
                    if let Some(v) = votes.filter(|_| standing.is_none() && rng.below(3) == 0) {
                        let favor = v.favor > 0 && (v.against == 0 || rng.below(2) == 0);
                        let o = s.vote_all(*decision, favor).expect("votes to cast");
                        roundtrip(&o);
                        ends.see(&o, seed);
                        deciders.push((0, None));
                        standing = Some(favor);
                        ordered += 1;
                        continue;
                    }
                    // The human's last decision is taken back, and asked
                    // again: nothing was decided since, it drew nothing.
                    if *undo && rng.below(2) == 0 {
                        undone += 1;
                        let (seat, mine) = deciders.pop().expect("a decision to undo");
                        assert_eq!(seat, 0, "seed {seed}: decided since");
                        if let Some((texts, (library, crypt))) = mine {
                            assert!(
                                sizes.0 >= library && sizes.1 >= crypt,
                                "seed {seed}: drew since"
                            );
                            again = Some((decision - 1, texts));
                        }
                        let before = s.engine().log.len();
                        s.undo().expect("undo");
                        ends = Ends {
                            in_combat: None,
                            ..Ends::default()
                        };
                        assert_eq!(s.engine().log.len(), before - 1);
                        continue;
                    }
                    let i = rng.below(options.len());
                    let answered = s.answer(*decision, i).expect("answer");
                    roundtrip(&answered);
                    ends.see(&answered, seed);
                    deciders.push((0, Some((texts, sizes))));
                    // A stale answer is refused.
                    assert!(s.answer(*decision, i).is_err());
                }
                Out::Decided {
                    seat, choice, view, ..
                } => {
                    check_view(view, &mut arena, &cards);
                    deciders.push((*seat, None));
                    if *seat == 0 && standing.is_some() {
                        assert!(
                            matches!(choice.kind.as_str(), "cast_votes" | "cast_card_vote"),
                            "seed {seed}: the order casts votes only"
                        );
                        cast_by_order += 1;
                    }
                    if *seat != 0 && matches!(choice.kind.as_str(), "bottom" | "find_card") {
                        assert_eq!(choice.card.and_then(|c| c.id), None, "seed {seed}: leaked");
                    }
                }
                Out::Over { vps, view, .. } => {
                    check_view(view, &mut arena, &cards);
                    assert_eq!(vps.len(), 4);
                    break;
                }
            }
        }
        // The replay reproduces the game.
        let r = s.replay();
        let mut again = Session::new(r.spec.clone()).expect("replay spec");
        let mut e = again.engine().clone();
        for &i in &r.log {
            let Step::Decision(_) = e.run() else {
                panic!("replay ended early")
            };
            e.choose(i);
        }
        assert!(matches!(e.run(), Step::Over(_)));
        assert_eq!(e.state.hash(), s.engine().state.hash());
        let _ = again.view(Some(SeatId(0)));
    }
    let missing: Vec<&str> = [
        "pass",
        "play",
        "use_ability",
        "discard",
        "take_action",
        "declare_block",
        "cast_votes",
        "transfer",
        "draw_crypt",
        "influence_out",
        "hand_strike",
    ]
    .into_iter()
    .filter(|k| !seen.contains(k))
    .collect();
    assert!(missing.is_empty(), "never offered: {missing:?}");
    eprintln!("decisions taken back: {undone}, resolution prompts: {prompts}");
    eprintln!(
        "allocations at once: {allocations} ({refused} refused first); \
         vote orders: {ordered}, later votes cast by them: {cast_by_order}"
    );
    eprintln!("option kinds described: {}: {seen:?}", seen.len());
}

/// Referendum shortcuts, aimed at: every seat human, on the decks holding
/// a card that allocates points; those cards played when offered, the
/// rest at random. Allocations answered at once (refused first when they
/// do not add up); standing vote orders cast the seat's later votes and
/// end with the polling.
#[test]
fn allocations_and_vote_orders_at_once() {
    const ALLOCATING: [&str; 6] = [
        "Camarilla's Iron Fist",
        "Conservative Agitation",
        "Kine Resources Contested",
        "Parity Shift",
        "Reckless Agitation",
        "Revolutionary Council",
    ];
    let holds = |d: &Deck| {
        d.library
            .iter()
            .any(|&(c, _)| ALLOCATING.contains(&LIBRARY[c as usize].name))
    };
    let pool: Vec<Deck> = Deck::all().filter(holds).collect();
    assert!(!pool.is_empty());
    let (mut allocations, mut ordered, mut cast_by_order) = (0, 0, 0);
    let (mut to_cards, mut capped) = (0, 0);
    for seed in 0..300u64 {
        let mut rng = Rng::new(seed);
        let decks: Vec<Deck> = (0..4).map(|_| pool[rng.below(pool.len())]).collect();
        let mut sp = spec(seed, &decks);
        for seat in &mut sp.seats {
            seat.player = Player::Human;
        }
        let mut s = Session::new(sp).expect("spec");
        let mut standing: Vec<(u8, bool)> = Vec::new();
        for _ in 0..20_000 {
            let out = s.advance();
            if !s
                .engine()
                .referendum()
                .is_some_and(|r| r.step == RefStep::Polling)
            {
                standing.clear();
            }
            match &out {
                Out::Over { .. } => break,
                Out::Decided { seat, choice, .. } => {
                    // Only a standing order answers a human seat by itself.
                    assert!(
                        standing.iter().any(|&(s, _)| s == *seat),
                        "seed {seed}: a decision nobody took"
                    );
                    let favor = standing.iter().find(|&&(s, _)| s == *seat).unwrap().1;
                    assert!(matches!(
                        choice.kind.as_str(),
                        "cast_votes" | "cast_card_vote"
                    ));
                    assert_eq!(choice.favor, Some(favor));
                    cast_by_order += 1;
                }
                Out::Decision {
                    decision,
                    seat,
                    options,
                    allocation,
                    votes,
                    ..
                } => {
                    if let Some(&(_, f)) = standing.iter().find(|&&(s, _)| s == *seat) {
                        let v = votes.unwrap_or(zvtes_protocol::session::VoteTotals {
                            favor: 0,
                            against: 0,
                        });
                        assert_eq!(if f { v.favor } else { v.against }, 0, "seed {seed}");
                    }
                    if let Some(a) = allocation {
                        to_cards += a.recipients.iter().any(|r| r.card.is_some()) as u32;
                        capped += a.recipients.iter().any(|r| r.cap.is_some()) as u32;
                        let before = s.engine().log.len();
                        let mut bad = amounts(a, &mut rng);
                        let last = bad.len() - 1;
                        bad[last] += 1;
                        assert!(matches!(
                            s.allocate(*decision, &bad),
                            Err(AnswerError::Allocation { .. })
                        ));
                        assert!(matches!(
                            s.allocate(*decision, &bad[..last]),
                            Err(AnswerError::Allocation { .. })
                        ));
                        assert_eq!(s.engine().log.len(), before, "refused: unchanged");
                        let good = amounts(a, &mut rng);
                        let had = allocated(s.engine()).unwrap_or_default().len();
                        let outs = s.allocate(*decision, &good).expect("valid amounts");
                        assert!(!outs.is_empty() && outs.len() <= good.len());
                        if let Some(now) = allocated(s.engine()) {
                            if now.len() >= had + good.len() {
                                assert_eq!(now[had..had + good.len()], good[..], "seed {seed}");
                            }
                        }
                        allocations += 1;
                        continue;
                    }
                    if let Some(v) = votes.filter(|_| rng.below(2) == 0) {
                        let favor = v.favor > 0 && (v.against == 0 || rng.below(2) == 0);
                        s.vote_all(*decision, favor).expect("votes to cast");
                        standing.push((*seat, favor));
                        ordered += 1;
                        continue;
                    }
                    let aimed = options.iter().position(|o| {
                        o.kind == "play" && ALLOCATING.iter().any(|n| o.text.contains(n))
                    });
                    let i = aimed.unwrap_or_else(|| rng.below(options.len()));
                    s.answer(*decision, i).expect("answer");
                }
            }
        }
        if allocations >= 10 && cast_by_order >= 10 {
            break;
        }
    }
    eprintln!(
        "allocations {allocations} ({to_cards} to cards, {capped} capped), \
         vote orders {ordered}, votes cast by them {cast_by_order}"
    );
    assert!(allocations >= 10 && cast_by_order >= 10);
}
