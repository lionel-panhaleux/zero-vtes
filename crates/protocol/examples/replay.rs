//! A replay exported from a game (the fork's "Export replay") read back:
//! every decision, whose it was, what was chosen and what happened in
//! between, every card shown. `cargo run -p zvtes-protocol --profile ci
//! --example replay -- <replay.json> [--options] [--table N]...`:
//! `--options` lists every decision's options, `--table N` prints the
//! table before decision N (pools, minions, their blood and cards).
//! `--upgrade OUT` writes the replay with its choices filled in from its
//! indices (`Replay::with_choices`), for a replay made before they were
//! logged: run on the engine it was played on.

use zvtes::crypt::CRYPT;
use zvtes::library::LIBRARY;
use zvtes_protocol::view::{CardView, MinionView};
use zvtes_protocol::{Out, Player, Replay, Session, TableView};

fn name(card: &CardView) -> String {
    let named = |id| {
        if card.crypt {
            CRYPT.iter().find(|c| c.id == id).map(|c| c.name)
        } else {
            LIBRARY.iter().find(|c| c.id == id).map(|c| c.name)
        }
    };
    card.id
        .and_then(named)
        .map_or_else(|| format!("#{}", card.copy), str::to_string)
}

fn minion(m: &MinionView) -> String {
    let mut s = format!("{} [{}", m.name, m.blood);
    if let Some(c) = m.capacity {
        s += &format!("/{c}");
    }
    s += "]";
    if m.locked {
        s += " locked";
    }
    if m.contested {
        s += " contested";
    }
    if let Some(t) = &m.title {
        s += &format!(" {t}");
    }
    for c in &m.cards {
        s += &format!(" +{}", name(&c.card));
        if c.counters > 0 {
            s += &format!("({})", c.counters);
        }
    }
    for k in &m.markers {
        s += &format!(" <{k}>");
    }
    s
}

fn table(view: &TableView, names: &[String]) {
    println!(
        "  ┌ table: turn {}, {} of {}",
        view.turn, view.phase, names[view.active as usize]
    );
    for s in &view.seats {
        let seat = &names[s.seat as usize];
        if s.ousted {
            println!("  │ {seat}: ousted");
            continue;
        }
        let edge = if s.edge { ", the Edge" } else { "" };
        println!("  │ {seat}: {} pool, {} VP{edge}", s.pool, s.vps);
        for m in &s.ready {
            println!("  │   {}", minion(m));
        }
        for m in &s.torpor {
            println!("  │   (torpor) {}", minion(m));
        }
        for c in &s.in_play {
            println!("  │   {}", name(&c.card));
        }
        let hand: Vec<_> = s.hand.iter().map(name).collect();
        println!("  │   hand: {}", hand.join(", "));
    }
    println!("  └");
}

fn main() {
    let mut path = None;
    let mut options = false;
    let mut tables = Vec::new();
    let mut upgrade = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--options" => options = true,
            "--table" => tables.push(
                args.next()
                    .and_then(|n| n.parse::<u64>().ok())
                    .expect("--table N"),
            ),
            "--upgrade" => upgrade = Some(args.next().expect("--upgrade OUT")),
            _ => path = Some(a),
        }
    }
    let path = path.expect("usage: replay <replay.json> [--options] [--table N]...");
    let text = std::fs::read_to_string(&path).expect("read the replay");
    let replay: Replay = serde_json::from_str(&text).expect("a replay");
    if let Some(out) = upgrade {
        let (replay, stop) = replay.with_choices().expect("the replay's spec");
        let json = serde_json::to_string(&replay).expect("a replay serializes");
        std::fs::write(&out, json).expect("write the replay");
        let n = replay.choices.len();
        println!("{out}: {n} choices of {}", replay.log.len());
        if let Some(stop) = stop {
            println!("  stopped at {stop}");
        }
        return;
    }
    let (mut spec, log) = (replay.spec.clone(), &replay.log);
    // Every seat answered from the log: the deal depends on the seed alone.
    let bots: Vec<_> = spec
        .seats
        .iter()
        .map(|s| match &s.player {
            Player::Bot { style } => format!(" ({style})"),
            Player::Human => String::new(),
        })
        .collect();
    for s in &mut spec.seats {
        s.player = Player::Human;
    }
    let names: Vec<String> = spec.seats.iter().map(|s| s.name.clone()).collect();
    println!("seed {}, {} decisions", spec.seed, log.len());
    for (i, s) in spec.seats.iter().enumerate() {
        println!("  seat {i}: {}{}", s.name, bots[i]);
    }
    let mut session = Session::new(spec).expect("the replay's spec");
    let mut reader = replay.reader();
    session.show_all();
    let tell = |events: &[String]| {
        for e in events {
            println!("      · {e}");
        }
    };
    loop {
        match session.advance() {
            Out::Decision {
                decision,
                seat,
                window,
                options: offered,
                events,
                view,
                ..
            } => {
                tell(&events);
                if tables.contains(&decision) {
                    table(&view, &names);
                }
                let req = session.request().expect("a decision is pending").clone();
                let chosen = reader.next(session.engine(), &req);
                let Some(chosen) = chosen else {
                    println!(
                        "#{decision} {} ({window}): the log ends here",
                        names[seat as usize]
                    );
                    for o in &offered {
                        println!("      {}. {}", o.index, o.text);
                    }
                    break;
                };
                let index = match chosen {
                    Ok(i) => i,
                    Err(e) => {
                        println!("#{decision}: {e}");
                        for o in &offered {
                            println!("      {}. {}", o.index, o.text);
                        }
                        break;
                    }
                };
                if options {
                    for o in &offered {
                        let mark = if o.index == index { '>' } else { ' ' };
                        println!("    {mark} {}. {}", o.index, o.text);
                    }
                }
                match session.answer(decision, index) {
                    Ok(Out::Decided { choice, events, .. }) => {
                        tell(&events);
                        println!(
                            "#{decision} [t{} {}] {} ({window}): {}",
                            view.turn, view.phase, names[seat as usize], choice.text
                        );
                    }
                    Ok(_) => unreachable!("an answer is decided"),
                    Err(e) => {
                        println!("#{decision}: the log's option {index} is refused: {e:?}");
                        break;
                    }
                }
            }
            Out::Decided { .. } => unreachable!("every seat is answered from the log"),
            Out::Over { vps, events, view } => {
                tell(&events);
                table(&view, &names);
                println!("game over: {vps:?}");
                break;
            }
        }
    }
}
