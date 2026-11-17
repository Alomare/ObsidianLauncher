// The shell of every dialog: on the body (a folded corner's mask can't cut it off), over a dark back layer,
// closed by Esc, by a press on the back layer and by the controller's east button (the `data-pad-back`
// button inside, pad.ts). The focus comes in once when it opens and stays inside while it is open.
//
// Dialogs and menus stack. Esc goes to the one on top only, so a question over a dialog closes alone, and
// a dialog under a menu stays. `useLayer` puts anything that closes on Esc (a menu, a popup) on that stack.
// The callbacks are read when needed, never as effect dependencies: a parent that renders again (a new
// snapshot) can't move the focus back to the default button.
import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type FormEvent, type ReactNode, type RefObject } from "react";
import { createPortal } from "react-dom";

type Layer = { order: number; escape: RefObject<(() => void) | null>; box: RefObject<HTMLElement | null> | null };

const layers: Layer[] = [];
let opened = 0;

const FOCUSABLE = "button:not(:disabled), [href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex='-1'])";
/** Popups that sit on the body beside a dialog (a list, a menu, the color picker): Tab there is theirs. */
const POPUPS = ".ctx, .select-list, .picker";

const top = () => layers.reduce<Layer | null>((a, l) => (!a || l.order > a.order ? l : a), null);

function onKey(e: KeyboardEvent) {
  const layer = top();
  if (!layer) return;
  if (e.key === "Escape") {
    if (e.defaultPrevented || e.isComposing) return;
    e.preventDefault();
    layer.escape.current?.();
  } else if (e.key === "Tab") {
    trap(e, layer.box?.current ?? null);
  }
}

/** Keeps Tab inside the dialog on top: past the last control it comes back to the first, and the other way. */
function trap(e: KeyboardEvent, box: HTMLElement | null) {
  if (!box) return;
  const at = document.activeElement as HTMLElement | null;
  if (at && at !== document.body && !box.contains(at) && at.closest(POPUPS)) return;
  const all = [...box.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((el) => !el.closest("[inert]") && !el.hidden);
  if (!all.length) { e.preventDefault(); box.focus(); return; }
  const first = all[0], last = all[all.length - 1];
  const inside = !!at && box.contains(at);
  if (!inside || at === box) { e.preventDefault(); (e.shiftKey ? last : first).focus(); return; }
  if (!e.shiftKey && at === last) { e.preventDefault(); first.focus(); }
  else if (e.shiftKey && at === first) { e.preventDefault(); last.focus(); }
}

/** Puts a dialog, menu or popup on the stack while it is mounted: Esc on top of the stack runs `onEscape`,
 *  and with `box`, Tab stays inside it. Only the newest layer hears Esc. */
export function useLayer(onEscape: () => void, box: RefObject<HTMLElement | null> | null = null) {
  // Numbered as it first renders: a parent renders before its children, so a question opened inside a
  // dialog in the same pass still sits on top of it.
  const [order] = useState(() => ++opened);
  const escape = useRef<(() => void) | null>(onEscape);
  escape.current = onEscape;
  useLayoutEffect(() => {
    const layer: Layer = { order, escape, box };
    if (!layers.length) window.addEventListener("keydown", onKey);
    layers.push(layer);
    return () => {
      layers.splice(layers.indexOf(layer), 1);
      if (!layers.length) window.removeEventListener("keydown", onKey);
    };
  }, [order]); // eslint-disable-line react-hooks/exhaustive-deps
}

type Props = {
  /** Esc, a press outside the dialog, and the close buttons. */
  onClose: () => void;
  label: string;
  /** The classes of the dialog box ("modal confirm"). */
  className: string;
  /** More classes for the back layer. */
  backClass?: string;
  role?: "dialog" | "alertdialog" | "status";
  /** A form box: Enter in a field submits. */
  onSubmit?: (e: FormEvent<HTMLFormElement>) => void;
  style?: CSSProperties;
  /** What takes the focus when it opens; a text box gets its text selected. Else the dialog box itself. */
  focus?: RefObject<HTMLElement | null>;
  /** False while a press outside must not close it (a form being filled, a step under way). */
  outside?: boolean;
  /** False while Esc must not close it. It still takes the Esc, so the dialog under it stays open. */
  escape?: boolean;
  children: ReactNode;
};

export function Modal({ onClose, label, className, backClass, role = "dialog", onSubmit, style, focus, outside = true, escape = true, children }: Props) {
  const box = useRef<HTMLElement | null>(null);
  const close = useRef(onClose);
  close.current = onClose;
  const allow = useRef({ outside, escape });
  allow.current = { outside, escape };
  useLayer(() => { if (allow.current.escape) close.current(); }, role === "status" ? null : box);
  // Once, as it opens: never again when the parent renders.
  useEffect(() => {
    const el = focus?.current ?? box.current;
    if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) el.select();
    else el?.focus();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps
  const common = {
    className, role, style, "aria-modal": role === "status" ? undefined : true, "aria-label": label, tabIndex: -1,
  } as const;
  return createPortal(
    <div className={backClass ? `modal-back ${backClass}` : "modal-back"} onPointerDown={(e) => { if (e.target === e.currentTarget && allow.current.outside) close.current(); }}>
      {onSubmit
        ? <form {...common} ref={box as RefObject<HTMLFormElement>} onSubmit={onSubmit}>{children}</form>
        : <div {...common} ref={box as RefObject<HTMLDivElement>}>{children}</div>}
    </div>,
    document.body,
  );
}
