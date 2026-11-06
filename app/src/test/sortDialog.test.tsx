import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import en from "../../../locales/en.json";
import { clashText, moveText, SortDialog, type SortPlan } from "../mods/SortDialog";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | undefined;
afterEach(() => {
  act(() => root?.unmount());
  document.body.replaceChildren();
});

const NAMES: Record<string, string> = { a: "Alpha", b: "Bravo", c: "Charlie", d: "Delta", z: "Zulu" };
const name = (id: string) => NAMES[id] ?? id;

function show(plan: SortPlan, props: { onApply?: () => void; onClose?: () => void; off?: string[] } = {}) {
  const host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  act(() => root!.render(<SortDialog plan={plan} name={name} off={new Set(props.off ?? [])} onApply={props.onApply ?? (() => {})} onClose={props.onClose ?? (() => {})} />));
  return document.querySelector(".modal.sort") as HTMLElement;
}
const buttons = (m: HTMLElement) => [...m.querySelectorAll(".modal-foot button")].map((b) => b.textContent);

const moved: SortPlan = {
  order: ["b", "c", "a", "z"],
  moves: [
    { id: "a", from: 0, to: 2, rules: [{ mod: "a", kind: "after", other: "c" }], tie: { other: "c", later: true, files: ["lua/x", "lua/y"] } },
    { id: "z", from: 1, to: 3, rules: [{ mod: "z", kind: "last" }] },
  ],
  conflicts: [],
  notes: [],
};

describe("sort dialog", () => {
  it("lists the whole order and marks the moved mods with the reason", () => {
    const m = show(moved, { off: ["c"] });
    expect(m.querySelector("h2")!.textContent).toBe("Sort Mods");
    expect(m.querySelector(".lead")!.textContent).toBe("2 mods move. The rest keep their order.");
    const rows = [...m.querySelectorAll("tbody tr")];
    expect(rows.map((r) => r.querySelector(".nm")!.textContent)).toEqual(["Bravo", "Charlie", "Alpha", "Zulu"]);
    expect(rows.map((r) => r.classList.contains("moved"))).toEqual([false, false, true, true]);
    expect(rows[1].classList.contains("off")).toBe(true);
    expect(rows[2].querySelector(".from-to")!.textContent).toBe("1 → 3");
    expect(rows[2].querySelector(".sort-arrow")).not.toBeNull();
    expect(rows[2].querySelector(".reason")!.textContent).toBe("Loads after Charlie. Overwrites less of Charlie: 2 shared files");
    expect((rows[2].querySelector(".why") as HTMLElement).dataset.tip).toBe("lua/x\nlua/y");
    expect(rows[3].querySelector(".reason")!.textContent).toBe("Asks to load last");
    expect((rows[3].querySelector(".why") as HTMLElement).dataset.tip).toBeUndefined();
    expect(m.textContent).not.toContain("Not moved");
    expect(buttons(m)).toEqual(["Cancel", "Apply"]);
  });

  it("applies, cancels and closes on Escape", () => {
    const onApply = vi.fn();
    const onClose = vi.fn();
    const m = show(moved, { onApply, onClose });
    const [cancel, apply] = [...m.querySelectorAll<HTMLButtonElement>(".modal-foot button")];
    expect(document.activeElement).toBe(apply);
    act(() => apply.click());
    expect(onApply).toHaveBeenCalledTimes(1);
    act(() => cancel.click());
    act(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })));
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it("says so when nothing moves, with one Close", () => {
    const m = show({ order: ["a", "b"], moves: [], conflicts: [], notes: [] });
    expect(m.querySelector(".lead")!.textContent).toBe("The order already follows every mod's request.");
    expect(m.querySelector("table")).toBeNull();
    expect(buttons(m)).toEqual(["Close"]);
  });

  it("names the mods that clash and stay, and the requests skipped", () => {
    const m = show({
      order: ["a", "b", "c"],
      moves: [],
      conflicts: [{ kind: "both_last", mods: ["a", "b"], rules: [{ mod: "a", kind: "last" }, { mod: "b", kind: "last" }] }],
      notes: [{ mod: "c", missing: "gone.mod" }],
    });
    expect(m.querySelector(".lead")!.textContent).toBe("No mod moves. The requests below can't all hold.");
    expect(m.textContent).toContain("Not moved");
    expect(m.textContent).toContain("Alpha and Bravo both ask to load last. Order them by hand, or ask their authors.");
    expect(m.textContent).toContain("Charlie names gone.mod, which is not installed. That request is skipped.");
    expect(buttons(m)).toEqual(["Close"]);
  });
});

describe("sort texts", () => {
  it("words a request from the moved mod's side", () => {
    expect(moveText({ id: "b", from: 2, to: 0, rules: [{ mod: "a", kind: "after", other: "b" }] }, name)).toBe("Alpha asks to load after this mod");
    expect(moveText({ id: "b", from: 2, to: 0, rules: [{ mod: "b", kind: "before_patches" }] }, name)).toBe("Loads before mods that replace game files");
    expect(moveText({ id: "b", from: 2, to: 0, rules: [{ mod: "a", kind: "first" }] }, name)).toBe("Alpha asks to load first");
    expect(moveText({ id: "b", from: 2, to: 0, rules: [{ mod: "b", kind: "before", other: "d" }], tie: { other: "a", later: false, files: ["lua/x"] } }, name))
      .toBe("Loads before Delta. Alpha overwrites less of this mod: lua/x");
  });

  it("words each kind of clash", () => {
    expect(clashText({ kind: "both_first", mods: ["a", "b", "c"], rules: [] }, name)).toBe("Alpha, Bravo and Charlie all ask to load first. Order them by hand, or ask their authors.");
    expect(clashText({ kind: "cycle", mods: ["a", "b"], rules: [{ mod: "a", kind: "after", other: "b" }, { mod: "b", kind: "after", other: "a" }] }, name))
      .toBe("These requests can't all hold: Alpha loads after Bravo; Bravo loads after Alpha. Order them by hand, or ask their authors.");
    expect(clashText({ kind: "locked", mods: ["a"], rules: [{ mod: "a", kind: "after", other: "b" }] }, name)).toContain("Unlock the folder");
  });
});

describe("sort text rules", () => {
  // Public text: no "its", no em dashes, no first person, no "...", and buttons in Title Case.
  const keys = Object.entries(en as Record<string, string>).filter(([k]) => k.startsWith("app.sort."));
  it("keeps the writing rules", () => {
    expect(keys.length).toBeGreaterThan(30);
    for (const [k, v] of keys) {
      expect(v, k).not.toMatch(/\bits\b|—|…|\.\.\.|\b(I|me|my|we|our)\b|Arsenal/i);
    }
    for (const k of ["app.sort.apply", "app.sort.undo", "app.sort.title"]) {
      const v = (en as Record<string, string>)[k];
      expect(v.split(" ").every((w) => /^[A-Z]/.test(w)), k).toBe(true);
    }
  });
});
