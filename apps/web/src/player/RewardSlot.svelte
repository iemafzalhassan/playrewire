<!-- Shows what a step earned for a moment, out of the way of the step's own
     buttons (bottom right; under the top edge on phones). Call show(rewards). -->
<script lang="ts">
  import type { Reward } from "../core";
  import Rewards from "./Rewards.svelte";

  let rewards = $state<Reward[]>([]);
  let key = $state(0);
  let timer: ReturnType<typeof setTimeout> | undefined;

  /** XP floats away quickly; streak and level notes stay a little longer. */
  export function show(r: Reward[]) {
    clearTimeout(timer);
    rewards = r;
    key++;
    timer = setTimeout(() => (rewards = []), r.some((x) => x.kind !== "xp") ? 6000 : 2800);
  }
  $effect(() => () => clearTimeout(timer));
</script>

{#key key}
  {#if rewards.length}
    <div class="reward-slot"><Rewards {rewards} /></div>
  {/if}
{/key}

<style>
  .reward-slot {
    position: fixed;
    z-index: 5;
    right: var(--space-6);
    bottom: var(--space-6);
  }
  @media (max-width: 34rem) {
    .reward-slot {
      right: auto;
      left: 50%;
      bottom: auto;
      top: var(--space-2);
      transform: translateX(-50%);
    }
  }
</style>
