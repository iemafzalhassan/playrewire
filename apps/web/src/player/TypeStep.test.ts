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
const typed = () => document.querySelectorAll(".c.done").length;
function beforeInput(el: HTMLElement, inputType: string, data: string | null = null) {
  return createEvent("beforeinput", el, { inputType, data, bubbles: true, cancelable: true }, { EventType: "InputEvent" });
}

describe("TypeStep", () => {
  it("TypeStep cancels paste and shows the hint", async () => {
    render(TypeStep, { props: { step, onDone: vi.fn() } });
    const area = screen.getByRole("textbox", { name: /type the code/i });
    expect(area.tagName).toBe("INPUT");
    expect(area.getAttribute("aria-describedby")).toBeTruthy();
    expect(document.activeElement).toBe(area);
    for (const [k, v] of Object.entries({ autocapitalize: "off", autocorrect: "off", spellcheck: "false", autocomplete: "off", inputmode: "text" }))
      expect(area.getAttribute(k), k).toBe(v);
    expect(area.getAttribute("type")).toBe("text");
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

    // nothing was typed by the paste
    expect(typed()).toBe(0);
  });

  it("typing the ghost finishes the step, with tips under their match", async () => {
    const onDone = vi.fn();
    render(TypeStep, { props: { step, onDone } });
    const area = screen.getByRole("textbox", { name: /type the code/i });
    expect(screen.getByText("A new box called x.").closest(".tip")?.classList.contains("shown")).toBe(false);

    press(area, [...'fmt.Println("hi")']);
    await fireEvent.keyDown(area, { key: "q" });
    expect(document.querySelector(".c.bad")).toBeTruthy();
    expect(screen.getByText(/backspace/i, { selector: ".status *" })).toBeTruthy();
    await fireEvent.keyDown(area, { key: "Backspace" });
    expect(document.querySelector(".c.bad")).toBeNull();

    press(area, ["Enter"]);
    await fireEvent.keyDown(area, { key: "x" });
    expect(screen.getByText("A new box called x.").closest(".tip")?.classList.contains("shown")).toBe(true);
    press(area, [..." := 1"]);
    const next = await screen.findByRole("button", { name: /next/i });
    expect(screen.getByText(/✓/)).toBeTruthy();
    await fireEvent.click(next);
    expect(onDone).toHaveBeenCalledWith({ mistakes: 1 });
  });

  it("insertFromPaste beforeinput is cancelled with the hint", async () => {
    render(TypeStep, { props: { step, onDone: vi.fn() } });
    const area = screen.getByRole("textbox", { name: /type the code/i });
    for (const kind of ["insertFromPaste", "insertFromDrop", "insertReplacementText"]) {
      const e = beforeInput(area, kind, "fmt");
      await fireEvent(area, e);
      expect(e.defaultPrevented, kind).toBe(true);
    }
    expect(await screen.findByText(HINT)).toBeTruthy();
    expect(typed()).toBe(0);
  });

  it("mobile insertText input advances the ghost", async () => {
    render(TypeStep, { props: { step, onDone: vi.fn() } });
    const area = screen.getByRole("textbox", { name: /type the code/i }) as HTMLInputElement;
    // phone keyboards send keydown "Unidentified", then the text as input
    await fireEvent.keyDown(area, { key: "Unidentified" });
    const e = beforeInput(area, "insertText", "fm");
    await fireEvent(area, e);
    expect(e.defaultPrevented).toBe(true);
    expect(typed()).toBe(2);
    // an IME that can't be cancelled lands in the value; it is read and cleared
    const ime = beforeInput(area, "insertCompositionText", "t");
    await fireEvent(area, ime);
    expect(ime.defaultPrevented).toBe(false);
    area.value = "t";
    await fireEvent.input(area);
    expect(typed()).toBe(3);
    expect(area.value).toBe("");
    await fireEvent(area, beforeInput(area, "deleteContentBackward"));
    expect(typed()).toBe(2);
    await fireEvent(area, beforeInput(area, "insertText", 't.Println("hi")'));
    await fireEvent(area, beforeInput(area, "insertLineBreak"));
    expect(typed()).toBe('fmt.Println("hi")\n'.length);
  });

  it("middle-click paste is cancelled", async () => {
    render(TypeStep, { props: { step, onDone: vi.fn() } });
    const area = screen.getByRole("textbox", { name: /type the code/i });
    for (const make of [createEvent.mouseDown, createEvent.mouseUp, createEvent("auxclick", area, { button: 1, bubbles: true, cancelable: true }, { EventType: "MouseEvent" })]) {
      const e = typeof make === "function" ? make(area, { button: 1 }) : make;
      await fireEvent(area, e);
      expect(e.defaultPrevented, e.type).toBe(true);
    }
    // an ordinary left click is left alone
    const left = createEvent.mouseUp(area, { button: 0 });
    await fireEvent(area, left);
    expect(left.defaultPrevented).toBe(false);
  });

  it("a key is applied once when both keydown and input fire", async () => {
    render(TypeStep, { props: { step, onDone: vi.fn() } });
    const area = screen.getByRole("textbox", { name: /type the code/i }) as HTMLInputElement;
    await fireEvent.keyDown(area, { key: "f" });
    await fireEvent(area, beforeInput(area, "insertText", "f"));
    area.value = "f";
    await fireEvent.input(area, { data: "f", inputType: "insertText" });
    expect(typed()).toBe(1);
    expect(document.querySelector(".c.bad")).toBeNull();
    expect(area.value).toBe("");
    // the next key, from either path, still counts
    await fireEvent(area, beforeInput(area, "insertText", "m"));
    expect(typed()).toBe(2);
  });
});
