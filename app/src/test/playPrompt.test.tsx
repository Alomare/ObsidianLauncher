import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import en from "../../../locales/en.json";
import { t } from "../i18n";
import type { AppSettings, ModRow, Snapshot } from "../types";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke, convertFileSrc: (p: string) => p }));
vi.mock("@tauri-apps/api/event", () => ({ listen: () => Promise.resolve(() => {}) }));
vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: () => ({ onDragDropEvent: () => Promise.resolve(() => {}) }) }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: () => Promise.resolve(null) }));
vi.mock("../opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));
const { default: App } = await import("../App");
const { setSnapshot } = await import("../api");
const { currentSettings, DEFAULTS } = await import("../settings");
const { asksBeforePlay } = await import("../PlayPrompt");
const { go } = await import("../nav");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mod = (id: string, on: boolean): ModRow => ({
  id, name: { en: id }, description: {}, authors: [], version: "1.0.0", scope: "client", on, options: [], keybinds: [], files: [],
  import: null, source: null, updatable: false, image: null, folder: null, requires: [], parts: [], optionGroups: [], optionPreviews: [], size: 0, deployed: false, deployedSize: null,
});
/** `deployed` is what the app tracks of the last deploy; `on` is the switch of the one mod in the list. */
const snap = ({ deployed, on }: { deployed: boolean; on: boolean }): Snapshot => ({
  mods: [mod("a.one", on)], folders: [], profile: "Default", profiles: ["Default"],
  profileRows: [{ name: "Default", on: on ? 1 : 0, blockPublic: false, guarded: false, image: null, color: null }],
  blockPublic: false, blockAchievements: false, freezeProgress: false, hideClientMods: false, loader: false, sandbox: null, conflicts: [], changes: on === deployed ? "UpToDate" : "Waiting", deployed, modded: deployed,
  game: "supported", gameChanges: [], gameDir: "C:\\Game", crashCulprit: null, broken: [], damagedProfiles: [], uncleanEnd: null, portable: false, version: "1.3.1",
});
const settings = (askVanillaPlay: boolean): AppSettings => ({ ...DEFAULTS, language: "en", updates: "manual", askVanillaPlay });

describe("whether Play asks first", () => {
  it("asks only when nothing is deployed and the question is on", () => {
    expect(asksBeforePlay(snap({ deployed: false, on: false }), settings(true))).toBe(true);
    expect(asksBeforePlay(snap({ deployed: true, on: true }), settings(true))).toBe(false);
    expect(asksBeforePlay(snap({ deployed: false, on: false }), settings(false))).toBe(false);
  });

  it("goes by the last deploy, not by the switches in the list", () => {
    expect(asksBeforePlay(snap({ deployed: false, on: true }), settings(true))).toBe(true);
    expect(asksBeforePlay(snap({ deployed: true, on: false }), settings(true))).toBe(false);
  });

  it("is on in a new install", () => expect(DEFAULTS.askVanillaPlay).toBe(true));
});

// The window, with a Rust side that answers from these values and keeps what Settings saves.
let saved: AppSettings;
let root: Root | undefined;
afterEach(() => {
  act(() => root?.unmount());
  document.body.replaceChildren();
  invoke.mockReset();
});
const sent = (cmd: string) => invoke.mock.calls.filter((c) => c[0] === cmd);

async function open(s: Snapshot, a: AppSettings) {
  saved = a;
  invoke.mockImplementation(async (cmd: string, args?: { settings?: AppSettings }) => {
    if (cmd === "get_app_settings") return saved;
    if (cmd === "set_app_settings") return (saved = args!.settings!);
    if (cmd === "snapshot" || cmd === "play") return s;
    if (cmd === "system_info") return { dataDir: "C:\\Data", backups: [], chosenGameDir: null, logsDir: null, foreign: [], removedPatches: null };
    return null;
  });
  setSnapshot(s);
  go("mods");
  const host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root!.render(<App />));
  await vi.waitFor(() => expect(currentSettings()).toEqual(a));
  await act(async () => {});
}
const press = async (el: Element | null | undefined) => {
  expect(el).toBeTruthy();
  await act(async () => (el as HTMLElement).click());
};
const playButton = () => document.querySelector(".nav .actions .btn.play");
const prompt = () => document.querySelector(".modal.confirm");
const answer = (word: string) => [...document.querySelectorAll(".modal-foot .btn")].find((b) => b.textContent === word);
const dontAsk = () => document.querySelector(".modal.confirm .check-line");

describe("Play with nothing deployed", () => {
  it("asks first, with the question, Yes and No, and plays only on Yes", async () => {
    await open(snap({ deployed: false, on: true }), settings(true));
    await press(playButton());
    expect(prompt()?.querySelector("p")?.textContent).toBe("No mods are deployed, so the game is vanilla. Play anyway?");
    expect([...document.querySelectorAll(".modal-foot .btn")].map((b) => b.textContent)).toEqual(["No", "Yes"]);
    expect(dontAsk()?.textContent).toBe("Don't ask again");
    expect(dontAsk()?.getAttribute("aria-checked")).toBe("false");
    expect(document.activeElement?.textContent).toBe("Yes");
    expect(sent("play")).toHaveLength(0);
    await press(answer("Yes"));
    expect(prompt()).toBeNull();
    expect(sent("play")).toHaveLength(1);
    expect(sent("set_app_settings")).toHaveLength(0);
  });

  it("No, Esc and a click outside start nothing", async () => {
    await open(snap({ deployed: false, on: false }), settings(true));
    await press(playButton());
    await press(answer("No"));
    expect(prompt()).toBeNull();
    await press(playButton());
    await act(async () => window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })));
    expect(prompt()).toBeNull();
    await press(playButton());
    await act(async () => document.querySelector(".modal-back")!.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true })));
    expect(prompt()).toBeNull();
    expect(sent("play")).toHaveLength(0);
  });

  it("Don't ask again with Yes plays and turns the question off, so the next Play goes at once", async () => {
    await open(snap({ deployed: false, on: true }), settings(true));
    await press(playButton());
    await press(dontAsk());
    expect(dontAsk()?.getAttribute("aria-checked")).toBe("true");
    await press(answer("Yes"));
    expect(sent("set_app_settings").map((c) => c[1].settings.askVanillaPlay)).toEqual([false]);
    expect(sent("play")).toHaveLength(1);
    await vi.waitFor(() => expect(currentSettings()?.askVanillaPlay).toBe(false));
    await press(playButton());
    expect(prompt()).toBeNull();
    expect(sent("play")).toHaveLength(2);
  });

  it("Don't ask again with No is dropped, and the next Play asks again", async () => {
    await open(snap({ deployed: false, on: true }), settings(true));
    await press(playButton());
    await press(dontAsk());
    await press(answer("No"));
    expect(sent("set_app_settings")).toHaveLength(0);
    await press(playButton());
    expect(prompt()).not.toBeNull();
    expect(dontAsk()?.getAttribute("aria-checked")).toBe("false");
    expect(sent("play")).toHaveLength(0);
  });
});

describe("Play that doesn't ask", () => {
  it("starts at once when mods are deployed, even with every switch in the list off", async () => {
    await open(snap({ deployed: true, on: false }), settings(true));
    await press(playButton());
    expect(prompt()).toBeNull();
    expect(sent("play")).toHaveLength(1);
  });

  it("starts at once when the question is turned off", async () => {
    await open(snap({ deployed: false, on: true }), settings(false));
    await press(playButton());
    expect(prompt()).toBeNull();
    expect(sent("play")).toHaveLength(1);
  });
});

describe("the Settings row", () => {
  it("shows the question on, turns it off and back on, and saves each change", async () => {
    await open(snap({ deployed: false, on: true }), settings(true));
    await act(async () => go("settings"));
    const row = () => [...document.querySelectorAll<HTMLButtonElement>(".set-sec button[role=switch]")].find((b) => b.getAttribute("aria-label") === t("app.settings.askVanilla"))!;
    expect(row().getAttribute("aria-checked")).toBe("true");
    const group = row().closest(".set-sec")!;
    expect(group.querySelector("h2 .cap")!.textContent).toBe(t("app.settings.play"));
    expect(group.querySelector(".help-q")).not.toBeNull();
    await press(row());
    await vi.waitFor(() => expect(row().getAttribute("aria-checked")).toBe("false"));
    await press(row());
    await vi.waitFor(() => expect(row().getAttribute("aria-checked")).toBe("true"));
    expect(sent("set_app_settings").map((c) => c[1].settings.askVanillaPlay)).toEqual([false, true]);
    expect(saved.askVanillaPlay).toBe(true);
  });
});

describe("the text of the question", () => {
  // Public text: no "its", no em dashes, no "...", no first person, and the buttons in Title Case.
  const keys = ["app.yes", "app.no", "app.play.vanilla", "app.play.dontAsk", "app.settings.play", "app.settings.askVanilla", "app.settings.askVanillaHint"];
  it("keeps the writing rules", () => {
    for (const k of keys) {
      const v = (en as Record<string, string>)[k];
      expect(v, k).toBeTruthy();
      expect(v, k).not.toMatch(/\bits\b|—|…|\.\.\.|\b(I|me|my|we|our)\b/i);
    }
    for (const k of ["app.yes", "app.no"]) expect((en as Record<string, string>)[k]).toMatch(/^[A-Z]/);
  });
});
