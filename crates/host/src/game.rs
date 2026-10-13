//! A game's lifecycle: who may say what, numbering and resending, the
//! bots' run between the human's decisions, the end, the record.

use std::collections::{HashMap, VecDeque};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};

use zvtes::agent::{Agent, RandomLegal};
use zvtes::bots::{Scripted, Style};
use zvtes::decision::DecisionRequest;
use zvtes::game::Engine;
use zvtes_api::event::{Event, EventKind};
use zvtes_api::message::{AllocationError, EndReason, Error, Joined, Over, SeatInfo, SpecError};
use zvtes_api::option::ChoiceKey;
use zvtes_api::record::{BotRecord, By, Entry, GameRecord, GameResult};
use zvtes_api::{Command, Decision, GameSpec, In, Message, Out, Player, Seat, Settings, TableView};
use zvtes_protocol as old;

use crate::describe::{self, EngineNames};
use crate::legacy::{self, Ctx};
use crate::numbering::Decks;
use crate::view::Viewer;

/// The built-in scripted bot profiles.
pub const BUILT_IN: [&str; 3] = ["aggro_bleed", "wall", "random"];

/// The keys of the bots' decisions as they take them: by decision
/// number, the option and its key by the engine's copies.
type Keys = Arc<Mutex<HashMap<u64, (u32, ChoiceKey)>>>;

/// A bot whose every decision is keyed as it is taken (the record names
/// a bot's choice by what it saw, hidden cards included).
struct Recorded {
    agent: Box<dyn Agent>,
    keys: Keys,
}

impl Agent for Recorded {
    fn decide(&mut self, e: &Engine, req: &DecisionRequest) -> usize {
        let i = self.agent.decide(e, req);
        let key = describe::key(e, req, &req.options[i], &EngineNames);
        self.keys
            .lock()
            .unwrap()
            .insert(e.log.len() as u64, (i as u32, key));
        i
    }
}

/// An event to tell, or where a decision was taken among them.
enum Told {
    Event(Box<Event>),
    Decided(u64),
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
    session: old::Session,
    human: Seat,
    names: Vec<String>,
    /// The human seat's view: its copy numbering.
    viewer: Viewer,
    decks: Decks,
    keys: Keys,
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
    record: GameRecord,
    clock: Box<dyn Clock>,
    t0: u64,
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
        let keys: Keys = Arc::default();
        let mut agents = Vec::new();
        let mut installed = Vec::new();
        let mut records = Vec::new();
        for (seat, s) in spec.seats.iter().enumerate() {
            let Player::Bot { profile } = &s.player else {
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
            installed.push(profile.clone());
            agents.push((
                seat,
                Box::new(Recorded {
                    agent,
                    keys: keys.clone(),
                }) as Box<dyn Agent>,
            ));
        }
        let names = spec.seats.iter().map(|s| s.name.clone()).collect();
        let installed: Vec<&str> = installed.iter().map(String::as_str).collect();
        let mut session =
            old::Session::with_styles(legacy_spec(&spec, seed)?, &installed).map_err(spec_error)?;
        for (seat, agent) in agents {
            session.set_agent(seat, agent);
        }
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
            keys,
            settings,
            seq: 0,
            sent: VecDeque::new(),
            pending: None,
            started: false,
            over: None,
            last_view: None,
            backlog: VecDeque::new(),
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
        for m in messages {
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
        if let Some(over) = &self.over {
            let _ = over;
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
                let key = self.session.request().and_then(|r| {
                    r.options
                        .get(option as usize)
                        .map(|c| describe::key(self.session.engine(), r, c, &EngineNames))
                });
                match self.session.answer(decision, option as usize) {
                    Ok(out) => {
                        let mut m = self.step(out, By::Human, key);
                        m.extend(self.advance());
                        m
                    }
                    Err(e) => vec![Message::Error(answer_error(e, option))],
                }
            }
            Command::Allocate { decision, amounts } => {
                match self.session.allocate(decision, &amounts) {
                    Ok(outs) => {
                        let mut m = Vec::new();
                        for o in outs {
                            m.extend(self.step(o, By::Order, None));
                        }
                        m.extend(self.advance());
                        m
                    }
                    Err(e) => vec![Message::Error(answer_error(e, 0))],
                }
            }
            Command::VoteAll { decision, favor } => match self.session.vote_all(decision, favor) {
                Ok(out) => {
                    let mut m = self.step(out, By::Order, None);
                    m.extend(self.advance());
                    m
                }
                Err(e) => vec![Message::Error(answer_error(e, 0))],
            },
            Command::Undo { decision } => {
                if decision != pending {
                    return vec![Message::Error(Error::Stale { expected: pending })];
                }
                match self.session.undo() {
                    Ok(()) => {
                        self.backlog.clear();
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
                    Err(e) => vec![Message::Error(answer_error(e, 0))],
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

    /// Runs the game to the human's next decision or the end.
    fn advance(&mut self) -> Vec<Message> {
        let mut m = Vec::new();
        self.pending = None;
        for _ in 0..BUDGET {
            match self.session.advance() {
                old::Out::Decision {
                    decision,
                    seat,
                    options,
                    prompt,
                    undo,
                    allocation,
                    votes,
                    ..
                } => {
                    let api_view = self.view();
                    let ctx = Ctx::new(&self.viewer, self.session.engine(), self.settings.text);
                    let req = self.session.request().cloned();
                    let window = self
                        .session
                        .request()
                        .map_or(zvtes_api::ids::Window::Unknown, |r| {
                            crate::ids::window(r.window)
                        });
                    let e = self.session.engine();
                    let source = req
                        .as_ref()
                        .and_then(|r| describe::source(e, r, &self.viewer.numbering));
                    let purpose = e.interrupt().map(|i| crate::ids::purpose(&i));
                    let d = Decision {
                        decision,
                        seat,
                        window,
                        source,
                        purpose,
                        options: options
                            .iter()
                            .map(|o| {
                                let mut v = ctx.option(o);
                                if let Some(r) = &req {
                                    let (e, c) = (self.session.engine(), &r.options[o.index]);
                                    let names = &self.viewer.numbering;
                                    v.key = describe::key(e, r, c, names);
                                    v.effects = describe::effects(e, r, c, names);
                                }
                                v
                            })
                            .collect(),
                        undo,
                        allocation: allocation.as_ref().map(|a| ctx.allocation(a)),
                        votes: votes.as_ref().map(legacy::votes),
                        prompt: prompt.filter(|_| self.settings.text),
                        view: api_view.clone(),
                    };
                    self.last_view = Some(api_view);
                    self.pending = Some(d.clone());
                    m.extend(self.flush());
                    m.push(Message::Decision(d));
                    return m;
                }
                out @ old::Out::Decided { .. } => {
                    let seat = match &out {
                        old::Out::Decided { seat, .. } => *seat,
                        _ => unreachable!(),
                    };
                    let by = if seat != self.human {
                        By::Bot
                    } else if matches!(&out, old::Out::Decided { choice, .. } if choice.kind == "pass")
                    {
                        By::Auto
                    } else {
                        By::Order
                    };
                    m.extend(self.step(out, by, None));
                    if self
                        .last_view
                        .as_ref()
                        .is_some_and(|v| v.seats[self.human as usize].ousted)
                    {
                        m.extend(self.end(EndReason::HumansOusted, None));
                        return m;
                    }
                }
                old::Out::Over { vps, .. } => {
                    let v = self.view();
                    let standing = v.seats.iter().filter(|s| !s.ousted).count();
                    self.last_view = Some(v);
                    let reason = if standing <= 1 {
                        EndReason::LastStanding
                    } else {
                        EndReason::TimeLimit
                    };
                    let _ = vps;
                    m.extend(self.end(reason, None));
                    return m;
                }
            }
        }
        m.extend(self.end(EndReason::EngineError, Some("step budget exhausted".into())));
        m
    }

    /// A decision taken: its event, its record entry.
    /// `key`: the choice by the engine's copies, when the caller keyed it
    /// before it was applied (a human's answer); a bot's comes from its
    /// recorder; an order's from the session's option.
    fn step(&mut self, out: old::Out, by: By, key: Option<ChoiceKey>) -> Vec<Message> {
        let old::Out::Decided {
            decision,
            seat,
            window,
            choice,
            ..
        } = out
        else {
            return vec![];
        };
        // The choice by the numbering it was taken under; then the table
        // after it (the numbering follows every step).
        let option =
            Ctx::new(&self.viewer, self.session.engine(), self.settings.text).option(&choice);
        let api_view = self.view();
        let window = legacy::window(&window);
        let recorded = self.keys.lock().unwrap().remove(&decision);
        let key = match (by, key, recorded) {
            (_, Some(k), _) => k,
            (By::Bot, None, Some((_, k))) => k,
            _ => legacy::engine_key(self.session.engine(), &choice),
        };
        self.record.entries.push(Entry::Decision {
            decision,
            seat,
            at: self.at(),
            option: option.index,
            choice: key,
            by,
            forced: false,
            shown: None,
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
        self.last_view = Some(api_view.clone());
        let decided = Event {
            kind: event,
            text: None,
        };
        let events = self.told_with(decision, decided);
        vec![self.events(events, steps.then_some(api_view))]
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
        let mut v = self
            .viewer
            .view(self.session.engine(), &self.decks, &self.names);
        v.viewer = Some(self.human);
        let told = self.session.take_told();
        let e = self.session.engine();
        for t in told {
            let told = match self.viewer.event(e, &self.decks, t.fact) {
                Some(kind) => Told::Event(Box::new(Event {
                    kind,
                    text: t.text.filter(|_| self.settings.text),
                })),
                None => match t.fact {
                    old::narrate::Fact::Decided(n) => Told::Decided(n),
                    _ => continue,
                },
            };
            self.backlog.push_back(told);
        }
        v
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

fn legacy_spec(spec: &GameSpec, seed: u32) -> Result<old::GameSpec, SpecError> {
    Ok(old::GameSpec {
        seed: seed as u64,
        time_limit: spec.time_limit,
        seats: spec
            .seats
            .iter()
            .map(|s| old::SeatSpec {
                name: s.name.clone(),
                deck: s
                    .deck
                    .iter()
                    .map(|(k, &v)| (k.clone(), v.min(u8::MAX as u32) as u8))
                    .collect(),
                player: match &s.player {
                    Player::Human { .. } => old::Player::Human,
                    Player::Bot { profile } => old::Player::Bot {
                        style: profile.clone(),
                    },
                },
            })
            .collect(),
    })
}

fn spec_error(e: old::SpecError) -> SpecError {
    match e {
        old::SpecError::Seats { n } => SpecError::Seats { n: n as u8 },
        old::SpecError::Style { seat, style } => SpecError::Profile {
            seat: seat as Seat,
            profile: style,
        },
        old::SpecError::Deck { seat, reason } => deck_error(seat as Seat, reason),
    }
}

pub(crate) fn deck_error(seat: Seat, e: old::deck::DeckError) -> SpecError {
    match e {
        old::deck::DeckError::Unsupported { ids } => SpecError::Unsupported { seat, cards: ids },
        old::deck::DeckError::TooSmall { crypt, library } => SpecError::DeckSize {
            seat,
            crypt: crypt as u32,
            library: library as u32,
        },
    }
}

fn answer_error(e: old::AnswerError, option: u32) -> Error {
    match e {
        old::AnswerError::Stale { expected } => Error::Stale { expected },
        old::AnswerError::NotYours { .. } => Error::NotYourTurn,
        old::AnswerError::Option { .. } => Error::Option { index: option },
        old::AnswerError::Over => Error::Over,
        old::AnswerError::NoUndo => Error::NoUndo,
        old::AnswerError::NoAllocation => Error::Allocation {
            reason: AllocationError::None,
        },
        old::AnswerError::Allocation { .. } => Error::Allocation {
            reason: AllocationError::Total,
        },
        old::AnswerError::NoVotes => Error::NoVotes,
    }
}

#[cfg(test)]
mod tests {
    //! The view from the engine (`view.rs`) against the session's (the
    //! A1b oracle), and the copy rules (docs/protocol.md §3), at every
    //! decision of random games.

    use std::collections::{HashMap, HashSet};

    use serde_json::{json, Value};
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

    fn ids(cards: &[Card]) -> Vec<Option<u32>> {
        cards.iter().map(|c| c.id).collect()
    }

    /// What both views must agree on.
    fn new_shape(v: &TableView) -> Value {
        json!({
            "turn": v.turn, "active": v.active,
            "seats": v.seats.iter().map(|s| json!({
                "pool": s.pool, "vp": s.vp_halves, "ousted": s.ousted, "edge": s.edge,
                "prey": s.prey, "predator": s.predator,
                "crypt": s.crypt.len(), "library": s.library.len(),
                "hand": ids(&s.hand), "ash": ids(&s.ash_heap), "removed": ids(&s.removed),
                "limbo": ids(&s.limbo),
                "slots": s.uncontrolled.iter().map(|u| json!([u.card.id, u.counters, u.banished, ids(&u.cards)])).collect::<Vec<_>>(),
                "minions": s.ready.iter().chain(&s.torpor).map(|m| json!([m.card.id, m.blood, m.locked, m.capacity, m.contested,
                    m.cards.iter().map(|c| json!([c.card.id, c.locked, c.counters, ids(&c.stored)])).collect::<Vec<_>>()])).collect::<Vec<_>>(),
                "in_play": s.in_play.iter().map(|c| json!([c.card.id, c.locked, c.counters, ids(&c.stored)])).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "action": v.action.as_ref().map(|a| json!([a.seat, a.stealth, a.bleed, a.x, a.bleed_target, a.declined, a.card.map(|c| c.id)])),
            "combat": v.combat.as_ref().map(|c| c.round),
            "referendum": v.referendum.as_ref().map(|r| json!([r.caller, r.votes_for, r.votes_against, r.votes.len()])),
        })
    }

    fn old_shape(v: &zvtes_protocol::TableView) -> Value {
        let ids = |cards: &[zvtes_protocol::view::CardView]| -> Vec<Option<u32>> {
            cards.iter().map(|c| c.id).collect()
        };
        json!({
            "turn": v.turn, "active": v.active,
            "seats": v.seats.iter().map(|s| json!({
                "pool": s.pool, "vp": (s.vps * 2.0).round() as u32, "ousted": s.ousted, "edge": s.edge,
                "prey": s.prey, "predator": s.predator,
                "crypt": s.crypt.len(), "library": s.library.len(),
                "hand": ids(&s.hand), "ash": ids(&s.ash_heap), "removed": ids(&s.removed),
                "limbo": ids(&s.limbo),
                "slots": s.uncontrolled.iter().map(|u| json!([u.card.id, u.counters, u.banished, ids(&u.cards)])).collect::<Vec<_>>(),
                "minions": s.ready.iter().chain(&s.torpor).map(|m| json!([m.card.id, m.blood, m.locked, m.capacity, m.contested,
                    m.cards.iter().map(|c| json!([c.card.id, c.locked, c.counters, ids(&c.stored)])).collect::<Vec<_>>()])).collect::<Vec<_>>(),
                "in_play": s.in_play.iter().map(|c| json!([c.card.id, c.locked, c.counters, ids(&c.stored)])).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "action": v.action.as_ref().map(|a| json!([a.seat, a.stealth, a.bleed, a.x, a.bleed_target, a.declined, a.card])),
            "combat": v.combat.as_ref().map(|c| c.round),
            "referendum": v.referendum.as_ref().map(|r| json!([r.caller, r.votes_for, r.votes_against, r.votes.len()])),
        })
    }

    /// Every card shown, with whether its place is hidden to the viewer.
    fn cards(v: &TableView, viewer: u8) -> Vec<(Card, bool)> {
        let mut out = Vec::new();
        for s in &v.seats {
            for c in s.crypt.iter().chain(&s.library) {
                out.push((*c, true));
            }
            for c in &s.hand {
                out.push((*c, s.seat != viewer));
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
    fn the_engine_view_agrees_with_the_session_and_keeps_the_copy_rules() {
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
                // The session's own view of this decision.
                let old = g.session.view(Some(zvtes::ids::SeatId(human)));
                assert_eq!(
                    new_shape(&d.view),
                    old_shape(&old),
                    "seed {seed}, step {step}"
                );
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
}
