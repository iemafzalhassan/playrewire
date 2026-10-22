// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { initCore } from "../core";
import type { RawRunResult } from "../lab/protocol";
import { BackendError } from "../backend/errors";
import { TOOLCHAIN_CONTEXT, createToolchain } from "../lab/toolchain.svelte";
import DrillStep from "./DrillStep.svelte";

beforeAll(async () => {
  await initCore();
});

const ok = (stdout: string): RawRunResult => ({ stdout, stderr: "", exitCode: 0, timedOut: false, truncated: false, durationMs: 5 });
const bad = (stderr: string): RawRunResult => ({ stdout: "", stderr, exitCode: 1, timedOut: false, truncated: false, durationMs: 5 });

const drill = (n: number) => ({
  prompt: `Print ${n}.`,
  starter: `fmt.Println(${n})`,
  solution: `fmt.Println(${n})`,
  expectStdout: `${n}\n`,
  wholeFile: false,
});
const drills = [1, 2, 3, 4, 5].map(drill);
const hints = ["first hint", "second hint", "third hint"];

function setup(results: RawRunResult[]) {
  const run = vi.fn(async () => results.shift() ?? ok(""));
  const onDone = vi.fn();
  render(DrillStep, { props: { drills, hints, run, onDone } });
  return { run, onDone };
}
const runBtn = () => screen.getByRole("button", { name: /^run/i });

describe("DrillStep", () => {
  it("DrillStep shows friendly card above raw error", async () => {
    const { run } = setup([bad("8:13: undefined: x\n")]);
    await fireEvent.click(runBtn());
    const title = await screen.findByText("Unknown name");
    expect(run).toHaveBeenCalledWith([{ name: "main.go", source: expect.stringContaining("func main() {\nfmt.Println(1)\n}") }]);

    const card = title.closest(".error-card") as HTMLElement;
    expect(card).toBeTruthy();
    expect(card.textContent).toMatch(/Go doesn't know what/);
    expect(card.textContent).toMatch(/line 1/);
    const details = card.querySelector("details")!;
    expect(details.open).toBe(false);
    expect(details.textContent).toContain("8:13: undefined: x");
    // friendly words first, the raw message after
    expect(title.compareDocumentPosition(details) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("pass/fail shows ✓/✗ glyphs not just color", async () => {
    setup([ok("7\n"), ok("1\n")]);
    await fireEvent.click(runBtn());
    const fail = await screen.findByTestId("drill-result");
    expect(fail.textContent).toContain("✗");
    expect(fail.textContent).not.toContain("✓");
    await fireEvent.click(runBtn());
    await waitFor(() => expect(screen.getByTestId("drill-result").textContent).toContain("✓"));
    expect(screen.getByTestId("drill-result").textContent).toContain("1");
  });

  it("three passes in a row complete the step; a fail resets the run", async () => {
    const { onDone } = setup([
      bad("8:1: undefined: y\n"),
      ok("1\n"),
      bad("8:1: undefined: y\n"),
      ok("2\n"),
      ok("3\n"),
      ok("4\n"),
    ]);
    const passes = () => screen.getByTestId("passes").textContent;

    await fireEvent.click(runBtn()); // the first run fails, so this is not a first try
    await screen.findByText("Unknown name");
    await fireEvent.click(runBtn()); // pass drill 1
    await screen.findByRole("button", { name: /next drill/i });
    expect(passes()).toMatch(/1 of 3/);
    await fireEvent.click(screen.getByRole("button", { name: /next drill/i }));
    expect(screen.getByText("Print 2.")).toBeTruthy();

    await fireEvent.click(runBtn()); // fail: the run of passes resets
    await screen.findByText("Unknown name");
    expect(passes()).toMatch(/0 of 3/);

    await fireEvent.click(runBtn()); // pass drill 2
    await fireEvent.click(await screen.findByRole("button", { name: /next drill/i }));
    await fireEvent.click(runBtn()); // pass drill 3
    await fireEvent.click(await screen.findByRole("button", { name: /next drill/i }));
    expect(onDone).not.toHaveBeenCalled();
    await fireEvent.click(runBtn()); // pass drill 4: three in a row
    await waitFor(() => expect(onDone).toHaveBeenCalledOnce());
    expect(onDone).toHaveBeenCalledWith({ hintsUsed: 0, firstTry: false });
  });

  it("hints come one at a time and are counted; a clean first run is first_try", async () => {
    const { onDone } = setup([ok("1\n"), ok("2\n"), ok("3\n")]);
    await fireEvent.click(runBtn());
    await fireEvent.click(await screen.findByRole("button", { name: /next drill/i }));
    const hint = () => screen.getByRole("button", { name: /hint/i });
    await fireEvent.click(hint());
    expect(screen.getByText("first hint")).toBeTruthy();
    expect(screen.queryByText("second hint")).toBeNull();
    await fireEvent.click(hint());
    expect(screen.getByText("second hint")).toBeTruthy();
    await fireEvent.click(runBtn());
    await fireEvent.click(await screen.findByRole("button", { name: /next drill/i }));
    await fireEvent.click(runBtn());
    await waitFor(() => expect(onDone).toHaveBeenCalledWith({ hintsUsed: 2, firstTry: true }));
  });

  it("switching drills mid-run discards the old result", async () => {
    let release: (r: RawRunResult) => void = () => {};
    let releaseB: (r: RawRunResult) => void = () => {};
    const results: (RawRunResult | Promise<RawRunResult>)[] = [
      bad("8:1: undefined: y\n"),
      bad("8:1: undefined: y\n"),
      new Promise<RawRunResult>((r) => (release = r)),
      new Promise<RawRunResult>((r) => (releaseB = r)),
    ];
    const run = vi.fn(async () => results.shift() ?? ok(""));
    const onDone = vi.fn();
    render(DrillStep, { props: { drills, hints, run, onDone } });
    await fireEvent.click(runBtn());
    await screen.findByText("Unknown name");
    await fireEvent.click(runBtn());
    await waitFor(() => expect(run).toHaveBeenCalledTimes(2));
    const other = await screen.findByRole("button", { name: /try a different one/i });

    await fireEvent.click(runBtn()); // this run hangs
    await waitFor(() => expect(other).toHaveProperty("disabled", true));
    // a click that slipped in before the button was disabled still switches drills
    other.removeAttribute("disabled");
    await fireEvent.click(other);
    expect(screen.getByText("Print 2.")).toBeTruthy();
    expect(runBtn()).toHaveProperty("disabled", false); // the new drill can run straight away

    await fireEvent.click(runBtn()); // drill 2's run is now in flight
    await waitFor(() => expect(run).toHaveBeenCalledTimes(4));
    expect(runBtn().textContent).toMatch(/running/);

    release(ok("1\n")); // drill 1's answer arrives late
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.getByTestId("drill-result").textContent).not.toContain("✓");
    expect(screen.getByTestId("passes").textContent).toMatch(/0 of 3/);
    expect(onDone).not.toHaveBeenCalled();
    // the late answer must not re-enable run while drill 2 is still running
    expect(runBtn()).toHaveProperty("disabled", true);
    expect(runBtn().textContent).toMatch(/running/);

    releaseB(ok("2\n"));
    await waitFor(() => expect(screen.getByTestId("drill-result").textContent).toContain("✓"));
    expect(screen.getByTestId("passes").textContent).toMatch(/1 of 3/);
  });

  it("a lab that fails to start does not cost the first try", async () => {
    const { onDone } = setup([
      { ...bad("could not start the code runner: Error: worker error\n") },
      ok("1\n"),
      ok("2\n"),
      ok("3\n"),
    ]);
    await fireEvent.click(runBtn());
    expect(await screen.findByText("The Go lab didn't start")).toBeTruthy();
    for (let i = 0; i < 2; i++) {
      await fireEvent.click(runBtn());
      await fireEvent.click(await screen.findByRole("button", { name: /next drill/i }));
    }
    await fireEvent.click(runBtn());
    await waitFor(() => expect(onDone).toHaveBeenCalledWith({ hintsUsed: 0, firstTry: true }));
  });

  it("toolchain_missing on run shows the Go download card, not a code error", async () => {
    const run = vi.fn(async () => {
      throw new BackendError("toolchain_missing");
    });
    const tc = createToolchain({ toolchain: async () => ({ status: "ready" }), installToolchain: async () => {} });
    render(DrillStep, {
      props: { drills, hints, run, onDone: vi.fn() },
      context: new Map([[TOOLCHAIN_CONTEXT, tc]]),
    });
    await fireEvent.click(runBtn());
    const card = await screen.findByTestId("toolchain-card");
    expect(card.textContent).toMatch(/download Go once \(about 70 MB\)/);
    expect(tc.phase).toBe("missing");
    expect(screen.getByTestId("passes").textContent).toContain("0 of 3");
  });

  it("sandbox_unavailable shows the Linux sandbox words", async () => {
    const run = vi.fn(async () => {
      throw new BackendError("sandbox_unavailable");
    });
    render(DrillStep, { props: { drills, hints, run, onDone: vi.fn() } });
    await fireEvent.click(runBtn());
    expect(
      await screen.findByText("Rewire couldn't start its safe practice space on this system. See docs: Linux sandbox setup."),
    ).toBeTruthy();
  });
});
