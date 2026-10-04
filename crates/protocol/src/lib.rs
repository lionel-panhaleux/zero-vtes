//! The client-facing contract (docs/human-play.md): JSON messages over the
//! engine — decks by KRCG id, the table as a viewer sees it (`TableView`,
//! with stable card copies), options described as data and text, and a
//! session stepping humans and bots one decision at a time. Spoken
//! in-process by the wasm module, later by a server.

pub mod deck;
pub mod describe;
pub mod logged;
mod narrate;
mod purpose;
pub mod session;
pub mod track;
pub mod view;

pub use session::{
    AnswerError, GameSpec, Out, Player, Replay, ReplayReader, SeatSpec, Session, SpecError,
};
pub use view::TableView;
