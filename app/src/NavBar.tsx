// The navigation bar: the tabs (Home, Mods, Saves), then the Guardrails of the active profile and Play, and the
// Wiki and Settings tabs as icons at the far end. Deploy and Purge are on the Mods tab.
// When a deploy, a purge or Play starts, one glow in the action's color crosses from Play toward the tabs, and fades out before them.
import { type ComponentType, useEffect, useLayoutEffect, useRef, useState } from "react";
import { playingWar, playLabel, reloadChoices, useChoices } from "./commander/choices";
import { CommanderIcon, MenuIcon } from "./commander/icons";
import { PlayChoice } from "./commander/PlayChoice";
import { BuildDialog } from "./BuildDialog";
import { GuardrailsDialog, guardrailsSummary, guardsOn } from "./GuardrailsDialog";
import { BookIcon, CogIcon, HomeIcon, ModsIcon, PlayIcon, SaveIcon, ShieldIcon, WarnIcon } from "./Icons";
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
  onPlay: () => void;
};

// Home, Mods and Saves carry their names; the Wiki and Settings are icons at the far end (the user, 2026-11-04).
const TABS: [Tab, ComponentType][] = [["home", HomeIcon], ["mods", ModsIcon], ["saves", SaveIcon], ["commander", CommanderIcon]];

export function NavBar({ tab, onTab, snap, busy, updateFound, wave, onWaveEnd, onPlay }: Props) {
  const running = snap?.game === "running";
  const missing = snap?.game === "notFound";
  const locked = busy || running || !snap || missing;
  const why = running ? t("app.action.closeGameFirst") : missing ? t("app.error.GameNotFound") : undefined;
  const [guarding, setGuarding] = useState(false);
  const guards = snap ? guardsOn(snap) : 0;
  // HD1 gets no more updates, so only a missing game or an unknown build needs a word here.
  const warn = snap && (missing || snap.game === "unknown") ? t(`app.game.${snap.game}`) : null;
  const [showBuild, setShowBuild] = useState(false);
  // What Play starts: the war and whether the last deploy put mods in. With wars or saves to choose from,
  // Play splits, and the narrow part opens the Play choice.
  const choices = useChoices();
  const [choosing, setChoosing] = useState(false);
  useEffect(() => { reloadChoices(); }, [snap]);
  const war = playingWar(choices);
  const label = playLabel(war?.name ?? null, !!snap?.modded);
  // The choice opens whenever there is something to choose: a custom war beside the official one, or a second save.
  const split = (choices?.wars.length ?? 0) > 0 || (choices?.saves ?? 0) > 1;
  // Yellow with no mods deployed, green with mods: the color says it, the tooltip says it in words.
  const tone = snap?.modded ? " modded" : "";
  const playButton = (
    <button className={`btn play${tone}`} onClick={onPlay} disabled={locked} aria-label={label.text} data-tip-title={label.text} data-tip={why ?? t("app.action.playHint")}>
      <PlayIcon />
      <span className="play-text">
        <span className="play-word">{label.play}</span>
        {label.war && <span className="play-war">{label.war}</span>}
      </span>
    </button>
  );
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
          {snap || split ? (
            <div className="play-split">
              {snap && (
                <button className={guards ? "btn icon guard on" : "btn icon guard"} onClick={() => setGuarding(true)} aria-haspopup="dialog" aria-label={t("app.nav.guardrails")}
                  data-tip-title={t("app.guardrails.button", snap.profile)} data-tip={guardrailsSummary(snap)}>
                  <ShieldIcon filled={guards > 0} />
                </button>
              )}
              {playButton}
              {split && (
                <button className={`btn play play-more${tone}`} onClick={() => setChoosing(true)} disabled={!snap || missing} aria-haspopup="dialog" aria-expanded={choosing}
                  aria-label={t("app.play.choose")} data-tip={t("app.play.choose")}>
                  <MenuIcon />
                </button>
              )}
            </div>
          ) : playButton}
          {choosing && snap && choices && <PlayChoice snap={snap} choices={choices} onClose={() => setChoosing(false)} />}
        </div>
      </div>
      <div className="tabs end-tabs" role="tablist">
        <button role="tab" aria-selected={tab === "wiki"} aria-label={t("app.nav.wiki")} data-tip={t("app.nav.wiki")} className={tab === "wiki" ? "tab cur" : "tab"} onClick={() => onTab("wiki")}>
          <BookIcon />
        </button>
        <button role="tab" aria-selected={tab === "settings"} aria-label={updateFound ? `${t("app.nav.settings")}: ${t("app.status.updateFound")}` : t("app.nav.settings")}
          className={`tab tab-settings${tab === "settings" ? " cur" : ""}${updateFound ? " lit" : ""}`} onClick={() => onTab("settings")}
          data-tip-title={updateFound ? t("app.nav.settings") : undefined} data-tip={updateFound ? t("app.status.updateFound") : t("app.nav.settings")}>
          <CogIcon />
          {updateFound && <i className="tab-dot" aria-hidden="true" />}
        </button>
      </div>
    </nav>
  );
}
