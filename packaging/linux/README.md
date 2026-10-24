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
- `deb/postinst`: copies bwrap for Rewire, then installs and loads the profile.
- `deb/prerm`: on removal, unloads the profile and deletes what `postinst` made.

## Why there is an AppArmor profile

Rewire runs a learner's Go code inside bubblewrap (`bwrap`). bwrap needs an unprivileged user namespace to build that sandbox. Ubuntu 24.04 and later block unprivileged user namespaces through AppArmor (`kernel.apparmor_restrict_unprivileged_userns=1`). They allow them only for programs whose AppArmor profile says `userns`. Without a profile, the sandbox can't start, and Rewire shows its "Linux sandbox setup" message instead of running code.

Other distros don't have this restriction. On Fedora and Debian the profile isn't needed. Where AppArmor isn't running, `postinst` doesn't load it.

## What the deb does

Everything happens in the maintainer scripts. Every step can run more than once, and a failed step prints a warning but never fails `dpkg`.

1. **It depends on `bubblewrap`.** The rpm does too.
2. **It ships the profile** at `/usr/share/rewire/apparmor/dev.playrewire.rewire`.
3. **`postinst` copies `/usr/bin/bwrap` to `/usr/lib/rewire/bwrap`.** This private copy is Rewire's sandbox helper. rewire-lab uses `/usr/lib/rewire/bwrap` when it exists, and otherwise the first `bwrap` on `PATH`.
4. **`postinst` installs the profile** at `/etc/apparmor.d/dev.playrewire.rewire`. If `/etc/apparmor.d` exists, it then loads the profile with `apparmor_parser -r` whenever AppArmor is running. AppArmor also loads it again at every boot.
5. **`prerm` cleans up on `remove`.** It unloads the profile, then deletes the profile, the bwrap copy and `/usr/lib/rewire`. On an upgrade it does nothing, and the new `postinst` refreshes both.

The profile is written for AppArmor 4 (Ubuntu 24.04 ships 4.0.1). It has the same shape as Ubuntu's own profiles for `flatpak` and `lxc-usernsexec`:

```
abi <abi/4.0>,
include <tunables/global>

profile dev.playrewire.rewire /usr/lib/rewire/bwrap flags=(unconfined) {
  userns,
  include if exists <local/dev.playrewire.rewire>
}
```

To change it on one machine, put your rules in `/etc/apparmor.d/local/dev.playrewire.rewire`. Upgrades overwrite the main file.

## The trade-off

Ubuntu restricts unprivileged user namespaces because they expose more of the kernel to unprivileged code, and many kernel exploits start from one. Any profile that grants `userns` hands that ability to **anyone who can run the program it attaches to**. That includes you, every other user on the machine, and any program running as them.

- **A profile on `/usr/bin/bwrap`** would let every bwrap user on the machine create user namespaces, not just Rewire. Since anyone can run bwrap, anything on the machine could use it to get around the restriction.
- **Rewire's profile attaches to `/usr/lib/rewire/bwrap` instead.** Be clear about what that buys. Anyone can run that path too, so a program that knows about it can still use it to create a user namespace. Installing Rewire on Ubuntu 24.04+ therefore weakens the user namespace restriction on that machine by one well-known path. The private path has three real benefits:
  - It doesn't change how the system's `/usr/bin/bwrap` behaves for other software.
  - It can't clash with a distro or admin profile for `/usr/bin/bwrap`. Two profiles that attach to the same path conflict.
  - It goes away cleanly when Rewire is removed.
- **The copy can go stale.** It is refreshed only when Rewire's package is configured: on install, on upgrade, or with `sudo dpkg-reconfigure rewire`. A bubblewrap security update reaches the copy only at the next of these. Run `sudo dpkg-reconfigure rewire` after a bubblewrap update.
- **Inside the sandbox, nested user namespaces are still off** (`bwrap --disable-userns`). Learner code can't create one.

If you don't want this trade-off, remove Rewire, or delete the profile and the copy by hand (the commands are below). Rewire then shows its sandbox setup message, and code doesn't run. The other way to make bwrap work is `sysctl kernel.apparmor_restrict_unprivileged_userns=0`. That turns the restriction off for every program, which is broader still.

## AppImage on Ubuntu 24.04+

An AppImage can't install anything, so it gets no profile. Install the `bubblewrap` package, then do by hand what `postinst` does. Save `apparmor/dev.playrewire.rewire` from this folder (or from the release's source code) first:

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

The release workflow installs the deb on a stock `ubuntu-24.04` runner and runs this, then runs the AppImage the same way. The runner keeps the user namespace restriction on, so that run checks the AppArmor profile for real.

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
