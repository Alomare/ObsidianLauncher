import { afterEach, describe, expect, it, vi } from "vitest";
import { MAX_AGE_MS, addTo, freshOnly, type Logged } from "../noticeLog";

describe("addTo", () => {
  it("appends with the next id", () => {
    const a = addTo([], "info", "Installed A", 1000);
    const b = addTo(a, "error", "B failed", 2000);
    expect(b.map((n) => [n.id, n.kind, n.text])).toEqual([[1, "info", "Installed A"], [2, "error", "B failed"]]);
  });
  it("folds the same text within five seconds into one entry", () => {
    const a = addTo(addTo([], "info", "Saved", 1000), "info", "Saved", 4000);
    expect(a).toEqual([{ id: 1, at: 4000, kind: "info", text: "Saved" }]);
    expect(addTo(a, "info", "Saved", 9001)).toHaveLength(2);
  });
  it("keeps the last 100", () => {
    let log: Logged[] = [];
    for (let i = 0; i < 105; i++) log = addTo(log, "info", `n${i}`, i * 10_000);
    expect(log).toHaveLength(100);
    expect(log[0].text).toBe("n5");
    expect(log[99].id).toBe(105);
  });
});

const entry = (id: number, at: number): Logged => ({ id, at, kind: "info", text: `n${id}` });

describe("freshOnly", () => {
  it("drops_notices_a_day_old_or_older", () => {
    const now = 10 * MAX_AGE_MS;
    const log = [entry(1, now - MAX_AGE_MS - 1), entry(2, now - MAX_AGE_MS), entry(3, now - MAX_AGE_MS + 1), entry(4, now)];
    expect(freshOnly(log, now).map((n) => n.id)).toEqual([3, 4]);
  });
  it("returns_the_same_array_when_nothing_aged_out", () => {
    const log = [entry(1, 5000)];
    expect(freshOnly(log, 6000)).toBe(log);
  });
});

describe("the kept log", () => {
  const kept = () => (JSON.parse(localStorage.getItem("obsidian.notifications") ?? "[]") as Logged[]).map((n) => n.text);
  afterEach(() => {
    vi.useRealTimers();
    localStorage.clear();
    vi.resetModules();
  });

  it("drops_notices_older_than_a_day_when_it_loads", async () => {
    const now = Date.now();
    localStorage.setItem("obsidian.notifications", JSON.stringify([{ ...entry(1, now - 25 * 3600_000), text: "old" }, { ...entry(2, now - 3600_000), text: "recent" }]));
    vi.resetModules();
    await import("../noticeLog");
    expect(kept()).toEqual(["recent"]);
  });

  it("drops_notices_as_time_passes", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-11-03T08:00:00Z"));
    vi.resetModules();
    const { logNotice } = await import("../noticeLog");
    logNotice("info", "first");
    vi.advanceTimersByTime(3600_000);
    logNotice("info", "second");
    expect(kept()).toEqual(["first", "second"]);
    vi.advanceTimersByTime(MAX_AGE_MS - 3600_000 + 1000);
    expect(kept()).toEqual(["second"]);
    vi.advanceTimersByTime(3600_000);
    expect(kept()).toEqual([]);
  });
});
