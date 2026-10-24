//! Rewire desktop: the Tauri v2 shell around the shared Svelte UI.
//!
//! Each `#[tauri::command]` below is a thin wrapper over a plain function in
//! [`commands`]; the frontend contract lives in `apps/web/src/backend/tauri.ts`.

pub mod commands;
pub mod smoke;
pub mod state;

use std::path::PathBuf;

use rewire_core::{ProgressState, StepRecord};
use rewire_lab::{RawRunResult, SourceFile};
use rewire_mentor::{AgUiEvent, HintRequest, KeyringStore, Preset};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use crate::commands::{Settings, ToolchainReply};
use crate::state::AppState;

/// Install progress as the UI reads it (`{done, total}`, bytes).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct InstallProgress {
    pub done: u64,
    pub total: u64,
}

// Every command is async so none of them runs on the main (UI) thread.

#[tauri::command]
async fn run_code(app_state: State<'_, AppState>, files: Vec<SourceFile>) -> Result<RawRunResult, String> {
    commands::run_code(&app_state, files).await
}

#[tauri::command]
async fn load_progress(app_state: State<'_, AppState>) -> Result<ProgressState, String> {
    commands::load_progress(&app_state)
}

#[tauri::command]
async fn save_progress(app_state: State<'_, AppState>, state: ProgressState, rec: StepRecord) -> Result<(), String> {
    commands::save_progress(&app_state, state, rec)
}

#[tauri::command]
async fn toolchain_status(app_state: State<'_, AppState>) -> Result<ToolchainReply, String> {
    Ok(commands::toolchain_status(&app_state))
}

#[tauri::command]
async fn install_toolchain(
    app: AppHandle,
    app_state: State<'_, AppState>,
    on_progress: Channel<InstallProgress>,
) -> Result<(), String> {
    commands::install_toolchain(&app_state, |done, total| {
        let _ = on_progress.send(InstallProgress { done, total });
    })
    .await?;
    // Warm the build cache in the background; the learner can already run code.
    tauri::async_runtime::spawn(async move {
        let st = app.state::<AppState>();
        commands::warm(&st).await;
    });
    Ok(())
}

#[tauri::command]
async fn mentor_hint(
    app_state: State<'_, AppState>,
    req: HintRequest,
    on_event: Channel<AgUiEvent>,
) -> Result<(), String> {
    let mut sink = |e: AgUiEvent| {
        let _ = on_event.send(e);
    };
    commands::mentor_hint(&app_state, req, &mut sink).await
}

#[tauri::command]
async fn get_settings(app_state: State<'_, AppState>) -> Result<Settings, String> {
    commands::get_settings(&app_state)
}

#[tauri::command]
async fn save_settings(app_state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    commands::save_settings(&app_state, settings)
}

#[tauri::command]
async fn set_api_key(app_state: State<'_, AppState>, preset_id: String, key: String) -> Result<(), String> {
    commands::set_api_key(&app_state, preset_id, key)
}

#[tauri::command]
async fn get_presets() -> Result<Vec<Preset>, String> {
    Ok(commands::get_presets().to_vec())
}

/// Longest a `--smoke` run may take (a cold build of `fmt` included).
const SMOKE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(180);

/// Starts the app. Data lives in Tauri's `app_data_dir`
/// (`$XDG_DATA_HOME/dev.playrewire.rewire/` on Linux).
///
/// With `--smoke` the app boots as usual, runs [`smoke::check`], prints
/// `SMOKE OK` (exit 0) or `SMOKE FAILED: <reason>` (exit 1), and quits.
pub fn run() {
    let smoke_mode = smoke::requested(std::env::args());
    tauri::Builder::default()
        .setup(move |app| {
            let data_dir = app.path().app_data_dir()?;
            let go_override = std::env::var_os("REWIRE_GO_ROOT").map(PathBuf::from);
            let st = AppState::open(&data_dir, Box::new(KeyringStore), go_override)?;
            app.manage(st);
            if smoke_mode {
                start_smoke(app.handle().clone());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            run_code,
            load_progress,
            save_progress,
            toolchain_status,
            install_toolchain,
            mentor_hint,
            get_settings,
            save_settings,
            set_api_key,
            get_presets,
        ])
        .run(tauri::generate_context!())
        .expect("Rewire could not start");
}

/// Runs the smoke check off the UI thread and exits the process with its
/// result. A watchdog thread fails the run if it hangs.
fn start_smoke(handle: AppHandle) {
    std::thread::spawn(|| {
        std::thread::sleep(SMOKE_TIMEOUT);
        finish_smoke(Err(format!("timed out after {} s", SMOKE_TIMEOUT.as_secs())));
    });
    tauri::async_runtime::spawn(async move {
        let st = handle.state::<AppState>();
        finish_smoke(smoke::check(&st).await);
    });
}

fn finish_smoke(res: Result<(), String>) -> ! {
    use std::io::Write;
    let code = match res {
        Ok(()) => {
            println!("{}", smoke::OK_LINE);
            0
        }
        Err(reason) => {
            eprintln!("SMOKE FAILED: {reason}");
            1
        }
    };
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    std::process::exit(code)
}
