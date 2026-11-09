// The Saves tab: one table of the saves of the active Steam account, Official then Sandbox, with the one the
// game boots next marked, and the panel of the selected save beside it, as the Mods tab lays out the mod list
// and drawer. Official saves only switch, snapshot and copy to a Sandbox; everything else is for Sandbox saves.
import { open, save } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useRef, useState, type MouseEvent, type PointerEvent, type ReactNode } from "react";
import { deployLoader } from "../actions";
import { call, errorText, setSnapshot, taskText } from "../api";
import { withTask } from "../busy";
import { Confirm } from "../Confirm";
import { ContextMenu, type MenuItem } from "../ContextMenu";
import { CheckIcon, CopyIcon, DeployIcon, ExportIcon, ImportIcon, PenIcon, PlusIcon, ResetIcon, SaveIcon, SlidersIcon, TrashIcon, UpdateIcon, WarnIcon } from "../Icons";
import { language, t } from "../i18n";
import { notify } from "../notices";
import { Prompt } from "../Prompt";
import { Help } from "../SettingRow";
import type { Snapshot } from "../types";
import { SaveEditor } from "./Editor";
import { fileSafe, playTime } from "./logic";
import type { SaveImported, SaveKind, SaveList, SaveRow, SavesDone } from "./types";

type Ask =
  | { kind: "new"; save: SaveKind }
  | { kind: "snapshot" | "duplicate" | "activate" | "delete"; row: SaveRow }
  | null;

/** One thing a save can have done to it: an icon button in the panel and a line in the right-click menu.
 *  `menuOnly` keeps it out of the panel (Rename, which the editor title also does). */
type Action = { id: string; label: string; icon: ReactNode; tip?: string; disabled: boolean; tone?: "go" | "danger"; menuOnly?: boolean; run: () => void };

const num = (n: number) => n.toLocaleString(language());

const DRAWER_KEY = "obsidian.savesDrawerWidth";
const DRAWER_MIN_PX = 300;
const LIST_MIN_PX = 420;
const savedWidth = () => {
  try {
    return Number(localStorage.getItem(DRAWER_KEY)) || 0;
  } catch {
    return 0;
  }
};

export function SavesPage({ snap, visible }: { snap: Snapshot; visible: boolean }) {
  const [list, setList] = useState<SaveList | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const [sel, setSel] = useState<string | null>(null);
  const [ask, setAsk] = useState<Ask>(null);
  const [menu, setMenu] = useState<{ x: number; y: number; row: SaveRow } | null>(null);
  const [addMenu, setAddMenu] = useState<{ x: number; y: number } | null>(null);
  const [editing, setEditing] = useState<string | null>(null);
  // Rename in the right-click menu opens the editor with the name already in a box.
  const [renameFirst, setRenameFirst] = useState(false);
  const [width, setWidth] = useState(savedWidth);
  const body = useRef<HTMLDivElement>(null);
  const running = snap.game === "running";

  const load = useCallback(async () => {
    try {
      setList(await call<SaveList>("saves_list"));
      setFailed(null);
    } catch (e) {
      setFailed(errorText(e));
    }
  }, []);
  // Again when the tab comes back and when the game stops: the game writes the saves.
  useEffect(() => { if (visible) load(); }, [visible, snap.game, load]);

  const close = useCallback(() => setAsk(null), []);

  const mutate = async (cmd: string, args: Record<string, unknown>, done?: string) => {
    try {
      const r = await withTask(taskText(cmd), () => call<SavesDone>(cmd, args), true);
      setSnapshot(r.snapshot);
      setList(r.saves);
      if (done) notify("info", done);
      return true;
    } catch (e) {
      notify("error", errorText(e));
      return false;
    }
  };

  const activate = async (row: SaveRow) => {
    setAsk(null);
    await mutate("saves_activate", { id: row.id }, t("app.saves.switched", row.name));
  };
  const askActivate = (row: SaveRow) => (row.kind === "official" && !row.live ? setAsk({ kind: "activate", row }) : activate(row));

  const exportSave = async (row: SaveRow) => {
    const dest = await save({ defaultPath: `${fileSafe(row.name)}.zip`, filters: [{ name: t("app.saves.zipFilter"), extensions: ["zip"] }] });
    if (!dest) return;
    try {
      await withTask(taskText("saves_export"), () => call("saves_export", { id: row.id, dest }), true);
      notify("info", t("app.saves.exported", row.name));
    } catch (e) {
      notify("error", errorText(e));
    }
  };
  const importSave = async () => {
    const path = await open({ filters: [{ name: t("app.saves.zipFilter"), extensions: ["zip"] }] });
    if (typeof path !== "string") return;
    try {
      const r = await withTask(taskText("saves_import"), () => call<SaveImported>("saves_import", { path, name: null }), true);
      setSnapshot(r.snapshot);
      setList(r.saves);
      setSel(r.id);
      notify("info", t("app.saves.imported"));
    } catch (e) {
      notify("error", errorText(e));
    }
  };

  // The resize handle, as on the Mods tab: the width is kept between visits, a double-click resets it.
  const resize = (e: PointerEvent<HTMLDivElement>) => {
    const el = e.currentTarget;
    el.setPointerCapture(e.pointerId);
    const box = body.current!.getBoundingClientRect();
    const onMove = (ev: globalThis.PointerEvent) => setWidth(Math.round(Math.max(DRAWER_MIN_PX, Math.min(box.right - ev.clientX, box.width - LIST_MIN_PX))));
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

  if (editing && list?.saves.some((s) => s.id === editing)) {
    return <SaveEditor id={editing} snap={snap} startRename={renameFirst} onClose={() => { setEditing(null); setRenameFirst(false); load(); }} />;
  }

  if (!list) {
    return (
      <div className="saves">
        <div className="head"><h1 className="cap">{t("app.nav.saves")}</h1></div>
        {failed && (
          <div className="list-empty">
            <h2 className="cap">{t("app.saves.cannotOpen")}</h2>
            <p className="muted">{failed}</p>
            <button className="btn small" onClick={load}><UpdateIcon />{t("app.saves.tryAgain")}</button>
          </div>
        )}
      </div>
    );
  }

  const official = list.saves.filter((s) => s.kind === "official");
  const sandbox = list.saves.filter((s) => s.kind === "sandbox");
  const active = list.saves.find((s) => s.active);
  const row = list.saves.find((s) => s.id === sel) ?? active ?? list.saves[0] ?? null;
  // A Sandbox needs the loader in the game, mods or not.
  const needsDeploy = !snap.loader;
  const sandboxActive = active?.kind === "sandbox";
  const closeFirst = running ? t("app.action.closeGameFirst") : undefined;

  /** What can be done to one save, in the order the panel shows the buttons. */
  const actions = (r: SaveRow): Action[] => {
    const isOfficial = r.kind === "official";
    const restore = isOfficial && !!r.snapshotOf;
    const out: Action[] = [];
    if (!r.active) out.push({ id: "switch", label: switchLabel(r), icon: restore ? <ResetIcon /> : <CheckIcon />, tip: closeFirst ?? t(restore ? "app.saves.restoreHint" : "app.saves.switchHint"), tone: "go", disabled: running, run: () => askActivate(r) });
    if (!isOfficial) out.push({ id: "edit", label: t("app.saves.edit"), icon: <SlidersIcon />, tip: closeFirst ?? (r.error ? t("app.saves.unreadable") : t("app.saves.editHint")), disabled: running || !!r.error, run: () => setEditing(r.id) });
    if (isOfficial) out.push({ id: "snapshot", label: t("app.saves.snapshot"), icon: <SaveIcon />, tip: closeFirst ?? t("app.saves.snapshotHint"), disabled: running, run: () => setAsk({ kind: "snapshot", row: r }) });
    out.push({
      id: "duplicate", label: isOfficial ? t("app.saves.duplicateSandbox") : t("app.saves.duplicate"), icon: <CopyIcon />,
      tip: closeFirst ?? (isOfficial ? t("app.saves.duplicateSandboxHint") : t("app.saves.duplicateHint")), disabled: running, run: () => setAsk({ kind: "duplicate", row: r }),
    });
    if (!isOfficial) {
      out.push({ id: "rename", label: t("app.saves.rename"), icon: <PenIcon />, tip: closeFirst, disabled: running || !!r.error, menuOnly: true, run: () => { setRenameFirst(true); setEditing(r.id); } });
      out.push({ id: "export", label: t("app.saves.export"), icon: <ExportIcon />, tip: closeFirst ?? t("app.saves.exportHint"), disabled: running, run: () => exportSave(r) });
      out.push({ id: "delete", label: t("app.saves.delete"), icon: <TrashIcon />, tip: r.active ? t("app.saves.deleteActive") : closeFirst, tone: "danger", disabled: running || r.active, run: () => setAsk({ kind: "delete", row: r }) });
    }
    return out;
  };
  const menuItems = (r: SaveRow): MenuItem[] => {
    const all = actions(r);
    const item = (a: Action): MenuItem => ({ label: a.label, icon: a.icon, disabled: a.disabled, danger: a.tone === "danger", run: a.run });
    const main = all.filter((a) => a.id === "switch" || a.id === "edit").map(item);
    const rest = all.filter((a) => a.id !== "switch" && a.id !== "edit" && a.id !== "delete").map(item);
    const del = all.filter((a) => a.id === "delete").map(item);
    return [...main, ...(main.length && rest.length ? ["sep" as const] : []), ...rest, ...(del.length ? ["sep" as const, ...del] : [])];
  };

  const openMenu = (e: MouseEvent, r: SaveRow) => {
    e.preventDefault();
    setSel(r.id);
    setMenu({ x: e.clientX, y: e.clientY, row: r });
  };

  return (
    <div className="saves">
      <div className="head">
        <h1 className="cap">{t("app.nav.saves")}</h1>
        <Help page="saves" tip={t("app.saves.helpTip")} />
        <button className="btn go head-tool" data-pop aria-haspopup="menu" aria-expanded={!!addMenu} disabled={running} aria-label={t("app.saves.new")}
          data-tip-title={t("app.saves.new")} data-tip={closeFirst ?? t("app.saves.newHint")}
          onClick={(e) => { const r = e.currentTarget.getBoundingClientRect(); setAddMenu({ x: r.left, y: r.bottom + 4 }); }}>
          <PlusIcon /><span className="label">{t("app.saves.new")}</span>
        </button>
        <button className={sandboxActive && needsDeploy ? "btn icon go head-tool head-deploy" : "btn icon head-tool head-deploy"} disabled={running || !sandboxActive || !snap.gameDir} onClick={() => deployLoader()}
          aria-label={t("app.saves.deployLoader")} data-tip-title={t("app.saves.deployLoader")}
          data-tip={closeFirst ?? (sandboxActive ? t("app.saves.deployLoaderHint") : t("app.saves.deployLoaderOfficial"))}><DeployIcon /></button>
        <span className="muted saves-account">{t("app.saves.account", list.account)}</span>
      </div>

      {running && <div className="banner"><WarnIcon /><span>{t("app.saves.gameRunning")}</span></div>}
      {!running && sandboxActive && needsDeploy && <div className="banner"><WarnIcon /><span>{t("app.saves.needsDeploy")}</span></div>}

      <div className="saves-body" ref={body} style={width ? { ["--drawer-w" as string]: `${width}px` } : undefined}>
        <section className="list saves-list" aria-label={t("app.nav.saves")}>
          <div className="table saves-table" role="table">
            <div className="sth cap" role="row">
              <span aria-hidden="true" />
              <span className="th-col left" role="columnheader">{t("app.saves.col.save")}</span>
              <span className="th-col" role="columnheader">{t("app.saves.stat.rank")}</span>
              <span className="th-col s-wide" role="columnheader">{t("app.saves.stat.samples")}</span>
              <span className="th-col s-wide" role="columnheader">{t("app.saves.col.played")}</span>
            </div>
            <div className="tbody">
              <GroupRow title={t("app.saves.official")} hint={t("app.saves.officialHint")} count={official.length} kind="official" />
              {official.map((r) => <Row key={r.id} row={r} list={list} selected={r === row} onSelect={setSel} onMenu={openMenu} onOpen={() => r.kind === "sandbox" && !running && !r.error && setEditing(r.id)} />)}
              <GroupRow title={t("app.saves.sandbox")} hint={t("app.saves.sandboxHint")} count={sandbox.length} kind="sandbox" />
              {sandbox.length === 0 && <p className="muted saves-none">{t("app.saves.noSandbox")}</p>}
              {sandbox.map((r) => <Row key={r.id} row={r} list={list} selected={r === row} onSelect={setSel} onMenu={openMenu} onOpen={() => !running && !r.error && setEditing(r.id)} />)}
            </div>
          </div>
        </section>

        {row && (
          <>
            <div className="split" role="separator" aria-orientation="vertical" aria-label={t("app.mods.resize")} onPointerDown={resize}
              onDoubleClick={() => { setWidth(0); try { localStorage.removeItem(DRAWER_KEY); } catch { /* fine */ } }} data-tip={t("app.mods.resizeHint")} />
            <Detail row={row} list={list} actions={actions(row)} running={running} needsDeploy={needsDeploy} />
          </>
        )}
      </div>

      {menu && <ContextMenu x={menu.x} y={menu.y} items={menuItems(menu.row)} onClose={() => setMenu(null)} />}
      {addMenu && (
        <ContextMenu x={addMenu.x} y={addMenu.y} onClose={() => setAddMenu(null)} items={[
          { label: t("app.saves.newSandbox"), icon: <PlusIcon />, run: () => setAsk({ kind: "new", save: "sandbox" }) },
          { label: t("app.saves.newOfficial"), icon: <PlusIcon />, run: () => setAsk({ kind: "new", save: "official" }) },
          "sep",
          { label: t("app.saves.importZip"), icon: <ImportIcon />, run: importSave },
        ]} />
      )}

      {ask?.kind === "new" && (
        <Prompt title={ask.save === "official" ? t("app.saves.newOfficial") : t("app.saves.newSandbox")} label={t("app.saves.name")} confirm={t("app.saves.create")} onCancel={close}
          onConfirm={async (name) => { const ok = await mutate(ask.save === "official" ? "saves_create_official" : "saves_create_sandbox", { name }); if (ok) close(); return ok; }} />
      )}
      {ask?.kind === "snapshot" && (
        <Prompt title={t("app.saves.snapshotTitle", ask.row.name)} label={t("app.saves.name")} initial={t("app.saves.snapshotName", ask.row.name)} confirm={t("app.saves.snapshot")} onCancel={close}
          onConfirm={async (name) => { const ok = await mutate("saves_snapshot", { id: ask.row.id, name }); if (ok) close(); return ok; }} />
      )}
      {ask?.kind === "duplicate" && (
        <Prompt title={ask.row.kind === "official" ? t("app.saves.duplicateSandboxTitle", ask.row.name) : t("app.saves.duplicateTitle", ask.row.name)} label={t("app.saves.name")}
          initial={t("app.saves.copyName", ask.row.name)} confirm={ask.row.kind === "official" ? t("app.saves.duplicateSandbox") : t("app.saves.duplicate")} onCancel={close}
          onConfirm={async (name) => { const ok = await mutate("saves_duplicate", { id: ask.row.id, name }); if (ok) close(); return ok; }} />
      )}
      {ask?.kind === "activate" && (
        <Confirm title={t("app.saves.switchTitle", ask.row.name)} text={t("app.saves.switchText")} confirm={switchLabel(ask.row)} tone="go" onCancel={close} onConfirm={() => activate(ask.row)} />
      )}
      {ask?.kind === "delete" && (
        <Confirm title={t("app.saves.deleteTitle", ask.row.name)} text={t("app.saves.deleteText")} confirm={t("app.saves.delete")} onCancel={close}
          onConfirm={() => { const id = ask.row.id; setAsk(null); mutate("saves_delete", { id }, t("app.saves.deleted")); }} />
      )}
    </div>
  );
}

const switchLabel = (r: SaveRow) => (r.kind === "official" && r.snapshotOf ? t("app.saves.restore") : t("app.saves.switch"));

/** The label over each group: the name and the count, with what the group is in the tooltip. */
function GroupRow({ title, hint, count, kind }: { title: string; hint: string; count: number; kind: SaveKind }) {
  return (
    <div className={`str sgroup ${kind}`} role="row">
      <span className="sgroup-name cap" role="rowheader" data-tip-title={title} data-tip={hint}>{title}<span className="sgroup-n">{count}</span></span>
    </div>
  );
}

type RowProps = { row: SaveRow; list: SaveList; selected: boolean; onSelect: (id: string) => void; onMenu: (e: MouseEvent, r: SaveRow) => void; onOpen: () => void };

function Row({ row, list, selected, onSelect, onMenu, onOpen }: RowProps) {
  const of = row.snapshotOf ? list.saves.find((s) => s.id === row.snapshotOf)?.name ?? null : null;
  const s = row.stats;
  return (
    <div className={["str", selected && "sel", row.active && "active"].filter(Boolean).join(" ")} role="row" tabIndex={0} aria-selected={selected}
      onClick={() => onSelect(row.id)} onDoubleClick={onOpen} onContextMenu={(e) => onMenu(e, row)}
      onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onSelect(row.id); } }}>
      <span className="s-icon" role="cell"><SaveIcon /></span>
      <div className="who" role="cell">
        <div className="nm">
          <span className="nm-text">{row.name}</span>
          {row.active && <span className="tag" data-tip={t("app.saves.bootsNext")}>{t("app.saves.active")}</span>}
          {row.snapshotOf && <span className="save-chip" data-tip={of ? t("app.saves.snapshotOf", of) : undefined}>{t("app.saves.snapshotTag")}</span>}
        </div>
        {row.error && <div className="sub warn">{t("app.saves.unreadable")}</div>}
      </div>
      <span className="s-val" role="cell">{s ? s.rank : ""}</span>
      <span className="s-val s-wide" role="cell">{s ? num(s.samples) : ""}</span>
      <span className="s-val s-wide" role="cell">{s ? playTime(s.timePlayed) : ""}</span>
    </div>
  );
}

function Detail({ row, list, actions, running, needsDeploy }: { row: SaveRow; list: SaveList; actions: Action[]; running: boolean; needsDeploy: boolean }) {
  const s = row.stats;
  const isOfficial = row.kind === "official";
  const created = new Date(row.created);
  const when = Number.isNaN(created.getTime()) ? row.created : created.toLocaleDateString(language(), { dateStyle: "medium" });
  const played = row.lastPlayed ? new Date(row.lastPlayed) : null;
  const playedText = played && !Number.isNaN(played.getTime()) ? played.toLocaleString(language(), { dateStyle: "medium", timeStyle: "short" }) : null;
  const of = row.snapshotOf ? list.saves.find((o) => o.id === row.snapshotOf)?.name ?? row.snapshotOf : null;
  // Only what the table and the tag do not already say.
  const state = row.active ? null : row.live ? t("app.saves.isLive") : isOfficial ? t("app.saves.inLibrary") : null;
  return (
    <aside className={`drawer saves-detail ${row.kind}`} key={row.id} aria-label={row.name}>
      <div className="dh">
        <div className="dh-text">
          <h2>{row.name}{row.active && <span className="tag" data-tip={t("app.saves.bootsNext")}>{t("app.saves.active")}</span>}</h2>
          <div className="meta">{[isOfficial ? t("app.saves.official") : t("app.saves.sandbox"), t("app.saves.created", when), of && t("app.saves.snapshotOf", of)].filter(Boolean).join(" · ")}</div>
        </div>
      </div>
      <div className="save-tools" role="toolbar" aria-label={row.name}>
        {actions.filter((a) => !a.menuOnly).map((a) => (
          <button key={a.id} className={["btn small icon head-tool", a.tone === "go" && "go", a.tone === "danger" && "danger"].filter(Boolean).join(" ")}
            disabled={a.disabled} onClick={a.run} aria-label={a.label} data-tip-title={a.label} data-tip={a.tip}>{a.icon}</button>
        ))}
      </div>
      <div className="dbody saves-dbody">
        {row.error && <p className="save-note warn"><b>{t("app.saves.unreadable")}</b> {row.error}</p>}
        {s && (
          <dl className="kv">
            <div><dt>{t("app.saves.stat.rank")}</dt><dd>{s.rank}</dd></div>
            <div><dt>{t("app.saves.stat.xp")}</dt><dd>{num(s.xp)}</dd></div>
            <div><dt>{t("app.saves.stat.samples")}</dt><dd>{num(s.samples)}</dd></div>
            <div><dt>{t("app.saves.stat.unlocks")}</dt><dd>{num(s.unlocks)}</dd></div>
            <div><dt>{t("app.saves.stat.missions")}</dt><dd>{t("app.saves.missionsValue", num(s.missionsWon), num(s.missionsFought))}</dd></div>
            <div><dt>{t("app.saves.stat.kills")}</dt><dd>{num(s.kills)}</dd></div>
            <div><dt>{t("app.saves.stat.deaths")}</dt><dd>{num(s.deaths)}</dd></div>
            <div><dt>{t("app.saves.stat.played")}</dt><dd>{playTime(s.timePlayed)}</dd></div>
            {playedText && <div><dt>{t("app.saves.stat.lastPlayed")}</dt><dd>{playedText}</dd></div>}
          </dl>
        )}
        {state && <p className="save-note">{state}</p>}
        {!isOfficial && !row.active && needsDeploy && !running && <p className="save-note warn">{t("app.saves.sandboxNeedsLoader")}</p>}
      </div>
    </aside>
  );
}
