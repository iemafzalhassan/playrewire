// Production check (Chromium via Playwright, against `vite preview`):
//  1. labcheck.html runs one Go program through webRun's default browser path
//     (module worker + dynamic import() of wasm_exec.js + lab.wasm) and loads
//     the core wasm, under the app's CSP.
//  2. The app shell renders Home and Map with no CSP violations or page errors.
//  3. The chunk player: the first coding chunk end to end (See, type the ghost,
//     three drills run as real Go by the browser lab) and its ✓ rewards.
// With --shots it also writes review screenshots to docs/design/.
//
// Usage: pnpm e2e [-- --shots]   (needs public/lab, public/content, src/core/pkg)
import { spawn, spawnSync } from "node:child_process";
import { mkdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const root = fileURLToPath(new URL("..", import.meta.url));
const bundle = JSON.parse(readFileSync(`${root}public/content/go.json`, "utf8"));
const allChunks = bundle.path.map((r) => bundle.packs.find((p) => p.id === r.pack).chunks.find((c) => c.id === r.chunk));
const firstCoding = allChunks.find((c) => c.steps.includes("type"));
const byPrompt = new Map(firstCoding.drills.map((d) => [d.prompt, d]));

/** Types a ghost the way a learner would: Enter at each line end skips the indent. */
async function typeGhost(page, ghost) {
  const lines = ghost.replace(/\r\n?/g, "\n").trimEnd().split("\n");
  for (const [i, line] of lines.entries()) {
    await page.keyboard.type(line.trimStart());
    if (i < lines.length - 1) await page.keyboard.press("Enter");
  }
}

/** Puts a drill's solution in the editor and runs it; resolves on the verdict. */
async function solveDrill(page, wrong = false, delay = 0) {
  const prompt = (await page.locator(".prompt-text").textContent()).trim();
  const d = byPrompt.get(prompt);
  await page.getByRole("textbox", { name: /your go code/i }).click();
  await page.keyboard.press("ControlOrMeta+a");
  const code = wrong ? "fmt.Println(oops)" : d.solution.trimEnd();
  if (delay) await page.keyboard.type(code, { delay });
  else await page.keyboard.insertText(code);
  await page.getByRole("button", { name: /^run/i }).click();
  await page.locator(wrong ? ".error-card" : '[data-testid="drill-result"] .pass-card').first().waitFor({ timeout: 60_000 });
}
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

  // 3. The chunk player: the first coding chunk with the real web runner.
  await page.goto(`${base}/#/learn/${firstCoding.pack}/${firstCoding.id}`);
  await page.waitForSelector(".see-step svg.scene[role=img]");
  await page.getByRole("button", { name: /next: type/i }).click();
  const area = page.getByRole("textbox", { name: /type the code/i });
  await area.waitFor();
  const pasted = await area.evaluate((el) => {
    const e = new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData: new DataTransfer() });
    el.dispatchEvent(e);
    return e.defaultPrevented;
  });
  check(pasted && (await page.getByText("Type it out, it's how your fingers learn.").isVisible()), "player: paste is cancelled with the hint");
  await typeGhost(page, firstCoding.type.ghost);
  await page.getByRole("button", { name: /next: drill/i }).click();
  for (let i = 0; i < 3; i++) {
    await solveDrill(page);
    if (i < 2) await page.getByRole("button", { name: /next drill/i }).click();
  }
  const verdict = await page.getByTestId("drill-result").textContent();
  check(verdict.includes("✓") && verdict.includes("Three in a row"), "player: three drills pass with ✓ via the web runner");
  const xp = await page.getByTestId("xp-float").textContent().catch(() => "");
  check(/\+\d+ xp/.test(xp), `player: reward shown (${xp.trim()})`);
  await page.getByRole("button", { name: /finish/i }).click();
  await page.getByText("Page done!").waitFor();
  const outbox = await page.evaluate(() => JSON.parse(localStorage.getItem("rewire.outbox.v1") ?? "[]"));
  check(
    outbox.some((r) => r.chunk_id === firstCoding.id && r.step === "drill" && r.success && /^\d{4}-\d\d-\d\d$/.test(r.local_date)),
    "player: the drill step is saved to the outbox",
  );

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

    // The chunk player (reduced motion, so scenes show their final frame).
    const play = async (name, seed, act, viewport = { width: 1200, height: 860 }) => {
      const c = await browser.newContext({ viewport, deviceScaleFactor: 1, reducedMotion: "reduce" });
      await c.addInitScript((p) => localStorage.setItem("rewire.progress.v1", JSON.stringify(p)), seed);
      const p = await c.newPage();
      await p.goto(`${base}/#/learn/go-academy/variables`);
      await p.waitForSelector("#page-title");
      await p.evaluate(() => document.fonts.ready);
      await act(p);
      await p.waitForTimeout(300);
      await p.screenshot({ path: `${out}${name}.png`, fullPage: true });
      console.log(`shot docs/design/${name}.png`);
      await c.close();
    };
    const others = progress.completed.filter((k) => !k.startsWith("variables:"));
    const at = (...done) => ({ ...progress, completed: [...others, ...done.map((s) => `variables:${s}`)] });
    const variables = bundle.packs.flatMap((p) => p.chunks).find((c) => c.id === "variables");
    for (const d of variables.drills) byPrompt.set(d.prompt, d);
    await play("see-step", at(), async (p) => p.waitForSelector(".see-step svg.scene"));
    await play("type-step", at("see"), async (p) => {
      await p.getByRole("textbox", { name: /type the code/i }).waitFor();
      await p.keyboard.type('name := "Mia"');
      await p.keyboard.press("Enter");
      await p.keyboard.type("lights := 5");
      await p.keyboard.press("Enter");
      await p.keyboard.type("lights = lighst");
    });
    await play("drill-error-card", at("see", "type"), async (p) => {
      await p.getByRole("button", { name: /hint/i }).click();
      await solveDrill(p, true);
      await p.locator(".error-card details").first().evaluate((d) => (d.open = true));
    });
    await play("drill-pass", at("see", "type"), async (p) => {
      await solveDrill(p, false, 110); // typed at a learner's pace, so the round's speed is real
      await p.waitForTimeout(3200); // let the xp float clear
    });
    await play("type-step-phone", at("see"), async (p) => {
      await p.getByRole("textbox", { name: /type the code/i }).waitFor();
      await p.keyboard.type('name := "Mia"');
      await p.keyboard.press("Enter");
      await p.keyboard.type("lights");
    }, { width: 390, height: 844 });
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
