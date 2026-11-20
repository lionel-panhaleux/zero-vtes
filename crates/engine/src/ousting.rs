//! The decision point before an oust (docs/engine.md §6.17): Life Boon,
//! Extremis Boon and any other card played as a pool reaches 0, right
//! before the oust (ANK 20220210; rules-qa 126.4).

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

/// Is a play by `seat` still under way somewhere in `flow`?
pub(crate) fn in_flight(flow: &Flow, seat: SeatId) -> bool {
    match flow {
        Flow::PlayTerms { ctx, resume } | Flow::CostTerms { ctx, resume, .. } => {
            ctx.seat == seat || in_flight(resume, seat)
        }
        Flow::AsPlayed { stack, resume } => {
            stack.iter().any(|p| p.ctx.seat == seat) || in_flight(resume, seat)
        }
        _ => false,
    }
}

impl Engine {
    /// Should the oust of `seats` wait on a card playable at it (any card
    /// with a `Window::Ousting` mode, held by a Methuselah other than the
    /// active one, inside the batch too: `boon_holders`)? Behind an oust
    /// already waiting it waits its turn, a batch of its own.
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
            let order = self.boon_holders();
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

    /// Who may play a card at an oust (`Window::Ousting`): its holders
    /// still in the game, the batch's too (`boon_plays`), clockwise from
    /// the active Methuselah, never the active one.
    fn boon_holders(&self) -> Vec<SeatId> {
        let st = &self.state;
        let at_oust = |c: u16| {
            crate::cards::lookup(c)
                .is_some_and(|d| d.modes.iter().any(|m| m.windows.contains(&Window::Ousting)))
        };
        st.clockwise_from(st.active)
            .into_iter()
            .filter(|&h| {
                h != st.active
                    && !st.seats[h.index()].ousted
                    && self.virtual_hand(h).iter().any(|&(c, _)| at_oust(c))
            })
            .collect()
    }

    /// The Boon plays `h` may make now, each on a Methuselah of the front
    /// batch: one of the batch saves none but themself (Extremis Boon).
    fn boon_plays(&self, h: SeatId) -> Vec<Choice> {
        let front = &self.ousting.as_ref().expect("ousting").batches[0];
        let inside = front.iter().any(|p| p.seat == h);
        self.play_options(h, &[Window::Ousting], None)
            .into_iter()
            .filter(|o| {
                matches!(o, Choice::Play { target: Target::Seat(t), .. }
                    if front.iter().any(|p| p.seat == *t) && (!inside || *t == h))
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
            // Extremis Boon's bids part of its play.
            let bids = self
                .interrupts
                .iter()
                .any(|i| matches!(i, crate::draws::Interrupt::BoonAuction { .. }));
            if bids || in_flight(&self.flow, o.order[o.next]) {
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
            if o.batches.is_empty() {
                self.ousting = None;
                return;
            }
            let order = self.boon_holders();
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

    /// Extremis Boon `card` played by `target`: the other Methuselahs bid,
    /// clockwise from its left (rules-qa 126.75).
    pub(crate) fn start_boon_auction(&mut self, target: SeatId, card: CardId) {
        match self.next_boon_bidder(target, target, None, crate::ids::SeatSet::default()) {
            // Asked before what the oust holds back (`asking_queued`).
            Some(seat) => self.interrupts.insert(
                0,
                crate::draws::Interrupt::BoonAuction {
                    seat,
                    target,
                    card,
                    high: None,
                    out: crate::ids::SeatSet::default(),
                },
            ),
            None => self.burn_card_in_play(card),
        }
    }

    /// The next bidder after `after`, clockwise: a living Methuselah other
    /// than the target and the high bidder who has not passed; none, the
    /// auction is over.
    pub(crate) fn next_boon_bidder(
        &self,
        target: SeatId,
        after: SeatId,
        high: Option<(SeatId, u8)>,
        out: crate::ids::SeatSet,
    ) -> Option<SeatId> {
        let st = &self.state;
        st.clockwise_from(st.prey(after)).into_iter().find(|&s| {
            s != target
                && !st.seats[s.index()].ousted
                && !out.contains(s)
                && high.is_none_or(|(h, _)| h != s)
        })
    }

    /// The bids `seat` may make: at least what keeps the target in the game
    /// (ANK 20190827), a raise of 1 over the high bid, up to their pool.
    pub(crate) fn boon_bids(
        &self,
        seat: SeatId,
        target: SeatId,
        high: Option<(SeatId, u8)>,
    ) -> Vec<u8> {
        let keep = 1 - self.state.seats[target.index()].pool;
        let low = high.map_or(keep, |(_, b)| (b as i32 + 1).max(keep)).max(1);
        let pool = self.state.seats[seat.index()].pool.min(u8::MAX as i32);
        (low..=pool).map(|b| b as u8).collect()
    }

    /// The auction over: the high bid moves to the target, kept in the
    /// game, the card against its bidder (LSJ 20031121-1); none, the card
    /// burns and the oust goes on (LSJ 20040601).
    pub(crate) fn settle_boon_auction(
        &mut self,
        target: SeatId,
        card: CardId,
        high: Option<(SeatId, u8)>,
    ) {
        match high {
            Some((by, b)) if self.state.seats[target.index()].oust_pending => {
                self.caused(Cause::Effect(Source::Card(card)), |e| {
                    e.state.seats[by.index()].lose_pool(b as i32);
                    e.state.gain_pool(target, b as i32);
                });
                self.state.seats[target.index()].oust_pending = false;
                self.state.cards[card.index()].against = crate::ids::SeatSet::single(by);
                if self.state.seats[by.index()].pool <= 0 {
                    self.oust(by);
                }
            }
            _ => self.burn_card_in_play(card),
        }
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
            let turn = self.state.turn;
            let by_seat = &mut self.state.seats[by.index()];
            by_seat.half_vps += take;
            if take > 0 {
                by_seat.vps_received.0.push((turn, take));
            }
            left -= take;
            if full {
                self.burn_card_in_play(id);
            }
        }
        let turn = self.state.turn;
        let s = &mut self.state.seats[seat.index()];
        s.half_vps += left;
        if left > 0 {
            s.vps_received.0.push((turn, left));
        }
    }
}
