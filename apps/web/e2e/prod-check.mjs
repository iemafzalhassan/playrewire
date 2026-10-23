// Production check (Chromium via Playwright, against `vite preview`):
//  1. labcheck.html runs one Go program through webRun's default browser path
//     (module worker + dynamic import() of wasm_exec.js + lab.wasm) and loads
//     the core wasm, under the app's CSP.
//  2. The app shell renders Home and Map with no CSP violations or page errors.
//  3. The chunk player: the first coding chunk end to end (See, type the ghost,
//     three drills run as real Go by the browser lab) and its ✓ rewards.
//  4. The Use step in the 3D street (Bevy wasm, WebGL2) under the CSP: both missions
//     walked with the keyboard, a wrong run (ink circle + caption) then a right one,
//     the canvas filling its box at DPR 1 and 2, and the no-GPU sketch street.
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
  // The keys go to a hidden, focused <input> (so phones get a keyboard); paste must still be refused.
  const pasted = await area.evaluate((el) => {
    const clip = new DataTransfer();
    clip.setData("text/plain", "fmt.Println");
    const paste = new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData: clip });
    el.dispatchEvent(paste);
    const before = new InputEvent("beforeinput", { bubbles: true, cancelable: true, inputType: "insertFromPaste", data: "fmt" });
    el.dispatchEvent(before);
    return {
      tag: el.tagName,
      focused: document.activeElement === el,
      cancelled: paste.defaultPrevented && before.defaultPrevented,
      value: el.value,
      typed: document.querySelectorAll(".c.done").length,
    };
  });
  check(
    pasted.tag === "INPUT" && pasted.focused && pasted.cancelled && pasted.value === "" && pasted.typed === 0 &&
      (await page.getByText("Type it out, it's how your fingers learn.").isVisible()),
    `player: paste into the focused input is cancelled with the hint ${JSON.stringify(pasted)}`,
  );
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

  // 4. The Use step: the street, both missions, as a learner would play them.
  const out = fileURLToPath(new URL("../../../docs/design/", import.meta.url));
  if (shots) mkdirSync(out, { recursive: true });
  const snap = async (p, name) => {
    if (!shots) return;
    await p.screenshot({ path: `${out}${name}.png` });
    console.log(`shot docs/design/${name}.png`);
  };
  const seeded = (chunk) => ({
    xp: 120,
    streak: { current: 2, best: 2, last_day: null, freezes: 0 },
    concepts: {},
    completed: ["see", "type", "drill"].map((s) => `${chunk}:${s}`),
    mission_encounters: {},
  });
  const missions = [
    {
      pack: "go-academy",
      chunk: "for-loop",
      id: "green-wave",
      // Spawn (-17, 0.6): up to the far kerb, then right until the terminal (-12, -3.5) is in reach.
      walk: ["ArrowUp", "ArrowRight"],
      wrong: "for i := 0; i < 5; i++ {\n    SetGreen(i, i*3)\n}",
      caption: "Light 1 turned green at the wrong time.",
      right: "for i := 0; i < 5; i++ {\n    SetGreen(i, i*2)\n}",
    },
    {
      pack: "silicon-quarter",
      chunk: "binary-bridge",
      id: "binary-bridge",
      // Spawn (2, 0.6), terminal at (7.6, -3.5): the same walk with W and D.
      walk: ["KeyW", "KeyD"],
      wrong: 'bits := fmt.Sprintf("%08b", 12)\nfor i := 0; i < 8; i++ {\n    Lamp(i, bits[i] == \'1\')\n}',
      caption: "Lamp 7 should be on.",
      right: 'bits := fmt.Sprintf("%08b", 13)\nfor i := 0; i < 8; i++ {\n    Lamp(i, bits[i] == \'1\')\n}',
    },
  ];
  const write = async (p, code) => {
    await p.getByRole("textbox", { name: /your go code/i }).click();
    await p.keyboard.press("ControlOrMeta+a");
    await p.keyboard.insertText(code);
  };
  for (const m of missions) {
    const c = await browser.newContext({ viewport: { width: 1280, height: 860 }, deviceScaleFactor: 1 });
    await c.addInitScript((seed) => {
      window.__csp = [];
      document.addEventListener("securitypolicyviolation", (e) => window.__csp.push(`${e.violatedDirective} ${e.blockedURI}`));
      localStorage.setItem("rewire.progress.v1", JSON.stringify(seed));
    }, seeded(m.chunk));
    const p = await c.newPage();
    const errs = [];
    p.on("pageerror", (e) => errs.push(String(e)));
    await p.goto(`${base}/#/learn/${m.pack}/${m.chunk}`);
    await p.locator(".use-step .hud").waitFor({ timeout: 60_000 });
    const fill = await p.evaluate(() => {
      const c = document.querySelector(".use-step .world canvas");
      return c && { w: c.width, h: c.height, cw: c.clientWidth, ch: c.clientHeight };
    });
    check(
      !!fill && Math.abs(fill.w - fill.cw) <= 1 && Math.abs(fill.h - fill.ch) <= 1 && fill.cw > 600,
      `${m.id}: the 3D street fills its box ${JSON.stringify(fill)}`,
    );
    // Walk there with the keyboard, like a learner: click the street to focus it, then keys.
    await p.locator(".use-step .world canvas").focus();
    // Software GL in CI draws few frames and Bevy caps each frame's step, so walk
    // "until there" rather than for a fixed time.
    const prompt = p.getByRole("button", { name: /use the terminal/i });
    const [up, right] = m.walk;
    await p.keyboard.down(up);
    await p.waitForTimeout(2500); // the kerb stops the player
    await p.keyboard.up(up);
    await p.keyboard.down(right);
    const near = await prompt.waitFor({ timeout: 20_000 }).then(() => true, () => false);
    await p.keyboard.up(right);
    check(near, `${m.id}: walking up to the terminal shows the prompt`);
    await p.waitForTimeout(400);
    if (m.id === "green-wave") await snap(p, "street-terminal-prompt");
    await p.keyboard.press("e");
    const opened = await p.getByRole("textbox", { name: /your go code/i }).waitFor({ timeout: 5000 }).then(() => true, () => false);
    check(opened, `${m.id}: E at the terminal opens the editor`);
    if (!opened) await p.getByRole("button", { name: /go to the terminal|use the terminal/i }).click();
    const template = await p.getByRole("textbox", { name: /your go code/i }).textContent();
    check(template.includes("___"), `${m.id}: the first encounter fills one blank`);

    await write(p, m.wrong);
    await p.getByRole("button", { name: /^run/i }).click();
    await p.locator(".wrong-card").waitFor({ timeout: 60_000 });
    const caption = await p.locator(".wrong-card .verdict").textContent();
    check(caption.includes(m.caption) && caption.includes("✗"), `${m.id}: wrong run says "${caption.trim()}"`);
    await p.waitForTimeout(900); // the ink circle draws itself
    await snap(p, m.id === "green-wave" ? "street-wrong-light" : "street-wrong-lamp");

    await write(p, m.right);
    await p.getByRole("button", { name: /^run/i }).click();
    if (m.id === "green-wave") {
      await p.getByText("Watch the street…").waitFor({ timeout: 60_000 });
      await p.waitForTimeout(650); // lights 0 and 1 are green, 2 is about to go
      await snap(p, "street-green-wave");
    }
    await p.locator(".pass-card").waitFor({ timeout: 60_000 });
    check((await p.locator(".pass-card").textContent()).includes("✓"), `${m.id}: a right run passes with ✓`);
    await p.waitForTimeout(1500);
    if (m.id === "binary-bridge") await snap(p, "street-bridge-13");
    const box = await p.evaluate(() => JSON.parse(localStorage.getItem("rewire.outbox.v1") ?? "[]"));
    check(
      box.some((r) => r.chunk_id === m.chunk && r.step === "use" && r.mission_id === m.id && r.success),
      `${m.id}: the use step is saved with its mission id`,
    );
    await p.getByRole("button", { name: /finish|next/i }).first().click();
    await p.getByText("Page done!").waitFor();
    const csp = await p.evaluate(() => window.__csp);
    check(csp.length === 0 && errs.length === 0, `${m.id}: no CSP violations or page errors ${JSON.stringify([...csp, ...errs])}`);
    await c.close();
  }

  // HiDPI: the backing store is CSS size x devicePixelRatio (the WebKitGTK quarter-window
  // bug). Playwright's emulated deviceScaleFactor reports devicePixelContentBoxSize in CSS
  // px, so this uses a browser with a real device scale factor of 2.
  {
    const hidpi = await chromium.launch({ args: ["--force-device-scale-factor=2"] });
    const c = await hidpi.newContext({ viewport: { width: 1100, height: 800 } });
    await c.addInitScript((seed) => localStorage.setItem("rewire.progress.v1", JSON.stringify(seed)), seeded("for-loop"));
    const p = await c.newPage();
    await p.goto(`${base}/#/learn/go-academy/for-loop`);
    await p.locator(".use-step .hud").waitFor({ timeout: 60_000 });
    await p.waitForTimeout(500);
    const r = await p.evaluate(() => {
      const c = document.querySelector(".use-step .world canvas");
      return { w: c.width, h: c.height, cw: c.clientWidth, ch: c.clientHeight };
    });
    check(Math.abs(r.w - r.cw * 2) <= 2 && Math.abs(r.h - r.ch * 2) <= 2, `HiDPI: canvas backing store is CSS x DPR ${JSON.stringify(r)}`);
    // And it follows a resize.
    await p.setViewportSize({ width: 800, height: 700 });
    // Software GL at 2x draws slowly: give the next frame time to land.
    await p
      .waitForFunction(() => {
        const c = document.querySelector(".use-step .world canvas");
        return Math.abs(c.width - c.clientWidth * 2) <= 2 && Math.abs(c.height - c.clientHeight * 2) <= 2;
      }, null, { timeout: 15_000 })
      .catch(() => {});
    const r2 = await p.evaluate(() => {
      const c = document.querySelector(".use-step .world canvas");
      return { w: c.width, h: c.height, cw: c.clientWidth, ch: c.clientHeight };
    });
    check(Math.abs(r2.w - r2.cw * 2) <= 2 && Math.abs(r2.h - r2.ch * 2) <= 2, `HiDPI: still filled after a resize ${JSON.stringify(r2)}`);
    await c.close();
    await hidpi.close();
  }

  // No GPU: the sketch street, same mission, same captions.
  {
    const c = await browser.newContext({ viewport: { width: 1280, height: 860 }, deviceScaleFactor: 1, reducedMotion: "reduce" });
    await c.addInitScript((seed) => {
      const get = HTMLCanvasElement.prototype.getContext;
      HTMLCanvasElement.prototype.getContext = function (kind, ...rest) {
        return kind === "webgl2" ? null : get.call(this, kind, ...rest);
      };
      localStorage.setItem("rewire.progress.v1", JSON.stringify(seed));
    }, seeded("for-loop"));
    const p = await c.newPage();
    await p.goto(`${base}/#/learn/go-academy/for-loop`);
    await p.getByRole("img", { name: /dock st/i }).waitFor({ timeout: 30_000 });
    check((await p.locator(".use-step canvas").count()) === 0, "no GPU: the sketch street stands in");
    await p.getByRole("button", { name: /go to the terminal/i }).click();
    await write(p, missions[0].wrong);
    await p.getByRole("button", { name: /^run/i }).click();
    await p.locator(".wrong-card").waitFor({ timeout: 60_000 });
    check((await p.locator(".sketch .ring").count()) === 1, "no GPU: the wrong light is circled");
    await snap(p, "street-no-gpu-fallback");
    await write(p, missions[0].right);
    await p.getByRole("button", { name: /^run/i }).click();
    await p.locator(".pass-card").waitFor({ timeout: 60_000 });
    const label = await p.getByRole("img", { name: /dock st/i }).getAttribute("aria-label");
    check(label.includes("Green: 0, 1, 2, 3, 4"), `no GPU: the mission passes and the sketch shows it (${label})`);
    await c.close();
  }

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
    const play = async (name, seed, act, viewport = { width: 1200, height: 860 }, extra = {}) => {
      const c = await browser.newContext({ viewport, deviceScaleFactor: 1, reducedMotion: "reduce", ...extra });
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
    // A phone: touch, mobile viewport; the hidden input holds focus so the keyboard would be up.
    await play("type-step-phone", at("see"), async (p) => {
      const keys = p.getByRole("textbox", { name: /type the code/i });
      await keys.waitFor();
      await p.locator(".ghost").tap();
      await p.keyboard.type('name := "Mia"');
      await p.keyboard.press("Enter");
      await p.keyboard.type("lights");
      const focused = await keys.evaluate((el) => document.activeElement === el);
      check(focused, "phone: the hidden input keeps focus while typing");
    }, { width: 390, height: 844 }, { isMobile: true, hasTouch: true, deviceScaleFactor: 1 });
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
