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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Range {
    #[default]
    Close,
    Long,
}

/// What a combat did, kept for a narrator when `Engine::combat_notes` is
/// on: a combat whose windows all elide leaves no decision to tell it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatNote {
    /// The acting side first.
    Begins {
        minions: [MinionId; 2],
    },
    /// A pair of strikes resolved (`pair` 0: the normal pair): each side's
    /// strike (`None`: no strike) and the damage it inflicts, before
    /// prevention.
    Strikes {
        minions: [MinionId; 2],
        round: u8,
        pair: u8,
        range: Range,
        strikes: [Option<StrikeKind>; 2],
        damage: [u8; 2],
    },
    Ends {
        minions: [MinionId; 2],
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Step {
    #[default]
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
    /// the preventer (`by`) chooses how many aggravated points it covers
    /// (spec §4.5.2; owner 2026-09-04: one may prefer torpor with blood
    /// intact), then the step goes `back` to side k's prevention, or (None)
    /// to the others' prevention window.
    Allocate {
        side: usize,
        n: u8,
        by: SeatId,
        back: Option<usize>,
    },
    /// Points just prevented on side `side` could have been strike damage
    /// or other damage (environmental, a retainer's): the preventer `by`
    /// picks how many were from the opposing strikes — aggravated in
    /// `lo_a..=hi_a`, normal in `lo_n..=hi_n` — which Disarm reads (owner,
    /// 2026-09-26: rules-qa 29.3). Then `back`, as for `Allocate`.
    AllocateSource {
        side: usize,
        lo_a: u8,
        hi_a: u8,
        lo_n: u8,
        hi_n: u8,
        by: SeatId,
        back: Option<usize>,
    },
    /// After both sides' prevention, every Methuselah in turn, clockwise
    /// from the acting one, may prevent with a minion out of the combat
    /// (Martyr's Resilience; ANK 20181101).
    OthersPrevent,
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

/// Does the weapon a strike came from carry rider `e` on its strike?
fn source_rider(st: &crate::state::GameState, source: Option<CardId>, e: Effect) -> bool {
    source.is_some_and(|w| {
        cards::lookup(st.cards[w.index()].card)
            .is_some_and(|d| d.abilities.iter().any(|m| m.effects.contains(&e)))
    })
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

impl Held {
    /// The card of `play`, to go on or after `minion`'s combat.
    pub fn from_play(play: &PlayCtx, minion: MinionId, counters: u8) -> Self {
        Held {
            card: play.card,
            seat: play.seat,
            mode: play.mode,
            counters,
            minion,
        }
    }
}

#[derive(Clone, Debug, Default)]
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
    /// Damage this side can prevent at will in any round of this combat
    /// (Beast Meld [ani][pro] if this vampire is blocked).
    pub combat_prevents: u8,
    /// This side can strike: combat ends in this round (Night Terrors
    /// [OBF][PRE]: the first).
    pub combat_ends_round: Option<u8>,
    /// This minion cannot use equipment this combat (Form of the Bat).
    pub no_equipment: bool,
    /// Form of Mist [PRO] rider, armed by the play, fired by the strike.
    pub continue_rider: Option<(u8, u8)>,
    /// Catatonic Fear [PRE] rider, armed by the play, fired by the strike:
    /// n unpreventable damage on the opposing minion after combat.
    pub after_damage_rider: Option<u8>,
    /// The card in play the chosen strike comes from (a weapon).
    pub strike_source: Option<CardId>,
    /// The weapon whose strike resolved, kept until its damage is applied
    /// (Sword of the Archangel: "burned during this weapon's strike
    /// resolution").
    pub resolved_with: Option<CardId>,
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
    /// The chosen strike is done with first strike (Donnybrook [obf]).
    pub first_strike: bool,
    pub held: Option<Held>,
    /// +n strength this round (Bear's Skin [ani][pro]).
    pub strength_round: u8,
    /// Damage this side can prevent each round of the combat (Bear's Skin
    /// [ANI][PRO]), added to `round_prevents` as each round begins.
    pub prevent_each_round: u8,
    /// Of `pending`, the damage from the opposing minion's strikes.
    pub strike_damage: Damage,
    /// Unpreventable damage from the opposing strike (Enhanced Coagulant):
    /// out of `pending`, so no prevention reaches it; added as the damage
    /// resolves.
    pub unpreventable: u8,
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
    /// Frenzy effects on this minion this combat (Terror Frenzy): it cannot
    /// maneuver to close range, press to continue or use equipment, and
    /// its combat cards cost this much more blood. Tranquility Shield
    /// cancels them and bars more (`frenzy_immune`).
    pub no_close_maneuver: bool,
    pub no_continue_press: bool,
    pub frenzy_no_equipment: bool,
    pub combat_card_surcharge: u8,
    pub frenzy_immune: bool,
    /// Flesh of Marble this combat: `Some(aggravated too)`.
    pub marble: Option<bool>,
    /// Damage was successfully inflicted on this minion this round.
    pub damaged_round: bool,
    /// Optional maneuvers this round, only usable to get to close range
    /// (Angel's Gift).
    pub close_maneuvers: u8,
    /// Damage from this minion's strikes successfully inflicted on the
    /// opposing one this round (Disarm).
    pub inflicted_round: u8,
}

impl Side {
    fn new(minion: MinionId, seat: SeatId) -> Self {
        Self {
            minion,
            seat,
            ..Default::default()
        }
    }

    /// Of the points prevented, `a` aggravated and `n` normal were from the
    /// opposing strikes.
    fn strike_prevented(&mut self, a: u8, n: u8) {
        let d = &mut self.strike_damage;
        d.aggravated -= a.min(d.aggravated);
        d.normal -= n.min(d.normal);
        d.gun = d.gun.min(d.normal);
    }

    /// This minion cannot use equipment this combat (Form of the Bat, a
    /// frenzy card).
    pub fn equipment_barred(&self) -> bool {
        self.no_equipment || self.frenzy_no_equipment
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
        self.first_strike = false;
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
            no_close_maneuver: self.no_close_maneuver,
            no_continue_press: self.no_continue_press,
            frenzy_no_equipment: self.frenzy_no_equipment,
            combat_card_surcharge: self.combat_card_surcharge,
            frenzy_immune: self.frenzy_immune,
            marble: self.marble,
            combat_ends_round: self.combat_ends_round,
            environmental: self.environmental,
            prevent_each_round: self.prevent_each_round,
            combat_prevents: self.combat_prevents,
            round_prevents: self.prevent_each_round,
            continue_rider: None,
            after_damage_rider: None,
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

#[derive(Clone, Debug, Default)]
pub struct CombatCtx {
    /// `sides[0]` is the acting minion's, `sides[1]` the opponent's.
    pub sides: [Side; 2],
    /// Weapons whose strike resolution burned the opposing vampire (Sword
    /// of the Archangel).
    pub burned_by: Vec<CardId>,
    pub round: u8,
    /// Strike pair within the round; 0 is the normal pair.
    pub pair: u8,
    pub range: Range,
    /// The range was set by an effect: skip the determine range step.
    pub range_set: bool,
    /// FBI Special Affairs Division: damage due to the acting vampire as
    /// the combat ends (an ally burned in it).
    pub fbi_damage: u8,
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
    /// Card modes played per minion, for `Limit::PerCombatThisMode`.
    pub played_combat_modes: Vec<(MinionId, u16, u8)>,
    /// Weather Control this combat: the round it was played and whether
    /// its damage grows each subsequent round.
    pub weather: Vec<(u8, bool)>,
    /// "All damage inflicted on vampires during the resulting combat is
    /// aggravated" (Dawn Operation).
    pub all_aggravated: bool,
    /// Rötschreck played on this vampire: locked and sent to torpor as the
    /// combat ends, not if it continues.
    pub rotschreck: Option<MinionId>,
    /// Hunting the Beast [AUS]: the acting vampire, and the uncontrolled
    /// clan vampire and blood its Methuselah adds at the end of the combat
    /// if it is ready and the opposing vampire is not.
    pub won_uncontrolled_blood: Option<(MinionId, &'static str, u8)>,
    /// Damage aimed at retainers in this strike pair, waiting for
    /// prevention like the combatants' (Martyr's Resilience).
    pub retainer_pending: Vec<(CardId, u8)>,
    /// The others' prevention window of this pair has run.
    pub others_prevented: bool,
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
    /// Begun by an ability used after the action's resolution (The
    /// Guardian): over, it returns to the after-resolution window.
    pub after_resolution: bool,
    /// Cards to put on their minion after combat ends (Pass Through Shadow
    /// [OBL]), from the ash heap; not if the combat continues.
    pub after_combat: Vec<Held>,
    /// Damage inflicted after combat ends, by a combatant on the other
    /// (Catatonic Fear [PRE]): unpreventable; not if the combat continues.
    pub after_combat_damage: Vec<(MinionId, u8)>,
}

impl CombatCtx {
    pub fn side_of(&self, seat: SeatId) -> Option<usize> {
        self.sides.iter().position(|s| s.seat == seat)
    }

    pub fn side_of_minion(&self, m: MinionId) -> Option<usize> {
        self.sides.iter().position(|s| s.minion == m)
    }

    /// The minion in combat with `m`, if `m` is in this combat.
    pub fn opponent_of(&self, m: MinionId) -> Option<MinionId> {
        self.side_of_minion(m).map(|i| self.sides[1 - i].minion)
    }

    pub fn both_ready(&self, st: &GameState) -> bool {
        self.sides
            .iter()
            .all(|s| st.minions[s.minion.index()].ready())
    }

    /// Does side `1 - i`'s chosen strike attempt to inflict aggravated
    /// damage on side `i`'s minion at the current range (Rötschreck)? A
    /// strike aimed at a retainer does not.
    pub fn aggravated_strike_on(
        &self,
        st: &GameState,
        action: Option<&crate::actions::ActionCtx>,
        i: usize,
    ) -> bool {
        let s = &self.sides[1 - i];
        if s.strike_at.is_some() {
            return false;
        }
        let close = self.range == Range::Close;
        let all = self.all_aggravated && st.minions[self.sides[i].minion.index()].is_vampire();
        match s.strike {
            Some(StrikeKind::Hand { .. }) => {
                close
                    && (all
                        || s.aggravated_hands
                        || s.aggravated_strike
                        || action.is_some_and(|a| a.aggravated_hands && a.acting == s.minion))
            }
            Some(StrikeKind::Melee { aggravated, .. }) => close && (aggravated || all),
            Some(StrikeKind::Damage {
                ranged, aggravated, ..
            }) => (ranged || close) && (aggravated || all),
            _ => false,
        }
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
    /// 2.2); additional strike cards only once the first pair is over, both
    /// combatants still ready (rules-qa 47.2, revising 4.4), and again
    /// after each additional pair (48.2).
    fn windows(&self) -> Vec<Window> {
        match self.step {
            Step::BeforeRange => vec![Window::CombatBeforeRange, Window::CombatUntimed],
            Step::DetermineRange => vec![Window::CombatDetermineRange, Window::CombatUntimed],
            Step::BeforeStrikes => vec![Window::CombatBeforeStrikes, Window::CombatUntimed],
            Step::ChooseStrike(_) => vec![Window::CombatStrikeDeclaration],
            Step::BeforeResolution => vec![Window::CombatUntimed, Window::CombatBeforeResolution],
            Step::Prevent(_)
            | Step::Allocate { .. }
            | Step::AllocateSource { .. }
            | Step::OthersPrevent => {
                vec![Window::CombatDamagePrevention]
            }
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
            Step::Prevent(_) | Step::OthersPrevent => WindowLabel::CombatPrevention,
            Step::Allocate { .. } | Step::AllocateSource { .. } => {
                WindowLabel::CombatPreventionAllocation
            }
            Step::AdditionalStrikes => WindowLabel::CombatAdditionalStrikes,
            Step::Press => WindowLabel::CombatPress,
            Step::EndOfRound => WindowLabel::CombatEndOfRound,
        }
    }

    /// Damage is pending on a retainer of side `i`'s minion.
    pub(crate) fn retainer_damage_on(&self, st: &GameState, i: usize) -> bool {
        self.retainer_pending
            .iter()
            .any(|&(r, n)| n > 0 && st.cards[r.index()].host == Some(self.sides[i].minion))
    }

    /// A maneuver now: none to close range under a frenzy (Terror Frenzy
    /// [ani]).
    pub(crate) fn may_maneuver(&self, side: usize) -> bool {
        !(self.sides[side].no_close_maneuver && self.range == Range::Long)
    }

    /// A press now: none to continue under a frenzy (Terror Frenzy [ani]).
    pub(crate) fn may_press(&self, side: usize) -> bool {
        !(self.sides[side].no_continue_press && !self.press_continue)
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
            // "Cannot maneuver to close range" (Terror Frenzy [ani]).
            Effect::Maneuver if self.step == Step::DetermineRange => {
                self.last_maneuver != Some(side) && self.may_maneuver(side)
            }
            // Nor the maneuver of a strike that cannot be used (LSJ
            // 20001126).
            Effect::CommitStrike => {
                self.sides[side].committed.is_none() && self.no_strikes_round != Some(self.round)
            }
            Effect::SetRange(_) => !self.range_set,
            // A strike card played at the maneuver: a maneuver in turn, one
            // commitment a round, a strike that can be used (LSJ 20021028).
            Effect::CommittedStrike(k) if self.step == Step::DetermineRange => {
                self.last_maneuver != Some(side)
                    && self.may_maneuver(side)
                    && self.sides[side].committed.is_none()
                    && self.sides[side].committed_card.is_none()
                    && self.no_strikes_round != Some(self.round)
                    && (!self.only_hand_strikes || matches!(k, StrikeKind::Hand { .. }))
            }
            Effect::Press if self.step == Step::Press => {
                self.last_press != Some(side) && self.may_press(side)
            }
            // One additional-strike card at a time: a combatant with an
            // additional strike still to make gains no other before that
            // pair resolves (rules-qa 48.2: Quickness after Pursuit's pair).
            Effect::AdditionalStrikes { limited: true, .. }
            | Effect::AdditionalStrikesX { limited: true }
            | Effect::AdditionalStrikeOnly { limited: true, .. } => {
                !self.sides[side].limited_used && self.sides[side].additional == 0
            }
            Effect::AdditionalStrikes { .. }
            | Effect::AdditionalStrikesX { .. }
            | Effect::AdditionalStrikeOnly { .. } => self.sides[side].additional == 0,
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

    /// After the action's resolution, a minion enters combat with the
    /// acting minion, which acts first (The Guardian, ANK 20221102).
    pub(crate) fn start_combat_after_resolution(&mut self, acting: MinionId, opposing: MinionId) {
        self.begin_combat(acting, opposing, false);
        self.combat.as_mut().expect("combat").after_resolution = true;
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
            // "Brujah get 1 optional maneuver each combat" (Brujah Debate).
            s.combat_maneuvers += st.clan_combat_maneuvers(s.minion);
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
            step: Step::BeforeRange,
            window: OrderedWindow::new(order),
            held_at_block,
            at_resolution,
            ..Default::default()
        });
        self.flow = Flow::Combat;
        self.note(CombatNote::Begins {
            minions: [acting, opposing],
        });
    }

    fn note(&mut self, n: CombatNote) {
        if let Some(notes) = self.combat_notes.as_mut() {
            notes.push(n);
        }
    }

    /// The strike choice among `options`, none without a strike to choose:
    /// with the abilities usable at any impulse, and the retainer targets
    /// of side `retainers`'s ranged strikes.
    fn strike_request(
        &self,
        seat: SeatId,
        mut options: Vec<Choice>,
        retainers: Option<usize>,
    ) -> Option<DecisionRequest> {
        if options.is_empty() {
            return None;
        }
        options.extend(self.ability_options(seat, Window::AnyImpulse));
        if let Some(i) = retainers {
            options = self.with_retainer_targets(i, options);
        }
        Some(DecisionRequest::new(
            seat,
            WindowLabel::CombatStrike,
            options,
        ))
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
                    return self.strike_request(side.seat, options, Some(i));
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
                                .filter(|&w| cards::is_weapon(self.state.cards[w.index()].card))
                                .map(|w| Choice::BurnWeaponStrike { weapon: w.0 }),
                        );
                    }
                    return self.strike_request(side.seat, options, None);
                }
                // The initial strike a card committed at the maneuver (Slam
                // [POT]), an additional strike only usable as one strike
                // (Wind Dance [THA]'s dodge): that strike, if it can be used.
                let only = if c.pair == 0 {
                    side.committed_card
                } else {
                    side.additional_only
                };
                if let Some(k) = only {
                    let usable = !c.only_hand_strikes || matches!(k, StrikeKind::Hand { .. });
                    let options = usable
                        .then_some(Choice::CommittedStrike)
                        .into_iter()
                        .collect();
                    return self.strike_request(side.seat, options, None);
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
                self.strike_request(side.seat, options, Some(i))
            }
            Step::Allocate { side, n, by, .. } => {
                let d = c.sides[side].pending;
                let lo = n.saturating_sub(d.normal);
                let hi = n.min(d.aggravated);
                Some(DecisionRequest::new(
                    by,
                    WindowLabel::CombatPreventionAllocation,
                    (lo..=hi).map(Choice::SetX).collect(),
                ))
            }
            Step::AllocateSource {
                lo_a,
                hi_a,
                lo_n,
                hi_n,
                by,
                ..
            } => Some(DecisionRequest::new(
                by,
                WindowLabel::CombatPreventionAllocation,
                (lo_a + lo_n..=hi_a + hi_n).map(Choice::SetX).collect(),
            )),
            Step::Prevent(i) => {
                let side = &c.sides[i];
                if !side.preventing
                    || (side.pending.total() == 0 && !c.retainer_damage_on(&self.state, i))
                {
                    return None;
                }
                let mut options = vec![Choice::Pass];
                // The round's allowance covers this minion only, not its
                // retainers.
                if (side.round_prevents > 0 || side.combat_prevents > 0) && side.pending.total() > 0
                {
                    options.push(Choice::UsePrevent);
                }
                options.extend(self.impulse_options(side.seat, &c.windows()));
                Some(DecisionRequest::new(
                    side.seat,
                    WindowLabel::CombatPrevention,
                    options,
                ))
            }
            // Prevention by minions out of the combat, any Methuselah's.
            Step::OthersPrevent => {
                let seat = c.window.current()?;
                let mut options = vec![Choice::Pass];
                options.extend(
                    self.play_options(seat, &c.windows(), None)
                        .into_iter()
                        .filter(|o| match *o {
                            Choice::Play { actor, .. } => {
                                actor.is_some_and(|m| c.side_of_minion(m).is_none())
                            }
                            _ => true,
                        }),
                );
                Some(DecisionRequest::new(
                    seat,
                    WindowLabel::CombatPrevention,
                    options,
                ))
            }
            _ => {
                let seat = c.window.current()?;
                let i = c.side_of(seat).expect("combatant seat");
                let mut options = vec![Choice::Pass];
                let mut mandatory = false;
                match c.step {
                    Step::DetermineRange
                        if (c.sides[i].maneuvers > 0
                            || c.sides[i].combat_maneuvers > 0
                            || (c.sides[i].close_maneuvers > 0 && c.range == Range::Long))
                            && c.last_maneuver != Some(i)
                            && c.may_maneuver(i) =>
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
                            && c.may_press(i)
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
                options.extend(self.impulse_options(seat, &c.windows()));
                if mandatory {
                    // Used before its controller passes (spec §1.1.2): a
                    // press card played to continue first leaves it unused,
                    // and it waits for a later round (rules-qa 14.1).
                    options.retain(|o| *o != Choice::Pass);
                }
                Some(DecisionRequest::new(seat, c.label(), options))
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
            // The round's committed initial strike (Slam), or the only
            // strike an additional one allows (Wind Dance [THA]).
            (Step::ChooseStrike(i), Choice::CommittedStrike) => {
                let k = if c.pair == 0 {
                    c.sides[i].committed_card.take()
                } else {
                    c.sides[i].additional_only.take()
                };
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
                if c.sides[i].round_prevents > 0 {
                    c.sides[i].round_prevents -= 1;
                } else {
                    c.sides[i].combat_prevents -= 1;
                }
                c.prevent(i, 1);
                None
            }
            (Step::Allocate { side, n, by, back }, Choice::SetX(k)) => {
                let before = c.sides[side].pending;
                let d = &mut c.sides[side].pending;
                d.aggravated -= k;
                d.normal -= n - k;
                d.gun = d.gun.min(d.normal);
                c.step = match back {
                    Some(b) => Step::Prevent(b),
                    None => Step::OthersPrevent,
                };
                c.settle_source(side, before, false, by, back);
                None
            }
            (
                Step::AllocateSource {
                    side,
                    lo_a,
                    hi_a,
                    hi_n,
                    back,
                    ..
                },
                Choice::SetX(k),
            ) => {
                let a = k.saturating_sub(hi_n).max(lo_a).min(hi_a);
                c.sides[side].strike_prevented(a, k - a);
                c.step = match back {
                    Some(b) => Step::Prevent(b),
                    None => Step::OthersPrevent,
                };
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
                // One only usable to close first, then this round's banked
                // maneuver, then the combat-long one.
                if c.range == Range::Long && c.sides[i].close_maneuvers > 0 {
                    c.sides[i].close_maneuvers -= 1;
                } else if c.sides[i].maneuvers > 0 {
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
                if self.weather_damage() {
                    return;
                }
                let c = self.combat.as_mut().expect("combat context");
                if c.round_over {
                    c.fresh_window(Step::EndOfRound);
                } else if c.range_set {
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
                let before = self.combat_notes.as_ref().map(|_| {
                    let c = self.combat.as_ref().expect("combat context");
                    let pending = c.sides.each_ref().map(|s| s.pending.total());
                    (c.sides.each_ref().map(|s| s.strike), pending)
                });
                self.resolve_strikes();
                // Still in combat (an oust may have ended it).
                if let (Some((strikes, pending)), Some(c)) = (before, self.combat.as_ref()) {
                    // Side i inflicts what side 1 - i now has pending.
                    let damage = [1, 0]
                        .map(|j: usize| c.sides[j].pending.total().saturating_sub(pending[j]));
                    let n = CombatNote::Strikes {
                        minions: c.sides.each_ref().map(|s| s.minion),
                        round: c.round,
                        pair: c.pair,
                        range: c.range,
                        strikes,
                        damage,
                    };
                    self.note(n);
                }
                self.after_prevention_or_next();
            }
            Step::Prevent(i) => {
                c.sides[i].preventing = false;
                self.after_prevention_or_next();
            }
            Step::Allocate { .. } | Step::AllocateSource { .. } => {
                unreachable!("allocation always offers a choice")
            }
            Step::OthersPrevent => self.after_prevention_or_next(),
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
            // No new round once a combatant left the ready region (Disarm
            // after Telepathic Tracking): the combat ends.
            Step::EndOfRound => {
                if (c.continued || c.press_continue) && c.both_ready(&self.state) {
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
                if c.sides[i].preventing
                    && (c.sides[i].pending.total() > 0 || c.retainer_damage_on(&self.state, i))
                {
                    c.step = Step::Prevent(i);
                    return;
                }
            }
            // Then the others' prevention, all Methuselahs in turn from the
            // acting one (Martyr's Resilience).
            let pending = c.sides.iter().any(|s| s.pending.total() > 0)
                || c.retainer_pending.iter().any(|&(_, n)| n > 0);
            if pending && !c.others_prevented {
                c.others_prevented = true;
                c.step = Step::OthersPrevent;
                let order = self.state.clockwise_from(c.sides[0].seat);
                c.window = OrderedWindow::new(order);
                return;
            }
        }
        if self.marble_clamp() {
            return;
        }
        if self.apply_damage() {
            return;
        }
        let c = self.combat.as_mut().expect("combat context");
        // After every pair, the normal one and each additional one, a new
        // chance to gain additional strikes (rules-qa 48.2); the window
        // closes into the next additional pair, if any, else the press.
        if c.round_over {
            c.fresh_window(Step::EndOfRound);
        } else {
            c.fresh_window(Step::AdditionalStrikes);
        }
    }

    /// Flesh of Marble: "if a damage is successfully inflicted on this
    /// vampire in a given round, any additional damage inflicted on this
    /// vampire in the same round is automatically prevented. Aggravated
    /// damage cannot be prevented this way" ([PRO]: it can) — applied as the
    /// damage is about to land, after other prevention. With both kinds
    /// pending, the superior's one point let through is its controller's
    /// pick (LSJ 20021001): true while that choice is asked.
    fn marble_clamp(&mut self) -> bool {
        let c = self.combat.as_mut().expect("combat context");
        for i in 0..2 {
            let Some(aggravated_too) = c.sides[i].marble else {
                continue;
            };
            let by = c.sides[i].seat;
            let s = &mut c.sides[i];
            let d = s.pending;
            if d.total() == 0 {
                continue;
            }
            if s.damaged_round {
                s.pending.normal = 0;
                s.pending.gun = 0;
                if aggravated_too {
                    s.pending.aggravated = 0;
                }
                if c.settle_source(i, d, false, by, Some(i)) {
                    return true;
                }
                continue;
            }
            if d.total() <= 1 {
                continue;
            }
            if !aggravated_too {
                // Aggravated damage all lands; one normal point only if
                // no aggravated one does.
                s.pending.normal = if d.aggravated > 0 { 0 } else { 1 };
                s.pending.gun = s.pending.gun.min(s.pending.normal);
                if c.settle_source(i, d, false, by, Some(i)) {
                    return true;
                }
            } else if d.normal > 0 && d.aggravated > 0 {
                c.step = Step::Allocate {
                    side: i,
                    n: d.total() - 1,
                    by: c.sides[i].seat,
                    back: Some(i),
                };
                return true;
            } else {
                s.pending.prevent(d.total() - 1, true);
                if c.settle_source(i, d, false, by, Some(i)) {
                    return true;
                }
            }
        }
        false
    }

    /// Dawn Operation: the damage on the vampires in this combat is
    /// aggravated, whatever its source.
    fn aggravate_vampires(&mut self) {
        let c = self.combat.as_mut().expect("combat context");
        if !c.all_aggravated {
            return;
        }
        for s in &mut c.sides {
            if !self.state.minions[s.minion.index()].is_vampire() {
                continue;
            }
            for d in [&mut s.pending, &mut s.strike_damage] {
                d.aggravated += d.normal;
                d.normal = 0;
                d.gun = 0;
            }
        }
    }

    /// Weather Control's damage this round, as the before-range step ends
    /// (ANK 20170930: both minions play their before-range effects first):
    /// each combatant and each retainer on them takes it, unpreventable,
    /// environmental — even if the combat is ending (LSJ 19971110, ANK
    /// 20190725). True when the flow moved on (`apply_damage`).
    pub(crate) fn weather_damage(&mut self) -> bool {
        let c = self.combat.as_mut().expect("combat context");
        let n: u8 = c
            .weather
            .iter()
            .map(|&(from, escalate)| 1 + if escalate { c.round - from } else { 0 })
            .sum();
        if n == 0 {
            return false;
        }
        let minions = [c.sides[0].minion, c.sides[1].minion];
        for s in &mut c.sides {
            s.pending.add(n, false);
        }
        self.aggravate_vampires();
        let st = &mut self.state;
        let mut lifeless = Vec::new();
        for (i, card) in st.cards.iter_mut().enumerate() {
            if card.present()
                && card.host.is_some_and(|h| minions.contains(&h))
                && LIBRARY[card.card as usize]
                    .types
                    .contains(&crate::library::CardType::Retainer)
            {
                card.counters = card.counters.saturating_sub(n);
                if card.counters == 0 {
                    lifeless.push(CardId(i as u32));
                }
            }
        }
        for r in lifeless {
            self.burn_card_in_play(r);
        }
        self.apply_damage()
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
                // Catatonic Fear [PRE]: at close range, not dodged (LSJ
                // 19980526).
                if let Some(n) = c.sides[i].after_damage_rider.take() {
                    if c.range == Range::Close && !matches!(strikes[1 - i], Some(StrikeKind::Dodge))
                    {
                        c.after_combat_damage.push((c.sides[1 - i].minion, n));
                    }
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
        // First strike (rulebook "First Strike"): resolved before a normal
        // strike, after a strike: combat ends; a dodge still protects from
        // it; both with first strike resolve together, as normal. The
        // pool's only first strike burns an equipment or retainer
        // (Donnybrook [obf]): a weapon burned so loses its strike, below.
        let mut strikes = strikes;
        let first: Vec<usize> = (0..2)
            .filter(|&i| c.sides[i].first_strike && strikes[i].is_some())
            .collect();
        if let [i] = first[..] {
            debug_assert_eq!(strikes[i], Some(StrikeKind::BurnEquipment));
            let j = 1 - i;
            let dodged = strikes[j] == Some(StrikeKind::Dodge)
                && !self.state.undodgeable(c.sides[i].minion)
                && !c.sides[i].undodgeable_strike;
            let burned = c.sides[i].strike_weapon.filter(|w| {
                let wc = &self.state.cards[w.index()];
                !dodged
                    && c.range == Range::Close
                    && wc.present()
                    && wc.host == Some(c.sides[j].minion)
            });
            strikes[i] = None;
            if let Some(w) = burned {
                self.burn_card_in_play(w);
            }
        }
        let c = self.combat.as_mut().expect("combat context");
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
        // A weapon put on the opposing vampire as its strike lands, becoming
        // another card (Enhanced Coagulant); dodged, it stays (ANK
        // 20220923).
        let mut becoming: Vec<(CardId, MinionId)> = Vec::new();
        for (i, &landed) in lands.iter().enumerate() {
            let opp = c.sides[1 - i].minion;
            if let Some(w) = c.sides[i].strike_source {
                // A weapon inflicting no damage (Blood Rage) still moves.
                if landed
                    && st.minions[opp.index()].is_vampire()
                    && source_rider(st, Some(w), Effect::BecomesOnOpponent)
                {
                    becoming.push((w, opp));
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
                && c.sides[i]
                    .strike_source
                    .is_some_and(|w| cards::is_weapon(st.cards[w.index()].card))
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
                if source_rider(st, Some(w), Effect::AdditionalStrikeWithSource)
                    && !c.sides[i].limited_used
                {
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
                            if source_rider(st, Some(w), Effect::CountersPerDamage) {
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
                        if source_rider(st, c.sides[i].strike_source, Effect::UnpreventableStrike) {
                            c.sides[j].unpreventable += n;
                        } else {
                            c.sides[j].strike_hit(n, aggravated, gun && !aggravated);
                        }
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
        // Damage to a retainer waits for prevention (Martyr's Resilience);
        // life stolen from it moves at once, becoming blood.
        let mut lifeless = Vec::new();
        for (r, n, steal, i) in retainer_hits {
            if !steal {
                c.retainer_pending.push((r, n));
                continue;
            }
            let card = &mut self.state.cards[r.index()];
            let taken = n.min(card.counters);
            card.counters -= taken;
            self.state.gain_blood(c.sides[i].minion, taken);
            if self.state.cards[r.index()].counters == 0 {
                lifeless.push(r);
            }
        }
        for s in &mut c.sides {
            s.resolved_with = s.strike_source;
            s.set_strike(None);
            s.strike_at = None;
            s.preventing = s.pending.total() > 0;
        }
        for i in 0..2 {
            let on_retainer = c.retainer_damage_on(&self.state, i);
            c.sides[i].preventing |= on_retainer;
        }
        self.aggravate_vampires();
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
        for (w, on) in becoming {
            self.become_on(w, on);
        }
    }

    /// Card `w` moves onto vampire `on` and becomes its windowless mode:
    /// that mode's layers replace its own (Enhanced Coagulant).
    fn become_on(&mut self, w: CardId, on: MinionId) {
        let c = &self.state.cards[w.index()];
        if !c.present() || !self.state.minions[on.index()].in_play {
            return;
        }
        let def = cards::lookup(c.card).expect("definition");
        let Some(mi) = def.modes.iter().position(|m| m.windows.is_empty()) else {
            return;
        };
        self.state.layers.retain(|l| l.source != w);
        let c = &mut self.state.cards[w.index()];
        c.host = Some(on);
        c.mode = mi as u8;
        self.push_mode_layers(w);
    }

    /// The card goes on minion `on`, if still in play: out of the ash heap
    /// it went to as it resolved ("put this card on …", rules-qa 47.1, the
    /// rule for every such card). Only if the card has left the ash heap
    /// since, an effect-only stand-in carries its counters and layers.
    pub(crate) fn place_held(&mut self, h: Held, on: MinionId) {
        if !self.state.minions[on.index()].in_play {
            return;
        }
        let ash = &mut self.state.seats[h.seat.index()].ash;
        let taken = ash
            .iter()
            .rposition(|&c| c == h.card)
            .map(|k| ash.remove(k))
            .is_some();
        let id = self.enter_play(crate::state::CardInPlay {
            by: Some(h.minion),
            counters: h.counters,
            effect_only: !taken,
            ..crate::state::CardInPlay::new(h.card, h.seat, Some(on), h.mode)
        });
        self.push_mode_layers(id);
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
            c.after_combat_damage.clear();
        }
    }

    /// Damage applied at once to both sides after prevention (rulebook
    /// "Damage Resolution"): blood burned, torpor, or burned outright.
    /// Returns true when the flow moved on: a torpor rider (Fame) ousted a
    /// combatant's Methuselah mid-combat — the game ended, the active
    /// Methuselah's turn passed, or the opponent left the game and the
    /// combat ended at once (DESIGN §10).
    pub(crate) fn apply_damage(&mut self) -> bool {
        // Retainers burn life for the damage left on them (aggravated alike,
        // rulebook "Damage Resolution"); gone or moved by now, no effect.
        let c = self.combat.as_mut().expect("combat context");
        let hits = std::mem::take(&mut c.retainer_pending);
        let minions = [c.sides[0].minion, c.sides[1].minion];
        let mut lifeless = Vec::new();
        for (r, n) in hits {
            let card = &mut self.state.cards[r.index()];
            if n == 0 || !card.present() || !card.host.is_some_and(|h| minions.contains(&h)) {
                continue;
            }
            card.counters -= n.min(card.counters);
            if card.counters == 0 {
                lifeless.push(r);
            }
        }
        for r in lifeless {
            self.burn_card_in_play(r);
        }
        let c = self.combat.as_mut().expect("combat context");
        let results: Vec<Resolved> = c
            .sides
            .iter()
            .map(|s| {
                let m = &self.state.minions[s.minion.index()];
                let mut d = s.pending;
                d.normal += s.unpreventable;
                if m.is_ally() {
                    resolve_life_damage(m.blood, d)
                } else {
                    resolve_damage(m.blood, d)
                }
            })
            .collect();
        // Damage landing this round (Flesh of Marble), and the part from the
        // opposing strikes (Disarm: environmental damage does not count,
        // LSJ 20010812; prevention taking the other damage first).
        for i in 0..2 {
            let d = c.sides[i].pending.total();
            debug_assert!(
                c.sides[i].strike_damage.total() <= d,
                "strike damage over pending"
            );
            let from_strikes = d.min(c.sides[i].strike_damage.total()) + c.sides[i].unpreventable;
            let d = d + std::mem::take(&mut c.sides[i].unpreventable);
            c.sides[i].damaged_round |= d > 0;
            c.sides[i].strike_damage = Damage::default();
            c.sides[1 - i].inflicted_round += from_strikes;
        }
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
        // A vampire burned by the strike of the opposing weapon (Sword of the
        // Archangel's unlock).
        for i in 0..2 {
            let opp = c.sides[1 - i].minion;
            if let Some(w) = c.sides[i].resolved_with.take() {
                if burned.contains(&opp) && self.state.minions[opp.index()].is_vampire() {
                    c.burned_by.push(w);
                }
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
            return self.combat_ousts();
        }
        false
    }

    /// After an effect in combat that may have ousted a Methuselah: true
    /// when the flow moved on — the game ended, the active Methuselah's
    /// turn passed, or a combatant's Methuselah left the game and the
    /// combat ended at once (DESIGN §10).
    pub(crate) fn combat_ousts(&mut self) -> bool {
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
        false
    }

    fn new_round(&mut self) {
        // The combat continues: after-combat effects are lost (RTR
        // 20020501).
        self.discard_held(true);
        let c = self.combat.as_mut().expect("combat context");
        c.round += 1;
        c.pair = 0;
        // Rötschreck's torpor is lost as the combat continues (RTR
        // 20020501): the card stays, and so does its unlock ban.
        c.rotschreck = None;
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
        // Ended before range is determined: Weather Control's damage still
        // lands (LSJ 19971110, ANK 20190725).
        if self
            .combat
            .as_ref()
            .is_some_and(|c| c.step == Step::BeforeRange)
            && self.weather_damage()
        {
            return;
        }
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
        if let Some(c) = &ended {
            self.note(CombatNote::Ends {
                minions: c.sides.each_ref().map(|s| s.minion),
            });
        }
        // FBI Special Affairs Division: its unpreventable environmental
        // damage to the acting vampire after the combat; enough counters,
        // burned.
        let fbi = ended.as_ref().map_or(0, |c| c.fbi_damage);
        if fbi > 0 {
            if let Some(acting) = self.action.as_ref().map(|a| a.acting) {
                if self.state.minions[acting.index()].present() {
                    self.damage_out_of_combat(acting, fbi, false);
                }
            }
            if let Some((f, _, burn_at)) = self.state.ally_burned_in_combat() {
                if self.state.cards[f.index()].counters >= burn_at {
                    self.burn_card_in_play(f);
                }
            }
        }
        // Rötschreck: locked and sent to torpor now that combat ended.
        if let Some(t) = ended.as_ref().and_then(|c| c.rotschreck) {
            if self.state.minions[t.index()].ready() {
                self.state.minions[t.index()].locked = true;
                self.minions_to_torpor(&[t]);
            }
        }
        // Who fought, for "after a combat involving this vampire ends"
        // (Monster).
        if let (Some(c), Some(a)) = (ended.as_ref(), self.action.as_mut()) {
            a.last_combat = Some([c.sides[0].minion, c.sides[1].minion]);
        }
        // "After combat ends, put this card on this vampire" (Pass Through
        // Shadow [OBL]).
        for h in ended.iter().flat_map(|c| c.after_combat.clone()) {
            self.place_held(h, h.minion);
        }
        // Catatonic Fear [PRE]: the vampire's damage, after combat, so no
        // combat card prevents it (LSJ 20010205; RTR 19970630).
        for (m, n) in ended.iter().flat_map(|c| c.after_combat_damage.clone()) {
            if self.state.minions[m.index()].present() {
                self.damage_out_of_combat(m, n, false);
            }
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
        if let Some((m, clan, n)) = ended.as_ref().and_then(|c| c.won_uncontrolled_blood) {
            let c = ended.as_ref().expect("combat");
            let other = c.sides.iter().find(|s| s.minion != m).map(|s| s.minion);
            let v = &self.state.minions[m.index()];
            if v.ready() && other.is_some_and(|o| !self.state.minions[o.index()].ready()) {
                self.interrupts
                    .push(crate::draws::Interrupt::UncontrolledBlood {
                        seat: v.owner,
                        clan,
                        n,
                    });
            }
        }
        for s in lowered {
            if !self.state.seats[s.index()].ousted {
                self.draw_up(s);
            }
        }
        if ended.as_ref().is_some_and(|c| c.after_resolution) {
            self.back_to_after_resolution();
        } else {
            self.after_combat();
        }
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
                    let id = self.playable_card(seat, card);
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
        // Effects of a minion out of the combat, on the target in it
        // (Martyr's Resilience).
        match effect {
            Effect::PreventOnTarget(_) | Effect::PreventOnTargetXPlus(_) => {
                let n = match effect {
                    Effect::PreventOnTarget(n) => n,
                    Effect::PreventOnTargetXPlus(k) => play.x + k,
                    _ => unreachable!(),
                };
                let back = match c.step {
                    Step::Prevent(k) => Some(k),
                    _ => None,
                };
                match play.target {
                    cards::Target::Minion(m) => {
                        if let Some(t) = c.side_of_minion(m) {
                            c.prevent_by(t, n, play.seat, back);
                        }
                    }
                    cards::Target::Card(r) => {
                        let mut left = n;
                        for e in c.retainer_pending.iter_mut().filter(|e| e.0 == r) {
                            let p = left.min(e.1);
                            e.1 -= p;
                            left -= p;
                        }
                    }
                    _ => {}
                }
                return;
            }
            // In the combat or not (an actor out of it).
            Effect::ActorBurnsBloodX => {
                self.state.burn_blood(actor, play.x);
                return;
            }
            _ => {}
        }
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
            Effect::AdditionalStrikeOnly { kind, limited } => {
                c.sides[i].additional += 1;
                c.sides[i].additional_only = Some(kind);
                c.sides[i].limited_used |= limited;
            }
            // No strike got, no limit used (owner, 2026-09-26: rules-qa
            // 29.8).
            Effect::AdditionalStrikesX { limited } => {
                c.sides[i].additional += play.x;
                c.sides[i].limited_used |= limited && play.x > 0;
            }
            // The weapon's own strike and its riders, +n damage, from that
            // card (gone by resolution, no effect: spec §4.10.4).
            Effect::WeaponStrikePlus(n) => {
                let w = play.target.card();
                let (ability, k) =
                    cards::melee_strike(self.state.cards[w.index()].card).expect("a melee weapon");
                let StrikeKind::Melee { bonus, aggravated } = k else {
                    unreachable!()
                };
                let initial = c.pair == 0;
                let s = &mut c.sides[i];
                s.set_strike(Some(StrikeKind::Melee {
                    bonus: bonus + n,
                    aggravated,
                }));
                s.strike_source = Some(w);
                s.strike_at = None;
                s.chosen = true;
                if initial {
                    s.committed = None;
                } else {
                    s.additional_with = None;
                }
                // "1 optional press only usable to continue, once each
                // combat" (Righteous Blade): had as the strike is chosen.
                let source = cards::AbilitySource::Card(w);
                let rides = source
                    .mode(&self.state, ability as u8)
                    .effects
                    .contains(&Effect::ContinuePressEachCombat);
                let key = source.use_key(ability as u8);
                if rides && !c.abilities_used.contains(&key) {
                    c.abilities_used.push(key);
                    c.sides[i].continue_presses += 1;
                }
            }
            // A frenzy card on the opposing minion, unless immune (Tranquility
            // Shield).
            Effect::OpposingRestraint => {
                let o = &mut c.sides[1 - i];
                if !o.frenzy_immune {
                    o.no_close_maneuver = true;
                    o.no_continue_press = true;
                    o.frenzy_no_equipment = true;
                }
            }
            Effect::OpposingCombatCardsCost(n) => {
                let vampire = !self.state.minions[c.sides[1 - i].minion.index()].is_ally();
                let o = &mut c.sides[1 - i];
                if vampire && !o.frenzy_immune {
                    o.combat_card_surcharge += n;
                }
            }
            Effect::FrenzyShield => {
                let s = &mut c.sides[i];
                s.frenzy_immune = true;
                s.no_close_maneuver = false;
                s.no_continue_press = false;
                s.frenzy_no_equipment = false;
                s.combat_card_surcharge = 0;
            }
            Effect::FleshOfMarble { aggravated } => {
                let s = &mut c.sides[i];
                s.marble = Some(aggravated || s.marble == Some(true));
            }
            Effect::WeatherDamage { escalate } => c.weather.push((c.round, escalate)),
            Effect::PutOnOpponent => {
                let on = c.sides[1 - i].minion;
                self.place_held(Held::from_play(play, actor, 0), on);
            }
            // At once, where it lands: the flow may move on (an oust).
            Effect::OpponentToTorpor => {
                let o = &mut c.sides[1 - i];
                if self.state.minions[o.minion.index()].ready() {
                    o.sent_to_torpor = true;
                    if self.apply_damage() {
                        self.ousted_during_play = true;
                    }
                }
            }
            Effect::UndodgeableStrike => c.sides[i].undodgeable_strike = true,
            Effect::AggravatedStrike => c.sides[i].aggravated_strike = true,
            Effect::FirstStrike => c.sides[i].first_strike = true,
            // At once: the flow may move on (an oust).
            Effect::OpposingControllerBurnsPool(n) => {
                let seat = self.state.minions[c.sides[1 - i].minion.index()].owner;
                let mut loss = vec![0; self.state.seats.len()];
                loss[seat.index()] = n as i32;
                self.burn_pool_batch(&loss);
                if self.combat_ousts() {
                    self.ousted_during_play = true;
                }
            }
            Effect::PutOnOpponentOnHit { counters } => {
                c.sides[i].held = Some(Held::from_play(play, actor, counters));
            }
            Effect::PutOnActorAfterCombat => c.after_combat.push(Held::from_play(play, actor, 0)),
            Effect::StrengthThisRound(n) => c.sides[i].strength_round += n,
            Effect::PreventThisRound(n) => c.sides[i].round_prevents += n,
            Effect::PreventEachRound(n) => {
                c.sides[i].prevent_each_round += n;
                c.sides[i].round_prevents += n;
            }
            Effect::PressThisRound => c.sides[i].presses += 1,
            Effect::ActorBurnsBlood(n) => {
                self.state.burn_blood(actor, n);
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
            Effect::SetRange(r) => {
                if !c.range_set {
                    c.range = r;
                    c.range_set = true;
                }
            }
            // Immunity: the damage is gone, not prevented.
            Effect::ImmuneStrikeNonAggravated => {
                let s = &mut c.sides[i];
                let n = s.strike_damage.normal.min(s.pending.normal);
                s.pending.normal -= n;
                s.pending.gun = s.pending.gun.min(s.pending.normal);
                s.strike_damage.normal = 0;
                s.strike_damage.gun = 0;
            }
            Effect::ImmuneEffectNonAggravated => {
                let s = &mut c.sides[i];
                s.pending.normal = s.strike_damage.normal.min(s.pending.normal);
                s.pending.gun = s.pending.gun.min(s.pending.normal);
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
            Effect::OptionalCloseManeuver => c.sides[i].close_maneuvers += 1,
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
                let before = c.sides[i].pending;
                c.sides[i].pending.prevent(n, false);
                let by = c.sides[i].seat;
                c.settle_source(i, before, false, by, Some(i));
            }
            Effect::PreventNonGun(n) => {
                let d = c.sides[i].pending;
                if d.normal > d.gun && d.aggravated > 0 && n < d.total() - d.gun {
                    c.step = Step::Allocate {
                        side: i,
                        n,
                        by: c.sides[i].seat,
                        back: Some(i),
                    };
                } else {
                    let before = c.sides[i].pending;
                    c.sides[i].pending.prevent_non_gun(n);
                    let by = c.sides[i].seat;
                    c.settle_source(i, before, true, by, Some(i));
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
            Effect::DamageAfterCombatIfClose(n) => c.sides[i].after_damage_rider = Some(n),
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
        self.others_prevented = false;
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
        let (by, back) = (self.sides[i].seat, Some(i));
        self.prevent_by(i, n, by, back);
    }

    /// As `prevent`, by `by`'s card, the step going `back` afterward.
    fn prevent_by(&mut self, i: usize, n: u8, by: SeatId, back: Option<usize>) {
        let d = self.sides[i].pending;
        if d.normal > 0 && d.aggravated > 0 && n < d.total() {
            self.step = Step::Allocate {
                side: i,
                n,
                by,
                back,
            };
        } else {
            let before = self.sides[i].pending;
            self.sides[i].pending.prevent(n, true);
            self.settle_source(i, before, false, by, back);
        }
    }

    /// After a prevention on side `i` (from `before`): which points were
    /// strike damage — settled at once when only one split fits, else the
    /// preventer's pick (`Step::AllocateSource`; rules-qa 29.3). With
    /// `non_gun`, no gun point was prevented. True when the pick is asked.
    fn settle_source(
        &mut self,
        i: usize,
        before: Damage,
        non_gun: bool,
        by: SeatId,
        back: Option<usize>,
    ) -> bool {
        let now = self.sides[i].pending;
        let sd = self.sides[i].strike_damage;
        let (pa, pn) = (
            before.aggravated - now.aggravated,
            before.normal - now.normal,
        );
        let sa = sd.aggravated.min(before.aggravated);
        let sn = sd.normal.min(before.normal);
        let (oa, on) = (before.aggravated - sa, before.normal - sn);
        // Strike points that may have been covered: no gun point for a
        // prevention of other than gun damage.
        let sn_open = if non_gun { sn - sd.gun.min(sn) } else { sn };
        let (lo_a, hi_a) = (pa.saturating_sub(oa), pa.min(sa));
        let (lo_n, hi_n) = (pn.saturating_sub(on), pn.min(sn_open));
        if lo_a == hi_a && lo_n == hi_n {
            self.sides[i].strike_prevented(lo_a, lo_n);
            return false;
        }
        self.step = Step::AllocateSource {
            side: i,
            lo_a,
            hi_a,
            lo_n,
            hi_n,
            by,
            back,
        };
        true
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
            | Effect::AdditionalStrikeOnly { .. }
            | Effect::OptionalCloseManeuver
            | Effect::PreventOnTarget(_)
            | Effect::PreventOnTargetXPlus(_)
            | Effect::OpposingRestraint
            | Effect::OpposingCombatCardsCost(_)
            | Effect::FrenzyShield
            | Effect::FleshOfMarble { .. }
            | Effect::WeatherDamage { .. }
            | Effect::PutOnOpponent
            | Effect::OpponentToTorpor
            | Effect::AdditionalStrikesX { .. }
            | Effect::ActorBurnsBloodX
            | Effect::WeaponStrikePlus(_)
            | Effect::UndodgeableStrike
            | Effect::AggravatedStrike
            | Effect::FirstStrike
            | Effect::OpposingControllerBurnsPool(_)
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
            | Effect::DamageAfterCombatIfClose(_)
            | Effect::SetRange(_)
            | Effect::ImmuneStrikeNonAggravated
            | Effect::ImmuneEffectNonAggravated
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
            &PlayCtx::new(SeatId(1), 0, 0, Some(x), cards::Target::None, 0),
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
