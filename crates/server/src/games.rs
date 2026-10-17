//! Live games: each on its own thread (the engine is CPU-bound and its
//! bots are not `Send`), reached by a channel; the record streamed to the
//! store as the game goes.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{mpsc::UnboundedSender, oneshot, watch};
use zvtes_api::message::SpecError;
use zvtes_api::record::GameRecord;
use zvtes_api::{Command, GameSpec, In, Out, Seat};
use zvtes_host::{Game, SystemClock};
use zvtes_policy::profiles::{Profiles, Served};

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
    /// The seat's joins, counted: the connection holding the latest plays
    /// the seat, an older one is closed (docs/protocol.md §2).
    pub joins: watch::Sender<u64>,
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

/// Starts a game on its thread; its id and the human's token once dealt.
pub async fn start(
    games: &Games,
    spec: GameSpec,
    policy: Option<Arc<Served>>,
    store: UnboundedSender<Put>,
    idle: Duration,
) -> Result<(String, Seat, String), SpecError> {
    let id = random_hex::<8>();
    let token = random_hex::<16>();
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
            let bots = Profiles(policy);
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
            joins: watch::channel(0).0,
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

/// `N` bytes of the system's randomness.
fn random_bytes<const N: usize>() -> [u8; N] {
    use std::io::Read;
    let mut b = [0u8; N];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut b))
        .expect("random bytes");
    b
}

/// A deal's seed from the system's randomness.
pub fn random_seed() -> u32 {
    u32::from_le_bytes(random_bytes())
}

fn random_hex<const N: usize>() -> String {
    random_bytes::<N>()
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}
