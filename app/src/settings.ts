// The app settings (app.json through the Rust side), applied to the page as they change.
import { call, errorText } from "./api";
import { withTask } from "./busy";
import { pickLanguage, setLanguage, t } from "./i18n";
import { notify } from "./notices";
import { createStore } from "./store";
import { applyAppearance } from "./theme";
import type { AppSettings } from "./types";

const store = createStore<AppSettings | null>(null);

function apply(s: AppSettings) {
  applyAppearance(s);
  setLanguage(pickLanguage(s.language, navigator.languages ?? [navigator.language]));
  store.set(s);
}

export const DEFAULTS: AppSettings = { theme: "default", scale: 100, text: "normal", motion: null, language: null, updates: "prompt", gameDir: null, askVanillaPlay: true, padGlyphs: "auto", padSpeed: 100 };

/** Loads the saved settings. If that fails, or takes long (the first calls after a start can
 *  hang), the window opens with the defaults and takes the saved ones when they arrive. */
export async function loadSettings() {
  const slow = setTimeout(() => store.get() ?? apply(DEFAULTS), 1500);
  try {
    apply(await call<AppSettings>("get_app_settings"));
  } catch (e) {
    if (!store.get()) apply(DEFAULTS);
    notify("error", errorText(e));
  } finally {
    clearTimeout(slow);
  }
}

/** Changes not sent yet, and the writes in order: two quick changes both land, the second on top of the first. */
let waiting: Partial<AppSettings> = {};
let queue: Promise<void> = Promise.resolve();

export function updateSettings(patch: Partial<AppSettings>): Promise<void> {
  if (!store.get()) return Promise.resolve();
  waiting = { ...waiting, ...patch };
  queue = queue.then(async () => {
    if (!Object.keys(waiting).length) return;
    const send = waiting;
    waiting = {};
    try {
      apply(await withTask(t("app.task.settings"), () => call<AppSettings>("set_app_settings", { settings: { ...store.get(), ...send } })));
    } catch (e) {
      notify("error", errorText(e));
    }
  });
  return queue;
}

export const currentSettings = () => store.get();
/** Takes settings the Rust side already saved (a sync download). */
export const applySettings = (s: AppSettings) => apply(s);

export const useAppSettings = () => store.use();
