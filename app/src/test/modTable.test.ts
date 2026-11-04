import { describe, expect, it } from "vitest";
import { filterMods } from "../mods/rows";
import type { ModRow } from "../types";

const row = (id: string, name: string, authors: string[], about = ""): ModRow => ({
  id, name: { en: name }, description: { en: about }, authors, version: "1.0.0", scope: "client", on: true,
  options: [], keybinds: [], files: [], import: null, source: null, updatable: false, image: null, customImage: false, folder: null, requires: [],
});
const rows = [row("a", "Helldivers Rework", ["Bia"], "New enemy waves"), row("b", "HD2CAM", ["camdev"]), row("c", "Third Person Camera", ["Zed"])];

describe("filterMods", () => {
  it("matches names and authors, ignoring case", () => {
    expect(filterMods(rows, "rework").map((r) => r.id)).toEqual(["a"]);
    expect(filterMods(rows, "CAM").map((r) => r.id)).toEqual(["b", "c"]);
    expect(filterMods(rows, "zed").map((r) => r.id)).toEqual(["c"]);
  });
  it("also looks in the description and the id", () => {
    expect(filterMods(rows, "enemy").map((r) => r.id)).toEqual(["a"]);
    expect(filterMods([row("obsidian.qol", "Obsidian Patches", [])], "obsidian.qol")).toHaveLength(1);
  });
  it("keeps everything for an empty or blank query", () => {
    expect(filterMods(rows, "")).toEqual(rows);
    expect(filterMods(rows, "  ")).toEqual(rows);
  });
  it("finds nothing when nothing matches", () => {
    expect(filterMods(rows, "trainer")).toEqual([]);
  });
});
