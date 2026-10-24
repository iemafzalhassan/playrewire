import { beforeAll, describe, expect, it, vi } from "vitest";
import { emptyProgress } from "../backend/types";
import { STORAGE_NOTICE } from "../backend/web";
import { initCore, type StepRecord } from "../core";
import { onNotice } from "../notice";
import { recordStep } from "./record";

beforeAll(async () => {
  await initCore();
});

const rec: StepRecord = {
  chunk_id: "hello-print",
  concept: "print",
  step: "see",
  mission_id: null,
  success: true,
  first_try: true,
  hints_used: 0,
  local_date: "2026-10-22",
};

describe("recordStep", () => {
  it("a save that fails shows the one-line notice", async () => {
    const seen: string[] = [];
    const off = onNotice((t) => seen.push(t));
    vi.spyOn(console, "error").mockImplementation(() => {});
    const saveProgress = vi.fn(async () => {
      throw new Error("quota");
    });
    expect(await recordStep(emptyProgress(), rec, { saveProgress })).toBeNull();
    // A state the rules refuse never reaches the backend, and says so the same way.
    const broken = { ...emptyProgress(), streak: { current: 0, best: 0, last_day: null, freezes: 999 } };
    const save2 = vi.fn();
    expect(await recordStep(broken, rec, { saveProgress: save2 })).toBeNull();
    expect(save2).not.toHaveBeenCalled();
    expect(seen).toEqual([STORAGE_NOTICE]);
    off();
  });
});
