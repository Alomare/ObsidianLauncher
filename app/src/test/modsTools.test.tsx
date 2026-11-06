import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { t } from "../i18n";
import type { ModRow, Snapshot } from "../types";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke, convertFileSrc: (p: string) => p }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));
vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: vi.fn() }));
const { ModsPage } = await import("../mods/ModsPage");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mod = (id: string): ModRow => ({
  id, name: { en: id }, description: {}, authors: [], version: "1.0.0", scope: "client", on: true, options: [], keybinds: [], files: [],
  import: null, source: null, updatable: false, image: null, customImage: false, folder: null, requires: [],
});
const snap = (ids: string[]): Snapshot => ({
  mods: ids.map(mod), folders: [], profile: "Default", profiles: ["Default"],
  profileRows: [{ name: "Default", on: ids.length, blockPublic: false, guarded: false, image: null, color: null }],
  blockPublic: false, blockAchievements: false, freezeProgress: false, conflicts: [], changes: "UpToDate", deployed: true, game: "supported",
  gameChanges: [], gameDir: "C:\\Game", crashCulprit: null, broken: [], damagedProfiles: [], uncleanEnd: null, portable: false, version: "1.3.1",
});

let root: Root | undefined;
afterEach(() => {
  act(() => root?.unmount());
  document.body.replaceChildren();
  invoke.mockReset();
});

/** The parts of the strip at the head of the Mods page. */
function show(s: Snapshot) {
  const host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  act(() => root!.render(<ModsPage snap={s} />));
  return [...host.querySelectorAll<HTMLButtonElement>(".head .tools > button")];
}

describe("the Mods tools", () => {
  it("hold the profile, Create Folder, Add mods and Sort Mods in one strip, each with a name and what it opens", () => {
    const parts = show(snap(["a.one", "b.two"]));
    expect(parts.map((b) => b.getAttribute("aria-label"))).toEqual([
      `${t("app.mods.profile")}: Default`, t("app.mods.createFolder"), t("app.mods.add"), t("app.sort.title"),
    ]);
    expect(parts.map((b) => b.getAttribute("aria-haspopup"))).toEqual(["dialog", "dialog", "menu", "dialog"]);
    expect(parts[0].textContent).toBe("Default");
    // Nothing in it is a plain .btn: the strip draws them, so no button rule adds an edge or a fold of its own.
    expect(parts.some((b) => b.classList.contains("btn"))).toBe(false);
  });

  it("leave Sort Mods out while there is one mod to order", () => {
    expect(show(snap(["a.one"])).map((b) => b.getAttribute("aria-label"))).not.toContain(t("app.sort.title"));
  });

  it("Add mods opens the import menu and says it is open", () => {
    const add = show(snap(["a.one", "b.two"]))[2];
    expect(add.getAttribute("aria-expanded")).toBe("false");
    act(() => add.click());
    expect(add.getAttribute("aria-expanded")).toBe("true");
    expect([...document.querySelectorAll(".ctx [role=menuitem]")].map((b) => b.textContent)).toEqual([t("app.mods.importZip"), t("app.mods.importFolder")]);
  });

  it("Create Folder opens the new folder dialog", () => {
    const folder = show(snap(["a.one", "b.two"]))[1];
    act(() => folder.click());
    expect(document.querySelector(".modal.confirm input")).not.toBeNull();
  });

  it("Sort Mods asks for the preview and shows it", async () => {
    invoke.mockResolvedValue({ order: ["a.one", "b.two"], moves: [], conflicts: [], notes: [] });
    const sort = show(snap(["a.one", "b.two"]))[3];
    await act(async () => sort.click());
    expect(invoke).toHaveBeenCalledWith("sort_preview", undefined);
    expect(document.querySelector(".modal.sort")).not.toBeNull();
  });
});
