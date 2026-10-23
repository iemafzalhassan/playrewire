// Pure helpers for the Use step: which template the help level opens, the files a run
// sends, and plain-words captions for a mission that went wrong.
import type { CityEvent, Level, Mission } from "../content/types";
import type { HelpLevel, MissionVerdict } from "../core";
import type { SourceFile } from "../lab/protocol";
import type { Target } from "../world/world";
import { wrapMain } from "./drill";

const LIGHTS = 5;
const LAMPS = 8;

export function templateFor(m: Mission, level: HelpLevel): Level {
  return level === "fill_blank" ? m.levels.fillBlank : level === "half_function" ? m.levels.halfFunction : m.levels.blankWithHints;
}

/** Ruling R9: city.go, setup.go, main.go, in that order. An empty setup still declares the package. */
export function missionFiles(
  cityStub: string,
  setup: string,
  code: string,
  wholeFile: boolean,
): { files: SourceFile[]; lineOffset: number } {
  const main = wrapMain(code, wholeFile);
  return {
    files: [
      { name: "city.go", source: cityStub },
      { name: "setup.go", source: setup.trim() ? setup : "package main\n" },
      { name: "main.go", source: main.source },
    ],
    lineOffset: main.lineOffset,
  };
}

/** The fill-in-the-blank marker is still in the code. */
export const hasBlank = (code: string) => code.includes("___");

/** Does this mission light traffic lights or bridge lamps? */
export function missionKind(m: Mission): "light" | "lamp" {
  return m.expectEvents.some((e) => e.op === "lamp") ? "lamp" : "light";
}

export interface Caption {
  text: string;
  /** The light or lamp to ring in ink, when there is one. */
  target: Target | null;
}

/** Plain words for what went wrong, and what to ring. Null for a pass. */
export function missionCaption(m: Mission, verdict: MissionVerdict, actual: CityEvent[]): Caption | null {
  const kind = missionKind(m);
  if (verdict.kind === "pass") return null;
  if (verdict.kind === "no_events") {
    return { text: `Nothing lit up. Did your code call ${kind === "light" ? "SetGreen" : "Lamp"}?`, target: null };
  }
  const k = verdict.first_mismatch;
  const want = m.expectEvents[k];
  const got = actual[k];
  if (kind === "light") {
    const ring = (i: number): Target | null => (i < LIGHTS ? { kind: "light", i } : null);
    if (got?.op === "green" && got.i >= LIGHTS) {
      return { text: `There's no light ${got.i}. Dock St has lights 0 to ${LIGHTS - 1}.`, target: null };
    }
    if (!got && want?.op === "green") return { text: `Light ${want.i} never turned green.`, target: ring(want.i) };
    if (got?.op === "green" && !want) return { text: `Light ${got.i} turned green one time too many.`, target: ring(got.i) };
    if (got?.op === "green" && want?.op === "green") {
      if (got.i === want.i) return { text: `Light ${got.i} turned green at the wrong time.`, target: ring(got.i) };
      return { text: `Light ${got.i} went green when it was light ${want.i}'s turn.`, target: ring(got.i) };
    }
    if (want?.op === "green") return { text: `Light ${want.i} never got its turn.`, target: ring(want.i) };
    return { text: "The street did something unexpected. Look at the order of your calls.", target: null };
  }
  const ring = (i: number): Target | null => (i < LAMPS ? { kind: "lamp", i } : null);
  if (got?.op === "lamp" && got.i >= LAMPS) {
    return { text: `There's no lamp ${got.i}. The bridge has lamps 0 to ${LAMPS - 1}.`, target: null };
  }
  if (!got && want?.op === "lamp") return { text: `Lamp ${want.i} never got set.`, target: ring(want.i) };
  if (got?.op === "lamp" && !want) return { text: `Lamp ${got.i} was set one time too many.`, target: ring(got.i) };
  if (got?.op === "lamp" && want?.op === "lamp") {
    if (got.i === want.i) return { text: `Lamp ${want.i} should be ${want.on ? "on" : "off"}.`, target: ring(want.i) };
    return { text: `Lamp ${want.i} got skipped. Go through the lamps in order, 0 to ${LAMPS - 1}.`, target: ring(want.i) };
  }
  if (want?.op === "lamp") return { text: `Lamp ${want.i} never got set.`, target: ring(want.i) };
  return { text: "The bridge did something unexpected. Look at the order of your calls.", target: null };
}
