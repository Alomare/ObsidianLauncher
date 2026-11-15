// The binds of one mod, to change: one row per action with its key, its controller input and how it
// fires (tap, double tap, press, long press, release). Every change applies at once, as in the drawer.
import { useEffect } from "react";
import { createPortal } from "react-dom";
import { run } from "../api";
import { CloseIcon, GamepadIcon, KeyboardMouseIcon, ResetIcon } from "../Icons";
import { pick, t } from "../i18n";
import { PadGlyph } from "../pad/PadGlyph";
import { Select } from "../Select";
import { INPUT_MODES, type KeyRow, type ModRow } from "../types";
import { KeyButton, PadControl, modeName, padOption, same } from "./BindControls";

export function BindsDialog({ mod, names, onClose }: { mod: ModRow; names: Map<string, string>; onClose: () => void }) {
  useEffect(() => {
    const key = (e: KeyboardEvent) => e.key === "Escape" && !e.defaultPrevented && onClose();
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [onClose]);
  const name = pick(mod.name, mod.id);
  const modes = INPUT_MODES.map((m) => ({ value: m, label: modeName(m) }));
  return createPortal(
    <div className="modal-back" onPointerDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal binds" role="dialog" aria-modal="true" aria-label={t("app.binds.title", name)}>
        <div className="head">
          <h2 className="cap">{t("app.binds.title", name)}</h2>
          <button data-pad-back className="bare close" onClick={onClose} aria-label={t("app.close")} data-tip={t("app.close")}><CloseIcon /><PadGlyph role="east" /></button>
        </div>
        <div className="binds-table" role="table" aria-label={t("app.binds.title", name)}>
          <div className="binds-row binds-head cap" role="row">
            <span role="columnheader">{t("app.binds.action")}</span>
            <span role="columnheader" aria-label={t("app.drawer.keyboard")} data-tip={t("app.drawer.keyboard")}><KeyboardMouseIcon /></span>
            <span role="columnheader" aria-label={t("app.drawer.controller")} data-tip={t("app.drawer.controller")}><GamepadIcon /></span>
            <span role="columnheader">{t("app.binds.mode")}</span>
          </div>
          {mod.keybinds.map((k: KeyRow) => {
            const label = pick(k.label, k.id);
            const pad = padOption(mod, k);
            return (
              <div key={k.id} className="binds-row" role="row">
                <div className="binds-name" role="rowheader">
                  <span>{label}</span>
                  {k.clash.length > 0 && <span className="warn">{t("app.drawer.keyClash", [name, ...k.clash.map((id) => names.get(id) ?? id)].join(", "))}</span>}
                </div>
                <div className="binds-cell" role="cell">
                  <KeyButton value={k.key ?? k.default} label={label} clash={k.clash.length > 0} onSet={(v) => run("set_key", { id: mod.id, keybind: k.id, key: v })} />
                  <button className="bare" disabled={k.key === null} onClick={() => run("set_key", { id: mod.id, keybind: k.id, key: null })}
                    aria-label={t("app.drawer.reset")} data-tip={t("app.drawer.resetKey", k.default)}><ResetIcon /></button>
                </div>
                <div className="binds-cell" role="cell">
                  {pad ? (
                    <>
                      <PadControl mod={mod} o={pad} />
                      <button className="bare" disabled={same(pad.value, pad.default)} onClick={() => run("set_option", { id: mod.id, option: pad.id, value: null })}
                        aria-label={t("app.drawer.reset")} data-tip={t("app.drawer.reset")}><ResetIcon /></button>
                    </>
                  ) : <span className="none" data-tip={t("app.drawer.noController")}>–</span>}
                </div>
                <div className="binds-cell" role="cell">
                  <Select value={k.mode} options={modes} label={t("app.binds.mode")} tip={t(`app.binds.modeHint.${k.mode}`)}
                    onChange={(m) => run("set_mode", { id: mod.id, keybind: k.id, mode: m === "press" ? null : m })} fill />
                </div>
              </div>
            );
          })}
        </div>
        <p className="binds-note muted">{t("app.binds.note")}</p>
      </div>
    </div>,
    document.body,
  );
}
