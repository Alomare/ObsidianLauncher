// The Store list (core/src/store.rs): read once per launch when the Store tab first opens, and again
// on Try again. Installs go one at a time, requirements from the Store first.
import { useSyncExternalStore } from "react";
import { call, errorText, setSnapshot } from "../api";
import { withTask } from "../busy";
import { t } from "../i18n";
import { importMessage } from "../mods/dropImport";
import { notify } from "../notices";
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
};
export type StoreState = { mods: StoreMod[] | null; offline: boolean; loading: boolean; error: string | null; installing: string[] };

let state: StoreState = { mods: null, offline: false, loading: false, error: null, installing: [] };
const listeners = new Set<() => void>();
const set = (patch: Partial<StoreState>) => {
  state = { ...state, ...patch };
  listeners.forEach((f) => f());
};

export const useStore = () =>
  useSyncExternalStore((f) => { listeners.add(f); return () => { listeners.delete(f); }; }, () => state);

/** Forgets the list, as at launch. For tests. */
export const forgetStore = () => set({ mods: null, offline: false, loading: false, error: null, installing: [] });

/** Reads the list, unless it is on the way or already read (`again` reads it anyway). A failed read
 *  keeps the list already shown. */
export async function loadStore(again = false) {
  if (state.loading || (state.mods && !again)) return;
  set({ loading: true, error: null });
  try {
    const got = await call<{ mods: StoreMod[]; offline: boolean }>("store_index");
    set({ mods: got.mods, offline: got.offline, loading: false });
  } catch (e) {
    set({ loading: false, error: errorText(e) });
  }
}

export type StoreMark = "none" | "installed" | "update" | "installing";

/** Where a Store mod stands in the library. */
export function markOf(m: StoreMod, mods: ModRow[], installing: string[]): StoreMark {
  if (installing.includes(m.id)) return "installing";
  const row = mods.find((r) => r.id === m.id);
  if (!row) return "none";
  return row.version && compareVersions(m.version, row.version) > 0 ? "update" : "installed";
}

/** Installs `m`, after the mods it requires that the Store has and the library doesn't. */
export async function install(m: StoreMod, all: StoreMod[], mods: ModRow[]) {
  const needed = m.requires.map((id) => all.find((x) => x.id === id)).filter((x): x is StoreMod => !!x && !mods.some((r) => r.id === x.id));
  for (const one of [...needed, m]) {
    if (state.installing.includes(one.id)) continue;
    set({ installing: [...state.installing, one.id] });
    try {
      const b = await withTask(t("app.store.installing", one.name), () => call<ImportBatch>("store_install", { id: one.id }), true);
      setSnapshot(b.snapshot);
      for (const r of b.results) notify(r.ok ? "info" : "error", importMessage(r));
    } catch (e) {
      notify("error", t("app.import.failed", one.name, errorText(e)));
      set({ installing: state.installing.filter((x) => x !== one.id) });
      return;
    }
    set({ installing: state.installing.filter((x) => x !== one.id) });
  }
}
