// worker_threads entry used by Vitest/Node (see nodeWorkerFactory.ts).
import { parentPort, workerData } from "node:worker_threads";
import { readFile } from "node:fs/promises";
import { startLab } from "./labHost.js";

let handler = () => {};
parentPort.on("message", (msg) => {
  if (msg.type === "init") {
    startLab({
      baseUrl: workerData.baseDir,
      loadText: (p) => readFile(p, "utf8"),
      loadBytes: (p) => readFile(p),
      post: (m) => parentPort.postMessage(m),
      onMessage: (cb) => { handler = cb; },
    });
  } else handler(msg);
});
