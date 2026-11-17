// A yes-or-no dialog for the few actions that ask first: Purge, Remove and Delete profile.
// Enter confirms, Esc or a click outside cancels. The focus starts on the action once, when it opens.
import { useRef, type ReactNode } from "react";
import { t } from "./i18n";
import { Modal } from "./Modal";
import { PadGlyph } from "./pad/PadGlyph";

/** `link` is a way elsewhere, set apart on the left as a text link so the two buttons stay the answer. */
type Props = { title: string; text: string; confirm: string; cancel?: string; onConfirm: () => void; onCancel: () => void; tone?: "warn" | "go"; children?: ReactNode; link?: { label: string; onClick: () => void } };

export function Confirm({ title, text, confirm, cancel, onConfirm, onCancel, tone = "warn", children, link }: Props) {
  const ok = useRef<HTMLButtonElement>(null);
  return (
    <Modal className="modal confirm" role="alertdialog" label={title} onClose={onCancel} focus={ok}>
      <h2 className="cap">{title}</h2>
      <p>{text}</p>
      {children}
      <div className="modal-foot">
        {link && <button className="link foot-link" onClick={link.onClick}>{link.label}</button>}
        <button data-pad-back className="btn" onClick={onCancel}><PadGlyph role="east" />{cancel ?? t("app.cancel")}</button>
        <button className={tone === "go" ? "btn play" : "btn danger"} ref={ok} onClick={onConfirm}>{confirm}</button>
      </div>
    </Modal>
  );
}
