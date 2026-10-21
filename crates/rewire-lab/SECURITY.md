# rewire-lab: threat model

On desktop, a learner's Go code is compiled and run by the real Go toolchain
on their own machine. We treat that code as untrusted. The sandbox is the only
thing between it and the user's files, network and system.

## How it runs

Each run gets a fresh temporary work directory, which is removed afterwards.
Removal works even if learner code locked directories with `chmod 000`: before
deleting, the runner gives every directory back owner access. It opens each
one by file descriptor and never follows a symlink.
The code runs in two phases, and each phase is its own bubblewrap
(`bwrap`) sandbox:

1. Compile: `go build -o /work/prog .`. Only the Go toolchain runs here. Learner
   code does not execute.
2. Run: `/work/prog`, under the learner limits.

Inside the sandbox the root is empty and read-only. It holds only these:
the Go toolchain (read-only at `/opt/go`), the work directory (`/work`), a
private `/proc`, a minimal read-only `/dev`, and a private `/tmp`. The build
cache (`/cache`) is mounted **only in the compile phase**. The learner's
program never sees it. The exact argument list is in `src/sandbox.rs`.

## What is contained

Each item has a test in `tests/go_native.rs`.

- **Network**: there is none. The sandbox has its own empty network namespace.
- **Home directory and host files**: they are not visible. The host's `/home`,
  `/usr`, `/etc` and the rest are never mounted.
- **Host processes**: they are not visible. The sandbox has its own process
  namespace, so `/proc` shows only bwrap's init and the program.
- **Environment**: no host variables leak in. The environment is cleared and
  then set to a fixed list.
- **Writes outside `/work` and `/tmp`**: they fail. The root and `/dev` are
  read-only, `/opt/go` is a read-only bind, and `/cache` doesn't exist while
  the program runs.
- **The build cache**: only the Go toolchain writes it, in the compile phase.
  Before each run, the host checks that `build/` and `gopath/` are real
  directories. If one isn't (for example a planted symlink), it is removed
  without being followed, and then recreated.
- **File size**: each file the program writes is capped at 10 MiB
  (`RLIMIT_FSIZE`). The whole of `/tmp` is capped at 64 MiB.
- **CPU and wall time**: the program gets 10 s of CPU and 10 s of wall time.
  The CPU limit (`RLIMIT_CPU`) has its soft and hard values equal, so the
  kernel kills the program at exactly 10 s of CPU. When time runs out, the whole process group is killed. The
  sandbox dies with its parent, and the process namespace takes any leftover
  processes with it.
- **Output**: stdout and stderr together are capped at 64 KiB. Past the cap,
  the program is killed.
- **Nested user namespaces**: they are blocked (`--disable-userns`). This
  narrows the kernel attack surface.
- **File names**: only plain names like `main.go` are accepted, so writing the
  files can't escape the work directory.

## Compile-phase limits

The compile phase gets wider limits than the program, because only the trusted
toolchain runs in it:

- the file size cap is 512 MiB;
- CPU time is the larger of the CPU limit and the compile wall limit (60 s);
- `/tmp` is capped at 512 MiB.

A cold build of the Go runtime writes a file of about 13 MB. A cold build of
bigger packages such as `net/http` needs about 100 MB of `/tmp`.

## Known gaps

- **No memory limit.** A program can allocate memory until the machine runs
  out, within its 10 s.
- **No process-count limit.** A program can start copies of itself (a fork
  bomb) within its 10 s.
- **No seccomp filter.** Every system call the kernel allows to an
  unprivileged user is reachable.
- **`/cache` persists across runs, but only the toolchain writes it.** Learner
  code can't reach the cache. The remaining risk is the compile phase itself:
  a compiler bug triggered by crafted source could write bad cache entries.
  Anything built from them still runs only inside the sandbox. Clearing the
  cache fixes it.
- **Disk use in `/work`.** Each file is capped at 10 MiB, but nothing caps the
  number of files. A program can write many of them during its 10 s. `/work`
  lives in the host temp dir, which is RAM on some systems. It is removed after
  the run, but until then it counts against the host. A directory nested deeper
  than the open-file limit may not be fully removed; the runner logs this.
- **Go ignores SIGXFSZ.** A Go program that hits the file size limit gets a
  "file too large" write error instead of being killed. If the code ignores
  that error, the run looks normal.

## Likely next step

Where it is available, wrap bwrap in
`systemd-run --user --scope -p MemoryMax=... -p TasksMax=...` to get memory and
process-count limits from cgroups, and add a seccomp profile to bwrap.

## Toolchain download

`toolchain::install` fetches the pinned Go tarball from go.dev over TLS, checks
its SHA-256 against the official listing, and only then extracts it. Extraction
refuses absolute paths, `..`, links with absolute or `..` targets and unusual
entry types, and installs by atomic rename. A failed install leaves nothing
behind except `toolchains/`.
