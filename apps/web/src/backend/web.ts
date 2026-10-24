// Web backend: code runs in the browser (Yaegi wasm worker), progress and
// settings live in localStorage. Every storage access is wrapped: private
// windows and blocked site data can throw on the getter itself. When that
// happens we keep state in memory and say so once.
import "../zod-config";
import { z } from "zod";
import { webRun } from "../lab/webRunner";
import type { ProgressState, StepRecord } from "../core";
import {
  DEFAULT_SETTINGS,
  MENTOR_FALLBACK,
  emptyProgress,
  type Backend,
  type Settings,
} from "./types";

export const PROGRESS_KEY = "rewire.progress.v1";
export const SETTINGS_KEY = "rewire.settings.v1";
/** Step records waiting for Phase 2 sync (spec 7.8), newest last. */
export const OUTBOX_KEY = "rewire.outbox.v1";
export const OUTBOX_MAX = 500;
export const STORAGE_NOTICE = "Progress won't be saved in this window.";

/** Where a stored progress value the schema rejects is kept, so nothing is lost. */
export const CORRUPT_PROGRESS_PREFIX = "rewire.progress.corrupt.";

// The same bounds as the Rust types (rewire-core types.rs), so anything that
// loads here also loads in applyStep. u64 stops at what a JS number holds exactly.
const u8 = z.number().int().min(0).max(255);
const u32 = z.number().int().min(0).max(4_294_967_295);
const u64 = z.number().int().min(0).max(Number.MAX_SAFE_INTEGER);
/** chrono's NaiveDate as serde writes it: YYYY-MM-DD (with a sign for far years). */
const date = z.string().regex(/^[+-]?\d{4,6}-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])$/);

const ProgressSchema = z.object({
  xp: u64,
  streak: z.object({
    current: u32,
    best: u32,
    last_day: date.nullable(),
    freezes: u8,
  }),
  concepts: z.record(
    z.string(),
    z.object({
      box_level: z.number().int().min(0).max(5),
      due: date,
      successes: u32,
      fails: u32,
    }),
  ),
  completed: z.array(z.string()),
  mission_encounters: z.record(z.string(), u32),
});

const SettingsSchema = z.object({
  reducedMotion: z.union([z.boolean(), z.literal("system")]),
  dyslexiaFont: z.boolean(),
  textScale: z.union([z.literal(1), z.literal(1.25), z.literal(1.5)]),
  mentor: z
    .object({ preset_id: z.string(), base_url: z.string(), model: z.string() })
    .nullable(),
});

export interface WebBackendOptions {
  /** Called once if storage can't be used in this window. */
  onNotice?: (text: string) => void;
}

export interface WebBackend extends Backend {
  /** The step records in the local outbox, oldest first. */
  pendingOutbox(): StepRecord[];
}

export function createWebBackend(opts: WebBackendOptions = {}): WebBackend {
  const memory = new Map<string, string>();
  let durable = true;

  function lost() {
    if (!durable) return;
    durable = false;
    opts.onNotice?.(STORAGE_NOTICE);
  }

  function read(key: string): string | null {
    if (durable) {
      try {
        return globalThis.localStorage.getItem(key);
      } catch {
        lost();
      }
    }
    return memory.get(key) ?? null;
  }

  function write(key: string, value: string) {
    memory.set(key, value);
    if (!durable) return;
    try {
      globalThis.localStorage.setItem(key, value);
    } catch {
      lost();
    }
  }

  /** `onReject` sees a stored value that isn't JSON or doesn't fit the schema. */
  function parse<T>(key: string, schema: z.ZodType<T>, fallback: T, onReject?: (raw: string) => void): T {
    const raw = read(key);
    if (raw === null) return fallback;
    try {
      const r = schema.safeParse(JSON.parse(raw));
      if (r.success) return r.data;
    } catch {
      // Not JSON: rejected below.
    }
    onReject?.(raw);
    return fallback;
  }

  function backUpProgress(raw: string) {
    console.warn("stored progress didn't load; keeping a copy and starting fresh");
    write(`${CORRUPT_PROGRESS_PREFIX}${Date.now()}`, raw);
  }

  // Validated loosely: the outbox is only ever written by saveProgress below.
  const OutboxSchema = z.array(z.looseObject({ chunk_id: z.string(), step: z.string() }));
  const readOutbox = () => parse(OUTBOX_KEY, OutboxSchema, []) as unknown as StepRecord[];

  return {
    kind: "web",
    pendingOutbox: readOutbox,
    run: (files) => webRun(files),
    async loadProgress() {
      return parse<ProgressState>(PROGRESS_KEY, ProgressSchema, emptyProgress(), backUpProgress);
    },
    async saveProgress(state, rec) {
      write(PROGRESS_KEY, JSON.stringify(state));
      // Every finished step also goes to the local outbox for Phase 2 sync,
      // like the desktop SQLite outbox. Capped so storage can't fill up.
      write(OUTBOX_KEY, JSON.stringify([...readOutbox(), rec].slice(-OUTBOX_MAX)));
    },
    async toolchain() {
      return { status: "n/a" };
    },
    async installToolchain() {
      // Nothing to install: the browser runs Go through the wasm lab.
    },
    mentorAvailable: () => false,
    async mentorHint(_req, onEvent) {
      onEvent({ type: "RUN_ERROR", message: MENTOR_FALLBACK });
    },
    async getSettings() {
      return parse<Settings>(SETTINGS_KEY, SettingsSchema, { ...DEFAULT_SETTINGS });
    },
    async saveSettings(s) {
      write(SETTINGS_KEY, JSON.stringify(s));
    },
    async setApiKey() {
      throw new Error("The mentor needs the desktop app.");
    },
    async getPresets() {
      return [];
    },
  };
}
