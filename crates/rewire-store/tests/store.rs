use chrono::NaiveDate;
use rewire_core::{ProgressState, Step, StepRecord, apply};
use rewire_store::Store;
use rusqlite::Connection;

fn rec(chunk: &str) -> StepRecord {
    StepRecord {
        chunk_id: chunk.into(),
        concept: "loops".into(),
        step: Step::Drill,
        mission_id: None,
        success: true,
        first_try: true,
        hints_used: 0,
        local_date: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
    }
}

#[test]
fn fresh_db_returns_default_progress() {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::open(&dir.path().join("a.db")).unwrap();
    assert_eq!(s.load_progress().unwrap(), ProgressState::default());
    assert!(s.outbox_unsynced(10).unwrap().is_empty());
}

#[test]
fn save_writes_state_and_outbox_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.db");
    let mut s = Store::open(&path).unwrap();
    let r = rec("c1");
    let (next, _) = apply(&ProgressState::default(), &r);
    s.save(&next, &r).unwrap();
    assert_eq!(s.load_progress().unwrap(), next);
    let rows = s.outbox_unsynced(10).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, "step");
    assert!(!rows[0].synced);
    assert_eq!(serde_json::from_str::<StepRecord>(&rows[0].payload).unwrap(), r);
    assert!(chrono::DateTime::parse_from_rfc3339(&rows[0].created_at).is_ok());
    assert!(rows[0].created_at.ends_with("+00:00") || rows[0].created_at.ends_with('Z'));

    // Atomic: a failure inside the transaction leaves neither write behind.
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("DROP TABLE outbox").unwrap();
    drop(conn);
    let (n2, _) = apply(&next, &rec("c2"));
    assert!(s.save(&n2, &rec("c2")).is_err());
    assert_eq!(s.load_progress().unwrap(), next, "kv write must roll back");
}

#[test]
fn outbox_limit_and_order() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("a.db")).unwrap();
    let mut st = ProgressState::default();
    for i in 0..3 {
        let r = rec(&format!("c{i}"));
        st = apply(&st, &r).0;
        s.save(&st, &r).unwrap();
    }
    let rows = s.outbox_unsynced(2).unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows[0].id < rows[1].id);
}

#[test]
fn reopen_keeps_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.db");
    let r = rec("c1");
    let (next, _) = apply(&ProgressState::default(), &r);
    {
        let mut s = Store::open(&path).unwrap();
        s.save(&next, &r).unwrap();
        s.set_setting("theme", "dark").unwrap();
    }
    let s = Store::open(&path).unwrap();
    assert_eq!(s.load_progress().unwrap(), next);
    assert_eq!(s.outbox_unsynced(10).unwrap().len(), 1);
    assert_eq!(s.get_setting("theme").unwrap().as_deref(), Some("dark"));
    assert_eq!(s.get_setting("nope").unwrap(), None);
}

#[test]
fn migrations_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.db");
    for _ in 0..3 {
        Store::open(&path).unwrap();
    }
    let conn = Connection::open(&path).unwrap();
    let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(v, 1);
    let mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
    assert_eq!(mode, "wal");
    for t in ["kv", "outbox", "settings"] {
        let n: i64 = conn
            .query_row("SELECT count(*) FROM sqlite_master WHERE name=?1", [t], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1, "{t}");
    }
}

#[test]
fn newer_schema_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.db");
    Store::open(&path).unwrap();
    Connection::open(&path).unwrap().execute_batch("PRAGMA user_version = 99").unwrap();
    assert!(Store::open(&path).is_err());
}

#[test]
fn corrupt_kv_json_falls_back_to_default_and_keeps_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.db");
    drop(Store::open(&path).unwrap());
    let conn = Connection::open(&path).unwrap();
    conn.execute("INSERT INTO kv(key,value) VALUES('progress.v1','{not json')", []).unwrap();
    drop(conn);

    let mut s = Store::open(&path).unwrap();
    assert_eq!(s.load_progress().unwrap(), ProgressState::default());
    // Loading twice must not pile up extra backups.
    assert_eq!(s.load_progress().unwrap(), ProgressState::default());

    let conn = Connection::open(&path).unwrap();
    let backups: Vec<(String, String)> = conn
        .prepare("SELECT key,value FROM kv WHERE key LIKE 'progress.corrupt.%'").unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap()
        .map(|r| r.unwrap()).collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(backups[0].1, "{not json");
    assert!(backups[0].0["progress.corrupt.".len()..].parse::<u64>().is_ok());
    // The corrupt key is not overwritten until the next successful save.
    let raw: String = conn.query_row("SELECT value FROM kv WHERE key='progress.v1'", [], |r| r.get(0)).unwrap();
    assert_eq!(raw, "{not json");
    drop(conn);

    let r = rec("c1");
    let (next, _) = apply(&ProgressState::default(), &r);
    s.save(&next, &r).unwrap();
    assert_eq!(s.load_progress().unwrap(), next);
}
