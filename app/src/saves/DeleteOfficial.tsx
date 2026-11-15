// The dialog before an Official save is deleted. Nothing in it deletes by accident: the name of the save
// has to be typed out (pasting does nothing), the Delete button stays off for the first seconds, Enter
// does nothing anywhere in the dialog, and focus starts in the name box, never on the button. The active
// Official save never gets here: the panel keeps Delete off for it, and the backend refuses it. The
// backend compares the typed name with the save again before deleting.
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { t } from "../i18n";
import type { SaveRow } from "./types";

/** How long the Delete button stays off after the dialog opens. */
export const DELETE_WAIT_S = 5;

/** Whether the typed text names the save. Exact: no trimming, no case folding. */
export const namesSave = (typed: string, name: string) => typed === name;

type Props = { row: SaveRow; onDelete: (typed: string) => Promise<boolean>; onCancel: () => void };

export function DeleteOfficial({ row, onDelete, onCancel }: Props) {
  const [typed, setTyped] = useState("");
  const [left, setLeft] = useState(DELETE_WAIT_S);
  const [busy, setBusy] = useState(false);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    input.current?.focus();
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !e.isComposing) onCancel();
      // No default answer: Enter never presses a button here.
      if (e.key === "Enter" && !e.isComposing) e.preventDefault();
    };
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  }, [onCancel]);
  useEffect(() => {
    if (left <= 0) return;
    const id = setTimeout(() => setLeft((n) => n - 1), 1000);
    return () => clearTimeout(id);
  }, [left]);

  const ready = left <= 0 && namesSave(typed, row.name) && !busy;
  const go = async () => {
    if (!ready) return;
    setBusy(true);
    if (!(await onDelete(typed))) setBusy(false);
  };

  return createPortal(
    <div className="modal-back" onPointerDown={(e) => e.target === e.currentTarget && !busy && onCancel()}>
      <div className="modal confirm delete-save" role="alertdialog" aria-modal="true" aria-label={t("app.saves.deleteTitle", row.name)}>
        <h2 className="cap">{t("app.saves.deleteTitle", row.name)}</h2>
        <p>{t("app.saves.deleteOfficialText")}</p>
        <p>{t("app.saves.deleteBackup")}</p>
        <label className="field">
          <span>{t("app.saves.deleteTypeName")} <b className="delete-name">{row.name}</b></span>
          <input ref={input} value={typed} maxLength={40} spellCheck={false} autoComplete="off" disabled={busy}
            onChange={(e) => setTyped(e.target.value)} onPaste={(e) => e.preventDefault()} onDrop={(e) => e.preventDefault()} />
        </label>
        <div className="modal-foot">
          <button type="button" className="btn" onClick={onCancel} disabled={busy}>{t("app.cancel")}</button>
          <button type="button" className="btn danger" disabled={!ready} onClick={go}>
            {left > 0 ? t("app.saves.deleteWait", left) : t("app.saves.delete")}
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
