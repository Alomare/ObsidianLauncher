import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ModRow } from "../types";

const openUrl = vi.fn(() => Promise.resolve());
vi.mock("../api", () => ({ run: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: (...a: unknown[]) => openUrl(...(a as [])) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
const { Drawer } = await import("../mods/Drawer");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | undefined;
afterEach(() => {
  act(() => root?.unmount());
  document.body.replaceChildren();
  openUrl.mockClear();
});

const SOURCE = "https://github.com/someone/a-mod-with-a-rather-long-repository-name/releases/latest";
const mod = (source: string | null) =>
  ({
    id: "someone.long-mod", name: { en: "Long Mod" }, description: { en: "" }, authors: ["Someone"], version: "1.0.0", scope: "client", on: true,
    options: [], keybinds: [], files: [], import: null, source, updatable: false, image: null, folder: null, requires: [],
  }) as ModRow;

function show(m: ModRow, updates: Record<string, string> = {}, onUpdate: (id: string) => void = () => {}) {
  const box = document.createElement("div");
  document.body.append(box);
  root = createRoot(box);
  act(() => root!.render(<Drawer view={{ kind: "mod", id: m.id }} mods={[m]} conflicts={[]} order={[m.id]} names={new Map([[m.id, "Long Mod"]])} onOpen={() => {}} onClose={() => {}}
    updates={updates} onUpdate={onUpdate} />));
  return box;
}

describe("the About tab", () => {
  it("labels_the_id_in_capitals", () => {
    const labels = [...show(mod(null)).querySelectorAll("dt")].map((d) => d.textContent);
    expect(labels).toContain("Mod ID");
  });

  it("shows_the_source_as_an_icon_with_the_link_in_the_tooltip", () => {
    const link = show(mod(SOURCE)).querySelector<HTMLButtonElement>("button.src-link")!;
    expect(link.textContent).toBe("");
    expect(link.querySelector("svg")).not.toBeNull();
    expect(link.dataset.tip).toBe(SOURCE);
    expect(link.getAttribute("aria-label")).toBe(SOURCE);
    act(() => link.click());
    expect(openUrl).toHaveBeenCalledWith(SOURCE);
  });

  it("has_no_source_row_without_a_source", () => {
    expect(show(mod(null)).querySelector(".src-link")).toBeNull();
  });
});

describe("the panel's update offer", () => {
  it("offers a newer version found, as the row does, and asks through the same question", () => {
    const onUpdate = vi.fn();
    const offer = show(mod(SOURCE), { "someone.long-mod": "1.2.0" }, onUpdate).querySelector<HTMLButtonElement>(".dh .dh-update");
    expect(offer?.textContent).toBe("Update to 1.2.0");
    act(() => offer!.click());
    expect(onUpdate).toHaveBeenCalledWith("someone.long-mod");
  });

  it("offers nothing without a newer version", () => {
    expect(show(mod(SOURCE), { "someone.long-mod": "1.0.0" }).querySelector(".dh-update")).toBeNull();
    expect(show(mod(SOURCE)).querySelector(".dh-update")).toBeNull();
  });
});
