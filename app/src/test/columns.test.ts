import { afterEach, describe, expect, it } from "vitest";
import { COLUMN_KEY, DEFAULTS, MINS, MOD_MIN, drag, parseWidths, readWidths, reset, saveWidths, serialize } from "../mods/columns";

describe("parseWidths", () => {
  it("uses_the_defaults_when_nothing_is_saved", () => {
    expect(parseWidths(null)).toEqual(DEFAULTS);
    expect(parseWidths("")).toEqual(DEFAULTS);
  });
  it("ignores_damaged_values", () => {
    expect(parseWidths("{not json")).toEqual(DEFAULTS);
    expect(parseWidths("7")).toEqual(DEFAULTS);
    expect(parseWidths('{"version":"wide","scope":null}')).toEqual(DEFAULTS);
  });
  it("takes_each_column_on_its_own_and_keeps_it_in_range", () => {
    expect(parseWidths('{"version":9}')).toEqual({ ...DEFAULTS, version: 9 });
    expect(parseWidths('{"version":0.5,"scope":99}')).toEqual({ version: MINS.version, scope: 30 });
  });
});

describe("drag", () => {
  const start = { version: 7, scope: 3.2 };
  it("moving_the_version_line_left_widens_version_and_narrows_mod", () => {
    expect(drag(start, "version", -2, 20).version).toBe(9);
    expect(drag(start, "version", 2, 20).version).toBe(5);
  });
  it("version_stops_at_its_minimum", () => {
    expect(drag(start, "version", 50, 20).version).toBe(MINS.version);
  });
  it("version_stops_where_mod_would_drop_below_its_minimum", () => {
    // The Mod column is 10 rem wide and keeps MOD_MIN, so 10 - MOD_MIN rem can go to Version.
    expect(drag(start, "version", -50, 10).version).toBe(start.version + 10 - MOD_MIN);
  });
  it("a_narrow_mod_column_does_not_shrink_version_by_itself", () => {
    expect(drag(start, "version", 0, MOD_MIN - 2).version).toBe(start.version);
  });
  it("the_scope_line_trades_width_between_version_and_scope", () => {
    expect(drag(start, "scope", 0.5, 20)).toEqual({ version: 7.5, scope: 2.7 });
    expect(drag(start, "scope", -1, 20)).toEqual({ version: 6, scope: 4.2 });
  });
  it("the_scope_line_stops_at_both_minimums", () => {
    expect(drag(start, "scope", 50, 20)).toEqual({ version: 7.6, scope: MINS.scope });
    expect(drag(start, "scope", -50, 20)).toEqual({ version: MINS.version, scope: 6.5 });
  });
});

describe("reset", () => {
  const wide = { version: 11, scope: 5 };
  it("the_version_line_resets_version", () => expect(reset(wide, "version")).toEqual({ version: DEFAULTS.version, scope: 5 }));
  it("the_scope_line_resets_the_columns_beside_it", () => expect(reset(wide, "scope")).toEqual(DEFAULTS));
});

describe("kept widths", () => {
  afterEach(() => localStorage.clear());

  it("only_columns_away_from_the_default_are_kept", () => {
    expect(serialize(DEFAULTS)).toBeNull();
    expect(JSON.parse(serialize({ version: 9, scope: DEFAULTS.scope })!)).toEqual({ version: 9 });
  });
  it("saved_widths_come_back_and_defaults_clear_the_key", () => {
    saveWidths({ version: 9, scope: 4 });
    expect(readWidths()).toEqual({ version: 9, scope: 4 });
    saveWidths(DEFAULTS);
    expect(localStorage.getItem(COLUMN_KEY)).toBeNull();
    expect(readWidths()).toEqual(DEFAULTS);
  });
});
