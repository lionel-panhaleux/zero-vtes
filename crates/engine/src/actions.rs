//! Actions: announcement, the as-announced window, the block loop (rulebook
//! states A/B/C, rules-qa 1.3), redirects, resolution, and P0 combat. The
//! action context lives on the engine so that card plays resolved from the
//! as-played stack can modify it.

use crate::cards::{self, Effect, PlayCtx, Scope, Window};
use crate::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use crate::game::{BloodHuntCtx, Engine, Flow};
use crate::ids::{MinionId, SeatId};
use crate::library::LIBRARY;
use crate::state::CardInPlay;
use crate::window::{OrderedWindow, PollingWindow};

/// Rulebook "Resolve Any Block Attempts": A (no attempt), B (an attempt is
/// ongoing), C (blocks declined by all); plus the as-announced window that
/// precedes them (spec §2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionState {
    AsAnnounced,
    Open,
    Attempt(MinionId),
    Declined,
}

#[derive(Clone, Debug)]
pub struct ActionCtx {
    pub seat: SeatId,
    pub acting: MinionId,
    pub kind: ActionKind,
    /// The action card, for card actions (employ …).
    pub card: Option<PlayCtx>,
    pub torpor_target: Option<MinionId>,
    /// The Methuselah being bled (changes on a redirect).
    pub bleed_target: Option<SeatId>,
    pub base_stealth: u8,
    pub stealth_mods: u8,
    /// Intercept of the current block attempt (reset per attempt).
    pub intercept: u8,
    pub bleed_mods: u8,
    /// The rulebook's one "(limited)" bleed increase per action.
    pub limited_bleed_used: bool,
    pub state: ActionState,
    pub window: OrderedWindow,
    pub order: Vec<SeatId>,
    /// Methuselahs who passed with no attempt pending: final (rules-qa 1.3).
    pub declined: Vec<SeatId>,
    /// Minions woken for this action.
    pub woken: Vec<MinionId>,
    /// Consign to Oblivion [OBL]: reactions replaced at the end of the action.
    pub defer_reaction_replacement: bool,
    pub deferred_draws: Vec<SeatId>,
    /// Archon Investigation: the action is not successful.
    pub failed: bool,
}

impl ActionCtx {
    pub fn stealth(&self) -> u8 {
        self.base_stealth + self.stealth_mods
    }

    pub fn bleed_amount(&self) -> u8 {
        1 + self.bleed_mods
    }

    /// The Methuselah the action is directed at, if directed.
    pub fn directed_at(&self) -> Option<SeatId> {
        self.bleed_target
    }

    fn fresh_window(&mut self) {
        self.window = OrderedWindow::new(self.order.clone());
    }
}

impl Engine {
    /// Announce an action (rulebook "Detailed action flow"): lock the acting
    /// minion, compute base stealth and the block-eligible seats, open the
    /// as-announced window. Card actions arrive with their play context.
    pub(crate) fn declare_action(
        &mut self,
        minion: MinionId,
        kind: ActionKind,
        torpor_target: Option<MinionId>,
        card: Option<PlayCtx>,
    ) {
        self.state.minions[minion.index()].locked = true;
        let seat = self.state.active;
        let own_target = torpor_target.map(|t| self.state.minions[t.index()].owner == seat);
        let (base_stealth, bleed_target, directed_seat) = match kind {
            ActionKind::Bleed => (0, Some(self.state.prey(seat)), Some(self.state.prey(seat))),
            ActionKind::Hunt | ActionKind::LeaveTorpor | ActionKind::Employ => (1, None, None),
            ActionKind::Rescue | ActionKind::Diablerie => {
                let t = torpor_target.expect("torpor target");
                if own_target == Some(true) {
                    (1, None, None)
                } else {
                    (0, None, Some(self.state.minions[t.index()].owner))
                }
            }
        };
        let order = self.action_order(seat, directed_seat);
        self.action = Some(ActionCtx {
            seat,
            acting: minion,
            kind,
            card,
            torpor_target,
            bleed_target,
            base_stealth,
            stealth_mods: 0,
            intercept: 0,
            bleed_mods: 0,
            limited_bleed_used: false,
            state: ActionState::AsAnnounced,
            window: OrderedWindow::new(order.clone()),
            order,
            declined: Vec::new(),
            woken: Vec::new(),
            defer_reaction_replacement: false,
            deferred_draws: Vec::new(),
            failed: false,
        });
        self.flow = Flow::Action;
    }

    /// Sequencing order during an action: the acting Methuselah, then the
    /// target of a directed action, then the rest clockwise.
    fn action_order(&self, seat: SeatId, directed: Option<SeatId>) -> Vec<SeatId> {
        let mut order = vec![seat];
        if let Some(t) = directed {
            if t != seat {
                order.push(t);
            }
        }
        for s in self.state.clockwise_from(seat) {
            if !order.contains(&s) {
                order.push(s);
            }
        }
        order
    }

    /// Seats entitled to attempt a block (rulebook "Who May Attempt to
    /// Block"): the target of a directed action; prey and predator of an
    /// undirected one.
    fn eligible_blockers(&self, a: &ActionCtx) -> Vec<SeatId> {
        let directed = match a.kind {
            ActionKind::Bleed => a.bleed_target,
            ActionKind::Rescue | ActionKind::Diablerie => {
                let t = a.torpor_target.expect("torpor target");
                let owner = self.state.minions[t.index()].owner;
                (owner != a.seat).then_some(owner)
            }
            _ => None,
        };
        match directed {
            Some(t) => vec![t],
            None => {
                let mut v = vec![self.state.prey(a.seat), self.state.predator(a.seat)];
                v.dedup();
                v.retain(|s| *s != a.seat);
                v
            }
        }
    }

    /// Minions of `seat` that may attempt to block: ready and unlocked, or
    /// woken for this action.
    fn block_candidates(&self, a: &ActionCtx, seat: SeatId) -> Vec<MinionId> {
        self.state
            .minions_of(seat)
            .filter(|m| {
                let v = &self.state.minions[m.index()];
                !v.in_torpor && (!v.locked || a.woken.contains(m))
            })
            .collect()
    }

    /// Intercept reachability (DESIGN §4.3 elision, owner-approved
    /// 2026-09-02, P1 form): an attempt whose intercept can never reach the
    /// action's stealth — nothing in the controller's hand this minion could
    /// play adds enough — is pruned rather than offered. The closure-time
    /// comparison in `action_advance` is the actual semantics.
    fn intercept_reachable(&self, a: &ActionCtx, seat: SeatId, m: MinionId) -> u8 {
        let st = &self.state;
        let mut total = 0u8;
        for &card in &st.seats[seat.index()].hand {
            let Some(def) = cards::lookup(card) else {
                continue;
            };
            for (mi, mode) in def.modes.iter().enumerate() {
                let tags = LIBRARY[card as usize].modes[mi].tags;
                if !cards::has_disciplines(st, m, card, tags) {
                    continue;
                }
                if mode.requires.contains(&cards::Requirement::DirectedAtYou)
                    && a.directed_at() != Some(seat)
                {
                    continue;
                }
                for e in mode.effects {
                    if let Effect::AddIntercept(n) = e {
                        total = total.max(*n);
                    }
                }
            }
        }
        total
    }

    /// Windows open for `seat` in the action's current state.
    fn action_windows(a: &ActionCtx, seat: SeatId) -> Vec<Window> {
        let acting = seat == a.seat;
        match a.state {
            ActionState::AsAnnounced => {
                if acting {
                    vec![Window::ActionAsAnnounced]
                } else {
                    vec![]
                }
            }
            ActionState::Open | ActionState::Attempt(_) => {
                if acting {
                    vec![Window::ActionDuring, Window::ActionStealthIntercept]
                } else {
                    vec![
                        Window::ReactionDuringAction,
                        Window::ActionStealthIntercept,
                        Window::Wake,
                    ]
                }
            }
            ActionState::Declined => {
                if acting {
                    vec![Window::ActionDuring]
                } else {
                    vec![
                        Window::ReactionDuringAction,
                        Window::ActionBlocksDeclined,
                        Window::Wake,
                    ]
                }
            }
        }
    }

    pub(crate) fn action_request(&self) -> Option<DecisionRequest> {
        let a = self.action.as_ref().expect("action context");
        let seat = a.window.current()?;
        let mut options = vec![Choice::Pass];
        if a.state == ActionState::Open
            && self.eligible_blockers(a).contains(&seat)
            && !a.declined.contains(&seat)
        {
            for m in self.block_candidates(a, seat) {
                if self.intercept_reachable(a, seat, m) >= a.stealth() {
                    options.push(Choice::DeclareBlock { minion: m });
                }
            }
        }
        options.extend(self.play_options(seat, &Self::action_windows(a, seat), None));
        let window = match a.state {
            ActionState::AsAnnounced => WindowLabel::ActionAsAnnounced,
            ActionState::Open | ActionState::Attempt(_) => WindowLabel::ActionBlock,
            ActionState::Declined => WindowLabel::ActionBlocksDeclined,
        };
        Some(DecisionRequest {
            seat,
            window,
            options,
        })
    }

    pub(crate) fn action_apply(&mut self, choice: &Choice) {
        let eligible = {
            let a = self.action.as_ref().expect("action context");
            self.eligible_blockers(a)
        };
        let a = self.action.as_mut().expect("action context");
        let seat = a.window.current().expect("impulse holder");
        match choice {
            Choice::Pass => {
                if a.state == ActionState::Open
                    && eligible.contains(&seat)
                    && !a.declined.contains(&seat)
                {
                    // rules-qa 1.3: passing with no attempt pending is a final
                    // decline (rulebook A.3).
                    a.declined.push(seat);
                }
                a.window.pass();
                self.flow = Flow::Action;
            }
            Choice::DeclareBlock { minion } => {
                a.state = ActionState::Attempt(*minion);
                a.intercept = 0;
                a.window.play();
                self.flow = Flow::Action;
            }
            Choice::Play {
                card,
                mode,
                actor,
                target,
            } => {
                a.window.play();
                self.flow = Flow::Action;
                self.announce_play(seat, *card, *mode, *actor, *target, Flow::Action);
            }
            other => unreachable!("illegal action choice {other:?}"),
        }
    }

    /// The action window closed on consecutive passes: state transition
    /// (rulebook A.4, B.4, C.3; spec §2.2 for the as-announced window).
    pub(crate) fn action_advance(&mut self) {
        let a = self.action.as_mut().expect("action context");
        debug_assert!(a.window.is_closed());
        match a.state {
            ActionState::AsAnnounced => {
                a.state = ActionState::Open;
                a.fresh_window();
                self.flow = Flow::Action;
            }
            ActionState::Open => {
                a.state = ActionState::Declined;
                a.fresh_window();
                self.flow = Flow::Action;
            }
            ActionState::Attempt(blocker) => {
                if a.intercept >= a.stealth() {
                    self.blocked(blocker);
                } else {
                    // The attempt fails: back to A, where the same Methuselah
                    // may attempt again (rulebook B.4, "if one attempt to
                    // block fails, another can be made").
                    a.state = ActionState::Open;
                    a.intercept = 0;
                    a.fresh_window();
                    self.flow = Flow::Action;
                }
            }
            ActionState::Declined => {
                if a.failed {
                    self.end_action();
                } else {
                    self.succeed();
                }
            }
        }
    }

    /// Effects of a modifier / reaction / master resolved during the action.
    pub(crate) fn apply_action_effect(&mut self, effect: Effect, play: &PlayCtx) {
        let mut a = self.action.take().expect("action context");
        match effect {
            Effect::AddStealth(n) => a.stealth_mods += n,
            Effect::AddIntercept(n) => a.intercept += n,
            Effect::BleedMod { n, limited } => {
                a.bleed_mods += n;
                if limited {
                    a.limited_bleed_used = true;
                }
            }
            Effect::Wake => a.woken.push(play.actor.expect("waking minion")),
            Effect::LockActor => {
                self.state.minions[play.actor.expect("actor").index()].locked = true;
            }
            Effect::RedirectBleed => {
                let cards::Target::Seat(t) = play.target else {
                    unreachable!("redirect without a seat target");
                };
                // A change of target reopens block attempts for the new
                // target (rulebook A.3 / C.2; rules-qa 1.3): back to A with
                // passes voided.
                a.bleed_target = Some(t);
                a.declined.clear();
                a.state = ActionState::Open;
                a.intercept = 0;
                a.order = self.action_order_for(a.seat, Some(t));
                a.fresh_window();
            }
            Effect::BurnActingMinion => {
                let m = a.acting;
                self.state.minions[m.index()].in_play = false;
                self.state.minions[m.index()].blood = 0;
            }
            Effect::ActionFails => a.failed = true,
            Effect::ActionLayer(modifier) => {
                self.state.layers.push(cards::Layer {
                    scope: Scope::Action,
                    modifier,
                    source: crate::state::CardId(u32::MAX),
                });
            }
            Effect::DeferReactionReplacement => a.defer_reaction_replacement = true,
            other => unreachable!("not an action effect: {other:?}"),
        }
        self.action = Some(a);
    }

    fn action_order_for(&self, seat: SeatId, directed: Option<SeatId>) -> Vec<SeatId> {
        self.action_order(seat, directed)
    }

    /// Block resolution: the blocker locks; combat, except a blocked
    /// leave-torpor (no combat; the blocker may diablerise — P1 later).
    fn blocked(&mut self, blocker: MinionId) {
        let (acting, kind) = {
            let a = self.action.as_ref().expect("action context");
            (a.acting, a.kind)
        };
        self.state.minions[blocker.index()].locked = true;
        if kind == ActionKind::Bleed {
            // NRA triggers on resolution, blocked included (rules-qa 1.4).
            self.state.minions[acting.index()].bled_this_turn = true;
        }
        match kind {
            ActionKind::LeaveTorpor => unreachable!("leave torpor blocked without intercept cards"),
            _ => self.combat(acting, blocker),
        }
        self.end_action();
    }

    fn succeed(&mut self) {
        let a = self.action.take().expect("action context");
        let seat = a.seat;
        match a.kind {
            ActionKind::Bleed => {
                let prey = a.bleed_target.expect("bleed target");
                self.state.minions[a.acting.index()].bled_this_turn = true;
                self.state.seats[prey.index()].pool -= a.bleed_amount() as i32;
                // Successful bleed: take the Edge.
                for s in &mut self.state.seats {
                    s.has_edge = false;
                }
                self.state.seats[seat.index()].has_edge = true;
                if self.state.seats[prey.index()].pool <= 0 {
                    self.state.seats[prey.index()].pool = 0;
                    self.oust(prey);
                }
            }
            ActionKind::Hunt => {
                let m = &mut self.state.minions[a.acting.index()];
                m.blood = (m.blood + 1).min(m.capacity());
            }
            ActionKind::LeaveTorpor => {
                // Cost is paid at resolution only (a blocked action pays
                // nothing); blood spent during the action can leave it
                // unpayable, in which case the action is unsuccessful.
                let m = &mut self.state.minions[a.acting.index()];
                if m.blood >= 2 {
                    m.blood -= 2;
                    m.in_torpor = false;
                }
            }
            ActionKind::Rescue => {
                // The 2-blood cost is paid at resolution; blood spent during
                // the action (an as-announced modifier) can leave it
                // unpayable, in which case the action is unsuccessful.
                let acting = self.state.minions[a.acting.index()].blood;
                let t = self.state.minions[a.torpor_target.expect("target").index()].blood;
                self.action = Some(a);
                if acting + t >= 2 {
                    self.flow = Flow::RescueCost;
                } else {
                    self.end_action();
                }
                return;
            }
            ActionKind::Diablerie => {
                let victim = a.torpor_target.expect("diablerie target");
                // Rulebook "Diablerie" steps 1–3 as a single unit: blood to
                // the diablerist (excess drains), victim burned. Equipment
                // and the advanced rule 4 (Discipline for an elder victim)
                // need cards — later.
                let taken = self.state.minions[victim.index()].blood;
                self.state.minions[victim.index()].blood = 0;
                self.state.minions[victim.index()].in_play = false;
                let d = &mut self.state.minions[a.acting.index()];
                d.blood = (d.blood + taken).min(d.capacity());
                // The blood hunt referendum follows immediately; rotation
                // starts at the acting Methuselah, clockwise (DESIGN §4.2 —
                // a modeling call, owner-approved 2026-09-02).
                let order = self.state.clockwise_from(seat);
                self.action = Some(a);
                self.finish_action();
                self.flow = Flow::BloodHunt(BloodHuntCtx {
                    diablerist: self.action.take().expect("ctx").acting,
                    window: PollingWindow::new(order),
                    spent: Vec::new(),
                    edge_spent: false,
                    for_votes: 0,
                    against_votes: 0,
                });
                self.action = None;
                return;
            }
            ActionKind::Employ => {
                let play = a.card.expect("action card");
                let def = cards::lookup(play.card).expect("definition");
                // Cost is paid at resolution (spec §1.7.1).
                let cost = cards::blood_cost(&self.state, play.card);
                self.state.minions[a.acting.index()].blood -= cost;
                for e in def.modes[play.mode as usize].effects {
                    if let Effect::EmployRetainer { life } = e {
                        self.state.cards.push(CardInPlay {
                            card: play.card,
                            owner: seat,
                            host: Some(a.acting),
                            in_play: true,
                            locked: false,
                            counters: *life,
                        });
                    }
                }
            }
        }
        self.action = Some(a);
        self.end_action();
    }

    /// Bookkeeping at the end of an action, successful or not: action-scoped
    /// layers pop, deferred replacements are drawn, the action card is
    /// discarded if it did not enter play, NRA identity is recorded.
    fn finish_action(&mut self) {
        let a = self.action.as_ref().expect("action context");
        let seat = a.seat;
        let acting = a.acting;
        let deferred = a.deferred_draws.clone();
        if let Some(play) = a.card {
            let in_play = self
                .state
                .cards
                .iter()
                .any(|c| c.card == play.card && c.host == Some(acting) && c.in_play);
            if !in_play {
                self.state.seats[seat.index()].ash.push(play.card);
            }
            self.state.minions[acting.index()].acted.push(play.card);
        }
        self.state.layers.retain(|l| l.scope != Scope::Action);
        for s in deferred {
            self.draw_to_hand_size(s);
        }
    }

    fn end_action(&mut self) {
        self.finish_action();
        self.action = None;
        // Back to the acting seat's minion phase, unless the action ended the
        // game (an oust that left one Methuselah).
        if self.state.living() > 1 {
            self.flow = Flow::MinionPhase;
        } else {
            self.flow = Flow::GameOver;
        }
    }

    pub(crate) fn rescue_request(&self) -> DecisionRequest {
        let a = self.action.as_ref().expect("action context");
        let st = &self.state;
        let acting = st.minions[a.acting.index()].blood;
        let t = st.minions[a.torpor_target.expect("target").index()].blood;
        let options = (0..=2u8)
            .filter(|&k| acting >= k && t >= 2 - k)
            .map(|k| Choice::PayRescue { from_acting: k })
            .collect();
        DecisionRequest {
            seat: st.active,
            window: WindowLabel::RescueCost,
            options,
        }
    }

    pub(crate) fn rescue_apply(&mut self, from_acting: u8) {
        let a = self.action.as_ref().expect("action context");
        let (acting, target) = (a.acting, a.torpor_target.expect("target"));
        self.state.minions[acting.index()].blood -= from_acting;
        self.state.minions[target.index()].blood -= 2 - from_acting;
        // Rescued: to the ready region, lock state untouched.
        self.state.minions[target.index()].in_torpor = false;
        self.end_action();
    }

    /// Rulebook "Minion Phase": a ready unlocked vampire with no blood must
    /// hunt, and no non-mandatory action may be taken while any such hunt is
    /// pending — so the phase cannot even end.
    pub(crate) fn minion_phase_options(&self) -> Vec<Choice> {
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
        // Card actions from hand (employ …), declared by a ready unlocked
        // minion; NRA by card name per minion per turn (rules-qa 1.4).
        options.extend(self.play_options(
            s,
            &[
                Window::ActionDeclare,
                Window::ActionDeclareEquip,
                Window::ActionDeclareRecruit,
                Window::ActionDeclareEmploy,
            ],
            None,
        ));
        options
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
}
