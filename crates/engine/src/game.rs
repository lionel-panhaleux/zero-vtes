//! The decision-request engine: flow states, the run/choose loop with
//! window elision (DESIGN §4.3), turn structure, ousting and scoring. Phase
//! and window logic lives in the sibling modules (`actions`, `contests`,
//! `influence`, `referendum`), which extend `Engine`.

use crate::crypt::PRECONS;
use crate::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use crate::ids::{MinionId, SeatId};
use crate::influence::transfers_for_turn;
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
pub(crate) struct BlockCtx {
    pub acting: MinionId,
    pub action: ActionKind,
    pub target: Option<MinionId>,
    pub stealth: u8,
    pub window: OrderedWindow,
    pub attempt: Option<MinionId>,
    pub declined: Vec<SeatId>,
}

/// Blood hunt referendum (rulebook "The Blood Hunt"): not an action, no
/// modifier/reaction windows — polling only. Sources (rules-qa 1.1): each
/// ready vampire with votes, and the Edge; each commits once, all its votes
/// one way, and is then spent for this referendum.
#[derive(Clone, Debug)]
pub(crate) struct BloodHuntCtx {
    pub diablerist: MinionId,
    pub window: PollingWindow,
    pub spent: Vec<MinionId>,
    pub edge_spent: bool,
    pub for_votes: u32,
    pub against_votes: u32,
}

/// One contested object of the active seat, resolved during its unlock
/// phase (rulebook "Contested Cards" / "Contested Titles").
#[derive(Clone, Copy, Debug)]
pub(crate) enum Contest {
    Card(MinionId),
    Title(MinionId),
}

#[derive(Clone, Debug)]
pub(crate) enum Flow {
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
    pub(crate) flow: Flow,
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
            Flow::Contests { items, at } => self.contest_request(items, *at),
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
            Flow::BloodHunt(ctx) => self.blood_hunt_request(ctx),
            Flow::Influence { left } => Some(self.influence_request(*left)),
            Flow::GameOver => None,
        }
    }

    /// Deterministic transitions for states with no decision to offer.
    fn advance(&mut self) {
        let flow = std::mem::replace(&mut self.flow, Flow::GameOver);
        match flow {
            // No Edge to consider: on to contest upkeep.
            Flow::Unlock => self.start_contests(),
            Flow::Contests { items, at } => self.contest_advance(items, at),
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
            (Flow::Contests { items, at }, c @ (Choice::PayContest | Choice::YieldContest)) => {
                self.contest_apply(items, at, *c == Choice::PayContest)
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
            (Flow::BloodHunt(ctx), c) => self.blood_hunt_apply(ctx, c),
            (Flow::Influence { left }, c) => self.influence_apply(left, c),
            (flow, choice) => unreachable!("illegal choice {choice:?} in {flow:?}"),
        }
    }

    /// End of contest upkeep: the master phase has no cards in P0 → elided.
    pub(crate) fn end_contests(&mut self) {
        self.flow = Flow::MinionPhase;
    }

    /// End of the influence phase: the discard phase has no cards in P0 →
    /// elided.
    pub(crate) fn end_influence(&mut self) {
        self.next_turn();
    }

    /// Pool reached 0 on the seat's own turn (a transfer, a crypt draw, or
    /// contest upkeep): ousted at once, and the turn passes.
    pub(crate) fn oust_and_pass(&mut self, s: SeatId) {
        self.oust(s);
        if self.state.living() > 1 {
            self.next_turn();
        }
    }

    pub(crate) fn oust(&mut self, s: SeatId) {
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
    use crate::crypt::{Sect, Title, CRYPT};
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
