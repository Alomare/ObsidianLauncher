// What the Play button starts: the official war or a custom war (Commander), with the mods of the last
// deploy or none. The wars come from `commander_choices`, read again whenever something may have
// changed them; the Play choice dialog sets the war and the save.
import { useSyncExternalStore } from "react";
import { call } from "../api";
import { t } from "../i18n";

export type WarChoice = {
  id: string; name: string; host: "local" | "lan"; base?: string; computer?: string; save?: string;
  model?: string; state?: "running" | "paused" | "won" | "lost";
};
export type Choices = { playing: string | null; wars: WarChoice[] };

let choices: Choices | null = null;
const subs = new Set<() => void>();

/** Reads the wars again. A failure keeps what was there: the button then shows the last known war. */
export async function reloadChoices() {
  try {
    const c = await call<Choices>("commander_choices");
    if (c && Array.isArray(c.wars)) {
      choices = c;
      subs.forEach((f) => f());
    }
  } catch { /* the Play button works without Commander */ }
}

export const useChoices = () =>
  useSyncExternalStore((f) => { subs.add(f); return () => { subs.delete(f); }; }, () => choices);

/** The war the next launch plays, when it is a custom war this PC still knows. */
export const playingWar = (c: Choices | null) => (c?.playing ? c.wars.find((w) => w.id === c.playing) ?? null : null);

/** The Play label in parts, so the war name can be cut short alone: `before` and `after` go round
 *  it. Without a war, the whole label is `before`. */
export function playLabel(war: string | null, modded: boolean): { before: string; war: string | null; after: string } {
  if (!war) return { before: t(modded ? "app.play.labelModded" : "app.play.labelVanilla"), war: null, after: "" };
  const MARK = "\u0001";
  const [before, after = ""] = t(modded ? "app.play.labelWarModded" : "app.play.labelWarVanilla", MARK).split(MARK);
  return { before, war, after };
}

/** The whole label as one line, for the tooltip and the screen reader. */
export const playLabelText = (l: ReturnType<typeof playLabel>) => `${l.before}${l.war ?? ""}${l.after}`;
