//! Live games: each on its own thread (the engine is CPU-bound and its
//! bots are not `Send`), reached by a channel; the record streamed to the
//! store as the game goes.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{mpsc::UnboundedSender, oneshot};
use zvtes_api::message::SpecError;
use zvtes_api::record::{BotRecord, GameRecord};
use zvtes_api::{Command, GameSpec, In, Out, Seat};
use zvtes_host::{Bots, Game, SystemClock};
use zvtes_policy::{Net, NetAgent, Nudges};

use crate::store::Put;

/// A request to a game's thread.
pub enum Job {
    Command {
        seat: Seat,
        input: In,
        reply: oneshot::Sender<Vec<Out>>,
    },
    Record(oneshot::Sender<GameRecord>),
}

pub struct Handle {
    pub token: String,
    pub seat: Seat,
    pub jobs: mpsc::Sender<Job>,
    /// Over: kept for a rejoin or its record, no longer counted live.
    pub over: Arc<AtomicBool>,
}

#[derive(Clone, Default)]
pub struct Games(Arc<Mutex<HashMap<String, Arc<Handle>>>>);

impl Games {
    pub fn get(&self, id: &str) -> Option<Arc<Handle>> {
        self.0.lock().unwrap().get(id).cloned()
    }

    /// Games under way (an ended game waiting out its idle time is not).
    pub fn live(&self) -> usize {
        self.0
            .lock()
            .unwrap()
            .values()
            .filter(|h| !h.over.load(Ordering::Relaxed))
            .count()
    }

    fn insert(&self, id: String, h: Handle) {
        self.0.lock().unwrap().insert(id, Arc::new(h));
    }

    fn remove(&self, id: &str) {
        self.0.lock().unwrap().remove(id);
    }
}

/// The trained policy, shared by every game, and its profiles.
pub struct Policy {
    pub net: Arc<Net>,
    /// The weights' SHA-256.
    pub hash: String,
}

/// A network profile: its temperature (the wasm module's measured
/// settings, `crates/wasm`; profiles move to `policy` with A4).
const PROFILES: [(&str, f32); 1] = [("network", 0.5)];

pub struct ServerBots(pub Option<Arc<Policy>>);

impl Bots for ServerBots {
    fn profiles(&self) -> Vec<String> {
        match &self.0 {
            Some(_) => PROFILES.iter().map(|(p, _)| p.to_string()).collect(),
            None => vec![],
        }
    }

    fn make(&self, profile: &str, seed: u32) -> Option<(Box<dyn zvtes::agent::Agent>, BotRecord)> {
        let p = self.0.as_ref()?;
        let &(name, temperature) = PROFILES.iter().find(|(n, _)| *n == profile)?;
        let agent = NetAgent::new(p.net.clone(), seed as u64)
            .temperature(temperature)
            .nudges(Nudges {
                rescue: 6.0,
                bleed: 4.0,
                idle: 3.0,
                draw: 7.0,
                stuck: 6.0,
                nightmare: 6.0,
                ..Nudges::default()
            });
        Some((
            Box::new(agent),
            BotRecord {
                seat: 0,
                profile: name.into(),
                kind: "network".into(),
                weights: Some(p.hash.clone()),
                temperature: Some(temperature),
                seed,
            },
        ))
    }
}

/// Starts a game on its thread; its id and the human's token once dealt.
pub async fn start(
    games: &Games,
    spec: GameSpec,
    policy: Option<Arc<Policy>>,
    store: UnboundedSender<Put>,
    idle: Duration,
) -> Result<(String, Seat, String), SpecError> {
    let id = random_hex(8);
    let token = random_hex(16);
    let (ready_tx, ready_rx) = oneshot::channel();
    let (jobs_tx, jobs_rx) = mpsc::channel::<Job>();
    let thread_id = id.clone();
    let registry = games.clone();
    let over = Arc::new(AtomicBool::new(false));
    let thread_over = over.clone();
    let token_hash = crate::store::hash(&token);
    std::thread::Builder::new()
        .name(format!("game-{id}"))
        .spawn(move || {
            let bots = ServerBots(policy);
            let mut game = match Game::new(thread_id.clone(), spec, &bots, Box::new(SystemClock)) {
                Ok(g) => g,
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                    return;
                }
            };
            let _ = store.send(Put::Start {
                game: thread_id.clone(),
                token_hash,
                record: GameRecord {
                    entries: vec![],
                    ..game.record().clone()
                },
            });
            let _ = ready_tx.send(Ok(game.human()));
            run(&mut game, &jobs_rx, &store, idle, &thread_over);
            registry.remove(&thread_id);
        })
        .expect("a game thread");
    let seat = ready_rx.await.expect("the game thread answers")?;
    games.insert(
        id.clone(),
        Handle {
            token: token.clone(),
            seat,
            jobs: jobs_tx,
            over,
        },
    );
    Ok((id, seat, token))
}

/// Serves a game's jobs until it is idle too long; then it ends (as
/// abandoned if not over) and the thread exits.
fn run(
    game: &mut Game,
    jobs: &mpsc::Receiver<Job>,
    store: &UnboundedSender<Put>,
    idle: Duration,
    over: &AtomicBool,
) {
    let mut flushed = 0;
    let mut ended = false;
    loop {
        match jobs.recv_timeout(idle) {
            Ok(Job::Command { seat, input, reply }) => {
                let _ = reply.send(game.handle(seat, input));
            }
            Ok(Job::Record(reply)) => {
                let _ = reply.send(game.record().clone());
            }
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => {
                if !game.is_over() {
                    let human = game.human();
                    game.handle(
                        human,
                        In {
                            id: None,
                            command: Command::Leave,
                        },
                    );
                }
                flush(game, store, &mut flushed, &mut ended);
                return;
            }
        }
        flush(game, store, &mut flushed, &mut ended);
        over.store(game.is_over(), Ordering::Relaxed);
    }
}

fn flush(game: &Game, store: &UnboundedSender<Put>, flushed: &mut usize, ended: &mut bool) {
    let r = game.record();
    if r.entries.len() > *flushed {
        let _ = store.send(Put::Entries {
            game: game.id().into(),
            from: *flushed,
            entries: r.entries[*flushed..].to_vec(),
        });
        *flushed = r.entries.len();
    }
    if game.is_over() && !*ended {
        *ended = true;
        let _ = store.send(Put::End {
            game: game.id().into(),
            record: GameRecord {
                entries: vec![],
                ..r.clone()
            },
        });
    }
}

fn random_hex(bytes: usize) -> String {
    use std::io::Read;
    let mut b = vec![0u8; bytes];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut b))
        .expect("random bytes");
    b.iter().map(|x| format!("{x:02x}")).collect()
}
