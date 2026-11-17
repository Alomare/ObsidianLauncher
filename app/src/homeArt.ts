// The pictures of the Home page, in name order. The Store also uses them, blurred, behind a mod with
// neither a thumbnail nor an icon.
export const ART = Object.entries(import.meta.glob("./assets/home/*.webp", { eager: true, query: "?url", import: "default" }))
  .sort(([a], [b]) => a.localeCompare(b))
  .map(([, url]) => url as string);

/** One of the pictures for `key`, always the same one for the same key. */
export function artFor(key: string): string | undefined {
  let h = 0;
  for (const c of key) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return ART.length ? ART[h % ART.length] : undefined;
}
