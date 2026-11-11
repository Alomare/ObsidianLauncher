// A styled dropdown in place of the native <select>: a button that opens a list. Arrow keys move,
// Enter or Space picks, Esc, a click outside, a scroll or a resize closes. The list sits on the
// body: inside a box with a folded corner, the corner's mask would cut it off at the edge of the box.
import { useEffect, useId, useLayoutEffect, useRef, useState, type CSSProperties, type KeyboardEvent, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { ChevronIcon } from "./Icons";
import { ScrollText } from "./ScrollText";

const GAP = 4;
const EDGE = 8;

type Props<T extends string> = {
  value: T;
  /** `view` draws the option in place of the label (a glyph, say); the label stays the name and shows beside it in the list. */
  options: { value: T; label: string; view?: ReactNode }[];
  onChange: (v: T) => void;
  /** Shown before the value inside the button, muted. */
  prefix?: string;
  label: string;
  tip?: string;
  /** Takes the whole width of the parent; a value too long for it scrolls. */
  fill?: boolean;
};

/** Under the button, or over it when the list doesn't fit there and there's more room above, and
 *  always inside the window. */
export function listPlace(
  button: { left: number; top: number; bottom: number; width: number },
  list: { width: number; height: number },
  win: { width: number; height: number },
): CSSProperties {
  const below = win.height - button.bottom - GAP - EDGE;
  const above = button.top - GAP - EDGE;
  const up = list.height > below && above > below;
  const left = Math.max(EDGE, Math.min(button.left, win.width - Math.max(list.width, button.width) - EDGE));
  return {
    left,
    minWidth: button.width,
    maxHeight: Math.min(list.height, up ? above : below),
    ...(up ? { bottom: win.height - button.top + GAP } : { top: button.bottom + GAP }),
  };
}

export function Select<T extends string>({ value, options, onChange, prefix, label, tip, fill }: Props<T>) {
  const [open, setOpen] = useState(false);
  const [hi, setHi] = useState(0);
  const [place, setPlace] = useState<CSSProperties>({ visibility: "hidden" });
  const root = useRef<HTMLDivElement>(null);
  const box = useRef<HTMLUListElement>(null);
  const list = useId();
  const current = options.find((o) => o.value === value);

  useLayoutEffect(() => {
    if (!open) return;
    const size = box.current!.getBoundingClientRect();
    setPlace(listPlace(root.current!.getBoundingClientRect(), size, { width: innerWidth, height: innerHeight }));
    return () => setPlace({ visibility: "hidden" });
  }, [open]);

  useEffect(() => {
    if (!open) return;
    setHi(Math.max(0, options.findIndex((o) => o.value === value)));
    const inList = (e: Event) => box.current?.contains(e.target as Node);
    const away = (e: PointerEvent) => !root.current?.contains(e.target as Node) && !inList(e) && setOpen(false);
    const moved = (e: Event) => !inList(e) && setOpen(false);
    const resized = () => setOpen(false);
    document.addEventListener("pointerdown", away, true);
    document.addEventListener("scroll", moved, true);
    window.addEventListener("resize", resized);
    return () => {
      document.removeEventListener("pointerdown", away, true);
      document.removeEventListener("scroll", moved, true);
      window.removeEventListener("resize", resized);
    };
  }, [open, options, value]);

  const pick = (v: T) => {
    setOpen(false);
    if (v !== value) onChange(v);
  };
  const key = (e: KeyboardEvent) => {
    if (e.key === "Escape" && open) {
      e.stopPropagation();
      setOpen(false);
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      if (!open) return setOpen(true);
      setHi((h) => (h + (e.key === "ArrowDown" ? 1 : -1) + options.length) % options.length);
    } else if ((e.key === "Enter" || e.key === " ") && open) {
      e.preventDefault();
      pick(options[hi].value);
    }
  };

  return (
    <div className={["select", open && "open", fill && "fill"].filter(Boolean).join(" ")} ref={root} onKeyDown={key}>
      <button type="button" className="select-btn" aria-haspopup="listbox" aria-expanded={open} aria-controls={list}
        aria-label={`${label}: ${current?.label ?? ""}`} data-tip-title={open || !tip ? undefined : label} data-tip={open ? undefined : tip} onClick={() => setOpen((o) => !o)}>
        {prefix && <span className="select-prefix">{prefix}</span>}
        {/* Every label sits in the same grid cell, so the button keeps the width of the widest. */}
        <span className="select-value">
          {fill && !current?.view ? <ScrollText text={current?.label ?? ""} /> : options.map((o) => <span key={o.value} aria-hidden={o.value !== value || undefined}>{o.view ?? o.label}</span>)}
        </span>
        <ChevronIcon />
      </button>
      {open && createPortal(
        <ul className="select-list" ref={box} role="listbox" id={list} aria-label={label} style={place}>
          {options.map((o, i) => (
            <li key={o.value} role="option" aria-selected={o.value === value} className={i === hi ? "hi" : undefined}
              onPointerEnter={() => setHi(i)} onClick={() => pick(o.value)}>
              {o.view ? <><span className="select-view">{o.view}</span>{o.label}</> : o.label}
            </li>
          ))}
        </ul>,
        document.body,
      )}
    </div>
  );
}
