//! Card behavior: the agreed representation (DESIGN §4.5,
//! docs/effects-framework.md). A card is a set of modes; a mode's *legality
//! half* (window, actor, requirements, cost, targets, terms, limit) is a
//! closed vocabulary evaluated here by the engine, so the option mask is
//! enumerated from data; its *execution half* (effects, riders) is
//! interpreted at resolution. Definitions live in `defs`.

pub mod defs;

use crate::ids::{MinionId, SeatId};
use crate::library::{Cost, LIBRARY};
use crate::state::{CardId, GameState};

/// Play windows: the names of data/window-matrix-reviewed.csv.
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
    Methuselah,
    ActingMinion,
    ReactingMinion,
    Combatant,
    Bearer,
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
    /// The card being played (as-played context) is a master card.
    PlayedCardIsMaster,
    /// The card being played was played by another Methuselah.
    PlayedByOtherSeat,
    /// Playable only on another Methuselah's turn (out-of-turn masters).
    NotOwnTurn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetKind {
    Minion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetSpec {
    pub kind: TargetKind,
    pub filter: &'static [Requirement],
}

/// Numeric terms chosen as the card is played (rules-qa 3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terms {
    /// An integer in `min..=max`, additionally capped by the target's blood.
    BloodRange { min: u8, max: u8 },
}

/// Continuous-effect modifiers carried by layers (effects-framework §3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modifier {
    /// Cards with this printed name cost `pool` more (or less) to play;
    /// with `on_host_only`, only when played on the layer source's host.
    CostMod {
        name: &'static str,
        on_host_only: bool,
        pool: i8,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// While the source card stays in play.
    InPlay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layer {
    pub scope: Scope,
    pub modifier: Modifier,
    pub source: CardId,
}

impl Layer {
    pub fn hash(&self) -> u64 {
        let (n, h, p) = match self.modifier {
            Modifier::CostMod {
                name,
                on_host_only,
                pool,
            } => (name.len() as u64, on_host_only as u64, pool as i64 as u64),
        };
        (self.source.0 as u64) << 40 ^ n << 20 ^ h << 8 ^ p
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
    /// Cancel the card being played; `refund`: its cost is not paid.
    CancelPlayed { refund: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModeDef {
    pub window: Window,
    pub actor: Actor,
    pub requires: &'static [Requirement],
    pub target: Option<TargetSpec>,
    pub terms: Option<Terms>,
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
    /// Stays in play after resolution (masters put in play / on a minion).
    pub permanent: bool,
    pub modes: &'static [ModeDef],
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
    pub target: Option<MinionId>,
    pub x: u8,
}

/// Evaluate a target-independent requirement.
pub fn holds(req: Requirement, st: &GameState, seat: SeatId, played: Option<&PlayCtx>) -> bool {
    match req {
        Requirement::Controlled | Requirement::BloodAtLeast(_) => true, // target predicates
        Requirement::PlayedCardIsMaster => played.is_some_and(|p| {
            LIBRARY[p.card as usize]
                .types
                .contains(&crate::library::CardType::Master)
        }),
        Requirement::PlayedByOtherSeat => played.is_some_and(|p| p.seat != seat),
        Requirement::NotOwnTurn => st.active != seat,
    }
}

/// Evaluate a requirement against a candidate minion target.
pub fn holds_for(req: Requirement, st: &GameState, seat: SeatId, m: MinionId) -> bool {
    let v = &st.minions[m.index()];
    match req {
        Requirement::Controlled => v.owner == seat && v.present(),
        Requirement::BloodAtLeast(n) => v.blood >= n,
        _ => true,
    }
}

/// Pool cost of playing `card` on `target`, base cost folded through the
/// cost layers (spec §1.7.2: cost arithmetic; §1.15: copies add).
pub fn pool_cost(st: &GameState, card: u16, target: Option<MinionId>) -> i32 {
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
            pool,
        } = l.modifier;
        if name != def.name {
            continue;
        }
        if on_host_only && st.cards[l.source.index()].host != target {
            continue;
        }
        cost += pool as i32;
    }
    cost.max(0)
}
