//! The engine and its bots in the browser (docs/human-play.md): a `Game`
//! speaking the protocol's JSON messages — the same a server will carry.
//! Strings cross the boundary, not JS objects: the contract is the JSON.

use std::rc::Rc;

use wasm_bindgen::prelude::*;
use zvtes_policy::{Net, NetAgent, Nudges};
use zvtes_protocol::{deck, GameSpec, Player, Session};

/// The bot style the trained policy plays (`Game.withPolicy`).
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

fn json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).expect("protocol types serialize")
}

#[wasm_bindgen]
pub struct Game {
    session: Session,
}

#[wasm_bindgen]
impl Game {
    /// A game from a `GameSpec` (JSON). Throws the `SpecError` (JSON).
    #[wasm_bindgen(constructor)]
    pub fn new(spec: &str) -> Result<Game, JsError> {
        console_error_panic_hook::set_once();
        let spec: GameSpec =
            serde_json::from_str(spec).map_err(|e| JsError::new(&e.to_string()))?;
        let session = Session::new(spec).map_err(|e| JsError::new(&json(&e)))?;
        Ok(Game { session })
    }

    /// A game whose `network` bots play the trained policy: `weights` is a
    /// `.zvp` file (`python -m train.export`). Throws the `SpecError`
    /// (JSON), or what is wrong with the weights.
    #[wasm_bindgen(js_name = withPolicy)]
    pub fn with_policy(spec: &str, weights: &[u8]) -> Result<Game, JsError> {
        console_error_panic_hook::set_once();
        let spec: GameSpec =
            serde_json::from_str(spec).map_err(|e| JsError::new(&e.to_string()))?;
        let net = Rc::new(Net::load(weights).map_err(|e| JsError::new(&e))?);
        let seats: Vec<usize> = (0..spec.seats.len())
            .filter(|&i| matches!(&spec.seats[i].player, Player::Bot { style } if style == NETWORK))
            .collect();
        let seed = spec.seed;
        let mut session =
            Session::with_styles(spec, &[NETWORK]).map_err(|e| JsError::new(&json(&e)))?;
        for seat in seats {
            let seed = seed.wrapping_mul(0x9e37_79b9).wrapping_add(seat as u64);
            let agent = NetAgent::new(net.clone(), seed)
                .temperature(TEMPERATURE)
                .nudges(Nudges {
                    rescue: RESCUE_BONUS,
                    ..Nudges::default()
                });
            session.set_agent(seat, Box::new(agent));
        }
        Ok(Game { session })
    }

    /// The next message (`Out`, JSON): a human decision (repeated until
    /// answered), a bot's decision, or the game's end.
    pub fn advance(&mut self) -> String {
        json(&self.session.advance())
    }

    /// Answer the pending human decision: the `Decided` message (JSON).
    /// Throws the `AnswerError` (JSON).
    pub fn answer(&mut self, decision: u32, index: u32) -> Result<String, JsError> {
        self.session
            .answer(decision as u64, index as usize)
            .map(|o| json(&o))
            .map_err(|e| JsError::new(&json(&e)))
    }

    /// Answer the allocation under way at once (a `decision` message's
    /// `allocation`): `amounts` in its recipients' order. The `Decided`
    /// messages (a JSON array), one per step the engine asked. Throws the
    /// `AnswerError` (JSON).
    pub fn allocate(&mut self, decision: u32, amounts: &[u8]) -> Result<String, JsError> {
        self.session
            .allocate(decision as u64, amounts)
            .map(|o| json(&o))
            .map_err(|e| JsError::new(&json(&e)))
    }

    /// Cast all the votes one way (a `decision` message's `votes`): this
    /// one now, the others as their turn comes in this polling (`advance`
    /// answers them). The `Decided` message (JSON). Throws the
    /// `AnswerError` (JSON).
    #[wasm_bindgen(js_name = voteAll)]
    pub fn vote_all(&mut self, decision: u32, favor: bool) -> Result<String, JsError> {
        self.session
            .vote_all(decision as u64, favor)
            .map(|o| json(&o))
            .map_err(|e| JsError::new(&json(&e)))
    }

    /// Take back the previous step of the multi-step choice under way (a
    /// `decision` message says when: `undo`); `advance` then asks it again.
    /// Throws the `AnswerError` (JSON).
    pub fn undo(&mut self) -> Result<(), JsError> {
        self.session.undo().map_err(|e| JsError::new(&json(&e)))
    }

    /// The replay (JSON): the spec and every decision.
    pub fn replay(&self) -> String {
        json(&self.session.replay())
    }
}

/// The decks the engine ships (JSON: `[[name, {krcg id: copies}], …]`).
#[wasm_bindgen]
pub fn catalog() -> String {
    json(&deck::catalog())
}

/// Check a decklist (JSON `{krcg id: copies}`): `null`, or the `DeckError`
/// (JSON).
#[wasm_bindgen(js_name = checkDeck)]
pub fn check_deck(list: &str) -> Result<String, JsError> {
    let list: deck::DeckList =
        serde_json::from_str(list).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(match deck::resolve(&list) {
        Ok(_) => "null".into(),
        Err(e) => json(&e),
    })
}
