import { describe, expect, it } from "vitest";
import { conflictsByMod, conflictsOf, overrides, winners } from "../mods/conflicts";
import { dependencies } from "../mods/deps";
import { keyName } from "../mods/keyCapture";
import { clampNumber } from "../mods/optionValue";
import type { ModRow } from "../types";

const key = (code: string, k = "") => ({ code, key: k }) as KeyboardEvent;

describe("dependencies", () => {
  const mod = (id: string, on: boolean, requires: string[] = []) => ({ id, on, requires }) as ModRow;
  const mods = [mod("api", false), mod("trainer", true, ["api", "gone"]), mod("extra", false, ["trainer"])];
  it("gives what a mod needs and what needs it, with their states", () => {
    expect(dependencies(mods[1], mods)).toEqual({
      needs: [{ id: "api", state: "off" }, { id: "gone", state: "missing" }],
      neededBy: [{ id: "extra", state: "off" }],
    });
    expect(dependencies(mods[0], mods)).toEqual({ needs: [], neededBy: [{ id: "trainer", state: "on" }] });
  });
  it("leaves out mods two steps away", () => {
    expect(dependencies(mods[2], mods).needs).toEqual([{ id: "trainer", state: "on" }]);
  });
});

describe("clampNumber", () => {
  it("snaps to the step and stays in range", () => {
    const spec = { min: 0, max: 10, step: 0.5 };
    expect(clampNumber(3.3, spec)).toBe(3.5);
    expect(clampNumber(-4, spec)).toBe(0);
    expect(clampNumber(99, spec)).toBe(10);
    expect(clampNumber(7.24, { min: 1, max: null, step: 0.1 })).toBe(7.2);
    expect(clampNumber(NaN, spec)).toBe(0);
  });
  it("counts steps from the minimum", () => {
    expect(clampNumber(4, { min: 1, max: 10, step: 2 })).toBe(5);
  });
});

describe("keyName", () => {
  it("maps letters, digits, F keys and a few named keys", () => {
    expect(keyName(key("KeyQ", "q"))).toBe("q");
    expect(keyName(key("Digit7", "7"))).toBe("7");
    expect(keyName(key("F11", "F11"))).toBe("f11");
    expect(keyName(key("Space", " "))).toBe("space");
    expect(keyName(key("Insert", "Insert"))).toBe("insert");
  });
  it("returns null for Esc and for keys the loader cannot read", () => {
    expect(keyName(key("Escape", "Escape"))).toBeNull();
    expect(keyName(key("ShiftLeft", "Shift"))).toBeNull();
    expect(keyName(key("Numpad5", "5"))).toBeNull();
  });
});

describe("winners", () => {
  const rows = [{ resource: "a/x", mods: ["tpc", "cam"], winner: "cam" }];
  it("picks the mod lowest in the given order", () => {
    expect(winners(rows, ["qol", "cam", "tpc"])[0]).toEqual({ resource: "a/x", mods: ["cam", "tpc"], winner: "tpc" });
    expect(winners(rows, ["tpc", "cam"])[0].winner).toBe("cam");
  });
  it("keeps mods missing from the order at the top", () => {
    expect(winners(rows, ["cam"])[0].mods).toEqual(["tpc", "cam"]);
  });
});

describe("overrides", () => {
  const rows = [
    { resource: "a/1", mods: ["x", "y"], winner: "y" },
    { resource: "a/2", mods: ["x", "y"], winner: "y" },
    { resource: "a/3", mods: ["x", "y", "z"], winner: "z" },
  ];
  it("gives one entry per winner and loser, with the files the winner takes", () => {
    expect(overrides(rows)).toEqual([
      { winner: "y", loser: "x", resources: ["a/1", "a/2"] },
      { winner: "z", loser: "x", resources: ["a/3"] },
      { winner: "z", loser: "y", resources: ["a/3"] },
    ]);
  });
  it("tells one mod which mods it overrides and which override it", () => {
    expect(conflictsOf("y", rows)).toEqual({ overrides: [["x", 2]], overriddenBy: [["z", 1]] });
    expect(conflictsOf("z", rows)).toEqual({ overrides: [["x", 1], ["y", 1]], overriddenBy: [] });
    expect(conflictsOf("q", rows)).toEqual({ overrides: [], overriddenBy: [] });
  });
  it("works out every mod at once as it does one by one", () => {
    const all = conflictsByMod(rows);
    expect([...all.keys()].sort()).toEqual(["x", "y", "z"]);
    for (const id of ["x", "y", "z"]) expect(all.get(id)).toEqual(conflictsOf(id, rows));
  });
});
