// The shapes the Rust side sends (app/src-tauri/src/snapshot.rs and app_settings.rs).

export type Localized = Record<string, string>;

export type OptionRow = {
  id: string; kind: "toggle" | "number" | "choice" | "key" | "color";
  label: Localized; description: Localized; default: unknown; value: unknown;
  min: number | null; max: number | null; step: number | null; choices: string[]; choiceLabels: Localized[];
  visible: boolean; deploy: boolean;
};
export type KeyRow = { id: string; label: Localized; default: string; key: string | null; clash: string[] };
export type ImportInfo = { date: string; file: string };
export type ImageRef = { path: string; v: number };
/** Every scope a manifest can give, as `Scope::name` in core/src/manifest.rs writes it. */
export const SCOPES = ["unspecified", "client", "session", "library", "expansion"] as const;
export type Scope = (typeof SCOPES)[number];
export type ModRow = {
  id: string; name: Localized; description: Localized; authors: string[];
  version: string | null; scope: Scope; on: boolean;
  options: OptionRow[]; keybinds: KeyRow[]; files: string[];
  import: ImportInfo | null; source: string | null; updatable: boolean; image: ImageRef | null; customImage: boolean;
  folder: string | null; requires: string[];
};
export type Folder = { id: string; name: string; color: string; collapsed: boolean; locked: boolean };
export type ConflictRow = { resource: string; mods: string[]; winner: string };
export type ChangesWaiting = "UpToDate" | "Waiting" | "Vanilla";
export type GameState = "supported" | "unknown" | "running" | "notFound";
export type ProfileRow = { name: string; on: number; blockPublic: boolean; guarded: boolean; image: ImageRef | null; color: string | null };
export type Snapshot = {
  mods: ModRow[]; folders: Folder[]; profile: string; profiles: string[]; profileRows: ProfileRow[]; blockPublic: boolean; blockAchievements: boolean; freezeProgress: boolean; conflicts: ConflictRow[];
  changes: ChangesWaiting; deployed: boolean; game: GameState; gameChanges: { path: string; missing: boolean }[]; gameDir: string | null; crashCulprit: string | null; broken: { folder: string; reason: string }[]; damagedProfiles: string[]; uncleanEnd: { stamp: number; last: string } | null;
  portable: boolean; version: string;
};

export type AppSettings = {
  theme: "default" | "light"; scale: 90 | 100 | 110 | 125;
  text: "small" | "normal" | "large"; motion: boolean | null; language: string | null;
  updates: "manual" | "prompt" | "auto"; gameDir: string | null;
};

export type CmdError = { code: string; message: string };

export type ImportResult = {
  file: string; ok: boolean; kind: "installed" | "updated" | "reinstalled" | "";
  name: Localized; version: string | null; oldVersion: string | null; error: CmdError | null;
};
export type ImportBatch = { snapshot: Snapshot; results: ImportResult[] };
