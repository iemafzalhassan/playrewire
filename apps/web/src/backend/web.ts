// Web backend: code runs in the browser (Yaegi wasm worker), progress and
// settings live in localStorage. Every storage access is wrapped: private
// windows and blocked site data can throw on the getter itself. When that
// happens we keep state in memory and say so once.
import "../zod-config";
import { z } from "zod";
import { webRun } from "../lab/webRunner";
import type { ProgressState } from "../core";
import {
  DEFAULT_SETTINGS,
  MENTOR_FALLBACK,
  emptyProgress,
  type Backend,
  type Settings,
} from "./types";

export const PROGRESS_KEY = "rewire.progress.v1";
export const SETTINGS_KEY = "rewire.settings.v1";
export const STORAGE_NOTICE = "Progress won't be saved in this window.";

const ProgressSchema = z.object({
  xp: z.number().int().nonnegative(),
  streak: z.object({
    current: z.number().int().nonnegative(),
    best: z.number().int().nonnegative(),
    last_day: z.string().nullable(),
    freezes: z.number().int().nonnegative(),
  }),
  concepts: z.record(
    z.string(),
    z.object({
      box_level: z.number().int(),
      due: z.string(),
      successes: z.number().int(),
      fails: z.number().int(),
    }),
  ),
  completed: z.array(z.string()),
  mission_encounters: z.record(z.string(), z.number().int()),
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

export function createWebBackend(opts: WebBackendOptions = {}): Backend {
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

  function parse<T>(key: string, schema: z.ZodType<T>, fallback: T): T {
    const raw = read(key);
    if (raw === null) return fallback;
    try {
      const r = schema.safeParse(JSON.parse(raw));
      return r.success ? r.data : fallback;
    } catch {
      return fallback;
    }
  }

  return {
    kind: "web",
    run: (files) => webRun(files),
    async loadProgress() {
      return parse<ProgressState>(PROGRESS_KEY, ProgressSchema, emptyProgress());
    },
    async saveProgress(state) {
      // The step record matters on desktop (SQLite outbox). The web MVP keeps state only.
      write(PROGRESS_KEY, JSON.stringify(state));
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
  };
}
