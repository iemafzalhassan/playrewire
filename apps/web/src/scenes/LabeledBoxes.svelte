<!-- variables: boxes with names on them. Values drop in; later one box gets
     a new value (5 is lifted out, 6 goes in). -->
<script lang="ts">
  import { roughLine, roughRect } from "../lib/sketch";

  const boxes = [
    { x: 30, name: "name", code: 'name := "Mia"' },
    { x: 180, name: "lights", code: "lights := 5" },
    { x: 330, name: "city", code: 'city := "Dock St"' },
  ];
  const front = boxes.map((b, i) => roughRect(b.x, 150, 120, 74, 100 + i));
  const back = boxes.map((b, i) => roughLine(b.x + 10, 138, b.x + 110, 138, 110 + i, 1));
  const flapL = boxes.map((b, i) => roughLine(b.x, 150, b.x - 14, 128, 120 + i, 1));
  const flapR = boxes.map((b, i) => roughLine(b.x + 120, 150, b.x + 136, 126, 130 + i, 1));
  const tags = boxes.map((b, i) => roughRect(b.x + 22, 168, 76, 26, 140 + i, 1));
</script>

{#snippet slip(x: number, y: number, text: string, accent = false)}
  {@const w = Math.max(56, text.length * 10.5 + 18)}
  <rect class="ink {accent ? 'accentfill' : 'card'}" x={x - w / 2} y={y - 18} width={w} height="34" rx="6" />
  <text class="mono mid {accent ? 'on-ink' : ''}" x={x} y={y + 6} font-size="16" font-weight="700">{text}</text>
{/snippet}

<svg
  class="scene"
  viewBox="0 0 480 270"
  role="img"
  aria-label="Three boxes with name tags: name, lights and city. Mia drops into the name box, 5 into lights and Dock St into city. Then the 5 is lifted out of lights and a 6 goes in instead."
>
  {#each boxes as b, i}
    <!-- back edge and flaps behind the values -->
    <g class="wob">
      <path class="ink soft a-draw" pathLength="1" style:--d="{0.1 + i * 0.15}s" d={back[i]} />
      <path class="ink a-draw" pathLength="1" style:--d="{0.2 + i * 0.15}s" d={flapL[i]} />
      <path class="ink a-draw" pathLength="1" style:--d="{0.25 + i * 0.15}s" d={flapR[i]} />
    </g>
  {/each}

  <!-- values (drawn before the box fronts, so they sit inside) -->
  <g class="a-drop" style:--d="1.2s">{@render slip(90, 128, "\"Mia\"")}</g>
  <g class="five">{@render slip(240, 128, "5")}</g>
  <g class="a-drop" style:--d="3.4s">{@render slip(240, 128, "6", true)}</g>
  <g class="a-drop" style:--d="2.2s">{@render slip(390, 128, "\"Dock St\"")}</g>

  {#each boxes as b, i}
    <g class="wob">
      <path class="ink card a-draw" pathLength="1" style:--d="{i * 0.15}s" d={front[i]} />
      <path class="ink thin paper a-draw" pathLength="1" style:--d="{0.4 + i * 0.15}s" d={tags[i]} />
    </g>
    <text class="hand mid" x={b.x + 60} y="188" font-size="22">{b.name}</text>
    <text class="mono mid a-fade" style:--d="{i === 0 ? 1.2 : i === 1 ? 1.7 : 2.2}s" x={b.x + 60} y="246" font-size="11.5">
      {#if i === 1}<tspan class="old-code">{b.code}</tspan>{:else}{b.code}{/if}
    </text>
  {/each}
  <text class="mono mid new-code" x="240" y="246" font-size="11.5">lights = lights + 1</text>

  <!-- notes -->
  <text class="note a-fade" style:--d="0.8s" x="24" y="40" font-size="20">a box has a name…</text>
  <path class="ink thin accent-line a-draw" pathLength="1" style:--d="0.9s" d="M40 50 C24 90 26 140 48 176" />
  <text class="note a-fade" style:--d="3.4s" x="300" y="40" font-size="20">…and you can swap</text>
  <text class="note a-fade" style:--d="3.4s" x="330" y="62" font-size="20">what's inside</text>
  <path class="ink thin accent-line a-draw" pathLength="1" style:--d="3.6s" d="M302 52 C284 62 274 80 270 100" />
</svg>

<style>
  /* 5 drops in, then is lifted out when the box gets a new value. */
  .five {
    opacity: 0;
    animation: five 2.3s ease-in-out 1.7s both;
  }
  @keyframes five {
    0% {
      opacity: 0;
      transform: translateY(-110px);
      animation-timing-function: cubic-bezier(0.5, 0, 1, 1);
    }
    8% {
      opacity: 1;
    }
    24% {
      transform: translateY(0);
    }
    70% {
      opacity: 1;
      transform: translateY(0);
    }
    100% {
      opacity: 0;
      transform: translate(-30px, -120px) rotate(-12deg);
    }
  }
  .old-code {
    opacity: 0;
    animation: old 1.6s linear 1.7s both;
  }
  @keyframes old {
    0%,
    95% {
      opacity: 1;
    }
    100% {
      opacity: 0;
    }
  }
  .new-code {
    animation: scene-fade 400ms ease-out 3.5s both;
  }
</style>
