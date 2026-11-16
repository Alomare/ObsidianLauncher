// Controller buttons as small glyphs for the Keys tab: face buttons, bumpers, triggers, stick clicks,
// Back, Start and the D-pad, in the set of the controller found or the one chosen in Settings (pad.ts),
// as the glyphs on the buttons of the app are. Drawn in a 20-unit-high box with whole-unit strokes, so
// they stay crisp at the size of a line of text. Decoration only: the name is in the tooltip and the
// accessible name (`padWord`).
import type { ReactNode } from "react";
import { usePad } from "../pad/pad";
import type { PadKind } from "../pad/glyphs";
import type { PadButton } from "./padButtons";

type Face = "a" | "b" | "x" | "y";
const isFace = (b: PadButton): b is Face => b === "a" || b === "b" || b === "x" || b === "y";

/** The names of the buttons on each kind of controller. `PadButton` goes by the Xbox name of the
 *  place: "a" is the bottom face button, Cross on PlayStation and B on Nintendo. */
const WORDS: Record<PadKind, Partial<Record<PadButton, string>>> = {
  xbox: {},
  steamdeck: { lb: "L1", rb: "R1", lt: "L2", rt: "R2", back: "View", start: "Menu" },
  playstation: { a: "Cross", b: "Circle", x: "Square", y: "Triangle", lb: "L1", rb: "R1", lt: "L2", rt: "R2", back: "Create", start: "Options" },
  nintendo: { a: "B", b: "A", x: "Y", y: "X", lb: "L", rb: "R", lt: "ZL", rt: "ZR", back: "Minus", start: "Plus" },
};
const XBOX: Record<PadButton, string> = {
  a: "A", b: "B", x: "X", y: "Y", lb: "LB", rb: "RB", lt: "LT", rt: "RT", l3: "L3", r3: "R3", back: "Back", start: "Start",
  dpad: "D-pad", dpad_up: "D-pad Up", dpad_down: "D-pad Down", dpad_left: "D-pad Left", dpad_right: "D-pad Right",
};

/** The name of a button on a controller of `kind`. */
export const padWord = (b: PadButton, kind: PadKind) => WORDS[kind][b] ?? XBOX[b];

/** Face button colors: Xbox letters on colored disks, the PlayStation shapes in their colors on a
 *  dark disk, plain disks for Nintendo, whose letters carry no color. */
const XBOX_FACE: Record<Face, string> = { a: "#2fb889", b: "#f2574b", x: "#4a90d9", y: "#f0b429" };
const PS_FACE: Record<Face, string> = { a: "#7ca3d6", b: "#e5636b", x: "#d48ac0", y: "#3fb3a4" };

const LINE = { fill: "none", stroke: "currentColor", strokeWidth: 1.5, strokeLinejoin: "round" } as const;
const FONT = { fontFamily: '"Saira Variable", Saira, sans-serif' };

function Label({ x, y, size, children }: { x: number; y: number; size: number; children: string }) {
  return <text x={x} y={y} fontSize={size} fontWeight={700} textAnchor="middle" fill="currentColor" stroke="none" style={FONT}>{children}</text>;
}

function PsShape({ button }: { button: Face }) {
  const s = { fill: "none", stroke: PS_FACE[button], strokeWidth: 1.8, strokeLinejoin: "round", strokeLinecap: "round" } as const;
  if (button === "a") return <path d="M6.5 6.5l7 7M13.5 6.5l-7 7" {...s} />;
  if (button === "b") return <circle cx="10" cy="10" r="4" {...s} />;
  if (button === "x") return <rect x="6.3" y="6.3" width="7.4" height="7.4" {...s} />;
  return <path d="M10 5.6l4.6 7.9H5.4z" {...s} />;
}

function Glyph({ button, kind }: { button: PadButton; kind: PadKind }) {
  const label = padWord(button, kind);
  const box = (w: number, body: ReactNode) => (
    <svg className="pad-glyph" viewBox={`0 0 ${w} 20`} aria-hidden="true" focusable="false">{body}</svg>
  );
  if (isFace(button)) {
    if (kind === "playstation") return box(20, <><circle cx="10" cy="10" r="9" fill="#262626" /><PsShape button={button} /></>);
    const fill = kind === "nintendo" ? "#d9d9d9" : XBOX_FACE[button];
    return box(20, <>
      <circle cx="10" cy="10" r="9" fill={fill} />
      <text x="10" y="14.2" fontSize="12" fontWeight={800} textAnchor="middle" fill="#151515" style={FONT}>{label}</text>
    </>);
  }
  if (button === "lb" || button === "rb") {
    return box(26, <><rect x="1" y="4.5" width="24" height="11" rx="5" {...LINE} /><Label x={13} y={13} size={9}>{label}</Label></>);
  }
  if (button === "lt" || button === "rt") {
    return box(26, <><path d="M1.5 18.5V10a7.5 7.5 0 0 1 7.5-7.5h8a7.5 7.5 0 0 1 7.5 7.5v8.5z" {...LINE} /><Label x={13} y={15.5} size={9}>{label}</Label></>);
  }
  if (button === "l3" || button === "r3") {
    return box(20, <><circle cx="10" cy="10" r="9" {...LINE} /><circle cx="10" cy="10" r="6" fill="none" stroke="currentColor" strokeWidth="1" opacity="0.45" /><Label x={10} y={12.9} size={8}>{label}</Label></>);
  }
  if (button === "back") {
    if (kind === "nintendo") return box(20, <><circle cx="10" cy="10" r="9" {...LINE} /><path d="M6 10h8" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" /></>);
    return box(20, <><circle cx="10" cy="10" r="9" {...LINE} /><rect x="5" y="6" width="7" height="6" {...LINE} strokeWidth={1.2} /><rect x="8" y="8.5" width="7" height="6" {...LINE} strokeWidth={1.2} /></>);
  }
  if (button === "start") {
    if (kind === "nintendo") return box(20, <><circle cx="10" cy="10" r="9" {...LINE} /><path d="M6 10h8M10 6v8" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" /></>);
    return box(20, <><circle cx="10" cy="10" r="9" {...LINE} /><path d="M6 7h8M6 10h8M6 13h8" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" /></>);
  }
  const arm = { dpad_up: [7.5, 1.5, 5, 6], dpad_down: [7.5, 12.5, 5, 6], dpad_left: [1.5, 7.5, 6, 5], dpad_right: [12.5, 7.5, 6, 5] }[button as string];
  return box(20, <>
    {arm && <rect x={arm[0]} y={arm[1]} width={arm[2]} height={arm[3]} fill="currentColor" />}
    <path d="M7.5 1.5h5v6h6v5h-6v6h-5v-6h-6v-5h6z" {...LINE} />
  </>);
}

/** The buttons in a row, a plus between them, for one input of the Keys tab. */
export function PadGlyphs({ buttons }: { buttons: PadButton[] }) {
  const kind = usePad((s) => s.kind);
  return (
    <span className="pad-glyphs" aria-hidden="true">
      {buttons.map((b, i) => (
        <span key={b} className="pad-btn">{i > 0 && <span className="pad-plus">+</span>}<Glyph button={b} kind={kind} /></span>
      ))}
    </span>
  );
}
