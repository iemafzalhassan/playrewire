import { describe, expect, it } from "vitest";
import type { CityEvent, Mission } from "../content/types";
import { missionCaption, missionFiles, templateFor } from "./mission";

const green = (i: number, delay: number): CityEvent => ({ op: "green", i, delay });
const lamp = (i: number, on: boolean): CityEvent => ({ op: "lamp", i, on });
const wave = [0, 1, 2, 3, 4].map((i) => green(i, i * 2));
const thirteen = [false, false, false, false, true, true, false, true].map((on, i) => lamp(i, on));

const lv = (template: string) => ({ template, solution: "x", wholeFile: false });
const mission = (expectEvents: CityEvent[]): Mission => ({
  id: "m",
  concept: "c",
  terminal: "t",
  title: "T",
  setup: "",
  pack: "p",
  levels: { fillBlank: lv("a ___"), halfFunction: lv("half"), blankWithHints: lv("") },
  expectEvents,
});

describe("missionCaption", () => {
  const cap = (expected: CityEvent[], actual: CityEvent[], first: number) =>
    missionCaption(mission(expected), { kind: "wrong_events", first_mismatch: first }, actual);

  it("a light at the wrong time", () => {
    const actual = wave.map((e, i) => (i === 3 ? green(3, 5) : e));
    expect(cap(wave, actual, 3)).toEqual({ text: "Light 3 turned green at the wrong time.", target: { kind: "light", i: 3 } });
  });
  it("a light out of turn, a light missing, one too many, one that isn't there", () => {
    expect(cap(wave, [green(1, 0)], 0)?.text).toBe("Light 1 went green when it was light 0's turn.");
    expect(cap(wave, wave.slice(0, 3), 3)).toEqual({ text: "Light 3 never turned green.", target: { kind: "light", i: 3 } });
    expect(cap(wave, [...wave, green(2, 9)], 5)?.text).toBe("Light 2 turned green one time too many.");
    expect(cap(wave, [...wave.slice(0, 4), green(5, 8)], 4)).toEqual({
      text: "There's no light 5. Dock St has lights 0 to 4.",
      target: null,
    });
  });
  it("lamps: should be on, should be off, skipped, not there", () => {
    const flip = (k: number) => thirteen.map((e, i) => (i === k && e.op === "lamp" ? lamp(i, !e.on) : e));
    expect(cap(thirteen, flip(5), 5)).toEqual({ text: "Lamp 5 should be on.", target: { kind: "lamp", i: 5 } });
    expect(cap(thirteen, flip(2), 2)).toEqual({ text: "Lamp 2 should be off.", target: { kind: "lamp", i: 2 } });
    expect(cap(thirteen, thirteen.slice(0, 6), 6)?.text).toBe("Lamp 6 never got set.");
    expect(cap(thirteen, [...thirteen.slice(0, 7), lamp(8, true)], 7)?.text).toBe("There's no lamp 8. The bridge has lamps 0 to 7.");
  });
  it("no events names the function to call", () => {
    expect(missionCaption(mission(wave), { kind: "no_events" }, [])?.text).toBe("Nothing lit up. Did your code call SetGreen?");
    expect(missionCaption(mission(thirteen), { kind: "no_events" }, [])?.text).toBe("Nothing lit up. Did your code call Lamp?");
    expect(missionCaption(mission(wave), { kind: "pass" }, wave)).toBeNull();
  });
});

describe("missionFiles and templates", () => {
  it("city.go, setup.go, then the wrapped main.go (R9); whole-file code runs unwrapped (R14)", () => {
    const { files, lineOffset } = missionFiles("CITY", "", "SetGreen(0, 0)", false);
    expect(files.map((f) => f.name)).toEqual(["city.go", "setup.go", "main.go"]);
    expect(files[1].source).toBe("package main\n");
    expect(files[2].source).toContain("func main() {\nSetGreen(0, 0)\n}");
    expect(lineOffset).toBe(7);
    const whole = missionFiles("CITY", "package main\n", "package main\nfunc main() {}\n", true);
    expect(whole.files[2].source).toBe("package main\nfunc main() {}\n");
    expect(whole.lineOffset).toBe(0);
  });
  it("help level picks the template", () => {
    const m = mission(wave);
    expect(templateFor(m, "fill_blank").template).toContain("___");
    expect(templateFor(m, "half_function").template).toBe("half");
    expect(templateFor(m, "blank_with_hints").template).toBe("");
  });
});
