//! Card plays from hand: option enumeration from the definitions (no card
//! code decides legality), the master phase and the discard phase as offer
//! windows, numeric terms at announcement (rules-qa 3.5), the as-played
//! window with a cancel stack (spec §1.8), resolution with costs folded
//! through layers, replacement draws, and in-play abilities.

use crate::cards::{
    self, Actor, Effect, Hook, Layer, Limit, PlayCtx, Requirement, Rider, Scope, Target,
    TargetKind, TargetSpec, Terms, Window,
};
use crate::combat::is_combat_effect;
use crate::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use crate::game::{Engine, Flow, Pending};
use crate::ids::{MinionId, SeatId};
use crate::library::{CardType, Label, LIBRARY};
use crate::referendum::is_vote_effect;
use crate::state::{ActionId, CardId, CardInPlay, OutOfTurn, HAND_SIZE};
use crate::window::OrderedWindow;

/// The once-per-phase key of an in-play ability: the copy, or the copy and
/// mode under `Limit::PerPhaseEach`.
fn phase_key(card: CardId, ability: u8, limit: Option<Limit>) -> (CardId, u8) {
    match limit {
        Some(Limit::PerPhaseEach) => (card, ability),
        _ => (card, 0),
    }
}

/// A card play between announcement and resolution: its as-played window is
/// open (other Methuselahs may cancel it), possibly beneath a nested cancel.
#[derive(Clone, Debug)]
pub(crate) struct PendingPlay {
    pub ctx: PlayCtx,
    pub window: OrderedWindow,
    pub canceled: bool,
}

impl Engine {
    /// Draw up to hand size from the library (rulebook "Drawing Cards"; an
    /// empty library means no replacement).
    pub(crate) fn draw_to_hand_size(&mut self, s: SeatId) {
        let seat = &mut self.state.seats[s.index()];
        while seat.hand.len() + (seat.delayed_draws as usize) < HAND_SIZE + seat.hand_bonus as usize
        {
            match seat.library.pop() {
                Some(c) => seat.hand.push(c),
                None => break,
            }
        }
    }

    /// Master phase actions for the active seat's master phase (rulebook
    /// "Master Phase"; out-of-turn accounting per rules-qa 3.3).
    pub(crate) fn start_master_phase(&mut self) {
        let s = self.state.active;
        let seat = &mut self.state.seats[s.index()];
        seat.mpa = 1;
        seat.trifle_used = false;
        if let Some(OutOfTurn { trifle }) = seat.out_of_turn.take() {
            seat.mpa -= 1;
            if trifle {
                seat.mpa += 1;
                seat.trifle_used = true;
            }
        }
        self.state.phase_uses.clear();
        self.flow = Flow::MasterPhase;
    }

    /// Minions of `seat` that may play a mode as its actor.
    fn actor_candidates(&self, seat: SeatId, mode: &cards::ModeDef) -> Vec<Option<MinionId>> {
        let st = &self.state;
        match mode.actor {
            Actor::Methuselah => vec![None],
            // Rulebook "Combat": the only minion cards playable during
            // combat are combat cards — modifiers and reactions (wakes in a
            // strike card's as-played window included) get no window there
            // (spec §4.1.3).
            Actor::ActingMinion => match &self.action {
                Some(a) if a.seat == seat && self.combat.is_none() => vec![Some(a.acting)],
                _ => vec![],
            },
            Actor::ReactingMinion => self.reacting_candidates(seat, mode),
            // The acting minion for its controller, a reacting minion for
            // any other Methuselah (the card's type follows the player).
            Actor::ActingOrReactingMinion => match &self.action {
                Some(a) if a.seat == seat && self.combat.is_none() => vec![Some(a.acting)],
                Some(_) => self.reacting_candidates(seat, mode),
                None => vec![],
            },
            Actor::ActingCandidate => {
                if self.action.is_some() || st.active != seat {
                    vec![]
                } else {
                    st.ready_unlocked(seat).into_iter().map(Some).collect()
                }
            }
            Actor::Host | Actor::Controller => vec![],
            // The player's minion in the combat; ready, except in the
            // end-of-round window (Taste of Vitae's "not usable by a vampire
            // being burned or going to torpor" is the card's own clause).
            Actor::Combatant => match &self.combat {
                Some(c) => c
                    .side_of(seat)
                    .map(|i| c.sides[i].minion)
                    .filter(|m| {
                        st.minions[m.index()].ready() || c.step == crate::combat::Step::EndOfRound
                    })
                    .map(|m| vec![Some(m)])
                    .unwrap_or_default(),
                None => vec![],
            },
        }
    }

    /// Ready unlocked minions of `seat` (other than the acting Methuselah's)
    /// that may play a reaction: woken or locked-only per the mode.
    fn reacting_candidates(&self, seat: SeatId, mode: &cards::ModeDef) -> Vec<Option<MinionId>> {
        let st = &self.state;
        match &self.action {
            Some(a) if a.seat != seat && self.combat.is_none() => {
                let needs_locked = mode.requires.contains(&Requirement::LockedActor);
                st.minions_of(seat)
                    .filter(|m| {
                        let v = &st.minions[m.index()];
                        // Expulsion: no reaction cards this turn.
                        if v.in_torpor || v.expelled {
                            return false;
                        }
                        let woken = a.woken.contains(m);
                        if needs_locked {
                            v.locked && !woken
                        } else {
                            !v.locked || woken
                        }
                    })
                    .map(Some)
                    .collect()
            }
            _ => vec![],
        }
    }

    /// Every legal play of a card from `seat`'s hand in any of `windows`,
    /// enumerated from the definitions: mode windows, actor, discipline
    /// tags, requirements, target filter, costs through layers, limits,
    /// out-of-turn availability.
    pub(crate) fn play_options(
        &self,
        seat: SeatId,
        windows: &[Window],
        played: Option<&PlayCtx>,
    ) -> Vec<Choice> {
        let st = &self.state;
        let action = self.action.as_ref();
        let combat = self.combat.as_ref();
        let referendum = self.referendum.as_ref();
        let mut out = Vec::new();
        for (hi, &card) in st.seats[seat.index()].hand.iter().enumerate() {
            let Some(def) = cards::lookup(card) else {
                continue;
            };
            if def.out_of_turn && st.seats[seat.index()].out_of_turn.is_some() {
                continue; // never two out-of-turn cards between your turns
            }
            // "You cannot voluntarily contest cards with yourself" (rulebook
            // "Contested Cards"; spec §1.13.1): a unique card is not
            // playable while a copy of it is in play under this seat —
            // contested or not (rules-qa 7.1).
            if LIBRARY[card as usize].labels.contains(&Label::Unique)
                && st.cards.iter().enumerate().any(|(i, c)| {
                    c.in_play
                        && st.card_controller(CardId(i as u32)) == seat
                        && LIBRARY[c.card as usize].name == LIBRARY[card as usize].name
                })
            {
                continue;
            }
            for (mi, mode) in def.modes.iter().enumerate() {
                if !mode.windows.iter().any(|w| windows.contains(w)) {
                    continue;
                }
                // The rulebook's one limited bleed increase per action.
                if mode
                    .effects
                    .iter()
                    .any(|e| matches!(e, Effect::BleedMod { limited: true, .. }))
                    && action.is_some_and(|a| a.limited_bleed_used)
                {
                    continue;
                }
                let tags = LIBRARY[card as usize].modes[mi].tags;
                let clans = LIBRARY[card as usize].clans;
                for actor in self.actor_candidates(seat, mode) {
                    // A political action card cannot be played if its terms
                    // cannot be met (LSJ 20100129).
                    if mode.effects.iter().any(|e| match (e, actor) {
                        (Effect::CallReferendum(r), Some(m)) => !r.feasible(st, seat, m),
                        _ => false,
                    }) {
                        continue;
                    }
                    // Clan requirement: the playing minion's clan, or (a
                    // Methuselah's card) a ready vampire of the clan.
                    if !clans.is_empty() {
                        let of_clan =
                            |m: MinionId| clans.contains(&st.minions[m.index()].card().clan);
                        let ok = match actor {
                            Some(m) => of_clan(m),
                            None => st
                                .minions_of(seat)
                                .any(|m| st.minions[m.index()].ready() && of_clan(m)),
                        };
                        if !ok {
                            continue;
                        }
                    }
                    if let Some(m) = actor {
                        if !cards::has_disciplines(st, m, card, tags) {
                            continue;
                        }
                        // One political action per minion per turn
                        // (rulebook "Political Action"); a political action
                        // canceled by Delaying Tactics cannot be performed
                        // again this turn by the seat's minions.
                        if mode.effects.contains(&Effect::DeclareAction(
                            crate::decision::ActionKind::Political,
                        )) && (st.minions[m.index()].political_this_turn
                            || st.seats[seat.index()]
                                .forbidden_political
                                .contains(&LIBRARY[card as usize].name))
                        {
                            continue;
                        }
                        // NRA (rules-qa 1.4): same action card name once per
                        // minion per turn.
                        if mode
                            .effects
                            .iter()
                            .any(|e| matches!(e, Effect::DeclareAction(_)))
                            && st.minions[m.index()]
                                .acted
                                .iter()
                                .any(|a| a.same(ActionId::Card(card)))
                        {
                            continue;
                        }
                        let x = cards::terms_x(st, seat, card, mode).unwrap_or(0);
                        if cards::blood_cost(st, card, x) > st.minions[m.index()].blood {
                            continue;
                        }
                        // "A vampire can play only one X each round / combat /
                        // action" (a canceled play counted at announcement).
                        let name = LIBRARY[card as usize].name;
                        let hit =
                            |v: &[(MinionId, &str)]| v.iter().any(|&(a, n)| a == m && n == name);
                        let limited = match mode.limit {
                            None => false,
                            Some(Limit::PerRound) => combat.is_some_and(|c| hit(&c.played_round)),
                            Some(Limit::PerCombat) => combat.is_some_and(|c| hit(&c.played_combat)),
                            Some(Limit::PerAction) => action.is_some_and(|a| hit(&a.played)),
                            Some(Limit::PerPhaseEach) => false,
                        };
                        if limited {
                            continue;
                        }
                        // Effect-derived combat legality (alternation, the
                        // limited additional strike, grapples).
                        if let Some(c) = combat {
                            if let Some(i) = c.side_of_minion(m) {
                                if !c.allows(mode, i) {
                                    continue;
                                }
                            }
                        }
                    }
                    if !mode.requires.iter().all(|&r| {
                        cards::holds(
                            r,
                            st,
                            seat,
                            actor,
                            played,
                            action,
                            combat,
                            referendum,
                            mode.waives_needed,
                        )
                    }) {
                        continue;
                    }
                    let pool = st.seats[seat.index()].pool;
                    let targets: Vec<Target> = match mode.target {
                        None => vec![Target::None],
                        Some(spec) => match spec.kind {
                            // No card play targets the uncontrolled region
                            // in the pool (an in-play ability does).
                            TargetKind::Uncontrolled => vec![],
                            TargetKind::Minion => (0..st.minions.len())
                                .map(|i| MinionId(i as u32))
                                .filter(|&id| {
                                    st.minions[id.index()].present()
                                        && spec.filter.iter().all(|&r| {
                                            cards::holds_for_minion(
                                                r,
                                                st,
                                                seat,
                                                referendum,
                                                action.map(|a| a.acting),
                                                id,
                                            )
                                        })
                                })
                                .map(Target::Minion)
                                .collect(),
                            TargetKind::Seat => (0..st.seats.len())
                                .map(|i| SeatId(i as u8))
                                .filter(|&t| {
                                    spec.filter
                                        .iter()
                                        .all(|&r| cards::holds_for_seat(r, st, seat, action, t))
                                })
                                .map(Target::Seat)
                                .collect(),
                        },
                    };
                    for target in targets {
                        if cards::pool_cost(st, seat, card, target) <= pool {
                            out.push(Choice::Play {
                                card: hi as u8,
                                mode: mi as u8,
                                actor,
                                target,
                            });
                        }
                    }
                }
            }
        }
        out
    }

    pub(crate) fn master_request(&self) -> DecisionRequest {
        let s = self.state.active;
        let mut options = vec![Choice::Pass];
        if self.state.seats[s.index()].mpa > 0 {
            options.extend(self.play_options(s, &[Window::MasterPhase], None));
        }
        // "During their master phase" abilities of cards in play (Blood
        // Doll): no master phase action spent; and the untimed ones.
        options.extend(self.ability_options(s, Window::MasterPhase));
        options.extend(self.ability_options(s, Window::AnyImpulse));
        DecisionRequest {
            seat: s,
            window: WindowLabel::MasterPhase,
            options,
        }
    }

    pub(crate) fn master_apply(&mut self, choice: &Choice) {
        match choice {
            Choice::Pass => self.open_phase_rotation(Flow::MasterPhase),
            Choice::Play {
                card,
                mode,
                actor,
                target,
            } => {
                let s = self.state.active;
                self.state.seats[s.index()].mpa -= 1;
                self.announce_play(s, *card, *mode, *actor, *target, Flow::MasterPhase);
            }
            Choice::UseAbility {
                card,
                ability,
                actor,
                target,
            } => {
                if !self.use_ability(*card, *ability, *actor, *target, Flow::MasterPhase) {
                    self.flow = Flow::MasterPhase;
                }
            }
            other => unreachable!("illegal master phase choice {other:?}"),
        }
    }

    /// Announce a play: the card leaves the hand (temporarily out of play,
    /// rulebook "Playing Cards"), out-of-turn accounting is charged even if
    /// it is later canceled, terms are chosen, then the as-played window
    /// opens. `resume` is where the flow returns once the play settles.
    pub(crate) fn announce_play(
        &mut self,
        s: SeatId,
        hand: u8,
        mode: u8,
        actor: Option<MinionId>,
        target: Target,
        resume: Flow,
    ) {
        let card = self.state.seats[s.index()].hand.remove(hand as usize);
        let def = cards::lookup(card).expect("played card has a definition");
        if let (Some(limit), Some(m)) = (def.modes[mode as usize].limit, actor) {
            let name = LIBRARY[card as usize].name;
            match limit {
                Limit::PerRound => {
                    if let Some(c) = self.combat.as_mut() {
                        c.played_round.push((m, name));
                    }
                }
                Limit::PerCombat => {
                    if let Some(c) = self.combat.as_mut() {
                        c.played_combat.push((m, name));
                    }
                }
                Limit::PerAction => {
                    if let Some(a) = self.action.as_mut() {
                        a.played.push((m, name));
                    }
                }
                Limit::PerPhaseEach => {}
            }
        }
        if def.out_of_turn {
            self.state.seats[s.index()].out_of_turn = Some(OutOfTurn { trifle: def.trifle });
        }
        let ctx = PlayCtx {
            seat: s,
            card,
            mode,
            actor,
            target,
            x: 0,
            burn: None,
        };
        let mut ctx = ctx;
        if let Some(x) = cards::terms_x(&self.state, s, card, &def.modes[mode as usize]) {
            ctx.x = x;
        }
        match def.modes[mode as usize].terms {
            Some(Terms::CopiesControlled) => self.open_as_played(ctx, resume),
            Some(_) => {
                // Terms are chosen before the as-played window opens; a
                // nested play (inside another as-played window) keeps that
                // window's stack as its resume point.
                let resume = match std::mem::replace(&mut self.flow, Flow::GameOver) {
                    outer @ Flow::AsPlayed { .. } => outer,
                    _ => resume,
                };
                self.flow = Flow::PlayTerms {
                    ctx,
                    resume: Box::new(resume),
                }
            }
            None => self.open_as_played(ctx, resume),
        }
    }

    pub(crate) fn terms_request(&self, ctx: &PlayCtx) -> DecisionRequest {
        let def = cards::lookup(ctx.card).expect("definition");
        let options = match def.modes[ctx.mode as usize].terms {
            // The amount is announced in the printed range; at resolution
            // you take what you can (Villein rulings, LSJ 20100216).
            Some(Terms::BloodRange { min, max }) => (min..=max).map(Choice::SetX).collect(),
            // X may exceed what is needed but never the actor's blood
            // (Hidden Strength rulings).
            Some(Terms::XBlood) => {
                let blood = self.state.minions[ctx.actor.expect("actor").index()].blood;
                (0..=blood).map(Choice::SetX).collect()
            }
            Some(Terms::UpToMargin(n)) => {
                let margin = self.action.as_ref().map_or(0, |a| a.margin) as u8;
                (0..=n.min(margin)).map(Choice::SetX).collect()
            }
            // "You can burn a card named X (if any)": any copy in play,
            // whoever controls it, or none; nothing in play elides it.
            Some(Terms::BurnNamedInPlay(name)) => {
                let mut options = vec![Choice::Pass];
                for (i, c) in self.state.cards.iter().enumerate() {
                    if c.present() && LIBRARY[c.card as usize].name == name {
                        options.push(Choice::BurnInPlay { card: i as u32 });
                    }
                }
                options
            }
            Some(Terms::CopiesControlled) | Some(Terms::Discard { .. }) | None => {
                unreachable!("terms window without a choice")
            }
        };
        DecisionRequest {
            seat: ctx.seat,
            window: WindowLabel::PlayTerms,
            options,
        }
    }

    pub(crate) fn terms_apply(&mut self, mut ctx: PlayCtx, resume: Flow, choice: &Choice) {
        match choice {
            Choice::SetX(n) => ctx.x = *n,
            Choice::BurnInPlay { card } => ctx.burn = Some(CardId(*card)),
            Choice::Pass => ctx.burn = None,
            other => unreachable!("illegal terms choice {other:?}"),
        }
        // A nested play resumes the outer as-played stack it interrupted;
        // restore it so the new window stacks on top.
        if let Flow::AsPlayed { .. } = resume {
            self.flow = resume;
            self.open_as_played(ctx, Flow::GameOver);
        } else {
            self.open_as_played(ctx, resume);
        }
    }

    /// Open the as-played window of `ctx`: an ordered window over the other
    /// living Methuselahs, clockwise from the player. Nested inside another
    /// as-played window (a cancel being canceled), it stacks.
    fn open_as_played(&mut self, ctx: PlayCtx, resume: Flow) {
        let order: Vec<SeatId> = self
            .state
            .clockwise_from(ctx.seat)
            .into_iter()
            .filter(|&s| s != ctx.seat)
            .collect();
        let pending = PendingPlay {
            ctx,
            window: OrderedWindow::new(order),
            canceled: false,
        };
        let flow = std::mem::replace(&mut self.flow, Flow::GameOver);
        match flow {
            Flow::AsPlayed { mut stack, resume } => {
                stack.push(pending);
                self.flow = Flow::AsPlayed { stack, resume };
            }
            _ => {
                self.flow = Flow::AsPlayed {
                    stack: vec![pending],
                    resume: Box::new(resume),
                }
            }
        }
    }

    /// The as-played window admits cancels, and wakes during an action
    /// (rules-qa 2.4; the Wake mode lists CardAsPlayed among its windows).
    pub(crate) fn as_played_request(&self, stack: &[PendingPlay]) -> Option<DecisionRequest> {
        let top = stack.last().expect("as-played stack");
        let seat = top.window.current()?;
        let mut options = vec![Choice::Pass];
        options.extend(self.play_options(seat, &[Window::CardAsPlayed], Some(&top.ctx)));
        Some(DecisionRequest {
            seat,
            window: WindowLabel::CardAsPlayed,
            options,
        })
    }

    pub(crate) fn as_played_apply(
        &mut self,
        mut stack: Vec<PendingPlay>,
        resume: Box<Flow>,
        choice: &Choice,
    ) {
        match choice {
            Choice::Pass => {
                stack.last_mut().expect("as-played stack").window.pass();
                self.flow = Flow::AsPlayed { stack, resume };
            }
            Choice::Play {
                card,
                mode,
                actor,
                target,
            } => {
                let seat = stack
                    .last()
                    .and_then(|p| p.window.current())
                    .expect("as-played impulse holder");
                // The play voids prior passes on the window it interrupts.
                stack.last_mut().expect("as-played stack").window.play();
                self.flow = Flow::AsPlayed { stack, resume };
                self.announce_play(seat, *card, *mode, *actor, *target, Flow::GameOver);
            }
            other => unreachable!("illegal as-played choice {other:?}"),
        }
    }

    /// The top as-played window closed: resolve (or discard) that play, then
    /// continue beneath it. A play canceled by the one just resolved is
    /// settled immediately: nothing is "being played" any more.
    pub(crate) fn as_played_advance(&mut self, mut stack: Vec<PendingPlay>, resume: Box<Flow>) {
        let top = stack.pop().expect("as-played stack");
        debug_assert!(top.window.is_closed());
        let mut declared_action = false;
        if top.canceled {
            self.settle_canceled(top.ctx);
        } else {
            declared_action = self.resolve_play(top.ctx, &mut stack);
        }
        while stack.last().is_some_and(|p| p.canceled) {
            let p = stack.pop().expect("canceled play");
            self.settle_canceled(p.ctx);
        }
        // Resolution may have ousted the player (pool paid down to 0), in
        // which case the flow already moved on and must not be resumed; a
        // resolved action card has opened its action instead.
        if self.ousted_during_play {
            self.ousted_during_play = false;
            return;
        }
        if declared_action {
            debug_assert!(stack.is_empty());
            return;
        }
        if stack.is_empty() {
            self.flow = *resume;
        } else {
            self.flow = Flow::AsPlayed { stack, resume };
        }
    }

    /// A canceled card was still played (spec §1.8.3): it goes to the ash
    /// heap and is replaced normally (§1.9.3); with "its cost is not paid"
    /// nothing was paid, since costs are paid at resolution (§1.7.1).
    fn settle_canceled(&mut self, ctx: PlayCtx) {
        self.state.seats[ctx.seat.index()].ash.push(ctx.card);
        self.draw_to_hand_size(ctx.seat);
    }

    /// Resolution: pay the cost, apply the effects, move the card, replace.
    /// Returns true when the play declared an action (the flow is now the
    /// action's).
    fn resolve_play(&mut self, ctx: PlayCtx, stack: &mut [PendingPlay]) -> bool {
        let s = ctx.seat;
        let def = cards::lookup(ctx.card).expect("definition");
        let mode = &def.modes[ctx.mode as usize];
        let is_reaction = LIBRARY[ctx.card as usize]
            .types
            .contains(&CardType::Reaction);
        // An action card: the action is declared now (cost at the action's
        // resolution) and the card is replaced before any modifier (spec
        // §1.9.1).
        if let Some(Effect::DeclareAction(kind)) = mode
            .effects
            .iter()
            .find(|e| matches!(e, Effect::DeclareAction(_)))
        {
            let actor = ctx.actor.expect("acting minion");
            self.draw_to_hand_size(s);
            self.declare_action(actor, *kind, None, Some(ctx), None);
            return true;
        }
        let pool = cards::pool_cost(&self.state, s, ctx.card, ctx.target);
        self.state.seats[s.index()].pool -= pool;
        if let Some(m) = ctx.actor {
            let blood = cards::blood_cost(&self.state, ctx.card, ctx.x);
            self.state.minions[m.index()].blood -= blood;
        }
        let mut in_play: Option<CardId> = None;
        for effect in mode.effects {
            match *effect {
                Effect::CancelPlayed { refund: _ } => {
                    // The play beneath this one on the stack is the card
                    // being played that this cancel targets.
                    if let Some(below) = stack.last_mut() {
                        below.canceled = true;
                    }
                }
                e => self.apply_card_effect(e, &ctx, &mut in_play),
            }
        }
        match in_play {
            // Entered play: a unique copy contests once its own entry
            // effects have resolved (spec §1.13.2).
            Some(id) => self.enter_library_contest(id),
            None => self.state.seats[s.index()].ash.push(ctx.card),
        }
        if def.trifle && self.state.active == s {
            let seat = &mut self.state.seats[s.index()];
            if !seat.trifle_used {
                seat.mpa += 1;
                seat.trifle_used = true;
            }
        }
        let deferred = is_reaction
            && self
                .action
                .as_ref()
                .is_some_and(|a| a.defer_reaction_replacement);
        if def.delayed_replacement {
            // "Do not replace until your next unlock phase".
            self.state.seats[s.index()].delayed_draws += 1;
        } else if deferred {
            self.action.as_mut().expect("action").deferred_draws.push(s);
        } else {
            self.draw_to_hand_size(s);
        }
        if self.state.seats[s.index()].pool == 0 {
            if s == self.state.active {
                self.oust_and_pass(s);
                self.ousted_during_play = true;
            } else {
                // An out-of-turn play paid down to 0 (Archon Investigation
                // at 3 pool): ousted, the turn goes on without them.
                self.oust(s);
                if self.state.living() <= 1 {
                    self.ousted_during_play = true;
                } else {
                    for p in stack.iter_mut() {
                        p.window.remove(s);
                    }
                    if let Some(a) = self.action.as_mut() {
                        a.order.retain(|&o| o != s);
                        a.window.remove(s);
                    }
                }
            }
        } else if self.state.seats[self.state.active.index()].ousted {
            // A rider fired by the play's effects (Archon Investigation's
            // burn under Tension in the Ranks) ousted the active Methuselah:
            // the turn passes with the action.
            if self.state.living() > 1 {
                self.next_turn();
            }
            self.ousted_during_play = true;
        }
        false
    }

    /// A library card enters play (the single entry point: contests are
    /// checked by the caller once the card's entry effects resolved).
    pub(crate) fn enter_play(&mut self, card: CardInPlay) -> CardId {
        self.state.cards.push(card);
        CardId(self.state.cards.len() as u32 - 1)
    }

    /// One effect of a card resolving (a play, or an action card on
    /// success): entering play, blood moves, layers, and the combat, vote
    /// and action effects of the contexts that exist.
    pub(crate) fn apply_card_effect(
        &mut self,
        effect: Effect,
        ctx: &PlayCtx,
        in_play: &mut Option<CardId>,
    ) {
        let s = ctx.seat;
        match effect {
            Effect::PutOnTarget | Effect::PutInPlay => {
                let host = match (effect, ctx.target) {
                    (Effect::PutOnTarget, Target::Minion(host)) => Some(host),
                    (Effect::PutOnTarget, _) => unreachable!("put-on without a minion target"),
                    _ => None,
                };
                *in_play = Some(self.enter_play(CardInPlay {
                    card: ctx.card,
                    owner: s,
                    controller: s,
                    host,
                    in_play: true,
                    contested: false,
                    locked: false,
                    counters: 0,
                }));
            }
            Effect::EnterWithCounters(n) => {
                let id = in_play.expect("counters on a card that entered play");
                self.state.cards[id.index()].counters = n;
            }
            Effect::MoveBloodTargetToPoolX => {
                let Target::Minion(t) = ctx.target else {
                    unreachable!("blood move without a minion target");
                };
                let moved = ctx.x.min(self.state.minions[t.index()].blood);
                self.state.minions[t.index()].blood -= moved;
                self.state.seats[s.index()].pool += moved as i32;
            }
            Effect::Layer(modifier) => {
                let source = in_play.expect("layer source must be in play");
                self.state.layers.push(Layer {
                    scope: Scope::InPlay,
                    modifier,
                    source,
                });
            }
            // Announcement-time effects: read where the action is declared.
            Effect::DeclareAction(_)
            | Effect::EmployRetainer { .. }
            | Effect::CallReferendum(_)
            | Effect::CancelPlayed { .. } => {}
            Effect::BurnChosenInPlay => {
                if let Some(id) = ctx.burn {
                    self.burn_card_in_play(id);
                }
            }
            Effect::HostBurnBloodToUnlock
            | Effect::HostBloodToPool(_)
            | Effect::PoolToHostBlood(_)
            | Effect::HuntingGroundBlood(_)
            | Effect::UnlockBurnPoolUnlessReady { .. }
            | Effect::PreyBurnsPool(_)
            | Effect::BurnPool(_)
            | Effect::PoolToCounter(_)
            | Effect::PoolOrBloodPerCounter
            | Effect::PoolOrRandomCardsPerCounter
            | Effect::PayPool(_)
            | Effect::TheirControllerBurnsPool(_)
            | Effect::SpendMasterPhaseAction
            | Effect::LockSelf
            | Effect::AddCounter(_)
            | Effect::BurnSelfAtCounters(_)
            | Effect::GainPool(_)
            | Effect::HandSizeBonusUntilEndOfTurn(_)
            | Effect::BloodToUncontrolledTarget(_)
            | Effect::BloodToTarget(_)
            | Effect::PredatorTakesControl
            | Effect::BurnSelf
            | Effect::BurnSource => unreachable!("ability or rider effect in a play"),
            e if is_combat_effect(&e) => self.apply_combat_effect(e, ctx),
            // A mode shared between a bleed and the polling step
            // (Protected District) carries both effects; the one
            // without a context does nothing.
            e if is_vote_effect(&e) => {
                if self.referendum.is_some() {
                    self.apply_vote_effect(e, ctx)
                }
            }
            action_effect => self.apply_action_effect(action_effect, ctx),
        }
    }

    pub(crate) fn discard_request(&self) -> DecisionRequest {
        let s = self.state.active;
        let seat = &self.state.seats[s.index()];
        let mut options = vec![Choice::Pass];
        if seat.dpa > 0 {
            options.extend((0..seat.hand.len()).map(|i| Choice::Discard(i as u8)));
        }
        options.extend(self.ability_options(s, Window::AnyImpulse));
        DecisionRequest {
            seat: s,
            window: WindowLabel::DiscardPhase,
            options,
        }
    }

    pub(crate) fn discard_apply(&mut self, choice: &Choice) {
        let s = self.state.active;
        match choice {
            Choice::Pass => self.open_phase_rotation(Flow::Discard),
            Choice::Discard(i) => {
                let seat = &mut self.state.seats[s.index()];
                let card = seat.hand.remove(*i as usize);
                seat.ash.push(card);
                seat.dpa -= 1;
                self.draw_to_hand_size(s);
                self.flow = Flow::Discard;
            }
            Choice::UseAbility {
                card,
                ability,
                actor,
                target,
            } => {
                if !self.use_ability(*card, *ability, *actor, *target, Flow::Discard) {
                    self.flow = Flow::Discard;
                }
            }
            other => unreachable!("illegal discard choice {other:?}"),
        }
    }

    /// In-play abilities usable in `window` by `seat`: host abilities of
    /// the seat's cards in play, and granted actions of any card in play
    /// ("Vampires can call a referendum …"), declared by the seat's ready
    /// unlocked minions like a card action (one political action per
    /// minion per turn; a Delaying Tactics ban is by card name).
    pub(crate) fn ability_options(&self, seat: SeatId, window: Window) -> Vec<Choice> {
        let st = &self.state;
        let mut out = Vec::new();
        for (ci, c) in st.cards.iter().enumerate() {
            // A contested card is out of play: nothing printed on it applies.
            if !c.present() {
                continue;
            }
            let Some(def) = cards::lookup(c.card) else {
                continue;
            };
            for (ai, ability) in def.abilities.iter().enumerate() {
                if !ability.windows.contains(&window) {
                    continue;
                }
                // "During X" abilities: once per phase per copy (rules-qa
                // 6.6) — per mode with `Limit::PerPhaseEach`.
                let key = phase_key(CardId(ci as u32), ai as u8, ability.limit);
                if window.is_phase() && st.phase_uses.contains(&key) {
                    continue;
                }
                match ability.actor {
                    // The controller picks the ability's target (a hunting
                    // ground: "a ready vampire you control can gain 1 blood").
                    Actor::Controller => {
                        if st.card_controller(CardId(ci as u32)) != seat {
                            continue;
                        }
                        let ok = ability.requires.iter().all(|&r| {
                            cards::holds_for_card(r, st, CardId(ci as u32))
                                && cards::holds(
                                    r,
                                    st,
                                    seat,
                                    None,
                                    None,
                                    self.action.as_ref(),
                                    self.combat.as_ref(),
                                    self.referendum.as_ref(),
                                    false,
                                )
                        });
                        if !ok {
                            continue;
                        }
                        let targets: Vec<Target> = match ability.target {
                            None => vec![Target::None],
                            Some(TargetSpec {
                                kind: TargetKind::Uncontrolled,
                                ..
                            }) => (0..st.seats[seat.index()].uncontrolled.len())
                                .map(|i| Target::Uncontrolled(i as u8))
                                .collect(),
                            Some(spec) => (0..st.minions.len())
                                .map(|i| MinionId(i as u32))
                                .filter(|&id| {
                                    st.minions[id.index()].present()
                                        && spec.filter.iter().all(|&r| {
                                            cards::holds_for_minion(
                                                r,
                                                st,
                                                seat,
                                                self.referendum.as_ref(),
                                                self.action.as_ref().map(|a| a.acting),
                                                id,
                                            )
                                        })
                                })
                                .map(Target::Minion)
                                .collect(),
                        };
                        for target in targets {
                            out.push(Choice::UseAbility {
                                card: ci as u32,
                                ability: ai as u8,
                                actor: None,
                                target,
                            });
                        }
                    }
                    // "Methuselahs can …": any Methuselah, in their own
                    // window, whoever controls the card.
                    Actor::Methuselah => {
                        let ok = ability.requires.iter().all(|&r| {
                            cards::holds_for_card(r, st, CardId(ci as u32))
                                && cards::holds(
                                    r,
                                    st,
                                    seat,
                                    None,
                                    None,
                                    self.action.as_ref(),
                                    self.combat.as_ref(),
                                    self.referendum.as_ref(),
                                    false,
                                )
                        });
                        if ok {
                            out.push(Choice::UseAbility {
                                card: ci as u32,
                                ability: ai as u8,
                                actor: None,
                                target: Target::None,
                            });
                        }
                    }
                    // The host's controller uses it (a card put on another
                    // Methuselah's vampire is theirs to use).
                    Actor::Host => {
                        let Some(host) = c.host.filter(|h| st.minions[h.index()].owner == seat)
                        else {
                            continue;
                        };
                        if !st.minions[host.index()].present() {
                            continue;
                        }
                        // In combat, the host must be the seat's combatant.
                        if let Some(cb) = self.combat.as_ref() {
                            if cb.side_of(seat) != cb.side_of_minion(host) {
                                continue;
                            }
                        }
                        // Per-copy limits ("each combat").
                        let id = CardId(ci as u32);
                        let spent = match ability.limit {
                            Some(Limit::PerCombat) => self
                                .combat
                                .as_ref()
                                .is_some_and(|cb| cb.abilities_used.contains(&id)),
                            Some(Limit::PerRound) | Some(Limit::PerAction) => {
                                unreachable!("limit not supported on abilities yet")
                            }
                            Some(Limit::PerPhaseEach) | None => false,
                        };
                        if spent {
                            continue;
                        }
                        let ok = ability.requires.iter().all(|&r| {
                            cards::holds_for_card(r, st, id)
                                && cards::holds_for_host(r, st, host)
                                && cards::holds(
                                    r,
                                    st,
                                    seat,
                                    Some(host),
                                    None,
                                    self.action.as_ref(),
                                    self.combat.as_ref(),
                                    self.referendum.as_ref(),
                                    false,
                                )
                        });
                        if ok {
                            out.push(Choice::UseAbility {
                                card: ci as u32,
                                ability: ai as u8,
                                actor: None,
                                target: Target::None,
                            });
                        }
                    }
                    Actor::ActingCandidate => {
                        let political = ability
                            .effects
                            .contains(&Effect::DeclareAction(ActionKind::Political));
                        let name = LIBRARY[c.card as usize].name;
                        if political && st.seats[seat.index()].forbidden_political.contains(&name) {
                            continue;
                        }
                        for m in self.actor_candidates(seat, ability).into_iter().flatten() {
                            if political && st.minions[m.index()].political_this_turn {
                                continue;
                            }
                            // NRA for a card in play: the same copy once per
                            // minion per turn (rules-qa 1.4).
                            if st.minions[m.index()]
                                .acted
                                .iter()
                                .any(|a| a.same(ActionId::InPlay(CardId(ci as u32))))
                            {
                                continue;
                            }
                            if ability.effects.iter().any(|e| match e {
                                Effect::CallReferendum(r) => !r.feasible(st, seat, m),
                                _ => false,
                            }) {
                                continue;
                            }
                            if !ability.requires.iter().all(|&r| {
                                cards::holds(r, st, seat, Some(m), None, None, None, None, false)
                            }) {
                                continue;
                            }
                            out.push(Choice::UseAbility {
                                card: ci as u32,
                                ability: ai as u8,
                                actor: Some(m),
                                target: Target::None,
                            });
                        }
                    }
                    other => unreachable!("not an ability actor: {other:?}"),
                }
            }
        }
        out
    }

    /// Use an in-play ability. Returns true when the flow moved on: a
    /// granted action was declared, or the user paid themselves out
    /// (`PoolToHostBlood` at 1 pool: ousted, the turn passed) — callers
    /// must not restore their flow then.
    pub(crate) fn use_ability(
        &mut self,
        card: u32,
        ability: u8,
        actor: Option<MinionId>,
        target: Target,
        resume: Flow,
    ) -> bool {
        let c = &self.state.cards[card as usize];
        let def = cards::lookup(c.card).expect("definition");
        let mode = &def.abilities[ability as usize];
        // A granted action: declared like a card action, the card in play
        // as its source (its referendum and cost are read at resolution).
        if let Some(Effect::DeclareAction(kind)) = mode
            .effects
            .iter()
            .find(|e| matches!(e, Effect::DeclareAction(_)))
        {
            let m = actor.expect("acting minion");
            self.declare_action(m, *kind, None, None, Some(CardId(card)));
            // The stealth of a granted action is the card's own ("as a +1
            // stealth political action"), not the rulebook's for political
            // action cards (rules-qa 6.2).
            let stealth: u8 = mode
                .effects
                .iter()
                .map(|e| match e {
                    Effect::AddStealth(n) => *n,
                    _ => 0,
                })
                .sum();
            let a = self.action.as_mut().expect("action");
            a.ability = Some(ability);
            // Added to the rulebook's default for the kind (none for a
            // granted action, rules-qa 6.2: the printed stealth only).
            a.base_stealth += stealth;
            return true;
        }
        // Terms announced with the use (the master cards to discard): the
        // ability resolves once they are chosen.
        if let Some(Terms::Discard { .. }) = mode.terms {
            self.flow = Flow::AbilityTerms {
                card: CardId(card),
                ability,
                picked: Vec::new(),
                resume: Box::new(resume),
            };
            return true;
        }
        self.resolve_ability(card, ability, target)
    }

    /// The Methuselah using an ability: the active one for "Methuselahs
    /// can …", the card's controller otherwise.
    fn ability_user(&self, card: CardId, ability: u8) -> SeatId {
        let def = cards::lookup(self.state.cards[card.index()].card).expect("definition");
        match def.abilities[ability as usize].actor {
            Actor::Methuselah => self.state.active,
            _ => self.state.card_controller(card),
        }
    }

    /// Terms of an ability: the next master card to discard, among those
    /// not picked yet.
    pub(crate) fn ability_terms_request(
        &self,
        card: CardId,
        ability: u8,
        picked: &[u8],
    ) -> DecisionRequest {
        let seat = self.ability_user(card, ability);
        let def = cards::lookup(self.state.cards[card.index()].card).expect("definition");
        let Some(Terms::Discard { masters, .. }) = def.abilities[ability as usize].terms else {
            unreachable!("ability terms without discards");
        };
        let options = self
            .state
            .hand_picks(seat, masters)
            .into_iter()
            .filter(|i| !picked.contains(i))
            .map(Choice::Discard)
            .collect();
        DecisionRequest {
            seat,
            window: WindowLabel::PlayTerms,
            options,
        }
    }

    pub(crate) fn ability_terms_apply(
        &mut self,
        card: CardId,
        ability: u8,
        mut picked: Vec<u8>,
        resume: Box<Flow>,
        pick: u8,
    ) {
        picked.push(pick);
        let def = cards::lookup(self.state.cards[card.index()].card).expect("definition");
        let Some(Terms::Discard { n, .. }) = def.abilities[ability as usize].terms else {
            unreachable!("ability terms without discards");
        };
        if picked.len() < n as usize {
            self.flow = Flow::AbilityTerms {
                card,
                ability,
                picked,
                resume,
            };
            return;
        }
        // The cost: discards to the ash heap, replaced as any card leaving
        // the hand.
        let seat = self.ability_user(card, ability);
        picked.sort_unstable_by(|a, b| b.cmp(a));
        for i in picked {
            let s = &mut self.state.seats[seat.index()];
            let c = s.hand.remove(i as usize);
            s.ash.push(c);
        }
        self.draw_to_hand_size(seat);
        self.flow = *resume;
        self.resolve_ability(card.0, ability, Target::None);
    }

    /// Resolve an in-play ability's effects (its terms, if any, chosen).
    /// Returns true when the flow moved on (the user ousted).
    fn resolve_ability(&mut self, card: u32, ability: u8, target: Target) -> bool {
        let c = &self.state.cards[card as usize];
        let def = cards::lookup(c.card).expect("definition");
        let mode = &def.abilities[ability as usize];
        let (seat, host) = match mode.actor {
            Actor::Controller => (self.state.card_controller(CardId(card)), None),
            Actor::Methuselah => (self.state.active, None),
            _ => {
                let host = c.host.expect("host");
                (self.state.minions[host.index()].owner, Some(host))
            }
        };
        let ctx = PlayCtx {
            seat,
            card: c.card,
            mode: ability,
            actor: host,
            target,
            x: 0,
            burn: None,
        };
        if let (Some(Limit::PerCombat), Some(cb)) = (mode.limit, self.combat.as_mut()) {
            cb.abilities_used.push(CardId(card));
        }
        if mode.windows.iter().any(|w| w.is_phase()) {
            self.state
                .phase_uses
                .push(phase_key(CardId(card), ability, mode.limit));
        }
        for e in mode.effects {
            match *e {
                Effect::SpendMasterPhaseAction => {
                    self.state.seats[ctx.seat.index()].mpa -= 1;
                }
                Effect::BurnSelf => self.burn_card_in_play(CardId(card)),
                Effect::LockSelf => self.state.cards[card as usize].locked = true,
                Effect::AddCounter(n) => self.state.cards[card as usize].counters += n,
                Effect::BurnSelfAtCounters(n) => {
                    if self.state.cards[card as usize].counters >= n {
                        self.burn_card_in_play(CardId(card));
                    }
                }
                Effect::GainPool(n) => self.state.seats[ctx.seat.index()].pool += n as i32,
                Effect::BloodToTarget(n) => {
                    let Target::Minion(t) = target else {
                        unreachable!("blood to a target without a minion");
                    };
                    let v = &mut self.state.minions[t.index()];
                    v.blood = (v.blood + n).min(v.capacity());
                }
                Effect::HandSizeBonusUntilEndOfTurn(n) => {
                    self.state.seats[ctx.seat.index()].hand_bonus += n;
                    self.draw_to_hand_size(ctx.seat);
                }
                Effect::BloodToUncontrolledTarget(n) => {
                    let Target::Uncontrolled(i) = target else {
                        unreachable!("blood to the uncontrolled region without a slot");
                    };
                    let slot = &mut self.state.seats[ctx.seat.index()].uncontrolled[i as usize];
                    let cap = crate::crypt::CRYPT[slot.card as usize].capacity;
                    slot.counters = (slot.counters + n).min(cap);
                }
                Effect::HuntingGroundBlood(n) => {
                    let Target::Minion(t) = target else {
                        unreachable!("hunting ground without a vampire");
                    };
                    let v = &mut self.state.minions[t.index()];
                    v.blood = (v.blood + n).min(v.capacity());
                    v.hunting_ground_fed = true;
                }
                Effect::HostBurnBloodToUnlock => {
                    let h = &mut self.state.minions[host.expect("host").index()];
                    h.blood -= 1;
                    h.locked = false;
                }
                Effect::HostBloodToPool(n) => {
                    let h = &mut self.state.minions[host.expect("host").index()];
                    let moved = n.min(h.blood);
                    h.blood -= moved;
                    self.state.seats[ctx.seat.index()].pool += moved as i32;
                }
                Effect::PoolToHostBlood(n) => {
                    let h = &mut self.state.minions[host.expect("host").index()];
                    let room = h.capacity() - h.blood;
                    let moved = n
                        .min(room)
                        .min(self.state.seats[ctx.seat.index()].pool.max(0) as u8);
                    h.blood += moved;
                    self.state.seats[ctx.seat.index()].pool -= moved as i32;
                    if self.state.seats[ctx.seat.index()].pool == 0 {
                        self.oust_and_pass(ctx.seat);
                    }
                }
                e if is_combat_effect(&e) => self.apply_combat_effect(e, &ctx),
                other => unreachable!("not an ability effect: {other:?}"),
            }
        }
        self.state.seats[ctx.seat.index()].ousted
    }

    /// Unlock-phase effects: an ordered window over the seats clockwise
    /// from the active one. The active Methuselah orders their pending
    /// mandatory riders (Anarch Revolt) among their own optional abilities
    /// and must resolve them all before passing the impulse (rules-qa
    /// 6.1); other Methuselahs then use theirs (Homunculus).
    pub(crate) fn abilities_request(
        &self,
        window: &OrderedWindow,
        pending: &[Pending],
    ) -> Option<DecisionRequest> {
        let seat = window.current()?;
        let mut options = Vec::new();
        if seat == self.state.active && !pending.is_empty() {
            for p in pending {
                match *p {
                    Pending::Rider(c) => options.push(Choice::ResolveRider { card: c.0 }),
                    // Re-read: a title's vampire may have lost its blood
                    // to another unlock effect since (a forced yield now).
                    Pending::Contest(item) => options.extend(self.contest_choices(item)),
                }
            }
        } else {
            options.push(Choice::Pass);
        }
        // The Edge may still be burned here, ordered among the other unlock
        // effects (Dreams of the Sphinx's Edge lock first, then the pool).
        if seat == self.state.active && self.state.seats[seat.index()].has_edge {
            options.push(Choice::BurnEdge);
        }
        options.extend(self.ability_options(seat, Window::UnlockPhaseAny));
        if seat == self.state.active {
            options.extend(self.ability_options(seat, Window::UnlockPhaseOwn));
        }
        options.extend(self.ability_options(seat, Window::AnyImpulse));
        Some(DecisionRequest {
            seat,
            window: WindowLabel::UnlockAbilities,
            options,
        })
    }

    pub(crate) fn abilities_apply(
        &mut self,
        mut window: OrderedWindow,
        mut pending: Vec<Pending>,
        choice: &Choice,
    ) {
        match choice {
            Choice::Pass => window.pass(),
            Choice::PayContest(item) | Choice::YieldContest(item) => {
                pending.retain(|p| *p != Pending::Contest(*item));
                if self.settle_contest(*item, matches!(choice, Choice::PayContest(_))) {
                    return; // ousted: the turn has passed
                }
                window.play();
            }
            Choice::BurnEdge => {
                let s = self.state.active;
                self.state.seats[s.index()].has_edge = false;
                self.state.seats[s.index()].pool += 1;
                window.play();
            }
            Choice::UseAbility {
                card,
                ability,
                actor,
                target,
            } => {
                window.play();
                let resume = Flow::Abilities {
                    window: window.clone(),
                    pending: pending.clone(),
                };
                if self.use_ability(*card, *ability, *actor, *target, resume) {
                    return; // ousted (the turn has passed) or terms pending
                }
            }
            Choice::ResolveRider { card } => {
                pending.retain(|p| *p != Pending::Rider(CardId(*card)));
                window.play();
                let resume = Flow::Abilities { window, pending };
                // Sets the flow: the resume, a choice window the rider
                // needs, or the next turn (the active Methuselah ousted).
                self.resolve_unlock_rider(CardId(*card), resume);
                return;
            }
            other => unreachable!("illegal ability choice {other:?}"),
        }
        self.flow = Flow::Abilities { window, pending };
    }

    /// Resolve a card in play's "during their unlock phase" rider on the
    /// active Methuselah's turn, then set the flow: `resume`, a choice
    /// window the rider needs (Smiling Jack's payments), or the next turn
    /// when the active Methuselah is ousted (or the game over).
    fn resolve_unlock_rider(&mut self, id: CardId, resume: Flow) {
        let s = self.state.active;
        let c = &self.state.cards[id.index()];
        if !c.present() {
            self.flow = resume;
            return;
        }
        let def = cards::lookup(c.card).expect("definition");
        let mut own_loss = 0i32;
        let mut to_counters = 0u8;
        let mut add_counters = 0u8;
        let mut per_counter = false;
        let mut loss = vec![0i32; self.state.seats.len()];
        let controller = self.state.card_controller(id);
        for r in def.riders.iter().filter(|r| {
            r.hook == Hook::UnlockPhase
                && r.scope.applies(controller, s)
                && self.state.rider_holds(r, id)
        }) {
            for e in r.effects {
                match *e {
                    Effect::BurnPool(n) => own_loss += n as i32,
                    Effect::PoolToCounter(n) => {
                        own_loss += n as i32;
                        to_counters += n;
                    }
                    Effect::PoolOrBloodPerCounter | Effect::PoolOrRandomCardsPerCounter => {
                        per_counter = true
                    }
                    Effect::AddCounter(n) => add_counters += n,
                    Effect::UnlockBurnPoolUnlessReady { sect, pool } => {
                        let has = self.state.minions_of(s).any(|m| {
                            let v = &self.state.minions[m.index()];
                            v.ready() && v.card().sect == sect
                        });
                        if !has {
                            own_loss += pool as i32;
                        }
                    }
                    Effect::PreyBurnsPool(n) => {
                        loss[self.state.prey(s).index()] += n as i32;
                    }
                    other => unreachable!("not a rider effect: {other:?}"),
                }
            }
        }
        if loss.iter().any(|&n| n > 0) {
            // Another Methuselah's loss: ousted at once if at 0, the active
            // seat scoring; the unlock phase goes on.
            self.burn_pool_batch(&loss);
            if self.state.living() <= 1 {
                return;
            }
        }
        if own_loss > 0 {
            let seat = &mut self.state.seats[s.index()];
            let moved = own_loss.min(seat.pool) as u8;
            seat.pool = (seat.pool - own_loss).max(0);
            // Counters move even as the last pool goes (the card leaves
            // with its ousted controller anyway).
            self.state.cards[id.index()].counters += to_counters.min(moved);
            if self.state.seats[s.index()].pool == 0 {
                self.oust_and_pass(s);
                return;
            }
        }
        self.state.cards[id.index()].counters += add_counters;
        let counters = self.state.cards[id.index()].counters;
        if per_counter && counters > 0 {
            self.flow = Flow::RiderTerms {
                card: id,
                left: counters,
                resume: Box::new(resume),
            };
            return;
        }
        self.flow = resume;
    }

    /// One unit of a rider's payment (Smiling Jack): 1 pool, or 1 blood
    /// from a vampire the active Methuselah controls that has some (LSJ
    /// 19971212: an empty vampire does not lessen the obligation).
    pub(crate) fn rider_terms_request(&self, card: CardId, left: u8) -> DecisionRequest {
        let s = self.state.active;
        let mut options = Vec::new();
        if self.rider_effect(card, Effect::PoolOrRandomCardsPerCounter) {
            // Constant Revolution: how many cards (the rest is pool), at
            // most the hand (LSJ 20090618) — one announcement.
            let hand = self.state.seats[s.index()].hand.len() as u8;
            options.extend((0..=left.min(hand)).map(Choice::SetX));
        } else {
            options.push(Choice::BurnPool);
            for m in self.state.minions_of(s) {
                if self.state.minions[m.index()].blood > 0 {
                    options.push(Choice::BurnBloodFrom { minion: m });
                }
            }
        }
        DecisionRequest {
            seat: s,
            window: WindowLabel::RiderTerms,
            options,
        }
    }

    /// Whether a card in play carries a rider with this effect.
    fn rider_effect(&self, card: CardId, effect: Effect) -> bool {
        cards::lookup(self.state.cards[card.index()].card)
            .is_some_and(|d| d.riders.iter().any(|r| r.effects.contains(&effect)))
    }

    pub(crate) fn rider_terms_apply(
        &mut self,
        card: CardId,
        left: u8,
        resume: Box<Flow>,
        choice: &Choice,
    ) {
        let s = self.state.active;
        match choice {
            Choice::SetX(k) => {
                // k cards burned at random, all at once, then the rest of
                // the units from the pool; the hand is drawn up afterward.
                for _ in 0..*k {
                    let seat = &mut self.state.seats[s.index()];
                    let i = self.rng.below(seat.hand.len());
                    let c = seat.hand.remove(i);
                    seat.ash.push(c);
                }
                let seat = &mut self.state.seats[s.index()];
                seat.pool = (seat.pool - (left - k) as i32).max(0);
                if seat.pool == 0 {
                    self.oust_and_pass(s);
                    return;
                }
                self.draw_to_hand_size(s);
                self.flow = *resume;
                return;
            }
            Choice::BurnPool => {
                let seat = &mut self.state.seats[s.index()];
                seat.pool -= 1;
                if seat.pool == 0 {
                    // Ousted on their own unlock phase: the turn passes.
                    self.oust_and_pass(s);
                    return;
                }
            }
            Choice::BurnBloodFrom { minion } => {
                self.state.minions[minion.index()].blood -= 1;
            }
            other => unreachable!("illegal rider terms choice {other:?}"),
        }
        if left > 1 {
            self.flow = Flow::RiderTerms {
                card,
                left: left - 1,
                resume,
            };
        } else {
            self.flow = *resume;
        }
    }

    /// "During your discard phase" riders the seat holding the impulse
    /// must resolve in the active Methuselah's discard phase: The Coven's
    /// transfer is the predator's, taken when they get the opportunity to
    /// play effects (LSJ 20100723).
    fn discard_pending(&self, seat: SeatId) -> Vec<CardId> {
        let s = self.state.active;
        let mut out = Vec::new();
        for ci in 0..self.state.cards.len() {
            let id = CardId(ci as u32);
            let c = &self.state.cards[ci];
            if !c.present() {
                continue;
            }
            let Some(def) = cards::lookup(c.card) else {
                continue;
            };
            let controller = self.state.card_controller(id);
            let resolver = self.state.predator(controller);
            if def.riders.iter().any(|r| {
                r.hook == Hook::DiscardPhase
                    && r.scope.applies(controller, s)
                    && self.state.rider_holds(r, id)
                    && resolver == seat
            }) {
                out.push(id);
            }
        }
        out
    }

    fn resolve_discard_rider(&mut self, id: CardId) {
        let ci = id.index();
        let def = cards::lookup(self.state.cards[ci].card).expect("definition");
        let controller = self.state.card_controller(id);
        let effects: Vec<Effect> = def
            .riders
            .iter()
            .filter(|r| r.hook == Hook::DiscardPhase)
            .flat_map(|r| r.effects.iter().copied())
            .collect();
        for e in effects {
            match e {
                Effect::PredatorTakesControl => {
                    let predator = self.state.predator(controller);
                    // A forced contest with a copy the predator already
                    // controls burns the incoming copy (spec §1.13.1).
                    let name = LIBRARY[self.state.cards[ci].card as usize].name;
                    let held = self.state.cards.iter().enumerate().any(|(j, o)| {
                        j != ci
                            && o.present()
                            && LIBRARY[o.card as usize].name == name
                            && self.state.card_controller(CardId(j as u32)) == predator
                    });
                    if held {
                        self.burn_card_in_play(id);
                    } else {
                        self.state.cards[ci].controller = predator;
                    }
                }
                other => unreachable!("not a discard-phase rider effect: {other:?}"),
            }
        }
    }

    /// The impulse round the other seats at the end of a phase (rules-qa
    /// 7.9): mandatory effects first (Pass masked), then untimed abilities.
    pub(crate) fn phase_rotation_request(
        &self,
        resume: &Flow,
        window: &OrderedWindow,
    ) -> Option<DecisionRequest> {
        let seat = window.current()?;
        let mut options = Vec::new();
        let pending = match resume {
            Flow::Discard => self.discard_pending(seat),
            _ => Vec::new(),
        };
        if pending.is_empty() {
            options.push(Choice::Pass);
            options.extend(self.ability_options(seat, Window::AnyImpulse));
        } else {
            options.extend(pending.iter().map(|c| Choice::ResolveRider { card: c.0 }));
        }
        Some(DecisionRequest {
            seat,
            window: WindowLabel::PhaseImpulse,
            options,
        })
    }

    pub(crate) fn phase_rotation_apply(
        &mut self,
        resume: Box<Flow>,
        mut window: OrderedWindow,
        choice: &Choice,
    ) {
        match choice {
            Choice::Pass => {
                window.pass();
                if window.is_closed() {
                    self.close_phase(*resume);
                } else {
                    self.flow = Flow::PhaseRotation { resume, window };
                }
            }
            Choice::ResolveRider { card } => {
                self.resolve_discard_rider(CardId(*card));
                self.flow = Flow::PhaseRotation { resume, window };
            }
            Choice::UseAbility {
                card,
                ability,
                actor,
                target,
            } => {
                // An effect: the impulse returns to the active Methuselah,
                // back in their phase.
                let phase = *resume;
                self.flow = phase.clone();
                if !self.use_ability(*card, *ability, *actor, *target, phase.clone()) {
                    self.flow = phase;
                }
            }
            other => unreachable!("illegal phase-rotation choice {other:?}"),
        }
    }

    /// Riders of `hook` fired for each minion of `subjects` that left the
    /// ready region: a host's own riders (`HostToTorpor`: the cards on it)
    /// or every card in play's (`ReadyMinionBurnedOrTorpor`). Pool losses
    /// are batched and applied at once — a Methuselah at 0 is ousted here,
    /// wherever it happened; the caller tears the combat or action down.
    pub(crate) fn leave_riders(&mut self, hook: Hook, subjects: &[MinionId]) {
        let mut loss = vec![0i32; self.state.seats.len()];
        for &subject in subjects {
            let controller = self.state.minions[subject.index()].owner;
            for ci in 0..self.state.cards.len() {
                let c = &self.state.cards[ci];
                if !c.present() || (hook == Hook::HostToTorpor && c.host != Some(subject)) {
                    continue;
                }
                let Some(def) = cards::lookup(c.card) else {
                    continue;
                };
                let riders: Vec<&Rider> = def
                    .riders
                    .iter()
                    .filter(|r| r.hook == hook && self.state.rider_holds(r, CardId(ci as u32)))
                    .collect();
                for r in riders {
                    for e in r.effects {
                        match *e {
                            Effect::BurnSelf => self.burn_card_in_play(CardId(ci as u32)),
                            Effect::TheirControllerBurnsPool(n) => {
                                loss[controller.index()] += n as i32;
                            }
                            other => unreachable!("not a leave rider effect: {other:?}"),
                        }
                    }
                }
            }
        }
        if loss.iter().any(|&n| n > 0) {
            self.burn_pool_batch(&loss);
        }
    }

    /// Name of a library card (debugging / tests).
    pub fn card_name(card: u16) -> &'static str {
        LIBRARY[card as usize].name
    }
}
