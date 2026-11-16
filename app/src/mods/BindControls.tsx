// The parts of a keybind row, shared by the Keys tab of the drawer (read only) and the binds dialog
// (where they change): the key, the controller input and how the bind fires.
import { useEffect, useRef, useState } from "react";
import { run } from "../api";
import { pick, t } from "../i18n";
import type { KeyRow, ModRow, OptionRow } from "../types";
import { keyName } from "./keyCapture";
import { PadGlyphs } from "./PadGlyphs";
import { padButtons, type PadButton } from "./padButtons";
import { recordPad, type EngineButton } from "../pad/pad";

export const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

/** The mouse buttons a bind can take, by MouseEvent.button. The left button stays for clicking. */
const MOUSE: Record<number, string> = { 1: "mouse_middle", 2: "mouse_right", 3: "mouse_extra_1", 4: "mouse_extra_2" };

/** A key as the Keys tab shows it: the name, or the mouse button in words. */
export const keyLabel = (k: string) => (k.startsWith("mouse_") ? t(`app.binds.${k}`) : k);

/** Shows a key; a click waits for the next key or mouse button (not the left one), Esc cancels. */
export function KeyButton({ value, label, onSet, clash }: { value: string; label: string; onSet: (k: string) => void; clash?: boolean }) {
  const [waiting, setWaiting] = useState(false);
  const take = (k: string) => {
    setWaiting(false);
    if (k !== value) onSet(k);
  };
  return (
    <button className={["key-btn", waiting && "waiting", clash && "clash"].filter(Boolean).join(" ")} aria-label={`${label}: ${keyLabel(value)}`}
      data-tip={waiting ? undefined : t("app.drawer.keyHint")}
      onClick={() => setWaiting(true)} onBlur={() => setWaiting(false)}
      onMouseDown={(e) => {
        if (!waiting || !MOUSE[e.button]) return;
        e.preventDefault();
        take(MOUSE[e.button]);
      }}
      onContextMenu={(e) => e.preventDefault()}
      onAuxClick={(e) => e.preventDefault()}
      onMouseUp={(e) => { if (e.button === 3 || e.button === 4) e.preventDefault(); }}
      onKeyDown={(e) => {
        if (!waiting) return;
        e.preventDefault();
        e.stopPropagation();
        if (e.key === "Escape") return setWaiting(false);
        const k = keyName(e.nativeEvent);
        if (k) take(k);
      }}>
      <span className="key-text">{waiting ? t("app.drawer.pressKey") : keyLabel(value)}</span>
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

const GLYPH: Record<EngineButton, PadButton> = {
  a: "a", b: "b", x: "x", y: "y", left_shoulder: "lb", right_shoulder: "rb", left_trigger: "lt", right_trigger: "rt",
  back: "back", start: "start", left_thumb: "l3", right_thumb: "r3", d_up: "dpad_up", d_down: "dpad_down", d_left: "dpad_left", d_right: "dpad_right",
};
const WORD: Record<PadButton, string> = {
  a: "A", b: "B", x: "X", y: "Y", lb: "LB", rb: "RB", lt: "LT", rt: "RT", l3: "L3", r3: "R3", back: "Back", start: "Start",
  dpad: "D-pad", dpad_up: "D-pad Up", dpad_down: "D-pad Down", dpad_left: "D-pad Left", dpad_right: "D-pad Right",
};

/** A combination as the loader reads it ("left_shoulder+d_up") as glyphs, with its name in words. */
function comboView(combo: string) {
  const buttons = combo.split("+").map((b) => GLYPH[b as EngineButton]).filter(Boolean);
  return { label: buttons.map((b) => WORD[b]).join(" + "), view: <PadGlyphs buttons={buttons} /> };
}

/** The controller input of a bind in effect: the one the player set, else the mod's own option. */
function padNow(mod: ModRow, k: KeyRow) {
  if (k.pad_bind) return comboView(k.pad_bind);
  const o = padOption(mod, k);
  return o ? padChoices(o).find((x) => x.value === String(o.value)) : undefined;
}

/** The controller input of a keybind as it is set, to read. */
export function PadView({ mod, k }: { mod: ModRow; k: KeyRow }) {
  const c = padNow(mod, k);
  if (!c) return <span className="none" data-tip={t("app.drawer.noController")}>–</span>;
  return <span className="pad-view" tabIndex={0} aria-label={c.label} data-tip={c.label}>{c.view ?? c.label}</span>;
}

/** The controller input of a keybind, to change: a click waits for buttons held together on the controller,
 *  set once all are let go. Esc, B alone, or a press elsewhere cancels: a controller click doesn't move the
 *  focus, so a press outside is watched for itself rather than through blur. */
export function PadButtonInput({ mod, k, label }: { mod: ModRow; k: KeyRow; label: string }) {
  const [waiting, setWaiting] = useState(false);
  const self = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!waiting) return;
    const stop = recordPad((buttons) => {
      setWaiting(false);
      const combo = buttons.join("+");
      if (combo !== k.pad_bind) run("set_pad", { id: mod.id, keybind: k.id, combo });
    }, () => setWaiting(false));
    const outside = (e: PointerEvent) => { if (!self.current?.contains(e.target as Node)) setWaiting(false); };
    window.addEventListener("pointerdown", outside, true);
    return () => { stop(); window.removeEventListener("pointerdown", outside, true); };
  }, [waiting, mod.id, k.id, k.pad_bind]);
  const c = padNow(mod, k);
  return (
    <button ref={self} className={waiting ? "key-btn pad-btn-in waiting" : "key-btn pad-btn-in"} aria-label={`${label}: ${c?.label ?? t("app.binds.padNone")}`}
      data-tip={waiting ? undefined : c ? `${c.label}. ${t("app.binds.padHint")}` : t("app.binds.padHint")}
      onClick={() => setWaiting(!waiting)} onBlur={() => setWaiting(false)}
      onKeyDown={(e) => { if (waiting && e.key === "Escape") { e.preventDefault(); e.stopPropagation(); setWaiting(false); } }}>
      <span className="key-text">{waiting ? t("app.binds.pressPad") : c ? (c.view ?? c.label) : t("app.binds.padNone")}</span>
    </button>
  );
}

/** The controller input of a keybind, when the manifest names one and it is shown. */
export function padOption(mod: ModRow, k: KeyRow): OptionRow | undefined {
  const o = k.pad ? mod.options.find((x) => x.id === k.pad) : undefined;
  return o?.visible ? o : undefined;
}

export const modeName = (mode: string) => t(`app.binds.mode.${mode}`);
