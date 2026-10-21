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

test-web:
    pnpm -r test

# Build every wasm output into apps/web/public/ (git-ignored build outputs).
# Later tasks add their own wasm-* recipe here (Task 4 lab.wasm, Task 12 core-wasm).
wasm: wasm-world

# Bevy street -> apps/web/public/world/ (rewire_world.js + rewire_world_bg.wasm).
wasm-world:
    CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}" cargo build -p rewire-world --target wasm32-unknown-unknown --release
    wasm-bindgen --target web --out-dir apps/web/public/world target/wasm32-unknown-unknown/release/rewire_world.wasm

# Validate content packs and emit the bundle.
content:
    @echo "content: not built yet (Task 5)"

dev-web:
    @echo "dev-web: not built yet (Task 13)"

dev-desktop:
    @echo "dev-desktop: not built yet (Task 15)"

bundle:
    @echo "bundle: not built yet (Task 15)"
