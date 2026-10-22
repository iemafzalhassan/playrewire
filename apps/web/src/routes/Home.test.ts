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
});
