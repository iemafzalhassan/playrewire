<!-- Stand-in for screens that are not built yet (the lesson player is Task 14). -->
<script lang="ts">
  import Card from "../lib/Card.svelte";
  import Icon from "../lib/Icon.svelte";

  let { title }: { title: string } = $props();
</script>

<section class="soon" aria-labelledby="page-title">
  <Card tape tilt={-1} edge="b">
    <h1 id="page-title" class="hand-title" tabindex="-1">{title}</h1>
    <p>This page is still being drawn.</p>
    <a class="back" href="#/"><Icon name="page" size={22} /> back to today</a>
  </Card>
</section>

<style>
  .soon {
    display: grid;
    justify-items: center;
    padding-top: var(--space-6);
  }
  .soon :global(.ink-card) {
    width: min(100%, 28rem);
    padding: var(--space-6) var(--space-5);
    text-align: center;
  }
  p {
    margin: var(--space-3) 0 var(--space-4);
  }
  .back {
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: var(--ink);
  }
</style>
