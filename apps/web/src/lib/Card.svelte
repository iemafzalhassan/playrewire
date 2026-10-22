<!-- A sheet of card paper: wobbly inked edge and an offset ink shadow. Only the
     edge goes through the wobble filter, so the text stays crisp. -->
<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLAttributes } from "svelte/elements";

  let {
    tilt = 0,
    edge = "a",
    tape = false,
    children,
    class: cls = "",
    ...rest
  }: HTMLAttributes<HTMLDivElement> & {
    tilt?: number;
    edge?: "a" | "b" | "c";
    tape?: boolean;
    children: Snippet;
  } = $props();
</script>

<div class="ink-card edge-{edge} {cls}" class:taped={tape} style:--tilt="{tilt}deg" {...rest}>
  {#if tape}<span class="tape" aria-hidden="true"></span>{/if}
  {@render children()}
</div>

<style>
  .ink-card {
    position: relative;
    isolation: isolate;
    transform: rotate(var(--tilt));
    padding: var(--space-4) var(--space-4) var(--space-3);
    --edge: var(--radius-wobble-a);
  }
  .edge-b {
    --edge: var(--radius-wobble-b);
  }
  .edge-c {
    --edge: var(--radius-wobble-c);
  }
  .ink-card::before,
  .ink-card::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: var(--edge);
    filter: url(#ink-wobble);
    pointer-events: none;
  }
  .ink-card::before {
    z-index: -1;
    background: var(--card);
    border: var(--ink-line) solid var(--ink);
  }
  .ink-card::after {
    z-index: -2;
    background: var(--ink);
    transform: translate(var(--ink-shadow-x), var(--ink-shadow-y));
  }
  .taped {
    margin-top: var(--space-3);
  }
  /* A strip of masking tape holding the card to the page. */
  .tape {
    position: absolute;
    top: -0.8rem;
    left: 50%;
    width: 5.5rem;
    height: 1.4rem;
    transform: translateX(-50%) rotate(-3deg);
    background: color-mix(in srgb, var(--highlight) 65%, transparent);
    border-inline: 2px dotted color-mix(in srgb, var(--ink) 15%, transparent);
  }
</style>
