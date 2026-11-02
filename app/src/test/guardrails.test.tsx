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

const snap = (consent: boolean, blockAchievements: boolean, freezeProgress: boolean) =>
  ({ profile: "Fun", consent, blockAchievements, freezeProgress }) as unknown as Snapshot;

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

  it("a_switch_sends_its_own_command_with_the_opposite_value", () => {
    const switches = show(snap(false, false, true));
    act(() => switches[1].click());
    act(() => switches[2].click());
    expect(run.mock.calls).toEqual([["set_block_achievements", { on: true }], ["set_freeze_progress", { on: false }]]);
  });

  it("the_count_is_of_what_protects_the_account", () => {
    expect(guardsOn(snap(true, false, false))).toBe(0);
    expect(guardsOn(snap(false, true, true))).toBe(2);
    expect(guardrailsSummary(snap(true, true, false)).split(". ")).toHaveLength(3);
  });
});
