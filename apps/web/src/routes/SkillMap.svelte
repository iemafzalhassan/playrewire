<!-- The skill map: one hand-drawn bar per concept seen so far (mastery is the
     review box out of 5), and a "keeps breaking" list for concepts that failed
     three or more times, each with a link to practise it. -->
<script lang="ts">
  import Card from "../lib/Card.svelte";
  import Icon from "../lib/Icon.svelte";
  import { roughLine, roughRect } from "../lib/sketch";
  import { MAX_BOX, type SkillMapData } from "../today";

  let { map }: { map: SkillMapData } = $props();

  const W = 300;
  const H = 30;
  /** Plain words for each box, so the bar never relies on its length alone. */
  const words = ["just started", "getting there", "getting there", "steady", "steady", "solid"];
  const seedOf = (s: string) => [...s].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 7);
</script>

<section class="skills" aria-labelledby="page-title">
  <h1 id="page-title" class="hand-title" tabindex="-1">
    skill map
    <svg class="scribble" viewBox="0 0 220 16" preserveAspectRatio="none" aria-hidden="true">
      <path d="M3 9 C40 12 90 5 140 9 S200 6 217 8" />
    </svg>
  </h1>

  {#if map.rows.length === 0}
    <Card tape edge="b" tilt={-0.6} class="empty">
      <p class="empty-title">Nothing here yet.</p>
      <p>Finish a drill and your first skill gets a bar here. Each review fills it a little more.</p>
      <a class="go" href="#/">to today's page <Icon name="arrow" size={24} /></a>
    </Card>
  {:else}
    <p class="intro">Each bar fills as you remember a skill on a later day. Five reviews in a row and it's solid.</p>
    <ul class="bars">
      {#each map.rows as r (r.concept)}
        {@const seed = seedOf(r.concept)}
        {@const fill = Math.max(0, r.mastery * (W - 8))}
        <li>
          <span class="name">{r.title}</span>
          <svg
            class="bar"
            viewBox="0 0 {W} {H}"
            preserveAspectRatio="none"
            role="meter"
            aria-label={r.title}
            aria-valuemin="0"
            aria-valuemax={MAX_BOX}
            aria-valuenow={r.box}
            aria-valuetext="{r.box} of {MAX_BOX}: {words[r.box]}"
          >
            <defs>
              <pattern id="hatch-{r.concept}" width="7" height="7" patternUnits="userSpaceOnUse" patternTransform="rotate(-35)">
                <line x1="0" y1="0" x2="0" y2="7" class="hatch" />
              </pattern>
            </defs>
            {#if fill > 0}
              <path class="fill" d={roughRect(4, 5, fill, H - 10, seed + 3, 1.2)} fill="url(#hatch-{r.concept})" />
            {/if}
            {#each [1, 2, 3, 4] as t}
              <path class="tick" d={roughLine(4 + (t * (W - 8)) / MAX_BOX, 6, 4 + (t * (W - 8)) / MAX_BOX, H - 6, seed + t, 0.6)} />
            {/each}
            <path class="edge" d={roughRect(3, 4, W - 6, H - 8, seed, 1.4)} />
          </svg>
          <span class="word">{words[r.box]}</span>
        </li>
      {/each}
    </ul>

    <Card edge="c" tilt={0.4} class="breaking">
      <h2 class="h2"><Icon name="wrench" size={26} /> keeps breaking</h2>
      {#if map.keepsBreaking.length}
        <p class="sub">These slipped a few times. That's normal: one more go helps them stick.</p>
        <ul class="broken" aria-label="Keeps breaking">
          {#each map.keepsBreaking as r (r.concept)}
            <li>
              <span class="b-name">{r.title}</span>
              <span class="b-count">slipped {r.fails} times</span>
              {#if r.practise}
                <a class="practise" href="#/practise/{encodeURIComponent(r.concept)}">practise it <Icon name="arrow" size={22} /></a>
              {/if}
            </li>
          {/each}
        </ul>
      {:else}
        <p class="sub"><span aria-hidden="true">✓</span> Nothing keeps breaking. Nice and steady.</p>
      {/if}
    </Card>
  {/if}
</section>

<style>
  .skills {
    display: grid;
    gap: var(--space-5);
  }
  .intro,
  .sub {
    margin: 0;
    color: var(--ink-soft);
  }
  .bars {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-3);
  }
  .bars li {
    display: grid;
    grid-template-columns: minmax(8rem, 13rem) minmax(0, 1fr) 7.5rem;
    align-items: center;
    gap: var(--space-3);
  }
  .name {
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    font-weight: 700;
    line-height: 1.1;
  }
  .bar {
    display: block;
    width: 100%;
    height: 2.2rem;
    overflow: visible;
  }
  .edge {
    fill: none;
    stroke: var(--ink);
    stroke-width: 2.2;
    stroke-linejoin: round;
    vector-effect: non-scaling-stroke;
  }
  .tick {
    fill: none;
    stroke: var(--pencil);
    stroke-width: 1.2;
    stroke-dasharray: 2 3;
    vector-effect: non-scaling-stroke;
  }
  .fill {
    stroke: var(--accent-deep);
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
    animation: grow var(--motion-slow) var(--motion-ease) both;
    transform-box: fill-box;
    transform-origin: left;
  }
  .hatch {
    stroke: var(--accent);
    stroke-width: 3;
  }
  @keyframes grow {
    from {
      transform: scaleX(0);
    }
  }
  .word {
    font-family: var(--font-display);
    font-size: 1.2rem;
    color: var(--ink-soft);
  }
  .h2 {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0 0 var(--space-2);
    font-family: var(--font-display);
    font-size: var(--size-hand);
    font-weight: 700;
    line-height: 1;
  }
  .skills :global(.breaking) {
    padding: var(--space-4) var(--space-5);
    display: grid;
    gap: var(--space-2);
  }
  .broken {
    list-style: none;
    margin: var(--space-2) 0 0;
    padding: 0;
    display: grid;
    gap: var(--space-2);
  }
  .broken li {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--space-1) var(--space-4);
    padding: var(--space-2) 0;
    border-bottom: 2px dotted var(--pencil);
  }
  .b-name {
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    font-weight: 700;
  }
  .b-count {
    color: var(--ink-soft);
    font-size: var(--size-text-sm);
  }
  .practise,
  .go {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-height: 2.75rem;
    margin-left: auto;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    font-weight: 700;
    color: var(--accent-deep);
  }
  .skills :global(.empty) {
    max-width: 32rem;
    padding: var(--space-5);
    display: grid;
    gap: var(--space-2);
  }
  .skills :global(.empty p) {
    margin: 0;
  }
  .empty-title {
    font-family: var(--font-display);
    font-size: var(--size-hand-lg);
    font-weight: 700;
    line-height: 1;
  }
  .go {
    margin-left: 0;
    justify-self: start;
  }
  @media (max-width: 34rem) {
    .bars li {
      grid-template-columns: minmax(0, 1fr) auto;
      grid-template-areas: "name word" "bar bar";
      row-gap: var(--space-1);
    }
    .name {
      grid-area: name;
    }
    .word {
      grid-area: word;
    }
    .bar {
      grid-area: bar;
    }
  }
</style>
