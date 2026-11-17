// UI text from the shared catalogs in locales/ (app.* keys). English fills any gap.
import en from "../../locales/en.json";
import ptBR from "../../locales/pt-BR.json";
import ru from "../../locales/ru.json";
import zhCN from "../../locales/zh-CN.json";
import type { Localized } from "./types";

type Catalog = Record<string, string>;
export const CATALOGS: Record<string, Catalog> = { en, "pt-BR": ptBR, ru, "zh-CN": zhCN };
export const LANGUAGES: [string, string][] = [["en", "English"], ["pt-BR", "Português (Brasil)"], ["ru", "Русский"], ["zh-CN", "中文 (简体)"]];

/** The saved choice if there is a catalog for it, else the first system language we have (by exact
 *  code, then by the first part, so "pt-PT" finds pt-BR), else English. */
export function pickLanguage(saved: string | null | undefined, system: readonly string[]): string {
  if (saved && CATALOGS[saved]) return saved;
  for (const s of system) {
    if (CATALOGS[s]) return s;
    const base = s.split("-")[0].toLowerCase();
    const hit = Object.keys(CATALOGS).find((c) => c.split("-")[0].toLowerCase() === base);
    if (hit) return hit;
  }
  return "en";
}

/** `%s` placeholders take the arguments in order. */
export function translate(catalogs: Record<string, Catalog>, lang: string, key: string, args: (string | number)[]): string {
  const text = catalogs[lang]?.[key] ?? catalogs.en?.[key] ?? key;
  let i = 0;
  return text.replace(/%s/g, () => String(args[i++] ?? ""));
}

/** `translate` for a text that names a count `n`: a catalog may give a form per plural category of its
 *  language as `<key>#one`, `#few`, `#many` (the categories of Intl.PluralRules), else `<key>` serves
 *  every count. English has `#one` beside the plain key; Russian has all three; Chinese needs none.
 *  `args` fill the `%s` in order, `[n]` when none are given. */
export function translateCount(catalogs: Record<string, Catalog>, lang: string, key: string, n: number, args: (string | number)[]): string {
  const form = (l: string) => {
    const rule = new Intl.PluralRules(l).select(n);
    return catalogs[l]?.[`${key}#${rule}`] ?? catalogs[l]?.[key];
  };
  const text = form(lang) ?? form("en") ?? key;
  const fill = args.length ? args : [n];
  let i = 0;
  return text.replace(/%s/g, () => String(fill[i++] ?? ""));
}

let current = "en";
export const setLanguage = (lang: string) => { current = CATALOGS[lang] ? lang : "en"; document.documentElement.lang = current; };
export const language = () => current;
export const t = (key: string, ...args: (string | number)[]) => translate(CATALOGS, current, key, args);
/** `t` for a text that names the count `n` (see `translateCount`). */
export const tn = (key: string, n: number, ...args: (string | number)[]) => translateCount(CATALOGS, current, key, n, args);

/** A mod's own text in the UI language, then English, then whatever it has. */
export function pick(map: Localized | null | undefined, fallback = ""): string {
  if (!map) return fallback;
  return map[current] ?? map.en ?? Object.values(map)[0] ?? fallback;
}
