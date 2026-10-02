//! A game's narration as the human seat would read it: seat 1 answered at
//! random, three scripted bots. `cargo run -p zvtes-protocol --example
//! narrate -- [seed] [messages]`.

use zvtes::decks::Deck;
use zvtes::rng::Rng;
use zvtes_protocol::deck::list_of;
use zvtes_protocol::{GameSpec, Out, Player, SeatSpec, Session};

fn main() {
    let mut args = std::env::args()
        .skip(1)
        .map(|a| a.parse::<u64>().expect("number"));
    let seed = args.next().unwrap_or(1);
    let limit = args.next().unwrap_or(120);
    let styles = ["aggro_bleed", "wall", "aggro_bleed"];
    let seats = (0..4)
        .map(|i| SeatSpec {
            name: if i == 0 {
                "You".into()
            } else {
                format!("Bot {i}")
            },
            deck: list_of(&Deck::precon((seed as usize + i * 3) % 14)),
            player: if i == 0 {
                Player::Human
            } else {
                Player::Bot {
                    style: styles[i - 1].into(),
                }
            },
        })
        .collect();
    let mut s = Session::new(GameSpec {
        seed,
        time_limit: None,
        seats,
    })
    .expect("spec");
    let mut rng = Rng::new(seed);
    for _ in 0..limit {
        let out = s.advance();
        let (Out::Decision { events, .. } | Out::Decided { events, .. } | Out::Over { events, .. }) =
            &out;
        for e in events {
            println!("      · {e}");
        }
        match out {
            Out::Decision {
                decision,
                seat,
                window,
                options,
                view,
                ..
            } => {
                println!(
                    "[turn {} {}] {window}: you choose among",
                    view.turn, view.phase
                );
                for o in &options {
                    println!("      {}. {}", o.index, o.text);
                }
                let i = rng.below(options.len());
                if let Ok(Out::Decided { choice, events, .. }) = s.answer(decision, i) {
                    for e in events {
                        println!("      · {e}");
                    }
                    println!("   seat {seat} → {}", choice.text);
                }
            }
            Out::Decided {
                seat,
                window,
                choice,
                view,
                ..
            } => {
                println!(
                    "[turn {} {}] {window}: seat {seat} → {}",
                    view.turn, view.phase, choice.text
                );
            }
            Out::Over { vps, .. } => {
                println!("game over: {vps:?}");
                break;
            }
        }
    }
}
