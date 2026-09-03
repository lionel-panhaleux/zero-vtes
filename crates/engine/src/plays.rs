//! Card plays from hand: the master phase and the discard phase as offer
//! windows, numeric terms at announcement (rules-qa 3.5), the as-played
//! window with a cancel stack (spec §1.8), resolution with cost folded
//! through layers, and replacement draws. The option list is enumerated
//! from the card definitions (`cards`): no card code decides legality.

use crate::cards::{self, Effect, Layer, PlayCtx, Scope, TargetKind, Terms, Window};
use crate::decision::{Choice, DecisionRequest, WindowLabel};
use crate::game::{Engine, Flow};
use crate::ids::{MinionId, SeatId};
use crate::library::LIBRARY;
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

    /// Every legal play of a card from `seat`'s hand in `window`, enumerated
    /// from the definitions: mode window, requirements, target filter, terms
    /// feasibility, cost through layers, out-of-turn availability.
    pub(crate) fn play_options(
        &self,
        seat: SeatId,
        window: Window,
        played: Option<&PlayCtx>,
    ) -> Vec<Choice> {
        let st = &self.state;
        let mut out = Vec::new();
        for (hi, &card) in st.seats[seat.index()].hand.iter().enumerate() {
            let Some(def) = cards::lookup(card) else {
                continue;
            };
            if def.out_of_turn && st.seats[seat.index()].out_of_turn.is_some() {
                continue; // never two out-of-turn cards between your turns
            }
            for (mi, mode) in def.modes.iter().enumerate() {
                if mode.window != window {
                    continue;
                }
                if !mode
                    .requires
                    .iter()
                    .all(|&r| cards::holds(r, st, seat, played))
                {
                    continue;
                }
                let pool = st.seats[seat.index()].pool;
                match mode.target {
                    None => {
                        if cards::pool_cost(st, seat, card, None) <= pool {
                            out.push(Choice::Play {
                                card: hi as u8,
                                mode: mi as u8,
                                target: None,
                            });
                        }
                    }
                    Some(spec) => {
                        let TargetKind::Minion = spec.kind;
                        for (i, m) in st.minions.iter().enumerate() {
                            let id = MinionId(i as u32);
                            if !m.present()
                                || !spec
                                    .filter
                                    .iter()
                                    .all(|&r| cards::holds_for(r, st, seat, id))
                            {
                                continue;
                            }
                            if cards::pool_cost(st, seat, card, Some(id)) <= pool {
                                out.push(Choice::Play {
                                    card: hi as u8,
                                    mode: mi as u8,
                                    target: Some(id),
                                });
                            }
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
            options.extend(self.play_options(s, Window::MasterPhase, None));
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
            Choice::Play { card, mode, target } => {
                let s = self.state.active;
                self.state.seats[s.index()].mpa -= 1;
                self.announce_play(s, *card, *mode, *target, Flow::MasterPhase);
            }
            other => unreachable!("illegal master phase choice {other:?}"),
        }
    }

    /// Announce a play: the card leaves the hand (temporarily out of play,
    /// rulebook "Playing Cards"), out-of-turn accounting is charged even if
    /// it is later canceled, terms are chosen, then the as-played window
    /// opens. `resume` is where the flow returns once the play settles.
    fn announce_play(
        &mut self,
        s: SeatId,
        hand: u8,
        mode: u8,
        target: Option<MinionId>,
        resume: Flow,
    ) {
        let card = self.state.seats[s.index()].hand.remove(hand as usize);
        let def = cards::lookup(card).expect("played card has a definition");
        if def.out_of_turn {
            self.state.seats[s.index()].out_of_turn = Some(OutOfTurn { trifle: def.trifle });
        }
        let ctx = PlayCtx {
            seat: s,
            card,
            mode,
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

    pub(crate) fn as_played_request(&self, stack: &[PendingPlay]) -> Option<DecisionRequest> {
        let top = stack.last().expect("as-played stack");
        let seat = top.window.current()?;
        let mut options = vec![Choice::Pass];
        options.extend(self.play_options(seat, Window::CardAsPlayed, Some(&top.ctx)));
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
            Choice::Play { card, mode, target } => {
                let seat = stack
                    .last()
                    .and_then(|p| p.window.current())
                    .expect("as-played impulse holder");
                // The play voids prior passes on the window it interrupts.
                stack.last_mut().expect("as-played stack").window.play();
                self.flow = Flow::AsPlayed { stack, resume };
                self.announce_play(seat, *card, *mode, *target, Flow::GameOver);
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
        if top.canceled {
            self.settle_canceled(top.ctx);
        } else {
            self.resolve_play(top.ctx, &mut stack);
        }
        while stack.last().is_some_and(|p| p.canceled) {
            let p = stack.pop().expect("canceled play");
            self.settle_canceled(p.ctx);
        }
        // Resolution may have ousted the player (pool paid down to 0), in
        // which case the flow already moved on and must not be resumed.
        if self.ousted_during_play {
            self.ousted_during_play = false;
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
    fn resolve_play(&mut self, ctx: PlayCtx, stack: &mut [PendingPlay]) {
        let s = ctx.seat;
        let def = cards::lookup(ctx.card).expect("definition");
        let mode = &def.modes[ctx.mode as usize];
        let cost = cards::pool_cost(&self.state, s, ctx.card, ctx.target);
        self.state.seats[s.index()].pool -= cost;
        let mut in_play: Option<CardId> = None;
        for effect in mode.effects {
            match *effect {
                Effect::PutOnTarget => {
                    self.state.cards.push(CardInPlay {
                        card: ctx.card,
                        owner: s,
                        host: ctx.target,
                        in_play: true,
                        locked: false,
                        counters: 0,
                    });
                    in_play = Some(CardId(self.state.cards.len() as u32 - 1));
                }
                Effect::MoveBloodTargetToPoolX => {
                    let t = ctx.target.expect("target");
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
        self.draw_to_hand_size(s);
        if self.state.seats[s.index()].pool == 0 {
            self.oust_and_pass(s);
            self.ousted_during_play = true;
        }
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

    /// Name of a library card (debugging / tests).
    pub fn card_name(card: u16) -> &'static str {
        LIBRARY[card as usize].name
    }
}
