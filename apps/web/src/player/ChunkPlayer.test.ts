// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { emptyProgress } from "../backend/types";
import type { Chunk } from "../content/types";
import { initCore, type ProgressState } from "../core";
import type { RawRunResult } from "../lab/protocol";
import { EditorView } from "@codemirror/view";
import ChunkPlayer from "./ChunkPlayer.svelte";
import { localDate } from "./drill";

// The Use step needs the lesson bundle (mission + city stub).
vi.mock("../content/load", () => ({
  loadTrouble: () => ({ title: "x", text: "x" }),
  loadBundle: async () => ({
    cityStub: "package main\n",
    packs: [
      {
        id: "go-academy",
        missions: [
          {
            id: "green-wave",
            concept: "for-loop",
            terminal: "dock-st-traffic",
            title: "Turn Dock St green, one light every 2 seconds",
            setup: "",
            pack: "go-academy",
            levels: {
              fillBlank: { template: "SetGreen(0, ___)", solution: "", wholeFile: false },
              halfFunction: { template: "", solution: "", wholeFile: false },
              blankWithHints: { template: "", solution: "", wholeFile: false },
            },
            expectEvents: [{ op: "green", i: 0, delay: 0 }],
          },
        ],
      },
    ],
  }),
}));

beforeAll(async () => {
  await initCore();
});

const base = {
  concept: "print",
  title: "Your first words",
  see: { scene: "print-say", caption: "A program is a list of steps. Println says something." },
  drills: [],
  hints: [],
  pack: "go-academy",
};

function setup(
  chunk: Chunk,
  progress: ProgressState = emptyProgress(),
  run: (files: unknown) => Promise<RawRunResult> = vi.fn(),
  saveDelay = 0,
) {
  const saveProgress = vi.fn(async () => {
    if (saveDelay) await new Promise((r) => setTimeout(r, saveDelay));
  });
  const onProgress = vi.fn();
  const backend = { kind: "web" as const, run, saveProgress };
  render(ChunkPlayer, { props: { chunk, progress, backend, onProgress, onHome: vi.fn(), warm: false } });
  return { saveProgress, onProgress };
}

const ok = (stdout: string): RawRunResult => ({ stdout, stderr: "", exitCode: 0, timedOut: false, truncated: false, durationMs: 5 });
const bad = (stderr: string): RawRunResult => ({ stdout: "", stderr, exitCode: 1, timedOut: false, truncated: false, durationMs: 5 });
const drillChunk: Chunk = {
  ...base,
  id: "hello-print",
  steps: ["drill"],
  drills: [1, 2, 3, 4, 5].map((n) => ({ prompt: `Print ${n}.`, solution: `fmt.Println(${n})`, expectStdout: `${n}\n`, wholeFile: false })),
  hints: ["a", "b", "c"],
};
/** Answers with the current drill's expected output, or a wrong one. */
function answering(right: () => boolean) {
  return vi.fn(async () => {
    const n = /Print (\d)\./.exec(document.querySelector(".prompt-text")!.textContent!)![1];
    return ok(right() ? `${n}\n` : "nope\n");
  });
}
const recs = (save: ReturnType<typeof vi.fn>) => save.mock.calls.map((c) => (c as unknown[])[1] as Record<string, unknown>);
async function runOnce(run: ReturnType<typeof vi.fn>, calls: number) {
  await fireEvent.click(await screen.findByRole("button", { name: /^run/i }));
  await waitFor(() => expect(run).toHaveBeenCalledTimes(calls));
  await waitFor(() => expect(screen.getByTestId("drill-result").textContent).not.toBe(""));
}

describe("ChunkPlayer", () => {
  it("records each finished step with the local date, then shows the reward", async () => {
    const chunk: Chunk = { ...base, id: "hello-print", steps: ["see", "type"], type: { ghost: "ab\n", tips: [], wholeFile: false } };
    const { saveProgress, onProgress } = setup(chunk);

    expect(screen.getByTestId("caption").textContent).toContain("list of steps.");
    await fireEvent.click(screen.getByRole("button", { name: /next/i }));
    await waitFor(() => expect(saveProgress).toHaveBeenCalledOnce());
    const [state, rec] = saveProgress.mock.calls[0] as unknown as [ProgressState, unknown];
    expect(rec).toEqual({
      chunk_id: "hello-print",
      concept: "print",
      step: "see",
      mission_id: null,
      success: true,
      first_try: true,
      hints_used: 0,
      local_date: localDate(),
    });
    expect(state.completed).toContain("hello-print:see");
    expect(onProgress).toHaveBeenCalledWith(state);
    expect((await screen.findByTestId("xp-float")).textContent).toContain("+5 xp");

    // Type is next
    const area = await screen.findByRole("textbox", { name: /type the code/i });
    await fireEvent.keyDown(area, { key: "a" });
    await fireEvent.keyDown(area, { key: "b" });
    // the last step's button says finish
    await fireEvent.click(await screen.findByRole("button", { name: /finish/i }));
    await waitFor(() => expect(saveProgress).toHaveBeenCalledTimes(2));
    expect((saveProgress.mock.calls[1] as unknown[])[1]).toMatchObject({ step: "type", first_try: true });
    expect(await screen.findByText(/page done/i)).toBeTruthy();
  });

  it("starts at the first step not done yet: the street, with the mission's terminal", async () => {
    const chunk: Chunk = {
      ...base,
      id: "for-loop",
      mission: "green-wave",
      steps: ["see", "use"],
      see: { scene: "lamp-street", caption: "Loops repeat." },
    };
    const progress = { ...emptyProgress(), completed: ["for-loop:see"] };
    setup(chunk, progress);
    expect(await screen.findByText("light up the city")).toBeTruthy();
    expect(screen.getByRole("list", { name: /steps/i }).textContent).toContain("street");
    // No WebGL2 in jsdom: the sketch street stands in, and the terminal is one press away.
    expect(await screen.findByRole("img", { name: /dock st/i })).toBeTruthy();
    expect(screen.getAllByText("Turn Dock St green, one light every 2 seconds").length).toBeGreaterThan(0);
    await fireEvent.click(screen.getByRole("button", { name: /go to the terminal/i }));
    expect(await screen.findByRole("textbox", { name: /your go code/i })).toBeTruthy();
  });

  it("page drill saves one failure per round", async () => {
    const run = answering(() => false);
    const { saveProgress } = setup(drillChunk, emptyProgress(), run);
    await runOnce(run, 1);
    await runOnce(run, 2);
    await waitFor(() => expect(saveProgress).toHaveBeenCalledOnce());
    expect(recs(saveProgress)[0]).toEqual({
      chunk_id: "hello-print",
      concept: "print",
      step: "drill",
      mission_id: null,
      success: false,
      first_try: false,
      hints_used: 0,
      local_date: localDate(),
    });
    // A new round, a new first miss.
    await fireEvent.click(screen.getByRole("button", { name: /try a different one/i }));
    await runOnce(run, 3);
    await runOnce(run, 4);
    await waitFor(() => expect(saveProgress).toHaveBeenCalledTimes(2));
    await new Promise((r) => setTimeout(r, 20));
    expect(saveProgress).toHaveBeenCalledTimes(2);
    expect(recs(saveProgress).every((r) => r.success === false)).toBe(true);
  });

  it("failure then pass in quick succession keeps both", async () => {
    let right = false;
    const run = answering(() => right);
    // Slow saves: the failure is still being written while the passes come in.
    const { saveProgress, onProgress } = setup(drillChunk, emptyProgress(), run, 40);
    await runOnce(run, 1);
    right = true;
    await runOnce(run, 2);
    for (let i = 3; i <= 4; i++) {
      await fireEvent.click(screen.getByRole("button", { name: /next drill/i }));
      await runOnce(run, i);
    }
    await waitFor(() => expect(saveProgress).toHaveBeenCalledTimes(2));
    expect(recs(saveProgress).map((r) => r.success)).toEqual([false, true]);
    const last = onProgress.mock.calls.at(-1)![0] as ProgressState;
    expect(last.concepts.print.fails).toBe(1);
    expect(last.concepts.print.successes).toBe(1);
    expect(last.completed).toContain("hello-print:drill");
  });

  it("use step saves a failure with its mission id", async () => {
    const chunk: Chunk = { ...base, id: "for-loop", concept: "for-loop", mission: "green-wave", steps: ["see", "use"], hints: ["a", "b", "c"] };
    const run = vi.fn(async () => bad("8:5: undefined: SetGren\n"));
    const { saveProgress } = setup(chunk, { ...emptyProgress(), completed: ["for-loop:see"] }, run);
    await fireEvent.click(await screen.findByRole("button", { name: /go to the terminal/i }));
    await screen.findByRole("textbox", { name: /your go code/i });
    const view = EditorView.findFromDOM(document.querySelector(".cm-editor") as HTMLElement)!;
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: "SetGren(0, 0)" } });
    await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
    await waitFor(() => expect(saveProgress).toHaveBeenCalledOnce());
    expect(recs(saveProgress)[0]).toEqual({
      chunk_id: "for-loop",
      concept: "for-loop",
      step: "use",
      mission_id: "green-wave",
      success: false,
      first_try: false,
      hints_used: 0,
      local_date: localDate(),
    });
  });
});
