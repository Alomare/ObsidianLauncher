// The columns at the right of the mod list: Size, Deployed, Version and Scope, the ring round the switch
// of a deployed mod, and the scope tooltip with the WIKI tag.
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { t } from "../i18n";
import type { ModRow, Snapshot } from "../types";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke, convertFileSrc: (p: string) => p }));
vi.mock("../opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));
vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: vi.fn() }));
const { ModsPage } = await import("../mods/ModsPage");
const { formatSize } = await import("../mods/ModTable");
const { Tooltip } = await import("../Tooltip");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const MB = 1024 * 1024;
const mod = (id: string, extra: Partial<ModRow>): ModRow => ({
  id, name: { en: id }, description: {}, authors: [], version: "1.0.0", scope: "client", on: true, options: [], keybinds: [], files: [],
  import: null, source: null, updatable: false, image: null, folder: null, requires: [], parts: [], optionGroups: [], optionPreviews: [],
  size: 0, deployed: false, deployedSize: null, ...extra,
});
const snap = (mods: ModRow[]): Snapshot => ({
  mods, folders: [], profile: "Default", profiles: ["Default"], profileRows: [{ name: "Default", on: 0, blockPublic: false, guarded: false, image: null, color: null }],
  blockPublic: false, blockAchievements: false, freezeProgress: false, hideClientMods: false, loader: false, sandbox: null, conflicts: [], changes: "UpToDate", deployed: true, modded: true, game: "supported",
  gameChanges: [], gameDir: "C:\\Game", crashCulprit: null, broken: [], damagedProfiles: [], uncleanEnd: null, portable: false, version: "2.0.0",
});

let root: Root | undefined;
afterEach(() => {
  act(() => root?.unmount());
  document.body.replaceChildren();
  vi.useRealTimers();
});

function show(mods: ModRow[]) {
  const host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  act(() => root!.render(<><ModsPage snap={snap(mods)} busy={false} onPurge={() => {}} onDeploy={() => {}} /><Tooltip /></>));
  return (id: string) => document.querySelector<HTMLElement>(`.tr[data-key="${id}"]`)!;
}

describe("sizes", () => {
  it("read in MB, and in GB from 1000 MB", () => {
    expect(formatSize(0)).toBe(t("app.mods.sizeMb", "0.0"));
    expect(formatSize(1000)).toBe(t("app.mods.sizeMb", "0.1"));
    expect(formatSize(12.34 * MB)).toBe(t("app.mods.sizeMb", "12.3"));
    expect(formatSize(412 * MB)).toBe(t("app.mods.sizeMb", "412"));
    expect(formatSize(1.5 * 1024 * MB)).toBe(t("app.mods.sizeGb", "1.50"));
  });
});

describe("the columns at the right of the mod list", () => {
  it("come after the mod in the order Size, Deployed, Version, Scope", () => {
    show([mod("a.one", {})]);
    const head = [...document.querySelectorAll(".th > *")].map((c) => c.textContent);
    expect(head.slice(-4)).toEqual([t("app.mods.colSize"), t("app.mods.colDeployed"), t("app.mods.colVersion"), t("app.mods.colScope")]);
  });

  it("show the folder size, and what the last deploy put in the game or a grey cross", () => {
    const row = show([
      mod("a.in", { size: 20 * MB, deployed: true, deployedSize: 3 * MB }),
      mod("b.out", { size: 5 * MB }),
      mod("c.old", { size: MB, deployed: true, deployedSize: null, on: false }),
    ]);
    expect(row("a.in").querySelector(".sz")!.textContent).toBe(formatSize(20 * MB));
    expect(row("a.in").querySelector(".dep-tag")!.textContent).toBe(formatSize(3 * MB));
    expect(row("b.out").querySelector(".dep-tag")).toBeNull();
    expect(row("b.out").querySelector(".dep-no")!.getAttribute("aria-label")).toBe(t("app.mods.notDeployed"));
    // Deployed before Obsidian counted the bytes.
    expect(row("c.old").querySelector(".dep-tag")!.textContent).toBe(t("app.mods.deployedYes"));
  });

  it("ring the switch of a deployed mod, on or off", () => {
    const row = show([mod("a.in", { deployed: true }), mod("b.off", { deployed: true, on: false }), mod("c.out", {})]);
    expect(row("a.in").querySelector(".sw")!.className).toBe("sw on dep");
    expect(row("b.off").querySelector(".sw")!.className).toBe("sw dep");
    expect(row("c.out").querySelector(".sw")!.className).toBe("sw on");
  });

  it("give the scope tooltip only the name, what it means and a WIKI tag", () => {
    vi.useFakeTimers();
    const row = show([mod("a.cam", { scope: "hybrid" })]);
    const diamond = row("a.cam").querySelector<HTMLElement>(".scope")!;
    expect(diamond.dataset.tip).toBe(t("app.scope.hybridHint"));
    // jsdom has no PointerEvent.
    act(() => { diamond.dispatchEvent(new Event("pointerover", { bubbles: true })); });
    act(() => { vi.advanceTimersByTime(1000); });
    const tip = document.querySelector(".tip")!;
    expect(tip.querySelector(".tip-title")!.firstChild!.textContent).toBe(t("app.scope.hybrid"));
    expect(tip.querySelector(".tip-wiki")!.textContent).toBe(t("app.tip.wiki"));
    expect(tip.querySelector(".tip-wiki svg")).not.toBeNull();
    expect(tip.textContent).not.toContain(t("app.scope.wikiHint"));
  });
});
