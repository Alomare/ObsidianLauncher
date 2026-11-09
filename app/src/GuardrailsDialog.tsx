// The Guardrails dialog, opened from the shield in the navigation bar: the active profile's switches.
// One keeps other players' session mods out of a public lobby; two protect the player's own account:
// achievements and progression; one keeps client mods off the lobby list. All start off. The loader reads
// them when the game starts, so a change applies at the next launch.
import { useEffect } from "react";
import { createPortal } from "react-dom";
import { run } from "./api";
import { CloseIcon, ShieldIcon } from "./Icons";
import { t } from "./i18n";
import type { Snapshot } from "./types";

type Row = { key: "blockPublic" | "blockAchievements" | "freezeProgress" | "hideClientMods"; command: string; text: string };

export const GUARDRAILS: Row[] = [
  { key: "blockPublic", command: "set_block_public", text: "blockPublic" },
  { key: "blockAchievements", command: "set_block_achievements", text: "achievements" },
  { key: "freezeProgress", command: "set_freeze_progress", text: "freeze" },
  { key: "hideClientMods", command: "set_hide_client_mods", text: "hideClientMods" },
];

/** How many guardrails are on. */
export const guardsOn = (snap: Snapshot) => [snap.blockPublic, snap.blockAchievements, snap.freezeProgress].filter(Boolean).length;

/** One line per switch, for the button's tooltip. */
export function guardrailsSummary(snap: Snapshot): string {
  const state = (on: boolean, name: string) => t(`app.guardrails.state${name}${on ? "On" : "Off"}`);
  return [state(snap.blockPublic, "BlockPublic"), state(snap.blockAchievements, "Achievements"), state(snap.freezeProgress, "Freeze")].join(". ") + ".";
}

export function GuardrailsDialog({ snap, onClose }: { snap: Snapshot; onClose: () => void }) {
  useEffect(() => {
    const key = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [onClose]);
  // On the body, as every overlay is: inside the navigation bar, the Mods page's drawer would paint over it.
  return createPortal(
    <div className="modal-back" onPointerDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal guardrails" role="dialog" aria-modal="true" aria-label={t("app.guardrails.title")}>
        <div className="head">
          <h2 className="cap"><ShieldIcon filled={guardsOn(snap) > 0} />{t("app.guardrails.title")}</h2>
          <button className="bare close" onClick={onClose} aria-label={t("app.close")} data-tip={t("app.close")}><CloseIcon /></button>
        </div>
        <p className="muted lead">{t("app.guardrails.lead")}</p>
        <ul className="guard-list">
          {GUARDRAILS.map((r) => {
            const on = snap[r.key];
            return (
              <li key={r.key} className="guard-row">
                <div className="guard-text">
                  <span className="name">{t(`app.guardrails.${r.text}`)}</span>
                  <span className="muted">{t(`app.guardrails.${r.text}Hint`)}</span>
                </div>
                <button role="switch" aria-checked={on} aria-label={t(`app.guardrails.${r.text}`)} className={on ? "sw on" : "sw"}
                  onClick={() => run(r.command, { on: !on })} />
              </li>
            );
          })}
        </ul>
        <p className="muted guard-note">{t("app.guardrails.always")}</p>
      </div>
    </div>,
    document.body,
  );
}
