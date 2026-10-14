//! A game's lifecycle: who may say what, numbering and resending, the
//! bots' run between the human's decisions, the end, the record.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::panic::{catch_unwind, AssertUnwindSafe};

use zvtes::agent::{Agent, RandomLegal};
use zvtes::bots::{Scripted, Style};
use zvtes::combat::CombatNote;
use zvtes::decision::{Choice, DecisionRequest, Zone};
use zvtes::library::LIBRARY;
use zvtes::referendum::Recipient as R;
use zvtes_api::event::{Event, EventKind};
use zvtes_api::ids::{OptionKind, Purpose, Window};
use zvtes_api::message::{AllocationError, EndReason, Error, Joined, Over, SeatInfo, SpecError};
use zvtes_api::option::{Allocation, ChoiceKey, OptionView, Recipient, VoteTotals};
use zvtes_api::record::{BotRecord, By, Entry, GameRecord, GameResult};
use zvtes_api::view::Target;
use zvtes_api::{Command, Decision, GameSpec, In, Message, Out, Player, Seat, Settings, TableView};

use crate::describe::{self, EngineNames, Names};
use crate::narrate::Fact;
use crate::numbering::Decks;
use crate::session::{searched, Next, Refused, Session, Taken};
use crate::view::Viewer;

/// The built-in scripted bot profiles.
pub const BUILT_IN: [&str; 3] = ["aggro_bleed", "wall", "random"];

/// An event to tell, or where a decision was taken among them.
enum Told {
    Event(Box<Event>),
    Decided(u64),
}

/// A decision taken and not told yet: what its line needs (the window,
/// what it was for, every option offered).
struct Asked {
    window: Window,
    purpose: Option<Purpose>,
    about: Option<zvtes_api::message::About>,
    seat: Seat,
    offered: Vec<OptionView>,
}

/// Messages kept per seat for a rejoin (`join {after}`).
const KEPT: usize = 4096;

/// Engine steps one command may run (bots included) before the game is
/// ended as an engine error: a loop is a bug, not a long game.
const BUDGET: usize = 200_000;

/// Milliseconds, injected so tests and replays are deterministic.
pub trait Clock {
    fn now(&self) -> u64;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64)
    }
}

/// The host's bots beyond the built-in ones: an agent per profile.
pub trait Bots {
    /// The profiles offered, built-in ones aside.
    fn profiles(&self) -> Vec<String>;
    /// The agent for `profile` at `seed`, and how to record it; `None`: no
    /// such profile.
    fn make(&self, profile: &str, seed: u32) -> Option<(Box<dyn Agent>, BotRecord)>;
}

pub struct Game {
    id: String,
    session: Session,
    human: Seat,
    names: Vec<String>,
    /// The human seat's view: its copy numbering.
    viewer: Viewer,
    decks: Decks,
    settings: Settings,
    seq: u64,
    sent: VecDeque<Out>,
    pending: Option<Decision>,
    started: bool,
    over: Option<Over>,
    last_view: Option<TableView>,
    /// What happened since the messages last told it, named as the view
    /// that took it in numbered it; the decisions taken among it marked.
    backlog: VecDeque<Told>,
    /// The viewer's numbering at each decision it was asked, by number:
    /// an undo restores it, the view asked again the view first sent.
    numberings: BTreeMap<u64, crate::numbering::Numbering>,
    /// The decisions taken and not told yet, by number (`text` only).
    asked: HashMap<u64, Asked>,
    record: GameRecord,
    clock: Box<dyn Clock>,
    t0: u64,
    /// The names the human seat's client knows, from the messages it was
    /// sent: what the English renderer reads.
    known: zvtes_text::Names<'static>,
}

impl Game {
    /// A game from its spec: one human seat (version 1), bots by profile.
    pub fn new(
        id: String,
        mut spec: GameSpec,
        bots: &dyn Bots,
        clock: Box<dyn Clock>,
    ) -> Result<Game, SpecError> {
        let n = spec.seats.len();
        if !(2..=6).contains(&n) {
            return Err(SpecError::Seats { n: n as u8 });
        }
        let humans: Vec<usize> = (0..n)
            .filter(|&i| matches!(spec.seats[i].player, Player::Human { .. }))
            .collect();
        if humans.len() != 1 {
            return Err(SpecError::Humans {
                n: humans.len() as u8,
            });
        }
        let human = humans[0] as Seat;
        let t0 = clock.now();
        let seed = *spec.seed.get_or_insert(t0 as u32 ^ (t0 >> 32) as u32);
        let settings = match &spec.seats[human as usize].player {
            Player::Human { settings } => *settings,
            Player::Bot { .. } => unreachable!(),
        };
        let mut agents: Vec<Option<Box<dyn Agent>>> = Vec::new();
        let mut records = Vec::new();
        for (seat, s) in spec.seats.iter().enumerate() {
            let Player::Bot { profile } = &s.player else {
                agents.push(None);
                continue;
            };
            let bot_seed = seed.wrapping_mul(0x9e37_79b9).wrapping_add(seat as u32);
            let (agent, mut r): (Box<dyn Agent>, BotRecord) = match profile.as_str() {
                "aggro_bleed" | "wall" | "random" => (
                    match profile.as_str() {
                        "aggro_bleed" => {
                            Box::new(Scripted::new(Style::AggroBleed, bot_seed as u64))
                        }
                        "wall" => Box::new(Scripted::new(Style::Wall, bot_seed as u64)),
                        _ => Box::new(RandomLegal::new(bot_seed as u64)),
                    },
                    BotRecord {
                        seat: 0,
                        profile: profile.clone(),
                        kind: profile.clone(),
                        weights: None,
                        temperature: None,
                        seed: bot_seed,
                    },
                ),
                _ => bots
                    .make(profile, bot_seed)
                    .ok_or_else(|| SpecError::Profile {
                        seat: seat as Seat,
                        profile: profile.clone(),
                    })?,
            };
            r.seat = seat as Seat;
            records.push(r);
            agents.push(Some(agent));
        }
        let decks = spec
            .seats
            .iter()
            .enumerate()
            .map(|(seat, s)| crate::deck::resolve(seat as Seat, &s.deck))
            .collect::<Result<Vec<_>, _>>()?;
        let names = spec.seats.iter().map(|s| s.name.clone()).collect();
        let mut session = Session::new(seed as u64, spec.time_limit, decks, agents);
        session.set_opportunities(settings.opportunities);
        let mut recorded = spec.clone();
        for (i, s) in recorded.seats.iter_mut().enumerate() {
            if i == human as usize {
                s.name = "human".into();
            }
        }
        let decks = Decks::dealt(session.engine());
        Ok(Game {
            id,
            session,
            human,
            names,
            viewer: Viewer::new(Some(zvtes::ids::SeatId(human))),
            decks,
            settings,
            seq: 0,
            sent: VecDeque::new(),
            pending: None,
            started: false,
            over: None,
            last_view: None,
            backlog: VecDeque::new(),
            numberings: Default::default(),
            asked: HashMap::new(),
            known: zvtes_text::Names::new(crate::cards::cards()),
            record: GameRecord {
                protocol: zvtes_api::PROTOCOL_VERSION.into(),
                engine: crate::ENGINE.into(),
                spec: recorded,
                bots: records,
                started: t0,
                ended: None,
                entries: vec![],
                result: None,
            },
            clock,
            t0,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn human(&self) -> Seat {
        self.human
    }

    pub fn is_over(&self) -> bool {
        self.over.is_some()
    }

    pub fn record(&self) -> &GameRecord {
        &self.record
    }

    /// A command as JSON text (`In`), answered as a JSON array of `Out`;
    /// text that is not a command is answered by a `malformed` error.
    pub fn handle_json(&mut self, seat: Seat, text: &str) -> String {
        let outs = match serde_json::from_str::<In>(text) {
            Ok(input) => self.handle(seat, input),
            Err(e) => self.emit(
                seat,
                None,
                vec![Message::Error(Error::Malformed {
                    reason: e.to_string(),
                })],
            ),
        };
        serde_json::to_string(&outs).expect("messages serialize")
    }

    /// The seat's command, answered: the messages for it, in order.
    pub fn handle(&mut self, seat: Seat, input: In) -> Vec<Out> {
        if let Command::Join {
            after: Some(after), ..
        } = input.command
        {
            if seat == self.human {
                if let Some(rest) = self.resend(after) {
                    return rest;
                }
            }
        }
        let messages = if seat != self.human {
            vec![Message::Error(Error::NotYourTurn)]
        } else {
            match catch_unwind(AssertUnwindSafe(|| self.command(input.command.clone()))) {
                Ok(m) => m,
                Err(panic) => self.fail(panic_text(&panic)),
            }
        };
        self.emit(seat, input.id, messages)
    }

    /// Messages numbered, kept for a rejoin; refusals recorded.
    fn emit(&mut self, seat: Seat, id: Option<u64>, messages: Vec<Message>) -> Vec<Out> {
        let at = self.at();
        let mut out = Vec::with_capacity(messages.len());
        for mut m in messages {
            self.tell(&mut m);
            let reply_to = match &m {
                Message::Error(e) => {
                    self.record.entries.push(Entry::Refused {
                        seat,
                        at,
                        error: e.clone(),
                    });
                    id
                }
                _ => None,
            };
            self.seq += 1;
            let o = Out {
                seq: self.seq,
                reply_to,
                message: m,
            };
            self.sent.push_back(o.clone());
            if self.sent.len() > KEPT {
                self.sent.pop_front();
            }
            out.push(o);
        }
        out
    }

    /// The client's names brought up to date with a message, and its
    /// English told from the data where the text setting asks it.
    fn tell(&mut self, m: &mut Message) {
        let text = self.settings.text;
        let known = &mut self.known;
        match m {
            Message::Joined(j) => known.see(&j.view),
            Message::Events { events, view } => {
                for e in events {
                    known.hear(&e.kind);
                    if !text {
                        continue;
                    }
                    e.text = match &e.kind {
                        // A decision taken: its option's line, a pass
                        // only where it ends a phase.
                        EventKind::Decided {
                            decision, choice, ..
                        } => self.asked.remove(decision).and_then(|a| {
                            let pass = choice.key.kind == OptionKind::Pass;
                            let ends = matches!(
                                a.window,
                                Window::MasterPhase
                                    | Window::MinionPhase
                                    | Window::InfluenceTransfers
                                    | Window::DiscardPhase
                            );
                            (!pass || ends).then(|| {
                                zvtes_text::option(
                                    choice,
                                    a.window,
                                    a.purpose,
                                    a.about.as_ref(),
                                    a.seat,
                                    &a.offered,
                                    known,
                                )
                            })?
                        }),
                        EventKind::AutoPassed { decision, .. } => {
                            self.asked.remove(decision);
                            None
                        }
                        k if e.text.is_some() => zvtes_text::event(k, known),
                        _ => None,
                    };
                }
                if let Some(v) = view {
                    known.see(v);
                }
            }
            Message::Decision(d) => {
                known.see(&d.view);
                if text {
                    d.prompt = zvtes_text::prompt(d, known);
                    let offered = d.options.clone();
                    for o in &mut d.options {
                        o.text = zvtes_text::option(
                            o,
                            d.window,
                            d.purpose,
                            d.about.as_ref(),
                            d.seat,
                            &offered,
                            known,
                        );
                    }
                }
            }
            Message::Over(o) => known.see(&o.view),
            Message::Error(_) => {}
        }
    }

    /// The messages after `after`, if all are still kept.
    fn resend(&self, after: u64) -> Option<Vec<Out>> {
        if after > self.seq {
            return None;
        }
        let first = self.sent.front().map_or(self.seq + 1, |o| o.seq);
        (after + 1 >= first).then(|| {
            self.sent
                .iter()
                .filter(|o| o.seq > after)
                .cloned()
                .collect()
        })
    }

    fn at(&self) -> u64 {
        self.clock.now().saturating_sub(self.t0)
    }

    fn command(&mut self, c: Command) -> Vec<Message> {
        if let Command::Join { .. } = c {
            return self.join();
        }
        if self.over.is_some() {
            return vec![Message::Error(Error::Over)];
        }
        if !self.started {
            return vec![Message::Error(Error::NotJoined)];
        }
        let Some(pending) = self.pending.as_ref().map(|d| d.decision) else {
            return vec![Message::Error(Error::NotYourTurn)];
        };
        match c {
            Command::Join { .. } => unreachable!(),
            Command::Answer { decision, option } => {
                if let Err(e) = self.session.check(decision, option as usize) {
                    return vec![Message::Error(refused(e, option))];
                }
                let taken = self.session.human();
                let mut m = self.take(option as usize, taken, By::Human);
                m.extend(self.advance());
                m
            }
            Command::Allocate { decision, amounts } => {
                match self.session.allocate(decision, &amounts) {
                    Ok(picks) => {
                        let mut m = Vec::new();
                        for (step, i) in picks.into_iter().enumerate() {
                            let taken = if step == 0 {
                                self.session.human()
                            } else {
                                Taken::Order
                            };
                            m.extend(self.take(i, taken, By::Order));
                        }
                        m.extend(self.advance());
                        m
                    }
                    Err(e) => vec![Message::Error(refused(e, 0))],
                }
            }
            Command::VoteAll { decision, favor } => match self.session.vote_all(decision, favor) {
                Ok(i) => {
                    let taken = self.session.human();
                    let mut m = self.take(i, taken, By::Order);
                    m.extend(self.advance());
                    m
                }
                Err(e) => vec![Message::Error(refused(e, 0))],
            },
            Command::Undo { decision } => {
                if decision != pending {
                    return vec![Message::Error(Error::Stale { expected: pending })];
                }
                match self.session.undo() {
                    Ok(_) => {
                        self.backlog.clear();
                        self.asked.clear();
                        let at = self.session.decision();
                        let _ = self.numberings.split_off(&(at + 1));
                        if let Some(n) = self.numberings.get(&at) {
                            self.viewer.numbering = n.clone();
                        }
                        let mut m = self.advance();
                        let to = self.pending.as_ref().map_or(pending, |d| d.decision);
                        self.record.entries.push(Entry::Undo {
                            seat: self.human,
                            at: self.at(),
                            to,
                        });
                        m.insert(0, self.events(vec![undone(self.human, to)], None));
                        m
                    }
                    Err(e) => vec![Message::Error(refused(e, 0))],
                }
            }
            Command::Settings(patch) => {
                self.settings.apply(patch);
                self.session.set_opportunities(self.settings.opportunities);
                self.record.entries.push(Entry::Settings {
                    seat: self.human,
                    at: self.at(),
                    settings: self.settings,
                });
                self.advance()
            }
            Command::Leave => self.end(EndReason::Abandoned { seat: self.human }, None),
        }
    }

    fn join(&mut self) -> Vec<Message> {
        let mut m = Vec::new();
        let view = self.view();
        m.push(Message::Joined(Joined {
            protocol: zvtes_api::PROTOCOL_VERSION.into(),
            engine: crate::ENGINE.into(),
            game: self.id.clone(),
            seat: self.human,
            seats: self
                .names
                .iter()
                .enumerate()
                .map(|(i, name)| SeatInfo {
                    name: name.clone(),
                    bot: self
                        .record
                        .bots
                        .iter()
                        .find(|b| b.seat as usize == i)
                        .map(|b| b.profile.clone()),
                })
                .collect(),
            settings: self.settings,
            idle_timeout: None,
            view,
        }));
        if let Some(over) = &self.over {
            m.push(Message::Over(over.clone()));
        } else if !self.started {
            self.started = true;
            m.extend(self.advance());
        } else if let Some(d) = &self.pending {
            m.push(Message::Decision(d.clone()));
        }
        m
    }

    /// Option `i` of `req` as the viewer knows it (its copies), what it
    /// does; a decision of another seat's told as the table may know it.
    fn option(&self, req: &DecisionRequest, i: usize) -> OptionView {
        let e = self.session.engine();
        let c = &req.options[i];
        let names = &self.viewer.numbering;
        let mut key = describe::key(e, req, c, names);
        if req.seat.0 != self.human {
            narrated(&mut key, c, req.window);
        }
        OptionView {
            index: i as u32,
            key,
            effects: describe::effects(e, req, c, names),
            abilities: describe::abilities(e, c),
            text: None,
        }
    }

    /// Runs the game to the human's next decision or the end.
    fn advance(&mut self) -> Vec<Message> {
        let mut m = Vec::new();
        self.pending = None;
        for _ in 0..BUDGET {
            match self.session.advance() {
                Next::Ask { shown, undo } => {
                    let view = self.view();
                    let req = self.session.request().expect("asked").clone();
                    let e = self.session.engine();
                    let names = &self.viewer.numbering;
                    let decision = self.session.decision();
                    let d = Decision {
                        decision,
                        seat: req.seat.0,
                        window: crate::ids::window(req.window),
                        source: describe::source(e, &req, names),
                        purpose: e.interrupt().map(|i| crate::ids::purpose(&i)),
                        about: describe::about(e, &req, names),
                        options: shown.iter().map(|&i| self.option(&req, i)).collect(),
                        undo,
                        allocation: self.allocation(&req),
                        votes: self
                            .session
                            .votes(&req)
                            .map(|(favor, against)| VoteTotals { favor, against }),
                        prompt: None,
                        view: view.clone(),
                    };
                    self.last_view = Some(view);
                    self.numberings
                        .insert(decision, self.viewer.numbering.clone());
                    if self.numberings.len() > 256 {
                        self.numberings.pop_first();
                    }
                    self.pending = Some(d.clone());
                    m.extend(self.flush());
                    m.push(Message::Decision(d));
                    return m;
                }
                Next::Take { index, taken } => {
                    let by = match taken {
                        Taken::Bot => By::Bot,
                        Taken::Passed => By::Auto,
                        _ => By::Order,
                    };
                    m.extend(self.take(index, taken, by));
                    if self
                        .last_view
                        .as_ref()
                        .is_some_and(|v| v.seats[self.human as usize].ousted)
                    {
                        m.extend(self.end(EndReason::HumansOusted, None));
                        return m;
                    }
                }
                Next::Over => {
                    let v = self.view();
                    let standing = v.seats.iter().filter(|s| !s.ousted).count();
                    self.last_view = Some(v);
                    let reason = if standing <= 1 {
                        EndReason::LastStanding
                    } else {
                        EndReason::TimeLimit
                    };
                    m.extend(self.end(reason, None));
                    return m;
                }
            }
        }
        m.extend(self.end(EndReason::EngineError, Some("step budget exhausted".into())));
        m
    }

    /// The allocation under way, while the decision asks its first
    /// recipient: answered whole by `allocate`.
    fn allocation(&self, req: &DecisionRequest) -> Option<Allocation> {
        let a = self.session.allocation(req)?;
        let e = self.session.engine();
        Some(Allocation {
            points: a.points,
            recipients: a
                .recipients
                .iter()
                .zip(&a.caps)
                .map(|(&r, &cap)| Recipient {
                    to: match r {
                        R::Seat(s) => Target::Seat { seat: s.0 },
                        R::Card(k) => Target::Card {
                            copy: self
                                .viewer
                                .numbering
                                .copy(*e.state.cards[k.index()].copy)
                                .unwrap_or(u32::MAX),
                        },
                    },
                    cap,
                    effects: vec![],
                })
                .collect(),
            min_nonzero: a.min_nonzero,
        })
    }

    /// The pending decision's option `index` taken (`taken` for undo,
    /// `by` for the record): its record entry, its event.
    fn take(&mut self, index: usize, taken: Taken, by: By) -> Vec<Message> {
        self.session.ready();
        let req = self.session.request().expect("a decision pending").clone();
        let decision = self.session.decision();
        let seat = req.seat.0;
        let window = crate::ids::window(req.window);
        let option = self.option(&req, index);
        let key = describe::key(
            self.session.engine(),
            &req,
            &req.options[index],
            &EngineNames,
        );
        // The options the seat was shown, when its settings hid some (F5).
        let shown = self
            .pending
            .as_ref()
            .filter(|d| {
                by != By::Bot && d.decision == decision && d.options.len() < req.options.len()
            })
            .map(|d| d.options.iter().map(|o| o.index).collect());
        if self.settings.text && by != By::Auto {
            let e = self.session.engine();
            self.asked.insert(
                decision,
                Asked {
                    window,
                    purpose: e.interrupt().map(|i| crate::ids::purpose(&i)),
                    about: describe::about(e, &req, &self.viewer.numbering),
                    seat,
                    offered: (0..req.options.len())
                        .map(|i| self.option(&req, i))
                        .collect(),
                },
            );
        }
        self.session.take(index, taken);
        // The table after it (the numbering follows every step).
        let view = self.view();
        self.record.entries.push(Entry::Decision {
            decision,
            seat,
            at: self.at(),
            option: index as u32,
            choice: key,
            by,
            forced: false,
            shown,
        });
        let event = if by == By::Auto {
            EventKind::AutoPassed {
                decision,
                seat,
                window,
            }
        } else {
            EventKind::Decided {
                decision,
                seat,
                window,
                choice: option,
            }
        };
        let steps = self.settings.views == zvtes_api::spec::Views::Steps;
        self.last_view = Some(view.clone());
        let decided = Event {
            kind: event,
            text: None,
        };
        let events = self.told_with(decision, decided);
        vec![self.events(events, steps.then_some(view))]
    }

    fn events(&self, events: Vec<Event>, view: Option<TableView>) -> Message {
        Message::Events { events, view }
    }

    /// The events up to the decision taken (`decided` in its place) and
    /// what it did, up to the next decision taken.
    fn told_with(&mut self, decision: u64, decided: Event) -> Vec<Event> {
        let mut events = Vec::new();
        let mut decided = Some(decided);
        while let Some(t) = self.backlog.pop_front() {
            match t {
                Told::Decided(n) if n == decision && decided.is_some() => {
                    events.extend(decided.take());
                }
                // The next decision's: what follows is its.
                Told::Decided(n) if decided.is_none() => {
                    self.backlog.push_front(Told::Decided(n));
                    break;
                }
                Told::Decided(_) => {}
                Told::Event(e) => events.push(*e),
            }
        }
        events.extend(decided);
        events
    }

    /// Everything not told yet, as one message.
    fn flush(&mut self) -> Option<Message> {
        let events: Vec<Event> = self
            .backlog
            .drain(..)
            .filter_map(|t| match t {
                Told::Event(e) => Some(*e),
                Told::Decided(_) => None,
            })
            .collect();
        (!events.is_empty()).then(|| self.events(events, None))
    }

    /// The table now, the numbering brought up to date; what happened
    /// since the last view named as it numbers it.
    fn view(&mut self) -> TableView {
        let e = self.session.engine();
        let mut v = self.viewer.view(e, &self.decks, &self.names);
        v.viewer = Some(self.human);
        self.searched_view(&mut v);
        self.struck_view(&mut v);
        let told = self.session.take_told();
        let e = self.session.engine();
        for t in told {
            let told = match self.viewer.event(e, &self.decks, t.fact) {
                Some(kind) => Told::Event(Box::new(Event {
                    kind,
                    // Filled by `tell` where a log tells it.
                    text: (t.told && self.settings.text).then(String::new),
                })),
                None => match t.fact {
                    Fact::Decided(n) => Told::Decided(n),
                    _ => continue,
                },
            };
            self.backlog.push_back(told);
        }
        v
    }

    /// The library cards the human seat may take, shown to it while it
    /// searches (owner: "reveal only the matching cards").
    fn searched_view(&self, v: &mut TableView) {
        let Some(req) = self.session.request() else {
            return;
        };
        let found = searched(req);
        if req.seat.0 != self.human || found.is_empty() {
            return;
        }
        let library = &self.session.engine().state.seats[self.human as usize].library;
        if let Some(s) = v.seats.iter_mut().find(|s| s.seat == self.human) {
            for (c, &card) in s.library.iter_mut().zip(library) {
                if found.contains(&card) {
                    c.id = Some(LIBRARY[card as usize].id);
                }
            }
        }
    }

    /// A strike spent as it resolved, kept in the combat's view until the
    /// next pair, with the damage it dealt.
    fn struck_view(&self, v: &mut TableView) {
        let e = self.session.engine();
        let (
            Some(c),
            Some(cv),
            Some(CombatNote::Strikes {
                minions,
                round,
                pair,
                strikes,
                damage,
                ..
            }),
        ) = (e.combat(), v.combat.as_mut(), self.session.struck())
        else {
            return;
        };
        if c.round != round || c.pair != pair {
            return;
        }
        for (side, view) in c.sides.iter().zip(&mut cv.sides) {
            let Some(j) = minions.iter().position(|&m| m == side.minion) else {
                continue;
            };
            if let (true, None, Some(k)) = (side.chosen, side.strike, strikes[j]) {
                let mut s = crate::view::strike(k, None);
                s.resolved = Some(damage[j]);
                view.strike = Some(s);
            }
        }
    }

    fn end(&mut self, reason: EndReason, failure: Option<String>) -> Vec<Message> {
        let view = match failure {
            Some(_) => self.last_view.clone(),
            None => catch_unwind(AssertUnwindSafe(|| self.view()))
                .ok()
                .or(self.last_view.clone()),
        };
        let view = view.unwrap_or_else(|| empty_view(self.human));
        let told = self.flush();
        let vp_halves: Vec<u32> = view.seats.iter().map(|s| s.vp_halves).collect();
        let best = vp_halves.iter().copied().max().unwrap_or(0);
        let winner = (best >= 4 && vp_halves.iter().filter(|&&v| v == best).count() == 1)
            .then(|| vp_halves.iter().position(|&v| v == best).unwrap() as Seat);
        let over = Over {
            vp_halves: vp_halves.clone(),
            winner,
            reason,
            view,
        };
        self.pending = None;
        self.over = Some(over.clone());
        self.record.ended = Some(self.clock.now());
        self.record.result = Some(GameResult {
            vp_halves,
            winner,
            reason,
            failure,
        });
        told.into_iter().chain([Message::Over(over)]).collect()
    }

    fn fail(&mut self, why: String) -> Vec<Message> {
        self.end(EndReason::EngineError, Some(why))
    }
}

/// Another seat's choice as the table may know it: the card a library
/// search took, a card put under the library, a card drawn from a store,
/// unnamed.
fn narrated(key: &mut ChoiceKey, c: &Choice, window: zvtes::decision::WindowLabel) {
    let hidden = match *c {
        // A library search's: an ash heap's pick is public.
        Choice::FindCard(_) => window == zvtes::decision::WindowLabel::LibrarySearch,
        Choice::Bottom(_) | Choice::DrawStored { .. } => true,
        Choice::TakeFrom { zone, .. } => zone != Zone::AshHeap,
        _ => false,
    };
    if hidden {
        if let Some(card) = key.card.as_mut() {
            card.id = None;
        }
    }
}

fn undone(seat: Seat, to: u64) -> Event {
    Event {
        kind: EventKind::Undone { seat, to },
        text: None,
    }
}

fn empty_view(viewer: Seat) -> TableView {
    TableView {
        viewer: Some(viewer),
        decision: 0,
        turn: 0,
        active: 0,
        phase: zvtes_api::ids::Phase::Unknown,
        seats: vec![],
        action: None,
        combat: None,
        referendum: None,
    }
}

fn panic_text(p: &Box<dyn std::any::Any + Send>) -> String {
    p.downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| p.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "engine panic".into())
}

fn refused(e: Refused, option: u32) -> Error {
    match e {
        Refused::Stale { expected } => Error::Stale { expected },
        Refused::NotYours => Error::NotYourTurn,
        Refused::Option => Error::Option { index: option },
        Refused::Over => Error::Over,
        Refused::NoUndo => Error::NoUndo,
        Refused::NoAllocation => Error::Allocation {
            reason: AllocationError::None,
        },
        Refused::Allocation => Error::Allocation {
            reason: AllocationError::Total,
        },
        Refused::NoVotes => Error::NoVotes,
    }
}

#[cfg(test)]
mod tests {
    //! The copy rules (docs/protocol.md §3) at every decision of random
    //! games, and records replaying by key as by index.

    use std::collections::{HashMap, HashSet};

    use zvtes_api::view::{Card, TableView};
    use zvtes_api::{Command, In, Message, Player, SeatSpec, Settings};

    use super::*;

    struct NoBots;

    impl Bots for NoBots {
        fn profiles(&self) -> Vec<String> {
            vec![]
        }
        fn make(&self, _: &str, _: u32) -> Option<(Box<dyn Agent>, BotRecord)> {
            None
        }
    }

    fn game(seed: u32) -> Game {
        let decks = crate::catalog();
        let styles = ["aggro_bleed", "wall", "random"];
        let spec = GameSpec {
            seed: Some(seed),
            time_limit: Some(40),
            client: None,
            seats: (0..4)
                .map(|i| SeatSpec {
                    name: format!("s{i}"),
                    deck: decks[(seed as usize + i * 5) % decks.len()].1.clone(),
                    player: if i == seed as usize % 4 {
                        Player::Human {
                            settings: Settings {
                                views: zvtes_api::spec::Views::Steps,
                                ..Settings::default()
                            },
                        }
                    } else {
                        Player::Bot {
                            profile: styles[(seed as usize + i) % 3].into(),
                        }
                    },
                })
                .collect(),
        };
        Game::new(format!("t{seed}"), spec, &NoBots, Box::new(SystemClock)).unwrap()
    }

    /// Every card shown, with whether its place is hidden to the viewer.
    fn cards(v: &TableView, viewer: u8) -> Vec<(Card, bool)> {
        let mut out = Vec::new();
        for s in &v.seats {
            for c in s.crypt.iter().chain(&s.library) {
                out.push((*c, true));
            }
            // Another seat's hand is hidden, unless a card reveals it.
            for c in &s.hand {
                out.push((*c, s.seat != viewer && c.id.is_none()));
            }
            for c in s.ash_heap.iter().chain(&s.removed).chain(&s.limbo) {
                out.push((*c, false));
            }
            for u in &s.uncontrolled {
                out.push((u.card, false));
                out.extend(u.cards.iter().map(|c| (*c, false)));
            }
            for m in s.ready.iter().chain(&s.torpor) {
                out.push((m.card, false));
                for c in &m.cards {
                    out.push((c.card, false));
                    out.extend(c.stored.iter().map(|x| (*x, false)));
                }
            }
            for c in &s.in_play {
                out.push((c.card, false));
                out.extend(c.stored.iter().map(|x| (*x, false)));
            }
        }
        out
    }

    #[test]
    fn the_views_keep_the_copy_rules() {
        let seeds: u32 = std::env::var("HOST_SEEDS").map_or(16, |s| s.parse().unwrap());
        for seed in 1..=seeds {
            let mut g = game(seed);
            let human = g.human();
            let mut outs = g.handle(
                human,
                In {
                    id: None,
                    command: Command::Join {
                        token: None,
                        after: None,
                    },
                },
            );
            // Per wire copy: its card; copies once out of sight.
            let mut faces: HashMap<u32, (Option<u32>, bool)> = HashMap::new();
            let mut gone: HashSet<u32> = HashSet::new();
            let mut last: HashSet<u32> = HashSet::new();
            let mut n = 0u64;
            for step in 0..3000 {
                let views: Vec<TableView> = outs
                    .iter()
                    .filter_map(|o| match &o.message {
                        Message::Decision(d) => Some(d.view.clone()),
                        Message::Events { view: Some(v), .. } => Some(v.clone()),
                        Message::Joined(j) => Some(j.view.clone()),
                        _ => None,
                    })
                    .collect();
                for v in &views {
                    let mut now = HashSet::new();
                    for (c, hidden) in cards(v, human) {
                        if hidden {
                            assert!(
                                c.copy.is_none(),
                                "seed {seed}: a hidden card numbered {c:?}"
                            );
                        }
                        let Some(w) = c.copy else { continue };
                        assert!(
                            c.nth.is_some_and(|n| n > 0),
                            "seed {seed}: {c:?} without its label"
                        );
                        assert!(
                            !gone.contains(&w),
                            "seed {seed}: copy {w} back after leaving sight"
                        );
                        let face = (c.id, c.crypt);
                        assert_eq!(
                            *faces.entry(w).or_insert(face),
                            face,
                            "seed {seed}: copy {w} changed card"
                        );
                        assert!(now.insert(w), "seed {seed}: copy {w} twice in a view");
                    }
                    gone.extend(last.difference(&now));
                    last = now;
                }
                if g.is_over() {
                    break;
                }
                let Some(d) = outs.iter().rev().find_map(|o| match &o.message {
                    Message::Decision(d) => Some(d.clone()),
                    _ => None,
                }) else {
                    break;
                };
                n += 1;
                let pick = d.options[(seed as usize * 7 + step * 13) % d.options.len()].index;
                outs = g.handle(
                    human,
                    In {
                        id: None,
                        command: Command::Answer {
                            decision: d.decision,
                            option: pick,
                        },
                    },
                );
            }
            assert!(n > 0);
        }
    }

    #[test]
    fn a_record_replays_by_key_as_by_index() {
        use crate::describe::{key, EngineNames};
        use zvtes::game::Step;
        for seed in 1..=24u32 {
            let mut g = game(seed);
            let human = g.human();
            let dealt = g.session.engine().clone();
            let mut outs = g.handle(
                human,
                In {
                    id: None,
                    command: Command::Join {
                        token: None,
                        after: None,
                    },
                },
            );
            for step in 0..3000usize {
                let Some(d) = outs.iter().rev().find_map(|o| match &o.message {
                    Message::Decision(d) => Some(d.clone()),
                    _ => None,
                }) else {
                    break;
                };
                let pick = d.options[(seed as usize * 11 + step * 7) % d.options.len()].index;
                outs = g.handle(
                    human,
                    In {
                        id: None,
                        command: Command::Answer {
                            decision: d.decision,
                            option: pick,
                        },
                    },
                );
            }
            let mut e = dealt;
            let mut n = 0;
            for entry in &g.record().entries {
                let Entry::Decision {
                    decision,
                    option,
                    choice,
                    by,
                    ..
                } = entry
                else {
                    continue;
                };
                let req = match e.run() {
                    Step::Decision(r) => r,
                    Step::Over(_) => {
                        panic!("seed {seed}: the replay ended before decision {decision}")
                    }
                };
                assert_eq!(e.log.len() as u64, *decision, "seed {seed}");
                let matching: Vec<usize> = (0..req.options.len())
                    .filter(|&i| key(&e, &req, &req.options[i], &EngineNames) == *choice)
                    .collect();
                assert!(
                    matching.contains(&(*option as usize)),
                    "seed {seed}, decision {decision} ({by:?}): key {choice:?} matches {matching:?}, recorded {option}"
                );
                e.choose(*option as usize);
                n += 1;
            }
            assert!(n > 0, "seed {seed}");
        }
    }

    /// Opportunities turned off while a decision is shown: it stays the
    /// seat's, as shown, never passed by itself (the client answering it
    /// would be refused as stale); later decisions follow the setting.
    #[test]
    fn a_settings_change_never_takes_the_decision_shown() {
        use zvtes_api::event::EventKind;
        use zvtes_api::spec::SettingsPatch;
        let mut kept = 0;
        for seed in 1..=24u32 {
            let mut g = game(seed);
            let human = g.human();
            let cmd = |command| In { id: None, command };
            let mut outs = g.handle(
                human,
                cmd(Command::Join {
                    token: None,
                    after: None,
                }),
            );
            let mut on = true;
            for step in 0..3000usize {
                let Some(d) = outs.iter().rev().find_map(|o| match &o.message {
                    Message::Decision(d) => Some(d.clone()),
                    _ => None,
                }) else {
                    break;
                };
                // Off at most decisions, on again now and then
                if on != (step % 4 == 3) {
                    on = step % 4 == 3;
                    outs = g.handle(
                        human,
                        cmd(Command::Settings(SettingsPatch {
                            opportunities: Some(on),
                            ..Default::default()
                        })),
                    );
                    for o in &outs {
                        if let Message::Events { events, .. } = &o.message {
                            for e in events {
                                if let EventKind::Decided { decision, .. }
                                | EventKind::AutoPassed { decision, .. } = &e.kind
                                {
                                    assert_ne!(
                                        *decision, d.decision,
                                        "seed {seed}: decision {} taken by a settings change",
                                        d.decision
                                    );
                                }
                            }
                        }
                    }
                    let again = outs.iter().rev().find_map(|o| match &o.message {
                        Message::Decision(d) => Some(d.clone()),
                        _ => None,
                    });
                    let again = again.expect("the decision shown again");
                    assert_eq!(again.decision, d.decision, "seed {seed}");
                    let indices = |d: &zvtes_api::Decision| -> Vec<u32> {
                        d.options.iter().map(|o| o.index).collect()
                    };
                    assert_eq!(indices(&again), indices(&d), "seed {seed}: options changed");
                    let req = g.session.request().expect("pending").clone();
                    let shown = g.session.shown(&req);
                    if !on && shown.len() == 1 && d.options.len() > 1 {
                        kept += 1;
                    }
                }
                let pick = d.options[(seed as usize * 11 + step * 7) % d.options.len()].index;
                outs = g.handle(
                    human,
                    cmd(Command::Answer {
                        decision: d.decision,
                        option: pick,
                    }),
                );
                assert!(
                    !outs.iter().any(|o| matches!(o.message, Message::Error(_))),
                    "seed {seed}: answer to {} refused",
                    d.decision
                );
            }
        }
        // Decisions an earlier server passed by themselves once shown
        assert!(kept > 0, "no decision left with a lone pass by the setting");
        eprintln!("{kept} decisions shown kept through the opportunities turned off");
    }
}
