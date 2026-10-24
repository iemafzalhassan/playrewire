<!-- Comfort settings. Changes apply at once and are saved. -->
<script lang="ts">
  import { untrack } from "svelte";
  import Card from "../lib/Card.svelte";
  import { BackendError } from "../backend/errors";
  import type { Preset, Settings, TextScale } from "../backend/types";

  let {
    settings,
    desktop = false,
    onChange,
    presets = [],
    saveKey = async () => {},
  }: {
    settings: Settings;
    desktop?: boolean;
    onChange: (s: Settings) => void;
    /** Mentor providers (desktop only; the web has no mentor). */
    presets?: Preset[];
    /** Sends a key to the OS keychain. It is never read back or shown. */
    saveKey?: (presetId: string, key: string) => Promise<void>;
  } = $props();

  const OMNIROUTE_NOTICE =
    "OmniRoute runs on your computer and pools free tiers from many providers. Each provider's free-tier rules differ, and OmniRoute can see your prompts.";

  const mentor = $derived(settings.mentor);
  const preset = $derived(presets.find((p) => p.id === mentor?.preset_id) ?? null);
  const isCustom = $derived(preset?.id === "custom");
  /** Hosted keyed services always use their own address (the backend pins it), so only local servers and custom can change it. */
  const editableUrl = $derived(!!preset && (preset.local || isCustom));
  const takesKey = $derived(!!preset && (preset.needs_key || isCustom));

  let keyDraft = $state("");
  let keySaving = $state(false);
  let savedFor = $state<string | null>(null);
  let keyTrouble = $state<string | null>(null);
  let urlDraft = $state<string | null>(null);
  /** The custom address a key was saved for: known this session, or maybe (set up before). */
  let keyUrl = $state<{ url: string; known: boolean } | null>(
    untrack(() => (settings.mentor?.preset_id === "custom" ? { url: settings.mentor.base_url, known: false } : null)),
  );
  const keyMoves = $derived(isCustom && keyUrl !== null && (urlDraft ?? mentor?.base_url ?? "") !== keyUrl.url);

  function pickPreset(id: string) {
    const p = presets.find((x) => x.id === id);
    keyDraft = "";
    keyTrouble = null;
    urlDraft = null;
    set({ mentor: p ? { preset_id: p.id, base_url: p.base_url, model: p.default_model } : null });
  }

  function setMentor(patch: Partial<{ base_url: string; model: string }>) {
    if (!mentor) return;
    set({ mentor: { ...mentor, ...patch } });
  }

  async function storeKey() {
    const key = keyDraft.trim();
    if (!mentor || !key || keySaving) return;
    keySaving = true;
    keyTrouble = null;
    try {
      await saveKey(mentor.preset_id, key);
      savedFor = mentor.preset_id;
      if (isCustom) keyUrl = { url: urlDraft ?? mentor.base_url, known: true };
    } catch (e) {
      keyTrouble = e instanceof BackendError ? e.message : "The key couldn't be saved. Try again.";
    } finally {
      // Never kept on screen, saved or not.
      keyDraft = "";
      keySaving = false;
    }
  }

  const motionChoices: { value: Settings["reducedMotion"]; label: string }[] = [
    { value: "system", label: "like my device" },
    { value: true, label: "calm" },
    { value: false, label: "lively" },
  ];
  const sizes: { value: TextScale; label: string }[] = [
    { value: 1, label: "normal" },
    { value: 1.25, label: "bigger" },
    { value: 1.5, label: "biggest" },
  ];
  const set = (patch: Partial<Settings>) => onChange({ ...settings, ...patch });
</script>

<section class="settings" aria-labelledby="page-title">
  <h1 id="page-title" class="hand-title" tabindex="-1">
    settings
    <svg class="scribble" viewBox="0 0 220 16" preserveAspectRatio="none" aria-hidden="true">
      <path d="M3 8 C45 12 100 5 150 9 S205 7 217 10" />
    </svg>
  </h1>

  <Card tilt={-0.5} edge="a">
    <fieldset>
      <legend>Motion</legend>
      <div class="chips">
        {#each motionChoices as c}
          <label class="chip">
            <input
              type="radio"
              name="motion"
              checked={settings.reducedMotion === c.value}
              onchange={() => set({ reducedMotion: c.value })}
            />
            <span>{c.label}</span>
          </label>
        {/each}
      </div>
    </fieldset>
  </Card>

  <Card tilt={0.6} edge="b">
    <fieldset>
      <legend>Text size</legend>
      <div class="chips">
        {#each sizes as s}
          <label class="chip">
            <input
              type="radio"
              name="size"
              checked={settings.textScale === s.value}
              onchange={() => set({ textScale: s.value })}
            />
            <span style:font-size="{s.value * 1.25}rem">{s.label}</span>
          </label>
        {/each}
      </div>
    </fieldset>
  </Card>

  <Card tilt={-0.3} edge="c">
    <label class="switch">
      <input
        type="checkbox"
        checked={settings.dyslexiaFont}
        onchange={(e) => set({ dyslexiaFont: e.currentTarget.checked })}
      />
      <span class="box" aria-hidden="true"></span>
      <span class="what">
        <strong>Dyslexia-friendly letters</strong>
        <span class="sub">Uses OpenDyslexic for reading text.</span>
      </span>
    </label>
  </Card>

  <Card tilt={0.4} edge="a" class="mentor">
    <h2 class="h2">Mentor</h2>
    {#if !desktop}
      <p>AI mentor is available in the desktop app.</p>
      <p class="sub">Here you get each lesson's built-in hints.</p>
    {:else}
      <p class="sub">A nudge in plain words when you're stuck. It never writes the answer for you. Use your own AI service.</p>
      <div class="field">
        <label for="mentor-preset">Mentor service</label>
        <select id="mentor-preset" value={mentor?.preset_id ?? ""} onchange={(e) => pickPreset(e.currentTarget.value)}>
          <option value="">none: built-in hints only</option>
          {#each presets as p (p.id)}<option value={p.id}>{p.label}</option>{/each}
        </select>
      </div>

      {#if preset && mentor}
        {#if preset.id === "omniroute"}
          <p class="notice" role="note">{OMNIROUTE_NOTICE}</p>
        {/if}
        <div class="field">
          <label for="mentor-model">Model</label>
          <input
            id="mentor-model"
            type="text"
            autocomplete="off"
            spellcheck="false"
            value={mentor.model}
            placeholder={preset.default_model || "the model's name"}
            onchange={(e) => setMentor({ model: e.currentTarget.value.trim() || preset.default_model })}
          />
        </div>
        {#if editableUrl}
          <div class="field">
            <label for="mentor-url">Address</label>
            <input
              id="mentor-url"
              type="url"
              autocomplete="off"
              spellcheck="false"
              value={mentor.base_url}
              placeholder="http://localhost:8080/v1"
              oninput={(e) => (urlDraft = e.currentTarget.value)}
              onchange={(e) => {
                setMentor({ base_url: e.currentTarget.value.trim() });
                urlDraft = null;
              }}
            />
            {#if keyMoves && keyUrl}
              <p class="notice" role="note">
                {keyUrl.known ? "Your saved key will be sent to this new address." : "If you saved a key for this server, it will be sent to this new address."}
              </p>
            {/if}
          </div>
        {/if}
        {#if takesKey}
          <div class="field">
            <label for="mentor-key">API key{isCustom ? " (if your server needs one)" : ""}</label>
            <div class="key-row">
              <input
                id="mentor-key"
                type="password"
                autocomplete="off"
                spellcheck="false"
                bind:value={keyDraft}
                onkeydown={(e) => e.key === "Enter" && storeKey()}
              />
              <button type="button" class="save" onclick={storeKey} disabled={!keyDraft.trim() || keySaving}>save key</button>
            </div>
            {#if savedFor === preset.id}<p class="saved" role="status">key saved ✓</p>{/if}
            {#if keyTrouble}<p class="trouble" role="alert"><span aria-hidden="true">✗</span> {keyTrouble}</p>{/if}
            <p class="sub">It goes to this computer's keychain. Rewire never shows it again.</p>
          </div>
        {:else if preset.local}
          <p class="sub">Runs on this computer. No key needed.</p>
        {/if}
      {/if}
      <p class="sub privacy">The mentor gets only the exercise and your code. Nothing about you.</p>
    {/if}
  </Card>
</section>

<style>
  .settings {
    display: grid;
    gap: var(--space-5);
    max-width: 40rem;
  }
  fieldset {
    border: 0;
    margin: 0;
    padding: 0;
    min-width: 0;
  }
  legend,
  .h2 {
    margin: 0 0 var(--space-3);
    padding: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand);
    font-weight: 700;
    line-height: 1;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
  }
  .chip {
    position: relative;
    cursor: pointer;
  }
  .chip input,
  .switch input {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
  }
  .chip span {
    display: inline-block;
    padding: var(--space-1) var(--space-4);
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    border: var(--ink-line) solid var(--ink-soft);
    border-radius: var(--radius-wobble-c);
    background: var(--paper);
  }
  .chip input:checked + span {
    background: var(--ink);
    color: var(--card);
    border-color: var(--ink);
  }
  .chip input:checked + span::before {
    content: "✓ " / "";
  }
  .chip input:focus-visible + span,
  .switch input:focus-visible + .box {
    outline: 3px solid var(--focus);
    outline-offset: 3px;
  }
  .switch {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    cursor: pointer;
  }
  .box {
    flex: none;
    width: 1.6rem;
    height: 1.6rem;
    margin-top: 0.1rem;
    border: var(--ink-line-bold) solid var(--ink);
    border-radius: var(--radius-wobble-a);
    background: var(--paper);
    display: grid;
    place-items: center;
  }
  .switch input:checked + .box {
    background: var(--ink);
  }
  .switch input:checked + .box::after {
    content: "✓" / "";
    color: var(--card);
    font-weight: 700;
  }
  .what {
    display: grid;
    gap: var(--space-1);
  }
  .sub {
    margin: 0;
    color: var(--ink-soft);
  }
  .settings :global(.mentor) {
    display: grid;
    gap: var(--space-3);
  }
  .settings :global(.mentor .h2),
  .settings :global(.mentor p) {
    margin: 0;
  }
  .field {
    display: grid;
    gap: var(--space-1);
  }
  .field label {
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    font-weight: 700;
  }
  select,
  input[type="text"],
  input[type="url"],
  input[type="password"] {
    width: 100%;
    min-height: 2.75rem;
    padding: var(--space-2) var(--space-3);
    border: var(--ink-line) solid var(--ink);
    border-radius: var(--radius-wobble-c);
    background: var(--paper);
    color: var(--ink);
    font: inherit;
  }
  input[type="text"],
  input[type="url"],
  input[type="password"] {
    font-family: var(--font-code);
    font-size: 0.95rem;
  }
  .key-row {
    display: flex;
    gap: var(--space-2);
  }
  .key-row input {
    flex: 1;
    min-width: 0;
  }
  .save {
    flex: none;
    min-height: 2.75rem;
    padding: 0 var(--space-4);
    border: var(--ink-line) solid var(--ink);
    border-radius: var(--radius-wobble-b);
    background: var(--ink);
    color: var(--card);
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    font-weight: 700;
    cursor: pointer;
  }
  .save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  /* A taped note in the margin: things worth knowing before you go on. */
  .notice {
    padding: var(--space-2) var(--space-3);
    background: color-mix(in srgb, var(--highlight) 40%, var(--card));
    border-left: 3px solid var(--highlight);
    transform: rotate(-0.4deg);
    line-height: 1.4;
  }
  .saved {
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    font-weight: 700;
    color: var(--ok);
  }
  .trouble span {
    color: var(--accent-deep);
  }
  .privacy {
    font-size: var(--size-text-sm);
  }
</style>
