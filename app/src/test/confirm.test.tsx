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
const buttons = () => [...document.querySelectorAll(".modal-foot button.btn")].map((b) => b.textContent);

describe("Confirm", () => {
  it("has two buttons, Cancel and the action, by default", () => {
    show(<Confirm title="Purge?" text="Removes every patch." confirm="Purge" onConfirm={() => {}} onCancel={() => {}} />);
    expect(buttons()).toEqual(["Cancel", "Purge"]);
    expect(document.querySelector(".modal-foot .foot-link")).toBeNull();
  });

  it("the update prompt: Later and Install as buttons, the changelog only as a link, no notes inside", () => {
    const install = vi.fn(), later = vi.fn(), changelog = vi.fn();
    show(
      <Confirm title="Obsidian 9.9.9 is out" text="Install it now?" confirm="Install" cancel="Later" tone="go"
        onConfirm={install} onCancel={later} link={{ label: "Changelog", onClick: changelog }} />,
    );
    expect(buttons()).toEqual(["Later", "Install"]);
    expect(document.querySelector(".modal .md")).toBeNull();
    const link = document.querySelector<HTMLButtonElement>(".modal-foot .foot-link")!;
    expect(link.textContent).toBe("Changelog");
    act(() => link.click());
    expect(changelog).toHaveBeenCalledOnce();
    act(() => document.querySelectorAll<HTMLButtonElement>(".modal-foot button.btn")[1].click());
    expect(install).toHaveBeenCalledOnce();
    act(() => document.querySelectorAll<HTMLButtonElement>(".modal-foot button.btn")[0].click());
    expect(later).toHaveBeenCalledOnce();
  });
});
