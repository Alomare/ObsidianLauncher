// The shapes the Rust side sends (app/src-tauri/src/snapshot.rs and app_settings.rs).

export type Localized = Record<string, string>;

export type OptionRow = {
  id: string; kind: "toggle" | "number" | "choice" | "key" | "color";
  label: Localized; description: Localized; default: unknown; value: unknown;
  min: number | null; max: number | null; step: number | null; choices: string[]; choiceLabels: Localized[];
  visible: boolean; deploy: boolean;
  /** The option group it shows under, and the part of the mod it belongs to. */
  group: string | null; part: string | null;
};
/** One part of a mod (`parts`): on is false while the part's toggle option is off. */
export type PartRow = { id: string; scope: "client" | "session"; name: Localized; description: Localized; on: boolean };
/** A collapsible heading of the Options tab (`option_groups`). */
export type OptionGroup = { id: string; label: Localized; description: Localized; collapsed: boolean };
/** An SVG template filled in with the option values (`option_previews`). */
export type OptionPreview = { id: string; group: string | null; svg: string; width: number; height: number; label: Localized };
/** pad: the choice option with the controller input for the same action, shown in this row instead of under Options. */
export type KeyRow = { id: string; label: Localized; default: string; key: string | null; clash: string[]; pad: string | null };
export type ImportInfo = { date: string; file: string };
export type ImageRef = { path: string; v: number };
/** Every scope a manifest can give, as `Scope::name` in core/src/manifest.rs writes it. */
export const SCOPES = ["unspecified", "client", "session", "library", "expansion"] as const;
export type Scope = (typeof SCOPES)[number];
/** What a mod runs as: a scope, or hybrid for a mod with a client part and a session part on. */
export type ModScope = Scope | "hybrid";
export type ModRow = {
  id: string; name: Localized; description: Localized; authors: string[];
  version: string | null; scope: ModScope; on: boolean;
  parts: PartRow[]; optionGroups: OptionGroup[]; optionPreviews: OptionPreview[];
  options: OptionRow[]; keybinds: KeyRow[]; files: string[];
  import: ImportInfo | null; source: string | null; updatable: boolean; image: ImageRef | null;
  folder: string | null; requires: string[];
};
export type Folder = { id: string; name: string; color: string; collapsed: boolean; locked: boolean };
export type ConflictRow = { resource: string; mods: string[]; winner: string };
export type ChangesWaiting = "UpToDate" | "Waiting" | "Vanilla";
export type GameState = "supported" | "unknown" | "running" | "notFound";
export type ProfileRow = { name: string; on: number; blockPublic: boolean; guarded: boolean; image: ImageRef | null; color: string | null };
export type Snapshot = {
  mods: ModRow[]; folders: Folder[]; profile: string; profiles: string[]; profileRows: ProfileRow[]; blockPublic: boolean; blockAchievements: boolean; freezeProgress: boolean; hideClientMods: boolean; loader: boolean; sandbox: string | null; conflicts: ConflictRow[];
  changes: ChangesWaiting; deployed: boolean; game: GameState; gameChanges: { path: string; missing: boolean }[]; gameDir: string | null; crashCulprit: string | null; broken: { folder: string; reason: string }[]; damagedProfiles: string[]; uncleanEnd: { stamp: number; last: string } | null;
  portable: boolean; version: string;
};

export type AppSettings = {
  theme: "default" | "light"; scale: 90 | 100 | 110 | 125;
  text: "small" | "normal" | "large"; motion: boolean | null; language: string | null;
  updates: "manual" | "prompt" | "auto"; gameDir: string | null;
  /** Play asks first when no mods are deployed (PlayPrompt.tsx). */
  askVanillaPlay: boolean;
};

export type CmdError = { code: string; message: string };

export type ImportResult = {
  file: string; ok: boolean; kind: "installed" | "updated" | "reinstalled" | "";
  name: Localized; version: string | null; oldVersion: string | null; error: CmdError | null;
};
export type ImportBatch = { snapshot: Snapshot; results: ImportResult[] };
