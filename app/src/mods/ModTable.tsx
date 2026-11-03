// The mod table in load order (spec 4.1), with folders. Mods and folders drag to a new place: a
// line shows where they land, and a folder's name lights up when they would go in at the end of that folder.
// Dragging a checked mod takes every checked mod. After a move the rows slide to their places.
// Alt+Up and Alt+Down move the focused mod or folder from the keyboard. A warning icon marks
// mods in a conflict and opens the conflicts; a right click opens the menu. The column beside the
// switches asks GitHub for updates to one mod, a folder or the whole list, as the checkboxes do.
// The lines at the left of the Version and Scope headers drag to resize those columns (kept between
// launches); a double click puts them back.
import { memo, useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type KeyboardEvent, type MouseEvent, type MutableRefObject, type PointerEvent } from "react";
import { CheckIcon, ChevronIcon, LockIcon, UnlockIcon, UpArrowIcon, UpdateIcon, WarnIcon } from "../Icons";
import { language, pick, t } from "../i18n";
import { Thumb } from "../Thumb";
import { offered } from "./modUpdates";
import type { ConflictRow, ImageRef, ModRow } from "../types";
import { conflictsByMod } from "./conflicts";
import { drag as dragWidths, readWidths, reset as resetWidths, saveWidths, type Col, type Widths } from "./columns";
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
  onToggle: (ids: string[], on: boolean) => void;
  onConflicts: (id: string) => void;
  onArrange: (tree: Node[]) => void;
  onCollapse: (folder: string, collapsed: boolean) => void;
  onLock: (folder: string, locked: boolean) => void;
  onMenu: (x: number, y: number, target: MenuTarget) => void;
  /** New versions found by the last check, by mod id. */
  updates: Record<string, string>;
  onUpdate: (id: string) => void;
  /** The mods of the update check under way. */
  checking: string[];
  /** `what` names the mod or folder; left out, the whole list. */
  onCheckUpdates: (ids: string[], what?: string) => void;
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

/** Asks GitHub about what its row stands for. The arrows turn while that check runs. */
function UpdateCheck({ label, spinning, off, onCheck }: { label: string; spinning: boolean; off: boolean; onCheck: () => void }) {
  return (
    <button className={spinning ? "bare up-ck spin" : "bare up-ck"} disabled={off} aria-label={label} data-tip-title={label} data-tip={t("app.mods.checkUpdatesHint")} {...stop} onClick={onCheck}>
      <UpdateIcon />
    </button>
  );
}

/** The line at the left of a header cell: drag to move it, double-click to put the columns back. */
function Grip({ label, onStart, onReset }: { label: string; onStart: (e: PointerEvent<HTMLDivElement>) => void; onReset: () => void }) {
  return <div className="col-grip" role="separator" aria-orientation="vertical" aria-label={label} data-tip={t("app.mods.resizeHint")} onPointerDown={onStart} onDoubleClick={onReset} />;
}

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
  checkUpdates: (ids: string[], what?: string) => void;
};
type RowProps = {
  id: string; folder: string | null; color: string | null; name: string; description: string; version: string | null; update: string | null;
  scope: string; on: boolean; imagePath: string | null; imageV: number; num: number; pinned: boolean; selected: boolean; checked: boolean;
  lifted: boolean; leaving: boolean; tips: string | null; lose: boolean; updatable: boolean; spinning: boolean; checkOff: boolean; h: MutableRefObject<Handlers>;
  /** The UI language: the row's texts change with it. */
  lang: string;
};

/** One mod. Every prop is a plain value, so a click elsewhere in a long list leaves this row alone. */
const ModLine = memo(function ModLine({ id, folder, color, name, description, version, update, scope, on, imagePath, imageV, num, pinned: fixed, selected, checked, lifted, leaving, tips, lose, updatable, spinning, checkOff, h }: RowProps) {
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
      {updatable ? <UpdateCheck label={t("app.mods.checkOne", name)} spinning={spinning} off={checkOff} onCheck={() => h.current.checkUpdates([id], name)} /> : <span />}
      <div className="mod-cell" data-tip-title={description ? name : undefined} data-tip={description || undefined}>
        <Thumb image={image} name={name} />
        <div className="who">
          <div className="nm">
            {name}
          </div>
        </div>
      </div>
      {tips ? (
        <button className={lose ? "bare cf-ico lose" : "bare cf-ico"} aria-label={t("app.drawer.conflicts")}
          data-tip-title={t("app.drawer.conflicts")} data-tip={tips} {...stop} onClick={() => h.current.conflicts(id)}>
          <WarnIcon />
        </button>
      ) : <span />}
      {update ? (
        <button className="ver ver-up" data-tip-title={t("app.mods.updateTo", update)} data-tip={t("app.mods.updateHint")} {...stop} onClick={() => h.current.update(id)}>
          <span>{version ?? t("app.mods.imported")}</span><UpArrowIcon />
        </button>
      ) : <span className="ver">{version ?? t("app.mods.imported")}</span>}
      <span className={`scope ${scope}`} role="img" aria-label={t(`app.scope.${scope}Mod`)} data-tip-title={t(`app.scope.${scope}Mod`)} data-tip={t(`app.scope.${scope}Hint`)}><i /></span>
    </div>
  );
});

/** How long the mods of a closing folder take to fade before the rows below move up. */
const CLOSE_MS = 140;

export function ModTable({ tree: saved, rows, byId, order, names, conflicts, selected, checked, onCheck, onSelect, onToggle, onConflicts, onArrange, onCollapse, onLock, onMenu, updates, onUpdate, checking, onCheckUpdates }: Props) {
  const body = useRef<HTMLDivElement>(null);
  const drag = useRef<Drag | null>(null);
  const before = useRef<Map<string, number> | null>(null);
  const lastCheck = useRef<string | null>(null);
  const modHead = useRef<HTMLSpanElement>(null);
  const versionHead = useRef<HTMLSpanElement>(null);
  const scopeHead = useRef<HTMLSpanElement>(null);
  const [widths, setWidths] = useState<Widths>(readWidths);
  const [marker, setMarker] = useState<Marker | null>(null);
  // Folders open and close on screen at once; the saved state follows.
  const [fold, setFold] = useState<Record<string, boolean>>({});
  const [leaving, setLeaving] = useState<string | null>(null);
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
    for (const el of rowEls()) {
      const prev = old.get(el.dataset.key!);
      if (prev === undefined) continue;
      const dy = prev - el.getBoundingClientRect().top;
      if (!dy) continue;
      el.style.transition = "none";
      el.style.transform = `translateY(${dy}px)`;
      el.getBoundingClientRect();
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
  /** Drags the line at the left of a header cell: the widths follow the pointer and are kept when it lets go. */
  const grip = (col: Col) => (e: PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    const el = e.currentTarget;
    el.setPointerCapture(e.pointerId);
    // Widths are worked out from what is on screen, in rem.
    const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
    const wide = (n: HTMLElement | null) => (n?.getBoundingClientRect().width ?? 0) / rem;
    const start: Widths = { version: wide(versionHead.current), scope: wide(scopeHead.current) };
    const mod = wide(modHead.current);
    const x0 = e.clientX;
    let last: Widths | null = null;
    const move = (ev: globalThis.PointerEvent) => {
      last = dragWidths(start, col, (ev.clientX - x0) / rem, mod);
      setWidths(last);
    };
    const up = () => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      el.removeEventListener("pointercancel", up);
      if (last) saveWidths(last);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
    el.addEventListener("pointercancel", up);
  };
  const regrip = (col: Col) => () => {
    const next = resetWidths(widths, col);
    setWidths(next);
    saveWidths(next);
  };
  const h = useRef<Handlers>(null!);
  h.current = { down, move, up, key, menu, check, toggle: (id, on) => onToggle([id], on), conflicts: onConflicts, update: onUpdate, checkUpdates: onCheckUpdates };
  const allState = visibleMods.length && visibleMods.every((id) => checked.has(id)) ? true : visibleMods.some((id) => checked.has(id)) ? "mixed" : false;
  const draggingKeys = new Set(drag.current?.moved ? (drag.current.folder ? [drag.current.key] : drag.current.ids) : []);
  // The whole library, closed folders and all, unlike the checkbox above: a check leaves nothing out.
  const updatable = order.filter((id) => byId.get(id)?.updatable);
  const checkingNow = checking.length > 0;

  return (
    <div className="table" role="table" style={{ "--col-ver": `${widths.version}rem`, "--col-scope": `${widths.scope}rem` } as CSSProperties}>
      <div className="th cap" role="row">
        <Check state={allState} label={t("app.mods.checkAll")} onChange={(on) => onCheck(visibleMods, on)} />
        <span data-tip-title={t("app.mods.orderTitle")} data-tip={tree ? t("app.mods.orderHint") : t("app.mods.reorderOff")}>#</span>
        <span>{t("app.mods.colOn")}</span>
        {updatable.length ? <UpdateCheck label={t("app.mods.checkUpdates")} spinning={updatable.every((id) => checking.includes(id))} off={checkingNow} onCheck={() => onCheckUpdates(updatable)} /> : <span />}
        <span ref={modHead}>{t("app.mods.colMod")}</span>
        <span />
        <span ref={versionHead} className="th-col">
          {t("app.mods.colVersion")}
          <Grip label={t("app.mods.resizeColumn", t("app.mods.colVersion"))} onStart={grip("version")} onReset={regrip("version")} />
        </span>
        <span ref={scopeHead} className="th-col th-scope">
          {t("app.mods.colScope")}
          <Grip label={t("app.mods.resizeColumn", t("app.mods.colScope"))} onStart={grip("scope")} onReset={regrip("scope")} />
        </span>
      </div>
      <div ref={body} className={marker ? "tbody dragging-list" : "tbody"}>
        {all.map((line) => {
          if (line.kind === "folder") {
            const f = line.folder;
            const node = folders.get(f.id);
            const ids = node?.kind === "folder" ? node.ids : [];
            const state = ids.length && ids.every((id) => checked.has(id)) ? true : ids.some((id) => checked.has(id)) ? "mixed" : false;
            const inFolder = ids.filter((id) => byId.get(id)?.updatable);
            const cls =["tr", "fr", f.collapsed && "closed", f.locked && "locked", marker?.onto === f.id && !f.locked && "onto", draggingKeys.has(keyOf(line)) && "lifted"].filter(Boolean).join(" ");
            return (
              <div key={keyOf(line)} data-key={keyOf(line)} role="row" tabIndex={0} className={cls} style={{ "--fc": f.color } as CSSProperties} aria-expanded={!f.collapsed}
                onPointerDown={(e) => down(e, line)} onPointerMove={move} onPointerUp={up} onPointerCancel={up} onKeyDown={(e) => key(e, line)} onContextMenu={(e) => menu(e, { kind: "folder", id: f.id })}>
                {ids.length ? <Check state={state} label={f.name} onChange={(v) => onCheck(ids, v)} /> : <span />}
                <span className="fr-chev"><ChevronIcon /></span>
                <span />
                {inFolder.length ? <UpdateCheck label={t("app.mods.checkOne", f.name)} spinning={inFolder.every((id) => checking.includes(id))} off={checkingNow} onCheck={() => onCheckUpdates(inFolder, f.name)} /> : <span />}
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
              description={pick(m.description)} version={m.version} update={offered(updates, m)} scope={m.scope} on={m.on} imagePath={m.image?.path ?? null} imageV={m.image?.v ?? 0} num={position.get(m.id) ?? 0}
              pinned={!!folder && folder.kind === "folder" && folder.folder.locked} selected={selected === m.id} checked={checked.has(m.id)}
              lifted={draggingKeys.has(m.id)} leaving={leaving !== null && line.folder === leaving} tips={conflict?.tips ?? null} lose={conflict?.lose ?? false}
              updatable={m.updatable} spinning={checking.includes(m.id)} checkOff={checkingNow} h={h} lang={language()} />
          );
        })}
        {marker && !marker.onto && <div className="drop-line" style={{ top: marker.y, ...(marker.color ? { ["--lc" as string]: marker.color } : {}) }} aria-hidden="true" />}
      </div>
      {marker && <div className="drag-ghost" style={{ left: marker.x + 14, top: marker.py + 10 }} aria-hidden="true">{marker.label}</div>}
    </div>
  );
}
