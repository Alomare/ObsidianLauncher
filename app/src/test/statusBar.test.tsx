import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it, vi } from "vitest";

vi.mock("../opener", () => ({ openUrl: vi.fn(() => Promise.resolve()) }));
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("StatusBar", () => {
  it("shows the version, opens About and lists notices", async () => {
    const { StatusBar } = await import("../StatusBar");
    const { notify } = await import("../notices");
    const el = document.createElement("div");
    document.body.append(el);
    const onUpdates = vi.fn();
    act(() => createRoot(el).render(<StatusBar version="0.1.0-beta.1" updateFound={false} onUpdates={onUpdates} />));
    const ver = [...el.querySelectorAll("button")].find((b) => b.textContent === "v0.1.0-beta.1")!;
    act(() => ver.click());
    expect(onUpdates).toHaveBeenCalled();

    act(() => (el.querySelector('[aria-label="About Obsidian"]') as HTMLButtonElement).click());
    expect(el.querySelector('[role="dialog"]')?.textContent).toContain("Apache License 2.0");

    act(() => notify("error", "X was not imported"));
    act(() => (el.querySelector('[aria-label="Notifications"]') as HTMLButtonElement).click());
    expect(el.querySelector(".note-row.error")?.textContent).toContain("X was not imported");
  });
});
