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
): StepRecord {
  return {
    chunk_id: chunkId,
    concept,
    step: "drill",
    mission_id: null,
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
