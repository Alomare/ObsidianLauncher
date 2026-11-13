import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { t } from "../i18n";
import type { ModRow, ProfileRow, Snapshot } from "../types";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke, convertFileSrc: (p: string) => p }));
vi.mock("../opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));
vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: vi.fn() }));
const { ModsPage } = await import("../mods/ModsPage");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/** A mod; one whose id starts with "gh." names a GitHub repository and can be checked for updates. */
const mod = (id: string): ModRow => ({
  id, name: { en: id }, description: {}, authors: [], version: "1.0.0", scope: "client", on: true, options: [], keybinds: [], files: [],
  import: null, source: null, updatable: id.startsWith("gh."), image: null, folder: null, requires: [], parts: [], optionGroups: [], optionPreviews: [], size: 0, deployed: false, deployedSize: null,
});
const row = (name: string, extra: Partial<ProfileRow> = {}): ProfileRow => ({ name, on: 0, blockPublic: false, guarded: false, image: null, color: null, ...extra });
const snap = (ids: string[], profileRows: ProfileRow[] = [row("Default")]): Snapshot => ({
  mods: ids.map(mod), folders: [], profile: "Default", profiles: profileRows.map((p) => p.name), profileRows,
  blockPublic: false, blockAchievements: false, freezeProgress: false, hideClientMods: false, loader: false, sandbox: null, conflicts: [], changes: "UpToDate", deployed: true, modded: true, game: "supported",
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
  act(() => root!.render(<ModsPage snap={s} busy={false} onPurge={() => {}} onDeploy={() => {}} />));
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

  it("with one profile, shows the chip and the name with no arrow, and opens the profiles dialog", () => {
    invoke.mockResolvedValue({ account: "1", active: null, saves: [] });
    const profile = show(snap(["a.one", "b.two"]))[0];
    expect(profile.classList.contains("profile-btn")).toBe(true);
    expect(profile.querySelector(".profile-chip")).not.toBeNull();
    expect(profile.textContent).toBe("Default");
    expect(profile.querySelectorAll(":scope > svg")).toHaveLength(0);
    expect(profile.getAttribute("aria-haspopup")).toBe("dialog");
    act(() => profile.click());
    expect(document.querySelector(".modal.profiles")).not.toBeNull();
    expect(document.querySelector(".ctx")).toBeNull();
  });

  describe("with other profiles", () => {
    const two = () => snap(["a.one", "b.two"], [row("Default", { color: "#123456" }), row("Fun", { blockPublic: true, guarded: true })]);
    const items = () => [...document.querySelectorAll<HTMLButtonElement>(".ctx > button")];

    it("shows an arrow and opens a menu: Manage Profiles, then every profile with its color and guardrails, the one in use checked", () => {
      const profile = show(two())[0];
      expect(profile.querySelectorAll(":scope > svg")).toHaveLength(1);
      expect(profile.getAttribute("aria-haspopup")).toBe("menu");
      expect(profile.getAttribute("aria-expanded")).toBe("false");
      act(() => profile.click());
      expect(profile.getAttribute("aria-expanded")).toBe("true");
      expect(document.querySelector(".ctx")!.getAttribute("aria-label")).toBe(t("app.profiles.title"));
      expect(items().map((b) => b.querySelector(".ctx-label")!.textContent)).toEqual([t("app.profiles.manage"), "Default", "Fun"]);
      expect(items().map((b) => b.getAttribute("role"))).toEqual(["menuitem", "menuitemradio", "menuitemradio"]);
      expect(items().map((b) => b.getAttribute("aria-checked"))).toEqual([null, "true", "false"]);
      expect(items()[1].querySelector<HTMLElement>(".ctx-swatch")!.style.background).toBe("rgb(18, 52, 86)");
      expect(items().slice(1).map((b) => b.querySelectorAll(".ctx-guards .consent-ico.on").length)).toEqual([0, 2]);
      expect(items()[2].querySelectorAll(".ctx-guards [role=img]")).toHaveLength(2);
    });

    it("switches to a profile on a click, does nothing for the one in use, and Manage Profiles opens the dialog", async () => {
      invoke.mockResolvedValue(two());
      const profile = show(two())[0];
      act(() => profile.click());
      await act(async () => items()[2].click());
      expect(invoke).toHaveBeenCalledWith("use_profile", { name: "Fun" });
      expect(document.querySelector(".ctx")).toBeNull();
      invoke.mockClear();
      act(() => profile.click());
      act(() => items()[1].click());
      expect(invoke).not.toHaveBeenCalledWith("use_profile", expect.anything());
      act(() => profile.click());
      act(() => items()[0].click());
      expect(document.querySelector(".modal.profiles")).not.toBeNull();
    });

    it("works from the keys: Arrow Down opens it at Manage, arrows, Home and End move, Esc closes it and the focus goes back", () => {
      const profile = show(two())[0];
      profile.focus();
      act(() => profile.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true })));
      expect(document.activeElement).toBe(items()[0]);
      const key = (k: string) => act(() => document.activeElement!.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true })));
      key("ArrowDown");
      expect(document.activeElement).toBe(items()[1]);
      key("End");
      expect(document.activeElement).toBe(items()[2]);
      key("ArrowDown");
      expect(document.activeElement).toBe(items()[0]);
      key("Home");
      expect(document.activeElement).toBe(items()[0]);
      act(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })));
      expect(document.querySelector(".ctx")).toBeNull();
      expect(document.activeElement).toBe(profile);
      expect(profile.getAttribute("aria-expanded")).toBe("false");
    });

    it("closes on a second click of the profile", () => {
      const profile = show(two())[0];
      act(() => profile.click());
      act(() => profile.click());
      expect(document.querySelector(".ctx")).toBeNull();
    });
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

  it("keeps the Version and Scope columns at fixed widths, with no line to drag, and drops widths an older build saved", () => {
    localStorage.setItem("obsidian.modColumns", JSON.stringify({ version: 20, scope: 9 }));
    show(snap(["a.one", "b.two"]));
    expect(localStorage.getItem("obsidian.modColumns")).toBeNull();
    expect(document.querySelector<HTMLElement>(".table")!.style.getPropertyValue("--col-ver")).toBe("");
    expect(document.querySelectorAll(".th [role=separator], .col-grip")).toHaveLength(0);
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

describe("the conflicts panel", () => {
  const withConflict = (): Snapshot => ({ ...snap(["a.one", "b.two"]), conflicts: [{ resource: "x", mods: ["a.one", "b.two"], winner: "b.two" }] });
  const panel = () => document.querySelector(".drawer.conflicts");

  it("never opens by itself, opens from a conflict icon and closes on the X", () => {
    show(withConflict());
    expect(panel()).toBeNull();
    expect(document.querySelector(".split")).toBeNull();
    act(() => document.querySelector<HTMLButtonElement>(".cf-ico")!.click());
    expect(panel()).not.toBeNull();
    act(() => panel()!.querySelector<HTMLButtonElement>(".dh button.bare")!.click());
    expect(panel()).toBeNull();
  });
});

describe("the list and the bar for checked mods", () => {
  const tick = (id: string) => act(() => document.querySelector<HTMLButtonElement>(`[data-key="${id}"] .ck`)!.click());
  const bar = () => document.querySelector<HTMLElement>(".batch[role=toolbar]");
  const checkButton = () => [...bar()!.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.getAttribute("aria-label") === t("app.mods.checkUpdates"))!;

  it("has no column of update checks: not in the header, the rows or a folder", () => {
    show(snap(["gh.one", "b.two"]));
    expect(document.querySelectorAll(".up-ck")).toHaveLength(0);
    // Check, #, On, Mod, the conflict icon, Size, Deployed, Version and Scope.
    expect(document.querySelector(".th")!.children).toHaveLength(9);
  });

  it("holds the count, Turn on, Turn off, Check for Updates, Remove and Uncheck all, in that order, and no Actions", () => {
    show(snap(["gh.one", "b.two"]));
    tick("gh.one");
    tick("b.two");
    expect(bar()!.querySelector(".batch-n")!.textContent).toBe(t("app.mods.nChecked", 2));
    const names = [...bar()!.querySelectorAll("button")].map((b) => b.getAttribute("aria-label") ?? b.textContent);
    expect(names).toEqual([t("app.menu.enable"), t("app.menu.disable"), t("app.mods.checkUpdates"), t("app.menu.remove"), t("app.mods.uncheck")]);
    expect(bar()!.querySelectorAll(".batch-sep")).toHaveLength(2);
    expect(checkButton().querySelector("svg")).not.toBeNull();
  });

  it("checks only the checked mods that name a GitHub repository, and waits while the check runs", async () => {
    let answer: (v: unknown) => void = () => {};
    invoke.mockImplementation((cmd: string) => (cmd === "check_mod_updates" ? new Promise((r) => (answer = r)) : Promise.resolve(null)));
    show(snap(["gh.one", "b.two", "gh.three"]));
    tick("gh.one");
    tick("b.two");
    await act(async () => checkButton().click());
    expect(invoke).toHaveBeenCalledWith("check_mod_updates", { ids: ["gh.one"], fresh: true });
    expect(checkButton().disabled).toBe(true);
    expect(checkButton().classList.contains("busy")).toBe(true);
    expect(checkButton().getAttribute("aria-busy")).toBe("true");
    await act(async () => answer([{ id: "gh.one", version: null, error: null }]));
    expect(checkButton().disabled).toBe(false);
    expect(checkButton().classList.contains("busy")).toBe(false);
  });

  it("is off, and says why, when no checked mod names a GitHub repository", () => {
    show(snap(["gh.one", "b.two"]));
    tick("b.two");
    expect(checkButton().disabled).toBe(true);
    expect(checkButton().dataset.tip).toBe(t("app.mods.checkUpdatesNone"));
  });

  it("leaves the rest to the menu of a checked row: a right click there acts on every checked mod", () => {
    show(snap(["gh.one", "b.two", "c.three"]));
    tick("gh.one");
    tick("b.two");
    act(() => document.querySelector(`[data-key="b.two"]`)!.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 5, clientY: 5 })));
    const labels = [...document.querySelectorAll(".ctx > button .ctx-label")].map((l) => l.textContent);
    expect(labels).toContain(t("app.menu.moveToFolder"));
    expect(labels).toContain(t("app.menu.removeN", 2));
  });
});
