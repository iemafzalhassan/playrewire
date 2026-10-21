import type { FromWorker, RawRunResult, SourceFile, WorkerLike } from "./protocol";

export type { RawRunResult, SourceFile } from "./protocol";

export const DEFAULT_WALL_MS = 10_000;
const INIT_TIMEOUT_MS = 60_000;

export type WorkerFactory = () => WorkerLike;

/** Browser factory: a module Web Worker; wasm assets live under /lab/. */
export function browserWorkerFactory(): WorkerLike {
  const w = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
  let failure: (m: string) => void = () => {};
  w.onerror = (e) => failure(e.message || "worker error");
  w.onmessageerror = () => failure("worker message error");
  const base = new URL("/lab/", self.location.href).href;
  return {
    postMessage: (m) => {
      w.postMessage(m);
    },
    onMessage: (cb) => {
      w.onmessage = (e: MessageEvent<FromWorker>) => cb(e.data);
    },
    onFailure: (cb) => {
      failure = cb;
    },
    terminate: () => w.terminate(),
    baseUrl: base,
  };
}

type Slot = { worker: WorkerLike; ready: Promise<void>; dead: boolean };

/**
 * One warm worker, runs are serialized: calls queue and execute one at a time
 * (the interpreter is single-threaded and a timeout kills the whole worker, so
 * overlapping runs would kill each other). On timeout the worker is terminated
 * and a fresh one is spawned straight away so the next run starts warm-ish.
 *
 * The wall timer here is the real kill switch. js/wasm has no goroutine
 * preemption, so Go-side deadlines can never be relied upon.
 */
export function createRunner(factory: WorkerFactory, baseUrl?: string) {
  let slot: Slot | null = null;
  let nextId = 1;
  let queue: Promise<unknown> = Promise.resolve();
  let pending: ((r: { failure: string } | { result: string }, id?: number) => void) | null = null;

  function spawn(): Slot {
    const worker = factory();
    const s: Slot = { worker, dead: false, ready: undefined as unknown as Promise<void> };
    s.ready = new Promise<void>((resolve, reject) => {
      const t = setTimeout(() => reject(new Error("lab.wasm init timed out")), INIT_TIMEOUT_MS);
      worker.onMessage((msg) => {
        if (msg.type === "ready") {
          clearTimeout(t);
          resolve();
        } else if (msg.type === "initError") {
          clearTimeout(t);
          reject(new Error(msg.message));
        } else if (msg.type === "result" && pending && slot === s) {
          pending({ result: msg.resultJson }, msg.id);
        }
      });
      worker.onFailure((m) => {
        s.dead = true;
        clearTimeout(t);
        reject(new Error(m));
        if (pending && slot === s) pending({ failure: m });
      });
    });
    s.ready.catch(() => {}); // surfaced when a run awaits it
    const b = baseUrl ?? worker.baseUrl ?? "/lab/";
    worker.postMessage({ type: "init", baseUrl: b });
    return s;
  }

  function killSlot() {
    if (slot) {
      slot.dead = true;
      slot.worker.terminate();
      slot = null;
    }
  }

  function fail(stderr: string, durationMs: number): RawRunResult {
    return { stdout: "", stderr, exitCode: 1, timedOut: false, truncated: false, durationMs };
  }

  async function runOne(files: SourceFile[], wallMs: number): Promise<RawRunResult> {
    const t0 = performance.now();
    const elapsed = () => Math.round(performance.now() - t0);
    if (!slot || slot.dead) slot = spawn();
    const s = slot;
    try {
      await s.ready; // load time does not count against the learner's wall clock
    } catch (e) {
      killSlot();
      return fail(`could not start the code runner: ${String(e)}\n`, elapsed());
    }
    const runStart = performance.now();
    const id = nextId++;
    return new Promise<RawRunResult>((resolve) => {
      const done = (r: RawRunResult) => {
        clearTimeout(timer);
        pending = null;
        resolve(r);
      };
      const timer = setTimeout(() => {
        killSlot();
        slot = spawn(); // respawn so the next run is warm
        done({ stdout: "", stderr: "", exitCode: null, timedOut: true, truncated: false, durationMs: Math.round(performance.now() - runStart) });
      }, wallMs);
      pending = (r, rid) => {
        if ("failure" in r) {
          killSlot();
          return done(fail(`the code runner crashed: ${r.failure}\n`, elapsed()));
        }
        if (rid !== id) return;
        try {
          const j = JSON.parse(r.result);
          done({
            stdout: j.stdout,
            stderr: j.stderr,
            exitCode: j.exitCode,
            timedOut: !!j.timedOut,
            truncated: !!j.truncated,
            durationMs: Math.round(performance.now() - runStart),
          });
        } catch (e) {
          done(fail(`bad result from the code runner: ${String(e)}\n`, elapsed()));
        }
      };
      s.worker.postMessage({ type: "run", id, files });
    });
  }

  return {
    run(files: SourceFile[], limits?: { wallMs?: number }): Promise<RawRunResult> {
      const wallMs = limits?.wallMs ?? DEFAULT_WALL_MS;
      const p = queue.then(() => runOne(files, wallMs));
      queue = p.catch(() => {});
      return p;
    },
    dispose() {
      killSlot();
    },
  };
}

let defaultRunner: ReturnType<typeof createRunner> | null = null;

/** Run learner Go code in the warm browser worker. Calls are queued. */
export function webRun(files: SourceFile[], limits?: { wallMs?: number }): Promise<RawRunResult> {
  defaultRunner ??= createRunner(browserWorkerFactory);
  return defaultRunner.run(files, limits);
}
