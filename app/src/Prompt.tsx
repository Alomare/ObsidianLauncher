// A dialog that asks for one line of text, such as a profile name. Enter confirms, Esc cancels.
// `onConfirm` returns false to keep the dialog open (the name was refused). The text is selected once,
// when it opens.
import { useRef, useState } from "react";
import { t } from "./i18n";
import { Modal } from "./Modal";
import { PadGlyph } from "./pad/PadGlyph";

type Props = { title: string; label: string; initial?: string; confirm: string; onConfirm: (v: string) => Promise<boolean>; onCancel: () => void };

export function Prompt({ title, label, initial = "", confirm, onConfirm, onCancel }: Props) {
  const [v, setV] = useState(initial);
  const [busy, setBusy] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const go = async () => {
    if (!v.trim() || busy) return;
    setBusy(true);
    let ok = false;
    try {
      ok = await onConfirm(v.trim());
    } finally {
      if (!ok) setBusy(false);
    }
  };
  return (
    <Modal className="modal confirm" label={title} onClose={onCancel} focus={input} onSubmit={(e) => { e.preventDefault(); go().catch(() => { /* the caller tells of its own errors */ }); }}>
      <h2 className="cap">{title}</h2>
      <label className="field">
        <span>{label}</span>
        <input ref={input} value={v} maxLength={40} onChange={(e) => setV(e.target.value)} spellCheck={false} />
      </label>
      <div className="modal-foot">
        <button data-pad-back type="button" className="btn" onClick={onCancel}><PadGlyph role="east" />{t("app.cancel")}</button>
        <button type="submit" className="btn play" disabled={!v.trim() || busy}>{confirm}</button>
      </div>
    </Modal>
  );
}
