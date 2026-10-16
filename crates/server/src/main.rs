//! zvtes-server: the engine served (docs/protocol.md §10). Configuration
//! from the environment (a systemd `EnvironmentFile`):
//!
//! - `ZVTES_LISTEN` (`127.0.0.1:8040`)
//! - `ZVTES_DATABASE_URL`: Postgres; absent, records go to `ZVTES_RECORDS`
//!   (`records/`) as JSON lines
//! - `ZVTES_POLICY`: the `.zvp` weights of the `network` profile
//! - `ZVTES_ORIGINS`: the page origins whose browsers may open a socket,
//!   comma-separated (a client sending no origin is let through)
//!   (absent: any — development)
//! - `ZVTES_MAX_GAMES` (64), `ZVTES_GAMES_PER_HOUR` per address (30),
//!   `ZVTES_IDLE` seconds (1800), `ZVTES_TIME_LIMIT` max turns (200),
//!   `ZVTES_RETENTION` days (730), `ZVTES_RATE` commands a second (20)
//! - `ZVTES_ADMIN_SHA256`: the admin token's SHA-256 (hex), never the
//!   token: `GET /admin/games`, `/admin/games/{id}/record`,
//!   `/admin/feedback` and `/admin/unsupported` with `Authorization:
//!   Bearer <token>` (absent: no admin routes)

mod games;
mod store;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::ws::rejection::WebSocketUpgradeRejection;
use axum::extract::ws::{Message as Ws, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Path, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{any, get, post};
use axum::{Json, Router};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, oneshot, watch};
use tower_http::cors::{AllowOrigin, CorsLayer};
use zvtes_api::message::{Error, SpecError};
use zvtes_api::record::GameRecord;
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
    /// The admin token's SHA-256.
    admin: Option<String>,
}

struct App {
    config: Config,
    games: Games,
    policy: Option<Arc<Policy>>,
    store: mpsc::UnboundedSender<Put>,
    /// Records of games no longer live.
    records: Store,
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
    let lost = store.clone();
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
    tokio::spawn(store.clone().run(rx));
    let app = Arc::new(App {
        config: Config {
            origins: env("ZVTES_ORIGINS")
                .map(|o| o.split(',').map(|s| s.trim().to_string()).collect()),
            max_games: env_or("ZVTES_MAX_GAMES", 64),
            per_hour: env_or("ZVTES_GAMES_PER_HOUR", 30),
            idle: Duration::from_secs(env_or("ZVTES_IDLE", 1800)),
            time_limit: env_or("ZVTES_TIME_LIMIT", 200),
            rate: env_or("ZVTES_RATE", 20),
            admin: env("ZVTES_ADMIN_SHA256").map(|h| h.to_lowercase()),
        },
        games: Games::default(),
        policy,
        store: tx,
        records: store,
        created: Mutex::new(HashMap::new()),
    });
    let router = Router::new()
        .route("/", get(|| async { Redirect::to("/docs") }))
        .route("/docs", get(|| async { Html(DOCS) }))
        .route(
            "/openapi.json",
            get(|| async { served("application/json", OPENAPI) }),
        )
        .route(
            "/schema.json",
            get(|| async { served("application/json", SCHEMA) }),
        )
        .route(
            "/protocol.ts",
            get(|| async { served("text/plain; charset=utf-8", PROTOCOL_TS) }),
        )
        .route("/health", get(|| async { "ok" }))
        .route("/version", get(version))
        .route("/catalog", get(|| async { Json(zvtes_host::catalog()) }))
        .route("/games", post(create))
        .route("/games/{id}/ws", any(socket))
        .route("/games/{id}/record", get(record))
        .route("/admin/games", get(admin_games))
        .route("/admin/games/{id}/record", get(admin_record))
        .route("/admin/feedback", get(admin_feedback))
        .route("/admin/unsupported", get(admin_unsupported))
        .layer(cors(&app.config.origins))
        .with_state(app);
    let listener = tokio::net::TcpListener::bind(&listen)
        .await
        .expect("the address");
    // After the bind: a second process on the same store dies on the
    // address before it ends the live server's games.
    let (ended, failed) = lost.end_lost().await;
    if ended > 0 {
        eprintln!("restart: {ended} games left going, ended");
    }
    for e in failed {
        eprintln!("restart: {e}");
    }
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

/// The API reference (Scalar, reading `/openapi.json`) and the contract
/// it shows, as generated in `crates/api`.
const DOCS: &str = include_str!("docs.html");
const OPENAPI: &str = include_str!("../../api/openapi.json");
const SCHEMA: &str = include_str!("../../api/schema.json");
const PROTOCOL_TS: &str = include_str!("../../api/protocol.ts");

fn served(kind: &'static str, body: &'static str) -> Response {
    ([(header::CONTENT_TYPE, kind)], body).into_response()
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
    if app.games.live() >= app.config.max_games {
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
        Err(e) => {
            // The cards players ask for (ids and counts only, no player
            // data), for the cards line's round choice.
            if let SpecError::Unsupported { cards, .. } = &e {
                let ids = demanded(cards);
                if let Err(why) = app.records.count_unsupported(&ids).await {
                    eprintln!("unsupported: {why}");
                }
            }
            refuse(StatusCode::BAD_REQUEST, e)
        }
    }
}

/// A refusal's card ids as counted: KRCG ids only, each once, at most a
/// deck's worth (a client sends what it likes).
fn demanded(cards: &[String]) -> Vec<u64> {
    let mut ids: Vec<u64> = cards
        .iter()
        .filter_map(|c| c.parse().ok())
        .filter(|id| (100_000..300_000).contains(id))
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids.truncate(200);
    ids
}

async fn record(State(app): State<Shared>, Path(id): Path<String>, headers: HeaderMap) -> Response {
    let Some(bearer) = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    // A game held answers from its thread; one gone from the store. Only
    // once over: a record holds the seed and every decision, so a player
    // replaying it mid-game would read the hidden cards.
    let over = |r: GameRecord| match r.result {
        Some(_) => Json(r).into_response(),
        None => StatusCode::CONFLICT.into_response(),
    };
    if let Some(h) = app.games.get(&id) {
        if bearer != h.token {
            return StatusCode::UNAUTHORIZED.into_response();
        }
        let (tx, rx) = oneshot::channel();
        if h.jobs.send(Job::Record(tx)).is_ok() {
            if let Ok(r) = rx.await {
                return over(r);
            }
        }
    }
    match app.records.record(&id, Some(&store::hash(bearer))).await {
        Some(r) => over(r),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Whether the request carries the admin token.
fn admin(app: &App, headers: &HeaderMap) -> bool {
    let Some(want) = app.config.admin.as_deref() else {
        return false;
    };
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|t| store::hash(t) == want)
}

#[derive(serde::Deserialize)]
struct Since {
    /// Unix ms.
    since: Option<u64>,
    engine: Option<String>,
    limit: Option<i64>,
}

/// The games recorded, oldest first (the admin's: training pulls them).
async fn admin_games(
    State(app): State<Shared>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<Since>,
) -> Response {
    if !admin(&app, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let limit = q.limit.unwrap_or(1000).clamp(1, 10_000);
    match app.records.list(q.since, q.engine.as_deref(), limit).await {
        Ok(games) => Json(games).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

#[derive(serde::Deserialize)]
struct FeedbackSince {
    /// Unix ms.
    since: Option<u64>,
    limit: Option<i64>,
}

/// The players' feedback, oldest first (the admin's: `intake` triages it).
async fn admin_feedback(
    State(app): State<Shared>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<FeedbackSince>,
) -> Response {
    if !admin(&app, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let limit = q.limit.unwrap_or(1000).clamp(1, 10_000);
    match app.records.feedback(q.since, limit).await {
        Ok(f) => Json(f).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

/// The card ids refused decks named, counted, the most asked first (the
/// admin's).
async fn admin_unsupported(State(app): State<Shared>, headers: HeaderMap) -> Response {
    if !admin(&app, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    match app.records.unsupported().await {
        Ok(counts) => {
            let list: Vec<_> = counts
                .into_iter()
                .map(|(id, count)| json!({"id": id, "count": count}))
                .collect();
            Json(list).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

/// Any game's record, as far as it went (the admin's).
async fn admin_record(
    State(app): State<Shared>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    if !admin(&app, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    match app.records.record(&id, None).await {
        Some(r) => Json(r).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn socket(
    State(app): State<Shared>,
    Path(id): Path<String>,
    headers: HeaderMap,
    // Checked after the game: a client's `fetch` (no upgrade headers) finds
    // a game gone by its 404, as a browser's socket cannot tell it.
    ws: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
) -> Response {
    // A browser says where the page is from: only the platforms' pages
    // may open a seat's socket. Other clients send no origin (and could
    // send any): the seat's token is what lets them in.
    if let (Some(origins), Some(origin)) = (&app.config.origins, headers.get("origin")) {
        let origin = origin.to_str().unwrap_or("");
        if !origins.iter().any(|o| o == origin) {
            return StatusCode::FORBIDDEN.into_response();
        }
    }
    if app.games.get(&id).is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
    match ws {
        Ok(ws) => ws.on_upgrade(move |s| serve(app, id, s)),
        Err(e) => e.into_response(),
    }
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

/// This connection's join, while it holds the seat: its number among the
/// seat's joins, watched for a later one.
struct Holding {
    mine: u64,
    joins: watch::Receiver<u64>,
}

impl Holding {
    fn taken(&self) -> bool {
        *self.joins.borrow() != self.mine
    }
}

/// Tells a connection its seat was joined elsewhere, and closes it.
async fn taken(mut ws: WebSocket) {
    let _ = ws.send(unnumbered(Error::SeatTaken)).await;
    let _ = ws.send(Ws::Close(None)).await;
}

async fn serve(app: Shared, id: String, mut ws: WebSocket) {
    let mut joined: Option<Holding> = None;
    let mut window = (Instant::now(), 0usize);
    let mut ping = tokio::time::interval(Duration::from_secs(30));
    loop {
        let msg = tokio::select! {
            m = ws.recv() => m,
            // A newer join of the seat (`changed` fails once the game is
            // gone: then only the client's next command finds it over).
            Some(true) = async {
                let h = joined.as_mut()?;
                h.joins.changed().await.ok()?;
                Some(h.taken())
            } => return taken(ws).await,
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
        match &joined {
            Some(holding) if holding.taken() => return taken(ws).await,
            Some(_) => {}
            None => match &input.command {
                Command::Join { token: Some(t), .. } if *t == h.token => {
                    let mut mine = 0;
                    h.joins.send_modify(|n| {
                        *n += 1;
                        mine = *n;
                    });
                    joined = Some(Holding {
                        mine,
                        joins: h.joins.subscribe(),
                    });
                }
                _ => {
                    let _ = ws.send(unnumbered(Error::NotJoined)).await;
                    continue;
                }
            },
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
