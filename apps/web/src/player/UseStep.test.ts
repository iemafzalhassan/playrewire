// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { emptyProgress } from "../backend/types";
import type { Mission } from "../content/types";
import { initCore, type ProgressState } from "../core";
import type { RawRunResult } from "../lab/protocol";
import { FakeWorld } from "../world/fake";
import { EditorView } from "@codemirror/view";
import UseStep from "./UseStep.svelte";

/** Replace the editor's text, as typing would. */
function setCode(text: string) {
  const view = EditorView.findFromDOM(document.querySelector(".cm-editor") as HTMLElement)!;
  view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } });
}

beforeAll(async () => {
  await initCore();
});

const greenWave: Mission = {
  id: "green-wave",
  concept: "for-loop",
  terminal: "dock-st-traffic",
  title: "Turn Dock St green, one light every 2 seconds",
  setup: "package main\n\n// Dock St has 5 traffic lights, numbered 0 to 4.\n",
  pack: "go-academy",
  levels: {
    fillBlank: {
      template: "for i := 0; i < 5; i++ {\n    SetGreen(i, ___)\n}\n",
      solution: "for i := 0; i < 5; i++ {\n    SetGreen(i, i*2)\n}\n",
      wholeFile: false,
    },
    halfFunction: {
      template: "// Wrap a loop around this line.\nSetGreen(i, i*2)\n",
      solution: "for i := 0; i < 5; i++ {\n    SetGreen(i, i*2)\n}\n",
      wholeFile: false,
    },
    blankWithHints: {
      template: "",
      solution: "for i := 0; i < 5; i++ {\n    SetGreen(i, i*2)\n}\n",
      wholeFile: false,
    },
  },
  expectEvents: [0, 1, 2, 3, 4].map((i) => ({ op: "green" as const, i, delay: i * 2 })),
};

const hints = ["A loop starts with for.", "Start at 0, stop before 5.", "Use i inside."];
const ok = (stdout: string): RawRunResult => ({ stdout, stderr: "", exitCode: 0, timedOut: false, truncated: false, durationMs: 5 });
const city = (delays: number[]) => delays.map((d, i) => `@city {"op":"green","i":${i},"delay":${d}}\n`).join("");

/** A 3D street whose wasm trapped: the chosen calls throw from then on. */
class CrashingWorld extends FakeWorld {
  constructor(private crashOn: "applyEvents" | "highlight") {
    super();
  }
  override applyEvents(json: string) {
    if (this.crashOn === "applyEvents") throw new Error("RuntimeError: unreachable");
    super.applyEvents(json);
  }
  override highlight(t: Parameters<FakeWorld["highlight"]>[0]) {
    if (this.crashOn === "highlight" && t === null && this.mission) throw new Error("RuntimeError: unreachable");
    super.highlight(t);
  }
}

function setup(encounters: number, results: (RawRunResult | Error)[] = [], onMiss = vi.fn(), world = new FakeWorld()) {
  const progress: ProgressState = { ...emptyProgress(), mission_encounters: encounters ? { "green-wave": encounters } : {} };
  const run = vi.fn(async () => {
    const r = results.shift() ?? ok("");
    if (r instanceof Error) throw r;
    return r;
  });
  const onDone = vi.fn();
  render(UseStep, {
    props: {
      mission: greenWave,
      cityStub: "package main\n// city stub\n",
      progress,
      hints,
      run,
      onDone,
      onMiss,
      openWorld: async () => world,
    },
  });
  return { world, run, onDone, onMiss };
}

/** Walk up and press E: the fake world says the player used the terminal. */
async function useTerminal(world: FakeWorld) {
  await waitFor(() => expect(world.mission).toBe("green-wave"));
  world.useTerminal();
  return screen.findByRole("textbox", { name: /your go code/i });
}

describe("UseStep", () => {
  it("UseStep picks template by help level", async () => {
    // First encounter: fill in one blank.
    const first = setup(0);
    const editor = await useTerminal(first.world);
    expect(editor.textContent).toContain("___");
    expect(screen.queryByRole("button", { name: /hint/i })).toBeNull();
    expect(first.world.inputEnabled).toBe(false);
  });

  it("third encounter: an empty editor, the title up front, and a hint button", async () => {
    const third = setup(2);
    const editor = await useTerminal(third.world);
    expect(editor.textContent?.trim()).toBe("");
    expect(screen.getAllByText(greenWave.title).length).toBeGreaterThan(0);
    const hint = screen.getByRole("button", { name: /hint/i });
    await fireEvent.click(hint);
    expect(screen.getByText(hints[0])).toBeTruthy();
  });

  it("wrong events show mismatch caption", async () => {
    const { world, run, onDone } = setup(0, [ok("warming up\n" + city([0, 3, 4, 6, 8]))]);
    await useTerminal(world);
    await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
    // A blank left in: no run, a friendly nudge.
    expect(await screen.findByText(/fill in the blank/i)).toBeTruthy();
    expect(run).not.toHaveBeenCalled();

    setCode("for i := 0; i < 5; i++ {\n    SetGreen(i, i*3)\n}\n");
    await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
    await waitFor(() => expect(run).toHaveBeenCalledOnce());
    const files = (run.mock.calls[0] as unknown as [{ name: string; source: string }[]])[0];
    expect(files.map((f) => f.name)).toEqual(["city.go", "setup.go", "main.go"]);
    expect(files[2].source).toContain("func main() {\nfor i := 0; i < 5; i++ {");

    const caption = await screen.findByText("Light 1 turned green at the wrong time.");
    expect(caption.closest("[role=status],[aria-live]")).toBeTruthy();
    expect(world.highlighted).toEqual({ kind: "light", i: 1 });
    expect(world.applied).toHaveLength(5);
    // The program's own words, without the @city lines.
    expect(screen.getByTestId("use-stdout").textContent).toBe("warming up");
    expect(screen.getByTestId("use-result").textContent).toContain("✗");
    expect(onDone).not.toHaveBeenCalled();
  });

  it("a right run lights the street, says so with ✓ and records the step", async () => {
    const { world, onDone } = setup(1, [ok(city([0, 2, 4, 6, 8]))]);
    await useTerminal(world);
    setCode(greenWave.levels.fillBlank.solution);
    await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
    await waitFor(() => expect(onDone).toHaveBeenCalledWith({ firstTry: true, hintsUsed: 0 }));
    expect(screen.getByTestId("use-result").textContent).toContain("✓");
    expect(world.highlighted).toBeNull();
  });

  it("no events: says which function to call", async () => {
    const { world } = setup(0, [ok("hi\n")]);
    await useTerminal(world);
    setCode('fmt.Println("hi")');
    await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
    expect(await screen.findByText("Nothing lit up. Did your code call SetGreen?")).toBeTruthy();
  });

  it("keyboard path: the go-to-the-terminal button walks there", async () => {
    const { world } = setup(0);
    await waitFor(() => expect(world.mission).toBe("green-wave"));
    await fireEvent.click(screen.getByRole("button", { name: /go to the terminal/i }));
    expect(await screen.findByRole("textbox", { name: /your go code/i })).toBeTruthy();
  });

  it("falls back to the sketch street when the 3D world can't start", async () => {
    const progress = emptyProgress();
    render(UseStep, {
      props: { mission: greenWave, cityStub: "", progress, hints, run: vi.fn(), onDone: vi.fn(), openWorld: async () => null },
    });
    expect(await screen.findByRole("img", { name: /dock st/i })).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: /go to the terminal/i }));
    expect(await screen.findByRole("textbox", { name: /your go code/i })).toBeTruthy();
  });

  it("use step saves one failure on wrong events", async () => {
    // No events, then wrong events: one miss for the attempt; the blank nudge isn't one.
    const { world, run, onMiss } = setup(0, [ok("hi\n"), ok(city([0, 3, 4, 6, 8]))]);
    await useTerminal(world);
    await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
    expect(await screen.findByText(/fill in the blank/i)).toBeTruthy();
    expect(onMiss).not.toHaveBeenCalled();
    setCode('fmt.Println("hi")');
    await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
    expect(await screen.findByText("Nothing lit up. Did your code call SetGreen?")).toBeTruthy();
    expect(onMiss).toHaveBeenCalledExactlyOnceWith({ hintsUsed: 0 });
    setCode("for i := 0; i < 5; i++ {\n    SetGreen(i, i*3)\n}\n");
    await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
    expect(await screen.findByText("Light 1 turned green at the wrong time.")).toBeTruthy();
    expect(run).toHaveBeenCalledTimes(2);
    expect(onMiss).toHaveBeenCalledOnce();
  });

  it("lab start failure in the street is not recorded as a miss", async () => {
    const { world, onMiss } = setup(1, [new Error("ipc gone")]);
    await useTerminal(world);
    setCode(greenWave.levels.fillBlank.solution);
    await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
    await screen.findByText(/not yet/i);
    expect(onMiss).not.toHaveBeenCalled();
  });

  describe("when the 3D street crashes", () => {
    beforeEach(() => {
      document.documentElement.dataset.motion = "reduced";
    });
    afterEach(() => {
      delete document.documentElement.dataset.motion;
    });

    it("a crash while playing finishes the run on the sketch street", async () => {
      const crashing = new CrashingWorld("applyEvents");
      const { onDone } = setup(1, [ok(city([0, 2, 4, 6, 8]))], vi.fn(), crashing);
      await useTerminal(crashing);
      setCode(greenWave.levels.fillBlank.solution);
      await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
      await waitFor(() => expect(onDone).toHaveBeenCalledWith({ firstTry: true, hintsUsed: 0 }));
      expect(screen.getByRole("img", { name: /dock st/i })).toBeTruthy();
      expect(screen.getByTestId("use-result").textContent).toContain("✓");
    });

    it("the run button comes back after a crash on a wrong run", async () => {
      const crashing = new CrashingWorld("applyEvents");
      setup(1, [ok(city([0, 3, 4, 6, 8]))], vi.fn(), crashing);
      await useTerminal(crashing);
      setCode("for i := 0; i < 5; i++ {\n    SetGreen(i, i*3)\n}\n");
      await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
      expect(await screen.findByText("Light 1 turned green at the wrong time.")).toBeTruthy();
      expect(screen.getByRole("img", { name: /dock st/i })).toBeTruthy();
      const again = screen.getByRole("button", { name: /^run/i }) as HTMLButtonElement;
      expect(again.disabled).toBe(false);
      expect(again.textContent).not.toContain("running");
    });

    it("a crash before the run starts still runs the code", async () => {
      const crashing = new CrashingWorld("highlight");
      const { run, onDone } = setup(1, [ok(city([0, 2, 4, 6, 8]))], vi.fn(), crashing);
      await useTerminal(crashing);
      setCode(greenWave.levels.fillBlank.solution);
      await fireEvent.click(screen.getByRole("button", { name: /^run/i }));
      await waitFor(() => expect(onDone).toHaveBeenCalledOnce());
      expect(run).toHaveBeenCalledOnce();
      expect(screen.getByRole("img", { name: /dock st/i })).toBeTruthy();
    });

    it("a street that reports it stopped swaps to the sketch", async () => {
      const failing = new FakeWorld();
      setup(0, [], vi.fn(), failing);
      await waitFor(() => expect(failing.mission).toBe("green-wave"));
      failing.fail();
      expect(await screen.findByRole("img", { name: /dock st/i })).toBeTruthy();
      await fireEvent.click(screen.getByRole("button", { name: /go to the terminal/i }));
      expect(await screen.findByRole("textbox", { name: /your go code/i })).toBeTruthy();
    });
  });
});
