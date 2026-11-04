import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Snapshot } from "../types";

const run = vi.fn();
vi.mock("../api", () => ({ run: (...a: unknown[]) => run(...a) }));
const { GuardrailsDialog, guardrailsSummary, guardsOn } = await import("../GuardrailsDialog");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | undefined;
afterEach(() => {
  act(() => root?.unmount());
  document.body.replaceChildren();
  run.mockReset();
});

const snap = (blockPublic: boolean, blockAchievements: boolean, freezeProgress: boolean) =>
  ({ profile: "Fun", blockPublic, blockAchievements, freezeProgress }) as unknown as Snapshot;

function show(s: Snapshot) {
  const box = document.createElement("div");
  document.body.append(box);
  root = createRoot(box);
  act(() => root!.render(<GuardrailsDialog snap={s} onClose={() => {}} />));
  return [...document.querySelectorAll<HTMLButtonElement>("button[role=switch]")];
}

describe("guardrails", () => {
  it("shows_the_profiles_three_switches_as_they_are", () => {
    const switches = show(snap(true, false, true));
    expect(switches.map((b) => b.getAttribute("aria-checked"))).toEqual(["true", "false", "true"]);
  });

  it("the_lead_says_the_current_profile_without_naming_it", () => {
    show(snap(true, false, true));
    const lead = document.querySelector(".lead")!.textContent!;
    expect(lead).toContain("current profile");
    expect(lead).not.toContain("Fun");
  });

  it("a_switch_sends_its_own_command_with_the_opposite_value", () => {
    const switches = show(snap(false, false, true));
    act(() => switches[1].click());
    act(() => switches[2].click());
    expect(run.mock.calls).toEqual([["set_block_achievements", { on: true }], ["set_freeze_progress", { on: false }]]);
  });

  it("blocking_others_sends_its_command", () => {
    const switches = show(snap(false, false, false));
    act(() => switches[0].click());
    expect(run.mock.calls).toEqual([["set_block_public", { on: true }]]);
  });

  it("the_count_is_of_the_guardrails_on", () => {
    expect(guardsOn(snap(false, false, false))).toBe(0);
    expect(guardsOn(snap(true, false, false))).toBe(1);
    expect(guardsOn(snap(false, true, true))).toBe(2);
    expect(guardrailsSummary(snap(true, true, false)).split(". ")).toHaveLength(3);
  });
});
