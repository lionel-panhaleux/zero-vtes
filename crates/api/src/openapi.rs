//! The server's HTTP surface as OpenAPI 3.1 (`crates/api/openapi.json`,
//! served at `/openapi.json` and read by the reference at `/docs`): the
//! routes of `docs/protocol.md` §10 but the operator's, the WebSocket as
//! a route whose frames are the `In` and `Out` models, and
//! `docs/protocol.md` as the intro. The models are the JSON Schema's
//! `$defs` (OpenAPI 3.1 schemas are JSON Schema 2020-12).

use serde_json::{json, Value};

fn model(name: &str) -> Value {
    json!({"$ref": format!("#/components/schemas/{name}")})
}

fn ok(description: &str, schema: Value) -> Value {
    json!({"description": description, "content": {"application/json": {"schema": schema}}})
}

fn plain(description: &str) -> Value {
    json!({"description": description, "content": {"text/plain": {"schema": {"type": "string"}}}})
}

fn game_id() -> Value {
    json!({
        "name": "game", "in": "path", "required": true,
        "description": "The game's id, from `POST /games`.",
        "schema": {"type": "string"},
    })
}

fn deck_list() -> Value {
    json!({
        "type": "object",
        "description": "KRCG card id (a string) → copies.",
        "additionalProperties": {"type": "integer", "minimum": 0},
    })
}

fn version() -> Value {
    json!({
        "tags": ["Server"],
        "summary": "Versions, bots and limits",
        "description": "The protocol version, the engine commit, the bot profiles a seat \
            may name (`network` is the trained policy; `aggro_bleed`, `wall` and `random` \
            are scripted) and the server's limits.",
        "operationId": "version",
        "responses": {"200": ok("The server's versions and limits.", json!({
            "type": "object",
            "required": ["protocol", "engine", "profiles", "limits"],
            "properties": {
                "protocol": {"type": "string", "description": "The protocol version (semver)."},
                "engine": {"type": "string", "description": "The engine's commit."},
                "profiles": {"type": "array", "items": {"type": "string"},
                    "description": "The bot profiles offered."},
                "weights": {"type": ["string", "null"],
                    "description": "The SHA-256 of the `network` profile's weights."},
                "limits": {
                    "type": "object",
                    "properties": {
                        "seats": {"type": "array", "items": {"type": "integer"},
                            "description": "The fewest and the most seats."},
                        "humans": {"type": "integer", "description": "Human seats per game."},
                        "time_limit": {"type": "integer",
                            "description": "The longest time limit, in player turns."},
                        "idle_timeout": {"type": "integer",
                            "description": "Seconds without a command before a game ends."},
                        "games_per_hour": {"type": "integer",
                            "description": "Games an address may create per hour."},
                    },
                },
            },
            "example": {
                "protocol": "2.0.0", "engine": "331ce66d7d55",
                "profiles": ["aggro_bleed", "wall", "random", "network"],
                "weights": "b9035931a5276ec68d7c8c03b516861df8502d475a670579b9615cdc6c870d65",
                "limits": {"seats": [2, 6], "humans": 1, "time_limit": 200,
                    "idle_timeout": 1800, "games_per_hour": 30},
            },
        }))},
    })
}

fn catalog() -> Value {
    json!({
        "tags": ["Server"],
        "summary": "The decks",
        "description": "The decklists the engine plays (the V5 preconstructed decks and \
            tournament-winning decks within its card pool), as `[name, deck]` pairs: a \
            deck goes as is in a seat of `POST /games`.",
        "operationId": "catalog",
        "responses": {"200": ok("The decks, by name.", json!({
            "type": "array",
            "items": {
                "type": "array",
                "prefixItems": [{"type": "string", "description": "The deck's name."}, deck_list()],
                "minItems": 2, "maxItems": 2,
            },
            "example": [["Toreador (Fifth Edition)", {"201538": 2, "201540": 1, "100031": 2}]],
        }))},
    })
}

fn create() -> Value {
    json!({
        "tags": ["Games"],
        "summary": "Create a game",
        "description": "Deals a game: 2 to 6 seats in seating order, one human, the others \
            bots (a profile from `GET /version`), each with a deck (from `GET /catalog`, or \
            any deck within the engine's pool: 12 crypt cards at least, 60 to 90 library \
            cards). The answer's token is the human seat's: send it in `join` on the \
            game's WebSocket. Not tried from this page: each call creates a recorded game.",
        "operationId": "createGame",
        "requestBody": {
            "required": true,
            "content": {"application/json": {
                "schema": model("GameSpec"),
                "example": {
                    "seats": [
                        {"name": "Me", "deck": {"201538": 2, "100031": 2},
                         "player": {"type": "human", "settings": {"text": true}}},
                        {"name": "Bot 1", "deck": {"201538": 2, "100031": 2},
                         "player": {"type": "bot", "profile": "network"}},
                        {"name": "Bot 2", "deck": {"201538": 2, "100031": 2},
                         "player": {"type": "bot", "profile": "wall"}},
                    ],
                    "client": "my-client 0.1",
                },
            }},
        },
        "responses": {
            "200": ok("The game is dealt.", json!({
                "type": "object",
                "required": ["game", "seat", "token"],
                "properties": {
                    "game": {"type": "string", "description": "The game's id."},
                    "seat": {"type": "integer", "description": "The human's seat."},
                    "token": {"type": "string",
                        "description": "The human seat's token: keep it, never in a URL."},
                },
            })),
            "400": ok("The spec is refused: a deck, a profile, the seats, the time limit.",
                model("SpecError")),
            "429": plain("Too many games from this address this hour."),
            "503": plain("The server is full: try again later."),
        },
    })
}

fn socket() -> Value {
    json!({
        "tags": ["Games"],
        "summary": "Play (WebSocket)",
        "description": "Upgrade to a WebSocket (`wss://`). One JSON object per text frame: \
            the client sends commands (the `In` model), the host sends messages (the `Out` \
            model). The first command is `join` with the token from `POST /games`; after a \
            disconnection, `join` again with `after`, the last `seq` received, and the host \
            sends what followed. A browser page must come from an origin the server serves \
            (403 otherwise); other clients send no `Origin`.",
        "operationId": "play",
        "parameters": [game_id()],
        "responses": {
            "101": {"description": "Switching protocols: the game's socket. Frames: `In` \
                (client → host), `Out` (host → client)."},
            "403": {"description": "A browser page from an origin the server does not serve."},
            "404": {"description": "No live game with this id."},
        },
    })
}

fn record() -> Value {
    json!({
        "tags": ["Games"],
        "summary": "A game's record",
        "description": "The game as recorded: the spec with its seed, the bot profiles, \
            every decision answered, the result. Once the game is over only: replaying the \
            record would show the hidden cards. Authorized by the seat's token.",
        "operationId": "gameRecord",
        "parameters": [game_id()],
        "security": [{"seat": []}],
        "responses": {
            "200": ok("The record.", model("GameRecord")),
            "401": {"description": "No token, or not this game's."},
            "404": {"description": "No such game."},
            "409": {"description": "The game is not over yet."},
        },
    })
}

/// `docs/protocol.md` as the reference's intro: without its title (the
/// reference's own) and its maintainers' note.
fn intro() -> &'static str {
    let doc = include_str!("../../../docs/protocol.md");
    let doc = doc
        .split_once('\n')
        .map_or(doc, |(_, rest)| rest)
        .trim_start();
    match doc.strip_prefix("<!--").and_then(|d| d.split_once("-->")) {
        Some((_, rest)) => rest.trim_start(),
        None => doc,
    }
}

/// The OpenAPI document, pretty JSON, stable across runs.
pub fn json() -> String {
    let schemas = Value::Object(crate::schema::defs());
    // The models refer to each other as OpenAPI components.
    let schemas: Value = serde_json::from_str(
        &serde_json::to_string(&schemas)
            .unwrap()
            .replace("\"#/$defs/", "\"#/components/schemas/"),
    )
    .unwrap();
    let doc = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "zero-vtes engine",
            "version": crate::PROTOCOL_VERSION,
            "description": intro(),
        },
        "servers": [{"url": "https://engine.krcg.org"}],
        "tags": [
            {"name": "Games", "description": "Create a game, play it, read its record."},
            {"name": "Server", "description": "What the server offers."},
        ],
        "paths": {
            "/games": {"post": create()},
            "/games/{game}/ws": {"get": socket()},
            "/games/{game}/record": {"get": record()},
            "/version": {"get": version()},
            "/catalog": {"get": catalog()},
            "/health": {"get": {
                "tags": ["Server"],
                "summary": "Health",
                "operationId": "health",
                "responses": {"200": plain("`ok`.")},
            }},
        },
        "components": {
            "schemas": schemas,
            "securitySchemes": {"seat": {
                "type": "http", "scheme": "bearer",
                "description": "The seat's token, from `POST /games`.",
            }},
        },
    });
    serde_json::to_string_pretty(&doc).unwrap()
}
