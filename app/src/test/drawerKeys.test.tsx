import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ModRow, OptionRow } from "../types";

const run = vi.fn();
vi.mock("../api", () => ({ run: (...a: unknown[]) => run(...a) }));
vi.mock("../opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
// The controller recording, played by the test: `recorded` gives the buttons let go.
let recorded: ((b: string[]) => void) | null = null;
vi.mock("../pad/pad", async (load) => ({
  ...(await load<typeof import("../pad/pad")>()),
  recordPad: (done: (b: string[]) => void) => { recorded = done; return () => { if (recorded === done) recorded = null; }; },
}));
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
  choiceLabels: [{ en: "Both sticks pressed in (L3 + R3)" }, { en: "Off" }], visible: true, deploy: false, group: null, part: null, ...extra,
});

const api = (padValue = "thumbs") =>
  ({
    id: "obsidian.api", name: { en: "Obsidian API" }, description: { en: "" }, authors: [], version: "1.36.0", scope: "client", on: true,
    options: [choice("pad_open", padValue), { ...choice("other", "off"), label: { en: "Another choice" } }],
    keybinds: [
      { id: "open", label: { en: "Open in-game menu" }, default: "f6", key: null, clash: [], mode: "press", pad: "pad_open", pad_bind: null },
      { id: "shot", label: { en: "Take a screenshot" }, default: "f7", key: "f9", clash: [], mode: "long_press", pad: null, pad_bind: null },
    ],
    files: [], import: null, source: null, updatable: false, image: null, folder: null, requires: [], parts: [], optionGroups: [], optionPreviews: [], size: 0, deployed: false, deployedSize: null,
  }) as ModRow;

function show(m: ModRow, tab: "keys" | "options") {
  const box = document.createElement("div");
  document.body.append(box);
  root = createRoot(box);
  act(() => root!.render(<Drawer view={{ kind: "mod", id: m.id }} mods={[m]} conflicts={[]} order={[m.id]} names={new Map([[m.id, "Obsidian API"]])}
    onOpen={() => {}} onClose={() => {}} />));
  const tabs = [...box.querySelectorAll<HTMLButtonElement>(".dtabs button")];
  act(() => tabs.find((b) => b.getAttribute("aria-label") === (tab === "keys" ? "Keys" : "Options"))!.click());
  return box;
}

describe("the Keys tab", () => {
  it("heads the names with Change Binds and the two input columns with an icon each", () => {
    const head = show(api(), "keys").querySelector(".keys-head")!;
    expect(head.querySelector(".keys-change button")!.textContent).toBe("Change binds");
    const cols = [...head.querySelectorAll("[role=img]")].map((s) => s.getAttribute("aria-label"));
    expect(cols).toEqual(["Mouse and keyboard", "Controller"]);
    expect(head.children).toHaveLength(3);
  });

  it("shows each action in three cells to read: the name, the key, the controller input", () => {
    const box = show(api(), "keys");
    const [open, shot] = [...box.querySelectorAll(".keys .opt")];
    expect([...open.children].map((c) => c.className)).toEqual(["opt-text", "opt-ctl key-col", "opt-ctl pad-col"]);
    expect(box.querySelectorAll(".keys .opts button, .keys .opts select"), "nothing in the rows changes a bind").toHaveLength(0);
    expect(open.querySelector(".l")!.textContent).toBe("Open in-game menu");
    expect(open.querySelector(".key-col .keycap")!.textContent).toBe("f6");
    expect(shot.querySelector(".key-col .keycap")!.textContent).toBe("f9");
    const pad = open.querySelector<HTMLElement>(".pad-col .pad-view")!;
    expect(pad.querySelectorAll(".pad-glyph")).toHaveLength(2);
    expect(pad.dataset.tip).toBe("Both sticks pressed in (L3 + R3)");
    expect(shot.querySelector<HTMLElement>(".pad-col .none")!.dataset.tip).toBe("No controller input for this action");
  });

  it("shows a controller input set by the player over the mod's own, and a mouse button in words", () => {
    const m = api();
    m.keybinds[0] = { ...m.keybinds[0], pad_bind: "left_shoulder+d_up" };
    m.keybinds[1] = { ...m.keybinds[1], key: "mouse_middle", pad_bind: "y" };
    const [open, shot] = [...show(m, "keys").querySelectorAll(".keys .opt")];
    expect(open.querySelector<HTMLElement>(".pad-view")!.dataset.tip).toBe("LB + D-pad Up");
    expect(shot.querySelector<HTMLElement>(".pad-view")!.dataset.tip).toBe("Y");
    expect(shot.querySelector(".keycap")!.textContent).toBe("Middle mouse");
  });

  it("names how a bind fires under the name, when not on press", () => {
    const [open, shot] = [...show(api(), "keys").querySelectorAll(".keys .opt")];
    expect(open.querySelector(".d")).toBeNull();
    expect(shot.querySelector(".d")!.textContent).toBe("Long press");
  });
});

describe("the binds dialog", () => {
  const open = () => {
    const box = show(api("off"), "keys");
    act(() => box.querySelector<HTMLButtonElement>(".keys-change button")!.click());
    return document.querySelector<HTMLElement>(".modal.binds")!;
  };

  it("lists every action with its key, controller input and input mode", () => {
    const dialog = open();
    expect(dialog.querySelector("h2")!.textContent).toBe("Binds: Obsidian API");
    const rows = [...dialog.querySelectorAll(".binds-row:not(.binds-head)")];
    expect(rows.map((r) => r.querySelector(".binds-name span")!.textContent)).toEqual(["Open in-game menu", "Take a screenshot"]);
    expect(rows[0].querySelector(".key-btn")!.textContent).toBe("f6");
    expect(rows[1].querySelector(".key-btn")!.textContent).toBe("f9");
    expect(rows[1].querySelector(".pad-btn-in")!.textContent).toBe("None");
    const modes = rows.map((r) => r.querySelector(".select-btn")!.textContent);
    expect(modes).toEqual(["Press", "Long press"]);
  });

  it("records any controller buttons for any action, and resets to the mod's own", () => {
    const rows = [...open().querySelectorAll(".binds-row:not(.binds-head)")];
    const pad = rows[1].querySelector<HTMLButtonElement>(".pad-btn-in")!;
    act(() => pad.click());
    expect(pad.textContent).toBe("Hold buttons...");
    act(() => recorded!(["left_shoulder", "d_up"]));
    expect(run).toHaveBeenCalledWith("set_pad", { id: "obsidian.api", keybind: "shot", combo: "left_shoulder+d_up" });
    expect(pad.textContent).not.toBe("Hold buttons...");
    const resets = rows[0].querySelectorAll<HTMLButtonElement>("button.bare");
    expect(resets[0].disabled, "the key is the default").toBe(true);
    expect(resets[1].disabled, "no controller input set by the player").toBe(true);
  });

  it("takes a mouse button other than the left one as the key", () => {
    const key = open().querySelector<HTMLButtonElement>(".binds-row:not(.binds-head) .key-btn")!;
    act(() => key.click());
    act(() => { key.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, button: 0 })); });
    expect(run).not.toHaveBeenCalled();
    act(() => { key.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, button: 1 })); });
    expect(run).toHaveBeenCalledWith("set_key", { id: "obsidian.api", keybind: "open", key: "mouse_middle" });
  });

  it("offers the five input modes, and press goes back to the default", () => {
    const rows = [...open().querySelectorAll(".binds-row:not(.binds-head)")];
    const pick = (btn: HTMLButtonElement, label: string) => {
      act(() => btn.click());
      const items = [...document.querySelectorAll<HTMLLIElement>(".select-list li")];
      expect(items.map((i) => i.textContent)).toEqual(["Tap", "Double tap", "Press", "Long press", "Release"]);
      act(() => items.find((i) => i.textContent === label)!.click());
    };
    pick(rows[0].querySelector<HTMLButtonElement>(".select-btn")!, "Double tap");
    expect(run).toHaveBeenCalledWith("set_mode", { id: "obsidian.api", keybind: "open", mode: "double_tap" });
    pick(rows[1].querySelector<HTMLButtonElement>(".select-btn")!, "Press");
    expect(run).toHaveBeenCalledWith("set_mode", { id: "obsidian.api", keybind: "shot", mode: null });
  });
});

describe("the Options tab", () => {
  it("marks an option that applies after Deploy with an asterisk and puts the note in the tooltip, under the description", () => {
    const m = api();
    m.options = [choice("power", "off", { deploy: true, label: { en: "Power" } }), choice("calm", "off", { label: { en: "Calm" }, description: { en: "" } })];
    m.keybinds = [];
    const names = [...show(m, "options").querySelectorAll<HTMLElement>(".opt-name")];
    expect(names[0].querySelector(".opt-star")?.textContent).toBe("*");
    expect(names[0].dataset.tip).toBe("The buttons that open the menu.");
    expect(names[0].dataset.tipNote).toBe("Applies after Deploy.");
    expect(names[0].tabIndex).toBe(0);
    expect(names[1].querySelector(".opt-star")).toBeNull();
    expect(names[1].dataset.tipNote).toBeUndefined();
    expect(document.body.textContent).not.toContain("Applies after Deploy.");
  });

  it("leaves out an option a keybind shows as its controller input, and counts the rest", () => {
    const box = show(api(), "options");
    expect([...box.querySelectorAll(".opts .l")].map((l) => l.textContent)).toEqual(["Another choice"]);
    const count = [...box.querySelectorAll(".dtabs button")].find((b) => b.getAttribute("aria-label") === "Options")!.querySelector(".count");
    expect(count!.textContent).toBe("1");
  });
});
