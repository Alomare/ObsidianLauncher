import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ContextMenu, type MenuItem } from "../ContextMenu";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const COLORS = ["#111111", "#222222", "#333333"];
const press = (key: string) => act(() => { document.activeElement!.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true })); });

let root: Root | undefined;
afterEach(() => act(() => root?.unmount()));

/** The menu sits on the body, wherever it was opened from. */
function open(items: MenuItem[], onClose = vi.fn()) {
  const el = document.createElement("div");
  document.body.append(el);
  root = createRoot(el);
  act(() => root!.render(<ContextMenu x={10} y={10} items={items} onClose={onClose} />));
  return document.body;
}

describe("ContextMenu colors", () => {
  it("puts the colors in one row and marks the current one", () => {
    const el = open([{ colors: COLORS, current: "#222222", pick: vi.fn() }]);
    const row = el.querySelectorAll(".ctx-colors button");
    expect(el.querySelectorAll(".ctx-colors")).toHaveLength(1);
    // The three colors, then the custom one.
    expect(row).toHaveLength(4);
    expect([...row].map((b) => b.getAttribute("aria-checked"))).toEqual(["false", "true", "false", "false"]);
    expect(el.querySelectorAll(".swatch.on")).toHaveLength(1);
    expect(row[3].classList.contains("custom")).toBe(true);
  });

  it("marks the custom swatch, in that color, when the current color isn't in the row", () => {
    const el = open([{ colors: COLORS, current: "#123456", pick: vi.fn() }]);
    const on = el.querySelectorAll<HTMLButtonElement>(".ctx-colors .swatch.on");
    expect(on).toHaveLength(1);
    expect(on[0].classList.contains("custom")).toBe(true);
    expect(on[0].style.background).toBe("rgb(18, 52, 86)");
  });

  it("picks a color and closes", () => {
    const pick = vi.fn();
    const onClose = vi.fn();
    const el = open([{ colors: COLORS, current: null, pick }], onClose);
    act(() => el.querySelectorAll<HTMLButtonElement>(".ctx-colors button")[2].click());
    expect(pick).toHaveBeenCalledWith("#333333");
    expect(onClose).toHaveBeenCalled();
  });

  it("moves along the row with left and right, and past it with up and down", () => {
    const el = open([{ colors: COLORS, current: null, pick: vi.fn() }, "sep", { label: "Default" }]);
    const row = el.querySelectorAll<HTMLButtonElement>(".ctx-colors button");
    const last = el.querySelector<HTMLButtonElement>(".ctx > button")!;
    expect(document.activeElement).toBe(row[0]);
    press("ArrowRight");
    expect(document.activeElement).toBe(row[1]);
    press("ArrowLeft");
    press("ArrowLeft");
    // Past the first, round to the last: the custom swatch.
    expect(document.activeElement).toBe(row[3]);
    press("ArrowDown");
    expect(document.activeElement).toBe(last);
    press("ArrowDown");
    expect(document.activeElement).toBe(row[0]);
  });

  it("takes the keyboard into a menu opened with the keyboard", () => {
    const el = open([{ label: "Color", sub: [{ colors: COLORS, current: null, pick: vi.fn() }] }]);
    expect(document.activeElement?.textContent).toBe("Color");
    press("ArrowRight");
    expect(document.activeElement).toBe(el.querySelector(".ctx-colors button"));
  });
});
