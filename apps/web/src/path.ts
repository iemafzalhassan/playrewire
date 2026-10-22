import type { Bundle, Chunk, PathRef } from "./content/types";
import type { ProgressState } from "./core";

export type ChunkRef = PathRef;

export function findChunk(bundle: Bundle, ref: ChunkRef): Chunk | undefined {
  return bundle.packs.find((p) => p.id === ref.pack)?.chunks.find((c) => c.id === ref.chunk);
}

/** True when every required step of the chunk (its `steps`, ruling R6) is completed. */
export function chunkDone(chunk: Chunk, completed: ReadonlySet<string> | string[]): boolean {
  const set = completed instanceof Set ? completed : new Set(completed);
  return chunk.steps.every((s) => set.has(`${chunk.id}:${s}`));
}

/**
 * The first ref in path order with any required step not completed. See-only
 * chunks need only See, so they never wait on a Use step. Refs that aren't in
 * the bundle are skipped. null when the whole path is done.
 */
export function nextChunk(path: ChunkRef[], state: ProgressState, bundle: Bundle): ChunkRef | null {
  const done = new Set(state.completed);
  for (const ref of path) {
    const c = findChunk(bundle, ref);
    if (c && !chunkDone(c, done)) return ref;
  }
  return null;
}
