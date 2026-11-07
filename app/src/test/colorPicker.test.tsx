import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ColorPickerHost, hexToHsv, hsvToHex, openColorPicker, parseHex } from "../ColorPicker";
import { ContextMenu } from "../ContextMenu";
import { FolderDialog } from "../mods/FolderDialog";
import { t } from "../i18n";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("the color math", () => {
  it("reads #rrggbb, rrggbb and #rgb, and nothing else", () => {
    expect(parseHex("#F0B429")).toBe("#f0b429");
    expect(parseHex("f0b429")).toBe("#f0b429");
    expect(parseHex("#abc")).toBe("#aabbcc");
    for (const bad of ["", "#12345", "#ggg000", "red", null]) expect(parseHex(bad)).toBeNull();
  });
  it("goes to hue, saturation and brightness and back without a change", () => {
    for (const c of ["#f0b429", "#000000", "#ffffff", "#123456", "#ff0000", "#2fb889", "#8a9099"]) expect(hsvToHex(hexToHsv(c))).toBe(c);
    expect(hexToHsv("#ff0000")).toEqual({ h: 0, s: 1, v: 1 });
    expect(hexToHsv("#0000ff").h).toBe(240);
  });
});

// The picker, opened from a button in a page, with the boxes a window would give them.
let root: Root | undefined;
let opener: HTMLButtonElement;
const rects = new Map<string, Partial<DOMRect>>();
beforeEach(() => {
  rects.clear();
  vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (this: Element) {
    const key = this.classList.contains("picker") ? "picker" : this.classList.contains("picker-area") ? "area" : this.classList.contains("picker-hue") ? "hue" : this.id;
    const r = { left: 0, top: 0, right: 0, bottom: 0, width: 0, height: 0, x: 0, y: 0, ...rects.get(key) };
    return { ...r, toJSON: () => r } as DOMRect;
  });
});
afterEach(() => {
  act(() => document.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true })));
  act(() => root?.unmount());
  document.body.replaceChildren();
  vi.restoreAllMocks();
});

function page(extra?: React.ReactNode) {
  const host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  act(() => root!.render(<><button id="opener">Color</button>{extra}<ColorPickerHost /></>));
  opener = document.getElementById("opener") as HTMLButtonElement;
}
const picker = () => document.querySelector<HTMLElement>(".picker");
const press = (key: string, shiftKey = false) =>
  act(() => document.activeElement!.dispatchEvent(new KeyboardEvent("keydown", { key, shiftKey, bubbles: true, cancelable: true })));

describe("the color picker", () => {
  it("opens below the control that asked, from the same left edge, with the focus on the field", () => {
    rects.set("opener", { left: 100, top: 170, bottom: 200, width: 40, height: 30 });
    rects.set("picker", { width: 250, height: 300 });
    page();
    act(() => openColorPicker(opener, { current: "#2fb889", title: "Custom color", onPick: () => {} }));
    expect(picker()!.style.left).toBe("100px");
    expect(picker()!.style.top).toBe("206px");
    expect(picker()!.getAttribute("role")).toBe("dialog");
    expect(picker()!.getAttribute("aria-label")).toBe("Custom color");
    expect(document.activeElement?.className).toBe("picker-area");
    expect(document.querySelector<HTMLInputElement>(".picker-hex")!.value).toBe("#2fb889");
  });

  it("opens above the control near the foot of the window, and inside it at the right edge", () => {
    rects.set("opener", { left: 900, top: 700, bottom: 730, width: 40, height: 30 });
    rects.set("picker", { width: 250, height: 300 });
    page();
    act(() => openColorPicker(opener, { current: null, title: "T", onPick: () => {} }));
    expect(picker()!.style.top).toBe(`${700 - 6 - 300}px`);
    expect(picker()!.style.left).toBe(`${innerWidth - 4 - 250}px`);
    expect(document.querySelector<HTMLInputElement>(".picker-hex")!.value).toBe("#f0b429");
  });

  it("moves with the arrow keys, Shift in bigger steps, and Enter applies, with the focus back on the control", () => {
    page();
    const onPick = vi.fn(), onInput = vi.fn();
    act(() => openColorPicker(opener, { current: "#808080", title: "T", onPick, onInput }));
    const area = document.querySelector(".picker-area")!;
    expect(area.getAttribute("aria-valuetext")).toBe(t("app.color.areaValue", 0, 50));
    press("ArrowRight", true);
    press("ArrowUp");
    expect(area.getAttribute("aria-valuetext")).toBe(t("app.color.areaValue", 10, 51));
    act(() => (document.querySelector(".picker-hue") as HTMLElement).focus());
    press("End");
    expect(document.querySelector(".picker-hue")!.getAttribute("aria-valuenow")).toBe("359");
    press("ArrowDown", true);
    expect(document.querySelector(".picker-hue")!.getAttribute("aria-valuenow")).toBe("349");
    expect(onInput).toHaveBeenCalled();
    const shown = document.querySelector<HTMLInputElement>(".picker-hex")!.value;
    press("Enter");
    expect(onPick).toHaveBeenCalledWith(shown);
    expect(picker()).toBeNull();
    expect(document.activeElement).toBe(opener);
  });

  it("takes a typed code, and Apply gives it", () => {
    page();
    const onPick = vi.fn();
    act(() => openColorPicker(opener, { current: "#f0b429", title: "T", onPick }));
    const hex = document.querySelector<HTMLInputElement>(".picker-hex")!;
    const type = (v: string) => act(() => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(hex, v);
      hex.dispatchEvent(new Event("input", { bubbles: true }));
    });
    type("#12");
    type("123456");
    expect(document.querySelector<HTMLElement>(".picker-preview i:last-child")!.style.background).toBe("rgb(18, 52, 86)");
    act(() => [...document.querySelectorAll<HTMLButtonElement>(".picker-foot .btn")].find((b) => b.textContent === t("app.color.apply"))!.click());
    expect(onPick).toHaveBeenCalledWith("#123456");
  });

  it("cancels on Esc, Cancel or a click outside, puts the first color back, and Esc stops at the picker", () => {
    const below = vi.fn();
    window.addEventListener("keydown", below);
    page();
    for (const how of ["esc", "cancel", "outside"]) {
      const onPick = vi.fn(), onInput = vi.fn();
      act(() => openColorPicker(opener, { current: "#2fb889", title: "T", onPick, onInput }));
      press("ArrowLeft", true);
      if (how === "esc") press("Escape");
      if (how === "cancel") act(() => [...document.querySelectorAll<HTMLButtonElement>(".picker-foot .btn")].find((b) => b.textContent === t("app.cancel"))!.click());
      if (how === "outside") act(() => document.body.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true })));
      expect(picker(), how).toBeNull();
      expect(onPick, how).not.toHaveBeenCalled();
      expect(onInput.mock.calls.at(-1), how).toEqual(["#2fb889"]);
    }
    expect(below.mock.calls.filter((c) => (c[0] as KeyboardEvent).key === "Escape")).toHaveLength(0);
    window.removeEventListener("keydown", below);
  });

  it("keeps Tab inside, going round from Apply to the field", () => {
    page();
    act(() => openColorPicker(opener, { current: null, title: "T", onPick: () => {} }));
    const apply = [...document.querySelectorAll<HTMLButtonElement>(".picker-foot .btn")].at(-1)!;
    act(() => apply.focus());
    press("Tab");
    expect(document.activeElement?.className).toBe("picker-area");
    press("Tab", true);
    expect(document.activeElement).toBe(apply);
  });

  it("from a color menu: the menu closes as the picker opens, and the color picked reaches the menu's choice", () => {
    const pick = vi.fn(), close = vi.fn();
    page(<ContextMenu x={1} y={1} onClose={close} items={[{ colors: ["#f0b429"], current: "#f0b429", pick }]} />);
    act(() => document.querySelector<HTMLButtonElement>(".ctx-colors .swatch.custom")!.click());
    expect(close).toHaveBeenCalled();
    expect(picker()).not.toBeNull();
    press("Enter");
    expect(pick).toHaveBeenCalledWith("#f0b429");
  });

  it("in the folder dialog: the dialog follows the color live, takes it on Apply, and stays open on Esc", () => {
    const cancel = vi.fn();
    page(<FolderDialog title="New folder" confirm="Create" color="#f0b429" onConfirm={() => {}} onCancel={cancel} />);
    act(() => document.querySelector<HTMLButtonElement>(".swatches .swatch.custom")!.click());
    press("ArrowDown", true);
    const form = document.querySelector<HTMLElement>("form.modal")!;
    expect(form.style.borderTopColor).not.toBe("rgb(240, 180, 41)");
    press("Escape");
    expect(cancel).not.toHaveBeenCalled();
    expect(form.style.borderTopColor).toBe("rgb(240, 180, 41)");
    act(() => document.querySelector<HTMLButtonElement>(".swatches .swatch.custom")!.click());
    press("ArrowDown", true);
    press("Enter");
    expect(document.querySelector(".swatches .swatch.custom.on")).not.toBeNull();
  });
});
