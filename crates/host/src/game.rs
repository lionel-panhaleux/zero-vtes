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

/// A feedback's length cap: a report, not a log.
const FEEDBACK_CHARS: usize = 4000;

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
    fn make(&self, profile: &str, seed: u64) -> Option<(Box<dyn Agent>, BotRecord)>;
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
    /// The viewer's numbering and minions' names at each decision it was
    /// asked, by number: an undo restores both, the view asked again the
    /// view first sent.
    numberings: BTreeMap<u64, (crate::numbering::Numbering, crate::numbering::MinionNames)>,
    /// The decisions taken and not told yet, by number (`text` only).
    asked: HashMap<u64, Asked>,
    record: GameRecord,
    clock: Box<dyn Clock>,
    t0: u64,
    /// The names the human seat's client knows, from the messages it was
    /// sent: what the English renderer reads.
    known: zvtes_text::Names<'static>,
    /// A record played back into the game (`Game::resume`): its entries,
    /// the next one, what stopped it.
    resuming: Option<Resuming>,
    /// Resumed, not yet joined: the first join is a fresh start (the
    /// client's numbering is the lost process's).
    resumed: bool,
    /// Rendering panics (a test's way into `handle`'s guard).
    #[cfg(test)]
    render_panics: bool,
}

struct Resuming {
    entries: Vec<Entry>,
    next: usize,
    failed: Option<String>,
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
        if spec.seed.is_some_and(|s| s > zvtes_api::SEED_MAX) {
            return Err(SpecError::Seed {
                max: zvtes_api::SEED_MAX,
            });
        }
        let t0 = clock.now();
        let seed = *spec.seed.get_or_insert(t0 & zvtes_api::SEED_MAX);
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
            // Each bot's own: the formula before 2.0.0 on the low 32 bits,
            // the seed's high bits kept, so a seed under 2^32 keeps its
            // bots (`resume` compares them with the record's).
            let low = (seed as u32)
                .wrapping_mul(0x9e37_79b9)
                .wrapping_add(seat as u32);
            let bot_seed = seed & !0xffff_ffff | low as u64;
            let (agent, mut r): (Box<dyn Agent>, BotRecord) = match profile.as_str() {
                "aggro_bleed" | "wall" | "random" => (
                    match profile.as_str() {
                        "aggro_bleed" => Box::new(Scripted::new(Style::AggroBleed, bot_seed)),
                        "wall" => Box::new(Scripted::new(Style::Wall, bot_seed)),
                        _ => Box::new(RandomLegal::new(bot_seed)),
                    },
                    BotRecord {
                        seat: 0,
                        profile: profile.clone(),
                        kind: profile.clone(),
                        weights: None,
                        temperature: None,
                        seed: bot_seed,
                        nudges: Default::default(),
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
        let mut session = Session::new(seed, spec.time_limit, decks, agents);
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
            resuming: None,
            resumed: false,
            #[cfg(test)]
            render_panics: false,
        })
    }

    /// A game rebuilt from the record of one left going (a server
    /// restarted): dealt again from its spec, its decisions taken again as
    /// recorded, by key (bots' and passes' as the game takes them, the
    /// human's answers, settings, undos and feedback as commands), then
    /// live. Its bots must be made as recorded (the same weights, the same
    /// nudges); its entries stay the record's. An error when it cannot be
    /// (a rules change, a bot no longer served): the game is ended instead.
    pub fn resume(
        id: String,
        record: &GameRecord,
        bots: &dyn Bots,
        clock: Box<dyn Clock>,
    ) -> Result<Game, String> {
        if record.result.is_some() {
            return Err("the game is over".into());
        }
        if record.spec.seed.is_none() {
            return Err("a record without its seed".into());
        }
        let mut g =
            Game::new(id, record.spec.clone(), bots, clock).map_err(|e| format!("{e:?}"))?;
        if g.record.bots != record.bots {
            return Err("its bots are no longer made as recorded".into());
        }
        g.resuming = Some(Resuming {
            entries: record.entries.clone(),
            next: 0,
            failed: None,
        });
        let human = g.human;
        let send = |g: &mut Game, command: Command| {
            g.handle(human, In { id: None, command });
        };
        send(
            &mut g,
            Command::Join {
                token: None,
                after: None,
            },
        );
        loop {
            if let Some(e) = &g.resuming.as_ref().expect("resuming").failed {
                return Err(e.clone());
            }
            g.resume_check()?;
            let r = g.resuming.as_ref().expect("resuming");
            let Some(entry) = r.entries.get(r.next).cloned() else {
                break;
            };
            g.resuming.as_mut().expect("resuming").next += 1;
            match entry {
                Entry::Decision { decision, by, .. } => {
                    let Some(req) = g.session.request().cloned() else {
                        return Err(format!("decision {decision}: the game is over"));
                    };
                    let at = g.resuming.as_ref().expect("resuming").next - 1;
                    let d = crate::replay::Recorded::of(at, &entry).expect("a decision");
                    let favor = d.choice.favor;
                    let i = crate::replay::recorded_option(g.session.engine(), &req, &d)?;
                    let decision = g.session.decision();
                    // An allocation answered whole: its steps taken as
                    // `Command::Allocate` takes them (the undo depth reads
                    // them so).
                    if by == By::Order && g.session.allocation(&req).is_some() {
                        let m = g.replay_allocation(i)?;
                        g.emit(human, None, m, true);
                        continue;
                    }
                    // A vote order: its first cast; the rest taken as the
                    // order casts them (`recorded_take`).
                    if by == By::Order
                        && matches!(
                            d.choice.kind,
                            OptionKind::CastVotes | OptionKind::CastCardVote
                        )
                    {
                        let Some(favor) = favor else {
                            return Err(format!("decision {decision}: a vote with no side"));
                        };
                        if g.session.casts(&req, favor).first().map(|&(c, _)| c) != Some(i) {
                            return Err(format!(
                                "decision {decision}: a vote order cast otherwise"
                            ));
                        }
                        send(&mut g, Command::VoteAll { decision, favor });
                        continue;
                    }
                    send(
                        &mut g,
                        Command::Answer {
                            decision,
                            option: i as u32,
                        },
                    );
                }
                Entry::Undo { to, .. } => {
                    let decision = g.session.decision();
                    send(&mut g, Command::Undo { decision });
                    if g.session.decision() != to {
                        return Err(format!("an undo to decision {to} went elsewhere"));
                    }
                }
                Entry::Settings { settings, .. } => send(
                    &mut g,
                    Command::Settings(zvtes_api::SettingsPatch {
                        opportunities: Some(settings.opportunities),
                        views: Some(settings.views),
                        text: Some(settings.text),
                    }),
                ),
                Entry::Feedback { text, .. } => send(&mut g, Command::Feedback { text }),
                Entry::Refused { .. } | Entry::Unknown => {}
            }
        }
        g.resuming = None;
        // The record as it was, its times its own (an end the replay
        // reached kept).
        g.record = GameRecord {
            entries: record.entries.clone(),
            ended: g.record.ended,
            result: g.record.result.clone(),
            ..record.clone()
        };
        g.t0 = record.started;
        g.sent.clear();
        g.resumed = true;
        Ok(g)
    }

    /// While resuming, an allocation answered whole: its first step `i`
    /// the human's, each next recorded step of it an order's, as
    /// `Command::Allocate` takes them; then the game advances.
    fn replay_allocation(&mut self, i: usize) -> Result<Vec<Message>, String> {
        let left = |g: &Game| g.session.engine().allocation().map(|a| a.recipients.len());
        let mut before = left(self);
        let taken = self.session.human();
        let mut m = self.take(i, taken, By::Order);
        loop {
            self.session.ready();
            // The same allocation while its recipients still to allocate
            // shrink (a next one starts from them all).
            let now = left(self);
            if !now.is_some_and(|n| before.is_some_and(|b| n < b)) {
                break;
            }
            before = now;
            let Some(req) = self.session.request().cloned() else {
                break;
            };
            self.resume_check()?;
            let r = self.resuming.as_ref().expect("resuming");
            let step = match r
                .entries
                .get(r.next)
                .and_then(|e| crate::replay::Recorded::of(r.next, e))
            {
                Some(d)
                    if d.by == By::Order
                        && d.seat == self.human
                        && req.seat.0 == self.human
                        && matches!(
                            d.choice.kind,
                            OptionKind::AllocateTo | OptionKind::AllocateToCard
                        ) =>
                {
                    d
                }
                _ => break,
            };
            self.resuming.as_mut().expect("resuming").next += 1;
            let i = crate::replay::recorded_option(self.session.engine(), &req, &step)?;
            m.extend(self.take(i, Taken::Order, By::Order));
        }
        m.extend(self.advance());
        Ok(m)
    }

    /// While resuming, the recorded option of a decision the game takes
    /// without asking (a bot's, a pass, an order's), in place of `index`.
    fn recorded_take(&mut self, index: usize) -> usize {
        if self.resuming.as_ref().is_none_or(|r| r.failed.is_some()) {
            return index;
        }
        if let Err(e) = self.resume_check() {
            self.resuming.as_mut().expect("resuming").failed = Some(e);
            return index;
        }
        let r = self.resuming.as_mut().expect("resuming");
        let Some(d) = r
            .entries
            .get(r.next)
            .and_then(|e| crate::replay::Recorded::of(r.next, e))
        else {
            r.failed = Some(format!(
                "entry {}: a decision taken the record does not hold",
                r.next
            ));
            return index;
        };
        r.next += 1;
        let req = self.session.request().expect("a decision").clone();
        match crate::replay::recorded_option(self.session.engine(), &req, &d) {
            Ok(i) => i,
            Err(e) => {
                self.resuming.as_mut().expect("resuming").failed = Some(e);
                index
            }
        }
    }

    /// While resuming, the record's next decision checked against the
    /// decision asked (`replay::past`): an error where the game is no
    /// longer the game recorded. Never read past, unlike a replay: the
    /// record's decision numbers would no longer be the session's (an
    /// undo's `to`, the entries kept), so a Pass in a window gone ends the
    /// game at the restart too.
    fn resume_check(&self) -> Result<(), String> {
        let (Some(r), Some(req)) = (self.resuming.as_ref(), self.session.request()) else {
            return Ok(());
        };
        let Some(d) = r
            .entries
            .get(r.next)
            .and_then(|e| crate::replay::Recorded::of(r.next, e))
        else {
            return Ok(());
        };
        // Alone, the decision is never read past: a window gone errs.
        crate::replay::past(self.session.engine(), req, std::slice::from_ref(&d)).map(|_| ())
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
                true,
            ),
        };
        serde_json::to_string(&outs).expect("messages serialize")
    }

    /// The seat's command, answered: the messages for it, in order. A
    /// panic past the engine's own guard (`command`), rendering the reply
    /// or a resend, ends the game `engine_error`, its end sent unrendered:
    /// never a game left going.
    pub fn handle(&mut self, seat: Seat, input: In) -> Vec<Out> {
        let id = input.id;
        match catch_unwind(AssertUnwindSafe(|| self.serve(seat, input))) {
            Ok(outs) => outs,
            Err(panic) => {
                if self.over.is_none() {
                    self.fail(panic_text(&panic));
                }
                // The end alone: events would carry their text unfilled.
                let over = self.over.clone().map(Message::Over).into_iter().collect();
                self.emit(seat, id, over, false)
            }
        }
    }

    fn serve(&mut self, seat: Seat, input: In) -> Vec<Out> {
        if let Command::Join {
            after: Some(after), ..
        } = input.command
        {
            if seat == self.human && !self.resumed {
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
        self.emit(seat, input.id, messages, true)
    }

    /// Messages numbered (`render`: their names and English told), kept
    /// for a rejoin; refusals recorded.
    fn emit(
        &mut self,
        seat: Seat,
        id: Option<u64>,
        messages: Vec<Message>,
        render: bool,
    ) -> Vec<Out> {
        let at = self.at();
        let mut out = Vec::with_capacity(messages.len());
        for mut m in messages {
            if render {
                self.tell(&mut m);
            }
            let reply_to = match &m {
                Message::Error(e) => {
                    self.record.entries.push(Entry::Refused {
                        seat,
                        at,
                        error: e.clone(),
                    });
                    id
                }
                Message::Noted { .. } => id,
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
        #[cfg(test)]
        if self.render_panics {
            panic!("render");
        }
        let text = self.settings.text;
        let known = &mut self.known;
        match m {
            Message::Joined(j) => {
                known.see(&j.view);
                if text {
                    tell_view(&mut j.view, known);
                }
            }
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
                    if text {
                        tell_view(v, known);
                    }
                }
            }
            Message::Decision(d) => {
                known.see(&d.view);
                if text {
                    tell_view(&mut d.view, known);
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
            Message::Over(o) => {
                known.see(&o.view);
                if text {
                    tell_view(&mut o.view, known);
                }
            }
            Message::Error(_) | Message::Noted { .. } => {}
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
            self.resumed = false;
            return self.join();
        }
        if let Command::Feedback { text } = c {
            if !self.started {
                return vec![Message::Error(Error::NotJoined)];
            }
            return self.feedback(&text);
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
            Command::Join { .. } | Command::Feedback { .. } => unreachable!(),
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
                        if let Some((n, names)) = self.numberings.get(&at) {
                            self.viewer.numbering = n.clone();
                            self.viewer.names = names.clone();
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

    /// The player's report, kept in the record at the decision it is about.
    fn feedback(&mut self, text: &str) -> Vec<Message> {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > FEEDBACK_CHARS {
            return vec![Message::Error(Error::Malformed {
                reason: format!("feedback: 1 to {FEEDBACK_CHARS} characters"),
            })];
        }
        let decision = self
            .pending
            .as_ref()
            .map(|d| d.decision)
            .unwrap_or_else(|| {
                self.record
                    .entries
                    .iter()
                    .rev()
                    .find_map(|e| match e {
                        Entry::Decision { decision, .. } => Some(*decision),
                        _ => None,
                    })
                    .unwrap_or(0)
            });
        self.record.entries.push(Entry::Feedback {
            seat: self.human,
            at: self.at(),
            decision,
            text: text.into(),
        });
        vec![Message::Noted { decision }]
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
        option_view(
            self.session.engine(),
            req,
            i,
            &self.viewer.numbering,
            self.human,
        )
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
                        due: describe::dues(e, &req, names),
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
                    self.numberings.insert(
                        decision,
                        (self.viewer.numbering.clone(), self.viewer.names.clone()),
                    );
                    if self.numberings.len() > 256 {
                        self.numberings.pop_first();
                    }
                    self.pending = Some(d.clone());
                    m.extend(self.flush());
                    m.push(Message::Decision(d));
                    return m;
                }
                Next::Take { index, taken } => {
                    let index = self.recorded_take(index);
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
        let digest = crate::digest::digest(self.session.engine());
        // What it chose for, as its decision tells it to its seat.
        let purpose = self
            .session
            .engine()
            .interrupt()
            .map(|i| crate::ids::purpose(&i));
        let text = self.settings.text && by != By::Auto;
        // As the events tell it: a card come into play since the last view
        // numbered fresh. Its English (the decision's particulars and
        // options) numbered on a copy: the text setting leaves the numbers
        // as they are.
        let e = self.session.engine();
        let human = self.human;
        let (option, source) = {
            let telling = describe::Telling {
                numbering: std::cell::RefCell::new(&mut self.viewer.numbering),
                decks: &self.decks,
                e,
            };
            (
                option_view(e, &req, index, &telling, human),
                describe::source(e, &req, &telling),
            )
        };
        let mut scratch = text.then(|| self.viewer.numbering.clone());
        let asked = scratch.as_mut().map(|n| {
            let telling = describe::Telling {
                numbering: std::cell::RefCell::new(n),
                decks: &self.decks,
                e,
            };
            let offered = (0..req.options.len())
                .map(|i| option_view(e, &req, i, &telling, human))
                .collect::<Vec<_>>();
            (describe::about(e, &req, &telling), offered)
        });
        let key = describe::key(
            self.session.engine(),
            &req,
            &req.options[index],
            &EngineNames,
        );
        // The options the seat was shown, when its settings hid some.
        let shown = self
            .pending
            .as_ref()
            .filter(|d| {
                by != By::Bot && d.decision == decision && d.options.len() < req.options.len()
            })
            .map(|d| d.options.iter().map(|o| o.index).collect());
        if let Some((about, offered)) = asked {
            self.asked.insert(
                decision,
                Asked {
                    window,
                    purpose,
                    about,
                    seat,
                    offered,
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
            window: Some(window),
            digest: Some(digest),
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
                source,
                purpose,
                choice: Box::new(option),
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
            let told = match self.viewer.event(e, &self.decks, &t) {
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
    /// next pair, with the damage it dealt; between a first strike and the
    /// strike it held back, the held one as declared, not yet resolved.
    fn struck_view(&self, v: &mut TableView) {
        let e = self.session.engine();
        let waiting = e.waiting_strikes().copied();
        let (
            Some(c),
            Some(cv),
            Some(CombatNote::Strikes {
                minions,
                round,
                pair,
                strikes,
                told,
                strike_damage,
                ..
            }),
        ) = (
            e.combat(),
            v.combat.as_mut(),
            waiting.or(self.session.struck()),
        )
        else {
            return;
        };
        if c.round != round || c.pair != pair {
            return;
        }
        let held = waiting
            .and(c.held_strike.as_ref())
            .map(|&(i, _)| c.sides[i].minion);
        for (side, view) in c.sides.iter().zip(&mut cv.sides) {
            let Some(j) = minions.iter().position(|&m| m == side.minion) else {
                continue;
            };
            if let (true, None, Some(k)) = (side.chosen, side.strike, strikes[j]) {
                // As told when it resolved: resolving cleared its riders.
                let mut s = crate::view::told(crate::view::strike(k, None), told[j]);
                s.resolved = (held != Some(side.minion)).then_some(strike_damage[j]);
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
        let winner = winner(&vp_halves);
        let over = Over {
            vp_halves: vp_halves.clone(),
            winner,
            reason,
            view,
        };
        self.pending = None;
        self.over = Some(over.clone());
        self.record.ended = Some(self.clock.now());
        let turns = Some(self.session.engine().state.turn);
        self.record.result = Some(GameResult {
            vp_halves,
            winner,
            reason,
            failure,
            turns,
        });
        told.into_iter().chain([Message::Over(over)]).collect()
    }

    /// The idle timeout (docs/protocol.md §2): the game ends abandoned by
    /// its human, joined or not (the seat's `leave` is refused before its
    /// join).
    pub fn abandon(&mut self) {
        if self.over.is_none() {
            self.end(EndReason::Abandoned { seat: self.human }, None);
        }
    }

    fn fail(&mut self, why: String) -> Vec<Message> {
        self.end(EndReason::EngineError, Some(why))
    }
}

/// The most victory points, at least 2 of them, alone.
pub(crate) fn winner(vp_halves: &[u32]) -> Option<Seat> {
    let best = vp_halves.iter().copied().max().unwrap_or(0);
    (best >= 4 && vp_halves.iter().filter(|&&v| v == best).count() == 1)
        .then(|| vp_halves.iter().position(|&v| v == best).unwrap() as Seat)
}

/// An option as `names` tells it (`human`: the viewer's seat).
fn option_view(
    e: &zvtes::game::Engine,
    req: &DecisionRequest,
    i: usize,
    names: &impl describe::Names,
    human: Seat,
) -> OptionView {
    let c = &req.options[i];
    let mut key = describe::key(e, req, c, names);
    // Game of Malkav: another seat chose a number, not which — no
    // amount, and the index (the number less one) told as 0.
    let secret = req.seat.0 != human
        && matches!(
            e.interrupt(),
            Some(zvtes::draws::Interrupt::SecretNumber { .. })
        );
    if req.seat.0 != human {
        narrated(&mut key, c, req.window);
    }
    if secret {
        key.amount = None;
    }
    OptionView {
        index: if secret { 0 } else { i as u32 },
        key,
        effects: if secret {
            Vec::new()
        } else {
            describe::effects(e, req, c, names)
        },
        abilities: describe::abilities(e, c),
        apart: describe::ability_apart(e, c),
        user: describe::ability_user(e, req, c, names),
        text: None,
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
        transfers: None,
    }
}

/// The view's strikes and markers in English (`Strike::text`,
/// `Marker::text`), from their data.
fn tell_view(v: &mut TableView, known: &zvtes_text::Names) {
    for side in v.combat.iter_mut().flat_map(|c| &mut c.sides) {
        if let Some(s) = side.strike.as_mut() {
            s.text = Some(zvtes_text::strike_declared(s, known));
        }
    }
    for s in &mut v.seats {
        for m in s.ready.iter_mut().chain(&mut s.torpor) {
            for k in &mut m.markers {
                k.text = Some(zvtes_text::marker(k, known));
            }
        }
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
        fn make(&self, _: &str, _: u64) -> Option<(Box<dyn Agent>, BotRecord)> {
            None
        }
    }

    fn game(seed: u64) -> Game {
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
            let mut g = game(seed.into());
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
        let mut futile = 0;
        // A seed past 32 bits too (protocol 2.0.0): the largest.
        for seed in (1..=24u64).chain([zvtes_api::SEED_MAX]) {
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
                futile += usize::from(e.futile_blocks(&req));
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
            // A record of this engine reads whole, its futile block
            // windows as recorded (rules-qa 73.8).
            let replay = crate::replay::Replay::new(g.record()).unwrap();
            assert_eq!(replay.read(usize::MAX).1, n, "seed {seed}");
            assert_eq!(g.record().spec.seed, Some(seed));
            assert!(g
                .record()
                .bots
                .iter()
                .all(|b| b.seed <= zvtes_api::SEED_MAX));
        }
        assert!(futile > 0, "no futile block window met");
    }

    /// A panic rendering a reply ends the game `engine_error`, recorded
    /// with its text, its `over` sent unrendered; a command after it gets
    /// that end again, the record's first kept.
    #[test]
    fn a_panic_rendering_a_reply_ends_the_game() {
        let mut g = game(1);
        let human = g.human();
        g.render_panics = true;
        let join = || In {
            id: None,
            command: Command::Join {
                token: None,
                after: None,
            },
        };
        let over = |outs: &[Out]| match outs.last().map(|o| &o.message) {
            Some(Message::Over(o)) => o.reason,
            m => panic!("not over: {m:?}"),
        };
        let outs = g.handle(human, join());
        assert_eq!(outs.len(), 1, "the end alone: {outs:?}");
        assert_eq!(over(&outs), EndReason::EngineError);
        let failure = |g: &Game| g.record().result.clone().expect("ended").failure;
        assert_eq!(failure(&g), Some("render".into()));
        let ended = g.record().ended;
        assert_eq!(over(&g.handle(human, join())), EndReason::EngineError);
        assert_eq!(
            (failure(&g), g.record().ended),
            (Some("render".into()), ended)
        );
    }

    /// A seed past `SEED_MAX` is refused (protocol 2.0.0): a JSON number
    /// would not hold it exactly.
    #[test]
    fn a_seed_past_53_bits_is_refused() {
        let mut spec = game(1).record().spec.clone();
        spec.seed = Some(zvtes_api::SEED_MAX + 1);
        let refused = Game::new("t".into(), spec, &NoBots, Box::new(SystemClock)).err();
        assert_eq!(
            refused,
            Some(SpecError::Seed {
                max: zvtes_api::SEED_MAX
            })
        );
    }

    /// A seed's bits past 32 reach the deal (protocol 2.0.0): the same
    /// spec dealt with the largest seed and with its low 32 bits.
    #[test]
    fn a_seeds_high_bits_change_the_deal() {
        let spec = game(1).record().spec.clone();
        let libraries = |seed: u64| {
            let spec = GameSpec {
                seed: Some(seed),
                ..spec.clone()
            };
            let g = Game::new("t".into(), spec, &NoBots, Box::new(SystemClock)).unwrap();
            g.session
                .engine()
                .state
                .seats
                .iter()
                .map(|s| s.library.clone())
                .collect::<Vec<_>>()
        };
        let max = zvtes_api::SEED_MAX;
        assert_ne!(libraries(max), libraries(max & 0xffff_ffff));
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
            let mut g = game(seed.into());
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
