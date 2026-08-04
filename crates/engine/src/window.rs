use crate::ids::SeatId;

/// Ordered window (rulebook Sequencing; DESIGN §4.2). Offers rotate through
/// `order`, index 0 being the acting Methuselah. Any play returns the impulse
/// to the acting Methuselah and voids every prior pass (rules-qa 1.2,
/// impulse-reset); the window closes only when all seats pass consecutively.
#[derive(Clone, Debug)]
pub struct OrderedWindow {
    order: Vec<SeatId>,
    cursor: usize,
    quiet: usize,
}

impl OrderedWindow {
    pub fn new(order: Vec<SeatId>) -> Self {
        Self {
            order,
            cursor: 0,
            quiet: 0,
        }
    }

    /// Seat currently holding the impulse; `None` once closed.
    pub fn current(&self) -> Option<SeatId> {
        if self.is_closed() {
            None
        } else {
            Some(self.order[self.cursor])
        }
    }

    pub fn is_closed(&self) -> bool {
        self.order.is_empty() || self.quiet >= self.order.len()
    }

    pub fn pass(&mut self) {
        self.quiet += 1;
        if !self.is_closed() {
            self.cursor = (self.cursor + 1) % self.order.len();
        }
    }

    /// The current seat played an effect: impulse returns to the acting
    /// Methuselah and prior passes are voided.
    pub fn play(&mut self) {
        self.quiet = 0;
        self.cursor = 0;
    }
}

/// Unordered commitment window (polling; DESIGN §4.2). No priority holder and
/// no impulse reset: commitments are irrevocable, a pass does not forfeit
/// later rotations, and closure requires a full rotation of consecutive
/// passes — so every commitment reopens a response opportunity for every
/// other seat before tally (rules-qa 1.1: sources commit independently,
/// possibly on different rotations).
#[derive(Clone, Debug)]
pub struct PollingWindow {
    order: Vec<SeatId>,
    cursor: usize,
    quiet: usize,
}

impl PollingWindow {
    pub fn new(order: Vec<SeatId>) -> Self {
        Self {
            order,
            cursor: 0,
            quiet: 0,
        }
    }

    pub fn current(&self) -> Option<SeatId> {
        if self.is_closed() {
            None
        } else {
            Some(self.order[self.cursor])
        }
    }

    pub fn is_closed(&self) -> bool {
        self.order.is_empty() || self.quiet >= self.order.len()
    }

    pub fn pass(&mut self) {
        self.quiet += 1;
        if !self.is_closed() {
            self.cursor = (self.cursor + 1) % self.order.len();
        }
    }

    /// The current seat makes an irrevocable commitment. The rotation
    /// continues from the next seat; no reset to a priority holder.
    pub fn commit(&mut self) {
        self.quiet = 0;
        self.cursor = (self.cursor + 1) % self.order.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seats(n: u8) -> Vec<SeatId> {
        (0..n).map(SeatId).collect()
    }

    #[test]
    fn ordered_closes_on_consecutive_passes() {
        let mut w = OrderedWindow::new(seats(3));
        assert_eq!(w.current(), Some(SeatId(0)));
        w.pass();
        w.pass();
        assert!(!w.is_closed());
        w.pass();
        assert!(w.is_closed());
        assert_eq!(w.current(), None);
    }

    #[test]
    fn ordered_play_resets_impulse_and_voids_passes() {
        // rules-qa 1.2: impulse-reset; a play voids prior passes.
        let mut w = OrderedWindow::new(seats(3));
        w.pass(); // seat 0
        w.pass(); // seat 1
        assert_eq!(w.current(), Some(SeatId(2)));
        w.play(); // seat 2 plays an effect
        assert_eq!(w.current(), Some(SeatId(0)));
        w.pass();
        w.pass();
        assert!(!w.is_closed()); // the two pre-play passes no longer count
        w.pass();
        assert!(w.is_closed());
    }

    #[test]
    fn polling_commit_reopens_response_opportunity() {
        // rules-qa 1.1 / DESIGN §4.2: closure needs a full quiet rotation,
        // so a commitment lets every other seat respond before tally.
        let mut w = PollingWindow::new(seats(3));
        w.commit(); // seat 0 casts
        assert_eq!(w.current(), Some(SeatId(1)));
        w.pass(); // seat 1
        w.pass(); // seat 2
        assert!(!w.is_closed()); // seat 0 must pass again for a full rotation
        w.pass();
        assert!(w.is_closed());
    }

    #[test]
    fn polling_allows_commits_across_rotations() {
        // rules-qa 1.1: independent sources may be spread across rotations;
        // a pass does not forfeit later commitments.
        let mut w = PollingWindow::new(seats(2));
        w.commit(); // seat 0, first source
        w.pass(); // seat 1
        assert_eq!(w.current(), Some(SeatId(0)));
        w.commit(); // seat 0 again, second source
        w.pass(); // seat 1
        w.pass(); // seat 0
        assert!(w.is_closed());
    }
}
