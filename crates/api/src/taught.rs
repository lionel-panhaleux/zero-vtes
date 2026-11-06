//! The decks the served bot plays worst (`GET /catalog/taught`, protocol
//! §10): the training line's measure of the `network` profile's weights
//! on the catalog's tournament-winning decks, each with its decklist.

use serde::{Deserialize, Serialize};

use crate::DeckList;

/// The served weights' measure, weakest deck first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TaughtDecks {
    /// The SHA-256 of the weights measured: `GET /version`'s `weights`.
    pub weights: String,
    /// The engine commit the games ran on.
    pub engine: String,
    /// The day measured, `YYYY-MM-DD`.
    pub measured: String,
    /// How, in one line: the deck at one seat, the weights on precons at
    /// the others, the games a deck.
    pub measure: String,
    /// By `vp_share`, the lowest first.
    pub decks: Vec<TaughtDeck>,
}

/// A deck measured, with its decklist.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TaughtDeck {
    /// The deck's own name: `name` without its ` (event, date)`.
    pub title: String,
    #[serde(flatten)]
    pub strength: DeckMeasure,
    /// The decklist, as `GET /catalog` gives it.
    pub list: DeckList,
}

/// One deck's measure, as the training line writes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DeckMeasure {
    /// The deck's name in `GET /catalog`.
    pub name: String,
    /// Its TWDA id.
    pub twda: String,
    /// The event it won.
    pub event: String,
    /// The event's day, `YYYY-MM-DD`.
    pub date: String,
    /// Within two years of `measured`.
    pub recent: bool,
    /// Games played with it.
    pub games: u32,
    /// Games it won.
    pub wins: u32,
    /// Its victory points a game.
    pub vp: f64,
    /// Its share of the victory points scored at its tables: 0.25 is
    /// even at four seats.
    pub vp_share: f64,
}
