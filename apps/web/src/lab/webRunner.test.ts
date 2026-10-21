import { afterAll, describe, expect, it } from "vitest";
import { fileURLToPath } from "node:url";
import { createRunner } from "./webRunner";
import { nodeWorkerFactory } from "./nodeWorkerFactory";

const labDir = fileURLToPath(new URL("../../public/lab/", import.meta.url));
const runner = createRunner(nodeWorkerFactory(labDir), labDir);
afterAll(() => runner.dispose());

const hello = [
  { name: "main.go", source: 'package main\nimport "fmt"\nfunc main() { fmt.Println("hi") }\n' },
];

describe("webRun (Yaegi in a worker)", () => {
  it("runs hello", async () => {
    const r = await runner.run(hello);
    expect(r.stdout).toBe("hi\n");
    expect(r.exitCode).toBe(0);
    expect(r.timedOut).toBe(false);
  }, 60_000);

  it("maps compile errors to exit code 1 with a position", async () => {
    const r = await runner.run([{ name: "main.go", source: "package main\nfunc main() { x := }\n" }]);
    expect(r.exitCode).toBe(1);
    expect(r.stderr).toMatch(/\d+:\d+/);
  });

  it("kills an infinite loop at wallMs", async () => {
    const t0 = performance.now();
    const r = await runner.run(
      [{ name: "main.go", source: "package main\nfunc main() { for {} }\n" }],
      { wallMs: 1000 },
    );
    const took = performance.now() - t0;
    expect(r.timedOut).toBe(true);
    expect(r.exitCode).toBeNull();
    expect(took).toBeLessThan(2000);
  }, 10_000);

  it("next run after a timeout still works", async () => {
    const r = await runner.run(hello);
    expect(r.stdout).toBe("hi\n");
    expect(r.timedOut).toBe(false);
  }, 60_000);

  it("queues concurrent calls and runs them one at a time", async () => {
    const [a, b] = await Promise.all([runner.run(hello), runner.run(hello)]);
    expect(a.stdout).toBe("hi\n");
    expect(b.stdout).toBe("hi\n");
  });
});
