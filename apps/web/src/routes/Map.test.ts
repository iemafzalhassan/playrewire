// @vitest-environment jsdom
import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import Map from "./Map.svelte";

describe("Map", () => {
  it("Map inks two districts and leaves eight as blank paper", () => {
    render(Map, {
      props: {
        districts: [
          { id: "silicon-quarter", name: "Silicon Quarter", done: 1, total: 4, chunks: [] },
          { id: "go-academy", name: "Go Academy", done: 0, total: 6, chunks: [] },
        ],
        onOpen: () => {},
      },
    });
    const open = screen.getAllByRole("button", { name: /Silicon Quarter|Go Academy/ });
    expect(open).toHaveLength(2);
    const closed = screen.getAllByLabelText(/not yet open/);
    expect(closed).toHaveLength(8);
    for (const c of closed) expect(c.tagName.toLowerCase()).not.toBe("button");
  });
});
