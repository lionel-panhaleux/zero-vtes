//! What a seat receives.

use serde::{Deserialize, Serialize};

use crate::event::Event;
use crate::ids::{CopyId, DecisionNo, Purpose, Seat, Window};
use crate::option::{Allocation, CardRef, OptionView, VoteTotals};
use crate::spec::Settings;
use crate::view::TableView;

/// A message as sent: numbered per seat from 1, so a client rejoining
/// `after` the last it received gets the rest.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Out {
    pub seq: u64,
    /// The client id of the command it answers (an `error`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<u64>,
    #[serde(flatten)]
    pub message: Message,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    Joined(Joined),
    /// What one step of the game did, in order: a decision taken (its
    /// first event: `decided`, `auto_passed` or `undone`) and what
    /// followed until the next decision; or, once, the deal. With the
    /// table after it when the seat's settings ask views on every step.
    Events {
        events: Vec<Event>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        view: Option<TableView>,
    },
    /// The seat must decide; sent again on `join` and after a settings
    /// change while pending.
    Decision(Decision),
    Over(Over),
    /// A command refused; the game is unchanged.
    Error(Error),
    /// The seat's `feedback`, recorded at this decision.
    Noted {
        decision: DecisionNo,
    },
}

/// The answer to `join`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Joined {
    /// `PROTOCOL_VERSION` of the host.
    pub protocol: String,
    /// The engine's version: its commit.
    pub engine: String,
    pub game: String,
    pub seat: Seat,
    pub seats: Vec<SeatInfo>,
    pub settings: Settings,
    /// Seconds without a command before the game ends as abandoned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idle_timeout: Option<u32>,
    pub view: TableView,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SeatInfo {
    pub name: String,
    /// The bot profile; absent for a human.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bot: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Decision {
    pub decision: DecisionNo,
    pub seat: Seat,
    pub window: Window,
    /// The card (or minion, by its card) whose terms or effect this
    /// decision chooses, when it is one: the options' `effects` say what
    /// each choice gets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<CardRef>,
    /// What an effect asks the choice for, when one does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<Purpose>,
    /// The particulars of the choice a prompt names, beyond its purpose,
    /// source and options: how many, whose, which minion or card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<About>,
    /// The options shown (the seat's settings may hide some: indices keep
    /// the engine's numbering).
    pub options: Vec<OptionView>,
    /// `Command::Undo` would take back the seat's previous decision.
    pub undo: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allocation: Option<Allocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub votes: Option<VoteTotals>,
    /// What is chosen, in English ("Camarilla's Iron Fist: choose the
    /// Methuselah who gains 1 pool"), when the seat's settings ask for text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    pub view: TableView,
}

/// What a choice is about (`Decision::about`): each field given where
/// the purpose (or the terms) has it (docs/protocol.md §5).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct About {
    /// How many: still to pay, at most, to discard, damage, blood…
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>,
    /// A second amount (the most a Methuselah burns, the damage blood
    /// avoids).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub m: Option<u8>,
    /// A capacity it depends on (the young vampires Neonate Breach spares).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap: Option<u8>,
    /// The Methuselah it concerns: whose minions, ash heap, hand,
    /// uncontrolled region.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seat: Option<Seat>,
    /// The minion it concerns: the one blood moves from, a block
    /// attempt's, the one entering play.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minion: Option<CopyId>,
    /// A second minion: the one blood moves to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<CopyId>,
    /// A card it names: in play (`copy`) or by name (`id`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<CardRef>,
    /// A clan, a circle (snake_case ids).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clan: Option<String>,
    /// What sets it apart within its purpose, a snake_case id
    /// (`minion_of`: `lock`, `lock_vampires`, `corrupt`…).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Over {
    /// Per seat, in halves of a victory point.
    pub vp_halves: Vec<u32>,
    /// The game's winner: the most victory points, at least 2 of them,
    /// alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub winner: Option<Seat>,
    pub reason: EndReason,
    pub view: TableView,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EndReason {
    /// One Methuselah left (or none).
    LastStanding,
    TimeLimit,
    /// The last human seat was ousted: the bots do not play on (victory
    /// points as they stand).
    HumansOusted,
    /// A seat left (`Command::Leave`) or went idle too long.
    Abandoned {
        seat: Seat,
    },
    /// The engine failed (or ran out of steps); the game is recorded with
    /// the failure.
    EngineError,
    /// The server restarted with the game going: recorded only (no client
    /// is told), victory points as its record reads on the restarted
    /// server's engine.
    ServerRestart,
    /// A reason a later host gives that this reader does not know (§14).
    /// Never sent.
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum Error {
    /// Not the pending decision (answered already, or a stale client).
    Stale {
        expected: DecisionNo,
    },
    /// The pending decision is another seat's, or none is pending.
    NotYourTurn,
    /// No option with that index.
    Option {
        index: u32,
    },
    /// Nothing to take back.
    NoUndo,
    /// No allocation under way, or amounts it does not allow.
    Allocation {
        reason: AllocationError,
    },
    /// No vote to cast that way.
    NoVotes,
    Over,
    /// `join` first, with a valid token.
    NotJoined,
    /// Not valid JSON, or not a command.
    Malformed {
        reason: String,
    },
    /// Too many commands: slow down.
    RateLimited,
    /// An error a later host sends that this reader does not know (§14).
    /// Never sent.
    #[serde(other)]
    Unknown,
}

crate::ids::ids!(
    /// Why an allocation is refused.
    AllocationError {
        /// No allocation under way.
        None,
        /// Not one amount per recipient.
        Length,
        /// The amounts do not add up to the points.
        Total,
        /// A recipient over its cap.
        Cap,
        /// Fewer recipients given points than required.
        MinNonzero,
    }
);

/// Why a game spec is refused (`POST /games`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum SpecError {
    Seats {
        n: u8,
    },
    /// Cards outside the engine's pool, by KRCG id.
    Unsupported {
        seat: Seat,
        cards: Vec<String>,
    },
    /// A crypt under 12 cards or a library under 60 (or over 90).
    DeckSize {
        seat: Seat,
        crypt: u32,
        library: u32,
    },
    /// A card id that is no card.
    UnknownCard {
        seat: Seat,
        id: String,
    },
    /// A bot profile the host does not offer.
    Profile {
        seat: Seat,
        profile: String,
    },
    /// Version 1 hosts one human seat per game, and no bot-only game.
    Humans {
        n: u8,
    },
    /// A time limit over the host's.
    TimeLimit {
        max: u32,
    },
}
