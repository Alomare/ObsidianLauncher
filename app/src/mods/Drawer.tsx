// The drawer beside the mod table (spec 4.2): one mod's About, Options, Keys, Files and Dependencies,
// or the conflicts. With nothing selected it shows the conflicts, unless the X closed them; with no
// conflicts either, the Mods page leaves it out (the user, 2026-11-04).
import { useEffect, useMemo, useState } from "react";
import { run } from "../api";
import { openColorPicker } from "../ColorPicker";
import { Confirm } from "../Confirm";
import { ArrowIcon, ChevronIcon, CloseIcon, ExternalIcon, FileIcon, GamepadIcon, HistoryIcon, InfoIcon, UpdateIcon, KeyboardIcon, KeyboardMouseIcon, LinkIcon, ResetIcon, SlidersIcon, TrashIcon } from "../Icons";
import { language, pick, t } from "../i18n";
import { Markdown, parse } from "../Markdown";
import { Select } from "../Select";
import { compareVersions, type Release } from "../updates";
import { loadMore, refresh, refreshWait, useModReleases } from "./modReleases";
import { installModVersion } from "./modUpdates";
import { Thumb } from "../Thumb";
import type { ConflictRow, KeyRow, ModRow, OptionGroup, OptionPreview, OptionRow } from "../types";
import { previewSrc } from "./preview";
import { overrides, type Override } from "./conflicts";
import { dependencies, type Dep } from "./deps";
import { keyName } from "./keyCapture";
import { PadGlyphs } from "./PadGlyphs";
import { padButtons } from "./padButtons";
import type { DrawerView } from "./ModsPage";
import { clampNumber } from "./optionValue";
import { openUrl } from "../opener";

type Props = {
  view: DrawerView;
  mods: ModRow[];
  /** Conflicts for the order on screen (a drag may be under way). */
  conflicts: ConflictRow[];
  order: string[];
  names: Map<string, string>;
  onOpen: (id: string) => void;
  onClose: () => void;
  /** The X closed the conflicts: they stay away until a conflict icon asks for them. */
  conflictsClosed?: boolean;
};

export function Drawer({ view, mods, conflicts, order, names, onOpen, onClose, conflictsClosed }: Props) {
  const mod = view?.kind === "mod" ? mods.find((m) => m.id === view.id) : undefined;
  if (mod) return <ModDrawer key={mod.id} mod={mod} mods={mods} conflicts={conflicts} names={names} onOpen={onOpen} onClose={onClose} />;
  if (view?.kind === "conflicts" || (conflicts.length && !conflictsClosed)) return <ConflictsView conflicts={conflicts} order={order} names={names} focus={view?.kind === "conflicts" ? view.focus : undefined} onClose={onClose} />;
  return null;
}

type Tab = "about" | "versions" | "options" | "keys" | "files" | "deps";

const TAB_ICONS: Record<Tab, () => React.JSX.Element> = { about: InfoIcon, versions: HistoryIcon, options: SlidersIcon, keys: KeyboardIcon, files: FileIcon, deps: LinkIcon };

function ModDrawer({ mod, mods, conflicts, names, onOpen, onClose }: { mod: ModRow; mods: ModRow[]; conflicts: ConflictRow[]; names: Map<string, string>; onOpen: (id: string) => void; onClose: () => void }) {
  const [tab, setTab] = useState<Tab>("about");
  const [ask, setAsk] = useState(false);
  const name = pick(mod.name, mod.id);
  const { needs, neededBy } = dependencies(mod, mods);
  const tabs: [Tab, number | null][] = [["about", null], ...(mod.updatable ? [["versions", null] as [Tab, null]] : []), ["options", ownOptions(mod).length], ["keys", mod.keybinds.length], ["files", mod.files.length], ["deps", needs.length + neededBy.length]];
  return (
    <aside className={`drawer ${mod.scope}`} aria-label={name}>
      <div className="dh">
        <Thumb image={mod.image} name={name} size="lg" zoom />
        <div className="dh-text">
          <h2>{name}</h2>
          <div className="meta">
            {[mod.authors.join(", "), mod.version ?? t("app.mods.imported"), t(`app.scope.${mod.scope}`)].filter(Boolean).join(" · ")}
          </div>
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
        {tab === "versions" && mod.updatable && <VersionsTab mod={mod} />}
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

/** Which option groups the player folded or opened, by mod and group: kept across mods and restarts. */
const GROUPS_KEY = "obsidian.optionGroups";
function savedFolds(): Record<string, Record<string, boolean>> {
  try {
    const v = JSON.parse(localStorage.getItem(GROUPS_KEY) ?? "{}");
    return v && typeof v === "object" ? v : {};
  } catch { return {}; }
}

/** The Options tab: previews with no group and the options with none first, then each option group
 *  under a heading that folds. A number being dragged shows in the previews before it is saved. */
function OptionsTab({ mod }: { mod: ModRow }) {
  const own = ownOptions(mod);
  const [draft, setDraft] = useState<Record<string, unknown>>({});
  // A saved value replaces the one being dragged; a refresh that changes nothing keeps the drag.
  const saved = JSON.stringify(mod.options.map((o) => o.value));
  useEffect(() => setDraft({}), [saved]);
  const [folds, setFolds] = useState<Record<string, boolean>>(() => savedFolds()[mod.id] ?? {});
  if (!own.length) return <p className="muted pad">{t("app.drawer.noOptions")}</p>;
  const values: Record<string, unknown> = { ...Object.fromEntries(mod.options.map((o) => [o.id, o.value])), ...draft };
  const known = new Set(mod.optionGroups.map((g) => g.id));
  const groupOf = (g: string | null) => (g && known.has(g) ? g : null);
  const folded = (g: OptionGroup) => folds[g.id] ?? g.collapsed;
  const fold = (g: OptionGroup) => setFolds((f) => {
    const next = { ...f, [g.id]: !(f[g.id] ?? g.collapsed) };
    try { localStorage.setItem(GROUPS_KEY, JSON.stringify({ ...savedFolds(), [mod.id]: next })); } catch { /* kept for this session only */ }
    return next;
  });
  const draftOf = (id: string) => (v: unknown) => setDraft((d) => ({ ...d, [id]: v }));
  const previews = (g: string | null) => mod.optionPreviews.filter((p) => groupOf(p.group) === g).map((p) => <Preview key={p.id} p={p} values={values} />);
  const table = (list: OptionRow[]) => list.length > 0 && (
    <ul className="opts opt-table">{list.map((o) => <OptionLine key={o.id} mod={mod} o={o} onDraft={draftOf(o.id)} />)}</ul>
  );
  const shown = own.filter((o) => o.visible);
  return (
    <div className="opt-tab">
      {previews(null)}
      {table(shown.filter((o) => groupOf(o.group) === null))}
      {mod.optionGroups.map((g) => {
        const list = shown.filter((o) => o.group === g.id);
        const pics = previews(g.id);
        if (!list.length && !pics.length) return null;
        const closed = folded(g);
        const about = pick(g.description);
        return (
          <section key={g.id} className={closed ? "opt-group" : "opt-group open"}>
            <button className="opt-group-head" aria-expanded={!closed} onClick={() => fold(g)} data-tip={about || undefined}>
              <ChevronIcon /><span>{pick(g.label, g.id)}</span>
            </button>
            {!closed && <div className="opt-group-body">{pics}{table(list)}</div>}
          </section>
        );
      })}
    </div>
  );
}

function OptionLine({ mod, o, onDraft }: { mod: ModRow; o: OptionRow; onDraft: (v: unknown) => void }) {
  const about = pick(o.description);
  return (
    <li className="opt">
      <div className="opt-text">
        {/* The description shows in the tooltip of the name box, on hover or keyboard focus, with the
            note about Deploy under it; a yellow asterisk after the name says there is one. */}
        <div className="l opt-name" tabIndex={about || o.deploy ? 0 : undefined} data-tip={about || undefined} data-tip-note={o.deploy ? t("app.drawer.afterDeploy") : undefined}>
          {pick(o.label, o.id)}{o.deploy && <span className="opt-star" aria-hidden="true">*</span>}
        </div>
      </div>
      <div className="opt-cell"><OptionControl mod={mod} o={o} onDraft={onDraft} /></div>
      <div className="opt-cell">
        <button className="bare" disabled={same(o.value, o.default)} onClick={() => run("set_option", { id: mod.id, option: o.id, value: null })}
          aria-label={t("app.drawer.reset")} data-tip={t("app.drawer.reset")}><ResetIcon /></button>
      </div>
    </li>
  );
}

/** An option preview: the mod's SVG template with the values being edited, as an image. */
function Preview({ p, values }: { p: OptionPreview; values: Record<string, unknown> }) {
  const key = JSON.stringify(values);
  const src = useMemo(() => previewSrc(p.svg, values), [p.svg, key]); // eslint-disable-line react-hooks/exhaustive-deps
  if (!src) return null;
  const label = pick(p.label);
  return (
    <figure className="opt-preview">
      {label && <figcaption>{label}</figcaption>}
      <img src={src} width={p.width} height={p.height} alt={label || t("app.drawer.preview")} draggable={false} />
    </figure>
  );
}

function OptionControl({ mod, o, onDraft }: { mod: ModRow; o: OptionRow; onDraft?: (v: unknown) => void }) {
  const set = (value: unknown) => run("set_option", { id: mod.id, option: o.id, value });
  const label = pick(o.label, o.id);
  if (o.kind === "toggle") {
    return <button role="switch" aria-checked={o.value === true} aria-label={label} className={o.value === true ? "sw on" : "sw"} onClick={() => set(o.value !== true)} />;
  }
  if (o.kind === "choice") {
    return <Select value={String(o.value)} options={o.choices.map((c, i) => ({ value: c, label: pick(o.choiceLabels[i], c) }))} label={label} onChange={set} fill />;
  }
  if (o.kind === "color") {
    return (
      <button className="color-ctl" aria-label={label} aria-haspopup="dialog" data-tip={String(o.value)}
        onClick={(e) => openColorPicker(e.currentTarget, { current: String(o.value), title: label, onPick: set })}>
        <i style={{ background: String(o.value) }} />
      </button>
    );
  }
  if (o.kind === "number") return <NumberControl o={o} label={label} onSet={set} onDraft={onDraft} />;
  if (o.kind === "key") return <KeyButton value={String(o.value)} label={label} onSet={set} />;
  return <span className="muted">{String(o.value)}</span>;
}

/** A slider and a box. onDraft hears every value on the way, for the previews; onSet only the one kept. */
function NumberControl({ o, label, onSet, onDraft }: { o: OptionRow; label: string; onSet: (v: number) => void; onDraft?: (v: number) => void }) {
  const [v, setV] = useState(Number(o.value));
  useEffect(() => setV(Number(o.value)), [o.value]);
  const change = (n: number) => {
    setV(n);
    if (Number.isFinite(n)) onDraft?.(clampNumber(n, o));
  };
  const commit = (n: number) => {
    const c = clampNumber(n, o);
    setV(c);
    if (c !== Number(o.value)) onSet(c);
  };
  return (
    <div className="num-ctl">
      {o.min !== null && o.max !== null && (
        <input type="range" min={o.min} max={o.max} step={o.step ?? "any"} value={v} aria-label={label}
          style={{ "--pct": `${o.max > o.min ? Math.min(100, Math.max(0, ((v - o.min) / (o.max - o.min)) * 100)).toFixed(2) : 0}%` } as React.CSSProperties}
          onChange={(e) => change(Number(e.target.value))} onPointerUp={() => commit(v)} onKeyUp={() => commit(v)} />
      )}
      <input type="number" value={v} min={o.min ?? undefined} max={o.max ?? undefined} step={o.step ?? "any"} aria-label={label}
        onChange={(e) => change(Number(e.target.value))} onBlur={() => commit(v)} onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && commit(v)} />
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
      <span className="key-text">{waiting ? t("app.drawer.pressKey") : value}</span>
    </button>
  );
}

/** One row per action in five columns: the name, the key on the mouse and keyboard and its reset, the
    controller input when the manifest names one (a keybind's `pad`, a choice option) and its reset. Every
    row shares the columns of the list, so the inputs line up; a head puts an icon over each input. */
function KeysTab({ mod, names }: { mod: ModRow; names: Map<string, string> }) {
  if (!mod.keybinds.length) return <p className="muted pad">{t("app.drawer.noKeys")}</p>;
  const options = new Map(mod.options.map((o) => [o.id, o]));
  return (
    <div className="keys">
      <div className="keys-head">
        <span />
        <span role="img" aria-label={t("app.drawer.keyboard")} data-tip={t("app.drawer.keyboard")}><KeyboardMouseIcon /></span>
        <span />
        <span role="img" aria-label={t("app.drawer.controller")} data-tip={t("app.drawer.controller")}><GamepadIcon /></span>
        <span />
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
                <div className="l" tabIndex={shown?.deploy ? 0 : undefined} data-tip-note={shown?.deploy ? t("app.drawer.afterDeploy") : undefined}>
                  {label}{shown?.deploy && <span className="opt-star" aria-hidden="true">*</span>}
                </div>
                {k.clash.length > 0 && <div className="d warn">{t("app.drawer.keyClash", [pick(mod.name, mod.id), ...k.clash.map((id) => names.get(id) ?? id)].join(", "))}</div>}
              </div>
              <div className="opt-ctl key-col">
                <KeyButton value={key} label={label} clash={k.clash.length > 0} onSet={(v) => run("set_key", { id: mod.id, keybind: k.id, key: v })} />
              </div>
              <div className="opt-ctl reset-col key-reset">
                <button className="bare" disabled={k.key === null} onClick={() => run("set_key", { id: mod.id, keybind: k.id, key: null })}
                  aria-label={t("app.drawer.reset")} data-tip={t("app.drawer.resetKey", k.default)}><ResetIcon /></button>
              </div>
              <div className="opt-ctl pad-col">
                {shown ? <PadControl mod={mod} o={shown} /> : <span className="none" data-tip={t("app.drawer.noController")}>–</span>}
              </div>
              <div className="opt-ctl reset-col pad-reset">
                {shown && (
                  <button className="bare" disabled={same(shown.value, shown.default)} onClick={() => run("set_option", { id: mod.id, option: shown.id, value: null })}
                    aria-label={t("app.drawer.reset")} data-tip={t("app.drawer.reset")}><ResetIcon /></button>
                )}
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
  // A choice that names controller buttons is drawn as them; the full name is in the tooltip and the list.
  const options = o.choices.map((c, i) => {
    const name = pick(o.choiceLabels[i], c);
    const buttons = padButtons(name, c);
    return { value: c, label: name, view: buttons ? <PadGlyphs buttons={buttons} /> : undefined };
  });
  const current = options.find((c) => c.value === String(o.value))?.label;
  return <Select value={String(o.value)} options={options} label={label} tip={current} onChange={(value) => run("set_option", { id: mod.id, option: o.id, value })} />;
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
      {mod.parts.length > 0 && <PartList mod={mod} />}
      <dl className="facts">
        {mod.authors.length > 0 && <><dt>{t("app.drawer.authors")}</dt><dd>{mod.authors.join(", ")}</dd></>}
        <dt>{t("app.drawer.id")}</dt><dd className="mono">{mod.id}</dd>
        {mod.source && <><dt>{t("app.drawer.source")}</dt><dd className="src"><button className="link src-link" aria-label={mod.source} data-tip-title={t("app.drawer.source")} data-tip={mod.source} onClick={() => openUrl(mod.source!).catch(() => {})}><ExternalIcon /></button></dd></>}
        {mod.import && <><dt>{t("app.drawer.imported")}</dt><dd>{date}</dd><dt>{t("app.drawer.file")}</dt><dd className="mono">{mod.import.file}</dd></>}
      </dl>
    </div>
  );
}

/** One line per secondary scope of the mod (Hybrid Scopes): the scope diamond, the name, the
 *  description, and Off while the option that switches it is off. */
function PartList({ mod }: { mod: ModRow }) {
  return (
    <section className="parts" aria-label={t("app.drawer.parts")}>
      <h3 className="cap">{t("app.drawer.parts")}</h3>
      <ul>
        {mod.parts.map((p) => (
          <li key={p.id} className={p.on ? "part" : "part off"}>
            <span className={`scope ${p.scope}`} role="img" aria-label={t(`app.scope.${p.scope}`)} data-tip={t(`app.scope.${p.scope}`)}><i /></span>
            <div className="part-text">
              <div className="part-name">
                {pick(p.name, p.id)}
                {!p.on && <span className="part-off" data-tip={t("app.drawer.partOffHint")}>{t("app.drawer.partOff")}</span>}
              </div>
              {pick(p.description) && <div className="part-about">{pick(p.description)}</div>}
            </div>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** How many versions show at first, and how many more each Load more adds. */
const LOG_STEP = 8;

/** The channel of a version, as the core names it: `stable`, or the first word of the pre-release part. */
export function channelOf(version: string | null): string {
  const pre = version?.split("+")[0].split("-").slice(1).join("-");
  if (!pre) return "stable";
  const word = pre.split(".")[0].match(/^[A-Za-z]+/)?.[0].toLowerCase();
  return word || "pre";
}

/** The channels of a release list: Stable first, then the others by name. */
export function channelsOf(list: Release[]): string[] {
  const set = new Set(list.map((r) => r.channel ?? "stable"));
  return [...set].sort((x, y) => (x === "stable" ? -1 : y === "stable" ? 1 : x.localeCompare(y)));
}

function channelName(c: string): string {
  if (c === "stable") return t("app.drawer.channelStable");
  return c.length <= 3 ? c.toUpperCase() : c[0].toUpperCase() + c.slice(1);
}

/** The Versions tab: the releases of the mod's GitHub repository, one tab per channel (Stable, Beta and
 *  so on, from the tags), newest first and each folded. An unfolded release can be installed in place
 *  of the version the mod has. */
function VersionsTab({ mod }: { mod: ModRow }) {
  const r = useModReleases(mod.id, true);
  const [picked, setPicked] = useState<string | null>(null);
  const [shown, setShown] = useState(LOG_STEP);
  const [unfolded, setUnfolded] = useState<Record<string, boolean>>({});
  const [busy, setBusy] = useState(false);
  const channels = useMemo(() => channelsOf(r.list), [r.list]);
  const mine = channelOf(mod.version);
  const channel = picked ?? (channels.includes(mine) ? mine : channels[0] ?? "stable");
  const latest = useMemo(() => r.list.filter((x) => (x.channel ?? "stable") === "stable").map((x) => x.version).sort(compareVersions).pop() ?? null, [r.list]);
  const entries = useMemo(() => r.list.filter((x) => (x.channel ?? "stable") === channel).sort((x, y) => compareVersions(y.version, x.version)), [r.list, channel]);
  const when = (v: string) => (!mod.version ? "older" : compareVersions(v, mod.version) > 0 ? "new" : compareVersions(v, mod.version) === 0 ? "installed" : "older");
  const more = () => {
    const next = shown + LOG_STEP;
    setShown(next);
    if (next > entries.length && r.more) loadMore(mod.id);
  };
  const install = async (e: Release) => {
    if (!e.tag || busy) return;
    setBusy(true);
    await installModVersion(mod.id, e.tag, pick(mod.name, mod.id), e.version, latest);
    setBusy(false);
  };
  const badge = (w: string) =>
    !mod.version ? null : w === "new" ? <span className="upd-badge new">{t("app.update.badgeNew")}</span> : w === "installed" ? <span className="upd-badge">{t("app.update.badgeInstalled")}</span> : null;
  const first = r.pages === 0 && (r.loading || !r.error);
  // The cooldown of Refresh counts down on the button's tooltip.
  const [, tick] = useState(0);
  const wait = refreshWait();
  useEffect(() => {
    if (wait <= 0) return;
    const timer = setTimeout(() => tick((n) => n + 1), Math.min(wait, 1000));
    return () => clearTimeout(timer);
  }, [wait]);
  const refreshTip = wait > 0 ? t("app.drawer.refreshWait", Math.ceil(wait / 1000)) : t("app.drawer.refreshVersions");
  return (
    <div className="versions pad">
      <div className="versions-head">
        {channels.length > 1 ? (
          <div className="seg versions-channels" role="tablist" aria-label={t("app.drawer.channels")}>
            {channels.map((c) => (
              <button key={c} role="tab" aria-selected={c === channel} className={c === channel ? "on" : undefined} onClick={() => { setPicked(c); setShown(LOG_STEP); }}>
                {channelName(c)}
              </button>
            ))}
          </div>
        ) : <span />}
        <button onClick={() => { setShown(LOG_STEP); refresh(mod.id).then(() => tick((n) => n + 1)); tick((n) => n + 1); }}
          disabled={r.loading} aria-disabled={wait > 0} className={wait > 0 ? "bare versions-refresh cooling" : "bare versions-refresh"} aria-label={t("app.drawer.refreshVersions")} data-tip={refreshTip}>
          <UpdateIcon />
        </button>
      </div>
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
          const w = when(e.version);
          return (
            <li key={e.version} className={`mod-log-entry ${w}${unfold ? " open" : ""}`}>
              <button className="upd-head" aria-expanded={unfold} onClick={() => setUnfolded((u) => ({ ...u, [e.version]: !unfold }))}>
                <ChevronIcon />
                <b>{e.version}</b>
                {badge(w)}
                <span className="date">{e.date ? new Date(e.date).toLocaleDateString(language(), { dateStyle: "medium" }) : ""}</span>
              </button>
              {unfold && (
                <div className="upd-body">
                  {w !== "installed" && e.tag && (
                    <button className="btn small versions-switch" onClick={() => install(e)} disabled={busy}>
                      {t(w === "new" ? "app.drawer.switchUp" : "app.drawer.switchDown", e.version)}
                    </button>
                  )}
                  {e.notes.trim() ? <Markdown tokens={parse(e.notes).tokens} /> : <p className="muted">{t("app.update.noNotes")}</p>}
                  {e.url && <button className="link small" onClick={() => openUrl(e.url).catch(() => {})}>{t("app.update.onGithub")}</button>}
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

function ConflictsView({ conflicts, order, names, focus, onClose }: { conflicts: ConflictRow[]; order: string[]; names: Map<string, string>; focus?: string; onClose: () => void }) {
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
        <button className="bare" onClick={onClose} aria-label={t("app.close")} data-tip={t("app.close")}><CloseIcon /></button>
      </div>
      <div className="dbody">
        {list.length === 0 && <p className="muted pad">{t("app.drawer.noConflicts")}</p>}
        {list.map((o) => <OverrideEntry key={o.winner + "\n" + o.loser} o={o} at={at} names={names} focus={!!focus && involves(o)} />)}
      </div>
    </aside>
  );
}
