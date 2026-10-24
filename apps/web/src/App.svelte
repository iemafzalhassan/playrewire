<script lang="ts">
  import { onMount, tick } from "svelte";
  import { getBackend } from "./backend";
  import { DEFAULT_SETTINGS, emptyProgress, type Preset, type Settings } from "./backend/types";
  import type { Bundle } from "./content/types";
  import { loadBundle, loadTrouble } from "./content/load";
  import { initCore, type ProgressState } from "./core";
  import Icon from "./lib/Icon.svelte";
  import Card from "./lib/Card.svelte";
  import InkButton from "./lib/InkButton.svelte";
  import ToolchainCard from "./lab/ToolchainCard.svelte";
  import { createToolchain, setToolchainContext } from "./lab/toolchain.svelte";
  import { onNotice } from "./notice";
  import { chunkDone, findChunk, nextChunk } from "./path";
  import ChunkPlayer from "./player/ChunkPlayer.svelte";
  import FixIt from "./player/FixIt.svelte";
  import WarmUp from "./player/WarmUp.svelte";
  import { localDate } from "./player/drill";
  import { createRouter, href, type Route } from "./router.svelte";
  import Home from "./routes/Home.svelte";
  import Map from "./routes/Map.svelte";
  import SettingsPage from "./routes/Settings.svelte";
  import SkillMap from "./routes/SkillMap.svelte";
  import Soon from "./routes/Soon.svelte";
  import { applySettings } from "./settings";
  import {
    advanceDay,
    buildSkillMap,
    buildToday,
    dayParts,
    drillFor,
    loadDay,
    planDay,
    resumeDay,
    saveDay,
    type ChunkRef,
    type DayPlan,
    type DayStage,
  } from "./today";
  import tokens from "../../../design/tokens.json";

  const wobble = tokens.ink;

  const backend = getBackend();
  const router = createRouter();
  // Desktop: the one-time Go download, shared with every drill.
  const toolchain = backend.kind === "desktop" ? createToolchain(backend) : null;
  if (toolchain) setToolchainContext(toolchain);

  let settings = $state<Settings>({ ...DEFAULT_SETTINGS });
  let progress = $state<ProgressState>(emptyProgress());
  let bundle = $state<Bundle | null>(null);
  let phase = $state<"loading" | "ready" | "failed">("loading");
  let notice = $state<string | null>(null);
  let coreReady = $state(false);
  let presets = $state<Preset[]>([]);

  // The learner's own calendar day (local time, never UTC).
  const today = localDate();
  let trouble = $state(loadTrouble());

  async function load() {
    phase = "loading";
    try {
      settings = await backend.getSettings();
      applySettings(settings);
      const [, p, b] = await Promise.all([initCore(), backend.loadProgress(), loadBundle()]);
      progress = p;
      bundle = b;
      coreReady = true;
      phase = "ready";
    } catch (e) {
      console.error(e);
      trouble = loadTrouble();
      phase = "failed";
    }
  }

  onMount(() => {
    const stopNotice = onNotice((t) => (notice = t));
    const calm = matchMedia("(prefers-reduced-motion: reduce)");
    const reapply = () => applySettings(settings);
    calm.addEventListener("change", reapply);

    load();
    toolchain?.check();
    if (backend.kind === "desktop") {
      backend
        .getPresets()
        .then((p) => (presets = p))
        .catch((e) => console.error("could not load the mentor presets", e));
    }

    return () => {
      stopNotice();
      calm.removeEventListener("change", reapply);
      router.destroy();
    };
  });

  // Move focus to the new page's heading so screen readers hear the change.
  let firstRoute = true;
  $effect(() => {
    void router.route;
    if (firstRoute) {
      firstRoute = false;
      return;
    }
    tick().then(() => document.getElementById("page-title")?.focus());
  });

  function saveSettings(s: Settings) {
    settings = s;
    applySettings(s);
    backend.saveSettings(s).catch((e) => console.error("could not save settings", e));
  }

  const next = $derived(bundle ? nextChunk(bundle.path, progress, bundle) : null);
  // Today's page (spec 4.6): warm-up, the next new page, then a fix-it.
  const day = $derived(bundle && coreReady ? buildToday(progress, bundle, today) : null);

  const home = $derived.by(() => {
    if (!bundle || !day) return null;
    const b = bundle;
    const chunks = b.path.map((r) => findChunk(b, r)).filter((c) => c !== undefined);
    const trail = chunks.map((c) => ({ title: c.title, done: chunkDone(c, progress.completed) }));
    return {
      streak: day.streak,
      xp: day.xp,
      level: day.level,
      newIdeas: trail.filter((t) => !t.done).length,
      toFix: day.toFix,
      trail,
      nextTitle: next ? (findChunk(b, next)?.title ?? null) : null,
      day: dayParts(plan, day, b),
    };
  });

  // Which part of today the learner is on; kept across reloads for the same day.
  let plan = $state<DayPlan | null>(loadDay(today));
  const parts = $derived(bundle && day && plan ? dayParts(plan, day, bundle) : null);

  function setPlan(p: DayPlan) {
    plan = p;
    saveDay(p);
  }

  function goTo(p: DayPlan) {
    if (p.stage === "warmup") router.go({ name: "warmup" });
    else if (p.stage === "chunk" && p.chunk) router.go({ name: "learn", ...p.chunk });
    else if (p.stage === "fixit" && p.fixIt) router.go({ name: "fix", ...p.fixIt });
    else router.go({ name: "home" });
  }

  /** Home's button: start the day, or pick it up where it was left. */
  function startDay() {
    if (!bundle || !day) return;
    const p = plan && plan.date === today ? resumeDay(plan, day, progress, bundle) : planDay(day, today);
    setPlan(p);
    if (p.stage !== "done") return goTo(p);
    // The day's page is done: carry on with the path, outside the plan.
    router.go(next ? { name: "learn", ...next } : { name: "map" });
  }

  /** The current part is finished: on to the next one. */
  function partDone() {
    if (!plan || !day) return router.go({ name: "home" });
    const p = advanceDay(plan, day);
    setPlan(p);
    goTo(p);
  }

  const onwardWords: Record<DayStage, string> = {
    warmup: "next: warm-up",
    chunk: "next: new page",
    fixit: "next: fix-it",
    done: "finish the day",
  };
  /** The words for the button that leaves the current part. */
  const onwardLabel = () => (plan && day ? onwardWords[advanceDay(plan, day).stage] : "back to today");
  const sameRef = (a: ChunkRef | null, c: { pack: string; id: string }) => !!a && a.pack === c.pack && a.chunk === c.id;
  const inDay = (stage: DayStage) => plan?.date === today && plan.stage === stage;

  const skillMap = $derived(bundle && coreReady ? buildSkillMap(progress, bundle, today) : null);
  const allChunks = $derived(bundle ? bundle.packs.flatMap((p) => p.chunks) : []);
  const practise = $derived.by(() => {
    const r = router.route;
    if (r.name !== "practise" || !bundle || !coreReady) return [];
    const d = drillFor(progress, bundle, r.concept, today);
    return d ? [d] : [];
  });
  const fixChunk = $derived.by(() => {
    const r = router.route;
    if (r.name !== "fix" || !bundle) return undefined;
    return findChunk(bundle, r);
  });

  const districts = $derived.by(() => {
    if (!bundle) return [];
    const b = bundle;
    return b.packs.map((p) => {
      const inPath = b.path.filter((r) => r.pack === p.id);
      const chunks = inPath
        .map((r) => ({ c: findChunk(b, r), ref: r }))
        .filter((x) => x.c !== undefined)
        .map(({ c, ref }) => ({ title: c!.title, done: chunkDone(c!, progress.completed), ref }));
      return { id: p.id, name: p.district, done: chunks.filter((c) => c.done).length, total: chunks.length, chunks };
    });
  });

  const nav: { route: Route; label: string; icon: "page" | "map" | "bars" | "sliders" }[] = [
    { route: { name: "home" }, label: "today", icon: "page" },
    { route: { name: "map" }, label: "map", icon: "map" },
    { route: { name: "skills" }, label: "skills", icon: "bars" },
    { route: { name: "settings" }, label: "settings", icon: "sliders" },
  ];
  const isCurrent = (r: Route) => router.route.name === r.name;
  const learnChunk = $derived.by(() => {
    const r = router.route;
    if (r.name !== "learn" || !bundle) return undefined;
    return findChunk(bundle, r);
  });
</script>

<!-- Ink filters shared by every hand-drawn edge (tokens: ink-wobble-*). -->
<svg class="defs" width="0" height="0" aria-hidden="true" focusable="false">
  <defs>
    {#each [1, 2, 3] as seed}
      <filter id={seed === 1 ? "ink-wobble" : `ink-wobble-${seed}`} x="-4%" y="-4%" width="108%" height="108%">
        <feTurbulence
          type="fractalNoise"
          baseFrequency={wobble["wobble-frequency"]}
          numOctaves={wobble["wobble-octaves"]}
          seed={seed * 5}
          result="noise"
        />
        <feDisplacementMap
          in="SourceGraphic"
          in2="noise"
          scale={wobble["wobble-scale"]}
          xChannelSelector="R"
          yChannelSelector="G"
        />
      </filter>
    {/each}
  </defs>
</svg>

<a class="skip" href="#main">Skip to the page</a>

<header class="top">
  <a class="brand" href="#/" aria-label="Rewire, today's page">
    <svg class="wire" viewBox="0 0 64 32" aria-hidden="true">
      <path d="M2 22 C10 22 10 8 18 8 S26 24 34 22 S40 6 48 10 C52 12 53 17 57 17" />
      <circle cx="59" cy="17" r="3.2" />
    </svg>
    <span>Rewire</span>
  </a>
  <nav aria-label="Main">
    <ul>
      {#each nav as n}
        <li>
          <a href={href(n.route)} aria-current={isCurrent(n.route) ? "page" : undefined}>
            <Icon name={n.icon} size={22} />
            <span>{n.label}</span>
          </a>
        </li>
      {/each}
    </ul>
  </nav>
</header>

{#if notice}
  <div class="notice-wrap">
    <Card tape tilt={0.5} edge="c" class="notice" role="status">
      <p>{notice}</p>
      <button class="x" type="button" aria-label="Dismiss" onclick={() => (notice = null)}>
        <Icon name="close" size={18} />
      </button>
    </Card>
  </div>
{/if}

{#if toolchain?.needed && !["learn", "warmup", "practise", "fix"].includes(router.route.name)}
  <!-- First launch on desktop: Go isn't here yet. Inside a lesson the drill
       shows this card itself when Run needs Go; See and Type never wait. -->
  <div class="notice-wrap">
    <ToolchainCard {toolchain} />
  </div>
{/if}

<main id="main">
  {#if router.route.name === "settings"}
    <SettingsPage
      {settings}
      desktop={backend.kind === "desktop"}
      onChange={saveSettings}
      {presets}
      saveKey={(id, key) => backend.setApiKey(id, key)}
    />
  {:else if phase === "loading"}
    <p class="hush" aria-live="polite">opening the sketchbook…</p>
  {:else if phase === "failed" || !home}
    <Card tilt={-0.6} edge="b" class="oops">
      <h1 id="page-title" class="hand-title" tabindex="-1">{trouble.title}</h1>
      <p>{trouble.text}</p>
      <InkButton onclick={load}>try again</InkButton>
    </Card>
  {:else if router.route.name === "home"}
    <Home {...home} onGo={startDay} />
  {:else if router.route.name === "map"}
    <Map
      {districts}
      here={next?.pack ?? null}
      onOpen={(ref) => router.go({ name: "learn", ...ref })}
    />
  {:else if router.route.name === "skills" && skillMap}
    <SkillMap map={skillMap} />
  {:else if router.route.name === "warmup" && day}
    {#if inDay("warmup")}
      <WarmUp
        items={day.warmup}
        chunks={allChunks}
        {progress}
        {backend}
        day={parts}
        onProgress={(s) => (progress = s)}
        doneLabel={onwardLabel()}
        onDone={partDone}
      />
    {:else}
      <WarmUp
        items={day.warmup}
        chunks={allChunks}
        {progress}
        {backend}
        onProgress={(s) => (progress = s)}
        onDone={() => router.go({ name: "home" })}
      />
    {/if}
  {:else if router.route.name === "practise"}
    {#key router.route.concept}
      <WarmUp
        title="practise"
        items={practise}
        chunks={allChunks}
        {progress}
        {backend}
        onProgress={(s) => (progress = s)}
        doneLabel="back to the skill map"
        onDone={() => router.go({ name: "skills" })}
      />
    {/key}
  {:else if router.route.name === "fix" && fixChunk?.fixIt}
    {@const dayFix = inDay("fixit") && sameRef(plan!.fixIt, fixChunk)}
    {#key fixChunk.id}
      <FixIt
        chunk={fixChunk}
        {progress}
        {backend}
        day={dayFix ? parts : null}
        onProgress={(s) => (progress = s)}
        doneLabel={dayFix ? onwardLabel() : "back to today"}
        onDone={dayFix ? partDone : () => router.go({ name: "home" })}
      />
    {/key}
  {:else if router.route.name === "learn" && learnChunk}
    {@const dayChunk = inDay("chunk") && sameRef(plan!.chunk, learnChunk)}
    {#key learnChunk.id}
      <ChunkPlayer
        chunk={learnChunk}
        {progress}
        {backend}
        day={dayChunk ? parts : null}
        onward={dayChunk ? { label: onwardLabel(), go: partDone } : null}
        onProgress={(s) => (progress = s)}
        onHome={() => router.go({ name: "home" })}
      />
    {/key}
  {:else}
    <Soon title="Lost page" />
  {/if}
</main>

<style>
  .defs {
    position: absolute;
  }
  .skip {
    position: absolute;
    left: var(--space-4);
    top: -4rem;
    z-index: 10;
    padding: var(--space-2) var(--space-4);
    background: var(--ink);
    color: var(--card);
  }
  .skip:focus {
    top: var(--space-2);
  }
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    max-width: var(--size-page);
    margin: 0 auto;
    padding: var(--space-4) var(--space-4) 0;
  }
  .brand {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--ink);
    text-decoration: none;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 2.25rem;
    line-height: 1;
  }
  .wire {
    width: 2.6rem;
    height: 1.3rem;
    overflow: visible;
  }
  .wire path {
    fill: none;
    stroke: var(--ink);
    stroke-width: 3;
    stroke-linecap: round;
  }
  .wire circle {
    fill: var(--accent);
    stroke: var(--ink);
    stroke-width: 2;
  }
  nav ul {
    list-style: none;
    display: flex;
    gap: var(--space-1);
    margin: 0;
    padding: 0;
  }
  nav a {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-3);
    min-height: 2.75rem;
    color: var(--ink-soft);
    text-decoration: none;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    font-weight: 600;
  }
  nav a:hover {
    color: var(--ink);
  }
  nav a[aria-current="page"] {
    color: var(--ink);
  }
  /* The current tab is circled in pen, like a note in the margin. */
  nav a[aria-current="page"]::after {
    content: "";
    position: absolute;
    inset: 0.2rem 0;
    border: 2px solid var(--accent);
    border-radius: 48% 52% 45% 55% / 55% 45% 55% 45%;
    transform: rotate(-2deg);
    filter: url(#ink-wobble);
  }
  main {
    max-width: var(--size-page);
    margin: 0 auto;
    padding: var(--space-5) var(--space-4) var(--space-8);
  }
  .notice-wrap {
    max-width: var(--size-page);
    margin: var(--space-4) auto 0;
    padding: 0 var(--space-4);
    display: grid;
    justify-items: center;
  }
  .notice-wrap :global(.notice) {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3) var(--space-2) var(--space-4);
  }
  .notice-wrap p {
    margin: 0;
  }
  .x {
    display: grid;
    place-items: center;
    width: 2.75rem;
    height: 2.75rem;
    border: 0;
    background: none;
    color: var(--ink-soft);
    cursor: pointer;
  }
  .x:hover {
    color: var(--ink);
  }
  .hush {
    text-align: center;
    font-family: var(--font-display);
    font-size: var(--size-hand);
    color: var(--ink-soft);
  }
  main :global(.oops) {
    max-width: 30rem;
    margin: var(--space-6) auto;
    padding: var(--space-5);
  }
  @media (max-width: 34rem) {
    .top {
      flex-direction: column;
      align-items: flex-start;
      gap: var(--space-2);
    }
    nav {
      align-self: stretch;
    }
    nav ul {
      justify-content: space-between;
    }
  }
</style>
