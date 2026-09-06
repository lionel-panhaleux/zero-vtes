//! The decision-request engine: flow states, the run/choose loop with
//! window elision (DESIGN §4.3), turn structure, ousting and scoring. Phase
//! and window logic lives in the sibling modules (`actions`, `contests`,
//! `influence`, `referendum`), which extend `Engine`.

use crate::actions::ActionCtx;
use crate::cards::{self, Hook, PlayCtx};
use crate::combat::CombatCtx;
use crate::crypt::PRECONS;
use crate::decision::{Choice, DecisionRequest, WindowLabel};
use crate::ids::{MinionId, SeatId};
use crate::influence::transfers_for_turn;
use crate::library::PRECON_LIBRARIES;
use crate::plays::PendingPlay;
use crate::referendum::ReferendumCtx;
use crate::rng::Rng;
use crate::state::{
    CardId, GameState, Minion, Seat, UncontrolledSlot, HAND_SIZE, START_POOL, UNCONTROLLED_START,
};
use crate::window::OrderedWindow;

/// Time-limit stand-in (owner, 2026-09-02): a tournament game rarely exceeds
/// 20 full rounds, so 100 player turns is ample. Checked at the turn
/// boundary, which stands in for the tournament rule "the game ends after the
/// current action / at the end of the current phase".
pub const TURN_LIMIT: u32 = 100;

/// One contested object of the active seat, resolved during its unlock
/// phase (rulebook "Contested Cards" / "Contested Titles").
#[derive(Clone, Copy, Debug)]
pub(crate) enum Contest {
    Card(MinionId),
    Title(MinionId),
    /// A unique library card in play (rulebook "Contested Cards").
    Library(CardId),
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
    /// Master phase: an offer window for the active seat's master phase
    /// actions (rulebook "Master Phase").
    MasterPhase,
    /// Numeric terms of a play, chosen as it is announced (rules-qa 3.5).
    PlayTerms {
        ctx: PlayCtx,
        resume: Box<Flow>,
    },
    /// As-played windows of pending plays, innermost last; `resume` is the
    /// flow to return to once the stack empties.
    AsPlayed {
        stack: Vec<PendingPlay>,
        resume: Box<Flow>,
    },
    /// In-play abilities usable during any Methuselah's unlock phase.
    Abilities {
        window: OrderedWindow,
        /// Mandatory unlock-phase riders the active seat has yet to resolve.
        pending: Vec<CardId>,
    },
    /// Minion phase: offer actions for the active seat's ready minions.
    MinionPhase,
    /// An action is under way (`Engine::action`).
    Action,
    /// A combat is under way inside the action (`Engine::combat`).
    Combat,
    /// A blocked leave-torpor: the blocking vampire may diablerise the
    /// acting vampire (rulebook "Leave Torpor").
    TorporBlock,
    /// Rescue resolved unblocked: split the 2-blood cost (paid at resolution).
    RescueCost,
    /// A referendum is under way (`Engine::referendum`): the blood hunt
    /// after a diablerie, or a political action's (inside the action).
    Referendum,
    Influence {
        left: u8,
    },
    /// Discard phase: an offer window for the discard phase action.
    Discard,
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
    /// The action under way, if any (`Flow::Action` and its sub-windows).
    pub(crate) action: Option<ActionCtx>,
    /// The combat under way inside the action, if any (`Flow::Combat`).
    pub(crate) combat: Option<CombatCtx>,
    /// The referendum under way, if any (`Flow::Referendum`).
    pub(crate) referendum: Option<ReferendumCtx>,
    /// Set when a play's resolution ousted its player: the as-played stack
    /// must not resume the ousted seat's phase.
    pub(crate) ousted_during_play: bool,
    /// How the most recent action ended (tests, and after-resolution
    /// riders to come).
    pub last_outcome: Option<crate::actions::Outcome>,
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
                let precon = rng.below(PRECONS.len());
                let (_, list) = PRECONS[precon];
                let mut crypt: Vec<u16> = list
                    .iter()
                    .flat_map(|&(card, copies)| std::iter::repeat_n(card, copies as usize))
                    .collect();
                rng.shuffle(&mut crypt);
                let uncontrolled = (0..UNCONTROLLED_START)
                    .map(|_| UncontrolledSlot {
                        card: crypt.pop().expect("crypt larger than initial draw"),
                        counters: 0,
                        banished: None,
                    })
                    .collect();
                // The matching precon library, shuffled; hand of 7.
                let (_, list) = PRECON_LIBRARIES[precon];
                let mut library: Vec<u16> = list
                    .iter()
                    .flat_map(|&(card, copies)| std::iter::repeat_n(card, copies as usize))
                    .collect();
                rng.shuffle(&mut library);
                let hand = library.split_off(library.len() - HAND_SIZE);
                Seat {
                    pool: START_POOL,
                    ousted: false,
                    half_vps: 0,
                    has_edge: false,
                    crypt,
                    uncontrolled,
                    library,
                    hand,
                    ash: Vec::new(),
                    mpa: 0,
                    trifle_used: false,
                    out_of_turn: None,
                    dpa: 0,
                    turns_taken: 0,
                    delayed_draws: 0,
                    forbidden_political: Vec::new(),
                }
            })
            .collect();
        let mut engine = Self {
            state: GameState {
                seats,
                minions: Vec::new(),
                cards: Vec::new(),
                phase_uses: Vec::new(),
                layers: Vec::new(),
                active: SeatId(0),
                turn: 0,
            },
            log: Vec::new(),
            decisions: 0,
            flow: Flow::GameOver, // replaced by start_turn
            action: None,
            combat: None,
            referendum: None,
            ousted_during_play: false,
            last_outcome: None,
        };
        engine.start_turn();
        engine
    }

    /// Test fixture: `n_seats` seats with empty crypts, uncontrolled regions,
    /// libraries and hands, no minions, positioned at seat 0's minion phase
    /// on turn 1. Populate with [`Engine::add_minion`] and by pushing cards
    /// into hands.
    pub fn bare(n_seats: usize) -> Self {
        let mut e = Self::new(0, n_seats);
        for seat in &mut e.state.seats {
            seat.crypt.clear();
            seat.uncontrolled.clear();
            seat.library.clear();
            seat.hand.clear();
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
            political_this_turn: false,
            expelled: false,
            hunting_ground_fed: false,
            acted: Vec::new(),
        });
        MinionId(self.state.minions.len() as u32 - 1)
    }

    /// Test fixture: position the game at the active seat's master phase
    /// (master phase actions granted per rules-qa 3.3).
    pub fn at_master_phase(&mut self) {
        self.start_master_phase();
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
            Flow::MasterPhase => Some(self.master_request()),
            Flow::PlayTerms { ctx, .. } => Some(self.terms_request(ctx)),
            Flow::AsPlayed { stack, .. } => self.as_played_request(stack),
            Flow::MinionPhase => Some(DecisionRequest {
                seat: st.active,
                window: WindowLabel::MinionPhase,
                options: self.minion_phase_options(),
            }),
            Flow::Abilities { window, pending } => self.abilities_request(window, pending),
            Flow::Action => self.action_request(),
            Flow::Combat => self.combat_request(),
            Flow::TorporBlock => Some(self.torpor_block_request()),
            Flow::RescueCost => Some(self.rescue_request()),
            Flow::Referendum => self.referendum_request(),
            Flow::Influence { left } => Some(self.influence_request(*left)),
            Flow::Discard => Some(self.discard_request()),
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
            Flow::AsPlayed { stack, resume } => self.as_played_advance(stack, resume),
            Flow::Abilities { window, .. } => {
                debug_assert!(window.is_closed());
                self.start_master_phase();
            }
            Flow::Action => self.action_advance(),
            Flow::Combat => self.combat_advance(),
            Flow::Referendum => self.referendum_advance(),
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
            (Flow::MasterPhase, c) => self.master_apply(c),
            (Flow::PlayTerms { ctx, resume }, c) => self.terms_apply(ctx, *resume, c),
            (Flow::AsPlayed { stack, resume }, c) => self.as_played_apply(stack, resume, c),
            (Flow::Discard, c) => self.discard_apply(c),
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
            ) => self.declare_action(*minion, *action, *target, None, None),
            (
                Flow::MinionPhase,
                Choice::UseAbility {
                    card,
                    ability,
                    actor,
                    target,
                },
            ) => {
                self.use_ability(*card, *ability, *actor, *target);
            }
            (
                Flow::MinionPhase,
                Choice::Play {
                    card,
                    mode,
                    actor,
                    target,
                },
            ) => {
                let s = self.state.active;
                self.announce_play(s, *card, *mode, *actor, *target, Flow::MinionPhase);
            }
            (Flow::Abilities { window, pending }, c) => self.abilities_apply(window, pending, c),
            (Flow::Action, c) => self.action_apply(c),
            (Flow::Combat, c) => self.combat_apply(c),
            (Flow::TorporBlock, c) => self.torpor_block_apply(c),
            (Flow::RescueCost, Choice::PayRescue { from_acting }) => {
                self.rescue_apply(*from_acting)
            }
            (Flow::Referendum, c) => self.referendum_apply(c),
            (Flow::Influence { left }, c) => self.influence_apply(left, c),
            (flow, choice) => unreachable!("illegal choice {choice:?} in {flow:?}"),
        }
    }

    /// End of the active seat's own unlock-phase effects (Edge, contests):
    /// the impulse rotates for other players' unlock-phase abilities
    /// (Homunculus), returning to the active seat after each (owner,
    /// 2026-09-04; rules-qa 3.7), then the master phase.
    pub(crate) fn end_contests(&mut self) {
        // The unlock-phase effects window: the active seat's pending
        // mandatory riders (every copy of Anarch Revolt in play, whoever
        // controls it — each burns on its own, spec §1.15) ordered by them
        // among their optional abilities and resolved before the impulse
        // leaves them (rules-qa 6.1), then the other seats' abilities.
        let s = self.state.active;
        let pending: Vec<CardId> = self
            .state
            .cards
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.present()
                    && cards::lookup(c.card).is_some_and(|d| {
                        d.riders.iter().any(|r| {
                            r.hook == Hook::UnlockPhase
                                && r.scope.applies(c.owner, s)
                                && self.state.rider_holds(r, c.host)
                        })
                    })
            })
            .map(|(i, _)| CardId(i as u32))
            .collect();
        let order = self.state.clockwise_from(s);
        self.flow = Flow::Abilities {
            window: OrderedWindow::new(order),
            pending,
        };
    }

    /// Burn a card in play: to its owner's ash heap, its layers dropped.
    pub(crate) fn burn_card_in_play(&mut self, id: CardId) {
        let c = &mut self.state.cards[id.index()];
        if !c.in_play {
            return;
        }
        c.in_play = false;
        let (owner, card) = (c.owner, c.card);
        self.state.seats[owner.index()].ash.push(card);
        self.state
            .layers
            .retain(|l| l.scope == crate::cards::Scope::Action || l.source != id);
    }

    /// End of the influence phase: on to the discard phase (one discard
    /// phase action; events are a later slice).
    pub(crate) fn end_influence(&mut self) {
        let s = self.state.active;
        self.state.seats[s.index()].dpa = 1;
        self.flow = Flow::Discard;
    }

    /// End of the discard phase: the turn passes.
    pub(crate) fn end_turn(&mut self) {
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
        self.oust_batch(&[s]);
    }

    /// Oust these Methuselahs at once (a referendum's simultaneous pool
    /// losses, spec §6.5.4): each predator scores the victory point even if
    /// ousted in the same batch; the 6 pool goes only to surviving
    /// predators, and only for their own prey.
    pub(crate) fn oust_batch(&mut self, seats: &[SeatId]) {
        let preds: Vec<(SeatId, SeatId)> =
            seats.iter().map(|&s| (s, self.state.predator(s))).collect();
        for &s in seats {
            self.remove_seat(s);
        }
        for (_, pred) in &preds {
            self.state.seats[pred.index()].half_vps += 2;
            if !seats.contains(pred) {
                self.state.seats[pred.index()].pool += 6;
            }
        }
        match self.state.living() {
            // The last Methuselah standing gains one additional VP.
            1 => {
                let survivor = (0..self.state.seats.len())
                    .find(|&i| !self.state.seats[i].ousted)
                    .expect("one seat must survive");
                self.state.seats[survivor].half_vps += 2;
                self.flow = Flow::GameOver;
            }
            // Everyone ousted at once (rules-qa 6.9): the prey VPs stand,
            // nobody is last standing; the game win, if any, follows from
            // the VPs (`GameState::game_win`).
            0 => self.flow = Flow::GameOver,
            _ => {}
        }
    }

    /// Rulebook "Ending the Game": all the cards the ousted Methuselah
    /// controls are removed from the game — with the layers they carried.
    fn remove_seat(&mut self, s: SeatId) {
        {
            let seat = &mut self.state.seats[s.index()];
            seat.ousted = true;
            seat.pool = 0;
            seat.has_edge = false;
            seat.crypt.clear();
            seat.uncontrolled.clear();
            seat.library.clear();
            seat.hand.clear();
            seat.ash.clear();
        }
        for m in &mut self.state.minions {
            if m.owner == s {
                m.in_play = false;
            }
        }
        for c in &mut self.state.cards {
            if c.owner == s {
                c.in_play = false;
            }
        }
        let cards = &self.state.cards;
        self.state
            .layers
            .retain(|l| l.scope == crate::cards::Scope::Action || cards[l.source.index()].in_play);
    }

    pub(crate) fn next_turn(&mut self) {
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
        // An action cannot outlive the turn (the acting Methuselah ousted
        // by its own referendum or a cost): its scoped state goes with it.
        if let Some(a) = self.action.take() {
            for s in a.deferred_draws {
                self.draw_to_hand_size(s);
            }
        }
        self.combat = None;
        self.referendum = None;
        self.state
            .layers
            .retain(|l| l.scope != crate::cards::Scope::Action);
        self.state.active = self.state.prey(self.state.active);
        self.start_turn();
    }

    fn start_turn(&mut self) {
        let s = self.state.active;
        self.state.turn += 1;
        self.state.seats[s.index()].turns_taken += 1;
        for seat in &mut self.state.seats {
            seat.forbidden_political.clear();
        }
        // Delayed replacements ("do not replace until your next unlock
        // phase") are drawn before unlocking (ANK 20200129).
        self.state.seats[s.index()].delayed_draws = 0;
        self.state.phase_uses.clear();
        self.draw_to_hand_size(s);
        for m in &mut self.state.minions {
            // Per-turn NRA flags reset each new turn.
            m.bled_this_turn = false;
            m.political_this_turn = false;
            m.expelled = false;
            m.hunting_ground_fed = false;
            m.acted.clear();
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
    use crate::decision::ActionKind;
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
        // (Empty hands keep the master and discard phases elided.)
        let mut e = Engine::new(7, 3);
        for seat in &mut e.state.seats {
            seat.library.clear();
            seat.hand.clear();
        }
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
            banished: None,
        });
        let alice = find_card("Alice Chen");
        let cap = CRYPT[alice as usize].capacity;
        e.state.seats[0].uncontrolled.push(UncontrolledSlot {
            card: alice,
            counters: cap,
            banished: None,
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
            banished: None,
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

    // rules-qa 6.9: the game win needs 2 VP and a strict lead, ousted or
    // not; ties are draws.
    #[test]
    fn game_win_needs_two_vps_and_a_strict_lead() {
        let mut e = Engine::bare(4);
        let set = |e: &mut Engine, vps: [u32; 4]| {
            for (s, v) in e.state.seats.iter_mut().zip(vps) {
                s.half_vps = v;
            }
        };
        set(&mut e, [6, 4, 0, 0]); // 3 VP (ousted) vs 2: the 3 wins
        e.state.seats[0].ousted = true;
        assert_eq!(e.state.game_win(), Some(SeatId(0)));
        set(&mut e, [4, 4, 0, 0]); // two at 2 VP: a draw
        assert_eq!(e.state.game_win(), None);
        set(&mut e, [2, 2, 2, 2]); // all at 1 VP: a draw
        assert_eq!(e.state.game_win(), None);
        set(&mut e, [3, 2, 1, 0]); // 1.5 VP leads but is under 2
        assert_eq!(e.state.game_win(), None);
        set(&mut e, [4, 2, 0, 0]);
        assert_eq!(e.state.game_win(), Some(SeatId(0)));
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
