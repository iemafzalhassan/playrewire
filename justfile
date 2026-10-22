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

# Build the web prerequisites that are missing (lab.wasm, core wasm, lesson bundle).
web-prereqs:
    #!/usr/bin/env bash
    set -euo pipefail
    [ -f apps/web/public/lab/lab.wasm ] && [ -f apps/web/public/lab/wasm_exec.js ] || just wasm-lab
    [ -f apps/web/src/core/pkg/rewire_core_wasm_bg.wasm ] || just wasm-core
    [ -f apps/web/public/content/go.json ] || just content

# Vite dev server for the web app (http://localhost:5173).
dev-web: web-prereqs
    cd apps/web && pnpm dev

# `just e2e-web --shots` also refreshes the review screenshots in docs/design/.
# Production-build checks in Chromium: worker + wasm under the CSP, Home and Map.
e2e-web *args: web-prereqs
    cd apps/web && pnpm e2e {{args}}

dev-desktop:
    @echo "dev-desktop: not built yet (Task 15)"

# Static web build -> apps/web/dist (desktop packaging comes with Task 15/18).
bundle: web-prereqs
    cd apps/web && pnpm build
