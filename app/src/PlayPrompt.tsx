// The question before Play starts the game with no mods deployed, which would be vanilla. Don't ask again
// is kept only with Yes; Settings, System turns the question back on.
import { useState } from "react";
import { Confirm } from "./Confirm";
import { CheckIcon } from "./Icons";
import { t } from "./i18n";
import type { AppSettings, Snapshot } from "./types";

/** Whether Play asks first. "Deployed" is what the last deploy wrote into the game, as the app keeps
 *  track of it, not the switches in the list: Play starts the game with the last deploy. A Sandbox
 *  cannot run vanilla, and Play asks for the deploy it needs instead. */
export const asksBeforePlay = (snap: Snapshot, settings: AppSettings) => !snap.deployed && settings.askVanillaPlay && !snap.sandbox;

export function PlayPrompt({ onPlay, onCancel }: { onPlay: (dontAsk: boolean) => void; onCancel: () => void }) {
  const [dontAsk, setDontAsk] = useState(false);
  return (
    <Confirm title={t("app.action.play")} text={t("app.play.vanilla")} confirm={t("app.yes")} cancel={t("app.no")} tone="go"
      onConfirm={() => onPlay(dontAsk)} onCancel={onCancel}>
      <button role="checkbox" aria-checked={dontAsk} className="check-line" onClick={() => setDontAsk(!dontAsk)}>
        <span className={dontAsk ? "ck on" : "ck"} aria-hidden="true">{dontAsk && <CheckIcon />}</span>
        {t("app.play.dontAsk")}
      </button>
    </Confirm>
  );
}
