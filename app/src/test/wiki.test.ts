import { describe, expect, it } from "vitest";
import { wikiPages } from "../wiki";

const LANGS = ["en", "pt-BR", "zh-CN"];
/** Pages written since the last translation pass: English for now, listed in internal/release/untranslated-keys.txt. */
const UNTRANSLATED = new Set(["saves"]);

describe("wiki", () => {
  it("has a translation of every page in every language", () => {
    const ids = wikiPages("en").pages.map((p) => p.id);
    for (const lang of LANGS) {
      const pages = wikiPages(lang).pages;
      expect(pages.map((p) => p.id)).toEqual(ids);
      if (lang !== "en") {
        // A translated page differs from the English one; a missing file would show English.
        const en = wikiPages("en").pages;
        pages.forEach((p, i) => {
          if (!UNTRANSLATED.has(p.id)) expect(p.title, `${lang} ${p.id}`).not.toBe(en[i].title);
        });
      }
    }
  });

  it("links only to pages and headings that exist, in every language", () => {
    for (const lang of LANGS) {
      const pages = wikiPages(lang).pages;
      const heads = new Map(pages.map((p) => [p.id, new Set(p.headings.map((h) => h.id))]));
      for (const p of pages) {
        const hrefs: string[] = [];
        const walk = (tokens: unknown[]) => {
          for (const t of tokens as { type: string; href?: string; tokens?: unknown[]; items?: unknown[]; rows?: unknown[][]; header?: unknown[] }[]) {
            if (t.type === "link" && t.href) hrefs.push(t.href);
            if (Array.isArray(t.tokens)) walk(t.tokens);
            if (Array.isArray(t.items)) walk(t.items);
            if (Array.isArray(t.header)) walk(t.header);
            if (Array.isArray(t.rows)) t.rows.forEach((r) => walk(r));
          }
        };
        walk(p.tokens);
        for (const href of hrefs.filter((h) => h.startsWith("wiki:") || h.startsWith("#"))) {
          const [page, anchor] = href.startsWith("#") ? [p.id, href.slice(1)] : href.slice(5).split("#");
          expect(heads.has(page), `${lang} ${p.id}: ${href}`).toBe(true);
          if (anchor) expect(heads.get(page)!.has(anchor), `${lang} ${p.id}: ${href}`).toBe(true);
        }
      }
    }
  });
});
