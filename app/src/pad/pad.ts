// Controller support. The left stick moves a cursor of its own over the window, and the face button at
// the bottom (A, Cross) clicks what it points at, and held down drags it (a mod row, a divider): the page gets the same pointer and mouse events a mouse
// would send, so every control works without knowing about controllers. The right stick scrolls what the
// cursor is over. The east button (B, Circle) backs out: the Back button of a page, else Esc on a menu or
// dialog. The north button (Y, Triangle) plays, Start opens the play choice, the triggers go through the
// main tabs and the bumpers through the tabs of the page. A real mouse moving puts the mouse back in charge.
//
// CSS can't be told an element is hovered, so while the controller is in charge the :hover and :active
// rules are copied once with .pad-hover and .pad-active in their place, and those classes follow the cursor.
import { useSyncExternalStore } from "react";
import { currentSettings } from "../settings";
import { glyphUrl, kindOf, shownKind, type GlyphSetting, type PadKind, type PadRole } from "./glyphs";
import "./pad.css";

export type PadState = {
  /** The controller is in charge: the cursor shows, and so do the button pictures. */
  active: boolean;
  /** The kind and name of the controller last used, once one has been found. */
  found: PadKind | null;
  name: string | null;
  /** The pictures shown, from the setting and the controller. */
  kind: PadKind;
};
let state: PadState = { active: false, found: null, name: null, kind: "xbox" };
const listeners = new Set<() => void>();
export const usePad = () => useSyncExternalStore((f) => { listeners.add(f); return () => { listeners.delete(f); }; }, () => state);
export const padState = () => state;

export type PadAction = "tabPrev" | "tabNext" | "play" | "options";
const handlers = new Map<PadAction, Set<() => void>>();
/** Runs `fn` when the controller asks for `action` with no menu or dialog open. Returns the undo. */
export function onPadAction(action: PadAction, fn: () => void) {
  const set = handlers.get(action) ?? new Set();
  handlers.set(action, set);
  set.add(fn);
  return () => { set.delete(fn); };
}
const emit = (action: PadAction) => handlers.get(action)?.forEach((f) => f());

// ---- Pure parts, tested on their own ----

/** The standard mapping (w3.org/TR/gamepad): buttons by position. */
export const BUTTON = { south: 0, east: 1, west: 2, north: 3, lb: 4, rb: 5, lt: 6, rt: 7, back: 8, start: 9 } as const;
export const DEAD_ZONE = 0.2;

/** A stick axis past the dead zone, from 0 to 1 and squared, so small moves stay fine and full tilt is fast. */
export function curve(v: number) {
  const m = Math.abs(v);
  if (m < DEAD_ZONE) return 0;
  const x = Math.min(1, (m - DEAD_ZONE) / (1 - DEAD_ZONE));
  return Math.sign(v) * x * x;
}

/** The next tab, going round at the ends. */
export const wrap = (i: number, n: number, dir: number) => (((i + dir) % n) + n) % n;

/** A CSS selector with :hover and :active read from the classes the cursor sets. */
export const padSelector = (sel: string) => sel.replace(/:hover\b/g, ".pad-hover").replace(/:active\b/g, ".pad-active");

/** The rules of `rules` that use :hover or :active, rewritten for the cursor, as CSS text. */
export function mirrorRules(rules: CSSRuleList | CSSRule[]): string {
  let out = "";
  for (const r of Array.from(rules)) {
    if (r instanceof CSSStyleRule) {
      if (/:hover\b|:active\b/.test(r.selectorText)) out += `${padSelector(r.selectorText)} { ${r.style.cssText} }\n`;
    } else if (r instanceof CSSMediaRule) {
      const inner = mirrorRules(r.cssRules);
      if (inner) out += `@media ${r.conditionText} {\n${inner}}\n`;
    } else if (typeof CSSSupportsRule !== "undefined" && r instanceof CSSSupportsRule) {
      const inner = mirrorRules(r.cssRules);
      if (inner) out += `@supports ${r.conditionText} {\n${inner}}\n`;
    }
  }
  return out;
}

// ---- The page ----

const ROLES: PadRole[] = ["south", "east", "west", "north", "lb", "rb", "lt", "rt", "start", "lstick", "rstick"];
const CURSOR_PX = 1100; // a second at full tilt, at speed 100
const SCROLL_PX = 1700;

let x = 0;
let y = 0;
let cursor: HTMLDivElement | null = null;
let hovered: Element | null = null;
let hoverChain: Element[] = [];
let pressed: Element | null = null;
let activeChain: Element[] = [];
let mirror: HTMLStyleElement | null = null;
let mirroredSheets = -1;
let before: boolean[] = [];
let raf = 0;
let last = 0;
let scrollRest = { x: 0, y: 0 };
let tabsMarkAt = 0;

function set(patch: Partial<PadState>) {
  const next = { ...state, ...patch };
  if (next.active === state.active && next.found === state.found && next.name === state.name && next.kind === state.kind) return;
  if (next.kind !== state.kind || !document.documentElement.style.getPropertyValue("--pad-south")) glyphVars(next.kind);
  state = next;
  document.documentElement.classList.toggle("gamepad", state.active);
  listeners.forEach((f) => f());
}

/** The pictures as CSS variables (`--pad-south` and so on), for masks in CSS. */
function glyphVars(kind: PadKind) {
  const root = document.documentElement.style;
  for (const role of ROLES) root.setProperty(`--pad-${role}`, `url("${glyphUrl(kind, role)}")`);
}

function buildMirror() {
  if (document.styleSheets.length === mirroredSheets) return;
  mirroredSheets = document.styleSheets.length;
  let text = "";
  for (const sheet of Array.from(document.styleSheets)) {
    if (sheet.ownerNode === mirror) continue;
    try { text += mirrorRules(sheet.cssRules); } catch { /* a sheet from another origin (fonts) can't be read */ }
  }
  if (!mirror) {
    mirror = document.createElement("style");
    mirror.id = "pad-hover";
    document.head.append(mirror);
  }
  mirror.textContent = text;
  mirroredSheets = document.styleSheets.length;
}

function showCursor(on: boolean) {
  if (!cursor) {
    cursor = document.createElement("div");
    cursor.className = "pad-cursor";
    cursor.setAttribute("aria-hidden", "true");
    cursor.innerHTML = '<svg viewBox="0 0 24 24"><path d="M4 2.5v17.2l4.6-4.3 3 6.6 3.1-1.4-3-6.5 6.3-.4z"/></svg>';
    document.body.append(cursor);
  }
  cursor.hidden = !on;
  place();
}

const place = () => { if (cursor) cursor.style.transform = `translate(${x}px, ${y}px)`; };

function activate() {
  if (state.active) return;
  buildMirror();
  if (mirror) mirror.disabled = false;
  set({ active: true });
  showCursor(true);
  hoverAt();
}

function deactivate() {
  if (!state.active) return;
  set({ active: false });
  showCursor(false);
  hover(null);
  release(false);
  if (mirror) mirror.disabled = true;
}

/** The id of the cursor pointer. Not 1, the real mouse: capturing that one would take the mouse away. */
export const PAD_POINTER = 4097;
/** What took the cursor pointer with setPointerCapture: while the button is held it gets the moves and the release. */
let captured: Element | null = null;

function fire(el: Element, type: string, init: MouseEventInit = {}) {
  const opts: PointerEventInit = { bubbles: true, cancelable: true, composed: true, clientX: x, clientY: y, button: 0, ...init };
  const pointer = /^(pointer|gotpointer|lostpointer)/.test(type) && typeof PointerEvent !== "undefined";
  return el.dispatchEvent(pointer ? new PointerEvent(type, { pointerId: PAD_POINTER, pointerType: "mouse", isPrimary: true, ...opts }) : new MouseEvent(type, opts));
}

/** Pointer capture for the cursor pointer, which the browser doesn't know: dragging a row or a divider holds on to it. */
function patchCapture() {
  const proto = Element.prototype as Element & Record<string, unknown>;
  const real = { set: proto.setPointerCapture, release: proto.releasePointerCapture, has: proto.hasPointerCapture };
  proto.setPointerCapture = function (this: Element, id: number) {
    if (id !== PAD_POINTER) return real.set?.call(this, id);
    if (captured === this) return;
    if (captured) loseCapture();
    captured = this;
    fire(this, "gotpointercapture");
  };
  proto.releasePointerCapture = function (this: Element, id: number) {
    if (id !== PAD_POINTER) return real.release?.call(this, id);
    if (captured === this) loseCapture();
  };
  proto.hasPointerCapture = function (this: Element, id: number) {
    return id === PAD_POINTER ? captured === this : (real.has?.call(this, id) ?? false);
  };
}

function loseCapture() {
  const el = captured;
  captured = null;
  if (el?.isConnected) fire(el, "lostpointercapture");
}

function chainOf(el: Element | null) {
  const out: Element[] = [];
  for (let e = el; e; e = e.parentElement) out.push(e);
  return out;
}

function swapClass(old: Element[], next: Element[], cls: string) {
  const keep = new Set(next);
  for (const e of old) if (!keep.has(e)) e.classList.remove(cls);
  for (const e of next) e.classList.add(cls);
}

function hover(el: Element | null) {
  if (el === hovered) return;
  const old = hovered;
  hovered = el;
  if (old?.isConnected) {
    fire(old, "pointerout", { relatedTarget: el });
    fire(old, "mouseout", { relatedTarget: el });
  }
  if (el) {
    fire(el, "pointerover", { relatedTarget: old });
    fire(el, "mouseover", { relatedTarget: old });
  }
  const next = chainOf(el);
  swapClass(hoverChain, next, "pad-hover");
  hoverChain = next;
}

function hoverAt() {
  const el = document.elementFromPoint(x, y);
  const buttons = pressed ? 1 : 0;
  // While held, what captured the pointer gets the moves and keeps the hover, as with a mouse.
  const to = captured?.isConnected ? captured : el;
  hover(to);
  if (to) fire(to, "pointermove", { buttons });
  if (el) fire(el, "mousemove", { buttons });
}

const TYPING = "input, textarea, select, [contenteditable=''], [contenteditable='true']";

function press() {
  const el = document.elementFromPoint(x, y);
  if (!el) return;
  pressed = el;
  fire(el, "pointerdown", { buttons: 1 });
  fire(el, "mousedown", { buttons: 1 });
  const field = el.closest<HTMLElement>(TYPING);
  if (field) field.focus();
  else if ((document.activeElement as HTMLElement | null)?.matches?.(TYPING)) (document.activeElement as HTMLElement).blur();
  activeChain = chainOf(el);
  activeChain.forEach((e) => e.classList.add("pad-active"));
}

function common(a: Element, b: Element) {
  const up = new Set(chainOf(a));
  for (let e: Element | null = b; e; e = e.parentElement) if (up.has(e)) return e;
  return null;
}

function release(click: boolean) {
  const down = pressed;
  pressed = null;
  activeChain.forEach((e) => e.classList.remove("pad-active"));
  activeChain = [];
  if (!down) { loseCapture(); return; }
  const at = document.elementFromPoint(x, y) ?? down;
  const el = captured?.isConnected ? captured : at;
  fire(el, "pointerup");
  loseCapture();
  fire(at, "mouseup");
  const target = down.isConnected ? common(down, el) : null;
  if (click && target) fire(target, "click", { detail: 1 });
  if (state.active) hoverAt();
}

const visible = (el: Element) => (el as HTMLElement).offsetParent !== null || el.getClientRects().length > 0;

/** The menu or dialog on top, or null for the page. */
function topLayer(): Element | null {
  const menus = document.querySelectorAll(".ctx");
  if (menus.length) return menus[menus.length - 1];
  const modals = [...document.querySelectorAll(".modal-back")].filter(visible);
  return modals.length ? modals[modals.length - 1] : null;
}

/** The tab row the bumpers move along: in the dialog on top, else the first one shown on the page. */
function pageTabs(): Element | null {
  const layer = topLayer();
  const scope: ParentNode = layer ?? document;
  return [...scope.querySelectorAll("[role=tablist]")].find((l) => visible(l) && !l.closest(".nav") && (layer || !l.closest(".modal-back"))) ?? null;
}

/** Marks the tab row the bumpers move along, for the pictures around it. */
function markTabs() {
  const tabs = state.active ? pageTabs() : null;
  document.querySelectorAll("[data-pad-tabs]").forEach((e) => e !== tabs && e.removeAttribute("data-pad-tabs"));
  tabs?.setAttribute("data-pad-tabs", "");
}

function stepTabs(dir: number) {
  const list = pageTabs();
  if (!list) return;
  const tabs = [...list.querySelectorAll<HTMLElement>("[role=tab], [role=radio], button")].filter((b) => visible(b) && !(b as HTMLButtonElement).disabled);
  if (tabs.length < 2) return;
  const at = tabs.findIndex((b) => b.getAttribute("aria-selected") === "true" || b.getAttribute("aria-checked") === "true" || b.classList.contains("cur"));
  tabs[wrap(Math.max(0, at), tabs.length, dir)].click();
}

function back() {
  const layer = topLayer();
  const scope: ParentNode = layer ?? document;
  const target = [...scope.querySelectorAll<HTMLElement>("[data-pad-back]")].find((b) => visible(b) && (layer || !b.closest(".modal-back")) && !(b as HTMLButtonElement).disabled);
  if (target) { target.click(); return; }
  if (!layer) return;
  const at = document.activeElement && layer.contains(document.activeElement) ? document.activeElement : layer;
  at.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", code: "Escape", bubbles: true, cancelable: true }));
}

function scrollerAt(el: Element | null): Element | null {
  for (let e = el; e && e !== document.documentElement; e = e.parentElement) {
    const s = getComputedStyle(e);
    const ys = /(auto|scroll|overlay)/.test(s.overflowY) && e.scrollHeight > e.clientHeight + 1;
    const xs = /(auto|scroll|overlay)/.test(s.overflowX) && e.scrollWidth > e.clientWidth + 1;
    if (ys || xs) return e;
  }
  return document.scrollingElement;
}

/** The controller used last: the one with the newest input. */
function current(): Gamepad | null {
  let best: Gamepad | null = null;
  for (const g of navigator.getGamepads?.() ?? []) if (g && g.connected && (!best || g.timestamp > best.timestamp)) best = g;
  return best;
}

function frame(now: number) {
  raf = 0;
  const pad = current();
  if (!pad) { before = []; return; }
  raf = requestAnimationFrame(frame);
  const dt = Math.min(0.05, last ? (now - last) / 1000 : 0);
  last = now;

  const found = kindOf(pad.id);
  const setting = (currentSettings()?.padGlyphs ?? "auto") as GlyphSetting;
  set({ found, name: pad.id.replace(/\s*\(.*\)\s*$/, "") || pad.id, kind: shownKind(setting, found) });

  const down = pad.buttons.map((b) => b.pressed || b.value > 0.5);
  const prev = before;
  const hit = (i: number) => down[i] && !prev[i];
  const lifted = (i: number) => !down[i] && prev[i];
  const [lx, ly, rx, ry] = [0, 1, 2, 3].map((i) => curve(pad.axes[i] ?? 0));
  const any = down.some(Boolean) || prev.some(Boolean) || lx || ly || rx || ry;
  before = down;
  if (!any) return;
  if (!state.active) {
    // The first touch only shows the cursor, so nothing is clicked unseen.
    activate();
    return;
  }

  if (lx || ly) {
    const speed = (CURSOR_PX * (currentSettings()?.padSpeed ?? 100)) / 100;
    x = Math.max(0, Math.min(innerWidth - 1, x + lx * speed * dt));
    y = Math.max(0, Math.min(innerHeight - 1, y + ly * speed * dt));
    place();
    hoverAt();
  }
  if (rx || ry) {
    const s = scrollerAt(document.elementFromPoint(x, y));
    scrollRest = { x: scrollRest.x + rx * SCROLL_PX * dt, y: scrollRest.y + ry * SCROLL_PX * dt };
    const step = { x: Math.trunc(scrollRest.x), y: Math.trunc(scrollRest.y) };
    scrollRest = { x: scrollRest.x - step.x, y: scrollRest.y - step.y };
    if (s && (step.x || step.y)) s.scrollBy({ left: step.x, top: step.y });
  }

  if (hit(BUTTON.south)) press();
  if (lifted(BUTTON.south)) release(true);
  if (hit(BUTTON.east)) back();
  const page = !topLayer();
  if (hit(BUTTON.north) && page) emit("play");
  if (hit(BUTTON.start) && page) emit("options");
  if (hit(BUTTON.lt) && page) emit("tabPrev");
  if (hit(BUTTON.rt) && page) emit("tabNext");
  if (hit(BUTTON.lb)) stepTabs(-1);
  if (hit(BUTTON.rb)) stepTabs(1);
  if (now - tabsMarkAt > 300) { tabsMarkAt = now; markTabs(); }
}

function wake() {
  if (!raf) { last = 0; raf = requestAnimationFrame(frame); }
}

let started = false;
/** Starts listening for controllers; mounted once in App. */
export function startPad() {
  if (started || typeof window === "undefined") return;
  started = true;
  x = innerWidth / 2;
  y = innerHeight / 2;
  glyphVars(state.kind);
  patchCapture();
  window.addEventListener("gamepadconnected", wake);
  // The cursor starts where the mouse was, and a real mouse takes over again.
  window.addEventListener("pointermove", (e) => {
    if (!e.isTrusted) return;
    if (state.active && (e.movementX || e.movementY)) deactivate();
    if (!state.active) { x = e.clientX; y = e.clientY; }
  }, true);
  window.addEventListener("pointerdown", (e) => { if (e.isTrusted) deactivate(); }, true);
  // A controller connected before the window opened shows up at the first button press, as a connect.
  if (current()) wake();
}
