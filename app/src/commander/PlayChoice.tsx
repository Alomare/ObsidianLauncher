// The Play choice: the right part of the split Play button opens it when there is more than one war to
// play. The official war comes first, then the custom wars this PC hosts (running or paused) and the
// LAN wars it joined; beside them, the saves the chosen war can boot. The official war boots any save.
// A custom war boots any Sandbox save, or only the one made for it when the war says so; with none
// picked, the war gets that save on first use. Select makes it what Play starts.
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

/** The saves the official war can boot: every one, war saves included. */
export const vanillaSaves = (list: SaveList) => list.saves;

/** The save the official war would boot when nothing else is picked: the active one, else the Official
 *  save in the game files. */
export const defaultVanillaSave = (list: SaveList, saves: SaveRow[]) =>
  saves.find((s) => s.id === list.active)?.id ?? saves.find((s) => s.kind === "official" && s.live)?.id ?? saves.find((s) => s.kind === "official")?.id ?? saves[0]?.id ?? null;

/** The saves a custom war can boot: the save made for it first, then every other Sandbox unless the war
 *  keeps to its own. An Official save never plays a custom war. */
export const warSaves = (list: SaveList, war: WarChoice) => {
  const own = list.saves.find((s) => s.id === war.save && s.kind === "sandbox");
  const others = war.ownSaveOnly ? [] : list.saves.filter((s) => s.kind === "sandbox" && s.id !== war.save);
  return own ? [own, ...others] : others;
};

/** The save a custom war starts from in the dialog: the active save when the war can boot it, else the
 *  save made for it. null: none yet, so selecting makes one. */
export const defaultWarSave = (list: SaveList, war: WarChoice, saves: SaveRow[]) =>
  saves.find((s) => s.id === list.active)?.id ?? saves.find((s) => s.id === war.save)?.id ?? null;

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
  const saves: SaveRow[] = useMemo(() => (!list ? [] : chosen ? warSaves(list, chosen) : vanillaSaves(list)), [list, chosen]);
  // The save follows the war: the last pick when the war can boot it, else the default of the war.
  useEffect(() => {
    if (!list) return;
    setSave((s) => (s && saves.some((v) => v.id === s) ? s : chosen ? defaultWarSave(list, chosen, saves) : defaultVanillaSave(list, saves)));
  }, [list, chosen, saves]);

  const unchanged = list != null && war === current && save != null && save === list.active;

  const apply = useCallback(async (row?: SaveRow) => {
    setAsk(null);
    if (unchanged) { onClose(); return; }
    setBusy(true);
    try {
      if (chosen) {
        const pick = { id: chosen.id, name: chosen.name, host: chosen.host, base: chosen.base, computer: chosen.computer, model: chosen.model === "enhanced" ? "enhanced" : undefined,
          ownSaveOnly: !!chosen.ownSaveOnly, allowAchievements: !!chosen.allowAchievements };
        const r = await withTask(t("app.commander.selecting"), () => call<Done>("commander_select", { war: pick, save }), true);
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

  const saveRow = saves.find((s) => s.id === save);
  const select = () => {
    // An Official save that isn't the one in the game files is written over them: the Saves tab asks first too.
    if (!chosen && saveRow && saveRow.kind === "official" && !saveRow.live && saveRow.id !== list?.active) setAsk(saveRow);
    else apply(saveRow);
  };

  const warMeta = (w: WarChoice) => [
    w.host === "lan" ? (w.computer ? t("app.commander.hostedBy", w.computer) : t("app.play.lanWar")) : t("app.commander.thisPc"),
    w.state === "paused" ? t("app.play.paused") : w.state === "running" ? t("app.play.ongoing") : null,
    w.model === "enhanced" ? t("app.play.enhanced") : null,
  ].filter(Boolean).join("  ·  ");

  const hasOwn = !!chosen && saves.some((s) => s.id === chosen.save);
  const saveMeta = (s: SaveRow) => [
    s.kind === "official" ? t("app.saves.official") : t("app.saves.sandbox"),
    chosen && s.id === chosen.save ? t("app.play.warSave") : null,
    s.stats ? t("app.saves.rankN", s.stats.rank) : null,
  ].filter(Boolean).join("  ·  ");

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
              {list && chosen && !hasOwn && (
                <PickRow on={save === null} now={false} title={t("app.play.warSaveNewTitle")} meta={t("app.play.warSaveNew")} onPick={() => setSave(null)} />
              )}
              {list && saves.map((s) => (
                <PickRow key={s.id} on={save === s.id} now={list.active === s.id} title={s.name} meta={saveMeta(s)} onPick={() => setSave(s.id)} />
              ))}
              {list && !chosen && saves.length === 0 && <div className="pc-empty">{t("app.play.noSaves")}</div>}
            </div>
            {chosen && <p className="pc-note">{t(chosen.ownSaveOnly ? "app.play.warSaveOnly" : "app.play.warSaveAny")}</p>}
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

/** One line of the dialog: a war or a save. `now` marks what Play starts today. */
function PickRow({ on, now, title, meta, onPick }: { on: boolean; now: boolean; title: string; meta: string; onPick: () => void }) {
  return (
    <button role="radio" aria-checked={on} className={`pc-row${on ? " on" : ""}`} onClick={onPick}>
      <span className="pc-mark" aria-hidden="true">{on && <CheckIcon />}</span>
      <span className="pc-text">
        <span className="pc-title">{title}</span>
        {meta && <span className="pc-meta">{meta}</span>}
      </span>
      {now && <span className="pc-now">{t("app.play.current")}</span>}
    </button>
  );
}
