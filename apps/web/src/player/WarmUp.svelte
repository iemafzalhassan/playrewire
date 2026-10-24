<!-- The warm-up: one quick drill per due concept, at most three, about a minute in
     all (spec 4.6). Each is one pass from memory. Every outcome is a `drill` step
     on the chunk the drill came from, so the review schedule moves: a pass sends the
     concept further out, and the first slip on a drill is saved as a fail once. -->
<script lang="ts">
  import { tick, untrack } from "svelte";
  import type { Backend } from "../backend/types";
  import type { Chunk } from "../content/types";
  import type { ProgressState } from "../core";
  import type { DayPart, DrillRef } from "../today";
  import Card from "../lib/Card.svelte";
  import DayTrack from "../lib/DayTrack.svelte";
  import Icon from "../lib/Icon.svelte";
  import InkButton from "../lib/InkButton.svelte";
  import DrillStep from "./DrillStep.svelte";
  import { saveQueue, stepRecord } from "./record";
  import RewardSlot from "./RewardSlot.svelte";

  let {
    items,
    chunks,
    progress,
    backend,
    onProgress,
    onDone,
    doneLabel = "back to today",
    title = "warm-up",
    day = null,
  }: {
    items: DrillRef[];
    /** Every chunk a drill may come from. */
    chunks: Chunk[];
    progress: ProgressState;
    backend: Pick<Backend, "kind" | "run" | "saveProgress"> & Partial<Pick<Backend, "mentorAvailable" | "mentorHint">>;
    onProgress: (s: ProgressState) => void;
    onDone: () => void;
    doneLabel?: string;
    title?: string;
    /** Today's plan, shown on top while the day runs. */
    day?: DayPart[] | null;
  } = $props();

  // The list is fixed when the warm-up opens: passing a drill makes its concept not due.
  const list = untrack(() =>
    items.flatMap((r) => {
      const c = chunks.find((x) => x.id === r.chunk && x.pack === r.pack);
      const d = c?.drills[r.drill];
      return c && d ? [{ ref: r, chunk: c, drill: d }] : [];
    }),
  );
  const mentor = untrack(() => (backend.mentorHint && backend.mentorAvailable ? (backend as Required<typeof backend>) : null));

  let at = $state(0);
  let slot: RewardSlot;
  /** Records are chained on the latest state, so a slip and the pass after it never race. */
  const saves = untrack(() => saveQueue(progress, backend, (s) => onProgress(s)));
  const missed = new Set<number>();

  const item = $derived(list[at]);
  const last = $derived(at >= list.length - 1);

  function save(i: number, outcome: { success: boolean; firstTry: boolean; hintsUsed: number }) {
    const { chunk } = list[i];
    saves.save(stepRecord(chunk.id, chunk.concept, outcome)).then((out) => {
      if (out && outcome.success) slot?.show(out.rewards);
    });
  }

  function miss(i: number, hintsUsed: number) {
    if (missed.has(i)) return;
    missed.add(i);
    save(i, { success: false, firstTry: false, hintsUsed });
  }

  async function onward() {
    if (last) return onDone();
    at++;
    await tick();
    document.getElementById("step-title")?.focus();
  }
</script>

<article class="warmup" aria-labelledby="page-title">
  <header class="head">
    <a class="back" href="#/"><Icon name="back" size={22} /> today</a>
    {#if day}<DayTrack parts={day} compact />{/if}
    <h1 id="page-title" class="hand-title" tabindex="-1">
      {title}
      <svg class="scribble" viewBox="0 0 220 16" preserveAspectRatio="none" aria-hidden="true">
        <path d="M3 9 C50 5 95 12 140 7 S198 6 217 10" />
      </svg>
    </h1>
  </header>

  <RewardSlot bind:this={slot} />

  {#if item}
    <p class="where">
      <span class="count">{at + 1} of {list.length}</span>
      <span aria-hidden="true">·</span>
      <span>about a minute</span>
      <span aria-hidden="true">·</span>
      <span>from <em>{item.chunk.title}</em></span>
    </p>
    <h2 id="step-title" class="step-title" tabindex="-1">from memory, one go</h2>
    {#key at}
      <DrillStep
        drills={[item.drill]}
        hints={item.chunk.hints}
        concept={item.chunk.concept}
        {mentor}
        needed={1}
        doneWords="Still got it!"
        run={(files) => backend.run(files)}
        nextLabel={last ? doneLabel : "next warm-up"}
        onMiss={(r) => miss(at, r.hintsUsed)}
        onDone={(r) => save(at, { success: true, firstTry: r.firstTry, hintsUsed: r.hintsUsed })}
        onContinue={onward}
      />
    {/key}
  {:else}
    <Card tape edge="c" tilt={-0.6} class="empty">
      <h2 id="step-title" class="empty-title" tabindex="-1">Nothing to warm up right now.</h2>
      <p>Everything you've learned is fresh. Come back when something is due.</p>
      <InkButton primary onclick={onDone}>{doneLabel} <Icon name="arrow" size={28} /></InkButton>
    </Card>
  {/if}
</article>

<style>
  .warmup {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-4);
  }
  .head {
    display: grid;
    gap: var(--space-3);
  }
  .back {
    justify-self: start;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-height: 2.75rem;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: var(--ink-soft);
    text-decoration: none;
  }
  .back:hover {
    color: var(--ink);
  }
  .where {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: var(--ink-soft);
  }
  .count {
    font-weight: 700;
    color: var(--ink);
  }
  .step-title {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand-lg);
    line-height: 1;
    color: var(--ink-soft);
    outline: none;
    transform: rotate(-1deg);
    transform-origin: left;
  }
  .warmup :global(.empty) {
    max-width: 32rem;
    padding: var(--space-5);
    display: grid;
    gap: var(--space-3);
    justify-items: start;
  }
  .empty-title {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand-lg);
    line-height: 1.05;
    outline: none;
  }
  .warmup :global(.empty p) {
    margin: 0;
  }
</style>
