<!-- "today's page": three stat cards, the trail through the path, one action. -->
<script lang="ts">
  import Card from "../lib/Card.svelte";
  import DayTrack from "../lib/DayTrack.svelte";
  import type { DayPart } from "../today";
  import Icon from "../lib/Icon.svelte";
  import InkButton from "../lib/InkButton.svelte";
  import { smoothPath, type Pt } from "../lib/sketch";

  export interface TrailStop {
    title: string;
    done: boolean;
  }

  let {
    streak,
    xp,
    level,
    newIdeas,
    toFix,
    trail,
    nextTitle,
    day = [],
    onGo,
  }: {
    streak: number;
    xp: number;
    level: number;
    newIdeas: number;
    toFix: number;
    trail: TrailStop[];
    /** null when the whole path is done. */
    nextTitle: string | null;
    /** Today's three parts: warm-up, new page, fix-it. */
    day?: DayPart[];
    onGo: () => void;
  } = $props();

  const W = 560;
  const H = 130;
  const here = $derived(trail.findIndex((t) => !t.done));
  const points = $derived<Pt[]>(
    trail.map((_, i) => {
      const x = trail.length === 1 ? W / 2 : 24 + (i * (W - 48)) / (trail.length - 1);
      const y = 72 + Math.sin(i * 1.15 + 0.4) * 28 + (i % 2 ? 6 : -4);
      return [x, y];
    }),
  );
  const started = $derived(day.some((p) => p.state === "done" || p.state === "now"));
  const dayDone = $derived(day.some((p) => p.state === "done") && day.every((p) => p.state === "done" || p.state === "none"));
  const walked = $derived(here === -1 ? points : points.slice(0, Math.max(here, 0) + 1));
</script>

<section class="today" aria-labelledby="page-title">
  <header class="head">
    <h1 id="page-title" class="hand-title" tabindex="-1">
      today's page
      <svg class="scribble" viewBox="0 0 220 16" preserveAspectRatio="none" aria-hidden="true">
        <path d="M3 10 C40 5 80 12 120 7 S190 4 217 9" />
      </svg>
    </h1>
    <p class="meta">
      <span data-testid="level">level {level}</span>
      <span class="dot" aria-hidden="true">·</span>
      <span data-testid="xp">{xp} xp</span>
    </p>
  </header>

  <ul class="stats">
    <li>
      <Card tilt={-1.2} edge="a" data-testid="stat-streak">
        <span class="stat-icon accent"><Icon name="flame" size={28} /></span>
        <span class="big">{streak}</span>
        <span class="lbl">day streak</span>
      </Card>
    </li>
    <li>
      <Card tilt={1.4} edge="b" data-testid="stat-ideas">
        <span class="stat-icon"><Icon name="bulb" size={28} /></span>
        <span class="big">{newIdeas}</span>
        <span class="lbl">new ideas</span>
      </Card>
    </li>
    <li>
      <Card tilt={-0.8} edge="c" data-testid="stat-fix">
        <span class="stat-icon"><Icon name="wrench" size={28} /></span>
        <span class="big">{toFix}</span>
        <span class="lbl">to fix</span>
      </Card>
    </li>
  </ul>

  {#if trail.length}
    <figure class="trail">
      <svg viewBox="0 0 {W} {H}" aria-hidden="true">
        <path class="road" d={smoothPath(points)} />
        {#if walked.length > 1}<path class="walked" d={smoothPath(walked)} />{/if}
        {#each points as [x, y], i}
          {#if trail[i].done}
            <circle class="stop done" cx={x} cy={y} r="10" />
            <path class="tick" d="M{x - 5} {y + 0.5} l3.5 3.5 l6.5 -7.5" />
          {:else if i === here}
            <circle class="halo" cx={x} cy={y} r="20" />
            <circle class="stop here" cx={x} cy={y} r="12" />
            <text class="you" x={x} y={y - 29}>you</text>
          {:else}
            <circle class="stop todo" cx={x} cy={y} r="7.5" />
          {/if}
        {/each}
      </svg>
      <figcaption class="sr-only">
        <ol>
          {#each trail as t}<li>{t.title}: {t.done ? "done" : "not done yet"}</li>{/each}
        </ol>
      </figcaption>
    </figure>
  {/if}

  {#if day.length}
    <section class="plan" aria-labelledby="plan-title">
      <h2 id="plan-title" class="plan-title">today: about 15 minutes</h2>
      <DayTrack parts={day} />
    </section>
  {/if}

  <div class="go">
    {#if dayDone}
      <p class="next"><span aria-hidden="true">✓</span> Today's page is done. See you tomorrow!</p>
      {#if nextTitle}
        <InkButton primary onclick={onGo}>one more page <Icon name="arrow" size={30} /></InkButton>
      {:else}
        <InkButton primary onclick={onGo}>see the map <Icon name="arrow" size={30} /></InkButton>
      {/if}
    {:else if day.length || nextTitle}
      {#if nextTitle}<p class="next">next up: <strong>{nextTitle}</strong></p>{/if}
      <InkButton primary onclick={onGo}>{started ? "keep going" : "let's go"} <Icon name="arrow" size={30} /></InkButton>
    {:else}
      <p class="next">You finished every page so far. More are on the way.</p>
      <InkButton primary onclick={onGo}>see the map <Icon name="arrow" size={30} /></InkButton>
    {/if}
  </div>
</section>

<style>
  .today {
    display: grid;
    gap: var(--space-5);
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-2) var(--space-4);
  }
  .meta {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: var(--ink-soft);
    transform: rotate(1deg);
  }
  .dot {
    margin-inline: 0.3em;
  }
  .stats {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-4);
  }
  .stats :global(.ink-card) {
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-areas: "icon big" "lbl lbl";
    align-items: center;
    column-gap: var(--space-2);
    height: 100%;
  }
  .stat-icon {
    grid-area: icon;
    color: var(--ink-soft);
  }
  .stat-icon.accent {
    color: var(--accent);
  }
  .big {
    grid-area: big;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: var(--size-hand-xl);
    line-height: 0.95;
  }
  .lbl {
    grid-area: lbl;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: var(--ink-soft);
  }

  .trail {
    margin: 0;
  }
  .trail svg {
    display: block;
    width: 100%;
    height: auto;
    overflow: visible;
  }
  .road {
    fill: none;
    stroke: var(--ink-soft);
    stroke-width: 2.5;
    stroke-linecap: round;
    stroke-dasharray: 0.5 9;
  }
  .walked {
    fill: none;
    stroke: var(--ink);
    stroke-width: 3;
    stroke-linecap: round;
    filter: url(#ink-wobble);
  }
  .stop {
    stroke: var(--ink);
    stroke-width: 2.5;
  }
  .stop.done {
    fill: var(--ink);
  }
  .tick {
    fill: none;
    stroke: var(--card);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .stop.here {
    fill: var(--accent);
    stroke-width: 2.5;
  }
  .halo {
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.5;
    stroke-dasharray: 3 4;
    transform-box: fill-box;
    transform-origin: center;
    animation: turn 9s linear infinite;
  }
  .stop.todo {
    fill: var(--paper);
    stroke: var(--ink-soft);
  }
  .you {
    font-family: var(--font-display);
    font-size: 26px;
    font-weight: 700;
    fill: var(--accent-deep);
    text-anchor: middle;
  }
  @keyframes turn {
    to {
      transform: rotate(360deg);
    }
  }

  .plan {
    display: grid;
    gap: var(--space-3);
  }
  .plan-title {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand);
    font-weight: 700;
    line-height: 1;
  }
  .go {
    display: grid;
    justify-items: center;
    gap: var(--space-3);
    padding-top: var(--space-2);
  }
  .go :global([data-primary]) {
    width: min(100%, 26rem);
  }
  .next {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: var(--ink-soft);
    text-align: center;
  }
  .next strong {
    color: var(--ink);
  }

  @media (max-width: 34rem) {
    .stats {
      gap: var(--space-3);
    }
    .stats :global(.ink-card) {
      grid-template-columns: 1fr;
      grid-template-areas: "icon" "big" "lbl";
      padding: var(--space-3);
    }
    .big {
      font-size: var(--size-hand-lg);
    }
    .lbl {
      font-size: 1.15rem;
      line-height: 1.1;
    }
  }
</style>
