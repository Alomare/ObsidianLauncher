import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { t } from "../i18n";
import { copyText, pastedMods, pastedPaths } from "../mods/clipboard";
import { checkable, checkAll, deletable, inFolder, zipNames } from "../saves/bulk";
import type { SaveList, SaveRow } from "../saves/types";
import { listKeysFree, typingIn, useListKeys, type ListKeys } from "../shortcuts";
import type { ModRow, Snapshot } from "../types";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke, convertFileSrc: (p: string) => p }));
vi.mock("../opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));
vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: vi.fn() }));
const { ModsPage } = await import("../mods/ModsPage");
const { SavesPage } = await import("../saves/SavesPage");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | undefined;
afterEach(() => {
  act(() => root?.unmount());
  root = undefined;
  document.body.replaceChildren();
  invoke.mockReset();
});
const mount = (el: React.ReactElement) => {
  const host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  act(() => root!.render(el));
  return host;
};
const ctrlA = (target: EventTarget = document.body) =>
  act(() => { target.dispatchEvent(new KeyboardEvent("keydown", { key: "a", ctrlKey: true, bubbles: true, cancelable: true })); });
/** A copy or paste event with a clipboard, which jsdom leaves out. */
function clip(type: "copy" | "paste", text = "", target: EventTarget = document.body) {
  const data = new Map<string, string>([["text/plain", text]]);
  const e = new Event(type, { bubbles: true, cancelable: true });
  Object.defineProperty(e, "clipboardData", { value: { getData: (k: string) => data.get(k) ?? "", setData: (k: string, v: string) => data.set(k, v) } });
  act(() => { target.dispatchEvent(e); });
  return { prevented: e.defaultPrevented, text: data.get("text/plain") };
}

describe("where the list shortcuts may act", () => {
  it("not in a box that takes text, nor while a dialog or a menu is open", () => {
    const input = document.createElement("input");
    const area = document.createElement("textarea");
    const edit = document.createElement("div");
    edit.setAttribute("contenteditable", "true");
    const inner = document.createElement("span");
    edit.append(inner);
    document.body.append(input, area, edit);
    for (const el of [input, area, edit, inner]) expect(typingIn(el)).toBe(true);
    expect(listKeysFree(document.body)).toBe(true);
    expect(listKeysFree(input)).toBe(false);
    const dialog = document.createElement("div");
    dialog.setAttribute("role", "dialog");
    document.body.append(dialog);
    expect(listKeysFree(document.body)).toBe(false);
    dialog.setAttribute("role", "menu");
    expect(listKeysFree(document.body)).toBe(false);
  });

  function Probe({ keys }: { keys: ListKeys }) {
    useListKeys(true, keys);
    return <input aria-label="box" />;
  }
  it("Ctrl+A, Ctrl+C and Ctrl+V reach the list, but never from an input", () => {
    const selectAll = vi.fn();
    const paste = vi.fn();
    const host = mount(<Probe keys={{ selectAll, copy: () => "a 1.0", paste }} />);
    const box = host.querySelector("input")!;
    ctrlA(box);
    expect(selectAll).not.toHaveBeenCalled();
    expect(clip("copy", "", box).prevented).toBe(false);
    clip("paste", "x", box);
    expect(paste).not.toHaveBeenCalled();

    ctrlA();
    expect(selectAll).toHaveBeenCalledTimes(1);
    expect(clip("copy").text).toBe("a 1.0");
    clip("paste", "x");
    expect(paste).toHaveBeenCalledWith("x");
  });

  it("does nothing while a dialog is open", () => {
    const selectAll = vi.fn();
    mount(<Probe keys={{ selectAll }} />);
    const dialog = document.createElement("div");
    dialog.setAttribute("role", "alertdialog");
    document.body.append(dialog);
    ctrlA();
    expect(selectAll).not.toHaveBeenCalled();
  });
});

describe("copying and pasting mods", () => {
  const mods = [{ id: "a.one", name: "One", version: "1.0.0" }, { id: "b.two", name: "Two Words", version: null }, { id: "c.three", name: "Three", version: "2.1" }];
  it("copies a name and a version a line", () => expect(copyText(mods)).toBe("One 1.0.0\nTwo Words\nThree 2.1"));
  it("a pasted copy checks the same mods, by name or id, whatever the case or version", () => {
    expect(pastedMods(copyText(mods), mods)).toEqual(["a.one", "b.two", "c.three"]);
    expect(pastedMods("three 9.9\r\nA.ONE\n\nnobody", mods)).toEqual(["a.one", "c.three"]);
    expect(pastedMods("Two", mods)).toEqual([]);
  });
  it("only a paste of paths imports", () => {
    expect(pastedPaths("C:\\Mods\\a.zip\n\"D:\\b c.zip\"\n/home/x/m")).toEqual(["C:\\Mods\\a.zip", "D:\\b c.zip", "/home/x/m"]);
    expect(pastedPaths("C:\\Mods\\a.zip\nOne 1.0.0")).toBeNull();
    expect(pastedPaths("https://github.com/a/b")).toBeNull();
  });
});

const mod = (id: string, name = id): ModRow => ({
  id, name: { en: name }, description: {}, authors: [], version: "1.0.0", scope: "client", on: true, options: [], keybinds: [], files: [],
  import: null, source: null, updatable: false, image: null, folder: null, requires: [], parts: [], optionGroups: [], optionPreviews: [], size: 0, deployed: false, deployedSize: null,
});
const snap = (mods: ModRow[]): Snapshot => ({
  mods, folders: [], profile: "Default", profiles: ["Default"], profileRows: [{ name: "Default", on: 0, blockPublic: false, guarded: false, image: null, color: null }],
  blockPublic: false, blockAchievements: false, freezeProgress: false, hideClientMods: false, loader: true, sandbox: null, conflicts: [], changes: "UpToDate", deployed: true, modded: true, game: "supported",
  gameChanges: [], gameDir: "C:\\Game", crashCulprit: null, broken: [], damagedProfiles: [], uncleanEnd: null, portable: false, version: "1.3.1",
});
const count = () => document.querySelector(".batch-n")?.textContent ?? null;

describe("the Mods list shortcuts", () => {
  const show = () => mount(<ModsPage snap={snap([mod("a.alpha", "Alpha"), mod("b.beta", "Beta"), mod("c.alpine", "Alpine")])} busy={false} onPurge={() => {}} onDeploy={() => {}} />);
  const search = (v: string) => {
    const input = document.querySelector<HTMLInputElement>(".search input")!;
    const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    act(() => { set.call(input, v); input.dispatchEvent(new Event("input", { bubbles: true })); });
    return input;
  };

  it("Ctrl+A checks the mods the search shows, and a second Ctrl+A unchecks them", () => {
    show();
    const input = search("alp");
    ctrlA(input);
    expect(count()).toBeNull();
    input.blur();
    ctrlA();
    expect(count()).toBe(t("app.mods.nChecked", 2));
    ctrlA();
    expect(count()).toBeNull();
    search("");
    ctrlA();
    expect(count()).toBe(t("app.mods.nChecked", 3));
  });

  it("Ctrl+C copies the checked mods, Ctrl+V checks the mods a list names", () => {
    show();
    clip("paste", "Beta 1.0.0\nalpine");
    expect(count()).toBe(t("app.mods.nChecked", 2));
    expect(clip("copy").text).toBe("Beta 1.0.0\nAlpine 1.0.0");
  });

  it("the switch of a checked mod switches every checked mod", async () => {
    invoke.mockResolvedValue(snap([mod("a.alpha", "Alpha"), mod("b.beta", "Beta"), mod("c.alpine", "Alpine")]));
    show();
    clip("paste", "Alpha\nAlpine");
    const sw = [...document.querySelectorAll<HTMLButtonElement>("button[role='switch']")];
    await act(async () => sw[0].click());
    expect(invoke).toHaveBeenCalledWith("set_enabled", { ids: ["a.alpha", "c.alpine"], on: false });
  });
});

const save = (id: string, kind: "official" | "sandbox", extra: Partial<SaveRow> = {}): SaveRow => ({
  id, name: id, kind, created: "", lastPlayed: null, snapshotOf: null, active: false, live: false, stats: null, error: null, ...extra,
});

describe("checked saves", () => {
  const saves = [save("Main", "official", { active: true, live: true }), save("Old", "official"), save("Box A", "sandbox"), save("Box B", "sandbox")];
  it("only Sandbox saves take a check, and none while the group is folded", () => {
    expect(checkable(saves, false)).toEqual(["Box A", "Box B"]);
    expect(checkable(saves, true)).toEqual([]);
  });
  it("check all checks, then unchecks", () => {
    const once = checkAll(["a", "b"], new Set(["a"]));
    expect([...once]).toEqual(["a", "b"]);
    expect([...checkAll(["a", "b"], once)]).toEqual([]);
  });
  it("a bulk delete never takes an Official save or the active one", () => {
    expect(deletable([saves[0], saves[1], saves[2], save("Box C", "sandbox", { active: true })]).map((r) => r.id)).toEqual(["Box A"]);
  });
  it("names one zip per save in one folder", () => {
    expect(zipNames([save("a", "sandbox", { name: "War" }), save("b", "sandbox", { name: "war" }), save("c", "sandbox", { name: "A:B" })])).toEqual(["War.zip", "war (2).zip", "A B.zip"]);
    expect(inFolder("C:\\Exports\\", "War.zip")).toBe("C:\\Exports\\War.zip");
    expect(inFolder("/home/x", "War.zip")).toBe("/home/x/War.zip");
  });
});

describe("the Saves tab", () => {
  const list: SaveList = { account: "1", active: "Main", saves: [save("Main", "official", { active: true, live: true }), save("Old", "official"), save("Box A", "sandbox"), save("Box B", "sandbox")] };
  async function show() {
    invoke.mockImplementation(async (cmd: string) => (cmd === "saves_list" ? list : null));
    const host = mount(<SavesPage snap={snap([])} visible />);
    await act(async () => {});
    return host;
  }

  it("Sandbox rows have check boxes, Official rows none; Ctrl+A checks the Sandbox saves", async () => {
    const host = await show();
    const rows = [...host.querySelectorAll(".str.in")];
    expect(rows.map((r) => !!r.querySelector("[role='checkbox']"))).toEqual([false, false, true, true]);
    ctrlA();
    expect(count()).toBe(t("app.mods.nChecked", 2));
    const tools = [...host.querySelectorAll<HTMLButtonElement>(".batch button")].map((b) => b.getAttribute("aria-label"));
    expect(tools).toEqual([t("app.saves.exportMany"), t("app.saves.deleteMany"), t("app.mods.uncheck")]);
    ctrlA();
    expect(count()).toBeNull();
  });

  it("Delete stays off for the Official save in the game files", async () => {
    const host = await show();
    const del = host.querySelector<HTMLButtonElement>(`.save-tools button[aria-label='${t("app.saves.delete")}']`)!;
    expect(del.disabled).toBe(true);
    expect(del.getAttribute("data-tip")).toBe(t("app.saves.deleteLiveOff"));
  });
});
