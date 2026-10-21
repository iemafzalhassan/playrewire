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
use serde::Deserialize;
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

#[derive(Deserialize)]
struct Release {
    #[serde(default)]
    files: Vec<ReleaseFile>,
}

#[derive(Deserialize)]
struct ReleaseFile {
    filename: String,
    #[serde(default)]
    sha256: String,
    #[serde(default)]
    size: u64,
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

/// Downloads, verifies and installs the pinned toolchain. Returns its root.
/// On any failure nothing is left behind except `toolchains/` itself.
pub async fn install(
    data_dir: &Path,
    base_url: &str,
    on_progress: impl Fn(u64, u64),
) -> Result<PathBuf, ToolchainError> {
    let name = archive_name()?;
    let base = base_url.trim_end_matches('/');
    let client = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .build()
        .map_err(|e| ToolchainError::Io(io::Error::other(e)))?;

    // 1. Expected checksum from the official listing.
    let releases: Vec<Release> = check_status(
        client
            .get(format!("{base}/?mode=json&include=all"))
            .send()
            .await
            .map_err(map_net)?,
    )?
    .json()
    .await
    .map_err(map_net)?;
    let entry = releases
        .into_iter()
        .flat_map(|r| r.files)
        .find(|f| f.filename == name)
        .ok_or_else(|| ToolchainError::Unsupported(format!("{name} is not in the Go listing")))?;
    let expected = entry.sha256.to_ascii_lowercase();

    // 2. Stream to a temp file next to the final location, hashing as we go.
    let toolchains = toolchains_dir(data_dir);
    std::fs::create_dir_all(&toolchains)?;
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
    let total = if entry.size > 0 {
        entry.size
    } else {
        resp.content_length().unwrap_or(0)
    };
    let mut hasher = Sha256::new();
    let mut done = 0u64;
    let mut stream = resp.bytes_stream();
    loop {
        let next = tokio::time::timeout(STALL_TIMEOUT, stream.next())
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

    // 3. Verify.
    let actual: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if actual != expected {
        return Err(ToolchainError::ChecksumMismatch); // temp file removed on drop
    }

    // 4. Extract into a temp dir, mark, and rename into place.
    let target = managed_root(data_dir);
    let archive_path = download.path().to_path_buf();
    let toolchains2 = toolchains.clone();
    let target2 = target.clone();
    tokio::task::spawn_blocking(move || extract_and_place(&archive_path, &toolchains2, &target2))
        .await
        .map_err(|e| ToolchainError::Io(io::Error::other(e)))??;
    drop(download);
    Ok(target)
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
    std::fs::write(staged.join(MARKER), GO_VERSION)?;
    // A target without a marker is a half-finished earlier attempt. (A target
    // with one means a concurrent or earlier success; replace it too.)
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
