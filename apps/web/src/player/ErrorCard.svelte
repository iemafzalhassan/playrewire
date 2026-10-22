<!-- A friendly card for each Go error: the plain-words title and explanation
     first, where it happened, and Go's exact words folded away underneath.
     Never a raw error on its own (spec 3, anti-goals). -->
<script lang="ts">
  import type { FriendlyError } from "../core";
  import Card from "../lib/Card.svelte";

  let { errors }: { errors: FriendlyError[] } = $props();

  const MAX = 3;
  const shown = $derived(errors.slice(0, MAX));

  /** `code` spans in the explanation become <code>, the rest stays text. */
  function pieces(s: string): { code: boolean; text: string }[] {
    return s.split("`").map((text, i) => ({ code: i % 2 === 1, text })).filter((p) => p.text);
  }
</script>

<div class="cards">
  {#each shown as e, i (i)}
    <Card edge={i % 2 ? "c" : "b"} tilt={i % 2 ? 0.4 : -0.5} class="error-card">
      <p class="title">
        <span class="glyph" aria-hidden="true">✗</span>
        <span class="sr-only">Problem: </span><span>{e.title}</span>
      </p>
      {#if e.line !== null}
        <p class="where">line {e.line}{e.col !== null ? `, column ${e.col}` : ""}</p>
      {/if}
      <p class="plain">
        {#each pieces(e.plain) as p}{#if p.code}<code>{p.text}</code>{:else}{p.text}{/if}{/each}
      </p>
      {#if e.raw.trim()}
        <details>
          <summary>what Go said, exactly</summary>
          <pre>{e.raw}</pre>
        </details>
      {/if}
    </Card>
  {/each}
  {#if errors.length > MAX}
    <p class="more">…and {errors.length - MAX} more. Fix these first; the rest often go away too.</p>
  {/if}
</div>

<style>
  .cards {
    display: grid;
    gap: var(--space-4);
  }
  .cards :global(.error-card) {
    padding: var(--space-4) var(--space-5);
  }
  .title {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    margin: 0;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: var(--size-hand);
    line-height: 1.1;
  }
  .glyph {
    color: var(--accent-deep);
    font-size: 1.1em;
  }
  .where {
    display: inline-block;
    margin: var(--space-1) 0 0;
    padding: 0 var(--space-2);
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: var(--accent-deep);
    border-bottom: 2px solid color-mix(in srgb, var(--accent) 55%, transparent);
    transform: rotate(-1deg);
  }
  .plain {
    margin: var(--space-2) 0 0;
    font-size: var(--size-text-lg);
    max-width: 38rem;
  }
  .plain code {
    padding: 0.05em 0.35em;
    border-radius: 4px;
    background: color-mix(in srgb, var(--highlight) 40%, transparent);
    font-size: 0.92em;
  }
  details {
    margin-top: var(--space-3);
  }
  summary {
    cursor: pointer;
    display: inline-flex;
    min-height: 2.75rem;
    align-items: center;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: var(--ink-soft);
  }
  summary:hover {
    color: var(--ink);
  }
  pre {
    margin: var(--space-2) 0 0;
    padding: var(--space-3);
    overflow-x: auto;
    border-left: 3px solid var(--pencil);
    background: color-mix(in srgb, var(--paper-deep) 70%, transparent);
    font-size: var(--size-text-sm);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .more {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: var(--ink-soft);
  }
</style>
