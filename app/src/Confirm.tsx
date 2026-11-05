// A yes-or-no dialog for the few actions that ask first: Purge, Remove and Delete profile.
// Enter confirms, Esc or a click outside cancels. It sits on the body, so a folded corner's mask
// can't cut it off.
import { useEffect, useRef, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { t } from "./i18n";

/** `link` is a way elsewhere, set apart on the left as a text link so the two buttons stay the answer. */
type Props = { title: string; text: string; confirm: string; cancel?: string; onConfirm: () => void; onCancel: () => void; tone?: "warn" | "go"; children?: ReactNode; link?: { label: string; onClick: () => void } };

export function Confirm({ title, text, confirm, cancel, onConfirm, onCancel, tone = "warn", children, link }: Props) {
  const ok = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    ok.current?.focus();
    const key = (e: KeyboardEvent) => e.key === "Escape" && onCancel();
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [onCancel]);
  return createPortal(
    <div className="modal-back" onPointerDown={(e) => e.target === e.currentTarget && onCancel()}>
      <div className="modal confirm" role="alertdialog" aria-modal="true" aria-label={title}>
        <h2 className="cap">{title}</h2>
        <p>{text}</p>
        {children}
        <div className="modal-foot">
          {link && <button className="link foot-link" onClick={link.onClick}>{link.label}</button>}
          <button className="btn" onClick={onCancel}>{cancel ?? t("app.cancel")}</button>
          <button className={tone === "go" ? "btn play" : "btn danger"} ref={ok} onClick={onConfirm}>{confirm}</button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
