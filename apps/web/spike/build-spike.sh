#!/usr/bin/env bash
# Compile the real src/lab runner and worker to plain JS for the spike page (no Vite yet).
set -euo pipefail
cd "$(dirname "$0")/.."
rm -rf spike/.build
npx tsc src/lab/webRunner.ts src/lab/worker.ts --allowJs --outDir spike/.build --rootDir src/lab \
  --target es2022 --module esnext --moduleResolution bundler --lib es2022,dom,webworker --skipLibCheck --types node
cp src/lab/labHost.js spike/.build/labHost.js  # plain JS: tsc only sees its .d.ts
