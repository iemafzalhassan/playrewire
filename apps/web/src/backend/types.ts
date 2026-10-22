// The one seam between the UI and the platform. The web app and the Tauri
// desktop app share every screen; only the Backend differs.
import type { RawRunResult, SourceFile } from "../lab/protocol";
import type { ProgressState, StepRecord } from "../core";

export type { ProgressState, RawRunResult, SourceFile, StepRecord };

/** Mirrors rewire-mentor's HintRequest (snake_case on the wire). Only these fields ever leave the device. */
export interface HintRequest {
  concept: string;
  exercise_prompt: string;
  code: string;
  last_error: string | null;
  attempt: number;
}

/** Mirrors rewire-mentor's ProviderConfig. The key is never part of it (OS keychain only). */
export interface ProviderConfig {
  preset_id: string;
  base_url: string;
  model: string;
}

/** AG-UI events from rewire-mentor (`{"type":"RUN_STARTED","threadId":..}`). */
export type AgUiEvent =
  | { type: "RUN_STARTED"; threadId: string; runId: string }
  | { type: "TEXT_MESSAGE_START"; messageId: string; role: string }
  | { type: "TEXT_MESSAGE_CONTENT"; messageId: string; delta: string }
  | { type: "TEXT_MESSAGE_END"; messageId: string }
  | { type: "RUN_FINISHED"; threadId: string; runId: string }
  | { type: "RUN_ERROR"; message: string };

export type TextScale = 1 | 1.25 | 1.5;

export interface Settings {
  /** 'system' follows prefers-reduced-motion. */
  reducedMotion: boolean | "system";
  dyslexiaFont: boolean;
  textScale: TextScale;
  mentor: ProviderConfig | null;
}

export type ToolchainStatus = "ready" | "missing" | "n/a";

export interface Backend {
  kind: "web" | "desktop";
  run(files: SourceFile[]): Promise<RawRunResult>;
  loadProgress(): Promise<ProgressState>;
  saveProgress(state: ProgressState, rec: StepRecord): Promise<void>;
  toolchain(): Promise<{ status: ToolchainStatus }>;
  installToolchain(onProgress: (done: number, total: number) => void): Promise<void>;
  mentorAvailable(): boolean;
  mentorHint(req: HintRequest, onEvent: (e: AgUiEvent) => void): Promise<void>;
  getSettings(): Promise<Settings>;
  saveSettings(s: Settings): Promise<void>;
  setApiKey(presetId: string, key: string): Promise<void>;
}

export const DEFAULT_SETTINGS: Settings = {
  reducedMotion: "system",
  dyslexiaFont: false,
  textScale: 1,
  mentor: null,
};

export function emptyProgress(): ProgressState {
  return {
    xp: 0,
    streak: { current: 0, best: 0, last_day: null, freezes: 0 },
    concepts: {},
    completed: [],
    mission_encounters: {},
  };
}

/** Same words as rewire-mentor's MSG_FALLBACK: the UI shows a built-in hint next. */
export const MENTOR_FALLBACK = "The mentor couldn't connect, so here's a built-in hint.";
