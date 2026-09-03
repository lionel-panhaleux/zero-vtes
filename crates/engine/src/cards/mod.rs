//! Card behavior: the agreed representation (DESIGN §4.5,
//! docs/effects-framework.md). A card is a set of modes; a mode's *legality
//! half* (windows, actor, requirements, cost, targets, terms) is a closed
//! vocabulary evaluated here by the engine, so the option mask is enumerated
//! from data; its *execution half* (effects) is interpreted at resolution.
//! Definitions live in `defs`.

pub mod defs;

use crate::actions::{ActionCtx, ActionState};
use crate::decision::ActionKind;
use crate::ids::{MinionId, SeatId};
use crate::library::{CardType, Cost, LIBRARY};
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
    /// The host of the card in play is locked.
    HostLocked,
    /// The host of the card in play has at least this much blood.
    HostBloodAtLeast(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetKind {
    Minion,
    Seat,
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
}

/// Numeric terms chosen as the card is played (rules-qa 3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terms {
    /// An integer in `min..=max` (the amount actually taken at resolution
    /// is capped by what is there: Villein rulings).
    BloodRange { min: u8, max: u8 },
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
        };
        (self.source.0 as u64) << 40 ^ (self.scope as u64) << 36 ^ m
    }
}

/// Effect primitives (effects-framework §3.1); `X` is the play's terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Put this card on the target minion.
    PutOnTarget,
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
    /// Playable modes (from hand).
    pub modes: &'static [ModeDef],
    /// In-play abilities (the same shape; actor is the host or controller).
    pub abilities: &'static [ModeDef],
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
pub fn holds(
    req: Requirement,
    st: &GameState,
    seat: SeatId,
    actor: Option<MinionId>,
    played: Option<&PlayCtx>,
    action: Option<&ActionCtx>,
    waives_needed: bool,
) -> bool {
    match req {
        Requirement::Controlled
        | Requirement::BloodAtLeast(_)
        | Requirement::NotActingController
        | Requirement::NotSelf => true, // target predicates
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
                    matches!(a.state, ActionState::Attempt(_)) && a.intercept >= a.stealth()
                })
        }
        Requirement::InterceptNeeded => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => actor == Some(m) && a.stealth() > a.intercept,
            _ => false,
        }),
        Requirement::LockedActor => actor.is_some_and(|m| st.minions[m.index()].locked),
        Requirement::HostLocked | Requirement::HostBloodAtLeast(_) => true, // host predicates
    }
}

/// Evaluate a requirement against a candidate minion target.
pub fn holds_for_minion(req: Requirement, st: &GameState, seat: SeatId, m: MinionId) -> bool {
    let v = &st.minions[m.index()];
    match req {
        Requirement::Controlled => v.owner == seat && v.present(),
        Requirement::BloodAtLeast(n) => v.blood >= n,
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

/// Evaluate a host predicate for an in-play ability.
pub fn holds_for_host(req: Requirement, st: &GameState, host: MinionId) -> bool {
    let v = &st.minions[host.index()];
    match req {
        Requirement::HostLocked => v.locked,
        Requirement::HostBloodAtLeast(n) => v.blood >= n,
        _ => true,
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
    for l in &st.layers {
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
pub fn blood_cost(st: &GameState, card: u16) -> u8 {
    let def = &LIBRARY[card as usize];
    let mut cost = match def.blood_cost {
        Cost::None => 0,
        Cost::Fixed(n) => n as i32,
        Cost::X => 0,
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
