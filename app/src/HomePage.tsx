// Home: the game's art under a blur and a dark gradient that come from the top left corner, the
// name at the top left and links at the bottom left. Each link opens in the browser; the tooltip
// shows where. The button beside the links tucks them away for a clear view of the art, and
// brings them back; the choice is remembered.
//
// The art goes round by itself: each picture fills the page, drifts in very slowly (a 2% zoom over
// its time on screen) and gives way to the next with a cross-fade. The small controls at the bottom
// right go to the one before or after. With animations off one picture stays still, and the next
// launch shows the next one.
import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import logo from "./assets/logo-shaded.png";
import { ChevronIcon, DiscordIcon, GitHubIcon, KofiIcon, OutboundIcon, SearchIcon } from "./Icons";
import { t } from "./i18n";
import { LINKS } from "./links";
import { useAppSettings } from "./settings";
import { motionOn, systemReducesMotion } from "./theme";

const TUCK_KEY = "obsidian.homeLinksTucked";
const ART_KEY = "obsidian.homeArt";
const ART_SECONDS = 20; // how long a picture stays before the next one comes
const ART = Object.entries(import.meta.glob("./assets/home/*.webp", { eager: true, query: "?url", import: "default" }))
  .sort(([a], [b]) => a.localeCompare(b))
  .map(([, url]) => url as string);

/** The picture a launch starts on: the one after the last launch's. */
function firstPicture() {
  let at = 0;
  try {
    at = (Number(localStorage.getItem(ART_KEY) ?? -1) + 1) % ART.length || 0;
    localStorage.setItem(ART_KEY, String(at));
  } catch { /* storage off: start on the first */ }
  return at;
}
let opening: number | null = null; // chosen once per launch; coming back to Home carries on from there

/** The picture that shows and, while it fades in, the one under it. A new key replays the fade and the zoom. */
function useArt() {
  const settings = useAppSettings();
  const moving = motionOn(settings?.motion ?? null, systemReducesMotion()) && ART.length > 1;
  const [layers, setLayers] = useState(() => [{ key: 0, at: (opening ??= firstPicture()) }]);
  const top = layers[layers.length - 1];
  useEffect(() => { opening = top.at; }, [top.at]);

  // A window that was hidden when a picture was due carries on when it shows again.
  const [woken, wake] = useState(0);
  useEffect(() => {
    const seen = () => { if (!document.hidden) wake((n) => n + 1); };
    document.addEventListener("visibilitychange", seen);
    return () => document.removeEventListener("visibilitychange", seen);
  }, []);

  /** Straight to the picture before or after this one; its full time starts again. */
  const step = (by: number) => setLayers((ls) => {
    const cur = ls[ls.length - 1];
    const next = { key: cur.key + 1, at: (cur.at + by + ART.length) % ART.length };
    return moving ? [cur, next] : [next];
  });

  useEffect(() => {
    if (!moving) return;
    const ready = new Image(); // read ahead, so the cross-fade never starts on a picture still loading
    ready.src = ART[(top.at + 1) % ART.length];
    const id = setTimeout(() => {
      if (document.hidden) return; // nobody is looking
      const show = () => setLayers((ls) => {
        const cur = ls[ls.length - 1];
        return [cur, { key: cur.key + 1, at: (cur.at + 1) % ART.length }];
      });
      ready.decode().then(show, show);
    }, ART_SECONDS * 1000);
    return () => clearTimeout(id);
  }, [moving, top.key, top.at, woken]);

  return { layers: moving ? layers : [top], top, moving, step };
}

function Link({ url, cls, icon, label, i, hidden }: { url: string; cls: string; icon: ReactNode; label: string; i: number; hidden: boolean }) {
  return (
    <button className={`btn ${cls}`} style={{ ["--i" as string]: i }} tabIndex={hidden ? -1 : undefined} aria-hidden={hidden}
      onClick={() => openUrl(url).catch(() => {})} data-tip-title={label} data-tip={url}>
      {icon}
      {label}
      <OutboundIcon />
    </button>
  );
}

function readTucked(): boolean {
  try {
    return localStorage.getItem(TUCK_KEY) === "1";
  } catch {
    return false;
  }
}

export function HomePage() {
  const [tucked, setTucked] = useState(readTucked);
  const list = useRef<HTMLDivElement>(null);
  // Each link flies to and from the toggle button, so each needs to know how far away it is.
  useLayoutEffect(() => {
    for (const el of list.current?.children ?? []) {
      const b = el as HTMLElement;
      b.style.setProperty("--off", `${b.offsetLeft + b.offsetWidth / 2}px`);
    }
  });
  const toggle = () =>
    setTucked((v) => {
      try { localStorage.setItem(TUCK_KEY, v ? "0" : "1"); } catch { /* remembered for this visit only */ }
      return !v;
    });
  const label = tucked ? t("app.home.showLinks") : t("app.home.hideLinks");
  const art = useArt();
  const time = { ["--art-time" as string]: `${ART_SECONDS}s` } as CSSProperties;
  return (
    <div className="home">
      <div className="home-art" aria-hidden="true" style={time}>
        {art.layers.map((l) => <img key={l.key} className="home-art-img" src={ART[l.at]} alt="" draggable={false} />)}
      </div>
      <div className="home-blur" aria-hidden="true"><i /><i /><i /><i /></div>
      <div className="home-shade" aria-hidden="true" />
      <div className="home-body">
        <div className="home-brand">
          <img src={logo} alt="" draggable={false} />
          <div>
            <h1>OBSIDIAN</h1>
            <p>{t("app.home.subtitle")}</p>
          </div>
        </div>
        <div className="home-foot">
          <div className={tucked ? "home-links tucked" : "home-links"}>
            <button className="btn home-tuck" onClick={toggle} aria-expanded={!tucked} aria-label={label} data-tip={label}>
              <ChevronIcon />
            </button>
            <div className="home-link-list" ref={list} style={{ ["--n" as string]: 4 }}>
              <Link i={0} hidden={tucked} url={LINKS.nexus} cls="play" icon={<SearchIcon />} label={t("app.home.nexus")} />
              <Link i={1} hidden={tucked} url={LINKS.discord} cls="discord" icon={<DiscordIcon />} label={t("app.home.discord")} />
              <Link i={2} hidden={tucked} url={LINKS.kofi} cls="" icon={<KofiIcon />} label={t("app.home.sponsor")} />
              <Link i={3} hidden={tucked} url={LINKS.repo} cls="" icon={<GitHubIcon />} label={t("app.home.github")} />
            </div>
          </div>
          {ART.length > 1 && (
            <div className="home-steps">
              <button onClick={() => art.step(-1)} aria-label={t("app.home.prevPicture")} data-tip={t("app.home.prevPicture")}><ChevronIcon /></button>
              {/* How long until the next picture: refills with each one, and isn't there while pictures stay still. */}
              {art.moving && <span key={art.top.key} className="art-time" style={time} aria-hidden="true"><i /></span>}
              <button onClick={() => art.step(1)} aria-label={t("app.home.nextPicture")} data-tip={t("app.home.nextPicture")}><ChevronIcon /></button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
