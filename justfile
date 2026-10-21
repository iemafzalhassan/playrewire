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
    pnpm -r test

# Build every wasm output into apps/web/public/ (git-ignored build outputs).
# Later tasks add their own wasm-* recipe here (Task 12 core-wasm).
wasm: wasm-world wasm-lab

# Bevy street -> apps/web/public/world/ (rewire_world.js + rewire_world_bg.wasm).
wasm-world:
    CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}" cargo build -p rewire-world --target wasm32-unknown-unknown --release
    wasm-bindgen --target web --out-dir apps/web/public/world target/wasm32-unknown-unknown/release/rewire_world.wasm

# Yaegi lab (Go -> js/wasm) -> apps/web/public/lab/ (lab.wasm + wasm_exec.js).
wasm-lab:
    mkdir -p apps/web/public/lab
    cd lab-wasm && GOOS=js GOARCH=wasm go build -trimpath -ldflags=-s -o ../apps/web/public/lab/lab.wasm ./cmd/lab
    cp "$(go env GOROOT)/lib/wasm/wasm_exec.js" apps/web/public/lab/wasm_exec.js

# Validate content packs and emit the bundle.
content:
    @echo "content: not built yet (Task 5)"

dev-web:
    @echo "dev-web: not built yet (Task 13)"

dev-desktop:
    @echo "dev-desktop: not built yet (Task 15)"

bundle:
    @echo "bundle: not built yet (Task 15)"
