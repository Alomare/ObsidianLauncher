// The Saves tab: the saves of the active Steam account in two groups, Official and Sandbox, with the one the
// game boots next marked, and a panel for the selected save with what it holds and what can be done to it.
// Official saves only switch, snapshot and copy to a Sandbox; everything else is for Sandbox saves.
import { open, save } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useState, type MouseEvent } from "react";
import { deploy } from "../actions";
import { call, errorText, setSnapshot, taskText } from "../api";
import { withTask } from "../busy";
import { Confirm } from "../Confirm";
import { ContextMenu, type MenuItem } from "../ContextMenu";
import { CheckIcon, CopyIcon, DeployIcon, ExportIcon, ImportIcon, PenIcon, PlusIcon, SaveIcon, TrashIcon, UpdateIcon, WarnIcon } from "../Icons";
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
  | { kind: "snapshot" | "duplicate" | "rename" | "activate" | "delete"; row: SaveRow }
  | null;

const num = (n: number) => n.toLocaleString(language());

export function SavesPage({ snap, visible }: { snap: Snapshot; visible: boolean }) {
  const [list, setList] = useState<SaveList | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const [sel, setSel] = useState<string | null>(null);
  const [ask, setAsk] = useState<Ask>(null);
  const [menu, setMenu] = useState<{ x: number; y: number; row: SaveRow } | null>(null);
  const [editing, setEditing] = useState<string | null>(null);
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

  if (editing && list?.saves.some((s) => s.id === editing)) {
    return <SaveEditor id={editing} snap={snap} onClose={() => { setEditing(null); load(); }} />;
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
  const needsDeploy = !snap.deployed;

  /** The actions of one save, for the panel and the right-click menu. */
  const actions = (r: SaveRow): MenuItem[] => {
    const off = running;
    const isOfficial = r.kind === "official";
    return [
      { label: switchLabel(r), icon: <CheckIcon />, disabled: off || r.active, run: () => askActivate(r) },
      ...(!isOfficial ? [{ label: t("app.saves.edit"), icon: <PenIcon />, disabled: off || !!r.error, run: () => setEditing(r.id) } as MenuItem] : []),
      "sep",
      ...(isOfficial ? [{ label: t("app.saves.snapshot"), icon: <CopyIcon />, disabled: off, run: () => setAsk({ kind: "snapshot", row: r }) } as MenuItem] : []),
      { label: isOfficial ? t("app.saves.duplicateSandbox") : t("app.saves.duplicate"), icon: <CopyIcon />, disabled: off, run: () => setAsk({ kind: "duplicate", row: r }) },
      ...(!isOfficial ? [
        { label: t("app.saves.rename"), icon: <PenIcon />, disabled: off, run: () => setAsk({ kind: "rename", row: r }) } as MenuItem,
        { label: t("app.saves.export"), icon: <ExportIcon />, disabled: off, run: () => exportSave(r) } as MenuItem,
        "sep" as const,
        { label: t("app.saves.delete"), icon: <TrashIcon />, danger: true, disabled: off || r.active, run: () => setAsk({ kind: "delete", row: r }) } as MenuItem,
      ] : []),
    ];
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
        <span className="muted saves-account">{t("app.saves.account", list.account)}</span>
      </div>

      {running && <div className="banner"><WarnIcon /><span>{t("app.saves.gameRunning")}</span></div>}
      {!running && active?.kind === "sandbox" && needsDeploy && (
        <div className="banner">
          <WarnIcon />
          <span>{t("app.saves.needsDeploy")}</span>
          <button className="btn small go" onClick={() => deploy()}><DeployIcon />{t("app.action.deploy")}</button>
        </div>
      )}

      <div className="saves-body">
        <section className="list saves-list" aria-label={t("app.nav.saves")}>
          <SaveGroup title={t("app.saves.official")} hint={t("app.saves.officialHint")}
            tools={<button className="btn small" disabled={running} onClick={() => setAsk({ kind: "new", save: "official" })} data-tip={running ? t("app.action.closeGameFirst") : t("app.saves.newOfficialHint")}><PlusIcon />{t("app.saves.new")}</button>}>
            {official.map((r) => <Row key={r.id} row={r} list={list} selected={r === row} onSelect={setSel} onMenu={openMenu} />)}
          </SaveGroup>
          <SaveGroup title={t("app.saves.sandbox")} hint={t("app.saves.sandboxHint")}
            tools={<>
              <button className="btn small" disabled={running} onClick={importSave} data-tip={running ? t("app.action.closeGameFirst") : t("app.saves.importHint")}><ImportIcon />{t("app.saves.import")}</button>
              <button className="btn small" disabled={running} onClick={() => setAsk({ kind: "new", save: "sandbox" })} data-tip={running ? t("app.action.closeGameFirst") : t("app.saves.newSandboxHint")}><PlusIcon />{t("app.saves.new")}</button>
            </>}>
            {sandbox.length === 0 && <p className="muted saves-none">{t("app.saves.noSandbox")}</p>}
            {sandbox.map((r) => <Row key={r.id} row={r} list={list} selected={r === row} onSelect={setSel} onMenu={openMenu} />)}
          </SaveGroup>
        </section>

        {row && <Detail row={row} list={list} running={running} needsDeploy={needsDeploy}
          onSwitch={() => askActivate(row)} onEdit={() => setEditing(row.id)} onSnapshot={() => setAsk({ kind: "snapshot", row })}
          onDuplicate={() => setAsk({ kind: "duplicate", row })} onRename={() => setAsk({ kind: "rename", row })}
          onExport={() => exportSave(row)} onDelete={() => setAsk({ kind: "delete", row })} />}
      </div>

      {menu && <ContextMenu x={menu.x} y={menu.y} items={actions(menu.row)} onClose={() => setMenu(null)} />}

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
      {ask?.kind === "rename" && (
        <Prompt title={t("app.saves.renameTitle", ask.row.name)} label={t("app.saves.name")} initial={ask.row.name} confirm={t("app.saves.rename")} onCancel={close}
          onConfirm={async (name) => { const ok = name === ask.row.name || (await mutate("saves_rename", { id: ask.row.id, name })); if (ok) close(); return ok; }} />
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

function SaveGroup({ title, hint, tools, children }: { title: string; hint: string; tools: React.ReactNode; children: React.ReactNode }) {
  return (
    <div className="save-group">
      <div className="save-group-head">
        <div className="save-group-text">
          <h2 className="cap">{title}</h2>
          <small>{hint}</small>
        </div>
        <div className="save-group-tools">{tools}</div>
      </div>
      {children}
    </div>
  );
}

function statLine(r: SaveRow): string {
  if (!r.stats) return r.error ? t("app.saves.unreadable") : "";
  return [t("app.saves.rankN", r.stats.rank), t("app.saves.samplesN", num(r.stats.samples)), playTime(r.stats.timePlayed)].join(" · ");
}

function Row({ row, list, selected, onSelect, onMenu }: { row: SaveRow; list: SaveList; selected: boolean; onSelect: (id: string) => void; onMenu: (e: MouseEvent, r: SaveRow) => void }) {
  const of = row.snapshotOf ? list.saves.find((s) => s.id === row.snapshotOf)?.name : null;
  return (
    <div className={["save-row", selected && "sel", row.active && "active"].filter(Boolean).join(" ")} role="button" tabIndex={0} aria-pressed={selected}
      onClick={() => onSelect(row.id)} onContextMenu={(e) => onMenu(e, row)}
      onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onSelect(row.id); } }}>
      <SaveIcon />
      <div className="save-row-text">
        <div className="nm">
          <span className="nm-text">{row.name}</span>
          {row.active && <span className="tag">{t("app.saves.active")}</span>}
          {row.snapshotOf && <span className="save-chip">{t("app.saves.snapshotTag")}</span>}
        </div>
        <div className={row.error ? "sub warn" : "sub"}>{row.error ? t("app.saves.unreadable") : of ? `${t("app.saves.snapshotOf", of)} · ${statLine(row)}` : statLine(row)}</div>
      </div>
    </div>
  );
}

type DetailProps = {
  row: SaveRow; list: SaveList; running: boolean; needsDeploy: boolean;
  onSwitch: () => void; onEdit: () => void; onSnapshot: () => void; onDuplicate: () => void; onRename: () => void; onExport: () => void; onDelete: () => void;
};

function Detail({ row, list, running, needsDeploy, onSwitch, onEdit, onSnapshot, onDuplicate, onRename, onExport, onDelete }: DetailProps) {
  const s = row.stats;
  const isOfficial = row.kind === "official";
  const created = new Date(row.created);
  const when = Number.isNaN(created.getTime()) ? row.created : created.toLocaleDateString(language(), { dateStyle: "medium" });
  const of = row.snapshotOf ? list.saves.find((o) => o.id === row.snapshotOf)?.name ?? row.snapshotOf : null;
  const closeFirst = running ? t("app.action.closeGameFirst") : undefined;
  return (
    <aside className={`drawer saves-detail ${row.kind}`} key={row.id} aria-label={row.name}>
      <div className="dh">
        <div className="dh-text">
          <h2>{row.name}</h2>
          <div className="meta">{[isOfficial ? t("app.saves.official") : t("app.saves.sandbox"), t("app.saves.created", when)].join(" · ")}</div>
          {of && <div className="meta">{t("app.saves.snapshotOf", of)}</div>}
        </div>
        {row.active && <span className="tag">{t("app.saves.active")}</span>}
      </div>
      <div className="dbody saves-dbody">
        <p className={row.active ? "save-state on" : "save-state"}>{row.active ? t("app.saves.bootsNext") : row.live ? t("app.saves.isLive") : isOfficial ? t("app.saves.inLibrary") : t("app.saves.notSelected")}</p>
        {row.error && <p className="save-state warn"><b>{t("app.saves.unreadable")}</b> {row.error}</p>}
        {s && (
          <dl className="save-stats">
            <div><dt>{t("app.saves.stat.rank")}</dt><dd>{s.rank}</dd></div>
            <div><dt>{t("app.saves.stat.xp")}</dt><dd>{num(s.xp)}</dd></div>
            <div><dt>{t("app.saves.stat.samples")}</dt><dd>{num(s.samples)}</dd></div>
            <div><dt>{t("app.saves.stat.missions")}</dt><dd>{t("app.saves.missionsValue", num(s.missionsWon), num(s.missionsFought))}</dd></div>
            <div><dt>{t("app.saves.stat.kills")}</dt><dd>{num(s.kills)}</dd></div>
            <div><dt>{t("app.saves.stat.deaths")}</dt><dd>{num(s.deaths)}</dd></div>
            <div><dt>{t("app.saves.stat.played")}</dt><dd>{playTime(s.timePlayed)}</dd></div>
            <div><dt>{t("app.saves.stat.unlocks")}</dt><dd>{num(s.unlocks)}</dd></div>
          </dl>
        )}
        <div className="save-actions">
          {!row.active && <button className="btn small play" disabled={running} onClick={onSwitch} data-tip={closeFirst}><CheckIcon />{switchLabel(row)}</button>}
          {isOfficial ? (
            <>
              <button className="btn small" disabled={running} onClick={onSnapshot} data-tip={closeFirst ?? t("app.saves.snapshotHint")}><CopyIcon />{t("app.saves.snapshot")}</button>
              <button className="btn small" disabled={running} onClick={onDuplicate} data-tip={closeFirst ?? t("app.saves.duplicateSandboxHint")}><CopyIcon />{t("app.saves.duplicateSandbox")}</button>
            </>
          ) : (
            <>
              <button className="btn small go" disabled={running || !!row.error} onClick={onEdit} data-tip={closeFirst ?? t("app.saves.editHint")}><PenIcon />{t("app.saves.edit")}</button>
              <button className="btn small" disabled={running} onClick={onDuplicate} data-tip={closeFirst}><CopyIcon />{t("app.saves.duplicate")}</button>
              <button className="btn small" disabled={running} onClick={onRename} data-tip={closeFirst}><PenIcon />{t("app.saves.rename")}</button>
              <button className="btn small" disabled={running} onClick={onExport} data-tip={closeFirst ?? t("app.saves.exportHint")}><ExportIcon />{t("app.saves.export")}</button>
              <button className="btn small danger" disabled={running || row.active} onClick={onDelete} data-tip={row.active ? t("app.saves.deleteActive") : closeFirst}><TrashIcon />{t("app.saves.delete")}</button>
            </>
          )}
        </div>
        {!isOfficial && !row.active && needsDeploy && <p className="save-why">{t("app.saves.sandboxNeedsLoader")}</p>}
        {!isOfficial && row.active && <p className="save-why muted">{t("app.saves.deleteActive")}</p>}
        {running && <p className="save-why">{t("app.saves.gameRunning")}</p>}
      </div>
    </aside>
  );
}
