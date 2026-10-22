//! App state shared by every command: the SQLite store, the data dir, the
//! keychain, and the Go runtime (built lazily once a toolchain is ready).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use rewire_lab::{GoNativeRuntime, ToolchainStatus};
use rewire_mentor::KeyStore;
use rewire_store::{Store, StoreError};

/// File name of the SQLite database inside the data dir.
pub const DB_FILE: &str = "rewire.db";
/// Go build cache and GOPATH for the sandbox, inside the data dir.
pub const CACHE_DIR: &str = "gocache";

/// Why the app state couldn't be opened (startup only; never sent to the UI).
#[derive(Debug)]
pub enum OpenError {
    DataDir(std::io::Error),
    Store(StoreError),
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenError::DataDir(e) => write!(f, "could not create the data dir: {e}"),
            OpenError::Store(e) => write!(f, "could not open the database: {e}"),
        }
    }
}

impl std::error::Error for OpenError {}

/// Data dir layout: `rewire.db`, `toolchains/` (managed by rewire-lab) and
/// `gocache/`. In the app the dir is Tauri's `app_data_dir`
/// (`$XDG_DATA_HOME/dev.playrewire.rewire/`).
pub struct AppState {
    data_dir: PathBuf,
    store: Mutex<Store>,
    keys: Box<dyn KeyStore + Send + Sync>,
    /// `REWIRE_GO_ROOT`, read once at startup. Tests pass `None`.
    go_override: Option<PathBuf>,
    runtime: Mutex<Option<Arc<GoNativeRuntime>>>,
}

impl AppState {
    /// Creates the data dir if needed and opens (or migrates) the database.
    pub fn open(
        data_dir: &Path,
        keys: Box<dyn KeyStore + Send + Sync>,
        go_override: Option<PathBuf>,
    ) -> Result<AppState, OpenError> {
        std::fs::create_dir_all(data_dir).map_err(OpenError::DataDir)?;
        let store = Store::open(&data_dir.join(DB_FILE)).map_err(OpenError::Store)?;
        Ok(AppState {
            data_dir: data_dir.to_path_buf(),
            store: Mutex::new(store),
            keys,
            go_override,
            runtime: Mutex::new(None),
        })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.data_dir.join(CACHE_DIR)
    }

    pub fn keys(&self) -> &(dyn KeyStore + Send + Sync) {
        self.keys.as_ref()
    }

    /// The store, even if an earlier command panicked while holding it (the
    /// connection itself is still fine: every write is one statement or one
    /// transaction).
    pub fn store(&self) -> MutexGuard<'_, Store> {
        self.store.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn toolchain(&self) -> ToolchainStatus {
        rewire_lab::status_with_env(&self.data_dir, self.go_override.as_deref())
    }

    /// The Go runtime, or `None` while no toolchain is installed. Built on
    /// first use and kept; a toolchain installed later is picked up on the
    /// next call.
    pub fn runtime(&self) -> Option<Arc<GoNativeRuntime>> {
        let mut slot = self.runtime.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(rt) = slot.as_ref() {
            return Some(rt.clone());
        }
        let ToolchainStatus::Ready { go_root } = self.toolchain() else {
            return None;
        };
        let rt = Arc::new(GoNativeRuntime {
            go_root,
            cache_dir: self.cache_dir(),
        });
        *slot = Some(rt.clone());
        Some(rt)
    }
}
