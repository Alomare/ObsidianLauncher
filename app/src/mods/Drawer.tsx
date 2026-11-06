// The drawer beside the mod table (spec 4.2): one mod's About, Options, Keys, Files and Dependencies,
// or the conflicts. With nothing selected it shows the conflicts; with no conflicts either, the Mods page
// leaves it out (the user, 2026-11-04).
import { useEffect, useMemo, useState } from "react";
import { run } from "../api";
import { openColorPicker } from "../ColorPicker";
import { Confirm } from "../Confirm";
import { ArrowIcon, ChevronIcon, CloseIcon, ExternalIcon, FileIcon, GamepadIcon, InfoIcon, KeyboardIcon, KeyboardMouseIcon, LinkIcon, ResetIcon, SlidersIcon, TrashIcon, UpArrowIcon } from "../Icons";
import { language, pick, t } from "../i18n";
import { Markdown, parse } from "../Markdown";
import { Select } from "../Select";
import { changelog, type ChangelogEntry } from "../updates";
import { loadMore, useModReleases } from "./modReleases";
import { offered } from "./modUpdates";
import { Thumb } from "../Thumb";
import type { ConflictRow, KeyRow, ModRow, OptionRow } from "../types";
import { overrides, type Override } from "./conflicts";
import { dependencies, type Dep } from "./deps";
import { keyName } from "./keyCapture";
import type { DrawerView } from "./ModsPage";
import { clampNumber } from "./optionValue";
import { openUrl } from "@tauri-apps/plugin-opener";

type Props = {
  view: DrawerView;
  mods: ModRow[];
  /** Conflicts for the order on screen (a drag may be under way). */
  conflicts: ConflictRow[];
  order: string[];
  names: Map<string, string>;
  onOpen: (id: string) => void;
  onClose: () => void;
  /** New versions found by the last check, by mod id: the panel offers them too. */
  updates: Record<string, string>;
  onUpdate: (id: string) => void;
};

export function Drawer({ view, mods, conflicts, order, names, onOpen, onClose, updates, onUpdate }: Props) {
  const mod = view?.kind === "mod" ? mods.find((m) => m.id === view.id) : undefined;
  if (mod) return <ModDrawer key={mod.id} mod={mod} mods={mods} conflicts={conflicts} names={names} onOpen={onOpen} onClose={onClose} update={offered(updates, mod)} onUpdate={() => onUpdate(mod.id)} />;
  if (view?.kind === "conflicts" || conflicts.length) return <ConflictsView conflicts={conflicts} order={order} names={names} focus={view?.kind === "conflicts" ? view.focus : undefined} onClose={view ? onClose : undefined} />;
  return null;
}

type Tab = "about" | "options" | "keys" | "files" | "deps";

const TAB_ICONS: Record<Tab, () => React.JSX.Element> = { about: InfoIcon, options: SlidersIcon, keys: KeyboardIcon, files: FileIcon, deps: LinkIcon };

function ModDrawer({ mod, mods, conflicts, names, onOpen, onClose, update, onUpdate }: { mod: ModRow; mods: ModRow[]; conflicts: ConflictRow[]; names: Map<string, string>; onOpen: (id: string) => void; onClose: () => void; update: string | null; onUpdate: () => void }) {
  const [tab, setTab] = useState<Tab>("about");
  const [ask, setAsk] = useState(false);
  const name = pick(mod.name, mod.id);
  const { needs, neededBy } = dependencies(mod, mods);
  const tabs: [Tab, number | null][] = [["about", null], ["options", ownOptions(mod).length], ["keys", mod.keybinds.length], ["files", mod.files.length], ["deps", needs.length + neededBy.length]];
  return (
    <aside className={`drawer ${mod.scope}`} aria-label={name}>
      <div className="dh">
        <Thumb image={mod.image} name={name} size="lg" pick={{ owner: "mod", key: mod.id, custom: mod.customImage }} />
        <div className="dh-text">
          <h2>{name}</h2>
          <div className="meta">
            {[mod.authors.join(", "), mod.version ?? t("app.mods.imported"), t(`app.scope.${mod.scope}`)].filter(Boolean).join(" · ")}
          </div>
          {update && (
            <button className="ver-up dh-update" onClick={onUpdate} data-tip={t("app.mods.updateHint")}>
              <span>{t("app.mods.updateTo", update)}</span><UpArrowIcon />
            </button>
          )}
        </div>
        <div className="dh-end">
          <button className="bare" onClick={onClose} aria-label={t("app.close")} data-tip={t("app.close")}><CloseIcon /></button>
          <button className="bare dh-remove" onClick={() => setAsk(true)} aria-label={t("app.drawer.remove")} data-tip={t("app.drawer.remove")}><TrashIcon /></button>
        </div>
      </div>
      <div className="dtabs" role="tablist">
        {tabs.map(([id, n]) => {
          const TabIcon = TAB_ICONS[id];
          const label = t(`app.drawer.${id}`);
          return (
            <button key={id} role="tab" aria-selected={tab === id} aria-label={label} data-tip={label} className={tab === id ? "cur" : undefined} onClick={() => setTab(id)}>
              <TabIcon />{n ? <span className="count">{n}</span> : null}
            </button>
          );
        })}
      </div>
      <div className="dbody">
        {tab === "about" && <AboutTab mod={mod} />}
        {tab === "options" && <OptionsTab mod={mod} />}
        {tab === "keys" && <KeysTab mod={mod} names={names} />}
        {tab === "files" && <FilesTab mod={mod} conflicts={conflicts} names={names} />}
        {tab === "deps" && <DepsTab needs={needs} neededBy={neededBy} names={names} onOpen={onOpen} />}
      </div>
      {ask && (
        <Confirm title={t("app.drawer.removeTitle", name)} text={t("app.drawer.removeText")} confirm={t("app.drawer.remove")}
          onCancel={() => setAsk(false)} onConfirm={async () => { setAsk(false); if (await run("remove_mods", { ids: [mod.id] })) onClose(); }} />
      )}
    </aside>
  );
}

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

/** The options the Options tab lists: all but those a keybind names as its controller input (the Keys tab shows those). */
function ownOptions(mod: ModRow): OptionRow[] {
  const pads = new Set(mod.keybinds.map((k) => k.pad));
  return mod.options.filter((o) => !pads.has(o.id));
}

function OptionsTab({ mod }: { mod: ModRow }) {
  const own = ownOptions(mod);
  if (!own.length) return <p className="muted pad">{t("app.drawer.noOptions")}</p>;
  return (
    <ul className="opts">
      {own.filter((o) => o.visible).map((o) => (
        <li key={o.id} className="opt">
          <div className="opt-text">
            <div className="l">{pick(o.label, o.id)}</div>
            {pick(o.description) && <div className="d">{pick(o.description)}</div>}
            {o.deploy && <div className="d note">{t("app.drawer.afterDeploy")}</div>}
          </div>
          <div className="opt-ctl">
            <OptionControl mod={mod} o={o} />
            <button className="bare" disabled={same(o.value, o.default)} onClick={() => run("set_option", { id: mod.id, option: o.id, value: null })}
              aria-label={t("app.drawer.reset")} data-tip={t("app.drawer.reset")}><ResetIcon /></button>
          </div>
        </li>
      ))}
    </ul>
  );
}

function OptionControl({ mod, o }: { mod: ModRow; o: OptionRow }) {
  const set = (value: unknown) => run("set_option", { id: mod.id, option: o.id, value });
  const label = pick(o.label, o.id);
  if (o.kind === "toggle") {
    return <button role="switch" aria-checked={o.value === true} aria-label={label} className={o.value === true ? "sw on" : "sw"} onClick={() => set(o.value !== true)} />;
  }
  if (o.kind === "choice") {
    return <Select value={String(o.value)} options={o.choices.map((c, i) => ({ value: c, label: pick(o.choiceLabels[i], c) }))} label={label} onChange={set} />;
  }
  if (o.kind === "color") {
    return (
      <button className="color-ctl" aria-label={label} aria-haspopup="dialog" data-tip={String(o.value)}
        onClick={(e) => openColorPicker(e.currentTarget, { current: String(o.value), title: label, onPick: set })}>
        <i style={{ background: String(o.value) }} />
      </button>
    );
  }
  if (o.kind === "number") return <NumberControl o={o} label={label} onSet={set} />;
  if (o.kind === "key") return <KeyButton value={String(o.value)} label={label} onSet={set} />;
  return <span className="muted">{String(o.value)}</span>;
}

function NumberControl({ o, label, onSet }: { o: OptionRow; label: string; onSet: (v: number) => void }) {
  const [v, setV] = useState(Number(o.value));
  useEffect(() => setV(Number(o.value)), [o.value]);
  const commit = (n: number) => {
    const c = clampNumber(n, o);
    setV(c);
    if (c !== Number(o.value)) onSet(c);
  };
  return (
    <div className="num-ctl">
      {o.min !== null && o.max !== null && (
        <input type="range" min={o.min} max={o.max} step={o.step ?? "any"} value={v} aria-label={label}
          onChange={(e) => setV(Number(e.target.value))} onPointerUp={() => commit(v)} onKeyUp={() => commit(v)} />
      )}
      <input type="number" value={v} min={o.min ?? undefined} max={o.max ?? undefined} step={o.step ?? "any"} aria-label={label}
        onChange={(e) => setV(Number(e.target.value))} onBlur={() => commit(v)} onKeyDown={(e) => e.key === "Enter" && commit(v)} />
    </div>
  );
}

/** Shows a key; a click waits for the next key press, Esc cancels. */
function KeyButton({ value, label, onSet, clash }: { value: string; label: string; onSet: (k: string) => void; clash?: boolean }) {
  const [waiting, setWaiting] = useState(false);
  return (
    <button className={["key-btn", waiting && "waiting", clash && "clash"].filter(Boolean).join(" ")} aria-label={`${label}: ${value}`}
      data-tip={waiting ? undefined : t("app.drawer.keyHint")}
      onClick={() => setWaiting(true)} onBlur={() => setWaiting(false)}
      onKeyDown={(e) => {
        if (!waiting) return;
        e.preventDefault();
        e.stopPropagation();
        if (e.key === "Escape") return setWaiting(false);
        const k = keyName(e.nativeEvent);
        if (k) {
          setWaiting(false);
          if (k !== value) onSet(k);
        }
      }}>
      {waiting ? t("app.drawer.pressKey") : value}
    </button>
  );
}

/** One row per action: its key on the mouse and keyboard, and its controller input when the manifest names
    one (a keybind's `pad`, a choice option), under a head with an icon over each column. */
function KeysTab({ mod, names }: { mod: ModRow; names: Map<string, string> }) {
  if (!mod.keybinds.length) return <p className="muted pad">{t("app.drawer.noKeys")}</p>;
  const options = new Map(mod.options.map((o) => [o.id, o]));
  return (
    <div className="keys">
      <div className="keys-head">
        <span />
        <span role="img" aria-label={t("app.drawer.keyboard")} data-tip={t("app.drawer.keyboard")}><KeyboardMouseIcon /></span>
        <span role="img" aria-label={t("app.drawer.controller")} data-tip={t("app.drawer.controller")}><GamepadIcon /></span>
      </div>
      <ul className="opts">
        {mod.keybinds.map((k: KeyRow) => {
          const key = k.key ?? k.default;
          const label = pick(k.label, k.id);
          const pad = k.pad ? options.get(k.pad) : undefined;
          const shown = pad?.visible ? pad : undefined;
          return (
            <li key={k.id} className="opt">
              <div className="opt-text">
                <div className="l">{label}</div>
                {shown && pick(shown.description) && <div className="d">{pick(shown.description)}</div>}
                {shown?.deploy && <div className="d note">{t("app.drawer.afterDeploy")}</div>}
                {k.clash.length > 0 && <div className="d warn">{t("app.drawer.keyClash", [pick(mod.name, mod.id), ...k.clash.map((id) => names.get(id) ?? id)].join(", "))}</div>}
              </div>
              <div className="opt-ctl key-col">
                <KeyButton value={key} label={label} clash={k.clash.length > 0} onSet={(v) => run("set_key", { id: mod.id, keybind: k.id, key: v })} />
                <button className="bare" disabled={k.key === null} onClick={() => run("set_key", { id: mod.id, keybind: k.id, key: null })}
                  aria-label={t("app.drawer.reset")} data-tip={t("app.drawer.resetKey", k.default)}><ResetIcon /></button>
              </div>
              <div className="opt-ctl pad-col">
                {shown ? <PadControl mod={mod} o={shown} /> : <span className="none" data-tip={t("app.drawer.noController")}>–</span>}
              </div>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

/** The controller input of a keybind: its choice option, narrowed to the column, the whole choice in a tooltip. */
function PadControl({ mod, o }: { mod: ModRow; o: OptionRow }) {
  const label = pick(o.label, o.id);
  const options = o.choices.map((c, i) => ({ value: c, label: pick(o.choiceLabels[i], c) }));
  const current = options.find((c) => c.value === String(o.value))?.label;
  return (
    <>
      <Select value={String(o.value)} options={options} label={label} tip={current} onChange={(value) => run("set_option", { id: mod.id, option: o.id, value })} />
      <button className="bare" disabled={same(o.value, o.default)} onClick={() => run("set_option", { id: mod.id, option: o.id, value: null })}
        aria-label={t("app.drawer.reset")} data-tip={t("app.drawer.reset")}><ResetIcon /></button>
    </>
  );
}

function FilesTab({ mod, conflicts, names }: { mod: ModRow; conflicts: ConflictRow[]; names: Map<string, string> }) {
  if (!mod.files.length) return <p className="muted pad">{t("app.drawer.noFiles")}</p>;
  const others = new Map(conflicts.map((c) => [c.resource, c.mods.filter((id) => id !== mod.id)]));
  return (
    <ul className="files">
      {mod.files.map((f) => {
        const o = others.get(f) ?? [];
        return (
          <li key={f}>
            <span className="f">{f}</span>
            {o.length > 0 && <span className="warn">{t("app.drawer.alsoChangedBy", o.map((id) => names.get(id) ?? id).join(", "))}</span>}
          </li>
        );
      })}
    </ul>
  );
}

function DepsTab({ needs, neededBy, names, onOpen }: { needs: Dep[]; neededBy: Dep[]; names: Map<string, string>; onOpen: (id: string) => void }) {
  if (!needs.length && !neededBy.length) return <p className="muted pad">{t("app.drawer.noDeps")}</p>;
  const list = (title: string, deps: Dep[]) => deps.length > 0 && (
    <>
      <h3 className="cap dep-head">{title}</h3>
      <ul className="opts">
        {deps.map((d) => (
          <li key={d.id} className="opt">
            <div className="opt-text">
              {d.state === "missing" ? <div className="l">{d.id}</div> : <button className="link" onClick={() => onOpen(d.id)}>{names.get(d.id) ?? d.id}</button>}
              {d.state === "missing" && <div className="d warn">{t("app.drawer.depMissingHint")}</div>}
            </div>
            <span className={`dep-state ${d.state}`}>{t(`app.drawer.dep.${d.state}`)}</span>
          </li>
        ))}
      </ul>
    </>
  );
  return (
    <>
      <p className="muted pad dep-hint">{t("app.drawer.depsHint")}</p>
      {list(t("app.drawer.requires"), needs)}
      {list(t("app.drawer.neededBy"), neededBy)}
    </>
  );
}

function AboutTab({ mod }: { mod: ModRow }) {
  const date =mod.import ? new Date(mod.import.date).toLocaleString(language(), { dateStyle: "medium", timeStyle: "short" }) : null;
  return (
    <div className="about-mod pad">
      {pick(mod.description) && <p>{pick(mod.description)}</p>}
      <dl className="facts">
        {mod.authors.length > 0 && <><dt>{t("app.drawer.authors")}</dt><dd>{mod.authors.join(", ")}</dd></>}
        <dt>{t("app.drawer.id")}</dt><dd className="mono">{mod.id}</dd>
        {mod.source && <><dt>{t("app.drawer.source")}</dt><dd className="src"><button className="link src-link" aria-label={mod.source} data-tip-title={t("app.drawer.source")} data-tip={mod.source} onClick={() => openUrl(mod.source!).catch(() => {})}><ExternalIcon /></button></dd></>}
        {mod.import && <><dt>{t("app.drawer.imported")}</dt><dd>{date}</dd><dt>{t("app.drawer.file")}</dt><dd className="mono">{mod.import.file}</dd></>}
      </dl>
      {mod.updatable && <ModChangelog mod={mod} />}
    </div>
  );
}

/** How many versions show at first, and how many more each Load more adds. */
const LOG_STEP = 8;
const LOG_OPEN_KEY = "obsidian.modChangelogOpen";
const logOpenSaved = () => { try { return localStorage.getItem(LOG_OPEN_KEY) !== "0"; } catch { return true; } };

/** The releases of the mod's GitHub repository, newest first, each folded. The list starts open;
 *  closed, it stays closed for the next mod too and asks GitHub nothing. */
function ModChangelog({ mod }: { mod: ModRow }) {
  const [open, setOpen] = useState(logOpenSaved);
  const [shown, setShown] = useState(LOG_STEP);
  const [unfolded, setUnfolded] = useState<Record<string, boolean>>({});
  const r = useModReleases(mod.id, open);
  const entries = useMemo(() => changelog(r.list, mod.version ?? "", []), [r.list, mod.version]);
  const toggle = () => {
    setOpen((v) => {
      try { localStorage.setItem(LOG_OPEN_KEY, v ? "0" : "1"); } catch { /* kept for this session only */ }
      return !v;
    });
  };
  const more = () => {
    const next = shown + LOG_STEP;
    setShown(next);
    if (next > r.list.length && r.more) loadMore(mod.id);
  };
  const badge = (e: ChangelogEntry) =>
    !mod.version ? null : e.when === "new" ? <span className="upd-badge new">{t("app.update.badgeNew")}</span> : e.when === "installed" ? <span className="upd-badge">{t("app.update.badgeInstalled")}</span> : null;
  const first = r.pages === 0 && (r.loading || !r.error);
  return (
    <section className={open ? "mod-log open" : "mod-log"}>
      <button className="mod-log-head" aria-expanded={open} onClick={toggle}>
        <ChevronIcon />
        <span className="cap">{t("app.drawer.changelogs")}</span>
      </button>
      {open && (
        <div className="mod-log-body">
          {first && <p className="muted mod-log-note"><span className="upd-spin" aria-hidden="true" />{t("app.drawer.changelogsLoading")}</p>}
          {r.error && (
            <p className="muted mod-log-note">
              {t("app.drawer.changelogsFailed", r.error)} <button className="link small" onClick={() => loadMore(mod.id)} disabled={r.loading}>{t("app.drawer.tryAgain")}</button>
            </p>
          )}
          {!first && !r.error && entries.length === 0 && <p className="muted mod-log-note">{t("app.update.noReleases")}</p>}
          <ol className="mod-log-list">
            {entries.slice(0, shown).map((e) => {
              const unfold = !!unfolded[e.version];
              return (
                <li key={e.version} className={`mod-log-entry ${e.when}${unfold ? " open" : ""}`}>
                  <button className="upd-head" aria-expanded={unfold} onClick={() => setUnfolded((u) => ({ ...u, [e.version]: !unfold }))}>
                    <ChevronIcon />
                    <b>{e.version}</b>
                    {badge(e)}
                    <span className="date">{e.date ? new Date(e.date).toLocaleDateString(language(), { dateStyle: "medium" }) : ""}</span>
                  </button>
                  {unfold && (
                    <div className="upd-body">
                      {e.notes ? <Markdown tokens={parse(e.notes).tokens} /> : <p className="muted">{t("app.update.noNotes")}</p>}
                      {e.url && <button className="link small" onClick={() => openUrl(e.url!).catch(() => {})}>{t("app.update.onGithub")}</button>}
                    </div>
                  )}
                </li>
              );
            })}
          </ol>
          {(entries.length > shown || (r.more && r.pages > 0 && !r.error)) && (
            <button className="btn small mod-log-more" onClick={more} disabled={r.loading}>{r.loading ? t("app.drawer.changelogsLoading") : t("app.drawer.loadMore")}</button>
          )}
        </div>
      )}
    </section>
  );
}

/** One line: the mod that wins, an arrow, the mod it overrides; the files fold out below. */
function OverrideEntry({ o, at, names, focus }: { o: Override; at: (id: string) => number; names: Map<string, string>; focus: boolean }) {
  const [open, setOpen] = useState(false);
  return (
    <div className={focus ? "ov focus" : "ov"}>
      <div className="ov-line">
        <span className="ov-mod win"><span className="num">{at(o.winner)}</span>{names.get(o.winner) ?? o.winner}</span>
        <span className="ov-arrow" role="img" aria-label={t("app.drawer.overrides")} data-tip={t("app.drawer.overrides")}><ArrowIcon /></span>
        <span className="ov-mod"><span className="num">{at(o.loser)}</span>{names.get(o.loser) ?? o.loser}</span>
      </div>
      <button className="link small" aria-expanded={open} onClick={() => setOpen((v) => !v)}>
        {t(o.resources.length === 1 ? "app.drawer.oneFile" : "app.drawer.files_n", o.resources.length)}
      </button>
      {open && <ul className="files">{o.resources.map((r) => <li key={r}><span className="f">{r}</span></li>)}</ul>}
    </div>
  );
}

function ConflictsView({ conflicts, order, names, focus, onClose }: { conflicts: ConflictRow[]; order: string[]; names: Map<string, string>; focus?: string; onClose?: () => void }) {
  const at = (id: string) => order.indexOf(id) + 1;
  const involves = (o: Override) => o.winner === focus || o.loser === focus;
  // Those of the mod the panel was opened for come first.
  const list = overrides(conflicts).sort((a, b) => Number(involves(b)) - Number(involves(a)));
  return (
    <aside className="drawer conflicts" aria-label={t("app.drawer.conflicts")}>
      <div className="dh">
        <div className="dh-text">
          <h2 className="cap">{t("app.drawer.conflicts")}</h2>
          <div className="meta">{t("app.drawer.conflictsMeta")}</div>
        </div>
        {onClose && <button className="bare" onClick={onClose} aria-label={t("app.close")} data-tip={t("app.close")}><CloseIcon /></button>}
      </div>
      <div className="dbody">
        {list.length === 0 && <p className="muted pad">{t("app.drawer.noConflicts")}</p>}
        {list.map((o) => <OverrideEntry key={o.winner + "\n" + o.loser} o={o} at={at} names={names} focus={!!focus && involves(o)} />)}
      </div>
    </aside>
  );
}
