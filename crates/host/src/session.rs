//! The engine stepped one decision at a time: who decides next (a human
//! seat asked, a bot or a standing order answering), the opportunities a
//! human seat passes on, allocations and vote orders answered whole, undo
//! by snapshots, and the facts the table's changes tell (`narrate`). No
//! words: the game names and describes (`game.rs`, `describe.rs`).

use std::collections::HashMap;

use zvtes::actions::ActionState;
use zvtes::agent::Agent;
use zvtes::cards::{Effect, Window};
use zvtes::combat::CombatNote;
use zvtes::decision::{Choice, DecisionRequest, WindowLabel, Zone};
use zvtes::game::{Engine, Step};
use zvtes::ids::SeatId;
use zvtes::referendum::{Recipient, RefStep};

use crate::deck::Resolved;
use crate::narrate::{Fact, Mark, Told};
use crate::numbering::Decks;
use crate::view::Viewer;

/// Engines kept for undo: past this many decisions in a row, the oldest
/// is dealt again and replayed instead.
const SNAPSHOTS: usize = 64;

/// How a decision was taken: by a bot, by a human seat (what it knew of
/// the cards when it was asked), by a human seat passing on an
/// opportunity by itself, or as the rest of a command answered whole (a
/// vote order's casts, an allocation's later steps): undone with it.
#[derive(Clone, Debug, PartialEq)]
pub enum Taken {
    Bot,
    Human(Knowledge),
    Passed,
    Order,
    /// A decision that showed library or crypt cards: a human's search, or
    /// any decision after which a card was revealed to all (no undo back
    /// past it).
    Revealed,
}

/// The cards a seat can see, by (crypt, KRCG id): what undo must not
/// let it learn and forget.
pub type Knowledge = HashMap<(bool, u32), usize>;

/// What the game does next.
pub enum Next {
    /// A human seat decides: the options it is shown (indices), whether
    /// its previous decision can be taken back.
    Ask {
        shown: Vec<usize>,
        undo: bool,
    },
    /// A decision taken without asking: a bot's pick, a standing order's
    /// vote, a lone pass (`Taken::Passed`). Apply it with `take`.
    Take {
        index: usize,
        taken: Taken,
    },
    Over,
}

/// An opportunity offered to a seat passing on them (`shown`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Opportunity {
    /// A wake, in its moment or not.
    Wake(bool),
    /// An ability usable at any time.
    AnyTime,
    /// A denial of a block attempt (Stygian Shroud: -1 intercept, or the
    /// attempt fails), needed or not: the attempt meets the stealth.
    Denial(bool),
    /// A legal no-op: an ability gaining nothing now (`Engine::gains_nothing`:
    /// Powerbase: Madrid's burn with no counter, votes for a vampire that
    /// cannot vote; owner, 2026-10-14: rules-qa 73.3, 73.4).
    Nothing,
    /// A block attempt that cannot reach the action's stealth
    /// (`Engine::block_can_reach`; rules-qa 48.5, 66.2, 73.8).
    Futile,
    /// The acting Methuselah's options other than the pass while the attempt
    /// under way is short of the stealth: the blocker's to answer and the
    /// table's to help (rules-qa 82.1).
    Unmatched,
}

/// Why an answer is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    Stale { expected: u64 },
    NotYours,
    Option,
    Over,
    NoUndo,
    NoAllocation,
    Allocation,
    NoVotes,
}

pub struct Session {
    seed: u64,
    time_limit: Option<u32>,
    decks: Vec<Resolved>,
    engine: Engine,
    agents: Vec<Option<Box<dyn Agent>>>,
    pending: Option<DecisionRequest>,
    over: Option<Vec<u32>>,
    /// Seats casting all their votes one way (`vote_all`), for the rest of
    /// the referendum's polling.
    standing: Vec<(SeatId, bool)>,
    /// Human seats passing on their opportunities (`set_opportunities`).
    quiet: Vec<SeatId>,
    /// The decision a human seat was asked, and the options it was shown:
    /// asked again as it was until taken (a settings change applies from
    /// the next decision, never passing one already shown).
    asked: Option<(u64, Vec<usize>)>,
    /// The last pair of strikes resolved: what each side struck with once
    /// the strikes are spent (the view's `resolved`).
    struck: Option<CombatNote>,
    /// The table at the last decision or message, and the facts since.
    mark: Option<Mark>,
    told: Vec<Told>,
    /// Who took each decision, and how (`undo`).
    history: Vec<(SeatId, Taken)>,
    /// The engine before each human decision undo may take back, by the
    /// history's length then: restored, not replayed. Cleared when a bot
    /// decides; the oldest dropped past `SNAPSHOTS`.
    snapshots: Vec<(usize, Engine)>,
    /// The copies dealt (for a seat's knowledge, `undo`).
    dealt: Decks,
}

/// The game as dealt: from the seed, nothing decided yet.
pub fn deal(seed: u64, time_limit: Option<u32>, decks: &[Resolved]) -> Engine {
    let lists: Vec<_> = decks
        .iter()
        .map(|d| (d.crypt.as_slice(), d.library.as_slice()))
        .collect();
    let mut engine = Engine::with_lists(seed, &lists);
    if let Some(t) = time_limit {
        engine.time_limit = t;
    }
    // Combats, reveals and plays told as they happen (`observe`).
    engine.combat_notes = Some(Vec::new());
    engine.reveal_notes = Some(Vec::new());
    engine.play_notes = Some(Vec::new());
    engine
}

impl Session {
    /// `agents[seat]`: the bot answering it, `None` for a human seat.
    pub fn new(
        seed: u64,
        time_limit: Option<u32>,
        decks: Vec<Resolved>,
        agents: Vec<Option<Box<dyn Agent>>>,
    ) -> Session {
        let engine = deal(seed, time_limit, &decks);
        let dealt = Decks::dealt(&engine);
        Session {
            seed,
            time_limit,
            decks,
            engine,
            agents,
            pending: None,
            over: None,
            standing: Vec::new(),
            quiet: Vec::new(),
            asked: None,
            struck: None,
            mark: None,
            told: Vec::new(),
            history: Vec::new(),
            snapshots: Vec::new(),
            dealt,
        }
    }

    /// Keep the engine's short declines (`Engine::short_declines`), for a
    /// record from before rules-qa 82.1.
    pub(crate) fn keep_short_declines(&mut self) {
        self.engine.short_declines = Some(Vec::new());
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// The decision pending (`None` once over).
    pub fn request(&self) -> Option<&DecisionRequest> {
        self.pending.as_ref()
    }

    /// The victory points (in halves) once over.
    pub fn over(&self) -> Option<&[u32]> {
        self.over.as_deref()
    }

    /// The pending decision's number.
    pub fn decision(&self) -> u64 {
        self.engine.log.len() as u64
    }

    /// The last pair of strikes resolved.
    pub fn struck(&self) -> Option<CombatNote> {
        self.struck
    }

    /// Whether the human seats are asked on their opportunities (on by
    /// default; owner, 2026-10-11: "it's very awkward to play with all the
    /// interruptions"). Off, an option out of its moment is not shown, and
    /// a decision left with only a pass passes by itself.
    pub fn set_opportunities(&mut self, on: bool) {
        self.quiet.clear();
        if !on {
            self.quiet = (0..self.agents.len())
                .filter(|&i| self.agents[i].is_none())
                .map(|i| SeatId(i as u8))
                .collect();
        }
    }

    /// The facts since the last call, in order, each decision taken among
    /// them marked (`Fact::Decided`).
    pub fn take_told(&mut self) -> Vec<Told> {
        std::mem::take(&mut self.told)
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
    /// not a decision's own effect) the rest in words.
    fn observe(&mut self, ran: bool) {
        // What combats did, the cards revealed and the plays finished, in
        // their order, first: their damage is among the changes. The
        // queues are drained (and reset, `undo`) together: `after` counts
        // the combat notes before, `PlayNote::reveals` the reveals.
        let notes = self
            .engine
            .combat_notes
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default();
        let reveals = self
            .engine
            .reveal_notes
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default();
        let plays = self
            .engine
            .play_notes
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default();
        let revealed = !reveals.is_empty();
        let mut reveals = reveals.into_iter().peekable();
        let mut plays = plays.into_iter().peekable();
        let mut shown = 0;
        let mut told = Vec::new();
        for i in 0..=notes.len() {
            loop {
                if let Some(p) = plays.next_if(|p| p.after <= i && p.reveals <= shown) {
                    told.push(Told {
                        fact: Fact::Played(p),
                        told: true,
                    });
                } else if let Some(r) = reveals.next_if(|r| r.after <= i) {
                    shown += 1;
                    told.push(Told {
                        fact: Fact::Revealed {
                            seat: r.seat,
                            card: r.card,
                            crypt: r.crypt,
                        },
                        told: true,
                    });
                } else {
                    break;
                }
            }
            let Some(n) = notes.get(i) else { break };
            told.push(Told {
                fact: Fact::Combat(*n),
                told: true,
            });
            if matches!(n, CombatNote::Strikes { .. }) {
                self.struck = Some(*n);
            }
        }
        // A card revealed: no undo back past it (it would be known and
        // its place restored).
        if revealed {
            if let Some(last) = self.history.last_mut() {
                last.1 = Taken::Revealed;
            }
            self.snapshots.clear();
        }
        let now = Mark::of(&self.engine);
        if let Some(before) = &self.mark {
            let e = &self.engine;
            before.draws(&now, &mut told);
            before.attempts(e, &now, &mut told);
            before.strikes(&now, &mut told);
            before.endings(e, &now, &mut told);
            before.changes(e, &now, ran, &mut told);
        }
        self.told.extend(told);
        self.mark = Some(now);
    }

    /// A standing order to vote ends with the referendum's polling.
    fn polling(&self) -> bool {
        self.engine
            .referendum()
            .is_some_and(|r| r.step == RefStep::Polling)
    }

    /// What the game does next: the engine run to its next decision, then
    /// a human seat asked, or a decision taken without asking (`take` it),
    /// or the end.
    pub fn advance(&mut self) -> Next {
        self.step();
        if !self.polling() {
            self.standing.clear();
        }
        let Some(req) = self.pending.clone() else {
            return Next::Over;
        };
        match self.agents[req.seat.index()].as_mut() {
            Some(agent) => Next::Take {
                index: agent.decide(&self.engine, &req),
                taken: Taken::Bot,
            },
            None => {
                // The seat's standing order casts its next vote.
                if let Some(&(_, favor)) = self.standing.iter().find(|&&(s, _)| s == req.seat) {
                    if let Some(&(i, _)) = self.casts(&req, favor).first() {
                        return Next::Take {
                            index: i,
                            taken: Taken::Order,
                        };
                    }
                }
                if let Some((d, shown)) = &self.asked {
                    if *d == self.decision() {
                        return Next::Ask {
                            shown: shown.clone(),
                            undo: self.undo_depth().is_some(),
                        };
                    }
                }
                let shown = self.shown(&req);
                // Only a pass left: the seat passes on the opportunity.
                if shown.len() < req.options.len()
                    && shown.len() == 1
                    && req.options[shown[0]] == Choice::Pass
                {
                    return Next::Take {
                        index: shown[0],
                        taken: Taken::Passed,
                    };
                }
                self.asked = Some((self.decision(), shown.clone()));
                Next::Ask {
                    shown,
                    undo: self.undo_depth().is_some(),
                }
            }
        }
    }

    /// The engine run to its next decision, if it has not been.
    pub fn ready(&mut self) {
        self.step();
    }

    /// A human seat's answer to the pending decision `decision`: checked,
    /// not applied (`take`).
    pub fn check(&mut self, decision: u64, index: usize) -> Result<(), Refused> {
        self.step();
        let Some(req) = self.pending.as_ref() else {
            return Err(Refused::Over);
        };
        let expected = self.decision();
        if decision != expected {
            return Err(Refused::Stale { expected });
        }
        if self.agents[req.seat.index()].is_some() {
            return Err(Refused::NotYours);
        }
        if index >= req.options.len() {
            return Err(Refused::Option);
        }
        Ok(())
    }

    /// The human taking option `index` of the pending decision: what it
    /// knew then, for undo.
    pub fn human(&self) -> Taken {
        let req = self.pending.as_ref().expect("a decision pending");
        if searched(req).is_empty() {
            Taken::Human(self.knowledge(req.seat))
        } else {
            Taken::Revealed
        }
    }

    /// Apply option `index` of the pending decision, taken as `taken`.
    pub fn take(&mut self, index: usize, taken: Taken) {
        let req = self.pending.take().expect("a decision pending");
        self.asked = None;
        let decision = self.decision();
        self.told.push(Told {
            fact: Fact::Decided(decision),
            told: false,
        });
        let said = !untold_effects(&req.options[index]);
        match taken {
            Taken::Bot => self.snapshots.clear(),
            Taken::Revealed => self.snapshots.clear(),
            _ => {
                if self.snapshots.len() == SNAPSHOTS {
                    self.snapshots.remove(0);
                }
                self.snapshots
                    .push((self.history.len(), self.engine.clone()));
            }
        }
        self.engine.choose(index);
        self.history.push((req.seat, taken));
        self.observe(!said);
    }

    /// The seat's knowledge: the cards its view shows, by id.
    fn knowledge(&self, seat: SeatId) -> Knowledge {
        let mut viewer = Viewer::new(Some(seat));
        let names = vec![String::new(); self.engine.state.seats.len()];
        let view = viewer.view(&self.engine, &self.dealt, &names);
        // Every card the view names: an object with a copy and an id.
        fn walk(v: &serde_json::Value, k: &mut Knowledge) {
            match v {
                serde_json::Value::Object(o) => {
                    if let (Some(_), Some(id)) =
                        (o.get("copy"), o.get("id").and_then(|i| i.as_u64()))
                    {
                        let crypt = o.get("crypt").and_then(|c| c.as_bool()).unwrap_or(false);
                        *k.entry((crypt, id as u32)).or_default() += 1;
                    }
                    o.values().for_each(|v| walk(v, k));
                }
                serde_json::Value::Array(a) => a.iter().for_each(|v| walk(v, k)),
                _ => {}
            }
        }
        let mut k = Knowledge::new();
        walk(
            &serde_json::to_value(&view).expect("views serialize"),
            &mut k,
        );
        k
    }

    /// How many decisions undoing takes back: the pending decision is the
    /// seat's again, right after its previous one, nothing decided since
    /// (by a bot, or a standing order; the opportunities it passed on by
    /// itself go back with it) and nothing hidden revealed to it.
    fn undo_depth(&self) -> Option<usize> {
        let req = self.pending.as_ref()?;
        if self.agents[req.seat.index()].is_some()
            || self.standing.iter().any(|&(s, _)| s == req.seat)
        {
            return None;
        }
        for (k, (seat, taken)) in self.history.iter().rev().enumerate() {
            if *seat != req.seat {
                return None;
            }
            match taken {
                Taken::Passed | Taken::Order => continue,
                Taken::Revealed | Taken::Bot => return None,
                Taken::Human(before) => {
                    let now = self.knowledge(*seat);
                    let learned = now
                        .iter()
                        .any(|(card, &n)| before.get(card).copied().unwrap_or(0) < n);
                    return (!learned).then_some(k + 1);
                }
            }
        }
        None
    }

    /// Take back the seat's previous decision: the engine as it was
    /// before it (kept; dealt again and replayed when a long run of
    /// decisions dropped it). The number of decisions taken back.
    pub fn undo(&mut self) -> Result<usize, Refused> {
        self.step();
        let Some(k) = self.undo_depth() else {
            return Err(Refused::NoUndo);
        };
        self.asked = None;
        let at = self.history.len() - k;
        let kept = self.snapshots.iter().position(|(n, _)| *n == at);
        let mut engine = match kept {
            Some(i) => {
                let (_, e) = self.snapshots.swap_remove(i);
                self.snapshots.truncate(i);
                e
            }
            None => {
                self.snapshots.clear();
                let log = &self.engine.log[..self.engine.log.len() - k];
                let mut engine = deal(self.seed, self.time_limit, &self.decks);
                for &index in log {
                    match engine.run() {
                        Step::Decision(_) => engine.choose(index),
                        Step::Over(_) => unreachable!("the log replays"),
                    }
                }
                engine
            }
        };
        // The combats, reveals and plays since were told already.
        engine.combat_notes = Some(Vec::new());
        engine.reveal_notes = Some(Vec::new());
        engine.play_notes = Some(Vec::new());
        self.engine = engine;
        self.history.truncate(at);
        self.pending = None;
        self.over = None;
        self.mark = None;
        self.told.clear();
        Ok(k)
    }

    /// Whether `c` is an opportunity: a wake (in its moment or not), an
    /// ability usable at any time (The Louvre, Paris), a block denial, a
    /// legal no-op, a futile block attempt, the acting seat's option against
    /// a short attempt.
    fn opportunity(&self, req: &DecisionRequest, c: &Choice) -> Option<Opportunity> {
        let e = &self.engine;
        let st = &e.state;
        // A legal no-op (Gathii's reveal before stealth is needed, owner
        // 2026-10-17: "really an uninteresting cornercase").
        if e.gains_nothing(req.seat, c) {
            return Some(Opportunity::Nothing);
        }
        if let Some(a) = e.action() {
            if let ActionState::Attempt(x) = a.state {
                if req.seat == a.seat
                    && req.window == WindowLabel::ActionBlock
                    && *c != Choice::Pass
                    && a.intercept_of(st, x) < a.stealth(st)
                {
                    return Some(Opportunity::Unmatched);
                }
            }
        }
        match *c {
            Choice::Play { card, mode, .. } => {
                let played = e.virtual_hand(req.seat)[card as usize].0;
                let mode = zvtes::cards::lookup(played).and_then(|d| d.modes.get(mode as usize));
                let wakes = mode.is_some_and(|m| {
                    m.effects
                        .iter()
                        .any(|f| matches!(f, Effect::Wake | Effect::WakeTarget))
                });
                // A wake's moment: an action directed at the seat, or an
                // undirected one of its prey or predator — never one
                // directed at another seat (owner, 2026-10-11), nor as a
                // card is played (owner, 2026-10-12: "more noise than
                // usefulness") — and while blocks are open, only the seat's
                // own block moment, not before the would-be blockers ahead
                // of it declined nor after its own decline (owner,
                // 2026-10-17: "really misleading, even though technically
                // correct"); a wake to react once blocks are declined
                // stays.
                let blocking = e.action().is_some_and(|a| {
                    matches!(a.state, ActionState::Open | ActionState::Attempt(_))
                });
                let moment = req.window != WindowLabel::CardAsPlayed
                    && e.action().is_some_and(|a| match a.directed_seat(st) {
                        Some(target) => target == req.seat,
                        None => a.seat == st.prey(req.seat) || a.seat == st.predator(req.seat),
                    })
                    && (!blocking || e.block_moment(req.seat));
                if wakes {
                    return Some(Opportunity::Wake(moment));
                }
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
            Choice::DeclareBlock { minion } if !e.block_can_reach(req.seat, minion) => {
                Some(Opportunity::Futile)
            }
            _ => None,
        }
    }

    /// `effects` deny the block attempt under way: needed only while the
    /// attempt meets the action's stealth (owner, 2026-10-12: "we should
    /// not propose block denials on attempts not matching stealth with
    /// intercept"); the acting seat is asked again if the blocker gains
    /// intercept.
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
    /// opportunities. Then no wake out of its moment, no denial short of
    /// the stealth, no legal no-op, no block attempt short of it, none of the
    /// acting seat's options but the pass while the attempt under way is
    /// short of it, and the
    /// abilities usable at any time only where the decision pauses anyway;
    /// all if none is left.
    pub(crate) fn shown(&self, req: &DecisionRequest) -> Vec<usize> {
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
                    && kinds[i] != Some(Opportunity::Nothing)
                    && kinds[i] != Some(Opportunity::Futile)
                    && kinds[i] != Some(Opportunity::Unmatched)
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

    /// The pending decision's options casting votes `favor`'s way, with
    /// the votes each casts: the seat's vampires' and its calling card's.
    pub fn casts(&self, req: &DecisionRequest, favor: bool) -> Vec<(usize, u8)> {
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

    /// The votes castable now each way (`None`: none).
    pub fn votes(&self, req: &DecisionRequest) -> Option<(u8, u8)> {
        let total = |favor| self.casts(req, favor).iter().map(|&(_, n)| n).sum::<u8>();
        let t = (total(true), total(false));
        (t.0 > 0 || t.1 > 0).then_some(t)
    }

    /// Cast every vote of the seat's `favor`'s way, now and as its turn
    /// comes in this polling: the first cast's option (`take` it as
    /// `Taken::Order`… the first is the seat's own answer).
    pub fn vote_all(&mut self, decision: u64, favor: bool) -> Result<usize, Refused> {
        self.check(decision, 0)?;
        let req = self.pending.as_ref().expect("checked");
        let Some(&(i, _)) = self.casts(req, favor).first() else {
            return Err(Refused::NoVotes);
        };
        let seat = req.seat;
        self.standing.retain(|&(s, _)| s != seat);
        self.standing.push((seat, favor));
        Ok(i)
    }

    /// The allocation under way, while the decision asks its first
    /// recipient: (points, recipients with caps, at least how many given
    /// some).
    pub fn allocation(&self, req: &DecisionRequest) -> Option<zvtes::referendum::Allocation> {
        let a = self.engine.allocation()?;
        let asks = |r: Recipient| {
            req.options.iter().any(|c| match (*c, r) {
                (Choice::AllocateTo { seat, .. }, Recipient::Seat(s)) => seat == s,
                (Choice::AllocateToCard { card, .. }, Recipient::Card(k)) => card == k.0,
                _ => false,
            })
        };
        (req.window == WindowLabel::ReferendumTerms && asks(a.recipients[0])).then_some(a)
    }

    /// The options answering the allocation under way with `amounts` (in
    /// its recipients' order), one per step the engine asks — a recipient
    /// with a single possible amount is not asked — found on a copy.
    pub fn allocate(&mut self, decision: u64, amounts: &[u8]) -> Result<Vec<usize>, Refused> {
        self.check(decision, 0)?;
        let req = self.pending.clone().expect("checked");
        let a = self.allocation(&req).ok_or(Refused::NoAllocation)?;
        if amounts.len() != a.recipients.len()
            || a.recipients
                .iter()
                .zip(&a.caps)
                .zip(amounts)
                .any(|((_, cap), &n)| cap.is_some_and(|c| n > c))
            || amounts.iter().map(|&n| u32::from(n)).sum::<u32>() != u32::from(a.points)
            || amounts.iter().filter(|&&n| n > 0).count() < usize::from(a.min_nonzero)
        {
            return Err(Refused::Allocation);
        }
        let mut probe = self.engine.clone();
        let mut picks = Vec::new();
        for (&r, &n) in a.recipients.iter().zip(amounts) {
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
                None if other => return Err(Refused::Allocation),
                // Not asked: its only amount.
                None => {}
            }
        }
        Ok(picks)
    }
}

/// The library cards a decision lets its seat take by name: a library
/// search, Childe of the Revolution's library (LIBRARY indices).
pub fn searched(req: &DecisionRequest) -> Vec<u16> {
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

/// A decision whose option does not tell what it did: a rider resolved,
/// an ability used (Fame's pool, Wider View's +2 pool) — its pool and
/// blood changes are told in words as an engine run's are.
fn untold_effects(c: &Choice) -> bool {
    matches!(
        c,
        Choice::ResolveRider { .. } | Choice::ResolvePrinted { .. } | Choice::UseAbility { .. }
    )
}

#[cfg(test)]
mod tests {
    use zvtes::cards::{AbilitySource, Target};
    use zvtes::copies::CopyId;
    use zvtes::decision::ActionKind;
    use zvtes::decks::Deck;
    use zvtes::state::{find_card, find_library, CardId, CardInPlay};

    use super::*;

    /// A session over three precon decks, its engine to be replaced.
    fn session() -> (Session, Vec<(u16, u8)>) {
        let decks: Vec<Resolved> = (0..3)
            .map(|i| {
                let d = Deck::precon(i);
                Resolved {
                    crypt: d.crypt.to_vec(),
                    library: d.library.to_vec(),
                }
            })
            .collect();
        let crypt = decks[0].crypt.clone();
        let s = Session::new(1, None, decks, (0..3).map(|_| None).collect());
        (s, crypt)
    }

    /// A card an effect reveals is told to every seat with its id, no copy
    /// number (it stays in its hidden place): a crypt's top, as Mesu
    /// Bedshet reveals it (the engine's `reveal_notes`).
    #[test]
    fn a_revealed_card_is_told_to_every_seat() {
        let (mut s, _) = session();
        s.observe(false);
        let top = *s.engine.state.seats[0].crypt.last().expect("a crypt");
        s.engine
            .reveal_notes
            .as_mut()
            .expect("reveals noted")
            .push(zvtes::game::Reveal {
                seat: SeatId(0),
                card: top,
                crypt: true,
                after: 0,
            });
        s.observe(true);
        let fact = s
            .told
            .iter()
            .map(|t| t.fact)
            .find(|f| matches!(f, Fact::Revealed { .. }))
            .expect("the reveal observed");
        for seat in 0..3 {
            let mut v = crate::view::Viewer::new(Some(SeatId(seat)));
            let told = v.event(&s.engine, &s.dealt, fact);
            let Some(zvtes_api::event::EventKind::Revealed { seat: from, card }) = told else {
                panic!("seat {seat}: {told:?}");
            };
            assert_eq!(from, 0);
            assert_eq!(
                card.id,
                Some(zvtes::crypt::CRYPT[top as usize].id),
                "seat {seat}"
            );
            assert!(card.crypt && card.copy.is_none(), "seat {seat}: {card:?}");
        }
    }

    /// The game to the next human decision a seat can take back: its
    /// first options taken until one is undoable.
    fn to_undoable(s: &mut Session) {
        for _ in 0..200 {
            match s.advance() {
                Next::Ask { undo: true, .. } => return,
                Next::Ask { shown, .. } => {
                    let t = s.human();
                    s.take(shown[0], t);
                }
                Next::Take { index, taken } => s.take(index, taken),
                Next::Over => panic!("over"),
            }
        }
        panic!("no undoable decision");
    }

    /// No undo past a revealed card (its place restored, the card known);
    /// and the replay an undo deals again tells no reveal anew.
    #[test]
    fn a_reveal_bars_the_undo() {
        let (mut s, _) = session();
        to_undoable(&mut s);
        let top = *s.engine.state.seats[0].crypt.last().expect("a crypt");
        s.engine
            .reveal_notes
            .as_mut()
            .expect("reveals noted")
            .push(zvtes::game::Reveal {
                seat: SeatId(0),
                card: top,
                crypt: true,
                after: 0,
            });
        s.observe(true);
        assert!(matches!(s.undo(), Err(Refused::NoUndo)), "barred");
    }

    /// Reveals told in their order among a combat's notes (`Reveal::after`:
    /// the combat notes before it; both queues drained together).
    #[test]
    fn a_reveal_is_told_in_its_place_among_the_combat_notes() {
        let (mut s, _) = session();
        s.observe(false);
        let m = [zvtes::ids::MinionId(0), zvtes::ids::MinionId(1)];
        s.engine.combat_notes = Some(vec![
            CombatNote::Begins { minions: m },
            CombatNote::Ends { minions: m },
        ]);
        let top = *s.engine.state.seats[0].crypt.last().expect("a crypt");
        s.engine.reveal_notes = Some(vec![zvtes::game::Reveal {
            seat: SeatId(0),
            card: top,
            crypt: true,
            after: 1,
        }]);
        let before = s.told.len();
        s.observe(true);
        let order: Vec<&str> = s.told[before..]
            .iter()
            .filter_map(|t| match t.fact {
                Fact::Combat(CombatNote::Begins { .. }) => Some("begins"),
                Fact::Revealed { .. } => Some("revealed"),
                Fact::Combat(CombatNote::Ends { .. }) => Some("ends"),
                _ => None,
            })
            .collect();
        assert_eq!(order, vec!["begins", "revealed", "ends"]);
    }

    /// Plays finished are told in their place among the combat notes and
    /// the reveals: one noted after a reveal follows it, one before the
    /// next note precedes that.
    #[test]
    fn a_play_is_told_in_its_place_among_the_notes() {
        let (mut s, _) = session();
        s.observe(false);
        let m = [zvtes::ids::MinionId(0), zvtes::ids::MinionId(1)];
        s.engine.combat_notes = Some(vec![
            CombatNote::Begins { minions: m },
            CombatNote::Ends { minions: m },
        ]);
        let top = *s.engine.state.seats[0].crypt.last().expect("a crypt");
        s.engine.reveal_notes = Some(vec![zvtes::game::Reveal {
            seat: SeatId(0),
            card: top,
            crypt: true,
            after: 1,
        }]);
        let play = |after, reveals| zvtes::game::PlayNote {
            seat: SeatId(0),
            card: zvtes::state::find_library("Villein"),
            copy: CopyId::NONE,
            mode: 0,
            actor: None,
            target: Target::None,
            canceled: false,
            no_effect: false,
            after,
            reveals,
        };
        s.engine.play_notes = Some(vec![play(1, 0), play(1, 1)]);
        let before = s.told.len();
        s.observe(true);
        let order: Vec<&str> = s.told[before..]
            .iter()
            .filter_map(|t| match t.fact {
                Fact::Combat(CombatNote::Begins { .. }) => Some("begins"),
                Fact::Revealed { .. } => Some("revealed"),
                Fact::Played(_) => Some("played"),
                Fact::Combat(CombatNote::Ends { .. }) => Some("ends"),
                _ => None,
            })
            .collect();
        assert_eq!(
            order,
            vec!["begins", "played", "revealed", "played", "ends"]
        );
    }

    /// A legal no-op is hidden with the opportunities off, shown with them
    /// on: Powerbase: Madrid's burn with no counter (rules-qa
    /// 73.3), shown again once the card has a counter.
    #[test]
    fn a_legal_no_op_is_hidden_with_the_opportunities_off() {
        let (mut s, crypt) = session();
        let mut e = Engine::bare(3);
        let m = e.add_minion(SeatId(0), crypt[0].0, 4, false);
        e.add_minion(SeatId(0), crypt[1].0, 4, false);
        e.add_minion(SeatId(1), crypt[2].0, 4, false);
        let madrid = find_library("Powerbase: Madrid");
        e.state.cards.push(CardInPlay::entering(
            madrid,
            CopyId::NONE,
            SeatId(1),
            None,
            0,
        ));
        let id = CardId(e.state.cards.len() as u32 - 1);
        s.engine = e;
        let Step::Decision(req) = s.engine.run() else {
            panic!("a decision");
        };
        let burn = Choice::UseAbility {
            source: AbilitySource::Card(id),
            ability: 2,
            actor: Some(m),
            target: Target::None,
        };
        let i = req
            .options
            .iter()
            .position(|o| *o == burn)
            .expect("the burn offered");
        assert!(
            s.shown(&req).contains(&i),
            "shown with the opportunities on"
        );
        s.set_opportunities(false);
        let shown = s.shown(&req);
        assert!(!shown.contains(&i), "hidden with them off");
        assert!(shown.len() > 1, "the rest shown: {shown:?}");
        s.engine.state.cards[id.index()].counters = 1;
        assert!(s.shown(&req).contains(&i), "a counter to burn: shown");
    }

    /// A block attempt that cannot reach the stealth is offered (rules-qa
    /// 73.8), hidden with the opportunities off: a hunt (+1 stealth)
    /// against a 0-intercept vampire whose Methuselah holds no card; shown
    /// again once an intercept card is in hand, a library to draw from
    /// (48.5).
    #[test]
    fn a_futile_block_attempt_is_hidden_with_the_opportunities_off() {
        let (mut s, _) = session();
        let mut e = Engine::bare(3);
        let h = e.add_minion(SeatId(0), find_card("Madison"), 4, false);
        e.add_minion(SeatId(0), find_card("Crystal Lynn"), 4, false);
        let z = e.add_minion(SeatId(1), find_card("Donny Kowalczyk"), 4, false);
        s.engine = e;
        let Step::Decision(req) = s.engine.run() else {
            panic!("a decision");
        };
        let hunt = Choice::TakeAction {
            minion: h,
            action: ActionKind::Hunt,
            target: None,
        };
        let i = req.options.iter().position(|o| *o == hunt).expect("hunt");
        s.engine.choose(i);
        let Step::Decision(req) = s.engine.run() else {
            panic!("a decision");
        };
        assert_eq!(req.window, WindowLabel::ActionBlock);
        let block = Choice::DeclareBlock { minion: z };
        let i = req
            .options
            .iter()
            .position(|o| *o == block)
            .expect("the attempt offered");
        assert!(!s.engine.block_can_reach(req.seat, z));
        assert!(
            s.shown(&req).contains(&i),
            "shown with the opportunities on"
        );
        s.set_opportunities(false);
        let shown = s.shown(&req);
        assert!(!shown.contains(&i), "hidden with them off");
        assert_eq!(
            shown.iter().map(|&j| req.options[j]).collect::<Vec<_>>(),
            [Choice::Pass]
        );
        let senses = find_library("Enhanced Senses");
        s.engine.state.seats[1].hand.put(senses, CopyId::NONE);
        s.engine.state.seats[1].library.put(senses, CopyId::NONE);
        assert!(s.shown(&req).contains(&i), "it may reach: shown");
    }
    /// Shown with the opportunities on, hidden with them off.
    fn hidden_off(s: &mut Session, req: &DecisionRequest, want: Choice) {
        let i = req
            .options
            .iter()
            .position(|o| *o == want)
            .unwrap_or_else(|| panic!("{want:?} offered in {:?}", req.options));
        s.set_opportunities(true);
        assert!(s.shown(req).contains(&i), "shown with the opportunities on");
        s.set_opportunities(false);
        assert!(!s.shown(req).contains(&i), "hidden with them off");
        s.set_opportunities(true);
    }

    /// Stealth no attempt needs is hidden with the opportunities off
    /// (owner, 2026-10-17): Gathii's ability during his bleed, no block
    /// attempt under way.
    #[test]
    fn stealth_no_attempt_needs_is_hidden_with_the_opportunities_off() {
        let (mut s, _) = session();
        let mut e = Engine::bare(3);
        let g = e.add_minion(SeatId(0), find_card("Gathii"), 4, false);
        e.add_minion(SeatId(0), find_card("Madison"), 4, false);
        e.add_minion(SeatId(1), find_card("Donny Kowalczyk"), 4, false);
        s.engine = e;
        let Step::Decision(req) = s.engine.run() else {
            panic!("a decision");
        };
        let bleed = Choice::TakeAction {
            minion: g,
            action: ActionKind::Bleed,
            target: None,
        };
        let i = req.options.iter().position(|o| *o == bleed).expect("bleed");
        s.engine.choose(i);
        let Step::Decision(req) = s.engine.run() else {
            panic!("a decision");
        };
        assert_eq!(
            (req.seat, req.window),
            (SeatId(0), WindowLabel::ActionBlock)
        );
        let reveal = Choice::UseAbility {
            source: AbilitySource::Vampire(g),
            ability: 0,
            actor: None,
            target: Target::None,
        };
        hidden_off(&mut s, &req, reveal);
    }

    /// The acting Methuselah's play while the block attempt under way is
    /// short of the stealth is hidden with the opportunities off (rules-qa
    /// 82.1): Revelation of the Serpent's bleed during Donny's attempt
    /// against Form of the Cobra's stealth.
    #[test]
    fn an_acting_play_against_a_short_attempt_is_hidden_with_the_opportunities_off() {
        let (mut s, _) = session();
        let mut e = Engine::bare(3);
        let p = e.add_minion(SeatId(0), find_card("Crystal Lynn"), 4, false);
        e.add_minion(SeatId(0), find_card("Madison"), 4, false);
        let z = e.add_minion(SeatId(1), find_card("Donny Kowalczyk"), 4, false);
        for c in ["Form of the Cobra", "Revelation of the Serpent"] {
            e.state.seats[0].hand.put(find_library(c), CopyId::NONE);
        }
        s.engine = e;
        let bleed = Choice::TakeAction {
            minion: p,
            action: ActionKind::Bleed,
            target: None,
        };
        let cobra = Choice::Play {
            card: 0,
            mode: 1,
            actor: Some(p),
            target: Target::None,
        };
        let block = Choice::DeclareBlock { minion: z };
        let mut req = None;
        for _ in 0..20 {
            let Step::Decision(r) = s.engine.run() else {
                panic!("a decision");
            };
            let attempt = s
                .engine
                .action()
                .is_some_and(|a| matches!(a.state, ActionState::Attempt(_)));
            if attempt && r.seat == SeatId(0) {
                req = Some(r);
                break;
            }
            let i = [bleed, cobra, block]
                .iter()
                .find_map(|w| r.options.iter().position(|o| o == w))
                .unwrap_or(0);
            s.engine.choose(i);
        }
        let req = req.expect("the acting seat asked during the attempt");
        let a = s.engine.action().unwrap();
        assert!(a.intercept_of(&s.engine.state, z) < a.stealth(&s.engine.state));
        let revelation = *req
            .options
            .iter()
            .find(|o| matches!(o, Choice::Play { .. }))
            .expect("Revelation of the Serpent offered");
        hidden_off(&mut s, &req, revelation);
    }

    /// A wake is shown in the seat's own block moment only (owner,
    /// 2026-10-17): On the Qui Vive for the predator's locked vampire is
    /// hidden while the prey's attempt is under way, the predator's turn to
    /// block not come.
    #[test]
    fn a_wake_waits_for_the_seats_own_block_moment() {
        let (mut s, _) = session();
        let mut e = Engine::bare(3);
        let h = e.add_minion(SeatId(0), find_card("Madison"), 4, false);
        e.add_minion(SeatId(0), find_card("Crystal Lynn"), 4, false);
        let z = e.add_minion(SeatId(1), find_card("Donny Kowalczyk"), 4, false);
        let w = e.add_minion(SeatId(2), find_card("Malachi"), 4, false);
        e.state.minions[w.index()].locked = true;
        e.state.seats[2]
            .hand
            .put(find_library("On the Qui Vive"), CopyId::NONE);
        s.engine = e;
        let Step::Decision(req) = s.engine.run() else {
            panic!("a decision");
        };
        let hunt = Choice::TakeAction {
            minion: h,
            action: ActionKind::Hunt,
            target: None,
        };
        let i = req.options.iter().position(|o| *o == hunt).expect("hunt");
        s.engine.choose(i);
        // The prey attempts; the impulse goes round the table.
        let mut asked = 0;
        let req = loop {
            asked += 1;
            assert!(asked < 40, "seat 2 never asked");
            let Step::Decision(req) = s.engine.run() else {
                panic!("a decision");
            };
            if let Some(i) = req
                .options
                .iter()
                .position(|o| *o == Choice::DeclareBlock { minion: z })
            {
                s.engine.choose(i);
                continue;
            }
            if req.seat == SeatId(2) {
                break req;
            }
            let pass = req
                .options
                .iter()
                .position(|o| *o == Choice::Pass)
                .expect("pass");
            s.engine.choose(pass);
        };
        assert!(s.engine.block_moment(SeatId(1)));
        assert!(!s.engine.block_moment(SeatId(2)));
        let wake = req
            .options
            .iter()
            .copied()
            .find(|o| matches!(o, Choice::Play { actor: Some(a), .. } if *a == w))
            .expect("On the Qui Vive offered");
        hidden_off(&mut s, &req, wake);
        // The prey's attempt fails, the prey declines: the predator's
        // moment, its wake shown with the opportunities off.
        let pass = |req: &DecisionRequest| {
            req.options
                .iter()
                .position(|o| *o == Choice::Pass)
                .expect("pass")
        };
        s.engine.choose(pass(&req));
        let mut asked = 0;
        let req = loop {
            asked += 1;
            assert!(asked < 40, "seat 2's moment never came");
            let Step::Decision(req) = s.engine.run() else {
                panic!("a decision");
            };
            if req.seat == SeatId(2) && s.engine.block_moment(SeatId(2)) {
                break req;
            }
            s.engine.choose(pass(&req));
        };
        let i = req
            .options
            .iter()
            .position(|o| *o == wake)
            .expect("the wake again");
        s.set_opportunities(false);
        assert!(s.shown(&req).contains(&i), "shown in its moment");
    }

    /// Once blocks are declined, a wake to react stays shown with the
    /// opportunities off (a locked vampire waking to play a reaction): the
    /// block moment filters only while blocks are open.
    #[test]
    fn a_wake_after_blocks_are_declined_is_shown() {
        let (mut s, _) = session();
        let mut e = Engine::bare(3);
        let p = e.add_minion(SeatId(0), find_card("Madison"), 4, false);
        e.add_minion(SeatId(0), find_card("Crystal Lynn"), 4, false);
        let z = e.add_minion(SeatId(1), find_card("Donny Kowalczyk"), 4, false);
        e.state.minions[z.index()].locked = true;
        e.state.seats[1]
            .hand
            .put(find_library("On the Qui Vive"), CopyId::NONE);
        s.engine = e;
        let Step::Decision(req) = s.engine.run() else {
            panic!("a decision");
        };
        let bleed = Choice::TakeAction {
            minion: p,
            action: ActionKind::Bleed,
            target: None,
        };
        let i = req.options.iter().position(|o| *o == bleed).expect("bleed");
        s.engine.choose(i);
        s.set_opportunities(false);
        let wake = |req: &DecisionRequest| {
            req.options
                .iter()
                .position(|o| matches!(o, Choice::Play { actor: Some(a), .. } if *a == z))
        };
        let (mut open, mut declined) = (false, false);
        for _ in 0..40 {
            let Step::Decision(req) = s.engine.run() else {
                break;
            };
            if req.seat == SeatId(1) {
                if let Some(i) = wake(&req) {
                    let state = s.engine.action().map(|a| a.state);
                    match state {
                        Some(ActionState::Open) => open |= s.shown(&req).contains(&i),
                        Some(ActionState::Declined) => declined |= s.shown(&req).contains(&i),
                        _ => {}
                    }
                }
            }
            if declined || req.window == WindowLabel::MinionPhase {
                break;
            }
            let pass = req
                .options
                .iter()
                .position(|o| *o == Choice::Pass)
                .expect("pass");
            s.engine.choose(pass);
        }
        assert!(open, "shown in its block moment");
        assert!(declined, "shown once blocks are declined");
    }
}
