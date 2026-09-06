//! Card behavior: the agreed representation (DESIGN §4.5,
//! docs/effects-framework.md). A card is a set of modes; a mode's *legality
//! half* (windows, actor, requirements, cost, targets, terms) is a closed
//! vocabulary evaluated here by the engine, so the option mask is enumerated
//! from data; its *execution half* (effects) is interpreted at resolution.
//! Definitions live in `defs`.

pub mod defs;

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
    /// The player holds at least n master cards in hand ("discard two
    /// master cards").
    HandMasters(u8),
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
                                && filter
                                    .iter()
                                    .all(|&r| holds_for_minion(r, st, seat, None, Some(actor), m))
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetKind {
    Minion,
    Seat,
    /// A vampire in the player's uncontrolled region (slot index).
    Uncontrolled,
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
    /// Ability cost "discard n cards" (`masters`: master cards only):
    /// which ones, chosen one at a time as the use is announced (Tension
    /// in the Ranks, The Barrens).
    Discard { n: u8, masters: bool },
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
        };
        (self.source.0 as u64) << 40 ^ (self.scope as u64) << 36 ^ m
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
    UnlockBurnPoolUnlessReady { sect: Sect, pool: u8 },
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
    /// Add n counters to this card in play ("add 1 counter to this card
    /// each time you lock it", Dreams of the Sphinx).
    AddCounter(u8),
    /// Burn this card once it carries n counters ("burn this card if it
    /// has 3 counters"), checked as the ability resolves.
    BurnSelfAtCounters(u8),
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
    CancelPlayed { refund: bool },
    /// Declare an action of this kind with this card (action cards).
    DeclareAction(ActionKind),
    /// +n stealth for the current action.
    AddStealth(u8),
    /// +n intercept for the actor's current block attempt.
    AddIntercept(u8),
    /// +n bleed for the current bleed; `limited` binds the rulebook cap.
    BleedMod { n: u8, limited: bool },
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
    /// The card (a retainer) enters play on the acting minion with this life.
    EmployRetainer { life: u8 },
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
    AdditionalStrikes { n: u8, limited: bool },
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
    ContinueAfterCombat { blood: u8, stealth: u8 },
    // Referendum effects (`referendum`).
    /// The political action card's referendum (with `DeclareAction`).
    CallReferendum(&'static ReferendumDef),
    /// The actor gets +n votes in this referendum (cast on the same side as
    /// votes it already cast).
    AddVotes(u8),
    /// The target vampire abstains: its cast votes and ballots are
    /// canceled and it casts no more; `lock` locks it, `burn` burns blood.
    Abstain { lock: bool, burn: u8 },
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardDef {
    /// Printed name; matched against `library::LIBRARY`.
    pub name: &'static str,
    /// Trifle: refunds a master phase action (rulebook "Trifle").
    pub trifle: bool,
    /// Out-of-turn master card (rulebook "Out-of-turn"; rules-qa 3.3).
    pub out_of_turn: bool,
    /// "Do not replace until your next unlock phase" (Old Friends).
    pub delayed_replacement: bool,
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
        | Requirement::HasCastVotes => true, // target predicates
        Requirement::PlayedCardIsMaster => {
            played.is_some_and(|p| LIBRARY[p.card as usize].types.contains(&CardType::Master))
        }
        Requirement::PlayedByOtherSeat => played.is_some_and(|p| p.seat != seat),
        Requirement::NotOwnTurn => st.active != seat,
        Requirement::DirectedAtYou => action.is_some_and(|a| a.directed_at() == Some(seat)),
        Requirement::IsBleed => action.is_some_and(|a| a.kind == ActionKind::Bleed),
        Requirement::BleedAmountAtLeast(n) => action.is_some_and(|a| a.bleed_amount() >= n),
        Requirement::BlocksDeclined => action.is_some_and(|a| a.state == ActionState::Declined),
        Requirement::StealthNeeded => {
            waives_needed
                || action.is_some_and(|a| {
                    matches!(a.state, ActionState::Attempt(_))
                        && a.attempt_intercept(st) >= a.stealth()
                })
        }
        Requirement::InterceptNeeded => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => actor == Some(m) && a.stealth() > a.intercept_of(st, m),
            _ => false,
        }),
        Requirement::LockedActor => actor.is_some_and(|m| st.minions[m.index()].locked),
        Requirement::MasterPhaseActionLeft => st.seats[seat.index()].mpa > 0,
        Requirement::HasEdge => st.seats[seat.index()].has_edge,
        Requirement::HandNotEmpty => !st.seats[seat.index()].hand.is_empty(),
        Requirement::PoolAtLeast(n) => st.seats[seat.index()].pool >= n as i32,
        Requirement::ActorVampire => actor.is_some(),
        Requirement::HandMasters(n) => st.hand_picks(seat, true).len() >= n as usize,
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
        Requirement::NoVotesCast => political.is_some_and(|r| !r.any_cast()),
        Requirement::ReferendumPassed => action.is_some_and(|a| a.referendum_passed),
        Requirement::Younger => true, // target predicate
        Requirement::ActorNotVotingFor => {
            political.zip(actor).is_some_and(|(r, m)| !r.voting_for(m))
        }
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
) -> bool {
    let v = &st.minions[m.index()];
    match req {
        Requirement::Controlled => v.owner == seat && v.present(),
        Requirement::BloodAtLeast(n) => v.blood >= n,
        Requirement::HasCastVotes => referendum.is_some_and(|r| r.has_cast(m)),
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

/// Evaluate a predicate on the card in play itself (an in-play ability's
/// "lock this card", a rider's counters).
pub fn holds_for_card(req: Requirement, st: &GameState, id: CardId) -> bool {
    let c = &st.cards[id.index()];
    match req {
        Requirement::CardUnlocked => !c.locked,
        Requirement::CountersAtLeast(n) => c.counters >= n,
        _ => true,
    }
}

/// Evaluate a host predicate for an in-play ability.
pub fn holds_for_host(req: Requirement, st: &GameState, host: MinionId) -> bool {
    let v = &st.minions[host.index()];
    match req {
        Requirement::HostLocked => v.locked,
        Requirement::HostBloodAtLeast(n) => v.blood >= n,
        Requirement::HostBelowCapacity => v.blood < v.capacity(),
        Requirement::HostControllerPoolAtLeast(n) => st.seats[v.owner.index()].pool >= n as i32,
        Requirement::HostInTorpor => v.in_play && v.in_torpor,
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
