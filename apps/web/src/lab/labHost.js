// Shared by the browser worker and the Node test worker: load lab.wasm once,
// then answer { type: "run", id, files } messages. Plain JS so Node can run it
// without a TS loader. `loadText(url)` / `loadBytes(url)` are environment hooks.
export async function startLab({ baseUrl, loadText, loadBytes, post, onMessage }) {
  try {
    // wasm_exec.js is a plain script that defines globalThis.Go.
    (0, eval)(await loadText(baseUrl + "wasm_exec.js"));
    const go = new globalThis.Go();
    const bytes = await loadBytes(baseUrl + "lab.wasm");
    const { instance } = await WebAssembly.instantiate(bytes, go.importObject);
    go.run(instance); // never returns; main() parks after registering rewireRun
    for (let i = 0; i < 1000 && !globalThis.rewireReady; i++) {
      await new Promise((r) => setTimeout(r, 1));
    }
    if (!globalThis.rewireReady) throw new Error("lab.wasm did not register rewireRun");
  } catch (e) {
    post({ type: "initError", message: String(e && e.stack ? e.stack : e) });
    return;
  }
  post({ type: "ready" });
  onMessage(async (msg) => {
    if (msg.type !== "run") return;
    let resultJson;
    try {
      resultJson = await globalThis.rewireRun(JSON.stringify(msg.files));
    } catch (e) {
      resultJson = JSON.stringify({
        stdout: "", stderr: "internal error: " + String(e) + "\n",
        exitCode: 1, timedOut: false, truncated: false,
      });
    }
    post({ type: "result", id: msg.id, resultJson });
  });
}
