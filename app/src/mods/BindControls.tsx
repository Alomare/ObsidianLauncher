// The parts of a keybind row, shared by the Keys tab of the drawer (read only) and the binds dialog
// (where they change): the key, the controller input and how the bind fires.
import { useState } from "react";
import { run } from "../api";
import { pick, t } from "../i18n";
import { Select } from "../Select";
import type { KeyRow, ModRow, OptionRow } from "../types";
import { keyName } from "./keyCapture";
import { PadGlyphs } from "./PadGlyphs";
import { padButtons } from "./padButtons";

export const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

/** Shows a key; a click waits for the next key press, Esc cancels. */
export function KeyButton({ value, label, onSet, clash }: { value: string; label: string; onSet: (k: string) => void; clash?: boolean }) {
  const [waiting, setWaiting] = useState(false);
  return (
    <button className={["key-btn", waiting && "waiting", clash && "clash"].filter(Boolean).join(" ")} aria-label={`${label}: ${value}`}
      data-tip={waiting ? undefined : t("app.drawer.keyHint")}
      onClick={() => setWaiting(true)} onBlur={() => setWaiting(false)}
      onKeyDown={(e) => {
        if (!waiting) return;
        e.preventDefault();
        e.stopPropagation();
        if (e.key === "Escape") return setWaiting(false);
        const k = keyName(e.nativeEvent);
        if (k) {
          setWaiting(false);
          if (k !== value) onSet(k);
        }
      }}>
      <span className="key-text">{waiting ? t("app.drawer.pressKey") : value}</span>
    </button>
  );
}

/** The choices of a controller input, each drawn as its buttons when the name says which. */
function padChoices(o: OptionRow) {
  return o.choices.map((c, i) => {
    const name = pick(o.choiceLabels[i], c);
    const buttons = padButtons(name, c);
    return { value: c, label: name, view: buttons ? <PadGlyphs buttons={buttons} /> : undefined };
  });
}

/** The controller input of a keybind: its choice option, narrowed to the column, the whole choice in a tooltip. */
export function PadControl({ mod, o }: { mod: ModRow; o: OptionRow }) {
  const options = padChoices(o);
  const current = options.find((c) => c.value === String(o.value))?.label;
  return <Select value={String(o.value)} options={options} label={pick(o.label, o.id)} tip={current} onChange={(value) => run("set_option", { id: mod.id, option: o.id, value })} />;
}

/** The controller input of a keybind as it is set, to read. */
export function PadView({ o }: { o: OptionRow }) {
  const c = padChoices(o).find((x) => x.value === String(o.value));
  if (!c) return <span className="none">–</span>;
  return <span className="pad-view" tabIndex={0} aria-label={c.label} data-tip={c.label}>{c.view ?? c.label}</span>;
}

/** The controller input of a keybind, when the manifest names one and it is shown. */
export function padOption(mod: ModRow, k: KeyRow): OptionRow | undefined {
  const o = k.pad ? mod.options.find((x) => x.id === k.pad) : undefined;
  return o?.visible ? o : undefined;
}

export const modeName = (mode: string) => t(`app.binds.mode.${mode}`);
