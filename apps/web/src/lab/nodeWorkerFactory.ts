import { Worker } from "node:worker_threads";
import { fileURLToPath } from "node:url";
import type { FromWorker, ToWorker, WorkerLike } from "./protocol";

/** Node worker_threads factory for tests. `baseDir` ends with a slash. */
export function nodeWorkerFactory(baseDir: string): () => WorkerLike {
  const entry = fileURLToPath(new URL("./nodeWorker.mjs", import.meta.url));
  return () => {
    // resourceLimits bounds the JS heap only; Go's wasm linear memory is not
    // covered by it (and nothing bounds it in browsers either).
    const w = new Worker(entry, { workerData: { baseDir }, resourceLimits: { maxOldGenerationSizeMb: 512 } });
    let failure: (m: string) => void = () => {};
    w.on("error", (e) => failure(String(e)));
    w.on("exit", (c) => failure(`worker exited with code ${c}`));
    return {
      postMessage: (m: ToWorker) => w.postMessage(m),
      onMessage: (cb: (m: FromWorker) => void) => void w.on("message", cb),
      onFailure: (cb) => {
        failure = cb;
      },
      terminate: () => void w.terminate(),
    };
  };
}
