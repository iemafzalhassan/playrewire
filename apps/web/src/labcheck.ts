// E2E-only entry (labcheck.html, REWIRE_E2E=1 builds). Exercises the default
// browser paths in a production build: the module worker, its dynamic
// import() of wasm_exec.js, lab.wasm, and the core wasm.
import { initCore, level } from "./core";
import { webRun } from "./lab/webRunner";

const out = document.getElementById("out")!;
(async () => {
  try {
    await initCore();
    const r = await webRun([
      { name: "main.go", source: 'package main\n\nimport "fmt"\n\nfunc main() { fmt.Println("hello, rewire") }\n' },
    ]);
    out.textContent = JSON.stringify({ level0: level(0), ...r });
  } catch (e) {
    out.textContent = JSON.stringify({ error: String(e) });
  }
  out.dataset.done = "1";
})();
