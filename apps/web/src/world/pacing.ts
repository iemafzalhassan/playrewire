// When each city event plays. Mirrors crates/rewire-world/src/city_events.rs so the
// sketch street (no-GPU fallback) keeps the same rhythm as the 3D one.
import type { CityEvent } from "../content/types";

/** Green delays play at a quarter of the seconds the program asked for. */
export const GREEN_DELAY_SCALE = 0.25;
/** The beat between two lamp events. */
export const LAMP_STAGGER = 0.15;
/** No event waits longer than this, whatever delay the program asked for. */
export const MAX_WAIT = 8;
/** After the last event, the street holds this long before it says it is done. */
export const SETTLE = 0.6;

/** [seconds from the start, event], sorted; ties keep program order. */
export function schedule(events: CityEvent[]): [number, CityEvent][] {
  let beat = 0;
  const out = events.map((e): [number, CityEvent] => {
    const at = e.op === "green" ? e.delay * GREEN_DELAY_SCALE : e.op === "lamp" ? beat++ * LAMP_STAGGER : 0;
    return [Math.min(at, MAX_WAIT), e];
  });
  return out.map((x, k) => [x, k] as const).sort((a, b) => a[0][0] - b[0][0] || a[1] - b[1]).map(([x]) => x);
}
