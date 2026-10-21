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

# Build both wasm crates.
wasm:
    @echo "wasm: not built yet (Tasks 4 and 12)"

# Validate content packs and emit the bundle.
content:
    @echo "content: not built yet (Task 5)"

dev-web:
    @echo "dev-web: not built yet (Task 13)"

dev-desktop:
    @echo "dev-desktop: not built yet (Task 15)"

bundle:
    @echo "bundle: not built yet (Task 15)"
