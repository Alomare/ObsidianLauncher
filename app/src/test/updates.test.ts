import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
const check = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check }));

const notesOf = (u: { phase: string }) => ("notes" in u ? u.notes : undefined);

const load = async () => {
  vi.resetModules();
  return import("../updates");
};

describe("the notes of an update", () => {
  beforeEach(() => {
    invoke.mockReset();
    check.mockReset();
    check.mockResolvedValue({ version: "1.0.1", body: "From latest.json" });
  });

  it("follow the text on GitHub, so an edit there shows", async () => {
    invoke.mockResolvedValue("Edited on GitHub\n");
    const { checkForUpdate, updateState } = await load();
    await checkForUpdate();
    await vi.waitFor(() => expect(notesOf(updateState())).toBe("Edited on GitHub"));
    expect(invoke).toHaveBeenCalledWith("release_notes", { version: "1.0.1" });
  });

  it("stay as latest.json has them when GitHub can't be read", async () => {
    invoke.mockRejectedValue({ code: "UpdateNetwork", message: "no network" });
    const { checkForUpdate, updateState } = await load();
    await checkForUpdate();
    await vi.waitFor(() => expect(invoke).toHaveBeenCalled());
    await Promise.resolve();
    expect(notesOf(updateState())).toBe("From latest.json");
    expect(updateState().phase).toBe("found");
  });

  it("stay as latest.json has them when GitHub has no text", async () => {
    invoke.mockResolvedValue(null);
    const { checkForUpdate, updateState } = await load();
    await checkForUpdate();
    await vi.waitFor(() => expect(invoke).toHaveBeenCalled());
    await Promise.resolve();
    expect(notesOf(updateState())).toBe("From latest.json");
  });
});

describe("the link to a mod's changelog", () => {
  it("is the releases page of a GitHub repository, and nothing elsewhere", async () => {
    const { changelogUrl } = await import("../mods/modUpdates");
    expect(changelogUrl("https://github.com/me/mod")).toBe("https://github.com/me/mod/releases");
    expect(changelogUrl("https://github.com/me/mod.git")).toBe("https://github.com/me/mod/releases");
    expect(changelogUrl("https://github.com/me/mod/tree/main")).toBe("https://github.com/me/mod/releases");
    expect(changelogUrl("https://nexusmods.com/x")).toBeNull();
    expect(changelogUrl(null)).toBeNull();
  });
});

describe("the changelog", () => {
  const remote = [
    { version: "1.0.10", date: "2026-11-02T00:00:00Z", notes: "Ten", url: "u10" },
    { version: "1.0.2", date: "2026-11-01T00:00:00Z", notes: "  ", url: "u2" },
  ];
  const local = [{ version: "1.0.2", notes: "Shipped" }, { version: "1.0.1", notes: "Old" }];

  it("puts new versions first and marks the installed one", async () => {
    const { changelog } = await load();
    expect(changelog(remote, "1.0.2", local).map((e) => [e.version, e.when])).toEqual([["1.0.10", "new"], ["1.0.2", "installed"], ["1.0.1", "older"]]);
  });

  it("keeps the shipped notes when GitHub has no text", async () => {
    const { changelog } = await load();
    expect(changelog(remote, "1.0.2", local)[1].notes).toBe("Shipped");
  });

  it("leaves out notes for a version that isn't released yet", async () => {
    const { changelog } = await load();
    expect(changelog([], "1.0.1", [{ version: "1.1.0", notes: "Draft" }, ...local]).map((e) => e.version)).toEqual(["1.0.1"]);
  });

  it("reads without GitHub, and has the versions this build shipped with", async () => {
    const { changelog } = await load();
    expect(changelog([], "1.0.1").map((e) => e.version)).toContain("1.0.1");
  });
});
