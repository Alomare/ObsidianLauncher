// About Obsidian: the version, the license, links and the credits. Opens from the status bar.
import { openUrl } from "./opener";
import { useEffect, useRef } from "react";
import logo from "./assets/logo.png";
import { CloseIcon, DiscordIcon, ExternalIcon, KofiIcon } from "./Icons";
import { t } from "./i18n";
import { LINKS } from "./links";

const CREDITS: { name: string; by: string; url: string; license: string; what: string }[] = [
  { name: "LuaJIT", by: "Mike Pall", url: "https://luajit.org", license: "MIT", what: "app.about.forLuajit" },
  { name: "Saira", by: "Omnibus-Type", url: "https://fonts.google.com/specimen/Saira", license: "OFL 1.1", what: "app.about.forSaira" },
  { name: "Barlow", by: "Jeremy Tribby", url: "https://fonts.google.com/specimen/Barlow", license: "OFL 1.1", what: "app.about.forBarlow" },
  { name: "Tauri", by: "Tauri Programme", url: "https://tauri.app", license: "MIT / Apache 2.0", what: "app.about.forTauri" },
  { name: "HELLDIVERS UI concept", by: "Steven Webb", url: "https://stevenwebbdesign.com/helldivers", license: "", what: "app.about.forConcept" },
];

const open = (url: string) => openUrl(url).catch(() => {});

export function AboutDialog({ version, onClose }: { version: string; onClose: () => void }) {
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    box.current?.focus();
    const key = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [onClose]);
  return (
    <div className="modal-back" onPointerDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal about" role="dialog" aria-modal="true" aria-label={t("app.about.title")} ref={box} tabIndex={-1}>
        <div className="about-head">
          <img src={logo} className="about-logo" alt="" />
          <div className="about-name">
            <b className="cap">Obsidian</b>
            <span>v{version}</span>
          </div>
          <button className="bare" onClick={onClose} aria-label={t("app.close")}><CloseIcon /></button>
        </div>
        <p className="muted">{t("app.about.blurb")}</p>
        <div className="row-btns">
          <button className="btn small" onClick={() => open(LINKS.repo)}>{t("app.about.source")} <ExternalIcon /></button>
          <button className="btn small" onClick={() => open(LINKS.discord)}><DiscordIcon /> Discord <ExternalIcon /></button>
          <button className="btn small" onClick={() => open(LINKS.kofi)}><KofiIcon /> Ko-fi <ExternalIcon /></button>
        </div>
        <dl className="facts">
          <dt>{t("app.about.license")}</dt>
          <dd><button className="link" onClick={() => open(LINKS.license)}>Apache License 2.0</button></dd>
        </dl>
        <h2 className="cap">{t("app.about.credits")}</h2>
        <ul className="credits">
          {CREDITS.map((c) => (
            <li key={c.name}>
              <button className="link" onClick={() => open(c.url)}>{c.name}</button>
              <span className="muted"> {t("app.about.by", c.by)}</span>
              {c.license && <span className="credit-license">{c.license}</span>}
              <span className="credit-for">{t(c.what)}</span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
