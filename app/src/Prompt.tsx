// A dialog that asks for one line of text, such as a profile name. Enter confirms, Esc cancels.
// `onConfirm` returns false to keep the dialog open (the name was refused). It sits on the body, so
// a folded corner's mask can't cut it off.
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { t } from "./i18n";
import { PadGlyph } from "./pad/PadGlyph";

type Props = { title: string; label: string; initial?: string; confirm: string; onConfirm: (v: string) => Promise<boolean>; onCancel: () => void };

export function Prompt({ title, label, initial = "", confirm, onConfirm, onCancel }: Props) {
  const [v, setV] = useState(initial);
  const [busy, setBusy] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    input.current?.select();
    const key = (e: KeyboardEvent) => e.key === "Escape" && onCancel();
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [onCancel]);
  const go = async () => {
    if (!v.trim() || busy) return;
    setBusy(true);
    if (!(await onConfirm(v.trim()))) setBusy(false);
  };
  return createPortal(
    <div className="modal-back" onPointerDown={(e) => e.target === e.currentTarget && onCancel()}>
      <form className="modal confirm" role="dialog" aria-modal="true" aria-label={title} onSubmit={(e) => { e.preventDefault(); go(); }}>
        <h2 className="cap">{title}</h2>
        <label className="field">
          <span>{label}</span>
          <input ref={input} value={v} maxLength={40} onChange={(e) => setV(e.target.value)} spellCheck={false} />
        </label>
        <div className="modal-foot">
          <button data-pad-back type="button" className="btn" onClick={onCancel}>{t("app.cancel")}<PadGlyph role="east" /></button>
          <button type="submit" className="btn play" disabled={!v.trim() || busy}>{confirm}</button>
        </div>
      </form>
    </div>,
    document.body,
  );
}
