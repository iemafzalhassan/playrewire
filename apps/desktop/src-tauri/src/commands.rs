//! The desktop commands as plain functions over `&AppState`. `lib.rs` wraps
//! each one in a thin `#[tauri::command]`; the tests call these directly.
//!
//! Errors cross to the UI as a short, stable code (see [`code`]) that the UI
//! maps to its own friendly copy. Technical detail stays in the log, and
//! mentor errors never leave Rust at all: the player only ever sees the two
//! fixed RUN_ERROR messages from rewire-mentor.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rewire_core::{ProgressState, StepRecord};
use rewire_lab::{LabError, LanguageRuntime, Limits, RawRunResult, SourceFile, ToolchainError, ToolchainStatus};
use rewire_mentor::{AgUiEvent, HintRequest, MSG_FALLBACK, Preset, ProviderConfig};
use rewire_store::StoreError;
use serde::{Deserialize, Serialize};

use crate::state::AppState;

/// Where the pinned Go archive is downloaded from. rewire-lab only follows
/// https redirects to go.dev and dl.google.com, and checks the pinned SHA-256.
pub const GO_DOWNLOAD_BASE: &str = "https://go.dev/dl";

/// Settings JSON lives in the store's `settings` table under this key.
pub const SETTINGS_KEY: &str = "settings.v1";

/// The only error strings a command ever returns.
pub mod code {
    /// No Go toolchain yet: the UI offers the one-time download.
    pub const TOOLCHAIN_MISSING: &str = "toolchain_missing";
    /// bubblewrap can't start here (Linux sandbox setup docs).
    pub const SANDBOX_UNAVAILABLE: &str = "sandbox_unavailable";
    /// The Go download server could not be reached.
    pub const OFFLINE: &str = "offline";
    /// The downloaded archive didn't match the pinned SHA-256.
    pub const CHECKSUM_MISMATCH: &str = "checksum_mismatch";
    /// Anything else on disk, in the database, or unexpected.
    pub const IO: &str = "io";
    /// The UI sent something the command refuses (unknown preset, empty key).
    pub const INVALID_INPUT: &str = "invalid_input";
    /// The OS keychain refused to store the key.
    pub const KEYCHAIN: &str = "keychain_unavailable";
}

pub fn lab_error_code(e: &LabError) -> &'static str {
    match e {
        LabError::ToolchainMissing => code::TOOLCHAIN_MISSING,
        LabError::SandboxUnavailable(_) => code::SANDBOX_UNAVAILABLE,
        LabError::Io(_) => code::IO,
    }
}

pub fn toolchain_error_code(e: &ToolchainError) -> &'static str {
    match e {
        ToolchainError::Offline => code::OFFLINE,
        ToolchainError::ChecksumMismatch => code::CHECKSUM_MISMATCH,
        // Non-Linux hosts and unexpected archives: nothing the player can fix.
        ToolchainError::Unsupported(_) | ToolchainError::Io(_) => code::IO,
    }
}

fn store_err(what: &str, e: StoreError) -> String {
    eprintln!("rewire: {what}: {e}");
    code::IO.to_string()
}

// ---------------------------------------------------------------- lab

pub async fn run_code(st: &AppState, files: Vec<SourceFile>) -> Result<RawRunResult, String> {
    let rt = st.runtime().ok_or_else(|| code::TOOLCHAIN_MISSING.to_string())?;
    rt.run(files, Limits::default()).await.map_err(|e| {
        eprintln!("rewire: run failed: {e}");
        lab_error_code(&e).to_string()
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToolchainReply {
    /// `"ready"` or `"missing"`.
    pub status: &'static str,
}

pub fn toolchain_status(st: &AppState) -> ToolchainReply {
    let status = match st.toolchain() {
        ToolchainStatus::Ready { .. } => "ready",
        ToolchainStatus::Missing => "missing",
    };
    ToolchainReply { status }
}

/// Downloads and installs the pinned Go toolchain. `on_progress(done, total)`
/// fires at most about once per percent, plus once at the end.
pub async fn install_toolchain(st: &AppState, on_progress: impl Fn(u64, u64) + Send + Sync) -> Result<(), String> {
    let last_pct = AtomicU64::new(u64::MAX);
    let report = |done: u64, total: u64| {
        let pct = done.saturating_mul(100).checked_div(total).unwrap_or(100);
        if last_pct.swap(pct, Ordering::Relaxed) != pct || done >= total {
            on_progress(done, total);
        }
    };
    rewire_lab::install(st.data_dir(), GO_DOWNLOAD_BASE, report)
        .await
        .map_err(|e| {
            eprintln!("rewire: Go install failed: {e}");
            toolchain_error_code(&e).to_string()
        })?;
    Ok(())
}

/// Builds `fmt` once so the learner's first run is quick. Best effort: a
/// failure here only means the first run compiles a bit longer.
pub async fn warm(st: &AppState) {
    let Some(rt) = st.runtime() else { return };
    if let Err(e) = rewire_lab::warm_cache(&rt).await {
        eprintln!("rewire: warming the Go cache failed: {e}");
    }
}

// ---------------------------------------------------------------- progress

pub fn load_progress(st: &AppState) -> Result<ProgressState, String> {
    st.store().load_progress().map_err(|e| store_err("load progress", e))
}

pub fn save_progress(st: &AppState, state: ProgressState, rec: StepRecord) -> Result<(), String> {
    st.store().save(&state, &rec).map_err(|e| store_err("save progress", e))
}

// ---------------------------------------------------------------- settings

/// `reducedMotion: boolean | "system"` from the TS `Settings`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ReducedMotion {
    Set(bool),
    Follow(FollowSystem),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FollowSystem {
    #[serde(rename = "system")]
    System,
}

/// Mirrors the TS `Settings`. The API key is never part of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub reduced_motion: ReducedMotion,
    pub dyslexia_font: bool,
    /// 1, 1.25 or 1.5; anything else is read back as 1.
    pub text_scale: f64,
    pub mentor: Option<ProviderConfig>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            reduced_motion: ReducedMotion::Follow(FollowSystem::System),
            dyslexia_font: false,
            text_scale: 1.0,
            mentor: None,
        }
    }
}

const TEXT_SCALES: [f64; 3] = [1.0, 1.25, 1.5];

fn normalized(mut s: Settings) -> Settings {
    if !TEXT_SCALES.contains(&s.text_scale) {
        s.text_scale = 1.0;
    }
    s
}

/// Defaults when nothing is saved or the saved JSON doesn't parse.
pub fn get_settings(st: &AppState) -> Result<Settings, String> {
    let raw = st.store().get_setting(SETTINGS_KEY).map_err(|e| store_err("load settings", e))?;
    let parsed = raw.and_then(|r| serde_json::from_str::<Settings>(&r).ok());
    Ok(normalized(parsed.unwrap_or_default()))
}

pub fn save_settings(st: &AppState, settings: Settings) -> Result<(), String> {
    let json = serde_json::to_string(&normalized(settings)).map_err(|e| {
        eprintln!("rewire: encode settings: {e}");
        code::IO.to_string()
    })?;
    st.store()
        .set_setting(SETTINGS_KEY, &json)
        .map_err(|e| store_err("save settings", e))
}

// ---------------------------------------------------------------- mentor

pub fn get_presets() -> &'static [Preset] {
    rewire_mentor::presets()
}

fn preset(id: &str) -> Option<&'static Preset> {
    rewire_mentor::presets().iter().find(|p| p.id == id)
}

/// Presets that may carry a key: the hosted ones, and "custom" (any
/// OpenAI-compatible server, which may or may not want one).
fn takes_key(p: &Preset) -> bool {
    p.needs_key || p.id == "custom"
}

/// Longest key we accept; real provider keys are far shorter.
const MAX_KEY_LEN: usize = 4096;

/// Stores the key in the OS keychain (account = preset id). It never goes
/// into SQLite, the settings JSON, an error, or the log.
pub fn set_api_key(st: &AppState, preset_id: String, key: String) -> Result<(), String> {
    let key = key.trim();
    let ok = preset(&preset_id).is_some_and(takes_key) && !key.is_empty() && key.len() <= MAX_KEY_LEN;
    if !ok {
        return Err(code::INVALID_INPUT.to_string());
    }
    st.keys().set(&preset_id, key).map_err(|_| {
        // MentorError::Keychain never holds the key, but there's nothing in it
        // the player needs either.
        eprintln!("rewire: the keychain refused to store the mentor key");
        code::KEYCHAIN.to_string()
    })
}

/// The config a hint actually uses. A hosted preset always talks to its own
/// server, whatever `base_url` the settings carry, so a key saved for that
/// preset can't be sent anywhere else. `None` for an unknown preset.
pub fn effective_config(mut cfg: ProviderConfig) -> Option<ProviderConfig> {
    let p = preset(&cfg.preset_id)?;
    if p.needs_key {
        cfg.base_url = p.base_url.to_string();
    }
    Some(cfg)
}

/// Streams a hint as AG-UI events. Every failure (no mentor set, keychain,
/// network, provider) reaches the UI only as one of rewire-mentor's two fixed
/// RUN_ERROR messages; the command itself always succeeds.
/// How long the keychain read may take before a hint falls back. A locked
/// Secret Service can block on an unlock prompt; the read runs on a blocking
/// thread so it can't hold up the async runtime, and is abandoned after this.
pub const KEYCHAIN_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn mentor_hint(
    st: &AppState,
    req: HintRequest,
    sink: &mut (dyn FnMut(AgUiEvent) + Send),
) -> Result<(), String> {
    mentor_hint_with_key_timeout(st, req, sink, KEYCHAIN_TIMEOUT).await
}

/// [`mentor_hint`] with the keychain timeout as a parameter, for tests.
pub async fn mentor_hint_with_key_timeout(
    st: &AppState,
    req: HintRequest,
    sink: &mut (dyn FnMut(AgUiEvent) + Send),
    key_timeout: Duration,
) -> Result<(), String> {
    let fallback = |sink: &mut (dyn FnMut(AgUiEvent) + Send)| {
        sink(AgUiEvent::RunError { message: MSG_FALLBACK.to_string() });
    };
    let cfg = get_settings(st).ok().and_then(|s| s.mentor).and_then(effective_config);
    let Some(cfg) = cfg else {
        fallback(sink);
        return Ok(());
    };
    let key = match preset(&cfg.preset_id) {
        Some(p) if takes_key(p) => {
            let keys = st.keys_shared();
            let account = cfg.preset_id.clone();
            let read = tokio::task::spawn_blocking(move || keys.get(&account));
            match tokio::time::timeout(key_timeout, read).await {
                Ok(Ok(Ok(k))) => k,
                Ok(Ok(Err(_))) | Ok(Err(_)) => {
                    eprintln!("rewire: could not read the mentor key from the keychain");
                    fallback(sink);
                    return Ok(());
                }
                Err(_) => {
                    // The blocking thread finishes (or stays parked) on its
                    // own; nothing waits for it.
                    eprintln!("rewire: the keychain took too long to answer");
                    fallback(sink);
                    return Ok(());
                }
            }
        }
        _ => None,
    };
    // On failure `hint` has already sent the right RUN_ERROR; its error value
    // is deliberately dropped here.
    let _ = rewire_mentor::hint(&cfg, key, &req, sink).await;
    Ok(())
}
