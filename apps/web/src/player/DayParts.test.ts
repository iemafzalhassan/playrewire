// @vitest-environment jsdom
// The warm-up and the break-it/fix-it task: real core rules, a fake Go runner.
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeAll, describe, expect, it, vi } from "vitest";
import type { Chunk } from "../content/types";
import { initCore, type ProgressState, type StepRecord } from "../core";
import type { RawRunResult } from "../lab/protocol";
import { localDate } from "./drill";
import FixIt from "./FixIt.svelte";
import WarmUp from "./WarmUp.svelte";

beforeAll(async () => {
  await initCore();
});

const ok = (stdout: string): RawRunResult => ({ stdout, stderr: "", exitCode: 0, timedOut: false, truncated: false, durationMs: 5 });

const mk = (id: string, concept: string, outs: string[]): Chunk => ({
  id,
  concept,
  title: `Title ${id}`,
  see: { scene: id, caption: id },
  drills: outs.map((o, n) => ({ prompt: `${id} drill ${n}`, solution: `fmt.Println("${o}")`, expectStdout: `${o}\n`, wholeFile: false })),
  hints: [`${id} hint 1`, `${id} hint 2`, `${id} hint 3`],
  fixIt: { broken: `fmt.Println("${id}`, solution: `fmt.Println("${id}")`, expectStdout: `${id}\n`, wholeFile: false },
  pack: "go-academy",
  steps: ["see", "type", "drill"],
});
const a = mk("a", "ca", ["a0", "a1", "a2", "a3", "a4"]);
const b = mk("b", "cb", ["b0", "b1", "b2", "b3", "b4"]);

const progress = (): ProgressState => ({
  xp: 100,
  streak: { current: 1, best: 1, last_day: null, freezes: 0 },
  concepts: {
    ca: { box_level: 2, due: "2000-01-01", successes: 2, fails: 0 },
    cb: { box_level: 1, due: "2000-01-01", successes: 1, fails: 0 },
  },
  completed: [...a.steps.map((s) => `a:${s}`), ...b.steps.map((s) => `b:${s}`)],
  mission_encounters: {},
});

/** Answers each run with whatever the current drill expects, unless told to fail. */
function runner(answer: () => string) {
  return vi.fn(async () => ok(answer()));
}

const backend = (run: ReturnType<typeof runner>) => {
  const saved: StepRecord[] = [];
  return {
    saved,
    kind: "web" as const,
    run,
    saveProgress: vi.fn(async (_s: ProgressState, rec: StepRecord) => {
      saved.push(rec);
    }),
    mentorAvailable: () => false,
    mentorHint: vi.fn(),
  };
};

describe("WarmUp", () => {
  it("one quick pass per due concept, each saved as a drill on its chunk", async () => {
    let want = "a2\n";
    const be = backend(runner(() => want));
    const onDone = vi.fn();
    const onProgress = vi.fn();
    render(WarmUp, {
      props: {
        items: [
          { pack: "go-academy", chunk: "a", drill: 2, concept: "ca" },
          { pack: "go-academy", chunk: "b", drill: 4, concept: "cb" },
        ],
        chunks: [a, b],
        progress: progress(),
        backend: be,
        onProgress,
        onDone,
        doneLabel: "next: new page",
      },
    });
    expect(screen.getByText(/1 of 2/)).toBeTruthy();
    expect(screen.getByText("a drill 2")).toBeTruthy();
    expect(screen.getByText(/about a minute/i)).toBeTruthy();
    expect(screen.getByText("AI mentor is available in the desktop app.")).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
    await waitFor(() => expect(be.saved).toHaveLength(1));
    expect(be.saved[0]).toEqual({
      chunk_id: "a",
      concept: "ca",
      step: "drill",
      mission_id: null,
      success: true,
      first_try: true,
      hints_used: 0,
      local_date: localDate(),
    });
    await fireEvent.click(screen.getByRole("button", { name: /next warm-up/i }));
    expect(screen.getByText(/2 of 2/)).toBeTruthy();
    expect(screen.getByText("b drill 4")).toBeTruthy();

    // A miss first: the slip is recorded once, then the pass.
    want = "nope\n";
    await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
    await waitFor(() => expect(be.saved).toHaveLength(2));
    expect(be.saved[1]).toMatchObject({ chunk_id: "b", concept: "cb", step: "drill", success: false });
    await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
    await new Promise((r) => setTimeout(r, 10));
    expect(be.saved).toHaveLength(2);
    want = "b4\n";
    await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
    await waitFor(() => expect(be.saved).toHaveLength(3));
    expect(be.saved[2]).toMatchObject({ chunk_id: "b", success: true, first_try: false });
    // The state passed on carries the fail.
    const last = onProgress.mock.calls.at(-1)![0] as ProgressState;
    expect(last.concepts.cb.fails).toBe(1);

    await fireEvent.click(screen.getByRole("button", { name: /next: new page/i }));
    expect(onDone).toHaveBeenCalledOnce();
  });
});

describe("FixIt", () => {
  it("fix-it starts from the broken code and records a #fix drill step", async () => {
    const be = backend(runner(() => "a\n"));
    const onDone = vi.fn();
    render(FixIt, {
      props: { chunk: a, progress: progress(), backend: be, onProgress: vi.fn(), onDone, doneLabel: "finish the day" },
    });
    expect(screen.getByText(/break it, fix it/i)).toBeTruthy();
    // The editor opens on the broken code; the output it should print is shown.
    expect(screen.getByRole("textbox", { name: /your go code/i }).textContent).toContain('fmt.Println("a');
    expect(screen.getByTestId("fix-want").textContent).toContain("a");
    // Built-in hints come from the chunk.
    expect(screen.getByRole("button", { name: /hint \(3 left\)/i })).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
    await waitFor(() => expect(be.saved).toHaveLength(1));
    expect(be.saved[0]).toEqual({
      chunk_id: "a#fix",
      concept: "ca",
      step: "drill",
      mission_id: null,
      success: true,
      first_try: true,
      hints_used: 0,
      local_date: localDate(),
    });
    await fireEvent.click(screen.getByRole("button", { name: /finish the day/i }));
    expect(onDone).toHaveBeenCalledOnce();
  });

  it("fix-it miss records a failed drill on <id>#fix", async () => {
    let out = "still broken\n";
    const be = backend(runner(() => out));
    const onProgress = vi.fn();
    render(FixIt, { props: { chunk: a, progress: progress(), backend: be, onProgress, onDone: vi.fn() } });
    await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
    await waitFor(() => expect(be.saved).toHaveLength(1));
    expect(be.saved[0]).toEqual({
      chunk_id: "a#fix",
      concept: "ca",
      step: "drill",
      mission_id: null,
      success: false,
      first_try: false,
      hints_used: 0,
      local_date: localDate(),
    });
    // A second miss isn't saved again; the fix is.
    await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
    await new Promise((r) => setTimeout(r, 10));
    expect(be.saved).toHaveLength(1);
    out = "a\n";
    await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
    await waitFor(() => expect(be.saved).toHaveLength(2));
    expect(be.saved[1]).toMatchObject({ chunk_id: "a#fix", success: true, first_try: false });
    const last = onProgress.mock.calls.at(-1)![0] as ProgressState;
    expect(last.concepts.ca.fails).toBe(1);
    expect(last.completed).toContain("a#fix:drill");
  });
});
