import { describe, expect, it } from "vitest";
import { inkOn } from "../colors";
import { FOLDER_COLORS, isCustomColor } from "../mods/FolderDialog";

describe("colors", () => {
  it("puts dark ink on light colors and white ink on dark ones", () => {
    expect(inkOn("#f0b429")).toBe("#1b1b1b");
    expect(inkOn("#ffffff")).toBe("#1b1b1b");
    expect(inkOn("#101820")).toBe("#ffffff");
    expect(inkOn("#5a1a8a")).toBe("#ffffff");
    expect(inkOn("not a color")).toBe("#1b1b1b");
  });

  it("the ink on every preset reads at 3.5:1 or better", () => {
    const lum = (hex: string) => {
      const [r, g, b] = [1, 3, 5].map((i) => { const c = parseInt(hex.slice(i, i + 2), 16) / 255; return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4; });
      return 0.2126 * r + 0.7152 * g + 0.0722 * b;
    };
    for (const c of FOLDER_COLORS) {
      const [a, b] = [lum(c), lum(inkOn(c))];
      expect((Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05), c).toBeGreaterThanOrEqual(3.5);
    }
  });

  it("knows a custom color from a preset, whatever the case", () => {
    expect(isCustomColor("#F0B429")).toBe(false);
    expect(isCustomColor("#123456")).toBe(true);
    expect(isCustomColor(null)).toBe(false);
    expect(isCustomColor("#123456", ["#123456"])).toBe(false);
  });
});
