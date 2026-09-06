//! Actions: announcement, the as-announced window, the block loop (rulebook
//! states A/B/C, rules-qa 1.3), redirects, block resolution (combat, or the
//! blocker's diablerie on a leave-torpor), the after-combat window with the
//! Form of Mist continuation, and resolution. The action context lives on
//! the engine so that card plays resolved from the as-played stack can
//! modify it.

use crate::cards::{self, Effect, PlayCtx, Scope, Window};
use crate::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use crate::game::{Engine, Flow};
use crate::ids::{MinionId, SeatId};
use crate::library::LIBRARY;
use crate::referendum::RefKind;
use crate::state::{ActionId, CardId, CardInPlay};
use crate::window::OrderedWindow;

/// Rulebook "Resolve Any Block Attempts": A (no attempt), B (an attempt is
/// ongoing), C (blocks declined by all); plus the as-announced window that
/// precedes them (spec §2.2).
/// How an action ended (owner, 2026-09-04): blocked; failed (Archon
/// Investigation: "not successful"); successful; or *fizzled* — successful
/// (not blocked, so after-resolution effects such as Freak Drive apply) but
/// without effect, because its cost could not be paid at resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Blocked,
    Failed,
    Successful,
    Fizzled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionState {
    AsAnnounced,
    Open,
    Attempt(MinionId),
    Declined,
    /// After the block's combat(s) (rules-qa 2.1): unlock reactions of the
    /// blocker, the acting minion's continuation, then the action ends
    /// blocked or re-enters the block loop.
    AfterCombat,
    /// After resolution of a successful action (for a political action,
    /// after its referendum: spec §3.7.5.1): Voter Captivation, Freak
    /// Drive.
    AfterResolution,
}

#[derive(Clone, Debug)]
pub struct ActionCtx {
    pub seat: SeatId,
    pub acting: MinionId,
    pub kind: ActionKind,
    /// The action card, for card actions (employ …).
    pub card: Option<PlayCtx>,
    /// The card in play granting the action, for granted actions (Anarch
    /// Revolt's referendum).
    pub source: Option<CardId>,
    /// Ability index on `source`.
    pub ability: Option<u8>,
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
    /// Set when the action ends.
    pub outcome: Option<Outcome>,
    /// The minion whose block resolved (for "who has blocked").
    pub blocker: Option<MinionId>,
    /// Form of Mist [PRO]: after combat, burn `blood` to continue with
    /// +`stealth` as if unblocked. Lost if the combat continues.
    pub continuation: Option<(u8, u8)>,
    /// The continuation was taken in the after-combat window.
    pub continued: bool,
    /// Card names played per minion this action (`Limit::PerAction`).
    pub played: Vec<(MinionId, &'static str)>,
    /// The political action's referendum passed, by this margin.
    pub referendum_passed: bool,
    pub margin: u32,
    /// Bleed reductions (Party Out Of Bounds [obf], Protected District).
    pub bleed_reduce: u8,
    /// The action card was returned to hand (Delaying Tactics): not
    /// discarded at the end of the action.
    pub card_returned: bool,
    /// The action card entered play at resolution (not discarded after).
    pub card_entered_play: bool,
}

impl ActionCtx {
    pub fn stealth(&self) -> u8 {
        self.base_stealth + self.stealth_mods
    }

    pub fn bleed_amount(&self) -> u8 {
        (1 + self.bleed_mods).saturating_sub(self.bleed_reduce)
    }

    /// Effective intercept of minion `m`'s attempt: the accumulated
    /// intercept folded through the action's intercept layers (Perfect
    /// Paragon [PRE]: younger vampires and allies get -1).
    pub fn intercept_of(&self, st: &crate::state::GameState, m: MinionId) -> u8 {
        (self.intercept as i32 + Self::intercept_mods(st, m) + self.host_intercept(st, m)).max(0)
            as u8
    }

    /// Intercept from cards on `m` keyed to the action (Guardian Angel: +1
    /// during bleed actions directed at its controller).
    pub fn host_intercept(&self, st: &crate::state::GameState, m: MinionId) -> i32 {
        let owner = st.minions[m.index()].owner;
        let bleed_at_owner = self.kind == ActionKind::Bleed && self.bleed_target == Some(owner);
        st.layers
            .iter()
            .filter(|l| {
                l.scope == Scope::InPlay
                    && st.layer_active(l)
                    && st.cards[l.source.index()].host == Some(m)
            })
            .filter_map(|l| match l.modifier {
                cards::Modifier::HostInterceptOnBleedAtYou(n) if bleed_at_owner => Some(n as i32),
                _ => None,
            })
            .sum()
    }

    /// Sum of the action's intercept layers applying to `m`.
    pub fn intercept_mods(st: &crate::state::GameState, m: MinionId) -> i32 {
        let cap = st.minions[m.index()].capacity();
        st.layers
            .iter()
            .filter(|l| st.layer_active(l))
            .filter_map(|l| match l.modifier {
                cards::Modifier::InterceptMod { younger_than, n } if cap < younger_than => {
                    Some(n as i32)
                }
                _ => None,
            })
            .sum()
    }

    /// The effective intercept of the current attempt, if any.
    pub fn attempt_intercept(&self, st: &crate::state::GameState) -> u8 {
        match self.state {
            ActionState::Attempt(m) => self.intercept_of(st, m),
            _ => 0,
        }
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
        source: Option<CardId>,
    ) {
        self.state.minions[minion.index()].locked = true;
        let seat = self.state.active;
        let own_target = torpor_target.map(|t| self.state.minions[t.index()].owner == seat);
        // The stealth an action card or a granted action prints ("+1
        // stealth action"); the rulebook's +1 applies to political action
        // cards and the undirected rulebook actions (rules-qa 6.2).
        let printed_stealth = |effects: &[Effect]| -> u8 {
            effects
                .iter()
                .map(|e| match e {
                    Effect::AddStealth(n) => *n,
                    _ => 0,
                })
                .sum()
        };
        let (base_stealth, bleed_target, directed_seat) = match kind {
            ActionKind::Bleed => (0, Some(self.state.prey(seat)), Some(self.state.prey(seat))),
            ActionKind::Hunt | ActionKind::LeaveTorpor | ActionKind::Employ => (1, None, None),
            ActionKind::Political => match card {
                Some(_) => (1, None, None),
                None => (0, None, None),
            },
            ActionKind::Card => {
                let play = card.expect("action card");
                let def = cards::lookup(play.card).expect("definition");
                (
                    printed_stealth(def.modes[play.mode as usize].effects),
                    None,
                    None,
                )
            }
            ActionKind::BurnCard => {
                // Directed at the card's controller; one's own card makes
                // it undirected. No default stealth for an action granted
                // by a card in play — only what its text prints (rules-qa
                // 6.2).
                let owner = self.state.cards[source.expect("source").index()].owner;
                (0, None, (owner != seat).then_some(owner))
            }
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
            source,
            ability: None,
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
            outcome: None,
            blocker: None,
            continuation: None,
            continued: false,
            played: Vec::new(),
            referendum_passed: false,
            margin: 0,
            bleed_reduce: 0,
            card_returned: false,
            card_entered_play: false,
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
            ActionKind::BurnCard => {
                let owner = self.state.cards[a.source.expect("source").index()].owner;
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
                !v.in_torpor && !v.expelled && (!v.locked || a.woken.contains(m))
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
        // The current attempt's intercept counts when re-checking in B.
        let mut total = match a.state {
            ActionState::Attempt(cur) if cur == m => a.intercept,
            _ => 0,
        };
        total = (total as i32 + a.host_intercept(st, m)).max(0) as u8;
        for &card in &st.seats[seat.index()].hand {
            let Some(def) = cards::lookup(card) else {
                continue;
            };
            let mut per_card = 0u8;
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
                // One play per card: the best mode of each card adds.
                let best = mode
                    .effects
                    .iter()
                    .filter_map(|e| match e {
                        Effect::AddIntercept(n) => Some(*n),
                        _ => None,
                    })
                    .max()
                    .unwrap_or(0);
                per_card = per_card.max(best);
            }
            total = total.saturating_add(per_card);
        }
        (total as i32 + ActionCtx::intercept_mods(st, m)).max(0) as u8
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
            ActionState::AfterCombat => vec![Window::ActionAfterCombat],
            ActionState::AfterResolution => vec![Window::ActionAfterResolution],
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
        if a.state == ActionState::AfterCombat && seat == a.seat {
            if let Some((blood, _)) = a.continuation {
                let m = &self.state.minions[a.acting.index()];
                if m.ready() && m.blood >= blood {
                    options.push(Choice::ContinueAction);
                }
            }
        }
        options.extend(self.play_options(seat, &Self::action_windows(a, seat), None));
        let window = match a.state {
            ActionState::AsAnnounced => WindowLabel::ActionAsAnnounced,
            ActionState::Open | ActionState::Attempt(_) => WindowLabel::ActionBlock,
            ActionState::Declined => WindowLabel::ActionBlocksDeclined,
            ActionState::AfterCombat => WindowLabel::ActionAfterCombat,
            ActionState::AfterResolution => WindowLabel::ActionAfterResolution,
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
            Choice::ContinueAction => {
                // Form of Mist [PRO]: the rider's blood is a burn, not a card
                // cost (rulings); +1 stealth "even if not yet needed" is a
                // plain bonus here, no play to gate.
                let (blood, stealth) = a.continuation.take().expect("continuation");
                a.stealth_mods += stealth;
                a.continued = true;
                a.window.play();
                self.state.minions[a.acting.index()].blood -= blood;
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
                if a.intercept_of(&self.state, blocker) >= a.stealth() {
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
                    a.outcome = Some(Outcome::Failed);
                    self.end_action();
                } else {
                    self.succeed();
                }
            }
            ActionState::AfterCombat => {
                if a.continued {
                    // Continue as if unblocked (rules-qa 3.4): back to A with
                    // the action's context kept — modifiers, reactions and
                    // the declines stand; only the attempt state resets.
                    a.continued = false;
                    a.state = ActionState::Open;
                    a.intercept = 0;
                    a.fresh_window();
                    self.flow = Flow::Action;
                } else {
                    a.outcome = Some(Outcome::Blocked);
                    self.end_action();
                }
            }
            ActionState::AfterResolution => self.end_action(),
        }
    }

    /// The action resolved successfully: open the after-resolution window
    /// over the action's order (elided when nothing can be played).
    pub(crate) fn after_resolution(&mut self) {
        let st = &self.state;
        let a = self.action.as_mut().expect("action context");
        a.state = ActionState::AfterResolution;
        // The resolution may have ousted seats of the order.
        a.order.retain(|s| !st.seats[s.index()].ousted);
        a.fresh_window();
        self.flow = Flow::Action;
    }

    /// The combat ended: open the after-combat window over the action's
    /// order (rules-qa 2.1: once, after all combats of the block).
    pub(crate) fn after_combat(&mut self) {
        let a = self.action.as_mut().expect("action context");
        a.state = ActionState::AfterCombat;
        a.fresh_window();
        self.flow = Flow::Action;
    }

    /// A blocked leave-torpor (rulebook "Leave Torpor"): no combat; the
    /// blocking vampire may diablerise the acting vampire, else the action
    /// simply fails (no cost paid).
    pub(crate) fn torpor_block_request(&self) -> DecisionRequest {
        let a = self.action.as_ref().expect("action context");
        let blocker = a.blocker.expect("blocker");
        DecisionRequest {
            seat: self.state.minions[blocker.index()].owner,
            window: WindowLabel::TorporBlock,
            options: vec![Choice::Pass, Choice::Diablerise],
        }
    }

    pub(crate) fn torpor_block_apply(&mut self, choice: &Choice) {
        let a = self.action.as_mut().expect("action context");
        a.outcome = Some(Outcome::Blocked);
        let (blocker, acting) = (a.blocker.expect("blocker"), a.acting);
        match choice {
            Choice::Pass => self.end_action(),
            Choice::Diablerise => {
                self.diablerise(blocker, acting);
                self.end_action_into_blood_hunt(blocker);
            }
            other => unreachable!("illegal torpor-block choice {other:?}"),
        }
    }

    /// Rulebook "Diablerie" steps 1–3 as a single unit: blood to the
    /// diablerist (excess drains), victim burned. Equipment and the
    /// advanced rule 4 (a Discipline for an elder victim) need cards —
    /// later.
    fn diablerise(&mut self, diablerist: MinionId, victim: MinionId) {
        let taken = self.state.minions[victim.index()].blood;
        self.state.minions[victim.index()].blood = 0;
        self.state.minions[victim.index()].in_play = false;
        let d = &mut self.state.minions[diablerist.index()];
        d.blood = (d.blood + taken).min(d.capacity());
    }

    /// The action ends and the blood hunt referendum follows immediately;
    /// rotation starts at the diablerist's controller, clockwise (DESIGN
    /// §4.2 — a modeling call, owner-approved 2026-09-02).
    fn end_action_into_blood_hunt(&mut self, diablerist: MinionId) {
        // The blood hunt is part of the action (spec §3.7.8.1): the action
        // ends after it, through its after-resolution window (rules-qa 5.8).
        let seat = self.state.minions[diablerist.index()].owner;
        self.start_referendum(RefKind::BloodHunt { diablerist }, seat, None);
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
            Effect::ReduceBleed(n) => a.bleed_reduce += n,
            Effect::YoungerIntercept(n) => {
                let younger_than =
                    self.state.minions[play.actor.expect("actor").index()].capacity();
                self.state.layers.push(cards::Layer {
                    scope: Scope::Action,
                    modifier: cards::Modifier::InterceptMod { younger_than, n },
                    source: crate::state::CardId(u32::MAX),
                });
            }
            Effect::GainBloodPerMargin => {
                // Voter Captivation: 1 blood per vote of the margin; X of
                // them to the pool instead (a passed referendum with no
                // polling step passes by 0: PIB 20150105).
                let m = play.actor.expect("actor");
                let to_pool = play.x.min(a.margin as u8);
                let v = &mut self.state.minions[m.index()];
                v.blood = (v.blood + (a.margin as u8 - to_pool)).min(v.capacity());
                self.state.seats[play.seat.index()].pool += to_pool as i32;
            }
            Effect::Wake => a.woken.push(play.actor.expect("waking minion")),
            Effect::LockActor => {
                self.state.minions[play.actor.expect("actor").index()].locked = true;
            }
            Effect::UnlockActor => {
                self.state.minions[play.actor.expect("actor").index()].locked = false;
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
    /// leave-torpor (no combat; the blocker may diablerise).
    fn blocked(&mut self, blocker: MinionId) {
        let (acting, kind) = {
            let a = self.action.as_mut().expect("action context");
            a.blocker = Some(blocker);
            (a.acting, a.kind)
        };
        self.state.minions[blocker.index()].locked = true;
        // NRA triggers on resolution, blocked included (rules-qa 1.4).
        match kind {
            ActionKind::Bleed => self.state.minions[acting.index()].bled_this_turn = true,
            ActionKind::Political => self.state.minions[acting.index()].political_this_turn = true,
            _ => {}
        }
        match kind {
            ActionKind::LeaveTorpor => self.flow = Flow::TorporBlock,
            _ => self.start_combat(acting, blocker),
        }
    }

    fn succeed(&mut self) {
        let mut a = self.action.take().expect("action context");
        let seat = a.seat;
        a.outcome = Some(Outcome::Successful);
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
                // unpayable, in which case the action fizzles (rules-qa
                // 3.6): successful, no effect.
                let m = &mut self.state.minions[a.acting.index()];
                if m.blood >= 2 {
                    m.blood -= 2;
                    m.in_torpor = false;
                } else {
                    a.outcome = Some(Outcome::Fizzled);
                }
            }
            ActionKind::Rescue => {
                // The 2-blood cost is paid at resolution; blood spent during
                // the action (an as-announced modifier) can leave it
                // unpayable, in which case the action fizzles (rules-qa
                // 3.6): successful, no effect.
                let acting = self.state.minions[a.acting.index()].blood;
                let t = self.state.minions[a.torpor_target.expect("target").index()].blood;
                if acting + t < 2 {
                    a.outcome = Some(Outcome::Fizzled);
                }
                let payable = acting + t >= 2;
                self.action = Some(a);
                if payable {
                    self.flow = Flow::RescueCost;
                } else {
                    self.end_action();
                }
                return;
            }
            ActionKind::Diablerie => {
                let victim = a.torpor_target.expect("diablerie target");
                let diablerist = a.acting;
                self.action = Some(a);
                self.diablerise(diablerist, victim);
                self.end_action_into_blood_hunt(diablerist);
                return;
            }
            ActionKind::Employ => {
                let play = a.card.expect("action card");
                let def = cards::lookup(play.card).expect("definition");
                // Cost is paid at resolution (spec §1.7.1); unpayable by
                // then, the action fizzles (rules-qa 3.6).
                let cost = cards::blood_cost(&self.state, play.card, play.x);
                if self.state.minions[a.acting.index()].blood < cost {
                    a.outcome = Some(Outcome::Fizzled);
                    self.action = Some(a);
                    self.after_resolution();
                    return;
                }
                self.state.minions[a.acting.index()].blood -= cost;
                for e in def.modes[play.mode as usize].effects {
                    if let Effect::EmployRetainer { life } = e {
                        let id = self.enter_play(CardInPlay {
                            card: play.card,
                            owner: seat,
                            host: Some(a.acting),
                            in_play: true,
                            contested: false,
                            locked: false,
                            counters: *life,
                        });
                        self.enter_library_contest(id);
                    }
                }
            }
            ActionKind::Card => {
                // The card's effects resolve now; its cost (X read now,
                // spec §1.7.1) unpayable by resolution fizzles (rules-qa
                // 3.6).
                let play = a.card.expect("action card");
                let def = cards::lookup(play.card).expect("definition");
                let mode = &def.modes[play.mode as usize];
                let x = cards::terms_x(&self.state, seat, play.card, mode).unwrap_or(play.x);
                let cost = cards::blood_cost(&self.state, play.card, x);
                if self.state.minions[a.acting.index()].blood < cost {
                    a.outcome = Some(Outcome::Fizzled);
                    self.action = Some(a);
                    self.after_resolution();
                    return;
                }
                self.state.minions[a.acting.index()].blood -= cost;
                let ctx = PlayCtx { x, ..play };
                let mut in_play = None;
                // Declaration-time effects (the kind, the printed stealth)
                // were read at announcement.
                for e in mode
                    .effects
                    .iter()
                    .filter(|e| !matches!(e, Effect::DeclareAction(_) | Effect::AddStealth(_)))
                {
                    self.apply_card_effect(*e, &ctx, &mut in_play);
                }
                a.card_entered_play = in_play.is_some();
                if let Some(id) = in_play {
                    self.enter_library_contest(id);
                }
            }
            ActionKind::BurnCard => {
                let src = a.source.expect("source");
                self.burn_card_in_play(src);
            }
            ActionKind::Political => {
                // Rulebook "The Referendum": the cost is paid and the
                // referendum called — only if the acting minion is still
                // ready (spec §3.7.5.1); NRA on resolution (rules-qa 1.4).
                // The referendum and cost come from the political action
                // card, or from the card in play granting the action (no
                // card-play cost, spec §1.5).
                let (mode, cost) = match (a.card, a.source) {
                    (Some(play), _) => {
                        let def = cards::lookup(play.card).expect("definition");
                        let cost = cards::blood_cost(&self.state, play.card, play.x);
                        (&def.modes[play.mode as usize], cost)
                    }
                    (None, Some(src)) => {
                        let c = &self.state.cards[src.index()];
                        let def = cards::lookup(c.card).expect("definition");
                        let ability = a.ability.expect("granted action's ability");
                        (&def.abilities[ability as usize], 0)
                    }
                    (None, None) => unreachable!("political action without a source"),
                };
                let m = &mut self.state.minions[a.acting.index()];
                m.political_this_turn = true;
                if m.blood < cost {
                    // Unpayable by resolution: fizzles (rules-qa 3.6), no
                    // referendum.
                    a.outcome = Some(Outcome::Fizzled);
                    self.action = Some(a);
                    self.after_resolution();
                    return;
                }
                m.blood -= cost;
                let referendum = mode
                    .effects
                    .iter()
                    .find_map(|e| match e {
                        Effect::CallReferendum(r) => Some(*r),
                        _ => None,
                    })
                    .expect("political action card calls a referendum");
                let ready = self.state.minions[a.acting.index()].ready();
                self.action = Some(a);
                if ready {
                    self.start_referendum(RefKind::Political, seat, Some(referendum));
                } else {
                    self.after_resolution();
                }
                return;
            }
        }
        self.action = Some(a);
        self.after_resolution();
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
            let in_play = a.card_entered_play
                || self
                    .state
                    .cards
                    .iter()
                    .any(|c| c.card == play.card && c.host == Some(acting) && c.in_play);
            if !in_play && !a.card_returned {
                self.state.seats[seat.index()].ash.push(play.card);
            }
            self.state.minions[acting.index()]
                .acted
                .push(ActionId::Card(play.card));
        }
        if let Some(src) = a.source {
            self.state.minions[acting.index()]
                .acted
                .push(ActionId::InPlay(src));
        }
        self.state.layers.retain(|l| l.scope != Scope::Action);
        for s in deferred {
            self.draw_to_hand_size(s);
        }
    }

    pub(crate) fn end_action(&mut self) {
        self.finish_action();
        self.last_outcome = self.action.as_ref().and_then(|a| a.outcome);
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
        self.after_resolution();
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
                Window::ActionDeclarePolitical,
                Window::ActionDeclareEquip,
                Window::ActionDeclareRecruit,
                Window::ActionDeclareEmploy,
            ],
            None,
        ));
        // Actions granted by cards in play (Anarch Revolt's referendum, "burn
        // this card as a Ⓓ action").
        options.extend(self.ability_options(s, Window::ActionDeclare));
        options.extend(self.ability_options(s, Window::ActionDeclarePolitical));
        options
    }
}
