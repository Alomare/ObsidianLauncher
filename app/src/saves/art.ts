// Pictures for the save editor: the rank insignia (`rank-1` to `rank-50`) and one icon per catalog item, named
// after the item. Where they came from is in assets/saves/SOURCES.md. An item without a picture (a mod item, or
// one added to the catalog later) gets the fallback glyph instead.
const files = import.meta.glob<string>("../assets/saves/*.png", { eager: true, query: "?url", import: "default" });

const byName = new Map<string, string>();
for (const [path, url] of Object.entries(files)) byName.set(path.slice(path.lastIndexOf("/") + 1, -4), url);

/** The picture of an item or insignia by name, or undefined when there is none. */
export const artFor = (name: string): string | undefined => byName.get(name);

export const rankArt = (level: number) => artFor(`rank-${level}`);
