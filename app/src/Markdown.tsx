// Shows Markdown (the Wiki's pages) as React elements. marked does the parsing; this draws the tokens,
// so no HTML string is ever injected and links can stay inside the app.
//
// Beyond plain Markdown, a page can use:
//   [text](wiki:page-id)    another Wiki page ("wiki:page-id#heading" for a part of it)
//   [text](app:mods)        a tab of the app (home, mods, saves, wiki, settings)
//   [text](#a-heading)      a heading on this page          [text](https://…)   the web browser
//   > [!TIP] … / [!NOTE] / [!WARNING]                        a callout
import { openUrl } from "@tauri-apps/plugin-opener";
import { marked, type Token, type Tokens } from "marked";
import { Fragment, type ReactNode } from "react";
import { t } from "./i18n";
import { OutboundIcon } from "./Icons";
import { go, type Tab } from "./nav";

export type Heading = { id: string; text: string; depth: number };
export type Doc = { tokens: Token[]; headings: Heading[]; text: string };

const slug = (text: string) => text.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "") || "section";

/** The words of some tokens, without the markup: for search, and for heading ids. */
export function plainText(tokens: Token[]): string {
  return tokens.map((t): string => {
    const k = t as Tokens.Generic;
    if (k.type === "list") return plainText(k.items);
    if (k.type === "table") return [...k.header, ...k.rows.flat()].map((c: Tokens.TableCell) => plainText(c.tokens)).join(" ");
    if (k.tokens) return plainText(k.tokens);
    return k.type === "html" ? "" : (k.text ?? "");
  }).join(" ");
}

/** Parses Markdown once: the tokens to draw, the headings for the outline (each with the id it gets on the page) and
    the plain text. A heading of a translated page ends in the id of the English one ("## Perfis {#profiles}"), so
    links into it work in every language. */
export function parse(md: string): Doc {
  const tokens = marked.lexer(md);
  const headings: Heading[] = [];
  const used = new Map<string, number>();
  for (const tok of tokens) {
    if (tok.type !== "heading") continue;
    const h = tok as Tokens.Heading;
    const named = /\s*\{#([a-z0-9-]+)\}\s*$/.exec(h.text);
    if (named) {
      h.text = h.text.slice(0, named.index);
      h.tokens = marked.Lexer.lexInline(h.text);
    }
    const text = plainText(h.tokens).trim();
    const base = named?.[1] ?? slug(text);
    const n = used.get(base) ?? 0;
    used.set(base, n + 1);
    const id = n ? `${base}-${n + 1}` : base;
    (tok as Tokens.Generic).id = id;
    headings.push({ id, text, depth: h.depth });
  }
  return { tokens, headings, text: plainText(tokens) };
}

const TABS: Tab[] = ["home", "mods", "saves", "wiki", "settings"];
const CALLOUT = /^\[!(TIP|NOTE|WARNING)\]\s*/;

/** A link to a place in the app. An inline element, not a button, so the line can wrap inside it. */
function InApp({ to, children }: { to: () => void; children: ReactNode }) {
  return (
    <a className="md-link" role="link" tabIndex={0} onClick={(e) => { e.preventDefault(); to(); }}
      onKeyDown={(e) => { if (e.key === "Enter") { e.preventDefault(); to(); } }}>{children}</a>
  );
}

function Link({ href, children }: { href: string; children: ReactNode }) {
  if (/^https?:\/\//i.test(href)) {
    return <a className="md-link" href={href} data-tip={href} onClick={(e) => { e.preventDefault(); openUrl(href).catch(() => {}); }}>{children}<OutboundIcon /></a>;
  }
  if (href.startsWith("#")) return <InApp to={() => document.getElementById(href.slice(1))?.scrollIntoView({ block: "start" })}>{children}</InApp>;
  const [scheme, rest = ""] = href.split(/:(.*)/s);
  if (scheme === "wiki") return <InApp to={() => go("wiki", rest)}>{children}</InApp>;
  if (scheme === "app" && TABS.includes(rest as Tab)) return <InApp to={() => go(rest as Tab)}>{children}</InApp>;
  return <>{children}</>;
}

function Inline({ tokens }: { tokens: Token[] | undefined }): ReactNode {
  return (tokens ?? []).map((tok, i) => {
    const k = tok as Tokens.Generic;
    switch (k.type) {
      case "strong": return <strong key={i}><Inline tokens={k.tokens} /></strong>;
      case "em": return <em key={i}><Inline tokens={k.tokens} /></em>;
      case "del": return <del key={i}><Inline tokens={k.tokens} /></del>;
      case "codespan": return <code key={i}>{k.text}</code>;
      case "br": return <br key={i} />;
      case "link": return <Link key={i} href={k.href}><Inline tokens={k.tokens} /></Link>;
      case "html": return null;
      default: return k.tokens ? <Inline key={i} tokens={k.tokens} /> : <Fragment key={i}>{k.text ?? ""}</Fragment>;
    }
  });
}

/** A blockquote that starts with [!TIP], [!NOTE] or [!WARNING] is a callout: the marker comes off the first line. */
function callout(q: Tokens.Blockquote): { kind: string; tokens: Token[] } | null {
  const m = CALLOUT.exec(q.text);
  const first = q.tokens[0] as Tokens.Generic | undefined;
  if (!m || !first?.tokens?.length) return null;
  const [lead, ...rest] = first.tokens as Tokens.Generic[];
  const stripped = { ...lead, text: String(lead.text ?? "").replace(CALLOUT, "") };
  return { kind: m[1], tokens: [{ ...first, tokens: [stripped, ...rest] } as Token, ...q.tokens.slice(1)] };
}

function Blocks({ tokens }: { tokens: Token[] }): ReactNode {
  return tokens.map((tok, i) => {
    const k = tok as Tokens.Generic;
    switch (k.type) {
      case "heading": {
        const H = (k.depth <= 2 ? "h2" : k.depth === 3 ? "h3" : "h4") as "h2";
        return <H key={i} id={k.id}><Inline tokens={k.tokens} /></H>;
      }
      case "paragraph": return <p key={i}><Inline tokens={k.tokens} /></p>;
      case "text": return <Fragment key={i}><Inline tokens={k.tokens ?? [tok]} /></Fragment>;
      case "list": {
        const L = (k.ordered ? "ol" : "ul") as "ol";
        return (
          <L key={i} start={k.ordered && k.start !== "" && k.start !== 1 ? k.start : undefined}>
            {(k.items as Tokens.ListItem[]).map((item, j) => <li key={j}><Blocks tokens={item.tokens} /></li>)}
          </L>
        );
      }
      case "blockquote": {
        const c = callout(tok as Tokens.Blockquote);
        return c ? (
          <aside key={i} className={`callout callout-${c.kind.toLowerCase()}`}><b className="cap">{t(`app.wiki.${c.kind.toLowerCase()}`)}</b><div><Blocks tokens={c.tokens} /></div></aside>
        ) : <blockquote key={i}><Blocks tokens={k.tokens ?? []} /></blockquote>;
      }
      case "code": return <pre key={i}><code>{k.text}</code></pre>;
      case "hr": return <hr key={i} />;
      case "table": {
        const table = tok as Tokens.Table;
        const cell = (c: Tokens.TableCell, j: number) => {
          const C = (c.header ? "th" : "td") as "td";
          return <C key={j} style={c.align ? { textAlign: c.align } : undefined}><Inline tokens={c.tokens} /></C>;
        };
        return (
          <div key={i} className="table-wrap"><table>
            <thead><tr>{table.header.map(cell)}</tr></thead>
            <tbody>{table.rows.map((r, j) => <tr key={j}>{r.map(cell)}</tr>)}</tbody>
          </table></div>
        );
      }
      default: return null; // space, html, link definitions
    }
  });
}

export function Markdown({ tokens }: { tokens: Token[] }) {
  return <div className="md"><Blocks tokens={tokens} /></div>;
}
