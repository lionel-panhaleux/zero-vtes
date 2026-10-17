//! The contract between a game and its clients (`docs/protocol.md`).
//!
//! A client sends [`Command`]s for its seat and receives [`Message`]s
//! addressed to it; the game runs the bots in between. Everything is
//! structured data: stable snake_case ids (an enum here, or a documented
//! open set: clans, titles, Disciplines), seats, card ids and copies,
//! amounts. The client composes what it displays; an English rendering
//! (`text`) comes only when the seat's settings ask for it. Copy numbers
//! are per viewer (§ "Copies" of the doc).
//!
//! JSON: externally visible names are snake_case; enums with data are
//! tagged by `type` (messages, commands, record entries), `kind` (events,
//! targets) or `error`.

pub mod command;
pub mod event;
pub mod ids;
pub mod message;
#[cfg(feature = "schema")]
pub mod openapi;
pub mod option;
pub mod record;
pub mod spec;
pub mod view;

pub use command::{Command, In};
pub use event::{Event, EventKind};
pub use ids::*;
pub use message::{Decision, EndReason, Error, Joined, Message, Out, Over, SpecError};
pub use option::{ChoiceKey, OptionView};
pub use record::GameRecord;
pub use spec::{DeckList, GameSpec, Player, SeatSpec, Settings, SettingsPatch};
pub use view::TableView;

/// The protocol's version (semver): a client checks the major in
/// [`Joined::protocol`].
pub const PROTOCOL_VERSION: &str = "1.8.0";

/// The contract as one JSON Schema document: commands, messages, the
/// game spec and the game record.
#[cfg(feature = "schema")]
pub mod schema {
    use schemars::{json_schema, schema_for, Schema};

    /// Pretty JSON, stable across runs.
    pub fn json() -> String {
        let mut root: Schema = json_schema!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": "zvtes protocol",
            "version": crate::PROTOCOL_VERSION,
        });
        root.insert("$defs".into(), serde_json::Value::Object(defs()));
        serde_json::to_string_pretty(&root).unwrap()
    }

    /// Every type a client meets, by name: commands, messages, the game
    /// spec, the game record and what they hold.
    pub fn defs() -> serde_json::Map<String, serde_json::Value> {
        let parts = [
            ("In", schema_for!(crate::In)),
            ("Out", schema_for!(crate::Out)),
            ("GameSpec", schema_for!(crate::GameSpec)),
            ("SpecError", schema_for!(crate::SpecError)),
            ("GameRecord", schema_for!(crate::GameRecord)),
        ];
        let mut defs = serde_json::Map::new();
        for (name, s) in parts {
            let mut v = s.to_value();
            if let Some(d) = v.as_object_mut().and_then(|o| o.remove("$defs")) {
                for (k, x) in d.as_object().cloned().unwrap_or_default() {
                    defs.insert(k, x);
                }
            }
            if let Some(o) = v.as_object_mut() {
                o.remove("$schema");
            }
            defs.insert(name.into(), v);
        }
        defs
    }
}
