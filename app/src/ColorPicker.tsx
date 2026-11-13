// The color picker: a field of saturation and brightness, a hue strip, the hex code, the color before and
// after, and Cancel or Apply. It opens beside the control that asked for it (above it when there is no room
// below) and lives on its own, as the color menus close when it opens; ColorPickerHost in App shows it.
// The arrow keys move the field and the strip, Shift in bigger steps; Enter applies; Esc or a click outside
// cancels. `onInput` follows every change and gets the first color back on Cancel.
import { useEffect, useLayoutEffect, useRef, useState, useSyncExternalStore, type CSSProperties, type KeyboardEvent, type PointerEvent } from "react";
import { createPortal } from "react-dom";
import { t } from "./i18n";

export type Hsv = { h: number; s: number; v: number };

/** `#rrggbb` from `#rrggbb`, `rrggbb`, `#rgb` or `rgb`, in lower case; null for anything else. */
export function parseHex(text: string | null | undefined): string | null {
  const m = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.exec((text ?? "").trim());
  if (!m) return null;
  const d = m[1].length === 3 ? [...m[1]].map((c) => c + c).join("") : m[1];
  return `#${d.toLowerCase()}`;
}

export function hexToHsv(hex: string): Hsv {
  const n = parseInt(hex.slice(1), 16);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((c) => c / 255);
  const max = Math.max(r, g, b), d = max - Math.min(r, g, b);
  let h = 0;
  if (d) h = max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  return { h: (h * 60 + 360) % 360, s: max ? d / max : 0, v: max };
}

export function hsvToHex({ h, s, v }: Hsv): string {
  const f = (n: number) => {
    const k = (n + h / 60) % 6;
    return Math.round((v - v * s * Math.max(0, Math.min(k, 4 - k, 1))) * 255).toString(16).padStart(2, "0");
  };
  return `#${f(5)}${f(3)}${f(1)}`;
}

const FALLBACK = "#f0b429";
const clamp = (n: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, n));

type Request = { anchor: Element; rect: DOMRect; start: string; title: string; onPick: (color: string) => void; onInput?: (color: string) => void; n: number };
let open: Request | null = null;
let count = 0;
const listeners = new Set<() => void>();
const emit = () => listeners.forEach((f) => f());

/** Opens the picker at `anchor`, on `current` (the default gold when there is none). A picker already open is cancelled. */
export function openColorPicker(anchor: Element, o: { current: string | null; title: string; onPick: (color: string) => void; onInput?: (color: string) => void }) {
  open?.onInput?.(open.start);
  open = { anchor, rect: anchor.getBoundingClientRect(), start: parseHex(o.current) ?? FALLBACK, title: o.title, onPick: o.onPick, onInput: o.onInput, n: ++count };
  emit();
}

export function ColorPickerHost() {
  const req = useSyncExternalStore((f) => { listeners.add(f); return () => { listeners.delete(f); }; }, () => open);
  return req ? <ColorPicker key={req.n} req={req} /> : null;
}

function ColorPicker({ req }: { req: Request }) {
  const [hsv, setHsv] = useState<Hsv>(() => hexToHsv(req.start));
  const [text, setText] = useState(req.start);
  const [pos, setPos] = useState<{ x: number; y: number } | null>(null);
  const box = useRef<HTMLDivElement>(null);
  const area = useRef<HTMLDivElement>(null);
  const strip = useRef<HTMLDivElement>(null);
  const dragging = useRef<"area" | "hue" | null>(null);
  const hex = hsvToHex(hsv);
  const now = useRef({ hsv, hex });
  now.current = { hsv, hex };

  const change = (next: Hsv, typed = false) => {
    setHsv(next);
    const h = hsvToHex(next);
    if (!typed) setText(h);
    req.onInput?.(h);
  };
  /** Applies `color`, or with none cancels; the focus goes back to the control that opened the picker. */
  const finish = useRef((color?: string) => {
    if (open !== req) return;
    if (color) req.onPick(color);
    else req.onInput?.(req.start);
    open = null;
    emit();
    if (req.anchor.isConnected && req.anchor instanceof HTMLElement) req.anchor.focus({ preventScroll: true });
  }).current;

  // Beside the control: below it and from its left edge, else above it, and always inside the window.
  useLayoutEffect(() => {
    const r = box.current!.getBoundingClientRect(), a = req.rect;
    const x = a.left + r.width > innerWidth - 4 ? innerWidth - 4 - r.width : a.left;
    const y = a.bottom + 6 + r.height > innerHeight - 4 ? a.top - 6 - r.height : a.bottom + 6;
    setPos({ x: Math.max(4, x), y: Math.max(4, y) });
  }, [req]);
  // Once it shows: hidden while it measures itself, it can't take the focus.
  const placed = pos !== null;
  useLayoutEffect(() => {
    if (placed) area.current?.focus({ preventScroll: true });
  }, [placed]);

  // Ahead of every other key handler, so Esc closes only the picker and not the dialog under it.
  useEffect(() => {
    const away = (e: globalThis.PointerEvent) => !box.current?.contains(e.target as Node) && finish();
    const key = (e: globalThis.KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        finish();
      } else if (e.key === "Enter" && !e.isComposing && !(e.target instanceof HTMLButtonElement) && box.current?.contains(e.target as Node)) {
        e.preventDefault();
        e.stopPropagation();
        finish(now.current.hex);
      }
    };
    const gone = () => finish();
    document.addEventListener("pointerdown", away, true);
    window.addEventListener("keydown", key, true);
    window.addEventListener("resize", gone);
    return () => {
      document.removeEventListener("pointerdown", away, true);
      window.removeEventListener("keydown", key, true);
      window.removeEventListener("resize", gone);
    };
  }, [finish]);

  const at = (el: HTMLElement | null, x: number, y: number) => {
    const r = el!.getBoundingClientRect();
    return { fx: r.width ? clamp((x - r.left) / r.width, 0, 1) : 0, fy: r.height ? clamp((y - r.top) / r.height, 0, 1) : 0 };
  };
  const follow = (which: "area" | "hue", x: number, y: number) => {
    const cur = now.current.hsv;
    if (which === "area") {
      const { fx, fy } = at(area.current, x, y);
      change({ ...cur, s: fx, v: 1 - fy });
    } else {
      change({ ...cur, h: at(strip.current, x, y).fx * 360 });
    }
  };
  const pointer = (which: "area" | "hue") => ({
    onPointerDown: (e: PointerEvent<HTMLDivElement>) => {
      if (e.button !== 0) return;
      dragging.current = which;
      e.currentTarget.setPointerCapture?.(e.pointerId);
      e.currentTarget.focus({ preventScroll: true });
      follow(which, e.clientX, e.clientY);
    },
    onPointerMove: (e: PointerEvent<HTMLDivElement>) => dragging.current === which && follow(which, e.clientX, e.clientY),
    onPointerUp: () => { dragging.current = null; },
    onPointerCancel: () => { dragging.current = null; },
  });
  const areaKey = (e: KeyboardEvent<HTMLDivElement>) => {
    const d = e.shiftKey ? 0.1 : 0.01;
    const move: Record<string, Partial<Hsv>> = { ArrowLeft: { s: hsv.s - d }, ArrowRight: { s: hsv.s + d }, ArrowUp: { v: hsv.v + d }, ArrowDown: { v: hsv.v - d } };
    const m = move[e.key];
    if (!m) return;
    e.preventDefault();
    const n = { ...hsv, ...m };
    change({ ...n, s: clamp(n.s, 0, 1), v: clamp(n.v, 0, 1) });
  };
  const hueKey = (e: KeyboardEvent<HTMLDivElement>) => {
    const d = e.shiftKey ? 10 : 1;
    const to: Record<string, number> = { ArrowLeft: hsv.h - d, ArrowDown: hsv.h - d, ArrowRight: hsv.h + d, ArrowUp: hsv.h + d, Home: 0, End: 359 };
    if (!(e.key in to)) return;
    e.preventDefault();
    change({ ...hsv, h: clamp(to[e.key], 0, 359) });
  };
  // Tab goes round the picker's own controls while it is open.
  const trap = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key !== "Tab") return;
    const stops = [...box.current!.querySelectorAll<HTMLElement>("[tabindex='0'], input, button")];
    const i = stops.indexOf(document.activeElement as HTMLElement);
    if (e.shiftKey && i <= 0) { e.preventDefault(); stops[stops.length - 1].focus(); }
    else if (!e.shiftKey && i === stops.length - 1) { e.preventDefault(); stops[0].focus(); }
  };
  const typed = (value: string) => {
    setText(value);
    const h = parseHex(value);
    if (!h) return;
    const n = hexToHsv(h);
    // A grey has no hue of its own: the strip stays where it was.
    change(n.s === 0 ? { ...n, h: hsv.h } : n, true);
  };
  const s = Math.round(hsv.s * 100), v = Math.round(hsv.v * 100), h = Math.round(hsv.h);

  return createPortal(
    <div ref={box} className="picker" role="dialog" aria-label={req.title} onKeyDown={trap}
      style={pos ? { left: pos.x, top: pos.y } : { left: 0, top: 0, visibility: "hidden" }}>
      <h3 className="picker-title cap">{req.title}</h3>
      <div ref={area} className="picker-area" role="slider" tabIndex={0} aria-label={t("app.color.area")} aria-valuemin={0} aria-valuemax={100} aria-valuenow={s}
        aria-valuetext={t("app.color.areaValue", s, v)} style={{ "--hue": `hsl(${h} 100% 50%)` } as CSSProperties} onKeyDown={areaKey} {...pointer("area")}>
        <i className="picker-dot" style={{ left: `${hsv.s * 100}%`, top: `${(1 - hsv.v) * 100}%`, background: hex }} />
      </div>
      <div ref={strip} className="picker-hue" role="slider" tabIndex={0} aria-label={t("app.color.hue")} aria-valuemin={0} aria-valuemax={359} aria-valuenow={h}
        aria-valuetext={t("app.color.hueValue", h)} onKeyDown={hueKey} {...pointer("hue")}>
        <i className="picker-bar" style={{ left: `${(hsv.h / 360) * 100}%` }} />
      </div>
      <div className="picker-row">
        <span className="picker-preview" aria-hidden="true"><i style={{ background: req.start }} /><i style={{ background: hex }} /></span>
        <input className="picker-hex mono" value={text} maxLength={7} spellCheck={false} aria-label={t("app.color.hex")}
          onChange={(e) => typed(e.target.value)} onBlur={() => setText(now.current.hex)} />
      </div>
      <div className="picker-foot">
        <button className="btn small" onClick={() => finish()}>{t("app.cancel")}</button>
        <button className="btn small play" onClick={() => finish(hex)}>{t("app.color.apply")}</button>
      </div>
    </div>,
    document.body,
  );
}
