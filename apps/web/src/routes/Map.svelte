<!-- The city map is the curriculum (spec 4.2). Open districts are inked in;
     the rest are blank paper with a pencilled name, announced as not yet open. -->
<script lang="ts">
  import Card from "../lib/Card.svelte";
  import Icon from "../lib/Icon.svelte";
  import InkButton from "../lib/InkButton.svelte";
  import { blob, smoothPath, type Pt } from "../lib/sketch";
  import type { ChunkRef } from "../path";

  export interface DistrictChunk {
    title: string;
    done: boolean;
    ref: ChunkRef;
  }
  export interface OpenDistrict {
    id: string;
    name: string;
    done: number;
    total: number;
    chunks: DistrictChunk[];
  }

  let {
    districts,
    here = null,
    onOpen,
  }: {
    districts: OpenDistrict[];
    /** The district holding the next page, marked "you are here". */
    here?: string | null;
    onOpen: (ref: ChunkRef) => void;
  } = $props();

  // All ten districts in unlock order (spec 4.2), placed along one road that
  // climbs from silicon to the summit. Wide screens get a landscape sheet;
  // phones get a tall one so the names stay readable.
  const NAMES = [
    "Silicon Quarter",
    "The Wire",
    "Terminal Alley",
    "Git Archive",
    "The Underground",
    "Go Academy",
    "DSA Arena",
    "Data Vaults",
    "Architect's Tower",
    "The Summit",
  ];
  type Spot = { at: Pt; rx: number; ry: number };
  const WIDE: { w: number; h: number; spots: Spot[] } = {
    w: 1000,
    h: 660,
    spots: [
      { at: [150, 520], rx: 120, ry: 78 },
      { at: [370, 560], rx: 95, ry: 58 },
      { at: [580, 520], rx: 92, ry: 56 },
      { at: [800, 545], rx: 108, ry: 62 },
      { at: [840, 360], rx: 112, ry: 66 },
      { at: [580, 330], rx: 128, ry: 84 },
      { at: [320, 330], rx: 102, ry: 66 },
      { at: [150, 165], rx: 104, ry: 62 },
      { at: [450, 140], rx: 116, ry: 62 },
      { at: [775, 150], rx: 112, ry: 70 },
    ],
  };
  const TALL: typeof WIDE = {
    w: 600,
    h: 1200,
    spots: [
      { at: [165, 1085], rx: 150, ry: 78 },
      { at: [440, 975], rx: 130, ry: 58 },
      { at: [175, 870], rx: 140, ry: 58 },
      { at: [435, 765], rx: 130, ry: 58 },
      { at: [190, 650], rx: 160, ry: 62 },
      { at: [420, 530], rx: 150, ry: 80 },
      { at: [170, 415], rx: 135, ry: 60 },
      { at: [430, 305], rx: 135, ry: 58 },
      { at: [190, 190], rx: 165, ry: 62 },
      { at: [430, 80], rx: 140, ry: 58 },
    ],
  };

  let width = $state(1000);
  const sheet = $derived(width < 600 ? TALL : WIDE);
  const city = $derived(NAMES.map((name, i) => ({ name, ...sheet.spots[i] })));
  const road = $derived(smoothPath(sheet.spots.map((d) => d.at)));
  const byName = $derived(new Map(districts.map((d) => [d.name, d])));

  let picked = $state<string | null>(null);
  const selected = $derived(
    districts.find((d) => d.id === picked) ?? districts.find((d) => d.id === here) ?? null,
  );

  function choose(d: OpenDistrict) {
    picked = d.id;
  }
  function key(e: KeyboardEvent, d: OpenDistrict) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      choose(d);
    }
  }
  const nextIn = (d: OpenDistrict) => d.chunks.find((c) => !c.done) ?? d.chunks[0];
</script>

<svelte:window bind:innerWidth={width} />

<section class="map" aria-labelledby="page-title">
  <h1 id="page-title" class="hand-title" tabindex="-1">
    the city
    <svg class="scribble" viewBox="0 0 220 16" preserveAspectRatio="none" aria-hidden="true">
      <path d="M3 9 C50 13 90 4 140 8 S200 11 217 6" />
    </svg>
  </h1>

  <div class="sheet">
    <svg class="city" viewBox="0 0 {sheet.w} {sheet.h}" role="group" aria-label="City map">
      <defs>
        <pattern id="hatch" width="9" height="9" patternUnits="userSpaceOnUse" patternTransform="rotate(38)">
          <line x1="0" y1="0" x2="0" y2="9" class="hatch-line" />
        </pattern>
      </defs>

      <path class="road" d={road} aria-hidden="true" />

      {#each city as c, i (c.name)}
        {@const d = byName.get(c.name)}
        {@const shape = blob(c.at[0], c.at[1], c.rx, c.ry, i * 7 + 3)}
        {#if d}
          <g
            class="district open"
            class:selected={selected?.id === d.id}
            role="button"
            tabindex="0"
            aria-label="{d.name}, open, {d.done} of {d.total} pages done"
            aria-pressed={selected?.id === d.id}
            onclick={() => choose(d)}
            onkeydown={(e) => key(e, d)}
          >
            <path class="shadow" d={shape} transform="translate(5 7)" />
            <path class="fill" d={shape} />
            <path class="hatch" d={shape} />
            <path class="edge" d={shape} pathLength="1" />
            <text class="name" x={c.at[0]} y={c.at[1] - 4}>{d.name}</text>
            <text class="count" x={c.at[0]} y={c.at[1] + 30}>{d.done} / {d.total}</text>
            {#if here === d.id}
              <g class="pin" transform="translate({c.at[0] + c.rx * 0.62} {c.at[1] - c.ry * 0.78})">
                <path d="M0 0 C-11 -14 -12 -30 0 -31 C12 -30 11 -14 0 0 Z" />
                <circle cx="0" cy="-20" r="4.5" />
              </g>
            {/if}
          </g>
        {:else}
          <g class="district closed" role="img" aria-label="{c.name}, not yet open">
            <path class="edge" d={shape} />
            <text class="name" x={c.at[0]} y={c.at[1] + 8}>{c.name}</text>
          </g>
        {/if}
      {/each}

      <g class="compass" transform="translate({sheet.w - 45} {sheet.h - 50})" aria-hidden="true">
        <path d="M0 -26 L6 -4 L0 0 L-6 -4 Z" class="needle" />
        <path d="M0 26 L6 4 L0 0 L-6 4 Z" />
        <text x="0" y="-32">N</text>
      </g>
    </svg>
  </div>

  {#if selected}
    {@const next = nextIn(selected)}
    <Card tape tilt={-0.6} edge="b" class="note">
      <h2 class="note-title">{selected.name}</h2>
      {#if selected.chunks.length}
        <ol class="pages">
          {#each selected.chunks as c}
            <li class:done={c.done}>
              <Icon name={c.done ? "check" : "circle"} size={20} label={c.done ? "done" : "not done yet"} />
              <span>{c.title}</span>
            </li>
          {/each}
        </ol>
      {/if}
      {#if next}
        <InkButton onclick={() => onOpen(next.ref)}>
          {next.done ? "look again" : "open"} <Icon name="arrow" size={24} />
        </InkButton>
      {/if}
    </Card>
  {:else}
    <p class="tip">Tap an inked district.</p>
  {/if}
</section>

<style>
  .map {
    display: grid;
    gap: var(--space-4);
  }
  .sheet {
    margin-inline: calc(var(--space-2) * -1);
  }
  .city {
    display: block;
    width: 100%;
    height: auto;
    overflow: visible;
  }
  .road {
    fill: none;
    stroke: var(--ink-soft);
    stroke-width: 4;
    stroke-linecap: round;
    stroke-dasharray: 0.5 13;
  }

  .district .name {
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 34px;
    text-anchor: middle;
  }
  .closed .edge {
    fill: color-mix(in srgb, var(--paper) 70%, var(--card));
    stroke: var(--pencil);
    stroke-width: 2;
    stroke-dasharray: 7 6;
    filter: url(#ink-wobble);
  }
  .closed .name {
    fill: var(--ink-soft);
    font-weight: 600;
    font-size: 27px;
    opacity: 0.85;
  }

  .open {
    cursor: pointer;
    outline: none;
  }
  .open .shadow {
    fill: var(--ink);
    filter: url(#ink-wobble);
  }
  .open .fill {
    fill: var(--card);
  }
  .open .hatch {
    fill: url(#hatch);
  }
  .hatch-line {
    stroke: var(--ink);
    stroke-width: 1.4;
    opacity: 0.16;
  }
  .open .edge {
    fill: none;
    stroke: var(--ink);
    stroke-width: 3;
    stroke-linejoin: round;
    filter: url(#ink-wobble);
    stroke-dasharray: 1;
    stroke-dashoffset: 0;
    animation: draw var(--motion-draw) var(--motion-ease) both;
  }
  .open .name {
    fill: var(--ink);
    paint-order: stroke;
    stroke: var(--card);
    stroke-width: 7px;
    stroke-linejoin: round;
  }
  .open .count {
    font-family: var(--font-display);
    font-size: 24px;
    font-weight: 600;
    fill: var(--ink-soft);
    text-anchor: middle;
    paint-order: stroke;
    stroke: var(--card);
    stroke-width: 6px;
  }
  .open.selected .fill {
    fill: color-mix(in srgb, var(--highlight) 45%, var(--card));
  }
  .open.selected .edge {
    stroke: var(--accent-deep);
    stroke-width: 3.5;
  }
  .open:focus-visible .edge {
    stroke: var(--focus);
    stroke-width: 5;
  }
  .open:hover .edge {
    animation: boil calc(var(--motion-boil-step) * 3) steps(1) infinite;
  }
  .pin path {
    fill: var(--accent);
    stroke: var(--ink);
    stroke-width: 2.5;
  }
  .pin circle {
    fill: var(--card);
    stroke: var(--ink);
    stroke-width: 2;
  }
  .compass path {
    fill: var(--paper);
    stroke: var(--ink-soft);
    stroke-width: 2;
  }
  .compass .needle {
    fill: var(--ink-soft);
  }
  .compass text {
    font-family: var(--font-display);
    font-size: 22px;
    fill: var(--ink-soft);
    text-anchor: middle;
  }
  @keyframes draw {
    from {
      stroke-dashoffset: 1;
    }
  }
  @keyframes boil {
    0% {
      filter: url(#ink-wobble);
    }
    33% {
      filter: url(#ink-wobble-2);
    }
    66% {
      filter: url(#ink-wobble-3);
    }
  }

  .map :global(.note .ink-btn) {
    justify-self: start;
  }
  .map :global(.note) {
    justify-self: center;
    width: min(100%, 30rem);
    display: grid;
    gap: var(--space-3);
    padding: var(--space-5);
  }
  .note-title {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand-lg);
    line-height: 1;
  }
  .pages {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-2);
  }
  .pages li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--ink-soft);
  }
  .pages li.done {
    color: var(--ink);
  }
  .pages li.done :global(.icon) {
    color: var(--ok);
  }
  .tip {
    margin: 0;
    text-align: center;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: var(--ink-soft);
  }
</style>
