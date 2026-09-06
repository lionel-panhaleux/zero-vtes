//! Influence phase (rulebook "Influence Phase"): transfers, withdrawals,
//! the 4-transfer + 1-pool crypt draw, and influencing out (no voluntary
//! self-contest).

use crate::cards::Window;
use crate::crypt::CRYPT;
use crate::decision::{Choice, DecisionRequest, WindowLabel};
use crate::game::{Engine, Flow};
use crate::state::UncontrolledSlot;

/// Transfers granted at the start of an influence phase: 1/2/3 on the first
/// three turns of the game, 4 thereafter (rulebook "Influence Phase").
pub fn transfers_for_turn(turn: u32) -> u8 {
    turn.min(4) as u8
}

impl Engine {
    pub(crate) fn influence_request(&self, left: u8) -> DecisionRequest {
        let st = &self.state;
        let s = st.active;
        let seat = &st.seats[s.index()];
        let mut options = vec![Choice::Pass];
        for (i, slot) in seat.uncontrolled.iter().enumerate() {
            let i = i as u8;
            let capacity = CRYPT[slot.card as usize].capacity;
            if left >= 1 && seat.pool > 0 && slot.counters < capacity {
                options.push(Choice::Transfer { slot: i });
            }
            if left >= 2 && slot.counters > 0 {
                options.push(Choice::Withdraw { slot: i });
            }
            // No voluntary self-contest: a second copy of a vampire you
            // already have in play cannot be moved out.
            if slot.counters >= capacity && !st.controls_copy(s, slot.card) {
                options.push(Choice::InfluenceOut { slot: i });
            }
        }
        if left >= 4 && seat.pool > 0 && !seat.crypt.is_empty() {
            options.push(Choice::DrawCrypt);
        }
        options.extend(self.ability_options(s, Window::AnyImpulse));
        DecisionRequest {
            seat: s,
            window: WindowLabel::InfluenceTransfers,
            options,
        }
    }

    pub(crate) fn influence_apply(&mut self, left: u8, choice: &Choice) {
        let s = self.state.active;
        match choice {
            Choice::UseAbility {
                card,
                ability,
                actor,
                target,
            } => {
                if !self.use_ability(*card, *ability, *actor, *target, Flow::Influence { left }) {
                    self.flow = Flow::Influence { left };
                }
            }
            Choice::Transfer { slot } => {
                let seat = &mut self.state.seats[s.index()];
                seat.pool -= 1;
                seat.uncontrolled[*slot as usize].counters += 1;
                self.flow = Flow::Influence { left: left - 1 };
                if seat.pool == 0 {
                    self.oust_and_pass(s);
                }
            }
            Choice::Withdraw { slot } => {
                let seat = &mut self.state.seats[s.index()];
                seat.pool += 1;
                seat.uncontrolled[*slot as usize].counters -= 1;
                self.flow = Flow::Influence { left: left - 2 };
            }
            Choice::DrawCrypt => {
                let seat = &mut self.state.seats[s.index()];
                seat.pool -= 1;
                let card = seat.crypt.pop().expect("draw from empty crypt");
                seat.uncontrolled.push(UncontrolledSlot {
                    card,
                    counters: 0,
                    banished: None,
                });
                self.flow = Flow::Influence { left: left - 4 };
                if seat.pool == 0 {
                    self.oust_and_pass(s);
                }
            }
            Choice::InfluenceOut { slot } => {
                let out = self.state.seats[s.index()]
                    .uncontrolled
                    .remove(*slot as usize);
                // Counters become blood; excess over capacity drains off.
                let capacity = CRYPT[out.card as usize].capacity;
                let m = match out.banished {
                    // A banished vampire returns as the entry it left — its
                    // history (titles lost, contests) and the cards on it,
                    // face up and unlocked (rules-qa 5.7; LSJ 20060213).
                    Some(m) => {
                        let v = &mut self.state.minions[m.index()];
                        v.in_play = true;
                        v.blood = capacity;
                        v.locked = false;
                        v.in_torpor = false;
                        for c in &mut self.state.cards {
                            if c.host == Some(m) {
                                c.in_play = true;
                                c.locked = false;
                            }
                        }
                        m
                    }
                    None => self.add_minion(s, out.card, capacity, false),
                };
                self.stage_contests(m);
                self.flow = Flow::Influence { left };
            }
            Choice::Pass => self.end_influence(),
            other => unreachable!("illegal influence choice {other:?}"),
        }
    }
}
