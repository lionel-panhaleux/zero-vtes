//! Decision-time search in the minion phase (DESIGN §11.2 "Search in the
//! minion phase"; owner, 2026-10-25): what searching the
//! network's own choices would gain, before any training on it. At each
//! minion-phase choice of the searching seat, the network's `k` favourite
//! options are each played out `n` times to the start of the seat's next
//! turn, every seat playing the network, and scored by the position's
//! potential there (`train.position --phi`: a linear fit of the seat's
//! result on `zvtes_env::position`), or by the result where the game ended
//! or the seat was ousted; the best mean is taken. With `--ab W`, a
//! play-out's score also gains W for each A&B the seat makes in it (its
//! actions declared, blocks landed and hindrances): steering toward more
//! of them (owner, 2026-10-25), the potential alone steering toward the
//! result.
//!
//! The oracle (the default) plays out the true game: every hidden card
//! where it is, the libraries in their order — the upper bound of what
//! search could gain. `--fair` re-deals before each play-out the other
//! seats' hands with their libraries (a hand's size kept) and every
//! library and crypt; it leaves the others' face-down uncontrolled
//! vampires and cards stored face down where they are, and re-deals a
//! hand the seat was shown too.
//!
//! Paired games: each seed played twice, the searching seat (seat `g % 4`)
//! with search and without, the others the network; per variant, the
//! seat's VPs, game wins, A&B a round (its actions declared, blocks
//! landed and hindrances, over its own turns) and its prey's R (twice
//! its pool plus the blood on its minions in play) change over its own
//! turns, each a mean of per-game ratios (not `waste`'s ratio of
//! totals), with the paired differences.
//!
//! `cargo run -p zvtes-policy --profile ci --example search -- <net.zvp>
//! <phi.json> <games> [--k 4] [--n 4] [--ab 0] [--fair]`.

use std::sync::Arc;
use zvtes::agent::Agent;
use zvtes::copies::{CopyId, Pile};
use zvtes::decision::{DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::SeatId;
use zvtes::profile::Count;
use zvtes::rng::Rng;
use zvtes_env::{obs, position};
use zvtes_policy::{mask, Net, NetAgent};

/// Decisions a play-out may take before it is scored where it stands.
const PLAYOUT_CAP: u64 = 4_000;

struct Phi {
    weights: Vec<f32>,
    intercept: f32,
}

/// Whether seat `s` won: the most VPs alone, 2 or more.
fn won(half_vps: &[u32], s: usize) -> bool {
    let best = *half_vps.iter().max().expect("seats");
    half_vps[s] == best && best >= 4 && half_vps.iter().filter(|&&v| v == best).count() == 1
}

/// A seat's result as training rewards it: (VPs + 2 for the game win) / 5.
fn result(half_vps: &[u32], s: usize) -> f32 {
    (half_vps[s] as f32 / 2.0 + if won(half_vps, s) { 2.0 } else { 0.0 }) / 5.0
}

fn value(e: &Engine, s: SeatId, phi: &Phi) -> f32 {
    let seat = &e.state.seats[s.index()];
    if seat.ousted {
        return seat.half_vps as f32 / 2.0 / 5.0;
    }
    let f = position::features(&e.state, s);
    phi.intercept + f.iter().zip(&phi.weights).map(|(x, w)| x * w).sum::<f32>()
}

/// The fair search's deal: what `s` cannot see shuffled again — the other
/// seats' hands with their libraries, every library and crypt.
fn redeal(e: &mut Engine, s: SeatId, rng: &mut Rng) {
    fn drain(p: &mut Pile) -> Vec<(u16, CopyId)> {
        (0..p.len()).map(|_| p.take(p.len() - 1)).collect()
    }
    fn shuffle(v: &mut [(u16, CopyId)], rng: &mut Rng) {
        for i in (1..v.len()).rev() {
            v.swap(i, rng.below(i + 1));
        }
    }
    // A card held but owned by another seat (rules-qa 96.2) stays in hand.
    let foreign: Vec<CopyId> = e.state.foreign.iter().map(|(c, _)| **c).collect();
    for (i, seat) in e.state.seats.iter_mut().enumerate() {
        let mut library = drain(&mut seat.library);
        let held = if i == s.index() {
            0
        } else {
            let (kept, hand): (Vec<_>, Vec<_>) = drain(&mut seat.hand)
                .into_iter()
                .partition(|(_, c)| foreign.contains(c));
            let n = hand.len();
            for (card, copy) in kept {
                seat.hand.put(card, copy);
            }
            library.extend(hand);
            n
        };
        shuffle(&mut library, rng);
        for (k, (card, copy)) in library.into_iter().enumerate() {
            if k < held {
                seat.hand.put(card, copy);
            } else {
                seat.library.put(card, copy);
            }
        }
        let mut crypt = drain(&mut seat.crypt);
        shuffle(&mut crypt, rng);
        for (card, copy) in crypt {
            seat.crypt.put(card, copy);
        }
    }
}

struct Search {
    net: Arc<Net>,
    base: NetAgent,
    phi: Arc<Phi>,
    k: usize,
    n: usize,
    fair: bool,
    ab: f32,
    rng: Rng,
}

/// The seat's A&B so far: actions declared, blocks landed, hindrances.
fn ab(e: &Engine, s: SeatId) -> u16 {
    let p = &e.profiles[s.index()];
    p.get(Count::Actions) + p.get(Count::BlocksLanded) + p.get(Count::Hindrances)
}

impl Search {
    /// The `k` options the network rates highest among those it may take.
    fn candidates(&self, e: &Engine, req: &DecisionRequest) -> Vec<usize> {
        let allowed = mask(e, req, true);
        let (logits, _) = self.net.forward(&obs::encode(e, req, self.net.version));
        let mut idx: Vec<usize> = (0..req.options.len())
            .filter(|&i| allowed.as_ref().is_none_or(|m| m[i]))
            .collect();
        idx.sort_by(|&a, &b| logits[b].total_cmp(&logits[a]));
        idx.truncate(self.k);
        idx
    }

    fn playout(&mut self, e: &Engine, req: &DecisionRequest, i: usize) -> f32 {
        let me = req.seat;
        let before = ab(e, me);
        let gained = |e: &Engine, ab_w: f32| ab_w * (ab(e, me) - before) as f32;
        let mut e = e.clone();
        if self.fair {
            redeal(&mut e, me, &mut self.rng);
        }
        e.choose(i);
        let start = e.state.turn;
        let n = e.state.seats.len();
        let mut agents: Vec<NetAgent> = (0..n)
            .map(|_| NetAgent::new(self.net.clone(), self.rng.next_u64()))
            .collect();
        let limit = e.decisions + PLAYOUT_CAP;
        loop {
            match e.run() {
                Step::Over(vps) => return result(&vps, me.index()) + gained(&e, self.ab),
                Step::Decision(r) => {
                    let next_turn = e.state.turn != start && e.state.active == me;
                    if next_turn || e.state.seats[me.index()].ousted || e.decisions > limit {
                        return value(&e, me, &self.phi) + gained(&e, self.ab);
                    }
                    let c = agents[r.seat.index()].decide(&e, &r);
                    e.choose(c);
                }
            }
        }
    }
}

impl Agent for Search {
    fn decide(&mut self, e: &Engine, req: &DecisionRequest) -> usize {
        if req.window != WindowLabel::MinionPhase {
            return self.base.decide(e, req);
        }
        let cands = self.candidates(e, req);
        if cands.len() < 2 {
            return self.base.decide(e, req);
        }
        let mut best = (f32::MIN, cands[0]);
        for &c in &cands {
            let v = (0..self.n).map(|_| self.playout(e, req, c)).sum::<f32>() / self.n as f32;
            if v > best.0 {
                best = (v, c);
            }
        }
        best.1
    }

    fn fork(&self) -> Box<dyn Agent> {
        Box::new(Search {
            net: self.net.clone(),
            base: self.base.clone(),
            phi: self.phi.clone(),
            k: self.k,
            n: self.n,
            fair: self.fair,
            ab: self.ab,
            rng: self.rng.clone(),
        })
    }
}

fn resource(e: &Engine, s: SeatId) -> i64 {
    let blood: i64 = e
        .state
        .minions
        .iter()
        .filter(|m| m.owner == s && m.in_play)
        .map(|m| m.blood as i64)
        .sum();
    2 * e.state.seats[s.index()].pool as i64 + blood
}

/// One seat's game: [VPs, game win, A&B, own turns, its prey's R change].
type Line = [f64; 5];

fn play(
    net: &Arc<Net>,
    phi: &Arc<Phi>,
    seed: u64,
    search: Option<(usize, usize, bool, f32)>,
) -> Line {
    let mut e = Engine::new(seed, 4);
    e.time_limit = 75;
    let at = SeatId((seed % 4) as u8);
    let mut agents: Vec<Box<dyn Agent>> = (0..4u64)
        .map(|s| -> Box<dyn Agent> {
            let base = NetAgent::new(net.clone(), seed * 10 + s);
            match search {
                Some((k, n, fair, ab)) if s as u8 == at.0 => Box::new(Search {
                    net: net.clone(),
                    base,
                    phi: phi.clone(),
                    k,
                    n,
                    fair,
                    ab,
                    rng: Rng::new(seed * 7919 + 1),
                }),
                _ => Box::new(base),
            }
        })
        .collect();
    let (mut turns, mut prey_r) = (0.0, 0i64);
    let mut open: Option<(u32, SeatId, i64)> = None;
    let close = |e: &Engine, open: &mut Option<(u32, SeatId, i64)>, prey_r: &mut i64| {
        if let Some((_, p, r)) = open.take() {
            *prey_r += resource(e, p) - r;
        }
    };
    let vps = loop {
        match e.run() {
            Step::Over(vps) => break vps.to_vec(),
            Step::Decision(req) => {
                if open.is_none_or(|(t, ..)| t != e.state.turn) {
                    close(&e, &mut open, &mut prey_r);
                    if e.state.active == at {
                        let p = e.state.prey(at);
                        open = Some((e.state.turn, p, resource(&e, p)));
                        turns += 1.0;
                    }
                }
                let i = agents[req.seat.index()].decide(&e, &req);
                e.choose(i);
                if e.decisions > 30_000 {
                    break e.state.seats.iter().map(|s| s.half_vps).collect();
                }
            }
        }
    };
    close(&e, &mut open, &mut prey_r);
    let s = at.index();
    [
        vps[s] as f64 / 2.0,
        won(&vps, s) as u8 as f64,
        ab(&e, at) as f64,
        turns,
        prey_r as f64,
    ]
}

/// Mean and two standard errors.
fn mean(xs: &[f64]) -> (f64, f64) {
    let n = xs.len().max(2) as f64;
    let m = xs.iter().sum::<f64>() / xs.len().max(1) as f64;
    let var = xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1.0);
    (m, 2.0 * (var / n).sqrt())
}

fn main() {
    let usage = "usage: search <net.zvp> <phi.json> <games> [--k 4] [--n 4] [--ab 0] [--fair]";
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (net, phi, games) = match &args[..] {
        [a, b, c, ..] => (a.clone(), b.clone(), c.parse::<u64>().expect(usage)),
        _ => panic!("{usage}"),
    };
    let flag = |name: &str, default: usize| {
        args.iter()
            .position(|a| a == name)
            .map_or(default, |i| args[i + 1].parse().expect(usage))
    };
    let (k, n, fair) = (
        flag("--k", 4),
        flag("--n", 4),
        args.iter().any(|a| a == "--fair"),
    );
    let ab_w: f32 = args
        .iter()
        .position(|a| a == "--ab")
        .map_or(0.0, |i| args[i + 1].parse().expect(usage));
    let net = Arc::new(Net::load(&std::fs::read(&net).expect("a policy")).expect("a policy"));
    let fit: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&phi).expect("a fit")).expect("a fit");
    let names: Vec<&str> = fit["names"]
        .as_array()
        .expect("names")
        .iter()
        .map(|v| v.as_str().expect("a name"))
        .collect();
    assert_eq!(names, position::NAMES, "{phi}: fitted on other features");
    let phi = Arc::new(Phi {
        weights: fit["weights"]
            .as_array()
            .expect("weights")
            .iter()
            .map(|v| v.as_f64().expect("a weight") as f32)
            .collect(),
        intercept: fit["intercept"].as_f64().expect("an intercept") as f32,
    });
    let threads = 8u64;
    let lines: Vec<(Line, Line)> = std::thread::scope(|scope| {
        let parts: Vec<_> = (0..threads)
            .map(|t| {
                let (net, phi) = (&net, &phi);
                scope.spawn(move || {
                    (t..games)
                        .step_by(threads as usize)
                        .map(|g| {
                            let seed = 60_000 + g;
                            (
                                play(net, phi, seed, None),
                                play(net, phi, seed, Some((k, n, fair, ab_w))),
                            )
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        parts
            .into_iter()
            .flat_map(|p| p.join().expect("a thread"))
            .collect()
    });
    println!(
        "{games} paired games, search k {k} n {n} ab {ab_w}{}",
        if fair { ", fair" } else { ", oracle" }
    );
    for (name, pick) in [("plain", 0usize), ("search", 1)] {
        let col = |c: usize| -> Vec<f64> {
            lines
                .iter()
                .map(|l| if pick == 0 { l.0[c] } else { l.1[c] })
                .collect()
        };
        let (vp, vp_e) = mean(&col(0));
        let (gw, _) = mean(&col(1));
        let per_turn = |c: usize| -> Vec<f64> {
            lines
                .iter()
                .map(|l| if pick == 0 { l.0 } else { l.1 })
                .filter(|l| l[3] > 0.0)
                .map(|l| l[c] / l[3])
                .collect()
        };
        let (ab, ab_e) = mean(&per_turn(2));
        let (pr, pr_e) = mean(&per_turn(4));
        println!(
            "  {name}: VP {vp:.2} ±{vp_e:.2}, game wins {:.0}%, A&B a round {ab:.2} ±{ab_e:.2}, its prey's R a turn {pr:+.2} ±{pr_e:.2}",
            gw * 100.0
        );
    }
    let diff = |c: usize| -> Vec<f64> { lines.iter().map(|l| l.1[c] - l.0[c]).collect() };
    let per_turn = |l: &Line, c: usize| l[c] / l[3];
    let diff_turn = |c: usize| -> Vec<f64> {
        lines
            .iter()
            .filter(|l| l.0[3] > 0.0 && l.1[3] > 0.0)
            .map(|l| per_turn(&l.1, c) - per_turn(&l.0, c))
            .collect()
    };
    let (d, d_e) = mean(&diff(0));
    let (a, a_e) = mean(&diff_turn(2));
    let (r, r_e) = mean(&diff_turn(4));
    println!(
        "  search − plain, paired: VP {d:+.2} ±{d_e:.2}, A&B a round {a:+.2} ±{a_e:.2}, its prey's R a turn {r:+.2} ±{r_e:.2}"
    );
}
