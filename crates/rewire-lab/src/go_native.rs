//! `GoNativeRuntime`: compiles and runs learner Go code with the real Go
//! toolchain, inside the bubblewrap sandbox.

use std::collections::HashSet;
use std::io;
use std::path::PathBuf;

use crate::runtime::{LabError, LanguageRuntime, Limits, RawRunResult, SourceFile};
use crate::sandbox::{
    COMPILE_TMP_BYTES, PhaseLimits, PhaseOutcome, RUN_TMP_BYTES, Sandbox, run_phase,
    sandbox_probe_async, verdict,
};

/// The `go.mod` written next to the learner's files.
const GO_MOD: &str = "module lab\n\ngo 1.27\n";

/// Compile-phase limits (controller ruling): RLIMIT_FSIZE is
/// max(max_file_bytes, 512 MiB) and RLIMIT_CPU is max(cpu_secs, compile_wall).
/// The learner's 10 MiB limit can't apply here: a cold build of `runtime`
/// writes a ~13 MB archive, and the compiler is multi-threaded so CPU time can
/// outrun wall time. Only the trusted toolchain runs in this phase (learner
/// code never executes), so these bounds just stop a runaway. The learner's
/// program in phase 2 gets exactly `cpu_secs` and `max_file_bytes`.
const COMPILE_MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;

const FILE_TOO_LARGE_NOTE: &str =
    "\nRewire stopped the program: it tried to write a file bigger than the size limit.\n";

/// Runs Go with the toolchain at `go_root` (bound read-only at `/opt/go`).
///
/// `cache_dir` is persistent and shared across runs (`build/` is GOCACHE,
/// `gopath/` is GOPATH); it is created if missing. Each run gets a fresh
/// temporary workdir, removed afterwards.
pub struct GoNativeRuntime {
    pub go_root: PathBuf,
    pub cache_dir: PathBuf,
}

#[async_trait::async_trait]
impl LanguageRuntime for GoNativeRuntime {
    fn id(&self) -> &'static str {
        "go"
    }

    /// Phase 1 compiles (`go build -o /work/prog .`) under `compile_wall`; a
    /// compile failure returns the compiler's stderr unchanged with its exit
    /// code and `duration_ms: 0`, and phase 2 never runs. Phase 2 runs
    /// `/work/prog` under the learner limits. `duration_ms` covers phase 2 only.
    async fn run(&self, files: Vec<SourceFile>, limits: Limits) -> Result<RawRunResult, LabError> {
        validate_names(&files)?;
        if !self.go_root.join("bin/go").is_file() {
            return Err(LabError::ToolchainMissing);
        }
        sandbox_probe_async().await?;

        std::fs::create_dir_all(self.cache_dir.join("build"))?;
        std::fs::create_dir_all(self.cache_dir.join("gopath"))?;

        // Dropped (and removed) on every return path.
        let workdir = tempfile::Builder::new().prefix("rewire-lab-").tempdir()?;
        std::fs::write(workdir.path().join("go.mod"), GO_MOD)?;
        for f in &files {
            std::fs::write(workdir.path().join(&f.name), &f.source)?;
        }

        let compile_sb = Sandbox {
            go_root: &self.go_root,
            workdir: workdir.path(),
            cache_dir: &self.cache_dir,
            network: limits.network,
            tmp_bytes: COMPILE_TMP_BYTES,
        };
        let run_sb = Sandbox {
            tmp_bytes: RUN_TMP_BYTES,
            ..compile_sb
        };

        let compile_secs =
            limits.compile_wall.as_secs() + u64::from(limits.compile_wall.subsec_nanos() > 0);
        let compile = run_phase(
            &compile_sb,
            &["go", "build", "-o", "/work/prog", "."],
            PhaseLimits {
                wall: limits.compile_wall,
                cpu_secs: limits.cpu_secs.max(compile_secs),
                max_file_bytes: limits.max_file_bytes.max(COMPILE_MAX_FILE_BYTES),
                max_output: limits.max_output,
            },
        )
        .await?;
        let v = verdict(&compile);
        if v.exit_code != Some(0) {
            return Ok(to_result(compile, 0));
        }

        let run = run_phase(
            &run_sb,
            &["/work/prog"],
            PhaseLimits {
                wall: limits.run_wall,
                cpu_secs: limits.cpu_secs,
                max_file_bytes: limits.max_file_bytes,
                max_output: limits.max_output,
            },
        )
        .await?;
        let ms = u64::try_from(run.elapsed.as_millis()).unwrap_or(u64::MAX);
        Ok(to_result(run, ms))
    }
}

fn to_result(o: PhaseOutcome, duration_ms: u64) -> RawRunResult {
    let v = verdict(&o);
    let mut stderr = String::from_utf8_lossy(&o.stderr).into_owned();
    if v.file_too_large {
        stderr.push_str(FILE_TOO_LARGE_NOTE);
    }
    RawRunResult {
        stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
        stderr,
        exit_code: v.exit_code,
        timed_out: v.timed_out,
        truncated: o.truncated,
        duration_ms,
    }
}

/// Accepts only distinct plain `[A-Za-z0-9_]+.go` basenames: no separators,
/// no `..`, no dotfiles, nothing that could land outside the workdir or
/// replace `go.mod`.
fn validate_names(files: &[SourceFile]) -> Result<(), LabError> {
    let mut seen = HashSet::new();
    for f in files {
        let ok = f.name.strip_suffix(".go").is_some_and(|stem| {
            !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        });
        if !ok {
            return Err(invalid(format!(
                "source file name {:?} is not a plain name like main.go",
                f.name
            )));
        }
        if !seen.insert(f.name.as_str()) {
            return Err(invalid(format!("source file {:?} appears twice", f.name)));
        }
    }
    Ok(())
}

fn invalid(msg: String) -> LabError {
    LabError::Io(io::Error::new(io::ErrorKind::InvalidInput, msg))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(n: &str) -> Vec<SourceFile> {
        vec![SourceFile {
            name: n.into(),
            source: String::new(),
        }]
    }

    #[test]
    fn accepts_plain_go_names() {
        for n in ["main.go", "city.go", "setup.go", "a_b9.go", "X.go"] {
            assert!(validate_names(&named(n)).is_ok(), "{n}");
        }
    }

    #[test]
    fn rejects_traversal_and_odd_names() {
        for n in [
            "../x.go", "a/b.go", "/x.go", "..", ".go", "x..go", "x.go/", "x.GO", "go.mod",
            "x.go\0", "a\\b.go", " x.go", "",
        ] {
            assert!(validate_names(&named(n)).is_err(), "{n:?}");
        }
    }
}
