import { describe, expect, it } from "vitest";
import type { Bundle, Chunk, Step } from "./content/types";
import type { ProgressState } from "./core";
import { nextChunk } from "./path";

function chunk(pack: string, id: string, steps: Step[]): Chunk {
  return {
    id,
    concept: id,
    title: id,
    see: { scene: id, caption: id },
    drills: [],
    hints: [],
    pack,
    steps,
  };
}

const bundle = {
  version: 1,
  language: "go",
  wrapperLineOffset: 7,
  cityStub: "",
  path: [],
  packs: [
    {
      id: "silicon-quarter",
      title: "How computers work",
      district: "Silicon Quarter",
      chunks: [chunk("silicon-quarter", "what-is-a-computer", ["see"]), chunk("silicon-quarter", "bits", ["see", "type", "drill", "use"])],
      missions: [],
    },
    {
      id: "go-academy",
      title: "Go basics",
      district: "Go Academy",
      chunks: [chunk("go-academy", "hello-print", ["see", "type", "drill"])],
      missions: [],
    },
  ],
} satisfies Bundle;

const path = [
  { pack: "silicon-quarter", chunk: "what-is-a-computer" },
  { pack: "go-academy", chunk: "hello-print" },
  { pack: "silicon-quarter", chunk: "bits" },
];

function state(completed: string[]): ProgressState {
  return {
    xp: 0,
    streak: { current: 0, best: 0, last_day: null, freezes: 0 },
    concepts: {},
    completed,
    mission_encounters: {},
  };
}

describe("nextChunk", () => {
  it("nextChunk returns first incomplete in path order", () => {
    expect(nextChunk(path, state([]), bundle)).toEqual(path[0]);
    // hello-print is half done, bits is untouched: hello-print comes first.
    const s = state(["what-is-a-computer:see", "hello-print:see", "hello-print:type"]);
    expect(nextChunk(path, s, bundle)).toEqual(path[1]);
    const done = state([
      "what-is-a-computer:see",
      "hello-print:see",
      "hello-print:type",
      "hello-print:drill",
      "bits:see",
      "bits:type",
      "bits:drill",
    ]);
    expect(nextChunk(path, done, bundle)).toEqual(path[2]); // bits still needs Use
    expect(nextChunk(path, state([...done.completed, "bits:use"]), bundle)).toBeNull();
  });

  it("nextChunk skips Use for see-only chunks", () => {
    // A see-only chunk is finished after See; it has no Use step to wait for.
    expect(nextChunk(path, state(["what-is-a-computer:see"]), bundle)).toEqual(path[1]);
  });

  it("nextChunk skips refs missing from the bundle", () => {
    const p = [{ pack: "go-academy", chunk: "gone" }, ...path];
    expect(nextChunk(p, state([]), bundle)).toEqual(path[0]);
  });
});
