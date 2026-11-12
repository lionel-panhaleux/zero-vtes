//! The decision point before an oust (docs/engine.md §6.17): Life Boon is
//! played as a pool reaches 0, right before the oust (ANK 20220210).

use crate::cards::{Target, Window};
use crate::decision::{Choice, DecisionRequest, WindowLabel};
use crate::game::{Engine, Flow};
use crate::ids::SeatId;
use crate::library::LIBRARY;
use crate::notes::{Cause, Source};
use crate::state::CardId;

/// A Methuselah about to be ousted: `at` its pool as it ran out, plus what
/// the ousts ahead of it have paid it since (a predator's 6 pool).
#[derive(Clone, Copy, Debug)]
pub struct Pending {
    pub seat: SeatId,
    pub at: i32,
}

/// Methuselahs about to be ousted, and the Methuselahs who may save them.
#[derive(Clone, Debug)]
pub struct Ousting {
    /// The batches in the order their pools ran out, each ousted together
    /// once no Boon saves it; the front one is being asked.
    pub batches: Vec<Vec<Pending>>,
    /// The front batch's Boon holders, clockwise from the active
    /// Methuselah; `next` the one deciding.
    pub order: Vec<SeatId>,
    pub next: usize,
    /// A Boon of `order[next]` is being played (its terms, its as-played
    /// window): the play's own decisions come first.
    pub playing: bool,
}

/// Pool burns owed by separate triggers of one event (minions lost
/// together under Tension in the Ranks): not simultaneous, the acting
/// Methuselah sets their order (LSJ 20090208-2).
#[derive(Clone, Debug)]
pub struct BurnOrder {
    /// Each Methuselah's burn, by seat.
    pub loss: Vec<i32>,
    /// Those to burn first, in order.
    pub picked: Vec<SeatId>,
    pub by: Option<Source>,
}

/// Life Boon's LIBRARY index.
pub fn life_boon() -> u16 {
    static CARD: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
    *CARD.get_or_init(|| {
        LIBRARY
            .iter()
            .position(|c| c.name == "Life Boon")
            .expect("Life Boon in the pool") as u16
    })
}

/// Is a play of `card` (any card if `None`) by `seat` still under way
/// somewhere in `flow`?
pub(crate) fn in_flight(flow: &Flow, card: Option<u16>, seat: SeatId) -> bool {
    let is = |c: u16| card.is_none_or(|k| k == c);
    match flow {
        Flow::PlayTerms { ctx, resume } | Flow::CostTerms { ctx, resume, .. } => {
            (is(ctx.card) && ctx.seat == seat) || in_flight(resume, card, seat)
        }
        Flow::AsPlayed { stack, resume } => {
            stack.iter().any(|p| is(p.ctx.card) && p.ctx.seat == seat)
                || in_flight(resume, card, seat)
        }
        _ => false,
    }
}

impl Engine {
    /// Should the oust of `seats` wait on a Life Boon (one held outside the
    /// batch, not by the active Methuselah)? Behind an oust already waiting
    /// it waits its turn, a batch of its own.
    pub(crate) fn defer_oust(&mut self, seats: &[SeatId]) -> bool {
        let st = &mut self.state;
        let batch: Vec<Pending> = seats
            .iter()
            .filter(|s| !st.seats[s.index()].oust_pending)
            .map(|&seat| Pending {
                seat,
                at: st.seats[seat.index()].pool,
            })
            .collect();
        if self.ousting.is_none() {
            let order = self.boon_holders(seats);
            if order.is_empty() {
                return false;
            }
            self.ousting = Some(Ousting {
                batches: Vec::new(),
                order,
                next: 0,
                playing: false,
            });
        }
        for p in &batch {
            self.state.seats[p.seat.index()].oust_pending = true;
        }
        if !batch.is_empty() {
            self.ousting.as_mut().expect("ousting").batches.push(batch);
        }
        true
    }

    /// Who may save a Methuselah of `batch`: the Boon holders outside it,
    /// clockwise from the active Methuselah, never the active one.
    fn boon_holders(&self, batch: &[SeatId]) -> Vec<SeatId> {
        let boon = life_boon();
        let st = &self.state;
        st.clockwise_from(st.active)
            .into_iter()
            .filter(|&h| {
                h != st.active
                    && !batch.contains(&h)
                    && !st.seats[h.index()].ousted
                    && self.virtual_hand(h).iter().any(|&(c, _)| c == boon)
            })
            .collect()
    }

    /// The Boon plays `h` may make now, each on a Methuselah of the front
    /// batch.
    fn boon_plays(&self, h: SeatId) -> Vec<Choice> {
        let boon = life_boon();
        let hand = self.virtual_hand(h);
        let front = &self.ousting.as_ref().expect("ousting").batches[0];
        self.play_options(h, &[Window::Ousting], None)
            .into_iter()
            .filter(|o| {
                matches!(o, Choice::Play { card, target: Target::Seat(t), .. }
                    if hand[*card as usize].0 == boon && front.iter().any(|p| p.seat == *t))
            })
            .collect()
    }

    /// Before any decision: end a finished Boon play's wait, drop the
    /// saved, and oust each batch in turn once no one is left to ask.
    pub(crate) fn settle_ousting(&mut self) {
        let Some(o) = self.ousting.as_ref() else {
            return;
        };
        if o.playing {
            if in_flight(&self.flow, Some(life_boon()), o.order[o.next]) {
                return;
            }
            self.ousting.as_mut().expect("ousting").playing = false;
        }
        loop {
            let st = &self.state;
            let o = self.ousting.as_mut().expect("ousting");
            o.batches[0].retain(|p| st.seats[p.seat.index()].oust_pending);
            if !o.batches[0].is_empty() {
                while self.ousting.as_ref().is_some_and(|o| {
                    o.next < o.order.len() && self.boon_plays(o.order[o.next]).is_empty()
                }) {
                    self.ousting.as_mut().expect("ousting").next += 1;
                }
                let o = self.ousting.as_ref().expect("ousting");
                if o.next < o.order.len() {
                    return;
                }
                let seats: Vec<SeatId> = o.batches[0].iter().map(|p| p.seat).collect();
                for &s in &seats {
                    self.state.seats[s.index()].oust_pending = false;
                }
                let pools =
                    |e: &Engine| -> Vec<i32> { e.state.seats.iter().map(|s| s.pool).collect() };
                let before = pools(self);
                self.oust_now(&seats);
                let after = pools(self);
                for p in self.ousting.as_mut().expect("ousting").batches[1..]
                    .iter_mut()
                    .flatten()
                {
                    p.at += after[p.seat.index()] - before[p.seat.index()];
                }
                self.after_deferred_oust();
            }
            let over = matches!(self.flow, Flow::GameOver);
            let o = self.ousting.as_mut().expect("ousting");
            o.batches.remove(0);
            // Kept in by an earlier oust's pool: not ousted after all.
            let seats = &mut self.state.seats;
            while let Some(next) = o.batches.first_mut() {
                next.retain(|p| {
                    let seat = &mut seats[p.seat.index()];
                    let spared = over || seat.ousted || p.at > 0 && seat.pool > 0;
                    if spared {
                        seat.oust_pending = false;
                    }
                    !spared
                });
                if !next.is_empty() {
                    break;
                }
                o.batches.remove(0);
            }
            let Some(next) = o.batches.first() else {
                self.ousting = None;
                return;
            };
            let batch: Vec<SeatId> = next.iter().map(|p| p.seat).collect();
            let order = self.boon_holders(&batch);
            let o = self.ousting.as_mut().expect("ousting");
            o.order = order;
            o.next = 0;
        }
    }

    /// The Boon holder's decision: pass, or play a Boon on a Methuselah of
    /// the batch.
    pub(crate) fn ousting_request(&self) -> Option<DecisionRequest> {
        let o = self.ousting.as_ref().filter(|o| !o.playing)?;
        let h = *o.order.get(o.next)?;
        let mut options = vec![Choice::Pass];
        options.extend(self.boon_plays(h));
        Some(DecisionRequest::new(
            h,
            WindowLabel::ResolutionChoice,
            options,
        ))
    }

    pub(crate) fn ousting_apply(&mut self, choice: &Choice) {
        let o = self.ousting.as_mut().expect("ousting");
        let h = o.order[o.next];
        match *choice {
            Choice::Pass => o.next += 1,
            Choice::Play {
                card,
                mode,
                actor,
                target,
            } => {
                o.playing = true;
                let resume = self.flow.clone();
                self.announce_play(h, card, mode, actor, target, resume);
            }
            other => unreachable!("illegal ousting choice {other:?}"),
        }
    }

    /// One event losing several minions (a strike resolution, a torpor
    /// with its riders): the pool burns its triggers owe are collected and
    /// ordered once, as it ends.
    pub(crate) fn lost_together(&mut self, event: impl FnOnce(&mut Self)) {
        let outer = std::mem::replace(&mut self.burns_collected, true);
        event(self);
        self.burns_collected = outer;
        if !outer {
            self.order_burns();
        }
    }

    /// A pool burn owed by a trigger of a minion lost, by seat: with those
    /// of the same event (`lost_together`), or of an order still asked.
    pub(crate) fn burn_separately(&mut self, loss: Vec<i32>, by: Option<Source>) {
        match self.burn_order.as_mut() {
            Some(b) => {
                for (owed, n) in b.loss.iter_mut().zip(&loss) {
                    *owed += n;
                }
            }
            None => {
                self.burn_order = Some(BurnOrder {
                    loss,
                    picked: Vec::new(),
                    by,
                });
            }
        }
        if !self.burns_collected {
            self.order_burns();
        }
    }

    /// The burns collected: their order is asked (`Interrupt::BurnOrder`)
    /// only when two Methuselahs or more would burn their last pool, no
    /// other order showing; else one batch, here.
    fn order_burns(&mut self) {
        let asked = self
            .interrupts
            .iter()
            .any(|i| matches!(i, crate::draws::Interrupt::BurnOrder { .. }));
        if self.burn_order.is_none() || asked {
            return;
        }
        if self.burn_order_left().len() < 2 {
            self.burn_in_order(false);
            return;
        }
        let seat = self.acting_seat();
        self.interrupts
            .push(crate::draws::Interrupt::BurnOrder { seat });
    }

    /// The Methuselahs whose place in the order is still to set: those
    /// who would burn their last pool, while two or more are left.
    pub(crate) fn burn_order_left(&self) -> Vec<SeatId> {
        let b = self.burn_order.as_ref().expect("burn order");
        (0..b.loss.len())
            .map(|i| SeatId(i as u8))
            .filter(|s| {
                let seat = &self.state.seats[s.index()];
                let n = b.loss[s.index()];
                n > 0 && !b.picked.contains(s) && !seat.ousted && seat.pool <= n
            })
            .collect()
    }

    /// Burn the pool owed: those picked one at a time, each ousted before
    /// the next burns, then the rest together; nothing past a game over.
    /// `late`, after the event that owed them: what an oust interrupts is
    /// settled here, no caller left to.
    pub(crate) fn burn_in_order(&mut self, late: bool) {
        let b = self.burn_order.take().expect("burn order");
        let living = self.state.living();
        let mut rest = b.loss;
        let mut turns: Vec<Vec<i32>> = Vec::new();
        for s in b.picked {
            let mut one = vec![0; rest.len()];
            one[s.index()] = std::mem::take(&mut rest[s.index()]);
            turns.push(one);
        }
        turns.push(rest);
        for loss in turns {
            if self.state.living() < 2 {
                break;
            }
            match b.by {
                Some(by) => self.caused(Cause::Effect(by), |e| e.burn_pool_batch(&loss)),
                None => self.burn_pool_batch(&loss),
            }
        }
        if late && self.state.living() < living {
            self.after_deferred_oust();
        }
    }

    /// Settle what a deferred oust interrupts, as its callers would have:
    /// game over, combat, the turn passing, the action's order.
    pub(crate) fn after_deferred_oust(&mut self) {
        if matches!(self.flow, Flow::GameOver) {
            return;
        }
        if self.combat.is_some() && self.combat_ousts() {
            return;
        }
        if self.state.seats[self.state.active.index()].ousted {
            self.next_turn();
            return;
        }
        let st = &self.state;
        if let Some(a) = self.action.as_mut() {
            a.order.retain(|s| !st.seats[s.index()].ousted);
        }
    }

    /// Life Boon resolving: `x` pool from its player to the Methuselah it
    /// saves, if still about to be ousted; the card remembers them.
    pub(crate) fn give_boon_pool(&mut self, seat: SeatId, target: SeatId, x: u8, card: CardId) {
        if !self.state.seats[target.index()].oust_pending {
            return;
        }
        self.caused(Cause::Effect(Source::Card(card)), |e| {
            e.state.seats[seat.index()].lose_pool(x as i32);
            e.state.gain_pool(target, x as i32);
        });
        self.state.seats[target.index()].oust_pending = false;
        self.state.cards[card.index()].against = crate::ids::SeatSet::single(target);
    }

    /// `seat` gains `n` half VPs, or its Life Boons' players do, oldest
    /// first, each until 1 VP (then burned); an ousted Boon player gets none.
    pub(crate) fn gain_half_vps(&mut self, seat: SeatId, n: u32) {
        let boon = life_boon();
        let mut left = n;
        while left > 0 {
            let st = &self.state;
            let Some(id) = (0..st.cards.len()).map(|i| CardId(i as u32)).find(|&id| {
                let c = &st.cards[id.index()];
                c.present() && c.card == boon && c.against.contains(seat) && {
                    let by = st.card_controller(id);
                    by != seat && !st.seats[by.index()].ousted
                }
            }) else {
                break;
            };
            let by = self.state.card_controller(id);
            let c = &mut self.state.cards[id.index()];
            let take = left.min(2u32.saturating_sub(c.counters as u32));
            c.counters += take as u8;
            let full = c.counters >= 2;
            self.state.seats[by.index()].half_vps += take;
            left -= take;
            if full {
                self.burn_card_in_play(id);
            }
        }
        self.state.seats[seat.index()].half_vps += left;
    }
}
