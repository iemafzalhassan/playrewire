// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { emptyProgress } from "../backend/types";
import type { Chunk } from "../content/types";
import { initCore, type ProgressState } from "../core";
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

function setup(chunk: Chunk, progress: ProgressState = emptyProgress()) {
  const saveProgress = vi.fn(async () => {});
  const onProgress = vi.fn();
  const backend = { kind: "web" as const, run: vi.fn(), saveProgress };
  render(ChunkPlayer, { props: { chunk, progress, backend, onProgress, onHome: vi.fn(), warm: false } });
  return { saveProgress, onProgress };
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
});
