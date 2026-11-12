// Home: the game's art under an even darkening, and one group in the middle of the window: the logo and
// name, then the links stacked in one box, all the same width. Each link opens in the browser; the
// tooltip shows where.
//
// The art goes round by itself: each picture fills the page, drifts in very slowly (a 2% zoom over
// the time on screen) and gives way to the next with a cross-fade. The small controls at the bottom
// center go to the one before or after. With animations off one picture stays still, and the next
// launch shows the next one.
import { openUrl } from "./opener";
import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import logo from "./assets/logo-shaded.png";
import { ChevronIcon, DiscordIcon, GitHubIcon, KofiIcon, OutboundIcon, SearchIcon } from "./Icons";
import { t } from "./i18n";
import { LINKS } from "./links";
import { useAppSettings } from "./settings";
import { motionOn, systemReducesMotion } from "./theme";

const ART_KEY = "obsidian.homeArt";
const ART_SECONDS = 30; // how long a picture stays before the next one comes
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

function Link({ url, cls, icon, label }: { url: string; cls: string; icon: ReactNode; label: string }) {
  return (
    <button className={`btn ${cls}`} onClick={() => openUrl(url).catch(() => {})} data-tip-title={label} data-tip={url}>
      {icon}
      {label}
      <OutboundIcon />
    </button>
  );
}

/** The width of the letters of a line as drawn, per unit of font size: from the first stroke to the last, with
    the letter spacing between them. */
function inkPerEm(el: HTMLElement): number {
  const style = getComputedStyle(el);
  const text = el.textContent ?? "";
  const pen = document.createElement("canvas").getContext("2d");
  if (!pen || !text) return 0;
  pen.font = style.font;
  const m = pen.measureText(text);
  const spacing = (parseFloat(style.letterSpacing) || 0) * ([...text].length - 1);
  return (m.actualBoundingBoxLeft + m.actualBoundingBoxRight + spacing) / parseFloat(style.fontSize);
}

/** Keeps the title as wide as the subtitle. The ratio of the two sizes depends only on the words and the
    font, so it is worked out here and the CSS scales both with the window. */
function useTitleFit() {
  const brand = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const box = brand.current;
    const h = box?.querySelector("h1");
    const p = box?.querySelector("p");
    if (!box || !h || !p) return;
    const fit = () => {
      const title = inkPerEm(h);
      if (title > 0) box.style.setProperty("--title-fit", String(inkPerEm(p) / title));
    };
    fit();
    if (typeof ResizeObserver === "undefined") return;
    // Fires when the font arrives, the language changes the subtitle, or the page comes back into view.
    const watch = new ResizeObserver(fit);
    watch.observe(h);
    watch.observe(p);
    return () => watch.disconnect();
  }, []);
  return brand;
}

export function HomePage() {
  const art = useArt();
  const brand = useTitleFit();
  const time = { ["--art-time" as string]: `${ART_SECONDS}s` } as CSSProperties;
  return (
    <div className="home">
      <div className="home-art" aria-hidden="true" style={time}>
        {art.layers.map((l) => <img key={l.key} className="home-art-img" src={ART[l.at]} alt="" draggable={false} />)}
      </div>
      <div className="home-shade" aria-hidden="true" />
      <div className="home-body">
        {ART.length > 1 && (
          <div className="home-steps">
            <button onClick={() => art.step(-1)} aria-label={t("app.home.prevPicture")} data-tip={t("app.home.prevPicture")}><ChevronIcon /></button>
            {/* How long until the next picture: refills with each one, and isn't there while pictures stay still. */}
            {art.moving && <span key={art.top.key} className="art-time" style={time} aria-hidden="true"><i /></span>}
            <button onClick={() => art.step(1)} aria-label={t("app.home.nextPicture")} data-tip={t("app.home.nextPicture")}><ChevronIcon /></button>
          </div>
        )}
        <div className="home-center">
          <div className="home-brand" ref={brand}>
            <img src={logo} alt="" draggable={false} />
            <div>
              <h1>OBSIDIAN</h1>
              <p>{t("app.home.subtitle")}</p>
            </div>
          </div>
          <div className="home-links">
            <Link url={LINKS.nexus} cls="play" icon={<SearchIcon />} label={t("app.home.nexus")} />
            <Link url={LINKS.discord} cls="discord" icon={<DiscordIcon />} label={t("app.home.discord")} />
            <Link url={LINKS.kofi} cls="" icon={<KofiIcon />} label={t("app.home.sponsor")} />
            <Link url={LINKS.repo} cls="" icon={<GitHubIcon />} label={t("app.home.github")} />
          </div>
        </div>
      </div>
    </div>
  );
}
