//! The engine stepped one decision at a time: who decides next (a human
//! seat asked, a bot or a standing order answering), the opportunities a
//! human seat passes on, allocations and vote orders answered whole, undo
//! by snapshots, and the facts the table's changes tell (`narrate`). No
//! words: the game names and describes (`game.rs`, `describe.rs`).

use std::collections::HashMap;

use zvtes::copies::{CopyId, Place};

use zvtes::actions::{ActionState, Outcome};
use zvtes::agent::Agent;
use zvtes::cards::{Effect, ModeDef, Window};
use zvtes::combat::{CombatNote, Step as CombatStep};
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel, Zone};
use zvtes::game::{Engine, Step};
use zvtes::ids::SeatId;
use zvtes::notes::Note;
use zvtes::referendum::{Recipient, RefStep};

use crate::deck::Resolved;
use crate::narrate::{changes, ActionFact, Fact, Mark, Told};
use crate::numbering::Decks;
use crate::view::Viewer;

/// Engines kept for undo: past this many human decisions, the oldest is
/// dealt again and replayed instead, or, against bots, no longer undone
/// (a replay would not have the bots answer as they did).
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
    /// past it). Never against bots (`Session::solo`).
    Revealed,
}

/// The game before a human decision, for undo: the history's length
/// then, the engine, the vote orders, (`Session::solo`) each bot's copy
/// and the strikes the view shows resolved.
struct Snapshot {
    at: usize,
    engine: Engine,
    standing: Vec<(SeatId, bool)>,
    bots: Vec<Option<Box<dyn Agent>>>,
    /// The last pair of strikes resolved then (`Session::struck`).
    struck: Option<CombatNote>,
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
    /// A strike's damage made aggravated (Claws of the Dead, Bone Spur), in
    /// its moment or not: the strikes chosen, before they resolve.
    Aggravation(bool),
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
    /// The order `vote_all` gives, standing once its decision is taken
    /// (after the snapshot: an undo there must not bring it back).
    ordered: Option<(SeatId, bool)>,
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
    /// Each copy's last place, as the facts told it.
    places: HashMap<CopyId, Place>,
    told: Vec<Told>,
    /// Who took each decision, and how (`undo`).
    history: Vec<(SeatId, Taken)>,
    /// The engine before each human decision undo may take back, by the
    /// history's length then: restored, not replayed; with the bots as they
    /// stood then in a game against bots (`solo`). Cleared when a bot
    /// decides (but in such a game); the oldest dropped past `SNAPSHOTS`.
    snapshots: Vec<Snapshot>,
    /// One human seat (the bots copied by `Agent::fork`): undo takes back
    /// the human's previous decision whatever was decided or revealed
    /// since (owner, 2026-10-18: "cancel anything anytime").
    solo: bool,
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
    // What happens told as it happens (`observe`).
    engine.notes = Some(Vec::new());
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
        let solo = agents.iter().filter(|a| a.is_none()).count() == 1;
        Session {
            seed,
            time_limit,
            decks,
            engine,
            agents,
            pending: None,
            over: None,
            standing: Vec::new(),
            ordered: None,
            quiet: Vec::new(),
            asked: None,
            struck: None,
            mark: None,
            places: HashMap::new(),
            told: Vec::new(),
            history: Vec::new(),
            snapshots: Vec::new(),
            solo,
            dealt,
        }
    }

    /// Keep the engine's short declines (`Engine::short_declines`), for a
    /// record from before rules-qa 82.1.
    pub(crate) fn keep_short_declines(&mut self) {
        self.engine.short_declines = Some(Vec::new());
    }

    /// Keep the engine's elided Passes (`Engine::elided_passes`), for a
    /// record read back.
    pub(crate) fn keep_elided_passes(&mut self) {
        self.engine.elided_passes = Some(Vec::new());
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// The engine, to set a position up (tests).
    #[cfg(test)]
    pub(crate) fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
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

    /// Mark the table, telling what happened since the last mark: the
    /// engine's notes in their order, each change to the table told by
    /// the ledger after it, with its cause (`ran`: across the engine's
    /// run, not a decision's own effect, in words), then the strikes
    /// declared in the combat still under way.
    fn observe(&mut self, ran: bool) {
        let notes = self.engine.take_notes().unwrap_or_default();
        let now = Mark::of(&self.engine);
        let mut told = Vec::new();
        let mut revealed = false;
        let mut table = self.mark.as_ref().map(|m| &m.ledger);
        let mut open = self.mark.as_ref().and_then(|m| m.action);
        for n in &notes {
            let fact = match n {
                Note::Ledger { ledger, cause } => {
                    let seen = &mut self.places;
                    match table {
                        Some(was) => {
                            let e = &self.engine;
                            changes(e, (was, ledger), *cause, ran, open, seen, &mut told);
                        }
                        None => seen.extend(ledger.census.iter().map(|&(c, _, p)| (c, p))),
                    }
                    table = Some(ledger);
                    continue;
                }
                Note::Action(a) => {
                    let action = ActionFact::of(&a.action);
                    open = (!a.ended).then_some(action);
                    match a.ended {
                        false => Fact::ActionStarts(action),
                        true => {
                            let outcome = a.action.outcome.filter(|_| !a.ousted);
                            Fact::ActionEnds {
                                action,
                                outcome,
                                ousted: a.ousted,
                                blocker: a
                                    .action
                                    .blocker
                                    .filter(|_| outcome == Some(Outcome::Blocked)),
                            }
                        }
                    }
                }
                Note::Acting { seat, minion } => {
                    if let Some(a) = open.as_mut() {
                        a.acting = *minion;
                    }
                    Fact::ActingChanges {
                        seat: *seat,
                        minion: *minion,
                    }
                }
                Note::Play(p) => Fact::Played(*p),
                Note::Reveal(r) => {
                    revealed = true;
                    Fact::Revealed {
                        seat: r.seat,
                        card: r.card,
                        crypt: r.crypt,
                    }
                }
                Note::Look(l) => {
                    revealed = true;
                    Fact::Looked(l.clone())
                }
                Note::Combat(c) => {
                    if matches!(c, CombatNote::Strikes { .. }) {
                        self.struck = Some(*c);
                    }
                    Fact::Combat(*c)
                }
            };
            told.push(Told::new(fact, true));
        }
        // The strikes declared in the combat still under way.
        if let Some(before) = &self.mark {
            before.strikes(&now, &mut told);
        }
        // A card revealed or a hand looked at: no undo back past it (it
        // would be known and its place restored), but against bots.
        if revealed && !self.solo {
            if let Some(last) = self.history.last_mut() {
                last.1 = Taken::Revealed;
            }
            self.snapshots.clear();
        }
        if !told.is_empty() {
            let faces = crate::narrate::faces(&self.engine);
            for t in &mut told {
                t.faces = faces.clone();
            }
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
    /// knew then, for undo (nothing against bots: an undo there does not
    /// ask).
    pub fn human(&self) -> Taken {
        let req = self.pending.as_ref().expect("a decision pending");
        if self.solo {
            Taken::Human(Knowledge::new())
        } else if searched(req).is_empty() {
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
        self.told.push(Told::new(Fact::Decided(decision), false));
        let said = !untold_effects(&req.options[index]);
        match taken {
            Taken::Bot if self.solo => {}
            Taken::Bot | Taken::Revealed => self.snapshots.clear(),
            // Where an undo can land: a human's decision of its own.
            Taken::Human(_) => {
                if self.snapshots.len() == SNAPSHOTS {
                    self.snapshots.remove(0);
                }
                let bots = match self.solo {
                    true => self
                        .agents
                        .iter()
                        .map(|a| a.as_ref().map(|a| a.fork()))
                        .collect(),
                    false => Vec::new(),
                };
                self.snapshots.push(Snapshot {
                    at: self.history.len(),
                    engine: self.engine.clone(),
                    standing: self.standing.clone(),
                    bots,
                    struck: self.struck,
                });
            }
            Taken::Passed | Taken::Order => {}
        }
        if let Some((seat, favor)) = self.ordered.take() {
            self.standing.retain(|&(s, _)| s != seat);
            self.standing.push((seat, favor));
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
    /// itself go back with it) and nothing hidden revealed to it; against
    /// bots (`solo`), back to its previous one whatever came since.
    fn undo_depth(&self) -> Option<usize> {
        let req = self.pending.as_ref()?;
        if self.agents[req.seat.index()].is_some()
            || (!self.solo && self.standing.iter().any(|&(s, _)| s == req.seat))
        {
            return None;
        }
        // Against bots: back to the seat's previous decision of its own,
        // the bots', the passes and the orders since with it, while its
        // snapshot is kept (the bots answering again as they did).
        if self.solo {
            let k =
                1 + self.history.iter().rev().position(|(seat, taken)| {
                    *seat == req.seat && matches!(taken, Taken::Human(_))
                })?;
            let at = self.history.len() - k;
            return self.snapshots.iter().any(|s| s.at == at).then_some(k);
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
    /// decisions dropped it, but against bots: then no undo). The number of
    /// decisions taken back.
    pub fn undo(&mut self) -> Result<usize, Refused> {
        self.step();
        let Some(k) = self.undo_depth() else {
            return Err(Refused::NoUndo);
        };
        self.asked = None;
        let at = self.history.len() - k;
        let kept = self.snapshots.iter().position(|s| s.at == at);
        let mut engine = match kept {
            Some(i) => {
                let s = self.snapshots.swap_remove(i);
                self.snapshots.truncate(i);
                self.standing = s.standing;
                self.struck = s.struck;
                for (seat, bot) in s.bots.into_iter().enumerate() {
                    if bot.is_some() {
                        self.agents[seat] = bot;
                    }
                }
                s.engine
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
                // The strikes the view shows resolved, as replayed.
                self.struck = engine
                    .take_notes()
                    .unwrap_or_default()
                    .into_iter()
                    .rev()
                    .find_map(|n| match n {
                        Note::Combat(c @ CombatNote::Strikes { .. }) => Some(c),
                        _ => None,
                    });
                engine
            }
        };
        // The combats, reveals, looks, plays and actions since were told already.
        engine.notes = Some(Vec::new());
        self.engine = engine;
        self.history.truncate(at);
        self.pending = None;
        self.over = None;
        // The table as restored: what runs from here is told against it.
        let mark = Mark::of(&self.engine);
        self.places = mark.ledger.census.iter().map(|&(c, _, p)| (c, p)).collect();
        self.mark = Some(mark);
        self.told.clear();
        Ok(k)
    }

    /// Whether `c` is an opportunity: a wake (in its moment or not), an
    /// ability usable at any time (The Louvre, Paris), a block denial, a
    /// legal no-op, a futile block attempt, the acting seat's option against
    /// a short attempt, a strike's damage made aggravated (in its moment or
    /// not).
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
                // usefulness") unless the woken minion could then cancel
                // it (`wake_cancels`) — and while blocks are open, only
                // the seat's own block moment, not before the would-be
                // blockers ahead of it declined nor after its own decline
                // (owner, 2026-10-17: "really misleading, even though
                // technically correct"); once blocks are declined, only on
                // a bleed at the seat, to wake and bounce it (owner,
                // 2026-10-19: "otherwise it's noise"); never in a
                // referendum (owner, 2026-10-19: "many times during a
                // referendum. Shouldn't").
                if wakes && req.window == WindowLabel::CardAsPlayed {
                    return Some(Opportunity::Wake(self.wake_cancels(req, c)));
                }
                let blocking = e.action().is_some_and(|a| {
                    matches!(a.state, ActionState::Open | ActionState::Attempt(_))
                });
                let declined = e.action().is_some_and(|a| a.state == ActionState::Declined);
                let moment = e.referendum().is_none()
                    && e.action().is_some_and(|a| match a.directed_seat(st) {
                        Some(target) => target == req.seat,
                        None => a.seat == st.prey(req.seat) || a.seat == st.predator(req.seat),
                    })
                    && (!blocking || e.block_moment(req.seat))
                    && (!declined
                        || e.action().is_some_and(|a| {
                            a.kind == ActionKind::Bleed && a.directed_seat(st) == Some(req.seat)
                        }));
                if wakes {
                    return Some(Opportunity::Wake(moment));
                }
                mode.and_then(|m| self.aggravation(m).or_else(|| self.denial(m.effects)))
            }
            Choice::UseAbility {
                source, ability, ..
            } => {
                let mode = source.mode(st, ability);
                if mode.windows.contains(&Window::AnyImpulse) {
                    return Some(Opportunity::AnyTime);
                }
                self.aggravation(mode).or_else(|| self.denial(mode.effects))
            }
            Choice::DeclareBlock { minion } if !e.block_can_reach(req.seat, minion) => {
                Some(Opportunity::Futile)
            }
            _ => None,
        }
    }

    /// The wake `c`, in a card's as-played window, lets the woken
    /// minion cancel that card (rules-qa 96.4: Rewind Time, Touch of
    /// Clarity): the seat is then offered a play canceling it (owner,
    /// 2026-10-25).
    fn wake_cancels(&self, req: &DecisionRequest, c: &Choice) -> bool {
        let Some(i) = req.options.iter().position(|o| o == c) else {
            return false;
        };
        // Held now: the replacement the wake draws as it is played is not
        // known yet, and the wake shown would tell it.
        let held: Vec<u16> = (self.engine.virtual_hand(req.seat).iter())
            .map(|&(card, _)| card)
            .collect();
        let mut probe = self.engine.clone();
        probe.choose(i);
        // The other seats pass on the wake as it is played (its own
        // as-played window): whether they could answer it is theirs.
        let mut next = None;
        for _ in 0..16 {
            let Step::Decision(asked) = probe.run() else {
                return false;
            };
            if asked.seat == req.seat {
                next = Some(asked);
                break;
            }
            let Some(pass) = asked.options.iter().position(|o| *o == Choice::Pass) else {
                return false;
            };
            probe.choose(pass);
        }
        let Some(next) = next else {
            return false;
        };
        next.window == WindowLabel::CardAsPlayed
            && next.options.iter().any(|o| {
                let Choice::Play { card, mode, .. } = *o else {
                    return false;
                };
                let played = probe.virtual_hand(next.seat)[card as usize].0;
                held.contains(&played)
                    && zvtes::cards::lookup(played)
                        .and_then(|d| d.modes.get(mode as usize))
                        .is_some_and(|m| {
                            m.effects
                                .iter()
                                .any(|f| matches!(f, Effect::CancelPlayed { .. }))
                        })
            })
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

    /// `mode` makes a strike's damage aggravated and is usable once the
    /// strikes are chosen: offered only then, before they resolve, not at
    /// every untimed step (owner, 2026-10-18: "offer to make damage
    /// aggravated only after choosing strike ... right now it's offered at
    /// many different points during combat (which is correct and legal but
    /// noisy)"). A mode not usable then (Raking Talons, before range) keeps
    /// its moments.
    fn aggravation(&self, mode: &ModeDef) -> Option<Opportunity> {
        let aggravates = mode.effects.iter().any(|f| {
            matches!(
                f,
                Effect::AggravatedHands | Effect::AggravatedHandsThisCombat
            )
        });
        let then = mode
            .windows
            .iter()
            .any(|w| matches!(w, Window::CombatUntimed | Window::CombatBeforeResolution));
        let c = self.engine.combat()?;
        (aggravates && then).then_some(Opportunity::Aggravation(
            c.step == CombatStep::BeforeResolution,
        ))
    }

    /// The options a human seat is shown: all, unless it passes on its
    /// opportunities. Then no wake out of its moment, no denial short of
    /// the stealth, no legal no-op, no block attempt short of it, none of the
    /// acting seat's options but the pass while the attempt under way is
    /// short of it, no aggravation before the strikes are chosen, and the
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
                    && kinds[i] != Some(Opportunity::Aggravation(false))
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
    /// comes in this polling: the first cast's option, to `take` as the
    /// seat's own answer (`human()`); the order stands from then on, its
    /// later casts `Taken::Order`.
    pub fn vote_all(&mut self, decision: u64, favor: bool) -> Result<usize, Refused> {
        self.check(decision, 0)?;
        let req = self.pending.as_ref().expect("checked");
        let Some(&(i, _)) = self.casts(req, favor).first() else {
            return Err(Refused::NoVotes);
        };
        self.ordered = Some((req.seat, favor));
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
    use zvtes::decision::{ActionKind, WindowLabel};
    use zvtes::decks::Deck;
    use zvtes::ids::MinionId;
    use zvtes::state::{find_card, find_library, CardId, CardInPlay};
    use zvtes_api::event::EventKind;

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

    /// A bare table where seat 0 announces a withdrawal at its unlock
    /// phase (its library and hand empty), seat 2's vampire bleeding it
    /// after if `bleed`; the events a viewer is told until seat 0 is out of
    /// the game or its next turn comes, and the last view.
    fn withdrawal_told(bleed: bool) -> (Vec<EventKind>, zvtes_api::view::TableView) {
        let (mut s, crypt) = session();
        let mut e = Engine::bare(3);
        for (seat, &(card, _)) in crypt.iter().take(3).enumerate() {
            e.add_minion(SeatId(seat as u8), card, 4, false);
        }
        e.notes = Some(Vec::new());
        s.engine = e;
        s.observe(false);
        let names = vec![String::new(); 3];
        let mut v = crate::view::Viewer::new(None);
        v.view(&s.engine, &s.dealt, &names);
        let mut told = Vec::new();
        let (mut announced, mut bled) = (false, false);
        for _ in 0..100 {
            let Step::Decision(req) = s.engine.run() else {
                panic!("a decision");
            };
            s.observe(true);
            told.extend(
                s.take_told()
                    .iter()
                    .filter_map(|t| v.event(&s.engine, &s.dealt, t)),
            );
            let at = |c: &Choice| req.options.iter().position(|o| o == c);
            let bleeds = Choice::TakeAction {
                minion: MinionId(2),
                action: ActionKind::Bleed,
                target: None,
            };
            if s.engine.state.seats[0].withdrew {
                break;
            }
            let i = match req.seat {
                SeatId(0) if !announced => at(&Choice::WithdrawFromGame),
                SeatId(0) if req.window == WindowLabel::UnlockAbilities => break,
                SeatId(2) if bleed && announced && !bled => at(&bleeds),
                _ => None,
            };
            announced |= i.is_some() && req.seat == SeatId(0);
            bled |= i.is_some() && req.seat == SeatId(2);
            s.engine
                .choose(i.or_else(|| at(&Choice::Pass)).unwrap_or(0));
        }
        let view = v.view(&s.engine, &s.dealt, &names);
        (told, view)
    }

    /// A blocker whose Methuselah a combat card's cost ousted (Pack Alpha
    /// employing a Dog Pack with the last pool) is named nowhere after: the
    /// action's end tells no blocker, no view an unnumbered one.
    #[test]
    fn a_blocker_ousted_with_its_methuselah_is_told_as_none() {
        let (mut s, _) = session();
        let vampire = |clan: &str, disc: &str| {
            (0..zvtes::crypt::CRYPT.len() as u16)
                .find(|&i| {
                    let c = &zvtes::crypt::CRYPT[i as usize];
                    c.clan == clan
                        && c.capacity >= 4
                        && c.disciplines.contains(&disc)
                        && zvtes::cards::vampire(i).is_none()
                })
                .expect("a vampire")
        };
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), vampire("Brujah", "pot"), 4, false);
        e.add_minion(SeatId(0), vampire("Brujah", "pot"), 4, false);
        let b = e.add_minion(SeatId(1), vampire("Gangrel", "ani"), 4, false);
        e.add_minion(SeatId(2), vampire("Ventrue", "dom"), 4, false);
        for name in ["Pack Alpha", "Dog Pack"] {
            e.state.seats[1].hand.push(find_library(name));
        }
        e.state.seats[1].pool = 2;
        for (i, m) in e.state.minions.iter_mut().enumerate() {
            m.copy = zvtes::copies::Untracked(CopyId(i as u16 + 1));
        }
        e.notes = Some(Vec::new());
        s.engine = e;
        s.observe(false);
        let names = vec![String::new(); 3];
        let mut v = crate::view::Viewer::new(None);
        v.view(&s.engine, &s.dealt, &names);
        let mut told = Vec::new();
        for _ in 0..40 {
            let Step::Decision(req) = s.engine.run() else {
                panic!("a decision");
            };
            s.observe(true);
            told.extend(
                s.take_told()
                    .iter()
                    .filter_map(|t| v.event(&s.engine, &s.dealt, t)),
            );
            let view = v.view(&s.engine, &s.dealt, &names);
            assert!(
                view.action
                    .as_ref()
                    .is_none_or(|x| x.blocker != Some(u32::MAX)),
                "an unnumbered blocker shown"
            );
            if s.engine.state.seats[1].ousted && s.engine.action().is_none() {
                break;
            }
            let at = |c: &Choice| req.options.iter().position(|o| o == c);
            let i = at(&Choice::TakeAction {
                minion: a,
                action: ActionKind::Bleed,
                target: None,
            })
            .or_else(|| at(&Choice::DeclareBlock { minion: b }))
            .or_else(|| {
                req.options.iter().position(|o| {
                    matches!(o, Choice::Play { card: 0, mode: 0, actor: Some(m), .. } if *m == b)
                })
            });
            s.engine
                .choose(i.or_else(|| at(&Choice::Pass)).unwrap_or(0));
        }
        assert!(s.engine.state.seats[1].ousted, "ousted");
        let end = told
            .iter()
            .find_map(|t| match t {
                EventKind::ActionEnds { blocker, .. } => Some(*blocker),
                _ => None,
            })
            .expect("the action told ended");
        assert_eq!(end, None, "no blocker named");
    }

    /// A withdrawal kept is told `withdrawal_announced` then `withdrew`,
    /// never `ousted`; the view shows the seat out of the game, withdrawn
    /// (protocol.md §9; rules-qa 95.4, 95.5).
    #[test]
    fn a_withdrawal_is_told() {
        let (told, view) = withdrawal_told(false);
        let at = |k: &EventKind| told.iter().position(|t| t == k);
        let announced = at(&EventKind::WithdrawalAnnounced { seat: 0 }).expect("announced");
        let withdrew = at(&EventKind::Withdrew { seat: 0 }).expect("withdrew");
        assert!(announced < withdrew);
        assert!(!told.iter().any(|t| matches!(t, EventKind::Ousted { .. })));
        let s = &view.seats[0];
        assert!(s.ousted && s.withdrew && !s.withdrawing);
    }

    /// A withdrawal broken by a pool loss is told `withdrawal_broken`, the
    /// seat still in the game (rules-qa 95.4).
    #[test]
    fn a_broken_withdrawal_is_told() {
        let (told, view) = withdrawal_told(true);
        assert!(
            told.contains(&EventKind::WithdrawalBroken { seat: 0 }),
            "{told:?}"
        );
        let s = &view.seats[0];
        assert!(!s.ousted && !s.withdrew && !s.withdrawing);
    }

    /// A card an effect reveals is told to every seat with its id, no copy
    /// number (it stays in its hidden place): a crypt's top, as Mesu
    /// Bedshet reveals it (the engine's `Note::Reveal`).
    #[test]
    fn a_revealed_card_is_told_to_every_seat() {
        let (mut s, _) = session();
        s.observe(false);
        let top = *s.engine.state.seats[0].crypt.last().expect("a crypt");
        s.engine
            .notes
            .as_mut()
            .expect("noting")
            .push(Note::Reveal(zvtes::game::Reveal {
                seat: SeatId(0),
                card: top,
                crypt: true,
            }));
        s.observe(true);
        let fact = s
            .told
            .iter()
            .find(|t| matches!(t.fact, Fact::Revealed { .. }))
            .expect("the reveal observed")
            .clone();
        for seat in 0..3 {
            let mut v = crate::view::Viewer::new(Some(SeatId(seat)));
            let told = v.event(&s.engine, &s.dealt, &fact);
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
            .notes
            .as_mut()
            .expect("noting")
            .push(Note::Reveal(zvtes::game::Reveal {
                seat: SeatId(0),
                card: top,
                crypt: true,
            }));
        s.observe(true);
        assert!(matches!(s.undo(), Err(Refused::NoUndo)), "barred");
    }

    /// A fact names its minion by the copy it had when told: a merge
    /// before the next view (the bots' turns between two of the human's)
    /// gives the minion the advanced card's copy, one the viewer never
    /// saw before.
    #[test]
    fn a_fact_names_its_minion_as_it_was_when_told() {
        use zvtes::copies::Untracked;
        use zvtes::state::{Minion, MinionKind};
        let (mut s, crypt) = session();
        s.observe(false);
        let names = vec![String::new(); 3];
        let mut v = crate::view::Viewer::new(Some(SeatId(1)));
        let mut m = Minion::new(SeatId(0), MinionKind::Vampire(crypt[0].0), 3);
        m.copy = Untracked(CopyId(500));
        s.engine.state.minions.push(m);
        let i = s.engine.state.minions.len() - 1;
        s.observe(true);
        v.view(&s.engine, &s.dealt, &names);
        let seen = v.numbering.get(CopyId(500)).expect("in sight").0;
        s.take_told();
        s.engine.state.minions[i].blood = 2;
        s.observe(true);
        s.engine.state.minions[i].merged_copy = Untracked(CopyId(500));
        s.engine.state.minions[i].copy = Untracked(CopyId(501));
        v.view(&s.engine, &s.dealt, &names);
        let told: Vec<_> = s
            .take_told()
            .iter()
            .filter_map(|t| v.event(&s.engine, &s.dealt, t))
            .collect();
        assert!(
            told.iter().any(|k| matches!(
                k,
                zvtes_api::event::EventKind::Blood { minion, delta: -1, .. } if *minion == seen
            )),
            "{told:?}"
        );
    }

    /// A hand looked at is told to its viewer alone, its cards as they
    /// stood: SchreckNET locked after a passed referendum (the look ends
    /// with the action, a view after it hides the hand again).
    #[test]
    fn a_look_at_a_hand_is_told_to_its_viewer_alone() {
        let (mut s, _) = session();
        let mut e = Engine::bare(3);
        for seat in &mut e.state.seats {
            seat.pool = 10;
        }
        e.state.turn = 4;
        let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
        e.add_minion(SeatId(0), find_card("Crystal Lynn"), 4, false);
        e.add_minion(SeatId(1), find_card("Donny Kowalczyk"), 4, false);
        e.add_minion(SeatId(2), find_card("Madison"), 4, false);
        let net = find_library("SchreckNET");
        e.state
            .cards
            .push(CardInPlay::entering(net, CopyId::NONE, SeatId(0), None, 0));
        let net = CardId(e.state.cards.len() as u32 - 1);
        e.state.seats[0]
            .hand
            .push(find_library("Kine Resources Contested"));
        e.state.seats[1].hand.push(find_library("Minion Tap"));
        e.notes = Some(Vec::new());
        s.engine = e;
        s.observe(false);
        let kine = Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(caller),
            target: Target::None,
        };
        let look = Choice::UseAbility {
            source: AbilitySource::Card(net),
            ability: 0,
            actor: None,
            target: Target::Seat(SeatId(1)),
        };
        let wanted = [
            kine,
            Choice::AllocateTo {
                seat: SeatId(0),
                n: 3,
            },
            Choice::CastVotes {
                minion: caller,
                favor: true,
            },
            look,
        ];
        let mut looked = false;
        for _ in 0..200 {
            let Step::Decision(req) = s.engine.run() else {
                panic!("a decision");
            };
            let i = wanted
                .iter()
                .find_map(|w| req.options.iter().position(|o| o == w))
                .or_else(|| {
                    req.options
                        .iter()
                        .position(|o| matches!(o, Choice::Pass | Choice::HandStrike))
                })
                .unwrap_or(0);
            looked = req.options[i] == look;
            s.engine.choose(i);
            if looked {
                break;
            }
        }
        assert!(looked, "SchreckNET used");
        let cards = s.engine.state.seats[1].hand.to_vec();
        assert!(!cards.is_empty());
        s.observe(true);
        let fact = s
            .told
            .iter()
            .find(|t| matches!(t.fact, Fact::Looked(_)))
            .expect("the look observed")
            .clone();
        for seat in [Some(0), Some(1), Some(2), None] {
            let mut v = crate::view::Viewer::new(seat.map(SeatId));
            let told = v.event(&s.engine, &s.dealt, &fact);
            match told {
                Some(zvtes_api::event::EventKind::Looked {
                    seat: of, cards: c, ..
                }) => {
                    assert_eq!((seat, of), (Some(0), 1));
                    let ids: Vec<_> = cards
                        .iter()
                        .map(|&k| Some(zvtes::library::LIBRARY[k as usize].id))
                        .collect();
                    assert_eq!(c.iter().map(|c| c.id).collect::<Vec<_>>(), ids);
                    assert!(c.iter().all(|c| c.copy.is_none() && !c.crypt));
                }
                None => assert_ne!(seat, Some(0), "told to its viewer"),
                other => panic!("seat {seat:?}: {other:?}"),
            }
        }
    }

    /// A hand looked at bars the undo, as a reveal does: undone, the look
    /// would be known.
    #[test]
    fn a_look_bars_the_undo() {
        let (mut s, _) = session();
        to_undoable(&mut s);
        let cards = s.engine.state.seats[1].hand.to_vec();
        s.engine
            .notes
            .as_mut()
            .expect("noting")
            .push(Note::Look(zvtes::game::Look {
                viewer: SeatId(0),
                seat: SeatId(1),
                cards,
                library: false,
            }));
        s.observe(true);
        assert!(matches!(s.undo(), Err(Refused::NoUndo)), "barred");
    }

    /// The notes told in their order, a change to the table in its
    /// place among them, with its cause (`Note::Ledger`).
    #[test]
    fn the_notes_and_the_table_s_changes_are_told_in_their_order() {
        use zvtes::notes::{Cause, Source};
        let (mut s, _) = session();
        s.observe(false);
        let m = [zvtes::ids::MinionId(0), zvtes::ids::MinionId(1)];
        let top = *s.engine.state.seats[0].crypt.last().expect("a crypt");
        let villein = zvtes::state::find_library("Villein");
        let play = zvtes::game::PlayNote {
            seat: SeatId(0),
            card: villein,
            copy: CopyId::NONE,
            mode: 0,
            actor: None,
            target: Target::None,
            canceled: false,
            no_effect: false,
        };
        let mut paid = s.engine.table();
        paid.pools[0] -= 1;
        let cost = Cause::Cost(Source::Played {
            card: villein,
            copy: CopyId::NONE,
        });
        s.engine.notes = Some(vec![
            Note::Combat(CombatNote::Begins { minions: m }),
            Note::Play(play),
            Note::Reveal(zvtes::game::Reveal {
                seat: SeatId(0),
                card: top,
                crypt: true,
            }),
            Note::Ledger {
                ledger: Box::new(paid),
                cause: Some(cost),
            },
            Note::Combat(CombatNote::Ends { minions: m }),
        ]);
        let before = s.told.len();
        s.observe(true);
        let order: Vec<&str> = s.told[before..]
            .iter()
            .filter_map(|t| match t.fact {
                Fact::Combat(CombatNote::Begins { .. }) => Some("begins"),
                Fact::Revealed { .. } => Some("revealed"),
                Fact::Played(_) => Some("played"),
                Fact::Pool {
                    delta: -1,
                    cause: Some(c),
                    ..
                } if c == cost => Some("paid"),
                Fact::Combat(CombatNote::Ends { .. }) => Some("ends"),
                _ => None,
            })
            .collect();
        assert_eq!(order, vec!["begins", "played", "revealed", "paid", "ends"]);
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
        e.notes = Some(Vec::new());
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
        e.notes = Some(Vec::new());
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
    /// A wake as an action card is played is shown with the opportunities
    /// off only when the woken minion could then cancel the card (owner,
    /// 2026-10-25; rules-qa 96.4): with Rewind Time in hand, not without,
    /// nor with it the wake's replacement to come.
    #[test]
    fn a_wake_as_a_card_is_played_is_hidden_with_the_opportunities_off_unless_it_cancels() {
        // A V5 vampire with `disc` and no printed text (no more decisions).
        let with = |disc: &str| {
            (0..zvtes::crypt::CRYPT.len() as u16)
                .find(|&i| {
                    zvtes::crypt::CRYPT[i as usize].disciplines.contains(&disc)
                        && zvtes::cards::vampire(i).is_none()
                })
                .expect("a vampire")
        };
        // Rewind Time in hand; on the library alone, drawn as the wake's
        // replacement: not yet the seat's to know of.
        // A third seat able to answer the wake itself (Direct Intervention
        // as it is played) changes nothing.
        for (rewind, top, answer) in [
            (true, false, false),
            (true, false, true),
            (false, false, false),
            (false, true, false),
        ] {
            let (mut s, _) = session();
            let mut e = Engine::bare(3);
            if answer {
                e.add_minion(SeatId(2), with("dom"), 4, false);
                let di = find_library("Direct Intervention");
                e.state.seats[2].hand.put(di, CopyId::NONE);
            }
            let a = e.add_minion(SeatId(0), with("dom"), 4, false);
            e.add_minion(SeatId(0), find_card("Madison"), 4, false);
            let t = e.add_minion(SeatId(1), with("dom"), 4, false);
            // Temporis, for Rewind Time (a free beast slot).
            let slots = &mut e.state.minions[t.index()].beast_disciplines;
            *slots.iter_mut().find(|d| d.is_none()).expect("a free slot") = Some("tem");
            e.state.minions[t.index()].locked = true;
            let govern = find_library("Govern the Unaligned");
            e.state.seats[0].hand.put(govern, CopyId::NONE);
            let mut hand = vec!["On the Qui Vive", "Eluding the Arms of Morpheus"];
            if rewind {
                hand.push("Rewind Time");
            }
            for c in hand {
                e.state.seats[1].hand.put(find_library(c), CopyId::NONE);
            }
            s.engine = e;
            let Step::Decision(req) = s.engine.run() else {
                panic!("a decision");
            };
            let i = req
                .options
                .iter()
                .position(|o| matches!(o, Choice::Play { actor: Some(x), .. } if *x == a))
                .expect("Govern the Unaligned offered");
            s.engine.choose(i);
            let Step::Decision(req) = s.engine.run() else {
                panic!("a decision");
            };
            assert_eq!(
                (req.seat, req.window),
                (SeatId(1), WindowLabel::CardAsPlayed)
            );
            if top {
                (s.engine.state.seats[1].library).put(find_library("Rewind Time"), CopyId::NONE);
            }
            let wake = req
                .options
                .iter()
                .copied()
                .find(|o| matches!(o, Choice::Play { actor: Some(x), .. } if *x == t))
                .expect("On the Qui Vive offered");
            if rewind {
                let i = req.options.iter().position(|o| *o == wake).unwrap();
                s.set_opportunities(true);
                assert!(
                    s.shown(&req).contains(&i),
                    "shown with the opportunities on"
                );
                s.set_opportunities(false);
                assert!(s.shown(&req).contains(&i), "the cancel to come: shown");
            } else {
                hidden_off(&mut s, &req, wake);
            }
        }
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
        e.notes = Some(Vec::new());
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
        e.notes = Some(Vec::new());
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

    /// With the opportunities off, `aggravates` (an option of seat 0's
    /// `vampire`, holding `card`) is hidden from before range on and shown
    /// once the strikes are chosen (owner, 2026-10-18).
    fn aggravation_waits_for_the_strikes(
        vampire: &str,
        card: Option<&str>,
        aggravates: fn(zvtes::ids::MinionId, &Choice) -> bool,
    ) {
        let (mut s, _) = session();
        let mut e = Engine::bare(3);
        let p = e.add_minion(SeatId(0), find_card(vampire), 4, false);
        e.add_minion(SeatId(0), find_card("Madison"), 4, false);
        let z = e.add_minion(SeatId(1), find_card("Donny Kowalczyk"), 4, false);
        if let Some(c) = card {
            e.state.seats[0].hand.put(find_library(c), CopyId::NONE);
        }
        e.notes = Some(Vec::new());
        s.engine = e;
        let bleed = Choice::TakeAction {
            minion: p,
            action: ActionKind::Bleed,
            target: None,
        };
        let block = Choice::DeclareBlock { minion: z };
        let mut steps = Vec::new();
        for _ in 0..40 {
            let Step::Decision(r) = s.engine.run() else {
                panic!("a decision");
            };
            if let Some(&want) = r.options.iter().find(|o| aggravates(p, o)) {
                assert_eq!(r.seat, SeatId(0));
                let step = s.engine.combat().expect("in combat").step;
                steps.push(step);
                if step == CombatStep::BeforeResolution {
                    s.set_opportunities(false);
                    let i = r.options.iter().position(|o| *o == want).unwrap();
                    assert!(
                        s.shown(&r).contains(&i),
                        "shown once the strikes are chosen"
                    );
                    s.set_opportunities(true);
                    break;
                }
                hidden_off(&mut s, &r, want);
            }
            let i = [bleed, block, Choice::Pass]
                .iter()
                .find_map(|w| r.options.iter().position(|o| o == w))
                .unwrap_or(0);
            s.engine.choose(i);
        }
        assert_eq!(
            steps.first().zip(steps.last()),
            Some((&CombatStep::BeforeRange, &CombatStep::BeforeResolution)),
            "{steps:?}"
        );
    }

    /// A card: Claws of the Dead, untimed.
    #[test]
    fn an_aggravating_card_waits_for_the_strikes_chosen() {
        aggravation_waits_for_the_strikes(
            "Adrino Manauara",
            Some("Claws of the Dead"),
            |p, o| matches!(o, Choice::Play { card: 0, mode: 0, actor: Some(a), .. } if *a == p),
        );
    }

    /// A vampire's ability: Crossbreaker's burn for aggravated hand strikes.
    #[test]
    fn an_aggravating_ability_waits_for_the_strikes_chosen() {
        aggravation_waits_for_the_strikes(
            "Crossbreaker",
            None,
            |p, o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == p),
        );
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
        e.notes = Some(Vec::new());
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

    /// Once blocks are declined, a wake is shown with the opportunities off
    /// only on a bleed at the seat, to wake and bounce it (owner,
    /// 2026-10-19): an undirected hunt of its predator's hides it then;
    /// both show it in the seat's block moment.
    #[test]
    fn a_wake_after_blocks_are_declined_is_shown_on_a_bleed_at_the_seat() {
        assert_eq!(wake_shown(ActionKind::Bleed), (true, true));
        assert_eq!(wake_shown(ActionKind::Hunt), (true, false));
    }

    /// Whether the seat's locked vampire's wake is shown in its predator's
    /// referendum, the opportunities `on` or off (`None`: never offered).
    fn referendum_wake_shown(on: bool) -> Option<bool> {
        let (mut s, _) = session();
        let mut e = Engine::bare(3);
        for seat in &mut e.state.seats {
            seat.pool = 10;
        }
        e.state.turn = 4;
        let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
        e.add_minion(SeatId(0), find_card("Crystal Lynn"), 4, false);
        let z = e.add_minion(SeatId(1), find_card("Donny Kowalczyk"), 4, false);
        e.state.minions[z.index()].locked = true;
        e.state.seats[1]
            .hand
            .put(find_library("On the Qui Vive"), CopyId::NONE);
        e.state.seats[0]
            .hand
            .put(find_library("Kine Resources Contested"), CopyId::NONE);
        e.notes = Some(Vec::new());
        s.engine = e;
        s.set_opportunities(on);
        let wanted = [
            Choice::Play {
                card: 0,
                mode: 0,
                actor: Some(caller),
                target: Target::None,
            },
            Choice::AllocateTo {
                seat: SeatId(0),
                n: 3,
            },
        ];
        let mut shown = None;
        for _ in 0..60 {
            let Step::Decision(req) = s.engine.run() else {
                break;
            };
            if req.seat == SeatId(1) && s.engine.referendum().is_some() {
                if let Some(i) = req
                    .options
                    .iter()
                    .position(|o| matches!(o, Choice::Play { actor: Some(a), .. } if *a == z))
                {
                    shown = Some(s.shown(&req).contains(&i));
                    break;
                }
            }
            if req.window == WindowLabel::MinionPhase
                && s.engine.referendum().is_none()
                && req.seat != SeatId(0)
            {
                break;
            }
            let i = wanted
                .iter()
                .find_map(|w| req.options.iter().position(|o| o == w))
                .or_else(|| req.options.iter().position(|o| *o == Choice::Pass))
                .unwrap_or(0);
            s.engine.choose(i);
        }
        shown
    }

    /// A wake is never shown in a referendum with the opportunities off
    /// (owner, 2026-10-19): only with them on.
    #[test]
    fn a_wake_in_a_referendum_is_hidden_with_the_opportunities_off() {
        assert_eq!(
            referendum_wake_shown(true),
            Some(true),
            "shown with them on"
        );
        assert_eq!(
            referendum_wake_shown(false),
            Some(false),
            "offered, hidden off"
        );
    }

    /// Seat 0's `action` (a bleed is at seat 1) against seat 1's locked
    /// vampire holding a wake: whether the wake is shown, opportunities
    /// off, in seat 1's block moment and once blocks are declined.
    fn wake_shown(action: ActionKind) -> (bool, bool) {
        let (mut s, _) = session();
        let mut e = Engine::bare(3);
        let p = e.add_minion(SeatId(0), find_card("Madison"), 4, false);
        e.add_minion(SeatId(0), find_card("Crystal Lynn"), 4, false);
        let z = e.add_minion(SeatId(1), find_card("Donny Kowalczyk"), 4, false);
        e.state.minions[z.index()].locked = true;
        e.state.seats[1]
            .hand
            .put(find_library("On the Qui Vive"), CopyId::NONE);
        e.notes = Some(Vec::new());
        s.engine = e;
        let Step::Decision(req) = s.engine.run() else {
            panic!("a decision");
        };
        let act = Choice::TakeAction {
            minion: p,
            action,
            target: None,
        };
        let i = req
            .options
            .iter()
            .position(|o| *o == act)
            .expect("the action");
        s.engine.choose(i);
        s.set_opportunities(false);
        let wake = |req: &DecisionRequest| {
            req.options
                .iter()
                .position(|o| matches!(o, Choice::Play { actor: Some(a), .. } if *a == z))
        };
        let (mut open, mut declined, mut offered) = (false, false, false);
        for _ in 0..40 {
            let Step::Decision(req) = s.engine.run() else {
                break;
            };
            if req.seat == SeatId(1) {
                if let Some(i) = wake(&req) {
                    let state = s.engine.action().map(|a| a.state);
                    match state {
                        Some(ActionState::Open) => open |= s.shown(&req).contains(&i),
                        Some(ActionState::Declined) => {
                            offered = true;
                            declined |= s.shown(&req).contains(&i);
                        }
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
        assert!(offered, "offered once blocks are declined");
        (open, declined)
    }
}
