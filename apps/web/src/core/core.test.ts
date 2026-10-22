import { beforeAll, describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import {
  applyStep, checkMission, dueConcepts, explainGoErrors, helpLevel, initCore, level, parseCityOutput,
  type ProgressState, type Reward, type StepRecord,
} from "./index";

const fixturePath = fileURLToPath(
  new URL("../../../../crates/rewire-core-wasm/tests/fixtures/apply_step.json", import.meta.url),
);
const fx = JSON.parse(readFileSync(fixturePath, "utf8")) as {
  state: ProgressState; record: StepRecord; expected_state: ProgressState; expected_rewards: Reward[];
};

beforeAll(async () => { await initCore(); });

describe("rewire-core via wasm", () => {
  it("applyStep via wasm matches native fixture", () => {
    const out = applyStep(fx.state, fx.record);
    expect(out.state).toEqual(fx.expected_state);
    expect(out.rewards).toEqual(fx.expected_rewards);
  });

  it("explainGoErrors returns Unused box", () => {
    const out = explainGoErrors("# lab\n./main.go:8:2: declared and not used: x\n", 7, false, false);
    expect(out).toHaveLength(1);
    expect(out[0].title).toBe("Unused box");
    expect(out[0].line).toBe(1);
    expect(out[0].raw).toBe("./main.go:8:2: declared and not used: x");
  });

  it("level, dueConcepts, helpLevel", () => {
    expect(level(0)).toBe(1);
    expect(level(250)).toBe(2);
    expect(dueConcepts(fx.expected_state, "2026-03-06")).toEqual(["loops"]);
    expect(dueConcepts(fx.expected_state, "2026-03-05")).toEqual([]);
    expect(helpLevel(fx.expected_state, "m1")).toBe("fill_blank");
  });

  it("parseCityOutput and checkMission", () => {
    const { stdout, events } = parseCityOutput('hi\n@city {"op":"lamp","i":1,"on":true}\n');
    expect(stdout).toBe("hi\n");
    expect(events).toEqual([{ op: "lamp", i: 1, on: true }]);
    expect(checkMission(events, events)).toEqual({ kind: "pass" });
    expect(checkMission(events, [])).toEqual({ kind: "no_events" });
    expect(checkMission(events, [{ op: "say", text: "x" }])).toEqual({ kind: "wrong_events", first_mismatch: 0 });
  });

  it("bad input throws a JS Error, not a wasm trap", () => {
    expect(() => applyStep({} as ProgressState, { nope: 1 } as unknown as StepRecord)).toThrow(/invalid record JSON/);
  });
});
