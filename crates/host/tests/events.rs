//! The events a seat is told (docs/protocol.md §8), checked against the
//! views it is sent: every action and referendum that ends is told
//! ended, a combat is told begun, its strikes, then ended, and the pool,
//! blood and torpor the events last told are the table's. Every copy an
//! event names is numbered; English only when the settings ask, and
//! then what `zvtes_text` renders from the event's data (A7's oracle).

use std::collections::HashMap;

use zvtes_api::event::{Event, EventKind};
use zvtes_api::ids::OptionKind;
use zvtes_api::spec::Views;
use zvtes_api::view::{Card, TableView};
use zvtes_api::{Command, GameSpec, In, Message, Out, Player, SeatSpec, Settings};
use zvtes_host::{Bots, Clock, Game};
use zvtes_text::Names;

/// The first seed a sweep plays (`HOST_SEED_FROM`, to look at one).
fn from() -> u32 {
    std::env::var("HOST_SEED_FROM").map_or(1, |s| s.parse().unwrap())
}

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

/// The precon with the most political action cards.
fn political(decks: &[(String, zvtes_api::DeckList)]) -> usize {
    use zvtes::library::{CardType, LIBRARY};
    let politics = |list: &zvtes_api::DeckList| -> u32 {
        list.iter()
            .filter(|(id, _)| {
                LIBRARY.iter().any(|c| {
                    c.id.to_string() == **id && c.types.contains(&CardType::PoliticalAction)
                })
            })
            .map(|(_, &n)| n)
            .sum()
    };
    (0..decks.len())
        .max_by_key(|&i| politics(&decks[i].1))
        .unwrap()
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
                // Every other game, the human plays the most political
                // precon: referendums, their allocations and votes.
                let pick = if i == human && seed.is_multiple_of(2) {
                    political(&decks)
                } else {
                    (seed as usize + i * 5) % decks.len()
                };
                let (name, list) = &decks[pick];
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
    let mut g = Game::new(
        format!("g{seed}"),
        spec,
        &NoBots,
        Box::new(Ticks(Default::default())),
    )
    .unwrap();
    // The oracle: the legacy lines, the renderer compared to them.
    g.legacy_text(true);
    g
}

/// An action or a referendum under way, as a view shows it.
type Under = (u8, u32, String, Option<u32>);

/// What the events told since the last view.
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
    names: Names<'static>,
    /// Each decision's view as first sent, and the decision an undo
    /// asks again: its view must be the same.
    asked: HashMap<u64, TableView>,
    again: Option<u64>,
    reasked: usize,
    /// Decisions an undo went back to that were never asked.
    unasked: Vec<u64>,
    /// The legacy lines the renderer told otherwise: (legacy, rendered).
    misses: Vec<(String, String)>,
}

fn numbered(c: &Card) -> bool {
    c.copy.is_some() && c.id.is_some() && c.nth.is_some()
}

impl Stream {
    fn event(&mut self, e: &Event) {
        let seed = self.seed;
        self.told += 1;
        self.names.hear(&e.kind);
        if std::env::var_os("HOST_TRACE").is_some() {
            eprintln!("event {:?} {:?}", e.kind, e.text);
        }
        if let Some(t) = &e.text {
            assert!(self.text, "seed {seed}: text unasked: {e:?}");
            assert!(!t.is_empty(), "seed {seed}");
            self.texts += 1;
            let rendered = zvtes_text::event(&e.kind, &self.names).unwrap_or_default();
            if &rendered != t {
                self.misses.push((t.clone(), rendered));
            }
        }
        let copy = |c: u32| assert_ne!(c, u32::MAX, "seed {seed}: an unnumbered minion: {e:?}");
        match &e.kind {
            EventKind::Undone { to, .. } => {
                self.again = Some(*to);
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
        self.names.see(v);
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
                // A step's view is the table after the step, or after the
                // whole allocation an `allocate` answered: checked where
                // everything is told (decisions, the end).
                Message::Events { events, view } => {
                    events.iter().for_each(|e| self.event(e));
                    if let Some(v) = view {
                        self.names.see(v);
                    }
                }
                Message::Decision(d) => {
                    if let Some(to) = self.again.take() {
                        assert_eq!(d.decision, to, "seed {}: asked again", self.seed);
                        let first = self.asked.get(&to);
                        if first.is_none() {
                            self.unasked.push(to);
                        }
                        if let Some(first) = first {
                            assert_eq!(&d.view, first, "seed {}: the view asked again", self.seed);
                            self.reasked += 1;
                        }
                    }
                    self.asked.insert(d.decision, d.view.clone());
                    self.view(&d.view, &format!("decision {}", d.decision))
                }
                Message::Over(over) => self.view(&over.view, "over"),
                Message::Joined(j) => self.view(&j.view, "joined"),
                _ => {}
            }
        }
    }
}

/// Amounts giving all the allocation's points, within each cap, to at
/// least `min_nonzero` recipients.
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

fn cmd(command: Command) -> In {
    In { id: None, command }
}

#[test]
fn the_events_tell_what_the_views_show() {
    let seeds: u32 = std::env::var("HOST_SEEDS").map_or(40, |s| s.parse().unwrap());
    let (mut told, mut texts, mut undone, mut allocated, mut voted) = (0, 0, 0, 0, 0);
    let mut reasked = 0;
    let mut misses = Vec::new();
    for seed in from()..=seeds {
        let settings = Settings {
            opportunities: seed % 3 != 0,
            views: Views::Steps,
            text: seed.is_multiple_of(2),
        };
        let mut g = game(seed, settings);
        let human = g.human();
        let mut rng = Rng(seed as u64 * 7 + 1);
        let mut s = Stream {
            seed,
            text: settings.text,
            last: None,
            action_ended: false,
            referendum_ended: false,
            in_combat: Some(false),
            pools: HashMap::new(),
            blood: HashMap::new(),
            torpor: HashMap::new(),
            told: 0,
            texts: 0,
            names: Names::new(zvtes_host::cards::cards()),
            misses: Vec::new(),
            asked: HashMap::new(),
            again: None,
            reasked: 0,
            unasked: Vec::new(),
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
            let c = if let Some(a) = d.allocation.as_ref().filter(|_| rng.below(2) == 0) {
                let Some(amounts) = amounts(a, &mut rng) else {
                    panic!("seed {seed}: an allocation with no way to give its points: {a:?}");
                };
                // Too many points: refused, nothing changes.
                let mut bad = amounts.clone();
                bad[0] += 1;
                let refused = g.handle(
                    human,
                    cmd(Command::Allocate {
                        decision: d.decision,
                        amounts: bad,
                    }),
                );
                assert!(
                    matches!(
                        refused[..],
                        [Out {
                            message: Message::Error(_),
                            ..
                        }]
                    ),
                    "seed {seed}: {refused:?}"
                );
                allocated += 1;
                Command::Allocate {
                    decision: d.decision,
                    amounts,
                }
            } else if let Some(v) = d
                .votes
                .filter(|v| v.favor + v.against > 0 && rng.below(3) == 0)
            {
                voted += 1;
                Command::VoteAll {
                    decision: d.decision,
                    favor: v.favor > 0 && (v.against == 0 || rng.below(2) == 0),
                }
            } else if d.undo && rng.below(6) == 0 {
                undone += 1;
                Command::Undo {
                    decision: d.decision,
                }
            } else {
                // Cards played half the time: referendums (allocations,
                // votes) come of political actions.
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
            s.see(&outs);
        }
        assert!(!settings.text || s.texts > 0, "seed {seed}: no text");
        reasked += s.reasked;
        assert!(
            s.unasked.is_empty(),
            "seed {seed}: undone to {:?}, never asked",
            s.unasked
        );
        told += s.told;
        texts += s.texts;
        misses.extend(s.misses.into_iter().map(|m| (seed, m)));
    }
    for (seed, (legacy, rendered)) in misses.iter().take(20) {
        eprintln!("seed {seed}: {legacy:?} rendered {rendered:?}");
    }
    assert!(
        misses.is_empty(),
        "{} events rendered otherwise",
        misses.len()
    );
    eprintln!(
        "{seeds} games: {told} events ({texts} with text), {undone} undos, \
         {allocated} allocations, {voted} votes cast at once, {reasked} views asked again alike"
    );
    assert!(
        allocated > 0 && voted > 0,
        "allocations and votes at once untried"
    );
}

/// A rendering of an ability or a play that tells the legacy line's
/// option and all its parts: the same up to its label, every part of
/// the legacy label among the rendering's.
fn reworded(kind: OptionKind, legacy: &str, r: &str) -> bool {
    if !matches!(kind, OptionKind::UseAbility | OptionKind::Play) {
        return false;
    }
    let (Some((lp, ll)), Some((rp, rl))) = (legacy.split_once(": "), r.split_once(": ")) else {
        return false;
    };
    let rparts: Vec<&str> = rl.split(", ").collect();
    lp == rp && ll.split(", ").all(|p| rparts.contains(&p))
}

/// The options a seat is shown render, from their data, as the legacy
/// text (A7's oracle): what the renderer tells must be the legacy line,
/// and it tells every option (one it does not, `None`, is listed by kind:
/// a field missing).
#[test]
fn the_options_render_as_the_legacy_text() {
    let seeds: u32 = std::env::var("HOST_SEEDS").map_or(40, |s| s.parse().unwrap());
    let mut misses: Vec<(u32, String, String)> = Vec::new();
    let mut left: std::collections::BTreeMap<String, (usize, String)> = Default::default();
    let (mut shown, mut rendered) = (0, 0);
    let mut rewordings: std::collections::BTreeSet<(String, String)> = Default::default();
    // Prompts alike: (asked, rendered), left by window and purpose.
    let (mut prompts, mut prompted) = (0, 0);
    let mut prompts_left: std::collections::BTreeMap<String, (usize, String)> = Default::default();
    let mut prompt_misses: Vec<(u32, String, String)> = Vec::new();
    for seed in from()..=seeds {
        let settings = Settings {
            opportunities: seed % 3 != 0,
            views: Views::Decisions,
            text: true,
        };
        let mut g = game(seed, settings);
        let human = g.human();
        let mut rng = Rng(seed as u64 * 11 + 3);
        let mut names = Names::new(zvtes_host::cards::cards());
        let mut outs = g.handle(
            human,
            cmd(Command::Join {
                token: None,
                after: None,
            }),
        );
        while !g.is_over() {
            for o in &outs {
                match &o.message {
                    Message::Joined(j) => names.see(&j.view),
                    Message::Events { events, view } => {
                        events.iter().for_each(|e| names.hear(&e.kind));
                        view.iter().for_each(|v| names.see(v));
                    }
                    _ => {}
                }
            }
            let d = outs
                .iter()
                .rev()
                .find_map(|o| match &o.message {
                    Message::Decision(d) => Some(d.clone()),
                    _ => None,
                })
                .unwrap();
            names.see(&d.view);
            if let Some(legacy) = &d.prompt {
                prompts += 1;
                match zvtes_text::prompt(&d, &names) {
                    Some(r) if &r == legacy => prompted += 1,
                    Some(r) => prompt_misses.push((seed, legacy.clone(), r)),
                    None => {
                        let e = prompts_left
                            .entry(match d.purpose {
                                Some(p) => format!("for {p:?}"),
                                None => format!("in {:?}", d.window),
                            })
                            .or_insert((0, legacy.clone()));
                        e.0 += 1;
                    }
                }
            }
            for o in &d.options {
                let legacy = o.text.clone().unwrap_or_default();
                shown += 1;
                match zvtes_text::option(o, d.window, d.purpose, d.seat, &d.options, &names) {
                    Some(r) if r == legacy => rendered += 1,
                    // Abilities and plays told from the curated effects
                    // (owner, B): the same option, every legacy part in
                    // it, maybe more (what all the source's abilities do,
                    // when one is offered alone).
                    Some(r) if reworded(o.key.kind, &legacy, &r) => {
                        rendered += 1;
                        rewordings.insert((legacy, r));
                    }
                    Some(r) => misses.push((seed, legacy, r)),
                    None => {
                        let e = left
                            .entry(match d.purpose {
                                Some(p) => format!("{:?} for {p:?}", o.key.kind),
                                None => format!("{:?} in {:?}", o.key.kind, d.window),
                            })
                            .or_insert((0, legacy.clone()));
                        e.0 += 1;
                    }
                }
            }
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
            outs = g.handle(
                human,
                cmd(Command::Answer {
                    decision: d.decision,
                    option: o.index,
                }),
            );
        }
    }
    let mut left: Vec<_> = left.into_iter().collect();
    left.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    for (kind, (n, eg)) in left.iter().take(60) {
        eprintln!("left: {n:6} {kind} (e.g. {eg:?})");
    }
    for (seed, legacy, r) in misses.iter().take(25) {
        eprintln!("seed {seed}: {legacy:?} rendered {r:?}");
    }
    for (legacy, r) in &rewordings {
        eprintln!("reworded: {legacy:?} as {r:?}");
    }
    eprintln!(
        "{seeds} games: {rendered} of {shown} options rendered ({} reworded)",
        rewordings.len()
    );
    let mut prompts_left: Vec<_> = prompts_left.into_iter().collect();
    prompts_left.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    for (kind, (n, eg)) in &prompts_left {
        eprintln!("prompt left: {n:6} {kind} (e.g. {eg:?})");
    }
    for (seed, legacy, r) in prompt_misses.iter().take(25) {
        eprintln!("seed {seed}: prompt {legacy:?} rendered {r:?}");
    }
    eprintln!("{seeds} games: {prompted} of {prompts} prompts rendered");
    assert!(
        prompt_misses.is_empty(),
        "{} prompts rendered otherwise",
        prompt_misses.len()
    );
    assert!(
        misses.is_empty(),
        "{} options rendered otherwise",
        misses.len()
    );
    // Every option renders (A7 done): one not told is a missing field.
    assert!(left.is_empty(), "{} options not rendered", shown - rendered);
}

/// What the host sends with the text setting: the renderer's English, the
/// same game played twice (the legacy lines, the renderer's) told alike —
/// every option, event and prompt, abilities and plays perhaps reworded
/// (every legacy part told).
#[test]
fn the_host_sends_the_renderers_text() {
    let seeds: u32 = std::env::var("HOST_SEEDS").map_or(20, |s| s.parse().unwrap());
    let (mut options, mut events) = (0, 0);
    for seed in from()..=seeds {
        let settings = Settings {
            opportunities: seed % 3 != 0,
            views: Views::Decisions,
            text: true,
        };
        let (mut a, mut b) = (game(seed, settings), game(seed, settings));
        b.legacy_text(false);
        let human = a.human();
        let mut rng = Rng(seed as u64 * 7 + 1);
        let join = || {
            cmd(Command::Join {
                token: None,
                after: None,
            })
        };
        let (mut outs_a, mut outs_b) = (a.handle(human, join()), b.handle(human, join()));
        loop {
            assert_eq!(outs_a.len(), outs_b.len(), "seed {seed}");
            let mut decision = None;
            for (x, y) in outs_a.iter().zip(&outs_b) {
                match (&x.message, &y.message) {
                    (Message::Events { events: ex, .. }, Message::Events { events: ey, .. }) => {
                        for (ex, ey) in ex.iter().zip(ey) {
                            assert_eq!(ex.kind, ey.kind, "seed {seed}");
                            assert_eq!(ex.text, ey.text, "seed {seed}");
                            events += ex.text.is_some() as usize;
                        }
                    }
                    (Message::Decision(dx), Message::Decision(dy)) => {
                        assert_eq!(dx.prompt, dy.prompt, "seed {seed}");
                        for (ox, oy) in dx.options.iter().zip(&dy.options) {
                            let (lx, ly) = (ox.text.as_deref(), oy.text.as_deref());
                            let (Some(lx), Some(ly)) = (lx, ly) else {
                                panic!("seed {seed}: an option without text: {ox:?}");
                            };
                            assert!(
                                lx == ly || reworded(ox.key.kind, lx, ly),
                                "seed {seed}: {lx:?} sent as {ly:?}"
                            );
                            options += 1;
                        }
                        let indices: Vec<_> = dx.options.iter().map(|o| o.index).collect();
                        decision = Some((dx.decision, indices));
                    }
                    _ => {}
                }
            }
            let Some((decision, indices)) = decision.filter(|_| !a.is_over()) else {
                break;
            };
            let command = cmd(Command::Answer {
                decision,
                option: indices[rng.below(indices.len())],
            });
            outs_a = a.handle(human, command.clone());
            outs_b = b.handle(human, command);
        }
    }
    eprintln!("{seeds} games: {options} options and {events} events sent alike");
}
