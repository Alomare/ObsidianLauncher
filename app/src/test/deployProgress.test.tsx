import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

type Progress = { percent: number; mod: string | null; stage: string };
const sent = vi.hoisted(() => ({ fn: null as null | ((e: { payload: Progress }) => void) }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: (_: string, f: (e: { payload: Progress }) => void) => { sent.fn = f; return Promise.resolve(() => {}); },
}));
const invoked = vi.hoisted(() => [] as string[]);
vi.mock("@tauri-apps/api/core", () => ({ invoke: (cmd: string) => { invoked.push(cmd); return Promise.resolve(); } }));
const { DeployProgress } = await import("../DeployProgress");
const { startTask } = await import("../busy");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("the deploy window", () => {
  let root: Root;
  let end: () => void;
  const box = () => document.querySelector(".deploy-now");
  const send = (p: Progress) => act(() => sent.fn!({ payload: p }));
  const pass = (ms: number) => act(() => { vi.advanceTimersByTime(ms); });

  beforeEach(() => {
    vi.useFakeTimers();
    root = createRoot(document.body.appendChild(document.createElement("div")));
    act(() => root.render(<DeployProgress />));
    act(() => { end = startTask("Deploying", true); });
  });
  afterEach(() => {
    act(() => root.unmount());
    vi.useRealTimers();
    document.body.innerHTML = "";
  });

  it("never shows for a quick deploy", () => {
    send({ percent: 0, mod: null, stage: "preparing" });
    pass(200);
    send({ percent: 100, mod: null, stage: "done" });
    act(() => end());
    pass(2000);
    expect(box()).toBeNull();
  });

  it("shows the mod, the stage and the percent once a deploy takes a while", () => {
    send({ percent: 10, mod: "Big Mod", stage: "sound banks" });
    pass(400);
    expect(box()!.textContent).toContain("Big Mod");
    expect(box()!.textContent).toContain("Sound banks");
    expect(box()!.textContent).toContain("10 %");
    act(() => end());
  });

  it("stays up long enough to read, then goes", () => {
    send({ percent: 10, mod: null, stage: "resources" });
    pass(400);
    act(() => end());
    pass(100);
    expect(box()).not.toBeNull();
    pass(400);
    expect(box()).toBeNull();
  });

  it("asks the deploy to stop and says so until it ends", () => {
    send({ percent: 50, mod: null, stage: "writing patches" });
    pass(400);
    const button = () => document.querySelector<HTMLButtonElement>(".deploy-progress .cancel")!;
    expect(button().textContent).toBe("Cancel");
    expect(button().querySelector("svg")).not.toBeNull();
    act(() => button().click());
    expect(invoked).toContain("cancel_deploy");
    expect(button().disabled).toBe(true);
    expect(button().textContent).toBe("Cancelling");
    act(() => end());
    pass(1000);
    expect(box()).toBeNull();
  });

  it("offers no cancel once the deploy is done", () => {
    send({ percent: 100, mod: null, stage: "done" });
    pass(400);
    expect(document.querySelector<HTMLButtonElement>(".deploy-progress .cancel")!.disabled).toBe(true);
    act(() => end());
  });
});