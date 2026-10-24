<!-- Drill: solve variations from memory and run real Go. One drill at a time;
     three passes in a row finish the step, and a miss starts the run again.
     Errors always arrive as friendly cards with Go's own words folded away. -->
<script lang="ts">
  import { tick, untrack } from "svelte";
  import type { Drill } from "../content/types";
  import { explainGoErrors, parseCityOutput, type FriendlyError } from "../core";
  import type { RawRunResult, SourceFile } from "../lab/protocol";
  import Card from "../lib/Card.svelte";
  import Icon from "../lib/Icon.svelte";
  import InkButton from "../lib/InkButton.svelte";
  import { BackendError, ERROR_COPY } from "../backend/errors";
  import ToolchainCard from "../lab/ToolchainCard.svelte";
  import { getToolchainContext } from "../lab/toolchain.svelte";
  import CodeEditor from "./CodeEditor.svelte";
  import { checkDrill, clock, cpm, wrapMain } from "./drill";
  import ErrorCard from "./ErrorCard.svelte";
  import MentorPanel from "./MentorPanel.svelte";
  import type { Backend, HintRequest } from "../backend/types";

  let {
    drills,
    hints,
    run,
    onDone,
    onContinue = () => {},
    nextLabel = "finish",
    concept = "",
    mentor = null,
    needed = 3,
    onMiss = () => {},
    showWant = false,
    doneWords = "Three in a row. Drill done!",
  }: {
    drills: Drill[];
    hints: string[];
    run: (files: SourceFile[]) => Promise<RawRunResult>;
    /** Called once, on the third pass in a row: the step is recorded right away. */
    onDone: (r: { hintsUsed: number; firstTry: boolean }) => void;
    /** The learner has looked at the last result and wants to move on. */
    onContinue?: () => void;
    nextLabel?: string;
    /** What this drill practises; sent to the mentor with the exercise. */
    concept?: string;
    /** The AI mentor (desktop) or null for built-in hints only. */
    mentor?: Pick<Backend, "kind" | "mentorAvailable" | "mentorHint"> | null;
    /** Passes in a row that finish the step: 3 for a page's drill, 1 for a warm-up or fix-it. */
    needed?: number;
    /** The first failed run of the learner's code in a drill round (a Go error or the
     *  wrong output; never a lab that failed to start). Once per round. */
    onMiss?: (r: { hintsUsed: number }) => void;
    /** Show the output the program should print (the fix-it task). */
    showWant?: boolean;
    doneWords?: string;
  } = $props();

  const NEEDED = untrack(() => Math.max(1, needed));

  type Result =
    | { kind: "pass"; stdout: string; ms: number; cpm: number }
    | { kind: "error"; errors: FriendlyError[] }
    | { kind: "wrong"; got: string[]; want: string[]; diffLine: number }
    | { kind: "toolchain" };

  // Desktop only: the shared Go download (see lab/toolchain.svelte.ts).
  const toolchain = getToolchainContext();

  let idx = $state(0);
  let round = $state(1);
  let passes = $state(0);
  /** Built-in hints shown plus mentor replies: both count toward hints_used. */
  let helpUsed = $state(0);
  let attempt = 0;
  let lastError: string | null = null;
  let code = $state(untrack(() => drills[0]?.starter ?? ""));
  let result = $state<Result | null>(null);
  let running = $state(false);
  let slow = $state(false);
  let missesHere = $state(0);
  let finished = $state(false);
  let firstRun: boolean | null = null;
  let resultEl: HTMLDivElement;

  let startedAt = $state(Date.now());
  let now = $state(Date.now());
  let stoppedAt = $state<number | null>(null);

  const drill = $derived(drills[idx]);
  const passed = $derived(result?.kind === "pass");
  // A fresh object per result, so the same line is marked again after a re-run.
  const errorLine = $derived.by(() => {
    const line = result?.kind === "error" ? (result.errors.find((e) => e.line !== null)?.line ?? null) : null;
    return line === null ? null : { line };
  });
  const elapsed = $derived((stoppedAt ?? now) - startedAt);

  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(t);
  });

  /** The lab failing to start is our problem, not the learner's code. */
  function labTrouble(stderr: string): FriendlyError | null {
    if (!/^(could not start the code runner|the code runner crashed|bad result from the code runner)/.test(stderr)) return null;
    return {
      line: null,
      col: null,
      title: "The Go lab didn't start",
      plain: "This isn't your code. The part of Rewire that runs Go couldn't get going. Check your connection, then press run again.",
      raw: stderr,
    };
  }

  /** A desktop command failed before Go ran: say why in plain words. */
  function thrownTrouble(e: unknown): FriendlyError {
    if (e instanceof BackendError) {
      const copy = ERROR_COPY[e.code];
      return { line: null, col: null, title: copy.title, plain: copy.text, raw: "" };
    }
    return labTrouble(`could not start the code runner: ${String(e)}\n`)!;
  }

  const lines = (s: string) => s.replace(/\r\n?/g, "\n").replace(/\n+$/, "").split("\n");

  async function runCode() {
    if (running || passed || finished) return;
    running = true;
    // The run belongs to this drill round; if the learner moves on meanwhile, its answer is dropped.
    const token = round;
    const slowTimer = setTimeout(() => {
      if (token === round) slow = true;
    }, 900);
    const d = drill;
    const source = code;
    const hintsAtRun = helpUsed;
    attempt++;
    const { source: file, lineOffset } = wrapMain(source, d.wholeFile);
    let r: RawRunResult | null = null;
    let thrown: { e: unknown } | null = null;
    try {
      r = await run([{ name: "main.go", source: file }]);
    } catch (e) {
      thrown = { e };
    }
    clearTimeout(slowTimer);
    // A stale answer touches nothing, not even `running`: another run may be in flight.
    if (token !== round || finished) return;
    slow = false;
    running = false;

    if (thrown || !r) {
      // Not the learner's code: no verdict, no reset of their run of passes.
      if (thrown?.e instanceof BackendError && thrown.e.code === "toolchain_missing" && toolchain) {
        toolchain.markMissing();
        result = { kind: "toolchain" };
      } else {
        result = { kind: "error", errors: [thrownTrouble(thrown?.e)] };
      }
      await tick();
      resultEl?.scrollIntoView?.({ block: "nearest" });
      return;
    }

    const trouble = labTrouble(r.stderr);
    const errors = trouble
      ? [trouble]
      : r.timedOut || r.truncated || r.stderr.trim()
        ? explainGoErrors(r.stderr, lineOffset, r.timedOut, r.truncated)
        : [];

    let ok = false;
    if (errors.length) {
      result = { kind: "error", errors };
    } else {
      const { stdout } = parseCityOutput(r.stdout);
      const check = checkDrill(d.expectStdout, stdout);
      if (check.pass) {
        ok = true;
        const end = Date.now();
        stoppedAt = end;
        result = { kind: "pass", stdout, ms: end - startedAt, cpm: cpm(source, d.starter ?? "", end - startedAt) };
      } else {
        result = { kind: "wrong", got: lines(stdout), want: lines(d.expectStdout), diffLine: check.diffLine ?? 1 };
      }
    }

    // A lab that failed to start says nothing about the learner: no first-try
    // verdict, no reset of their run of passes.
    if (trouble) {
      await tick();
      resultEl?.scrollIntoView?.({ block: "nearest" });
      return;
    }
    if (firstRun === null) firstRun = ok && hintsAtRun === 0;
    lastError = describeMiss(result);
    if (ok) {
      passes++;
    } else {
      passes = 0;
      missesHere++;
      if (missesHere === 1) onMiss({ hintsUsed: helpUsed });
    }
    if (passes >= NEEDED) {
      finished = true;
      onDone({ hintsUsed: helpUsed, firstTry: firstRun });
    }
    // Bring the verdict into view on short screens.
    await tick();
    resultEl?.scrollIntoView?.({ block: "nearest" });
  }

  function nextDrill() {
    // Any run still in flight belongs to the old round and will be dropped.
    running = false;
    slow = false;
    idx = (idx + 1) % drills.length;
    code = drills[idx].starter ?? "";
    round++;
    result = null;
    missesHere = 0;
    attempt = 0;
    lastError = null;
    startedAt = Date.now();
    now = startedAt;
    stoppedAt = null;
  }

  /** What went wrong last time, in words the mentor can use (never the learner's name or ids). */
  function describeMiss(r: Result | null): string | null {
    if (r?.kind === "error") return r.errors.map((e) => e.raw.trim() || `${e.title}. ${e.plain}`).join("\n");
    if (r?.kind === "wrong") return `It ran but printed:\n${r.got.join("\n")}\nIt should print:\n${r.want.join("\n")}`;
    return null;
  }

  function hintRequest(): HintRequest {
    const want = showWant ? `\nIt should print:\n${drill.expectStdout.replace(/\n+$/, "")}` : "";
    return { concept, exercise_prompt: drill.prompt + want, code, last_error: lastError, attempt };
  }
</script>

<section class="drill-step" aria-label="Drill">
  <header class="bar">
    {#if NEEDED > 1}<p class="round">drill {round}</p>{/if}
    {#if NEEDED > 1}<p class="passes" data-testid="passes">
      <span class="marks" aria-hidden="true">
        {#each Array(NEEDED) as _, i}<span class="mark" class:got={i < passes}>{i < passes ? "✓" : "○"}</span>{/each}
      </span>
      <span>{passes} of {NEEDED} in a row</span>
    </p>{/if}
    <p class="timer" role="timer" aria-label="time this round">{clock(elapsed)}</p>
  </header>

  <Card tape tilt={-0.6} edge="c" class="prompt">
    <p class="prompt-text">{drill.prompt}</p>
    {#if showWant}
      <p class="want-title">It should print:</p>
      <pre class="want" data-testid="fix-want">{drill.expectStdout.replace(/\n+$/, "")}</pre>
    {/if}
  </Card>

  <Card edge="b" class="editor-card">
    {#key round}
      <CodeEditor
        doc={drill.starter ?? ""}
        label="Your Go code for this drill"
        {errorLine}
        onChange={(v) => (code = v)}
        onRun={runCode}
      />
    {/key}
  </Card>

  <div class="actions">
    {#if finished}
      <InkButton primary onclick={onContinue}>{nextLabel} <Icon name="arrow" size={28} /></InkButton>
    {:else if passed}
      <InkButton primary onclick={nextDrill}>next drill <Icon name="arrow" size={28} /></InkButton>
    {:else}
      <InkButton primary onclick={runCode} disabled={running} aria-busy={running}>
        {running ? "running…" : "run"}
        <Icon name="play" size={24} />
      </InkButton>
      {#if missesHere >= 2 && drills.length > 1}
        <button type="button" class="other" onclick={nextDrill} disabled={running}>try a different one</button>
      {/if}
    {/if}
  </div>
  {#if !passed && !finished}
    <p class="kbd-tip"><kbd>Ctrl</kbd> + <kbd>Enter</kbd> runs your code too</p>
  {/if}
  {#if slow}
    <p class="slow" role="status">Starting Go… the first run takes a few seconds.</p>
  {/if}

  {#key round}
    <MentorPanel {mentor} {hints} request={hintRequest} onHelp={() => helpUsed++} active={!passed && !finished} />
  {/key}

  <div class="result" data-testid="drill-result" aria-live="polite" bind:this={resultEl}>
    {#if result?.kind === "pass"}
      <Card edge="a" tilt={0.5} class="pass-card">
        <p class="verdict">
          <span class="glyph ok" aria-hidden="true">✓</span>
          <span>{finished ? doneWords : "It works!"}</span>
        </p>
        <div class="screen" aria-label="What your program printed">
          <pre>{result.stdout.replace(/\n+$/, "")}</pre>
        </div>
        <p class="speed">
          <span>{clock(result.ms)}</span>
          {#if result.cpm > 0}<span aria-hidden="true">·</span><span>{result.cpm} characters a minute</span>{/if}
        </p>
      </Card>
    {:else if result?.kind === "error"}
      <p class="verdict small"><span class="glyph" aria-hidden="true">✗</span> <span>Not yet. Go stopped here:</span></p>
      <ErrorCard errors={result.errors} />
    {:else if result?.kind === "toolchain" && toolchain}
      <ToolchainCard {toolchain} tilt={0.4} />
    {:else if result?.kind === "wrong"}
      <Card edge="c" tilt={-0.4} class="wrong-card">
        <p class="verdict">
          <span class="glyph" aria-hidden="true">✗</span>
          <span>Close. It ran, but printed something else.</span>
        </p>
        <p class="diff-note">Look at line {result.diffLine}.</p>
        <div class="compare">
          <div>
            <p class="col-title">you printed</p>
            <pre>{#each result.got as l, i}<span class:diff={i + 1 === result.diffLine}>{l || " "}</span>
{/each}</pre>
          </div>
          <div>
            <p class="col-title">the drill wants</p>
            <pre>{#each result.want as l, i}<span class:diff={i + 1 === result.diffLine}>{l || " "}</span>
{/each}</pre>
          </div>
        </div>
      </Card>
    {/if}
  </div>
</section>

<style>
  .drill-step {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-4);
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-2) var(--space-4);
  }
  .bar p {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: var(--ink-soft);
  }
  .round {
    font-weight: 700;
    color: var(--ink) !important;
  }
  .passes {
    display: inline-flex;
    gap: var(--space-2);
    align-items: baseline;
  }
  .marks {
    display: inline-flex;
    gap: 0.2rem;
    font-size: 1.35em;
    line-height: 1;
  }
  .mark {
    color: var(--pencil);
  }
  .mark.got {
    color: var(--ok);
    font-weight: 700;
  }
  .timer {
    font-variant-numeric: tabular-nums;
    min-width: 3.5ch;
    text-align: right;
  }
  .drill-step :global(.prompt) {
    padding: var(--space-4) var(--space-5);
  }
  .prompt-text {
    margin: 0;
    font-size: var(--size-text-lg);
    font-weight: 700;
  }
  .drill-step :global(.editor-card) {
    padding: var(--space-1) var(--space-2) var(--space-1) 0;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3) var(--space-4);
  }
  .other {
    min-height: 2.75rem;
    border: 0;
    background: none;
    color: var(--ink-soft);
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    text-decoration: underline;
    text-decoration-style: wavy;
    text-underline-offset: 4px;
    cursor: pointer;
  }
  .other:hover:not(:disabled) {
    color: var(--ink);
  }
  .other:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }
  .kbd-tip,
  .slow {
    margin: calc(-1 * var(--space-2)) 0 0;
    font-size: var(--size-text-sm);
    color: var(--ink-soft);
  }
  kbd {
    padding: 0 0.35em;
    border: 1.5px solid var(--ink-soft);
    border-bottom-width: 3px;
    border-radius: 5px;
    background: var(--card);
    font-size: 0.85em;
  }
  .want-title {
    margin: var(--space-3) 0 var(--space-1);
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: var(--ink-soft);
  }
  .want {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-wobble-c);
    background: var(--ink);
    color: var(--card);
    white-space: pre-wrap;
    font-size: 1rem;
  }
  .result:empty {
    display: none;
  }
  .result {
    display: grid;
    gap: var(--space-3);
  }
  .verdict {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand-lg);
    font-weight: 700;
    line-height: 1.05;
  }
  .verdict.small {
    font-size: var(--size-hand);
  }
  .glyph {
    color: var(--accent-deep);
  }
  .glyph.ok {
    color: var(--ok);
    font-size: 1.15em;
  }
  .result :global(.pass-card),
  .result :global(.wrong-card) {
    padding: var(--space-4) var(--space-5);
    display: grid;
    gap: var(--space-3);
  }
  .screen {
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-wobble-c);
    background: var(--ink);
    color: var(--card);
  }
  .screen pre,
  .compare pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 1rem;
  }
  .speed {
    display: flex;
    gap: var(--space-2);
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: var(--ink-soft);
  }
  .diff-note {
    margin: 0;
  }
  .compare {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-4);
  }
  .col-title {
    margin: 0 0 var(--space-1);
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: var(--ink-soft);
  }
  .compare pre {
    padding: var(--space-2) var(--space-3);
    background: color-mix(in srgb, var(--paper-deep) 70%, transparent);
    border-radius: 4px;
  }
  .diff {
    background: color-mix(in srgb, var(--highlight) 70%, transparent);
    box-shadow: inset 0 -2px 0 var(--accent);
  }
  @media (max-width: 34rem) {
    .compare {
      grid-template-columns: 1fr;
    }
  }
</style>
