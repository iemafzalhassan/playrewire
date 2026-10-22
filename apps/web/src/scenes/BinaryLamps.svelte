<!-- bits-and-binary and binary-bridge: eight lamps on the Binary Bridge, one
     per bit. Reading 00001101 left to right lights a lamp for every 1: 13. -->
<script lang="ts">
  import { roughLine } from "../lib/sketch";

  const bits = "00001101";
  const places = [128, 64, 32, 16, 8, 4, 2, 1];
  const xs = places.map((_, i) => 58 + i * 52);
  const step = 0.42;
  const start = 1.3;
  const posts = xs.map((x, i) => roughLine(x, 196, x + (i % 2 ? 0.8 : -0.8), 118, 80 + i, 1));
  const deck = roughLine(20, 196, 460, 196, 90, 2);
  const rail = roughLine(20, 172, 460, 172, 91, 2);
</script>

<svg
  class="scene"
  viewBox="0 0 480 270"
  role="img"
  aria-label="Eight lamps on a bridge, one for each bit. The digits 0 0 0 0 1 1 0 1 are read from left to right, and a lamp lights for every 1: the lamps worth 8, 4 and 1 glow, which makes 13."
>
  <!-- bridge -->
  <g class="wob">
    <path class="ink a-draw" pathLength="1" style:--d="0s" d={deck} />
    <path class="ink thin soft a-draw" pathLength="1" style:--d="0.15s" d={rail} />
    {#each [0, 1, 2, 3, 4, 5, 6, 7, 8] as i}
      <path class="ink thin soft a-draw" pathLength="1" style:--d="{0.4 + i * 0.04}s" d="M{32 + i * 52} 172 L{34 + i * 52} 196" />
    {/each}
  </g>

  {#each xs as x, i}
    {@const on = bits[i] === "1"}
    {@const t = start + i * step}
    <!-- the lamp post -->
    <g class="wob">
      <path class="ink a-draw" pathLength="1" style:--d="{0.3 + i * 0.05}s" d={posts[i]} />
    </g>
    {#if on}
      <circle class="glow halo a-pop" style:--d="{t}s" cx={x} cy="104" r="22" />
    {/if}
    <circle class="ink {on ? 'glow' : 'card'} a-fade" style:--d="{0.6 + i * 0.05}s" cx={x} cy="104" r="14" />
    {#if on}
      <g class="a-fade" style:--d="{t}s">
        <path class="ink thin" d="M{x} 76 L{x} 66 M{x - 15} 82 L{x - 20} 76 M{x + 15} 82 L{x + 20} 76" />
      </g>
    {/if}
    <!-- the bit and what it is worth -->
    <text class="mono mid a-pop" style:--d="{t}s" x={x} y="226" font-size="26" font-weight="700">{bits[i]}</text>
    <text class="mid {on ? 'note' : 'hand'} a-fade" style:--d="{t}s" x={x} y="250" font-size="15" opacity={on ? 1 : 0.45}>
      {places[i]}
    </text>
  {/each}

  <!-- the reader's pencil moving left to right -->
  <g class="reader" aria-hidden="true">
    <path class="ink accent-line bold" d="M58 34 L58 52 M51 45 L58 53 L65 45" />
  </g>

  <text class="hand a-fade" style:--d="{start + 8 * step}s" x="240" y="30" font-size="28" text-anchor="middle">
    8 + 4 + 1 = <tspan class="note" font-size="34">13</tspan>
  </text>
</svg>

<style>
  .halo {
    opacity: 0.55;
  }
  .reader {
    opacity: 0;
    animation: read 3.36s steps(8, jump-none) 1.1s both;
  }
  @keyframes read {
    0% {
      opacity: 1;
      transform: translateX(0);
    }
    99% {
      opacity: 1;
      transform: translateX(364px);
    }
    100% {
      opacity: 0;
      transform: translateX(364px);
    }
  }
</style>
