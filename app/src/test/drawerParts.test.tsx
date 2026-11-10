import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ModRow, OptionRow } from "../types";

vi.mock("../api", () => ({ run: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
const { Drawer } = await import("../mods/Drawer");
const { partLines } = await import("../mods/ModTable");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | undefined;
afterEach(() => {
  act(() => root?.unmount());
  document.body.replaceChildren();
  localStorage.clear();
});

const option = (id: string, extra: Partial<OptionRow>): OptionRow => ({
  id, kind: "number", label: { en: id }, description: {}, default: 4, value: 4, min: 1, max: 10, step: 1, choices: [], choiceLabels: [],
  visible: true, deploy: false, group: null, part: null, ...extra,
});

const cam = (): ModRow => ({
  id: "a.cam", name: { en: "Cam" }, description: { en: "A camera." }, authors: [], version: "1.0.0", scope: "hybrid", on: true,
  parts: [
    { id: "view", scope: "client", name: { en: "Camera and view" }, description: { en: "Keeps working in any lobby." }, on: true },
    { id: "range", scope: "session", name: { en: "Max weapon range" }, description: { en: "Off in lobbies with players who don't run it." }, on: false },
  ],
  optionGroups: [
    { id: "idle", label: { en: "Idle crosshair" }, description: { en: "The crosshair while not aiming." }, collapsed: false },
    { id: "aim", label: { en: "Aiming" }, description: {}, collapsed: true },
  ],
  optionPreviews: [
    { id: "pic", group: "idle", svg: `<svg xmlns="http://www.w3.org/2000/svg"><circle r="{{size}}"/></svg>`, width: 120, height: 60, label: { en: "Preview" } },
  ],
  options: [
    option("loose", {}),
    option("size", { group: "idle" }),
    option("zoom", { group: "aim" }),
    option("hidden", { group: "idle", visible: false }),
  ],
  keybinds: [], files: [], import: null, source: null, updatable: false, image: null, folder: null, requires: [],
});

function show(m: ModRow, tab: "About" | "Options") {
  const box = document.createElement("div");
  document.body.append(box);
  root = createRoot(box);
  act(() => root!.render(<Drawer view={{ kind: "mod", id: m.id }} mods={[m]} conflicts={[]} order={[m.id]} names={new Map([[m.id, "Cam"]])}
    onOpen={() => {}} onClose={() => {}} updates={{}} onUpdate={() => {}} />));
  const tabs = [...box.querySelectorAll<HTMLButtonElement>(".dtabs button")];
  act(() => tabs.find((b) => b.getAttribute("aria-label") === tab)!.click());
  return box;
}

const names = (box: Element) => [...box.querySelectorAll(".opt-name")].map((n) => n.textContent);

describe("a mod with parts", () => {
  it("lists each part in About with the scope, the description and Off", () => {
    const box = show(cam(), "About");
    const parts = [...box.querySelectorAll(".parts .part")];
    expect(parts.map((p) => p.querySelector(".scope")!.className)).toEqual(["scope client", "scope session"]);
    expect(parts[0].querySelector(".part-name")!.textContent).toBe("Camera and view");
    expect(parts[0].querySelector(".part-about")!.textContent).toBe("Keeps working in any lobby.");
    expect(parts[1].classList.contains("off")).toBe(true);
    expect(parts[1].querySelector(".part-off")!.textContent).toBe("Off");
    expect(box.querySelector("aside")!.classList.contains("hybrid")).toBe(true);
  });

  it("names the parts and their scopes in the diamond's tooltip", () => {
    expect(partLines(cam())).toBe("Camera and view: Client\nMax weapon range: Session, off");
    expect(partLines({ ...cam(), parts: [] })).toBeNull();
  });
});

describe("option groups", () => {
  it("list the options with no group first, then each group, folded as the manifest says", () => {
    const box = show(cam(), "Options");
    const heads = [...box.querySelectorAll<HTMLButtonElement>(".opt-group-head")];
    expect(heads.map((h) => [h.textContent, h.getAttribute("aria-expanded")])).toEqual([["Idle crosshair", "true"], ["Aiming", "false"]]);
    expect(heads[0].dataset.tip).toBe("The crosshair while not aiming.");
    expect(names(box)).toEqual(["loose", "size"]);
  });

  it("open and close on a click, and remember it per mod", () => {
    let box = show(cam(), "Options");
    act(() => box.querySelectorAll<HTMLButtonElement>(".opt-group-head")[1].click());
    act(() => box.querySelectorAll<HTMLButtonElement>(".opt-group-head")[0].click());
    expect(names(box)).toEqual(["loose", "zoom"]);
    act(() => root!.unmount());
    root = undefined;
    box = show(cam(), "Options");
    expect(names(box), "the next time the details open").toEqual(["loose", "zoom"]);
    expect(JSON.parse(localStorage.getItem("obsidian.optionGroups")!)).toEqual({ "a.cam": { idle: true, aim: false } });
  });
});

describe("option previews", () => {
  const radius = (box: Element) => {
    const src = decodeURIComponent(box.querySelector<HTMLImageElement>(".opt-preview img")!.src.split(",")[1]);
    return new DOMParser().parseFromString(src, "image/svg+xml").querySelector("circle")!.getAttribute("r");
  };

  it("show at the top of the group with the values", () => {
    const box = show(cam(), "Options");
    const group = box.querySelector(".opt-group.open")!;
    expect(group.querySelector(".opt-group-body")!.firstElementChild!.classList.contains("opt-preview")).toBe(true);
    expect(group.querySelector("figcaption")!.textContent).toBe("Preview");
    expect(radius(box)).toBe("4");
  });

  it("follow a value while it is being changed, before it is saved", () => {
    const box = show(cam(), "Options");
    const slider = box.querySelector<HTMLInputElement>(".opt-group input[type=range]")!;
    const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    act(() => {
      set.call(slider, "7");
      slider.dispatchEvent(new Event("input", { bubbles: true }));
    });
    expect(radius(box)).toBe("7");
  });
});
