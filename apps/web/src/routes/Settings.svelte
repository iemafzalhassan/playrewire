<!-- Comfort settings. Changes apply at once and are saved. -->
<script lang="ts">
  import Card from "../lib/Card.svelte";
  import type { Settings, TextScale } from "../backend/types";

  let {
    settings,
    desktop = false,
    onChange,
  }: { settings: Settings; desktop?: boolean; onChange: (s: Settings) => void } = $props();

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

  <Card tilt={0.4} edge="a">
    <h2 class="h2">Mentor</h2>
    {#if desktop}
      <p class="sub">Bring your own AI model. Setup comes in a later update.</p>
    {:else}
      <p class="sub">The AI mentor comes with the desktop app. Here you get the built-in hints.</p>
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
</style>
