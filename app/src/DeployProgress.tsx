// The window while a deploy runs: the mod being worked on and the stage over the bar, since mods that
// rebuild sound banks can take a minute. Every deploy sends progress, so a quick one must not flash: the
// window waits a moment before showing, and once up stays long enough to read.
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useLocked } from "./busy";
import { t } from "./i18n";

type Progress = { percent: number; mod: string | null; stage: string };

/** A deploy that ends before this never shows the window. */
const SHOW_AFTER_MS = 350;
/** Once shown, it stays this long at least. */
const MIN_SHOWN_MS = 400;

/** The stage in the UI language: "sound banks" is the key `soundBanks`. */
function stageText(stage: string) {
  const key = `app.action.deployStage.${stage.replace(/ (\w)/g, (_, c: string) => c.toUpperCase())}`;
  const text = t(key);
  return text === key ? stage : text;
}

export function DeployProgress() {
  const locked = useLocked();
  const [p, setP] = useState<Progress | null>(null);
  const [show, setShow] = useState(false);
  const shownAt = useRef(0);
  const has = p !== null;
  useEffect(() => {
    const off = listen<Progress>("deploy-progress", (e) => setP(e.payload));
    return () => { off.then((f) => f()); };
  }, []);
  useEffect(() => {
    if (!has || show) return;
    const timer = setTimeout(() => { shownAt.current = performance.now(); setShow(true); }, SHOW_AFTER_MS);
    return () => clearTimeout(timer);
  }, [has, show]);
  useEffect(() => {
    if (locked) return;
    if (!show) { setP(null); return; }
    const timer = setTimeout(() => { setP(null); setShow(false); }, Math.max(0, MIN_SHOWN_MS - (performance.now() - shownAt.current)));
    return () => clearTimeout(timer);
  }, [locked, show]);
  if (!show || !p) return null;
  const title = t("app.action.deploying");
  return createPortal(
    <div className="modal-back">
      <div className="modal confirm" role="status" aria-label={title}>
        <h2 className="cap">{title}</h2>
        <p className="deploy-now">
          {p.mod && <b>{p.mod}</b>}
          <span>{stageText(p.stage)}</span>
          <span className="deploy-pct">{p.percent} %</span>
        </p>
        <div className="update-bar"><i style={{ width: `${p.percent}%` }} /></div>
      </div>
    </div>,
    document.body,
  );
}
