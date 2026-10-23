<!-- Plays one chunk's steps in order (ruling R6: its `steps`). Each finished
     step becomes a StepRecord -> applyStep -> saveProgress -> rewards. Only
     step outcomes are saved; keys typed and code written never leave here.
     The Use step walks the hand-inked street to the mission's terminal. -->
<script lang="ts">
  import { tick, untrack } from "svelte";
  import type { Backend } from "../backend/types";
  import type { Bundle, Chunk, Mission, Step } from "../content/types";
  import { loadBundle, loadTrouble } from "../content/load";
  import { applyStep, initCore, type ProgressState, type Reward, type StepRecord } from "../core";
  import Card from "../lib/Card.svelte";
  import Icon from "../lib/Icon.svelte";
  import InkButton from "../lib/InkButton.svelte";
  import DrillStep from "./DrillStep.svelte";
  import { localDate } from "./drill";
  import Rewards from "./Rewards.svelte";
  import SeeStep from "./SeeStep.svelte";
  import TypeStep from "./TypeStep.svelte";
  import UseStep from "./UseStep.svelte";

  let {
    chunk,
    progress,
    backend,
    onProgress,
    onHome,
    warm = true,
  }: {
    chunk: Chunk;
    progress: ProgressState;
    backend: Pick<Backend, "kind" | "run" | "saveProgress">;
    onProgress: (s: ProgressState) => void;
    onHome: () => void;
    /** Start the browser Go lab early, while the learner reads and types. */
    warm?: boolean;
  } = $props();

  type Playable = Step;
  const playable = $derived(chunk.steps);
  const done = (s: Step) => progress.completed.includes(`${chunk.id}:${s}`);

  /** Where to begin: the first step not done; a finished page plays again from the top. */
  function startAt(): number | "finished" {
    const i = playable.findIndex((s) => !done(s));
    return i >= 0 ? i : 0;
  }

  let at = $state<number | "finished">(untrack(startAt));
  let rewards = $state<Reward[]>([]);
  let rewardKey = $state(0);
  let saving = $state(false);
  let trouble = $state<{ title: string; text: string } | null>(null);
  let drillReady = $state(false);
  let bundle = $state<Bundle | null>(null);
  const mission = $derived<Mission | undefined>(
    bundle?.packs.find((p) => p.id === chunk.pack)?.missions.find((m) => m.id === chunk.mission),
  );

  const current = $derived(at === "finished" ? null : playable[at]);

  // Drill needs the core rules and the bundle; both are cached after the first load.
  async function prepareDrill() {
    trouble = null;
    try {
      const [, b] = await Promise.all([initCore(), loadBundle()]);
      bundle = b;
      drillReady = true;
    } catch (e) {
      console.error(e);
      trouble = loadTrouble();
    }
  }

  let warmed = false;
  $effect(() => {
    if ((current === "drill" || current === "use") && !drillReady) untrack(prepareDrill);
    // Wake the browser lab while the learner is busy, so the first run is quick.
    if (warm && !warmed && backend.kind === "web" && (current === "type" || current === "drill" || current === "use")) {
      warmed = true;
      backend
        .run([{ name: "main.go", source: 'package main\n\nfunc main() {}\n' }])
        .catch(() => {});
    }
  });

  // XP floats away quickly; streak and level notes stay a little longer.
  let rewardTimer: ReturnType<typeof setTimeout> | undefined;
  function showRewards(r: Reward[]) {
    clearTimeout(rewardTimer);
    rewards = r;
    rewardKey++;
    const linger = r.some((x) => x.kind !== "xp") ? 6000 : 2800;
    rewardTimer = setTimeout(() => (rewards = []), linger);
  }
  $effect(() => () => clearTimeout(rewardTimer));

  /** Record a finished step; `advance` false keeps the step on screen until the learner moves on. */
  async function finish(step: Playable, outcome: { firstTry: boolean; hintsUsed: number }, advance = true) {
    if (saving) return;
    saving = true;
    const rec: StepRecord = {
      chunk_id: chunk.id,
      concept: chunk.concept,
      step,
      mission_id: step === "use" ? (chunk.mission ?? null) : null,
      success: true,
      first_try: outcome.firstTry,
      hints_used: outcome.hintsUsed,
      local_date: localDate(),
    };
    try {
      await initCore();
      const out = applyStep(progress, rec);
      await backend.saveProgress(out.state, rec);
      onProgress(out.state);
      showRewards(out.rewards);
    } catch (e) {
      // The web backend already handles storage trouble; anything else should
      // not trap the learner on a finished step.
      console.error("could not save the step", e);
    } finally {
      saving = false;
    }
    if (advance) await next();
  }

  async function next() {
    at = typeof at === "number" && at + 1 < playable.length ? at + 1 : "finished";
    await tick();
    // The typing area takes focus itself (so keys land there); otherwise the
    // new step's heading does, so screen readers hear the change.
    if (current !== "type") document.getElementById("step-title")?.focus();
  }

  const stepWords: Record<Playable, { title: string; label: string }> = {
    see: { title: "watch", label: "see" },
    type: { title: "type it", label: "type" },
    drill: { title: "from memory", label: "drill" },
    use: { title: "light up the city", label: "street" },
  };
  const nextLabel = (i: number) => {
    const n = playable[i + 1];
    return n ? `next: ${stepWords[n].label}` : "finish";
  };
  const stepState = (s: Playable, i: number) =>
    at === "finished" || (typeof at === "number" && i < at) || (done(s) && at !== i) ? "done" : at === i ? "now" : "later";
</script>

<article class="player" aria-labelledby="page-title">
  <header class="head">
    <a class="back" href="#/"><Icon name="back" size={22} /> today</a>
    <h1 id="page-title" class="hand-title" tabindex="-1">
      {chunk.title}
      <svg class="scribble" viewBox="0 0 220 16" preserveAspectRatio="none" aria-hidden="true">
        <path d="M3 9 C50 4 90 12 130 8 S195 5 217 10" />
      </svg>
    </h1>
    <ol class="steps" aria-label="Steps on this page">
      {#each playable as s, i}
        {@const st = stepState(s, i)}
        <li class={st} aria-current={st === "now" ? "step" : undefined}>
          <span class="dot" aria-hidden="true">{st === "done" ? "✓" : i + 1}</span>
          <span>{stepWords[s].label}</span>
          <span class="sr-only">{st === "done" ? "(done)" : st === "now" ? "(now)" : ""}</span>
        </li>
      {/each}
    </ol>
  </header>

  {#key rewardKey}
    {#if rewards.length}
      <div class="reward-slot"><Rewards {rewards} /></div>
    {/if}
  {/key}

  {#if current}
    <h2 id="step-title" class="step-title" tabindex="-1">{stepWords[current].title}</h2>
  {/if}

  {#if current === "see"}
    <SeeStep see={chunk.see} nextLabel={nextLabel(at as number)} onDone={() => finish("see", { firstTry: true, hintsUsed: 0 })} />
  {:else if current === "type" && chunk.type}
    <TypeStep step={chunk.type} nextLabel={nextLabel(at as number)} onDone={(r) => finish("type", { firstTry: r.mistakes === 0, hintsUsed: 0 })} />
  {:else if current === "drill" || current === "use"}
    {#if trouble}
      <Card edge="b" tilt={-0.5} class="trouble" role="alert">
        <p class="t-title"><span aria-hidden="true">✗</span> {trouble.title}</p>
        <p>{trouble.text}</p>
        <InkButton onclick={prepareDrill}>try again</InkButton>
      </Card>
    {:else if drillReady && current === "use"}
      {#if mission && bundle}
        <UseStep
          {mission}
          cityStub={bundle.cityStub}
          {progress}
          hints={chunk.hints}
          run={(files) => backend.run(files)}
          nextLabel={nextLabel(at as number)}
          onDone={(r) => finish("use", r, false)}
          onContinue={next}
        />
      {:else}
        <p class="hush">This page's street isn't drawn yet.</p>
      {/if}
    {:else if drillReady}
      <DrillStep
        drills={chunk.drills}
        hints={chunk.hints}
        run={(files) => backend.run(files)}
        nextLabel={nextLabel(at as number)}
        onDone={(r) => finish("drill", r, false)}
        onContinue={next}
      />
    {:else}
      <p class="hush" aria-live="polite">sharpening pencils…</p>
    {/if}
  {:else}
    <section class="finished" aria-labelledby="done-title">
      <Card tape edge="c" tilt={-0.8} class="done-card">
        <h2 id="step-title" class="done-title" tabindex="-1">
          <span class="tick" aria-hidden="true">✓</span>
          <span id="done-title">Page done!</span>
        </h2>
        <p>{chunk.title}: {playable.map((s) => stepWords[s].label).join(", ")}. All yours now.</p>
        <div class="done-actions">
          <InkButton primary onclick={onHome}>back to today <Icon name="arrow" size={28} /></InkButton>
          <InkButton onclick={() => ((at = 0), (rewards = []))}><Icon name="replay" size={22} /> play it again</InkButton>
        </div>
      </Card>
    </section>
  {/if}
</article>

<style>
  .player {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-5);
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
  .steps {
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2) var(--space-5);
    margin: var(--space-3) 0 0;
    padding: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
  }
  .steps li {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--ink-soft);
  }
  .steps li + li::before {
    content: "";
    width: 1.6rem;
    margin-right: var(--space-2);
    border-top: 2.5px dotted var(--pencil);
  }
  .dot {
    display: inline-grid;
    place-items: center;
    width: 1.9rem;
    height: 1.9rem;
    border: 2px solid currentColor;
    border-radius: 48% 52% 45% 55% / 55% 45% 55% 45%;
    font-weight: 700;
    line-height: 1;
  }
  .steps .done {
    color: var(--ink);
  }
  .steps .done .dot {
    background: var(--ink);
    border-color: var(--ink);
    color: var(--card);
  }
  .steps .now {
    color: var(--ink);
    font-weight: 700;
  }
  .steps .now .dot {
    background: var(--accent);
    border-color: var(--ink);
    color: var(--card);
  }
  .steps .later .dot {
    border-style: dashed;
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
  /* Out of the way of the step's own buttons: bottom right on wide screens,
     under the top edge on phones. */
  .reward-slot {
    position: fixed;
    z-index: 5;
    right: var(--space-6);
    bottom: var(--space-6);
  }
  @media (max-width: 34rem) {
    .reward-slot {
      right: auto;
      left: 50%;
      bottom: auto;
      top: var(--space-2);
      transform: translateX(-50%);
    }
  }
  .hush {
    text-align: center;
    font-family: var(--font-display);
    font-size: var(--size-hand);
    color: var(--ink-soft);
  }
  .player :global(.trouble) {
    max-width: 30rem;
    padding: var(--space-5);
  }
  .t-title {
    margin: 0;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: var(--size-hand);
  }
  .finished {
    display: grid;
    justify-items: center;
    padding-top: var(--space-3);
  }
  .finished :global(.done-card) {
    width: min(100%, 34rem);
    padding: var(--space-6) var(--space-5) var(--space-5);
    text-align: center;
  }
  .done-title {
    display: inline-flex;
    gap: var(--space-2);
    align-items: baseline;
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand-xl);
    line-height: 1;
    outline: none;
  }
  .tick {
    color: var(--ok);
  }
  .done-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--space-4);
    margin-top: var(--space-4);
  }
</style>
