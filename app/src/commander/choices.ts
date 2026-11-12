// What the Play button starts: the official war or a custom war (Commander), with the mods of the last
// deploy or none. The wars come from `commander_choices`, read again whenever something may have
// changed them; the Play choice dialog sets the war and the save.
import { useSyncExternalStore } from "react";
import { call } from "../api";
import { t } from "../i18n";

export type WarChoice = {
  id: string; name: string; host: "local" | "lan"; base?: string; computer?: string; save?: string;
  model?: string; state?: "running" | "paused" | "won" | "lost";
  /** Only the Sandbox made for the war plays it. */
  ownSaveOnly?: boolean;
  /** Steam achievements are not blocked while the war is played. */
  allowAchievements?: boolean;
};
/** `playing`: the war the next launch plays, which needs a Sandbox the war accepts to be active. */
/** `saves`: how many saves the account has. */
export type Choices = { playing: string | null; wars: WarChoice[]; saves?: number };

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

/** The Play label: PLAY, then the war name for a custom war. Vanilla or modded shows in the button
 *  color only; `text` says it in words for the tooltip and the screen reader. */
export function playLabel(war: string | null, modded: boolean): { play: string; war: string | null; text: string } {
  const text = war ? t(modded ? "app.play.labelWarModded" : "app.play.labelWarVanilla", war) : t(modded ? "app.play.labelModded" : "app.play.labelVanilla");
  return { play: t("app.action.play"), war, text };
}
