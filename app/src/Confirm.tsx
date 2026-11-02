// A yes-or-no dialog for the few actions that ask first: Purge, Remove and Delete profile.
// Enter confirms, Esc or a click outside cancels. It sits on the body, so a folded corner's mask
// can't cut it off.
import { useEffect, useRef, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { t } from "./i18n";

type Props = { title: string; text: string; confirm: string; onConfirm: () => void; onCancel: () => void; tone?: "warn" | "go"; children?: ReactNode; extra?: { label: string; onClick: () => void } };

export function Confirm({ title, text, confirm, onConfirm, onCancel, tone = "warn", children, extra }: Props) {
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
          {extra && <button className="btn" onClick={extra.onClick}>{extra.label}</button>}
          <button className="btn" onClick={onCancel}>{t("app.cancel")}</button>
          <button className={tone === "go" ? "btn play" : "btn danger"} ref={ok} onClick={onConfirm}>{confirm}</button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
