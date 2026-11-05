// Widths of the mod list's resizable columns, in rem so they follow the UI scale. The Mod column takes
// whatever the others leave, so moving a line trades width between the two columns beside it.

export type Col = "version" | "scope";
export type Widths = Record<Col, number>;

export const COLUMN_KEY = "obsidian.modColumns";
const COLS: Col[] = ["version", "scope"];
/** Version fits its header and 1.29.0 with the update arrow, so Scope sits close (the user, 2026-11-04);
 *  a longer version ends in an ellipsis until the line is dragged. */
export const DEFAULTS: Widths = { version: 5.2, scope: 3.2 };
/** Version keeps room for its header; Scope for the diamond. */
export const MINS: Widths = { version: 3.7, scope: 2.6 };
/** What the Mod column keeps, so a name still shows. */
export const MOD_MIN = 7;
const MAX = 30;

const round = (n: number) => Math.round(n * 100) / 100;
const clamp = (n: number, lo: number, hi: number) => Math.min(Math.max(n, lo), Math.max(lo, hi));

/** The saved widths; a missing, damaged or out of range value falls back to the default. */
export function parseWidths(raw: string | null): Widths {
  let saved: Partial<Record<Col, unknown>> | null = null;
  try {
    const v: unknown = raw ? JSON.parse(raw) : null;
    if (v && typeof v === "object") saved = v as Partial<Record<Col, unknown>>;
  } catch { /* a damaged value counts as none */ }
  const one = (c: Col) => {
    const v = saved?.[c];
    return typeof v === "number" && Number.isFinite(v) ? clamp(v, MINS[c], MAX) : DEFAULTS[c];
  };
  return { version: one("version"), scope: one("scope") };
}

/**
 * The widths after the line at the left of `col` moves `dx` rem (right is positive), from the widths
 * `start` and the Mod column's width `mod`. Beside the Version line is the Mod column, which gives or
 * takes; beside the Scope line is Version, which trades with Scope.
 */
export function drag(start: Widths, col: Col, dx: number, mod: number): Widths {
  if (col === "version") {
    const room = Math.max(start.version, start.version + mod - MOD_MIN);
    return { ...start, version: round(clamp(start.version - dx, MINS.version, room)) };
  }
  const d = clamp(dx, MINS.version - start.version, start.scope - MINS.scope);
  return { version: round(start.version + d), scope: round(start.scope - d) };
}

/** A double click on a line: the columns beside it go back to their default width. */
export const reset = (w: Widths, col: Col): Widths => (col === "version" ? { ...w, version: DEFAULTS.version } : { ...DEFAULTS });

/** Only the columns away from the default are kept, so a later change of the default reaches the rest. */
export function serialize(w: Widths): string | null {
  const changed = Object.fromEntries(COLS.filter((c) => w[c] !== DEFAULTS[c]).map((c) => [c, w[c]]));
  return Object.keys(changed).length ? JSON.stringify(changed) : null;
}

export function readWidths(): Widths {
  try {
    return parseWidths(localStorage.getItem(COLUMN_KEY));
  } catch {
    return parseWidths(null);
  }
}

export function saveWidths(w: Widths) {
  try {
    const raw = serialize(w);
    if (raw) localStorage.setItem(COLUMN_KEY, raw);
    else localStorage.removeItem(COLUMN_KEY);
  } catch { /* the widths last for this visit */ }
}
