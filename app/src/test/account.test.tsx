// The Account tab and the account icon: signed out offers the sign-in, signed in shows the login and a
// check on the nav icon, and the reports list counts unread replies.
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Account, Report } from "../github";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke, convertFileSrc: (p: string) => p }));
vi.mock("../opener", () => ({ openUrl: vi.fn(() => Promise.resolve()) }));
const { AccountPage } = await import("../account/AccountPage");
const { NavBar } = await import("../NavBar");
const { startGitHub } = await import("../github");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const ACCOUNT: Account = { login: "octo", avatar: null, scopes: ["public_repo"], syncGist: null };
const REPORT: Report = {
  repo: "o/hud", number: 7, title: "Crash on start", modId: "hud", modName: "Better HUD", url: "https://github.com/o/hud/issues/7",
  created: "2026-11-10T10:00:00Z", state: "open", comments: 3, seen: 1, updated: "2026-11-12T10:00:00Z",
};

let root: Root | undefined;
afterEach(() => {
  act(() => root?.unmount());
  document.body.replaceChildren();
});

async function mount(node: React.ReactNode) {
  const host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root!.render(node));
  return host;
}

const nav = () => <NavBar tab="mods" onTab={() => {}} snap={null} busy={false} updateFound={false} wave={null} onWaveEnd={() => {}} onPlay={() => {}} />;

describe("Account", () => {
  it("offers the sign-in while signed out, with no check on the nav icon", async () => {
    invoke.mockResolvedValue(undefined);
    const host = await mount(<AccountPage tab="github" onTab={() => {}} />);
    expect([...host.querySelectorAll("button")].some((b) => b.textContent === "Sign in with GitHub")).toBe(true);
    expect(host.querySelector(".game-card b")?.textContent).toBe("Not signed in");
    act(() => root!.unmount());
    const bar = await mount(nav());
    expect(bar.querySelector(".tab-account")?.getAttribute("aria-label")).toBe("GitHub account");
    expect(bar.querySelector(".tab-check")).toBeNull();
  });

  it("shows the login and the nav check once signed in, and the unread count of a report", async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "github_account" || cmd === "github_verify") return ACCOUNT;
      if (cmd === "github_reports" || cmd === "github_reports_refresh") return [REPORT];
      return undefined;
    });
    await act(async () => { await startGitHub(); });
    const host = await mount(<AccountPage tab="github" onTab={() => {}} />);
    expect(host.querySelector(".acc-login")?.textContent).toBe("octo");
    expect(host.querySelectorAll(".acc-granted").length).toBe(1);
    expect([...host.querySelectorAll(".acc-access-text b")].map((b) => b.textContent)).toEqual(["Public repository access", "Gist access", "Private repository access"]);
    expect(host.querySelector(".acc-avatar.letter")?.textContent).toBe("O");
    expect([...host.querySelectorAll("button")].filter((b) => b.textContent === "Allow").length).toBe(2);
    act(() => root!.unmount());

    const bar = await mount(nav());
    expect(bar.querySelector(".tab-account .tab-check")).not.toBeNull();
    expect(bar.querySelector(".tab-account")?.getAttribute("aria-label")).toBe("Signed in as octo");
    act(() => root!.unmount());

    const list = await mount(<AccountPage tab="reports" onTab={() => {}} />);
    expect(list.querySelector(".acc-report .acc-report-title b")?.textContent).toBe("Crash on start");
    expect(list.querySelector(".acc-report-mod")?.textContent).toBe("Better HUD");
    expect(list.querySelector(".acc-report.new")).not.toBeNull();
    expect(list.querySelector(".acc-unread")?.textContent).toBe("2");
    expect(list.querySelector(".acc-state")?.textContent).toBe("Open");
  });

  it("asks GitHub for a sized avatar and falls back to the first letter when it fails to load", async () => {
    const withPic = { ...ACCOUNT, avatar: "https://avatars.githubusercontent.com/u/1?v=4" };
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "github_account" || cmd === "github_verify") return withPic;
      if (cmd === "github_reports" || cmd === "github_reports_refresh") return [];
      return undefined;
    });
    await act(async () => { await startGitHub(); });
    const host = await mount(<AccountPage tab="github" onTab={() => {}} />);
    const img = host.querySelector<HTMLImageElement>("img.acc-avatar")!;
    expect(new URL(img.src).searchParams.get("s")).toBe("96");
    await act(async () => { img.dispatchEvent(new Event("error")); });
    expect(host.querySelector("img.acc-avatar")).toBeNull();
    expect(host.querySelector(".acc-avatar.letter")?.textContent).toBe("O");
  });
});
