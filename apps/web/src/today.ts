// The daily loop (spec 4.6): a quick warm-up of what's due, the next new page,
// then a break-it/fix-it task. A 15-20 minute day. Pure functions over the
// progress state and the lesson bundle; the core rules (rewire-core) decide
// what's due and the level.
import type { Bundle, Chunk } from "./content/types";
import { dueConcepts, level, type ProgressState } from "./core";
import { chunkDone, findChunk, nextChunk, type ChunkRef } from "./path";

export type { ChunkRef };

/** One warm-up drill: drill `drill` of chunk `chunk`, which teaches `concept`. */
export interface DrillRef {
  pack: string;
  chunk: string;
  drill: number;
  concept: string;
}

export interface Today {
  warmup: DrillRef[];
  next: ChunkRef | null;
  fixIt: ChunkRef | null;
  streak: number;
  xp: number;
  level: number;
  /** The fixing the day asks for: its warm-up drills plus the fix-it task. */
  toFix: number;
}

/** One drill per due concept, at most three: about a minute (spec 4.6). */
export const WARMUP_MAX = 3;

/** A fix-it is recorded as a `drill` step on `<chunk id>#fix`. */
export const FIX_SUFFIX = "#fix";

export const fixKey = (chunkId: string) => `${chunkId}${FIX_SUFFIX}:drill`;

/** Path chunks in learning order (refs missing from the bundle are skipped). */
function pathChunks(bundle: Bundle): Chunk[] {
  return bundle.path.map((r) => findChunk(bundle, r)).filter((c): c is Chunk => c !== undefined);
}

/** Days since 1970 for a local YYYY-MM-DD; only used to rotate drills. */
function dayNumber(day: string): number {
  const [y, m, d] = day.split("-").map(Number);
  const n = Math.floor(Date.UTC(y, (m || 1) - 1, d || 1) / 86_400_000);
  return Number.isFinite(n) ? n : 0;
}

/** A small stable number for a string, so two concepts don't share a rotation. */
function spread(s: string): number {
  let h = 0;
  for (const ch of s) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
  return h;
}

/**
 * A drill to recall `concept`: from the most recently finished chunk (latest in
 * path order) that teaches it and whose drill step is done. The drill rotates by
 * date, so the same day gives the same drill and the next day a different one.
 */
export function drillFor(state: ProgressState, bundle: Bundle, concept: string, today: string): DrillRef | null {
  const done = new Set(state.completed);
  const source = pathChunks(bundle)
    .filter((c) => c.concept === concept && c.drills.length > 0 && done.has(`${c.id}:drill`))
    .at(-1);
  if (!source) return null;
  const drill = (dayNumber(today) + spread(concept)) % source.drills.length;
  return { pack: source.pack, chunk: source.id, drill, concept };
}

/** The fix-it of the most recently finished chunk that has one not done yet. */
function pickFixIt(state: ProgressState, bundle: Bundle): ChunkRef | null {
  const done = new Set(state.completed);
  const c = pathChunks(bundle)
    .filter((c) => c.fixIt && chunkDone(c, done) && !done.has(fixKey(c.id)))
    .at(-1);
  return c ? { pack: c.pack, chunk: c.id } : null;
}

export function buildToday(state: ProgressState, bundle: Bundle, today: string): Today {
  const warmup: DrillRef[] = [];
  for (const concept of dueConcepts(state, today)) {
    if (warmup.length >= WARMUP_MAX) break;
    const d = drillFor(state, bundle, concept, today);
    if (d) warmup.push(d);
  }
  const fixIt = pickFixIt(state, bundle);
  return {
    warmup,
    next: nextChunk(bundle.path, state, bundle),
    fixIt,
    streak: state.streak.current,
    xp: state.xp,
    level: level(state.xp),
    toFix: warmup.length + (fixIt ? 1 : 0),
  };
}

// ---- The day plan: which of the three parts the learner is on. ----

export type DayStage = "warmup" | "chunk" | "fixit" | "done";

export interface DayPlan {
  /** The learner's local day this plan belongs to. */
  date: string;
  stage: DayStage;
  /** Today's new page, fixed when the day starts. */
  chunk: ChunkRef | null;
  /** Taken when the fix-it part begins, so a page learned today can be the one to fix. */
  fixIt: ChunkRef | null;
  /** Warm-up drills due when the day started (0: the day had no warm-up). */
  warmupCount: number;
}

const ORDER: DayStage[] = ["warmup", "chunk", "fixit", "done"];

/** Moves to `from` or the first later part that has something to do. */
function settle(plan: DayPlan, from: DayStage, t: Today): DayPlan {
  for (const stage of ORDER.slice(ORDER.indexOf(from))) {
    if (stage === "warmup" && t.warmup.length) return { ...plan, stage };
    if (stage === "chunk" && plan.chunk) return { ...plan, stage };
    if (stage === "fixit" && t.fixIt) return { ...plan, stage, fixIt: t.fixIt };
    if (stage === "done") return { ...plan, stage };
  }
  return { ...plan, stage: "done" };
}

/** A new day: starts at the warm-up if anything is due, else the new page, else the fix-it. */
export function planDay(t: Today, date: string): DayPlan {
  return settle({ date, stage: "warmup", chunk: t.next, fixIt: t.fixIt, warmupCount: t.warmup.length }, "warmup", t);
}

/** The current part is finished: on to the next part that has something to do. */
export function advanceDay(plan: DayPlan, t: Today): DayPlan {
  if (plan.stage === "done") return plan;
  return settle(plan, ORDER[ORDER.indexOf(plan.stage) + 1], t);
}

/**
 * Where to pick up (Home's button, a reload): the current part again, unless it
 * has nothing left (warm-up all reviewed, new page already finished elsewhere,
 * fix-it done), in which case the next one.
 */
export function resumeDay(plan: DayPlan, t: Today, state: ProgressState, bundle: Bundle): DayPlan {
  const done = new Set(state.completed);
  let p = plan;
  for (let i = 0; i < ORDER.length; i++) {
    const finished =
      (p.stage === "warmup" && t.warmup.length === 0) ||
      (p.stage === "chunk" && (!p.chunk || chunkFinished(bundle, p.chunk, done))) ||
      (p.stage === "fixit" && (!p.fixIt || done.has(fixKey(p.fixIt.chunk))));
    if (!finished) return p;
    p = advanceDay(p, t);
  }
  return p;
}

function chunkFinished(bundle: Bundle, ref: ChunkRef, done: Set<string>): boolean {
  const c = findChunk(bundle, ref);
  return !c || chunkDone(c, done);
}

export type PartKey = "warmup" | "chunk" | "fixit";

/** One of the day's three parts, as Home and the day tracker show it. */
export interface DayPart {
  key: PartKey;
  label: string;
  detail: string;
  /** none: nothing to do in this part today. */
  state: "done" | "now" | "later" | "none";
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** The three parts in order. With no plan yet (the day hasn't started) everything is ahead. */
export function dayParts(plan: DayPlan | null, t: Today, bundle: Bundle): DayPart[] {
  const at = plan ? ORDER.indexOf(plan.stage) : -1;
  const title = (r: ChunkRef | null) => (r ? (findChunk(bundle, r)?.title ?? null) : null);
  const state = (i: number, has: boolean): DayPart["state"] => {
    if (at > i) return has ? "done" : "none";
    if (at === i) return "now";
    return has ? "later" : "none";
  };

  const warmN = plan && at > 0 ? plan.warmupCount : t.warmup.length;
  const chunk = plan ? plan.chunk : t.next;
  const dayChunk = chunk ? findChunk(bundle, chunk) : undefined;
  // Before the fix-it part, today's new page (if it has a fix-it) is the one that will
  // come: once learned it is the most recent finished page. Otherwise the live pick.
  const ahead = at < 2;
  const fix = ahead ? (dayChunk?.fixIt && chunk ? chunk : t.fixIt) : plan!.fixIt;
  return [
    {
      key: "warmup",
      label: "warm-up",
      detail: warmN ? `${plural(warmN, "quick drill", "quick drills")}, about a minute` : "nothing due today",
      state: state(0, warmN > 0),
    },
    {
      key: "chunk",
      label: "new page",
      detail: title(chunk) ?? "every page is done",
      state: state(1, !!chunk),
    },
    {
      key: "fixit",
      label: "fix-it",
      detail: title(fix) ?? "nothing to fix today",
      state: state(2, !!fix),
    },
  ];
}

// Per-viewer convenience: which part of today's page we're on survives a reload.
// Like every storage access here, it may throw (private windows), so it's wrapped
// and the day simply starts over from the progress state.
export const DAY_KEY = "rewire.today.v1";

export function loadDay(date: string): DayPlan | null {
  try {
    const raw = globalThis.localStorage?.getItem(DAY_KEY);
    if (!raw) return null;
    const p = JSON.parse(raw) as DayPlan;
    const ref = (r: unknown) => r === null || (typeof r === "object" && !!r && "pack" in r && "chunk" in r);
    if (p?.date !== date || !ORDER.includes(p.stage) || !ref(p.chunk) || !ref(p.fixIt)) return null;
    if (typeof p.warmupCount !== "number") p.warmupCount = 0;
    return p;
  } catch {
    return null;
  }
}

export function saveDay(plan: DayPlan) {
  try {
    globalThis.localStorage?.setItem(DAY_KEY, JSON.stringify(plan));
  } catch {
    // Not saved: the day picks up from the progress state next time.
  }
}

// ---- The skill map. ----

export interface SkillRow {
  concept: string;
  title: string;
  /** box_level / 5, from 0 to 1. */
  mastery: number;
  box: number;
  fails: number;
  /** A warm-up drill to practise it, when one is available. */
  practise: DrillRef | null;
}

export interface SkillMapData {
  rows: SkillRow[];
  /** Concepts that failed 3 or more times. */
  keepsBreaking: SkillRow[];
}

export const MAX_BOX = 5;
export const KEEPS_BREAKING_AT = 3;

/** One row per concept seen so far, in learning order. */
export function buildSkillMap(state: ProgressState, bundle: Bundle, today: string): SkillMapData {
  const chunks = pathChunks(bundle);
  const order = (c: string) => {
    const i = chunks.findIndex((x) => x.concept === c);
    return i < 0 ? Number.MAX_SAFE_INTEGER : i;
  };
  const rows = Object.entries(state.concepts)
    .sort(([a], [b]) => order(a) - order(b) || a.localeCompare(b))
    .map(([concept, c]): SkillRow => {
      const box = Math.max(0, Math.min(MAX_BOX, c.box_level));
      return {
        concept,
        title: chunks.find((x) => x.concept === concept)?.title ?? concept,
        mastery: box / MAX_BOX,
        box,
        fails: c.fails,
        practise: drillFor(state, bundle, concept, today),
      };
    });
  return { rows, keepsBreaking: rows.filter((r) => r.fails >= KEEPS_BREAKING_AT) };
}
