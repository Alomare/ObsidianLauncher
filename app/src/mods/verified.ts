// Verified mods: a short list kept with the app, each id paired with the GitHub repository the mod
// comes from. A mod counts only when both match, so another mod can't take a listed id and show the
// mark. The mark is cosmetic: a verified mod deploys and runs like any other.
import type { ModRow } from "../types";

/** Mod id to "owner/repository" in lower case; a renamed repository keeps the old name too. */
export const VERIFIED: Record<string, string[]> = {
  "obsidian.api": ["alomare/obsidianapi"],
  "obsidian.qol": ["alomare/obsidianpatches"],
  "obsidian.trainer": ["alomare/obsidiantrainer"],
  "alomare.betterhud": ["alomare/hd1-betterhud", "alomare/betterhud"],
};

/** "owner/repository" of a GitHub address, in lower case, or null. */
export function repoOf(source: string | null | undefined): string | null {
  const m = /^https:\/\/github\.com\/([\w.-]+)\/([\w.-]+?)(?:\.git)?(?:[/?#].*)?$/i.exec(source?.trim() ?? "");
  return m ? `${m[1]}/${m[2]}`.toLowerCase() : null;
}

export function isVerified(m: Pick<ModRow, "id" | "source">): boolean {
  const repo = repoOf(m.source);
  return !!repo && !!VERIFIED[m.id]?.includes(repo);
}
