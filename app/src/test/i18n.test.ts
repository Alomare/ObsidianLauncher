import { describe, expect, it } from "vitest";
import { pickLanguage, translate, translateCount } from "../i18n";

const catalogs = { en: { hi: "Hello %s, %s", only: "English only" }, "pt-BR": { hi: "Olá %s, %s" } };

describe("translate", () => {
  it("placeholders_fill_in_order", () => expect(translate(catalogs, "pt-BR", "hi", ["a", 2])).toBe("Olá a, 2"));
  it("missing_key_falls_back_to_english", () => expect(translate(catalogs, "pt-BR", "only", [])).toBe("English only"));
  it("missing_everywhere_returns_key", () => expect(translate(catalogs, "pt-BR", "nope", [])).toBe("nope"));
});

describe("translateCount", () => {
  const counts = {
    en: { mods: "%s mods", "mods#one": "%s mod", left: "%s left of %s" },
    ru: { mods: "%s модов", "mods#one": "%s мод", "mods#few": "%s мода" },
    "zh-CN": { mods: "%s 个模组" },
  };
  it("takes_the_form_of_the_plural_category", () => {
    expect(translateCount(counts, "en", "mods", 1, [])).toBe("1 mod");
    expect(translateCount(counts, "en", "mods", 2, [])).toBe("2 mods");
    expect(translateCount(counts, "ru", "mods", 21, [])).toBe("21 мод");
    expect(translateCount(counts, "ru", "mods", 3, [])).toBe("3 мода");
    expect(translateCount(counts, "ru", "mods", 5, [])).toBe("5 модов");
  });
  it("uses_the_plain_key_where_a_language_has_no_form", () => expect(translateCount(counts, "zh-CN", "mods", 1, [])).toBe("1 个模组"));
  it("never_takes_an_english_form_over_a_translation", () => expect(translateCount(counts, "zh-CN", "mods", 1, [])).not.toContain("mod "));
  it("fills_the_arguments_given", () => expect(translateCount(counts, "en", "left", 2, [2, 9])).toBe("2 left of 9"));
});

describe("pickLanguage", () => {
  it("prefers_the_saved_choice", () => expect(pickLanguage("zh-CN", ["pt-BR"])).toBe("zh-CN"));
  it("matches_the_system_language_by_its_first_part", () => expect(pickLanguage(null, ["pt-PT", "en-US"])).toBe("pt-BR"));
  it("falls_back_to_english", () => expect(pickLanguage("xx", ["de-DE"])).toBe("en"));
});
