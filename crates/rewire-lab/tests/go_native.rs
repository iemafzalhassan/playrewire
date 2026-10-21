//! End-to-end tests for the sandboxed native Go runner.
//!
//! Every test compiles and runs real Go code inside bubblewrap. A test skips
//! (with a printed reason) only when `sandbox_probe()` fails, e.g. on a host
//! where unprivileged user namespaces are disabled. CI turns them on (R7).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use rewire_lab::{
    GoNativeRuntime, LanguageRuntime, Limits, RawRunResult, SourceFile, sandbox_probe,
};

macro_rules! require_sandbox {
    () => {
        if let Err(e) = sandbox_probe() {
            eprintln!("SKIP: sandbox unavailable on this host: {e}");
            return;
        }
    };
}

fn go_root() -> PathBuf {
    let out = Command::new("go")
        .args(["env", "GOROOT"])
        .output()
        .expect("go on PATH");
    assert!(out.status.success(), "go env GOROOT failed");
    PathBuf::from(String::from_utf8(out.stdout).unwrap().trim())
}

/// Persistent cache shared by most tests, so only the first build is cold.
fn shared_cache() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("rewire-lab-cache")
}

fn runtime_with(cache_dir: &Path) -> GoNativeRuntime {
    GoNativeRuntime {
        go_root: go_root(),
        cache_dir: cache_dir.to_path_buf(),
    }
}

fn main_go(source: &str) -> Vec<SourceFile> {
    vec![SourceFile {
        name: "main.go".into(),
        source: source.into(),
    }]
}

/// Warms the shared cache once (one cold stdlib build) before tests build in
/// parallel; a dozen concurrent cold builds can exceed the 60 s compile wall
/// on a small CI runner.
async fn warm_shared_cache() {
    static WARM: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();
    WARM.get_or_init(|| async {
        let src = "package main\n\nimport (\n\t\"fmt\"\n\t\"net\"\n\t\"os\"\n\t\"sort\"\n\t\"strconv\"\n\t\"strings\"\n\t\"time\"\n)\n\nvar _ = net.Dial\nvar _ = os.Exit\nvar _ = sort.Strings\nvar _ = strconv.Itoa\nvar _ = strings.Join\nvar _ = time.Second\n\nfunc main() { fmt.Println(\"warm\") }\n";
        let r = runtime_with(&shared_cache()).run(main_go(src), Limits::default()).await.expect("warm-up run");
        assert_eq!(r.stdout, "warm\n", "warm-up failed: {r:?}");
    })
    .await;
}

async fn run(source: &str, limits: Limits) -> RawRunResult {
    warm_shared_cache().await;
    runtime_with(&shared_cache())
        .run(main_go(source), limits)
        .await
        .expect("run returns a result")
}

fn host_home() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").expect("HOME set in the test process"))
}

#[tokio::test]
async fn hello_runs() {
    require_sandbox!();
    let r = run(
        "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"hello, city\")\n}\n",
        Limits::default(),
    )
    .await;
    assert_eq!(r.stdout, "hello, city\n", "stderr: {}", r.stderr);
    assert_eq!(r.exit_code, Some(0));
    assert!(!r.timed_out && !r.truncated);
}

#[tokio::test]
async fn compile_error_returns_stderr_with_position() {
    require_sandbox!();
    // Line 9, column 2 is the undefined identifier.
    let src = "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"a\")\n\tfmt.Println(\"b\")\n\tfmt.Println(\"c\")\n\tundefinedThing()\n}\n";
    let r = run(src, Limits::default()).await;
    assert!(
        matches!(r.exit_code, Some(n) if n != 0),
        "exit {:?}",
        r.exit_code
    );
    assert!(r.stderr.contains("./main.go:9:2:"), "stderr: {}", r.stderr);
    assert!(
        r.stderr.contains("undefined: undefinedThing"),
        "stderr: {}",
        r.stderr
    );
    assert_eq!(r.stdout, "", "phase 2 must not run");
    assert_eq!(r.duration_ms, 0, "duration covers the run phase only");
    assert!(!r.timed_out);
}

#[tokio::test]
async fn infinite_loop_times_out_at_run_wall() {
    require_sandbox!();
    let limits = Limits {
        run_wall: Duration::from_secs(2),
        ..Limits::default()
    };
    let t = Instant::now();
    let r = run("package main\n\nfunc main() {\n\tfor {\n\t}\n}\n", limits).await;
    let total = t.elapsed();
    assert!(r.timed_out, "{r:?}");
    assert_eq!(r.exit_code, None);
    assert!(
        r.duration_ms >= 2000 && r.duration_ms < 4000,
        "run phase took {} ms",
        r.duration_ms
    );
    // The whole call also includes a (warm-cache) compile; keep a loose bound.
    assert!(total < Duration::from_secs(30), "whole call took {total:?}");
}

#[tokio::test]
async fn cpu_limit_times_out() {
    require_sandbox!();
    // Four spinning goroutines burn CPU faster than wall time, so RLIMIT_CPU
    // (1 s soft, 2 s hard) fires well before the 10 s wall limit.
    let limits = Limits {
        cpu_secs: 1,
        ..Limits::default()
    };
    let src = "package main\n\nfunc spin() {\n\tfor {\n\t}\n}\n\nfunc main() {\n\tfor i := 0; i < 3; i++ {\n\t\tgo spin()\n\t}\n\tspin()\n}\n";
    let r = run(src, limits).await;
    assert!(r.timed_out, "{r:?}");
    assert_eq!(r.exit_code, None);
    assert!(
        r.duration_ms < 8000,
        "RLIMIT_CPU should fire before the wall: {} ms",
        r.duration_ms
    );
}

#[tokio::test]
async fn endless_print_truncated() {
    require_sandbox!();
    let limits = Limits::default();
    let t = Instant::now();
    let r = run(
        "package main\n\nimport (\n\t\"fmt\"\n\t\"os\"\n)\n\nfunc main() {\n\tfor {\n\t\tfmt.Println(\"spam spam spam\")\n\t\tfmt.Fprintln(os.Stderr, \"err err\")\n\t}\n}\n",
        limits.clone(),
    )
    .await;
    assert!(r.truncated, "{:?}", (r.exit_code, r.timed_out));
    assert!(!r.timed_out);
    assert!(
        r.stdout.len() + r.stderr.len() <= limits.max_output,
        "{} bytes",
        r.stdout.len() + r.stderr.len()
    );
    assert!(r.stdout.starts_with("spam spam spam\n"));
    assert!(
        r.duration_ms < 5000,
        "kill on cap, not at the wall: {} ms",
        r.duration_ms
    );
    assert!(t.elapsed() < Duration::from_secs(30));
}

#[tokio::test]
async fn network_is_denied() {
    require_sandbox!();
    let r = run(
        "package main\n\nimport (\n\t\"fmt\"\n\t\"net\"\n\t\"time\"\n)\n\nfunc main() {\n\tc, err := net.DialTimeout(\"tcp\", \"1.1.1.1:80\", 3*time.Second)\n\tif err != nil {\n\t\tfmt.Println(\"DENIED:\", err)\n\t\treturn\n\t}\n\tc.Close()\n\tfmt.Println(\"CONNECTED\")\n}\n",
        Limits::default(),
    )
    .await;
    assert!(
        r.stdout.starts_with("DENIED:"),
        "stdout: {} stderr: {}",
        r.stdout,
        r.stderr
    );
    assert!(!r.stdout.contains("CONNECTED"));
}

#[tokio::test]
async fn cannot_read_home() {
    require_sandbox!();
    let home = host_home();
    // Precondition: the host process *can* list the real home directory, so
    // the sandbox failing to is meaningful.
    assert!(
        std::fs::read_dir(&home).is_ok(),
        "host cannot read {home:?}"
    );
    let home = home.to_str().unwrap();
    let src = format!(
        "package main\n\nimport (\n\t\"fmt\"\n\t\"os\"\n)\n\nfunc main() {{\n\thome := {home:?}\n\tif _, err := os.ReadFile(home + \"/.bashrc\"); err == nil {{\n\t\tfmt.Println(\"READ bashrc\")\n\t}}\n\tif _, err := os.ReadDir(home); err == nil {{\n\t\tfmt.Println(\"LISTED home\")\n\t}}\n\tfmt.Println(\"REAL_HOME=\" + os.Getenv(\"REAL_HOME\"))\n\tfmt.Println(\"done\")\n}}\n"
    );
    let r = run(&src, Limits::default()).await;
    assert_eq!(r.stdout, "REAL_HOME=\ndone\n", "stderr: {}", r.stderr);
}

#[tokio::test]
async fn cannot_write_outside_work() {
    require_sandbox!();
    // A real host path that exists outside the sandbox's mounts.
    let host_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    assert!(host_dir.is_dir());
    let host_probe = host_dir.join(format!("rewire-lab-escape-{}", std::process::id()));
    let host_probe = host_probe.to_str().unwrap();
    let src = format!(
        "package main\n\nimport (\n\t\"fmt\"\n\t\"os\"\n)\n\nfunc main() {{\n\tfor _, p := range []string{{\"/escape\", \"/etc/escape\", \"/opt/go/escape\", \"/opt/go/bin/escape\", \"/proc/escape\", \"/sys/escape\", \"/dev/escape\", \"/usr/escape\", \"/home/escape\", {host_probe:?}}} {{\n\t\tif err := os.WriteFile(p, []byte(\"x\"), 0o644); err == nil {{\n\t\t\tfmt.Println(\"WROTE\", p)\n\t\t}}\n\t}}\n\tfor _, p := range []string{{\"/work/ok\", \"/tmp/ok\", \"/cache/ok\"}} {{\n\t\tif err := os.WriteFile(p, []byte(\"x\"), 0o644); err != nil {{\n\t\t\tfmt.Println(\"FAILED\", p, err)\n\t\t}}\n\t}}\n\tfmt.Println(\"done\")\n}}\n"
    );
    let r = run(&src, Limits::default()).await;
    assert_eq!(r.stdout, "done\n", "stderr: {}", r.stderr);
    assert!(!Path::new(host_probe).exists());
}

#[tokio::test]
async fn file_size_limit() {
    require_sandbox!();
    // 1 MiB limit, 2 MiB writes to every writable place.
    let limits = Limits {
        max_file_bytes: 1 << 20,
        ..Limits::default()
    };
    let src = "package main\n\nimport (\n\t\"fmt\"\n\t\"os\"\n)\n\nfunc main() {\n\tbuf := make([]byte, 2<<20)\n\tfor _, p := range []string{\"/work/big\", \"/tmp/big\", \"/cache/big\"} {\n\t\terr := os.WriteFile(p, buf, 0o644)\n\t\tst, _ := os.Stat(p)\n\t\tfmt.Println(p, err != nil, st.Size())\n\t\tos.Remove(p)\n\t}\n}\n";
    let r = run(src, limits).await;
    assert_eq!(
        r.stdout, "/work/big true 1048576\n/tmp/big true 1048576\n/cache/big true 1048576\n",
        "stderr: {}",
        r.stderr
    );
}

#[tokio::test]
async fn tmp_is_size_capped() {
    require_sandbox!();
    // /tmp is a 64 MiB tmpfs. Files stay under the 10 MiB RLIMIT_FSIZE so the
    // only thing that can stop them is the tmpfs size cap.
    let src = "package main\n\nimport (\n\t\"fmt\"\n\t\"os\"\n)\n\nfunc main() {\n\tbuf := make([]byte, 8<<20)\n\ttotal := 0\n\tfor i := 0; i < 12; i++ {\n\t\tf, err := os.Create(fmt.Sprintf(\"/tmp/f%d\", i))\n\t\tif err != nil {\n\t\t\tfmt.Println(\"STOPPED\", err)\n\t\t\tbreak\n\t\t}\n\t\tn, err := f.Write(buf)\n\t\tf.Close()\n\t\ttotal += n\n\t\tif err != nil {\n\t\t\tfmt.Println(\"STOPPED\", err)\n\t\t\tbreak\n\t\t}\n\t}\n\tfmt.Println(\"total\", total, total <= 64<<20)\n}\n";
    let r = run(src, Limits::default()).await;
    assert!(
        r.stdout.contains("STOPPED") && r.stdout.contains("no space left on device"),
        "stdout: {} stderr: {}",
        r.stdout,
        r.stderr
    );
    assert!(
        r.stdout.trim_end().ends_with("true"),
        "wrote more than 64 MiB: {}",
        r.stdout
    );
}

#[tokio::test]
async fn nested_userns_denied() {
    require_sandbox!();
    // Re-exec ourselves into a new user namespace. A multi-threaded Go
    // process can't unshare(CLONE_NEWUSER) in place (always EINVAL), so the
    // clone flag on a child is the real test of --disable-userns.
    let src = "package main\n\nimport (\n\t\"fmt\"\n\t\"os\"\n\t\"os/exec\"\n\t\"syscall\"\n)\n\nfunc main() {\n\tif len(os.Args) > 1 {\n\t\tfmt.Println(\"IN NESTED USERNS\")\n\t\treturn\n\t}\n\tcmd := exec.Command(\"/proc/self/exe\", \"child\")\n\tcmd.SysProcAttr = &syscall.SysProcAttr{Cloneflags: syscall.CLONE_NEWUSER}\n\tout, err := cmd.CombinedOutput()\n\tif err != nil {\n\t\tfmt.Println(\"DENIED:\", err)\n\t\treturn\n\t}\n\tfmt.Print(string(out))\n}\n";
    let r = run(src, Limits::default()).await;
    assert!(
        r.stdout.starts_with("DENIED:"),
        "stdout: {} stderr: {}",
        r.stdout,
        r.stderr
    );
    assert!(!r.stdout.contains("IN NESTED USERNS"));
}

#[tokio::test]
async fn no_usr_visible() {
    require_sandbox!();
    let src = "package main\n\nimport (\n\t\"fmt\"\n\t\"os\"\n)\n\nfunc main() {\n\tfor _, p := range []string{\"/usr\", \"/bin/sh\", \"/etc/passwd\", \"/home\", \"/root\", \"/var\", \"/run\", \"/sys\"} {\n\t\tif _, err := os.Stat(p); err == nil {\n\t\t\tfmt.Println(\"VISIBLE\", p)\n\t\t}\n\t}\n\tfmt.Println(\"done\")\n}\n";
    let r = run(src, Limits::default()).await;
    assert_eq!(r.stdout, "done\n", "stderr: {}", r.stderr);
}

#[tokio::test]
async fn cannot_see_host_processes() {
    require_sandbox!();
    // In a fresh pid namespace the only processes are bwrap's init (pid 1)
    // and the program itself. On the host, pid 1 is the system init.
    let src = "package main\n\nimport (\n\t\"fmt\"\n\t\"os\"\n\t\"strconv\"\n\t\"strings\"\n)\n\nfunc main() {\n\tents, _ := os.ReadDir(\"/proc\")\n\tvar pids []string\n\tfor _, e := range ents {\n\t\tif _, err := strconv.Atoi(e.Name()); err == nil {\n\t\t\tpids = append(pids, e.Name())\n\t\t}\n\t}\n\tcomm, _ := os.ReadFile(\"/proc/1/comm\")\n\tfmt.Printf(\"self=%d pids=%s init=%s\\n\", os.Getpid(), strings.Join(pids, \",\"), strings.TrimSpace(string(comm)))\n}\n";
    let r = run(src, Limits::default()).await;
    let host_init = std::fs::read_to_string("/proc/1/comm").unwrap_or_default();
    let line = r.stdout.trim();
    let self_pid: &str = line
        .split_whitespace()
        .next()
        .unwrap()
        .trim_start_matches("self=");
    assert_eq!(
        line,
        format!("self={self_pid} pids=1,{self_pid} init=bwrap"),
        "stderr: {}",
        r.stderr
    );
    assert_ne!(
        host_init.trim(),
        "bwrap",
        "host pid 1 should be the system init"
    );
    assert!(
        self_pid.parse::<u32>().unwrap() < 10,
        "sandbox pids start at 1"
    );
}

#[tokio::test]
async fn env_is_clean() {
    require_sandbox!();
    let src = "package main\n\nimport (\n\t\"fmt\"\n\t\"os\"\n\t\"sort\"\n)\n\nfunc main() {\n\tenv := os.Environ()\n\tsort.Strings(env)\n\tfor _, kv := range env {\n\t\tfmt.Println(kv)\n\t}\n}\n";
    let r = run(src, Limits::default()).await;
    let expected = [
        "CGO_ENABLED=0",
        "GOCACHE=/cache/build",
        "GOFLAGS=-mod=mod",
        "GOPATH=/cache/gopath",
        "GOROOT=/opt/go",
        "GOTOOLCHAIN=local",
        "HOME=/tmp",
        "PATH=/opt/go/bin",
        // bwrap sets PWD for --chdir; it names a sandbox path, not a host one.
        "PWD=/work",
    ];
    let got: Vec<&str> = r.stdout.lines().collect();
    assert_eq!(got, expected, "stderr: {}", r.stderr);
}

#[tokio::test]
async fn city_lines_present_in_stdout() {
    require_sandbox!();
    let city = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/go/city/city.go"
    ))
    .unwrap();
    let files = vec![
        SourceFile { name: "city.go".into(), source: city },
        SourceFile { name: "setup.go".into(), source: "package main\n\nfunc setup() {\n\tLamp(0, true)\n}\n".into() },
        SourceFile {
            name: "main.go".into(),
            source: "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tsetup()\n\tSetGreen(2, 3)\n\tSay(\"hi \\\"there\\\"\")\n\tfmt.Println(\"plain\")\n}\n".into(),
        },
    ];
    warm_shared_cache().await;
    let r = runtime_with(&shared_cache())
        .run(files, Limits::default())
        .await
        .unwrap();
    assert_eq!(
        r.stdout,
        "@city {\"op\":\"lamp\",\"i\":0,\"on\":true}\n@city {\"op\":\"green\",\"i\":2,\"delay\":3}\n@city {\"op\":\"say\",\"text\":\"hi \\\"there\\\"\"}\nplain\n",
        "stderr: {}",
        r.stderr
    );
    assert_eq!(r.exit_code, Some(0));
}

#[tokio::test]
async fn second_run_uses_cache_and_is_faster() {
    require_sandbox!();
    let cache = tempfile::tempdir().unwrap();
    let rt = runtime_with(cache.path());
    let src = "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"cached\")\n}\n";

    let t = Instant::now();
    let first = rt.run(main_go(src), Limits::default()).await.unwrap();
    let cold = t.elapsed();
    assert_eq!(first.stdout, "cached\n", "stderr: {}", first.stderr);
    assert!(
        cache
            .path()
            .join("build")
            .read_dir()
            .unwrap()
            .next()
            .is_some(),
        "GOCACHE is populated"
    );
    assert!(cache.path().join("gopath").is_dir());

    let t = Instant::now();
    let second = rt.run(main_go(src), Limits::default()).await.unwrap();
    let warm = t.elapsed();
    assert_eq!(second.stdout, "cached\n");
    assert!(
        warm < cold.mul_f64(0.8) || warm < Duration::from_secs(2),
        "cold {cold:?}, warm {warm:?}"
    );
}

#[tokio::test]
async fn rejects_unsafe_file_names() {
    // Name validation happens before anything touches the disk or bwrap.
    let rt = runtime_with(&shared_cache());
    for bad in [
        "../evil.go",
        "a/b.go",
        "/etc/x.go",
        ".go",
        "main.go.txt",
        "go.mod",
        "ma in.go",
        "main",
        "..",
        "x..go",
        "é.go",
    ] {
        let files = vec![SourceFile {
            name: bad.into(),
            source: "package main\n".into(),
        }];
        let err = rt.run(files, Limits::default()).await.expect_err(bad);
        assert!(
            matches!(err, rewire_lab::LabError::Io(ref e) if e.kind() == std::io::ErrorKind::InvalidInput),
            "{bad}: {err:?}"
        );
    }
    let dup = vec![
        SourceFile {
            name: "main.go".into(),
            source: String::new(),
        },
        SourceFile {
            name: "main.go".into(),
            source: String::new(),
        },
    ];
    assert!(rt.run(dup, Limits::default()).await.is_err());
}

#[tokio::test]
async fn missing_toolchain_is_reported() {
    require_sandbox!();
    let rt = GoNativeRuntime {
        go_root: PathBuf::from("/nonexistent/go"),
        cache_dir: shared_cache(),
    };
    let err = rt
        .run(main_go("package main\nfunc main() {}\n"), Limits::default())
        .await
        .unwrap_err();
    assert!(
        matches!(err, rewire_lab::LabError::ToolchainMissing),
        "{err:?}"
    );
}

#[test]
fn raw_run_result_serializes_to_ts_shape() {
    let r = RawRunResult {
        stdout: "o".into(),
        stderr: "e".into(),
        exit_code: None,
        timed_out: true,
        truncated: false,
        duration_ms: 12,
    };
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(
        v,
        serde_json::json!({"stdout":"o","stderr":"e","exitCode":null,"timedOut":true,"truncated":false,"durationMs":12})
    );
    let back: RawRunResult = serde_json::from_value(serde_json::json!({"stdout":"","stderr":"","exitCode":3,"timedOut":false,"truncated":true,"durationMs":0})).unwrap();
    assert_eq!(back.exit_code, Some(3));
    assert!(back.truncated);
}

#[test]
fn limits_default_matches_spec() {
    let l = Limits::default();
    assert_eq!(l.compile_wall, Duration::from_secs(60));
    assert_eq!(l.run_wall, Duration::from_secs(10));
    assert_eq!(l.cpu_secs, 10);
    assert_eq!(l.max_file_bytes, 10 * 1024 * 1024);
    assert_eq!(l.max_output, 65_536);
    assert!(!l.network);
}

#[test]
fn runtime_id_is_go() {
    assert_eq!(runtime_with(&shared_cache()).id(), "go");
}
