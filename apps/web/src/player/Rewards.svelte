<!-- What a finished step earned: the XP floats up from the button (static
     with reduced motion), and streak or level news arrives as small inked notes. -->
<script lang="ts">
  import type { Reward } from "../core";
  import Card from "../lib/Card.svelte";
  import Icon from "../lib/Icon.svelte";

  let { rewards }: { rewards: Reward[] } = $props();

  const xp = $derived(rewards.reduce((n, r) => (r.kind === "xp" ? n + r.value : n), 0));
  const notes = $derived(
    rewards.flatMap((r): { icon: "flame" | "bulb" | "check"; text: string }[] => {
      switch (r.kind) {
        case "streak_extended":
          return [{ icon: "flame", text: r.value === 1 ? "Day 1 of your streak!" : `${r.value} days in a row!` }];
        case "level_up":
          return [{ icon: "bulb", text: `Level ${r.value}!` }];
        case "freeze_earned":
          return [{ icon: "check", text: "You earned a streak freeze." }];
        case "freeze_used":
          return [{ icon: "check", text: "A streak freeze kept your streak safe." }];
        default:
          return [];
      }
    }),
  );
</script>

<div class="rewards" role="status">
  {#if xp > 0}
    <p class="xp" data-testid="xp-float"><span class="sr-only">You earned</span> +{xp} xp</p>
  {/if}
  {#each notes as n, i}
    <Card tape tilt={i % 2 ? 1.2 : -1.2} edge="c" class="toast">
      <span class="icon"><Icon name={n.icon} size={26} /></span>
      <span>{n.text}</span>
    </Card>
  {/each}
</div>

<style>
  .rewards {
    display: grid;
    justify-items: center;
    gap: var(--space-3);
    pointer-events: none;
  }
  .xp {
    margin: 0;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: var(--size-hand-lg);
    color: var(--accent-deep);
    transform: rotate(-4deg);
    animation: float 1.6s var(--motion-ease) both;
  }
  .rewards :global(.toast) {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    font-family: var(--font-display);
    font-weight: 700;
    font-size: var(--size-hand);
    white-space: nowrap;
    animation: land 500ms cubic-bezier(0.3, 1.5, 0.5, 1) 300ms both;
  }
  .icon {
    color: var(--accent);
  }
  @keyframes float {
    from {
      opacity: 0;
      transform: translateY(26px) rotate(-4deg) scale(0.8);
    }
    40% {
      opacity: 1;
    }
  }
  @keyframes land {
    from {
      opacity: 0;
      transform: translateY(14px) rotate(-6deg);
    }
  }
</style>
