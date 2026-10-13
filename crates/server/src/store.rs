//! Where records go: entries appended as the game goes, so a crash keeps
//! the game. Postgres in production (`games`, `game_entries`), a
//! directory of JSON lines in development.

use std::io::Write;
use std::path::PathBuf;

use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedReceiver;
use zvtes_api::record::{Entry, GameRecord};

/// What a game sends its store.
pub enum Put {
    /// The game begins: its record without entries.
    Start { game: String, record: GameRecord },
    /// Entries from position `from` (positions count from 0).
    Entries {
        game: String,
        from: usize,
        entries: Vec<Entry>,
    },
    /// The game ended: its record whole (result, end time).
    End { game: String, record: GameRecord },
}

pub enum Store {
    Postgres(PgPool),
    Files(PathBuf),
}

const SCHEMA: &str = "
create table if not exists games (
    id text primary key,
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
                    Put::Start { game, record } => (game, json(&("start", record))),
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
                Put::Start { game, record } => sqlx::query(
                    "insert into games (id, protocol, engine, client, spec, bots, started)
                     values ($1, $2, $3, $4, $5, $6, to_timestamp($7::double precision / 1000))",
                )
                .bind(&game)
                .bind(&record.protocol)
                .bind(&record.engine)
                .bind(&record.spec.client)
                .bind(value(&record.spec))
                .bind(value(&record.bots))
                .bind(record.started as f64)
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
                Put::End { game, record } => sqlx::query(
                    "update games set ended = to_timestamp($2::double precision / 1000),
                     result = $3 where id = $1",
                )
                .bind(&game)
                .bind(record.ended.unwrap_or(record.started) as f64)
                .bind(value(&record.result))
                .execute(pool)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string()),
            },
        }
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

fn json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).expect("records serialize")
}

fn value<T: serde::Serialize>(v: &T) -> serde_json::Value {
    serde_json::to_value(v).expect("records serialize")
}
