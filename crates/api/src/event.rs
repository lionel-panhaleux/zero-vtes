//! What happened, told to a seat: decisions taken and their public
//! consequences, the engine's elided steps included. Everything an event
//! says is public to its recipient.

use serde::{Deserialize, Serialize};

use crate::ids::{ActionKind, CopyId, DecisionNo, MinionKind, Range, Seat, Window, Zone};
use crate::option::OptionView;
use crate::view::{Card, Strike};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Event {
    #[serde(flatten)]
    pub kind: EventKind,
    /// The event told in English, when the seat's settings ask for text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventKind {
    /// A decision taken, by any seat, as the recipient may know it.
    Decided {
        decision: DecisionNo,
        seat: Seat,
        window: Window,
        choice: OptionView,
    },
    /// A decision passed by itself (the seat's settings left only a pass).
    AutoPassed {
        decision: DecisionNo,
        seat: Seat,
        window: Window,
    },
    /// The seat took back its decisions from `to` on: the client drops
    /// what it showed since (the next view is the table at `to`).
    Undone {
        seat: Seat,
        to: DecisionNo,
    },
    Turn {
        turn: u32,
        active: Seat,
    },
    Draw {
        seat: Seat,
        n: u8,
        crypt: bool,
    },
    Pool {
        seat: Seat,
        delta: i32,
        pool: i32,
    },
    Blood {
        minion: CopyId,
        delta: i32,
        blood: u8,
    },
    /// A minion or a card comes into play (on a minion, or not).
    EntersPlay {
        card: Card,
        /// Its controller.
        seat: Seat,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on: Option<CopyId>,
        /// A minion, of this kind (not a card on the table).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        minion: Option<MinionKind>,
        /// A lasting effect whose card is in the ash heap (a marker on
        /// its minion), not a card of its own: no copy.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        effect: bool,
    },
    /// A card in play moves to another minion (or off one).
    Moves {
        card: Card,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on: Option<CopyId>,
        /// A lasting effect whose card is in the ash heap (a marker on
        /// its minion), not a card of its own: no copy.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        effect: bool,
    },
    /// A minion or a card leaves play: to `to` when it went to a seat's
    /// zone (a burned minion to its ash heap), contested (a vampire set
    /// aside while its Methuselahs contest it).
    LeavesPlay {
        card: Card,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<Zone>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        minion: Option<MinionKind>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        contested: bool,
        /// A lasting effect whose card is in the ash heap (a marker on
        /// its minion), not a card of its own: no copy.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        effect: bool,
    },
    Torpor {
        minion: CopyId,
        /// Goes to torpor (true) or leaves it.
        enters: bool,
    },
    Ousted {
        seat: Seat,
    },
    /// The Edge goes to `seat` (absent: burned, no one holds it).
    Edge {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        seat: Option<Seat>,
    },
    BlockFails {
        minion: CopyId,
    },
    ActionEnds {
        seat: Seat,
        acting: CopyId,
        action: ActionRef,
        outcome: ActionOutcome,
        /// The minion that blocked it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        blocker: Option<CopyId>,
    },
    ReferendumEnds {
        outcome: ReferendumOutcome,
        votes_for: u32,
        votes_against: u32,
        /// The political action it was called by.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        action: Option<ActionRef>,
        /// A blood hunt's: the diablerist hunted.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hunted: Option<CopyId>,
    },
    CombatBegins {
        minions: [CopyId; 2],
    },
    /// A combatant's strike declared (the acting minion's first: the
    /// blocker chooses knowing it); `None`: no strike.
    StrikeDeclared {
        minion: CopyId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        strike: Option<Strike>,
    },
    /// A pair of strikes resolved: each side's strike and the damage it
    /// dealt before prevention.
    Strikes {
        round: u8,
        pair: u8,
        range: Range,
        sides: Vec<StruckSide>,
    },
    CombatEnds,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StruckSide {
    pub minion: CopyId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strike: Option<Strike>,
    pub damage: u8,
}

/// What an action is: its kind, its card, what grants it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ActionRef {
    pub kind: ActionKind,
    /// The action card, for a card action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<Card>,
    /// What grants the action: a card in play, or another vampire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Card>,
}

crate::ids::ids!(ActionOutcome {
    Succeeds,
    Blocked,
    Fails,
    Fizzles,
    /// Its Methuselah was ousted.
    Ousted,
    /// Ended otherwise (its acting minion gone).
    Ends,
});

crate::ids::ids!(ReferendumOutcome {
    Passes,
    Fails,
    Canceled,
});
