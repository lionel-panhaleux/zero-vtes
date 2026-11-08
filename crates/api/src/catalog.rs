//! What each deck of the catalog is (`GET /catalog/decks`, protocol §10):
//! a preconstructed deck, a New Blood starter, or a tournament-winning one
//! with its archive's title, event and date, each with its decklist.

use serde::{Deserialize, Serialize};

use crate::DeckList;

/// A deck the engine ships, as `GET /catalog` names it, with what it is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CatalogDeck {
    /// Its name in `GET /catalog`.
    pub name: String,
    /// The deck's own name: `name` without its ` (set)` (a precon's,
    /// a starter's) or ` (event, date)` (a tournament winner's).
    pub title: String,
    pub kind: DeckKind,
    /// The decklist, as `GET /catalog` gives it.
    pub list: DeckList,
    /// A tournament winner's TWDA id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub twda: Option<String>,
    /// The event it won.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// The event's day, `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

/// Where a catalog deck comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum DeckKind {
    /// A preconstructed deck: the V5 precons, the anniversary and the
    /// Sabbat V5 decks.
    Precon,
    /// A New Blood mini-starter: a half deck (6 crypt cards, 48–49
    /// library cards).
    Starter,
    /// A tournament-winning deck of the TWDA, all its cards in the pool.
    Twda,
}
