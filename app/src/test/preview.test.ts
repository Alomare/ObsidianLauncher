import { describe, expect, it } from "vitest";
import { evaluate, fill, keeps, previewSrc, renderPreview, text } from "../mods/preview";

const V = { size: 4, scale: 1.5, shape: "cross", color: "#ff8800", outline: true, shadow: false, gap: 0 };

describe("the expression grammar", () => {
  it("reads numbers, strings, option values and literals", () => {
    expect(evaluate("12", V)).toBe(12);
    expect(evaluate(".5", V)).toBe(0.5);
    expect(evaluate("'dot'", V)).toBe("dot");
    expect(evaluate("'don\\'t'", V)).toBe("don't");
    expect(evaluate("size", V)).toBe(4);
    expect(evaluate("color", V)).toBe("#ff8800");
    expect(evaluate("outline", V)).toBe(true);
    expect(evaluate("false", V)).toBe(false);
  });

  it("follows the usual precedence", () => {
    expect(evaluate("1 + 2 * 3", V)).toBe(7);
    expect(evaluate("(1 + 2) * 3", V)).toBe(9);
    expect(evaluate("10 - 4 - 3", V)).toBe(3);
    expect(evaluate("7 % 4", V)).toBe(3);
    expect(evaluate("-size + 1", V)).toBe(-3);
    expect(evaluate("size * scale / 2", V)).toBe(3);
  });

  it("compares and combines", () => {
    expect(evaluate("shape == 'cross'", V)).toBe(true);
    expect(evaluate("shape != 'cross'", V)).toBe(false);
    expect(evaluate("size >= 4 && scale < 2", V)).toBe(true);
    expect(evaluate("shadow || size > 9", V)).toBe(false);
    expect(evaluate("!shadow", V)).toBe(true);
    expect(evaluate("shape == 'dot' ? 1 : shape == 'cross' ? 2 : 3", V)).toBe(2);
  });

  it("joins text with +", () => {
    expect(evaluate("'r' + size", V)).toBe("r4");
  });

  it("calls the functions", () => {
    expect(evaluate("min(size, 2, 3)", V)).toBe(2);
    expect(evaluate("max(size, 9)", V)).toBe(9);
    expect(evaluate("abs(-3)", V)).toBe(3);
    expect(evaluate("round(scale)", V)).toBe(2);
    expect(evaluate("floor(scale) + ceil(scale)", V)).toBe(3);
    expect(evaluate("clamp(size * 10, 0, 25)", V)).toBe(25);
  });

  it("refuses what isn't in the grammar", () => {
    for (const bad of ["size +", "unknown", "alert(1)", "min()", "clamp(1, 2)", "'open", "a.b", "Size", "1 2", "(1", "size = 2", "shape * 2", "window", "constructor", "__proto__"]) {
      expect(() => evaluate(bad, V), bad).toThrow();
    }
    expect(() => evaluate("(".repeat(100) + "1" + ")".repeat(100), V)).toThrow();
    expect(() => evaluate("1+".repeat(300) + "1", V)).toThrow();
  });
});

describe("filling a template", () => {
  it("puts values in text and attributes, with at most four decimals", () => {
    expect(fill("r={{size * scale}} c={{color}}", V)).toBe("r=6 c=#ff8800");
    expect(fill("{{1 / 3}}", V)).toBe("0.3333");
    expect(text(1 / 0)).toBe("");
  });

  it("gives an empty string for a bad expression", () => {
    expect(fill("a{{size +}}b{{nothing}}c{{1 / gap}}", V)).toBe("abc");
  });

  it("keeps an element by data-if, and drops it on a bad expression", () => {
    expect(keeps("outline", V)).toBe(true);
    expect(keeps("shadow", V)).toBe(false);
    expect(keeps("broken (", V)).toBe(false);
  });
});

const SVG = `<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="240" height="140">
  <defs><circle id="dot" r="2"/></defs>
  <rect width="240" height="140" fill="#222" onclick="alert(1)"/>
  <g data-if="shape == 'cross'"><line x1="{{120 - size * 4}}" x2="{{120 + size * 4}}" y1="70" y2="70" stroke="{{color}}"/></g>
  <circle data-if="shape == 'dot'" cx="120" cy="70" r="{{size}}"/>
  <rect data-if="outline" class="outline" width="{{size}}" height="1"/>
  <rect data-if="shadow" class="shadow" width="1" height="1"/>
  <use href="#dot" x="10"/>
  <use href="https://example.org/x.svg#a"/>
  <image xlink:href="http://example.org/p.png"/>
  <rect fill="url(http://example.org/p.png)" stroke="url(#grad)" width="1" height="1"/>
  <script>alert(1)</script>
  <foreignObject><div xmlns="http://www.w3.org/1999/xhtml">x</div></foreignObject>
  <style>@import url(http://example.org/a.css);</style>
  <text x="4" y="12">size {{size}}</text>
</svg>`;

describe("rendering a preview", () => {
  const out = renderPreview(SVG, V)!;
  const doc = new DOMParser().parseFromString(out, "image/svg+xml");

  it("fills the values and keeps what data-if allows", () => {
    expect(doc.querySelector("line")!.getAttribute("x1")).toBe("104");
    expect(doc.querySelector("line")!.getAttribute("stroke")).toBe("#ff8800");
    expect(doc.querySelector("circle[r='4']")).toBeNull();
    expect(doc.querySelector(".outline")!.getAttribute("width")).toBe("4");
    expect(doc.querySelector(".shadow")).toBeNull();
    expect(doc.querySelector("text")!.textContent).toBe("size 4");
    expect(out).not.toContain("data-if");
  });

  it("takes out scripts, foreign content, event attributes and outside links", () => {
    expect(out).not.toMatch(/script|foreignObject|onclick|example\.org|@import/i);
    expect(doc.querySelector("use[href='#dot']")).not.toBeNull();
    expect(doc.querySelector("rect[stroke='url(#grad)']")).not.toBeNull();
  });

  it("follows the values as they change", () => {
    const dot = renderPreview(SVG, { ...V, shape: "dot", outline: false })!;
    expect(dot).toContain('r="4"');
    expect(dot).not.toContain("<line");
    expect(dot).not.toContain('class="outline"');
  });

  it("is an image source, or nothing for a document that isn't SVG", () => {
    expect(previewSrc(SVG, V)).toMatch(/^data:image\/svg\+xml;charset=utf-8,%3Csvg/);
    expect(renderPreview("<html><body/></html>", V)).toBeNull();
    expect(renderPreview("<svg><g></svg>", V)).toBeNull();
  });
});
