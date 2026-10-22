<!-- slices: a train of values, one car after another, numbered from 0. Then
     append couples a new car on the end and len counts four. -->
<script lang="ts">
  import { roughLine, roughRect } from "../lib/sketch";

  const cars = [
    { x: 104, v: "red" },
    { x: 192, v: "blue" },
    { x: 280, v: "green" },
  ];
  const gold = { x: 368, v: "gold" };
  const body = (x: number, seed: number) => roughRect(x, 128, 78, 54, seed);
  const rail = roughLine(6, 200, 474, 200, 170, 1.5);
  const sleepers = Array.from({ length: 16 }, (_, i) => roughLine(14 + i * 30, 196, 24 + i * 30, 208, 171 + i, 0.6));
</script>

{#snippet car(x: number, v: string, i: number, seed: number, accent = false)}
  <g class="wob">
    <path class="ink {accent ? 'glow' : 'card'}" d={body(x, seed)} />
  </g>
  <circle class="ink card" cx={x + 18} cy="188" r="9" />
  <circle class="ink card" cx={x + 60} cy="188" r="9" />
  <text class="mono mid" x={x + 39} y="161" font-size="15" font-weight="700">"{v}"</text>
  <circle class="ink thin paper" cx={x + 39} cy="232" r="14" />
  <text class="hand mid" x={x + 39} y="240" font-size="22">{i}</text>
  <path class="ink thin soft" d="M{x + 39} 218 L{x + 39} 206" />
{/snippet}

<svg
  class="scene"
  viewBox="0 0 480 270"
  role="img"
  aria-label="A train: an engine pulling three cars that hold red, blue and green. The cars are numbered 0, 1 and 2, so the first car is number 0. Then a fourth car holding gold rolls up and joins the end as number 3, and the train's length is 4."
>
  <g class="wob">
    <path class="ink a-draw" pathLength="1" d={rail} />
    {#each sleepers as d, i}<path class="ink thin soft a-draw" pathLength="1" style:--d="{0.1 + i * 0.03}s" {d} />{/each}
  </g>

  <!-- engine -->
  <g class="a-fade" style:--d="0.3s">
    <g class="wob">
      <path class="ink inkfill" d="M14 182 L14 120 L52 120 L52 100 L88 100 L88 182 Z" />
      <path class="ink" d="M24 120 L24 98 L36 98 L36 120" />
    </g>
    <rect class="card" x="58" y="108" width="22" height="18" rx="3" />
    <circle class="ink card" cx="32" cy="188" r="11" />
    <circle class="ink card" cx="72" cy="188" r="11" />
    <path class="ink thin soft puff" d="M30 84 C20 80 22 66 34 68 C36 56 54 58 52 70 C62 70 62 84 50 84 Z" />
  </g>

  {#each cars as c, i}
    <g class="a-fade" style:--d="{0.6 + i * 0.25}s">
      <path class="ink" d="M{c.x - 16} 170 L{c.x} 170" />
      {@render car(c.x, c.v, i, 180 + i)}
    </g>
  {/each}

  <!-- append: a new car rolls in and couples on -->
  <g class="rolls">
    <path class="ink" d="M{gold.x - 16} 170 L{gold.x} 170" />
    {@render car(gold.x, gold.v, 3, 190, true)}
  </g>

  <!-- notes -->
  <g class="a-fade" style:--d="1.6s">
    <text class="note" x="104" y="80" font-size="20">cars[0] is the first</text>
    <path class="ink thin accent-line" d="M136 88 C138 100 140 110 142 122 M135 114 L142 123 L148 113" />
  </g>
  <g class="a-fade" style:--d="3.6s">
    <text class="mono" x="300" y="48" font-size="14">append(cars, "gold")</text>
    <text class="hand" x="350" y="84" font-size="26">len = <tspan class="note" font-size="30">4</tspan></text>
  </g>
</svg>

<style>
  .rolls {
    animation: roll 1.4s cubic-bezier(0.2, 0.7, 0.3, 1) 2.2s both;
  }
  @keyframes roll {
    from {
      opacity: 0;
      transform: translateX(150px);
    }
    20% {
      opacity: 1;
    }
  }
  .puff {
    animation: puff 2.4s ease-in-out infinite;
  }
  @keyframes puff {
    50% {
      transform: translateY(-6px);
      opacity: 0.6;
    }
  }
</style>
