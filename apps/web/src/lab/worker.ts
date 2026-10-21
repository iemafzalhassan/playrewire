// Browser Web Worker entry (module worker). Loads lab.wasm once and answers
// run requests. No eval: wasm_exec.js is loaded with a dynamic import (a plain
// same-origin script fetch that defines globalThis.Go). A strict CSP needs only
// script-src 'self' plus 'wasm-unsafe-eval' (for WebAssembly.instantiate).
import { startLab } from "./labHost.js";
import type { ToWorker } from "./protocol";

let handler: (m: ToWorker) => void = () => {};
self.onmessage = (e: MessageEvent<ToWorker>) => {
  const msg = e.data;
  if (msg.type === "init") {
    void startLab({
      baseUrl: msg.baseUrl,
      loadRuntime: async (u) => {
        await import(/* @vite-ignore */ u);
      },
      loadBytes: async (u) => {
        const r = await fetch(u);
        if (!r.ok) throw new Error(`fetch ${u}: HTTP ${r.status}`);
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
