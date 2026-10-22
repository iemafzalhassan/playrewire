// @vitest-environment jsdom
// initCore must still pick the Node loader when a DOM test environment
// defines `window` (component tests run under jsdom).
import { describe, expect, it } from "vitest";
import { initCore, level } from "./index";

describe("rewire-core under jsdom", () => {
  it("initCore loads under a DOM test environment", async () => {
    await initCore();
    expect(level(0)).toBeGreaterThanOrEqual(1);
  });
});
