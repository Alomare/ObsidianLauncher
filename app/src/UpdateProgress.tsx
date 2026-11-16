// The window while Obsidian's own update downloads and starts installing. The page behind it may be
// any page, and the app is about to close, so it stays in front until then. A failed install ends
// it, and the reason shows in Settings beside the Install button.
import { createPortal } from "react-dom";
import { t } from "./i18n";
import { useUpdates } from "./updates";

export function UpdateProgress() {
  const u = useUpdates();
  if (u.phase !== "downloading" && u.phase !== "installing") return null;
  const installing = u.phase === "installing";
  const total = u.phase === "downloading" ? u.total : 0;
  const pct = u.phase === "downloading" && total ? Math.min(100, Math.round((u.got / total) * 100)) : 0;
  const title = t("app.update.progressTitle", u.version);
  return createPortal(
    <div className="modal-back">
      <div className="modal confirm" role="status" aria-label={title}>
        <h2 className="cap">{title}</h2>
        <p>{installing ? t("app.update.installing") : t("app.update.downloading", pct)}</p>
        <div className={installing || !total ? "update-bar waiting" : "update-bar"}>
          <i style={installing || !total ? undefined : { width: `${pct}%` }} />
        </div>
      </div>
    </div>,
    document.body,
  );
}
