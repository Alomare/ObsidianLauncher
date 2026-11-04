// Settings (spec 4.4): System (game, other patches, language, updates, Obsidian Kit, data), Updates
// and Theme (colours, size, animations). One column of groups under headings, each with a "?" to the
// Wiki, as in Audiodiver. App settings save to app.json at once. The subtab scrolls; the header stays.
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { call, errorText, run, setSnapshot, taskText } from "./api";
import { Confirm } from "./Confirm";
import { withTask } from "./busy";
import { FolderIcon } from "./Icons";
import { LANGUAGES, language, t } from "./i18n";
import { notify } from "./notices";
import { copyReport } from "./report";
import { Select } from "./Select";
import { Field, Group } from "./SettingRow";
import { updateSettings, useAppSettings } from "./settings";
import { motionOn, systemReducesMotion } from "./theme";
import type { AppSettings, Snapshot } from "./types";
import { UpdatesTab } from "./UpdatesTab";

type OtherPatch = { name: string; bundle: string; size: number; modified: number | null };
type SystemInfo = {
  dataDir: string; backups: { name: string; path: string }[]; chosenGameDir: string | null; logsDir: string | null;
  foreign: OtherPatch[]; removedPatches: string | null;
};
type OtherPatchesRemoved = { snapshot: Snapshot; removed: number; keptIn: string | null };

export type SettingsTab = "system" | "updates" | "theme";
const TABS: SettingsTab[] = ["system", "updates", "theme"];

export function SettingsPage({ snap, tab, onTab }: { snap: Snapshot; tab: SettingsTab; onTab: (t: SettingsTab) => void }) {
  return (
    <div className="settings">
      <div className="head">
        <h1 className="cap">{t("app.nav.settings")}</h1>
        <div className="subtabs" role="tablist">
          {TABS.map((id) => (
            <button key={id} role="tab" aria-selected={tab === id} className={tab === id ? "cap cur" : "cap"} data-tip={t(`app.settings.${id}Tip`)} onClick={() => onTab(id)}>
              {t(`app.settings.${id}`)}
            </button>
          ))}
        </div>
      </div>
      <div className="view settings-body" key={tab}>
        {tab === "system" ? <SystemTab snap={snap} /> : tab === "updates" ? <UpdatesTab snap={snap} onSystem={() => onTab("system")} /> : <ThemeTab />}
      </div>
    </div>
  );
}

function Segmented<T extends string | number>({ value, options, onChange, label }: { value: T; options: [T, string][]; onChange: (v: T) => void; label: string }) {
  return (
    <div className="seg" role="radiogroup" aria-label={label}>
      {options.map(([v, text]) => (
        <button key={String(v)} role="radio" aria-checked={v === value} className={v === value ? "on" : undefined} onClick={() => v !== value && onChange(v)}>
          {text}
        </button>
      ))}
    </div>
  );
}

function stamp(name: string) {
  const m = /^(\d{4})(\d{2})(\d{2})-(\d{2})(\d{2})(\d{2})/.exec(name);
  if (!m) return name;
  const d = new Date(+m[1], +m[2] - 1, +m[3], +m[4], +m[5], +m[6]);
  return d.toLocaleString(language(), { dateStyle: "medium", timeStyle: "short" });
}

export function bytes(n: number) {
  const units = ["B", "KB", "MB", "GB"];
  let i = 0;
  while (n >= 1024 && i < units.length - 1) { n /= 1024; i++; }
  return `${n.toLocaleString(language(), { maximumFractionDigits: i ? 1 : 0 })} ${units[i]}`;
}

const GAME_TONE: Record<Snapshot["game"], string> = { supported: "ok", running: "busy", unknown: "bad", notFound: "bad" };

function SystemTab({ snap }: { snap: Snapshot }) {
  const settings = useAppSettings()!;
  const [info, setInfo] = useState<SystemInfo | null>(null);
  const [checking, setChecking] = useState(false);
  const [restoring, setRestoring] = useState(false);
  const [removing, setRemoving] = useState(false);
  useEffect(() => {
    let live = true;
    call<SystemInfo>("system_info").then((i) => live && setInfo(i)).catch((e) => notify("error", errorText(e)));
    return () => { live = false; };
  }, [snap]);
  const openFolder = (path: string) => call("open_folder", { path }).catch((e) => notify("error", errorText(e)));
  const change = async () => {
    const picked = await open({ directory: true, defaultPath: snap.gameDir ?? undefined, title: t("app.settings.pickGame") });
    if (typeof picked === "string") run("set_game_dir", { path: picked });
  };
  const check = async () => {
    setChecking(true);
    await run("check_game");
    setChecking(false);
  };
  const restore = async () => {
    setRestoring(false);
    if (await run("restore_starter_pack")) notify("info", t("app.settings.starterDone"));
  };
  const removeOthers = async () => {
    setRemoving(false);
    try {
      const r = await withTask(taskText("remove_other_patches"), () => call<OtherPatchesRemoved>("remove_other_patches"));
      setSnapshot(r.snapshot);
      notify("info", r.removed === 1 ? t("app.settings.otherRemovedOne") : t("app.settings.otherRemoved", r.removed));
    } catch (e) {
      notify("error", errorText(e));
    }
  };
  const running = snap.game === "running";
  const others = info?.foreign ?? [];

  return (
    <div className="set-page">
      <Group title={t("app.settings.game")} help="deploy-and-purge#the-build-check" tip={t("app.settings.gameTip")}>
        <div className={`game-card ${GAME_TONE[snap.game]}`}>
          <span className="game-dot" aria-hidden="true" />
          <div className="game-text">
            <b>{t(`app.settings.state.${snap.game}`)}</b>
            <span className="mono">{snap.gameDir ?? t("app.game.notFound")}</span>
            <small>{t(`app.settings.build.${snap.game}`)}{snap.gameDir ? ` ${info?.chosenGameDir ? t("app.settings.chosenByYou") : t("app.settings.foundSteam")}` : ""}</small>
          </div>
        </div>
        <div className="set-btns">
          {snap.gameDir && <button className="btn small" onClick={() => openFolder(snap.gameDir!)}><FolderIcon />{t("app.settings.openFolder")}</button>}
          <button className="btn small" onClick={change}>{t("app.settings.changeFolder")}</button>
          {info?.chosenGameDir && <button className="btn small" onClick={() => run("set_game_dir", { path: null })} data-tip={t("app.settings.useSteamHint")}>{t("app.settings.useSteam")}</button>}
          <button className="btn small" onClick={check} disabled={checking || !snap.gameDir}>{t("app.settings.checkAgain")}</button>
        </div>

        {snap.gameDir && (
          <div className="other-patches">
            <h3>
              {t("app.settings.otherPatches")}
              {others.length > 0 && <span className="count">{others.length}</span>}
            </h3>
            {info && others.length === 0 && <p className="set-note">{t("app.settings.otherNone")}</p>}
            {others.length > 0 && (
              <>
                <p className="set-note">{t("app.settings.otherHint")}</p>
                <div className="patch-table" role="table" aria-label={t("app.settings.otherPatches")}>
                  <div className="pt-row pt-head" role="row">
                    <span role="columnheader">{t("app.settings.otherFile")}</span>
                    <span role="columnheader">{t("app.settings.otherSize")}</span>
                    <span role="columnheader">{t("app.settings.otherDate")}</span>
                  </div>
                  {others.map((p) => (
                    <div key={p.name} className="pt-row" role="row">
                      <span className="mono" role="cell">{p.name}</span>
                      <span role="cell">{bytes(p.size)}</span>
                      <span role="cell">{p.modified ? new Date(p.modified * 1000).toLocaleDateString(language(), { dateStyle: "medium" }) : ""}</span>
                    </div>
                  ))}
                </div>
              </>
            )}
            <div className="set-btns">
              {others.length > 0 && (
                <button className="btn small danger" onClick={() => setRemoving(true)} disabled={running} data-tip={running ? t("app.action.closeGameFirst") : t("app.settings.otherRemoveHint")}>
                  {t("app.settings.otherRemove")}
                </button>
              )}
              {info?.removedPatches && <button className="btn small" onClick={() => openFolder(info.removedPatches!)}><FolderIcon />{t("app.settings.otherKept")}</button>}
            </div>
          </div>
        )}
      </Group>

      <Group title={t("app.settings.language")}>
        <Field label={t("app.settings.languageLabel")}>
          <Select value={settings.language ?? ""} label={t("app.settings.language")}
            options={[{ value: "", label: t("app.settings.systemLanguage") }, ...LANGUAGES.map(([value, label]) => ({ value, label }))]}
            onChange={(v) => updateSettings({ language: v || null })} />
        </Field>
      </Group>

      {!snap.portable && (
        <Group title={t("app.settings.updates")} help="how-obsidian-works#updating-obsidian">
          <Field label={t("app.settings.updateMode")} note={t(`app.settings.updates.${settings.updates}Hint`)}>
            <Segmented<AppSettings["updates"]> value={settings.updates} label={t("app.settings.updateMode")}
              options={(["manual", "prompt", "auto"] as const).map((v) => [v, t(`app.settings.updates.${v}`)])}
              onChange={(v) => updateSettings({ updates: v })} />
          </Field>
        </Group>
      )}

      <Group title={t("app.settings.starter")} help="adding-mods#obsidian-kit">
        <Field label={t("app.settings.starterLabel")} note={t("app.settings.starterHint")}>
          <button className="btn small" onClick={() => setRestoring(true)}>{t("app.settings.starterRestore")}</button>
        </Field>
      </Group>

      <Group title={t("app.settings.data")} help="how-obsidian-works#where-your-files-are">
        <Field label={t("app.settings.dataFolder")} note={snap.portable ? t("app.settings.portable") : t("app.settings.dataHint")}>
          <div className="path-line">
            <span className="mono">{info?.dataDir ?? ""}</span>
            {info && <button className="btn small" onClick={() => openFolder(info.dataDir)}><FolderIcon />{t("app.settings.open")}</button>}
          </div>
        </Field>
        <Field label={t("app.settings.backups")} note={t("app.settings.backupsHint")}>
          {info && info.backups.length === 0 && <p className="set-note">{t("app.settings.noBackups")}</p>}
          {info && info.backups.length > 0 && (
            <ul className="backups">
              {info.backups.map((b) => (
                <li key={b.name}>
                  <span>{stamp(b.name)}</span>
                  <button className="link small" onClick={() => openFolder(b.path)}>{t("app.settings.open")}</button>
                </li>
              ))}
            </ul>
          )}
        </Field>
      </Group>

      <Group title={t("app.settings.report")} help="troubleshooting#asking-for-help">
        <Field note={t("app.settings.reportHint")}>
          <div className="set-btns">
            <button className="btn small" onClick={copyReport}>{t("app.crash.copyReport")}</button>
            {info?.logsDir && <button className="btn small" onClick={() => openFolder(info.logsDir!)}><FolderIcon />{t("app.settings.logs")}</button>}
          </div>
        </Field>
      </Group>

      {restoring && (
        <Confirm title={t("app.settings.starterRestore")} text={t("app.settings.starterText", snap.profile)} confirm={t("app.settings.starterRestore")}
          onCancel={() => setRestoring(false)} onConfirm={restore} />
      )}
      {removing && (
        <Confirm title={t("app.settings.otherRemove")} text={t("app.settings.otherRemoveText", others.length)} confirm={t("app.settings.otherRemove")}
          onCancel={() => setRemoving(false)} onConfirm={removeOthers} />
      )}
    </div>
  );
}

function ThemeTab() {
  const s = useAppSettings()!;
  const motion = motionOn(s.motion, systemReducesMotion());
  return (
    <div className="set-page">
      <Group title={t("app.settings.appearance")}>
        <Field label={t("app.settings.colours")}>
          <Segmented label={t("app.settings.colours")} value={s.theme} onChange={(theme) => updateSettings({ theme })}
            options={[["default", t("app.settings.theme.default")], ["light", t("app.settings.theme.light")]]} />
        </Field>
        <Field label={t("app.settings.scale")} note={t("app.settings.scaleHint")}>
          <Segmented label={t("app.settings.scale")} value={s.scale} onChange={(scale) => updateSettings({ scale })}
            options={([90, 100, 110, 125] as const).map((v) => [v, `${v} %`])} />
        </Field>
        <Field label={t("app.settings.textSize")} note={t("app.settings.textSizeHint")}>
          <Segmented label={t("app.settings.textSize")} value={s.text} onChange={(text) => updateSettings({ text })}
            options={[["small", t("app.settings.text.small")], ["normal", t("app.settings.text.normal")], ["large", t("app.settings.text.large")]]} />
        </Field>
      </Group>
      <Group title={t("app.settings.motion")}>
        <label className="switch-line">
          <button role="switch" aria-checked={motion} aria-label={t("app.settings.animations")} className={motion ? "sw on" : "sw"}
            onClick={() => updateSettings({ motion: !motion })} />
          <span>
            <b>{t("app.settings.animations")}</b>
            <small>{s.motion === null && systemReducesMotion() ? t("app.settings.animationsSystem") : t("app.settings.animationsHint")}</small>
          </span>
        </label>
      </Group>
    </div>
  );
}
