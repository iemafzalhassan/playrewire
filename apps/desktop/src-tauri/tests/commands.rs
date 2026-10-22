//! The desktop commands, called directly with a temp data dir: no Tauri
//! runtime, no real keychain (an in-memory `KeyStore` stands in).

use std::path::Path;

use rewire_core::{ProgressState, StepRecord};
use rewire_desktop::commands::{self, Settings};
use rewire_desktop::state::AppState;
use rewire_lab::{LabError, SourceFile, ToolchainError};
use rewire_mentor::{AgUiEvent, HintRequest, KeyStore, MSG_FALLBACK, MemoryStore, ProviderConfig};

fn open(dir: &Path) -> AppState {
    // `None`: ignore any REWIRE_GO_ROOT in the test environment.
    AppState::open(dir, Box::new(MemoryStore::default()), None).expect("open app state")
}

fn hello() -> SourceFile {
    SourceFile {
        name: "main.go".into(),
        source: "package main\n\nimport \"fmt\"\n\nfunc main() { fmt.Println(\"hi\") }\n".into(),
    }
}

fn rec() -> StepRecord {
    // Built from the same JSON the UI sends.
    serde_json::from_value(serde_json::json!({
        "chunk_id": "hello-print", "concept": "print", "step": "see", "mission_id": null,
        "success": true, "first_try": true, "hints_used": 0, "local_date": "2026-10-22"
    }))
    .unwrap()
}

fn mentor_cfg() -> ProviderConfig {
    ProviderConfig {
        preset_id: "openai".into(),
        base_url: "https://api.openai.com/v1".into(),
        model: "gpt-5-mini".into(),
    }
}

#[tokio::test]
async fn run_code_without_toolchain_returns_toolchain_missing() {
    let dir = tempfile::tempdir().unwrap();
    let st = open(dir.path());
    assert_eq!(commands::toolchain_status(&st).status, "missing");
    let err = commands::run_code(&st, vec![hello()]).await.unwrap_err();
    assert_eq!(err, "toolchain_missing");
}

#[test]
fn data_dir_layout() {
    let dir = tempfile::tempdir().unwrap();
    let st = open(dir.path());
    assert!(dir.path().join("rewire.db").is_file());
    assert_eq!(st.cache_dir(), dir.path().join("gocache"));
}

#[test]
fn save_then_load_progress() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = ProgressState { xp: 30, ..Default::default() };
    state.completed.insert("hello-print:see".into());
    {
        let st = open(dir.path());
        assert_eq!(commands::load_progress(&st).unwrap(), ProgressState::default());
        commands::save_progress(&st, state.clone(), rec()).unwrap();
    }
    // A fresh AppState on the same dir is an app restart.
    let st = open(dir.path());
    assert_eq!(commands::load_progress(&st).unwrap(), state);
}

#[test]
fn settings_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    {
        let st = open(dir.path());
        let s = commands::get_settings(&st).unwrap();
        assert_eq!(s, Settings::default());
        // Same JSON shape as the TS `Settings` (camelCase, 'system' literal).
        assert_eq!(
            serde_json::to_value(&s).unwrap(),
            serde_json::json!({"reducedMotion": "system", "dyslexiaFont": false, "textScale": 1.0, "mentor": null})
        );
        let new: Settings = serde_json::from_value(serde_json::json!({
            "reducedMotion": true, "dyslexiaFont": true, "textScale": 1.25,
            "mentor": {"preset_id": "openai", "base_url": "https://api.openai.com/v1", "model": "gpt-5-mini"}
        }))
        .unwrap();
        commands::save_settings(&st, new).unwrap();
    }
    let st = open(dir.path());
    let back = commands::get_settings(&st).unwrap();
    assert!(back.dyslexia_font);
    assert_eq!(back.text_scale, 1.25);
    assert_eq!(back.mentor, Some(mentor_cfg()));
    assert_eq!(serde_json::to_value(&back).unwrap()["reducedMotion"], serde_json::json!(true));
}

#[test]
fn bad_settings_json_falls_back_to_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let conn = {
        let _st = open(dir.path());
        rusqlite::Connection::open(dir.path().join("rewire.db")).unwrap()
    };
    conn.execute("INSERT OR REPLACE INTO settings(key,value) VALUES('settings.v1','{not json')", [])
        .unwrap();
    let st = open(dir.path());
    assert_eq!(commands::get_settings(&st).unwrap(), Settings::default());
}

/// Every text value in every table, plus the raw bytes of the db and its WAL.
fn db_contains(dir: &Path, needle: &str) -> bool {
    let conn = rusqlite::Connection::open(dir.join("rewire.db")).unwrap();
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(tables.iter().any(|t| t == "settings") && tables.iter().any(|t| t == "kv"));
    for t in &tables {
        let mut stmt = conn.prepare(&format!("SELECT * FROM \"{t}\"")).unwrap();
        let cols = stmt.column_count();
        let mut rows = stmt.query([]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            for i in 0..cols {
                let v: rusqlite::types::Value = row.get(i).unwrap();
                let text = match v {
                    rusqlite::types::Value::Text(s) => s,
                    rusqlite::types::Value::Blob(b) => String::from_utf8_lossy(&b).into_owned(),
                    _ => continue,
                };
                if text.contains(needle) {
                    return true;
                }
            }
        }
    }
    drop(conn);
    for f in ["rewire.db", "rewire.db-wal"] {
        if let Ok(bytes) = std::fs::read(dir.join(f))
            && bytes.windows(needle.len()).any(|w| w == needle.as_bytes())
        {
            return true;
        }
    }
    false
}

#[test]
fn api_key_never_written_to_db() {
    const KEY: &str = "sk-test-NEVER-IN-SQLITE-4f1c";
    let dir = tempfile::tempdir().unwrap();
    let keys = std::sync::Arc::new(MemoryStore::default());
    let st = AppState::open(dir.path(), Box::new(SharedKeys(keys.clone())), None).unwrap();

    commands::set_api_key(&st, "openai".into(), KEY.into()).unwrap();
    let s = Settings { mentor: Some(mentor_cfg()), ..Default::default() };
    commands::save_settings(&st, s).unwrap();
    commands::save_progress(&st, ProgressState::default(), rec()).unwrap();

    assert_eq!(keys.get("openai").unwrap().as_deref(), Some(KEY), "key went to the keystore");
    assert!(!db_contains(dir.path(), KEY), "the key must never reach SQLite");
}

#[test]
fn set_api_key_rejects_unknown_preset_and_empty_key() {
    let dir = tempfile::tempdir().unwrap();
    let st = open(dir.path());
    assert_eq!(commands::set_api_key(&st, "../etc".into(), "k".into()).unwrap_err(), "invalid_input");
    assert_eq!(commands::set_api_key(&st, "openai".into(), "  ".into()).unwrap_err(), "invalid_input");
}

#[test]
fn get_presets_lists_the_mentor_presets() {
    let presets = commands::get_presets();
    assert!(presets.iter().any(|p| p.id == "ollama" && p.local && !p.needs_key));
    assert!(presets.iter().any(|p| p.id == "openai" && p.needs_key));
    let json = serde_json::to_value(presets).unwrap();
    assert!(json[0].get("base_url").is_some() && json[0].get("default_model").is_some());
}

#[tokio::test]
async fn mentor_hint_without_a_mentor_sends_the_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let st = open(dir.path());
    let req = HintRequest {
        concept: "print".into(),
        exercise_prompt: "say hi".into(),
        code: "".into(),
        last_error: None,
        attempt: 1,
    };
    let mut got = Vec::new();
    commands::mentor_hint(&st, req, &mut |e| got.push(e)).await.unwrap();
    assert_eq!(got, vec![AgUiEvent::RunError { message: MSG_FALLBACK.into() }]);
}

#[test]
fn keyed_presets_always_use_their_own_base_url() {
    // A key saved for a preset only ever goes to that preset's server, even
    // if the settings JSON carries another base_url.
    let mut cfg = mentor_cfg();
    cfg.base_url = "https://attacker.example/v1".into();
    assert_eq!(commands::effective_config(cfg).unwrap().base_url, "https://api.openai.com/v1");
    let custom = ProviderConfig {
        preset_id: "custom".into(),
        base_url: "http://my-box:8080/v1".into(),
        model: "m".into(),
    };
    assert_eq!(commands::effective_config(custom).unwrap().base_url, "http://my-box:8080/v1");
    let unknown = ProviderConfig { preset_id: "nope".into(), base_url: String::new(), model: String::new() };
    assert!(commands::effective_config(unknown).is_none());
}

#[test]
fn error_codes_are_stable() {
    assert_eq!(commands::lab_error_code(&LabError::ToolchainMissing), "toolchain_missing");
    assert_eq!(
        commands::lab_error_code(&LabError::SandboxUnavailable("x".into())),
        "sandbox_unavailable"
    );
    assert_eq!(commands::lab_error_code(&LabError::Io(std::io::Error::other("x"))), "io");
    assert_eq!(commands::toolchain_error_code(&ToolchainError::Offline), "offline");
    assert_eq!(commands::toolchain_error_code(&ToolchainError::ChecksumMismatch), "checksum_mismatch");
    assert_eq!(commands::toolchain_error_code(&ToolchainError::Io(std::io::Error::other("x"))), "io");
}

/// A `KeyStore` the test can still read after handing it to `AppState`.
struct SharedKeys(std::sync::Arc<MemoryStore>);

impl KeyStore for SharedKeys {
    fn set(&self, account: &str, key: &str) -> Result<(), rewire_mentor::MentorError> {
        self.0.set(account, key)
    }
    fn get(&self, account: &str) -> Result<Option<String>, rewire_mentor::MentorError> {
        self.0.get(account)
    }
}
