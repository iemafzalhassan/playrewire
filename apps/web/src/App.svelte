<script lang="ts">
  import { onMount, tick } from "svelte";
  import { getBackend } from "./backend";
  import { DEFAULT_SETTINGS, emptyProgress, type Settings } from "./backend/types";
  import { BundleSchema, type Bundle } from "./content/types";
  import { dueConcepts, initCore, level, type ProgressState } from "./core";
  import Icon from "./lib/Icon.svelte";
  import Card from "./lib/Card.svelte";
  import { onNotice } from "./notice";
  import { chunkDone, findChunk, nextChunk } from "./path";
  import { createRouter, href, type Route } from "./router.svelte";
  import Home from "./routes/Home.svelte";
  import Map from "./routes/Map.svelte";
  import SettingsPage from "./routes/Settings.svelte";
  import Soon from "./routes/Soon.svelte";
  import { applySettings } from "./settings";
  import tokens from "../../../design/tokens.json";

  const wobble = tokens.ink;

  const backend = getBackend();
  const router = createRouter();

  let settings = $state<Settings>({ ...DEFAULT_SETTINGS });
  let progress = $state<ProgressState>(emptyProgress());
  let bundle = $state<Bundle | null>(null);
  let phase = $state<"loading" | "ready" | "failed">("loading");
  let notice = $state<string | null>(null);
  let coreReady = $state(false);

  const today = (() => {
    const d = new Date();
    const p = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
  })();

  async function loadBundle(): Promise<Bundle> {
    const r = await fetch(`${import.meta.env.BASE_URL}content/go.json`);
    if (!r.ok) throw new Error(`content: HTTP ${r.status}`);
    return BundleSchema.parse(await r.json());
  }

  onMount(() => {
    const stopNotice = onNotice((t) => (notice = t));
    const calm = matchMedia("(prefers-reduced-motion: reduce)");
    const reapply = () => applySettings(settings);
    calm.addEventListener("change", reapply);

    (async () => {
      settings = await backend.getSettings();
      applySettings(settings);
      try {
        const [, p, b] = await Promise.all([initCore(), backend.loadProgress(), loadBundle()]);
        progress = p;
        bundle = b;
        coreReady = true;
        phase = "ready";
      } catch (e) {
        console.error(e);
        phase = "failed";
      }
    })();

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
    void backend.saveSettings(s);
  }

  const next = $derived(bundle ? nextChunk(bundle.path, progress, bundle) : null);

  const home = $derived.by(() => {
    if (!bundle) return null;
    const b = bundle;
    const chunks = b.path.map((r) => findChunk(b, r)).filter((c) => c !== undefined);
    const trail = chunks.map((c) => ({ title: c.title, done: chunkDone(c, progress.completed) }));
    return {
      streak: progress.streak.current,
      xp: progress.xp,
      level: coreReady ? level(progress.xp) : 1,
      newIdeas: trail.filter((t) => !t.done).length,
      toFix: coreReady ? dueConcepts(progress, today).length : 0,
      trail,
      nextTitle: next ? (findChunk(b, next)?.title ?? null) : null,
    };
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

  const nav: { route: Route; label: string; icon: "page" | "map" | "sliders" }[] = [
    { route: { name: "home" }, label: "today", icon: "page" },
    { route: { name: "map" }, label: "map", icon: "map" },
    { route: { name: "settings" }, label: "settings", icon: "sliders" },
  ];
  const isCurrent = (r: Route) => router.route.name === r.name;
  const learnTitle = $derived.by(() => {
    const r = router.route;
    if (r.name !== "learn" || !bundle) return "this page";
    return findChunk(bundle, r)?.title ?? "this page";
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

<main id="main">
  {#if router.route.name === "settings"}
    <SettingsPage {settings} desktop={backend.kind === "desktop"} onChange={saveSettings} />
  {:else if phase === "loading"}
    <p class="hush" aria-live="polite">opening the sketchbook…</p>
  {:else if phase === "failed" || !home}
    <Card tilt={-0.6} edge="b" class="oops">
      <h1 id="page-title" class="hand-title" tabindex="-1">The pages didn't load</h1>
      <p>Something went wrong while opening the lessons. Reload the page to try again.</p>
    </Card>
  {:else if router.route.name === "home"}
    <Home
      {...home}
      onGo={() => router.go(next ? { name: "learn", ...next } : { name: "map" })}
    />
  {:else if router.route.name === "map"}
    <Map
      {districts}
      here={next?.pack ?? null}
      onOpen={(ref) => router.go({ name: "learn", ...ref })}
    />
  {:else if router.route.name === "learn"}
    <Soon title={learnTitle} />
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
