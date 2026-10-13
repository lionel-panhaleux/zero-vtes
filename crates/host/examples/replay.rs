//! A game record read back: every decision, whose it was, what was
//! chosen and what happened in between, every card shown. `cargo run -p
//! zvtes-host --profile ci --example replay -- <record.json> [--options]`:
//! `--options` lists every decision's options.

use zvtes_api::record::GameRecord;
use zvtes_host::replay::{Read, Replayer, Step};

fn main() {
    let mut path = None;
    let mut options = false;
    for a in std::env::args().skip(1) {
        match a.as_str() {
            "--options" => options = true,
            _ => path = Some(a),
        }
    }
    let path = path.expect("usage: replay <record.json> [--options]");
    let record: GameRecord =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("a record")).expect("a record");
    let mut r = Replayer::new(&record).expect("a record this engine reads");
    let names = r.names().to_vec();
    let window = |d: &Read| {
        serde_json::to_value(d.window)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default()
    };
    let before = |lines: &[String]| {
        for l in lines {
            println!("      · {l}");
        }
    };
    let list = |d: &Read| {
        for (i, o) in d.options.iter().enumerate() {
            println!("      {i}. {o}");
        }
    };
    loop {
        match r.step() {
            Step::Decision(d) => {
                before(&d.before);
                let who = &names[d.seat as usize];
                match d.chosen {
                    Some(i) => {
                        println!(
                            "#{} [t{} {}] {who}: {}",
                            d.decision,
                            d.turn,
                            window(&d),
                            d.options[i]
                        );
                        if options {
                            list(&d);
                        }
                        r.take();
                    }
                    None => {
                        println!(
                            "#{} {who} ({}): the record ends here",
                            d.decision,
                            window(&d)
                        );
                        list(&d);
                        break;
                    }
                }
            }
            Step::Stopped { why, last } => {
                before(&last.before);
                println!("#{} stops: {why}", last.decision);
                list(&last);
                break;
            }
            Step::Over {
                vp_halves,
                before: b,
            } => {
                before(&b);
                let vps: Vec<String> = vp_halves
                    .iter()
                    .zip(&names)
                    .map(|(v, n)| format!("{n} {}", *v as f32 / 2.0))
                    .collect();
                println!("over: {}", vps.join(", "));
                break;
            }
        }
    }
}
