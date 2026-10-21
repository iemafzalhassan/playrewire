//! Toolchain manager tests: everything is served by a local httpmock server.

use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

use httpmock::prelude::*;
use rewire_lab::{
    GO_VERSION, GoNativeRuntime, ToolchainError, ToolchainStatus, install, status_with_env,
    warm_cache,
};
use sha2::{Digest, Sha256};

fn arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => panic!("unsupported test arch {other}"),
    }
}

fn filename() -> String {
    format!("{GO_VERSION}.linux-{}.tar.gz", arch())
}

fn sha_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn gz(tar_bytes: Vec<u8>) -> Vec<u8> {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    enc.write_all(&tar_bytes).unwrap();
    enc.finish().unwrap()
}

/// A tiny tarball shaped like the real one: `go/bin/go` plus `go/VERSION`.
fn fake_tarball() -> Vec<u8> {
    let mut b = tar::Builder::new(Vec::new());
    let script = b"#!/bin/sh\necho fake go\n";
    let mut h = tar::Header::new_gnu();
    h.set_size(script.len() as u64);
    h.set_mode(0o755);
    h.set_cksum();
    b.append_data(&mut h, "go/bin/go", &script[..]).unwrap();
    let v = GO_VERSION.as_bytes();
    let mut h = tar::Header::new_gnu();
    h.set_size(v.len() as u64);
    h.set_mode(0o644);
    h.set_cksum();
    b.append_data(&mut h, "go/VERSION", v).unwrap();
    gz(b.into_inner().unwrap())
}

/// Raw entry with an unvalidated name/link, which `tar::Builder` refuses to write.
fn raw_entry(out: &mut Vec<u8>, name: &str, kind: u8, link: &str, data: &[u8]) {
    let mut h = tar::Header::new_gnu();
    {
        let old = h.as_old_mut();
        old.name[..name.len()].copy_from_slice(name.as_bytes());
        old.linkname[..link.len()].copy_from_slice(link.as_bytes());
    }
    h.set_entry_type(tar::EntryType::new(kind));
    h.set_size(data.len() as u64);
    h.set_mode(0o644);
    h.set_cksum();
    out.extend_from_slice(h.as_bytes());
    out.extend_from_slice(data);
    out.resize(out.len().div_ceil(512) * 512, 0);
}

fn finish_raw(mut out: Vec<u8>) -> Vec<u8> {
    out.extend_from_slice(&[0u8; 1024]);
    gz(out)
}

fn listing(tarball: &[u8], claimed_sha: &str) -> String {
    serde_json::json!([
        {"version": "go1.26.0", "files": []},
        {"version": GO_VERSION, "files": [
            {"filename": format!("{GO_VERSION}.src.tar.gz"), "os": "", "arch": "", "sha256": "00", "size": 1, "kind": "source"},
            {"filename": filename(), "os": "linux", "arch": arch(), "sha256": claimed_sha,
             "size": tarball.len(), "kind": "archive"}
        ]}
    ])
    .to_string()
}

fn serve(server: &MockServer, tarball: &[u8], claimed_sha: &str) {
    let body = listing(tarball, claimed_sha);
    server.mock(|when, then| {
        when.method(GET)
            .path("/")
            .query_param("mode", "json")
            .query_param("include", "all");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });
    let tb = tarball.to_vec();
    server.mock(|when, then| {
        when.method(GET).path(format!("/{}", filename()));
        then.status(200).body(tb);
    });
}

fn only_toolchains_dir(data: &Path) {
    let names: Vec<_> = std::fs::read_dir(data)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names, vec!["toolchains".to_string()], "data dir: {names:?}");
}

fn toolchains_entries(data: &Path) -> Vec<String> {
    match std::fs::read_dir(data.join("toolchains")) {
        Ok(rd) => rd
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect(),
        Err(_) => vec![],
    }
}

#[tokio::test]
async fn status_missing_then_ready_after_install() {
    let data = tempfile::tempdir().unwrap();
    assert_eq!(status_with_env(data.path(), None), ToolchainStatus::Missing);

    let tb = fake_tarball();
    let server = MockServer::start();
    serve(&server, &tb, &sha_hex(&tb));

    let root = install(data.path(), &server.base_url(), |_, _| {})
        .await
        .unwrap();
    assert_eq!(root, data.path().join("toolchains").join(GO_VERSION));
    assert!(root.join("bin/go").is_file());
    assert!(root.join(".rewire-ok").is_file());
    assert_eq!(
        status_with_env(data.path(), None),
        ToolchainStatus::Ready { go_root: root }
    );
    // Only the final directory remains in toolchains/.
    assert_eq!(
        toolchains_entries(data.path()),
        vec![GO_VERSION.to_string()]
    );
}

#[test]
fn env_override_wins() {
    let data = tempfile::tempdir().unwrap();
    let fake = tempfile::tempdir().unwrap();
    // Without bin/go the override is ignored.
    assert_eq!(
        status_with_env(data.path(), Some(fake.path())),
        ToolchainStatus::Missing
    );
    std::fs::create_dir_all(fake.path().join("bin")).unwrap();
    std::fs::write(fake.path().join("bin/go"), "#!/bin/sh\n").unwrap();
    assert_eq!(
        status_with_env(data.path(), Some(fake.path())),
        ToolchainStatus::Ready {
            go_root: fake.path().to_path_buf()
        }
    );
}

#[test]
fn directory_without_marker_is_not_ready() {
    let data = tempfile::tempdir().unwrap();
    let dir = data.path().join("toolchains").join(GO_VERSION);
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    std::fs::write(dir.join("bin/go"), "x").unwrap();
    assert_eq!(status_with_env(data.path(), None), ToolchainStatus::Missing);
}

#[tokio::test]
async fn checksum_mismatch_rejected_and_nothing_left_behind() {
    let data = tempfile::tempdir().unwrap();
    let tb = fake_tarball();
    let server = MockServer::start();
    serve(&server, &tb, &"0".repeat(64));

    let err = install(data.path(), &server.base_url(), |_, _| {})
        .await
        .unwrap_err();
    assert!(matches!(err, ToolchainError::ChecksumMismatch), "{err:?}");
    assert_eq!(status_with_env(data.path(), None), ToolchainStatus::Missing);
    assert!(
        toolchains_entries(data.path()).is_empty(),
        "{:?}",
        toolchains_entries(data.path())
    );
    only_toolchains_dir(data.path());
}

#[tokio::test]
async fn offline_maps_to_offline_error() {
    let data = tempfile::tempdir().unwrap();
    let err = install(data.path(), "http://127.0.0.1:1", |_, _| {})
        .await
        .unwrap_err();
    assert!(matches!(err, ToolchainError::Offline), "{err:?}");
    assert_eq!(status_with_env(data.path(), None), ToolchainStatus::Missing);
}

#[tokio::test]
async fn progress_reported() {
    let data = tempfile::tempdir().unwrap();
    let tb = fake_tarball();
    let server = MockServer::start();
    serve(&server, &tb, &sha_hex(&tb));

    let calls = Mutex::new(Vec::<(u64, u64)>::new());
    install(data.path(), &server.base_url(), |d, t| {
        calls.lock().unwrap().push((d, t))
    })
    .await
    .unwrap();
    let calls = calls.into_inner().unwrap();
    assert!(!calls.is_empty());
    assert!(
        calls.iter().all(|&(_, t)| t == tb.len() as u64),
        "{calls:?}"
    );
    assert!(
        calls.windows(2).all(|w| w[0].0 <= w[1].0),
        "monotonic: {calls:?}"
    );
    assert_eq!(calls.last().unwrap().0, tb.len() as u64);
}

#[tokio::test]
async fn missing_listing_entry_is_unsupported() {
    let data = tempfile::tempdir().unwrap();
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/").query_param("mode", "json");
        then.status(200)
            .body(r#"[{"version":"go1.26.0","files":[]}]"#);
    });
    let err = install(data.path(), &server.base_url(), |_, _| {})
        .await
        .unwrap_err();
    assert!(matches!(err, ToolchainError::Unsupported(_)), "{err:?}");
    assert!(toolchains_entries(data.path()).is_empty());
}

#[tokio::test]
async fn half_finished_target_without_marker_is_replaced() {
    let data = tempfile::tempdir().unwrap();
    let stale = data.path().join("toolchains").join(GO_VERSION);
    std::fs::create_dir_all(stale.join("junk")).unwrap();
    std::fs::write(stale.join("junk/old"), "x").unwrap();

    let tb = fake_tarball();
    let server = MockServer::start();
    serve(&server, &tb, &sha_hex(&tb));
    let root = install(data.path(), &server.base_url(), |_, _| {})
        .await
        .unwrap();
    assert!(!root.join("junk").exists());
    assert!(root.join(".rewire-ok").is_file());
}

async fn assert_malicious_rejected(entries: impl FnOnce(&mut Vec<u8>)) {
    let data = tempfile::tempdir().unwrap();
    let mut raw = Vec::new();
    raw_entry(&mut raw, "go/bin/go", b'0', "", b"#!/bin/sh\n");
    entries(&mut raw);
    let tb = finish_raw(raw);
    let server = MockServer::start();
    serve(&server, &tb, &sha_hex(&tb)); // checksum is right: the archive itself is hostile

    let err = install(data.path(), &server.base_url(), |_, _| {})
        .await
        .unwrap_err();
    assert!(
        matches!(err, ToolchainError::Unsupported(_) | ToolchainError::Io(_)),
        "{err:?}"
    );
    assert!(
        toolchains_entries(data.path()).is_empty(),
        "{:?}",
        toolchains_entries(data.path())
    );
    assert_eq!(status_with_env(data.path(), None), ToolchainStatus::Missing);
    // Nothing escaped next to the data dir either.
    let parent = data.path().parent().unwrap();
    assert!(!parent.join("evil").exists());
}

#[tokio::test]
async fn tar_slip_dotdot_rejected() {
    assert_malicious_rejected(|r| raw_entry(r, "go/../../evil", b'0', "", b"pwn")).await;
}

#[tokio::test]
async fn tar_slip_absolute_path_rejected() {
    assert_malicious_rejected(|r| raw_entry(r, "/tmp/rewire-evil-abs", b'0', "", b"pwn")).await;
}

#[tokio::test]
async fn tar_slip_symlink_escape_rejected() {
    assert_malicious_rejected(|r| raw_entry(r, "go/evil", b'2', "../../../etc", b"")).await;
}

#[tokio::test]
async fn tar_slip_absolute_symlink_rejected() {
    assert_malicious_rejected(|r| raw_entry(r, "go/evil", b'2', "/etc/passwd", b"")).await;
}

#[tokio::test]
async fn tar_slip_hardlink_escape_rejected() {
    assert_malicious_rejected(|r| raw_entry(r, "go/evil", b'1', "../../etc/passwd", b"")).await;
}

#[tokio::test]
async fn tar_slip_write_through_symlink_rejected() {
    assert_malicious_rejected(|r| {
        raw_entry(r, "go/l", b'2', "..", b"");
        raw_entry(r, "go/l/evil", b'0', "", b"pwn");
    })
    .await;
}

#[tokio::test]
async fn archive_without_go_dir_rejected() {
    assert_malicious_rejected(|r| raw_entry(r, "other/file", b'0', "", b"x")).await;
}

/// Reuses the system Go (its GOROOT is a static official toolchain).
#[tokio::test]
async fn warm_cache_compiles_hello() {
    if rewire_lab::sandbox_probe().is_err() {
        eprintln!("sandbox unavailable; skipping");
        return;
    }
    let out = std::process::Command::new("go")
        .args(["env", "GOROOT"])
        .output();
    let Ok(out) = out else { return };
    let go_root = String::from_utf8(out.stdout).unwrap().trim().into();
    let cache = tempfile::tempdir().unwrap();
    let rt = GoNativeRuntime {
        go_root,
        cache_dir: cache.path().to_path_buf(),
    };
    warm_cache(&rt).await.unwrap();
    assert!(cache.path().join("build").is_dir());
}
