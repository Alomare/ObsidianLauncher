// Option previews (`option_previews` in obsidian.json): an SVG template from the mod, filled in with the
// option values being edited and shown as an image. `{{expr}}` in attribute values and text becomes the
// value of the expression; `data-if="expr"` removes the element when the expression is false. A bad
// expression gives an empty string, never an error. The result is sanitized: no scripts, no
// foreignObject, no event attributes and no links outside the document.

export type Value = number | string | boolean;
export type Values = Record<string, unknown>;

/** Longest expression read, and deepest nesting, so a template can't make the window work for long. */
const MAX_EXPR = 400;
const MAX_DEPTH = 40;

class ExprError extends Error {}

type Token = { kind: "num"; v: number } | { kind: "str"; v: string } | { kind: "id"; v: string } | { kind: "op"; v: string };

const OPS = ["&&", "||", "==", "!=", "<=", ">=", "+", "-", "*", "/", "%", "<", ">", "!", "?", ":", "(", ")", ","];

function tokens(src: string): Token[] {
  if (src.length > MAX_EXPR) throw new ExprError("too long");
  const out: Token[] = [];
  let i = 0;
  while (i < src.length) {
    const c = src[i];
    if (/\s/.test(c)) { i++; continue; }
    if (c === "'") {
      let s = "";
      i++;
      while (i < src.length && src[i] !== "'") {
        if (src[i] === "\\" && i + 1 < src.length) i++;
        s += src[i++];
      }
      if (i >= src.length) throw new ExprError("string not closed");
      i++;
      out.push({ kind: "str", v: s });
      continue;
    }
    const word = /^[A-Za-z0-9_.]+/.exec(src.slice(i));
    if (word) {
      const w = word[0];
      if (/^(\d+\.?\d*|\.\d+)$/.test(w)) out.push({ kind: "num", v: Number(w) });
      else if (/^[a-z0-9_]+$/.test(w)) out.push({ kind: "id", v: w });
      else throw new ExprError(`bad word ${w}`);
      i += w.length;
      continue;
    }
    const op = OPS.find((o) => src.startsWith(o, i));
    if (!op) throw new ExprError(`bad character ${c}`);
    out.push({ kind: "op", v: op });
    i += op.length;
  }
  return out;
}

type Node =
  | { t: "lit"; v: Value }
  | { t: "var"; name: string }
  | { t: "un"; op: string; a: Node }
  | { t: "bin"; op: string; a: Node; b: Node }
  | { t: "if"; c: Node; a: Node; b: Node }
  | { t: "call"; name: string; args: Node[] };

const FUNCTIONS: Record<string, [number, number, (...n: number[]) => number]> = {
  min: [1, 16, Math.min],
  max: [1, 16, Math.max],
  abs: [1, 1, Math.abs],
  round: [1, 1, Math.round],
  floor: [1, 1, Math.floor],
  ceil: [1, 1, Math.ceil],
  clamp: [3, 3, (x, lo, hi) => Math.min(Math.max(x, lo), hi)],
};

function parse(src: string): Node {
  const toks = tokens(src);
  let at = 0;
  let depth = 0;
  const peek = (v: string) => toks[at]?.kind === "op" && toks[at].v === v;
  const take = (v: string) => {
    if (!peek(v)) throw new ExprError(`expected ${v}`);
    at++;
  };
  const deeper = <T>(f: () => T): T => {
    if (++depth > MAX_DEPTH) throw new ExprError("too deep");
    try { return f(); } finally { depth--; }
  };
  const left = (next: () => Node, ops: string[]) => (): Node => {
    let a = next();
    for (let op = ops.find(peek); op; op = ops.find(peek)) {
      at++;
      a = { t: "bin", op, a, b: next() };
    }
    return a;
  };
  const primary = (): Node => deeper(() => {
    const tok = toks[at++];
    if (!tok) throw new ExprError("unexpected end");
    if (tok.kind === "num" || tok.kind === "str") return { t: "lit", v: tok.v };
    if (tok.kind === "id") {
      if (peek("(")) {
        at++;
        const args: Node[] = [];
        if (!peek(")")) {
          args.push(expr());
          while (peek(",")) { at++; args.push(expr()); }
        }
        take(")");
        return { t: "call", name: tok.v, args };
      }
      if (tok.v === "true" || tok.v === "false") return { t: "lit", v: tok.v === "true" };
      return { t: "var", name: tok.v };
    }
    if (tok.v === "(") {
      const e = expr();
      take(")");
      return e;
    }
    if (tok.v === "!" || tok.v === "-" || tok.v === "+") return { t: "un", op: tok.v, a: primary() };
    throw new ExprError(`unexpected ${tok.v}`);
  });
  const mul = left(primary, ["*", "/", "%"]);
  const add = left(mul, ["+", "-"]);
  const rel = left(add, ["<=", ">=", "<", ">"]);
  const eq = left(rel, ["==", "!="]);
  const and = left(eq, ["&&"]);
  const or = left(and, ["||"]);
  function expr(): Node {
    return deeper(() => {
      const c = or();
      if (!peek("?")) return c;
      at++;
      const a = expr();
      take(":");
      return { t: "if", c, a, b: expr() };
    });
  }
  const e = expr();
  if (at < toks.length) throw new ExprError("text after the expression");
  return e;
}

export function truthy(v: Value): boolean {
  return typeof v === "number" ? v !== 0 && !Number.isNaN(v) : typeof v === "string" ? v !== "" : v;
}

function num(v: Value): number {
  if (typeof v === "number") return v;
  if (typeof v === "boolean") return v ? 1 : 0;
  throw new ExprError("not a number");
}

function run(n: Node, values: Values): Value {
  switch (n.t) {
    case "lit": return n.v;
    case "var": {
      const v = Object.prototype.hasOwnProperty.call(values, n.name) ? values[n.name] : undefined;
      if (typeof v === "number" || typeof v === "string" || typeof v === "boolean") return v;
      throw new ExprError(`unknown ${n.name}`);
    }
    case "un": {
      const a = run(n.a, values);
      return n.op === "!" ? !truthy(a) : n.op === "-" ? -num(a) : num(a);
    }
    case "if": return truthy(run(n.c, values)) ? run(n.a, values) : run(n.b, values);
    case "call": {
      const f = FUNCTIONS[n.name];
      if (!f || n.args.length < f[0] || n.args.length > f[1]) throw new ExprError(`bad call ${n.name}`);
      return f[2](...n.args.map((a) => num(run(a, values))));
    }
    case "bin": {
      if (n.op === "&&") { const a = run(n.a, values); return truthy(a) ? run(n.b, values) : a; }
      if (n.op === "||") { const a = run(n.a, values); return truthy(a) ? a : run(n.b, values); }
      const a = run(n.a, values);
      const b = run(n.b, values);
      switch (n.op) {
        case "==": return a === b;
        case "!=": return a !== b;
        case "+": return typeof a === "string" || typeof b === "string" ? text(a) + text(b) : num(a) + num(b);
        case "-": return num(a) - num(b);
        case "*": return num(a) * num(b);
        case "/": return num(a) / num(b);
        case "%": return num(a) % num(b);
      }
      const [x, y] = typeof a === "string" && typeof b === "string" ? [a, b] : [num(a), num(b)];
      return n.op === "<" ? x < y : n.op === ">" ? x > y : n.op === "<=" ? x <= y : x >= y;
    }
  }
}

/** The value of an expression over the option values. Throws on a bad expression. */
export function evaluate(expr: string, values: Values): Value {
  return run(parse(expr), values);
}

/** A value as it goes into the SVG: numbers with at most four decimals, nothing for a number that isn't finite. */
export function text(v: Value): string {
  if (typeof v === "number") {
    if (!Number.isFinite(v)) return "";
    return Number.isInteger(v) ? String(v) : v.toFixed(4).replace(/\.?0+$/, "");
  }
  return String(v);
}

/** Every `{{expr}}` in `s` replaced with the value; a bad one with nothing. */
export function fill(s: string, values: Values): string {
  return s.replace(/\{\{([\s\S]*?)\}\}/g, (_, expr: string) => {
    try { return text(evaluate(expr, values)); } catch { return ""; }
  });
}

/** Whether a `data-if` keeps the element: a bad expression doesn't. */
export function keeps(expr: string, values: Values): boolean {
  try { return truthy(evaluate(expr, values)); } catch { return false; }
}

/** Elements that run code or show another document; removed with everything in them. */
const DROPPED = new Set(["script", "foreignobject", "iframe", "object", "embed", "audio", "video", "canvas", "handler", "listener"]);

/** A link that stays inside the document: `#id`. */
const inside = (v: string) => v.trim().startsWith("#");
/** `url(...)` to something outside the document, or an import. */
const outsideUrl = (v: string) => /url\s*\(\s*(?!['"]?\s*#)/i.test(v) || /@import/i.test(v);

function clean(el: Element, values: Values) {
  const cond = el.getAttribute("data-if");
  if ((cond !== null && !keeps(cond, values)) || DROPPED.has(el.localName.toLowerCase())) {
    el.remove();
    return;
  }
  for (const a of [...el.attributes]) {
    const name = a.name.toLowerCase();
    if (name === "data-if" || name.startsWith("on")) {
      el.removeAttribute(a.name);
      continue;
    }
    const v = fill(a.value, values);
    if (((name === "href" || name.endsWith(":href") || name === "src") && !inside(v)) || outsideUrl(v)) el.removeAttributeNS(a.namespaceURI, a.localName);
    else if (v !== a.value) el.setAttributeNS(a.namespaceURI, a.name, v);
  }
  for (const node of [...el.childNodes]) {
    if (node.nodeType === 1) clean(node as Element, values);
    else if (node.nodeType === 3 || node.nodeType === 4) node.nodeValue = fill(node.nodeValue ?? "", values);
    else if (node.nodeType === 7) node.remove();
  }
  if (el.localName === "style" && outsideUrl(el.textContent ?? "")) el.remove();
}

/** The template filled in with `values` and sanitized, as SVG text; null when it isn't an SVG document. */
export function renderPreview(svg: string, values: Values): string | null {
  const doc = new DOMParser().parseFromString(svg, "image/svg+xml");
  const root = doc.documentElement;
  if (!root || root.localName !== "svg" || doc.getElementsByTagName("parsererror").length) return null;
  clean(root, values);
  if (!doc.documentElement) return null;
  if (!root.getAttribute("xmlns")) root.setAttribute("xmlns", "http://www.w3.org/2000/svg");
  return new XMLSerializer().serializeToString(root);
}

/** renderPreview as an image source. */
export function previewSrc(svg: string, values: Values): string | null {
  const out = renderPreview(svg, values);
  return out === null ? null : `data:image/svg+xml;charset=utf-8,${encodeURIComponent(out)}`;
}
