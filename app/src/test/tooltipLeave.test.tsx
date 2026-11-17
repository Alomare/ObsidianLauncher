import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Tooltip } from "../Tooltip";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | undefined;
let target: HTMLButtonElement;
beforeEach(() => {
  vi.useFakeTimers();
  target = document.createElement("button");
  target.dataset.tip = "Turn the mod off";
  document.body.append(target);
  const host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  act(() => root!.render(<Tooltip />));
});
afterEach(() => {
  act(() => root?.unmount());
  document.body.innerHTML = "";
  vi.useRealTimers();
});

const fire = (el: Element, type: string, relatedTarget: Element | null) =>
  act(() => { el.dispatchEvent(new MouseEvent(type, { bubbles: true, relatedTarget })); });

describe("Tooltip", () => {
  it("shows after the wait while the pointer stays", () => {
    fire(target, "pointerover", null);
    act(() => { vi.advanceTimersByTime(600); });
    expect(document.querySelector("[role=tooltip]")?.textContent).toBe("Turn the mod off");
  });

  it("never shows once the pointer left the window, as onto another window over it", () => {
    fire(target, "pointerover", null);
    fire(target, "pointerout", null);
    act(() => { vi.advanceTimersByTime(600); });
    expect(document.querySelector("[role=tooltip]")).toBeNull();
  });

  it("goes away when the pointer leaves the window with the tooltip up", () => {
    fire(target, "pointerover", null);
    act(() => { vi.advanceTimersByTime(600); });
    fire(target, "pointerout", null);
    expect(document.querySelector("[role=tooltip]")).toBeNull();
  });
});
