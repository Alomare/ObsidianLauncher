import { CloseIcon } from "./Icons";
import { t } from "./i18n";
import { dismiss, useNotices } from "./notices";

export function Notices() {
  const list = useNotices();
  return (
    <div className="notices" role="status" aria-live="polite">
      {list.map((n) => (
        <div key={n.id} className={`notice ${n.kind}${n.leaving ? " leaving" : ""}`}>
          <span>{n.text}</span>
          <button className="bare" onClick={() => dismiss(n.id)} aria-label={t("app.close")}><CloseIcon /></button>
        </div>
      ))}
    </div>
  );
}
