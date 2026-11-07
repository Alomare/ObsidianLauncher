import { describe, expect, it } from "vitest";
import { applyAppearance, motionOn } from "../theme";
import type { AppSettings } from "../types";

const base: AppSettings = { theme: "light", scale: 125, text: "large", motion: null, language: null, updates: "prompt", gameDir: null, askVanillaPlay: true };

describe("applyAppearance", () => {
  it("sets_attributes_and_scale", () => {
    const root = document.createElement("html");
    applyAppearance(base, root, false);
    expect([root.dataset.theme, root.dataset.motion]).toEqual(["light", "on"]);
    expect(root.style.getPropertyValue("--scale")).toBe("1.25");
    expect(root.style.getPropertyValue("--text")).toBe("1.15");
  });
  it("motion_null_follows_reduced_motion", () => {
    expect(motionOn(null, true)).toBe(false);
    expect(motionOn(null, false)).toBe(true);
    expect(motionOn(true, true)).toBe(true);
    const root = document.createElement("html");
    applyAppearance({ ...base, motion: false }, root, false);
    expect(root.dataset.motion).toBe("off");
  });
});
