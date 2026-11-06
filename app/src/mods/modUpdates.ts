// Mod updates from GitHub (design: 2026-10-30-obsidian-mod-updates-design.md). A check runs at startup
// and when the player asks, for one mod, a folder or the whole list, and what it finds lasts until the
// app closes. Nothing installs by itself. The core keeps each answer: the startup check takes one from
// the last few hours instead of asking GitHub again, and a check the player starts always asks.
import { useSyncExternalStore } from "react";
import { call, errorText, setSnapshot } from "../api";
import { withTask } from "../busy";
import { t } from "../i18n";
import { notify } from "../notices";
import type { CmdError, ImportBatch } from "../types";
import { importMessage } from "./dropImport";

type Checked = { id: string; version: string | null; error: CmdError | null };
/** `checking` holds the mods of the check under way; one runs at a time. */
export type ModUpdates = { checking: string[]; found: Record<string, string> };

let state: ModUpdates = { checking: [], found: {} };
const listeners = new Set<() => void>();
const set = (patch: Partial<ModUpdates>) => {
  state = { ...state, ...patch };
  listeners.forEach((f) => f());
};

/** The version to offer for a mod. A check goes stale once a newer copy has been imported by hand. */
export const offered = (found: ModUpdates["found"], mod: { id: string; version: string | null }): string | null =>
  found[mod.id] && found[mod.id] !== mod.version ? found[mod.id] : null;

/** Where a mod's changelog is, when it comes from a GitHub repository. */
export function changelogUrl(source: string | null): string | null {
  const repo = source?.trim().match(/^https:\/\/github\.com\/[^/\s]+\/[^/\s]+/)?.[0];
  return repo ? `${repo.replace(/\.git$/, "")}/releases` : null;
}

/** The notice for a finished check; `what` names the mod or folder when it wasn't the whole list. */
export function checkMessage(count: number, what?: string): string {
  if (count === 0) return what ? t("app.modUpdates.noneFor", what) : t("app.modUpdates.none");
  return count === 1 ? t("app.modUpdates.one") : t("app.modUpdates.many", count);
}

/** At `startup` it speaks only when it finds something: no update, or no network, is no news. */
export async function checkModUpdates(ids: string[], names: Map<string, string>, { what, startup = false }: { what?: string; startup?: boolean } = {}) {
  if (state.checking.length) return;
  set({ checking: ids });
  try {
    const list = await withTask(t("app.task.check_mod_updates"), () => call<Checked[]>("check_mod_updates", { ids, fresh: !startup }));
    // What this check didn't ask about keeps its last answer.
    const found = Object.fromEntries(Object.entries(state.found).filter(([id]) => !ids.includes(id)));
    for (const c of list) if (c.version) found[c.id] = c.version;
    const count = list.filter((c) => c.version).length;
    set({ checking: [], found });
    if (startup) {
      if (count) notify("info", checkMessage(count));
      return;
    }
    const failed = list.filter((c) => c.error);
    if (failed.length < list.length) notify("info", checkMessage(count, what));
    // The rate limit is one problem, however many mods it stopped.
    const limit = failed.find((c) => c.error?.code === "UpdateLimited");
    if (limit) notify("error", errorText(limit.error));
    for (const c of failed) if (c !== limit && c.error?.code !== "UpdateLimited") notify("error", t("app.modUpdates.failed", names.get(c.id) ?? c.id, errorText(c.error)));
  } catch (e) {
    set({ checking: [] });
    if (!startup) notify("error", errorText(e));
  }
}

export async function updateMod(id: string, name: string) {
  try {
    const b = await withTask(t("app.task.update_mod", name), () => call<ImportBatch>("update_mod", { id }), true);
    setSnapshot(b.snapshot);
    const found = { ...state.found };
    delete found[id];
    set({ found });
    for (const r of b.results) notify("info", importMessage(r));
  } catch (e) {
    notify("error", t("app.import.failed", name, errorText(e)));
  }
}

/** One at a time: each update changes the library. */
export async function updateMods(ids: string[], names: Map<string, string>) {
  for (const id of ids) await updateMod(id, names.get(id) ?? id);
}

export const useModUpdates = () =>
  useSyncExternalStore((f) => { listeners.add(f); return () => { listeners.delete(f); }; }, () => state);
