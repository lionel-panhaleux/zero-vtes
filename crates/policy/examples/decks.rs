//! How a network does with each deck (owner, 2026-10-09: "quite good
//! with the pure bleed decks, decent with vote decks … really
//! underperforming with wall/combat or more toolboxy decks"), 4-seat
//! tables, decks drawn with at most 20% of their crypt shared (as
//! training draws them), games ending at 75 player turns (the server's
//! end at the engine's `TURN_LIMIT`, 100). Two measures over the training
//! precons (V5, anniversary, Sabbat):
//! - self-play of network A: per deck, the VPs and game wins of its seats
//!   (the deck under the network's play, against the same play);
//! - A at one seat, B at the three others: per A's deck, A's VPs (where
//!   A gained over B); over every deck, A's VP a seat and game wins with
//!   two standard errors (the training skill's "Serve" reads them).
//!
//! `--file`, the served bot on every TWDA deck the engine keeps (those
//! `GET /catalog` lists): the deck at one seat, precons at the three
//! others, every seat played as the server plays it
//! (`zvtes_policy::profiles::NETWORK`); per deck its games, game wins, VP
//! a game and VP share (its VPs over all those scored at its tables, 0.25
//! even), weakest first, written beside the weights as
//! `<served>.decks.json` with their SHA-256, the engine and the date: what
//! the server serves at `GET /catalog/taught` (`ZVTES_DECKS`), written as
//! the `zvtes_api::DecksMeasure` it reads; "recent": a deck dated within
//! two years of the measure (the training skill's "Serve" writes the file).
//!
//! `cargo run -p zvtes-policy --profile ci --example decks -- <A.zvp>
//! <B.zvp> <games per measure>`, or `-- --file <served.zvp> <games a
//! deck>`.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;
use zvtes::agent::Agent;
use zvtes::crypt::{ANNIVERSARY_PRECONS, PRECONS, SABBAT_PRECONS};
use zvtes::decks::Deck;
use zvtes::game::{Engine, Step};
use zvtes::rng::Rng;
use zvtes::twda::TWDA;
use zvtes_policy::profiles::NETWORK;
use zvtes_policy::{Net, NetAgent};

fn precons() -> Vec<Deck> {
    (0..PRECONS.len())
        .map(Deck::precon)
        .chain((0..ANNIVERSARY_PRECONS.len()).map(Deck::anniversary))
        .chain((0..SABBAT_PRECONS.len()).map(Deck::sabbat))
        .collect()
}

/// Four decks, `first` (if any) at a seat drawn uniformly, none sharing
/// more than 20% of their crypt.
fn table(rng: &mut Rng, decks: &[Deck], first: Option<usize>) -> (Vec<usize>, usize) {
    loop {
        let mut picks: Vec<usize> = (0..4).map(|_| rng.below(decks.len())).collect();
        let at = rng.below(4);
        if let Some(d) = first {
            picks[at] = d;
        }
        let clash =
            (0..4).any(|i| (i + 1..4).any(|j| decks[picks[i]].crypt_share(&decks[picks[j]]) > 0.2));
        if !clash {
            return (picks, at);
        }
    }
}

/// (VPs in half points, game wins, seats, squared half points) per deck index.
type Tally = BTreeMap<usize, (u64, u64, u64, u64)>;

/// A seat's agent: `NetAgent::new` (temperature 1, no nudges) or, as the
/// server plays it, `NETWORK.agent`.
type Seat = fn(Arc<Net>, u64) -> NetAgent;

fn play(
    nets: [&Arc<Net>; 2],
    decks: &[Deck],
    picks: &[usize],
    at: usize,
    seed: u64,
    agent: Seat,
) -> Vec<u32> {
    let seats: Vec<Deck> = picks.iter().map(|&d| decks[d]).collect();
    let mut e = Engine::with_decks(seed, &seats);
    e.time_limit = 75;
    let mut agents: Vec<NetAgent> = (0..4)
        .map(|s| {
            agent(
                if s == at { nets[0] } else { nets[1] }.clone(),
                seed * 10 + s as u64,
            )
        })
        .collect();
    loop {
        match e.run() {
            Step::Decision(req) => {
                let i = agents[req.seat.index()].decide(&e, &req);
                e.choose(i);
                if e.decisions > 30_000 {
                    return e.state.seats.iter().map(|s| s.half_vps).collect();
                }
            }
            Step::Over(vps) => return vps.to_vec(),
        }
    }
}

/// Seat `s` won the game: the most VPs alone, two at least.
fn won(vps: &[u32], s: usize) -> bool {
    let best = *vps.iter().max().expect("seats");
    vps[s] == best && best >= 4 && vps.iter().filter(|&&v| v == best).count() == 1
}

fn add(t: &mut Tally, d: usize, vps: &[u32], s: usize) {
    let e = t.entry(d).or_default();
    e.0 += vps[s] as u64;
    e.1 += won(vps, s) as u64;
    e.2 += 1;
    e.3 += (vps[s] as u64).pow(2);
}

fn main() {
    let mut args = std::env::args().skip(1);
    let usage = "usage: decks <A.zvp> <B.zvp> <games per measure> | decks --file <served.zvp> <games a deck>";
    let (a, b) = (args.next().expect(usage), args.next().expect(usage));
    let games: u64 = args.next().and_then(|n| n.parse().ok()).expect(usage);
    if a == "--file" {
        return deck_file(&b, games);
    }
    let decks = precons();
    let threads = 8u64;
    let (selfplay, versus) = std::thread::scope(|scope| {
        let parts: Vec<_> = (0..threads)
            .map(|t| {
                let (a, b, decks) = (&a, &b, &decks);
                scope.spawn(move || {
                    let load = |p: &str| {
                        Arc::new(Net::load(&std::fs::read(p).expect("a policy")).expect("a policy"))
                    };
                    let (na, nb) = (load(a), load(b));
                    let mut rng = Rng::new(1000 + t);
                    let (mut selfplay, mut versus) = (Tally::new(), Tally::new());
                    for g in (t..games).step_by(threads as usize) {
                        let (picks, _) = table(&mut rng, decks, None);
                        let vps = play([&na, &na], decks, &picks, 0, 20_000 + g, NetAgent::new);
                        for (s, &d) in picks.iter().enumerate() {
                            add(&mut selfplay, d, &vps, s);
                        }
                        let d = (g as usize) % decks.len();
                        let (picks, at) = table(&mut rng, decks, Some(d));
                        let vps = play([&na, &nb], decks, &picks, at, 40_000 + g, NetAgent::new);
                        add(&mut versus, d, &vps, at);
                    }
                    (selfplay, versus)
                })
            })
            .collect();
        let mut out = (Tally::new(), Tally::new());
        for p in parts {
            let (s, v) = p.join().expect("a thread");
            for (into, from) in [(&mut out.0, s), (&mut out.1, v)] {
                for (d, (x, y, z, q)) in from {
                    let e = into.entry(d).or_default();
                    e.0 += x;
                    e.1 += y;
                    e.2 += z;
                    e.3 += q;
                }
            }
        }
        out
    });
    println!("deck | self-play VP a seat (game wins, seats) | A against B: A's VP a seat (seats)");
    let mut rows: Vec<usize> = (0..decks.len()).collect();
    let vp = |t: &Tally, d: usize| {
        t.get(&d)
            .map_or(0.0, |e| e.0 as f64 / 2.0 / e.2.max(1) as f64)
    };
    rows.sort_by(|&x, &y| vp(&selfplay, y).total_cmp(&vp(&selfplay, x)));
    for d in rows {
        let s = selfplay.get(&d).copied().unwrap_or_default();
        let v = versus.get(&d).copied().unwrap_or_default();
        println!(
            "{:45} | {:.2} ({:.0}%, {}) | {:.2} ({})",
            decks[d].name,
            vp(&selfplay, d),
            100.0 * s.1 as f64 / s.2.max(1) as f64,
            s.2,
            vp(&versus, d),
            v.2
        );
    }
    let (h, w, n, q) = versus.values().fold((0, 0, 0, 0), |a, e| {
        (a.0 + e.0, a.1 + e.1, a.2 + e.2, a.3 + e.3)
    });
    let n = n.max(2) as f64;
    let mean = h as f64 / 2.0 / n;
    let var = (q as f64 / 4.0 / n - mean * mean) * n / (n - 1.0);
    let wins = w as f64 / n;
    println!(
        "A against B: VP a seat {mean:.3} ±{:.3}, game wins {:.1}% ±{:.1} (two standard errors), {n} seats",
        2.0 * (var / n).sqrt(),
        100.0 * wins,
        200.0 * (wins * (1.0 - wins) / n).sqrt()
    );
}

/// Today in UTC, `YYYY-MM-DD`.
fn today() -> String {
    let out = std::process::Command::new("date")
        .args(["-u", "+%F"])
        .output()
        .expect("date");
    String::from_utf8(out.stdout).expect("a date").trim().into()
}

/// Per TWDA deck: (its half VPs, game wins, games, every seat's half VPs).
type Rows = BTreeMap<usize, (u64, u64, u64, u64)>;

/// `--file`: the served bot on every TWDA deck, written beside the weights.
fn deck_file(path: &str, games: u64) {
    let bytes = std::fs::read(path).expect("the served weights");
    let weights = format!("{:x}", Sha256::digest(&bytes));
    let net = Arc::new(Net::load(&bytes).expect("a policy"));
    let tasks: Vec<(usize, u64)> = (0..TWDA.len())
        .flat_map(|d| (0..games).map(move |g| (d, g)))
        .collect();
    let threads = 8;
    let rows: Rows = std::thread::scope(|scope| {
        let parts: Vec<_> = (0..threads)
            .map(|t| {
                let (tasks, net) = (&tasks, &net);
                scope.spawn(move || {
                    let mut rows = Rows::new();
                    for &(d, g) in tasks.iter().skip(t).step_by(threads) {
                        // The deck last: a seat drawing it again clashes
                        // with it, so the others are precons.
                        let mut decks = precons();
                        decks.push(Deck::twda(d));
                        let seed = 100_000 + d as u64 * games + g;
                        // The table's draw apart from the game's shuffles.
                        let mut rng = Rng::new(1 << 40 | seed);
                        let (picks, at) = table(&mut rng, &decks, Some(decks.len() - 1));
                        let vps = play([net, net], &decks, &picks, at, seed, |net, seed| {
                            NETWORK.agent(net, seed)
                        });
                        let e = rows.entry(d).or_default();
                        e.0 += vps[at] as u64;
                        e.1 += won(&vps, at) as u64;
                        e.2 += 1;
                        e.3 += vps.iter().map(|&v| v as u64).sum::<u64>();
                    }
                    rows
                })
            })
            .collect();
        let mut rows = Rows::new();
        for p in parts {
            for (d, (h, w, n, all)) in p.join().expect("a thread") {
                let e = rows.entry(d).or_default();
                e.0 += h;
                e.1 += w;
                e.2 += n;
                e.3 += all;
            }
        }
        rows
    });
    let measured = today();
    let year: i32 = measured[..4].parse().expect("a year");
    // Dated within two years: the date two years back, compared as text.
    let since = format!("{}{}", year - 2, &measured[4..]);
    let recent = |d: usize| TWDA[d].date >= since.as_str();
    let share = |d: usize| rows[&d].0 as f64 / rows[&d].3.max(1) as f64;
    let mut order: Vec<usize> = rows.keys().copied().collect();
    order.sort_by(|&a, &b| share(a).total_cmp(&share(b)));
    let round = |x: f64| (x * 1000.0).round() / 1000.0;
    let decks: Vec<zvtes_api::DeckMeasure> = order
        .iter()
        .map(|&d| {
            let deck = &TWDA[d];
            let (h, w, n, _) = rows[&d];
            zvtes_api::DeckMeasure {
                name: deck.name.into(),
                twda: deck.id.into(),
                event: deck.event.into(),
                date: deck.date.into(),
                recent: recent(d),
                games: n as u32,
                wins: w as u32,
                vp: round(h as f64 / 2.0 / n as f64),
                vp_share: round(share(d)),
            }
        })
        .collect();
    let file = zvtes_api::DecksMeasure {
        weights,
        engine: zvtes_host::ENGINE.into(),
        measured,
        measure: format!(
            "the deck at one seat drawn uniformly, precons (V5, anniversary, Sabbat) at the \
             three others, none sharing more than 20% of their crypt; 4 seats, every seat the \
             served bot (temperature {}, its nudges); games ending at 75 player turns; {games} \
             games a deck",
            NETWORK.temperature
        ),
        decks,
    };
    let out = std::path::Path::new(path).with_extension("decks.json");
    let text = serde_json::to_string_pretty(&file).expect("json") + "\n";
    std::fs::write(&out, text).expect("the deck file");
    println!(
        "{}: {} decks, {games} games a deck, weights {}",
        out.display(),
        order.len(),
        &file.weights[..12]
    );
    for keep in [true, false] {
        let set: Vec<usize> = order
            .iter()
            .copied()
            .filter(|&d| recent(d) == keep)
            .collect();
        let (h, all) = set
            .iter()
            .fold((0, 0), |a, d| (a.0 + rows[d].0, a.1 + rows[d].3));
        println!(
            "{}: {} decks, VP share {:.3}",
            if keep { "recent" } else { "older" },
            set.len(),
            h as f64 / all.max(1) as f64
        );
    }
    for &d in order.iter().filter(|&&d| recent(d)).take(10) {
        println!("{:.3} ({} wins) {}", share(d), rows[&d].1, TWDA[d].name);
    }
}
