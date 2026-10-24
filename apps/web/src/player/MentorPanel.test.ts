// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MENTOR_FALLBACK, type AgUiEvent, type HintRequest } from "../backend/types";
import MentorPanel from "./MentorPanel.svelte";

const hints = ["first hint", "second hint", "third hint"];
const req: HintRequest = {
  concept: "variables",
  exercise_prompt: "Make a box called count that holds 3.",
  code: "count = 3",
  last_error: "undefined: count",
  attempt: 2,
};

type Emit = (e: AgUiEvent) => void;

/** A desktop mentor whose stream the test drives by hand. */
function desktopMentor(script: (emit: Emit) => Promise<void> | void) {
  return {
    kind: "desktop" as const,
    mentorAvailable: () => true,
    mentorHint: vi.fn(async (_r: HintRequest, onEvent: Emit) => {
      await script(onEvent);
    }),
  };
}

afterEach(() => {
  delete document.documentElement.dataset.motion;
});

describe("MentorPanel", () => {
  it("web shows built-in hints only", async () => {
    const mentorHint = vi.fn();
    const onHelp = vi.fn();
    render(MentorPanel, {
      props: { mentor: { kind: "web", mentorAvailable: () => false, mentorHint }, hints, request: () => req, onHelp },
    });
    expect(screen.getByText("AI mentor is available in the desktop app.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /ask the mentor/i })).toBeNull();
    await fireEvent.click(screen.getByRole("button", { name: /hint \(3 left\)/i }));
    expect(screen.getByText("first hint")).toBeTruthy();
    expect(screen.queryByText("second hint")).toBeNull();
    expect(onHelp).toHaveBeenCalledOnce();
    expect(mentorHint).not.toHaveBeenCalled();
  });

  it("mentor panel falls back on RUN_ERROR", async () => {
    const mentor = desktopMentor((emit) => {
      emit({ type: "RUN_STARTED", threadId: "t", runId: "r" });
      emit({ type: "RUN_ERROR", message: MENTOR_FALLBACK });
    });
    const onHelp = vi.fn();
    render(MentorPanel, { props: { mentor, hints, request: () => req, onHelp } });
    expect(screen.getByText("Sends only this exercise and your code.")).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: /ask the mentor/i }));
    await waitFor(() => expect(screen.getByText(MENTOR_FALLBACK)).toBeTruthy());
    // Straight onto the lesson's own hint ladder: the first hint is out, the rest wait.
    expect(screen.getByText("first hint")).toBeTruthy();
    expect(screen.queryByText("second hint")).toBeNull();
    expect(screen.queryByRole("button", { name: /ask the mentor/i })).toBeNull();
    await fireEvent.click(screen.getByRole("button", { name: /hint \(2 left\)/i }));
    expect(screen.getByText("second hint")).toBeTruthy();
    // The fallback hint, then one more; the failed ask itself gave no help.
    expect(onHelp).toHaveBeenCalledTimes(2);
  });

  it("the key error says so and still falls back", async () => {
    const keyMsg = "The mentor couldn't connect. Check your key in Settings.";
    const mentor = desktopMentor((emit) => emit({ type: "RUN_ERROR", message: keyMsg }));
    render(MentorPanel, { props: { mentor, hints, request: () => req } });
    await fireEvent.click(screen.getByRole("button", { name: /ask the mentor/i }));
    await waitFor(() => expect(screen.getByText(keyMsg)).toBeTruthy());
    expect(screen.getByText("first hint")).toBeTruthy();
  });

  it("a mentor call that throws falls back like RUN_ERROR", async () => {
    const mentor = desktopMentor(() => {
      throw new Error("ipc gone");
    });
    render(MentorPanel, { props: { mentor, hints, request: () => req } });
    await fireEvent.click(screen.getByRole("button", { name: /ask the mentor/i }));
    await waitFor(() => expect(screen.getByText(MENTOR_FALLBACK)).toBeTruthy());
    expect(screen.getByText("first hint")).toBeTruthy();
  });

  it("streams the note in, sends only the hint request, and can't be asked twice at once", async () => {
    let emit!: Emit;
    let end!: () => void;
    const mentor = desktopMentor(
      (e) =>
        new Promise<void>((resolve) => {
          emit = e;
          end = resolve;
        }),
    );
    const extra = { ...req, name: "Mia", player_id: "p1", settings: { key: "sk-x" } } as unknown as HintRequest;
    render(MentorPanel, { props: { mentor, hints, request: () => extra } });
    const ask = screen.getByRole("button", { name: /ask the mentor/i });
    await fireEvent.click(ask);
    expect(mentor.mentorHint).toHaveBeenCalledOnce();
    expect(Object.keys(mentor.mentorHint.mock.calls[0][0]).sort()).toEqual(
      ["attempt", "code", "concept", "exercise_prompt", "last_error"],
    );
    expect(mentor.mentorHint.mock.calls[0][0]).toEqual(req);
    await waitFor(() => expect((screen.getByRole("button", { name: /mentor is writing|ask the mentor/i }) as HTMLButtonElement).disabled).toBe(true));

    emit({ type: "RUN_STARTED", threadId: "t", runId: "r" });
    emit({ type: "TEXT_MESSAGE_START", messageId: "m", role: "assistant" });
    emit({ type: "TEXT_MESSAGE_CONTENT", messageId: "m", delta: "Look at how " });
    const note = await screen.findByTestId("mentor-note");
    await waitFor(() => expect(note.textContent).toContain("Look at how"));
    expect(note.textContent).not.toContain("count");
    emit({ type: "TEXT_MESSAGE_CONTENT", messageId: "m", delta: "you made count." });
    await waitFor(() => expect(note.textContent).toContain("Look at how you made count."));
    emit({ type: "TEXT_MESSAGE_END", messageId: "m" });
    emit({ type: "RUN_FINISHED", threadId: "t", runId: "r" });
    end();
    await waitFor(() => expect((screen.getByRole("button", { name: /ask the mentor/i }) as HTMLButtonElement).disabled).toBe(false));
    // No fallback, no built-in hint forced out.
    expect(screen.queryByText(MENTOR_FALLBACK)).toBeNull();
    expect(screen.queryByText("first hint")).toBeNull();
  });

  it("with reduced motion the note appears all at once", async () => {
    document.documentElement.dataset.motion = "reduced";
    let emit!: Emit;
    let end!: () => void;
    const mentor = desktopMentor(
      (e) =>
        new Promise<void>((resolve) => {
          emit = e;
          end = resolve;
        }),
    );
    render(MentorPanel, { props: { mentor, hints, request: () => req } });
    await fireEvent.click(screen.getByRole("button", { name: /ask the mentor/i }));
    await waitFor(() => expect(emit).toBeTypeOf("function"));
    emit({ type: "TEXT_MESSAGE_START", messageId: "m", role: "assistant" });
    emit({ type: "TEXT_MESSAGE_CONTENT", messageId: "m", delta: "Look at how " });
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("mentor-note")?.textContent ?? "").not.toContain("Look");
    emit({ type: "TEXT_MESSAGE_CONTENT", messageId: "m", delta: "you made count." });
    emit({ type: "TEXT_MESSAGE_END", messageId: "m" });
    emit({ type: "RUN_FINISHED", threadId: "t", runId: "r" });
    end();
    await waitFor(() => expect(screen.getByTestId("mentor-note").textContent).toContain("Look at how you made count."));
  });

  it("desktop without a mentor set up points to Settings and keeps the built-in hints", () => {
    render(MentorPanel, {
      props: { mentor: { kind: "desktop", mentorAvailable: () => false, mentorHint: vi.fn() }, hints, request: () => req },
    });
    expect(screen.queryByRole("button", { name: /ask the mentor/i })).toBeNull();
    expect(screen.getByRole("link", { name: /settings/i }).getAttribute("href")).toBe("#/settings");
    expect(screen.getByRole("button", { name: /hint/i })).toBeTruthy();
  });
});
