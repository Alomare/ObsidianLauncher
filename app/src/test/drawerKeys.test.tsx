import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ModRow, OptionRow } from "../types";

const run = vi.fn();
vi.mock("../api", () => ({ run: (...a: unknown[]) => run(...a) }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
const { Drawer } = await import("../mods/Drawer");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | undefined;
afterEach(() => {
  act(() => root?.unmount());
  document.body.replaceChildren();
  run.mockClear();
});

const choice = (id: string, value: string, extra: Partial<OptionRow> = {}): OptionRow => ({
  id, kind: "choice", label: { en: "Open the menu with a controller" }, description: { en: "The buttons that open the menu." },
  default: "thumbs", value, min: null, max: null, step: null, choices: ["thumbs", "off"],
  choiceLabels: [{ en: "Both sticks pressed in (L3 + R3)" }, { en: "Off" }], visible: true, deploy: false, ...extra,
});

const api = (padValue = "thumbs") =>
  ({
    id: "obsidian.api", name: { en: "Obsidian API" }, description: { en: "" }, authors: [], version: "1.36.0", scope: "client", on: true,
    options: [choice("pad_open", padValue), { ...choice("other", "off"), label: { en: "Another choice" } }],
    keybinds: [
      { id: "open", label: { en: "Open in-game overlay menu" }, default: "f6", key: null, clash: [], pad: "pad_open" },
      { id: "shot", label: { en: "Take a screenshot" }, default: "f7", key: "f9", clash: [], pad: null },
    ],
    files: [], import: null, source: null, updatable: false, image: null, customImage: false, folder: null, requires: [],
  }) as ModRow;

function show(m: ModRow, tab: "keys" | "options") {
  const box = document.createElement("div");
  document.body.append(box);
  root = createRoot(box);
  act(() => root!.render(<Drawer view={{ kind: "mod", id: m.id }} mods={[m]} conflicts={[]} order={[m.id]} names={new Map([[m.id, "Obsidian API"]])}
    onOpen={() => {}} onClose={() => {}} updates={{}} onUpdate={() => {}} />));
  const tabs = [...box.querySelectorAll<HTMLButtonElement>(".dtabs button")];
  act(() => tabs.find((b) => b.getAttribute("aria-label") === (tab === "keys" ? "Keys" : "Options"))!.click());
  return box;
}

describe("the Keys tab", () => {
  it("heads the two input columns with an icon each", () => {
    const head = show(api(), "keys").querySelector(".keys-head")!;
    const cols = [...head.querySelectorAll("[role=img]")].map((s) => s.getAttribute("aria-label"));
    expect(cols).toEqual(["Mouse and keyboard", "Controller"]);
    expect(head.querySelectorAll("svg")).toHaveLength(2);
  });

  it("shows the key and the controller input of one action in one row", () => {
    const rows = [...show(api(), "keys").querySelectorAll(".keys .opt")];
    expect(rows).toHaveLength(2);
    const [open, shot] = rows;
    expect(open.querySelector(".l")!.textContent).toBe("Open in-game overlay menu");
    expect(open.querySelector(".d")!.textContent).toBe("The buttons that open the menu.");
    expect(open.querySelector(".key-col .key-btn")!.textContent).toBe("f6");
    const pad = open.querySelector<HTMLButtonElement>(".pad-col .select-btn")!;
    expect(pad.getAttribute("aria-label")).toBe("Open the menu with a controller: Both sticks pressed in (L3 + R3)");
    expect(pad.dataset.tip).toBe("Both sticks pressed in (L3 + R3)");
    expect(shot.querySelector(".key-col .key-btn")!.textContent).toBe("f9");
    expect(shot.querySelector(".pad-col .select-btn")).toBeNull();
    expect(shot.querySelector<HTMLElement>(".pad-col .none")!.dataset.tip).toBe("No controller input for this action");
  });

  it("changes and resets the controller input as an option", () => {
    const open = show(api("off"), "keys").querySelector(".keys .opt")!;
    act(() => open.querySelector<HTMLButtonElement>(".pad-col .select-btn")!.click());
    const choices = [...document.querySelectorAll<HTMLLIElement>(".select-list li")];
    act(() => choices.find((c) => c.textContent === "Both sticks pressed in (L3 + R3)")!.click());
    expect(run).toHaveBeenCalledWith("set_option", { id: "obsidian.api", option: "pad_open", value: "thumbs" });
    const reset = open.querySelector<HTMLButtonElement>(".pad-col button.bare")!;
    expect(reset.disabled).toBe(false);
    act(() => reset.click());
    expect(run).toHaveBeenCalledWith("set_option", { id: "obsidian.api", option: "pad_open", value: null });
  });
});

describe("the Options tab", () => {
  it("leaves out an option a keybind shows as its controller input, and counts the rest", () => {
    const box = show(api(), "options");
    expect([...box.querySelectorAll(".opts .l")].map((l) => l.textContent)).toEqual(["Another choice"]);
    const count = [...box.querySelectorAll(".dtabs button")].find((b) => b.getAttribute("aria-label") === "Options")!.querySelector(".count");
    expect(count!.textContent).toBe("1");
  });
});
