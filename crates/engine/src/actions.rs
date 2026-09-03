//! Minion phase: the card-free rulebook actions, the block loop (rules-qa
//! 1.3), action resolution, and P0 combat.

use crate::decision::{ActionKind, Choice};
use crate::game::{BlockCtx, BloodHuntCtx, Engine, Flow};
use crate::ids::MinionId;
use crate::window::{OrderedWindow, PollingWindow};

impl Engine {
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
        options
    }

    /// Announce an action: lock the acting minion, compute stealth and the
    /// seats entitled to block (rulebook "Who May Attempt to Block"), open
    /// the block loop.
    pub(crate) fn declare_action(
        &mut self,
        minion: MinionId,
        action: ActionKind,
        target: Option<MinionId>,
    ) {
        self.state.minions[minion.index()].locked = true;
        let acting_seat = self.state.active;
        let own_target = target.map(|t| self.state.minions[t.index()].owner == acting_seat);
        let (stealth, mut order) = match action {
            // Directed at the prey, 0 stealth: only the defender may block.
            ActionKind::Bleed => (0, vec![self.state.prey(acting_seat)]),
            // Undirected, +1 stealth: prey then predator.
            ActionKind::Hunt | ActionKind::LeaveTorpor => (
                1,
                vec![
                    self.state.prey(acting_seat),
                    self.state.predator(acting_seat),
                ],
            ),
            ActionKind::Rescue | ActionKind::Diablerie => {
                let t = target.expect("torpor target");
                if own_target == Some(true) {
                    (
                        1,
                        vec![
                            self.state.prey(acting_seat),
                            self.state.predator(acting_seat),
                        ],
                    )
                } else {
                    // Directed at the target's controller, 0 stealth.
                    (0, vec![self.state.minions[t.index()].owner])
                }
            }
        };
        order.dedup();
        self.flow = Flow::Block(BlockCtx {
            acting: minion,
            action,
            target,
            stealth,
            window: OrderedWindow::new(order),
            attempt: None,
            declined: Vec::new(),
        });
    }

    pub(crate) fn resolve_action(&mut self, ctx: BlockCtx) {
        let intercept = 0u8;
        match ctx.attempt {
            Some(blocker) if intercept >= ctx.stealth => {
                // Block resolution: the blocker locks and combat begins.
                self.state.minions[blocker.index()].locked = true;
                if ctx.action == ActionKind::Bleed {
                    // NRA triggers on resolution, blocked included (rules-qa 1.4).
                    self.state.minions[ctx.acting.index()].bled_this_turn = true;
                }
                match ctx.action {
                    // Rulebook "Leave Torpor": a blocked leave-torpor has no
                    // combat; a vampire blocker may diablerise instead. Never
                    // reached in P0 (+1 stealth, no intercept source) — P1
                    // implements the blocker's diablerie opportunity here.
                    ActionKind::LeaveTorpor => unreachable!("leave torpor blocked in P0"),
                    _ => self.combat(ctx.acting, blocker),
                }
                self.flow = Flow::MinionPhase;
            }
            // A pending attempt that fails at closure: the action proceeds.
            _ => self.succeed(ctx),
        }
    }

    fn succeed(&mut self, ctx: BlockCtx) {
        self.flow = Flow::MinionPhase;
        match ctx.action {
            ActionKind::Bleed => {
                let acting_seat = self.state.active;
                let prey = self.state.prey(acting_seat);
                self.state.minions[ctx.acting.index()].bled_this_turn = true;
                self.state.seats[prey.index()].pool -= 1;
                // Successful bleed of the prey: take the Edge.
                for seat in &mut self.state.seats {
                    seat.has_edge = false;
                }
                self.state.seats[acting_seat.index()].has_edge = true;
                if self.state.seats[prey.index()].pool <= 0 {
                    self.oust(prey);
                }
            }
            ActionKind::Hunt => {
                let m = &mut self.state.minions[ctx.acting.index()];
                m.blood = (m.blood + 1).min(m.capacity());
            }
            ActionKind::LeaveTorpor => {
                // Cost is paid at resolution only (a blocked action pays
                // nothing).
                let m = &mut self.state.minions[ctx.acting.index()];
                m.blood -= 2;
                m.in_torpor = false;
            }
            ActionKind::Rescue => {
                self.flow = Flow::RescueCost {
                    acting: ctx.acting,
                    target: ctx.target.expect("rescue target"),
                };
            }
            ActionKind::Diablerie => {
                let victim = ctx.target.expect("diablerie target");
                // Rulebook "Diablerie" steps 1–3 as a single unit: blood to
                // the diablerist (excess drains), victim burned. Equipment
                // and the advanced rule 4 (Discipline for an elder victim)
                // need cards — P1.
                let taken = self.state.minions[victim.index()].blood;
                self.state.minions[victim.index()].blood = 0;
                self.state.minions[victim.index()].in_play = false;
                let d = &mut self.state.minions[ctx.acting.index()];
                d.blood = (d.blood + taken).min(d.capacity());
                // The blood hunt referendum follows immediately; rotation
                // starts at the acting Methuselah, clockwise (DESIGN §4.2 —
                // a modeling call, owner-approved 2026-09-02).
                let order = self.state.clockwise_from(self.state.active);
                self.flow = Flow::BloodHunt(BloodHuntCtx {
                    diablerist: ctx.acting,
                    window: PollingWindow::new(order),
                    spent: Vec::new(),
                    edge_spent: false,
                    for_votes: 0,
                    against_votes: 0,
                });
            }
        }
    }

    /// P0 combat: close range, one round, simultaneous hand strikes for 1.
    /// Both combatants are ready (torpor vampires never enter combat).
    fn combat(&mut self, a: MinionId, b: MinionId) {
        debug_assert!(self.state.minions[a.index()].ready());
        debug_assert!(self.state.minions[b.index()].ready());
        self.damage(b, 1);
        self.damage(a, 1);
    }

    /// Damage a vampire: burn blood; a vampire that cannot mend goes to
    /// torpor, and one already in torpor is burned.
    fn damage(&mut self, m: MinionId, n: u8) {
        let minion = &mut self.state.minions[m.index()];
        for _ in 0..n {
            if minion.blood > 0 {
                minion.blood -= 1;
            } else if minion.in_torpor {
                minion.in_play = false;
            } else {
                minion.in_torpor = true;
            }
        }
    }
}
