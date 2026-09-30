//! A game between human seats and bots (docs/human-play.md): the engine
//! stepped one decision at a time, bot seats answered inside, every
//! message carrying the viewer's table as of that decision so a client can
//! animate the change, then narrate or ask.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use zvtes::agent::{Agent, RandomLegal};
use zvtes::bots::{Scripted, Style};
use zvtes::decision::{DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::SeatId;

use crate::deck::{resolve, DeckError, DeckList, Resolved};
use crate::describe::{window_text, Describer, OptionView};
use crate::track::Tracker;
use crate::view::{table_view, TableView};

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
        /// a step of the choice under way (terms, an allocation).
        undo: bool,
        view: TableView,
    },
    /// A decision was taken (by a bot, or a human's answer), as the
    /// viewer may know it; `view` is the table it was taken on.
    Decided {
        decision: u64,
        seat: u8,
        window: String,
        choice: OptionView,
        view: TableView,
    },
    /// The game ended: victory points per seat.
    Over { vps: Vec<f32>, view: TableView },
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
    /// Who took each logged decision, in which window.
    history: Vec<(SeatId, WindowLabel)>,
    engine: Engine,
    agents: Vec<Option<Box<dyn Agent>>>,
    /// Who the session narrates to: the first human seat (`None`: all
    /// bots, everything shown).
    viewer: Option<SeatId>,
    trackers: HashMap<Option<u8>, Tracker>,
    pending: Option<DecisionRequest>,
    over: Option<Vec<u32>>,
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
        engine
    }

    /// The pending decision continues a multi-step choice of the same seat
    /// (the terms of a play or of a referendum, an allocation, a
    /// distribution, the influence phase's transfers): its previous step
    /// can be taken back. Nothing hidden is learned between such steps.
    fn can_undo(&self) -> bool {
        let multi_step = |w: WindowLabel| {
            matches!(
                w,
                WindowLabel::PlayTerms
                    | WindowLabel::ReferendumTerms
                    | WindowLabel::Distribution
                    | WindowLabel::ActionTerms
                    | WindowLabel::RiderTerms
            )
        };
        match (&self.pending, self.history.last()) {
            (Some(req), Some(&(seat, window))) => {
                req.seat == seat
                    && req.window == window
                    && multi_step(window)
                    && self.agents[seat.index()].is_none()
            }
            _ => false,
        }
    }

    /// Take back the previous step of the multi-step choice under way
    /// (`Out::Decision::undo`): the game is dealt again and replayed to the
    /// decision before — the bots decided nothing since.
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
        self.engine = engine;
        self.history.pop();
        self.pending = None;
        self.over = None;
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
        let named = choice.target.is_some();
        if let Some(seat) = at
            .and_then(|s| self.spec.seats.get(s.index()))
            .filter(|_| !named)
        {
            choice.text += &format!(" → {}", seat.name);
        }
    }

    /// Install a bot for a seat (a style the session does not build).
    pub fn set_agent(&mut self, seat: usize, agent: Box<dyn Agent>) {
        self.agents[seat] = Some(agent);
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
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
        }
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
        }
    }

    fn names(&self) -> Vec<String> {
        self.spec.seats.iter().map(|s| s.name.clone()).collect()
    }

    /// The next message: a human seat's pending decision (repeated until
    /// answered), a bot's decision (taken now), or the game's end.
    pub fn advance(&mut self) -> Out {
        self.step();
        let view = self.view(self.viewer);
        let Some(req) = self.pending.clone() else {
            let vps = self.over.clone().unwrap_or_default();
            return Out::Over {
                vps: vps.iter().map(|&v| v as f32 / 2.0).collect(),
                view,
            };
        };
        let decision = self.engine.log.len() as u64;
        let window = window_text(req.window).to_string();
        let names = self.names();
        match self.agents[req.seat.index()].as_mut() {
            None => {
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
                self.engine.choose(idx);
                self.history.push((req.seat, req.window));
                self.aim(had_action, &mut choice);
                self.pending = None;
                Out::Decided {
                    decision,
                    seat: req.seat.0,
                    window,
                    choice,
                    view,
                }
            }
        }
    }

    /// Answer the pending human decision.
    pub fn answer(&mut self, decision: u64, index: usize) -> Result<Out, AnswerError> {
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
        self.engine.choose(index);
        self.history.push((req.seat, req.window));
        self.aim(had_action, &mut choice);
        self.pending = None;
        Ok(Out::Decided {
            decision,
            seat: req.seat.0,
            window: window_text(req.window).to_string(),
            choice,
            view,
        })
    }
}

/// A bot seat whose agent the host has not installed: a host error.
struct Unset;

impl Agent for Unset {
    fn decide(&mut self, _: &Engine, _: &DecisionRequest) -> usize {
        panic!("bot style not installed: Session::set_agent")
    }
}
