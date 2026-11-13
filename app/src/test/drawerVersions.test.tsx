import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ModRow } from "../types";

const call = vi.fn();
vi.mock("../api", () => ({ run: vi.fn(), call: (...a: unknown[]) => call(...a), errorText: (e: unknown) => String(e), setSnapshot: vi.fn() }));
vi.mock("../busy", () => ({ withTask: (_: string, f: () => unknown) => f() }));
vi.mock("../notices", () => ({ notify: vi.fn() }));
vi.mock("../opener", () => ({ openUrl: vi.fn(() => Promise.resolve()) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
const { Drawer, channelOf, channelsOf } = await import("../mods/Drawer");
const { compareVersions } = await import("../updates");
const { useModUpdates } = await import("../mods/modUpdates");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | undefined;
afterEach(() => {
  act(() => root?.unmount());
  document.body.replaceChildren();
  call.mockReset();
});

const RELEASES = [
  { version: "1.2.0", date: null, notes: "Two", url: "u2", tag: "v1.2.0", channel: "stable" },
  { version: "1.3.0-beta.1", date: null, notes: "Beta", url: "u3", tag: "v1.3.0-beta.1", channel: "beta" },
  { version: "1.1.0", date: null, notes: "One", url: "u1", tag: "v1.1.0", channel: "stable" },
];

const mod = (version: string) =>
  ({
    id: "me.mod", name: { en: "My Mod" }, description: { en: "" }, authors: [], version, scope: "client", on: true,
    options: [], keybinds: [], files: [], import: null, source: "https://github.com/me/mod", updatable: true, image: null, folder: null,
    requires: [], parts: [], optionGroups: [], optionPreviews: [],
  }) as ModRow;

let found: Record<string, string> = {};
function Probe() {
  found = useModUpdates().found;
  return null;
}

async function show(m: ModRow) {
  call.mockImplementation((cmd: string) => {
    if (cmd === "mod_releases") return Promise.resolve({ releases: RELEASES, more: false });
    if (cmd === "install_mod_version") return Promise.resolve({ snapshot: {}, results: [] });
    return Promise.reject(new Error(cmd));
  });
  const box = document.createElement("div");
  document.body.append(box);
  root = createRoot(box);
  act(() => root!.render(<><Probe /><Drawer view={{ kind: "mod", id: m.id }} mods={[m]} conflicts={[]} order={[m.id]} names={new Map([[m.id, "My Mod"]])} onOpen={() => {}} onClose={() => {}} /></>));
  const tab = box.querySelector<HTMLButtonElement>('button[aria-label="Versions"]')!;
  act(() => tab.click());
  // The list loads a moment after the tab opens.
  await act(async () => { await new Promise((r) => setTimeout(r, 400)); });
  return box;
}

describe("versions", () => {
  it("orders pre-releases before their release", () => {
    expect(compareVersions("1.3.0-beta.1", "1.3.0")).toBeLessThan(0);
    expect(compareVersions("1.3.0-beta.2", "1.3.0-beta.10")).toBeLessThan(0);
    expect(compareVersions("1.3.0-beta.1", "1.2.9")).toBeGreaterThan(0);
    expect(compareVersions("1.3.0-alpha", "1.3.0-beta")).toBeLessThan(0);
    expect(compareVersions("1.0.10", "1.0.2")).toBeGreaterThan(0);
    expect(compareVersions("2.0.0+build", "2.0.0")).toBe(0);
  });

  it("names channels from the tags, stable first", () => {
    expect(channelOf("1.2.0")).toBe("stable");
    expect(channelOf("1.3.0-beta.2")).toBe("beta");
    expect(channelOf("1.3.0-RC1")).toBe("rc");
    expect(channelOf("1.3.0-1")).toBe("pre");
    expect(channelOf(null)).toBe("stable");
    expect(channelsOf([{ channel: "rc" }, { channel: "beta" }, { channel: "stable" }, {}] as never)).toEqual(["stable", "beta", "rc"]);
  });
});

describe("the Versions tab", () => {
  it("has a tab per channel and lists the installed channel first", async () => {
    const box = await show(mod("1.2.0"));
    const tabs = [...box.querySelectorAll(".versions-channels button")].map((b) => b.textContent);
    expect(tabs).toEqual(["Stable", "Beta"]);
    const versions = [...box.querySelectorAll(".mod-log-entry b")].map((b) => b.textContent);
    expect(versions).toEqual(["1.2.0", "1.1.0"]);
    act(() => box.querySelectorAll<HTMLButtonElement>(".versions-channels button")[1].click());
    expect([...box.querySelectorAll(".mod-log-entry b")].map((b) => b.textContent)).toEqual(["1.3.0-beta.1"]);
  });

  it("refreshes from GitHub at most once every 20 seconds", async () => {
    const box = await show(mod("1.2.0"));
    const asked = () => call.mock.calls.filter((c) => c[0] === "mod_releases").length;
    const before = asked();
    const button = box.querySelector<HTMLButtonElement>(".versions-refresh")!;
    await act(async () => { button.click(); await Promise.resolve(); });
    expect(asked()).toBe(before + 1);
    expect(button.classList.contains("cooling")).toBe(true);
    expect(button.dataset.tip).toMatch(/^Refresh again in \d+ s$/);
    await act(async () => { button.click(); await Promise.resolve(); });
    expect(asked()).toBe(before + 1);
  });

  it("puts a switch button above the notes of an unfolded release, none on the installed one", async () => {
    const box = await show(mod("1.2.0"));
    const heads = box.querySelectorAll<HTMLButtonElement>(".mod-log-entry .upd-head");
    act(() => heads[0].click());
    expect(box.querySelector(".mod-log-entry.installed .versions-switch")).toBeNull();
    act(() => heads[1].click());
    const body = box.querySelectorAll(".mod-log-entry")[1].querySelector(".upd-body")!;
    const button = body.querySelector<HTMLButtonElement>(".versions-switch")!;
    expect(button.textContent).toBe("Switch to 1.1.0");
    expect(body.firstElementChild).toBe(button);
    await act(async () => { button.click(); });
    expect(call).toHaveBeenCalledWith("install_mod_version", { id: "me.mod", tag: "v1.1.0" });
    expect(found["me.mod"]).toBe("1.2.0");
  });
});
