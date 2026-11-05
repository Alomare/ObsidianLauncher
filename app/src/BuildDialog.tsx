// The game-files dialog, opened from the warning in the navigation bar and from Settings when the game
// files differ from the build Obsidian knows. Deploy and Play still work; this says which files
// differ, what that usually means, and how to get the known files back.
import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { call, errorText, run } from "./api";
import { CloseIcon, FolderIcon, WarnIcon } from "./Icons";
import { t } from "./i18n";
import { notify } from "./notices";
import type { Snapshot } from "./types";

const KINDS: Record<string, string> = {
  "binaries/x64/helldivers.exe": "exe",
  "contents/9cf71054ea26dddf": "boot",
  "contents/9e13b2414b41b842": "gameplay",
};

/** Which of the known game files a path is: exe, boot, gameplay or other. */
export const fileKind = (path: string) => KINDS[path] ?? "other";

export function BuildDialog({ snap, onClose }: { snap: Snapshot; onClose: () => void }) {
  const [checking, setChecking] = useState(false);
  useEffect(() => {
    const key = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [onClose]);
  const changes = snap.gameChanges;
  const exe = changes.some((c) => fileKind(c.path) === "exe");
  const bundles = changes.some((c) => fileKind(c.path) !== "exe");
  const check = async () => {
    setChecking(true);
    await run("check_game");
    setChecking(false);
  };
  const openGame = () => snap.gameDir && call("open_folder", { path: snap.gameDir }).catch((e) => notify("error", errorText(e)));
  return createPortal(
    <div className="modal-back" onPointerDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal build" role="dialog" aria-modal="true" aria-label={t("app.build.title")}>
        <div className="head">
          <h2 className="cap"><WarnIcon />{t("app.build.title")}</h2>
          <button className="bare close" onClick={onClose} aria-label={t("app.close")} data-tip={t("app.close")}><CloseIcon /></button>
        </div>
        <p className="lead">{t("app.build.lead")}</p>
        {changes.length > 0 && (
          <ul className="build-files">
            {changes.map((c) => (
              <li key={c.path}>
                <span className="name">{t(`app.build.file.${fileKind(c.path)}`)}</span>
                <span className="mono muted">{c.path}</span>
                <span className={c.missing ? "tag bad" : "tag"}>{t(c.missing ? "app.build.missing" : "app.build.changed")}</span>
              </li>
            ))}
          </ul>
        )}
        <h3>{t("app.build.meansTitle")}</h3>
        {bundles && <p>{t("app.build.meansBundles")}</p>}
        {exe && <p>{t("app.build.meansExe")}</p>}
        <h3>{t("app.build.fixTitle")}</h3>
        <ol className="build-steps">
          <li>{t("app.build.fix1")}</li>
          <li>{t("app.build.fix2")}</li>
          <li>{t("app.build.fix3")}</li>
        </ol>
        <p className="muted note">{t("app.build.note")}</p>
        <div className="foot">
          {snap.gameDir && <button className="btn small" onClick={openGame}><FolderIcon />{t("app.settings.openFolder")}</button>}
          <button className="btn small" onClick={check} disabled={checking}>{t("app.settings.checkAgain")}</button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
