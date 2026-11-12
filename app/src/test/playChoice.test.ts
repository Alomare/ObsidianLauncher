import { describe, expect, it, vi } from "vitest";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const { playLabel, playLabelText, playingWar } = await import("../commander/choices");
const { defaultVanillaSave, vanillaSaves } = await import("../commander/PlayChoice");
import type { SaveList, SaveRow } from "../saves/types";

const row = (id: string, kind: "official" | "sandbox", live = false): SaveRow => ({ id, name: id, kind, created: "", lastPlayed: null, snapshotOf: null, active: false, live, stats: null, error: null });

describe("the Play label", () => {
  it("says vanilla or modded, with the war name apart", () => {
    expect(playLabelText(playLabel(null, false))).toBe("Play Vanilla");
    expect(playLabelText(playLabel(null, true))).toBe("Play Modded");
    const l = playLabel("Night War", true);
    expect(l).toEqual({ before: "Play ", war: "Night War", after: " (Modded)" });
    expect(playLabelText(playLabel("Night War", false))).toBe("Play Night War (Vanilla)");
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
  const wars = [{ id: "w", name: "W", host: "local" as const, save: "war1" }];

  it("leave out the war saves and start from the live Official save when a war save is active", () => {
    const saves = vanillaSaves(list, wars);
    expect(saves.map((s) => s.id)).toEqual(["off", "old", "sb"]);
    expect(defaultVanillaSave(list, saves)).toBe("off");
  });

  it("start from the active save when it is not a war save", () => {
    const l = { ...list, active: "sb" };
    expect(defaultVanillaSave(l, vanillaSaves(l, wars))).toBe("sb");
  });
});
