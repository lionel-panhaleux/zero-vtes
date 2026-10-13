//! A game between human seats and bots (docs/human-play.md): the engine
//! stepped one decision at a time, bot seats answered inside, every
//! message carrying the viewer's table as of that decision so a client can
//! animate the change, then narrate or ask.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use zvtes::actions::ActionState;
use zvtes::agent::{Agent, RandomLegal};
use zvtes::bots::{Scripted, Style};
use zvtes::cards::{AbilitySource, Effect, Window};
use zvtes::combat::CombatNote;
use zvtes::decision::{Choice, DecisionRequest, WindowLabel, Zone};
use zvtes::game::{Engine, Step};
use zvtes::ids::SeatId;
use zvtes::library::LIBRARY;
use zvtes::referendum::{Recipient, RefStep};

use crate::deck::{resolve, DeckError, DeckList, Resolved};
use crate::describe::{window_text, CardRef, Describer, OptionView};
use crate::logged::{logged, moot, resolve as resolve_logged, Logged};
use crate::narrate::{action_thing, combat_text, minion_name, strike_text, Fact, Mark, Told};
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
        /// What the decision is for, when its options alone do not say: a
        /// referendum's term, a play's second vampire ("Camarilla's Iron
        /// Fist: choose the Methuselah who gains 1 pool").
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prompt: Option<String>,
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

/// A game as a replay: the spec, and every decision's option index and
/// choice (`logged`: what keeps it readable when the engine changes the
/// options it offers; absent from replays made before).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Replay {
    pub spec: GameSpec,
    pub log: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<Logged>,
}

impl Replay {
    /// The replay's decisions read in order, against a game replaying it.
    pub fn reader(&self) -> ReplayReader<'_> {
        ReplayReader {
            replay: self,
            next: 0,
        }
    }

    /// A replay made before `choices`, its choices filled in from its
    /// indices, as far as they read (why they stop, if they do): read on
    /// the engine it was played on (its commit), it reads on later ones
    /// too, up to a change in the rules its game met.
    pub fn with_choices(mut self) -> Result<(Replay, Option<String>), String> {
        let decks: Vec<Resolved> = self
            .spec
            .seats
            .iter()
            .map(|s| resolve(&s.deck).map_err(|e| format!("{e:?}")))
            .collect::<Result<_, _>>()?;
        let mut engine = Session::deal(&self.spec, &decks);
        self.choices.clear();
        for (decision, &index) in self.log.iter().enumerate() {
            let Step::Decision(req) = engine.run() else {
                return Ok((self, Some(format!("decision {decision}: the game is over"))));
            };
            if index >= req.options.len() {
                return Ok((
                    self,
                    Some(format!("decision {decision}: no option {index}")),
                ));
            }
            self.choices.push(logged(&engine, &req, index));
            engine.choose(index);
        }
        Ok((self, None))
    }
}

/// A replay's decisions, answered one at a time as a game replaying it
/// asks them (`Replay::reader`).
pub struct ReplayReader<'a> {
    replay: &'a Replay,
    /// The replay's next decision.
    next: usize,
}

impl ReplayReader<'_> {
    /// The replay's next decision.
    pub fn position(&self) -> usize {
        self.next
    }

    /// The option the replay takes at the decision `req` asks (on `e`, as
    /// it stands then): by its choice, or by its index past the replay's
    /// choices (a replay made before them). `None` when the replay ends;
    /// an error when that option is not offered (the engine changed what
    /// the game asks). A decision the replay's engine did not ask (every
    /// option the same choice) is answered without reading one; one it
    /// asked and this engine does not (`Logged::forced`) is passed over.
    pub fn next(&mut self, e: &Engine, req: &DecisionRequest) -> Option<Result<usize, String>> {
        let r = self.replay;
        loop {
            let Some(l) = r.choices.get(self.next) else {
                let &i = r.log.get(self.next)?;
                self.next += 1;
                return Some(if i < req.options.len() {
                    Ok(i)
                } else {
                    Err(format!("option {i} of {}", req.options.len()))
                });
            };
            if let Some(i) = resolve_logged(e, req, l, r.log.get(self.next).copied()) {
                self.next += 1;
                return Some(Ok(i));
            }
            if l.forced {
                self.next += 1;
                continue;
            }
            if moot(e, req) {
                return Some(Ok(0));
            }
            return Some(Err(format!(
                "seat {}'s {} (cards {:?}) is not offered to seat {}",
                l.seat, l.choice, l.cards, req.seat.0
            )));
        }
    }
}

pub struct Session {
    spec: GameSpec,
    decks: Vec<Resolved>,
    /// Every logged decision by what it chose (`Replay::choices`).
    choices: Vec<Logged>,
    /// Who took each logged decision, in which window, and how (`undo`).
    history: Vec<(SeatId, WindowLabel, Taken)>,
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
    /// Human seats passing on their opportunities (`set_opportunities`).
    quiet: Vec<SeatId>,
    /// The last pair of strikes resolved (`CombatNote::Strikes`): what each
    /// side struck with once the strikes are spent (`view`).
    struck: Option<CombatNote>,
    /// The table at the last decision or message (`narrate`), and what
    /// changed since, told with the next message.
    mark: Option<Mark>,
    events: Vec<String>,
    /// Every fact since the host last took them (`take_told`), with the
    /// decisions taken among them.
    told: Vec<Told>,
    /// The card the last decision played or used, and by whom: what the
    /// seat's terms decisions that follow choose for.
    about: Option<(SeatId, String)>,
}

/// How a logged decision was taken: by a bot, by a human seat (what it
/// knew of the cards when it was asked: `undo_depth`), or by a human seat
/// passing on an opportunity by itself (`set_opportunities`).
enum Taken {
    Bot,
    Human(Knowledge),
    Passed,
    /// A human decision that showed it library cards (a search).
    Revealed,
}

/// An opportunity offered to a seat passing on them (`Session::shown`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Opportunity {
    /// A wake, in its moment or not.
    Wake(bool),
    /// An ability usable at any time.
    AnyTime,
    /// A denial of a block attempt (Stygian Shroud: -1 intercept, or the
    /// attempt fails), needed or not: the attempt meets the stealth.
    Denial(bool),
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
            choices: Vec::new(),
            engine,
            agents,
            viewer,
            trackers: HashMap::new(),
            pending: None,
            over: None,
            standing: Vec::new(),
            quiet: Vec::new(),
            struck: None,
            mark: None,
            events: Vec::new(),
            told: Vec::new(),
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
    /// order to vote; the opportunities it passed on by itself are taken
    /// back with it) and nothing hidden was revealed to it (a draw, a card
    /// turned face up), so taking that decision back teaches it nothing.
    /// How many logged decisions undoing takes back.
    fn undo_depth(&self) -> Option<usize> {
        let req = self.pending.as_ref()?;
        if self.agents[req.seat.index()].is_some()
            || self.standing.iter().any(|&(s, _)| s == req.seat)
        {
            return None;
        }
        for (k, (seat, _, taken)) in self.history.iter().rev().enumerate() {
            if *seat != req.seat {
                return None;
            }
            match taken {
                Taken::Passed => continue,
                // A search showed library cards: no taking it back.
                Taken::Revealed => return None,
                Taken::Human(before) => {
                    return learned_nothing(before, &knowledge(&self.engine, *seat))
                        .then_some(k + 1)
                }
                Taken::Bot => return None,
            }
        }
        None
    }

    fn can_undo(&self) -> bool {
        self.undo_depth().is_some()
    }

    /// Take back the seat's previous decision (`Out::Decision::undo`): the
    /// game is dealt again and replayed to the decision before.
    pub fn undo(&mut self) -> Result<(), AnswerError> {
        self.step();
        let Some(k) = self.undo_depth() else {
            return Err(AnswerError::NoUndo);
        };
        let log = &self.engine.log[..self.engine.log.len() - k];
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
        self.history.truncate(self.history.len() - k);
        self.choices.truncate(self.choices.len() - k);
        self.pending = None;
        self.over = None;
        self.mark = None;
        self.events.clear();
        self.told.clear();
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

    /// Whether the human seats are asked on their opportunities (on by
    /// default; owner, 2026-10-11: "it's very awkward to play with all the
    /// interruptions", "the whole idea is to avoid adding unnecessary
    /// pauses"). Off, an option out of its moment (`shown`) is not offered,
    /// and a decision left with only a pass passes by itself.
    pub fn set_opportunities(&mut self, on: bool) {
        self.quiet.clear();
        if !on {
            self.quiet = (0..self.agents.len())
                .filter(|&i| self.agents[i].is_none())
                .map(|i| SeatId(i as u8))
                .collect();
        }
    }

    /// Whether `c` is an opportunity: a wake (in its moment or not), or an
    /// ability usable at any time (The Louvre, Paris).
    fn opportunity(&self, req: &DecisionRequest, c: &Choice) -> Option<Opportunity> {
        let e = &self.engine;
        let st = &e.state;
        match *c {
            Choice::Play { card, mode, .. } => {
                let played = e.virtual_hand(req.seat)[card as usize].0;
                let wakes = zvtes::cards::lookup(played)
                    .and_then(|d| d.modes.get(mode as usize))
                    .is_some_and(|m| {
                        m.effects
                            .iter()
                            .any(|f| matches!(f, Effect::Wake | Effect::WakeTarget))
                    });
                // A wake's moment: an action directed at the seat, or an
                // undirected one of its prey or predator — never one
                // directed at another seat (owner, 2026-10-11: "not bleeds
                // directed at my cross").
                // Nor as a card is played: legal, rarely of use (owner,
                // 2026-10-12: "more noise than usefulness").
                let moment = req.window != WindowLabel::CardAsPlayed
                    && e.action().is_some_and(|a| match a.directed_seat(st) {
                        Some(target) => target == req.seat,
                        None => a.seat == st.prey(req.seat) || a.seat == st.predator(req.seat),
                    });
                if wakes {
                    return Some(Opportunity::Wake(moment));
                }
                let mode = zvtes::cards::lookup(played).and_then(|d| d.modes.get(mode as usize));
                mode.and_then(|m| self.denial(m.effects))
            }
            Choice::UseAbility {
                source, ability, ..
            } => {
                let mode = source.mode(st, ability);
                if mode.windows.contains(&Window::AnyImpulse) {
                    return Some(Opportunity::AnyTime);
                }
                self.denial(mode.effects)
            }
            _ => None,
        }
    }

    /// `effects` deny the block attempt under way: needed only while the
    /// attempt meets the action's stealth — short of it, it fails anyway
    /// (owner, 2026-10-12: "we should not propose block denials on
    /// attempts not matching stealth with intercept"); the acting seat is
    /// asked again if the blocker gains intercept.
    fn denial(&self, effects: &[Effect]) -> Option<Opportunity> {
        let denies = effects.iter().any(|f| {
            matches!(f, Effect::AttemptFails)
                || matches!(f, Effect::AttemptingIntercept(n) if *n < 0)
        });
        let a = self.engine.action()?;
        let ActionState::Attempt(x) = a.state else {
            return None;
        };
        let st = &self.engine.state;
        denies.then(|| Opportunity::Denial(a.intercept_of(st, x) >= a.stealth(st)))
    }

    /// The options a human seat is shown: all, unless it passes on its
    /// opportunities. Then no wake out of its moment, and the abilities
    /// usable at any time only where the decision pauses anyway (another
    /// option than a pass on offer: its master phase with a master card,
    /// its minion, influence or discard phase...); all if none is left.
    fn shown(&self, req: &DecisionRequest) -> Vec<usize> {
        let all = (0..req.options.len()).collect::<Vec<_>>();
        if !self.quiet.contains(&req.seat) {
            return all;
        }
        let kinds: Vec<_> = req
            .options
            .iter()
            .map(|c| self.opportunity(req, c))
            .collect();
        let mut shown: Vec<usize> = all
            .iter()
            .copied()
            .filter(|&i| {
                kinds[i] != Some(Opportunity::Wake(false))
                    && kinds[i] != Some(Opportunity::Denial(false))
            })
            .collect();
        let pause = shown
            .iter()
            .any(|&i| req.options[i] != Choice::Pass && kinds[i] != Some(Opportunity::AnyTime));
        if !pause {
            shown.retain(|&i| kinds[i] != Some(Opportunity::AnyTime));
        }
        if shown.is_empty() {
            all
        } else {
            shown
        }
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
            choices: self.choices.clone(),
        }
    }

    /// The table as `seat` sees it (`None`: everything).
    pub fn view(&mut self, seat: Option<SeatId>) -> TableView {
        let tracker = self.trackers.entry(seat.map(|s| s.0)).or_default();
        let mut view = table_view(&self.engine, seat, tracker);
        self.struck_view(&mut view);
        self.searched_view(seat, &mut view);
        view
    }

    /// The library cards a decision lets its seat take by name: a library
    /// search, Childe of the Revolution's library (LIBRARY indices).
    fn searched(req: &DecisionRequest) -> Vec<u16> {
        req.options
            .iter()
            .filter_map(|c| match *c {
                Choice::FindCard(card) if req.window == WindowLabel::LibrarySearch => Some(card),
                Choice::TakeFrom {
                    zone: Zone::Library,
                    card,
                } => Some(card),
                _ => None,
            })
            .collect()
    }

    /// The seat's library cards it may take, shown to it while it searches
    /// (owner: "let the player search in the library by clicking it …
    /// reveal only the matching cards"); no undo of that decision then
    /// (`Taken::Revealed`).
    fn searched_view(&self, seat: Option<SeatId>, view: &mut TableView) {
        let (Some(seat), Some(req)) = (seat, self.pending.as_ref()) else {
            return;
        };
        let found = Self::searched(req);
        if req.seat != seat || found.is_empty() {
            return;
        }
        let library = &self.engine.state.seats[seat.index()].library;
        if let Some(s) = view.seats.iter_mut().find(|s| s.seat == seat.0) {
            for (c, &card) in s.library.iter_mut().zip(library) {
                if found.contains(&card) {
                    c.id = Some(LIBRARY[card as usize].id);
                }
            }
        }
    }

    /// A strike spent as it resolved, in the combat's view until the next
    /// pair (owner: hand strikes chosen by themselves, then the prevention
    /// "shows 'no strike' for both"): "hand strike (resolved, 1 damage)".
    fn struck_view(&self, view: &mut TableView) {
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
        ) = (self.engine.combat(), view.combat.as_mut(), self.struck)
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
            if side.chosen && side.strike.is_none() && strikes[j].is_some() {
                let what = strike_text(strikes[j]);
                view.strike = Some(match damage[j] {
                    0 => format!("{what} (resolved)"),
                    n => format!("{what} (resolved, {n} damage)"),
                });
            }
        }
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
        let mut told = Vec::new();
        for n in &notes {
            let text = combat_text(&self.engine, n);
            told.push(Told {
                fact: Fact::Combat(*n),
                text: Some(text),
            });
            if matches!(n, CombatNote::Strikes { .. }) {
                self.struck = Some(*n);
            }
        }
        let now = Mark::of(&self.engine);
        if let Some(before) = &self.mark {
            let names = self.names();
            let e = &self.engine;
            before.draws(&now, &names, &mut told);
            before.attempts(e, &now, &mut told);
            before.strikes(e, &now, &mut told);
            before.endings(e, &now, &names, &mut told);
            before.changes(e, &now, &names, ran, &mut told);
        }
        self.events
            .extend(told.iter().filter_map(|t| t.text.clone()));
        self.told.extend(told);
        self.mark = Some(now);
    }

    fn decided(&mut self, decision: u64) {
        self.told.push(Told {
            fact: Fact::Decided(decision),
            text: None,
        });
    }

    /// The facts since the last call, in order, each decision taken
    /// among them marked (`Fact::Decided`): what the host tells.
    pub fn take_told(&mut self) -> Vec<Told> {
        std::mem::take(&mut self.told)
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
        // Business Pressure's round: one pool at a time, not a vote order.
        if ctx.pressure.is_some() {
            return Vec::new();
        }
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
                let shown = self.shown(&req);
                // Only a pass left: the seat passes on the opportunity.
                if shown.len() < req.options.len()
                    && shown.len() == 1
                    && req.options[shown[0]] == Choice::Pass
                {
                    let out = self.answer(decision, shown[0]).expect("a pass offered");
                    self.history.last_mut().expect("just answered").2 = Taken::Passed;
                    return out;
                }
                let mut d = self.describer(&req, &view, false);
                d.names = &names;
                d.offered = &req.options;
                let options = shown
                    .iter()
                    .map(|&i| d.describe(i, &req.options[i]))
                    .collect();
                let prompt = d.prompt();
                Out::Decision {
                    decision,
                    seat: req.seat.0,
                    window,
                    options,
                    prompt,
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
                self.choices.push(logged(&self.engine, &req, idx));
                let effects = untold_effects(&req.options[idx]);
                self.decided(decision);
                self.engine.choose(idx);
                self.history.push((req.seat, req.window, Taken::Bot));
                self.aim(had_action, &mut choice);
                self.observe(effects);
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
        self.choices.push(logged(&self.engine, &req, index));
        let effects = untold_effects(&req.options[index]);
        self.decided(decision);
        self.engine.choose(index);
        let taken = if Self::searched(&req).is_empty() {
            Taken::Human(known)
        } else {
            Taken::Revealed
        };
        self.history.push((req.seat, req.window, taken));
        self.aim(had_action, &mut choice);
        self.observe(effects);
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

/// A decision whose text does not tell what it did: a rider resolved, an
/// ability used (Fame's pool, Wider View's +2 pool) — its pool and blood
/// changes are told as an engine run's are.
fn untold_effects(c: &Choice) -> bool {
    matches!(
        c,
        Choice::ResolveRider { .. } | Choice::ResolvePrinted { .. } | Choice::UseAbility { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use zvtes::crypt::CRYPT;
    use zvtes::decision::ActionKind;

    /// A plain vampire with Oblivion at superior, capacity 4 or more.
    fn oblivion() -> u16 {
        CRYPT
            .iter()
            .enumerate()
            .find(|(i, c)| {
                c.disciplines.contains(&"OBL")
                    && c.capacity >= 4
                    && zvtes::cards::vampire(*i as u16).is_none()
            })
            .map(|(i, _)| i as u16)
            .expect("an Oblivion vampire")
    }

    /// A vampire with no printed text and no Oblivion, with `with`
    /// ("aus") if given.
    fn plain(n: usize, with: Option<&str>) -> u16 {
        CRYPT
            .iter()
            .enumerate()
            .filter(|(i, c)| {
                !c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("obl"))
                    && with.is_none_or(|w| c.disciplines.contains(&w))
                    && zvtes::cards::vampire(*i as u16).is_none()
            })
            .nth(n)
            .map(|(i, _)| i as u16)
            .expect("a plain vampire")
    }

    /// Stygian Shroud on a block attempt, with the opportunities off:
    /// offered as the attempt meets the stealth (a bleed), not short of it
    /// (a hunt's +1 stealth, the blocker at 0 intercept, Enhanced Senses
    /// unplayed: it fails unless the blocker plays it, and then the acting
    /// seat is asked again).
    #[test]
    fn denials_wait_for_an_attempt_that_meets_the_stealth() {
        let card = |name: &str| LIBRARY.iter().position(|c| c.name == name).unwrap() as u16;
        let shroud = card("Stygian Shroud");
        let decks = crate::deck::catalog();
        let spec = GameSpec {
            seed: 0,
            time_limit: None,
            seats: decks[..3]
                .iter()
                .enumerate()
                .map(|(i, (_, list))| SeatSpec {
                    name: format!("P{i}"),
                    deck: list.clone(),
                    player: Player::Human,
                })
                .collect(),
        };
        for (kind, offered) in [(ActionKind::Bleed, true), (ActionKind::Hunt, false)] {
            let mut s = Session::new(spec.clone()).unwrap();
            let mut e = Engine::bare(3);
            let a = e.add_minion(SeatId(0), oblivion(), 1, false);
            e.add_minion(SeatId(0), plain(0, None), 4, false);
            // Enhanced Senses [aus]: +1 intercept within the blocker's reach.
            let b = e.add_minion(SeatId(1), plain(0, Some("aus")), 4, false);
            e.state.seats[0].hand.push(shroud);
            e.state.seats[1].hand.push(card("Enhanced Senses"));
            s.engine = e;
            s.set_opportunities(true);
            s.set_opportunities(false);
            s.quiet = vec![SeatId(0)];
            let mut req = None;
            for _ in 0..30 {
                let Step::Decision(r) = s.engine.run() else {
                    panic!("over")
                };
                let i = r
                    .options
                    .iter()
                    .position(|c| {
                        matches!(*c, Choice::TakeAction { minion, action, .. } if minion == a && action == kind)
                            || *c == Choice::DeclareBlock { minion: b }
                    })
                    .or_else(|| r.options.iter().position(|c| *c == Choice::Pass));
                if r.window == WindowLabel::ActionBlock
                    && r.seat == SeatId(0)
                    && r.options.iter().any(|c| matches!(c, Choice::Play { .. }))
                {
                    req = Some(r);
                    break;
                }
                s.engine.choose(i.unwrap_or(0));
            }
            let req = req.expect("the acting seat asked on the attempt");
            let play = req
                .options
                .iter()
                .position(|c| matches!(c, Choice::Play { .. }))
                .expect("Stygian Shroud offered by the engine");
            assert_eq!(s.shown(&req).contains(&play), offered, "{kind:?}");
            // A wake of the bleed's target: in its moment during the block
            // loop, not as a card is played (owner, 2026-10-12).
            if kind == ActionKind::Bleed {
                s.engine.state.seats[1].hand.push(card("On the Qui Vive"));
                let wake = Choice::Play {
                    card: s.engine.state.seats[1].hand.len() as u8 - 1,
                    mode: 0,
                    actor: Some(b),
                    target: zvtes::cards::Target::None,
                };
                let asked = |window| DecisionRequest::new(SeatId(1), window, vec![wake]);
                let moment = |window| s.opportunity(&asked(window), &wake);
                assert_eq!(
                    moment(WindowLabel::ActionBlock),
                    Some(Opportunity::Wake(true))
                );
                assert_eq!(
                    moment(WindowLabel::CardAsPlayed),
                    Some(Opportunity::Wake(false))
                );
            }
        }
    }
}
