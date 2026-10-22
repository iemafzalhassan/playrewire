// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import { BackendError, GO_OFFLINE_COPY } from "../backend/errors";
import ToolchainCard from "./ToolchainCard.svelte";
import { createToolchain } from "./toolchain.svelte";

type Progress = (done: number, total: number) => void;

function fakeBackend(status: "ready" | "missing", installs: ((p: Progress) => Promise<void>)[]) {
  return {
    toolchain: vi.fn(async () => ({ status })),
    installToolchain: vi.fn(async (p: Progress) => (installs.shift() ?? (async () => {}))(p)),
  };
}

describe("toolchain", () => {
  it("missing toolchain shows the one-time download card with progress", async () => {
    let finish!: () => void;
    const b = fakeBackend("missing", [
      (p) =>
        new Promise<void>((res) => {
          p(35_000_000, 70_590_635);
          finish = res;
        }),
    ]);
    const tc = createToolchain(b);
    await tc.check();
    expect(tc.phase).toBe("missing");
    expect(tc.needed).toBe(true);

    render(ToolchainCard, { props: { toolchain: tc } });
    expect(screen.getByText(/download Go once \(about 70 MB\)/)).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: /download Go/i }));

    const bar = (await screen.findByRole("progressbar", { name: "Go download" })) as HTMLProgressElement;
    expect(bar.value).toBe(49);
    expect(screen.getByText("35 of 71 MB")).toBeTruthy();

    finish();
    expect(await screen.findByText(/Go is ready/)).toBeTruthy();
    expect(tc.needed).toBe(false);
  });

  it("offline download shows the Review Focus 2 words and Retry works", async () => {
    const b = fakeBackend("missing", [
      async () => {
        throw new BackendError("offline");
      },
      async (p) => p(70_590_635, 70_590_635),
    ]);
    const tc = createToolchain(b);
    await tc.check();
    render(ToolchainCard, { props: { toolchain: tc } });

    await fireEvent.click(screen.getByRole("button", { name: /download Go/i }));
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toBe(GO_OFFLINE_COPY);
    expect(GO_OFFLINE_COPY).toBe("Rewire needs to download Go once (about 70 MB). Connect to the internet and tap Retry.");
    expect(screen.getByText("✗")).toBeTruthy(); // not color alone

    await fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(tc.phase).toBe("installed"));
    expect(b.installToolchain).toHaveBeenCalledTimes(2);
  });

  it("a second install joins the one already running", async () => {
    let finish!: () => void;
    const b = fakeBackend("missing", [() => new Promise<void>((res) => (finish = res))]);
    const tc = createToolchain(b);
    const one = tc.install();
    const two = tc.install();
    finish();
    await Promise.all([one, two]);
    expect(b.installToolchain).toHaveBeenCalledTimes(1);
  });

  it("ready toolchain needs nothing", async () => {
    const tc = createToolchain(fakeBackend("ready", []));
    await tc.check();
    expect(tc.phase).toBe("ready");
    expect(tc.needed).toBe(false);
  });
});
