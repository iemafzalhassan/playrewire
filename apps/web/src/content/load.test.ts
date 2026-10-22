import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { loadBundle, resetBundleCache } from "./load";

const good = readFileSync(fileURLToPath(new URL("../../../../tools/packcheck/testdata/good.bundle.json", import.meta.url)), "utf8");
const ok = () => new Response(good, { status: 200 });

describe("loadBundle", () => {
  beforeEach(() => resetBundleCache());

  it("fetches and parses once, then serves the cached bundle", async () => {
    const f = vi.fn(async () => ok());
    const a = await loadBundle(f as unknown as typeof fetch);
    const b = await loadBundle(f as unknown as typeof fetch);
    expect(a).toBe(b);
    expect(f).toHaveBeenCalledOnce();
  });

  it("does not cache a failed load, so try again really retries", async () => {
    const f = vi
      .fn()
      .mockRejectedValueOnce(new TypeError("Failed to fetch"))
      .mockResolvedValueOnce(new Response("nope", { status: 503 }))
      .mockResolvedValueOnce(ok());
    await expect(loadBundle(f)).rejects.toThrow(/fetch/);
    await expect(loadBundle(f)).rejects.toThrow(/503/);
    await expect(loadBundle(f)).resolves.toMatchObject({ language: "go" });
    expect(f).toHaveBeenCalledTimes(3);
  });
});
