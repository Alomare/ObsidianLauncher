// The picture of a controller button, beside the control it works: shown only while the controller is in
// charge (the `pad` class on the page), in the set of the controller found or the one chosen in Settings.
import type { PadRole } from "./glyphs";
import "./pad.css";

export function PadGlyph({ role }: { role: PadRole }) {
  return <i className="pad-glyph" data-role={role} aria-hidden="true" />;
}
