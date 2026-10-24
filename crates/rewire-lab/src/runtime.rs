//! The language-runtime interface shared by every lab runner.

use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// One learner source file. `name` must be a plain `[A-Za-z0-9_]+.go`
/// basename; the runner rejects anything else before touching the disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFile {
    pub name: String,
    pub source: String,
}

/// Resource limits for one run (spec §7.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    /// Wall-clock limit for the compile phase.
    pub compile_wall: Duration,
    /// Wall-clock limit for the learner's program.
    pub run_wall: Duration,
    /// `RLIMIT_CPU` for the learner's program, in seconds.
    pub cpu_secs: u64,
    /// `RLIMIT_FSIZE` for the learner's program, in bytes.
    pub max_file_bytes: u64,
    /// Cap on combined stdout + stderr bytes.
    pub max_output: usize,
    /// `RLIMIT_AS` for the learner's program, in bytes. The compile phase is
    /// not limited (only the trusted toolchain runs there).
    pub max_memory_bytes: u64,
    /// Allow network access. Always `false` in the MVP.
    pub network: bool,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            compile_wall: Duration::from_secs(60),
            run_wall: Duration::from_secs(10),
            cpu_secs: 10,
            max_file_bytes: 10 * 1024 * 1024,
            max_output: 65_536,
            max_memory_bytes: 1024 * 1024 * 1024,
            network: false,
        }
    }
}

/// What a run produced. Serializes (camelCase) to exactly the TS
/// `RawRunResult` in `apps/web/src/lab/protocol.ts`.
///
/// `duration_ms` covers the run phase only (spawn to reap of the learner's
/// program). It is `0` when the program never ran, e.g. on a compile error.
/// `exit_code` is `None` when the program was killed (wall or CPU limit, or
/// the output cap).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawRunResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub truncated: bool,
    pub duration_ms: u64,
}

#[derive(Debug)]
pub enum LabError {
    /// The sandbox can't start on this host; the message is user-facing.
    SandboxUnavailable(String),
    /// `go_root` does not contain a Go toolchain.
    ToolchainMissing,
    /// I/O failure, or `InvalidInput` for a rejected source file name.
    Io(std::io::Error),
}

impl fmt::Display for LabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LabError::SandboxUnavailable(msg) => f.write_str(msg),
            LabError::ToolchainMissing => f.write_str("the Go toolchain is missing"),
            LabError::Io(e) => write!(f, "lab I/O error: {e}"),
        }
    }
}

impl std::error::Error for LabError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LabError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for LabError {
    fn from(e: std::io::Error) -> Self {
        LabError::Io(e)
    }
}

#[async_trait::async_trait]
pub trait LanguageRuntime: Send + Sync {
    fn id(&self) -> &'static str;
    async fn run(&self, files: Vec<SourceFile>, limits: Limits) -> Result<RawRunResult, LabError>;
}
