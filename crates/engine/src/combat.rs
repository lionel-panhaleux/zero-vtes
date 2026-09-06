//! Combat (rulebook "Combat"; spec §4). A combat is part of the action that
//! produced it — `Engine::action` stays alive and its layers stay in force
//! (spec §4.1.2). Each ordered step is a window over the two combatant
//! seats, the acting minion's controller first, with impulse-reset
//! (rules-qa 1.2); strike choice and damage prevention are single-seat
//! offers. The steps of a round (rulebook "Combat Sequence"):
//!
//! before range → determine range (alternating maneuvers) → before strikes
//! → choose strikes (acting first) → before resolution (spec §4.3.2) →
//! resolve (combat ends first, else simultaneous; steal before mend) →
//! prevention (acting side first, spec §4.5.4) → damage applied at once →
//! additional strikes → press (alternating) → end of round, shared with the
//! would-end replacement point (spec §4.9.1).
//!
//! A combatant no longer ready ends the round at once, but the end-of-round
//! window still opens (rulebook step 7).

use crate::cards::{self, Effect, PlayCtx, StrikeKind, Window};
use crate::decision::{Choice, DecisionRequest, WindowLabel};
use crate::game::{Engine, Flow};
use crate::ids::{MinionId, SeatId};
use crate::state::{CardId, GameState};
use crate::window::OrderedWindow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Range {
    Close,
    Long,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    BeforeRange,
    DetermineRange,
    BeforeStrikes,
    /// Side `i` chooses its strike.
    ChooseStrike(usize),
    /// Both strikes chosen, before they resolve.
    BeforeResolution,
    /// Side `i` prevents the damage pending on it.
    Prevent(usize),
    /// A prevention of `n` covers only part of mixed damage on side `side`:
    /// the preventer chooses how many aggravated points it covers (spec
    /// §4.5.2; owner 2026-09-04: one may prefer torpor with blood intact).
    Allocate {
        side: usize,
        n: u8,
    },
    AdditionalStrikes,
    Press,
    EndOfRound,
}

/// Damage waiting for prevention, by kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Damage {
    pub normal: u8,
    pub aggravated: u8,
}

impl Damage {
    pub fn total(&self) -> u8 {
        self.normal + self.aggravated
    }

    fn add(&mut self, n: u8, aggravated: bool) {
        if aggravated {
            self.aggravated += n;
        } else {
            self.normal += n;
        }
    }

    /// Prevent up to `n` points, aggravated first when allowed. Returns
    /// what was prevented. Mixed damage with a partial prevention goes
    /// through `Step::Allocate` instead (rules-qa 4.2).
    fn prevent(&mut self, n: u8, aggravated_ok: bool) -> u8 {
        let mut left = n;
        if aggravated_ok {
            let a = left.min(self.aggravated);
            self.aggravated -= a;
            left -= a;
        }
        let b = left.min(self.normal);
        self.normal -= b;
        n - (left - b)
    }
}

#[derive(Clone, Debug)]
pub struct Side {
    pub minion: MinionId,
    pub seat: SeatId,
    /// This pair's strike, once chosen; `None` with `chosen` for a side
    /// without a strike in an additional pair.
    pub strike: Option<StrikeKind>,
    pub chosen: bool,
    /// Maneuvers and presses banked for this round.
    pub maneuvers: u8,
    pub presses: u8,
    /// Additional strikes left this round; one "(limited)" source a round.
    pub additional: u8,
    pub limited_used: bool,
    pub aggravated_hands: bool,
    /// Blood burned to damage this round (Taste of Vitae).
    pub lost_to_damage: u8,
    pub pending: Damage,
    /// The prevention offer is open (closes on a pass).
    pub preventing: bool,
    /// "Gets a strength of n this combat" (spec §4.4.1: replaces the base).
    pub strength_set: Option<u8>,
    pub next_round_maneuvers: u8,
    /// Form of Mist [PRO] rider, armed by the play, fired by the strike.
    pub continue_rider: Option<(u8, u8)>,
}

impl Side {
    fn new(minion: MinionId, seat: SeatId) -> Self {
        Self {
            minion,
            seat,
            strike: None,
            chosen: false,
            maneuvers: 0,
            presses: 0,
            additional: 0,
            limited_used: false,
            aggravated_hands: false,
            lost_to_damage: 0,
            pending: Damage::default(),
            preventing: false,
            strength_set: None,
            next_round_maneuvers: 0,
            continue_rider: None,
        }
    }

    fn new_round(&mut self) {
        let maneuvers = self.next_round_maneuvers;
        *self = Self {
            strength_set: self.strength_set,
            continue_rider: None,
            maneuvers,
            ..Self::new(self.minion, self.seat)
        };
    }

    /// Rulebook "Hand Strike": vampires have a default strength of 1.
    pub fn strength(&self) -> u8 {
        self.strength_set.unwrap_or(1)
    }
}

#[derive(Clone, Debug)]
pub struct CombatCtx {
    /// `sides[0]` is the acting minion's, `sides[1]` the opponent's.
    pub sides: [Side; 2],
    pub round: u8,
    /// Strike pair within the round; 0 is the normal pair.
    pub pair: u8,
    pub range: Range,
    /// The range was set by an effect: skip the determine range step.
    pub range_set: bool,
    pub next_round_close: bool,
    pub only_hand_strikes: bool,
    pub step: Step,
    pub window: OrderedWindow,
    pub last_maneuver: Option<usize>,
    pub last_press: Option<usize>,
    /// An uncanceled press to continue stands.
    pub press_continue: bool,
    /// The round ended early (a combat ends strike, a combatant no longer
    /// ready): straight to the end-of-round window.
    pub round_over: bool,
    /// A would-end replacement started a new round.
    pub continued: bool,
    /// Card names played per minion, for `Limit::PerRound` / `PerCombat`.
    pub played_round: Vec<(MinionId, &'static str)>,
    pub played_combat: Vec<(MinionId, &'static str)>,
    /// In-play abilities used this combat, by copy ("each combat").
    pub abilities_used: Vec<CardId>,
}

impl CombatCtx {
    pub fn side_of(&self, seat: SeatId) -> Option<usize> {
        self.sides.iter().position(|s| s.seat == seat)
    }

    pub fn side_of_minion(&self, m: MinionId) -> Option<usize> {
        self.sides.iter().position(|s| s.minion == m)
    }

    pub fn both_ready(&self, st: &GameState) -> bool {
        self.sides
            .iter()
            .all(|s| st.minions[s.minion.index()].ready())
    }

    /// The end-of-round window, combat about to end (spec §4.9.1).
    pub fn would_end(&self) -> bool {
        self.step == Step::EndOfRound && !self.press_continue && !self.continued
    }

    fn order(&self) -> Vec<SeatId> {
        vec![self.sides[0].seat, self.sides[1].seat]
    }

    fn fresh_window(&mut self, step: Step) {
        self.step = step;
        self.window = OrderedWindow::new(self.order());
    }

    /// Play windows open at the current step. `COMBAT_UNTIMED` cards are
    /// playable at any impulse before the resolution they affect (rules-qa
    /// 2.2); limited additional strikes before or after the normal pair
    /// (spec §4.3.5).
    fn windows(&self) -> Vec<Window> {
        match self.step {
            Step::BeforeRange => vec![Window::CombatBeforeRange, Window::CombatUntimed],
            Step::DetermineRange => vec![Window::CombatDetermineRange, Window::CombatUntimed],
            Step::BeforeStrikes => vec![
                Window::CombatBeforeStrikes,
                Window::CombatAdditionalStrike,
                Window::CombatUntimed,
            ],
            Step::ChooseStrike(_) => vec![Window::CombatStrikeDeclaration],
            Step::BeforeResolution => vec![Window::CombatUntimed, Window::CombatAdditionalStrike],
            Step::Prevent(_) | Step::Allocate { .. } => vec![Window::CombatDamagePrevention],
            Step::AdditionalStrikes => vec![Window::CombatAdditionalStrike, Window::CombatUntimed],
            Step::Press => vec![Window::CombatPress],
            Step::EndOfRound => {
                let mut w = vec![Window::CombatEndOfRound];
                if self.would_end() {
                    w.push(Window::CombatWouldEnd);
                }
                w
            }
        }
    }

    fn label(&self) -> WindowLabel {
        match self.step {
            Step::BeforeRange => WindowLabel::CombatBeforeRange,
            Step::DetermineRange => WindowLabel::CombatDetermineRange,
            Step::BeforeStrikes => WindowLabel::CombatBeforeStrikes,
            Step::ChooseStrike(_) => WindowLabel::CombatStrike,
            Step::BeforeResolution => WindowLabel::CombatBeforeResolution,
            Step::Prevent(_) => WindowLabel::CombatPrevention,
            Step::Allocate { .. } => WindowLabel::CombatPreventionAllocation,
            Step::AdditionalStrikes => WindowLabel::CombatAdditionalStrikes,
            Step::Press => WindowLabel::CombatPress,
            Step::EndOfRound => WindowLabel::CombatEndOfRound,
        }
    }

    /// Effect-derived legality at the current step: no two maneuvers or
    /// presses in a row by the same minion, one limited additional-strike
    /// source per round, only hand strikes under a grapple.
    pub(crate) fn allows(&self, mode: &cards::ModeDef, side: usize) -> bool {
        mode.effects.iter().all(|e| match e {
            Effect::Maneuver if self.step == Step::DetermineRange => {
                self.last_maneuver != Some(side)
            }
            Effect::Press if self.step == Step::Press => self.last_press != Some(side),
            Effect::AdditionalStrikes { limited: true, .. } => !self.sides[side].limited_used,
            Effect::Strike(k) if self.only_hand_strikes => matches!(k, StrikeKind::Hand { .. }),
            _ => true,
        })
    }
}

/// Outcome of damage resolution on one minion (rulebook "Damage
/// Resolution").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Resolved {
    pub blood_burned: u8,
    pub wounded: bool,
    pub burned: bool,
}

/// Normal damage first: 1 blood per point mends it, unmended points wound.
/// Then aggravated: no mending, the first point wounds; each further point
/// on a wounded vampire burns 1 blood or the vampire (rulebook examples
/// Nassir, Tamoszius, Ryan).
pub(crate) fn resolve_damage(blood: u8, d: Damage) -> Resolved {
    let mut blood = blood;
    let mut burned_blood = 0;
    let mend = blood.min(d.normal);
    blood -= mend;
    burned_blood += mend;
    let mut wounded = d.normal > mend;
    let mut burned = false;
    for _ in 0..d.aggravated {
        if !wounded {
            wounded = true;
        } else if blood > 0 {
            blood -= 1;
            burned_blood += 1;
        } else {
            burned = true;
            break;
        }
    }
    Resolved {
        blood_burned: burned_blood,
        wounded,
        burned,
    }
}

impl Engine {
    /// Block resolution produced a combat: `acting` is the acting minion,
    /// `opposing` the blocker (rules-qa 1.2: the opponent is a "blocker"
    /// only if it actually blocked).
    pub(crate) fn start_combat(&mut self, acting: MinionId, opposing: MinionId) {
        let st = &self.state;
        let sides = [
            Side::new(acting, st.minions[acting.index()].owner),
            Side::new(opposing, st.minions[opposing.index()].owner),
        ];
        let order = vec![sides[0].seat, sides[1].seat];
        self.combat = Some(CombatCtx {
            sides,
            round: 1,
            pair: 0,
            range: Range::Close,
            range_set: false,
            next_round_close: false,
            only_hand_strikes: false,
            step: Step::BeforeRange,
            window: OrderedWindow::new(order),
            last_maneuver: None,
            last_press: None,
            press_continue: false,
            round_over: false,
            continued: false,
            played_round: Vec::new(),
            played_combat: Vec::new(),
            abilities_used: Vec::new(),
        });
        self.flow = Flow::Combat;
    }

    pub(crate) fn combat_request(&self) -> Option<DecisionRequest> {
        let c = self.combat.as_ref().expect("combat context");
        match c.step {
            Step::ChooseStrike(i) => {
                let side = &c.sides[i];
                if side.chosen {
                    return None;
                }
                let mut options = vec![Choice::HandStrike];
                options.extend(self.play_options(side.seat, &c.windows(), None));
                Some(DecisionRequest {
                    seat: side.seat,
                    window: WindowLabel::CombatStrike,
                    options,
                })
            }
            Step::Allocate { side, n } => {
                let d = c.sides[side].pending;
                let lo = n.saturating_sub(d.normal);
                let hi = n.min(d.aggravated);
                Some(DecisionRequest {
                    seat: c.sides[side].seat,
                    window: WindowLabel::CombatPreventionAllocation,
                    options: (lo..=hi).map(Choice::SetX).collect(),
                })
            }
            Step::Prevent(i) => {
                let side = &c.sides[i];
                if !side.preventing || side.pending.total() == 0 {
                    return None;
                }
                let mut options = vec![Choice::Pass];
                options.extend(self.play_options(side.seat, &c.windows(), None));
                for w in c.windows() {
                    options.extend(self.ability_options(side.seat, w));
                }
                Some(DecisionRequest {
                    seat: side.seat,
                    window: WindowLabel::CombatPrevention,
                    options,
                })
            }
            _ => {
                let seat = c.window.current()?;
                let i = c.side_of(seat).expect("combatant seat");
                let mut options = vec![Choice::Pass];
                match c.step {
                    Step::DetermineRange
                        if c.sides[i].maneuvers > 0 && c.last_maneuver != Some(i) =>
                    {
                        options.push(Choice::UseManeuver);
                    }
                    Step::Press if c.sides[i].presses > 0 && c.last_press != Some(i) => {
                        options.push(Choice::UsePress);
                    }
                    _ => {}
                }
                options.extend(self.play_options(seat, &c.windows(), None));
                for w in c.windows() {
                    options.extend(self.ability_options(seat, w));
                }
                Some(DecisionRequest {
                    seat,
                    window: c.label(),
                    options,
                })
            }
        }
    }

    pub(crate) fn combat_apply(&mut self, choice: &Choice) {
        self.flow = Flow::Combat;
        let c = self.combat.as_mut().expect("combat context");
        // The seat acting on this choice, and whether the play goes through
        // the as-played stack (announced after the borrow ends).
        let play = match (c.step, choice) {
            (Step::ChooseStrike(i), Choice::HandStrike) => {
                c.sides[i].strike = Some(StrikeKind::Hand { bonus: 0 });
                c.sides[i].chosen = true;
                None
            }
            (Step::ChooseStrike(i), Choice::Play { .. }) => Some(c.sides[i].seat),
            (Step::Prevent(i), Choice::Pass) => {
                c.sides[i].preventing = false;
                None
            }
            (Step::Allocate { side, n }, Choice::SetX(k)) => {
                let d = &mut c.sides[side].pending;
                d.aggravated -= k;
                d.normal -= n - k;
                c.step = Step::Prevent(side);
                None
            }
            (Step::Prevent(i), Choice::Play { .. }) => Some(c.sides[i].seat),
            // An ability is not a card play: no as-played window.
            (Step::Prevent(_), Choice::UseAbility { .. }) => None,
            (_, Choice::UseAbility { .. }) => {
                c.window.play();
                None
            }
            (_, Choice::Pass) => {
                c.window.pass();
                None
            }
            (Step::DetermineRange, Choice::UseManeuver) => {
                let i = c
                    .side_of(c.window.current().expect("impulse"))
                    .expect("side");
                c.sides[i].maneuvers -= 1;
                c.window.play();
                c.maneuver(i);
                None
            }
            (Step::Press, Choice::UsePress) => {
                let i = c
                    .side_of(c.window.current().expect("impulse"))
                    .expect("side");
                c.sides[i].presses -= 1;
                c.window.play();
                c.press(i);
                None
            }
            (_, Choice::Play { .. }) => {
                let seat = c.window.current().expect("impulse holder");
                c.window.play();
                Some(seat)
            }
            (step, other) => unreachable!("illegal combat choice {other:?} at {step:?}"),
        };
        if let (
            Some(seat),
            Choice::Play {
                card,
                mode,
                actor,
                target,
            },
        ) = (play, choice)
        {
            self.announce_play(seat, *card, *mode, *actor, *target, Flow::Combat);
        }
        if let Choice::UseAbility {
            card,
            ability,
            actor,
            target,
        } = choice
        {
            self.use_ability(*card, *ability, *actor, *target);
        }
    }

    /// The current step is done (window closed, strike chosen, prevention
    /// passed): move to the next one.
    pub(crate) fn combat_advance(&mut self) {
        self.flow = Flow::Combat;
        let step = self.combat.as_ref().expect("combat context").step;
        let c = self.combat.as_mut().expect("combat context");
        match step {
            Step::BeforeRange => {
                if c.range_set {
                    c.fresh_window(Step::BeforeStrikes);
                } else {
                    c.fresh_window(Step::DetermineRange);
                }
            }
            Step::DetermineRange => c.fresh_window(Step::BeforeStrikes),
            Step::BeforeStrikes => c.begin_pair(),
            Step::ChooseStrike(0) => c.fresh_window(Step::ChooseStrike(1)),
            Step::ChooseStrike(_) => c.fresh_window(Step::BeforeResolution),
            Step::BeforeResolution => {
                self.resolve_strikes();
                self.after_prevention_or_next();
            }
            Step::Prevent(i) => {
                c.sides[i].preventing = false;
                self.after_prevention_or_next();
            }
            Step::Allocate { .. } => unreachable!("allocation always offers a choice"),
            Step::AdditionalStrikes => {
                if c.sides.iter().any(|s| s.additional > 0) {
                    c.begin_pair();
                } else {
                    c.fresh_window(Step::Press);
                }
            }
            Step::Press => {
                if c.press_continue {
                    self.new_round();
                } else {
                    c.fresh_window(Step::EndOfRound);
                }
            }
            Step::EndOfRound => {
                if c.continued || c.press_continue {
                    self.new_round();
                } else {
                    self.end_combat();
                }
            }
        }
    }

    /// After strike resolution: the acting side prevents first, then the
    /// opponent; then all damage applies at once (spec §4.5.4).
    fn after_prevention_or_next(&mut self) {
        {
            let c = self.combat.as_mut().expect("combat context");
            for i in 0..2 {
                if c.sides[i].preventing && c.sides[i].pending.total() > 0 {
                    c.step = Step::Prevent(i);
                    return;
                }
            }
        }
        self.apply_damage();
        let c = self.combat.as_mut().expect("combat context");
        if c.round_over {
            c.fresh_window(Step::EndOfRound);
        } else if c.pair == 0 {
            c.fresh_window(Step::AdditionalStrikes);
        } else if c.sides.iter().any(|s| s.additional > 0) {
            c.begin_pair();
        } else {
            c.fresh_window(Step::Press);
        }
    }

    /// Strike resolution (rulebook "Resolve Strike"; spec §4.3.3, §4.7):
    /// a combat ends strike resolves first and alone; otherwise strikes
    /// resolve simultaneously, a dodge voiding the opposing strike, stolen
    /// blood moving before damage is mended.
    fn resolve_strikes(&mut self) {
        let c = self.combat.as_mut().expect("combat context");
        let strikes = [c.sides[0].strike, c.sides[1].strike];
        let ends: Vec<usize> = (0..2)
            .filter(|&i| matches!(strikes[i], Some(StrikeKind::CombatEnds { .. })))
            .collect();
        if !ends.is_empty() {
            for i in ends {
                let Some(StrikeKind::CombatEnds { unlock }) = strikes[i] else {
                    unreachable!()
                };
                let m = c.sides[i].minion;
                if unlock {
                    // "Unlock this vampire before combat ends": resolves with
                    // the strike, so it survives a continued combat (spec
                    // §4.9.2).
                    self.state.minions[m.index()].locked = false;
                }
                if let Some((blood, stealth)) = c.sides[i].continue_rider.take() {
                    // Form of Mist [PRO]: only the acting minion "was
                    // blocked" (every v1 combat arises from a block).
                    if i == 0 {
                        let a = self.action.as_mut().expect("action context");
                        a.continuation = Some((blood, stealth));
                    }
                }
            }
            c.round_over = true;
            for s in &mut c.sides {
                s.strike = None;
            }
            return;
        }
        let close = c.range == Range::Close;
        let mut steals = [0u8; 2];
        for i in 0..2 {
            let j = 1 - i;
            let Some(k) = strikes[i] else {
                continue;
            };
            let dodged = strikes[j] == Some(StrikeKind::Dodge);
            match k {
                StrikeKind::Hand { bonus } => {
                    if close && !dodged {
                        let n = c.sides[i].strength() + bonus;
                        let aggravated = c.sides[i].aggravated_hands;
                        c.sides[j].pending.add(n, aggravated);
                    }
                }
                StrikeKind::Damage {
                    n,
                    ranged,
                    aggravated,
                } => {
                    if (ranged || close) && !dodged {
                        c.sides[j].pending.add(n, aggravated);
                    }
                }
                StrikeKind::StealBlood { n, ranged } => {
                    if (ranged || close) && !dodged {
                        steals[i] = n.min(self.state.minions[c.sides[j].minion.index()].blood);
                    }
                }
                StrikeKind::Dodge | StrikeKind::CombatEnds { .. } => {}
            }
        }
        // Simultaneous steals: amounts from the pre-strike state, capacity
        // applied after both moves (Theft of Vitae rulings).
        for (i, &n) in steals.iter().enumerate() {
            let j = 1 - i;
            self.state.minions[c.sides[j].minion.index()].blood -= n;
            self.state.minions[c.sides[i].minion.index()].blood += n;
        }
        for s in &c.sides {
            let m = &mut self.state.minions[s.minion.index()];
            m.blood = m.blood.min(m.capacity());
        }
        for s in &mut c.sides {
            s.strike = None;
            s.preventing = s.pending.total() > 0;
        }
    }

    /// Damage applied at once to both sides after prevention (rulebook
    /// "Damage Resolution"): blood burned, torpor, or burned outright.
    fn apply_damage(&mut self) {
        let c = self.combat.as_mut().expect("combat context");
        let results: Vec<Resolved> = c
            .sides
            .iter()
            .map(|s| resolve_damage(self.state.minions[s.minion.index()].blood, s.pending))
            .collect();
        let mut wounded = Vec::new();
        for (s, r) in c.sides.iter_mut().zip(results) {
            let m = &mut self.state.minions[s.minion.index()];
            m.blood -= r.blood_burned;
            s.lost_to_damage += r.blood_burned;
            s.pending = Damage::default();
            if r.burned {
                m.in_play = false;
                m.blood = 0;
            } else if r.wounded {
                m.in_torpor = true;
                wounded.push(s.minion);
            }
        }
        if !c.both_ready(&self.state) {
            c.round_over = true;
        }
        // "After this vampire goes to torpor" riders fire here (rules-qa
        // 6.4).
        if !wounded.is_empty() {
            self.host_torpor_riders(&wounded);
        }
    }

    fn new_round(&mut self) {
        let c = self.combat.as_mut().expect("combat context");
        c.round += 1;
        c.pair = 0;
        for s in &mut c.sides {
            s.new_round();
        }
        c.range = Range::Close;
        c.range_set = c.next_round_close;
        c.next_round_close = false;
        c.only_hand_strikes = false;
        c.last_maneuver = None;
        c.last_press = None;
        c.press_continue = false;
        c.round_over = false;
        c.continued = false;
        c.played_round.clear();
        // After-combat riders are lost when the combat continues (spec
        // §4.9.2).
        if let Some(a) = self.action.as_mut() {
            a.continuation = None;
        }
        c.fresh_window(Step::BeforeRange);
    }

    /// Combat over: back to the action's after-combat window.
    fn end_combat(&mut self) {
        self.combat = None;
        self.after_combat();
    }

    /// Effects of a combat card resolved during the combat.
    pub(crate) fn apply_combat_effect(&mut self, effect: Effect, play: &PlayCtx) {
        let c = self.combat.as_mut().expect("combat context");
        let actor = play.actor.expect("combat card actor");
        let i = c.side_of_minion(actor).expect("actor in combat");
        match effect {
            Effect::Strike(k) => {
                c.sides[i].strike = Some(k);
                c.sides[i].chosen = true;
            }
            Effect::Maneuver => {
                if c.step == Step::DetermineRange {
                    c.maneuver(i);
                }
            }
            Effect::Press => {
                if c.step == Step::Press {
                    c.press(i);
                }
            }
            Effect::OptionalPress => {
                // "Press, or prevent with 1 optional press": in the press
                // step the card is the press itself.
                if c.step != Step::Press {
                    c.sides[i].presses += 1;
                }
            }
            Effect::NextRoundManeuver => c.sides[i].next_round_maneuvers += 1,
            Effect::AdditionalStrikes { n, limited } => {
                c.sides[i].additional += n;
                if limited {
                    c.sides[i].limited_used = true;
                }
            }
            Effect::SetStrength(n) => c.sides[i].strength_set = Some(n),
            Effect::AggravatedHands => c.sides[i].aggravated_hands = true,
            Effect::Prevent(n) => c.prevent(i, n),
            Effect::PreventXPlus(k) => c.prevent(i, play.x + k),
            Effect::PreventNonAggravated(n) => {
                c.sides[i].pending.prevent(n, false);
            }
            Effect::OnlyHandStrikes => c.only_hand_strikes = true,
            Effect::NextRoundClose => c.next_round_close = true,
            Effect::NewRound => c.continued = true,
            Effect::GainBloodLostByOpponent => {
                let gained = c.sides[1 - i].lost_to_damage;
                let m = &mut self.state.minions[actor.index()];
                m.blood = (m.blood + gained).min(m.capacity());
            }
            Effect::ContinueAfterCombat { blood, stealth } => {
                c.sides[i].continue_rider = Some((blood, stealth));
            }
            other => unreachable!("not a combat effect: {other:?}"),
        }
    }
}

impl CombatCtx {
    /// Start a strike pair: the normal pair (both choose) or an additional
    /// one (only minions with additional strikes choose; the range is the
    /// round's).
    fn begin_pair(&mut self) {
        if self.step != Step::BeforeStrikes {
            self.pair += 1;
        }
        for s in &mut self.sides {
            s.strike = None;
            if self.pair == 0 {
                s.chosen = false;
            } else if s.additional > 0 {
                s.additional -= 1;
                s.chosen = false;
            } else {
                s.chosen = true;
            }
        }
        self.fresh_window(Step::ChooseStrike(0));
    }

    /// A prevention of `n` (aggravated allowed) on side `i`: a partial
    /// cover of mixed damage is the preventer's allocation (rules-qa 4.2).
    fn prevent(&mut self, i: usize, n: u8) {
        let d = self.sides[i].pending;
        if d.normal > 0 && d.aggravated > 0 && n < d.total() {
            self.step = Step::Allocate { side: i, n };
        } else {
            self.sides[i].pending.prevent(n, true);
        }
    }

    fn maneuver(&mut self, i: usize) {
        self.range = match self.range {
            Range::Close => Range::Long,
            Range::Long => Range::Close,
        };
        self.last_maneuver = Some(i);
    }

    /// A press: to continue if none stands, else to end the opponent's.
    fn press(&mut self, i: usize) {
        self.press_continue = !self.press_continue;
        self.last_press = Some(i);
    }
}

pub(crate) fn is_combat_effect(e: &Effect) -> bool {
    matches!(
        e,
        Effect::Strike(_)
            | Effect::Maneuver
            | Effect::Press
            | Effect::OptionalPress
            | Effect::NextRoundManeuver
            | Effect::AdditionalStrikes { .. }
            | Effect::SetStrength(_)
            | Effect::AggravatedHands
            | Effect::Prevent(_)
            | Effect::PreventXPlus(_)
            | Effect::PreventNonAggravated(_)
            | Effect::OnlyHandStrikes
            | Effect::NextRoundClose
            | Effect::NewRound
            | Effect::GainBloodLostByOpponent
            | Effect::ContinueAfterCombat { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(normal: u8, aggravated: u8) -> Damage {
        Damage { normal, aggravated }
    }

    #[test]
    fn nassir_one_aggravated_wounds_without_burning_blood() {
        // Rulebook: "Nassir is ready and has 1 blood when he receives 1
        // point of aggravated damage. He cannot mend this damage, so he is
        // wounded and goes to torpor with 1 blood."
        let r = resolve_damage(1, d(0, 1));
        assert_eq!(
            r,
            Resolved {
                blood_burned: 0,
                wounded: true,
                burned: false
            }
        );
    }

    #[test]
    fn tamoszius_three_aggravated_on_two_blood() {
        // "Tamoszius is ready and has 2 blood when he receives 3 points of
        // aggravated damage. […] wounded by 1 point, and so he must burn 2
        // blood to prevent destruction from the other 2 points, leaving him
        // empty and in torpor."
        let r = resolve_damage(2, d(0, 3));
        assert_eq!(
            r,
            Resolved {
                blood_burned: 2,
                wounded: true,
                burned: false
            }
        );
    }

    #[test]
    fn ryan_normal_first_then_aggravated_burns() {
        // "Ryan is ready and has 1 blood when he receives 2 normal damage and
        // 1 aggravated damage. He burns 1 blood to mend the first point […]
        // becomes wounded. The aggravated damage burns him."
        let r = resolve_damage(1, d(2, 1));
        assert!(r.burned);
        assert_eq!(r.blood_burned, 1);
    }

    #[test]
    fn mended_damage_leaves_the_vampire_ready() {
        let r = resolve_damage(3, d(2, 0));
        assert_eq!(
            r,
            Resolved {
                blood_burned: 2,
                wounded: false,
                burned: false
            }
        );
    }

    #[test]
    fn partial_prevention_of_mixed_damage_is_the_preventers_allocation() {
        // rules-qa 4.2: 2 normal + 1 aggravated pending, prevent 2 — the
        // preventer may cover 0 or 1 aggravated point (going to torpor with
        // blood intact is a legitimate choice).
        let mut e = Engine::bare(2);
        let a = e.add_minion(SeatId(0), 0, 3, false);
        let x = e.add_minion(SeatId(1), 1, 3, false);
        e.start_combat(a, x);
        let c = e.combat.as_mut().unwrap();
        c.sides[1].pending = d(2, 1);
        c.sides[1].preventing = true;
        c.step = Step::Prevent(1);
        e.apply_combat_effect(
            Effect::Prevent(2),
            &PlayCtx {
                seat: SeatId(1),
                card: 0,
                mode: 0,
                actor: Some(x),
                target: cards::Target::None,
                x: 0,
                burn: None,
            },
        );
        let req = e.combat_request().unwrap();
        assert_eq!(req.window, WindowLabel::CombatPreventionAllocation);
        assert_eq!(req.options, vec![Choice::SetX(0), Choice::SetX(1)]);
        e.combat_apply(&Choice::SetX(0));
        assert_eq!(e.combat.as_ref().unwrap().sides[1].pending, d(0, 1));
        assert_eq!(e.combat.as_ref().unwrap().step, Step::Prevent(1));
    }

    #[test]
    fn prevention_takes_aggravated_first_when_allowed() {
        let mut p = d(2, 1);
        assert_eq!(p.prevent(2, true), 2);
        assert_eq!(p, d(1, 0));
        let mut q = d(2, 1);
        assert_eq!(q.prevent(4, false), 2);
        assert_eq!(q, d(0, 1));
    }
}
