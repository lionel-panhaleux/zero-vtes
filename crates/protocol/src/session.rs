//! A game between human seats and bots (docs/human-play.md): the engine
//! stepped one decision at a time, bot seats answered inside, every
//! message carrying the viewer's table as of that decision so a client can
//! animate the change, then narrate or ask.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use zvtes::agent::{Agent, RandomLegal};
use zvtes::bots::{Scripted, Style};
use zvtes::cards::AbilitySource;
use zvtes::decision::{Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::SeatId;
use zvtes::library::LIBRARY;
use zvtes::referendum::{Recipient, RefStep};

use crate::deck::{resolve, DeckError, DeckList, Resolved};
use crate::describe::{window_text, CardRef, Describer, OptionView};
use crate::narrate::{action_thing, combat_text, minion_name, Mark};
use crate::track::Tracker;
use crate::view::{knowledge, learned_nothing, table_view, Knowledge, TableView};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GameSpec {
    pub seed: u64,
    /// In player turns; the engine's default when absent.
    #[serde(default)]
    pub time_limit: Option<u32>,
    /// In seating order.
    pub seats: Vec<SeatSpec>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeatSpec {
    pub name: String,
    pub deck: DeckList,
    pub player: Player,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Player {
    Human,
    /// A built-in bot: "aggro_bleed", "wall" or "random"; or one a host
    /// installs (`Session::set_agent`), e.g. the network policy.
    Bot {
        style: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum SpecError {
    Seats { n: usize },
    Deck { seat: usize, reason: DeckError },
    Style { seat: usize, style: String },
}

/// What a session says, one message per decision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Out {
    /// A human seat must decide: answer with `decision` and an option index.
    Decision {
        decision: u64,
        seat: u8,
        window: String,
        options: Vec<OptionView>,
        /// The seat's previous decision can be taken back (`Session::undo`):
        /// nothing was decided since and nothing hidden revealed to it.
        undo: bool,
        /// An allocation of points under way, whole: answered at once with
        /// `Session::allocate` (or one recipient at a time by `options`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        allocation: Option<AllocationView>,
        /// The votes the seat's vampires and calling card can cast now, each
        /// way: all of one way at once with `Session::vote_all`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        votes: Option<VoteTotals>,
        /// What happened since the previous message, before this one
        /// (`narrate`): an action's or a referendum's end, pool and blood
        /// moved, minions and cards in and out of play.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        events: Vec<String>,
        view: TableView,
    },
    /// A decision was taken (by a bot, or a human's answer), as the
    /// viewer may know it; `view` is the table it was taken on.
    Decided {
        decision: u64,
        seat: u8,
        window: String,
        choice: OptionView,
        /// What happened since the previous message, before this one
        /// (`narrate`): an action's or a referendum's end, pool and blood
        /// moved, minions and cards in and out of play.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        events: Vec<String>,
        view: TableView,
    },
    /// The game ended: victory points per seat.
    Over {
        vps: Vec<f32>,
        /// What happened since the previous message, before this one
        /// (`narrate`): an action's or a referendum's end, pool and blood
        /// moved, minions and cards in and out of play.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        events: Vec<String>,
        view: TableView,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum AnswerError {
    /// Not the pending decision (answered already, or a stale client).
    Stale {
        expected: u64,
    },
    NotYours {
        seat: u8,
    },
    Option {
        options: usize,
    },
    Over,
    /// Nothing to take back: only a step of the multi-step choice under way.
    NoUndo,
    /// No allocation of points under way.
    NoAllocation,
    /// Amounts the allocation under way does not allow.
    Allocation {
        reason: String,
    },
    /// No vote to cast that way.
    NoVotes,
}

/// An allocation of points under way (a referendum's terms, or a
/// shortfall's re-allocation): every recipient still to allocate to, in
/// the order the engine asks them. Valid amounts: all `points` given, at
/// most `cap` to a recipient, at least `min_nonzero` recipients given
/// some.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AllocationView {
    pub points: u8,
    pub recipients: Vec<RecipientView>,
    pub min_nonzero: u8,
}

/// A Methuselah (`seat`) or a card in play (`card`: a location or an
/// equipment).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecipientView {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seat: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<CardRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap: Option<u8>,
}

/// Votes castable now in favor of and against the referendum: the seat's
/// vampires' and its calling card's (not the Edge, a card burned for a
/// vote, a play or an ability — those stay single options).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoteTotals {
    pub favor: u8,
    pub against: u8,
}

/// A game as a replay: the spec, and every decision's option index.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Replay {
    pub spec: GameSpec,
    pub log: Vec<usize>,
}

pub struct Session {
    spec: GameSpec,
    decks: Vec<Resolved>,
    /// Who took each logged decision, in which window; for a human seat,
    /// what it knew of the cards when it was asked (`can_undo`).
    history: Vec<(SeatId, WindowLabel, Option<Knowledge>)>,
    engine: Engine,
    agents: Vec<Option<Box<dyn Agent>>>,
    /// Who the session narrates to: the first human seat (`None`: all
    /// bots, everything shown).
    viewer: Option<SeatId>,
    trackers: HashMap<Option<u8>, Tracker>,
    pending: Option<DecisionRequest>,
    over: Option<Vec<u32>>,
    /// Seats casting all their votes one way (`vote_all`), for the rest of
    /// the referendum's polling.
    standing: Vec<(SeatId, bool)>,
    /// The table at the last decision or message (`narrate`), and what
    /// changed since, told with the next message.
    mark: Option<Mark>,
    events: Vec<String>,
    /// The card the last decision played or used, and by whom: what the
    /// seat's terms decisions that follow choose for.
    about: Option<(SeatId, String)>,
}

/// The bot styles a session builds itself.
pub const BUILT_IN: [&str; 3] = ["aggro_bleed", "wall", "random"];

impl Session {
    /// A session whose bots are all built in.
    pub fn new(spec: GameSpec) -> Result<Session, SpecError> {
        Self::with_styles(spec, &[])
    }

    /// A session whose bots may also use `installed` styles: the host
    /// installs their agents (`set_agent`) before advancing.
    pub fn with_styles(spec: GameSpec, installed: &[&str]) -> Result<Session, SpecError> {
        let n = spec.seats.len();
        if !(2..=6).contains(&n) {
            return Err(SpecError::Seats { n });
        }
        for (seat, s) in spec.seats.iter().enumerate() {
            if let Player::Bot { style } = &s.player {
                if !BUILT_IN.contains(&style.as_str()) && !installed.contains(&style.as_str()) {
                    return Err(SpecError::Style {
                        seat,
                        style: style.clone(),
                    });
                }
            }
        }
        let decks: Vec<Resolved> = spec
            .seats
            .iter()
            .enumerate()
            .map(|(seat, s)| resolve(&s.deck).map_err(|reason| SpecError::Deck { seat, reason }))
            .collect::<Result<_, _>>()?;
        let engine = Self::deal(&spec, &decks);
        let mut agents = Vec::new();
        for (seat, s) in spec.seats.iter().enumerate() {
            let seed = spec.seed.wrapping_mul(31).wrapping_add(seat as u64);
            agents.push(match &s.player {
                Player::Human => None,
                Player::Bot { style } => Some(match style.as_str() {
                    "aggro_bleed" => {
                        Box::new(Scripted::new(Style::AggroBleed, seed)) as Box<dyn Agent>
                    }
                    "wall" => Box::new(Scripted::new(Style::Wall, seed)),
                    "random" => Box::new(RandomLegal::new(seed)),
                    // An installed style: its agent comes from the host.
                    _ => Box::new(Unset),
                }),
            });
        }
        let viewer = spec
            .seats
            .iter()
            .position(|s| s.player == Player::Human)
            .map(|i| SeatId(i as u8));
        Ok(Session {
            spec,
            decks,
            history: Vec::new(),
            engine,
            agents,
            viewer,
            trackers: HashMap::new(),
            pending: None,
            over: None,
            standing: Vec::new(),
            mark: None,
            events: Vec::new(),
            about: None,
        })
    }

    /// The game as dealt: from the seed, nothing decided yet.
    fn deal(spec: &GameSpec, decks: &[Resolved]) -> Engine {
        let lists: Vec<_> = decks
            .iter()
            .map(|d| (d.crypt.as_slice(), d.library.as_slice()))
            .collect();
        let mut engine = Engine::with_lists(spec.seed, &lists);
        if let Some(t) = spec.time_limit {
            engine.time_limit = t;
        }
        // Combats told as they happen (`observe`).
        engine.combat_notes = Some(Vec::new());
        engine
    }

    /// The pending decision is the seat's again, right after its previous
    /// one: nothing was decided since (by a bot, or by the seat's standing
    /// order to vote) and nothing hidden was revealed to it (a draw, a card
    /// turned face up), so taking that decision back teaches it nothing.
    fn can_undo(&self) -> bool {
        let (Some(req), Some((seat, _, Some(before)))) = (&self.pending, self.history.last())
        else {
            return false;
        };
        req.seat == *seat
            && self.agents[seat.index()].is_none()
            && !self.standing.iter().any(|&(s, _)| s == *seat)
            && learned_nothing(before, &knowledge(&self.engine, *seat))
    }

    /// Take back the seat's previous decision (`Out::Decision::undo`): the
    /// game is dealt again and replayed to the decision before.
    pub fn undo(&mut self) -> Result<(), AnswerError> {
        self.step();
        if !self.can_undo() {
            return Err(AnswerError::NoUndo);
        }
        let log = &self.engine.log[..self.engine.log.len() - 1];
        let mut engine = Self::deal(&self.spec, &self.decks);
        for &index in log {
            match engine.run() {
                Step::Decision(_) => engine.choose(index),
                Step::Over(_) => unreachable!("the log replays"),
            }
        }
        // The replayed combats were told already.
        engine.combat_notes = Some(Vec::new());
        self.engine = engine;
        self.history.pop();
        self.pending = None;
        self.over = None;
        self.mark = None;
        self.events.clear();
        self.about = None;
        Ok(())
    }

    /// The Methuselah a decision's action is aimed at, once it started one:
    /// what the option's text does not say (a bleed's target).
    fn aim(&self, had_action: bool, choice: &mut OptionView) {
        let Some(a) = self.engine.action().filter(|_| !had_action) else {
            return;
        };
        let at = a.bleed_target.or(match a.target {
            zvtes::cards::Target::Seat(s) => Some(s),
            _ => None,
        });
        let named = choice.target.is_some() || choice.text.contains('→');
        if let Some(seat) = at
            .and_then(|s| self.spec.seats.get(s.index()))
            .filter(|_| !named)
        {
            choice.text += &format!(" → {}", seat.name);
        }
    }

    /// Narrate to no seat: every card shown (a replay read to debug).
    pub fn show_all(&mut self) {
        self.viewer = None;
    }

    /// Install a bot for a seat (a style the session does not build).
    pub fn set_agent(&mut self, seat: usize, agent: Box<dyn Agent>) {
        self.agents[seat] = Some(agent);
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// The decision the last `advance` asked (`None` once over): what the
    /// observation encoders read (`zvtes::obs`) when a game is replayed.
    pub fn request(&self) -> Option<&DecisionRequest> {
        self.pending.as_ref()
    }

    pub fn replay(&self) -> Replay {
        Replay {
            spec: self.spec.clone(),
            log: self.engine.log.clone(),
        }
    }

    /// The table as `seat` sees it (`None`: everything).
    pub fn view(&mut self, seat: Option<SeatId>) -> TableView {
        let tracker = self.trackers.entry(seat.map(|s| s.0)).or_default();
        table_view(&self.engine, seat, tracker)
    }

    fn step(&mut self) {
        if self.pending.is_none() && self.over.is_none() {
            match self.engine.run() {
                Step::Decision(req) => self.pending = Some(req),
                Step::Over(vps) => self.over = Some(vps),
            }
            self.observe(true);
        }
    }

    /// Mark the table, telling what changed since the last mark: how an
    /// action or a referendum ended, and (`ran`: across the engine's run,
    /// not a decision's own effect) the rest.
    fn observe(&mut self, ran: bool) {
        // What combats did first: their damage is among the changes.
        let notes = self
            .engine
            .combat_notes
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default();
        for n in &notes {
            self.events.push(combat_text(&self.engine, n));
        }
        let all = self.view(None);
        let now = Mark::of(&self.engine, &all);
        if let Some(before) = &self.mark {
            let names = self.names();
            before.draws(&now, &all, &names, &mut self.events);
            before.endings(&self.engine, &now, &names, &mut self.events);
            if ran {
                before.changes(&self.engine, &now, &names, &mut self.events);
            }
        }
        self.mark = Some(now);
    }

    fn describer<'a>(
        &'a self,
        req: &DecisionRequest,
        view: &'a TableView,
        narrate: bool,
    ) -> Describer<'a> {
        Describer {
            e: &self.engine,
            seat: req.seat,
            window: req.window,
            view,
            names: &[],
            narrate,
            offered: &[],
            about: match req.window {
                // The political action's card, or what grants the action.
                WindowLabel::ReferendumTerms | WindowLabel::ActionTerms => {
                    self.engine.action().map(|a| action_thing(&self.engine, a))
                }
                WindowLabel::PlayTerms => self
                    .about
                    .as_ref()
                    .filter(|(s, _)| *s == req.seat)
                    .map(|(_, c)| c.clone()),
                _ => None,
            },
        }
    }

    /// After a decision: the card it played or used, kept through the
    /// seat's terms decisions that follow.
    fn remember(&mut self, req: &DecisionRequest, choice: &Choice) {
        let e = &self.engine;
        self.about = match *choice {
            Choice::Play { card, .. } => {
                let played = e.virtual_hand(req.seat)[card as usize].0;
                Some((req.seat, LIBRARY[played as usize].name.to_string()))
            }
            Choice::UseAbility { source, .. } => Some((
                req.seat,
                match source {
                    AbilitySource::Card(c) => LIBRARY[e.state.cards[c.index()].card as usize]
                        .name
                        .to_string(),
                    AbilitySource::Vampire(m) => minion_name(e, m),
                },
            )),
            _ if req.window == WindowLabel::PlayTerms => self.about.take(),
            _ => None,
        };
    }

    fn names(&self) -> Vec<String> {
        self.spec.seats.iter().map(|s| s.name.clone()).collect()
    }

    /// The pending decision's options, each as `Choice`, casting votes
    /// `favor`'s way with the votes each casts: the seat's vampires' and its
    /// calling card's.
    fn casts(&self, req: &DecisionRequest, favor: bool) -> Vec<(usize, u8)> {
        let Some(ctx) = self.engine.referendum() else {
            return Vec::new();
        };
        req.options
            .iter()
            .enumerate()
            .filter_map(|(i, c)| match *c {
                Choice::CastVotes { minion, favor: f } if f == favor => {
                    Some((i, ctx.votes_cast(&self.engine.state, minion, favor)))
                }
                Choice::CastCardVote { favor: f } if f == favor => Some((i, 1)),
                _ => None,
            })
            .collect()
    }

    fn votes(&self, req: &DecisionRequest) -> Option<VoteTotals> {
        let total = |favor| self.casts(req, favor).iter().map(|&(_, n)| n).sum::<u8>();
        let t = VoteTotals {
            favor: total(true),
            against: total(false),
        };
        (t.favor > 0 || t.against > 0).then_some(t)
    }

    fn allocation(&self, req: &DecisionRequest, view: &TableView) -> Option<AllocationView> {
        let a = self.engine.allocation()?;
        // Only while the decision asks for the first recipient.
        let asks = |r: Recipient| {
            req.options.iter().any(|c| match (*c, r) {
                (Choice::AllocateTo { seat, .. }, Recipient::Seat(s)) => seat == s,
                (Choice::AllocateToCard { card, .. }, Recipient::Card(k)) => card == k.0,
                _ => false,
            })
        };
        if req.window != WindowLabel::ReferendumTerms || !asks(a.recipients[0]) {
            return None;
        }
        let names = self.names();
        let mut d = self.describer(req, view, false);
        d.names = &names;
        let recipients = a
            .recipients
            .iter()
            .zip(&a.caps)
            .map(|(&r, &cap)| match r {
                Recipient::Seat(s) => RecipientView {
                    name: d.seat_name(s),
                    seat: Some(s.0),
                    card: None,
                    cap,
                },
                Recipient::Card(c) => RecipientView {
                    name: d.in_play_name(c.0).into(),
                    seat: None,
                    card: Some(d.in_play(c.0)),
                    cap,
                },
            })
            .collect();
        Some(AllocationView {
            points: a.points,
            recipients,
            min_nonzero: a.min_nonzero,
        })
    }

    /// A standing order to vote ends with the referendum's polling.
    fn polling(&self) -> bool {
        self.engine
            .referendum()
            .is_some_and(|r| r.step == RefStep::Polling)
    }

    /// The next message: a human seat's pending decision (repeated until
    /// answered), a bot's decision (taken now), or the game's end.
    pub fn advance(&mut self) -> Out {
        self.step();
        if !self.polling() {
            self.standing.clear();
        }
        let view = self.view(self.viewer);
        let Some(req) = self.pending.clone() else {
            let vps = self.over.clone().unwrap_or_default();
            return Out::Over {
                vps: vps.iter().map(|&v| v as f32 / 2.0).collect(),
                events: std::mem::take(&mut self.events),
                view,
            };
        };
        let decision = self.engine.log.len() as u64;
        let window = window_text(req.window).to_string();
        let names = self.names();
        match self.agents[req.seat.index()].as_mut() {
            None => {
                // The seat's standing order casts its next vote.
                if let Some(&(_, favor)) = self.standing.iter().find(|&&(s, _)| s == req.seat) {
                    if let Some(&(i, _)) = self.casts(&req, favor).first() {
                        return self.answer(decision, i).expect("a vote offered");
                    }
                }
                let mut d = self.describer(&req, &view, false);
                d.names = &names;
                d.offered = &req.options;
                let options = req
                    .options
                    .iter()
                    .enumerate()
                    .map(|(i, c)| d.describe(i, c))
                    .collect();
                Out::Decision {
                    decision,
                    seat: req.seat.0,
                    window,
                    options,
                    undo: self.can_undo(),
                    allocation: self.allocation(&req, &view),
                    votes: self.votes(&req),
                    events: std::mem::take(&mut self.events),
                    view,
                }
            }
            Some(agent) => {
                let idx = agent.decide(&self.engine, &req);
                let mut d = self.describer(&req, &view, true);
                d.names = &names;
                d.offered = &req.options;
                let mut choice = d.describe(idx, &req.options[idx]);
                let had_action = self.engine.action().is_some();
                let events = std::mem::take(&mut self.events);
                self.remember(&req, &req.options[idx]);
                self.engine.choose(idx);
                self.history.push((req.seat, req.window, None));
                self.aim(had_action, &mut choice);
                self.observe(false);
                self.pending = None;
                Out::Decided {
                    decision,
                    seat: req.seat.0,
                    window,
                    choice,
                    events,
                    view,
                }
            }
        }
    }

    /// The pending human decision, if `decision` is its number.
    fn pending_human(&mut self, decision: u64) -> Result<DecisionRequest, AnswerError> {
        self.step();
        let Some(req) = self.pending.clone() else {
            return Err(AnswerError::Over);
        };
        let expected = self.engine.log.len() as u64;
        if decision != expected {
            return Err(AnswerError::Stale { expected });
        }
        if self.agents[req.seat.index()].is_some() {
            return Err(AnswerError::NotYours { seat: req.seat.0 });
        }
        Ok(req)
    }

    /// Answer the pending human decision.
    pub fn answer(&mut self, decision: u64, index: usize) -> Result<Out, AnswerError> {
        let req = self.pending_human(decision)?;
        if index >= req.options.len() {
            return Err(AnswerError::Option {
                options: req.options.len(),
            });
        }
        let view = self.view(self.viewer);
        let names = self.names();
        let mut d = self.describer(&req, &view, true);
        d.names = &names;
        d.offered = &req.options;
        let mut choice = d.describe(index, &req.options[index]);
        let had_action = self.engine.action().is_some();
        let known = knowledge(&self.engine, req.seat);
        let events = std::mem::take(&mut self.events);
        self.remember(&req, &req.options[index]);
        self.engine.choose(index);
        self.history.push((req.seat, req.window, Some(known)));
        self.aim(had_action, &mut choice);
        self.observe(false);
        self.pending = None;
        Ok(Out::Decided {
            decision,
            seat: req.seat.0,
            window: window_text(req.window).to_string(),
            choice,
            events,
            view,
        })
    }

    /// Answer the allocation under way at once (`Out::Decision::allocation`):
    /// `amounts` in its recipients' order. The engine is asked one
    /// recipient at a time — a recipient with a single possible amount is
    /// not asked — so one `Decided` per step asked. Amounts the allocation
    /// does not allow change nothing.
    pub fn allocate(&mut self, decision: u64, amounts: &[u8]) -> Result<Vec<Out>, AnswerError> {
        let req = self.pending_human(decision)?;
        let view = self.view(self.viewer);
        let a = self
            .allocation(&req, &view)
            .ok_or(AnswerError::NoAllocation)?;
        let refuse = |reason: String| Err(AnswerError::Allocation { reason });
        if amounts.len() != a.recipients.len() {
            return refuse(format!("{} recipients", a.recipients.len()));
        }
        for (r, &n) in a.recipients.iter().zip(amounts) {
            if r.cap.is_some_and(|cap| n > cap) {
                return refuse(format!("at most {} to {}", r.cap.unwrap_or(0), r.name));
            }
        }
        let total: u32 = amounts.iter().map(|&n| u32::from(n)).sum();
        if total != u32::from(a.points) {
            return refuse(format!("{} points to allocate, not {total}", a.points));
        }
        let given = amounts.iter().filter(|&&n| n > 0).count();
        if given < usize::from(a.min_nonzero) {
            return refuse(format!(
                "points to at least {} of them, not {given}",
                a.min_nonzero
            ));
        }
        // The steps the engine asks, found on a copy: a valid allocation
        // has one completion when a recipient is not asked.
        let recipients = self.engine.allocation().expect("an allocation").recipients;
        let mut probe = self.engine.clone();
        let mut picks = Vec::new();
        for (&r, &n) in recipients.iter().zip(amounts) {
            let Step::Decision(req) = probe.run() else {
                break;
            };
            let asked = req.options.iter().position(|c| match (*c, r) {
                (Choice::AllocateTo { seat, n: m }, Recipient::Seat(s)) => seat == s && m == n,
                (Choice::AllocateToCard { card, n: m }, Recipient::Card(k)) => {
                    card == k.0 && m == n
                }
                _ => false,
            });
            let other = req.options.iter().any(|c| match (*c, r) {
                (Choice::AllocateTo { seat, .. }, Recipient::Seat(s)) => seat == s,
                (Choice::AllocateToCard { card, .. }, Recipient::Card(k)) => card == k.0,
                _ => false,
            });
            match asked {
                Some(i) => {
                    probe.choose(i);
                    picks.push(i);
                }
                None if other => {
                    return refuse(format!("{n} is not possible for this recipient"));
                }
                // Not asked: its only amount.
                None => {}
            }
        }
        let mut outs = Vec::new();
        for i in picks {
            let decision = self.engine.log.len() as u64;
            outs.push(self.answer(decision, i).expect("checked on a copy"));
        }
        Ok(outs)
    }

    /// Cast every vote the seat's vampires and calling card can cast now
    /// `favor`'s way (`Out::Decision::votes`), and the ones they can cast
    /// later in this polling: one commitment per decision, the others
    /// answered as their turn comes (`advance`). The Edge, cards burned for
    /// a vote, plays and abilities stay the seat's own decisions.
    pub fn vote_all(&mut self, decision: u64, favor: bool) -> Result<Out, AnswerError> {
        let req = self.pending_human(decision)?;
        let Some(&(i, _)) = self.casts(&req, favor).first() else {
            return Err(AnswerError::NoVotes);
        };
        self.standing.retain(|&(s, _)| s != req.seat);
        self.standing.push((req.seat, favor));
        self.answer(decision, i)
    }
}

/// A bot seat whose agent the host has not installed: a host error.
struct Unset;

impl Agent for Unset {
    fn decide(&mut self, _: &Engine, _: &DecisionRequest) -> usize {
        panic!("bot style not installed: Session::set_agent")
    }
}
