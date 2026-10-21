// Task 4 spike: cold-start of the first hello in Playwright Chromium.
// Usage: node spike/measure-lab.mjs <url> [runs]  (each run = fresh context, cold HTTP cache)
import { chromium } from "playwright";
const [url = "http://127.0.0.1:8767/spike/lab.html", runs = "5"] = process.argv.slice(2);
const browser = await chromium.launch();
const out = [];
for (let i = 0; i < Number(runs); i++) {
  const ctx = await browser.newContext();
  const page = await ctx.newPage();
  await page.goto(url);
  await page.waitForFunction(() => /cold_ms|error/.test(document.getElementById("out").textContent), null, { timeout: 60000 });
  out.push(JSON.parse(await page.textContent("#out")));
  await ctx.close();
}
await browser.close();
console.log(JSON.stringify(out));
