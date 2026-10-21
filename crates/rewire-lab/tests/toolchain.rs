//! Toolchain manager tests: everything is served by a local httpmock server.

use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use httpmock::prelude::*;
use rewire_lab::{
    GO_VERSION, GoNativeRuntime, Pinned, SHA256_LINUX_AMD64, SHA256_LINUX_ARM64, SIZE_LINUX_AMD64,
    SIZE_LINUX_ARM64, ToolchainError, ToolchainStatus, install, install_with, status_with_env,
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
    raw_entry_mode(out, name, kind, link, data, 0o644);
}

fn raw_entry_mode(out: &mut Vec<u8>, name: &str, kind: u8, link: &str, data: &[u8], mode: u32) {
    let mut h = tar::Header::new_gnu();
    {
        let old = h.as_old_mut();
        old.name[..name.len()].copy_from_slice(name.as_bytes());
        old.linkname[..link.len()].copy_from_slice(link.as_bytes());
    }
    h.set_entry_type(tar::EntryType::new(kind));
    h.set_size(data.len() as u64);
    h.set_mode(mode);
    h.set_cksum();
    out.extend_from_slice(h.as_bytes());
    out.extend_from_slice(data);
    out.resize(out.len().div_ceil(512) * 512, 0);
}

fn finish_raw(mut out: Vec<u8>) -> Vec<u8> {
    out.extend_from_slice(&[0u8; 1024]);
    gz(out)
}

fn pinned(tarball: &[u8]) -> Pinned {
    Pinned {
        sha256: Box::leak(sha_hex(tarball).into_boxed_str()),
        size: tarball.len() as u64,
    }
}

fn wrong_pin(tarball: &[u8]) -> Pinned {
    Pinned {
        sha256: Box::leak("0".repeat(64).into_boxed_str()),
        size: tarball.len() as u64,
    }
}

async fn run_install(
    data: &Path,
    base: &str,
    pin: Pinned,
    on_progress: impl Fn(u64, u64),
) -> Result<std::path::PathBuf, ToolchainError> {
    install_with(data, base, pin, Duration::from_secs(30), on_progress).await
}

/// Serves the archive; returns the mock so tests can count requests.
fn serve<'a>(server: &'a MockServer, tarball: &[u8]) -> httpmock::Mock<'a> {
    let tb = tarball.to_vec();
    server.mock(|when, then| {
        when.method(GET).path(format!("/{}", filename()));
        then.status(200).body(tb);
    })
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
    serve(&server, &tb);

    let root = run_install(data.path(), &server.base_url(), pinned(&tb), |_, _| {})
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
    serve(&server, &tb);

    let err = run_install(data.path(), &server.base_url(), wrong_pin(&tb), |_, _| {})
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
    serve(&server, &tb);

    let calls = Mutex::new(Vec::<(u64, u64)>::new());
    run_install(data.path(), &server.base_url(), pinned(&tb), |d, t| {
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
async fn half_finished_target_without_marker_is_replaced() {
    let data = tempfile::tempdir().unwrap();
    let stale = data.path().join("toolchains").join(GO_VERSION);
    std::fs::create_dir_all(stale.join("junk")).unwrap();
    std::fs::write(stale.join("junk/old"), "x").unwrap();

    let tb = fake_tarball();
    let server = MockServer::start();
    serve(&server, &tb);
    let root = run_install(data.path(), &server.base_url(), pinned(&tb), |_, _| {})
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
    serve(&server, &tb); // checksum is right: the archive itself is hostile

    let err = run_install(data.path(), &server.base_url(), pinned(&tb), |_, _| {})
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

#[tokio::test]
async fn second_install_is_noop_and_keeps_toolchain() {
    let data = tempfile::tempdir().unwrap();
    let tb = fake_tarball();
    let server = MockServer::start();
    let archive = serve(&server, &tb);
    let root = run_install(data.path(), &server.base_url(), pinned(&tb), |_, _| {})
        .await
        .unwrap();
    std::fs::write(root.join("sentinel"), "keep").unwrap();

    let again = run_install(data.path(), &server.base_url(), pinned(&tb), |_, _| {})
        .await
        .unwrap();
    assert_eq!(again, root);
    assert_eq!(
        std::fs::read_to_string(root.join("sentinel")).unwrap(),
        "keep"
    );
    assert_eq!(archive.calls(), 1, "second install must not download");
}

#[tokio::test]
async fn concurrent_installs_both_succeed() {
    let data = tempfile::tempdir().unwrap();
    let tb = fake_tarball();
    let server = MockServer::start();
    let archive = serve(&server, &tb);
    let url = server.base_url();
    let (a, b) = tokio::join!(
        run_install(data.path(), &url, pinned(&tb), |_, _| {}),
        run_install(data.path(), &url, pinned(&tb), |_, _| {}),
    );
    assert_eq!(a.unwrap(), b.unwrap());
    assert_eq!(archive.calls(), 1);
    assert_eq!(
        toolchains_entries(data.path()),
        vec![GO_VERSION.to_string()]
    );
}

#[tokio::test]
async fn stale_temp_entries_are_swept() {
    let data = tempfile::tempdir().unwrap();
    let t = data.path().join("toolchains");
    std::fs::create_dir_all(t.join(".tmp-old/go")).unwrap();
    std::fs::write(t.join(".dl-old"), "x").unwrap();
    let tb = fake_tarball();
    let server = MockServer::start();
    serve(&server, &tb);
    run_install(data.path(), &server.base_url(), pinned(&tb), |_, _| {})
        .await
        .unwrap();
    assert_eq!(
        toolchains_entries(data.path()),
        vec![GO_VERSION.to_string()]
    );
}

#[tokio::test]
async fn stalled_server_maps_to_offline() {
    let data = tempfile::tempdir().unwrap();
    let tb = fake_tarball();
    let server = MockServer::start();
    let body = tb.clone();
    server.mock(|when, then| {
        when.method(GET).path(format!("/{}", filename()));
        then.status(200).delay(Duration::from_secs(3)).body(body);
    });
    let started = std::time::Instant::now();
    let err = install_with(
        data.path(),
        &server.base_url(),
        pinned(&tb),
        Duration::from_millis(300),
        |_, _| {},
    )
    .await
    .unwrap_err();
    assert!(matches!(err, ToolchainError::Offline), "{err:?}");
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(toolchains_entries(data.path()).is_empty());
}

#[test]
fn pinned_constants_match_reviewed_values() {
    assert_eq!(
        SHA256_LINUX_AMD64,
        "ecbadb99091a3f46e31f5f934b068b1864eafa7995211b39eaddf76996045fe5"
    );
    assert_eq!(SIZE_LINUX_AMD64, 70_590_635);
    assert_eq!(
        SHA256_LINUX_ARM64,
        "94f3e30b8e374bc285e7dadc11e0865726b9bc6e85b841ccceaabc0214c6b7c8"
    );
    assert_eq!(SIZE_LINUX_ARM64, 67_076_180);
    assert_eq!(GO_VERSION, "go1.27.2");
}

#[tokio::test]
async fn non_https_non_loopback_base_url_rejected() {
    let data = tempfile::tempdir().unwrap();
    for url in ["http://go.dev/dl", "http://example.com", "ftp://go.dev/dl"] {
        let err = install(data.path(), url, |_, _| {}).await.unwrap_err();
        assert!(
            matches!(err, ToolchainError::Unsupported(_)),
            "{url}: {err:?}"
        );
    }
    assert!(!data.path().join("toolchains").exists());
}

#[tokio::test]
async fn redirect_to_other_host_rejected() {
    let data = tempfile::tempdir().unwrap();
    let tb = fake_tarball();
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path(format!("/{}", filename()));
        then.status(302)
            .header("location", "http://localhost:1/evil.tar.gz");
    });
    let err = run_install(data.path(), &server.base_url(), pinned(&tb), |_, _| {})
        .await
        .unwrap_err();
    assert!(!matches!(err, ToolchainError::ChecksumMismatch), "{err:?}");
    assert!(toolchains_entries(data.path()).is_empty());
}

#[tokio::test]
async fn setuid_bits_are_dropped() {
    use std::os::unix::fs::PermissionsExt;
    let data = tempfile::tempdir().unwrap();
    let mut raw = Vec::new();
    raw_entry_mode(&mut raw, "go/bin/go", b'0', "", b"#!/bin/sh\n", 0o6755);
    let tb = finish_raw(raw);
    let server = MockServer::start();
    serve(&server, &tb);
    let root = run_install(data.path(), &server.base_url(), pinned(&tb), |_, _| {})
        .await
        .unwrap();
    let mode = std::fs::metadata(root.join("bin/go"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o7000, 0, "{mode:o}");
    assert_eq!(mode & 0o777, 0o755);
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
