import { afterEach, describe, expect, it, vi } from "vitest";
import { _useModuleForTest, bevyView } from "./world";

afterEach(() => _useModuleForTest(null));

describe("bevyView after the wasm traps", () => {
  it("a throwing call marks the street failed, tells listeners, and stops calling in", () => {
    const applyEvents = vi.fn(() => {
      throw new Error("RuntimeError: unreachable");
    });
    const highlight = vi.fn();
    _useModuleForTest({ applyEvents, highlight, labels: () => "null" });
    const failed = vi.fn();
    const off = bevyView.onFailed!(failed);
    vi.spyOn(console, "error").mockImplementation(() => {});

    expect(() => bevyView.applyEvents("[]")).toThrow(/unreachable/);
    expect(failed).toHaveBeenCalledOnce();
    expect(() => bevyView.highlight(null)).toThrow(/stopped/);
    expect(highlight).not.toHaveBeenCalled();
    expect(bevyView.labels()).toBeNull();
    expect(failed).toHaveBeenCalledOnce();
    off();
  });

  it("a healthy module is called straight through", () => {
    const highlight = vi.fn();
    _useModuleForTest({ highlight });
    bevyView.highlight({ kind: "light", i: 2 });
    expect(highlight).toHaveBeenCalledWith('{"kind":"light","i":2}');
  });
});
