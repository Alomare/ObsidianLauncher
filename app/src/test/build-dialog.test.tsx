import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Snapshot } from "../types";

const run = vi.fn();
const call = vi.fn(() => Promise.resolve());
vi.mock("../api", () => ({ run: (...a: unknown[]) => run(...a), call: (...a: unknown[]) => call(...(a as [])), errorText: String }));
const { BuildDialog, fileKind } = await import("../BuildDialog");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | undefined;
afterEach(() => {
  act(() => root?.unmount());
  document.body.replaceChildren();
  run.mockReset();
});

const snap = (gameChanges: Snapshot["gameChanges"]) => ({ game: "unknown", gameDir: "D:\\HELLDIVERS", gameChanges }) as unknown as Snapshot;
function show(s: Snapshot, onClose = () => {}) {
  const host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  act(() => root!.render(<BuildDialog snap={s} onClose={onClose} />));
  return document.querySelector(".modal.build") as HTMLElement;
}

describe("game files dialog", () => {
  it("names the known files", () => {
    expect(fileKind("binaries/x64/helldivers.exe")).toBe("exe");
    expect(fileKind("contents/9e13b2414b41b842")).toBe("gameplay");
    expect(fileKind("contents/0000")).toBe("other");
  });

  it("lists each changed file and explains copied mods", () => {
    const m = show(snap([{ path: "contents/9e13b2414b41b842", missing: false }, { path: "contents/9cf71054ea26dddf", missing: true }]));
    const rows = m.querySelectorAll(".build-files li");
    expect(rows).toHaveLength(2);
    expect(rows[0].textContent).toContain("contents/9e13b2414b41b842");
    expect(rows[1].querySelector(".tag")!.textContent).toBe("Missing");
    expect(m.textContent).toContain("copied straight into the contents folder of the game");
    expect(m.textContent).not.toContain("different version or was patched");
  });

  it("explains a changed game program", () => {
    const m = show(snap([{ path: "binaries/x64/helldivers.exe", missing: false }]));
    expect(m.textContent).toContain("different version or was patched");
    expect(m.textContent).not.toContain("copied straight");
  });

  it("checks again and closes on Escape", () => {
    const onClose = vi.fn();
    const m = show(snap([{ path: "contents/9e13b2414b41b842", missing: false }]), onClose);
    const again = [...m.querySelectorAll("button")].find((b) => b.textContent === "Check again")!;
    act(() => again.click());
    expect(run).toHaveBeenCalledWith("check_game");
    act(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })));
    expect(onClose).toHaveBeenCalled();
  });
});
