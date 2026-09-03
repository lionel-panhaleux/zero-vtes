//! Referendums. P0/P1 slice: the blood hunt (rulebook "The Blood Hunt") on
//! the polling primitive — sources per rules-qa 1.1 (ready vampires with
//! votes, the Edge), each committing once, all its votes one way.

use crate::decision::{Choice, DecisionRequest, WindowLabel};
use crate::game::{BloodHuntCtx, Engine, Flow};

impl Engine {
    pub(crate) fn blood_hunt_request(&self, ctx: &BloodHuntCtx) -> Option<DecisionRequest> {
        let st = &self.state;
        let seat = ctx.window.current()?;
        let mut options = vec![Choice::Pass];
        for id in st.minions_of(seat) {
            // Torpor vampires must abstain; locked ready ones may vote;
            // contested titles carry no votes.
            if st.minions[id.index()].votes() > 0 && !ctx.spent.contains(&id) {
                options.push(Choice::CastVotes {
                    minion: id,
                    favor: true,
                });
                options.push(Choice::CastVotes {
                    minion: id,
                    favor: false,
                });
            }
        }
        if st.seats[seat.index()].has_edge && !ctx.edge_spent {
            options.push(Choice::CastEdge { favor: true });
            options.push(Choice::CastEdge { favor: false });
        }
        Some(DecisionRequest {
            seat,
            window: WindowLabel::BloodHuntPolling,
            options,
        })
    }

    pub(crate) fn blood_hunt_apply(&mut self, mut ctx: BloodHuntCtx, choice: &Choice) {
        match choice {
            Choice::Pass => ctx.window.pass(),
            Choice::CastVotes { minion, favor } => {
                let n = self.state.minions[minion.index()].votes() as u32;
                if *favor {
                    ctx.for_votes += n;
                } else {
                    ctx.against_votes += n;
                }
                ctx.spent.push(*minion);
                ctx.window.commit();
            }
            Choice::CastEdge { favor } => {
                let seat = ctx.window.current().expect("commit on closed window");
                self.state.seats[seat.index()].has_edge = false;
                if *favor {
                    ctx.for_votes += 1;
                } else {
                    ctx.against_votes += 1;
                }
                ctx.edge_spent = true;
                ctx.window.commit();
            }
            other => unreachable!("illegal polling choice {other:?}"),
        }
        self.flow = Flow::BloodHunt(ctx);
    }

    pub(crate) fn resolve_blood_hunt(&mut self, ctx: BloodHuntCtx) {
        // Passes only with strictly more votes for than against; ties fail.
        if ctx.for_votes > ctx.against_votes {
            self.state.minions[ctx.diablerist.index()].in_play = false;
        }
        self.flow = Flow::MinionPhase;
    }
}
