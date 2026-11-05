import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { deploy, play, purge } from "./actions";
import { loadSnapshot, refresh, useSnapshot } from "./api";
import { useLocked } from "./busy";
import { Confirm } from "./Confirm";
import { pick, t } from "./i18n";
import { Markdown, parse } from "./Markdown";
import { listenForDrops, type DropState } from "./mods/dropImport";
import { checkModUpdates } from "./mods/modUpdates";
import { HomePage } from "./HomePage";
import { ModsPage } from "./mods/ModsPage";
import { NavBar, type Wave } from "./NavBar";
import { go as setTab, useNav } from "./nav";
import { WikiPage } from "./WikiPage";
import { SettingsPage, type SettingsTab } from "./SettingsPage";
import { Notices } from "./NoticeList";
import { loadSettings, useAppSettings } from "./settings";
import { StatusBar } from "./StatusBar";
import { Tooltip } from "./Tooltip";
import { installUpdate, startupUpdate, useUpdates } from "./updates";
import { DeployProgress } from "./DeployProgress";
import { UpdateProgress } from "./UpdateProgress";

export default function App() {
  const snap = useSnapshot();
  const settings = useAppSettings();
  const busy = useLocked();
  const { tab } = useNav();
  const [dropping, setDropping] = useState<DropState>(null);
  const [settingsTab, setSettingsTab] = useState<SettingsTab>("system");
  const [askUpdate, setAskUpdate] = useState(false);
  const [wave, setWave] = useState<Wave | null>(null);
  const update = useUpdates();
  const startWave = (tone: Wave["tone"]) => {
    if (document.documentElement.dataset.motion !== "off") setWave((w) => ({ tone, n: (w?.n ?? 0) + 1 }));
  };
  const ready = !!snap && !!settings;
  // The color of the active profile tints the Profile button and the switches that are on.
  const profileColor = snap?.profileRows.find((p) => p.name === snap.profile)?.color ?? null;
  useEffect(() => {
    const root = document.documentElement.style;
    if (profileColor) root.setProperty("--pc", profileColor);
    else root.removeProperty("--pc");
  }, [profileColor]);
  // The update checks run once per launch, as soon as the window knows whether it is a portable copy.
  useEffect(() => {
    if (!snap || !settings) return;
    startupUpdate(settings.updates, snap.portable).then((a) => setAskUpdate(a === "ask"));
    const updatable = snap.mods.filter((m) => m.updatable).map((m) => m.id);
    if (updatable.length) checkModUpdates(updatable, new Map(snap.mods.map((m) => [m.id, pick(m.name, m.id)])), { startup: true });
  }, [ready]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    loadSettings();
    loadSnapshot();
    // The browser menu (Back, Reload, Inspect) has no place in the app; text fields keep theirs.
    const menu = (e: MouseEvent) => !(e.target as Element).closest?.("input, textarea") && e.preventDefault();
    document.addEventListener("contextmenu", menu);
    // Files dragged in bring the Mods tab forward, since only the mod list takes them.
    const stopDrops = listenForDrops(() => document.querySelector(".mods .list"), () => setTab("mods"), setDropping);
    // The Rust side says when the game starts or stops; the snapshot then locks or frees the buttons.
    const game = listen("game-state", () => refresh());
    return () => {
      stopDrops();
      game.then((f) => f());
      document.removeEventListener("contextmenu", menu);
    };
  }, []);

  if (!settings) return null;
  return (
    <div className="app">
      <NavBar tab={tab} onTab={(to) => { if (to === "settings" && update.phase === "found") setSettingsTab("updates"); setTab(to); }} snap={snap} busy={busy} updateFound={update.phase === "found"} wave={wave} onWaveEnd={() => setWave(null)} onPurge={() => { startWave(snap?.deployed ? "red" : "grey"); purge(); }}
        onDeploy={() => { startWave(snap?.changes === "Waiting" ? "green" : "grey"); deploy(); }} onPlay={() => { startWave("yellow"); play(); }} />
      <main className={tab === "home" || tab === "wiki" ? "page bleed" : "page"}>
        {tab === "home" && <div className="view" key="home"><HomePage /></div>}
        {/* Mods stays mounted while hidden: a long list then costs nothing to come back to. */}
        {snap && <div className="view" hidden={tab !== "mods"}><ModsPage snap={snap} /></div>}
        {tab === "wiki" && <div className="view" key="wiki"><WikiPage /></div>}
        {snap && tab === "settings" && <div className="view" key="settings"><SettingsPage snap={snap} tab={settingsTab} onTab={setSettingsTab} /></div>}
      </main>
      <StatusBar version={snap?.version ?? null} updateFound={update.phase === "found"} onUpdates={() => { setSettingsTab("updates"); setTab("settings"); }} />
      {dropping && (
        <div className={dropping.inside ? "dropzone cap over" : "dropzone cap"}
          style={{ left: dropping.rect.left, top: dropping.rect.top, width: dropping.rect.width, height: dropping.rect.height }}>
          {t("app.mods.dropHere")}
        </div>
      )}
      {askUpdate && (
        <Confirm className="update-ask" title={t("app.update.title", update.version ?? "")} text={t("app.update.text")}
          confirm={t("app.update.install")} cancel={t("app.update.later")}
          tone="go" onCancel={() => setAskUpdate(false)} onConfirm={() => { setAskUpdate(false); installUpdate(); }}>
          {update.notes && <div className="update-notes"><Markdown tokens={parse(update.notes).tokens} /></div>}
        </Confirm>
      )}
      <UpdateProgress />
      <DeployProgress />
      <Notices />
      <Tooltip />
    </div>
  );
}
