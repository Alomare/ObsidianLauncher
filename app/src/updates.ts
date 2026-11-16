// Obsidian's own updates through the updater plugin: it reads latest.json from the newest GitHub
// release, compares versions, downloads the installer, checks the signature and runs it (the app
// closes while it installs and opens again on the new version). On Linux the AppImage or the package
// is replaced while the app runs, so the app starts again after. Portable copies never install:
// they show the new version with a link to the releases page.
import { check, type Update } from "@tauri-apps/plugin-updater";
import { call } from "./api";
import { createStore } from "./store";
import type { AppSettings } from "./types";

/** Where an update stands. Each phase carries only what it has: a version once one is found, the bytes
 *  while it downloads, the reason a check or an install failed. */
export type UpdateState =
  | { phase: "idle" }
  | { phase: "checking" }
  | { phase: "none" }
  | { phase: "failed"; error: string }
  /** `error`: the last install of this version failed. `notes`: what the new version changes, from its release. */
  | { phase: "found"; version: string; notes: string | null; error: string | null }
  | { phase: "downloading"; version: string; notes: string | null; got: number; total: number }
  | { phase: "installing"; version: string; notes: string | null };

type Action =
  | { type: "check" }
  | { type: "checked"; version: string; notes: string | null }
  | { type: "noUpdate" }
  | { type: "checkFailed"; error: string }
  | { type: "notes"; version: string; notes: string }
  | { type: "download" }
  | { type: "size"; total: number }
  | { type: "chunk"; bytes: number }
  | { type: "install" }
  | { type: "installFailed"; error: string };

/** The next state, or the same one when the action does not apply to the phase. */
export function step(s: UpdateState, a: Action): UpdateState {
  switch (a.type) {
    case "check": return s.phase === "downloading" || s.phase === "installing" ? s : { phase: "checking" };
    case "checked": return { phase: "found", version: a.version, notes: a.notes, error: null };
    case "noUpdate": return { phase: "none" };
    case "checkFailed": return { phase: "failed", error: a.error };
    case "notes": return "version" in s && s.version === a.version ? { ...s, notes: a.notes } : s;
    case "download": return s.phase === "found" ? { phase: "downloading", version: s.version, notes: s.notes, got: 0, total: 0 } : s;
    case "size": return s.phase === "downloading" ? { ...s, total: a.total } : s;
    case "chunk": return s.phase === "downloading" ? { ...s, got: s.got + a.bytes } : s;
    case "install": return s.phase === "downloading" ? { phase: "installing", version: s.version, notes: s.notes } : s;
    case "installFailed": return "version" in s ? { phase: "found", version: s.version, notes: s.notes, error: a.error } : s;
  }
}

const store = createStore<UpdateState>({ phase: "idle" });
const dispatch = (a: Action) => store.set((s) => step(s, a));
let found: Update | null = null;
const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

// latest.json is frozen at the build, so the text on GitHub replaces it when it can be read.
const liveNotes = (version: string) =>
  call<string | null>("release_notes", { version })
    .then((notes) => { if (notes) dispatch({ type: "notes", version, notes: notes.trim() }); })
    .catch(() => {});

export async function checkForUpdate(): Promise<boolean> {
  const now = store.get().phase;
  if (now === "checking" || now === "downloading" || now === "installing") return now !== "checking" && !!found;
  dispatch({ type: "check" });
  try {
    found = await check({ timeout: 20000 });
    dispatch(found ? { type: "checked", version: found.version, notes: found.body?.trim() || null } : { type: "noUpdate" });
    if (found) void liveNotes(found.version);
    return !!found;
  } catch (e) {
    dispatch({ type: "checkFailed", error: message(e) });
    return false;
  }
}

/** Downloads and installs the version found. Only from "found": a second click while it downloads does nothing. */
export async function installUpdate() {
  if (!found || store.get().phase !== "found") return;
  dispatch({ type: "download" });
  try {
    await found.downloadAndInstall((ev) => {
      if (ev.event === "Started") dispatch({ type: "size", total: ev.data.contentLength ?? 0 });
      else if (ev.event === "Progress") dispatch({ type: "chunk", bytes: ev.data.chunkLength });
      else dispatch({ type: "install" });
    });
    // Reached on Linux only: the Windows installer has closed the app by now.
    await call("restart_app");
  } catch (e) {
    dispatch({ type: "installFailed", error: message(e) });
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

export const updateState = () => store.get();

export type ChangelogEntry = { version: string; date: string | null; notes: string; url: string | null; when: "new" | "installed" | "older" };

/** Orders versions as semver does: 1.0.2 before 1.0.10, and 1.1.0-beta.2 before 1.1.0-beta.10 and
 *  before 1.1.0. Build metadata after `+` doesn't count. Below zero when a comes first. */
export function compareVersions(a: string, b: string) {
  const split = (v: string) => {
    const plain = v.split("+")[0];
    const dash = plain.indexOf("-");
    const core = (dash < 0 ? plain : plain.slice(0, dash)).split(".").map((n) => parseInt(n, 10) || 0);
    return { core, pre: dash < 0 ? [] : plain.slice(dash + 1).split(".") };
  };
  const x = split(a), y = split(b);
  for (let i = 0; i < 3; i++) if ((x.core[i] ?? 0) !== (y.core[i] ?? 0)) return (x.core[i] ?? 0) - (y.core[i] ?? 0);
  if (!x.pre.length || !y.pre.length) return (x.pre.length ? -1 : 0) + (y.pre.length ? 1 : 0);
  for (let i = 0; i < Math.max(x.pre.length, y.pre.length); i++) {
    const p = x.pre[i], q = y.pre[i];
    if (p === undefined) return -1;
    if (q === undefined) return 1;
    if (p === q) continue;
    const pn = /^\d+$/.test(p), qn = /^\d+$/.test(q);
    if (pn && qn) return parseInt(p, 10) - parseInt(q, 10);
    if (pn !== qn) return pn ? -1 : 1;
    return p < q ? -1 : 1;
  }
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

/** `tag` and `channel` come with a mod's releases (the Versions tab); the app's own list has neither. */
export type Release = { version: string; date: string | null; notes: string; url: string; tag?: string; channel?: string };
type ReleaseList = { list: Release[]; failed: boolean; loaded: boolean };

const releases = createStore<ReleaseList>({ list: [], failed: false, loaded: false });

/** Asks GitHub once per launch, when the changelog first shows. */
export function loadReleases() {
  if (releases.get().loaded) return;
  releases.set((r) => ({ ...r, loaded: true }));
  call<Release[]>("release_list")
    .then((list) => releases.set((r) => ({ ...r, list, failed: false })))
    .catch(() => releases.set((r) => ({ ...r, failed: true })));
}

export const useReleases = () => releases.use();

/** The whole state, or the part `select` picks: the App reads only the phase and the version, so the
 *  download progress renders only the window that shows it. */
export const useUpdates = <S = UpdateState>(select?: (u: UpdateState) => S) => store.use(select);
/** The version on offer, in the phases that have one. */
export const versionOf = (u: UpdateState) => ("version" in u ? u.version : null);
