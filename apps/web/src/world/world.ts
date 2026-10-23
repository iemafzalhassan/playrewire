// The hand-inked street (Bevy, compiled to wasm) behind the Use step.
//
// The module-level functions are the brief's API (start, loadMission, applyEvents,
// setReducedMotion, onTerminal, onEventsDone) plus a few the Use step needs
// (setInputEnabled, goToTerminal, highlight, onNear, labels). `openWorld` mounts the
// one shared canvas into a host element and hands back a WorldView; it resolves to
// null when WebGL2 is missing or the world fails to start, and the Use step then
// draws the sketch street instead, so a learner without a GPU can still finish.
//
// Callbacks: the wasm side never holds JS functions. It queues signals, and a
// requestAnimationFrame loop here drains them (`pollSignals`) and calls the callbacks
// (see crates/rewire-world/src/mailbox.rs).
//
// CSP: the glue is loaded with a dynamic import() of a same-origin file and the wasm
// is compiled by WebAssembly.instantiateStreaming, so `script-src 'self'
// 'wasm-unsafe-eval'` is enough (no 'unsafe-eval').

export type MissionId = "green-wave" | "binary-bridge";
export type Target = { kind: "light"; i: number } | { kind: "lamp"; i: number };
export type Point = [number, number];
/** Where things are on the canvas, in CSS px from its top-left (null: off screen). */
export interface Labels {
  /** The street's own viewport size; scale points by the canvas CSS size over this. */
  size: Point;
  lights: (Point | null)[];
  lamps: (Point | null)[];
  terminal: Point | null;
  ring: Point | null;
}

export interface WorldView {
  kind: "bevy" | "sketch";
  loadMission(missionId: MissionId | string): void;
  applyEvents(eventsJson: string): void;
  setReducedMotion(on: boolean): void;
  setInputEnabled(on: boolean): void;
  goToTerminal(): void;
  highlight(target: Target | null): void;
  /** Each returns an unsubscribe function. */
  onTerminal(cb: (missionId: string) => void): () => void;
  onEventsDone(cb: () => void): () => void;
  onNear(cb: (missionId: string | null) => void): () => void;
  labels(): Labels | null;
}

interface WorldModule {
  default: (input?: unknown) => Promise<unknown>;
  start(canvasId: string): void;
  loadMission(id: string): void;
  applyEvents(json: string): void;
  setReducedMotion(on: boolean): void;
  setInputEnabled(on: boolean): void;
  goToTerminal(): void;
  highlight(json: string): void;
  pollSignals(): string;
  labels(): string;
}

type Signal =
  | { kind: "ready" }
  | { kind: "near"; mission: string | null }
  | { kind: "terminal"; mission: string }
  | { kind: "events_done" };

export const CANVAS_ID = "rewire-world";
const READY_TIMEOUT_MS = 20_000;

let mod: WorldModule | null = null;
let starting: Promise<void> | null = null;
let failed = false;
let ready = false;
const readyWaiters: (() => void)[] = [];
const terminalCbs = new Set<(m: string) => void>();
const doneCbs = new Set<() => void>();
const nearCbs = new Set<(m: string | null) => void>();

/** Can this browser draw the 3D street at all? */
export function webgl2Available(): boolean {
  try {
    const c = document.createElement("canvas");
    return !!c.getContext("webgl2");
  } catch {
    return false;
  }
}

function dispatch(s: Signal) {
  // A callback that throws must not stop the pump.
  const safe = (f: () => void) => {
    try {
      f();
    } catch (e) {
      console.error(e);
    }
  };
  if (s.kind === "ready") {
    ready = true;
    readyWaiters.splice(0).forEach((w) => w());
  } else if (s.kind === "terminal") terminalCbs.forEach((cb) => safe(() => cb(s.mission)));
  else if (s.kind === "events_done") doneCbs.forEach((cb) => safe(cb));
  else if (s.kind === "near") nearCbs.forEach((cb) => safe(() => cb(s.mission)));
}

function pump() {
  if (!mod || failed) return;
  try {
    for (const s of JSON.parse(mod.pollSignals()) as Signal[]) dispatch(s);
  } catch (e) {
    // A trapped wasm module (a Bevy panic) can't be used again.
    console.error("the street stopped", e);
    failed = true;
    return;
  }
  requestAnimationFrame(pump);
}

/** Loads the world wasm (once) and starts the street in `<canvas id=canvasId>`. */
export function start(canvasId: string): Promise<void> {
  if (failed) return Promise.reject(new Error("the street could not start"));
  starting ??= (async () => {
    const spec = `${import.meta.env.BASE_URL}world/rewire_world.js`;
    const m = (await import(/* @vite-ignore */ spec)) as WorldModule;
    await m.default();
    try {
      m.start(canvasId);
    } catch (e) {
      // winit may unwind out of start() to hand the loop to the browser.
      if (!String(e).includes("control flow")) throw e;
    }
    mod = m;
    requestAnimationFrame(pump);
    await new Promise<void>((resolve, reject) => {
      if (ready) return resolve();
      const t = setTimeout(() => reject(new Error("the street did not draw its first frame")), READY_TIMEOUT_MS);
      readyWaiters.push(() => (clearTimeout(t), resolve()));
    });
  })().catch((e) => {
    failed = true;
    throw e;
  });
  return starting;
}

export const loadMission = (id: string) => mod?.loadMission(id);
export const applyEvents = (eventsJson: string) => mod?.applyEvents(eventsJson);
export const setReducedMotion = (on: boolean) => mod?.setReducedMotion(on);
export const setInputEnabled = (on: boolean) => mod?.setInputEnabled(on);
export const goToTerminal = () => mod?.goToTerminal();
export const highlight = (t: Target | null) => mod?.highlight(JSON.stringify(t));
export function onTerminal(cb: (missionId: string) => void): () => void {
  terminalCbs.add(cb);
  return () => terminalCbs.delete(cb);
}
export function onEventsDone(cb: () => void): () => void {
  doneCbs.add(cb);
  return () => doneCbs.delete(cb);
}
export function onNear(cb: (missionId: string | null) => void): () => void {
  nearCbs.add(cb);
  return () => nearCbs.delete(cb);
}
export function labels(): Labels | null {
  try {
    const s = mod?.labels();
    return s ? (JSON.parse(s) as Labels) : null;
  } catch {
    return null;
  }
}

// One canvas for the whole page: winit owns it once started, so it moves between
// hosts instead of being recreated. Off screen it parks in a 1px holder.
let canvas: HTMLCanvasElement | null = null;
let parking: HTMLDivElement | null = null;

function sharedCanvas(): HTMLCanvasElement {
  if (canvas) return canvas;
  canvas = document.createElement("canvas");
  canvas.id = CANVAS_ID;
  canvas.tabIndex = 0;
  canvas.setAttribute("role", "application");
  canvas.setAttribute(
    "aria-label",
    "The street. Walk with the arrow keys or W A S D, or click where to go. Press E at the terminal.",
  );
  canvas.style.display = "block";
  canvas.style.outline = "none";
  // Arrow keys and space walk; they must not scroll the page. Tab still leaves.
  canvas.addEventListener("keydown", (e) => {
    if (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", " "].includes(e.key)) e.preventDefault();
  });
  return canvas;
}

function park() {
  if (!canvas) return;
  if (!parking) {
    parking = document.createElement("div");
    parking.setAttribute("aria-hidden", "true");
    parking.style.cssText = "position:fixed;left:-10px;top:-10px;width:1px;height:1px;overflow:hidden;pointer-events:none";
    document.body.appendChild(parking);
  }
  parking.appendChild(canvas);
}

const bevyView: WorldView = {
  kind: "bevy",
  loadMission: (id) => loadMission(id),
  applyEvents: (j) => applyEvents(j),
  setReducedMotion: (on) => setReducedMotion(on),
  setInputEnabled: (on) => setInputEnabled(on),
  goToTerminal: () => goToTerminal(),
  highlight: (t) => highlight(t),
  onTerminal,
  onEventsDone,
  onNear,
  labels,
};

/**
 * Puts the street in `host` (sized by CSS) and starts it if needed. Resolves to null
 * when the 3D street can't run here; the caller shows the sketch street instead.
 * Call the returned `release` when the host goes away.
 */
export async function openWorld(host: HTMLElement): Promise<{ view: WorldView; release: () => void } | null> {
  if (failed || !webgl2Available()) return null;
  const c = sharedCanvas();
  host.appendChild(c);
  try {
    await start(CANVAS_ID);
  } catch (e) {
    console.warn("3D street unavailable, drawing the sketch street", e);
    c.remove();
    return null;
  }
  return {
    view: bevyView,
    release: () => {
      if (c.parentElement === host) park();
    },
  };
}
