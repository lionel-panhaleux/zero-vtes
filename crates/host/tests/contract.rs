//! The contract held over random games (docs/online-plan.md A8): every
//! message validates against `schema.json` (records too); a game is
//! deterministic (the same spec and commands, the same messages, byte
//! for byte); no message names a card the seat was never shown.

use serde_json::{json, Value};
use zvtes_api::ids::OptionKind;
use zvtes_api::spec::Views;
use zvtes_api::{Command, GameSpec, In, Message, Out, Player, SeatSpec, Settings};
use zvtes_host::{Bots, Clock, Game};

struct NoBots;

impl Bots for NoBots {
    fn profiles(&self) -> Vec<String> {
        vec![]
    }
    fn make(
        &self,
        _: &str,
        _: u64,
    ) -> Option<(Box<dyn zvtes::agent::Agent>, zvtes_api::record::BotRecord)> {
        None
    }
}

struct Ticks(std::cell::Cell<u64>);

impl Clock for Ticks {
    fn now(&self) -> u64 {
        self.0.set(self.0.get() + 1);
        self.0.get()
    }
}

struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n.max(1)
    }
}

fn game(seed: u32) -> Game {
    let decks = zvtes_host::catalog();
    let styles = ["aggro_bleed", "wall", "random"];
    let human = seed as usize % 4;
    let settings = Settings {
        opportunities: !seed.is_multiple_of(3),
        views: if seed.is_multiple_of(2) {
            Views::Steps
        } else {
            Views::Decisions
        },
        text: seed % 4 < 2,
    };
    let spec = GameSpec {
        seed: Some(seed.into()),
        time_limit: Some(40),
        client: Some("tests".into()),
        seats: (0..4)
            .map(|i| SeatSpec {
                name: format!("Seat {i}"),
                deck: decks[(seed as usize + i * 7) % decks.len()].1.clone(),
                player: if i == human {
                    Player::Human { settings }
                } else {
                    Player::Bot {
                        profile: styles[(seed as usize + i) % 3].into(),
                    }
                },
            })
            .collect(),
    };
    Game::new(
        format!("g{seed}"),
        spec,
        &NoBots,
        Box::new(Ticks(Default::default())),
    )
    .unwrap()
}

fn cmd(command: Command) -> In {
    In { id: None, command }
}

/// A game played from its seed: every message sent, in order.
fn play(seed: u32) -> (Vec<Out>, Game) {
    play_game(game(seed), seed)
}

/// `g` played by random answers (its seed's) to the end or the budget.
fn play_game(mut g: Game, seed: u32) -> (Vec<Out>, Game) {
    let human = g.human();
    let mut rng = Rng(seed as u64 * 13 + 5);
    let mut all = Vec::new();
    let mut outs = g.handle(
        human,
        cmd(Command::Join {
            token: None,
            after: None,
        }),
    );
    for _ in 0..20_000 {
        all.extend(outs.iter().cloned());
        if g.is_over() {
            break;
        }
        let Some(d) = outs.iter().rev().find_map(|o| match &o.message {
            Message::Decision(d) => Some(d.clone()),
            _ => None,
        }) else {
            break;
        };
        let c = if d.undo && rng.below(8) == 0 {
            Command::Undo {
                decision: d.decision,
            }
        } else if let Some(v) = d.votes.filter(|v| v.favor > 0 && rng.below(3) == 0) {
            let _ = v;
            Command::VoteAll {
                decision: d.decision,
                favor: true,
            }
        } else {
            let plays: Vec<_> = d
                .options
                .iter()
                .filter(|o| o.key.kind == OptionKind::Play)
                .collect();
            let o = if !plays.is_empty() && rng.below(2) == 0 {
                plays[rng.below(plays.len())]
            } else {
                &d.options[rng.below(d.options.len())]
            };
            Command::Answer {
                decision: d.decision,
                option: o.index,
            }
        };
        outs = g.handle(human, cmd(c));
    }
    (all, g)
}

fn schema(def: &str) -> jsonschema::Validator {
    let file: Value = serde_json::from_str(include_str!("../../api/schema.json")).unwrap();
    let root = json!({
        "$schema": file["$schema"],
        "$defs": file["$defs"],
        "$ref": format!("#/$defs/{def}"),
    });
    jsonschema::validator_for(&root).expect("the schema compiles")
}

#[test]
fn every_message_and_record_validates_against_the_schema() {
    let seeds: u32 = std::env::var("HOST_SEEDS").map_or(12, |s| s.parse().unwrap());
    let (out, record) = (schema("Out"), schema("GameRecord"));
    let mut n = 0;
    for seed in 1..=seeds {
        let (all, g) = play(seed);
        for o in &all {
            let v = serde_json::to_value(o).unwrap();
            let bad = out
                .iter_errors(&v)
                .next()
                .map(|e| format!("{} at {}", e, e.instance_path));
            if let Some(e) = bad {
                panic!("seed {seed}: message {} invalid: {e}", o.seq);
            }
            n += 1;
        }
        let v = serde_json::to_value(g.record()).unwrap();
        let bad = record
            .iter_errors(&v)
            .next()
            .map(|e| format!("{} at {}", e, e.instance_path));
        if let Some(e) = bad {
            panic!("seed {seed}: record invalid: {e}");
        }
    }
    eprintln!("{seeds} games: {n} messages valid");
    // The same, typed against `protocol.ts` (`scripts/check_ts.sh`).
    if let Ok(dir) = std::env::var("CONTRACT_TS") {
        let (all, g) = play(4);
        let protocol = concat!(env!("CARGO_MANIFEST_DIR"), "/../api/protocol");
        let ts = format!(
            "import type {{ Out, GameRecord }} from '{protocol}'\n\
             export const out: Out[] = {}\n\
             export const record: GameRecord = {}\n",
            serde_json::to_string_pretty(&all).unwrap(),
            serde_json::to_string_pretty(g.record()).unwrap()
        );
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/fixtures.ts"), ts).unwrap();
    }
}

#[test]
fn a_game_is_deterministic() {
    for seed in [3, 8, 21] {
        let a = serde_json::to_string(&play(seed).0).unwrap();
        let b = serde_json::to_string(&play(seed).0).unwrap();
        assert!(a == b, "seed {seed}: the same game told otherwise");
    }
}

/// Another seat's decision told without what the table cannot know: the
/// card a library search took, one put under the library, drawn from a
/// store, taken from a hidden zone (views keep the copy rules:
/// `game.rs`'s tests).
#[test]
fn another_seats_hidden_choices_name_no_card() {
    use zvtes_api::event::EventKind;
    use zvtes_api::ids::{Window, Zone};
    let seeds: u32 = std::env::var("HOST_SEEDS").map_or(24, |s| s.parse().unwrap());
    let mut checked = 0;
    for seed in 1..=seeds {
        let (all, g) = play(seed);
        let human = g.human();
        for o in &all {
            let Message::Events { events, .. } = &o.message else {
                continue;
            };
            for e in events {
                let EventKind::Decided {
                    seat,
                    window,
                    choice,
                    ..
                } = &e.kind
                else {
                    continue;
                };
                if *seat == human {
                    continue;
                }
                let k = &choice.key;
                let hidden = match k.kind {
                    OptionKind::FindCard => *window == Window::LibrarySearch,
                    OptionKind::Bottom | OptionKind::DrawStored => true,
                    OptionKind::TakeFrom => k.zone.is_some_and(|z| z.zone != Zone::AshHeap),
                    _ => false,
                };
                if hidden {
                    checked += 1;
                    assert!(
                        k.card.as_ref().is_none_or(|c| c.id.is_none()),
                        "seed {seed}: another seat's hidden card named: {e:?}"
                    );
                }
            }
        }
    }
    eprintln!("{seeds} games: {checked} hidden choices told without their card");
}

/// Game of Malkav: another seat's secret number is told chosen, not which
/// (`Purpose::SecretNumber`); the human's own pick is its own. Every deck a
/// Malkavian precon with Game of Malkav added, the bots random.
#[test]
fn another_seats_secret_number_is_not_told() {
    use zvtes_api::event::EventKind;
    use zvtes_api::ids::Window;
    let precon = zvtes_host::catalog()
        .into_iter()
        .find(|(name, _)| name.contains("Malkavian"))
        .expect("a Malkavian precon")
        .1;
    let mut deck = precon.clone();
    *deck.entry("100800".into()).or_default() += 8;
    let (mut hidden, mut own) = (0, 0);
    for seed in 1..=12u32 {
        let spec = GameSpec {
            seed: Some(seed.into()),
            time_limit: Some(12),
            client: Some("tests".into()),
            seats: (0..3)
                .map(|i| SeatSpec {
                    name: format!("Seat {i}"),
                    deck: deck.clone(),
                    player: if i == 0 {
                        Player::Human {
                            settings: Settings {
                                opportunities: true,
                                views: Views::Decisions,
                                text: false,
                            },
                        }
                    } else {
                        Player::Bot {
                            profile: "random".into(),
                        }
                    },
                })
                .collect(),
        };
        let g = Game::new(
            format!("m{seed}"),
            spec,
            &NoBots,
            Box::new(Ticks(Default::default())),
        )
        .unwrap();
        let (all, g) = play_game(g, seed);
        for o in &all {
            let Message::Events { events, .. } = &o.message else {
                continue;
            };
            for e in events {
                let EventKind::Decided {
                    seat,
                    window: Window::ResolutionChoice,
                    choice,
                    ..
                } = &e.kind
                else {
                    continue;
                };
                if choice.key.kind != OptionKind::SetX {
                    continue;
                }
                if *seat == g.human() {
                    own += 1;
                    assert!(choice.key.amount.is_some(), "the human's own pick");
                } else {
                    hidden += 1;
                    // Nothing says the number: no amount, index 0, no effects.
                    assert!(
                        choice.key.amount.is_none()
                            && choice.index == 0
                            && choice.effects.is_empty(),
                        "seed {seed}: told {e:?}"
                    );
                }
            }
        }
    }
    eprintln!("{hidden} secret numbers of other seats, {own} of the human's");
    assert!(hidden > 0, "no Game of Malkav played");
}
