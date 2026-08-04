use crate::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use crate::ids::{MinionId, SeatId};
use crate::rng::Rng;
use crate::state::{
    GameState, Minion, Seat, UncontrolledSlot, CRYPT_SIZE, START_POOL, UNCONTROLLED_TARGET,
};
use crate::window::OrderedWindow;

/// Time-out stand-in: a game reaching this many turns ends where it stands.
pub const TURN_LIMIT: u32 = 500;

/// The block loop per rules-qa 1.3: one ordered window, no stealth/intercept
/// failure checkpoint. In the card-less P0 pool a declared attempt always
/// resolves as a successful block at closure (0 intercept >= 0 stealth), and
/// a pass with no ongoing attempt is a final decline. Only defending seats
/// are polled: with no cards, no other seat ever has a non-pass option.
#[derive(Clone, Debug)]
struct BlockCtx {
    acting: MinionId,
    action: ActionKind,
    window: OrderedWindow,
    attempt: Option<MinionId>,
    declined: Vec<SeatId>,
}

#[derive(Clone, Debug)]
enum Flow {
    /// Unlock-phase decision point: burn the Edge for 1 pool, or keep it.
    Unlock,
    /// Minion phase: offer actions for the active seat's ready minions.
    MinionPhase,
    Block(BlockCtx),
    Influence {
        left: u8,
    },
    GameOver,
}

pub enum Step {
    Decision(DecisionRequest),
    /// Final VPs per seat.
    Over(Vec<u32>),
}

pub struct Engine {
    pub state: GameState,
    /// Chosen option indices for real (non-elided) decisions only.
    pub log: Vec<usize>,
    pub decisions: u64,
    flow: Flow,
}

impl Engine {
    pub fn new(seed: u64, n_seats: usize) -> Self {
        assert!((2..=6).contains(&n_seats));
        let mut rng = Rng::new(seed);
        let seats = (0..n_seats)
            .map(|_| {
                let mut crypt: Vec<u8> = (0..CRYPT_SIZE).map(|_| 3 + rng.below(6) as u8).collect();
                rng.shuffle(&mut crypt);
                let uncontrolled = (0..UNCONTROLLED_TARGET)
                    .map(|_| UncontrolledSlot {
                        capacity: crypt.pop().expect("crypt larger than initial draw"),
                        counters: 0,
                    })
                    .collect();
                Seat {
                    pool: START_POOL,
                    ousted: false,
                    vps: 0,
                    has_edge: false,
                    crypt,
                    uncontrolled,
                    turns_taken: 0,
                }
            })
            .collect();
        let mut engine = Self {
            state: GameState {
                seats,
                minions: Vec::new(),
                active: SeatId(0),
                turn: 0,
            },
            log: Vec::new(),
            decisions: 0,
            flow: Flow::GameOver, // replaced by start_turn
        };
        engine.start_turn();
        engine
    }

    /// Advance until a real decision is needed or the game is over. Windows
    /// with a single legal option are elided (DESIGN §4.3): auto-applied and
    /// not logged.
    pub fn run(&mut self) -> Step {
        loop {
            if matches!(self.flow, Flow::GameOver) {
                return Step::Over(self.state.seats.iter().map(|s| s.vps).collect());
            }
            match self.request() {
                None => self.advance(),
                Some(req) => {
                    debug_assert!(!req.options.is_empty());
                    if req.options.len() == 1 {
                        let choice = req.options[0];
                        self.apply(&choice);
                    } else {
                        return Step::Decision(req);
                    }
                }
            }
        }
    }

    /// Answer the pending decision request by option index.
    pub fn choose(&mut self, idx: usize) {
        let req = self.request().expect("no pending decision");
        assert!(idx < req.options.len(), "option index out of range");
        self.log.push(idx);
        self.decisions += 1;
        let choice = req.options[idx];
        self.apply(&choice);
    }

    fn request(&self) -> Option<DecisionRequest> {
        let st = &self.state;
        match &self.flow {
            Flow::Unlock => {
                let s = st.active;
                if st.seats[s.index()].has_edge {
                    Some(DecisionRequest {
                        seat: s,
                        window: WindowLabel::UnlockEdge,
                        options: vec![Choice::Pass, Choice::BurnEdge],
                    })
                } else {
                    None
                }
            }
            Flow::MinionPhase => {
                let s = st.active;
                let mut options = vec![Choice::Pass];
                for id in st.minions_of(s) {
                    let m = &st.minions[id.index()];
                    if m.locked {
                        continue;
                    }
                    if m.in_torpor {
                        // Leave torpor: requires the 2-blood cost to be payable.
                        if m.blood >= 2 {
                            options.push(Choice::TakeAction {
                                minion: id,
                                action: ActionKind::LeaveTorpor,
                            });
                        }
                    } else if m.blood == 0 {
                        // A vampire with no blood must hunt when it acts.
                        options.push(Choice::TakeAction {
                            minion: id,
                            action: ActionKind::Hunt,
                        });
                    } else {
                        if !m.bled_this_turn {
                            options.push(Choice::TakeAction {
                                minion: id,
                                action: ActionKind::Bleed,
                            });
                        }
                        options.push(Choice::TakeAction {
                            minion: id,
                            action: ActionKind::Hunt,
                        });
                    }
                }
                Some(DecisionRequest {
                    seat: s,
                    window: WindowLabel::MinionPhase,
                    options,
                })
            }
            Flow::Block(ctx) => {
                let seat = ctx.window.current()?;
                let mut options = vec![Choice::Pass];
                if ctx.attempt.is_none() && !ctx.declined.contains(&seat) {
                    for id in st.ready_unlocked(seat) {
                        options.push(Choice::DeclareBlock { minion: id });
                    }
                }
                Some(DecisionRequest {
                    seat,
                    window: WindowLabel::ActionBlock,
                    options,
                })
            }
            Flow::Influence { left } => {
                let s = st.active;
                let seat = &st.seats[s.index()];
                let mut options = vec![Choice::Pass];
                if *left > 0 && seat.pool > 0 {
                    for (i, slot) in seat.uncontrolled.iter().enumerate() {
                        if slot.counters < slot.capacity {
                            options.push(Choice::Transfer { slot: i as u8 });
                        }
                    }
                }
                Some(DecisionRequest {
                    seat: s,
                    window: WindowLabel::InfluenceTransfers,
                    options,
                })
            }
            Flow::GameOver => None,
        }
    }

    /// Deterministic transitions for states with no decision to offer.
    fn advance(&mut self) {
        let flow = std::mem::replace(&mut self.flow, Flow::GameOver);
        match flow {
            // No Edge to consider; Master phase has no cards in P0 → elided.
            Flow::Unlock => self.flow = Flow::MinionPhase,
            Flow::Block(ctx) => {
                debug_assert!(ctx.window.is_closed());
                self.resolve_action(ctx);
            }
            other => unreachable!("advance() with a pending decision: {other:?}"),
        }
    }

    fn apply(&mut self, choice: &Choice) {
        let flow = std::mem::replace(&mut self.flow, Flow::GameOver);
        match (flow, choice) {
            (Flow::Unlock, Choice::BurnEdge) => {
                let s = self.state.active.index();
                self.state.seats[s].has_edge = false;
                self.state.seats[s].pool += 1;
                self.flow = Flow::MinionPhase;
            }
            (Flow::Unlock, Choice::Pass) => self.flow = Flow::MinionPhase,
            (Flow::MinionPhase, Choice::Pass) => {
                let seat = &self.state.seats[self.state.active.index()];
                let left = seat.turns_taken.min(4) as u8;
                self.flow = Flow::Influence { left };
            }
            (Flow::MinionPhase, Choice::TakeAction { minion, action }) => {
                self.state.minions[minion.index()].locked = true;
                let acting_seat = self.state.active;
                let mut order = match action {
                    // Directed at the prey: only the defender may block.
                    ActionKind::Bleed => vec![self.state.prey(acting_seat)],
                    // Undirected: prey then predator (DESIGN §4.2 ordering).
                    ActionKind::Hunt | ActionKind::LeaveTorpor => vec![
                        self.state.prey(acting_seat),
                        self.state.predator(acting_seat),
                    ],
                };
                order.dedup();
                self.flow = Flow::Block(BlockCtx {
                    acting: *minion,
                    action: *action,
                    window: OrderedWindow::new(order),
                    attempt: None,
                    declined: Vec::new(),
                });
            }
            (Flow::Block(mut ctx), Choice::DeclareBlock { minion }) => {
                ctx.attempt = Some(*minion);
                // Declaring changes the window state: prior passes are void.
                ctx.window.play();
                self.flow = Flow::Block(ctx);
            }
            (Flow::Block(mut ctx), Choice::Pass) => {
                let seat = ctx.window.current().expect("pass on closed window");
                if ctx.attempt.is_none() && !ctx.declined.contains(&seat) {
                    // rules-qa 1.3: declining to block is final.
                    ctx.declined.push(seat);
                }
                ctx.window.pass();
                self.flow = Flow::Block(ctx);
            }
            (Flow::Influence { left }, Choice::Transfer { slot }) => {
                let s = self.state.active;
                let seat = &mut self.state.seats[s.index()];
                seat.pool -= 1;
                seat.uncontrolled[*slot as usize].counters += 1;
                if seat.pool == 0 {
                    // Pool 0 ousts, whatever the cause; the turn ends at once.
                    self.oust(s);
                    if self.state.living() > 1 {
                        self.next_turn();
                    }
                } else {
                    self.flow = Flow::Influence { left: left - 1 };
                }
            }
            (Flow::Influence { .. }, Choice::Pass) => self.end_influence(),
            (flow, choice) => unreachable!("illegal choice {choice:?} in {flow:?}"),
        }
    }

    fn resolve_action(&mut self, ctx: BlockCtx) {
        match ctx.attempt {
            Some(blocker) => {
                // Block resolution: 0 intercept >= 0 stealth in P0 — the
                // blocker locks and combat begins.
                self.state.minions[blocker.index()].locked = true;
                if ctx.action == ActionKind::Bleed {
                    // NRA triggers on resolution, blocked included (rules-qa 1.4).
                    self.state.minions[ctx.acting.index()].bled_this_turn = true;
                }
                self.combat(ctx.acting, blocker);
                self.flow = Flow::MinionPhase;
            }
            None => {
                self.flow = Flow::MinionPhase;
                match ctx.action {
                    ActionKind::Bleed => {
                        let acting_seat = self.state.active;
                        let prey = self.state.prey(acting_seat);
                        self.state.minions[ctx.acting.index()].bled_this_turn = true;
                        self.state.seats[prey.index()].pool -= 1;
                        // Successful bleed of the prey: take the Edge.
                        for seat in &mut self.state.seats {
                            seat.has_edge = false;
                        }
                        self.state.seats[acting_seat.index()].has_edge = true;
                        if self.state.seats[prey.index()].pool <= 0 {
                            self.oust(prey);
                        }
                    }
                    ActionKind::Hunt => {
                        let m = &mut self.state.minions[ctx.acting.index()];
                        m.blood = (m.blood + 1).min(m.capacity);
                    }
                    ActionKind::LeaveTorpor => {
                        // Cost is paid at resolution only (§1.7: a blocked
                        // action pays nothing).
                        let m = &mut self.state.minions[ctx.acting.index()];
                        m.blood -= 2;
                        m.in_torpor = false;
                    }
                }
            }
        }
    }

    /// P0 combat: close range, one round, simultaneous hand strikes for 1.
    /// A combatant in torpor strikes for nothing; a vampire that cannot burn
    /// blood for damage goes to torpor (or is burned if already there).
    fn combat(&mut self, a: MinionId, b: MinionId) {
        let dmg_from = |st: &GameState, m: MinionId| -> u8 {
            if st.minions[m.index()].in_torpor {
                0
            } else {
                1
            }
        };
        let (to_b, to_a) = (dmg_from(&self.state, a), dmg_from(&self.state, b));
        self.damage(b, to_b);
        self.damage(a, to_a);
    }

    fn damage(&mut self, m: MinionId, n: u8) {
        let minion = &mut self.state.minions[m.index()];
        for _ in 0..n {
            if minion.blood > 0 {
                minion.blood -= 1;
            } else if minion.in_torpor {
                minion.in_play = false;
            } else {
                minion.in_torpor = true;
            }
        }
    }

    fn oust(&mut self, s: SeatId) {
        {
            let seat = &mut self.state.seats[s.index()];
            seat.ousted = true;
            seat.pool = 0;
            seat.has_edge = false;
            seat.crypt.clear();
            seat.uncontrolled.clear();
        }
        for m in &mut self.state.minions {
            if m.owner == s {
                m.in_play = false;
            }
        }
        let pred = self.state.predator(s);
        self.state.seats[pred.index()].vps += 1;
        self.state.seats[pred.index()].pool += 6;
        if self.state.living() == 1 {
            // The sole survivor gains one additional VP.
            let survivor = (0..self.state.seats.len())
                .find(|&i| !self.state.seats[i].ousted)
                .expect("one seat must survive");
            self.state.seats[survivor].vps += 1;
            self.flow = Flow::GameOver;
        }
    }

    fn end_influence(&mut self) {
        let s = self.state.active;
        // Influence out: filled slots enter play ready and unlocked, counters
        // becoming blood (auto-applied; keeping a filled vampire uncontrolled
        // is a P0 simplification we do not offer).
        let seat = &mut self.state.seats[s.index()];
        let mut kept = Vec::new();
        let mut out = Vec::new();
        for slot in seat.uncontrolled.drain(..) {
            if slot.counters >= slot.capacity {
                out.push(slot.capacity);
            } else {
                kept.push(slot);
            }
        }
        seat.uncontrolled = kept;
        // Refill the uncontrolled region from the crypt (P0 simplification of
        // the 4-transfers-for-a-new-crypt-card rule).
        while seat.uncontrolled.len() < UNCONTROLLED_TARGET {
            match seat.crypt.pop() {
                Some(capacity) => seat.uncontrolled.push(UncontrolledSlot {
                    capacity,
                    counters: 0,
                }),
                None => break,
            }
        }
        for capacity in out {
            self.state.minions.push(Minion {
                owner: s,
                capacity,
                blood: capacity,
                locked: false,
                in_torpor: false,
                in_play: true,
                bled_this_turn: false,
            });
        }
        // Discard phase: no cards in P0 → elided.
        self.next_turn();
    }

    fn next_turn(&mut self) {
        if self.state.turn >= TURN_LIMIT {
            self.flow = Flow::GameOver;
            return;
        }
        self.state.active = self.state.prey(self.state.active);
        self.start_turn();
    }

    fn start_turn(&mut self) {
        let s = self.state.active;
        self.state.turn += 1;
        self.state.seats[s.index()].turns_taken += 1;
        for m in &mut self.state.minions {
            // Per-turn NRA flags reset each new turn.
            m.bled_this_turn = false;
            // Unlock phase: the active seat's minions unlock (torpor included).
            if m.owner == s && m.in_play {
                m.locked = false;
            }
        }
        self.flow = Flow::Unlock;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 3 seats, empty crypts/influence. Seat 0 has two ready vampires
    /// (`MinionId(0)`, `MinionId(1)`) so `run()` always halts at seat 0's
    /// minion phase instead of eliding through to the next turn; seat 1 has
    /// one (`MinionId(2)`).
    fn bare() -> Engine {
        let mut e = Engine::new(0, 3);
        for seat in &mut e.state.seats {
            seat.crypt.clear();
            seat.uncontrolled.clear();
        }
        for owner in [0u8, 0u8, 1u8] {
            e.state.minions.push(Minion {
                owner: SeatId(owner),
                capacity: 5,
                blood: 5,
                locked: false,
                in_torpor: false,
                in_play: true,
                bled_this_turn: false,
            });
        }
        e.flow = Flow::MinionPhase;
        e
    }

    fn pick(e: &mut Engine, want: Choice) {
        match e.run() {
            Step::Decision(req) => {
                let idx = req
                    .options
                    .iter()
                    .position(|o| *o == want)
                    .unwrap_or_else(|| panic!("option {want:?} not in {:?}", req.options));
                e.choose(idx);
            }
            Step::Over(_) => panic!("game over while expecting {want:?}"),
        }
    }

    fn options(e: &mut Engine) -> Vec<Choice> {
        match e.run() {
            Step::Decision(req) => req.options,
            Step::Over(_) => panic!("game over while expecting a decision"),
        }
    }

    #[test]
    fn blocked_bleed_costs_no_pool_but_locks_nra() {
        let mut e = bare();
        let acting = MinionId(0);
        let blocker = MinionId(2);
        pick(
            &mut e,
            Choice::TakeAction {
                minion: acting,
                action: ActionKind::Bleed,
            },
        );
        pick(&mut e, Choice::DeclareBlock { minion: blocker });
        // Attempt pending; nobody can play anything in P0, so the window
        // closes and the block resolves. run() halts back at seat 0's minion
        // phase because MinionId(1) is still ready.
        let opts = options(&mut e);
        assert_eq!(e.state.seats[1].pool, START_POOL);
        assert!(e.state.minions[acting.index()].bled_this_turn);
        assert_eq!(e.state.minions[acting.index()].blood, 4);
        assert_eq!(e.state.minions[blocker.index()].blood, 4);
        assert!(e.state.minions[blocker.index()].locked);
        // The acting minion is locked: no further action options for it.
        assert!(opts
            .iter()
            .all(|o| !matches!(o, Choice::TakeAction { minion, .. } if *minion == acting)));
    }

    #[test]
    fn declined_block_lets_bleed_resolve_and_move_the_edge() {
        let mut e = bare();
        let acting = MinionId(0);
        pick(
            &mut e,
            Choice::TakeAction {
                minion: acting,
                action: ActionKind::Bleed,
            },
        );
        // Prey declines (final, rules-qa 1.3); bleed resolves.
        pick(&mut e, Choice::Pass);
        let _ = options(&mut e); // settle back at seat 0's minion phase
        assert_eq!(e.state.seats[1].pool, START_POOL - 1);
        assert!(e.state.seats[0].has_edge);
        assert!(e.state.minions[acting.index()].bled_this_turn);
    }

    #[test]
    fn nra_bars_second_bleed_not_second_hunt() {
        let mut e = bare();
        e.state.minions[MinionId(0).index()].bled_this_turn = true;
        let opts = options(&mut e);
        assert!(!opts.contains(&Choice::TakeAction {
            minion: MinionId(0),
            action: ActionKind::Bleed,
        }));
        assert!(opts.contains(&Choice::TakeAction {
            minion: MinionId(0),
            action: ActionKind::Hunt,
        }));
    }

    #[test]
    fn hunt_is_blockable_by_prey_and_predator() {
        let mut e = bare();
        // Give the predator (seat 2) a ready minion too: MinionId(3).
        e.state.minions.push(Minion {
            owner: SeatId(2),
            capacity: 5,
            blood: 5,
            locked: false,
            in_torpor: false,
            in_play: true,
            bled_this_turn: false,
        });
        pick(
            &mut e,
            Choice::TakeAction {
                minion: MinionId(0),
                action: ActionKind::Hunt,
            },
        );
        // Prey is offered first, declines; predator is offered next.
        let req = match e.run() {
            Step::Decision(req) => req,
            Step::Over(_) => panic!(),
        };
        assert_eq!(req.seat, SeatId(1));
        pick(&mut e, Choice::Pass);
        let req = match e.run() {
            Step::Decision(req) => req,
            Step::Over(_) => panic!(),
        };
        assert_eq!(req.seat, SeatId(2));
        pick(
            &mut e,
            Choice::DeclareBlock {
                minion: MinionId(3),
            },
        );
        let _ = options(&mut e); // settle: block resolves, combat happens
        assert_eq!(e.state.minions[0].blood, 4);
        assert_eq!(e.state.minions[3].blood, 4);
    }

    #[test]
    fn edge_burn_is_offered_on_unlock() {
        let mut e = bare();
        e.state.seats[0].has_edge = false;
        e.state.seats[1].has_edge = true;
        // Seat 0 passes its minion phase; the empty influence phase elides
        // and the turn passes to seat 1, whose unlock phase opens with the
        // Edge decision.
        pick(&mut e, Choice::Pass);
        let req = match e.run() {
            Step::Decision(req) => req,
            Step::Over(_) => panic!(),
        };
        assert_eq!(req.seat, SeatId(1));
        assert_eq!(req.window, WindowLabel::UnlockEdge);
        let pool = e.state.seats[1].pool;
        pick(&mut e, Choice::BurnEdge);
        assert_eq!(e.state.seats[1].pool, pool + 1);
        assert!(!e.state.seats[1].has_edge);
    }
}
