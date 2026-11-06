// The folder dialogs: a name and a colour for a new or existing folder, and the question asked
// before a folder with mods in it goes (keep the mods, or remove them too).
import { useEffect, useRef, useState } from "react";
import { openColorPicker } from "../ColorPicker";
import { t } from "../i18n";

export const FOLDER_COLORS = ["#f0b429", "#e8891c", "#d64545", "#c062b8", "#9b7bd4", "#4a90d9", "#2fb889", "#8a9099"];

/** A color outside the row's presets, as a folder or profile can have one. */
export const isCustomColor = (c: string | null | undefined, presets: string[] = FOLDER_COLORS) => !!c && !presets.some((p) => p.toLowerCase() === c.toLowerCase());

type SwatchProps = { current: string | null; presets?: string[]; onPick: (c: string) => void; onInput?: (c: string) => void; onOpen?: () => void; role?: "radio" | "menuitemradio" };

/**
 * The last swatch of a row: the color picker for any color. It shows a color wheel until a custom color
 * is the current one, then that color, marked like a picked preset. `onOpen` runs as the picker opens,
 * for a menu to close.
 */
export function CustomSwatch({ current, presets = FOLDER_COLORS, onPick, onInput, onOpen, role = "radio" }: SwatchProps) {
  const custom = isCustomColor(current, presets);
  return (
    <button type="button" role={role} aria-checked={custom} aria-haspopup="dialog" aria-label={t("app.folder.customColor")} data-tip={t("app.folder.customColor")}
      className={custom ? "swatch custom on" : "swatch custom"} style={custom ? { background: current! } : undefined}
      onClick={(e) => { openColorPicker(e.currentTarget, { current, title: t("app.folder.customColor"), onPick, onInput }); onOpen?.(); }} />
  );
}

type Props = { title: string; confirm: string; name?: string; color?: string; onConfirm: (name: string, color: string) => void; onCancel: () => void };

export function FolderDialog({ title, confirm, name = "", color = FOLDER_COLORS[0], onConfirm, onCancel }: Props) {
  const [v, setV] = useState(name);
  const [c, setC] = useState(color);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    input.current?.select();
    const key = (e: KeyboardEvent) => e.key === "Escape" && onCancel();
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [onCancel]);
  return (
    <div className="modal-back" onPointerDown={(e) => e.target === e.currentTarget && onCancel()}>
      <form className="modal confirm" role="dialog" aria-modal="true" aria-label={title} style={{ borderTopColor: c }}
        onSubmit={(e) => { e.preventDefault(); if (v.trim()) onConfirm(v.trim(), c); }}>
        <h2 className="cap">{title}</h2>
        <label className="field">
          <span>{t("app.folder.name")}</span>
          <input ref={input} value={v} maxLength={40} onChange={(e) => setV(e.target.value)} spellCheck={false} />
        </label>
        <div className="field">
          <span>{t("app.folder.color")}</span>
          <div className="swatches" role="radiogroup" aria-label={t("app.folder.color")}>
            {FOLDER_COLORS.map((x) => (
              <button key={x} type="button" role="radio" aria-checked={x === c} aria-label={x} className={x === c ? "swatch on" : "swatch"} style={{ background: x }} onClick={() => setC(x)} />
            ))}
            <CustomSwatch current={c} onPick={setC} onInput={setC} />
          </div>
        </div>
        <div className="modal-foot">
          <button type="button" className="btn" onClick={onCancel}>{t("app.cancel")}</button>
          <button type="submit" className="btn play" disabled={!v.trim()}>{confirm}</button>
        </div>
      </form>
    </div>
  );
}

type DeleteProps = { name: string; color: string; count: number; onDelete: (removeMods: boolean) => void; onCancel: () => void };

export function DeleteFolder({ name, color, count, onDelete, onCancel }: DeleteProps) {
  const keep = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    keep.current?.focus();
    const key = (e: KeyboardEvent) => e.key === "Escape" && onCancel();
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [onCancel]);
  return (
    <div className="modal-back" onPointerDown={(e) => e.target === e.currentTarget && onCancel()}>
      <div className="modal confirm" role="alertdialog" aria-modal="true" aria-label={t("app.folder.deleteTitle", name)} style={{ borderTopColor: color }}>
        <h2 className="cap">{t("app.folder.deleteTitle", name)}</h2>
        <p>{count === 1 ? t("app.folder.deleteTextOne") : t("app.folder.deleteText", count)}</p>
        <div className="choices">
          <button className="btn play choice" ref={keep} onClick={() => onDelete(false)}>
            {t("app.folder.ungroup")}
            <small>{t("app.folder.ungroupHint")}</small>
          </button>
          <button className="btn danger choice" onClick={() => onDelete(true)}>
            {t("app.folder.deleteWithMods")}
            <small>{t("app.folder.deleteWithModsHint")}</small>
          </button>
        </div>
        <div className="modal-foot">
          <button className="btn" onClick={onCancel}>{t("app.cancel")}</button>
        </div>
      </div>
    </div>
  );
}
