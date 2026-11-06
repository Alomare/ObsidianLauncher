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

/** The buttons at the head of the Mods page: the profile, then the tools. */
function show(s: Snapshot) {
  const host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  act(() => root!.render(<ModsPage snap={s} />));
  return [...host.querySelectorAll<HTMLButtonElement>(".mods .head > button")];
}

describe("the head of the Mods page", () => {
  it("has the profile, then Sort, Create Folder and Add, each named and saying what it opens", () => {
    const parts = show(snap(["a.one", "b.two"]));
    expect(parts.map((b) => b.getAttribute("aria-label"))).toEqual([
      `${t("app.mods.profile")}: Default`, t("app.sort.title"), t("app.mods.createFolder"), t("app.mods.add"),
    ]);
    expect(parts.map((b) => b.getAttribute("aria-haspopup"))).toEqual(["dialog", "dialog", "dialog", "menu"]);
  });

  it("shows the profile as a choice: the chip, the name and an arrow", () => {
    const profile = show(snap(["a.one", "b.two"]))[0];
    expect(profile.classList.contains("profile-btn")).toBe(true);
    expect(profile.querySelector(".profile-chip")).not.toBeNull();
    expect(profile.textContent).toBe("Default");
    expect(profile.querySelectorAll(":scope > svg")).toHaveLength(1);
  });

  it("shows Sort and Create Folder as icons with a tooltip, and Add with a word in the color of the main action", () => {
    const tools = show(snap(["a.one", "b.two"])).slice(1);
    expect(tools.every((b) => b.classList.contains("btn") && b.querySelector("svg"))).toBe(true);
    expect(tools.map((b) => b.querySelector(".label")?.textContent ?? null)).toEqual([null, null, t("app.mods.addButton")]);
    expect(tools.map((b) => b.dataset.tipTitle)).toEqual([t("app.sort.title"), t("app.mods.createFolder"), t("app.mods.add")]);
    expect(tools.map((b) => b.classList.contains("go"))).toEqual([false, false, true]);
    // The word a reader sees is part of the name a screen reader says.
    expect(tools[2].getAttribute("aria-label")).toContain(t("app.mods.addButton"));
  });

  it("leaves Sort out while there is one mod to order", () => {
    expect(show(snap(["a.one"])).map((b) => b.getAttribute("aria-label"))).not.toContain(t("app.sort.title"));
  });

  it("Add opens the import menu and says it is open", () => {
    const add = show(snap(["a.one", "b.two"]))[3];
    expect(add.getAttribute("aria-expanded")).toBe("false");
    act(() => add.click());
    expect(add.getAttribute("aria-expanded")).toBe("true");
    expect([...document.querySelectorAll(".ctx [role=menuitem]")].map((b) => b.textContent)).toEqual([t("app.mods.importZip"), t("app.mods.importFolder")]);
  });

  it("Create Folder opens the new folder dialog", () => {
    const folder = show(snap(["a.one", "b.two"]))[2];
    act(() => folder.click());
    expect(document.querySelector(".modal.confirm input")).not.toBeNull();
  });

  it("Sort asks for the preview and shows it", async () => {
    invoke.mockResolvedValue({ order: ["a.one", "b.two"], moves: [], conflicts: [], notes: [] });
    const sort = show(snap(["a.one", "b.two"]))[1];
    await act(async () => sort.click());
    expect(invoke).toHaveBeenCalledWith("sort_preview", undefined);
    expect(document.querySelector(".modal.sort")).not.toBeNull();
  });
});
