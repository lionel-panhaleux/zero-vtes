//! zero-vtes env (roadmap, objective C): what sits between the engine and
//! an agent — the observations a network reads (`obs`, with the seats'
//! context, `context`), the behaviour
//! around a network seat (`habits`, `politics`), the scripted bots
//! (`bots`). Agent behaviour, never an engine mask (rules-qa 17.4); the
//! engine crate holds the rules only.

pub mod bots;
pub mod context;
pub mod habits;
pub mod obs;
pub mod politics;
