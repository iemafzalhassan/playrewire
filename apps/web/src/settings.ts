import type { Settings } from "./backend/types";

/** Apply comfort settings to <html>; CSS reads the data attributes. */
export function applySettings(s: Settings, root: HTMLElement = document.documentElement) {
  const systemCalm = globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
  const calm = s.reducedMotion === "system" ? systemCalm : s.reducedMotion;
  root.dataset.motion = calm ? "reduced" : "full";
  root.dataset.font = s.dyslexiaFont ? "dyslexic" : "standard";
  root.style.setProperty("--text-scale", String(s.textScale));
}
