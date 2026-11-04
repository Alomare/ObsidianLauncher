import { describe, expect, it } from "vitest";
import { isVerified, repoOf } from "../mods/verified";

describe("verified mods", () => {
  it("reads the repository of a GitHub address", () => {
    expect(repoOf("https://github.com/Alomare/BetterHUD")).toBe("alomare/betterhud");
    expect(repoOf("https://github.com/Alomare/BetterHUD.git")).toBe("alomare/betterhud");
    expect(repoOf("https://github.com/Alomare/BetterHUD/releases")).toBe("alomare/betterhud");
    expect(repoOf("http://github.com/Alomare/BetterHUD")).toBeNull();
    expect(repoOf("https://example.com/Alomare/BetterHUD")).toBeNull();
    expect(repoOf(null)).toBeNull();
  });

  it("needs the listed id and the listed source", () => {
    expect(isVerified({ id: "alomare.betterhud", source: "https://github.com/Alomare/BetterHUD" })).toBe(true);
    expect(isVerified({ id: "alomare.betterhud", source: "https://github.com/Alomare/HD1-BetterHUD" })).toBe(true);
    expect(isVerified({ id: "obsidian.api", source: "https://github.com/Alomare/ObsidianAPI" })).toBe(true);
    expect(isVerified({ id: "alomare.betterhud", source: "https://github.com/someone/BetterHUD" })).toBe(false);
    expect(isVerified({ id: "alomare.betterhud", source: null })).toBe(false);
    expect(isVerified({ id: "someone.mod", source: "https://github.com/Alomare/BetterHUD" })).toBe(false);
  });
});
