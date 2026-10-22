// @vitest-environment jsdom
import { render } from "@testing-library/svelte";
import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { scenes } from "./index";

// The real lesson files (the built bundle is git-ignored).
// Not a literal `new URL("...", import.meta.url)`: Vite rewrites that under jsdom.
const rel = "../../../../content/go/packs/";
const packs = fileURLToPath(new URL(rel, import.meta.url));
function sceneIdsInContent(): Set<string> {
  const ids = new Set<string>();
  for (const pack of readdirSync(packs)) {
    const dir = `${packs}${pack}/chunks/`;
    for (const f of readdirSync(dir).filter((n) => n.endsWith(".yaml"))) {
      const m = readFileSync(dir + f, "utf8").match(/^\s+scene:\s*['"]?([\w-]+)/m);
      if (!m) throw new Error(`${pack}/${f} has no see.scene`);
      ids.add(m[1]);
    }
  }
  return ids;
}

describe("scenes", () => {
  it("every scene id used in the content has a drawing", () => {
    const used = sceneIdsInContent();
    expect(used.size).toBe(9);
    const missing = [...used].filter((id) => !(id in scenes));
    expect(missing).toEqual([]);
  });

  it("every scene is an image with a description", () => {
    for (const [id, Scene] of Object.entries(scenes)) {
      const { container, unmount } = render(Scene);
      const svg = container.querySelector("svg")!;
      expect(svg.getAttribute("role"), id).toBe("img");
      expect(svg.getAttribute("aria-label")?.length ?? 0, id).toBeGreaterThan(40);
      unmount();
    }
  });
});
