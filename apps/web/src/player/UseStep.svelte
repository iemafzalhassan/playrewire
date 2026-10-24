<!-- Use: walk the street to the mission's terminal, write the code, and watch the city
     do what it says. The editor opens on the template for this encounter (help fades:
     fill one blank, then half a function, then a blank page with hints). A run sends
     city.go + setup.go + main.go; the @city events play in the street, then the verdict
     comes: a ✓, or the light or lamp that went wrong ringed in ink with a caption. -->
<script lang="ts">
  import { onMount, tick, untrack } from "svelte";
  import { BackendError, ERROR_COPY } from "../backend/errors";
  import type { CityEvent, Mission } from "../content/types";
  import {
    checkMission,
    explainGoErrors,
    helpLevel,
    parseCityOutput,
    type FriendlyError,
    type ProgressState,
  } from "../core";
  import type { RawRunResult, SourceFile } from "../lab/protocol";
  import Card from "../lib/Card.svelte";
  import Icon from "../lib/Icon.svelte";
  import InkButton from "../lib/InkButton.svelte";
  import { SketchWorld } from "../world/sketch.svelte";
  import StreetSketch from "../world/StreetSketch.svelte";
  import type { Labels, WorldView } from "../world/world";
  import CodeEditor from "./CodeEditor.svelte";
  import ErrorCard from "./ErrorCard.svelte";
  import MentorPanel from "./MentorPanel.svelte";
  import type { Backend, HintRequest } from "../backend/types";
  import { hasBlank, missionCaption, missionFiles, missionKind, templateFor, type Caption } from "./mission";

  type Opener = (host: HTMLElement) => Promise<WorldView | { view: WorldView; release: () => void } | null>;

  let {
    mission,
    cityStub,
    progress,
    hints,
    run,
    onDone,
    onContinue = () => {},
    nextLabel = "finish",
    openWorld,
    mentor = null,
    onMiss = () => {},
  }: {
    mission: Mission;
    cityStub: string;
    progress: ProgressState;
    /** The chunk's hints, offered on the blank-page level. */
    hints: string[];
    run: (files: SourceFile[]) => Promise<RawRunResult>;
    /** Called once, on the first passing run: the step is recorded right away. */
    onDone: (r: { firstTry: boolean; hintsUsed: number }) => void;
    onContinue?: () => void;
    nextLabel?: string;
    /** Mounts the 3D street in the host; null means draw the sketch street. */
    openWorld?: Opener;
    /** The AI mentor (desktop) or null for built-in hints only. */
    mentor?: Pick<Backend, "kind" | "mentorAvailable" | "mentorHint"> | null;
    /** The first failed run of this attempt (a Go error, wrong events or no events;
     *  never a lab that failed to start). Once per attempt. */
    onMiss?: (r: { hintsUsed: number }) => void;
  } = $props();

  let missed = false;
  function miss() {
    if (missed) return;
    missed = true;
    onMiss({ hintsUsed: helpUsed });
  }

  const level = untrack(() => helpLevel(progress, mission.id));
  const lv = untrack(() => templateFor(mission, level));
  const kind = untrack(() => missionKind(mission));
  const place = kind === "light" ? "Dock St" : "the Binary Bridge";

  type Result =
    | { kind: "blank" }
    | { kind: "error"; errors: FriendlyError[] }
    | { kind: "watching" }
    | { kind: "wrong"; caption: Caption }
    | { kind: "pass" };

  let host: HTMLDivElement;
  let world = $state<WorldView | null>(null);
  let sketch = $state<SketchWorld | null>(null);
  let phase = $state<"loading" | "street" | "editor">("loading");
  let near = $state(false);
  let code = $state(untrack(() => lv.template));
  let result = $state<Result | null>(null);
  let stdout = $state("");
  let says = $state<string[]>([]);
  let running = $state(false);
  let slow = $state(false);
  let passed = $state(false);
  /** Built-in hints shown plus mentor replies: both count toward hints_used. */
  let helpUsed = $state(0);
  let attempt = 0;
  let lastError: string | null = null;
  let labels = $state<Labels | null>(null);
  let firstRun: boolean | null = null;
  let editor = $state<{ focus: () => void } | null>(null);
  let resultEl = $state<HTMLDivElement>();

  const errorLine = $derived.by(() => {
    const line = result?.kind === "error" ? (result.errors.find((e) => e.line !== null)?.line ?? null) : null;
    return line === null ? null : { line };
  });

  const reduced = () => document.documentElement.dataset.motion === "reduced";

  const levelNote = {
    fill_blank: "One piece is missing. Replace the ___ with your own code.",
    half_function: "Half of it is here. Finish the rest.",
    blank_with_hints: "A blank page this time. You've done this before. Hints are here if you need them.",
  }[level];

  let alive = true;
  let release = () => {};
  /** Subscriptions on the current street; dropped when it changes. */
  let offs: (() => void)[] = [];

  function attach(view: WorldView) {
    offs.splice(0).forEach((f) => f());
    world = view;
    offs.push(
      view.onTerminal((m) => m === mission.id && openEditor()),
      view.onNear((m) => (near = m === mission.id)),
    );
    if (view.onFailed) offs.push(view.onFailed(() => fallBack()));
  }

  /** The 3D street died (a trapped wasm module): carry on in the sketch street. */
  function fallBack(e?: unknown) {
    if (!alive || !world || world.kind === "sketch") return;
    if (e) console.warn("the 3D street stopped; drawing the sketch street", e);
    const s = new SketchWorld();
    sketch = s;
    release();
    release = () => {};
    labels = null;
    near = false;
    attach(s);
    s.setReducedMotion(reduced());
    s.loadMission(mission.id);
    if (playing) startPlay(s, playing);
  }

  /** Calls the street; if the 3D street throws, swaps to the sketch and calls it there. */
  function onStreet(f: (v: WorldView) => void) {
    if (!world) return;
    try {
      f(world);
    } catch (e) {
      fallBack(e);
      if (world) f(world);
    }
  }

  onMount(() => {
    (async () => {
      let opened: Awaited<ReturnType<Opener>> = null;
      try {
        opened = openWorld ? await openWorld(host) : await (await import("../world/world")).openWorld(host);
      } catch (e) {
        console.warn(e);
      }
      if (!alive) {
        if (opened && "release" in opened) opened.release();
        return;
      }
      let view: WorldView;
      if (!opened) {
        sketch = new SketchWorld();
        view = sketch;
      } else if ("release" in opened) {
        view = opened.view;
        release = opened.release;
      } else {
        view = opened;
      }
      attach(view);
      onStreet((v) => {
        v.setReducedMotion(reduced());
        v.setInputEnabled(true);
        v.highlight(null);
        v.loadMission(mission.id);
      });
      phase = "street";
      if (world?.kind === "bevy" && typeof requestAnimationFrame === "function") {
        const bevy = world;
        const loop = () => {
          if (!alive || world !== bevy) return;
          labels = bevy.labels();
          requestAnimationFrame(loop);
        };
        requestAnimationFrame(loop);
      }
    })();
    return () => {
      alive = false;
      offs.splice(0).forEach((f) => f());
      try {
        world?.setInputEnabled(true);
        world?.highlight(null);
      } catch {
        // A dead street needs no tidying.
      }
      sketch?.destroy();
      release();
    };
  });

  async function openEditor() {
    if (phase === "editor") return;
    phase = "editor";
    onStreet((v) => v.setInputEnabled(false));
    await tick();
    editor?.focus();
  }

  async function backToStreet() {
    phase = "street";
    onStreet((v) => v.setInputEnabled(true));
    await tick();
    host?.querySelector("canvas")?.focus();
  }

  function goToTerminal() {
    onStreet((v) => v.setReducedMotion(reduced()));
    if (near) {
      openEditor();
    } else {
      onStreet((v) => v.goToTerminal());
    }
  }

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

  function thrownTrouble(e: unknown): FriendlyError {
    if (e instanceof BackendError) {
      const copy = ERROR_COPY[e.code];
      return { line: null, col: null, title: copy.title, plain: copy.text, raw: "" };
    }
    return labTrouble(`could not start the code runner: ${String(e)}\n`)!;
  }

  type Playing = { json: string; finish: () => void; off: () => void };
  /** Events being played right now; replayed on the sketch if the 3D street dies. */
  let playing: Playing | null = null;

  function startPlay(view: WorldView, p: Playing) {
    p.off();
    p.off = view.onEventsDone(p.finish);
    try {
      view.applyEvents(p.json);
    } catch (e) {
      // fallBack replays `playing` on the sketch street.
      if (view.kind === "sketch") throw e;
      fallBack(e);
    }
  }

  /** Plays the events and resolves when the street is done (or after a safety timeout). */
  function playEvents(json: string): Promise<void> {
    return new Promise((resolve) => {
      const p: Playing = { json, finish, off: () => {} };
      const t = setTimeout(finish, 15_000);
      function finish() {
        clearTimeout(t);
        p.off();
        if (playing === p) playing = null;
        resolve();
      }
      playing = p;
      if (world) startPlay(world, p);
      else finish();
    });
  }

  async function show() {
    await tick();
    resultEl?.scrollIntoView?.({ block: "nearest" });
  }

  async function runCode() {
    if (running || passed || !world) return;
    if (level === "fill_blank" && hasBlank(code)) {
      result = { kind: "blank" };
      return show();
    }
    running = true;
    try {
      await runOnce();
    } finally {
      running = false;
      slow = false;
    }
    return show();
  }

  async function runOnce() {
    result = null;
    onStreet((v) => v.highlight(null));
    const slowTimer = setTimeout(() => (slow = true), 900);
    const { files, lineOffset } = missionFiles(cityStub, mission.setup, code, lv.wholeFile);
    const hintsAtRun = helpUsed;
    attempt++;
    let r: RawRunResult | null = null;
    let thrown: unknown = null;
    try {
      r = await run(files);
    } catch (e) {
      thrown = e ?? new Error("run failed");
    }
    clearTimeout(slowTimer);
    slow = false;

    if (thrown || !r) {
      result = { kind: "error", errors: [thrownTrouble(thrown)] };
      return;
    }
    const trouble = labTrouble(r.stderr);
    const errors = trouble
      ? [trouble]
      : r.timedOut || r.truncated || r.stderr.trim()
        ? explainGoErrors(r.stderr, lineOffset, r.timedOut, r.truncated)
        : [];
    const parsed = parseCityOutput(r.stdout);
    stdout = parsed.stdout.replace(/\n+$/, "");
    says = parsed.events.flatMap((e: CityEvent) => (e.op === "say" ? [e.text] : []));
    if (errors.length) {
      result = { kind: "error", errors };
      if (!trouble && firstRun === null) firstRun = false;
      if (!trouble) {
        lastError = errors.map((e) => e.raw.trim() || `${e.title}. ${e.plain}`).join("\n");
        miss();
      }
      return;
    }

    // Let the city play first; the verdict comes once the street has finished.
    onStreet((v) => v.setReducedMotion(reduced()));
    const verdict = checkMission(mission.expectEvents, parsed.events);
    result = { kind: "watching" };
    await playEvents(JSON.stringify(parsed.events));

    if (firstRun === null) firstRun = verdict.kind === "pass" && hintsAtRun === 0;
    if (verdict.kind === "pass") {
      passed = true;
      result = { kind: "pass" };
      onDone({ firstTry: firstRun, hintsUsed: helpUsed });
    } else {
      const caption = missionCaption(mission, verdict, parsed.events)!;
      onStreet((v) => v.highlight(caption.target));
      result = { kind: "wrong", caption };
      lastError = `It ran, but the street went wrong: ${caption.text}`;
      miss();
    }
  }

  // Built-in hints only on the blank-page level (help fades); the mentor is there at every level.
  const ladder = level === "blank_with_hints" ? untrack(() => hints) : [];

  function hintRequest(): HintRequest {
    return {
      concept: mission.concept,
      exercise_prompt: `${mission.title}. ${levelNote}`,
      code,
      last_error: lastError,
      attempt,
    };
  }

  const tags = $derived.by(() => {
    if (!labels) return [];
    const pts = kind === "light" ? labels.lights : labels.lamps;
    const canvas = host?.querySelector("canvas");
    const sx = canvas && labels.size[0] > 0 ? canvas.clientWidth / labels.size[0] : 1;
    const sy = canvas && labels.size[1] > 0 ? canvas.clientHeight / labels.size[1] : 1;
    return pts.flatMap((p, i) => (p ? [{ i, x: p[0] * sx, y: p[1] * sy }] : []));
  });
  const passWords =
    kind === "light" ? "It works! Dock St turns green, one light after another." : "It works! The bridge reads just right.";
</script>

<section class="use-step" class:editing={phase === "editor"} aria-label="Use it in the street">
  <div class="world-wrap">
    <div class="world" bind:this={host}>
      {#if sketch}
        <StreetSketch world={sketch} {kind} />
      {/if}
      {#each tags as t (t.i)}
        <span class="tag" style:left="{t.x}px" style:top="{t.y}px" aria-hidden="true">{t.i}</span>
      {/each}
    </div>

    {#if phase === "loading"}
      <p class="hush" aria-live="polite">inking the street…</p>
    {:else if phase === "street"}
      <Card tape tilt={-0.6} edge="c" class="hud">
        <p class="hud-place">{place}</p>
        <p class="hud-title">{mission.title}</p>
        {#if sketch}
          <p class="hud-text">Open the terminal to write your code. The street will show what it does.</p>
        {:else if near}
          <p class="hud-text" aria-live="polite"><kbd>E</kbd> uses the terminal.</p>
        {:else}
          <p class="hud-text">
            Walk to the terminal under the red arrow: <kbd>←</kbd><kbd>↑</kbd><kbd>→</kbd><kbd>↓</kbd> or <kbd>W</kbd><kbd>A</kbd><kbd>S</kbd><kbd>D</kbd>,
            or click the street. Then press <kbd>E</kbd>.
          </p>
        {/if}
        <InkButton primary onclick={goToTerminal}>
          {near ? "use the terminal" : "go to the terminal"}
          <Icon name="arrow" size={26} />
        </InkButton>
      </Card>
    {/if}
  </div>

  {#if phase === "editor"}
    <div class="panel">
      <header class="panel-head">
        <p class="at">at the terminal · {place}</p>
        <h3 class="mission-title" class:big={level === "blank_with_hints"}>{mission.title}</h3>
        <p class="level-note">{levelNote}</p>
      </header>

      <Card edge="b" class="editor-card">
        <CodeEditor
          bind:this={editor}
          doc={code}
          label="Your Go code for the street"
          {errorLine}
          onChange={(v) => (code = v)}
          onRun={runCode}
        />
      </Card>

      <div class="actions">
        {#if passed}
          <InkButton primary onclick={onContinue}>{nextLabel} <Icon name="arrow" size={28} /></InkButton>
        {:else}
          <InkButton primary onclick={runCode} disabled={running} aria-busy={running}>
            {running ? "running…" : "run"}
            <Icon name="play" size={24} />
          </InkButton>
          <button type="button" class="leave" onclick={backToStreet}>back to the street</button>
        {/if}
      </div>
      {#if !passed}
        <p class="kbd-tip"><kbd>Ctrl</kbd> + <kbd>Enter</kbd> runs your code too</p>
      {/if}
      {#if slow}
        <p class="slow" role="status">Starting Go… the first run takes a few seconds.</p>
      {/if}

      <MentorPanel {mentor} hints={ladder} request={hintRequest} onHelp={() => helpUsed++} active={!passed} />

      <div class="result" data-testid="use-result" aria-live="polite" bind:this={resultEl}>
        {#if result?.kind === "blank"}
          <p class="nudge"><span class="glyph" aria-hidden="true">✗</span> Fill in the blank first: swap the ___ for your code, then run.</p>
        {:else if result?.kind === "error"}
          <p class="verdict small"><span class="glyph" aria-hidden="true">✗</span> <span>Not yet. Go stopped here:</span></p>
          <ErrorCard errors={result.errors} />
        {:else if result?.kind === "watching"}
          <p class="watching">Watch the street…</p>
        {:else if result?.kind === "wrong"}
          <Card edge="c" tilt={-0.4} class="wrong-card">
            <p class="verdict" role="status">
              <span class="glyph" aria-hidden="true">✗</span>
              <span>{result.caption.text}</span>
            </p>
            {#if result.caption.target}
              <p class="ring-note">It's circled in red on the street. Change your code and run again.</p>
            {:else}
              <p class="ring-note">Change your code and run again.</p>
            {/if}
          </Card>
        {:else if result?.kind === "pass"}
          <Card edge="a" tilt={0.5} class="pass-card">
            <p class="verdict"><span class="glyph ok" aria-hidden="true">✓</span> <span>{passWords}</span></p>
          </Card>
        {/if}
        {#if says.length && result && result.kind !== "error"}
          <ul class="says" aria-label="The city says">
            {#each says as s}<li>“{s}”</li>{/each}
          </ul>
        {/if}
        {#if stdout && result && result.kind !== "blank"}
          <div class="screen" aria-label="What your program printed">
            <p class="col-title">your program printed</p>
            <pre data-testid="use-stdout">{stdout}</pre>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</section>

<style>
  .use-step {
    /* Break out of the page column: the street wants room. */
    --bleed: min(50vw - var(--space-4), 40rem);
    width: calc(2 * var(--bleed));
    margin-inline: calc(50% - var(--bleed));
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-4);
  }
  .world-wrap {
    position: relative;
  }
  .world {
    position: relative;
    height: clamp(22rem, 62vh, 40rem);
    overflow: hidden;
    border: var(--ink-line-bold) solid var(--ink);
    border-radius: var(--radius-wobble-b);
    background: var(--paper);
    box-shadow: var(--ink-shadow-x) var(--ink-shadow-y) 0 var(--ink);
  }
  .world :global(canvas) {
    width: 100%;
    height: 100%;
  }
  .world :global(canvas:focus-visible) {
    outline: 3px solid var(--focus);
    outline-offset: -5px;
  }
  .tag {
    position: absolute;
    transform: translate(-50%, -100%);
    min-width: 1.7rem;
    padding: 0 0.35rem;
    border: 2px solid var(--ink);
    border-radius: 48% 52% 45% 55% / 55% 45% 55% 45%;
    background: var(--card);
    font-family: var(--font-display);
    font-weight: 700;
    font-size: var(--size-hand-sm);
    line-height: 1.3;
    text-align: center;
    pointer-events: none;
  }
  .hush {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand);
    color: var(--ink-soft);
  }
  .world-wrap :global(.hud) {
    position: absolute;
    left: var(--space-4);
    top: var(--space-4);
    max-width: min(24rem, calc(100% - 2 * var(--space-4)));
    padding: var(--space-4) var(--space-5);
    display: grid;
    gap: var(--space-2);
    justify-items: start;
  }
  .hud-place,
  .at {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: var(--ink-soft);
  }
  .hud-title {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand);
    font-weight: 700;
    line-height: 1.05;
  }
  .hud-text {
    margin: 0;
    line-height: 1.6;
  }
  kbd {
    display: inline-block;
    min-width: 1.4em;
    margin: 0 0.1em;
    padding: 0 0.3em;
    border: 1.5px solid var(--ink-soft);
    border-bottom-width: 3px;
    border-radius: 5px;
    background: var(--card);
    font-size: 0.85em;
    text-align: center;
    line-height: 1.4;
  }
  .panel {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-4);
    align-content: start;
  }
  /* Wide screens: the street on the left, the terminal on the right, both in view. */
  @media (min-width: 62rem) {
    .use-step.editing {
      grid-template-columns: minmax(0, 1fr) minmax(22rem, 30rem);
      align-items: start;
    }
    .use-step.editing .world-wrap {
      position: sticky;
      top: var(--space-4);
    }
    .use-step.editing .world {
      height: clamp(24rem, 78vh, 44rem);
    }
  }
  @media (max-width: 61.99rem) {
    .use-step.editing .world {
      height: clamp(16rem, 42vh, 26rem);
    }
  }
  .panel-head {
    display: grid;
    gap: var(--space-1);
  }
  .mission-title {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand);
    line-height: 1.05;
  }
  .mission-title.big {
    font-size: var(--size-hand-lg);
  }
  .level-note {
    margin: 0;
  }
  .use-step :global(.editor-card) {
    padding: var(--space-1) var(--space-2) var(--space-1) 0;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3) var(--space-4);
  }
  .leave {
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
  .leave:hover {
    color: var(--ink);
  }
  .kbd-tip,
  .slow {
    margin: calc(-1 * var(--space-2)) 0 0;
    font-size: var(--size-text-sm);
    color: var(--ink-soft);
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
    font-size: var(--size-hand);
    font-weight: 700;
    line-height: 1.1;
  }
  .verdict.small {
    font-size: var(--size-hand-sm);
  }
  .glyph {
    color: var(--accent-deep);
  }
  .glyph.ok {
    color: var(--ok);
    font-size: 1.15em;
  }
  .nudge {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
  }
  .watching {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--size-hand);
    color: var(--ink-soft);
  }
  .result :global(.wrong-card),
  .result :global(.pass-card) {
    padding: var(--space-4) var(--space-5);
    display: grid;
    gap: var(--space-2);
  }
  .ring-note {
    margin: 0;
  }
  .says {
    margin: 0;
    padding-left: var(--space-4);
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
  }
  .screen {
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-wobble-c);
    background: var(--ink);
    color: var(--card);
  }
  .screen .col-title {
    margin: 0 0 var(--space-1);
    font-family: var(--font-display);
    font-size: var(--size-hand-sm);
    color: color-mix(in srgb, var(--card) 70%, transparent);
  }
  .screen pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
