//! The positions where a human beat the bot (the training board's
//! evaluation set): games a human won, played out (last standing or the
//! time limit), read whole by this engine, a run's `--bc-held-games`, out
//! of its imitation and its starts, in `data/records` and harvested; a
//! resumed network keeps what its ancestors trained on (every one to date,
//! the served one too, descends from tables begun from the first five). Two
//! scores for a network:
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
//! `--apart <part,…>`: the wins whose file name holds one of the parts are
//! scored apart from the others (the training skill's held-out five, starts
//! of every ancestor's tables). `--map`: the agreement by window and kind of
//! the human's choice, with the share of the human's choices the network
//! gives under 1% ("collapsed": the missed-opportunity map, DESIGN §11.2).
//! With 0 games a position, no game is replayed.
//!
//! `cargo run -p zvtes-policy --profile ci --example humanset -- <network.zvp>
//! <bots.zvp> <games a position> [--apart <part,…>] [--map]
//! <record.json | records dir>...`

use std::collections::BTreeMap;
use std::sync::Arc;
use zvtes::agent::Agent;
use zvtes::game::{Engine, Step};
use zvtes_api::message::EndReason;
use zvtes_api::record::GameRecord;
use zvtes_env::obs;
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

/// The winner's decisions with an alternative: how many, how many the
/// network's favourite was the human's choice, the sum of the
/// log-probabilities it gives that choice, how many under 1%.
#[derive(Clone, Copy, Default)]
struct Agreement {
    n: u64,
    same: u64,
    logp: f64,
    collapsed: u64,
}

impl Agreement {
    fn add(&mut self, o: &Agreement) {
        self.n += o.n;
        self.same += o.same;
        self.logp += o.logp;
        self.collapsed += o.collapsed;
    }

    fn line(&self) -> String {
        let n = self.n as f64;
        format!(
            "{} decisions, favourite {:.1}%, mean log-probability {:.3}, under 1% {:.1}%",
            self.n,
            100.0 * self.same as f64 / n,
            self.logp / n,
            100.0 * self.collapsed as f64 / n
        )
    }
}

/// The winner's decisions read back, by (window, kind of the human's
/// choice).
fn agreement(net: &Net, w: &Won) -> BTreeMap<(String, String), Agreement> {
    let replay = Replay::new(&w.record).expect("read once");
    let mut e: Engine = replay.deal();
    let mut reader = replay.reader();
    let mut by = BTreeMap::<(String, String), Agreement>::new();
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
            let logp = (logits[i] - max) as f64 - sum.ln();
            let a = by
                .entry((
                    format!("{:?}", req.window),
                    zvtes_policy::choice_kind(&req.options[i]),
                ))
                .or_default();
            a.n += 1;
            a.same += (top == Some(i)) as u64;
            a.logp += logp;
            a.collapsed += (logp < 0.01f64.ln()) as u64;
        }
        e.choose(i);
    }
    by
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
    let usage = "usage: humanset <network.zvp> <bots.zvp> <games a position> [--apart <part,…>] [--map] <record.json | records dir>...";
    let load = |p: String| Arc::new(Net::load(&std::fs::read(p).expect(usage)).expect("a policy"));
    let (net, bots) = (
        load(args.next().expect(usage)),
        load(args.next().expect(usage)),
    );
    let games: u64 = args.next().and_then(|n| n.parse().ok()).expect(usage);
    let (mut apart, mut map) = (Vec::<String>::new(), false);
    let mut paths = Vec::new();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--apart" => {
                let parts = args.next().expect(usage);
                apart.extend(parts.split(',').map(str::to_string));
                continue;
            }
            "--map" => {
                map = true;
                continue;
            }
            _ => {}
        }
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
    let is_apart = |w: &Won| apart.iter().any(|p| w.name.contains(p.as_str()));
    // (the wins apart, the others): the agreement by (window, kind).
    let mut groups = [BTreeMap::new(), BTreeMap::new()];
    println!(
        "game | seat | agreement (decisions){}",
        if games > 0 {
            " | from each position: wins, VP a game"
        } else {
            ""
        }
    );
    for (k, w) in set.iter().enumerate() {
        let by = agreement(&net, w);
        let mut game = Agreement::default();
        for (key, a) in &by {
            game.add(a);
            let g: &mut BTreeMap<_, Agreement> = &mut groups[!is_apart(w) as usize];
            g.entry(key.clone()).or_default().add(a);
        }
        let points: Vec<String> = (0..POINTS.len() * (games > 0) as usize)
            .map(|p| {
                let r: Vec<_> = results.iter().filter(|r| r.0 == k && r.1 == p).collect();
                let wins = r.iter().filter(|r| r.3).count();
                let vp = r.iter().map(|r| r.2 as f64).sum::<f64>() / 2.0 / r.len().max(1) as f64;
                format!("{wins}/{}, {vp:.2}", r.len())
            })
            .collect();
        println!(
            "{}{} | {} | {:.0}% ({}){}",
            w.name,
            if is_apart(w) { " (apart)" } else { "" },
            w.seat,
            100.0 * game.same as f64 / game.n.max(1) as f64,
            game.n,
            points.iter().map(|p| format!(" | {p}")).collect::<String>()
        );
    }
    let labels = ["the wins apart", "the other wins"];
    for (g, label) in groups.iter().zip(labels) {
        if g.is_empty() {
            continue;
        }
        let mut all = Agreement::default();
        g.values().for_each(|a| all.add(a));
        println!("agreement, {label}: {}", all.line());
        if map {
            let mut rows: Vec<_> = g.iter().collect();
            rows.sort_by_key(|a| std::cmp::Reverse(a.1.n));
            for ((window, kind), a) in rows {
                println!("  {window} | {kind} | {}", a.line());
            }
        }
    }
    for (p, (_, label)) in POINTS
        .iter()
        .enumerate()
        .take(POINTS.len() * (games > 0) as usize)
    {
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
