// The navigation bar: the tabs (Home, Mods, Wiki), then the "prepare to play" area: the Guardrails of the active
// profile, Purge, Deploy and Play, and the Settings tab at the far end (spec section 3). Deploy turns green while changes wait.
// When one of the three starts, one glow in its color crosses from Play toward the tabs, and fades out before them.
import { type ComponentType, useLayoutEffect, useRef, useState } from "react";
import { BuildDialog } from "./BuildDialog";
import { GuardrailsDialog, guardrailsSummary, guardsOn } from "./GuardrailsDialog";
import { BookIcon, CogIcon, DeployIcon, HomeIcon, ModsIcon, PlayIcon, PurgeIcon, ShieldIcon, WarnIcon } from "./Icons";
import { t } from "./i18n";
import type { Tab } from "./nav";
import type { Snapshot } from "./types";

/** `n` restarts the wave when one is already crossing. */
export type Wave = { tone: "red" | "green" | "grey" | "yellow"; n: number };

type Props = {
  tab: Tab;
  onTab: (t: Tab) => void;
  snap: Snapshot | null;
  busy: boolean;
  /** A new version of Obsidian is out: the Settings button says so, and opens on Updates. */
  updateFound: boolean;
  wave: Wave | null;
  onWaveEnd: () => void;
  onPurge: () => void;
  onDeploy: () => void;
  onPlay: () => void;
};

const TABS: [Tab, ComponentType][] = [["home", HomeIcon], ["mods", ModsIcon], ["wiki", BookIcon]];

export function NavBar({ tab, onTab, snap, busy, updateFound, wave, onWaveEnd, onPurge, onDeploy, onPlay }: Props) {
  const running = snap?.game === "running";
  const missing = snap?.game === "notFound";
  const locked = busy || running || !snap || missing;
  const why = running ? t("app.action.closeGameFirst") : missing ? t("app.error.GameNotFound") : undefined;
  const waiting = snap?.changes === "Waiting";
  const [guarding, setGuarding] = useState(false);
  const guards = snap ? guardsOn(snap) : 0;
  // HD1 gets no more updates, so only a missing game or an unknown build needs a word here.
  const warn = snap && (missing || snap.game === "unknown") ? t(`app.game.${snap.game}`) : null;
  const [showBuild, setShowBuild] = useState(false);
  // The glow runs from the end of Play to the end of the first tabs, so it never reaches them.
  const waveRef = useRef<HTMLSpanElement>(null);
  const tabsRef = useRef<HTMLDivElement>(null);
  const endRef = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const w = waveRef.current, tabs = tabsRef.current, end = endRef.current;
    if (!w || !tabs || !end) return;
    const nav = w.parentElement!.getBoundingClientRect();
    w.style.left = `${tabs.getBoundingClientRect().right - nav.left}px`;
    w.style.right = `${nav.right - end.getBoundingClientRect().right}px`;
  }, [wave]);
  return (
    <nav className="nav">
      {wave && <span key={wave.n} ref={waveRef} className={`nav-wave ${wave.tone}`} aria-hidden="true" onAnimationEnd={onWaveEnd} />}
      <div className="tabs" role="tablist" ref={tabsRef}>
        {TABS.map(([id, Icon]) => (
          <button key={id} role="tab" aria-selected={tab === id} aria-label={t(`app.nav.${id}`)} data-tip={t(`app.nav.${id}`)} className={tab === id ? "tab cur" : "tab"} onClick={() => onTab(id)}>
            <Icon />
            <span className="cap">{t(`app.nav.${id}`)}</span>
          </button>
        ))}
      </div>
      <div className="nav-end" ref={endRef}>
        {warn && missing && (
          <span className="nav-warn" data-tip-title={warn} data-tip={t("app.settings.build.notFound")}>
            <WarnIcon />
            {warn}
          </span>
        )}
        {warn && !missing && (
          <button className="bare nav-warn" onClick={() => setShowBuild(true)} aria-haspopup="dialog" data-tip-title={warn} data-tip={t("app.game.unknownHint")}>
            <WarnIcon />
            {warn}
          </button>
        )}
        {showBuild && snap?.game === "unknown" && <BuildDialog snap={snap} onClose={() => setShowBuild(false)} />}
        {guarding && snap && <GuardrailsDialog snap={snap} onClose={() => setGuarding(false)} />}
        <div className="actions">
          <button className={snap?.deployed ? "btn icon danger" : "btn icon"} onClick={onPurge} disabled={locked} data-tip-title={t("app.action.purge")} data-tip={why ?? t("app.action.purgeHint")} aria-label={t("app.action.purge")}>
            <PurgeIcon />
          </button>
          <button className={waiting ? "btn icon go" : "btn icon"} onClick={onDeploy} disabled={locked} data-tip-title={t("app.action.deploy")}
            data-tip={why ?? (waiting ? t("app.action.deployWaiting") : t(`app.action.deploy${snap?.changes ?? "UpToDate"}`))} aria-label={t("app.action.deploy")}>
            <DeployIcon />
          </button>
          {snap && (
            <button className={guards ? "btn icon guard on" : "btn icon guard"} onClick={() => setGuarding(true)} aria-haspopup="dialog" aria-label={t("app.nav.guardrails")}
              data-tip-title={t("app.guardrails.button", snap.profile)} data-tip={guardrailsSummary(snap)}>
              <ShieldIcon filled={guards > 0} />
              {guards > 0 && <i className="guard-count" aria-hidden="true">{guards}</i>}
            </button>
          )}
          <button className="btn play" onClick={onPlay} disabled={locked} data-tip={why ?? t("app.action.playHint")}>
            <PlayIcon />
            <span>{t("app.action.play")}</span>
          </button>
        </div>
      </div>
      <div className="tabs" role="tablist">
        <button role="tab" aria-selected={tab === "settings"} aria-label={updateFound ? `${t("app.nav.settings")}: ${t("app.status.updateFound")}` : t("app.nav.settings")}
          className={`tab${tab === "settings" ? " cur" : ""}${updateFound ? " lit" : ""}`} onClick={() => onTab("settings")}
          data-tip-title={updateFound ? t("app.nav.settings") : undefined} data-tip={updateFound ? t("app.status.updateFound") : t("app.nav.settings")}>
          <CogIcon />
          {updateFound && <i className="tab-dot" aria-hidden="true" />}
        </button>
      </div>
    </nav>
  );
}
