// @vitest-environment jsdom
import { afterEach, describe, expect, it } from "vitest";
import { createRouter, parseHash } from "./router.svelte";

afterEach(() => {
  location.hash = "";
});

describe("router", () => {
  it("parses the known routes", () => {
    expect(parseHash("")).toEqual({ name: "home" });
    expect(parseHash("#/map")).toEqual({ name: "map" });
    expect(parseHash("#/learn/go-academy/for-loop")).toEqual({ name: "learn", pack: "go-academy", chunk: "for-loop" });
  });

  it("malformed hash shows the missing page at startup", () => {
    expect(parseHash("#/%E0%A4%A")).toEqual({ name: "missing" });
    location.hash = "#/%E0%A4%A";
    const r = createRouter();
    expect(r.route).toEqual({ name: "missing" });
    r.destroy();
  });

  it("malformed hash shows the missing page on hashchange", () => {
    location.hash = "#/map";
    const r = createRouter();
    expect(r.route).toEqual({ name: "map" });
    location.hash = "#/learn/%E0%A4%A/x";
    dispatchEvent(new HashChangeEvent("hashchange"));
    expect(r.route).toEqual({ name: "missing" });
    r.destroy();
  });
});
