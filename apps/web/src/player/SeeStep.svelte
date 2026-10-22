<!-- See: an inked scene that acts out the idea, a two-line caption, next. -->
<script lang="ts">
  import type { Component } from "svelte";
  import type { See } from "../content/types";
  import Card from "../lib/Card.svelte";
  import Icon from "../lib/Icon.svelte";
  import InkButton from "../lib/InkButton.svelte";
  import { scenes } from "../scenes";

  let { see, nextLabel = "next", onDone }: { see: See; nextLabel?: string; onDone: () => void } = $props();

  const Scene = $derived(scenes[see.scene] as Component | undefined);
  // One sentence per line keeps the caption to two short lines.
  const lines = $derived(see.caption.match(/[^.!?]+[.!?]+/g)?.map((s) => s.trim()) ?? [see.caption]);
  let take = $state(0);
  const calm = () => document.documentElement.dataset.motion === "reduced";
  let canReplay = $state(true);
  $effect(() => {
    canReplay = !calm();
  });
</script>

<section class="see-step" aria-label="See">
  <Card edge="a" tilt={-0.5} class="stage">
    {#key take}
      {#if Scene}
        <Scene />
      {:else}
        <p class="no-scene">(this drawing is still being inked)</p>
      {/if}
    {/key}
  </Card>
  <p class="caption" data-testid="caption">
    {#each lines as l, i}{#if i}<br />{/if}{l}{/each}
  </p>
  <div class="actions">
    {#if canReplay && Scene}
      <InkButton onclick={() => take++}><Icon name="replay" size={22} /> watch again</InkButton>
    {/if}
    <InkButton primary onclick={onDone}>{nextLabel} <Icon name="arrow" size={28} /></InkButton>
  </div>
</section>

<style>
  .see-step {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-5);
  }
  .see-step :global(.stage) {
    padding: var(--space-5) var(--space-5) var(--space-4);
  }
  .no-scene {
    margin: var(--space-6) 0;
    text-align: center;
    font-family: var(--font-display);
    font-size: var(--size-hand);
    color: var(--ink-soft);
  }
  .caption {
    margin: 0 auto;
    max-width: 48rem;
    text-align: center;
    text-wrap: balance;
    font-size: var(--size-text-lg);
    line-height: 1.55;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    align-items: center;
    gap: var(--space-4);
  }
</style>
