# PlayRewire: one entry point for every task.

default:
    @just --list

# Run every test suite: Rust, Go (each go.work module), and pnpm.
test: test-rust test-go test-web

test-rust:
    cargo test --workspace

# Runs `go test ./...` in every module listed in go.work.
test-go:
    #!/usr/bin/env bash
    set -euo pipefail
    mods=$(awk '/^use[[:space:]]*\(/{b=1;next} b&&/^\)/{b=0} b{print $1;next} /^use[[:space:]]/{print $2}' go.work)
    if [ -z "$mods" ]; then
        echo "go: no packages (go.work has no modules yet)"
        exit 0
    fi
    for m in $mods; do
        echo "go test ./... in $m"
        (cd "$m" && go test ./...)
    done

# Builds lab.wasm first when it is missing so a clean checkout passes.
test-web:
    #!/usr/bin/env bash
    set -euo pipefail
    [ -f apps/web/public/lab/lab.wasm ] && [ -f apps/web/public/lab/wasm_exec.js ] || just wasm-lab
    [ -f apps/web/src/core/pkg/rewire_core_wasm_bg.wasm ] || just wasm-core
    pnpm -r test

# Build every wasm output into apps/web/public/ (git-ignored build outputs).
wasm: wasm-world wasm-lab wasm-core

# Bevy street -> apps/web/public/world/ (rewire_world.js + rewire_world_bg.wasm).
wasm-world:
    @test "$(wasm-bindgen --version)" = "wasm-bindgen $(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"/\1/p')" || { echo "wasm-bindgen CLI $(wasm-bindgen --version) does not match the crate in Cargo.lock"; exit 1; }
    CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}" cargo build -p rewire-world --target wasm32-unknown-unknown --release
    wasm-bindgen --target web --out-dir apps/web/public/world target/wasm32-unknown-unknown/release/rewire_world.wasm

# Yaegi lab (Go -> js/wasm) -> apps/web/public/lab/ (lab.wasm + wasm_exec.js).
wasm-lab:
    mkdir -p apps/web/public/lab
    cd lab-wasm && GOOS=js GOARCH=wasm go build -trimpath -ldflags=-s -o ../apps/web/public/lab/lab.wasm ./cmd/lab
    cp "$(go env GOROOT)/lib/wasm/wasm_exec.js" apps/web/public/lab/wasm_exec.js

# rewire-core rules -> apps/web/src/core/pkg/ (git-ignored; imported by src/core/index.ts).
wasm-core:
    cargo build -p rewire-core-wasm --target wasm32-unknown-unknown --release
    wasm-bindgen --target web --out-dir apps/web/src/core/pkg target/wasm32-unknown-unknown/release/rewire_core_wasm.wasm

# Validate content packs (running every solution), then emit the bundle.
content:
    go run ./tools/packcheck validate content/go
    go run ./tools/packcheck build content/go -o apps/web/public/content/go.json

# Regenerate tools/packcheck/testdata/good.bundle.json (read by the Vitest schema test).
packcheck-fixture:
    cd tools/packcheck && UPDATE_BUNDLE=1 go test -count=1 -run TestGoodBundleIsNotStale ./...

# Build the web prerequisites that are missing (lab.wasm, core wasm, lesson bundle, street).
web-prereqs:
    #!/usr/bin/env bash
    set -euo pipefail
    [ -f apps/web/public/lab/lab.wasm ] && [ -f apps/web/public/lab/wasm_exec.js ] || just wasm-lab
    [ -f apps/web/src/core/pkg/rewire_core_wasm_bg.wasm ] || just wasm-core
    [ -f apps/web/public/content/go.json ] || just content
    [ -f apps/web/public/world/rewire_world_bg.wasm ] || just wasm-world

# Vite dev server for the web app (http://localhost:5173).
dev-web: web-prereqs
    cd apps/web && pnpm dev

# `just e2e-web --shots` also refreshes the review screenshots in docs/design/.
# Production-build checks in Chromium: worker + wasm under the CSP, Home and Map.
e2e-web *args: web-prereqs
    cd apps/web && pnpm e2e {{args}}

# Tauri desktop app with the Vite dev server (tauri.conf.json starts `pnpm --filter web dev`).
dev-desktop: web-prereqs
    cd apps/desktop/src-tauri && CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}" cargo tauri dev

# Release desktop binary without installers -> target/release/rewire.
build-desktop: web-prereqs
    cd apps/desktop/src-tauri && CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}" cargo tauri build --no-bundle

# Release build, in order: every wasm, the lesson bundle, the web app, the
# Linux packages (target/release/bundle/{deb,rpm,appimage}/) and the web tarball.
bundle:
    just wasm
    just content
    cd apps/web && pnpm build
    cd apps/desktop/src-tauri && CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}" cargo tauri build --bundles deb,rpm,appimage
    just web-tarball

# Checks apps/web/dist, then packs it as target/release/bundle/web/rewire-web-<ver>.tar.gz.
# Reproducible: sorted names, fixed owner, mtime from the last commit, gzip -n.
web-tarball:
    #!/usr/bin/env bash
    set -euo pipefail
    ver=$(sed -n 's/^  "version": "\(.*\)",$/\1/p' apps/desktop/src-tauri/tauri.conf.json)
    [ -n "$ver" ] || { echo "no version in tauri.conf.json"; exit 1; }
    dist=apps/web/dist
    for f in index.html content/go.json lab/lab.wasm lab/wasm_exec.js; do
        [ -f "$dist/$f" ] || { echo "web build is missing $f"; exit 1; }
    done
    compgen -G "$dist/world/*.wasm" >/dev/null || { echo "web build is missing world/*.wasm"; exit 1; }
    compgen -G "$dist/assets/rewire_core_wasm_bg-*.wasm" >/dev/null || { echo "web build is missing the core wasm"; exit 1; }
    grep -q 'http-equiv="Content-Security-Policy"' "$dist/index.html" || { echo "index.html lost its CSP"; exit 1; }
    out=target/release/bundle/web
    mkdir -p "$out"
    name="rewire-web-$ver"
    epoch="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct)}"
    tar --sort=name --owner=0 --group=0 --numeric-owner --mtime="@$epoch" \
        --transform "s,^\.,$name," -C "$dist" -cf - . | gzip -n -9 > "$out/$name.tar.gz"
    echo "$out/$name.tar.gz"
