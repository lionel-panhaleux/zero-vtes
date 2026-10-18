//! The positions where a human beat the bot (the training board's
//! evaluation set): games a human won, played out (last standing or the
//! time limit), read whole by this engine, never trained on — a run's
//! `--bc-held-games`, in `data/records` and harvested. Two scores for a
//! network:
//! - agreement: at each of the winner's decisions with an alternative, the
//!   network's favourite (its plain logits, no nudge or mask, as imitation
//!   reads them) against the human's choice, and the log-probability it
//!   gives that choice;
//! - the outcome replayed: from the position at a quarter, half and three
//!   quarters of the decisions the engine reads, the network takes the
//!   human's seat and the bots the others, every seat as the server plays
//!   them (`zvtes_policy::profiles::NETWORK`), libraries and crypts shuffled
//!   anew each game (`zvtes_host::start`); the network's game wins and
//!   VPs from where the human won.
//!
//! `cargo run -p zvtes-policy --profile ci --example humanset -- <network.zvp>
//! <bots.zvp> <games a position> <record.json | records dir>...`

use std::sync::Arc;
use zvtes::agent::Agent;
use zvtes::game::{Engine, Step};
use zvtes::obs;
use zvtes_api::message::EndReason;
use zvtes_api::record::GameRecord;
use zvtes_host::replay::Replay;
use zvtes_host::start::Start;
use zvtes_policy::profiles::NETWORK;
use zvtes_policy::{Net, NetAgent};

const POINTS: [(f64, &str); 3] = [
    (0.25, "a quarter in"),
    (0.5, "halfway"),
    (0.75, "three quarters in"),
];

/// A human's win to start from.
struct Won {
    name: String,
    seat: usize,
    record: GameRecord,
    start: Start,
}

fn won(path: &str) -> Result<Won, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let record: GameRecord = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let result = record.result.as_ref().ok_or("unfinished")?;
    if !matches!(
        result.reason,
        EndReason::LastStanding | EndReason::TimeLimit
    ) {
        return Err(format!("not played out ({:?})", result.reason));
    }
    let replay = Replay::new(&record)?;
    let seat = result.winner.ok_or("no winner")? as usize;
    if !replay.humans.contains(&(seat as u8)) {
        return Err("a bot won".into());
    }
    // A game this engine stops reading (a rules change since) would be
    // scored on its prefix: out whole, as imitation leaves it.
    let mut e: Engine = replay.deal();
    let mut reader = replay.reader();
    while let Step::Decision(req) = e.run() {
        let i = match replay.elided(&e, &req) {
            Some(i) => i,
            None => match reader.next(&e, &req) {
                Some(Ok(i)) => i,
                Some(Err(why)) => return Err(format!("read partway: {why}")),
                None => break,
            },
        };
        e.choose(i);
    }
    let name = std::path::Path::new(path)
        .file_stem()
        .map_or(path.into(), |s| s.to_string_lossy().into_owned());
    Ok(Won {
        name,
        seat,
        start: Start::new(&text)?,
        record,
    })
}

/// The winner's decisions read back: (with an alternative, the network's
/// favourite was the human's, the log-probability of the human's).
fn agreement(net: &Net, w: &Won) -> (u64, u64, f64) {
    let replay = Replay::new(&w.record).expect("read once");
    let mut e: Engine = replay.deal();
    let mut reader = replay.reader();
    let (mut n, mut same, mut logp) = (0, 0, 0.0);
    while let Step::Decision(req) = e.run() {
        if let Some(i) = replay.elided(&e, &req) {
            e.choose(i);
            continue;
        }
        let Some(Ok(i)) = reader.next(&e, &req) else {
            break;
        };
        if req.seat.index() == w.seat && req.options.len() > 1 {
            let (logits, _) = net.forward(&obs::encode(&e, &req, net.version));
            let max = logits.iter().copied().fold(f32::MIN, f32::max);
            let sum: f64 = logits.iter().map(|&l| ((l - max) as f64).exp()).sum();
            let top = (0..logits.len()).max_by(|&a, &b| logits[a].total_cmp(&logits[b]));
            n += 1;
            same += (top == Some(i)) as u64;
            logp += (logits[i] - max) as f64 - sum.ln();
        }
        e.choose(i);
    }
    (n, same, logp)
}

/// The network in the winner's seat from decision `at`: its half VPs and
/// whether it won the game.
fn replayed(nets: [&Arc<Net>; 2], w: &Won, at: usize, seed: u64) -> (u32, bool) {
    let mut e = w.start.position(at, seed, 75);
    let mut agents: Vec<NetAgent> = (0..w.start.seats())
        .map(|s| {
            let net = if s == w.seat { nets[0] } else { nets[1] };
            NETWORK.agent(net.clone(), seed * 10 + s as u64)
        })
        .collect();
    let cap = e.decisions + 30_000;
    let vps: Vec<u32> = loop {
        match e.run() {
            Step::Decision(req) => {
                let i = agents[req.seat.index()].decide(&e, &req);
                e.choose(i);
                if e.decisions > cap {
                    break e.state.seats.iter().map(|s| s.half_vps).collect();
                }
            }
            Step::Over(vps) => break vps.to_vec(),
        }
    };
    let best = *vps.iter().max().expect("seats");
    let win = vps[w.seat] == best && best >= 4 && vps.iter().filter(|&&v| v == best).count() == 1;
    (vps[w.seat], win)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let usage = "usage: humanset <network.zvp> <bots.zvp> <games a position> <record.json | records dir>...";
    let load = |p: String| Arc::new(Net::load(&std::fs::read(p).expect(usage)).expect("a policy"));
    let (net, bots) = (
        load(args.next().expect(usage)),
        load(args.next().expect(usage)),
    );
    let games: u64 = args.next().and_then(|n| n.parse().ok()).expect(usage);
    let mut paths = Vec::new();
    for a in args {
        match std::fs::read_dir(&a) {
            Ok(dir) => {
                let mut files: Vec<String> = dir
                    .filter_map(|f| f.ok().map(|f| f.path()))
                    .filter(|p| p.extension().is_some_and(|x| x == "json"))
                    .map(|p| p.display().to_string())
                    .collect();
                files.sort();
                paths.extend(files);
            }
            Err(_) => paths.push(a),
        }
    }
    let set: Vec<Won> = paths
        .iter()
        .filter_map(|p| match won(p) {
            Ok(w) => Some(w),
            Err(why) => {
                eprintln!("{p}: not in the set ({why})");
                None
            }
        })
        .collect();
    assert!(!set.is_empty(), "no human win to start from");
    // (game, point, game number) over the threads.
    let tasks: Vec<(usize, usize, u64)> = (0..set.len())
        .flat_map(|w| (0..POINTS.len()).flat_map(move |p| (0..games).map(move |g| (w, p, g))))
        .collect();
    let threads = 8;
    let results: Vec<(usize, usize, u32, bool)> = std::thread::scope(|scope| {
        let parts: Vec<_> = (0..threads)
            .map(|t| {
                let (tasks, set, net, bots) = (&tasks, &set, &net, &bots);
                scope.spawn(move || {
                    tasks
                        .iter()
                        .skip(t)
                        .step_by(threads)
                        .map(|&(w, p, g)| {
                            let at = (set[w].start.readable as f64 * POINTS[p].0) as usize;
                            let seed = 60_000 + 1_000 * w as u64 + 100 * p as u64 + g;
                            let (vp, win) = replayed([net, bots], &set[w], at, seed);
                            (w, p, vp, win)
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
    let (mut n, mut same, mut logp) = (0, 0, 0.0);
    println!("game | seat | agreement (decisions) | from each position: wins, VP a game");
    for (k, w) in set.iter().enumerate() {
        let (dn, ds, dl) = agreement(&net, w);
        (n, same, logp) = (n + dn, same + ds, logp + dl);
        let points: Vec<String> = (0..POINTS.len())
            .map(|p| {
                let r: Vec<_> = results.iter().filter(|r| r.0 == k && r.1 == p).collect();
                let wins = r.iter().filter(|r| r.3).count();
                let vp = r.iter().map(|r| r.2 as f64).sum::<f64>() / 2.0 / r.len().max(1) as f64;
                format!("{wins}/{}, {vp:.2}", r.len())
            })
            .collect();
        println!(
            "{} | {} | {:.0}% ({dn}) | {}",
            w.name,
            w.seat,
            100.0 * ds as f64 / dn.max(1) as f64,
            points.join(" | ")
        );
    }
    println!(
        "agreement: {:.1}% of {n} decisions, mean log-probability of the human's choice {:.3}",
        100.0 * same as f64 / n.max(1) as f64,
        logp / n.max(1) as f64
    );
    for (p, (_, label)) in POINTS.iter().enumerate() {
        let r: Vec<_> = results.iter().filter(|r| r.1 == p).collect();
        let wins = r.iter().filter(|r| r.3).count();
        let vp = r.iter().map(|r| r.2 as f64).sum::<f64>() / 2.0 / r.len().max(1) as f64;
        println!(
            "from {label}: won {wins} of {} ({:.0}%), {vp:.2} VP a game",
            r.len(),
            100.0 * wins as f64 / r.len().max(1) as f64
        );
    }
}
