// Settings > Updates: this version, whether a newer one is out, and what every version changed.
// When to install is a setting in System, which the line under the card links to. While a new version installs, UpdateProgress stands in front of every page.
import { openUrl } from "./opener";
import { useEffect, useMemo, useState, type ReactNode } from "react";
import logo from "./assets/logo.png";
import { CheckIcon, ChevronIcon, ExternalIcon, UpdateIcon, WarnIcon } from "./Icons";
import { language, t } from "./i18n";
import { LINKS } from "./links";
import { Markdown, parse } from "./Markdown";
import { useAppSettings } from "./settings";
import type { Snapshot } from "./types";
import { changelog, checkForUpdate, installUpdate, loadReleases, useReleases, useUpdates, type ChangelogEntry } from "./updates";

export function UpdatesTab({ snap, onSystem }: { snap: Snapshot; onSystem: () => void }) {
  const settings = useAppSettings()!;
  return (
    <>
      <UpdateCard snap={snap} />
      {!snap.portable && (
        <p className="upd-foot">
          {t(`app.settings.updates.${settings.updates}Hint`)} <button className="link" onClick={onSystem}>{t("app.settings.change")}</button>
        </p>
      )}
      <Changelog current={snap.version} />
    </>
  );
}

function UpdateCard({ snap }: { snap: Snapshot }) {
  const u = useUpdates();
  const busy = u.phase === "checking" || u.phase === "downloading" || u.phase === "installing";
  const pct = u.total ? Math.round((u.got / u.total) * 100) : 0;
  let tone = "current", icon: ReactNode = <CheckIcon />, line = t("app.update.none"), detail: string | null = null, action: ReactNode = null;
  if (u.phase === "checking" || u.phase === "idle") {
    tone = "wait"; icon = <span className="upd-spin" aria-hidden="true" />; line = t("app.update.checking");
  } else if (u.phase === "failed") {
    tone = "warn"; icon = <WarnIcon />; line = t("app.update.failed");
  } else if (u.phase === "downloading" || u.phase === "installing") {
    tone = "new"; icon = <UpdateIcon />; line = u.phase === "installing" ? t("app.update.installing") : t("app.update.downloading", pct);
  } else if (u.phase === "found") {
    tone = "new"; icon = <UpdateIcon />; line = t("app.update.found", u.version ?? "");
    detail = u.error ? t("app.update.failedInstall", u.error) : snap.portable ? t("app.settings.portableHint") : null;
    if (!snap.portable) action = <button className="btn small play" onClick={installUpdate}>{t("app.update.install")}</button>;
  }
  return (
    <section className={`set-section upd-card ${tone}`}>
      <div className="upd-app">
        <img src={logo} className="about-logo" alt="" />
        <b>{t("app.settings.version", snap.version)}</b>
        <button className="btn small" disabled={busy} onClick={() => checkForUpdate()}>{t("app.update.checkNow")}</button>
        <button className="btn small" onClick={() => openUrl(LINKS.releases).catch(() => {})}>{t("app.settings.releases")} <ExternalIcon /></button>
      </div>
      <div className="upd-state" role="status">
        <span className={`upd-icon ${tone}`}>{icon}</span>
        <div className="upd-text">
          <b>{line}</b>
          {detail && <span>{detail}</span>}
        </div>
        {action}
      </div>
    </section>
  );
}

const dateOf = (iso: string | null) => (iso ? new Date(iso).toLocaleDateString(language(), { dateStyle: "medium" }) : "");

/** New versions and the installed one start open, older ones folded. */
function Changelog({ current }: { current: string }) {
  const releases = useReleases();
  useEffect(loadReleases, []);
  const entries = useMemo(() => changelog(releases.list, current), [releases.list, current]);
  const [open, setOpen] = useState<Record<string, boolean>>({});
  const isOpen = (e: ChangelogEntry) => open[e.version] ?? e.when !== "older";
  return (
    <section className="upd-log">
      <h2 className="cap">{t("app.update.changelog")}</h2>
      {releases.failed && <p className="upd-empty">{t("app.update.changelogFailed")}</p>}
      {entries.length === 0 && !releases.failed && <p className="upd-empty">{t("app.update.noReleases")}</p>}
      <ol className="upd-timeline">
        {entries.map((e) => (
          <li key={e.version} className={`upd-entry ${e.when}${isOpen(e) ? " open" : ""}`}>
            <button className="upd-head" aria-expanded={isOpen(e)} onClick={() => setOpen((o) => ({ ...o, [e.version]: !isOpen(e) }))}>
              <ChevronIcon />
              <b>{e.version}</b>
              {e.when === "new" && <span className="upd-badge new">{t("app.update.badgeNew")}</span>}
              {e.when === "installed" && <span className="upd-badge">{t("app.update.badgeInstalled")}</span>}
              <span className="date">{dateOf(e.date)}</span>
            </button>
            {isOpen(e) && (
              <div className="upd-body">
                {e.notes ? <Markdown tokens={parse(e.notes).tokens} /> : <p className="muted">{t("app.update.noNotes")}</p>}
                {e.url && <button className="link small" onClick={() => openUrl(e.url!).catch(() => {})}>{t("app.update.onGithub")}</button>}
              </div>
            )}
          </li>
        ))}
      </ol>
    </section>
  );
}
