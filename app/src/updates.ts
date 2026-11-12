// Obsidian's own updates through the updater plugin: it reads latest.json from the newest GitHub
// release, compares versions, downloads the installer, checks the signature and runs it (the app
// closes while it installs and opens again on the new version). On Linux the AppImage or the package
// is replaced while the app runs, so the app starts again after. Portable copies never install:
// they show the new version with a link to the releases page.
import { check, type Update } from "@tauri-apps/plugin-updater";
import { useSyncExternalStore } from "react";
import { call } from "./api";
import type { AppSettings } from "./types";

export type UpdateState = {
  phase: "idle" | "checking" | "none" | "found" | "failed" | "downloading" | "installing";
  version: string | null;
  /** What the new version changes, from its release. */
  notes: string | null;
  error: string | null;
  got: number;
  total: number;
};

let state: UpdateState = { phase: "idle", version: null, notes: null, error: null, got: 0, total: 0 };
let found: Update | null = null;
const listeners = new Set<() => void>();
const set = (patch: Partial<UpdateState>) => {
  state = { ...state, ...patch };
  listeners.forEach((f) => f());
};

// latest.json is frozen at the build, so the text on GitHub replaces it when it can be read.
const liveNotes = (version: string) =>
  call<string | null>("release_notes", { version })
    .then((notes) => { if (notes && state.version === version) set({ notes: notes.trim() }); })
    .catch(() => {});

export async function checkForUpdate(): Promise<boolean> {
  if (state.phase === "checking" || state.phase === "downloading" || state.phase === "installing") return state.phase !== "checking" && !!found;
  set({ phase: "checking", error: null });
  try {
    found = await check({ timeout: 20000 });
    set(found ? { phase: "found", version: found.version, notes: found.body?.trim() || null } : { phase: "none", version: null, notes: null });
    if (found) void liveNotes(found.version);
    return !!found;
  } catch (e) {
    set({ phase: "failed", error: e instanceof Error ? e.message : String(e) });
    return false;
  }
}

export async function installUpdate() {
  if (!found) return;
  set({ phase: "downloading", got: 0, total: 0, error: null });
  try {
    await found.downloadAndInstall((ev) => {
      if (ev.event === "Started") set({ total: ev.data.contentLength ?? 0 });
      else if (ev.event === "Progress") set({ got: state.got + ev.data.chunkLength });
      else set({ phase: "installing" });
    });
    // Reached on Linux only: the Windows installer has closed the app by now.
    await call("restart_app");
  } catch (e) {
    set({ phase: "found", error: e instanceof Error ? e.message : String(e) });
  }
}

/** The check at startup. Returns "ask" when the window should ask before installing. */
export async function startupUpdate(mode: AppSettings["updates"], portable: boolean): Promise<"ask" | null> {
  if (!(await checkForUpdate()) || portable || mode === "manual") return null;
  if (mode === "auto") {
    await installUpdate();
    return null;
  }
  return "ask";
}

export const updateState = () => state;

export type ChangelogEntry = { version: string; date: string | null; notes: string; url: string | null; when: "new" | "installed" | "older" };

/** Orders plain versions: 1.0.2 before 1.0.10. Below zero when a comes first. */
export function compareVersions(a: string, b: string) {
  const parts = (v: string) => v.split(".").map((n) => parseInt(n, 10) || 0);
  const x = parts(a), y = parts(b);
  for (let i = 0; i < 3; i++) if ((x[i] ?? 0) !== (y[i] ?? 0)) return (x[i] ?? 0) - (y[i] ?? 0);
  return 0;
}

// The notes this build shipped with, so the changelog reads without a connection.
const shipped = Object.entries(import.meta.glob("../../docs/releases/*.md", { query: "?raw", import: "default", eager: true }) as Record<string, string>)
  .map(([path, notes]) => ({ version: path.replace(/^.*\/|\.md$/g, ""), notes }));

/** Every version there are notes for, newest first. GitHub's text wins, as it can be edited there. */
export function changelog(remote: Release[], current: string, local: { version: string; notes: string }[] = shipped): ChangelogEntry[] {
  const when = (v: string): ChangelogEntry["when"] => {
    const c = compareVersions(v, current);
    return c > 0 ? "new" : c === 0 ? "installed" : "older";
  };
  const all = new Map<string, ChangelogEntry>();
  // Notes for a version past this one are a draft for the next release.
  for (const r of local) if (when(r.version) !== "new") all.set(r.version, { version: r.version, date: null, notes: r.notes.trim(), url: null, when: when(r.version) });
  for (const r of remote) {
    all.set(r.version, { version: r.version, date: r.date, url: r.url, notes: r.notes.trim() || all.get(r.version)?.notes || "", when: when(r.version) });
  }
  return [...all.values()].sort((a, b) => compareVersions(b.version, a.version));
}

export type Release = { version: string; date: string | null; notes: string; url: string };
type ReleaseList = { list: Release[]; failed: boolean; loaded: boolean };

let releases: ReleaseList = { list: [], failed: false, loaded: false };
const releaseListeners = new Set<() => void>();

/** Asks GitHub once per launch, when the changelog first shows. */
export function loadReleases() {
  if (releases.loaded) return;
  releases = { ...releases, loaded: true };
  call<Release[]>("release_list")
    .then((list) => { releases = { ...releases, list, failed: false }; })
    .catch(() => { releases = { ...releases, failed: true }; })
    .finally(() => releaseListeners.forEach((f) => f()));
}

export const useReleases = () =>
  useSyncExternalStore((f) => { releaseListeners.add(f); return () => { releaseListeners.delete(f); }; }, () => releases);

export const useUpdates = () =>
  useSyncExternalStore((f) => { listeners.add(f); return () => { listeners.delete(f); }; }, updateState);
