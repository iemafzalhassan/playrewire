// @vitest-environment jsdom
import { render, screen, within } from "@testing-library/svelte";
import { beforeAll, describe, expect, it } from "vitest";
import type { Bundle, Chunk } from "../content/types";
import { initCore, type ProgressState } from "../core";
import { buildSkillMap } from "../today";
import SkillMap from "./SkillMap.svelte";

beforeAll(async () => {
  await initCore();
});

const mk = (id: string, title: string): Chunk => ({
  id,
  concept: id,
  title,
  see: { scene: id, caption: id },
  drills: [0, 1, 2, 3, 4].map((n) => ({ prompt: `p${n}`, solution: "s", expectStdout: "x\n", wholeFile: false })),
  hints: ["a", "b", "c"],
  pack: "go-academy",
  steps: ["see", "type", "drill"],
});
const chunks = [mk("print", "Your first words"), mk("variables", "Boxes with names"), mk("if-else", "Forks in the road")];
const bundle: Bundle = {
  version: 1,
  language: "go",
  wrapperLineOffset: 7,
  cityStub: "",
  path: chunks.map((c) => ({ pack: "go-academy", chunk: c.id })),
  packs: [{ id: "go-academy", title: "Go", district: "Go Academy", chunks, missions: [] }],
};
const state: ProgressState = {
  xp: 300,
  streak: { current: 3, best: 3, last_day: null, freezes: 0 },
  concepts: {
    print: { box_level: 4, due: "2026-11-01", successes: 5, fails: 0 },
    variables: { box_level: 0, due: "2026-10-25", successes: 2, fails: 4 },
    "if-else": { box_level: 1, due: "2026-10-26", successes: 1, fails: 2 },
  },
  completed: chunks.flatMap((c) => c.steps.map((s) => `${c.id}:${s}`)),
  mission_encounters: {},
};

describe("SkillMap", () => {
  it("skill map lists keeps-breaking at fails>=3", () => {
    render(SkillMap, { props: { map: buildSkillMap(state, bundle, "2026-10-24") } });
    const bars = screen.getAllByRole("meter");
    expect(bars.map((b) => b.getAttribute("aria-label"))).toEqual(["Your first words", "Boxes with names", "Forks in the road"]);
    expect(bars.map((b) => b.getAttribute("aria-valuenow"))).toEqual(["4", "0", "1"]);
    expect(bars[0].getAttribute("aria-valuemax")).toBe("5");

    const list = screen.getByRole("list", { name: /keeps breaking/i });
    const items = within(list).getAllByRole("listitem");
    expect(items).toHaveLength(1);
    expect(items[0].textContent).toContain("Boxes with names");
    const link = within(items[0]).getByRole("link", { name: /practise it/i });
    expect(link.getAttribute("href")).toBe("#/practise/variables");
  });

  it("a new learner sees an empty page that points to today", () => {
    const fresh: ProgressState = { ...state, concepts: {}, completed: [] };
    render(SkillMap, { props: { map: buildSkillMap(fresh, bundle, "2026-10-24") } });
    expect(screen.queryAllByRole("meter")).toHaveLength(0);
    expect(screen.getByText(/nothing here yet/i)).toBeTruthy();
    expect(screen.getByRole("link", { name: /today/i }).getAttribute("href")).toBe("#/");
  });
});
