// The Mods page: header, crash banner, the table and the drawer (spec 4.1 and 4.2). Checked mods
// get a bar of batch actions over the foot of the list, so the rows stay where they are; a right
// click opens the same actions for one mod or folder. The line between the table and the drawer
// drags to resize them.
import { useEffect, useMemo, useRef, useState, type PointerEvent } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { call, errorText, run } from "../api";
import { notify } from "../notices";
import { SortDialog, SortIcon, type SortPlan } from "./SortDialog";
import { Confirm } from "../Confirm";
import { ContextMenu, type MenuItem } from "../ContextMenu";
import { ChevronIcon, CloseIcon, ExternalIcon, FolderAddIcon, FolderIcon, ImportIcon, LockIcon, PlusIcon, ProfilesIcon, SearchIcon, SlidersIcon, SwitchOffIcon, SwitchOnIcon, TrashIcon, UnlockIcon, UpdateIcon } from "../Icons";
import { language, pick, t } from "../i18n";
import { useNav } from "../nav";
import { ProfileGuards, ProfilesDialog } from "../ProfilesDialog";
import { Thumb } from "../Thumb";
import type { ModRow, Snapshot } from "../types";
import { winners } from "./conflicts";
import { CrashBanner } from "./CrashBanner";
import { Problems } from "./Problems";
import { Drawer } from "./Drawer";
import { pickAndImport } from "./dropImport";
import { DeleteFolder, FOLDER_COLORS, FolderDialog } from "./FolderDialog";
import { changes, flatten, moveMods, nudge, pinned, toEnd, toTree, type Node } from "./layout";
import { ModTable, type MenuTarget } from "./ModTable";
import { changelogUrl, checkModUpdates, offered, updateMod, updateMods, useModUpdates } from "./modUpdates";
import { filterMods } from "./rows";

/** A mod, or the conflicts with those of one mod first. */
export type DrawerView = { kind: "mod"; id: string } | { kind: "conflicts"; focus?: string } | null;

type Ask =
  | { kind: "newFolder"; ids: string[] }
  | { kind: "editFolder"; id: string }
  | { kind: "deleteFolder"; id: string }
  | { kind: "remove"; ids: string[] }
  | { kind: "update"; id: string }
  | null;

const DRAWER_KEY = "obsidian.drawerWidth";
const DRAWER_MIN_PX = 300;
const LIST_MIN_PX = 420;

function savedWidth(): number {
  try {
    return Number(localStorage.getItem(DRAWER_KEY)) || 0;
  } catch {
    return 0;
  }
}

export function ModsPage({ snap }: { snap: Snapshot }) {
  const [query, setQuery] = useState("");
  const [pending, setPending] = useState<Node[] | null>(null);
  const [view, setView] = useState<DrawerView>(null);
  const [profiles, setProfiles] = useState(false);
  const [checked, setChecked] = useState<Set<string>>(new Set());
  const [menu, setMenu] = useState<{ x: number; y: number; up?: boolean; target: MenuTarget } | null>(null);
  const [addMenu, setAddMenu] = useState<{ x: number; y: number } | null>(null);
  const [profileMenu, setProfileMenu] = useState<{ x: number; y: number; opener: HTMLElement } | null>(null);
  const [ask, setAsk] = useState<Ask>(null);
  const [width, setWidth] = useState(savedWidth);
  const [sortPlan, setSortPlan] = useState<SortPlan | null>(null);
  /** The last sort, for Undo: the tree before it and the order it left. */
  const [sorted, setSorted] = useState<{ before: Node[]; after: string; count: number } | null>(null);
  const modUpdates = useModUpdates();
  const body = useRef<HTMLDivElement>(null);
  const search = useRef<HTMLInputElement>(null);
  const { tab } = useNav();
  const profileImage = snap.profileRows.find((p) => p.name === snap.profile)?.image ?? null;
  /** Other profiles to switch to: the profile button opens a menu of them. */
  const choose = snap.profileRows.length > 1;
  const openProfileMenu = (el: HTMLElement) => {
    const r = el.getBoundingClientRect();
    setProfileMenu({ x: r.left, y: r.bottom + 4, opener: el });
  };
  /** Manage opens the profiles dialog; below it every profile, with its color and guardrails, switches on a click. */
  const profileItems = (): MenuItem[] => [
    { label: t("app.profiles.manage"), icon: <SlidersIcon />, run: () => setProfiles(true) },
    "sep",
    ...snap.profileRows.map((p): MenuItem => ({
      label: p.name, swatch: p.color ?? "var(--gold)", checked: p.name === snap.profile, extra: <span className="ctx-guards"><ProfileGuards p={p} /></span>,
      run: () => { if (p.name !== snap.profile) run("use_profile", { name: p.name }); },
    })),
  ];

  const byId = useMemo(() => new Map(snap.mods.map((m) => [m.id, m])), [snap.mods]);
  const lang = language();
  const names = useMemo(() => new Map(snap.mods.map((m) => [m.id, pick(m.name, m.id)])), [snap.mods, lang]); // eslint-disable-line react-hooks/exhaustive-deps
  const saved = useMemo(() => toTree(snap.mods, snap.folders), [snap.mods, snap.folders]);
  const tree = pending ?? saved;
  const order = useMemo(() => flatten(tree), [tree]);
  const searching = query.trim() !== "";
  const rows = filterMods(order.flatMap((id) => byId.get(id) ?? []), query);
  const shownConflicts = useMemo(() => (pending ? winners(snap.conflicts, order) : snap.conflicts), [snap.conflicts, pending, order]);

  // Checks go with mods that leave the library.
  useEffect(() => {
    setChecked((c) => (([...c].every((id) => byId.has(id))) ? c : new Set([...c].filter((id) => byId.has(id)))));
  }, [byId]);

  // This page stays mounted behind the other tabs, so Ctrl+F only belongs to it while it shows.
  useEffect(() => {
    if (tab !== "mods") return;
    const key = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        search.current?.focus();
        search.current?.select();
      }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [tab]);

  const arrange = async (next: Node[]) => {
    const { ids, moves } = changes(saved, next);
    setPending(next);
    await run("set_order", { ids, moves });
    setPending(null);
  };

  // Sort Mods: the preview first, then the order it showed; Undo puts the tree from before back.
  const openSort = () => call<SortPlan>("sort_preview").then(setSortPlan, (e) => notify("error", errorText(e)));
  const applySort = async (plan: SortPlan) => {
    const before = saved;
    setSortPlan(null);
    if (await run("apply_sort", { ids: plan.order, moved: plan.moves.map((m) => m.id) })) {
      setSorted({ before, after: plan.order.join("\n"), count: plan.moves.length });
    }
  };
  const undoSort = (s: { before: Node[] }) => {
    setSorted(null);
    arrange(s.before);
  };
  // The Undo bar goes with the next change to the order, and after a while.
  useEffect(() => {
    if (sorted && order.join("\n") !== sorted.after) setSorted(null);
  }, [order, sorted]);
  useEffect(() => {
    if (!sorted) return;
    const timer = setTimeout(() => setSorted(null), 15000);
    return () => clearTimeout(timer);
  }, [sorted]);
  const check = (ids: string[], on: boolean) =>
    setChecked((c) => {
      const n = new Set(c);
      for (const id of ids) if (on) n.add(id); else n.delete(id);
      return n;
    });
  const inOrder = (ids: string[]) => order.filter((id) => ids.includes(id));
  const folderOf = (id: string) => snap.folders.find((f) => f.id === id);
  const membersOf = (id: string) => snap.mods.filter((m) => m.folder === id).map((m) => m.id);

  /** The actions for some mods: the menu of a row, and the batch bar's "More". */
  const modItems = (ids: string[]): MenuItem[] => {
    const one = ids.length === 1 ? byId.get(ids[0]) : undefined;
    const allOn = ids.every((id) => byId.get(id)?.on);
    const allOff = ids.every((id) => !byId.get(id)?.on);
    const inFolder = ids.some((id) => byId.get(id)?.folder && !pinned(tree, id));
    const fixed = searching || ids.every((id) => pinned(tree, id));
    const update = one ? offered(modUpdates.found, one) : null;
    return [
      ...(one ? [{ label: t("app.menu.details"), run: () => setView({ kind: "mod", id: one.id }) } as MenuItem] : []),
      ...(one && update ? [{ label: t("app.mods.updateTo", update), run: () => setAsk({ kind: "update", id: one.id }) } as MenuItem] : []),
      ...(one ? ["sep" as const] : []),
      { label: t("app.menu.enable"), disabled: allOn, run: () => run("set_enabled", { ids, on: true }) },
      { label: t("app.menu.disable"), disabled: allOff, run: () => run("set_enabled", { ids, on: false }) },
      "sep",
      { label: t("app.menu.moveTop"), disabled: fixed, run: () => arrange(toEnd(tree, ids, "top")) },
      ...(one ? [
        { label: t("app.menu.moveUp"), hint: "Alt+↑", disabled: fixed, run: () => arrange(nudge(tree, one.id, -1)) } as MenuItem,
        { label: t("app.menu.moveDown"), hint: "Alt+↓", disabled: fixed, run: () => arrange(nudge(tree, one.id, 1)) } as MenuItem,
      ] : []),
      { label: t("app.menu.moveBottom"), disabled: fixed, run: () => arrange(toEnd(tree, ids, "bottom")) },
      {
        label: t("app.menu.moveToFolder"), icon: <FolderIcon />, disabled: fixed,
        sub: [
          ...snap.folders.filter((f) => !f.locked).map((f): MenuItem => ({ label: f.name, swatch: f.color, run: () => arrange(moveMods(tree, inOrder(ids), { folder: f.id, index: Infinity })) })),
          ...(snap.folders.some((f) => !f.locked) ? ["sep" as const] : []),
          { label: t("app.menu.newFolder"), run: () => setAsk({ kind: "newFolder", ids: inOrder(ids) }) },
          ...(inFolder ? [{ label: t("app.menu.outOfFolder"), run: () => arrange(moveFolderless(ids)) } as MenuItem] : []),
        ],
      },
      "sep",
      { label: ids.length > 1 ? t("app.menu.removeN", ids.length) : t("app.menu.remove"), icon: <TrashIcon />, danger: true, run: () => setAsk({ kind: "remove", ids }) },
    ];
  };
  /** Takes mods out of their folders, right after the folder they were in. */
  const moveFolderless = (ids: string[]) => {
    let next = tree;
    for (const id of inOrder(ids).reverse()) {
      const f = byId.get(id)?.folder;
      if (!f) continue;
      const at = next.findIndex((n) => n.kind === "folder" && n.folder.id === f);
      next = moveMods(next, [id], { folder: null, index: at + 1 });
    }
    return next;
  };
  const folderItems = (id: string): MenuItem[] => {
    const f = folderOf(id)!;
    const ids = membersOf(id);
    return [
      { label: f.collapsed ? t("app.menu.expand") : t("app.menu.collapse"), run: () => run("edit_folder", { id, collapsed: !f.collapsed }) },
      { label: f.locked ? t("app.menu.unlock") : t("app.menu.lock"), icon: f.locked ? <UnlockIcon /> : <LockIcon />, run: () => run("edit_folder", { id, locked: !f.locked }) },
      { label: t("app.menu.editFolder"), disabled: f.locked, run: () => setAsk({ kind: "editFolder", id }) },
      { label: t("app.folder.color"), disabled: f.locked, sub: [{ colors: FOLDER_COLORS, current: f.color, pick: (color) => run("edit_folder", { id, color }) }] },
      "sep",
      { label: t("app.menu.enableAll"), disabled: !ids.length || ids.every((m) => byId.get(m)?.on), run: () => run("set_enabled", { ids, on: true }) },
      { label: t("app.menu.disableAll"), disabled: !ids.length || ids.every((m) => !byId.get(m)?.on), run: () => run("set_enabled", { ids, on: false }) },
      { label: t("app.menu.checkAll"), disabled: !ids.length, run: () => check(ids, true) },
      "sep",
      { label: t("app.menu.deleteFolder"), icon: <TrashIcon />, danger: true, disabled: f.locked, run: () => (ids.length ? setAsk({ kind: "deleteFolder", id }) : run("delete_folder", { id, removeMods: false })) },
    ];
  };
  const menuItems = (target: MenuTarget): MenuItem[] =>
    target.kind === "folder" ? folderItems(target.id) : modItems(checked.has(target.id) ? inOrder([...checked]) : [target.id]);

  // The resize handle: the drawer width is kept between visits.
  const resize = (e: PointerEvent<HTMLDivElement>) => {
    const el = e.currentTarget;
    el.setPointerCapture(e.pointerId);
    const box = body.current!.getBoundingClientRect();
    const moveTo = (x: number) => setWidth(Math.round(Math.max(DRAWER_MIN_PX, Math.min(box.right - x, box.width - LIST_MIN_PX))));
    const onMove = (ev: globalThis.PointerEvent) => moveTo(ev.clientX);
    const onUp = () => {
      el.removeEventListener("pointermove", onMove);
      el.removeEventListener("pointerup", onUp);
      setWidth((w) => {
        try { localStorage.setItem(DRAWER_KEY, String(w)); } catch { /* the width is a convenience */ }
        return w;
      });
    };
    el.addEventListener("pointermove", onMove);
    el.addEventListener("pointerup", onUp);
  };

  const sel = inOrder([...checked]);
  const waiting = snap.mods.filter((m) => offered(modUpdates.found, m)).map((m) => m.id);
  const askFolder = ask?.kind === "editFolder" || ask?.kind === "deleteFolder" ? folderOf(ask.id) : undefined;
  // The panel shows a selected mod, or the conflicts; with neither, the list takes the whole width.
  const panel = (view?.kind === "mod" && snap.mods.some((m) => m.id === view.id)) || view?.kind === "conflicts" || shownConflicts.length > 0;

  return (
    <div className="mods">
      <div className="head">
        {/* The profile as a choice, then the tools for its list: Sort and Create Folder as icons, Add as the main one.
            With other profiles to switch to, the profile opens a menu of them; with one, the profiles dialog. */}
        <button className="profile-btn" data-pop aria-haspopup={choose ? "menu" : "dialog"} aria-expanded={choose ? !!profileMenu : undefined}
          aria-label={`${t("app.mods.profile")}: ${snap.profile}`} data-tip-title={t("app.mods.profile")} data-tip={t("app.mods.profileHint")}
          onClick={(e) => (!choose ? setProfiles(true) : profileMenu ? setProfileMenu(null) : openProfileMenu(e.currentTarget))}
          onKeyDown={(e) => { if (choose && (e.key === "ArrowDown" || e.key === "ArrowUp")) { e.preventDefault(); openProfileMenu(e.currentTarget); } }}>
          {profileImage ? <Thumb image={profileImage} name={snap.profile} size="sm" /> : <span className="profile-chip"><ProfilesIcon /></span>}
          <span className="profile-name">{snap.profile}</span>
          {choose && <ChevronIcon />}
        </button>
        {snap.mods.length > 1 && (
          <button className="btn icon head-tool" onClick={openSort} aria-haspopup="dialog" aria-label={t("app.sort.title")} data-tip-title={t("app.sort.title")} data-tip={t("app.sort.hint")}>
            <SortIcon />
          </button>
        )}
        <button className="btn icon head-tool" onClick={() => setAsk({ kind: "newFolder", ids: [] })} aria-haspopup="dialog" aria-label={t("app.mods.createFolder")} data-tip-title={t("app.mods.createFolder")} data-tip={t("app.folder.newHint")}>
          <FolderAddIcon />
        </button>
        <button className="btn go head-tool" data-pop aria-haspopup="menu" aria-expanded={!!addMenu} aria-label={t("app.mods.add")} data-tip-title={t("app.mods.add")} data-tip={t("app.mods.addHint")}
          onClick={(e) => { const r = e.currentTarget.getBoundingClientRect(); setAddMenu({ x: r.left, y: r.bottom + 4 }); }}>
          <PlusIcon /><span className="label">{t("app.mods.addButton")}</span>
        </button>
        {waiting.length > 0 && <button className="btn play head-update" onClick={() => updateMods(waiting, names)}><UpdateIcon />{t("app.mods.updateAll", waiting.length)}</button>}
        <label className="search">
          <SearchIcon />
          <input ref={search} type="text" value={query} placeholder={t("app.mods.search")} aria-label={t("app.mods.search")} onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Escape" && setQuery("")} />
          {query && <button className="bare" onClick={() => setQuery("")} aria-label={t("app.mods.clearSearch")} data-tip={t("app.mods.clearSearch")}><CloseIcon /></button>}
        </label>
      </div>
      <CrashBanner snap={snap} />
      <Problems snap={snap} />
      <div className="mods-body" ref={body} style={width ? { ["--drawer-w" as string]: `${width}px` } : undefined}>
        <section className={sel.length ? "list batched" : "list"}>
          <ModTable tree={searching ? null : tree} rows={rows} byId={byId} order={order} names={names} conflicts={shownConflicts}
            selected={view?.kind === "mod" ? view.id : null} checked={checked} onCheck={check}
            onSelect={(id) => setView({ kind: "mod", id })}
            onToggle={(ids, on) => run("set_enabled", { ids, on })}
            onConflicts={(focus) => setView({ kind: "conflicts", focus })}
            onArrange={arrange}
            onCollapse={(id, collapsed) => run("edit_folder", { id, collapsed })}
            onLock={(id, locked) => run("edit_folder", { id, locked })}
            onMenu={(x, y, target) => setMenu({ x, y, target })}
            updates={modUpdates.found} onUpdate={(id) => setAsk({ kind: "update", id })}
            checking={modUpdates.checking} onCheckUpdates={(ids, what) => checkModUpdates(ids, names, { what })} />
          {rows.length === 0 && searching && <p className="empty">{t("app.mods.noMatch")}</p>}
          {snap.mods.length === 0 && (
            <div className="list-empty">
              <h2 className="cap">{t("app.mods.emptyTitle")}</h2>
              <p className="muted">{t("app.mods.emptyText")}</p>
              <div className="row-btns">
                <button className="btn small" onClick={() => pickAndImport(false)}><ImportIcon />{t("app.mods.importZip")}</button>
                <button className="btn small" onClick={() => pickAndImport(true)}><FolderIcon />{t("app.mods.importFolder")}</button>
                <button className="btn small" onClick={() => run("restore_starter_pack")}>{t("app.settings.starterRestore")}</button>
              </div>
            </div>
          )}
          {sel.length > 0 && (
            <div className="batch-dock">
              <div className="batch" role="toolbar" aria-label={t("app.mods.selection")}>
                <span className="batch-n">{t("app.mods.nChecked", sel.length)}</span>
                <button className="btn small icon" onClick={() => run("set_enabled", { ids: sel, on: true })} aria-label={t("app.menu.enable")} data-tip={t("app.menu.enable")}><SwitchOnIcon /></button>
                <button className="btn small icon" onClick={() => run("set_enabled", { ids: sel, on: false })} aria-label={t("app.menu.disable")} data-tip={t("app.menu.disable")}><SwitchOffIcon /></button>
                <button className="btn small" data-pop onClick={(e) => { const r = e.currentTarget.getBoundingClientRect(); setMenu({ x: r.left, y: r.top - 4, up: true, target: { kind: "mod", id: sel[0] } }); }}>
                  <FolderIcon />{t("app.mods.moreActions")}
                </button>
                <button className="btn small danger" onClick={() => setAsk({ kind: "remove", ids: sel })}><TrashIcon />{t("app.menu.remove")}</button>
                <button className="bare batch-x" onClick={() => setChecked(new Set())} aria-label={t("app.mods.uncheck")} data-tip={t("app.mods.uncheck")}><CloseIcon /></button>
              </div>
            </div>
          )}
          {sorted && sel.length === 0 && (
            <div className="batch-dock">
              <div className="batch sort-done" role="status">
                <span className="batch-n">{sorted.count === 1 ? t("app.sort.doneOne") : t("app.sort.done", sorted.count)}</span>
                <button className="btn small" onClick={() => undoSort(sorted)}>{t("app.sort.undo")}</button>
                <button className="bare batch-x" onClick={() => setSorted(null)} aria-label={t("app.close")} data-tip={t("app.close")}><CloseIcon /></button>
              </div>
            </div>
          )}
        </section>
        {panel && (
          <>
            <div className="split" role="separator" aria-orientation="vertical" aria-label={t("app.mods.resize")} onPointerDown={resize}
              onDoubleClick={() => { setWidth(0); try { localStorage.removeItem(DRAWER_KEY); } catch { /* fine */ } }} data-tip={t("app.mods.resizeHint")} />
            <Drawer view={view} mods={snap.mods} conflicts={shownConflicts} order={order} names={names} onOpen={(id) => setView({ kind: "mod", id })} onClose={() => setView(null)} />
          </>
        )}
      </div>
      {profiles && <ProfilesDialog snap={snap} onClose={() => setProfiles(false)} />}
      {sortPlan && (
        <SortDialog plan={sortPlan} name={(id) => names.get(id) ?? id} off={new Set(snap.mods.filter((m) => !m.on).map((m) => m.id))}
          onApply={() => applySort(sortPlan)} onClose={() => setSortPlan(null)} />
      )}
      {menu && <ContextMenu x={menu.x} y={menu.y} up={menu.up} items={menuItems(menu.target)} onClose={() => setMenu(null)} />}
      {profileMenu && choose && (
        <ContextMenu x={profileMenu.x} y={profileMenu.y} opener={profileMenu.opener} label={t("app.profiles.title")} onClose={() => setProfileMenu(null)} items={profileItems()} />
      )}
      {addMenu && (
        <ContextMenu x={addMenu.x} y={addMenu.y} onClose={() => setAddMenu(null)} items={[
          { label: t("app.mods.importZip"), icon: <ImportIcon />, run: () => pickAndImport(false) },
          { label: t("app.mods.importFolder"), icon: <FolderIcon />, run: () => pickAndImport(true) },
        ]} />
      )}
      {ask?.kind === "newFolder" && (
        <FolderDialog title={t("app.menu.newFolder")} confirm={t("app.folder.create")}
          color={FOLDER_COLORS[snap.folders.length % FOLDER_COLORS.length]} onCancel={() => setAsk(null)}
          onConfirm={(name, color) => { run("create_folder", { name, color, ids: ask.ids }); setAsk(null); setChecked(new Set()); }} />
      )}
      {ask?.kind === "editFolder" && askFolder && (
        <FolderDialog title={t("app.menu.editFolder")} confirm={t("app.save")} name={askFolder.name} color={askFolder.color} onCancel={() => setAsk(null)}
          onConfirm={(name, color) => { run("edit_folder", { id: askFolder.id, name, color }); setAsk(null); }} />
      )}
      {ask?.kind === "deleteFolder" && askFolder && (
        <DeleteFolder name={askFolder.name} color={askFolder.color} count={membersOf(askFolder.id).length} onCancel={() => setAsk(null)}
          onDelete={(removeMods) => { run("delete_folder", { id: askFolder.id, removeMods }); setAsk(null); }} />
      )}
      {ask?.kind === "update" && byId.get(ask.id) && <UpdateConfirm mod={byId.get(ask.id)!} to={offered(modUpdates.found, byId.get(ask.id)!) ?? ""} onDone={() => setAsk(null)} />}
      {ask?.kind === "remove" && (
        <Confirm title={ask.ids.length === 1 ? t("app.drawer.removeTitle", names.get(ask.ids[0]) ?? ask.ids[0]) : t("app.mods.removeNTitle", ask.ids.length)}
          text={t("app.drawer.removeText")} confirm={t("app.drawer.remove")} onCancel={() => setAsk(null)}
          onConfirm={() => { run("remove_mods", { ids: ask.ids }); setAsk(null); if (view?.kind === "mod" && ask.ids.includes(view.id)) setView(null); }} />
      )}
    </div>
  );
}

function UpdateConfirm({ mod, to, onDone }: { mod: ModRow; to: string; onDone: () => void }) {
  const name = pick(mod.name, mod.id);
  const changelog = changelogUrl(mod.source);
  return (
    <Confirm title={t("app.mods.updateTitle", name)} text={t("app.mods.updateText", to, mod.version ?? t("app.mods.imported"))} confirm={t("app.mods.updateConfirm")} tone="go"
      onCancel={onDone} onConfirm={() => { onDone(); updateMod(mod.id, name); }}>
      {changelog && <button className="link link-row" onClick={() => openUrl(changelog).catch(() => {})}>{t("app.mods.updateChangelog")} <ExternalIcon /></button>}
    </Confirm>
  );
}
