// The app settings (app.json through the Rust side), applied to the page as they change.
import { useSyncExternalStore } from "react";
import { call, errorText } from "./api";
import { withTask } from "./busy";
import { pickLanguage, setLanguage, t } from "./i18n";
import { notify } from "./notices";
import { applyAppearance } from "./theme";
import type { AppSettings } from "./types";

let current: AppSettings | null = null;
const listeners = new Set<() => void>();

function apply(s: AppSettings) {
  current = s;
  applyAppearance(s);
  setLanguage(pickLanguage(s.language, navigator.languages ?? [navigator.language]));
  listeners.forEach((f) => f());
}

export const DEFAULTS: AppSettings = { theme: "default", scale: 100, text: "normal", motion: null, language: null, updates: "prompt", gameDir: null, askVanillaPlay: true };

/** Loads the saved settings. If that fails, or takes long (the first calls after a start can
 *  hang), the window opens with the defaults and takes the saved ones when they arrive. */
export async function loadSettings() {
  const slow = setTimeout(() => current ?? apply(DEFAULTS), 1500);
  try {
    apply(await call<AppSettings>("get_app_settings"));
  } catch (e) {
    if (!current) apply(DEFAULTS);
    notify("error", errorText(e));
  } finally {
    clearTimeout(slow);
  }
}

export async function updateSettings(patch: Partial<AppSettings>) {
  if (!current) return;
  try { apply(await withTask(t("app.task.settings"), () => call<AppSettings>("set_app_settings", { settings: { ...current, ...patch } }))); } catch (e) { notify("error", errorText(e)); }
}

export const currentSettings = () => current;

export const useAppSettings = () =>
  useSyncExternalStore((f) => { listeners.add(f); return () => { listeners.delete(f); }; }, () => current);
