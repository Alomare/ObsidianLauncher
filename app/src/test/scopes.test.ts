import { describe, expect, it } from "vitest";
import en from "../../../locales/en.json";
import ptBR from "../../../locales/pt-BR.json";
import zhCN from "../../../locales/zh-CN.json";
import { SCOPES } from "../types";

// The mod list shows a diamond per scope named in the title, with a hint, and the details name the scope.
describe("every scope", () => {
  for (const scope of SCOPES) {
    it(`${scope}_has_a_name_and_a_hint_in_every_app_language`, () => {
      for (const catalog of [en, ptBR, zhCN] as Record<string, string>[]) {
        for (const key of [`app.scope.${scope}`, `app.scope.${scope}Hint`]) expect(catalog[key], key).toBeTruthy();
      }
    });
  }
});
