// A right-click menu, and the menu of a button. Items run and close it; an item with `sub` opens a menu to
// the side on hover or Enter, and `colors` is a row of swatches to pick from, ending in one for a custom
// color. `checked` makes an item one of a set of choices, with the chosen one marked, and `extra` adds icons
// at the end of its row. Arrows, Home and End move; Esc, Tab, a click outside, a scroll or leaving the
// window closes it. It sits on the body, so a folded corner's mask can't cut it off.
// `up` hangs it above y, for a button low in the window. With `opener`, the button it came from, a click on
// that button is left to the button (to close it), and the focus goes back there when the menu closes.
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { ChevronIcon } from "./Icons";
import { CustomSwatch } from "./mods/FolderDialog";

export type MenuItem =
  | { label: string; icon?: ReactNode; danger?: boolean; disabled?: boolean; hint?: string; run?: () => void; sub?: MenuItem[]; swatch?: string; checked?: boolean; extra?: ReactNode }
  | { colors: string[]; current: string | null; pick: (color: string) => void }
  | "sep";

type Props = { x: number; y: number; up?: boolean; items: MenuItem[]; onClose: () => void; opener?: HTMLElement | null; label?: string };

export function ContextMenu({ x, y, up, items, onClose, opener, label }: Props) {
  const close = useRef(onClose);
  close.current = onClose;
  useEffect(() => {
    const away = (e: PointerEvent) => {
      const at = e.target as Element;
      if (!at.closest?.(".ctx") && !(opener && opener.contains(at))) close.current();
    };
    const key = (e: KeyboardEvent) => e.key === "Escape" && close.current();
    const gone = () => close.current();
    document.addEventListener("pointerdown", away, true);
    window.addEventListener("keydown", key);
    window.addEventListener("blur", gone);
    window.addEventListener("resize", gone);
    document.addEventListener("scroll", gone, true);
    return () => {
      document.removeEventListener("pointerdown", away, true);
      window.removeEventListener("keydown", key);
      window.removeEventListener("blur", gone);
      window.removeEventListener("resize", gone);
      document.removeEventListener("scroll", gone, true);
      // The focus was in the menu, which is gone now: back to the button, unless something else took it.
      if (opener?.isConnected && (!document.activeElement || document.activeElement === document.body)) opener.focus({ preventScroll: true });
    };
  }, [opener]);
  return createPortal(<Menu x={x} y={y} up={up} items={items} onClose={onClose} label={label} root />, document.body);
}

function Menu({ x, y, up, items, onClose, root, keys, label }: { x: number; y: number; up?: boolean; items: MenuItem[]; onClose: () => void; root?: boolean; keys?: boolean; label?: string }) {
  const box = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x, y });
  const [open, setOpen] = useState<{ i: number; x: number; y: number; keys?: boolean } | null>(null);
  // Kept inside the window: a menu near the right or bottom edge opens the other way.
  useLayoutEffect(() => {
    const r = box.current!.getBoundingClientRect();
    const nx = x + r.width > innerWidth - 4 ? (root ? x - r.width : x - r.width * 2 - 4) : x;
    const top = up ? y - r.height : y;
    const ny = top + r.height > innerHeight - 4 ? innerHeight - r.height - 4 : top;
    setPos({ x: Math.max(4, nx), y: Math.max(4, ny) });
    if (root || keys) box.current!.querySelector<HTMLElement>("button:not(:disabled)")?.focus();
  }, [x, y, up, root, keys]);

  const openSub = (i: number, el: HTMLElement, keys?: boolean) => {
    const r = el.getBoundingClientRect();
    setOpen({ i, x: r.right + 2, y: r.top - 5, keys });
  };
  const step = (all: HTMLElement[], from: Element | null | undefined, forward: boolean) => {
    const at = all.indexOf(from as HTMLElement);
    all[(at + (forward ? 1 : all.length - 1)) % all.length]?.focus();
  };
  const key = (e: React.KeyboardEvent) => {
    // A row of colors is one stop going up and down; left and right move along it.
    const row = (e.target as Element).closest(".ctx-colors");
    const stops = () => [...box.current!.querySelectorAll<HTMLElement>(":scope > button:not(:disabled), .ctx-colors > :first-child")];
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      step(stops(), row ? row.firstElementChild : document.activeElement, e.key === "ArrowDown");
    } else if (e.key === "Home" || e.key === "End") {
      e.preventDefault();
      const all = stops();
      all[e.key === "Home" ? 0 : all.length - 1]?.focus();
    } else if (e.key === "Tab") {
      e.preventDefault();
      onClose();
    } else if (row && (e.key === "ArrowLeft" || e.key === "ArrowRight")) {
      e.preventDefault();
      step([...row.children] as HTMLElement[], document.activeElement, e.key === "ArrowRight");
    }
  };
  return (
    <>
      <div ref={box} className="ctx" role="menu" aria-label={label} style={{ left: pos.x, top: pos.y }} onKeyDown={key} onContextMenu={(e) => e.preventDefault()}>
        {items.map((it, i) =>
          it === "sep" ? (
            <hr key={i} />
          ) : "colors" in it ? (
            <div key={i} className="ctx-colors" role="group">
              {it.colors.map((c) => (
                <button key={c} role="menuitemradio" aria-checked={c === it.current} aria-label={c} className={c === it.current ? "swatch on" : "swatch"} style={{ background: c }}
                  onClick={() => {
                    onClose();
                    it.pick(c);
                  }} />
              ))}
              {/* The picker outlives the menu, which closes as the picker opens. */}
              <CustomSwatch role="menuitemradio" current={it.current} presets={it.colors} onOpen={onClose} onPick={it.pick} />
            </div>
          ) : (
            <button key={i} role={it.checked === undefined ? "menuitem" : "menuitemradio"} aria-checked={it.checked} className={it.danger ? "danger" : undefined}
              disabled={it.disabled} aria-haspopup={it.sub ? "menu" : undefined}
              aria-expanded={it.sub ? open?.i === i : undefined}
              onPointerEnter={(e) => (it.sub ? openSub(i, e.currentTarget) : setOpen(null))}
              onKeyDown={(e) => it.sub && (e.key === "ArrowRight" || e.key === "Enter") && (e.preventDefault(), openSub(i, e.currentTarget, true))}
              onClick={(e) => {
                if (it.sub) return openSub(i, e.currentTarget);
                onClose();
                it.run?.();
              }}>
              <span className="ctx-ico">{it.swatch ? <i className="ctx-swatch" style={{ background: it.swatch }} /> : it.icon}</span>
              <span className="ctx-label">{it.label}</span>
              {it.extra}
              {it.hint && <span className="ctx-hint">{it.hint}</span>}
              {it.sub && <span className="ctx-more"><ChevronIcon /></span>}
            </button>
          ),
        )}
      </div>
      {open && (items[open.i] as { sub?: MenuItem[] }).sub && <Menu x={open.x} y={open.y} items={(items[open.i] as { sub: MenuItem[] }).sub} onClose={onClose} keys={open.keys} />}
    </>
  );
}
