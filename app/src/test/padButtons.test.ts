import { describe, expect, it } from "vitest";
import { padButtons } from "../mods/padButtons";

describe("padButtons", () => {
  it("finds the buttons a label names, in order", () => {
    expect(padButtons("Both sticks pressed in (L3 + R3)")).toEqual(["l3", "r3"]);
    expect(padButtons("Back + right stick pressed in (R3)")).toEqual(["back", "r3"]);
    expect(padButtons("LB + RB + Back")).toEqual(["lb", "rb", "back"]);
    expect(padButtons("Start")).toEqual(["start"]);
    expect(padButtons("D-pad Up + RT")).toEqual(["dpad_up", "rt"]);
  });

  it("takes a single letter as a button only in a plain combination", () => {
    expect(padButtons("A + B")).toEqual(["a", "b"]);
    expect(padButtons("Y")).toEqual(["y"]);
    expect(padButtons("Press A to start")).toEqual(["start"]);
    expect(padButtons("Hold a button")).toBeNull();
  });

  it("is plain text when nothing is named, then tries the id", () => {
    expect(padButtons("Off")).toBeNull();
    expect(padButtons("Both sticks", "thumbs")).toBeNull();
    expect(padButtons("Shoulders", "lb_rb_back")).toEqual(["lb", "rb", "back"]);
    expect(padButtons("Pad", "dpad_left")).toEqual(["dpad_left"]);
  });
});
