import { describe, expect, it } from "vitest";
import catalog from "../../../core/assets/save-catalog.json";
import { artFor, rankArt } from "../saves/art";
import { ladderState, levelPick, tally, upgradeMarks } from "../saves/logic";

describe("saves: service record helpers", () => {
  it("places a rank on the ladder", () => {
    expect([1, 33, 34, 35, 36].map((lv) => ladderState(lv, 34))).toEqual(["earned", "earned", "current", "next", "ahead"]);
    expect(ladderState(50, 50)).toBe("current");
  });

  it("lights the tally by the share of missions won", () => {
    expect(tally(709, 1010, 20)).toBe(14);
    expect(tally(0, 0, 20)).toBe(0);
    expect(tally(5, 5, 20)).toBe(20);
    expect(tally(9, 5, 20)).toBe(20);
  });

  it("marks upgrades by bit or by level", () => {
    const ups = [{ name: "a", display: "A", value: 1 }, { name: "b", display: "B", value: 2 }, { name: "c", display: "C", value: 4 }];
    expect(upgradeMarks({ upgradeKind: "bitmask", upgrade: 5, upgrades: ups })).toEqual([true, false, true]);
    const mk = [{ name: "mk2", display: "Mk2", value: 1 }, { name: "mk3", display: "Mk3", value: 2 }];
    expect(upgradeMarks({ upgradeKind: "level", upgrade: 1, upgrades: mk })).toEqual([true, false]);
  });

  it("climbs and steps down a Mk ladder", () => {
    expect(levelPick(0, 2)).toBe(2);
    expect(levelPick(2, 2)).toBe(1);
    expect(levelPick(2, 1)).toBe(0);
    expect(levelPick(1, 2)).toBe(2);
  });

  it("has a picture for every rank and catalog item, and none for a mod item", () => {
    for (let lv = 1; lv <= 50; lv++) expect(rankArt(lv)).toBeTruthy();
    const items = [...catalog.primary_weapons, ...catalog.sidearm_weapons, ...catalog.stratagems, ...catalog.perks];
    expect(items.filter((i) => !artFor(i.name)).map((i) => i.name)).toEqual([]);
    expect(artFor("mod_thing")).toBeUndefined();
  });
});
