// One tooltip for the whole window: any element with `data-tip` (the explanation) and/or
// `data-tip-title` (the name, in bold above it) and/or `data-tip-note` (a line set apart under the
// explanation, in the accent color) shows it on hover after a short wait and on keyboard
// focus. Mounted once in App. It goes above the element, or below when it doesn't fit there;
// `data-tip-side="bottom"` puts it below by choice.
import { useEffect, useLayoutEffect, useRef, useState } from "react";

const WAIT_MS = 450;
const GAP_PX = 8;
const EDGE_PX = 8;

type Size = { width: number; height: number };
type Tip = { title?: string; text?: string; note?: string; anchor: DOMRect; bottom: boolean };

/** Where a tooltip of this size goes: centred on the element, on the side it fits, inside the window. */
export function tipPlace(anchor: Pick<DOMRect, "left" | "right" | "top" | "bottom">, tip: Size, win: Size, bottom: boolean) {
  const above = anchor.top - GAP_PX - tip.height;
  const under = anchor.bottom + GAP_PX;
  const below = bottom ? under + tip.height <= win.height - EDGE_PX : above < EDGE_PX;
  return {
    x: Math.max(EDGE_PX, Math.min((anchor.left + anchor.right - tip.width) / 2, win.width - tip.width - EDGE_PX)),
    y: Math.max(EDGE_PX, Math.min(below ? under : above, win.height - tip.height - EDGE_PX)),
    below,
  };
}

export function Tooltip() {
  const [tip, setTip] = useState<Tip | null>(null);
  const box = useRef<HTMLDivElement>(null);
  const [place, setPlace] = useState<ReturnType<typeof tipPlace> | null>(null);

  useEffect(() => {
    let timer = 0;
    let target: HTMLElement | null = null;
    const show = (el: HTMLElement) => {
      const { tip: text, tipTitle: title, tipNote: note } = el.dataset;
      if (!text && !title && !note) return;
      setTip({ title, text, note, anchor: el.getBoundingClientRect(), bottom: el.dataset.tipSide === "bottom" });
    };
    const hide = () => {
      clearTimeout(timer);
      target = null;
      setTip(null);
    };
    const over = (e: Event) => {
      const el = (e.target as Element).closest?.<HTMLElement>("[data-tip], [data-tip-title], [data-tip-note]") ?? null;
      if (el === target) return;
      // A click also focuses the button; only keyboard focus shows the tooltip.
      if (e.type === "focusin" && el && !el.matches(":focus-visible")) return;
      hide();
      if (!el) return;
      target = el;
      timer = window.setTimeout(() => show(el), e.type === "focusin" ? 0 : WAIT_MS);
    };
    document.addEventListener("pointerover", over);
    document.addEventListener("focusin", over);
    document.addEventListener("pointerdown", hide, true);
    document.addEventListener("keydown", hide, true);
    window.addEventListener("blur", hide);
    return () => {
      document.removeEventListener("pointerover", over);
      document.removeEventListener("focusin", over);
      document.removeEventListener("pointerdown", hide, true);
      document.removeEventListener("keydown", hide, true);
      window.removeEventListener("blur", hide);
    };
  }, []);

  // Its size is known only once it's on the page, and this runs before that is painted.
  useLayoutEffect(() => {
    const el = box.current;
    setPlace(el && tip ? tipPlace(tip.anchor, el.getBoundingClientRect(), { width: innerWidth, height: innerHeight }, tip.bottom) : null);
  }, [tip]);

  if (!tip) return null;
  return (
    <div className="tip-layer">
      <div ref={box} role="tooltip" className={place?.below ? "tip below" : "tip"}
        style={place ? { left: place.x, top: place.y } : { left: 0, top: 0, visibility: "hidden" }}>
        {tip.title && <b className="tip-title">{tip.title}</b>}
        {tip.text && <span>{tip.text}</span>}
        {tip.note && <span className="tip-note">{tip.note}</span>}
      </div>
    </div>
  );
}
