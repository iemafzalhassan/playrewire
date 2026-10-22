<!-- The one-time Go download (desktop). Shown at the top of the page on first
     launch and inside a drill when Run needs Go. Reading and typing never
     wait for it. Offline uses the exact Review Focus 2 words and a Retry. -->
<script lang="ts">
  import { ERROR_COPY, GO_DOWNLOAD_COPY } from "../backend/errors";
  import Card from "../lib/Card.svelte";
  import InkButton from "../lib/InkButton.svelte";
  import type { Toolchain } from "./toolchain.svelte";

  let { toolchain, tilt = -0.4 }: { toolchain: Toolchain; tilt?: number } = $props();

  const mb = (n: number) => Math.round(n / 1_000_000);
  const pct = $derived(toolchain.total > 0 ? Math.min(100, Math.floor((toolchain.done / toolchain.total) * 100)) : 0);
  const failed = $derived(toolchain.phase === "failed" ? ERROR_COPY[toolchain.error ?? "io"] : null);

  function start() {
    void toolchain.install();
  }
</script>

<Card {tilt} edge="b" tape class="toolchain-card" data-testid="toolchain-card">
  {#if toolchain.phase === "installing"}
    <p class="title">Downloading Go…</p>
    <progress max="100" value={pct} aria-label="Go download">{pct}%</progress>
    <p class="sub" aria-live="polite">
      {#if toolchain.total > 0}{mb(toolchain.done)} of {mb(toolchain.total)} MB{:else}starting…{/if}
    </p>
  {:else if failed}
    <p class="title"><span class="glyph" aria-hidden="true">✗</span> {failed.title}</p>
    <p role="alert">{failed.text}</p>
    <InkButton primary onclick={start}>Retry</InkButton>
  {:else if toolchain.phase === "installed"}
    <p class="title" role="status"><span class="glyph ok" aria-hidden="true">✓</span> Go is ready. Press run.</p>
  {:else}
    <p class="title">One quick download</p>
    <p>{GO_DOWNLOAD_COPY}</p>
    <InkButton primary onclick={start}>download Go</InkButton>
  {/if}
</Card>

<style>
  :global(.toolchain-card) {
    display: grid;
    gap: var(--space-3);
    justify-items: start;
    max-width: 34rem;
    padding: var(--space-4) var(--space-5);
  }
  p {
    margin: 0;
  }
  .title {
    font-family: var(--font-display);
    font-size: var(--size-hand);
    font-weight: 700;
  }
  .sub {
    color: var(--ink-soft);
    font-variant-numeric: tabular-nums;
  }
  .glyph {
    color: var(--accent);
  }
  .glyph.ok {
    color: var(--ok);
  }
  progress {
    width: 100%;
    height: 0.9rem;
    appearance: none;
    border: 2px solid var(--ink);
    border-radius: 0.5rem 0.4rem 0.6rem 0.45rem;
    background: var(--card);
    overflow: hidden;
  }
  progress::-webkit-progress-bar {
    background: var(--card);
  }
  progress::-webkit-progress-value {
    background: var(--accent);
  }
  progress::-moz-progress-bar {
    background: var(--accent);
  }
</style>
