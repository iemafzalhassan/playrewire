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
| Cold start, Chromium (Playwright), 5 fresh contexts | 518 to 554 ms (median 527), limit 3000 ms |
| Cold start, WebKitGTK MiniBrowser, fresh cache dir per run | 645 and 671 ms |
| Node worker_threads (Vitest) hello | about 350 ms |

Cold start is page start to the first `hello` result: worker spawn, fetch of `wasm_exec.js` and
`lab.wasm`, compile, `main()` registration, and one run. Of that, about 70 to 75 percent is
load/compile (`ready_ms`). **Caveat:** the static server is on localhost, uncompressed, so the 8.7 MB
(gz) download cost is not included. At 20 Mbit/s that adds roughly 3.5 s on a first visit, 0 on cached
visits; the browser should be served the gzip/brotli copy and the wasm is cacheable. Cold start
without the network is well under the 3 s limit; the network part is outside what this spike measures.

WebKit: Playwright WebKit does not run on Arch, so (ruling R10) WebKit numbers come from the
WebKitGTK MiniBrowser (same engine as Tauri on Linux), reported by the page to the server log.
Only two of three launches logged a result; treat the WebKit figure as indicative.

Reproduce: `just wasm-lab`; `python3 apps/web/spike/server.py 8767` then
`node apps/web/spike/measure-lab.mjs`; `apps/web/spike/measure-lab-webkit.sh 3`.

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
- `wasm_exec.js` is evaluated from fetched text inside the worker (module workers have no
  `importScripts`). A strict CSP would need to allow that; revisit in Task 13.

## Trimming

Not needed: the binary (including go/types from the Task 3 unused-variable check) gzips to 8.7 MB.
If it ever grows past budget, trim the stdlib allow-list in `labcore/symbols.go` first.
