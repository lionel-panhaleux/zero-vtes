//! The events a seat is told (docs/protocol.md §8), checked against the
//! views it is sent: every action and referendum that ends is told
//! ended, a combat is told begun, its strikes, then ended, and the pool,
//! blood and torpor the events last told are the table's. Every copy an
//! event names is numbered; English only when the settings ask.

use std::collections::HashMap;

use zvtes_api::event::{Event, EventKind};
use zvtes_api::spec::Views;
use zvtes_api::view::{Card, TableView};
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
        _: u32,
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
        ((self.0 >> 33) as usize) % n
    }
}

fn game(seed: u32, settings: Settings) -> Game {
    let decks = zvtes_host::catalog();
    let styles = ["aggro_bleed", "wall", "random"];
    let human = seed as usize % 4;
    let spec = GameSpec {
        seed: Some(seed),
        time_limit: Some(40),
        client: Some("tests".into()),
        seats: (0..4)
            .map(|i| {
                let (name, list) = &decks[(seed as usize + i * 5) % decks.len()];
                SeatSpec {
                    name: name.clone(),
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
    };
    Game::new(
        format!("g{seed}"),
        spec,
        &NoBots,
        Box::new(Ticks(Default::default())),
    )
    .unwrap()
}

/// An action or a referendum under way, as a view shows it.
type Under = (u8, u32, String, Option<u32>);

/// What the events told since the last view.
#[derive(Default)]
struct Stream {
    seed: u32,
    text: bool,
    /// The action and referendum at the last view (none after an undo).
    last: Option<(Option<Under>, bool)>,
    action_ended: bool,
    referendum_ended: bool,
    /// A combat told begun, not yet told ended; unknown after an undo.
    in_combat: Option<bool>,
    pools: HashMap<u8, i32>,
    blood: HashMap<u32, u8>,
    torpor: HashMap<u32, bool>,
    told: usize,
    texts: usize,
}

fn numbered(c: &Card) -> bool {
    c.copy.is_some() && c.id.is_some() && c.nth.is_some()
}

impl Stream {
    fn event(&mut self, e: &Event) {
        let seed = self.seed;
        self.told += 1;
        if let Some(t) = &e.text {
            assert!(self.text, "seed {seed}: text unasked: {e:?}");
            assert!(!t.is_empty(), "seed {seed}");
            self.texts += 1;
        }
        let copy = |c: u32| assert_ne!(c, u32::MAX, "seed {seed}: an unnumbered minion: {e:?}");
        match &e.kind {
            EventKind::Undone { .. } => {
                self.last = None;
                self.in_combat = None;
                self.pools.clear();
                self.blood.clear();
                self.torpor.clear();
            }
            EventKind::CombatBegins { minions } => {
                minions.iter().for_each(|&m| copy(m));
                assert_ne!(self.in_combat, Some(true), "seed {seed}: begun twice");
                self.in_combat = Some(true);
            }
            EventKind::Strikes { sides, .. } => {
                sides.iter().for_each(|s| copy(s.minion));
                assert_ne!(
                    self.in_combat,
                    Some(false),
                    "seed {seed}: strikes out of combat"
                );
            }
            EventKind::StrikeDeclared { minion, .. } => {
                copy(*minion);
                assert_ne!(
                    self.in_combat,
                    Some(false),
                    "seed {seed}: a strike out of combat"
                );
            }
            EventKind::CombatEnds => {
                assert_ne!(
                    self.in_combat,
                    Some(false),
                    "seed {seed}: an untold combat ended"
                );
                self.in_combat = Some(false);
            }
            EventKind::ActionEnds {
                acting, blocker, ..
            } => {
                copy(*acting);
                blocker.iter().for_each(|&m| copy(m));
                self.action_ended = true;
            }
            EventKind::ReferendumEnds { .. } => self.referendum_ended = true,
            EventKind::Pool { seat, pool, .. } => {
                self.pools.insert(*seat, *pool);
            }
            EventKind::Blood { minion, blood, .. } => {
                copy(*minion);
                self.blood.insert(*minion, *blood);
            }
            EventKind::Torpor { minion, enters } => {
                copy(*minion);
                self.torpor.insert(*minion, *enters);
            }
            EventKind::EntersPlay {
                card, on, effect, ..
            }
            | EventKind::Moves { card, on, effect } => {
                assert!(numbered(card) != *effect, "seed {seed}: {e:?}");
                on.iter().for_each(|&m| copy(m));
            }
            EventKind::LeavesPlay { card, effect, .. } => {
                assert!(numbered(card) != *effect, "seed {seed}: {e:?}");
                if let Some(c) = card.copy {
                    self.blood.remove(&c);
                    self.torpor.remove(&c);
                }
            }
            EventKind::BlockFails { minion } => copy(*minion),
            _ => {}
        }
    }

    fn view(&mut self, v: &TableView, at: &str) {
        let seed = self.seed;
        let action = v.action.as_ref().map(|a| {
            let card = a.card.and_then(|c| c.id);
            (a.seat, a.acting, format!("{:?}", a.kind), card)
        });
        let referendum = v.referendum.is_some();
        if let Some((had, had_referendum)) = &self.last {
            if had.is_some() && *had != action {
                assert!(
                    self.action_ended,
                    "seed {seed} {at}: an action's end untold: {had:?}"
                );
            }
            if *had_referendum && !referendum {
                assert!(
                    self.referendum_ended,
                    "seed {seed} {at}: a referendum's end untold"
                );
            }
        }
        for (&s, &pool) in &self.pools {
            assert_eq!(v.seats[s as usize].pool, pool, "seed {seed} {at}: pool");
        }
        for s in &v.seats {
            for (m, torpor) in s
                .ready
                .iter()
                .map(|m| (m, false))
                .chain(s.torpor.iter().map(|m| (m, true)))
            {
                let Some(c) = m.card.copy else { continue };
                if let Some(&b) = self.blood.get(&c) {
                    assert_eq!(m.blood, b, "seed {seed} {at}: {}'s blood", m.name);
                }
                if let Some(&t) = self.torpor.get(&c) {
                    assert_eq!(torpor, t, "seed {seed} {at}: {}'s torpor", m.name);
                }
            }
        }
        self.pools.clear();
        self.blood.clear();
        self.torpor.clear();
        self.action_ended = false;
        self.referendum_ended = false;
        self.last = Some((action, referendum));
    }

    fn see(&mut self, outs: &[Out]) {
        for o in outs {
            match &o.message {
                Message::Events { events, view } => {
                    events.iter().for_each(|e| self.event(e));
                    if let Some(v) = view {
                        self.view(v, &format!("seq {}", o.seq));
                    }
                }
                Message::Decision(d) => self.view(&d.view, &format!("decision {}", d.decision)),
                Message::Over(over) => self.view(&over.view, "over"),
                _ => {}
            }
        }
    }
}

fn cmd(command: Command) -> In {
    In { id: None, command }
}

#[test]
fn the_events_tell_what_the_views_show() {
    let seeds: u32 = std::env::var("HOST_SEEDS").map_or(16, |s| s.parse().unwrap());
    let (mut told, mut texts, mut undone) = (0, 0, 0);
    for seed in 1..=seeds {
        let settings = Settings {
            opportunities: seed % 3 != 0,
            views: Views::Steps,
            text: seed % 2 == 0,
        };
        let mut g = game(seed, settings);
        let human = g.human();
        let mut rng = Rng(seed as u64 * 7 + 1);
        let mut s = Stream {
            seed,
            text: settings.text,
            in_combat: Some(false),
            ..Stream::default()
        };
        let mut outs = g.handle(
            human,
            cmd(Command::Join {
                token: None,
                after: None,
            }),
        );
        s.see(&outs);
        let mut steps = 0;
        while !g.is_over() {
            steps += 1;
            assert!(steps < 20_000, "seed {seed}: no end");
            let d = outs
                .iter()
                .rev()
                .find_map(|o| match &o.message {
                    Message::Decision(d) => Some(d.clone()),
                    _ => None,
                })
                .expect("a decision until the end");
            let c = if d.undo && rng.below(6) == 0 {
                undone += 1;
                Command::Undo {
                    decision: d.decision,
                }
            } else {
                Command::Answer {
                    decision: d.decision,
                    option: d.options[rng.below(d.options.len())].index,
                }
            };
            outs = g.handle(human, cmd(c));
            s.see(&outs);
        }
        assert!(!settings.text || s.texts > 0, "seed {seed}: no text");
        told += s.told;
        texts += s.texts;
    }
    eprintln!("{seeds} games: {told} events ({texts} with text), {undone} undos");
}
