// Controller button pictures (Kenney's Input Prompts, CC0, in assets/pad), and which set a controller gets.
// Buttons go by position, as the browser's standard mapping reports them: "south" is the bottom face
// button, A on Xbox, Cross on PlayStation and B on Nintendo.

export type PadKind = "xbox" | "playstation" | "nintendo" | "steamdeck";
export type PadRole = "south" | "east" | "west" | "north" | "lb" | "rb" | "lt" | "rt" | "start" | "lstick" | "rstick";
export type GlyphSetting = "auto" | "xbox" | "playstation" | "nintendo";

const URLS = import.meta.glob("../assets/pad/*/*.svg", { eager: true, query: "?url", import: "default" }) as Record<string, string>;

export function glyphUrl(kind: PadKind, role: PadRole): string {
  return URLS[`../assets/pad/${kind}/${role}.svg`] ?? "";
}

/** The pad a controller is, from the id the browser gives it: the USB vendor when the id names one
    ("... Vendor: 054c Product: 0ce6" in Chromium, "054c-0ce6-..." elsewhere), else words in the name.
    An Xbox pad through XInput names no vendor, and anything unknown gets Xbox pictures, the most common layout. */
export function kindOf(id: string): PadKind {
  const vendor = (/vendor:\s*([0-9a-f]{4})/i.exec(id) ?? /^([0-9a-f]{4})-[0-9a-f]{4}-/i.exec(id))?.[1]?.toLowerCase();
  if (vendor === "054c") return "playstation";
  if (vendor === "057e") return "nintendo";
  if (vendor === "28de" && /deck/i.test(id)) return "steamdeck";
  if (vendor === "045e") return "xbox";
  if (/dualsense|dualshock|playstation|ps[345]\b/i.test(id)) return "playstation";
  if (/nintendo|pro controller|joy-?con/i.test(id)) return "nintendo";
  if (/steam deck/i.test(id)) return "steamdeck";
  return "xbox";
}

/** The pictures to show: the setting, or with Auto the kind of the controller found. */
export function shownKind(setting: GlyphSetting, found: PadKind | null): PadKind {
  return setting === "auto" ? found ?? "xbox" : setting;
}
