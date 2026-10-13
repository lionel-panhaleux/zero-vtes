//! The engine and its bots in the browser (docs/protocol.md, "Embedded"):
//! a `Game` speaking the protocol's JSON — the same a server carries.
//! Strings cross the boundary, not JS objects: the contract is the JSON.

use std::sync::Arc;

use wasm_bindgen::prelude::*;
use zvtes_api::record::BotRecord;
use zvtes_api::{DeckList, GameSpec};
use zvtes_host::{Bots, Clock};
use zvtes_policy::{Net, NetAgent, Nudges};

/// The bot profile the trained policy plays (`Game.withPolicy`).
const NETWORK: &str = "network";

/// The network bots' temperature (`NetAgent::temperature`): measured on
/// `p7-politics@1400` (`crates/policy/examples/strength.rs`, 512 games),
/// 1.16 VP a seat at 0.5 against 0.82 sampling as training does (0.3:
/// 1.18 / 0.80; 0.7: 1.06 / 0.92) — the network's favourites more often,
/// a card it rates low played less "for the sake of it".
const TEMPERATURE: f32 = 0.5;

/// The network bots' rescue bonus (`Nudges::rescue`; owner,
/// 2026-10-05: rescuing one's own vampire encouraged, not forced): +6
/// takes 7% of the rescues offered at temperature 1 (0% without), at no
/// measured cost (512 games: 0.94 / 1.05 VP a seat, within the control's
/// spread; with the temperature, 1.17 / 0.81 as without).
const RESCUE_BONUS: f32 = 6.0;

/// The network bots' nudges: those `p10@3840` trained with (p9's, nightmare
/// 4 → 6), never faded
/// (`nudge_until` 0) — off them it plays off its training (a transfer onto
/// a second copy of a unique: 35% of the offers instead of 4%; Z-train,
/// 2026-10-12).
fn nudges() -> Nudges {
    Nudges {
        rescue: RESCUE_BONUS,
        bleed: 4.0,
        idle: 3.0,
        draw: 7.0,
        stuck: 6.0,
        nightmare: 6.0,
        ..Nudges::default()
    }
}

fn json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).expect("protocol types serialize")
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Date)]
    fn now() -> f64;
}

/// The page's clock (`Date.now`).
struct PageClock;

impl Clock for PageClock {
    fn now(&self) -> u64 {
        now() as u64
    }
}

/// The `network` profile, when weights were given.
struct NetBots(Option<Arc<Net>>);

impl Bots for NetBots {
    fn profiles(&self) -> Vec<String> {
        self.0.iter().map(|_| NETWORK.to_string()).collect()
    }

    fn make(&self, profile: &str, seed: u32) -> Option<(Box<dyn zvtes::agent::Agent>, BotRecord)> {
        let net = self.0.as_ref().filter(|_| profile == NETWORK)?;
        let agent = NetAgent::new(net.clone(), seed as u64)
            .temperature(TEMPERATURE)
            .nudges(nudges());
        let record = BotRecord {
            seat: 0,
            profile: NETWORK.into(),
            kind: "network".into(),
            weights: None,
            temperature: Some(TEMPERATURE),
            seed,
        };
        Some((Box::new(agent), record))
    }
}

#[wasm_bindgen]
pub struct Game {
    game: zvtes_host::Game,
}

#[wasm_bindgen]
impl Game {
    /// A game from a `GameSpec` (JSON), its bots scripted. Throws the
    /// `SpecError` (JSON).
    #[wasm_bindgen(constructor)]
    pub fn new(spec: &str) -> Result<Game, JsError> {
        Self::make(spec, None)
    }

    /// A game whose `network` bots play the trained policy: `weights` is a
    /// `.zvp` file (`python -m train.export`). Throws the `SpecError`
    /// (JSON), or what is wrong with the weights.
    #[wasm_bindgen(js_name = withPolicy)]
    pub fn with_policy(spec: &str, weights: &[u8]) -> Result<Game, JsError> {
        let net = Net::load(weights).map_err(|e| JsError::new(&e))?;
        Self::make(spec, Some(Arc::new(net)))
    }

    fn make(spec: &str, net: Option<Arc<Net>>) -> Result<Game, JsError> {
        console_error_panic_hook::set_once();
        let spec: GameSpec =
            serde_json::from_str(spec).map_err(|e| JsError::new(&e.to_string()))?;
        let game = zvtes_host::Game::new("local".into(), spec, &NetBots(net), Box::new(PageClock))
            .map_err(|e| JsError::new(&json(&e)))?;
        Ok(Game { game })
    }

    /// The human seat.
    pub fn seat(&self) -> u8 {
        self.game.human()
    }

    /// A command (`In`, JSON) for the human seat: the messages answering
    /// it (a JSON array of `Out`). The first is `{"type": "join"}`.
    pub fn handle(&mut self, command: &str) -> String {
        let seat = self.game.human();
        self.game.handle_json(seat, command)
    }

    /// The game's record (`GameRecord`, JSON).
    pub fn record(&self) -> String {
        json(self.game.record())
    }
}

/// The decks the engine ships (JSON: `[[name, {krcg id: copies}], …]`).
#[wasm_bindgen]
pub fn catalog() -> String {
    json(&zvtes_host::catalog())
}

/// Check a decklist (JSON `{krcg id: copies}`): `null`, or the
/// `SpecError` (JSON).
#[wasm_bindgen(js_name = checkDeck)]
pub fn check_deck(list: &str) -> Result<String, JsError> {
    let list: DeckList = serde_json::from_str(list).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(match zvtes_host::check_deck(&list) {
        Ok(()) => "null".into(),
        Err(e) => json(&e),
    })
}
