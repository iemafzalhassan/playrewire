// A WorldView for component tests: records what the Use step asks of the street and
// finishes event playback at once.
import type { CityEvent } from "../content/types";
import type { Labels, Target, WorldView } from "./world";

export class FakeWorld implements WorldView {
  readonly kind = "bevy" as const;
  mission: string | null = null;
  applied: CityEvent[] = [];
  highlighted: Target | null = null;
  reduced = false;
  inputEnabled = true;
  private terminal = new Set<(m: string) => void>();
  private done = new Set<() => void>();
  private near = new Set<(m: string | null) => void>();

  loadMission(id: string) {
    this.mission = id;
  }
  applyEvents(json: string) {
    this.applied = JSON.parse(json);
    queueMicrotask(() => this.done.forEach((cb) => cb()));
  }
  setReducedMotion(on: boolean) {
    this.reduced = on;
  }
  setInputEnabled(on: boolean) {
    this.inputEnabled = on;
  }
  goToTerminal() {
    this.useTerminal();
  }
  highlight(t: Target | null) {
    this.highlighted = t;
  }
  /** The player pressed E at the live terminal. */
  useTerminal() {
    const m = this.mission;
    if (m) this.terminal.forEach((cb) => cb(m));
  }
  walkNear(m: string | null) {
    this.near.forEach((cb) => cb(m));
  }
  onTerminal(cb: (m: string) => void) {
    this.terminal.add(cb);
    return () => this.terminal.delete(cb);
  }
  onEventsDone(cb: () => void) {
    this.done.add(cb);
    return () => this.done.delete(cb);
  }
  onNear(cb: (m: string | null) => void) {
    this.near.add(cb);
    return () => this.near.delete(cb);
  }
  labels(): Labels | null {
    return null;
  }
}
