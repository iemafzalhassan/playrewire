// Task 2 spike: measure the world page in Playwright Chromium at 1280x720.
// Usage: node spike/measure.mjs <url> <seconds> <screenshot.png>
// Prints every "rewire-world fps" console line and a JSON summary at the end.
// WebKit is measured with the WebKitGTK MiniBrowser instead (see docs/spikes).
import { chromium } from "playwright";

const [url = "http://127.0.0.1:8765/spike/world.html", secondsArg = "60", shot = "world-chromium.png"] =
  process.argv.slice(2);
const seconds = Number(secondsArg);
// The first 5 s window includes startup and shader compiles; it is dropped.
const WINDOW_S = 5;

const browser = await chromium.launch({
  headless: false,
  args: ["--ignore-gpu-blocklist", "--enable-gpu-rasterization"],
});
const page = await browser.newPage({ viewport: { width: 1280, height: 720 }, deviceScaleFactor: 1 });
const samples = [];
const t0 = Date.now();
page.on("console", (msg) => {
  const text = msg.text();
  if (process.env.DEBUG || /error/i.test(msg.type()) || /panicked|ERROR|WARN/.test(text)) console.error("[page]", msg.type(), text.slice(0, 400));
  const m = text.match(/rewire-world fps avg5s=([\d.]+)/);
  if (m) {
    const t = (Date.now() - t0) / 1000;
    samples.push({ t, fps: Number(m[1]) });
    console.log(`t=${t.toFixed(1)}s fps=${m[1]}`);
  }
});
await page.goto(url);
const gl = await page.evaluate(() => {
  const c = document.createElement("canvas").getContext("webgl2");
  const ext = c && c.getExtension("WEBGL_debug_renderer_info");
  return ext ? c.getParameter(ext.UNMASKED_RENDERER_WEBGL) : "unknown";
});
console.log("renderer:", gl);
await page.waitForTimeout((seconds + 2 * WINDOW_S + 2) * 1000);
await page.screenshot({ path: shot });
await browser.close();

const steady = samples.slice(1, 1 + Math.round(seconds / WINDOW_S));
const fps = steady.map((s) => s.fps);
const summary = {
  browser: "chromium",
  renderer: gl,
  windows: fps.length,
  mean: fps.reduce((a, b) => a + b, 0) / (fps.length || 1),
  min: Math.min(...fps),
  max: Math.max(...fps),
};
console.log(JSON.stringify(summary));
