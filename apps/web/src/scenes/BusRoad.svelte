<!-- cpu-ram-bus: the CPU and RAM with a road of wires between them, and
     little bits of data riding it both ways, over and over. -->
<script lang="ts">
  import { roughLine, roughRect } from "../lib/sketch";

  const cpu = roughRect(24, 82, 96, 96, 21);
  const core = roughRect(42, 100, 60, 60, 22, 1.2);
  const ram = roughRect(360, 98, 100, 64, 23);
  const chips = [0, 1, 2].map((i) => roughRect(370 + i * 30, 110, 22, 40, 24 + i, 0.8));
  const pins = [0, 1, 2, 3].flatMap((i) => [
    roughLine(38 + i * 22, 82, 38 + i * 22, 70, 30 + i, 0.5),
    roughLine(38 + i * 22, 178, 38 + i * 22, 190, 40 + i, 0.5),
  ]);
  const edgeTop = roughLine(120, 108, 360, 110, 50, 3);
  const edgeBottom = roughLine(120, 156, 360, 154, 51, 3);
  const wires = [116, 122, 140, 146].map((y, i) => roughLine(124, y, 356, y + (i % 2 ? 1 : -1), 52 + i, 1.4));
  // [x, lane y, label, delay, where it sits when nothing moves]: right-going cars on top, left-going below
  const right = [
    [240, 119, "1", "0s", "60px"],
    [240, 119, "0", "-1.3s", "-50px"],
  ] as const;
  const left = [
    [240, 143, "42", "-0.6s", "-80px"],
    [240, 143, "1", "-1.9s", "20px"],
  ] as const;
</script>

<svg
  class="scene"
  viewBox="0 0 480 270"
  role="img"
  aria-label="The CPU on the left and RAM on the right, joined by a road of tiny wires called a bus. Little cars of data ride the road in both directions, again and again."
>
  <g class="wob">
    {#each pins as d, i}<path class="ink thin a-draw" pathLength="1" style:--d="{0.3 + i * 0.03}s" {d} />{/each}
    <path class="ink card a-draw" pathLength="1" style:--d="0.1s" d={cpu} />
    <path class="ink thin paper a-draw" pathLength="1" style:--d="0.4s" d={core} />
    <path class="ink card a-draw" pathLength="1" style:--d="0.25s" d={ram} />
    {#each chips as d, i}<path class="ink thin paper a-draw" pathLength="1" style:--d="{0.5 + i * 0.1}s" {d} />{/each}
  </g>
  <text class="hand mid" x="72" y="138" font-size="26">CPU</text>
  <text class="hand mid a-fade" style:--d="0.8s" x="410" y="190" font-size="26">RAM</text>

  <!-- the road -->
  <path class="paper a-fade" style:--d="0.6s" d="M120 108 L360 110 L360 154 L120 156 Z" />
  <g class="wob">
    <path class="ink a-draw" pathLength="1" style:--d="0.6s" d={edgeTop} />
    <path class="ink a-draw" pathLength="1" style:--d="0.7s" d={edgeBottom} />
  </g>
  {#each wires as d, i}<path class="ink thin soft a-draw" pathLength="1" style:--d="{0.9 + i * 0.1}s" {d} />{/each}
  <path class="ink thin pencil" d="M126 132 L356 132" />

  <!-- data cars -->
  {#each right as [x, y, label, delay, o]}
    <g transform="translate({x} {y})">
      <g class="car go-right" style:animation-delay={delay} style:--o={o}>
        <rect class="ink card" x="-15" y="-9" width="30" height="18" rx="5" />
        <text class="mono mid" y="5" font-size="13">{label}</text>
      </g>
    </g>
  {/each}
  {#each left as [x, y, label, delay, o]}
    <g transform="translate({x} {y})">
      <g class="car go-left" style:animation-delay={delay} style:--o={o}>
        <rect class="ink accentfill" x="-15" y="-9" width="30" height="18" rx="5" />
        <text class="mono mid on-ink" y="5" font-size="13">{label}</text>
      </g>
    </g>
  {/each}

  <!-- labels -->
  <g class="a-fade" style:--d="1.4s">
    <text class="hand mid" x="240" y="196" font-size="26">the bus</text>
    <path class="ink thin" d="M240 176 C238 170 239 166 240 160 M234 166 L240 159 L246 166" />
    <text class="note mid" x="240" y="222" font-size="18">a road of tiny wires</text>
  </g>
  <g class="a-fade" style:--d="1.8s">
    <path class="ink accent-line" d="M176 62 C210 50 270 50 304 62 M296 54 L305 62 L294 67" />
    <path class="ink accent-line" d="M304 84 C270 96 210 96 176 84 M184 92 L175 84 L186 79" />
    <text class="note mid" x="240" y="78" font-size="18">back and forth</text>
  </g>
  <text class="note mid a-fade" style:--d="2.2s" x="240" y="252" font-size="17">billions of trips every second</text>
</svg>

<style>
  .car {
    transform: translateX(var(--o, 0));
    animation: 2.6s linear infinite;
  }
  .go-right {
    animation-name: right;
  }
  .go-left {
    animation-name: left;
  }
  @keyframes right {
    0% {
      opacity: 0;
      transform: translateX(-104px);
    }
    10%,
    90% {
      opacity: 1;
    }
    100% {
      opacity: 0;
      transform: translateX(104px);
    }
  }
  @keyframes left {
    0% {
      opacity: 0;
      transform: translateX(104px);
    }
    10%,
    90% {
      opacity: 1;
    }
    100% {
      opacity: 0;
      transform: translateX(-104px);
    }
  }
</style>
