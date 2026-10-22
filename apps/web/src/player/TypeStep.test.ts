// @vitest-environment jsdom
import { createEvent, fireEvent, render, screen } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import TypeStep from "./TypeStep.svelte";

const step = {
  ghost: 'fmt.Println("hi")\nx := 1\n',
  tips: [{ match: "x := 1", text: "A new box called x." }],
  wholeFile: false,
};
const HINT = "Type it out, it's how your fingers learn.";

function press(el: HTMLElement, keys: string[]) {
  for (const key of keys) fireEvent.keyDown(el, { key });
}

describe("TypeStep", () => {
  it("TypeStep cancels paste and shows the hint", async () => {
    render(TypeStep, { props: { step, onDone: vi.fn() } });
    const area = screen.getByRole("textbox", { name: /type the code/i });
    expect(area.getAttribute("aria-describedby")).toBeTruthy();
    expect(screen.queryByText(HINT)).toBeNull();

    const paste = createEvent.paste(area, { clipboardData: { getData: () => step.ghost } });
    await fireEvent(area, paste);
    expect(paste.defaultPrevented).toBe(true);
    expect(await screen.findByText(HINT)).toBeTruthy();

    // dropping text and the context-menu paste path are cancelled too
    const drop = createEvent.drop(area);
    await fireEvent(area, drop);
    expect(drop.defaultPrevented).toBe(true);
    const menu = createEvent.contextMenu(area);
    await fireEvent(area, menu);
    expect(menu.defaultPrevented).toBe(true);
    const input = createEvent(
      "beforeinput",
      area,
      { inputType: "insertFromPaste", bubbles: true, cancelable: true },
      { EventType: "InputEvent" },
    );
    await fireEvent(area, input);
    expect(input.defaultPrevented).toBe(true);

    // nothing was typed by the paste
    expect(area.querySelectorAll(".c.done")).toHaveLength(0);
  });

  it("typing the ghost finishes the step, with tips under their match", async () => {
    const onDone = vi.fn();
    render(TypeStep, { props: { step, onDone } });
    const area = screen.getByRole("textbox", { name: /type the code/i });
    expect(screen.getByText("A new box called x.").closest(".tip")?.classList.contains("shown")).toBe(false);

    press(area, [...'fmt.Println("hi")']);
    await fireEvent.keyDown(area, { key: "q" });
    expect(area.querySelector(".c.bad")).toBeTruthy();
    expect(screen.getByText(/backspace/i, { selector: ".status *" })).toBeTruthy();
    await fireEvent.keyDown(area, { key: "Backspace" });
    expect(area.querySelector(".c.bad")).toBeNull();

    press(area, ["Enter"]);
    await fireEvent.keyDown(area, { key: "x" });
    expect(screen.getByText("A new box called x.").closest(".tip")?.classList.contains("shown")).toBe(true);
    press(area, [..." := 1"]);
    const next = await screen.findByRole("button", { name: /next/i });
    expect(screen.getByText(/✓/)).toBeTruthy();
    await fireEvent.click(next);
    expect(onDone).toHaveBeenCalledWith({ mistakes: 1 });
  });
});
