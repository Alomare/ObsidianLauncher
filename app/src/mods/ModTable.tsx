// The mod table in load order (spec 4.1), with folders. Mods and folders drag to a new place: a
// line shows where they land, and a folder's name lights up when they would go in at the end of that folder.
// Dragging a checked mod takes every checked mod. After a move the rows slide to their places.
// Alt+Up and Alt+Down move the focused mod or folder from the keyboard. A warning icon marks
// mods in a conflict and opens the conflicts; a right click opens the menu. A mod with a newer version
// found shows it in gold in the Version column. Checked mods are checked for updates from the bar under the list.
// The Version and Scope columns have fixed widths (styles.css, .table).
import { memo, useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type KeyboardEvent, type MouseEvent, type MutableRefObject, type PointerEvent } from "react";
import { CheckIcon, ChevronIcon, LockIcon, UnlockIcon, UpArrowIcon, WarnIcon } from "../Icons";
import { go } from "../nav";
import { language, pick, t } from "../i18n";
import { Thumb } from "../Thumb";
import { offered } from "./modUpdates";
import type { ConflictRow, ImageRef, ModRow } from "../types";
import { conflictsByMod } from "./conflicts";
import { dropSpot, folderDropIndex, lines, moveFolder, moveMods, nudge, pinned, type Line, type Node } from "./layout";

export type MenuTarget = { kind: "mod"; id: string } | { kind: "folder"; id: string };

type Props = {
  /** The tree to show, or null while searching: then `rows` show flat and nothing drags. */
  tree: Node[] | null;
  rows: ModRow[];
  byId: Map<string, ModRow>;
  /** Every mod id in load order, for the position numbers. */
  order: string[];
  names: Map<string, string>;
  conflicts: ConflictRow[];
  selected: string | null;
  checked: Set<string>;
  onCheck: (ids: string[], on: boolean) => void;
  onSelect: (id: string) => void;
  /** Switches mods on or off; the switches show the new state until the returned promise settles. */
  onToggle: (ids: string[], on: boolean) => unknown;
  onConflicts: (id: string) => void;
  onArrange: (tree: Node[]) => void;
  onCollapse: (folder: string, collapsed: boolean) => void;
  onLock: (folder: string, locked: boolean) => void;
  onMenu: (x: number, y: number, target: MenuTarget) => void;
  /** New versions found by the last check, by mod id. */
  updates: Record<string, string>;
  onUpdate: (id: string) => void;
};

type Drag = { key: string; folder: string | null; ids: string[]; x0: number; y0: number; moved: boolean; gap: number; onto: string | null; fixed: boolean };
type Shown = { key: string; el: HTMLElement; line: Line };
type Marker = { y: number; color: string | null; onto: string | null; x: number; py: number; label: string };
const THRESHOLD_PX = 5;
const EDGE_PX = 36;

const keyOf = (l: Line) => (l.kind === "folder" ? `folder:${l.folder.id}` : l.id);
/** Clicks on these do their own thing and never start a drag. */
const stop = { onPointerDown: (e: PointerEvent) => e.stopPropagation(), onKeyDown: (e: KeyboardEvent) => e.stopPropagation() };

function Check({ state, label, onChange }: { state: boolean | "mixed"; label: string; onChange: (on: boolean, shift: boolean) => void }) {
  return (
    <button role="checkbox" aria-checked={state} aria-label={label} className={state ? "ck on" : "ck"} {...stop}
      onClick={(e) => onChange(state !== true, e.shiftKey)}>
      {state === "mixed" ? <i className="ck-dash" /> : state ? <CheckIcon /> : null}
    </button>
  );
}

/** Where builds that let the Version and Scope columns be dragged kept the widths. Nothing reads it now. */
export const OLD_COLUMN_KEY = "obsidian.modColumns";

/** What a row calls back into; kept in a ref so the rows never see new functions. */
type Handlers = {
  down: (e: PointerEvent<HTMLDivElement>, line: Line) => void;
  move: (e: PointerEvent<HTMLDivElement>) => void;
  up: (e: PointerEvent<HTMLDivElement>) => void;
  key: (e: KeyboardEvent<HTMLDivElement>, line: Line) => void;
  menu: (e: MouseEvent, target: MenuTarget) => void;
  check: (id: string, on: boolean, shift: boolean) => void;
  toggle: (id: string, on: boolean) => void;
  conflicts: (id: string) => void;
  update: (id: string) => void;
};
type RowProps = {
  id: string; folder: string | null; color: string | null; name: string; description: string; version: string | null; update: string | null;
  scope: string; on: boolean; imagePath: string | null; imageV: number; num: number; pinned: boolean; selected: boolean; checked: boolean;
  lifted: boolean; leaving: boolean; tips: string | null; lose: boolean; h: MutableRefObject<Handlers>;
  /** The UI language: the row's texts change with it. */
  lang: string;
};

/** One mod. Every prop is a plain value, so a click elsewhere in a long list leaves this row alone. */
const ModLine = memo(function ModLine({ id, folder, color, name, description, version, update, scope, on, imagePath, imageV, num, pinned: fixed, selected, checked, lifted, leaving, tips, lose, h }: RowProps) {
  const line: Line = { kind: "mod", id, folder };
  const image: ImageRef | null = useMemo(() => (imagePath ? { path: imagePath, v: imageV } : null), [imagePath, imageV]);
  const cls = ["tr", folder && "in", fixed && "pinned", selected && "sel", checked && "checked", lifted && "lifted", leaving && "leaving"].filter(Boolean).join(" ");
  return (
    <div data-key={id} role="row" tabIndex={0} className={cls} aria-selected={selected} style={color ? ({ "--fc": color } as CSSProperties) : undefined}
      onPointerDown={(e) => h.current.down(e, line)} onPointerMove={(e) => h.current.move(e)} onPointerUp={(e) => h.current.up(e)} onPointerCancel={(e) => h.current.up(e)}
      onKeyDown={(e) => h.current.key(e, line)} onContextMenu={(e) => h.current.menu(e, { kind: "mod", id })}>
      <Check state={checked} label={name} onChange={(v, shift) => h.current.check(id, v, shift)} />
      <span className="num">{num}</span>
      <button role="switch" aria-checked={on} aria-label={name} className={on ? "sw on" : "sw"} data-tip={t(on ? "app.mods.turnOff" : "app.mods.turnOn", name)}
        {...stop} onClick={() => h.current.toggle(id, !on)} />
      <div className="mod-cell" data-tip-title={description ? name : undefined} data-tip={description || undefined}>
        <Thumb image={image} name={name} />
        <div className="who">
          <div className="nm">
            <span className="nm-text">{name}</span>
          </div>
        </div>
      </div>
      {tips ? (
        <button className={lose ? "bare cf-ico lose" : "bare cf-ico"} aria-label={t("app.drawer.conflicts")}
          data-tip-title={t("app.drawer.conflicts")} data-tip={tips} {...stop} onClick={() => h.current.conflicts(id)}>
          <WarnIcon filled={!lose} />
        </button>
      ) : <span />}
      {update ? (
        <button className="ver ver-up" data-tip-title={t("app.mods.updateTo", update)} data-tip={t("app.mods.updateHint")} {...stop} onClick={() => h.current.update(id)}>
          <span>{version ?? t("app.mods.imported")}</span><UpArrowIcon />
        </button>
      ) : <span className="ver">{version ?? t("app.mods.imported")}</span>}
      <button className={`bare scope ${scope}`} aria-label={t(`app.scope.${scope}`)} data-tip-title={t(`app.scope.${scope}`)} data-tip={`${t(`app.scope.${scope}Hint`)}
${t("app.scope.wikiHint")}`}
        {...stop} onClick={() => go("wiki", `scopes#${SCOPE_ANCHORS[scope] ?? ""}`)}><i /></button>
    </div>
  );
});

/** The part of the Scopes page in the Wiki about each scope. */
const SCOPE_ANCHORS: Record<string, string> = { client: "client-mods", session: "session-mods", expansion: "expansions", unspecified: "unspecified-mods", library: "libraries" };

/** How long the mods of a closing folder take to fade before the rows below move up. */
const CLOSE_MS = 140;

export function ModTable({ tree: saved, rows, byId, order, names, conflicts, selected, checked, onCheck, onSelect, onToggle, onConflicts, onArrange, onCollapse, onLock, onMenu, updates, onUpdate }: Props) {
  const body = useRef<HTMLDivElement>(null);
  const drag = useRef<Drag | null>(null);
  const before = useRef<Map<string, number> | null>(null);
  const lastCheck = useRef<string | null>(null);
  useEffect(() => {
    try { localStorage.removeItem(OLD_COLUMN_KEY); } catch { /* storage may be off; nothing reads the key */ }
  }, []);
  const [marker, setMarker] = useState<Marker | null>(null);
  // Folders open and close on screen at once; the saved state follows.
  const [fold, setFold] = useState<Record<string, boolean>>({});
  const [leaving, setLeaving] = useState<string | null>(null);
  // A switch flips at once and holds the new state while the change saves; a later click on the
  // same switch takes over, so an earlier answer can't flip it back.
  const [flips, setFlips] = useState<Record<string, { on: boolean; n: number }>>({});
  const flipCount = useRef(0);
  const toggle = async (ids: string[], on: boolean) => {
    const n = ++flipCount.current;
    setFlips((f) => ({ ...f, ...Object.fromEntries(ids.map((id) => [id, { on, n }])) }));
    try {
      await onToggle(ids, on);
    } finally {
      setFlips((f) => Object.fromEntries(Object.entries(f).filter(([id, v]) => !(ids.includes(id) && v.n === n))));
    }
  };
  const tree = useMemo(
    () => saved?.map((n) => (n.kind === "folder" && fold[n.folder.id] !== undefined ? { ...n, folder: { ...n.folder, collapsed: fold[n.folder.id] } } : n)) ?? null,
    [saved, fold],
  );
  useEffect(() => {
    // Overrides the saved state has caught up with are dropped.
    const done = Object.keys(fold).filter((id) => saved?.some((n) => n.kind === "folder" && n.folder.id === id && n.folder.collapsed === fold[id]));
    if (done.length) setFold((f) => Object.fromEntries(Object.entries(f).filter(([id]) => !done.includes(id))));
  }, [saved]); // eslint-disable-line react-hooks/exhaustive-deps
  const collapse = (id: string, closed: boolean) => {
    const apply = () => {
      measure();
      setFold((f) => ({ ...f, [id]: closed }));
      setLeaving(null);
      onCollapse(id, closed);
    };
    if (!closed || document.documentElement.dataset.motion === "off") return apply();
    setLeaving(id);
    setTimeout(apply, CLOSE_MS);
  };
  const all: Line[] = tree ? lines(tree) : rows.map((r) => ({ kind: "mod", id: r.id, folder: null }));
  const visibleMods = all.flatMap((l) => (l.kind === "mod" ? [l.id] : []));

  const rowEls = () => [...(body.current?.querySelectorAll<HTMLElement>("[data-key]") ?? [])];
  /** Where every row is now, so the next render can slide them from there. */
  const measure = () => {
    before.current = new Map(rowEls().map((el) => [el.dataset.key!, el.getBoundingClientRect().top]));
  };
  useLayoutEffect(() => {
    const old = before.current;
    before.current = null;
    if (!old) return;
    // Every position is read before any row moves, and one reflow serves them all: reading a row
    // after moving the one before it would lay out the whole list again for each row.
    const moved: [HTMLElement, number][] = [];
    for (const el of rowEls()) {
      const prev = old.get(el.dataset.key!);
      if (prev === undefined) continue;
      const dy = prev - el.getBoundingClientRect().top;
      if (dy) moved.push([el, dy]);
    }
    if (!moved.length) return;
    for (const [el, dy] of moved) {
      el.style.transition = "none";
      el.style.transform = `translateY(${dy}px)`;
    }
    body.current?.getBoundingClientRect();
    for (const [el] of moved) {
      el.style.transition = "";
      el.style.transform = "";
    }
  });

  const arrange = (next: Node[]) => {
    measure();
    onArrange(next);
  };

  /** The rows still in place while `d` is dragged. */
  const shownFor = (d: Drag): Shown[] => {
    const els = new Map(rowEls().map((el) => [el.dataset.key!, el]));
    return all
      .filter((l) => (d.folder ? !(l.kind === "folder" && l.folder.id === d.folder) && !(l.kind === "mod" && l.folder === d.folder) : !(l.kind === "mod" && d.ids.includes(l.id))))
      .flatMap((line) => {
        const el = els.get(keyOf(line));
        return el ? [{ key: keyOf(line), el, line }] : [];
      });
  };

  const down = (e: PointerEvent<HTMLDivElement>, line: Line) => {
    if (e.button !== 0) return;
    const ids = line.kind === "folder" ? [] : (checked.has(line.id) ? order.filter((id) => checked.has(id)) : [line.id]).filter((id) => !tree || !pinned(tree, id));
    // Locked folders and the mods in them stay put; a click still opens or folds them.
    const fixed = line.kind === "folder" ? line.folder.locked : !ids.length;
    drag.current = { key: keyOf(line), folder: line.kind === "folder" ? line.folder.id : null, ids, x0: e.clientX, y0: e.clientY, moved: false, gap: -1, onto: null, fixed };
    e.currentTarget.setPointerCapture(e.pointerId);
  };
  const move = (e: PointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d || !tree || d.fixed) return;
    if (!d.moved && Math.abs(e.clientY - d.y0) < THRESHOLD_PX) return;
    d.moved = true;
    // Near the top or bottom edge the list scrolls.
    const box = body.current!.getBoundingClientRect();
    if (e.clientY < box.top + EDGE_PX) body.current!.scrollTop -= 14;
    else if (e.clientY > box.bottom - EDGE_PX) body.current!.scrollTop += 14;
    const shown = shownFor(d);
    const rects = shown.map((s) => s.el.getBoundingClientRect());
    d.onto = null;
    if (!d.folder) {
      const i = shown.findIndex((s, j) => s.line.kind === "folder" && e.clientY > rects[j].top + rects[j].height * 0.25 && e.clientY < rects[j].bottom - rects[j].height * 0.25);
      if (i >= 0) d.onto = (shown[i].line as Line & { kind: "folder" }).folder.id;
    }
    d.gap = rects.filter((r) => r.top + r.height / 2 < e.clientY).length;
    const at = shown[d.gap];
    const last = shown.at(-1);
    const y = at ? at.el.offsetTop : last ? last.el.offsetTop + last.el.offsetHeight : 0;
    // A line that lands the mods in a folder takes the colour of that folder.
    const into = d.folder || d.onto ? null : dropSpot(tree, shown.map((s) => s.line), d.gap, null, d.ids).folder;
    const color = into ? (folders.get(into) as (Node & { kind: "folder" }) | undefined)?.folder.color ?? null : null;
    const label = d.folder
      ? (tree.find((n) => n.kind === "folder" && n.folder.id === d.folder) as Node & { kind: "folder" }).folder.name
      : d.ids.length === 1 ? names.get(d.ids[0]) ?? d.ids[0] : t("app.mods.nMods", d.ids.length);
    setMarker({ y, color, onto: d.onto, x: e.clientX, py: e.clientY, label });
  };
  const up = (e: PointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    drag.current = null;
    setMarker(null);
    if (!d || e.type === "pointercancel") return;
    if (!d.moved) {
      if (d.folder) {
        const f = tree?.find((n) => n.kind === "folder" && n.folder.id === d.folder);
        if (f?.kind === "folder") collapse(f.folder.id, !f.folder.collapsed);
      } else onSelect(d.key);
      return;
    }
    if (!tree || d.gap < 0) return;
    const shown = shownFor(d).map((s) => s.line);
    arrange(d.folder ? moveFolder(tree, d.folder, folderDropIndex(tree, d.gap, d.folder)) : moveMods(tree, d.ids, dropSpot(tree, shown, d.gap, d.onto, d.ids)));
  };
  const key = (e: KeyboardEvent<HTMLDivElement>, line: Line) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      if (line.kind === "folder") collapse(line.folder.id, !line.folder.collapsed);
      else onSelect(line.id);
    } else if (tree && e.altKey && (e.key === "ArrowUp" || e.key === "ArrowDown")) {
      e.preventDefault();
      const by = e.key === "ArrowUp" ? -1 : 1;
      if (line.kind === "mod") arrange(nudge(tree, line.id, by));
      else arrange(moveFolder(tree, line.folder.id, Math.max(0, tree.findIndex((n) => n.kind === "folder" && n.folder.id === line.folder.id) + by)));
    } else if (e.key === "ContextMenu" || (e.shiftKey && e.key === "F10")) {
      e.preventDefault();
      const r = e.currentTarget.getBoundingClientRect();
      onMenu(r.left + 40, r.top + r.height / 2, line.kind === "folder" ? { kind: "folder", id: line.folder.id } : { kind: "mod", id: line.id });
    }
  };
  const menu = (e: MouseEvent, target: MenuTarget) => {
    e.preventDefault();
    onMenu(e.clientX, e.clientY, target);
  };
  /** Checks one mod; with Shift, every mod between it and the one checked last. */
  const check = (id: string, on: boolean, shift: boolean) => {
    const a = lastCheck.current ? visibleMods.indexOf(lastCheck.current) : -1;
    const b = visibleMods.indexOf(id);
    lastCheck.current = id;
    onCheck(shift && a >= 0 ? visibleMods.slice(Math.min(a, b), Math.max(a, b) + 1) : [id], on);
  };

  // Worked out once per change instead of once per row: positions, folder colours, conflict tips.
  const position = useMemo(() => new Map(order.map((id, i) => [id, i + 1])), [order]);
  const folders = useMemo(() => new Map((tree ?? []).flatMap((n) => (n.kind === "folder" ? [[n.folder.id, n] as const] : []))), [tree]);
  const tipsOf = useMemo(() => {
    const nameOf = (id: string) => names.get(id) ?? id;
    const withFiles = ([id, n]: [string, number]) => (n === 1 ? t("app.mods.withOneFile", nameOf(id)) : t("app.mods.withFiles", nameOf(id), n));
    const out = new Map<string, { tips: string; lose: boolean }>();
    for (const [id, c] of conflictsByMod(conflicts)) {
      const tips = [
        c.overrides.length > 0 && t("app.mods.overridesList", c.overrides.map(withFiles).join(", ")),
        c.overriddenBy.length > 0 && t("app.mods.overriddenByList", c.overriddenBy.map(withFiles).join(", ")),
        t("app.mods.conflictsClick"),
      ].filter(Boolean);
      out.set(id, { tips: tips.join("\n"), lose: c.overriddenBy.length > 0 });
    }
    return out;
  }, [conflicts, names]);
  const h = useRef<Handlers>(null!);
  h.current = { down, move, up, key, menu, check, toggle: (id, on) => toggle([id], on), conflicts: onConflicts, update: onUpdate };
  const allState = visibleMods.length && visibleMods.every((id) => checked.has(id)) ? true : visibleMods.some((id) => checked.has(id)) ? "mixed" : false;
  const draggingKeys = new Set(drag.current?.moved ? (drag.current.folder ? [drag.current.key] : drag.current.ids) : []);

  return (
    <div className="table" role="table">
      <div className="th cap" role="row">
        <Check state={allState} label={t("app.mods.checkAll")} onChange={(on) => onCheck(visibleMods, on)} />
        <span data-tip-title={t("app.mods.orderTitle")} data-tip={tree ? t("app.mods.orderHint") : t("app.mods.reorderOff")}>#</span>
        <span>{t("app.mods.colOn")}</span>
        <span>{t("app.mods.colMod")}</span>
        <span />
        <span className="th-col">{t("app.mods.colVersion")}</span>
        <span className="th-col th-scope">{t("app.mods.colScope")}</span>
      </div>
      <div ref={body} className={marker ? "tbody dragging-list" : "tbody"}>
        {all.map((line) => {
          if (line.kind === "folder") {
            const f = line.folder;
            const node = folders.get(f.id);
            const ids = node?.kind === "folder" ? node.ids : [];
            const state = ids.length && ids.every((id) => checked.has(id)) ? true : ids.some((id) => checked.has(id)) ? "mixed" : false;
            const cls =["tr", "fr", f.collapsed && "closed", f.locked && "locked", marker?.onto === f.id && !f.locked && "onto", draggingKeys.has(keyOf(line)) && "lifted"].filter(Boolean).join(" ");
            return (
              <div key={keyOf(line)} data-key={keyOf(line)} role="row" tabIndex={0} className={cls} style={{ "--fc": f.color } as CSSProperties} aria-expanded={!f.collapsed}
                onPointerDown={(e) => down(e, line)} onPointerMove={move} onPointerUp={up} onPointerCancel={up} onKeyDown={(e) => key(e, line)} onContextMenu={(e) => menu(e, { kind: "folder", id: f.id })}>
                {ids.length ? <Check state={state} label={f.name} onChange={(v) => onCheck(ids, v)} /> : <span />}
                <span className="fr-chev"><ChevronIcon /></span>
                <span />
                <span className="fr-title">
                  <button className={f.locked ? "bare fr-lock on" : "bare fr-lock"} aria-pressed={f.locked} aria-label={t(f.locked ? "app.menu.unlock" : "app.menu.lock")}
                    data-tip-title={t(f.locked ? "app.folder.locked" : "app.menu.lock")} data-tip={t(f.locked ? "app.folder.lockedHint" : "app.folder.lockHint")}
                    {...stop} onClick={() => onLock(f.id, !f.locked)}>
                    {f.locked ? <LockIcon /> : <UnlockIcon />}
                  </button>
                  <span className="fr-name">{f.name}</span>
                </span>
                <span className="fr-end">
                  {!ids.length && <span className="fr-count">{t("app.folder.empty")}</span>}
                </span>
              </div>
            );
          }
          const m = byId.get(line.id);
          if (!m) return null;
          const conflict = tipsOf.get(m.id);
          const folder = line.folder ? folders.get(line.folder) : undefined;
          return (
            <ModLine key={m.id} id={m.id} folder={line.folder} color={folder?.kind === "folder" ? folder.folder.color : null} name={names.get(m.id) ?? m.id}
              description={pick(m.description)} version={m.version} update={offered(updates, m)} scope={m.scope} on={flips[m.id]?.on ?? m.on} imagePath={m.image?.path ?? null} imageV={m.image?.v ?? 0} num={position.get(m.id) ?? 0}
              pinned={!!folder && folder.kind === "folder" && folder.folder.locked} selected={selected === m.id} checked={checked.has(m.id)}
              lifted={draggingKeys.has(m.id)} leaving={leaving !== null && line.folder === leaving} tips={conflict?.tips ?? null} lose={conflict?.lose ?? false}
              h={h} lang={language()} />
          );
        })}
        {marker && !marker.onto && <div className="drop-line" style={{ top: marker.y, ...(marker.color ? { ["--lc" as string]: marker.color } : {}) }} aria-hidden="true" />}
      </div>
      {marker && <div className="drag-ghost" style={{ left: marker.x + 14, top: marker.py + 10 }} aria-hidden="true">{marker.label}</div>}
    </div>
  );
}
