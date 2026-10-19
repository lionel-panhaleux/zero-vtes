//! The decision point before an oust (docs/engine.md §6.17): an oust waits
//! while a Methuselah holding Life Boon could still save the batch.

use crate::cards::Window;
use crate::decision::{Choice, DecisionRequest, WindowLabel};
use crate::game::{Engine, Flow};
use crate::ids::SeatId;
use crate::library::LIBRARY;
use crate::notes::{Cause, Source};
use crate::state::CardId;

/// Methuselahs about to be ousted, and the Methuselahs who may save them.
#[derive(Clone, Debug)]
pub struct Ousting {
    /// The batch, ousted together once no Boon saves them.
    pub seats: Vec<SeatId>,
    /// Boon holders, clockwise from the active Methuselah; `next` the one
    /// deciding.
    pub order: Vec<SeatId>,
    pub next: usize,
    /// A Boon of `order[next]` is being played (its terms, its as-played
    /// window): the play's own decisions come first.
    pub playing: bool,
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

/// Is a play of `card` by `seat` still under way somewhere in `flow`?
fn in_flight(flow: &Flow, card: u16, seat: SeatId) -> bool {
    match flow {
        Flow::PlayTerms { ctx, resume } | Flow::CostTerms { ctx, resume, .. } => {
            (ctx.card == card && ctx.seat == seat) || in_flight(resume, card, seat)
        }
        Flow::AsPlayed { stack, resume } => {
            stack
                .iter()
                .any(|p| p.ctx.card == card && p.ctx.seat == seat)
                || in_flight(resume, card, seat)
        }
        _ => false,
    }
}

impl Engine {
    /// Should the oust of `seats` wait on a Life Boon (one held outside the
    /// batch, not by the active Methuselah)? A pending oust takes them in.
    pub(crate) fn defer_oust(&mut self, seats: &[SeatId]) -> bool {
        if let Some(o) = self.ousting.as_mut() {
            for &s in seats {
                if !o.seats.contains(&s) {
                    o.seats.push(s);
                    self.state.seats[s.index()].oust_pending = true;
                }
            }
            return true;
        }
        let boon = life_boon();
        let st = &self.state;
        let order: Vec<SeatId> = st
            .clockwise_from(st.active)
            .into_iter()
            .filter(|&h| {
                h != st.active
                    && !seats.contains(&h)
                    && !st.seats[h.index()].ousted
                    && self.virtual_hand(h).iter().any(|&(c, _)| c == boon)
            })
            .collect();
        if order.is_empty() {
            return false;
        }
        for &s in seats {
            self.state.seats[s.index()].oust_pending = true;
        }
        self.ousting = Some(Ousting {
            seats: seats.to_vec(),
            order,
            next: 0,
            playing: false,
        });
        true
    }

    /// The Boon plays `h` may make now, each on a Methuselah of the batch.
    fn boon_plays(&self, h: SeatId) -> Vec<Choice> {
        let boon = life_boon();
        let hand = self.virtual_hand(h);
        self.play_options(h, &[Window::Ousting], None)
            .into_iter()
            .filter(|o| matches!(o, Choice::Play { card, .. } if hand[*card as usize].0 == boon))
            .collect()
    }

    /// Before any decision: end a finished Boon play's wait, drop the
    /// saved, and oust the rest once no one is left to ask.
    pub(crate) fn settle_ousting(&mut self) {
        let Some(o) = self.ousting.as_ref() else {
            return;
        };
        if o.playing {
            if in_flight(&self.flow, life_boon(), o.order[o.next]) {
                return;
            }
            self.ousting.as_mut().expect("ousting").playing = false;
        }
        let st = &self.state;
        let mut o = self.ousting.take().expect("ousting");
        o.seats.retain(|s| st.seats[s.index()].oust_pending);
        if o.seats.is_empty() {
            return;
        }
        while o.next < o.order.len() && self.boon_plays(o.order[o.next]).is_empty() {
            o.next += 1;
        }
        if o.next < o.order.len() {
            self.ousting = Some(o);
            return;
        }
        for &s in &o.seats {
            self.state.seats[s.index()].oust_pending = false;
        }
        self.oust_now(&o.seats);
        self.after_deferred_oust();
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

    /// Settle what a deferred oust interrupts, as its callers would have:
    /// game over, combat, the turn passing, the action's order.
    fn after_deferred_oust(&mut self) {
        if matches!(self.flow, Flow::GameOver) {
            return;
        }
        if self.combat.is_some() {
            self.combat_ousts();
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
            e.state.seats[seat.index()].pool -= x as i32;
            e.state.seats[target.index()].pool += x as i32;
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
