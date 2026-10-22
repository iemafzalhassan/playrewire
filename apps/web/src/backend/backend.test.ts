import { afterEach, describe, expect, it, vi } from "vitest";
import type { AgUiEvent } from "./types";

const PROGRESS_KEY = "rewire.progress.v1";
const SETTINGS_KEY = "rewire.settings.v1";

const rec = {
  chunk_id: "hello-print",
  concept: "print",
  step: "see" as const,
  mission_id: null,
  success: true,
  first_try: true,
  hints_used: 0,
  local_date: "2026-10-22",
};

function memoryStorage(): Storage {
  const m = new Map<string, string>();
  return {
    get length() {
      return m.size;
    },
    clear: () => m.clear(),
    getItem: (k) => m.get(k) ?? null,
    key: (i) => [...m.keys()][i] ?? null,
    removeItem: (k) => void m.delete(k),
    setItem: (k, v) => void m.set(k, String(v)),
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.resetModules();
  vi.doUnmock("@tauri-apps/api/core");
});

describe("getBackend", () => {
  it("getBackend picks web without tauri", async () => {
    const { getBackend } = await import("./index");
    expect(getBackend().kind).toBe("web");
  });

  it("getBackend picks desktop when the Tauri bridge is present", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    const { getBackend } = await import("./index");
    expect(getBackend().kind).toBe("desktop");
  });
});

describe("web backend", () => {
  it("web backend survives localStorage throwing", async () => {
    // Some browsers throw on the localStorage getter itself (blocked site data).
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      get() {
        throw new DOMException("denied", "SecurityError");
      },
    });
    try {
      const { createWebBackend } = await import("./web");
      const notices: string[] = [];
      const b = createWebBackend({ onNotice: (t) => notices.push(t) });

      const fresh = await b.loadProgress();
      expect(fresh.xp).toBe(0);
      expect(fresh.completed).toEqual([]);

      await b.saveProgress({ ...fresh, xp: 30, completed: ["hello-print:see"] }, rec);
      const again = await b.loadProgress();
      expect(again.xp).toBe(30); // kept in memory for this window
      expect(again.completed).toEqual(["hello-print:see"]);

      await b.saveSettings({ reducedMotion: true, dyslexiaFont: true, textScale: 1.25, mentor: null });
      expect((await b.getSettings()).dyslexiaFont).toBe(true);

      expect(notices).toEqual(["Progress won't be saved in this window."]);
    } finally {
      delete (globalThis as { localStorage?: Storage }).localStorage;
    }
  });

  it("web backend persists progress and settings under the v1 keys", async () => {
    const storage = memoryStorage();
    vi.stubGlobal("localStorage", storage);
    const { createWebBackend } = await import("./web");
    const b = createWebBackend();
    const s = await b.loadProgress();
    await b.saveProgress({ ...s, xp: 12 }, rec);
    await b.saveSettings({ reducedMotion: "system", dyslexiaFont: false, textScale: 1.5, mentor: null });

    expect(JSON.parse(storage.getItem(PROGRESS_KEY)!).xp).toBe(12);
    expect(JSON.parse(storage.getItem(SETTINGS_KEY)!).textScale).toBe(1.5);
    const b2 = createWebBackend();
    expect((await b2.loadProgress()).xp).toBe(12);
    expect((await b2.getSettings()).textScale).toBe(1.5);
  });

  it("web backend ignores corrupt stored data", async () => {
    const storage = memoryStorage();
    storage.setItem(PROGRESS_KEY, "{not json");
    storage.setItem(SETTINGS_KEY, JSON.stringify({ textScale: 9 }));
    vi.stubGlobal("localStorage", storage);
    const { createWebBackend } = await import("./web");
    const b = createWebBackend();
    expect((await b.loadProgress()).xp).toBe(0);
    expect((await b.getSettings()).textScale).toBe(1);
  });

  it("web mentor is off and answers with a RUN_ERROR", async () => {
    vi.stubGlobal("localStorage", memoryStorage());
    const { createWebBackend } = await import("./web");
    const b = createWebBackend();
    expect(b.mentorAvailable()).toBe(false);
    expect(await b.toolchain()).toEqual({ status: "n/a" });
    const events: AgUiEvent[] = [];
    await b.mentorHint(
      { concept: "loops", exercise_prompt: "x", code: "", last_error: null, attempt: 1 },
      (e) => events.push(e),
    );
    expect(events).toHaveLength(1);
    expect(events[0].type).toBe("RUN_ERROR");
  });
});

describe("tauri backend", () => {
  it("calls the Task 15 command names", async () => {
    const invoke = vi.fn(async (cmd: string) => {
      if (cmd === "load_progress") return { xp: 5, streak: { current: 1, best: 1, last_day: null, freezes: 0 }, concepts: {}, completed: [], mission_encounters: {} };
      if (cmd === "toolchain_status") return { status: "missing" };
      if (cmd === "get_settings") return { reducedMotion: "system", dyslexiaFont: false, textScale: 1, mentor: null };
      return undefined;
    });
    class Channel<T> {
      onmessage: (m: T) => void = () => {};
    }
    vi.doMock("@tauri-apps/api/core", () => ({ invoke, Channel }));
    const { createTauriBackend } = await import("./tauri");
    const b = createTauriBackend();
    expect(b.kind).toBe("desktop");
    expect((await b.loadProgress()).xp).toBe(5);
    expect(await b.toolchain()).toEqual({ status: "missing" });
    await b.run([{ name: "main.go", source: "package main" }]);
    await b.saveProgress((await b.loadProgress()), rec);
    await b.installToolchain(() => {});
    await b.mentorHint({ concept: "c", exercise_prompt: "p", code: "", last_error: null, attempt: 1 }, () => {});
    await b.getSettings();
    await b.saveSettings({ reducedMotion: true, dyslexiaFont: false, textScale: 1, mentor: null });
    await b.setApiKey("anthropic", "k");
    expect(invoke.mock.calls.map((c) => c[0])).toEqual([
      "load_progress",
      "toolchain_status",
      "run_code",
      "load_progress",
      "save_progress",
      "install_toolchain",
      "mentor_hint",
      "get_settings",
      "save_settings",
      "set_api_key",
    ]);
  });
});
