//! `rewire --smoke`: a release check that boots the app, runs the first
//! hello drill through the real sandbox and prints `SMOKE OK`.
//!
//! It never downloads anything. The Go toolchain must already be there:
//! installed in the data dir, or pointed at with `REWIRE_GO_ROOT` (what the
//! container and CI smoke tests do).

use rewire_lab::{LanguageRuntime, Limits, SourceFile, ToolchainStatus};

use crate::state::AppState;

/// The command-line flag that turns a normal launch into a smoke test.
pub const FLAG: &str = "--smoke";

/// Printed on stdout when every check passed.
pub const OK_LINE: &str = "SMOKE OK";

/// The "city" stub every lesson program is compiled with (ruling R9).
const CITY_GO: &str = include_str!("../../../../content/go/city/city.go");

/// The first drill of `go-academy/hello-print`: "Print the word hello.",
/// wrapped exactly as the UI and packcheck wrap body-of-main code.
const MAIN_GO: &str = "package main\n\nimport \"fmt\"\n\nvar _ = fmt.Sprint\n\nfunc main() {\nfmt.Println(\"hello\")\n}\n";
const EXPECT_STDOUT: &str = "hello\n";

/// True when the process was started with `--smoke`.
pub fn requested(mut args: impl Iterator<Item = String>) -> bool {
    args.any(|a| a == FLAG)
}

/// The drill's files in the order the UI sends them: city.go, setup.go, main.go.
pub fn hello_drill() -> Vec<SourceFile> {
    vec![
        SourceFile { name: "city.go".into(), source: CITY_GO.into() },
        SourceFile { name: "setup.go".into(), source: "package main\n".into() },
        SourceFile { name: "main.go".into(), source: MAIN_GO.into() },
    ]
}

/// Runs the checks. `Err` is a one-line reason for the person reading the
/// CI log; it may name paths and tool output (unlike command errors).
pub async fn check(st: &AppState) -> Result<(), String> {
    let ToolchainStatus::Ready { go_root } = st.toolchain() else {
        return Err("no Go toolchain: set REWIRE_GO_ROOT to a Go root (with bin/go), or install it in the app first".into());
    };
    rewire_lab::sandbox_probe().map_err(|e| format!("sandbox probe failed (bubblewrap can't create its namespaces here): {e}"))?;
    let rt = st.runtime().ok_or("the Go runtime did not start")?;
    let out = rt
        .run(hello_drill(), Limits::default())
        .await
        .map_err(|e| format!("the hello drill could not run: {e}"))?;
    if out.exit_code != Some(0) || out.timed_out || out.truncated {
        return Err(format!(
            "the hello drill failed (go root {}): exit {:?}, timed out {}, truncated {}, stderr: {}",
            go_root.display(),
            out.exit_code,
            out.timed_out,
            out.truncated,
            out.stderr.trim()
        ));
    }
    if out.stdout != EXPECT_STDOUT {
        return Err(format!("the hello drill printed {:?}, expected {:?}", out.stdout, EXPECT_STDOUT));
    }
    Ok(())
}
