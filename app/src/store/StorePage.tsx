// The Store tab: the Obsidian Kit as a row of small cards, then the other mods as cards with a filter
// row (scopes, sort, Installed, search). A card opens the page of the mod: the description from the
// README, the versions with their notes, the details and the mods it requires. The list comes from
// github.com/Alomare/ObsidianStoreIndex (storeIndex.ts); installing downloads the newest release.
import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { useLocked } from "../busy";
import { CheckIcon, ChevronIcon, CloseIcon, GitHubIcon, ImportIcon, NoImageIcon, OutboundIcon, SearchIcon, UpdateIcon } from "../Icons";
import { language, t } from "../i18n";
import { Markdown, parse } from "../Markdown";
import { go } from "../nav";
import { openUrl } from "../opener";
import { Select } from "../Select";
import type { Scope, Snapshot } from "../types";
import { install, loadStore, markOf, useStore, type StoreMark, type StoreMod } from "./storeIndex";

const FILTER_SCOPES: Scope[] = ["library", "client", "session", "expansion"];
type Sort = "az" | "za" | "scope" | "new" | "updated" | "random";
const SORTS: Sort[] = ["new", "updated", "az", "za", "scope", "random"];
const later = (a: string | null, b: string | null) => ((a ?? "") < (b ?? "") ? 1 : (a ?? "") > (b ?? "") ? -1 : 0);
const byName = (a: StoreMod, b: StoreMod) => a.name.localeCompare(b.name);

export function sortMods(list: StoreMod[], sort: Sort, order: string[]): StoreMod[] {
  const out = list.slice();
  switch (sort) {
    case "az": return out.sort(byName);
    case "za": return out.sort((a, b) => byName(b, a));
    case "scope": return out.sort((a, b) => FILTER_SCOPES.indexOf(a.scope) - FILTER_SCOPES.indexOf(b.scope) || byName(a, b));
    case "new": return out.sort((a, b) => later(a.added, b.added) || byName(a, b));
    case "updated": return out.sort((a, b) => later(a.released, b.released) || byName(a, b));
    case "random": return out.sort((a, b) => order.indexOf(a.id) - order.indexOf(b.id));
  }
}

const shuffled = (ids: string[]) => ids.map((id) => [Math.random(), id] as const).sort((a, b) => a[0] - b[0]).map(([, id]) => id);
const date = (d: string | null) => (d ? new Date(d).toLocaleDateString(language(), { dateStyle: "medium" }) : "");
const src = (path: string) => convertFileSrc(path);

function ModIcon({ m, size = "" }: { m: StoreMod; size?: string }) {
  return (
    <span className={`mico ${size}${m.icon ? "" : " letter"}`}>
      {m.icon ? <img src={src(m.icon)} alt="" draggable={false} /> : <span aria-hidden="true">{[...m.name.trim()][0]?.toUpperCase() ?? "?"}</span>}
    </span>
  );
}

function ScopeTag({ scope }: { scope: Scope }) {
  return (
    <span className={`store-scope ${scope}`} role="img" aria-label={t(`app.scope.${scope}`)} data-tip-title={t(`app.scope.${scope}`)} data-tip={t(`app.scope.${scope}Hint`)} />
  );
}

const Early = ({ m }: { m: StoreMod }) =>
  m.early ? <span className="store-tag early" data-tip={t("app.store.earlyHint", m.version)}>{t("app.store.early")}</span> : null;

/** A corner over the top right of a card: green with a check when installed, yellow with an arrow when the
    Store has a newer version. */
function Flag({ mark }: { mark: StoreMark }) {
  if (mark !== "installed" && mark !== "update") return null;
  const label = mark === "installed" ? t("app.store.installed") : t("app.store.updateFlag");
  return (
    <span className={`c-flag ${mark === "installed" ? "inst" : "upd"}`} role="img" aria-label={label} data-tip={label}>
      {mark === "installed" ? <CheckIcon /> : <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M12 19V5M5.5 11.5 12 5l6.5 6.5" /></svg>}
    </span>
  );
}

function InstallButton({ m, mark, size, onInstall }: { m: StoreMod; mark: StoreMark; size: "small" | "big"; onInstall: () => void }) {
  const locked = useLocked();
  const stop = (e: { stopPropagation: () => void }) => e.stopPropagation();
  if (mark === "installing") {
    return <button className={`btn inst ${size} installing`} disabled onClick={stop}><span className="upd-spin" aria-hidden="true" />{t("app.store.installingShort")}</button>;
  }
  if (mark === "installed") {
    return (
      <button className={`btn inst ${size} done`} onClick={(e) => { stop(e); go("mods"); }} data-tip={t("app.store.showInMods")}>
        <CheckIcon />{t("app.store.installed")}
      </button>
    );
  }
  const update = mark === "update";
  return (
    <button className={`btn inst ${size} ${update ? "play" : "go"}`} disabled={locked} onClick={(e) => { stop(e); onInstall(); }}
      data-tip={update ? t("app.store.updateHint", m.version) : t("app.store.installHint", m.version)}>
      {update ? <UpdateIcon /> : <ImportIcon />}
      {update ? t("app.store.update") : t("app.store.install")}
    </button>
  );
}

/** Enter or Space on a card opens it, as a click does. */
const keyOpen = (open: () => void) => (e: KeyboardEvent) => {
  if (e.target !== e.currentTarget || (e.key !== "Enter" && e.key !== " ")) return;
  e.preventDefault();
  open();
};

export function StorePage({ snap, home }: { snap: Snapshot; home: number }) {
  const store = useStore();
  const [open, setOpen] = useState<string | null>(null);
  const [text, setText] = useState("");
  const [scopes, setScopes] = useState<Scope[]>([]);
  const [installedOnly, setInstalledOnly] = useState(false);
  const [sort, setSort] = useState<Sort>("new");
  const [order, setOrder] = useState<string[]>([]);
  const scroller = useRef<HTMLDivElement>(null);
  const frontTop = useRef(0);
  useEffect(() => { loadStore(); }, []);
  // A click on the Store tab while it shows a mod goes back to the list.
  useEffect(() => { if (home) setOpen(null); }, [home]);
  const all = store.mods ?? [];
  const mark = (m: StoreMod) => markOf(m, snap.mods, store.installing);
  const doInstall = (m: StoreMod) => install(m, all, snap.mods);
  const show = (id: string | null) => {
    if (id && !open) frontTop.current = scroller.current?.scrollTop ?? 0;
    setOpen(id);
  };
  // The list comes back where it was left; a mod page starts at the top.
  useLayoutEffect(() => {
    if (scroller.current) scroller.current.scrollTop = open ? 0 : frontTop.current;
  }, [open]);

  const list = useMemo(() => {
    const q = text.trim().toLowerCase();
    return sortMods(all.filter((m) => !m.kit), sort, order)
      .filter((m) => !q || `${m.name} ${m.summary}`.toLowerCase().includes(q))
      .filter((m) => !scopes.length || scopes.includes(m.scope))
      .filter((m) => !installedOnly || snap.mods.some((r) => r.id === m.id));
  }, [all, text, scopes, installedOnly, sort, order, snap.mods]);

  const mod = open ? all.find((m) => m.id === open) : undefined;
  let body;
  if (!store.mods) {
    body = store.error ? (
      <div className="store-empty">
        <h2>{t("app.store.failed")}</h2>
        <p>{store.error}</p>
        <button className="btn" onClick={() => loadStore(true)} disabled={store.loading}>{t("app.drawer.tryAgain")}</button>
      </div>
    ) : (
      <div className="store-empty"><p><span className="upd-spin" aria-hidden="true" />{t("app.store.loading")}</p></div>
    );
  } else if (mod) {
    body = <ModPage m={mod} all={all} snap={snap} mark={mark} onOpen={show} onInstall={doInstall} />;
  } else {
    const filtered = !!(text || scopes.length || installedOnly);
    body = (
      <div className="store-in">
        {store.offline && (
          <div className="store-note">
            {t("app.store.offline")}
            <button className="link small" onClick={() => loadStore(true)} disabled={store.loading}>{t("app.drawer.tryAgain")}</button>
          </div>
        )}
        <div className="kit-grid">
          {all.filter((m) => m.kit).map((m) => (
            <div key={m.id} className="kit-card" role="link" tabIndex={0} onClick={() => show(m.id)} onKeyDown={keyOpen(() => show(m.id))}>
              <ModIcon m={m} />
              <div className="kc-text">
                <b>{m.name}</b>
                <div className="kc-tags"><ScopeTag scope={m.scope} /><Early m={m} /></div>
              </div>
            </div>
          ))}
        </div>
        <div className="fbar">
          <div className="fbar-left">
            {FILTER_SCOPES.map((s) => {
              const on = scopes.includes(s);
              return (
                <button key={s} className={on ? "chip on" : "chip"} aria-pressed={on} onClick={() => setScopes(on ? scopes.filter((x) => x !== s) : [...scopes, s])}>
                  <span className={`scope ${s}`}><i /></span><span>{t(`app.scope.${s}`)}</span>
                </button>
              );
            })}
            <span className="sep" />
            <Select value={sort} label={t("app.store.sort")} prefix={t("app.store.sort")}
              options={SORTS.map((s) => ({ value: s, label: t(`app.store.sort_${s}`) }))}
              onChange={(s) => { setSort(s); if (s === "random") setOrder(shuffled(all.map((m) => m.id))); }} />
            <button role="checkbox" aria-checked={installedOnly} className="check-line" onClick={() => setInstalledOnly(!installedOnly)}>
              <span className={installedOnly ? "ck on" : "ck"}>{installedOnly && <CheckIcon />}</span>{t("app.store.installed")}
            </button>
            {filtered && (
              <button className="chip clear" onClick={() => { setText(""); setScopes([]); setInstalledOnly(false); }}><CloseIcon />{t("app.store.clear")}</button>
            )}
          </div>
          <label className="search">
            <SearchIcon />
            <input type="search" value={text} onChange={(e) => setText(e.target.value)} placeholder={t("app.store.search")} aria-label={t("app.store.search")} />
          </label>
        </div>
        {list.length ? (
          <div className="grid-cards">
            {list.map((m) => {
              const k = mark(m);
              return (
                <div key={m.id} className="card" role="link" tabIndex={0} onClick={() => show(m.id)} onKeyDown={keyOpen(() => show(m.id))}>
                  <div className="c-img">
                    {m.banner ? <img src={src(m.banner)} alt="" draggable={false} loading="lazy" /> : <div className="noimg"><NoImageIcon /></div>}
                    {k !== "installed" && <div className="c-quick"><InstallButton m={m} mark={k} size="small" onInstall={() => doInstall(m)} /></div>}
                  </div>
                  <Flag mark={k} />
                  <div className="c-body">
                    <ModIcon m={m} />
                    <b>{m.name}</b>
                    <p className="c-desc">{m.summary}</p>
                    <div className="c-tags"><ScopeTag scope={m.scope} /><Early m={m} /></div>
                  </div>
                </div>
              );
            })}
          </div>
        ) : (
          <div className="store-empty"><h2>{t("app.store.noMatch")}</h2><p>{t("app.store.noMatchHint")}</p></div>
        )}
      </div>
    );
  }
  return <div className="store" ref={scroller}>{body}</div>;
}

type PageProps = { m: StoreMod; all: StoreMod[]; snap: Snapshot; mark: (m: StoreMod) => StoreMark; onOpen: (id: string | null) => void; onInstall: (m: StoreMod) => void };

function ModPage({ m, all, snap, mark, onOpen, onInstall }: PageProps) {
  const [unfolded, setUnfolded] = useState<Record<string, boolean>>({});
  const k = mark(m);
  const row = snap.mods.find((r) => r.id === m.id);
  const desc = useMemo(() => parse(m.description || m.summary).tokens, [m.description, m.summary]);
  const needs = m.requires.map((id) => ({ id, entry: all.find((x) => x.id === id), row: snap.mods.find((r) => r.id === id) }));
  const neededBy = all.filter((x) => x.requires.includes(m.id));
  const url = `https://github.com/${m.repo}`;
  const facts: [string, ReactNode][] = [
    [t("app.store.factVersion"), m.version],
    ...(m.released ? [[t("app.store.factReleased"), date(m.released)] as [string, string]] : []),
    [t("app.store.factScope"), <><span className={`scope ${m.scope}`}><i /></span>{t(`app.scope.${m.scope}`)}</>],
    [t("app.store.factOptions"), m.options ? String(m.options) : t("app.store.none")],
    ...(row?.version ? [[t("app.store.factInstalled"), row.version] as [string, string]] : []),
  ];
  const reqRow = (key: string, target: StoreMod | undefined, name: string, ok: boolean) => (
    <li key={key}>
      <button className="req-row" onClick={() => target && onOpen(target.id)} disabled={!target}>
        {target ? <ModIcon m={target} size="sm" /> : <span className="mico sm letter"><span aria-hidden="true">{[...name][0]?.toUpperCase()}</span></span>}
        <span className="rq-name"><b>{name}</b>{target && <small>{target.version}</small>}</span>
        <span className={ok ? "rq-state ok" : "rq-state"}>{ok ? t("app.store.installed") : target ? t("app.store.notInstalled") : t("app.store.notInStore")}</span>
      </button>
    </li>
  );
  return (
    <div className="store-mod">
      <header className={m.banner ? "mhero" : "mhero plain"}>
        {m.banner && <div className="mh-back" aria-hidden="true"><img src={src(m.banner)} alt="" draggable={false} /></div>}
        <div className="mhero-in">
          <div className="mh-left">
            <button className="crumbs" onClick={() => onOpen(null)}><ChevronIcon />{t("app.nav.store")}</button>
            <div className="mh-title">
              <ModIcon m={m} size="xl" />
              <div>
                <h1>{m.name}</h1>
                <div className="mh-tags"><ScopeTag scope={m.scope} /><Early m={m} />{m.kit && <span className="store-tag">{t("app.store.kit")}</span>}</div>
              </div>
            </div>
            <p className="mh-sum">{m.summary}</p>
            <div className="mh-act">
              <InstallButton m={m} mark={k} size="big" onInstall={() => onInstall(m)} />
              <span className="inst-note">{t("app.store.fromGithub", m.version)}</span>
            </div>
          </div>
          {m.banner && <div className="mh-right"><div className="mh-shot"><img src={src(m.banner)} alt="" draggable={false} /></div></div>}
        </div>
      </header>
      <div className="mbody">
        <div className="mmain">
          <section className="store-box"><Markdown tokens={desc} /></section>
          {m.versions.length > 0 && (
            <section>
              <h2 className="store-h">{t("app.store.versions")}</h2>
              <ol className="store-box mod-log-list">
                {m.versions.map((v, i) => {
                  const unfold = unfolded[v.version] ?? i === 0;
                  return (
                    <li key={v.version} className={unfold ? "mod-log-entry open" : "mod-log-entry"}>
                      <button className="upd-head" aria-expanded={unfold} onClick={() => setUnfolded((u) => ({ ...u, [v.version]: !unfold }))}>
                        <ChevronIcon />
                        <b>{v.version}</b>
                        {v.channel !== "stable" && <span className="upd-badge">{v.channel.length <= 3 ? v.channel.toUpperCase() : v.channel[0].toUpperCase() + v.channel.slice(1)}</span>}
                        {row?.version === v.version && <span className="upd-badge new">{t("app.update.badgeInstalled")}</span>}
                        <span className="date">{date(v.date)}</span>
                      </button>
                      {unfold && (
                        <div className="upd-body">
                          {v.notes.trim() ? <Markdown tokens={parse(v.notes).tokens} /> : <p className="muted">{t("app.update.noNotes")}</p>}
                        </div>
                      )}
                    </li>
                  );
                })}
              </ol>
            </section>
          )}
        </div>
        <aside className="mside">
          <div className="store-box">
            <h3 className="cap">{t("app.store.details")}</h3>
            <dl className="store-facts">{facts.map(([a, b]) => <div key={a}><dt>{a}</dt><dd>{b}</dd></div>)}</dl>
            <button className="btn small" onClick={() => openUrl(url).catch(() => {})} data-tip={url}><GitHubIcon />GitHub<OutboundIcon /></button>
          </div>
          <div className="store-box">
            <h3 className="cap">{t("app.store.requires")}</h3>
            {needs.length ? <ul className="req">{needs.map((n) => reqRow(n.id, n.entry, n.entry?.name ?? n.id, !!n.row))}</ul> : <span className="muted">{t("app.store.requiresNone")}</span>}
            {neededBy.length > 0 && (
              <>
                <h3 className="cap">{t("app.store.neededBy")}</h3>
                <ul className="req">{neededBy.map((x) => reqRow(x.id, x, x.name, snap.mods.some((r) => r.id === x.id)))}</ul>
              </>
            )}
          </div>
        </aside>
      </div>
    </div>
  );
}
