# Spike: Yaegi (Go interpreter) in the browser via WASM

Date: 2026-10-21. Task 4 of the MVP plan. Verdict: **PASS**.

## Question

Can learner Go code run in a Yaegi interpreter compiled to js/wasm inside a Web Worker, with a
hard wall-clock kill, at a download size and cold start we can ship?

## What was built

- `lab-wasm/cmd/lab/main.go` (`js && wasm`): registers `rewireRun(filesJSON) -> Promise<resultJSON>`
  and calls `labcore.Run` with `context.Background()`.
- `apps/web/src/lab/`: `webRunner.ts` (`webRun`, `createRunner`), `worker.ts` (browser module
  worker), `labHost.js` (shared load-wasm-once host), `nodeWorker.mjs` + `nodeWorkerFactory.ts`
  (worker_threads, used by Vitest).
- `just wasm-lab` builds `apps/web/public/lab/{lab.wasm,wasm_exec.js}` (git-ignored). `just test-web`
  builds it first when missing.

## Numbers

| Measure | Value |
| --- | --- |
| `lab.wasm` raw (`-trimpath -ldflags=-s`) | 40861701 bytes (40 MB) |
| `lab.wasm` gzip -9 | 8713026 bytes (8.7 MB), limit 12 MB |
| Cold start, Chromium (Playwright), 5 fresh contexts | 519 to 562 ms (5 runs, real `worker.ts`), limit 3000 ms |
| Cold start, WebKitGTK MiniBrowser, fresh cache dir per run | 664 ms (1 sample, real `worker.ts`) |
| Node worker_threads (Vitest) hello | about 350 ms |

Cold start is page start to the first `hello` result: worker spawn, fetch of `wasm_exec.js` and
`lab.wasm`, compile, `main()` registration, and one run. Of that, about 70 to 75 percent is
load/compile (`ready_ms`). **Caveat:** the static server is on localhost, uncompressed, so the 8.7 MB
(gz) download cost is not included. At 20 Mbit/s that adds roughly 3.5 s on a first visit, 0 on cached
visits; the browser should be served the gzip/brotli copy and the wasm is cacheable. Cold start
without the network is well under the 3 s limit; the network part is outside what this spike measures.

WebKit: Playwright WebKit does not run on Arch, so (ruling R10) WebKit numbers come from the
WebKitGTK MiniBrowser (same engine as Tauri on Linux), reported by the page to the server log.
The WebKit figure is a single sample; treat it as indicative.

The spike page runs the real `webRunner.ts` and `worker.ts`, compiled by `apps/web/spike/build-spike.sh`
(tsc, output in the git-ignored `apps/web/spike/.build/`). The earlier hand-copied `lab-worker.js` is gone.

Reproduce: `just wasm-lab`; `python3 apps/web/spike/server.py 8767` then
`node apps/web/spike/measure-lab.mjs http://127.0.0.1:8767/spike/lab.html 5` (it runs build-spike.sh);
`apps/web/spike/measure-lab-webkit.sh 2`.

## Real-worker browser check

The page runs hello, then `for {}` with `wallMs: 1000`, then hello again, through the real worker.

- Chromium (5 fresh contexts): hello prints `hi`; the loop is killed by `terminate()` at 1000 ms with
  `timedOut: true`; hello after the kill prints `hi`.
- WebKitGTK MiniBrowser (1 sample): hello prints `hi`; loop killed at 1001 ms with `timedOut: true`;
  hello after the kill prints `hi`.

## Findings

- **Kill switch must be on the JS side.** js/wasm has no goroutine preemption; a `for {}` never
  yields, so the Go context deadline is never seen. `webRun` starts a wall timer (default 10 s) and,
  on expiry, `worker.terminate()`s and respawns. Test: `for {}` with `wallMs: 1000` resolves
  `timedOut: true`, `exitCode: null` in about 1.0 s, and the next run works.
- The `timedOut` result is built in TS; labcore's `ExitCode` (0, 1) maps straight to `exitCode`.
- The wall timer starts after the worker reports ready, so slow first-load does not eat the learner's
  10 s. Init has its own 60 s limit.
- Runs are serialized per worker (queued): a timeout kills the whole worker, so overlap would kill
  other runs.
- **Memory is not bounded.** Go's wasm linear memory can grow until the browser stops it; browsers give
  no per-worker limit. In Node tests `resourceLimits` is set but only covers the JS heap, not wasm
  memory. The wall timer is the only guard against allocation loops. Worth a follow-up (an output/alloc
  guard in labcore or a periodic memory watchdog).
- No eval on the browser path. `wasm_exec.js` is loaded by a dynamic `import()` in the module worker,
  so a strict CSP needs only `script-src 'self'` and `'wasm-unsafe-eval'` (for WebAssembly
  instantiation). The server must serve `wasm_exec.js` with a JavaScript MIME type, because `import()`
  enforces it. Only the Node test worker uses `vm`. Fetches check `r.ok` and fail with `HTTP <status>`.

## Trimming

Not needed: the binary (including go/types from the Task 3 unused-variable check) gzips to 8.7 MB.
If it ever grows past budget, trim the stdlib allow-list in `labcore/symbols.go` first.
