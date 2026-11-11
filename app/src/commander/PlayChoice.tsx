// The Play choice: the right part of the split Play button opens it when there is more than one war to
// play. The official war comes first, then the custom wars this PC hosts (running or paused) and the
// LAN wars it joined; beside them, the saves the chosen war can boot. Select makes it what Play starts.
import { useCallback, useEffect, useMemo, useState } from "react";
import { createPortal } from "react-dom";
import { call, errorText, setSnapshot } from "../api";
import { withTask } from "../busy";
import { Confirm } from "../Confirm";
import { CheckIcon } from "../Icons";
import { t } from "../i18n";
import { notify } from "../notices";
import type { SaveList, SaveRow } from "../saves/types";
import type { Snapshot } from "../types";
import { reloadChoices, type Choices, type WarChoice } from "./choices";
import "./play.css";

type Done = { snapshot: Snapshot };

/** The saves the official war can boot: all but the ones made for custom wars. */
export const vanillaSaves = (list: SaveList, wars: WarChoice[]) => {
  const warSaves = new Set(wars.map((w) => w.save).filter(Boolean));
  return list.saves.filter((s) => !warSaves.has(s.id));
};

/** The save the official war would boot when nothing else is picked: the active one when it is not a war
 *  save, else the Official save in the game files. */
export const defaultVanillaSave = (list: SaveList, saves: SaveRow[]) =>
  saves.find((s) => s.id === list.active)?.id ?? saves.find((s) => s.kind === "official" && s.live)?.id ?? saves.find((s) => s.kind === "official")?.id ?? saves[0]?.id ?? null;

export function PlayChoice({ snap, choices, onClose }: { snap: Snapshot; choices: Choices; onClose: () => void }) {
  const running = snap.game === "running";
  const current = choices.wars.some((w) => w.id === choices.playing) ? choices.playing : null;
  // null: the official war.
  const [war, setWar] = useState<string | null>(current);
  const [list, setList] = useState<SaveList | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const [save, setSave] = useState<string | null>(null);
  const [ask, setAsk] = useState<SaveRow | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    call<SaveList>("saves_list").then(setList).catch((e) => setFailed(errorText(e)));
  }, []);
  useEffect(() => {
    const key = (e: KeyboardEvent) => e.key === "Escape" && !ask && onClose();
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [onClose, ask]);

  const chosen = war ? choices.wars.find((w) => w.id === war) ?? null : null;
  const vanilla = useMemo(() => (list ? vanillaSaves(list, choices.wars) : []), [list, choices.wars]);
  // The save follows the war: the war save for a custom war, the last pick or the default for the official war.
  useEffect(() => {
    if (!list) return;
    if (chosen) setSave(chosen.save ?? null);
    else setSave((s) => (s && vanilla.some((v) => v.id === s) ? s : defaultVanillaSave(list, vanilla)));
  }, [list, chosen, vanilla]);

  const unchanged = list != null && war === current && (chosen ? !chosen.save || chosen.save === list.active : save === list.active);

  const apply = useCallback(async (row?: SaveRow) => {
    setAsk(null);
    if (unchanged) { onClose(); return; }
    setBusy(true);
    try {
      if (chosen) {
        const pick = { id: chosen.id, name: chosen.name, host: chosen.host, base: chosen.base, computer: chosen.computer, model: chosen.model === "evolved" ? "evolved" : undefined };
        const r = await withTask(t("app.commander.selecting"), () => call<Done>("commander_select", { war: pick }), true);
        setSnapshot(r.snapshot);
        notify("info", t("app.commander.selected", chosen.name));
      } else if (save) {
        const r = await withTask(t("app.commander.selecting"), () => call<Done>("commander_vanilla", { save }), true);
        setSnapshot(r.snapshot);
        notify("info", row ? t("app.play.vanillaOn", row.name) : t("app.commander.officialSelected"));
      }
      await reloadChoices();
      onClose();
    } catch (e) {
      notify("error", errorText(e));
      setBusy(false);
    }
  }, [unchanged, chosen, save, onClose]);

  const saveRow = vanilla.find((s) => s.id === save);
  const select = () => {
    // An Official save that isn't the one in the game files is written over them: the Saves tab asks first too.
    if (!chosen && saveRow && saveRow.kind === "official" && !saveRow.live && saveRow.id !== list?.active) setAsk(saveRow);
    else apply(saveRow);
  };

  const warMeta = (w: WarChoice) => [
    w.host === "lan" ? (w.computer ? t("app.commander.hostedBy", w.computer) : t("app.play.lanWar")) : t("app.commander.thisPc"),
    w.state === "paused" ? t("app.play.paused") : w.state === "running" ? t("app.play.ongoing") : null,
    w.model === "evolved" ? t("app.play.evolved") : null,
  ].filter(Boolean).join("  ·  ");

  const warSave = chosen?.save ? list?.saves.find((s) => s.id === chosen.save) ?? null : null;
  const saves: SaveRow[] = chosen ? (warSave ? [warSave] : []) : vanilla;

  return createPortal(
    <div className="modal-back" onPointerDown={(e) => e.target === e.currentTarget && !ask && onClose()}>
      <div className="modal play-choice" role="dialog" aria-modal="true" aria-label={t("app.play.choose")}>
        <div className="pc-body">
          <section className="pc-col">
            <h3 className="pc-h">{t("app.play.wars")}</h3>
            <div className="pc-list" role="radiogroup" aria-label={t("app.play.wars")}>
              <PickRow on={war === null} now={current === null} title={t("app.play.vanillaGame")} meta={t("app.play.vanillaGameHint")} onPick={() => setWar(null)} />
              {choices.wars.map((w) => (
                <PickRow key={`${w.host}:${w.id}`} on={war === w.id} now={current === w.id} title={w.name} meta={warMeta(w)} onPick={() => setWar(w.id)} />
              ))}
            </div>
          </section>
          <section className="pc-col">
            <h3 className="pc-h">{t("app.play.saves")}</h3>
            <div className="pc-list" role="radiogroup" aria-label={t("app.play.saves")}>
              {!list && <div className="pc-empty">{failed ?? t("app.play.loadingSaves")}</div>}
              {list && saves.map((s) => (
                <PickRow key={s.id} on={save === s.id} now={list.active === s.id} title={s.name}
                  meta={[s.kind === "official" ? t("app.saves.official") : t("app.saves.sandbox"), s.stats ? t("app.saves.rankN", s.stats.rank) : null].filter(Boolean).join("  ·  ")}
                  onPick={() => !chosen && setSave(s.id)} fixed={!!chosen} />
              ))}
              {list && chosen && !warSave && <div className="pc-empty">{t("app.play.warSaveNew")}</div>}
              {list && !chosen && saves.length === 0 && <div className="pc-empty">{t("app.play.noSaves")}</div>}
            </div>
            {chosen && <p className="pc-note">{t("app.play.warSaveNote")}</p>}
          </section>
        </div>
        <div className="modal-foot">
          {running && <span className="pc-warn">{t("app.commander.gameRunning")}</span>}
          <button className="btn" onClick={onClose}>{t("app.cancel")}</button>
          <button className="btn play" disabled={busy || running || !list || (!chosen && !save)} onClick={select}>{t("app.play.select")}</button>
        </div>
      </div>
      {ask && <Confirm title={t("app.saves.switchTitle", ask.name)} text={t("app.saves.switchText")} confirm={t("app.play.select")} tone="go" onCancel={() => setAsk(null)} onConfirm={() => apply(ask)} />}
    </div>,
    document.body,
  );
}

/** One line of the dialog: a war or a save. `now` marks what Play starts today; `fixed` a line that is the
 *  only choice (the save of a custom war). */
function PickRow({ on, now, title, meta, onPick, fixed }: { on: boolean; now: boolean; title: string; meta: string; onPick: () => void; fixed?: boolean }) {
  return (
    <button role="radio" aria-checked={on} className={`pc-row${on ? " on" : ""}${fixed ? " fixed" : ""}`} onClick={onPick}>
      <span className="pc-mark" aria-hidden="true">{on && <CheckIcon />}</span>
      <span className="pc-text">
        <span className="pc-title">{title}</span>
        {meta && <span className="pc-meta">{meta}</span>}
      </span>
      {now && <span className="pc-now">{t("app.play.current")}</span>}
    </button>
  );
}
