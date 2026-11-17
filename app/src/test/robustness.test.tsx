// @ts-nocheck: uses Node file access, which the app types leave out; vitest runs it as is.
// Properties over random input and checks of the text catalogs against the code. Seeds are fixed.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke, convertFileSrc: (p: string) => p }));
vi.mock("../opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));
vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: vi.fn() }));
const { compareVersions } = await import("../updates");
let seed = 99;
const rand = (n: number) => { seed = (seed * 1103515245 + 12345) % 2147483648; return seed % n; };

describe("version order", () => {
  const version = () => {
    const core = `${rand(3)}.${rand(12)}.${rand(12)}`;
    const pre = ["", "-beta.1", "-beta.2", "-beta.10", "-rc.1", "-alpha", "-1"][rand(7)];
    return core + pre + (rand(5) === 0 ? "+build" : "");
  };
  it("is a total order: antisymmetric and transitive over random versions", () => {
    const list = Array.from({ length: 300 }, version);
    for (const a of list.slice(0, 60)) for (const b of list.slice(0, 60)) {
      expect(Math.sign(compareVersions(a, b)) + 0).toBe(-Math.sign(compareVersions(b, a)) + 0);
      for (const c of list.slice(0, 15)) {
        if (compareVersions(a, b) <= 0 && compareVersions(b, c) <= 0) expect(compareVersions(a, c)).toBeLessThanOrEqual(0);
      }
    }
  });
});

const SRC = join(__dirname, "..");
function files(dir: string, out: string[] = []): string[] {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) { if (f !== "test" && f !== "node_modules") files(p, out); }
    else if (/\.(ts|tsx)$/.test(f)) out.push(p);
  }
  return out;
}

describe("text catalogs", () => {
  const en: Record<string, string> = JSON.parse(readFileSync(join(SRC, "../../locales/en.json"), "utf8"));
  const code = files(SRC).map((p) => readFileSync(p, "utf8")).join("\n");

  it("every literal t(\"key\") in the code is in en.json", () => {
    const used = new Set([...code.matchAll(/\btn?\(\s*"([a-zA-Z0-9_.]+)"/g)].map((m) => m[1]));
    const missing = [...used].filter((k) => k.startsWith("app.") && !(k in en));
    expect(missing).toEqual([]);
  });

  it("every translation has the placeholders of the English text", () => {
    const dir = join(SRC, "../../locales");
    const bad: string[] = [];
    for (const f of readdirSync(dir).filter((f) => f.endsWith(".json") && f !== "en.json")) {
      const cat: Record<string, string> = JSON.parse(readFileSync(join(dir, f), "utf8"));
      for (const [k, v] of Object.entries(cat)) {
        const want = (en[k]?.match(/%s/g) ?? []).length;
        const got = (v.match(/%s/g) ?? []).length;
        if (k in en && want !== got) bad.push(`${f} ${k}: ${got} of ${want}`);
      }
    }
    expect(bad).toEqual([]);
  });
});
