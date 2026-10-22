//! SQLite persistence for PlayRewire on desktop: progress in `kv`, an outbox of
//! step events for a later sync phase (nothing leaves the device in the MVP),
//! and plain settings. The store may read the host clock; `rewire-core` may not.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::DateTime;
use rewire_core::{ProgressState, StepRecord};
use rusqlite::{Connection, OptionalExtension, params};

const PROGRESS_KEY: &str = "progress.v1";
const CORRUPT_PREFIX: &str = "progress.corrupt.";

/// Each entry upgrades the schema by one `PRAGMA user_version`.
const MIGRATIONS: &[&str] = &[
    "CREATE TABLE kv(key TEXT PRIMARY KEY, value TEXT NOT NULL);
     CREATE TABLE outbox(
         id INTEGER PRIMARY KEY,
         kind TEXT NOT NULL,
         payload TEXT NOT NULL,
         created_at TEXT NOT NULL,
         synced INTEGER NOT NULL DEFAULT 0);
     CREATE TABLE settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);",
];

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("could not encode data: {0}")]
    Encode(#[from] serde_json::Error),
    #[error("database schema version {found} is newer than this app supports ({supported})")]
    SchemaTooNew { found: i64, supported: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxRow {
    pub id: i64,
    pub kind: String,
    pub payload: String,
    pub created_at: String,
    pub synced: bool,
}

pub struct Store {
    conn: Connection,
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl Store {
    pub fn open(path: &Path) -> Result<Store, StoreError> {
        let mut conn = Connection::open(path)?;
        // `journal_mode` returns a row, so read it instead of executing.
        conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
        migrate(&mut conn)?;
        Ok(Store { conn })
    }

    /// Default state when nothing is saved. A value that does not parse is
    /// copied to `progress.corrupt.<unix_ts>` (once) and the default is returned;
    /// the bad key stays until the next successful `save`.
    pub fn load_progress(&self) -> Result<ProgressState, StoreError> {
        let raw = self.get_kv(PROGRESS_KEY)?;
        let Some(raw) = raw else {
            return Ok(ProgressState::default());
        };
        match serde_json::from_str::<ProgressState>(&raw) {
            Ok(s) => Ok(s),
            Err(_) => {
                let already: bool = self.conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM kv WHERE key LIKE ?1 AND value = ?2)",
                    params![format!("{CORRUPT_PREFIX}%"), raw],
                    |r| r.get(0),
                )?;
                if !already {
                    let key = format!("{CORRUPT_PREFIX}{}", now_unix());
                    self.conn.execute(
                        "INSERT OR REPLACE INTO kv(key,value) VALUES(?1,?2)",
                        params![key, raw],
                    )?;
                }
                Ok(ProgressState::default())
            }
        }
    }

    /// Writes the state and appends the step to the outbox in one transaction.
    pub fn save(&mut self, state: &ProgressState, record: &StepRecord) -> Result<(), StoreError> {
        let state_json = serde_json::to_string(state)?;
        let payload = serde_json::to_string(record)?;
        let created_at = DateTime::from_timestamp(now_unix(), 0)
            .map(|d| d.to_rfc3339())
            .unwrap_or_default();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT OR REPLACE INTO kv(key,value) VALUES(?1,?2)",
            params![PROGRESS_KEY, state_json],
        )?;
        tx.execute(
            "INSERT INTO outbox(kind,payload,created_at) VALUES('step',?1,?2)",
            params![payload, created_at],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn outbox_unsynced(&self, limit: usize) -> Result<Vec<OutboxRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id,kind,payload,created_at,synced FROM outbox WHERE synced = 0 ORDER BY id LIMIT ?1",
        )?;
        let rows = stmt
            .query_map([limit as i64], |r| {
                Ok(OutboxRow {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                    payload: r.get(2)?,
                    created_at: r.get(3)?,
                    synced: r.get::<_, i64>(4)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT value FROM settings WHERE key=?1", [key], |r| r.get(0))
            .optional()?)
    }

    pub fn set_setting(&mut self, key: &str, value: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings(key,value) VALUES(?1,?2)",
            params![key, value],
        )?;
        Ok(())
    }

    fn get_kv(&self, key: &str) -> Result<Option<String>, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT value FROM kv WHERE key=?1", [key], |r| r.get(0))
            .optional()?)
    }
}

fn migrate(conn: &mut Connection) -> Result<(), StoreError> {
    let supported = MIGRATIONS.len() as i64;
    let found: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if found > supported {
        return Err(StoreError::SchemaTooNew { found, supported });
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(found as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.execute_batch(&format!("PRAGMA user_version = {}", i + 1))?;
        tx.commit()?;
    }
    Ok(())
}
