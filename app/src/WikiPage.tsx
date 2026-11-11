// The Wiki tab, in two subtabs: Launcher (how Obsidian works, playing online, making mods) and Commander
// (custom Galactic Wars). The list of guides on the left (subtabs, a search, the FAQ and Troubleshooting pinned
// on top), the page in the middle, the headings of the page on the right. Ctrl+F finds words on the open page.
// Pages are Markdown files in src/wiki (see wiki.ts).
import { useEffect, useMemo, useRef, useState, type PointerEvent, type RefObject } from "react";
import { ChevronIcon, SearchIcon } from "./Icons";
import { language, t } from "./i18n";
import { Markdown } from "./Markdown";
import { go, landed, useNav } from "./nav";
import { WIKI_BOOKS, wikiPages, type WikiBook } from "./wiki";

const WIDTH_KEY = "obsidian.wikiListWidth";
/** The page last open in each subtab, so switching back returns to it (this run of the app). */
const lastIn: Partial<Record<WikiBook, string>> = {};

function readWidth(): number | null {
  try {
    const v = Number(localStorage.getItem(WIDTH_KEY));
    return v > 0 ? v : null;
  } catch {
    return null;
  }
}

export function WikiPage() {
  const { wiki: id, anchor } = useNav();
  const lang = language();
  const { pages: every, books } = wikiPages(lang);
  const page = every.find((p) => p.id === id) ?? books.launcher.steps[0];
  const book = page?.book ?? "launcher";
  // The pinned pages have entries above the groups and aren't Previous/Next steps.
  const { pages: all, steps, pinned, groups: allGroups } = books[book];
  const index = steps.indexOf(page);
  useEffect(() => { if (page) lastIn[page.book] = page.id; }, [page]);
  const [query, setQuery] = useState("");
  const [width, setWidth] = useState(readWidth);
  const [at, setAt] = useState("");
  const [find, setFind] = useState<{ n: number; text?: string } | null>(null);
  const reader = useRef<HTMLDivElement>(null);
  const article = useRef<HTMLElement>(null);
  const picked = useRef("");

  const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  const hits = (p: (typeof all)[number]) => words.every((w) => `${p.title} ${p.group} ${p.summary} ${p.text}`.toLowerCase().includes(w));
  const groups = useMemo(
    () => allGroups.map((g) => ({ ...g, pages: g.pages.filter(hits) })).filter((g) => g.pages.length),
    [query, lang, book], // eslint-disable-line react-hooks/exhaustive-deps
  );
  const pinsFound = pinned.filter(hits);
  const switchBook = (b: WikiBook) => {
    if (b === book) return;
    const back = lastIn[b];
    go("wiki", back && books[b].pages.some((p) => p.id === back) ? back : books[b].steps[0]?.id);
  };

  // Ctrl+F opens the find bar, with the words selected on the page if there are some.
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        const sel = window.getSelection();
        const text = sel && !sel.isCollapsed && article.current?.contains(sel.anchorNode) ? sel.toString().trim() : "";
        setFind((f) => ({ n: (f?.n ?? 0) + 1, text: text && text.length <= 80 && !text.includes("\n") ? text : undefined }));
      }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, []);

  // Another page starts at the top, or at the heading a link names.
  const shown = useRef<string | null>(null);
  useEffect(() => {
    const el = anchor ? document.getElementById(anchor) : null;
    if (el) {
      picked.current = anchor;
      setAt(anchor);
      el.scrollIntoView({ block: "start" });
      landed();
    } else if (shown.current !== page?.id) {
      reader.current?.scrollTo({ top: 0 });
      setAt("");
    }
    shown.current = page?.id ?? null;
  }, [page?.id, anchor]);

  if (!page) return null;
  const outline = page.headings.filter((h) => h.depth <= 3);
  /** The outline follows the reading position: the last heading that has passed the top. */
  const follow = () => {
    const box = reader.current;
    if (!box) return;
    const view = box.getBoundingClientRect();
    let now = "";
    for (const h of outline) {
      const el = document.getElementById(h.id);
      if (el && el.getBoundingClientRect().top <= view.top + 90) now = h.id;
    }
    // At the end of the page the last headings never reach the top: the one clicked, else the last in view.
    if (box.scrollTop + box.clientHeight >= box.scrollHeight - 2) {
      const seen = outline.filter((h) => { const el = document.getElementById(h.id); return el && el.getBoundingClientRect().top < view.bottom; });
      now = seen.some((h) => h.id === picked.current) ? picked.current : seen.at(-1)?.id ?? now;
    }
    setAt(now);
  };
  const jump = (heading: string) => {
    picked.current = heading;
    setAt(heading);
    document.getElementById(heading)?.scrollIntoView({ block: "start", behavior: document.documentElement.dataset.motion === "off" ? "auto" : "smooth" });
  };
  const open = (pid: string) => go("wiki", pid);
  const resize = (e: PointerEvent<HTMLDivElement>) => {
    const startX = e.clientX;
    const start = (e.currentTarget.previousElementSibling as HTMLElement).offsetWidth;
    e.currentTarget.setPointerCapture(e.pointerId);
    const move = (m: globalThis.PointerEvent) => setWidth(Math.max(180, Math.min(480, start + m.clientX - startX)));
    const up = (u: globalThis.PointerEvent) => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      try { localStorage.setItem(WIDTH_KEY, String(Math.max(180, Math.min(480, start + u.clientX - startX)))); } catch { /* this visit only */ }
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };
  const prev = steps[index - 1];
  const next = index >= 0 ? steps[index + 1] : undefined;

  return (
    <section className="wiki" aria-label={t("app.nav.wiki")} style={width ? { ["--wiki-w" as string]: `${width}px` } : undefined}>
      <nav className="wiki-nav" aria-label={t("app.wiki.guides")}>
        <div className="subtabs wiki-books" role="tablist">
          {WIKI_BOOKS.map((b) => (
            <button key={b} role="tab" aria-selected={book === b} className={book === b ? "cap cur" : "cap"} data-tip={t(`app.wiki.${b}Tip`)} onClick={() => switchBook(b)}>
              {t(`app.wiki.${b}`)}
            </button>
          ))}
        </div>
        <div className="search wiki-search">
          <SearchIcon />
          <input type="search" placeholder={t("app.wiki.search")} aria-label={t("app.wiki.search")} value={query} onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") { if (query) setQuery(""); else e.currentTarget.blur(); }
              else if (e.key === "Enter" && groups.length) open(groups[0].pages[0].id);
              else if (e.key === "Enter" && pinsFound.length) open(pinsFound[0].id);
            }} />
        </div>
        <div className="wiki-list">
          {pinsFound.map((p) => (
            <button key={p.id} className={p.id === page.id ? `wiki-faq ${p.pin} active` : `wiki-faq ${p.pin}`} aria-current={p.id === page.id ? "page" : undefined}
              onClick={() => open(p.id)} data-tip={p.summary}>
              <span className="wiki-faq-mark" aria-hidden="true">{p.pin === "faq" ? "?" : "!"}</span>
              <b>{p.title}</b>
            </button>
          ))}
          {groups.map((g) => (
            <div key={g.name} className="wiki-group">
              <h2 className="cap">{g.name}</h2>
              {g.pages.map((p) => (
                <button key={p.id} className={p.id === page.id ? "wiki-link active" : "wiki-link"} aria-current={p.id === page.id ? "page" : undefined}
                  onClick={() => open(p.id)} data-tip={p.summary}>{p.title}</button>
              ))}
            </div>
          ))}
          {!groups.length && !pinsFound.length && <p className="empty">{t("app.wiki.noMatch", query)}</p>}
        </div>
      </nav>
      <div className="split" role="separator" aria-orientation="vertical" aria-label={t("app.wiki.resize")} onPointerDown={resize}
        onDoubleClick={() => { setWidth(null); try { localStorage.removeItem(WIDTH_KEY); } catch { /* nothing kept */ } }} />
      <div className={find ? "wiki-read finding" : "wiki-read"} ref={reader} onScroll={follow}>
        {find && <FindBar article={article} reader={reader} page={page.id} open={find} onClose={() => setFind(null)} />}
        <div className="wiki-cols">
          <article className="wiki-article" key={page.id} ref={article}>
            <header>
              <h1>{page.title}</h1>
              {page.summary && <p className="wiki-lead">{page.summary}</p>}
            </header>
            <Markdown tokens={page.tokens} />
            <footer className="wiki-foot">
              {prev ? <button className="wiki-step" onClick={() => open(prev.id)}><small className="cap">{t("app.wiki.previous")}</small>{prev.title}</button> : <span />}
              {next && <button className="wiki-step next" onClick={() => open(next.id)}><small className="cap">{t("app.wiki.next")}</small>{next.title}</button>}
            </footer>
          </article>
          {outline.length > 1 && (
            <aside className="wiki-outline" aria-label={t("app.wiki.onThisPage")}>
              <h2 className="cap">{t("app.wiki.onThisPage")}</h2>
              {outline.map((h) => (
                <button key={h.id} className={[h.depth === 3 && "sub", at === h.id && "active"].filter(Boolean).join(" ")} onClick={() => jump(h.id)}>{h.text}</button>
              ))}
            </aside>
          )}
        </div>
      </div>
    </section>
  );
}

/** Find on the page: every match is marked (CSS highlights, so the page itself doesn't change), the current one in
    gold. Enter and Shift+Enter step through them; Esc closes. Another page is searched for the same words. */
function FindBar({ article, reader, page, open, onClose }: {
  article: RefObject<HTMLElement | null>; reader: RefObject<HTMLDivElement | null>; page: string;
  open: { n: number; text?: string }; onClose: () => void;
}) {
  const [query, setQuery] = useState(open.text ?? "");
  const [matches, setMatches] = useState<Range[]>([]);
  const [now, setNow] = useState(0);
  const box = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (open.text) setQuery(open.text);
    requestAnimationFrame(() => { box.current?.focus(); box.current?.select(); });
  }, [open.n]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    const root = article.current;
    const words = query.replace(/\s+/g, " ").toLowerCase();
    setMatches(root && words.trim() ? findRanges(root, words) : []);
    setNow(0);
  }, [query, page]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    const marks = CSS.highlights;
    if (!marks) return;
    if (matches.length) {
      marks.set("wiki-find", new Highlight(...matches));
      marks.set("wiki-find-now", new Highlight(matches[now]));
    } else {
      marks.delete("wiki-find");
      marks.delete("wiki-find-now");
    }
    const r = matches[now];
    const view = reader.current;
    if (!r || !view) return;
    const a = r.getBoundingClientRect();
    const frame = view.getBoundingClientRect();
    if (a.top < frame.top + 64 || a.bottom > frame.bottom - 24) view.scrollTo({ top: view.scrollTop + a.top - frame.top - frame.height / 3 });
  }, [matches, now]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => () => { CSS.highlights?.delete("wiki-find"); CSS.highlights?.delete("wiki-find-now"); }, []);
  const step = (by: number) => { if (matches.length) setNow((i) => (i + by + matches.length) % matches.length); };
  const count = matches.length ? t("app.wiki.matchOf", now + 1, matches.length) : query.trim() ? t("app.wiki.noMatches") : "";
  return (
    <div className="wiki-find-dock">
      <div className="wiki-find" role="search" aria-label={t("app.wiki.find")}>
        <SearchIcon />
        <input ref={box} type="text" value={query} placeholder={t("app.wiki.find")} aria-label={t("app.wiki.find")} spellCheck={false}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") { e.preventDefault(); step(e.shiftKey ? -1 : 1); }
            else if (e.key === "Escape") { e.preventDefault(); onClose(); }
          }} />
        <span className="wiki-find-count" aria-live="polite">{count}</span>
        <button className="bare up" onClick={() => step(-1)} disabled={!matches.length} aria-label={t("app.wiki.prevMatch")} data-tip={t("app.wiki.prevMatch")}><ChevronIcon /></button>
        <button className="bare" onClick={() => step(1)} disabled={!matches.length} aria-label={t("app.wiki.nextMatch")} data-tip={t("app.wiki.nextMatch")}><ChevronIcon /></button>
        <button className="bare" onClick={onClose} aria-label={t("app.wiki.close")} data-tip={t("app.wiki.close")}>✕</button>
      </div>
    </div>
  );
}

/** The ranges of a page's text where `words` (lower case, single spaces) are. A match can run through bold words or a
    link, but not from one paragraph into the next. Previous/Next aren't searched. */
function findRanges(root: HTMLElement, words: string): Range[] {
  const nodes: { node: Text; at: number }[] = [];
  let text = "";
  let block: Element | null = null;
  const walk = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode: (n) => (n.parentElement?.closest(".wiki-foot") ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT),
  });
  for (let n = walk.nextNode(); n; n = walk.nextNode()) {
    const node = n as Text;
    const b = node.parentElement?.closest("p, li, h1, h2, h3, h4, td, th, pre, blockquote, aside, header, div") ?? null;
    if (b !== block && text) text += "\0";
    block = b;
    nodes.push({ node, at: text.length });
    text += node.data;
  }
  const hay = text.toLowerCase().replace(/\s/g, " ");
  const place = (i: number) => {
    let lo = 0;
    let hi = nodes.length - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (nodes[mid].at <= i) lo = mid;
      else hi = mid - 1;
    }
    return nodes[lo];
  };
  const out: Range[] = [];
  for (let i = hay.indexOf(words); i >= 0 && out.length < 2000; i = hay.indexOf(words, i + words.length)) {
    const a = place(i);
    const b = place(i + words.length - 1);
    const r = document.createRange();
    r.setStart(a.node, i - a.at);
    r.setEnd(b.node, i + words.length - b.at);
    out.push(r);
  }
  return out;
}
