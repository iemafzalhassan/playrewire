// The lesson bundle, fetched and checked once. A failed load is not cached,
// so "try again" really tries again.
import { BundleSchema, type Bundle } from "./types";

let cached: Promise<Bundle> | null = null;

export function loadBundle(fetcher: typeof fetch = fetch): Promise<Bundle> {
  cached ??= (async () => {
    const r = await fetcher(`${import.meta.env.BASE_URL}content/go.json`);
    if (!r.ok) throw new Error(`content: HTTP ${r.status}`);
    return BundleSchema.parse(await r.json());
  })().catch((e) => {
    cached = null;
    throw e;
  });
  return cached;
}

/** Friendly words for a failed load: offline, or something else. */
export function loadTrouble(): { title: string; text: string } {
  const offline = typeof navigator !== "undefined" && navigator.onLine === false;
  return offline
    ? { title: "You're offline", text: "Rewire needs a connection to fetch today's pages. Connect, then try again." }
    : { title: "The pages didn't load", text: "Something went wrong while opening the lessons. Try again in a moment." };
}

/** Test hook. */
export function resetBundleCache() {
  cached = null;
}
