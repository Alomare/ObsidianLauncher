// Sharing a profile as a ZIP: the load order, folders, options, keys and image, with the files of
// every enabled mod. Importing adds it as a new profile and installs the mods it carries.
import { open, save } from "@tauri-apps/plugin-dialog";
import { call, errorText, setSnapshot } from "./api";
import { withTask } from "./busy";
import { t } from "./i18n";
import { notify } from "./notices";
import type { Snapshot } from "./types";

const filters = () => [{ name: t("app.profiles.zipFilter"), extensions: ["zip"] }];

export async function exportProfile(name: string) {
  const dest = await save({ defaultPath: `${name}.zip`, filters: filters() });
  if (!dest) return;
  try {
    const n = await withTask(t("app.task.export_profile", name), () => call<number>("export_profile", { name, dest }), true);
    notify("info", n === 1 ? t("app.profiles.exportedOne", name) : t("app.profiles.exported", name, n));
  } catch (e) {
    notify("error", errorText(e));
  }
}

type Imported = { snapshot: Snapshot; imported: { name: string; installed: string[]; kept: string[]; guardrails: string[] } };

export async function importProfile() {
  const path = await open({ filters: filters() });
  if (typeof path !== "string") return;
  try {
    const r = await withTask(t("app.task.import_profile"), () => call<Imported>("import_profile", { path }), true);
    setSnapshot(r.snapshot);
    const n = r.imported.installed.length;
    notify("info", n === 1 ? t("app.profiles.importedOne", r.imported.name) : t("app.profiles.imported", r.imported.name, n));
    // The guardrails come along with a profile, so the player hears which ones are on.
    const guards = r.imported.guardrails.map((g) => t(g === "freeze_progress" ? "app.guardrails.freeze" : "app.guardrails.achievements"));
    if (guards.length > 0) notify("info", t("app.profiles.importedGuardrails", r.imported.name, guards.join(", ")));
  } catch (e) {
    notify("error", errorText(e));
  }
}
