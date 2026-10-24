// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_SETTINGS, type Preset, type Settings } from "../backend/types";
import { applySettings } from "../settings";
import SettingsPage from "./Settings.svelte";

const OMNI =
  "OmniRoute runs on your computer and pools free tiers from many providers. Each provider's free-tier rules differ, and OmniRoute can see your prompts.";

const presets: Preset[] = [
  { id: "ollama", label: "Ollama (on this computer)", base_url: "http://localhost:11434/v1", api: "OpenAiCompatible", needs_key: false, local: true, default_model: "llama3.2:3b" },
  { id: "openai", label: "OpenAI", base_url: "https://api.openai.com/v1", api: "OpenAiCompatible", needs_key: true, local: false, default_model: "gpt-5-mini" },
  { id: "omniroute", label: "OmniRoute (on this computer)", base_url: "http://localhost:20128/v1", api: "OpenAiCompatible", needs_key: false, local: true, default_model: "auto" },
  { id: "custom", label: "Custom (any OpenAI-compatible server)", base_url: "", api: "OpenAiCompatible", needs_key: false, local: false, default_model: "" },
];

/** Renders Settings the way App does: every change is applied and fed back in. */
function mount(start: Partial<Settings> = {}, extra: Record<string, unknown> = {}) {
  let current: Settings = { ...DEFAULT_SETTINGS, ...start };
  const saveKey = vi.fn(async () => {});
  const changes: Settings[] = [];
  const view = render(SettingsPage, {
    props: {
      settings: current,
      desktop: true,
      presets,
      saveKey,
      onChange: (s: Settings) => {
        current = s;
        changes.push(s);
        applySettings(s);
        view.rerender({ settings: s });
      },
      ...extra,
    },
  });
  applySettings(current);
  return { saveKey, changes, current: () => current };
}

function systemMotion(reduce: boolean) {
  vi.stubGlobal("matchMedia", (q: string) => ({
    matches: q.includes("reduce") ? reduce : false,
    media: q,
    addEventListener() {},
    removeEventListener() {},
  }));
}

afterEach(() => {
  vi.unstubAllGlobals();
  const root = document.documentElement;
  delete root.dataset.motion;
  delete root.dataset.font;
  document.head.querySelectorAll("style[data-test]").forEach((s) => s.remove());
});

describe("Settings: accessibility", () => {
  it("dyslexia toggle switches body font", async () => {
    // The real stylesheets: tokens then the global sketchbook rules.
    // jsdom's import.meta.url is http, so read from the package root (vitest's cwd).
    const src = (f: string) => readFileSync(resolve(process.cwd(), "src", f), "utf8");
    for (const f of ["design/tokens.css", "app.css"]) {
      const style = document.createElement("style");
      style.dataset.test = "";
      style.textContent = src(f);
      document.head.append(style);
    }
    const tokens = src("design/tokens.css");
    const dyslexicStack = /--font-dyslexic:\s*([^;]+);/.exec(tokens)![1];
    expect(dyslexicStack).toMatch(/^["']?OpenDyslexic/);

    mount();
    const font = () => getComputedStyle(document.body).fontFamily;
    expect(document.documentElement.dataset.font).toBe("standard");
    expect(font()).not.toMatch(/dyslexic/i);
    await fireEvent.click(screen.getByRole("checkbox", { name: /dyslexia-friendly letters/i }));
    expect(document.documentElement.dataset.font).toBe("dyslexic");
    // jsdom leaves var() unresolved; Chromium resolves it (checked in the e2e run).
    expect(font()).toMatch(/OpenDyslexic|var\(--font-dyslexic\)/);
    await fireEvent.click(screen.getByRole("checkbox", { name: /dyslexia-friendly letters/i }));
    expect(document.documentElement.dataset.font).toBe("standard");
  });

  it("reduced motion setting overrides system", async () => {
    // The device asks for calm; the learner picks lively.
    systemMotion(true);
    mount();
    expect(document.documentElement.dataset.motion).toBe("reduced");
    await fireEvent.click(screen.getByRole("radio", { name: /lively/i }));
    expect(document.documentElement.dataset.motion).toBe("full");
    await fireEvent.click(screen.getByRole("radio", { name: /like my device/i }));
    expect(document.documentElement.dataset.motion).toBe("reduced");
    vi.unstubAllGlobals();

    // The device is lively; the learner picks calm.
    systemMotion(false);
    applySettings({ ...DEFAULT_SETTINGS });
    expect(document.documentElement.dataset.motion).toBe("full");
    await fireEvent.click(screen.getByRole("radio", { name: /calm/i }));
    expect(document.documentElement.dataset.motion).toBe("reduced");
  });

  it("text size scales the root", async () => {
    mount();
    await fireEvent.click(screen.getByRole("radio", { name: /biggest/i }));
    expect(document.documentElement.style.getPropertyValue("--text-scale")).toBe("1.5");
  });
});

describe("Settings: mentor", () => {
  it("omniroute notice shown", async () => {
    const { changes } = mount();
    expect(screen.queryByText(OMNI)).toBeNull();
    await fireEvent.change(screen.getByRole("combobox", { name: /mentor service/i }), { target: { value: "omniroute" } });
    expect(changes.at(-1)!.mentor).toEqual({ preset_id: "omniroute", base_url: "http://localhost:20128/v1", model: "auto" });
    expect(screen.getByText(OMNI)).toBeTruthy();
    // Local: no key asked for.
    expect(screen.queryByLabelText(/api key/i)).toBeNull();
    await fireEvent.change(screen.getByRole("combobox", { name: /mentor service/i }), { target: { value: "ollama" } });
    expect(screen.queryByText(OMNI)).toBeNull();
  });

  it("the key goes to the keychain, the field clears, and it is never shown again", async () => {
    const { saveKey } = mount({ mentor: { preset_id: "openai", base_url: "https://api.openai.com/v1", model: "gpt-5-mini" } });
    const field = screen.getByLabelText(/api key/i) as HTMLInputElement;
    expect(field.type).toBe("password");
    await fireEvent.input(field, { target: { value: "sk-secret-123" } });
    await fireEvent.click(screen.getByRole("button", { name: /save key/i }));
    expect(saveKey).toHaveBeenCalledWith("openai", "sk-secret-123");
    await waitFor(() => expect(screen.getByText(/key saved ✓/)).toBeTruthy());
    expect((screen.getByLabelText(/api key/i) as HTMLInputElement).value).toBe("");
    expect(document.body.innerHTML).not.toContain("sk-secret-123");
  });

  it("the model defaults to the preset's and can be changed", async () => {
    const { changes } = mount();
    await fireEvent.change(screen.getByRole("combobox", { name: /mentor service/i }), { target: { value: "openai" } });
    const model = screen.getByLabelText(/^model/i) as HTMLInputElement;
    expect(model.value).toBe("gpt-5-mini");
    await fireEvent.change(model, { target: { value: "gpt-5" } });
    expect(changes.at(-1)!.mentor).toEqual({ preset_id: "openai", base_url: "https://api.openai.com/v1", model: "gpt-5" });
  });

  it("the address can be edited only for local servers and custom", async () => {
    mount({ mentor: { preset_id: "openai", base_url: "https://api.openai.com/v1", model: "gpt-5-mini" } });
    expect(screen.queryByLabelText(/address/i)).toBeNull();
    await fireEvent.change(screen.getByRole("combobox", { name: /mentor service/i }), { target: { value: "ollama" } });
    expect((screen.getByLabelText(/address/i) as HTMLInputElement).value).toBe("http://localhost:11434/v1");
    await fireEvent.change(screen.getByRole("combobox", { name: /mentor service/i }), { target: { value: "custom" } });
    expect(screen.getByLabelText(/address/i)).toBeTruthy();
  });

  it("changing the custom address after saving a key says where the key goes", async () => {
    mount({ mentor: { preset_id: "custom", base_url: "http://10.0.0.5:8080/v1", model: "m" } });
    await fireEvent.input(screen.getByLabelText(/key/i), { target: { value: "k-1" } });
    await fireEvent.click(screen.getByRole("button", { name: /save key/i }));
    await waitFor(() => expect(screen.getByText(/key saved ✓/)).toBeTruthy());
    expect(screen.queryByText(/key will be sent to this new address/i)).toBeNull();
    await fireEvent.input(screen.getByLabelText(/address/i), { target: { value: "http://example.test/v1" } });
    expect(screen.getByText("Your saved key will be sent to this new address.")).toBeTruthy();
  });

  it("on the web the mentor card says it needs the desktop app", () => {
    render(SettingsPage, { props: { settings: { ...DEFAULT_SETTINGS }, desktop: false, onChange: vi.fn() } });
    expect(screen.getByText("AI mentor is available in the desktop app.")).toBeTruthy();
    expect(screen.queryByRole("combobox", { name: /mentor service/i })).toBeNull();
  });
});
