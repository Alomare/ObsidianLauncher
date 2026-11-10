// The Commander tab: custom Galactic Wars (ObsidianCommander, docs/spec-m1.md). Obsidian keeps the
// frame: installing the package, the wars this PC plays, LAN join and which war the next launch
// plays. The package's server serves the war screens, shown here in a frame; a LAN war's screens
// come read-only from the PC that hosts it. The frame talks to Obsidian only through the messages
// in the spec's section 6, and only from the frame's own window.
import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { call, errorText, setSnapshot } from "../api";
import { withTask } from "../busy";
import { Confirm } from "../Confirm";
import { ContextMenu } from "../ContextMenu";
import { t } from "../i18n";
import { CheckIcon, CloseIcon, ImportIcon, TrashIcon, UpdateIcon } from "../Icons";
import { notify } from "../notices";
import type { Snapshot } from "../types";
import { PackageIcon, ThisPcIcon } from "./icons";

/** What Commander keeps on this PC, for Remove All Commander Data (`commander_footprint`). */
type Footprint = { package: string | null; hosted: number; joined: number; saves: { id: string; name: string }[]; namesFile: boolean; firewallRules: number; bytes: number };
type WipeReport = { removedSaves: number; keptSaves: string[]; firewall: { kind: "notAsked" | "none" | "removed" | "declined" | "failed"; count?: number; message?: string } };
const hasData = (f: Footprint | null) => !!f && (f.package != null || f.hosted > 0 || f.joined > 0 || f.saves.length > 0 || f.namesFile || f.firewallRules > 0 || f.bytes > 0);

type WarHost = "local" | "lan";
type WarRef = { id: string; name: string; host: WarHost; base?: string; save?: string; computer?: string };
type Installed = { version: string; protocol: number; server: string; ui: string };
type ServerInfo = { port: number; token: string };
export type CommanderView = { protocol: number; installed: Installed | null; wars: WarRef[]; playing: string | null; server: ServerInfo | null; names: [string, string][] };
type Done = { commander: CommanderView; snapshot: Snapshot };
type Found = { war: string; name: string; model: string; season: number; base: string; computer: string; players: number; version: string };
type Pick = { id: string; name: string; host: WarHost; base?: string; computer?: string };

/** What the frame shows: this PC's server, or one LAN war. */
type Source = { kind: "local" } | { kind: "lan"; war: WarRef };

const sameSource = (a: Source, b: Source) => a.kind === b.kind && (a.kind === "local" || (b.kind === "lan" && a.war.id === b.war.id));

type Props = { snap: Snapshot; visible: boolean; onPlay: () => void };

export function CommanderPage({ snap, visible, onPlay }: Props) {
  const [view, setView] = useState<CommanderView | null>(null);
  const [source, setSource] = useState<Source | null>(null);
  const [frameUrl, setFrameUrl] = useState<string | null>(null);
  const [starting, setStarting] = useState(false);
  const [joining, setJoining] = useState(false);
  const [menu, setMenu] = useState<{ x: number; y: number; opener: HTMLElement } | null>(null);
  const [ask, setAsk] = useState<null | "remove" | "wipe">(null);
  const [footprint, setFootprint] = useState<Footprint | null>(null);
  const frame = useRef<HTMLIFrameElement>(null);
  const running = snap.game === "running";

  const load = useCallback(async () => {
    try { setView(await call<CommanderView>("commander_state")); } catch (e) { notify("error", errorText(e)); }
  }, []);
  useEffect(() => { if (visible) load(); }, [visible, load]);
  // Without the package, the intro says when Commander still keeps something on this PC.
  const loadFootprint = useCallback(async () => {
    try { setFootprint(await call<Footprint>("commander_footprint")); } catch { setFootprint(null); }
  }, []);
  useEffect(() => { if (visible && view && !view.installed) loadFootprint(); }, [visible, view, loadFootprint]);

  const lanWars = useMemo(() => (view?.wars ?? []).filter((w) => w.host === "lan"), [view]);
  const installed = !!view?.installed;

  // The first source: this PC when the package is there, else the first LAN war.
  useEffect(() => {
    if (!view || source) return;
    if (installed) setSource({ kind: "local" });
    else if (lanWars[0]) setSource({ kind: "lan", war: lanWars[0] });
  }, [view, source, installed, lanWars]);

  // The frame's address: this PC's server, started when needed, or the LAN host's read-only screens.
  useEffect(() => {
    if (!source) { setFrameUrl(null); return; }
    if (source.kind === "lan") {
      setFrameUrl(`${source.war.base}/?war=${encodeURIComponent(source.war.id)}`);
      return;
    }
    if (!installed) { setFrameUrl(null); return; }
    let live = true;
    setStarting(true);
    call<ServerInfo>("commander_server_start")
      .then((s) => { if (live) setFrameUrl(`http://127.0.0.1:${s.port}/?token=${s.token}`); })
      .catch((e) => notify("error", errorText(e)))
      .finally(() => live && setStarting(false));
    return () => { live = false; };
  }, [source, installed]);

  const frameOrigin = frameUrl ? new URL(frameUrl).origin : null;
  const post = useCallback((msg: object) => {
    if (frame.current?.contentWindow && frameOrigin) frame.current.contentWindow.postMessage(msg, frameOrigin);
  }, [frameOrigin]);

  const hello = useCallback(() => {
    const theme = document.documentElement.dataset.theme === "light" ? "light" : "dark";
    post({ type: "obsidian:hello", theme, lang: document.documentElement.lang || "en", names: Object.fromEntries(view?.names ?? []), playing: view?.playing ?? null, game_running: running });
  }, [post, view, running]);

  // A theme or language switch while the frame is open reaches it as a new hello, which every
  // package version reads whole.
  useEffect(() => {
    const mo = new MutationObserver(() => hello());
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme", "lang"] });
    return () => mo.disconnect();
  }, [hello]);

  // The frame learns when the war being played or the game's state changes.
  useEffect(() => { post({ type: "obsidian:state", playing: view?.playing ?? null, game_running: running }); }, [post, view?.playing, running]);

  const applied = (r: Done) => { setView(r.commander); setSnapshot(r.snapshot); };

  const select = useCallback(async (pick: Pick, thenPlay: boolean) => {
    if (running) { notify("error", t("app.commander.gameRunning")); return; }
    try {
      const r = await withTask(t("app.commander.selecting"), () => call<Done>("commander_select", { war: pick }), true);
      applied(r);
      if (thenPlay) onPlay();
      else notify("info", t("app.commander.selected", pick.name));
    } catch (e) { notify("error", errorText(e)); }
  }, [running, onPlay]);

  const official = useCallback(async () => {
    if (running) { notify("error", t("app.commander.gameRunning")); return; }
    try {
      applied(await withTask(t("app.commander.selecting"), () => call<Done>("commander_official"), true));
      notify("info", t("app.commander.officialSelected"));
    } catch (e) { notify("error", errorText(e)); }
  }, [running]);

  // Messages from the frame, checked: its own window, a known type, plain fields.
  useEffect(() => {
    const on = (e: MessageEvent) => {
      if (!frame.current || e.source !== frame.current.contentWindow || e.origin !== frameOrigin) return;
      const m = e.data as { type?: unknown; war?: unknown; name?: unknown };
      if (!m || typeof m.type !== "string") return;
      if (m.type === "commander:ready") { hello(); return; }
      if (m.type === "commander:official") { official(); return; }
      if (m.type !== "commander:play" && m.type !== "commander:select") return;
      if (typeof m.war !== "string" || !/^[\w-]{1,32}$/.test(m.war)) return;
      const name = typeof m.name === "string" ? m.name.slice(0, 60) : m.war;
      const pick: Pick | null = source?.kind === "lan"
        ? (source.war.id === m.war ? { id: m.war, name, host: "lan", base: source.war.base, computer: source.war.computer } : null)
        : { id: m.war, name, host: "local" };
      if (pick) select(pick, m.type === "commander:play");
    };
    window.addEventListener("message", on);
    return () => window.removeEventListener("message", on);
  }, [frameOrigin, hello, official, select, source]);


  const installed_ = (v: CommanderView) => {
    setView(v);
    setSource({ kind: "local" });
    notify("info", t("app.commander.installed", v.installed?.version ?? ""));
  };

  // The newest release from GitHub.
  const install = async () => {
    try {
      installed_(await withTask(t("app.commander.fetching"), () => call<CommanderView>("commander_fetch"), true));
    } catch (e) {
      const code = (e as { code?: string })?.code;
      notify("error", code === "NoRelease" || code === "NoUpdateAsset" ? t("app.commander.noRelease") : errorText(e));
    }
  };

  // A package ZIP from disk, such as a local build.
  const installFile = async () => {
    const path = await open({ filters: [{ name: t("app.commander.zipFilter"), extensions: ["zip"] }] });
    if (typeof path !== "string") return;
    try {
      installed_(await withTask(t("app.commander.installing"), () => call<CommanderView>("commander_install", { path }), true));
    } catch (e) { notify("error", errorText(e)); }
  };

  const uninstall = async () => {
    setAsk(null);
    try {
      const v = await call<CommanderView>("commander_uninstall");
      setView(v);
      setSource(null);
      setFrameUrl(null);
      notify("info", t("app.commander.uninstalled"));
    } catch (e) { notify("error", errorText(e)); }
  };

  // Everything Commander keeps on this PC goes; the dialog says what that is first.
  const wiped = (r: { commander: CommanderView; snapshot: Snapshot; report: WipeReport }) => {
    setAsk(null);
    setSource(null);
    setFrameUrl(null);
    setView(r.commander);
    setSnapshot(r.snapshot);
    setFootprint(null);
    loadFootprint();
    notify("info", r.report.keptSaves.length ? t("app.commander.wipedKept", r.report.keptSaves.length) : t("app.commander.wiped"));
    const fw = r.report.firewall.kind;
    if (fw === "declined" || fw === "failed") notify("error", t("app.commander.firewallKept"));
  };

  const forget = async (war: WarRef) => {
    try {
      const v = await call<CommanderView>("commander_forget", { id: war.id });
      setView(v);
      if (source?.kind === "lan" && source.war.id === war.id) setSource(null);
    } catch (e) { notify("error", errorText(e)); }
  };

  const openMenu = (e: { currentTarget: HTMLButtonElement }) => {
    if (menu) { setMenu(null); return; }
    const r = e.currentTarget.getBoundingClientRect();
    setMenu({ x: r.right - 240, y: r.bottom + 4, opener: e.currentTarget });
  };

  if (!view) return <div className="commander" />;
  const playingWar = view.playing ? view.wars.find((w) => w.id === view.playing) : null;

  const dialogs = (
    <>
      {menu && (
        <ContextMenu x={Math.max(8, menu.x)} y={menu.y} opener={menu.opener} label={t("app.commander.package")} onClose={() => setMenu(null)} items={[
          ...(installed ? [
            { label: t("app.commander.packageVersion", view.installed?.version ?? ""), disabled: true },
            { label: t("app.commander.update"), icon: <UpdateIcon />, run: install },
          ] : []),
          { label: t("app.commander.installFile"), icon: <ImportIcon />, run: installFile },
          "sep",
          ...(installed ? [{ label: t("app.commander.uninstall"), icon: <TrashIcon />, run: () => setAsk("remove") }] : []),
          { label: t("app.commander.wipe"), icon: <TrashIcon />, danger: true, disabled: running, hint: running ? t("app.commander.gameRunning") : undefined, run: () => setAsk("wipe") },
        ]} />
      )}
      {ask === "remove" && (
        <Confirm title={t("app.commander.uninstall")} text={t("app.commander.uninstallText")} confirm={t("app.commander.uninstall")} onConfirm={uninstall} onCancel={() => setAsk(null)} />
      )}
      {ask === "wipe" && <WipeDialog running={running} onClose={() => setAsk(null)} onDone={wiped} />}
      {joining && <JoinDialog onClose={() => setJoining(false)} onJoin={(f) => {
        setJoining(false);
        const war: WarRef = { id: f.war, name: f.name, host: "lan", base: f.base, computer: f.computer };
        select({ id: f.war, name: f.name, host: "lan", base: f.base, computer: f.computer }, false).then(() => { load(); setSource({ kind: "lan", war }); });
      }} />}
    </>
  );

  if (!installed && lanWars.length === 0) {
    return (
      <div className="commander">
        <Intro onInstall={install} onInstallFile={installFile} onJoin={() => setJoining(true)} leftovers={hasData(footprint) ? footprint : null} onWipe={() => setAsk("wipe")} />
        {dialogs}
      </div>
    );
  }

  return (
    <div className="commander shell">
      <div className="cmd-bar">
        <div className="cmd-sources" role="tablist">
          {installed && (
            <button role="tab" aria-selected={source?.kind === "local"} className={source?.kind === "local" ? "cmd-src cur" : "cmd-src"} onClick={() => setSource({ kind: "local" })}
              data-tip={t("app.commander.thisPcHint")}>
              <ThisPcIcon /><span>{t("app.commander.thisPc")}</span>
            </button>
          )}
          {lanWars.map((w) => {
            const s: Source = { kind: "lan", war: w };
            const cur = !!source && sameSource(source, s);
            return (
              <span key={w.id} className={cur ? "cmd-src lan cur" : "cmd-src lan"}>
                <button role="tab" aria-selected={cur} onClick={() => setSource(s)} data-tip={w.computer ? t("app.commander.hostedBy", w.computer) : undefined}>
                  <span>{w.name}</span>
                </button>
                <button className="cmd-forget" aria-label={t("app.commander.forget", w.name)} data-tip={t("app.commander.forget", w.name)} onClick={() => forget(w)}><CloseIcon /></button>
              </span>
            );
          })}
          <button className="btn cmd-join" onClick={() => setJoining(true)}>{t("app.commander.joinLan")}</button>
        </div>
        <div className="cmd-next">
          <span className="cmd-next-label">{t("app.commander.nextLaunch")}</span>
          <span className={playingWar ? "cmd-next-war on" : "cmd-next-war"} data-tip={t("app.commander.nextLaunchHint")}>{playingWar ? playingWar.name : t("app.commander.officialWar")}</span>
          {playingWar && <button className="btn" disabled={running} onClick={official} data-tip={running ? t("app.commander.gameRunning") : t("app.commander.officialHint")}>{t("app.commander.officialWar")}</button>}
          {!installed && <button className="btn" onClick={install}>{t("app.commander.install")}</button>}
          <button className={menu ? "btn icon cmd-pkg open" : "btn icon cmd-pkg"} aria-haspopup="menu" aria-expanded={!!menu} aria-label={t("app.commander.package")}
            data-tip={menu ? undefined : t("app.commander.package")} onClick={openMenu}><PackageIcon /></button>
        </div>
      </div>
      <div className="cmd-stage">
        {frameUrl ? (
          <iframe ref={frame} key={frameUrl} className="cmd-frame" src={frameUrl} title={t("app.nav.commander")} sandbox="allow-scripts allow-same-origin allow-forms allow-downloads" />
        ) : (
          <div className="cmd-wait">{starting ? t("app.commander.starting") : t("app.commander.nothing")}</div>
        )}
      </div>
      {dialogs}
    </div>
  );
}

/** Remove All Commander Data: lists what Commander keeps here, with the war saves and the firewall
 *  rules as choices, since those are the player's progress and need Windows' permission. */
function WipeDialog({ running, onClose, onDone }: { running: boolean; onClose: () => void; onDone: (r: { commander: CommanderView; snapshot: Snapshot; report: WipeReport }) => void }) {
  const [fp, setFp] = useState<Footprint | null>(null);
  const [saves, setSaves] = useState(true);
  const [firewall, setFirewall] = useState(true);
  const [busy, setBusy] = useState(false);
  // Once per opening: the count reads the firewall rules through netsh, which takes a moment.
  const close = useRef(onClose);
  close.current = onClose;
  useEffect(() => {
    call<Footprint>("commander_footprint").then(setFp).catch((e) => { notify("error", errorText(e)); close.current(); });
  }, []);
  const go = async () => {
    if (busy || running || !fp) return;
    setBusy(true);
    try {
      onDone(await withTask(t("app.commander.wiping"), () => call<{ commander: CommanderView; snapshot: Snapshot; report: WipeReport }>("commander_wipe", { saves: saves && !!fp?.saves.length, firewall: firewall && !!fp?.firewallRules }), true));
    } catch (e) {
      notify("error", errorText(e));
      setBusy(false);
    }
  };
  const mb = fp ? Math.max(0.1, fp.bytes / 1048576) : 0;
  const lines = fp ? [
    fp.package != null ? (fp.package ? t("app.commander.wipePackage", fp.package) : t("app.commander.wipePackageAny")) : null,
    fp.hosted ? t("app.commander.wipeHosted", fp.hosted) : null,
    fp.joined ? t("app.commander.wipeJoined", fp.joined) : null,
    t("app.commander.wipeFiles", mb.toFixed(1)),
  ].filter(Boolean) as string[] : [];
  return (
    <Confirm title={t("app.commander.wipe")} text={running ? t("app.commander.gameRunning") : t("app.commander.wipeText")} confirm={t("app.commander.wipeConfirm")} onConfirm={go} onCancel={onClose}>
      {!fp ? <p className="cmd-wipe-wait">{t("app.commander.wipeLooking")}</p> : (
        <div className="cmd-wipe">
          <ul>{lines.map((l) => <li key={l}>{l}</li>)}</ul>
          {fp.saves.length > 0 && (
            <button role="checkbox" aria-checked={saves} className="check-line" onClick={() => setSaves(!saves)} data-tip={fp.saves.map((s) => s.name).join("\n")}>
              <span className={saves ? "ck on" : "ck"} aria-hidden="true">{saves && <CheckIcon />}</span>
              {t("app.commander.wipeSaves", fp.saves.length)}
            </button>
          )}
          {fp.firewallRules > 0 && (
            <button role="checkbox" aria-checked={firewall} className="check-line" onClick={() => setFirewall(!firewall)} data-tip={t("app.commander.wipeFirewallHint")}>
              <span className={firewall ? "ck on" : "ck"} aria-hidden="true">{firewall && <CheckIcon />}</span>
              {t("app.commander.wipeFirewall", fp.firewallRules)}
            </button>
          )}
        </div>
      )}
    </Confirm>
  );
}

function Intro({ onInstall, onInstallFile, onJoin, leftovers, onWipe }: { onInstall: () => void; onInstallFile: () => void; onJoin: () => void; leftovers: Footprint | null; onWipe: () => void }) {
  const left = leftovers ? [
    leftovers.hosted ? t("app.commander.wipeHosted", leftovers.hosted) : null,
    leftovers.joined ? t("app.commander.wipeJoined", leftovers.joined) : null,
    leftovers.saves.length ? t("app.commander.leftSaves", leftovers.saves.length) : null,
    leftovers.firewallRules ? t("app.commander.leftFirewall", leftovers.firewallRules) : null,
  ].filter(Boolean).join(", ") : "";
  return (
    <div className="cmd-intro">
      <div className="cmd-hero">
        <GalaxyArt />
        <div className="cmd-hero-text">
          <h1 className="cap">{t("app.commander.title")}</h1>
          <p className="cmd-lead">{t("app.commander.lead")}</p>
          <div className="cmd-actions">
            <button className="btn play" onClick={onInstall} data-tip={t("app.commander.installNote")}>{t("app.commander.install")}</button>
            <button className="btn" onClick={onJoin} data-tip={t("app.commander.joinHint")}>{t("app.commander.joinLan")}</button>
          </div>
          <p className="cmd-note"><button className="link" onClick={onInstallFile} data-tip={t("app.commander.installFileHint")}>{t("app.commander.installFile")}</button></p>
          {leftovers && (
            <p className="cmd-left">
              <span>{left ? t("app.commander.leftover", left) : t("app.commander.leftoverFiles")}</span>
              <button className="link" onClick={onWipe}>{t("app.commander.wipe")}</button>
            </p>
          )}
        </div>
      </div>
      <div className="cmd-cards">
        <div className="cmd-card"><h3>{t("app.commander.cardVanilla")}</h3><p>{t("app.commander.cardVanillaText")}</p></div>
        <div className="cmd-card later"><h3>{t("app.commander.cardEvolved")}</h3><p>{t("app.commander.cardEvolvedText")}</p></div>
        <div className="cmd-card"><h3>{t("app.commander.cardLan")}</h3><p>{t("app.commander.cardLanText")}</p></div>
        <div className="cmd-card"><h3>{t("app.commander.cardSafe")}</h3><p>{t("app.commander.cardSafeText")}</p></div>
      </div>
    </div>
  );
}

/** The game's war map in miniature: three fronts of ten rings around Super Earth. */
function GalaxyArt() {
  const fronts = [
    { from: -165, to: -60, color: "var(--cmd-cyborgs)" },
    { from: -45, to: 60, color: "var(--cmd-bugs)" },
    { from: 75, to: 180, color: "var(--cmd-illuminates)" },
  ];
  const arc = (r0: number, r1: number, a0: number, a1: number) => {
    const p = (r: number, a: number) => `${100 + r * Math.cos((a * Math.PI) / 180)} ${100 + r * Math.sin((a * Math.PI) / 180)}`;
    return `M${p(r0, a0)} A${r0} ${r0} 0 0 1 ${p(r0, a1)} L${p(r1, a1)} A${r1} ${r1} 0 0 0 ${p(r1, a0)}Z`;
  };
  return (
    <svg className="cmd-art" viewBox="0 0 200 200" aria-hidden="true">
      {fronts.map((f, i) => Array.from({ length: 10 }, (_, k) => {
        const r0 = 22 + k * 7.2, r1 = r0 + 6.4;
        const liberated = k < 3 + i;
        return <path key={`${i}-${k}`} d={arc(r0, r1, f.from, f.to)} className={liberated ? "cmd-art-lib" : ""} style={liberated ? undefined : { fill: f.color }} />;
      }))}
      <circle cx="100" cy="100" r="16" className="cmd-art-earth" />
    </svg>
  );
}

function JoinDialog({ onClose, onJoin }: { onClose: () => void; onJoin: (f: Found) => void }) {
  const [found, setFound] = useState<Found[] | null>(null);
  const [scanning, setScanning] = useState(false);
  const scan = useCallback(async () => {
    setScanning(true);
    try { setFound(await call<Found[]>("commander_scan", { ms: 3500 })); } catch (e) { notify("error", errorText(e)); setFound([]); }
    setScanning(false);
  }, []);
  useEffect(() => { scan(); }, [scan]);
  useEffect(() => {
    const key = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [onClose]);
  return createPortal(
    <div className="modal-back" onPointerDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal cmd-join-dialog" role="dialog" aria-modal="true" aria-label={t("app.commander.joinLan")}>
        <div className="head">
          <h2 className="cap">{t("app.commander.joinLan")}</h2>
          <button className="close" aria-label={t("app.close")} onClick={onClose}><CloseIcon /></button>
        </div>
        <p className="lead">{t("app.commander.joinLead")}</p>
        <div className="cmd-found">
          {scanning && <div className="cmd-found-empty">{t("app.commander.scanning")}</div>}
          {!scanning && found?.length === 0 && <div className="cmd-found-empty">{t("app.commander.noneFound")}</div>}
          {!scanning && found?.map((f) => (
            <div key={`${f.war}@${f.base}`} className="cmd-found-row">
              <div className="cmd-found-text">
                <strong>{f.name}</strong>
                <span>{t("app.commander.foundLine", f.computer || f.base, f.season, f.players)}</span>
              </div>
              <button className="btn go" onClick={() => onJoin(f)}>{t("app.commander.join")}</button>
            </div>
          ))}
        </div>
        <div className="modal-foot">
          <button className="btn" disabled={scanning} onClick={scan}>{t("app.commander.scanAgain")}</button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
