// The checked saves of the Saves tab and what can be done to several at once. Only Sandbox saves take
// a check: Official saves switch, snapshot and delete one at a time, the delete behind the typed name.
import { fileSafe } from "./logic";
import type { SaveRow } from "./types";

/** The saves a check can go on, in the order the list shows them: Sandbox saves, unless the group is folded. */
export const checkable = (saves: SaveRow[], sandboxFolded: boolean): string[] =>
  sandboxFolded ? [] : saves.filter((s) => s.kind === "sandbox").map((s) => s.id);

/** What Ctrl+A or the box in the header does: checks every shown save, or unchecks them when all are checked. */
export function checkAll(shown: string[], checked: Set<string>): Set<string> {
  const all = shown.length > 0 && shown.every((id) => checked.has(id));
  const next = new Set(checked);
  for (const id of shown) if (all) next.delete(id); else next.add(id);
  return next;
}

/** The checked saves still in the list and still Sandbox saves, in list order. */
export const checkedRows = (saves: SaveRow[], checked: Set<string>): SaveRow[] => saves.filter((s) => s.kind === "sandbox" && checked.has(s.id));

/** Of the checked saves, those a bulk delete removes: never an Official save, never the active one. */
export const deletable = (rows: SaveRow[]): SaveRow[] => rows.filter((r) => r.kind === "sandbox" && !r.active);

/** One zip name per save for an export into one folder; two saves of the same name get " (2)" and so on. */
export function zipNames(rows: SaveRow[]): string[] {
  const used = new Set<string>();
  return rows.map((r) => {
    const base = fileSafe(r.name);
    let name = base;
    for (let n = 2; used.has(name.toLowerCase()); n++) name = `${base} (${n})`;
    used.add(name.toLowerCase());
    return `${name}.zip`;
  });
}

/** A file in a folder picked by the dialog, with the separator the folder uses. */
export const inFolder = (dir: string, file: string) => {
  const sep = dir.includes("\\") ? "\\" : "/";
  return `${dir.replace(/[\\/]+$/, "")}${sep}${file}`;
};
