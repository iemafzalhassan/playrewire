import { describe, expect, it } from "vitest";
import { level } from "./index";

describe("rewire-core wrapper before init", () => {
  it("throws a clear error", () => {
    expect(() => level(1)).toThrow(/initCore/);
  });
});
