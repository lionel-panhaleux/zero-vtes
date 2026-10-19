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
        _: u64,
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

fn spec(seed: u64, human: usize, settings: Settings) -> GameSpec {
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
        spec(seed.into(), human, settings),
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
/// forced hunt the human meets is seed 115 or so (a lone hunt elides).
#[test]
fn a_minion_phase_without_a_pass_says_why() {
    let seeds: u32 = std::env::var("HOST_SEEDS").map_or(230, |s| s.parse().unwrap());
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

/// An allocation's amounts, each within its recipient's cap, the least
/// recipients given some first (as the events sweep gives them).
fn amounts(a: &zvtes_api::option::Allocation, rng: &mut Rng) -> Option<Vec<u8>> {
    let n = a.recipients.len();
    let cap = |i: usize| a.recipients[i].cap.unwrap_or(a.points);
    let mut out = vec![0u8; n];
    let mut left = a.points;
    for (i, o) in out.iter_mut().enumerate().take(a.min_nonzero as usize) {
        if left > 0 && cap(i) > 0 {
            *o = 1;
            left -= 1;
        }
    }
    while left > 0 {
        let open: Vec<usize> = (0..n).filter(|&i| out[i] < cap(i)).collect();
        if open.is_empty() {
            return None;
        }
        out[open[rng.below(open.len())]] += 1;
        left -= 1;
    }
    Some(out)
}

/// A game left going is rebuilt from its record (a server restarted):
/// at the same decision, the same options, and from there it plays on as
/// the original does, answer for answer (its bots' moves taken as
/// recorded, their agents then as they were); undos, settings changes,
/// allocations answered whole and vote orders on the way replayed too,
/// the undo depth as they left it.
#[test]
fn a_game_resumed_from_its_record_plays_on_as_it_was() {
    let seeds: u32 = std::env::var("HOST_SEEDS").map_or(8, |s| s.parse().unwrap());
    let (mut undone, mut resumed, mut allocated, mut voted) = (0, 0, 0, 0);
    for seed in 1..=seeds {
        let settings = Settings {
            opportunities: seed % 2 == 0,
            ..Settings::default()
        };
        let mut g = game(seed, settings);
        let human = g.human();
        let mut rng = Rng(seed as u64 * 13 + 5);
        let mut outs = g.handle(human, join());
        let cut = 20 + rng.below(60);
        let mut answered = 0;
        // Play to the cut, an undo or a settings change now and then.
        while !g.is_over() && answered < cut {
            let d = pending(&outs).cloned().expect("a decision");
            let whole = d.allocation.as_ref().and_then(|a| amounts(a, &mut rng));
            let c = if let Some(amounts) = whole.filter(|_| rng.below(2) == 0) {
                allocated += 1;
                Command::Allocate {
                    decision: d.decision,
                    amounts,
                }
            } else if let Some(v) = d
                .votes
                .filter(|v| v.favor + v.against > 0 && rng.below(2) == 0)
            {
                voted += 1;
                Command::VoteAll {
                    decision: d.decision,
                    favor: v.favor > 0,
                }
            } else if d.undo && rng.below(8) == 0 {
                undone += 1;
                Command::Undo {
                    decision: d.decision,
                }
            } else if rng.below(15) == 0 {
                Command::Settings(SettingsPatch {
                    opportunities: Some(rng.below(2) == 0),
                    ..SettingsPatch::default()
                })
            } else {
                answered += 1;
                Command::Answer {
                    decision: d.decision,
                    option: d.options[rng.below(d.options.len())].index,
                }
            };
            outs = g.handle(human, cmd(c));
            if pending(&outs).is_none() && !g.is_over() {
                outs = g.handle(human, join());
            }
        }
        if g.is_over() {
            continue;
        }
        let record = g.record().clone();
        let mut r = Game::resume(
            format!("g{seed}"),
            &record,
            &NoBots,
            Box::new(Ticks(Default::default())),
        )
        .unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        resumed += 1;
        assert_eq!(
            r.record().entries,
            record.entries,
            "seed {seed}: the record kept"
        );
        let mut routs = r.handle(human, join());
        // Answer for answer, both games alike.
        for step in 0..40 {
            let (Some(a), Some(b)) = (pending(&outs).cloned(), pending(&routs).cloned()) else {
                assert_eq!(g.is_over(), r.is_over(), "seed {seed} step {step}");
                break;
            };
            assert_eq!(a.decision, b.decision, "seed {seed} step {step}");
            let keys = |d: &zvtes_api::Decision| -> Vec<_> {
                d.options.iter().map(|o| (o.index, o.key.clone())).collect()
            };
            assert_eq!(keys(&a), keys(&b), "seed {seed} step {step}: the options");
            let option = a.options[rng.below(a.options.len())].index;
            let answer = |decision| cmd(Command::Answer { decision, option });
            outs = g.handle(human, answer(a.decision));
            routs = r.handle(human, answer(b.decision));
        }
        let decisions = |g: &Game| -> Vec<_> {
            g.record()
                .entries
                .iter()
                .filter_map(|e| match e {
                    Entry::Decision {
                        decision, choice, ..
                    } => Some((*decision, choice.clone())),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(
            decisions(&g),
            decisions(&r),
            "seed {seed}: the records alike"
        );
    }
    assert!(
        resumed > 0 && undone > 0,
        "{resumed} resumed, {undone} undos, {allocated} allocations, {voted} vote orders"
    );
}

/// A vote order (`vote_all`) and an allocation answered whole
/// (`allocate`) resume as they were taken: the resumed game at the same
/// decision, its undo where the original's is, and on as the original
/// plays (each game found where it first offers one).
#[test]
fn a_resumed_game_keeps_its_vote_orders_and_allocations() {
    let (mut voted, mut allocated) = (0, 0);
    // About twice the seed of the first allocation (171 over the catalog
    // of 153 kept decks): the catalog's size deals every seed's decks.
    for seed in 1..=400 {
        if voted > 0 && allocated > 0 {
            break;
        }
        let mut g = game(seed, Settings::default());
        let human = g.human();
        let mut rng = Rng(seed as u64 * 17 + 3);
        let mut outs = g.handle(human, join());
        let mut taken = None;
        // To the first vote order or whole allocation, taken; resumed just
        // after it, where the undo depth and the order's casts tell.
        while !g.is_over() && taken.is_none() {
            let Some(d) = pending(&outs).cloned() else {
                break;
            };
            let c = match (d.votes, d.allocation.as_ref()) {
                (Some(v), _) if voted == 0 && v.favor + v.against > 0 => {
                    taken = Some("vote");
                    Command::VoteAll {
                        decision: d.decision,
                        favor: v.favor > 0,
                    }
                }
                (_, Some(a)) if allocated == 0 && amounts(a, &mut Rng(1)).is_some() => {
                    taken = Some("allocation");
                    Command::Allocate {
                        decision: d.decision,
                        amounts: amounts(a, &mut rng).expect("amounts"),
                    }
                }
                _ => Command::Answer {
                    decision: d.decision,
                    option: d.options[rng.below(d.options.len())].index,
                },
            };
            outs = g.handle(human, cmd(c));
        }
        let (Some(kind), false) = (taken, g.is_over()) else {
            continue;
        };
        let before = pending(&outs).cloned().expect("a decision");
        let mut r = Game::resume(
            format!("g{seed}"),
            g.record(),
            &NoBots,
            Box::new(Ticks(Default::default())),
        )
        .unwrap_or_else(|e| panic!("seed {seed} ({kind}): {e}"));
        let mut routs = r.handle(human, join());
        let now = pending(&routs).cloned().expect("a decision");
        assert_eq!(
            (now.decision, now.undo),
            (before.decision, before.undo),
            "seed {seed} ({kind})"
        );
        // The undo, in both: to the same decision.
        if before.undo {
            outs = g.handle(
                human,
                cmd(Command::Undo {
                    decision: before.decision,
                }),
            );
            routs = r.handle(
                human,
                cmd(Command::Undo {
                    decision: now.decision,
                }),
            );
            assert_eq!(
                pending(&outs).map(|d| d.decision),
                pending(&routs).map(|d| d.decision),
                "seed {seed} ({kind}): the undo"
            );
        }
        // On, answer for answer: an order's casts as it cast them.
        for step in 0..15 {
            let (Some(a), Some(b)) = (pending(&outs).cloned(), pending(&routs).cloned()) else {
                break;
            };
            assert_eq!(a.decision, b.decision, "seed {seed} ({kind}) step {step}");
            let option = a.options[rng.below(a.options.len())].index;
            let answer = |decision| cmd(Command::Answer { decision, option });
            outs = g.handle(human, answer(a.decision));
            routs = r.handle(human, answer(b.decision));
        }
        match kind {
            "vote" => voted += 1,
            _ => allocated += 1,
        }
    }
    assert!(
        voted > 0 && allocated > 0,
        "{voted} vote orders, {allocated} allocations"
    );
}

/// A bot that plays otherwise each time it is made (as a changed engine
/// or new weights would), recorded alike: a resumed game reaches its
/// decision only by taking the bots' moves as recorded.
struct Fickle(std::cell::Cell<u64>);

impl Bots for Fickle {
    fn profiles(&self) -> Vec<String> {
        vec!["fickle".into()]
    }
    fn make(
        &self,
        profile: &str,
        seed: u64,
    ) -> Option<(Box<dyn zvtes::agent::Agent>, zvtes_api::record::BotRecord)> {
        (profile == "fickle").then(|| {
            self.0.set(self.0.get() + 1);
            let agent = zvtes::agent::RandomLegal::new(seed.wrapping_mul(1000) + self.0.get());
            let r = zvtes_api::record::BotRecord {
                seat: 0,
                profile: "fickle".into(),
                kind: "random".into(),
                weights: None,
                temperature: None,
                seed,
                nudges: Default::default(),
            };
            (Box::new(agent) as Box<dyn zvtes::agent::Agent>, r)
        })
    }
}

#[test]
fn a_resumed_games_bots_take_their_recorded_moves() {
    let bots = Fickle(Default::default());
    let mut checked = 0;
    for seed in 1..=6 {
        let mut s = spec(seed, 0, Settings::default());
        for seat in &mut s.seats[1..] {
            seat.player = Player::Bot {
                profile: "fickle".into(),
            };
        }
        let mut g = Game::new(
            format!("f{seed}"),
            s,
            &bots,
            Box::new(Ticks(Default::default())),
        )
        .unwrap();
        let mut rng = Rng(seed * 3 + 1);
        let mut outs = g.handle(0, join());
        for _ in 0..40 {
            let Some(d) = pending(&outs).cloned() else {
                break;
            };
            let option = d.options[rng.below(d.options.len())].index;
            outs = g.handle(
                0,
                cmd(Command::Answer {
                    decision: d.decision,
                    option,
                }),
            );
        }
        let Some(before) = pending(&outs).cloned() else {
            continue;
        };
        let mut r = Game::resume(
            format!("f{seed}"),
            g.record(),
            &bots,
            Box::new(Ticks(Default::default())),
        )
        .unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        let after = pending(&r.handle(0, join())).cloned().expect("a decision");
        assert_eq!(before.decision, after.decision, "seed {seed}");
        assert_eq!(
            before.options.iter().map(|o| &o.key).collect::<Vec<_>>(),
            after.options.iter().map(|o| &o.key).collect::<Vec<_>>(),
            "seed {seed}"
        );
        checked += 1;
    }
    assert!(checked > 0);
}

/// The record of the first game from `seed` on still going after
/// `answers` of the human's answers.
fn midgame(seed: u32, answers: usize) -> zvtes_api::record::GameRecord {
    played(seed, answers, |seed| game(seed, Settings::default()))
}

/// As `midgame`, the seats playing four precons named, so its deal reads
/// no catalog index (the catalog grows with each cards round).
fn precon_midgame(seed: u32, answers: usize) -> zvtes_api::record::GameRecord {
    const DECKS: [&str; 4] = [
        "Hecata (Fifth Edition)",
        "Lasombra (Fifth Edition)",
        "Malkavian (Fifth Edition)",
        "Nosferatu (Fifth Edition)",
    ];
    let catalog = zvtes_host::catalog();
    played(seed, answers, |seed| {
        let mut s = spec(seed.into(), seed as usize % 4, Settings::default());
        for (seat, name) in s.seats.iter_mut().zip(DECKS) {
            seat.deck = catalog
                .iter()
                .find(|(n, _)| n == name)
                .expect(name)
                .1
                .clone();
        }
        Game::new(
            format!("p{seed}"),
            s,
            &NoBots,
            Box::new(Ticks(Default::default())),
        )
        .unwrap()
    })
}

fn played(
    seed: u32,
    answers: usize,
    mut make: impl FnMut(u32) -> Game,
) -> zvtes_api::record::GameRecord {
    for seed in seed.. {
        let mut g = make(seed);
        let human = g.human();
        let mut rng = Rng(seed as u64 * 7 + 3);
        let mut outs = g.handle(human, join());
        for _ in 0..answers {
            let Some(d) = pending(&outs).cloned() else {
                break;
            };
            let option = d.options[rng.below(d.options.len())].index;
            outs = g.handle(
                human,
                cmd(Command::Answer {
                    decision: d.decision,
                    option,
                }),
            );
        }
        if !g.is_over() {
            return g.record().clone();
        }
    }
    unreachable!()
}

/// The decision entries' positions in `entries`.
fn decision_entries(record: &zvtes_api::record::GameRecord) -> Vec<usize> {
    (0..record.entries.len())
        .filter(|&i| matches!(record.entries[i], Entry::Decision { .. }))
        .collect()
}

/// How far a record reads: its replay's decisions, the reason it stops,
/// and its resumed game's error.
fn reads(record: &zvtes_api::record::GameRecord) -> (usize, Option<String>, Option<String>) {
    let replay = zvtes_host::replay::Replay::new(record).unwrap();
    let read = replay.read(usize::MAX).1;
    let mut r = zvtes_host::replay::Replayer::new(record).unwrap();
    let why = loop {
        match r.step() {
            zvtes_host::replay::Step::Decision(d) if d.chosen.is_some() => r.take(),
            zvtes_host::replay::Step::Stopped { why, .. } => break Some(why),
            _ => break None,
        }
    };
    let resumed = Game::resume(
        "r".into(),
        record,
        &NoBots,
        Box::new(Ticks(Default::default())),
    )
    .err();
    (read, why, resumed)
}

/// A rules change moving hidden state with no decision (docs/protocol.md
/// §11): a record from 2.4.0 stops at the first decision whose position
/// differs from the one recorded, replayed and resumed alike, though its
/// choice is still offered.
#[test]
fn a_record_stops_where_its_position_differs() {
    let record = midgame(1, 30);
    let at = decision_entries(&record);
    assert_eq!(
        reads(&record),
        (at.len(), None, None),
        "the record reads whole"
    );
    let k = at.len() / 2;
    let mut moved = record.clone();
    let Entry::Decision {
        digest, decision, ..
    } = &mut moved.entries[at[k]]
    else {
        unreachable!()
    };
    *digest = Some("0123456789abcdef".into());
    let why = format!("decision {decision}: the position differs from the record's");
    assert_eq!(reads(&moved), (k, Some(why.clone()), Some(why)));
}

/// The Passes recorded in a window a later engine no longer asks are read
/// past when the next decision is the one asked, in its window and
/// position (a replay; a resumed game ends there); a window newly asked
/// stops the record.
#[test]
fn a_pass_in_a_window_gone_is_read_past_and_a_window_newly_asked_stops() {
    let record = midgame(2, 30);
    let at = decision_entries(&record);
    let pass = at
        .iter()
        .find_map(|&i| match &record.entries[i] {
            Entry::Decision { choice, .. } if choice.kind == OptionKind::Pass => {
                Some(choice.clone())
            }
            _ => None,
        })
        .expect("a pass");
    let k = at.len() / 2;
    let Entry::Decision {
        decision,
        seat,
        window: Some(window),
        digest,
        ..
    } = record.entries[at[k]].clone()
    else {
        unreachable!()
    };
    let gone = if window == zvtes_api::ids::Window::ActionAfterResolution {
        zvtes_api::ids::Window::MinionPhase
    } else {
        zvtes_api::ids::Window::ActionAfterResolution
    };
    // The window gone, as an older engine recorded it: two seats' Passes
    // there before the decision asked, every number after them two up.
    let mut older = record.clone();
    for e in &mut older.entries[at[k]..] {
        match e {
            Entry::Decision { decision: n, .. } => *n += 2,
            Entry::Undo { to, .. } => *to += 2,
            _ => {}
        }
    }
    for (i, s) in [seat, (seat + 1) % 4].into_iter().enumerate() {
        older.entries.insert(
            at[k] + i,
            Entry::Decision {
                decision: decision + i as u64,
                seat: s,
                at: 0,
                option: 0,
                choice: pass.clone(),
                by: zvtes_api::record::By::Bot,
                forced: false,
                shown: None,
                window: Some(gone),
                digest: digest.clone(),
            },
        );
    }
    // Replayed, read past; resumed, the game ends there (its numbers would
    // no longer be the session's).
    let why =
        format!("decision {decision}: seat {seat} asked in {window:?}, the record's in {gone:?}");
    assert_eq!(
        reads(&older),
        (at.len(), None, Some(why.clone())),
        "read past the window gone"
    );
    // Each decision read names the entry it took, the Passes skipped.
    let mut taken = vec![];
    let mut r = zvtes_host::replay::Replayer::new(&older).unwrap();
    while let zvtes_host::replay::Step::Decision(d) = r.step() {
        let Some(entry) = d.entry else { break };
        taken.push(entry);
        r.take();
    }
    let kept: Vec<usize> = decision_entries(&older)
        .into_iter()
        .filter(|&i| i != at[k] && i != at[k] + 1)
        .collect();
    assert_eq!(taken, kept, "the entries taken");
    // The window newly asked: the record's decision taken in another one.
    let mut newer = record.clone();
    let Entry::Decision { window: w, .. } = &mut newer.entries[at[k]] else {
        unreachable!()
    };
    *w = Some(gone);
    assert_eq!(reads(&newer), (k, Some(why.clone()), Some(why)));
}

/// The digest is stable (docs/protocol.md §11): a fixed deal's first
/// decisions, pinned. It moves only when the position does (the four
/// precons' lists, the dealing, the first turns' rules); its definition is
/// frozen (a new one read by the record's `protocol`).
#[test]
fn the_position_digests_of_a_fixed_deal_are_pinned() {
    let record = precon_midgame(3, 4);
    let digests: Vec<String> = record
        .entries
        .iter()
        .filter_map(|e| match e {
            Entry::Decision { digest, .. } => digest.clone(),
            _ => None,
        })
        .take(4)
        .collect();
    assert_eq!(
        digests,
        [
            "20b6371c3bbc3b7d",
            "cec658129cbe7f4b",
            "8b757eb4b61ac97d",
            "401e8bb61595b17f"
        ]
    );
}

/// The digest's whole definition pinned on a position built by hand: a
/// ready minion with a card on it, a locked one, one in torpor, a card on
/// the table, an uncontrolled vampire with counters, a burned vampire.
#[test]
fn the_digest_of_a_built_position_is_pinned() {
    use zvtes::copies::{CopyId, Untracked};
    use zvtes::ids::{MinionId, SeatId};
    use zvtes::state::{CardInPlay, Minion, MinionKind, UncontrolledSlot};
    let crypt = |n: &str| {
        zvtes::crypt::CRYPT
            .iter()
            .position(|c| c.name == n)
            .unwrap() as u16
    };
    let library = |n: &str| {
        zvtes::library::LIBRARY
            .iter()
            .position(|c| c.name == n)
            .unwrap() as u16
    };
    let record = precon_midgame(3, 4);
    let mut e = zvtes_host::replay::Replay::new(&record).unwrap().deal();
    let st = &mut e.state;
    let first = st.minions.len() as u32;
    st.minions.push(Minion::new(
        SeatId(0),
        MinionKind::Vampire(crypt("Martina Srnankova")),
        3,
    ));
    st.minions.push(Minion {
        locked: true,
        ..Minion::new(SeatId(0), MinionKind::Vampire(crypt("Joaquín de Cádiz")), 1)
    });
    st.minions.push(Minion {
        in_torpor: true,
        ..Minion::new(SeatId(1), MinionKind::Vampire(crypt("Jean-François")), 0)
    });
    st.cards.push(CardInPlay::entering(
        library("Laptop Computer"),
        CopyId::NONE,
        SeatId(0),
        Some(MinionId(first)),
        0,
    ));
    st.cards.push(CardInPlay::entering(
        library("Baltimore Purge"),
        CopyId::NONE,
        SeatId(1),
        None,
        0,
    ));
    st.seats[2].uncontrolled.push(UncontrolledSlot {
        card: crypt("Tabitha Fisk"),
        copy: Untracked(CopyId::NONE),
        counters: 2,
        banished: None,
    });
    st.seats[3]
        .ash_crypt
        .put(crypt("Elisha Tucker"), CopyId::NONE);
    assert_eq!(zvtes_host::digest::digest(&e), "10d41669a03aeac7");
}
