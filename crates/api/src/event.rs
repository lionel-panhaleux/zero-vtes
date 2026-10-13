//! What happened, told to a seat: decisions taken and their public
//! consequences, the engine's elided steps included. Everything an event
//! says is public to its recipient.

use serde::{Deserialize, Serialize};

use crate::ids::{CopyId, DecisionNo, Range, Seat, Window, Zone};
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
        seat: Seat,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on: Option<CopyId>,
    },
    /// A card in play moves to another minion (or off one).
    Moves {
        card: Card,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on: Option<CopyId>,
    },
    /// A minion or a card leaves play, to a zone when public.
    LeavesPlay {
        card: Card,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<Zone>,
    },
    Torpor {
        minion: CopyId,
        /// Goes to torpor (true) or leaves it.
        enters: bool,
    },
    Ousted {
        seat: Seat,
    },
    BlockFails {
        minion: CopyId,
    },
    ActionEnds {
        seat: Seat,
        acting: CopyId,
        outcome: ActionOutcome,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        blocker: Option<CopyId>,
    },
    ReferendumEnds {
        outcome: ReferendumOutcome,
        votes_for: u32,
        votes_against: u32,
    },
    CombatBegins {
        minions: [CopyId; 2],
    },
    StrikeDeclared {
        minion: CopyId,
        strike: Strike,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ActionOutcome {
    Succeeds,
    Blocked,
    Fails,
    Fizzles,
    /// Ended otherwise (its acting minion gone, its Methuselah ousted).
    Ends,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ReferendumOutcome {
    Passes,
    Fails,
    Canceled,
}
