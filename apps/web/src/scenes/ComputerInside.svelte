<!-- what-is-a-computer: the parts of a computer, and one keypress travelling
     from the keyboard, through the CPU and RAM, to the screen. -->
<script lang="ts">
  import { roughLine, roughRect } from "../lib/sketch";

  const cpu = roughRect(196, 92, 88, 82, 11);
  const cpuCore = roughRect(214, 110, 52, 46, 12, 1.2);
  const ram = roughRect(320, 160, 136, 38, 13);
  const screen = roughRect(326, 22, 128, 86, 14);
  const glass = roughRect(336, 32, 108, 64, 15, 1);
  const keyboard = roughRect(28, 196, 150, 50, 16);
  const pins = [0, 1, 2, 3].flatMap((i) => [
    roughLine(208 + i * 21, 92, 208 + i * 21, 80, 20 + i, 0.6),
    roughLine(208 + i * 21, 174, 208 + i * 21, 186, 30 + i, 0.6),
    roughLine(196, 104 + i * 19, 184, 104 + i * 19, 40 + i, 0.6),
    roughLine(284, 104 + i * 19, 296, 104 + i * 19, 50 + i, 0.6),
  ]);
  const chips = [0, 1, 2, 3].map((i) => roughRect(332 + i * 31, 168, 22, 22, 60 + i, 0.8));
  const keys = [0, 1, 2].flatMap((row) =>
    Array.from({ length: 7 - (row === 2 ? 3 : 0) }, (_, k) =>
      roughRect(38 + k * 19 + (row === 2 ? 38 : 0), 203 + row * 13, 14, 9, 70 + row * 10 + k, 0.5),
    ),
  );
  const wires = [
    "M150 196 C160 170 175 150 184 140",
    "M296 150 C305 160 312 168 320 172",
    "M296 112 C312 100 330 106 344 108",
    "M184 112 C160 100 150 96 138 92",
  ];
</script>

<svg
  class="scene"
  viewBox="0 0 480 270"
  role="img"
  aria-label="Inside a computer: a keyboard sends a letter in, the CPU thinks about it, RAM holds it for now, storage keeps things for later, and the screen shows the letter back."
>
  <!-- storage: a drum of disks -->
  <g class="wob">
    <path class="ink card a-draw" pathLength="1" style:--d="0.1s" d="M44 46 C44 34 136 34 136 46 L136 112 C136 124 44 124 44 112 Z" />
    <path class="ink thin a-draw" pathLength="1" style:--d="0.3s" d="M44 46 C44 58 136 58 136 46" />
    <path class="ink thin soft a-draw" pathLength="1" style:--d="0.4s" d="M44 72 C44 84 136 84 136 72" />
    <path class="ink thin soft a-draw" pathLength="1" style:--d="0.5s" d="M44 96 C44 108 136 108 136 96" />
  </g>
  <text class="hand mid a-fade" style:--d="1.3s" x="90" y="146" font-size="22">storage</text>
  <text class="note mid a-fade" style:--d="1.5s" x="90" y="166" font-size="17">remembers for later</text>

  <!-- wires -->
  {#each wires as d, i}
    <path class="ink thin soft a-draw" pathLength="1" style:--d="{0.9 + i * 0.12}s" {d} />
  {/each}

  <!-- CPU -->
  <g class="wob">
    {#each pins as d, i}<path class="ink thin a-draw" pathLength="1" style:--d="{0.4 + i * 0.015}s" {d} />{/each}
    <path class="ink card a-draw" pathLength="1" style:--d="0.2s" d={cpu} />
    <path class="ink thin paper a-draw" pathLength="1" style:--d="0.5s" d={cpuCore} />
  </g>
  <text class="hand mid" x="240" y="141" font-size="24">CPU</text>
  <g class="spark a-pop" style:--d="2.6s">
    <path class="ink accent-line" d="M214 50 l-8 12 h9 l-8 13" />
  </g>
  <text class="note a-fade" style:--d="1.4s" x="222" y="66" font-size="18">thinks</text>

  <!-- RAM -->
  <g class="wob">
    <path class="ink card a-draw" pathLength="1" style:--d="0.3s" d={ram} />
    {#each chips as d, i}<path class="ink thin paper a-draw" pathLength="1" style:--d="{0.6 + i * 0.08}s" {d} />{/each}
  </g>
  <text class="hand a-fade" style:--d="1.4s" x="352" y="224" font-size="22">RAM</text>
  <text class="note a-fade" style:--d="1.6s" x="324" y="246" font-size="17">remembers for now</text>

  <!-- screen -->
  <g class="wob">
    <path class="ink card a-draw" pathLength="1" style:--d="0.25s" d={screen} />
    <path class="ink thin a-draw" pathLength="1" style:--d="0.45s" d={glass} />
    <path class="ink a-draw" pathLength="1" style:--d="0.6s" d="M382 110 L378 126 M398 110 L402 126 M366 128 C380 126 400 127 414 128" />
  </g>
  <text class="mono a-fade" style:--d="4.3s" x="346" y="72" font-size="22">h<tspan class="cursor">_</tspan></text>
  <text class="hand a-fade" style:--d="1.5s" x="356" y="16" font-size="19">screen <tspan class="note">· out</tspan></text>

  <!-- keyboard -->
  <g class="wob">
    <path class="ink card a-draw" pathLength="1" style:--d="0.35s" d={keyboard} />
    {#each keys as d, i}<path class="ink thin soft a-draw" pathLength="1" style:--d="{0.6 + i * 0.02}s" {d} />{/each}
  </g>
  <text class="hand a-fade" style:--d="1.6s" x="30" y="188" font-size="19">keyboard <tspan class="note">· in</tspan></text>

  <!-- one keypress, travelling through the parts -->
  <g class="trip" aria-hidden="true">
    <circle class="accentfill" r="11" />
    <text class="mono on-ink mid" y="5" font-size="14">h</text>
  </g>
</svg>

<style>
  .trip {
    opacity: 0;
    animation: trip 2.6s ease-in-out 1.8s both;
  }
  @keyframes trip {
    0% {
      opacity: 0;
      transform: translate(96px, 214px);
    }
    6% {
      opacity: 1;
      transform: translate(96px, 214px);
    }
    30%,
    42% {
      transform: translate(240px, 133px);
    }
    62%,
    72% {
      transform: translate(388px, 178px);
    }
    94% {
      opacity: 1;
      transform: translate(355px, 66px);
    }
    100% {
      opacity: 0;
      transform: translate(355px, 66px);
    }
  }
  .cursor {
    animation: blink 1s steps(1) 4.5s infinite;
  }
  @keyframes blink {
    50% {
      opacity: 0;
    }
  }
</style>
