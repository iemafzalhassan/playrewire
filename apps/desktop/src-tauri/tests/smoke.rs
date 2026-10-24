//! `rewire --smoke` checks, run without the Tauri runtime.

use std::path::{Path, PathBuf};

use rewire_desktop::smoke;
use rewire_desktop::state::AppState;
use rewire_mentor::MemoryStore;

fn open(dir: &Path, go: Option<PathBuf>) -> AppState {
    AppState::open(dir, Box::new(MemoryStore::default()), go).expect("open app state")
}

#[test]
fn flag_is_detected_anywhere_in_argv() {
    let args = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>().into_iter();
    assert!(smoke::requested(args(&["rewire", "--smoke"])));
    assert!(!smoke::requested(args(&["rewire"])));
    assert!(!smoke::requested(args(&["rewire", "--smoker"])));
}

#[test]
fn hello_drill_is_city_setup_main_in_order() {
    let files = smoke::hello_drill();
    let names: Vec<_> = files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["city.go", "setup.go", "main.go"]);
    assert!(files[0].source.starts_with("package main"));
    assert!(files[2].source.contains("fmt.Println(\"hello\")"));
}

#[tokio::test]
async fn smoke_without_toolchain_fails_with_a_reason() {
    let dir = tempfile::tempdir().unwrap();
    let st = open(dir.path(), None);
    let err = smoke::check(&st).await.unwrap_err();
    assert!(err.contains("REWIRE_GO_ROOT"), "{err}");
}

/// The real thing: the host Go toolchain through bwrap. Skips (with a reason)
/// where the sandbox can't start, like the rewire-lab tests.
#[tokio::test]
async fn smoke_passes_with_a_go_root_and_a_working_sandbox() {
    if let Err(e) = rewire_lab::sandbox_probe() {
        eprintln!("skipping: {e}");
        return;
    }
    let Ok(out) = std::process::Command::new("go").args(["env", "GOROOT"]).output() else {
        eprintln!("skipping: no go on PATH");
        return;
    };
    let root = PathBuf::from(String::from_utf8(out.stdout).unwrap().trim());
    let dir = tempfile::tempdir().unwrap();
    let st = open(dir.path(), Some(root));
    smoke::check(&st).await.expect("smoke check");
}
