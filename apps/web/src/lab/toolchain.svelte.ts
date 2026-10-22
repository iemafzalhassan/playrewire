// The desktop Go toolchain, as the UI sees it: is it here, is it downloading,
// did the download fail. One instance per app (App puts it in context), so the
// top-of-page card and the card inside a drill show the same download.
import { getContext, setContext } from "svelte";
import { BackendError, type BackendErrorCode } from "../backend/errors";
import type { Backend } from "../backend/types";

export type ToolchainPhase = "unknown" | "n/a" | "ready" | "missing" | "installing" | "failed" | "installed";

export interface Toolchain {
  readonly phase: ToolchainPhase;
  /** Bytes so far and expected, while installing. */
  readonly done: number;
  readonly total: number;
  /** Why the last download failed. */
  readonly error: BackendErrorCode | null;
  /** True while the player still has to do something (download or retry). */
  readonly needed: boolean;
  check(): Promise<void>;
  /** A run said the toolchain is missing (it may have been removed). */
  markMissing(): void;
  /** Starts the download, or joins the one already running. */
  install(): Promise<void>;
}

export function createToolchain(backend: Pick<Backend, "toolchain" | "installToolchain">): Toolchain {
  const s = $state({ phase: "unknown" as ToolchainPhase, done: 0, total: 0, error: null as BackendErrorCode | null });
  let inflight: Promise<void> | null = null;

  return {
    get phase() {
      return s.phase;
    },
    get done() {
      return s.done;
    },
    get total() {
      return s.total;
    },
    get error() {
      return s.error;
    },
    get needed() {
      return s.phase === "missing" || s.phase === "installing" || s.phase === "failed";
    },
    async check() {
      if (inflight) return;
      try {
        const { status } = await backend.toolchain();
        if (!inflight) s.phase = status;
      } catch {
        // Can't tell; a run will report it if Go is really missing.
        if (!inflight) s.phase = "unknown";
      }
    },
    markMissing() {
      if (!inflight) s.phase = "missing";
    },
    install() {
      if (inflight) return inflight;
      s.phase = "installing";
      s.done = 0;
      s.total = 0;
      s.error = null;
      inflight = backend
        .installToolchain((done, total) => {
          s.done = done;
          s.total = total;
        })
        .then(
          () => {
            s.phase = "installed";
          },
          (e: unknown) => {
            s.error = e instanceof BackendError ? e.code : "io";
            s.phase = "failed";
          },
        )
        .finally(() => {
          inflight = null;
        });
      return inflight;
    },
  };
}

/** Exported for tests that render a component inside the shell's context. */
export const TOOLCHAIN_CONTEXT = Symbol("toolchain");

export function setToolchainContext(t: Toolchain) {
  setContext(TOOLCHAIN_CONTEXT, t);
}

/** `undefined` outside the app shell (web tests, stories). */
export function getToolchainContext(): Toolchain | undefined {
  return getContext<Toolchain | undefined>(TOOLCHAIN_CONTEXT);
}
