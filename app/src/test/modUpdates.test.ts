import { act, createElement } from "react";
import { createRoot } from "react-dom/client";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { t } from "../i18n";
import { checkMessage, checkModUpdates, offered, updateMod, updateMods, useModUpdates, type ModUpdates } from "../mods/modUpdates";

const invoke = vi.hoisted(() => vi.fn());
const notify = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../notices", () => ({ notify }));
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const names = new Map([["a.mod", "Mod A"], ["b.mod", "Mod B"]]);
const all = [...names.keys()];
const failure = { code: "UpdateNetwork", message: "GitHub answered 403" };

describe("offered", () => {
  it("offers a version the mod doesn't have yet", () => {
    expect(offered({ "a.mod": "1.1.0" }, { id: "a.mod", version: "1.0.0" })).toBe("1.1.0");
  });
  it("offers nothing to other mods, or once that version is in", () => {
    expect(offered({ "a.mod": "1.1.0" }, { id: "b.mod", version: "1.0.0" })).toBeNull();
    expect(offered({ "a.mod": "1.1.0" }, { id: "a.mod", version: "1.1.0" })).toBeNull();
  });
});

describe("a check", () => {
  beforeEach(() => {
    invoke.mockReset();
    notify.mockReset();
  });

  it("counts what it found", () => {
    expect(checkMessage(0)).toBe(t("app.modUpdates.none"));
    expect(checkMessage(1)).toBe(t("app.modUpdates.one"));
    expect(checkMessage(3)).toBe(t("app.modUpdates.many", 3));
  });

  it("names what it checked when that wasn't everything", () => {
    expect(checkMessage(0, "Mod A")).toBe(t("app.modUpdates.noneFor", "Mod A"));
    expect(checkMessage(1, "Mod A")).toBe(t("app.modUpdates.one"));
  });

  it("says how many mods have updates", async () => {
    invoke.mockResolvedValue([{ id: "a.mod", version: "1.1.0", error: null }, { id: "b.mod", version: null, error: null }]);
    await checkModUpdates(all, names);
    expect(invoke).toHaveBeenCalledWith("check_mod_updates", { ids: all, fresh: true });
    expect(notify.mock.calls).toEqual([["info", t("app.modUpdates.one")]]);
  });

  it("asks only about the mods it was given, and leaves the others' results alone", async () => {
    invoke.mockResolvedValueOnce([{ id: "a.mod", version: "1.1.0", error: null }, { id: "b.mod", version: "2.0.0", error: null }]);
    await checkModUpdates(all, names);
    invoke.mockResolvedValueOnce([{ id: "b.mod", version: null, error: null }]);
    await checkModUpdates(["b.mod"], names, { what: "Mod B" });
    expect(invoke).toHaveBeenLastCalledWith("check_mod_updates", { ids: ["b.mod"], fresh: true });
    expect(notify).toHaveBeenLastCalledWith("info", t("app.modUpdates.noneFor", "Mod B"));
    let seen: ModUpdates | undefined;
    const Spy = () => {
      seen = useModUpdates();
      return null;
    };
    act(() => createRoot(document.createElement("div")).render(createElement(Spy)));
    expect(seen?.found).toEqual({ "a.mod": "1.1.0" });
  });

  it("names the mod that failed, and still reports the others", async () => {
    invoke.mockResolvedValue([{ id: "a.mod", version: null, error: null }, { id: "b.mod", version: null, error: failure }]);
    await checkModUpdates(all, names);
    expect(notify.mock.calls[0]).toEqual(["info", t("app.modUpdates.none")]);
    expect(notify.mock.calls[1][0]).toBe("error");
    expect(notify.mock.calls[1][1]).toContain("Mod B");
  });

  it("doesn't call mods up to date when no check got through", async () => {
    invoke.mockResolvedValue([{ id: "a.mod", version: null, error: failure }]);
    await checkModUpdates(all, names);
    expect(notify.mock.calls.map((c) => c[0])).toEqual(["error"]);
  });

  it("reports the rate limit once, however many mods it stopped", async () => {
    const limited = { code: "UpdateLimited", message: "limit" };
    invoke.mockResolvedValue([{ id: "a.mod", version: null, error: limited }, { id: "b.mod", version: null, error: limited }]);
    await checkModUpdates(all, names);
    expect(notify.mock.calls.filter((c) => c[0] === "error")).toHaveLength(1);
  });

  it("at startup, only speaks when it finds something", async () => {
    invoke.mockResolvedValue([{ id: "a.mod", version: null, error: null }, { id: "b.mod", version: null, error: failure }]);
    await checkModUpdates(all, names, { startup: true });
    expect(invoke).toHaveBeenCalledWith("check_mod_updates", { ids: all, fresh: false });
    expect(notify).not.toHaveBeenCalled();
    invoke.mockRejectedValue("offline");
    await checkModUpdates(all, names, { startup: true });
    expect(notify).not.toHaveBeenCalled();
    invoke.mockReset();
    invoke.mockResolvedValue([{ id: "a.mod", version: "1.1.0", error: null }]);
    await checkModUpdates(all, names, { startup: true });
    expect(notify.mock.calls).toEqual([["info", t("app.modUpdates.one")]]);
  });

  it("can run again after a failure", async () => {
    invoke.mockRejectedValueOnce("boom").mockResolvedValue([]);
    await checkModUpdates(all, names);
    await checkModUpdates(all, names);
    expect(invoke).toHaveBeenCalledTimes(2);
  });
});

describe("an update", () => {
  beforeEach(() => {
    invoke.mockReset();
    notify.mockReset();
  });

  it("shows the old and new version", async () => {
    invoke.mockResolvedValue({
      snapshot: { mods: [] },
      results: [{ file: "a.mod", ok: true, kind: "updated", name: { en: "Mod A" }, version: "1.1.0", oldVersion: "1.0.0", error: null }],
    });
    await updateMod("a.mod", "Mod A");
    expect(invoke).toHaveBeenCalledWith("update_mod", { id: "a.mod" });
    expect(notify.mock.calls).toEqual([["info", t("app.import.updated", "Mod A", "1.0.0", "1.1.0")]]);
  });

  it("updates several, one after the other", async () => {
    const started: string[] = [];
    invoke.mockImplementation(async (_cmd, args) => {
      started.push(args.id);
      return { snapshot: { mods: [] }, results: [] };
    });
    await updateMods(["a.mod", "b.mod"], names);
    expect(started).toEqual(["a.mod", "b.mod"]);
  });

  it("says why it failed", async () => {
    invoke.mockRejectedValue({ code: "UpdateMismatch", message: "no" });
    await updateMod("a.mod", "Mod A");
    expect(notify.mock.calls[0][0]).toBe("error");
    expect(notify.mock.calls[0][1]).toContain("Mod A");
  });
});
