import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { renderTokensCss } from "../../scripts/tokens";

const cssUrl = new URL("./tokens.css", import.meta.url);
const jsonUrl = new URL("../../../../design/tokens.json", import.meta.url);

describe("design tokens", () => {
  it("tokens.css contains the four spec colors", () => {
    const css = readFileSync(cssUrl, "utf8").replace(/\s+/g, "");
    expect(css).toContain("--paper:#f6f0e4;");
    expect(css).toContain("--ink:#2a2620;");
    expect(css).toContain("--accent:#e2572b;");
    expect(css).toContain("--card:#fffdf7;");
  });

  it("tokens.css is up to date with design/tokens.json", () => {
    const json = JSON.parse(readFileSync(jsonUrl, "utf8"));
    expect(readFileSync(cssUrl, "utf8")).toBe(renderTokensCss(json));
  });
});
