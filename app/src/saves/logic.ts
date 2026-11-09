// What the Saves page and the editor work out without the Rust side: rank from XP, the upgrade bits,
// how an item came to be owned, the Arsenal's groups, and the unsaved edits. Nothing here touches a file.
import type { CosmeticState, Edit, EditorState, ItemState, ModChoice, ModFieldState, RankRow } from "./types";

/** The highest rank `xp` has reached. */
export function rankOfXp(ranks: RankRow[], xp: number): RankRow {
  let at = ranks[0];
  for (const r of ranks) if (xp >= r.requiredXp) at = r;
  return at;
}

export const xpOfRank = (ranks: RankRow[], level: number) => ranks.find((r) => r.level === level)?.requiredXp ?? 0;

export const topXp = (ranks: RankRow[]) => ranks[ranks.length - 1]?.requiredXp ?? 0;

/** How far `xp` is through the current rank: 0 to 1, and 1 at the top rank. `need` is the XP left to the next. */
export function rankProgress(ranks: RankRow[], xp: number): { fraction: number; need: number | null; next: RankRow | null } {
  const at = rankOfXp(ranks, xp);
  const next = ranks.find((r) => r.level === at.level + 1) ?? null;
  if (!next) return { fraction: 1, need: null, next: null };
  return { fraction: Math.min(1, (xp - at.requiredXp) / (next.requiredXp - at.requiredXp)), need: next.requiredXp - xp, next };
}

export const researchPoints = (samples: number, perPoint: number) => Math.floor(samples / perPoint);

export const hasBit = (value: number, bit: number) => (value & bit) === bit;
export const withBit = (value: number, bit: number, on: boolean) => (on ? value | bit : value & ~bit);

/** Where a rank stands on the ladder of the Overview: reached, the save's rank, the next one, or still ahead. */
export function ladderState(level: number, rank: number): "earned" | "current" | "next" | "ahead" {
  if (level === rank) return "current";
  if (level < rank) return "earned";
  return level === rank + 1 ? "next" : "ahead";
}

/** How many of `of` marks to light for `won` out of `fought`: none when nothing was fought. */
export function tally(won: number, fought: number, of: number) {
  if (fought <= 0 || won <= 0) return 0;
  return Math.min(of, Math.round((won / fought) * of));
}

/** Which upgrades an item has, in catalog order: by bit for a bitmask, by level for Mk levels. */
export function upgradeMarks(item: Pick<ItemState, "upgradeKind" | "upgrade" | "upgrades">): boolean[] {
  return item.upgrades.map((u) => (item.upgradeKind === "bitmask" ? hasBit(item.upgrade, u.value) : item.upgrade >= u.value));
}

/** The new level after a click on the level `value` of a Mk ladder: up to that level, or one below when the
 *  item has it already. */
export const levelPick = (current: number, value: number) => (current >= value ? value - 1 : value);

/** A whole number from what was typed, kept inside the limits; null when nothing usable was typed. */
export function parseWhole(text: string, min: number | null, max: number | null): number | null {
  const n = Number(text.replace(/[\s,]/g, ""));
  if (!text.trim() || !Number.isFinite(n)) return null;
  let v = Math.round(n);
  if (min !== null) v = Math.max(min, v);
  if (max !== null) v = Math.min(max, v);
  return v;
}

export type OwnedRoute = "dlc" | "entitled" | "unlocked" | "rank" | "fixed" | "none";

/** Why an item is owned, which says what the owned switch can do: only `unlocked` and `none` can change.
 *  `dlc` is paid DLC the account does not own (locked); `entitled` is paid DLC it owns, which Steam grants. */
export function ownedRoute(item: Pick<ItemState, "dlc" | "locked" | "owned" | "inUnlocks" | "grantedRank">, rank: number): OwnedRoute {
  if (item.locked) return "dlc";
  if (item.dlc) return "entitled";
  if (item.inUnlocks) return "unlocked";
  if (item.grantedRank !== null && item.grantedRank <= rank) return "rank";
  return item.owned ? "fixed" : "none";
}

/** The choices of a cosmetic slot, by 1-based index: all but DLC the account does not own, and always
 *  the one the save wears now, so a worn locked item still shows. */
export function slotChoices(list: Pick<CosmeticState, "locked">[], current: number | undefined): number[] {
  const out: number[] = [];
  list.forEach((c, i) => { if (!c.locked || current === i + 1) out.push(i + 1); });
  return out;
}

/** Whether the search text matches the item's name, raw name, category or description. */
export function itemMatches(item: ItemState, query: string) {
  const hay = `${item.display} ${item.name} ${item.category ?? ""} ${item.description}`.toLowerCase();
  return query.toLowerCase().split(/\s+/).filter(Boolean).every((w) => hay.includes(w));
}

/** The matching items by category, in catalog order. */
export function groupItems(items: ItemState[], query: string): { category: string | null; items: ItemState[] }[] {
  const groups: { category: string | null; items: ItemState[] }[] = [];
  for (const it of items) {
    if (!itemMatches(it, query)) continue;
    let g = groups.find((x) => x.category === it.category);
    if (!g) groups.push((g = { category: it.category, items: [] }));
    g.items.push(it);
  }
  return groups;
}

/** A mod field's `when`: every named field of the same mod holds the named value. */
export function fieldEnabled(field: ModFieldState, all: ModFieldState[]) {
  if (!field.when) return true;
  return Object.entries(field.when).every(([id, value]) => {
    const other = all.find((f) => f.id === id);
    return !other || other.value === value;
  });
}

export const choiceId = (c: ModChoice) => (typeof c === "string" ? c : c.id);

/** One change waiting per thing changed; `set_xp` and `set_rank` are the same thing. */
export function editKey(e: Edit): string {
  switch (e.op) {
    case "set_xp": case "set_rank": return "xp";
    case "set_samples": return "samples";
    case "set_owned": return `own:${e.item}`;
    case "set_upgrade": return `up:${e.item}`;
    case "set_customization": return `cos:${e.slot}`;
    case "set_boss_influence": return `boss:${e.index}`;
    case "set_flag": return `flag:${e.key}`;
    case "set_tutorial_stage": return "stage";
    case "set_lifetime": return `life:${e.key}`;
    case "mod_set": case "mod_delete": return `mod:${e.mod_id}:${e.key}`;
    case "mod_clear": return `clear:${e.mod_id}`;
    case "mod_item_owned": return `item:${e.mod_id}:${e.name}`;
    default: return e.op;
  }
}

function findItem(s: EditorState, name: string): ItemState | CosmeticState | undefined {
  for (const list of [s.primaryWeapons, s.sidearmWeapons, s.stratagems, s.perks]) {
    const hit = list.find((i) => i.name === name);
    if (hit) return hit;
  }
  return Object.values(s.cosmetics).flat().find((c) => c.name === name);
}

/** The edit asks for what the save holds already, so there is nothing to write. */
function changesNothing(base: EditorState, e: Edit): boolean {
  switch (e.op) {
    case "set_xp": return base.xp === e.xp;
    case "set_rank": return base.rank === e.rank;
    case "set_samples": return base.samples === e.samples;
    case "set_owned": {
      const it = findItem(base, e.item);
      return it ? it.inUnlocks === e.owned : e.owned === base.unknownUnlocks.includes(e.item);
    }
    case "set_upgrade": return (findItem(base, e.item) as ItemState | undefined)?.upgrade === e.value;
    case "set_customization": return base.customization[e.slot] === e.index;
    case "set_boss_influence": return base.bossInfluence[e.index - 1] === e.value;
    case "set_flag": return base.flags.find((f) => f.key === e.key)?.value === e.value;
    case "set_tutorial_stage": return base.tutorialStage === e.value;
    case "set_lifetime": return base.lifetime[e.key] === e.value;
    case "mod_set": return base.mods.find((m) => m.id === e.mod_id)?.fields.find((f) => f.id === e.key)?.value === e.value;
    case "mod_item_owned": return base.mods.find((m) => m.id === e.mod_id)?.items.find((i) => i.name === e.name)?.owned === e.owned;
    default: return false;
  }
}

/** The waiting edits with `e` added: the newest per thing wins, and asking for the saved value drops the edit. */
export function stage(pending: Map<string, Edit>, base: EditorState, e: Edit): Map<string, Edit> {
  const next = new Map(pending);
  const key = editKey(e);
  next.delete(key);
  if (e.op === "mod_clear") for (const k of [...next.keys()]) if (k.startsWith(`mod:${e.mod_id}:`)) next.delete(k);
  if (!changesNothing(base, e)) next.set(key, e);
  return next;
}

const mapItems = (s: EditorState, f: (i: ItemState) => ItemState): Pick<EditorState, "primaryWeapons" | "sidearmWeapons" | "stratagems" | "perks"> => ({
  primaryWeapons: s.primaryWeapons.map(f), sidearmWeapons: s.sidearmWeapons.map(f), stratagems: s.stratagems.map(f), perks: s.perks.map(f),
});

const mapCosmetics = (s: EditorState, f: (c: CosmeticState) => CosmeticState) =>
  Object.fromEntries(Object.entries(s.cosmetics).map(([slot, list]) => [slot, list.map(f)])) as EditorState["cosmetics"];

function applyEdit(s: EditorState, e: Edit): EditorState {
  switch (e.op) {
    case "set_xp": return { ...s, xp: e.xp, rank: rankOfXp(s.ranks, e.xp).level };
    case "set_rank": return { ...s, rank: e.rank, xp: xpOfRank(s.ranks, e.rank) };
    case "set_samples": return { ...s, samples: e.samples, researchPoints: researchPoints(e.samples, s.samplesPerPoint) };
    case "set_owned":
      return {
        ...s, ...mapItems(s, (i) => (i.name === e.item ? { ...i, inUnlocks: e.owned } : i)),
        cosmetics: mapCosmetics(s, (c) => (c.name === e.item ? { ...c, inUnlocks: e.owned } : c)),
        unknownUnlocks: e.owned ? s.unknownUnlocks : s.unknownUnlocks.filter((n) => n !== e.item),
      };
    case "set_upgrade": return { ...s, ...mapItems(s, (i) => (i.name === e.item ? { ...i, upgrade: e.value } : i)) };
    case "set_customization": return { ...s, customization: { ...s.customization, [e.slot]: e.index } };
    case "set_boss_influence": return { ...s, bossInfluence: s.bossInfluence.map((v, i) => (i === e.index - 1 ? e.value : v)) };
    case "set_flag": return { ...s, flags: s.flags.map((f) => (f.key === e.key ? { ...f, value: e.value } : f)) };
    case "set_tutorial_stage": return { ...s, tutorialStage: e.value };
    case "set_lifetime": return { ...s, lifetime: { ...s.lifetime, [e.key]: e.value } };
    case "mod_set": return { ...s, mods: s.mods.map((m) => (m.id !== e.mod_id ? m : { ...m, fields: m.fields.map((f) => (f.id === e.key ? { ...f, value: e.value } : f)) })) };
    case "mod_delete": return { ...s, mods: s.mods.map((m) => (m.id !== e.mod_id ? m : { ...m, data: m.data.filter((d) => d.key !== e.key) })) };
    case "mod_clear": return { ...s, mods: s.mods.map((m) => (m.id !== e.mod_id ? m : { ...m, data: [] })) };
    case "mod_item_owned": return { ...s, mods: s.mods.map((m) => (m.id !== e.mod_id ? m : { ...m, items: m.items.map((i) => (i.name === e.name ? { ...i, owned: e.owned } : i)) })) };
    default: return s;
  }
}

/** The save as it will be once the waiting edits are written. Ownership by rank follows the new rank. */
export function draftOf(base: EditorState, pending: Map<string, Edit>): EditorState {
  const s = [...pending.values()].reduce(applyEdit, base);
  // Owned without being in the unlock list, and without the saved rank giving it: entitlement or fixed.
  const fixed = new Set<string>();
  const note = (i: ItemState | CosmeticState) => { if (i.owned && !i.inUnlocks && !(i.grantedRank !== null && i.grantedRank <= base.rank)) fixed.add(i.name); };
  mapItems(base, (i) => (note(i), i));
  mapCosmetics(base, (c) => (note(c), c));
  const owned = <T extends ItemState | CosmeticState>(i: T): T => ({ ...i, owned: i.inUnlocks || fixed.has(i.name) || (i.grantedRank !== null && i.grantedRank <= s.rank) });
  return { ...s, ...mapItems(s, owned), cosmetics: mapCosmetics(s, owned), mods: s.mods.filter((m) => m.state !== "removed" || m.data.length > 0) };
}

/** A save name as a file name: the characters a file name can't hold become spaces. */
export const fileSafe = (name: string) => name.replace(/[\\/:*?"<>|]/g, " ").replace(/\s+/g, " ").trim() || "save";

/** Seconds as "12 h 5 min". */
export function playTime(seconds: number) {
  const m = Math.floor(seconds / 60);
  const h = Math.floor(m / 60);
  return h ? `${h} h ${m % 60} min` : `${m} min`;
}
