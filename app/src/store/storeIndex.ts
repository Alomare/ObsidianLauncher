// The Store list (core/src/store.rs): read once per launch when the Store tab first opens, and again
// on Try again. Installs go one at a time, requirements from the Store first.
import { call, errorText, setSnapshot } from "../api";
import { withTask } from "../busy";
import { t } from "../i18n";
import { importMessage } from "../mods/dropImport";
import { notify } from "../notices";
import { createStore } from "../store";
import { compareVersions } from "../updates";
import type { ImportBatch, ModRow, Scope } from "../types";

export type StoreOption = { label: string; description: string; group: string | null };
export type StoreVersion ={ version: string; date: string | null; channel: string; notes: string };
/** `icon` and `banner` are files on this PC, kept in the data folder. */
export type StoreMod = {
  id: string; name: string; summary: string; description: string; version: string; early: boolean;
  scope: Scope; requires: string[]; options: number; option_list: StoreOption[]; option_groups: { id: string; label: string }[]; repo: string; kit: boolean;
  icon: string | null; banner: string | null; released: string | null; added: string | null; versions: StoreVersion[];
  /** Missing from indexes built before 2026-11-14. */
  authors?: string[]; verified?: boolean;
  /** Missing from indexes built before 2026-11-15. `private`: early access, from a private repository. */
  stars?: number; downloads?: number; private?: boolean;
  /** Hybrid Scopes: the scope of each secondary scope, as declared. Missing from indexes built before 2026-11-23. */
  secondary_scopes?: string[];
};

/** The secondary scope a scope tag shows beside the primary one: of the secondary scopes unlike the
 *  primary, the scope most of them have, the first declared on a tie. None when all match the primary. */
export function secondScope(m: Pick<StoreMod, "scope" | "secondary_scopes">): Scope | null {
  const others = (m.secondary_scopes ?? []).filter((s): s is Scope => (s === "client" || s === "session") && s !== m.scope);
  let best: Scope | null = null, most = 0;
  for (const s of others) {
    const n = others.filter((x) => x === s).length;
    if (n > most) { best = s; most = n; }
  }
  return best;
}
/** The list as it stands. A reload of a list already shown keeps it on screen: `refresh` says how that
 *  reload went, and `stale` that another was asked for while it ran, so it runs again after. */
export type StoreIndex =
  | { kind: "empty" }
  | { kind: "loading" }
  | { kind: "failed"; error: string }
  | { kind: "ready"; mods: StoreMod[]; offline: boolean; refresh: "idle" | "running" | "failed"; error: string | null; stale: boolean };
export type StoreState = { index: StoreIndex; installing: string[]; starred: string[] | null };

const START: StoreState = { index: { kind: "empty" }, installing: [], starred: null };
const store = createStore<StoreState>(START);
const set = (patch: Partial<StoreState>) => store.set((s) => ({ ...s, ...patch }));
const setIndex = (index: StoreIndex) => set({ index });
const state = () => store.get();

export const useStore = () => store.use();

/** The mods of the list, when it has been read. */
export const modsOf = (index: StoreIndex): StoreMod[] | null => (index.kind === "ready" ? index.mods : null);

/** Forgets the list, as at launch. For tests. */
export const forgetStore = () => { again = false; store.set(START); };

/** A reload asked for while the first read runs, when there is no list yet to mark. */
let again = false;

/** Reads the list, unless it is already read (`again` reads it anyway). A reload asked for while one
 *  runs runs after it. A failed reload keeps the list already shown and says so beside it. */
export async function loadStore(reload = false) {
  const now = state().index;
  if (now.kind === "loading" || (now.kind === "ready" && now.refresh === "running")) {
    if (reload) {
      if (now.kind === "ready") setIndex({ ...now, stale: true });
      else again = true;
    }
    return;
  }
  if (now.kind === "ready" && !reload) return;
  const shown = now.kind === "ready" ? now : null;
  setIndex(shown ? { ...shown, refresh: "running", error: null, stale: false } : { kind: "loading" });
  try {
    const got = await call<{ mods: StoreMod[]; offline: boolean }>("store_index");
    const after = state().index;
    setIndex({ kind: "ready", mods: got.mods, offline: got.offline, refresh: "idle", error: null, stale: after.kind === "ready" && after.stale });
  } catch (e) {
    const after = state().index;
    setIndex(after.kind === "ready" ? { ...after, refresh: "failed", error: errorText(e) } : { kind: "failed", error: errorText(e) });
  }
  const end = state().index;
  if (again || (end.kind === "ready" && end.stale)) {
    again = false;
    if (end.kind === "ready") setIndex({ ...end, stale: false });
    await loadStore(true);
  }
}

/** The star count of a mod after the player starred or unstarred it, until the index catches up. */
export function setStars(id: string, stars: number) {
  const ix = state().index;
  if (ix.kind === "ready") setIndex({ ...ix, mods: ix.mods.map((m) => (m.id === id ? { ...m, stars: Math.max(0, stars) } : m)) });
}

/** The repositories the signed-in player starred (lower case), or null signed out or not read yet. */
export async function loadStarred(signedIn: boolean) {
  if (!signedIn) { set({ starred: null }); return; }
  try {
    set({ starred: await call<string[]>("github_starred_repos") });
  } catch {
    set({ starred: null });
  }
}

export const isStarred = (m: StoreMod, starred: string[] | null) => !!starred?.includes(m.repo.toLowerCase());

/** Stars or unstars `m` on GitHub, and counts it here at once. */
export async function toggleStar(m: StoreMod) {
  const on = !isStarred(m, state().starred);
  await call("github_star", { repo: m.repo, on });
  const repo = m.repo.toLowerCase();
  set({ starred: on ? [...(state().starred ?? []), repo] : (state().starred ?? []).filter((r) => r !== repo) });
  setStars(m.id, (m.stars ?? 0) + (on ? 1 : -1));
}

export type StoreMark = "none" | "installed" | "update" | "installing";

/** Where a Store mod stands in the library. */
export function markOf(m: StoreMod, mods: ModRow[] | ReadonlyMap<string, ModRow>, installing: string[]): StoreMark {
  if (installing.includes(m.id)) return "installing";
  const row = Array.isArray(mods) ? mods.find((r) => r.id === m.id) : (mods as ReadonlyMap<string, ModRow>).get(m.id);
  if (!row) return "none";
  return row.version && compareVersions(m.version, row.version) > 0 ? "update" : "installed";
}

/** Installs `m`, after the mods it requires that the Store has and the library doesn't. */
export async function install(m: StoreMod, all: StoreMod[], mods: ModRow[]) {
  const needed = m.requires.map((id) => all.find((x) => x.id === id)).filter((x): x is StoreMod => !!x && !mods.some((r) => r.id === x.id));
  for (const one of [...needed, m]) {
    if (state().installing.includes(one.id)) continue;
    set({ installing: [...state().installing, one.id] });
    try {
      const b = await withTask(t("app.store.installing", one.name), () => call<ImportBatch>("store_install", { id: one.id }), true);
      setSnapshot(b.snapshot);
      for (const r of b.results) notify(r.ok ? "info" : "error", importMessage(r));
    } catch (e) {
      notify("error", t("app.import.failed", one.name, errorText(e)));
      set({ installing: state().installing.filter((x) => x !== one.id) });
      return;
    }
    set({ installing: state().installing.filter((x) => x !== one.id) });
  }
}
