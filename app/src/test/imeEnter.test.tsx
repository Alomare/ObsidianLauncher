import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const go = vi.hoisted(() => vi.fn());
vi.mock("../nav", async (real) => ({ ...(await real<typeof import("../nav")>()), go }));
import { WikiPage } from "../WikiPage";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root;
beforeEach(() => {
  go.mockClear();
  Element.prototype.scrollTo ??= () => {}; // jsdom has no scrolling
  const host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  act(() => root.render(<WikiPage />));
});
afterEach(() => {
  act(() => root.unmount());
  document.body.replaceChildren();
});

const enter = (box: HTMLInputElement, isComposing: boolean) =>
  act(() => box.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", isComposing, bubbles: true, cancelable: true })));

describe("Enter in the wiki search with an IME (Chinese input)", () => {
  it("leaves the Enter that finishes a word to the IME, and opens the first result on the next one", () => {
    const box = document.querySelector<HTMLInputElement>(".wiki-search input")!;
    expect(box).not.toBeNull();
    enter(box, true);
    expect(go).not.toHaveBeenCalled();
    enter(box, false);
    expect(go).toHaveBeenCalledWith("wiki", expect.any(String));
  });
});
