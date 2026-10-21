// Browser Web Worker entry (module worker). Loads lab.wasm once and answers
// run requests. NOTE: wasm_exec.js is evaluated from fetched text, so a strict
// CSP must allow 'unsafe-eval' (or wasm-unsafe-eval plus this script) for the
// worker. Revisit in Task 13 if the app ships a CSP.
import { startLab } from "./labHost.js";
import type { ToWorker } from "./protocol";

let handler: (m: ToWorker) => void = () => {};
self.onmessage = (e: MessageEvent<ToWorker>) => {
  const msg = e.data;
  if (msg.type === "init") {
    void startLab({
      baseUrl: msg.baseUrl,
      loadText: async (u) => (await fetch(u)).text(),
      loadBytes: async (u) => {
        const r = await fetch(u);
        if (!r.ok) throw new Error(`fetch ${u}: ${r.status}`);
        return r.arrayBuffer();
      },
      post: (m) => self.postMessage(m),
      onMessage: (cb) => {
        handler = cb;
      },
    });
  } else {
    handler(msg);
  }
};
