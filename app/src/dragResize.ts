// The drag of a resize divider, shared by the Mods and Saves drawers and the Wiki list. The pointer is
// captured on the divider, so the drag follows it past the window edge; it ends on release and on a
// cancel too (a touch taken by the system, a window switch), where the width reached is kept.
import type { PointerEvent } from "react";

/** Starts a drag from `e`: each move sets the width `widthAt` gives for the pointer's x, and the end
 *  hands the last width to `done` (to keep it between visits). A press without a move changes nothing,
 *  so the clicks of a double-click (which resets the width) don't keep one. */
export function dragResize(e: PointerEvent<HTMLElement>, widthAt: (x: number) => number, set: (w: number) => void, done: (w: number) => void) {
  const el = e.currentTarget;
  el.setPointerCapture?.(e.pointerId);
  let last: number | null = null;
  const move = (m: globalThis.PointerEvent) => {
    last = widthAt(m.clientX);
    set(last);
  };
  const end = () => {
    el.removeEventListener("pointermove", move);
    el.removeEventListener("pointerup", end);
    el.removeEventListener("pointercancel", end);
    if (last !== null) done(last);
  };
  el.addEventListener("pointermove", move);
  el.addEventListener("pointerup", end);
  el.addEventListener("pointercancel", end);
}

/** Keeps a width in the browser storage of this window; a failure only loses it for the next visit. */
export function keepWidth(key: string, w: number) {
  try { localStorage.setItem(key, String(w)); } catch { /* the width is a convenience */ }
}
