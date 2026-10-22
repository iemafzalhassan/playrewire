// Desktop commands fail with a short, stable code (apps/desktop/src-tauri/src/commands.rs).
// The UI never shows the code itself, only the plain words below.

export type BackendErrorCode =
  | "toolchain_missing"
  | "sandbox_unavailable"
  | "offline"
  | "checksum_mismatch"
  | "io"
  | "invalid_input"
  | "keychain_unavailable";

/** Review Focus 2: Go isn't installed and the device is offline. Exact words. */
export const GO_OFFLINE_COPY = "Rewire needs to download Go once (about 70 MB). Connect to the internet and tap Retry.";

/** Before the download: what it is and why. */
export const GO_DOWNLOAD_COPY = "Rewire needs to download Go once (about 70 MB) to run your code. Reading and typing work without it.";

export const ERROR_COPY: Record<BackendErrorCode, { title: string; text: string }> = {
  toolchain_missing: { title: "Go isn't here yet", text: GO_DOWNLOAD_COPY },
  sandbox_unavailable: {
    title: "The practice space didn't start",
    text: "Rewire couldn't start its safe practice space on this system. See docs: Linux sandbox setup.",
  },
  offline: { title: "Can't reach the Go download", text: GO_OFFLINE_COPY },
  checksum_mismatch: {
    title: "The download didn't check out",
    text: "The Go download didn't match what Rewire expects, so Rewire threw it away. Tap Retry to download it again.",
  },
  io: { title: "Something went wrong", text: "Something went wrong on this computer. Try again in a moment." },
  invalid_input: { title: "That didn't work", text: "Rewire couldn't use that. Check it and try again." },
  keychain_unavailable: {
    title: "Your key wasn't saved",
    text: "Rewire couldn't save your key in this computer's keychain. Check that a keychain (keyring) app is running, then try again.",
  },
};

export class BackendError extends Error {
  readonly code: BackendErrorCode;
  constructor(code: BackendErrorCode) {
    super(ERROR_COPY[code].text);
    this.name = "BackendError";
    this.code = code;
  }
}

const isCode = (v: unknown): v is BackendErrorCode => typeof v === "string" && Object.hasOwn(ERROR_COPY, v);

/** Turns what a Tauri command rejected with into an error the UI can explain. */
export function toBackendError(e: unknown): Error {
  if (e instanceof Error) return e;
  // Commands reject with a bare code string; anything else is unexpected.
  return new BackendError(isCode(e) ? e : "io");
}
