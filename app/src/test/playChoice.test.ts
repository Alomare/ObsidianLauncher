import { describe, expect, it, vi } from "vitest";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const { playLabel, playingWar } = await import("../commander/choices");
const { defaultVanillaSave, defaultWarSave, vanillaSaves, warSaves } = await import("../commander/PlayChoice");
import type { SaveList, SaveRow } from "../saves/types";

const row = (id: string, kind: "official" | "sandbox", live = false): SaveRow => ({ id, name: id, kind, created: "", lastPlayed: null, snapshotOf: null, active: false, live, stats: null, error: null });

describe("the Play label", () => {
  it("shows PLAY and the war name only, and says vanilla or modded in the text for the tooltip", () => {
    expect(playLabel(null, false)).toEqual({ play: "Play", war: null, text: "Play Vanilla" });
    expect(playLabel(null, true).text).toBe("Play Modded");
    expect(playLabel("Night War", true)).toEqual({ play: "Play", war: "Night War", text: "Play Night War (Modded)" });
    expect(playLabel("Night War", false).text).toBe("Play Night War (Vanilla)");
  });

  it("finds the war being played among the choices", () => {
    const wars = [{ id: "a", name: "A", host: "local" as const }];
    expect(playingWar({ playing: "a", wars })?.name).toBe("A");
    expect(playingWar({ playing: "gone", wars })).toBeNull();
    expect(playingWar({ playing: null, wars })).toBeNull();
  });
});

describe("the saves of the official war", () => {
  const list: SaveList = { account: "1", active: "war1", saves: [row("off", "official", true), row("old", "official"), row("sb", "sandbox"), row("war1", "sandbox")] };

  it("are every save, war saves included, starting from the active one", () => {
    const saves = vanillaSaves(list);
    expect(saves.map((s) => s.id)).toEqual(["off", "old", "sb", "war1"]);
    expect(defaultVanillaSave(list, saves)).toBe("war1");
  });

  it("start from the live Official save when no save is active", () => {
    const l = { ...list, active: null };
    expect(defaultVanillaSave(l, vanillaSaves(l))).toBe("off");
  });
});

describe("the saves of a custom war", () => {
  const list: SaveList = { account: "1", active: "sb", saves: [row("off", "official", true), row("sb", "sandbox"), row("war1", "sandbox"), row("sb2", "sandbox")] };
  const war = { id: "w", name: "W", host: "local" as const, save: "war1" };

  it("are the war save first, then every other Sandbox, never an Official save", () => {
    expect(warSaves(list, war).map((s) => s.id)).toEqual(["war1", "sb", "sb2"]);
    expect(defaultWarSave(list, war, warSaves(list, war))).toBe("sb");
  });

  it("are only the war save when the war keeps to it", () => {
    const strict = { ...war, ownSaveOnly: true };
    expect(warSaves(list, strict).map((s) => s.id)).toEqual(["war1"]);
    expect(defaultWarSave(list, strict, warSaves(list, strict))).toBe("war1");
  });

  it("start from a new save when the war has none and nothing else fits", () => {
    const fresh = { ...war, save: undefined, ownSaveOnly: true };
    expect(warSaves(list, fresh)).toEqual([]);
    expect(defaultWarSave(list, fresh, [])).toBeNull();
    const official = { ...list, active: "off" };
    expect(defaultWarSave(official, { ...war, save: undefined }, warSaves(official, { ...war, save: undefined }))).toBeNull();
  });
});
