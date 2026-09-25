//! Card behavior: the agreed representation (DESIGN §4.5,
//! docs/effects-framework.md). A card is a set of modes; a mode's *legality
//! half* (windows, actor, requirements, cost, targets, terms) is a closed
//! vocabulary evaluated here by the engine, so the option mask is enumerated
//! from data; its *execution half* (effects) is interpreted at resolution.
//! Definitions live in `defs`.

pub mod defs;
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
    ActionDeclareRecruit,
    ActionDeclareEmploy,
    ActionAsAnnounced,
    ActionDuring,
    ActionStealthIntercept,
    ActionBlockAttempt,
    ActionBlocked,
    ActionBlocksDeclined,
    ActionWouldBeSuccessful,
    ActionAfterResolution,
    ActionAfterCombat,
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
    CombatPress,
    CombatEndOfRound,
    CombatWouldEnd,
    CombatUntimed,
    /// In-play ability hook: during any Methuselah's unlock phase.
    UnlockPhaseAny,
    /// In-play ability hook: during the host's controller's unlock phase
    /// ("during their unlock phase": Vessel).
    UnlockPhaseOwn,
    /// The active Methuselah's discard phase, for an ability spending a
    /// discard phase action ("you can use a discard phase action to …",
    /// Ayse Dhanial): no "during" clause, so not once per phase.
    DiscardPhaseAction,
    /// In-play ability hook: any decision point where the controller
    /// holds the impulse — "can be used during an action, when the
    /// controller has the opportunity to play an effect" (ANK 20200616) —
    /// except the as-played window (cancels and wakes only, rules-qa
    /// 2.4). Matched only where a request site asks for it.
    AnyImpulse,
}

impl Window {
    /// A phase window: an in-play ability offered here follows the
    /// "during X, do Y" template — once per that phase per copy (rules-qa
    /// 6.6).
    pub fn is_phase(self) -> bool {
        matches!(
            self,
            Window::MasterPhase
                | Window::UnlockPhaseAny
                | Window::UnlockPhaseOwn
                | Window::DiscardPhase
        )
    }
}

impl Window {
    pub fn from_name(name: &str) -> Option<Window> {
        use Window::*;
        Some(match name {
            "MASTER_PHASE" => MasterPhase,
            "CARD_AS_PLAYED" => CardAsPlayed,
            "DISCARD_PHASE" => DiscardPhase,
            "ACTION_DECLARE" => ActionDeclare,
            "ACTION_DECLARE_POLITICAL" => ActionDeclarePolitical,
            "ACTION_DECLARE_EQUIP" => ActionDeclareEquip,
            "ACTION_DECLARE_RECRUIT" => ActionDeclareRecruit,
            "ACTION_DECLARE_EMPLOY" => ActionDeclareEmploy,
            "ACTION_AS_ANNOUNCED" => ActionAsAnnounced,
            "ACTION_DURING" => ActionDuring,
            "ACTION_STEALTH_INTERCEPT" => ActionStealthIntercept,
            "ACTION_BLOCK_ATTEMPT" => ActionBlockAttempt,
            "ACTION_BLOCKED" => ActionBlocked,
            "ACTION_BLOCKS_DECLINED" => ActionBlocksDeclined,
            "ACTION_WOULD_BE_SUCCESSFUL" => ActionWouldBeSuccessful,
            "ACTION_AFTER_RESOLUTION" => ActionAfterResolution,
            "ACTION_AFTER_COMBAT" => ActionAfterCombat,
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
            "COMBAT_PRESS" => CombatPress,
            "COMBAT_END_OF_ROUND" => CombatEndOfRound,
            "COMBAT_WOULD_END" => CombatWouldEnd,
            "COMBAT_UNTIMED" => CombatUntimed,
            _ => return None,
        })
    }
}

/// Who may use a mode (effects-framework §2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Actor {
    /// The Methuselah, no minion (masters, events).
    Methuselah,
    /// The acting minion (action modifiers).
    ActingMinion,
    /// A ready unlocked minion of a Methuselah other than the acting one
    /// (reaction cards); widened to locked minions by a wake or by the
    /// mode's own `LockedActor` requirement.
    ReactingMinion,
    /// A ready unlocked minion of the active Methuselah declaring an action
    /// with this card (action cards, equipment, allies, retainers).
    ActingCandidate,
    /// The host minion of a card in play (in-play abilities).
    Host,
    /// The Methuselah controlling a card in play, no minion: the ability
    /// picks a `target` (a hunting ground's vampire).
    Controller,
    /// The player's minion in the current combat (combat cards). Ready,
    /// except in the end-of-round window, where a minion being burned or
    /// sent to torpor may still play unless the card excludes it.
    Combatant,
    /// An ability's host — on a vampire's own text, the vampire — in the
    /// combat or out of it, in torpor included, used for its controller's
    /// minion in combat: the combat effects apply to that minion (Opikun).
    ForOwnCombatant,
    /// "Action Modifier/Reaction" (Scalpel Tongue, Ominous Chorus): the
    /// acting minion for its controller, a reacting minion for anyone else.
    ActingOrReactingMinion,
}

/// A class of printed title, for "requires a prince or justicar" style
/// requirements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleClass {
    Any,
    Primogen,
    Prince,
    Baron,
    Justicar,
    /// No V5 vampire holds it yet: `crypt::Title` gains the Sabbat titles
    /// with the Sabbat precons (owner, 2026-09-13: keep the card's text).
    Archbishop,
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
            _ => false,
        }
    }
}

/// What a minion may hold only one of ("a minion can have only one
/// vehicle", "… only one Kevlar Vest").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holding {
    Label(crate::library::Label),
    Named(&'static str),
}

/// The trait printed text names the bearer by ("this Ravnos", "this
/// Anarch", "the bearer with Auspex"): read on the current bearer, so a
/// minion without it gets nothing from that text (rules-qa 9.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bearer {
    Any,
    Clan(&'static str),
    Sect(Sect),
    /// "The bearer with Auspex [aus]": lower case at any level, upper case
    /// superior only.
    Discipline(&'static str),
}

impl Bearer {
    pub fn is(self, st: &GameState, m: MinionId) -> bool {
        let v = st.minions[m.index()].card();
        match self {
            Bearer::Any => true,
            Bearer::Clan(c) => v.clan == c,
            Bearer::Sect(s) => v.sect == s,
            Bearer::Discipline(d) => has_discipline(v.disciplines, d),
        }
    }
}

/// Does a printed discipline list meet `tag` (lower case: any level; upper
/// case: superior)?
fn has_discipline(disciplines: &[&str], tag: &str) -> bool {
    if tag.chars().all(|c| c.is_ascii_uppercase()) {
        disciplines.contains(&tag)
    } else {
        let upper = tag.to_ascii_uppercase();
        disciplines.iter().any(|d| *d == tag || *d == upper)
    }
}

/// The hand cards a cost or a requirement names ("discard two master
/// cards", "a card requiring Dominate [dom]").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardFilter {
    Any,
    Type(CardType),
    /// A library card whose printed disciplines name this one.
    Discipline(&'static str),
    /// A card of this type bearing this label ("a master archetype card").
    Labeled(CardType, crate::library::Label),
    /// An equipment in play bearing none of these labels ("a
    /// non-electronic, non-vehicle, non-weapon equipment"): a locquipment
    /// is only a location once in play (spec §1.3.1).
    EquipmentWithout(&'static [crate::library::Label]),
    /// A card requiring a Discipline, any one (Aniel).
    RequiresDiscipline,
}

impl CardFilter {
    pub fn matches(self, card: u16) -> bool {
        let c = &LIBRARY[card as usize];
        match self {
            CardFilter::Any => true,
            CardFilter::Type(t) => c.types.contains(&t),
            CardFilter::Discipline(d) => c.disciplines.contains(&d),
            CardFilter::Labeled(t, l) => c.types.contains(&t) && c.labels.contains(&l),
            CardFilter::EquipmentWithout(ls) => {
                c.types.contains(&CardType::Equipment)
                    && !c.labels.contains(&crate::library::Label::Location)
                    && !ls.iter().any(|l| c.labels.contains(l))
            }
            CardFilter::RequiresDiscipline => !c.disciplines.is_empty(),
        }
    }
}

/// A minion statistic a card in play modifies on its bearer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stat {
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
    /// Its controller's hand size.
    HandSize,
    /// The damage of its ranged strikes, at either range (Noluthando).
    RangedStrikeDamage,
    /// Its controller's discard phase actions, read as the discard phase
    /// begins (Sreelekha).
    DiscardPhaseActions,
    /// A press (mandatory) each combat, only usable to continue combat
    /// (Adrino Manauara).
    MandatoryContinuePress,
    /// Of its votes cast against a blood hunt referendum (Jason "Son"
    /// Newberry).
    VotesAgainstBloodHunt,
    /// Votes in favor of a blood hunt referendum called on it, counted as
    /// the referendum is called (Styles Margs).
    BloodHuntVotesInFavor,
}

/// Legality predicates — the closed vocabulary (effects-framework §2.3).
/// Evaluated by the engine over game state and the current context; a
/// predicate on a target reads the candidate target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Requirement {
    /// Target minion is controlled by the player.
    Controlled,
    /// Target minion has at least this much blood.
    BloodAtLeast(u8),
    /// Target seat is not the acting minion's controller (bleed redirects).
    NotActingController,
    /// Target seat is not the player.
    NotSelf,
    /// The card being played (as-played context) is a master card.
    PlayedCardIsMaster,
    /// The card being played was played by another Methuselah.
    PlayedByOtherSeat,
    /// Playable only on another Methuselah's turn (out-of-turn masters).
    NotOwnTurn,
    /// The current action is directed at the player.
    DirectedAtYou,
    /// The current action is a bleed.
    IsBleed,
    /// The current bleed amount is at least this.
    BleedAmountAtLeast(u8),
    /// Blocks have been declined by all Methuselahs (state C).
    BlocksDeclined,
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
    /// The player controls a ready vampire holding a title of one of these
    /// classes ("requires a ready vampire with a city title", Papillon).
    ControlsReadyTitled(&'static [TitleClass]),
    /// The player controls a ready vampire of one of these sects
    /// ("requires a ready Anarch", Carfax Abbey).
    ControlsReadySect(&'static [Sect]),
    /// The host's controller has at least n pool.
    HostControllerPoolAtLeast(u8),
    /// The host of the card in play is locked.
    HostLocked,
    /// The host of the card in play has at least this much blood.
    HostBloodAtLeast(u8),
    /// The host of the card in play is in torpor (Fame's unlock rider;
    /// a host that left play with its Methuselah does not qualify).
    HostInTorpor,
    /// The player has a master phase action left ("use a master phase
    /// action", Tension in the Ranks).
    MasterPhaseActionLeft,
    /// The player holds at least n cards matching the filter ("discard two
    /// master cards", "discard a card requiring Dominate [dom]").
    HandHas(CardFilter, u8),
    /// The actor is a vampire ("Vampires can burn this card"): every
    /// minion is a crypt vampire until allies land (P1 ally slice).
    ActorVampire,
    /// This card in play carries at least n counters (a rider with
    /// nothing to pay is not pending: Smiling Jack at 0 counters).
    CountersAtLeast(u8),
    /// This card in play is unlocked ("you can lock this card to …").
    CardUnlocked,
    /// The player has the Edge ("if you have the Edge").
    HasEdge,
    /// The player's hand is not empty ("discard a card").
    HandNotEmpty,
    /// The player has at least n pool (a granted action "that costs 1
    /// pool" is not declared unpayable).
    PoolAtLeast(u8),
    /// Combat: the round is at close range ("only usable at close range").
    RangeClose,
    RangeLong,
    /// Combat: the first round ("only usable during the first round").
    FirstRound,
    NotFirstRound,
    /// Combat: both combatants are still ready (Telepathic Tracking [AUS]).
    BothCombatantsReady,
    /// Combat: the end-of-round window with no press to continue standing
    /// and no replacement yet ("combat would end", spec §4.9.1).
    CombatWouldEnd,
    /// The actor is ready ("not usable by a vampire being burned or going
    /// to torpor").
    ActorReady,
    /// Damage is pending on the actor (prevention cannot be played with
    /// no damage to prevent, spec §4.5.1).
    DamagePending,
    /// Non-aggravated damage is pending on the actor (Soak).
    NonAggravatedDamagePending,
    /// Damage from gun strikes is pending on the actor (Kevlar Vest).
    GunStrikeDamagePending,
    /// Damage from any source but gun strikes is pending on the actor.
    NonGunDamagePending,
    /// This card was on the blocker when it blocked (Sniper Rifle: not an
    /// equipment equipped after the block, rules-qa 10.5).
    SourceHeldWhenBlocking,
    /// The card's controller controls the minion it is on ("if you control
    /// this minion", Vast Wealth; RTR 19951017).
    ControllerControlsHost,
    /// The card's host is ready ("while this Ravnos is ready", Shilmulo
    /// Tarot; "if the bearer is ready", Heart of Nizchetus).
    HostReady,
    /// No press to continue stands ("press, only usable to continue").
    NoContinuePress,
    /// The actor is the minion that blocked this action ("who has
    /// blocked").
    ActorHasBlocked,
    /// The actor holds a title of one of these classes ("requires a prince
    /// or justicar"; effective title: contested or yielded titles do not
    /// qualify).
    ActorTitleIn(&'static [TitleClass]),
    /// The actor belongs to one of these sects ("requires an Anarch").
    ActorSectIn(&'static [Sect]),
    /// The actor's capacity is at least n.
    ActorCapacityAtLeast(u8),
    /// The current action is a political action.
    IsPolitical,
    /// The polling step of a political action's referendum ("only usable
    /// during the polling step of a political action"; not the blood hunt,
    /// spec §3.7.5.4).
    PollingStep,
    /// The polling step of any referendum, blood hunts included ("during
    /// the polling step of any referendum", Aisha az-Zahra).
    AnyPollingStep,
    /// The referendum is the one the actor's political action called ("a
    /// referendum called by Anne").
    ActorCalledReferendum,
    /// Target vampire is not abstaining from this referendum.
    NotAbstaining,
    /// The actor is the acting minion ("during an action she performs").
    ActorActing,
    /// The current action succeeded, read after resolution; a fizzled
    /// action counts (ANK 20220218).
    ActionSuccessful,
    /// The current action is directed.
    ActionDirected,
    /// The current action is a diablerie.
    IsDiablerie,
    /// The actor diablerised a vampire in this action: as the acting minion
    /// or as the blocker of a leave-torpor action (rules-qa 16.2).
    ActorDiablerised,
    /// The acting minion belongs to another Methuselah: an action of one's
    /// own minions is undirected (LSJ 20030214-2).
    ActingOtherController,
    /// The acting minion is the player's and is not the actor ("another
    /// Anarch you control", Aline Gädeke).
    OtherActingControlled,
    /// The acting minion belongs to one of these sects.
    ActingSectIn(&'static [Sect]),
    /// The Methuselah the action is directed at has cards in hand.
    DirectedHandNotEmpty,
    /// Target minion is controlled by the Methuselah the action is
    /// directed at ("controlled by the target Methuselah", Alicia Cortez).
    ControlledByDirectedSeat,
    /// Target minion is not the vampire whose ability it is ("another ready
    /// Gargoyle you control", Alabástrom).
    NotHost,
    /// The minion attempting to block is the player's and needs intercept
    /// ("give a vampire you control +1 intercept", Oskar Anasov; rules-qa
    /// 3.2).
    YourAttemptNeedsIntercept,
    /// The acting minion is of this clan and is not the actor ("another
    /// Nosferatu", Daria).
    OtherActingClan(&'static str),
    /// The host is unlocked (a lock cost: "can lock to …").
    HostUnlocked,
    /// It is the player's turn ("during your turn", Kalinda).
    OwnTurn,
    /// The player has a discard phase action left.
    DiscardPhaseActionLeft,
    /// The player has played a master card in the current master phase
    /// ("after playing a master card", Nonu Dis).
    MasterCardPlayed,
    /// The player's ash heap is not empty (it holds library cards only:
    /// burned vampires are not kept in it).
    AshNotEmpty,
    /// No votes or ballots have been cast yet in this referendum.
    NoVotesCast,
    /// The action's referendum passed (Voter Captivation).
    ReferendumPassed,
    /// Target minion has cast votes or ballots in this referendum, still
    /// standing (Scalpel Tongue).
    HasCastVotes,
    /// Target minion is younger than the acting minion (lower capacity).
    Younger,
    /// The actor is not voting in favor of the referendum (Protected
    /// District: votes against cannot join a cast in favor).
    ActorNotVotingFor,
    /// Any one of these requirement lists holds in full ("only usable
    /// during a bleed action against you or during the polling step").
    AnyOf(&'static [&'static [Requirement]]),
    /// The actor has no card in play of this kind ("a minion can have only
    /// one vehicle"): an equip is not offered to a minion already holding
    /// one.
    ActorWithout(Holding),
    /// The host of the card in play is of this clan ("this Ravnos can
    /// strike").
    HostClan(&'static str),
    /// The minion that blocked the current action holds a title of one of
    /// these classes ("blocked by a prince or an archbishop").
    BlockerTitleIn(&'static [TitleClass]),
    /// The host of the card in play has this discipline (lower case: any
    /// level; upper case: superior) — "the bearer with superior Auspex".
    HostDiscipline(&'static str),
    /// Printed-modifier predicates (`GameState::printed_mod`), read on the
    /// vampire whose text it is, the modified minion ("actor"), and the
    /// opposing minion — the acting minion for a block attempt, the
    /// opposing combatant in combat. "While this vampire is ready".
    ThisVampireReady,
    /// The action performed is directed / not directed (Castellan).
    IsDirected,
    IsUndirected,
    /// The opposing minion is of this clan ("in combat with a Brujah").
    OpponentClan(&'static str),
    /// The opposing minion is younger / older than the modified minion.
    OpponentYounger,
    OpponentOlder,
    /// The opposing minion holds a title (effective title).
    OpponentTitled,
    /// The modified minion is of this clan ("Brujah get +1 strength").
    ActorClan(&'static str),
    /// The vampire's controller has at most n pool ("while you have 9 or
    /// fewer pool").
    PoolAtMost(u8),
    /// The vampire's controller controls an uncontested location / none.
    ControlsLocation,
    ControlsNoLocation,
    /// The vampire's controller controls a vampire in play of this name
    /// ("while you control Mithras").
    ControlsVampireNamed(&'static str),
    /// The Methuselah controls a locked minion ("if you control a locked
    /// minion", Elen Kamjian).
    ControlsLockedMinion,
    /// Target minion or card is controlled by another Methuselah (a minion
    /// does not enter combat with one its controller controls).
    NotControlled,
    /// Target card in play is of this kind.
    TargetCard(CardFilter),
    /// Target card is on a ready minion ("from another ready minion").
    TargetOnReadyMinion,
    /// The acting minion could hold the target card: none of its "only
    /// one" limits reached (a second vehicle).
    ActorMayHoldTarget,
    /// Target minion is locked ("a locked vampire", Nathaniel Bordruff).
    TargetLocked,
    /// Target minion is a vampire: every minion in play is one until allies
    /// land.
    TargetVampire,
}

/// How many points a referendum allocates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Points {
    Fixed(u8),
    /// "X is the number of Methuselahs in the game" (Conservative
    /// Agitation).
    Methuselahs,
    /// "Allocate n of their pool" (Parity Shift): n points of the chosen
    /// Methuselah's pool; a shortfall at resolution is re-allocated by the
    /// caller within the terms (LSJ 20010606).
    PoolOfChosen(u8),
}

/// Which Methuselahs may receive points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeatFilter {
    Any,
    /// "A Methuselah who has more pool than you do" (Parity Shift).
    MorePoolThanCaller,
}

/// One term of a referendum, chosen stepwise by the caller once the action
/// succeeds (rulebook "The Referendum" step 1; rules-qa 3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    /// Choose one Methuselah (the "chosen" one for later terms).
    ChooseSeat(SeatFilter),
    /// Choose between `min` and `max` minions meeting the filter.
    ChooseMinions {
        filter: &'static [Requirement],
        min: u8,
        max: u8,
    },
    /// Choose a clan.
    ChooseClan,
}

/// What a passed referendum does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefEffect {
    /// Each Methuselah burns 1 pool per point allocated.
    BurnAllocated,
    /// Each Methuselah burns 1 pool per minion they control (`locked_only`:
    /// per locked minion, counted when the effect applies).
    BurnPerMinion { locked_only: bool },
    /// Each chosen Methuselah burns `base` pool, plus `young_bonus` if they
    /// control a ready vampire with capacity `young_cap` or less.
    BurnChosen {
        base: u8,
        young_bonus: u8,
        young_cap: u8,
    },
    /// As `BurnAllocated`, and the chosen Methuselah gains n pool.
    BurnAllocatedChosenGains(u8),
    /// The allocated points move from the chosen Methuselah's pool to the
    /// recipients (Parity Shift).
    MoveAllocatedFromChosen,
    /// Each Methuselah gains 1 pool per vampire of the chosen clan they
    /// control.
    GainPerClanVampire,
    /// Each ready Anarch gains 1 blood; each Methuselah controlling an
    /// Anarch gains 1 pool.
    AnarchSalon,
    /// The chosen vampire goes to its controller's uncontrolled region.
    Banish,
    /// The chosen minions cannot play reaction cards, block or cast votes
    /// or ballots this turn.
    Expel,
    /// Burn the card in play whose granted action called the referendum
    /// ("call a referendum to burn this card").
    BurnSource,
}

/// A political action card's referendum: its terms and its effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferendumDef {
    pub terms: &'static [Term],
    pub effect: RefEffect,
}

impl ReferendumDef {
    /// Can the terms be met (LSJ 20100129: a political action cannot be
    /// played if the conditions for its terms cannot be met)? A choice of
    /// Methuselah is feasible only if some choice leaves the following
    /// allocation feasible.
    pub fn feasible(&self, st: &GameState, seat: SeatId, actor: MinionId) -> bool {
        if st.seats[seat.index()].ousted {
            return false;
        }
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
                Term::ChooseMinions { filter, min, .. } => {
                    let n = (0..st.minions.len())
                        .map(|i| MinionId(i as u32))
                        .filter(|&m| {
                            st.minions[m.index()].present()
                                && filter.iter().all(|&r| {
                                    holds_for_minion(r, st, seat, None, Some(actor), m, None)
                                })
                        })
                        .count();
                    if (n as u8) < min {
                        return false;
                    }
                }
                Term::ChooseClan => {}
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
    }
}

/// A strike (rulebook "Strike Effects"), from the hands, a card or a weapon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrikeKind {
    /// Hand strike: strength (+bonus) damage at close range.
    Hand { bonus: u8 },
    /// A melee weapon's "strength+n damage": close range, not a hand strike
    /// (hand-strike effects do not reach it).
    Melee { bonus: u8, aggravated: bool },
    /// Defensive: the opposing strike has no effect on the dodging minion.
    Dodge,
    /// Combat ends before any other strike resolves; `unlock`: "unlock
    /// this vampire before combat ends".
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
}

/// "A vampire can play only one X each round / combat / action": one play
/// of the card name per minion per scope (a canceled play still counts:
/// Immortal Grapple rulings).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limit {
    PerRound,
    PerCombat,
    PerAction,
    /// An in-play ability in a phase window is once per phase per copy by
    /// default (the "during X" clause, rules-qa 6.6); with this limit each
    /// mode of the clause is its own once-per-phase use (Carfax Abbey's
    /// two picks).
    PerPhaseEach,
    /// "A referendum ability is usable once per referendum" (advanced
    /// rules §1.2.1): unlocking mid-vote does not restore it.
    PerReferendum,
    /// "Once each turn": once during each Methuselah's turn (advanced rules
    /// §1.2.1), per copy of the card or per vampire ability.
    PerTurn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetKind {
    Minion,
    Seat,
    /// A vampire in the player's uncontrolled region (slot index).
    Uncontrolled,
    /// A library card in play (equipment to steal, a card to burn).
    Card,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetSpec {
    pub kind: TargetKind,
    pub filter: &'static [Requirement],
}

/// A resolved target of a play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
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

/// Where an ability used with `Choice::UseAbility` is printed: a card in
/// play (arena index) or a vampire's own text (`cards::vampires`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbilitySource {
    Card(CardId),
    Vampire(MinionId),
}

impl AbilitySource {
    /// The card in play of an ability whose effects act on that card.
    pub fn card(self) -> CardId {
        match self {
            AbilitySource::Card(c) => c,
            AbilitySource::Vampire(m) => {
                unreachable!("an effect on its card in play, used from vampire {m:?}")
            }
        }
    }

    /// The key an "each action / combat / round" limit counts uses by: a
    /// card in play is limited per copy (Kevlar Vest's two branches share
    /// one use, rules-qa 10.3); a vampire's abilities are separate grants
    /// (Abaddon's maneuver and press).
    pub fn use_key(self, ability: u8) -> (AbilitySource, u8) {
        match self {
            AbilitySource::Card(_) => (self, 0),
            AbilitySource::Vampire(_) => (self, ability),
        }
    }

    /// The NRA identity of the action this ability grants: the copy of a
    /// card in play, or the vampire's printed action (spec §3.8.1).
    pub fn action_id(self, ability: u8) -> crate::state::ActionId {
        match self {
            AbilitySource::Card(c) => crate::state::ActionId::InPlay(c),
            AbilitySource::Vampire(m) => crate::state::ActionId::Printed(m, ability),
        }
    }

    /// The definition of ability `ability`, wherever it is printed.
    pub fn mode(self, st: &crate::state::GameState, ability: u8) -> &'static ModeDef {
        let abilities = match self {
            AbilitySource::Card(c) => {
                lookup(st.cards[c.index()].card)
                    .expect("definition")
                    .abilities
            }
            AbilitySource::Vampire(m) => {
                vampire(st.minions[m.index()].card)
                    .expect("vampire definition")
                    .abilities
            }
        };
        &abilities[ability as usize]
    }
}

/// Numeric terms chosen as the card is played (rules-qa 3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terms {
    /// An integer in `min..=max` (the amount actually taken at resolution
    /// is capped by what is there: Villein rulings).
    BloodRange { min: u8, max: u8 },
    /// X blood, 0 up to the actor's blood (Hidden Strength: X may exceed
    /// what is needed, never below zero).
    XBlood,
    /// Up to n of the votes the referendum passed by (Voter Captivation
    /// [PRE]: "move up to 2 of those blood to your pool instead").
    UpToMargin(u8),
    /// "X is the number of copies of this card you control" (Creeping
    /// Sabotage): no decision; read when the cost is paid.
    CopiesControlled,
    /// "You can burn a card named X (if any)" (Vessel): which copy in play,
    /// any controller's, or none — announced with the play like any target
    /// (rules-qa 6.7), burned at resolution.
    BurnNamedInPlay(&'static str),
    /// Ability cost "discard n cards" (`of`: the cards that qualify): which
    /// ones, chosen one at a time as the use is announced (Tension in the
    /// Ranks, The Barrens, Kasim Bayar, Alexa Draper).
    Discard { n: u8, of: CardFilter },
    /// A number of cards in `min..=max`, chosen as the ability is used
    /// (Heart of Nizchetus: "up to 3"; rules-qa 11.1).
    Count { min: u8, max: u8 },
    /// A card in the user's ash heap, chosen by name as the ability is used
    /// (Mora, the Death Seer).
    AshCard,
    /// A card in the hand of the Methuselah the action is directed at,
    /// chosen by name, discarded as the ability resolves (Richard de
    /// Worde: the choice shows the hand, his look).
    DirectedHandCard,
}

/// Continuous-effect modifiers carried by layers (effects-framework §3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modifier {
    /// Cards with this printed name cost `pool` more (or less) to play;
    /// with `on_host_only`, only when played on the layer source's host;
    /// with `owner_only`, only for the layer source's controller ("cost
    /// *you* +1 pool").
    CostMod {
        name: &'static str,
        on_host_only: bool,
        owner_only: bool,
        pool: i8,
    },
    /// Reaction cards cost this much more blood (or life) this action.
    ReactionBloodCost(i8),
    /// Minions younger than this capacity (allies included) get this
    /// intercept modifier this action (Perfect Paragon [PRE]).
    InterceptMod { younger_than: u8, n: i8 },
    /// The layer source's host gets this intercept during bleed actions
    /// directed at its controller (Guardian Angel).
    HostInterceptOnBleedAtYou(i8),
    /// The layer source's controller gets this hand size (Elder Library).
    HandSize(i8),
    /// The layer source's controller gets +1 hand size per counter on it
    /// (Visit from the Capuchin).
    HandSizePerCounter,
    /// The layer source's host, if `who`, gets `n` to `stat` during its
    /// actions of these kinds (any action when empty): Sport Bike's +1
    /// intercept, Unlicensed Taxicab's +1 stealth during hunt and employ
    /// actions, Treasured Samadji's +1 bleed.
    Bearer {
        stat: Stat,
        n: i8,
        during: &'static [ActionKind],
        who: Bearer,
    },
    /// Vampires younger than the layer source's host, if `who`, get `n`
    /// intercept against its actions (Stolen Police Cruiser; allies too,
    /// once they exist).
    YoungerInterceptAgainstBearer { n: i8, who: Bearer },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// While the source card stays in play.
    InPlay,
    /// Until the current action ends.
    Action,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layer {
    pub scope: Scope,
    pub modifier: Modifier,
    pub source: CardId,
}

impl Layer {
    pub fn hash(&self) -> u64 {
        let m = match self.modifier {
            Modifier::CostMod {
                name,
                on_host_only,
                owner_only,
                pool,
            } => {
                (name.len() as u64) << 16
                    ^ (on_host_only as u64) << 9
                    ^ (owner_only as u64) << 8
                    ^ (pool as i64 as u64 & 0xff)
            }
            Modifier::ReactionBloodCost(n) => 1 << 32 ^ (n as i64 as u64 & 0xff),
            Modifier::InterceptMod { younger_than, n } => {
                1 << 33 ^ (younger_than as u64) << 8 ^ (n as i64 as u64 & 0xff)
            }
            Modifier::HostInterceptOnBleedAtYou(n) => 1 << 34 ^ (n as i64 as u64 & 0xff),
            Modifier::HandSize(n) => 1 << 35 ^ (n as i64 as u64 & 0xff),
            Modifier::HandSizePerCounter => 1 << 35 ^ 1 << 12,
            Modifier::Bearer {
                stat,
                n,
                during,
                who,
            } => {
                1 << 37
                    ^ (stat as u64) << 24
                    ^ (during.len() as u64) << 16
                    ^ bearer_hash(who) << 8
                    ^ (n as i64 as u64 & 0xff)
            }
            Modifier::YoungerInterceptAgainstBearer { n, who } => {
                1 << 38 ^ bearer_hash(who) << 8 ^ (n as i64 as u64 & 0xff)
            }
        };
        (self.source.0 as u64) << 40 ^ (self.scope as u64) << 36 ^ m
    }
}

fn bearer_hash(who: Bearer) -> u64 {
    match who {
        Bearer::Any => 0,
        Bearer::Clan(c) => 1 ^ (c.len() as u64) << 2,
        Bearer::Sect(s) => 2 ^ (s as u64) << 2,
        Bearer::Discipline(d) => 3 ^ (d.as_bytes()[0] as u64) << 2,
    }
}

/// Effect primitives (effects-framework §3.1); `X` is the play's terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Put this card on the target minion.
    PutOnTarget,
    /// Put this card in play with no host (Anarch Revolt).
    PutInPlay,
    /// Rider effect: the Methuselah whose unlock phase it is burns `pool`
    /// unless they control a ready vampire of `sect` (Anarch Revolt; each
    /// copy in play burns on its own, spec §1.15).
    UnlockBurnPoolUnlessReady {
        sect: Sect,
        pool: u8,
    },
    /// Rider effect: the controller's prey burns n pool (Creeping
    /// Sabotage).
    PreyBurnsPool(u8),
    /// Rider effect: the Methuselah resolving it burns n pool ("that
    /// Methuselah burns 1 pool", Fame's unlock rider).
    BurnPool(u8),
    /// Rider effect: the Methuselah resolving it moves n counters from
    /// their pool to this card ("move 1 counter from your pool to this
    /// card", Smiling Jack) — down to 0 pool ousts them.
    PoolToCounter(u8),
    /// Rider effect: for each counter on this card, the Methuselah
    /// resolving it burns 1 pool or 1 blood from a vampire they control
    /// (Smiling Jack), each unit chosen as it is paid — any mix (ANK
    /// 20181006), a vampire with no blood is no source (LSJ 19971212).
    PoolOrBloodPerCounter,
    /// Rider effect: the Methuselah resolving it burns X pool and/or
    /// cards at random from their hand, X the counters on this card
    /// (Constant Revolution): they announce how many cards (at most their
    /// hand), those are burned at once, the rest is pool (LSJ 20090618).
    PoolOrRandomCardsPerCounter,
    /// Put in play with n counters ("put this card in play with 1
    /// counter").
    EnterWithCounters(u8),
    /// A granted action's pool cost, paid at resolution ("as a Ⓓ action
    /// that costs 1 pool"; unpayable by then it fizzles, rules-qa 3.6).
    PayPool(u8),
    /// Rider effect: the controller of the minion the hook names burns n
    /// pool — the host ("after this vampire goes to torpor, their
    /// controller burns 3 pool", Fame) or the minion that left ("after a
    /// ready minion is burned or sent to torpor, their controller burns 1
    /// pool", Tension in the Ranks) — at once, where it happened (rules-qa
    /// 6.4).
    TheirControllerBurnsPool(u8),
    /// Ability cost: one of the player's master phase actions ("use a
    /// master phase action … to burn this card").
    SpendMasterPhaseAction,
    /// Ability cost: lock this card in play ("you can lock this card to
    /// …").
    LockSelf,
    /// Ability cost: burn the Edge ("burn the Edge to …", Kalinda).
    BurnEdge,
    /// Ability cost: one of the player's discard phase actions.
    SpendDiscardPhaseAction,
    /// Add n blood to the host, up to its capacity ("add 1 blood to her").
    AddHostBlood(u8),
    /// The library card chosen by `Terms::AshCard` goes from the ash heap to
    /// the bottom of the library (Mora, the Death Seer).
    AshCardToLibraryBottom,
    /// Add n counters to this card in play ("add 1 counter to this card
    /// each time you lock it", Dreams of the Sphinx).
    AddCounter(u8),
    /// Burn this card once it carries n counters ("burn this card if it
    /// has 3 counters"), checked as the ability resolves.
    BurnSelfAtCounters(u8),
    /// Rider effect: burn n counters from this card (Visit from the
    /// Capuchin: "instead burn 1 counter from this card").
    RemoveCounter(u8),
    /// Rider effect: burn this card if it has no counters left.
    BurnSelfWithoutCounters,
    /// The player gains n pool.
    GainPool(u8),
    /// The player's hand size grows by n until the end of the turn (the
    /// end of the discard phase, LSJ 20100812) — the hand is drawn up at
    /// once; cards over size afterward stay (ANK 20180318).
    HandSizeBonusUntilEndOfTurn(u8),
    /// Add n blood to the targeted vampire in the player's uncontrolled
    /// region (capped at its capacity).
    BloodToUncontrolledTarget(u8),
    /// Add n blood to the targeted minion (capped at its capacity; The
    /// Coven: "add 2 blood to a ready vampire you control").
    BloodToTarget(u8),
    /// Rider effect: the controller's predator takes control of this card
    /// (The Coven; spec §6.3: requirements are not re-checked, where it
    /// goes when it leaves play is still its owner's).
    PredatorTakesControl,
    /// Rider effect: burn this card (Guardian Angel: "if this vampire is in
    /// torpor, burn this card").
    BurnSelf,
    /// Burn the card in play granting this action (a "burn this card as a
    /// Ⓓ action" granted action), with `DeclareAction(BurnCard)`.
    BurnSource,
    /// Move X blood from the target minion to the player's pool.
    MoveBloodTargetToPoolX,
    /// Push a continuous layer sourced by this card while it is in play.
    Layer(Modifier),
    /// Push a layer scoped to the current action.
    ActionLayer(Modifier),
    /// Cancel the card being played; `refund`: its cost is not paid.
    CancelPlayed {
        refund: bool,
    },
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
    BleedMod {
        n: u8,
        limited: bool,
    },
    /// The actor wakes for the rest of the action.
    Wake,
    /// Lock the actor.
    LockActor,
    /// Change the bleed's target to the target seat (reopens blocks).
    RedirectBleed,
    /// Burn the acting minion.
    BurnActingMinion,
    /// The action is not successful.
    ActionFails,
    /// Reaction cards are not replaced until the end of the action.
    DeferReactionReplacement,
    /// If this action is blocked, lock the blocking minion and end the
    /// action before block resolution (Mirror Walk [THA]).
    EndActionIfBlocked,
    /// The card (a retainer) enters play on the acting minion with this life.
    EmployRetainer {
        life: u8,
    },
    /// Burn n blood from the host of this card in play (an ability's cost:
    /// Bowl of Convergence [AUS]).
    HostBurnBlood(u8),
    /// A granted action "that costs n blood": paid by the acting minion at
    /// resolution (fizzles if unpayable by then, rules-qa 3.6).
    PayBlood(u8),
    /// A granted bleed "gets +n bleed if the target Methuselah controls no
    /// ready unlocked minions", read at resolution (Codex of the Edenic
    /// Groundskeepers).
    BleedIfTargetHasNoReadyUnlocked(u8),
    /// On a successful granted burn: the card's host is locked and does
    /// not unlock as normal during its controller's next unlock phase
    /// (Stolen Police Cruiser), if it is still `who`.
    LockHostSkipNextUnlock(Bearer),
    /// Burn 1 blood from the host of this card in play to unlock it.
    HostBurnBloodToUnlock,
    /// Move n blood from the host of this card to its controller's pool
    /// (Blood Doll, Vessel).
    HostBloodToPool(u8),
    /// Move n pool from the host's controller to the host (Blood Doll,
    /// Vessel).
    PoolToHostBlood(u8),
    /// The target vampire gains n blood from a hunting ground (capped at
    /// capacity; it counts as fed by a hunting ground this turn either
    /// way, rules-qa 6.5).
    HuntingGroundBlood(u8),
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
    /// Reveal the top card of the user's library, which stays on top: the
    /// host burns `burn` blood if it is a master card, else it gets
    /// `stealth` stealth, and only while stealth is needed (Gathii, ANK
    /// 20231031).
    RevealTopCard {
        burn: u8,
        stealth: u8,
    },
    /// The vampire whose text it is goes to torpor (Cedrick Calhoun).
    GoToTorpor,
    /// Lock the target minion.
    LockTarget,
    /// The acting minion's controller discards the chosen count
    /// (`Terms::Count`) of cards at random from hand, replaced as normal
    /// (Ilonka).
    ActingControllerDiscardsAtRandom,
    // Combat effects (`combat`).
    /// Choose this strike for the actor.
    Strike(StrikeKind),
    /// Maneuver now (determine range step): toggle the range.
    Maneuver,
    /// Press now (press step): continue, or end the opponent's press.
    Press,
    /// Bank an optional press for this round's press step.
    OptionalPress,
    /// If another round occurs, the actor gets 1 optional maneuver then.
    NextRoundManeuver,
    /// Additional strikes this round; `limited`: one such source per
    /// combatant per round (rulebook "Additional Strikes").
    AdditionalStrikes {
        n: u8,
        limited: bool,
    },
    /// The actor gets a strength of n this combat (replaces the base).
    SetStrength(u8),
    /// The actor's hand strikes are aggravated this round.
    AggravatedHands,
    /// Prevent up to n damage (aggravated included).
    Prevent(u8),
    /// Prevent up to X+n damage.
    PreventXPlus(u8),
    /// Prevent up to n non-aggravated damage.
    PreventNonAggravated(u8),
    /// Strikes that are not hand strikes cannot be used this round.
    OnlyHandStrikes,
    /// The next round, if any, is at close range (skip determine range).
    NextRoundClose,
    /// Would-end replacement: start a new round instead.
    NewRound,
    /// The actor gains blood equal to what the opposing vampire lost to
    /// damage this round.
    GainBloodLostByOpponent,
    /// Rider of a strike: combat ends — after combat, if the actor was
    /// blocked, it may burn `blood` to continue the action with +`stealth`
    /// as if unblocked. Lost if the combat continues (spec §4.9.2).
    ContinueAfterCombat {
        blood: u8,
        stealth: u8,
    },
    /// Set this round's range to long, skipping the determine range step;
    /// usable only while no other effect has set it (Sniper Rifle).
    SetRangeLong,
    /// The host's initial strike this round must be this card's strike: a
    /// weapon's maneuver used (rulebook "Determine Range"), Sniper Rifle's
    /// range set. One such commitment a round.
    CommitStrike,
    /// Rider of a weapon's strike, read once the strike has resolved: the
    /// bearer gets 1 optional additional strike (limited) this round, only
    /// usable to strike with this weapon (AK-47).
    AdditionalStrikeWithSource,
    /// "Once each combat, can prevent n damage from gun strikes": one point
    /// a use, the rest kept for later in the combat (rules-qa 10.3).
    PreventGunStrikeAllowance(u8),
    /// Prevent up to n damage from sources other than gun strikes.
    PreventNonGun(u8),
    /// Rider of a weapon's strike, had as it is chosen: 1 optional press
    /// only usable to continue combat, once each combat per copy (Righteous
    /// Blade; kept though the strike is dodged or does not resolve,
    /// rules-qa 10.4).
    /// "Discard n cards at random" from the user's hand (a cost; the cards
    /// are replaced).
    DiscardAtRandom(u8),
    /// "+n strength that combat" (Kasim Bayar): added each round.
    StrengthThisCombat(u8),
    ContinuePressEachCombat,
    /// On success, search the library for an equipment the acting vampire
    /// can equip and equip it — requirements and cost as normal, shuffle
    /// afterward (Magic of the Smith; chosen at resolution, spec §1.14.1).
    SearchEquip,
    /// On success, search the library for a card of this kind, reveal it
    /// and move it to hand, or find none; shuffle afterward (Sakhar's
    /// equipment, Dominica's master archetype).
    SearchToHand(CardFilter),
    /// The printed action is mandatory while its requirements hold: taken
    /// before any other action, and a vampire that cannot take it is stuck
    /// and takes none (spec §3.9, Elen Kamjian).
    MandatoryAction,
    /// On success, the acting minion enters combat with the target, the
    /// combat part of the action (spec §4.1.2).
    EnterCombatWithTarget,
    /// On success, the target card moves to the acting minion (a steal:
    /// not played nor equipped, so no requirement is checked, spec
    /// §3.7.3.2).
    StealTargetCard,
    /// On success, the target card is burned.
    BurnTargetCard,
    /// On success, move up to n blood from the target vampire to the acting
    /// one — as much as there is (rulebook "Target") — the excess over its
    /// capacity draining off.
    StealBloodFromTarget(u8),
    /// The vampire does not unlock as normal during its next unlock phase.
    SkipNextUnlock,
    /// Exchange a card from hand for a library card in the ash heap
    /// (Lenelle): both named as the action is announced (LSJ 20000222),
    /// exchanged on success if both are still there.
    ExchangeHandForAsh,
    /// On success, equip with the first equipment found working down from
    /// the top — requirements and cost as normal, shuffle afterward (Vast
    /// Wealth): one that cannot be placed or paid for in blood is burned
    /// without cost (TOM 19950407); a pool cost that ousts is paid (RTR
    /// 19941109).
    EquipFirstFound,
    /// Move the top n cards of the controller's library onto this card,
    /// out of play — as many as there are (Shilmulo Tarot, on equip and in
    /// its unlock ability; rules-qa 11.2).
    StoreTopCards(u8),
    /// As the card resolves, once it is replaced (rules-qa 12.4), search the
    /// library (shuffle afterward) for up to n non-unique equipment cards,
    /// one at a time, and put them on this card; the card enters contest
    /// only then (spec §1.13.2), and is burned at once if it holds none
    /// (Black Market Cache; rules-qa 11.3). Applied by `resolve_play`.
    SearchStore(u8),
    /// Rider of a draw alternative: "if this card has no cards on it, burn
    /// it", checked as its last card is drawn (Black Market Cache).
    BurnSelfWithoutStored,
    /// "Draw up to X cards without discarding and then move the same number
    /// of cards from your hand to the bottom of your library": X capped by
    /// the library as the ability resolves (Heart of Nizchetus; rules-qa
    /// 9.5, 11.1).
    DrawThenBottom,
    // Referendum effects (`referendum`).
    /// The political action card's referendum (with `DeclareAction`).
    CallReferendum(&'static ReferendumDef),
    /// The actor gets +n votes in this referendum (cast on the same side as
    /// votes it already cast).
    AddVotes(u8),
    /// As `AddVotes`, +1 per ready vampire of this sect the player controls,
    /// counted as it resolves (Aisha az-Zahra).
    AddVotesPerReadySect(Sect),
    /// The target vampire abstains: its cast votes and ballots are
    /// canceled and it casts no more; `lock` locks it, `burn` burns blood.
    Abstain {
        lock: bool,
        burn: u8,
    },
    /// The actor gets +n votes against the referendum (and votes against).
    AddVotesAgainst(u8),
    /// Methuselahs casting votes or ballots against burn n pool once the
    /// results are tallied (Scorn of Adonis; once per Scorn per Methuselah).
    TallyPenaltyAgainst(u8),
    /// Non-Camarilla vampires cannot cast votes or ballots (Closed Session).
    BarNonCamarillaVotes,
    /// Cancel the referendum: no tally; the political action card returns
    /// to its owner's hand (discard down afterward). `unlock_acting`:
    /// unlock the acting vampire; `forbid_repeat`: the acting Methuselah's
    /// minions cannot perform the same political action again this turn.
    CancelReferendum {
        unlock_acting: bool,
        forbid_repeat: bool,
    },
    /// After a passed referendum: the actor gains 1 blood per vote of the
    /// margin, X of them to the player's pool instead (Voter Captivation).
    GainBloodPerMargin,
    /// Reduce the current bleed by n (never below 0).
    ReduceBleed(u8),
    /// Allies and vampires younger than the actor get this intercept
    /// modifier for the rest of the action (Perfect Paragon [PRE]).
    YoungerIntercept(i8),
}

/// Trigger points where riders of cards in play fire (effects-framework
/// §4: mandatory effects in canonical order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hook {
    /// "During their unlock phase": a mandatory effect the active
    /// Methuselah resolves among their own unlock-phase effects, in the
    /// order they choose, and before the impulse leaves them (spec §2.4.4;
    /// rules-qa 6.1). Its condition is read when it resolves (spec §5.2.2,
    /// Anarch Revolt).
    UnlockPhase,
    /// The host of this card went to torpor (Guardian Angel; Fame), fired
    /// at once where the torpor happened (rules-qa 6.4).
    HostToTorpor,
    /// "During your discard phase" (The Coven): a mandatory effect fired as
    /// the impulse leaves the active Methuselah at the end of the phase.
    DiscardPhase,
    /// A ready minion was burned or sent to torpor (Tension in the Ranks),
    /// whoever controls it; a minion removed from the game with its ousted
    /// Methuselah is not burned (spec §1.10), nor is one burned from
    /// torpor or from a contest.
    ReadyMinionBurnedOrTorpor,
    /// The controller would replace a card (effects-framework §5
    /// `replacement`): the rider's effects happen instead of the draw
    /// (Visit from the Capuchin). Every replacement draw counts, whatever
    /// took the card out of hand (spec §6.8.3), a delayed one when it
    /// finally comes due included.
    WouldReplace,
    /// The card's host was blocked ("if this minion is blocked …, burn
    /// this vehicle"): applied where the block resolves, before combat or
    /// any other effect (ANK 20220116).
    HostBlocked,
    /// The controller would draw a card from their library: a draw
    /// alternative ("you can draw one of those cards instead") offered for
    /// each card drawn, while the library holds one (rules-qa 9.4). The
    /// rider's effects apply as a card is taken from this card.
    WouldDraw,
    /// An action targeting this vampire was announced (Evan Klein): fired
    /// as the as-announced window closes, before any block.
    ActionAnnouncedAtTarget,
    /// A minion in combat with this vampire left the ready region (Egidia
    /// Arrú): fired with `ReadyMinionBurnedOrTorpor`, even if the vampire
    /// left in the same event (advanced rules §6.4.4).
    CombatOpponentLeftReady,
    /// The referendum of this vampire's political action was canceled or
    /// failed (Cedrick Calhoun): applied as the action resolves, before the
    /// after-resolution window.
    OwnReferendumFailed,
    /// This vampire successfully hunted: blood was gained (Adisa; a hunt
    /// action gaining none is not a successful hunt, spec §3.7.2.1).
    SuccessfulHunt,
}

/// Whose turn a phase rider fires on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RiderScope {
    /// Every Methuselah's ("during their unlock phase": Anarch Revolt).
    Every,
    /// The controller's only ("during your unlock phase": Creeping
    /// Sabotage).
    Controller,
    /// Every other Methuselah's (Smiling Jack).
    Others,
}

/// A mandatory effect of a card in play, fired at its hook.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rider {
    pub hook: Hook,
    pub scope: RiderScope,
    /// Host predicates read when the rider is collected and again when it
    /// resolves ("if this vampire is in torpor", Fame); an unmet rider is
    /// not pending.
    pub requires: &'static [Requirement],
    pub effects: &'static [Effect],
}

impl RiderScope {
    pub fn applies(self, controller: SeatId, active: SeatId) -> bool {
        match self {
            RiderScope::Every => true,
            RiderScope::Controller => controller == active,
            RiderScope::Others => controller != active,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// When a card played with a "do not replace until" clause is replaced:
/// at the first moment the condition is met, ahead of any other effect
/// (spec §1.9.2). A canceled card is replaced at once (§1.9.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Until {
    /// "Until your next unlock phase": before cards unlock.
    NextUnlockPhase,
    /// "Until your next discard phase": as the phase begins.
    NextDiscardPhase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    /// In-play abilities (the same shape; actor is the host, or a ready
    /// unlocked minion declaring a granted action).
    pub abilities: &'static [ModeDef],
    /// Riders of the card in play: mandatory effects fired at a hook
    /// (effects-framework §4, `phase_trigger`).
    pub riders: &'static [Rider],
}

/// Behavior of a library card, if implemented. A card without a definition
/// is unplayable (but can be discarded); coverage grows as defs land.
pub fn lookup(card: u16) -> Option<&'static CardDef> {
    let name = LIBRARY[card as usize].name;
    defs::DEFS.iter().find(|d| d.name == name).copied()
}

/// Whom a vampire's printed modifier applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrintedTo {
    /// The vampire itself.
    Itself,
    /// The minion opposing it ("Brujah get +1 strength in combat with
    /// him").
    Opponent,
    /// The other minions its controller controls ("primogen you control
    /// get +1 stealth").
    OthersControlled,
}

/// How many times a printed modifier counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Per {
    One,
    /// "For each unique equipment attached to him" (uncontested).
    UniqueEquipmentOn,
}

/// A modifier printed on a vampire, read live by `GameState::printed_mod`
/// while the vampire is in play — in torpor included (LSJ 20011022), not
/// while contested.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Printed {
    pub stat: Stat,
    pub n: i8,
    pub to: PrintedTo,
    pub per: Per,
    pub requires: &'static [Requirement],
}

/// Behavior of a vampire's printed text (`crypt.rs` stays the generated
/// printed data: sect, title, disciplines).
#[derive(Debug)]
pub struct VampireDef {
    /// Printed name; matched against `crypt::CRYPT`.
    pub name: &'static str,
    pub printed: &'static [Printed],
    /// Abilities printed on the vampire, in the card vocabulary: the actor
    /// `Host` is the vampire itself.
    pub abilities: &'static [ModeDef],
    /// Mandatory effects printed on the vampire, fired at their hooks
    /// (host predicates read on the vampire).
    pub riders: &'static [Rider],
    /// The printed text not implemented yet, if any: a vampire lands with
    /// part of its text and ends with all of it (rules-qa 13.1).
    pub incomplete: Option<&'static str>,
}

/// Behavior of a crypt card, if its printed text is implemented. A vampire
/// without a definition plays with no text.
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
                    .find(|d| d.name == c.name)
                    .copied()
            })
            .collect()
    })[card as usize]
}

/// The card play under announcement / in its as-played window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayCtx {
    pub seat: SeatId,
    pub card: u16,
    pub mode: u8,
    /// The minion playing the card (modifiers, reactions, action cards).
    pub actor: Option<MinionId>,
    pub target: Target,
    pub x: u8,
    /// The card in play chosen by `Terms::BurnNamedInPlay`.
    pub burn: Option<CardId>,
}

/// Does `m` meet a mode's discipline tags? Lower case needs the discipline
/// at any level, upper case needs superior; a combo card needs every tag,
/// any other card needs one (its text lists alternatives).
pub fn has_disciplines(st: &GameState, m: MinionId, card: u16, tags: &[&str]) -> bool {
    let disc = st.minions[m.index()].card().disciplines;
    let tags: Vec<&&str> = tags
        .iter()
        .filter(|t| t.len() == 3 && t.chars().all(|c| c.is_ascii_alphabetic()))
        .collect();
    if tags.is_empty() {
        return true;
    }
    let has = |t: &str| {
        if t.chars().all(|c| c.is_ascii_uppercase()) {
            disc.contains(&t)
        } else {
            let upper = t.to_ascii_uppercase();
            disc.iter().any(|d| *d == t || *d == upper)
        }
    };
    if LIBRARY[card as usize].combo {
        tags.iter().all(|t| has(t))
    } else {
        tags.iter().any(|t| has(t))
    }
}

/// Evaluate a target-independent requirement for `seat` playing with
/// `actor`, given the as-played context and the current action.
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
    let side = combat.and_then(|c| actor.and_then(|m| c.side_of_minion(m)));
    let political = referendum.filter(|r| r.kind == RefKind::Political);
    match req {
        Requirement::Controlled
        | Requirement::BloodAtLeast(_)
        | Requirement::NotActingController
        | Requirement::NotSelf
        | Requirement::HasCastVotes
        | Requirement::NotAbstaining
        | Requirement::NotControlled
        | Requirement::TargetLocked
        | Requirement::TargetVampire
        | Requirement::TargetCard(_)
        | Requirement::TargetOnReadyMinion
        | Requirement::ActorMayHoldTarget => true, // target predicates
        Requirement::PlayedCardIsMaster => {
            played.is_some_and(|p| LIBRARY[p.card as usize].types.contains(&CardType::Master))
        }
        Requirement::PlayedByOtherSeat => played.is_some_and(|p| p.seat != seat),
        Requirement::NotOwnTurn => st.active != seat,
        Requirement::DirectedAtYou => action.is_some_and(|a| a.directed_seat(st) == Some(seat)),
        Requirement::IsBleed => action.is_some_and(|a| a.kind == ActionKind::Bleed),
        Requirement::BleedAmountAtLeast(n) => action.is_some_and(|a| a.bleed_amount(st) >= n),
        Requirement::BlocksDeclined => action.is_some_and(|a| a.state == ActionState::Declined),
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
        Requirement::MasterPhaseActionLeft => st.seats[seat.index()].mpa > 0,
        Requirement::HasEdge => st.seats[seat.index()].has_edge,
        Requirement::HandNotEmpty => !st.seats[seat.index()].hand.is_empty(),
        Requirement::PoolAtLeast(n) => st.seats[seat.index()].pool >= n as i32,
        Requirement::ActorVampire => actor.is_some(),
        Requirement::HandHas(f, n) => st.hand_picks(seat, f).len() >= n as usize,
        Requirement::HostLocked
        | Requirement::HostBloodAtLeast(_)
        | Requirement::HostBelowCapacity
        | Requirement::HostControllerPoolAtLeast(_)
        | Requirement::HostInTorpor
        | Requirement::CountersAtLeast(_)
        | Requirement::CardUnlocked
        | Requirement::NotHuntingGroundFed => true, // host / card / target predicates
        Requirement::ControlsReadyTitled(classes) => st.minions_of(seat).any(|m| {
            let v = &st.minions[m.index()];
            v.ready() && classes.iter().any(|c| c.matches(v.title()))
        }),
        Requirement::ControlsReadySect(sects) => st.minions_of(seat).any(|m| {
            let v = &st.minions[m.index()];
            v.ready() && sects.contains(&v.card().sect)
        }),
        Requirement::RangeClose => combat.is_some_and(|c| c.range == Range::Close),
        Requirement::RangeLong => combat.is_some_and(|c| c.range == Range::Long),
        Requirement::FirstRound => combat.is_some_and(|c| c.round == 1),
        Requirement::NotFirstRound => combat.is_some_and(|c| c.round > 1),
        Requirement::BothCombatantsReady => combat.is_some_and(|c| c.both_ready(st)),
        Requirement::CombatWouldEnd => combat.is_some_and(|c| c.would_end()),
        Requirement::ActorReady => actor.is_some_and(|m| st.minions[m.index()].ready()),
        // The prevention half of a card needs damage to prevent; a mode
        // with another window (Indomitability [FOR]'s press) is unaffected
        // outside the prevention step.
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
        | Requirement::HostReady => true,
        Requirement::NoContinuePress => combat.is_some_and(|c| !c.press_continue),
        Requirement::ActorHasBlocked => {
            action.is_some_and(|a| a.blocker.is_some() && a.blocker == actor)
        }
        Requirement::ActorTitleIn(classes) => actor.is_some_and(|m| {
            let t = st.minions[m.index()].title();
            classes.iter().any(|c| c.matches(t))
        }),
        Requirement::ActorSectIn(sects) => {
            actor.is_some_and(|m| sects.contains(&st.minions[m.index()].card().sect))
        }
        Requirement::ActorCapacityAtLeast(n) => {
            actor.is_some_and(|m| st.minions[m.index()].capacity() >= n)
        }
        Requirement::IsPolitical => action.is_some_and(|a| a.kind == ActionKind::Political),
        Requirement::PollingStep => political.is_some_and(|r| r.step == RefStep::Polling),
        Requirement::AnyPollingStep => referendum.is_some_and(|r| r.step == RefStep::Polling),
        Requirement::ActorActing => action.zip(actor).is_some_and(|(a, m)| a.acting == m),
        Requirement::ActionSuccessful => action.is_some_and(|a| {
            matches!(
                a.outcome,
                Some(crate::actions::Outcome::Successful | crate::actions::Outcome::Fizzled)
            )
        }),
        Requirement::ActionDirected => action.is_some_and(|a| a.directed_seat(st).is_some()),
        Requirement::IsDiablerie => action.is_some_and(|a| a.kind == ActionKind::Diablerie),
        Requirement::ActorDiablerised => action
            .zip(actor)
            .is_some_and(|(a, m)| a.diablerist == Some(m)),
        Requirement::ActingOtherController => action.is_some_and(|a| a.seat != seat),
        Requirement::OtherActingControlled => {
            action.is_some_and(|a| a.seat == seat && actor != Some(a.acting))
        }
        Requirement::ActingSectIn(sects) => {
            action.is_some_and(|a| sects.contains(&st.minions[a.acting.index()].card().sect))
        }
        Requirement::DirectedHandNotEmpty => action
            .and_then(|a| a.directed_seat(st))
            .is_some_and(|t| !st.seats[t.index()].hand.is_empty()),
        // Target predicates (`holds_for_minion`).
        Requirement::ControlledByDirectedSeat | Requirement::NotHost => true,
        Requirement::YourAttemptNeedsIntercept => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => {
                st.minions[m.index()].owner == seat && a.stealth(st) > a.intercept_of(st, m)
            }
            _ => false,
        }),
        Requirement::OtherActingClan(c) => action.is_some_and(|a| {
            actor != Some(a.acting) && st.minions[a.acting.index()].card().clan == c
        }),
        // Host predicate (`holds_for_host`).
        Requirement::HostUnlocked => true,
        Requirement::OwnTurn => st.active == seat,
        Requirement::DiscardPhaseActionLeft => st.seats[seat.index()].dpa > 0,
        Requirement::MasterCardPlayed => st.seats[seat.index()].master_played,
        Requirement::AshNotEmpty => !st.seats[seat.index()].ash.is_empty(),
        Requirement::ActorCalledReferendum => {
            political.is_some() && action.zip(actor).is_some_and(|(a, m)| a.acting == m)
        }
        Requirement::NoVotesCast => political.is_some_and(|r| !r.any_cast()),
        Requirement::ReferendumPassed => action.is_some_and(|a| a.referendum_passed),
        Requirement::Younger => true, // target predicate
        Requirement::ActorNotVotingFor => {
            political.zip(actor).is_some_and(|(r, m)| !r.voting_for(m))
        }
        Requirement::ActorWithout(h) => actor.is_some_and(|m| !st.holds_a(m, h)),
        Requirement::HostClan(_) | Requirement::HostDiscipline(_) => true, // host predicates
        Requirement::ThisVampireReady
        | Requirement::IsDirected
        | Requirement::IsUndirected
        | Requirement::OpponentClan(_)
        | Requirement::OpponentYounger
        | Requirement::OpponentOlder
        | Requirement::OpponentTitled
        | Requirement::ActorClan(_)
        | Requirement::PoolAtMost(_)
        | Requirement::ControlsLocation
        | Requirement::ControlsNoLocation
        | Requirement::ControlsVampireNamed(_) => true, // printed-modifier predicates
        Requirement::ControlsLockedMinion => {
            st.minions_of(seat).any(|m| st.minions[m.index()].locked)
        }
        Requirement::BlockerTitleIn(classes) => action.and_then(|a| a.blocker).is_some_and(|b| {
            let t = st.minions[b.index()].title();
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
        Requirement::Controlled => v.owner == seat && v.present(),
        Requirement::NotControlled => v.owner != seat,
        Requirement::TargetLocked => v.locked,
        Requirement::TargetVampire => true,
        Requirement::BloodAtLeast(n) => v.blood >= n,
        Requirement::HasCastVotes => referendum.is_some_and(|r| r.has_cast(m)),
        Requirement::NotAbstaining => !referendum.is_some_and(|r| r.abstaining.contains(&m)),
        Requirement::ActorClan(c) => v.card().clan == c,
        Requirement::ActorReady => v.ready(),
        Requirement::ActorTitleIn(classes) => classes.iter().any(|c| c.matches(v.title())),
        Requirement::ActorSectIn(sects) => sects.contains(&v.card().sect),
        Requirement::NotHuntingGroundFed => !v.hunting_ground_fed,
        Requirement::Younger => {
            acting.is_some_and(|a| v.capacity() < st.minions[a.index()].capacity())
        }
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
        Requirement::NotHost => actor.is_none() || c.host != actor,
        Requirement::TargetCard(f) => f.matches(c.card),
        Requirement::TargetOnReadyMinion => c.host.is_some_and(|h| st.minions[h.index()].ready()),
        Requirement::ActorMayHoldTarget => actor.is_some_and(|m| st.can_hold(m, c.card, &[])),
        _ => true,
    }
}

/// Evaluate a predicate on the card in play itself (an in-play ability's
/// "lock this card", a rider's counters).
pub fn holds_for_card(req: Requirement, st: &GameState, id: CardId) -> bool {
    let c = &st.cards[id.index()];
    match req {
        Requirement::CardUnlocked => !c.locked,
        Requirement::CountersAtLeast(n) => c.counters >= n,
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
        Requirement::HostBloodAtLeast(n) => v.blood >= n,
        Requirement::HostBelowCapacity => v.blood < v.capacity(),
        Requirement::HostControllerPoolAtLeast(n) => st.seats[v.owner.index()].pool >= n as i32,
        Requirement::HostInTorpor => v.in_play && v.in_torpor,
        Requirement::HostClan(c) => v.card().clan == c,
        Requirement::HostDiscipline(d) => has_discipline(v.card().disciplines, d),
        _ => true,
    }
}

/// The value of X for terms the game state determines (no decision):
/// `CopiesControlled` counts the seat's copies of the card in play.
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
        _ => None,
    }
}

/// Pool cost for `payer` of playing `card` on `target`: base cost folded
/// through the cost layers (spec §1.7.2: cost arithmetic; §1.15: copies add).
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

/// Blood cost of playing `card`, paid by the playing minion at resolution:
/// base cost folded through the action-scoped cost layers (Consign to
/// Oblivion: reaction cards cost +1 blood).
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
