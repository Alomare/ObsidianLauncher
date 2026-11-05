// Colors picked for folders and profiles: the readable ink on one, and the system color picker
// for a custom one beside the presets.

/** Dark or white, whichever reads better on a #rrggbb background (WCAG relative luminance). */
export function inkOn(hex: string): string {
  const m = /^#([0-9a-f]{6})$/i.exec(hex);
  if (!m) return "#1b1b1b";
  const [r, g, b] = [0, 2, 4].map((i) => {
    const c = parseInt(m[1].slice(i, i + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  const l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
  // Contrast with #1b1b1b (luminance about 0.011) against contrast with white.
  return (l + 0.05) / 0.061 >= 1.05 / (l + 0.05) ? "#1b1b1b" : "#ffffff";
}

/**
 * Opens the system color picker next to `anchor`, starting on `current`. `onInput` follows the picker
 * as it moves; `onPick` gets the color once it's chosen. The input lives on the body, so a menu
 * that closes when the picker takes focus doesn't take the picker with it.
 */
export function pickColor(anchor: Element, current: string | null, onPick: (color: string) => void, onInput?: (color: string) => void) {
  // A picker closed without a choice leaves its input behind; the next one clears it.
  document.querySelectorAll("input[data-color-pick]").forEach((el) => el.remove());
  const r = anchor.getBoundingClientRect();
  const input = document.createElement("input");
  input.type = "color";
  input.dataset.colorPick = "";
  input.value = current && /^#[0-9a-f]{6}$/i.test(current) ? current.toLowerCase() : "#f0b429";
  input.setAttribute("aria-hidden", "true");
  input.tabIndex = -1;
  Object.assign(input.style, { position: "fixed", left: `${r.left}px`, top: `${r.bottom}px`, width: "1px", height: "1px", opacity: "0", pointerEvents: "none" });
  document.body.append(input);
  if (onInput) input.addEventListener("input", () => onInput(input.value.toLowerCase()));
  input.addEventListener("change", () => {
    onPick(input.value.toLowerCase());
    input.remove();
  });
  try {
    input.showPicker();
  } catch {
    input.click();
  }
}
