//! Moments a card from hand may answer before they happen (docs/engine.md
//! §6.17).

use crate::cards::{Target, Window};
use crate::decision::{Choice, DecisionRequest, WindowLabel};
use crate::game::Engine;
use crate::ids::{MinionId, SeatId};
use crate::state::CardId;

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
    /// Back as the current minion phase ends (Miguel Cordovera).
    UntilMinionPhaseEnd,
    /// Back as the next action ends, any minion's, else with the turn
    /// (Spirit Marionette [OBE]; owner, rules-qa 124.218).
    UntilNextActionEnd,
    /// For good, moved to the taker's ready region, out of torpor, not a
    /// rescue (Sonja Blue).
    ForGoodToReady,
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

/// Who answers a wait's minion: `order` asked in turn, `next` the one
/// deciding, `playing` while their card resolves, `saved` once a card
/// saved it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Ask {
    pub order: Vec<SeatId>,
    pub next: usize,
    pub playing: bool,
    pub saved: bool,
}

/// A queued wait: `queue[0]` asked, the others in turn after it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Answers<T> {
    pub queue: Vec<T>,
    pub ask: Ask,
}

/// A passed blood hunt about to burn `minion` (one hunt resolves at a
/// time).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HuntBurn {
    pub minion: MinionId,
    pub ask: Ask,
}

/// Allies about to be burned in combat, still in play, their burn waiting
/// on the Methuselahs (Left for Dead), each beside the minion burning it.
pub type CombatBurnWait = Answers<(MinionId, Option<MinionId>)>;

/// What a vampire's burn, waiting on its controller (`BurnWait`), goes on
/// with: a plain burn, or a diablerie's at its site.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BurnThen {
    Burn,
    /// The diablerie action's resolution.
    DiablerieAction {
        diablerist: MinionId,
    },
    /// A blocked leave-torpor's diablerie.
    TorporBlock {
        blocker: MinionId,
    },
    /// A diablerie in combat: Amaranth's, else Liquefy the Mortal Coil's.
    CombatDiablerie {
        diablerist: MinionId,
        amaranth: bool,
    },
    /// Sacrificial Lamb's burn of `minion` by `taker`, for `seat`.
    Sacrifice {
        taker: MinionId,
        seat: SeatId,
    },
    /// The retainer card on the waiting minion burns (Consume the Dead's
    /// "a minion or retainer").
    Retainer(CardId),
}

/// What a replacing card saves from its burn (Consume the Dead).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Burning {
    Minion(MinionId),
    Retainer(CardId),
}

/// Vampires about to be burned (but by a blood hunt, LSJ 20070417), each
/// burn waiting on its controller's card replacing it (Abandoning the
/// Flesh): asked before the burn and before a diablerie's steps, which
/// nothing interrupts (advanced rules §3.7.8); never while a standing
/// replacement applies (`Engine::burn_replacements`; rules-qa 122.2).
pub type BurnWait = Answers<(MinionId, BurnThen)>;

/// Vampires about to go to torpor, provisionally there (a combat ends on
/// it), their torpor waiting on the other Methuselahs.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TorporWait {
    pub answers: Answers<MinionId>,
    /// Sent there in the combat under way: asked as it ends (RTR
    /// 19950509-2).
    pub in_combat: bool,
    /// The combat's end waits on it.
    pub ending: bool,
    /// Those declined so far: their torpors land together once the queue
    /// drains, lost together (rules-qa 118.2).
    pub declined: Vec<MinionId>,
}

/// The waits a card from hand answers, in the order they are settled and
/// asked (docs/engine.md §6.17).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Wait {
    Hunt,
    CombatBurn,
    Torpor,
    Burn,
}

impl Wait {
    const ALL: [Wait; 4] = [Wait::Hunt, Wait::CombatBurn, Wait::Torpor, Wait::Burn];

    /// The windows its answers are played in: a burn's also any
    /// Methuselah's replacement (Consume the Dead); a blood hunt's burn opens
    /// no such window (advanced rules §5.4.4, rules-qa 122.1).
    fn windows(self) -> &'static [Window] {
        match self {
            Wait::Hunt => &[Window::BloodHuntWouldBurn],
            Wait::CombatBurn => &[Window::AllyWouldBurnInCombat],
            Wait::Torpor => &[Window::GoingToTorpor],
            Wait::Burn => &[Window::VampireWouldBurn, Window::MinionWouldBurn],
        }
    }
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
        // Hukros's cards go with him, still their owner's.
        let from = self.state.minions[m.index()].owner;
        let st = &mut self.state;
        for copy in st.minions[m.index()].stored.0.copies().to_vec() {
            if copy != crate::copies::CopyId::NONE && !st.foreign.iter().any(|(c, _)| c.0 == copy) {
                st.foreign.push((crate::copies::Untracked(copy), from));
            }
        }
        match term {
            ControlTerm::ForGood => self.take_for_good_now(m, to),
            ControlTerm::ForGoodToReady => {
                self.take_for_good_now(m, to);
                let v = &self.state.minions[m.index()];
                if v.in_play && v.in_torpor && v.owner == to {
                    self.leave_torpor(m);
                }
            }
            _ => {
                self.take_control_now(m, to);
                let v = &mut self.state.minions[m.index()];
                match term {
                    ControlTerm::UntilNextTurn => v.temp_skip = 1,
                    ControlTerm::UntilUnlock => v.temp_until_unlock = true,
                    ControlTerm::UntilMinionPhaseEnd => {
                        v.temp_until_minion_phase_end = crate::state::Sparse(1)
                    }
                    // Taken during an action: that one is not the next.
                    ControlTerm::UntilNextActionEnd => {
                        v.temp_until_action_end =
                            crate::state::Sparse(if self.state.acting.is_some() { 2 } else { 1 })
                    }
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
    fn settle_control_change(&mut self) {
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
    fn control_request(&self) -> Option<DecisionRequest> {
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

    fn control_apply(&mut self, choice: &Choice) {
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

    /// `w`'s minion asked and who answers it, if it waits.
    fn ask(&self, w: Wait) -> Option<(MinionId, &Ask)> {
        let st = &self.state;
        match w {
            Wait::Hunt => st.hunt_burn.as_ref().map(|h| (h.minion, &h.ask)),
            Wait::CombatBurn => st
                .combat_burn_wait
                .0
                .as_ref()
                .and_then(|a| Some((a.queue.first()?.0, &a.ask))),
            Wait::Torpor => st
                .torpor_wait
                .0
                .as_ref()
                .and_then(|t| Some((*t.answers.queue.first()?, &t.answers.ask))),
            Wait::Burn => st
                .burn_wait
                .0
                .as_ref()
                .and_then(|a| Some((a.queue.first()?.0, &a.ask))),
        }
    }

    fn ask_mut(&mut self, w: Wait) -> Option<&mut Ask> {
        let st = &mut self.state;
        match w {
            Wait::Hunt => st.hunt_burn.as_mut().map(|h| &mut h.ask),
            Wait::CombatBurn => st.combat_burn_wait.0.as_mut().map(|a| &mut a.ask),
            Wait::Torpor => st.torpor_wait.0.as_mut().map(|t| &mut t.answers.ask),
            Wait::Burn => st.burn_wait.0.as_mut().map(|a| &mut a.ask),
        }
    }

    /// Who may answer `m`'s wait: every Methuselah still in, clockwise
    /// from the active one; a torpor's not its controller; a burn's its
    /// controller first.
    fn answer_order(&self, w: Wait, m: MinionId) -> Vec<SeatId> {
        let owner = self.state.minions[m.index()].owner;
        let living = |s: &SeatId| !self.state.seats[s.index()].ousted;
        match w {
            // Its controller first (Abandoning the Flesh), then the others
            // clockwise (Consume the Dead).
            Wait::Burn => std::iter::once(owner)
                .chain(
                    self.state
                        .clockwise_from(self.state.active)
                        .into_iter()
                        .filter(|&s| s != owner),
                )
                .filter(living)
                .collect(),
            Wait::Hunt | Wait::CombatBurn | Wait::Torpor => self
                .state
                .clockwise_from(self.state.active)
                .into_iter()
                .filter(|s| living(s) && (w != Wait::Torpor || *s != owner))
                .collect(),
        }
    }

    /// A Methuselah's card (or a vampire's ability, LSJ 20081203-2:
    /// Dogs of War) for `w` is pending: no other decision is asked.
    pub(crate) fn answer_pending(&self) -> bool {
        self.state.control_change.is_some() || Wait::ALL.into_iter().any(|w| self.ask(w).is_some())
    }

    /// Asked now: a torpor in combat as the combat ends (or once it is
    /// gone), any other at once.
    fn answer_asked(&self, w: Wait) -> bool {
        match w {
            Wait::Torpor => self
                .state
                .torpor_wait
                .0
                .as_ref()
                .is_some_and(|t| !t.in_combat || t.ending || self.combat.is_none()),
            _ => self.ask(w).is_some(),
        }
    }

    /// The plays `seat` may make on `w`'s minion: on that minion where the
    /// card targets one, a blood hunt's abilities too.
    fn answer_plays(&self, w: Wait, seat: SeatId) -> Vec<Choice> {
        let Some((m, _)) = self.ask(w) else {
            return Vec::new();
        };
        // A diablerie's burn: its controller's card only (Consume the Dead:
        // "by any means except diablerie").
        let windows = match self.state.burn_wait.0.as_ref().map(|a| a.queue[0].1) {
            Some(BurnThen::Burn | BurnThen::Sacrifice { .. }) | None => w.windows(),
            Some(BurnThen::Retainer(_)) if w == Wait::Burn => &w.windows()[1..],
            Some(_) if w == Wait::Burn => &w.windows()[..1],
            Some(_) => w.windows(),
        };
        let mut options = self.play_options(seat, windows, None);
        match w {
            Wait::CombatBurn | Wait::Torpor => options.retain(
                |o| matches!(o, Choice::Play { target, .. } if *target == Target::Minion(m)),
            ),
            Wait::Hunt => options.extend(self.ability_options(seat, Window::BloodHuntWouldBurn)),
            Wait::Burn => {}
        }
        options
    }

    /// Some Methuselah of `w`'s order could answer it.
    fn answerable(&self, w: Wait) -> bool {
        self.ask(w)
            .is_some_and(|(_, a)| a.order.iter().any(|&s| !self.answer_plays(w, s).is_empty()))
    }

    /// The first Methuselah from `next` on who may answer, `next` moved to
    /// them.
    fn answer_next(&mut self, w: Wait) -> Option<SeatId> {
        loop {
            let (_, a) = self.ask(w)?;
            let s = *a.order.get(a.next)?;
            if !self.answer_plays(w, s).is_empty() {
                return Some(s);
            }
            self.ask_mut(w).expect("a wait").next += 1;
        }
    }

    /// Before any decision: the control change, then each wait: a finished
    /// play's wait ended; once a card saved the minion or no one is left to
    /// ask, the kind's own end (`answered`).
    pub(crate) fn settle_answers(&mut self) {
        self.settle_control_change();
        for w in Wait::ALL {
            self.settle_answer(w);
        }
    }

    fn settle_answer(&mut self, w: Wait) {
        loop {
            if !self.answer_asked(w) {
                return;
            }
            let a = self.ask_mut(w).expect("a wait");
            if a.playing {
                let s = a.order[a.next];
                if crate::ousting::in_flight(&self.flow, None, s) {
                    return;
                }
                let a = self.ask_mut(w).expect("a wait");
                a.playing = false;
                a.next += 1;
            }
            let saved = self.ask(w).expect("a wait").1.saved;
            if !saved && self.answer_next(w).is_some() {
                return;
            }
            if self.answered(w) {
                return;
            }
        }
    }

    /// The control change's request, else the first wait's.
    pub(crate) fn answer_request(&self) -> Option<DecisionRequest> {
        self.control_request()
            .or_else(|| Wait::ALL.into_iter().find_map(|w| self.wait_request(w)))
    }

    fn wait_request(&self, w: Wait) -> Option<DecisionRequest> {
        if !self.answer_asked(w) {
            return None;
        }
        let (_, a) = self.ask(w).filter(|(_, a)| !a.playing)?;
        let s = *a.order.get(a.next)?;
        let plays = self.answer_plays(w, s);
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

    /// Applies `choice` to the request `answer_request` serves: false when
    /// it serves none.
    pub(crate) fn answer_apply(&mut self, choice: &Choice) -> bool {
        if self.control_request().is_some() {
            self.control_apply(choice);
            return true;
        }
        let Some(w) = Wait::ALL
            .into_iter()
            .find(|&w| self.wait_request(w).is_some())
        else {
            return false;
        };
        let a = self.ask_mut(w).expect("a wait");
        let s = a.order[a.next];
        match *choice {
            Choice::Pass => {
                a.next += 1;
            }
            Choice::Play {
                card,
                mode,
                actor,
                target,
            } => {
                a.playing = true;
                let resume = self.flow.clone();
                self.announce_play(s, card, mode, actor, target, resume);
            }
            // An ability resolves at once (no terms at this moment).
            Choice::UseAbility {
                source,
                ability,
                actor,
                target,
            } => {
                a.next += 1;
                let resume = self.flow.clone();
                self.use_ability(s, source, ability, actor, target, resume);
            }
            other => unreachable!("illegal {w:?} choice {other:?}"),
        }
        true
    }

    /// The minion whose burn waits on the card now resolving (Consume the
    /// Dead), and that wait marked saved.
    pub(crate) fn save_burning(&mut self) -> Option<Burning> {
        let (m, a) = self.ask(Wait::Burn)?;
        if !a.playing {
            return None;
        }
        let retainer = match self.state.burn_wait.0.as_ref().map(|a| a.queue[0].1) {
            Some(BurnThen::Retainer(c)) => Some(c),
            _ => None,
        };
        self.ask_mut(Wait::Burn).expect("a wait").saved = true;
        Some(retainer.map_or(Burning::Minion(m), Burning::Retainer))
    }

    /// A card saved `w`'s minion if it is `target`: true when it did.
    pub(crate) fn save_answer(&mut self, w: Wait, target: Target) -> bool {
        if self.ask(w).is_none_or(|(m, _)| Target::Minion(m) != target) {
            return false;
        }
        self.ask_mut(w).expect("a wait").saved = true;
        true
    }

    /// `w`'s minion answered: its kind's end. True when the settling
    /// stops (a combat's end the torpors held, now run).
    fn answered(&mut self, w: Wait) -> bool {
        match w {
            Wait::Hunt => {
                let h = self.state.hunt_burn.take().expect("hunt burn");
                if !h.ask.saved {
                    self.hunt_burn_now(h.minion);
                }
                self.end_blood_hunt();
                false
            }
            Wait::CombatBurn => {
                let mut a = self.state.combat_burn_wait.0.take().expect("burn wait");
                let (m, by) = a.queue.remove(0);
                if !a.ask.saved && self.state.minions[m.index()].present() {
                    self.lost_together(|e| {
                        if let Some(by) = by {
                            e.red_list_burned(by, m);
                        }
                        e.burn_minion(m);
                    });
                }
                if let Some(&(n, _)) = a.queue.first() {
                    a.ask = self.fresh_ask(w, n);
                    self.state.combat_burn_wait.0 = Some(a);
                }
                false
            }
            Wait::Torpor => self.torpor_answered(),
            Wait::Burn => {
                let mut a = self.state.burn_wait.0.take().expect("burn wait");
                let saved = a.ask.saved;
                let (m, then) = a.queue.remove(0);
                if let Some(&(n, _)) = a.queue.first() {
                    a.ask = self.fresh_ask(w, n);
                    self.state.burn_wait.0 = Some(a);
                }
                if let BurnThen::Retainer(c) = then {
                    if !saved && self.state.cards[c.index()].in_play {
                        self.burn_card_in_play_now(c);
                    }
                } else if self.state.minions[m.index()].present() {
                    self.burn_goes_on(m, then);
                } else {
                    self.burn_saved(then);
                }
                false
            }
        }
    }

    fn fresh_ask(&self, w: Wait, m: MinionId) -> Ask {
        Ask {
            order: self.answer_order(w, m),
            ..Ask::default()
        }
    }

    /// A passed blood hunt burns (or sends to torpor: Genina) `m` — unless
    /// a Methuselah may still answer it: then it waits, the referendum's
    /// end with it. True when it waits.
    pub(crate) fn defer_hunt_burn(&mut self, m: MinionId) -> bool {
        // Its own Methuselah too: a cancel of a hunt called on their
        // vampire (Dogs of War; LSJ 20081203-2).
        self.state.hunt_burn = Some(HuntBurn {
            minion: m,
            ask: self.fresh_ask(Wait::Hunt, m),
        });
        if self.answer_next(Wait::Hunt).is_some() {
            return true;
        }
        self.state.hunt_burn = None;
        false
    }

    /// The allies of `burned` a Methuselah may save as they would burn in
    /// combat (Left for Dead) are held back, still in play, their burn
    /// waiting; the others are returned. Only the damage resolution's
    /// burns reach here, never a burn as the combat ends (Escaped Mental
    /// Patient, LSJ 20070507).
    pub(crate) fn defer_combat_burns(&mut self, burned: Vec<MinionId>) -> Vec<MinionId> {
        let held = (0..self.state.seats.len()).any(|i| {
            !self.state.seats[i].ousted
                && self.holds_card_for(SeatId(i as u8), Window::AllyWouldBurnInCombat)
        });
        if !held {
            return burned;
        }
        let mut rest = Vec::new();
        for m in burned {
            if !self.state.minions[m.index()].is_ally() || !self.combat_burn_savable(m) {
                rest.push(m);
                continue;
            }
            let by = self.combat.as_ref().and_then(|c| c.opponent_of(m));
            match self.state.combat_burn_wait.0.as_mut() {
                Some(a) => a.queue.push((m, by)),
                None => {
                    self.state.combat_burn_wait.0 = Some(Answers {
                        queue: vec![(m, by)],
                        ask: self.fresh_ask(Wait::CombatBurn, m),
                    })
                }
            }
        }
        rest
    }

    /// Some Methuselah could play a card saving `m` from its burn, the
    /// wait under way set aside meanwhile.
    fn combat_burn_savable(&mut self, m: MinionId) -> bool {
        let alone = Answers {
            queue: vec![(m, None)],
            ask: self.fresh_ask(Wait::CombatBurn, m),
        };
        let before = self.state.combat_burn_wait.0.replace(alone);
        let any = self.answerable(Wait::CombatBurn);
        self.state.combat_burn_wait.0 = before;
        any
    }

    /// The ready vampires of `ms` another Methuselah may save from torpor
    /// (Minor Boon) go there provisionally, their torpor waiting; the
    /// others are returned.
    pub(crate) fn defer_torpor(&mut self, ms: &[MinionId]) -> Vec<MinionId> {
        let mut rest = Vec::new();
        for &m in ms {
            let v = &self.state.minions[m.index()];
            let owner = v.owner;
            let held = v.is_vampire()
                && v.ready()
                && self.state.seats.iter().enumerate().any(|(i, s)| {
                    i != owner.index()
                        && !s.ousted
                        && self.holds_card_for(SeatId(i as u8), Window::GoingToTorpor)
                });
            if !held || !self.torpor_savable(m) {
                rest.push(m);
                continue;
            }
            self.state.minions[m.index()].in_torpor = true;
            let in_combat = self
                .combat
                .as_ref()
                .is_some_and(|c| c.side_of_minion(m).is_some());
            match self.state.torpor_wait.0.as_mut() {
                Some(t) => t.answers.queue.push(m),
                None => {
                    self.state.torpor_wait.0 = Some(TorporWait {
                        answers: Answers {
                            queue: vec![m],
                            ask: self.fresh_ask(Wait::Torpor, m),
                        },
                        in_combat,
                        ending: false,
                        declined: Vec::new(),
                    })
                }
            }
        }
        rest
    }

    /// Some Methuselah could play a card saving `m`, its torpor waiting
    /// (the wait under way set aside meanwhile).
    fn torpor_savable(&mut self, m: MinionId) -> bool {
        let alone = TorporWait {
            answers: Answers {
                queue: vec![m],
                ask: self.fresh_ask(Wait::Torpor, m),
            },
            in_combat: false,
            ending: false,
            declined: Vec::new(),
        };
        let before = self.state.torpor_wait.0.replace(alone);
        self.state.minions[m.index()].in_torpor = true;
        let any = self.answerable(Wait::Torpor);
        self.state.minions[m.index()].in_torpor = false;
        self.state.torpor_wait.0 = before;
        any
    }

    /// The combat's end waits on the torpors it caused: true when it does.
    pub(crate) fn torpor_holds_combat_end(&mut self) -> bool {
        match self.state.torpor_wait.0.as_mut() {
            Some(t) if t.in_combat && !t.ending => {
                t.ending = true;
                true
            }
            _ => false,
        }
    }

    /// A torpor answered: the next one's asked, else the torpors declined
    /// land together; then the combat's end it held. True once that end
    /// ran.
    fn torpor_answered(&mut self) -> bool {
        let mut t = self.state.torpor_wait.0.take().expect("torpor wait");
        let m = t.answers.queue.remove(0);
        let ending = t.ending;
        if !t.answers.ask.saved {
            t.declined.push(m);
        }
        if let Some(&n) = t.answers.queue.first() {
            t.answers.ask = self.fresh_ask(Wait::Torpor, n);
            self.state.torpor_wait.0 = Some(t);
        } else {
            let declined: Vec<MinionId> = t
                .declined
                .into_iter()
                .filter(|&m| {
                    let v = &self.state.minions[m.index()];
                    v.present() && v.in_torpor
                })
                .collect();
            if !declined.is_empty() {
                for &m in &declined {
                    self.state.minions[m.index()].in_torpor = false;
                }
                self.torpor_now(&declined);
            }
        }
        if ending {
            match self.state.torpor_wait.0.as_mut() {
                Some(t) => t.ending = true,
                None => {
                    if self.combat.is_some() {
                        self.end_combat();
                    }
                    return true;
                }
            }
        }
        false
    }

    /// `m` (or, `BurnThen::Retainer`, a retainer on it) is about to be
    /// burned (`then` going on once it is): it waits if its controller may
    /// play a card instead, or any Methuselah one replacing a minion's
    /// burn (`BurnWait`; Consume the Dead, not a diablerie's), behind any
    /// burn already waiting (two vampires burned together). True when it
    /// waits. Not asked while a standing replacement applies (Charigger's,
    /// Byzar's own, Book of Going Forth by Night's): it applies, the acting
    /// Methuselah choosing among several, and the card is not playable
    /// (owner, rules-qa 122.2, 121.19).
    pub(crate) fn ask_burn(&mut self, m: MinionId, then: BurnThen) -> bool {
        let v = &self.state.minions[m.index()];
        let seat = v.owner;
        let retainer = matches!(then, BurnThen::Retainer(_));
        let own = v.is_vampire()
            && !retainer
            && !self.state.seats[seat.index()].ousted
            && self.holds_card_for(seat, Window::VampireWouldBurn);
        let any = matches!(
            then,
            BurnThen::Burn | BurnThen::Sacrifice { .. } | BurnThen::Retainer(_)
        ) && (0..self.state.seats.len()).any(|i| {
            !self.state.seats[i].ousted
                && self.holds_card_for(SeatId(i as u8), Window::MinionWouldBurn)
        });
        if !v.present() || !own && !any || !retainer && !self.burn_replacements(m).is_empty() {
            return false;
        }
        let alone = Answers {
            queue: vec![(m, then)],
            ask: self.fresh_ask(Wait::Burn, m),
        };
        let before = self.state.burn_wait.0.replace(alone);
        let any = self.answerable(Wait::Burn);
        match before {
            Some(mut a) => {
                if any {
                    a.queue.push((m, then));
                }
                self.state.burn_wait.0 = Some(a);
            }
            None if !any => self.state.burn_wait.0 = None,
            None => {}
        }
        any
    }

    /// `Actor::BurningVampire` for `seat`.
    pub(crate) fn burning_vampire(&self, seat: SeatId) -> Option<MinionId> {
        let (m, a) = self.ask(Wait::Burn)?;
        if a.order.first() != Some(&seat) {
            return None;
        }
        let v = &self.state.minions[m.index()];
        if !v.present() {
            return None;
        }
        if let Some(c) = &self.combat {
            return c.side_of_minion(m).map(|_| m);
        }
        // An action card's effects resolve with the action out of
        // `Engine::action`: its acting minion tells whose it is.
        let acting = self.state.acting?;
        let woken = self.action.as_ref().is_some_and(|a| a.woken.contains(&m));
        (self.state.minions[acting.index()].owner != seat && (!v.locked || woken)).then_some(m)
    }

    /// The burn not replaced: as if never asked.
    fn burn_goes_on(&mut self, m: MinionId, then: BurnThen) {
        match then {
            BurnThen::Burn => self.burn_minion_now(m),
            BurnThen::DiablerieAction { diablerist } => self.diablerie_action(diablerist, m),
            BurnThen::TorporBlock { blocker } => self.torpor_block_diablerie(blocker, m),
            BurnThen::CombatDiablerie {
                diablerist,
                amaranth,
            } => self.combat_diablerie(diablerist, m, amaranth),
            BurnThen::Sacrifice { taker, seat } => self.sacrifice(seat, taker, m),
            BurnThen::Retainer(c) => self.burn_card_in_play_now(c),
        }
    }

    /// The vampire removed instead: a diablerie action unsuccessful, as a
    /// replaced burn makes it (rules-qa 121.20).
    fn burn_saved(&mut self, then: BurnThen) {
        match then {
            BurnThen::DiablerieAction { .. } | BurnThen::TorporBlock { .. } => {
                self.after_resolution()
            }
            BurnThen::Burn
            | BurnThen::CombatDiablerie { .. }
            | BurnThen::Sacrifice { .. }
            | BurnThen::Retainer(_) => {}
        }
    }
}

impl Engine {
    /// `blocker`'s attempt is about to succeed: its controller, holding a
    /// card for the moment (Lesser Boon), is asked first; else block
    /// resolution begins.
    pub(crate) fn block_would_succeed(&mut self, blocker: MinionId) {
        let seat = self.state.minions[blocker.index()].owner;
        if self.holds_card_for(seat, Window::BlockWouldSucceed)
            && !self.block_moment_plays(seat).is_empty()
        {
            self.interrupts
                .push(crate::draws::Interrupt::BlockWouldSucceed { seat, blocker });
            self.flow = crate::game::Flow::Action;
        } else {
            self.blocked(blocker);
        }
    }

    /// The plays `seat` may make as its minion's block would succeed.
    pub(crate) fn block_moment_plays(&self, seat: SeatId) -> Vec<Choice> {
        self.play_options(seat, &[Window::BlockWouldSucceed], None)
    }

    /// The attempt under way does not succeed: the action resolves as if
    /// unblocked, no further attempt made (Lesser Boon; RTR 20080808).
    pub(crate) fn void_block(&mut self) {
        if let Some(a) = self.action.as_mut() {
            if matches!(a.state, crate::actions::ActionState::Attempt(_)) {
                a.state = crate::actions::ActionState::Declined;
                a.fresh_window();
            }
        }
    }
}
