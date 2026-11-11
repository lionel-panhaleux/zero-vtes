//! Combat (rulebook "Combat"; spec §4): `CombatCtx`, its round steps and
//! damage. A combat is part of its action (spec §4.1.2); each step is a
//! window over the combatants, the acting side first (rules-qa 1.2).

use crate::actions::{ActionState, PsycheNext};
use crate::cards::{self, Effect, PlayCtx, StrikeKind, Window};
use crate::decision::{Choice, DecisionRequest, WindowLabel};
use crate::game::{Engine, Flow};
use crate::ids::{MinionId, SeatId};
use crate::library::{Label, LIBRARY};
use crate::notes::{Cause, Rule, Source};
use crate::plays::ModeLayers;
use crate::state::{CardId, Expiry, GameState, Release, TimedStat};
use crate::window::OrderedWindow;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Range {
    #[default]
    Close,
    Long,
}

/// What a combat did, kept for a narrator when `Engine::notes` is
/// on: a combat whose windows all elide leaves no decision to tell it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatNote {
    /// The acting side first.
    Begins {
        minions: [MinionId; 2],
    },
    /// A pair of strikes resolved (`pair` 0: the normal pair): each side's
    /// strike (`None`: no strike) and what it was told to inflict as it
    /// resolved (`told`), the damage the side inflicts, before prevention
    /// (`damage`: its retainers' and environmental damage included), and
    /// its strike's share (`strike_damage`).
    Strikes {
        minions: [MinionId; 2],
        round: u8,
        pair: u8,
        range: Range,
        strikes: [Option<StrikeKind>; 2],
        told: [Option<StrikeHit>; 2],
        damage: [u8; 2],
        strike_damage: [u8; 2],
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
    /// A prevention of `n` covering part of mixed damage on `side`: `by`
    /// picks the aggravated points it covers (spec §4.5.2; owner
    /// 2026-09-04), then `back` to that side's prevention, None the others'.
    Allocate {
        side: usize,
        n: u8,
        by: SeatId,
        back: Option<usize>,
    },
    /// Points just prevented on `side`: `by` picks how many were strike
    /// damage, in `lo_a..=hi_a` / `lo_n..=hi_n` (Disarm reads it; rules-qa
    /// 29.3). Then `back`, as for `Allocate`.
    AllocateSource {
        side: usize,
        lo_a: u8,
        hi_a: u8,
        lo_n: u8,
        hi_n: u8,
        by: SeatId,
        back: Option<usize>,
    },
    /// Prevention by minions out of the combat, each Methuselah from the
    /// acting one (Martyr's Resilience; ANK 20181101).
    OthersPrevent,
    /// The damage about to land sends side `1 - i`'s vampire to torpor:
    /// side `i` may replace it (Decapitate [POT]).
    GoingToTorpor(usize),
    AdditionalStrikes,
    Press,
    EndOfRound,
}

/// The damage a strike inflicts on the opposing minion if it lands,
/// before prevention (`CombatCtx::strike_hit`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StrikeHit {
    pub n: u8,
    pub aggravated: bool,
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

    /// Prevent up to `n` points, aggravated first when allowed; returns
    /// what was prevented (a partial cover is `Step::Allocate`, rules-qa 4.2).
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

    /// Prevent up to `n` aggravated points.
    pub(crate) fn prevent_aggravated(&mut self, n: u8) {
        self.aggravated -= n.min(self.aggravated);
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

/// The copies of Cleave on the strike's weapon this action.
fn cleaves(action: Option<&crate::actions::ActionCtx>, source: Option<CardId>) -> u8 {
    action.map_or(0, |a| {
        a.cleaved.iter().filter(|&&w| Some(w) == source).count() as u8
    })
}

/// Side `i`'s printed bonus to a ranged strike, at either range
/// (Noluthando's damage, The Dracon's steal).
fn ranged_bonus(st: &GameState, c: &CombatCtx, i: usize, ranged: bool, stat: cards::Stat) -> i32 {
    if !ranged {
        return 0;
    }
    let ctx = crate::state::PrintedCtx {
        opponent: Some(c.sides[1 - i].minion),
        ..Default::default()
    };
    st.printed_mod(c.sides[i].minion, stat, ctx)
}

/// Darkness Within: 1 blood or life from `foe` to its `card`, if any left.
fn drain_one(st: &mut GameState, card: CardId, foe: MinionId) {
    if st.minions[foe.index()].blood > 0 {
        st.lose_blood(foe, 1);
        st.cards[card.index()].counters += 1;
    }
}

/// Does the weapon a strike came from carry rider `e` on its strike?
fn source_rider(st: &crate::state::GameState, source: Option<CardId>, e: Effect) -> bool {
    source.is_some_and(|w| {
        cards::lookup(st.cards[w.index()].card)
            .is_some_and(|d| d.abilities.iter().any(|m| m.effects.contains(&e)))
    })
}

/// Does the weapon's ability striking `k` carry rider `e` (Dagger: its
/// ranged strike's only)?
fn strike_rider(st: &crate::state::GameState, w: CardId, k: StrikeKind, e: Effect) -> bool {
    cards::lookup(st.cards[w.index()].card).is_some_and(|d| {
        d.abilities.iter().any(|m| {
            m.effects.contains(&e)
                && m.effects.iter().any(|x| {
                    matches!(x, Effect::Strike(s) if std::mem::discriminant(s) == std::mem::discriminant(&k))
                })
        })
    })
}

/// A combat card's delayed "put this card on …" (as its strike lands, after
/// combat): taken back from the ash heap, else an effect-only card in play
/// (rules-qa 21.1, 47.1; `Engine::place_held`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Held {
    pub card: u16,
    pub copy: crate::copies::Untracked<crate::copies::CopyId>,
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
            copy: play.copy,
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
    /// Its hand damage aggravated for the rest of the combat (Bone Spur
    /// [PRO]): each new round starts with `aggravated_hands`.
    pub aggravated_hands_combat: bool,
    /// Blood burned to damage this round (Taste of Vitae).
    pub lost_to_damage: u8,
    /// Damage landed on it this combat, every round (Franciscus).
    pub combat_damage: u8,
    /// Damage the opposing minion inflicts on it before range, mended as
    /// the before-range window closes (Outside the Hourglass [TEM]).
    pub pre_range: u8,
    /// Damage of the opposing strike its own strike prevents as they
    /// resolve (Stunt Cycle).
    pub strike_shield: u8,
    /// It may burn n blood to prevent the opposing strike's damage this
    /// round, if it has Celerity [cel] (Stunt Cycle's opponent); the
    /// card it pays for.
    pub celerity_burn_prevent: Option<(u8, Source)>,
    pub pending: Damage,
    /// The prevention offer is open (closes on a pass).
    pub preventing: bool,
    /// The going-to-torpor step offered to this side, this damage.
    pub torpor_offered: bool,
    /// Burned instead of going to torpor by this damage (Decapitate [POT]).
    pub burn_instead_of_torpor: bool,
    /// Diablerized instead of going to torpor by this damage, by the
    /// opposing vampire (Amaranth).
    pub diablerized_instead_of_torpor: bool,
    /// Wounded, its torpor waiting for the combat's end (Undead
    /// Persistence [for]); not in a new combat (LSJ 20030214).
    pub persisting: bool,
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
    /// This minion cannot use equipment this combat (Form of the Bat); set
    /// through `bar_equipment`, which keeps `equipment_barred` in step.
    pub no_equipment: bool,
    /// Form of Mist [PRO] rider, armed by the play, fired by the strike.
    pub continue_rider: Option<(u8, u8)>,
    /// Torrent [CEL]: "if this vampire was blocked while performing an
    /// action, the action continues as if unblocked" — mandatory, free.
    pub continue_free: bool,
    /// Catatonic Fear [PRE] rider, armed by the play, fired by the strike:
    /// n unpreventable damage on the opposing minion after combat.
    pub after_damage_rider: Option<u8>,
    /// `after_damage_rider` at any range (Arianne, The Conqueror).
    pub after_damage_any: bool,
    /// Kiss of Cathari [OBF]/[PRE]: its strike stuns the opposing minion
    /// after combat, at close range and not dodged.
    pub after_stun_rider: bool,
    /// Serpent's Numbing Kiss [PRE][SER]: its strike puts the card on the
    /// opposing minion after combat, at close range and not dodged.
    pub after_put_rider: Option<(Held, u8, bool)>,
    /// Chave de Braço [POT]: its card, put on the opposing minion as the
    /// round ends if the range is close and this side inflicted more.
    pub round_end_put: Option<Held>,
    /// Oubliette [OBT]: its strike makes the opposing minion burn n blood
    /// or life after combat, at any range, not dodged.
    pub after_burn_rider: Option<u8>,
    /// The card in play the chosen strike comes from (a weapon).
    pub strike_source: Option<CardId>,
    /// The card and mode played for the strike chosen, a strike card from
    /// hand (Increased Strength reads the Disciplines it required).
    pub strike_play: Option<(u16, u8)>,
    /// +n damage to its damaging strikes requiring the Discipline, this
    /// combat (Increased Strength).
    pub discipline_strike_bonus: Vec<(&'static str, u8)>,
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
    /// `committed` by setting the range (Sniper Rifle), not by a maneuver:
    /// Lucian's opponent may still decline it (LSJ 20071017 against LSJ
    /// 20090528-2-2).
    pub committed_range_set: bool,
    /// `committed` to this ability of the card only (Tinglestripe [MYT]'s
    /// 1R strike).
    pub committed_ability: Option<u8>,
    /// The card in play this round's initial strike came from (Joumlon's
    /// Axe: "must be or have been with this weapon").
    pub initial_with: Option<CardId>,
    /// The weapon whose strike on this side burns it if any of that damage
    /// is prevented, and the damage it dealt (Dagger).
    pub prevented_watch: Option<(CardId, u8)>,
    /// The weapon this side struck with, out of play once the damage is
    /// applied (Dagger's ranged strike).
    pub out_after: Option<CardId>,
    /// The additional strike granted by this weapon is only usable to
    /// strike with it, and may be declined (AK-47).
    pub additional_with: Option<CardId>,
    /// Optional presses only usable to continue combat (Righteous Blade).
    pub continue_presses: u8,
    /// Presses this round only usable to end combat (Cold Aura [NEC]).
    pub end_presses: u8,
    /// Optional presses only usable to continue, in any round of this
    /// combat (Harass).
    pub combat_continue_presses: u8,
    /// Additional strikes (limited) it gets each round of the combat
    /// (Octopod).
    pub strikes_each_round: u8,
    /// The round it cannot play strike cards (Coordinate Attacks).
    pub no_strike_cards_round: Option<u8>,
    /// No strike card this combat (Thoughts Betrayed [DOM]).
    pub no_strike_cards_combat: bool,
    /// The round it cannot maneuver in (Lapse [tem]).
    pub no_maneuver_round: Option<u8>,
    /// The round its initial strike is barred in (Lapse [TEM]).
    pub no_initial_strike_round: Option<u8>,
    /// (n, round): environmental damage the opposing minion takes that
    /// round (Thoughts Betrayed [dom]).
    pub environmental_round: Option<(u8, u8)>,
    /// Each opposing strike landing on it is half prevented, rounded up,
    /// the rest of the combat (Rockheart [VIS]).
    pub halve_strikes: bool,
    /// (n, round): unpreventable damage the opposing minion takes in that
    /// round's normal strike resolution (Gianna di Canneto).
    pub resolution_damage: Option<(u8, u8)>,
    /// +n to each damaging strike or effect against it, this combat (Dam
    /// the Heart's River: a frenzy effect on it).
    pub more_damage_taken: u8,
    /// Blood or life each strike of its not from a card costs it, this
    /// combat (Dam the Heart's River [QUI]), and `toll_due` its strike
    /// chosen and unpaid; it may then choose not to strike.
    pub strike_toll: u8,
    pub toll_due: bool,
    /// "If this strike is used at close range, this striking vampire also
    /// takes n environmental aggravated damage" (Breath of the Dragon): this
    /// round's strike.
    pub close_self_damage: u8,
    /// One optional maneuver or press for the combat, either (Mind Tricks
    /// [DEM]); `round_maneuver_or_press` for this round only (Hanna
    /// Redmonds's older opponent, the first).
    pub maneuver_or_press: u8,
    pub round_maneuver_or_press: u8,
    /// Optional presses usable in any round of this combat (Hunter's
    /// Mark).
    pub combat_presses: u8,
    /// This side cannot strike: combat ends in this round (Hunter's Mark
    /// [CEL][THA] on the opposing minion: the first).
    pub no_combat_ends_round: Option<u8>,
    /// This side cannot strike: dodge in this round (Sanguine Entrapment).
    pub no_dodge_round: Option<u8>,
    /// From the round after this one, each round of the combat, it may
    /// burn n blood to prevent the opposing initial strike's damage
    /// (Superior Mettle [FOR]), paid for that card; `burn_to_prevent_asked`:
    /// the round offered.
    pub burn_to_prevent: Option<(u8, u8, Source)>,
    pub burn_to_prevent_asked: Option<u8>,
    /// Barred from "strike: combat ends" this combat (Mind of the Wilds;
    /// set again in a later combat of the action).
    pub no_combat_ends_combat: bool,
    /// No maneuver nor strike this round (Ossian's Auspex-playing foe).
    pub frozen_round: Option<u8>,
    /// No strike this round, nor a strike's maneuver (The Status
    /// Perfectus: the acting minion, the first round; LSJ 20001126).
    pub no_strike_round: Option<u8>,
    /// Strength this side can gain for the combat before range is
    /// determined in the first round (Show of Force): `Choice::GainStrength`.
    pub strength_option: u8,
    /// A mandatory press, only usable to continue combat (Adrino
    /// Manauara): its controller cannot pass the press step while it can be
    /// used (rules-qa 14.1).
    pub mandatory_press: u8,
    /// As `mandatory_press`, only in the first round (Talbot's Chainsaw).
    pub first_round_press: u8,
    /// "+n strength that combat" (Kasim Bayar), kept each round.
    pub strength_mods: u8,
    /// "-n strength this combat" from the opposing side (Blight).
    pub strength_minus: u8,
    /// This combat, no additional strikes and no press to continue (Blight
    /// [NEC]): additional-strike cards not offered, none used.
    pub no_additional_strikes: bool,
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
    /// `round_strike` is good for one strike this round (Mythic Form).
    pub round_strike_once: bool,
    /// A strike it can use once each round of this combat (Mythic Form
    /// [PRO]): each round's `round_strike`, once.
    pub combat_strike: Option<StrikeKind>,
    /// Non-aggravated damage it can prevent this round, at will (Mythic
    /// Form), and each round of the combat.
    pub round_normal_prevents: u8,
    pub normal_prevent_each_round: u8,
    /// Environmental damage the opposing minion takes each round, this
    /// combat (Carrion Crows); `environmental_close` at close range only
    /// (Dark Steel).
    pub environmental: u8,
    pub environmental_close: u8,
    /// Environmental damage the opposing minion takes each round of this
    /// combat while the card is in play (Bestial Vengeance's retainer).
    pub environmental_while: Vec<(CardId, u8)>,
    /// Riders of the chosen strike: undodgeable (Dust Up [ani]),
    /// aggravated (Sculpt the Flesh [pro]), its card put on the opposing
    /// minion as it lands (`held`).
    pub undodgeable_strike: bool,
    pub aggravated_strike: bool,
    /// Damage its landing damage strike does to each retainer on the
    /// opposing minion (Hálito Negro).
    pub retainer_strike_damage: u8,
    /// Its strikes this round cannot be dodged (Sanguinary Wind).
    pub undodgeable_round: bool,
    /// The aim decision after this side's strike was chosen was passed
    /// (an aim is played on top of the chosen strike, owner 2026-10-11).
    pub aim_declined: bool,
    /// "Any effect which would make damage from this strike aggravated
    /// leaves the damage normal instead" (Psychic Assault).
    pub never_aggravated: bool,
    /// "If this striking vampire is Black Hand, the damage from this strike
    /// is unpreventable" (Psychic Assault), read as the strike resolves.
    pub unpreventable_if_black_hand: bool,
    /// Of `pending`, normal damage no effect may make aggravated (Psychic
    /// Assault's, against Dawn Operation).
    pub keep_normal: u8,
    /// Of `pending`, normal damage from a cleaved weapon, inflicted past an
    /// immunity to it (Cleave).
    pub cleaved_hit: u8,
    /// Of `pending`, normal damage mended for `mend_cost` blood a point
    /// (Sword of Nuln).
    pub costly_hit: u8,
    pub mend_cost: u8,
    /// Expiate on this imbued, copies (kept across rounds), and whether
    /// it burned the combatants.
    pub expiate: u8,
    pub expiated: bool,
    /// The chosen strike is done with first strike (Donnybrook [obf]).
    pub first_strike: bool,
    /// Its strikes this round are with first strike (Christopher
    /// Houghton).
    pub first_strike_round: bool,
    pub held: Option<Held>,
    /// The card going on this side's minion as its strike resolves, landed
    /// or not (Molotov Cocktail).
    pub held_self: Option<Held>,
    /// +n strength this round (Bear's Skin [ani][pro]).
    pub strength_round: u8,
    /// Damage this side can prevent each round of the combat (Bear's Skin
    /// [ANI][PRO]), added to `round_prevents` as each round begins.
    pub prevent_each_round: u8,
    /// Of `pending`, the damage from the opposing minion's strikes.
    pub strike_damage: Damage,
    /// This pair's damage the side inflicts on itself as it strikes
    /// (Breath of the Dragon, Zip Gun, Improvised Flamethrower): in its own
    /// `pending`, never the opponent's doing.
    pub self_damage: u8,
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
    /// Its strike, landing, burns the opposing minion if an ally
    /// (Entombment [obt]); nothing on a vampire.
    pub burns_ally_on_hit: bool,
    /// Its strike, landing, stuns the opposing minion (Father Anthony).
    pub stuns_on_hit: bool,
    /// Smite: the electronic equipment on both combatants burns as its
    /// strike resolves, dodged or not.
    pub burns_electronic: bool,
    /// Inflict: for the rest of the combat, the damage this minion takes
    /// from hand and melee weapon strikes and from retainers is inflicted
    /// on the striker or the retainer too, as normal damage, prevented or
    /// not.
    pub reflects: bool,
    /// The round's initial strike, committed by a card played at the
    /// maneuver (Slam [POT]).
    pub committed_card: Option<StrikeKind>,
    /// The weapon and bonus of a weapon strike committed at the maneuver
    /// (Lam Into [for]): its strike, from that card, once chosen.
    pub committed_weapon: Option<(CardId, u8)>,
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
    /// Set through `bar_equipment`, as `no_equipment`.
    pub frenzy_no_equipment: bool,
    /// Drawing Out the Beast on this vampire: no maneuver to long range, no
    /// press to end combat, +n strength, and n unpreventable environmental
    /// damage in each round's press step ([ANI]) — frenzy effects.
    pub no_long_maneuver: bool,
    pub no_end_press: bool,
    /// This round only, no equipment and no press to end combat (Frenzy).
    pub no_equipment_round: bool,
    pub no_end_press_round: bool,
    /// This combat, it cannot cancel frenzy cards (Taming the Beast).
    pub no_frenzy_cancel: bool,
    pub frenzy_strength: u8,
    pub press_damage: u8,
    /// Necrosis [THN] landed: that much more, this round's press step only.
    pub press_damage_round: u8,
    /// Its last strike was a hand strike (Shoulder Drop).
    pub hand_struck: bool,
    /// Its last strike was with a melee weapon (Blade Clot).
    pub melee_struck: bool,
    /// That strike inflicted damage (Shoulder Drop).
    pub hand_landed: bool,
    /// A hand strike of its inflicted damage this round, and in the
    /// previous one (Smallpox Griet).
    pub hand_landed_round: bool,
    pub hand_landed_last_round: bool,
    /// The blood Liquefy the Mortal Coil [vic] burns as its strike lands.
    pub liquefy_rider: Option<u8>,
    /// Elixir of Distillation's strike is on.
    pub distill_rider: Option<u8>,
    /// The round its initial strike may diablerize (Liquefy [VIC]).
    pub diablerie_round: Option<u8>,
    /// That strike is on.
    pub diablerie_rider: bool,
    /// Necrosis [THN]'s, until its strike resolves.
    pub press_damage_rider: Option<u8>,
    /// This minion cannot maneuver this combat (Ni Dieu ni Maître [ani]).
    pub no_maneuver: bool,
    /// Imprison, copies: for the rest of the combat, no maneuver, no press
    /// to continue, and 1 more blood or life a copy for each strike but
    /// "combat ends", the copies adding up as Archon's tolls (no frenzy:
    /// Tranquility Shield leaves it).
    pub imprisoned: u8,
    pub combat_card_surcharge: u8,
    pub frenzy_immune: bool,
    /// Flesh of Marble this combat: `Some(aggravated too)`.
    pub marble: Option<bool>,
    /// Damage was successfully inflicted on this minion this round.
    pub damaged_round: bool,
    /// How much this round, and in the previous one (Ecstatic Agony).
    pub damage_round: u8,
    pub damage_last_round: u8,
    /// Optional maneuvers this round, only usable to get to close range
    /// (Angel's Gift).
    pub close_maneuvers: u8,
    /// Optional maneuvers this combat, only usable to get to long range
    /// (Repulsion [pre]).
    pub long_maneuvers: u8,
    /// Combat ends once this side's strike has resolved (Anesthetic Touch
    /// [obe]).
    pub ends_combat_after: bool,
    /// Dance with the Devil [OBF]/[THA]: a maneuver to close range for
    /// this blood, once this round (0: none).
    pub close_maneuver_blood: u8,
    /// Quick Meld [PRO]: a maneuver, either way, for this blood, once this
    /// round (0: none).
    pub maneuver_blood: u8,
    /// Rage of Apedemak: +n damage on this minion's hand strikes this
    /// combat.
    pub hand_damage_bonus: u8,
    /// +n to this round's hand strike (Invoke Poison Glands [ABO]: its
    /// Orun).
    pub hand_damage_round: u8,
    /// This round, aggravated damage to it treated as normal (Jua Vema).
    pub aggravated_as_normal: bool,
    /// Ensnare a Beast [ANI]/[CEL]: this minion's strike cards cost +n
    /// blood or life this combat.
    pub strike_card_toll: u8,
    /// An aim on this minion's coming strike (Target Vitals): +n damage if
    /// it lands (0: none).
    pub aim: u8,
    /// An aim played or used on the chosen strike, canceled or not: one
    /// each strike (Target Vitals; LSJ 20100511).
    pub aim_played: bool,
    /// The aim card found in the library, due to be played as if from hand
    /// (Mikael Birkholm: `state::LIBRARY_PLAY`).
    pub library_aim: Option<u16>,
    /// Hit by an aimed strike: no press this round (Target Vitals).
    pub no_press_round: bool,
    /// Damage from this minion's strikes successfully inflicted on the
    /// opposing one this round (Disarm).
    pub inflicted_round: u8,
    /// The card named with the strike, taken as it lands (Absorb the Mind
    /// [MYT]).
    pub steal_card_on_hit: Option<CardId>,
    /// The strike's damage reduced to zero (Autonomic Mastery [DOM]).
    pub strike_zero: bool,
    /// `ends_combat_after` lapses if the strike is dodged (Autonomic
    /// Mastery [DOM]).
    pub ends_unless_dodged: bool,
    /// Additional hand strikes (limited) granted after each normal pair of
    /// the combat (Fractura [str]); `hand_only_strikes` those left this
    /// round, `hand_only_pair` this pair's strike one of them.
    pub hand_strikes_each_round: u8,
    pub hand_only_strikes: u8,
    pub hand_only_pair: bool,
    /// Unpreventable environmental damage the opposing minion takes in each
    /// round's normal strike resolution at close range (Blissful Agony
    /// [val]).
    pub resolution_damage_close: u8,
    /// The minion the opposing vampire enters combat with if this side's
    /// combat ends strike resolves undodged (Blissful Agony [VAL]).
    pub redirect_rider: Option<MinionId>,
    /// Pool its controller gains if the striker is burned by this strike's
    /// resolution (Eye of Unforgiving Heaven).
    pub pool_if_burned: u8,
    /// This strike's damage is unpreventable (Hexe [STR]); of the
    /// unpreventable damage on this side, the aggravated part.
    pub unpreventable_strike: bool,
    pub unpreventable_aggravated: u8,
    /// Blood it may burn once this round for a press only usable to
    /// continue (Lead Fist [vis]; 0: none).
    pub continue_press_blood: u8,
    /// Blood it may burn to unlock until the combat ends (Loving Agony
    /// [VAL]; 0: none).
    pub unlock_blood: u8,
    /// No maneuver to close range nor press to continue this combat, not a
    /// frenzy effect (Olid Loa).
    pub restrained: bool,
    /// `aggravated_as_normal` each round of the combat (Penitent Resilience
    /// [for][obe]).
    pub aggravated_as_normal_combat: bool,
    /// Card names with no effect on this minion, this combat or only the
    /// round given (Penitent Resilience: Flash Grenade).
    pub immune_to: Vec<(&'static str, Option<u8>)>,
    /// Riders of the chosen strike: landing, the opposing minion strikes no
    /// more this round; dodged, n damage to the striker and an optional
    /// press to the opposing minion (Pounce).
    pub bars_strikes_on_hit: bool,
    pub dodged_rider: u8,
    /// Melee weapons dealing +n damage (aggravated if set) in its strikes,
    /// this combat (Sword of the Righteous).
    pub weapon_bonus: Vec<(CardId, u8, bool)>,
}

impl Side {
    /// No additional strike for this side: Blight's, or a card on it
    /// (Decompose), read live.
    pub(crate) fn no_additional(&self, st: &GameState) -> bool {
        self.no_additional_strikes
            || st.hosts(self.minion, cards::Modifier::HostNoAdditionalStrikes)
    }

    /// What a strike of `kind` costs this minion (Dam the Heart's River
    /// [QUI], Imprison but for "combat ends").
    pub fn toll_for(&self, kind: Option<StrikeKind>) -> u8 {
        self.strike_toll + self.imprisoned_toll(kind)
    }

    /// Imprison's toll on a strike of `kind`: none on "combat ends".
    pub fn imprisoned_toll(&self, kind: Option<StrikeKind>) -> u8 {
        match kind {
            Some(StrikeKind::CombatEnds { .. }) => 0,
            _ => self.imprisoned,
        }
    }

    /// Any toll on its strikes: it may choose not to strike.
    pub fn tolled(&self) -> bool {
        self.strike_toll > 0 || self.imprisoned > 0
    }

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

    /// Prevent up to `n` of the pending damage from the opposing strike,
    /// aggravated first when `aggravated` (none otherwise).
    pub(crate) fn prevent_strike(&mut self, n: u8, aggravated: bool) {
        let (s, p) = (self.strike_damage, self.pending);
        let before = Damage {
            normal: s.normal.min(p.normal),
            aggravated: s.aggravated.min(p.aggravated),
            gun: s.gun.min(p.gun),
        };
        let mut d = before;
        d.prevent(n, aggravated);
        let (pa, pn) = (before.aggravated - d.aggravated, before.normal - d.normal);
        let p = &mut self.pending;
        p.aggravated -= pa;
        p.normal -= pn;
        p.gun = p.gun.saturating_sub(before.gun - d.gun).min(p.normal);
        self.strike_prevented(pa, pn);
    }

    /// This minion cannot use equipment this combat (Form of the Bat, a
    /// frenzy card).
    pub fn equipment_barred(&self) -> bool {
        self.no_equipment || self.frenzy_no_equipment || self.no_equipment_round
    }

    /// Bar this side from equipment this combat (`frenzy`: a frenzy card's,
    /// Tranquility Shield lifts it), its minion listed in
    /// `GameState::equipment_barred` too: its equipment's Disciplines are
    /// not its own meanwhile (rules-qa 76.16).
    pub(crate) fn bar_equipment(&mut self, st: &mut GameState, frenzy: bool) {
        if frenzy {
            self.frenzy_no_equipment = true;
        } else {
            self.no_equipment = true;
        }
        if !st.equipment_barred.contains(&self.minion) {
            st.equipment_barred.push(self.minion);
        }
    }

    /// Damage from the opposing minion's strike, unless those are prevented
    /// this round (Rolling with the Punches [FOR]); `gun`: from a gun.
    fn strike_hit(&mut self, n: u8, aggravated: bool, gun: bool) {
        if self.strikes_prevented {
            return;
        }
        // Dam the Heart's River: a damaging strike only (RTR 19960221).
        let n = if n > 0 { n + self.more_damage_taken } else { 0 };
        self.pending.add(n, aggravated);
        self.strike_damage.add(n, aggravated);
        if gun {
            self.pending.gun += n;
            self.strike_damage.gun += n;
        }
        // Rockheart [VIS]: half of each later strike, rounded up.
        if self.halve_strikes {
            self.prevent_strike(n.div_ceil(2), true);
        }
    }

    /// The strike chosen at +n damage (Strike with Conviction's weapon
    /// strike).
    pub(crate) fn raise_strike(&mut self, n: u8) {
        self.strike = match self.strike {
            Some(StrikeKind::Melee { bonus, aggravated }) => Some(StrikeKind::Melee {
                bonus: bonus + n,
                aggravated,
            }),
            Some(StrikeKind::Damage {
                n: d,
                ranged,
                aggravated,
            }) => Some(StrikeKind::Damage {
                n: d + n,
                ranged,
                aggravated,
            }),
            k => k,
        };
    }

    /// A new strike chosen: the old one's riders are gone.
    fn set_strike(&mut self, k: Option<StrikeKind>) {
        self.strike = k;
        self.strike_source = None;
        self.strike_play = None;
        self.fortitude_proof = false;
        self.nullifies_weapons = false;
        self.undodgeable_strike = false;
        self.aggravated_strike = false;
        self.retainer_strike_damage = 0;
        self.never_aggravated = false;
        self.unpreventable_if_black_hand = false;
        self.aim_declined = false;
        self.aim_played = false;
        self.first_strike = false;
        self.strike_weapon = None;
        self.steal_card_on_hit = None;
        self.strike_zero = false;
        self.ends_unless_dodged = false;
        self.redirect_rider = None;
        self.unpreventable_strike = false;
        self.bars_strikes_on_hit = false;
        self.dodged_rider = 0;
    }

    fn new_round(&mut self) {
        let maneuvers = self.next_round_maneuvers;
        *self = Self {
            strength_set: self.strength_set,
            strength_mods: self.strength_mods,
            strength_minus: self.strength_minus,
            no_additional_strikes: self.no_additional_strikes,
            combat_damage: self.combat_damage,
            damage_last_round: self.damage_round,
            hand_landed_last_round: self.hand_landed_round,
            combat_maneuvers: self.combat_maneuvers,
            combat_presses: self.combat_presses,
            persisting: self.persisting,
            maneuver_or_press: self.maneuver_or_press,
            diablerie_round: self.diablerie_round,
            long_maneuvers: self.long_maneuvers,
            combat_continue_presses: self.combat_continue_presses,
            no_combat_ends_round: self.no_combat_ends_round,
            no_dodge_round: self.no_dodge_round,
            burn_to_prevent: self.burn_to_prevent,
            burn_to_prevent_asked: self.burn_to_prevent_asked,
            no_combat_ends_combat: self.no_combat_ends_combat,
            frozen_round: self.frozen_round,
            no_strike_round: self.no_strike_round,
            no_equipment: self.no_equipment,
            no_close_maneuver: self.no_close_maneuver,
            no_continue_press: self.no_continue_press,
            frenzy_no_equipment: self.frenzy_no_equipment,
            no_long_maneuver: self.no_long_maneuver,
            no_end_press: self.no_end_press,
            no_frenzy_cancel: self.no_frenzy_cancel,
            frenzy_strength: self.frenzy_strength,
            press_damage: self.press_damage,
            no_maneuver: self.no_maneuver,
            imprisoned: self.imprisoned,
            expiate: self.expiate,
            expiated: self.expiated,
            reflects: self.reflects,
            combat_card_surcharge: self.combat_card_surcharge,
            hand_damage_bonus: self.hand_damage_bonus,
            strike_card_toll: self.strike_card_toll,
            frenzy_immune: self.frenzy_immune,
            marble: self.marble,
            combat_ends_round: self.combat_ends_round,
            environmental: self.environmental,
            environmental_close: self.environmental_close,
            environmental_while: std::mem::take(&mut self.environmental_while),
            aggravated_hands: self.aggravated_hands_combat,
            aggravated_hands_combat: self.aggravated_hands_combat,
            prevent_each_round: self.prevent_each_round,
            strikes_each_round: self.strikes_each_round,
            hand_strikes_each_round: self.hand_strikes_each_round,
            resolution_damage_close: self.resolution_damage_close,
            unlock_blood: self.unlock_blood,
            restrained: self.restrained,
            aggravated_as_normal: self.aggravated_as_normal_combat,
            aggravated_as_normal_combat: self.aggravated_as_normal_combat,
            immune_to: self.immune_to.clone(),
            weapon_bonus: self.weapon_bonus.clone(),
            combat_strike: self.combat_strike,
            round_strike: self.combat_strike,
            round_strike_once: self.combat_strike.is_some(),
            normal_prevent_each_round: self.normal_prevent_each_round,
            round_normal_prevents: self.normal_prevent_each_round,
            no_strike_cards_round: self.no_strike_cards_round,
            no_strike_cards_combat: self.no_strike_cards_combat,
            discipline_strike_bonus: self.discipline_strike_bonus.clone(),
            no_maneuver_round: self.no_maneuver_round,
            no_initial_strike_round: self.no_initial_strike_round,
            environmental_round: self.environmental_round,
            halve_strikes: self.halve_strikes,
            resolution_damage: self.resolution_damage,
            more_damage_taken: self.more_damage_taken,
            strike_toll: self.strike_toll,
            combat_prevents: self.combat_prevents,
            round_prevents: self.prevent_each_round,
            continue_rider: None,
            continue_free: false,
            after_damage_rider: None,
            after_stun_rider: false,
            after_burn_rider: None,
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

/// A strike effect landed (`CombatCtx::landed`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Landed {
    /// Liquefy the Mortal Coil [vic]: that blood from that vampire.
    Liquefy(MinionId, u8),
    /// Elixir of Distillation: that vampire and blood, if it landed, that
    /// weapon.
    Distill(Option<(MinionId, u8)>, CardId),
    /// Father Anthony: that minion stunned.
    Stun(MinionId),
}

#[derive(Clone, Debug, Default)]
pub struct CombatCtx {
    /// `sides[0]` is the acting minion's, `sides[1]` the opponent's.
    pub sides: [Side; 2],
    /// The Methuselahs in the game clockwise from the acting one, as the
    /// combat began (an ousted one dropped): every one is polled
    /// (`poll`), Martyr's Resilience's prevention in this order (rules-qa
    /// 29.1).
    pub ring: Vec<SeatId>,
    /// The combatants as the combat began, acting first, before Nar-Sheptha
    /// (`GameState::combat_order`; `Engine::reorder_combat`).
    pub natural: [MinionId; 2],
    /// Weapons whose strike resolution burned the opposing vampire (Sword
    /// of the Archangel).
    pub burned_by: Vec<CardId>,
    /// Weapons whose bearer took their once-each-combat damage for
    /// striking with them (Zip Gun).
    pub bearer_damaged: Vec<CardId>,
    /// Minions burned in this combat: (the minion burning them, their
    /// controller) — Predator's Transformation [ABO].
    pub burned_minions: Vec<(MinionId, SeatId)>,
    /// Each combatant's blood as last seen (Anathema: "reduced to 0 in
    /// combat" — a loss, not entering at 0; `Engine::settle_anathema`).
    pub anathema_seen: [u8; 2],
    pub round: u8,
    /// Strike pair within the round; 0 is the normal pair.
    pub pair: u8,
    pub range: Range,
    /// The range was set by an effect: skip the determine range step.
    pub range_set: bool,
    /// FBI Special Affairs Division: damage due to the acting vampire as
    /// the combat ends (an ally burned in it).
    pub fbi_damage: u8,
    /// The card inflicting it.
    pub fbi_card: Option<CardId>,
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
    /// Frenzy: this round has a press, only to continue, no minion's,
    /// before any other (LSJ 20040521, ANK 20200525-1).
    pub round_press: bool,
    /// The side that just pressed to continue, its opponent's printed toll
    /// to pay (Petra; `Engine::pay_press_toll`).
    pub press_toll: Option<usize>,
    /// The round ended early (a combat ends strike, a combatant no longer
    /// ready): straight to the end-of-round window.
    pub round_over: bool,
    /// Damage dealt in the end-of-round window awaits prevention (Pulled
    /// Fangs): the window resumes after it.
    pub end_of_round_damage: bool,
    /// Pre-range damage awaits prevention (Outside the Hourglass): the
    /// before-range step then closes.
    pub pre_range_damage: bool,
    /// A would-end replacement started a new round.
    pub continued: bool,
    /// Card names played per minion, for `Limit::PerRound` / `PerCombat`.
    pub played_round: Vec<(MinionId, &'static str)>,
    pub played_combat: Vec<(MinionId, &'static str)>,
    /// Card modes played per minion, for `Limit::PerCombatThisMode`.
    pub played_combat_modes: Vec<(MinionId, u16, u8)>,
    /// The cards each combatant played, canceled ones too, by copy (Marthe
    /// Dizier).
    pub played_cards: Vec<(MinionId, u16, crate::copies::CopyId)>,
    /// Weather Control this combat: the round it was played and whether
    /// its damage grows each subsequent round.
    pub weather: Vec<(u8, bool)>,
    /// The round Weather Control's damage was dealt or staged, and the
    /// amount staged with the pre-range damage (one window, rules-qa
    /// 78.32): applied after prevention, unpreventable.
    pub weather_round: u8,
    /// The previous round was at close range (Expiate).
    pub prev_close: bool,
    pub weather_staged: u8,
    /// "All damage inflicted on vampires during the resulting combat is
    /// aggravated" (Dawn Operation).
    pub all_aggravated: bool,
    /// Rötschreck played on this vampire: locked and sent to torpor as the
    /// combat ends, not if it continues.
    pub rotschreck: Option<MinionId>,
    /// A card was played this round, by anyone (Undead Persistence).
    pub card_played_round: bool,
    /// Rounds in a row with no card played since a combatant persists,
    /// and how many end the combat (Undead Persistence).
    pub quiet_rounds: u8,
    pub quiet_limit: Option<u8>,
    /// Hunting the Beast [AUS]: the acting vampire, and the uncontrolled
    /// clan vampire and blood its Methuselah adds at the end of the combat
    /// if it is ready and the opposing vampire is not.
    pub won_uncontrolled_blood: Option<(MinionId, &'static str, u8)>,
    /// Games of Instinct: at the end, the only ready combatant gains blood
    /// equal to the opposing vampire's capacity.
    pub lone_ready_gains_capacity: bool,
    /// Damage aimed at retainers in this strike pair, waiting for
    /// prevention like the combatants' (Martyr's Resilience).
    pub retainer_pending: Vec<(CardId, u8)>,
    /// The others' prevention window of this pair has run.
    pub others_prevented: bool,
    /// Potio Martyrium's damage just dealt, awaiting its prevention before
    /// the combat goes on (LSJ 20100205).
    pub martyrdom: bool,
    /// Byzar's burn replaced in an Amaranth's diablerie: his torpor goes
    /// through the going-to-torpor step again (ANK 20210424).
    pub back_to_torpor: bool,
    /// The strike waiting on a first strike (a vampire striking "with first
    /// strike", Elimelech): its side as chosen, resolved once the first
    /// strike's damage is applied, if that minion is still ready.
    pub held_strike: Option<(usize, Side)>,
    /// The held strike is resolving: the pair's retainer and environmental
    /// damage are already in.
    pub held_pass: bool,
    /// In-play abilities used this combat, by copy ("each combat").
    pub abilities_used: Vec<(cards::AbilitySource, u8)>,
    /// Abilities used this round ("each round", Rowena).
    pub abilities_used_round: Vec<(cards::AbilitySource, u8)>,
    /// Gun-strike prevention left this combat, by copy (Kevlar Vest).
    pub gun_prevention_left: Vec<(CardId, u8)>,
    /// The equipment on the blocker when it blocked (Sniper Rifle); empty
    /// unless this is the blocked minion's own combat (cleared after a new
    /// combat, Coordinate Attacks, a slave's swap).
    pub held_at_block: Vec<CardId>,
    /// The contracts on its minions as it began: (host, card, chosen).
    pub contracts: Vec<(MinionId, CardId, Vec<MinionId>)>,
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
    pub after_combat_damage: Vec<(MinionId, u8, MinionId)>,
    /// Meat Hook: the copies whose press on a hit was had this combat.
    pub hit_presses_had: Vec<CardId>,
    /// The ammo used on each gun this combat (Caseless Rounds,
    /// Scattershot): one a gun, its effect for the rest of the combat.
    pub ammo: Vec<(CardId, cards::Ammo)>,
    /// Kiss of Cathari [OBF]/[PRE]: the minions stunned after combat ends.
    pub after_combat_stun: Vec<MinionId>,
    /// Serpent's Numbing Kiss [PRE][SER], Morphean Blow [val]: the cards put
    /// on these minions after combat ends, their strikers burning that
    /// blood, the minion locked if set.
    pub after_combat_put: Vec<(Held, MinionId, u8, bool)>,
    /// Oubliette [OBT]: blood or life burned after combat ends, not damage
    /// (LSJ 20080226).
    pub after_combat_burn: Vec<(MinionId, u8, MinionId)>,
    /// A combat the action queued (Hedonism): the action's step resumes
    /// after it.
    pub queued: bool,
    /// Psyche! [CEL]: a combat with the same minions queued as this one
    /// ends.
    pub psyche: bool,
    /// Coordinate Attacks: the Blood Brother who locks and enters combat
    /// with the opposing minion as this one ends, in place of side `.1`'s
    /// minion, with an optional maneuver the first round if `.2`.
    pub coordinate: Option<(MinionId, usize, bool)>,
    /// The vampire taking that place is Marie-Pierre's, no strike barred.
    pub joined: bool,
    /// Marie-Pierre's offer made: the combat's end resumes past it.
    pub join_asked: bool,
    /// Haven Hunting's offers made, by side.
    pub again_asked: [bool; 2],
    /// Stand-ins that last the combat (Liquefy [VIC]).
    pub stand_ins: Vec<CardId>,
    /// Strike effects landed, applied once the strikes have resolved.
    pub landed: Vec<Landed>,
    /// Liquefy [VIC]'s strike landed: its diablerie, made as the damage
    /// applies.
    pub diablerie: Option<(MinionId, MinionId)>,
    /// A diablerie made in the combat (Amaranth's, Liquefy [VIC]'s): its
    /// blood hunt next, the combat suspended (rules-qa 113.10).
    pub diablerized: Option<(MinionId, MinionId)>,
    /// Abuse of Power: at the end of the combat, if only one combatant is
    /// ready, the controller of the other burns n pool.
    pub one_ready_burn: Option<(u8, Source)>,
    /// Taunt the Caged Beast [ANI]: at the end of the combat, if only one
    /// combatant is ready, it gains n blood.
    pub one_ready_gains: Option<u8>,
    /// Pillars Fall: the card put on the acting vampire at the end of the
    /// combat if the opposing vampire is not ready.
    pub put_if_opposing_down: Option<Held>,
    /// Yawp Court: the Sabbat vampire in combat with the acting vampire
    /// before the referendum, and the location.
    pub yawp: Option<(MinionId, CardId)>,
    /// Hunting the Quarry [cel]/[tha] played as the combat is about to
    /// end: no "would end" effect after it (ANK 20191219).
    pub end_fixed: bool,
    /// The minions sent to torpor during this combat.
    pub torpored: Vec<MinionId>,
    /// Morbidity: (the card in play holding the blood, the vampire it came
    /// from) — moved back as the combat ends and the card burns.
    pub morbidity: Vec<(CardId, MinionId)>,
    /// Trap and Conquer the Beast in play: the card, and for Conquer its
    /// vampire's side and whether its press is optional.
    pub traps: Vec<(CardId, Option<(usize, bool)>)>,
    /// Cards put on a combatant for this combat, burned as it ends (Inner
    /// Essence: "once this combat").
    pub burn_at_end: Vec<CardId>,
    /// Out of play until the combat ends, no action under way (Dagger).
    pub return_at_end: Vec<CardId>,
    /// Cards put in play this round ("not usable the round it is put in
    /// play", Molotov Cocktail).
    pub placed_this_round: Vec<CardId>,
    /// Blissful Agony [VAL]: the chosen minion the opposing vampire enters
    /// combat with as this one ends, and the striker's side; lost if the
    /// combat continues (RTR 20020501).
    pub redirect: Option<(MinionId, usize)>,
    /// Blissful Agony [VAL] by both: the side whose resolves first.
    pub redirect_first: Option<usize>,
    /// Darkness Within: (the card in play the blood moves to, its vampire),
    /// each round of this combat.
    pub drains: Vec<(CardId, MinionId)>,
    /// Darkness Within beside a steal strike: whether the drain goes first,
    /// as the acting Methuselah ordered it for this resolution.
    pub drain_first: Option<bool>,
}

/// A combat window's order: the acting Methuselah, the defending one,
/// then the others clockwise (rulebook "Combat": the impulse goes on to
/// every Methuselah; rules-qa 87.28).
fn poll(sides: &[Side; 2], ring: &[SeatId]) -> Vec<SeatId> {
    let mut order = vec![sides[0].seat];
    for &s in std::iter::once(&sides[1].seat).chain(ring) {
        if !order.contains(&s) {
            order.push(s);
        }
    }
    order
}

impl CombatCtx {
    /// "The acting vampire" of this combat for the action `a`: the one
    /// considered acting (side 0: Deep Song's target, Nar-Sheptha's
    /// opponent) in a combat of its acting minion, else that minion
    /// (Coordinate Attacks' Blood Brother's combat; rules-qa 106.11).
    pub fn acting_in(&self, a: &crate::actions::ActionCtx) -> MinionId {
        if self.natural.contains(&a.acting) {
            self.sides[0].minion
        } else {
            a.acting
        }
    }

    pub fn side_of(&self, seat: SeatId) -> Option<usize> {
        self.sides.iter().position(|s| s.seat == seat)
    }

    pub fn side_of_minion(&self, m: MinionId) -> Option<usize> {
        self.sides.iter().position(|s| s.minion == m)
    }

    /// Darkness Within's drains this round: (its card still in play, the
    /// minion opposing its vampire).
    fn live_drains(&self, st: &GameState) -> Vec<(CardId, MinionId)> {
        self.drains
            .iter()
            .filter(|&&(card, _)| st.cards[card.index()].present())
            .filter_map(|&(card, v)| {
                self.side_of_minion(v)
                    .map(|k| (card, self.sides[1 - k].minion))
            })
            .collect()
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
        let in_range = match s.strike {
            Some(StrikeKind::Hand { .. } | StrikeKind::Melee { .. }) => close,
            Some(StrikeKind::Damage { ranged, .. }) => ranged || close,
            Some(StrikeKind::RangedStrength { .. }) => true,
            _ => false,
        };
        in_range
            && s.strike
                .and_then(|k| self.strike_told(st, action, 1 - i, k))
                .is_some_and(|h| h.aggravated)
    }

    /// Side `i`'s strength this round: printed strength (Massimiliano;
    /// Kevin Jackson's, on him and on a Brujah opposing him) added to the
    /// strength of 1 or to a set strength (spec §4.4.1).
    pub(crate) fn strength_now(
        &self,
        st: &GameState,
        action: Option<&crate::actions::ActionCtx>,
        i: usize,
    ) -> u8 {
        let s = &self.sides[i];
        let ctx = crate::state::PrintedCtx {
            opponent: Some(self.sides[1 - i].minion),
            round: self.round,
            ..Default::default()
        };
        // The acting minion's strength this action (Invigorate).
        let action_bonus = match action {
            Some(a) if a.acting == s.minion => a.strength_bonus as i32,
            _ => 0,
        };
        // Strength from cards in play on the minion (Saulot's Avenging
        // Fist).
        let bearer = action.map_or(0, |a| {
            st.bearer_mod_in(s.minion, cards::Stat::Strength, a.kind, a.card_hunt)
        });
        // Ecstatic Agony: the damage it took last round.
        let agony = st
            .borne(s.minion)
            .filter(|l| l.modifier == cards::Modifier::HostStrengthPerLastRoundDamage)
            .count() as i32
            * s.damage_last_round as i32;
        (s.strength(st.base_strength(s.minion)) as i32 + action_bonus + s.strength_mods as i32
            - s.strength_minus as i32
            + s.frenzy_strength as i32
            + bearer
            + agony
            + st.strength_against(s.minion, self.sides[1 - i].minion)
            + st.lasting(s.minion).1
            + st.printed_mod(s.minion, cards::Stat::Strength, ctx))
        .max(0) as u8
            + s.strength_round
    }

    /// What strike `k` by side `i` inflicts on the opposing minion if it
    /// lands, before prevention, as the strike resolution reads it (`None`:
    /// not a damage strike). `strength`: side `i`'s, from `strength_now`.
    fn strike_hit(
        &self,
        st: &GameState,
        action: Option<&crate::actions::ActionCtx>,
        i: usize,
        k: StrikeKind,
        strength: u8,
    ) -> Option<StrikeHit> {
        let (s, opp) = (&self.sides[i], self.sides[1 - i].minion);
        // "Damage inflicted on a Kiasyd by this weapon is aggravated"
        // (Banshee Ironwail; Dagger's both strikes).
        let against_clan = || {
            let against = st.minions[opp.index()].clan();
            s.strike_source.is_some_and(|w| {
                cards::lookup(st.cards[w.index()].card).is_some_and(|d| {
                    d.abilities.iter().any(|m| {
                        m.effects.iter().any(|e| {
                            matches!(e, Effect::AggravatedAgainstClan(k) if against == Some(*k))
                        })
                    })
                })
            })
        };
        // "Each strike … made by the opposing minion against Nephandus …
        // inflicts 1 less damage": before prevention, unpreventable damage
        // included (LSJ 20100519).
        let less = st.damage_reduction(opp, s.minion);
        // "Damage Petra inflicts on Baali is aggravated": her strikes'.
        let ctx = crate::state::PrintedCtx {
            opponent: Some(opp),
            ..Default::default()
        };
        let printed = st.printed_mod(s.minion, cards::Stat::AggravatedDamage, ctx) > 0;
        // Harry Reese: +n on every damage-dealing strike, read on the
        // opponent; none on a strike dealing none (RTR 19960221).
        let plus = st
            .printed_mod(s.minion, cards::Stat::StrikeDamage, ctx)
            .max(0) as u8;
        let with = |n: u8| if n > 0 { n + plus } else { n };
        let mut hit = match k {
            StrikeKind::Hand { bonus } => StrikeHit {
                n: with(
                    strength
                        + bonus
                        + s.hand_damage_bonus
                        + s.hand_damage_round
                        + action
                            .filter(|a| a.acting == s.minion)
                            .map_or(0, |a| a.hand_damage),
                )
                .saturating_sub(less),
                aggravated: s.aggravated_hands
                    || printed
                    || st.printed_mod(s.minion, cards::Stat::AggravatedHandStrikes, ctx) > 0
                    || s.aggravated_strike
                    || st.minions[s.minion.index()]
                        .timed
                        .iter()
                        .any(|&(l, ..)| l == cards::Lasting::AggravatedHands)
                    || action.is_some_and(|a| a.aggravated_hands && a.acting == s.minion)
                    || st.bears(s.minion, cards::Modifier::BearerAggravatedHands),
            },
            StrikeKind::Melee { bonus, aggravated } => {
                let bonus = bonus
                    + st.printed_mod(s.minion, cards::Stat::MeleeDamage, ctx)
                        .max(0) as u8;
                let clan = against_clan();
                // Living Wood Staff: +X for an imbued.
                let x = s.strike_source.map_or(0, |w| {
                    let c = &st.cards[w.index()];
                    let marked = cards::lookup(c.card).is_some_and(|d| {
                        d.abilities
                            .iter()
                            .any(|m| m.effects.contains(&Effect::ImbuedStrikesPlusX))
                    });
                    if marked && st.minions[s.minion.index()].is_imbued() {
                        c.fixed_x
                    } else {
                        0
                    }
                });
                // Blessing of the Name, Blood Tempering: none on a strike
                // dealing none (RTR 19960221).
                let base = strength + bonus + x + cleaves(action, s.strike_source);
                // Sword of the Righteous: this weapon's, this combat.
                let (sword, sword_aggravated) = s
                    .weapon_bonus
                    .iter()
                    .filter(|&&(w, ..)| s.strike_source == Some(w))
                    .fold((0, false), |(n, a), &(_, k, g)| (n + k, a || g));
                let layered = if base > 0 {
                    st.melee_damage_bonus(s.minion, s.strike_source) + sword
                } else {
                    0
                };
                StrikeHit {
                    n: with(base + layered).saturating_sub(less),
                    aggravated: aggravated || printed || clan || sword_aggravated,
                }
            }
            StrikeKind::RangedStrength { bonus } => {
                let more = ranged_bonus(st, self, i, true, cards::Stat::RangedStrikeDamage);
                let n = with(((strength + bonus) as i32 + more).max(0) as u8);
                StrikeHit {
                    n: n.saturating_sub(less),
                    aggravated: (printed || s.aggravated_strike || against_clan())
                        && !s.never_aggravated,
                }
            }
            StrikeKind::Damage {
                n,
                ranged,
                aggravated,
            } => {
                // "+1 damage with ranged strikes (even at close range)"
                // (Noluthando): part of the strike's damage.
                let more = ranged_bonus(st, self, i, ranged, cards::Stat::RangedStrikeDamage);
                let n = with((n as i32 + more).max(0) as u8).saturating_sub(less);
                let gun = s
                    .strike_source
                    .is_some_and(|w| cards::is_gun(st.cards[w.index()].card));
                let by_range = s.strike_source.and_then(|w| {
                    self.ammo.iter().find_map(|&(g, a)| match a {
                        cards::Ammo::DamageByRange { close, long } if g == w => Some((close, long)),
                        _ => None,
                    })
                });
                let n = match (by_range, self.range) {
                    (Some((close, _)), Range::Close) => n.saturating_add(close),
                    (Some((_, long)), _) => n.saturating_sub(long),
                    _ => n,
                };
                // Joaquin Murietta: his guns at long range.
                let n = if gun && self.range != Range::Close {
                    let more = st.printed_mod(
                        s.minion,
                        cards::Stat::GunDamageAtLong,
                        crate::state::PrintedCtx::default(),
                    );
                    (n as i32 + more).max(0) as u8
                } else {
                    n
                };
                StrikeHit {
                    n,
                    aggravated: (aggravated || printed) && !s.never_aggravated,
                }
            }
            // Eye of Unforgiving Heaven: an opposing ally only if a demon.
            StrikeKind::Area { n, .. } => {
                if st.minions[opp.index()].is_ally()
                    && !st.ally_of(opp, &[crate::library::Label::Demon])
                {
                    return Some(StrikeHit {
                        n: 0,
                        aggravated: true,
                    });
                }
                let mut n = self.area_damage(st, i, n).saturating_sub(less);
                if let Some(cap) = st.damage_cap(opp) {
                    n = n.min(cap);
                }
                return Some(StrikeHit {
                    n,
                    aggravated: !s.never_aggravated,
                });
            }
            // Scourge of Alecto: read as it resolves; the excess over the
            // cap ignored, Dam the Heart's River's +n on the recipient
            // counted in (rules-qa 113.135).
            StrikeKind::CapacityGap { minus, cap } => {
                let seat = st.minions[s.minion.index()].owner;
                let mine = st.capacity_by(s.minion, seat).unwrap_or(0);
                let theirs = st.capacity_or_cost_by(opp, seat).unwrap_or(0);
                let damage = StrikeKind::Damage {
                    n: mine.saturating_sub(theirs).saturating_sub(minus),
                    ranged: true,
                    aggravated: false,
                };
                let mut hit = self.strike_hit(st, action, i, damage, strength)?;
                let taken = self.sides[1 - i].more_damage_taken;
                hit.n = hit.n.min(cap.saturating_sub(taken).max(1));
                return Some(hit);
            }
            _ => return None,
        };
        // Increased Strength: its strikes requiring the Discipline.
        let tags = self.strike_tags(st, i);
        hit.n += s
            .discipline_strike_bonus
            .iter()
            .filter(|(d, _)| tags.iter().any(|t| t.eq_ignore_ascii_case(d)))
            .map(|&(_, n)| n)
            .sum::<u8>();
        // Remnant of the Endless Storm: the excess ignored.
        if let Some(cap) = st.damage_cap(self.sides[1 - i].minion) {
            hit.n = hit.n.min(cap);
        }
        // Autonomic Mastery [DOM]: reduced to zero, every bonus included.
        if s.strike_zero {
            hit.n = 0;
        }
        Some(hit)
    }

    /// Side `i`'s area strike of `n` as its own bonuses raise it, the same
    /// on each of its recipients (Eye of Unforgiving Heaven, rules-qa
    /// 113.87); the opposing minion's own reductions are `strike_hit`'s.
    fn area_damage(&self, st: &GameState, i: usize, n: u8) -> u8 {
        let s = &self.sides[i];
        let ctx = crate::state::PrintedCtx {
            opponent: Some(self.sides[1 - i].minion),
            ..Default::default()
        };
        // Harry Reese: none on a strike dealing none (RTR 19960221).
        let plus = st
            .printed_mod(s.minion, cards::Stat::StrikeDamage, ctx)
            .max(0);
        let more = ranged_bonus(st, self, i, true, cards::Stat::RangedStrikeDamage);
        let tags = self.strike_tags(st, i);
        let disc: u8 = s
            .discipline_strike_bonus
            .iter()
            .filter(|(d, _)| tags.iter().any(|t| t.eq_ignore_ascii_case(d)))
            .map(|&(_, n)| n)
            .sum();
        let n = (n as i32 + more).max(0);
        let n = if n > 0 { n + plus } else { 0 };
        (n as u8).saturating_add(disc)
    }

    /// The Disciplines side `i`'s strike required: its in-play source's, as
    /// that card was played (Heroic Might, LSJ 20081120-2), else the strike
    /// card's mode played (rules-qa 38.4).
    fn strike_tags(&self, st: &GameState, i: usize) -> &'static [&'static str] {
        let s = &self.sides[i];
        if let Some(w) = s.strike_source {
            let c = &st.cards[w.index()];
            return LIBRARY[c.card as usize]
                .modes
                .get(cards::library_mode(c.card, c.mode as usize))
                .map_or(&[], |m| m.tags);
        }
        match s.strike_play {
            Some((card, mode))
                if cards::lookup(card).is_some_and(|d| {
                    d.modes
                        .get(mode as usize)
                        .is_some_and(|m| m.effects.iter().any(|e| matches!(e, Effect::Strike(_))))
                }) =>
            {
                LIBRARY[card as usize]
                    .modes
                    .get(cards::library_mode(card, mode as usize))
                    .map_or(&[], |m| m.tags)
            }
            _ => &[],
        }
    }

    /// `strike_hit` as a player is told it: the side's strength now, a
    /// weapon strike made harmless (Blood Rage) at no damage, and Dawn
    /// Operation's aggravated damage on vampires (`aggravate_vampires`
    /// applies it after the strikes; Psychic Assault's stays normal).
    pub fn strike_told(
        &self,
        st: &GameState,
        action: Option<&crate::actions::ActionCtx>,
        i: usize,
        k: StrikeKind,
    ) -> Option<StrikeHit> {
        let strength = self.strength_now(st, action, i);
        let mut h = self.strike_hit(st, action, i, k, strength)?;
        let s = &self.sides[i];
        if s.weapons_harmless
            && s.strike_source
                .is_some_and(|w| cards::is_weapon(st.cards[w.index()].card))
        {
            h.n = 0;
        }
        let damage_strike = matches!(k, StrikeKind::Damage { .. });
        if self.all_aggravated
            && st.minions[self.sides[1 - i].minion.index()].is_vampire()
            && !(damage_strike && self.sides[i].never_aggravated)
        {
            h.aggravated = true;
        }
        Some(h)
    }

    /// The end-of-round window, combat about to end (spec §4.9.1).
    pub fn would_end(&self) -> bool {
        self.step == Step::EndOfRound && !self.press_continue && !self.continued
    }

    fn order(&self) -> Vec<SeatId> {
        poll(&self.sides, &self.ring)
    }

    fn fresh_window(&mut self, step: Step) {
        self.step = step;
        self.window = OrderedWindow::new(self.order());
    }

    /// Play windows open at the current step: untimed cards before the
    /// resolution they affect (rules-qa 2.2); additional strikes after each
    /// pair, both combatants ready (rules-qa 47.2, 48.2).
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
            Step::GoingToTorpor(_) => vec![Window::CombatGoingToTorpor],
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
            Step::Prevent(_) | Step::OthersPrevent | Step::GoingToTorpor(_) => {
                WindowLabel::CombatPrevention
            }
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
    /// [ani]), Olid Loa, or against an imbued bearing The Crusader Sword,
    /// read live.
    pub(crate) fn may_maneuver(&self, st: &GameState, side: usize) -> bool {
        let s = &self.sides[side];
        let opponent = self.sides[1 - side].minion;
        let sworded = st.minions[s.minion.index()].is_vampire()
            && st.minions[opponent.index()].is_imbued()
            && st.bears(opponent, cards::Modifier::OpposingVampiresNoCloseManeuver);
        // Sword of Troile's bearer, read live.
        let held_close = st.bears(s.minion, cards::Modifier::BearerNoManeuverToLong);
        !((s.no_close_maneuver || s.restrained || sworded) && self.range == Range::Long)
            && !((s.no_long_maneuver || held_close) && self.range == Range::Close)
            && !s.no_maneuver
            && s.imprisoned == 0
            && s.no_maneuver_round != Some(self.round)
            && !self.frozen(side)
    }

    /// Side `side` can use no maneuver nor strike this round (Ossian).
    pub(crate) fn frozen(&self, side: usize) -> bool {
        self.sides[side].frozen_round == Some(self.round)
    }

    /// Side `side` cannot strike this round: neither combatant (One With
    /// the Land [PRO]), or this side (The Status Perfectus).
    pub(crate) fn strikes_barred(&self, side: usize) -> bool {
        self.no_strikes_round == Some(self.round)
            || self.sides[side].no_strike_round == Some(self.round)
            || (self.pair == 0 && self.sides[side].no_initial_strike_round == Some(self.round))
    }

    /// A press now would end the combat and the side's minion cannot press
    /// to end it (Don Michael Antonio Giovanni), or continue it and it
    /// cannot press to continue (Mukhtar Bey; others' presses for him
    /// still count, LSJ 20040521).
    pub(crate) fn end_press_barred(&self, st: &GameState, side: usize) -> bool {
        let stat = if self.press_continue {
            cards::Stat::NoPressToEnd
        } else {
            cards::Stat::NoPressToContinue
        };
        st.printed_mod(
            self.sides[side].minion,
            stat,
            crate::state::PrintedCtx::default(),
        ) > 0
    }

    /// The side's first-round mandatory presses left, none past round 1
    /// (Talbot's Chainsaw).
    pub(crate) fn first_round_press(&self, side: usize) -> u8 {
        if self.round == 1 {
            self.sides[side].first_round_press
        } else {
            0
        }
    }

    /// A press now: none to continue under a frenzy (Terror Frenzy [ani]) or
    /// Olid Loa.
    pub(crate) fn may_press(&self, side: usize) -> bool {
        !((self.sides[side].no_continue_press
            || self.sides[side].restrained
            || self.sides[side].imprisoned > 0)
            && !self.press_continue)
            && !((self.sides[side].no_end_press || self.sides[side].no_end_press_round)
                && self.press_continue)
            && !self.sides[side].no_press_round
    }

    /// Only hand strikes for side `i` now: a grapple's (Immortal Grapple)
    /// or this additional strike's (Fractura).
    pub(crate) fn hand_only(&self, i: usize) -> bool {
        self.only_hand_strikes || self.pair > 0 && self.sides[i].hand_only_pair
    }

    /// Cards named as `card` have no effect on side `i`'s minion now
    /// (Penitent Resilience: Flash Grenade).
    pub(crate) fn immune_to(&self, i: usize, card: u16) -> bool {
        let name = LIBRARY[card as usize].name;
        self.sides[i]
            .immune_to
            .iter()
            .any(|&(n, r)| n == name && r.is_none_or(|r| r == self.round))
    }

    /// Can `mode` be played by `side` at this step (no two maneuvers or
    /// presses in a row, one limited additional strike, hand strikes under
    /// a grapple, one strike commitment a round: rulebook "Determine
    /// Range")?
    pub(crate) fn allows(&self, st: &GameState, mode: &cards::ModeDef, side: usize) -> bool {
        self.allows_from(st, mode, side, None)
    }

    /// `allows`, for an ability of card `from`: the strike already committed
    /// to that card commits again (Deer Rifle's second maneuver).
    pub(crate) fn allows_from(
        &self,
        st: &GameState,
        mode: &cards::ModeDef,
        side: usize,
        from: Option<CardId>,
    ) -> bool {
        // "Minions in combat with the employer cannot strike: combat ends"
        // (Dog Pack; Faruq Abd al-Qadir for minions with his Methuselah's
        // corruption counters).
        let no_ends = st.combat_ends_barred(self.sides[side].minion, self.sides[1 - side].minion)
            || self.sides[side].no_combat_ends_round == Some(self.round)
            || self.sides[side].no_combat_ends_combat;
        let frozen = self.frozen(side);
        let only_evade =
            st.strikes_only_dodge_or_ends(self.sides[side].minion, self.sides[1 - side].minion);
        let no_ranged = || {
            st.hosts(
                self.sides[side].minion,
                cards::Modifier::HostNoRangedStrikes,
            )
        };
        mode.effects.iter().all(|e| match e {
            // Shadowed Eyes: no strike usable at long range, nor a gun's
            // maneuver committing to its strike (LSJ 20001126).
            Effect::Strike(k) | Effect::CommittedStrike(k) if k.ranged() && no_ranged() => false,
            Effect::CommitStrike | Effect::CommitStrikeOf(_)
                if no_ranged()
                    && from.is_some_and(|c| {
                        cards::lookup(st.cards[c.index()].card).is_some_and(|d| {
                            d.abilities
                                .iter()
                                .flat_map(|a| a.effects)
                                .any(|e| matches!(e, Effect::Strike(k) if k.ranged()))
                        })
                    }) =>
            {
                false
            }
            Effect::Strike(_)
            | Effect::Maneuver
            | Effect::CommitStrike
            | Effect::CommitStrikeOf(_)
            | Effect::CommittedStrike(_)
            | Effect::WeaponStrikePlus(_)
            | Effect::CommittedWeaponStrikePlus(_)
                if frozen =>
            {
                false
            }
            Effect::Strike(StrikeKind::CombatEnds { .. }) if no_ends => false,
            // Tanginé against a Salubri: a dodge or a combat ends only.
            Effect::Strike(k) | Effect::CommittedStrike(k)
                if only_evade
                    && !matches!(k, StrikeKind::Dodge | StrikeKind::CombatEnds { .. }) =>
            {
                false
            }
            Effect::CommitStrike
            | Effect::CommitStrikeOf(_)
            | Effect::WeaponStrikePlus(_)
            | Effect::CommittedWeaponStrikePlus(_)
                if only_evade =>
            {
                false
            }
            Effect::Strike(StrikeKind::Dodge)
                if self.sides[side].no_dodge_round == Some(self.round) =>
            {
                false
            }
            // Gabi: no dodge against her the first round.
            Effect::Strike(StrikeKind::Dodge)
                if self.round == 1
                    && st.printed_mod(
                        self.sides[1 - side].minion,
                        cards::Stat::OpposingNoDodgeFirstRound,
                        crate::state::PrintedCtx::default(),
                    ) > 0 =>
            {
                false
            }
            // "Cannot maneuver to close range" (Terror Frenzy [ani]).
            Effect::Maneuver if self.step == Step::DetermineRange => {
                self.last_maneuver != Some(side) && self.may_maneuver(st, side)
            }
            // Nor the maneuver of a strike that cannot be used (LSJ
            // 20001126).
            Effect::CommitStrike | Effect::CommitStrikeOf(_) => {
                let committed = self.sides[side].committed;
                (committed.is_none() || committed == from && from.is_some())
                    && !self.strikes_barred(side)
            }
            Effect::SetRange(_) => !self.range_set,
            // A strike card played at the maneuver: a maneuver in turn, one
            // commitment a round, a strike that can be used (LSJ 20021028).
            Effect::CommittedStrike(k) if self.step == Step::DetermineRange => {
                self.last_maneuver != Some(side)
                    && self.may_maneuver(st, side)
                    && self.sides[side].committed.is_none()
                    && self.sides[side].committed_card.is_none()
                    && !self.strikes_barred(side)
                    && (!self.hand_only(side) || matches!(k, StrikeKind::Hand { .. }))
            }
            // The weapon's strike is not a hand strike (rules-qa 55.8).
            Effect::CommittedWeaponStrikePlus(_) if self.step == Step::DetermineRange => {
                self.last_maneuver != Some(side)
                    && self.may_maneuver(st, side)
                    && self.sides[side].committed.is_none()
                    && self.sides[side].committed_card.is_none()
                    && !self.strikes_barred(side)
                    && !self.hand_only(side)
            }
            Effect::WeaponStrikePlus(_) | Effect::CommittedWeaponStrikePlus(_) => {
                !self.hand_only(side)
            }
            Effect::Press if self.step == Step::Press => {
                self.last_press != Some(side)
                    && self.may_press(side)
                    && !self.end_press_barred(st, side)
            }
            // One additional-strike card at a time: a combatant with an
            // additional strike still to make gains no other before that
            // pair resolves (rules-qa 48.2: Quickness after Pursuit's pair).
            Effect::AdditionalStrikes { limited: true, .. }
            | Effect::AdditionalStrikesX { limited: true }
            | Effect::AdditionalStrikeOnly { limited: true, .. }
            | Effect::AdditionalHandStrikesEachRoundX => {
                !self.sides[side].limited_used
                    && self.sides[side].additional == 0
                    && !self.sides[side].no_additional(st)
            }
            Effect::AdditionalStrikes { .. }
            | Effect::AdditionalStrikesX { .. }
            | Effect::AdditionalStrikeOnly { .. } => {
                self.sides[side].additional == 0 && !self.sides[side].no_additional(st)
            }
            Effect::Strike(k) if self.hand_only(side) => matches!(k, StrikeKind::Hand { .. }),
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

/// Side `i`'s vampire, by the damage pending on it as it stands, would go
/// to torpor, not be burned (Decapitate [POT]: "going into torpor"); the
/// victim of Liquefy [VIC]'s landed diablerie is being diablerized, not
/// going to torpor (rules-qa 113.155).
pub(crate) fn going_to_torpor(st: &crate::state::GameState, c: &CombatCtx, i: usize) -> bool {
    let s = &c.sides[i];
    let m = &st.minions[s.minion.index()];
    // Averted by the first card played (RTR 20010711).
    if m.is_ally()
        || m.in_torpor
        || s.persisting
        || s.burn_instead_of_torpor
        || s.diablerized_instead_of_torpor
        || c.diablerie.is_some_and(|(_, v)| v == s.minion)
    {
        return false;
    }
    let mut d = s.pending;
    d.normal += s.unpreventable;
    d.aggravated += s.unpreventable_aggravated;
    let r = resolve_damage(m.blood, d, false);
    (r.wounded || s.sent_to_torpor) && !r.burned
}

/// Side `i`'s minion, by the damage pending on it, is being burned or going
/// to torpor (Decapitate [POT]: "not usable by a vampire being burned or
/// going into torpor"); the victim of Liquefy [VIC]'s landed diablerie is
/// being burned (rules-qa 113.155).
pub(crate) fn going_down(st: &crate::state::GameState, c: &CombatCtx, i: usize) -> bool {
    let s = &c.sides[i];
    if c.diablerie.is_some_and(|(_, v)| v == s.minion) {
        return true;
    }
    let m = &st.minions[s.minion.index()];
    let mut d = s.pending;
    d.normal += s.unpreventable;
    d.aggravated += s.unpreventable_aggravated;
    let r = if m.is_ally() {
        resolve_life_damage(m.blood, d)
    } else {
        resolve_damage(m.blood, d, m.in_torpor)
    };
    r.wounded || r.burned || s.sent_to_torpor
}

/// Normal damage first: 1 blood per point mends it, unmended points wound.
/// Then aggravated: no mending, the first point wounds; each further point
/// on a wounded vampire burns 1 blood or the vampire (rulebook examples
/// Nassir, Tamoszius, Ryan). A vampire in torpor is already wounded
/// (rulebook "Wounded"): `in_torpor` makes every aggravated point burn, and
/// `Resolved::wounded` reports only a vampire newly wounded.
pub(crate) fn resolve_damage(blood: u8, d: Damage, in_torpor: bool) -> Resolved {
    resolve_damage_costly(blood, d, in_torpor, 0, 1)
}

/// As `resolve_damage`, `costly` of the normal points mended for `cost`
/// blood each (Sword of Nuln): short, the blood left is paid, the point
/// unmended (LSJ 20090901).
pub(crate) fn resolve_damage_costly(
    blood: u8,
    d: Damage,
    in_torpor: bool,
    costly: u8,
    cost: u8,
) -> Resolved {
    let mut blood = blood;
    let mut burned_blood = 0;
    let costly = costly.min(d.normal);
    let plain = d.normal - costly;
    let mend = blood.min(plain);
    blood -= mend;
    burned_blood += mend;
    let mut short = plain > mend;
    for _ in 0..costly {
        let paid = blood.min(cost);
        blood -= paid;
        burned_blood += paid;
        short |= paid < cost;
    }
    let mut wounded = in_torpor || short;
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
        wounded: wounded && !in_torpor,
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
        let natural = [acting, opposing];
        let [first, second] = self.state.combat_order(acting, opposing);
        // "None of your minions enter combat" (rulebook "Withdrawing from
        // the Game").
        for m in [acting, opposing] {
            let owner = self.state.minions[m.index()].owner;
            self.state.seats[owner.index()].withdrawal.r#break();
        }
        // "When Michael enters combat, you may draw 1 card" (Michael Luther).
        for m in [acting, opposing] {
            let owner = self.state.minions[m.index()].owner;
            for r in self.printed_riders(m, crate::cards::Hook::HostEntersCombat, None) {
                // Padraic Conroy: the combat of his block of an older vampire.
                if r.requires.contains(&crate::cards::Requirement::BlocksOlder)
                    && !self.action.as_ref().is_some_and(|a| {
                        a.blocker == Some(m) && self.state.younger_than(m, a.acting)
                    })
                {
                    continue;
                }
                for e in r.effects {
                    match *e {
                        crate::cards::Effect::DiscardAtRandom(n) => {
                            let k = (n as usize).min(self.state.seats[owner.index()].hand.len());
                            self.discard_at_random(owner, k);
                            self.replace(owner, k, None);
                        }
                        crate::cards::Effect::DrawThenDiscardDown(n) => {
                            for _ in 0..n {
                                let seat = &mut self.state.seats[owner.index()];
                                if let Some((c, copy)) = seat.library.take_top() {
                                    seat.hand.put(c, copy);
                                    self.offer_redraw(owner, c);
                                }
                            }
                        }
                        other => unreachable!("not an enters-combat effect: {other:?}"),
                    }
                }
            }
        }
        let st = &self.state;
        let mut sides = [
            Side::new(first, st.minions[first.index()].owner),
            Side::new(second, st.minions[second.index()].owner),
        ];
        for (i, s) in sides.iter_mut().enumerate() {
            let opponent = if i == 0 { second } else { first };
            // Persisting through the combat just ended: its torpor due now,
            // the protection gone (LSJ 20030214).
            s.sent_to_torpor = st.minions[s.minion.index()].torpor_after_combat;
            s.mandatory_press = st
                .printed_mod(
                    s.minion,
                    crate::cards::Stat::MandatoryContinuePress,
                    crate::state::PrintedCtx::default(),
                )
                .max(0) as u8;
            // "Brujah get 1 optional maneuver each combat" (Brujah Debate).
            s.combat_maneuvers += st.clan_combat_maneuvers(s.minion);
            // "1 optional press each combat" (Serhat Gunde; Dread Mastiff
            // [ANI] on its employer).
            s.combat_presses += st.combat_presses_of(s.minion, opponent);
            s.first_round_press = st.first_round_presses_of(s.minion);
            // Highway Haven: RV: 1 optional press to end combat, the first
            // round.
            s.end_presses += st
                .borne(s.minion)
                .filter(|l| l.modifier == crate::cards::Modifier::BearerFirstRoundEndPress)
                .count() as u8;
            s.environmental_close += st
                .printed_mod(
                    s.minion,
                    crate::cards::Stat::EnvironmentalClose,
                    crate::state::PrintedCtx::default(),
                )
                .max(0) as u8;
            s.presses += st
                .printed_mod(
                    s.minion,
                    crate::cards::Stat::FirstRoundPress,
                    crate::state::PrintedCtx::default(),
                )
                .max(0) as u8;
            s.round_maneuver_or_press = st
                .printed_mod(
                    s.minion,
                    crate::cards::Stat::FirstRoundManeuverOrPress,
                    crate::state::PrintedCtx {
                        opponent: Some(opponent),
                        ..Default::default()
                    },
                )
                .max(0) as u8;
            // Barred from combat ends for the action (Mind of the Wilds).
            s.no_combat_ends_combat = self
                .action
                .as_ref()
                .is_some_and(|a| a.no_combat_ends.contains(&s.minion));
        }
        // NSA Trio: the turn's first combat of an acting vampire and the
        // vampire that blocked it, counted whether the event is in play or
        // not (LSJ 20050215).
        let block_combat = !at_resolution
            && self
                .action
                .as_ref()
                .is_some_and(|a| a.acting == acting && a.blocker == Some(opposing))
            && st.minions[acting.index()].is_vampire()
            && st.minions[opposing.index()].is_vampire();
        if block_combat && st.vampire_block_combat_turn != Some(st.turn) {
            if st
                .global(crate::cards::Modifier::NoCombatEndsFirstBlockCombat)
                .is_some()
            {
                for s in &mut sides {
                    s.no_combat_ends_round = Some(1);
                }
            }
            self.state.vampire_block_combat_turn = Some(self.state.turn);
        }
        let ring = self.state.clockwise_from(sides[0].seat);
        let order = poll(&sides, &ring);
        self.state.combatants = Some([first, second]);
        let st = &self.state;
        let held_at_block = if at_resolution {
            Vec::new()
        } else {
            st.equipment_on(opposing)
        };
        let anathema_seen = [
            self.state.minions[first.index()].blood,
            self.state.minions[second.index()].blood,
        ];
        let contracts = (0..st.cards.len())
            .map(|k| CardId(k as u32))
            .filter_map(|id| {
                let c = &st.cards[id.index()];
                let host = c.host.filter(|&h| h == acting || h == opposing)?;
                (c.present() && LIBRARY[c.card as usize].labels.contains(&Label::Contract))
                    .then(|| (host, id, c.chosen.clone()))
            })
            .collect();
        self.combat = Some(CombatCtx {
            sides,
            ring,
            natural,
            anathema_seen,
            round: 1,
            step: Step::BeforeRange,
            window: OrderedWindow::new(order),
            held_at_block,
            at_resolution,
            contracts,
            ..Default::default()
        });
        self.flow = Flow::Combat;
        self.note(CombatNote::Begins {
            minions: [acting, opposing],
        });
        self.freeze_auspex_foes();
        self.bar_small_foes_first_round();
        self.turnips_move([acting, opposing]);
    }

    /// Blood Turnip: each moves with 1 blood or life, converted to the
    /// opposing minion's (LSJ 20080704); an Anarch gets the blood and moves
    /// the card, which must move (LSJ 20080704).
    fn turnips_move(&mut self, pair: [MinionId; 2]) {
        // Read for both first: one moved is not moved back.
        let turnips: Vec<(MinionId, MinionId, CardId)> = (0..2)
            .flat_map(|i| {
                let (from, to) = (pair[i], pair[1 - i]);
                self.state
                    .borne(from)
                    .filter(|l| l.modifier == cards::Modifier::MovesAsCombatBegins)
                    .map(move |l| (from, to, l.source))
                    .collect::<Vec<_>>()
            })
            .collect();
        for (from, to, card) in turnips {
            if self.state.minions[from.index()].blood > 0 {
                let moved = Cause::Effect(Source::Card(card));
                let last = self.caused(moved, |e| {
                    let last = e.state.burn_blood_or_life(from, 1);
                    let b = e.state.minions[to.index()].blood + 1;
                    e.state.set_blood(to, b);
                    last
                });
                if last {
                    self.lifeless(from);
                }
            }
            if self.state.sect_of(to) == Some(crate::crypt::Sect::Anarch) {
                let seat = self.state.minions[to.index()].owner;
                self.interrupts.push(crate::draws::Interrupt::MinionOf {
                    seat,
                    of: seat,
                    purpose: crate::draws::MinionPurpose::MoveOntoAny(card, to),
                });
            } else {
                self.state.set_host(card, to);
            }
        }
    }

    /// Ossian: in the first round, a vampire opposing him that played a
    /// card requiring Auspex during this action can use no maneuver nor
    /// strike — read at the combat's start and as each card is played.
    pub(crate) fn freeze_auspex_foes(&mut self) {
        let (Some(c), Some(a)) = (self.combat.as_mut(), self.action.as_ref()) else {
            return;
        };
        if c.round != 1 {
            return;
        }
        for i in 0..2 {
            let (m, foe) = (c.sides[i].minion, c.sides[1 - i].minion);
            if self.state.minions[m.index()].is_vampire()
                && a.disciplines_played.contains(&(m, "aus"))
                && self
                    .state
                    .borne(foe)
                    .any(|l| l.modifier == cards::Modifier::FreezesAuspexFoeFirstRound)
            {
                c.sides[i].frozen_round = Some(1);
            }
        }
    }

    /// Rock Cat: an opposing vampire of capacity n or less cannot strike in
    /// the first round, read at the combat's start and as each card is
    /// played (LSJ 20001126: nor its strikes' maneuvers).
    pub(crate) fn bar_small_foes_first_round(&mut self) {
        let Some(c) = self.combat.as_mut() else {
            return;
        };
        if c.round != 1 {
            return;
        }
        for i in 0..2 {
            let (m, foe) = (c.sides[i].minion, c.sides[1 - i].minion);
            let foe_owner = self.state.minions[foe.index()].owner;
            let small = |n: u8| {
                self.state
                    .capacity_by(m, foe_owner)
                    .is_some_and(|cap| cap <= n)
            };
            if self.state.minions[m.index()].is_vampire()
                && self.state.borne(foe).any(|l| {
                    matches!(l.modifier,
                        cards::Modifier::FoesUpToCapacityNoStrikeFirstRound(n) if small(n))
                })
            {
                c.sides[i].no_strike_round = Some(1);
            }
        }
    }

    pub(crate) fn note(&mut self, n: CombatNote) {
        // A pair's note still waiting for a held strike: out before the end.
        let waiting = matches!(n, CombatNote::Ends { .. })
            .then(|| self.held_note.take())
            .flatten();
        for n in waiting.into_iter().chain([n]) {
            self.push_note(crate::notes::Note::Combat(n));
        }
    }

    /// `n` noted at `at`, before the notes since, if none was drained
    /// since (a decision between a first strike's two passes).
    fn insert_note(&mut self, at: Option<(usize, u64)>, n: CombatNote) {
        let drained = self.notes_drained;
        match (self.notes.as_mut(), at) {
            (Some(notes), Some((at, d))) if d == drained && at <= notes.len() => {
                notes.insert(at, crate::notes::Note::Combat(n))
            }
            _ => self.note(n),
        }
    }

    /// Where the next note goes, while the notes are not drained.
    fn note_at(&self) -> Option<(usize, u64)> {
        self.notes.as_ref().map(|n| (n.len(), self.notes_drained))
    }

    /// The aims `seat` may play on the strike its minion just chose (Target
    /// Vitals), a strike committed at the maneuver included (LSJ 20080409;
    /// rules-qa 55.8).
    fn aim_options(&self, seat: SeatId, windows: &[Window]) -> Vec<Choice> {
        let hand = self.virtual_hand(seat);
        self.play_options(seat, windows, None)
            .into_iter()
            .filter(|o| match *o {
                Choice::Play { card, .. } => hand
                    .get(card as usize)
                    .is_some_and(|&(lib, _)| LIBRARY[lib as usize].labels.contains(&Label::Aim)),
                _ => false,
            })
            // An ability aiming the chosen strike (Mikael Birkholm).
            .chain(
                self.ability_options(seat, Window::CombatStrikeDeclaration)
                    .into_iter()
                    .filter(|o| match *o {
                        Choice::UseAbility {
                            source, ability, ..
                        } => source
                            .mode(&self.state, ability)
                            .requires
                            .contains(&cards::Requirement::NoAimYet),
                        _ => false,
                    }),
            )
            .collect()
    }

    /// The aim card found in the library (Mikael Birkholm), due as its
    /// seat's next play: its plays, none if its requirements are not met.
    pub(crate) fn library_play_due(&self) -> Option<(SeatId, Vec<Choice>)> {
        let c = self.combat.as_ref()?;
        let side = c.sides.iter().find(|s| s.library_aim.is_some())?;
        let hand = self.virtual_hand(side.seat);
        let options: Vec<Choice> = self
            .play_options(side.seat, &c.windows(), None)
            .into_iter()
            .filter(|o| {
                matches!(*o, Choice::Play { card, actor, .. }
                    if actor == Some(side.minion)
                        && hand.get(card as usize).map(|&(_, f)| f)
                            == Some(Some(crate::state::LIBRARY_PLAY)))
            })
            .collect();
        (!options.is_empty()).then_some((side.seat, options))
    }

    /// The strike choice among `options` (none without one), with the
    /// any-impulse abilities and side `retainers`'s retainer targets.
    fn strike_request(
        &self,
        seat: SeatId,
        mut options: Vec<Choice>,
        retainers: Option<usize>,
    ) -> Option<DecisionRequest> {
        if options.is_empty() {
            return None;
        }
        // Dam the Heart's River [QUI]: a toll it cannot pay leaves it no
        // strike but a strike card costing it (LSJ 20090703, rules-qa D3).
        let c = self.combat.as_ref().expect("combat context");
        if let Some(i) = c.side_of(seat) {
            let s = &c.sides[i];
            let blood = self.state.minions[s.minion.index()].blood;
            let printed = self.printed_strike_toll(i);
            if s.strike_toll + printed > blood {
                options.retain(|o| matches!(o, Choice::Pass | Choice::Play { .. }));
                if !options.contains(&Choice::Pass) {
                    options.insert(0, Choice::Pass);
                }
            } else if s.toll_for(None) + printed > blood {
                // Imprison: a strike but "combat ends" it cannot pay.
                options.retain(|o| {
                    matches!(
                        o,
                        Choice::Pass | Choice::Play { .. } | Choice::StrikeCombatEnds
                    )
                });
                if !options.contains(&Choice::Pass) {
                    options.insert(0, Choice::Pass);
                }
            }
        }
        options.extend(self.any_impulse_options(seat));
        if let Some(i) = retainers {
            options = self.with_retainer_targets(i, options);
        }
        // Earthshock: not against a minion with flight, its retainers still
        // aimed at (LSJ 20060509): read after the aims are added.
        let c = self.combat.as_ref().expect("combat context");
        if let Some(i) = c.side_of(seat) {
            if self.state.discipline_level(c.sides[1 - i].minion, "flight") > 0 {
                options.retain(|o| !self.strike_barred_against_flight(seat, o));
            }
        }
        Some(DecisionRequest::new(
            seat,
            WindowLabel::CombatStrike,
            options,
        ))
    }

    /// Spiritual Guidance resolving: its player and the plays of an ash
    /// heap combat card they can make now (rules-qa 67.4).
    pub(crate) fn ash_play_due(&self) -> Option<(SeatId, Vec<Choice>)> {
        let c = self.combat.as_ref()?;
        let seat = c
            .sides
            .iter()
            .map(|s| s.seat)
            .find(|s| self.state.seats[s.index()].ash_combat_play)?;
        let hand = self.virtual_hand(seat);
        let options: Vec<Choice> = self
            .play_options(seat, &c.windows(), None)
            .into_iter()
            .filter(|o| {
                matches!(*o, Choice::Play { card, .. }
                    if hand.get(card as usize).map(|&(_, f)| f)
                        == Some(Some(crate::state::ASH_PLAY)))
            })
            .collect();
        (!options.is_empty()).then_some((seat, options))
    }

    /// Hanna Redmonds' first-round maneuver or press, its "older vampire"
    /// read live (the counter only spends it).
    fn round_maneuver_or_press(&self, c: &CombatCtx, i: usize) -> u8 {
        let live = self.state.printed_mod(
            c.sides[i].minion,
            crate::cards::Stat::FirstRoundManeuverOrPress,
            crate::state::PrintedCtx {
                opponent: Some(c.sides[1 - i].minion),
                ..Default::default()
            },
        );
        c.sides[i].round_maneuver_or_press.min(live.max(0) as u8)
    }

    pub(crate) fn combat_request(&self) -> Option<DecisionRequest> {
        let c = self.combat.as_ref().expect("combat context");
        // Spiritual Guidance's play from the ash heap: its resolution, the
        // impulse not going around (owner, 2026-10-12: rules-qa 67.4).
        if let Some((seat, options)) = self.ash_play_due().or_else(|| self.library_play_due()) {
            return Some(DecisionRequest::new(seat, c.label(), options));
        }
        match c.step {
            Step::ChooseStrike(i) => {
                let side = &c.sides[i];
                // The strike chosen: an aim on top of it, or pass (owner,
                // 2026-10-11: rules-qa 62.1).
                if side.chosen {
                    if side.strike.is_none() || side.aim > 0 || side.aim_declined || side.aim_played
                    {
                        return None;
                    }
                    let aims = self.aim_options(side.seat, &c.windows());
                    if aims.is_empty() {
                        return None;
                    }
                    let mut options = vec![Choice::Pass];
                    options.extend(aims);
                    return Some(DecisionRequest::new(
                        side.seat,
                        WindowLabel::CombatStrike,
                        options,
                    ));
                }
                // A committed strike: that card's strike only — none at all
                // if an effect now bars it (spec §4.3.1); AK-47's additional
                // strike only with the gun, or none.
                let committed = if c.pair == 0 {
                    side.committed
                } else {
                    side.additional_with
                };
                // Sword of Troile: at close range, each strike with it if it
                // can strike.
                let forced = committed
                    .is_none()
                    .then(|| {
                        (c.range == Range::Close)
                            .then(|| {
                                self.state
                                    .borne(side.minion)
                                    .find(|l| l.modifier == cards::Modifier::CloseStrikesWithThis)
                                    .map(|l| l.source)
                            })
                            .flatten()
                    })
                    .flatten();
                let (c_pair, committed_ability) = (c.pair, side.committed_ability);
                if let Some(w) = committed.or(forced) {
                    // Or that weapon's strike made through a card or an
                    // ability using it (Strike with Conviction at +1, as
                    // Target Vitals aims it: rules-qa 87.42).
                    let st = &self.state;
                    let through = |mode: &cards::ModeDef, t: cards::Target| {
                        t == cards::Target::Card(w)
                            && mode
                                .effects
                                .iter()
                                .any(|e| matches!(e, Effect::WeaponStrikePlus(_)))
                    };
                    let hand = &st.seats[side.seat.index()].hand;
                    let mut options: Vec<Choice> = self
                        .play_options(side.seat, &c.windows(), None)
                        .into_iter()
                        .chain(self.ability_options(side.seat, Window::CombatStrikeDeclaration))
                        .filter(|o| match *o {
                            Choice::UseAbility {
                                source: cards::AbilitySource::Card(c),
                                ability,
                                ..
                            } if c == w => {
                                c_pair > 0 || committed_ability.is_none_or(|a| a == ability)
                            }
                            Choice::UseAbility {
                                source,
                                ability,
                                target,
                                ..
                            } => through(source.mode(st, ability), target),
                            Choice::Play {
                                card, mode, target, ..
                            } => hand
                                .get(card as usize)
                                .and_then(|&h| cards::lookup(h))
                                .is_some_and(|d| through(&d.modes[mode as usize], target)),
                            _ => false,
                        })
                        .collect();
                    // Dam the Heart's River [QUI] after the commitment, and
                    // Lucian after a range set: it may still choose not to
                    // strike (LSJ 20071017).
                    if c.pair > 0
                        || side.tolled()
                        || side.committed_range_set && self.printed_strike_toll(i) > 0
                    {
                        options.insert(0, Choice::Pass);
                    }
                    if committed.is_some() || !options.is_empty() {
                        return self.strike_request(side.seat, options, Some(i));
                    }
                }
                // An additional strike only usable to burn a weapon
                // (Voracious Vermin [ANI]): none if no weapon is left.
                if c.pair > 0 && side.additional_only == Some(StrikeKind::BurnWeapon) {
                    let opposing = c.sides[1 - i].minion;
                    let mut options = Vec::new();
                    if !c.hand_only(i) {
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
                    let evade = !self
                        .state
                        .strikes_only_dodge_or_ends(c.sides[i].minion, c.sides[1 - i].minion)
                        || matches!(k, StrikeKind::Dodge | StrikeKind::CombatEnds { .. });
                    let usable = (!c.hand_only(i) || matches!(k, StrikeKind::Hand { .. })) && evade;
                    let mut options: Vec<Choice> = usable
                        .then_some(Choice::CommittedStrike)
                        .into_iter()
                        .collect();
                    if side.tolled() || !evade {
                        options.push(Choice::Pass);
                    }
                    return self.strike_request(side.seat, options, None);
                }
                let only_evade = self
                    .state
                    .strikes_only_dodge_or_ends(c.sides[i].minion, c.sides[1 - i].minion);
                let mut options = vec![if only_evade {
                    Choice::Pass
                } else {
                    Choice::HandStrike
                }];
                // Dam the Heart's River [QUI], Imprison, Lucian: it may
                // choose not to strike — a strike committed aside (LSJ
                // 20090528-2-2).
                if !only_evade && (side.tolled() || self.printed_strike_toll(i) > 0) {
                    options.push(Choice::Pass);
                }
                // A granted "strike: combat ends" this round, unless combat
                // ends strikes or strikes other than hand strikes are
                // barred (Dog Pack, a grapple).
                if side.combat_ends_round == Some(c.round)
                    && side.no_combat_ends_round != Some(c.round)
                    && !side.no_combat_ends_combat
                    && !c.hand_only(i)
                    && !self
                        .state
                        .combat_ends_barred(c.sides[i].minion, c.sides[1 - i].minion)
                {
                    options.push(Choice::StrikeCombatEnds);
                }
                // A strike granted for the round (Hunger of Marduk).
                if let Some(k) = side.round_strike {
                    let evades = matches!(k, StrikeKind::Dodge | StrikeKind::CombatEnds { .. });
                    if (!c.hand_only(i) || matches!(k, StrikeKind::Hand { .. }))
                        && (!only_evade || evades)
                    {
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
                    || side.round_normal_prevents > 0 && side.pending.normal > 0
                {
                    options.push(Choice::UsePrevent);
                }
                // A minion out of the combat prevents in the others' step
                // (Randall).
                options.extend(
                    self.impulse_options(side.seat, &c.windows())
                        .into_iter()
                        .filter(|o| !self.out_of_combat_ability(o)),
                );
                Some(DecisionRequest::new(
                    side.seat,
                    WindowLabel::CombatPrevention,
                    options,
                ))
            }
            Step::GoingToTorpor(i) => {
                let side = &c.sides[i];
                if side.torpor_offered {
                    return None;
                }
                // What this moment offers only, as it opened (an
                // any-impulse ability had the prevention window).
                let w = Window::CombatGoingToTorpor;
                let mut options = vec![Choice::Pass];
                options.extend(self.play_options(side.seat, &[w], None));
                options.extend(self.ability_options(side.seat, w));
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
                for &w in &c.windows() {
                    options.extend(
                        self.ability_options(seat, w)
                            .into_iter()
                            .filter(|o| self.out_of_combat_ability(o)),
                    );
                }
                Some(DecisionRequest::new(
                    seat,
                    WindowLabel::CombatPrevention,
                    options,
                ))
            }
            _ => {
                let seat = c.window.current()?;
                let mut options = vec![Choice::Pass];
                let mut mandatory = false;
                // A combatant's own options; any Methuselah's cards and
                // abilities after.
                match (c.step, c.side_of(seat)) {
                    (Step::DetermineRange, Some(i))
                        if (c.sides[i].maneuvers > 0
                            || c.sides[i].combat_maneuvers > 0
                            || c.sides[i].maneuver_or_press > 0
                            || self.round_maneuver_or_press(c, i) > 0
                            || (c.sides[i].close_maneuvers > 0 && c.range == Range::Long)
                            || (c.sides[i].long_maneuvers > 0 && c.range == Range::Close)
                            || (c.sides[i].close_maneuver_blood > 0
                                && c.range == Range::Long
                                && self.state.minions[c.sides[i].minion.index()].blood
                                    >= c.sides[i].close_maneuver_blood)
                            || (c.sides[i].maneuver_blood > 0
                                && self.state.minions[c.sides[i].minion.index()].blood
                                    >= c.sides[i].maneuver_blood))
                            && c.last_maneuver != Some(i)
                            && c.may_maneuver(&self.state, i)
                            // "Cannot maneuver to long range" (Pedro Cortez).
                            && !(c.range == Range::Close
                                && self.state.printed_mod(
                                    c.sides[i].minion,
                                    cards::Stat::NoManeuverToLong,
                                    crate::state::PrintedCtx::default(),
                                ) > 0) =>
                    {
                        options.push(Choice::UseManeuver);
                    }
                    // Show of Force: the strength, before range is
                    // determined in the first round.
                    (Step::BeforeRange, Some(i))
                        if c.round == 1 && c.sides[i].strength_option > 0 =>
                    {
                        options.push(Choice::GainStrength);
                    }
                    (Step::Press, Some(i))
                        if c.last_press != Some(i)
                            && c.may_press(i)
                            && !c.end_press_barred(&self.state, i)
                            && (c.sides[i].presses > 0
                                || c.sides[i].combat_presses > 0
                                || c.sides[i].maneuver_or_press > 0
                                || self.round_maneuver_or_press(c, i) > 0
                                || ((c.sides[i].continue_presses > 0
                                    || c.sides[i].combat_continue_presses > 0
                                    || c.sides[i].mandatory_press > 0
                                    || c.first_round_press(i) > 0)
                                    && !c.press_continue)
                                || (c.sides[i].continue_press_blood > 0
                                    && !c.press_continue
                                    && self.state.minions[c.sides[i].minion.index()].blood
                                        >= c.sides[i].continue_press_blood)
                                || (c.sides[i].end_presses > 0 && c.press_continue)) =>
                    {
                        options.push(Choice::UsePress);
                        // Mandatory only while it can be used: to continue,
                        // with both combatants ready (rules-qa 14.1).
                        mandatory = (c.sides[i].mandatory_press > 0 || c.first_round_press(i) > 0)
                            && !c.press_continue
                            && c.both_ready(&self.state);
                    }
                    _ => {}
                }
                // Loving Agony [VAL]: until the combat ends, after the
                // end-of-round effects too (ANK 20180618).
                if let Some(i) = c.side_of(seat) {
                    let (m, n) = (c.sides[i].minion, c.sides[i].unlock_blood);
                    let v = &self.state.minions[m.index()];
                    if n > 0 && v.locked && v.ready() && v.blood >= n {
                        options.push(Choice::UnlockForBlood { minion: m });
                    }
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
        if let (
            Some((seat, _)),
            Choice::Play {
                card,
                mode,
                actor,
                target,
            },
        ) = (
            self.ash_play_due().or_else(|| self.library_play_due()),
            choice,
        ) {
            self.announce_play(seat, *card, *mode, *actor, *target, Flow::Combat);
            return;
        }
        let c = self.combat.as_mut().expect("combat context");
        // A maneuver's or a press's blood, paid after the borrow ends.
        let mut maneuver_paid = None;
        // Loving Agony [VAL]: the blood burned, the vampire unlocked.
        let mut unlocked_for = None;
        // A committed weapon strike, made after the borrow ends.
        let mut weapon_due = None;
        // The seat playing a card through the as-played stack or using an
        // ability (announced or used after the borrow ends).
        let acting = match (c.step, choice) {
            (Step::ChooseStrike(i), Choice::StrikeCombatEnds) => {
                c.sides[i].set_strike(Some(StrikeKind::CombatEnds { unlock: false }));
                c.sides[i].chosen = true;
                c.sides[i].toll_due = true;
                None
            }
            (Step::ChooseStrike(i), Choice::HandStrike) => {
                c.sides[i].set_strike(Some(StrikeKind::Hand { bonus: 0 }));
                c.sides[i].chosen = true;
                c.sides[i].toll_due = true;
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
                // A committed weapon strike (Lam Into [for]): from that
                // weapon, still on this minion — gone, no strike.
                match c.sides[i].committed_weapon.take().filter(|_| c.pair == 0) {
                    Some((w, n)) => {
                        let m = c.sides[i].minion;
                        let card = &self.state.cards[w.index()];
                        if card.present() && card.host == Some(m) {
                            weapon_due = Some((m, w, n));
                        } else {
                            c.sides[i].set_strike(None);
                            c.sides[i].chosen = true;
                        }
                    }
                    None => {
                        c.sides[i].set_strike(k);
                        c.sides[i].chosen = true;
                        // An additional strike is granted, not a card's: it
                        // pays Dam's toll (LSJ 20090529).
                        c.sides[i].toll_due = c.pair > 0;
                    }
                }
                None
            }
            (Step::ChooseStrike(i), Choice::BurnWeaponStrike { weapon }) => {
                c.sides[i].set_strike(Some(StrikeKind::BurnWeapon));
                c.sides[i].strike_weapon = Some(CardId(*weapon));
                c.sides[i].additional_only = None;
                c.sides[i].chosen = true;
                c.sides[i].toll_due = true;
                None
            }
            (Step::ChooseStrike(i), Choice::RoundStrike { at }) => {
                let k = c.sides[i].round_strike;
                if std::mem::take(&mut c.sides[i].round_strike_once) {
                    c.sides[i].round_strike = None;
                }
                c.sides[i].set_strike(k);
                c.sides[i].strike_at = *at;
                c.sides[i].chosen = true;
                c.sides[i].toll_due = true;
                None
            }
            // No aim on the chosen strike.
            (Step::ChooseStrike(i), Choice::Pass) if c.sides[i].chosen => {
                c.sides[i].aim_declined = true;
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
            (Step::GoingToTorpor(i), Choice::Pass) => {
                c.sides[i].torpor_offered = true;
                None
            }
            (Step::GoingToTorpor(i), Choice::Play { .. }) => {
                c.sides[i].torpor_offered = true;
                Some(c.sides[i].seat)
            }
            // The narrower allowance first (Mythic Form: non-aggravated only).
            (Step::Prevent(i), Choice::UsePrevent)
                if c.sides[i].round_normal_prevents > 0 && c.sides[i].pending.normal > 0 =>
            {
                c.sides[i].round_normal_prevents -= 1;
                let before = c.sides[i].pending;
                c.sides[i].pending.prevent(1, false);
                let by = c.sides[i].seat;
                c.settle_source(i, before, false, by, Some(i));
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
            (
                Step::Prevent(i) | Step::ChooseStrike(i) | Step::GoingToTorpor(i),
                Choice::UseAbility { .. },
            ) => Some(c.sides[i].seat),
            (_, Choice::UseAbility { .. }) => {
                let seat = c.window.current().expect("impulse holder");
                c.window.play();
                Some(seat)
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
                } else if c.range == Range::Close && c.sides[i].long_maneuvers > 0 {
                    c.sides[i].long_maneuvers -= 1;
                } else if c.sides[i].maneuvers == 0
                    && c.sides[i].combat_maneuvers == 0
                    && c.range == Range::Long
                    && c.sides[i].close_maneuver_blood > 0
                {
                    // Dance with the Devil: the blood paid as it is used.
                    let n = std::mem::take(&mut c.sides[i].close_maneuver_blood);
                    maneuver_paid = Some((c.sides[i].minion, n));
                } else if c.sides[i].maneuvers > 0 {
                    c.sides[i].maneuvers -= 1;
                } else if c.sides[i].combat_maneuvers > 0 {
                    c.sides[i].combat_maneuvers -= 1;
                } else if c.sides[i].round_maneuver_or_press > 0 {
                    c.sides[i].round_maneuver_or_press -= 1;
                } else if c.sides[i].maneuver_or_press > 0 {
                    c.sides[i].maneuver_or_press -= 1;
                } else {
                    // Quick Meld [PRO]: its blood as it is used.
                    let n = std::mem::take(&mut c.sides[i].maneuver_blood);
                    maneuver_paid = Some((c.sides[i].minion, n));
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
                if !c.press_continue && c.first_round_press(i) > 0 {
                    c.sides[i].first_round_press -= 1;
                } else if !c.press_continue && c.sides[i].mandatory_press > 0 {
                    c.sides[i].mandatory_press -= 1;
                } else if !c.press_continue && c.sides[i].continue_presses > 0 {
                    c.sides[i].continue_presses -= 1;
                } else if !c.press_continue && c.sides[i].combat_continue_presses > 0 {
                    c.sides[i].combat_continue_presses -= 1;
                } else if c.press_continue && c.sides[i].end_presses > 0 {
                    c.sides[i].end_presses -= 1;
                } else if c.sides[i].presses > 0 {
                    c.sides[i].presses -= 1;
                } else if c.sides[i].combat_presses > 0 {
                    c.sides[i].combat_presses -= 1;
                } else if c.sides[i].round_maneuver_or_press > 0 {
                    c.sides[i].round_maneuver_or_press -= 1;
                } else if c.sides[i].maneuver_or_press > 0 {
                    c.sides[i].maneuver_or_press -= 1;
                } else {
                    // Lead Fist [vis]: its blood as it is used.
                    let n = std::mem::take(&mut c.sides[i].continue_press_blood);
                    maneuver_paid = Some((c.sides[i].minion, n));
                }
                c.window.play();
                c.press(i);
                None
            }
            (_, Choice::UnlockForBlood { minion }) => {
                let i = c.side_of_minion(*minion).expect("a combatant");
                let n = std::mem::take(&mut c.sides[i].unlock_blood);
                c.window.play();
                unlocked_for = Some((*minion, n));
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
        if let Some((m, n)) = maneuver_paid {
            let paid = Cause::Cost(Source::Minion(m));
            self.caused(paid, |e| e.state.lose_blood(m, n));
        }
        if let Some((m, n)) = unlocked_for {
            self.caused(Cause::Effect(Source::Minion(m)), |e| {
                e.state.burn_blood(m, n)
            });
            self.unlock_minion(m);
        }
        if let Some((m, w, n)) = weapon_due {
            self.weapon_strike_plus(m, w, n, true);
        }
        if let (
            Some(seat),
            Choice::Play {
                card,
                mode,
                actor,
                target,
            },
        ) = (acting, choice)
        {
            self.announce_play(seat, *card, *mode, *actor, *target, Flow::Combat);
        }
        if let (
            Some(seat),
            Choice::UseAbility {
                source,
                ability,
                actor,
                target,
            },
        ) = (acting, choice)
        {
            self.use_ability(seat, *source, *ability, *actor, *target, Flow::Combat);
        }
    }

    /// The press step opens: each Trap and Conquer the Beast burns a counter
    /// for its press — Trap's to continue, no combatant's, first (ANK
    /// 20200525-1); Conquer's its vampire's, mandatory at [ani].
    fn trap_presses(&mut self) {
        let both = self
            .combat
            .as_ref()
            .expect("combat")
            .both_ready(&self.state);
        let c = self.combat.as_mut().expect("combat");
        if std::mem::take(&mut c.round_press) && both && !c.press_continue {
            c.press_continue = true;
        }
        let traps = self.combat.as_ref().expect("combat").traps.clone();
        for (id, owner) in traps {
            let card = &mut self.state.cards[id.index()];
            if !card.present() || card.counters == 0 {
                continue;
            }
            card.counters -= 1;
            let c = self.combat.as_mut().expect("combat");
            match owner {
                None if both && !c.press_continue => c.press_continue = true,
                None => {}
                Some((i, false)) => c.sides[i].mandatory_press += 1,
                Some((i, true)) => c.sides[i].continue_presses += 1,
            }
        }
    }

    /// A vampire's or a card's ability used on the combat with the others'
    /// prevention (`Actor::HostOutOfCombat`, `Actor::HostAnyCombat`).
    fn out_of_combat_ability(&self, o: &Choice) -> bool {
        use crate::cards::Actor;
        matches!(*o, Choice::UseAbility { source: s, ability, .. }
        if matches!(
            s.mode(&self.state, ability).actor,
            Actor::HostOutOfCombat | Actor::HostAnyCombat
        ))
    }

    /// Vampiric Disease: at close range, a vampire combatant with a disease
    /// counter gives one to the other vampire as strikes are about to be
    /// chosen — whatever strike follows (ANK 20210422; LSJ 20040830).
    fn disease_spreads(&mut self) {
        let c = self.combat.as_ref().expect("combat context");
        if c.range != Range::Close {
            return;
        }
        let pair = [c.sides[0].minion, c.sides[1].minion];
        let st = &mut self.state;
        if pair.iter().all(|&m| st.minions[m.index()].is_vampire())
            && pair.iter().any(|&m| st.minions[m.index()].disease)
        {
            for m in pair {
                st.minions[m.index()].disease = true;
            }
        }
    }

    /// The current step is done (window closed, strike chosen, prevention
    /// passed): move to the next one.
    pub(crate) fn combat_advance(&mut self) {
        self.flow = Flow::Combat;
        let step = self.combat.as_ref().expect("combat context").step;
        // Nar-Sheptha gone since: the usual order before the next step
        // (owner, 2026-10-29: "swap back at the burn").
        if matches!(
            step,
            Step::BeforeRange
                | Step::DetermineRange
                | Step::BeforeStrikes
                | Step::AdditionalStrikes
                | Step::Press
                | Step::EndOfRound
        ) {
            self.reorder_combat();
        }
        let c = self.combat.as_mut().expect("combat context");
        match step {
            Step::BeforeRange => {
                if self.stage_pre_range_damage() {
                    return;
                }
                if self.weather_damage() {
                    return;
                }
                let c = self.combat.as_mut().expect("combat context");
                if c.round_over {
                    c.fresh_window(Step::EndOfRound);
                } else if c.range_set {
                    self.disease_spreads();
                    let c = self.combat.as_mut().expect("combat context");
                    c.fresh_window(Step::BeforeStrikes);
                } else {
                    c.fresh_window(Step::DetermineRange);
                }
            }
            Step::DetermineRange => {
                self.disease_spreads();
                let c = self.combat.as_mut().expect("combat context");
                c.fresh_window(Step::BeforeStrikes);
            }
            Step::BeforeStrikes => c.begin_pair(&self.state),
            Step::ChooseStrike(0) => c.fresh_window(Step::ChooseStrike(1)),
            Step::ChooseStrike(_) => {
                if self.pay_strike_tolls() {
                    return;
                }
                let c = self.combat.as_mut().expect("combat context");
                c.fresh_window(Step::BeforeResolution);
            }
            Step::BeforeResolution => {
                if self.ask_drain_order() {
                    return;
                }
                self.resolve_strikes_noted();
                if let Some(c) = self.combat.as_mut() {
                    c.drain_first = None;
                }
                // Stunt Cycle: the prevention its strike carries, used up
                // in this resolution (RTR 20041202, ANK 20200318).
                let c = self.combat.as_mut().expect("combat context");
                for s in &mut c.sides {
                    let n = std::mem::take(&mut s.strike_shield);
                    if n > 0 {
                        s.prevent_strike(n, true);
                    }
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
            Step::GoingToTorpor(i) => {
                c.sides[i].torpor_offered = true;
                self.after_prevention_or_next();
            }
            Step::AdditionalStrikes => {
                if c.sides
                    .iter()
                    .any(|s| s.additional > 0 && !s.no_additional(&self.state))
                {
                    c.begin_pair(&self.state);
                } else {
                    // Drawing Out the Beast [ANI]: its damage as the press
                    // step begins (environmental, RTR 19970630; none in a
                    // round the combat ended early, LSJ 20010312), as
                    // Expiate's burn.
                    self.expiate_burns();
                    if self.press_step_damage() {
                        return;
                    }
                    let c = self.combat.as_mut().expect("combat context");
                    if c.round_over {
                        c.fresh_window(Step::EndOfRound);
                    } else {
                        c.fresh_window(Step::Press);
                        self.trap_presses();
                        self.press_step_draws();
                    }
                }
            }
            // The end of round follows the press step whether a press
            // continues the combat or not (rules-qa 61.1).
            Step::Press => c.fresh_window(Step::EndOfRound),
            // No new round once a combatant left the ready region (Disarm
            // after Telepathic Tracking): the combat ends.
            Step::EndOfRound => {
                // Chave de Braço: as the round ends, its damage counted.
                let close = c.range == Range::Close;
                let puts: Vec<(Held, MinionId)> = (0..2)
                    .filter_map(|i| {
                        let h = c.sides[i].round_end_put.take()?;
                        let more = c.sides[i].inflicted_round > c.sides[1 - i].inflicted_round;
                        (close && more).then_some((h, c.sides[1 - i].minion))
                    })
                    .collect();
                for (h, on) in puts {
                    if self.may_put_held(Held { minion: on, ..h }) {
                        self.place_held(h, on);
                    }
                }
                let c = self.combat.as_mut().expect("combat context");
                // Ossian: a life for each blood the opposing vampire lost to
                // damage this round.
                let mut gains = Vec::new();
                for i in 0..2 {
                    let foe = c.sides[1 - i].minion;
                    let n = c.sides[1 - i].lost_to_damage;
                    let m = c.sides[i].minion;
                    if n > 0
                        && self.state.minions[foe.index()].is_vampire()
                        && self.state.minions[m.index()].present()
                        && self
                            .state
                            .borne(m)
                            .any(|l| l.modifier == cards::Modifier::LifePerFoeBloodLost)
                    {
                        gains.push((m, n));
                    }
                }
                for (m, n) in gains {
                    let own = Cause::Effect(Source::Minion(m));
                    self.caused(own, |e| e.state.gain_blood(m, n));
                }
                let c = self.combat.as_mut().expect("combat context");
                if c.sides.iter().any(|s| s.persisting) {
                    c.quiet_rounds = if c.card_played_round {
                        0
                    } else {
                        c.quiet_rounds + 1
                    };
                }
                if (c.continued || c.press_continue)
                    && c.both_ready(&self.state)
                    && c.quiet_limit.is_none_or(|k| c.quiet_rounds < k)
                {
                    self.new_round();
                } else {
                    self.end_combat();
                }
            }
        }
    }

    /// The pre-range damage the combatants inflicted (Outside the Hourglass
    /// [TEM]), all of it prevented and mended together as the before-range
    /// effects are done (ANK 20170930): staged for prevention. True when
    /// staged.
    fn stage_pre_range_damage(&mut self) -> bool {
        let c = self.combat.as_mut().expect("combat context");
        // A torpor postponed past a new combat lands as it begins (LSJ
        // 20030214): its window opens with the pre-range damage.
        if c.sides
            .iter()
            .all(|s| s.pre_range == 0 && !s.sent_to_torpor)
        {
            return false;
        }
        let mut backfired = Vec::new();
        for i in 0..2 {
            let n = std::mem::take(&mut c.sides[i].pre_range);
            // The range set long before it (Neutral Guard, Sniper Rifle):
            // the Flamethrower too (LSJ 20100709).
            if n > 0 && c.range_set && c.range == Range::Long {
                backfired.extend(backfire(&self.state, c, i));
            }
            let (to, from) = (c.sides[i].minion, c.sides[1 - i].minion);
            let n = n.saturating_sub(self.state.damage_reduction(to, from));
            if n > 0 {
                c.sides[i].pending.normal += n;
            }
            // A backfire too, its hit ignored or not (LSJ 20040802).
            c.sides[i].preventing |= c.sides[i].pending.total() > 0;
        }
        c.others_prevented = false;
        c.pre_range_damage = true;
        // Weather Control's damage this round joins it (rules-qa 78.32).
        let n = weather_amount(c);
        if n > 0 && c.weather_round != c.round {
            c.weather_round = c.round;
            c.weather_staged = n;
        }
        // Dawn Operation, Domain of Evernight [tem]: aggravated (ANK
        // 20220114).
        self.aggravate_vampires();
        for w in backfired {
            self.burn_card_in_play(w);
        }
        self.after_prevention_or_next();
        true
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
                c.window = OrderedWindow::new(c.ring.clone());
                return;
            }
        }
        let staged =
            std::mem::take(&mut self.combat.as_mut().expect("combat context").weather_staged);
        if staged > 0 {
            self.weather_hits(staged);
        }
        if self.marble_clamp() {
            return;
        }
        // Decapitate [POT]: the damage as it stands sends a vampire to
        // torpor; its opponent's player may replace that (RTR 20010711:
        // sequenced after the preventions).
        for i in 0..2 {
            let c = self.combat.as_ref().expect("combat context");
            // Either player, the acting one first (advanced rules 5.3.1).
            if c.sides[i].torpor_offered
                || !(going_to_torpor(&self.state, c, 1 - i) || going_to_torpor(&self.state, c, i))
            {
                continue;
            }
            let seat = c.sides[i].seat;
            // Opened for what this moment offers only: an any-impulse
            // ability had the prevention window just closed (a recorded
            // game read through one, data/records).
            let w = [Window::CombatGoingToTorpor];
            if self.play_options(seat, &w, None).is_empty()
                && self.ability_options(seat, w[0]).is_empty()
            {
                continue;
            }
            self.combat.as_mut().expect("combat context").step = Step::GoingToTorpor(i);
            return;
        }
        if self.apply_damage() {
            return;
        }
        let c = self.combat.as_mut().expect("combat context");
        // Potio Martyrium's damage: its prevention, then applied; Byzar's
        // torpor after Amaranth: its going-to-torpor step.
        if std::mem::take(&mut c.martyrdom) || std::mem::take(&mut c.back_to_torpor) {
            self.after_prevention_or_next();
            return;
        }
        // Damage dealt at the end of the round (Pulled Fangs): back to it.
        if std::mem::take(&mut c.end_of_round_damage) {
            c.fresh_window(Step::EndOfRound);
            return;
        }
        // Pre-range damage mended: the before-range step closes on.
        if std::mem::take(&mut c.pre_range_damage) {
            c.step = Step::BeforeRange;
            self.combat_advance();
            return;
        }
        // Anesthetic Touch: its strike resolved, the combat ends; a strike
        // its first strike held back never resolves.
        let held = c.held_strike.as_ref().map(|&(j, _)| j);
        if (0..2).any(|i| c.sides[i].ends_combat_after && held != Some(i)) {
            c.round_over = true;
            c.held_strike = None;
            let enders = (0..2).filter(|&i| c.sides[i].ends_combat_after && held != Some(i));
            let enders: Vec<MinionId> = enders.map(|i| c.sides[i].minion).collect();
            if let Some(a) = self.action.as_mut() {
                a.ended_combat_early.extend(enders);
            }
        }
        // The strike a first strike held back: resolved now, if its minion
        // is still ready and the round goes on.
        if let Some((j, held)) = c.held_strike.take() {
            // Pounce with first strike: none once barred (LSJ 20011214).
            if !c.round_over
                && self.state.minions[c.sides[j].minion.index()].ready()
                && c.sides[j].no_strike_round != Some(c.round)
            {
                let s = &mut c.sides[j];
                s.strike = held.strike;
                s.unpreventable_strike = held.unpreventable_strike;
                s.bars_strikes_on_hit = held.bars_strikes_on_hit;
                s.dodged_rider = held.dodged_rider;
                s.strike_source = held.strike_source;
                s.strike_at = held.strike_at;
                s.strike_weapon = held.strike_weapon;
                s.fortitude_proof = held.fortitude_proof;
                s.nullifies_weapons = held.nullifies_weapons;
                s.undodgeable_strike = held.undodgeable_strike;
                s.aggravated_strike = held.aggravated_strike;
                s.close_self_damage = held.close_self_damage;
                c.others_prevented = false;
                c.held_pass = true;
                self.resolve_strikes_noted();
                self.combat.as_mut().expect("combat context").held_pass = false;
                self.after_prevention_or_next();
                return;
            }
        }
        // The held strike never resolved: its pair's note as it stands.
        if let Some(n) = self.held_note.take() {
            self.note(n);
        }
        // "Gets an additional strike (limited) each round" (Renegade Garou):
        // after the normal pair, mandatory (PIB 20121028), the limited slot
        // taken.
        let each_round: Vec<usize> = {
            let c = self.combat.as_ref().expect("combat context");
            (0..2)
                .filter(|&i| {
                    c.pair == 0
                        && !c.round_over
                        && !c.sides[i].limited_used
                        && (c.sides[i].strikes_each_round > 0
                            || self
                                .state
                                .borne(c.sides[i].minion)
                                .any(|l| l.modifier == cards::Modifier::AdditionalStrikeEachRound))
                })
                .collect()
        };
        let c = self.combat.as_mut().expect("combat context");
        for i in each_round {
            c.sides[i].additional += c.sides[i].strikes_each_round.max(1);
            c.sides[i].limited_used = true;
        }
        // Fractura [str]: its X hand strikes again, the round's limited
        // slot taken (rules-qa 68.4).
        for s in &mut c.sides {
            let x = s.hand_strikes_each_round;
            if c.pair == 0 && !c.round_over && !s.limited_used && x > 0 {
                s.additional += x;
                s.hand_only_strikes += x;
                s.limited_used = true;
            }
        }
        self.reorder_combat();
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

    /// Flesh of Marble: past a round's first damage, the rest is prevented
    /// (aggravated too at [PRO]), after other prevention; the superior's one
    /// point let through is its controller's pick (LSJ 20021001): true
    /// while that is asked.
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
                // Psychic Assault's damage stays normal.
                let keep = s.keep_normal.min(d.normal);
                d.aggravated += d.normal - keep;
                d.normal = keep;
                d.gun = 0;
            }
        }
    }

    /// Superior Mettle [FOR]: in a later round's prevention step, the
    /// opposing initial strike's damage pending, its vampire is offered the
    /// burn (`Interrupt::MayPreventForBlood`), once a round.
    pub(crate) fn settle_burn_to_prevent(&mut self) {
        let Some(c) = self.combat.as_mut() else {
            return;
        };
        let Step::Prevent(i) = c.step else {
            return;
        };
        let s = &c.sides[i];
        // Stunt Cycle: "if the opposing minion has Celerity [cel], he or she
        // can burn 1 blood to prevent all damage from this strike".
        if let Some((n, card)) = s.celerity_burn_prevent {
            let m = s.minion;
            if s.preventing
                && s.strike_damage.total().min(s.pending.total()) > 0
                && self.state.has_discipline(m, "cel")
                && !self.state.minions[m.index()].is_ally()
                && self.state.minions[m.index()].blood >= n
            {
                let seat = s.seat;
                c.sides[i].celerity_burn_prevent = None;
                self.caused(Cause::Cost(card), |e| {
                    e.interrupts
                        .push(crate::draws::Interrupt::MayPreventForBlood { seat, minion: m, n })
                });
                return;
            }
        }
        let s = &c.sides[i];
        let Some((from, n, card)) = s.burn_to_prevent else {
            return;
        };
        let m = s.minion;
        if c.round <= from
            || c.pair != 0
            || !s.preventing
            || s.burn_to_prevent_asked == Some(c.round)
            || s.strike_damage.total().min(s.pending.total()) == 0
            || self.state.minions[m.index()].blood < n
        {
            return;
        }
        let seat = s.seat;
        c.sides[i].burn_to_prevent_asked = Some(c.round);
        self.caused(Cause::Cost(card), |e| {
            e.interrupts
                .push(crate::draws::Interrupt::MayPreventForBlood { seat, minion: m, n })
        });
    }

    /// Anathema: a combatant carrying one whose blood went from some to 0
    /// in this combat is burned, the opposing Methuselah gaining its
    /// capacity in pool (RTR 19980623; LSJ 20021120, 20021117).
    pub(crate) fn settle_anathema(&mut self) {
        let Some(c) = self.combat.as_ref() else {
            return;
        };
        let st = &self.state;
        let mut fired = None;
        for i in 0..2 {
            let m = c.sides[i].minion;
            let v = &st.minions[m.index()];
            if fired.is_none()
                && c.anathema_seen[i] > 0
                && v.blood == 0
                && v.present()
                && v.is_vampire()
            {
                fired = st
                    .hosted(m)
                    .find(|l| l.modifier == cards::Modifier::Anathema)
                    .map(|l| (i, l.source));
            }
        }
        let seen = std::array::from_fn(|i| st.minions[c.sides[i].minion.index()].blood);
        let c = self.combat.as_mut().expect("combat context");
        c.anathema_seen = seen;
        let Some((i, anathema)) = fired else {
            return;
        };
        let (m, opp) = (c.sides[i].minion, c.sides[1 - i].minion);
        let gainer = self.state.minions[opp.index()].owner;
        let cap = self.state.capacity_of(m).unwrap_or(0);
        self.burn_minion(m);
        if !self.state.seats[gainer.index()].ousted {
            let gain = Cause::Effect(Source::Card(anathema));
            self.caused(gain, |e| e.state.gain_pool(gainer, cap as i32));
        }
        if let Some(c) = self.combat.as_mut() {
            if !c.both_ready(&self.state) {
                c.round_over = true;
            }
        }
    }

    /// Fast Hands: the weapon moves to the striker, if it can hold it (a
    /// steal, not an equip: no requirement or cost).
    fn steal_weapon(&mut self, w: CardId, to: MinionId) {
        let card = self.state.cards[w.index()].card;
        if self.state.minions[to.index()].present() && self.state.can_hold(to, card, &[]) {
            self.state.set_host(w, to);
        }
    }

    /// Expiate: the second of two rounds in a row at close range, the
    /// imbued still ready with 5 convictions burns them, itself and the
    /// opposing monster.
    fn expiate_burns(&mut self) {
        let c = self.combat.as_ref().expect("combat context");
        if !(c.prev_close && c.range == Range::Close) {
            return;
        }
        for i in 0..2 {
            let c = self.combat.as_ref().expect("combat context");
            let (m, foe) = (c.sides[i].minion, c.sides[1 - i].minion);
            // No opposing monster (a mortal): the "otherwise" conviction.
            if c.sides[i].expiate == 0
                || c.sides[i].expiated
                || !self.state.minions[m.index()].ready()
                || !self.state.is_monster(foe)
                || self.state.convictions(m).len() < 5
            {
                continue;
            }
            self.combat.as_mut().expect("combat context").sides[i].expiated = true;
            let held = self.state.convictions(m);
            for card in held.into_iter().take(5) {
                self.burn_card_in_play(card);
            }
            self.burn_minion(m);
            if self.state.minions[foe.index()].ready() {
                self.burn_minion(foe);
            }
        }
    }

    /// Improvised Tactics [aus]: a card drawn as the press step begins,
    /// then down to hand size (`Interrupt::PursuitDiscard`).
    fn press_step_draws(&mut self) {
        let c = self.combat.as_ref().expect("combat context");
        let sides: Vec<(MinionId, SeatId)> = c.sides.iter().map(|s| (s.minion, s.seat)).collect();
        for (m, seat) in sides {
            let n = self
                .state
                .borne(m)
                .filter(|l| l.modifier == cards::Modifier::DrawEachPressStep)
                .count();
            for _ in 0..n {
                let st = &mut self.state.seats[seat.index()];
                if let Some((card, copy)) = st.library.take_top() {
                    st.hand.put(card, copy);
                }
                let excess = self.state.seats[seat.index()]
                    .hand
                    .len()
                    .saturating_sub(self.state.hand_size(seat));
                if excess > 0 {
                    self.interrupts
                        .push(crate::draws::Interrupt::PursuitDiscard {
                            seat,
                            left: excess as u8,
                        });
                }
            }
        }
    }

    /// Drawing Out the Beast [ANI]: 1 unpreventable environmental damage
    /// on each vampire it was played against, as the press step begins.
    /// True when the flow moved on (an oust).
    fn press_step_damage(&mut self) -> bool {
        let c = self.combat.as_mut().expect("combat context");
        if c.sides
            .iter()
            .all(|s| s.press_damage == 0 && s.press_damage_round == 0)
        {
            return false;
        }
        for s in &mut c.sides {
            s.unpreventable += s.press_damage + s.press_damage_round;
        }
        self.apply_damage()
    }

    /// Weather Control's damage, as the before-range step ends (ANK
    /// 20170930): unpreventable, environmental, on each combatant and its
    /// retainers, even if the combat is ending (LSJ 19971110, ANK
    /// 20190725). True when the flow moved on.
    pub(crate) fn weather_damage(&mut self) -> bool {
        let c = self.combat.as_mut().expect("combat context");
        let n = weather_amount(c);
        if n == 0 || c.weather_round == c.round {
            return false;
        }
        c.weather_round = c.round;
        self.weather_hits(n);
        self.apply_damage()
    }

    /// Weather Control's `n` on each combatant and its retainers.
    fn weather_hits(&mut self, n: u8) {
        let c = self.combat.as_mut().expect("combat context");
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
    }

    /// Darkness Within beside a steal strike that can land: the acting
    /// Methuselah orders the drain against the steals (LSJ 20020301,
    /// `Interrupt::DrainBeforeSteals`); true while that order is asked.
    fn ask_drain_order(&mut self) -> bool {
        let c = self.combat.as_ref().expect("combat context");
        if c.drain_first.is_some() || c.pair != 0 || c.held_pass {
            return false;
        }
        let steals = (0..2).any(|i| match c.sides[i].strike {
            Some(StrikeKind::StealBlood { ranged, .. }) => {
                (ranged || c.range == Range::Close)
                    && c.sides[i].strike_at.is_none()
                    && c.sides[1 - i].strike != Some(StrikeKind::Dodge)
            }
            _ => false,
        });
        let Some(&(card, _)) = c.live_drains(&self.state).first() else {
            return false;
        };
        if !steals {
            return false;
        }
        let seat = c.sides[0].seat;
        self.interrupts
            .push(crate::draws::Interrupt::DrainBeforeSteals { seat, card });
        true
    }

    /// The pair of strikes resolved, noted for a narrator when notes are on
    /// (`CombatNote::Strikes`).
    fn resolve_strikes_noted(&mut self) {
        let before = self.notes.as_ref().map(|_| {
            let c = self.combat.as_ref().expect("combat context");
            let pending = c.sides.each_ref().map(|s| s.pending.total());
            let unpreventable = c
                .sides
                .each_ref()
                .map(|s| s.unpreventable + s.unpreventable_aggravated);
            // Told before the strikes resolve: resolving clears their
            // sources and riders.
            let told: [Option<StrikeHit>; 2] = std::array::from_fn(|i| {
                let s = &c.sides[i];
                s.strike
                    .filter(|_| s.strike_at.is_none())
                    .and_then(|k| c.strike_told(&self.state, self.action.as_ref(), i, k))
            });
            (
                c.sides.each_ref().map(|s| s.strike),
                told,
                pending,
                unpreventable,
            )
        });
        // The pair's note goes before the damage it tells.
        self.checkpoint();
        let at = self.note_at();
        let strikes = Cause::Effect(Source::Rule(Rule::Strikes));
        self.caused(strikes, Self::resolve_strikes);
        self.apply_landed();
        // Still in combat (an oust may have ended it).
        let (Some((strikes, told, pending, unpreventable)), Some(c)) =
            (before, self.combat.as_ref())
        else {
            return;
        };
        // Side i inflicts what side 1 - i now has pending, but its own
        // damage as it strikes, and its strike's unpreventable damage (kept
        // out of `pending`).
        let inflicted_unpreventable = [1, 0].map(|j: usize| {
            (c.sides[j].unpreventable + c.sides[j].unpreventable_aggravated)
                .saturating_sub(unpreventable[j])
        });
        let damage = [1, 0].map(|j: usize| {
            c.sides[j]
                .pending
                .total()
                .saturating_sub(pending[j] + c.sides[j].self_damage)
                + inflicted_unpreventable[1 - j]
        });
        let strike_damage = [1, 0]
            .map(|j: usize| c.sides[j].strike_damage.total() + inflicted_unpreventable[1 - j]);
        let n = CombatNote::Strikes {
            minions: c.sides.each_ref().map(|s| s.minion),
            round: c.round,
            pair: c.pair,
            range: c.range,
            strikes,
            told,
            damage,
            strike_damage,
        };
        let holding = c.held_strike.is_some() && !c.held_pass;
        match (self.held_note.take(), n) {
            // A strike held back by a first strike: the pair's note waits.
            (None, n) if holding => {
                self.held_note = Some(n);
                self.held_at = at;
            }
            // The held strike resolved: each side's strike as it resolved,
            // its damage over both passes (the first pass's retainers and
            // environment, the held pass's strike).
            (
                Some(CombatNote::Strikes {
                    strikes: s0,
                    told: t0,
                    damage: d0,
                    strike_damage: sd0,
                    ..
                }),
                CombatNote::Strikes {
                    minions,
                    round,
                    pair,
                    range,
                    strikes,
                    told,
                    damage,
                    strike_damage,
                },
            ) => {
                let now: [bool; 2] = std::array::from_fn(|i| strikes[i].is_some());
                let pick = |i: usize| if now[i] { 1 } else { 0 };
                // Before the first pass's damage too.
                let drained = self.notes_drained;
                let at = self.held_at.take().filter(|&(_, d)| d == drained).or(at);
                self.insert_note(
                    at,
                    CombatNote::Strikes {
                        minions,
                        round,
                        pair,
                        range,
                        strikes: std::array::from_fn(|i| [s0[i], strikes[i]][pick(i)]),
                        told: std::array::from_fn(|i| [t0[i], told[i]][pick(i)]),
                        damage: std::array::from_fn(|i| d0[i] + damage[i]),
                        strike_damage: std::array::from_fn(|i| sd0[i] + strike_damage[i]),
                    },
                );
            }
            (_, n) => self.insert_note(at, n),
        }
    }

    /// Strike resolution (rulebook "Resolve Strike"; spec §4.3.3, §4.7):
    /// a combat ends strike resolves first and alone; otherwise strikes
    /// resolve simultaneously, a dodge voiding the opposing strike, stolen
    /// blood moving before damage is mended.
    fn resolve_strikes(&mut self) {
        let c = self.combat.as_mut().expect("combat context");
        let strikes = [c.sides[0].strike, c.sides[1].strike];
        for (side, strike) in c.sides.iter_mut().zip(strikes) {
            side.hand_struck = matches!(strike, Some(StrikeKind::Hand { .. }));
            side.melee_struck = matches!(strike, Some(StrikeKind::Melee { .. }));
            side.hand_landed = false;
        }
        // Drink the Blood of Ahriman: inflicted, even if prevented.
        let scorched: Vec<CardId> = (0..2)
            .filter(|&i| {
                matches!(
                    strikes[i],
                    Some(StrikeKind::Damage {
                        ranged: true,
                        aggravated: true,
                        ..
                    })
                ) && !matches!(strikes[1 - i], Some(StrikeKind::Dodge))
            })
            .flat_map(|i| {
                self.state
                    .borne(c.sides[1 - i].minion)
                    .filter(|l| l.modifier == cards::Modifier::BurnedByRangedAggravated)
                    .map(|l| l.source)
            })
            .collect();
        for id in scorched {
            self.burn_card_in_play(id);
        }
        let c = self.combat.as_mut().expect("combat context");
        // Necrosis [THN]: at close range, not dodged, the opposing vampire
        // only; lost if the combat ends first (no press step).
        for i in 0..2 {
            if let Some(n) = c.sides[i].press_damage_rider.take() {
                if c.range == Range::Close
                    && !matches!(strikes[1 - i], Some(StrikeKind::Dodge))
                    && self.state.minions[c.sides[1 - i].minion.index()].is_vampire()
                {
                    c.sides[1 - i].press_damage_round += n;
                }
            }
        }
        // A combat ends strike against a minion whose retainer bars it has
        // no effect (Dog Pack put on it after the strike was chosen, ANK
        // 20190412).
        let immune: [bool; 2] = std::array::from_fn(|i| {
            c.sides[i]
                .strike_source
                .is_some_and(|w| c.immune_to(1 - i, self.state.cards[w.index()].card))
        });
        let barred: [bool; 2] = std::array::from_fn(|i| {
            self.state
                .combat_ends_barred(c.sides[i].minion, c.sides[1 - i].minion)
                || c.sides[i].no_combat_ends_round == Some(c.round)
                || c.sides[i].no_combat_ends_combat
                || immune[i]
        });
        // A Flash Grenade with no effect on the opposing vampire still
        // stuns its bearer at close range and burns (owner, 2026-11-10).
        for i in (0..2).filter(|&i| immune[i]) {
            let Some(w) = c.sides[i].strike_source else {
                continue;
            };
            if !matches!(strikes[i], Some(StrikeKind::CombatEnds { .. })) {
                continue;
            }
            let m = c.sides[i].minion;
            if source_rider(&self.state, Some(w), Effect::StunsAfterCombat)
                && c.range == Range::Close
                && !c.immune_to(i, self.state.cards[w.index()].card)
                && self.state.minions[m.index()].is_vampire()
            {
                let v = &mut self.state.minions[m.index()];
                v.locked = true;
                v.skip_unlock = true;
            }
            if source_rider(&self.state, Some(w), Effect::BurnAfterStrike) {
                c.burn_at_end.push(w);
            }
        }
        let ends: Vec<usize> = (0..2)
            .filter(|&i| !barred[i] && matches!(strikes[i], Some(StrikeKind::CombatEnds { .. })))
            .collect();
        if !ends.is_empty() {
            let mut unlocks = Vec::new();
            for i in ends {
                let Some(StrikeKind::CombatEnds { unlock }) = strikes[i] else {
                    unreachable!()
                };
                let m = c.sides[i].minion;
                if let Some(a) = self.action.as_mut() {
                    a.ended_combat_early.push(m);
                }
                // Flash Grenade: after combat, the opposing vampire not
                // dodging, and its vampire bearer at close range, lock and
                // skip their next unlock (LSJ 20040601: again if unlocked
                // meanwhile; none if the combat goes on).
                if source_rider(
                    &self.state,
                    c.sides[i].strike_source,
                    Effect::StunsAfterCombat,
                ) {
                    let o = c.sides[1 - i].minion;
                    if !matches!(strikes[1 - i], Some(StrikeKind::Dodge))
                        && self.state.minions[o.index()].is_vampire()
                    {
                        c.after_combat_stun.push(o);
                    }
                    let own =
                        self.state.cards[c.sides[i].strike_source.expect("a weapon").index()].card;
                    if c.range == Range::Close
                        && self.state.minions[m.index()].is_vampire()
                        && !c.immune_to(i, own)
                    {
                        c.after_combat_stun.push(m);
                    }
                }
                // "Burn after use" with a combat ends strike (Flash
                // Grenade), against another combat ends too (LSJ
                // 20001127-2).
                if let Some(w) = c.sides[i]
                    .strike_source
                    .filter(|&w| source_rider(&self.state, Some(w), Effect::BurnAfterStrike))
                {
                    c.burn_at_end.push(w);
                }
                if unlock && !self.action.as_ref().is_some_and(|a| a.no_unlock) {
                    // "Unlock this vampire before combat ends": resolves with
                    // the strike, so it survives a continued combat (spec
                    // §4.9.2).
                    unlocks.push(m);
                }
                // Catatonic Fear [PRE]: at close range, not dodged (LSJ
                // 19980526).
                if let Some(n) = c.sides[i].after_damage_rider.take() {
                    if (c.range == Range::Close || c.sides[i].after_damage_any)
                        && !matches!(strikes[1 - i], Some(StrikeKind::Dodge))
                    {
                        // A strike's damage after combat: reduced too (LSJ
                        // 20080630), and raised by Dam the Heart's River
                        // (ANK 20170111).
                        let target = c.sides[1 - i].minion;
                        let from = c.sides[i].minion;
                        let n = (n + c.sides[1 - i].more_damage_taken)
                            .saturating_sub(self.state.damage_reduction(target, from));
                        c.after_combat_damage.push((target, n, from));
                    }
                }
                // Kiss of Cathari [OBF]/[PRE] likewise (LSJ 20070124).
                if std::mem::take(&mut c.sides[i].after_stun_rider)
                    && c.range == Range::Close
                    && !matches!(strikes[1 - i], Some(StrikeKind::Dodge))
                {
                    c.after_combat_stun.push(c.sides[1 - i].minion);
                }
                if let Some((h, blood, lock)) = c.sides[i].after_put_rider.take() {
                    if c.range == Range::Close && !matches!(strikes[1 - i], Some(StrikeKind::Dodge))
                    {
                        c.after_combat_put
                            .push((h, c.sides[1 - i].minion, blood, lock));
                    }
                }
                // Oubliette [OBT]: even at long range, not dodged (LSJ
                // 19980526).
                if let Some(n) = c.sides[i].after_burn_rider.take() {
                    if !matches!(strikes[1 - i], Some(StrikeKind::Dodge)) {
                        let from = c.sides[i].minion;
                        c.after_combat_burn.push((c.sides[1 - i].minion, n, from));
                    }
                }
                // Blissful Agony [VAL]: undodged (LSJ 20051020); a second
                // one lost (PIB 20120225).
                if let Some(t) = c.sides[i].redirect_rider.take() {
                    if !matches!(strikes[1 - i], Some(StrikeKind::Dodge))
                        && (c.redirect.is_none() || c.redirect_first == Some(i))
                    {
                        c.redirect = Some((t, i));
                    }
                }
                if std::mem::take(&mut c.sides[i].continue_free)
                    && self
                        .action
                        .as_ref()
                        .is_some_and(|a| a.acting == c.sides[i].minion)
                {
                    self.action.as_mut().expect("action context").continue_free = true;
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
            for m in unlocks {
                self.unlock_minion(m);
            }
            // Combat ended before the other strike resolved: a card to be
            // put in play as it lands stays in the ash heap.
            self.discard_held(false);
            return;
        }
        // First strike (rulebook "First Strike"), a dodge still protecting;
        // the pool's only one (Donnybrook [obf]) burns equipment, so a
        // weapon burned loses its strike, below.
        let mut strikes = strikes;
        let first: Vec<usize> = (0..2)
            .filter(|&i| c.sides[i].first_strike && strikes[i].is_some())
            .collect();
        let equipment_first = |k: Option<StrikeKind>| {
            matches!(
                k,
                Some(
                    StrikeKind::BurnEquipment
                        | StrikeKind::RangedBurnEquipment
                        | StrikeKind::StealWeapon
                )
            )
        };
        if let [i] = first[..]
            .iter()
            .copied()
            .filter(|&i| equipment_first(strikes[i]))
            .collect::<Vec<_>>()[..]
        {
            let ranged = strikes[i] == Some(StrikeKind::RangedBurnEquipment);
            let steal = strikes[i] == Some(StrikeKind::StealWeapon);
            let j = 1 - i;
            let dodged = strikes[j] == Some(StrikeKind::Dodge)
                && !self.state.undodgeable(c.sides[i].minion)
                && !c.sides[i].undodgeable_strike
                && !c.sides[i].undodgeable_round;
            let burned = c.sides[i].strike_weapon.filter(|w| {
                let wc = &self.state.cards[w.index()];
                !dodged
                    && (ranged || c.range == Range::Close)
                    && wc.present()
                    && wc.host == Some(c.sides[j].minion)
            });
            strikes[i] = None;
            if let Some(w) = burned {
                let striker = self.combat.as_ref().expect("combat context").sides[i].minion;
                if steal {
                    self.steal_weapon(w, striker);
                } else {
                    self.burn_card_in_play(w);
                    // Aye: the striker burned an equipment (rules-qa 109.6).
                    self.burn_one_burning(striker);
                }
            }
        }
        let c = self.combat.as_mut().expect("combat context");
        // Darkness Within: (its card, the minion drained) this round, at
        // either range, dodge or not (LSJ 20051123); before the steals if
        // the acting Methuselah ordered it so (LSJ 20020301).
        let mut drained: Vec<(CardId, MinionId)> = if c.pair == 0 && !c.held_pass {
            c.live_drains(&self.state)
        } else {
            Vec::new()
        };
        if c.drain_first == Some(true) {
            for (card, foe) in std::mem::take(&mut drained) {
                drain_one(&mut self.state, card, foe);
            }
        }
        // A strike from a card no longer on the striker has no effect (spec
        // §4.10.4); a card on nobody strikes for whoever uses it (Watchtower:
        // The Wolves Feed).
        let st = &self.state;
        let mut strikes: [Option<StrikeKind>; 2] = std::array::from_fn(|i| {
            let s = &c.sides[i];
            match s.strike_source {
                Some(w)
                    if !(st.cards[w.index()].present()
                        && st.cards[w.index()].bearer().is_none_or(|b| b == s.minion)) =>
                {
                    None
                }
                _ => strikes[i],
            }
        });
        // A vampire striking "with first strike" (Elimelech, Wise Frog's
        // read with the opponent): the opposing non-dodge strike waits for
        // its damage (`held_strike`, rulebook "First Strike").
        let printed_first: [bool; 2] = std::array::from_fn(|i| {
            let ctx = crate::state::PrintedCtx {
                opponent: Some(c.sides[1 - i].minion),
                ..Default::default()
            };
            // Ruth McGinley's, striking with a gun.
            let gun = c.sides[i]
                .strike_source
                .is_some_and(|w| cards::is_gun(st.cards[w.index()].card));
            st.printed_mod(c.sides[i].minion, cards::Stat::FirstStrike, ctx) > 0
                || c.sides[i].first_strike_round
                || gun && st.printed_mod(c.sides[i].minion, cards::Stat::GunFirstStrike, ctx) > 0
        });
        // A card's first strike on another strike (Anesthetic Touch [OBE])
        // holds the opposing strike back alike.
        let first: [bool; 2] = std::array::from_fn(|i| {
            printed_first[i] || (c.sides[i].first_strike && !equipment_first(strikes[i]))
        });
        for i in 0..2 {
            let j = 1 - i;
            if first[i]
                && !first[j]
                && !c.sides[j].first_strike
                && strikes[i].is_some_and(|k| k != StrikeKind::Dodge)
                && strikes[j].is_some_and(|k| k != StrikeKind::Dodge)
            {
                c.held_strike = Some((j, c.sides[j].clone()));
                strikes[j] = None;
            }
        }
        let strength: [u8; 2] =
            std::array::from_fn(|i| c.strength_now(st, self.action.as_ref(), i));
        let close = c.range == Range::Close;
        // Ranged strikes aimed at a retainer: (retainer, amount, steal,
        // striking side), applied once the strikes are read.
        let mut retainer_hits: Vec<(CardId, u8, bool, usize)> = Vec::new();
        // Retainer damage, in the normal strike pair only (not repeated by
        // additional strikes, RTR 19941109), dodge or not (rulebook
        // "Environmental damage").
        if c.pair == 0 && !c.held_pass {
            for i in 0..2 {
                let mut n = st.retainer_damage(c.sides[i].minion);
                // At close range only (Dread Mastiff).
                if c.range == Range::Close {
                    n += st.retainer_damage_close(c.sides[i].minion);
                }
                // Carolina Vález: immune to retainers' damage.
                let target = c.sides[1 - i].minion;
                if st.printed_mod(target, cards::Stat::NoRetainerDamage, Default::default()) > 0 {
                    n = 0;
                }
                // Inflict: each damaging retainer takes its damage too.
                if n > 0 && c.sides[1 - i].reflects {
                    let striker = c.sides[i].minion;
                    for l in st.in_play_layers() {
                        let k = match l.modifier {
                            cards::Modifier::RetainerDamage(k) => k,
                            cards::Modifier::RetainerDamageClose(k) if c.range == Range::Close => k,
                            _ => continue,
                        };
                        if k > 0 && st.cards[l.source.index()].host == Some(striker) {
                            retainer_hits.push((l.source, k, false, i));
                        }
                    }
                }
                // Dam the Heart's River: a retainer's damage too.
                if n > 0 {
                    n += c.sides[1 - i].more_damage_taken;
                }
                c.sides[1 - i].pending.add(n, false);
                // Environmental damage (Carrion Crows): dodge or not.
                let mut n = c.sides[i].environmental;
                if c.range == Range::Close {
                    n += c.sides[i].environmental_close;
                }
                if let Some((k, r)) = c.sides[i].environmental_round {
                    if r == c.round {
                        n += k;
                    }
                }
                n += c.sides[i]
                    .environmental_while
                    .iter()
                    .filter(|&&(card, _)| st.cards[card.index()].present())
                    .map(|&(_, k)| k)
                    .sum::<u8>();
                if let Some((k, r)) = c.sides[i].resolution_damage {
                    if r == c.round {
                        c.sides[1 - i].unpreventable += k + c.sides[1 - i].more_damage_taken;
                    }
                }
                // Blissful Agony [val]: environmental, through a dodge (RTR
                // 19970630, RTR 19941109).
                let k = c.sides[i].resolution_damage_close;
                if k > 0 && c.range == Range::Close {
                    c.sides[1 - i].unpreventable += k + c.sides[1 - i].more_damage_taken;
                }
                // Dam the Heart's River: environmental damage too (ANK
                // 20181128).
                if n > 0 {
                    n += c.sides[1 - i].more_damage_taken;
                }
                c.sides[1 - i].pending.add(n, false);
            }
        }
        for s in &mut c.sides {
            s.self_damage = 0;
        }
        // Breath of the Dragon: at close range, the striker takes its
        // environmental aggravated damage (LSJ 19970801).
        for (i, strike) in strikes.iter().enumerate() {
            let n = std::mem::take(&mut c.sides[i].close_self_damage);
            if strike.is_some() && n > 0 && c.range == Range::Close {
                c.sides[i].pending.add(n, true);
                c.sides[i].self_damage += n;
            }
        }
        // Zip Gun: its bearer takes 1 environmental damage as it strikes
        // with it, once each combat; a gun gone before its strike resolves
        // deals none (LSJ 19970801, LSJ 20100310).
        for (i, strike) in strikes.iter().enumerate() {
            if let Some(w) = c.sides[i].strike_source.filter(|_| strike.is_some()) {
                if st.cards[w.index()].present()
                    && source_rider(st, Some(w), Effect::BearerDamageOncePerCombat)
                    && !c.bearer_damaged.contains(&w)
                {
                    c.sides[i].pending.add(1, false);
                    c.sides[i].self_damage += 1;
                    c.bearer_damaged.push(w);
                }
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
                && !c.sides[i].undodgeable_round
        });
        // Does the strike land — in range, not dodged, not aimed at a
        // retainer?
        let lands: [bool; 2] = std::array::from_fn(|i| {
            c.sides[i].strike_at.is_none()
                && !dodged[i]
                && match strikes[i] {
                    Some(StrikeKind::Hand { .. } | StrikeKind::Melee { .. }) => close,
                    Some(
                        StrikeKind::Damage { ranged, .. }
                        | StrikeKind::StealBlood { ranged, .. }
                        | StrikeKind::CardEffect { ranged },
                    ) => ranged || close,
                    // A strike works at close range unless it says ranged
                    // (owner, 2026-09-26: rules-qa 21.1).
                    Some(
                        StrikeKind::TorporOrBurn | StrikeKind::Torpor { .. } | StrikeKind::Corrupt,
                    ) => close,
                    Some(
                        StrikeKind::BurnWeapon
                        | StrikeKind::RangedBurnEquipment
                        | StrikeKind::RangedStrength { .. }
                        | StrikeKind::BurnLife { .. }
                        | StrikeKind::Area { .. }
                        | StrikeKind::CapacityGap { .. },
                    ) => true,
                    Some(StrikeKind::BurnEquipment | StrikeKind::StealWeapon) => close,
                    Some(StrikeKind::Dodge | StrikeKind::CombatEnds { .. }) | None => false,
                }
        });
        let mut taken_cards: Vec<(CardId, MinionId)> = Vec::new();
        for (i, &landed) in lands.iter().enumerate() {
            let (m, foe) = (c.sides[i].minion, c.sides[1 - i].minion);
            if let Some(n) = c.sides[i].liquefy_rider.take().filter(|_| landed) {
                if st.minions[foe.index()].is_ally() {
                    c.sides[1 - i].sent_to_torpor = true;
                } else if st.younger_by(foe, m, st.minions[m.index()].owner) {
                    c.landed.push(Landed::Liquefy(foe, n));
                }
            }
            // Burned after use, dodged too; a no-op on an ally.
            if let Some(n) = c.sides[i].distill_rider.take() {
                if let Some(w) = c.sides[i].strike_source {
                    let on = landed && st.minions[foe.index()].is_vampire();
                    c.landed.push(Landed::Distill(on.then_some((foe, n)), w));
                }
            }
            if std::mem::take(&mut c.sides[i].diablerie_rider)
                && landed
                && st.younger_by(foe, m, st.minions[m.index()].owner)
            {
                c.diablerie = Some((m, foe));
                c.round_over = true;
            }
            if std::mem::take(&mut c.sides[i].stuns_on_hit) && landed {
                c.landed.push(Landed::Stun(foe));
            }
            // Absorb the Mind [MYT]: still on the opposing minion, its
            // Discipline still not superior on the striker (LSJ
            // 20011205-2).
            if let Some(w) = c.sides[i].steal_card_on_hit.take().filter(|_| landed) {
                let wc = &st.cards[w.index()];
                if wc.present()
                    && wc.host == Some(foe)
                    && st
                        .card_discipline(w)
                        .is_some_and(|d| st.discipline_level(m, d) < 2)
                {
                    taken_cards.push((w, m));
                }
            }
            // Pounce: landing, no more strike this round; dodged, the
            // striker's environmental damage and the opposing press (LSJ
            // 19970801, TOM 19960521).
            if std::mem::take(&mut c.sides[i].bars_strikes_on_hit) && landed {
                c.sides[1 - i].no_strike_round = Some(c.round);
            }
            let n = std::mem::take(&mut c.sides[i].dodged_rider);
            if n > 0 && dodged[i] && strikes[i].is_some() {
                c.sides[i].pending.add(n, false);
                c.sides[i].self_damage += n;
                c.sides[1 - i].presses += 1;
            }
            // Autonomic Mastery [DOM]: "unless it is dodged".
            if dodged[i] && c.sides[i].ends_unless_dodged {
                c.sides[i].ends_combat_after = false;
            }
            if std::mem::take(&mut c.sides[i].burns_ally_on_hit)
                && landed
                && st.minions[c.sides[1 - i].minion.index()].is_ally()
            {
                c.sides[1 - i].sent_to_torpor = true;
            }
        }
        // Smite: its strike resolving, dodged or not, burns the electronic
        // equipment on both combatants, every Smite's flag cleared.
        let smites = [0, 1].map(|i| std::mem::take(&mut c.sides[i].burns_electronic));
        let electronic: Vec<MinionId> = if (0..2).any(|i| smites[i] && strikes[i].is_some()) {
            vec![c.sides[0].minion, c.sides[1].minion]
        } else {
            Vec::new()
        };
        // A card the strike puts on the opposing minion, as it lands (else
        // it stays in the ash heap).
        let mut placed: Vec<(Held, MinionId)> = Vec::new();
        for (i, &landed) in lands.iter().enumerate() {
            if let Some(h) = c.sides[i].held.take() {
                if landed {
                    placed.push((h, c.sides[1 - i].minion));
                }
            }
            // Dodged, the strike never resolved: not placed (advanced
            // rules §4.6.2, §4.7.1).
            if let Some(h) = c.sides[i].held_self.take().filter(|_| !dodged[i]) {
                placed.push((h, c.sides[i].minion));
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
        let mut life_burns: Vec<(MinionId, u8)> = Vec::new();
        // Contagion [DAI]: the striking sides whose corruption lands.
        let mut corrupts: Vec<usize> = Vec::new();
        // Coma [DEM]: the vampires that do not unlock as normal next.
        let mut comatose: Vec<MinionId> = Vec::new();
        let mut burn_weapons: Vec<CardId> = Vec::new();
        // Strikers whose strike burned an equipment (Aye, rules-qa 109.6).
        let mut equipment_burners: Vec<MinionId> = Vec::new();
        // Fast Hands: (weapon, striker) to move as the strikes land.
        let mut steal_weapons: Vec<(CardId, MinionId)> = Vec::new();
        // Dagger: (struck side, weapon) watched for prevention.
        let mut dagger_watch: Vec<(usize, CardId)> = Vec::new();
        // Weapons losing counters per damage inflicted (Weighted Walking
        // Stick): the card and the points.
        let mut counter_hits: Vec<(CardId, u8)> = Vec::new();
        for i in 0..2 {
            let j = 1 - i;
            let Some(k) = strikes[i] else {
                continue;
            };
            // "Burn after use" (Waxen Poetica, Molotov Cocktail): its strike
            // resolved, dodged too (owner: rules-qa 84.2); one combat ended
            // before never got here.
            if let Some(w) = c.sides[i]
                .strike_source
                .filter(|&w| source_rider(st, Some(w), Effect::BurnAfterStrike))
            {
                burn_weapons.push(w);
            }
            // Dagger: its resolved strike, dodged too.
            if let Some(w) = c.sides[i].strike_source {
                if strike_rider(st, w, k, Effect::BurnIfDamagePrevented) {
                    dagger_watch.push((j, w));
                }
                if strike_rider(st, w, k, Effect::OutOfPlayUntilActionEnd) {
                    c.sides[i].out_after = Some(w);
                }
            }
            // "After the bearer strikes with this gun" (AK-47; Caseless
            // Rounds on it): struck, dodged or not; one limited
            // additional-strike source a round.
            if let Some(w) = c.sides[i].strike_source {
                if (source_rider(st, Some(w), Effect::AdditionalStrikeWithSource)
                    || c.ammo.contains(&(w, cards::Ammo::CaselessRounds)))
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
                        StrikeKind::Damage { n, ranged, .. } => {
                            let more =
                                ranged_bonus(st, c, i, ranged, cards::Stat::RangedStrikeDamage);
                            retainer_hits.push((r, (n as i32 + more).max(0) as u8, false, i));
                        }
                        StrikeKind::StealBlood { n, ranged } => {
                            let more =
                                ranged_bonus(st, c, i, ranged, cards::Stat::RangedStrikeSteal);
                            retainer_hits.push((r, (n as i32 + more).max(0) as u8, true, i));
                        }
                        StrikeKind::RangedStrength { .. } if !harmless[i] => {
                            if let Some(h) =
                                c.strike_hit(st, self.action.as_ref(), i, k, strength[i])
                            {
                                retainer_hits.push((r, h.n, false, i));
                            }
                        }
                        _ => {}
                    }
                }
                continue;
            }
            match k {
                StrikeKind::Hand { .. } => {
                    if close && !dodged {
                        let h = c.strike_hit(st, self.action.as_ref(), i, k, strength[i]);
                        let h = h.expect("a damage strike");
                        c.sides[j].strike_hit(h.n, h.aggravated, false);
                        // Inflict: the amount on the striker too, normal
                        // (owner, rules-qa 102.15), prevented or not.
                        if c.sides[j].reflects {
                            c.sides[i].pending.add(h.n, false);
                        }
                    }
                }
                StrikeKind::Melee { .. } => {
                    if close && !dodged && !harmless[i] {
                        let h = c.strike_hit(st, self.action.as_ref(), i, k, strength[i]);
                        let StrikeHit { n, aggravated, .. } = h.expect("a damage strike");
                        c.sides[j].strike_hit(n, aggravated, false);
                        if !aggravated
                            && cleaves(self.action.as_ref(), c.sides[i].strike_source) > 0
                        {
                            c.sides[j].cleaved_hit += n;
                        }
                        // Meat Hook: once each combat, a hit at close range
                        // not dodged.
                        if let Some(w) = c.sides[i].strike_source.filter(|w| {
                            !c.hit_presses_had.contains(w)
                                && st.layers.iter().any(|l| {
                                    l.source == *w
                                        && l.modifier == cards::Modifier::ContinuePressOnHitOnce
                                })
                        }) {
                            c.hit_presses_had.push(w);
                            c.sides[i].continue_presses += 1;
                        }
                        let mend_cost = c.sides[i].strike_source.and_then(|w| {
                            st.layers.iter().find_map(|l| match l.modifier {
                                cards::Modifier::MendingCost(k) if l.source == w => Some(k),
                                _ => None,
                            })
                        });
                        if let Some(k) = mend_cost.filter(|_| !aggravated) {
                            c.sides[j].costly_hit += n;
                            c.sides[j].mend_cost = k;
                        }
                        // Inflict: the amount on the striker too, normal
                        // (owner, rules-qa 102.15), prevented or not.
                        if c.sides[j].reflects {
                            c.sides[i].pending.add(n, false);
                        }
                        // Inflicted, prevented or not (LSJ 20100519).
                        if let Some(w) = c.sides[i].strike_source {
                            if source_rider(st, Some(w), Effect::CountersPerDamage) {
                                counter_hits.push((w, n));
                            }
                        }
                    }
                }
                StrikeKind::Damage { .. } | StrikeKind::CapacityGap { .. } => {
                    if (k.ranged() || close) && !dodged && !harmless[i] {
                        let h = c.strike_hit(st, self.action.as_ref(), i, k, strength[i]);
                        let StrikeHit { n, aggravated, .. } = h.expect("a damage strike");
                        let gun = c.sides[i]
                            .strike_source
                            .is_some_and(|w| cards::is_gun(st.cards[w.index()].card));
                        let black_hand = c.sides[i].unpreventable_if_black_hand
                            && st.is_black_hand(c.sides[i].minion);
                        if black_hand
                            || c.sides[i].unpreventable_strike
                            || source_rider(
                                st,
                                c.sides[i].strike_source,
                                Effect::UnpreventableStrike,
                            )
                        {
                            // Dam the Heart's River: a damaging strike's
                            // too (RTR 19960221).
                            let n = if n > 0 {
                                n + c.sides[j].more_damage_taken
                            } else {
                                0
                            };
                            if aggravated {
                                c.sides[j].unpreventable_aggravated += n;
                            } else {
                                c.sides[j].unpreventable += n;
                            }
                        } else {
                            if c.sides[i].never_aggravated {
                                c.sides[j].keep_normal += n;
                            }
                            c.sides[j].strike_hit(n, aggravated, gun && !aggravated);
                        }
                        let r = c.sides[i].retainer_strike_damage;
                        if r > 0 {
                            for (ri, rc) in st.cards.iter().enumerate() {
                                if rc.present()
                                    && rc.host == Some(c.sides[j].minion)
                                    && LIBRARY[rc.card as usize]
                                        .types
                                        .contains(&crate::library::CardType::Retainer)
                                {
                                    retainer_hits.push((CardId(ri as u32), r, false, i));
                                }
                            }
                        }
                    }
                }
                // Eye of Unforgiving Heaven: the opposing minion unless
                // dodged, its demon retainers regardless (a dodge spares
                // the dodger only; owner, 2026-11-10); the striking vampire
                // and its own demon retainers regardless, its part in its
                // own pending; the striker's bonuses on each (rules-qa
                // 113.87).
                StrikeKind::Area { n, self_normal } => {
                    let n = c.area_damage(st, i, n);
                    let demons = |h: MinionId| {
                        st.cards
                            .iter()
                            .enumerate()
                            .filter(move |(_, rc)| {
                                let lib = &LIBRARY[rc.card as usize];
                                rc.present()
                                    && rc.host == Some(h)
                                    && lib.types.contains(&crate::library::CardType::Retainer)
                                    && lib.labels.contains(&crate::library::Label::Demon)
                            })
                            .map(|(ri, _)| CardId(ri as u32))
                            .collect::<Vec<_>>()
                    };
                    if !dodged {
                        let h = c.strike_hit(st, self.action.as_ref(), i, k, strength[i]);
                        let hit = h.expect("a damage strike");
                        if hit.n > 0 {
                            c.sides[j].strike_hit(hit.n, hit.aggravated, false);
                        }
                    }
                    for r in demons(c.sides[j].minion) {
                        retainer_hits.push((r, n, false, i));
                    }
                    let me = c.sides[i].minion;
                    if st.minions[me.index()].is_vampire() {
                        c.sides[i].pending.add(n, !self_normal);
                        c.sides[i].self_damage += n;
                    }
                    for r in demons(me) {
                        retainer_hits.push((r, n, false, i));
                    }
                }
                // Earthshock: strength damage at any range.
                StrikeKind::RangedStrength { .. } => {
                    if !dodged && !harmless[i] {
                        let h = c.strike_hit(st, self.action.as_ref(), i, k, strength[i]);
                        let StrikeHit { n, aggravated, .. } = h.expect("a damage strike");
                        c.sides[j].strike_hit(n, aggravated, false);
                    }
                }
                // Argent Baton: life burned, not damage.
                StrikeKind::BurnLife { n } => {
                    if !dodged {
                        life_burns.push((c.sides[j].minion, n));
                    }
                }
                StrikeKind::StealBlood { n, ranged } => {
                    if (ranged || close) && !dodged {
                        let ctx = crate::state::PrintedCtx {
                            opponent: Some(c.sides[j].minion),
                            card: c.sides[i].strike_play,
                            ..Default::default()
                        };
                        let more = ranged_bonus(st, c, i, ranged, cards::Stat::RangedStrikeSteal)
                            + st.printed_mod(c.sides[i].minion, cards::Stat::StrikeSteal, ctx);
                        let n = (n as i32 + more).max(0) as u8;
                        steals[i] = n.min(self.state.minions[c.sides[j].minion.index()].blood);
                        // Tunnel Runner: a vampire stealing its life takes
                        // as much environmental damage, with the strikes.
                        let (taker, from) = (c.sides[i].minion, c.sides[j].minion);
                        if st.minions[taker.index()].is_vampire()
                            && st.minions[from.index()].is_ally()
                            && st.bears(from, cards::Modifier::StealersTakeDamage)
                        {
                            c.sides[i].pending.add(steals[i], false);
                        }
                    }
                }
                StrikeKind::TorporOrBurn => {
                    if close && !dodged {
                        c.sides[j].sent_to_torpor = true;
                    }
                }
                StrikeKind::Corrupt => {
                    if close && !dodged {
                        corrupts.push(i);
                    }
                }
                // Coma: a vampire only, wounded as it is sent.
                StrikeKind::Torpor { skip_unlock } => {
                    if close && !dodged && st.minions[c.sides[j].minion.index()].is_vampire() {
                        c.sides[j].sent_to_torpor = true;
                        if skip_unlock {
                            comatose.push(c.sides[j].minion);
                        }
                    }
                }
                StrikeKind::BurnWeapon
                | StrikeKind::BurnEquipment
                | StrikeKind::RangedBurnEquipment => {
                    let reaches = k != StrikeKind::BurnEquipment || close;
                    if !dodged && reaches {
                        if let Some(w) = c.sides[i].strike_weapon {
                            let wc = &st.cards[w.index()];
                            if wc.present() && wc.host == Some(c.sides[j].minion) {
                                burn_weapons.push(w);
                                // Aye counts equipment, not a retainer.
                                if LIBRARY[wc.card as usize]
                                    .types
                                    .contains(&crate::library::CardType::Equipment)
                                {
                                    equipment_burners.push(c.sides[i].minion);
                                }
                            }
                        }
                    }
                }
                StrikeKind::StealWeapon => {
                    if !dodged && close {
                        if let Some(w) = c.sides[i].strike_weapon {
                            let wc = &st.cards[w.index()];
                            if wc.present() && wc.host == Some(c.sides[j].minion) {
                                steal_weapons.push((w, c.sides[i].minion));
                            }
                        }
                    }
                }
                StrikeKind::Dodge
                | StrikeKind::CombatEnds { .. }
                | StrikeKind::CardEffect { .. } => {}
            }
        }
        // Contagion: the counter lands with the strike; the steal, if
        // reached, is the striker's Methuselah's choice after it resolves
        // (`Interrupt::StealForCorruption`, ending the combat).
        let corrupted: Vec<(SeatId, MinionId)> = corrupts
            .into_iter()
            .map(|i| {
                (
                    self.state.minions[c.sides[i].minion.index()].owner,
                    c.sides[1 - i].minion,
                )
            })
            .collect();
        for m in comatose {
            self.state.minions[m.index()].skip_unlock = true;
        }
        for (seat, t) in corrupted {
            self.corrupt_may_steal(seat, t);
        }
        let c = self.combat.as_mut().expect("combat context");
        // Simultaneous steals: amounts from the pre-strike state, capacity
        // applied after both moves (Theft of Vitae rulings).
        for (i, &n) in steals.iter().enumerate() {
            self.state.lose_blood(c.sides[1 - i].minion, n);
        }
        for (i, &n) in steals.iter().enumerate() {
            self.state.gain_blood(c.sides[i].minion, n);
        }
        // Darkness Within not ordered first: after the steals, before
        // damage is mended (LSJ 20051123); an ally's last life taken burns
        // it with the damage.
        for (card, foe) in drained {
            drain_one(&mut self.state, card, foe);
        }
        // Argent Baton: life burned as a steal is, an ally at 0 burned with
        // the damage's casualties.
        for (m, n) in life_burns {
            self.state.burn_blood(m, n);
        }
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
        // Improvised Flamethrower: a bearer struck at long range, the
        // damage prevented or not (LSJ 20040802).
        let mut backfired = Vec::new();
        if c.range == Range::Long {
            for i in 0..2 {
                if c.sides[i].strike_damage.total() > 0 {
                    backfired.extend(backfire(&self.state, c, i));
                }
            }
        }
        for (j, w) in dagger_watch {
            let dealt = c.sides[j].strike_damage.total();
            c.sides[j].prevented_watch = Some((w, dealt));
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
        for w in burn_weapons.into_iter().chain(backfired) {
            self.burn_card_in_play(w);
        }
        for m in equipment_burners {
            if self.state.minions[m.index()].in_play {
                self.burn_one_burning(m);
            }
        }
        for (w, to) in steal_weapons.into_iter().chain(taken_cards) {
            self.steal_weapon(w, to);
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
        let burned: Vec<CardId> = (0..self.state.cards.len())
            .map(|k| CardId(k as u32))
            .filter(|&id| {
                let card = &self.state.cards[id.index()];
                card.present()
                    && card.host.is_some_and(|h| electronic.contains(&h))
                    && LIBRARY[card.card as usize]
                        .labels
                        .contains(&crate::library::Label::Electronic)
            })
            .collect();
        for id in burned {
            self.burn_card_in_play(id);
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
        self.state.set_host(w, on);
        self.state.cards[w.index()].mode = mi as u8;
        self.push_mode_layers(w);
    }

    /// `h` may go on its minion: still there, with no card of its name.
    pub(crate) fn may_put_held(&self, h: Held) -> bool {
        self.state.minions[h.minion.index()].present()
            && !self.state.holds_its_name(h.minion, h.card)
    }

    /// The card goes on minion `on`, if still in play, out of the ash heap
    /// (rules-qa 47.1); gone from there, an effect-only stand-in.
    pub(crate) fn place_held(&mut self, h: Held, on: MinionId) {
        if !self.state.minions[on.index()].in_play {
            return;
        }
        // The card itself, out of the ash heap (by its copy, else a copy
        // of it); none there, its effect alone (rules-qa 21.1).
        let ash = &mut self.state.seats[h.seat.index()].ash;
        let k = (0..ash.len())
            .rev()
            .find(|&k| *h.copy != crate::copies::CopyId::NONE && ash.copy(k) == *h.copy)
            .or_else(|| ash.iter().rposition(|&c| c == h.card));
        let taken = k.map(|k| ash.take(k));
        let copy = taken.map_or(crate::copies::CopyId::NONE, |t| t.1);
        let id = self.enter_play(crate::state::CardInPlay {
            by: Some(h.minion),
            counters: h.counters,
            effect_only: taken.is_none(),
            ..crate::state::CardInPlay::entering(h.card, copy, h.seat, Some(on), h.mode)
        });
        self.push_mode_layers(id);
        // Molotov Cocktail: for this combat, not usable this round.
        let ends = cards::burns_at_combat_end(h.card, h.mode);
        if let Some(c) = self.combat.as_mut() {
            c.placed_this_round.push(id);
            if ends {
                c.burn_at_end.push(id);
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
            s.held_self = None;
        }
        if after_combat {
            c.redirect = None;
            c.redirect_first = None;
            c.after_combat.clear();
            c.after_combat_damage.clear();
            c.after_combat_stun.clear();
            c.after_combat_burn.clear();
            c.after_combat_put.clear();
        }
    }

    /// Dagger, its damage prevented: burned; its ranged strike resolved: out
    /// of play until the action ends, or the combat without one.
    fn dagger_after_prevention(&mut self) {
        let c = self.combat.as_mut().expect("combat context");
        let mut burned = Vec::new();
        let mut out = Vec::new();
        for s in &mut c.sides {
            if let Some((w, dealt)) = s.prevented_watch.take() {
                if s.strike_damage.total() < dealt {
                    burned.push(w);
                }
            }
            out.extend(s.out_after.take());
        }
        for w in burned {
            if self.state.cards[w.index()].present() {
                self.burn_card_in_play(w);
            }
        }
        for w in out {
            if !self.state.cards[w.index()].present() {
                continue;
            }
            self.state.cards[w.index()].out_of_play = true;
            match self.action.as_mut() {
                Some(a) => a.return_at_end.push(w),
                None => self
                    .combat
                    .as_mut()
                    .expect("combat context")
                    .return_at_end
                    .push(w),
            }
        }
    }

    /// Damage applied at once to both sides after prevention (rulebook
    /// "Damage Resolution"). True when the flow moved on: a rider (Fame)
    /// ousted a Methuselah (`combat_ousts`).
    pub(crate) fn apply_damage(&mut self) -> bool {
        let strikes = Cause::Effect(Source::Rule(Rule::Strikes));
        self.caused(strikes, Self::apply_damage_now)
    }

    fn apply_damage_now(&mut self) -> bool {
        self.dagger_after_prevention();
        let mut clots = Vec::new();
        let mut poxed = Vec::new();
        let mut corrupted = Vec::new();
        // Immune to non-aggravated damage: none inflicted (Tye Cooper).
        let c = self.combat.as_mut().expect("combat context");
        for s in &mut c.sides {
            if self.state.immune_non_aggravated(s.minion) {
                s.pending.normal = s.cleaved_hit.min(s.pending.normal);
                s.pending.gun = 0;
                s.unpreventable = 0;
                s.strike_damage.normal = 0;
                s.strike_damage.gun = 0;
            }
        }
        // Retainers burn life for the damage left on them (aggravated alike,
        // rulebook "Damage Resolution"); gone or moved by now, no effect.
        let c = self.combat.as_mut().expect("combat context");
        let hits = std::mem::take(&mut c.retainer_pending);
        let minions = [c.sides[0].minion, c.sides[1].minion];
        for (r, n) in hits {
            let card = &self.state.cards[r.index()];
            if n == 0 || !card.present() || !card.host.is_some_and(|h| minions.contains(&h)) {
                continue;
            }
            self.retainer_loses_life(r, n);
        }
        let c = self.combat.as_mut().expect("combat context");
        // An aimed strike that landed (Target Vitals): +n of its kind — not
        // to be prevented, the prevention step past — and no press this
        // round (rules-qa 55.1).
        for i in 0..2 {
            let aim = std::mem::take(&mut c.sides[1 - i].aim);
            let landed = c.sides[i]
                .pending
                .total()
                .min(c.sides[i].strike_damage.total());
            if aim > 0 && landed > 0 {
                let aggravated = c.sides[i].strike_damage.aggravated > 0;
                c.sides[i].pending.add(aim, aggravated);
                c.sides[i].strike_damage.add(aim, aggravated);
                c.sides[i].no_press_round = true;
            }
        }
        let action_long = self
            .action
            .as_ref()
            .map_or(&[][..], |a| &a.aggravated_as_normal[..]);
        let results: Vec<Resolved> = c
            .sides
            .iter()
            .map(|s| {
                let m = &self.state.minions[s.minion.index()];
                let mut d = s.pending;
                d.normal += s.unpreventable;
                d.aggravated += s.unpreventable_aggravated;
                // Jua Vema: the aggravated damage as normal.
                if s.aggravated_as_normal || action_long.contains(&s.minion) {
                    d.normal += std::mem::take(&mut d.aggravated);
                }
                if m.is_ally() {
                    resolve_life_damage(m.blood, d)
                } else {
                    // Persisting, still wounded (Undead Persistence).
                    let wounded = m.in_torpor || s.persisting;
                    resolve_damage_costly(m.blood, d, wounded, s.costly_hit, s.mend_cost)
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
            let unpreventable = std::mem::take(&mut c.sides[i].unpreventable)
                + std::mem::take(&mut c.sides[i].unpreventable_aggravated);
            let from_strikes = d.min(c.sides[i].strike_damage.total()) + unpreventable;
            let d = d + unpreventable;
            c.sides[i].damaged_round |= d > 0;
            c.sides[i].damage_round = c.sides[i].damage_round.saturating_add(d);
            c.sides[i].combat_damage = c.sides[i].combat_damage.saturating_add(d);
            c.sides[i].strike_damage = Damage::default();
            c.sides[1 - i].inflicted_round += from_strikes;
            let striker = c.sides[1 - i].minion;
            let key = crate::state::once_corrupt_key(striker);
            if from_strikes > 0
                && !c.abilities_used.contains(&key)
                && self.state.printed_mod(
                    striker,
                    cards::Stat::CorruptOnDamageOncePerCombat,
                    crate::state::PrintedCtx::default(),
                ) > 0
            {
                c.abilities_used.push(key);
                corrupted.push((c.sides[1 - i].seat, c.sides[i].minion));
            }
            c.sides[1 - i].hand_landed |= from_strikes > 0 && c.sides[1 - i].hand_struck;
            if from_strikes > 0 && c.sides[1 - i].hand_struck {
                let striker = &mut c.sides[1 - i];
                // Each time, additional strikes included (PIB 20141210).
                if striker.hand_landed_last_round
                    && self.state.printed_mod(
                        striker.minion,
                        cards::Stat::PoxOnConsecutiveHandDamage,
                        crate::state::PrintedCtx::default(),
                    ) > 0
                {
                    poxed.push(c.sides[i].minion);
                }
                c.sides[1 - i].hand_landed_round = true;
            }
            if from_strikes > 0 && (c.sides[1 - i].hand_struck || c.sides[1 - i].melee_struck) {
                clots.push((
                    c.sides[1 - i].seat,
                    c.sides[1 - i].minion,
                    c.sides[i].minion,
                ));
            }
        }
        let mut wounded = Vec::new();
        let mut burned = Vec::new();
        let mut lifeless = Vec::new();
        let mut aye = Vec::new();
        let mut diablerized = Vec::new();
        // "Burned (without going into torpor) if he takes more than n damage
        // during a single combat" (Franciscus).
        let over = c.sides.each_ref().map(|s| {
            let cap = self.state.printed_mod(
                s.minion,
                cards::Stat::BurnedOverCombatDamage,
                crate::state::PrintedCtx::default(),
            );
            cap > 0 && s.combat_damage as i32 > cap
        });
        // Potio Martyrium: its Gargoyle downed burns instead, its damage
        // dealt below.
        let martyrs = c.sides.each_ref().map(|s| {
            self.state.hosted(s.minion).find_map(|l| match l.modifier {
                cards::Modifier::HostMartyrdom(n) => Some(n),
                _ => None,
            })
        });
        let mut martyrdom: Vec<(usize, u8)> = Vec::new();
        for (i, ((s, r), over)) in c.sides.iter_mut().zip(results).zip(over).enumerate() {
            self.state.lose_blood(s.minion, r.blood_burned);
            let m = &self.state.minions[s.minion.index()];
            s.lost_to_damage += r.blood_burned;
            s.pending = Damage::default();
            s.keep_normal = 0;
            s.cleaved_hit = 0;
            // Sent to torpor by the strike (Touch of Oblivion [OBL]): a
            // vampire wounded, an ally burned.
            let sent = std::mem::take(&mut s.sent_to_torpor);
            // Decapitate [POT]: burned instead of going to torpor.
            let instead = std::mem::take(&mut s.burn_instead_of_torpor);
            // Amaranth: diablerized instead, now (rules-qa 113.10).
            let eaten = std::mem::take(&mut s.diablerized_instead_of_torpor);
            s.torpor_offered = false;
            if eaten && (r.wounded || sent) && !m.is_ally() {
                diablerized.push(s.minion);
            } else if let Some(n) =
                martyrs[i].filter(|_| over || r.burned || ((r.wounded || sent) && !s.persisting))
            {
                burned.push(s.minion);
                martyrdom.push((1 - i, n));
            } else if over || (instead && (r.wounded || sent)) {
                burned.push(s.minion);
            } else if m.is_imbued() && (r.burned || sent) {
                lifeless.push(s.minion);
            } else if r.burned || (sent && m.is_ally()) {
                burned.push(s.minion);
            } else if (r.wounded || sent) && !s.persisting {
                wounded.push(s.minion);
            }
        }
        let diablerized: Vec<(MinionId, MinionId)> = diablerized
            .into_iter()
            .filter_map(|v| c.side_of_minion(v).map(|i| (c.sides[1 - i].minion, v)))
            .collect();
        // Who burned whom (Predator's Transformation [ABO]).
        for i in 0..2 {
            let opp = c.sides[1 - i].minion;
            if burned.contains(&opp) {
                let owner = self.state.minions[opp.index()].owner;
                c.burned_minions.push((c.sides[i].minion, owner));
                // Aye: one per minion its strikes burned (rules-qa 109.6).
                aye.push(c.sides[i].minion);
            }
        }
        // Eye of Unforgiving Heaven: its striker burned by this resolution.
        for s in &mut c.sides {
            let n = std::mem::take(&mut s.pool_if_burned);
            if n > 0 && burned.contains(&s.minion) {
                self.state.gain_pool(s.seat, n as i32);
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
        for m in poxed {
            let v = &mut self.state.minions[m.index()];
            v.pox = v.pox.saturating_add(1);
        }
        for (seat, m) in corrupted {
            self.state.minions[m.index()].add_corruption(seat, 1);
        }
        // Burns and torpors land at once, their riders with them (rules-qa
        // 6.4): a Methuselah may be ousted here.
        for (seat, striker, to) in clots {
            if self.state.sect_of(striker) == Some(crate::crypt::Sect::Anarch) {
                let sources: Vec<CardId> = self
                    .state
                    .cards
                    .iter()
                    .enumerate()
                    .filter_map(|(i, card)| {
                        let id = CardId(i as u32);
                        (card.present()
                            && card.counters > 0
                            && self.state.card_controller(id) == seat
                            && self.state.layers.iter().any(|l| {
                                l.source == id && l.modifier == cards::Modifier::MovesClotsOnHit
                            }))
                        .then_some(id)
                    })
                    .collect();
                for card in sources {
                    self.interrupts
                        .push(crate::draws::Interrupt::MoveClot { seat, card, to });
                }
            }
        }
        // Liquefy [VIC]: its strike's diablerie, made with the damage that
        // lands beside it, if both are still there (rules-qa 113.155).
        let liquefy = self.combat.as_mut().and_then(|c| c.diablerie.take());
        let st = &self.state;
        let liquefy = liquefy.filter(|(d, _)| st.minions[d.index()].ready());
        for &m in &burned {
            let by = self.combat.as_ref().and_then(|c| c.opponent_of(m));
            if let Some(by) = by {
                self.red_list_burned(by, m);
            }
            self.burn_minion(m);
        }
        for m in aye {
            if self.state.minions[m.index()].in_play {
                self.burn_one_burning(m);
            }
        }
        self.incapacitate(&lifeless);
        if !wounded.is_empty() {
            self.minions_to_torpor(&wounded);
        }
        let st = &self.state;
        let liquefy = liquefy
            .filter(|(d, v)| st.minions[d.index()].present() && st.minions[v.index()].present());
        // A diablerie's own oust (Sandra White's pool burn) is read by
        // `oust_now`'s game over, `after_deferred_oust`,
        // `settle_acting_ousted` or `resume_hunted_combat`.
        if let Some((d, v)) = liquefy {
            let rule = Cause::Effect(Source::Rule(Rule::Diablerie));
            if self.caused(rule, |e| e.diablerise_to(d, v, true)) {
                self.combat.as_mut().expect("combat context").diablerized = Some((d, v));
            }
        }
        for (d, v) in diablerized {
            let rule = Cause::Effect(Source::Rule(Rule::Diablerie));
            let made = self.caused(rule, |e| e.diablerise_to(d, v, false));
            let c = self.combat.as_mut().expect("combat context");
            if made {
                c.diablerized = Some((d, v));
                c.round_over = true;
            } else if self.state.minions[v.index()].ready() {
                if let Some(i) = c.side_of_minion(v) {
                    c.sides[i].sent_to_torpor = true;
                    c.back_to_torpor = true;
                }
            }
        }
        // Range does not matter (ANK 20181024); the opposing minion and its
        // retainers may prevent it with combat cards (LSJ 20100205).
        let c = self.combat.as_mut().expect("combat context");
        for (j, n) in martyrdom {
            let o = c.sides[j].minion;
            if !self.state.minions[o.index()].present() {
                continue;
            }
            c.sides[j].pending.aggravated += n;
            c.sides[j].preventing = true;
            c.others_prevented = false;
            c.martyrdom = true;
            for (ri, card) in self.state.cards.iter().enumerate() {
                if card.present()
                    && card.host == Some(o)
                    && LIBRARY[card.card as usize]
                        .types
                        .contains(&crate::library::CardType::Retainer)
                {
                    c.retainer_pending.push((CardId(ri as u32), n));
                }
            }
        }
        if !c.both_ready(&self.state) {
            c.round_over = true;
        }
        if !wounded.is_empty() || !burned.is_empty() || !lifeless.is_empty() {
            return self.combat_ousts();
        }
        false
    }

    /// After a combat effect that may have ousted a Methuselah: true when
    /// the game ended, the turn passed, or the combat ended (DESIGN §10).
    pub(crate) fn combat_ousts(&mut self) -> bool {
        if self.state.living() <= 1 {
            // The deciding pair's note, if it waited for a held strike.
            if let Some(n) = self.held_note.take() {
                self.note(n);
            }
            self.combat = None;
            self.state.leave_combat();
            self.place_action_card();
            self.drop_action();
            self.flow = Flow::GameOver;
            return true;
        }
        if self.state.seats[self.state.active.index()].ousted {
            self.next_turn();
            return true;
        }
        // Suspended for a blood hunt: read as it resumes.
        if self
            .action
            .as_ref()
            .is_some_and(|a| a.hunt_in_combat.is_some())
        {
            return false;
        }
        let c = self.combat.as_ref().expect("combat context");
        if c.sides
            .iter()
            .any(|s| self.state.seats[s.seat.index()].ousted)
        {
            // A combatant's Methuselah ousted: the combat ends at once, no
            // end-of-round window; the action goes on without them.
            let st = &self.state;
            let a = self.action.as_mut().expect("action context");
            a.order.retain(|s| !st.seats[s.index()].ousted);
            self.end_combat();
            return true;
        }
        false
    }

    /// The combat's acting side re-read (`GameState::combat_order`): with
    /// Nar-Sheptha gone, the acting minion is the usual one again at once,
    /// for the rest of the combat (ANK 20211022). Read where no step names
    /// a side: as such a step advances, after each pair of strikes, as a
    /// round begins.
    fn reorder_combat(&mut self) {
        let natural = self.combat.as_ref().expect("combat context").natural;
        let order = self.state.combat_order(natural[0], natural[1]);
        let c = self.combat.as_mut().expect("combat context");
        if c.sides[0].minion != order[0] {
            // Everything indexed by side follows it.
            c.sides.swap(0, 1);
            c.anathema_seen.swap(0, 1);
            let flip = |i: &mut usize| *i = 1 - *i;
            c.last_maneuver.iter_mut().for_each(flip);
            c.last_press.iter_mut().for_each(flip);
            c.press_toll.iter_mut().for_each(flip);
            if let Some((i, _)) = c.held_strike.as_mut() {
                flip(i);
            }
            if let Some((_, i, _)) = c.coordinate.as_mut() {
                flip(i);
            }
            for (_, t) in &mut c.traps {
                if let Some((i, _)) = t.as_mut() {
                    flip(i);
                }
            }
            let first = c.sides[0].seat;
            if let Some(k) = c.ring.iter().position(|&s| s == first) {
                c.ring.rotate_left(k);
            }
            self.state.combatants = Some(order);
        }
    }

    fn new_round(&mut self) {
        // The combat continues: after-combat effects are lost (RTR
        // 20020501).
        self.discard_held(true);
        for m in &mut self.state.minions {
            m.timed.retain(|&(_, until, _)| until != Expiry::Round);
        }
        self.reorder_combat();
        // Frenzy: its round's equipment bar lifted.
        let c = self.combat.as_ref().expect("combat context");
        let lifted: Vec<MinionId> = c
            .sides
            .iter()
            .filter(|s| s.no_equipment_round && !s.no_equipment && !s.frenzy_no_equipment)
            .map(|s| s.minion)
            .collect();
        self.state.equipment_barred.retain(|m| !lifted.contains(m));
        let c = self.combat.as_mut().expect("combat context");
        c.round_press = false;
        c.prev_close = c.range == Range::Close;
        c.round += 1;
        c.pair = 0;
        c.placed_this_round.clear();
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
        c.card_played_round = false;
        c.abilities_used_round.clear();
        // After-combat riders are lost when the combat continues (spec
        // §4.9.2).
        if let Some(a) = self.action.as_mut() {
            a.continuation = None;
            a.continue_free = false;
        }
        c.fresh_window(Step::BeforeRange);
    }

    /// "End a combat" outside strike resolution (Saulot's Guiding Wisdom):
    /// no strike resolves nor prevention, the end-of-round window still
    /// opens (advanced rules §4.7.2, §4.9).
    pub(crate) fn end_combat_now(&mut self) {
        let before_range = self
            .combat
            .as_ref()
            .is_some_and(|c| c.step == Step::BeforeRange);
        let pre_range = before_range
            && self
                .combat
                .as_ref()
                .is_some_and(|c| c.sides.iter().any(|s| s.pre_range > 0));
        // Ended before range is determined: Weather Control's damage still
        // lands (LSJ 19971110, ANK 20190725), and the combatants' (Outside
        // the Hourglass; ANK 20180104), the weather then with the
        // before-range step's close.
        if before_range && !pre_range && self.weather_damage() {
            return;
        }
        self.discard_held(false);
        let c = self.combat.as_mut().expect("combat context");
        c.round_over = true;
        c.press_continue = false;
        for s in &mut c.sides {
            s.set_strike(None);
            s.committed = None;
            s.committed_ability = None;
            s.committed_range_set = false;
            s.additional = 0;
        }
        if pre_range && self.stage_pre_range_damage() {
            return;
        }
        let c = self.combat.as_mut().expect("combat context");
        c.fresh_window(Step::EndOfRound);
    }

    /// What strikes landed that burns blood or cards, after they resolved.
    fn apply_landed(&mut self) {
        let Some(c) = self.combat.as_mut() else {
            return;
        };
        let mut end = false;
        for l in std::mem::take(&mut c.landed) {
            match l {
                Landed::Stun(v) => {
                    let x = &mut self.state.minions[v.index()];
                    x.locked = true;
                    x.skip_unlock = true;
                }
                Landed::Liquefy(v, n) => {
                    self.state.burn_blood(v, n);
                    end |= self.state.minions[v.index()].blood == 0;
                }
                Landed::Distill(v, weapon) => {
                    let card = self.state.cards[weapon.index()].card;
                    self.burn_card_in_play(weapon);
                    let Some((v, n)) = v else { continue };
                    self.state.burn_blood(v, n);
                    let owner = self.state.minions[v.index()].owner;
                    let mut stand_in = crate::state::CardInPlay::entering(
                        card,
                        crate::copies::CopyId::NONE,
                        owner,
                        Some(v),
                        1,
                    );
                    stand_in.effect_only = true;
                    self.place_now(stand_in, crate::plays::ModeLayers::Push, false);
                }
            }
        }
        if end {
            self.end_combat_now();
        }
    }

    /// Haven Hunting: both combatants ready, an Anarch among them, no
    /// combat queued (RTR 20020501): the end waits for its controller.
    fn offer_new_combat(&mut self) -> bool {
        let Some(c) = self.combat.as_ref() else {
            return false;
        };
        let st = &self.state;
        if c.psyche || c.coordinate.is_some() || c.redirect.is_some() || !c.both_ready(st) {
            return false;
        }
        let hook = crate::cards::Hook::OwnAnarchCombatEnded;
        let offer = (0..2).filter(|&i| !c.again_asked[i]).find_map(|i| {
            let s = &c.sides[i];
            (st.sect_of(s.minion) == Some(crate::crypt::Sect::Anarch))
                .then(|| self.fired_riders(hook, &crate::plays::HookAt::of(s.seat)))
                .and_then(|r| r.into_iter().next())
                .map(|(card, _)| (i, s.seat, card))
        });
        let Some((i, seat, card)) = offer else {
            return false;
        };
        self.combat.as_mut().expect("combat context").again_asked[i] = true;
        self.interrupts
            .push(crate::draws::Interrupt::MayNewCombat { seat, card });
        true
    }

    /// A vampire may enter combat with the opposing minion as this combat
    /// of its controller's clanmate ends (Marie-Pierre; ANK 20180928-2: after
    /// "would end" effects, before after-combat ones): the end waits for
    /// the answer.
    fn offer_join_combat(&mut self) -> bool {
        let Some(c) = self.combat.as_ref().filter(|c| {
            !c.join_asked && !c.psyche && c.coordinate.is_none() && c.redirect.is_none()
        }) else {
            return false;
        };
        let st = &self.state;
        let mut offer = None;
        for i in 0..2 {
            let (seat, fought) = (c.sides[i].seat, c.sides[i].minion);
            let opposing = c.sides[1 - i].minion;
            if st.seats[seat.index()].ousted
                || st.seats[c.sides[1 - i].seat.index()].ousted
                || !st.minions[opposing.index()].ready()
            {
                continue;
            }
            for m in st.minions_of(seat) {
                let v = &st.minions[m.index()];
                if m == fought || !v.ready() || v.locked {
                    continue;
                }
                let joins = self
                    .printed_riders(m, crate::cards::Hook::OwnClanCombatEnded, None)
                    .iter()
                    .flat_map(|r| r.effects)
                    .any(|e| {
                        matches!(*e, Effect::MayJoinCombat(clan)
                        if st.minions[fought.index()].clan() == Some(clan))
                    });
                if joins && offer.is_none() {
                    offer = Some((seat, m, i));
                }
            }
        }
        self.combat.as_mut().expect("combat context").join_asked = true;
        let Some((seat, minion, side)) = offer else {
            return false;
        };
        self.interrupts
            .push(crate::draws::Interrupt::MayJoinCombat {
                seat,
                minion,
                side: side as u8,
            });
        true
    }

    /// Combat over: back to the action's after-combat window (a combat
    /// begun at resolution then to after resolution, rules-qa 17.3).
    pub(crate) fn end_combat(&mut self) {
        // A diablerie made in the combat, the combat ended before its steps
        // (an oust settled first): they come first.
        if let Some((d, v)) = self.combat.as_mut().and_then(|c| c.diablerized.take()) {
            if self.suspend_for_hunt(d, v, true) {
                return;
            }
        }
        if self.offer_new_combat() || self.offer_join_combat() {
            return;
        }
        // "After combat with Amelia, the opposing minion burns 1 blood or
        // life."
        if let Some(c) = self.combat.as_ref() {
            let pairs = [
                (c.sides[0].minion, c.sides[1].minion),
                (c.sides[1].minion, c.sides[0].minion),
            ];
            for (m, o) in pairs {
                let n = self.state.printed_mod(
                    m,
                    cards::Stat::AfterCombatOpponentBurns,
                    crate::state::PrintedCtx::default(),
                );
                // Jason: the vampire blocking him.
                let jason = self.action.as_ref().is_some_and(|a| {
                    a.acting == m
                        && a.blocker == Some(o)
                        && self.state.minions[o.index()].is_vampire()
                });
                let n = n + if jason {
                    self.state.printed_mod(
                        m,
                        cards::Stat::BlockerBurnsAfterBlockedCombat,
                        crate::state::PrintedCtx::default(),
                    )
                } else {
                    0
                };
                let printed = Cause::Effect(crate::notes::Source::Minion(m));
                if n > 0
                    && self.state.minions[o.index()].present()
                    && self.caused(printed, |e| e.state.burn_blood_or_life(o, n as u8))
                {
                    self.lifeless(o);
                }
            }
        }
        let stand_ins = self
            .combat
            .as_mut()
            .map_or(Vec::new(), |c| std::mem::take(&mut c.stand_ins));
        for id in stand_ins {
            self.burn_card_in_play(id);
        }
        self.close_combat();
    }

    /// A diablerie made in the combat (Amaranth's, Liquefy [VIC]'s), its
    /// own steps done: the cards played on it and its blood hunt follow at
    /// once, the combat suspended (owner, 2026-11-11: rules-qa 113.10),
    /// `ending` it once over. Whether it was (none once the diablerist left
    /// play).
    pub(crate) fn suspend_for_hunt(&mut self, d: MinionId, v: MinionId, ending: bool) -> bool {
        if !self.state.minions[d.index()].in_play {
            return false;
        }
        let a = self.action.as_mut().expect("action context");
        a.hunt_in_combat = Some((a.state, ending));
        self.after_diablerie(d, v);
        true
    }

    /// Before any decision: a diablerie made in the combat, what it asked
    /// settled, the combat at rest between its steps.
    pub(crate) fn settle_combat_diablerie(&mut self) {
        let settled = self.interrupts.is_empty()
            && self.ousting.is_none()
            && self.state.control_change.is_none()
            && self.state.hunt_burn.is_none();
        if !settled || !matches!(self.flow, Flow::Combat) {
            return;
        }
        if let Some((d, v)) = self.combat.as_mut().and_then(|c| c.diablerized.take()) {
            self.suspend_for_hunt(d, v, false);
        }
    }

    /// The suspended combat's blood hunt over, or none called: back into
    /// the combat where it was, a combatant's Methuselah ousted meanwhile
    /// ending it. Whether one was suspended.
    pub(crate) fn resume_hunted_combat(&mut self) -> bool {
        let Some((state, ending)) = self.action.as_mut().and_then(|a| a.hunt_in_combat.take())
        else {
            return false;
        };
        self.action.as_mut().expect("the action").state = state;
        self.flow = Flow::Combat;
        if !self.combat_ousts() && ending {
            self.end_combat();
        }
        true
    }

    /// A combat cut short with its action (its Methuselah ousted), ended as
    /// told.
    pub(crate) fn cut_combat(&mut self) {
        if let Some(c) = self.combat.take() {
            self.note(CombatNote::Ends {
                minions: c.sides.each_ref().map(|s| s.minion),
            });
            for id in c.stand_ins {
                self.burn_card_in_play(id);
            }
            self.persisted_to_torpor(c.sides.each_ref().map(|s| s.minion));
        }
        self.state.leave_combat();
    }

    /// Undead Persistence: to torpor now that the combat ended, a new
    /// combat past (LSJ 20030214).
    pub(crate) fn persisted_to_torpor(&mut self, combatants: [MinionId; 2]) {
        for m in combatants {
            let v = &mut self.state.minions[m.index()];
            if std::mem::take(&mut v.torpor_after_combat) && v.ready() {
                self.minions_to_torpor(&[m]);
            }
        }
    }

    fn close_combat(&mut self) {
        let ended = self.combat.take();
        for s in ended.iter().flat_map(|c| c.sides.iter()) {
            self.state.minions[s.minion.index()].as_vampire_spent = false;
        }
        if let (Some(c), Some(a)) = (&ended, self.action.as_mut()) {
            a.combat_rounds = a.combat_rounds.saturating_add(c.round);
            a.combat_played = c.played_cards.clone();
        }
        // Yawp Court reads the acting vampire as the combat ends, before a
        // Rötschreck's torpor (LSJ 20080821).
        let acting_ready = self
            .action
            .as_ref()
            .is_some_and(|a| self.state.minions[a.acting.index()].ready());
        if let Some(c) = &ended {
            self.note(CombatNote::Ends {
                minions: c.sides.each_ref().map(|s| s.minion),
            });
            // Expiate unspent: a conviction a copy as the combat ends.
            for s in c.sides.iter().filter(|s| s.expiate > 0 && !s.expiated) {
                let held = self.state.convictions(s.minion).len();
                let n = (s.expiate as usize).min(held) as u8;
                if n > 0 && self.state.minions[s.minion.index()].present() {
                    self.interrupts
                        .push(crate::draws::Interrupt::BurnConvictions {
                            seat: s.seat,
                            minion: s.minion,
                            n,
                        });
                }
            }
        }
        // "Do not replace until after combat" (Lam Into): before a new
        // combat Psyche! queued too (LSJ 20001122).
        self.release_held(Release::AfterCombat);
        // Darkness Within: half the blood, rounded up, to its vampire; the
        // card burned with the rest, a new combat next too.
        for (id, to) in ended.iter().flat_map(|c| c.drains.clone()) {
            let n = std::mem::take(&mut self.state.cards[id.index()].counters).div_ceil(2);
            if self.state.minions[to.index()].present() {
                self.caused(Cause::Effect(Source::Card(id)), |e| {
                    e.state.gain_blood(to, n)
                });
            }
            if self.state.cards[id.index()].present() {
                self.burn_card_in_play(id);
            }
        }
        // Psyche! [CEL]: the new combat, both minions ready; this one's
        // after-combat effects lost, continuations included (RTR 20020501),
        // not its unlocks (LSJ 19980109).
        // Its natural order: Nar-Sheptha's re-read in the new combat.
        if let Some(c) = ended.as_ref().filter(|c| c.psyche) {
            let [x, y] = c.natural;
            let ready = |m: MinionId| self.state.minions[m.index()].ready();
            if ready(x) && ready(y) {
                if let Some(a) = self.action.as_mut() {
                    a.continue_free = false;
                    a.continued = false;
                    a.last_combat = Some([x, y]);
                }
                let (at_resolution, after_resolution, queued, yawp) =
                    (c.at_resolution, c.after_resolution, c.queued, c.yawp);
                self.state.leave_combat();
                let next = PsycheNext {
                    pair: [x, y],
                    at_resolution,
                    after_resolution,
                    queued,
                    yawp,
                };
                // A window before the new combat (rules-qa 88.10).
                if self.facing_acting(x, y) {
                    let a = self.action.as_mut().expect("action context");
                    a.psyche_next = Some((next, a.state));
                    a.state = ActionState::QueuedCombat;
                    a.fresh_window();
                    self.flow = Flow::Action;
                    return;
                }
                self.settle_hand_rises();
                self.begin_psyche_combat(next);
                return;
            }
        }
        // Coordinate Attacks: the Blood Brother locks and enters combat with
        // the opposing minion, still ready; the opposing minion plays no
        // strike card the first round.
        if let Some((c, (m, i, maneuver))) =
            ended.as_ref().and_then(|c| c.coordinate.map(|k| (c, k)))
        {
            let opposing = c.sides[1 - i].minion;
            let ready = |m: MinionId| self.state.minions[m.index()].ready();
            if ready(m) && ready(opposing) && !self.state.minions[m.index()].locked {
                let at_resolution = c.at_resolution;
                self.state.leave_combat();
                self.settle_hand_rises();
                self.state.minions[m.index()].locked = true;
                let (x, y) = if i == 0 { (m, opposing) } else { (opposing, m) };
                self.begin_combat(x, y, at_resolution);
                let n = self.combat.as_mut().expect("the new combat");
                n.held_at_block.clear();
                if !c.joined {
                    n.sides[1 - i].no_strike_cards_round = Some(1);
                }
                if maneuver {
                    n.sides[i].maneuvers += 1;
                }
                return;
            }
        }
        // Blissful Agony [VAL]: the opposing vampire, still ready, enters
        // combat with the chosen minion, still ready, going first (LSJ
        // 20011214-3, LSJ 20100604-1).
        if let Some((c, (m, i))) = ended.as_ref().and_then(|c| c.redirect.map(|k| (c, k))) {
            let opposing = c.sides[1 - i].minion;
            let ready = |m: MinionId| self.state.minions[m.index()].ready();
            if ready(m) && ready(opposing) {
                let at_resolution = c.at_resolution;
                self.state.leave_combat();
                self.settle_hand_rises();
                self.begin_combat(opposing, m, at_resolution);
                let n = self.combat.as_mut().expect("the new combat");
                n.held_at_block.clear();
                return;
            }
        }
        // FBI Special Affairs Division: its unpreventable environmental
        // damage to the acting vampire after the combat; enough counters,
        // burned.
        let fbi = ended.as_ref().map_or(0, |c| c.fbi_damage);
        if fbi > 0 {
            let acting = ended
                .as_ref()
                .zip(self.action.as_ref())
                .map(|(c, a)| c.acting_in(a));
            if let Some(acting) = acting {
                if self.state.minions[acting.index()].present() {
                    let by = match ended.as_ref().and_then(|c| c.fbi_card) {
                        Some(f) => Source::Card(f),
                        None => Source::Rule(Rule::Strikes),
                    };
                    self.caused(Cause::Effect(by), |e| {
                        e.damage_out_of_combat(acting, fbi, false)
                    });
                }
            }
            if let Some((f, _, burn_at)) = self.state.ally_burned_in_combat() {
                if self.state.cards[f.index()].counters >= burn_at {
                    self.burn_card_in_play(f);
                }
            }
        }
        if let Some(c) = &ended {
            self.persisted_to_torpor(c.sides.each_ref().map(|s| s.minion));
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
            a.last_combat = Some(c.natural);
            a.last_contracts = c.contracts.clone();
        }
        // "After combat ends, put this card on this vampire" (Pass Through
        // Shadow [OBL]).
        for h in ended.iter().flat_map(|c| c.after_combat.clone()) {
            self.place_held(h, h.minion);
        }
        // Catatonic Fear [PRE]: the vampire's damage, after combat, so no
        // combat card prevents it (LSJ 20010205; RTR 19970630).
        for (m, n, from) in ended.iter().flat_map(|c| c.after_combat_damage.clone()) {
            if self.state.minions[m.index()].present() {
                let strike = Cause::Effect(Source::Minion(from));
                self.caused(strike, |e| e.damage_out_of_combat(m, n, false));
            }
        }
        // Kiss of Cathari [OBF]/[PRE]: locked again if it unlocked during
        // the combat (LSJ 20040601), and it does not unlock next time.
        // Oubliette [OBT]: an ally with no life left is burned.
        for (m, n, from) in ended.iter().flat_map(|c| c.after_combat_burn.clone()) {
            if self.state.minions[m.index()].present() {
                let strike = Cause::Effect(Source::Minion(from));
                self.caused(strike, |e| e.state.burn_blood(m, n));
                let v = &self.state.minions[m.index()];
                if v.is_ally() && v.blood == 0 {
                    self.lifeless(m);
                }
            }
        }
        // Serpent's Numbing Kiss [PRE][SER]: the blood burned is mandatory
        // (ANK 20180420-2); locked again if unlocked meanwhile (LSJ
        // 20040601).
        for (h, on, blood, lock) in ended.iter().flat_map(|c| c.after_combat_put.clone()) {
            if blood > 0 && self.state.minions[h.minion.index()].present() {
                let by = Cause::Effect(Source::Minion(h.minion));
                self.caused(by, |e| e.state.burn_blood(h.minion, blood));
            }
            if self.state.minions[on.index()].present() {
                self.place_held(h, on);
                if lock {
                    self.state.minions[on.index()].locked = true;
                }
            }
        }
        for m in ended.iter().flat_map(|c| c.after_combat_stun.clone()) {
            let v = &mut self.state.minions[m.index()];
            if v.present() {
                v.locked = true;
                v.skip_unlock = true;
            }
        }
        if ended.as_ref().is_some_and(|c| c.at_resolution) {
            self.action.as_mut().expect("action context").resolved = true;
        }
        // Morbidity: the blood back to the vampire it came from, the card
        // burned.
        for (id, from) in ended.iter().flat_map(|c| c.morbidity.clone()) {
            let n = std::mem::take(&mut self.state.cards[id.index()].counters);
            if self.state.minions[from.index()].present() {
                self.caused(Cause::Effect(Source::Card(id)), |e| {
                    e.state.gain_blood(from, n)
                });
            }
            if self.state.cards[id.index()].present() {
                self.burn_card_in_play(id);
            }
        }
        // Inner Essence: its discount for this combat only.
        for id in ended.iter().flat_map(|c| c.burn_at_end.clone()) {
            if self.state.cards[id.index()].present() {
                self.burn_card_in_play(id);
            }
        }
        for id in ended.iter().flat_map(|c| c.return_at_end.clone()) {
            self.return_out_of_play(id);
        }
        // Trap, Conquer the Beast: burned as the combat ends.
        for (id, _) in ended.iter().flat_map(|c| c.traps.clone()) {
            if self.state.cards[id.index()].present() {
                self.burn_card_in_play(id);
            }
        }
        // Rage of Apedemak's hand size ends with the combat: the excess
        // discarded at once. Infernal Pursuit's draws, Aura Reading's open
        // hand end too.
        for s in ended.iter().flat_map(|c| c.sides.iter().map(|s| s.seat)) {
            self.state.seats[s.index()].timed.expire(Expiry::Combat);
            self.state.seats[s.index()].combat_open_hand = false;
        }
        // Abuse of Power: one combatant ready, the other not — the
        // controller of the ready one's opponent burns pool, the acting
        // minion's own if it is the one down (owner, 2026-10-08: rules-qa
        // 54.7).
        if let Some(c) = &ended {
            if let Some((n, card)) = c.one_ready_burn {
                let ready = |m: MinionId| self.state.minions[m.index()].ready();
                let [a, b] = [c.sides[0].minion, c.sides[1].minion];
                if ready(a) != ready(b) {
                    let down = if ready(a) { b } else { a };
                    let mut loss = vec![0; self.state.seats.len()];
                    loss[self.state.minions[down.index()].owner.index()] = n as i32;
                    self.caused(Cause::Effect(card), |e| e.burn_pool_batch(&loss));
                }
            }
            if let Some(n) = c.one_ready_gains {
                let ready = |m: MinionId| self.state.minions[m.index()].ready();
                let [a, b] = [c.sides[0].minion, c.sides[1].minion];
                if ready(a) != ready(b) {
                    self.state.gain_blood(if ready(a) { a } else { b }, n);
                }
            }
            // Pillars Fall: "if the opposing vampire is not ready, you can
            // put this card on this acting vampire" — one per vampire; the
            // action card enters play rather than the ash heap.
            if let Some(h) = c.put_if_opposing_down {
                let opposing = c.sides.iter().map(|s| s.minion).find(|&m| m != h.minion);
                let placed = (opposing.is_some_and(|o| !self.state.minions[o.index()].ready())
                    && self.state.minions[h.minion.index()].ready())
                .then(|| {
                    self.place_now(
                        crate::state::CardInPlay {
                            by: Some(h.minion),
                            ..crate::state::CardInPlay::entering(
                                h.card,
                                *h.copy,
                                h.seat,
                                Some(h.minion),
                                h.mode,
                            )
                        },
                        ModeLayers::Push,
                        true,
                    )
                })
                .flatten();
                if let (Some(_), Some(a)) = (placed, self.action.as_mut()) {
                    a.card_entered_play = true;
                }
            }
        }
        self.state.leave_combat();
        // Games of Instinct: after the combat (LSJ 20100324).
        if let Some(c) = ended.as_ref().filter(|c| c.lone_ready_gains_capacity) {
            let [a, b] = [c.sides[0].minion, c.sides[1].minion];
            let ready = |m: MinionId| self.state.minions[m.index()].ready();
            let winner = match (ready(a), ready(b)) {
                (true, false) => Some((a, b)),
                (false, true) => Some((b, a)),
                _ => None,
            };
            if let Some((w, l)) = winner {
                if self.state.minions[l.index()].is_vampire() {
                    let n = self.state.capacity_of(l).unwrap_or(0);
                    self.state.gain_blood(w, n);
                }
            }
        }
        if let Some((m, clan, n)) = ended.as_ref().and_then(|c| c.won_uncontrolled_blood) {
            let c = ended.as_ref().expect("combat");
            let other = c.sides.iter().find(|s| s.minion != m).map(|s| s.minion);
            let v = &self.state.minions[m.index()];
            if v.ready() && other.is_some_and(|o| !self.state.minions[o.index()].ready()) {
                self.interrupts
                    .push(crate::draws::Interrupt::UncontrolledBlood {
                        seat: v.owner,
                        clan: Some(clan),
                        n,
                        may: false,
                        cause: Cause::Effect(Source::Minion(m)),
                    });
            }
        }
        let yawp = ended.as_ref().and_then(|c| {
            let acting = self.action.as_ref().map(|a| a.acting);
            let torpored = acting.is_some_and(|m| c.torpored.contains(&m));
            c.yawp.map(|yawp| (yawp, acting_ready && !torpored))
        });
        self.after_combat_ends(
            ended.as_ref().is_some_and(|c| c.after_resolution),
            ended.as_ref().is_some_and(|c| c.queued),
            yawp,
        );
    }

    /// Where the action goes as its combats are over: back to its
    /// after-resolution window, its step after a queued combat, Yawp Court's
    /// end (its Sabbat vampire, the acting one still ready), or the
    /// after-combat window.
    fn after_combat_ends(
        &mut self,
        after_resolution: bool,
        queued: bool,
        yawp: Option<((MinionId, CardId), bool)>,
    ) {
        if after_resolution {
            self.back_to_after_resolution();
        } else if queued {
            self.resume_after_queued_combat();
        } else if let Some(((sabbat, court), ready)) = yawp {
            self.after_yawp_combat(sabbat, court, ready);
        } else {
            self.after_combat();
        }
    }

    /// Psyche!'s combat begins, the same two minions: no block began it, so
    /// block riders and "when blocking" equipment (Sniper Rifle) do not
    /// carry over (LSJ 20010813, LSJ 20010814). Either one no longer ready
    /// after Obedience's window, the action goes on as the first combat
    /// ended.
    pub(crate) fn begin_psyche_combat(&mut self, next: PsycheNext) {
        let [x, y] = next.pair;
        let ready = |m: MinionId| self.state.minions[m.index()].ready();
        if !(ready(x) && ready(y)) {
            let a = self.action.as_mut().expect("action context");
            a.resolved |= next.at_resolution;
            let ready = self.state.minions[a.acting.index()].ready();
            let yawp = next.yawp.map(|yawp| (yawp, ready));
            self.after_combat_ends(next.after_resolution, next.queued, yawp);
            return;
        }
        self.begin_combat(x, y, next.at_resolution);
        let n = self.combat.as_mut().expect("the new combat");
        n.held_at_block.clear();
        n.after_resolution = next.after_resolution;
        n.queued = next.queued;
        n.yawp = next.yawp;
    }

    /// Psyche!'s next combat is with the acting minion: reactions keyed to
    /// a minion "about to enter combat with" it may come first (Obedience,
    /// LSJ 19991025), the window elided when none can.
    fn facing_acting(&self, x: MinionId, y: MinionId) -> bool {
        self.action
            .as_ref()
            .is_some_and(|a| (x == a.acting) != (y == a.acting))
    }

    /// Dam the Heart's River [QUI]: each chosen strike not from a card
    /// costs its minion the toll now, an ally's last life burning it (LSJ
    /// 20090703). True when the combat ended.
    fn pay_strike_tolls(&mut self) -> bool {
        // Lucian's: none on a strike at a card on him (LSJ 20090528-2).
        let printed: Vec<u8> = (0..2).map(|i| self.printed_strike_toll(i)).collect();
        let c = self.combat.as_mut().expect("combat context");
        let mut due = Vec::new();
        for (i, s) in c.sides.iter_mut().enumerate() {
            let against = if s.strike_at.is_some() { 0 } else { printed[i] };
            let toll = s.toll_for(s.strike) + against;
            if std::mem::take(&mut s.toll_due) && s.strike.is_some() && toll > 0 {
                due.push((s.minion, toll));
            }
        }
        for (m, n) in due {
            let toll = Cause::Cost(Source::Minion(m));
            if self.caused(toll, |e| e.state.burn_blood_or_life(m, n)) {
                self.lifeless(m);
            }
        }
        self.combat.is_none()
    }

    /// "Strikes against Lucian cost an additional blood or life": the
    /// toll on side `i`'s strikes printed on the opposing minion, read live.
    pub(crate) fn printed_strike_toll(&self, i: usize) -> u8 {
        let c = self.combat.as_ref().expect("combat context");
        let ctx = crate::state::PrintedCtx {
            opponent: Some(c.sides[1 - i].minion),
            ..Default::default()
        };
        self.state
            .printed_mod(c.sides[i].minion, cards::Stat::StrikesAgainstToll, ctx)
            .max(0) as u8
    }

    /// A strike choice at the opposing minion itself (not aimed at a
    /// retainer) whose mode is not usable against flight (Earthshock).
    fn strike_barred_against_flight(&self, seat: SeatId, o: &Choice) -> bool {
        let mode = match *o {
            Choice::Play {
                card,
                mode,
                target: cards::Target::None | cards::Target::Minion(_),
                ..
            } => cards::lookup(self.playable_card(seat, card)).map(|d| &d.modes[mode as usize]),
            Choice::UseAbility {
                source,
                ability,
                target: cards::Target::None | cards::Target::Minion(_),
                ..
            } => Some(source.mode(&self.state, ability)),
            _ => None,
        };
        mode.is_some_and(|m| m.effects.contains(&Effect::NotAgainstFlight))
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
        // Only usable against a vampire (Waxen Poetica): never a retainer.
        let ranged = |mode: &cards::ModeDef| {
            !mode.requires.contains(&cards::Requirement::OpponentVampire)
                && mode.effects.iter().any(|e| {
                    matches!(
                        e,
                        Effect::Strike(
                            StrikeKind::Damage { ranged: true, .. }
                                | StrikeKind::StealBlood { ranged: true, .. }
                                | StrikeKind::RangedStrength { .. }
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

    /// "A vampire burns 1 blood after using a press to continue combat with
    /// her" (Petra): the press just used, if its opponent prints a toll.
    pub(crate) fn pay_press_toll(&mut self) {
        let Some(c) = self.combat.as_mut() else {
            return;
        };
        let Some(i) = c.press_toll.take() else {
            return;
        };
        let (presser, foe) = (c.sides[i].minion, c.sides[1 - i].minion);
        if !self.state.minions[presser.index()].is_vampire() {
            return;
        }
        let ctx = crate::state::PrintedCtx {
            opponent: Some(presser),
            ..Default::default()
        };
        let n = self
            .state
            .printed_mod(foe, cards::Stat::ContinuePressToll, ctx)
            .max(0) as u8;
        if n > 0 {
            self.state.burn_blood(presser, n);
        }
    }

    /// Effects of a combat card resolved during the combat.
    pub(crate) fn apply_combat_effect(&mut self, effect: Effect, play: &PlayCtx) {
        // From a minion out of the combat (Alejandro Aguirre).
        if let Effect::DamageEachCombatantNow(n) = effect {
            let c = self.combat.as_ref().expect("combat context");
            for m in [c.sides[0].minion, c.sides[1].minion] {
                if self.state.minions[m.index()].present() {
                    self.damage_out_of_combat(m, n, false);
                }
            }
            return;
        }
        if let Effect::DamageOpposingOwnCombatantNow(n) = effect {
            let c = self.combat.as_ref().expect("combat context");
            let foes: Vec<MinionId> = (0..2)
                .filter(|&i| c.sides[i].seat == play.seat)
                .map(|i| c.sides[1 - i].minion)
                .collect();
            for m in foes {
                self.damage_out_of_combat(m, n, false);
            }
            return;
        }
        if let Effect::AggravatedAsNormal { lock: Some(h) } = effect {
            let actor = play.actor.expect("combat card actor");
            if self.state.holds_unlocked(actor, h) > 0 {
                self.lock_held(actor, h, 1, None);
                if let Some(a) = self.action.as_mut() {
                    a.aggravated_as_normal.push(actor);
                }
            }
        }
        if let Effect::LockHeldToUnlock(h) = effect {
            let actor = play.actor.expect("combat card actor");
            if self.state.holds_unlocked(actor, h) > 0 {
                self.lock_held(actor, h, 1, None);
                self.effect_unlock(actor);
            }
            return;
        }
        let c = self.combat.as_mut().expect("combat context");
        let actor = play.actor.expect("combat card actor");
        // Effects of a minion out of the combat, on the target in it
        // (Martyr's Resilience).
        match effect {
            Effect::PreventNonAggravatedOnTarget(n) => {
                let back = match c.step {
                    Step::Prevent(k) => Some(k),
                    _ => None,
                };
                if let cards::Target::Minion(m) = play.target {
                    if let Some(t) = c.side_of_minion(m) {
                        let before = c.sides[t].pending;
                        c.sides[t].pending.prevent(n, false);
                        c.settle_source(t, before, false, play.seat, back);
                    }
                }
                return;
            }
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
            Effect::ActorBurnsBlood(n) => {
                self.state.burn_blood(actor, n);
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
                // A strike card paid its toll in its cost; a weapon's or a
                // vampire's strike pays as the pair is chosen.
                let card = cards::lookup(play.card).is_some_and(|d| {
                    d.modes
                        .get(play.mode as usize)
                        .is_some_and(|m| m.effects.contains(&Effect::Strike(k)))
                });
                let s = &mut c.sides[i];
                s.set_strike(Some(k));
                s.strike_play = Some((play.card, play.mode));
                s.toll_due = !card;
                // A card target: the retainer a ranged strike is aimed at,
                // or the equipment a burn-equipment strike names.
                match (k, play.target) {
                    (
                        StrikeKind::BurnEquipment
                        | StrikeKind::RangedBurnEquipment
                        | StrikeKind::StealWeapon,
                        cards::Target::Card(w),
                    ) => {
                        s.strike_at = None;
                        s.strike_weapon = Some(w);
                    }
                    (_, cards::Target::Card(r)) => s.strike_at = Some(r),
                    _ => s.strike_at = None,
                }
                s.chosen = true;
                if initial {
                    s.committed = None;
                    s.committed_ability = None;
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
            // At the maneuver, a maneuver committing the weapon's strike
            // (Lam Into [for]); at strike declaration, the strike.
            Effect::CommittedWeaponStrikePlus(n) if c.step == Step::DetermineRange => {
                let w = play.target.card();
                let k = cards::weapon_damage_strike(self.state.cards[w.index()].card)
                    .expect("a weapon's strike")
                    .1;
                c.maneuver(i);
                c.sides[i].committed_card = Some(k);
                c.sides[i].committed_weapon = Some((w, n));
            }
            // The weapon's own strike and its riders, +n damage, from that
            // card (gone by resolution, no effect: spec §4.10.4).
            Effect::WeaponStrikePlus(n) | Effect::CommittedWeaponStrikePlus(n) => {
                self.weapon_strike_plus(actor, play.target.card(), n, true);
            }
            // Orgy of Blood: the burned ally's recruit cost, to each
            // vampire of the player's in play (in torpor too).
            Effect::OrgyOfBlood => {
                let o = c.sides[1 - i].minion;
                let x = if self.state.minions[o.index()].kind.ally_card().is_some() {
                    self.state.ally_cost(o)
                } else {
                    0
                };
                let mine: Vec<MinionId> = self
                    .state
                    .minions_of(play.seat)
                    .filter(|&m| self.state.minions[m.index()].is_vampire())
                    .collect();
                for m in mine {
                    self.state.gain_blood(m, x);
                }
            }
            // Drawing Out the Beast: a frenzy card on the opposing vampire,
            // unless immune (Tranquility Shield).
            Effect::DrawOutBeast { damage } => {
                let o = &mut c.sides[1 - i];
                if !o.frenzy_immune {
                    o.frenzy_strength += 1;
                    o.no_long_maneuver = true;
                    o.no_end_press = true;
                    o.bar_equipment(&mut self.state, true);
                    o.press_damage += u8::from(damage);
                }
            }
            // A frenzy card on the opposing minion, unless immune (Tranquility
            // Shield).
            Effect::OpposingRestraint => {
                let o = &mut c.sides[1 - i];
                if !o.frenzy_immune {
                    o.no_close_maneuver = true;
                    o.no_continue_press = true;
                    o.bar_equipment(&mut self.state, true);
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
                // Dam the Heart's River's frenzy lifted.
                s.strike_card_toll = s.strike_card_toll.saturating_sub(s.strike_toll);
                s.strike_toll = 0;
                s.more_damage_taken = 0;
                s.no_close_maneuver = false;
                s.no_continue_press = false;
                s.frenzy_no_equipment = false;
                if !s.no_equipment {
                    let m = s.minion;
                    self.state.equipment_barred.retain(|&b| b != m);
                }
                let s = &mut c.sides[i];
                s.combat_card_surcharge = 0;
                s.no_long_maneuver = false;
                s.no_end_press = false;
                s.frenzy_strength = 0;
                s.press_damage = 0;
            }
            Effect::FleshOfMarble { aggravated } => {
                let s = &mut c.sides[i];
                s.marble = Some(aggravated || s.marble == Some(true));
            }
            Effect::WeatherDamage { escalate } => c.weather.push((c.round, escalate)),
            Effect::PreRangeDamage(n) => c.sides[1 - i].pre_range += n,
            Effect::ShieldFromOpposingStrike(n) => c.sides[i].strike_shield += n,
            Effect::OpposingCelerityMayBurnToPrevent(n) => {
                c.sides[1 - i].celerity_burn_prevent = Some((
                    n,
                    Source::Played {
                        card: play.card,
                        copy: *play.copy,
                    },
                ))
            }
            // Pulled Fangs: not strike damage, offered to prevention before
            // it lands, the end of the round then resumed (rules-qa 74.1);
            // on a vampire sent to torpor this round too (74.13); one
            // burned is no target (`Requirement::OpponentPresent`).
            Effect::DamageOpponentNow(n) => {
                let (from, to) = (c.sides[i].minion, c.sides[1 - i].minion);
                let n = n.saturating_sub(self.state.damage_reduction(to, from));
                if n > 0 {
                    let o = &mut c.sides[1 - i];
                    o.pending.normal += n;
                    o.preventing = true;
                    c.others_prevented = false;
                    c.end_of_round_damage = true;
                    c.step = Step::Prevent(1 - i);
                }
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
            Effect::RetainerDamageWithStrike(n) => c.sides[i].retainer_strike_damage = n,
            Effect::UndodgeableThisRound => c.sides[i].undodgeable_round = true,
            Effect::AggravatedStrike => c.sides[i].aggravated_strike = true,
            Effect::BurnElectronicOnCombatants => c.sides[i].burns_electronic = true,
            Effect::Expiate => c.sides[i].expiate += 1,
            Effect::NeverAggravatedStrike => c.sides[i].never_aggravated = true,
            Effect::UnpreventableIfBlackHand => c.sides[i].unpreventable_if_black_hand = true,
            Effect::FirstStrike => c.sides[i].first_strike = true,
            Effect::FirstStrikeThisRound => c.sides[i].first_strike_round = true,
            Effect::LoadGun(ammo) => {
                if let Some(w) = c.sides[i].strike_source {
                    c.ammo.push((w, ammo));
                }
            }
            Effect::BurnOpponentInsteadOfTorpor => c.sides[1 - i].burn_instead_of_torpor = true,
            Effect::DiablerizeOpponentInsteadOfTorpor => {
                c.sides[1 - i].diablerized_instead_of_torpor = true;
            }
            Effect::TorporAfterCombat { quiet_rounds } => {
                c.sides[i].persisting = true;
                c.quiet_rounds = 0;
                c.quiet_limit = Some(quiet_rounds);
                self.state.minions[c.sides[i].minion.index()].torpor_after_combat = true;
            }
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
            Effect::PutOnSelfAsStrikeResolves => {
                c.sides[i].held_self = Some(Held::from_play(play, actor, 0));
            }
            Effect::BurnOpposingAllyOnHit => c.sides[i].burns_ally_on_hit = true,
            Effect::StunOpponentOnHit => c.sides[i].stuns_on_hit = true,
            Effect::PutOnActorAfterCombat => c.after_combat.push(Held::from_play(play, actor, 0)),
            Effect::AimExtraDamage(n) => c.sides[i].aim = n,
            Effect::StrengthThisRound(n) => c.sides[i].strength_round += n,
            Effect::CombatEndsAfterStrike { unless_dodged } => {
                c.sides[i].ends_combat_after = true;
                c.sides[i].ends_unless_dodged = unless_dodged;
            }
            Effect::StrikeDamageZero => c.sides[i].strike_zero = true,
            Effect::StealTargetCardOnHit => {
                // The card is the one taken, not a retainer aimed at.
                c.sides[i].strike_at = None;
                c.sides[i].steal_card_on_hit = Some(play.target.card());
            }
            // Evil Eye [mal]: the canceled strike card's minion strikes not
            // this exchange.
            Effect::OpposingGetsNoStrike => {
                let o = &mut c.sides[1 - i];
                o.set_strike(None);
                o.chosen = true;
            }
            // No strike got, no limit used (rules-qa 29.8).
            Effect::AdditionalHandStrikesEachRoundX => {
                let s = &mut c.sides[i];
                s.additional += play.x;
                s.hand_only_strikes += play.x;
                s.hand_strikes_each_round += play.x;
                s.limited_used |= play.x > 0;
            }
            Effect::ResolutionDamageEachRoundIfClose(n) => c.sides[i].resolution_damage_close += n,
            Effect::OpponentEntersCombatWithTarget => {
                c.sides[i].redirect_rider = Some(play.target.minion());
                // Played by both: the acting Methuselah picks which
                // resolves first, the other lost (PIB 20120225, LSJ
                // 20011215).
                if c.sides[1 - i].redirect_rider.is_some() {
                    self.interrupts
                        .push(crate::draws::Interrupt::RedirectFirst {
                            seat: c.sides[0].seat,
                            strikers: [c.sides[0].minion, c.sides[1].minion],
                        });
                }
            }
            Effect::PoolIfStrikerBurned(n) => c.sides[i].pool_if_burned = n,
            Effect::StrengthThisRoundVotes => {
                c.sides[i].strength_round += self.state.votes_held(actor);
            }
            Effect::PreventThisRound(n) => c.sides[i].round_prevents += n,
            Effect::PreventEachRound(n) => {
                c.sides[i].prevent_each_round += n;
                c.sides[i].round_prevents += n;
            }
            Effect::PressThisRound => c.sides[i].presses += 1,
            Effect::PreventOpposingStrikesThisRound | Effect::PreventStrikeDamage => {
                let s = &mut c.sides[i];
                s.prevent_strike(u8::MAX, true);
                // Rolling with the Punches: the rest of the round's strikes
                // too; Touch of Valeren: that strike only.
                s.strikes_prevented |= effect == Effect::PreventOpposingStrikesThisRound;
            }
            Effect::PreventStrike { n, aggravated } => c.sides[i].prevent_strike(n, aggravated),
            Effect::NullifyOpposingWeapons => c.sides[i].nullifies_weapons = true,
            Effect::OpposingNoDodgeThisRound => c.sides[1 - i].no_dodge_round = Some(c.round),
            Effect::MayBurnToPreventInitialStrikes(n) => {
                c.sides[i].burn_to_prevent = Some((
                    c.round,
                    n,
                    Source::Played {
                        card: play.card,
                        copy: *play.copy,
                    },
                ))
            }
            Effect::OpposingNoCombatEndsThisRound => {
                c.sides[1 - i].no_combat_ends_round = Some(c.round)
            }
            Effect::EnvironmentalDamage { n, close: false } => c.sides[i].environmental += n,
            Effect::EnvironmentalDamage { n, close: true } => c.sides[i].environmental_close += n,
            Effect::EnvironmentalDamageWhileTarget(n) => {
                if let crate::cards::Target::Card(card) = play.target {
                    c.sides[i].environmental_while.push((card, n));
                }
            }
            Effect::RoundStrike(k) => c.sides[i].round_strike = Some(k),
            Effect::StrikeOnceEachRound(k) => {
                let s = &mut c.sides[i];
                s.combat_strike = Some(k);
                s.round_strike = Some(k);
                s.round_strike_once = true;
            }
            Effect::PreventNonAggravatedEachRound(n) => {
                let s = &mut c.sides[i];
                s.normal_prevent_each_round += n;
                s.round_normal_prevents += n;
            }
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
            Effect::CloseManeuverForBlood(n) => c.sides[i].close_maneuver_blood = n,
            Effect::ManeuverForBlood(n) => c.sides[i].maneuver_blood = n,
            Effect::HandStrikeDamageThisCombat(n) => c.sides[i].hand_damage_bonus += n,
            Effect::AggravatedAsNormal { .. } => c.sides[i].aggravated_as_normal = true,
            Effect::NoFrenzyCancelNorEndPress => {
                c.sides[i].no_frenzy_cancel = true;
                c.sides[i].no_end_press = true;
            }
            Effect::HandStrikeBonusPerHeld(h) => {
                c.sides[i].hand_damage_round += self.state.holds_count(actor, h);
            }
            Effect::ExtraDrawPerReplacementThisCombat => {
                let seat = c.sides[i].seat;
                self.state.seats[seat.index()]
                    .timed
                    .add(TimedStat::ExtraDraws, Expiry::Combat, 1);
            }
            Effect::AdditionalStrikesEachRound(n) => c.sides[i].strikes_each_round += n,
            Effect::CloseSelfDamage(n) => c.sides[i].close_self_damage = n,
            Effect::ContinuePressThisRound => c.sides[i].continue_presses += 1,
            Effect::EndPressThisRound => c.sides[i].end_presses += 1,
            Effect::OpposingNoManeuverThisRound => c.sides[1 - i].no_maneuver_round = Some(c.round),
            Effect::OpposingImprisoned => c.sides[1 - i].imprisoned += 1,
            Effect::ReflectsDamageThisCombat => c.sides[i].reflects = true,
            Effect::OpposingNoInitialStrikeThisRound => {
                c.sides[1 - i].no_initial_strike_round = Some(c.round)
            }
            Effect::OpposingNoStrikeCardsThisCombat => c.sides[1 - i].no_strike_cards_combat = true,
            // Two plays add up (no limit printed), Call the Wild Hunt's
            // too.
            Effect::EnvironmentalDamageThisRound(n) => {
                add_this_round(&mut c.sides[i].environmental_round, n, c.round)
            }
            Effect::PreventHalfStrike => {
                let d = c.sides[i].strike_damage;
                let n = (d.normal + d.aggravated).div_ceil(2);
                c.sides[i].prevent_strike(n, true);
            }
            Effect::HalveOpposingStrikesThisCombat => c.sides[i].halve_strikes = true,
            Effect::CombatManeuver => c.sides[i].combat_maneuvers += 1,
            Effect::FrenzyImmuneThisCombat => c.sides[i].frenzy_immune = true,
            // Frenzy on the opposing minion (ANK 20220409): not on one
            // immune.
            Effect::OpposingTakesMoreThisCombat(n) if !c.sides[1 - i].frenzy_immune => {
                c.sides[1 - i].more_damage_taken += n
            }
            Effect::OpposingStrikeTollThisCombat(n) if !c.sides[1 - i].frenzy_immune => {
                c.sides[1 - i].strike_toll += n;
                c.sides[1 - i].strike_card_toll += n;
            }
            Effect::OpposingTakesMoreThisCombat(_) | Effect::OpposingStrikeTollThisCombat(_) => {}
            Effect::ResolutionDamageThisRound(n) => {
                add_this_round(&mut c.sides[i].resolution_damage, n, c.round)
            }
            Effect::EnvironmentalDamageThisRoundX => {
                add_this_round(&mut c.sides[i].environmental_round, play.x, c.round)
            }
            Effect::DiscardChosenX(of) => {
                if play.x > 0 {
                    self.interrupts
                        .push(crate::draws::Interrupt::DiscardChosen {
                            seat: play.seat,
                            left: play.x,
                            done: 0,
                            of: Some(of),
                            optional: false,
                        });
                }
            }
            Effect::DisciplineStrikeDamageThisCombat { discipline, n } => {
                c.sides[i].discipline_strike_bonus.push((discipline, n))
            }
            Effect::OpposingBurnsBlood(n) => {
                let o = c.sides[1 - i].minion;
                self.state.burn_blood(o, n);
            }
            Effect::OpposingHandStrikerBurns { n, cap } => {
                let o = c.sides[1 - i].minion;
                let small = self
                    .state
                    .capacity_by(o, c.sides[i].seat)
                    .is_some_and(|k| k <= cap);
                if c.sides[1 - i].hand_struck && small {
                    self.state.burn_blood(o, n);
                }
            }
            Effect::HandSizeThisCombat(n) => {
                let seat = c.sides[i].seat;
                self.state.seats[seat.index()].timed.add(
                    TimedStat::HandSize,
                    Expiry::Combat,
                    n as i8,
                );
            }
            // Played at once if one can be (`ash_play_due`).
            Effect::PlayCombatFromAsh => {
                self.state.seats[play.seat.index()].ash_combat_play = true;
                if self.ash_play_due().is_none() {
                    self.state.seats[play.seat.index()].ash_combat_play = false;
                }
            }
            Effect::OpponentOpenHandThisCombat => {
                let seat = c.sides[1 - i].seat;
                self.state.seats[seat.index()].combat_open_hand = true;
            }
            Effect::AdditionalStrikes { n, limited } => {
                c.sides[i].additional += n;
                if limited {
                    c.sides[i].limited_used = true;
                }
            }
            Effect::SetStrength(n) => c.sides[i].strength_set = Some(n),
            Effect::StrengthThisCombat(n) => c.sides[i].strength_mods += n,
            Effect::OpposingStrengthMinus(n) => c.sides[1 - i].strength_minus += n,
            Effect::OpposingNoAdditionalStrikes => {
                let o = &mut c.sides[1 - i];
                o.no_additional_strikes = true;
                o.no_continue_press = true;
            }
            Effect::AggravatedHands => c.sides[i].aggravated_hands = true,
            Effect::AggravatedHandsThisCombat => {
                c.sides[i].aggravated_hands = true;
                c.sides[i].aggravated_hands_combat = true;
            }
            Effect::Prevent(n) => c.prevent(i, n),
            Effect::PreventXPlus(k) => c.prevent(i, play.x + k),
            Effect::PreventNonAggravated(n) => {
                let before = c.sides[i].pending;
                c.sides[i].pending.prevent(n, false);
                let by = c.sides[i].seat;
                c.settle_source(i, before, false, by, Some(i));
            }
            Effect::PreventAggravated(n) => {
                let before = c.sides[i].pending;
                c.sides[i].pending.prevent_aggravated(n);
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
            Effect::QueueCombatWithOpponent => {
                c.psyche = true;
                c.end_fixed = true;
            }
            // Coordinate Attacks: the actor out of the combat takes its
            // Methuselah's combatant's place as the combat ends.
            Effect::CoordinateAttack { maneuver } => {
                c.coordinate = Some((actor, i, maneuver));
                c.end_fixed = true;
            }
            Effect::GainBloodLostByOpponent => {
                let gained = c.sides[1 - i].lost_to_damage;
                self.state.gain_blood(actor, gained);
            }
            Effect::ContinueFreeAfterCombat(not) => {
                if !self.action.as_ref().is_some_and(|a| not.contains(&a.kind)) {
                    c.sides[i].continue_free = true;
                }
            }
            Effect::BurnAfterCombat(n) => c.sides[i].after_burn_rider = Some(n),
            Effect::ContinueAfterCombat { blood, stealth } => {
                c.sides[i].continue_rider = Some((blood, stealth));
            }
            Effect::DamageAfterCombatIfClose(n) => c.sides[i].after_damage_rider = Some(n),
            Effect::PressDamageIfClose(n) => c.sides[i].press_damage_rider = Some(n),
            Effect::LiquefyOnHit(n) => c.sides[i].liquefy_rider = Some(n),
            Effect::DistillOnHit(n) => c.sides[i].distill_rider = Some(n),
            Effect::DiablerizeOnHit => c.sides[i].diablerie_rider = true,
            Effect::StrikeDiablerizesNextRound => {
                c.sides[i].no_strike_round = Some(c.round);
                c.sides[i].diablerie_round = Some(c.round + 1);
                let mut stand_in = crate::state::CardInPlay::entering(
                    play.card,
                    crate::copies::CopyId::NONE,
                    play.seat,
                    Some(actor),
                    play.mode,
                );
                stand_in.effect_only = true;
                if let Some(id) = self.place_now(stand_in, crate::plays::ModeLayers::Push, false) {
                    self.combat
                        .as_mut()
                        .expect("combat context")
                        .stand_ins
                        .push(id);
                }
            }
            // After strike resolution, its prevention as any damage's.
            Effect::GrappleDamage(n) => {
                c.sides[i].hand_landed = false;
                c.sides[1 - i].no_press_round = true;
                c.sides[1 - i].pending.add(n, false);
                c.sides[1 - i].preventing = true;
                self.after_prevention_or_next();
            }
            Effect::DamageAfterCombat(n) => {
                c.sides[i].after_damage_rider = Some(n);
                c.sides[i].after_damage_any = true;
            }
            Effect::StunAfterCombatIfClose => c.sides[i].after_stun_rider = true,
            Effect::PutOnOpponentAfterCombatIfClose { blood, lock } => {
                c.sides[i].after_put_rider = Some((Held::from_play(play, actor, 0), blood, lock));
            }
            Effect::UnpreventableStrikeDamage => c.sides[i].unpreventable_strike = true,
            Effect::ContinuePressForBloodThisRound(n) => c.sides[i].continue_press_blood = n,
            Effect::UnlockForBloodThisCombat(n) => c.sides[i].unlock_blood = n,
            Effect::OpposingRetainersLoseTextThisCombat => {
                let o = c.sides[1 - i].minion;
                if !self.state.retainers_silenced.0.contains(&o) {
                    self.state.retainers_silenced.0.push(o);
                }
            }
            Effect::OpposingNoCloseNorContinue => c.sides[1 - i].restrained = true,
            Effect::AggravatedAsNormalThisCombat => {
                c.sides[i].aggravated_as_normal = true;
                c.sides[i].aggravated_as_normal_combat = true;
            }
            Effect::NamedCardNoEffect { name, combat } => {
                let round = (!combat).then_some(c.round);
                c.sides[i].immune_to.push((name, round));
            }
            Effect::OpposingNoStrikeRestOfRound => c.sides[i].bars_strikes_on_hit = true,
            Effect::IfDodgedSelfDamageAndOpposingPress(n) => c.sides[i].dodged_rider = n,
            Effect::WeaponDamageThisCombat { n, aggravated } => {
                if let cards::Target::Card(w) = play.target {
                    c.sides[i].weapon_bonus.push((w, n, aggravated));
                }
            }
            Effect::PutOnOpponentAtRoundEndIfMore => {
                c.sides[i].round_end_put = Some(Held::from_play(play, actor, 0));
            }
            // Hunting the Quarry [cel]/[tha]: on the opposing minion as the
            // combat ends, its player keeping control.
            Effect::PutOnOpposingKeepControl => {
                let opposing = c.sides[1 - i].minion;
                c.after_combat.push(Held::from_play(play, opposing, 0));
                c.end_fixed = true;
            }
            other => unreachable!("not a combat effect: {other:?}"),
        }
    }
}

impl CombatCtx {
    /// Start a strike pair: the normal pair (both choose) or an additional
    /// one (only minions with additional strikes choose; the range is the
    /// round's).
    fn begin_pair(&mut self, st: &GameState) {
        if self.step != Step::BeforeStrikes {
            self.pair += 1;
        }
        self.others_prevented = false;
        // "Cannot strike" this round: no strike is chosen, in any pair
        // (additional strikes are still gained, uselessly: LSJ 19970814-2).
        let barred = self.no_strikes_round == Some(self.round);
        let round = self.round;
        for s in &mut self.sides {
            s.set_strike(None);
            s.hand_only_pair = false;
            let initial = self.pair == 0 && s.no_initial_strike_round == Some(round);
            if barred
                || initial
                || s.frozen_round == Some(round)
                || s.no_strike_round == Some(round)
            {
                // The pair's additional strike is spent, unused.
                if self.pair > 0 {
                    s.additional = s.additional.saturating_sub(1);
                    s.hand_only_strikes = s.hand_only_strikes.saturating_sub(1);
                }
                s.chosen = true;
            } else if self.pair == 0 {
                s.chosen = false;
            } else if s.additional > 0 && !s.no_additional(st) {
                s.additional -= 1;
                s.chosen = false;
                s.hand_only_pair = s.hand_only_strikes > 0;
                s.hand_only_strikes = s.hand_only_strikes.saturating_sub(1);
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
        // A press to continue: Petra's toll, paid as the engine settles it.
        if self.press_continue {
            self.press_toll = Some(i);
        }
    }
}

/// Weather Control's damage this round: 1 a copy, growing each round after
/// it for [THA].
fn weather_amount(c: &CombatCtx) -> u8 {
    c.weather
        .iter()
        .map(|&(from, escalate)| 1 + if escalate { c.round - from } else { 0 })
        .sum()
}

/// `n` more for `round`, an earlier round's amount dropped.
pub(crate) fn add_this_round(slot: &mut Option<(u8, u8)>, n: u8, round: u8) {
    let k = slot.filter(|&(_, r)| r == round).map_or(0, |(k, _)| k);
    *slot = Some((k + n, round));
}

/// Improvised Flamethrower: each copy on side `i`'s minion burns, its
/// bearer taking n aggravated damage, its own (a second copy its own too,
/// ANK 20230316).
fn backfire(st: &GameState, c: &mut CombatCtx, i: usize) -> Vec<CardId> {
    let mut out = Vec::new();
    for l in st.borne(c.sides[i].minion) {
        if let cards::Modifier::BackfiresAtLong(n) = l.modifier {
            c.sides[i].pending.add(n, true);
            c.sides[i].self_damage += n;
            out.push(l.source);
        }
    }
    out
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
        let r = resolve_damage(1, d(0, 1), false);
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
        let r = resolve_damage(2, d(0, 3), false);
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
        let r = resolve_damage(1, d(2, 1), false);
        assert!(r.burned);
        assert_eq!(r.blood_burned, 1);
    }

    #[test]
    fn a_vampire_in_torpor_mends_normal_and_burns_to_aggravated() {
        // Rulebook "Wounded": "one that is in torpor"; unmended normal damage
        // leaves him as he is, each aggravated point burns 1 blood or him.
        let r = resolve_damage(1, d(2, 0), true);
        assert_eq!(
            r,
            Resolved {
                blood_burned: 1,
                wounded: false,
                burned: false
            }
        );
        let r = resolve_damage(1, d(0, 2), true);
        assert!(r.burned && !r.wounded);
        assert_eq!(r.blood_burned, 1);
    }

    #[test]
    fn mended_damage_leaves_the_vampire_ready() {
        let r = resolve_damage(3, d(2, 0), false);
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
