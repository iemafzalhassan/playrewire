<!-- An inked button. `primary` is the one thing to do on a screen (spec 6.2):
     accent fill, marked with data-primary. The edge "boils" on hover unless
     motion is reduced. -->
<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  let {
    primary = false,
    type = "button",
    children,
    class: cls = "",
    ...rest
  }: HTMLButtonAttributes & { primary?: boolean; children: Snippet } = $props();
</script>

<button {type} class="ink-btn {cls}" class:primary data-primary={primary ? "" : undefined} {...rest}>
  <span class="label">{@render children()}</span>
</button>

<style>
  .ink-btn {
    position: relative;
    isolation: isolate;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    min-height: 2.75rem;
    padding: var(--space-2) var(--space-5);
    border: 0;
    background: none;
    color: var(--ink);
    font-family: var(--font-display);
    font-size: var(--size-hand);
    font-weight: 700;
    line-height: 1;
    cursor: pointer;
    transition: transform var(--motion-fast) var(--motion-ease);
  }
  .ink-btn::before,
  .ink-btn::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: var(--radius-wobble-b);
    filter: url(#ink-wobble);
    transition: transform var(--motion-fast) var(--motion-ease);
  }
  .ink-btn::before {
    z-index: -1;
    background: var(--card);
    border: var(--ink-line) solid var(--ink);
  }
  .ink-btn::after {
    z-index: -2;
    background: var(--ink);
    transform: translate(var(--ink-shadow-x), var(--ink-shadow-y));
  }
  .primary {
    color: var(--card);
    font-size: var(--size-hand-lg);
    padding: var(--space-3) var(--space-6);
    min-height: 3.5rem;
  }
  .primary::before {
    background: var(--accent);
  }
  .label {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding-bottom: 0.1em;
  }
  .ink-btn:hover {
    transform: translate(-1px, -2px);
  }
  .ink-btn:hover::after {
    transform: translate(calc(var(--ink-shadow-x) + 1px), calc(var(--ink-shadow-y) + 2px));
  }
  .ink-btn:hover::before {
    animation: boil calc(var(--motion-boil-step) * 3) steps(1) infinite;
  }
  .ink-btn:active {
    transform: translate(var(--ink-shadow-x), var(--ink-shadow-y));
  }
  .ink-btn:active::after {
    transform: translate(0, 0);
  }
  .ink-btn:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }
  @keyframes boil {
    0% {
      filter: url(#ink-wobble);
    }
    33% {
      filter: url(#ink-wobble-2);
    }
    66% {
      filter: url(#ink-wobble-3);
    }
  }
</style>
