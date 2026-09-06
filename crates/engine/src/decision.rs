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
    /// Call a referendum with a political action card (rulebook "Political
    /// Action": +1 stealth, undirected; one per minion per turn).
    Political,
    /// An action card whose effects resolve on success (put in play, …);
    /// its stealth is the card's own ("+1 stealth action").
    Card,
    /// Burn a card in play, granted by that card ("Minions can burn this
    /// card as a Ⓓ action"): directed at its controller.
    BurnCard,
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
    /// Use an in-play ability: arena index of the card, ability index, and
    /// the minion using it when the ability is a granted action (a card in
    /// play's "can call a referendum … as a political action").
    UseAbility {
        card: u32,
        ability: u8,
        actor: Option<MinionId>,
    },
    /// Resolve a mandatory effect of a card in play (arena index) pending
    /// at this hook — the player orders them among their own effects.
    ResolveRider {
        card: u32,
    },
    /// Terms of a play: burn this card in play (arena index) at resolution
    /// — "you can burn a card named X (if any)"; `Pass` for none.
    BurnInPlay {
        card: u32,
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
    /// Polling commitment: the political action card that called the
    /// referendum provides 1 vote for the acting Methuselah (rulebook
    /// "Gaining Votes").
    CastCardVote {
        favor: bool,
    },
    /// Polling commitment: burn a political action card from hand (hand
    /// index) for 1 vote; a Methuselah gains at most 1 vote from political
    /// action cards per referendum.
    BurnForVote {
        card: u8,
        favor: bool,
    },
    /// Referendum terms: allocate `n` points to this Methuselah (stepwise,
    /// one seat at a time; rules-qa 3.5).
    AllocateTo {
        seat: SeatId,
        n: u8,
    },
    /// Referendum terms: choose this Methuselah.
    ChooseSeat(SeatId),
    /// Referendum terms: choose this minion.
    ChooseMinion(MinionId),
    /// Referendum terms: choose this clan.
    ChooseClan(&'static str),
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
    /// Combat, choose strike: the default hand strike (rulebook "Strike
    /// Effects").
    HandStrike,
    /// Combat, determine range: use a maneuver banked this round.
    UseManeuver,
    /// Combat, press step: use a press banked this round. Its direction
    /// follows from the state: continue, or end the opponent's press to
    /// continue (rulebook "Press").
    UsePress,
    /// After combat: continue the action as if unblocked (Form of Mist
    /// [PRO]), paying the rider's blood.
    ContinueAction,
    /// A blocked leave-torpor: the blocking vampire diablerises the acting
    /// vampire (rulebook "Leave Torpor").
    Diablerise,
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
    /// After resolution of a successful action (Voter Captivation, Freak
    /// Drive); for a political action, after the referendum.
    ActionAfterResolution,
    /// Blocks declined by all (rulebook state C): redirects, Archon
    /// Investigation, remaining modifiers.
    ActionBlocksDeclined,
    /// After combat (after *all* combats of the block, rules-qa 2.1):
    /// unlock reactions of the blocker, the Form of Mist continuation.
    ActionAfterCombat,
    /// A blocked leave-torpor: the blocker's diablerie opportunity.
    TorporBlock,
    RescueCost,
    /// The blood hunt: polling only, no cards (rulebook "The Blood Hunt").
    BloodHuntPolling,
    /// Terms of a political referendum, chosen by the caller once the
    /// action succeeds (rules-qa 3.5).
    ReferendumTerms,
    /// The polling step of a political referendum: votes and ballots
    /// (unordered commitments) and card plays (rulebook "The Referendum").
    ReferendumPolling,
    /// A canceled referendum returned the political action card to hand:
    /// "discard down afterward".
    DiscardDown,
    InfluenceTransfers,
    DiscardPhase,
    /// Combat steps (rulebook "Combat Sequence"): ordered windows over the
    /// two combatant seats, the acting minion's controller first.
    CombatBeforeRange,
    CombatDetermineRange,
    CombatBeforeStrikes,
    /// One combatant chooses its strike (acting first, then the opponent).
    CombatStrike,
    /// Both strikes chosen, before they resolve (spec §4.3.2).
    CombatBeforeResolution,
    /// Damage prevention by the minion taking damage (acting side first).
    CombatPrevention,
    /// A partial prevention of mixed damage: how many aggravated points it
    /// covers (`SetX`; rules-qa 4.2).
    CombatPreventionAllocation,
    /// Additional strikes after the normal pair.
    CombatAdditionalStrikes,
    CombatPress,
    /// End of round, shared with the would-end replacement point (spec
    /// §4.9.1).
    CombatEndOfRound,
}

#[derive(Clone, Debug)]
pub struct DecisionRequest {
    pub seat: SeatId,
    pub window: WindowLabel,
    pub options: Vec<Choice>,
}
