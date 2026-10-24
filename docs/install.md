# Installing Rewire

Rewire runs on Linux (x86-64) for now, as a desktop app or in the browser. Windows, macOS and mobile come later.

Every release on the [Releases page](https://github.com/iemafzalhassan/playrewire/releases) has these files:

| File | What it is |
|---|---|
| `Rewire_<ver>_amd64.deb` | Ubuntu, Debian and their derivatives |
| `Rewire-<ver>-1.x86_64.rpm` | Fedora, openSUSE and other rpm distros |
| `Rewire_<ver>_amd64.AppImage` | any recent distro, nothing to install |
| `rewire-web-<ver>.tar.gz` | the web app, to host yourself |
| `rewire-<ver>.spdx.json` | the SBOM: every dependency, in SPDX JSON |
| `SHA256SUMS` | the checksum of every file above |
| `SHA256SUMS.cosign.bundle` | the signature over `SHA256SUMS` |

## 1. Check the download

The release workflow signs `SHA256SUMS` with [cosign](https://docs.sigstore.dev/cosign/) "keyless". There is no key to trust. The signature proves that this repo's release workflow, running for a version tag, made the file. Put the files in one folder, then run:

```sh
cosign verify-blob SHA256SUMS \
  --bundle SHA256SUMS.cosign.bundle \
  --certificate-identity-regexp '^https://github\.com/iemafzalhassan/playrewire/\.github/workflows/release\.yml@refs/tags/v[0-9]' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
sha256sum --ignore-missing -c SHA256SUMS
```

Both commands should end with `OK`. If either one fails, don't install the file.

## 2. Install

### deb (Ubuntu, Debian)

```sh
sudo apt install ./Rewire_<ver>_amd64.deb
```

`apt` also installs `bubblewrap`, the sandbox that runs your code. On Ubuntu 24.04 and later, the package also sets up the AppArmor profile that the sandbox needs (see [Linux sandbox setup](#linux-sandbox-setup)). To remove Rewire: `sudo apt remove rewire`.

### rpm (Fedora, openSUSE)

```sh
sudo dnf install ./Rewire-<ver>-1.x86_64.rpm      # Fedora
sudo zypper install ./Rewire-<ver>-1.x86_64.rpm   # openSUSE
```

This also installs `bubblewrap`. To remove Rewire: `sudo dnf remove rewire`.

### AppImage

Install `bubblewrap` with your package manager first. Then:

```sh
chmod +x Rewire_<ver>_amd64.AppImage
./Rewire_<ver>_amd64.AppImage
```

On Ubuntu 24.04 and later, also follow [Linux sandbox setup](#linux-sandbox-setup).

## 3. First launch

The first time you run code, Rewire offers to download the Go toolchain (Go 1.27.2, about 70 MB) from go.dev. It checks the download against a pinned SHA-256 and keeps it in `~/.local/share/dev.playrewire.rewire/`. After that, Rewire works offline.

Your code runs only on your computer, inside a sandbox. It has no network and can't see your files.

## Linux sandbox setup

If Rewire says it couldn't start its safe practice space, bubblewrap can't create its sandbox on your system. Check these in order:

1. **Is bubblewrap installed?** Run `bwrap --version`. You need 0.9 or newer.
2. **Ubuntu 24.04 and later:** Ubuntu blocks the "user namespaces" bubblewrap needs, unless an AppArmor profile allows them. The deb sets this up for you: it copies bwrap to `/usr/lib/rewire/bwrap` and loads the profile `dev.playrewire.rewire` for that copy. Check that it is loaded:

   ```sh
   sudo aa-status | grep dev.playrewire.rewire
   ```

   If it's missing, run `sudo dpkg-reconfigure rewire`. With the AppImage, follow the steps in [packaging/linux/README.md](../packaging/linux/README.md#appimage-on-ubuntu-2404).
3. **Other distros:** check that unprivileged user namespaces are on. `cat /proc/sys/user/max_user_namespaces` should print a number above 0.

**What the profile changes:** it lets anyone on the machine create a user namespace through `/usr/lib/rewire/bwrap`, which is the same thing Ubuntu's restriction tries to limit. [packaging/linux/README.md](../packaging/linux/README.md#the-trade-off) explains the trade-off in full. Removing Rewire removes the profile and the copy.

## Self-host the web app

The web app is a static site. All code runs in the visitor's browser, so you need nothing but a web server.

```sh
tar -xzf rewire-web-<ver>.tar.gz
# serves rewire-web-<ver>/ at http://localhost:8080/
python3 -m http.server -d rewire-web-<ver> 8080
```

For a real host:

- **Serve it at the root of a domain or subdomain** (for example `https://rewire.example.org/`). The build loads its files from `/`, so it doesn't work under a sub-path.
- **Serve `.wasm` files as `application/wasm`.** Most servers already do.
- **Keep the Content Security Policy.** It's in `index.html` as a `<meta>` tag. If your server also sends a `Content-Security-Policy` header, make it the same or stricter. The app needs `'wasm-unsafe-eval'` to run WebAssembly, and nothing else beyond `'self'`.
- **Cache with care.** Files under `assets/` have a content hash in their name and can be cached forever. Cache `index.html`, `content/`, `lab/` and `world/` for a short time, because their names stay the same between releases.

The web app makes no requests to any other server. Progress stays in the visitor's browser.

## Build it yourself

See [Getting started](../README.md#getting-started). `just bundle` builds every file listed above except the signature, which only the release workflow can make.
