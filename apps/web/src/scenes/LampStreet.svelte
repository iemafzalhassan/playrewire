<!-- for-loop: the same job again and again. A counter i goes 0, 1, 2, 3, 4,
     and each round lights the next lamp on the street. -->
<script lang="ts">
  import { roughLine } from "../lib/sketch";

  const xs = [62, 152, 242, 332, 422];
  const t0 = 1.2;
  const gap = 0.7;
  const posts = xs.map((x, i) => roughLine(x, 214, x + (i % 2 ? 1 : -1), 112, 150 + i, 1.2));
  const street = roughLine(10, 214, 470, 214, 160, 2);
  const kerb = roughLine(10, 226, 470, 226, 161, 2);
</script>

<svg
  class="scene"
  viewBox="0 0 480 270"
  role="img"
  aria-label="A street with five dark lamps. A loop counts i from 0 to 4, and each round lights the next lamp, until every lamp on the street has had its turn."
>
  <g class="wob">
    <path class="ink a-draw" pathLength="1" d={street} />
    <path class="ink thin soft a-draw" pathLength="1" style:--d="0.2s" d={kerb} />
  </g>

  {#each xs as x, i}
    {@const t = t0 + i * gap}
    <g class="wob">
      <path class="ink a-draw" pathLength="1" style:--d="{0.2 + i * 0.08}s" d={posts[i]} />
      <path class="ink a-draw" pathLength="1" style:--d="{0.5 + i * 0.08}s" d="M{x} 112 C{x} 96 {x + 20} 94 {x + 24} 104" />
    </g>
    <circle class="glow halo a-pop" style:--d="{t}s" cx={x + 24} cy="116" r="24" />
    <path class="ink card a-fade" style:--d="{0.6 + i * 0.08}s" d="M{x + 14} 104 L{x + 34} 104 L{x + 30} 118 L{x + 18} 118 Z" />
    <path class="ink glow lit a-fade" style:--d="{t}s" style:--t="250ms" d="M{x + 14} 104 L{x + 34} 104 L{x + 30} 118 L{x + 18} 118 Z" />
    <!-- this round's counter -->
    <g class="a-pop" style:--d="{t}s">
      <rect class="ink thin card" x={x - 22} y="236" width="44" height="24" rx="5" />
      <text class="mono mid" x={x} y="253" font-size="14" font-weight="700">i={i}</text>
    </g>
  {/each}

  <!-- the loop goes round again -->
  <g class="a-fade" style:--d="0.9s">
    <path class="ink thin accent-line spin" d="M70 66 C130 30 350 30 410 66 M400 58 L411 67 L398 72" />
    <text class="note mid" x="240" y="24" font-size="20">again and again</text>
    <text class="mono mid" x="240" y="72" font-size="13">for i := 0; i &lt; 5; i++</text>
  </g>
</svg>

<style>
  .halo {
    opacity: 0.5;
  }
  .spin {
    stroke-dasharray: 6 5;
    animation: march 1.2s linear infinite;
  }
  @keyframes march {
    to {
      stroke-dashoffset: -22;
    }
  }
</style>
