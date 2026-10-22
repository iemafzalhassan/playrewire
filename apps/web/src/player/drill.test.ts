import { afterEach, describe, expect, it } from "vitest";
import { checkDrill, cpm, localDate, wrapMain } from "./drill";

describe("drill helpers", () => {
  it("checkDrill ignores trailing spaces and crlf", () => {
    expect(checkDrill("a\nb\n", "a  \r\nb\t\r\n")).toEqual({ pass: true, diffLine: null });
    expect(checkDrill("Cars: 3\n", "Cars: 3")).toEqual({ pass: true, diffLine: null });
    expect(checkDrill("x\n", "x\n\n")).toEqual({ pass: true, diffLine: null });
    // leading spaces still count
    expect(checkDrill("x\n", " x\n").pass).toBe(false);
  });

  it("diffLine points at first differing line", () => {
    expect(checkDrill("Ready\nSet\nGo!\n", "Ready\nset\nGo!\n")).toEqual({ pass: false, diffLine: 2 });
    expect(checkDrill("a\nb\n", "a\n")).toEqual({ pass: false, diffLine: 2 });
    expect(checkDrill("a\n", "a\nb\n")).toEqual({ pass: false, diffLine: 2 });
    expect(checkDrill("a\n", "")).toEqual({ pass: false, diffLine: 1 });
  });

  it("wrapMain puts user line 1 on file line 8", () => {
    const w = wrapMain('fmt.Println("hi")\n\n');
    expect(w.lineOffset).toBe(7);
    expect(w.source.split("\n")[7]).toBe('fmt.Println("hi")');
    expect(w.source).toBe(
      'package main\n\nimport "fmt"\n\nvar _ = fmt.Sprint\n\nfunc main() {\nfmt.Println("hi")\n}\n',
    );
  });

  it("wrapMain leaves a whole file alone", () => {
    const file = 'package main\n\nfunc main() {}\n';
    expect(wrapMain(file, true)).toEqual({ source: file, lineOffset: 0 });
  });

  it("cpm counts the characters added in the round", () => {
    expect(cpm("abcdefghij", "", 30_000)).toBe(20);
    expect(cpm("start+more", "start", 60_000)).toBe(5);
    expect(cpm("x", "", 0)).toBe(0);
  });

  describe("localDate", () => {
    const tz = process.env.TZ;
    afterEach(() => {
      process.env.TZ = tz;
    });
    it("localDate uses the local calendar day, not UTC", () => {
      process.env.TZ = "Pacific/Kiritimati"; // UTC+14
      const early = new Date(2026, 9, 22, 5, 0); // 05:00 local is still the 21st in UTC
      expect(early.toISOString().slice(0, 10)).toBe("2026-10-21");
      expect(localDate(early)).toBe("2026-10-22");
      process.env.TZ = "America/Los_Angeles";
      const late = new Date(2026, 0, 3, 23, 30); // 23:30 local is already the 4th in UTC
      expect(localDate(late)).toBe("2026-01-03");
    });
  });
});
