<!-- Type: the learner types over a faint ghost of the code. Keys are caught on
     a focused region (no text field), so there is nothing to paste into, and
     paste, drop and the context menu are cancelled with a gentle note. Keys
     stay in memory as the ghost state; nothing is stored or sent. -->
<script lang="ts">
  import { onMount } from "svelte";
  import type { TypeStep } from "../content/types";
  import Card from "../lib/Card.svelte";
  import Icon from "../lib/Icon.svelte";
  import InkButton from "../lib/InkButton.svelte";
  import { PASTE_HINT, ghostKey, initialGhost, normalizeGhost } from "./ghost";

  let {
    step,
    onDone,
    nextLabel = "next: drill",
    autofocus = true,
  }: {
    step: TypeStep;
    nextLabel?: string;
    onDone: (r: { mistakes: number }) => void;
    autofocus?: boolean;
  } = $props();

  const ghost = $derived(normalizeGhost(step.ghost));
  let s = $state(initialGhost());
  let mistakes = $state(0);
  let focused = $state(false);
  let pasteHint = $state(false);
  let area: HTMLDivElement;

  const lines = $derived.by(() => {
    let start = 0;
    return ghost.split("\n").map((text) => {
      const l = { text, start };
      start += text.length + 1;
      return l;
    });
  });

  const tips = $derived(
    step.tips
      .map((t) => ({ ...t, idx: ghost.indexOf(t.match) }))
      .filter((t) => t.idx >= 0)
      .map((t) => {
        const line = ghost.slice(0, t.idx).split("\n").length - 1;
        return { ...t, line, col: t.idx - lines[line].start, end: t.idx + t.match.length };
      }),
  );
  const isShown = (t: { idx: number }) => s.done || s.correctUpTo >= t.idx;
  const latestTip = $derived([...tips].reverse().find(isShown) ?? null);

  const currentLine = $derived(Math.max(0, ghost.slice(0, s.correctUpTo).split("\n").length - 1));

  function cls(i: number): string {
    let c = "c";
    if (i === s.errorAt) c += " bad";
    else if (i < s.correctUpTo) c += " done";
    else if (i === s.correctUpTo && !s.done) c += " cur";
    else c += " todo";
    if (tips.some((t) => isShown(t) && i >= t.idx && i < t.end)) c += " tipped";
    return c;
  }

  const shownKey = (k: string) => (k === "\n" ? "Enter" : k === "\t" ? "Tab" : k === " " ? "space" : k);
  const wrongKey = $derived(s.errorAt !== null ? s.typed.slice(s.errorAt) : "");
  const expected = $derived(s.errorAt !== null ? ghost[s.errorAt] : "");

  function stop(e: Event) {
    e.preventDefault();
    pasteHint = true;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      area.blur();
      return;
    }
    if (e.isComposing) return;
    // AltGr arrives as Ctrl+Alt and types real characters ({ and [ on many layouts).
    const shortcut = e.metaKey || (e.ctrlKey && !e.altKey);
    if (shortcut) {
      if (e.key.toLowerCase() === "v") stop(e);
      return;
    }
    if (e.shiftKey && e.key === "Insert") return stop(e);
    if (s.done) return;
    const k = e.key;
    if (k === "Enter" || k === "Backspace" || k === "Tab" || [...k].length === 1) {
      e.preventDefault();
      const before = s.errorAt;
      s = ghostKey(ghost, s, k);
      if (before === null && s.errorAt !== null) mistakes++;
    }
  }

  onMount(() => {
    if (autofocus) area.focus();
  });
</script>

<section class="type-step" aria-label="Type">
  <p class="lead">Type over the faint code. Your fingers learn it by doing.</p>

  <Card edge="a" tilt={-0.3} class="sheet">
    <div
      class="ghost"
      bind:this={area}
      role="textbox"
      tabindex="0"
      aria-multiline="true"
      aria-label="Type the code"
      aria-describedby="type-help type-now"
      onkeydown={onKey}
      onpaste={stop}
      ondrop={stop}
      ondragover={(e) => e.preventDefault()}
      oncontextmenu={stop}
      onbeforeinput={stop}
      onfocus={() => (focused = true)}
      onblur={() => (focused = false)}
    >
      <div class="code" aria-hidden="true">
        {#each lines as line, li}
          <div class="line">
            {#each line.text as ch, ci}<span class={cls(line.start + ci)}>{ch}</span>{/each}{#if li < lines.length - 1}<span
                class="nl {cls(line.start + line.text.length)}">↵</span
              >{/if}
          </div>
          {#each tips.filter((t) => t.line === li) as t}
            <div class="tip" class:shown={isShown(t)} style:--col={t.col}>
              <svg class="arrow" viewBox="0 0 34 24" aria-hidden="true">
                <path d="M30 20 C20 20 9 16 6 4" />
                <path d="M2.5 9.5 L6 3.5 L11 8" />
              </svg>
              <span class="tip-text">{t.text}</span>
            </div>
          {/each}
        {/each}
      </div>
      {#if !focused && !s.done}
        <span class="wake" aria-hidden="true">tap here, then type</span>
      {/if}
    </div>
  </Card>

  <p id="type-help" class="sr-only">
    Type the code one character at a time. A wrong key is marked; press Backspace to fix it. Enter at the end of a
    line also skips the next line's indent, and Tab types an indent. Press Escape to leave the typing area. The code
    is: {ghost}
  </p>
  <p id="type-now" class="sr-only" aria-live="polite">
    {#if s.errorAt !== null}
      Wrong key. Expected {shownKey(expected)}. Press Backspace.
    {:else if latestTip}
      Tip: {latestTip.text}
    {/if}
  </p>

  <div class="status">
    {#if s.errorAt !== null}
      <p class="wrong-key">
        <span aria-hidden="true">✗</span>
        <span>That key was <kbd>{shownKey(wrongKey)}</kbd>, the code wants <kbd>{shownKey(expected)}</kbd>. Press Backspace to fix it.</span>
      </p>
    {:else if !s.done}
      <p class="where">line {currentLine + 1} of {lines.length}</p>
    {/if}
  </div>

  {#if pasteHint}
    <div class="paste-wrap">
      <Card tape tilt={1} edge="c" class="paste-note" role="status">
        <p>{PASTE_HINT}</p>
      </Card>
    </div>
  {/if}

  {#if s.done}
    <div class="finished">
      <p class="yay"><span class="tick" aria-hidden="true">✓</span> typed it, every character</p>
      <InkButton primary onclick={() => onDone({ mistakes })}>{nextLabel} <Icon name="arrow" size={28} /></InkButton>
    </div>
  {/if}
</section>

<style>
  .type-step {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-4);
  }
  .lead {
    margin: 0;
    font-size: var(--size-text-lg);
    color: var(--ink-soft);
  }
  .type-step :global(.sheet) {
    padding: 0;
    min-width: 0;
  }
  .ghost {
    position: relative;
    padding: var(--space-5) var(--space-5) var(--space-5) var(--space-5);
    overflow-x: auto;
    cursor: text;
    border-radius: var(--radius-wobble-a);
  }
  .ghost:focus-visible {
    outline-offset: -6px;
  }
  .code {
    font-family: var(--font-code);
    font-size: 1.15rem;
    line-height: 1.9;
    min-width: max-content;
  }
  .line {
    white-space: pre;
  }
  .c {
    position: relative;
    transition: color var(--motion-fast) linear;
  }
  .todo {
    color: color-mix(in srgb, var(--pencil) 70%, transparent);
  }
  .done {
    color: var(--ink);
  }
  .cur {
    color: var(--ink-soft);
    background: color-mix(in srgb, var(--highlight) 55%, transparent);
    border-radius: 2px;
    box-shadow: inset 0 -3px 0 var(--accent);
  }
  .ghost:not(:focus) .cur {
    background: none;
  }
  .ghost:focus .cur {
    animation: blink 1.1s steps(1) infinite;
  }
  .bad {
    color: var(--card);
    background: var(--accent-deep);
    border-radius: 3px;
  }
  /* A tiny pen cross above the wrong spot: never colour alone. */
  .bad::after {
    content: "✗";
    position: absolute;
    left: 92%;
    top: -0.62em;
    transform: rotate(-8deg);
    font-family: var(--font-display);
    font-size: 0.95rem;
    font-weight: 700;
    line-height: 1;
    color: var(--accent-deep);
  }
  .nl {
    font-size: 0.8em;
    padding-left: 0.15em;
  }
  .nl.todo,
  .nl.done {
    visibility: hidden;
  }
  .tipped.done {
    text-decoration: underline wavy color-mix(in srgb, var(--accent) 70%, transparent);
    text-decoration-thickness: 1.5px;
    text-underline-offset: 6px;
  }
  .tip {
    display: flex;
    align-items: flex-start;
    gap: var(--space-1);
    margin: -0.4rem 0 0.35rem min(calc(var(--col) * 1ch), max(0px, 100% - 26rem));
    max-width: min(30rem, calc(100vw - 6rem));
    white-space: normal;
    /* Pencilled in ahead of time, inked once the caret reaches the match. */
    opacity: 0.38;
    --tip-ink: var(--pencil);
  }
  .tip.shown {
    opacity: 1;
    --tip-ink: var(--accent-deep);
    animation: jot var(--motion-slow) var(--motion-ease) both;
  }
  .arrow {
    flex: none;
    width: 1.9rem;
    height: 1.4rem;
    margin-left: -0.3rem;
    overflow: visible;
  }
  .arrow path {
    fill: none;
    stroke: var(--tip-ink);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .tip-text {
    padding-top: 0.35rem;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    font-weight: 600;
    line-height: 1.05;
    color: var(--tip-ink);
    transform: rotate(-1deg);
  }
  .wake {
    position: absolute;
    right: var(--space-4);
    bottom: var(--space-2);
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: var(--accent-deep);
    transform: rotate(-2deg);
    pointer-events: none;
  }
  .status {
    min-height: 2.5rem;
  }
  .status p {
    margin: 0;
  }
  .where {
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: var(--ink-soft);
  }
  .wrong-key {
    display: flex;
    gap: var(--space-2);
    align-items: baseline;
  }
  .wrong-key > span:first-child {
    font-family: var(--font-display);
    font-weight: 700;
    font-size: var(--size-hand);
    color: var(--accent-deep);
  }
  kbd {
    padding: 0 0.35em;
    border: 1.5px solid var(--ink-soft);
    border-bottom-width: 3px;
    border-radius: 5px;
    background: var(--card);
    font-size: 0.9em;
  }
  .paste-wrap {
    display: grid;
    justify-items: center;
  }
  .paste-wrap :global(.paste-note) {
    padding: var(--space-3) var(--space-5);
  }
  .paste-wrap p {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand);
    font-weight: 600;
  }
  .finished {
    display: grid;
    justify-items: center;
    gap: var(--space-3);
  }
  .yay {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand);
    font-weight: 700;
  }
  .tick {
    color: var(--ok);
    font-size: 1.2em;
  }
  @keyframes blink {
    50% {
      box-shadow: inset 0 -3px 0 transparent;
    }
  }
  @keyframes jot {
    from {
      opacity: 0.38;
      transform: translateY(-3px);
    }
  }
</style>
