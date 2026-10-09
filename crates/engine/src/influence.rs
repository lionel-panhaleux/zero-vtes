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
    /// The transfers an ability use gains ("lock to get +1 transfer").
    fn transfer_gain(&self, choice: &Choice) -> u8 {
        let Choice::UseAbility {
            source, ability, ..
        } = choice
        else {
            return 0;
        };
        source
            .mode(&self.state, *ability)
            .effects
            .iter()
            .map(|e| match e {
                crate::cards::Effect::GainTransfers(n) => *n,
                _ => 0,
            })
            .sum()
    }

    /// The transfers an ability use costs ("you can use 1 transfer to …").
    fn transfer_cost(&self, choice: &Choice) -> u8 {
        let Choice::UseAbility {
            source, ability, ..
        } = choice
        else {
            return 0;
        };
        source
            .mode(&self.state, *ability)
            .effects
            .iter()
            .map(|e| match e {
                crate::cards::Effect::SpendTransfers(n) => *n,
                _ => 0,
            })
            .sum()
    }

    pub(crate) fn influence_request(&self, left: u8) -> DecisionRequest {
        let st = &self.state;
        let s = st.active;
        let seat = &st.seats[s.index()];
        let mut options = vec![Choice::Pass];
        for (i, slot) in seat.uncontrolled.iter().enumerate() {
            let i = i as u8;
            let (capacity, _) = st.slot_vampire(s, i as usize);
            // Any amount, over capacity too (rules-qa 28.3).
            if left >= 1 && seat.pool > 0 {
                options.push(Choice::Transfer { slot: i });
            }
            if left >= 2 && slot.counters > 0 {
                options.push(Choice::Withdraw { slot: i });
            }
            // No voluntary self-contest: a second copy of a vampire you
            // already have in play cannot be moved out — but Anarch
            // Convert, removed as he enters play before any contest (the
            // copy in play is a vampire he can make Anarch).
            if slot.counters >= capacity
                && (slot.card == crate::state::CREATED
                    || !st.controls_copy(s, slot.card)
                    || is_anarch_convert(slot.card))
            {
                options.push(Choice::InfluenceOut { slot: i });
            }
        }
        if left >= 4 && seat.pool > 0 && !seat.crypt.is_empty() {
            options.push(Choice::DrawCrypt);
        }
        options.extend(self.ability_options(s, Window::AnyImpulse));
        options.extend(self.ability_options(s, Window::InfluencePhaseOwn));
        // Abilities paid in transfers: while enough are left.
        options.extend(
            self.ability_options(s, Window::Transfers)
                .into_iter()
                .filter(|o| self.transfer_cost(o) <= left),
        );
        // Frontal Assault's burn: in the order the player chooses, before
        // they pass (rules-qa 28.9).
        let due = self.influence_burns_due();
        if !due.is_empty() {
            options.retain(|o| *o != Choice::Pass);
            options.extend(due.iter().map(|c| Choice::ResolveRider { card: c.0 }));
        }
        DecisionRequest::new(s, WindowLabel::InfluenceTransfers, options)
    }

    pub(crate) fn influence_apply(&mut self, left: u8, choice: &Choice) {
        let s = self.state.active;
        match choice {
            Choice::UseAbility {
                source,
                ability,
                actor,
                target,
            } => {
                let left = left - self.transfer_cost(choice) + self.transfer_gain(choice);
                if !self.use_ability(*source, *ability, *actor, *target, Flow::Influence { left }) {
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
                let forced =
                    out.card != crate::state::CREATED && self.state.controls_copy(s, out.card);
                // Counters become blood; excess over capacity drains off —
                // a returning vampire's capacity counting the cards on it
                // again (advanced rules 5.1.1).
                let m = match out.banished.map(|b| (b.minion, b.cards)) {
                    // A banished vampire returns as the entry it left — its
                    // history (titles lost, contests) and the cards on it,
                    // face up and unlocked (rules-qa 5.7; LSJ 20060213).
                    Some((m, on)) => {
                        let v = &mut self.state.minions[m.index()];
                        v.in_play = true;
                        v.locked = false;
                        v.in_torpor = false;
                        for c in on {
                            let c = &mut self.state.cards[c.index()];
                            c.in_play = true;
                            c.locked = false;
                        }
                        let capacity = self.state.capacity_of(m).unwrap_or(1);
                        self.state.minions[m.index()].blood = out.counters.min(capacity);
                        m
                    }
                    None => {
                        let capacity = CRYPT[out.card as usize].capacity;
                        self.add_minion(s, out.card, capacity, false)
                    }
                };
                // An Anarch entering play: counted first, the Convert's
                // ability next (Hackerspace, ANK 20210410).
                if self.state.sect_of(m) == Some(crate::crypt::Sect::Anarch) {
                    self.anarch_arrives(s);
                }
                // Anarch Convert: his entry ability before any contest.
                if is_anarch_convert(out.card) {
                    self.interrupts
                        .push(crate::draws::Interrupt::AnarchConvert {
                            seat: s,
                            convert: m,
                            forced,
                            chosen: None,
                        });
                } else {
                    self.stage_contests(m);
                }
                self.flow = Flow::Influence { left };
                // Twilight Camp: a non-titled non-Anarch vampire put in play
                // from the uncontrolled region may be made Anarch.
                if self.state.title_of(m) == crate::crypt::Title::None
                    && self.state.sect_of(m) != Some(crate::crypt::Sect::Anarch)
                {
                    for ci in 0..self.state.cards.len() {
                        let id = crate::state::CardId(ci as u32);
                        let c = &self.state.cards[ci];
                        if c.present()
                            && c.counters > 0
                            && self.state.card_controller(id) == s
                            && crate::cards::lookup(c.card).is_some_and(|d| {
                                d.riders
                                    .iter()
                                    .any(|r| r.hook == crate::cards::Hook::OwnVampireInfluenced)
                            })
                        {
                            self.interrupts.push(crate::draws::Interrupt::MakeAnarch {
                                seat: s,
                                card: id,
                                minion: m,
                            });
                        }
                    }
                }
            }
            Choice::ResolveRider { card } => {
                self.flow = Flow::Influence { left };
                self.influence_burn(crate::state::CardId(*card));
            }
            Choice::Pass => self.open_phase_rotation(Flow::Influence { left }),
            other => unreachable!("illegal influence choice {other:?}"),
        }
    }
}

/// Anarch Convert's crypt card ("as the convert enters play, you can remove
/// him from the game …").
fn is_anarch_convert(card: u16) -> bool {
    card != crate::state::CREATED && CRYPT[card as usize].name == "Anarch Convert"
}
