//! A game as recorded: enough to replay it on the engine it ran on, and
//! to read it on a later one (each answer kept as its `ChoiceKey`).
//! Copy numbers here are the engine's (one per card, in decklist order),
//! not a viewer's.

use serde::{Deserialize, Serialize};

use crate::ids::{DecisionNo, Seat};
use crate::message::{EndReason, Error};
use crate::option::ChoiceKey;
use crate::spec::{GameSpec, Settings};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GameRecord {
    pub protocol: String,
    /// The engine's commit.
    pub engine: String,
    /// The spec as played: the seed always set, human seats' names
    /// replaced by "human".
    pub spec: GameSpec,
    /// Per bot seat, the profile resolved.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bots: Vec<BotRecord>,
    /// Unix milliseconds.
    pub started: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended: Option<u64>,
    pub entries: Vec<Entry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<GameResult>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BotRecord {
    pub seat: Seat,
    pub profile: String,
    /// "network" or a scripted style.
    pub kind: String,
    /// The weights' SHA-256, for a network.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weights: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Its sampling seed: replaying uses the recorded options, the seed
    /// re-simulates the bot.
    pub seed: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Entry {
    /// A decision answered (by a human, a bot, a standing order or a lone
    /// pass).
    Decision {
        decision: DecisionNo,
        seat: Seat,
        /// Milliseconds since the start, when the host received the
        /// answer (a human's think time includes the client's pacing).
        at: u64,
        option: u32,
        choice: ChoiceKey,
        by: By,
        /// Every option offered was this choice.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        forced: bool,
        /// The options the seat was shown, when its settings hid some.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        shown: Option<Vec<u32>>,
    },
    /// The seat took back its decisions from `to` on: the game continues
    /// from `to`, decision numbers repeat from there, and a reader drops
    /// the decision entries it takes back.
    Undo { seat: Seat, at: u64, to: DecisionNo },
    Settings {
        seat: Seat,
        at: u64,
        settings: Settings,
    },
    /// A command refused.
    Refused { seat: Seat, at: u64, error: Error },
    /// The seat's `feedback`, at the decision it was sent during.
    Feedback {
        seat: Seat,
        at: u64,
        decision: DecisionNo,
        text: String,
    },
    /// An entry of a later host this reader does not know: read past
    /// (docs/protocol.md §14). Never written.
    #[serde(other)]
    Unknown,
}

crate::ids::ids!(
    /// Who answered a decision.
    By {
        Human,
        Bot,
        /// A standing order (`vote_all`) or an allocation answered whole.
        Order,
        /// A lone pass, the seat's settings hiding the rest.
        Auto,
    }
);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GameResult {
    pub vp_halves: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub winner: Option<Seat>,
    pub reason: EndReason,
    /// The engine's failure, when it failed: message, location, backtrace;
    /// for a game a restart ended, why its record did not read to its end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<String>,
}
