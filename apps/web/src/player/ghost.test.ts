import { describe, expect, it } from "vitest";
import { ghostKey, initialGhost, type GhostState } from "./ghost";

const typeAll = (ghost: string, keys: string[], from: GhostState = initialGhost()) =>
  keys.reduce((s, k) => ghostKey(ghost, s, k), from);
const chars = (s: string) => [...s];

describe("ghost typing", () => {
  it("correct keys advance", () => {
    const s = typeAll("fmt.Println(1)", chars("fmt."));
    expect(s).toEqual({ typed: "fmt.", correctUpTo: 4, errorAt: null, done: false });
  });

  it("wrong key blocks until backspace", () => {
    const ghost = "abc";
    let s = typeAll(ghost, ["a", "x"]);
    expect(s.errorAt).toBe(1);
    expect(s.correctUpTo).toBe(1);
    // further keys, even the right one, do nothing until the mistake is fixed
    s = typeAll(ghost, ["b", "c", "Enter", "Tab"], s);
    expect(s.errorAt).toBe(1);
    expect(s.correctUpTo).toBe(1);
    s = ghostKey(ghost, s, "Backspace");
    expect(s).toEqual({ typed: "a", correctUpTo: 1, errorAt: null, done: false });
    s = typeAll(ghost, ["b", "c"], s);
    expect(s.done).toBe(true);
  });

  it("backspace steps back over a correct key", () => {
    const s = typeAll("abc", ["a", "b", "Backspace"]);
    expect(s).toEqual({ typed: "a", correctUpTo: 1, errorAt: null, done: false });
    expect(ghostKey("abc", initialGhost(), "Backspace")).toEqual(initialGhost());
  });

  it("enter skips leading indentation", () => {
    const ghost = "if x {\n    y()\n}";
    let s = typeAll(ghost, chars("if x {"));
    s = ghostKey(ghost, s, "Enter");
    expect(s.correctUpTo).toBe(ghost.indexOf("y"));
    expect(s.errorAt).toBeNull();
    s = typeAll(ghost, [...chars("y()"), "Enter", "}"], s);
    expect(s.done).toBe(true);
    // Enter where the ghost wants a character is a wrong key
    expect(ghostKey(ghost, initialGhost(), "Enter").errorAt).toBe(0);
  });

  it("backspace after enter undoes the skipped indentation too", () => {
    const ghost = "a\n    b";
    const s = typeAll(ghost, ["a", "Enter", "Backspace"]);
    expect(s.correctUpTo).toBe(2); // just after the newline, at the start of the line
  });

  it("tab matches indentation", () => {
    const ghost = "{\n\tx\n    y\n}";
    let s = typeAll(ghost, ["{"]);
    // type the newline without the skip: Enter skips, so step back to the line start
    s = ghostKey(ghost, s, "Enter");
    s = ghostKey(ghost, s, "Backspace");
    expect(s.correctUpTo).toBe(2);
    s = ghostKey(ghost, s, "Tab");
    expect(s.correctUpTo).toBe(3);
    s = typeAll(ghost, ["x", "Enter", "Backspace", "Tab"], s);
    expect(s.correctUpTo).toBe(ghost.indexOf("y"));
    // spaces typed one by one also work
    const t = typeAll("{\n  z", ["{", "Enter", "Backspace", " ", " ", "z"]);
    expect(t.done).toBe(true);
    // Tab where the ghost has no whitespace is a wrong key
    expect(ghostKey("ab", initialGhost(), "Tab").errorAt).toBe(0);
  });

  it("crlf ghost normalized", () => {
    const ghost = "a\r\n  b\r\n";
    const s = typeAll(ghost, ["a", "Enter", "b"]);
    expect(s.done).toBe(true);
    expect(s.typed).toBe("a\n  b");
  });

  it("done at end", () => {
    const ghost = 'fmt.Println("hi")\n';
    let s = typeAll(ghost, chars('fmt.Println("hi"'));
    expect(s.done).toBe(false);
    s = ghostKey(ghost, s, ")");
    // the trailing newline is not part of the exercise
    expect(s).toEqual({ typed: 'fmt.Println("hi")', correctUpTo: 17, errorAt: null, done: true });
    // a finished ghost ignores more keys
    expect(typeAll(ghost, ["x", "Backspace"], s)).toEqual(s);
  });

  it("ignores keys that are not typing", () => {
    const s = typeAll("ab", ["Shift", "ArrowLeft", "Control", "a"]);
    expect(s.correctUpTo).toBe(1);
  });
});
