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
use crate::library::{Label, LIBRARY};
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
    /// Of `normal`, the points from gun strikes (Kevlar Vest); a prevention
    /// not keyed on guns takes the other points first. No gun in the pool
    /// deals aggravated damage.
    pub gun: u8,
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
        self.gun = self.gun.min(self.normal);
        n - (left - b)
    }

    /// Prevent up to `n` points of gun-strike damage.
    pub(crate) fn prevent_gun(&mut self, n: u8) {
        let p = n.min(self.gun);
        self.gun -= p;
        self.normal -= p;
    }

    /// Prevent up to `n` points from other sources, aggravated first.
    fn prevent_non_gun(&mut self, n: u8) {
        let a = n.min(self.aggravated);
        self.aggravated -= a;
        let b = (n - a).min(self.normal - self.gun);
        self.normal -= b;
    }
}

/// A combat card's delayed effect "put this card on …": on the opposing
/// minion as its strike lands, or on its minion after combat. The card went
/// to the ash heap as it resolved and stays there (only action cards wait in
/// limbo); the effect applies whether the card is there or not, represented
/// in play as an effect-only card (owner, 2026-09-26: rules-qa 21.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Held {
    pub card: u16,
    pub seat: SeatId,
    pub mode: u8,
    pub counters: u8,
    pub minion: MinionId,
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
    /// Optional maneuvers usable in any round of this combat (Spirit's
    /// Touch [AUS] if this vampire blocks).
    pub combat_maneuvers: u8,
    /// Damage this side can prevent this round, at will (Precognition
    /// [AUS] in the first round).
    pub round_prevents: u8,
    /// This side can strike: combat ends in this round (Night Terrors
    /// [OBF][PRE]: the first).
    pub combat_ends_round: Option<u8>,
    /// This minion cannot use equipment this combat (Form of the Bat).
    pub no_equipment: bool,
    /// Form of Mist [PRO] rider, armed by the play, fired by the strike.
    pub continue_rider: Option<(u8, u8)>,
    /// The card in play the chosen strike comes from (a weapon).
    pub strike_source: Option<CardId>,
    /// The opposing minion's retainer this ranged strike is aimed at, at
    /// long range (rulebook "Retainers and Combat"): announced with it.
    pub strike_at: Option<CardId>,
    /// The initial strike this round must be this card's (a weapon's
    /// maneuver used, Sniper Rifle's range set): rulebook "Choose Strike".
    pub committed: Option<CardId>,
    /// The additional strike granted by this weapon is only usable to
    /// strike with it, and may be declined (AK-47).
    pub additional_with: Option<CardId>,
    /// Optional presses only usable to continue combat (Righteous Blade).
    pub continue_presses: u8,
    /// Optional presses usable in any round of this combat (Hunter's
    /// Mark).
    pub combat_presses: u8,
    /// This side cannot strike: combat ends in this round (Hunter's Mark
    /// [CEL][THA] on the opposing minion: the first).
    pub no_combat_ends_round: Option<u8>,
    /// Strength this side can gain for the combat before range is
    /// determined in the first round (Show of Force): `Choice::GainStrength`.
    pub strength_option: u8,
    /// A mandatory press, only usable to continue combat (Adrino
    /// Manauara): its controller cannot pass the press step while it can be
    /// used (rules-qa 14.1).
    pub mandatory_press: u8,
    /// "+n strength that combat" (Kasim Bayar), kept each round.
    pub strength_mods: u8,
    /// Riders of the chosen strike (Blood Rage, Soul Burn), fired as it
    /// lands: `Effect::FortitudeProofStrike`, `NullifyOpposingWeapons`.
    pub fortitude_proof: bool,
    pub nullifies_weapons: bool,
    /// This round, this minion cannot play prevention cards requiring
    /// Fortitude (a Fortitude-proof strike landed on it).
    pub no_fortitude_prevention: bool,
    /// This round, this vampire's weapon strikes inflict no damage.
    pub weapons_harmless: bool,
    /// A strike this minion can choose this round (Hunger of Marduk).
    pub round_strike: Option<StrikeKind>,
    /// Environmental damage the opposing minion takes each round, this
    /// combat (Carrion Crows).
    pub environmental: u8,
    /// Riders of the chosen strike: undodgeable (Dust Up [ani]),
    /// aggravated (Sculpt the Flesh [pro]), its card put on the opposing
    /// minion as it lands (`held`).
    pub undodgeable_strike: bool,
    pub aggravated_strike: bool,
    pub held: Option<Held>,
    /// +n strength this round (Bear's Skin [ani][pro]).
    pub strength_round: u8,
    /// Damage this side can prevent each round of the combat (Bear's Skin
    /// [ANI][PRO]), added to `round_prevents` as each round begins.
    pub prevent_each_round: u8,
    /// Of `pending`, the damage from the opposing minion's strikes.
    pub strike_damage: Damage,
    /// This round, the opposing minion's strikes deal this side no damage
    /// (Rolling with the Punches [FOR]).
    pub strikes_prevented: bool,
    /// Sent to torpor (or burned, an ally) by the opposing strike as the
    /// damage applies (Touch of Oblivion [OBL]).
    pub sent_to_torpor: bool,
    /// The round's initial strike, committed by a card played at the
    /// maneuver (Slam [POT]).
    pub committed_card: Option<StrikeKind>,
    /// The next additional strike is only usable as this strike (Voracious
    /// Vermin [ANI]).
    pub additional_only: Option<StrikeKind>,
    /// The opposing weapon a "burn weapon" strike is aimed at.
    pub strike_weapon: Option<CardId>,
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
            combat_maneuvers: 0,
            round_prevents: 0,
            no_equipment: false,
            combat_ends_round: None,
            continue_rider: None,
            strike_source: None,
            strike_at: None,
            committed: None,
            additional_with: None,
            continue_presses: 0,
            combat_presses: 0,
            no_combat_ends_round: None,
            strength_option: 0,
            mandatory_press: 0,
            strength_mods: 0,
            fortitude_proof: false,
            nullifies_weapons: false,
            no_fortitude_prevention: false,
            weapons_harmless: false,
            round_strike: None,
            environmental: 0,
            undodgeable_strike: false,
            aggravated_strike: false,
            held: None,
            strength_round: 0,
            prevent_each_round: 0,
            strike_damage: Damage::default(),
            strikes_prevented: false,
            sent_to_torpor: false,
            committed_card: None,
            additional_only: None,
            strike_weapon: None,
        }
    }

    /// Damage from the opposing minion's strike, unless those are prevented
    /// this round (Rolling with the Punches [FOR]); `gun`: from a gun.
    fn strike_hit(&mut self, n: u8, aggravated: bool, gun: bool) {
        if self.strikes_prevented {
            return;
        }
        self.pending.add(n, aggravated);
        self.strike_damage.add(n, aggravated);
        if gun {
            self.pending.gun += n;
            self.strike_damage.gun += n;
        }
    }

    /// A new strike chosen: the old one's riders are gone.
    fn set_strike(&mut self, k: Option<StrikeKind>) {
        self.strike = k;
        self.strike_source = None;
        self.fortitude_proof = false;
        self.nullifies_weapons = false;
        self.undodgeable_strike = false;
        self.aggravated_strike = false;
        self.strike_weapon = None;
    }

    fn new_round(&mut self) {
        let maneuvers = self.next_round_maneuvers;
        *self = Self {
            strength_set: self.strength_set,
            strength_mods: self.strength_mods,
            combat_maneuvers: self.combat_maneuvers,
            combat_presses: self.combat_presses,
            no_combat_ends_round: self.no_combat_ends_round,
            no_equipment: self.no_equipment,
            combat_ends_round: self.combat_ends_round,
            environmental: self.environmental,
            prevent_each_round: self.prevent_each_round,
            round_prevents: self.prevent_each_round,
            continue_rider: None,
            maneuvers,
            ..Self::new(self.minion, self.seat)
        };
    }

    /// Rulebook "Hand Strike": the minion's strength (`base`: a vampire's
    /// default 1, an ally's printed strength) unless set by an effect.
    pub fn strength(&self, base: u8) -> u8 {
        self.strength_set.unwrap_or(base)
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
    /// "Neither combatant can strike" this round (One With the Land
    /// [PRO]: the first round).
    pub no_strikes_round: Option<u8>,
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
    pub abilities_used: Vec<(cards::AbilitySource, u8)>,
    /// Abilities used this round ("each round", Rowena).
    pub abilities_used_round: Vec<(cards::AbilitySource, u8)>,
    /// Gun-strike prevention left this combat, by copy (Kevlar Vest).
    pub gun_prevention_left: Vec<(CardId, u8)>,
    /// The equipment on the blocker when it blocked (Sniper Rifle).
    pub held_at_block: Vec<CardId>,
    /// Begun by a successful action's resolution (entering combat as a Ⓓ
    /// action), not by a block: part of the action (spec §4.1.2), its
    /// after-combat window leading to after resolution (rules-qa 17.3).
    pub at_resolution: bool,
    /// Cards to put on their minion after combat ends (Pass Through Shadow
    /// [OBL]), from the ash heap; not if the combat continues.
    pub after_combat: Vec<Held>,
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
    /// source per round, only hand strikes under a grapple, one strike
    /// commitment a round (rulebook "Determine Range": no second strike
    /// card or weapon to maneuver again), a range not set twice.
    pub(crate) fn allows(&self, st: &GameState, mode: &cards::ModeDef, side: usize) -> bool {
        // "Minions in combat with the employer cannot strike: combat ends"
        // (Dog Pack; Faruq Abd al-Qadir for minions with his Methuselah's
        // corruption counters).
        let no_ends = st.combat_ends_barred(self.sides[side].minion, self.sides[1 - side].minion)
            || self.sides[side].no_combat_ends_round == Some(self.round);
        mode.effects.iter().all(|e| match e {
            Effect::Strike(StrikeKind::CombatEnds { .. }) if no_ends => false,
            Effect::Maneuver if self.step == Step::DetermineRange => {
                self.last_maneuver != Some(side)
            }
            // Nor the maneuver of a strike that cannot be used (LSJ
            // 20001126).
            Effect::CommitStrike => {
                self.sides[side].committed.is_none() && self.no_strikes_round != Some(self.round)
            }
            Effect::SetRangeLong => !self.range_set,
            // A strike card played at the maneuver: a maneuver in turn, one
            // commitment a round, a strike that can be used (LSJ 20021028).
            Effect::CommittedStrike(k) if self.step == Step::DetermineRange => {
                self.last_maneuver != Some(side)
                    && self.sides[side].committed.is_none()
                    && self.sides[side].committed_card.is_none()
                    && self.no_strikes_round != Some(self.round)
                    && (!self.only_hand_strikes || matches!(k, StrikeKind::Hand { .. }))
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

/// An ally (or retainer) treats aggravated damage as normal: 1 life burned
/// per point, burned with its last life (rulebook "Damage Resolution",
/// advanced rules box) — never wounded, allies have no torpor (advanced
/// rules 5.3.3). Life stolen to 0 burns it too.
pub(crate) fn resolve_life_damage(life: u8, d: Damage) -> Resolved {
    let lost = life.min(d.total());
    Resolved {
        blood_burned: lost,
        wounded: false,
        burned: life == lost,
    }
}

impl Engine {
    /// Block resolution produced a combat: `acting` is the acting minion,
    /// `opposing` the blocker (rules-qa 1.2: the opponent is a "blocker"
    /// only if it actually blocked).
    pub(crate) fn start_combat(&mut self, acting: MinionId, opposing: MinionId) {
        self.begin_combat(acting, opposing, false);
    }

    /// A successful action enters combat with its target (Theo Bell).
    pub(crate) fn start_combat_at_resolution(&mut self, acting: MinionId, opposing: MinionId) {
        self.begin_combat(acting, opposing, true);
    }

    fn begin_combat(&mut self, acting: MinionId, opposing: MinionId, at_resolution: bool) {
        let st = &self.state;
        let mut sides = [
            Side::new(acting, st.minions[acting.index()].owner),
            Side::new(opposing, st.minions[opposing.index()].owner),
        ];
        for s in &mut sides {
            s.mandatory_press = st
                .printed_mod(
                    s.minion,
                    crate::cards::Stat::MandatoryContinuePress,
                    crate::state::PrintedCtx::default(),
                )
                .max(0) as u8;
        }
        let order = vec![sides[0].seat, sides[1].seat];
        self.state.combatants = Some([acting, opposing]);
        let st = &self.state;
        let held_at_block = if at_resolution {
            Vec::new()
        } else {
            st.equipment_on(opposing)
        };
        self.combat = Some(CombatCtx {
            sides,
            round: 1,
            pair: 0,
            range: Range::Close,
            range_set: false,
            next_round_close: false,
            only_hand_strikes: false,
            no_strikes_round: None,
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
            abilities_used_round: Vec::new(),
            gun_prevention_left: Vec::new(),
            held_at_block,
            at_resolution,
            after_combat: Vec::new(),
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
                // A committed strike: that card's strike only — none at all
                // if an effect now bars it (spec §4.3.1); AK-47's additional
                // strike only with the gun, or none.
                let committed = if c.pair == 0 {
                    side.committed
                } else {
                    side.additional_with
                };
                if let Some(w) = committed {
                    let mut options: Vec<Choice> = self
                        .ability_options(side.seat, Window::CombatStrikeDeclaration)
                        .into_iter()
                        .filter(|o| {
                            matches!(o, Choice::UseAbility { source: cards::AbilitySource::Card(c), .. } if *c == w)
                        })
                        .collect();
                    if c.pair > 0 {
                        options.insert(0, Choice::Pass);
                    }
                    if options.is_empty() {
                        return None;
                    }
                    options.extend(self.ability_options(side.seat, Window::AnyImpulse));
                    let options = self.with_retainer_targets(i, options);
                    return Some(DecisionRequest {
                        seat: side.seat,
                        window: WindowLabel::CombatStrike,
                        options,
                    });
                }
                // The initial strike a card committed at the maneuver
                // (Slam [POT]): that strike only.
                if c.pair == 0 {
                    if let Some(k) = side.committed_card {
                        let mut options = Vec::new();
                        if !c.only_hand_strikes || matches!(k, StrikeKind::Hand { .. }) {
                            options.push(Choice::CommittedStrike);
                        }
                        if options.is_empty() {
                            return None;
                        }
                        options.extend(self.ability_options(side.seat, Window::AnyImpulse));
                        return Some(DecisionRequest {
                            seat: side.seat,
                            window: WindowLabel::CombatStrike,
                            options,
                        });
                    }
                }
                // An additional strike only usable to burn a weapon
                // (Voracious Vermin [ANI]): none if no weapon is left.
                if c.pair > 0 && side.additional_only == Some(StrikeKind::BurnWeapon) {
                    let opposing = c.sides[1 - i].minion;
                    let mut options = Vec::new();
                    if !c.only_hand_strikes {
                        options.extend(
                            self.state
                                .equipment_on(opposing)
                                .into_iter()
                                .filter(|&w| {
                                    let card = self.state.cards[w.index()].card;
                                    LIBRARY[card as usize].labels.contains(&Label::Weapon)
                                        || cards::becomes_weapon(card)
                                })
                                .map(|w| Choice::BurnWeaponStrike { weapon: w.0 }),
                        );
                    }
                    if options.is_empty() {
                        return None;
                    }
                    options.extend(self.ability_options(side.seat, Window::AnyImpulse));
                    return Some(DecisionRequest {
                        seat: side.seat,
                        window: WindowLabel::CombatStrike,
                        options,
                    });
                }
                let mut options = vec![Choice::HandStrike];
                // A granted "strike: combat ends" this round, unless combat
                // ends strikes or strikes other than hand strikes are
                // barred (Dog Pack, a grapple).
                if side.combat_ends_round == Some(c.round)
                    && side.no_combat_ends_round != Some(c.round)
                    && !c.only_hand_strikes
                    && !self
                        .state
                        .combat_ends_barred(c.sides[i].minion, c.sides[1 - i].minion)
                {
                    options.push(Choice::StrikeCombatEnds);
                }
                // A strike granted for the round (Hunger of Marduk).
                if let Some(k) = side.round_strike {
                    if !c.only_hand_strikes || matches!(k, StrikeKind::Hand { .. }) {
                        options.push(Choice::RoundStrike { at: None });
                    }
                }
                options.extend(self.play_options(side.seat, &c.windows(), None));
                // Strikes a card in play gives its bearer (Treasured
                // Samadji's dodge).
                options.extend(self.ability_options(side.seat, Window::CombatStrikeDeclaration));
                options.extend(self.ability_options(side.seat, Window::AnyImpulse));
                let options = self.with_retainer_targets(i, options);
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
                if side.round_prevents > 0 {
                    options.push(Choice::UsePrevent);
                }
                options.extend(self.play_options(side.seat, &c.windows(), None));
                for w in c.windows() {
                    options.extend(self.ability_options(side.seat, w));
                }
                options.extend(self.ability_options(side.seat, Window::AnyImpulse));
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
                let mut mandatory = false;
                match c.step {
                    Step::DetermineRange
                        if (c.sides[i].maneuvers > 0 || c.sides[i].combat_maneuvers > 0)
                            && c.last_maneuver != Some(i) =>
                    {
                        options.push(Choice::UseManeuver);
                    }
                    // Show of Force: the strength, before range is
                    // determined in the first round.
                    Step::BeforeRange if c.round == 1 && c.sides[i].strength_option > 0 => {
                        options.push(Choice::GainStrength);
                    }
                    Step::Press
                        if c.last_press != Some(i)
                            && (c.sides[i].presses > 0
                                || c.sides[i].combat_presses > 0
                                || ((c.sides[i].continue_presses > 0
                                    || c.sides[i].mandatory_press > 0)
                                    && !c.press_continue)) =>
                    {
                        options.push(Choice::UsePress);
                        // Mandatory only while it can be used: to continue,
                        // with both combatants ready (rules-qa 14.1).
                        mandatory = c.sides[i].mandatory_press > 0
                            && !c.press_continue
                            && c.both_ready(&self.state);
                    }
                    _ => {}
                }
                options.extend(self.play_options(seat, &c.windows(), None));
                for w in c.windows() {
                    options.extend(self.ability_options(seat, w));
                }
                options.extend(self.ability_options(seat, Window::AnyImpulse));
                if mandatory {
                    // Used before its controller passes (spec §1.1.2): a
                    // press card played to continue first leaves it unused,
                    // and it waits for a later round (rules-qa 14.1).
                    options.retain(|o| *o != Choice::Pass);
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
            (Step::ChooseStrike(i), Choice::StrikeCombatEnds) => {
                c.sides[i].set_strike(Some(StrikeKind::CombatEnds { unlock: false }));
                c.sides[i].chosen = true;
                None
            }
            (Step::ChooseStrike(i), Choice::HandStrike) => {
                c.sides[i].set_strike(Some(StrikeKind::Hand { bonus: 0 }));
                c.sides[i].chosen = true;
                None
            }
            (Step::ChooseStrike(i), Choice::CommittedStrike) => {
                let k = c.sides[i].committed_card.take();
                c.sides[i].set_strike(k);
                c.sides[i].chosen = true;
                None
            }
            (Step::ChooseStrike(i), Choice::BurnWeaponStrike { weapon }) => {
                c.sides[i].set_strike(Some(StrikeKind::BurnWeapon));
                c.sides[i].strike_weapon = Some(CardId(*weapon));
                c.sides[i].additional_only = None;
                c.sides[i].chosen = true;
                None
            }
            (Step::ChooseStrike(i), Choice::RoundStrike { at }) => {
                let k = c.sides[i].round_strike;
                c.sides[i].set_strike(k);
                c.sides[i].strike_at = *at;
                c.sides[i].chosen = true;
                None
            }
            // An optional additional strike declined.
            (Step::ChooseStrike(i), Choice::Pass) => {
                c.sides[i].set_strike(None);
                c.sides[i].additional_only = None;
                c.sides[i].additional_with = None;
                c.sides[i].chosen = true;
                None
            }
            (Step::ChooseStrike(i), Choice::Play { .. }) => Some(c.sides[i].seat),
            (Step::Prevent(i), Choice::Pass) => {
                c.sides[i].preventing = false;
                None
            }
            (Step::Prevent(i), Choice::UsePrevent) => {
                c.sides[i].round_prevents -= 1;
                c.prevent(i, 1);
                None
            }
            (Step::Allocate { side, n }, Choice::SetX(k)) => {
                let d = &mut c.sides[side].pending;
                d.aggravated -= k;
                d.normal -= n - k;
                d.gun = d.gun.min(d.normal);
                c.step = Step::Prevent(side);
                None
            }
            (Step::Prevent(i), Choice::Play { .. }) => Some(c.sides[i].seat),
            // An ability is not a card play: no as-played window.
            (Step::Prevent(_) | Step::ChooseStrike(_), Choice::UseAbility { .. }) => None,
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
                // This round's banked maneuver before the combat-long one.
                if c.sides[i].maneuvers > 0 {
                    c.sides[i].maneuvers -= 1;
                } else {
                    c.sides[i].combat_maneuvers -= 1;
                }
                c.window.play();
                c.maneuver(i);
                None
            }
            (Step::Press, Choice::UsePress) => {
                let i = c
                    .side_of(c.window.current().expect("impulse"))
                    .expect("side");
                // A press only usable to continue goes first when
                // continuing, the mandatory one before the optional.
                if !c.press_continue && c.sides[i].mandatory_press > 0 {
                    c.sides[i].mandatory_press -= 1;
                } else if !c.press_continue && c.sides[i].continue_presses > 0 {
                    c.sides[i].continue_presses -= 1;
                } else if c.sides[i].presses > 0 {
                    c.sides[i].presses -= 1;
                } else {
                    c.sides[i].combat_presses -= 1;
                }
                c.window.play();
                c.press(i);
                None
            }
            (Step::BeforeRange, Choice::GainStrength) => {
                let i = c
                    .side_of(c.window.current().expect("impulse"))
                    .expect("side");
                let n = std::mem::take(&mut c.sides[i].strength_option);
                c.sides[i].strength_mods += n;
                c.window.play();
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
            source,
            ability,
            actor,
            target,
        } = choice
        {
            self.use_ability(*source, *ability, *actor, *target, Flow::Combat);
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
        if self.apply_damage() {
            return;
        }
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
        // A combat ends strike against a minion whose retainer bars it has
        // no effect (Dog Pack put on it after the strike was chosen, ANK
        // 20190412).
        let barred: [bool; 2] = std::array::from_fn(|i| {
            self.state
                .combat_ends_barred(c.sides[i].minion, c.sides[1 - i].minion)
                || c.sides[i].no_combat_ends_round == Some(c.round)
        });
        let ends: Vec<usize> = (0..2)
            .filter(|&i| !barred[i] && matches!(strikes[i], Some(StrikeKind::CombatEnds { .. })))
            .collect();
        if !ends.is_empty() {
            for i in ends {
                let Some(StrikeKind::CombatEnds { unlock }) = strikes[i] else {
                    unreachable!()
                };
                let m = c.sides[i].minion;
                if unlock && !self.action.as_ref().is_some_and(|a| a.no_unlock) {
                    // "Unlock this vampire before combat ends": resolves with
                    // the strike, so it survives a continued combat (spec
                    // §4.9.2).
                    self.state.minions[m.index()].locked = false;
                }
                if let Some((blood, stealth)) = c.sides[i].continue_rider.take() {
                    // Form of Mist [PRO]: only the acting minion "was
                    // blocked".
                    if self
                        .action
                        .as_ref()
                        .is_some_and(|a| a.acting == c.sides[i].minion)
                    {
                        let a = self.action.as_mut().expect("action context");
                        a.continuation = Some((blood, stealth));
                    }
                }
            }
            c.round_over = true;
            for s in &mut c.sides {
                s.set_strike(None);
                s.strike_at = None;
            }
            // Combat ended before the other strike resolved: a card to be
            // put in play as it lands stays in the ash heap.
            self.discard_held(false);
            return;
        }
        // A strike from a card no longer on the striker has no effect (spec
        // §4.10.4).
        let st = &self.state;
        let strikes: [Option<StrikeKind>; 2] = std::array::from_fn(|i| {
            let s = &c.sides[i];
            match s.strike_source {
                Some(w)
                    if !(st.cards[w.index()].present()
                        && st.cards[w.index()].bearer() == Some(s.minion)) =>
                {
                    None
                }
                _ => strikes[i],
            }
        });
        // Printed strength (Massimiliano; Kevin Jackson's, on him and on a
        // Brujah opposing him): added to the strength of 1 or to a set
        // strength (spec §4.4.1).
        let strength: [u8; 2] = std::array::from_fn(|i| {
            let ctx = crate::state::PrintedCtx {
                opponent: Some(c.sides[1 - i].minion),
                ..Default::default()
            };
            // The acting minion's strength this action (Invigorate).
            let action_bonus = match self.action.as_ref() {
                Some(a) if a.acting == c.sides[i].minion => a.strength_bonus as i32,
                _ => 0,
            };
            // Strength from cards in play on the minion (Saulot's
            // Avenging Fist).
            let bearer = self.action.as_ref().map_or(0, |a| {
                st.bearer_mod(c.sides[i].minion, cards::Stat::Strength, a.kind)
            });
            (c.sides[i].strength(st.base_strength(c.sides[i].minion)) as i32
                + action_bonus
                + c.sides[i].strength_mods as i32
                + bearer
                + st.printed_mod(c.sides[i].minion, cards::Stat::Strength, ctx))
            .max(0) as u8
                + c.sides[i].strength_round
        });
        let close = c.range == Range::Close;
        // Ranged strikes aimed at a retainer: (retainer, amount, steal,
        // striking side), applied once the strikes are read.
        let mut retainer_hits: Vec<(CardId, u8, bool, usize)> = Vec::new();
        // Retainer damage, in the normal strike pair only (not repeated by
        // additional strikes, RTR 19941109), dodge or not (rulebook
        // "Environmental damage").
        if c.pair == 0 {
            for i in 0..2 {
                let n = st.retainer_damage(c.sides[i].minion);
                c.sides[1 - i].pending.add(n, false);
                // Environmental damage (Carrion Crows): dodge or not.
                let n = c.sides[i].environmental;
                c.sides[1 - i].pending.add(n, false);
            }
        }
        // This pair's damage from the opposing strikes, apart.
        for s in &mut c.sides {
            s.strike_damage = Damage::default();
        }
        // A dodge has no effect on a strike that cannot be dodged
        // (Aggressive Corpse, Dust Up [ani]; LSJ 20030902-2).
        let dodged: [bool; 2] = std::array::from_fn(|i| {
            strikes[1 - i] == Some(StrikeKind::Dodge)
                && !st.undodgeable(c.sides[i].minion)
                && !c.sides[i].undodgeable_strike
        });
        // Does the strike land — in range, not dodged, not aimed at a
        // retainer?
        let lands: [bool; 2] = std::array::from_fn(|i| {
            c.sides[i].strike_at.is_none()
                && !dodged[i]
                && match strikes[i] {
                    Some(StrikeKind::Hand { .. } | StrikeKind::Melee { .. }) => close,
                    Some(
                        StrikeKind::Damage { ranged, .. } | StrikeKind::StealBlood { ranged, .. },
                    ) => ranged || close,
                    // A strike works at close range unless it says ranged
                    // (owner, 2026-09-26: rules-qa 21.1).
                    Some(StrikeKind::TorporOrBurn | StrikeKind::CardEffect) => close,
                    Some(StrikeKind::BurnWeapon) => true,
                    Some(StrikeKind::BurnEquipment) => close,
                    Some(StrikeKind::Dodge | StrikeKind::CombatEnds { .. }) | None => false,
                }
        });
        // A card the strike puts on the opposing minion, as it lands (else
        // it stays in the ash heap).
        let mut placed: Vec<(Held, MinionId)> = Vec::new();
        for (i, &landed) in lands.iter().enumerate() {
            if let Some(h) = c.sides[i].held.take() {
                if landed {
                    placed.push((h, c.sides[1 - i].minion));
                }
            }
        }
        // Strike riders fire as the strike lands (LSJ 20040928) and reach
        // the weapon strikes resolving with it (ANK 20170519): Blood Rage,
        // Soul Burn.
        for (i, &landed) in lands.iter().enumerate() {
            let j = 1 - i;
            let s = &c.sides[i];
            if !(s.fortitude_proof || s.nullifies_weapons) {
                continue;
            }
            if landed {
                let (fp, nw) = (s.fortitude_proof, s.nullifies_weapons);
                // "The opposing vampire's" weapons: not an ally's.
                let vampire = !st.minions[c.sides[j].minion.index()].is_ally();
                c.sides[j].no_fortitude_prevention |= fp;
                c.sides[j].weapons_harmless |= nw && vampire;
            }
        }
        // Weapon strikes that inflict no damage this round.
        let harmless: [bool; 2] = std::array::from_fn(|i| {
            c.sides[i].weapons_harmless
                && c.sides[i].strike_source.is_some_and(|w| {
                    let card = st.cards[w.index()].card;
                    LIBRARY[card as usize].labels.contains(&Label::Weapon)
                        || cards::becomes_weapon(card)
                })
        });
        let mut steals = [0u8; 2];
        let mut burn_weapons: Vec<CardId> = Vec::new();
        // Weapons losing counters per damage inflicted (Weighted Walking
        // Stick): the card and the points.
        let mut counter_hits: Vec<(CardId, u8)> = Vec::new();
        for i in 0..2 {
            let j = 1 - i;
            let Some(k) = strikes[i] else {
                continue;
            };
            // "After the bearer strikes with this gun" (AK-47): struck,
            // dodged or not; one limited additional-strike source a round.
            if let Some(w) = c.sides[i].strike_source {
                let def = cards::lookup(st.cards[w.index()].card).expect("definition");
                let rider = def
                    .abilities
                    .iter()
                    .any(|m| m.effects.contains(&Effect::AdditionalStrikeWithSource));
                if rider && !c.sides[i].limited_used {
                    c.sides[i].additional += 1;
                    c.sides[i].limited_used = true;
                    c.sides[i].additional_with = Some(w);
                }
            }
            let dodged = dodged[i];
            // Aimed at a retainer: a dodge does not protect it (rulebook
            // "Dodge"); gone or moved by now, the strike has no effect.
            if let Some(r) = c.sides[i].strike_at {
                let rc = &st.cards[r.index()];
                if rc.present() && rc.host == Some(c.sides[j].minion) {
                    match k {
                        StrikeKind::Damage { .. } if harmless[i] => {}
                        StrikeKind::Damage { n, .. } => {
                            let ctx = crate::state::PrintedCtx {
                                opponent: Some(c.sides[j].minion),
                                ..Default::default()
                            };
                            let more = st.printed_mod(
                                c.sides[i].minion,
                                cards::Stat::RangedStrikeDamage,
                                ctx,
                            );
                            retainer_hits.push((r, (n as i32 + more).max(0) as u8, false, i));
                        }
                        StrikeKind::StealBlood { n, .. } => retainer_hits.push((r, n, true, i)),
                        _ => {}
                    }
                }
                continue;
            }
            match k {
                StrikeKind::Hand { bonus } => {
                    if close && !dodged {
                        let n = strength[i] + bonus;
                        let aggravated = c.sides[i].aggravated_hands
                            || c.sides[i].aggravated_strike
                            || self.action.as_ref().is_some_and(|a| {
                                a.aggravated_hands && a.acting == c.sides[i].minion
                            });
                        c.sides[j].strike_hit(n, aggravated, false);
                    }
                }
                StrikeKind::Melee { bonus, aggravated } => {
                    if close && !dodged && !harmless[i] {
                        let n = strength[i] + bonus;
                        c.sides[j].strike_hit(n, aggravated, false);
                        // Inflicted, prevented or not (LSJ 20100519).
                        if let Some(w) = c.sides[i].strike_source {
                            let per_damage =
                                cards::lookup(st.cards[w.index()].card).is_some_and(|d| {
                                    d.abilities
                                        .iter()
                                        .any(|m| m.effects.contains(&Effect::CountersPerDamage))
                                });
                            if per_damage {
                                counter_hits.push((w, n));
                            }
                        }
                    }
                }
                StrikeKind::Damage {
                    n,
                    ranged,
                    aggravated,
                } => {
                    if (ranged || close) && !dodged && !harmless[i] {
                        // "+1 damage with ranged strikes (even at close
                        // range)" (Noluthando): part of the strike's damage.
                        let n = if ranged {
                            let ctx = crate::state::PrintedCtx {
                                opponent: Some(c.sides[j].minion),
                                ..Default::default()
                            };
                            let more = st.printed_mod(
                                c.sides[i].minion,
                                cards::Stat::RangedStrikeDamage,
                                ctx,
                            );
                            (n as i32 + more).max(0) as u8
                        } else {
                            n
                        };
                        let gun = c.sides[i].strike_source.is_some_and(|w| {
                            LIBRARY[st.cards[w.index()].card as usize]
                                .labels
                                .contains(&Label::Gun)
                        });
                        c.sides[j].strike_hit(n, aggravated, gun && !aggravated);
                    }
                }
                StrikeKind::StealBlood { n, ranged } => {
                    if (ranged || close) && !dodged {
                        steals[i] = n.min(self.state.minions[c.sides[j].minion.index()].blood);
                    }
                }
                StrikeKind::TorporOrBurn => {
                    if close && !dodged {
                        c.sides[j].sent_to_torpor = true;
                    }
                }
                StrikeKind::BurnWeapon | StrikeKind::BurnEquipment => {
                    let reaches = k == StrikeKind::BurnWeapon || close;
                    if !dodged && reaches {
                        if let Some(w) = c.sides[i].strike_weapon {
                            let wc = &st.cards[w.index()];
                            if wc.present() && wc.host == Some(c.sides[j].minion) {
                                burn_weapons.push(w);
                            }
                        }
                    }
                }
                StrikeKind::Dodge | StrikeKind::CombatEnds { .. } | StrikeKind::CardEffect => {}
            }
        }
        // Simultaneous steals: amounts from the pre-strike state, capacity
        // applied after both moves (Theft of Vitae rulings).
        for (i, &n) in steals.iter().enumerate() {
            self.state.minions[c.sides[1 - i].minion.index()].blood -= n;
        }
        for (i, &n) in steals.iter().enumerate() {
            self.state.gain_blood(c.sides[i].minion, n);
        }
        // A retainer burns life for damage (aggravated alike, rulebook
        // "Damage Resolution"), and life stolen from it becomes blood.
        let mut lifeless = Vec::new();
        for (r, n, steal, i) in retainer_hits {
            let card = &mut self.state.cards[r.index()];
            let taken = n.min(card.counters);
            card.counters -= taken;
            if steal {
                self.state.gain_blood(c.sides[i].minion, taken);
            }
            if self.state.cards[r.index()].counters == 0 {
                lifeless.push(r);
            }
        }
        for s in &mut c.sides {
            s.set_strike(None);
            s.strike_at = None;
            s.preventing = s.pending.total() > 0;
        }
        for r in lifeless {
            self.burn_card_in_play(r);
        }
        for w in burn_weapons {
            self.burn_card_in_play(w);
        }
        for (w, n) in counter_hits {
            let card = &mut self.state.cards[w.index()];
            card.counters = card.counters.saturating_sub(n);
            if card.counters == 0 && card.in_play {
                self.burn_card_in_play(w);
            }
        }
        for (h, on) in placed {
            self.place_held(h, on);
        }
    }

    /// The delayed effect applies on minion `on`, if still in play: an
    /// effect-only card in play (its counters, its mode's layers), the card
    /// itself left in the ash heap.
    fn place_held(&mut self, h: Held, on: MinionId) {
        if !self.state.minions[on.index()].in_play {
            return;
        }
        let id = self.enter_play(crate::state::CardInPlay {
            card: h.card,
            owner: h.seat,
            controller: h.seat,
            host: Some(on),
            body: None,
            mode: h.mode,
            by: Some(h.minion),
            in_play: true,
            contested: false,
            locked: false,
            counters: h.counters,
            stored: Vec::new(),
            effect_only: true,
            against: Default::default(),
            entered: 0,
            chosen: Vec::new(),
        });
        let mode = &cards::lookup(h.card).expect("definition").modes[h.mode as usize];
        for e in mode.effects {
            if let Effect::Layer(modifier) = e {
                self.state.layers.push(cards::Layer {
                    scope: cards::Scope::InPlay,
                    modifier: *modifier,
                    source: id,
                });
            }
        }
    }

    /// Delayed placements that will not happen — a strike that did not
    /// resolve, after-combat effects of a combat that continued: the cards
    /// stay in the ash heap.
    fn discard_held(&mut self, after_combat: bool) {
        let c = self.combat.as_mut().expect("combat context");
        for s in &mut c.sides {
            s.held = None;
        }
        if after_combat {
            c.after_combat.clear();
        }
    }

    /// Damage applied at once to both sides after prevention (rulebook
    /// "Damage Resolution"): blood burned, torpor, or burned outright.
    /// Returns true when the flow moved on: a torpor rider (Fame) ousted a
    /// combatant's Methuselah mid-combat — the game ended, the active
    /// Methuselah's turn passed, or the opponent left the game and the
    /// combat ended at once (DESIGN §10).
    pub(crate) fn apply_damage(&mut self) -> bool {
        let c = self.combat.as_mut().expect("combat context");
        let results: Vec<Resolved> = c
            .sides
            .iter()
            .map(|s| {
                let m = &self.state.minions[s.minion.index()];
                if m.is_ally() {
                    resolve_life_damage(m.blood, s.pending)
                } else {
                    resolve_damage(m.blood, s.pending)
                }
            })
            .collect();
        let mut wounded = Vec::new();
        let mut burned = Vec::new();
        for (s, r) in c.sides.iter_mut().zip(results) {
            let m = &mut self.state.minions[s.minion.index()];
            m.blood -= r.blood_burned;
            s.lost_to_damage += r.blood_burned;
            s.pending = Damage::default();
            // Sent to torpor by the strike (Touch of Oblivion [OBL]): a
            // vampire wounded, an ally burned.
            let sent = std::mem::take(&mut s.sent_to_torpor);
            if r.burned || (sent && m.is_ally()) {
                burned.push(s.minion);
            } else if r.wounded || sent {
                wounded.push(s.minion);
            }
        }
        // Burns and torpors land at once, their riders with them (rules-qa
        // 6.4): a Methuselah may be ousted here.
        for &m in &burned {
            self.burn_minion(m);
        }
        if !wounded.is_empty() {
            self.minions_to_torpor(&wounded);
        }
        let c = self.combat.as_mut().expect("combat context");
        if !c.both_ready(&self.state) {
            c.round_over = true;
        }
        if !wounded.is_empty() || !burned.is_empty() {
            if self.state.living() <= 1 {
                self.combat = None;
                self.state.combatants = None;
                self.action = None;
                self.flow = Flow::GameOver;
                return true;
            }
            if self.state.seats[self.state.active.index()].ousted {
                self.next_turn();
                return true;
            }
            let c = self.combat.as_ref().expect("combat context");
            if c.sides
                .iter()
                .any(|s| self.state.seats[s.seat.index()].ousted)
            {
                // The opponent's Methuselah ousted: their minion left the
                // game with them, so the combat ends at once (no
                // end-of-round window) and the blocked action goes on
                // without them.
                let st = &self.state;
                let a = self.action.as_mut().expect("action context");
                a.order.retain(|s| !st.seats[s.index()].ousted);
                self.end_combat();
                return true;
            }
        }
        false
    }

    fn new_round(&mut self) {
        // The combat continues: after-combat effects are lost (RTR
        // 20020501).
        self.discard_held(true);
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
        c.abilities_used_round.clear();
        // After-combat riders are lost when the combat continues (spec
        // §4.9.2).
        if let Some(a) = self.action.as_mut() {
            a.continuation = None;
        }
        c.fresh_window(Step::BeforeRange);
    }

    /// "End a combat" outside strike resolution (Saulot's Guiding Wisdom):
    /// no strike resolves, damage prevention cannot be played, the round
    /// is over and its end-of-round window opens (advanced rules §4.7.2,
    /// §4.9: ending combat does not close that window).
    pub(crate) fn end_combat_now(&mut self) {
        self.discard_held(false);
        let c = self.combat.as_mut().expect("combat context");
        c.round_over = true;
        c.press_continue = false;
        for s in &mut c.sides {
            s.set_strike(None);
            s.committed = None;
            s.additional = 0;
        }
        c.fresh_window(Step::EndOfRound);
    }

    /// Combat over: back to the action's after-combat window — after
    /// which a combat the action's resolution began leads to the
    /// after-resolution window (rules-qa 17.3).
    fn end_combat(&mut self) {
        let ended = self.combat.take();
        // "After combat ends, put this card on this vampire" (Pass Through
        // Shadow [OBL]).
        for h in ended.iter().flat_map(|c| c.after_combat.clone()) {
            self.place_held(h, h.minion);
        }
        if ended.as_ref().is_some_and(|c| c.at_resolution) {
            self.action.as_mut().expect("action context").resolved = true;
        }
        // A hand size the combat lowered comes back: that Methuselah draws
        // up (Raptor [ANI]; LSJ 20030530).
        let lowered: Vec<SeatId> = ended
            .iter()
            .flat_map(|c| {
                (0..2).filter_map(|i| {
                    (self.state.combat_hand_penalty(c.sides[1 - i].minion) > 0)
                        .then_some(c.sides[i].seat)
                })
            })
            .collect();
        self.state.combatants = None;
        for s in lowered {
            if !self.state.seats[s.index()].ousted {
                self.draw_up(s);
            }
        }
        self.after_combat();
    }

    /// At long range, each ranged strike among `options` may instead be
    /// aimed at one of the opposing minion's retainers (rulebook "Retainers
    /// and Combat"): the same choice with that retainer as its target.
    fn with_retainer_targets(&self, i: usize, mut options: Vec<Choice>) -> Vec<Choice> {
        let c = self.combat.as_ref().expect("combat context");
        if c.range != Range::Long {
            return options;
        }
        let st = &self.state;
        let opposing = c.sides[1 - i].minion;
        let retainers: Vec<CardId> = (0..st.cards.len())
            .map(|k| CardId(k as u32))
            .filter(|r| {
                let card = &st.cards[r.index()];
                card.present()
                    && card.host == Some(opposing)
                    && LIBRARY[card.card as usize]
                        .types
                        .contains(&crate::library::CardType::Retainer)
            })
            .collect();
        if retainers.is_empty() {
            return options;
        }
        let ranged = |mode: &cards::ModeDef| {
            mode.effects.iter().any(|e| {
                matches!(
                    e,
                    Effect::Strike(
                        StrikeKind::Damage { ranged: true, .. }
                            | StrikeKind::StealBlood { ranged: true, .. }
                    )
                )
            })
        };
        let seat = c.sides[i].seat;
        let mut aimed = Vec::new();
        for o in &options {
            let mode = match *o {
                Choice::Play { card, mode, .. } => {
                    let id = st.seats[seat.index()].hand[card as usize];
                    cards::lookup(id).map(|d| &d.modes[mode as usize])
                }
                Choice::UseAbility {
                    source, ability, ..
                } => Some(source.mode(st, ability)),
                _ => None,
            };
            if !mode.is_some_and(ranged) {
                continue;
            }
            for &r in &retainers {
                aimed.push(match *o {
                    Choice::Play {
                        card, mode, actor, ..
                    } => Choice::Play {
                        card,
                        mode,
                        actor,
                        target: cards::Target::Card(r),
                    },
                    Choice::UseAbility {
                        source,
                        ability,
                        actor,
                        ..
                    } => Choice::UseAbility {
                        source,
                        ability,
                        actor,
                        target: cards::Target::Card(r),
                    },
                    _ => unreachable!(),
                });
            }
        }
        // The round's granted strike, if ranged.
        let round_ranged = matches!(
            c.sides[i].round_strike,
            Some(
                StrikeKind::Damage { ranged: true, .. }
                    | StrikeKind::StealBlood { ranged: true, .. }
            )
        );
        if round_ranged && options.contains(&Choice::RoundStrike { at: None }) {
            aimed.extend(
                retainers
                    .iter()
                    .map(|&r| Choice::RoundStrike { at: Some(r) }),
            );
        }
        options.extend(aimed);
        options
    }

    /// Effects of a combat card resolved during the combat.
    pub(crate) fn apply_combat_effect(&mut self, effect: Effect, play: &PlayCtx) {
        let c = self.combat.as_mut().expect("combat context");
        let actor = play.actor.expect("combat card actor");
        // An actor out of the combat plays for its Methuselah's combatant
        // (`Actor::OwnOutOfCombat`).
        let i = c
            .side_of_minion(actor)
            .or_else(|| c.side_of(play.seat))
            .expect("actor in combat");
        match effect {
            // A strike card's; `resolve_ability` names a card in play as
            // the source afterward.
            Effect::Strike(k) => {
                let initial = c.pair == 0;
                let s = &mut c.sides[i];
                s.set_strike(Some(k));
                // A card target: the retainer a ranged strike is aimed at,
                // or the equipment a burn-equipment strike names.
                match (k, play.target) {
                    (StrikeKind::BurnEquipment, cards::Target::Card(w)) => {
                        s.strike_at = None;
                        s.strike_weapon = Some(w);
                    }
                    (_, cards::Target::Card(r)) => s.strike_at = Some(r),
                    _ => s.strike_at = None,
                }
                s.chosen = true;
                if initial {
                    s.committed = None;
                } else {
                    s.additional_with = None;
                }
            }
            Effect::FortitudeProofStrike => c.sides[i].fortitude_proof = true,
            // At the maneuver, a maneuver committing the strike (Slam
            // [POT], Aid from Bats); at strike declaration, the strike.
            Effect::CommittedStrike(k) if c.step == Step::DetermineRange => {
                c.maneuver(i);
                c.sides[i].committed_card = Some(k);
            }
            Effect::CommittedStrike(k) => self.apply_combat_effect(Effect::Strike(k), play),
            Effect::AdditionalStrikeOnly(k) => {
                c.sides[i].additional += 1;
                c.sides[i].additional_only = Some(k);
            }
            Effect::UndodgeableStrike => c.sides[i].undodgeable_strike = true,
            Effect::AggravatedStrike => c.sides[i].aggravated_strike = true,
            Effect::PutOnOpponentOnHit { counters } => {
                c.sides[i].held = Some(Held {
                    card: play.card,
                    seat: play.seat,
                    mode: play.mode,
                    counters,
                    minion: actor,
                });
            }
            Effect::PutOnActorAfterCombat => c.after_combat.push(Held {
                card: play.card,
                seat: play.seat,
                mode: play.mode,
                counters: 0,
                minion: actor,
            }),
            Effect::StrengthThisRound(n) => c.sides[i].strength_round += n,
            Effect::PreventThisRound(n) => c.sides[i].round_prevents += n,
            Effect::PreventEachRound(n) => {
                c.sides[i].prevent_each_round += n;
                c.sides[i].round_prevents += n;
            }
            Effect::PressThisRound => c.sides[i].presses += 1,
            Effect::ActorBurnsBlood(n) => {
                let v = &mut self.state.minions[actor.index()];
                v.blood -= n.min(v.blood);
            }
            Effect::PreventOpposingStrikesThisRound | Effect::PreventStrikeDamage => {
                let s = &mut c.sides[i];
                let d = std::mem::take(&mut s.strike_damage);
                s.pending.normal -= d.normal.min(s.pending.normal);
                s.pending.aggravated -= d.aggravated.min(s.pending.aggravated);
                s.pending.gun = s.pending.gun.saturating_sub(d.gun).min(s.pending.normal);
                // Rolling with the Punches: the rest of the round's strikes
                // too; Touch of Valeren: that strike only.
                s.strikes_prevented |= effect == Effect::PreventOpposingStrikesThisRound;
            }
            Effect::NullifyOpposingWeapons => c.sides[i].nullifies_weapons = true,
            Effect::EnvironmentalDamage(n) => c.sides[i].environmental += n,
            Effect::RoundStrike(k) => c.sides[i].round_strike = Some(k),
            Effect::SetRangeLong => {
                if !c.range_set {
                    c.range = Range::Long;
                    c.range_set = true;
                }
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
            Effect::OptionalManeuver => c.sides[i].maneuvers += 1,
            Effect::AdditionalStrikes { n, limited } => {
                c.sides[i].additional += n;
                if limited {
                    c.sides[i].limited_used = true;
                }
            }
            Effect::SetStrength(n) => c.sides[i].strength_set = Some(n),
            Effect::StrengthThisCombat(n) => c.sides[i].strength_mods += n,
            Effect::AggravatedHands => c.sides[i].aggravated_hands = true,
            Effect::Prevent(n) => c.prevent(i, n),
            Effect::PreventXPlus(k) => c.prevent(i, play.x + k),
            Effect::PreventNonAggravated(n) => {
                c.sides[i].pending.prevent(n, false);
            }
            Effect::PreventNonGun(n) => {
                let d = c.sides[i].pending;
                if d.normal > d.gun && d.aggravated > 0 && n < d.total() - d.gun {
                    c.step = Step::Allocate { side: i, n };
                } else {
                    c.sides[i].pending.prevent_non_gun(n);
                }
            }
            Effect::OnlyHandStrikes => c.only_hand_strikes = true,
            Effect::NextRoundClose => c.next_round_close = true,
            Effect::NewRound => c.continued = true,
            Effect::GainBloodLostByOpponent => {
                let gained = c.sides[1 - i].lost_to_damage;
                self.state.gain_blood(actor, gained);
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
        // "Cannot strike" this round: no strike is chosen, in any pair
        // (additional strikes are still gained, uselessly: LSJ 19970814-2).
        let barred = self.no_strikes_round == Some(self.round);
        for s in &mut self.sides {
            s.set_strike(None);
            if barred {
                // The pair's additional strike is spent, unused.
                if self.pair > 0 {
                    s.additional = s.additional.saturating_sub(1);
                }
                s.chosen = true;
            } else if self.pair == 0 {
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
            | Effect::OptionalManeuver
            | Effect::NextRoundManeuver
            | Effect::AdditionalStrikes { .. }
            | Effect::SetStrength(_)
            | Effect::AggravatedHands
            | Effect::FortitudeProofStrike
            | Effect::CommittedStrike(_)
            | Effect::AdditionalStrikeOnly(_)
            | Effect::UndodgeableStrike
            | Effect::AggravatedStrike
            | Effect::PutOnOpponentOnHit { .. }
            | Effect::PutOnActorAfterCombat
            | Effect::StrengthThisRound(_)
            | Effect::PreventThisRound(_)
            | Effect::PreventEachRound(_)
            | Effect::PressThisRound
            | Effect::ActorBurnsBlood(_)
            | Effect::PreventOpposingStrikesThisRound
            | Effect::PreventStrikeDamage
            | Effect::NullifyOpposingWeapons
            | Effect::EnvironmentalDamage(_)
            | Effect::RoundStrike(_)
            | Effect::Prevent(_)
            | Effect::PreventXPlus(_)
            | Effect::PreventNonAggravated(_)
            | Effect::OnlyHandStrikes
            | Effect::NextRoundClose
            | Effect::NewRound
            | Effect::GainBloodLostByOpponent
            | Effect::ContinueAfterCombat { .. }
            | Effect::SetRangeLong
            | Effect::PreventNonGun(_)
            | Effect::StrengthThisCombat(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(normal: u8, aggravated: u8) -> Damage {
        Damage {
            normal,
            aggravated,
            gun: 0,
        }
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
                named: None,
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
