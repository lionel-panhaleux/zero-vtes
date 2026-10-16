//! Games played through `Game::handle` as a client would: one human seat
//! answering at random by option index, scripted bots, every message
//! checked against the contract's lifecycle.

use zvtes_api::event::EventKind;
use zvtes_api::message::{EndReason, Error};
use zvtes_api::record::Entry;
use zvtes_api::{Command, GameSpec, In, Message, Out, Player, SeatSpec, Settings, SettingsPatch};
use zvtes_host::{Bots, Clock, Game};

struct NoBots;

impl Bots for NoBots {
    fn profiles(&self) -> Vec<String> {
        vec![]
    }
    fn make(
        &self,
        _: &str,
        _: u32,
    ) -> Option<(Box<dyn zvtes::agent::Agent>, zvtes_api::record::BotRecord)> {
        None
    }
}

/// A clock ticking one millisecond per reading.
struct Ticks(std::cell::Cell<u64>);

impl Clock for Ticks {
    fn now(&self) -> u64 {
        self.0.set(self.0.get() + 1);
        self.0.get()
    }
}

fn spec(seed: u32, human: usize, settings: Settings) -> GameSpec {
    let decks = zvtes_host::catalog();
    let styles = ["aggro_bleed", "wall", "random"];
    GameSpec {
        seed: Some(seed),
        time_limit: Some(40),
        client: Some("tests".into()),
        seats: (0..4)
            .map(|i| {
                let (name, list) = &decks[(seed as usize + i * 3) % decks.len()];
                SeatSpec {
                    name: name.to_string(),
                    deck: list.clone(),
                    player: if i == human {
                        Player::Human { settings }
                    } else {
                        Player::Bot {
                            profile: styles[(seed as usize + i) % 3].into(),
                        }
                    },
                }
            })
            .collect(),
    }
}

fn game(seed: u32, settings: Settings) -> Game {
    let human = seed as usize % 4;
    Game::new(
        format!("g{seed}"),
        spec(seed, human, settings),
        &NoBots,
        Box::new(Ticks(Default::default())),
    )
    .unwrap()
}

fn cmd(c: Command) -> In {
    In {
        id: None,
        command: c,
    }
}

fn join() -> In {
    cmd(Command::Join {
        token: None,
        after: None,
    })
}

struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n
    }
}

/// Every message survives JSON, and `seq` counts up from `last`.
fn check(outs: &[Out], last: &mut u64) {
    for o in outs {
        assert_eq!(o.seq, *last + 1, "seq");
        *last = o.seq;
        let j = serde_json::to_string(o).unwrap();
        let back: Out = serde_json::from_str(&j).unwrap();
        assert_eq!(&back, o, "round trip");
    }
}

/// The decisions the messages tell taken.
fn told(outs: &[Out]) -> usize {
    outs.iter()
        .filter_map(|o| match &o.message {
            Message::Events { events, .. } => Some(events),
            _ => None,
        })
        .flatten()
        .filter(|e| {
            matches!(
                e.kind,
                EventKind::Decided { .. } | EventKind::AutoPassed { .. }
            )
        })
        .count()
}

fn pending(outs: &[Out]) -> Option<&zvtes_api::Decision> {
    outs.iter().rev().find_map(|o| match &o.message {
        Message::Decision(d) => Some(d),
        _ => None,
    })
}

#[test]
fn games_play_to_their_end_through_handle() {
    let seeds: u32 = std::env::var("HOST_SEEDS").map_or(12, |s| s.parse().unwrap());
    for seed in 1..=seeds {
        let mut g = game(seed, Settings::default());
        let human = g.human();
        let mut rng = Rng(seed as u64);
        let mut last = 0;
        let mut outs = g.handle(human, join());
        check(&outs, &mut last);
        assert!(matches!(outs[0].message, Message::Joined(_)));
        let mut decided = told(&outs);
        let mut steps = 0;
        while !g.is_over() {
            steps += 1;
            assert!(steps < 20_000, "seed {seed}: no end");
            let d = pending(&outs).expect("a decision until the end").clone();
            assert_eq!(d.seat, human);
            let pick = &d.options[rng.below(d.options.len())];
            outs = g.handle(
                human,
                cmd(Command::Answer {
                    decision: d.decision,
                    option: pick.index,
                }),
            );
            check(&outs, &mut last);
            if !g.is_over() {
                // The same answer again is stale now.
                let again = g.handle(
                    human,
                    cmd(Command::Answer {
                        decision: d.decision,
                        option: pick.index,
                    }),
                );
                check(&again, &mut last);
                assert!(matches!(
                    again[0].message,
                    Message::Error(Error::Stale { .. }) | Message::Error(Error::NotYourTurn)
                ));
            }
            decided += told(&outs);
        }
        let over = outs
            .iter()
            .find_map(|o| match &o.message {
                Message::Over(over) => Some(over.clone()),
                _ => None,
            })
            .expect("over");
        let r = g.record();
        assert_eq!(
            r.result.as_ref().unwrap().reason,
            over.reason,
            "seed {seed}"
        );
        assert!(r.spec.seats[human as usize].name == "human");
        // The turn it ended on, recorded (the report reads no replay).
        let turns = r.result.as_ref().unwrap().turns;
        assert!(turns.is_some_and(|t| t >= 1), "seed {seed}: {turns:?}");
        let entries = r
            .entries
            .iter()
            .filter(|e| matches!(e, Entry::Decision { .. }))
            .count();
        // Every decision is told once (the first, before the human
        // joined, too).
        assert_eq!(entries, decided, "seed {seed}");
        assert_ne!(over.reason, EndReason::EngineError, "seed {seed}");
    }
}

#[test]
fn a_rejoin_resends_what_followed() {
    let mut g = game(3, Settings::default());
    let human = g.human();
    let first = g.handle(human, join());
    let d = pending(&first).unwrap().clone();
    let more = g.handle(
        human,
        cmd(Command::Answer {
            decision: d.decision,
            option: d.options[0].index,
        }),
    );
    let after = first.last().unwrap().seq;
    let resent = g.handle(
        human,
        cmd(Command::Join {
            token: None,
            after: Some(after),
        }),
    );
    assert_eq!(resent, more);
    // Nothing missed: nothing resent.
    let none = g.handle(
        human,
        cmd(Command::Join {
            token: None,
            after: Some(more.last().unwrap().seq),
        }),
    );
    assert!(none.is_empty());
}

#[test]
fn an_undo_sent_twice_undoes_once() {
    for seed in 1..=20 {
        let mut g = game(seed, Settings::default());
        let human = g.human();
        let mut outs = g.handle(human, join());
        let mut rng = Rng(seed as u64);
        for _ in 0..200 {
            let Some(d) = pending(&outs).cloned() else {
                break;
            };
            if d.undo {
                let undo = cmd(Command::Undo {
                    decision: d.decision,
                });
                let first = g.handle(human, undo.clone());
                assert!(
                    matches!(&first[0].message, Message::Events { events, .. }
                        if matches!(events[0].kind, zvtes_api::EventKind::Undone { .. })),
                    "seed {seed}: {first:?}"
                );
                let second = g.handle(human, undo);
                assert!(matches!(
                    second[0].message,
                    Message::Error(Error::Stale { .. })
                ));
                assert!(g
                    .record()
                    .entries
                    .iter()
                    .any(|e| matches!(e, Entry::Undo { .. })));
                return;
            }
            let pick = d.options[rng.below(d.options.len())].index;
            outs = g.handle(
                human,
                cmd(Command::Answer {
                    decision: d.decision,
                    option: pick,
                }),
            );
        }
    }
    panic!("no undo offered in 20 games");
}

#[test]
fn settings_resend_the_pending_decision() {
    let mut g = game(5, Settings::default());
    let human = g.human();
    let outs = g.handle(human, join());
    assert!(pending(&outs).unwrap().options[0].text.is_none());
    let outs = g.handle(
        human,
        cmd(Command::Settings(SettingsPatch {
            text: Some(true),
            ..Default::default()
        })),
    );
    let d = pending(&outs).expect("the decision again");
    assert!(d.options.iter().all(|o| o.text.is_some()));
}

#[test]
fn other_seats_and_strangers_are_refused() {
    let mut g = game(7, Settings::default());
    let human = g.human();
    let other = (human + 1) % 4;
    let outs = g.handle(other, join());
    assert!(matches!(
        outs[0].message,
        Message::Error(Error::NotYourTurn)
    ));
    let outs = g.handle(
        human,
        In {
            id: Some(9),
            command: Command::Undo { decision: 0 },
        },
    );
    assert!(matches!(outs[0].message, Message::Error(Error::NotJoined)));
    assert_eq!(outs[0].reply_to, Some(9));
    let outs = g.handle(human, cmd(Command::Leave));
    let _ = outs;
}

/// Feedback (docs/protocol.md §4): kept in the record at the pending
/// decision, refused before the seat joins or when empty, still taken once
/// the game is over (at the last decision).
#[test]
fn feedback_is_recorded_at_its_decision() {
    let mut g = game(7, Settings::default());
    let human = g.human();
    let feedback = |text: &str| cmd(Command::Feedback { text: text.into() });
    let outs = g.handle(human, feedback("before"));
    assert!(matches!(outs[0].message, Message::Error(Error::NotJoined)));
    let outs = g.handle(human, join());
    let d = pending(&outs).expect("a decision").decision;
    let outs = g.handle(
        human,
        In {
            id: Some(9),
            command: Command::Feedback {
                text: "  the Edge vanished  ".into(),
            },
        },
    );
    assert_eq!(outs[0].message, Message::Noted { decision: d });
    assert_eq!(outs[0].reply_to, Some(9), "noted answers its command");
    assert!(matches!(
        g.record().entries.last(),
        Some(Entry::Feedback { decision, text, .. }) if *decision == d && text == "the Edge vanished"
    ));
    let outs = g.handle(human, feedback(" "));
    assert!(matches!(
        outs[0].message,
        Message::Error(Error::Malformed { .. })
    ));
    // Still the pending decision: feedback answers nothing.
    let outs = g.handle(human, join());
    assert_eq!(pending(&outs).unwrap().decision, d);
    g.handle(human, cmd(Command::Leave));
    let outs = g.handle(human, feedback("it ended"));
    assert!(matches!(outs[0].message, Message::Noted { .. }));
}
