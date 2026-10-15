//! Where records go: entries appended as the game goes, so a crash keeps
//! the game. Postgres in production (`games`, `game_entries`), a
//! directory of JSON lines in development.

use std::io::Write;
use std::path::PathBuf;

use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedReceiver;
use zvtes_api::message::EndReason;
use zvtes_api::record::{Entry, GameRecord, GameResult};

/// What a game sends its store.
pub enum Put {
    /// The game begins: its record without entries, and its seat
    /// token's hash (who may read the record later).
    Start {
        game: String,
        token_hash: String,
        record: GameRecord,
    },
    /// Entries from position `from` (positions count from 0).
    Entries {
        game: String,
        from: usize,
        entries: Vec<Entry>,
    },
    /// The game ended: its record whole (result, end time).
    End { game: String, record: GameRecord },
}

/// A game's row in the admin's list: id, engine, client, started and
/// ended (Unix ms), result, entries.
type SummaryRow = (
    String,
    String,
    Option<String>,
    f64,
    Option<f64>,
    Option<serde_json::Value>,
    i64,
);

/// A game as the admin's list shows it.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Summary {
    pub id: String,
    pub engine: String,
    pub client: Option<String>,
    /// Unix ms.
    pub started: u64,
    pub ended: Option<u64>,
    pub result: Option<serde_json::Value>,
    /// Its record's entries so far.
    pub entries: u64,
}

/// A player's feedback as the admin's list shows it: the game, its engine,
/// when (Unix ms), the decision it is about, the text.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Feedback {
    pub game: String,
    pub engine: String,
    pub at: u64,
    pub decision: u64,
    pub text: String,
}

#[derive(Clone)]
pub enum Store {
    Postgres(PgPool),
    Files(PathBuf),
}

const SCHEMA: &str = "
create table if not exists games (
    id text primary key,
    token_hash text not null,
    protocol text not null,
    engine text not null,
    client text,
    spec jsonb not null,
    bots jsonb not null,
    started timestamptz not null,
    ended timestamptz,
    result jsonb
);
create table if not exists game_entries (
    game text not null references games(id) on delete cascade,
    n integer not null,
    entry jsonb not null,
    primary key (game, n)
);
create index if not exists games_started on games (started);
";

impl Store {
    pub async fn open(database: Option<String>, dir: PathBuf) -> Store {
        match database {
            Some(url) => {
                let pool = PgPool::connect(&url).await.expect("the database");
                sqlx::raw_sql(SCHEMA)
                    .execute(&pool)
                    .await
                    .expect("the schema");
                Store::Postgres(pool)
            }
            None => {
                std::fs::create_dir_all(&dir).expect("the records directory");
                Store::Files(dir)
            }
        }
    }

    /// Writes until every game's sender is gone.
    pub async fn run(self, mut rx: UnboundedReceiver<Put>) {
        while let Some(w) = rx.recv().await {
            if let Err(e) = self.write(w).await {
                eprintln!("store: {e}");
            }
        }
    }

    async fn write(&self, w: Put) -> Result<(), String> {
        match self {
            Store::Files(dir) => {
                let (game, line) = match &w {
                    Put::Start {
                        game,
                        token_hash,
                        record,
                    } => (game, json(&("start", token_hash, record))),
                    Put::Entries {
                        game,
                        from,
                        entries,
                    } => (game, json(&("entries", from, entries))),
                    Put::End { game, record } => (game, json(&("end", record))),
                };
                let mut f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(dir.join(format!("{game}.jsonl")))
                    .map_err(|e| e.to_string())?;
                writeln!(f, "{line}").map_err(|e| e.to_string())
            }
            Store::Postgres(pool) => match w {
                Put::Start {
                    game,
                    token_hash,
                    record,
                } => sqlx::query(
                    "insert into games (id, token_hash, protocol, engine, client, spec, bots, started)
                     values ($1, $8, $2, $3, $4, $5, $6, to_timestamp($7::double precision / 1000))",
                )
                .bind(&game)
                .bind(&record.protocol)
                .bind(&record.engine)
                .bind(&record.spec.client)
                .bind(value(&record.spec))
                .bind(value(&record.bots))
                .bind(record.started as f64)
                .bind(&token_hash)
                .execute(pool)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string()),
                Put::Entries {
                    game,
                    from,
                    entries,
                } => {
                    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
                    for (i, e) in entries.iter().enumerate() {
                        sqlx::query(
                            "insert into game_entries (game, n, entry) values ($1, $2, $3)
                             on conflict do nothing",
                        )
                        .bind(&game)
                        .bind((from + i) as i32)
                        .bind(value(e))
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| e.to_string())?;
                    }
                    tx.commit().await.map_err(|e| e.to_string())
                }
                Put::End { game, record } => {
                    let ended = record.ended.unwrap_or(record.started);
                    end_row(pool, &game, ended, value(&record.result)).await
                }
            },
        }
    }

    /// A game's record (as far as it went), for the holder of its seat
    /// token (`None`: the admin).
    pub async fn record(&self, game: &str, token_hash: Option<&str>) -> Option<GameRecord> {
        match self {
            Store::Postgres(pool) => {
                let row: (
                    serde_json::Value,
                    serde_json::Value,
                    String,
                    String,
                    f64,
                    Option<f64>,
                    Option<serde_json::Value>,
                ) = sqlx::query_as(
                    "select spec, bots, protocol, engine,
                                extract(epoch from started)::double precision * 1000,
                                extract(epoch from ended)::double precision * 1000, result
                         from games where id = $1 and ($2::text is null or token_hash = $2)",
                )
                .bind(game)
                .bind(token_hash)
                .fetch_optional(pool)
                .await
                .ok()??;
                let entries: Vec<(serde_json::Value,)> =
                    sqlx::query_as("select entry from game_entries where game = $1 order by n")
                        .bind(game)
                        .fetch_all(pool)
                        .await
                        .ok()?;
                Some(GameRecord {
                    protocol: row.2,
                    engine: row.3,
                    spec: serde_json::from_value(row.0).ok()?,
                    bots: serde_json::from_value(row.1).ok()?,
                    started: row.4 as u64,
                    ended: row.5.map(|t| t as u64),
                    // Whole or not at all, as the file store reads: a
                    // dropped entry would replay another game, and leave
                    // the record short of the listing's count.
                    entries: entries
                        .into_iter()
                        .map(|(e,)| serde_json::from_value(e))
                        .collect::<Result<_, _>>()
                        .ok()?,
                    result: row.6.and_then(|r| serde_json::from_value(r).ok()),
                })
            }
            Store::Files(dir) => {
                let text = std::fs::read_to_string(dir.join(format!("{game}.jsonl"))).ok()?;
                let mut record: Option<GameRecord> = None;
                for line in text.lines() {
                    let v: serde_json::Value = serde_json::from_str(line).ok()?;
                    match v[0].as_str()? {
                        "start" if token_hash.is_none_or(|h| v[1].as_str() == Some(h)) => {
                            record = serde_json::from_value(v[2].clone()).ok();
                        }
                        "entries" => {
                            let more: Vec<Entry> = serde_json::from_value(v[2].clone()).ok()?;
                            record.as_mut()?.entries.extend(more);
                        }
                        "end" => {
                            let end: GameRecord = serde_json::from_value(v[1].clone()).ok()?;
                            let r = record.as_mut()?;
                            r.ended = end.ended;
                            r.result = end.result;
                        }
                        _ => {}
                    }
                }
                record
            }
        }
    }

    /// The games started since `since` (Unix ms), of engine `engine`, at
    /// most `limit`, oldest first: what the admin pulls.
    pub async fn list(
        &self,
        since: Option<u64>,
        engine: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Summary>, String> {
        match self {
            Store::Postgres(pool) => {
                let rows: Vec<SummaryRow> = sqlx::query_as(
                    "select g.id, g.engine, g.client,
                            extract(epoch from g.started)::double precision * 1000,
                            extract(epoch from g.ended)::double precision * 1000, g.result,
                            (select count(*) from game_entries e where e.game = g.id)
                     from games g
                     where ($1::double precision is null
                            or g.started >= to_timestamp($1::double precision / 1000))
                       and ($2::text is null or g.engine = $2)
                     order by g.started limit $3",
                )
                .bind(since.map(|s| s as f64))
                .bind(engine)
                .bind(limit)
                .fetch_all(pool)
                .await
                .map_err(|e| e.to_string())?;
                Ok(rows
                    .into_iter()
                    .map(|r| Summary {
                        id: r.0,
                        engine: r.1,
                        client: r.2,
                        started: r.3 as u64,
                        ended: r.4.map(|t| t as u64),
                        result: r.5,
                        entries: r.6 as u64,
                    })
                    .collect())
            }
            Store::Files(dir) => {
                let mut out = Vec::new();
                let files = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
                for f in files.flatten() {
                    let path = f.path();
                    let Some(id) = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .and_then(|n| n.strip_suffix(".jsonl"))
                    else {
                        continue;
                    };
                    let Some(r) = self.record(id, None).await else {
                        continue;
                    };
                    if since.is_some_and(|s| r.started < s) || engine.is_some_and(|e| e != r.engine)
                    {
                        continue;
                    }
                    out.push(Summary {
                        id: id.into(),
                        engine: r.engine,
                        client: r.spec.client,
                        started: r.started,
                        ended: r.ended,
                        result: r.result.map(|x| value(&x)),
                        entries: r.entries.len() as u64,
                    });
                }
                out.sort_by_key(|s| s.started);
                out.truncate(limit.max(0) as usize);
                Ok(out)
            }
        }
    }

    /// The feedback sent since `since` (Unix ms), at most `limit`, oldest
    /// first: what the `intake` skill triages.
    pub async fn feedback(&self, since: Option<u64>, limit: i64) -> Result<Vec<Feedback>, String> {
        let out = match self {
            Store::Postgres(pool) => {
                let rows: Vec<(String, String, f64, i64, String)> = sqlx::query_as(
                    "select g.id, g.engine,
                            extract(epoch from g.started)::double precision * 1000
                                + (e.entry->>'at')::double precision as at,
                            (e.entry->>'decision')::bigint, e.entry->>'text'
                     from game_entries e join games g on g.id = e.game
                     where e.entry->>'type' = 'feedback'
                       and ($1::double precision is null
                            or extract(epoch from g.started)::double precision * 1000
                                + (e.entry->>'at')::double precision >= $1)
                     order by at limit $2",
                )
                .bind(since.map(|s| s as f64))
                .bind(limit)
                .fetch_all(pool)
                .await
                .map_err(|e| e.to_string())?;
                rows.into_iter()
                    .map(|r| Feedback {
                        game: r.0,
                        engine: r.1,
                        at: r.2 as u64,
                        decision: r.3 as u64,
                        text: r.4,
                    })
                    .collect()
            }
            Store::Files(_) => {
                let mut out = Vec::new();
                for g in self.list(None, None, i64::MAX).await? {
                    let Some(r) = self.record(&g.id, None).await else {
                        continue;
                    };
                    for e in &r.entries {
                        if let Entry::Feedback {
                            at, decision, text, ..
                        } = e
                        {
                            out.push(Feedback {
                                game: g.id.clone(),
                                engine: r.engine.clone(),
                                at: r.started + at,
                                decision: *decision,
                                text: text.clone(),
                            });
                        }
                    }
                }
                out.sort_by_key(|f| f.at);
                out.retain(|f| since.is_none_or(|s| f.at >= s));
                out.truncate(limit.max(0) as usize);
                out
            }
        };
        Ok(out)
    }

    /// The games still going, ended `server_restart`: run at startup,
    /// before any game is live, so a going game is one a previous process
    /// lost. How many it ended, and why it could not end the others.
    pub async fn end_lost(&self) -> (usize, Vec<String>) {
        let going: Vec<(String, i32)> = match self {
            Store::Postgres(pool) => match sqlx::query_as(
                "select id, case when jsonb_typeof(spec->'seats') = 'array'
                     then jsonb_array_length(spec->'seats') else 0 end
                 from games where ended is null",
            )
            .fetch_all(pool)
            .await
            {
                Ok(rows) => rows,
                Err(e) => return (0, vec![e.to_string()]),
            },
            // A file whose record does not read is not listed either.
            Store::Files(_) => match self.list(None, None, i64::MAX).await {
                Ok(games) => games
                    .into_iter()
                    .filter(|g| g.ended.is_none())
                    .map(|g| (g.id, 0))
                    .collect(),
                Err(e) => return (0, vec![e]),
            },
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("after 1970")
            .as_millis() as u64;
        let (mut ended, mut failed) = (0, Vec::new());
        for (game, seats) in going {
            let done = match (self.record(&game, None).await, self) {
                (Some(record), _) => {
                    let result = zvtes_host::replay::ended(&record, EndReason::ServerRestart);
                    self.write(Put::End {
                        game: game.clone(),
                        record: GameRecord {
                            entries: vec![],
                            ended: Some(now),
                            result: Some(result),
                            ..record
                        },
                    })
                    .await
                }
                // A row that no longer reads as a record (an older api).
                (None, Store::Postgres(pool)) => {
                    let result = GameResult {
                        vp_halves: vec![0; seats.max(0) as usize],
                        winner: None,
                        reason: EndReason::ServerRestart,
                        failure: Some("the record does not read".into()),
                    };
                    end_row(pool, &game, now, value(&result)).await
                }
                (None, Store::Files(_)) => Err("the record does not read".into()),
            };
            match done {
                Ok(()) => ended += 1,
                Err(e) => failed.push(format!("{game}: {e}")),
            }
        }
        (ended, failed)
    }

    /// Games started before `days` ago, deleted (the 2-year retention).
    pub async fn purge(pool: &PgPool, days: i32) -> Result<u64, String> {
        sqlx::query("delete from games where started < now() - make_interval(days => $1)")
            .bind(days)
            .execute(pool)
            .await
            .map(|r| r.rows_affected())
            .map_err(|e| e.to_string())
    }
}

/// A game's row ended (Unix ms) with its result.
async fn end_row(
    pool: &PgPool,
    game: &str,
    ended: u64,
    result: serde_json::Value,
) -> Result<(), String> {
    sqlx::query(
        "update games set ended = to_timestamp($2::double precision / 1000),
         result = $3 where id = $1",
    )
    .bind(game)
    .bind(ended as f64)
    .bind(result)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// A token's SHA-256: what the store keeps of it.
pub fn hash(token: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

fn json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).expect("records serialize")
}

fn value<T: serde::Serialize>(v: &T) -> serde_json::Value {
    serde_json::to_value(v).expect("records serialize")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Postgres tests share one scratch database, and `end_lost` ends
    /// every game going in it: one at a time.
    static SCRATCH: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    fn corpus(name: &str) -> GameRecord {
        let path = format!("{}/../../data/records/{name}", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).expect("the record")).expect("a record")
    }

    async fn start(store: &Store, game: &str, mut record: GameRecord) {
        let entries = std::mem::take(&mut record.entries);
        record.ended = None;
        record.result = None;
        let game = game.to_string();
        store
            .write(Put::Start {
                game: game.clone(),
                token_hash: hash("token"),
                record,
            })
            .await
            .unwrap();
        store
            .write(Put::Entries {
                game,
                from: 0,
                entries,
            })
            .await
            .unwrap();
    }

    fn result(r: Option<GameRecord>) -> GameResult {
        r.expect("the record").result.expect("a result")
    }

    /// A store holding three games going (one whose record reads whole,
    /// one partway, one not at all) and one finished; `n`: the games
    /// `end_lost` ends.
    async fn restart(store: &Store, id: impl Fn(&str) -> String, n: usize) {
        let whole = corpus("zero-vtes-replay-2026-10-04T12-09-51-567Z.json");
        let played = whole.result.clone().expect("a finished game");
        start(store, &id("going"), whole.clone()).await;
        let mut unseeded = whole.clone();
        unseeded.spec.seed = None;
        start(store, &id("unreadable"), unseeded).await;
        // A vote cast at decision 100, where plays are offered: the record
        // reads as far as a later engine reads an older one.
        let mut changed = whole.clone();
        let Entry::Decision { choice, .. } = changed
            .entries
            .iter_mut()
            .find(|e| matches!(e, Entry::Decision { decision: 100, .. }))
            .unwrap()
        else {
            unreachable!()
        };
        choice.kind = zvtes_api::ids::OptionKind::CastVotes;
        start(store, &id("partway"), changed).await;
        start(store, &id("finished"), whole.clone()).await;
        store
            .write(Put::End {
                game: id("finished"),
                record: GameRecord {
                    entries: vec![],
                    ..whole.clone()
                },
            })
            .await
            .unwrap();

        assert_eq!(store.end_lost().await, (n, vec![]));
        let listed = store.list(None, None, 1000).await.unwrap();
        assert!(listed.iter().all(|g| g.ended.is_some()));
        let going = result(store.record(&id("going"), None).await);
        assert_eq!(going.reason, EndReason::ServerRestart);
        assert_eq!(
            (going.vp_halves, going.winner),
            (played.vp_halves, played.winner)
        );
        assert_eq!(going.failure, None);
        let unreadable = result(store.record(&id("unreadable"), None).await);
        assert_eq!(unreadable.reason, EndReason::ServerRestart);
        assert_eq!(unreadable.vp_halves, vec![0; 4]);
        assert!(unreadable.failure.is_some());
        let partway = result(store.record(&id("partway"), None).await);
        assert_eq!(partway.reason, EndReason::ServerRestart);
        assert_eq!(partway.vp_halves.len(), 4);
        assert_eq!(
            partway.failure.as_deref(),
            Some("the record reads 100 of its 369 decisions")
        );
        let finished = result(store.record(&id("finished"), None).await);
        assert_eq!(finished.reason, played.reason);
        // Nothing is left going: a second restart ends none.
        assert_eq!(store.end_lost().await, (0, vec![]));
    }

    /// Feedback stored with its game's entries is listed with the game, its
    /// engine, when and the decision, oldest first, from `since`.
    async fn feedback_listed(store: &Store, id: impl Fn(&str) -> String) {
        let mut record = corpus("zero-vtes-replay-2026-10-04T12-09-51-567Z.json");
        record.started = 1_000_000;
        for (at, decision, text) in [(5_000, 12, "a"), (9_000, 40, "b")] {
            record.entries.push(Entry::Feedback {
                seat: 0,
                at,
                decision,
                text: text.into(),
            });
        }
        start(store, &id("fed"), record.clone()).await;
        let all = store.feedback(None, 100).await.unwrap();
        let ours: Vec<_> = all.iter().filter(|f| f.game == id("fed")).collect();
        assert_eq!(ours.len(), 2);
        assert_eq!(
            (ours[0].at, ours[0].decision, ours[0].text.as_str()),
            (1_005_000, 12, "a")
        );
        assert_eq!(ours[0].engine, record.engine);
        let later = store.feedback(Some(1_006_000), 100).await.unwrap();
        assert!(later.iter().any(|f| f.game == id("fed") && f.text == "b"));
        assert!(!later.iter().any(|f| f.game == id("fed") && f.text == "a"));
    }

    #[tokio::test]
    async fn feedback_is_listed_for_the_admin() {
        let dir = std::env::temp_dir().join(format!("zvtes-feedback-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::open(None, dir.clone()).await;
        feedback_listed(&store, |g| g.to_string()).await;
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn feedback_is_listed_for_the_admin_on_postgres() {
        let Ok(url) = std::env::var("ZVTES_TEST_DATABASE_URL") else {
            eprintln!("skipped: no ZVTES_TEST_DATABASE_URL");
            return;
        };
        let _scratch = SCRATCH.lock().await;
        let store = Store::open(Some(url), PathBuf::new()).await;
        let Store::Postgres(pool) = &store else {
            unreachable!()
        };
        let run = format!("{}-{}", std::process::id(), random());
        feedback_listed(&store, |g| format!("{run}-{g}")).await;
        sqlx::query("delete from games where id like $1")
            .bind(format!("{run}-%"))
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn a_restart_ends_the_games_left_going() {
        let dir = std::env::temp_dir().join(format!("zvtes-restart-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::open(None, dir.clone()).await;
        restart(&store, |g| g.to_string(), 3).await;
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The same on Postgres, plus a row that no longer reads as a record:
    /// with `ZVTES_TEST_DATABASE_URL`, a scratch database (the test ends
    /// every game going in it).
    #[tokio::test]
    async fn a_restart_ends_the_games_left_going_on_postgres() {
        let Ok(url) = std::env::var("ZVTES_TEST_DATABASE_URL") else {
            eprintln!("skipped: no ZVTES_TEST_DATABASE_URL");
            return;
        };
        let _scratch = SCRATCH.lock().await;
        let store = Store::open(Some(url), PathBuf::new()).await;
        let Store::Postgres(pool) = &store else {
            unreachable!()
        };
        let run = format!("{}-{}", std::process::id(), random());
        let id = |g: &str| format!("{run}-{g}");
        start(
            &store,
            &id("old-api"),
            corpus("zero-vtes-replay-2026-10-04T12-09-51-567Z.json"),
        )
        .await;
        sqlx::query("update games set bots = '\"not bots\"' where id = $1")
            .bind(id("old-api"))
            .execute(pool)
            .await
            .unwrap();
        restart(&store, id, 4).await;
        let row: (Option<serde_json::Value>,) =
            sqlx::query_as("select result from games where id = $1")
                .bind(id("old-api"))
                .fetch_one(pool)
                .await
                .unwrap();
        let old: GameResult = serde_json::from_value(row.0.expect("ended")).unwrap();
        assert_eq!(old.reason, EndReason::ServerRestart);
        assert_eq!(old.vp_halves, vec![0; 4]);
        assert_eq!(old.failure.as_deref(), Some("the record does not read"));
        sqlx::query("delete from games where id like $1")
            .bind(format!("{run}-%"))
            .execute(pool)
            .await
            .unwrap();
    }

    fn random() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}
