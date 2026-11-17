import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { glyphUrl, kindOf, shownKind } from "../pad/glyphs";
import { BUTTON, PAD_POINTER, curve, mirrorRules, onPadAction, padSelector, padState, startPad, wrap } from "../pad/pad";

describe("controller kinds", () => {
  it("reads the vendor from the id, then words in the name", () => {
    expect(kindOf("DualSense Wireless Controller (STANDARD GAMEPAD Vendor: 054c Product: 0ce6)")).toBe("playstation");
    expect(kindOf("054c-09cc-Wireless Controller")).toBe("playstation");
    expect(kindOf("Pro Controller (STANDARD GAMEPAD Vendor: 057e Product: 2009)")).toBe("nintendo");
    expect(kindOf("Steam Deck (Vendor: 28de Product: 1205)")).toBe("steamdeck");
    expect(kindOf("Xbox 360 Controller (XInput STANDARD GAMEPAD)")).toBe("xbox");
    expect(kindOf("Wireless Controller DualShock 4")).toBe("playstation");
    expect(kindOf("Some Generic Pad")).toBe("xbox");
  });

  it("takes the setting over the controller, and the controller with Auto", () => {
    expect(shownKind("auto", "playstation")).toBe("playstation");
    expect(shownKind("auto", null)).toBe("xbox");
    expect(shownKind("nintendo", "playstation")).toBe("nintendo");
  });

  it("has a picture for every button of every set", () => {
    for (const kind of ["xbox", "playstation", "nintendo", "steamdeck"] as const) {
      for (const role of ["south", "east", "west", "north", "lb", "rb", "lt", "rt", "start", "lstick", "rstick"] as const) {
        expect(glyphUrl(kind, role), `${kind} ${role}`).not.toBe("");
      }
    }
  });
});

describe("controller helpers", () => {
  it("ignores a stick inside the dead zone and squares the rest", () => {
    expect(curve(0.1)).toBe(0);
    expect(curve(-0.15)).toBe(0);
    expect(curve(1)).toBe(1);
    expect(curve(-1)).toBe(-1);
    expect(curve(0.6)).toBeCloseTo(0.25);
  });

  it("goes round the tabs at both ends", () => {
    expect(wrap(0, 5, -1)).toBe(4);
    expect(wrap(4, 5, 1)).toBe(0);
    expect(wrap(2, 5, 1)).toBe(3);
  });

  it("copies hover and active rules for the cursor, media rules included", () => {
    expect(padSelector(".btn:hover, .a:active .b")).toBe(".btn.pad-hover, .a.pad-active .b");
    const style = document.createElement("style");
    style.textContent = ".x:hover { color: red; } .y { color: blue; } @media (min-width: 1px) { .z:active { color: green; } }";
    document.head.append(style);
    const text = mirrorRules(style.sheet!.cssRules);
    style.remove();
    expect(text).toContain(".x.pad-hover");
    expect(text).toContain(".z.pad-active");
    expect(text).not.toContain(".y");
    expect(text).toContain("@media");
  });
});

// A controller the test holds: buttons by the standard mapping, axes at rest.
const buttons = Array.from({ length: 17 }, () => ({ pressed: false, value: 0, touched: false }));
const axes = [0, 0, 0, 0];
let stamp = 0;
const pad = () => ({ id: "Xbox 360 Controller (XInput STANDARD GAMEPAD)", index: 0, connected: true, mapping: "standard", timestamp: ++stamp, buttons, axes });
let frames: FrameRequestCallback[] = [];
let now = 0;
function tick() {
  const run = frames;
  frames = [];
  now += 16;
  run.forEach((f) => f(now));
}
function hold(i: number, on: boolean) {
  buttons[i].pressed = on;
  tick();
}

describe("the controller on the page", () => {
  beforeAll(() => {
    Object.defineProperty(navigator, "getGamepads", { configurable: true, value: () => [pad()] });
    vi.stubGlobal("requestAnimationFrame", (f: FrameRequestCallback) => { frames.push(f); return frames.length; });
    startPad();
    window.dispatchEvent(new Event("gamepadconnected"));
  });
  afterEach(() => {
    document.body.innerHTML = "";
    buttons.forEach((b) => (b.pressed = false));
    tick();
  });

  it("shows the cursor at the first touch, then clicks what it points at", () => {
    const target = document.createElement("button");
    const click = vi.fn();
    target.addEventListener("click", click);
    document.body.append(target);
    document.elementFromPoint = () => target;

    hold(BUTTON.south, true);
    hold(BUTTON.south, false);
    expect(padState().active).toBe(true);
    expect(document.documentElement.classList.contains("gamepad")).toBe(true);
    expect(click).not.toHaveBeenCalled();

    hold(BUTTON.south, true);
    expect(target.classList.contains("pad-active")).toBe(true);
    hold(BUTTON.south, false);
    expect(click).toHaveBeenCalledOnce();
    expect(target.classList.contains("pad-hover")).toBe(true);
  });

  it("backs out with the Back button of the page, or Esc in a dialog", () => {
    document.elementFromPoint = () => document.body;
    const backBtn = document.createElement("button");
    backBtn.setAttribute("data-pad-back", "");
    const back = vi.fn();
    backBtn.addEventListener("click", back);
    document.body.append(backBtn);
    // jsdom lays nothing out: count every element as shown.
    Object.defineProperty(HTMLElement.prototype, "offsetParent", { configurable: true, get() { return document.body; } });
    hold(BUTTON.south, true); hold(BUTTON.south, false);
    hold(BUTTON.east, true); hold(BUTTON.east, false);
    expect(back).toHaveBeenCalledOnce();

    const modal = document.createElement("div");
    modal.className = "modal-back";
    document.body.append(modal);
    const esc = vi.fn();
    window.addEventListener("keydown", esc);
    hold(BUTTON.east, true); hold(BUTTON.east, false);
    window.removeEventListener("keydown", esc);
    expect(back).toHaveBeenCalledOnce();
    expect(esc.mock.calls[0][0].key).toBe("Escape");
  });

  it("asks for the next tab with a trigger and Play with the north button, not under a dialog", () => {
    document.elementFromPoint = () => document.body;
    const next = vi.fn();
    const play = vi.fn();
    const offs = [onPadAction("tabNext", next), onPadAction("play", play)];
    hold(BUTTON.south, true); hold(BUTTON.south, false);
    hold(BUTTON.rt, true); hold(BUTTON.rt, false);
    hold(BUTTON.north, true); hold(BUTTON.north, false);
    expect(next).toHaveBeenCalledOnce();
    expect(play).toHaveBeenCalledOnce();

    const modal = document.createElement("div");
    modal.className = "modal-back";
    document.body.append(modal);
    hold(BUTTON.north, true); hold(BUTTON.north, false);
    expect(play).toHaveBeenCalledOnce();
    offs.forEach((f) => f());
  });

  it("drags while the button is held: the element that captured the pointer gets the moves and the release", () => {
    const row = document.createElement("div");
    const other = document.createElement("div");
    document.body.append(row, other);
    document.elementFromPoint = () => row;
    const log: string[] = [];
    // jsdom has no PointerEvent, so its events carry no pointerId; the browser gives PAD_POINTER.
    row.addEventListener("pointerdown", (e) => row.setPointerCapture((e as PointerEvent).pointerId ?? PAD_POINTER));
    row.addEventListener("pointermove", (e) => log.push(`move ${e.buttons} ${e.clientX > 0}`));
    row.addEventListener("pointerup", () => log.push("up"));
    other.addEventListener("pointermove", () => log.push("other"));
    hold(BUTTON.south, true); hold(BUTTON.south, false);
    log.length = 0;

    hold(BUTTON.south, true);
    expect(row.hasPointerCapture(PAD_POINTER)).toBe(true);
    document.elementFromPoint = () => other;
    axes[0] = 1;
    tick();
    axes[0] = 0;
    hold(BUTTON.south, false);
    const held = log.slice(0, log.indexOf("up") + 1);
    expect(held).toContain("move 1 true");
    expect(held).not.toContain("other");
    expect(held.at(-1)).toBe("up");
    expect(row.hasPointerCapture(PAD_POINTER)).toBe(false);
  });

  it("gives the mouse back when a real mouse moves", () => {
    document.elementFromPoint = () => document.body;
    hold(BUTTON.south, true); hold(BUTTON.south, false);
    expect(padState().active).toBe(true);
    // A real mouse event is trusted; a test can't make one, so this checks the synthetic ones are ignored.
    window.dispatchEvent(new MouseEvent("pointermove", { movementX: 5 } as MouseEventInit));
    expect(padState().active).toBe(true);
  });
});
