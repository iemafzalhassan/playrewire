//! bubblewrap sandbox: the pinned argument list, process-group control,
//! resource limits and the capped output reader.
//!
//! Containment comes from bwrap (`--unshare-all`: user, mount, pid, net, ipc,
//! uts and cgroup namespaces; a fresh empty root holding only the Go
//! toolchain read-only, the per-run workdir, the cache, a private `/proc`,
//! a minimal `/dev` and a private `/tmp`). This module adds what bwrap does not
//! do on its own: CPU and file-size rlimits, a wall clock, killing the whole
//! process group, and a cap on output.

use std::ffi::OsString;
use std::io;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::time::{Duration, Instant};

use tokio::io::AsyncReadExt;
use tokio::process::Command;

use crate::runtime::LabError;

/// User-facing message when bwrap can't start (e.g. Ubuntu's AppArmor
/// restriction on unprivileged user namespaces; Task 18 ships a profile).
pub const SANDBOX_UNAVAILABLE: &str =
    "Rewire couldn't start its safe practice space on this system. See docs: Linux sandbox setup.";

/// Checks that bubblewrap can create its namespaces on this host by running
/// `bwrap --unshare-all --ro-bind / / true`.
pub fn sandbox_probe() -> Result<(), LabError> {
    let ok = std::process::Command::new(bwrap_path())
        .args(["--unshare-all", "--ro-bind", "/", "/", "true"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err(LabError::SandboxUnavailable(
            SANDBOX_UNAVAILABLE.to_string(),
        ))
    }
}

/// Async twin of [`sandbox_probe`], used on the run path so it doesn't block
/// an executor thread.
pub(crate) async fn sandbox_probe_async() -> Result<(), LabError> {
    let ok = Command::new(bwrap_path())
        .args(["--unshare-all", "--ro-bind", "/", "/", "true"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err(LabError::SandboxUnavailable(
            SANDBOX_UNAVAILABLE.to_string(),
        ))
    }
}

/// Resolves `bwrap` on the host `PATH` once, so the sandboxed command itself
/// can be spawned with a cleared environment.
fn bwrap_path() -> PathBuf {
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join("bwrap");
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    PathBuf::from("/usr/bin/bwrap")
}

/// The mounts for one run.
pub(crate) struct Sandbox<'a> {
    pub go_root: &'a Path,
    pub workdir: &'a Path,
    pub cache_dir: &'a Path,
    pub network: bool,
    /// Size cap for the private `/tmp` tmpfs (`--size`).
    pub tmp_bytes: u64,
}

/// `/tmp` cap for the learner's program (controller ruling: 64 MiB).
pub(crate) const RUN_TMP_BYTES: u64 = 64 * 1024 * 1024;

/// `/tmp` cap for the compile phase. `go build` keeps its `$WORK` in `/tmp`,
/// and a cold build of a larger stdlib set (net/http, crypto/tls,
/// encoding/json, ...) needs about 100 MB there, so 64 MiB fails with
/// "no space left on device". Only the trusted toolchain runs in this phase.
pub(crate) const COMPILE_TMP_BYTES: u64 = 512 * 1024 * 1024;

impl Sandbox<'_> {
    /// The pinned bwrap argument list (Task 9 brief), in order, plus the
    /// controller's hardening ruling: --unshare-user --disable-userns,
    /// --size 64 MiB on /tmp, and --remount-ro for / and /dev.
    pub(crate) fn args(&self) -> Vec<OsString> {
        let mut a: Vec<OsString> = Vec::with_capacity(48);
        for x in [
            "--unshare-all",
            // --disable-userns blocks nested user namespaces (less kernel
            // attack surface). It requires an explicit --unshare-user:
            // --unshare-all alone only implies --unshare-user-try.
            "--unshare-user",
            "--disable-userns",
            "--die-with-parent",
            "--new-session",
            "--clearenv",
        ] {
            a.push(x.into());
        }
        a.push("--ro-bind".into());
        a.push(self.go_root.into());
        a.push("/opt/go".into());
        a.push("--bind".into());
        a.push(self.workdir.into());
        a.push("/work".into());
        a.push("--bind".into());
        a.push(self.cache_dir.into());
        a.push("/cache".into());
        for x in ["--proc", "/proc", "--dev", "/dev"] {
            a.push(x.into());
        }
        // Private /tmp with a size cap, so it can't eat host RAM.
        a.push("--size".into());
        a.push(self.tmp_bytes.to_string().into());
        for x in [
            "--tmpfs",
            "/tmp",
            // The root and /dev are private tmpfs mounts that bwrap leaves
            // writable. Make them read-only after every mount is in place;
            // /work, /cache and /tmp are separate mounts and stay writable.
            "--remount-ro",
            "/",
            "--remount-ro",
            "/dev",
            "--chdir",
            "/work",
            "--setenv",
            "GOROOT",
            "/opt/go",
            "--setenv",
            "GOCACHE",
            "/cache/build",
            "--setenv",
            "GOPATH",
            "/cache/gopath",
            "--setenv",
            "GOTOOLCHAIN",
            "local",
            "--setenv",
            "GOFLAGS",
            "-mod=mod",
            "--setenv",
            "CGO_ENABLED",
            "0",
            "--setenv",
            "HOME",
            "/tmp",
            "--setenv",
            "PATH",
            "/opt/go/bin",
        ] {
            a.push(x.into());
        }
        if self.network {
            // Only when a caller explicitly opts in; the MVP never does.
            a.insert(1, "--share-net".into());
        }
        a
    }
}

/// Limits for one phase (compile or run).
#[derive(Debug, Clone, Copy)]
pub(crate) struct PhaseLimits {
    pub wall: Duration,
    pub cpu_secs: u64,
    pub max_file_bytes: u64,
    pub max_output: usize,
}

/// What one sandboxed process did.
#[derive(Debug)]
pub(crate) struct PhaseOutcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// Exit status of the outer bwrap. bwrap reports a sandboxed child that
    /// died from signal N as exit code 128 + N.
    pub status: ExitStatus,
    /// We killed the group because the wall clock ran out.
    pub wall_killed: bool,
    /// We killed the group because the output cap was hit.
    pub truncated: bool,
    /// Spawn to reap.
    pub elapsed: Duration,
}

/// Runs `command` inside the sandbox under `lim`.
pub(crate) async fn run_phase(
    sb: &Sandbox<'_>,
    command: &[&str],
    lim: PhaseLimits,
) -> io::Result<PhaseOutcome> {
    let mut cmd = Command::new(bwrap_path());
    cmd.args(sb.args())
        .args(command)
        // bwrap's --clearenv already empties the sandbox env; clearing here too
        // keeps host variables away from bwrap itself.
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // If the run future is dropped, bwrap is SIGKILLed and
        // --die-with-parent plus the pid namespace take the rest down.
        .kill_on_drop(true);
    let (cpu, fsize) = (lim.cpu_secs, lim.max_file_bytes);
    // SAFETY: the closure only calls async-signal-safe libc functions
    // (setsid, getrlimit, setrlimit, close_range) and allocates nothing.
    unsafe {
        cmd.pre_exec(move || {
            // New session and process group (pgid == pid) so the whole
            // group can be killed with kill(-pgid).
            if libc::setsid() < 0 {
                return Err(io::Error::last_os_error());
            }
            set_rlimit(libc::RLIMIT_CPU, cpu, cpu.saturating_add(1))?;
            set_rlimit(libc::RLIMIT_FSIZE, fsize, fsize)?;
            // Defence in depth: no inherited host fd survives exec, even one
            // some other part of the app opened without O_CLOEXEC. Best
            // effort (needs Linux 5.11); std's own fds are already CLOEXEC.
            libc::syscall(
                libc::SYS_close_range,
                3 as libc::c_uint,
                libc::c_uint::MAX,
                libc::CLOSE_RANGE_CLOEXEC,
            );
            Ok(())
        });
    }

    let start = Instant::now();
    let mut child = cmd.spawn()?;
    let pgid = child.id().expect("a just-spawned child has a pid") as libc::pid_t;
    let mut out = child.stdout.take().expect("stdout is piped");
    let mut err = child.stderr.take().expect("stderr is piped");

    let deadline = tokio::time::Instant::now() + lim.wall;
    let sleep = tokio::time::sleep_until(deadline);
    tokio::pin!(sleep);

    let mut cap = OutputCap::new(lim.max_output);
    let (mut out_done, mut err_done) = (false, false);
    let mut out_buf = [0u8; 8192];
    let mut err_buf = [0u8; 8192];
    let mut wall_killed = false;

    // Read both pipes concurrently so neither can fill and block the child.
    while !(out_done && err_done) && !cap.truncated {
        tokio::select! {
            r = out.read(&mut out_buf), if !out_done => match r {
                Ok(0) | Err(_) => out_done = true,
                Ok(n) => cap.push(Stream::Out, &out_buf[..n]),
            },
            r = err.read(&mut err_buf), if !err_done => match r {
                Ok(0) | Err(_) => err_done = true,
                Ok(n) => cap.push(Stream::Err, &err_buf[..n]),
            },
            _ = &mut sleep => {
                wall_killed = true;
                break;
            }
        }
    }
    drop(out);
    drop(err);

    let status = if wall_killed || cap.truncated {
        kill_group(pgid);
        child.wait().await?
    } else {
        // Both pipes closed; the program may still be running (it can close
        // its own stdout). Wait for it under the same deadline.
        match tokio::time::timeout_at(deadline, child.wait()).await {
            Ok(status) => status?,
            Err(_) => {
                wall_killed = true;
                kill_group(pgid);
                child.wait().await?
            }
        }
    };
    // `child.wait()` has reaped bwrap; we never signal the pid after this.

    Ok(PhaseOutcome {
        stdout: cap.stdout,
        stderr: cap.stderr,
        status,
        wall_killed,
        truncated: cap.truncated,
        elapsed: start.elapsed(),
    })
}

/// SIGKILLs the whole process group. Called only before the leader is
/// reaped, so the pgid can't have been reused.
fn kill_group(pgid: libc::pid_t) {
    // SAFETY: plain syscall; a negative pid addresses the process group.
    unsafe {
        libc::kill(-pgid, libc::SIGKILL);
    }
}

/// Sets a resource limit, never above the inherited hard limit (raising it
/// would need privilege and fail the spawn).
fn set_rlimit(resource: libc::__rlimit_resource_t, soft: u64, hard: u64) -> io::Result<()> {
    let mut cur = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: valid out-pointer.
    if unsafe { libc::getrlimit(resource, &mut cur) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let max = hard.min(cur.rlim_max);
    let lim = libc::rlimit {
        rlim_cur: soft.min(max) as libc::rlim_t,
        rlim_max: max as libc::rlim_t,
    };
    // SAFETY: valid pointer to an initialised rlimit.
    if unsafe { libc::setrlimit(resource, &lim) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum Stream {
    Out,
    Err,
}

/// Collects stdout and stderr under one combined byte cap.
struct OutputCap {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    remaining: usize,
    truncated: bool,
}

impl OutputCap {
    fn new(max: usize) -> Self {
        Self {
            stdout: Vec::new(),
            stderr: Vec::new(),
            remaining: max,
            truncated: false,
        }
    }

    fn push(&mut self, stream: Stream, bytes: &[u8]) {
        let take = bytes.len().min(self.remaining);
        let dst = match stream {
            Stream::Out => &mut self.stdout,
            Stream::Err => &mut self.stderr,
        };
        dst.extend_from_slice(&bytes[..take]);
        self.remaining -= take;
        if take < bytes.len() {
            self.truncated = true;
        }
    }
}

/// How a phase ended, in `RawRunResult` terms.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Verdict {
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    /// The program hit RLIMIT_FSIZE and died from SIGXFSZ.
    pub file_too_large: bool,
}

const SIG_EXIT_BASE: i32 = 128;

pub(crate) fn verdict(o: &PhaseOutcome) -> Verdict {
    if o.wall_killed {
        return Verdict {
            exit_code: None,
            timed_out: true,
            file_too_large: false,
        };
    }
    if o.truncated {
        return Verdict {
            exit_code: None,
            timed_out: false,
            file_too_large: false,
        };
    }
    // bwrap's init turns "child died from signal N" into exit code 128 + N.
    // If bwrap itself was signalled (not by us), treat it the same way.
    let code = match (o.status.code(), o.status.signal()) {
        (Some(c), _) => c,
        (None, Some(sig)) => SIG_EXIT_BASE + sig,
        (None, None) => {
            return Verdict {
                exit_code: None,
                timed_out: false,
                file_too_large: false,
            };
        }
    };
    if code == SIG_EXIT_BASE + libc::SIGXCPU || code == SIG_EXIT_BASE + libc::SIGKILL {
        // RLIMIT_CPU: SIGXCPU at the soft limit, SIGKILL at the hard limit.
        return Verdict {
            exit_code: None,
            timed_out: true,
            file_too_large: false,
        };
    }
    Verdict {
        exit_code: Some(code),
        timed_out: false,
        file_too_large: code == SIG_EXIT_BASE + libc::SIGXFSZ,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(raw_wait_status: i32) -> PhaseOutcome {
        PhaseOutcome {
            stdout: vec![],
            stderr: vec![],
            status: ExitStatus::from_raw(raw_wait_status),
            wall_killed: false,
            truncated: false,
            elapsed: Duration::ZERO,
        }
    }
    fn exited(code: i32) -> PhaseOutcome {
        outcome(code << 8)
    }

    #[test]
    fn normal_exit_codes_pass_through() {
        assert_eq!(
            verdict(&exited(0)),
            Verdict {
                exit_code: Some(0),
                timed_out: false,
                file_too_large: false
            }
        );
        assert_eq!(verdict(&exited(2)).exit_code, Some(2));
    }

    #[test]
    fn rlimit_cpu_kill_is_a_timeout() {
        for code in [128 + libc::SIGXCPU, 128 + libc::SIGKILL] {
            assert_eq!(
                verdict(&exited(code)),
                Verdict {
                    exit_code: None,
                    timed_out: true,
                    file_too_large: false
                }
            );
        }
        assert!(
            verdict(&outcome(libc::SIGXCPU)).timed_out,
            "bwrap itself signalled"
        );
    }

    #[test]
    fn sigxfsz_is_a_nonzero_exit_flagged_file_too_large() {
        let v = verdict(&exited(128 + libc::SIGXFSZ));
        assert_eq!(
            v,
            Verdict {
                exit_code: Some(128 + libc::SIGXFSZ),
                timed_out: false,
                file_too_large: true
            }
        );
        assert!(verdict(&outcome(libc::SIGXFSZ)).file_too_large);
    }

    #[test]
    fn our_kills_win_over_the_status() {
        let mut o = exited(0);
        o.wall_killed = true;
        assert_eq!(
            verdict(&o),
            Verdict {
                exit_code: None,
                timed_out: true,
                file_too_large: false
            }
        );
        let mut o = exited(0);
        o.truncated = true;
        assert_eq!(
            verdict(&o),
            Verdict {
                exit_code: None,
                timed_out: false,
                file_too_large: false
            }
        );
    }

    #[test]
    fn output_cap_is_combined_and_exact() {
        let mut c = OutputCap::new(10);
        c.push(Stream::Out, b"12345");
        c.push(Stream::Err, b"abc");
        assert!(!c.truncated);
        c.push(Stream::Out, b"6789");
        assert!(c.truncated);
        assert_eq!(
            (c.stdout.as_slice(), c.stderr.as_slice()),
            (&b"1234567"[..], &b"abc"[..])
        );
        let mut c = OutputCap::new(3);
        c.push(Stream::Out, b"abc");
        assert!(!c.truncated, "exactly at the cap is not truncation");
    }

    #[test]
    fn args_are_the_pinned_list() {
        let sb = Sandbox {
            tmp_bytes: RUN_TMP_BYTES,
            go_root: Path::new("/g"),
            workdir: Path::new("/w"),
            cache_dir: Path::new("/c"),
            network: false,
        };
        let got: Vec<String> = sb
            .args()
            .into_iter()
            .map(|a| a.into_string().unwrap())
            .collect();
        let want = "--unshare-all --unshare-user --disable-userns --die-with-parent --new-session --clearenv --ro-bind /g /opt/go --bind /w /work --bind /c /cache --proc /proc --dev /dev --size 67108864 --tmpfs /tmp --remount-ro / --remount-ro /dev --chdir /work --setenv GOROOT /opt/go --setenv GOCACHE /cache/build --setenv GOPATH /cache/gopath --setenv GOTOOLCHAIN local --setenv GOFLAGS -mod=mod --setenv CGO_ENABLED 0 --setenv HOME /tmp --setenv PATH /opt/go/bin";
        assert_eq!(got.join(" "), want);
    }
}
