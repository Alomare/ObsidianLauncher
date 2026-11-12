import { describe, expect, it } from "vitest";
import { draftOf, fieldEnabled, fileSafe, groupItems, hasBit, itemMatches, ownedRoute, parseWhole, playTime, rankOfXp, rankProgress, researchPoints, slotChoices, stage, withBit, xpOfRank } from "../saves/logic";
import type { Edit, EditorState, ItemState, ModFieldState } from "../saves/types";

const ranks = [{ level: 1, requiredXp: 0, title: "A" }, { level: 2, requiredXp: 100, title: "B" }, { level: 3, requiredXp: 300, title: "C" }];

const item = (over: Partial<ItemState>): ItemState => ({
  name: "w", display: "W", description: "", category: "shotgun", dlc: false, locked: false, owned: false, inUnlocks: false, grantedRank: null,
  upgradeKind: "bitmask", upgrade: 0, maxUpgrade: 7, upgrades: [], ...over,
});

const state = (over: Partial<EditorState> = {}): EditorState => ({
  id: "x", name: "S", xp: 150, rank: 2, samples: 25, researchPoints: 2, maxSamples: 100, samplesPerPoint: 10, ranks,
  bossInfluence: [0, 0, 0], bossInfluenceRequired: 50, tutorialStage: 1, primaryWeapons: [item({ name: "free", inUnlocks: true, owned: true }), item({ name: "ranked", grantedRank: 3 })],
  sidearmWeapons: [], stratagems: [], perks: [], customization: {}, cosmetics: { helmet: [], body: [], cape: [], gender: [] },
  unknownUnlocks: ["mod_thing"], flags: [{ key: "f", value: false }], lifetime: { kills: 1 }, mods: [], ...over,
});

describe("saves: rank and XP", () => {
  it("finds the rank from XP and the XP from a rank", () => {
    expect(rankOfXp(ranks, 0).level).toBe(1);
    expect(rankOfXp(ranks, 99).level).toBe(1);
    expect(rankOfXp(ranks, 100).level).toBe(2);
    expect(rankOfXp(ranks, 9999).level).toBe(3);
    expect(xpOfRank(ranks, 3)).toBe(300);
  });

  it("gives the way to the next rank, and none at the top", () => {
    expect(rankProgress(ranks, 200)).toEqual({ fraction: 0.5, need: 100, next: ranks[2] });
    expect(rankProgress(ranks, 300)).toEqual({ fraction: 1, need: null, next: null });
  });

  it("makes research points from samples", () => {
    expect(researchPoints(25, 10)).toBe(2);
    expect(researchPoints(9, 10)).toBe(0);
  });
});

describe("saves: upgrade bits", () => {
  it("reads and changes one bit without the others", () => {
    expect(hasBit(5, 1)).toBe(true);
    expect(hasBit(5, 2)).toBe(false);
    expect(withBit(5, 2, true)).toBe(7);
    expect(withBit(5, 4, false)).toBe(1);
  });
});

describe("saves: what the owned switch can do", () => {
  it("names the route", () => {
    expect(ownedRoute(item({ dlc: true, locked: true, owned: false }), 1)).toBe("dlc");
    expect(ownedRoute(item({ dlc: true, locked: true, inUnlocks: true }), 1)).toBe("dlc");
    expect(ownedRoute(item({ dlc: true, owned: true }), 1)).toBe("entitled");
    expect(ownedRoute(item({ dlc: true, owned: true, inUnlocks: true }), 1)).toBe("entitled");
    expect(ownedRoute(item({ inUnlocks: true, owned: true }), 1)).toBe("unlocked");
    expect(ownedRoute(item({ grantedRank: 2, owned: true }), 2)).toBe("rank");
    expect(ownedRoute(item({ grantedRank: 5, owned: false }), 2)).toBe("none");
    expect(ownedRoute(item({ owned: true }), 2)).toBe("fixed");
  });
});

describe("saves: cosmetic choices", () => {
  it("lists owned DLC and leaves out locked DLC, unless the save wears it", () => {
    const list = [{ locked: false }, { locked: true }, { locked: false }];
    expect(slotChoices(list, 1)).toEqual([1, 3]);
    expect(slotChoices(list, 2)).toEqual([1, 2, 3]);
    expect(slotChoices(list, undefined)).toEqual([1, 3]);
  });
});

describe("saves: owned DLC in the draft", () => {
  it("stays owned through other edits, since Steam grants it", () => {
    const base = state({ primaryWeapons: [item({ name: "dlc_gun", dlc: true, owned: true })] });
    const d = draftOf(base, stage(new Map(), base, { op: "set_xp", xp: 0 }));
    expect(d.primaryWeapons[0].owned).toBe(true);
  });
});

describe("saves: the Arsenal list", () => {
  it("searches and groups by category in order", () => {
    const list = [item({ name: "a", category: "shotgun", display: "Punisher" }), item({ name: "b", category: "machinegun", display: "Stalwart" }), item({ name: "c", category: "shotgun", display: "Breaker" })];
    expect(groupItems(list, "").map((g) => [g.category, g.items.length])).toEqual([["shotgun", 2], ["machinegun", 1]]);
    expect(groupItems(list, "break").map((g) => g.items[0].name)).toEqual(["c"]);
    expect(itemMatches(list[0], "pun sho")).toBe(true);
  });
});

describe("saves: typing numbers", () => {
  it("clamps, rounds and refuses text", () => {
    expect(parseWhole("150", 0, 100)).toBe(100);
    expect(parseWhole("-4", 0, null)).toBe(0);
    expect(parseWhole("12.6", null, null)).toBe(13);
    expect(parseWhole("1,000", 0, null)).toBe(1000);
    expect(parseWhole("abc", 0, 10)).toBeNull();
  });
});

describe("saves: waiting edits", () => {
  const base = state();
  const add = (p: Map<string, Edit>, e: Edit) => stage(p, base, e);

  it("keeps one edit per thing, and rank and XP are one thing", () => {
    let p = add(new Map(), { op: "set_xp", xp: 200 });
    p = add(p, { op: "set_rank", rank: 3 });
    expect([...p.values()]).toEqual([{ op: "set_rank", rank: 3 }]);
  });

  it("drops an edit that goes back to the saved value", () => {
    let p = add(new Map(), { op: "set_samples", samples: 40 });
    expect(p.size).toBe(1);
    p = add(p, { op: "set_samples", samples: 25 });
    expect(p.size).toBe(0);
    p = add(new Map(), { op: "set_rank", rank: 3 });
    p = add(p, { op: "set_rank", rank: 2 });
    expect(p.size).toBe(0);
  });

  it("shows the save as it will be", () => {
    let p = add(new Map(), { op: "set_rank", rank: 3 });
    p = add(p, { op: "set_samples", samples: 40 });
    p = add(p, { op: "set_owned", item: "mod_thing", owned: false });
    const d = draftOf(base, p);
    expect(d.xp).toBe(300);
    expect(d.researchPoints).toBe(4);
    expect(d.unknownUnlocks).toEqual([]);
    expect(d.primaryWeapons.find((i) => i.name === "ranked")!.owned).toBe(true);
    expect(base.xp).toBe(150);
  });

  it("clearing a mod drops the edits waiting for that mod", () => {
    const mod = { id: "m", name: null, state: "removed" as const, items: [], fields: [], data: [{ key: "a", value: 1 }, { key: "b", value: 2 }] };
    const b = state({ mods: [mod] });
    let p = stage(new Map(), b, { op: "mod_delete", mod_id: "m", key: "a" });
    p = stage(p, b, { op: "mod_clear", mod_id: "m" });
    expect([...p.keys()]).toEqual(["clear:m"]);
    expect(draftOf(b, p).mods).toEqual([]);
  });
});

describe("saves: small helpers", () => {
  it("field when, file names and time", () => {
    const f = (id: string, value: ModFieldState["value"], when: ModFieldState["when"] = null): ModFieldState => ({ id, kind: "toggle", label: null, min: null, max: null, choices: null, when, value });
    const a = f("a", "hard");
    expect(fieldEnabled(f("b", 1, { a: "hard" }), [a])).toBe(true);
    expect(fieldEnabled(f("b", 1, { a: "easy" }), [a])).toBe(false);
    expect(fileSafe('My: save?')).toBe("My save");
    expect(playTime(3723)).toBe("1 h 2 min");
  });
});
