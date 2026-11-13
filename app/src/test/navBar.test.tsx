import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Snapshot } from "../types";

let choices: unknown = { playing: null, wars: [], saves: 1 };
vi.mock("../api", () => ({ call: async () => choices, run: async () => {} }));
const { NavBar } = await import("../NavBar");
const { reloadChoices } = await import("../commander/choices");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | undefined;
afterEach(() => {
  act(() => root?.unmount());
  document.body.replaceChildren();
});

const snap = { profile: "Default", game: "found", modded: false, blockPublic: false, blockAchievements: false, freezeProgress: false, hideClientMods: false } as unknown as Snapshot;

async function show() {
  await act(async () => { await reloadChoices(); });
  const box = document.createElement("div");
  document.body.append(box);
  root = createRoot(box);
  await act(async () => root!.render(<NavBar tab="home" onTab={() => {}} snap={snap} busy={false} updateFound={false} wave={null} onWaveEnd={() => {}} onPlay={() => {}} />));
  await act(async () => { await reloadChoices(); });
  return [...document.querySelectorAll<HTMLButtonElement>(".actions .play-split > button")].map((b) => b.className);
}

describe("nav bar", () => {
  it("joins_guardrails_to_the_left_of_play", async () => {
    choices = { playing: null, wars: [], saves: 1 };
    const parts = await show();
    expect(parts).toHaveLength(2);
    expect(parts[0]).toContain("guard");
    expect(parts[1]).toContain("play");
  });

  it("puts_the_choice_button_on_the_right_when_there_is_a_choice", async () => {
    choices = { playing: null, wars: [], saves: 2 };
    const parts = await show();
    expect(parts).toHaveLength(3);
    expect(parts[0]).toContain("guard");
    expect(parts[1]).toContain("play");
    expect(parts[2]).toContain("play-more");
  });
});
