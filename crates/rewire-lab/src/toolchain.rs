//! Pinned Go toolchain manager: download once, verify, install atomically.
//!
//! The official `go.dev` Linux tarball is fully static, which is what the
//! sandbox needs (it has no `/usr`). Layout under the app data dir:
//! `toolchains/go1.27.2/` (ready once it holds the `.rewire-ok` marker).

use std::fmt;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use futures_util::StreamExt;
use sha2::{Digest, Sha256};

use crate::go_native::GoNativeRuntime;
use crate::runtime::{LabError, LanguageRuntime, Limits, SourceFile};

/// The one Go release the lab is pinned to.
pub const GO_VERSION: &str = "go1.27.2";

const MARKER: &str = ".rewire-ok";
const ENV_OVERRIDE: &str = "REWIRE_GO_ROOT";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// No bytes for this long counts as being offline.
const STALL_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolchainStatus {
    Ready { go_root: PathBuf },
    Missing,
}

#[derive(Debug)]
pub enum ToolchainError {
    /// Could not reach the download server (connect, DNS, timeout, stall).
    Offline,
    ChecksumMismatch,
    /// This OS/CPU has no pinned toolchain, or the archive is not acceptable.
    Unsupported(String),
    Io(io::Error),
}

impl fmt::Display for ToolchainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Offline => write!(f, "could not reach the Go download server"),
            Self::ChecksumMismatch => write!(f, "the downloaded Go toolchain failed its checksum"),
            Self::Unsupported(why) => write!(f, "unsupported: {why}"),
            Self::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for ToolchainError {}

impl From<io::Error> for ToolchainError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// `REWIRE_GO_ROOT` wins when set and it holds `bin/go`; otherwise the managed
/// toolchain counts only if its marker exists.
pub fn status(data_dir: &Path) -> ToolchainStatus {
    let env = std::env::var_os(ENV_OVERRIDE).map(PathBuf::from);
    status_with_env(data_dir, env.as_deref())
}

/// [`status`] with the env override passed in, so tests don't touch the process
/// environment.
pub fn status_with_env(data_dir: &Path, env_override: Option<&Path>) -> ToolchainStatus {
    if let Some(root) = env_override
        && root.join("bin/go").is_file()
    {
        return ToolchainStatus::Ready {
            go_root: root.to_path_buf(),
        };
    }
    let root = managed_root(data_dir);
    if root.join(MARKER).is_file() {
        ToolchainStatus::Ready { go_root: root }
    } else {
        ToolchainStatus::Missing
    }
}

fn toolchains_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("toolchains")
}

fn managed_root(data_dir: &Path) -> PathBuf {
    toolchains_dir(data_dir).join(GO_VERSION)
}

fn archive_name() -> Result<String, ToolchainError> {
    if std::env::consts::OS != "linux" {
        return Err(ToolchainError::Unsupported(format!(
            "the Go lab is only available on Linux for now (this is {})",
            std::env::consts::OS
        )));
    }
    let arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => {
            return Err(ToolchainError::Unsupported(format!(
                "no Go toolchain for {other}"
            )));
        }
    };
    Ok(format!("{GO_VERSION}.linux-{arch}.tar.gz"))
}

fn map_net(e: reqwest::Error) -> ToolchainError {
    // Connect, DNS, timeouts and mid-body failures all mean "can't get it now".
    if e.is_connect() || e.is_timeout() || e.is_request() || e.is_body() || e.is_decode() {
        ToolchainError::Offline
    } else {
        ToolchainError::Io(io::Error::other(e))
    }
}

fn check_status(resp: reqwest::Response) -> Result<reqwest::Response, ToolchainError> {
    let status = resp.status();
    if status.is_success() {
        Ok(resp)
    } else if status.is_server_error() {
        Err(ToolchainError::Offline)
    } else {
        Err(ToolchainError::Io(io::Error::other(format!(
            "download server answered {status}"
        ))))
    }
}

/// The expected archive: lowercase hex SHA-256 and byte size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pinned {
    pub sha256: &'static str,
    pub size: u64,
}

/// SHA-256 and size of `go1.27.2.linux-amd64.tar.gz` (checked against go.dev
/// and dl.google.com). Pinned in the binary so a compromised mirror can't
/// vouch for its own archive.
pub const SHA256_LINUX_AMD64: &str =
    "ecbadb99091a3f46e31f5f934b068b1864eafa7995211b39eaddf76996045fe5";
pub const SIZE_LINUX_AMD64: u64 = 70_590_635;
/// Same for `go1.27.2.linux-arm64.tar.gz`.
pub const SHA256_LINUX_ARM64: &str =
    "94f3e30b8e374bc285e7dadc11e0865726b9bc6e85b841ccceaabc0214c6b7c8";
pub const SIZE_LINUX_ARM64: u64 = 67_076_180;

fn pinned_for_host() -> Result<Pinned, ToolchainError> {
    archive_name()?;
    Ok(match std::env::consts::ARCH {
        "x86_64" => Pinned {
            sha256: SHA256_LINUX_AMD64,
            size: SIZE_LINUX_AMD64,
        },
        _ => Pinned {
            sha256: SHA256_LINUX_ARM64,
            size: SIZE_LINUX_ARM64,
        },
    })
}

/// Hosts a production download may be redirected to.
const REDIRECT_HOSTS: [&str; 2] = ["go.dev", "dl.google.com"];

/// Serializes installs within the process (double clicks, retries).
static INSTALL_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn is_loopback(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "::1")
}

/// `https` only, except loopback (used by tests). Returns the host.
fn validate_base(base: &str) -> Result<(String, bool), ToolchainError> {
    let url = reqwest::Url::parse(base)
        .map_err(|e| ToolchainError::Unsupported(format!("bad download URL: {e}")))?;
    let host = url.host_str().unwrap_or("").to_ascii_lowercase();
    let loopback = is_loopback(&host);
    match url.scheme() {
        "https" => Ok((host, false)),
        "http" if loopback => Ok((host, true)),
        _ => Err(ToolchainError::Unsupported(
            "the download URL must use https".into(),
        )),
    }
}

/// Downloads, verifies and installs the pinned toolchain. Returns its root.
/// On any failure nothing is left behind except `toolchains/` itself. An
/// already-installed toolchain is returned as is, without any download.
pub async fn install(
    data_dir: &Path,
    base_url: &str,
    on_progress: impl Fn(u64, u64),
) -> Result<PathBuf, ToolchainError> {
    install_with(
        data_dir,
        base_url,
        pinned_for_host()?,
        STALL_TIMEOUT,
        on_progress,
    )
    .await
}

/// [`install`] with the expected archive and stall limit passed in. Only for
/// tests; production always goes through [`install`] and the pinned values.
#[doc(hidden)]
pub async fn install_with(
    data_dir: &Path,
    base_url: &str,
    pinned: Pinned,
    stall: Duration,
    on_progress: impl Fn(u64, u64),
) -> Result<PathBuf, ToolchainError> {
    let name = archive_name()?;
    let (base_host, base_is_loopback) = validate_base(base_url)?;
    let base = base_url.trim_end_matches('/');

    let _guard = INSTALL_LOCK.lock().await;
    let toolchains = toolchains_dir(data_dir);
    std::fs::create_dir_all(&toolchains)?;
    sweep_stale(&toolchains);
    let target = managed_root(data_dir);
    if target.join(MARKER).is_file() {
        return Ok(target);
    }

    let policy = reqwest::redirect::Policy::custom(move |attempt| {
        if attempt.previous().len() >= 5 {
            return attempt.error("too many redirects");
        }
        let host = attempt.url().host_str().unwrap_or("").to_ascii_lowercase();
        let ok = if base_is_loopback {
            host == base_host
        } else {
            attempt.url().scheme() == "https" && REDIRECT_HOSTS.contains(&host.as_str())
        };
        if ok {
            attempt.follow()
        } else {
            attempt.error("redirect to a host that is not allowed")
        }
    });
    let client = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(stall)
        .redirect(policy)
        .build()
        .map_err(|e| ToolchainError::Io(io::Error::other(e)))?;

    // Stream to a temp file next to the final location, hashing as we go.
    let mut download = tempfile::Builder::new()
        .prefix(".dl-")
        .tempfile_in(&toolchains)?;
    let resp = check_status(
        client
            .get(format!("{base}/{name}"))
            .send()
            .await
            .map_err(map_net)?,
    )?;
    let total = pinned.size;
    let mut hasher = Sha256::new();
    let mut done = 0u64;
    let mut stream = resp.bytes_stream();
    loop {
        let next = tokio::time::timeout(stall, stream.next())
            .await
            .map_err(|_| ToolchainError::Offline)?;
        let Some(chunk) = next else { break };
        let chunk = chunk.map_err(map_net)?;
        hasher.update(&chunk);
        download.write_all(&chunk)?;
        done += chunk.len() as u64;
        on_progress(done, total);
    }
    download.flush()?;

    let actual: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if actual != pinned.sha256.to_ascii_lowercase() {
        return Err(ToolchainError::ChecksumMismatch); // temp file removed on drop
    }

    let archive_path = download.path().to_path_buf();
    let toolchains2 = toolchains.clone();
    let target2 = target.clone();
    tokio::task::spawn_blocking(move || extract_and_place(&archive_path, &toolchains2, &target2))
        .await
        .map_err(|e| ToolchainError::Io(io::Error::other(e)))??;
    drop(download);
    Ok(target)
}

/// Removes leftovers of crashed installs. Called with the install lock held.
fn sweep_stale(toolchains: &Path) {
    let Ok(rd) = std::fs::read_dir(toolchains) else {
        return;
    };
    for e in rd.flatten() {
        let n = e.file_name();
        let n = n.to_string_lossy();
        if n.starts_with(".dl-") || n.starts_with(".tmp-") {
            let p = e.path();
            let _ = if p.is_dir() {
                std::fs::remove_dir_all(&p)
            } else {
                std::fs::remove_file(&p)
            };
        }
    }
}

/// Drops setuid/setgid/sticky bits from everything under `dir`.
fn mask_modes(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    for e in std::fs::read_dir(dir)? {
        let e = e?;
        let ft = e.file_type()?;
        if ft.is_symlink() {
            continue;
        }
        let meta = e.metadata()?;
        let mode = meta.permissions().mode();
        if mode & !0o777 & 0o7777 != 0 {
            std::fs::set_permissions(e.path(), std::fs::Permissions::from_mode(mode & 0o777))?;
        }
        if ft.is_dir() {
            mask_modes(&e.path())?;
        }
    }
    Ok(())
}

fn extract_and_place(
    archive: &Path,
    toolchains: &Path,
    target: &Path,
) -> Result<(), ToolchainError> {
    // Removed on drop, including on every error path.
    let tmp = tempfile::Builder::new()
        .prefix(".tmp-")
        .tempdir_in(toolchains)?;
    extract_tar_gz(archive, tmp.path())?;
    let staged = tmp.path().join("go");
    if !staged.join("bin/go").is_file() {
        return Err(ToolchainError::Unsupported(
            "archive has no go/bin/go".into(),
        ));
    }
    mask_modes(&staged)?;
    std::fs::write(staged.join(MARKER), GO_VERSION)?;
    if target.join(MARKER).is_file() {
        return Ok(()); // a Ready toolchain is never replaced
    }
    // Without a marker the target is a half-finished earlier attempt.
    if target.exists() {
        std::fs::remove_dir_all(target)?;
    }
    std::fs::rename(&staged, target)?;
    Ok(())
}

/// Lexically safe relative path: only normal components (and `.`), no root,
/// no `..`.
fn is_safe_relative(p: &Path) -> bool {
    p.components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

fn unsafe_entry(why: &str, path: &Path) -> ToolchainError {
    ToolchainError::Unsupported(format!("unsafe archive entry {} ({why})", path.display()))
}

/// Extracts a `.tar.gz` into `dest`, refusing anything that could land outside
/// it: absolute paths, `..`, links with absolute or `..` targets, and entry
/// kinds other than files, directories and links. Everything must live under a
/// top-level `go/` directory.
fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<(), ToolchainError> {
    let file = std::fs::File::open(archive)?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
    tar.set_preserve_permissions(true);
    tar.set_unpack_xattrs(false);
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        if !is_safe_relative(&path) {
            return Err(unsafe_entry("path escapes the extraction root", &path));
        }
        if path
            .components()
            .find(|c| matches!(c, Component::Normal(_)))
            != Some(Component::Normal("go".as_ref()))
        {
            return Err(unsafe_entry("not under go/", &path));
        }
        let kind = entry.header().entry_type();
        if kind.is_symlink() || kind.is_hard_link() {
            let link = entry
                .link_name()?
                .ok_or_else(|| unsafe_entry("link without target", &path))?
                .into_owned();
            // `..` is rejected outright: a lexical check can't see through
            // earlier symlinks, so only plain descending targets are allowed.
            if !is_safe_relative(&link) {
                return Err(unsafe_entry(
                    "link target leaves the extraction root",
                    &path,
                ));
            }
        } else if !(kind.is_file() || kind.is_dir()) {
            if kind.is_pax_global_extensions() || kind.is_pax_local_extensions() {
                continue;
            }
            return Err(unsafe_entry("unsupported entry type", &path));
        }
        if !entry.unpack_in(dest)? {
            return Err(unsafe_entry("refused by the unpacker", &path));
        }
    }
    Ok(())
}

/// Compiles and runs a hello program once so `fmt` (and `runtime`) are in the
/// build cache before the learner's first run.
pub async fn warm_cache(rt: &GoNativeRuntime) -> Result<(), LabError> {
    let hello = SourceFile {
        name: "main.go".into(),
        source: "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"ready\")\n}\n"
            .into(),
    };
    let out = rt.run(vec![hello], Limits::default()).await?;
    if out.exit_code == Some(0) {
        Ok(())
    } else {
        Err(LabError::Io(io::Error::other(format!(
            "warm-up program failed: {}",
            out.stderr.trim()
        ))))
    }
}
