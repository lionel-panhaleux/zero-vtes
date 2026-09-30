//! Protocol sweep: full games, one seat "human" (answered at random), the
//! others bots, over the shipped decks. Every message round-trips through
//! JSON; copies are unique in every view; a card in play keeps its copy;
//! every option offered to any seat is described; a narrated decision
//! shows no card the viewer may not know.

use std::collections::{BTreeSet, HashMap};

use zvtes::crypt::CRYPT;
use zvtes::decks::Deck;
use zvtes::game::{Engine, Step};
use zvtes::ids::SeatId;
use zvtes::library::LIBRARY;
use zvtes::rng::Rng;
use zvtes_protocol::deck::list_of;
use zvtes_protocol::describe::{kind, Describer};
use zvtes_protocol::track::Tracker;
use zvtes_protocol::view::{table_view, CardView, TableView};
use zvtes_protocol::{GameSpec, Out, Player, SeatSpec, Session};

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

/// Describe every option of the pending request as its own seat sees it
/// (exercises every variant any seat is offered, bots' included).
fn describe_all(
    e: &Engine,
    trackers: &mut HashMap<u8, Tracker>,
    seen: &mut BTreeSet<&'static str>,
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
    };
    for (i, c) in req.options.iter().enumerate() {
        let o = d.describe(i, c);
        assert!(!o.text.is_empty());
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

#[test]
fn sessions_over_the_shipped_decks() {
    let all: Vec<Deck> = Deck::all().collect();
    let seeds: u64 = std::env::var("PROTOCOL_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(24);
    let mut seen = BTreeSet::new();
    let mut undone = 0;
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
        loop {
            n += 1;
            assert!(n < 20_000, "seed {seed}: no end");
            describe_all(s.engine(), &mut trackers, &mut seen);
            check_ids(s.engine(), &mut omniscient, &decks);
            let out = s.advance();
            roundtrip(&out);
            match &out {
                Out::Decision {
                    decision,
                    options,
                    view,
                    ..
                } => {
                    check_view(view, &mut arena, &cards);
                    // A step of a multi-step choice is taken back, and asked again.
                    if let Out::Decision { undo: true, .. } = &out {
                        if rng.below(2) == 0 {
                            undone += 1;
                            let before = s.engine().log.len();
                            s.undo().expect("undo");
                            assert_eq!(s.engine().log.len(), before - 1);
                            continue;
                        }
                    }
                    let i = rng.below(options.len());
                    let answered = s.answer(*decision, i).expect("answer");
                    roundtrip(&answered);
                    // A stale answer is refused.
                    assert!(s.answer(*decision, i).is_err());
                }
                Out::Decided {
                    seat, choice, view, ..
                } => {
                    check_view(view, &mut arena, &cards);
                    if *seat != 0 && matches!(choice.kind.as_str(), "bottom" | "find_card") {
                        assert_eq!(choice.card.and_then(|c| c.id), None, "seed {seed}: leaked");
                    }
                }
                Out::Over { vps, view } => {
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
    eprintln!("multi-step choices taken back: {undone}");
    eprintln!("option kinds described: {}: {seen:?}", seen.len());
}
