// Colors picked for folders and profiles: the readable ink on one. The picker for a custom color is
// ColorPicker.tsx.

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
