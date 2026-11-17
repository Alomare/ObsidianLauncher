// The picture of a controller button, beside the control it works: shown only while the controller is in
// charge (the `gamepad` class on the page), in the set of the controller found or the one chosen in Settings.
//
// Where it goes in a button: before the label on a label-only button, after the icon on an icon-only one,
// and in place of the icon on a button with both (`swap`: the icon just before it hides).
import type { PadRole } from "./glyphs";
import "./pad.css";

export function PadGlyph({ role, swap }: { role: PadRole; swap?: boolean }) {
  return <i className={swap ? "gp-glyph gp-swap" : "gp-glyph"} data-role={role} aria-hidden="true" />;
}
