import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DELETE_WAIT_S, DeleteOfficial, namesSave } from "../saves/DeleteOfficial";
import type { SaveRow } from "../saves/types";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const row: SaveRow = { id: "a1", name: "Main War", kind: "official", created: "", lastPlayed: null, snapshotOf: null, active: false, live: false, stats: null, error: null };

let root: Root | undefined;
beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  act(() => root?.unmount());
  document.body.replaceChildren();
  vi.useRealTimers();
});

function show(onDelete: (n: string) => Promise<boolean>) {
  const host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  act(() => root!.render(<DeleteOfficial row={row} onDelete={onDelete} onCancel={() => {}} />));
}
const input = () => document.querySelector<HTMLInputElement>(".delete-save input")!;
const del = () => document.querySelectorAll<HTMLButtonElement>(".modal-foot button.btn")[1];
function type(v: string) {
  const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
  act(() => {
    set.call(input(), v);
    input().dispatchEvent(new Event("input", { bubbles: true }));
  });
}
const wait = (s: number) => { for (let i = 0; i < s; i++) act(() => { vi.advanceTimersByTime(1000); }); };

describe("deleting an Official save", () => {
  it("matches the name exactly", () => {
    expect(namesSave("Main War", "Main War")).toBe(true);
    for (const v of ["main war", "Main War ", " Main War", "Main", ""]) expect(namesSave(v, "Main War")).toBe(false);
  });

  it("starts in the name box with Delete off, and needs the name and the wait", () => {
    const onDelete = vi.fn(async () => true);
    show(onDelete);
    expect(document.activeElement).toBe(input());
    expect(del().disabled).toBe(true);
    type("Main War");
    expect(del().disabled).toBe(true);
    wait(DELETE_WAIT_S - 1);
    expect(del().disabled).toBe(true);
    wait(1);
    expect(del().disabled).toBe(false);
    type("Main war");
    expect(del().disabled).toBe(true);
    type("Main War");
    act(() => del().click());
    expect(onDelete).toHaveBeenCalledWith("Main War");
  });

  it("does nothing on Enter, even when ready", () => {
    const onDelete = vi.fn(async () => true);
    show(onDelete);
    type("Main War");
    wait(DELETE_WAIT_S);
    const enter = new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true });
    act(() => { input().dispatchEvent(enter); });
    expect(enter.defaultPrevented).toBe(true);
    del().focus();
    const onButton = new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true });
    act(() => { del().dispatchEvent(onButton); });
    expect(onButton.defaultPrevented).toBe(true);
    expect(onDelete).not.toHaveBeenCalled();
  });
});
