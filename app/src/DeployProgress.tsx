// The window while a long deploy writes its patches: mods that rebuild sound banks can take a minute, and
// the app has to show it's working. A short deploy never sends progress, so it shows nothing.
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { useLocked } from "./busy";
import { t } from "./i18n";

export function DeployProgress() {
  const locked = useLocked();
  const [percent, setPercent] = useState<number | null>(null);
  useEffect(() => {
    const off = listen<number>("deploy-progress", (e) => setPercent(e.payload));
    return () => { off.then((f) => f()); };
  }, []);
  useEffect(() => {
    if (!locked) setPercent(null);
  }, [locked]);
  if (percent === null || !locked) return null;
  const title = t("app.action.deploying");
  return createPortal(
    <div className="modal-back">
      <div className="modal confirm" role="status" aria-label={title}>
        <h2 className="cap">{title}</h2>
        <p>{t("app.action.deployProgress", percent)}</p>
        <div className="update-bar"><i style={{ width: `${percent}%` }} /></div>
      </div>
    </div>,
    document.body,
  );
}
