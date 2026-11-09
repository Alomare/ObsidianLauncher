// The shapes the save commands send and take (internal/planning/specs/saves-api.md).
import type { Localized, Snapshot } from "../types";

export type SaveKind = "official" | "sandbox";
export type SaveStats = { rank: number; xp: number; samples: number; unlocks: number; kills: number; deaths: number; missionsFought: number; missionsWon: number; timePlayed: number };
export type SaveRow = { id: string; name: string; kind: SaveKind; created: string; lastPlayed: string | null; snapshotOf: string | null; active: boolean; live: boolean; stats: SaveStats | null; error: string | null };
export type SaveList = { account: string; active: string | null; saves: SaveRow[] };
export type SavesDone = { saves: SaveList; snapshot: Snapshot };
export type SaveImported = SavesDone & { id: string };

export type RankRow = { level: number; requiredXp: number; title: string };
export type Upgrade = { name: string; display: string; value: number };
export type ItemState = {
  name: string; display: string; description: string; category: string | null; dlc: boolean; locked: boolean; owned: boolean; inUnlocks: boolean;
  grantedRank: number | null; upgradeKind: "bitmask" | "level"; upgrade: number; maxUpgrade: number; upgrades: Upgrade[];
};
/** `dlc`: paid DLC. `locked`: paid DLC the account does not own, which the editor never writes. */
export type CosmeticState = { name: string; display: string; dlc: boolean; locked: boolean; owned: boolean; inUnlocks: boolean; grantedRank: number | null };
export type CosmeticSlot = "helmet" | "body" | "cape" | "gender";
export const COSMETIC_SLOTS: CosmeticSlot[] = ["helmet", "body", "cape", "gender"];

/** A mod's text: a table by language, or plain text. */
export type ModText = Localized | string | null;
export type ModChoice = string | { id: string; label?: ModText };
export type ModItemState = { kind: string; name: string; label: ModText; unlock: string | { rank: number } | null; owned: boolean | null };
export type ModFieldKind = "number" | "toggle" | "choice";
export type ModFieldValue = string | number | boolean | null;
export type ModFieldState = { id: string; kind: ModFieldKind; label: ModText; min: number | null; max: number | null; choices: ModChoice[] | null; when: Record<string, unknown> | null; value: ModFieldValue };
export type ModSection = { id: string; name: Localized | null; state: "enabled" | "disabled" | "removed"; items: ModItemState[]; fields: ModFieldState[]; data: { key: string; value: unknown }[] };

export type EditorState = {
  id: string; name: string; xp: number; rank: number; samples: number; researchPoints: number; maxSamples: number; samplesPerPoint: number;
  ranks: RankRow[]; bossInfluence: number[]; bossInfluenceRequired: number; tutorialStage: number | null;
  primaryWeapons: ItemState[]; sidearmWeapons: ItemState[]; stratagems: ItemState[]; perks: ItemState[];
  customization: Partial<Record<CosmeticSlot, number>>; cosmetics: Record<CosmeticSlot, CosmeticState[]>;
  unknownUnlocks: string[]; flags: { key: string; value: boolean }[]; lifetime: Record<string, number>; mods: ModSection[];
};

export type Edit =
  | { op: "set_xp"; xp: number }
  | { op: "set_rank"; rank: number }
  | { op: "set_samples"; samples: number }
  | { op: "set_owned"; item: string; owned: boolean }
  | { op: "set_upgrade"; item: string; value: number }
  | { op: "set_customization"; slot: CosmeticSlot; index: number }
  | { op: "set_boss_influence"; index: number; value: number }
  | { op: "set_flag"; key: string; value: boolean }
  | { op: "set_tutorial_stage"; value: number }
  | { op: "max_everything" }
  | { op: "reset_progression" }
  | { op: "mod_set"; mod_id: string; key: string; value: string | number | boolean }
  | { op: "mod_delete"; mod_id: string; key: string }
  | { op: "mod_clear"; mod_id: string }
  | { op: "mod_item_owned"; mod_id: string; name: string; owned: boolean };
