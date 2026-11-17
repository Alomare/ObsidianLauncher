// What Ctrl+C and Ctrl+V do on the Mods list. A copy is one line per mod, the name and the version,
// in load order. A paste of such a list (or of names or ids, one per line) checks the mods it names;
// a paste of file paths imports them, as a drop on the list does.

export type ClipMod = { id: string; name: string; version: string | null };

/** The lines a copy puts on the clipboard. */
export function copyText(mods: ClipMod[]): string {
  return mods.map((m) => (m.version ? `${m.name} ${m.version}` : m.name)).join("\n");
}

const lines = (text: string) => text.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);

/** A path on Windows or Linux: a drive, a share or a root, quoted or not. */
const isPath = (l: string) => /^"?([A-Za-z]:[\\/]|\\\\|\/)/.test(l);

/** The paths in a paste when every line is one, without the quotes Explorer adds; null otherwise. */
export function pastedPaths(text: string): string[] | null {
  const all = lines(text);
  if (!all.length || !all.every(isPath)) return null;
  return all.map((l) => l.replace(/^"(.*)"$/, "$1"));
}

/** The ids of the mods a pasted list names, by name or id, with or without the version after it. Case does not count. */
export function pastedMods(text: string, mods: ClipMod[]): string[] {
  const key = (s: string) => s.trim().toLowerCase();
  const byKey = new Map<string, string>();
  for (const m of mods) {
    byKey.set(key(m.id), m.id);
    byKey.set(key(m.name), m.id);
    if (m.version) byKey.set(key(`${m.name} ${m.version}`), m.id);
  }
  const found = new Set<string>();
  for (const l of lines(text)) {
    const k = key(l);
    // The whole line first, then without the last word, which a copy fills with the version.
    const id = byKey.get(k) ?? byKey.get(k.replace(/\s+\S+$/, ""));
    if (id) found.add(id);
  }
  return mods.filter((m) => found.has(m.id)).map((m) => m.id);
}
