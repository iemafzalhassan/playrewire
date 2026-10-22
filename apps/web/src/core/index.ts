// Thin typed wrapper over rewire-core-wasm (JSON in, JSON out). The same Rust
// rules run on desktop and web. Field names stay snake_case: that is the
// persistence format (see crates/rewire-core/src/types.rs).
import initWasm, * as wasm from "./pkg/rewire_core_wasm.js";
import type { CityEvent, Step } from "../content/types";

export type { CityEvent, Step };

export type HelpLevel = "fill_blank" | "half_function" | "blank_with_hints";

export interface StepRecord {
  chunk_id: string;
  concept: string;
  step: Step;
  mission_id: string | null;
  success: boolean;
  first_try: boolean;
  hints_used: number;
  /** The player's local day, YYYY-MM-DD. */
  local_date: string;
}

export interface StreakState {
  current: number;
  best: number;
  last_day: string | null;
  freezes: number;
}

export interface ConceptState {
  box_level: number;
  due: string;
  successes: number;
  fails: number;
}

export interface ProgressState {
  xp: number;
  streak: StreakState;
  concepts: Record<string, ConceptState>;
  /** "{chunk_id}:{step}" */
  completed: string[];
  mission_encounters: Record<string, number>;
}

export type Reward =
  | { kind: "xp"; value: number }
  | { kind: "streak_extended"; value: number }
  | { kind: "freeze_used" }
  | { kind: "freeze_earned" }
  | { kind: "level_up"; value: number };

export interface FriendlyError {
  line: number | null;
  col: number | null;
  title: string;
  plain: string;
  raw: string;
}

export type MissionVerdict =
  | { kind: "pass" }
  | { kind: "no_events" }
  | { kind: "wrong_events"; first_mismatch: number };

let ready = false;

/**
 * Load the wasm once before calling anything else. Browser: fetches the file
 * next to the glue code. Node (Vitest): reads it from disk.
 */
export async function initCore(): Promise<void> {
  if (ready) return;
  const isNode =
    typeof process !== "undefined" && !!process.versions?.node && typeof window === "undefined";
  if (isNode) {
    const spec = "node:fs"; // variable specifier keeps bundlers from pulling it into the web build
    const { readFileSync } = (await import(/* @vite-ignore */ spec)) as typeof import("node:fs");
    wasm.initSync({
      module: readFileSync(new URL("./pkg/rewire_core_wasm_bg.wasm", import.meta.url)),
    });
  } else {
    await initWasm();
  }
  ready = true;
}

function need(): typeof wasm {
  if (!ready) throw new Error("rewire-core is not loaded: await initCore() first");
  return wasm;
}

export function applyStep(
  state: ProgressState,
  rec: StepRecord,
): { state: ProgressState; rewards: Reward[] } {
  return JSON.parse(need().apply_step(JSON.stringify(state), JSON.stringify(rec)));
}

export function dueConcepts(state: ProgressState, today: string): string[] {
  return JSON.parse(need().due_concepts_json(JSON.stringify(state), today));
}

export function helpLevel(state: ProgressState, missionId: string): HelpLevel {
  return JSON.parse(need().help_level_json(JSON.stringify(state), missionId));
}

export function level(xp: number): number {
  return need().level_of(xp);
}

export function explainGoErrors(
  stderr: string,
  lineOffset: number,
  timedOut: boolean,
  truncated: boolean,
): FriendlyError[] {
  return JSON.parse(need().explain_errors_json(stderr, lineOffset, timedOut, truncated));
}

export function parseCityOutput(stdout: string): { stdout: string; events: CityEvent[] } {
  return JSON.parse(need().parse_city_output_json(stdout));
}

export function checkMission(expected: CityEvent[], actual: CityEvent[]): MissionVerdict {
  return JSON.parse(need().check_mission_json(JSON.stringify(expected), JSON.stringify(actual)));
}
