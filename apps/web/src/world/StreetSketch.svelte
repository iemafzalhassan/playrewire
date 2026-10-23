<!-- The sketch street: an inked SVG of Dock St's 5 lights or the Binary Bridge's 8
     lamps, played by the same events as the 3D street. Shown when WebGL2 is missing
     or the 3D street fails to start, so the mission always works. Lit lights and lamps
     get pen rays, so "on" is never colour alone. -->
<script lang="ts">
  import type { SketchWorld } from "./sketch.svelte";

  let { world, kind }: { world: SketchWorld; kind: "light" | "lamp" } = $props();

  const lightX = (i: number) => 150 + i * 125;
  const lampX = (i: number) => 150 + i * 72;
  const list = (xs: boolean[]) => {
    const on = xs.flatMap((v, i) => (v ? [i] : []));
    return on.length ? on.join(", ") : "none";
  };
  const label = $derived(
    kind === "light"
      ? `Dock St, drawn in pen: 5 traffic lights, numbered 0 to 4. Green: ${list(world.lights)}.`
      : `The Binary Bridge, drawn in pen: 8 lamps, numbered 0 to 7. On: ${list(world.lamps)}.`,
  );
  const ring = $derived.by(() => {
    const t = world.ring;
    if (!t) return null;
    if (t.kind === "light" && kind === "light" && t.i < 5) return { x: lightX(t.i), y: 112, rx: 50, ry: 72 };
    if (t.kind === "lamp" && kind === "lamp" && t.i < 8) return { x: lampX(t.i), y: 104, rx: 30, ry: 32 };
    return null;
  });
  const rays = (cx: number, cy: number, r0: number, r1: number, n: number) =>
    Array.from({ length: n }, (_, k) => {
      const a = (k / n) * Math.PI * 2 + 0.3;
      return `M${cx + Math.cos(a) * r0} ${cy + Math.sin(a) * r0} L${cx + Math.cos(a) * r1} ${cy + Math.sin(a) * r1}`;
    }).join(" ");
</script>

<svg class="sketch" class:calm={world.reduced} viewBox="0 0 800 320" role="img" aria-label={label}>
  <g class="pen">
    {#if kind === "light"}
      <path class="ground" d="M10 262 C200 258 600 266 790 260" />
      <path class="road" d="M10 286 C220 282 580 290 790 284" />
      {#each [0, 1, 2, 3, 4] as i}
        {@const x = lightX(i)}
        {@const green = world.lights[i]}
        <path class="post" d="M{x} 260 L{x + 1} 160" />
        <rect class="housing" x={x - 22} y="62" width="44" height="98" rx="8" />
        <circle class="lamp red" class:on={!green} cx={x} cy="88" r="14" />
        <circle class="lamp green" class:on={green} cx={x} cy="134" r="14" />
        {#if green}<path class="rays green-rays" d={rays(x, 134, 20, 30, 8)} />{/if}
        <text class="num" x={x} y="305">{i}</text>
      {/each}
    {:else}
      <path class="water" d="M10 280 q30 -8 60 0 t60 0 t60 0 t60 0 t60 0 t60 0 t60 0 t60 0 t60 0 t60 0 t60 0 t60 0 t60 0" />
      <path class="deck" d="M60 190 L740 190 M60 204 L740 204" />
      <path class="arch" d="M120 204 C260 120 540 120 680 204" />
      <path class="rail" d="M100 150 L700 150" />
      {#each [0, 1, 2, 3, 4, 5, 6, 7] as i}
        {@const x = lampX(i)}
        {@const on = world.lamps[i]}
        <path class="post" d="M{x} 190 L{x} 118" />
        <circle class="bulb" class:on cx={x} cy="104" r="15" />
        {#if on}<path class="rays" d={rays(x, 104, 21, 31, 7)} />{/if}
        <text class="num" x={x} y="62">{i}</text>
      {/each}
    {/if}
    {#if ring}
      <ellipse class="ring" cx={ring.x} cy={ring.y} rx={ring.rx} ry={ring.ry} pathLength="1" />
    {/if}
  </g>
</svg>

<style>
  .sketch {
    display: block;
    width: 100%;
    height: 100%;
  }
  .pen {
    filter: url(#ink-wobble);
    stroke: var(--ink);
    stroke-width: 3;
    stroke-linecap: round;
    stroke-linejoin: round;
    fill: none;
  }
  .ground {
    stroke-width: 3.5;
  }
  .road {
    stroke: var(--pencil);
    stroke-dasharray: 24 18;
  }
  .housing {
    fill: var(--pencil);
  }
  .lamp {
    fill: color-mix(in srgb, var(--ink) 65%, var(--ink-soft));
    transition: fill var(--motion-base) var(--motion-ease);
  }
  .lamp.red.on {
    fill: var(--accent);
  }
  .lamp.green.on {
    fill: color-mix(in srgb, var(--ok) 70%, var(--highlight));
  }
  .rays {
    stroke: var(--accent-deep);
    stroke-width: 3.5;
  }
  .green-rays {
    stroke: var(--ok);
  }
  .bulb {
    fill: color-mix(in srgb, var(--pencil) 70%, var(--paper));
    transition: fill var(--motion-base) var(--motion-ease);
  }
  .bulb.on {
    fill: var(--highlight);
  }
  .water {
    stroke: color-mix(in srgb, var(--focus) 45%, var(--paper));
  }
  .deck,
  .arch,
  .rail {
    stroke-width: 3.5;
  }
  .rail {
    stroke: var(--accent-deep);
  }
  .num {
    fill: var(--ink);
    stroke: none;
    font-family: var(--font-display);
    font-size: 30px;
    font-weight: 700;
    text-anchor: middle;
  }
  .ring {
    stroke: var(--accent);
    stroke-width: 5;
    stroke-dasharray: 1;
    stroke-dashoffset: 1;
    animation: draw var(--motion-slow) var(--motion-ease) forwards;
  }
  .calm .ring {
    animation: none;
    stroke-dashoffset: 0;
  }
  .calm .lamp,
  .calm .bulb {
    transition: none;
  }
  @keyframes draw {
    to {
      stroke-dashoffset: 0;
    }
  }
</style>
