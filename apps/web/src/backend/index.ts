import { notifyOnce } from "../notice";
import { createTauriBackend } from "./tauri";
import type { Backend } from "./types";
import { createWebBackend } from "./web";

export type * from "./types";
export { nextChunk } from "../path";

let backend: Backend | null = null;

/** Desktop when the Tauri bridge is present, otherwise the browser backend. */
export function getBackend(): Backend {
  if (backend) return backend;
  const w = (globalThis as { window?: { __TAURI_INTERNALS__?: unknown } }).window;
  backend = w?.__TAURI_INTERNALS__ ? createTauriBackend() : createWebBackend({ onNotice: notifyOnce });
  return backend;
}
