<!-- Break it, fix it: a finished page's broken program. The learner starts from the
     broken code and makes it print what it should. Fixing it is saved as a `drill`
     step on "<chunk id>#fix" with the chunk's concept, so it counts for review and
     the same fix-it isn't offered again. -->
<script lang="ts">
  import { untrack } from "svelte";
  import type { Backend } from "../backend/types";
  import type { Chunk, Drill } from "../content/types";
  import type { ProgressState } from "../core";
  import { FIX_SUFFIX, type DayPart } from "../today";
  import DayTrack from "../lib/DayTrack.svelte";
  import Icon from "../lib/Icon.svelte";
  import DrillStep from "./DrillStep.svelte";
  import { saveQueue, stepRecord } from "./record";
  import RewardSlot from "./RewardSlot.svelte";

  let {
    chunk,
    progress,
    backend,
    onProgress,
    onDone,
    doneLabel = "back to today",
    day = null,
  }: {
    chunk: Chunk;
    progress: ProgressState;
    backend: Pick<Backend, "kind" | "run" | "saveProgress"> & Partial<Pick<Backend, "mentorAvailable" | "mentorHint">>;
    onProgress: (s: ProgressState) => void;
    onDone: () => void;
    doneLabel?: string;
    day?: DayPart[] | null;
  } = $props();

  const fix = untrack(() => chunk.fixIt);
  const drill: Drill | null = fix
    ? {
        prompt: "This program is broken. Find what's wrong and fix it, so it prints this:",
        starter: fix.broken,
        solution: fix.solution,
        expectStdout: fix.expectStdout,
        wholeFile: fix.wholeFile,
      }
    : null;
  const mentor = untrack(() => (backend.mentorHint && backend.mentorAvailable ? (backend as Required<typeof backend>) : null));
  let slot: RewardSlot;
  const saves = untrack(() => saveQueue(progress, backend, (s) => onProgress(s)));
  const fixId = untrack(() => `${chunk.id}${FIX_SUFFIX}`);

  /** The first miss (once; DrillStep calls it once per round and the fix-it has one round). */
  function missed(r: { hintsUsed: number }) {
    saves.save(stepRecord(fixId, chunk.concept, { success: false, firstTry: false, hintsUsed: r.hintsUsed }));
  }

  async function fixed(r: { firstTry: boolean; hintsUsed: number }) {
    const out = await saves.save(stepRecord(fixId, chunk.concept, { success: true, ...r }));
    if (out) slot?.show(out.rewards);
  }
</script>

<article class="fixit" aria-labelledby="page-title">
  <header class="head">
    <a class="back" href="#/"><Icon name="back" size={22} /> today</a>
    {#if day}<DayTrack parts={day} compact />{/if}
    <h1 id="page-title" class="hand-title" tabindex="-1">
      break it, fix it
      <svg class="scribble" viewBox="0 0 220 16" preserveAspectRatio="none" aria-hidden="true">
        <path d="M3 10 C45 6 92 12 138 8 S196 5 217 9" />
      </svg>
    </h1>
    <p class="from"><Icon name="wrench" size={22} /> from <em>{chunk.title}</em></p>
  </header>

  <RewardSlot bind:this={slot} />

  {#if drill}
    <h2 id="step-title" class="step-title" tabindex="-1">someone broke it</h2>
    <DrillStep
      drills={[drill]}
      hints={chunk.hints}
      concept={chunk.concept}
      {mentor}
      needed={1}
      showWant
      doneWords="Fixed it! It prints just right."
      run={(files) => backend.run(files)}
      nextLabel={doneLabel}
      onDone={fixed}
      onMiss={missed}
      onContinue={onDone}
    />
  {:else}
    <p class="from">This page has nothing to fix.</p>
  {/if}
</article>

<style>
  .fixit {
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
  .from {
    display: inline-flex;
    gap: var(--space-2);
    align-items: center;
    margin: var(--space-2) 0 0;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: var(--ink-soft);
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
</style>
