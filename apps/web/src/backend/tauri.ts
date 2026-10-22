// Desktop backend: every call goes to a Tauri command (apps/desktop/src-tauri).
// Argument keys are camelCase here; Tauri maps them to the snake_case Rust
// parameters (presetId -> preset_id).
//
// Command contract:
//   run_code({files: SourceFile[]})            -> RawRunResult (camelCase fields)
//   load_progress()                            -> ProgressState (snake_case fields)
//   save_progress({state, rec})                -> ()
//   toolchain_status()                         -> {status: 'ready' | 'missing'}
//   install_toolchain({onProgress: Channel<{done, total}>}) -> ()
//   mentor_hint({req, onEvent: Channel<AgUiEvent>})          -> ()
//   get_settings()                             -> Settings
//   save_settings({settings})                  -> ()
//   set_api_key({presetId, key})               -> ()  (key goes to the OS keychain)
//   get_presets()                              -> Preset[]
//
// A failing command rejects with a stable code string (see ./errors.ts); every
// call here turns that into a BackendError whose message is friendly copy.
import { Channel, invoke } from "@tauri-apps/api/core";
import { toBackendError } from "./errors";
import type { AgUiEvent, Backend, Preset, Settings } from "./types";
import type { ProgressState, RawRunResult } from "./types";

function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(cmd, args).catch((e: unknown) => {
    throw toBackendError(e);
  });
}

export function createTauriBackend(): Backend {
  // mentorAvailable() is synchronous, so remember the last settings we saw.
  let mentorOn = false;
  const remember = (s: Settings) => {
    mentorOn = s.mentor !== null;
    return s;
  };

  return {
    kind: "desktop",
    run: (files) => call<RawRunResult>("run_code", { files }),
    loadProgress: () => call<ProgressState>("load_progress"),
    async saveProgress(state, rec) {
      await call("save_progress", { state, rec });
    },
    toolchain: () => call<{ status: "ready" | "missing" }>("toolchain_status"),
    async installToolchain(onProgress) {
      const ch = new Channel<{ done: number; total: number }>();
      ch.onmessage = (m) => onProgress(m.done, m.total);
      await call("install_toolchain", { onProgress: ch });
    },
    mentorAvailable: () => mentorOn,
    async mentorHint(req, onEvent) {
      const ch = new Channel<AgUiEvent>();
      ch.onmessage = onEvent;
      await call("mentor_hint", { req, onEvent: ch });
    },
    getSettings: () => call<Settings>("get_settings").then(remember),
    async saveSettings(settings) {
      await call("save_settings", { settings });
      remember(settings);
    },
    async setApiKey(presetId, key) {
      await call("set_api_key", { presetId, key });
    },
    getPresets: () => call<Preset[]>("get_presets"),
  };
}
