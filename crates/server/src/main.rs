//! zvtes-server: the engine served (docs/protocol.md §10). Configuration
//! from the environment (a systemd `EnvironmentFile`):
//!
//! - `ZVTES_LISTEN` (`127.0.0.1:8040`)
//! - `ZVTES_DATABASE_URL`: Postgres; absent, records go to `ZVTES_RECORDS`
//!   (`records/`) as JSON lines
//! - `ZVTES_POLICY`: the `.zvp` weights of the `network` profile
//! - `ZVTES_ORIGINS`: the WebSocket origins served, comma-separated
//!   (absent: any — development)
//! - `ZVTES_MAX_GAMES` (64), `ZVTES_GAMES_PER_HOUR` per address (30),
//!   `ZVTES_IDLE` seconds (1800), `ZVTES_TIME_LIMIT` max turns (200),
//!   `ZVTES_RETENTION` days (730), `ZVTES_RATE` commands a second (20)

mod games;
mod store;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::ws::{Message as Ws, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Path, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get, post};
use axum::{Json, Router};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, oneshot};
use tower_http::cors::{AllowOrigin, CorsLayer};
use zvtes_api::message::{Error, SpecError};
use zvtes_api::{Command, GameSpec, In, Message, Out, Player};
use zvtes_host::Bots;

use crate::games::{Games, Job, Policy, ServerBots};
use crate::store::{Put, Store};

struct Config {
    origins: Option<Vec<String>>,
    max_games: usize,
    per_hour: usize,
    idle: Duration,
    time_limit: u32,
    /// Commands a second per connection, at most (an undo replays the
    /// game from the deal).
    rate: usize,
}

struct App {
    config: Config,
    games: Games,
    policy: Option<Arc<Policy>>,
    store: mpsc::UnboundedSender<Put>,
    /// Games created per address, the last hour.
    created: Mutex<HashMap<String, Vec<Instant>>>,
}

type Shared = Arc<App>;

fn env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

fn env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    env(key).and_then(|v| v.parse().ok()).unwrap_or(default)
}

#[tokio::main]
async fn main() {
    let listen = env("ZVTES_LISTEN").unwrap_or_else(|| "127.0.0.1:8040".into());
    let policy = env("ZVTES_POLICY").map(|path| {
        let bytes = std::fs::read(&path).expect("the policy weights");
        let hash = format!("{:x}", Sha256::digest(&bytes));
        let net = zvtes_policy::Net::load(&bytes).expect("a policy");
        eprintln!("policy {path}: sha256 {hash}");
        Arc::new(Policy {
            net: Arc::new(net),
            hash,
        })
    });
    let database = env("ZVTES_DATABASE_URL");
    let store = Store::open(
        database.clone(),
        env("ZVTES_RECORDS")
            .unwrap_or_else(|| "records".into())
            .into(),
    )
    .await;
    if let Store::Postgres(pool) = &store {
        let pool = pool.clone();
        let days = env_or("ZVTES_RETENTION", 730);
        tokio::spawn(async move {
            loop {
                match Store::purge(&pool, days).await {
                    Ok(n) if n > 0 => eprintln!("purge: {n} games over {days} days old"),
                    Ok(_) => {}
                    Err(e) => eprintln!("purge: {e}"),
                }
                tokio::time::sleep(Duration::from_secs(24 * 3600)).await;
            }
        });
    }
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(store.run(rx));
    let app = Arc::new(App {
        config: Config {
            origins: env("ZVTES_ORIGINS")
                .map(|o| o.split(',').map(|s| s.trim().to_string()).collect()),
            max_games: env_or("ZVTES_MAX_GAMES", 64),
            per_hour: env_or("ZVTES_GAMES_PER_HOUR", 30),
            idle: Duration::from_secs(env_or("ZVTES_IDLE", 1800)),
            time_limit: env_or("ZVTES_TIME_LIMIT", 200),
            rate: env_or("ZVTES_RATE", 20),
        },
        games: Games::default(),
        policy,
        store: tx,
        created: Mutex::new(HashMap::new()),
    });
    let router = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/version", get(version))
        .route("/catalog", get(|| async { Json(zvtes_host::catalog()) }))
        .route("/games", post(create))
        .route("/games/{id}/ws", any(socket))
        .route("/games/{id}/record", get(record))
        .layer(cors(&app.config.origins))
        .with_state(app);
    let listener = tokio::net::TcpListener::bind(&listen)
        .await
        .expect("the address");
    eprintln!(
        "zvtes-server {} (protocol {}) on {listen}",
        zvtes_host::ENGINE,
        zvtes_api::PROTOCOL_VERSION
    );
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
    .expect("the server");
}

/// The HTTP endpoints, for the platforms served (any origin in
/// development).
fn cors(origins: &Option<Vec<String>>) -> CorsLayer {
    let layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);
    match origins {
        Some(list) => layer.allow_origin(AllowOrigin::list(
            list.iter().filter_map(|o| HeaderValue::from_str(o).ok()),
        )),
        None => layer.allow_origin(AllowOrigin::any()),
    }
}

async fn version(State(app): State<Shared>) -> Json<serde_json::Value> {
    let mut profiles: Vec<String> = zvtes_host::BUILT_IN.iter().map(|s| s.to_string()).collect();
    profiles.extend(ServerBots(app.policy.clone()).profiles());
    Json(json!({
        "protocol": zvtes_api::PROTOCOL_VERSION,
        "engine": zvtes_host::ENGINE,
        "profiles": profiles,
        "weights": app.policy.as_ref().map(|p| p.hash.clone()),
        "limits": {
            "seats": [2, 6],
            "humans": 1,
            "time_limit": app.config.time_limit,
            "idle_timeout": app.config.idle.as_secs(),
            "games_per_hour": app.config.per_hour,
        },
    }))
}

/// The client's address: nginx's `X-Real-IP` (or `X-Forwarded-For`)
/// before the socket's.
fn address(headers: &HeaderMap, peer: SocketAddr) -> String {
    headers
        .get("x-real-ip")
        .or_else(|| headers.get("x-forwarded-for"))
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|v| v.trim().to_string())
        .unwrap_or_else(|| peer.ip().to_string())
}

fn refuse(status: StatusCode, e: SpecError) -> Response {
    (status, Json(e)).into_response()
}

async fn create(
    State(app): State<Shared>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(spec): Json<GameSpec>,
) -> Response {
    if let Some(max) = spec.time_limit.filter(|&t| t > app.config.time_limit) {
        let _ = max;
        return refuse(
            StatusCode::BAD_REQUEST,
            SpecError::TimeLimit {
                max: app.config.time_limit,
            },
        );
    }
    let humans = spec
        .seats
        .iter()
        .filter(|s| matches!(s.player, Player::Human { .. }))
        .count();
    if humans != 1 {
        return refuse(
            StatusCode::BAD_REQUEST,
            SpecError::Humans { n: humans as u8 },
        );
    }
    if app.games.len() >= app.config.max_games {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "the server is full: try again later",
        )
            .into_response();
    }
    {
        let who = address(&headers, peer);
        let mut created = app.created.lock().unwrap();
        let mine = created.entry(who).or_default();
        mine.retain(|t| t.elapsed() < Duration::from_secs(3600));
        if mine.len() >= app.config.per_hour {
            return (StatusCode::TOO_MANY_REQUESTS, "too many games this hour").into_response();
        }
        mine.push(Instant::now());
    }
    match games::start(
        &app.games,
        spec,
        app.policy.clone(),
        app.store.clone(),
        app.config.idle,
    )
    .await
    {
        Ok((game, seat, token)) => {
            Json(json!({"game": game, "seat": seat, "token": token})).into_response()
        }
        Err(e) => refuse(StatusCode::BAD_REQUEST, e),
    }
}

async fn record(State(app): State<Shared>, Path(id): Path<String>, headers: HeaderMap) -> Response {
    let Some(h) = app.games.get(&id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let bearer = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    if bearer != Some(h.token.as_str()) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let (tx, rx) = oneshot::channel();
    if h.jobs.send(Job::Record(tx)).is_err() {
        return StatusCode::NOT_FOUND.into_response();
    }
    match rx.await {
        Ok(r) => Json(r).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn socket(
    State(app): State<Shared>,
    Path(id): Path<String>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if let Some(origins) = &app.config.origins {
        let origin = headers
            .get("origin")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !origins.iter().any(|o| o == origin) {
            return StatusCode::FORBIDDEN.into_response();
        }
    }
    if app.games.get(&id).is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
    ws.on_upgrade(move |s| serve(app, id, s))
}

fn frame(outs: &[Out]) -> Vec<Ws> {
    outs.iter()
        .map(|o| Ws::Text(serde_json::to_string(o).expect("messages serialize").into()))
        .collect()
}

/// A refusal outside the game's numbering (before `join`).
fn unnumbered(e: Error) -> Ws {
    let o = Out {
        seq: 0,
        reply_to: None,
        message: Message::Error(e),
    };
    Ws::Text(
        serde_json::to_string(&o)
            .expect("messages serialize")
            .into(),
    )
}

async fn serve(app: Shared, id: String, mut ws: WebSocket) {
    let mut joined = false;
    let mut window = (Instant::now(), 0usize);
    let mut ping = tokio::time::interval(Duration::from_secs(30));
    loop {
        let msg = tokio::select! {
            m = ws.recv() => m,
            _ = ping.tick() => {
                if ws.send(Ws::Ping(Default::default())).await.is_err() { return; }
                continue;
            }
        };
        let text = match msg {
            Some(Ok(Ws::Text(t))) => t.to_string(),
            Some(Ok(Ws::Close(_))) | None | Some(Err(_)) => return,
            Some(Ok(_)) => continue,
        };
        if window.0.elapsed() > Duration::from_secs(1) {
            window = (Instant::now(), 0);
        }
        window.1 += 1;
        if window.1 > app.config.rate {
            if ws.send(unnumbered(Error::RateLimited)).await.is_err() {
                return;
            }
            continue;
        }
        let Some(h) = app.games.get(&id) else {
            let _ = ws.send(unnumbered(Error::Over)).await;
            return;
        };
        let input = match serde_json::from_str::<In>(&text) {
            Ok(i) => i,
            Err(e) => {
                let _ = ws
                    .send(unnumbered(Error::Malformed {
                        reason: e.to_string(),
                    }))
                    .await;
                continue;
            }
        };
        if !joined {
            match &input.command {
                Command::Join { token: Some(t), .. } if *t == h.token => joined = true,
                _ => {
                    let _ = ws.send(unnumbered(Error::NotJoined)).await;
                    continue;
                }
            }
        }
        let (tx, rx) = oneshot::channel();
        if h.jobs
            .send(Job::Command {
                seat: h.seat,
                input,
                reply: tx,
            })
            .is_err()
        {
            return;
        }
        let Ok(outs) = rx.await else { return };
        for f in frame(&outs) {
            if ws.send(f).await.is_err() {
                return;
            }
        }
    }
}
