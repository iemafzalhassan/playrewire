// @vitest-environment jsdom
import { render, screen } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import Home from "./Home.svelte";

describe("Home", () => {
  it("Home shows streak, xp, level and a single primary action", async () => {
    const onGo = vi.fn();
    const { container } = render(Home, {
      props: {
        streak: 12,
        xp: 340,
        level: 3,
        newIdeas: 4,
        toFix: 2,
        trail: [
          { title: "What is a computer?", done: true },
          { title: "Hello, print", done: false },
          { title: "Variables", done: false },
        ],
        nextTitle: "Hello, print",
        onGo,
      },
    });
    expect(screen.getByTestId("stat-streak").textContent).toContain("12");
    expect(screen.getByTestId("stat-ideas").textContent).toContain("4");
    expect(screen.getByTestId("stat-fix").textContent).toContain("2");
    expect(screen.getByTestId("xp").textContent).toContain("340");
    expect(screen.getByTestId("level").textContent).toContain("3");

    const primary = container.querySelectorAll("button[data-primary]");
    expect(primary).toHaveLength(1);
    expect(primary[0].textContent).toMatch(/let's go/);
    (primary[0] as HTMLButtonElement).click();
    expect(onGo).toHaveBeenCalledOnce();
  });

  it("Home shows the day's three parts in order, with their progress", () => {
    render(Home, {
      props: {
        streak: 3,
        xp: 120,
        level: 1,
        newIdeas: 6,
        toFix: 3,
        trail: [{ title: "Hello, print", done: true }],
        nextTitle: "Variables",
        day: [
          { key: "warmup", label: "warm-up", detail: "2 quick drills, about a minute", state: "done" },
          { key: "chunk", label: "new page", detail: "Variables", state: "now" },
          { key: "fixit", label: "fix-it", detail: "Your first words", state: "later" },
        ],
        onGo: vi.fn(),
      },
    });
    const parts = screen.getByRole("list", { name: /today's plan/i });
    const items = Array.from(parts.querySelectorAll("li"));
    expect(items.map((li) => li.dataset.part)).toEqual(["warmup", "chunk", "fixit"]);
    expect(items[0].textContent).toContain("✓");
    expect(items[0].textContent).toMatch(/done/);
    expect(items[1].getAttribute("aria-current")).toBe("step");
    expect(items[2].textContent).toContain("Your first words");
    expect(screen.getByRole("button", { name: /keep going/i }).hasAttribute("data-primary")).toBe(true);
  });
});
