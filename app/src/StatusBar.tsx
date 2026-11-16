// Along the bottom: what the app is doing on the left (importing, deploying), with a glow that
// travels to the right and fades out; on the right the version (it leads to the updates in
// Settings), the notifications shown so far, and About.
import { useEffect, useRef, useState } from "react";
import { AboutDialog } from "./About";
import { useBusy } from "./busy";
import { useLayer } from "./Modal";
import { BellIcon, InfoIcon } from "./Icons";
import { language, t } from "./i18n";
import { useNoticeLog } from "./noticeLog";

/** The time today, the date and time before. */
function shownTime(at: number) {
  const d = new Date(at);
  const time = d.toLocaleTimeString(language(), { hour: "2-digit", minute: "2-digit" });
  return d.toDateString() === new Date().toDateString() ? time : `${d.toLocaleDateString(language(), { month: "short", day: "numeric" })} ${time}`;
}

export function StatusBar({ version, updateFound, onUpdates }: { version: string | null; updateFound: boolean; onUpdates: () => void }) {
  const [about, setAbout] = useState(false);
  const [log, setLog] = useState(false);
  const busy = useBusy();
  return (
    <footer className="statusbar">
      <div className="sb-task" role="status">
        {busy && <><span className="sb-wave" aria-hidden="true" /><span className="sb-busy">{busy}</span></>}
      </div>
      {version && (
        <button className={updateFound ? "sb-btn lit" : "sb-btn"} onClick={onUpdates} data-tip-title={`Obsidian ${version}`}
          data-tip={updateFound ? t("app.status.updateFound") : t("app.status.updates")}>
          v{version}{updateFound && <i className="sb-dot" aria-hidden="true" />}
        </button>
      )}
      <button className={log ? "sb-btn round on" : "sb-btn round"} data-pop onClick={() => setLog((v) => !v)} aria-expanded={log}
        aria-label={t("app.status.notifications")} data-tip-title={t("app.status.notifications")} data-tip={t("app.status.notificationsHint")}>
        <BellIcon />
      </button>
      <button className={about ? "sb-btn round on" : "sb-btn round"} onClick={() => setAbout(true)} aria-label={t("app.about.title")} data-tip-title={t("app.about.title")} data-tip={t("app.status.aboutHint")}>
        <InfoIcon />
      </button>
      {log && <NotificationList onClose={() => setLog(false)} />}
      {about && version && <AboutDialog version={version} onClose={() => setAbout(false)} />}
    </footer>
  );
}

/** The notices shown so far, newest first. Esc, a click outside or the bell again closes it. */
function NotificationList({ onClose }: { onClose: () => void }) {
  const log = useNoticeLog();
  const box = useRef<HTMLDivElement>(null);
  const close = useRef(onClose);
  close.current = onClose;
  useEffect(() => {
    box.current?.focus();
    const away = (e: PointerEvent) => {
      const el = e.target as Element;
      if (!box.current?.contains(el) && !el.closest?.("[data-pop]")) close.current();
    };
    document.addEventListener("pointerdown", away, true);
    return () => document.removeEventListener("pointerdown", away, true);
  }, []);
  useLayer(() => close.current());
  return (
    <div className="sb-pop" role="dialog" aria-label={t("app.status.notifications")} ref={box} tabIndex={-1}>
      <h2 className="cap">{t("app.status.notifications")}</h2>
      {log.length ? (
        <ol className="note-log">
          {[...log].reverse().map((n) => (
            <li key={n.id} className={`note-row ${n.kind}`}>
              <span className="note-time">{shownTime(n.at)}</span>
              <span>{n.text}</span>
            </li>
          ))}
        </ol>
      ) : (
        <p className="muted">{t("app.status.noNotifications")}</p>
      )}
    </div>
  );
}
