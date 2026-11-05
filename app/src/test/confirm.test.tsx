import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Confirm } from "../Confirm";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | undefined;
afterEach(() => {
  act(() => root?.unmount());
  document.body.replaceChildren();
});

function show(el: React.ReactElement) {
  const host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  act(() => root!.render(el));
}
const buttons = () => [...document.querySelectorAll(".modal-foot button")].map((b) => b.textContent);

describe("Confirm", () => {
  it("has two buttons, Cancel and the action, by default", () => {
    show(<Confirm title="Purge?" text="Removes every patch." confirm="Purge" onConfirm={() => {}} onCancel={() => {}} />);
    expect(buttons()).toEqual(["Cancel", "Purge"]);
  });

  it("the update prompt says Later, shows the notes inside, and installs or closes", () => {
    const install = vi.fn(), later = vi.fn();
    show(
      <Confirm className="update-ask" title="Obsidian 9.9.9 is out" text="Install it now?" confirm="Install" cancel="Later" tone="go" onConfirm={install} onCancel={later}>
        <div className="update-notes"><p>Fixed a thing.</p></div>
      </Confirm>,
    );
    expect(document.querySelector(".modal.confirm.update-ask")).not.toBeNull();
    expect(document.querySelector(".modal .update-notes")?.textContent).toBe("Fixed a thing.");
    expect(buttons()).toEqual(["Later", "Install"]);
    act(() => (document.querySelectorAll<HTMLButtonElement>(".modal-foot button")[1]).click());
    expect(install).toHaveBeenCalledOnce();
    act(() => (document.querySelectorAll<HTMLButtonElement>(".modal-foot button")[0]).click());
    expect(later).toHaveBeenCalledOnce();
  });
});
