<!-- The day's three parts in order (warm-up, new page, fix-it), with where the
     learner is. Done parts get an ink ✓, never colour alone. `compact` is the
     one-line version shown on top of each part. -->
<script lang="ts">
  import type { DayPart } from "../today";

  let { parts, compact = false }: { parts: DayPart[]; compact?: boolean } = $props();

  const words: Record<DayPart["state"], string> = { done: "(done)", now: "(now)", later: "", none: "(nothing today)" };
</script>

<ol class="day" class:compact aria-label="Today's plan">
  {#each parts as p, i}
    <li data-part={p.key} class={p.state} aria-current={p.state === "now" ? "step" : undefined}>
      <span class="dot" aria-hidden="true">{p.state === "done" ? "✓" : p.state === "none" ? "–" : i + 1}</span>
      <span class="what">
        <span class="label">{p.label}</span>
        {#if !compact}<span class="detail">{p.detail}</span>{/if}
      </span>
      <span class="sr-only">{words[p.state]}</span>
    </li>
  {/each}
</ol>

<style>
  .day {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-3);
    counter-reset: part;
  }
  li {
    position: relative;
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    min-width: 0;
    color: var(--ink-soft);
  }
  /* A dotted pencil line leads from one part to the next. */
  li + li::before {
    content: "";
    position: absolute;
    top: 0.95rem;
    right: calc(100% + 0.15rem);
    width: calc(var(--space-3) - 0.3rem);
    border-top: 2.5px dotted var(--pencil);
  }
  .dot {
    flex: none;
    display: inline-grid;
    place-items: center;
    width: 1.9rem;
    height: 1.9rem;
    border: 2px dashed currentColor;
    border-radius: 48% 52% 45% 55% / 55% 45% 55% 45%;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    font-weight: 700;
    line-height: 1;
  }
  .what {
    display: grid;
    min-width: 0;
  }
  .label {
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    font-weight: 700;
    line-height: 1.2;
  }
  .detail {
    font-size: var(--size-text-sm);
    line-height: 1.3;
    overflow-wrap: anywhere;
  }
  .done {
    color: var(--ink);
  }
  .done .dot {
    border-style: solid;
    background: var(--ink);
    border-color: var(--ink);
    color: var(--card);
  }
  .now {
    color: var(--ink);
  }
  .now .dot {
    border-style: solid;
    background: var(--accent);
    border-color: var(--ink);
    color: var(--card);
  }
  .none {
    opacity: 0.7;
  }
  .none .label {
    text-decoration: line-through;
    text-decoration-thickness: 1.5px;
  }
  .compact {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2) var(--space-5);
  }
  .compact li + li::before {
    width: calc(var(--space-5) - 0.5rem);
    right: calc(100% + 0.25rem);
  }
  .compact .dot {
    width: 1.6rem;
    height: 1.6rem;
    font-size: 1.05rem;
  }
  .compact .label {
    font-size: 1.2rem;
    line-height: 1.6rem;
  }
  @media (max-width: 34rem) {
    .day:not(.compact) {
      grid-template-columns: 1fr;
    }
    .day:not(.compact) li + li::before {
      display: none;
    }
  }
</style>
