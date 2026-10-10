//! How a network does with each precon (owner, 2026-10-09: "quite good
//! with the pure bleed decks, decent with vote decks … really
//! underperforming with wall/combat or more toolboxy decks"). Two
//! measures over the training precons (V5, anniversary, Sabbat), 4-seat
//! tables, decks drawn with at most 20% of their crypt shared (as
//! training draws them):
//! - self-play of network A: per deck, the VPs and game wins of its seats
//!   (the deck under the network's play, against the same play);
//! - A at one seat, B at the three others: per A's deck, A's VPs (where
//!   A gained over B).
//!
//! `cargo run -p zvtes-policy --profile ci --example decks -- <A.zvp>
//! <B.zvp> <games per measure>`.

use std::collections::BTreeMap;
use std::rc::Rc;
use zvtes::agent::Agent;
use zvtes::crypt::{ANNIVERSARY_PRECONS, PRECONS, SABBAT_PRECONS};
use zvtes::decks::Deck;
use zvtes::game::{Engine, Step};
use zvtes::rng::Rng;
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

/// (VPs in half points, game wins, seats) per deck index.
type Tally = BTreeMap<usize, (u64, u64, u64)>;

fn play(nets: [&Rc<Net>; 2], decks: &[Deck], picks: &[usize], at: usize, seed: u64) -> Vec<u32> {
    let seats: Vec<Deck> = picks.iter().map(|&d| decks[d]).collect();
    let mut e = Engine::with_decks(seed, &seats);
    e.time_limit = 75;
    let mut agents: Vec<NetAgent> = (0..4)
        .map(|s| {
            NetAgent::new(
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

fn add(t: &mut Tally, d: usize, vps: &[u32], s: usize) {
    let best = *vps.iter().max().expect("seats");
    let win = vps[s] == best && best >= 4 && vps.iter().filter(|&&v| v == best).count() == 1;
    let e = t.entry(d).or_default();
    e.0 += vps[s] as u64;
    e.1 += win as u64;
    e.2 += 1;
}

fn main() {
    let mut args = std::env::args().skip(1);
    let usage = "usage: decks <A.zvp> <B.zvp> <games per measure>";
    let (a, b) = (args.next().expect(usage), args.next().expect(usage));
    let games: u64 = args.next().and_then(|n| n.parse().ok()).expect(usage);
    let decks = precons();
    let threads = 8u64;
    let (selfplay, versus) = std::thread::scope(|scope| {
        let parts: Vec<_> = (0..threads)
            .map(|t| {
                let (a, b, decks) = (&a, &b, &decks);
                scope.spawn(move || {
                    let load = |p: &str| {
                        Rc::new(Net::load(&std::fs::read(p).expect("a policy")).expect("a policy"))
                    };
                    let (na, nb) = (load(a), load(b));
                    let mut rng = Rng::new(1000 + t);
                    let (mut selfplay, mut versus) = (Tally::new(), Tally::new());
                    for g in (t..games).step_by(threads as usize) {
                        let (picks, _) = table(&mut rng, decks, None);
                        let vps = play([&na, &na], decks, &picks, 0, 20_000 + g);
                        for (s, &d) in picks.iter().enumerate() {
                            add(&mut selfplay, d, &vps, s);
                        }
                        let d = (g as usize) % decks.len();
                        let (picks, at) = table(&mut rng, decks, Some(d));
                        let vps = play([&na, &nb], decks, &picks, at, 40_000 + g);
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
                for (d, (x, y, z)) in from {
                    let e = into.entry(d).or_default();
                    e.0 += x;
                    e.1 += y;
                    e.2 += z;
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
}
