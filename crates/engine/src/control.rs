//! Moments a card from hand may answer before they happen (docs/engine.md
//! §6.17).

use crate::cards::Window;
use crate::decision::{Choice, DecisionRequest, WindowLabel};
use crate::game::Engine;
use crate::ids::{MinionId, SeatId};

/// How long the control taken lasts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ControlTerm {
    /// Until the end of the turn (Puppet Master [DOM], Gengis).
    UntilEndOfTurn,
    /// Back at the end of its controller's next turn (Ayelea).
    UntilNextTurn,
    /// Back at the taker's next unlock phase.
    UntilUnlock,
    /// For good, a combat it is in ended first (Graverobbing; corruption
    /// counters at capacity: Cave of Apples, Contagion).
    ForGood,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ControlChange {
    pub minion: MinionId,
    pub from: SeatId,
    pub to: SeatId,
    pub term: ControlTerm,
    /// A card of `from`'s is being played (its terms, its as-played
    /// window): the play's own decisions come first.
    pub playing: bool,
    /// A card stopped it.
    pub stopped: bool,
}

/// A passed blood hunt about to burn `minion`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HuntBurn {
    pub minion: MinionId,
    /// The Methuselahs who may answer, `next` the one deciding.
    pub order: Vec<SeatId>,
    pub next: usize,
    pub playing: bool,
    /// A card saved it.
    pub saved: bool,
}

impl Engine {
    /// `to` is about to take control of `m` for `term`: the change waits
    /// while `m`'s controller may still stop it, behind any change already
    /// waiting (rules-qa 111.5).
    pub(crate) fn take_control(&mut self, m: MinionId, to: SeatId, term: ControlTerm) {
        if self.state.control_change.is_some() {
            self.state.control_queue.push((m, to, term));
            return;
        }
        let from = self.state.minions[m.index()].owner;
        if from == to {
            return;
        }
        self.state.control_change = Some(ControlChange {
            minion: m,
            from,
            to,
            term,
            playing: false,
            stopped: false,
        });
        if self.control_plays(from).is_empty() {
            self.state.control_change = None;
            self.change_control(m, to, term);
        }
    }

    /// The next queued change, once the one before settled: one whose
    /// minion left play meanwhile is dropped.
    fn next_control_change(&mut self) {
        while self.state.control_change.is_none() && !self.state.control_queue.is_empty() {
            let (m, to, term) = self.state.control_queue.remove(0);
            if self.state.minions[m.index()].present() {
                self.take_control(m, to, term);
            }
        }
    }

    fn change_control(&mut self, m: MinionId, to: SeatId, term: ControlTerm) {
        match term {
            ControlTerm::ForGood => self.take_for_good_now(m, to),
            _ => {
                self.take_control_now(m, to);
                let v = &mut self.state.minions[m.index()];
                match term {
                    ControlTerm::UntilNextTurn => v.temp_skip = 1,
                    ControlTerm::UntilUnlock => v.temp_until_unlock = true,
                    _ => {}
                }
            }
        }
    }

    /// The plays `seat` may make on the change about to happen.
    fn control_plays(&self, seat: SeatId) -> Vec<Choice> {
        self.play_options(seat, &[Window::ControlAboutToChange], None)
    }

    /// Before any decision: end a finished play's wait; make the change
    /// once no play may stop it, drop it once one did.
    pub(crate) fn settle_control_change(&mut self) {
        let Some(mut c) = self.state.control_change else {
            return;
        };
        if c.playing {
            if crate::ousting::in_flight(&self.flow, None, c.from) {
                return;
            }
            c.playing = false;
            self.state.control_change = Some(c);
        }
        let v = &self.state.minions[c.minion.index()];
        if c.stopped || !v.present() || v.owner != c.from {
            self.state.control_change = None;
        } else if self.control_plays(c.from).is_empty() {
            self.state.control_change = None;
            self.change_control(c.minion, c.to, c.term);
        }
        self.next_control_change();
    }

    /// The losing Methuselah's decision: let it change, or play a card.
    pub(crate) fn control_request(&self) -> Option<DecisionRequest> {
        let c = self.state.control_change.filter(|c| !c.playing)?;
        let plays = self.control_plays(c.from);
        if plays.is_empty() {
            return None;
        }
        let mut options = vec![Choice::Pass];
        options.extend(plays);
        Some(DecisionRequest::new(
            c.from,
            WindowLabel::ResolutionChoice,
            options,
        ))
    }

    pub(crate) fn control_apply(&mut self, choice: &Choice) {
        let mut c = self.state.control_change.expect("a control change");
        match *choice {
            Choice::Pass => {
                self.state.control_change = None;
                self.change_control(c.minion, c.to, c.term);
                self.next_control_change();
            }
            Choice::Play {
                card,
                mode,
                actor,
                target,
            } => {
                c.playing = true;
                self.state.control_change = Some(c);
                let resume = self.flow.clone();
                self.announce_play(c.from, card, mode, actor, target, resume);
            }
            other => unreachable!("illegal control-change choice {other:?}"),
        }
    }

    /// A passed blood hunt burns (or sends to torpor: Genina) `m` — unless
    /// another Methuselah may still answer it: then it waits, the
    /// referendum's end with it. True when it waits.
    pub(crate) fn defer_hunt_burn(&mut self, m: MinionId) -> bool {
        let owner = self.state.minions[m.index()].owner;
        let order: Vec<SeatId> = self
            .state
            .clockwise_from(self.state.active)
            .into_iter()
            .filter(|&s| s != owner && !self.state.seats[s.index()].ousted)
            .collect();
        self.state.hunt_burn = Some(HuntBurn {
            minion: m,
            order,
            next: 0,
            playing: false,
            saved: false,
        });
        if self.hunt_burn_next().is_some() {
            return true;
        }
        self.state.hunt_burn = None;
        false
    }

    /// The first Methuselah from `next` on who may answer the burn, `next`
    /// moved to them.
    fn hunt_burn_next(&mut self) -> Option<SeatId> {
        loop {
            let h = self.state.hunt_burn.as_ref()?;
            let s = *h.order.get(h.next)?;
            if !self.hunt_burn_plays(s).is_empty() {
                return Some(s);
            }
            self.state.hunt_burn.as_mut().expect("hunt burn").next += 1;
        }
    }

    fn hunt_burn_plays(&self, seat: SeatId) -> Vec<Choice> {
        self.play_options(seat, &[Window::BloodHuntWouldBurn], None)
    }

    /// Before any decision: end a finished play's wait; once one saved the
    /// vampire or no one is left to ask, the burn (if any) and the
    /// referendum's end.
    pub(crate) fn settle_hunt_burn(&mut self) {
        let Some(h) = self.state.hunt_burn.as_ref() else {
            return;
        };
        if h.playing {
            let s = h.order[h.next];
            if crate::ousting::in_flight(&self.flow, None, s) {
                return;
            }
            let h = self.state.hunt_burn.as_mut().expect("hunt burn");
            h.playing = false;
            h.next += 1;
        }
        if !self.state.hunt_burn.as_ref().expect("hunt burn").saved
            && self.hunt_burn_next().is_some()
        {
            return;
        }
        let h = self.state.hunt_burn.take().expect("hunt burn");
        if !h.saved {
            self.hunt_burn_now(h.minion);
        }
        self.end_blood_hunt();
    }

    pub(crate) fn hunt_burn_request(&self) -> Option<DecisionRequest> {
        let h = self.state.hunt_burn.as_ref().filter(|h| !h.playing)?;
        let s = *h.order.get(h.next)?;
        let plays = self.hunt_burn_plays(s);
        if plays.is_empty() {
            return None;
        }
        let mut options = vec![Choice::Pass];
        options.extend(plays);
        Some(DecisionRequest::new(
            s,
            WindowLabel::ResolutionChoice,
            options,
        ))
    }

    pub(crate) fn hunt_burn_apply(&mut self, choice: &Choice) {
        let h = self.state.hunt_burn.as_mut().expect("a hunt burn");
        let s = h.order[h.next];
        match *choice {
            Choice::Pass => h.next += 1,
            Choice::Play {
                card,
                mode,
                actor,
                target,
            } => {
                h.playing = true;
                let resume = self.flow.clone();
                self.announce_play(s, card, mode, actor, target, resume);
            }
            other => unreachable!("illegal blood hunt choice {other:?}"),
        }
    }
}
