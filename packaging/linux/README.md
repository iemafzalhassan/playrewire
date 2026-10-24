# Linux packaging

`just bundle` builds every Linux package from `apps/desktop/src-tauri/tauri.conf.json`:

| Package | Path | For |
|---|---|---|
| deb | `target/release/bundle/deb/Rewire_<ver>_amd64.deb` | Ubuntu, Debian and their derivatives |
| rpm | `target/release/bundle/rpm/Rewire-<ver>-1.x86_64.rpm` | Fedora, openSUSE and other rpm distros |
| AppImage | `target/release/bundle/appimage/Rewire_<ver>_amd64.AppImage` | any recent distro, no install |
| web | `target/release/bundle/web/rewire-web-<ver>.tar.gz` | self-hosting the web app |

The release workflow (`.github/workflows/release.yml`) runs the same `just bundle` on `ubuntu-24.04`.

## Files here

- `apparmor/dev.playrewire.rewire`: the AppArmor profile for Rewire's sandbox helper.
- `deb/postinst`: sets up the helper, the apt hook and the profile, but only where they're needed.
- `deb/prerm`: on removal, unloads the profile and deletes what `postinst` made.
- `deb/refresh-bwrap`: installed as `/usr/lib/rewire/refresh-bwrap`. It makes, refreshes or removes the helper copy.
- `deb/50rewire`: the apt hook. `postinst` copies it to `/etc/apt/apt.conf.d/50rewire`.

## Why there is an AppArmor profile

Rewire runs a learner's Go code inside bubblewrap (`bwrap`). bwrap needs an unprivileged user namespace to build that sandbox. Ubuntu 24.04 and later block unprivileged user namespaces through AppArmor (`kernel.apparmor_restrict_unprivileged_userns=1`). They allow them only for programs whose AppArmor profile says `userns`. Without a profile, the sandbox can't start, and Rewire shows its "Linux sandbox setup" message instead of running code.

Other distros don't have this restriction, so on them the deb sets up nothing and Rewire uses the system bwrap.

## What the deb does

Everything happens in the maintainer scripts. Every step can run more than once, and a failed step prints a warning but never fails `dpkg`.

1. **It depends on `bubblewrap (>= 0.9)`.** That's the first version with `--size` (0.8 added `--disable-userns`). The rpm depends on plain `bubblewrap`, because Tauri's rpm bundler can't express a version. Rewire's sandbox probe reports an older bwrap with the friendly setup message.
2. **It ships** the profile at `/usr/share/rewire/apparmor/dev.playrewire.rewire`, the hook at `/usr/share/rewire/apt/50rewire`, and the script `/usr/lib/rewire/refresh-bwrap`.
3. **`postinst` checks for `/proc/sys/kernel/apparmor_restrict_unprivileged_userns`.** If the kernel doesn't have it, `postinst` removes any copy, hook or profile left from before, and stops. Rewire then uses the system bwrap.
4. **Where it exists, `refresh-bwrap` copies `/usr/bin/bwrap` to `/usr/lib/rewire/bwrap`.** This private copy is Rewire's sandbox helper. rewire-lab uses `/usr/lib/rewire/bwrap` when it exists, and otherwise the first `bwrap` on `PATH`.
5. **`postinst` installs the apt hook** `/etc/apt/apt.conf.d/50rewire` (`DPkg::Post-Invoke`). After every dpkg run it calls `refresh-bwrap`, which re-copies when `cmp -s` shows the copy differs from `/usr/bin/bwrap`. A bubblewrap security update therefore reaches the copy in the same `apt` run.
6. **`postinst` installs the profile** at `/etc/apparmor.d/dev.playrewire.rewire`, but only if this system's `apparmor_parser -Q -K` accepts it (parse and compile, nothing loaded). AppArmor 3 can't read an AppArmor 4 profile. There, `postinst` removes any old profile file and prints a warning. When AppArmor is running, it loads the profile with `apparmor_parser -r`. AppArmor also loads it again at every boot.
7. **`prerm` cleans up on `remove`.** It removes the hook, unloads and deletes the profile, and deletes the copy. dpkg then removes the packaged files and `/usr/lib/rewire`. On an upgrade, `prerm` does nothing, and the new `postinst` refreshes everything.

If the kernel gains the restriction later (for example after a release upgrade to Ubuntu 24.04), run `sudo dpkg-reconfigure rewire`.

## The profile

It is based on the upstream AppArmor 4.0 profile `bwrap-userns-restrict` (`profiles/apparmor/profiles/extras/bwrap-userns-restrict` on the `apparmor-4.0` branch of gitlab.com/apparmor/apparmor). Only the profile names, the attachment path and the local include names changed. It has two parts:

- **`dev.playrewire.rewire`** attaches to `/usr/lib/rewire/bwrap`. It allows `userns`, and capabilities inside the new namespace, because bwrap needs them to build its mounts.
- **`dev.playrewire.rewire-unpriv`** is stacked onto everything bwrap executes (`px /** -> dev.playrewire.rewire//&dev.playrewire.rewire-unpriv`). It has `audit deny capability`, so nothing bwrap runs gets a capability. That includes the Go toolchain, the learner's program, and a shell if someone points the helper at one. Rewire's payloads (`go build`, `/work/prog`) need none.

The stacking is needed because bwrap sets no-new-privs, which forbids a plain profile change on exec. Both parts are written for AppArmor 4 (Ubuntu 24.04 ships 4.0.1). To change them on one machine, put rules in `/etc/apparmor.d/local/dev.playrewire.rewire` or `.../local/dev.playrewire.rewire-unpriv`, but read the next section first. Upgrades overwrite the main file.

## The trade-off

Ubuntu restricts unprivileged user namespaces because they expose more of the kernel to unprivileged code, and many kernel exploits start from one. Any profile that grants `userns` hands something to **anyone who can run the program it attaches to**. That includes you, every other user on the machine, and any program running as them.

- **Anyone can run `/usr/lib/rewire/bwrap`.** Through it, any local user or program can create a user namespace. The child profile makes sure that whatever runs inside has **no capabilities**. That is the same limit upstream AppArmor puts on bwrap with `bwrap-userns-restrict`. What stays exposed is bwrap's own setup code, which runs with capabilities inside the namespace, plus the namespace-related kernel paths that need no capability. Installing Rewire on Ubuntu 24.04+ still weakens the restriction on that machine, by one well-known path. It just doesn't hand out a capable root-in-namespace shell.
- **Why a private copy and not a profile on `/usr/bin/bwrap`:**
  - It doesn't change how the system's `/usr/bin/bwrap` behaves for other software.
  - It can't clash with a distro or admin profile for `/usr/bin/bwrap`. Two profiles that attach to the same path conflict.
  - It goes away cleanly when Rewire is removed.
- **The copy follows bubblewrap updates** through the apt hook. The hook runs after every dpkg run that apt starts, including `apt`, `apt-get` and unattended-upgrades. A bubblewrap package installed with plain `dpkg -i` doesn't trigger it; run `sudo dpkg-reconfigure rewire` after one.
- **Inside the sandbox, nested user namespaces are still off** (`bwrap --disable-userns`).

If you don't want this trade-off, remove Rewire, or delete the profile and the copy by hand (the commands are below). Rewire then shows its sandbox setup message, and code doesn't run. The other way to make bwrap work is `sysctl kernel.apparmor_restrict_unprivileged_userns=0`. That turns the restriction off for every program, which is much broader.

## AppImage on Ubuntu 24.04+

An AppImage can't install anything, so it gets no profile. Install the `bubblewrap` package (0.9 or newer), then do by hand what `postinst` does. Save `apparmor/dev.playrewire.rewire` from this folder (or from the release's source code) first. There is no apt hook this way, so repeat the first command after every bubblewrap update:

```sh
sudo install -D -m 0755 /usr/bin/bwrap /usr/lib/rewire/bwrap
sudo install -m 0644 dev.playrewire.rewire /etc/apparmor.d/dev.playrewire.rewire
sudo apparmor_parser -r /etc/apparmor.d/dev.playrewire.rewire
```

To undo it:

```sh
sudo apparmor_parser -R /etc/apparmor.d/dev.playrewire.rewire
sudo rm -f /etc/apparmor.d/dev.playrewire.rewire /usr/lib/rewire/bwrap
sudo rmdir /usr/lib/rewire
```

## Smoke test

`rewire --smoke` boots the app, runs the first hello drill through the real sandbox, prints `SMOKE OK` and exits 0. On any failure it prints `SMOKE FAILED: <reason>` and exits 1. It never downloads the Go toolchain. Point `REWIRE_GO_ROOT` at a Go root if none is installed in the app's data dir. Headless, wrap it in `xvfb-run`:

```sh
REWIRE_GO_ROOT=/opt/go xvfb-run -a rewire --smoke
```

The release workflow installs the deb on a stock `ubuntu-24.04` runner and runs this, then runs the AppImage the same way. It first checks that the restriction is on (`1`), and that `aa-status` lists `dev.playrewire.rewire` after the install, so a pass can't come from a machine where the profile doesn't matter.

An AppImage carries most of its libraries, so it only runs on distros with the same or a newer glibc than the machine that built it. Build release AppImages on the oldest distro you support. The release workflow uses `ubuntu-24.04`. An AppImage built on a rolling distro like Arch won't start on Ubuntu 24.04 or Fedora 44.

### In containers

bwrap needs to create namespaces inside the container. Docker's defaults block that: the seccomp profile blocks `unshare`/`clone` with new namespaces, and the masked `/proc` paths stop bwrap from mounting a fresh `/proc`. The least privilege that worked, without `--privileged`, is:

```sh
docker run --rm \
  --security-opt seccomp=unconfined \
  --security-opt systempaths=unconfined \
  --security-opt apparmor=unconfined \
  -v "$(go env GOROOT):/opt/go:ro" -v "$PWD/target/release/bundle:/bundle:ro" \
  ubuntu:24.04 ...
```

`apparmor=unconfined` only matters on an AppArmor host. Docker's own AppArmor profile denies mounts. A container shares the host kernel, so the AppArmor user namespace restriction (and Rewire's profile) can't be tested inside one. The release workflow tests that part on the runner.
