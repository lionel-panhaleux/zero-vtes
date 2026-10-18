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
use zvtes_api::record::{Entry, GameRecord};
use zvtes_api::{GameSpec, In, Out, Seat};
use zvtes_host::{Game, SystemClock};
use zvtes_policy::profiles::{Profiles, Served};

use crate::store::{Put, Store};

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
    /// The seat token's SHA-256 (`store::hash`): all a resumed game has.
    pub token_hash: String,
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
    let token_hash_kept = token_hash.clone();
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
            run(&mut game, &jobs_rx, &store, idle, &thread_over, 0);
            registry.remove(&thread_id);
        })
        .expect("a game thread");
    let seat = ready_rx.await.expect("the game thread answers")?;
    games.insert(
        id.clone(),
        Handle {
            token_hash: token_hash_kept,
            seat,
            jobs: jobs_tx,
            over,
            joins: watch::channel(0).0,
        },
    );
    Ok((id, seat, token))
}

/// A game left going by a previous process, resumed on its thread from
/// its record (`Game::resume`), its seat's token known by its hash.
pub async fn resume(
    games: &Games,
    id: String,
    record: GameRecord,
    token_hash: String,
    policy: Option<Arc<Served>>,
    store: UnboundedSender<Put>,
    idle: Duration,
) -> Result<(), String> {
    let (ready_tx, ready_rx) = oneshot::channel();
    let (jobs_tx, jobs_rx) = mpsc::channel::<Job>();
    let thread_id = id.clone();
    let registry = games.clone();
    let over = Arc::new(AtomicBool::new(false));
    let thread_over = over.clone();
    std::thread::Builder::new()
        .name(format!("game-{id}"))
        .spawn(move || {
            let bots = Profiles(policy);
            // A panic in the replay ends this game, not the server's start.
            let resumed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                Game::resume(thread_id.clone(), &record, &bots, Box::new(SystemClock))
            }));
            let mut game = match resumed {
                Ok(Ok(g)) => g,
                Ok(Err(e)) => {
                    let _ = ready_tx.send(Err(e));
                    return;
                }
                Err(_) => {
                    let _ = ready_tx.send(Err("the replay panicked".into()));
                    return;
                }
            };
            let _ = ready_tx.send(Ok(game.human()));
            let flushed = record.entries.len();
            run(&mut game, &jobs_rx, &store, idle, &thread_over, flushed);
            registry.remove(&thread_id);
        })
        .expect("a game thread");
    let seat = ready_rx
        .await
        .map_err(|_| "the game thread ended".to_string())??;
    games.insert(
        id,
        Handle {
            token_hash,
            seat,
            jobs: jobs_tx,
            over,
            joins: watch::channel(0).0,
        },
    );
    Ok(())
}

/// The games a previous process left going: each its idle time has not
/// run out on resumed from its record (`Game::resume`), the others ended
/// `server_restart`, as one that cannot be resumed (a rules change, its
/// bots no longer served). How many resumed, how many ended, and why some
/// could not be ended.
pub async fn restore(
    games: &Games,
    store: &Store,
    policy: Option<Arc<Served>>,
    put: UnboundedSender<Put>,
    idle: Duration,
) -> (usize, usize, Vec<String>) {
    let going = match store.going().await {
        Ok(g) => g,
        Err(e) => return (0, 0, vec![e]),
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after 1970")
        .as_millis() as u64;
    // Each resumed on its own thread at once, all awaited before the
    // server takes connections.
    let mut tasks = tokio::task::JoinSet::new();
    let mut ending = Vec::new();
    for (game, seats) in going {
        let record = store.record(&game, None).await;
        let token_hash = store.token_hash(&game).await;
        let (Some(record), Some(token_hash)) = (record, token_hash) else {
            ending.push((game, seats));
            continue;
        };
        let last = record.started + record.entries.iter().map(entry_at).max().unwrap_or(0);
        if now.saturating_sub(last) >= idle.as_millis() as u64 {
            ending.push((game, seats));
            continue;
        }
        let (games, policy, put) = (games.clone(), policy.clone(), put.clone());
        tasks.spawn(async move {
            let done = resume(&games, game.clone(), record, token_hash, policy, put, idle).await;
            (game, seats, done)
        });
    }
    let mut resumed = 0;
    while let Some(joined) = tasks.join_next().await {
        let (game, seats, done) = joined.expect("a resume task");
        match done {
            Ok(()) => resumed += 1,
            Err(e) => {
                eprintln!("restart: {game} not resumed: {e}");
                ending.push((game, seats));
            }
        }
    }
    let (mut ended, mut failed) = (0, Vec::new());
    for (game, seats) in ending {
        match store.end_lost_game(&game, seats).await {
            Ok(()) => ended += 1,
            Err(e) => failed.push(format!("{game}: {e}")),
        }
    }
    (resumed, ended, failed)
}

/// When an entry was written, in milliseconds since its game started.
fn entry_at(e: &Entry) -> u64 {
    match e {
        Entry::Decision { at, .. }
        | Entry::Undo { at, .. }
        | Entry::Settings { at, .. }
        | Entry::Refused { at, .. }
        | Entry::Feedback { at, .. } => *at,
        Entry::Unknown => 0,
    }
}

/// Serves a game's jobs until it is idle too long; then it ends (as
/// abandoned if not over) and the thread exits. `flushed`: the record's
/// entries already stored.
fn run(
    game: &mut Game,
    jobs: &mpsc::Receiver<Job>,
    store: &UnboundedSender<Put>,
    idle: Duration,
    over: &AtomicBool,
    mut flushed: usize,
) {
    let mut ended = false;
    // A resumed game its replay ended: stored ended at once.
    flush(game, store, &mut flushed, &mut ended);
    over.store(game.is_over(), Ordering::Relaxed);
    loop {
        match jobs.recv_timeout(idle) {
            Ok(Job::Command { seat, input, reply }) => {
                let _ = reply.send(game.handle(seat, input));
            }
            Ok(Job::Record(reply)) => {
                let _ = reply.send(game.record().clone());
            }
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => {
                game.abandon();
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

/// A deal's seed from the system's randomness, 53 bits (`SEED_MAX`).
pub fn random_seed() -> u64 {
    u64::from_le_bytes(random_bytes()) & zvtes_api::SEED_MAX
}

fn random_hex<const N: usize>() -> String {
    random_bytes::<N>()
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use zvtes_api::message::{Decision, EndReason, Message};
    use zvtes_api::{Command, Player, SeatSpec, Settings};

    use super::*;

    /// The server's seeds are 53 bits (protocol 2.0.0): none past
    /// `SEED_MAX`, and past 32 bits all but always.
    #[test]
    fn a_seed_is_drawn_from_53_bits() {
        let seeds: Vec<u64> = (0..64).map(|_| random_seed()).collect();
        assert!(seeds.iter().all(|&s| s <= zvtes_api::SEED_MAX));
        assert!(seeds.iter().any(|&s| s > u32::MAX as u64));
    }

    fn spec() -> GameSpec {
        let (_, deck) = zvtes_host::catalog().remove(0);
        GameSpec {
            seed: Some(7),
            time_limit: None,
            client: None,
            seats: (0..4)
                .map(|i| SeatSpec {
                    name: format!("p{i}"),
                    deck: deck.clone(),
                    player: if i == 0 {
                        Player::Human {
                            settings: Settings::default(),
                        }
                    } else {
                        Player::Bot {
                            profile: "wall".into(),
                        }
                    },
                })
                .collect(),
        }
    }

    async fn ask(h: &Handle, command: Command) -> Vec<Out> {
        let (reply, rx) = oneshot::channel();
        h.jobs
            .send(Job::Command {
                seat: h.seat,
                input: In { id: None, command },
                reply,
            })
            .unwrap();
        rx.await.unwrap()
    }

    fn pending(outs: &[Out]) -> Option<Decision> {
        outs.iter().rev().find_map(|o| match &o.message {
            Message::Decision(d) => Some(d.clone()),
            _ => None,
        })
    }

    // A game never joined ends at its idle timeout too (docs/protocol.md
    // §2), recorded abandoned: its seat's leave would be refused, not
    // joined, and the record left going.
    #[tokio::test]
    async fn a_game_never_joined_ends_at_its_idle_timeout() {
        let dir = std::env::temp_dir().join(format!("zvtes-unjoined-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::open(None, dir.clone()).await;
        let (put, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(store.clone().run(rx));
        let games = Games::default();
        let (id, seat, _) = start(&games, spec(), None, put, Duration::from_millis(50))
            .await
            .unwrap();
        let mut ended = None;
        for _ in 0..300 {
            ended = store.record(&id, None).await.filter(|r| r.ended.is_some());
            if ended.is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let r = ended.expect("recorded ended");
        assert_eq!(
            r.result.map(|g| g.reason),
            Some(EndReason::Abandoned { seat })
        );
        assert!(r.entries.is_empty(), "nothing refused: {:?}", r.entries);
        assert!(!store.going().await.unwrap().iter().any(|(g, _)| *g == id));
        let _ = std::fs::remove_dir_all(&dir);
    }

    // A server restart resumes a game left going (docs/protocol.md §2): the
    // seat's token joins it again, a fresh start at the decision it was
    // at, the same options, and it plays on; a game past its idle time is
    // ended `server_restart` instead.
    #[tokio::test]
    async fn a_game_left_going_is_resumed_at_its_decision() {
        let dir = std::env::temp_dir().join(format!("zvtes-resume-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::open(None, dir.clone()).await;
        let (put, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(store.clone().run(rx));
        let idle = Duration::from_secs(60);
        let games = Games::default();
        let (id, _, token) = start(&games, spec(), None, put.clone(), idle)
            .await
            .unwrap();
        let h = games.get(&id).unwrap();
        let mut outs = ask(
            &h,
            Command::Join {
                token: Some(token.clone()),
                after: None,
            },
        )
        .await;
        for _ in 0..12 {
            let d = pending(&outs).expect("a decision");
            outs = ask(
                &h,
                Command::Answer {
                    decision: d.decision,
                    option: d.options[0].index,
                },
            )
            .await;
        }
        let before = pending(&outs).expect("a decision");
        // The client's numbering is the replay's own (nothing refused):
        // a join after it still a fresh start, never a resend of nothing.
        let seq = outs.last().unwrap().seq;
        let (tx, rx) = oneshot::channel();
        h.jobs.send(Job::Record(tx)).unwrap();
        let record = rx.await.unwrap();
        // The store caught up; a copy of the game started two idle times ago.
        while store.record(&id, None).await.map_or(0, |r| r.entries.len()) < record.entries.len() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let stale = GameRecord {
            started: record.started - 2 * idle.as_millis() as u64,
            entries: vec![],
            ..record.clone()
        };
        put.send(Put::Start {
            game: "stale".into(),
            token_hash: crate::store::hash("other"),
            record: stale,
        })
        .unwrap();
        put.send(Put::Entries {
            game: "stale".into(),
            from: 0,
            entries: record.entries.clone(),
        })
        .unwrap();
        while store
            .record("stale", None)
            .await
            .map_or(0, |r| r.entries.len())
            < record.entries.len()
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        // The restart: a new process's registry.
        let restarted = Games::default();
        let (resumed, ended, failed) = restore(&restarted, &store, None, put, idle).await;
        assert_eq!((resumed, ended, failed), (1, 1, vec![]));
        let h = restarted.get(&id).expect("resumed");
        let outs = ask(
            &h,
            Command::Join {
                token: Some(token),
                after: Some(seq),
            },
        )
        .await;
        assert!(
            matches!(outs[0].message, Message::Joined(_)),
            "a fresh start"
        );
        let after = pending(&outs).expect("the decision");
        assert_eq!(after.decision, before.decision);
        let keys = |d: &Decision| d.options.iter().map(|o| o.key.clone()).collect::<Vec<_>>();
        assert_eq!(keys(&after), keys(&before));
        let next = ask(
            &h,
            Command::Answer {
                decision: after.decision,
                option: after.options[0].index,
            },
        )
        .await;
        assert!(
            next.iter()
                .any(|o| matches!(o.message, Message::Decision(_) | Message::Over(_))),
            "it plays on"
        );
        let stale = store.record("stale", None).await.expect("the stale game");
        assert_eq!(
            stale.result.expect("ended").reason,
            EndReason::ServerRestart
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
