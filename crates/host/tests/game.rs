//! Games played through `Game::handle` as a client would: one human seat
//! answering at random by option index, scripted bots, every message
//! checked against the contract's lifecycle.

use zvtes_api::event::EventKind;
use zvtes_api::ids::{ActionKind, DueKind, OptionKind};
use zvtes_api::message::{EndReason, Error};
use zvtes_api::option::{Effect, EffectKind, User};
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

/// Plays `seeds` games with the text off, the human answering at random,
/// and hands each of its decisions to `f` (its seed and the decision).
fn each_decision(seeds: u32, mut f: impl FnMut(u32, &zvtes_api::Decision)) {
    for seed in 1..=seeds {
        let settings = Settings {
            text: false,
            ..Settings::default()
        };
        let mut g = game(seed, settings);
        let human = g.human();
        let mut rng = Rng(seed as u64 * 7 + 1);
        let mut outs = g.handle(human, join());
        while !g.is_over() {
            let Some(d) = pending(&outs).cloned() else {
                break;
            };
            f(seed, &d);
            let o = &d.options[rng.below(d.options.len())];
            outs = g.handle(
                human,
                cmd(Command::Answer {
                    decision: d.decision,
                    option: o.index,
                }),
            );
        }
    }
}

/// A decision's `use_ability` options.
fn abilities(d: &zvtes_api::Decision) -> impl Iterator<Item = &zvtes_api::option::OptionView> {
    d.options
        .iter()
        .filter(|o| o.key.kind == OptionKind::UseAbility)
}

/// An ability's source: its card's copy, or the vampire it is on.
type Source = (Option<u32>, Option<u32>);

/// An ability's `apart` is alike on every option of it, whatever its actor
/// or target, so a client asking "which ability?" names each by it; two
/// abilities of one source offered together are told apart by it (Wider
/// View's transfer and its burn, Dreams of the Sphinx's hand size and its
/// blood) — never by the English text (docs/protocol.md §7).
#[test]
fn an_ability_is_told_apart_from_its_siblings_by_data() {
    let seeds: u32 = std::env::var("HOST_SEEDS").map_or(12, |s| s.parse().unwrap());
    let mut apart_offered = 0;
    each_decision(seeds, |seed, d| {
        // Each (source, ability) offered: its `apart`.
        let mut seen: Vec<(Source, u8, &[Effect])> = Vec::new();
        for o in abilities(d) {
            let source = (o.key.card.as_ref().and_then(|c| c.copy), o.key.minion);
            let ability = o.key.ability.unwrap_or(0);
            assert!(
                o.apart.is_empty() || o.abilities.unwrap_or(1) > 1,
                "seed {seed}: a lone ability set apart: {o:?}"
            );
            match seen.iter().find(|(s, a, _)| *s == source && *a == ability) {
                Some((_, _, apart)) => {
                    assert_eq!(*apart, &o.apart[..], "seed {seed}: one ability, two aparts")
                }
                None => seen.push((source, ability, &o.apart)),
            }
        }
        for (i, (s, _, x)) in seen.iter().enumerate() {
            for (t, _, y) in &seen[i + 1..] {
                if s == t && !x.is_empty() && !y.is_empty() {
                    assert_ne!(x, y, "seed {seed}: two abilities alike");
                    apart_offered += 1;
                }
            }
        }
    });
    assert!(
        apart_offered > 0,
        "no two abilities of a source offered together"
    );
}

/// Every ability offered says who uses it (docs/protocol.md §7): the
/// minion declaring the action it grants (its `actor`), a minion with no
/// `actor` (the bearer, the vampire whose text it is, War Ghoul burning a
/// location), or the deciding seat's Methuselah (a master's, a location's)
/// — never an absent `actor` read as the Methuselah.
#[test]
fn an_abilitys_user_is_said() {
    let seeds: u32 = std::env::var("HOST_SEEDS").map_or(12, |s| s.parse().unwrap());
    let (mut minion, mut methuselah) = (0, 0);
    each_decision(seeds, |seed, d| {
        for o in abilities(d) {
            match (o.user, o.key.actor) {
                (Some(User::Minion { copy }), Some(a)) => {
                    assert_eq!(copy, a, "seed {seed}: the actor uses it: {o:?}")
                }
                (Some(User::Minion { copy }), None) => {
                    if let Some(k) = o.key.minion {
                        assert_eq!(copy, k, "seed {seed}: the minion it is on: {o:?}");
                    }
                    minion += 1;
                }
                (Some(User::Methuselah { seat }), None) => {
                    assert_eq!(seat, d.seat, "seed {seed}: the deciding seat's: {o:?}");
                    methuselah += 1;
                }
                _ => panic!("seed {seed}: who uses it, unsaid or not the actor: {o:?}"),
            }
        }
    });
    assert!(
        minion > 0 && methuselah > 0,
        "{minion} by a minion, {methuselah} by a Methuselah"
    );
}

/// A minion phase offering no `pass` says why (docs/protocol.md §5): its
/// `due`, each naming a minion or card its options act on — a vampire
/// that must hunt first, with a hunt on offer (rules-qa 17.2) — and one
/// offering `pass` none; never inferred from the missing pass. The first
/// forced hunt the human meets is seed 34 or so (a lone hunt elides).
#[test]
fn a_minion_phase_without_a_pass_says_why() {
    let seeds: u32 = std::env::var("HOST_SEEDS").map_or(70, |s| s.parse().unwrap());
    let mut hunts = 0;
    each_decision(seeds, |seed, d| {
        if d.window != zvtes_api::ids::Window::MinionPhase {
            assert!(d.due.is_empty(), "seed {seed}: due outside a minion phase");
            return;
        }
        let pass = d.options.iter().any(|o| o.key.kind == OptionKind::Pass);
        assert_eq!(
            pass,
            d.due.is_empty(),
            "seed {seed}: {:?} {:?}",
            d.due,
            d.options
        );
        for due in &d.due {
            let acts = |o: &&zvtes_api::option::OptionView| {
                [
                    o.key.actor,
                    o.key.minion,
                    o.key.card.as_ref().and_then(|c| c.copy),
                ]
                .contains(&Some(due.copy))
            };
            assert!(
                d.options.iter().any(|o| acts(&o)),
                "seed {seed}: {due:?} acts on nothing"
            );
            if due.kind == DueKind::Hunt {
                assert!(
                    d.options.iter().any(|o| o.key.actor == Some(due.copy)
                        && (o.key.action == Some(ActionKind::Hunt)
                            || o.effects.iter().any(|e| e.kind == EffectKind::Hunt))),
                    "seed {seed}: {due:?} has no hunt"
                );
                hunts += 1;
            }
        }
    });
    assert!(hunts > 0, "no forced hunt met");
}

/// Minions of one name are told apart in the view (docs/protocol.md §3):
/// a second minion of a name, from any deck, carries `numbered`, distinct
/// from the others' and kept for the game; ghoul tokens never. Markers
/// are data too: a kind the protocol names, its count, an effect's card.
/// The first second of a name the human sees is seed 18 or so; a fixed
/// table: `view.rs`'s `minions_of_one_name_are_numbered_in_the_view`.
#[test]
fn minions_of_one_name_are_told_apart_and_markers_named() {
    use std::collections::HashMap;
    use zvtes_api::ids::{MarkerKind, MinionKind};
    let seeds: u32 = std::env::var("HOST_SEEDS").map_or(40, |s| s.parse().unwrap());
    let (mut seconds, mut markers) = (0, 0);
    let mut kept: HashMap<(u32, u32), Option<u8>> = HashMap::new();
    each_decision(seeds, |seed, d| {
        let mut names: Vec<(&str, Option<u8>)> = Vec::new();
        for s in &d.view.seats {
            for m in s.ready.iter().chain(&s.torpor) {
                let copy = m.card.copy.expect("a minion in sight is numbered");
                let was = kept.entry((seed, copy)).or_insert(m.card.numbered);
                assert_eq!(
                    *was, m.card.numbered,
                    "seed {seed}: {copy}'s number changed"
                );
                if m.kind == MinionKind::Token {
                    assert_eq!(m.card.numbered, None, "seed {seed}: a token numbered");
                    continue;
                }
                assert!(
                    !names.contains(&(m.name.as_str(), m.card.numbered)),
                    "seed {seed}: two {} alike ({:?})",
                    m.name,
                    m.card.numbered
                );
                names.push((&m.name, m.card.numbered));
                seconds += usize::from(m.card.numbered == Some(2));
                for k in &m.markers {
                    assert_ne!(k.kind, MarkerKind::Unknown, "seed {seed}: {k:?}");
                    if k.kind == MarkerKind::Effect {
                        assert!(k.card.is_some(), "seed {seed}: an effect unnamed: {k:?}");
                    }
                    markers += 1;
                }
            }
        }
    });
    assert!(
        seconds > 0,
        "no second minion of a name met ({markers} markers)"
    );
}
