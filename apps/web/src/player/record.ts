// One finished (or failed) step: core rules -> saved -> passed up. Shared by the
// chunk player, the warm-up and the fix-it task. Only the step outcome is saved,
// never the code or the keys typed.
import type { Backend } from "../backend/types";
import { applyStep, initCore, type ProgressState, type Reward, type StepRecord } from "../core";
import { localDate } from "./drill";

export type Saver = Pick<Backend, "saveProgress">;

export function stepRecord(
  chunkId: string,
  concept: string,
  outcome: { success: boolean; firstTry: boolean; hintsUsed: number },
  step: StepRecord["step"] = "drill",
  missionId: string | null = null,
): StepRecord {
  return {
    chunk_id: chunkId,
    concept,
    step,
    mission_id: missionId,
    success: outcome.success,
    first_try: outcome.firstTry,
    hints_used: outcome.hintsUsed,
    local_date: localDate(),
  };
}

/** Applies and saves one record. On trouble it logs and returns null, so no learner is trapped on a step. */
export async function recordStep(
  progress: ProgressState,
  rec: StepRecord,
  backend: Saver,
): Promise<{ state: ProgressState; rewards: Reward[] } | null> {
  try {
    await initCore();
    const out = applyStep(progress, rec);
    await backend.saveProgress(out.state, rec);
    return out;
  } catch (e) {
    console.error("could not save the step", e);
    return null;
  }
}

/**
 * Saves records one after another, each applied to the state the previous one
 * produced. A failed round saved just before the pass that follows it can't race
 * on a stale state and lose the failure.
 */
export function saveQueue(start: ProgressState, backend: Saver, onSaved: (s: ProgressState) => void) {
  let latest = start;
  let queue: Promise<unknown> = Promise.resolve();
  return {
    save(rec: StepRecord): Promise<{ state: ProgressState; rewards: Reward[] } | null> {
      const done = queue.then(async () => {
        const out = await recordStep(latest, rec, backend);
        if (out) {
          latest = out.state;
          onSaved(out.state);
        }
        return out;
      });
      queue = done;
      return done;
    },
  };
}
