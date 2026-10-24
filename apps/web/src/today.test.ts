import { beforeAll, describe, expect, it } from "vitest";
import type { Bundle, Chunk, Drill, Step } from "./content/types";
import { initCore, type ConceptState, type ProgressState } from "./core";
import { advanceDay, buildSkillMap, buildToday, dayParts, FIX_SUFFIX, planDay, WARMUP_MAX } from "./today";

beforeAll(async () => {
  await initCore();
});

const drill = (n: number): Drill => ({
  prompt: `drill ${n}`,
  solution: `fmt.Println(${n})`,
  expectStdout: `${n}\n`,
  wholeFile: false,
});

function chunk(id: string, steps: Step[], opts: { fix?: boolean; concept?: string } = {}): Chunk {
  const coding = steps.includes("drill");
  return {
    id,
    concept: opts.concept ?? id,
    title: `Title ${id}`,
    see: { scene: id, caption: id },
    drills: coding ? [0, 1, 2, 3, 4].map(drill) : [],
    hints: coding ? ["h1", "h2", "h3"] : [],
    ...(opts.fix ? { fixIt: { broken: `broken ${id}`, solution: "x", expectStdout: "ok\n", wholeFile: false } } : {}),
    pack: "go-academy",
    steps,
  };
}

const chunks = [
  chunk("intro", ["see"]),
  chunk("a", ["see", "type", "drill"], { fix: true }),
  chunk("b", ["see", "type", "drill"], { fix: true }),
  chunk("c", ["see", "type", "drill"]),
  chunk("d", ["see", "type", "drill"], { fix: true }),
  chunk("e", ["see", "type", "drill"]),
];

const bundle: Bundle = {
  version: 1,
  language: "go",
  wrapperLineOffset: 7,
  cityStub: "",
  path: chunks.map((c) => ({ pack: "go-academy", chunk: c.id })),
  packs: [{ id: "go-academy", title: "Go", district: "Go Academy", chunks, missions: [] }],
};

const done = (...ids: string[]) =>
  ids.flatMap((id) => chunks.find((c) => c.id === id)!.steps.map((s) => `${id}:${s}`));

function state(completed: string[], concepts: Record<string, Partial<ConceptState>> = {}): ProgressState {
  return {
    xp: 260,
    streak: { current: 4, best: 6, last_day: "2026-10-23", freezes: 0 },
    concepts: Object.fromEntries(
      Object.entries(concepts).map(([k, v]) => [k, { box_level: 1, due: "2026-10-30", successes: 1, fails: 0, ...v }]),
    ),
    completed,
    mission_encounters: {},
  };
}

const TODAY = "2026-10-24";

describe("buildToday", () => {
  it("buildToday caps warmup at 3", () => {
    const s = state(done("intro", "a", "b", "c", "d"), {
      a: { due: "2026-10-20" },
      b: { due: "2026-10-21" },
      c: { due: "2026-10-22" },
      d: { due: "2026-10-24" },
    });
    const t = buildToday(s, bundle, TODAY);
    expect(WARMUP_MAX).toBe(3);
    expect(t.warmup).toHaveLength(3);
    // One drill per due concept, the most overdue first.
    expect(t.warmup.map((w) => w.concept)).toEqual(["a", "b", "c"]);
    expect(new Set(t.warmup.map((w) => w.chunk)).size).toBe(3);
    for (const w of t.warmup) expect(w.drill).toBeGreaterThanOrEqual(0);
    expect(t.streak).toBe(4);
    expect(t.xp).toBe(260);
    expect(t.level).toBe(2);
    expect(t.next).toEqual({ pack: "go-academy", chunk: "e" });
  });

  it("warmup empty when nothing due", () => {
    const s = state(done("intro", "a", "b"), { a: { due: "2026-10-25" }, b: { due: "2026-11-02" } });
    const t = buildToday(s, bundle, TODAY);
    expect(t.warmup).toEqual([]);
    // Fresh learner too.
    expect(buildToday(state([]), bundle, TODAY).warmup).toEqual([]);
  });

  it("warm-up drills rotate by date, not always drill 0", () => {
    const s = state(done("intro", "a"), { a: { due: "2026-10-01" } });
    const picks = ["2026-10-24", "2026-10-25", "2026-10-26", "2026-10-27", "2026-10-28"].map(
      (d) => buildToday(s, bundle, d).warmup[0].drill,
    );
    expect(new Set(picks).size).toBe(5);
    // Same day, same drill.
    expect(buildToday(s, bundle, "2026-10-24").warmup[0].drill).toBe(picks[0]);
  });

  it("a due concept with no finished drill set is skipped", () => {
    // Concept seen only in a chunk whose drill step isn't done: nothing to recall from.
    const s = state([...done("intro"), "a:see", "a:type"], { a: { due: "2026-10-01" } });
    expect(buildToday(s, bundle, TODAY).warmup).toEqual([]);
  });

  it("fixIt picks most recent completed chunk with an undone fix_it", () => {
    // a, b, c, d done; d is the most recent with a fix_it.
    const s = state(done("intro", "a", "b", "c", "d"));
    expect(buildToday(s, bundle, TODAY).fixIt).toEqual({ pack: "go-academy", chunk: "d" });
    // d's fix-it done: back to b (c has none).
    const s2 = state([...s.completed, `d${FIX_SUFFIX}:drill`]);
    expect(buildToday(s2, bundle, TODAY).fixIt).toEqual({ pack: "go-academy", chunk: "b" });
    // Every fix-it done: nothing.
    const s3 = state([...s2.completed, `b${FIX_SUFFIX}:drill`, `a${FIX_SUFFIX}:drill`]);
    expect(buildToday(s3, bundle, TODAY).fixIt).toBeNull();
    // A chunk not finished yet doesn't count.
    const s4 = state([...done("intro", "a"), "b:see", "b:type"]);
    expect(buildToday(s4, bundle, TODAY).fixIt).toEqual({ pack: "go-academy", chunk: "a" });
  });

  it("toFix counts today's warm-up drills plus the fix-it", () => {
    const s = state(done("intro", "a", "b"), { a: { due: "2026-10-20" } });
    const t = buildToday(s, bundle, TODAY);
    expect(t.warmup).toHaveLength(1);
    expect(t.fixIt).not.toBeNull();
    expect(t.toFix).toBe(2);
    expect(buildToday(state([]), bundle, TODAY).toFix).toBe(0);
  });
});

describe("the day plan", () => {
  it("runs warm-up, then the next chunk, then fix-it, skipping empty parts", () => {
    const s = state(done("intro", "a", "b"), { a: { due: "2026-10-20" } });
    let plan = planDay(buildToday(s, bundle, TODAY), TODAY);
    expect(plan.stage).toBe("warmup");
    expect(plan.chunk).toEqual({ pack: "go-academy", chunk: "c" });
    plan = advanceDay(plan, buildToday(s, bundle, TODAY));
    expect(plan.stage).toBe("chunk");
    // c has no fix_it, so after it the fix-it is b's (the most recent with one).
    const after = state([...s.completed, ...done("c")]);
    plan = advanceDay(plan, buildToday(after, bundle, TODAY));
    expect(plan.stage).toBe("fixit");
    expect(plan.fixIt).toEqual({ pack: "go-academy", chunk: "b" });
    plan = advanceDay(plan, buildToday(after, bundle, TODAY));
    expect(plan.stage).toBe("done");
  });

  it("a fresh learner starts on the next chunk; with nothing to fix the day ends after it", () => {
    const t = buildToday(state([]), bundle, TODAY);
    const plan = planDay(t, TODAY);
    expect(plan.stage).toBe("chunk");
    expect(plan.chunk).toEqual({ pack: "go-academy", chunk: "intro" });
    expect(advanceDay(plan, buildToday(state(done("intro")), bundle, TODAY)).stage).toBe("done");
  });
});

describe("day parts", () => {
  it("before the fix-it part, today's page with a fix-it is named as the one to fix", () => {
    // b is done; the new page d has a fix_it, so that is what comes after it.
    const s = state(done("intro", "a", "b", "c"));
    const t = buildToday(s, bundle, TODAY);
    expect(t.next).toEqual({ pack: "go-academy", chunk: "d" });
    const parts = dayParts(planDay(t, TODAY), t, bundle);
    expect(parts.map((p) => [p.key, p.state])).toEqual([
      ["warmup", "none"],
      ["chunk", "now"],
      ["fixit", "later"],
    ]);
    expect(parts[2].detail).toBe("Title d");
  });
});

describe("skill map", () => {
  it("skill map lists keeps-breaking at fails>=3", () => {
    const s = state(done("intro", "a", "b", "c"), {
      a: { box_level: 5, fails: 0 },
      b: { box_level: 0, fails: 3, due: "2026-10-25" },
      c: { box_level: 2, fails: 2 },
    });
    const m = buildSkillMap(s, bundle, TODAY);
    expect(m.rows.map((r) => [r.concept, r.mastery])).toEqual([
      ["a", 1],
      ["b", 0],
      ["c", 0.4],
    ]);
    expect(m.rows[0].title).toBe("Title a");
    expect(m.keepsBreaking.map((r) => r.concept)).toEqual(["b"]);
    expect(m.keepsBreaking[0].practise).toMatchObject({ pack: "go-academy", chunk: "b", concept: "b" });
    expect(buildSkillMap(state([]), bundle, TODAY).rows).toEqual([]);
  });
});
