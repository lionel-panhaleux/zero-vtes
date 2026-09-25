//! Actions: announcement, the as-announced window, the block loop (rulebook
//! states A/B/C, rules-qa 1.3), redirects, block resolution (combat, or the
//! blocker's diablerie on a leave-torpor), the after-combat window with the
//! Form of Mist continuation, and resolution. The action context lives on
//! the engine so that card plays resolved from the as-played stack can
//! modify it.

use crate::cards::{self, Effect, Hook, Limit, PlayCtx, Scope, Stat, Target, Window};
use crate::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use crate::game::{Engine, Flow};
use crate::ids::{MinionId, SeatId};
use crate::library::LIBRARY;
use crate::referendum::RefKind;
use crate::state::{ActionId, CardId, CardInPlay};
use crate::window::OrderedWindow;

/// An equipment card the engine can put in play (it has a definition).
fn is_equipment(card: u16) -> bool {
    LIBRARY[card as usize]
        .types
        .contains(&crate::library::CardType::Equipment)
        && cards::lookup(card).is_some()
}

/// The play context of an equipment found in the library and equipped on
/// `acting` (its only mode).
fn found_play(seat: SeatId, acting: MinionId, card: u16) -> PlayCtx {
    PlayCtx {
        seat,
        card,
        mode: 0,
        actor: Some(acting),
        target: Target::Minion(acting),
        x: 0,
        burn: None,
    }
}

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
    /// The vampire that diablerised in this action: the acting minion, or
    /// the blocker of a leave-torpor action (rules-qa 16.2).
    pub diablerist: Option<MinionId>,
    /// The Methuselah being bled (changes on a redirect).
    pub bleed_target: Option<SeatId>,
    pub base_stealth: u8,
    pub stealth_mods: u8,
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
    /// Mirror Walk [THA]: if blocked, the blocker locks and the action
    /// ends before block resolution.
    pub end_on_block: bool,
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
    /// An equip action from another minion: that minion, and the equipment
    /// announced (rulebook "Equip").
    pub giver: Option<MinionId>,
    pub taken: Vec<CardId>,
    /// In-play abilities used this action, by copy ("during an action":
    /// once per action, rules-qa 9.6).
    pub abilities_used: Vec<(cards::AbilitySource, u8)>,
    /// Intercept gained by a minion, from any source, for the rest of the
    /// action: a failed attempt keeps it for a new one (rules-qa 15.2).
    pub intercept_given: Vec<(MinionId, i8)>,
}

impl ActionCtx {
    /// The action's stealth, the acting minion's cards in play included
    /// (Unlicensed Taxicab, Codex of the Edenic Groundskeepers), never
    /// below 0.
    pub fn stealth(&self, st: &crate::state::GameState) -> u8 {
        (self.base_stealth as i32
            + self.stealth_mods as i32
            + st.bearer_mod(self.acting, Stat::Stealth, self.kind)
            + st.printed_mod(self.acting, Stat::Stealth, self.printed_ctx(st, None)))
        .max(0) as u8
    }

    /// The bleed amount, the acting minion's cards in play included
    /// (Treasured Samadji), after reductions.
    pub fn bleed_amount(&self, st: &crate::state::GameState) -> u8 {
        // The granting card's own bleed condition, live (Codex of the Edenic
        // Groundskeepers: "+3 bleed if the target Methuselah controls no
        // ready unlocked minions").
        let granted: i32 = match (self.source, self.ability, self.bleed_target) {
            (Some(src), Some(ab), Some(t)) => {
                cards::lookup(st.cards[src.index()].card).map_or(0, |def| {
                    def.abilities[ab as usize]
                        .effects
                        .iter()
                        .map(|e| match e {
                            Effect::BleedIfTargetHasNoReadyUnlocked(n)
                                if st.ready_unlocked(t).is_empty() =>
                            {
                                *n as i32
                            }
                            _ => 0,
                        })
                        .sum()
                })
            }
            _ => 0,
        };
        let n = (1
            + self.bleed_mods as i32
            + st.bearer_mod(self.acting, Stat::Bleed, self.kind)
            + st.printed_mod(self.acting, Stat::Bleed, self.printed_ctx(st, None))
            + granted)
            .max(0) as u8;
        n.saturating_sub(self.bleed_reduce)
    }

    /// Effective intercept of minion `m`'s attempt: the accumulated
    /// intercept folded through the action's intercept layers (Perfect
    /// Paragon [PRE]: younger vampires and allies get -1).
    pub fn intercept_of(&self, st: &crate::state::GameState, m: MinionId) -> u8 {
        (Self::intercept_mods(st, m) + self.host_intercept(st, m) + self.given_intercept(m)).max(0)
            as u8
    }

    /// Intercept given to `m` for the rest of the action.
    pub fn given_intercept(&self, m: MinionId) -> i32 {
        self.intercept_given
            .iter()
            .filter(|(g, _)| *g == m)
            .map(|(_, n)| *n as i32)
            .sum()
    }

    /// Intercept from cards in play keyed to the action: on `m` (Guardian
    /// Angel: +1 during bleed actions directed at its controller; Sport
    /// Bike), and on the acting minion against `m` (Stolen Police Cruiser:
    /// younger vampires get -1 intercept against its bearer).
    pub fn host_intercept(&self, st: &crate::state::GameState, m: MinionId) -> i32 {
        let owner = st.minions[m.index()].owner;
        let bleed_at_owner = self.kind == ActionKind::Bleed && self.bleed_target == Some(owner);
        let younger = st.minions[m.index()].capacity() < st.minions[self.acting.index()].capacity();
        let on = |host: MinionId| {
            st.layers.iter().filter(move |l| {
                l.scope == Scope::InPlay
                    && st.layer_active(l)
                    && st.cards[l.source.index()].host == Some(host)
            })
        };
        let own: i32 = on(m)
            .filter_map(|l| match l.modifier {
                cards::Modifier::HostInterceptOnBleedAtYou(n) if bleed_at_owner => Some(n as i32),
                _ => None,
            })
            .sum();
        let against: i32 = on(self.acting)
            .filter_map(|l| match l.modifier {
                cards::Modifier::YoungerInterceptAgainstBearer { n, who }
                    if younger && who.is(st, self.acting) =>
                {
                    Some(n as i32)
                }
                _ => None,
            })
            .sum();
        let printed = st.printed_mod(m, Stat::Intercept, self.printed_ctx(st, Some(self.acting)));
        own + st.bearer_mod(m, Stat::Intercept, self.kind) + against + printed
    }

    /// The Methuselah a directed action is directed at, if any.
    pub fn directed_seat(&self, st: &crate::state::GameState) -> Option<SeatId> {
        match self.kind {
            ActionKind::Bleed => self.bleed_target,
            ActionKind::Rescue | ActionKind::Diablerie => {
                let t = self.torpor_target.expect("torpor target");
                let owner = st.minions[t.index()].owner;
                (owner != self.seat).then_some(owner)
            }
            ActionKind::BurnCard => {
                let controller = st.card_controller(self.source.expect("source"));
                (controller != self.seat).then_some(controller)
            }
            _ => None,
        }
    }

    /// What a printed modifier reads of this action (`GameState::printed_mod`).
    pub fn printed_ctx(
        &self,
        st: &crate::state::GameState,
        opponent: Option<MinionId>,
    ) -> crate::state::PrintedCtx {
        crate::state::PrintedCtx {
            kind: Some(self.kind),
            directed: Some(self.directed_seat(st).is_some()),
            opponent,
        }
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

/// The stealth an action card, a granted action or an equipment prints ("+1
/// stealth action"); the rulebook's +1 applies to political action cards and
/// the undirected rulebook actions (rules-qa 6.2).
fn printed_stealth(effects: &[Effect]) -> u8 {
    effects
        .iter()
        .map(|e| match e {
            Effect::AddStealth(n) => *n,
            _ => 0,
        })
        .sum()
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
        let (base_stealth, bleed_target, directed_seat) = match kind {
            ActionKind::Bleed => (0, Some(self.state.prey(seat)), Some(self.state.prey(seat))),
            ActionKind::Hunt
            | ActionKind::LeaveTorpor
            | ActionKind::Employ
            | ActionKind::EquipFrom => (1, None, None),
            // Granted by a card in play: its printed stealth only (rules-qa
            // 6.2), added by `use_ability`.
            ActionKind::EquipFound => (0, None, None),
            ActionKind::Equip => {
                // +1 stealth, and what the equipment prints for the action
                // to equip with it (Unlicensed Taxicab).
                let play = card.expect("equipment card");
                let def = cards::lookup(play.card).expect("definition");
                (
                    1 + printed_stealth(def.modes[play.mode as usize].effects),
                    None,
                    None,
                )
            }
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
                let controller = self.state.card_controller(source.expect("source"));
                (0, None, (controller != seat).then_some(controller))
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
            bleed_mods: 0,
            limited_bleed_used: false,
            state: ActionState::AsAnnounced,
            window: OrderedWindow::new(order.clone()),
            order,
            declined: Vec::new(),
            woken: Vec::new(),
            defer_reaction_replacement: false,
            end_on_block: false,
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
            giver: None,
            taken: Vec::new(),
            diablerist: None,
            abilities_used: Vec::new(),
            intercept_given: Vec::new(),
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
        match self.directed_target(a) {
            Some(t) => vec![t],
            None => {
                let mut v = vec![self.state.prey(a.seat), self.state.predator(a.seat)];
                v.dedup();
                v.retain(|s| *s != a.seat);
                v
            }
        }
    }

    /// The Methuselah a directed action is directed at, if any.
    fn directed_target(&self, a: &ActionCtx) -> Option<SeatId> {
        a.directed_seat(&self.state)
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
        // Intercept gained earlier in the action counts (rules-qa 15.2).
        let mut total = (a.host_intercept(st, m) + a.given_intercept(m)).max(0) as u8;
        total = total.saturating_add(self.printed_intercept_reach(seat, m));
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
        // Unused intercept abilities of the cards on `m` (Bowl of
        // Convergence [AUS]).
        for (ci, c) in st.cards.iter().enumerate() {
            if !c.present() || c.host != Some(m) {
                continue;
            }
            let Some(def) = cards::lookup(c.card) else {
                continue;
            };
            for ab in def
                .abilities
                .iter()
                .filter(|ab| ab.windows.contains(&Window::ActionStealthIntercept))
            {
                if ab.limit == Some(Limit::PerAction)
                    && a.abilities_used
                        .contains(&cards::AbilitySource::Card(CardId(ci as u32)).use_key(0))
                {
                    continue;
                }
                if !ab.requires.iter().all(|&r| cards::holds_for_host(r, st, m)) {
                    continue;
                }
                for e in ab.effects {
                    if let Effect::AddIntercept(n) = e {
                        total = total.saturating_add(*n);
                    }
                }
            }
        }
        (total as i32 + ActionCtx::intercept_mods(st, m)).max(0) as u8
    }

    /// An upper bound on the intercept `m` can get from abilities printed on
    /// `seat`'s vampires: a use per blood burned (Oskar Anasov) or card
    /// discarded (Abraham DuSable, on his own attempt), once per action for
    /// "during an action" (rules-qa 15.2).
    fn printed_intercept_reach(&self, seat: SeatId, m: MinionId) -> u8 {
        let st = &self.state;
        let mut total = 0u8;
        for v in st.minions_of(seat) {
            let Some(def) = cards::vampire(st.minions[v.index()].card) else {
                continue;
            };
            for (ai, ab) in def
                .abilities
                .iter()
                .enumerate()
                .filter(|(_, ab)| ab.windows.contains(&Window::ActionStealthIntercept))
            {
                let per_use: u8 = ab
                    .effects
                    .iter()
                    .map(|e| match e {
                        Effect::AttemptingIntercept(n) if *n > 0 => *n as u8,
                        Effect::AddIntercept(n) if v == m => *n,
                        _ => 0,
                    })
                    .sum();
                if per_use == 0 {
                    continue;
                }
                let uses = match ab.terms {
                    Some(cards::Terms::Discard { n, of }) => {
                        (st.hand_picks(seat, of).len() / n.max(1) as usize).min(u8::MAX as usize)
                            as u8
                    }
                    _ if ab
                        .effects
                        .iter()
                        .any(|e| matches!(e, Effect::HostBurnBlood(_))) =>
                    {
                        st.minions[v.index()].blood
                    }
                    _ => 1,
                };
                let uses = match ab.limit {
                    Some(Limit::PerAction) => {
                        let used = self.action.as_ref().is_some_and(|a| {
                            a.abilities_used
                                .contains(&cards::AbilitySource::Vampire(v).use_key(ai as u8))
                        });
                        if used {
                            0
                        } else {
                            uses.min(1)
                        }
                    }
                    _ => uses,
                };
                total = total.saturating_add(per_use.saturating_mul(uses));
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
                if self.intercept_reachable(a, seat, m) >= a.stealth(&self.state) {
                    options.push(Choice::DeclareBlock { minion: m });
                }
            }
        }
        if a.state == ActionState::AfterCombat && seat == a.seat {
            if let Some((blood, _)) = a.continuation {
                let m = &self.state.minions[a.acting.index()];
                // A directed action whose target Methuselah left the game
                // mid-combat (DESIGN §10.8) has nothing to continue against.
                let target_gone = self
                    .directed_target(a)
                    .is_some_and(|t| self.state.seats[t.index()].ousted);
                if m.ready() && m.blood >= blood && !target_gone {
                    options.push(Choice::ContinueAction);
                }
            }
        }
        let windows = Self::action_windows(a, seat);
        options.extend(self.play_options(seat, &windows, None));
        // Abilities of cards in play timed like the cards of these windows
        // (Bowl of Convergence [AUS] on a block attempt).
        for &w in &windows {
            options.extend(self.ability_options(seat, w));
        }
        options.extend(self.ability_options(seat, Window::AnyImpulse));
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
            Choice::UseAbility {
                source,
                ability,
                actor,
                target,
            } => {
                // An untimed ability (The Barrens) is an effect: the impulse
                // returns to the acting Methuselah.
                a.window.play();
                self.flow = Flow::Action;
                self.use_ability(*source, *ability, *actor, *target, Flow::Action);
            }
            other => unreachable!("illegal action choice {other:?}"),
        }
    }

    /// The action window closed on consecutive passes: state transition
    /// (rulebook A.4, B.4, C.3; spec §2.2 for the as-announced window).
    pub(crate) fn action_advance(&mut self) {
        if self.action.as_ref().expect("action context").state == ActionState::AsAnnounced
            && self.announced_at_target_riders()
        {
            return;
        }
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
                if a.intercept_of(&self.state, blocker) >= a.stealth(&self.state) {
                    self.blocked(blocker);
                } else {
                    // The attempt fails: back to A, where the same Methuselah
                    // may attempt again (rulebook B.4, "if one attempt to
                    // block fails, another can be made").
                    a.state = ActionState::Open;
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
        // Burned from torpor: the cards on it go, no "ready minion" rider.
        self.burn_minion(victim);
        let d = &mut self.state.minions[diablerist.index()];
        d.blood = (d.blood + taken).min(d.capacity());
    }

    /// The action ends and the blood hunt referendum follows immediately;
    /// rotation starts at the diablerist's controller, clockwise (DESIGN
    /// §4.2 — a modeling call, owner-approved 2026-09-02).
    fn end_action_into_blood_hunt(&mut self, diablerist: MinionId) {
        // The blood hunt is part of the action (spec §3.7.8.1): the action
        // ends after it, through its after-resolution window (rules-qa 5.8).
        self.action.as_mut().expect("action context").diablerist = Some(diablerist);
        let seat = self.state.minions[diablerist.index()].owner;
        self.start_referendum(RefKind::BloodHunt { diablerist }, seat, None);
    }

    /// Effects of a modifier / reaction / master resolved during the action.
    pub(crate) fn apply_action_effect(&mut self, effect: Effect, play: &PlayCtx) {
        let mut a = self.action.take().expect("action context");
        let mut burned = None;
        match effect {
            Effect::AddStealth(n) => a.stealth_mods += n,
            // For the rest of the action (rules-qa 15.2).
            Effect::AddIntercept(n) => {
                // The minion given the intercept: the card's actor, else the
                // minion attempting to block.
                let m = play
                    .actor
                    .or(match a.state {
                        ActionState::Attempt(m) => Some(m),
                        _ => None,
                    })
                    .expect("intercepting minion");
                a.intercept_given.push((m, n as i8));
            }
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
                a.order = self.action_order_for(a.seat, Some(t));
                a.fresh_window();
            }
            Effect::BurnActingMinion => burned = Some(a.acting),
            Effect::ActionFails => a.failed = true,
            Effect::ActionLayer(modifier) => {
                self.state.layers.push(cards::Layer {
                    scope: Scope::Action,
                    modifier,
                    source: crate::state::CardId(u32::MAX),
                });
            }
            Effect::DeferReactionReplacement => a.defer_reaction_replacement = true,
            Effect::EndActionIfBlocked => a.end_on_block = true,
            other => unreachable!("not an action effect: {other:?}"),
        }
        self.action = Some(a);
        if let Some(m) = burned {
            // Its riders (Tension in the Ranks) may oust the acting
            // Methuselah: the play's resolution passes the turn.
            self.burn_minion(m);
        }
    }

    fn action_order_for(&self, seat: SeatId, directed: Option<SeatId>) -> Vec<SeatId> {
        self.action_order(seat, directed)
    }

    /// Riders printed on the vampire an action targets, fired as the
    /// as-announced window closes (Evan Klein's coin flip). Returns true
    /// when the action ended.
    fn announced_at_target_riders(&mut self) -> bool {
        let a = self.action.as_ref().expect("action context");
        let (kind, acting) = (a.kind, a.acting);
        let Some(target) = a.torpor_target else {
            return false;
        };
        let v = &self.state.minions[target.index()];
        let owner = v.owner;
        let Some(def) = cards::vampire(v.card).filter(|_| v.present()) else {
            return false;
        };
        let effects: Vec<Effect> = def
            .riders
            .iter()
            .filter(|r| {
                r.hook == Hook::ActionAnnouncedAtTarget
                    && r.requires.iter().all(|&req| {
                        cards::holds(
                            req,
                            &self.state,
                            owner,
                            Some(target),
                            None,
                            self.action.as_ref(),
                            None,
                            None,
                            false,
                        )
                    })
            })
            .flat_map(|r| r.effects.iter().copied())
            .collect();
        for e in effects {
            match e {
                // Tails: the action fails at once — no block, no cost — the
                // acting minion locked and the action counted for NRA
                // (rules-qa 16.6).
                Effect::CoinFlipActionFails if self.rng.below(2) == 0 => {
                    {
                        let a = self.action.as_mut().expect("action context");
                        a.outcome = Some(Outcome::Failed);
                    }
                    match kind {
                        ActionKind::Bleed => {
                            self.state.minions[acting.index()].bled_this_turn = true;
                        }
                        ActionKind::Political => {
                            self.state.minions[acting.index()].political_this_turn = true;
                        }
                        _ => {}
                    }
                    self.end_action();
                    return true;
                }
                Effect::CoinFlipActionFails => {}
                other => unreachable!("not an announce rider effect: {other:?}"),
            }
        }
        false
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
        // "If blocked" riders of the acting minion's cards (Unlicensed
        // Taxicab) apply before any other card or effect (ANK 20220116).
        self.host_blocked_riders(acting);
        // Mirror Walk [THA]: the action ends before block resolution — no
        // combat, no diablerie opportunity on a blocked leave-torpor (PIB
        // 20121216), nothing more played on it (RTR 20180719).
        if self.action.as_ref().is_some_and(|a| a.end_on_block) {
            self.action.as_mut().expect("action context").outcome = Some(Outcome::Blocked);
            self.end_action();
            return;
        }
        match kind {
            ActionKind::LeaveTorpor => self.flow = Flow::TorporBlock,
            _ => self.start_combat(acting, blocker),
        }
    }

    /// Riders of the cards on the acting minion fired as its action is
    /// blocked, their conditions read on the blocker.
    fn host_blocked_riders(&mut self, acting: MinionId) {
        let seat = self.state.minions[acting.index()].owner;
        let mut burns = Vec::new();
        for (ci, c) in self.state.cards.iter().enumerate() {
            if !c.present() || c.host != Some(acting) {
                continue;
            }
            let Some(def) = cards::lookup(c.card) else {
                continue;
            };
            for r in def.riders.iter().filter(|r| r.hook == Hook::HostBlocked) {
                let holds = r.requires.iter().all(|&req| {
                    cards::holds(
                        req,
                        &self.state,
                        seat,
                        Some(acting),
                        None,
                        self.action.as_ref(),
                        None,
                        None,
                        false,
                    )
                });
                if !holds {
                    continue;
                }
                for e in r.effects {
                    match e {
                        Effect::BurnSelf => burns.push(CardId(ci as u32)),
                        other => unreachable!("not a host-blocked rider effect: {other:?}"),
                    }
                }
            }
        }
        for id in burns {
            self.burn_card_in_play(id);
        }
        // Printed on the acting vampire (Marialena).
        if let Some(def) = cards::vampire(self.state.minions[acting.index()].card) {
            for r in def.riders.iter().filter(|r| r.hook == Hook::HostBlocked) {
                if !r
                    .requires
                    .iter()
                    .all(|&req| cards::holds_for_host(req, &self.state, acting))
                {
                    continue;
                }
                for e in r.effects {
                    match *e {
                        Effect::HostBurnBlood(n) => {
                            let v = &mut self.state.minions[acting.index()];
                            v.blood -= n.min(v.blood);
                        }
                        other => unreachable!("not a host-blocked rider effect: {other:?}"),
                    }
                }
            }
        }
    }

    /// The effects of the ability granting the current action, if any.
    fn granted_effects(&self, a: &ActionCtx) -> &'static [Effect] {
        match (a.source, a.ability) {
            (Some(src), Some(ab)) => cards::lookup(self.state.cards[src.index()].card)
                .map_or(&[], |def| def.abilities[ab as usize].effects),
            _ => &[],
        }
    }

    fn succeed(&mut self) {
        let mut a = self.action.take().expect("action context");
        let seat = a.seat;
        a.outcome = Some(Outcome::Successful);
        match a.kind {
            ActionKind::Bleed => {
                let prey = a.bleed_target.expect("bleed target");
                debug_assert!(
                    !self.state.seats[prey.index()].ousted,
                    "bleed on an ousted seat"
                );
                self.state.minions[a.acting.index()].bled_this_turn = true;
                // A bleed granted by a card on the acting minion (Codex of
                // the Edenic Groundskeepers): it fizzles if the card is no
                // longer on the acting minion (RTR 19960221) or its blood
                // cost is unpayable by now (rules-qa 3.6).
                if let Some(src) = a.source {
                    let effects = self.granted_effects(&a);
                    let cost: u8 = effects
                        .iter()
                        .map(|e| match e {
                            Effect::PayBlood(n) => *n,
                            _ => 0,
                        })
                        .sum();
                    let card = &self.state.cards[src.index()];
                    let attached = card.present() && card.host == Some(a.acting);
                    if !attached || self.state.minions[a.acting.index()].blood < cost {
                        a.outcome = Some(Outcome::Fizzled);
                        self.action = Some(a);
                        self.after_resolution();
                        return;
                    }
                    self.state.minions[a.acting.index()].blood -= cost;
                }
                self.state.seats[prey.index()].pool -= a.bleed_amount(&self.state) as i32;
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
                            controller: seat,
                            host: Some(a.acting),
                            in_play: true,
                            contested: false,
                            locked: false,
                            counters: *life,
                            stored: Vec::new(),
                        });
                        self.enter_library_contest(id);
                    }
                }
            }
            ActionKind::Equip => {
                // Rulebook "Equip": the card goes on the acting minion, its
                // cost paid now (spec §1.7.1). Unpayable by then, or no
                // longer holdable ("a minion can have only one vehicle"),
                // the action fizzles (rules-qa 3.6) and the card is
                // discarded; paid down to 0 pool, the Methuselah is ousted.
                let play = a.card.expect("equipment card");
                let acting = a.acting;
                let pool = cards::pool_cost(&self.state, seat, play.card, Target::Minion(acting));
                let blood = cards::blood_cost(&self.state, play.card, play.x);
                if self.state.seats[seat.index()].pool < pool
                    || self.state.minions[acting.index()].blood < blood
                    || !self.state.can_hold(acting, play.card, &[])
                {
                    a.outcome = Some(Outcome::Fizzled);
                } else {
                    self.state.seats[seat.index()].pool -= pool;
                    self.state.minions[acting.index()].blood -= blood;
                    self.put_equipment(seat, acting, play);
                    a.card_entered_play = true;
                    if pool > 0 && self.state.seats[seat.index()].pool == 0 {
                        self.action = Some(a);
                        self.oust_and_pass(seat);
                        return;
                    }
                }
            }
            ActionKind::EquipFound => {
                // Vast Wealth: fizzles if the card left the acting minion
                // (as the Codex's bleed). Working down from the top, the
                // first equipment is equipped as normal — or burned without
                // cost if it cannot be placed or its blood paid (TOM
                // 19950407, rules-qa 9.3), its pool paid even if that ousts
                // (RTR 19941109); the library is shuffled afterward.
                let src = a.source.expect("source");
                let acting = a.acting;
                let c = &self.state.cards[src.index()];
                if !(c.present() && c.host == Some(acting)) {
                    a.outcome = Some(Outcome::Fizzled);
                    self.action = Some(a);
                    self.after_resolution();
                    return;
                }
                let mut ousted = false;
                let library = &self.state.seats[seat.index()].library;
                if let Some(k) = library.iter().rposition(|&c| is_equipment(c)) {
                    let card = self.state.seats[seat.index()].library.remove(k);
                    let blood = cards::blood_cost(&self.state, card, 0);
                    if self.equip_requirements_met(acting, card)
                        && self.state.minions[acting.index()].blood >= blood
                    {
                        let pool =
                            cards::pool_cost(&self.state, seat, card, Target::Minion(acting));
                        let s = &mut self.state.seats[seat.index()];
                        s.pool = (s.pool - pool).max(0);
                        ousted = pool > 0 && s.pool == 0;
                        self.state.minions[acting.index()].blood -= blood;
                        self.put_equipment(seat, acting, found_play(seat, acting, card));
                    } else {
                        self.state.seats[seat.index()].ash.push(card);
                    }
                }
                self.rng
                    .shuffle(&mut self.state.seats[seat.index()].library);
                if ousted {
                    self.action = Some(a);
                    self.oust_and_pass(seat);
                    return;
                }
            }
            ActionKind::EquipFrom => {
                // The equipment still on the minion it was announced from
                // moves to the acting minion; one it could no longer hold
                // stays where it is.
                let acting = a.acting;
                for &id in &a.taken {
                    let c = &self.state.cards[id.index()];
                    if c.present() && c.host == a.giver && self.state.can_hold(acting, c.card, &[])
                    {
                        self.state.cards[id.index()].host = Some(acting);
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
                if mode.effects.contains(&Effect::SearchEquip) {
                    // Magic of the Smith: the equipment is chosen now that
                    // the action succeeded (spec §1.14.1, §3.4).
                    let acting = a.acting;
                    self.action = Some(a);
                    self.flow = Flow::LibrarySearch { minion: acting };
                    return;
                }
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
                // The granted action's pool cost is paid at resolution
                // ("as a Ⓓ action that costs 1 pool"); unpayable by then
                // it fizzles (rules-qa 3.6); paid down to 0, the acting
                // Methuselah is ousted and the turn passes.
                let src = a.source.expect("source");
                let cost: u8 = a
                    .ability
                    .and_then(|ab| {
                        let def = cards::lookup(self.state.cards[src.index()].card)?;
                        Some(
                            def.abilities[ab as usize]
                                .effects
                                .iter()
                                .map(|e| match e {
                                    Effect::PayPool(n) => *n,
                                    _ => 0,
                                })
                                .sum(),
                        )
                    })
                    .unwrap_or(0);
                if self.state.seats[seat.index()].pool < cost as i32 {
                    a.outcome = Some(Outcome::Fizzled);
                } else {
                    self.state.seats[seat.index()].pool -= cost as i32;
                    let host = self.state.cards[src.index()].host;
                    let effects = self.granted_effects(&a);
                    self.burn_card_in_play(src);
                    for e in effects {
                        if let (Effect::LockHostSkipNextUnlock(who), Some(h)) = (e, host) {
                            // Stolen Police Cruiser: "this Anarch is locked
                            // and does not unlock as normal during their
                            // next unlock phase".
                            if self.state.minions[h.index()].present() && who.is(&self.state, h) {
                                let v = &mut self.state.minions[h.index()];
                                v.locked = true;
                                v.skip_unlock = true;
                            }
                        }
                    }
                    if self.state.seats[seat.index()].pool == 0 {
                        self.action = Some(a);
                        self.oust_and_pass(seat);
                        return;
                    }
                }
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
        self.release_action_holds();
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

    /// Put an equipment card from outside play on `acting`, its cost paid:
    /// its mode's own effects apply, and a unique enters contest.
    fn put_equipment(&mut self, seat: SeatId, acting: MinionId, play: PlayCtx) {
        let def = cards::lookup(play.card).expect("definition");
        let ctx = PlayCtx {
            target: Target::Minion(acting),
            ..play
        };
        let mut in_play = Some(self.enter_play(CardInPlay {
            card: play.card,
            owner: seat,
            controller: seat,
            host: Some(acting),
            in_play: true,
            contested: false,
            locked: false,
            counters: 0,
            stored: Vec::new(),
        }));
        for e in def.modes[play.mode as usize]
            .effects
            .iter()
            .filter(|e| !matches!(e, Effect::DeclareAction(_) | Effect::AddStealth(_)))
        {
            self.apply_card_effect(*e, &ctx, &mut in_play);
        }
        if let Some(id) = in_play {
            self.enter_library_contest(id);
        }
    }

    /// Could minion `m` equip with library card `card` as a card play would
    /// — clan and discipline requirements, "a minion can have only one", no
    /// unique copy already in play under its controller (rules-qa 7.1)?
    /// Cards without a definition cannot be put in play by the engine yet.
    fn equip_requirements_met(&self, m: MinionId, card: u16) -> bool {
        let st = &self.state;
        let lib = &LIBRARY[card as usize];
        is_equipment(card)
            && !st.controls_unique_copy(st.minions[m.index()].owner, card)
            && (lib.clans.is_empty() || lib.clans.contains(&st.minions[m.index()].card().clan))
            && lib
                .modes
                .first()
                .is_some_and(|mode| cards::has_disciplines(st, m, card, mode.tags))
            && st.can_hold(m, card, &[])
    }

    /// The search of a successful Magic of the Smith: every equipment in
    /// the library the vampire can equip and pay for (a pool cost down to 0
    /// included; no unpayable blood cost, rules-qa 9.3), by card, or none.
    pub(crate) fn search_request(&self, minion: MinionId) -> DecisionRequest {
        let st = &self.state;
        let seat = st.active;
        let mut found: Vec<u16> = st.seats[seat.index()]
            .library
            .iter()
            .copied()
            .filter(|&c| {
                self.equip_requirements_met(minion, c)
                    && st.seats[seat.index()].pool
                        >= cards::pool_cost(st, seat, c, Target::Minion(minion))
                    && st.minions[minion.index()].blood >= cards::blood_cost(st, c, 0)
            })
            .collect();
        // By card, not by position: the order of the library is hidden.
        found.sort_unstable();
        found.dedup();
        let mut options = vec![Choice::Pass];
        options.extend(found.into_iter().map(Choice::FindCard));
        DecisionRequest {
            seat,
            window: WindowLabel::LibrarySearch,
            options,
        }
    }

    pub(crate) fn search_apply(&mut self, minion: MinionId, choice: &Choice) {
        let seat = self.state.active;
        let mut ousted = false;
        match *choice {
            Choice::FindCard(card) => {
                let library = &mut self.state.seats[seat.index()].library;
                let k = library
                    .iter()
                    .rposition(|&c| c == card)
                    .expect("found card in the library");
                library.remove(k);
                let pool = cards::pool_cost(&self.state, seat, card, Target::Minion(minion));
                let blood = cards::blood_cost(&self.state, card, 0);
                self.state.seats[seat.index()].pool -= pool;
                self.state.minions[minion.index()].blood -= blood;
                self.put_equipment(seat, minion, found_play(seat, minion, card));
                ousted = pool > 0 && self.state.seats[seat.index()].pool == 0;
            }
            Choice::Pass => {}
            other => unreachable!("illegal search choice {other:?}"),
        }
        self.rng
            .shuffle(&mut self.state.seats[seat.index()].library);
        if ousted {
            self.oust_and_pass(seat);
        } else {
            self.after_resolution();
        }
    }

    /// The equipment an equip action takes from `giver`, announced one at
    /// a time with the action (rulebook "Equip": several at once); `Pass`
    /// once at least one is taken.
    pub(crate) fn action_terms_request(
        &self,
        minion: MinionId,
        giver: MinionId,
        picked: &[CardId],
    ) -> DecisionRequest {
        let st = &self.state;
        let with: Vec<u16> = picked.iter().map(|p| st.cards[p.index()].card).collect();
        let mut options = Vec::new();
        if !picked.is_empty() {
            options.push(Choice::Pass);
        }
        for id in st.equipment_on(giver) {
            if !picked.contains(&id) && st.can_hold(minion, st.cards[id.index()].card, &with) {
                options.push(Choice::TakeEquipment { card: id.0 });
            }
        }
        DecisionRequest {
            seat: st.active,
            window: WindowLabel::ActionTerms,
            options,
        }
    }

    pub(crate) fn action_terms_apply(
        &mut self,
        minion: MinionId,
        giver: MinionId,
        mut picked: Vec<CardId>,
        choice: &Choice,
    ) {
        match choice {
            Choice::TakeEquipment { card } => {
                picked.push(CardId(*card));
                self.flow = Flow::ActionTerms {
                    minion,
                    giver,
                    picked,
                };
            }
            Choice::Pass => {
                self.declare_action(minion, ActionKind::EquipFrom, None, None, None);
                // Each equipment's own bonus for the action to equip with it
                // applies (Unlicensed Taxicab, LSJ 20060222).
                let stealth: u8 = picked
                    .iter()
                    .filter_map(|p| cards::lookup(self.state.cards[p.index()].card))
                    .flat_map(|def| def.modes.iter().take(1))
                    .map(|mode| printed_stealth(mode.effects))
                    .sum();
                let a = self.action.as_mut().expect("action");
                a.base_stealth += stealth;
                a.giver = Some(giver);
                a.taken = picked;
            }
            other => unreachable!("illegal action terms choice {other:?}"),
        }
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
            // The forced hunt — after any untimed ability the Methuselah
            // wants first (The Coven's blood spares the hunt; owner,
            // 2026-09-06).
            let mut options: Vec<Choice> = starving
                .into_iter()
                .map(|minion| Choice::TakeAction {
                    minion,
                    action: ActionKind::Hunt,
                    target: None,
                })
                .collect();
            options.extend(self.ability_options(s, Window::AnyImpulse));
            return options;
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
            // Equip from another minion the Methuselah controls, ready or
            // in torpor (rules-qa 9.2), with something this one could hold.
            for g in st.minions_of(s).filter(|&g| g != id) {
                if st
                    .equipment_on(g)
                    .iter()
                    .any(|e| st.can_hold(id, st.cards[e.index()].card, &[]))
                {
                    options.push(Choice::TakeAction {
                        minion: id,
                        action: ActionKind::EquipFrom,
                        target: Some(g),
                    });
                }
            }
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
        options.extend(self.ability_options(s, Window::AnyImpulse));
        options
    }
}
