// The Store tab: the Kit row and the cards, the filters and the sort, the mark of each mod against the
// library, the page of a mod, and an install that brings the requirements first.
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { t } from "../i18n";
import type { ModRow, Snapshot } from "../types";
import type { StoreMod } from "../store/storeIndex";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke, convertFileSrc: (p: string) => p }));
vi.mock("../opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));
vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: vi.fn() }));
const { StorePage, sortMods } = await import("../store/StorePage");
const { forgetStore, install, markOf } = await import("../store/storeIndex");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const entry = (id: string, extra: Partial<StoreMod> = {}): StoreMod => ({
  id, name: id.toUpperCase(), summary: `About ${id}`, description: `# ${id}\n\nLong text`, version: "1.2.0", early: false, scope: "client",
  requires: [], options: 0, option_list: [], option_groups: [], repo: `o/${id}`, kit: false, icon: null, banner: null, released: "2026-11-01", added: "2026-01-01",
  versions: [{ version: "1.2.0", date: "2026-11-01", channel: "stable", notes: "Fixed a thing" }], ...extra,
});
const row = (id: string, version: string): ModRow => ({
  id, name: { en: id }, description: {}, authors: [], version, scope: "client", on: false, options: [], keybinds: [], files: [],
  import: null, source: null, updatable: false, image: null, folder: null, requires: [], parts: [], optionGroups: [], optionPreviews: [],
  size: 0, deployed: false, deployedSize: null,
});
const snap = (mods: ModRow[]): Snapshot => ({
  mods, folders: [], profile: "Default", profiles: ["Default"], profileRows: [{ name: "Default", on: 0, blockPublic: false, guarded: false, image: null, color: null }],
  blockPublic: false, blockAchievements: false, freezeProgress: false, hideClientMods: false, loader: false, sandbox: null, conflicts: [], changes: "UpToDate", deployed: true, modded: true, game: "supported",
  gameChanges: [], gameDir: "C:\\Game", crashCulprit: null, broken: [], damagedProfiles: [], uncleanEnd: null, portable: false, version: "2.0.0",
});

const INDEX = [
  entry("api", { kit: true, scope: "library" }),
  entry("hud", { added: "2026-03-01", released: "2026-10-01" }),
  entry("arcade", { scope: "expansion", requires: ["api", "hud"], added: "2026-06-01", released: "2026-09-01", early: true, version: "0.4.0",
    banner: "C:\\store\\arcade.jpg", options: 3, option_groups: [{ id: "shop", label: "Shop" }],
    option_list: [{ label: "Lives", description: "Lives per mission", group: null }, { label: "Life", description: "Price of a life", group: "shop" }, { label: "Armor", description: "Price of armor", group: "shop" }] }),
  entry("crosshair", { added: "2026-02-01", released: "2026-11-05" }),
];

let root: Root | undefined;
afterEach(() => {
  act(() => root?.unmount());
  document.body.replaceChildren();
  invoke.mockReset();
});

async function show(mods: ModRow[]) {
  invoke.mockImplementation(async (cmd: string) => (cmd === "store_index" ? { mods: INDEX, offline: false } : undefined));
  const host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root!.render(<StorePage snap={snap(mods)} home={0} />));
  return host;
}
const names = (host: Element) => [...host.querySelectorAll(".card .c-body b")].map((b) => b.textContent);

describe("sortMods", () => {
  const list = INDEX.filter((m) => !m.kit);
  it("sorts by first release, last release, name and scope", () => {
    expect(sortMods(list, "new", []).map((m) => m.id)).toEqual(["arcade", "hud", "crosshair"]);
    expect(sortMods(list, "updated", []).map((m) => m.id)).toEqual(["crosshair", "hud", "arcade"]);
    expect(sortMods(list, "az", []).map((m) => m.id)).toEqual(["arcade", "crosshair", "hud"]);
    expect(sortMods(list, "za", []).map((m) => m.id)).toEqual(["hud", "crosshair", "arcade"]);
    expect(sortMods(list, "scope", []).map((m) => m.id)).toEqual(["crosshair", "hud", "arcade"]);
    expect(sortMods(list, "random", ["hud", "arcade", "crosshair"]).map((m) => m.id)).toEqual(["hud", "arcade", "crosshair"]);
  });
});

describe("markOf", () => {
  it("compares the Store version with the copy in the library", () => {
    const m = entry("hud");
    expect(markOf(m, [], [])).toBe("none");
    expect(markOf(m, [row("hud", "1.2.0")], [])).toBe("installed");
    expect(markOf(m, [row("hud", "1.3.0")], [])).toBe("installed");
    expect(markOf(m, [row("hud", "1.1.9")], [])).toBe("update");
    expect(markOf(m, [row("hud", "1.1.9")], ["hud"])).toBe("installing");
  });
});

describe("StorePage", () => {
  it("shows the Kit apart, the other mods newest first, and filters them", async () => {
    const host = await show([row("hud", "1.2.0")]);
    expect([...host.querySelectorAll(".kit-card b")].map((b) => b.textContent)).toEqual(["API"]);
    expect(names(host)).toEqual(["ARCADE", "HUD", "CROSSHAIR"]);
    expect(host.querySelector(".card .store-tag.early")?.textContent).toBe(t("app.store.early"));
    // No thumbnail and no icon: a blurred picture alone, never an empty box.
    const plain = host.querySelector(".card .c-img .backdrop")!;
    expect(plain.querySelector(".bd-blur")).not.toBeNull();
    expect(plain.querySelector(".mico")).toBeNull();
    const hud = [...host.querySelectorAll(".card")].find((c) => c.textContent?.includes("HUD"))!;
    expect(hud.querySelector(".c-flag.inst")).not.toBeNull();

    const chip = [...host.querySelectorAll(".chip")].find((b) => b.textContent === t("app.scope.expansion"))!;
    await act(async () => (chip as HTMLElement).click());
    expect(names(host)).toEqual(["ARCADE"]);
    await act(async () => (chip as HTMLElement).click());

    const inst = host.querySelector(".fbar .check-line") as HTMLElement;
    await act(async () => inst.click());
    expect(names(host)).toEqual(["HUD"]);
    await act(async () => inst.click());

    const search = host.querySelector(".fbar input") as HTMLInputElement;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(search, "cross");
      search.dispatchEvent(new Event("input", { bubbles: true }));
    });
    expect(names(host)).toEqual(["CROSSHAIR"]);
  });

  it("opens the page of a mod with the description, the versions and the requirements", async () => {
    const host = await show([row("api", "1.2.0")]);
    const card = [...host.querySelectorAll(".card")].find((c) => c.textContent?.includes("ARCADE")) as HTMLElement;
    await act(async () => card.click());
    expect(host.querySelector(".mid h1")?.textContent).toBe("ARCADE");
    expect(host.querySelector(".store-mod .md")?.textContent).toContain("Long text");
    expect(host.querySelector(".msum")?.textContent).toBe("About arcade");
    // One row of facts: scope, version, first release, newest release.
    expect([...host.querySelectorAll(".mfacts dt")].map((d) => d.textContent)).toEqual(
      [t("app.store.factScope"), t("app.store.factVersion"), t("app.store.factReleased"), t("app.store.factUpdated")]);
    expect(host.querySelectorAll(".mfacts dd")[1].textContent).toBe("0.4.0");
    // The side panels in order, each with a count: Versions, Options, Requires.
    const panels = [...host.querySelectorAll(".mside .mpanel")];
    expect(panels.map((p) => p.querySelector(".mp-h h2")?.textContent)).toEqual([t("app.store.versions"), t("app.store.options"), t("app.store.requires")]);
    expect(panels.map((p) => p.querySelector(".mp-h i")?.textContent)).toEqual(["1", "3", "2"]);
    expect(panels[0].querySelector(".mp-h .gh")).not.toBeNull();
    expect(host.querySelector(".mod-log-entry.open .upd-body")?.textContent).toContain("Fixed a thing");
    const reqs = [...panels[2].querySelectorAll(".req .rq-state")].map((s) => s.textContent);
    expect(reqs).toEqual([t("app.store.installed"), t("app.store.notInstalled")]);
    // Options: loose ones listed, the rest under their group, folded.
    expect([...panels[1].querySelectorAll(":scope > .sopt-rows .sopt-name")].map((b) => b.textContent)).toEqual(["Lives"]);
    const group = panels[1].querySelector("details.sopt-group") as HTMLDetailsElement;
    expect(group.open).toBe(false);
    expect(group.querySelector("summary")?.textContent).toContain("Shop");
    expect([...group.querySelectorAll(".sopt-name")].map((s) => [s.textContent, s.getAttribute("data-tip")])).toEqual([["Life", "Price of a life"], ["Armor", "Price of armor"]]);
    // The band opens the thumbnail large; Back sits over the band.
    await act(async () => (host.querySelector(".mband .mb-zoom") as HTMLElement).click());
    expect(document.querySelector(".image-view img")).not.toBeNull();
    await act(async () => (document.querySelector(".image-view") as HTMLElement).click());
    await act(async () => (host.querySelector(".mband > .back") as HTMLElement).click());
    expect(host.querySelector(".store-mod")).toBeNull();
    expect(names(host).length).toBe(3);
  });

  it("reads the list from the cache with a note when GitHub is out of reach, and offers Try again on a failure", async () => {
    invoke.mockImplementation(async () => { throw { code: "UpdateNetwork", message: "GitHub could not be reached: offline" }; });
    act(() => forgetStore());
    const host = document.createElement("div");
    document.body.append(host);
    root = createRoot(host);
    await act(async () => root!.render(<StorePage snap={snap([])} home={0} />));
    expect(host.querySelector(".store-empty h2")?.textContent).toBe(t("app.store.failed"));

    invoke.mockImplementation(async () => ({ mods: INDEX, offline: true }));
    await act(async () => (host.querySelector(".store-empty .btn") as HTMLElement).click());
    expect(host.querySelector(".store-note")).not.toBeNull();
    expect(names(host).length).toBe(3);
  });
});

describe("install", () => {
  it("installs the requirements the library lacks first, one at a time", async () => {
    const done: string[] = [];
    invoke.mockImplementation(async (cmd: string, args: { id: string }) => {
      done.push(`${cmd}:${args.id}`);
      return { snapshot: snap([]), results: [{ file: args.id, ok: true, kind: "installed", name: { en: args.id }, version: "1.0.0", oldVersion: null, error: null }] };
    });
    await install(INDEX[2], INDEX, [row("api", "1.2.0")]);
    expect(done).toEqual(["store_install:hud", "store_install:arcade"]);
  });

  it("stops at a failed requirement", async () => {
    const done: string[] = [];
    invoke.mockImplementation(async (_cmd: string, args: { id: string }) => {
      done.push(args.id);
      throw { code: "UpdateNetwork", message: "offline" };
    });
    await install(INDEX[2], INDEX, []);
    expect(done).toEqual(["api"]);
  });
});
