use crate::cards::Target;
use crate::ids::{MinionId, SeatId};

/// Action kinds: the card-free rulebook actions, plus card actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
    Bleed,
    Hunt,
    LeaveTorpor,
    /// Rescue a vampire from torpor (`target`); 2 blood, split at resolution.
    Rescue,
    /// Diablerise a vampire in torpor (`target`); triggers the blood hunt.
    Diablerie,
    /// Employ a retainer from hand (a card action).
    Employ,
}

/// One indexed option inside a decision request. `Pass` is always explicit;
/// its meaning is window-specific (decline to block, end phase, keep the
/// Edge, forfeit remaining transfers, cast no more votes this rotation).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Pass,
    BurnEdge,
    /// Unlock-phase contest upkeep: pay 1 pool (card) or 1 blood (title).
    PayContest,
    /// Unlock-phase contest: yield — the card is burned / the title is lost.
    YieldContest,
    /// Play a card from hand: hand index, mode index, the playing minion
    /// (modifiers, reactions, action cards), resolved target.
    Play {
        card: u8,
        mode: u8,
        actor: Option<MinionId>,
        target: Target,
    },
    /// Use an in-play ability: arena index of the card, ability index.
    UseAbility {
        card: u32,
        ability: u8,
    },
    /// Numeric terms of the play being announced (rules-qa 3.5).
    SetX(u8),
    /// Discard phase action: discard the card at this hand index.
    Discard(u8),
    TakeAction {
        minion: MinionId,
        action: ActionKind,
        /// Torpor vampire targeted by Rescue / Diablerie; `None` otherwise.
        target: Option<MinionId>,
    },
    DeclareBlock {
        minion: MinionId,
    },
    /// Rescue cost split: blood paid by the acting vampire (the rescued
    /// vampire pays the rest of the 2).
    PayRescue {
        from_acting: u8,
    },
    /// Polling commitment: a vampire source casts all its votes one way
    /// (rules-qa 1.1: sources are atomic and independent).
    CastVotes {
        minion: MinionId,
        favor: bool,
    },
    /// Polling commitment: burn the Edge for 1 vote (a Methuselah ballot).
    CastEdge {
        favor: bool,
    },
    /// 1 transfer: move 1 pool onto an uncontrolled vampire.
    Transfer {
        slot: u8,
    },
    /// 2 transfers: move 1 counter from an uncontrolled vampire back to pool.
    Withdraw {
        slot: u8,
    },
    /// 4 transfers and 1 pool: move the top crypt card to the uncontrolled
    /// region.
    DrawCrypt,
    /// Move a fully-funded uncontrolled vampire to the ready region.
    InfluenceOut {
        slot: u8,
    },
}

/// One name per legality filter (DESIGN §4.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowLabel {
    UnlockEdge,
    UnlockContest,
    /// In-play abilities usable during any Methuselah's unlock phase.
    UnlockAbilities,
    MasterPhase,
    /// Numeric terms of a play, chosen as it is announced.
    PlayTerms,
    /// The as-played window of a card: cancels (and wakes, during an action)
    /// only (rules-qa 2.4).
    CardAsPlayed,
    MinionPhase,
    /// "As the action is announced" plays.
    ActionAsAnnounced,
    /// The block loop (rulebook states A and B): attempts, modifiers,
    /// reactions.
    ActionBlock,
    /// Blocks declined by all (rulebook state C): redirects, Archon
    /// Investigation, remaining modifiers.
    ActionBlocksDeclined,
    RescueCost,
    BloodHuntPolling,
    InfluenceTransfers,
    DiscardPhase,
}

#[derive(Clone, Debug)]
pub struct DecisionRequest {
    pub seat: SeatId,
    pub window: WindowLabel,
    pub options: Vec<Choice>,
}
