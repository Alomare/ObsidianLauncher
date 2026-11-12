// One line of text in a box of set width. Text too long for the box slides to the end and back; with
// animations off it ends in an ellipsis (styles.css, .scroll-text).
import { useLayoutEffect, useRef, useState, type CSSProperties } from "react";

/** Pixels per second the text moves at, and the shortest trip, so a few pixels of overflow don't jitter. */
const SPEED = 30;
const MIN_SECONDS = 3;

export function ScrollText({ text }: { text: string }) {
  const box = useRef<HTMLSpanElement>(null);
  const line = useRef<HTMLSpanElement>(null);
  const [over, setOver] = useState(0);
  useLayoutEffect(() => {
    const b = box.current!;
    const l = line.current!;
    const measure = () => setOver(Math.max(0, l.offsetWidth - b.clientWidth));
    measure();
    if (typeof ResizeObserver === "undefined") return;
    // The line resizes too when the font arrives.
    const watch = new ResizeObserver(measure);
    watch.observe(b);
    watch.observe(l);
    return () => watch.disconnect();
  }, [text]);
  const style = over ? ({ "--over": `${over}px`, "--trip": `${Math.max(MIN_SECONDS, over / SPEED + 2)}s` } as CSSProperties) : undefined;
  return <span className={over ? "scroll-text moving" : "scroll-text"} ref={box} style={style}><span ref={line}>{text}</span></span>;
}
