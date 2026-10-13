//! What a game is made of: seats, decks, players, settings.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A deck: KRCG id (as a string, JSON object keys) → copies.
pub type DeckList = BTreeMap<String, u32>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GameSpec {
    /// The deal's seed; the host picks one when absent (and records it).
    /// 32 bits: exact as a JSON number in every language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,
    /// In player turns; the engine's default when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_limit: Option<u32>,
    /// In seating order: 2 to 6.
    pub seats: Vec<SeatSpec>,
    /// The client's self-description ("succubus-club 1.4"), recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SeatSpec {
    /// Shown to the table; a human seat's is not recorded (records say
    /// "human").
    pub name: String,
    pub deck: DeckList,
    pub player: Player,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Player {
    Human {
        #[serde(default)]
        settings: Settings,
    },
    /// A bot, by profile name: one the host offers (`/version` lists
    /// them): a network policy at a temperature, or a scripted style.
    Bot { profile: String },
}

/// A human seat's preferences (fields left out take their default);
/// changed during the game by `Command::Settings`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct Settings {
    /// Asked about every opportunity (true), or only in its moment
    /// (false): a wake outside an action that concerns the seat, a block
    /// denial short of the action's stealth, an any-time ability where
    /// nothing else is on offer are passed over; a decision left with a
    /// pass alone passes by itself.
    pub opportunities: bool,
    /// Table views sent on every step (bots' included: a client
    /// animating each move) or only with the seat's decisions and the
    /// game's end.
    pub views: Views,
    /// English renderings beside the data (`text` on options, events,
    /// markers, strikes; a decision's `prompt`): for a client that does
    /// not compose its own.
    pub text: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            opportunities: true,
            views: Views::Decisions,
            text: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Views {
    Decisions,
    Steps,
}

/// Some settings to change; the others stay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SettingsPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opportunities: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub views: Option<Views>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<bool>,
}

impl Settings {
    pub fn apply(&mut self, p: SettingsPatch) {
        if let Some(o) = p.opportunities {
            self.opportunities = o;
        }
        if let Some(v) = p.views {
            self.views = v;
        }
        if let Some(t) = p.text {
            self.text = t;
        }
    }
}
