// The sketch street: the no-GPU fallback. Same API and the same pacing as the 3D
// street, drawn as an inked SVG (StreetSketch.svelte). There is no walking: "go to the
// terminal" opens it straight away.
import type { CityEvent } from "../content/types";
import { SETTLE, schedule } from "./pacing";
import type { Labels, Target, WorldView } from "./world";

export const LIGHTS = 5;
export const LAMPS = 8;

export class SketchWorld implements WorldView {
  readonly kind = "sketch" as const;
  mission = $state<string | null>(null);
  lights = $state<boolean[]>(Array(LIGHTS).fill(false));
  lamps = $state<boolean[]>(Array(LAMPS).fill(false));
  ring = $state<Target | null>(null);
  reduced = $state(false);
  private timers: ReturnType<typeof setTimeout>[] = [];
  private terminalCbs = new Set<(m: string) => void>();
  private doneCbs = new Set<() => void>();

  loadMission(id: string) {
    this.mission = id;
    this.reset();
  }

  private reset() {
    this.timers.splice(0).forEach(clearTimeout);
    this.lights = Array(LIGHTS).fill(false);
    this.lamps = Array(LAMPS).fill(false);
    this.ring = null;
  }

  private play(e: CityEvent) {
    if (e.op === "green" && e.i < LIGHTS) this.lights[e.i] = true;
    if (e.op === "lamp" && e.i < LAMPS) this.lamps[e.i] = e.on;
  }

  private done() {
    this.doneCbs.forEach((cb) => cb());
  }

  applyEvents(eventsJson: string) {
    this.reset();
    const queue = schedule(JSON.parse(eventsJson) as CityEvent[]);
    if (this.reduced) {
      queue.forEach(([, e]) => this.play(e));
      queueMicrotask(() => this.done());
      return;
    }
    for (const [at, e] of queue) this.timers.push(setTimeout(() => this.play(e), at * 1000));
    const last = queue.length ? queue[queue.length - 1][0] : 0;
    this.timers.push(setTimeout(() => this.done(), (last + SETTLE) * 1000));
  }

  setReducedMotion(on: boolean) {
    this.reduced = on;
  }
  setInputEnabled() {}
  goToTerminal() {
    const m = this.mission;
    if (m) this.terminalCbs.forEach((cb) => cb(m));
  }
  highlight(t: Target | null) {
    this.ring = t;
  }
  onTerminal(cb: (m: string) => void) {
    this.terminalCbs.add(cb);
    return () => this.terminalCbs.delete(cb);
  }
  onEventsDone(cb: () => void) {
    this.doneCbs.add(cb);
    return () => this.doneCbs.delete(cb);
  }
  onNear() {
    return () => {};
  }
  labels(): Labels | null {
    return null;
  }
  destroy() {
    this.timers.splice(0).forEach(clearTimeout);
  }
}
