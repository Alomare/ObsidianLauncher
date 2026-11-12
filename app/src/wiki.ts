// The Wiki's pages: Markdown files in src/wiki, each starting with a short header:
//
//   ---
//   title: How Obsidian works
//   group: Getting started       pages with the same group are listed together, groups in order of first use
//   summary: One line for the list's tooltip and under the title.
//   wiki: commander              optional: the Commander subtab; without it the page is in the Launcher subtab
//   pin: faq                     optional: "faq" or "help", listed above the groups and left out of Previous/Next
//   ---
//
// The file name sets the order and the page id ("10-how-obsidian-works.md" is "how-obsidian-works"), so ids
// are unique across both subtabs. A translation has the same file name in a folder named after the language
// (src/wiki/pt-BR/…); a page with no translation shows in English. `wiki` and `pin` are read from the English
// file only, so a translation never moves a page.
import { parse, type Doc } from "./Markdown";

export type WikiBook = "launcher" | "commander";
export const WIKI_BOOKS: WikiBook[] = ["launcher", "commander"];
export type WikiPin = "faq" | "help";

export type WikiPage = Doc & { id: string; title: string; group: string; summary: string; book: WikiBook; pin: WikiPin | null };

const sources = import.meta.glob("./wiki/*.md", { query: "?raw", import: "default", eager: true }) as Record<string, string>;
const translations = import.meta.glob("./wiki/*/*.md", { query: "?raw", import: "default", eager: true }) as Record<string, string>;

function header(md: string) {
  const head = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/.exec(md);
  const about: Record<string, string> = {};
  for (const line of head?.[1].split(/\r?\n/) ?? []) {
    const m = /^(\w+):\s*(.*)$/.exec(line);
    // A value with a colon in it is quoted, as YAML wants.
    if (m) about[m[1]] = m[2].trim().replace(/^"(.*)"$/, "$1");
  }
  return { about, body: head ? md.slice(head[0].length) : md };
}

function page(path: string, md: string, english: string): WikiPage {
  const name = path.split("/").pop()!.replace(/\.md$/, "");
  const { about, body } = header(md);
  const en = header(english).about;
  const doc = parse(body);
  return {
    ...doc, id: name.replace(/^\d+-/, ""), title: about.title || name, group: about.group || "Guides", summary: about.summary || "",
    book: en.wiki === "commander" ? "commander" : "launcher",
    pin: en.pin === "faq" || en.pin === "help" ? en.pin : null,
  };
}

type Book = { pages: WikiPage[]; steps: WikiPage[]; pinned: WikiPage[]; groups: { name: string; pages: WikiPage[] }[] };
const cache = new Map<string, { pages: WikiPage[]; books: Record<WikiBook, Book> }>();

/** Every page in `lang` (English where a page has no translation), and per subtab: its pages, the steps of
 *  Previous/Next, the pinned pages (FAQ first, then Troubleshooting) and the other pages by group. */
export function wikiPages(lang: string) {
  let hit = cache.get(lang);
  if (!hit) {
    const pages = Object.keys(sources).sort().map((path) => page(path, translations[path.replace("./wiki/", `./wiki/${lang}/`)] ?? sources[path], sources[path]));
    const books = {} as Record<WikiBook, Book>;
    for (const b of WIKI_BOOKS) {
      const all = pages.filter((p) => p.book === b);
      const steps = all.filter((p) => !p.pin);
      const groups: Book["groups"] = [];
      for (const p of steps) {
        let g = groups.find((x) => x.name === p.group);
        if (!g) groups.push((g = { name: p.group, pages: [] }));
        g.pages.push(p);
      }
      const pinned = [...all.filter((p) => p.pin === "faq"), ...all.filter((p) => p.pin === "help")];
      books[b] = { pages: all, steps, pinned, groups };
    }
    hit = { pages, books };
    cache.set(lang, hit);
  }
  return hit;
}
