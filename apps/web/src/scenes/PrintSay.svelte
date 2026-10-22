<!-- hello-print: a program is a list of steps run top to bottom; each
     Println makes the screen say one line. -->
<script lang="ts">
  import { roughLine, roughRect } from "../lib/sketch";

  const sheet = roughRect(22, 34, 246, 190, 31);
  const screen = roughRect(300, 40, 160, 134, 32);
  const steps = [
    { code: 'fmt.Println("Hello, city!")', out: "Hello, city!" },
    { code: "fmt.Println(3 + 4)", out: "7" },
    { code: 'fmt.Println("Lights:", 5)', out: "Lights: 5" },
  ];
  const t0 = 1.2;
  const gap = 1.1;
  const rules = [0, 1, 2, 3].map((i) => roughLine(36, 96 + i * 36, 256, 96 + i * 36, 33 + i, 1));
</script>

<svg
  class="scene"
  viewBox="0 0 480 270"
  role="img"
  aria-label="A program written as three steps on a sheet of paper. The computer follows them from top to bottom, and each fmt.Println makes the screen say one line: Hello, city!, then 7, then Lights: 5."
>
  <!-- the program on paper -->
  <g class="wob">
    <path class="ink card a-draw" pathLength="1" style:--d="0s" d={sheet} />
    {#each rules as d}<path class="ink thin pencil" {d} />{/each}
    <path class="ink thin accent-line a-draw" pathLength="1" style:--d="0.3s" d="M60 40 L60 220" />
  </g>
  <text class="hand a-fade" style:--d="0.4s" x="70" y="66" font-size="24">my program</text>

  {#each steps as s, i}
    {@const t = t0 + i * gap}
    <rect class="glow now" style:--d="{t}s" x="62" y={77 + i * 36} width="198" height="26" rx="4" />
    <text class="hand mid a-fade" style:--d="0.5s" x="44" y={96 + i * 36} font-size="20">{i + 1}</text>
    <text class="mono" x="68" y={95 + i * 36} font-size="11.6">{s.code}</text>
  {/each}

  <!-- top to bottom -->
  <g class="a-fade" style:--d="0.7s">
    <path class="ink thin soft" d="M12 84 C10 120 11 160 12 196 M6 188 L12 197 L18 188" />
  </g>
  <text class="note a-fade" style:--d="0.9s" x="24" y="250" font-size="18">top to bottom, one step at a time</text>

  <!-- the screen -->
  <g class="wob">
    <path class="ink inkfill a-draw" pathLength="1" style:--d="0.2s" d={screen} />
    <path class="ink a-draw" pathLength="1" style:--d="0.5s" d="M370 176 L366 196 M390 176 L394 196 M350 198 C372 196 392 197 412 198" />
  </g>
  {#each steps as s, i}
    <text class="mono on-ink a-fade" style:--d="{t0 + i * gap + 0.45}s" style:--t="200ms" x="314" y={74 + i * 30} font-size="17">
      {s.out}
    </text>
  {/each}
  <text class="hand a-fade" style:--d="1s" x="316" y="226" font-size="22">the screen says</text>

  <!-- each step's arrow to the screen -->
  {#each steps as _, i}
    <path
      class="ink thin accent-line a-draw"
      pathLength="1"
      style:--d="{t0 + i * gap + 0.15}s"
      style:--t="380ms"
      d="M262 {90 + i * 36} C278 {88 + i * 36} 286 {74 + i * 30} 298 {69 + i * 30}"
    />
  {/each}
</svg>

<style>
  /* Each line is highlighted while it runs; afterwards nothing is highlighted. */
  .now {
    opacity: 0;
    animation: now 1s ease-in-out var(--d) both;
  }
  @keyframes now {
    0%,
    100% {
      opacity: 0;
    }
    15%,
    80% {
      opacity: 0.75;
    }
  }
</style>
