//! Card behavior (DESIGN §4.5): a card is a set of modes; a mode's legality
//! half (windows, actor, requirements, cost, targets, terms) is a closed
//! vocabulary evaluated here, so the option mask is enumerated from data; its
//! execution half (effects) is interpreted at resolution. Definitions live in
//! `defs`.

pub mod defs;
pub mod families;
pub mod slots;
pub mod vampires;

use crate::actions::{ActionCtx, ActionState};
use crate::combat::{CombatCtx, Range, Step};
use crate::crypt::{Sect, Title};
use crate::decision::ActionKind;
use crate::ids::{MinionId, SeatId};
use crate::library::{CardType, Cost, LIBRARY};
use crate::referendum::{RefKind, RefStep, ReferendumCtx};
use crate::state::{CardId, GameState};

/// Play windows: the names of data/window-matrix-reviewed.csv, plus the
/// in-play ability hooks the engine opens (`UnlockPhaseAny`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Window {
    MasterPhase,
    CardAsPlayed,
    DiscardPhase,
    ActionDeclare,
    ActionDeclarePolitical,
    ActionDeclareEquip,
    /// An imbued's action to get a power (rulebook "Appendix: Imbued
    /// Rules" 3: +1 stealth, as equipment).
    ActionDeclarePower,
    /// The active Methuselah's unlock phase: a conviction played on one of
    /// their imbued, from hand or ash heap (rulebook "Appendix: Imbued
    /// Rules" 2).
    UnlockConviction,
    ActionDeclareRecruit,
    ActionDeclareEmploy,
    ActionAsAnnounced,
    /// As the action is announced, for a Methuselah other than the acting
    /// one, after the acting one's own as-announced effects (Screamer, LSJ
    /// 20081105).
    ReactionAsAnnounced,
    ActionDuring,
    ActionStealthIntercept,
    ActionBlockAttempt,
    ActionBlocked,
    ActionBlocksDeclined,
    ActionWouldBeSuccessful,
    ActionAfterResolution,
    ActionAfterCombat,
    /// After a diablerie, before its blood hunt (Slake the Thirst; ANK
    /// 20201228).
    ActionAfterDiablerie,
    /// Before a combat Psyche! queued with the acting vampire (Obedience;
    /// LSJ 19991025).
    ActionQueuedCombat,
    ReactionDuringAction,
    ReactionUnlock,
    ReactionUnlockAttempt,
    Wake,
    Referendum,
    CombatBeforeRange,
    CombatDetermineRange,
    CombatBeforeStrikes,
    CombatStrikeDeclaration,
    CombatAdditionalStrike,
    CombatDamagePrevention,
    /// After the preventions, damage about to send the opposing vampire to
    /// torpor (Decapitate [POT]).
    CombatGoingToTorpor,
    CombatPress,
    CombatEndOfRound,
    CombatWouldEnd,
    CombatUntimed,
    /// Both strikes chosen, before they resolve, for a card triggered by
    /// the opposing strike (Rötschreck; LSJ 19990217).
    CombatBeforeResolution,
    /// The first strike announced, before the second is chosen
    /// (`Step::BetweenStrikes`): Rötschreck, before or after the vampire's
    /// own strike (LSJ 19990217).
    CombatStrikeAnnounced,
    /// A Methuselah with no pool, before the oust applies (Life Boon).
    Ousting,
    /// Control of a minion about to change, before it does: its
    /// controller's (The Diamond Thunderbolt; `crate::control`).
    ControlAboutToChange,
    /// A passed blood hunt about to burn its vampire, before it does:
    /// another Methuselah's (Redline; `crate::control`).
    BloodHuntWouldBurn,
    /// A ready titled vampire about to be burned or sent to torpor
    /// (Leadership Vacuum; `Interrupt::ReadyTitledLeft`).
    ReadyTitledLeft,
    /// In-play ability hook: during any Methuselah's unlock phase.
    UnlockPhaseAny,
    /// In-play ability hook: during the host's controller's unlock phase
    /// (Vessel).
    UnlockPhaseOwn,
    /// The active Methuselah's discard phase, for an ability spending a
    /// discard phase action (Ayse Dhanial): no "during", not once per phase.
    DiscardPhaseAction,
    /// In-play ability hook: during the controller's influence phase, once
    /// per phase per copy (Arcane Library).
    InfluencePhaseOwn,
    /// The controller's influence phase, for an ability spending transfers
    /// (Wider View): no "during", as often as the transfers last.
    Transfers,
    /// In-play ability hook: any decision point where the controller holds
    /// the impulse (ANK 20200616), except the as-played window (rules-qa
    /// 2.4).
    AnyImpulse,
    /// In-play ability hook: another Methuselah's minion phase, as they
    /// declare its end (rules-qa 7.9; Enkil Cog, rules-qa 49.6).
    OthersMinionPhaseEnd,
    /// Another Methuselah's discard phase, once each per copy (Giotto
    /// Verducci; rules-qa 6.6).
    OthersDiscardPhase,
    /// Another Methuselah's influence phase, offered in the rotation that
    /// closes it (Gather; ANK 20240118).
    OthersInfluencePhase,
}

impl Window {
    /// A phase window: an ability here is once per phase per copy
    /// (rules-qa 6.6).
    pub fn is_phase(self) -> bool {
        matches!(
            self,
            Window::MasterPhase
                | Window::UnlockPhaseAny
                | Window::UnlockPhaseOwn
                | Window::InfluencePhaseOwn
                | Window::DiscardPhase
                | Window::OthersDiscardPhase
                | Window::OthersInfluencePhase
        )
    }
}

impl Window {
    /// A combat window.
    pub fn is_combat(self) -> bool {
        use Window::*;
        matches!(
            self,
            CombatBeforeRange
                | CombatDetermineRange
                | CombatBeforeStrikes
                | CombatStrikeDeclaration
                | CombatAdditionalStrike
                | CombatDamagePrevention
                | CombatPress
                | CombatEndOfRound
                | CombatWouldEnd
                | CombatUntimed
                | CombatBeforeResolution
                | CombatStrikeAnnounced
        )
    }

    pub fn from_name(name: &str) -> Option<Window> {
        use Window::*;
        Some(match name {
            "MASTER_PHASE" => MasterPhase,
            "CARD_AS_PLAYED" => CardAsPlayed,
            "DISCARD_PHASE" => DiscardPhase,
            "ACTION_DECLARE" => ActionDeclare,
            "ACTION_DECLARE_POLITICAL" => ActionDeclarePolitical,
            "ACTION_DECLARE_EQUIP" => ActionDeclareEquip,
            "ACTION_DECLARE_POWER" => ActionDeclarePower,
            "UNLOCK_CONVICTION" => UnlockConviction,
            "ACTION_DECLARE_RECRUIT" => ActionDeclareRecruit,
            "ACTION_DECLARE_EMPLOY" => ActionDeclareEmploy,
            "ACTION_AS_ANNOUNCED" => ActionAsAnnounced,
            "REACTION_AS_ANNOUNCED" => ReactionAsAnnounced,
            "CONTROL_ABOUT_TO_CHANGE" => ControlAboutToChange,
            "BLOOD_HUNT_WOULD_BURN" => BloodHuntWouldBurn,
            "UNLOCK_PHASE_ANY" => UnlockPhaseAny,
            "ACTION_DURING" => ActionDuring,
            "ACTION_STEALTH_INTERCEPT" => ActionStealthIntercept,
            "ACTION_BLOCK_ATTEMPT" => ActionBlockAttempt,
            "ACTION_BLOCKED" => ActionBlocked,
            "ACTION_BLOCKS_DECLINED" => ActionBlocksDeclined,
            "ACTION_WOULD_BE_SUCCESSFUL" => ActionWouldBeSuccessful,
            "ACTION_AFTER_RESOLUTION" => ActionAfterResolution,
            "ACTION_AFTER_COMBAT" => ActionAfterCombat,
            "ACTION_AFTER_DIABLERIE" => ActionAfterDiablerie,
            "ACTION_QUEUED_COMBAT" => ActionQueuedCombat,
            "REACTION_DURING_ACTION" => ReactionDuringAction,
            "REACTION_UNLOCK" => ReactionUnlock,
            "REACTION_UNLOCK_ATTEMPT" => ReactionUnlockAttempt,
            "WAKE" => Wake,
            "REFERENDUM" => Referendum,
            "COMBAT_BEFORE_RANGE" => CombatBeforeRange,
            "COMBAT_DETERMINE_RANGE" => CombatDetermineRange,
            "COMBAT_BEFORE_STRIKES" => CombatBeforeStrikes,
            "COMBAT_STRIKE_DECLARATION" => CombatStrikeDeclaration,
            "COMBAT_ADDITIONAL_STRIKE" => CombatAdditionalStrike,
            "COMBAT_DAMAGE_PREVENTION" => CombatDamagePrevention,
            "COMBAT_GOING_TO_TORPOR" => CombatGoingToTorpor,
            "COMBAT_PRESS" => CombatPress,
            "COMBAT_END_OF_ROUND" => CombatEndOfRound,
            "COMBAT_WOULD_END" => CombatWouldEnd,
            "COMBAT_UNTIMED" => CombatUntimed,
            "COMBAT_BEFORE_RESOLUTION" => CombatBeforeResolution,
            "COMBAT_STRIKE_ANNOUNCED" => CombatStrikeAnnounced,
            "OUSTING" => Ousting,
            "READY_TITLED_LEFT" => ReadyTitledLeft,
            "ANY_IMPULSE" => AnyImpulse,
            _ => return None,
        })
    }
}

/// Who may use a mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Actor {
    /// The Methuselah, no minion (masters, events).
    Methuselah,
    /// The acting minion (action modifiers).
    ActingMinion,
    /// A ready unlocked minion of another Methuselah (reactions); locked
    /// too with a wake or the mode's `LockedActor`.
    ReactingMinion,
    /// A ready unlocked minion of the active Methuselah acting with this
    /// card (actions, equipment, allies, retainers).
    ActingCandidate,
    /// The host minion of a card in play (in-play abilities).
    Host,
    /// The controller of a card in play, no minion: the ability picks a
    /// `target` (a hunting ground's vampire).
    Controller,
    /// The prey of a card in play's controller, no minion (419 Operation).
    ControllersPrey,
    /// The player's minion in combat; ready, but in the end-of-round window
    /// a minion being burned or sent to torpor may still play.
    Combatant,
    /// An ability's host, in combat or not, torpor included, used for its
    /// controller's combatant, to which the effects apply (Opikun).
    ForOwnCombatant,
    /// The acting minion for its controller, a reacting minion for others
    /// (Scalpel Tongue, Ominous Chorus).
    ActingOrReactingMinion,
    /// Another ready minion of the acting Methuselah, locked or not (Cloak
    /// the Gathering [OBF], Veil the Legions; LSJ 19990425).
    OtherOwnMinion,
    /// A ready vampire out of the combat of another of its controller's
    /// minions, to which the effects apply (Touch of Valeren [FOR]).
    OwnOutOfCombat,
    /// The controller of the card's chosen vampire, in play (The Rack; ANK
    /// 20200130-2).
    ChosenController,
    /// A master played for a ready unlocked minion of its player, which
    /// meets its requirements and pays its costs (Piper).
    MasterForMinion,
    /// Any ready vampire of the user (Carver's Meat Packing and Storage).
    AnyReadyVampire,
    /// The minion in combat with the card's bearer, the card the target
    /// (Kaymakli Fragment).
    OpposingCombatant,
    /// An unlocked vampire out of the combat, any Methuselah's (Martyr's
    /// Resilience; ANK 20181101), in torpor too (LSJ 20020416).
    UnlockedOutOfCombat,
    /// The vampire whose text it is, or a card's host, out of the combat,
    /// acting on it: any Methuselah's combat (Randall, Maman Boumba,
    /// Abjure; rules-qa 98.2).
    HostOutOfCombat,
    /// A card's host in the combat or out of it, acting on it with the
    /// others' prevention: any Methuselah's combat (Draeven Softfoot).
    HostAnyCombat,
    /// The prey of the controller of the vampire whose text it is, the
    /// vampire the host (Keith Moody).
    HostControllersPrey,
    /// The predator of the controller of the vampire whose text it is, the
    /// vampire the host (Jacob, The Glitch).
    HostControllersPredator,
    /// A ready unlocked minion of another Methuselah than the controller of
    /// the vampire whose text it is, acting with it, the vampire the host
    /// (Calebros).
    OtherMethuselahsMinion,
    /// As `OtherMethuselahsMinion`, its own Methuselah's minions included
    /// (Calvin Cleaver).
    AnyMethuselahsMinion,
    /// A vampire of another Methuselah than the controller of the vampire
    /// whose text it is, in torpor too, using it outside an action of its
    /// own, the vampire the host (Mustafa).
    OtherMethuselahsVampire,
    /// Any Methuselah's vampire out of the current combat, locked or in
    /// torpor too (Bliss; ANK 20181101, LSJ 20020416).
    OutOfCombatVampire,
}

/// The city titles: prince, baron and archbishop (rules-qa 112.21).
pub const CITY_TITLES: &[TitleClass] = &[
    TitleClass::Prince,
    TitleClass::Baron,
    TitleClass::Archbishop,
];

/// A class of printed title, for title requirements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TitleClass {
    Any,
    Primogen,
    Prince,
    Baron,
    Justicar,
    InnerCircle,
    Archbishop,
    Bishop,
    Priscus,
    Cardinal,
    Regent,
    Magaji,
}

/// A mode requiring a title other than a Laibon's (Hiding in the Open's
/// bar): a title class other than any title or magaji.
pub fn requires_non_laibon_title(mode: &ModeDef) -> bool {
    mode.requires.iter().any(|r| {
        matches!(r, Requirement::ActorTitleIn(cs)
            if cs.iter().any(|c| !matches!(c, TitleClass::Any | TitleClass::Magaji)))
    })
}

impl TitleClass {
    pub fn matches(self, t: Title) -> bool {
        match (self, t) {
            (_, Title::None) => false,
            (TitleClass::Any, _) => true,
            (TitleClass::Primogen, Title::Primogen) => true,
            (TitleClass::Prince, Title::Prince(_)) => true,
            (TitleClass::Baron, Title::Baron(_)) => true,
            (TitleClass::Justicar, Title::Justicar(_)) => true,
            (TitleClass::InnerCircle, Title::InnerCircle(_)) => true,
            (TitleClass::Archbishop, Title::Archbishop(_)) => true,
            (TitleClass::Bishop, Title::Bishop) => true,
            (TitleClass::Priscus, Title::Priscus) => true,
            (TitleClass::Cardinal, Title::Cardinal) => true,
            (TitleClass::Regent, Title::Regent) => true,
            (TitleClass::Magaji, Title::Magaji) => true,
            _ => false,
        }
    }
}

/// The Methuselah a Hold is given to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Recipient {
    /// The target Methuselah (Cold Amber's Hold).
    Target,
    /// The acting minion's controller (Chiram's, Elder Michaelis's Hold).
    ActingController,
    /// The Methuselah whose turn it is (Regarhagan's Hold).
    Active,
}

/// An ammo card's effect on a gun for the rest of a combat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ammo {
    /// Once each round, striking with it, an optional additional strike
    /// (limited), only with it.
    CaselessRounds,
    /// +`close` damage at close range, -`long` at long, each strike
    /// (Scattershot).
    DamageByRange { close: u8, long: u8 },
}

/// What a minion may hold only one of (a vehicle, a Kevlar Vest).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Holding {
    Label(crate::library::Label),
    Named(&'static str),
    /// A weapon with a melee strike (Castiel: "does not have a melee
    /// weapon").
    MeleeWeapon,
}

/// An ally's permission to play cards "as a vampire".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AsVampire {
    /// The basic Discipline it meets (one).
    pub disciplines: &'static [&'static str],
    /// Its capacity for the cards it so plays (1 unless printed).
    pub capacity: u8,
    /// Only non-action cards (Spectral Servitor).
    pub non_action: bool,
    /// Only non-ally cards (Nocturn).
    pub non_ally: bool,
    /// The sect it plays them as: Independent (rulebook "Allies"), or the
    /// printed one ("as an Anarch vampire", Vivienne Géroux).
    pub sect: Sect,
    /// Only strike cards (Ghouled).
    pub strike_only: bool,
    /// Once each combat (Ghouled; `Minion::as_vampire_spent`).
    pub once_per_combat: bool,
    /// Only cards played as combat cards (Reanimated Corpse).
    pub combat_only: bool,
}

// A layer's hash: `combat_only` written only when set, the games' state
// hashes (the fuzz goldens) unchanged by the field (docs/hazards.md).
impl std::hash::Hash for AsVampire {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.disciplines.hash(h);
        self.capacity.hash(h);
        self.non_action.hash(h);
        self.non_ally.hash(h);
        self.sect.hash(h);
        self.strike_only.hash(h);
        self.once_per_combat.hash(h);
        if self.combat_only {
            true.hash(h);
        }
    }
}

/// The trait printed text names the bearer by ("this Ravnos"), read on the
/// current bearer: one without it gets nothing (rules-qa 9.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bearer {
    Any,
    Clan(&'static str),
    Sect(Sect),
    /// Lower case at any level, upper case superior only.
    Discipline(&'static str),
}

impl Bearer {
    pub fn is(self, st: &GameState, m: MinionId) -> bool {
        let v = &st.minions[m.index()];
        match self {
            Bearer::Any => true,
            Bearer::Clan(c) => v.clan() == Some(c),
            Bearer::Sect(s) => st.sect_of(m) == Some(s),
            Bearer::Discipline(d) => st.has_discipline(m, d),
        }
    }
}

/// The cards a cost or a requirement names ("a card requiring Dominate").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CardFilter {
    Any,
    Type(CardType),
    /// A library card whose printed disciplines name this one.
    Discipline(&'static str),
    /// A card of this type bearing this label ("a master archetype card").
    Labeled(CardType, crate::library::Label),
    /// An equipment bearing none of these labels, a locquipment not one
    /// (only a location once in play, spec §1.3.1).
    EquipmentWithout(&'static [crate::library::Label]),
    /// An equipment card bearing none of these labels, a locquipment
    /// included: read as played, for its cost (Marie Faucigny; LSJ
    /// 20080114, 20080109).
    EquipmentCardWithout(&'static [crate::library::Label]),
    /// A card requiring a Discipline, any one (Aniel).
    RequiresDiscipline,
    /// An ally or retainer card.
    AllyOrRetainer,
    /// A location (a locquipment in play included, spec §1.3.1).
    Location,
    /// A master location, not a locquipment (Danylo; LSJ 20080114).
    MasterLocation,
    /// A weapon with a melee strike, Weighted Walking Stick included.
    MeleeWeapon,
    /// A non-unique equipment card.
    NonUniqueEquipment,
    /// A card costing at most n pool (a blood or no cost: 0 pool).
    PoolCostAtMost(u8),
    /// Any card bearing this label (a ghoul ally or retainer).
    HasLabel(crate::library::Label),
    /// A card requiring this clan ("an ally requiring Hecata").
    Clan(&'static str),
    /// A card of this name ("a card named Golconda: Inner Peace").
    Named(&'static str),
    /// A card of one of these names.
    NamedAny(&'static [&'static str]),
    /// A minion card: any library card but a master or an event (rules-qa
    /// 60.3).
    MinionCard,
    /// An action card: an action, political action, ally, equipment or
    /// retainer card (rules-qa 60.4; ANK 20200813-2).
    ActionCard,
    /// Concealed Weapon's: a non-unique weapon whose regular strike (at
    /// strength 1, LSJ 20020821) is neither aggravated nor 4 or more; its
    /// cost is checked against the state (`equippable_from_hand`).
    ConcealableWeapon,
    /// A card that requires an imbued, a creed or a virtue: a power or a
    /// conviction, a card of a creed or a virtue (the crypt's imbued's),
    /// or one a mode of which requires an imbued (Unity).
    RequiresImbued,
    /// A card of this type printing this Discipline requirement (Academia
    /// de Campeões: combat cards requiring Potence).
    TypeRequiring(CardType, &'static str),
}

impl CardFilter {
    pub fn matches(self, card: u16) -> bool {
        let c = &LIBRARY[card as usize];
        match self {
            CardFilter::Any => true,
            CardFilter::Type(t) => c.types.contains(&t),
            CardFilter::Discipline(d) => c.disciplines.contains(&d),
            CardFilter::TypeRequiring(t, d) => c.types.contains(&t) && c.disciplines.contains(&d),
            CardFilter::Labeled(t, l) => c.types.contains(&t) && c.labels.contains(&l),
            CardFilter::EquipmentWithout(ls) => {
                c.types.contains(&CardType::Equipment)
                    && !c.labels.contains(&crate::library::Label::Location)
                    && !ls.iter().any(|l| c.labels.contains(l))
            }
            CardFilter::PoolCostAtMost(n) => match c.pool_cost {
                crate::library::Cost::Fixed(k) => k <= n,
                crate::library::Cost::None => true,
                crate::library::Cost::X => false,
            },
            CardFilter::EquipmentCardWithout(ls) => {
                c.types.contains(&CardType::Equipment) && !ls.iter().any(|l| c.labels.contains(l))
            }
            CardFilter::RequiresDiscipline => !c.disciplines.is_empty(),
            CardFilter::AllyOrRetainer => {
                c.types.contains(&CardType::Ally) || c.types.contains(&CardType::Retainer)
            }
            CardFilter::Location => c.labels.contains(&crate::library::Label::Location),
            CardFilter::MasterLocation => {
                c.types.contains(&CardType::Master)
                    && c.labels.contains(&crate::library::Label::Location)
            }
            CardFilter::MeleeWeapon => melee_strike(card).is_some(),
            CardFilter::RequiresImbued => requires_imbued(card),
            CardFilter::HasLabel(l) => c.labels.contains(&l),
            CardFilter::Clan(k) => c.clans.contains(&k),
            CardFilter::Named(n) => c.name == n,
            CardFilter::NamedAny(ns) => ns.contains(&c.name),
            CardFilter::MinionCard => !c
                .types
                .iter()
                .any(|t| matches!(t, CardType::Master | CardType::Event)),
            CardFilter::ActionCard => c.types.iter().any(|t| {
                matches!(
                    t,
                    CardType::Action
                        | CardType::PoliticalAction
                        | CardType::Ally
                        | CardType::Equipment
                        | CardType::Retainer
                )
            }),
            CardFilter::ConcealableWeapon => {
                c.labels.contains(&crate::library::Label::Weapon)
                    && !c.labels.contains(&crate::library::Label::Unique)
                    && regular_strike_damage(card)
                        .is_some_and(|(n, aggravated)| !aggravated && n < 4)
            }
            CardFilter::NonUniqueEquipment => {
                c.types.contains(&CardType::Equipment)
                    && !c.labels.contains(&crate::library::Label::Unique)
            }
        }
    }
}

/// A minion statistic a card in play modifies on its bearer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stat {
    /// Its damage is aggravated (Petra, the opponent read by the
    /// requirements).
    AggravatedDamage,
    /// Its hand strikes' damage is aggravated (Nizzam al-Latif).
    AggravatedHandStrikes,
    /// Optional presses in the first round of each combat (Shaggydog).
    FirstRoundPress,
    /// Frenzy cards cannot be played on it (Laecanus).
    FrenzyImmune,
    /// Damage its melee weapon strikes inflict more (Nuriel).
    MeleeDamage,
    /// Environmental damage each opposing minion takes each round at close
    /// range (Mambo Jeanne).
    EnvironmentalClose,
    /// Prisci ballots, cast with its votes, titled or not (Gratiano; LSJ
    /// 20040519).
    Ballots,
    /// Any positive total bars its block attempts (Monçada).
    CannotBlock,
    /// Blood gained after any political action's referendum it cast no
    /// vote or ballot in (Alvaro, The Scion of Angelica).
    BloodIfAbstained,
    /// Pool a directed action at it costs more, paid by the acting
    /// Methuselah as it resolves (Ambrosio Luis Monçada).
    DirectedAtCostsPool,
    /// Blood burned by a vampire pressing to continue combat with it (Petra).
    ContinuePressToll,
    /// It cannot press to end combat (Don Michael Antonio Giovanni).
    NoPressToEnd,
    /// It cannot use presses to continue combat (Mukhtar Bey).
    NoPressToContinue,
    /// The controller of an older vampire it diablerizes burns n pool
    /// (Sandra White).
    PoolBurnAfterOlderDiablerie,
    /// Sent to torpor or burned in combat, it burns, the opposing minion's
    /// controller gaining n pool (Hector Trelane).
    PoolToOpponentIfBurnedInCombat,
    /// It cannot act once any minion has acted this turn (Gharston Roland).
    NoActionAfterAnother,
    /// Contracts cannot be placed on it (Iliana).
    NoContractsOn,
    /// Minions opposing it in combat cannot use weapons (Count Germaine).
    OpponentsNoWeapons,
    /// It cannot commit diablerie (Ferox).
    CannotDiablerize,
    /// Its damage-dealing strikes inflict +n damage (Harry Reese, against
    /// the opponent its requirements read).
    StrikeDamage,
    /// It burns n blood when blocked, the blocker still ready at the end of
    /// the action (Spider).
    BurnsIfBlockerReady,
    /// It cannot rescue other vampires from torpor (Boss Callihan).
    CannotRescue,
    /// It cannot maneuver to long range (Pedro Cortez).
    NoManeuverToLong,
    /// While it is in play, any incapacitated imbued may leave the region
    /// for 2 conviction in any unlock phase (Jennie "Cassie247" Orne).
    GrantsImbuedRecovery,
    /// Other Methuselahs' actions targeting it cost n more blood or life,
    /// paid by the acting minion as they resolve (François "Warden" Loehr).
    TargetedCostsBlood,
    /// Other Methuselahs' actions directed at it cost n additional blood,
    /// as Chanjelin Ward's (`GameState::targeted_surcharge`): an ally
    /// never pays it (Étienne Fauberge; LSJ 20030815, rules-qa 73.5).
    DirectedCostsBlood,
    /// A card matching the requirements played on it by another than
    /// itself costs n less blood or pool (Guillermo Arsuaga's haven
    /// "played on" him; `GameState::played_on_cost_off`).
    CostOffPlayedOn,
    /// No equip action, no equipment taken (Enkidu, The Noah; LSJ
    /// 20050315), a location in play aside.
    NoEquipment,
    /// It unlocks at the end of the turn after a successful employ (Enkidu,
    /// The Noah).
    UnlockAfterEmploy,
    /// An extra draw on playing a card the requirements match (Ira Rivers).
    ExtraDrawOnPlay,
    /// After replacing such a card, its controller may draw 1 more (Duality).
    ExtraDrawOnReplace,
    /// After another Methuselah plays a master card, its controller may
    /// draw 1, discarding after (Sascha Vykos; ANK 20190606-2).
    DrawAfterOthersMaster,
    /// Each hand strike landing the round after one landed gives the
    /// opponent a pox counter (Smallpox Griet; PIB 20141210).
    PoxOnConsecutiveHandDamage,
    /// Once each combat, its damage landing gives the opponent 1 of its
    /// Methuselah's corruption counters (Elie Sansaricq).
    CorruptOnDamageOncePerCombat,
    /// Damage of its gun strikes at long range (Joaquin Murietta).
    GunDamageAtLong,
    /// Animal retainers lose their text while it acts or is in combat
    /// (Midget).
    AnimalRetainersLoseText,
    /// Blood it can gain after diablerizing an older vampire (Marta).
    BloodAfterOlderDiablerie,
    /// Burned once it has gone to torpor, its torpor riders fired (Julius).
    BurnedAfterTorpor,
    /// Of the bearer's bleeds.
    Bleed,
    /// Of the bearer's actions.
    Stealth,
    /// Of the bearer's block attempts.
    Intercept,
    /// Of the minion's strikes: added to its strength of 1 or to a strength
    /// an effect sets (spec §4.4.1).
    Strength,
    /// Votes the vampire can cast (only while ready).
    Votes,
    /// Votes once a Methuselah burned the Edge for a vote in the referendum
    /// (Tyler McGill).
    VotesAfterEdgeBurned,
    /// Damage the opposing minion inflicts is reduced to 0 (Saulot, The
    /// Wanderer, against Baali; `GameState::damage_reduction`).
    NoDamageFromOpponent,
    /// Its controller's hand size.
    HandSize,
    /// The damage of its ranged strikes, at either range (Noluthando).
    RangedStrikeDamage,
    /// The blood or life its ranged strikes steal, at either range (The
    /// Dracon).
    RangedStrikeSteal,
    /// Blood or life its stealing strikes steal more, at any range, the
    /// strike card read (`PrintedCtx::card`: Ardan Lane's [tha]).
    StrikeSteal,
    /// The combat cards it plays are not replaced until its controller's
    /// next unlock phase (Nedal, The Careless).
    CombatCardsHeldUntilUnlock,
    /// Its controller's discard phase actions, read as the discard phase
    /// begins (Sreelekha).
    DiscardPhaseActions,
    /// A press (mandatory) each combat, only usable to continue combat
    /// (Adrino Manauara).
    MandatoryContinuePress,
    /// Optional presses each combat (Serhat Gunde: "1 optional press each
    /// combat").
    CombatPress,
    /// Starting life of the zombies its controller recruits or employs
    /// (Ashur-uballit, while ready).
    ZombieStartingLife,
    /// Of its votes cast against a blood hunt referendum (Jason "Son"
    /// Newberry).
    VotesAgainstBloodHunt,
    /// Votes in favor of a blood hunt referendum called on it, counted as
    /// the referendum is called (Styles Margs).
    BloodHuntVotesInFavor,
    /// While it is ready, blocking a wraith or zombie ally costs the top
    /// library card (Parijat, the Dark Oracle).
    WraithZombieBlockToll,
    /// Any positive total bars strike: combat ends (Faruq Abd al-Qadir's
    /// corrupted opponents).
    NoCombatEnds,
    /// Each vampire casting votes or ballots against a referendum it calls
    /// burns this much blood once results are tallied (Hartmut Stover;
    /// rules-qa 49.2: not a cost).
    AgainstCastersBurnBlood,
    /// Each Methuselah voting against a referendum it calls, by a minion
    /// too, burns this much pool once each, after the tally (Marcos Belegrad).
    AgainstMethuselahsBurnPool,
    /// Its controller's transfers, read as the influence phase begins
    /// (Ingrid Rossler).
    Transfers,
    /// Allies cannot block it (Stanislava): any positive total bars them.
    AlliesCannotBlock,
    /// Vampires with capacity n or less cannot block it (Rexton: 3).
    SmallVampiresCannotBlock,
    /// Any positive total, read with the minion attempting to block as the
    /// opponent, bars its attempt (Aisling Sturbridge: younger Tremere).
    CannotBeBlockedBy,
    /// Votes it casts in a blood hunt referendum, either side (Karsh).
    VotesInBloodHunt,
    /// Votes its controller gets more for burning the Edge for a vote
    /// (Fiorenza Savona).
    EdgeVoteBonus,
    /// Votes its controller gets more for burning a political action card
    /// for a vote (Fiorenza Savona).
    BurnVoteBonus,
    /// The retainers of a minion opposing it in combat lose their text for
    /// that combat (Stanislava; rules-qa 49.3): any positive total.
    OpposingRetainersLoseText,
    /// Blood off its rescue from torpor, the rescued one its opponent, off
    /// the action's cost whoever pays it (Adelino; PIB 20110918).
    RescueDiscount,
    /// Blood or life a minion, its opponent, burns to attempt to block it
    /// (Sergio).
    BlockToll,
    /// Blood a vampire, its opponent, burns to vote against a referendum it
    /// calls (Alexander).
    VoteAgainstToll,
    /// Pool off a card it plays or puts in play, the card read by the
    /// printed predicates (Anne-Marie).
    PoolCostOff,
    /// Blood off such a card (Roger).
    BloodCostOff,
    /// Blood or pool off such a card, whichever it costs (Kuyén).
    CostOff,
    /// Its blood may pay up to n of the cost of a card its Methuselah
    /// plays, read by the printed predicates (Regina).
    BloodPaysCost,
    /// Any positive total bars directed actions (The Guardian).
    NoDirectedActions,
    /// Any positive total bars political actions (Bela).
    NoPoliticalActions,
    /// Intercept of a minion blocking it, read with the blocker as the
    /// opponent (Rutor: "Tzimisce get -1 intercept against Rutor";
    /// Gavrylo's allies and younger vampires).
    InterceptAgainst,
    /// Any positive total: no damage from retainers (Carolina Vález).
    NoRetainerDamage,
    /// Blood more it pays to keep contesting a title (Vasily).
    TitleContestToll,
    /// Any positive total bars its votes and ballots against a blood hunt
    /// (Troius).
    NoVotesAgainstBloodHunt,
    /// Any positive total bars its votes and ballots against the
    /// referendum, its requirements read with the caller (Samira).
    NoVotesAgainst,
    /// Blood its actions cost more, by kind and directedness (Daria,
    /// Marcellus): payable at announcement, paid first at resolution.
    ActionCost,
    /// Blood, or an ally's life, the opposing minion's strike cards cost
    /// more (Djeneba; `Engine::combat_card_surcharge`).
    StrikeCardToll,
    /// Blood, or an ally's life, each strike of the opposing minion costs
    /// more, but one aimed at a card on this one; it may choose not to
    /// strike (Lucian, the Perfect; `Engine::printed_strike_toll`).
    StrikesAgainstToll,
    /// Any positive total: it cannot hunt as normal, a card's hunt left to
    /// it (Kyoko Shinsegawa; `GameState::cannot_hunt`).
    CannotHunt,
    /// Once each turn, it may burn this much blood instead of locking for a
    /// successful block (Lucas Halton; `Interrupt::MayBurnForBlock`).
    MayBurnInsteadOfBlockLock,
    /// Any positive total: it does not lock to recruit an ally, and
    /// recruits one each turn at most (Marlena).
    RecruitsUnlockedOncePerTurn,
    /// Any positive total: its strikes with a gun are first strikes (Ruth
    /// McGinley).
    GunFirstStrike,
    /// Blood or pool its recruit ally actions of the card cost less, the
    /// action's only as Charisma's (Erichtho; LSJ 20090115-1).
    ActionRecruitCostOff,
    /// As it diablerizes, it unlocks and gains this much blood from the bank
    /// (Tyler).
    UnlockAndBloodOnDiablerie,
    /// Any positive total: at the end of its controller's discard phase,
    /// the cards discarded in it may go to the library, shuffled (Rachel
    /// Brandywine; `Interrupt::MayReturnDiscards`).
    DiscardsToLibrary,
    /// Any positive total makes the become-Anarch action free (Brunhilde).
    BecomeAnarchFree,
    /// Any positive total: its strikes resolve first (Elimelech).
    FirstStrike,
    /// Any positive total bars blocking or rushing the opponent (Reginald
    /// Moore).
    NoBlockOrRush,
    /// Blood a card it plays costs more, read by the printed predicates
    /// (Scout Youngwood).
    BloodCostMore,
    /// The controller of the minion in combat with this vampire plays with
    /// an open hand (Harold Tanner).
    OpponentOpenHandInCombat,
    /// Any positive total: Gehenna events do not affect it (Karen Suadela).
    GehennaImmune,
    /// Any positive total: plays cards as a justicar (Karl Schrekt).
    PlaysAsJusticar,
    /// Any positive total: a strike card played with Blood Sorcery that is
    /// not usable in the first round is (Al-Ashrad).
    FirstRoundSorceryStrikes,
    /// Any positive total, ready: its controller looks at the hand of a
    /// predator or prey who used a discard phase action (Count Jocalo).
    LooksAfterDiscardAction,
    /// Any positive total: plays political action cards from other
    /// Methuselahs' ash heaps as if from hand, removed from the game
    /// instead of burned (Vestgeir).
    PlaysPoliticalFromOthersAsh,
    /// Printed on the vampire calling a referendum, to `Opponent`: the
    /// minions it reaches cast no votes or ballots in it (Legba).
    CastsNoVotesInCallersReferendum,
    /// Any positive total: it and its Methuselah play cards requiring a
    /// Camarilla vampire as if it were one (Victor Gerard).
    PlaysAsCamarilla,
    /// Any positive total: it and its Methuselah play cards requiring a
    /// prince as if it were one (Victor Gerard, while a baron).
    PlaysAsPrince,
    /// Blood a card it plays costs more once every reduction leaves blood
    /// to pay: its player orders them so (Jenna Cross; ANK 20200709).
    BloodCostMoreIfAny,
    /// Pool off a master card its Methuselah plays, read by the printed
    /// predicates (Salvador Garcia).
    SeatCostOff,
    /// Any positive total: it cannot play the card in the mode read
    /// (Nehemiah: cards requiring Celerity; LSJ 20050322-2).
    CannotPlay,
    /// Any positive total: other Methuselahs cannot play master cards
    /// targeting it (Mimir).
    NoOthersMasterTargeting,
    /// Any positive total: its controller's vampires' votes are doubled as
    /// a referendum is tallied (Orlando Oriundus).
    DoublesControllersVotes,
    /// Any positive total: meets every sect and title requirement of the
    /// cards it plays or uses, and is of that sect for them (Vlad Tepes).
    MeetsSectAndTitleRequirements,
    /// Any positive total: no blood hunt on it (Theo Bell G2).
    NoBloodHunt,
    /// Any positive total: a blood hunt passed on it sends it to torpor
    /// instead of burning it (Genina).
    TorporInsteadOfBloodHuntBurn,
    /// Blood more a rescue of it from torpor costs, read on the rescued
    /// vampire (Mitchell, The Headhunter).
    RescueCostMore,
    /// Any positive total: a vampire blocking it does not lock for the
    /// block, the blocker its opponent (Reverend Adams: older ones).
    BlockerDoesNotLock,
    /// Any positive total: its title, as it would be contested, is yielded
    /// at once instead (Victor Pelletier).
    YieldsTitleInsteadOfContest,
    /// Any positive total: once each turn, it may take 1 blood or pool off a
    /// card its Methuselah plays, announced with it (Baldesar Rossellini:
    /// a location or an equipment, his requirements).
    SeatCostOffOncePerTurn,
    /// As `SeatCostOffOncePerTurn`, pool only, a card of its Methuselah's
    /// discarded at random for it (Kali).
    SeatPoolOffForRandomDiscard,
    /// Any positive total: it cannot have or use retainers (Angelo).
    NoRetainers,
    /// Any positive total: Blood Brothers play cards requiring Sanguinus as
    /// if it were of any circle (Angelo; `GameState::same_circle_for_play`).
    AnyCircleForSanguinus,
    /// Of its hunts, non-standard ones too (Ariadne; RTR 20030519).
    Hunt,
    /// It cannot play action cards (The Beast): any positive total.
    NoActionCards,
    /// Burned, it goes to the uncontrolled region for +n transfers next
    /// influence phase (The Capuchin).
    ToUncontrolledWhenBurned,
    /// Any positive total: never contested (Jimmy Dunn).
    NeverContested,
    /// It cannot have or use equipment or retainers (The Beast).
    NoEquipmentOrRetainers,
    /// Blood off one card it plays each turn, chosen at play
    /// (`PlayCtx::once_off`, Nergal).
    BloodCostOffOncePerTurn,
    /// Lock it for pool off a master location, announced with the card, in
    /// torpor too (Jack; LSJ 20011022).
    LockForLocationDiscount,
    /// Its opponent cannot dodge in the first round (Gabi).
    OpposingNoDodgeFirstRound,
    /// A canceled card it plays may return to the library
    /// (`PrintedCtx::card`, Victor Donaldson).
    RecyclesCanceled,
    /// Any positive total: two master phase actions, read live (Anson;
    /// `GameState::mpa_left`).
    TwoMasterPhaseActions,
    /// n more master phase actions, its requirements read as the master
    /// phase starts (Cybele: "if unlocked at the start").
    MasterPhaseActionAtStart,
    /// n pool from the bank as its controller's turn begins, its
    /// requirements read then, before any unlock (Bartholomew: "if
    /// unlocked at the beginning of your turn").
    PoolAtTurnStart,
    /// Master cards targeting it cost its controller n more pool (Walker
    /// Grimes; `GameState::master_surcharge_on`).
    MasterTargetingCostsYou,
    /// Acting, its controller's played cards are replaced at the end of the
    /// action (Skidmark).
    HoldsReplacementsWhileActing,
    /// Present and unlocked, its controller's cards played during their
    /// turn are replaced as it locks or the turn ends (Hagar Stone).
    HoldsReplacementsWhileUnlocked,
    /// Equipping from its controller's hand, it may look at their prey's
    /// hand and take an equipment instead of the one it played (Topaz).
    SwapsEquipmentFromPrey,
    /// A younger vampire its controller puts in play from the uncontrolled
    /// region may burn a blood to become Anarch (Jack Drake).
    YoungerEntrantsMayTurnAnarch,
    /// Burned, not sent to torpor, once more than n damage landed on it in
    /// one combat (Franciscus).
    BurnedOverCombatDamage,
    /// After combat with it, the opposing minion burns n blood or life
    /// (Amelia, The Blood Red Tears).
    AfterCombatOpponentBurns,
    /// Blocked, the blocking vampire burns n blood after the combat (Jason,
    /// The World's Voice).
    BlockerBurnsAfterBlockedCombat,
    /// A younger vampire failing to block it is locked as the action ends
    /// (Gracis Nostinus).
    LockFailedYoungerBlockers,
    /// Its controller gets n votes in a blood hunt referendum on it (Ilyana
    /// Ravidovich).
    OwnerVotesInOwnBloodHunt,
    /// Once each turn, a successful action of it requiring Animalism: it
    /// unlocks as the action ends (Cock Robin).
    UnlockAfterAniAction,
    /// Any positive total: its title rivals cannot pay to contest (Rafael).
    RivalsYieldTitle,
    /// Any positive total: it may block any Methuselah's vampire (Anneke).
    BlocksAnyVampire,
    /// Any positive total: one redraw each turn (Blackhorse Tanner;
    /// `Interrupt::Redraw`).
    RedrawOncePerTurn,
    /// One optional maneuver or press in the first round of each combat,
    /// used either way (Hanna Redmonds, to an older opponent).
    FirstRoundManeuverOrPress,
    /// Capacity +n while in play (Jacob, The Glitch: "while he is
    /// controlled"; `GameState::capacity_of`).
    Capacity,
    /// Blood the opposing vampire burns to announce a directed action at
    /// it or a card on it, unless canceled as played (Hazimel;
    /// `Engine::announce_toll`).
    DirectedAtAnnounceBlood,
    /// Any positive total: the vampire burns, checked as each step settles,
    /// read live (Al-Muntathir, Obaluaye; `Engine::settle_printed_burns`).
    BurnsWhile,
    /// Any positive total: it performs no action, leaving torpor included
    /// (Toy; `GameState::only_actions`).
    CannotAct,
    /// Any positive total: n of the blood its hunt would gain may go to
    /// another vampire its controller controls instead (Azrael;
    /// `Engine::hunt_gain`).
    HuntBloodElsewhere,
    /// n: the burn counters that remove it from the game. Any positive
    /// total: a burn of it, while controlled, puts 1 burn counter on it
    /// and moves it to torpor instead, a replacement (Byzar;
    /// `Engine::replace_burn`).
    TorporInsteadOfBurn,
    /// Any positive total: no blood hunt is called for its diablerie, read
    /// as the diablerie begins (Harlan Graves; `ActionCtx::no_blood_hunt`).
    NoBloodHuntIfDiablerized,
    /// Any positive total: after an action card it plays, a library copy may
    /// go to the ash heap, back to hand if the action succeeds (Lord Ephraim
    /// Wainwright; `Interrupt::MayMillCopy`).
    MillsPlayedActionCopy,
    /// Any positive total: its controller looks at the hand of another
    /// Methuselah who played a master card, after its replacement
    /// (Troglodytia; `Engine::others_master_draws`; ANK 20180512).
    LooksAfterMaster,
    /// Any positive total, ready: its controller sees the replacements other
    /// Methuselahs draw during a referendum (Slag; `Engine::replace`).
    ShowsReferendumReplacements,
    /// Any positive total: its only strikes are dodges and combat ends
    /// (Tanginé, against a Salubri; `GameState::strikes_only_dodge_or_ends`).
    StrikesOnlyDodgeOrCombatEnds,
}

/// Legality predicates, the closed vocabulary, evaluated over the state and
/// context; a target predicate reads the candidate target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Requirement {
    /// On a rider: one copy per controller resolves it each turn (Army of
    /// Rats).
    OneCopyPerTurn,
    /// The acting minion, allies included, is not of this clan.
    ActorNotClan(&'static str),
    /// The actor is not Black Hand (Mateusz Kazimirski).
    ActorNotBlackHand,
    /// The Methuselah the action is at controls a ready ally (Preston
    /// Varrick).
    TargetSeatControlsReadyAlly,
    /// The Methuselah the action is at controls a ready minion (Fairuza).
    TargetSeatControlsReadyMinion,
    /// The Methuselah the action is at controls no ready vampire above this
    /// capacity (Zubeida).
    TargetSeatNoReadyCapacityAbove(u8),
    /// The printed vampire is in torpor (Lin Jun).
    SourceInTorpor,
    /// The modified minion is controlled by the printed vampire's
    /// Methuselah's predator (Lin Jun).
    ActorOfSourcesPredator,
    /// Printed: the modified minion is controlled by the printed vampire's
    /// Methuselah's prey or predator (Barth).
    ActorOfSourcesPreyOrPredator,
    /// Printed: the modified minion is a vampire of capacity n or less
    /// (Barth).
    ActorCapacityAtMost(u8),
    /// Printed: the opponent holds a title of one of these classes (Ellison
    /// Humboldt: princes).
    OpponentTitleIn(&'static [TitleClass]),
    /// The modified minion has this Discipline (Lin Jun).
    ActorHasDiscipline(&'static str),
    /// The actor is a vampire who can commit diablerie (Cloak of Blood,
    /// Amaranth).
    ActorCanDiablerize,
    /// The actor's initial strike of the round its [VIC] named (Liquefy the
    /// Mortal Coil).
    DiablerieStrikeRound,
    /// The minion opposing the actor is younger (Liquefy the Mortal Coil).
    CombatOpponentYounger,
    /// The actor is a crypt vampire, unique, no created one (Unleash the
    /// Hounds [VIS]).
    ActorCryptVampire,
    /// The actor just diablerized a vampire of capacity n or more (Blade
    /// Clot).
    DiablerizedCapacityAtLeast(u8),
    /// The actor is a base vampire, unmerged (Epiphany).
    ActorUnmergedBase,
    /// The action targets a minion of this clan its player controls (Joseph
    /// Pander).
    DirectedAtOwnClan(&'static str),
    /// An ally is acting (Final Loosening [for]).
    ActingIsAlly,
    /// Its player controls another unlocked ready Anarch (Power of All).
    ControlsOtherUnlockedAnarch,
    /// The actor's hand strike just inflicted damage (Shoulder Drop).
    HandStrikeInflicted,
    /// Candidate: its action succeeded or fizzled this turn (Mustajib).
    TargetActedSuccessfully,
    /// Uncontrolled target: no card of this name on it (Cadet).
    SlotWithout(&'static str),
    /// The acting minion is Black Hand (Seraph's Second).
    ActingBlackHand,
    /// The acting minion is not Black Hand (Truth in Ink).
    ActingNotBlackHand,
    /// The acting minion is no Seraph (Seraph's Second).
    ActingNotSeraph,
    /// The minion attempting to block is Black Hand (Census Taker).
    AttemptingBlackHand,
    /// The ability's host is neither the acting minion nor the one
    /// attempting to block ("any other", Census Taker).
    HostNotInvolved,
    /// The actor (in a target filter, the candidate) is Black Hand
    /// (Recruitment Exercise, Shakar: the Hunt, Karif al Numair).
    ActorBlackHand,
    /// Its player controls a ready Black Hand vampire (a master card's
    /// "Requires a Black Hand vampire": Shakar: the Hunt).
    ControlsReadyBlackHand,
    /// Its player controls a ready Seraph (Watchtower: Four Ride Forth).
    ControlsReadySeraph,
    /// The minion attempting to block has at least this capacity (Kumpania).
    AttemptingCapacityAtLeast(u8),
    /// The minion attempting to block is an ally or a vampire younger than
    /// the acting one (Crocodile's Tongue).
    AttemptingAllyOrYounger,
    /// The minion attempting to block is a vampire younger than the acting
    /// one (Mélusine d'Anjou).
    AttemptingYoungerVampire,
    /// The target vampire has at least this capacity (Golconda).
    TargetCapacityAtLeast(u8),
    /// The target vampire has at most this capacity (Scourge of the
    /// Enochians).
    TargetCapacityAtMost(u8),
    /// The target is of the actor's clan (Grooming the Protégé).
    SameClanAsActor,
    /// The actor and the target are both of the sect (My Kin Against the
    /// World's Laibon).
    ActorAndTargetSect(Sect),
    /// The user has a minion in the current combat (Mob Connections).
    OwnMinionInCombat,
    /// Some minion attempts to block and needs intercept (WMRH Talk Radio).
    AnyAttemptNeedsIntercept,
    /// The host was in the combat that just ended and its opponent is not
    /// ready (Monster).
    HostCombatOpponentNotReady,
    /// As above, the opponent a vampire (Valerie Diaz).
    HostCombatOpposingVampireNotReady,
    /// Normal damage from the opposing strike is pending on the actor's
    /// side (Valerie Diaz, Opikun).
    StrikeNormalDamagePending,
    /// Normal damage from other sources than the opposing strike is pending
    /// on the actor's side (Valerie Diaz).
    EffectNormalDamagePending,
    /// The actor is an ally whose card matches (Mobile HQ).
    ActorAllyCard(CardFilter),
    /// The target minion is an ally whose card matches.
    TargetAllyCard(CardFilter),
    /// The action card played would target the player or theirs (Esprit de
    /// Corps; rules-qa 38.6, 77.12).
    PlayedActionCardTargetsYou,
    /// The card played is an action card at a minion or card of the user's
    /// with this label (Mobile HQ).
    PlayedActionCardAtYour(crate::library::Label),
    /// The host used at least n Disciplines to play cards during the action
    /// (Dabbler).
    HostUsedDisciplines(u8),
    /// The action's card, or granting card, requires this clan or
    /// Discipline (Cappadocian Crypt); an empty Discipline: the clan alone
    /// (Keegan).
    ActionRequiresClanOrDiscipline(&'static str, &'static str),
    /// The action's card requires this Discipline in the mode played
    /// (Amenophobis: "any successful action requiring Serpentis").
    ActionRequiresDiscipline(&'static str),
    /// The Methuselah the action is directed at has the Edge (Amenophobis).
    TargetSeatHasEdge,
    /// The target has this Discipline (Powerbase: Munich).
    TargetDiscipline(&'static str),
    /// The player controls no vampire of this name, and has one in their
    /// crypt (The Mithraeum, London).
    NoVampireNamedButInCrypt(&'static str),
    /// The target is /Mithras/ or a Mithraic Mysteries member.
    TargetMithraic,
    /// The target is not a vampire of this name (Mithraic Cultist).
    TargetNotNamed(&'static str),
    /// The target is Anarch.
    TargetAnarch,
    /// The directed action targets a ready vampire of this clan the player
    /// controls (Yoruba Shrine).
    DirectedAtYourReadyClan(&'static str),
    /// The directed action targets a vampire of this sect the player
    /// controls, not the actor (Familial Bond: another Laibon).
    DirectedAtYourOtherOfSect(Sect),
    /// Printed: the directed action targets a vampire of this clan the
    /// vampire's controller controls, in torpor too (Delizbieta of the Dark
    /// Eyes).
    DirectedAtYourClan(&'static str),
    /// The referendum's terms chose a ready vampire of this clan the player
    /// controls (Yoruba Shrine).
    TermsChoseYourReadyClan(&'static str),
    /// A plain equip from hand, the acting minion not yet unlocked this way
    /// this turn (British Museum).
    PlainEquipFromHandOncePerTurn,
    /// The player's own action card costs blood (The Line: by a vampire),
    /// or blood or pool (The Shard), or is an equip (Ravnos Cache).
    OwnActionCardCosts(CostSource),
    /// The action under way is the player's, from an action card not put in
    /// play: bound for their ash heap (Epikasta Rigatos).
    OwnActionCardToAsh,
    /// On an ability: fewer than n of the controller's copies used it this
    /// phase (Split the Veil).
    FewerCopiesUsedThisPhase(u8),
    /// No action other than a mandatory one has been performed this turn
    /// (Khabar: Glory).
    NoOptionalActionThisTurn,
    /// An ally is in some Methuselah's ash heap (Psychophagia: ANK
    /// 20200420-3).
    AllyInAnyAsh,
    /// The player's ash heap holds an ally matching one of these (Hel-Blá).
    AshHasAlly(&'static [CardFilter]),
    /// On an unlock-phase rider: the active Methuselah controls a vampire
    /// not of this sect, read as it resolves (Judgment: Camarilla
    /// Segregation; ANK 20181016).
    ActiveControlsNonSect(Sect),
    /// The target minion is not of this sect.
    TargetSectNot(Sect),
    /// The target is of this sect (Praetorian Backer).
    TargetSect(Sect),
    /// Minion target: a Red List minion (Big Game).
    TargetRedList,
    /// The target minion is not Black Hand (Shakar).
    TargetNotBlackHand,
    /// The actor's strike this round is with a gun it bears on which no
    /// ammo card was used this combat (Caseless Rounds; LSJ 20020425).
    StrikingWithUnloadedGun,
    /// The combatant using it strikes this round with the gun this card is
    /// on, no ammo used on it this combat (Magazine).
    StrikingWithItsGun,
    /// The Methuselah a Hold goes to controls no copy of it ("only 1") and
    /// has a card of the type it takes in their ash heap (D14, rules-qa
    /// 60.17).
    HoldRecipient(Recipient),
    /// Its controller's hand holds a copy of the card on this one (the
    /// Holds' discard).
    HandHasStoredCopy,
    /// The target minion is Black Hand (Black Hand Ritual).
    TargetBlackHand,
    /// The target vampire has not merged (Gift of Experience).
    TargetUnmerged,
    /// The opposing minion lacks this Discipline, read live (Waxen
    /// Poetica: "not usable against a vampire with Celerity").
    OpponentLacksDiscipline(&'static str),
    /// Minion target: not a Red List minion.
    TargetNotRedList,
    /// Minion target: not the action's blocker (The Status Perfectus).
    TargetNotBlocker,
    /// The target's controller is the action's target, its directed
    /// Methuselahs, or for an undirected one the actor's predator or prey
    /// (No Confidence).
    TargetAtActionsEnd,
    /// The vampire cast votes in the latest referendum, since the player's
    /// last turn (Tribunal Judgment).
    TargetCastVotes,
    /// Minion target: not the action's acting minion (Kateline Nadasdy's
    /// "non-acting vampire").
    TargetNotActing,
    /// Minion target: not the minion attempting to block (Palavra Sagrada).
    TargetNotAttempting,
    /// The action's blocker is an Anarch the player controls (The Status
    /// Perfectus).
    BlockerOwnAnarch,
    /// The action's blocker is a minion the player controls (Bar Último
    /// Trago).
    BlockerYours,
    /// The blocker is the player's, and the combat just ended was between
    /// it and the acting minion (Fast Reaction: not after the combat it
    /// queued).
    BlockerOwn,
    /// The opposing vampire's pending damage, as it stands, sends it to
    /// torpor (Decapitate [POT]).
    OpponentGoingToTorpor,
    /// The actor's own pending damage, as it stands, sends it to torpor
    /// (Undead Persistence [for]).
    ActorGoingToTorpor,
    /// The actor is not being burned or going to torpor by the damage
    /// pending on it (Decapitate [POT]).
    ActorNotGoingDown,
    /// The actor is not the blocker (Fast Reaction: "other than this
    /// vampire").
    ActorNotBlocker,
    /// The combat just ended was the blocker's with the acting minion, the
    /// blocker still ready (Hidden Lurker; LSJ 20100604-1).
    BlockerReadyAfterItsCombat,
    /// Target minion is controlled by the player.
    Controlled,
    /// Target minion has at least this much blood.
    BloodAtLeast(u8),
    /// Target seat is not the acting minion's controller (bleed redirects).
    NotActingController,
    /// Target seat is not the player.
    NotSelf,
    /// Target seat is not the player's prey: a bleed at the prey is the
    /// plain bleed already offered (Julia Prima's "any Methuselah").
    NotPrey,
    /// The card being played (as-played context) is a master card.
    PlayedCardIsMaster,
    /// The card being played was played by another Methuselah.
    PlayedByOtherSeat,
    /// Playable only on another Methuselah's turn (out-of-turn masters).
    NotOwnTurn,
    /// The current action is directed at the player.
    DirectedAtYou,
    /// The acting minion's controller is not the player's predator (Two
    /// Wrongs).
    ActingNotYourPredator,
    /// The current action is a bleed.
    IsBleed,
    /// The current action is a bleed against the acting Methuselah's prey
    /// (Enkil Cog): read live, a redirected bleed is not (rules-qa 50.3);
    /// a prey this bleed ousted still is (rules-qa 61.2).
    BleedAgainstPrey,
    /// The current bleed resolved for 1 pool or more (Ilonka; rulebook
    /// "Bleed").
    BleedSuccessful,
    /// The current bleed amount is at least this.
    BleedAmountAtLeast(u8),
    /// Blocks have been declined by all Methuselahs (state C).
    BlocksDeclined,
    /// Blocks were declined, the action unblocked since: from then until
    /// after its resolution (Ophidian Gaze [PRE][SER]: Voter Captivation,
    /// Freak Drive; ANK 20180909).
    AfterBlocksDeclined,
    /// Stealth is needed: a block attempt is pending with intercept at
    /// least equal to stealth (rules-qa 3.2). Waived by `waives_needed`.
    StealthNeeded,
    /// Intercept is needed: the actor is the attempting minion and stealth
    /// exceeds its intercept (rules-qa 3.2).
    InterceptNeeded,
    /// The actor minion is locked (wakes: "only usable by a locked vampire").
    LockedActor,
    /// The host of this card in play is below its capacity.
    HostBelowCapacity,
    /// Target vampire has not gained blood from a hunting ground this turn.
    NotHuntingGroundFed,
    /// The player controls a ready vampire with a title of these classes
    /// (Papillon).
    ControlsReadyTitled(&'static [TitleClass]),
    /// The player controls a ready vampire of these sects (Carfax Abbey).
    ControlsReadySect(&'static [Sect]),
    /// The player controls at least n ready vampires of this sect (Communal
    /// Haven: Cathedral's "between any two").
    ControlsReadyOfSect(Sect, u8),
    /// The host's controller has at least n pool.
    HostControllerPoolAtLeast(u8),
    /// The host of the card in play is locked.
    HostLocked,
    /// The host of the card in play has at least this much blood.
    HostBloodAtLeast(u8),
    /// The host holds at least n unlocked cards of this kind (Aye: "lock
    /// three of his Aye").
    HostHoldsUnlocked(Holding, u8),
    /// The host ally, its own card, is below its starting life (The Vozhd
    /// of Juiz de Fora).
    HostBelowStartingLife,
    /// The host has at most this much blood or life (Double Deuce).
    HostBloodAtMost(u8),
    /// The host of the card in play is in torpor (Fame's unlock rider;
    /// a host that left play with its Methuselah does not qualify).
    HostInTorpor,
    /// The host took no hunt action this turn (César Holfield).
    HostDidNotHunt,
    /// Its host announced no action this turn (Tori Longwood).
    HostAnnouncedNoAction,
    /// Card predicate: the vampire it is on is out of play (Descent into
    /// Darkness: never an ally, LSJ 20060124).
    HostOutOfPlay,
    /// Engine-checked at a successful bleed's printed riders: the bled
    /// Methuselah controls a ready titled vampire (Lubomira Hradok).
    BledControlsReadyTitled,
    /// The host's controller is the active Methuselah (Fear of Mekhet's
    /// "this vampire's unlock phase").
    HostControllerActive,
    /// The card's controller is the active Methuselah, wherever it moved
    /// (Houngan's "during your unlock phase").
    CardControllerActive,
    /// The host's controller put no Gehenna card in play this discard phase
    /// (Afifa, The Herald; `Seat::gehenna_put`).
    HostControllerPutNoGehenna,
    /// The host's controller holds the Edge (Lucina's unlock rider; an
    /// ability's own `HasEdge` reads its user's).
    HostControllerHasEdge,
    /// The player has a master phase action left (Tension in the Ranks).
    MasterPhaseActionLeft,
    /// The player holds at least n cards matching the filter.
    HandHas(CardFilter, u8),
    /// The actor is a vampire, not an ally.
    ActorVampire,
    /// The actor has a Discipline at superior to choose (Field Training:
    /// ANK 20180731-2).
    ActorHasSuperior,
    /// The actor is an imbued (a power: rulebook "Appendix: Imbued Rules"
    /// 3).
    ActorImbued,
    /// This card in play carries at least n counters (a rider with
    /// nothing to pay is not pending: Smiling Jack at 0 counters).
    CountersAtLeast(u8),
    /// This card has n counters, the one its pending bleed gain adds
    /// included (Forward Momentum, `ActionCtx::momentum`).
    CountersWithBleedGainAtLeast(u8),
    /// This card has at most n counters (Powerbase: Madrid: "if this card
    /// has 3 or fewer counters").
    CountersAtMost(u8),
    /// Printed condition: its controller controls a ready vampire of these
    /// titles (Damian).
    ControlsReadyTitledOther(&'static [TitleClass]),
    /// Printed: another ready vampire, any Methuselah's, holds one of these
    /// titles (Anastasz di Zagreb).
    OtherReadyTitled(&'static [TitleClass]),
    /// Printed: the vampire holds no card of this kind (Arriette Sylla: no
    /// Orun).
    SourceHoldsNone(Holding),
    /// Printed condition: its controller's predator controls more ready
    /// minions than they do (Khin Aye).
    PredatorMoreReadyMinions,
    /// Printed condition: its controller's prey has n pool or fewer
    /// (Üresség).
    PreyPoolAtMost(u8),
    /// The source's controller's prey controls a vampire not of this sect,
    /// in torpor too (Molly MacDonald: "a non-Camarilla vampire").
    PreyControlsVampireNotOfSect(Sect),
    /// The acting Methuselah's prey burned pool during this action
    /// (Věnceslava).
    PreyBurnedPool,
    /// The user controls at least n ready vampires following the path
    /// (Privileged Position).
    ControlsReadyOnPath(&'static str, u8),
    /// Printed condition: the action's target Methuselah controls a minion
    /// of these clans (Osvaldo Kühnemann).
    TargetSeatControlsClan(&'static [&'static str]),
    /// The minion target follows the path (Terrifying Visage).
    TargetFollowsPath(&'static str),
    /// The acting minion follows the path (Forward Momentum).
    ActingFollowsPath(&'static str),
    /// No combat is queued in this action (advanced rules 4.9.3: one at a
    /// time).
    NoQueuedCombat,
    /// The acting minion is an ally or a vampire younger than the actor
    /// (Confusion of the Eye [obf]).
    ActingAllyOrYounger,
    /// The current action is a hunt, a card's included (Kalyani; LSJ
    /// 20050720-2, rules-qa 72.1).
    IsHunt,
    /// The current action is the rulebook hunt (Zöe; rules-qa 72.2).
    RulebookHunt,
    /// Printed-modifier predicate: the action is a diablerie (Lisette
    /// Vizquel).
    IsDiablerie,
    /// The actor is not the acting minion (Yawp Court's Sabbat vampire).
    ActorNotActing,
    /// The acting minion is the user's ally or a vampire younger than the
    /// host (Abderrahim).
    OwnActingAllyOrYounger,
    /// The current action is a recruit or an employ (Frau Schädel).
    IsRecruitOrEmploy,
    /// The current action employs a retainer (Rosa Hernández).
    IsEmploy,
    /// An action to equip with a weapon card from hand (Adhiambo).
    IsEquipWeaponFromHand,
    /// An action to enter combat with a minion (Adhiambo).
    ActionEntersCombat,
    /// An action whose card puts a vampire in play (João Bilé: The Embrace).
    ActionPutsVampireInPlay,
    /// The opposing combatant is an ally (Entombment [obt], Carolina Vález).
    OpponentAlly,
    /// The opposing minion is an ally not a mortal (Harry Reese).
    OpponentNonMortalAlly,
    /// The player lacks the Edge ("unless you have the Edge").
    NoEdge,
    /// The action's card, or granting card, has this path icon (Sakura).
    ActionRequiresPath(&'static str),
    /// The acting minion is an ally of these subtypes (Gifts From
    /// Hereafter).
    ActingAllyOf(&'static [crate::library::Label]),
    /// The acting minion holds no card of this kind (Pallid: no Aye).
    ActingHoldsNone(Holding),
    /// The acting minion holds a card of this kind (Car Bomb: a vehicle).
    ActingHolds(Holding),
    /// The minion target is an ally of these subtypes (Fiorella, Burial
    /// Site Hunting Ground).
    TargetAllyOf(&'static [crate::library::Label]),
    /// The minion target is neither a vampire nor an ally of these subtypes
    /// (Heartrender); a retainer target is read by the card filter.
    TargetAllyNotOf(&'static [crate::library::Label]),
    /// The minion target is another copy of the actor (Gravebound Drone).
    TargetOtherCopyControlled,
    /// The acting minion is a vampire younger than the actor, which is
    /// about to enter combat with it as its blocker (Obedience).
    BlockingYoungerActing,
    /// The actor is about to enter combat with the acting minion, a
    /// younger vampire: by the action's resolution, blocks declined, or in
    /// the combat Psyche! queued (Obedience; rules-qa 55.7, 56.2, 88.10).
    FoughtByYoungerActing,
    /// The own minion in combat is another copy of the host (Gravebound
    /// Drone's prevention).
    OwnCombatantOtherCopy,
    /// The actor is an ally of one of these subtypes (Dance of the Dead).
    ActorAllyOf(&'static [crate::library::Label]),
    /// The minion attempting to block is another ally of the user's of one
    /// of these subtypes (Fiorella).
    AttemptingAllyOf(&'static [crate::library::Label]),
    /// The actor has chosen its strike, with no aim on it yet: one aim each
    /// strike (Target Vitals; rules-qa 62.1).
    NoAimYet,
    /// This card in play is unlocked ("you can lock this card to …").
    CardUnlocked,
    /// The player has the Edge ("if you have the Edge").
    HasEdge,
    /// The player's hand is not empty.
    HandNotEmpty,
    /// The user's crypt is not empty (Family Gathering, Wider View: RTR
    /// 20000501, LSJ 20070411-2).
    CryptNotEmpty,
    /// The target vampire is in torpor.
    TargetInTorpor,
    /// The target imbued is incapacitated (Bond).
    TargetIncapacitated,
    /// The target imbued holds n conviction cards or more (The Rose
    /// Foundation).
    TargetConvictionAtLeast(u8),
    /// The acting imbued holds n conviction cards or more (Expiate).
    ActorConvictionAtLeast(u8),
    /// The target vampire has hostage counters.
    TargetHostage,
    /// This card was not played before in this game.
    NotPlayedThisGame,
    /// "A Methuselah can play only 1 <card> in a game" (Retribution): not
    /// played by its player before, canceled included.
    NotPlayedThisGameBySeat,
    /// Not played this turn by anyone, a canceled play counting (Ashur
    /// Tablets).
    NotPlayedThisTurn,
    /// n or more other Gehenna events in play, checked at play only (PIB
    /// 20121031).
    GehennaInPlay(u8),
    /// At least n Gehenna cards other Methuselahs control in play
    /// (Becoming of Ennoia; checked as played, PIB 20121031).
    GehennaOfOthersInPlay(u8),
    /// No Gehenna card in play (Claudio Severino's unlock-phase toll).
    NoGehennaInPlay,
    /// Not while its player controls a copy (Raising the Portcullis; LSJ
    /// 20100129).
    OnePerMethuselah,
    /// The card being played has one of these labels and is played by the
    /// minion opposing this card's bearer (Sword of the Archangel: grapple
    /// or aim; Joumlon's Axe: grapple): engine-checked.
    PlayedLabeledByOpponent(&'static [crate::library::Label]),
    /// The card being played is played as a combat card by the minion
    /// opposing the user (Asguresh): engine-checked.
    PlayedCombatByOpponent,
    /// This weapon's strike resolution burned the opposing vampire this
    /// combat (Sword of the Archangel): engine-checked.
    WeaponBurnedOpponent,
    /// The card being played is a frenzy card played on a vampire of the
    /// clan the user controls (Meditative Grove): engine-checked.
    PlayedFrenzyOnYour(&'static str),
    /// The card being played is a frenzy card played on this card's host
    /// (Aye; LSJ 20051113): engine-checked.
    PlayedFrenzyOnHost,
    /// The card being played is a frenzy card played on the actor
    /// (Predator's Transformation [abo] reflex; LSJ 20051113): engine-checked.
    PlayedFrenzyOnActor,
    /// The actor burned a minion, any Methuselah's, in this combat or by
    /// this action, a diablerie's only before its blood hunt (Taking the
    /// Skin: Minion, RTR 19991206).
    ActorBurnedMinion,
    /// The actor burned a minion of its controller's prey, in this combat
    /// or by this directed action (Predator's Transformation [ABO]; ANK
    /// 20220507: a directed action that ends up burning an ally counts).
    ActorBurnedPreyMinion,
    /// The target is the minion this card is on (Haven Uncovered):
    /// engine-checked.
    TargetIsHost,
    /// The target vampire was chosen this turn by the ability's vampire
    /// (Lucinde, Alastor G7): engine-checked.
    TargetChosenByHostThisTurn,
    /// The acting minion is the one this card is on (Haven Uncovered's
    /// burn): engine-checked.
    ActorIsHost,
    /// The acting minion is this card's chosen one (Priority Contract):
    /// engine-checked.
    ActorIsChosen,
    /// The actor is older than this card's host (Dive into Madness):
    /// engine-checked.
    ActorOlderThanHost,
    /// The actor is older than the card's chosen vampire (Emissary).
    ActorOlderThanChosen,
    /// The actor has not forced a vampire's cast yet this referendum
    /// (Mustafa: once each Toreador, "during the polling step", D7).
    ActorNotForcedYet,
    /// This card was not put in play this round of the combat ("not usable
    /// the round it is put in play", Molotov Cocktail): engine-checked.
    NotPlacedThisRound,
    /// The target minion bears corruption counters, anyone's (Rise of the
    /// Fallen).
    TargetHasCorruption,
    /// Terms after the first: not the minion the first term chose (Public
    /// Vilification's "other vampires").
    NotFirstChosen,
    /// The referendum is the one this card's own political action calls to
    /// burn it (Tabriz Assembly's vote).
    ReferendumToBurnThisCard,
    /// The target minion is controlled by the user's prey.
    ControlledByPrey,
    /// The target card is controlled by the user's prey (Conquest of
    /// Humanity).
    CardControlledByPrey,
    /// The target card's controller controls more locations than the user
    /// (Vritra).
    CardControllerMoreLocations,
    /// Card target: controlled by the player's predator or prey (Smash and
    /// Grab [pot]: a location).
    CardControlledByPredatorOrPrey,
    /// The user controls n ready vampires of the clan.
    ControlsReadyClan(&'static str, u8),
    /// The user controls a ready vampire of the clan with blood (Chantry).
    OwnReadyClanWithBlood(&'static str),
    /// The acting minion is a vampire.
    ActingVampire,
    /// The acting minion is a monster (rulebook "Appendix: Imbued Rules" 4).
    ActingMonster,
    /// The acting minion is an imbued.
    ActingImbued,
    /// The host imbued holds n conviction cards or more.
    HostConvictionAtLeast(u8),
    /// The host imbued is incapacitated.
    HostIncapacitated,
    /// The host is a combatant of the combat under way (Abjure).
    HostInCombat,
    /// A minion in play grants every incapacitated imbued its cheaper
    /// recovery (Jennie "Cassie247" Orne's `Stat::GrantsImbuedRecovery`).
    ImbuedRecoveryGranted,
    /// The host may use a [REACTION] ability: ready, unlocked or woken, in
    /// another Methuselah's action (advanced rules §1.5.1; LSJ 20070403).
    HostMayReact,
    /// Target minion is a monster.
    TargetMonster,
    /// The opposing combatant is a monster.
    OpponentMonster,
    /// The opposing combatant is not infernal (Shame).
    OpponentNotInfernal,
    /// The opposing combatant is no vampire of capacity above n (Shame).
    OpponentCapacityAtMost(u8),
    /// The combat under way is between a monster and a mortal (Abjure).
    CombatMonsterVsMortal,
    /// The combat under way is between two monsters (Maman Boumba).
    CombatBothMonsters,
    /// The actor is a monster (Illuminate's granted action).
    ActorMonster,
    /// The actor holds a card of this kind (Lupe's vehicle).
    ActorHolds(Holding),
    /// The action's target minion is an imbued (Champion).
    ActionTargetsImbued,
    /// The card being played targets the actor (React with Conviction).
    PlayedTargetsActor,
    /// The card being played would take control of the actor (React with
    /// Conviction).
    PlayedTakesControlOfActor,
    /// "An imbued may burn only one <name> … each action": the host has
    /// used no copy of this card's name this action (Second Sight).
    HostNameUnusedThisAction,
    /// Printed: the action is not one to enter combat (Earl).
    ActionNotEnteringCombat,
    /// The combat under way comes from the action's success, not a block
    /// (Erick "Shophet125" Franco).
    CombatFromSuccess,
    /// The host's controller controls n ready imbued or more (Earl).
    HostControllerReadyImbuedAtLeast(u8),
    /// A locked host may use a [REACTION] ability: ready, in another
    /// Methuselah's action (Vigilance; LSJ 20070403).
    HostMayReactLocked,
    /// The acting minion is a vampire with capacity n or more (Monastery
    /// of Shadows).
    ActingCapacityAtLeast(u8),
    /// The acting minion is a vampire with capacity n or less (Victorine
    /// Lafourcade).
    ActingCapacityAtMost(u8),
    /// Another Methuselah's minion attempts to block and needs intercept
    /// (KRCG News Radio's paid half).
    OthersAttemptNeedsIntercept,
    /// The acting minion is of the clan.
    ActingClan(&'static str),
    /// The current action is undirected; in a printed modifier, the action
    /// performed (Castellan).
    ActionUndirected,
    /// Not the first action announced in this minion phase (Channel 10).
    NotFirstActionOfPhase,
    /// The current action is a successful hunt (`ActionCtx::hunt_successful`)
    /// by an Anarch the user controls (The Anarch Free Press).
    AnarchHuntedYours,
    /// The user's ash heap holds a card requiring an Anarch, and their hand
    /// a card (Garibaldi-Meucci Museum).
    AnarchCardInAshAndHand,
    /// The combat is between an Anarch the user controls and another
    /// Anarch (Garibaldi-Meucci Museum).
    AnarchCombatWithYours,
    /// Another Methuselah controls this card (a stealable location):
    /// engine-checked.
    OthersControlThisCard,
    /// It is the turn of this card's controller (The Rack): engine-checked.
    CardControllersTurn,
    /// The target card is on a minion the user's prey controls (Anarch
    /// Troublemaker).
    TargetOnPreyMinion,
    /// The player has at least n pool (a granted action costing pool is not
    /// declared unpayable).
    PoolAtLeast(u8),
    /// Combat: the round is at close range.
    RangeClose,
    RangeLong,
    /// The actor is in combat with the acting minion (Chiram's Hold).
    OpponentIsActing,
    /// Damage was successfully inflicted on the actor this round of its
    /// combat (Ecstatic Agony [AUS]).
    ActorDamagedThisRound,
    /// Combat: the first round.
    FirstRound,
    NotFirstRound,
    /// Combat: a previous round, at long range as it ended (Seren
    /// Sukardi).
    PreviousRoundLong,
    /// Combat: no damage was successfully inflicted on the actor in the
    /// previous round (Seren Sukardi).
    ActorUndamagedLastRound,
    /// Combat: both combatants are still ready (Telepathic Tracking [AUS]).
    BothCombatantsReady,
    /// Both combatants are vampires of this sect (Alfred Benezri: "combat
    /// involving two other Sabbat vampires").
    BothCombatantsSect(Sect),
    /// Both combatants are vampires other Methuselahs than the user control
    /// (Sydnelson).
    BothCombatantsOthersVampires,
    /// Combat: the end-of-round window, no press to continue standing, no
    /// replacement yet (spec §4.9.1).
    CombatWouldEnd,
    /// The actor is ready (not being burned or going to torpor).
    ActorReady,
    /// Damage is pending on the actor: no prevention without damage to
    /// prevent (spec §4.5.1).
    DamagePending,
    /// Non-aggravated damage is pending on the actor (Soak).
    NonAggravatedDamagePending,
    /// Aggravated damage is pending on the actor (O Encourado).
    AggravatedDamagePending,
    /// Damage from gun strikes is pending on the actor (Kevlar Vest).
    GunStrikeDamagePending,
    /// Damage from any source but gun strikes is pending on the actor.
    NonGunDamagePending,
    /// This card was on the blocker when it blocked, not equipped after
    /// (Sniper Rifle; rules-qa 10.5).
    SourceHeldWhenBlocking,
    /// The card's controller controls its host (Vast Wealth; RTR 19951017).
    ControllerControlsHost,
    /// The card's host is ready (Shilmulo Tarot, Heart of Nizchetus).
    HostReady,
    /// The card's host is a vampire ("the vampire with this location":
    /// Palatial Estate; an ally gets nothing, rules-qa 60.17).
    HostVampire,
    /// The card's host is an ally (Chave de Braço's "or 1 life").
    HostAlly,
    /// No press to continue stands.
    NoContinuePress,
    /// The actor is the minion that blocked this action.
    ActorHasBlocked,
    /// The actor holds a title of one of these classes (effective title:
    /// contested or yielded titles do not qualify).
    ActorTitleIn(&'static [TitleClass]),
    /// The actor holds no title of these classes (Valerius; effective
    /// title).
    ActorTitleNotIn(&'static [TitleClass]),
    /// The actor belongs to one of these sects.
    ActorSectIn(&'static [Sect]),
    /// The actor's capacity is at least n.
    ActorCapacityAtLeast(u8),
    /// The current action is a political action.
    IsPolitical,
    /// The current action is not a political action (Smoke and Mirrors [obf]).
    NotPolitical,
    /// The polling step of a political action's referendum, not the blood
    /// hunt's (spec §3.7.5.4).
    PollingStep,
    /// The polling step of any referendum, blood hunts included (Aisha
    /// az-Zahra).
    AnyPollingStep,
    /// The referendum is the one the actor's political action called
    /// (Queen Anne, Diana Iadanza).
    ActorCalledReferendum,
    /// Target vampire is not abstaining from this referendum.
    NotAbstaining,
    /// The actor is the acting minion.
    ActorActing,
    /// The current action succeeded, read after resolution; a fizzled
    /// action counts (ANK 20220218).
    ActionSuccessful,
    /// The action's cost, as paid after modifications, was 1 blood or more,
    /// whoever paid it (Cavalier; PIB 20150725, LSJ 20100315).
    ActionCostBlood,
    /// A vampire of the player's ash heap burned from play while
    /// controlled, with these superior Disciplines printed (Blessed
    /// Resilience; ANK 20210813).
    OwnAshBurnedVampire(&'static [&'static str]),
    /// The current action did not fizzle: its effect happened ("after
    /// successfully equipping", Synner-G, Vulture).
    ActionNotFizzled,
    /// The action successfully rescued a vampire from torpor, not one that
    /// fizzled (Ritual Goblet).
    RescuedFromTorpor,
    /// The current action is directed; in a printed modifier, the action
    /// performed (Castellan).
    ActionDirected,
    /// The current action is directed at every Methuselah this card
    /// remembers (`CardInPlay::against`), its host's controller aside
    /// (Shadow Cast [OBL]; LSJ 20100527, rules-qa 24.5).
    DirectedAtRemembered,
    /// The actor diablerised a vampire in this action, as the acting minion
    /// or a leave-torpor action's blocker (rules-qa 16.2).
    ActorDiablerised,
    /// The acting minion belongs to another Methuselah: an action of one's
    /// own minions is undirected (LSJ 20030214-2).
    ActingOtherController,
    /// The acting minion is the player's and is not the actor (Aline
    /// Gädeke).
    OtherActingControlled,
    /// The acting minion is the player's, the actor or not (Maila).
    OwnActing,
    /// No action, combat or referendum under way; the blood hunt and the
    /// post-resolution damage are in one (Heidelberg Castle; ANK 20171124,
    /// LSJ 20090722-2).
    NoActionUnderway,
    /// An action is under way, its combats included (Alvusia's strength).
    ActionUnderway,
    /// The player controls n ready vampires or more.
    ControlsReadyVampires(u8),
    /// "Requires a ready imbued": the player controls one (Angel of
    /// Berlin, The Church of Vindicated Faith, Moise Kasavubu).
    ControlsReadyImbued,
    /// "Requires a ready imbued in play": any Methuselah's (Blood Cult
    /// Awareness Network).
    ReadyImbuedInPlay,
    /// The acting minion belongs to one of these sects.
    ActingSectIn(&'static [Sect]),
    /// The acting minion holds a title of these classes, effective (The Art
    /// Hole: a primogen).
    ActingTitleIn(&'static [TitleClass]),
    /// The vampire with this text belongs to one of these sects (Marcus
    /// Vitel merged: "while he is ready and Sabbat").
    SourceSectIn(&'static [Sect]),
    /// The acting minion is not of this sect, an ally included (The Mole).
    ActingSectNot(Sect),
    /// The actor is a vampire in torpor, its action a torpor action: blocked,
    /// no combat, the blocker may diablerize (Rapid Healing, Movement of the
    /// Slow Body).
    ActorInTorpor,
    /// An action of the actor succeeded this turn, a fizzle too (Loner).
    ActorSucceededThisTurn,
    /// No other minion of the actor's controller announced an action this
    /// turn (Loner; LSJ 20060616-1: those controlled now).
    NoOtherOwnAttemptedThisTurn,
    /// The target card uses counters, blood or pool ones (Church of the
    /// Order of St. Blaise; PIB 20110712-1).
    TargetUsesCounters,
    /// A contract may be placed on the target minion (Iliana: "Contracts
    /// cannot be placed on Iliana").
    TargetTakesContracts,
    /// A vampire of the Methuselah the action is directed at successfully
    /// hunted since the user's last turn (Songs of the Distant Vitae
    /// [QUI]).
    DirectedSeatHuntedSinceYourLastTurn,
    /// The acting minion lacks this Discipline, read live (The Mole [cel]).
    ActingLacksDiscipline(&'static str),
    /// The Methuselah the action is directed at has cards in hand.
    DirectedHandNotEmpty,
    /// Target minion is controlled by the Methuselah the action is
    /// directed at (Alicia Cortez).
    ControlledByDirectedSeat,
    /// Target minion, or an ability's user, is not the vampire whose
    /// ability it is (Alabástrom, Lord Ashton).
    NotHost,
    /// The minion attempting to block is the player's and needs intercept
    /// (Oskar Anasov; rules-qa 3.2).
    YourAttemptNeedsIntercept,
    /// The acting minion is of this clan and is not the actor (Daria).
    OtherActingClan(&'static str),
    /// The host is unlocked (a lock cost).
    HostUnlocked,
    /// It is the player's turn (Kalinda).
    OwnTurn,
    /// The actor must hunt: no blood, or made to (Restricted Vitae).
    ActorMustHunt,
    /// The player has a discard phase action left.
    DiscardPhaseActionLeft,
    /// None of the player's minions announced an action this turn (NSA
    /// Trio).
    NoActionAnnouncedThisTurn,
    /// The player has played a master card this master phase (Nonu Dis).
    MasterCardPlayed,
    /// The player's ash heap holds a library card, burned vampires aside
    /// (Lenelle, Mora; `Seat::ash_crypt`).
    AshNotEmpty,
    /// The current action requires an Anarch or makes its vampire Anarch
    /// (CrimethInc.; `ActionCtx::requires_anarch`).
    ActionRequiresAnarch,
    /// The player's library holds at least n cards (Liquidation; LSJ
    /// 20070901).
    LibraryAtLeast(u8),
    /// A minion of the active Methuselah has pox counters (Smallpox Griet).
    ActivePoxed,
    /// The player's ash heap holds at least n cards, burned vampires
    /// included (Marchesa Liliana; rules-qa 42.7).
    AshAtLeast(u8),
    /// No votes or ballots have been cast yet in this referendum.
    NoVotesCast,
    /// The referendum was called by a titled vampire of this sect, read now
    /// (T.J.).
    CallerTitledOfSect(Sect),
    /// Printed: the referendum's caller holds a title of these classes,
    /// effective (Samira).
    CallerTitleIn(&'static [TitleClass]),
    /// The action's referendum passed (Voter Captivation).
    ReferendumPassed,
    /// Target minion has cast votes or ballots in this referendum, still
    /// standing (Scalpel Tongue).
    HasCastVotes,
    /// Target minion is younger than the acting minion (lower capacity).
    Younger,
    /// The target is older than the actor (Stealing Years).
    TargetOlder,
    /// The actor is not voting in favor: votes against cannot join a cast
    /// in favor (Protected District).
    ActorNotVotingFor,
    /// Any one of these requirement lists holds in full.
    AnyOf(&'static [&'static [Requirement]]),
    /// The actor has no card in play of this kind ("only one vehicle"): no
    /// equip offered to a minion already holding one.
    ActorWithout(Holding),
    /// The actor holds fewer than n cards of this kind ("no more than two
    /// Dive into Madness").
    ActorHoldsFewer(Holding, u8),
    /// The actor holds at least n unlocked cards of this kind (Dusk Work:
    /// three Aye).
    ActorHoldsUnlocked(Holding, u8),
    /// The card being played is an action card targeting the actor
    /// (Supernatural Resistance): engine-checked.
    PlayedActionTargetsActor,
    /// The card being played requires none of these Disciplines in the mode
    /// used (Supernatural Resistance: not Celerity nor Potence).
    PlayedRequiresNone(&'static [&'static str]),
    /// The actor holds at least n cards of this kind (Neebi: three Aye).
    ActorHoldsAtLeast(Holding, u8),
    /// The player gained a victory point during the current action (Edge
    /// of the World): `GameState::action_vps`.
    GainedVpThisAction,
    /// The target minion has no card in play of this kind ("only one
    /// archetype").
    TargetWithout(Holding),
    /// The target's capacity or cost is at most twice the actor's unlocked
    /// cards of this kind (Exile [PRE]: Orun).
    TargetCapacityAtMostTwiceHeld(Holding),
    /// The target holds at least n cards of this kind (Transcendent Laibon).
    TargetHoldsAtLeast(Holding, u8),
    /// The target holds a title of one of these classes (Valerius;
    /// effective title).
    TargetTitleIn(&'static [TitleClass]),
    /// The target holds this very title (Calebros's "is the Prince of New
    /// York"; effective title).
    TargetTitle(Title),
    /// The target's title is under contest (Ephor).
    TargetTitleContested,
    /// The prey's ash heap holds two copies of a card (Giovanni del
    /// Georgio).
    PreyAshHasPair,
    /// The minion is an infernal vampire.
    ActorInfernal,
    /// The minion is a Seraph (`CryptCard::seraph`: Tattoo Signal).
    ActorSeraph,
    /// The action is an equip action (Hag's Wrinkles).
    IsEquipAction,
    /// The action's last combatants are both still ready (Pocket Out of
    /// Time; ANK 20220429).
    LastCombatantsReady,
    /// No card of this name was played during this action ("X and Y cannot
    /// both be played on the same action": Into Thin Air, Lost in Crowds).
    NotPlayedThisAction(&'static str),
    /// The actor played no card of this name this action ("A vampire cannot
    /// play both Pack Tactics and Elder Intervention").
    ActorNotPlayedThisAction(&'static str),
    /// The player has n victory points or more (Momentum's Edge).
    VictoryPointsAtLeast(u8),
    /// This card in play was played in mode n (Condemnation: Mute's [DAI],
    /// its third).
    CardMode(u8),
    /// The player has n or more vampires in their uncontrolled region.
    UncontrolledAtLeast(u8),
    /// The player's ash heap holds a library card (The Sargon Fragment, Gear
    /// Up [nec]).
    OwnAshNotEmpty,
    /// Their prey's ash heap holds a library card (The Erciyes Fragments).
    PreyAshNotEmpty,
    /// The player controls no Great Beast (LSJ 20101130).
    NoGreatBeastControlled,
    /// No card is on this card in play ("only 1 card … at a time").
    StoreEmpty,
    /// The actor meets the requirements of the card at this index on this
    /// card and can pay for it (Delivery Truck; ANK 20180817), no copy of
    /// an earlier one (Aaron Fletcher).
    ActorMayEquipStored(u8),
    /// A card is on this card in play (Maabara).
    StoreNotEmpty,
    /// "Once each turn" on a rider: once in each Methuselah's turn per copy
    /// (Rebel).
    OncePerTurn,
    /// The target vampire is of none of these clans.
    TargetClanNotIn(&'static [&'static str]),
    /// The target lacks this Discipline at superior (lower-case tag), read
    /// live (Discipline master cards).
    TargetNotSuperior(&'static str),
    /// The target is another minion than the one acting.
    OtherThanActor,
    /// The target is another minion than the one playing the card
    /// (Netwar).
    OtherThanPlayer,
    /// The target is younger than the vampire whose ability it is, not
    /// the acting minion (Javier Montoya).
    YoungerThanHost,
    /// This card in play entered before the current turn (Puppet Master
    /// [DOM]).
    EnteredBeforeThisTurn,
    /// This card in play entered before the current action (Under My Skin).
    EnteredBeforeThisAction,
    /// The card in play whose ability this is is the player's:
    /// engine-checked.
    YouControlThisCard,
    /// Target minion is controlled by the player's predator or prey (Pit of
    /// Contemplation).
    ControlledByPredatorOrPrey,
    /// Target vampire's capacity is below this card's counters (Pit of
    /// Contemplation): engine-checked.
    TargetCapacityBelowCounters,
    /// The target vampire has no more blood than this card has counters
    /// (Form of Corruption): engine-checked.
    TargetBloodAtMostCounters,
    /// The target minion carries corruption counters, anyone's (Aniel).
    TargetCorrupted,
    /// The target minion carries the user's corruption counters
    /// (Revelation of Desire [SER]).
    TargetHasYourCorruption,
    /// At the maneuver, only to get to close range: the range is long (Slam
    /// [POT]); any other step, no condition.
    ManeuverToClose,
    /// The opposing minion has a weapon in play, else the superior is not
    /// chosen (Voracious Vermin [ANI]; rules-qa 22.3).
    OpposingWeapon,
    /// The actor is unlocked.
    ActorUnlocked,
    /// The actor may use equipment: not barred this combat (Form of the
    /// Bat) nor by its own card (Bone Shambler).
    ActorMayUseEquipment,
    /// Combat: the actor's strikes inflicted more damage on the opposing
    /// vampire this round than it took, environmental damage aside (Disarm;
    /// LSJ 20010812).
    InflictedMoreThisRound,
    /// The current action is not a bleed.
    NotBleed,
    /// The acting minion is younger than the actor (Redirection [dom]).
    ActingYounger,
    /// The current action recruits or employs one of these subtypes (Paths
    /// in Two Worlds).
    RecruitingSubtype(&'static [crate::library::Label]),
    /// The host of this card is attempting to block (Dogged Pursuit):
    /// engine-checked.
    HostAttempting,
    /// Another minion than the host is attempting to block (Gianni
    /// D'Angelo): engine-checked.
    AttemptingNotHost,
    /// The ability's user played this card (`CardInPlay::by`: Melange's
    /// acting minion, Ensconced's vampire): engine-checked.
    ActingIsBy,
    /// The bleed is against the controller of this card's host (Melange):
    /// engine-checked.
    BleedAgainstHostController,
    /// Combat: the opposing minion holds no card in play of this kind
    /// ("only one Disarm").
    OpponentWithout(Holding),
    /// Target: a minion in the combat, or its retainer, with damage pending
    /// (LSJ 20001114), not damage a Fortitude-proof strike inflicted to a
    /// Fortitude card (LSJ 20050628): engine-checked.
    TargetInCombatWithDamage,
    /// As `TargetInCombatWithDamage`, non-aggravated damage pending
    /// (Randall).
    TargetInCombatWithNormalDamage,
    /// Target: a weapon whose damaging strike its bearer may use now, its
    /// own requirements and limit read (Strike with Conviction's "weapon
    /// strike"): engine-checked.
    TargetWeaponStrikeUsable,
    /// The card being played is of this type (Hide the Mind).
    PlayedCardType(CardType),
    /// The card being played is not of this type (Touch of Clarity).
    PlayedNotCardType(CardType),
    /// The card played, in the mode played, requires a Discipline (Botched
    /// Move; rules-qa 38.4).
    PlayedRequiresAnyDiscipline,
    /// The card being played requires this Discipline in the mode used,
    /// not its other Disciplines (PIB 20130704).
    PlayedRequiresDiscipline(&'static str),
    /// On a printed cost reduction: the card played or put in play matches
    /// (`PrintedCtx::card`).
    PlayedCard(CardFilter),
    /// Card target: on the minion in combat with the ability's host
    /// (Heroic Might): engine-checked.
    TargetOnOpposingCombatant,
    /// The current action was blocked (Freak Drive [FOR]).
    ActionBlocked,
    /// The seat has not played a superior Veil the Legions this turn.
    NoSuperiorVeilThisTurn,
    /// The target card is on the actor (Pack Alpha [ANI]).
    TargetOnActor,
    /// The combatant has damage pending from the opposing strikes (Rolling
    /// with the Punches [FOR]; LSJ 20001114).
    StrikeDamagePending,
    /// No reaction card was played this action, a canceled one counting
    /// (Perfectionist; LSJ 20070330).
    NoReactionPlayed,
    /// The card's host is not in the combat (Saulot's Guiding Wisdom).
    HostOutOfCombat,
    /// The host of the card in play is of this clan.
    HostClan(&'static str),
    /// The minion that blocked the current action holds a title of one of
    /// these classes.
    BlockerTitleIn(&'static [TitleClass]),
    /// The blocker is a non-Anarch vampire older than the actor, and both
    /// its controller's crypt and the actor's controller's hold a card
    /// (Bear-Baiting; RTR 20000501).
    BlockerOlderNonAnarchCrypts,
    /// The host of the card in play has this discipline (lower case: any
    /// level; upper case: superior).
    HostDiscipline(&'static str),
    /// Printed-modifier predicates (`GameState::printed_mod`) start here:
    /// the vampire whose text it is is ready.
    ThisVampireReady,
    /// The modified minion is an ally with any graft counters (Dr. Morrow;
    /// LSJ 20070306).
    ActorGraftedAlly,
    /// The opposing minion is of this clan.
    OpponentClan(&'static str),
    /// The opposing minion is not of this clan (Hazimel: "non-Ravnos").
    OpponentNotClan(&'static str),
    /// The opposing minion bears this name, any version (Elizabeth
    /// Dimitros: Hesha Ruhadze).
    OpponentNamed(&'static str),
    /// Printed: the modified minion is another Methuselah's than the
    /// printed vampire's (Legba).
    ActorOfOtherMethuselah,
    /// The opposing minion is younger / older than the modified minion.
    OpponentYounger,
    OpponentOlder,
    /// Printed: the opposing minion is Red List (Echo).
    OpponentRedList,
    /// Printed: the opposing minion is infernal (Righteous Endeavor).
    OpponentInfernal,
    /// Printed: the vampire's controller's prey controls a ready vampire
    /// with one of these titles (Suhailah).
    PreyControlsReadyTitled(&'static [TitleClass]),
    /// Printed: the action is directed at a card in play matching one of
    /// these (Bulscu: a location or an equipment).
    DirectedAtCard(&'static [CardFilter]),
    /// Printed: the vampire calls the referendum the votes are read in
    /// (Maldavis).
    SourceCalledReferendum,
    /// Printed: another vampire calls the referendum the votes are read in
    /// (Jezebelle).
    OtherCalledReferendum,
    /// Printed: the referendum the votes are read in, passing, gives a
    /// vampire a title of these classes (Jezebelle: "to declare a prince";
    /// `PrintedCtx::referendum_title`).
    ReferendumGrantsTitle(&'static [TitleClass]),
    /// Printed: the opposing minion is the one the vampire chose for its
    /// blocks until its controller's next unlock (Christopher Houghton;
    /// `GameState::unlock_marks`).
    OpponentMarkedForBlocks,
    /// The action equips its actor with an equipment card from hand bearing
    /// this label (Synner-G: electronic).
    IsEquipLabeledFromHand(crate::library::Label),
    /// The opposing minion is an ally or a younger vampire (Ragnar
    /// Nordstrom).
    OpponentAllyOrYounger,
    /// The opposing minion holds a title (effective title).
    OpponentTitled,
    /// The modified minion carries the printed vampire's Methuselah's
    /// corruption counters (Faruq Abd al-Qadir).
    ActorCorruptedBySource,
    /// The actor (for a printed modifier, the modified minion) is of this
    /// clan.
    ActorClan(&'static str),
    /// The actor is a Blood Brother of the acting minion's circle (Claiming
    /// the Body; LSJ 20020609: no circle, a circle of one).
    SameCircleAsActing,
    /// The actor, out of the combat, is a Blood Brother of its player's
    /// combatant's circle (Octopod, Three's a Crowd); `sanguinus`: the card
    /// requires Sanguinus, so Angelo is of any circle for it (pinned to the
    /// card's Disciplines by a test).
    SameCircleAsOwnCombatant {
        sanguinus: bool,
    },
    /// The minion opposing its player's combatant is still ready
    /// (Coordinate Attacks).
    OwnCombatantsOpponentReady,
    /// Major Boon: a bleed for 1 or more of another Methuselah, by a minion
    /// the player does not control, nobody burning pool in their place yet.
    BoonableBleed,
    /// Major Boon's burn: a bleed of its controller for 1 or more, nobody
    /// burning pool in their place yet.
    BleedAtYouUnboonedForOne,
    /// The acting minion is the host of this card (Loki's Gift).
    ActingIsHost,
    /// The acting vampire is of the clan the vampire whose ability this is
    /// is enslaved to (Raw Recruit's Gargoyle: `GameState::slave_clan`).
    ActingOfHostsMasterClan,
    /// A granted action's actor put the card in play or is a Blood Brother
    /// of its circle (Oppugnant Night).
    ActorOfByCircle,
    /// Its player controls another ready unlocked Blood Brother of the
    /// actor's circle (Camaraderie [SAN]).
    ControlsOtherReadyUnlockedSameCircle,
    /// Target filter: another Blood Brother of the actor's circle (Hive
    /// Mind, Slake the Thirst); uncontrolled, its printed circle;
    /// `sanguinus` as `SameCircleAsOwnCombatant`'s.
    TargetSameCircle {
        sanguinus: bool,
    },
    /// The actor is a Blood Brother with no printed circle (New Blood,
    /// Angelo: Unwholesome Bond, LSJ 20050927).
    ActorUnprintedCircle,
    /// Target filter: a vampire lacking at superior a Discipline the actor
    /// has at superior (Sanguine Instruction).
    TargetLacksActorSuperior,
    /// Target filter: an uncontrolled Blood Brother with no printed circle,
    /// in a circle of one (Unwholesome Bond, ANK 20211113).
    TargetCirclelessBloodBrother,
    /// The actor gained blood just before (`GameState::blood_gained`:
    /// Slake the Thirst).
    ActorGainedBlood,
    /// The vampire's controller, or the player, has at most n pool
    /// (Failsafe).
    PoolAtMost(u8),
    /// The vampire's controller controls an uncontested location / none.
    ControlsLocation,
    ControlsNoLocation,
    /// The vampire's controller controls a vampire in play of this name.
    ControlsVampireNamed(&'static str),
    /// The vampire's controller controls no ready unique (crypt card)
    /// vampire of this clan older than it (The Guardian).
    ControlsNoReadyOlderOfClan(&'static str),
    /// Printed: the vampire's controller controls at least n vampires of
    /// this clan, ready or in torpor, itself included (Al-Muntathir).
    ControlsOfClan(&'static str, u8),
    /// Printed: the vampire's controller controls no ready vampire with a
    /// title of these classes (Jezebelle).
    ControlsNoReadyTitled(&'static [TitleClass]),
    /// The current action is the player's, blocked, and no slave took its
    /// combat yet (rulebook "Slave").
    NoSlaveInstead,
    /// Printed: the vampire claims a title of these classes, its sect not
    /// read (Karen Suadela: her immunity keeps her sect through a Fall).
    ActorClaimsTitleIn(&'static [TitleClass]),
    /// Printed: the vampire has no title (Gerald Windham).
    ActorUntitled,
    /// The actor follows no printed path (The Path of the Scorched Heart:
    /// "a vampire can follow only one path", its own card aside).
    ActorFollowsNoPath,
    /// Printed: the card is played as a reaction (Scout Youngwood).
    PlayedAsReaction,
    /// Printed: the card is played in a strike mode, a dodge included
    /// (Antoinette DuChamp; rules-qa 42.2).
    PlayedInStrikeMode,
    /// Printed: the vampire has diablerized an older vampire (Smudge the
    /// Ignored: "after … for the first time").
    DiablerizedOlder,
    /// Printed: the vampire whose text it is is the acting minion (Nergal,
    /// advanced).
    SourceActing,
    /// Printed: the card is played as a combat card (Adana de Sforza); of
    /// several types, the type it is played as (PIB 20150924-2).
    PlayedAsCombat,
    /// The actor could have performed the action under way, every effect
    /// on it so far included (Mask of a Thousand Faces): not one equipment
    /// or a card in play grants (LSJ 20020926), its card within its
    /// requirements, modifiers within clan and Disciplines (RTR 19980623,
    /// RTR 20030519).
    MaskCapable,
    /// The target is not immune to Gehenna events (Blood Trade, Karen
    /// Suadela).
    NotGehennaImmune,
    /// No ready vampire is Black Hand, any Methuselah's (Fall of the
    /// Sabbat).
    NoReadyBlackHand,
    /// A vampire in play is Black Hand, any Methuselah's: one to choose
    /// (Hand Contract).
    BlackHandVampireInPlay,
    /// No vampire of this name is ready, any Methuselah's (Valkyrie).
    NoReadyNamed(&'static str),
    /// The Methuselah controls a locked minion (Elen Kamjian).
    ControlsLockedMinion,
    /// Target minion or card is controlled by another Methuselah (no combat
    /// with one's own minion).
    NotControlled,
    /// Target card in play is of this kind.
    TargetCard(CardFilter),
    /// Target card in play has no card of this name on it ("an equipment
    /// can have only one": Blood Tempering).
    TargetCardWithout(&'static str),
    /// Target minion is a slave, not freed (Defender of the Haven;
    /// `GameState::slave_clan`).
    TargetSlave,
    /// Target card is on a ready minion.
    TargetOnReadyMinion,
    /// The acting minion could hold the target card: none of its "only
    /// one" limits reached.
    ActorMayHoldTarget,
    /// Target minion is locked (Nathaniel Bordruff).
    TargetLocked,
    /// Target minion is a vampire, not an ally.
    TargetVampire,
    /// Target minion is an imbued.
    TargetImbued,
    /// Target imbued has had no conviction played on it this unlock phase
    /// (rulebook "Appendix: Imbued Rules" 2).
    TargetConvictionUnplayed,
    /// The current action is directed at a minion the player controls
    /// (Szlachta Bodyguard).
    DirectedAtYourMinion,
    /// As `DirectedAtYourMinion`, an ally (Screamer).
    DirectedAtYourAlly,
    /// The current action is directed at a card in play the player controls
    /// matching one of these (General Perfidio Díos: a location).
    DirectedAtYourCard(&'static [CardFilter]),
    /// The acting minion is controlled by the player's predator or prey
    /// (Screamer).
    ActingByPredatorOrPrey,
    /// In another Methuselah's minion phase: engine-checked
    /// (`Engine::in_others_minion_phase`).
    OthersMinionPhase,
    /// This card's minion (an ally's own card) is locked.
    CardLocked,
    /// The card being played is a strike card of the host's opponent:
    /// engine-checked (`Engine::played_strike_by_opponent`).
    PlayedStrikeByOpponent,
    /// The card being played is a strike or grapple card of the actor's
    /// opponent (Charismatic Aura): engine-checked.
    PlayedStrikeOrGrappleByOpponent,
    /// The card being played is the opposing combatant's (Botched Move).
    PlayedByOpposingMinion,
    /// The opponent's card would restrict the actor's strikes (Groundfighting):
    /// to hand strikes (a grapple, Immortal Grapple), or no equipment while
    /// it has a weapon (Drawing Out the Beast; LSJ 20050221): engine-checked.
    PlayedRestrictsStrikesByOpponent,
    /// The card being played is an aim, frenzy or grapple card of the
    /// actor's opponent (Bollix [pre]): engine-checked.
    PlayedAimFrenzyGrappleByOpponent,
    /// The actor is an unmerged advanced vampire (Legacy).
    ActorUnmergedAdvanced,
    /// The actor performed a political action whose referendum passed this
    /// turn (Antonio Delgado).
    ActorPassedReferendumThisTurn,
    /// The player has a vampire in their uncontrolled region (Lázár
    /// Dobrescu: LSJ 19990215).
    OwnUncontrolledVampire,
    /// The current action is directed at the host of this card (Zaire River
    /// Ferry): engine-checked.
    ActionTargetsHost,
    /// Minion target: entered play since the player's last minion phase
    /// ended (Chameleon; `Minion::entered_at`).
    TargetEnteredSinceYourLastMinionPhase,
    /// Minion target: entered play in the player's last influence phase
    /// (Legendary Vampire; `Minion::entered_at`).
    TargetEnteredInYourLastInfluencePhase,
    /// Card target: a weapon (Fast Hands).
    TargetWeapon,
    /// Minion target: an ally, or a vampire with capacity n or less
    /// (Approximation of Loyalty).
    TargetAllyOrCapacityAtMost(u8),
    /// Minion target: its controller has n pool or less, read live (From a
    /// Sinking Ship).
    TargetControllerPoolAtMost(i32),
    /// The card being played is played by an ally or a vampire with
    /// capacity n or less (Approximation of Loyalty [PRE]).
    PlayedByAllyOrCapacityAtMost(u8),
    /// The card being played is played by a vampire with capacity n or less
    /// (Lucita).
    PlayedByVampireCapacityAtMost(u8),
    /// The card being played is played by a vampire younger than the actor
    /// (Mithras G3).
    PlayedByYoungerVampire,
    /// No older ready untitled vampire of this clan and sect, any
    /// Methuselah's (Osebo Kholo).
    NoOlderReadyUntitled {
        clan: &'static str,
        sect: crate::crypt::Sect,
    },
    /// The player's ash heap holds a burned vampire (Redeem the Lost Soul).
    OwnAshVampire,
    /// Combat: the opposing minion is an ally being burned this round, in
    /// the end-of-round window (Orgy of Blood).
    OpposingAllyBurned,
    /// The player's ash heap holds a card matching the filter (Drozodny).
    OwnAshHas(CardFilter),
    /// The actor is no slave (Virstania's bleed).
    ActorNotSlave,
    /// No Methuselah's crypt is empty (Petra Resonance).
    NoCryptEmpty,
    /// The predator is not the prey, and each controls a ready vampire
    /// (Taunt the Caged Beast [ANI]).
    PredatorAndPreyReadyVampires,
    /// Seat target: the player's predator or prey (Josef von Bauren).
    TargetPredatorOrPrey,
    /// Seat target: a Methuselah with more pool than the player
    /// (Malabranca).
    TargetPoolMoreThanYours,
    /// Host rider: the host was blocked by an older vampire (Spleen,
    /// Georgio Gutterpunk): engine-checked.
    BlockedByOlder,
    /// Host rider: the host blocked an older vampire, the combat its block's
    /// (Padraic Conroy): engine-checked.
    BlocksOlder,
    /// Host rider: the referendum was held and failed, not canceled
    /// (Hellene Panhard): engine-checked.
    ReferendumFailed,
    /// The user controls the prince of this city, its effective title (The
    /// Louvre, Paris).
    ControlsPrinceOf(&'static str),
    /// Any time of the player's own minion phase, actions included
    /// (`Engine::in_own_minion_phase`; rules-qa 26.5).
    OwnMinionPhase,
    /// Target minion is controlled by the player's predator.
    ControlledByPredator,
    /// Target minion is controlled by the active Methuselah (Owain Evans).
    ControlledByActive,
    /// No combat is under way (War Ghoul).
    NotInCombat,
    /// The current block has not been answered by continuing the action as
    /// if unblocked already (Crypt's Sons: once per block).
    NotContinued,
    /// No damage resolution step is under way (Vagabond Mystic, LSJ
    /// 20001111).
    NotInDamageResolution,
    /// This card has at least n life: a retainer's counters, an ally's own
    /// life.
    LifeAtLeast(u8),
    /// The actor is neither attempting to block nor the blocker (Underbridge
    /// Stray).
    ActorNotBlocking,
    /// The current action recruits a ghoul ally requiring a Tzimisce
    /// (Szlachta Assistant).
    RecruitingTzimisceGhoul,
    /// This card in play is its superior version (entered with mode 1).
    SuperiorVersion,
    /// This card in play is its basic version (entered with mode 0).
    BasicVersion,
    /// Target minion is another copy of the actor ally (Bone Shambler).
    TargetOtherCopy,
    /// Target minion is unlocked.
    TargetUnlocked,
    /// Target minion is an ally.
    TargetAlly,
    /// The target card is on a vampire (Far Mastery).
    TargetOnVampire,
    /// Target minion is an ally below its starting life (Vagabond Mystic;
    /// advanced rules 5.5.2).
    TargetAllyHurt,
    /// No block is being attempted in the current action, nor has one
    /// succeeded ("fail an action" after the as-announced window, ANK
    /// 20260316).
    NoBlockAttempt,
    /// Target minion is an ally or a vampire younger than the acting
    /// minion (Adisa).
    AllyOrYounger,
    /// The actor's opposing combatant is an ally of this subtype (Rowena).
    OpponentSubtype(crate::library::Label),
    /// The actor (printed: the modified minion) is an ally of this subtype
    /// (Jacob Fermor, Black Forest Base).
    ActorSubtype(crate::library::Label),
    /// The actor's opposing combatant is a vampire: a card naming the
    /// opposing vampire has no object in an ally (Taste of Vitae; advanced
    /// rules 5.5.1).
    OpponentVampire,
    /// The opposing minion is a vampire not older than the actor (Alpha
    /// Glint).
    OpponentVampireNotOlder,
    /// The actor's opposing combatant is not ready (Hell-for-Leather
    /// [obt]).
    OpponentNotReady,
    /// The actor's opposing combatant is still in play, not burned
    /// (Disarm, Pulled Fangs: a card put on it; rules-qa 75.1).
    OpponentPresent,
    /// The target Methuselah is about to be ousted, waiting on a Life
    /// Boon (`Seat::oust_pending`).
    TargetSeatOusting,
    /// The target Methuselah's ready titled vampire just left the ready
    /// region (Leadership Vacuum: `GameState::titled_left`).
    TargetSeatLostTitled,
    /// Rider: the active Methuselah is one the card remembers (`against`;
    /// Leadership Vacuum).
    ActiveIsAgainst,
    /// The user can give the target enough pool to stay in the game (1
    /// above what they owe) and keep 1 of their own (Life Boon).
    CanSaveTarget,
    /// The target Methuselah this card remembers is the active one (Life
    /// Boon).
    BoonTargetActive,
    /// A vampire in combat faces an opposing strike attempting aggravated
    /// damage, effective at this range (Rötschreck: RTR 19980928; a strike
    /// only, ANK 20220114; dodged or not, LSJ 20011005); any Methuselah's
    /// (LSJ 20080821: on the acting vampire, by the Yawp Court's player).
    CombatantFacesAggravatedStrike,
    /// The target is that vampire (Rötschreck: RTR 19980623).
    TargetFacesAggravatedStrike,
    /// The target minion is in the combat under way.
    TargetCombatant,
    /// Control of the target is about to change (`crate::control`).
    TargetControlAboutToChange,
    /// The target is the vampire a passed blood hunt is about to burn
    /// (`crate::control`).
    TargetHuntedToBurn,
    /// The target is about to enter combat: blocked or blocking, or fought
    /// by the action's resolution (blocks declined) or the combat Psyche!
    /// queued.
    TargetAboutToEnterCombat,
    /// The combat's acting side is a vampire the user controls, Deep Song's
    /// target included (Powerbase: Savannah; ANK 20230814).
    ActingCombatantYourVampire,
    /// The target minion has at most n blood (Jake Washington).
    BloodAtMost(u8),
    /// The target vampire committed diablerie since the user's last turn
    /// ended, the current turn counting (Carlton Van Wyk; LSJ 20020429).
    TargetDiablerieSinceYourLastTurn,
    /// The target minion successfully bled for at least `min` since the
    /// user's last turn (its current turn counting; LSJ 20020429), the user
    /// bled if `against_you` (Public Enemy, Retribution).
    TargetBledSinceYourLastTurn {
        min: u8,
        against_you: bool,
    },
    /// The target minion is untitled (Public Enemy).
    TargetUntitled,
    /// The target minion is infernal (Public Enemy); uncontrolled, its
    /// printed card (Tend the Flock).
    TargetInfernal,
    /// The uncontrolled vampire is unique (Thicker than Blood).
    TargetUnique,
    /// No vampire of the uncontrolled vampire's name is in play, anyone's:
    /// it would not contest (Thicker than Blood, as Gather).
    TargetWouldNotContest,
    /// The actor is a slave (`CryptCard::slave`; Alvusia).
    ActorSlave,
    /// The actor is of the clan the card's player is enslaved to (Raw
    /// Recruit: "a vampire you control to whom this Gargoyle is enslaved").
    ActorIsBysMaster,
    /// The actor is not Red List (Public Enemy).
    ActorNotRedList,
    /// The actor blocked an acting vampire of this sect (Promise of 1528,
    /// as Obedience's `BlockingYoungerActing`).
    BlockingActingOfSect(Sect),
    /// The actor is the combat target of an acting vampire of this sect,
    /// blocks declined (Promise of 1528, as `FoughtByYoungerActing`).
    FoughtByActingOfSect(Sect),
    /// The actor is the minion target of a directed action of an acting
    /// vampire of this sect (Promise of 1528).
    TargetOfActingOfSect(Sect),
    /// The actor is chosen in the terms of the referendum under way
    /// (Promise of 1528).
    TermsChoseActor,
    /// The actor is the acting minion, blocked by a vampire of this sect,
    /// or about to fight one again in the combat Psyche! queued (Promise of
    /// 1528 as an action modifier; rules-qa 88.11).
    BlockedByVampireOfSect(Sect),
    /// The actor, ready, fought the combat just ended against a minion now
    /// not ready that had a contract choosing it as it began (Provision of
    /// the Silsila).
    ActorFoughtItsContractsMinion,
    /// Target minion, or vampire in the uncontrolled region, is of this
    /// clan (Thing).
    TargetClan(&'static str),
    /// The acting minion is controlled by the actor's predator
    /// (Instinctive Reaction, My Enemy's Enemy [AUS]).
    ActingByPredator,
    /// More than two Methuselahs are left in the game (My Enemy's Enemy
    /// [AUS]).
    MoreThanTwoLeft,
    /// The minion attempting to block is of one of these sects (Organized
    /// Resistance).
    AttemptingSectIn(&'static [crate::crypt::Sect]),
    /// The minion attempting to block is of this clan (Market Square).
    AttemptingClan(&'static str),
    /// The action was unsuccessful: blocked or failed (Neighborhood Watch
    /// Commander).
    ActionUnsuccessful,
    /// No Neighborhood Watch Commander has inflicted its damage this
    /// action.
    NoWatchDamageThisAction,
    /// No combat is queued (Psyche! [CEL]).
    NoCombatQueued,
    /// A vampire without this Discipline attempts to block, read live (The
    /// Kiss of Ra).
    AttemptingVampireWithout(&'static str),
    /// The bleed's current target has n or fewer pool (Foreshadowing
    /// Destruction [DOM]).
    BleedTargetPoolAtMost(u8),
    /// Target minion is of one of these subtypes (Shadow Sentinel [OBL]).
    TargetSubtypeIn(&'static [crate::library::Label]),
    /// The card being played is a minion card, not a master (Direct
    /// Intervention).
    PlayedCardIsMinionCard,
    /// The card being played is played by a minion holding no card of this
    /// kind (Mundane: no Orun).
    PlayedByMinionWithout(Holding),
    /// The card being played is played by the actor's predator or prey
    /// (Wash).
    PlayedByPredatorOrPrey,
    /// The card being played is played by a monster (Determine).
    PlayedByMonster,
    /// The actor has at least n blood, an effect burning it (ANK 20210226);
    /// on a card played, beyond its cost as paid (`play_options`).
    ActorBloodAtLeast(u8),
    /// The actor can attempt to block the current action, its lock aside,
    /// or is attempting already (`Engine::can_attempt_block`; ANK
    /// 20180623, LSJ 20010714-2).
    CanAttemptBlock,
    /// As `CanAttemptBlock`, ignoring prey/predator/target restrictions:
    /// any Methuselah but the acting one, every other bar standing (Eagle's
    /// Sight [AUS]; RTR 19950413, RTR 20020501).
    CanAttemptBlockAnyone,
    /// The actor may be locked or unlocked: a permission, not a requirement
    /// (Shadow Sentinel [OBL]).
    MayBeLocked,
    /// Seat target: the player's prey (Shroud of Decay [OBL]).
    TargetPrey,
    /// The target Methuselah is the user's predator (San Lorenzo de El
    /// Escorial).
    TargetPredator,
    /// Seat target: at least n cards in their ash heap, burned vampires
    /// included (Shroud of Decay [OBL]; rules-qa 42.7).
    TargetAshAtLeast(u8),
    /// Seat target: a minion in their uncontrolled region (San Lorenzo de
    /// El Escorial; D14).
    TargetHasUncontrolled,
    /// Seat target: the predator of the actor's predator (My Enemy's
    /// Enemy [AUS]).
    PredatorsPredator,
    /// Printed: the opposing minion is a vampire of one of these sects
    /// (Céleste Lamontagne).
    OpponentSectIn(&'static [Sect]),
    /// Printed: the opposing minion's controller controls a Gehenna card
    /// (Céleste Lamontagne).
    OpponentControllerHasGehenna,
    /// A vampire of this name is ready, any Methuselah's (Rurik Rakoczy).
    ReadyNamed(&'static str),
    /// A vampire of this name is in the controller's ready region (Viri,
    /// Zizi; rules-qa 73.7).
    OwnReadyNamed(&'static str),
    /// The actor is not sterile (Childe of the Revolution; rulebook
    /// "Traits").
    ActorNotSterile,
    /// In any Methuselah's master phase (Powerbase: Cape Verde):
    /// engine-checked.
    InMasterPhase,
    /// The active Methuselah is the player's prey (Donatello Giovanni).
    ActiveIsPrey,
    /// Minion target: not controlled by the player's predator.
    TargetNotOfPredator,
    /// The action under way has not failed (Andre LeRoux: not after a
    /// Spying Mission, PIB 20150612).
    ActionNotFailed,
    /// Printed: the action is neither a hunt nor one the vampire's own text
    /// grants (Black Annis).
    OtherNonHuntAction,
    /// The card played is not an out-of-turn one (Bleeding the Vine).
    PlayedNotOutOfTurn,
    /// The minion that blocked the action is not ready (Shadow Boxing
    /// [OBF][POT]).
    BlockerNotReady,
    /// No combatant has declared a strike: combat ends this round
    /// (Gianna di Canneto's damage; ANK 20211130).
    NoCombatEndsStrike,
    /// After a combat resulting from a block, not one the action's
    /// resolution began (Shadow Boxing [OBF][POT]; RTR 19970630).
    CombatFromBlock,
    /// No slave took the action's combat in its acting minion's place
    /// (Shadow Boxing [OBF][POT]; ANK 20200114).
    NoSlaveTookCombat,
    /// Printed: the Methuselah the action is directed at controls the
    /// source vampire (Sir Ralph Hamilton: "when bleeding Ralph's
    /// controller").
    TargetIsSourceController,
    /// Printed: the target Methuselah controls a ready minion of one of
    /// these clans, or (`not`) none (Queen Anne, Martin Franckel).
    TargetSeatControlsReadyClan {
        clans: &'static [&'static str],
        not: bool,
    },
    /// Printed: the target Methuselah controls a ready vampire of this
    /// sect, or (`not`) not of it (Cornelius Ottavio, Marcus Vitel).
    TargetSeatControlsReadySect {
        sect: Sect,
        not: bool,
    },
    /// Printed: the target Methuselah controls no ready vampire of this
    /// sect (Kestrelle Hayes).
    TargetSeatControlsNoReadySect(Sect),
    /// The Methuselah the action is directed at controls a ready vampire
    /// with one of these titles (Maldavis: "a ready primogen").
    TargetSeatControlsReadyTitled(&'static [TitleClass]),
    /// Printed: the target Methuselah controls a location, uncontested
    /// (Porphyry).
    TargetSeatControlsLocation,
    /// Printed: the vampire whose text it is holds one of these titles
    /// (Marcus Vitel).
    SourceTitleIn(&'static [TitleClass]),
    /// The other Methuselahs control at least n non-derivative hunting
    /// grounds (Poacher's Hunting Ground: one pick each).
    OthersHuntingGroundsAtLeast(u8),
    /// Card target: on a locked minion (Smear Campaign).
    TargetOnLockedMinion,
    /// Card target: on a minion not of this sect (Smear Campaign).
    TargetOnMinionNotOfSect(Sect),
    /// Card target: on a minion controlled by the player's predator or prey.
    TargetOnPredatorOrPreyMinion,
    /// Card target: not a card the filter matches (Smear Campaign).
    TargetCardNot(CardFilter),
    /// On a card's ability: its controller controls fewer copies of it than
    /// ready vampires of this clan (Emerald Legionnaire).
    FewerCopiesThanReadyOfClan(&'static str),
    /// The prey's uncontrolled region holds blood (Solomon Batanea).
    PreyUncontrolledHasBlood,
    /// The Methuselah the bleed is directed at has n cards in their ash
    /// heap, burned vampires included (Trochomancy's removal).
    BleedTargetAshAtLeast(u8),
    /// Card target: on a minion the player controls (Threading the Path of
    /// Orpheus).
    TargetOnOwnMinion,
    /// Card target: played by another Methuselah (Threading the Path of
    /// Orpheus).
    TargetCardPlayedByOther,
    /// Target minion: not infernal (Warding the Beast).
    TargetNotInfernal,
    /// Target minion: a slave enslaved to the acting vampire's clan
    /// (Potio Martyrium).
    TargetSlaveOfActorClan,
    /// An ally burned from play is in an ash heap (Pressing Flesh; ANK
    /// 20200420-3).
    BurnedAllyInAnyAsh,
    /// The actor's opposing combatant is not an ally of this subtype
    /// (Autonomic Mastery's "non-wraith ally").
    OpponentNotSubtype(crate::library::Label),
    /// The card being played is a frenzy card, the actor not barred from
    /// canceling one (Bliss; `Engine::frenzy_on`).
    PlayedFrenzy,
    /// Card target: a card giving a Discipline the actor lacks at superior
    /// (Absorb the Mind [MYT]; LSJ 20011205-2).
    TargetDisciplineNotSuperiorOnActor,
    /// No card on the actor's opposing combatant meets these card-target
    /// requirements (Absorb the Mind [MYT] with nothing to take).
    OpponentHoldsNoCard(&'static [Requirement]),
    /// Card target: on another Blood Brother of the actor's circle;
    /// `sanguinus` as `SameCircleAsOwnCombatant`'s (Clockwerx).
    TargetOnSameCircle {
        sanguinus: bool,
    },
    /// Target minion: not controlled by the controller of the minion
    /// opposing the actor in combat (Blissful Agony [VAL]).
    TargetNotOfOpposingController,
    /// Target minion: younger than the minion playing the card, not the
    /// acting one (Knotted Cord by a reacting infernal): engine-checked.
    YoungerThanPlayer,
    /// The action is directed at the card this card is on (Ensconced).
    DirectedAtCardItIsOn,
    /// The actor blocked an acting ally (Gift of Sleep, as
    /// `BlockingActingOfSect`).
    BlockingActingAlly,
    /// The actor is about to fight an acting ally, blocks declined or a
    /// combat Psyche! queued (Gift of Sleep, as `FoughtByActingOfSect`).
    FoughtByActingAlly,
    /// The actor is the acting minion about to fight an ally: its blocker,
    /// its rush's target, Psyche!'s opponent (Gift of Sleep).
    FightsAlly,
    /// The action was taken with an action card bound for its player's ash
    /// heap as the action ends (Hay Ride; `Engine::action_card_to_ash`):
    /// engine-checked.
    ActionCardToAsh,
    /// The actor is unlocked to lock as the effect, a woken one not (Hide
    /// the Heart [VAL]; ANK 20230817).
    ActorUnlockedToLock,
    /// The actor is the minion attempting to block, intercept needed or not
    /// (Strix [str]: "even if intercept is not yet needed", rules-qa 3.2).
    ActorAttempting,
    /// The minion attempting to block is a vampire of the clan the actor is
    /// enslaved to, read live (Voices of the Castle [vis]; rules-qa 84.9).
    AttemptingOfActorsMasterClan,
    /// The action's blocker is an ally or a vampire of capacity n or less
    /// (Momentary Delay; a slave taking the combat: the blocker, ANK
    /// 20200114).
    BlockerAllyOrCapacityAtMost(u8),
    /// The action had at most n rounds of combat, every combat's (Momentary
    /// Delay).
    ActionCombatRoundsAtMost(u8),
    /// The actor lost fewer than n blood since the action was declared,
    /// costs and effects included (Momentary Delay).
    ActorBloodLostThisActionBelow(u8),
    /// The actor ended no combat of the action before its press step: a
    /// combat ends strike, Anesthetic Touch's (Momentary Delay).
    ActorEndedNoCombatEarly,
    /// The card being played puts itself on a vampire with this Discipline
    /// its player does not control, read live (Treat the Sick Mind [OBE]).
    PlayedPutsOnUncontrolledWith(&'static str),
    /// The actor is a slave of the host's controller enslaved to the host's
    /// clan (Jar of Skin Eaters' Gargoyle; rules-qa 84.9).
    ActorSlaveOfHostClan,
    /// The host's initial strike this round is or may still be this weapon:
    /// none chosen yet and none committed elsewhere, or made with it
    /// (Joumlon's Axe; LSJ 20001126).
    InitialStrikeIsOrMayBeThis,
    /// The acting minion is not the actor and is infernal, a demon ally by
    /// its card's Infernal label included (The Horde; rules-digest §3.6).
    OtherActingInfernal,
    /// The target's capacity, as the actor's player reads it, is at most n
    /// times the actor's, read live (Jozz).
    TargetCapacityAtMostActorTimes(u8),
    /// The card is played in the master phase of the vampire's controller
    /// (Matthew; `PrintedCtx::master_phase_of`).
    InOwnMasterPhase,
    /// The actor's chosen strike is a gun's (Mikael Birkholm).
    StrikingWithGun,
    /// The target minion is not of this clan, a clanless one included
    /// (Tanginé: "any non-Salubri minion").
    TargetNotOfClan(&'static str),
}

/// A reaction's gift to its minion in the combat a block begins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockRider {
    /// 1 optional maneuver in the resulting combat (Spirit's Touch).
    Maneuver,
    /// 1 optional maneuver in its first round (Mind of the Wilds).
    ManeuverFirstRound,
    /// The opposing minion (the blocker) gets 1 optional press in the first
    /// round (Dive Bomb).
    OpponentPressFirstRound,
    /// The blocker's "strike: combat ends" denied for the rest of the action,
    /// any round of any combat (Mind of the Wilds; LSJ 20100308).
    OpponentNoCombatEndsThisAction,
    /// Prevent n damage in the first round (Precognition).
    PreventFirstRound(u8),
    /// Strike: combat ends in the first round (Night Terrors [OBF][PRE]).
    CombatEndsFirstRound,
    /// Neither combatant strikes in the first round (One With the Land
    /// [PRO]).
    NoStrikesFirstRound,
    /// Its minion's equipment abilities are unusable in the combat (Form of
    /// the Bat).
    NoEquipment,
    /// Burn n blood to unlock, offered after the combat (Truth in Darkness
    /// [OBL]).
    UnlockAfterForBlood(u8),
    /// 1 optional press in that combat (Hunter's Mark).
    Press,
    /// At that combat's end, the only ready combatant gains blood equal to
    /// the opposing vampire's capacity (Games of Instinct).
    LoneReadyGainsCapacity,
    /// The opposing "strike: combat ends" denied in the first round
    /// (Hunter's Mark [CEL][THA]).
    OpponentNoCombatEndsFirstRound,
    /// +n strength for the combat, chosen before range in the first round
    /// (Show of Force).
    StrengthBeforeRange(u8),
    /// Prevent n damage in any round of the combat (Beast Meld [ani][pro]).
    PreventInCombat(u8),
    /// n environmental damage on the opposing minion in the first round's
    /// strike resolution (Bar Último Trago, given to both combatants).
    EnvironmentalFirstRound(u8),
    /// +n strength in that combat (Make the Misere [pot]).
    Strength(u8),
    /// Combat over with this vampire ready and the opponent not: n blood to
    /// an uncontrolled `clan` vampire, chosen then (Hunting the Beast [AUS]).
    UncontrolledBloodIfWon { clan: &'static str, n: u8 },
    /// 1 optional maneuver or press, one use (Mind Tricks [DEM]).
    ManeuverOrPress,
    /// The opposing minion cannot maneuver (Ni Dieu ni Maître [ani]).
    OpponentNoManeuver,
    /// 1 optional maneuver in that combat, only to get to long range
    /// (Repulsion [pre]).
    ManeuverToLong,
    /// 1 optional press in that combat, only to continue (Harass).
    ContinuePress,
    /// Its minion's controller's hand size +n in that combat (Rumble).
    HandSize(u8),
    /// Its minion cannot strike in the first round (Fast Reaction).
    NoStrikeFirstRound,
    /// Before block resolution, its player locks a younger non-blocking
    /// vampire the blocker's controller controls (Loose Cannon [dem]).
    LockYoungerOfBlockersSeat,
    /// Before block resolution, n blood from a non-blocking vampire the
    /// blocker's controller controls to its minion (Loose Cannon [tha]).
    BloodFromBlockersSeat(u8),
    /// Its minion's hand strikes aggravated in that combat (Loose Cannon
    /// [vic]).
    AggravatedHands,
    /// Before block resolution, its minion goes to torpor (Cry Wolf).
    ActingToTorpor,
    /// The first round's range is close (Charge of the Buffalo; RTR
    /// 19970630).
    CloseFirstRound,
    /// Its minion's first-round initial strike is a hand strike at +n
    /// damage (Charge of the Buffalo; rules-qa 10.1).
    InitialHandStrikeFirstRound(u8),
    /// Its minion does not unlock as normal during its next unlock phase
    /// (Visionquest; LSJ 20050114).
    SkipNextUnlock,
}

/// How a card in play reduces its controller's costs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CostSource {
    /// Lock: an action card a vampire plays costs 1 blood less (The Line).
    Line,
    /// With the Edge, lock: a card costs 2 pool or blood less (The Shard).
    Shard,
    /// Its counters pay the blood of an action card a vampire of this clan
    /// plays (Ravnos Carnival).
    Carnival(&'static str),
    /// Lock: its counters pay an equip's blood or pool (Ravnos Cache).
    Cache,
    /// Its counters pay an Anarch's equipment blood or pool, no lock
    /// (Hackerspace).
    Hackerspace,
    /// Its ally locks: an equipment or location costs 1 pool less
    /// (Eccentric Billionaire).
    Billionaire,
    /// Burned: a combat card its host plays costs 1 blood less (Focus the
    /// Blood).
    FocusBlood,
    /// Lock: recruiting a mortal ally or employing a mortal retainer costs
    /// 1 blood or pool less (Sunset Strip, Hollywood), as an action or not
    /// (ANK 20230824).
    SunsetStrip,
    /// Burned: recruiting an ally or employing a retainer requiring the
    /// clan costs up to n blood or pool less, as an action or not (Soul of
    /// the Earth: a Gargoyle; ANK 20210928).
    HireRequiringClan { clan: &'static str, n: u8 },
}

/// A mortal ally or retainer (Sunset Strip, Hollywood's reduction).
pub fn mortal_hire(card: u16) -> bool {
    let c = &crate::library::LIBRARY[card as usize];
    c.labels.contains(&crate::library::Label::Mortal)
        && (c.types.contains(&CardType::Ally) || c.types.contains(&CardType::Retainer))
}

/// Whom a conditional bonus reads its condition on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum On {
    /// The minion playing the card.
    Actor,
    /// The acting minion.
    Acting,
}

/// How many points a referendum allocates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Points {
    Fixed(u8),
    /// The Methuselahs in the game (Conservative Agitation).
    Methuselahs,
    /// n points of the chosen Methuselah's pool; a shortfall at resolution
    /// is re-allocated by the caller (Parity Shift; LSJ 20010606).
    PoolOfChosen(u8),
    /// Half the acting vampire's capacity, rounded up, read as the terms
    /// are chosen (Free States Rant).
    HalfActorCapacity,
}

/// Which Methuselahs may receive points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Among {
    /// Any Methuselah, the caller included.
    Any,
    /// Other Methuselahs only.
    Others,
    /// Any Methuselah but the one chosen by the previous term.
    AnyNotChosen,
    /// Other Methuselahs, the chosen one excluded (Camarilla's Iron Fist).
    OthersNotChosen,
}

/// Which Methuselahs a `ChooseSeat` term may pick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SeatFilter {
    Any,
    /// One with more pool than the caller (Parity Shift).
    MorePoolThanCaller,
    /// Another Methuselah (The Meddling of Semsith).
    Other,
    /// One given points by the allocation before it (Information
    /// Exchange): read where the term is offered.
    Allocated,
}

/// One term of a referendum, chosen stepwise by the caller once the action
/// succeeds (rulebook "The Referendum" step 1; rules-qa 3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Term {
    /// Allocate points among at least `min_seats` Methuselahs (a point
    /// total may exceed a Methuselah's pool: KRC rulings).
    Allocate {
        points: Points,
        among: Among,
        min_seats: u8,
    },
    /// Choose between `min` and `max` Methuselahs.
    ChooseSeats { min: u8, max: u8 },
    /// Choose at least half of the Methuselahs in the game, rounded up
    /// (Finding the Path).
    ChooseSeatsHalfOrMore,
    /// Choose one or more Methuselahs no card of this name in play chose
    /// (Year of Fortune).
    ChooseSeatsUnchosen(&'static str),
    /// Choose one Methuselah (the "chosen" one for later terms).
    ChooseSeat(SeatFilter),
    /// Choose between `min` and `max` minions meeting the filter.
    ChooseMinions {
        filter: &'static [Requirement],
        min: u8,
        max: u8,
    },
    /// Choose X minions meeting the filter, X the political card's, fewer
    /// if no more qualify (My Kin Against the World).
    ChooseMinionsX { filter: &'static [Requirement] },
    /// Choose a clan.
    ChooseClan,
    /// Choose a card in play matching the filter: a choice, never a target
    /// (Disputed Territory; rules-qa 38.6).
    ChooseCard(CardFilter),
    /// Name `n` cards of the caller's ash heap matching the filter, by name,
    /// a name twice for two copies (Undele; LSJ 20100917).
    ChooseAshCards { of: CardFilter, n: u8 },
    /// A Methuselah or a card, both with one of the `both` titles (Cold
    /// War).
    SeatOrCard {
        card: CardFilter,
        both: &'static [TitleClass],
    },
    /// Allocate `per` points per minion chosen by the previous term among
    /// Methuselahs, locations and equipment (Revolutionary Council).
    AllocatePerChosenWithCards { per: u8 },
    /// Distribute blood among vampires, one point a pick, a vampire picked
    /// at most `per_max` times if not 0 (Dmitra Ilyanova, Free States Rant).
    DistributeBlood {
        points: Points,
        per_max: u8,
        filter: &'static [Requirement],
    },
}

/// What `Effect::MoveBetweenVampires` moves, and between whom.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Moves {
    /// Retainers too, not only blood and equipment.
    pub retainers: bool,
    /// Both vampires of this sect (Communal Haven: Cathedral).
    pub sect: Option<Sect>,
}

/// What a passed referendum does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RefEffect {
    /// The calling vampire takes this title (Horatio Ballard, Maxwell).
    CallerTitle(crate::crypt::Title),
    /// Each Methuselah burns 1 pool per point allocated.
    BurnAllocated,
    /// Each Methuselah burns 1 pool per minion they control (`locked_only`:
    /// per locked minion, counted when the effect applies).
    BurnPerMinion { locked_only: bool },
    /// Every vampire in play, torpor too, of current bleed `n` or more is
    /// burned, read as it passes (Justicar Retribution).
    BurnVampiresBleedAtLeast(u8),
    /// The calling vampire and the chosen minions unlock (My Kin Against
    /// the World).
    UnlockCallerAndChosen,
    /// Each chosen Methuselah burns `base` pool, plus `young_bonus` if they
    /// control a ready vampire with capacity `young_cap` or less.
    BurnChosen {
        base: u8,
        young_bonus: u8,
        young_cap: u8,
    },
    /// As `BurnAllocated`, and the chosen Methuselah gains n pool.
    BurnAllocatedChosenGains(u8),
    /// As `BurnAllocated`, and the caller looks at the chosen Methuselah's
    /// hand and discards up to n cards of it (Information Exchange).
    BurnAllocatedChosenDiscardsUpTo(u8),
    /// The allocated points move from the chosen Methuselah's pool to the
    /// recipients (Parity Shift).
    MoveAllocatedFromChosen,
    /// Each Methuselah gains 1 pool per vampire of the chosen clan they
    /// control.
    GainPerClanVampire,
    /// Each Methuselah gains 1 pool per clan among their ready vampires
    /// (Diversity).
    GainPerReadyClan,
    /// Each chosen Methuselah burns n pool, each other one gains n
    /// (Finding the Path).
    ChosenBurnOthersGain(u8),
    /// Each ready Anarch gains 1 blood; each Methuselah controlling an
    /// Anarch gains 1 pool.
    AnarchSalon,
    /// The chosen vampire goes to its controller's uncontrolled region.
    Banish,
    /// The chosen minions cannot play reaction cards, block or cast votes
    /// or ballots this turn.
    Expel,
    /// Burn the card in play whose granted action called the referendum.
    BurnSource,
    /// The political action card is put on the chosen vampire, if still in
    /// play (Toreador Justicar: its title card); `equipment`: its caller
    /// may then search their library for an equipment to put on it,
    /// requirements ignored, for half its cost rounded down (Alastor;
    /// `Interrupt::SearchEquipFor`).
    PutOnChosen { equipment: bool },
    /// The political action card put on the vampire who called it, still in
    /// play (Praxis Seizure: York); `unlocks_if`: of that clan at the next
    /// discard phase, it unlocks then (Crusade: Istanbul).
    PutOnCaller { unlocks_if: Option<&'static str> },
    /// The political action card is put in play, its caller's (Raising the
    /// Portcullis).
    PutInPlay,
    /// As `PutInPlay`, locking every vampire with capacity below n (Snipe
    /// Hunt).
    PutInPlayLockBelow(u8),
    /// The vampire granting the action is burned (Theo Bell, advanced).
    BurnSourceVampire,
    /// n damage on each ready minion with any of the caller's corruption
    /// counters (Kahina the Sorceress).
    DamageYourCorrupted(u8),
    /// The chosen Methuselah takes the chosen card (Disputed Territory): one
    /// on a minion moves onto one of theirs (RTR 19960112), not when it
    /// stays with the same Methuselah (LSJ 19971002).
    ChosenTakesChosenCard,
    /// Lock each chosen minion, burn 1 pool per point allocated, burn each
    /// card allocated a point (Revolutionary Council).
    LockChosenBurnAllocated,
    /// Each Methuselah can choose a ready vampire (`RefStep::Choose`), gains
    /// its capacity in pool, burns n (Ancient Influence).
    GainChosenCapacityBurn(u8),
    /// Each Methuselah can choose a ready vampire of theirs, gains n pool,
    /// burns their predator's chosen capacity (Reins of Power).
    GainBurnPredatorsChosenCapacity(u8),
    /// Each Methuselah gains n pool per vampire of capacity `cap` or more
    /// (Political Stranglehold).
    GainPerVampireCapacityAtLeast { cap: u8, n: u8 },
    /// Each Methuselah gains n pool per minion, allies and torpor included
    /// (Autarkis Persecution).
    GainPerMinion(u8),
    /// The chosen minion is removed from the game (Permanent Vacation).
    RemoveChosen,
    /// The chosen card moves onto the chosen vampire (Confiscation).
    ChosenVampireTakesChosenCard,
    /// Each chosen Methuselah shuffles up to n ash heap library cards into
    /// their library (Reinforcements).
    ChosenShuffleFromAsh(u8),
    /// The chosen Methuselah burns n pool and the chosen card is burned
    /// (Cold War).
    ChosenBurnsPoolCardBurned(u8),
    /// The calling vampire's controller gains n pool (Black Forest Base).
    CallerGainsPool(u8),
    /// The chosen vampire takes as much damage as it cast votes in the
    /// previous referendum (Tribunal Judgment).
    ChosenDamagedByItsVotes,
    /// The chosen minion, still ready, takes n unpreventable damage
    /// (Mamadou Keita).
    DamageChosen(u8),
    /// The ash heap cards named (`Term::ChooseAshCards`), a copy of each
    /// still there, go to the caller's hand, discarded down after (Undele;
    /// LSJ 20100917).
    AshChosenToHand,
    /// Each Methuselah gains 1 pool per ready vampire of these sects they
    /// control (Exclusion Principle).
    PoolPerReadySect(&'static [Sect]),
    /// The caller gains 1 pool per ready vampire of the clan they control;
    /// each vampire of another clan with votes or ballots standing in favor
    /// gains 1 blood (Benefit Performance).
    CallerPoolPerReadyClanVotersBlood(&'static str),
    /// Each chosen vampire still in play gains n blood; `put_in_play`, the
    /// card enters play too (Fanfare for Elysium).
    ChosenGainBlood { n: u8, put_in_play: bool },
    /// Burn the Edge, if still the caller's controller's, to untitle the
    /// vampire and burn n of its controller's pool (Patsy).
    BurnEdgeToUntitle(u8),
    /// The `DistributeBlood` points, from the bank, to vampires still
    /// ready; what finds no room is lost.
    BloodToChosenEach,
    /// The `DistributeBlood` points moved from the first chosen vampire,
    /// still ready, to the vampires picked; short of blood, the caller
    /// picks which receive (Public Vilification; LSJ 20010606).
    MoveDistributedFromFirst,
    /// +n hand size for the game for the caller, per passed referendum
    /// (Benjamin Rose).
    CallerHandSizeForGame(u8),
    /// The calling vampire receives the chosen vampire's title and contests
    /// it with them (Zane): both still in play, the chosen still a prince.
    CallerContestsChosenTitle,
    /// Each chosen Methuselah burns `base` pool, plus `bonus` if they
    /// control a ready titled vampire (Eat the Rich).
    BurnChosenPlusIfTitled { base: u8, bonus: u8 },
    /// Each chosen Methuselah burns `base` pool, plus `bonus` if they
    /// control a ready vampire with capacity `cap` or more (Empires Fall).
    BurnChosenPlusIfBig { base: u8, bonus: u8, cap: u8 },
    /// Burn all allies; each Methuselah (`RefStep::Choose`) keeps theirs by
    /// repaying their pool cost, 0 for a blood cost or none (Kindred
    /// Segregation; RTR 20010711, LSJ 20060409; ANK 20190724).
    BurnAlliesUnlessKept,
    /// As `BurnAlliesUnlessKept`, the weapons on each Methuselah's minions
    /// (Peace Treaty; RTR 20010710: the pool cost only; ANK 20190724: one
    /// with no pool cost may be kept or burned).
    BurnWeaponsUnlessKept,
    /// Each vampire picked by `DistributeBlood` burns 1 blood per point
    /// (Free States Rant).
    BurnBloodPerPick,
}

impl RefEffect {
    /// Burned unless each Methuselah repays its pool cost (`RefStep::Choose`
    /// one keep at a time).
    pub fn keeps_by_repaying(self) -> bool {
        matches!(
            self,
            RefEffect::BurnAlliesUnlessKept | RefEffect::BurnWeaponsUnlessKept
        )
    }

    /// Each Methuselah chooses a ready vampire they control once the
    /// referendum passes (rules-qa 49.1).
    pub fn chooses_vampires(self) -> bool {
        matches!(
            self,
            RefEffect::GainChosenCapacityBurn(_) | RefEffect::GainBurnPredatorsChosenCapacity(_)
        )
    }
}

/// A political action card's referendum: its terms, its effect, and the
/// votes it grants while it lasts (Toreador Justicar).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ReferendumDef {
    pub terms: &'static [Term],
    pub effect: RefEffect,
    pub grants: &'static [VoteGrant],
    /// Non-Anarch titles worth -1 vote (Fee Stake: Boston).
    pub non_anarch_titles_minus: bool,
    /// Priscus -1 ballot, other titled -1 vote, read live as votes are cast
    /// (Eat the Rich).
    pub titled_minus: bool,
    /// Burning the Edge worth n more votes (Eat the Rich).
    pub edge_bonus: u8,
    /// Only this sect's vampires vote (Cardinal Benediction).
    pub only_sect: Option<Sect>,
}

/// Whom a referendum's vote penalty reaches (`ReferendumCtx::penalties`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Penalized {
    /// Vampires not of the sect (Camarilla Conclave).
    NotSect(Sect),
    /// Vampires not following the path (Absolute Tyranny [POT][PRE]).
    NotPath(&'static str),
}

impl Penalized {
    pub fn reaches(self, st: &GameState, m: MinionId) -> bool {
        match self {
            Penalized::NotSect(sect) => st.sect_of(m) != Some(sect),
            Penalized::NotPath(path) => !st.follows(m, path),
        }
    }
}

/// The vampires a vote grant reaches, read live while the referendum lasts
/// (one that has already cast adds them to its cast).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Voters {
    /// A clan's vampires (Toreador Justicar, Power Structure).
    Clan(&'static str),
    /// Titled vampires of the sect, effective title and sect (Elysium: The
    /// Palace of Versailles).
    TitledSect(Sect),
    /// Vampires of the clan and sect, effective (The Eldest Are Kholo).
    ClanSect(&'static str, Sect),
}

/// +n votes to each vampire `voters` names (of the granting Methuselah's
/// when the grant comes from a card in play).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VoteGrant {
    pub voters: Voters,
    pub n: u8,
}

impl Voters {
    pub fn reach(self, st: &GameState, m: MinionId) -> bool {
        let v = &st.minions[m.index()];
        match self {
            Voters::Clan(c) => v.clan() == Some(c),
            Voters::TitledSect(sect) => {
                st.sect_of(m) == Some(sect) && st.title_of(m) != Title::None
            }
            Voters::ClanSect(c, sect) => v.clan() == Some(c) && st.sect_of(m) == Some(sect),
        }
    }
}

impl ReferendumDef {
    /// Can the terms be met (LSJ 20100129)? A choice of Methuselah only if
    /// some choice leaves the following allocation feasible.
    /// `mode`: the card's or ability's, read for Vlad Tepes's as if.
    pub fn feasible(&self, st: &GameState, seat: SeatId, actor: MinionId, mode: &ModeDef) -> bool {
        if st.seats[seat.index()].ousted {
            return false;
        }
        let as_if = meets_as_if(st, actor, mode);
        let living = st.clockwise_from(seat);
        let mut i = 0;
        while i < self.terms.len() {
            match self.terms[i] {
                Term::ChooseSeat(filter) => {
                    let next = self.terms.get(i + 1).copied();
                    let ok = living.iter().any(|&c| {
                        seat_filter_holds(filter, st, seat, c)
                            && match next {
                                Some(Term::Allocate {
                                    among, min_seats, ..
                                }) => recipients(st, seat, among, Some(c)).len() as u8 >= min_seats,
                                _ => true,
                            }
                    });
                    if !ok {
                        return false;
                    }
                    if matches!(next, Some(Term::Allocate { .. })) {
                        i += 1;
                    }
                }
                Term::Allocate {
                    among, min_seats, ..
                } => {
                    if (recipients(st, seat, among, None).len() as u8) < min_seats {
                        return false;
                    }
                }
                Term::ChooseSeats { min, .. } => {
                    if (living.len() as u8) < min {
                        return false;
                    }
                }
                // At least half of them: always feasible.
                Term::ChooseSeatsHalfOrMore => {}
                // One no copy in play chose (LSJ 20100129, ANK 20191228).
                Term::ChooseSeatsUnchosen(name) => {
                    let taken = |s: SeatId| {
                        st.cards.iter().any(|c| {
                            c.present()
                                && LIBRARY[c.card as usize].name == name
                                && c.against.contains(s)
                        })
                    };
                    if living.iter().all(|&s| taken(s)) {
                        return false;
                    }
                }
                Term::ChooseMinions { filter, .. } | Term::DistributeBlood { filter, .. } => {
                    // A distribution needs one recipient at least; capped
                    // per vampire, enough to take every point (Free States
                    // Rant; LSJ 20010810).
                    let min = match self.terms[i] {
                        Term::ChooseMinions { min, .. } => min,
                        Term::DistributeBlood {
                            points, per_max, ..
                        } => {
                            let x = match points {
                                Points::HalfActorCapacity => {
                                    st.capacity_of(actor).unwrap_or(0).div_ceil(2)
                                }
                                Points::Fixed(n) | Points::PoolOfChosen(n) => n,
                                Points::Methuselahs => st.living() as u8,
                            };
                            if per_max > 0 {
                                x.div_ceil(per_max).max(1)
                            } else {
                                1
                            }
                        }
                        _ => unreachable!("a minion term"),
                    };
                    let n = (0..st.minions.len())
                        .map(|i| MinionId(i as u32))
                        .filter(|&m| {
                            st.minions[m.index()].present()
                                && filter.iter().all(|&r| {
                                    term_filter_holds(
                                        r,
                                        st,
                                        seat,
                                        None,
                                        Some(actor),
                                        m,
                                        None,
                                        as_if,
                                    )
                                })
                        })
                        .count();
                    if (n as u8) < min {
                        return false;
                    }
                }
                Term::ChooseAshCards { of, n } => {
                    let ash = &st.seats[seat.index()].ash;
                    if ash.iter().filter(|&&c| of.matches(c)).count() < n as usize {
                        return false;
                    }
                }
                // As many as qualify, up to X (none: the term is passed).
                Term::ChooseClan
                | Term::AllocatePerChosenWithCards { .. }
                | Term::ChooseMinionsX { .. } => {}
                // A Methuselah is always there to choose.
                Term::SeatOrCard { .. } => {}
                Term::ChooseCard(filter) => {
                    if !(0..st.cards.len())
                        .any(|i| crate::referendum::chooseable_card(st, CardId(i as u32), filter))
                    {
                        return false;
                    }
                }
            }
            i += 1;
        }
        true
    }
}

/// Methuselahs eligible to receive points, clockwise from the caller.
pub fn recipients(
    st: &GameState,
    caller: SeatId,
    among: Among,
    chosen: Option<SeatId>,
) -> Vec<SeatId> {
    st.clockwise_from(caller)
        .into_iter()
        .filter(|&s| match among {
            Among::Any => true,
            Among::Others => s != caller,
            Among::AnyNotChosen => Some(s) != chosen,
            Among::OthersNotChosen => s != caller && Some(s) != chosen,
        })
        .collect()
}

pub fn seat_filter_holds(filter: SeatFilter, st: &GameState, caller: SeatId, s: SeatId) -> bool {
    match filter {
        SeatFilter::Any => true,
        SeatFilter::MorePoolThanCaller => st.seats[s.index()].pool > st.seats[caller.index()].pool,
        SeatFilter::Other => s != caller,
        // Needs the allocation: filtered where the term is offered
        // (`ReferendumCtx` options), any other reader answered true.
        SeatFilter::Allocated => true,
    }
}

/// A strike (rulebook "Strike Effects"), from the hands, a card or a weapon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StrikeKind {
    /// Hand strike: strength (+bonus) damage at close range.
    Hand { bonus: u8 },
    /// A melee weapon's strength+n damage: close range, not a hand strike.
    Melee { bonus: u8, aggravated: bool },
    /// Defensive: the opposing strike has no effect on the dodging minion.
    Dodge,
    /// Combat ends before any other strike resolves; `unlock`: the striker
    /// unlocks first.
    CombatEnds { unlock: bool },
    /// Fixed damage; `ranged` works at any range.
    Damage {
        n: u8,
        ranged: bool,
        aggravated: bool,
    },
    /// Move up to n blood from the opposing minion to the striker, before
    /// damage is mended.
    StealBlood { n: u8, ranged: bool },
    /// Torpor a vampire (wounded, LSJ 20090622-2) or burn an ally, close
    /// range (rules-qa 21.1), unpreventable (Touch of Oblivion [OBL]).
    TorporOrBurn,
    /// Torpor a vampire (wounded, LSJ 20090622-2), an ally untouched, close
    /// range (Coma [dem]); `skip_unlock` ([DEM]): no next unlock.
    Torpor { skip_unlock: bool },
    /// A strike whose effect is its card's rider (Touch of Oblivion [obl]):
    /// no damage, close range unless `ranged` (rules-qa 21.1; Molotov
    /// Cocktail's "Ranged strike"), dodged like any strike.
    CardEffect { ranged: bool },
    /// Steal the weapon named with the strike (`Side.strike_weapon`), close
    /// range (rules-qa 21.1), if still there and holdable (Fast Hands).
    StealWeapon,
    /// Ranged: burn the weapon named with the strike, if still there
    /// (Voracious Vermin [ANI]).
    BurnWeapon,
    /// Burn the equipment or retainer named with the strike, close range
    /// only (rules-qa 21.1), if still there (Heroic Might, Autonomic Mastery
    /// [dom]).
    BurnEquipment,
    /// As `BurnEquipment`, at either range (Canine Horde [ANI], Morlock).
    RangedBurnEquipment,
    /// 1 corruption counter on the opponent, enough of them to steal it
    /// (Contagion [DAI]): close range, dodged like any strike (ANK 20200909).
    Corrupt,
    /// Strength (+bonus) damage at any range, not a hand strike
    /// (Earthshock).
    RangedStrength { bonus: u8 },
    /// Burn n life from the opposing ally, at any range: no damage, so no
    /// prevention (Argent Baton against a werewolf).
    BurnLife { n: u8 },
    /// Ranged n aggravated damage to the vampires, demon allies and demon
    /// retainers in combat, the striker included (normal on it if
    /// `self_normal`), a dodge sparing only the dodger's side (Eye of
    /// Unforgiving Heaven).
    Area { n: u8, self_normal: bool },
    /// Ranged normal damage: the striker's capacity minus the opposing
    /// minion's capacity or cost, minus `minus`, read as it resolves, none
    /// below 0, at most `cap` (Scourge of Alecto).
    CapacityGap { minus: u8, cap: u8 },
}

impl StrikeKind {
    /// Usable at long range (Shadowed Eyes' "ranged strikes"; a gun's).
    pub fn ranged(self) -> bool {
        match self {
            StrikeKind::Damage { ranged, .. }
            | StrikeKind::StealBlood { ranged, .. }
            | StrikeKind::CardEffect { ranged } => ranged,
            StrikeKind::BurnWeapon
            | StrikeKind::RangedBurnEquipment
            | StrikeKind::RangedStrength { .. }
            | StrikeKind::BurnLife { .. }
            | StrikeKind::Area { .. }
            | StrikeKind::CapacityGap { .. } => true,
            StrikeKind::Hand { .. }
            | StrikeKind::Melee { .. }
            | StrikeKind::Dodge
            | StrikeKind::CombatEnds { .. }
            | StrikeKind::TorporOrBurn
            | StrikeKind::Torpor { .. }
            | StrikeKind::StealWeapon
            | StrikeKind::BurnEquipment
            | StrikeKind::Corrupt => false,
        }
    }
}

/// "Only one X each round / combat / action": one play of the card name per
/// minion per scope, a canceled play counting (Immortal Grapple rulings).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Limit {
    PerRound,
    PerCombat,
    PerAction,
    /// Once each action per copy, the card's abilities so limited sharing
    /// the use (Camarilla Conclave; rules-qa 51.2).
    PerActionCard,
    /// Each mode of a "during X" clause its own once-per-phase use, not one
    /// per copy (Carfax Abbey; rules-qa 6.6).
    PerPhaseEach,
    /// Once per referendum, unlocking mid-vote not restoring it (advanced
    /// rules §1.2.1).
    PerReferendum,
    /// As `PerReferendum`, the source's abilities so limited sharing the
    /// use (Sean Rycek's lock or blood).
    PerReferendumCard,
    /// Once per referendum for each minion using it (Tegyrius' grant to
    /// each Camarilla Banu Haqim, Rant!'s to each ready anarch).
    PerReferendumEachUser,
    /// Once each Methuselah's turn per copy or vampire ability (advanced
    /// rules §1.2.1); on a card mode per minion and card (Forced March).
    PerTurn,
    /// One play of the card each action, by any minion (Veil the Legions).
    OnePerAction,
    /// Once each combat per minion and card mode (Terror Frenzy).
    PerCombatThisMode,
    /// Once each combat per copy, the card's abilities so limited sharing
    /// the use (Dance of the Dead).
    PerCombatCard,
    /// Once each Methuselah's turn, the source's abilities so limited
    /// sharing the use (Tyler: "+1 bleed or an additional vote").
    PerTurnCard,
    /// Once each game per copy or vampire ability (Qadir al-Asmai).
    PerGame,
    /// Twice each combat per copy (Deer Rifle).
    TwicePerCombat,
    /// Once per minion and card name until its controller's next unlock
    /// phase (On the Qui Vive).
    BetweenUnlockPhases,
    /// One play of the card in this mode each action, by any minion, a
    /// canceled one counted (Hide the Heart [val]; rules-qa 46.1).
    OnePerActionThisMode,
    /// Once each round for the card, whichever of its abilities (Tinglestripe:
    /// "used to strike only once each round").
    PerRoundCard,
}

/// Which Blood Brothers of the actor's circle a `Terms::CirclePicks`
/// offers (all ready, controlled by the player).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CirclePick {
    /// Others, unlocked: they lock (Shell Game).
    OthersUnlocked,
    /// Others (Coagulated Entity).
    Others,
    /// The actor included, with 1 blood or more: they burn it (Walk of
    /// Caine).
    WithBloodSelfIncluded,
    /// The actor included (Gestalt).
    SelfIncluded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TargetKind {
    Minion,
    Seat,
    /// A vampire in the player's uncontrolled region (slot index).
    Uncontrolled,
    /// A library card in play (equipment to steal, a card to burn).
    Card,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TargetSpec {
    pub kind: TargetKind,
    pub filter: &'static [Requirement],
}

/// A resolved target of a play.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Target {
    #[default]
    None,
    Minion(MinionId),
    Seat(SeatId),
    /// A slot of the player's uncontrolled region.
    Uncontrolled(u8),
    /// A library card in play: the action is directed at its controller
    /// unless that is the acting Methuselah (rules-qa 17.1), not at the
    /// minion it is on (rulebook "Target").
    Card(CardId),
}

impl Target {
    /// The minion targeted, where the mode's target spec makes one.
    #[track_caller]
    pub fn minion(self) -> MinionId {
        match self {
            Target::Minion(m) => m,
            _ => unreachable!("{self:?}: no minion target"),
        }
    }

    /// The card in play targeted, where the mode's target spec makes one.
    #[track_caller]
    pub fn card(self) -> CardId {
        match self {
            Target::Card(c) => c,
            _ => unreachable!("{self:?}: no card target"),
        }
    }
}

/// Where an ability used with `Choice::UseAbility` is printed: a card in
/// play (arena index) or a vampire's own text (`cards::vampires`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AbilitySource {
    Card(CardId),
    Vampire(MinionId),
}

impl AbilitySource {
    /// The Methuselah who played its card or vampire, whoever controls it
    /// now: the reader Orun compares with the Laibon's controller (vampires
    /// are played cards, LSJ 20090205; a card in play knows who played it,
    /// LSJ 20090626; a stolen minion's ability reads as its player's, LSJ
    /// 20090930).
    pub fn player(self, st: &GameState) -> SeatId {
        match self {
            AbilitySource::Card(c) => st.cards[c.index()].owner,
            AbilitySource::Vampire(m) => st.minions[m.index()].owner_seat(),
        }
    }

    /// The card in play of an ability whose effects act on that card.
    pub fn card(self) -> CardId {
        match self {
            AbilitySource::Card(c) => c,
            AbilitySource::Vampire(m) => {
                unreachable!("an effect on its card in play, used from vampire {m:?}")
            }
        }
    }

    /// The key an "each action / combat / round" limit counts by: per copy
    /// (Kevlar Vest, rules-qa 10.3), per vampire ability (Abaddon).
    pub fn use_key(self, ability: u8) -> (AbilitySource, u8) {
        match self {
            AbilitySource::Card(_) => (self, 0),
            AbilitySource::Vampire(_) => (self, ability),
        }
    }

    /// The key a "during an action" limit counts by: per ability of a copy
    /// (Bone Shambler, rules-qa 42.9).
    pub fn action_key(self, ability: u8) -> (AbilitySource, u8) {
        (self, ability)
    }

    /// The NRA identity of the action this ability grants (spec §3.8.1), one
    /// split by target kind keeping one (`Effect::SameActionAs`, Aniel).
    pub fn action_id(self, st: &crate::state::GameState, ability: u8) -> crate::state::ActionId {
        let same = self.mode(st, ability).effects.iter().find_map(|e| match e {
            Effect::SameActionAs(n) => Some(*n),
            _ => None,
        });
        match self {
            AbilitySource::Card(c) => crate::state::ActionId::InPlay(c, same.unwrap_or(ability)),
            AbilitySource::Vampire(m) => {
                crate::state::ActionId::Printed(m, same.unwrap_or(ability))
            }
        }
    }

    /// The definition of ability `ability`, wherever it is printed.
    pub fn mode(self, st: &crate::state::GameState, ability: u8) -> &'static ModeDef {
        &self.abilities(st)[ability as usize]
    }

    /// The abilities printed where this one is.
    pub fn abilities(self, st: &crate::state::GameState) -> &'static [ModeDef] {
        match self {
            AbilitySource::Card(c) => {
                lookup(st.cards[c.index()].card)
                    .expect("definition")
                    .abilities
            }
            AbilitySource::Vampire(m) => {
                vampire_of(&st.minions[m.index()])
                    .expect("vampire definition")
                    .abilities
            }
        }
    }
}

/// Numeric terms chosen as the card is played (rules-qa 3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Terms {
    /// An integer in `min..=max`, what is taken at resolution capped by
    /// what is there (Villein rulings).
    BloodRange { min: u8, max: u8 },
    /// 0 up to the target's blood as played, what is left taken at
    /// resolution (Minion Tap; rules-qa 38.1).
    UpToTargetBlood,
    /// X blood, 0 up to the actor's, more than needed allowed (Hidden
    /// Strength).
    XBlood,
    /// X in `min..=max`, paid in blood, never above the actor's (Monkey
    /// Wrench).
    XBloodIn { min: u8, max: u8 },
    /// X convictions, 0 up to the actor's (Living Wood Staff; X never
    /// below 0, LSJ 20030502).
    XConvictions,
    /// X, 0 up to the actor's unlocked cards of this kind (Brutal
    /// Influence: Orun).
    XUnlockedHeld(Holding),
    /// X from half the target's capacity or cost, rounded up, to the
    /// actor's unlocked cards of this kind (Exile [PRE]: Orun).
    XHeldForTarget(Holding),
    /// Up to n of the votes the referendum passed by (Voter Captivation
    /// [PRE]).
    UpToMargin(u8),
    /// The copies of this card controlled, read as the cost is paid
    /// (Creeping Sabotage).
    CopiesControlled,
    /// X: the ready unlocked vampires of this clan the player controls,
    /// counted as the card is played (Ensemble).
    OwnReadyUnlockedOfClan(&'static str),
    /// A copy in play of the name, any controller's, or none, announced
    /// like a target, burned at resolution (Vessel; rules-qa 6.7).
    BurnNamedInPlay(&'static str),
    /// Discard n cards of `of`, one at a time as the use is announced
    /// (Tension in the Ranks, The Barrens, Kasim Bayar, Alexa Draper).
    Discard { n: u8, of: CardFilter },
    /// Discard X cards of `of`, one pick at a time, Pass ending after one;
    /// X the number discarded (Angelica, LSJ 19981005).
    DiscardAny { of: CardFilter },
    /// A count in `min..=max`, chosen as the ability is used (Heart of
    /// Nizchetus; rules-qa 11.1).
    Count { min: u8, max: u8 },
    /// Blood from the target to the host, 1 up to what both allow (the
    /// host's starting life; The Vozhd of Juiz de Fora).
    LifeFromTarget,
    /// A card in the user's ash heap, chosen by name as the ability is used
    /// (Mora, the Death Seer).
    AshCard,
    /// A card the host played in the combat just ended, still in the
    /// user's ash heap, named as the ability is used, to hand (Marthe
    /// Dizier; `ActionCtx::combat_played`).
    CombatPlayedAshCard,
    /// A card with two copies in the prey's ash heap, named as announced,
    /// both removed at resolution (Giovanni del Georgio).
    PreyAshPair,
    /// Another vampire of the user's, torpor too, able to hold the target
    /// card, which moves onto it (Luciano Carvalho, rules-qa 41.7).
    VampireToHoldTarget,
    /// X pool, 1 to the user's pool (Oxford University, England).
    XPool,
    /// X counters, 1 to the card's counters (Alamut).
    XCounters,
    /// An ally in any ash heap, the ash heap then the ally, named as played
    /// (Psychophagia; rules-qa 32.12).
    AnyAshAlly,
    /// As `AnyAshAlly`, an ally burned from play (`Seat::burned_allies`;
    /// Pressing Flesh, LSJ 20100325-2).
    AnyAshBurnedAlly,
    /// A retainer from hand matching the filter to employ, card and version
    /// named as played (Pack Alpha [ani]: an animal; LSJ 20100130; Nosferatu
    /// Bestial: one requiring Animalism).
    EmployFromHand(CardFilter),
    /// As `EmployFromHand`, requirements ignored, the basic version always
    /// (Biothaumaturgic Experiment [tha]; LSJ 20080803).
    EmployFromHandIgnoring(CardFilter),
    /// An ally or retainer from hand to recruit or employ, named as played
    /// (Piper).
    RecruitOrEmployFromHand,
    /// A card of the directed-at Methuselah's hand, by name, the choice
    /// showing the hand; discarded at resolution (Richard de Worde).
    DirectedHandCard,
    /// Equipment from hand, named as played, outside an action (Angel's
    /// Gift, Contraband; LSJ 20100130); `blood_share`: up to half its pool
    /// cost paid in blood (Contraband [OBF]).
    EquipFromHand {
        filter: CardFilter,
        blood_share: bool,
    },
    /// The prey's vampire is the target, the predator's `PlayCtx::picks[0]`,
    /// both targets (Taunt the Caged Beast [ANI]; rules-qa 71.21).
    TauntPair,
    /// "Up to two": a second minion meeting the mode's own target filter,
    /// or none (Pass), picked as played (`PlayCtx::picks[0]`, the action
    /// directed at its Methuselah too: Liberty Club Intrigue);
    /// `same_controller`: the first's controller's (Shattering Crescendo).
    OptionalSecondTarget { same_controller: bool },
    /// A ready vampire of the clan the user controls, as played; `other`:
    /// not the target (Priority Contract, Toreador Grand Ball; rules-qa 28.5).
    OwnOfClan { clan: &'static str, other: bool },
    /// An ally of these subtypes the user controls, the target too
    /// (`PlayCtx::chosen`: Funeral Wake [NEC]'s "up to two", the target
    /// again picking none more).
    OwnAllyOfSubtype(&'static [crate::library::Label]),
    /// X up to the animals in hand, chosen as the card is played (Call the
    /// Wild Hunt [ANI][PRO]).
    XAnimalsInHand,
    /// Any Methuselah's Black Hand vampire in play, chosen as the card is
    /// played (Hand Contract; rules-qa 28.5).
    AnyBlackHandVampire,
    /// X Blood Brothers of the actor's circle, picked one at a time as
    /// played, Pass stopping (`PlayCtx::picks`; Shell Game, Walk of Caine,
    /// Gestalt, Coagulated Entity).
    CirclePicks(CirclePick),
    /// Up to n other minions the player controls, torpid ones too (D28),
    /// picked one at a time as played, Pass stopping (`PlayCtx::picks`;
    /// Blessing of the Name).
    OwnOtherMinionsUpTo(u8),
    /// A superior Discipline of the actor's the target lacks at superior
    /// (Sanguine Instruction; `PlayCtx::discipline`).
    SuperiorDiscipline,
    /// Any superior Discipline of the actor's (Field Training: a target
    /// already superior gains nothing, a legal no-op).
    ActorSuperiorDiscipline,
    /// Any Discipline the target lacks at superior (Spontaneous Power;
    /// `PlayCtx::discipline`).
    AnyDiscipline,
    /// A Discipline the ability's host vampire has at inferior, chosen as
    /// the ability is used (Camarilla Vitae Slave).
    InferiorDiscipline,
    /// Which of the actor's recent blood gains the card answers, by amount
    /// (`PlayCtx::x`; Slake the Thirst, LSJ 20100203).
    ActorGain,
    /// How many cards of the filter in hand the card takes as it resolves,
    /// from none to all (`PlayCtx::x`; Gift of Proteus, PIB 20150428).
    HandCards(CardFilter),
    /// Life Boon's gift: from the target's debt plus 1 up to leaving its
    /// player 1 pool.
    BoonPool,
    /// X is the minion target's capacity, no decision (Chameleon).
    TargetCapacity,
    /// A burned vampire in the user's ash heap, by crypt card, chosen as
    /// the card is played (Redeem the Lost Soul; `PlayCtx::ash`).
    OwnAshVampire,
    /// `OwnAshVampire` burned from play, with these superior Disciplines
    /// printed (Blessed Resilience).
    OwnAshBurnedVampire(&'static [&'static str]),
    /// A vampire of the player's uncontrolled region meeting the filter
    /// read against the minion target, by slot, chosen after the target as
    /// the card is played (`PlayCtx::commanded`'s first: Tend the Flock,
    /// Thicker than Blood).
    OwnUncontrolled(&'static [Requirement]),
    /// n ash heap library cards, named one at a time as played, a name
    /// again only for another copy (Sudario Refraction).
    AshCards(u8),
    /// X (+ `more`) younger uncontrolled vampires named one at a time as
    /// played, X paid in blood (`PlayCtx::commanded`; Chain of Command, D1).
    CommandPicks,
    /// The Hold's card of the target's ash heap, named as played
    /// (`PlayCtx::ash`; Cold Amber's Hold, D1, rules-qa 60.17).
    HoldCardOfTarget,
    /// X: the copies of this card already played in this combat, by either
    /// combatant, canceled ones too, no decision (Groaning Corpse).
    NamePlayedThisCombat,
}

/// The type of card a Hold played in `mode` takes from an ash heap.
pub fn hold_of(mode: &ModeDef) -> Option<CardType> {
    mode.effects.iter().find_map(|e| match e {
        Effect::GiveHold { of, .. } => Some(*of),
        _ => None,
    })
}

/// A bonus lasting on a minion until an expiry (`Minion::timed`): Dive
/// into Madness [dem], Gear Up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lasting {
    Stealth(i8),
    Strength(i8),
    /// Superior level in this Discipline (lower case).
    Superior(&'static str),
    /// Its hand strikes' damage is aggravated (Mantle of the Bestial
    /// Majesty).
    AggravatedHands,
    /// Bleed added (Andre LeRoux).
    Bleed(i8),
    /// Unlocks as `next_turn` sweeps it, pushed with `Expiry::EndOfTurn`
    /// only (`GameState::unlock_at_end_of_turn`: Enkidu, The Noah's
    /// employ, NRA PAC's equipping).
    UnlockAtEndOfTurn,
    /// May unlock as the minion phase ends, pushed with `Expiry::EndOfTurn`
    /// (Code of Samiel).
    MayUnlockAfterMinionPhase,
}

/// Continuous-effect modifiers carried by layers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Modifier {
    /// +n votes for the host in referendums younger vampires call
    /// (Paragon).
    HostVotesWhenCallerYounger(u8),
    /// Its host gets a vote per counter on the card in referendums it
    /// calls (Aura of Invincibility).
    HostVotesPerCounterWhenCalling,
    /// Its host gets a vote per Gehenna card in play in each political
    /// action's referendum (Emergency Powers).
    HostVotesPerGehennaInPolitics,
    /// The card burns once no Gehenna card is in play (Emergency Powers).
    GehennaBound,
    /// Its host gets +X strength each round, X the damage successfully
    /// inflicted on it the previous round of the combat (Ecstatic Agony).
    HostStrengthPerLastRoundDamage,
    /// Its host's controller draws a card as each press step of its combat
    /// begins, then discards down to hand size (Improvised Tactics [aus]).
    DrawEachPressStep,
    /// Each strike its bearer makes at close range is with this weapon if
    /// it can strike (Sword of Troile).
    CloseStrikesWithThis,
    /// Its bearer cannot maneuver to long range (Sword of Troile).
    BearerNoManeuverToLong,
    /// The damage of this weapon's strikes is mended for n blood a point
    /// (Sword of Nuln).
    MendingCost(u8),
    /// Cards named as the card on this one cost its controller's vampires
    /// n more blood (the Holds).
    StoredCardsCostBlood(u8),
    /// A vampire of its controller's blocking a Camarilla vampire, the
    /// card may lock instead of it (Elysium: Sforzesco Castle;
    /// `Interrupt::MayLockCardForBlock`).
    LocksInsteadOfBlockerOfCamarilla,
    /// A directed action at its bearer costs the acting Methuselah n more
    /// pool as it resolves (Alia, God's Messenger; Ambrosio's printed
    /// `Stat::DirectedAtCostsPool`).
    DirectedAtBearerCostsPool(u8),
    /// Its bearer has subtype `is` instead of `not` (Ghouled: a ghoul, not
    /// a mortal).
    BearerSubtypeInstead {
        is: crate::library::Label,
        not: crate::library::Label,
    },
    /// Once each combat, a strike with this weapon at close range not
    /// dodged gives its bearer an optional press that round, only to
    /// continue (Meat Hook).
    ContinuePressOnHitOnce,
    /// Its bearer gets 1 optional press to end combat the first round
    /// (Highway Haven: RV).
    BearerFirstRoundEndPress,
    /// Its bearer anarch, a vampire attempting an action directed at it
    /// does not unlock as normal next (Highway Haven: RV).
    ActingAtAnarchBearerSkipsUnlock,
    /// Its controller ousted or withdrawing, their predator gains n more
    /// victory points (Gambit Accepted; RTR 20040501).
    PredatorVpIfOusted(u8),
    /// At the end of the game, its controller not ousted gains n victory
    /// points (Gambit Accepted).
    VpAtGameEndIfStanding(u8),
    /// +n hand size for each Methuselah its terms chose (Year of Fortune;
    /// PIB 20121028: gone with the card).
    HandSizeForChosen(i8),
    /// Vampires with capacity below n do not unlock as normal (Snipe Hunt).
    CapacityBelowDoesNotUnlock(u8),
    /// Each Methuselah gets +n hand size for each victory point they have
    /// (The Bitter and Sweet Story).
    HandSizePerVp(u8),
    /// The layer source's host gets n bleed against its controller's prey
    /// (Tier of Souls [ANI]).
    HostBleedAgainstPrey(i8),
    /// Its bearer gets +n bleed bleeding the Methuselah the card remembers
    /// (`CardInPlay::against`; Bear-Baiting).
    HostBleedAgainstRemembered(i8),
    /// Cards of this name cost n more blood (Becoming of Ennoia: Earth
    /// Meld), but to a vampire Gehenna events do not affect.
    NamedCardsCost(&'static str, u8),
    /// The host's recruit ally actions cost n less (Charisma).
    HostRecruitDiscount(u8),
    /// The host gets n votes while its controller controls the named
    /// vampire (Mithraic Cultist).
    HostVotesWhileControlling { name: &'static str, n: i8 },
    /// The host gets n intercept during actions directed at /Mithras/ or a
    /// Mithraic Mysteries card or member (Mithraic Cultist).
    HostInterceptVsMithraic(i8),
    /// The host gets n intercept against acting vampires (Carlton Van Wyk).
    HostInterceptVsVampires(i8),
    /// The host cannot hunt; made to hunt, it goes to torpor instead
    /// (Pulled Fangs), locked first with `lock` (Antithesis).
    HostHuntsToTorpor { lock: bool },
    /// An ally opposing the host vampire in combat, about to burn, is stolen
    /// and locked, with n life if none (The Book of Going Forth by Night).
    HostStealsOpposingAllyOnBurn(u8),
    /// The host gets n intercept against the card's chosen minion (Divine
    /// Sign).
    HostInterceptVsChosen(i8),
    /// The host blocking the card's chosen minion opens its controller's
    /// hand for that combat (Divine Sign [NEC]).
    OpenHandOfChosenBlocked,
    /// Every Methuselah's allies get n intercept against acting vampires
    /// (The Unmasking).
    AlliesInterceptVsVampires(i8),
    /// Every imbued gets +1 intercept blocking a monster, once whatever the
    /// copies, not with The Unmasking's (Illuminate).
    ImbuedInterceptVsMonsters,
    /// Every imbued gets n intercept attempting to block a monster
    /// (Break the Code: -1).
    ImbuedInterceptVsMonstersMod(i8),
    /// The host cannot gain blood; blood it would gain goes to the bank
    /// (Punish).
    HostCannotGainBlood,
    /// Vampires opposing the bearer, if it is imbued, cannot maneuver to
    /// close range (The Crusader Sword).
    OpposingVampiresNoCloseManeuver,
    /// Intercept this action for monsters (Hide, Second Sight).
    MonsterInterceptMod(i8),
    /// The host is a Mithraic Mysteries member (Mithraic Cultist).
    MithraicMember,
    /// A card in play reducing the cost of its controller's cards.
    CostSource(CostSource),
    /// The host is untargetable by others' actions; masters on it cost 1
    /// more pool (Secure Haven).
    Haven,
    /// Non-electronic, non-vehicle, non-weapon equipment cards cost the
    /// controller's minions n less blood or pool to equip (British Museum).
    PlainEquipDiscount(u8),
    /// Cards requiring this Discipline cost vampires of this clan n fewer
    /// blood, any Methuselah's (The Path of Bone: Necromancy, Giovanni);
    /// `slave`: slave ones only, read live (The Path of Service).
    ClanDisciplineBloodOff {
        clan: &'static str,
        discipline: &'static str,
        n: u8,
        slave: bool,
    },
    /// A discard phase action's discard is replaced at its Methuselah's next
    /// unlock phase, not now (Port Authority).
    DiscardActionReplacesAtUnlock,
    /// Frenzy cards cannot be used on the bearer, in or out of combat (The
    /// Path of the Scorched Heart).
    BearerFrenzyImmune,
    /// While the bearer is acting or in combat, other Methuselahs' minions
    /// cannot play cards requiring these Disciplines (The Path of the
    /// Scorched Heart: Presence; rules-qa 38.4).
    DisciplineBarAround(&'static [&'static str]),
    /// The bearer is infernal while the card is on it (Infamous
    /// Insurgent).
    BearerInfernal,
    /// Locations cost the controller n less blood or pool, a locquipment
    /// too (Therbold Realty; LSJ 20080107).
    LocationDiscount(u8),
    /// Cards of this label cost every Methuselah n more pool (Centralized
    /// Background Check: weapons), with `while_host_ready` only while the
    /// card's bearer is ready (Guarded Rubrics: Gehenna cards).
    LabelPoolSurcharge {
        label: crate::library::Label,
        pool: u8,
        while_host_ready: bool,
    },
    /// Minions of the clan get 1 optional maneuver each combat (Brujah
    /// Debate).
    ClanCombatManeuver(&'static str),
    /// The host, while of one of these clans, gets +1 vote in referendums
    /// it calls (Fee Stake: Boston).
    HostVoteWhenCalling(&'static [&'static str]),
    /// The host, while untitled and of the clan, gets +n votes in
    /// referendums a vampire of the clan calls, read live (Skaald).
    HostVotesWhenCallerOfClan { clan: &'static str, n: u8 },
    /// The host gets +1 vote per ready Black Hand vampire its controller
    /// controls (Black Hand Emissary).
    HostVotesPerOwnBlackHand,
    /// Burned once its controller controls no ready Seraph (Praetorian
    /// Backer).
    SeraphBound,
    /// While the layer source is unlocked, vampires of capacity `max` or
    /// less get n bleed against its controller (Aranthebes, The Immortal).
    YoungBleedAgainstController { max: u8, n: i8 },
    /// Every minion of the clan gets n to the stat (Week of Nightmares);
    /// `own`: the controller's (Gangrel Revel); `titled`: only titled ones
    /// (New Carthage) or only untitled (Foundation Exhibit); `during`: in
    /// its actions of these kinds (The Khabar: Community).
    ClanStat {
        clan: &'static str,
        stat: Stat,
        n: i8,
        own: bool,
        titled: Option<bool>,
        during: &'static [ActionKind],
    },
    /// The card's controller's minions get n of `stat` (Esprit de Corps),
    /// `vampires` only (Vivienne Géroux), during their actions of these
    /// kinds (any, if empty).
    OwnMinionsStat {
        stat: Stat,
        n: i8,
        vampires: bool,
        during: &'static [ActionKind],
    },
    /// Every Methuselah's vampires of the sect get n of `stat` during
    /// `during` (any action, if empty) (Festivo dello Estinto).
    SectStat {
        sect: Sect,
        stat: Stat,
        n: i8,
        during: &'static [ActionKind],
    },
    /// A vampire of the sect successfully hunting gains enough blood from
    /// the bank to reach its capacity (Festivo dello Estinto; RTR 20030519).
    SectHuntToCapacity(Sect),
    /// Minions of the clan do not hunt as normal (Week of Nightmares).
    ClanCannotHunt(&'static str),
    /// The controller's vampires of the sect get +n hunt (The Hungry
    /// Coyote).
    OwnSectHunt(Sect, u8),
    /// The host cannot recruit allies or employ retainers (Depravity).
    HostCannotRecruitOrEmploy,
    /// The host gets n optional presses each combat (Dread Mastiff [ANI]).
    HostCombatPress(u8),
    /// The host gets an optional press each combat with a vampire of the
    /// sect, read live (Redline).
    HostCombatPressVsSect(Sect),
    /// As a vampire plays its first card requiring a Discipline each turn,
    /// its controller reveals the top of their library: a master there,
    /// the card has no effect (no cost paid), and the vampire burns `burn`
    /// blood if the card required the Discipline `disc` (Veil of
    /// Darkness).
    FirstDisciplineCardRevealsTop { disc: &'static str, burn: u8 },
    /// A Methuselah beginning their unlock phase with their library
    /// exhausted and a short hand must attempt to withdraw, ousted at their
    /// next unlock phase if it failed; any withdrawal ousts the card's
    /// controller (Brinksmanship).
    ForcedWithdrawals,
    /// The host gets n mandatory presses each combat, only usable to
    /// continue combat in the first round (Talbot's Chainsaw).
    HostFirstRoundMandatoryPress(u8),
    /// The host cannot hunt (Talbot's Chainsaw).
    HostCannotHunt,
    /// The host cannot commit diablerie (Alastor).
    HostCannotDiablerize,
    /// The host cannot block (Pentex™ Subversion).
    HostCannotBlock,
    /// The second chosen vampire's non-bleed actions are unblockable
    /// (Toreador Grand Ball).
    SecondChosenUnblockable,
    /// The first chosen vampire does not unlock as normal (Toreador Grand
    /// Ball).
    FirstChosenDoesNotUnlock,
    /// +1 level of the Discipline and +1 capacity on the host (Discipline
    /// masters: Oblivion).
    DisciplineLevel(&'static str),
    /// A successful equip action's minion unlocks at the end of the turn,
    /// read as the action ends (NRA PAC).
    EquipUnlocksEndOfTurn,
    /// As `DisciplineLevel`, of the Discipline chosen as the card was played
    /// (`CardInPlay::discipline`: Sanguine Instruction).
    ChosenDisciplineLevel,
    /// The bearer vampire gets +1 level of the Discipline, no capacity,
    /// nothing on an ally (Ritual Goblet).
    BearerDisciplineLevel(&'static str),
    /// As `BearerDisciplineLevel`, the bearer of this clan only (Vial of
    /// Zapathasura's Vitae; rules-digest D31).
    BearerClanDisciplineLevel {
        clan: &'static str,
        discipline: &'static str,
    },
    /// As `BearerDisciplineLevel`, of the Discipline chosen as the card
    /// was played (`CardInPlay::discipline`: Field Training).
    BearerChosenDisciplineLevel,
    /// The card burns once its host's sect is no longer this one, read
    /// live: a change of the actual sect or an override over it (Field
    /// Training, The Red Question; LSJ 20100811).
    SectBound(Sect),
    /// The bearer vampire gets a level of the Discipline its counters reach
    /// in this order, one each, no capacity (Seal of Veddartha: dom, dom,
    /// for, for; LSJ 20070320-2).
    BearerCounterLevels(&'static [&'static str]),
    /// Every vampire of the clan gets n stealth on its actions requiring the
    /// Discipline (Tremere Convocation: Tremere, Blood Sorcery).
    ClanStealthOnActionsRequiring {
        clan: &'static str,
        discipline: &'static str,
        n: i8,
    },
    /// Vampires with the Discipline at basic play combat cards requiring it
    /// as if superior (Tremere Convocation: Blood Sorcery).
    BasicPlaysSuperiorCombat(&'static str),
    /// The bearer vampire has the Discipline at superior, nothing on an ally
    /// (The Sargon Fragment).
    BearerSuperiorDiscipline(&'static str),
    /// Cards of this name cost `pool` more (or less); `on_host_only`: played
    /// on the source's host; `owner_only`: for its controller only.
    CostMod {
        name: &'static str,
        on_host_only: bool,
        owner_only: bool,
        pool: i8,
    },
    /// Reaction cards cost this much more blood (or life) this action.
    ReactionBloodCost(i8),
    /// Intercept this action for minions younger than the capacity (Perfect
    /// Paragon [PRE]); 0 allies only (Obedient Flesh), `u8::MAX` all.
    InterceptMod { younger_than: u8, n: i8 },
    /// Intercept this action for minions with none of these Disciplines
    /// (Acheron Vortex).
    InterceptModWithout {
        without: &'static [&'static str],
        n: i8,
    },
    /// The bearer is untargetable by directed actions of allies or vampires
    /// of capacity n or less (Veneficti).
    NotDirectedByAllyOrCapacityAtMost(u8),
    /// An extra draw (discard afterward) replacing a card the bearer played
    /// as the acting minion (Learjet).
    ExtraDrawOnReplace,
    /// +n hunt for the hunt of this action (Hospital Food), action-scoped.
    ActionHunt(u8),
    /// The bearer's mandatory additional strike after each round's normal
    /// pair (Renegade Garou).
    AdditionalStrikeEachRound,
    /// The bearer gets +n hunt (Aaron's Feeding Razor;
    /// `GameState::hunt_bonus`).
    BearerHunt(u8),
    /// As `BearerHunt`, once each turn: spent by a successful hunt, one at
    /// capacity too (Harvest Rites; rules-qa 89.1; `CardInPlay::spent_turn`).
    BearerHuntOncePerTurn(u8),
    /// The opposing minion's strikes and damaging effects inflict n less on
    /// the bearer (Nephandus; `GameState::damage_reduction`).
    OpposingDamageLess(u8),
    /// Minions with any corruption counters get this intercept modifier
    /// this action (Sense the Sin [DAI]), read live.
    CorruptedInterceptMod(i8),
    /// The layer source's host gets this intercept during bleed actions
    /// directed at its controller (Guardian Angel).
    HostInterceptOnBleedAtYou(i8),
    /// The host gets n intercept during actions directed at its controller
    /// or a card they control (Abbot; LSJ 20061222).
    HostInterceptDirectedAtYou(i8),
    /// The layer source's controller gets this hand size (Elder Library).
    HandSize(i8),
    /// The layer source's controller gets +n transfers in their influence
    /// phase (Information Highway).
    Transfers(u8),
    /// The layer source's controller gets +1 hand size per counter on it
    /// (Visit from the Capuchin).
    HandSizePerCounter,
    /// The host, if `who`, gets `n` to `stat` in its actions of these kinds,
    /// any when empty (Sport Bike, Unlicensed Taxicab, Treasured Samadji).
    Bearer {
        stat: Stat,
        n: i8,
        during: &'static [ActionKind],
        who: Bearer,
    },
    /// The retainer's lost life, or its burning, is the host's blood or life
    /// instead, the host burned if it has none (Ilomba; ANK 20180525).
    LifeFromHost,
    /// A vampire stealing life from the bearer takes 1 damage per point
    /// stolen, environmental (Tunnel Runner; RTR 19970630, ANK 20200517-2).
    StealersTakeDamage,
    /// The bearer, an ally, gets +1 strength per life it has (Remnant of
    /// the Endless Storm).
    StrengthPerLife,
    /// The bearer, an ally, gets +1 bleed per life above n (Cãoera).
    BleedPerLifeAbove(u8),
    /// A vampire in combat with the bearer pays n more blood for cards it
    /// plays using a Discipline (Father Anthony; rules-digest D29).
    OpposingVampireDisciplineCardsCost(u8),
    /// Damage over n from any one effect on the bearer is ignored (Remnant
    /// of the Endless Storm): each strike's, each out of combat.
    DamageCapPerEffect(u8),
    /// +n hand size while the bearer is a ready vampire of this sect
    /// (Reliquary: Trinket).
    HandSizeWhileBearer { n: u8, sect: Sect },
    /// No card requiring a non-Laibon title is played on the host nor while
    /// it acts (Hiding in the Open): `requires_non_laibon_title`.
    NoNonLaibonTitleCards,
    /// Each vampire of this sect gets +X stealth when hunting and X+1 more
    /// blood on a successful hunt, X its cards of this kind (Guinea-Bissau
    /// Carnival: Aye).
    SectHuntPerHeld { sect: Sect, held: Holding },
    /// The host, if `who`, gets +1 to `stat` per counter on the card
    /// (Despiral).
    BearerPerCounter { stat: Stat, who: Bearer },
    /// The layer source's bearer, while Anarch, gets +n stealth on actions
    /// that require an Anarch (An Anarch Manifesto).
    StealthOnAnarchActions(u8),
    /// Titled non-Anarch vampires get +n strength in combat with the layer
    /// source's bearer (An Anarch Manifesto).
    TitledNonAnarchStrengthVsBearer(u8),
    /// Allies and vampires younger than the host, if `who`, get `n`
    /// intercept against its actions (Stolen Police Cruiser).
    YoungerInterceptAgainstBearer { n: i8, who: Bearer },
    /// Only minions with one of these Disciplines can direct actions at the
    /// host (Shadow Cloak [OBL]).
    DirectedOnlyBy(&'static [&'static str]),
    /// The host burns a counter instead of unlocking (Touch of Oblivion): a
    /// substitution, an unlocked host burning one too, a host kept from
    /// unlocking not (advanced rules §5.2.2).
    CounterInsteadOfUnlock,
    /// The host's rescues cost n less, whoever pays, and a successful one
    /// gives `gain` blood (Saulot's Healing Touch; PIB 20110918, LSJ
    /// 20020304-2).
    RescueDiscount {
        not_clan: Option<&'static str>,
        n: u8,
        gain: u8,
    },
    /// A retainer's nR damage each round (Murder of Crows): environmental,
    /// its source the retainer (advanced rules 5.6.3), past a dodge
    /// (rulebook "Environmental damage"), stopped by a combat ends.
    RetainerDamage(u8),
    /// As `RetainerDamage`, at close range only (Dread Mastiff).
    RetainerDamageClose(u8),
    /// No "strike: combat ends" against the host (Dog Pack).
    NoCombatEndsAgainstHost,
    /// The minion this card is cannot block vampires (Vagabond Mystic).
    CannotBlockVampires,
    /// Vampires of capacity n or more cannot block the bearer (The Signet of
    /// King Saul).
    BigVampiresCannotBlockBearer(u8),
    /// The minion this card is cannot have or use equipment or retainers
    /// (Bone Shambler).
    NoEquipmentOrRetainers,
    /// The minion this card is cannot have or use equipment (Gravebound
    /// Drone).
    NoEquipmentHeld,
    /// A minion the card's controller's predator or prey puts in play out
    /// of the unlock phase enters locked; a vampire younger than the card's
    /// bearer burns 1 blood (Gran Madre di Dio, Italy).
    LocksEnteringFromNeighbours,
    /// The minion this card is casts no votes or ballots (Nosferatu
    /// Bestial).
    CastsNoVotes,
    /// The minion this card is performs only these actions: none ("cannot
    /// act", Ponticulus), hunts (Nosferatu Bestial).
    OnlyActions(&'static [crate::decision::ActionKind]),
    /// The host cannot have or use a weapon with a ranged strike (LSJ
    /// 20060825) but the named one (Anachronism).
    NoRangedWeaponsExcept(&'static str),
    /// A vampire that played an Auspex card this action (any action, LSJ
    /// 20050201) has no maneuver or strike in the first round, read as cards
    /// are played (Ossian; ANK 20210614).
    FreezesAuspexFoeFirstRound,
    /// Any damage the opposing strikes inflict on the bearer at long range,
    /// prevented too, burns this weapon and deals the bearer n aggravated
    /// damage (Improvised Flamethrower; LSJ 20040802).
    BackfiresAtLong(u8),
    /// 1 life per blood the opponent spent healing or preventing that round,
    /// after the presses, an early end too (Ossian; RTR 20030519), over
    /// starting life (ANK 20171109).
    LifePerFoeBloodLost,
    /// The host's opponent's controller gets -n hand size in combat (Raptor
    /// [ANI]).
    OpponentHandSizeInCombat(u8),
    /// The host's controller gets +n hand size while the host is in combat
    /// (Nar-Sheptha).
    HandSizeInCombat(u8),
    /// The host's opponent in combat is considered the acting minion, over
    /// Deep Song (Nar-Sheptha; ANK 20211022).
    OpponentActsInCombat,
    /// The host's opponent's controller plays with an open hand in combat
    /// (Owl Companion).
    OpponentOpenHandInCombat,
    /// Combat cards cost the vampire bearer n fewer blood (Eye of Hazimel).
    BearerCombatCostOff(u8),
    /// This card does not unlock as normal (Corporal Reservoir).
    CardDoesNotUnlock,
    /// The host skips the normal unlock, other unlock effects still working
    /// (Puppet Master [DOM]).
    HostDoesNotUnlock,
    /// The controller's prey plays with an open hand, read live
    /// (Revelations [AUS]).
    PreyOpenHand,
    /// The bearer cannot play cards of this name (Preternatural Strength).
    BearerCannotPlay(&'static str),
    /// Two master phase actions for the Methuselahs the card remembers
    /// (`CardInPlay::against`; Rumors of Gehenna).
    TwoMasterPhaseActionsForChosen,
    /// A discard phase action's discard costs n pool per copy (Camarilla
    /// Threat).
    DiscardActionCostsPool(u8),
    /// The remembered Methuselah replaces only as each minion phase ends
    /// (The Meddling of Semsith).
    NoReplaceInMinionPhaseForChosen,
    /// A bleed burns at most n of the controller's pool (Protected
    /// Resources).
    BleedBurnCap(u8),
    /// The bearer's non-action modifier blood costs halved, rounded down,
    /// before any other change (The Ankara Citadel, Turkey; RTR 20070707).
    HalvesBloodCosts,
    /// Anathema on its host: emptied in combat, it burns, its opponent's
    /// controller gaining its capacity (`Engine::settle_anathema`).
    Anathema,
    /// The minion this card is cannot be the target of directed actions
    /// requiring these Disciplines (Aggressive Corpse: Dominate, Presence).
    ImmuneToDirectedActions(&'static [&'static str]),
    /// The ally plays cards requiring basic X as a vampire, `non_action`
    /// excepting actions (The Vozhd of Juiz de Fora, Spectral Servitor).
    PlaysAsVampire(AsVampire),
    /// Allies cannot block the bearer (Libertas).
    AlliesCannotBlockBearer,
    /// Other Methuselahs' actions targeting the host cost n additional
    /// blood, the card's controller's others (Chanjelin Ward; an ally never
    /// pays it, 73.5). Not `Stat::TargetedCostsBlood` (François Loehr's:
    /// directed actions only, blood or life).
    TargetedCostsBlood(u8),
    /// Other Methuselahs' actions targeting the host cost monsters n more
    /// blood or life, paid as they resolve; a non-monster pays none (Lock).
    TargetedCostsMonsters(u8),
    /// While any ready imbued is in play, an action to put a vampire or a
    /// ghoul in play or to add counters to an uncontrolled vampire gets -1
    /// stealth and costs an additional blood or life (Blood Cult Awareness
    /// Network; `ActionCtx::makes_vampires`).
    VampireMakingTaxed,
    /// The host, a monster, burns n blood or life as it attempts an action
    /// or a block; short, what it has (Lock).
    HostBurnsToAttempt(u8),
    /// Only allies of this subtype may block the bearer's undirected hunts
    /// (Ravager: changelings).
    OnlySubtypeBlocksUndirectedHunts(crate::library::Label),
    /// Every ally of the subtype gets n to the stat (Song of Pan: changeling
    /// allies +1 bleed).
    SubtypeStat {
        label: crate::library::Label,
        stat: Stat,
        n: i8,
    },
    /// Others' cards requiring these Disciplines cost +n blood around the
    /// acting, blocked or fighting bearer (Libertas).
    DisciplineSurchargeAround(&'static [&'static str], u8),
    /// The host's cards requiring a Discipline cost +n blood (Enhanced
    /// Coagulant, once it became that card).
    HostDisciplineCardsCost(u8),
    /// Its controller's Anarchs may move a counter to the minions their hand
    /// or melee damage hits (Blade Clot).
    MovesClotsOnHit,
    /// Black Hand vampires get +n stealth (Watchtower: The Wolves Feed).
    BlackHandStealth(u8),
    /// The host is Black Hand, whatever its sect (Trophy: Chosen).
    HostBlackHand,
    /// The host is a Seraph, Black Hand as it took the card, while the card
    /// is on it (Trophy: Chosen).
    HostSeraph,
    /// The host's capacity n lower, at least 1 (Elixir of Distillation).
    HostCapacityDown(u8),
    /// The host, of this sect, has n more capacity for other Methuselahs'
    /// cards (Orun; LSJ 20051204; ANK 20221208): `GameState::capacity_by`.
    CapacityToOthers(u8, Sect),
    /// The host, of this sect, gets a vote for every three cards in play on
    /// it with this layer (Orun; LSJ 20051204).
    HostVotePerThree(Sect),
    /// The card burns once its host holds a card of this kind (Mundane: an
    /// Orun): `Engine::settle_over_capacity`.
    BurnWhileHostHolds(Holding),
    /// The card burns while its host, of this sect, holds more of these
    /// cards than its capacity (Orun, Aye): `Engine::settle_over_capacity`.
    BurnOverCapacity(&'static [Holding], Sect),
    /// The host, of this sect, burns one card with this layer after a
    /// successful bleed for more than 2 or directed action at a non-mortal
    /// minion (Orun; LSJ 20090603): `Engine::burn_one_on_success`.
    BurnOneOnSuccess(Sect),
    /// The host, of this sect, burns one card with this layer after it
    /// burns a minion, equipment or location (Aye):
    /// `Engine::burn_one_burning`.
    BurnOneOnBurning(Sect),
    /// While the host is in torpor, Blood Brothers of the card's chosen
    /// circle get +1 bleed against its controller (Achilles' Heel).
    CircleBleedWhileHostTorpid,
    /// Two on a vampire: it loses the benefit of its title (No Confidence).
    NoConfidence,
    /// Burned once the minion it chose leaves the ready region (No
    /// Confidence).
    ChosenBound,
    /// As its minion enters combat, moves with 1 blood to the opposing
    /// minion; an Anarch moves it instead (Blood Turnip).
    MovesAsCombatBegins,
    /// Cards requiring this Discipline cost the host n fewer blood (Drink
    /// the Blood of Ahriman).
    HostDisciplineBloodOff { discipline: &'static str, n: u8 },
    /// The host burns n blood to attempt any action but a hunt or one
    /// entering combat; this card burns as its action ends (Enrage [PRE]).
    HostActionToll(u8),
    /// The host's directed actions cost it n more blood (Kuta).
    HostDirectedToll(u8),
    /// The host cannot attempt to block older vampires (Kuta).
    HostCannotBlockOlder,
    /// Burned if ranged aggravated damage is inflicted on the host, even
    /// prevented (Drink the Blood of Ahriman).
    BurnedByRangedAggravated,
    /// No Gehenna events can be played (The Fourth Cycle).
    NoGehennaEvents,
    /// Minion cards redirecting a bleed cost +n blood or life (Narrow
    /// Minds).
    RedirectCardsCost(u8),
    /// No combat ends as a strike in the first round of the turn's first
    /// combat between an acting vampire and its vampire blocker (NSA Trio;
    /// LSJ 20050215).
    NoCombatEndsFirstBlockCombat,
    /// Torpor actions and rescues of an older vampire cost +1 blood (Torpid
    /// Blood).
    TorpidBlood,
    /// Cards requiring a superior Discipline cost +n blood, diablerists
    /// aside (The Slow Withering).
    SuperiorCardsCost(u8),
    /// Discipline cards played in an action replaced at its end or an empty
    /// hand, diablerists aside (Blood Weakens).
    HoldDisciplineReplacements,
    /// After each minion phase, ready vampires that did not hunt, of
    /// capacity under the Gehenna events in play, burn 1 blood (Thirst).
    Thirst,
    /// Capacities capped at the card's counters, minimum 1 (Wormwood).
    CapacityCapByCounters,
    /// +n capacity on the host (Camarilla Vitae Slave).
    HostCapacity(u8),
    /// No blood hunt on a diablerist above the counters in capacity who
    /// diablerized a younger vampire (Fueled by Heart's Blood).
    NoBloodHuntAboveCounters,
    /// Burn another event at random each controller's discard phase (The
    /// Uncoiling).
    BurnsAnEventEachDiscard,
    /// 1 pool per vampire in torpor, each discard phase (Dragonbound).
    TorporPoolBurn,
    /// Only vampires that must hunt can hunt (Restricted Vitae).
    OnlyMustHunt,
    /// The sect's vampires are Independent (Fall of the Camarilla, Fall of
    /// the Sabbat).
    SectFallen(Sect),
    /// Absimiliard's Army: after a Methuselah is ousted, burn this card;
    /// as it leaves play, its ghouls are removed from the game.
    GhoulArmy,
    /// An ally burned fighting an acting vampire: 1 counter, `damage`
    /// unpreventable after the combat, burned at `burn_at` (FBI Special
    /// Affairs Division).
    AllyBurnedInCombat { damage: u8, burn_at: u8 },
    /// The controller's vampires get +n bleed while the prey has one in
    /// torpor, read live (Raising the Portcullis).
    OwnBleedWhilePreyTorpor(i8),
    /// The controller's `clan` vampires of capacity `min` or more get +1
    /// bleed against their prey per vampire that prey controls in torpor
    /// (Blessings of the Loa).
    ClanBleedPerPreyTorpor { clan: &'static str, min: u8 },
    /// Blockers of the host burn n blood, cumulative (Archon; LSJ 19980225),
    /// one short burning what it has and still blocking (LSJ 19970707).
    BlockersOfHostBurn(u8),
    /// Allies and younger vampires burn n blood or life to attempt to block
    /// the host (Transcendent Laibon).
    SmallBlockersOfHostBurn(u8),
    /// Allies and younger vampires burn n blood or life to perform an
    /// action directed at the host (Transcendent Laibon).
    SmallTargetersCost(u8),
    /// The card burns once its host is not ready or holds fewer than n of
    /// any of these kinds (Transcendent Laibon): `settle_over_capacity`.
    BurnUnlessHostHolds { held: &'static [Holding], n: u8 },
    /// No blood hunt on the host (Archon).
    NoBloodHuntOnHost,
    /// The card's bearer is Black Hand while Sabbat, read live (Cadet,
    /// Mustajib; LSJ 20070322).
    HostBlackHandWhileSabbat,
    /// The strikes of the minion this card is cannot be dodged: a dodge has
    /// no effect on them (Aggressive Corpse; LSJ 20030902-2).
    UndodgeableStrikes,
    /// Intercept this action for vampires with any of these Disciplines
    /// and for mortal or other allies of these subtypes (The Ailing Spirit
    /// [obe]: Dementation, mortals and mages).
    InterceptModFor {
        disciplines: &'static [&'static str],
        subtypes: &'static [crate::library::Label],
        n: i8,
    },
    /// Minions without the Discipline cannot block this action, allies
    /// included (Cheat the Fates [tem]).
    CannotBlockWithout(&'static str),
    /// The host vampire is sterile (Barrenness [mal]; `GameState::is_sterile`).
    HostSterile,
    /// Each minion the card chose gets n of the stat while it is in play
    /// (Blessing of the Name: intercept, melee weapon damage).
    ChosenStat { stat: Stat, n: i8 },
    /// The equipment this card is on: as a melee weapon, +n damage each
    /// strike (Blood Tempering; `GameState::melee_damage_bonus`).
    WeaponDamage(u8),
    /// The equipment this card is on prevents n more each time it prevents
    /// damage in combat (Blood Tempering).
    EquipmentPreventsMore(u8),
    /// The bearer gets n of the stat on its non-hunt actions, a card hunt
    /// being a hunt (Decompose).
    BearerOnNonHunt { stat: Stat, n: i8 },
    /// The host cannot use additional strikes, nor play a card giving one
    /// (Decompose; LSJ 20011214-5).
    HostNoAdditionalStrikes,
    /// Every press the host uses costs it n more blood or life, offered
    /// only if payable (Decompose [THN]).
    HostPressCost(u8),
    /// The host gets +n intercept against directed actions (Defender of the
    /// Haven).
    HostInterceptVsDirected(i8),
    /// Bleed actions cost the host n more blood or life (Greater Curse).
    HostBleedToll(u8),
    /// Combat cards cost the host n more blood or life (Greater Curse).
    HostCombatCardsCost(u8),
    /// Any vampire may attempt to block this action, ignoring the prey,
    /// predator and target restrictions (Lord of Serenity [for]).
    VampiresBlockAnywhere,
    /// The host cannot perform directed actions (Shadowed Eyes;
    /// `GameState::no_directed_actions`).
    HostNoDirectedActions,
    /// The host cannot use strikes usable at long range (Shadowed Eyes).
    HostNoRangedStrikes,
    /// Burned or sent to torpor in combat, the host burns instead and deals
    /// n aggravated damage to the opposing minion and its retainers, which
    /// they may prevent (Potio Martyrium; LSJ 20100205, ANK 20181024).
    HostMartyrdom(u8),
    /// The cards the host plays are not replaced until the release
    /// (Minor Curse: the end of the action, [MAL] that turn's discard
    /// phase).
    HostPlaysHeldUntil(crate::state::Release),
    /// The bearer's only subtype is this one, its text kept (Pressing
    /// Flesh: a zombie; LSJ 20100212).
    BearerSubtypeOnly(crate::library::Label),
    /// +n strength for the bearer in combat with a vampire of this sect,
    /// read live (The Black Beast; `GameState::strength_against`).
    BearerStrengthVsSect { sect: Sect, n: u8 },
    /// The bearer's hand strikes deal aggravated damage (Spirit Claws
    /// [PRO][SPI]).
    BearerAggravatedHands,
    /// Other Methuselahs' actions directed at the bearer cost n more blood or
    /// life, paid as they resolve (Safe Passage [OBE]; François Loehr's).
    DirectedAtBearerCostsBloodOrLife(u8),
    /// The vampire bearer can burn `blood` to play a card requiring basic
    /// `discipline` as if it had it, once each turn per copy (Amulet of
    /// Temporal Perception; LSJ 19980303, ANK 20180719-4).
    BearerPaysToPlayAsBasic { discipline: &'static str, blood: u8 },
    /// Opposing vampires of capacity n or less cannot strike in the first
    /// round, read live (Rock Cat; LSJ 20001126).
    FoesUpToCapacityNoStrikeFirstRound(u8),
    /// Cards requiring these Disciplines can neither be played while the
    /// bearer acts, blocks or is in combat, nor target it (Draeven Softfoot).
    ShunsDiscipline(&'static [&'static str]),
    /// Marker: a vampire of the clan burning a ready vampire or sending one to
    /// torpor, in combat or by a directed action, may unlock after that minion
    /// phase (Code of Samiel; `Lasting::MayUnlockAfterMinionPhase`).
    UnlockAfterMinionPhaseForClan(&'static str),
    /// The card burns once its host is not infernal, read live (Maleficia,
    /// Striga).
    InfernalBound,
    /// A Methuselah gains no pool in their own turn without the Edge or a
    /// victory point: it goes to the bank (The Rising; ANK 20181105).
    PoolGainNeedsEdgeOrVp,
    /// No boons can be put in play (Blood Trade).
    NoBoons,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    /// While the source card stays in play.
    InPlay,
    /// Until the current action ends.
    Action,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Layer {
    pub scope: Scope,
    pub modifier: Modifier,
    pub source: CardId,
}

/// The vampire a trained card becomes (Web of Knives Recruit): its clan,
/// sect, what it is born with, and its blood.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Trained {
    pub clan: &'static str,
    pub sect: Sect,
    pub born: crate::state::Born,
    pub blood: u8,
}

/// Do these effects give a press (Decompose [THN]'s surcharge)?
pub fn gives_press(effects: &[Effect]) -> bool {
    effects.iter().any(|e| {
        matches!(
            e,
            Effect::Press
                | Effect::OptionalPress
                | Effect::PressThisRound
                | Effect::ContinuePressThisRound
                | Effect::EndPressThisRound
        )
    })
}

/// An equipment or a conviction card (Din of the Damned's "equipment and
/// conviction cards"); a vampire's own text (`u16::MAX`) is neither.
pub fn equipment_or_conviction(card: u16) -> bool {
    LIBRARY.get(card as usize).is_some_and(|c| {
        c.types.contains(&CardType::Equipment) || c.types.contains(&CardType::Conviction)
    })
}

/// Do these effects grant intercept for the action (Din of the Damned's
/// toll)?
pub fn grants_intercept(effects: &[Effect]) -> bool {
    effects.iter().any(|e| match *e {
        Effect::AddIntercept(n) => n > 0,
        Effect::AttemptingIntercept(n) => n > 0,
        Effect::AddInterceptX
        | Effect::TargetInterceptThisAction(_)
        | Effect::TargetAttemptsBlock(_)
        | Effect::CircleIntercept { .. }
        | Effect::BurnForInterceptThisAction => true,
        Effect::If { then, .. } => grants_intercept(std::slice::from_ref(then)),
        _ => false,
    })
}

/// Effect primitives; `X` is the play's terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Move 1 counter from this card to the target minion; none left, each
    /// vampire of the clan burns 1 blood per counter or is burned (Week of
    /// Nightmares).
    CounterToTarget(&'static str),
    /// The granted action is a hunt, successful only if it gains blood
    /// (Week of Nightmares; LSJ 20050720-2).
    HuntAction,
    /// Leave rider: a vampire of capacity n or less sent to torpor gets 1
    /// hostage counter (Carver's Meat Packing and Storage).
    HostageOnTorpor(u8),
    /// Add X blood to the target, X the vampires with hostage counters.
    BloodPerHostage,
    /// Burn the target vampire's hostage counters.
    BurnTargetHostages,
    /// Unlock the acting minion of the clan after a successful action
    /// (Warsaw Station).
    UnlockActingIfSuccessful(&'static str),
    /// The acting minion unlocks after resolution if the action is
    /// successful (Hag's Wrinkles [thn]).
    UnlockActorIfSuccessful,
    /// The acting minion unlocks at the end of the action, if still ready
    /// (Big Game).
    UnlockAtEndIfReady,
    /// Move the target vampire from torpor to its controller's ready region
    /// (Chantry).
    TargetLeavesTorpor,
    /// A ready vampire of the clan the user controls burns 1 blood, chosen
    /// as it resolves (Chantry's cost).
    OwnClanBurnsBlood(&'static str),
    /// The card's second chosen vampire is its terms' (`Terms::OwnOfClan`:
    /// Toreador Grand Ball).
    ChooseSecondOfClan(&'static str),
    /// The card's chosen vampire is its terms' (Priority Contract).
    ChooseOwnOfClan(&'static str),
    /// Burn this card to gain n pool as the host is about to leave the
    /// ready region (Priority Contract): also as its controller is ousted
    /// (ANK 20180129-2), and if the card goes with it (PIB 20150512).
    PoolWhenHostLeaves(u8),
    /// Leave rider: the controller gains n pool when a ready minion of
    /// their prey's is burned or sent to torpor (Frontal Assault).
    ControllerGainsPoolIfPrey(u8),
    /// As the influence phase begins, burn this card and 1 pool per ready
    /// minion of the prey (Frontal Assault).
    BurnAtInfluencePerPreyReady,
    /// Add n blood to the acting minion (The Anarch Free Press).
    BloodToActing(u8),
    /// Exchange a hand card for an ash heap card requiring an Anarch
    /// (Garibaldi-Meucci Museum): both named on use (LSJ 20000222), Anarch
    /// or baron requirements only (ANK 20250320).
    ExchangeForAnarchCard,
    /// +1 discard phase action (Powerbase: Los Angeles; `Seat::granted_dpa`).
    GrantDiscardAction,
    /// The target vampire becomes the card's chosen one as it is played
    /// (`CardInPlay::chosen`, The Rack).
    ChooseTarget,
    /// n pool moved from the user's prey to this card, as counters (419
    /// Operation); their last pool ousts them.
    PreyPoolToCounter(u8),
    /// Unlock rider: n pool moves from the controller's prey's pool to
    /// theirs (Gambit Accepted).
    PreyPoolToController(u8),
    /// The user gives n pool to the controller of the card used (Convento
    /// da Penha).
    PayPoolToCardController(u8),
    /// The player loses n victory points as the card is played (Gambit
    /// Accepted).
    LoseVictoryPoints(u8),
    /// An in-play ability's target vampire becomes its card's chosen one
    /// (Powerbase: Luanda).
    CardChoosesTarget,
    /// The actor becomes the card's chosen one (Clandestine Contract: "this
    /// Assamite is chosen for this contract").
    ChooseActor,
    /// The new controller rechooses a ready vampire as control changes
    /// (The Rack): read on the card's first mode.
    RechoosesOnControlChange,
    /// Add n blood to the card's chosen vampire, torpor included (The Rack;
    /// PIB 20150522), the controller's choice (ANK 20200130-2).
    BloodToChosen(u8),
    /// Give the prey control of this card (Anarch Troublemaker).
    GiveToPrey,
    /// Lock up to n of the prey's vampires, chosen as it resolves, a locked
    /// one legal (Anarch Troublemaker; PIB 20121031).
    LockPreyVampires(u8),
    /// An ability's cost in the influence phase's transfers (Wider View).
    SpendTransfers(u8),
    /// +n transfers left this influence phase, at any moment of it
    /// (Ennoia's Theater; RTR 20180303).
    GainTransfers(u8),
    /// Move n blood from the target minion to this card (Wasserschloss Anif).
    BloodToCard(u8),
    /// Move all counters on this card to the target in the uncontrolled
    /// region, over capacity too (Wasserschloss Anif; rules-qa 28.3).
    CountersToUncontrolledTarget,
    /// A granted action's stealth for a titled acting vampire, read live
    /// (Powerbase: Barranquilla).
    StealthForTitled(i8),
    /// A granted action's stealth for a clan, read live on the acting
    /// minion (Wasserschloss Anif, Toreador Grand Ball).
    StealthForClan { clan: &'static str, n: i8 },
    /// Draw 1 crypt card, then remove one from the uncontrolled region,
    /// chosen as it resolves (Wider View).
    DrawCryptThenRemove,
    /// Reveal the crypt's top card: a vampire of the clan is drawn with
    /// blood, else bottomed (Family Gathering) or, `if_younger` than the
    /// acting vampire and `shuffle`, shuffled back (Mesu Bedshet).
    RevealCryptTop {
        clan: &'static str,
        blood: u8,
        if_younger: bool,
        shuffle: bool,
    },
    /// Remove the crypt's top card; of the clan, a non-titled vampire the
    /// user controls (chosen as it resolves) becomes Anarch (Gilbert Duane).
    RemoveCryptTopAnarch(&'static str),
    /// The master card enters play as an ally with this type line; not
    /// recruited, so it may act this turn (Jake Washington).
    BecomesAlly(crate::library::Body),
    /// Add the ability's count X (`Terms::Count`) blood to the target
    /// (Jake Washington).
    BloodToTargetX,
    /// Lock the target card in play (Powerbase: Savannah's cost).
    LockTargetCard,
    /// Ashur Tablets' three-copy payoff, checked as a copy enters play; the
    /// cards chosen then (LSJ 20091030).
    AshurTablets,
    /// Life Boon's gift: X pool (`Terms::BoonPool`) to the target, no
    /// longer about to be ousted; the card in play remembers them.
    GiveBoonPool,
    /// Steal 1 pool from Life Boon's Methuselah, the last one ousting them.
    StealPoolFromBoonTarget,
    /// Combat ends at once, no strike resolving; lock and torpor land once
    /// it has ended (Rötschreck; LSJ 20070907), not if it continues (RTR
    /// 20020501).
    RotschreckEndsCombat,
    /// The target combatant cannot use equipment nor press to end combat
    /// this round, and the round has a press only to continue (Frenzy).
    FrenzyRound,
    /// After `PutInPlay`, the card becomes a vampire of the `sect` printed,
    /// else its sire's (Childe of the Revolution, The Embrace; rulebook
    /// "Sect"; rules-qa 32.6); `master_search`: its Discipline master
    /// search, if the actor meets these (Bamba: a magaji), with
    /// `blood_from_sire` blood it may move from the actor.
    BecomesVampire {
        sect: Option<Sect>,
        /// The clan printed (Create Gargoyle, Hatchling), else its sire's.
        clan: Option<&'static str>,
        /// A slave of its sire's clan (Create Gargoyle; `Minion::slave_of`).
        slave: bool,
        master_search: Option<&'static [Requirement]>,
        blood_from_sire: u8,
        /// Must hunt this turn (Childe of the Revolution).
        must_hunt: bool,
        /// Cannot perform actions this turn (Third Tradition: Progeny).
        idle: bool,
        /// Its capacity and Disciplines (1, none: `NEWBORN`; or the
        /// card's own, written in its definition: Nosferatu Bestial).
        born: &'static crate::state::Born,
    },
    /// Put this card on the target minion.
    PutOnTarget,
    /// The host imbued leaves the incapacitated region and gains 1 life up
    /// to its starting life (rulebook "Appendix: Imbued Rules" 5).
    LeaveIncapacitated,
    /// On success, a monster of the targeted prey's, torpid too, at random,
    /// enters combat with the acting minion (Muse of Flame).
    EnterCombatWithRandomPreyMonster,
    /// As `LeaveIncapacitated`, the target imbued (Bond).
    TargetLeavesIncapacitated,
    /// "[n CONVICTION]" from the target imbued, its player choosing which
    /// (The Rose Foundation: "burn a conviction from a ready imbued you
    /// control").
    BurnTargetConviction(u8),
    /// A granted action "that costs 1 conviction" (every card's is 1): named
    /// as announced, the acting imbued's, burned at resolution (Travis,
    /// John; RTR 20111202).
    PayConviction,
    /// Unlock rider: another ready imbued its controller controls, of their
    /// choice, takes n unpreventable damage (Béatrice "Oracle171" Tremblay).
    OtherReadyImbuedDamaged(u8),
    /// Rider: the counters on this card burn (Surge, as the action ends).
    ClearCounters,
    /// The action fails and the acting minion enters combat with the host
    /// at once, the action going on until it ends (Champion; LSJ 20070217,
    /// 20070214-2).
    FailsIntoCombatWithHost,
    /// The user looks at the hand of the host's combat opponent's
    /// controller (Foresee).
    PeekOpponentControllersHand,
    /// The user may then cancel the combat; the host blocking, the action
    /// continues as if unblocked (Foresee; ANK 20190116, LSJ 20070808-1).
    MayCancelCombat,
    /// The host's combat opponent takes environmental damage equal to its
    /// strength (Shame; RTR 19970630).
    DamageOpponentByStrength,
    /// The conviction this granted action paid moved from its player's ash
    /// heap to the target imbued (Inspire).
    PaidConvictionToTarget,
    /// The card granting this bleed burns only as it resolves, not if
    /// blocked or canceled (Strike with Conviction; LSJ 20060421).
    BurnSourceOnResolution,
    /// Rider: the host imbued burns n conviction, its player choosing which
    /// (Earl "Shaka74" Deams).
    HostBurnsConviction(u8),
    /// Its user looks at the top n cards of its library (Anna "Dictatrix11"
    /// Suljic).
    LookAtLibraryTop(u8),
    /// Its user looks at the top card of the library of the Methuselah
    /// whose turn it is (Weirding Stone).
    LookAtTurnLibraryTop,
    /// Look at the top card of each Methuselah's library (Sweeper).
    LookAtEachLibraryTop,
    /// Look at the top n cards of the target Methuselah's library and burn
    /// up to one of them (Tapestry of Blood; `Interrupt::BurnFromLibraryTop`).
    LookAtTargetLibraryTopBurnOne(u8),
    /// Steal up to n blood from an uncontrolled minion of the target
    /// Methuselah's region to the actor, both named as announced (San
    /// Lorenzo de El Escorial; `Interrupt::StealFromSlot`).
    StealFromTargetSeatSlot(u8),
    /// A burned vampire of capacity above `burned_above`: this card burns
    /// and its controller searches their crypt for a vampire of capacity
    /// below `below`, ready with 1 blood, not acting this turn (Virolax
    /// Facility; `Interrupt::CryptToReady`).
    CryptVampireToReady {
        burned_above: u8,
        below: u8,
        blood: u8,
    },
    /// The card looked at a master, the host vampire with n blood: its
    /// player may burn n of it to burn that card (Weirding Stone;
    /// `Interrupt::MayBurnLookedMaster`), the look's information its own.
    MayBurnLookedMaster(u8),
    /// Granted action: the target vampire gets the title (Tobias Smith).
    TargetGetsTitle(crate::crypt::Title),
    /// Granted action: the acting slave is no longer one, and becomes
    /// Independent (Virstania, The Great Mother).
    ActorFreedFromSlavery,
    /// Its user may search its library or ash heap for a card of the
    /// holdings and put it on the target minion (Umdava;
    /// `Interrupt::SearchOnto`).
    SearchOntoTarget(&'static [Holding]),
    /// After its user looked at its own library's top card, the host
    /// vampire with n blood: its player may burn n of it to move that card
    /// to the bottom (Kurt Densch; `Interrupt::MayBottomLooked`).
    MayBottomLookedForBlood(u8),
    /// The host imbued may gain a conviction from its player's hand or ash
    /// heap, `library` too (Leaf, Xian, Erick, Abjure).
    GainConviction { library: bool },
    /// The acting imbued may gain a conviction from its player's hand or
    /// ash heap, on success (Respire).
    ActingGainsConviction,
    /// "[n CONVICTION]": the host imbued burns n of its conviction cards,
    /// its player choosing which (rulebook "Appendix: Imbued Rules" 2;
    /// RTR 20111202).
    BurnConviction(u8),
    /// "Doesn't gain a conviction as normal when entering play" (Liz
    /// Thornton): an entry marker.
    NoEntryConviction,
    /// As it enters play, its player may search their library (shuffled
    /// after) or hand for a power requiring the virtue and put it on it
    /// (Inez "Nurse216" Villagrande; LSJ 20080526: before any contest).
    SearchPowerOnEntry(&'static str),
    /// A conviction played on the target imbued, burned instead above five
    /// (rulebook "Appendix: Imbued Rules" 2).
    ConvictionOnTarget,
    /// Put this card in play with no host (Anarch Revolt).
    PutInPlay,
    /// This card, on a vampire, represents the title, burned if it is
    /// yielded or lost (glossary "Title"); read by `card_title`.
    RepresentsTitle(Title),
    /// Rider effect: the Methuselah whose unlock phase it is burns `pool`
    /// unless they control a ready vampire of `sect` (Anarch Revolt; each
    /// copy on its own, spec §1.15).
    UnlockBurnPoolUnlessReady { sect: Sect, pool: u8 },
    /// Rider effect: the controller's prey burns n pool (Creeping
    /// Sabotage).
    PreyBurnsPool(u8),
    /// Rider effect: the Methuselah resolving it burns n pool (Fame's
    /// unlock rider).
    BurnPool(u8),
    /// As a card is played: its player burns n pool, a cost (Framing an
    /// Ancient Grudge's justicar).
    PlayerBurnsPool(u8),
    /// A diablerie card: the diablerist may gain a level of a Discipline the
    /// victim had (Cloak of Blood).
    GainVictimDiscipline,
    /// In the diablerie's blood hunt, the diablerist gets n more votes (Cloak
    /// of Blood).
    BloodHuntVotesForDiablerist(u8),
    /// In the diablerie's blood hunt, each Anarch gets n more votes (Stealing
    /// Years).
    BloodHuntVotesForAnarchs(u8),
    /// After the strike, 1 more damage on the opposing minion, which cannot
    /// press this round (Shoulder Drop).
    GrappleDamage(u8),
    /// The own vampire chosen is the target; a prey's minion is chosen next
    /// (Blind Spot).
    ChooseBlindSpot,
    /// The reacting minion locks and enters combat with the acting one,
    /// getting an optional maneuver, else press, there (Follow the Blood).
    FollowActing { maneuver: bool },
    /// Blocked, the card on the actor burns and the blocker fights its
    /// chosen minion instead (The Framing; `Hook::HostBlocked`).
    FramesBlocker,
    /// Offered as a combat of its controller's Anarch ends, both still
    /// ready: burn the card for another combat (Haven Hunting;
    /// `Hook::OwnAnarchCombatEnded`).
    MayBurnForNewCombat,
    /// Lock another of the player's unlocked ready Anarchs, chosen now
    /// (Power of All).
    LockOtherAnarch,
    /// As the action succeeds, a bleed of 0 too: the acting vampire gains n
    /// blood (Keystone Kine [cel]).
    ActorGainsBlood(u8),
    /// On a successful bleed: an ally of the prey costing no more than the
    /// bleed may be burned (Keystone Kine [obf]).
    BurnPreyAllyUpToBleed,
    /// Half the blood, rounded down, on a crypt card in the prey's
    /// uncontrolled region burned (The World's a Canvas [dom]).
    BurnHalfPreySlot,
    /// The host burns n blood or this card burns, its controller's choice
    /// (Drink the Blood of Ahriman).
    HostBurnsBloodOrBurnSelf(u8),
    /// The actor merges with its advanced version, from its crypt,
    /// uncontrolled region or ash heap (Epiphany).
    MergeWithAdvanced,
    /// The card's chosen minion forgotten (Joseph Pander's turn over).
    ClearChosen,
    /// At 1 life, a ready Anarch may burn a blood to add 1 life to the host
    /// ally (Grey Thorne).
    AnarchMayHealHost,
    /// Each ready vampire of this clan gets +1 vote this referendum, lost if
    /// one of them opposes or abstains on the player's (Unexpected
    /// Coalition).
    ClanCoalition(&'static str),
    /// A circle is chosen for the card as it enters (Achilles' Heel).
    ChooseCardCircle,
    /// A trophy (rulebook "Trophy"): no effect until moved to a vampire.
    Trophy,
    /// Moved to a vampire, it gains n blood, becomes Sabbat, unlocks (Trophy:
    /// Chosen).
    TrophyBlood(u8),
    /// An ability's: the target is the card's chosen minion (Joseph Pander).
    ChooseTargetByAbility,
    /// The target vampire loses its title (Framing an Ancient Grudge).
    TargetLosesTitle,
    /// Rider effect: the Methuselah resolving it moves n counters from
    /// their pool to this card, down to 0 ousting them (Smiling Jack).
    PoolToCounter(u8),
    /// Rider effect: per counter, 1 pool or 1 blood from a vampire they
    /// control, each unit chosen as paid (Smiling Jack; any mix, ANK
    /// 20181006; no blood, no source, LSJ 19971212).
    PoolOrBloodPerCounter,
    /// Rider effect: X pool and/or random hand cards, X the counters; the
    /// count of cards announced, burned at once (Constant Revolution; LSJ
    /// 20090618).
    PoolOrRandomCardsPerCounter,
    /// Put in play with n counters.
    EnterWithCounters(u8),
    /// Put in play locked, however equipped (Helicopter; LSJ 20100119).
    EntersLocked,
    /// A granted action's pool cost, paid at resolution; unpayable then,
    /// it fizzles (rules-qa 3.6).
    PayPool(u8),
    /// Rider effect: the controller of the minion the hook names burns n
    /// pool, the host (Fame) or the minion that left (Tension in the
    /// Ranks), at once where it happened (rules-qa 6.4).
    TheirControllerBurnsPool(u8),
    /// Ability cost: one of the player's master phase actions.
    SpendMasterPhaseAction,
    /// The host takes a card of these kinds from its controller's library
    /// or ash heap (Nkishi: an Aye or an Orun; `Interrupt::SearchOnto`).
    SearchOntoHost(&'static [Holding]),
    /// Ability cost: lock this card in play.
    LockSelf,
    /// Ability cost: burn the Edge (Kalinda).
    BurnEdge,
    /// Ability cost: one of the player's discard phase actions.
    SpendDiscardPhaseAction,
    /// Add n blood to the host, up to its capacity.
    AddHostBlood(u8),
    /// The library card chosen by `Terms::AshCard` goes from the ash heap to
    /// the bottom of the library (Mora, the Death Seer).
    AshCardToLibraryBottom,
    /// Add n counters to this card in play (Dreams of the Sphinx).
    AddCounter(u8),
    /// Burn this card once it carries n counters, checked as the ability
    /// or the burn action granted resolves (Dreams of the Sphinx, Pulled Fangs).
    BurnSelfAtCounters(u8),
    /// Rider effect: burn n counters from this card (Visit from the
    /// Capuchin).
    RemoveCounter(u8),
    /// Rider effect: burn this card if it has no counters left.
    BurnSelfWithoutCounters,
    /// The player gains n pool.
    GainPool(u8),
    /// Hand size +n until the end of the discard phase (LSJ 20100812),
    /// drawn up at once; cards over size afterward stay (ANK 20180318).
    HandSizeBonusUntilEndOfTurn(u8),
    /// Hand size +X until the end of the turn, X the ready minions the
    /// active Methuselah controls, drawn up at once (Shahid).
    HandSizePerActiveReadyMinions,
    /// Add n blood to the targeted vampire in the player's uncontrolled
    /// region (capped at its capacity).
    BloodToUncontrolledTarget(u8),
    /// Lock X unlocked cards of this kind on the actor to add X blood to
    /// the target in the uncontrolled region; fewer left, nothing (Brutal
    /// Influence [POT]).
    LockHeldForUncontrolledBloodX(Holding),
    /// Lock X unlocked cards of this kind on the actor as the action is
    /// announced, kept locked whatever its outcome (Brutal Influence [pot]).
    LockActorHeldX(Holding),
    /// Lock X unlocked cards of this kind on the actor to send the target,
    /// its capacity or cost at most 2X, to its owner's uncontrolled region;
    /// either short, nothing (Exile [PRE]).
    LockHeldToUncontrolledX(Holding),
    /// Lock n unlocked cards of this kind on the actor (Dusk Work: three
    /// Aye).
    LockActorHeld(Holding, u8),
    /// Move n counters from the user's pool to the target crypt card in its
    /// uncontrolled region (Paul "Sixofswords29" Moreton).
    PoolToUncontrolledTarget(u8),
    /// Add n blood to the targeted minion, capped at its capacity (The
    /// Coven).
    BloodToTarget(u8),
    /// Burn the target vampire in torpor, not as diablerie: the actor may
    /// take its equipment first, then gains blood equal to its capacity
    /// (Sacrificial Lamb; `Interrupt::DiablerieEquipment`).
    SacrificeTarget,
    /// Move blood and cards from the target to another ready vampire of
    /// the user's, chosen next (Heidelberg Castle, Communal Haven:
    /// Cathedral; `Interrupt::MoveBetween`).
    MoveBetweenVampires(Moves),
    /// This vampire takes over the action: its modifiers stay, the inherent
    /// ones are the new actor's (Mask of a Thousand Faces; RTR 19951110); a
    /// blocker attempting still is (LSJ 19990106).
    TakeOverAction,
    /// The target seat's hand is visible to the user until the current
    /// action ends (SchreckNET).
    PeekHand,
    /// The player sees their predator's and prey's hands until the action
    /// ends (Augur).
    PeekNeighborHands,
    /// The player looks at the top n cards of their library (Augur).
    ActorLooksAtLibraryTop(u8),
    /// Rider: the card's bearer having bled, its controller looks at the
    /// bled Methuselah's hand (Tunnel Runner).
    PeekBledHandIfBearer,
    /// Own-bleed rider: the card's bearer, bleeding its controller's prey
    /// successfully, gains n blood (Capitalist).
    BearerBloodIfBledPrey(u8),
    /// As `PeekHand`, the hand of the host's controller (Iris Bennett's
    /// older blocker).
    PeekHostControllersHand,
    /// As `PeekHand`, the hand of the acting minion's controller (Netwar
    /// [aus]).
    PeekActingControllersHand,
    /// Unlock the acting minion, of the clan if any, if its capacity is n
    /// or more, read at resolution (SchreckNET, Well-Marked).
    UnlockActorIfClanCapacity { clan: Option<&'static str>, n: u8 },
    /// Rider effect: the controller's predator takes control of this card
    /// (The Coven); requirements not re-checked, its owner's on leaving
    /// play (spec §6.3).
    PredatorTakesControl,
    /// Rider effect: control passes counter-clockwise to the first
    /// Methuselah controlling a ready vampire of the sect, back to its
    /// controller at most; burned if none (The Admonitions; LSJ 20031121).
    PassesToSect(Sect),
    /// Rider effect: n unpreventable damage to each vampire of the clan and
    /// ally of the subtype its controller controls that did not bleed
    /// successfully this turn (Song of Pan).
    DamageUnbledThisTurn {
        clan: &'static str,
        label: crate::library::Label,
        n: u8,
    },
    /// Rider effect: burn this card (Guardian Angel).
    BurnSelf,
    /// Burn the card in play granting this action, with
    /// `DeclareAction(BurnCard)`.
    BurnSource,
    /// Move X blood from the target minion to the player's pool.
    MoveBloodTargetToPoolX,
    /// Push a continuous layer sourced by this card while it is in play.
    Layer(Modifier),
    /// Push a layer scoped to the current action.
    ActionLayer(Modifier),
    /// Cancel the card being played; `refund`: its cost is not paid.
    CancelPlayed { refund: bool },
    /// The action is directed at the source card's controller, wherever
    /// its target (Black Hand Emissary's move).
    DirectedAtSourceController,
    /// Declare an action of this kind with this card (action cards).
    DeclareAction(ActionKind),
    /// +n stealth for the current action.
    AddStealth(u8),
    /// +n intercept for the actor's current block attempt.
    AddIntercept(u8),
    /// The minion attempting to block gets n intercept for the rest of the
    /// action (Oskar Anasov's +1, Gwenllian Arwyn's -1).
    AttemptingIntercept(i8),
    /// Marker of a granted bleed: no equipment but the granting one may
    /// increase its bleed amount (Camera Phone).
    NoOtherEquipmentBleed,
    /// +n bleed for the current bleed; `limited` binds the rulebook cap.
    BleedMod { n: u8, limited: bool },
    /// The actor wakes for the rest of the action.
    Wake,
    /// Lock the actor.
    LockActor,
    /// Ability cost: lock the card's host.
    LockHost,
    /// Ability cost: lock n unlocked cards of this kind on the host, this
    /// one first (Aye: "lock three of his Aye").
    LockHostHeld(Holding, u8),
    /// Vampires cannot block this action (Daring the Dawn).
    VampiresCannotBlock,
    /// "Vampires with capacity less than n and monster allies who cost less
    /// than m pool cannot block this action" (Project).
    SmallMonstersCannotBlock {
        capacity_below: u8,
        ally_pool_below: u8,
    },
    /// "Non-infernal vampires with capacity less than n cannot block this
    /// action" (Notorious Brutality).
    SmallNonInfernalCannotBlock(u8),
    /// n unpreventable environmental aggravated damage if the action
    /// reaches resolution, after combat if blocked (Daring the Dawn; RTR
    /// 19980623, ANK 20220331).
    DamageAfterResolution(u8),
    /// The chosen vampire cannot block this action (Seduction, The
    /// Sleeping Mind [dom]).
    TargetCannotBlock,
    /// Minions cannot unlock during this action; wakes still work (The
    /// Sleeping Mind [DOM]; PIB 20150703).
    NoUnlockThisAction,
    /// The chosen minion cannot play reaction cards this action
    /// (Unleashing the Bestial Soul).
    TargetNoReactions,
    /// The next reaction card played this action costs +n blood or life
    /// (Unleashing the Bestial Soul [DOM]).
    NextReactionCosts(u8),
    /// Reduce a bleed against the user to 0, a reduction by the bleed
    /// amount as it resolves (Visions of Zapathasura [OBF][PRE]).
    ReduceBleedToZero,
    /// Unlock the minion, the action ends unsuccessfully, not again this
    /// turn (Change of Target); also before an unblocked action's combat
    /// (Obedience, rules-qa 55.7).
    EndBlockedAction,
    /// The next X actions of the user's minions this turn get +1 stealth, X
    /// blood burned from the actor (Veil the Legions [OBF]).
    StealthForNextActions,
    /// End the combat at once, outside strike resolution: no strike
    /// resolves, the end-of-round window still opens (advanced rules
    /// §4.7.2, §4.9).
    EndCombat,
    /// The actor steals the target's title: the target loses it, the actor
    /// gains it (Valerius).
    StealTargetTitle,
    /// Put 1 graft counter on the target ally (Dr. Morrow).
    GraftOnTarget,
    /// Each other Methuselah secretly picks 1 to 5, the player 1 to 6;
    /// revealed, each gains their number, or burns it if another picked
    /// exactly 1 less (Game of Malkav).
    GameOfMalkav,
    /// "You can add 1 counter to this card/location" (Cursed Abattoir, as a
    /// zombie of the controller's burns; Form of Corruption, as the prey
    /// gets the Edge anew; rules-qa 55.6).
    MayAddCounter,
    /// The card inflicts n damage on its bearer (Charnas the Imp, in the
    /// employer's controller's unlock phase).
    DamageHost(u8),
    /// Its bearer about to be burned, the bearer's controller may put the
    /// card on another minion (Charnas the Imp).
    MayMoveOffBurnedBearer,
    /// The card goes on its striker as its strike resolves, in range or
    /// not; not if it never resolves — combat ended first (ANK 20200203-1)
    /// or dodged (advanced rules §4.6.2) — (Molotov Cocktail).
    PutOnSelfAsStrikeResolves,
    /// A weapon burned once its strike resolved ("burn after use",
    /// Molotov Cocktail).
    BurnAfterStrike,
    /// The ammo's effect on the gun its player's combatant strikes with,
    /// for the rest of the combat (`CombatCtx::ammo`).
    LoadGun(Ammo),
    /// Its strike: combat ends, then the opposing vampire, not dodging, and
    /// the vampire bearer at close range lock and do not unlock as normal
    /// next (Flash Grenade).
    StunsAfterCombat,
    /// The card's controller chooses a ready minion they control to take n
    /// unpreventable damage, the card's, not its bearer's (Talbot's
    /// Chainsaw; LSJ 20090304-1).
    ControllerChoosesMinionDamaged(u8),
    /// The actor moves n blood from itself to the target vampire (Mustafa).
    ActorBloodToTarget(u8),
    /// The target vampire's votes and ballots are cast on this side, as the
    /// user chooses: a cast of its own turned, the last such use deciding
    /// (Mustafa; LSJ 20090304-2).
    CastTargetVotes(bool),
    /// Change the bleed's target to the target seat (reopens blocks).
    RedirectBleed,
    /// Change the bleed's target to the acting minion's controller's prey
    /// (reopens blocks); the next card that would change it is canceled as
    /// played (Two Wrongs).
    RedirectBleedToActingPrey,
    /// n votes to the targeted vampire for the current referendum (Census
    /// Taker).
    TargetVotes(u8),
    /// The top X library cards looked at, X the Gehenna cards in play
    /// (Reverend Adams).
    LookAtLibraryTopPerGehenna,
    /// n damage on the opposing minion after combat, at any range
    /// (Arianne, The Conqueror).
    DamageAfterCombat(u8),
    /// n cards burned from the top of the bled Methuselah's library
    /// (Hecate).
    BurnBledLibraryTop(u8),
    /// The action is undirected, whoever controls its target (Shattering
    /// [dem]).
    UndirectedAction,
    /// Burn the acting minion.
    BurnActingMinion,
    /// n damage on the acting minion, out of combat, after resolution
    /// (Touch of Pain).
    DamageActing(u8),
    /// The acting minion's cards of this kind burn (Car Bomb: its vehicle).
    BurnActingHeld(Holding),
    /// The action gets -n stealth, a played card's (Car Bomb).
    ActionStealthMinus(u8),
    /// The player draws a card, then moves a card from hand to the ash heap
    /// or the top of the library (Aisata Swanou; `Interrupt::HandCardAway`).
    DrawThenHandCardAway,
    /// The player discards up to n cards chosen from hand, one at a time,
    /// replaced once done (Reliquary: Trinket).
    DiscardUpTo(u8),
    /// The target minion gets +n intercept for the rest of the action
    /// (Pallid).
    TargetInterceptThisAction(u8),
    /// n unpreventable damage on the acting minion and its blocker, out of
    /// combat (Make an Example).
    DamageActingAndBlocker(u8),
    /// The action is not successful.
    ActionFails,
    /// The action fails unless the acting minion burns n blood; an ally
    /// cannot (Detect Authority [ani]), or pays life with `or_life`, its last
    /// life burning it (Scobax [STR]).
    ActingBurnsOrFails { n: u8, or_life: bool },
    /// The acting vampire's next blood gain this action is 1, this reacting
    /// vampire gaining 1 too (Final Loosening [aus]).
    SplitBloodGain,
    /// The acting minion does not unlock as normal during its next unlock
    /// phase, by a card played on the action (Faerie Wards [MYT]).
    ActingSkipsNextUnlock,
    /// Reaction cards are not replaced until the end of the action.
    DeferReactionReplacement,
    /// If this action is blocked, lock the blocking minion and end the
    /// action before block resolution (Mirror Walk [THA]).
    EndActionIfBlocked,
    /// If this action is blocked (by a non-titled vampire when
    /// `untitled_blocker`), the acting vampire may cancel the resulting
    /// combat, burning `blood` and unlocking if `unlock` (Stone Travel
    /// [MYT], Sowing Dissension).
    MayCancelCombatIfBlocked {
        blood: u8,
        unlock: bool,
        untitled_blocker: bool,
    },
    /// The card (a retainer) enters play on the acting minion with this life.
    EmployRetainer { life: u8 },
    /// Burn n life from this card (a retainer's, an ally's own: Crypt's
    /// Sons, Underbridge Stray), the last burning it; an ally pays it.
    BurnOwnLife(u8),
    /// The action continues as if unblocked, before block resolution
    /// (Crypt's Sons).
    ContinueUnblocked,
    /// Take control of the target ally, the cards on it following
    /// (Entrancement [PRE], Far Mastery [DOM]; `takes_control`).
    StealTargetAlly,
    /// Rider effect: move n blood from the host to the minion that played
    /// this card (`CardInPlay::by`; Slaughtering the Herd [DOM]).
    HostBloodToBy(u8),
    /// Allies cannot block this action (Visions of Gehenna, Invigorate
    /// [dom]).
    AlliesCannotBlock,
    /// The acting Anarch gets +n strength this action (Invigorate [ani]).
    StrengthThisAction(u8),
    /// The acting Anarch's hand strikes are aggravated this action
    /// (Invigorate [pro]).
    AggravatedHandsThisAction,
    /// The acting minion's hand strikes inflict +n damage this action
    /// (Uncontrollable Rage).
    HandStrikeDamageThisAction(u8),
    /// This ally cannot gain life, set as it enters (Aggressive Corpse).
    CannotGainLife,
    /// This ally can act the turn it is recruited (Spectral Servitor).
    ActsWhenRecruited,
    /// Unlock the recruiter on its first successful recruit ally action
    /// this turn, read by the recruit's resolution (Spectral Servitor).
    UnlockRecruiterOnFirst,
    /// The recruiter may perform this action again this turn, its cost +1
    /// blood for each time this turn (Nocturn [OBT]; `Minion::repeats`).
    MayRepeatThisTurn,
    /// Rider effect: in the unlock phase, burn n pool instead of this ally,
    /// the controller's choice (Spectral Servitor [OBL]; `Flow::RiderTerms`).
    BurnSelfUnlessPool(u8),
    /// Move X blood from the target minion to the host, as life (The Vozhd
    /// of Juiz de Fora; X the terms).
    StealBloodToHostX,
    /// Add n blood or life to the target minion, not to exceed starting
    /// life (a vampire's capacity; Seraphina).
    BloodToTargetToStart(u8),
    /// Move n blood from the host to the target minion, not to exceed its
    /// starting life (Sri Sansa).
    HostBloodToTarget(u8),
    /// Move n blood from the host to the target vampire in the user's
    /// uncontrolled region (Saulot, The Wanderer).
    HostBloodToUncontrolledTarget(u8),
    /// Move n blood or life from the target minion to the host, blood above
    /// its capacity draining as usual (Sri Sansa); an ally left with no
    /// life burns.
    TargetBloodToHost(u8),
    /// The host enters combat with the acting minion, which acts first, the
    /// combat over returning to the after-resolution window (The Guardian;
    /// ANK 20221102).
    EnterCombatWithActing,
    /// The target minion enters combat with the acting minion, which acts
    /// first and has no strikes in round one; then back to the
    /// after-resolution window (The Status Perfectus).
    TargetEntersCombatWithActing,
    /// The prey chooses a ready minion they control, as it resolves, to
    /// take n unpreventable damage (Gnaeus Aemilius Augustinus).
    PreyChoosesMinionDamaged(u8),
    /// The acting minion gets -n stealth (Screamer).
    ReduceStealth(u8),
    /// Unlock this card; an ally's own card unlocks the ally (The Vozhd of
    /// Szczecin).
    UnlockSelf,
    /// The action card costs n less blood or pool at resolution (Szlachta
    /// Assistant).
    ReduceActionCost(u8),
    /// The target minion burns n blood (Bone Shambler [OBL]).
    TargetBurnsBlood(u8),
    /// The acting minion (`with_blocker`: the blocker) enters combat with
    /// the actor once the after-combat window closes, unable to strike in
    /// its first round; `press`: one for the actor in it (Fast Reaction,
    /// Hidden Lurker).
    QueueCombatAfterBlock { with_blocker: bool, press: bool },
    /// Look at each Methuselah's hand and discard a card from among them,
    /// one's own included; its owner replaces it (Vaticination).
    DiscardFromAllHands,
    /// If this referendum passes, the next one a vampire of the player's
    /// calls passes automatically, no "this turn" (Malkavian Rider Clause).
    NextReferendumPassesIfPasses,
    /// The opposing vampire is burned instead of going to torpor by the
    /// damage about to land (Decapitate [POT]).
    BurnOpponentInsteadOfTorpor,
    /// The opposing vampire is diablerized by the actor instead of going to
    /// torpor by the damage about to land (Amaranth).
    DiablerizeOpponentInsteadOfTorpor,
    /// The actor, wounded, goes to torpor only after the combat, a new one
    /// past it (LSJ 20030214); the combat ends after `quiet_rounds` rounds
    /// with no card played (Undead Persistence [for]).
    TorporAfterCombat { quiet_rounds: u8 },
    /// Marker: the bearer's aim cards go on this card once used, canceled
    /// too, and only the bearer plays them from it as if from hand, then
    /// removed from the game (Light Intensifying Goggles).
    BearerAimStore,
    /// Marker: only the bearer plays the cards on this card, as if from
    /// hand; burned once none is left (Gift of Proteus).
    BearerStore,
    /// Marker: the card's counters are removed if it goes to another minion
    /// (Seal of Veddartha; `GameState::set_host`).
    CountersClearOnNewBearer,
    /// Granted action: the target Methuselah takes control of the ally whose
    /// card grants it (Vivienne Géroux).
    TargetSeatTakesSourceAlly,
    /// The target retainer card burns n life, burned with none left (Smash
    /// and Grab [ani]).
    TargetRetainerBurnsLife(u8),
    /// An ally version's own type line (Underbridge Stray [ANI]).
    Version(crate::library::Body),
    /// After this ally enters play, the controller picks an ally or retainer
    /// they control to burn (War Ghoul).
    EntryBurnAllyOrRetainer,
    /// Unlock the actor during the next discard phase (Feral Hound [ani],
    /// on the employ action's success).
    UnlockActorAtDiscard,
    /// The host burns n blood, an ally life (Phantasmagoria, Marialena, Bowl
    /// of Convergence; rules-qa 73.5; as a cost never an ally's, adv. 5.5.1).
    HostBurnBlood(u8),
    /// An ally's action cost in life, paid at resolution like a blood cost
    /// but not one: an ally pays it (Rotting Behemoth).
    PayLife(u8),
    /// After this ally enters play, burn it unless the controller removes
    /// an ally or vampire in their ash heap (Rotting Behemoth): an interrupt.
    EntryRemoveFromAshOrBurn,
    /// As it enters play, up to n equipment cards of its controller's hand,
    /// then ash heap, put face down on this ally (Aaron Fletcher).
    EntryStoreEquipmentUpTo(u8),
    /// As `EntryRemoveFromAshOrBurn`, n cards of any kind (Mulher de
    /// Branco; the ash heap's cards, rules-digest §3.18).
    EntryRemoveAshCardsOrBurn(u8),
    /// The ally whose card this is leaves play, shuffled into its owner's
    /// library, not burned (Mulher de Branco).
    ShuffleHostIntoLibrary,
    /// n damage on the acting minion at once, from an ability (Mulher de
    /// Branco).
    DamageActingNow(u8),
    /// A granted action's blood cost, paid by the actor at resolution;
    /// unpayable then, it fizzles (rules-qa 3.6).
    PayBlood(u8),
    /// A granted action's blood cost, one per counter on the card, as
    /// `PayBlood` (Despiral).
    PayBloodPerCounter,
    /// A granted bleed's +n if the target Methuselah controls no ready
    /// unlocked minion, read at resolution (Codex of the Edenic
    /// Groundskeepers).
    BleedIfTargetHasNoReadyUnlocked(u8),
    /// On a successful granted burn, the host, if still `who`, is locked and
    /// skips its next unlock (Stolen Police Cruiser).
    LockHostSkipNextUnlock(Bearer),
    /// Burn 1 blood from the host of this card in play to unlock it.
    HostBurnBloodToUnlock,
    /// Move n blood from the host of this card to its controller's pool
    /// (Blood Doll, Vessel).
    HostBloodToPool(u8),
    /// Move n pool from the host's controller to the host (Blood Doll,
    /// Vessel).
    PoolToHostBlood(u8),
    /// The target vampire gains n blood from a hunting ground, capped at
    /// capacity; fed by a hunting ground this turn either way (rules-qa 6.5).
    HuntingGroundBlood(u8),
    /// 1 blood to each of the two oldest ready vampires the user controls,
    /// picked as it resolves, each unless already fed by a hunting ground
    /// this turn (Gurchon Hall).
    HuntingGroundTwoOldest,
    /// Enters the uncontrolled region with n training counters, out of play
    /// (`CardInPlay::training`), the last burned making it the vampire
    /// `becomes` (Web of Knives Recruit).
    TrainInUncontrolled {
        counters: u8,
        becomes: &'static Trained,
    },
    /// +X intercept for the host's block attempt, X the ability's terms
    /// (Angelica, The Canonicus).
    AddInterceptX,
    /// Take control of the target until the end of the user's next turn
    /// (Ayelea, The Manipulator; `Minion::temp_skip`; `takes_control`).
    TakeControlOfTargetUntilNextTurn,
    /// Move 1 blood from a crypt card in the prey's uncontrolled region to
    /// one in the user's (Lázár Dobrescu).
    PreySlotToOwnSlot,
    /// Move any pool to vampires the user controls (Gird Minions;
    /// `Interrupt::PoolToVampires`).
    PoolToVampires,
    /// Burn the card in play chosen by the play's `BurnNamedInPlay` terms
    /// (Vessel), if it is still in play.
    BurnChosenInPlay,
    /// Unlock the actor.
    UnlockActor,
    /// Unlock the target minion (Alabástrom).
    UnlockTarget,
    /// Flip a coin: on tails the announced action fails at once, with no
    /// block and no cost (Evan Klein).
    CoinFlipActionFails,
    /// Reveal the library's top card, left on top: a master burns `burn`
    /// blood from the host, else `stealth` stealth, only while needed
    /// (Gathii; ANK 20231031).
    RevealTopCard { burn: u8, stealth: u8 },
    /// Burn the library's top card: requiring `disc`, `stealth` stealth
    /// this action, else the host burns `burn` blood (Tusk).
    BurnTopCardByDiscipline {
        disc: &'static str,
        stealth: u8,
        burn: u8,
    },
    /// Reveal the user's crypt's top card, then shuffle the crypt: a copy
    /// of the host gives it `stealth` (only while needed, ANK 20231031),
    /// another card `strength` this action (Bolesław Gutowski).
    RevealCryptTopForCopy { stealth: u8, strength: u8 },
    /// The vampire whose text it is goes to torpor (Cedrick Calhoun).
    GoToTorpor,
    /// Lock the target minion.
    LockTarget,
    /// The acting minion's controller discards the chosen count
    /// (`Terms::Count`) of cards at random from hand, replaced as normal
    /// (Ilonka).
    ActingControllerDiscardsAtRandom,
    /// The target Methuselah discards n cards at random from their hand,
    /// drawing back up (Josef von Bauren).
    TargetSeatDiscardsAtRandom(u8),
    // Combat effects (`combat`).
    /// Choose this strike for the actor.
    Strike(StrikeKind),
    /// Maneuver now (determine range step): toggle the range.
    Maneuver,
    /// Press now (press step): continue, or end the opponent's press.
    Press,
    /// Bank an optional press for this round's press step.
    OptionalPress,
    /// Bank an optional maneuver for this round (Obedient Flesh [DOM][PRO]).
    OptionalManeuver,
    /// Per counter, 1 pool or a random library card of the ash heap removed
    /// from the game, announced as a count (The Gate of Acheron).
    PoolOrAshCardsPerCounter,
    /// Move the target vampire to its owner's uncontrolled region, as
    /// banished (Pit of Contemplation).
    TargetToUncontrolled,
    /// If another round occurs, the actor gets 1 optional maneuver then.
    NextRoundManeuver,
    /// Additional strikes this round; `limited`: one such source per
    /// combatant per round (rulebook "Additional Strikes").
    AdditionalStrikes { n: u8, limited: bool },
    /// The actor gets a strength of n this combat (replaces the base).
    SetStrength(u8),
    /// The opposing minion gets -n strength this combat (Blight).
    OpposingStrengthMinus(u8),
    /// This combat, the opposing minion cannot use additional strikes or
    /// presses to continue combat (Blight [NEC]).
    OpposingNoAdditionalStrikes,
    /// The actor's hand strikes are aggravated this round.
    AggravatedHands,
    /// The actor's hand damage is aggravated for the rest of the combat
    /// (Bone Spur [PRO]).
    AggravatedHandsThisCombat,
    /// Strike rider, after its `Strike`: once it lands, no Fortitude
    /// prevention cards this round, even cycled (LSJ 20050628).
    FortitudeProofStrike,
    /// Strike rider: once it lands (a dodge stops it, LSJ 20040928), the
    /// opposing weapons inflict no damage this round, its own resolution
    /// included (ANK 20170519).
    NullifyOpposingWeapons,
    /// Each later round of this combat, this vampire may burn n blood to
    /// prevent all damage from the opposing initial strike (Superior Mettle
    /// [FOR]; `Interrupt::MayPreventForBlood`).
    MayBurnToPreventInitialStrikes(u8),
    /// The opposing minion cannot strike: dodge this round (Sanguine
    /// Entrapment [cel][qui]).
    OpposingNoDodgeThisRound,
    /// The opposing minion cannot strike: combat ends this round (Sanguine
    /// Entrapment [CEL][QUI]).
    OpposingNoCombatEndsThisRound,
    /// nR environmental damage to the opponent each round in strike
    /// resolution, dodge or not, after a strike: combat ends (Carrion Crows);
    /// `close`: at close range only (Dark Steel).
    EnvironmentalDamage { n: u8, close: bool },
    /// n environmental damage to the opponent each round in strike
    /// resolution this combat while the target card is in play (Bestial
    /// Vengeance's retainer; RTR 19970630, LSJ 20090616).
    EnvironmentalDamageWhileTarget(u8),
    /// Strike rider: undodgeable; a dodge may still be played, to no effect
    /// (Dust Up [ani]; LSJ 20030902-2).
    UndodgeableStrike,
    /// With its damage strike landing, n damage on each retainer on the
    /// opposing minion (Hálito Negro).
    RetainerDamageWithStrike(u8),
    /// This round, the vampire's strikes cannot be dodged: a dodge has no
    /// effect (Sanguinary Wind; LSJ 20030902-2).
    UndodgeableThisRound,
    /// Strike rider: the strike aggravated, a hand strike (Sculpt the Flesh
    /// [pro]) or a ranged strength one (Smite).
    AggravatedStrike,
    /// Strike rider: as it resolves, dodged or not, the electronic
    /// equipment on both combatants burns (Smite).
    BurnElectronicOnCombatants,
    /// The chosen strike resolves before a normal one (Donnybrook [obf];
    /// rulebook "First Strike"), after a strike: combat ends; a dodge still
    /// protects.
    FirstStrike,
    /// Every strike of the user this round is with first strike (Christopher
    /// Houghton).
    FirstStrikeThisRound,
    /// The controller chooses a ready minion, marked for the host's blocks
    /// until their next unlock phase (Christopher Houghton;
    /// `GameState::unlock_marks`): a printed unlock rider's effect.
    ChooseBlockMark,
    /// The strike, landing, burns an opposing ally or n blood from an
    /// opposing younger vampire, combat ending if it has none left (Liquefy
    /// the Mortal Coil [vic]): a `StrikeKind::CardEffect` strike's effect.
    LiquefyOnHit(u8),
    /// The weapon's strike, landing, burns n blood from the opposing vampire
    /// and lowers its capacity; the weapon burns (Elixir of Distillation).
    DistillOnHit(u8),
    /// No strike this round; next round a stand-in on the vampire offers the
    /// initial strike that diablerizes (Liquefy the Mortal Coil [VIC]).
    StrikeDiablerizesNextRound,
    /// That strike, landing, diablerizes the opposing younger vampire.
    DiablerizeOnHit,
    /// The strike, landing, burns the opposing minion if an ally (Entombment
    /// [obt]): a `StrikeKind::CardEffect` strike's effect.
    BurnOpposingAllyOnHit,
    /// Strike rider: the card goes on the opposing minion (with n counters)
    /// as the strike lands, held until then, else discarded (Sculpt the
    /// Flesh [PRO], Touch of Oblivion [obl]).
    PutOnOpponentOnHit { counters: u8 },
    /// After combat, put this card on the actor: held; lost if the combat
    /// continues (Pass Through Shadow [OBL]; RTR 20020501).
    PutOnActorAfterCombat,
    /// The animal recruited or employed gets +n strength (ally) or damage
    /// (damage retainer) and +life starting life (Tainted Spring).
    AnimalBoostIfSuccessful { n: u8, life: u8 },
    /// Combat ends once this strike has resolved, dodged or not (Anesthetic
    /// Touch [obe]; LSJ 20011210-1, -2, -3), or only undodged if
    /// `unless_dodged` (Autonomic Mastery [DOM]).
    CombatEndsAfterStrike { unless_dodged: bool },
    /// +n strength this round (Bear's Skin).
    StrengthThisRound(u8),
    /// +X strength this round, X the actor's votes (Island of Yiaros;
    /// a priscus's ballots none, RTR 19970630).
    StrengthThisRoundVotes,
    /// Can prevent n damage this round, at will (Bear's Skin).
    PreventThisRound(u8),
    /// Can prevent n damage each round this combat (Bear's Skin [ANI]).
    PreventEachRound(u8),
    /// 1 optional press usable this round only (Form of the Wolf [PRO]; TOM
    /// 19960521).
    PressThisRound,
    /// The actor locks one of its unlocked cards of this kind to unlock;
    /// none, nothing (Devil-Channel: Feet [ABO]: an Aye).
    LockHeldToUnlock(Holding),
    /// Prevent all damage from the opposing strikes this round (Rolling
    /// with the Punches [FOR]).
    PreventOpposingStrikesThisRound,
    /// A strike with a close-range maneuver: at strike declaration the
    /// strike; at the maneuver, it commits the round's initial strike (Slam
    /// [POT]; `Side.committed_card`; window review: maneuver riders commit
    /// early).
    CommittedStrike(StrikeKind),
    /// This combat, the opponent cannot maneuver to close, press to
    /// continue or use equipment: a frenzy effect (Terror Frenzy [ani];
    /// Tranquility Shield cancels it).
    OpposingRestraint,
    /// This combat, combat cards cost the opponent +n blood, end-of-round
    /// cards included (Terror Frenzy [ANI]; ANK 20191221).
    OpposingCombatCardsCost(u8),
    /// This combat, no frenzy cards on this vampire, those used cancelled
    /// (Tranquility Shield).
    FrenzyShield,
    /// This combat, damage beyond the first landed each round is prevented;
    /// `aggravated`: aggravated too (Flesh of Marble [PRO]).
    FleshOfMarble { aggravated: bool },
    /// 1 unpreventable environmental damage to both combatants and their
    /// retainers before range each round; `escalate`: +1 each later round
    /// (Weather Control [THA]).
    WeatherDamage { escalate: bool },
    /// Prevent up to n damage to the target minion or retainer in combat
    /// (Martyr's Resilience [aus][for]), from a minion out of it.
    PreventOnTarget(u8),
    /// As `PreventOnTarget`, X+n, X the play's terms ([AUS][FOR]).
    PreventOnTargetXPlus(u8),
    /// Prevent n of another combatant's non-aggravated damage (Randall).
    PreventNonAggravatedOnTarget(u8),
    /// Equip the actor with the card its terms named, from hand, X blood
    /// (Contraband [OBF]); not an action (LSJ 20080815), so no action cost
    /// reduction (LSJ 20090115-1).
    EquipNamed,
    /// That named card costs `pool` less if it costs pool, else `blood`
    /// less (Children of Stone: 1 or 1; Jack of Both Sides: 1 pool or up to
    /// 2 blood).
    NamedCardOff { pool: u8, blood: u8 },
    /// 1 optional maneuver this round, to close range only (Angel's Gift).
    OptionalCloseManeuver,
    /// If blocked, damage to vampires in the combat is aggravated,
    /// environmental and retainer damage included (Dawn Operation; PIB
    /// 20150603-1); `cancel`: a blocker may cancel its attempt ([for]).
    DawnOperation { cancel: bool },
    /// If this vampire does not block, put this card on them after
    /// resolution, one per vampire (Dogged Pursuit [ANI]).
    PutOnActorIfNoBlock,
    /// If this vampire blocks, put this card on the acting minion, still
    /// controlled by the user (Melange [AUS]).
    PutOnActingIfBlocks,
    /// If they do not block, they burn n blood before resolution; earlier
    /// blocks do not count (Forced Awakening; ANK 20230319).
    BurnIfNoBlock(u8),
    /// If blocked, the blocker's controller burns n pool before block
    /// resolution (Aelswith, Terrifying Visage).
    BlockerControllerBurnsPool(u8),
    /// A blood toll to attempt a block: an ally has none to pay (Seeds of
    /// Terror).
    BlockTollBlood(u8),
    /// Lock the minion attempting to block (Hedonism).
    LockAttempting,
    /// Queue a combat with the minion attempting to block, after block
    /// resolution or a successful action's effects (Hedonism [for]/[pre];
    /// rules-qa 53.3).
    QueueCombatWithAttempting,
    /// On a successful bleed, put this card on the actor after resolution,
    /// one per vampire (Fever Pitch [FOR]/[PRE]).
    PutOnActorIfBleedSuccessful,
    /// On a successful bleed, the actor may burn n blood to draw a crypt
    /// card after resolution (Enthrall [OBF]/[PRE]; `Choice::DrawCrypt`).
    CryptForBloodIfBleedSuccessful(u8),
    /// Forward Momentum's counters: +1 after a successful bleed of one's
    /// path vampire, -1 after an unsuccessful one (`Hook::OwnBleedEnds`).
    BleedOutcomeCounters,
    /// This card's pending bleed gain added now (Forward Momentum's unlock,
    /// before its counters are burned).
    TakeBleedGain,
    /// Two copies of a card in the prey's ash heap, named as the action is
    /// announced, removed at resolution (gone, it fizzles), then the hunt
    /// at +n (Giovanni del Georgio).
    PreyAshPairHunt(u8),
    /// The slave locks to take the blocked combat of a master of its clan,
    /// the acting vampire unlocked (rulebook "Slave"; `ActionCtx::slave`).
    SlaveTakesCombat,
    /// Blocked, the action's block resolves as the host's combat with the
    /// blocker, which locks (Iara; `ActionCtx::stand_in`).
    StandInIfBlocked,
    /// After the end of its controller's minion phase, the host burns and
    /// the top card of their crypt moves to their uncontrolled region
    /// (Tupdog; `Hook::MinionPhaseEnd`).
    BurnHostCryptTopOut,
    /// Strike rider: its damage stays normal whatever would aggravate it
    /// (Psychic Assault).
    NeverAggravatedStrike,
    /// Strike rider: unpreventable if the striker is Black Hand (Psychic
    /// Assault).
    UnpreventableIfBlackHand,
    /// The card entering play gets counters (blood) equal to its target
    /// vampire's capacity, read as it resolves (Powerbase: Barranquilla).
    CountersFromTargetCapacity,
    /// The acting vampire casts no votes or ballots in this referendum
    /// (Confusion of the Eye [OBF]).
    ActingAbstains,
    /// The referendum fails if the calling card requires a title (Confusion
    /// of the Eye [OBF]).
    FailIfTitleRequired,
    /// This combat, each replacement of its player's draws an extra card,
    /// then discards to hand size, copies adding up (Infernal Pursuit [CEL]).
    ExtraDrawPerReplacementThisCombat,
    /// Baltimore Purge's unlock-phase rider: the card burned, a vampire not
    /// older than its host chosen per other Methuselah, all to torpor; by
    /// its controller instead if they control a ready vampire of
    /// `chooser_clan`.
    Purge { chooser_clan: &'static str },
    /// The target retainer's life up to its version's starting life, zombie
    /// bonuses read now (Renewed Vigor [obe]).
    RetainerToStartingLife,
    /// The card's host moves from torpor to the ready region (Renewed Vigor
    /// [for]).
    HostLeavesTorpor,
    /// Burn X counters from this card to reduce a bleed by X, X the
    /// ability's terms (Illusory Resources).
    ReduceBleedX,
    /// +n hunt for this action (Hospital Food).
    HuntBonusThisAction(u8),
    /// As `PutOnActorNow`, copies stacking: no one-per-vampire limit (Focus
    /// the Blood).
    PutOnActorNowStacking,
    /// The card's lasting effect on this vampire, the card itself in the
    /// ash heap (Inner Essence, rules-qa 68.12): an effect-only card in
    /// play carrying the mode's layers.
    LastingOnActor,
    /// An action card's hunt at +n: 1 + n blood, hunt bonuses included, as
    /// it resolves (Abactor).
    HuntAt(u8),
    /// After that hunt, if successful, a blood hunt referendum on the actor,
    /// no diablerie (Abactor).
    BloodHuntIfHuntSuccessful,
    /// Burn the top n cards of the player's library (Liquidation).
    BurnLibraryTop(u8),
    /// Search library and/or hand for a mage ally the host could play, put
    /// in play paying its cost, the host's pick at resolution (Antonio
    /// d'Erlette; `Interrupt::AllySearch`).
    SearchMageAlly,
    /// May gain n pool (Matasuntha; `Interrupt::MayGainPool`).
    MayGainPool(u8),
    /// +n hand size until the next discard phase (Fotini Katsikaris).
    HandSizeUntilDiscard(u8),
    /// After a successful bleed, n unpreventable damage on each Anarch of
    /// the bled Methuselah, in torpor too (Alonso Petrodon).
    DamageBledAnarchs(u8),
    /// If this vampire blocks, strike cards cost the actor +n blood or life
    /// in the combat (Ensnare a Beast [ANI]).
    StrikeTollIfBlocks(u8),
    /// The sect's minions get n intercept during this action (Fiendish
    /// Tongue).
    SectInterceptThisAction(Sect, i8),
    /// On a successful bleed, unlock for n blood in the next discard phase
    /// (`Choice::UnlockForBlood`), the clan read again then (Fiendish Tongue).
    DiscardUnlockIfBleedSuccessful { clan: &'static str, blood: u8 },
    /// At the end of that combat, with only one combatant ready, the
    /// opposing minion's controller burns n pool (Abuse of Power).
    OpposingBurnsPoolIfOneReadyAfterCombat(u8),
    /// At the end of that combat, the opponent not ready, put this card on
    /// the actor whenever it can be, one per vampire (Pillars Fall).
    PutOnActorIfOpposingNotReadyAfterCombat,
    /// In the prey's unlock phase, 1 pool per vampire in torpor they
    /// control; none, burn this card (Augury of Doom).
    PreyBurnsPoolPerTorpor,
    /// Put this card on the opposing minion, its player keeping control
    /// (Hunting the Quarry [cel]/[tha]).
    PutOnOpposingKeepControl,
    /// Burn the card granting this action as it is declared: not a cost,
    /// an action's costs paid at resolution (Hunting the Quarry; rules-qa
    /// 54.4).
    BurnSourceOnDeclare,
    /// Once this round, burn n blood for 1 maneuver to close range only
    /// (Dance with the Devil [OBF]/[THA]).
    CloseManeuverForBlood(u8),
    /// Once this round, burn n blood for a maneuver, either way (Quick Meld
    /// [PRO]).
    ManeuverForBlood(u8),
    /// This combat, the actor's hand strikes inflict +n damage (Rage of
    /// Apedemak).
    HandStrikeDamageThisCombat(u8),
    /// This combat, the actor cannot cancel frenzy cards nor press to end
    /// combat (Taming the Beast).
    NoFrenzyCancelNorEndPress,
    /// The actor treats aggravated damage as normal this round; `lock`: one
    /// of its cards of this kind locked, for the rest of the action (Jua
    /// Vema).
    AggravatedAsNormal { lock: Option<Holding> },
    /// This round's hand strike +X, X the actor's cards of this kind
    /// (Invoke Poison Glands [ABO]: Orun).
    HandStrikeBonusPerHeld(Holding),
    /// This combat, +n hand size (Rage of Apedemak).
    HandSizeThisCombat(u8),
    /// The chosen Sabbat vampire enters combat with the actor before the
    /// referendum, which goes on only if the actor is still ready (Yawp
    /// Court; rules-qa 54.1).
    YawpCourtCombat,
    /// A combat card put on its vampire as it resolves, usable for the
    /// rest of the combat (Wall of Filth).
    PutOnActorNow,
    /// An aim: if any strike damage is inflicted, +n of its kind and no
    /// opposing press this round (Target Vitals; rules-qa 55.1).
    AimExtraDamage(u8),
    /// The opposing minion's controller may discard n combat cards to
    /// cancel this card as it is played (Target Vitals).
    CancelableByDiscardingCombat(u8),
    /// Up to n blood from the opposing vampire to this card in play, back
    /// to it after combat as the card burns (Morbidity).
    MorbidityTake(u8),
    /// 3 counters, reset by each card played; each press step burns one for
    /// a press to continue, no combatant's (Trap; LSJ 20040521), before any
    /// other press (ANK 20200525-1); burned as the combat ends.
    TrapPresses,
    /// As Trap, the press this vampire's, mandatory unless `optional`; no
    /// equipment this combat (Conquer the Beast).
    BeastPresses { optional: bool },
    /// The ash heap ally named at play removed from the game for n life to
    /// the target zombie ally (Putrescent Sustenance).
    EatAshAllyToTarget(u8),
    /// The acting ally removed from the game at resolution, to burn the
    /// target ally or retainer (Heartrender; rules-qa 55.4).
    RemoveActorToBurnTarget,
    /// The pool this action's bleed burns, at most n; the bleed amount
    /// untouched (Night Moves: "ignore the excess"). Read from the card.
    BleedPoolCap(u8),
    /// The acting minion's controller burns n pool if it is not ready at the
    /// end of the action, blocked or not (Rumble). Read from the card.
    BurnPoolAtEndIfNotReady(u8),
    /// Look at and reorder the top n cards of the library, then unlock if
    /// so (Eulogio Sánchez de los Reyes; Maris Streck without).
    ReorderTop { n: u8, unlock: bool },
    /// An ability's look at and reorder of the top n library cards, then
    /// their move to the bottom if its user so chooses (St. Patrick's
    /// Cathedral).
    ReorderTopMayBottom(u8),
    /// After resolution, the actor may burn n blood to unlock the acting
    /// ally (Gifts From Hereafter [DOM]/[OBL]).
    UnlockActingForActorBlood(u8),
    /// +1 life to the target ally, not above its starting life (Burial
    /// Site Hunting Ground), marking the hunting ground used.
    HuntingGroundLife(u8),
    /// On an equipment: the successful action equipping it unlocks the
    /// acting minion at the end of the turn (Baseball Bat).
    EquipActionUnlocksEndOfTurn,
    /// Offered as a combat of its controller's vampire of this clan ends:
    /// lock to enter combat with the opposing minion (Marie-Pierre;
    /// `Hook::OwnClanCombatEnded`).
    MayJoinCombat(&'static str),
    /// During the unlock phase, its controller burns X pool, X the Gehenna
    /// cards in play (at least 1), or it burns (Watchtower: The Wolves Feed).
    BurnPoolPerGehennaOrSelf,
    /// Offered: burn 1 counter to make the vampire just influenced out
    /// Anarch (Twilight Camp; `Hook::OwnVampireInfluenced`).
    MayMakeAnarchForCounter,
    /// The Anarch's controller, whoever controls the card, may move n bank
    /// blood to a vampire in their uncontrolled region (Flames of
    /// Insurrection; `Hook::CombatOpponentOfAnarchLeft`).
    AnarchControllerMayBloodToUncontrolled(u8),
    /// Offered: lock the card for a torture counter on the opposing vampire
    /// sent to torpor (The Path of Lilith;
    /// `Hook::OwnNonCamarillaSendsToTorpor`).
    MayLockForTorture,
    /// The host ally burns n life as its own text's cost, not a blood cost
    /// (Gravebound Drone; advanced rules 5.5.1); paid as `HostBurnBlood`.
    HostBurnLife(u8),
    /// The target minion burns n blood or life, an ally left with none
    /// burned (Gravebound Drone [OBL]; Smash and Grab [ani], rules-qa 73.5).
    TargetBurnLife(u8),
    /// As `EndBlockedAction`, the acting minion left locked (Obedience
    /// [DOM]).
    EndBlockedActionLocked,
    /// A blocked action continues as if unblocked after its resolution
    /// (Go-getter [OBF]).
    ContinueAfterResolution,
    /// If successful, the actor may burn n blood to unlock after resolution
    /// (Paths in Two Worlds [OBL]).
    UnlockForBloodIfSuccessful(u8),
    /// Unlock rider: the host burns n blood, all or nothing, or is burned;
    /// else this card moves to another justicar or Inner Circle member, or
    /// burns if none (Fear of Mekhet; rules-qa 49.4).
    HostBurnsBloodOrBurnedThenMoves(u8),
    /// Unlock rider: the host burns n blood or is burned, its controller's
    /// choice — burned when it has fewer (Society of Leopold).
    HostBurnsBloodOrIsBurned(u8),
    /// Unlock rider: the host burns n blood or this card is burned, its
    /// controller's choice (Robert Carter).
    HostBurnsBloodOrThisBurns(u8),
    /// Stun the target minion: it locks and skips its next unlock (Mind
    /// Numb).
    Stun,
    /// Stun the rider's host (Hellene Panhard).
    StunHost,
    /// Each Methuselah, from the player on, looks at their crypt's top n
    /// cards and moves one to their uncontrolled region, the others to the
    /// crypt's bottom (Porto de Santos).
    EachCryptTopPick(u8),
    /// Reveal the top of the player's crypt and move it to the bottom: +1
    /// vote to the user if it is an older vampire (Tubarão).
    RevealCryptTopVoteIfOlder,
    /// +n transfers this turn: its influence phase's, from its minion phase
    /// (Julia Sowinski).
    TransfersThisTurn(u8),
    /// Titled vampires cannot block this action (Propaganda).
    TitledCannotBlock,
    /// The target Methuselah locks the ready unlocked minion named as the
    /// card is played (Propaganda [PRE]).
    TargetLocksReadyUnlocked,
    /// Steal a location; one on a minion moves onto a minion of the new
    /// controller's (Dominate Kine [DOM]; RTR 19960112, 19980623).
    StealLocation,
    /// Take control of the target minion for good, the cards on it
    /// following (Graverobbing; `takes_control`); an ally into a unique
    /// card's contest too (From a Sinking Ship).
    StealTargetVampire,
    /// The actor may burn n blood to move the stolen vampire to the ready
    /// region: a burn, not a cost (Graverobbing [DOM]; ANK 20210226).
    StolenToReadyForBlood(u8),
    /// Burn the target minion (Judgment: Camarilla Segregation).
    BurnTargetMinion,
    /// Burn the target minion at the second such action of the actor at
    /// it, in a later turn; the first is recorded (Quincy, The Trapper;
    /// ANK 20170202; `Minion::progress`).
    BurnTargetOnSecondAction,
    /// A granted "burn this card" action's alternative: shuffle the card
    /// into its owner's library (Aranthebes, The Immortal).
    ShuffleSourceIntoLibrary,
    /// No counters left: burn this card, a matching ally from the ash heap
    /// put in play, not recruited: inferior version, no requirement or
    /// cost, able to act this turn (Split the Veil; LSJ 20100204).
    RaiseAllyWhenEmpty(&'static [CardFilter]),
    /// Move the vampire of the player's ash heap named as the action was
    /// declared to their uncontrolled region, if still there (Hand of
    /// Conrad).
    AshVampireToUncontrolled,
    /// A matching ally from the ash heap put in play locked, named as the
    /// action is declared (Hel-Blá; rules-qa 41.6), as Split the Veil's
    /// (LSJ 20100204).
    AllyFromAshLocked(&'static [CardFilter]),
    /// Remove the ash heap ally named at play (`PlayCtx::ash`), if still
    /// there, to gain n blood (and unlock) (Psychophagia).
    EatAshAlly { blood: u8, unlock: bool },
    /// At `at` counters, burn this card; its controller gains `pool` pool
    /// (Open War; rules-qa 32.7).
    AtCountersBurnAndGain { at: u8, pool: u8 },
    /// +1 master phase action this phase (The Parthenon).
    GainMasterPhaseAction,
    /// Burn X pool (the ability's terms), paid down to 0 ousts (Oxford
    /// University, England).
    PayPoolX,
    /// The Methuselah's own `per` × X votes, on one side (Oxford
    /// University, England).
    MethuselahVotesX { per: u8, favor: bool },
    /// The target vampire gains blood to its capacity (Giant's Blood).
    BloodToTargetFull,
    /// Remove the target minion from the game and gain pool: a vampire's
    /// capacity (Golconda), an ally's printed recruit cost (Khobar Towers;
    /// no ally of the pool costs X, whose X would be the one paid, ANK
    /// 20170716).
    RemoveTargetForPool,
    /// The target minion's controller can burn n pool to cancel this card
    /// as it is played (Golconda: Inner Peace).
    CancelableForPoolByTargetController(u8),
    /// Move X blood from the actor to the uncontrolled target (Grooming the
    /// Protégé).
    MoveBloodXToUncontrolledTarget,
    /// The user's minion in the combat gets 1 press only usable to
    /// continue combat (Mob Connections).
    ContinuePressForOwnCombatant,
    /// Draw n cards, then discard down to hand size (Fragment of the Book
    /// of Nod).
    DrawThenDiscardDown(u8),
    /// If the attempting minion does not block, this card's controller
    /// burns 1 pool after resolution (WMRH Talk Radio).
    PoolIfAttemptingDoesNotBlock,
    /// The target vampire can burn 1 blood for +1 intercept during the
    /// action, once (The Rumor Mill, Tabloid Newspaper; Marciana Giovanni,
    /// rules-qa 79.2).
    TargetBurnsBloodForIntercept,
    /// Remove X counters from this card (X from the ability's terms).
    RemoveCountersX,
    /// The actor gets +X votes (Alamut).
    AddVotesX,
    /// The pool a successful bleed by the acting Methuselah's `clan`
    /// vampire costs goes on this card as counters (Alamut).
    CountersFromOwnBleed(&'static str),
    /// Move n blood from the target vampire to its controller's pool
    /// (Powerbase: Munich).
    TargetBloodToPool(u8),
    /// Move n pool to the target vampire (Powerbase: Munich).
    PoolToTargetBlood(u8),
    /// Move 1 counter from this card to the target as blood (Twilight Camp).
    CounterToTargetBlood,
    /// Search the crypt for a copy of the named vampire, revealed, into the
    /// uncontrolled region (The Mithraeum, London).
    CryptNamedToUncontrolled(&'static str),
    /// Unlock the acting minion (Yoruba Shrine).
    UnlockActing,
    /// The action's card costs n blood less (The Line, during the action).
    ReduceActionBlood(u8),
    /// The action's equipment is paid with X counters of this card (Ravnos
    /// Cache, during the action).
    PayActionWithCountersX,
    /// Move n pool to this card as counters, and add m counters from the
    /// bank (Ravnos Cache).
    PoolToCounterPlusBank { pool: u8, bank: u8 },
    /// Enter play with 1 counter per vampire of these clans the player
    /// controls (Ravnos Carnival), and per ally of these subtypes, each
    /// minion once (In Memory of the Two Lands: Followers of Set and
    /// mummies); with none, burned.
    EnterWithCountersPerOwn {
        clans: &'static [&'static str],
        allies: &'static [crate::library::Label],
    },
    /// Enter play with 1 counter per Anarch controlled; no burn at 0
    /// (Hackerspace).
    EnterWithCountersPerOwnAnarch,
    /// The hand cards picked by the ability's terms go on this card, face up
    /// and out of play, replaced (Fleshforge Chamber; rules-qa 37.1).
    StorePickedFromHand,
    /// The card remembers the vampire of the uncontrolled-region target
    /// (`CardInPlay::chosen_crypt`; Gather).
    ChooseUncontrolledTarget,
    /// Its chosen vampire, still in its controller's uncontrolled region,
    /// to their ready region with its counters, unless it would contest a
    /// vampire in play (Gather).
    ChosenUncontrolledToReady,
    /// n blood from the bank on the uncontrolled vampire the terms picked
    /// (Tend the Flock; ANK 20190311).
    BloodToPickedUncontrolled(u8),
    /// The uncontrolled vampire the terms picked to the ready region, its
    /// counters and cards kept (Thicker than Blood; LSJ 20100206-2).
    PickedUncontrolledToReady,
    /// Up to n cards of the player's library, one at a time, face down to
    /// their research area, the library shuffled after (Research;
    /// `Interrupt::ResearchSearch`).
    SearchLibraryToResearch(u8),
    /// n blood of the vampire that hunted, what it has, onto this card as
    /// counters; at `burn_at` the card burns (Hunger Moon).
    HunterBloodToCounters { n: u8, burn_at: u8 },
    /// If the recruit succeeds, this card goes from the ash heap onto the
    /// ally recruited, with n more life (Ghouled; gone from it, nothing,
    /// ANK 20220805).
    PutOnRecruitedIfSuccessful(u8),
    /// The target and the second target lock and do not unlock as normal
    /// next (Liberty Club Intrigue).
    LockTargetsSkipUnlock,
    /// The card enters play under the recipient's control, and its player
    /// moves a card of this type from the recipient's ash heap onto it,
    /// face up (the Holds; `Interrupt::StoreFromAshOf`).
    GiveHold {
        to: Recipient,
        of: crate::library::CardType,
    },
    /// Discard from hand a copy of the card on this one, replaced (the
    /// Holds).
    DiscardStoredCopy,
    /// Move 1 up to `max` blood, chosen at resolution, from the actor onto
    /// this card as counters; at `burn_at` counters the card burns (Cold
    /// Amber's Hold; `Interrupt::BloodToCardUpTo`).
    ActorBloodToSourceCardUpTo { max: u8, burn_at: u8 },
    /// Put this card on the target card in play (its bearer's), then a card
    /// of the filter from hand on this card, face up (Magazine:
    /// `Interrupt::StoreOwnHand`).
    PutOnTargetCardStoring(CardFilter),
    /// The ammo card on this card used as if played from hand on the gun
    /// it is on, its pool cost paid (Magazine).
    LoadGunFromStore,
    /// Put a card from hand face down on this card as it is played,
    /// replaced (Storage Annex; `Interrupt::StoreOwnHand`).
    StoreFromOwnHand,
    /// Put X cards of the filter from hand face down on this card as it
    /// resolves, X announced with the play (`Terms::HandCards`), all there
    /// are if fewer; not replaced (Gift of Proteus; PIB 20150428).
    StoreFromHandX(CardFilter),
    /// Ability terms: the card picked from hand and the one on this card
    /// change places, neither replaced (Storage Annex; PIB 20150224).
    SwapPickedWithStored,
    /// Its controller's vampires of this clan play the cards on it as if
    /// from the hand (Fleshforge Chamber).
    PlayableFromStore(&'static str),
    /// A card played from this one is removed from the game whenever it
    /// would be burned (The Erciyes Fragments).
    StoredRemovedWhenBurned,
    /// The controller names a library card in their prey's ash heap; it
    /// moves onto this card (The Erciyes Fragments).
    StoreFromPreyAsh,
    /// Move a library card from the ash heap face down onto this card,
    /// picked as it resolves, seen by all (Maabara; ANK 20200523).
    StoreFromOwnAsh,
    /// Move a card from this location to the top of the library (Maabara).
    StoredToLibraryTop,
    /// The referendum fails, its polling over (Yoruba Shrine).
    FailReferendum,
    /// Put 1 of the player's corruption counters on the target; at its
    /// capacity (an ally's cost) they can burn them to steal it (Cave of
    /// Apples).
    CorruptTargetMayStealAtCapacity,
    /// That Methuselah locks one of their oldest vampires of the clan, their
    /// pick among ties, a locked one allowed (Brujah Debate; LSJ 20010508).
    LockOldestOfClan(&'static str),
    /// Put this card itself on the opposing minion now, its layers while
    /// there (Disarm; rules-qa 47.1).
    PutOnOpponent,
    /// Send the opposing vampire to torpor (Disarm).
    OpponentToTorpor,
    /// Inflict n damage on the opposing minion at once (Pulled Fangs).
    DamageOpponentNow(u8),
    /// Unlock-phase rider: a six-sided die, each `(low, high, effects)` row
    /// giving its effects on a roll in `low..=high`, until its controller's
    /// next unlock phase (Ozzy).
    UnlockDieRoll(&'static [(u8, u8, &'static [Lasting])]),
    /// The player looks at a card at random from their predator's hand; the
    /// user may burn 1 blood to exchange it with one of that predator's ash
    /// heap (Jara Drory; `Interrupt::ExchangeLooked`).
    LookAtRandomPredatorHandMayExchange,
    /// On success, n damage to each vampire of this sect the target
    /// Methuselah controls (Jaroslav Pascek).
    DamageSeatSectVampires { sect: Sect, n: u8 },
    /// Ability cost: lock X unlocked cards of this kind on the host, X the
    /// ability's terms (Jibade el-Bahrawi).
    LockHostHeldX(Holding),
    /// Flip a coin: heads, +n bleed this action (Esau).
    CoinFlipBleed(u8),
    /// The blocking minion's controller plays with an open hand in the
    /// combat to come (Fleurdumal).
    BlockerControllerOpenHand,
    /// On success, the target unlocks and its controller's is the acting
    /// Methuselah until the end of the turn (Gengis).
    UnlockAndControlTargetUntilEndOfTurn,
    /// The player looks at a card at random from their prey's hand (Gerard
    /// Rafin).
    LookAtRandomPreyHandCard,
    /// The named ready vampire its controller controls burns 1 blood: the
    /// acting minion +1 stealth (Enid Blount's Edith).
    NamedPaysForStealth(&'static str),
    /// Every vampire of the clan gets +n bleed for the rest of the turn
    /// (Annabelle Triabell G3; LSJ 20030811).
    ClanBleedThisTurn(&'static str, u8),
    /// n unpreventable damage on each combatant at once, from a minion out
    /// of the combat (Alejandro Aguirre).
    DamageEachCombatantNow(u8),
    /// n unpreventable damage at once on the minion opposing the user's
    /// combatant, from out of the combat (Ron Zertnert).
    DamageOpposingOwnCombatantNow(u8),
    /// The combatant inflicts n damage on the opposing minion before range,
    /// mended as the before-range window closes (Outside the Hourglass).
    PreRangeDamage(u8),
    /// Its strike prevents n damage of the opposing strike as they resolve
    /// (Stunt Cycle).
    ShieldFromOpposingStrike(u8),
    /// The opposing minion, with Celerity [cel], may burn n blood to prevent
    /// all of this strike's damage (Stunt Cycle; ANK 20210226: no cost).
    OpposingCelerityMayBurnToPrevent(u8),
    /// An additional strike only usable for that strike, none if its object
    /// is gone (Voracious Vermin [ANI]); `limited` (Wind Dance [THA]).
    AdditionalStrikeOnly { kind: StrikeKind, limited: bool },
    /// Burn X blood for X additional strikes, X the play's terms; the blood
    /// is an effect, not a cost (Lightning Reflexes [CEL]; ANK 20210226).
    AdditionalStrikesX { limited: bool },
    /// The actor burns X blood, X the play's terms: an effect, not a cost,
    /// no cost reduction reaches it (ANK 20210226).
    ActorBurnsBloodX,
    /// The actor burns n blood, the cost of an in-play ability it uses
    /// (Meat Hook's destroying strike).
    ActorBurnsBlood(u8),
    /// The target weapon's own strike at +n damage, its limit and riders
    /// included (`Engine::weapon_strike_plus`; spec §4.10.4); which weapons
    /// is the target's filter (Lucky Blow's melee, Strike with Conviction's
    /// any).
    WeaponStrikePlus(u8),
    /// As `WeaponStrikePlus` with a maneuver (Lam Into [for]): at the
    /// maneuver it commits the initial strike (`CommittedStrike`); at strike
    /// declaration it is the strike.
    CommittedWeaponStrikePlus(u8),
    /// The card in play counts as melee weapon equipment (Weighted Walking
    /// Stick).
    BecomesWeapon,
    /// A gun once in play (Zip Gun; `cards::is_gun`).
    BecomesGun,
    /// The vampire does not lock for the block (Read the Winds [ani]).
    NoLockForBlock,
    /// The minion attempting the block burns n blood, as much as it has
    /// (The Kiss of Ra).
    AttemptingBurnsBlood(u8),
    /// Send the blocking vampire to torpor (The Kiss of Ra [FOR]).
    AttemptingToTorpor,
    /// The action ends at once, before resolution, no card played after
    /// (The Kiss of Ra; RTR 20180719): canceled, no NRA mark unless blocked
    /// and continued before (ANK 20211015), still taken (ANK 20210124).
    CancelAction,
    /// Each Methuselah with a vampire with torture counters burns n pool
    /// (The Path of Lilith, its controller's unlock phase).
    TortureBurnPool(u8),
    /// n damage to the acting vampire after resolution, out of combat, once
    /// each action (Neighborhood Watch Commander).
    WatchDamagesActing(u8),
    /// As combat is about to end, a new one with the opposing minion begins
    /// in the action (Psyche! [CEL]): after-combat effects lost but unlocks
    /// (LSJ 19980109, RTR 20020501); no "would end" after it (ANK 20191219).
    QueueCombatWithOpponent,
    /// A new combat between the action's last combatants, beginning as the
    /// after-combat window closes (Pocket Out of Time [TEM]).
    QueueCombatWithLast,
    /// Unlock rider: the active Methuselah burns 1 pool for each hunting
    /// ground they control (Arcanum Chapterhouse, Alexandria).
    BurnPoolPerHuntingGround,
    /// The opposing minion's controller plays with an open hand for the
    /// rest of combat (Aura Reading [aus]).
    OpponentOpenHandThisCombat,
    /// Blocks rider: the acting minion burns n blood or life before combat,
    /// before any other effect (Banshee Ironwail; ANK 20210627).
    ActingBurnsBloodOrLife(u8),
    /// The acting vampire burns n blood (Friend of Mine [obt]).
    ActingBurnsBlood(u8),
    /// The controller gains the Edge (Cyscek).
    GainEdge,
    /// Marker: the referendum this political action calls failing or
    /// canceled, the acting vampire goes to torpor (Theo Bell, advanced).
    TorporIfReferendumFails,
    /// The vampire gains `even` blood if its controller's pool is even,
    /// burns `odd` if odd (Jeremy Talbot).
    BloodByPoolParity { even: u8, odd: u8 },
    /// The vampire with this text is burned (Mukhtar Bey).
    BurnHostVampire,
    /// The acting vampire leaves torpor (Rapid Healing).
    ActorLeavesTorpor,
    /// The target vampire goes to torpor (Sleep of Reason).
    TargetToTorpor,
    /// Each other Methuselah discards n cards at random, replaced: an action
    /// directed at each of them (Border Skirmish).
    EachOtherDiscardsAtRandom(u8),
    /// Each other Methuselah, clockwise, locks an unlocked ready vampire
    /// they control younger than the actor: an action directed at each of
    /// them (Sowing Dissension).
    EachOtherLocksYoungerVampire,
    /// The acting vampire steals `steal` blood from each of one or more
    /// vampires in torpor chosen at resolution, younger (if `younger`)
    /// unless a Gehenna event is in play; one of `clan` moves a card from
    /// the ash heap to hand for each blood stolen (Siphon).
    StealFromTorpid {
        steal: u8,
        younger: bool,
        clan: &'static str,
    },
    /// Host-blocked rider: a younger vampire blocking the host burns n blood
    /// (Paragon).
    YoungerBlockerBurnsBlood(u8),
    /// n counters added to the target card (Church of the Order of St.
    /// Blaise).
    AddCounterToTargetCard(u8),
    /// One of the player's corruption counters on the target burns
    /// (Revelation of Desire [SER]).
    BurnOwnCorruptionFromTarget(u8),
    /// The actor gains n blood if it is a vampire with min or more, read at
    /// resolution (Entrenching; an ally nothing, D16).
    ActorGainsBloodIfAtLeast { min: u8, n: u8 },
    /// The card enters play with n counters per X (Secret Horde).
    EnterWithCountersPerX(u8),
    /// The host's controller discards the whole hand, replaced (Beckett
    /// advanced's rider) or drawn up (Jacob, The Glitch's ability).
    DiscardHand,
    /// A ready vampire of the controller's other than the host burns
    /// `blood`, or they burn `pool` (Afifa, The Herald;
    /// `Interrupt::BloodOrPool`).
    OtherVampireBloodOrPool { blood: u8, pool: u8 },
    /// Look at the target Methuselah's hand and discard up to n cards of
    /// the filter from it, replaced once done (Alicia Barrows).
    DiscardUpToFromTargetHand(u8, CardFilter),
    /// The Methuselah who played the card burns n pool (Cyscek).
    GehennaPlayerBurnsPool(u8),
    /// The controller of the target card's host takes control of this card
    /// (Smear Campaign).
    TargetHostControllerTakesControl,
    /// Weapon strike rider: aggravated damage against the clan, read as the
    /// strike lands (Banshee Ironwail).
    AggravatedAgainstClan(&'static str),
    /// Weapon strike rider: an imbued striking with it inflicts +X, the X
    /// it was equipped with (Living Wood Staff).
    ImbuedStrikesPlusX,
    /// "[n CONVICTION]" printed in one mode: that mode's cost
    /// (`conviction_cost`; Cleave).
    ConvictionCost(u8),
    /// Expiate on this imbued: two rounds in a row at close range, it
    /// still ready and holding 5 convictions, burns it and the opposing
    /// monster as the second round's press step begins; else a
    /// conviction burned as the combat ends.
    Expiate,
    /// The melee weapon chosen (a card target) or this card, put on the
    /// actor just before, is cleaved for the action (Cleave).
    CleaveWeapon,
    /// Weapon rider: Cleave does not burn it (Living Wood Staff).
    SparedByCleave,
    /// After a two-card discard cost, the same two (the use's X is 1) shrink
    /// each other hand size by n to their next unlock (Babalawo Alafin).
    OthersHandSmallerIfSame(u8),
    /// Search the crypt for a vampire of the clan (None: any vampire, Evil
    /// Jensen), revealed, into the uncontrolled region with `blood` from the
    /// bank, the player's pick or none (Disciple of Lazarus, Coroner's
    /// Contact).
    CryptClanToUncontrolled {
        clan: Option<&'static str>,
        blood: u8,
    },
    /// Distribute n blood from the bank among the player's uncontrolled
    /// vampires of the clan, one at a time, all of it (Great Symposium).
    BankBloodAmongUncontrolled { n: u8, clan: &'static str },
    /// Move the named ally from the ash heap to the ready region, unlocked,
    /// with its starting life (Emerald Legionnaire; ANK 20180509).
    RaiseNamedAllyFromAsh(&'static str),
    /// Shuffle the hand into the library and draw up; the draws are
    /// replacements (Nicomedes; ANK 20180925-1).
    ShuffleHandDrawUp,
    /// Move n blood from a prey's uncontrolled vampire (the user's pick) to
    /// the prey's pool (Solomon Batanea).
    PreyUncontrolledBloodToPool(u8),
    /// Entry rider: distribute up to n blood to uncontrolled vampires of the
    /// clan, before any contest (Erlik; LSJ 20080526).
    DistributeToUncontrolled { n: u8, clan: &'static str },
    /// Remove n cards of the target Methuselah's ash heap from the game, the
    /// player's choice (Trochomancy; LSJ 20090710), at once (PIB 20120525).
    RemoveBleedTargetAshCards(u8),
    /// Prey's unlock rider: per location, burn n pool or the location, one
    /// choice each, in their unlock effects' order (Arika; ANK 20200629).
    PreyPaysOrBurnsLocations(u8),
    /// Each other Methuselah's unlock phase: burn n pool or lose the next
    /// influence phase's transfers (Leandro; `Interrupt::PoolOrNoTransfers`).
    BurnPoolOrNoTransfers(u8),
    /// Search the library for a card of the filter into the hand, shown
    /// to all, then discard down (Fortschritt Library;
    /// `Interrupt::LibraryToHand`).
    SearchLibraryToHand(CardFilter),
    /// As `SearchLibraryToHand`, a haven found then put on the actor if
    /// its player likes, its cost paid (Goblinism [MYT];
    /// `Interrupt::PutFoundOn`).
    SearchLibraryToHandOrHavenOnActor(CardFilter),
    /// The hands of each Methuselah with a ready vampire of the clan, open
    /// to the user until the next action ends (The Baron).
    PeekHandsOfReadyClan(&'static str),
    /// Mark the target's bleed toll for the host (Donatello Giovanni;
    /// `GameState::unlock_marks`).
    MarkBleedToll,
    /// The player's next decision may play a combat card from the ash heap
    /// (Spiritual Guidance).
    PlayCombatFromAsh,
    /// The named cards (any copy, the current one aside) to the library top,
    /// a named card gone skipped (Sudario Refraction; LSJ 20090722).
    SudarioRefraction,
    /// The host gets +n bleed this turn (Andre LeRoux).
    HostBleedThisTurn(u8),
    /// Unlock rider: lock this card or burn n pool, the controller's choice
    /// as it resolves (Bleeding the Vine).
    LockOrBurnPool(u8),
    /// Entry rider: draw 1 card from the crypt (New Blood).
    DrawCryptOnEntry,
    /// As New Blood enters play, his circle, chosen among the crypt's
    /// (`Interrupt::ChooseCircle`).
    ChooseCircleOnEntry,
    /// Entry rider: burn n blood or be burned, the controller's choice
    /// after the excess drained (Hermana Hambrienta; LSJ 20051126).
    BurnsBloodOrIsBurnedOnEntry(u8),
    /// Lock the minion that blocked (Claiming the Body).
    LockBlocker,
    /// The Blood Brothers picked (`Terms::CirclePicks`) lock (Shell Game).
    LockPicks,
    /// +1 stealth for each Blood Brother picked (Shell Game).
    StealthPerPick,
    /// If the action is successful, its player may unlock up to n of the
    /// Blood Brothers picked (Shell Game).
    UnlockPicksIfSuccessful(u8),
    /// Each Blood Brother picked burns 1 blood (Walk of Caine).
    PicksBurnBlood,
    /// +1 bleed (limited) for each Blood Brother picked (Walk of Caine).
    BleedPerPick,
    /// +1 bleed (limited) per 10 cards in the bled Methuselah's ash heap
    /// (Inevitability of the Void).
    BleedPerTargetAshTens,
    /// +1 bleed (limited) per ready vampire of the clan the actor is enslaved
    /// to its controller controls (Unleash the Hounds).
    BleedPerEnslavingReady,
    /// The other Blood Brother gets n additional strikes (limited) every
    /// round, after the normal pair, as Renegade Garou's (Octopod).
    AdditionalStrikesEachRound(u8),
    /// An optional press this round, only usable to continue combat
    /// (Three's a Crowd).
    ContinuePressThisRound,
    /// The strike not usable against a minion with flight, still aimed at
    /// its retainers (Earthshock; LSJ 20060509).
    NotAgainstFlight,
    /// 1 optional press this round, only usable to end combat (Cold Aura
    /// [NEC]; TOM 19960521).
    EndPressThisRound,
    /// The opposing vampire burns n blood (Thin Blood).
    OpposingBurnsBlood(u8),
    /// The opposing vampire, its strike this round a hand strike, burns n
    /// blood if its capacity is at most `cap` (Blood Shield; LSJ
    /// 20090528-1).
    OpposingHandStrikerBurns { n: u8, cap: u8 },
    /// The opposing minion cannot maneuver this round, a strike's maneuver
    /// included (Lapse [tem]).
    OpposingNoManeuverThisRound,
    /// For the rest of the combat, the opposing minion cannot maneuver or
    /// press to continue, and its strikes but "combat ends" cost it 1 more
    /// blood or life (Imprison).
    OpposingImprisoned,
    /// For the rest of the combat, the damage this minion takes from hand
    /// strikes, melee weapon strikes and retainers is inflicted on the
    /// striker or the retainer too, prevented or not (Inflict).
    ReflectsDamageThisCombat,
    /// The opposing minion cannot strike in this round's initial strike
    /// step, its strike's maneuver included; additional strikes stay
    /// (Lapse [TEM]; LSJ 20001126).
    OpposingNoInitialStrikeThisRound,
    /// The opposing minion cannot play strike cards this combat, one played
    /// for its maneuver included (Thoughts Betrayed [DOM]; ANK 20210928-2).
    OpposingNoStrikeCardsThisCombat,
    /// n environmental damage on the opposing minion in this round's
    /// strike resolution, a dodge or not (Thoughts Betrayed [dom]).
    EnvironmentalDamageThisRound(u8),
    /// Prevent half (rounded up) of the opposing strike's damage (Rockheart
    /// [vis]); twice, all of it.
    PreventHalfStrike,
    /// Each later strike of the opponent this combat is half prevented,
    /// rounded up (Rockheart [VIS]).
    HalveOpposingStrikesThisCombat,
    /// +n damage this combat to the vampire's damaging strikes requiring
    /// this Discipline (Increased Strength: Potence).
    DisciplineStrikeDamageThisCombat { discipline: &'static str, n: u8 },
    /// Immune to frenzy cards for the rest of combat, one played before
    /// staying (Call the Wild Hunt; LSJ 20040210).
    FrenzyImmuneThisCombat,
    /// Discard X cards of this kind from hand, chosen one by one, replaced
    /// once all are made (Call the Wild Hunt [ANI][PRO]: animals; LSJ
    /// 20060215).
    DiscardChosenX(CardFilter),
    /// X environmental damage on the opposing minion in this round's strike
    /// resolution (Call the Wild Hunt [ANI][PRO]).
    EnvironmentalDamageThisRoundX,
    /// n unpreventable damage on the opposing minion in this round's normal
    /// strike resolution, if it comes (Gianna di Canneto; ANK 20211130).
    ResolutionDamageThisRound(u8),
    /// The card being played beneath: its player reveals the top `n` cards
    /// of their library; another copy of it among them cancels it, its cost
    /// paid, its minion takes `damage` unpreventable environmental damage,
    /// the copies revealed burned (Botched Move: not a cancel Dark
    /// Influences reads, LSJ 20081120-2).
    RevealTopForCopy { n: u8, damage: u8 },
    /// Frenzy on the opposing minion: this combat, each strike or damaging
    /// effect against it inflicts +n damage (Dam the Heart's River [qui];
    /// ANK 20181128, RTR 19960221).
    OpposingTakesMoreThisCombat(u8),
    /// Frenzy on the opposing minion: this combat, its strikes cost it n
    /// more blood or life, a strike card in its cost (LSJ 20070829-1),
    /// another strike as chosen (LSJ 20090703), and it may choose not to
    /// strike (Dam the Heart's River [QUI]).
    OpposingStrikeTollThisCombat(u8),
    /// 1 optional maneuver usable in any round of this combat (Ruins of
    /// Villers Abbey, per blood burned).
    CombatManeuver,
    /// The card put on the combatant is burned as the combat ends (Inner
    /// Essence, Molotov Cocktail).
    BurnAtCombatEnd,
    /// With `PutOnActorNow`: the card burned as the action ends (Into Thin
    /// Air's once-this-action burn).
    BurnAtActionEnd,
    /// With the strike, at close range the striker also takes n
    /// environmental aggravated damage (Breath of the Dragon).
    CloseSelfDamage(u8),
    /// Move 1 blood from each ready vampire the user controls to their pool
    /// (Tribute to the Master).
    OwnReadyVampiresBloodToPool,
    /// After this round, this Blood Brother locks and enters combat with the
    /// opposing minion, which cannot play strike cards the first round;
    /// `maneuver`: an optional one that round (Coordinate Attacks).
    CoordinateAttack { maneuver: bool },
    /// Before its combat, each Blood Brother picked moves 1 blood to the
    /// acting vampire or the action is canceled; then +X strength, capacity
    /// and, with `prevent`, X prevention (X the picks; Coagulated Entity).
    CoagulateFromPicks { prevent: bool },
    /// Each Blood Brother picked burns 1 blood or no effect; then +X
    /// intercept, and with `borrow` the picked ones' Disciplines for the
    /// action (X the picks; Gestalt).
    CircleIntercept { borrow: bool },
    /// Gain X+1 blood, X the ready unlocked Blood Brothers of the acting
    /// vampire's circle, any Methuselah's, counted at resolution
    /// (Unwholesome Bond [san]).
    GainPerUnlockedCircleMember,
    /// 1 blood to each uncontrolled Blood Brother of the acting vampire's
    /// circle (Unwholesome Bond [SAN]).
    BloodToUncontrolledCircle,
    /// Move blood (and equipment with `equipment`) among the controller's
    /// Blood Brothers of the host's circle (Jerry, Redistribution;
    /// `Interrupt::CircleBlood`).
    CircleBloodMoves { equipment: bool },
    /// +n stealth if the user controls the target card or minion, read
    /// with the action's stealth (Rampage, Eurayle Gelasia Mylonas).
    StealthIfOwnTarget(u8),
    /// The vampire cannot play cards requiring Disciplines until the action
    /// is complete (Concoction of Vitality).
    NoDisciplineCardsThisAction,
    /// Its player gains no Edge this action: one it would get is burned
    /// (Leverage).
    NoEdgeThisAction,
    /// Dominate and Presence reactions do not affect this vampire this
    /// action; `superior`: its combat opponents cannot play such cards
    /// (Perfect Clarity).
    PerfectClarity { superior: bool },
    /// The chosen vampires enter combat, the prey's first (LSJ 20070319);
    /// the only one ready after it gains n blood (Taunt the Caged Beast
    /// [ANI]).
    TauntCombat(u8),
    /// Each Methuselah reveals their crypt's top card, a vampire's cost its
    /// capacity (Petra Resonance).
    PetraResonance,
    /// During its controller's unlock phase, each anarch (every
    /// Methuselah's) burns 1 blood or becomes Camarilla, its controller's
    /// choice; each ally burns 1 life (Waiting Game).
    AnarchsBloodOrCamarillaAlliesBurn,
    /// Major Boon played: its player burns pool for this bleed instead of
    /// the target, the card remembering the target (`CardInPlay::against`).
    TakeBleedForTarget,
    /// Major Boon burned: the Methuselah it remembers burns pool for this
    /// bleed instead of its controller.
    BleedToRemembered,
    /// The card it cancels cannot be played again this turn (Dark
    /// Influences).
    BarCanceledThisTurn,
    /// The minion that played the card it cancels cannot play that card
    /// again this turn (Determine).
    BarCanceledForActorThisTurn,
    /// A shield read as a cancel is played: the next cancel of another
    /// Methuselah's minion card is canceled instead (Dark Influences;
    /// `Engine::open_as_played`).
    CancelShield,
    /// The host gets +n votes (Loki's Gift [PRO]: burned in a polling step).
    HostVotes(u8),
    /// The target gains the blood the actor just gained, chosen as the card
    /// is played (`Terms::ActorGain`; Slake the Thirst).
    TargetGainsActorGain,
    /// The host may block and play Auspex reactions as if unlocked, this
    /// action (Helena).
    HostAsIfUnlockedForAuspex,
    /// All this card's counters (its blood) to its controller's pool
    /// (Blood Puppy).
    CountersToPool,
    /// One counter (blood) from this card to its controller's pool (Blood
    /// Puppy).
    CounterToPool,
    /// Move the crypt's top card to the uncontrolled region (Effective
    /// Management).
    CryptTopToUncontrolled,
    /// Burn the target's ranged weapons, `except` the named one
    /// (Anachronism).
    BurnRangedWeaponsOnTarget { except: &'static str },
    /// Weapon strike rider: the bearer takes 1 environmental damage (LSJ
    /// 19970801) once each combat, none if the strike never resolves (Zip
    /// Gun; LSJ 20100310).
    BearerDamageOncePerCombat,
    /// Weapon strike rider: burn 1 counter per damage inflicted (prevented
    /// too), the card burned at none (Weighted Walking Stick).
    CountersPerDamage,
    /// Employ the retainer named by the play's terms, from hand, on the
    /// actor: requirements and cost apply, not an action (Pack Alpha [ani]).
    EmployNamed,
    /// The vampire can use this strike at every strike choice of the round,
    /// additional strikes included (Hunger of Marduk; ANK 20220207).
    RoundStrike(StrikeKind),
    /// The vampire can use this strike once each round this combat (Mythic
    /// Form [PRO]).
    StrikeOnceEachRound(StrikeKind),
    /// Can prevent n non-aggravated damage each round this combat (Mythic
    /// Form; LSJ 20040812-2: never aggravated, even as normal).
    PreventNonAggravatedEachRound(u8),
    /// Prevent up to n damage (aggravated included).
    Prevent(u8),
    /// Prevent up to X+n damage.
    PreventXPlus(u8),
    /// Prevent up to n non-aggravated damage.
    PreventNonAggravated(u8),
    /// Prevent up to n aggravated damage (O Encourado).
    PreventAggravated(u8),
    /// Strikes that are not hand strikes cannot be used this round.
    OnlyHandStrikes,
    /// The next round, if any, is at close range (skip determine range).
    NextRoundClose,
    /// Would-end replacement: start a new round instead.
    NewRound,
    /// The actor gains blood equal to what the opposing vampire lost to
    /// damage this round.
    GainBloodLostByOpponent,
    /// A blocked action continues as if unblocked, mandatory and free, as
    /// the after-combat window closes (Torrent [CEL]); not for the listed
    /// kinds (Shadow Body [OBT]).
    ContinueFreeAfterCombat(&'static [ActionKind]),
    /// Strike rider, combat ends: the opponent burns n blood or life after
    /// combat (Oubliette [OBT]): not if dodged (LSJ 19980526) or the combat
    /// continues (RTR 20020501); no damage (LSJ 20080226).
    BurnAfterCombat(u8),
    /// Strike rider, combat ends: a blocked actor may burn `blood` after
    /// combat to continue as if unblocked with +`stealth`; lost if the
    /// combat continues (spec §4.9.2).
    ContinueAfterCombat { blood: u8, stealth: u8 },
    /// Strike rider, combat ends: after combat at close range, n
    /// unpreventable damage on the opponent, unless dodged or the combat
    /// continues (Catatonic Fear [PRE]).
    DamageAfterCombatIfClose(u8),
    /// At close range and not dodged, n unpreventable environmental damage
    /// on the opposing vampire in this round's press step (Necrosis).
    PressDamageIfClose(u8),
    /// After combat at close range, stun the opponent (Kiss of Cathari
    /// [OBF]/[PRE]): not if dodged (LSJ 20070124) or the combat continues
    /// (RTR 20020501).
    StunAfterCombatIfClose,
    /// After combat at close range, not dodged, the striker burns `blood`
    /// to put this card on the opposing minion, locked if `lock`, its
    /// layers then (Serpent's Numbing Kiss [PRE][SER], Morphean Blow [val];
    /// RTR 20041202, ANK 20180420-1).
    PutOnOpponentAfterCombatIfClose { blood: u8, lock: bool },
    /// Set this round's range, skipping determine range, only while no other
    /// effect set it (Sniper Rifle, Michael Harris; PIB 20120214).
    SetRange(crate::combat::Range),
    /// The pending normal damage of the opposing strike is gone, not
    /// prevented (Valerie Diaz).
    ImmuneStrikeNonAggravated,
    /// "… or damaging effect": the pending normal damage of other sources
    /// (environmental, retainers), as one effect (rules-qa 38.3).
    ImmuneEffectNonAggravated,
    /// The host's initial strike this round must be this card's (a weapon's
    /// maneuver, rulebook "Determine Range"; Sniper Rifle's range): one a
    /// round.
    CommitStrike,
    /// Weapon strike rider, after it resolves: 1 optional additional strike
    /// (limited) this round, only with this weapon (AK-47).
    AdditionalStrikeWithSource,
    /// The library's top card in play as a nameless, textless ghoul ally
    /// with this type line, not played (Absimiliard's Army; LSJ 20040531).
    GhoulFromLibraryTop(crate::library::Body),
    /// The acting Independent gets the unique Liaison title (4 votes); a
    /// younger holder or claimant yields at once (Rise of the Nephtali).
    GainLiaison,
    /// 1 blood from the target, the user's vampire, to another Methuselah's
    /// vampire, their choice (Blood Trade).
    GiveBloodToOthers,
    /// 1 blood from the target to another ready Anarch of the player's,
    /// chosen next (The Status Perfectus; `Interrupt::GiveBlood`).
    GiveBloodToOwnAnarch,
    /// The chosen location is burned unless its controller burns n pool
    /// (Conquest of Humanity).
    BurnUnlessControllerPays(u8),
    /// The chosen or targeted minion takes n unpreventable environmental
    /// damage, out of combat (The New Inquisition, Retribution).
    DamageTarget(u8),
    /// Unlock rider: each of the active Methuselah's minions locks unless
    /// they discard (not mortal allies, nor vampires of capacity above the
    /// Gehenna events in play; Nightmares upon Nightmares).
    LockUnlessDiscard,
    /// Unlock rider: with 3+ vampires of a clan, burn one; capacity 6+, the
    /// card is ignored for the rest of the game (Recalled to the Founder).
    BurnOfCrowdedClan,
    /// After entering play, burn all Gehenna events and conviction cards,
    /// then torpor every vampire of capacity n or less (The Fourth Cycle).
    FourthCycle(u8),
    /// Pay 1 pool per other event in play, paid down to 0 ousting (The
    /// Uncoiling's burn).
    PayPoolPerOtherEvent,
    /// Weapon strike rider: its damage is unpreventable, kept apart from the
    /// pending damage, out of every prevention's reach (Enhanced Coagulant).
    UnpreventableStrike,
    /// Weapon strike rider, on landing on a vampire: the weapon moves onto
    /// it as its other mode, only that mode's layers and riders (Enhanced
    /// Coagulant); dodged, it stays (ANK 20220923).
    BecomesOnOpponent,
    /// Once each combat, prevent n damage from gun strikes: one point a use,
    /// the rest kept for later in the combat (rules-qa 10.3).
    PreventGunStrikeAllowance(u8),
    /// Prevent up to n damage from sources other than gun strikes.
    PreventNonGun(u8),
    /// Discard n cards at random from the user's hand (a cost; the cards
    /// are replaced).
    DiscardAtRandom(u8),
    /// +n strength that combat, added each round (Kasim Bayar).
    StrengthThisCombat(u8),
    /// Weapon strike rider, had as it is chosen: 1 optional press to
    /// continue, once each combat per copy (Righteous Blade), kept though
    /// dodged or unresolved (rules-qa 10.4).
    ContinuePressEachCombat,
    /// On success, search the library for an equipment the acting vampire
    /// can equip and equip it, as normal (Magic of the Smith; spec §1.14.1),
    /// matching all these (Dark Web Procurer: non-unique, 1 pool or less).
    SearchEquip(&'static [CardFilter]),
    /// `SearchEquip` equipped at no cost (Lambach: Eye of Hazimel); still
    /// an equip action (TOM 19960130).
    SearchEquipFree(&'static [CardFilter]),
    /// On success, search the library (shuffled afterward) or the hand for
    /// an equipment the target can equip and equip it, its cost paid as
    /// normal (Moise Kasavubu).
    SearchEquipForTarget,
    /// Move n cards of the filter from the user's ash heap to the top of
    /// their library, one at a time, to the bottom with `bottom`, fewer
    /// allowed with `up_to`; then, with `cut`, they may cut their library
    /// once (Unity; Academia de Campeões).
    AshToLibraryTop {
        n: u8,
        filter: CardFilter,
        cut: bool,
        bottom: bool,
        up_to: bool,
    },
    /// The card burned by its granted action, the acting minion, if a
    /// vampire, takes n unpreventable environmental damage (The Path of
    /// Bone).
    ActingVampireTakesDamage(u8),
    /// The action equips with the equipment named in an ash heap as it is
    /// announced (`Interrupt::NameAshEquipment`),
    /// 1 pool more from its controller's own, 1 less pool or blood from
    /// another's (Filchware's Pawn Shop; resolved as a found equipment).
    EquipFromAnyAsh,
    /// The acting (or reacting) minion gains blood to its capacity
    /// (Provision of the Silsila).
    ActorBloodToCapacity,
    /// The actor, if acting, unlocks and gains blood to its capacity (Edge
    /// of the World).
    IfActingUnlockAndFill,
    /// The actor's block attempt failing, the acting Methuselah may make
    /// the action fail; successful, the actor may lock after resolution to
    /// enter combat with the acting minion (Familial Bond).
    FailOrFightBond,
    /// The contract the actor's last combat remembers, choosing it, burns
    /// if still in play (Provision of the Silsila).
    BurnRememberedContract,
    /// The chosen vampire of the terms is the card's chosen (Hand
    /// Contract).
    ChooseTermsMinion,
    /// 1 counter on this card if the vampire entering is of these clans
    /// (Alvusia's influence-phase rider).
    AddCounterForEnteringClan(&'static [&'static str]),
    /// The card remembers the target Methuselah and their vampire's votes
    /// as it left, as counters and as a fixed X (Leadership Vacuum; ANK
    /// 20221024: counters changed later do not change X).
    RememberTargetSeatsVotes,
    /// Unlock-phase rider: the active Methuselah burns the card's fixed X
    /// pool (Leadership Vacuum).
    ActiveBurnsPoolFixedX,
    /// Unlock-phase rider: each of the active Methuselah's minions with pox
    /// counters takes n environmental damage (Smallpox Griet; RTR
    /// 19970630).
    PoxDamage(u8),
    /// Every pox counter burned (Smallpox Griet; `Hook::HostLeftReady`).
    BurnAllPox,
    /// Unlock-phase rider: the card burns if the active Methuselah controls
    /// a ready vampire of capacity n or more or titled (Leadership
    /// Vacuum), before its pool is burned.
    BurnSelfIfActiveControlsReadyCapacityOrTitled(u8),
    /// The target minion becomes Red List, for good (Public Enemy).
    TargetBecomesRedList,
    /// Its user reveals their hand to every other Methuselah (Prophecies of
    /// Gehenna's burn: `GameState::peeks`).
    RevealOwnHandToAll,
    /// On success: the acting vampire gains 1 blood and searches the
    /// library for a master archetype it can take, put on it at its cost
    /// plus 1 pool per copy on its controller's minions (Guide and Mentor;
    /// PIB 20110914: the blood even with none found).
    SearchArchetypeOntoActor,
    /// On success: the acting vampire may take a Discipline master of these
    /// names from its controller's library, shuffled after (Lord Ashton;
    /// `Interrupt::DisciplineSearch`).
    SearchDisciplineMasterOntoActor(&'static [&'static str]),
    /// The target vampire in torpor moves onto the card put in play, face up
    /// and out of play, the cards on it burned, its blood gone (Raw
    /// Recruit; LSJ 20100302-2); no longer in torpor: no effect.
    HoldTargetVampire,
    /// On success: the vampire held on the card is removed from the game
    /// and the card becomes a Gargoyle of its capacity, with Fortitude,
    /// Potence, Visceratika and flight, enslaved to the acting vampire's
    /// clan, of its player's sect (Raw Recruit; not unique: LSJ 20100221).
    HeldVampireBecomesGargoyle,
    /// On success: the master Discipline card named with the announcement,
    /// from hand or ash heap, enters play as a fledgling of the acting
    /// vampire's clan and sect, tethered to it, and the rest of the hand is
    /// discarded and replaced (Legion).
    FledglingFromNamedMaster,
    /// The target Methuselah burns the top card of their crypt (Yong-Sun).
    BurnTargetCryptTop,
    /// Every ready vampire of this clan its player controls locks
    /// (Ensemble).
    LockOwnReadyOfClan(&'static str),
    /// The acting vampire changes its clan to this one, for good (Barbaro
    /// Lucchese).
    ActorBecomesClan(&'static str),
    /// 1 taint counter on the target minion; at 2, they burn and it becomes
    /// infernal (Barbaro Lucchese).
    TaintTarget,
    /// On success, search the library for a card of this kind into the hand,
    /// or find none (Sakhar's equipment, Dominica's master archetype);
    /// `shown`: the card found revealed to all (the card says so; Dan
    /// Murdock's does not).
    SearchToHand { filter: CardFilter, shown: bool },
    /// The host may unlock, once each turn (Baba Yaga; `Interrupt::MayUnlock`,
    /// `state::once_unlock_key`).
    MayUnlockOncePerTurn,
    /// "Once each turn, after a vampire in combat with it goes to torpor,
    /// you can get +n hand size until the end of the turn" (Sascha Vykos):
    /// a `CombatOpponentLeftReady` rider, the subject a vampire in torpor.
    MayHandSizeOncePerTurn(u8),
    /// "When it enters play, you may pay n pool to make it <title>", one of
    /// these (Victor Pelletier): an entry rider, before any contest.
    BuyTitleOnEntry(&'static [(u8, crate::crypt::Title)]),
    /// "As it enters play, you can search your crypt for a <clan> … and
    /// move them to your uncontrolled region" (Verbruch): an entry rider,
    /// before any contest (`CryptClanToUncontrolled`'s search).
    CryptSearchOnEntry(&'static str),
    /// As the host plays a card requiring `disc`: the library's top card
    /// revealed; requiring it too, the play is canceled and the host burns
    /// `burn` blood (Andrew Stuart).
    RevealTopCancels { disc: &'static str, burn: u8 },
    /// Whenever its vampire is not a ready one of this clan and sect, or an
    /// older ready untitled one is, the card moves to the oldest ready
    /// untitled one, if any (Osebo Kholo; `Engine::settle_moving_titles`).
    MovesToOldestUntitled {
        clan: &'static str,
        sect: crate::crypt::Sect,
    },
    /// On success, a card costing X or less, or (`vampire`) a vampire of
    /// capacity X or less, searched for and put in play for the acting
    /// minion (Summon History; `Interrupt::PutInPlaySearch`).
    SearchPutInPlayUpToX { vampire: bool },
    /// The target vampire gets +1 vote for each counter on this card
    /// (Powerbase: Madrid).
    TargetVotesPerCounter,
    /// A granted action's effect: burn all the counters from the card
    /// granting it (Powerbase: Madrid).
    RemoveAllSourceCounters,
    /// The first referendum a Sabbat vampire of the player's calls this turn
    /// passes automatically (Día de los Muertos).
    AutoPassFirstSabbatReferendum,
    /// Minions burn n blood or life to attempt to block this action
    /// (Unthinkable Humiliation).
    BlockToll(u8),
    /// A marker the diablerie reads: a Sabbat diablerist takes this card,
    /// before the blood hunt (Regent).
    MovesToSabbatDiablerist,
    /// A marker the diablerie reads: the diablerist cannot take this
    /// equipment (Soul Gem of Etrius).
    NotTakenByDiablerie,
    /// Add n blood to each uncontrolled vampire of the printed sect
    /// (Magnetic Authority [DOM]/[POT]).
    BloodToEachUncontrolledOfSect(Sect, u8),
    /// +(mul × X + plus) votes to the vampire playing it, X its blood cost
    /// (Awe).
    AddVotesXLinear { mul: u8, plus: u8 },
    /// Each non-Camarilla vampire gets -n votes in this referendum, votes
    /// already cast included; a vampire's votes stop at 0 (Camarilla
    /// Conclave; ANK 20251125).
    NonCamarillaVotesMinus(u8),
    /// Each vampire not following the path gets -n votes in this
    /// referendum, as `NonCamarillaVotesMinus` (Absolute Tyranny).
    NotPathVotesMinus(&'static str, u8),
    /// Only vampires of the sect cast votes or ballots in this referendum
    /// (Private Audience).
    BarNonSectVotes(Sect),
    /// A round where Methuselahs burn pool for votes, one at a time: `per`
    /// votes a pool, `own` for the card's player (Business Pressure).
    PoolForVotesRound { per: u8, own: u8 },
    /// The card is removed from the game once played, whatever happens to
    /// its action or referendum, canceled too (Reinforcements).
    RemovedOncePlayed,
    /// The host takes any action it may, its Methuselah's pick (Enkil Cog;
    /// `Flow::CogAction`).
    TakeAnyAction,
    /// The printed action is mandatory while its requirements hold: first,
    /// and a vampire unable to take it takes none (spec §3.9, Elen Kamjian).
    MandatoryAction,
    /// On success, the acting minion enters combat with the target, the
    /// combat part of the action (spec §4.1.2).
    EnterCombatWithTarget,
    /// On success, the target card moves to the acting minion: a steal, no
    /// requirement checked (spec §3.7.3.2).
    StealTargetCard,
    /// On success, the target vampire yields its contested title, its
    /// rivals keeping theirs (Ephor).
    TargetYieldsTitle,
    /// Lock the target minion if unlocked, else unlock it (Lord Tremere).
    ToggleLockTarget,
    /// Unleash Hell's Fury in play: a token 9-capacity infernal vampire, +2
    /// intercept, 0 blood, that only attempts to block actions at its
    /// controller.
    FuryEnters,
    /// The card just put on the acting vampire takes counters equal to its
    /// capacity (Call the Great Beast).
    CountersFromActorCapacity,
    /// At 11 counters, Call the Great Beast rises off its Baali, which
    /// burns, into the ready region as the Great Beast.
    GreatBeastAtEleven,
    /// The controller draws the crypt's top card: a younger vampire enters
    /// ready with the Gem and blood to capacity, else uncontrolled and the
    /// Gem burns (Soul Gem of Etrius).
    SoulGemDraw,
    /// On success, a copy of the card named at declaration moves from its
    /// Methuselah's ash heap to their hand (The Sargon Fragment).
    AshCardToHandNamed,
    /// Unlock rider: the active Methuselah burns `pool` unless they burn a
    /// vampire of capacity `cap`+ to burn this card (Antediluvian
    /// Awakening; `Interrupt::PoolOrVampire`).
    BurnPoolUnlessBurnVampire { pool: u8, cap: u8 },
    /// Every uncontrolled region back to its crypt (blood to the pool, cards
    /// on them burned), shuffled and as many drawn; a created vampire burned
    /// instead (Kaymakli Nightmares).
    KaymakliNightmares,
    /// The target vampire changes sect to this one (Chameleon: Anarch;
    /// Leila Monroe: Sabbat).
    TargetBecomesSect(Sect),
    /// The acting vampire changes sect to this one (The Red Question:
    /// Anarch).
    ActorBecomesSect(Sect),
    /// Marker on a bleed card: during its action, minion cards that reduce
    /// the bleed or change its target cost +n blood or life (Suspension of
    /// Disbelief [CHI]; `Engine::card_surcharge`).
    BleedChangeCardsCost(u8),
    /// Unlock the minion the terms chose (`PlayCtx::chosen`: Funeral Wake).
    UnlockChosen,
    /// The target vampire is chosen by the ability's vampire this turn
    /// (`GameState::chosen_this_turn`, Lucinde, Alastor G7).
    ChooseTargetThisTurn,
    /// The actor becomes Black Hand, a trait kept through sect changes
    /// (Mateusz Kazimirski; LSJ 20070322).
    ActorBecomesBlackHand,
    /// Move up to n blood from the acting vampire to the new one, decided
    /// as it resolves (Third Tradition: Progeny; PIB 20150418).
    ActingBloodToNewUpTo(u8),
    /// Move n blood from the acting vampire to the new one, what it has
    /// (Brother in Arms).
    ActingBloodToNew(u8),
    /// n blood from the bank to the new vampire (Hatchling).
    BankBloodToNew(u8),
    /// Unlock rider: this card's vampire, ready, burns if its controller
    /// controls more ready vampires created by cards of this name than
    /// ready crypt vampires of the clan (Brother in Arms).
    BurnBodyIfExcessCreated { clan: &'static str },
    /// The acting minion and the terms' picks become this card's chosen,
    /// and unlock (Blessing of the Name).
    ChooseActorAndPicksUnlock,
    /// Unlock rider: the minions this card chose lock (Blessing of the
    /// Name).
    LockChosen,
    /// Put this card on the target card in play (Blood Tempering).
    PutOnTargetCard,
    /// The prey burns `per` pool for each card of this name the player
    /// controls, then those burn (Harmony).
    PreyBurnsPoolPerNamed { name: &'static str, per: u8 },
    /// Pushed as an action card's action is declared: an action-scope layer
    /// holding for the whole action (The Ailing Spirit [obe], Cheat the
    /// Fates [tem]).
    DeclaredActionLayer(Modifier),
    /// Removed from the game with no infernal vampire controlled or after a
    /// successful action (Infernal Servitor; `Engine::settle_servitors`).
    RemovedAfterSuccessOrWithoutInfernal,
    /// The blocking minion burns n blood or life before block resolution,
    /// what it has; an ally's last life burns it (Power of One [obt]).
    BlockerBurnsBloodOrLife(u8),
    /// Vampires burn n blood to attempt to block this action, an ally
    /// attempting for free (Ni Dieu ni Maître [for]).
    VampireBlockToll(u8),
    /// Unlock the blocking minion (Horrific Countenance).
    UnlockBlocker,
    /// No more attempts to block this action (Horrific Countenance).
    Unblockable,
    /// No more action modifiers on this action (Approximation of Loyalty
    /// [PRE]).
    NoMoreModifiers,
    /// The burned vampire the terms chose, if still in the ash heap, removed
    /// from the game for half its capacity in pool (Redeem the Lost Soul).
    RedeemAshVampire,
    /// The ash heap vampire named as played (`PlayCtx::ash`), if still there
    /// burned from play, to the ready region with no blood (Blessed
    /// Resilience).
    AshVampireToReady,
    /// Each vampire of the player's gains the opposing ally's recruit cost
    /// in blood (Orgy of Blood).
    OrgyOfBlood,
    /// A frenzy on the opposing vampire this combat; with `damage`, 1
    /// unpreventable environmental damage each press step (Drawing Out the
    /// Beast).
    DrawOutBeast { damage: bool },
    /// As `AshCardToHandNamed`, the card named matching the filter, from a
    /// printed action or an action card (Drozodny, Gear Up [nec]).
    AshCardOfToHand(CardFilter),
    /// On success, a card of the filter its Methuselah may move from their
    /// ash heap to hand, picked then (Tye Cooper: "if that action is
    /// successful, you can move").
    AshCardToHandOnSuccess(CardFilter),
    /// While its bearer is ready, the equipment cannot be moved or stolen
    /// (Eye of Hazimel; `GameState::pinned`).
    PinnedWhileBearerReady,
    /// The equipment cannot be moved or stolen (Vial of Zapathasura's
    /// Vitae; `GameState::pinned`).
    PinnedAlways,
    /// Burned, the equipment sends its bearer to torpor (Vial of
    /// Zapathasura's Vitae).
    BurnedSendsBearerToTorpor,
    /// Burned, the equipment inflicts n unpreventable environmental damage
    /// on its bearer (Eye of Hazimel).
    BurnedDamagesBearer(u8),
    /// After `PutInPlay`: the target Methuselah takes control of the card
    /// (Curse of Nitocris).
    TargetSeatTakesCard,
    /// Edge rider: the card's controller picks the Methuselah who takes
    /// control of it (Curse of Nitocris; `Interrupt::GiveCardTo`).
    ControllerGivesCard,
    /// The ability's user takes an action in another Methuselah's minion
    /// phase, as they end it (Madness Network's Malkavians; Enkil Cog's
    /// flow, rules-qa 49.6).
    ActorTakesAnyAction,
    /// Marker: any unlocked vampire of the clan may also attempt to block
    /// the action burning this card (Madness Network).
    ClanBlocksItsBurn(&'static str),
    /// After `PutInPlay`: the card becomes a non-unique copy of the acting
    /// vampire (clan, sect, capacity, Disciplines), locked, the actor's
    /// capacity 1 less for good first (Dual Form [PRO]; LSJ 20050116).
    DualFormCopy,
    /// After `PutOnActor`: the actor, its other cards and counters out of
    /// play, temporary control broken, this card staying (Descent into
    /// Darkness).
    DescendActor,
    /// The host back in play from out of play with its cards (unlocked),
    /// itself `unlocked` or locked, this card burned; with n blood first
    /// (Descent into Darkness).
    ReturnHost { unlocked: bool },
    /// The out-of-play host gains n blood (Descent into Darkness).
    OutOfPlayHostGainsBlood(u8),
    /// n pact counters on the acting minion; reaching its capacity or cost,
    /// they burn and it becomes infernal (Diabolic Lure).
    PactCountersOnActing(u8),
    /// With `PutOnTarget`: the target is considered the acting vampire's
    /// clan and sect while the card is on it (Derange).
    DerangeTarget,
    /// Take control of the target minion until the controller's next
    /// unlock phase begins (Malkavian Dementia; `takes_control`).
    TakeControlOfTargetUntilUnlock,
    /// Take control of the target minion until the end of the turn (The
    /// Ailing Spirit [OBE]; `takes_control`).
    TakeControlOfTargetUntilEndOfTurn,
    /// The actor may bleed once more this turn past the cap (Taking the
    /// Skin: Minion; rules-qa 1.4).
    MayBleedAgainThisTurn,
    /// The ally is immune to non-aggravated damage: none inflicted,
    /// unpreventable or outside combat too (Tye Cooper; LSJ 20010627).
    ImmuneNonAggravated,
    /// Gain n pool after resolution if successful, a fizzled action
    /// successful (I am Legion [DAI][OBF]).
    PoolIfSuccessful(u8),
    /// After resolution, its player draws 1 card if the action's referendum
    /// passed (Deadly Secrets [OBF]).
    DrawIfReferendumPasses,
    /// The other cards so labeled on the target minion burn, once this one
    /// is put on it (Condemnation: Mute).
    BurnOthersLabeledOnTarget(crate::library::Label),
    /// The block attempt fails, and the blocker's Methuselah cannot attempt
    /// to block this action again (Crocodile's Tongue).
    AttemptFailsBarSeat,
    /// The blocking minion can burn n blood or life to cancel this card as
    /// it is played (Crocodile's Tongue).
    CancelableForBlockerBlood(u8),
    /// ±n hand size until the next unlock phase (Juliet Parr, Luc, Thucimia).
    HandSizeUntilUnlock(i8),
    /// The actor gets the bonus until the expiry, its controller's unlock
    /// phase for `UnlockPhase` (Dive into Madness [dem], Gear Up).
    ActorLasting(Lasting, crate::state::Expiry),
    /// As `ActorLasting`, on the target minion (In Memory of the Two
    /// Lands: a mummy's strength).
    TargetLasting(Lasting, crate::state::Expiry),
    /// All vampires +n stealth on hunts until the next unlock phase (Sunset
    /// Strip, Hollywood).
    HuntStealthUntilUnlock(u8),
    /// The player draws a crypt card, revealed: n blood from the bank on it
    /// if it is Black Hand (Recruitment Exercise).
    DrawCryptBloodIfBlackHand(u8),
    /// The target vampire becomes Black Hand (Gerald FitzGerald).
    TargetBecomesBlackHand,
    /// A Laibon draws a crypt card; else, of these clans, becomes Laibon and
    /// unlocks (Mozambique Allure).
    LaibonOrCryptDraw(&'static [&'static str]),
    /// The target vampire gets a disease counter, one at most (Vampiric
    /// Disease).
    DiseaseTarget,
    /// On success, the uncontrolled vampire named at declaration is removed
    /// from the game and the target gets n blood; either gone, nothing
    /// (Malgorzata).
    RemoveUncontrolledForBlood(u8),
    /// On success, the card granting the action moves to the target
    /// vampire (Society of Leopold).
    MoveSourceToTarget,
    /// On success, the card granting the action moves to the acting
    /// vampire (Houngan).
    MoveSourceToActor,
    /// The target card's own "do not replace until" ends: its controller
    /// replaces it now if it waits still (Black Hand Ritual).
    EndTargetCardHold,
    /// On success, the target card is burned.
    BurnTargetCard,
    /// Steal up to n blood from the target, the amount chosen at resolution
    /// (François Villon G2; rules-qa 62.4; `Interrupt::StealUpTo`).
    StealBloodUpTo(u8),
    /// On success, move up to n blood from the target to the acting vampire
    /// (rulebook "Target"), the excess over capacity draining off.
    StealBloodFromTarget(u8),
    /// The vampire does not unlock as normal during its next unlock phase.
    SkipNextUnlock,
    /// Exchange a hand card for a library card in the ash heap, both named
    /// as announced (Lenelle; LSJ 20000222), on success if both still there.
    ExchangeHandForAsh,
    /// On success, equip the first equipment found from the top, of
    /// `label` if any (Vast Wealth; Repo Man: a vehicle), on the actor or,
    /// `on_any_own`, on any ready minion of its controller's meeting it
    /// (`MinionPurpose::EquipFoundOn`): one that cannot be placed or paid
    /// in blood burned free (TOM 19950407); a pool cost that ousts paid
    /// (RTR 19941109).
    EquipFirstFound {
        label: Option<crate::library::Label>,
        on_any_own: bool,
    },
    /// On success, the card at this index on this card equips the actor as
    /// played, requirements and cost as normal; unmet, it stays (Delivery
    /// Truck, Aaron Fletcher).
    EquipStored(u8),
    /// Each Methuselah from its player, clockwise, rearranges the
    /// equipment on their ready minions, all chosen then moved (Nod; ANK
    /// 20190617-1).
    RearrangeEquipment,
    /// The recruiting minion unlocks at the end of the turn if the recruit
    /// succeeds (Repo Man).
    RecruiterUnlocksAtEndOfTurn,
    /// Move the top n library cards onto this card, out of play, as many as
    /// there are (Shilmulo Tarot; rules-qa 11.2).
    StoreTopCards(u8),
    /// Once replaced (rules-qa 12.4), search up to n non-unique equipment
    /// onto this card, then contest (spec §1.13.2); none, burned (Black
    /// Market Cache; rules-qa 11.3). Applied by `resolve_play`.
    SearchStore(u8),
    /// Draw-alternative rider: burn this card as its last card is drawn
    /// (Black Market Cache).
    BurnSelfWithoutStored,
    /// Draw up to X without discarding, then bottom as many from the hand, X
    /// capped by the library (Heart of Nizchetus; rules-qa 9.5, 11.1).
    DrawThenBottom,
    // Referendum effects (`referendum`).
    /// The political action card's referendum (with `DeclareAction`).
    CallReferendum(&'static ReferendumDef),
    /// The actor gets +n votes in this referendum, on the side of its votes
    /// already cast.
    AddVotes(u8),
    /// Each vampire of the user's the grant names gets its votes in this
    /// referendum (Elysium: The Palace of Versailles, Power Structure).
    GrantVotes(VoteGrant),
    /// As `GrantVotes`, to every Methuselah's vampires the grant names (The
    /// Eldest Are Kholo: no "you control", rules-qa 69.2).
    GrantVotesAll(VoteGrant),
    /// The next referendum a vampire of the user's calls this turn passes
    /// automatically (Cryptic Rider).
    NextReferendumPasses,
    /// Unlock, search for the base version and merge with it (Legacy;
    /// `Interrupt::BaseSearch`).
    MergeWithBase,
    /// Lock the target vampire and merge it from its controller's crypt
    /// only: with its advanced version, or with its base if advanced (Gift
    /// of Experience).
    LockTargetMergeFromCrypt,
    /// Pool lost to the passed referendum is lost by the acting vampire's
    /// controller too (Poison Pill).
    PoisonPill,
    /// The user's own votes, cast at once on one side, a vote source of
    /// their own like the Edge (Ventrue Headquarters).
    MethuselahVotes { n: u8, favor: bool },
    /// As `AddVotes`, +1 per ready vampire of this sect the player controls,
    /// counted as it resolves (Aisha az-Zahra).
    AddVotesPerReadySect(Sect),
    /// The target vampire abstains: its cast votes and ballots are
    /// canceled and it casts no more; `lock` locks it, `burn` burns blood.
    Abstain { lock: bool, burn: u8 },
    /// The actor gets +n votes against the referendum (and votes against).
    AddVotesAgainst(u8),
    /// Methuselahs casting votes or ballots against burn n pool once the
    /// results are tallied (Scorn of Adonis; once per Scorn per Methuselah).
    TallyPenaltyAgainst(u8),
    /// Every other Methuselah casting votes or ballots in favor and none
    /// against gains n pool once the results are tallied (Bribes; one per
    /// Bribes per Methuselah).
    TallyBonusForOthers(u8),
    /// Around the table from the player, each Methuselah may burn blood one
    /// at a time from a ready vampire of capacity above `min` for `votes`
    /// each, `extra` more above `more`, until all pass (Mob Rule;
    /// `Interrupt::BloodForVotes`).
    BloodForVotesRound {
        min: u8,
        more: u8,
        votes: u8,
        extra: u8,
    },
    /// The card's effect stays in play on the actor for this referendum,
    /// its abilities the grant, its chosen vampire the target if any;
    /// burned as the referendum ends (Rant!, Emissary).
    GrantForReferendum,
    /// The actor takes n unpreventable damage if this referendum fails
    /// (Rant!).
    ActorDamagedIfReferendumFails(u8),
    /// The target gets n votes in this referendum (Emissary).
    TargetGainsVotes(u8),
    /// The card's chosen vampire's votes and ballots in this referendum
    /// canceled, none more cast (Emissary's older Camarilla vampire).
    CancelChosenVotes,
    /// The control change about to happen to the target does not
    /// (`crate::control`).
    StopControlChange,
    /// Put the X (+ `more`) younger vampires named as played
    /// (`Terms::CommandPicks`) in play, `blood` each from the bank over
    /// their own, no unique one already in play (two copies neither, LSJ
    /// 20040722); they must bleed, then go to the bottom of the crypt
    /// (Chain of Command; `crate::state::Command`).
    CommandUncontrolled { more: u8, blood: u8 },
    /// The vampire a passed blood hunt is about to burn is not burned
    /// (`crate::control`).
    SaveFromHuntBurn,
    /// If this referendum passes, the card goes from the ash heap onto the
    /// calling vampire with n counters (Aura of Invincibility).
    PutOnCallingIfPasses(u8),
    /// Non-Camarilla vampires cannot cast votes or ballots (Closed Session).
    BarNonCamarillaVotes,
    /// Minions with any corruption counters cannot cast votes or ballots in
    /// this referendum, read live (Velvet Tongue [SER]).
    BarCorruptedVotes,
    /// Put 1 of the player's corruption counters on the target minion, no
    /// steal offered (Velvet Tongue [ser]; ANK 20200925).
    CorruptTarget,
    /// As `CorruptTarget`, a Shared effect for a play or an ability (O
    /// Grande Arquiteto); `CorruptTarget` is the referendum's (Vote).
    CorruptTargetMinion,
    /// Cancel the referendum: no tally, the card back to its owner's hand.
    /// `unlock_acting`: unlock the actor; `forbid_repeat`: no same political
    /// action by the acting Methuselah's minions this turn.
    CancelReferendum {
        unlock_acting: bool,
        forbid_repeat: bool,
    },
    /// After a passed referendum: the actor gains 1 blood per vote of the
    /// margin, X of them to the player's pool instead (Voter Captivation).
    GainBloodPerMargin,
    /// Distribute the margin's blood among the ready vampires of the clan (1
    /// each) and the pool (capped), chosen as it resolves, the pool first;
    /// what finds no room is lost (Amici Noctis).
    DistributeMargin {
        clan: &'static str,
        pool: u8,
        titled_pool: u8,
    },
    /// Reduce the current bleed by n (never below 0).
    ReduceBleed(u8),
    /// On another Methuselah's minion, the card stays its player's (Puppet
    /// Master [DOM]; read by `GameState::card_controller`).
    StillYours,
    /// Unlock the host and give the card's controller control of it until
    /// the end of their turn (Puppet Master [DOM]).
    TakeControlOfHost,
    /// This mode is a frenzy card (Deep Song), a marker read by Meditative
    /// Grove, Tranquility Shield.
    Frenzy,
    /// With `EnterCombatWithTarget`: the target locks as combat begins and
    /// is the acting minion in it (Deep Song [ANI]).
    TargetActsInCombat,
    /// What the acting minion gets in the combat the card action's
    /// resolution begins (Fleetness [CEL]).
    InResultingCombat(BlockRider),
    /// Look at the target Methuselah's hand and discard one; they replace it
    /// unseen (Revelations [aus]; LSJ 20100119).
    DiscardFromTargetHand,
    /// Steal n pool from the target Methuselah (Line Brawl [cel]).
    StealPoolFromTarget(u8),
    /// The controller of the target card burns n pool (Rewilding).
    TargetControllerBurnsPool(u8),
    /// The acting minion's controller burns n pool (Burnt Offerings
    /// [AUS], Diabolic Lure [DAI]).
    ActingControllerBurnsPool(u8),
    /// The opposing combatant's controller burns n pool (Hell-for-Leather
    /// [obt]).
    OpposingControllerBurnsPool(u8),
    /// The target Methuselah discards n cards of their choice (Shroud of
    /// Decay [obl]).
    TargetMethuselahDiscards(u8),
    /// Remove `remove` cards of the prey's ash heap, chosen as it resolves
    /// (LSJ 20090710), to burn `pool` of their pool (Shroud of Decay [OBL]).
    RemovePreyAshToBurnPool { remove: u8, pool: u8 },
    /// Cost: remove n cards of the user's ash heap, picked one at a time on
    /// use (Marchesa Liliana; `Interrupt::RemoveFromAsh`).
    RemoveOwnAshCards(u8),
    /// The action's card goes to the player's library, shuffled, instead of
    /// their ash heap as the action ends (Epikasta Rigatos: "from your ash
    /// heap … after resolution").
    ActionCardToLibraryShuffled,
    /// Vampires of this sect cast no votes or ballots in the referendum the
    /// political action calls (Bruce de Guy).
    BarSectVotesInReferendum(crate::crypt::Sect),
    /// The ability's count X of cards moved from hand to the bottom of the
    /// library, one at a time (Nehemiah; `Interrupt::Bottom`).
    HandToBottomX,
    /// The top card of the player's crypt removed from the game (Paul
    /// Cordwood's cost).
    RemoveCryptTop,
    /// The action's card, if any, back to its player's hand as the action
    /// ends, discarded down after (Tangle Atropos' Hand [tem]).
    ActionCardToHand,
    /// The acting vampire's next action this turn gets +n stealth (Tangle
    /// Atropos' Hand [TEM]; `Minion::next_action_stealth`).
    StealthOnNextAction(u8),
    /// The host vampire gets superior in the Discipline chosen for the
    /// card until its controller's next master phase, even if the card goes
    /// (Camarilla Vitae Slave; LSJ 19970224, rules-qa 76.24).
    HostSuperiorChosenUntilMaster,
    /// Unlock the host of this card in play, not under "cannot unlock"
    /// (Rutor's Hand).
    UnlockHost,
    /// Lock the card this play put in play (Rutor's Hand).
    LockPlayedCard,
    /// The acting minion takes `damage` unpreventable environmental
    /// aggravated damage; with `avoid_for` > 0 it can burn that much blood
    /// to be immune instead (Rutor's Hand [THA]).
    ActorTakesAggravated { damage: u8, avoid_for: u8 },
    /// Prevent all the damage pending from the opposing strike on the
    /// player's minion (Touch of Valeren [COMBAT]).
    PreventStrikeDamage,
    /// Prevent up to `n` of the pending damage from the opposing strike,
    /// aggravated too when `aggravated`, never other damage (Lodin, Rego
    /// Motum; Opikun's non-aggravated). A strike's damage is of one kind, so
    /// no split is asked (rules-qa 4.2).
    PreventStrike { n: u8, aggravated: bool },
    /// This printed action is ability n's on another kind of target: one
    /// action for NRA (Aniel).
    SameActionAs(u8),
    /// The target minion loses 1 corruption counter, whose chosen as the
    /// action resolves (Aniel).
    BurnCorruptionFromTarget,
    /// Once this action, burn 1 corruption counter from a blocker to fail
    /// its attempt and bar it (Enchanting Gaze [PRE][PRO];
    /// `Choice::BurnCorruptionToFail`).
    FailBlockForCorruption,
    /// On a successful bleed, burn n corruption counters from a target's
    /// minion to unlock (Revelation of the Serpent [PRE][PRO];
    /// `Choice::BurnCorruptionToUnlock`).
    UnlockForCorruption(u8),
    /// A bleed action card's effects if the bleed is successful (for 1 or
    /// more), applied after the pool is burned.
    IfBleedSuccessful(&'static [Effect]),
    /// Lock a minion of the target Methuselah's, chosen as it resolves
    /// (Break the Bonds [obf]; ANK 20240706).
    LockMinionOfTarget,
    /// 1 corruption counter on a minion of the target Methuselah's, chosen
    /// as it resolves (The Platinum Protocol [pro]; ANK 20240706).
    CorruptMinionOfTarget,
    /// Put this card on the minion playing it (Heart of the City).
    PutOnActor,
    /// As `PutOnActor`, remembering the Methuselahs the action is directed
    /// at (`CardInPlay::against`; Spying Mission [OBF], Shadow Cast [OBL]).
    PutOnActorRemembering,
    /// The active Methuselah chooses a ready vampire they control, which
    /// takes n unpreventable damage (Becoming of Ennoia: each Methuselah's
    /// unlock phase).
    ActiveChoosesVampireDamaged(u8),
    /// Compare the capacities of the tops of the actor's controller's and
    /// the blocker's controller's crypts, shuffle both: the actor's
    /// younger, it takes 1 unpreventable damage; else the block's combat is
    /// canceled and this card goes on the actor, remembering the blocker's
    /// controller (`CardInPlay::against`; Bear-Baiting).
    CompareCryptTopsWithBlocker { damage: u8 },
    /// What the actor gets in the combat its block (or its action's block)
    /// begins, for each such block this action (LSJ 20010813, 20010814-2).
    IfBlocks(BlockRider),
    /// As `IfBlocks`, for the minion attempting to block, from a card that
    /// is not its own (Destilaria Estrela da Paraíba).
    IfAttemptingBlocks(BlockRider),
    /// A block made, the rider for both combatants of its combat (Bar
    /// Último Trago).
    BlockRiderBoth(BlockRider),
    /// With a card-effect strike: landing, it stuns the opposing minion
    /// (Father Anthony).
    StunOpponentOnHit,
    /// As the round ends, at close range, its vampire having inflicted more
    /// damage this round, the card goes on the opposing minion (Chave de
    /// Braço [POT]).
    PutOnOpponentAtRoundEndIfMore,
    /// The actor burns n blood as an effect, not a cost, in an action, a
    /// combat or neither (Second Tradition: Domain, Rolling with the Punches
    /// [FOR]; ANK 20210226); an ability's user, not its host (Feo Ramos).
    BurnActorBlood(u8),
    /// The target minion unlocks, attempts to block with +n intercept for
    /// the rest of the action (Organized Resistance).
    TargetAttemptsBlock(u8),
    /// This action, the minion can burn 1 blood for +1 intercept, once (Eyes
    /// of the Wild; `Choice::BurnBloodForIntercept`; rules-qa 79.1).
    BurnForInterceptThisAction,
    /// Lock the minion after resolution if it did not block (Eyes of the
    /// Wild).
    LockIfNoBlock,
    /// A fixed -X stealth, X the stealth as played (RTR 20030519), nothing
    /// at 0 or less (LSJ 20021211); the actor can still raise it.
    ReduceStealthToZero,
    /// The current block attempt fails; that minion cannot attempt to
    /// block this action again (Forced Confessional [DOM]).
    AttemptFails,
    /// This action, minions without the Discipline burn 1 blood to attempt
    /// to block, allies cannot (Where the Veil Thins [OBL], Tenebrous Form
    /// [OBT]): each copy its own (rules-qa 69.12); pay or resign (LSJ
    /// 20031121-2).
    VeilThins(&'static str),
    /// Minions who attempt to block this action and fail become locked
    /// before action resolution (Faceless Night [OBF]).
    LockFailedBlockers,
    /// The canceled master's Methuselah gets +1 master phase action, at
    /// their next master phase for an out-of-turn master (Wash); not a
    /// trifle's bonus (ANK 20170124).
    CanceledGetsMasterAction,
    /// The blocking minion's controller can burn n pool to cancel this card
    /// as played (True Love's Face [OBF][PRE]; `Choice::PayToCancel`).
    CancelableForPool(u8),
    /// Any Methuselah can burn n pool to cancel this card as it is played,
    /// its cost then not paid (Personal Involvement).
    CancelableForPoolByAny(u8),
    /// The player's prey burns n pool if richer than the player after the
    /// card's cost (Personal Involvement).
    PreyBurnsPoolIfRicher(u8),
    /// The actor attempts to block the action, keeping an attempt already
    /// under way (ANK 20181122-2).
    AttemptBlock,
    /// The actor, if an ally, does not unlock as normal during its next
    /// unlock phase (On the Qui Vive).
    AllySkipsNextUnlock,
    /// The target minion wakes for the rest of the action (Shadow
    /// Sentinel [OBL]).
    WakeTarget,
    /// +X bleed for the current bleed, X the play's (Monkey Wrench);
    /// `limited` binds the rulebook cap.
    BleedModX { limited: bool },
    /// `then` (a bleed or intercept bonus this action) counts while
    /// `requires` holds of `on`, read live (Aire of Elation, Protection
    /// Racket; ANK 20190416).
    If {
        on: On,
        requires: &'static [Requirement],
        then: &'static Effect,
    },
    /// Allies and vampires younger than the actor get this intercept
    /// modifier for the rest of the action (Perfect Paragon [PRE]).
    YoungerIntercept(i8),
    /// n bank blood on each of the player's uncontrolled vampires meeting
    /// the filter, read against the acting vampire (Little Mountain
    /// Cemetery: younger Samedi).
    BloodToOwnUncontrolled {
        filter: &'static [Requirement],
        n: u8,
    },
    /// No Edge: the player gets it; with it, they may burn it to gain n
    /// pool (Off Kilter; `Interrupt::MayBurnEdgeForPool`).
    GainEdgeOrBurnItForPool(u8),
    /// The minion the target card is on locks (Threading the Path of
    /// Orpheus), before the card burns.
    LockTargetCardHost,
    /// In this diablerie's blood hunt only vampires of these clans cast
    /// votes or ballots in favor (Warding the Beast [VAL]).
    BloodHuntFavorOnlyClans(&'static [&'static str]),
    /// The target and the second pick (`Terms::OptionalSecondTarget`) still
    /// in torpor leave it, no blood paid (Lord of Serenity [for]; ANK
    /// 20181017).
    TargetsLeaveTorpor,
    /// As `AshCardOfToHand`, the card named at declaration to the top of
    /// the library (Scarlet Lore [nec]; LSJ 20010627).
    AshCardOfToLibraryTop(CardFilter),
    /// Search the library for a card of the filter, revealed, the library
    /// shuffled, the card on top (Scarlet Lore [myt]).
    SearchLibraryToTop(CardFilter),
    /// `copies` copies of this card on its host: they burn, and each other
    /// Methuselah with no shattered counter gets one (`Seat::shattered`,
    /// `burn` pool each unlock phase; Shatter the Gate).
    ShatterAtCopies { copies: u8, burn: u8 },
    /// Discard a card of this name from hand as the action resolves, the
    /// action fizzling with none (Shattering Crescendo [mel]; rules-qa 59.1).
    DiscardNamedFromHand(&'static str),
    /// n unpreventable damage on the target and on the second pick
    /// (Shattering Crescendo [mel]).
    DamageTargets(u8),
    /// The ash heap ally named as played (`Terms::AnyAshBurnedAlly`) to the
    /// player's ready region with this card on it, its starting life plus
    /// `extra`, put in play as Split the Veil's (Pressing Flesh; LSJ
    /// 20100204).
    RaiseAshAllyWithThis { extra: u8 },
    /// The card named with the strike moves to the striker as the strike
    /// lands, if still on the opposing minion (Absorb the Mind [MYT]).
    StealTargetCardOnHit,
    /// The strike's damage is reduced to zero, still a hand strike
    /// (Autonomic Mastery [DOM]).
    StrikeDamageZero,
    /// The minion whose card this cancels locks (Evil Eye [MAL]).
    LockCanceledActor,
    /// The opposing minion whose strike card this cancels gets no strike
    /// this exchange (Evil Eye [mal]).
    OpposingGetsNoStrike,
    /// X additional strikes (limited), hand strikes only, now and after
    /// each later round's normal pair (Fractura [str]).
    AdditionalHandStrikesEachRoundX,
    /// Put this card in play; 1 blood or life of the opposing minion moves
    /// onto it in each round's normal strike resolution, half of it rounded
    /// up to the actor as the combat ends (Darkness Within).
    DrainToCardEachRound,
    /// The target card moves to the actor: a move, no requirement nor cost
    /// (Clockwerx).
    TargetCardToActor,
    /// n unpreventable environmental damage to the opposing minion in each
    /// round's normal strike resolution at close range, this combat
    /// (Blissful Agony [val]).
    ResolutionDamageEachRoundIfClose(u8),
    /// The combat ending by this strike undodged, the opposing vampire
    /// enters combat with the target minion, going first (Blissful Agony
    /// [VAL]; `CombatCtx::redirect`).
    OpponentEntersCombatWithTarget,
    /// The striker burned by this strike's damage resolution: its
    /// controller gains n pool (Eye of Unforgiving Heaven).
    PoolIfStrikerBurned(u8),
    /// This strike's damage is unpreventable, aggravated or not (Hexe
    /// [STR]).
    UnpreventableStrikeDamage,
    /// Once this round, the vampire may burn n blood for a press only usable
    /// to continue (Lead Fist [vis]; an effect, ANK 20210226).
    ContinuePressForBloodThisRound(u8),
    /// Until the combat ends, the vampire may burn n blood to unlock (Loving
    /// Agony [VAL]; ANK 20180618, RTR 20020501).
    UnlockForBloodThisCombat(u8),
    /// The retainers on the opposing minion lose their text this combat
    /// (Olid Loa; `GameState::retainers_silenced`).
    OpposingRetainersLoseTextThisCombat,
    /// The opposing minion cannot maneuver to close range nor press to
    /// continue this combat, not a frenzy effect (Olid Loa).
    OpposingNoCloseNorContinue,
    /// As `AggravatedAsNormal`, for the rest of the combat (Penitent
    /// Resilience [for][obe]).
    AggravatedAsNormalThisCombat,
    /// Cards of this name have no effect on the vampire this round, or this
    /// combat (Penitent Resilience: Flash Grenade).
    NamedCardNoEffect { name: &'static str, combat: bool },
    /// Landing, the opposing minion strikes no more this round, its
    /// additional strikes and a strike held by first strike included (Pounce;
    /// LSJ 19970814-2, LSJ 20011214).
    OpposingNoStrikeRestOfRound,
    /// Dodged, the striker takes n environmental damage in this resolution
    /// and the opposing minion gets an optional press this round (Pounce;
    /// LSJ 19970801, TOM 19960521).
    IfDodgedSelfDamageAndOpposingPress(u8),
    /// The target melee weapon's strikes by the actor deal +n damage for the
    /// rest of the combat, aggravated if `aggravated` (Sword of the
    /// Righteous; none on a strike dealing none, RTR 19960221).
    WeaponDamageThisCombat { n: u8, aggravated: bool },
    /// The action fails and the acting minion enters combat with the actor
    /// as the impulse window closes, the action going on until it ends
    /// (Ensconced; LSJ 20070217, 20070214-2; Champion's `fail_into_combat`).
    FailsIntoCombatWithActor,
    /// Lock the minion the actor is about to fight (the blocker, the rush's
    /// target or Psyche!'s opponent), or the acting minion when the actor is
    /// that one; a locked blocker stays locked as the action ends (Gift of
    /// Sleep [obe]).
    LockFoe,
    /// Equipment and conviction cards grant no intercept this action, any
    /// they grant lost (Din of the Damned).
    NoGearIntercept,
    /// Reaction modes and power reactions granting intercept this action
    /// cost n more blood or life (Din of the Damned; `grants_intercept`).
    InterceptReactionToll(u8),
    /// The chosen minion may play this action's action card from its
    /// player's ash heap as if from hand, as that player's next action (Hay
    /// Ride; `Seat::next_from_ash`; LSJ 20081126).
    NextActionFromAsh,
    /// This mode costs n fewer blood, before other reductions (Hay Ride
    /// [SAN]).
    CostsLessBlood(u8),
    /// The player burns n pool, an effect (Psalm of the Damned [pre]).
    BurnOwnPool(u8),
    /// The chosen minion plays no reaction cards nor [REACTION] powers this
    /// action (Psalm of the Damned [mal]; `TargetNoReactions` bars cards only,
    /// advanced rules §1.5.1).
    TargetNoReactionsOrPowers,
    /// The actor unlocks at the end of the action if it blocked it and is
    /// still ready (Strix [str]).
    UnlockAtEndIfBlocks,
    /// The bleed burns no pool, still successful: its Edge and successful
    /// bleed effects stand (Strix [STR]; ANK 20200221).
    BleedBurnsNoPool,
    /// The actor enters combat with the acting minion as the action resolves,
    /// after-resolution effects waiting for the combat (Strix [STR]; ANK
    /// 20200221, advanced rules §3.4.2).
    FightActingAtResolution,
    /// The minion commits its initial strike to this card's ability n, as a
    /// maneuver does (Tinglestripe [MYT]'s 1R strike).
    CommitStrikeOf(u8),
    /// n aggravated damage to the target minion out of combat (Jar of Skin
    /// Eaters).
    AggravatedDamageTarget(u8),
    /// Move n blood from the actor onto this card as counters, as much as it
    /// has (Jar of Skin Eaters' Gargoyle).
    ActorBloodToSourceCard(u8),
    /// The host is removed from the game (Journal of Hrorsh).
    RemoveHostFromGame,
    /// The ally this card is leaves the game once no counter is left on it
    /// (Reanimated Corpse).
    RemoveSelfWithoutCounters,
    /// The ally enters play with X + n counters, X its cost as recruited, 0
    /// put in play otherwise; none, it leaves the game at once (Reanimated
    /// Corpse; LSJ 20100204, LSJ 20071015).
    EntryCountersXPlus(u8),
    /// After its strike resolves, the weapon is out of play until the action
    /// ends, or the combat without one (Dagger).
    OutOfPlayUntilActionEnd,
    /// The weapon burns if any damage of its strike is prevented (Dagger).
    BurnIfDamagePrevented,
    /// Equipped from hand: another copy of it from the hand or ash heap may
    /// be put on the minion too (Dagger; LSJ 20100421).
    MaySecondCopyFromHandOrAsh,
    /// Draw 1 up to n cards, the number picked as it resolves before the
    /// draw (`Interrupt::DrawUpTo`; rules-qa 11.1), then discard down (La
    /// Viuda Blanca).
    DrawUpToThenDiscardDown(u8),
    /// As the vampire enters play, a location its controller controls not
    /// bound to another present vampire of its name is bound to it
    /// (`Interrupt::BindLocation`); none, it is removed from the game (LSJ
    /// 20091216); removed once its controller no longer controls that
    /// location (Grotesque; `Engine::settle_printed_burns`).
    BindLocationOnEntry,
    /// The first aim card of the user's library from the top is played as
    /// if from hand on the chosen strike, the library shuffled (Mikael
    /// Birkholm; LSJ 20100511).
    PlayFirstLibraryAim,
    /// Every boon in play burned, any Methuselah's (Blood Trade).
    BurnAllBoons,
}

/// Trigger points where riders of cards in play fire (mandatory
/// effects in canonical order).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hook {
    /// A vampire of the controller's is blocked, before block resolution,
    /// ordered with the acting minion's "if blocked" riders (Millicent
    /// Smith; ANK 20220116).
    OwnVampireBlocked,
    /// The minion the card chose is blocked, before block resolution: its
    /// player may lock it for 1 blood on its vampire (Curmudgeon).
    ChosenMinionBlocked,
    /// A vampire the controller controls is burned (Virolax Facility).
    OwnVampireBurned,
    /// A referendum fails that the card's bearer cast votes or ballots
    /// against, read at the tally (Traditionalist).
    BearerVotedAgainstFailed,
    /// A vampire's own entry, influenced out, before its contests (Erlik).
    EntersPlay,
    /// Another card of this label enters play (Watchtower: Four Ride
    /// Forth burns).
    LabelEntersPlay(crate::library::Label),
    /// The host plays a card, as it is played, before any cancel (Andrew
    /// Stuart; LSJ 20100728).
    HostPlaysCard,
    /// A vampire of the controller's predator or prey is burned, ready or
    /// in torpor (Mina Grotius).
    NeighborVampireBurned,
    /// "During their unlock phase": ordered by the active Methuselah among
    /// their unlock effects (spec §2.4.4; rules-qa 6.1), its condition read
    /// as it resolves.
    UnlockPhase,
    /// "During their unlock phase", its condition read as the phase ends:
    /// resolved as the active Methuselah passes (Anarch Revolt; spec §5.2.2,
    /// rules-qa 88.5).
    UnlockPhaseEnd,
    /// Any non-Anarch vampire's action, as announced, a later-canceled one
    /// counted (Waiting Game; ANK 20210124).
    NonAnarchVampireActs,
    /// The turn ends (Loup).
    EndOfTurn,
    /// A minion of the clan's successful action, as it ends (Loup).
    ClanActionSuccessful(&'static str),
    /// The host went to torpor, fired where the torpor happened (Guardian
    /// Angel, Fame; rules-qa 6.4).
    HostToTorpor,
    /// The host successfully recruits an ally or employs a retainer, by an
    /// action or by another card (Baba Yaga; PIB 20150105-2).
    HostRecruitsOrEmploys,
    /// An action ends during which the card's controller's corruption
    /// counters were burned (Rise of the Fallen).
    CorruptionBurnedInAction,
    /// "During your discard phase", as the impulse leaves the active
    /// Methuselah at the phase's end (The Coven).
    DiscardPhase,
    /// A ready minion, anyone's, was burned or sent to torpor (Tension in
    /// the Ranks); not one removed with its ousted Methuselah (spec §1.10),
    /// nor one burned from torpor or a contest.
    ReadyMinionBurnedOrTorpor,
    /// The controller would replace a card: the rider's effects instead,
    /// for every replacement, a delayed one included (Visit from the
    /// Capuchin; spec §6.8.3).
    WouldReplace,
    /// The host was blocked, applied where the block resolves, before
    /// combat or any other effect (vehicles; ANK 20220116).
    HostBlocked,
    /// The controller would draw from their library: a draw alternative
    /// per card, while the library holds one (rules-qa 9.4).
    WouldDraw,
    /// An action at this vampire was announced, fired as the as-announced
    /// window closes, before any block (Evan Klein).
    ActionAnnouncedAtTarget,
    /// A minion in combat with this vampire left the ready region, fired
    /// with `ReadyMinionBurnedOrTorpor`, even if both left (Egidia Arrú;
    /// advanced rules §6.4.4).
    CombatOpponentLeftReady,
    /// This vampire left the ready region (burned or to torpor), fired
    /// with `ReadyMinionBurnedOrTorpor` (Bez Dunsirn).
    HostLeftReady,
    /// Its vampire enters combat (Michael Luther).
    HostEntersCombat,
    /// This vampire's political action's referendum was canceled or
    /// failed, as the action resolves (Cedrick Calhoun).
    OwnReferendumFailed,
    /// This vampire's referendum passed, as the action resolves (Juliet
    /// Parr).
    OwnReferendumPassed,
    /// Its bearer vampire is burned, read before the card burns with it
    /// (Soul Gem of Etrius).
    BearerBurned,
    /// Any referendum passed, a blood hunt included; the host read
    /// (Condemnation: Mute [DAI]).
    AnyReferendumPassed,
    /// This vampire hunted successfully: blood was gained (Adisa; spec
    /// §3.7.2.1).
    SuccessfulHunt,
    /// Any vampire hunted successfully, any card in play's (Hunger Moon;
    /// LSJ 20050720-2).
    VampireHuntsSuccessfully,
    /// Another Gehenna event was played (Wormwood, Fueled by Heart's
    /// Blood).
    GehennaPlayed,
    /// The host blocked, before block resolution; conditions read on the
    /// acting minion and the action (Rebel).
    HostBlocks,
    /// The host announces an action (Slaughtering the Herd [DOM]).
    HostAnnouncesAction,
    /// The controller's prey is ousted (Khabar: Glory).
    PreyOusted,
    /// This ally is burned: where its card goes is replaced, still burned
    /// (Saatet-ta; ANK 20200813), not a discard (RTR 20010711).
    SelfBurned,
    /// A minion with a contract choosing a vampire of the controller's is
    /// about to leave the ready region; optional, the card unlocked (The
    /// Black Throne).
    ContractHostLeaves,
    /// Each master phase, ordered among the phase's others, a copy played
    /// in it included (Brujah Debate; `Engine::master_dues`).
    MasterPhaseStart,
    /// A vampire in combat with one of `clan` of the controller's left the
    /// ready region (Dead Pool).
    CombatOpponentOfOwnClanLeft(&'static str),
    /// A minion opposing an Anarch in combat left the ready region; that
    /// Anarch's controller acts, whoever controls this card (Flames of
    /// Insurrection).
    CombatOpponentOfAnarchLeft,
    /// A non-Camarilla vampire of the controller's sent an opposing vampire
    /// to torpor in combat (The Path of Lilith).
    OwnNonCamarillaSendsToTorpor,
    /// A vampire from the controller's uncontrolled region entered play
    /// (Twilight Camp).
    OwnVampireInfluenced,
    /// A combat of another of the controller's vampires ends (Marie-Pierre).
    OwnClanCombatEnded,
    /// A combat of the controller's Anarch ends (Haven Hunting).
    OwnAnarchCombatEnded,
    /// A vampire its controller influences out enters play (Alvusia: of
    /// some clans, `Effect::AddCounterForEnteringClan`).
    OwnVampireEntersInInfluence,
    /// The controller's vampire successfully bled (Alamut).
    OwnBleedSuccessful,
    /// The controller's bleed is over (resolved, blocked or failed), before
    /// its after-resolution window, read on the acting minion (Forward
    /// Momentum).
    OwnBleedEnds,
    /// A zombie ally or retainer of the controller's is burned (Cursed
    /// Abattoir).
    OwnZombieBurned,
    /// A Methuselah who did not hold the Edge gets it, read for them (Form
    /// of Corruption, `RiderScope::Prey`).
    GainsEdgeAnew,
    /// The minion that played this card (`CardInPlay::by`) leaves the
    /// ready region (Slaughtering the Herd [DOM]).
    ByLeavesReady,
    /// An Anarch arrives under the controller's control, or one of theirs
    /// becomes Anarch (Hackerspace).
    OwnAnarchArrives,
    /// The host is about to bleed the remembered Methuselah for 1 or more,
    /// after every modifier and reaction (Spying Mission [OBF]; LSJ
    /// 19980105), each such card at once (ANK 20200609).
    HostAboutToBleedRemembered,
    /// Any political action's referendum passed, read on a ready vampire
    /// after all its effects (Lutz von Hohenzollern).
    PoliticalReferendumPassed,
    /// The controller's minion phase ended, read on their vampires in play,
    /// torpid ones included (Tupdog).
    MinionPhaseEnd,
    /// The host imbued goes to the incapacitated region (Illuminate's burn:
    /// "if this imbued leaves the ready region").
    HostIncapacitated,
    /// The action under way ends, its combats over (Surge's counters).
    ActionEnds,
    /// The controller's prey lost pool, any way, on neither's turn
    /// (Conniver; `Engine::settle_pool_losses`).
    PreyLosesPool,
    /// The bearer's successful action, as it ends, a fizzled one included
    /// (Journal of Hrorsh).
    HostActionSuccessful,
}

/// Whose turn a phase rider fires on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RiderScope {
    /// Every Methuselah's (Anarch Revolt).
    Every,
    /// The controller's only (Creeping Sabotage).
    Controller,
    /// Every other Methuselah's (Smiling Jack).
    Others,
    /// The controller's prey's (The Gate of Acheron).
    Prey,
    /// The controller's predator's and prey's (Arcanum Chapterhouse,
    /// Alexandria).
    PredatorOrPrey,
}

/// A mandatory effect of a card in play, fired at its hook.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rider {
    pub hook: Hook,
    pub scope: RiderScope,
    /// Host predicates read when the rider is collected and again when it
    /// resolves (Fame); an unmet rider is not pending.
    pub requires: &'static [Requirement],
    pub effects: &'static [Effect],
}

impl RiderScope {
    pub fn applies(self, st: &GameState, controller: SeatId, active: SeatId) -> bool {
        match self {
            RiderScope::Every => true,
            RiderScope::Controller => controller == active,
            RiderScope::Others => controller != active,
            RiderScope::Prey => controller != active && active == st.prey(controller),
            RiderScope::PredatorOrPrey => {
                controller != active
                    && (active == st.prey(controller) || active == st.predator(controller))
            }
        }
    }
}

impl Hook {
    /// Whether this hook's rider requirements are read in the action's
    /// context (`holds`), else on the card and host
    /// (`GameState::rider_holds`) or the vampire (`holds_for_host`).
    /// `printed`: a vampire's riders.
    pub fn reads_action(self, printed: bool) -> bool {
        match self {
            Hook::HostBlocks | Hook::HostBlocked | Hook::OwnBleedEnds => !printed,
            Hook::ActionAnnouncedAtTarget | Hook::PoliticalReferendumPassed => printed,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModeDef {
    pub windows: &'static [Window],
    pub actor: Actor,
    pub requires: &'static [Requirement],
    pub target: Option<TargetSpec>,
    pub terms: Option<Terms>,
    /// "Even if stealth is not yet needed": waives `StealthNeeded`.
    pub waives_needed: bool,
    /// "A vampire can play only one X each …" (`Limit`).
    pub limit: Option<Limit>,
    pub effects: &'static [Effect],
}

/// When a "do not replace until" card is replaced: the first moment the
/// condition is met, ahead of any other effect (spec §1.9.2); a canceled
/// card at once (§1.9.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Until {
    /// "Until your next unlock phase": before cards unlock.
    NextUnlockPhase,
    /// "Until your next discard phase": as the phase begins.
    NextDiscardPhase,
    /// "Until after this action", at its end (Elder Intervention).
    EndOfAction,
    /// "Until after combat" (Lam Into): as the combat ends.
    AfterCombat,
    /// "Until a Methuselah is ousted" (Absimiliard's Army).
    MethuselahOusted,
    /// "Until a vampire commits diablerie".
    Diablerie,
    /// "As long as this card is in play": as it leaves play.
    LeavesPlay,
    /// "Until a titled vampire goes to torpor" (The New Inquisition).
    TitledToTorpor,
    /// "Until a vampire successfully hunts" (Restricted Vitae).
    SuccessfulHunt,
    /// "Until a vampire moves from torpor to the ready region" (Torpid
    /// Blood).
    LeavesTorpor,
    /// "Until a vampire becomes a liaison" (Rise of the Nephtali).
    Liaison,
    /// "Until your prey is ousted" (The Rising).
    PreyOusted,
}

impl Until {
    /// The release moment of a card held by this clause; `LeavesPlay`
    /// waits on the card itself (`Seat::held_in_play`).
    pub fn release(self) -> Option<crate::state::Release> {
        use crate::state::Release;
        Some(match self {
            Until::NextUnlockPhase => Release::NextUnlockPhase,
            Until::NextDiscardPhase => Release::NextDiscardPhase,
            Until::EndOfAction => Release::EndOfAction,
            Until::AfterCombat => Release::AfterCombat,
            Until::MethuselahOusted => Release::MethuselahOusted,
            Until::Diablerie => Release::Diablerie,
            Until::TitledToTorpor => Release::TitledToTorpor,
            Until::SuccessfulHunt => Release::SuccessfulHunt,
            Until::LeavesTorpor => Release::LeavesTorpor,
            Until::Liaison => Release::Liaison,
            Until::PreyOusted => Release::PreyOusted,
            Until::LeavesPlay => return None,
        })
    }
}

/// A mode of `actor` in `windows`, no target, terms or limit (spread
/// `..mode_of(..)` under the fields that differ).
pub const fn mode_of(
    windows: &'static [Window],
    actor: Actor,
    requires: &'static [Requirement],
    effects: &'static [Effect],
) -> ModeDef {
    ModeDef {
        windows,
        actor,
        requires,
        target: None,
        terms: None,
        waives_needed: false,
        limit: None,
        effects,
    }
}

/// A target of `kind` passing `filter`.
pub const fn target(kind: TargetKind, filter: &'static [Requirement]) -> Option<TargetSpec> {
    Some(TargetSpec { kind, filter })
}

/// `+n bleed`, limited or not.
pub const fn bleed(n: u8, limited: bool) -> Effect {
    Effect::BleedMod { n, limited }
}

/// A card's defaults, spread as `..CARD` under its name and modes.
pub const CARD: CardDef = CardDef {
    name: "",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[],
    abilities: &[],
    riders: &[],
    cheaper_for: None,
    lines: &[],
    burn_option_torpid: false,
    retake_from_ash: false,
    replace_from_research: false,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CardDef {
    /// Printed name; matched against `library::LIBRARY`.
    pub name: &'static str,
    /// Trifle: refunds a master phase action (rulebook "Trifle").
    pub trifle: bool,
    /// Out-of-turn master card (rulebook "Out-of-turn"; rules-qa 3.3).
    pub out_of_turn: bool,
    /// "Do not replace until …" (Old Friends, Mirror Walk).
    pub delayed_replacement: Option<Until>,
    /// Playable modes (from hand).
    pub modes: &'static [ModeDef],
    /// In-play abilities (the host's, or a granted action's).
    pub abilities: &'static [ModeDef],
    /// Riders of the card in play: mandatory effects fired at a hook.
    pub riders: &'static [Rider],
    /// A clan's blood cost reduction (Mental Maze).
    pub cheaper_for: Option<(&'static str, u8)>,
    /// The printed line (`LIBRARY` mode) of each mode, where the definition
    /// splits a printed choice; empty: the same index.
    pub lines: &'static [u8],
    /// A torpid minion meeting its requirement bars its burn option too
    /// (The Red Question: LSJ 20091204, over rules-qa 33.8's ready one).
    pub burn_option_torpid: bool,
    /// "If you would replace this card, instead you can take this card from
    /// your ash heap" (Steely Tenacity; `Seat::retakes`).
    pub retake_from_ash: bool,
    /// "When drawing to replace this card, you may take … from your
    /// research area instead" (Development; `Interrupt::ReplaceFromResearch`).
    pub replace_from_research: bool,
}

/// An ally matching one of `of` (any, if empty): the allies an ash heap
/// offers (Split the Veil, Hel-Blá).
pub fn ash_ally(card: u16, of: &[CardFilter]) -> bool {
    LIBRARY[card as usize].types.contains(&CardType::Ally)
        && (of.is_empty() || of.iter().any(|f| f.matches(card)))
}

/// `CardFilter::RequiresImbued`.
pub fn requires_imbued(card: u16) -> bool {
    let c = &LIBRARY[card as usize];
    let imbued = || crate::crypt::CRYPT.iter().filter(|v| v.imbued);
    c.types
        .iter()
        .any(|t| matches!(t, CardType::Power | CardType::Conviction))
        || c.clans.iter().any(|k| imbued().any(|v| v.clan == *k))
        || c.disciplines
            .iter()
            .any(|d| imbued().any(|v| v.disciplines.iter().any(|vd| vd.eq_ignore_ascii_case(d))))
        || lookup(card).is_some_and(|d| {
            d.modes.iter().any(|m| {
                m.requires.iter().any(|r| {
                    matches!(
                        r,
                        Requirement::ActorImbued
                            | Requirement::ControlsReadyImbued
                            | Requirement::ReadyImbuedInPlay
                    )
                })
            })
        })
}

/// A melee weapon's printed strike (Anticipation): its `Melee` strike
/// ability, if any.
pub fn melee_strike(card: u16) -> Option<(usize, StrikeKind)> {
    weapon_damage_strike(card).filter(|(_, k)| matches!(k, StrikeKind::Melee { .. }))
}

/// A weapon's damaging strike, melee or not: its ability's index and kind.
pub fn weapon_damage_strike(card: u16) -> Option<(usize, StrikeKind)> {
    let def = lookup(card)?;
    def.abilities.iter().enumerate().find_map(|(i, m)| {
        m.effects.iter().find_map(|e| match e {
            Effect::Strike(k @ (StrikeKind::Melee { .. } | StrikeKind::Damage { .. })) => {
                Some((i, *k))
            }
            _ => None,
        })
    })
}

/// A weapon's regular strike damage against a generic opponent, and
/// whether any is aggravated (RTR 19980623); None without a damaging strike.
pub fn regular_strike_damage(card: u16) -> Option<(u8, bool)> {
    let def = lookup(card)?;
    def.abilities
        .iter()
        .flat_map(|m| m.effects.iter())
        .filter_map(|e| match e {
            Effect::Strike(StrikeKind::Damage { n, aggravated, .. }) => Some((*n, *aggravated)),
            Effect::Strike(StrikeKind::Melee { bonus, aggravated }) => {
                Some((1 + bonus, *aggravated))
            }
            _ => None,
        })
        .reduce(|(a, x), (b, y)| (a.max(b), x || y))
}

/// A card whose `mode` in play burns as the combat ends (Inner Essence,
/// Molotov Cocktail).
pub fn burns_at_combat_end(card: u16, mode: u8) -> bool {
    lookup(card).is_some_and(|d| {
        d.modes[mode as usize]
            .effects
            .contains(&Effect::BurnAtCombatEnd)
    })
}

/// As `burns_at_combat_end`, for the action (Into Thin Air).
pub fn burns_at_action_end(card: u16, mode: u8) -> bool {
    lookup(card).is_some_and(|d| {
        d.modes[mode as usize]
            .effects
            .contains(&Effect::BurnAtActionEnd)
    })
}

/// A card that becomes weapon equipment once in play (Weighted Walking
/// Stick).
pub fn becomes_weapon(card: u16) -> bool {
    lookup(card).is_some_and(|d| {
        d.modes
            .iter()
            .any(|m| m.effects.contains(&Effect::BecomesWeapon))
    })
}

/// A weapon with a ranged strike, Zip Gun included (LSJ 20060825).
pub fn is_ranged_weapon(card: u16) -> bool {
    is_weapon(card)
        && lookup(card).is_some_and(|d| {
            d.abilities.iter().flat_map(|m| m.effects.iter()).any(|e| {
                matches!(
                    e,
                    Effect::Strike(
                        StrikeKind::Damage { ranged: true, .. }
                            | StrikeKind::StealBlood { ranged: true, .. }
                            | StrikeKind::RangedBurnEquipment
                    )
                )
            })
        })
}

/// A gun: printed so, or one once in play (Zip Gun, `Effect::BecomesGun`).
pub fn is_gun(card: u16) -> bool {
    LIBRARY[card as usize]
        .labels
        .contains(&crate::library::Label::Gun)
        || lookup(card).is_some_and(|d| {
            d.modes
                .iter()
                .any(|m| m.effects.contains(&Effect::BecomesGun))
        })
}

/// A weapon: printed so, or one once in play (`becomes_weapon`).
pub fn is_weapon(card: u16) -> bool {
    LIBRARY[card as usize]
        .labels
        .contains(&crate::library::Label::Weapon)
        || becomes_weapon(card)
}

/// The title a title card represents (`Effect::RepresentsTitle`), if any.
pub fn card_title(card: u16) -> Option<Title> {
    lookup(card)?.modes.iter().find_map(|m| {
        m.effects.iter().find_map(|e| match e {
            Effect::RepresentsTitle(t) => Some(*t),
            _ => None,
        })
    })
}

/// A Discipline tag's identity, whatever its level ("obl" for "OBL").
pub fn discipline_key(tag: &str) -> &'static str {
    let lower = tag.to_ascii_lowercase();
    DISCIPLINE_KEYS
        .iter()
        .copied()
        .find(|k| *k == lower)
        .unwrap_or("?")
}

/// A tag requiring a Discipline: Maleficia and Striga are "not a
/// Discipline" (their master cards' text).
pub fn is_discipline(tag: &str) -> bool {
    !matches!(discipline_key(tag), "?" | "mal" | "str")
}

/// Every Discipline key (`discipline_key`), an index for a level table
/// (Dual Form's copy).
pub const DISCIPLINE_KEYS: [&str; 34] = [
    "abo", "ani", "aus", "cel", "chi", "dai", "dem", "dom", "fli", "for", "mel", "myt", "nec",
    "obe", "obf", "obl", "obt", "pot", "pre", "pro", "qui", "san", "ser", "spi", "str", "tem",
    "tha", "thn", "val", "vic", "vis", "mal", "flight", "vin",
];

/// A non-electronic, non-vehicle, non-weapon equipment card (British
/// Museum), locations in play included (ANK 20220824).
pub fn plain_equipment(card: u16) -> bool {
    use crate::library::Label;
    let c = &LIBRARY[card as usize];
    c.types.contains(&CardType::Equipment)
        && ![Label::Electronic, Label::Vehicle, Label::Weapon, Label::Gun]
            .iter()
            .any(|l| c.labels.contains(l))
}

/// The library mode (printed Discipline level, windows) of definition mode
/// `mi`: its `CardDef::lines` entry, else the same index.
pub fn library_mode(card: u16, mi: usize) -> usize {
    match lookup(card) {
        Some(d) if !d.lines.is_empty() => d.lines[mi] as usize,
        _ => mi,
    }
}

/// An action card targeting `seat`'s minion or card with `label` (Mobile
/// HQ), played from hand or as if (Fleshforge Chamber, LSJ 20060106); a
/// card Piper names has no as-played window (RTR 20001020).
pub fn played_action_card_at(
    st: &GameState,
    seat: SeatId,
    label: crate::library::Label,
    p: &PlayCtx,
) -> bool {
    // A political action card's terms choose, never target (rules-qa
    // 38.6).
    let action_card = lookup(p.card).is_some_and(|d| {
        d.modes[p.mode as usize].effects.iter().any(|e| {
            matches!(e, Effect::DeclareAction(k) if *k != crate::decision::ActionKind::Political)
        })
    });
    let labeled = |card: u16| LIBRARY[card as usize].labels.contains(&label);
    action_card
        && match p.target {
            Target::Minion(m) => {
                let v = &st.minions[m.index()];
                v.owner == seat
                    && matches!(v.kind, crate::state::MinionKind::Ally { card, .. }
                        if labeled(card))
            }
            Target::Card(c) => st.card_controller(c) == seat && labeled(st.cards[c.index()].card),
            _ => false,
        }
}

/// Would the action card `p` target `seat`, a minion or a card of theirs
/// (Esprit de Corps)? A bleed targets the Methuselah it bleeds.
pub fn played_action_card_targets(st: &GameState, seat: SeatId, p: &PlayCtx) -> bool {
    let kind = lookup(p.card).and_then(|d| {
        d.modes[p.mode as usize]
            .effects
            .iter()
            .find_map(|e| match *e {
                Effect::DeclareAction(k) => Some(k),
                _ => None,
            })
    });
    match (kind, p.target) {
        (None | Some(crate::decision::ActionKind::Political), _) => false,
        (_, Target::Seat(s)) => s == seat,
        (_, Target::Minion(m)) => st.minions[m.index()].owner == seat,
        (_, Target::Card(c)) => st.card_controller(c) == seat,
        (Some(crate::decision::ActionKind::Bleed), _) => st.prey(p.seat) == seat,
        _ => false,
    }
}

/// `m` meets requirement `r` of the card it plays as if it were a Camarilla
/// vampire or a prince (Victor Gerard), or a justicar (Karl Schrekt: a list
/// naming one, never "a titled Camarilla vampire", ANK 20250410, rules-qa
/// 59.14): read on the play and as its action resolves, never by the
/// card's effects in play nor by an ability (LSJ 20101013), so not in
/// `holds`.
pub fn plays_as_sect_or_title(st: &GameState, m: MinionId, r: Requirement) -> bool {
    let has = |stat| st.printed_mod(m, stat, crate::state::PrintedCtx::default()) > 0;
    match r {
        Requirement::ActorSectIn(sects) => {
            sects.contains(&Sect::Camarilla) && has(Stat::PlaysAsCamarilla)
        }
        Requirement::ActorTitleIn(classes) => {
            classes.contains(&TitleClass::Prince) && has(Stat::PlaysAsPrince)
                || classes.contains(&TitleClass::Justicar) && has(Stat::PlaysAsJusticar)
        }
        _ => false,
    }
}

/// `m` plays the card in `mode` as a Camarilla vampire: a requirement met
/// as if, a Camarilla title implying the sect for that card (LSJ 20050128,
/// LSJ 20050128-2: Closed Session's caller votes; advanced rules §5.8.3).
pub fn plays_as_camarilla(st: &GameState, m: MinionId, mode: &ModeDef) -> bool {
    mode.requires
        .iter()
        .any(|&r| plays_as_sect_or_title(st, m, r))
}

/// A title requirement beside a Camarilla one that `m` meets only as if: its
/// off-sect title is dormant as it plays as Camarilla (Victor Gerard's
/// baron's, owner, rules-qa 104.18). `plays_as_sect_or_title` reads the
/// prince he may play as first.
pub fn title_dormant_as_camarilla(
    st: &GameState,
    m: MinionId,
    mode: &ModeDef,
    r: Requirement,
) -> bool {
    matches!(r, Requirement::ActorTitleIn(_))
        && mode
            .requires
            .iter()
            .any(|q| matches!(q, Requirement::ActorSectIn(s) if s.contains(&Sect::Camarilla)))
        && st.sect_of(m) != Some(Sect::Camarilla)
        && st.printed_mod(
            m,
            Stat::PlaysAsCamarilla,
            crate::state::PrintedCtx::default(),
        ) > 0
}

/// `m` meets the mode's sect or title requirement as if (Vlad Tepes; ANK
/// 20201028).
pub fn meets_as_if(st: &GameState, m: MinionId, mode: &ModeDef) -> bool {
    mode.requires.iter().any(|r| {
        matches!(
            r,
            Requirement::ActorSectIn(_) | Requirement::ActorTitleIn(_)
        )
    }) && st.meets_sect_and_title_requirements(m)
}

/// A referendum term's filter on `m`; `as_if`, the acting vampire choosing
/// himself is of the sect and title his card required (Vlad Tepes: he can
/// Archon himself, ANK 20201028).
#[allow(clippy::too_many_arguments)]
pub fn term_filter_holds(
    r: Requirement,
    st: &GameState,
    seat: SeatId,
    referendum: Option<&ReferendumCtx>,
    acting: Option<MinionId>,
    m: MinionId,
    action: Option<&ActionCtx>,
    as_if: bool,
) -> bool {
    as_if
        && acting == Some(m)
        && matches!(
            r,
            Requirement::ActorSectIn(_) | Requirement::ActorTitleIn(_)
        )
        || holds_for_minion(r, st, seat, referendum, acting, m, action)
}

/// A trophy (rulebook "Trophy"), any mode.
pub fn is_trophy(card: u16) -> bool {
    lookup(card).is_some_and(|d| d.modes.iter().any(|m| m.effects.contains(&Effect::Trophy)))
}

/// A card in play that became another (Enhanced Coagulant): its windowless
/// mode, typeless; only that mode's layers and riders apply.
pub fn became(card: u16, mode: u8) -> bool {
    lookup(card).is_some_and(|d| {
        d.modes
            .get(mode as usize)
            .is_some_and(|m| m.windows.is_empty())
    })
}

/// Equipment once in play: not a location (Living Manse).
pub fn is_equipment_in_play(card: u16) -> bool {
    let c = &LIBRARY[card as usize];
    c.types.contains(&CardType::Equipment) && !c.labels.contains(&crate::library::Label::Location)
}

/// A library card's definition, if implemented (without one, it is
/// unplayable but can be discarded). `None` for `u16::MAX`, the card a
/// vampire's own ability or strike plays as.
pub fn lookup(card: u16) -> Option<&'static CardDef> {
    // Indexed once: this is the engine's hottest path.
    static INDEX: std::sync::OnceLock<Vec<Option<&'static CardDef>>> = std::sync::OnceLock::new();
    INDEX
        .get_or_init(|| {
            LIBRARY
                .iter()
                .map(|c| defs::DEFS.iter().find(|d| d.name == c.name).copied())
                .collect()
        })
        .get(card as usize)
        .copied()
        .flatten()
}

/// Whom a vampire's printed modifier applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrintedTo {
    /// The vampire itself.
    Itself,
    /// The minion opposing it.
    Opponent,
    /// The other minions its controller controls.
    OthersControlled,
    /// Every other minion, any Methuselah's (Mary Anne Blaire).
    Others,
}

/// How many times a printed modifier counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Per {
    One,
    /// The Gehenna cards in play (Beckett advanced, Emergency Powers).
    GehennaInPlay,
    /// Per unique equipment on it (uncontested).
    UniqueEquipmentOn,
    /// Per ready titled vampire of other Methuselahs (Gerald Windham).
    ReadyTitledOfOthers,
    /// Per other vampire its controller controls with a title of this class
    /// (Carter Vanderweyden: each other primogen).
    OtherOwnTitled(TitleClass),
    /// Per titled vampire of the target Methuselah (Appolonius, advanced).
    TitledOfTarget,
    /// Per location its controller controls, uncontested (Guillaume
    /// Giovanni; LSJ 20080107).
    LocationsControlled,
    /// Per card of the holding on it (Luanda Magere: "each Orun he has").
    Held(Holding),
    /// Per ready unique infernal vampire its controller controls, itself
    /// included (Xeper, Sultan of Lepers).
    OwnReadyUniqueInfernal,
}

/// A vampire's printed modifier, read live (`GameState::printed_mod`) while
/// it is in play, torpor included (LSJ 20011022), not while contested.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Printed {
    pub stat: Stat,
    pub n: i8,
    pub to: PrintedTo,
    pub per: Per,
    pub requires: &'static [Requirement],
}

/// Plays cards requiring a clan as if of it (Abraham Mellon); with
/// `methuselah`, its Methuselah's cards too (Lady Scarlett Churchill).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlaysAs {
    /// The clans it plays as (Lisandro Giovanni: two).
    pub clans: &'static [&'static str],
    pub methuselah: bool,
}

/// A vampire's defaults, spread as `..VAMP` under its name.
pub const VAMP: VampireDef = VampireDef {
    name: "",
    adv: false,
    group: None,
    printed: &[],
    abilities: &[],
    riders: &[],
    plays_as: None,
    recruits_as_if: None,
    incomplete: None,
    levels: &[],
    pool_surcharge: &[],
};

/// Behavior of a vampire's printed text (`crypt.rs` stays the generated
/// printed data: sect, title, disciplines).
#[derive(Debug)]
pub struct VampireDef {
    /// Printed name; matched against `crypt::CRYPT`.
    pub name: &'static str,
    /// The advanced card of the name (`CryptCard::adv`).
    pub adv: bool,
    /// The group, when versions share the name (Theo Bell G2, G6); `None`
    /// matches any. A merged definition's: its base card's.
    pub group: Option<u8>,
    pub printed: &'static [Printed],
    /// Printed abilities; the actor `Host` is the vampire.
    pub abilities: &'static [ModeDef],
    /// Printed mandatory effects; host predicates read on the vampire.
    pub riders: &'static [Rider],
    /// The clan it plays cards requiring as if it were of it.
    pub plays_as: Option<PlaysAs>,
    /// The allies of this label it recruits as if it met their clan
    /// requirement (Winterlich's changelings).
    pub recruits_as_if: Option<crate::library::Label>,
    /// The printed text not implemented yet, if any (rules-qa 13.1).
    pub incomplete: Option<&'static str>,
    /// "+1 level of" these Disciplines (merged Helena).
    pub levels: &'static [&'static str],
    /// Cards of these names cost every Methuselah 1 more pool while it is
    /// ready (Tarautas).
    pub pool_surcharge: &'static [&'static str],
}

/// The printed trait a card requires of its minion, beyond clan and
/// Discipline ("Requires a Seraph", "an infernal vampire", "an unmerged
/// advanced vampire", a sect, "non-titled": The Red Question), or a master
/// card's "requires a ready Seraph": its first mode's (the burn option,
/// rules-qa 33.8, 104.8).
pub fn trait_requirements(card: u16) -> impl Iterator<Item = Requirement> {
    lookup(card)
        .and_then(|d| d.modes.first())
        .map_or(&[][..], |m| m.requires)
        .iter()
        .copied()
        .filter(|r| {
            matches!(
                r,
                Requirement::ActorSeraph
                    | Requirement::ActorInfernal
                    | Requirement::ActorUnmergedAdvanced
                    | Requirement::ControlsReadySeraph
                    | Requirement::ActorUntitled
                    | Requirement::ActorSectIn(_)
            )
        })
}

/// The traits a master's minion target must have, read for its burn
/// option: "put this card on a Laibon with at least three Aye and three
/// Orun" (Transcendent Laibon; LSJ 20110611-2) — never its uniqueness (LSJ
/// 20100203-2).
pub fn trait_target_requirements(card: u16) -> impl Iterator<Item = Requirement> {
    lookup(card)
        .and_then(|d| d.modes.first())
        .and_then(|m| m.target)
        .filter(|t| t.kind == TargetKind::Minion)
        .map_or(&[][..], |t| t.filter)
        .iter()
        .copied()
        .filter(|r| {
            matches!(
                r,
                Requirement::TargetSect(_)
                    | Requirement::TargetClan(_)
                    | Requirement::TargetHoldsAtLeast(..)
            )
        })
}

/// The card leaves the game once played in `mode` (Reinforcements).
pub fn removed_once_played(card: u16, mode: u8) -> bool {
    lookup(card).is_some_and(|d| {
        d.modes
            .get(mode as usize)
            .is_some_and(|m| m.effects.contains(&Effect::RemovedOncePlayed))
    })
}

/// A vampire's `VampireDef` (an ally's text is its `CardDef`).
pub fn vampire_of(m: &crate::state::Minion) -> Option<&'static VampireDef> {
    // The Great Beast's text is its card's.
    if let crate::state::MinionKind::GreatBeast { .. } = m.kind {
        return Some(&vampires::GREAT_BEAST);
    }
    // A created slave (Raw Recruit's Gargoyle): the slave's ability.
    if m.slave_of.is_some() {
        return Some(&vampires::RECRUITED_SLAVE);
    }
    let card = m.crypt_card()?;
    // Merged: the definition of both cards' text (`vampires::MERGED`), by
    // the base card's group: the advanced card merges with any group's
    // base (ANK 20220805-2).
    m.merged
        .and_then(|base| {
            let b = &crate::crypt::CRYPT[base as usize];
            vampires::MERGED
                .iter()
                .find(|d| d.name == b.name && d.group.is_none_or(|g| g == b.group))
                .copied()
        })
        .or_else(|| vampire(card))
}

/// Whether crypt card `card` prints `stat` unconditionally, read out of
/// play (Jimmy Dunn).
pub fn prints(card: u16, stat: Stat) -> bool {
    vampire(card).is_some_and(|d| d.printed.iter().any(|p| p.stat == stat && p.n > 0))
}

/// The Disciplines the Great Beast may take (rulebook "Disciplines"),
/// superior.
pub const GREAT_BEAST_DISCIPLINES: &[&str] = &[
    "ABO", "ANI", "AUS", "CEL", "CHI", "DAI", "DEM", "DOM", "FOR", "MEL", "MYT", "NEC", "OBE",
    "OBF", "OBT", "POT", "PRE", "PRO", "QUI", "SAN", "SER", "SPI", "TEM", "THA", "THN", "VAL",
    "VIC", "VIS",
];

/// A crypt card's `VampireDef`, if its text is implemented (without one,
/// it plays with no text).
pub fn vampire(card: u16) -> Option<&'static VampireDef> {
    // Read on every stat computation: indexed once by crypt card.
    static TABLE: std::sync::OnceLock<Vec<Option<&'static VampireDef>>> =
        std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        crate::crypt::CRYPT
            .iter()
            .map(|c| {
                vampires::VAMPIRES
                    .iter()
                    .find(|d| {
                        d.name == c.name && d.adv == c.adv && d.group.is_none_or(|g| g == c.group)
                    })
                    .copied()
            })
            .collect()
    })[card as usize]
}

/// The card play under announcement / in its as-played window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayCtx {
    pub seat: SeatId,
    pub card: u16,
    /// The card's copy (`copies`), out of every zone while it is played.
    pub copy: crate::copies::Untracked<crate::copies::CopyId>,
    pub mode: u8,
    /// The minion playing the card (modifiers, reactions, action cards).
    pub actor: Option<MinionId>,
    pub target: Target,
    pub x: u8,
    /// The card in play chosen by `Terms::BurnNamedInPlay`.
    pub burn: Option<CardId>,
    /// A card in hand and the mode it is used with, named by the terms
    /// (`Terms::EmployFromHand`).
    pub named: Option<(u16, u8)>,
    /// A vampire the card chooses as it is played (`Terms::OwnOfClan`:
    /// Priority Contract, Toreador Grand Ball).
    pub chosen: Option<MinionId>,
    /// A card in a Methuselah's ash heap named as the card is played
    /// (`Terms::AnyAshAlly`, Psychophagia); the card `u16::MAX` while only
    /// the ash heap is chosen.
    pub ash: Option<(SeatId, u16)>,
    /// `ash` names a crypt card among library ones (Pressing Flesh's
    /// imbued).
    pub ash_crypt: bool,
    /// The cost reductions announced with the card: the source in play and
    /// the amount (The Line, The Shard, Ravnos Carnival, Ravnos Cache).
    pub discounts: [Option<(CardId, u8)>; 3],
    /// The vampire whose blood pays 1 of the card's cost, announced with the
    /// reductions (Regina Blake).
    pub blood_payer: Option<MinionId>,
    /// The card in play the card was played from, not the hand (Fleshforge
    /// Chamber): it is not replaced.
    pub from: Option<CardId>,
    /// Nergal's once-each-turn blood off, chosen for this card (rules-qa
    /// 60.7).
    pub once_off: bool,
    /// The library cards of the ash heap named as the card is played
    /// (Sudario Refraction's three; LSJ 20050422).
    pub ash_names: [Option<u16>; 3],
    /// The Blood Brothers chosen by `Terms::CirclePicks`, in order.
    pub picks: [Option<MinionId>; 8],
    /// The uncontrolled vampires named by `Terms::CommandPicks`, by slot
    /// and card.
    pub commanded: [Option<(u8, u16)>; 8],
    /// The Discipline chosen as the card is played (`Terms::SuperiorDiscipline`).
    pub discipline: Option<&'static str>,
    /// The vampire locked to take pool off the card (Jack:
    /// `Stat::LockForLocationDiscount`), announced with it.
    pub locker: Option<MinionId>,
    /// The ready vampire taking 1 blood or pool off the card once each
    /// turn (Baldesar Rossellini: `Stat::SeatCostOffOncePerTurn`),
    /// announced with it.
    pub reducer: Option<MinionId>,
}

impl PlayCtx {
    /// A play with no cost source, burn, name, chosen minion or found card.
    pub fn new(
        seat: SeatId,
        card: u16,
        mode: u8,
        actor: Option<MinionId>,
        target: Target,
        x: u8,
    ) -> Self {
        PlayCtx {
            seat,
            card,
            copy: crate::copies::Untracked(crate::copies::CopyId::NONE),
            mode,
            actor,
            target,
            x,
            burn: None,
            named: None,
            chosen: None,
            ash: None,
            ash_crypt: false,
            discounts: [None; 3],
            blood_payer: None,
            from: None,
            once_off: false,
            ash_names: [None; 3],
            picks: [None; 8],
            commanded: [None; 8],
            locker: None,
            reducer: None,
            discipline: None,
        }
    }
}

/// The "as a vampire" grant ally `m` plays mode `mi` of `card` with, if
/// any: a card requiring one of its Disciplines, no action card where it
/// says "non-action"; only that card sees a vampire (rules-qa 73.6).
pub fn plays_card_as_vampire(
    st: &GameState,
    m: MinionId,
    card: u16,
    mi: usize,
) -> Option<AsVampire> {
    if !st.minions[m.index()].is_ally() {
        return None;
    }
    let tags = LIBRARY[card as usize].modes[library_mode(card, mi)].tags;
    let declares = lookup(card).is_some_and(|d| {
        d.modes[mi]
            .effects
            .iter()
            .any(|e| matches!(e, Effect::DeclareAction(_)))
    });
    let strike = crate::plays::is_strike_mode(card, mi as u8);
    // Played as a combat card: a dual card's combat mode only (rules-digest
    // D18).
    let lib = &LIBRARY[card as usize];
    let as_combat = lib.types.contains(&CardType::Combat)
        && (lib.types.len() == 1
            || lookup(card).is_some_and(|d| d.modes[mi].windows.iter().all(|w| w.is_combat())));
    st.plays_as_vampire(m).filter(|g| {
        !(g.strike_only && !strike)
            && !(g.combat_only && !as_combat)
            && !(g.once_per_combat && st.minions[m.index()].as_vampire_spent)
            && tags.iter().any(|t| g.disciplines.contains(t))
            && !(g.non_action && declares)
            && !(g.non_ally && LIBRARY[card as usize].types.contains(&CardType::Ally))
    })
}

/// Whether `m` meets the Discipline tags of mode `mi` of `card` (lower
/// case any level, upper superior): every tag for a combo card, else one.
pub fn has_disciplines(st: &GameState, m: MinionId, card: u16, mi: usize) -> bool {
    disciplines_met(st, m, card, mi, true)
}

/// The equipment whose blood meets mode `mi`'s Discipline for vampire `m`,
/// and that blood, when nothing else meets it (Amulet of Temporal
/// Perception).
pub fn paid_discipline(
    st: &GameState,
    m: MinionId,
    card: u16,
    mi: usize,
) -> Option<(crate::state::CardId, u8)> {
    if disciplines_met(st, m, card, mi, false) || !disciplines_met(st, m, card, mi, true) {
        return None;
    }
    LIBRARY[card as usize].modes[library_mode(card, mi)]
        .tags
        .iter()
        .find_map(|t| st.pays_to_play_as_basic(m, t))
}

/// `has_disciplines`, a paid "as if" counted when `paid`.
fn disciplines_met(st: &GameState, m: MinionId, card: u16, mi: usize, paid: bool) -> bool {
    // An ally "as a vampire" meets a grant's basic Discipline, one
    // requirement at a time (The Vozhd of Juiz de Fora, Spectral Servitor;
    // LSJ 20011217; ANK 20180111), within the whole grant (rules-qa 73.6).
    // An imbued's virtues are its own (rulebook "Appendix: Imbued Rules" 1).
    let vampire = st.minions[m.index()].is_vampire() || st.minions[m.index()].is_imbued();
    let disc = if vampire {
        &[][..]
    } else {
        plays_card_as_vampire(st, m, card, mi).map_or(&[][..], |g| g.disciplines)
    };
    // A Discipline's tag, or "requires flight" (As the Crow, Patrol).
    let tags: Vec<&&str> = LIBRARY[card as usize].modes[library_mode(card, mi)]
        .tags
        .iter()
        .filter(|t| **t == "FLIGHT" || t.len() == 3 && t.chars().all(|c| c.is_ascii_alphabetic()))
        .collect();
    if tags.is_empty() {
        return true;
    }
    // Tremere Convocation: a combat card's superior at basic.
    let combat = LIBRARY[card as usize].types.contains(&CardType::Combat);
    let has = |t: &str| {
        // An ally's printed flight too (Herald of Topheth).
        if t == "FLIGHT" {
            st.has_discipline(m, t)
        } else if vampire {
            st.has_discipline(m, t)
                || paid && st.pays_to_play_as_basic(m, t).is_some()
                || combat
                    && t.chars().all(|c| c.is_ascii_uppercase())
                    && st.basic_plays_superior_combat(t)
                    && st.has_discipline(m, &t.to_ascii_lowercase())
        } else if t.chars().all(|c| c.is_ascii_uppercase()) {
            disc.contains(&t)
        } else {
            let upper = t.to_ascii_uppercase();
            disc.iter().any(|d| *d == t || *d == upper)
        }
    };
    // A combo card, or a multi-Discipline card's mode (the set of
    // Disciplines used, LSJ 20011204-3): every tag.
    let lib = &LIBRARY[card as usize];
    let multi = lib.labels.contains(&crate::library::Label::MultiDiscipline);
    // A grant plays one requirement, not both of its Disciplines at once
    // (Grey Thorne; ANK 20180731-1).
    if multi && !vampire && tags.len() > 1 {
        return false;
    }
    if lib.combo || multi {
        tags.iter().all(|t| has(t))
    } else {
        tags.iter().any(|t| has(t))
    }
}

/// A target-independent requirement for `seat` playing with `actor`, in
/// context; an arm returning `true` is read elsewhere (`Requirement::slots`).
#[allow(clippy::too_many_arguments)]
pub fn holds(
    req: Requirement,
    st: &GameState,
    seat: SeatId,
    actor: Option<MinionId>,
    played: Option<&PlayCtx>,
    action: Option<&ActionCtx>,
    combat: Option<&CombatCtx>,
    referendum: Option<&ReferendumCtx>,
    waives_needed: bool,
) -> bool {
    // An actor out of the combat plays for its Methuselah's combatant
    // (`Actor::OwnOutOfCombat`).
    let side = combat.and_then(|c| {
        actor
            .and_then(|m| c.side_of_minion(m))
            .or_else(|| actor.and(c.side_of(seat)))
    });
    let political = referendum.filter(|r| r.kind == RefKind::Political);
    match req {
        Requirement::TargetSameCircle { .. }
        | Requirement::ActorSlaveOfHostClan
        | Requirement::InitialStrikeIsOrMayBeThis
        | Requirement::TargetLacksActorSuperior
        | Requirement::TargetCirclelessBloodBrother
        | Requirement::TargetUnique
        | Requirement::TargetWouldNotContest
        | Requirement::Controlled
        | Requirement::BloodAtLeast(_)
        | Requirement::NotActingController
        | Requirement::NotSelf
        | Requirement::NotPrey
        | Requirement::HasCastVotes
        | Requirement::NotAbstaining
        | Requirement::NotControlled
        | Requirement::TargetLocked
        | Requirement::TargetVampire
        | Requirement::TargetImbued
        | Requirement::TargetMonster
        | Requirement::TargetConvictionUnplayed
        | Requirement::TargetDiablerieSinceYourLastTurn
        | Requirement::TargetBledSinceYourLastTurn { .. }
        | Requirement::TargetUntitled
        | Requirement::TargetInfernal
        | Requirement::TargetNotInfernal
        | Requirement::TargetSlaveOfActorClan
        | Requirement::BloodAtMost(_)
        | Requirement::TargetCombatant
        | Requirement::TargetNotOfOpposingController
        | Requirement::TargetControlAboutToChange
        | Requirement::TargetHuntedToBurn
        | Requirement::TargetAboutToEnterCombat
        | Requirement::TargetFacesAggravatedStrike
        | Requirement::TargetSeatOusting
        | Requirement::TargetSeatLostTitled
        | Requirement::ActiveIsAgainst
        | Requirement::CanSaveTarget
        | Requirement::BoonTargetActive
        | Requirement::TargetCard(_)
        | Requirement::TargetUsesCounters
        | Requirement::TargetOnReadyMinion
        | Requirement::ActorMayHoldTarget
        | Requirement::DirectedAtRemembered
        | Requirement::TargetCorrupted
        | Requirement::TargetOnOpposingCombatant
        | Requirement::TargetAllyCard(_)
        | Requirement::CardControlledByPrey
        | Requirement::CardControllerMoreLocations
        | Requirement::CardControlledByPredatorOrPrey
        | Requirement::CardControllerActive => true, // target or card predicates
        Requirement::PlayedCardIsMaster => {
            played.is_some_and(|p| LIBRARY[p.card as usize].types.contains(&CardType::Master))
        }
        Requirement::PlayedByOtherSeat => played.is_some_and(|p| p.seat != seat),
        Requirement::PlayedCardIsMinionCard => {
            // Neither a master card nor an event.
            played.is_some_and(|p| {
                let t = LIBRARY[p.card as usize].types;
                !t.contains(&CardType::Master) && !t.contains(&CardType::Event)
            })
        }
        Requirement::PlayedByMonster => {
            played.is_some_and(|p| p.actor.is_some_and(|m| st.is_monster(m)))
        }
        Requirement::PlayedByPredatorOrPrey => played.is_some_and(|p| {
            p.seat != seat && (p.seat == st.predator(seat) || p.seat == st.prey(seat))
        }),
        Requirement::NotOwnTurn => st.active != seat,
        Requirement::DirectedAtYou => action.is_some_and(|a| a.directed_seats(st).contains(seat)),
        Requirement::IsBleed => action.is_some_and(|a| a.kind == ActionKind::Bleed),
        Requirement::ActingNotYourPredator => action.is_some_and(|a| a.seat != st.predator(seat)),
        Requirement::IsHunt => action.is_some_and(|a| a.is_hunt()),
        Requirement::RulebookHunt => action.is_some_and(|a| a.kind == ActionKind::Hunt),
        Requirement::IsDiablerie => action.is_some_and(|a| a.kind == ActionKind::Diablerie),
        Requirement::ActorNotActing => !action.is_some_and(|a| actor == Some(a.acting)),
        Requirement::IsEmploy => action.is_some_and(|a| a.is_employ_action(st)),
        Requirement::IsRecruitOrEmploy => {
            action.is_some_and(|a| a.kind == ActionKind::Recruit || a.is_employ_action(st))
        }
        Requirement::IsEquipLabeledFromHand(label) => action.is_some_and(|a| {
            a.kind == ActionKind::Equip
                && a.card
                    .is_some_and(|p| LIBRARY[p.card as usize].labels.contains(&label))
        }),
        Requirement::IsEquipWeaponFromHand => {
            action.is_some_and(|a| a.printed_ctx(st, None).equip_weapon)
        }
        Requirement::ActionEntersCombat => {
            action.is_some_and(|a| a.printed_ctx(st, None).enters_combat)
        }
        Requirement::ActionPutsVampireInPlay => {
            action.is_some_and(|a| a.printed_ctx(st, None).puts_vampire_in_play)
        }
        Requirement::ActionRequiresPath(path) => action.is_some_and(|a| {
            let card = a.card.map(|p| p.card).or(match a.source {
                Some(AbilitySource::Card(c)) => Some(st.cards[c.index()].card),
                _ => None,
            });
            card.is_some_and(|c| LIBRARY[c as usize].path == Some(path))
        }),
        Requirement::ActingAllyOf(labels) => action.is_some_and(|a| st.ally_of(a.acting, labels)),
        Requirement::ActingHoldsNone(h) => action.is_some_and(|a| st.holds_count(a.acting, h) == 0),
        Requirement::ActingHolds(h) => action.is_some_and(|a| st.holds_count(a.acting, h) > 0),
        Requirement::FoughtByYoungerActing => action.is_some_and(|a| {
            actor.is_some_and(|m| {
                a.about_to_fight(st, m)
                    && st.minions[m.index()].is_vampire()
                    && st.minions[a.acting.index()].is_vampire()
                    && st.younger_by(a.acting, m, seat)
            })
        }),
        Requirement::BlockingActingAlly => action.is_some_and(|a| {
            a.blocker.is_some() && a.blocker == actor && st.minions[a.acting.index()].is_ally()
        }),
        Requirement::FoughtByActingAlly => action.is_some_and(|a| {
            actor.is_some_and(|m| a.about_to_fight(st, m)) && st.minions[a.acting.index()].is_ally()
        }),
        Requirement::FightsAlly => action.is_some_and(|a| {
            actor == Some(a.acting)
                && a.acting_foe(st)
                    .is_some_and(|f| st.minions[f.index()].is_ally())
        }),
        Requirement::ActorAttempting => {
            action.is_some_and(|a| matches!(a.state, ActionState::Attempt(m) if actor == Some(m)))
        }
        Requirement::AttemptingOfActorsMasterClan => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => {
                Some(m) != actor
                    && st.minions[m.index()].is_vampire()
                    && actor
                        .and_then(|r| st.slave_clan(r))
                        .is_some_and(|k| st.clan_or_creed(m) == Some(k))
            }
            _ => false,
        }),
        Requirement::BlockerAllyOrCapacityAtMost(n) => {
            action.and_then(|a| a.blocker).is_some_and(|b| {
                st.minions[b.index()].is_ally() || st.capacity_by(b, seat).is_some_and(|c| c <= n)
            })
        }
        Requirement::ActionCombatRoundsAtMost(n) => action.is_some_and(|a| a.combat_rounds <= n),
        Requirement::ActorBloodLostThisActionBelow(n) => {
            actor.is_some_and(|m| st.minions[m.index()].blood_lost_action.0 < n)
        }
        Requirement::ActorEndedNoCombatEarly => {
            action.is_some_and(|a| actor.is_none_or(|m| !a.ended_combat_early.contains(&m)))
        }
        // Its own placement as it resolves, not one on a hit or after combat
        // (no such card is played outside combat: LSJ 20080618).
        Requirement::PlayedPutsOnUncontrolledWith(d) => played.is_some_and(|p| {
            lookup(p.card).is_some_and(|def| {
                def.modes[p.mode as usize]
                    .effects
                    .contains(&Effect::PutOnTarget)
            }) && matches!(p.target, Target::Minion(m)
                    if st.minions[m.index()].is_vampire()
                        && st.minions[m.index()].owner != p.seat
                        && st.discipline_level(m, d) > 0)
        }),
        Requirement::DirectedAtCard(filters) => action.is_some_and(|a| {
            a.directed_seat(st).is_some()
                && matches!(a.target, Target::Card(c)
                    if filters.iter().any(|f| f.matches(st.cards[c.index()].card)))
        }),
        Requirement::BlockingActingOfSect(sect) => action.is_some_and(|a| {
            a.blocker.is_some()
                && a.blocker == actor
                && st.minions[a.acting.index()].is_vampire()
                && st.sect_of(a.acting) == Some(sect)
        }),
        Requirement::FoughtByActingOfSect(sect) => action.is_some_and(|a| {
            actor.is_some_and(|m| a.about_to_fight(st, m))
                && st.minions[a.acting.index()].is_vampire()
                && st.sect_of(a.acting) == Some(sect)
        }),
        Requirement::TargetOfActingOfSect(sect) => action.is_some_and(|a| {
            actor.is_some_and(|m| a.target_minions().any(|t| t == m))
                && a.directed_seat(st).is_some()
                && st.minions[a.acting.index()].is_vampire()
                && st.sect_of(a.acting) == Some(sect)
        }),
        Requirement::BlockedByVampireOfSect(sect) => action.is_some_and(|a| {
            let opposing = match a.state {
                ActionState::QueuedCombat => a.queued_opponent(a.acting),
                _ => a.blocker,
            };
            actor == Some(a.acting)
                && opposing.is_some_and(|b| {
                    st.minions[b.index()].is_vampire() && st.sect_of(b) == Some(sect)
                })
        }),
        Requirement::TermsChoseActor => {
            political.is_some_and(|r| actor.is_some_and(|m| r.terms.minions.contains(&m)))
        }
        Requirement::BlockingYoungerActing => action.is_some_and(|a| {
            a.blocker.is_some()
                && a.blocker == actor
                && st.minions[a.acting.index()].is_vampire()
                && actor.is_some_and(|m| st.younger_by(a.acting, m, seat))
        }),
        Requirement::ActorAllyOf(labels) => actor.is_some_and(|m| st.ally_of(m, labels)),
        Requirement::AttemptingAllyOf(labels) => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => {
                Some(m) != actor && st.minions[m.index()].owner == seat && st.ally_of(m, labels)
            }
            _ => false,
        }),
        Requirement::NoAimYet => combat.is_some_and(|c| {
            actor.and_then(|m| c.side_of_minion(m)).is_some_and(|i| {
                let s = &c.sides[i];
                s.aim == 0 && s.chosen && s.strike.is_some() && !s.aim_declined && !s.aim_played
            })
        }),
        Requirement::OwnCombatantOtherCopy => true, // a host predicate
        Requirement::PreyBurnedPool => action.is_some_and(|a| a.prey_burned),
        Requirement::ActingFollowsPath(path) => action.is_some_and(|a| st.follows(a.acting, path)),
        Requirement::NoQueuedCombat => !action.is_some_and(|a| a.queued_combat.is_some()),
        // The target counted as seated: a bleed ousting the prey was
        // against the prey (rules-qa 61.2).
        Requirement::BleedAgainstPrey => action.is_some_and(|a| {
            a.kind == ActionKind::Bleed
                && a.bleed_target
                    .is_some_and(|t| st.prey_counting(a.seat, t) == t)
        }),
        Requirement::BleedSuccessful => action.is_some_and(|a| a.bleed_successful),
        Requirement::BleedAmountAtLeast(n) => action.is_some_and(|a| a.bleed_amount(st) >= n),
        Requirement::BlocksDeclined => action.is_some_and(|a| a.state == ActionState::Declined),
        Requirement::AfterBlocksDeclined => action.is_some_and(|a| {
            matches!(
                a.state,
                ActionState::Declined | ActionState::WouldSucceed | ActionState::AfterResolution
            ) && a.blocker.is_none()
        }),
        Requirement::StealthNeeded => {
            waives_needed
                || action.is_some_and(|a| {
                    matches!(a.state, ActionState::Attempt(_))
                        && a.attempt_intercept(st) >= a.stealth(st)
                })
        }
        Requirement::InterceptNeeded => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => actor == Some(m) && a.stealth(st) > a.intercept_of(st, m),
            _ => false,
        }),
        Requirement::LockedActor => actor.is_some_and(|m| st.minions[m.index()].locked),
        Requirement::ActorInTorpor => actor.is_some_and(|m| st.minions[m.index()].in_torpor),
        Requirement::ActorSucceededThisTurn => {
            actor.is_some_and(|m| st.minions[m.index()].succeeded_this_turn)
        }
        Requirement::NoOtherOwnAttemptedThisTurn => actor.is_some_and(|m| {
            st.minions_of(st.minions[m.index()].owner)
                .all(|o| o == m || !st.minions[o.index()].attempted_this_turn)
        }),
        Requirement::MasterPhaseActionLeft => st.mpa_left(seat) > 0,
        Requirement::HasEdge => st.seats[seat.index()].has_edge,
        Requirement::NoEdge => !st.seats[seat.index()].has_edge,
        Requirement::HandNotEmpty => !st.seats[seat.index()].hand.is_empty(),
        Requirement::CryptNotEmpty => !st.seats[seat.index()].crypt.is_empty(),
        Requirement::NoCryptEmpty => st.seats.iter().all(|s| s.ousted || !s.crypt.is_empty()),
        Requirement::PredatorAndPreyReadyVampires => {
            let ready = |s: SeatId| {
                st.minions_of(s).any(|m| {
                    let v = &st.minions[m.index()];
                    v.is_vampire() && v.ready()
                })
            };
            st.predator(seat) != st.prey(seat) && ready(st.predator(seat)) && ready(st.prey(seat))
        }
        // Only master cards requiring the clan print it: a vampire playing
        // as one for its Methuselah counts (Lady Scarlett Churchill).
        Requirement::ControlsReadyClan(clan, n) => {
            st.minions_of(seat)
                .filter(|&m| st.minions[m.index()].ready() && st.meets_clan(m, &[clan], true))
                .count()
                >= n as usize
        }
        Requirement::OwnReadyClanWithBlood(clan) => st.minions_of(seat).any(|m| {
            let v = &st.minions[m.index()];
            v.ready() && v.clan() == Some(clan) && v.blood > 0
        }),
        Requirement::TargetIsHost
        | Requirement::TargetChosenByHostThisTurn
        | Requirement::ActorIsHost
        | Requirement::ActorIsChosen
        | Requirement::ActorOlderThanHost
        | Requirement::ActorOlderThanChosen
        | Requirement::StrikingWithItsGun
        | Requirement::HoldRecipient(_)
        | Requirement::HandHasStoredCopy
        | Requirement::NotPlacedThisRound
        | Requirement::PlayedFrenzyOnYour(_)
        | Requirement::PlayedFrenzyOnHost
        | Requirement::PlayedFrenzyOnActor
        | Requirement::PlayedFrenzy
        | Requirement::PlayedActionTargetsActor
        | Requirement::NotPlayedThisGame
        | Requirement::NotPlayedThisGameBySeat
        | Requirement::NotPlayedThisTurn
        | Requirement::OnePerMethuselah
        | Requirement::PlayedLabeledByOpponent(_)
        | Requirement::PlayedCombatByOpponent
        | Requirement::WeaponBurnedOpponent
        | Requirement::TargetInCombatWithDamage
        | Requirement::TargetInCombatWithNormalDamage
        | Requirement::TargetWeaponStrikeUsable
        | Requirement::OneCopyPerTurn
        | Requirement::FewerCopiesUsedThisPhase(_)
        | Requirement::ActiveControlsNonSect(_) => true, // engine-checked
        Requirement::NoOptionalActionThisTurn => !st.optional_action,
        Requirement::AshHasAlly(of) => st.seats[seat.index()].ash.iter().any(|&c| ash_ally(c, of)),
        Requirement::BurnedAllyInAnyAsh => (0..st.seats.len()).any(|i| {
            let s = SeatId(i as u8);
            !st.seats[i].ousted
                && !(st.ash_burned_allies(s).is_empty() && st.ash_burned_imbued(s).is_empty())
        }),
        Requirement::AllyInAnyAsh => st.seats.iter().any(|s| {
            !s.ousted
                && s.ash
                    .iter()
                    .any(|&c| LIBRARY[c as usize].types.contains(&CardType::Ally))
        }),
        Requirement::TargetInTorpor
        | Requirement::TargetIncapacitated
        | Requirement::TargetConvictionAtLeast(_)
        | Requirement::ControlledByPrey
        | Requirement::TargetSectNot(_)
        | Requirement::TargetSect(_)
        | Requirement::TargetRedList
        | Requirement::TargetNotBlackHand
        | Requirement::TargetBlackHand
        | Requirement::TargetUnmerged
        | Requirement::TargetNotRedList
        | Requirement::TargetNotBlocker
        | Requirement::TargetAtActionsEnd
        | Requirement::TargetCastVotes
        | Requirement::TargetNotActing
        | Requirement::TargetNotAttempting
        | Requirement::TargetCapacityAtLeast(_)
        | Requirement::TargetCapacityAtMost(_)
        | Requirement::TargetCapacityAtMostTwiceHeld(_)
        | Requirement::TargetCapacityAtMostActorTimes(_)
        | Requirement::TargetHoldsAtLeast(..)
        | Requirement::NotGehennaImmune
        | Requirement::SameClanAsActor
        | Requirement::ActorAndTargetSect(_)
        | Requirement::TargetDiscipline(_)
        | Requirement::TargetMithraic
        | Requirement::TargetNotNamed(_)
        | Requirement::TargetAnarch
        | Requirement::TargetHostage => {
            true // target predicates
        }
        Requirement::ActingVampire => {
            action.is_some_and(|a| st.minions[a.acting.index()].is_vampire())
        }
        Requirement::ActingMonster => action.is_some_and(|a| st.is_monster(a.acting)),
        Requirement::ActingImbued => {
            action.is_some_and(|a| st.minions[a.acting.index()].is_imbued())
        }
        Requirement::ActingCapacityAtLeast(n) => action.is_some_and(|a| {
            st.minions[a.acting.index()].is_vampire()
                && st.capacity_by(a.acting, seat).is_some_and(|c| c >= n)
        }),
        Requirement::ActingCapacityAtMost(n) => action.is_some_and(|a| {
            st.minions[a.acting.index()].is_vampire()
                && st.capacity_by(a.acting, seat).is_some_and(|c| c <= n)
        }),
        Requirement::ActionUndirected => action.is_some_and(|a| a.directed_seat(st).is_none()),
        Requirement::ActingClan(c) => {
            action.is_some_and(|a| st.minions[a.acting.index()].clan() == Some(c))
        }
        Requirement::NotFirstActionOfPhase => st.phase_actions >= 2,
        Requirement::AnarchHuntedYours => action.is_some_and(|a| {
            a.is_hunt()
                && a.hunt_successful
                && a.seat == seat
                && st.sect_of(a.acting) == Some(Sect::Anarch)
        }),
        Requirement::PoolAtLeast(n) => st.seats[seat.index()].pool >= n as i32,
        Requirement::PoolAtMost(n) => st.seats[seat.index()].pool <= n as i32,
        Requirement::ControlsReadyOnPath(path, n) => {
            st.minions_of(seat)
                .filter(|&m| st.minions[m.index()].ready() && st.follows(m, path))
                .count()
                >= n as usize
        }
        Requirement::ControlsPrinceOf(city) => st
            .minions_of(seat)
            .any(|m| st.title_of(m) == crate::crypt::Title::Prince(city)),
        Requirement::ActorVampire => actor.is_some_and(|m| !st.minions[m.index()].is_ally()),
        Requirement::ActorHasSuperior => actor.is_some_and(|m| {
            crate::state::disciplines()
                .into_iter()
                .any(|d| st.discipline_level(m, d) == 2)
        }),
        // "Changeling allies can burn this location" (Black Forest Base).
        Requirement::ActorSubtype(l) => actor.is_some_and(|m| st.has_subtype(m, l)),
        Requirement::ActorImbued => actor.is_some_and(|m| st.minions[m.index()].is_imbued()),
        Requirement::ActorConvictionAtLeast(n) => {
            actor.is_some_and(|m| st.convictions(m).len() >= n as usize)
        }
        Requirement::HandHas(f, n) => st.hand_picks(seat, f).len() >= n as usize,
        Requirement::HostLocked
        | Requirement::HostDidNotHunt
        | Requirement::HostAnnouncedNoAction
        | Requirement::HostOutOfPlay
        | Requirement::BledControlsReadyTitled
        | Requirement::HostBloodAtLeast(_)
        | Requirement::HostHoldsUnlocked(..)
        | Requirement::HostConvictionAtLeast(_)
        | Requirement::HostIncapacitated
        | Requirement::ImbuedRecoveryGranted
        | Requirement::HostControllerReadyImbuedAtLeast(_)
        | Requirement::ActionNotEnteringCombat
        | Requirement::HostMayReactLocked
        | Requirement::HostNameUnusedThisAction
        | Requirement::HostInCombat
        | Requirement::HostBloodAtMost(_)
        | Requirement::HostBelowStartingLife
        | Requirement::HostBelowCapacity
        | Requirement::HostControllerPoolAtLeast(_)
        | Requirement::HostInTorpor
        | Requirement::HostControllerActive
        | Requirement::HostControllerPutNoGehenna
        | Requirement::HostControllerHasEdge
        | Requirement::ControlsReadyTitledOther(_)
        | Requirement::OtherReadyTitled(_)
        | Requirement::ActorOfSourcesPreyOrPredator
        | Requirement::ActorCapacityAtMost(_)
        | Requirement::OpponentTitleIn(_)
        | Requirement::CallerTitleIn(_)
        | Requirement::SourceHoldsNone(_)
        | Requirement::PredatorMoreReadyMinions
        | Requirement::PreyPoolAtMost(_)
        | Requirement::PreyControlsVampireNotOfSect(_)
        | Requirement::TargetSeatControlsClan(_)
        | Requirement::TargetFollowsPath(_)
        | Requirement::TargetAllyOf(_)
        | Requirement::TargetAllyNotOf(_)
        | Requirement::TargetOtherCopyControlled
        | Requirement::CountersAtLeast(_)
        | Requirement::CountersWithBleedGainAtLeast(_)
        | Requirement::CountersAtMost(_)
        | Requirement::CardUnlocked
        | Requirement::ReferendumToBurnThisCard
        | Requirement::NotFirstChosen
        | Requirement::TargetHasCorruption
        | Requirement::NotHuntingGroundFed
        | Requirement::TargetWithout(_)
        | Requirement::TargetTitleIn(_)
        | Requirement::TargetTitle(_)
        | Requirement::TargetTitleContested
        | Requirement::HostOutOfCombat
        | Requirement::OncePerTurn
        | Requirement::TargetClanNotIn(_)
        | Requirement::TargetNotOfClan(_)
        | Requirement::TargetNotSuperior(_)
        | Requirement::OtherThanActor
        | Requirement::TargetOnActor => true, // host / card / target predicates
        Requirement::ActorUnlocked | Requirement::ActorUnlockedToLock => {
            actor.is_some_and(|m| !st.minions[m.index()].locked)
        }
        Requirement::ActorMayUseEquipment => actor.is_some_and(|m| {
            !st.no_equipment(m)
                && combat
                    .and_then(|c| c.side_of_minion(m).map(|i| !c.sides[i].equipment_barred()))
                    .unwrap_or(true)
        }),
        Requirement::InflictedMoreThisRound => combat
            .zip(side)
            .is_some_and(|(c, i)| c.sides[i].inflicted_round > c.sides[1 - i].inflicted_round),
        Requirement::NotBleed => action.is_some_and(|a| a.kind != ActionKind::Bleed),
        Requirement::ActingYounger => action
            .zip(actor)
            .is_some_and(|(a, m)| st.younger_by(a.acting, m, seat)),
        Requirement::ActingAllyOrYounger => action.zip(actor).is_some_and(|(a, m)| {
            st.minions[a.acting.index()].is_ally() || st.younger_by(a.acting, m, seat)
        }),
        Requirement::RecruitingSubtype(labels) => action.is_some_and(|a| {
            matches!(a.kind, ActionKind::Recruit | ActionKind::Employ)
                && a.card.is_some_and(|p| {
                    labels
                        .iter()
                        .any(|l| LIBRARY[p.card as usize].labels.contains(l))
                })
        }),
        Requirement::HostAttempting
        | Requirement::AttemptingNotHost
        | Requirement::HostMayReact
        | Requirement::ActingIsBy
        | Requirement::BleedAgainstHostController => true, // engine-checked
        Requirement::PlayedCardType(t) => {
            played.is_some_and(|p| LIBRARY[p.card as usize].types.contains(&t))
        }
        Requirement::PlayedNotCardType(t) => {
            played.is_some_and(|p| !LIBRARY[p.card as usize].types.contains(&t))
        }
        Requirement::PlayedCard(f) => played.is_some_and(|p| f.matches(p.card)),
        Requirement::PlayedByMinionWithout(h) => played
            .and_then(|p| p.actor)
            .is_some_and(|m| st.holds_count(m, h) == 0),
        Requirement::PlayedRequiresNone(ds) => played.is_some_and(|p| {
            !LIBRARY[p.card as usize].modes[library_mode(p.card, p.mode as usize)]
                .tags
                .iter()
                .any(|t| ds.iter().any(|d| t.eq_ignore_ascii_case(d)))
        }),
        Requirement::PlayedRequiresAnyDiscipline => played.is_some_and(|p| {
            LIBRARY[p.card as usize].modes[library_mode(p.card, p.mode as usize)]
                .tags
                .iter()
                .any(|t| is_discipline(t))
        }),
        Requirement::PlayedRequiresDiscipline(d) => played.is_some_and(|p| {
            LIBRARY[p.card as usize].modes[library_mode(p.card, p.mode as usize)]
                .tags
                .iter()
                .any(|t| t.eq_ignore_ascii_case(d))
        }),
        Requirement::OpponentWithout(h) => combat
            .zip(side)
            .is_some_and(|(c, i)| !st.holds_a(c.sides[1 - i].minion, h)),
        Requirement::ActionBlocked => {
            action.is_some_and(|a| a.outcome == Some(crate::actions::Outcome::Blocked))
        }
        Requirement::ActionUnsuccessful => action.is_some_and(|a| {
            matches!(
                a.outcome,
                Some(crate::actions::Outcome::Blocked | crate::actions::Outcome::Failed)
            )
        }),
        Requirement::NoWatchDamageThisAction => action.is_some_and(|a| !a.watch_damaged),
        Requirement::NoCombatQueued => {
            combat.is_none_or(|c| !c.psyche && c.coordinate.is_none() && c.redirect.is_none())
                && action.is_none_or(|a| a.queued_combat.is_none())
        }
        Requirement::NoSuperiorVeilThisTurn => !st.veil_superior,
        Requirement::OpposingWeapon => combat.zip(actor).is_some_and(|(c, m)| {
            c.opponent_of(m).is_some_and(|o| {
                st.equipment_on(o)
                    .into_iter()
                    .any(|w| is_weapon(st.cards[w.index()].card))
            })
        }),
        Requirement::ManeuverToClose => combat.is_none_or(|c| {
            c.step != crate::combat::Step::DetermineRange || c.range == Range::Long
        }),
        Requirement::StrikeDamagePending => combat
            .zip(side)
            .is_some_and(|(c, i)| c.sides[i].strike_damage.total() > 0),
        Requirement::StrikeNormalDamagePending => combat
            .zip(side)
            .is_some_and(|(c, i)| c.sides[i].strike_damage.normal > 0),
        Requirement::EffectNormalDamagePending => combat
            .zip(side)
            .is_some_and(|(c, i)| c.sides[i].pending.normal > c.sides[i].strike_damage.normal),
        Requirement::ActorAllyCard(f) => actor.is_some_and(|m| {
            matches!(st.minions[m.index()].kind, crate::state::MinionKind::Ally { card, .. }
                if f.matches(card))
        }),
        Requirement::PlayedActionCardTargetsYou => {
            played.is_some_and(|p| played_action_card_targets(st, seat, p))
        }
        Requirement::PlayedActionCardAtYour(label) => {
            played.is_none_or(|p| played_action_card_at(st, seat, label, p))
        }
        Requirement::NoReactionPlayed => action.is_some_and(|a| !a.reaction_played),
        Requirement::OwnAshNotEmpty => !st.seats[seat.index()].ash.is_empty(),
        Requirement::OwnAshHas(f) => st.seats[seat.index()].ash.iter().any(|&c| f.matches(c)),
        Requirement::PreyAshNotEmpty => !st.seats[st.prey(seat).index()].ash.is_empty(),
        Requirement::NoGreatBeastControlled => !st.minions_of(seat).any(|m| {
            matches!(
                st.minions[m.index()].kind,
                crate::state::MinionKind::GreatBeast { .. }
            )
        }),
        Requirement::UncontrolledAtLeast(n) => {
            st.seats[seat.index()].uncontrolled.len() >= n as usize
        }
        Requirement::ActorInfernal => actor.is_some_and(|m| st.is_infernal(m)),
        Requirement::ActorSeraph => actor.is_some_and(|m| st.is_seraph(m)),
        // Meat Hook: the bearer's opponent with Potence, read live.
        Requirement::ActorHasDiscipline(d) => actor.is_some_and(|m| st.has_discipline(m, d)),
        Requirement::IsEquipAction => action.is_some_and(|a| a.is_equip_action(st)),
        Requirement::LastCombatantsReady => action
            .and_then(|a| a.last_combat)
            .is_some_and(|pair| pair.iter().all(|m| st.minions[m.index()].ready())),
        Requirement::NotPlayedThisAction(name) => {
            action.is_none_or(|a| !a.played.iter().any(|&(_, n)| n == name))
        }
        Requirement::ActorNotPlayedThisAction(name) => {
            action.is_none_or(|a| !a.played.iter().any(|&(m, n)| n == name && Some(m) == actor))
        }
        Requirement::PreyAshHasPair => {
            let ash = &st.seats[st.prey(seat).index()].ash;
            ash.iter()
                .any(|c| ash.iter().filter(|&x| x == c).count() >= 2)
        }
        Requirement::ControlsReadyTitled(classes) => st.minions_of(seat).any(|m| {
            st.minions[m.index()].ready() && classes.iter().any(|c| c.matches(st.title_of(m)))
        }),
        Requirement::ControlsReadyOfSect(sect, n) => {
            st.minions_of(seat)
                .filter(|&m| st.minions[m.index()].ready() && st.sect_of(m) == Some(sect))
                .count()
                >= n as usize
        }
        Requirement::ControlsReadySect(sects) => st.minions_of(seat).any(|m| {
            let v = &st.minions[m.index()];
            v.ready() && st.sect_of(m).is_some_and(|s| sects.contains(&s))
        }),
        Requirement::RangeClose => combat.is_some_and(|c| c.range == Range::Close),
        Requirement::RangeLong => combat.is_some_and(|c| c.range == Range::Long),
        Requirement::FirstRound => combat.is_some_and(|c| c.round == 1),
        Requirement::StrikingWithGun => combat.is_some_and(|c| {
            actor.and_then(|m| c.side_of_minion(m)).is_some_and(|i| {
                c.sides[i].strike_source.is_some_and(|w| {
                    is_gun(st.cards[w.index()].card) && st.cards[w.index()].host == actor
                })
            })
        }),
        Requirement::StrikingWithUnloadedGun => combat.is_some_and(|c| {
            actor.and_then(|m| c.side_of_minion(m)).is_some_and(|i| {
                c.sides[i].strike_source.is_some_and(|w| {
                    is_gun(st.cards[w.index()].card)
                        && st.cards[w.index()].host == actor
                        && !c.ammo.iter().any(|&(g, _)| g == w)
                })
            })
        }),
        Requirement::OpponentIsActing => combat.is_some_and(|c| {
            actor
                .and_then(|m| c.side_of_minion(m))
                .is_some_and(|i| action.is_some_and(|a| c.sides[1 - i].minion == a.acting))
        }),
        Requirement::ActorDamagedThisRound => combat.is_some_and(|c| {
            actor
                .and_then(|m| c.side_of_minion(m))
                .is_some_and(|i| c.sides[i].damaged_round)
        }),
        Requirement::NotFirstRound => combat.is_some_and(|c| c.round > 1),
        Requirement::PreviousRoundLong => combat.is_some_and(|c| c.round > 1 && !c.prev_close),
        Requirement::ActorUndamagedLastRound => combat.is_some_and(|c| {
            actor
                .and_then(|m| c.side_of_minion(m))
                .is_some_and(|i| c.sides[i].damage_last_round == 0)
        }),
        Requirement::BothCombatantsReady => combat.is_some_and(|c| c.both_ready(st)),
        Requirement::BothCombatantsOthersVampires => combat.is_some_and(|c| {
            c.sides.iter().all(|s| {
                let v = &st.minions[s.minion.index()];
                v.is_vampire() && v.owner != seat
            })
        }),
        Requirement::BothCombatantsSect(sect) => combat.is_some_and(|c| {
            c.sides.iter().all(|s| {
                st.minions[s.minion.index()].is_vampire() && st.sect_of(s.minion) == Some(sect)
            })
        }),
        Requirement::AnarchCombatWithYours => combat.is_some_and(|c| {
            let anarch = |m: MinionId| st.sect_of(m) == Some(Sect::Anarch);
            let (a, b) = (c.sides[0].minion, c.sides[1].minion);
            anarch(a)
                && anarch(b)
                && (st.minions[a.index()].owner == seat || st.minions[b.index()].owner == seat)
        }),
        Requirement::AnarchCardInAshAndHand => {
            let s = &st.seats[seat.index()];
            !s.hand.is_empty()
                && s.ash.iter().any(|&c| {
                    LIBRARY[c as usize]
                        .labels
                        .contains(&crate::library::Label::RequiresAnarch)
                })
        }
        Requirement::CombatWouldEnd => combat.is_some_and(|c| c.would_end() && !c.end_fixed),
        Requirement::ActorReady => actor.is_some_and(|m| st.minions[m.index()].ready()),
        // The prevention half of a card needs damage to prevent; a mode
        // with another window (Indomitability [FOR]'s press) is unaffected
        // outside the prevention step.
        Requirement::OpponentGoingToTorpor => combat
            .zip(side)
            .is_some_and(|(c, i)| crate::combat::going_to_torpor(st, c, 1 - i)),
        Requirement::ActorGoingToTorpor => combat
            .zip(side)
            .is_some_and(|(c, i)| crate::combat::going_to_torpor(st, c, i)),
        Requirement::ActorNotGoingDown => combat
            .zip(side)
            .is_some_and(|(c, i)| !crate::combat::going_down(st, c, i)),
        Requirement::DamagePending => combat.zip(side).is_some_and(|(c, i)| match c.step {
            Step::Prevent(p) => p == i && c.sides[i].pending.total() > 0,
            _ => true,
        }),
        Requirement::NonAggravatedDamagePending => {
            combat.zip(side).is_some_and(|(c, i)| match c.step {
                Step::Prevent(p) => p == i && c.sides[i].pending.normal > 0,
                _ => true,
            })
        }
        Requirement::AggravatedDamagePending => {
            combat.zip(side).is_some_and(|(c, i)| match c.step {
                Step::Prevent(p) => p == i && c.sides[i].pending.aggravated > 0,
                _ => true,
            })
        }
        Requirement::GunStrikeDamagePending => {
            combat.zip(side).is_some_and(|(c, i)| match c.step {
                Step::Prevent(p) => p == i && c.sides[i].pending.gun > 0,
                _ => true,
            })
        }
        Requirement::NonGunDamagePending => combat.zip(side).is_some_and(|(c, i)| match c.step {
            Step::Prevent(p) => p == i && c.sides[i].pending.total() > c.sides[i].pending.gun,
            _ => true,
        }),
        // Card and host predicates.
        Requirement::SourceHeldWhenBlocking
        | Requirement::ControllerControlsHost
        | Requirement::HostReady
        | Requirement::ActivePoxed
        | Requirement::HostVampire
        | Requirement::HostAlly => true,
        Requirement::NoContinuePress => combat.is_some_and(|c| !c.press_continue),
        Requirement::ActorHasBlocked => {
            action.is_some_and(|a| a.blocker.is_some() && a.blocker == actor)
        }
        Requirement::ActorTitleIn(classes) => actor.is_some_and(|m| {
            let t = st.title_of(m);
            classes.iter().any(|c| c.matches(t)) || st.meets_sect_and_title_requirements(m)
        }),
        Requirement::ActorTitleNotIn(classes) => actor.is_some_and(|m| {
            let t = st.title_of(m);
            !classes.iter().any(|c| c.matches(t))
        }),
        // An ally playing a card "as a vampire" is read so by that card
        // only (`plays_card_as_vampire`, the play's listing).
        Requirement::ActorSectIn(sects) => actor.is_some_and(|m| {
            st.sect_of(m).is_some_and(|s| sects.contains(&s))
                || st.meets_sect_and_title_requirements(m)
        }),
        Requirement::ActorCapacityAtLeast(n) => {
            actor.is_some_and(|m| st.capacity_by(m, seat).is_some_and(|c| c >= n))
        }
        Requirement::IsPolitical => action.is_some_and(|a| a.kind == ActionKind::Political),
        Requirement::NotPolitical => action.is_some_and(|a| a.kind != ActionKind::Political),
        Requirement::PollingStep => political.is_some_and(|r| r.step == RefStep::Polling),
        Requirement::ActorNotForcedYet => {
            political.is_some_and(|r| actor.is_some_and(|m| !r.forcers.contains(&m)))
        }
        Requirement::AnyPollingStep => referendum.is_some_and(|r| r.step == RefStep::Polling),
        Requirement::ActorActing => action.zip(actor).is_some_and(|(a, m)| a.acting == m),
        Requirement::GehennaInPlay(n) => st.gehenna_in_play() >= n as usize,
        Requirement::GehennaOfOthersInPlay(n) => st.gehenna_of_others(seat) >= n as usize,
        Requirement::NoGehennaInPlay => st.gehenna_in_play() == 0,
        Requirement::RescuedFromTorpor => action.is_some_and(|a| {
            a.kind == ActionKind::Rescue && a.outcome == Some(crate::actions::Outcome::Successful)
        }),
        Requirement::ActionSuccessful => action.is_some_and(|a| {
            matches!(
                a.outcome,
                Some(crate::actions::Outcome::Successful | crate::actions::Outcome::Fizzled)
            )
        }),
        Requirement::ActionNotFizzled => {
            action.is_some_and(|a| a.outcome == Some(crate::actions::Outcome::Successful))
        }
        Requirement::ActionCostBlood => action.is_some_and(|a| a.blood_cost > 0),
        Requirement::ActionDirected => action.is_some_and(|a| a.directed_seat(st).is_some()),
        Requirement::ActorDiablerised => action
            .zip(actor)
            .is_some_and(|(a, m)| a.diablerist == Some(m)),
        Requirement::ActingOtherController => action.is_some_and(|a| a.seat != seat),
        Requirement::OtherActingControlled => {
            action.is_some_and(|a| a.seat == seat && actor != Some(a.acting))
        }
        Requirement::OwnActing => action.is_some_and(|a| a.seat == seat),
        Requirement::ActionRequiresAnarch => action.is_some_and(|a| a.requires_anarch(st)),
        Requirement::NoSlaveInstead => action.is_some_and(|a| {
            a.seat == seat
                && a.slave.is_none()
                && !a.continued
                && matches!(a.state, crate::actions::ActionState::Blocked(_))
        }),
        Requirement::CallerTitledOfSect(sect) => {
            referendum.and_then(|r| r.calling).is_some_and(|m| {
                st.sect_of(m) == Some(sect) && st.title_of(m) != crate::crypt::Title::None
            })
        }
        Requirement::MaskCapable => match (action, actor) {
            (Some(a), Some(m)) => mask_capable(st, a, m),
            _ => false,
        },
        Requirement::NoReadyBlackHand => !(0..st.minions.len()).any(|i| {
            let m = MinionId(i as u32);
            st.minions[i].ready() && st.is_black_hand(m)
        }),
        Requirement::BlackHandVampireInPlay => (0..st.minions.len()).any(|i| {
            let v = &st.minions[i];
            v.in_play && v.is_vampire() && st.is_black_hand(MinionId(i as u32))
        }),
        Requirement::NoActionUnderway => {
            action.is_none() && combat.is_none() && referendum.is_none()
        }
        Requirement::ActionUnderway => action.is_some(),
        Requirement::ControlsReadyVampires(n) => {
            st.minions_of(seat)
                .filter(|&m| {
                    let v = &st.minions[m.index()];
                    v.ready() && v.is_vampire()
                })
                .count()
                >= n as usize
        }
        Requirement::ControlsReadyImbued => st
            .minions_of(seat)
            .any(|m| st.minions[m.index()].ready() && st.minions[m.index()].is_imbued()),
        Requirement::ReadyImbuedInPlay => st.ready_imbued_in_play(),
        Requirement::ActingSectIn(sects) => {
            action.is_some_and(|a| st.sect_of(a.acting).is_some_and(|s| sects.contains(&s)))
        }
        Requirement::ActingTitleIn(classes) => action.is_some_and(|a| {
            let t = st.title_of(a.acting);
            classes.iter().any(|c| c.matches(t))
        }),
        Requirement::ActingSectNot(sect) => {
            action.is_some_and(|a| st.sect_of(a.acting) != Some(sect))
        }
        Requirement::ActingLacksDiscipline(d) => {
            action.is_some_and(|a| st.discipline_level(a.acting, d) == 0)
        }
        Requirement::DirectedHandNotEmpty => action
            .and_then(|a| a.directed_seat(st))
            .is_some_and(|t| !st.seats[t.index()].hand.is_empty()),
        // Target predicates (`holds_for_minion`).
        Requirement::ControlledByDirectedSeat
        | Requirement::DirectedSeatHuntedSinceYourLastTurn
        | Requirement::NotHost
        | Requirement::OtherThanPlayer
        | Requirement::YoungerThanPlayer
        | Requirement::ActionCardToAsh
        | Requirement::DirectedAtCardItIsOn
        | Requirement::YoungerThanHost
        | Requirement::TargetClan(_)
        | Requirement::PredatorsPredator
        | Requirement::TargetPrey
        | Requirement::TargetPredator
        | Requirement::TargetPoolMoreThanYours
        | Requirement::TargetPredatorOrPrey
        | Requirement::TargetAshAtLeast(_)
        | Requirement::TargetHasUncontrolled => true,
        // Still the predator once the action ousted them (Donald Cargill,
        // LSJ 20090115-2).
        Requirement::ActingByPredator => {
            action.is_some_and(|a| a.seat == st.predator_keeping(seat, a.seat) && a.seat != seat)
        }
        Requirement::MoreThanTwoLeft => st.living() > 2,
        Requirement::TargetSubtypeIn(_) | Requirement::TargetSlave | Requirement::MayBeLocked => {
            true
        }
        Requirement::ActorBloodAtLeast(n) => {
            actor.is_some_and(|m| st.minions[m.index()].blood >= n)
        }
        Requirement::CanAttemptBlock | Requirement::CanAttemptBlockAnyone => true, // engine-checked
        Requirement::BleedTargetPoolAtMost(n) => action
            .and_then(|a| a.bleed_target)
            .is_some_and(|t| st.seats[t.index()].pool <= n as i32),
        Requirement::AttemptingSectIn(sects) => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => st.sect_of(m).is_some_and(|s| sects.contains(&s)),
            _ => false,
        }),
        Requirement::AnyAttemptNeedsIntercept => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => a.stealth(st) > a.intercept_of(st, m),
            _ => false,
        }),
        Requirement::ActionRequiresDiscipline(disc) => action.is_some_and(|a| {
            a.card.is_some_and(|p| {
                LIBRARY[p.card as usize].modes[library_mode(p.card, p.mode as usize)]
                    .tags
                    .iter()
                    .any(|t| discipline_key(t) == disc)
            })
        }),
        Requirement::ActionRequiresClanOrDiscipline(clan, disc) => action.is_some_and(|a| {
            let from_card = a.card.is_some_and(|p| {
                let lib = &LIBRARY[p.card as usize];
                lib.clans.contains(&clan)
                    || lib.modes[library_mode(p.card, p.mode as usize)]
                        .tags
                        .iter()
                        .any(|t| discipline_key(t) == disc)
            });
            let from_source = a.source.zip(a.ability).is_some_and(|(src, ab)| {
                src.mode(st, ab).requires.iter().any(|r| match r {
                    Requirement::ActorClan(c) => *c == clan,
                    _ => false,
                })
            });
            from_card || from_source
        }),
        Requirement::NoVampireNamedButInCrypt(name) => {
            !st.minions_of(seat)
                .any(|m| st.minions[m.index()].name() == name)
                && st.seats[seat.index()]
                    .crypt
                    .iter()
                    .any(|&c| crate::crypt::CRYPT[c as usize].name == name)
        }
        Requirement::HostCombatOpponentNotReady
        | Requirement::HostCombatOpposingVampireNotReady
        | Requirement::HostUsedDisciplines(_) => true, // engine-checked
        Requirement::DirectedAtYourOtherOfSect(sect) => action.is_some_and(|a| {
            a.directed_seat(st).is_some()
                && a.target_minions().any(|m| {
                    st.minions[m.index()].owner == seat
                        && Some(m) != actor
                        && st.sect_of(m) == Some(sect)
                })
        }),
        Requirement::DirectedAtYourReadyClan(clan) => action.is_some_and(|a| {
            a.directed_seat(st).is_some()
                && a.target_minions().any(|m| {
                    let v = &st.minions[m.index()];
                    v.owner == seat && v.ready() && v.clan() == Some(clan)
                })
        }),
        Requirement::TermsChoseYourReadyClan(clan) => political.is_some_and(|r| {
            r.terms.minions.iter().any(|&m| {
                let v = &st.minions[m.index()];
                v.owner == seat && v.ready() && v.clan() == Some(clan)
            })
        }),
        Requirement::OwnActionCardToAsh => action.is_some_and(|a| {
            a.seat == seat && a.card.is_some() && !a.card_entered_play && !a.card_returned
        }),
        Requirement::OwnActionCardCosts(kind) => action.is_some_and(|a| {
            a.seat == seat
                && a.card.is_some_and(|p| {
                    let blood = blood_cost(st, p.card, p.x);
                    let pool = pool_cost(st, seat, p.card, Target::Minion(a.acting));
                    match kind {
                        CostSource::Line => {
                            blood > a.blood_reduction && st.minions[a.acting.index()].is_vampire()
                        }
                        CostSource::Cache => a.kind == crate::decision::ActionKind::Equip,
                        CostSource::SunsetStrip => mortal_hire(p.card) && (blood > 0 || pool > 0),
                        CostSource::Hackerspace => {
                            a.kind == crate::decision::ActionKind::Equip
                                && st.sect_of(a.acting) == Some(Sect::Anarch)
                        }
                        _ => blood > 0 || pool > 0,
                    }
                })
        }),
        Requirement::PlainEquipFromHandOncePerTurn => action.is_some_and(|a| {
            a.kind == crate::decision::ActionKind::Equip
                && a.card.is_some_and(|p| plain_equipment(p.card))
                && !st
                    .turn_uses
                    .contains(&(AbilitySource::Vampire(a.acting), u8::MAX - 1))
        }),
        Requirement::AttemptingAllyOrYounger => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => {
                st.minions[m.index()].is_ally() || st.younger_by(m, a.acting, seat)
            }
            _ => false,
        }),
        Requirement::AttemptingYoungerVampire => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => {
                st.minions[m.index()].is_vampire() && st.younger_by(m, a.acting, seat)
            }
            _ => false,
        }),
        Requirement::AttemptingCapacityAtLeast(n) => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => st.capacity_by(m, seat).is_some_and(|c| c >= n),
            _ => false,
        }),
        Requirement::ActorNotClan(c) => {
            actor.is_some_and(|m| st.minions[m.index()].clan() != Some(c))
        }
        Requirement::ActorNotBlackHand => actor.is_some_and(|m| !st.is_black_hand(m)),
        Requirement::ActingBlackHand => action.is_some_and(|a| st.is_black_hand(a.acting)),
        Requirement::ActingNotBlackHand => action.is_some_and(|a| !st.is_black_hand(a.acting)),
        Requirement::ActingNotSeraph => action.is_some_and(|a| !st.is_seraph(a.acting)),
        Requirement::AttemptingBlackHand => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => st.is_black_hand(m),
            _ => false,
        }),
        Requirement::ActorBlackHand => actor.is_some_and(|m| st.is_black_hand(m)),
        Requirement::ControlsReadyBlackHand => st
            .minions_of(seat)
            .any(|m| st.minions[m.index()].ready() && st.is_black_hand(m)),
        Requirement::ControlsReadySeraph => st
            .minions_of(seat)
            .any(|m| st.minions[m.index()].ready() && st.is_seraph(m)),
        Requirement::OwnMinionInCombat => {
            combat.is_some_and(|cb| cb.sides.iter().any(|s| s.seat == seat))
        }
        Requirement::AttemptingClan(c) => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => st.clan_or_creed(m) == Some(c),
            _ => false,
        }),
        // Any minion's: in combat the opponent burned it (LSJ 20090922); by
        // the action, not a referendum's or a side effect's (RTR 19960124).
        Requirement::ActorBurnedMinion => actor.is_some_and(|m| {
            combat.is_some_and(|c| c.burned_minions.iter().any(|&(b, _)| b == m))
                || action.is_some_and(|a| {
                    a.acting == m && !a.burned_owners.is_empty() && a.diablerist != Some(m)
                })
        }),
        Requirement::ActorBurnedPreyMinion => actor.is_some_and(|m| {
            let prey = st.prey(st.minions[m.index()].owner);
            combat.is_some_and(|c| c.burned_minions.contains(&(m, prey)))
                || action.is_some_and(|a| a.acting == m && a.burned_owners.contains(&prey))
        }),
        Requirement::AttemptingVampireWithout(d) => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => {
                st.minions[m.index()].is_vampire() && st.discipline_level(m, d) == 0
            }
            _ => false,
        }),
        Requirement::YourAttemptNeedsIntercept => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => {
                st.minions[m.index()].owner == seat && a.stealth(st) > a.intercept_of(st, m)
            }
            _ => false,
        }),
        Requirement::OthersAttemptNeedsIntercept => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => {
                st.minions[m.index()].owner != seat && a.stealth(st) > a.intercept_of(st, m)
            }
            _ => false,
        }),
        Requirement::OtherActingClan(c) => action.is_some_and(|a| {
            actor != Some(a.acting) && st.minions[a.acting.index()].clan() == Some(c)
        }),
        Requirement::OtherActingInfernal => action.is_some_and(|a| {
            actor != Some(a.acting)
                && (st.is_infernal(a.acting) || st.unlocks_as_infernal(a.acting))
        }),
        // Host predicate (`holds_for_host`).
        Requirement::HostUnlocked => true,
        Requirement::OwnTurn => st.active == seat,
        Requirement::ActorMustHunt => actor.is_some_and(|m| st.must_hunt(m)),
        Requirement::DiscardPhaseActionLeft => st.seats[seat.index()].dpa > 0,
        Requirement::NoActionAnnouncedThisTurn => !st.seats[seat.index()].acted_this_turn,
        Requirement::MasterCardPlayed => st.seats[seat.index()].master_played,
        Requirement::AshNotEmpty => !st.seats[seat.index()].ash.is_empty(),
        Requirement::LibraryAtLeast(n) => st.seats[seat.index()].library.len() >= n as usize,
        Requirement::AshAtLeast(n) => {
            let s = &st.seats[seat.index()];
            s.ash.len() + s.ash_crypt.len() >= n as usize
        }
        Requirement::ActorCalledReferendum => {
            political.is_some() && action.zip(actor).is_some_and(|(a, m)| a.acting == m)
        }
        Requirement::NoVotesCast => political.is_some_and(|r| !r.any_cast()),
        Requirement::ReferendumPassed => action.is_some_and(|a| a.referendum_passed),
        Requirement::Younger
        | Requirement::TargetOlder
        | Requirement::AllyOrYounger
        | Requirement::TargetAllyOrCapacityAtMost(_)
        | Requirement::TargetControllerPoolAtMost(_)
        | Requirement::TargetEnteredSinceYourLastMinionPhase
        | Requirement::TargetEnteredInYourLastInfluencePhase
        | Requirement::ActionTargetsHost
        | Requirement::TargetWeapon => true, // target and card predicates
        Requirement::PlayedByVampireCapacityAtMost(n) => {
            played.and_then(|p| p.actor).is_some_and(|m| {
                st.minions[m.index()].is_vampire()
                    && st.capacity_by(m, seat).is_some_and(|c| c <= n)
            })
        }
        Requirement::PlayedByAllyOrCapacityAtMost(n) => {
            played.and_then(|p| p.actor).is_some_and(|m| {
                st.minions[m.index()].is_ally() || st.capacity_by(m, seat).is_some_and(|c| c <= n)
            })
        }
        Requirement::NoOlderReadyUntitled { clan, sect } => actor.is_some_and(|a| {
            st.older_ready_untitled(clan, sect, a, seat)
                .next()
                .is_none()
        }),
        Requirement::PlayedByYoungerVampire => played
            .and_then(|p| p.actor)
            .zip(actor)
            .is_some_and(|(m, a)| st.minions[m.index()].is_vampire() && st.younger_by(m, a, seat)),
        Requirement::OwnAshVampire => !st.seats[seat.index()].ash_crypt.is_empty(),
        Requirement::OwnAshBurnedVampire(superior) => {
            !st.ash_burned_vampires(seat, superior).is_empty()
        }
        // The end-of-round window of the round it burned in: an ally out of
        // play, the opposing side's.
        Requirement::OpposingAllyBurned => combat.zip(side).is_some_and(|(c, i)| {
            let o = &st.minions[c.sides[1 - i].minion.index()];
            o.is_ally() && !o.in_play
        }),
        Requirement::DirectedAtYourMinion => action.is_some_and(|a| {
            a.directed_seats(st).contains(seat)
                && a.target_minions()
                    .any(|m| st.minions[m.index()].owner == seat)
        }),
        Requirement::DirectedAtYourCard(filters) => action.is_some_and(|a| {
            matches!(a.target, Target::Card(c)
                if st.card_controller(c) == seat
                    && filters.iter().any(|f| f.matches(st.cards[c.index()].card)))
        }),
        Requirement::DirectedAtYourAlly => action.is_some_and(|a| {
            a.directed_seat(st) == Some(seat)
                && a.target_minion().is_some_and(|m| {
                    st.minions[m.index()].owner == seat && st.minions[m.index()].is_ally()
                })
        }),
        Requirement::ActingByPredatorOrPrey => {
            action.is_some_and(|a| a.seat == st.predator(seat) || a.seat == st.prey(seat))
        }
        Requirement::OthersMinionPhase
        | Requirement::PlayedStrikeByOpponent
        | Requirement::PlayedStrikeOrGrappleByOpponent
        | Requirement::PlayedByOpposingMinion
        | Requirement::PlayedRestrictsStrikesByOpponent
        | Requirement::PlayedAimFrenzyGrappleByOpponent
        | Requirement::BlockedByOlder
        | Requirement::BlocksOlder
        | Requirement::ReferendumFailed
        | Requirement::OwnMinionPhase
        | Requirement::ControlledByPredator
        | Requirement::ControlledByActive
        | Requirement::CardLocked => true, // engine / card / target predicates
        Requirement::NotInCombat => combat.is_none(),
        Requirement::NotContinued => action.is_some_and(|a| !a.continued),
        Requirement::NotInDamageResolution => combat.is_none_or(|c| {
            !matches!(
                c.step,
                crate::combat::Step::Prevent(_)
                    | crate::combat::Step::Allocate { .. }
                    | crate::combat::Step::AllocateSource { .. }
            )
        }),
        Requirement::ActorNotBlocking => action.is_none_or(|a| {
            a.blocker != actor && !matches!(a.state, ActionState::Attempt(m) if Some(m) == actor)
        }),
        Requirement::LifeAtLeast(_)
        | Requirement::TargetAlly
        | Requirement::TargetOnVampire
        | Requirement::TargetOnOwnMinion
        | Requirement::TargetCardPlayedByOther
        | Requirement::SuperiorVersion
        | Requirement::BasicVersion
        | Requirement::TargetAllyHurt
        | Requirement::TargetTakesContracts
        | Requirement::TargetHasYourCorruption
        | Requirement::TargetOtherCopy
        | Requirement::TargetUnlocked => true, // card / target predicates
        Requirement::RecruitingTzimisceGhoul => action.is_some_and(|a| {
            a.kind == ActionKind::Recruit
                && a.card.is_some_and(|p| {
                    let c = &LIBRARY[p.card as usize];
                    c.labels.contains(&crate::library::Label::Ghoul)
                        && c.clans.contains(&"Tzimisce")
                })
        }),
        Requirement::NoBlockAttempt => action.is_some_and(|a| {
            matches!(
                a.state,
                ActionState::AsAnnounced | ActionState::Open | ActionState::Declined
            )
        }),
        // Non-derivative hunting grounds the other Methuselahs control
        // (Poacher's Hunting Ground's X), read live.
        Requirement::OthersHuntingGroundsAtLeast(n) => {
            let count = st
                .cards
                .iter()
                .enumerate()
                .filter(|&(j, c)| {
                    let l = LIBRARY[c.card as usize].labels;
                    c.present()
                        && l.contains(&crate::library::Label::HuntingGround)
                        && !l.contains(&crate::library::Label::Derivative)
                        && st.card_controller(CardId(j as u32)) != seat
                })
                .count();
            count >= n as usize
        }
        Requirement::FewerCopiesThanReadyOfClan(_) | Requirement::InMasterPhase => true, // engine-checked
        Requirement::ActiveIsPrey => st.active != seat && st.active == st.prey(seat),
        Requirement::ActionNotFailed => action.is_some_and(|a| !a.failed),
        Requirement::PlayedNotOutOfTurn => {
            played.is_some_and(|p| lookup(p.card).is_none_or(|d| !d.out_of_turn))
        }
        // The last block's minion (the blocker field clears with its combat).
        Requirement::BlockerNotReady => action
            .and_then(|a| a.blocked_by.last().copied())
            .is_some_and(|b| !st.minions[b.index()].ready()),
        Requirement::NoCombatEndsStrike => combat.is_none_or(|c| {
            !c.sides
                .iter()
                .any(|s| matches!(s.strike, Some(StrikeKind::CombatEnds { .. })))
        }),
        Requirement::OtherNonHuntAction => true, // printed predicate
        Requirement::NoSlaveTookCombat => action.is_some_and(|a| a.slave.is_none()),
        Requirement::SameCircleAsActing => {
            matches!((actor, action), (Some(m), Some(a)) if st.same_circle_for_play(m, a.acting))
        }
        Requirement::ControlsOtherReadyUnlockedSameCircle => actor.is_some_and(|m| {
            st.minions_of(seat).any(|o| {
                let v = &st.minions[o.index()];
                v.ready() && !v.locked && st.same_circle_for_play(m, o)
            })
        }),
        Requirement::ActorGainedBlood => actor.is_some_and(|m| st.blood_gained(m) > 0),
        Requirement::ActorUnprintedCircle => actor.is_some_and(|m| {
            let v = &st.minions[m.index()];
            v.clan() == Some("Blood Brother")
                && v.crypt_card()
                    .is_some_and(|c| crate::crypt::CRYPT[c as usize].circle.is_none())
        }),
        Requirement::SameCircleAsOwnCombatant { sanguinus } => match (actor, combat) {
            (Some(m), Some(c)) => c.side_of(seat).is_some_and(|i| {
                let o = c.sides[i].minion;
                if sanguinus {
                    st.same_circle_for_play(m, o)
                } else {
                    st.same_circle(m, o)
                }
            }),
            _ => false,
        },
        // Read against the card (`Engine::ability_options`).
        Requirement::ActorOfByCircle
        | Requirement::ActorIsBysMaster
        | Requirement::ActingIsHost
        | Requirement::HostNotInvolved
        | Requirement::ActingOfHostsMasterClan
        | Requirement::OwnActingAllyOrYounger => true,
        Requirement::BoonableBleed => action.is_some_and(|a| {
            a.kind == ActionKind::Bleed
                && a.bleed_target.is_some_and(|t| t != seat)
                && st.minions[a.acting.index()].owner != seat
                && a.bleed_payer.is_none()
                && a.bleed_amount(st) >= 1
        }),
        Requirement::BleedAtYouUnboonedForOne => action.is_some_and(|a| {
            a.kind == ActionKind::Bleed
                && a.bleed_target == Some(seat)
                && a.bleed_payer.is_none()
                && a.bleed_amount(st) >= 1
        }),
        Requirement::OwnCombatantsOpponentReady => combat.is_some_and(|c| {
            c.side_of(seat)
                .is_some_and(|i| st.minions[c.sides[1 - i].minion.index()].ready())
        }),
        Requirement::CombatFromBlock => {
            action.is_some_and(|a| !a.blocked_by.is_empty() && !a.resolved)
        }
        Requirement::CombatFromSuccess => {
            combat.is_some() && action.is_some_and(|a| a.blocked_by.is_empty())
        }
        Requirement::TargetNotOfPredator => true, // target predicate
        Requirement::ActorNotSterile => actor.is_none_or(|m| !st.is_sterile(m)),
        Requirement::PreyUncontrolledHasBlood => {
            let prey = st.prey(seat);
            prey != seat
                && st.seats[prey.index()]
                    .uncontrolled
                    .iter()
                    .any(|u| u.counters > 0)
        }
        Requirement::BleedTargetAshAtLeast(n) => action.is_some_and(|a| {
            a.kind == ActionKind::Bleed
                && a.directed_seat(st).is_some_and(|t| {
                    let s = &st.seats[t.index()];
                    s.ash.len() + s.ash_crypt.len() >= n as usize
                })
        }),
        Requirement::TargetOnLockedMinion
        | Requirement::TargetOnMinionNotOfSect(_)
        | Requirement::TargetOnPredatorOrPreyMinion
        | Requirement::TargetCardNot(_)
        | Requirement::TargetCardWithout(_)
        | Requirement::TargetDisciplineNotSuperiorOnActor
        | Requirement::TargetOnSameCircle { .. } => true, // card-target predicates
        Requirement::OpponentSubtype(l) => combat
            .zip(side)
            .is_some_and(|(c, i)| st.ally_of(c.sides[1 - i].minion, &[l])),
        Requirement::OpponentVampire => combat
            .zip(side)
            .is_some_and(|(c, i)| !st.minions[c.sides[1 - i].minion.index()].is_ally()),
        Requirement::OpponentMonster => combat
            .zip(side)
            .is_some_and(|(c, i)| st.is_monster(c.sides[1 - i].minion)),
        Requirement::OpponentNotInfernal => combat
            .zip(side)
            .is_some_and(|(c, i)| !st.is_infernal(c.sides[1 - i].minion)),
        Requirement::OpponentCapacityAtMost(n) => combat.zip(side).is_some_and(|(c, i)| {
            let o = c.sides[1 - i].minion;
            !st.minions[o.index()].is_vampire() || st.capacity_by(o, seat).is_some_and(|k| k <= n)
        }),
        Requirement::CombatMonsterVsMortal => combat.is_some_and(|c| {
            let [a, b] = [c.sides[0].minion, c.sides[1].minion];
            (st.is_monster(a) && st.is_mortal(b)) || (st.is_mortal(a) && st.is_monster(b))
        }),
        Requirement::CombatBothMonsters => {
            combat.is_some_and(|c| c.sides.iter().all(|s| st.is_monster(s.minion)))
        }
        Requirement::ActorMonster => actor.is_some_and(|m| st.is_monster(m)),
        Requirement::ActorHolds(h) => actor.is_some_and(|m| st.holds_a(m, h)),
        Requirement::PlayedTargetsActor => played
            .zip(actor)
            .is_some_and(|(p, a)| p.target == Target::Minion(a)),
        Requirement::PlayedTakesControlOfActor => played.zip(actor).is_some_and(|(p, a)| {
            p.target == Target::Minion(a)
                && lookup(p.card).is_some_and(|d| {
                    d.modes[p.mode as usize]
                        .effects
                        .iter()
                        .any(|e| e.takes_control())
                })
        }),
        Requirement::ActionTargetsImbued => action.is_some_and(|a| {
            matches!(a.target, Target::Minion(t) if st.minions[t.index()].is_imbued())
                || a.torpor_target
                    .is_some_and(|t| st.minions[t.index()].is_imbued())
        }),
        Requirement::OpponentLacksDiscipline(d) => combat
            .zip(side)
            .is_some_and(|(c, i)| st.discipline_level(c.sides[1 - i].minion, d) == 0),
        Requirement::OpponentAlly => combat
            .zip(side)
            .is_some_and(|(c, i)| st.minions[c.sides[1 - i].minion.index()].is_ally()),
        Requirement::OpponentNotSubtype(l) => combat
            .zip(side)
            .is_some_and(|(c, i)| !st.ally_of(c.sides[1 - i].minion, &[l])),
        Requirement::OpponentSectIn(sects) => combat
            .zip(side)
            .is_some_and(|(c, i)| st.sect_in(c.sides[1 - i].minion, sects)),
        Requirement::OpponentHoldsNoCard(reqs) => combat.zip(side).is_some_and(|(c, i)| {
            let o = c.sides[1 - i].minion;
            !(0..st.cards.len()).map(|k| CardId(k as u32)).any(|id| {
                let card = &st.cards[id.index()];
                card.present()
                    && card.host == Some(o)
                    && reqs
                        .iter()
                        .all(|&r| holds_for_card_target(r, st, seat, actor, id))
            })
        }),
        Requirement::OpponentNonMortalAlly => combat.zip(side).is_some_and(|(c, i)| {
            let o = c.sides[1 - i].minion;
            st.minions[o.index()].is_ally() && !st.is_mortal(o)
        }),
        Requirement::OpponentVampireNotOlder => combat.zip(side).is_some_and(|(c, i)| {
            let (me, opp) = (c.sides[i].minion, c.sides[1 - i].minion);
            !st.minions[opp.index()].is_ally() && !st.younger_by(me, opp, seat)
        }),
        Requirement::ActingCombatantYourVampire => combat.is_some_and(|c| {
            let v = &st.minions[c.sides[0].minion.index()];
            v.is_vampire() && v.owner == seat
        }),
        Requirement::CombatantFacesAggravatedStrike => combat.is_some_and(|c| {
            (0..2).any(|i| {
                st.minions[c.sides[i].minion.index()].is_vampire()
                    && c.aggravated_strike_on(st, action, i)
            })
        }),
        Requirement::OpponentNotReady => combat
            .zip(side)
            .is_some_and(|(c, i)| !st.minions[c.sides[1 - i].minion.index()].ready()),
        Requirement::OpponentPresent => combat
            .zip(side)
            .is_some_and(|(c, i)| st.minions[c.sides[1 - i].minion.index()].present()),
        Requirement::ActorNotVotingFor => {
            political.zip(actor).is_some_and(|(r, m)| !r.voting_for(m))
        }
        Requirement::ActorWithout(h) => actor.is_some_and(|m| !st.holds_a(m, h)),
        Requirement::ActorHoldsFewer(h, n) => actor.is_some_and(|m| st.holds_count(m, h) < n),
        Requirement::ActorHoldsUnlocked(h, n) => {
            actor.is_some_and(|m| st.holds_unlocked(m, h) >= n)
        }
        Requirement::ActorHoldsAtLeast(h, n) => actor.is_some_and(|m| st.holds_count(m, h) >= n),
        Requirement::VictoryPointsAtLeast(n) => st.seats[seat.index()].half_vps >= 2 * n as u32,
        Requirement::GainedVpThisAction => {
            action.is_some()
                && st.seats[seat.index()].half_vps
                    >= st
                        .action_vps
                        .get(seat.index())
                        .map_or(u32::MAX, |&v| v.saturating_add(2))
        }
        Requirement::ActorClan(c) => actor.and_then(|m| st.minions[m.index()].clan()) == Some(c),
        Requirement::YouControlThisCard
        | Requirement::ActorMayEquipStored(_)
        | Requirement::OthersControlThisCard
        | Requirement::CardControllersTurn
        | Requirement::TargetCapacityBelowCounters
        | Requirement::TargetBloodAtMostCounters => true, // engine-checked
        Requirement::TargetOnPreyMinion => true, // card-target predicate
        Requirement::TargetActedSuccessfully | Requirement::SlotWithout(_) => true, // target predicates
        Requirement::EnteredBeforeThisTurn
        | Requirement::EnteredBeforeThisAction
        | Requirement::StoreEmpty
        | Requirement::StoreNotEmpty
        | Requirement::CardMode(_) => true, // card predicate
        Requirement::ControlledByPredatorOrPrey => true, // target predicate
        Requirement::HostClan(_) | Requirement::HostDiscipline(_) => true, // host predicates
        Requirement::ThisVampireReady
        | Requirement::InOwnMasterPhase
        | Requirement::ActorGraftedAlly
        | Requirement::OpponentClan(_)
        | Requirement::OpponentNotClan(_)
        | Requirement::OpponentNamed(_)
        | Requirement::ActorOfOtherMethuselah
        | Requirement::DirectedAtYourClan(_)
        | Requirement::OpponentInfernal
        | Requirement::OpponentRedList
        | Requirement::TargetSeatControlsNoReadySect(_)
        | Requirement::TargetSeatControlsReadyAlly
        | Requirement::TargetSeatControlsReadyMinion
        | Requirement::TargetSeatNoReadyCapacityAbove(_)
        | Requirement::SourceInTorpor
        | Requirement::ActorOfSourcesPredator
        | Requirement::TargetSeatControlsLocation
        | Requirement::PreyControlsReadyTitled(_)
        | Requirement::SourceCalledReferendum
        | Requirement::OtherCalledReferendum
        | Requirement::ReferendumGrantsTitle(_)
        | Requirement::OpponentMarkedForBlocks
        | Requirement::ControlsNoReadyTitled(_)
        | Requirement::OpponentControllerHasGehenna
        | Requirement::TargetSeatControlsReadyClan { .. }
        | Requirement::TargetSeatControlsReadySect { .. }
        | Requirement::TargetSeatControlsReadyTitled(_)
        | Requirement::TargetIsSourceController
        | Requirement::SourceTitleIn(_)
        | Requirement::SourceSectIn(_)
        | Requirement::TargetSeatHasEdge
        | Requirement::OpponentYounger
        | Requirement::OpponentOlder
        | Requirement::OpponentAllyOrYounger
        | Requirement::OpponentTitled
        | Requirement::ActorCorruptedBySource
        | Requirement::ControlsLocation
        | Requirement::ControlsNoLocation
        | Requirement::ControlsVampireNamed(_)
        | Requirement::ControlsNoReadyOlderOfClan(_)
        | Requirement::ControlsOfClan(..)
        | Requirement::NoReadyNamed(_)
        | Requirement::PlayedAsReaction
        | Requirement::PlayedInStrikeMode
        | Requirement::DiablerizedOlder
        | Requirement::SourceActing
        | Requirement::PlayedAsCombat
        | Requirement::ActorClaimsTitleIn(_) => true, // printed-modifier predicates
        // An actor with no title (Powerbase: Barranquilla's untitled
        // burner); the printed predicate's reading too.
        Requirement::OwnUncontrolledVampire => st.seats[seat.index()]
            .uncontrolled
            .iter()
            .any(|u| u.card == crate::state::CREATED || crate::state::vampire_card(u.card)),
        Requirement::ReadyNamed(name) => st.ready_named(name, None),
        // "If /Viri/ is in your ready region" (Zizi's ability).
        Requirement::OwnReadyNamed(name) => st.ready_named(name, Some(seat)),
        Requirement::ActorPassedReferendumThisTurn => {
            actor.is_some_and(|m| st.minions[m.index()].referendum_passed_turn)
        }
        Requirement::ActorUnmergedAdvanced => actor.is_some_and(|m| {
            let v = &st.minions[m.index()];
            v.merged.is_none() && v.vampire().is_some_and(|c| c.adv)
        }),
        Requirement::ActorUntitled => {
            actor.is_none_or(|m| st.title_of(m) == crate::crypt::Title::None)
        }
        Requirement::ActorFollowsNoPath => {
            actor.is_none_or(|m| st.minions[m.index()].path().is_none())
        }
        Requirement::ActorNotRedList => actor.is_none_or(|m| !st.is_red_list(m)),
        Requirement::ActorSlave => actor.is_some_and(|m| st.slave_clan(m).is_some()),
        Requirement::ActorNotSlave => actor.is_some_and(|m| st.slave_clan(m).is_none()),
        Requirement::ActorCanDiablerize => actor.is_some_and(|m| !st.cannot_diablerize(m)),
        Requirement::DiablerieStrikeRound => combat.is_some_and(|c| {
            c.pair == 0
                && actor
                    .and_then(|m| c.side_of_minion(m))
                    .is_some_and(|i| c.sides[i].diablerie_round == Some(c.round))
        }),
        Requirement::CombatOpponentYounger => combat.is_some_and(|c| {
            actor.is_some_and(|m| c.opponent_of(m).is_some_and(|o| st.younger_by(o, m, seat)))
        }),
        Requirement::ActorCryptVampire => actor.is_some_and(|m| {
            matches!(
                st.minions[m.index()].kind,
                crate::state::MinionKind::Vampire(_)
            )
        }),
        Requirement::DiablerizedCapacityAtLeast(n) => action.is_some_and(|a| match a.state {
            ActionState::AfterDiablerie(d, _) => Some(d) == actor && st.victim_capacity >= n,
            _ => false,
        }),
        Requirement::ActorUnmergedBase => actor.is_some_and(|m| {
            let v = &st.minions[m.index()];
            v.merged.is_none() && v.vampire().is_some_and(|c| !c.adv)
        }),
        Requirement::DirectedAtOwnClan(clan) => action.is_some_and(|a| {
            a.target_minion().is_some_and(|t| {
                let v = &st.minions[t.index()];
                v.owner == seat && v.clan() == Some(clan) && a.seat != seat
            })
        }),
        Requirement::ActingIsAlly => action.is_some_and(|a| st.minions[a.acting.index()].is_ally()),
        Requirement::ControlsOtherUnlockedAnarch => st
            .ready_unlocked(seat)
            .into_iter()
            .any(|m| Some(m) != actor && st.sect_of(m) == Some(Sect::Anarch)),
        Requirement::HandStrikeInflicted => combat.is_some_and(|c| {
            actor
                .and_then(|m| c.side_of_minion(m))
                .is_some_and(|i| c.sides[i].hand_landed)
        }),
        Requirement::ActorFoughtItsContractsMinion => match (actor, action) {
            (Some(m), Some(a)) => a.last_combat.is_some_and(|pair| {
                pair.contains(&m)
                    && st.minions[m.index()].ready()
                    && pair.iter().any(|&o| {
                        o != m
                            && !st.minions[o.index()].ready()
                            && a.last_contracts
                                .iter()
                                .any(|(h, _, chosen)| *h == o && chosen.contains(&m))
                    })
            }),
            _ => false,
        },
        Requirement::ControlsLockedMinion => {
            st.minions_of(seat).any(|m| st.minions[m.index()].locked)
        }
        Requirement::BlockerOwn => action.is_some_and(|a| {
            a.blocker.is_some_and(|b| {
                st.minions[b.index()].owner == seat
                    && a.last_combat
                        .is_some_and(|c| c.contains(&b) && c.contains(&a.acting))
            })
        }),
        Requirement::BlockerReadyAfterItsCombat => action.is_some_and(|a| {
            a.blocker.is_some_and(|b| {
                st.minions[b.index()].ready()
                    && a.last_combat
                        .is_some_and(|c| c.contains(&b) && c.contains(&a.acting))
            })
        }),
        Requirement::ActorNotBlocker => action
            .and_then(|a| a.blocker)
            .is_none_or(|b| actor != Some(b)),
        Requirement::BlockerYours => action
            .and_then(|a| a.blocker)
            .is_some_and(|b| st.minions[b.index()].owner == seat),
        Requirement::BlockerOwnAnarch => action.and_then(|a| a.blocker).is_some_and(|b| {
            st.minions[b.index()].owner == seat && st.sect_of(b) == Some(Sect::Anarch)
        }),
        Requirement::BlockerOlderNonAnarchCrypts => {
            action.and_then(|a| a.blocker).is_some_and(|b| {
                let v = &st.minions[b.index()];
                v.is_vampire()
                    && st.sect_of(b) != Some(Sect::Anarch)
                    && actor.is_some_and(|m| st.younger_by(m, b, seat))
                    && !st.seats[v.owner.index()].crypt.is_empty()
                    && !st.seats[seat.index()].crypt.is_empty()
            })
        }
        Requirement::BlockerTitleIn(classes) => action.and_then(|a| a.blocker).is_some_and(|b| {
            let t = st.title_of(b);
            classes.iter().any(|c| c.matches(t))
        }),
        Requirement::AnyOf(alternatives) => alternatives.iter().any(|reqs| {
            reqs.iter().all(|&r| {
                holds(
                    r,
                    st,
                    seat,
                    actor,
                    played,
                    action,
                    combat,
                    referendum,
                    waives_needed,
                )
            })
        }),
    }
}

/// Evaluate a requirement against a candidate minion target.
pub fn holds_for_minion(
    req: Requirement,
    st: &GameState,
    seat: SeatId,
    referendum: Option<&ReferendumCtx>,
    acting: Option<MinionId>,
    m: MinionId,
    action: Option<&ActionCtx>,
) -> bool {
    let v = &st.minions[m.index()];
    match req {
        Requirement::ControlledByDirectedSeat => action
            .and_then(|a| a.directed_seat(st))
            .is_some_and(|t| v.owner == t),
        Requirement::DirectedSeatHuntedSinceYourLastTurn => {
            action.and_then(|a| a.directed_seat(st)).is_some_and(|t| {
                st.minions_of(t).any(|o| {
                    st.minions[o.index()]
                        .hunted_turn
                        .is_some_and(|h| h > st.seats[seat.index()].previous_turn)
                })
            })
        }
        Requirement::Controlled => v.owner == seat && v.present(),
        Requirement::NotControlled => v.owner != seat,
        Requirement::TargetSameCircle { sanguinus } => acting.is_some_and(|a| {
            if sanguinus {
                st.same_circle_for_play(a, m)
            } else {
                st.same_circle(a, m)
            }
        }),
        Requirement::TargetLacksActorSuperior => acting.is_some_and(|a| {
            crate::state::disciplines()
                .into_iter()
                .any(|d| st.discipline_level(a, d) == 2 && st.discipline_level(m, d) < 2)
        }),
        Requirement::TargetLocked => v.locked,
        Requirement::TargetVampire => !v.is_ally(),
        Requirement::TargetImbued => v.is_imbued(),
        Requirement::TargetMonster => st.is_monster(m),
        Requirement::TargetConvictionUnplayed => !v.conviction_played,
        Requirement::TargetFollowsPath(path) => st.follows(m, path),
        Requirement::TargetAllyOf(labels) => st.ally_of(m, labels),
        // An imbued is a mortal ally (rulebook "Appendix: Imbued Rules" 1).
        Requirement::TargetAllyNotOf(labels) if v.is_imbued() => {
            !labels.contains(&crate::library::Label::Mortal)
        }
        Requirement::TargetAllyNotOf(labels) => v.kind.ally_card().is_some_and(|c| {
            !labels
                .iter()
                .any(|l| LIBRARY[c as usize].labels.contains(l))
        }),
        Requirement::TargetOtherCopyControlled => {
            v.owner == seat
                && acting.is_some_and(|a| a != m && st.minions[a.index()].name() == v.name())
        }
        Requirement::TargetCombatant => st.combatants.is_some_and(|p| p.contains(&m)),
        Requirement::TargetNotOfOpposingController => acting
            .zip(st.combatants)
            .and_then(|(a, p)| {
                p.iter()
                    .copied()
                    .find(|&o| o != a)
                    .filter(|_| p.contains(&a))
            })
            .is_some_and(|o| v.owner != st.minions[o.index()].owner),
        Requirement::TargetHuntedToBurn => st.hunt_burn.as_ref().is_some_and(|h| h.minion == m),
        Requirement::TargetControlAboutToChange => st
            .control_change
            .is_some_and(|c| c.minion == m && c.from == seat),
        Requirement::TargetAboutToEnterCombat => action.is_some_and(|a| {
            let blocked = matches!(a.state, crate::actions::ActionState::Blocked(b)
                if a.acting == m || b == m);
            let fights = |x: MinionId| a.about_to_fight(st, x);
            blocked
                || fights(m)
                || (a.acting == m && a.combat_target(st).is_some_and(fights))
                || (a.acting == m && a.queued_opponent(m).is_some_and(fights))
        }),
        // The strike is read by the play's options (`play_options`), which
        // see the combat; here, without it, a combatant.
        Requirement::TargetFacesAggravatedStrike => st.combatants.is_some_and(|p| p.contains(&m)),
        Requirement::TargetDiablerieSinceYourLastTurn => v
            .diablerie_turn
            .is_some_and(|t| t > st.seats[seat.index()].previous_turn),
        Requirement::TargetBledSinceYourLastTurn { min, against_you } => {
            v.bleeds.iter().any(|&(t, bled, n)| {
                t > st.seats[seat.index()].previous_turn
                    && n >= min
                    && (!against_you || bled == seat)
            })
        }
        Requirement::TargetUntitled => st.title_of(m) == crate::crypt::Title::None,
        Requirement::TargetInfernal => st.is_infernal(m),
        Requirement::TargetNotInfernal => !st.is_infernal(m),
        Requirement::TargetSlaveOfActorClan => acting.is_some_and(|a| {
            let clan = st.minions[a.index()].clan();
            clan.is_some() && st.slave_clan(m) == clan
        }),
        Requirement::BloodAtLeast(n) => v.blood >= n,
        Requirement::BloodAtMost(n) => v.blood <= n,
        Requirement::HasCastVotes => referendum.is_some_and(|r| r.has_cast(m)),
        Requirement::NotAbstaining => !referendum.is_some_and(|r| r.abstaining.contains(&m)),
        Requirement::ActorClan(c) => v.clan() == Some(c),
        Requirement::ActorBlackHand => st.is_black_hand(m),
        Requirement::ActorNotBlackHand => !st.is_black_hand(m),
        Requirement::ActorSlave => st.slave_clan(m).is_some(),
        Requirement::TargetActedSuccessfully => st.turn_successes.contains(&m),
        Requirement::TargetHasCorruption => !v.corruption.is_empty(),
        Requirement::NotFirstChosen => {
            referendum.is_none_or(|r| r.terms.term == 0 || r.terms.minions.first() != Some(&m))
        }
        Requirement::ActorTitleNotIn(classes) => !classes.iter().any(|c| c.matches(st.title_of(m))),
        Requirement::ActorReady => v.ready(),
        Requirement::ActorTitleIn(classes) => classes.iter().any(|c| c.matches(st.title_of(m))),
        Requirement::ActorSectIn(sects) => st.sect_of(m).is_some_and(|s| sects.contains(&s)),
        Requirement::NotHuntingGroundFed => !v.hunting_ground_fed,
        Requirement::Younger => acting.is_some_and(|a| st.younger_by(m, a, seat)),
        Requirement::TargetOlder => acting.is_some_and(|a| st.younger_by(a, m, seat)),
        Requirement::TargetAllyHurt => st.ally_body(m).is_some_and(|b| v.blood < b.life),
        Requirement::TargetHasYourCorruption => v.corruption_of(seat) > 0,
        Requirement::TargetTakesContracts => {
            st.printed_mod(m, Stat::NoContractsOn, crate::state::PrintedCtx::default()) == 0
        }
        Requirement::TargetAllyCard(f) if v.is_imbued() => {
            f == CardFilter::HasLabel(crate::library::Label::Mortal)
        }
        Requirement::TargetAllyCard(f) => {
            matches!(v.kind, crate::state::MinionKind::Ally { card, .. } if f.matches(card))
        }
        Requirement::TargetUnlocked => !v.locked,
        Requirement::TargetAlly => v.is_ally(),
        Requirement::ControlledByPredator => v.owner == st.predator(seat),
        Requirement::ControlledByActive => v.owner == st.active,
        Requirement::ControlledByPredatorOrPrey => {
            v.owner != seat && (v.owner == st.predator(seat) || v.owner == st.prey(seat))
        }
        Requirement::TargetOtherCopy => {
            acting.is_some_and(|a| a != m && st.minions[a.index()].name() == v.name())
        }
        Requirement::AllyOrYounger => {
            v.is_ally() || acting.is_some_and(|a| st.younger_by(m, a, seat))
        }
        Requirement::TargetControllerPoolAtMost(n) => st.seats[v.owner.index()].pool <= n,
        Requirement::TargetAllyOrCapacityAtMost(n) => {
            v.is_ally() || st.capacity_by(m, seat).is_some_and(|c| c <= n)
        }
        Requirement::TargetEnteredSinceYourLastMinionPhase => {
            v.entered_at > st.seats[seat.index()].minion_phase_end
        }
        Requirement::TargetEnteredInYourLastInfluencePhase => {
            let me = &st.seats[seat.index()];
            v.entered_at > me.minion_phase_end && v.entered_at < me.influence_end
        }
        Requirement::TargetClan(c) => v.clan() == Some(c),
        Requirement::TargetNotOfPredator => {
            v.owner != st.predator(seat) || st.predator(seat) == seat
        }
        // One of the alternatives ("another ready Tremere or Tremere
        // antitribu", Lord Tremere).
        Requirement::AnyOf(alternatives) => alternatives.iter().any(|reqs| {
            reqs.iter()
                .all(|&r| holds_for_minion(r, st, seat, referendum, acting, m, action))
        }),
        // "Of the same clan as this acting vampire" (My Kin Against the
        // World).
        Requirement::SameClanAsActor => {
            acting.is_some_and(|a| v.clan().is_some() && st.minions[a.index()].clan() == v.clan())
        }
        Requirement::ActorAndTargetSect(s) => {
            acting.is_some_and(|a| st.sect_of(a) == Some(s)) && st.sect_of(m) == Some(s)
        }
        Requirement::TargetWithout(h) => !st.holds_a(m, h),
        Requirement::OtherThanActor => acting != Some(m),
        Requirement::TargetCorrupted => !v.corruption.is_empty(),
        Requirement::TargetNotSuperior(d) => st.discipline_level(m, d) < 2,
        // A hostage cannot be moved to the ready region (Carver's Meat
        // Packing and Storage): no effect can target it to.
        Requirement::TargetInTorpor => v.in_torpor && v.hostage == 0,
        Requirement::TargetIncapacitated => v.incapacitated,
        Requirement::TargetConvictionAtLeast(n) => st.convictions(m).len() >= n as usize,
        Requirement::TargetHostage => v.hostage > 0,
        Requirement::ControlledByPrey => v.owner == st.prey(seat),
        Requirement::TargetClanNotIn(clans) => v.clan().is_some_and(|c| !clans.contains(&c)),
        Requirement::TargetNotOfClan(clan) => v.clan() != Some(clan),
        Requirement::TargetSectNot(sect) => v.is_vampire() && st.sect_of(m) != Some(sect),
        Requirement::TargetRedList => st.is_red_list(m),
        Requirement::TargetNotBlackHand => !st.is_black_hand(m),
        Requirement::TargetBlackHand => st.is_black_hand(m),
        Requirement::TargetUnmerged => st.minions[m.index()].merged.is_none(),
        Requirement::TargetNotRedList => !st.is_red_list(m),
        Requirement::TargetNotBlocker => action.and_then(|a| a.blocker) != Some(m),
        Requirement::TargetAtActionsEnd => action.is_some_and(|a| {
            let owner = v.owner;
            let directed = a.directed_seats(st);
            if directed.is_empty() {
                owner == st.prey(a.seat) || owner == st.predator(a.seat)
            } else {
                directed.contains(owner)
            }
        }),
        Requirement::TargetCastVotes => st.cast_votes_lately(m) > 0,
        Requirement::TargetNotActing => action.is_none_or(|a| a.acting != m),
        Requirement::TargetNotAttempting => {
            action.is_none_or(|a| a.state != ActionState::Attempt(m))
        }
        Requirement::TargetCapacityAtLeast(n) => st.capacity_by(m, seat).is_some_and(|c| c >= n),
        Requirement::TargetCapacityAtMost(n) => st.capacity_by(m, seat).is_some_and(|c| c <= n),
        Requirement::TargetCapacityAtMostActorTimes(n) => {
            acting.is_some_and(
                |a| match (st.capacity_by(m, seat), st.capacity_by(a, seat)) {
                    (Some(t), Some(own)) => t as u16 <= n as u16 * own as u16,
                    _ => false,
                },
            )
        }
        Requirement::TargetHoldsAtLeast(h, n) => st.holds_count(m, h) >= n,
        Requirement::TargetCapacityAtMostTwiceHeld(h) => acting.is_some_and(|a| {
            st.capacity_or_cost_by(m, seat)
                .is_some_and(|c| c <= 2 * st.holds_unlocked(a, h))
        }),
        Requirement::NotGehennaImmune => !st.gehenna_immune(m),
        Requirement::TargetDiscipline(d) => st.has_discipline(m, d),
        Requirement::TargetMithraic => st.is_mithraic(m),
        Requirement::TargetNotNamed(name) => v.name() != name,
        Requirement::TargetAnarch => st.sect_of(m) == Some(Sect::Anarch),
        Requirement::TargetSect(s) => st.sect_of(m) == Some(s),
        Requirement::TargetTitleIn(classes) => {
            let t = st.title_of(m);
            classes.iter().any(|c| c.matches(t))
        }
        Requirement::TargetTitle(t) => st.title_of(m) == t,
        Requirement::TargetTitleContested => v.title_contested,
        Requirement::TargetSubtypeIn(labels) => st.ally_of(m, labels),
        Requirement::TargetSlave => st.slave_clan(m).is_some(),
        Requirement::BleedTargetPoolAtMost(n) => action
            .and_then(|a| a.bleed_target)
            .is_some_and(|t| st.seats[t.index()].pool <= n as i32),
        _ => true,
    }
}

/// The slots of `seat`'s uncontrolled region meeting `filter`, read
/// against `chooser` (`Terms::OwnUncontrolled`).
pub fn uncontrolled_picks(
    st: &GameState,
    seat: SeatId,
    chooser: MinionId,
    filter: &[Requirement],
) -> Vec<u8> {
    (0..st.seats[seat.index()].uncontrolled.len())
        .filter(|&i| {
            filter
                .iter()
                .all(|&r| holds_for_uncontrolled(r, st, seat, Some(chooser), i))
        })
        .map(|i| i as u8)
        .collect()
}

/// Evaluate a requirement against the vampire in slot `i` of `seat`'s
/// uncontrolled region (Govern the Unaligned [DOM], Thing).
pub fn holds_for_uncontrolled(
    req: Requirement,
    st: &GameState,
    seat: SeatId,
    acting: Option<MinionId>,
    i: usize,
) -> bool {
    if i >= st.seats[seat.index()].uncontrolled.len() {
        return false;
    }
    let (capacity, clan) = st.slot_vampire(seat, i);
    match req {
        Requirement::Younger => acting
            .and_then(|a| st.capacity_of(a))
            .is_some_and(|c| capacity < c),
        Requirement::TargetClan(c) => clan == c,
        Requirement::TargetSect(s) => st.slot_sect(seat, i) == Some(s),
        Requirement::TargetCirclelessBloodBrother => {
            let slot = st.seats[seat.index()].uncontrolled[i].card;
            slot != crate::state::CREATED
                && clan == "Blood Brother"
                && crate::crypt::CRYPT[slot as usize].circle.is_none()
        }
        // Its printed circle: an uncontrolled New Blood has chosen none.
        Requirement::TargetSameCircle { .. } => acting.is_some_and(|a| {
            let slot = st.seats[seat.index()].uncontrolled[i].card;
            slot != crate::state::CREATED
                && st.minions[a.index()].clan() == Some("Blood Brother")
                && st.minions[a.index()].circle().is_some()
                && crate::crypt::CRYPT[slot as usize].circle == st.minions[a.index()].circle()
        }),
        Requirement::SameClanAsActor => {
            acting.is_some_and(|a| st.minions[a.index()].clan() == Some(clan))
        }
        Requirement::TargetCapacityAtLeast(n) => capacity >= n,
        // Gated where the targets are listed (`uncontrolled_target_qualifies`).
        Requirement::TargetImbued => true,
        // Printed on the card in the uncontrolled region (Unholy
        // Sacrament's "titled Sabbat vampire").
        Requirement::ActorSectIn(sects) => {
            st.slot_sect(seat, i).is_some_and(|s| sects.contains(&s))
        }
        Requirement::TargetTitleIn(classes) => {
            let title = st.slot_title(seat, i);
            classes.iter().any(|c| c.matches(title))
        }
        Requirement::TargetTitle(t) => st.slot_title(seat, i) == t,
        Requirement::ActorNotBlackHand => {
            let card = st.seats[seat.index()].uncontrolled[i].card;
            card == crate::state::CREATED || !crate::crypt::CRYPT[card as usize].black_hand
        }
        Requirement::TargetInfernal => {
            let card = st.seats[seat.index()].uncontrolled[i].card;
            card != crate::state::CREATED && crate::crypt::CRYPT[card as usize].infernal
        }
        Requirement::TargetUnique => {
            let card = st.seats[seat.index()].uncontrolled[i].card;
            card != crate::state::CREATED && crate::crypt::CRYPT[card as usize].unique
        }
        Requirement::TargetWouldNotContest => {
            let card = st.seats[seat.index()].uncontrolled[i].card;
            card == crate::state::CREATED || !st.name_in_play(card)
        }
        Requirement::SlotWithout(name) => !st.cards.iter().any(|c| {
            c.in_play
                && c.owner == seat
                && c.on_slot == Some(i as u8)
                && LIBRARY[c.card as usize].name == name
        }),
        _ => true,
    }
}

/// Evaluate a requirement against a candidate seat target.
pub fn holds_for_seat(
    req: Requirement,
    st: &GameState,
    seat: SeatId,
    action: Option<&ActionCtx>,
    t: SeatId,
) -> bool {
    if st.seats[t.index()].ousted {
        return false;
    }
    match req {
        Requirement::NotActingController => action.is_some_and(|a| a.seat != t),
        Requirement::NotSelf => t != seat,
        Requirement::NotPrey => t != st.prey(seat),
        Requirement::PredatorsPredator => t == st.predator(st.predator(seat)),
        Requirement::TargetPrey => t == st.prey(seat),
        Requirement::TargetPredator => t == st.predator(seat) && t != seat,
        Requirement::TargetPredatorOrPrey => t == st.prey(seat) || t == st.predator(seat),
        Requirement::TargetPoolMoreThanYours => {
            st.seats[t.index()].pool > st.seats[seat.index()].pool
        }
        Requirement::TargetSeatOusting => st.seats[t.index()].oust_pending,
        Requirement::TargetSeatLostTitled => st.titled_left.iter().any(|&(s, _)| s == t),
        Requirement::CanSaveTarget => st.seats[seat.index()].pool > 1 - st.seats[t.index()].pool,
        Requirement::TargetAshAtLeast(n) => {
            let s = &st.seats[t.index()];
            s.ash.len() + s.ash_crypt.len() >= n as usize
        }
        Requirement::TargetHasUncontrolled => !st.seats[t.index()].uncontrolled.is_empty(),
        _ => true,
    }
}

/// Evaluate a requirement against a candidate card target of `actor`'s
/// action.
pub fn holds_for_card_target(
    req: Requirement,
    st: &GameState,
    seat: SeatId,
    actor: Option<MinionId>,
    id: CardId,
) -> bool {
    let c = &st.cards[id.index()];
    match req {
        Requirement::NotControlled => st.card_controller(id) != seat,
        Requirement::Controlled => st.card_controller(id) == seat,
        Requirement::NotHost => actor.is_none() || c.host != actor,
        Requirement::TargetOnActor => actor.is_some() && c.host == actor,
        // A card that became another has no type nor label, still a card
        // (Enhanced Coagulant).
        Requirement::TargetCard(f) if f != CardFilter::Any && became(c.card, c.mode) => false,
        // A location in play is no equipment (Living Manse; rules-qa 17.4);
        // a card that became a weapon is (Weighted Walking Stick, Zip Gun).
        Requirement::TargetCard(CardFilter::Type(CardType::Equipment)) => {
            is_equipment_in_play(c.card) || becomes_weapon(c.card)
        }
        Requirement::TargetCard(f) => f.matches(c.card),
        Requirement::TargetCardWithout(name) => !st
            .cards
            .iter()
            .any(|o| o.present() && o.on_card == Some(id) && LIBRARY[o.card as usize].name == name),
        Requirement::TargetUsesCounters => uses_counters(c.card),
        Requirement::TargetWeapon => is_weapon(c.card),
        Requirement::TargetOnReadyMinion => c.host.is_some_and(|h| st.minions[h.index()].ready()),
        Requirement::TargetOnVampire => c.host.is_some_and(|h| !st.minions[h.index()].is_ally()),
        Requirement::TargetOnOwnMinion => {
            c.host.is_some_and(|h| st.minions[h.index()].owner == seat)
        }
        Requirement::TargetCardPlayedByOther => c.owner != seat,
        Requirement::TargetOnPreyMinion => c
            .host
            .is_some_and(|h| st.minions[h.index()].owner == st.prey(seat)),
        Requirement::CardControlledByPrey => st.card_controller(id) == st.prey(seat),
        Requirement::CardControllerMoreLocations => {
            st.locations_controlled(st.card_controller(id)) > st.locations_controlled(seat)
        }
        Requirement::CardControlledByPredatorOrPrey => {
            let c = st.card_controller(id);
            c == st.prey(seat) || c == st.predator(seat)
        }
        Requirement::ActorMayHoldTarget => {
            actor.is_some_and(|m| st.can_hold(m, c.card, &[])) && !st.pinned(id)
        }
        // Unlocked, and not a card of that name (Powerbase: Savannah).
        Requirement::TargetUnlocked => !c.locked,
        Requirement::TargetNotNamed(n) => LIBRARY[c.card as usize].name != n,
        // A card on a locked non-anarch minion of the predator or prey
        // (Smear Campaign).
        Requirement::TargetOnLockedMinion => c.host.is_some_and(|h| st.minions[h.index()].locked),
        Requirement::TargetOnMinionNotOfSect(sect) => {
            c.host.is_some_and(|h| st.sect_of(h) != Some(sect))
        }
        Requirement::TargetOnPredatorOrPreyMinion => c.host.is_some_and(|h| {
            let o = st.minions[h.index()].owner;
            o != seat && (o == st.predator(seat) || o == st.prey(seat))
        }),
        Requirement::TargetCardNot(f) => {
            !holds_for_card_target(Requirement::TargetCard(f), st, seat, actor, id)
        }
        Requirement::TargetDisciplineNotSuperiorOnActor => st
            .card_discipline(id)
            .zip(actor)
            .is_some_and(|(d, a)| st.discipline_level(a, d) < 2),
        Requirement::TargetOnSameCircle { sanguinus } => c.host.zip(actor).is_some_and(|(h, a)| {
            if sanguinus {
                st.same_circle_for_play(a, h)
            } else {
                st.same_circle(a, h)
            }
        }),
        // Equipment or retainer (Donnybrook [obf]).
        Requirement::AnyOf(alternatives) => alternatives.iter().any(|reqs| {
            reqs.iter()
                .all(|&r| holds_for_card_target(r, st, seat, actor, id))
        }),
        _ => true,
    }
}

/// Evaluate a predicate on the card in play itself (an in-play ability's
/// "lock this card", a rider's counters), within the current action if any.
pub fn holds_for_card(
    req: Requirement,
    st: &GameState,
    id: CardId,
    action: Option<&ActionCtx>,
) -> bool {
    let c = &st.cards[id.index()];
    match req {
        // "This Lasombra" on a card its controller uses (Zaire River
        // Ferry): read on its bearer, which may have changed (RTR 19960112).
        Requirement::HostClan(_) | Requirement::HostDiscipline(_) => {
            c.bearer().is_none_or(|h| holds_for_host(req, st, h))
        }
        Requirement::ActionTargetsHost => action.is_some_and(|a| {
            a.directed_seat(st).is_some()
                && c.host.is_some_and(|h| a.target_minions().any(|t| t == h))
        }),
        Requirement::DirectedAtCardItIsOn => action.is_some_and(|a| {
            a.directed_seat(st).is_some()
                && c.on_card.is_some_and(|on| a.target == Target::Card(on))
        }),
        Requirement::CardControllerActive => st.card_controller(id) == st.active,
        Requirement::HostOutOfPlay => c.host.is_some_and(|h| {
            let v = &st.minions[h.index()];
            v.in_play && v.out_of_play && v.is_vampire()
        }),
        // The host in the combat that just ended, its opponent not ready
        // (Monster).
        Requirement::HostCombatOpponentNotReady => {
            let host = c.bearer();
            action.and_then(|a| a.last_combat).is_some_and(|pair| {
                host.is_some_and(|h| pair.contains(&h))
                    && pair
                        .iter()
                        .any(|&o| Some(o) != host && !st.minions[o.index()].ready())
            })
        }
        // A burned vampire is not ready; an ally opponent does not count.
        Requirement::HostCombatOpposingVampireNotReady => {
            let host = c.bearer();
            action.and_then(|a| a.last_combat).is_some_and(|pair| {
                host.is_some_and(|h| pair.contains(&h))
                    && pair.iter().any(|&o| {
                        let v = &st.minions[o.index()];
                        Some(o) != host && v.is_vampire() && !v.ready()
                    })
            })
        }
        Requirement::HostUsedDisciplines(n) => {
            let host = c.bearer();
            action.is_some_and(|a| {
                let mut used: Vec<&str> = a
                    .disciplines_played
                    .iter()
                    .filter(|(m, _)| Some(*m) == host)
                    .map(|(_, d)| *d)
                    .collect();
                used.sort_unstable();
                used.dedup();
                used.len() >= n as usize
            })
        }
        // Emerald Legionnaire: the copies its controller controls in play,
        // against the ready vampires of the clan they control.
        Requirement::FewerCopiesThanReadyOfClan(clan) => {
            let controller = st.card_controller(id);
            let copies = st
                .cards
                .iter()
                .enumerate()
                .filter(|&(j, o)| {
                    o.present()
                        && o.card == c.card
                        && st.card_controller(CardId(j as u32)) == controller
                })
                .count();
            let ready = st
                .minions_of(controller)
                .filter(|&m| {
                    let v = &st.minions[m.index()];
                    v.is_vampire() && v.ready() && v.clan() == Some(clan)
                })
                .count();
            copies < ready
        }
        // Phase uses by the copies its controller controls (Split the Veil).
        Requirement::FewerCopiesUsedThisPhase(n) => {
            let controller = st.card_controller(id);
            let used = st
                .cards
                .iter()
                .enumerate()
                .filter(|&(j, o)| {
                    o.card == c.card
                        && st.card_controller(CardId(j as u32)) == controller
                        && st
                            .phase_uses
                            .iter()
                            .any(|(src, _)| *src == AbilitySource::Card(CardId(j as u32)))
                })
                .count();
            used < n as usize
        }
        // The remembered set, its host's controller not counted (LSJ
        // 20100527), all among the action's targets (rules-qa 24.5).
        Requirement::DirectedAtRemembered => {
            let remembered = match c.host {
                Some(h) => c.against.without(st.minions[h.index()].owner),
                None => c.against,
            };
            !remembered.is_empty()
                && action.is_some_and(|a| remembered.is_subset(a.directed_seats(st)))
        }
        // An ally's own card: the ally locks ("this ally can lock to").
        Requirement::ReferendumToBurnThisCard => action.is_some_and(|a| {
            a.kind == crate::decision::ActionKind::Political
                && a.source == Some(AbilitySource::Card(id))
        }),
        Requirement::CardUnlocked => match c.body {
            Some(m) => !st.minions[m.index()].locked,
            None => !c.locked,
        },
        Requirement::CountersAtLeast(n) => c.counters >= n,
        Requirement::CountersWithBleedGainAtLeast(n) => {
            let gain = action.is_some_and(|a| a.momentum.contains(&(id, true)));
            c.counters + u8::from(gain) >= n
        }
        Requirement::CountersAtMost(n) => c.counters <= n,
        Requirement::BoonTargetActive => c.against.contains(st.active),
        Requirement::EnteredBeforeThisTurn => c.entered < st.turn,
        Requirement::EnteredBeforeThisAction => c.entered_action != st.action_serial,
        Requirement::CardMode(n) => c.mode == n,
        Requirement::StoreEmpty => c.stored.is_empty(),
        Requirement::StoreNotEmpty => !c.stored.is_empty(),
        Requirement::SuperiorVersion => c.mode > 0,
        Requirement::BasicVersion => c.mode == 0,
        Requirement::CardLocked => match c.body {
            Some(m) => st.minions[m.index()].locked,
            None => c.locked,
        },
        Requirement::LifeAtLeast(n) => match c.body {
            Some(m) => st.minions[m.index()].blood >= n,
            None => c.counters >= n,
        },
        Requirement::ControllerControlsHost => c
            .host
            .is_some_and(|h| st.minions[h.index()].owner == c.controller),
        _ => true,
    }
}

/// Evaluate a host predicate for an in-play ability.
pub fn holds_for_host(req: Requirement, st: &GameState, host: MinionId) -> bool {
    let v = &st.minions[host.index()];
    match req {
        Requirement::HostLocked => v.locked,
        Requirement::HostUnlocked => !v.locked,
        Requirement::HostReady => v.ready(),
        Requirement::ActivePoxed => st
            .minions_of(st.active)
            .any(|m| st.minions[m.index()].pox > 0),
        Requirement::HostConvictionAtLeast(n) => st.convictions(host).len() >= n as usize,
        Requirement::HostHoldsUnlocked(h, n) => st.holds_unlocked(host, h) >= n,
        Requirement::HostIncapacitated => v.present() && v.incapacitated,
        Requirement::ImbuedRecoveryGranted => (0..st.minions.len()).any(|i| {
            st.minions[i].present()
                && st.printed_mod(
                    MinionId(i as u32),
                    Stat::GrantsImbuedRecovery,
                    crate::state::PrintedCtx::default(),
                ) > 0
        }),
        Requirement::HostControllerReadyImbuedAtLeast(n) => {
            st.minions_of(v.owner)
                .filter(|&o| st.minions[o.index()].is_imbued() && st.minions[o.index()].ready())
                .count()
                >= n as usize
        }
        Requirement::HostVampire => v.is_vampire(),
        Requirement::HostAlly => v.is_ally(),
        Requirement::HostBloodAtLeast(n) => v.blood >= n,
        Requirement::HostBloodAtMost(n) => v.blood <= n,
        Requirement::HostBelowCapacity => st.capacity_of(host).is_some_and(|c| v.blood < c),
        Requirement::HostBelowStartingLife => st.ally_body(host).is_some_and(|b| v.blood < b.life),
        Requirement::HostControllerPoolAtLeast(n) => st.seats[v.owner.index()].pool >= n as i32,
        Requirement::HostInTorpor => v.in_play && v.in_torpor,
        Requirement::HostDidNotHunt => !v.hunted,
        Requirement::HostAnnouncedNoAction => !v.announced_action,
        Requirement::GehennaInPlay(n) => st.gehenna_in_play() >= n as usize,
        Requirement::NoGehennaInPlay => st.gehenna_in_play() == 0,
        Requirement::HostControllerActive => v.in_play && v.owner == st.active,
        Requirement::HostControllerPutNoGehenna => !st.seats[v.owner.index()].gehenna_put,
        // Gravebound Drone: another copy of it is the user's combatant.
        Requirement::OwnCombatantOtherCopy => st.combatants.is_some_and(|pair| {
            pair.iter().any(|&m| {
                m != host
                    && st.minions[m.index()].owner == v.owner
                    && st.minions[m.index()].name() == v.name()
            })
        }),
        Requirement::HostClan(c) => v.clan() == Some(c),
        Requirement::HostDiscipline(d) => st.has_discipline(host, d),
        Requirement::HostControllerHasEdge => st.seats[v.owner.index()].has_edge,
        _ => true,
    }
}

/// X for terms the game state determines, no decision (`CopiesControlled`:
/// the seat's copies in play).
pub fn terms_x(st: &GameState, seat: SeatId, card: u16, mode: &ModeDef) -> Option<u8> {
    match mode.terms {
        Some(Terms::CopiesControlled) => Some(
            st.cards
                .iter()
                .filter(|c| {
                    c.present()
                        && c.owner == seat
                        && LIBRARY[c.card as usize].name == LIBRARY[card as usize].name
                })
                .count() as u8,
        ),
        Some(Terms::OwnReadyUnlockedOfClan(clan)) => Some(
            st.minions_of(seat)
                .filter(|&m| {
                    let v = &st.minions[m.index()];
                    v.is_vampire() && v.ready() && !v.locked && v.clan() == Some(clan)
                })
                .count() as u8,
        ),
        _ => None,
    }
}

/// `payer`'s pool cost of `card` on `target`, through the cost layers
/// (spec §1.7.2; §1.15: copies add).
pub fn pool_cost(st: &GameState, payer: SeatId, card: u16, target: Target) -> i32 {
    let def = &LIBRARY[card as usize];
    let mut cost = match def.pool_cost {
        Cost::None => 0,
        Cost::Fixed(n) => n as i32,
        Cost::X => 0,
    };
    for l in st.layers.iter().filter(|l| st.layer_active(l)) {
        let Modifier::CostMod {
            name,
            on_host_only,
            owner_only,
            pool,
        } = l.modifier
        else {
            continue;
        };
        if name != def.name {
            continue;
        }
        let source = &st.cards[l.source.index()];
        if on_host_only && Target::from(source.host) != target {
            continue;
        }
        if owner_only && source.owner != payer {
            continue;
        }
        cost += pool as i32;
    }
    cost += st
        .minions
        .iter()
        .filter(|v| {
            v.ready() && vampire_of(v).is_some_and(|d| d.pool_surcharge.contains(&def.name))
        })
        .count() as i32;
    // Centralized Background Check: weapons, any Methuselah's.
    for l in st.in_play_layers() {
        if let Modifier::LabelPoolSurcharge {
            label,
            pool,
            while_host_ready,
        } = l.modifier
        {
            let ready = || {
                st.cards[l.source.index()]
                    .bearer()
                    .is_some_and(|h| st.minions[h.index()].ready())
            };
            if def.labels.contains(&label) && (!while_host_ready || ready()) {
                cost += pool as i32;
            }
        }
    }
    // Therbold Realty: a location master card (a locquipment's is
    // `GameState::equip_discount`'s).
    if def.types.contains(&CardType::Master)
        && def.labels.contains(&crate::library::Label::Location)
    {
        cost -= st.location_discount(payer) as i32;
    }
    if let Target::Minion(m) = target {
        if def.types.contains(&CardType::Master) {
            cost += st.master_surcharge_on(m, payer);
        }
    }
    // A master card its Methuselah's vampire reduces (Salvador Garcia).
    if def.types.contains(&CardType::Master) {
        let ctx = crate::state::PrintedCtx {
            card: Some((card, 0)),
            ..Default::default()
        };
        cost -= st
            .minions_of(payer)
            .map(|m| st.printed_mod(m, Stat::SeatCostOff, ctx).max(0))
            .sum::<i32>();
    }
    cost.max(0)
}

impl From<Option<MinionId>> for Target {
    fn from(m: Option<MinionId>) -> Target {
        match m {
            Some(m) => Target::Minion(m),
            None => Target::None,
        }
    }
}

/// The convictions "[n CONVICTION]" printed as the card's cost: burned from
/// the imbued playing it (rules-qa 87.10, 102.1); X as chosen.
pub fn conviction_cost(card: u16, mode: u8, x: u8) -> u8 {
    match LIBRARY[card as usize].conviction_cost {
        // A mode's own (Cleave).
        Cost::None => lookup(card)
            .and_then(|d| d.modes.get(mode as usize))
            .and_then(|m| {
                m.effects.iter().find_map(|e| match e {
                    Effect::ConvictionCost(n) => Some(*n),
                    _ => None,
                })
            })
            .unwrap_or(0),
        Cost::Fixed(n) => n,
        Cost::X => x,
    }
}

/// Blood cost of `card`, paid by the playing minion at resolution, through
/// the action-scoped cost layers (Consign to Oblivion).
pub fn blood_cost(st: &GameState, card: u16, x: u8) -> u8 {
    let def = &LIBRARY[card as usize];
    let mut cost = match def.blood_cost {
        Cost::None => 0,
        Cost::Fixed(n) => n as i32,
        Cost::X => x as i32,
    };
    if def.types.contains(&CardType::Reaction) {
        for l in &st.layers {
            if let Modifier::ReactionBloodCost(n) = l.modifier {
                cost += n as i32;
            }
        }
    }
    cost.max(0) as u8
}

/// Whether `m` could have performed action `a`, with every card the acting
/// minion played on it, read as for an action card's actor (Mask of a
/// Thousand Faces).
pub fn mask_capable(st: &GameState, a: &ActionCtx, m: MinionId) -> bool {
    use crate::decision::ActionKind as K;
    let v = &st.minions[m.index()];
    // An effect of a card on the acting minion used this action (Aching
    // Beauty): not the masking vampire's (RTR 19980623); nor its inherent
    // stealth, "not even a minion that has a similar stealth bonus" (RTR
    // 20041202; rules-qa 58.13). The block's combat canceled, the action stays
    // blocked (Bear-Baiting: LSJ 20090322).
    if m == a.acting
        || a.block_combat_canceled
        || !v.is_vampire()
        || !v.ready()
        || v.locked
        || a.source.is_some()
        || a.host_card_applied
        || st.printed_mod(a.acting, Stat::Stealth, a.printed_ctx(st, None)) > 0
    {
        return false;
    }
    // Out of its Methuselah's turn, only a vampire that may act then takes
    // it over (Madness Network: LSJ 19981007).
    if a.seat != st.active && !st.acts_out_of_turn(m) {
        return false;
    }
    // A library card's clan and one of its Disciplines (the level played
    // unknown for a modifier: any).
    let may_play = |card: u16, tags: &[&str]| {
        let c = &LIBRARY[card as usize];
        (c.clans.is_empty() || st.meets_clan(m, c.clans, true))
            && (tags.is_empty() || tags.iter().any(|t| st.has_discipline(m, t)))
    };
    let kind_ok = match a.kind {
        K::Bleed => v.may_bleed() && !st.no_directed_actions(m),
        K::Hunt => !st.cannot_hunt(m),
        K::Diablerie => !st.cannot_diablerize(m),
        K::Rescue | K::EquipFrom => true,
        K::BecomeAnarch => st.may_become_anarch(m),
        K::Political => !v.political_this_turn && !st.cannot_act_politically(m),
        K::Card | K::Equip | K::Employ | K::Recruit => true,
        K::Power => v.is_imbued(),
        K::BurnImbued => true,
        K::LeaveTorpor
        | K::BurnCard
        | K::StealCard
        | K::EquipFound
        | K::Ability
        | K::RedListHunt => false,
    };
    let card_ok = a.card.is_none_or(|p| {
        let tags = LIBRARY[p.card as usize].modes[library_mode(p.card, p.mode as usize)].tags;
        let reqs_ok = lookup(p.card).is_some_and(|d| {
            d.modes[p.mode as usize].requires.iter().all(|&r| match r {
                Requirement::ActorTitleIn(cs) => cs.iter().any(|c| c.matches(st.title_of(m))),
                Requirement::ActorSectIn(ss) => st.sect_of(m).is_some_and(|s| ss.contains(&s)),
                Requirement::ActorVampire | Requirement::ActorImbued => true,
                _ => true,
            })
        });
        may_play(p.card, tags) && reqs_ok
    });
    let played_ok = a
        .played
        .iter()
        .filter(|(p, _)| *p == a.acting)
        .all(|&(_, name)| {
            let card = crate::state::find_library(name);
            may_play(card, LIBRARY[card as usize].disciplines)
        });
    kind_ok && card_ok && played_ok
}

/// Card `card`'s definition puts, moves or reads counters on it (Church of
/// the Order of St. Blaise's "a location … that uses counters").
pub fn uses_counters(card: u16) -> bool {
    let counting = |e: &Effect| {
        matches!(
            e,
            Effect::EnterWithCounters(_)
                | Effect::EnterWithCountersPerX(_)
                | Effect::AddCounter(_)
                | Effect::RemoveCounter(_)
                | Effect::CounterToPool
                | Effect::CountersToPool
                | Effect::CounterToTargetBlood
                | Effect::BurnSelfWithoutCounters
        )
    };
    lookup(card).is_some_and(|d| {
        d.modes
            .iter()
            .chain(d.abilities)
            .any(|m| m.effects.iter().any(counting))
            || d.riders.iter().any(|r| r.effects.iter().any(counting))
    })
}
