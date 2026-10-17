//! The events a seat is told (docs/protocol.md §8), checked against the
//! views it is sent: every action and referendum that ends is told
//! ended, a combat is told begun, its strikes, then ended, and the pool,
//! blood and torpor the events last told are the table's. Every copy an
//! event names is numbered; English only when the settings ask, and
//! then what `zvtes_text` renders from the event's data (A7's oracle).

use std::collections::HashMap;

use zvtes_api::event::{Event, EventKind};
use zvtes_api::ids::{MinionKind, OptionKind};
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
    /// Each minion's `numbered`, as the views and its entering or leaving
    /// play told it (forgotten at an undo: wire copies are given again).
    name_numbers: HashMap<u32, Option<u8>>,
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
            let rendered = match e.kind {
                // Its option's line (`zvtes_text::option`).
                EventKind::Decided { .. } => t.clone(),
                _ => zvtes_text::event(&e.kind, &self.names).unwrap_or_default(),
            };
            if &rendered != t {
                self.misses.push((t.clone(), rendered));
            }
        }
        // A minion entering or leaving play: its name's number as the
        // views give it (docs/protocol.md §3), a ghoul token's none.
        if let EventKind::EntersPlay {
            card,
            minion: Some(kind),
            ..
        }
        | EventKind::LeavesPlay {
            card,
            minion: Some(kind),
            ..
        } = &e.kind
        {
            if let Some(c) = card.copy {
                if *kind == MinionKind::Token {
                    assert_eq!(card.numbered, None, "seed {seed}: a token numbered");
                }
                self.number(c, card.numbered, "an event");
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
                self.name_numbers.clear();
            }
            EventKind::CombatBegins { minions } => {
                minions.iter().for_each(|&m| copy(m));
                assert_ne!(self.in_combat, Some(true), "seed {seed}: begun twice");
                self.in_combat = Some(true);
            }
            EventKind::Strikes { sides, .. } => {
                sides.iter().for_each(|s| copy(s.minion));
                for s in sides {
                    assert!(
                        s.strike_damage <= s.damage,
                        "seed {seed}: a strike's share beyond the damage: {s:?}"
                    );
                }
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

    /// `c`'s name number, `n`, as one message tells it: the same as every
    /// other told it since the last undo.
    fn number(&mut self, c: u32, n: Option<u8>, what: &str) {
        let seed = self.seed;
        let was = *self.name_numbers.entry(c).or_insert(n);
        assert_eq!(was, n, "seed {seed}: {c}'s number in {what}");
    }

    fn view(&mut self, v: &TableView, at: &str) {
        let seed = self.seed;
        self.names.see(v);
        // Minions of one name told apart, each as told before.
        let mut names: Vec<(&str, Option<u8>)> = Vec::new();
        for m in v.seats.iter().flat_map(|s| s.ready.iter().chain(&s.torpor)) {
            if let Some(c) = m.card.copy {
                self.number(c, m.card.numbered, at);
            }
            if m.kind == MinionKind::Token {
                continue;
            }
            let n = (m.name.as_str(), m.card.numbered);
            assert!(!names.contains(&n), "seed {seed} {at}: two {n:?}");
            names.push(n);
        }
        // A combat's strikes in English exactly when the settings ask.
        for s in v
            .combat
            .iter()
            .flat_map(|c| &c.sides)
            .filter_map(|s| s.strike.as_ref())
        {
            let want = self.text.then(|| zvtes_text::strike_declared(s));
            assert_eq!(s.text, want, "seed {seed} {at}: a strike's text");
        }
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
            name_numbers: HashMap::new(),
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

/// Every option a seat is shown is told in English with the text
/// setting, every decision taken too (a pass where it ends a phase), and
/// no line falls back to an unnamed card or minion: a name the client
/// was never shown is a field missing.
#[test]
fn every_option_and_choice_is_told() {
    let seeds: u32 = std::env::var("HOST_SEEDS").map_or(40, |s| s.parse().unwrap());
    let (mut options, mut decided, mut prompts) = (0, 0, 0);
    let mut unnamed: std::collections::BTreeMap<String, (usize, String)> = Default::default();
    for seed in from()..=seeds {
        let settings = Settings {
            opportunities: seed % 3 != 0,
            views: Views::Decisions,
            text: true,
        };
        let mut g = game(seed, settings);
        let human = g.human();
        let mut rng = Rng(seed as u64 * 11 + 3);
        let mut outs = g.handle(
            human,
            cmd(Command::Join {
                token: None,
                after: None,
            }),
        );
        let mut check = |what: String, text: &str| {
            // What the table may not know (another seat's library search,
            // a card put under the library or drawn from a store) and
            // words that name no card ("equip from a minion").
            let hidden = ["Bottom in", "DrawStored in", "in LibrarySearch"];
            if hidden.iter().any(|h| what.contains(h)) {
                return;
            }
            let text = text
                .replace("from a minion", "")
                .replace("a card requiring", "")
                .replace("with a card in your hand", "")
                .replace("a minion of", "")
                .replace("bottom a card", "")
                .replace("Discard a card from", "");
            for bare in ["a card", "a minion"] {
                if text.contains(bare) {
                    let e = unnamed
                        .entry(what.clone())
                        .or_insert((0, format!("seed {seed}: {text}")));
                    e.0 += 1;
                }
            }
        };
        loop {
            for o in &outs {
                if let Message::Events { events, .. } = &o.message {
                    for e in events {
                        if let EventKind::Decided { choice, window, .. } = &e.kind {
                            let t = e.text.as_deref();
                            if choice.key.kind != OptionKind::Pass {
                                assert!(
                                    t.is_some_and(|t| !t.is_empty()),
                                    "seed {seed}: a decision untold: {e:?}"
                                );
                            }
                            if let Some(t) = t {
                                decided += 1;
                                check(format!("decided {:?} in {window:?}", choice.key.kind), t);
                            }
                        } else if let Some(t) = &e.text {
                            check(format!("{:?}", std::mem::discriminant(&e.kind)), t);
                        }
                    }
                }
            }
            if g.is_over() {
                break;
            }
            let d = outs
                .iter()
                .rev()
                .find_map(|o| match &o.message {
                    Message::Decision(d) => Some(d.clone()),
                    _ => None,
                })
                .unwrap();
            if let Some(p) = &d.prompt {
                prompts += 1;
                check(format!("prompt in {:?} for {:?}", d.window, d.purpose), p);
            }
            for o in &d.options {
                let t = o.text.as_deref();
                assert!(
                    t.is_some_and(|t| !t.is_empty()),
                    "seed {seed}: an option untold in {:?} for {:?}: {o:?}",
                    d.window,
                    d.purpose
                );
                options += 1;
                check(format!("{:?} in {:?}", o.key.kind, d.window), t.unwrap());
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
    let mut unnamed: Vec<_> = unnamed.into_iter().collect();
    unnamed.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    for (what, (n, eg)) in &unnamed {
        eprintln!("unnamed: {n:6} {what} (e.g. {eg:?})");
    }
    eprintln!("{seeds} games: {options} options, {decided} decisions, {prompts} prompts told");
    assert!(
        unnamed.is_empty(),
        "{} kinds of lines name nothing",
        unnamed.len()
    );
}
