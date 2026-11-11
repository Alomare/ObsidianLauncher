// The controller buttons a choice names, for the Keys tab to draw instead of spelling out. A controller
// input is a choice option of the mod (Obsidian has no capture for it: the mod lists the combinations it
// reads), so the buttons come from the choice's label, or the id when the label names none. HD1 and the
// loader call them LB, RB, LT, RT, L3 (left stick pressed in), R3, Back, Start, A, B, X, Y and the D-pad.

export type PadButton =
  | "a" | "b" | "x" | "y"
  | "lb" | "rb" | "lt" | "rt"
  | "l3" | "r3"
  | "back" | "start"
  | "dpad" | "dpad_up" | "dpad_down" | "dpad_left" | "dpad_right";

const WORDS = "LB|RB|LT|RT|L3|R3|Back|Select|Start|D-?pad(?:[ _-]?(?:up|down|left|right))?";
const WORD = new RegExp(`\\b(?:${WORDS})\\b`, "gi");
const LETTERS = `(?:${WORDS}|[ABXY])`;
/** A label that is nothing but buttons joined by plus signs, "A + B" or "LB + RB + Back". */
const COMBO = new RegExp(`^\\s*${LETTERS}(?:\\s*\\+\\s*${LETTERS})*\\s*$`, "i");

function button(word: string): PadButton | null {
  const w = word.toLowerCase().replace(/[ _-]/g, "");
  if (w === "select") return "back";
  if (/^dpad(up|down|left|right)?$/.test(w)) return (w === "dpad" ? "dpad" : `dpad_${w.slice(4)}`) as PadButton;
  return ["a", "b", "x", "y", "lb", "rb", "lt", "rt", "l3", "r3", "back", "start"].includes(w) ? (w as PadButton) : null;
}

function dedupe(list: PadButton[]): PadButton[] {
  return list.filter((b, i) => list.indexOf(b) === i);
}

/** The buttons in a choice's label (or id), in the order named, or null when it names none and is plain text.
 *  A single letter counts as a button only when the whole label is a combination: "Press A" is a sentence. */
export function padButtons(label: string, id = ""): PadButton[] | null {
  const bare = label.replace(/\s*\([^)]*\)\s*$/, "");
  for (const text of [label, bare]) {
    if (COMBO.test(text)) {
      const list = text.split("+").map((w) => button(w.trim()));
      if (list.every(Boolean)) return dedupe(list as PadButton[]);
    }
  }
  const found = (label.match(WORD) ?? []).map(button).filter((b): b is PadButton => !!b);
  if (found.length) return dedupe(found);
  const name = id.replace(/d-?pad[_-]?(?=up|down|left|right)/i, "dpad");
  if (name && COMBO.test(name.replace(/[_-]+/g, " + "))) {
    const list = name.split(/[_-]+/).map(button);
    if (list.every(Boolean)) return dedupe(list as PadButton[]);
  }
  return null;
}
