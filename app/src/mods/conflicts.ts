// Conflicts as people read them: who overrides whom. `winners` works them out for an order that
// is not saved yet (a drag under way), so the view follows the rows as they move.
import type { ConflictRow } from "../types";

/** Each conflict with the mods involved in `order` and the last one as the winner. */
export function winners(rows: ConflictRow[], order: string[]): ConflictRow[] {
  const pos = (id: string) => order.indexOf(id);
  return rows.map((r) => {
    const mods = [...r.mods].sort((a, b) => pos(a) - pos(b));
    return { ...r, mods, winner: mods[mods.length - 1] };
  });
}

export type Override = { winner: string; loser: string; resources: string[] };

/** Who overrides whom: one entry per winner and loser, with the files the winner takes. */
export function overrides(rows: ConflictRow[]): Override[] {
  const pairs = new Map<string, Override>();
  for (const r of rows) {
    for (const loser of r.mods) {
      if (loser === r.winner) continue;
      const k = `${r.winner}
${loser}`;
      const o = pairs.get(k) ?? { winner: r.winner, loser, resources: [] };
      o.resources.push(r.resource);
      pairs.set(k, o);
    }
  }
  return [...pairs.values()];
}

type ModConflicts = { overrides: [string, number][]; overriddenBy: [string, number][] };

/** For one mod: the mods it overrides and the mods that override it, each with a file count. */
export function conflictsOf(id: string, rows: ConflictRow[]): ModConflicts {
  return conflictsByMod(rows).get(id) ?? { overrides: [], overriddenBy: [] };
}

/** `conflictsOf` for every mod in a conflict, from one pass: the table needs them all after every
 * change, and working them out mod by mod went over every conflict once per mod. */
export function conflictsByMod(rows: ConflictRow[]): Map<string, ModConflicts> {
  const out = new Map<string, ModConflicts>();
  const of = (id: string) => {
    let c = out.get(id);
    if (!c) out.set(id, (c = { overrides: [], overriddenBy: [] }));
    return c;
  };
  for (const o of overrides(rows)) {
    of(o.winner).overrides.push([o.loser, o.resources.length]);
    of(o.loser).overriddenBy.push([o.winner, o.resources.length]);
  }
  return out;
}
