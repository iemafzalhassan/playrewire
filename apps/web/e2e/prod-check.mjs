// Production check (Chromium via Playwright, against `vite preview`):
//  1. labcheck.html runs one Go program through webRun's default browser path
//     (module worker + dynamic import() of wasm_exec.js + lab.wasm) and loads
//     the core wasm, under the app's CSP.
//  2. The app shell renders Home and Map with no CSP violations or page errors.
// With --shots it also writes review screenshots to docs/design/.
//
// Usage: pnpm e2e [-- --shots]   (needs public/lab, public/content, src/core/pkg)
import { spawn, spawnSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const root = fileURLToPath(new URL("..", import.meta.url));
const shots = process.argv.includes("--shots");
const PORT = 4179;
const base = `http://127.0.0.1:${PORT}`;

const build = spawnSync("pnpm", ["exec", "vite", "build", "--logLevel", "warn"], {
  cwd: root,
  stdio: "inherit",
  env: { ...process.env, REWIRE_E2E: "1" },
});
if (build.status !== 0) process.exit(build.status ?? 1);

const server = spawn("pnpm", ["exec", "vite", "preview", "--outDir", "dist-e2e", "--port", String(PORT), "--strictPort", "--host", "127.0.0.1"], {
  cwd: root,
  stdio: ["ignore", "pipe", "inherit"],
  env: { ...process.env, REWIRE_E2E: "1" },
});
await new Promise((resolve, reject) => {
  server.stdout.on("data", (d) => String(d).includes(String(PORT)) && resolve());
  server.on("exit", (c) => reject(new Error(`preview exited ${c}`)));
});

const failures = [];
const check = (ok, what) => {
  console.log(`${ok ? "ok  " : "FAIL"} ${what}`);
  if (!ok) failures.push(what);
};

const browser = await chromium.launch();
try {
  const ctx = await browser.newContext({ viewport: { width: 1200, height: 860 }, deviceScaleFactor: 1 });
  await ctx.addInitScript(() => {
    window.__csp = [];
    document.addEventListener("securitypolicyviolation", (e) =>
      window.__csp.push(`${e.violatedDirective} ${e.blockedURI} at ${e.sourceFile}:${e.lineNumber}:${e.columnNumber}`),
    );
  });
  const page = await ctx.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));

  // 1. Lab + core wasm in the production build, under CSP.
  await page.goto(`${base}/labcheck.html`);
  await page.waitForSelector("#out[data-done]", { timeout: 60_000 });
  const res = JSON.parse(await page.textContent("#out"));
  console.log("labcheck:", JSON.stringify(res));
  check(res.stdout === "hello, rewire\n" && res.exitCode === 0, "webRun hello in the production build");
  check(res.level0 === 1, "core wasm loads in the production build");
  check((await page.evaluate(() => window.__csp)).length === 0, "labcheck: no CSP violations");

  // 2. The shell: Home, then Map.
  await page.goto(`${base}/`);
  await page.waitForSelector("h1:has-text(\"today's page\")");
  check((await page.locator("button[data-primary]").count()) === 1, "home: one primary action");
  check(await page.evaluate(() => document.fonts.check("700 32px Caveat")), "home: Caveat font loaded from the bundle");
  await page.click('nav a[href="#/map"]');
  await page.waitForSelector('[role="button"][aria-label^="Silicon Quarter"]');
  check((await page.locator('[aria-label$="not yet open"]').count()) === 8, "map: eight districts not yet open");
  const appCsp = await page.evaluate(() => window.__csp);
  check(appCsp.length === 0, `app: no CSP violations${appCsp.length ? ": " + appCsp.join(" | ") : ""}`);
  check(errors.length === 0, `no page errors${errors.length ? ": " + errors.join(" | ") : ""}`);

  if (shots) {
    const out = fileURLToPath(new URL("../../../docs/design/", import.meta.url));
    mkdirSync(out, { recursive: true });
    // A player a few days in: two pages done, one half done.
    const progress = {
      xp: 340,
      streak: { current: 4, best: 6, last_day: null, freezes: 1 },
      concepts: { "print": { box_level: 1, due: "2000-01-01", successes: 2, fails: 1 } },
      completed: ["what-is-a-computer:see", "hello-print:see", "hello-print:type", "hello-print:drill", "cpu-ram-bus:see", "variables:see"],
      mission_encounters: {},
    };
    const shot = async (name, hash, viewport, seed) => {
      const c = await browser.newContext({ viewport, deviceScaleFactor: 1, reducedMotion: "reduce" });
      if (seed) await c.addInitScript((p) => localStorage.setItem("rewire.progress.v1", JSON.stringify(p)), seed);
      const p = await c.newPage();
      await p.goto(`${base}/${hash}`);
      await p.waitForSelector("#page-title");
      if (hash === "#/map") await p.click('[role="button"][aria-label^="Go Academy"]');
      await p.evaluate(() => document.fonts.ready);
      await p.screenshot({ path: `${out}${name}.png`, fullPage: true });
      console.log(`shot docs/design/${name}.png`);
      await c.close();
    };
    await shot("home", "#/", { width: 1200, height: 860 }, progress);
    await shot("home-first-day", "#/", { width: 1200, height: 860 }, null);
    await shot("home-phone", "#/", { width: 390, height: 844 }, progress);
    await shot("map", "#/map", { width: 1200, height: 860 }, progress);
    await shot("map-phone", "#/map", { width: 390, height: 844 }, progress);
    await shot("settings", "#/settings", { width: 1200, height: 860 }, progress);
  }
} finally {
  await browser.close();
  server.kill();
}
if (failures.length) {
  console.error(`\n${failures.length} check(s) failed`);
  process.exit(1);
}
console.log("\nall production checks passed");
