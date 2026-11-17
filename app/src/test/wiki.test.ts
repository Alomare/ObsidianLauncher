import { describe, expect, it } from "vitest";
import { TABS } from "../Markdown";
import { wikiPages } from "../wiki";

const LANGS = ["en", "pt-BR", "ru", "zh-CN"];

describe("wiki", () => {
  it("splits into a Launcher and a Commander subtab, each with the FAQ and Troubleshooting pinned", () => {
    for (const lang of LANGS) {
      const { pages, books } = wikiPages(lang);
      expect(books.launcher.pages.length + books.commander.pages.length).toBe(pages.length);
      expect(books.launcher.pinned.map((p) => p.id)).toEqual(["faq", "troubleshooting"]);
      expect(books.commander.pinned.map((p) => p.id)).toEqual(["commander-faq", "commander-troubleshooting"]);
      for (const b of [books.launcher, books.commander]) {
        expect(b.steps.some((p) => p.pin)).toBe(false);
        // Every step is in a group (an untranslated page can sit in a group of its own, under the English name).
        expect(b.groups.flatMap((g) => g.pages.map((p) => p.id)).sort()).toEqual(b.steps.map((p) => p.id).sort());
      }
      expect(books.commander.steps[0].id).toBe("commander");
      expect(books.launcher.steps[0].id).toBe("how-obsidian-works");
    }
  });

  it("keeps the archived Evolved wars out of the Commander pages", () => {
    for (const p of wikiPages("en").books.commander.pages) expect(p.text, p.id).not.toMatch(/evolved/i);
  });

  it("has a translation of every page in every language", () => {
    const ids = wikiPages("en").pages.map((p) => p.id);
    for (const lang of LANGS) {
      const pages = wikiPages(lang).pages;
      expect(pages.map((p) => p.id)).toEqual(ids);
      if (lang !== "en") {
        // A missing file would show the English page, word for word. Titles can match ("Saves"), the text can't.
        const en = wikiPages("en").pages;
        pages.forEach((p, i) => expect(p.text, `${lang} ${p.id}`).not.toBe(en[i].text));
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
        for (const href of hrefs.filter((h) => h.startsWith("app:"))) expect(TABS, `${lang} ${p.id}: ${href}`).toContain(href.slice(4));
        for (const href of hrefs.filter((h) => h.startsWith("wiki:") || h.startsWith("#"))) {
          const [page, anchor] = href.startsWith("#") ? [p.id, href.slice(1)] : href.slice(5).split("#");
          expect(heads.has(page), `${lang} ${p.id}: ${href}`).toBe(true);
          if (anchor) expect(heads.get(page)!.has(anchor), `${lang} ${p.id}: ${href}`).toBe(true);
        }
      }
    }
  });
});
