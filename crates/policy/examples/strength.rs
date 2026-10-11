//! What a way of playing costs or gains a network: native self-play of
//! one exported network (`.zvp`), half the seats playing a variant, each
//! deal played twice with the halves swapped (decks and seats cancel
//! out). `cargo run -p zvtes-policy --profile ci --example strength --
//! <policy.zvp> <deals> [variant]...`: each variant prints the VPs a seat
//! of each half. A variant is `none` (the control) or `+`-joined parts:
//! `habits:<set>` (`zvtes::habits::Set::parse`, comma-separated),
//! `t:<temperature>` (logits divided by it), and the nudges
//! (`zvtes_policy::Nudges`): `rescue:<bonus>` (added to the logit of a
//! rescue of one's own vampire), `pass:<bonus>` (to passing one's discard
//! phase), `reaction:<bonus>` (taken off cycling a reaction),
//! `bleed:<bonus>` (taken off a bleed for nothing), `idle:<bonus>` (taken
//! off ending the influence phase with a crypt draw offered), `draw:<bonus>`
//! (to that crypt draw), `chores:1`
//! (the chores never declined) — e.g.
//! `t:0.5+rescue:1.5`. Every seat plays the default politics, as in
//! training.

use std::rc::Rc;
use zvtes::agent::Agent;
use zvtes::decision::DecisionRequest;
use zvtes::decks::Deck;
use zvtes::game::{Engine, Step};
use zvtes::habits::{self, Set};
use zvtes::rng::Rng;
use zvtes::{obs, politics};
use zvtes_policy::{chores, Net, Nudges};

/// A way of playing: the plain network is `Variant::default()`.
#[derive(Clone, Copy, Debug)]
struct Variant {
    habits: Option<Set>,
    temperature: f32,
    nudges: Nudges,
    /// The chores never declined (`zvtes_policy::chores`).
    chores: bool,
}

impl Default for Variant {
    fn default() -> Variant {
        Variant {
            habits: None,
            temperature: 1.0,
            nudges: Nudges::default(),
            chores: false,
        }
    }
}

impl Variant {
    fn parse(spec: &str) -> Result<Variant, String> {
        let mut v = Variant::default();
        if spec == "none" {
            return Ok(v);
        }
        for part in spec.split('+') {
            let (k, x) = part.split_once(':').ok_or(format!("{part:?}: key:value"))?;
            let num = || x.parse::<f32>().map_err(|e| format!("{part:?}: {e}"));
            match k {
                "habits" => v.habits = Some(Set::parse(x)?),
                "t" => v.temperature = num()?,
                "rescue" => v.nudges.rescue = num()?,
                "pass" => v.nudges.pass = num()?,
                "reaction" => v.nudges.reaction = num()?,
                "bleed" => v.nudges.bleed = num()?,
                "idle" => v.nudges.idle = num()?,
                "draw" => v.nudges.draw = num()?,
                "stuck" => v.nudges.stuck = num()?,
                "nightmare" => v.nudges.nightmare = num()?,
                "chores" => v.chores = num()? != 0.0,
                _ => {
                    return Err(format!(
                        "{k:?}: habits, t, rescue, pass, reaction, bleed, idle, draw"
                    ))
                }
            }
        }
        Ok(v)
    }
}

/// The network sampling its option, within the politics and the habits.
struct Player {
    net: Rc<Net>,
    rng: Rng,
    v: Variant,
}

impl Agent for Player {
    fn decide(&mut self, e: &Engine, req: &DecisionRequest) -> usize {
        if let Some(i) = politics::terms_choice(e, req) {
            return i;
        }
        if let Some(i) = self.v.habits.and_then(|set| habits::choice(e, req, set)) {
            return i;
        }
        let (mut logits, _) = self.net.forward(&obs::encode(e, req, self.net.version));
        for (l, o) in logits.iter_mut().zip(&req.options) {
            *l += self.v.nudges.bias(e, req, o);
            *l /= self.v.temperature;
        }
        let masks = [
            politics::mask(e, req),
            self.v.habits.and_then(|set| habits::mask(e, req, set)),
            self.v.chores.then(|| chores::mask(e, req)).flatten(),
        ];
        let ok = |i: usize| masks.iter().flatten().all(|m| m[i]);
        let max = (0..logits.len())
            .filter(|&i| ok(i))
            .map(|i| logits[i])
            .fold(f32::MIN, f32::max);
        let w: Vec<f64> = (0..logits.len())
            .map(|i| {
                if ok(i) {
                    ((logits[i] - max) as f64).exp()
                } else {
                    0.0
                }
            })
            .collect();
        let mut u =
            ((self.rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64) * w.iter().sum::<f64>();
        for (i, x) in w.iter().enumerate() {
            if u < *x {
                return i;
            }
            u -= x;
        }
        (0..w.len())
            .rev()
            .find(|&i| w[i] > 0.0)
            .expect("an allowed option")
    }
}

/// VPs (half points) of the plain half and the habit half over `deals`
/// deals, each played twice; this thread's share (`t` of `threads`).
fn play(path: &str, deals: u64, variant: Variant, t: u64, threads: u64) -> (u64, u64) {
    let net = Rc::new(Net::load(&std::fs::read(path).expect("the policy")).expect("a policy"));
    let decks: Vec<Deck> = Deck::all().take(14).collect();
    let (mut plain, mut other) = (0, 0);
    for game in (t..2 * deals).step_by(threads as usize) {
        let (deal, swap) = (game / 2, (game % 2) as usize);
        let seats: Vec<Deck> = (0..4)
            .map(|i| decks[((deal * 5 + i * 3) % 14) as usize])
            .collect();
        let mut e = Engine::with_decks(5000 + deal, &seats);
        e.time_limit = 75;
        let half = |s: usize| (s + swap) % 2 == 1;
        let mut players: Vec<Player> = (0..4)
            .map(|s| Player {
                net: net.clone(),
                rng: Rng::new(game * 10 + s as u64),
                v: if half(s) { variant } else { Variant::default() },
            })
            .collect();
        let vps = loop {
            match e.run() {
                Step::Decision(req) => {
                    let i = players[req.seat.index()].decide(&e, &req);
                    e.choose(i);
                    if e.decisions > 30_000 {
                        break e.state.seats.iter().map(|s| s.half_vps).collect();
                    }
                }
                Step::Over(vps) => break vps,
            }
        };
        for (s, &v) in vps.iter().enumerate() {
            if half(s) {
                other += v as u64;
            } else {
                plain += v as u64;
            }
        }
    }
    (plain, other)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: strength <policy.zvp> <deals> [variant]...");
    let deals: u64 = args.next().and_then(|n| n.parse().ok()).expect("deals");
    let mut variants: Vec<String> = args.collect();
    if variants.is_empty() {
        variants.push("none".into());
    }
    let threads = 8;
    for name in variants {
        let variant = Variant::parse(&name).expect("a variant");
        let (plain, other) = std::thread::scope(|s| {
            let parts: Vec<_> = (0..threads)
                .map(|t| {
                    s.spawn({
                        let path = &path;
                        move || play(path, deals, variant, t, threads)
                    })
                })
                .collect();
            parts
                .into_iter()
                .map(|h| h.join().expect("a thread"))
                .fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1))
        });
        // Two seats a half, two games a deal, half points.
        let per_seat = |v: u64| v as f64 / 2.0 / (4 * deals) as f64;
        println!(
            "{name}: plain {:.3} VP a seat, variant {:.3} ({} games)",
            per_seat(plain),
            per_seat(other),
            2 * deals
        );
    }
}
