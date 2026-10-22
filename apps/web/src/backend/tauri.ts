// Desktop backend: every call goes to a Tauri command (implemented in Task 15,
// apps/desktop/src-tauri). Argument keys are camelCase here; Tauri maps them
// to the snake_case Rust parameters (presetId -> preset_id).
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
import { Channel, invoke } from "@tauri-apps/api/core";
import type { AgUiEvent, Backend, Settings } from "./types";
import type { ProgressState, RawRunResult } from "./types";

export function createTauriBackend(): Backend {
  // mentorAvailable() is synchronous, so remember the last settings we saw.
  let mentorOn = false;
  const remember = (s: Settings) => {
    mentorOn = s.mentor !== null;
    return s;
  };

  return {
    kind: "desktop",
    run: (files) => invoke<RawRunResult>("run_code", { files }),
    loadProgress: () => invoke<ProgressState>("load_progress"),
    async saveProgress(state, rec) {
      await invoke("save_progress", { state, rec });
    },
    toolchain: () => invoke<{ status: "ready" | "missing" }>("toolchain_status"),
    async installToolchain(onProgress) {
      const ch = new Channel<{ done: number; total: number }>();
      ch.onmessage = (m) => onProgress(m.done, m.total);
      await invoke("install_toolchain", { onProgress: ch });
    },
    mentorAvailable: () => mentorOn,
    async mentorHint(req, onEvent) {
      const ch = new Channel<AgUiEvent>();
      ch.onmessage = onEvent;
      await invoke("mentor_hint", { req, onEvent: ch });
    },
    getSettings: () => invoke<Settings>("get_settings").then(remember),
    async saveSettings(settings) {
      await invoke("save_settings", { settings });
      remember(settings);
    },
    async setApiKey(presetId, key) {
      await invoke("set_api_key", { presetId, key });
    },
  };
}
