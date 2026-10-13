//! Identifiers and the closed sets a client branches on.

use serde::{Deserialize, Serialize};

/// A seat, in seating order from 0.
pub type Seat = u8;

/// A card copy, as one viewer knows it. Numbered places only: cards on
/// the table (minions, cards in play and stored on them, uncontrolled
/// vampires), face-up piles (ash heaps, removed, limbo) and the viewer's
/// own hand. A card in a hidden place — another seat's hand, any library
/// or crypt — has no number. A card coming into a numbered place from a
/// hidden one takes a fresh number, never one used before for this
/// viewer: a number tells nothing the card's face does not. Numbers
/// differ between viewers; records use the engine's own (one per card,
/// in decklist order).
pub type CopyId = u32;

pub type KrcgId = u32;

/// A decision's number: the decisions taken so far in the game, every
/// seat's. Answers name it; a stale one is refused.
pub type DecisionNo = u64;

/// A closed set of snake_case ids, read tolerantly: a value a later host
/// adds reads as `unknown` (a new id is a minor protocol version).
macro_rules! ids {
    ($(#[$m:meta])* $name:ident { $($(#[$vm:meta])* $v:ident),* $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($(#[$vm])* $v,)*
            /// A value this reader does not know.
            #[serde(other)]
            Unknown,
        }
    };
}
pub(crate) use ids;

ids!(
    /// The active Methuselah's phase.
    Phase {
        Unlock,
        Master,
        Minion,
        Influence,
        Discard,
        /// The game is over.
        Over,
    }
);

ids!(
    /// The window a decision is asked in.
    Window {
        UnlockAbilities,
        MasterPhase,
        /// Terms of a play or an in-play ability, chosen as it is announced.
        PlayTerms,
        /// Choices a mandatory rider needs as it resolves.
        RiderTerms,
        /// The end of one of the active Methuselah's phases: the other
        /// seats' untimed effects.
        PhaseImpulse,
        /// A card's as-played window: cancels (and wakes, in an action).
        CardAsPlayed,
        MinionPhase,
        ActionAsAnnounced,
        /// Block attempts, modifiers, reactions.
        ActionBlock,
        ActionAfterResolution,
        ActionWouldBeSuccessful,
        ActionBlocksDeclined,
        ActionBlocked,
        ActionAfterCombat,
        TorporBlock,
        RescueCost,
        EntryEffect,
        /// A choice made as an effect resolves.
        ResolutionChoice,
        BlockToll,
        Distribution,
        BloodHuntPolling,
        ReferendumTerms,
        ReferendumPolling,
        DiscardDown,
        ActionTerms,
        LibrarySearch,
        ActionExchange,
        Draw,
        BottomCards,
        InfluenceTransfers,
        DiscardPhase,
        CombatBeforeRange,
        CombatDetermineRange,
        CombatBeforeStrikes,
        CombatStrike,
        CombatBeforeResolution,
        CombatPrevention,
        CombatPreventionAllocation,
        CombatAdditionalStrikes,
        CombatPress,
        CombatEndOfRound,
    }
);

ids!(ActionKind {
    Bleed,
    Hunt,
    LeaveTorpor,
    Rescue,
    Diablerie,
    Employ,
    Recruit,
    Political,
    /// An action card's action (not one of the kinds above).
    Card,
    BurnCard,
    StealCard,
    Equip,
    EquipFrom,
    EquipFound,
    /// An action granted by a card in play or a vampire's ability.
    Ability,
    BecomeAnarch,
    /// Enter combat with a Red List minion.
    RedListHunt,
});

ids!(
    /// Where an action stands.
    ActionStep {
        Announced,
        /// Blocks may be attempted.
        Open,
        /// A block is being attempted.
        Attempt,
        /// Blocks declined by all.
        Declined,
        WouldSucceed,
        Blocked,
        AfterCombat,
        Resolved,
        /// A diablerie after the action (the diablerist's choices).
        Diablerie,
    }
);

ids!(MinionKind {
    Vampire,
    Ally,
    /// A library card as a vampire.
    Created,
    Token,
});

ids!(
    /// A seat's zone.
    Zone {
        Crypt,
        Library,
        Hand,
        AshHeap,
        Removed,
        Uncontrolled,
        Ready,
        Torpor,
        /// In play on no minion.
        InPlay,
        /// A card being played, not resolved yet.
        Limbo,
    }
);

ids!(Range { Close, Long });

ids!(StrikeKind {
    Hand,
    Melee,
    Dodge,
    CombatEnds,
    Damage,
    StealBlood,
    TorporOrBurn,
    Torpor,
    /// A strike whose effect is its card's text.
    CardEffect,
    BurnWeapon,
    BurnEquipment,
    RangedBurnEquipment,
    Corrupt,
});

ids!(ReferendumStep {
    Terms,
    Polling,
    Shortfall,
    /// A choice the passed referendum's effect asks.
    Choose,
});

ids!(
    /// Where votes come from.
    VoteSource {
        /// A vampire casting its votes.
        Vampire,
        /// Votes printed on a minion that does not cast them itself.
        Printed,
        Edge,
        /// A political action card's vote.
        Card,
        /// A card in play's votes.
        InPlay,

        /// Pool burned for votes.
        Pool,    }
);

ids!(
    /// What an option does (the engine's choice kinds).
    OptionKind {
        Pass,
        EdgePool,
        PayContest,
        YieldContest,
        Play,
        UseAbility,
        ResolveRider,
        ResolvePrinted,
        BurnInPlay,
        BurnPool,
        BurnBloodFrom,
        SetX,
        Discard,
        FindCard,
        TakeFrom,
        Slot,
        AshVampire,
        DrawLibrary,
        DrawStored,
        Bottom,
        TakeEquipment,
        TakeAction,
        DeclareBlock,
        PayRescue,
        CastVotes,
        CastEdge,
        CastCardVote,
        BurnForVote,
        AllocateTo,
        ChooseSeat,
        ChooseMinion,
        ChooseClan,
        ChooseCircle,
        ChooseDiscipline,
        ChooseCard,
        AllocateToCard,
        Transfer,
        Withdraw,
        DrawCrypt,
        InfluenceOut,
        Merge,
        HandStrike,
        UseManeuver,
        UsePress,
        UsePrevent,
        PayBlockToll,
        PayToCancel,
        StrikeCombatEnds,
        RoundStrike,
        TakeDamageAfterResolution,
        CommittedStrike,
        BurnWeaponStrike,
        EmployFromHand,
        BurnBloodForIntercept,
        UnlockForBlood,
        UnlockInfernal,
        ContinueAction,
        GainStrength,
        BurnBloodToAvoid,
        BurnCorruptionToFail,
        BurnCorruptionToUnlock,
        Diablerise,
    }
);

ids!(
    /// What a chosen minion or Methuselah becomes for the card choosing it
    /// (completed with the host, A3).
    Role {
        /// Does not unlock as normal (Toreador Grand Ball's first).
        DoesNotUnlock,
        /// Its non-bleed actions cannot be blocked (the Grand Ball's second).
        UnblockableNonBleed,
        /// The card's chosen minion, for its later effects.
        Chosen,
    }
);

ids!(
    /// Public state on a minion beyond blood and cards (completed with
    /// the host, A3).
    MarkerKind {
        /// Corruption counters (`seat`: whose, `n`).
        Corruption,
        Hostage,
        Nightmare,
        Torture,
        Disease,
        TitleContested,
        Expelled,
        /// A card's lasting effect on it (`card` says which, `n` its
        /// counters).
        Effect,
    }
);
