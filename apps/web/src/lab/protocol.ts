export type SourceFile = { name: string; source: string };

/** Same shape the desktop backend returns (Task 13). */
export type RawRunResult = {
  stdout: string;
  stderr: string;
  exitCode: number | null;
  timedOut: boolean;
  truncated: boolean;
  durationMs: number;
};

// main thread -> worker
export type ToWorker =
  | { type: "init"; baseUrl: string }
  | { type: "run"; id: number; files: SourceFile[] };

// worker -> main thread
export type FromWorker =
  | { type: "ready" }
  | { type: "initError"; message: string }
  | { type: "result"; id: number; resultJson: string };

/** Minimal worker surface so the runner works in browsers and in Node. */
export interface WorkerLike {
  postMessage(msg: ToWorker): void;
  onMessage(cb: (msg: FromWorker) => void): void;
  /** Uncaught error or unexpected exit. */
  onFailure(cb: (message: string) => void): void;
  terminate(): void;
  /** Where lab.wasm and wasm_exec.js are served from (ends with a slash). */
  baseUrl?: string;
}
