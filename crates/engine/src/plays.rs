//! Card plays from hand: option enumeration from the definitions (no card
//! code decides legality), the master phase and the discard phase as offer
//! windows, numeric terms at announcement (rules-qa 3.5), the as-played
//! window with a cancel stack (spec §1.8), resolution with costs folded
//! through layers, replacement draws, and in-play abilities.

use crate::cards::{
    self, Actor, Effect, Layer, Limit, PlayCtx, Requirement, Scope, Target, TargetKind, Terms,
    Window,
};
use crate::combat::is_combat_effect;
use crate::decision::{Choice, DecisionRequest, WindowLabel};
use crate::game::{Engine, Flow};
use crate::ids::{MinionId, SeatId};
use crate::library::{CardType, LIBRARY};
use crate::referendum::is_vote_effect;
use crate::state::{CardId, CardInPlay, OutOfTurn, HAND_SIZE};
use crate::window::OrderedWindow;

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
        while seat.hand.len() < HAND_SIZE {
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
            Actor::Host => vec![],
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
                        if v.in_torpor {
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
                // A political action card cannot be played if its terms
                // cannot be met (LSJ 20100129).
                if mode.effects.iter().any(|e| match e {
                    Effect::CallReferendum(r) => !r.feasible(st, seat),
                    _ => false,
                }) {
                    continue;
                }
                for actor in self.actor_candidates(seat, mode) {
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
                        // (rulebook "Political Action").
                        if mode.effects.contains(&Effect::DeclareAction(
                            crate::decision::ActionKind::Political,
                        )) && st.minions[m.index()].political_this_turn
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
                                .any(|c| LIBRARY[*c as usize].name == LIBRARY[card as usize].name)
                        {
                            continue;
                        }
                        if cards::blood_cost(st, card, 0) > st.minions[m.index()].blood {
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
                            TargetKind::Minion => (0..st.minions.len())
                                .map(|i| MinionId(i as u32))
                                .filter(|&id| {
                                    st.minions[id.index()].present()
                                        && spec.filter.iter().all(|&r| {
                                            cards::holds_for_minion(r, st, seat, referendum, id)
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
        DecisionRequest {
            seat: s,
            window: WindowLabel::MasterPhase,
            options,
        }
    }

    pub(crate) fn master_apply(&mut self, choice: &Choice) {
        match choice {
            Choice::Pass => self.flow = Flow::MinionPhase,
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
        };
        match def.modes[mode as usize].terms {
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
            None => unreachable!("terms window without terms"),
        };
        DecisionRequest {
            seat: ctx.seat,
            window: WindowLabel::PlayTerms,
            options,
        }
    }

    pub(crate) fn terms_apply(&mut self, mut ctx: PlayCtx, resume: Flow, choice: &Choice) {
        match choice {
            Choice::SetX(n) => {
                ctx.x = *n;
                // A nested play resumes the outer as-played stack it
                // interrupted; restore it so the new window stacks on top.
                if let Flow::AsPlayed { .. } = resume {
                    self.flow = resume;
                    self.open_as_played(ctx, Flow::GameOver);
                } else {
                    self.open_as_played(ctx, resume);
                }
            }
            other => unreachable!("illegal terms choice {other:?}"),
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
            self.declare_action(actor, *kind, None, Some(ctx));
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
                Effect::PutOnTarget => {
                    let Target::Minion(host) = ctx.target else {
                        unreachable!("put-on without a minion target");
                    };
                    self.state.cards.push(CardInPlay {
                        card: ctx.card,
                        owner: s,
                        host: Some(host),
                        in_play: true,
                        locked: false,
                        counters: 0,
                    });
                    in_play = Some(CardId(self.state.cards.len() as u32 - 1));
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
                Effect::CancelPlayed { refund: _ } => {
                    // The play beneath this one on the stack is the card
                    // being played that this cancel targets.
                    if let Some(below) = stack.last_mut() {
                        below.canceled = true;
                    }
                }
                Effect::DeclareAction(_)
                | Effect::EmployRetainer { .. }
                | Effect::CallReferendum(_) => {}
                Effect::HostBurnBloodToUnlock => unreachable!("ability effect in a play"),
                e if is_combat_effect(&e) => self.apply_combat_effect(e, &ctx),
                e if is_vote_effect(&e) => self.apply_vote_effect(e, &ctx),
                action_effect => self.apply_action_effect(action_effect, &ctx),
            }
        }
        if in_play.is_none() {
            self.state.seats[s.index()].ash.push(ctx.card);
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
        if deferred {
            self.action.as_mut().expect("action").deferred_draws.push(s);
        } else {
            self.draw_to_hand_size(s);
        }
        if self.state.seats[s.index()].pool == 0 {
            self.oust_and_pass(s);
            self.ousted_during_play = true;
        }
        false
    }

    pub(crate) fn discard_request(&self) -> DecisionRequest {
        let s = self.state.active;
        let seat = &self.state.seats[s.index()];
        let mut options = vec![Choice::Pass];
        if seat.dpa > 0 {
            options.extend((0..seat.hand.len()).map(|i| Choice::Discard(i as u8)));
        }
        DecisionRequest {
            seat: s,
            window: WindowLabel::DiscardPhase,
            options,
        }
    }

    pub(crate) fn discard_apply(&mut self, choice: &Choice) {
        let s = self.state.active;
        match choice {
            Choice::Pass => self.end_turn(),
            Choice::Discard(i) => {
                let seat = &mut self.state.seats[s.index()];
                let card = seat.hand.remove(*i as usize);
                seat.ash.push(card);
                seat.dpa -= 1;
                self.draw_to_hand_size(s);
                self.flow = Flow::Discard;
            }
            other => unreachable!("illegal discard choice {other:?}"),
        }
    }

    /// In-play abilities usable in `window` by `seat`'s cards in play.
    pub(crate) fn ability_options(&self, seat: SeatId, window: Window) -> Vec<Choice> {
        let st = &self.state;
        let mut out = Vec::new();
        for (ci, c) in st.cards.iter().enumerate() {
            if !c.in_play || c.owner != seat {
                continue;
            }
            let Some(def) = cards::lookup(c.card) else {
                continue;
            };
            for (ai, ability) in def.abilities.iter().enumerate() {
                if !ability.windows.contains(&window) {
                    continue;
                }
                let Some(host) = c.host else {
                    continue;
                };
                if !st.minions[host.index()].present() {
                    continue;
                }
                if ability
                    .requires
                    .iter()
                    .all(|&r| cards::holds_for_host(r, st, host))
                {
                    out.push(Choice::UseAbility {
                        card: ci as u32,
                        ability: ai as u8,
                    });
                }
            }
        }
        out
    }

    pub(crate) fn use_ability(&mut self, card: u32, ability: u8) {
        let c = &self.state.cards[card as usize];
        let def = cards::lookup(c.card).expect("definition");
        let host = c.host.expect("host");
        for e in def.abilities[ability as usize].effects {
            match e {
                Effect::HostBurnBloodToUnlock => {
                    let h = &mut self.state.minions[host.index()];
                    h.blood -= 1;
                    h.locked = false;
                }
                other => unreachable!("not an ability effect: {other:?}"),
            }
        }
    }

    /// Abilities usable during any Methuselah's unlock phase: an ordered
    /// window over the seats clockwise from the active one.
    pub(crate) fn abilities_request(&self, window: &OrderedWindow) -> Option<DecisionRequest> {
        let seat = window.current()?;
        let mut options = vec![Choice::Pass];
        options.extend(self.ability_options(seat, Window::UnlockPhaseAny));
        Some(DecisionRequest {
            seat,
            window: WindowLabel::UnlockAbilities,
            options,
        })
    }

    pub(crate) fn abilities_apply(&mut self, mut window: OrderedWindow, choice: &Choice) {
        match choice {
            Choice::Pass => window.pass(),
            Choice::UseAbility { card, ability } => {
                self.use_ability(*card, *ability);
                window.play();
            }
            other => unreachable!("illegal ability choice {other:?}"),
        }
        self.flow = Flow::Abilities { window };
    }

    /// Name of a library card (debugging / tests).
    pub fn card_name(card: u16) -> &'static str {
        LIBRARY[card as usize].name
    }
}
