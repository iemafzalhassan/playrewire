<!-- Help for one exercise: the AI mentor's nudge (desktop, when one is set up) or
     the lesson's own hint ladder. The mentor's reply streams in as a handwritten
     note (all at once with reduced motion). On any RUN_ERROR the panel says so and
     switches to the built-in hints. Only the HintRequest fields ever leave here:
     the exercise, the code, the last error and the attempt number. -->
<script lang="ts">
  import { untrack } from "svelte";
  import type { AgUiEvent, Backend, HintRequest } from "../backend/types";
  import { MENTOR_FALLBACK } from "../backend/types";
  import Icon from "../lib/Icon.svelte";
  import InkButton from "../lib/InkButton.svelte";

  let {
    mentor = null,
    hints,
    request,
    onHelp = () => {},
    active = true,
  }: {
    /** Null: built-in hints only, with no mentor line (e.g. an exercise without a backend). */
    mentor?: Pick<Backend, "kind" | "mentorAvailable" | "mentorHint"> | null;
    /** The lesson's built-in hints, revealed one at a time. */
    hints: string[];
    /** Built when the learner asks; trimmed to the HintRequest fields before it is sent. */
    request: () => HintRequest;
    /** Each built-in hint shown and each mentor reply counts as help used. */
    onHelp?: () => void;
    /** False once the exercise is solved: the buttons go, the notes stay. */
    active?: boolean;
  } = $props();

  /** The longest code we send; drills are a few lines, so this only stops accidents. */
  const MAX_CODE = 20_000;

  // Asked once: the panel belongs to one exercise, and Settings changes apply to the next one.
  const available = untrack(() => mentor?.mentorAvailable() ?? false);
  let mode = $state<"mentor" | "builtin">(available ? "mentor" : "builtin");
  let shown = $state(0);
  let streaming = $state(false);
  /** The note as pieces, so each new piece can be inked in. */
  let pieces = $state<string[]>([]);
  let trouble = $state<string | null>(null);
  let run = 0;

  const calm = () => document.documentElement.dataset.motion === "reduced";

  /** Exactly the five HintRequest fields: no names, ids or settings, whatever the caller built. */
  function trim(r: HintRequest): HintRequest {
    return {
      concept: String(r.concept),
      exercise_prompt: String(r.exercise_prompt),
      code: String(r.code).slice(0, MAX_CODE),
      last_error: r.last_error === null || r.last_error === undefined ? null : String(r.last_error),
      attempt: Number.isFinite(r.attempt) ? Math.max(0, Math.floor(r.attempt)) : 0,
    };
  }

  function showHint() {
    if (shown >= hints.length) return;
    shown++;
    onHelp();
  }

  /** When there's no built-in hint to fall back to (below the blank-page level). */
  const NO_HINTS_FALLBACK = "The mentor couldn't connect. Look at the faint code and try one small change.";

  function fallBack(message: string) {
    streaming = false;
    pieces = [];
    // "…so here's a built-in hint" would promise something that isn't there.
    trouble = message === MENTOR_FALLBACK && hints.length === 0 ? NO_HINTS_FALLBACK : message;
    mode = "builtin";
    // Straight onto the lesson's own ladder, so the learner isn't left empty-handed.
    showHint();
  }

  async function ask() {
    if (!mentor || streaming || mode !== "mentor") return;
    const token = ++run;
    streaming = true;
    pieces = [];
    trouble = null;
    let held = "";
    let failed = false;
    // Help is counted once the mentor actually says something; a failed ask gave none.
    let counted = false;
    const onEvent = (e: AgUiEvent) => {
      if (token !== run || failed) return;
      switch (e.type) {
        case "TEXT_MESSAGE_CONTENT":
          if (!counted) {
            counted = true;
            onHelp();
          }
          if (calm()) held += e.delta;
          else pieces = [...pieces, e.delta];
          break;
        case "TEXT_MESSAGE_END":
        case "RUN_FINISHED":
          if (held) {
            pieces = [held];
            held = "";
          }
          if (e.type === "RUN_FINISHED") streaming = false;
          break;
        case "RUN_ERROR":
          failed = true;
          fallBack(e.message || MENTOR_FALLBACK);
          break;
      }
    };
    try {
      await mentor.mentorHint(trim(request()), onEvent);
    } catch {
      if (token === run && !failed) {
        failed = true;
        fallBack(MENTOR_FALLBACK);
      }
    }
    if (token !== run || failed) return;
    if (held) pieces = [held];
    streaming = false;
  }
</script>

<section class="help" aria-label="Help">
  {#if trouble}
    <p class="mentor-trouble" role="status"><span aria-hidden="true">✗</span> {trouble}</p>
  {/if}

  {#if active}
    <div class="help-actions">
      {#if mode === "mentor"}
        <InkButton onclick={ask} disabled={streaming} aria-busy={streaming}>
          <Icon name="pen" size={22} />
          {streaming ? "the mentor is writing…" : "ask the mentor"}
        </InkButton>
      {:else if hints.length}
        <InkButton onclick={showHint} disabled={shown >= hints.length}>
          <Icon name="bulb" size={24} />
          {shown >= hints.length ? "no more hints" : `hint (${hints.length - shown} left)`}
        </InkButton>
      {/if}
      {#if mode === "mentor"}
        <p class="line">Sends only this exercise and your code.</p>
      {:else if mentor?.kind === "web"}
        <p class="line">AI mentor is available in the desktop app.</p>
      {:else if mentor?.kind === "desktop" && !available}
        <p class="line">Want a nudge from an AI mentor? Set one up in <a href="#/settings">Settings</a>.</p>
      {/if}
    </div>
  {/if}

  {#if pieces.length || streaming}
    <figure class="note">
      <figcaption class="from"><Icon name="pen" size={18} /> the mentor</figcaption>
      <p class="note-text" data-testid="mentor-note" aria-live="polite" aria-busy={streaming}>
        {#each pieces as p}<span class="ink">{p}</span>{/each}{#if streaming}<span class="nib" aria-hidden="true"></span>{/if}
      </p>
    </figure>
  {/if}

  {#if shown}
    <ol class="hints" aria-label="Hints">
      {#each hints.slice(0, shown) as h, i}
        <li style:--r="{i % 2 ? 0.8 : -0.8}deg"><span class="hint-n" aria-hidden="true">{i + 1}</span>{h}</li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .help {
    display: grid;
    gap: var(--space-3);
  }
  .help:empty {
    display: none;
  }
  .help-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-4);
  }
  .line {
    margin: 0;
    font-size: var(--size-text-sm);
    color: var(--ink-soft);
  }
  .mentor-trouble {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    line-height: 1.2;
  }
  .mentor-trouble span {
    color: var(--accent-deep);
  }
  /* A handwritten note on lined paper, pinned under the editor. */
  .note {
    margin: 0;
    padding: var(--space-3) var(--space-4) var(--space-3) var(--space-5);
    background:
      repeating-linear-gradient(
        to bottom,
        transparent 0,
        transparent calc(1.75rem - 1px),
        color-mix(in srgb, var(--focus) 22%, transparent) calc(1.75rem - 1px),
        color-mix(in srgb, var(--focus) 22%, transparent) 1.75rem
      ),
      var(--card);
    border-left: 3px double color-mix(in srgb, var(--accent) 55%, transparent);
    box-shadow: 3px 4px 0 color-mix(in srgb, var(--ink) 14%, transparent);
    transform: rotate(-0.6deg);
  }
  .from {
    display: inline-flex;
    gap: var(--space-1);
    align-items: center;
    font-family: var(--font-display);
    font-size: 1.1rem;
    color: var(--ink-soft);
  }
  .note-text {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    line-height: 1.75rem;
    white-space: pre-wrap;
  }
  /* Each new piece of the reply is inked in, left to right. */
  .ink {
    animation: ink-in var(--motion-slow) var(--motion-ease) both;
  }
  @keyframes ink-in {
    from {
      opacity: 0;
      filter: blur(1.5px);
    }
  }
  .nib {
    display: inline-block;
    width: 0.5em;
    height: 0.5em;
    margin-left: 0.15em;
    border-radius: 50%;
    background: var(--ink);
    animation: nib 0.9s ease-in-out infinite alternate;
  }
  @keyframes nib {
    to {
      transform: translate(0.25em, -0.15em);
      opacity: 0.4;
    }
  }
  .hints {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-2);
  }
  .hints li {
    display: flex;
    gap: var(--space-3);
    align-items: baseline;
    padding: var(--space-2) var(--space-4);
    background: color-mix(in srgb, var(--highlight) 38%, var(--card));
    border-left: 3px solid var(--highlight);
    transform: rotate(var(--r));
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    line-height: 1.2;
    box-shadow: 2px 3px 0 color-mix(in srgb, var(--ink) 12%, transparent);
  }
  .hint-n {
    font-weight: 700;
    color: var(--accent-deep);
  }
</style>
