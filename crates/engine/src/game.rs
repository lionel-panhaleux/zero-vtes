use crate::crypt::{CRYPT, PRECONS};
use crate::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use crate::ids::{MinionId, SeatId};
use crate::rng::Rng;
use crate::state::{GameState, Minion, Seat, UncontrolledSlot, START_POOL, UNCONTROLLED_START};
use crate::window::{OrderedWindow, PollingWindow};

/// Time-limit stand-in (owner, 2026-09-02): a tournament game rarely exceeds
/// 20 full rounds, so 100 player turns is ample. Checked at the turn
/// boundary, which stands in for the tournament rule "the game ends after the
/// current action / at the end of the current phase".
pub const TURN_LIMIT: u32 = 100;

/// The block loop per rules-qa 1.3: one ordered window, no stealth/intercept
/// failure checkpoint of its own. A declared attempt stays pending until the
/// window closes; at closure the block succeeds iff intercept >= stealth. A
/// pass with no ongoing attempt is a final decline.
#[derive(Clone, Debug)]
struct BlockCtx {
    acting: MinionId,
    action: ActionKind,
    target: Option<MinionId>,
    stealth: u8,
    window: OrderedWindow,
    attempt: Option<MinionId>,
    declined: Vec<SeatId>,
}

/// Blood hunt referendum (rulebook "The Blood Hunt"): not an action, no
/// modifier/reaction windows — polling only. Sources (rules-qa 1.1): each
/// ready vampire with votes, and the Edge; each commits once, all its votes
/// one way, and is then spent for this referendum.
#[derive(Clone, Debug)]
struct BloodHuntCtx {
    diablerist: MinionId,
    window: PollingWindow,
    spent: Vec<MinionId>,
    edge_spent: bool,
    for_votes: u32,
    against_votes: u32,
}

/// One contested object of the active seat, resolved during its unlock
/// phase (rulebook "Contested Cards" / "Contested Titles"; DESIGN §4.4
/// factoring: each Methuselah settles their own side, in arena order).
#[derive(Clone, Copy, Debug)]
enum Contest {
    Card(MinionId),
    Title(MinionId),
}

#[derive(Clone, Debug)]
enum Flow {
    /// Unlock-phase decision point: burn the Edge for 1 pool, or keep it.
    Unlock,
    /// Unlock-phase contest upkeep, one contested object at a time.
    Contests {
        items: Vec<Contest>,
        at: usize,
    },
    /// Minion phase: offer actions for the active seat's ready minions.
    MinionPhase,
    Block(BlockCtx),
    /// Rescue resolved unblocked: split the 2-blood cost (paid at resolution).
    RescueCost {
        acting: MinionId,
        target: MinionId,
    },
    BloodHunt(BloodHuntCtx),
    Influence {
        left: u8,
    },
    GameOver,
}

pub enum Step {
    Decision(DecisionRequest),
    /// Final VPs per seat, in half-point units.
    Over(Vec<u32>),
}

pub struct Engine {
    pub state: GameState,
    /// Chosen option indices for real (non-elided) decisions only.
    pub log: Vec<usize>,
    pub decisions: u64,
    flow: Flow,
}

/// Transfers granted at the start of an influence phase: 1/2/3 on the first
/// three turns of the game, 4 thereafter (rulebook "Influence Phase").
pub fn transfers_for_turn(turn: u32) -> u8 {
    turn.min(4) as u8
}

impl Engine {
    /// A new game: each seat gets a random full-size V5 precon crypt,
    /// shuffled, with the top four cards dealt face down to the uncontrolled
    /// region (rulebook "Setup").
    pub fn new(seed: u64, n_seats: usize) -> Self {
        assert!((2..=6).contains(&n_seats));
        let mut rng = Rng::new(seed);
        let seats = (0..n_seats)
            .map(|_| {
                let (_, list) = PRECONS[rng.below(PRECONS.len())];
                let mut crypt: Vec<u16> = list
                    .iter()
                    .flat_map(|&(card, copies)| std::iter::repeat_n(card, copies as usize))
                    .collect();
                rng.shuffle(&mut crypt);
                let uncontrolled = (0..UNCONTROLLED_START)
                    .map(|_| UncontrolledSlot {
                        card: crypt.pop().expect("crypt larger than initial draw"),
                        counters: 0,
                    })
                    .collect();
                Seat {
                    pool: START_POOL,
                    ousted: false,
                    half_vps: 0,
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

    /// Test fixture: `n_seats` seats with empty crypts and uncontrolled
    /// regions, no minions, positioned at seat 0's minion phase on turn 1.
    /// Populate with [`Engine::add_minion`].
    pub fn bare(n_seats: usize) -> Self {
        let mut e = Self::new(0, n_seats);
        for seat in &mut e.state.seats {
            seat.crypt.clear();
            seat.uncontrolled.clear();
        }
        e.flow = Flow::MinionPhase;
        e
    }

    /// Test fixture: put the vampire `card` (a CRYPT index) into play for
    /// `owner`, ready and unlocked (or in torpor), with `blood`. Contests
    /// are *not* entered — use [`Engine::stage_contests`] to stage one.
    pub fn add_minion(&mut self, owner: SeatId, card: u16, blood: u8, in_torpor: bool) -> MinionId {
        self.state.minions.push(Minion {
            owner,
            card,
            blood,
            locked: false,
            in_torpor,
            in_play: true,
            contested: false,
            title_contested: false,
            title_lost: false,
            bled_this_turn: false,
        });
        MinionId(self.state.minions.len() as u32 - 1)
    }

    /// Test fixture: enter the card and title contests `m` would have
    /// entered on coming into play.
    pub fn stage_contests(&mut self, m: MinionId) {
        self.enter_card_contest(m);
        self.enter_title_contest(m);
    }

    /// Advance until a real decision is needed or the game is over. Windows
    /// with a single legal option are elided (DESIGN §4.3): auto-applied and
    /// not logged.
    pub fn run(&mut self) -> Step {
        loop {
            if matches!(self.flow, Flow::GameOver) {
                return Step::Over(self.state.seats.iter().map(|s| s.half_vps).collect());
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
            Flow::Contests { items, at } => {
                let item = *items.get(*at)?;
                let options = match item {
                    Contest::Card(m) if st.card_rivals(m).is_empty() => return None,
                    Contest::Title(m) if st.title_rivals(m).is_empty() => return None,
                    // Paying is always offered for a card: a living
                    // Methuselah has pool, and paying down to 0 is legal.
                    Contest::Card(_) => vec![Choice::PayContest, Choice::YieldContest],
                    Contest::Title(m) => {
                        let v = &st.minions[m.index()];
                        // Only ready vampires contest titles; no blood means
                        // a forced yield.
                        if v.in_torpor || v.blood == 0 {
                            vec![Choice::YieldContest]
                        } else {
                            vec![Choice::PayContest, Choice::YieldContest]
                        }
                    }
                };
                Some(DecisionRequest {
                    seat: st.active,
                    window: WindowLabel::UnlockContest,
                    options,
                })
            }
            Flow::MinionPhase => Some(DecisionRequest {
                seat: st.active,
                window: WindowLabel::MinionPhase,
                options: self.minion_phase_options(),
            }),
            Flow::Block(ctx) => {
                let seat = ctx.window.current()?;
                let mut options = vec![Choice::Pass];
                // P0 elision (DESIGN §4.3; owner-approved 2026-09-02): with
                // no intercept source in the card-less pool an attempt
                // against stealth is provably futile, so it is pruned rather
                // than offered. P1 replaces this with intercept
                // reachability; the closure-time comparison in
                // `resolve_action` is the actual semantics.
                let intercept = 0;
                if ctx.attempt.is_none()
                    && !ctx.declined.contains(&seat)
                    && intercept >= ctx.stealth
                {
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
            Flow::RescueCost { acting, target } => {
                let a = st.minions[acting.index()].blood;
                let t = st.minions[target.index()].blood;
                let options = (0..=2u8)
                    .filter(|&k| a >= k && t >= 2 - k)
                    .map(|k| Choice::PayRescue { from_acting: k })
                    .collect();
                Some(DecisionRequest {
                    seat: st.active,
                    window: WindowLabel::RescueCost,
                    options,
                })
            }
            Flow::BloodHunt(ctx) => {
                let seat = ctx.window.current()?;
                let mut options = vec![Choice::Pass];
                for id in st.minions_of(seat) {
                    // Torpor vampires must abstain; locked ready ones may
                    // vote; contested titles carry no votes.
                    if st.minions[id.index()].votes() > 0 && !ctx.spent.contains(&id) {
                        options.push(Choice::CastVotes {
                            minion: id,
                            favor: true,
                        });
                        options.push(Choice::CastVotes {
                            minion: id,
                            favor: false,
                        });
                    }
                }
                if st.seats[seat.index()].has_edge && !ctx.edge_spent {
                    options.push(Choice::CastEdge { favor: true });
                    options.push(Choice::CastEdge { favor: false });
                }
                Some(DecisionRequest {
                    seat,
                    window: WindowLabel::BloodHuntPolling,
                    options,
                })
            }
            Flow::Influence { left } => {
                let s = st.active;
                let seat = &st.seats[s.index()];
                let mut options = vec![Choice::Pass];
                for (i, slot) in seat.uncontrolled.iter().enumerate() {
                    let i = i as u8;
                    let capacity = CRYPT[slot.card as usize].capacity;
                    if *left >= 1 && seat.pool > 0 && slot.counters < capacity {
                        options.push(Choice::Transfer { slot: i });
                    }
                    if *left >= 2 && slot.counters > 0 {
                        options.push(Choice::Withdraw { slot: i });
                    }
                    // No voluntary self-contest: a second copy of a vampire
                    // you already have in play cannot be moved out.
                    if slot.counters >= capacity && !st.controls_copy(s, slot.card) {
                        options.push(Choice::InfluenceOut { slot: i });
                    }
                }
                if *left >= 4 && seat.pool > 0 && !seat.crypt.is_empty() {
                    options.push(Choice::DrawCrypt);
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

    /// Rulebook "Minion Phase": a ready unlocked vampire with no blood must
    /// hunt, and no non-mandatory action may be taken while any such hunt is
    /// pending — so the phase cannot even end.
    fn minion_phase_options(&self) -> Vec<Choice> {
        let st = &self.state;
        let s = st.active;
        let ready = st.ready_unlocked(s);
        let starving: Vec<MinionId> = ready
            .iter()
            .copied()
            .filter(|m| st.minions[m.index()].blood == 0)
            .collect();
        if !starving.is_empty() {
            return starving
                .into_iter()
                .map(|minion| Choice::TakeAction {
                    minion,
                    action: ActionKind::Hunt,
                    target: None,
                })
                .collect();
        }
        let mut options = vec![Choice::Pass];
        let torpid = st.in_torpor();
        for id in st.minions_of(s) {
            let m = &st.minions[id.index()];
            if m.locked {
                continue;
            }
            if m.in_torpor {
                // Leave torpor: the 2-blood cost must be payable.
                if m.blood >= 2 {
                    options.push(Choice::TakeAction {
                        minion: id,
                        action: ActionKind::LeaveTorpor,
                        target: None,
                    });
                }
                continue;
            }
            if !m.bled_this_turn {
                options.push(Choice::TakeAction {
                    minion: id,
                    action: ActionKind::Bleed,
                    target: None,
                });
            }
            options.push(Choice::TakeAction {
                minion: id,
                action: ActionKind::Hunt,
                target: None,
            });
            for &t in &torpid {
                // Rescue: 2 blood payable by acting, rescued, or split.
                if m.blood + st.minions[t.index()].blood >= 2 {
                    options.push(Choice::TakeAction {
                        minion: id,
                        action: ActionKind::Rescue,
                        target: Some(t),
                    });
                }
                options.push(Choice::TakeAction {
                    minion: id,
                    action: ActionKind::Diablerie,
                    target: Some(t),
                });
            }
        }
        options
    }

    /// Deterministic transitions for states with no decision to offer.
    fn advance(&mut self) {
        let flow = std::mem::replace(&mut self.flow, Flow::GameOver);
        match flow {
            // No Edge to consider: on to contest upkeep.
            Flow::Unlock => self.start_contests(),
            Flow::Contests { items, at } => match items.get(at) {
                // A contested object with no rival left returns for free
                // "during your next unlock phase".
                Some(Contest::Card(m)) => {
                    self.state.minions[m.index()].contested = false;
                    self.enter_title_contest(*m);
                    self.flow = Flow::Contests { items, at: at + 1 };
                }
                Some(Contest::Title(m)) => {
                    self.state.minions[m.index()].title_contested = false;
                    self.flow = Flow::Contests { items, at: at + 1 };
                }
                // Master phase has no cards in P0 → elided.
                None => self.flow = Flow::MinionPhase,
            },
            Flow::Block(ctx) => {
                debug_assert!(ctx.window.is_closed());
                self.resolve_action(ctx);
            }
            Flow::BloodHunt(ctx) => {
                debug_assert!(ctx.window.is_closed());
                self.resolve_blood_hunt(ctx);
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
                self.start_contests();
            }
            (Flow::Unlock, Choice::Pass) => self.start_contests(),
            (
                Flow::Contests { items, at },
                choice @ (Choice::PayContest | Choice::YieldContest),
            ) => {
                let s = self.state.active;
                let pay = *choice == Choice::PayContest;
                match items[at] {
                    Contest::Card(m) => {
                        if pay {
                            self.state.seats[s.index()].pool -= 1;
                        } else {
                            // Yielded: burned with everything on it.
                            let v = &mut self.state.minions[m.index()];
                            v.in_play = false;
                            v.blood = 0;
                        }
                    }
                    Contest::Title(m) => {
                        let v = &mut self.state.minions[m.index()];
                        if pay {
                            v.blood -= 1;
                        } else {
                            v.title_contested = false;
                            v.title_lost = true;
                        }
                    }
                }
                self.flow = Flow::Contests { items, at: at + 1 };
                if self.state.seats[s.index()].pool == 0 {
                    self.oust_and_pass(s);
                }
            }
            (Flow::MinionPhase, Choice::Pass) => {
                self.flow = Flow::Influence {
                    left: transfers_for_turn(self.state.turn),
                };
            }
            (
                Flow::MinionPhase,
                Choice::TakeAction {
                    minion,
                    action,
                    target,
                },
            ) => self.declare_action(*minion, *action, *target),
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
            (Flow::RescueCost { acting, target }, Choice::PayRescue { from_acting }) => {
                self.state.minions[acting.index()].blood -= from_acting;
                self.state.minions[target.index()].blood -= 2 - from_acting;
                // Rescued: to the ready region, lock state untouched.
                self.state.minions[target.index()].in_torpor = false;
                self.flow = Flow::MinionPhase;
            }
            (Flow::BloodHunt(mut ctx), Choice::Pass) => {
                ctx.window.pass();
                self.flow = Flow::BloodHunt(ctx);
            }
            (Flow::BloodHunt(mut ctx), Choice::CastVotes { minion, favor }) => {
                let n = self.state.minions[minion.index()].votes() as u32;
                if *favor {
                    ctx.for_votes += n;
                } else {
                    ctx.against_votes += n;
                }
                ctx.spent.push(*minion);
                ctx.window.commit();
                self.flow = Flow::BloodHunt(ctx);
            }
            (Flow::BloodHunt(mut ctx), Choice::CastEdge { favor }) => {
                let seat = ctx.window.current().expect("commit on closed window");
                self.state.seats[seat.index()].has_edge = false;
                if *favor {
                    ctx.for_votes += 1;
                } else {
                    ctx.against_votes += 1;
                }
                ctx.edge_spent = true;
                ctx.window.commit();
                self.flow = Flow::BloodHunt(ctx);
            }
            (Flow::Influence { left }, Choice::Transfer { slot }) => {
                let s = self.state.active;
                let seat = &mut self.state.seats[s.index()];
                seat.pool -= 1;
                seat.uncontrolled[*slot as usize].counters += 1;
                self.flow = Flow::Influence { left: left - 1 };
                if seat.pool == 0 {
                    self.oust_and_pass(s);
                }
            }
            (Flow::Influence { left }, Choice::Withdraw { slot }) => {
                let s = self.state.active;
                let seat = &mut self.state.seats[s.index()];
                seat.pool += 1;
                seat.uncontrolled[*slot as usize].counters -= 1;
                self.flow = Flow::Influence { left: left - 2 };
            }
            (Flow::Influence { left }, Choice::DrawCrypt) => {
                let s = self.state.active;
                let seat = &mut self.state.seats[s.index()];
                seat.pool -= 1;
                let card = seat.crypt.pop().expect("draw from empty crypt");
                seat.uncontrolled
                    .push(UncontrolledSlot { card, counters: 0 });
                self.flow = Flow::Influence { left: left - 4 };
                if seat.pool == 0 {
                    self.oust_and_pass(s);
                }
            }
            (Flow::Influence { left }, Choice::InfluenceOut { slot }) => {
                let s = self.state.active;
                let out = self.state.seats[s.index()]
                    .uncontrolled
                    .remove(*slot as usize);
                // Counters become blood; excess over capacity drains off.
                let capacity = CRYPT[out.card as usize].capacity;
                let m = self.add_minion(s, out.card, capacity, false);
                self.stage_contests(m);
                self.flow = Flow::Influence { left };
            }
            (Flow::Influence { .. }, Choice::Pass) => {
                // Discard phase: no cards in P0 → elided.
                self.next_turn();
            }
            (flow, choice) => unreachable!("illegal choice {choice:?} in {flow:?}"),
        }
    }

    /// Unlock-phase contest upkeep for the active seat, after the Edge
    /// decision. Ordering call (DESIGN §10): Edge first, then contests in
    /// arena order — the only line this drops is paying oneself to 0 pool
    /// before taking the Edge's pool.
    fn start_contests(&mut self) {
        let s = self.state.active;
        let items: Vec<Contest> = self
            .state
            .minions
            .iter()
            .enumerate()
            .filter(|(_, m)| m.owner == s && m.in_play)
            .flat_map(|(i, m)| {
                let id = MinionId(i as u32);
                let card = m.contested.then_some(Contest::Card(id));
                let title = (m.title_contested && !m.contested).then_some(Contest::Title(id));
                card.into_iter().chain(title)
            })
            .collect();
        self.flow = Flow::Contests { items, at: 0 };
    }

    /// A copy of a unique vampire entering play while another Methuselah has
    /// one: every copy is contested (face down, out of play).
    fn enter_card_contest(&mut self, m: MinionId) {
        let rivals = self.state.card_rivals(m);
        if !rivals.is_empty() {
            self.state.minions[m.index()].contested = true;
            for r in rivals {
                self.state.minions[r.index()].contested = true;
            }
        }
    }

    /// A vampire (re-)entering play whose unique title is already claimed:
    /// the title is contested for every claimant.
    fn enter_title_contest(&mut self, m: MinionId) {
        if !self.state.minions[m.index()].present() || self.state.minions[m.index()].title_lost {
            return;
        }
        let rivals = self.state.title_rivals(m);
        if !rivals.is_empty() {
            self.state.minions[m.index()].title_contested = true;
            for r in rivals {
                self.state.minions[r.index()].title_contested = true;
            }
        }
    }

    /// Announce an action: lock the acting minion, compute stealth and the
    /// seats entitled to block (rulebook "Who May Attempt to Block"), open
    /// the block loop.
    fn declare_action(&mut self, minion: MinionId, action: ActionKind, target: Option<MinionId>) {
        self.state.minions[minion.index()].locked = true;
        let acting_seat = self.state.active;
        let own_target = target.map(|t| self.state.minions[t.index()].owner == acting_seat);
        let (stealth, mut order) = match action {
            // Directed at the prey, 0 stealth: only the defender may block.
            ActionKind::Bleed => (0, vec![self.state.prey(acting_seat)]),
            // Undirected, +1 stealth: prey then predator.
            ActionKind::Hunt | ActionKind::LeaveTorpor => (
                1,
                vec![
                    self.state.prey(acting_seat),
                    self.state.predator(acting_seat),
                ],
            ),
            ActionKind::Rescue | ActionKind::Diablerie => {
                let t = target.expect("torpor target");
                if own_target == Some(true) {
                    (
                        1,
                        vec![
                            self.state.prey(acting_seat),
                            self.state.predator(acting_seat),
                        ],
                    )
                } else {
                    // Directed at the target's controller, 0 stealth.
                    (0, vec![self.state.minions[t.index()].owner])
                }
            }
        };
        order.dedup();
        self.flow = Flow::Block(BlockCtx {
            acting: minion,
            action,
            target,
            stealth,
            window: OrderedWindow::new(order),
            attempt: None,
            declined: Vec::new(),
        });
    }

    fn resolve_action(&mut self, ctx: BlockCtx) {
        let intercept = 0u8;
        match ctx.attempt {
            Some(blocker) if intercept >= ctx.stealth => {
                // Block resolution: the blocker locks and combat begins.
                self.state.minions[blocker.index()].locked = true;
                if ctx.action == ActionKind::Bleed {
                    // NRA triggers on resolution, blocked included (rules-qa 1.4).
                    self.state.minions[ctx.acting.index()].bled_this_turn = true;
                }
                match ctx.action {
                    // Rulebook "Leave Torpor": a blocked leave-torpor has no
                    // combat; a vampire blocker may diablerise instead. Never
                    // reached in P0 (+1 stealth, no intercept source) — P1
                    // implements the blocker's diablerie opportunity here.
                    ActionKind::LeaveTorpor => unreachable!("leave torpor blocked in P0"),
                    _ => self.combat(ctx.acting, blocker),
                }
                self.flow = Flow::MinionPhase;
            }
            // A pending attempt that fails at closure: the action proceeds.
            _ => self.succeed(ctx),
        }
    }

    fn succeed(&mut self, ctx: BlockCtx) {
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
                m.blood = (m.blood + 1).min(m.capacity());
            }
            ActionKind::LeaveTorpor => {
                // Cost is paid at resolution only (a blocked action pays
                // nothing).
                let m = &mut self.state.minions[ctx.acting.index()];
                m.blood -= 2;
                m.in_torpor = false;
            }
            ActionKind::Rescue => {
                self.flow = Flow::RescueCost {
                    acting: ctx.acting,
                    target: ctx.target.expect("rescue target"),
                };
            }
            ActionKind::Diablerie => {
                let victim = ctx.target.expect("diablerie target");
                // Rulebook "Diablerie" steps 1–3 as a single unit: blood to
                // the diablerist (excess drains), victim burned. Equipment
                // and the advanced rule 4 (Discipline for an elder victim)
                // need cards — P1.
                let taken = self.state.minions[victim.index()].blood;
                self.state.minions[victim.index()].blood = 0;
                self.state.minions[victim.index()].in_play = false;
                let d = &mut self.state.minions[ctx.acting.index()];
                d.blood = (d.blood + taken).min(d.capacity());
                // The blood hunt referendum follows immediately; rotation
                // starts at the acting Methuselah, clockwise (DESIGN §4.2 —
                // a modeling call, owner-approved 2026-09-02).
                let order = self.state.clockwise_from(self.state.active);
                self.flow = Flow::BloodHunt(BloodHuntCtx {
                    diablerist: ctx.acting,
                    window: PollingWindow::new(order),
                    spent: Vec::new(),
                    edge_spent: false,
                    for_votes: 0,
                    against_votes: 0,
                });
            }
        }
    }

    fn resolve_blood_hunt(&mut self, ctx: BloodHuntCtx) {
        // Passes only with strictly more votes for than against; ties fail.
        if ctx.for_votes > ctx.against_votes {
            self.state.minions[ctx.diablerist.index()].in_play = false;
        }
        self.flow = Flow::MinionPhase;
    }

    /// P0 combat: close range, one round, simultaneous hand strikes for 1.
    /// Both combatants are ready (torpor vampires never enter combat).
    fn combat(&mut self, a: MinionId, b: MinionId) {
        debug_assert!(self.state.minions[a.index()].ready());
        debug_assert!(self.state.minions[b.index()].ready());
        self.damage(b, 1);
        self.damage(a, 1);
    }

    /// Damage a vampire: burn blood; a vampire that cannot mend goes to
    /// torpor, and one already in torpor is burned.
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

    /// Pool reached 0 on the seat's own turn (a transfer, a crypt draw, or
    /// contest upkeep): ousted at once, and the turn passes.
    fn oust_and_pass(&mut self, s: SeatId) {
        self.oust(s);
        if self.state.living() > 1 {
            self.next_turn();
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
        self.state.seats[pred.index()].half_vps += 2;
        self.state.seats[pred.index()].pool += 6;
        if self.state.living() == 1 {
            // The last Methuselah standing gains one additional VP.
            let survivor = (0..self.state.seats.len())
                .find(|&i| !self.state.seats[i].ousted)
                .expect("one seat must survive");
            self.state.seats[survivor].half_vps += 2;
            self.flow = Flow::GameOver;
        }
    }

    fn next_turn(&mut self) {
        if self.state.turn >= TURN_LIMIT {
            // Time limit (tournament rules 3.7): 0.5 VP per survivor.
            for seat in &mut self.state.seats {
                if !seat.ousted {
                    seat.half_vps += 1;
                }
            }
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
    use crate::crypt::{Sect, Title};
    use crate::state::find_card;

    /// Untitled vampires for generic fixtures.
    fn plain(i: usize) -> u16 {
        find_card(["Abraham DuSable", "Abraham Mellon", "Abaddon"][i])
    }

    /// 3 seats. Seat 0 has two ready vampires (`MinionId(0)`, `MinionId(1)`)
    /// so `run()` always halts at seat 0's minion phase instead of eliding
    /// through to the next turn; seat 1 has one (`MinionId(2)`).
    fn bare() -> Engine {
        let mut e = Engine::bare(3);
        e.add_minion(SeatId(0), plain(0), 5, false);
        e.add_minion(SeatId(0), plain(1), 5, false);
        e.add_minion(SeatId(1), plain(2), 5, false);
        e
    }

    fn act(minion: MinionId, action: ActionKind) -> Choice {
        Choice::TakeAction {
            minion,
            action,
            target: None,
        }
    }

    fn request(e: &mut Engine) -> DecisionRequest {
        match e.run() {
            Step::Decision(req) => req,
            Step::Over(_) => panic!("game over while expecting a decision"),
        }
    }

    fn pick(e: &mut Engine, want: Choice) {
        let req = request(e);
        let idx = req
            .options
            .iter()
            .position(|o| *o == want)
            .unwrap_or_else(|| panic!("option {want:?} not in {:?}", req.options));
        e.choose(idx);
    }

    fn expect(e: &mut Engine, seat: SeatId, window: WindowLabel) -> DecisionRequest {
        let req = request(e);
        assert_eq!((req.seat, req.window), (seat, window), "{:?}", req.options);
        req
    }

    #[test]
    fn crypt_data_is_complete() {
        assert_eq!(CRYPT.len(), 200);
        assert_eq!(PRECONS.len(), 14);
        for (name, list) in PRECONS.iter() {
            let n: u32 = list.iter().map(|&(_, k)| k as u32).sum();
            assert_eq!(n, 12, "{name}");
        }
        let horst = &CRYPT[find_card("Horst von Brühl") as usize];
        assert_eq!(horst.title, Title::Prince("Mannheim"));
        assert_eq!(horst.sect, Sect::Camarilla);
        let aline = &CRYPT[find_card("Aline Gädeke") as usize];
        assert_eq!(aline.title, Title::Baron("Mannheim"));
        assert_eq!(aline.sect, Sect::Anarch);
    }

    #[test]
    fn new_game_deals_four_crypt_cards_from_a_precon() {
        let e = Engine::new(3, 4);
        for s in &e.state.seats {
            assert_eq!(s.uncontrolled.len(), 4);
            assert_eq!(s.crypt.len(), 8);
        }
    }

    #[test]
    fn hunt_has_stealth_so_its_block_window_elides() {
        let mut e = bare();
        e.state.minions[0].blood = 3;
        let logged = e.log.len();
        pick(&mut e, act(MinionId(0), ActionKind::Hunt));
        // No seat is ever asked: the +1 stealth action resolves at once and
        // control returns to seat 0's minion phase.
        expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.log.len(), logged + 1);
        assert_eq!(e.state.minions[0].blood, 4);
    }

    #[test]
    fn leave_torpor_has_stealth_and_pays_at_resolution() {
        let mut e = bare();
        let t = e.add_minion(SeatId(0), find_card("Alice Chen"), 3, true);
        let poor = e.add_minion(SeatId(0), find_card("Alexa Draper"), 1, true);
        let req = request(&mut e);
        assert!(req.options.contains(&act(t, ActionKind::LeaveTorpor)));
        assert!(!req.options.contains(&act(poor, ActionKind::LeaveTorpor))); // cannot pay
        let logged = e.log.len();
        pick(&mut e, act(t, ActionKind::LeaveTorpor));
        // +1 stealth: no block window; the cost is paid at resolution.
        expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.log.len(), logged + 1);
        let m = &e.state.minions[t.index()];
        assert!((!m.in_torpor, m.blood, m.locked) == (true, 1, true));
    }

    #[test]
    fn mandatory_hunt_gates_the_whole_minion_phase() {
        let mut e = bare();
        e.state.minions[0].blood = 0;
        e.state.minions[1].blood = 0;
        // Two starving vampires: only their hunts, in either order, no Pass.
        let req = request(&mut e);
        assert_eq!(
            req.options,
            vec![
                act(MinionId(0), ActionKind::Hunt),
                act(MinionId(1), ActionKind::Hunt)
            ]
        );
        pick(&mut e, act(MinionId(1), ActionKind::Hunt));
        // One left: a single mandatory option is elided, not offered.
        let logged = e.log.len();
        let req = request(&mut e);
        assert_eq!(e.log.len(), logged);
        assert_eq!(e.state.minions[0].blood, 1);
        assert!(e.state.minions[0].locked);
        assert!(req.options.contains(&Choice::Pass));
    }

    #[test]
    fn rescue_of_another_controllers_vampire_is_directed_at_them() {
        let mut e = bare();
        let t = e.add_minion(SeatId(2), find_card("Alice Chen"), 3, true);
        let guard = e.add_minion(SeatId(2), find_card("Alexa Draper"), 5, false);
        pick(
            &mut e,
            Choice::TakeAction {
                minion: MinionId(0),
                action: ActionKind::Rescue,
                target: Some(t),
            },
        );
        // Directed at seat 2 (not the prey): seat 2 alone may block.
        let req = expect(&mut e, SeatId(2), WindowLabel::ActionBlock);
        assert!(req
            .options
            .contains(&Choice::DeclareBlock { minion: guard }));
        assert!(!req.options.contains(&Choice::DeclareBlock { minion: t })); // torpor cannot block
        pick(&mut e, Choice::Pass);
        // Cost split at resolution: 2/1/0 from the acting vampire.
        let req = expect(&mut e, SeatId(0), WindowLabel::RescueCost);
        assert_eq!(req.options.len(), 3);
        pick(&mut e, Choice::PayRescue { from_acting: 0 });
        assert!(!e.state.minions[t.index()].in_torpor);
        assert_eq!(e.state.minions[t.index()].blood, 1);
        assert_eq!(e.state.minions[0].blood, 5);
        assert!(!e.state.minions[t.index()].locked);
    }

    #[test]
    fn rescue_of_own_vampire_has_stealth_and_forced_split_elides() {
        let mut e = bare();
        let t = e.add_minion(SeatId(0), find_card("Alice Chen"), 1, true);
        e.state.minions[0].blood = 1;
        pick(
            &mut e,
            Choice::TakeAction {
                minion: MinionId(0),
                action: ActionKind::Rescue,
                target: Some(t),
            },
        );
        // +1 stealth, no block window; only split 1/1 is payable → elided.
        // The rescued vampire comes out ready at 0 blood, so its mandatory
        // hunt is elided too, straight away.
        let logged = e.log.len();
        expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.log.len(), logged);
        assert_eq!(e.state.minions[0].blood, 0);
        assert!(!e.state.minions[t.index()].in_torpor);
        assert_eq!(e.state.minions[t.index()].blood, 1);
        assert!(e.state.minions[t.index()].locked);
    }

    #[test]
    fn diablerie_moves_blood_burns_victim_and_calls_the_blood_hunt() {
        let mut e = bare();
        let victim = e.add_minion(SeatId(1), find_card("Alice Chen"), 4, true);
        // Seat 1's ready vampire is a Prince: 2 votes.
        let prince = e.add_minion(SeatId(1), find_card("Horst von Brühl"), 5, false);
        e.state.minions[0].blood = 6;
        pick(
            &mut e,
            Choice::TakeAction {
                minion: MinionId(0),
                action: ActionKind::Diablerie,
                target: Some(victim),
            },
        );
        pick(&mut e, Choice::Pass); // seat 1 declines to block
                                    // Polling: seat 0 has no source (elided), seat 1 is asked.
        expect(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
        assert!(!e.state.minions[victim.index()].in_play);
        assert_eq!(e.state.minions[0].blood, 8); // 6 + 4 capped at capacity 8
        pick(
            &mut e,
            Choice::CastVotes {
                minion: prince,
                favor: true,
            },
        );
        // Full quiet rotation closes; 2 for > 0 against: diablerist burned.
        expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.minions[0].in_play);
    }

    #[test]
    fn blood_hunt_with_no_vote_source_fails_silently() {
        let mut e = bare();
        let victim = e.add_minion(SeatId(1), find_card("Alice Chen"), 0, true);
        let logged = e.log.len();
        pick(
            &mut e,
            Choice::TakeAction {
                minion: MinionId(0),
                action: ActionKind::Diablerie,
                target: Some(victim),
            },
        );
        pick(&mut e, Choice::Pass);
        expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.log.len(), logged + 2); // the whole referendum elided
        assert!(e.state.minions[0].in_play);
    }

    #[test]
    fn transfers_ramp_by_game_turn_not_by_seat() {
        assert_eq!(transfers_for_turn(1), 1);
        assert_eq!(transfers_for_turn(2), 2);
        assert_eq!(transfers_for_turn(3), 3);
        assert_eq!(transfers_for_turn(4), 4);
        assert_eq!(transfers_for_turn(9), 4);
        // Seat 1's first influence phase is game turn 2: two transfers.
        let mut e = Engine::new(7, 3);
        let req = expect(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
        assert!(!req.options.contains(&Choice::DrawCrypt)); // needs 4 transfers
        pick(&mut e, Choice::Transfer { slot: 0 });
        expect(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
        pick(&mut e, Choice::Transfer { slot: 0 });
        let req = expect(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
        assert!(req.options.contains(&Choice::Transfer { slot: 1 }));
        assert!(!req.options.contains(&Choice::Withdraw { slot: 0 })); // needs 2 transfers
    }

    #[test]
    fn crypt_draw_costs_four_transfers_and_one_pool() {
        let mut e = bare();
        e.state.seats[0].crypt = vec![plain(2), find_card("Alice Chen")];
        e.state.turn = 4;
        pick(&mut e, Choice::Pass); // end minion phase: 4 transfers
        pick(&mut e, Choice::DrawCrypt);
        assert_eq!(e.state.seats[0].pool, START_POOL - 1);
        assert_eq!(e.state.seats[0].crypt, vec![plain(2)]);
        assert_eq!(
            e.state.seats[0].uncontrolled[0].card,
            find_card("Alice Chen")
        );
        // No transfers left: only Pass remains and the phase elides away.
        expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
    }

    #[test]
    fn influence_out_is_explicit_withdraw_costs_two_and_no_self_contest() {
        let mut e = bare();
        // Seat 0 already controls Abraham DuSable (MinionId(0)): a second
        // copy cannot be moved out (no voluntary self-contest).
        e.state.seats[0].uncontrolled.push(UncontrolledSlot {
            card: plain(0),
            counters: CRYPT[plain(0) as usize].capacity,
        });
        let alice = find_card("Alice Chen");
        let cap = CRYPT[alice as usize].capacity;
        e.state.seats[0].uncontrolled.push(UncontrolledSlot {
            card: alice,
            counters: cap,
        });
        e.state.turn = 4;
        pick(&mut e, Choice::Pass); // end minion phase
        let req = expect(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
        assert!(!req.options.contains(&Choice::InfluenceOut { slot: 0 }));
        assert!(req.options.contains(&Choice::InfluenceOut { slot: 1 }));
        assert!(req.options.contains(&Choice::Withdraw { slot: 1 }));
        assert!(!req.options.contains(&Choice::Transfer { slot: 1 })); // already full
        pick(&mut e, Choice::Withdraw { slot: 1 });
        assert_eq!(e.state.seats[0].pool, START_POOL + 1);
        let req = request(&mut e);
        assert!(!req.options.contains(&Choice::InfluenceOut { slot: 1 }));
        pick(&mut e, Choice::Transfer { slot: 1 });
        pick(&mut e, Choice::InfluenceOut { slot: 1 });
        let m = e.state.minions.last().unwrap();
        assert_eq!(m.name(), "Alice Chen");
        assert_eq!(
            (m.owner, m.blood, m.votes(), m.in_torpor),
            (SeatId(0), cap, 2, false)
        );
        assert_eq!(e.state.seats[0].uncontrolled.len(), 1);
    }

    #[test]
    fn card_contest_pay_yield_and_return() {
        // Seat 1 brings out Abraham Mellon while seat 0 has one (MinionId(1)):
        // both go face down; seat 0 pays, seat 1 yields, seat 0's returns.
        let mut e = bare();
        e.state.seats[1].uncontrolled.push(UncontrolledSlot {
            card: plain(1),
            counters: CRYPT[plain(1) as usize].capacity,
        });
        e.state.turn = 4;
        pick(&mut e, Choice::Pass); // seat 0 minion phase; its influence elides
        pick(&mut e, Choice::Pass); // seat 1 minion phase
        pick(&mut e, Choice::InfluenceOut { slot: 0 });
        let copy = MinionId(3);
        assert!(e.state.minions[1].contested && e.state.minions[copy.index()].contested);
        // Seat 2 has nothing and elides; seat 0's unlock phase: upkeep.
        let req = expect(&mut e, SeatId(0), WindowLabel::UnlockContest);
        assert_eq!(req.options, vec![Choice::PayContest, Choice::YieldContest]);
        pick(&mut e, Choice::PayContest);
        assert_eq!(e.state.seats[0].pool, START_POOL - 1);
        assert!(e.state.minions[1].contested);
        // Contested copies are out of play: seat 0 cannot act with it.
        let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!req
            .options
            .iter()
            .any(|o| matches!(o, Choice::TakeAction { minion, .. } if *minion == MinionId(1))));
        pick(&mut e, Choice::Pass);
        // Seat 1 yields: its copy is burned.
        expect(&mut e, SeatId(1), WindowLabel::UnlockContest);
        pick(&mut e, Choice::YieldContest);
        assert!(!e.state.minions[copy.index()].in_play);
        expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
        pick(&mut e, Choice::Pass);
        // Seat 0's next unlock: no rival left, the copy returns for free.
        let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.minions[1].contested);
        assert!(req.options.contains(&act(MinionId(1), ActionKind::Hunt)));
    }

    #[test]
    fn title_contest_prince_and_baron_of_one_city() {
        let mut e = bare();
        let horst = e.add_minion(SeatId(0), find_card("Horst von Brühl"), 4, false);
        let aline = e.add_minion(SeatId(1), find_card("Aline Gädeke"), 2, false);
        e.stage_contests(aline);
        assert!(e.state.minions[horst.index()].title_contested);
        assert_eq!(e.state.minions[horst.index()].votes(), 0);
        assert_eq!(e.state.minions[aline.index()].votes(), 0);
        e.state.turn = 4;
        pick(&mut e, Choice::Pass); // seat 0 minion phase
                                    // Seat 1's unlock: Aline pays 1 blood.
        expect(&mut e, SeatId(1), WindowLabel::UnlockContest);
        pick(&mut e, Choice::PayContest);
        assert_eq!(e.state.minions[aline.index()].blood, 1);
        expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
        pick(&mut e, Choice::Pass);
        // Seat 0's unlock: Horst pays too.
        expect(&mut e, SeatId(0), WindowLabel::UnlockContest);
        pick(&mut e, Choice::PayContest);
        assert_eq!(e.state.minions[horst.index()].blood, 3);
        expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        pick(&mut e, Choice::Pass);
        // Seat 1's unlock: Aline yields — titleless for the rest of the game.
        expect(&mut e, SeatId(1), WindowLabel::UnlockContest);
        pick(&mut e, Choice::YieldContest);
        let a = &e.state.minions[aline.index()];
        assert!(a.title_lost && !a.title_contested && a.in_play);
        expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
        pick(&mut e, Choice::Pass);
        // Seat 0's unlock: no rival left, Horst acquires the title for free.
        expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.minions[horst.index()].title_contested);
        assert_eq!(e.state.minions[horst.index()].votes(), 2);
    }

    #[test]
    fn title_contest_torpor_and_no_blood_force_a_yield() {
        let mut e = bare();
        let horst = e.add_minion(SeatId(0), find_card("Horst von Brühl"), 0, false);
        let aline = e.add_minion(SeatId(1), find_card("Aline Gädeke"), 3, true);
        e.stage_contests(aline);
        e.state.minions[0].blood = 0; // keep Horst from being the only starving one
        e.state.turn = 4;
        // Seat 0's minion phase: mandatory hunts for the two starving
        // vampires; take Horst's first.
        pick(&mut e, act(horst, ActionKind::Hunt));
        pick(&mut e, Choice::Pass);
        // Seat 1's unlock: Aline in torpor must yield (elided).
        let logged = e.log.len();
        expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.log.len(), logged);
        assert!(e.state.minions[aline.index()].title_lost);
        // Horst (now at 1 blood, but that is beside the point: no rival) is
        // titled again at his next unlock phase.
        pick(&mut e, Choice::Pass);
        expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[horst.index()].votes(), 2);
    }

    #[test]
    fn time_limit_awards_half_a_vp_per_survivor() {
        let mut e = bare();
        e.state.turn = TURN_LIMIT;
        e.state.seats[2].ousted = true;
        e.state.seats[0].half_vps = 2;
        pick(&mut e, Choice::Pass);
        match e.run() {
            Step::Over(v) => assert_eq!(v, vec![3, 1, 0]),
            Step::Decision(_) => panic!("expected time-out"),
        }
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
        expect(&mut e, SeatId(1), WindowLabel::UnlockEdge);
        let pool = e.state.seats[1].pool;
        pick(&mut e, Choice::BurnEdge);
        assert_eq!(e.state.seats[1].pool, pool + 1);
        assert!(!e.state.seats[1].has_edge);
    }
}
