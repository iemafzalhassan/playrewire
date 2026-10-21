// worker_threads entry used by Vitest/Node (see nodeWorkerFactory.ts).
import { parentPort, workerData } from "node:worker_threads";
import { readFile } from "node:fs/promises";
import vm from "node:vm";
import { startLab } from "./labHost.js";

let handler = () => {};
parentPort.on("message", (msg) => {
  if (msg.type === "init") {
    startLab({
      baseUrl: workerData.baseDir,
      // Node (tests only): run the script in this context; no CSP here.
      loadRuntime: async (p) => { vm.runInThisContext(await readFile(p, "utf8"), { filename: p }); },
      loadBytes: (p) => readFile(p),
      post: (m) => parentPort.postMessage(m),
      onMessage: (cb) => { handler = cb; },
    });
  } else handler(msg);
});
