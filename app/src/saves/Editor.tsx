// The save editor, for Sandbox saves with the game closed: Overview, Character, Arsenal, Campaign and Mods.
// Changes wait as edits until Save writes them all at once; the page always shows the save as it will be.
import { useCallback, useEffect, useMemo, useState } from "react";
import { call, errorText } from "../api";
import { withTask } from "../busy";
import { Confirm } from "../Confirm";
import { ArrowIcon, CheckIcon, ResetIcon, TrashIcon, WarnIcon } from "../Icons";
import { language, pick, t } from "../i18n";
import { notify } from "../notices";
import { Select } from "../Select";
import { Field, Help } from "../SettingRow";
import type { Snapshot } from "../types";
import { ArsenalTab } from "./ArsenalTab";
import { choiceId, draftOf, fieldEnabled, parseWhole, playTime, rankProgress, stage, topXp } from "./logic";
import { COSMETIC_SLOTS, type CosmeticSlot, type Edit, type EditorState, type ModChoice, type ModFieldState, type ModSection, type ModText } from "./types";

type Tab = "overview" | "character" | "arsenal" | "campaign" | "mods";
const TABS: Tab[] = ["overview", "character", "arsenal", "campaign", "mods"];
type Bulk = "max" | "reset" | null;

const num = (n: number) => n.toLocaleString(language());

/** A mod's own text: a table by language, or plain words. */
const modText = (v: ModText, fallback: string) => (typeof v === "string" ? v : pick(v, fallback));

export function SaveEditor({ id, snap, onClose }: { id: string; snap: Snapshot; onClose: () => void }) {
  const [base, setBase] = useState<EditorState | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const [pending, setPending] = useState<Map<string, Edit>>(new Map());
  const [tab, setTab] = useState<Tab>("overview");
  const [bulk, setBulk] = useState<Bulk>(null);
  const [leaving, setLeaving] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let live = true;
    call<EditorState>("saves_editor_state", { id }).then((s) => live && setBase(s)).catch((e) => live && setFailed(errorText(e)));
    return () => { live = false; };
  }, [id]);

  const draft = useMemo(() => (base ? draftOf(base, pending) : null), [base, pending]);
  const edit = useCallback((e: Edit) => { setPending((p) => (base ? stage(p, base, e) : p)); }, [base]);
  const running = snap.game === "running";

  const write = async (edits: Edit[], done: string) => {
    setSaving(true);
    try {
      const fresh = await withTask(t("app.task.saves_apply_edits"), () => call<EditorState>("saves_apply_edits", { id, edits }), true);
      setBase(fresh);
      setPending(new Map());
      notify("info", done);
    } catch (e) {
      notify("error", errorText(e));
    } finally {
      setSaving(false);
    }
  };

  const count = pending.size;
  const back = () => (count ? setLeaving(true) : onClose());
  const backBtn = <button className="btn small" onClick={back}><ArrowIcon />{t("app.saves.backToList")}</button>;

  if (!draft) {
    return (
      <div className="ed">
        <div className="head">{backBtn}</div>
        {failed && <p className="set-note">{failed}</p>}
      </div>
    );
  }

  const canWrite = !running && !saving;
  return (
    <div className="ed">
      <div className="head ed-head">
        {backBtn}
        <div className="ed-title">
          <h1 className="cap">{draft.name}</h1>
          <span className="save-chip sandbox">{t("app.saves.sandbox")}</span>
          <Help page="saves#the-save-editor" tip={t("app.saves.editorHelpTip")} />
        </div>
        <div className="ed-savebar">
          {count > 0 && <span className="ed-dirty">{t(count === 1 ? "app.saves.unsavedOne" : "app.saves.unsaved", count)}</span>}
          <button className="btn small" disabled={!count || saving} onClick={() => setPending(new Map())}><ResetIcon />{t("app.saves.discard")}</button>
          <button className="btn small play" disabled={!count || !canWrite} onClick={() => write([...pending.values()], t("app.saves.written"))}
            data-tip={running ? t("app.action.closeGameFirst") : undefined}><CheckIcon />{t("app.saves.save")}</button>
        </div>
      </div>
      <div className="subtabs ed-tabs" role="tablist">
        {TABS.map((id2) => (
          <button key={id2} role="tab" aria-selected={tab === id2} className={tab === id2 ? "cap cur" : "cap"} onClick={() => setTab(id2)}>
            {t(`app.saves.tab.${id2}`)}
          </button>
        ))}
      </div>
      {running && <div className="banner"><WarnIcon /><span>{t("app.saves.gameRunningEditor")}</span></div>}
      <div className="ed-body" key={tab}>
        {tab === "overview" && <OverviewTab draft={draft} dirty={count > 0} blocked={!canWrite} onBulk={setBulk} />}
        {tab === "character" && <CharacterTab draft={draft} onEdit={edit} />}
        {tab === "arsenal" && <ArsenalTab draft={draft} onEdit={edit} />}
        {tab === "campaign" && <CampaignTab draft={draft} onEdit={edit} />}
        {tab === "mods" && <ModsTab draft={draft} onEdit={edit} />}
      </div>
      {bulk && (
        <Confirm title={t(bulk === "max" ? "app.saves.maxTitle" : "app.saves.resetTitle", draft.name)} text={t(bulk === "max" ? "app.saves.maxText" : "app.saves.resetText")}
          confirm={t(bulk === "max" ? "app.saves.maxEverything" : "app.saves.resetProgression")} tone={bulk === "max" ? "go" : "warn"}
          onCancel={() => setBulk(null)} onConfirm={() => { const op = bulk === "max" ? "max_everything" : "reset_progression"; setBulk(null); write([{ op } as Edit], t("app.saves.written")); }} />
      )}
      {leaving && (
        <Confirm title={t("app.saves.leaveTitle")} text={t("app.saves.leaveText")} confirm={t("app.saves.leaveConfirm")} cancel={t("app.saves.leaveCancel")}
          onCancel={() => setLeaving(false)} onConfirm={() => { setLeaving(false); onClose(); }} />
      )}
    </div>
  );
}

/** A whole number box: no browser spinner, keeps the limits, written on Enter or leaving the box. */
function NumField({ value, min, max, label, disabled, onCommit }: { value: number; min: number | null; max: number | null; label: string; disabled?: boolean; onCommit: (n: number) => void }) {
  const [text, setText] = useState(String(value));
  useEffect(() => setText(String(value)), [value]);
  const commit = () => {
    const v = parseWhole(text, min, max);
    if (v === null) return setText(String(value));
    setText(String(v));
    if (v !== value) onCommit(v);
  };
  return <input className="ed-num" inputMode="numeric" value={text} disabled={disabled} aria-label={label} spellCheck={false}
    onChange={(e) => setText(e.target.value)} onBlur={commit} onKeyDown={(e) => { if (e.key === "Enter") commit(); }} />;
}

function Card({ title, help, children, wide }: { title: string; help?: string; children: React.ReactNode; wide?: boolean }) {
  return (
    <section className={wide ? "ed-card wide" : "ed-card"}>
      <h3 className="cap">{title}{help && <Help page={help} />}</h3>
      {children}
    </section>
  );
}

function OverviewTab({ draft, dirty, blocked, onBulk }: { draft: EditorState; dirty: boolean; blocked: boolean; onBulk: (b: Bulk) => void }) {
  const l = draft.lifetime;
  const title = draft.ranks.find((r) => r.level === draft.rank)?.title ?? "";
  const off = blocked || dirty;
  const why = dirty ? t("app.saves.saveOrDiscardFirst") : undefined;
  return (
    <div className="ed-grid">
      <Card title={t("app.saves.statsTitle")}>
        <dl className="save-stats">
          <div><dt>{t("app.saves.stat.rank")}</dt><dd>{draft.rank}<small>{title}</small></dd></div>
          <div><dt>{t("app.saves.stat.xp")}</dt><dd>{num(draft.xp)}</dd></div>
          <div><dt>{t("app.saves.stat.samples")}</dt><dd>{num(draft.samples)}</dd></div>
          <div><dt>{t("app.saves.researchPoints")}</dt><dd>{num(draft.researchPoints)}</dd></div>
          <div><dt>{t("app.saves.stat.missions")}</dt><dd>{t("app.saves.missionsValue", num(l.missions_won ?? 0), num(l.missions_fought ?? 0))}</dd></div>
          <div><dt>{t("app.saves.stat.kills")}</dt><dd>{num(l.kills ?? 0)}</dd></div>
          <div><dt>{t("app.saves.stat.deaths")}</dt><dd>{num(l.deaths ?? 0)}</dd></div>
          <div><dt>{t("app.saves.stat.played")}</dt><dd>{playTime(l.time_played ?? 0)}</dd></div>
        </dl>
      </Card>
      <Card title={t("app.saves.bulkTitle")}>
        <p className="ed-note">{t("app.saves.maxHint")}</p>
        <div className="ed-bulk">
          <button className="btn small go" disabled={off} onClick={() => onBulk("max")} data-tip={why}>{t("app.saves.maxEverything")}</button>
        </div>
        <p className="ed-note">{t("app.saves.resetHint")}</p>
        <div className="ed-bulk">
          <button className="btn small danger" disabled={off} onClick={() => onBulk("reset")} data-tip={why}>{t("app.saves.resetProgression")}</button>
        </div>
        {dirty && <p className="save-why">{t("app.saves.saveOrDiscardFirst")}</p>}
      </Card>
    </div>
  );
}

function CharacterTab({ draft, onEdit }: { draft: EditorState; onEdit: (e: Edit) => void }) {
  const prog = rankProgress(draft.ranks, draft.xp);
  const slotOptions = (slot: CosmeticSlot) => draft.cosmetics[slot]
    .map((c, i) => ({ c, i }))
    .filter(({ c, i }) => !c.dlc || draft.customization[slot] === i + 1)
    .map(({ c, i }) => ({ value: String(i + 1), label: c.owned || slot === "gender" ? c.display : t("app.saves.cosmeticNotOwned", c.display) }));
  const pickCosmetic = (slot: CosmeticSlot, value: string) => {
    const index = Number(value);
    const c = draft.cosmetics[slot][index - 1];
    // Gender is a choice, not something a save owns.
    if (c && !c.owned && slot !== "gender") onEdit({ op: "set_owned", item: c.name, owned: true });
    onEdit({ op: "set_customization", slot, index });
  };
  return (
    <div className="ed-grid">
      <Card title={t("app.saves.rankXp")}>
        <div className="ed-pair">
          <Field label={t("app.saves.stat.rank")}>
            <Select value={String(draft.rank)} label={t("app.saves.stat.rank")}
              options={draft.ranks.map((r) => ({ value: String(r.level), label: `${r.level}. ${r.title}` }))}
              onChange={(v) => onEdit({ op: "set_rank", rank: Number(v) })} />
          </Field>
          <Field label={t("app.saves.stat.xp")}>
            <NumField value={draft.xp} min={0} max={topXp(draft.ranks)} label={t("app.saves.stat.xp")} onCommit={(xp) => onEdit({ op: "set_xp", xp })} />
          </Field>
        </div>
        <div className="ed-meter" aria-hidden="true"><i style={{ width: `${prog.fraction * 100}%` }} /></div>
        <p className="ed-note">{prog.need === null ? t("app.saves.topRank") : t("app.saves.xpToNext", num(prog.need), `${prog.next!.level}. ${prog.next!.title}`)}</p>
      </Card>
      <Card title={t("app.saves.stat.samples")}>
        <div className="ed-pair">
          <Field label={t("app.saves.stat.samples")}>
            <NumField value={draft.samples} min={0} max={draft.maxSamples} label={t("app.saves.stat.samples")} onCommit={(samples) => onEdit({ op: "set_samples", samples })} />
          </Field>
          <div className="ed-derived">
            <span className="field-label">{t("app.saves.researchPoints")}</span>
            <b>{num(draft.researchPoints)}</b>
          </div>
        </div>
        <p className="ed-note">{t("app.saves.samplesHint", draft.samplesPerPoint, num(draft.maxSamples))}</p>
      </Card>
      <Card title={t("app.saves.cosmetics")} wide>
        <div className="ed-slots">
          {COSMETIC_SLOTS.map((slot) => (
            <Field key={slot} label={t(`app.saves.slot.${slot}`)}>
              <Select value={String(draft.customization[slot] ?? "")} label={t(`app.saves.slot.${slot}`)} options={slotOptions(slot)} onChange={(v) => pickCosmetic(slot, v)} />
            </Field>
          ))}
        </div>
        <p className="ed-note">{t("app.saves.cosmeticsHint")}</p>
      </Card>
    </div>
  );
}

const BOSSES = ["bugs", "cyborgs", "illuminate"];

function CampaignTab({ draft, onEdit }: { draft: EditorState; onEdit: (e: Edit) => void }) {
  return (
    <div className="ed-grid">
      <Card title={t("app.saves.bossInfluence")}>
        <div className="ed-lines">
          {BOSSES.map((boss, i) => (
            <div className="ed-line" key={boss}>
              <span>{t(`app.saves.boss.${boss}`)}</span>
              <NumField value={draft.bossInfluence[i] ?? 0} min={0} max={draft.bossInfluenceRequired} label={t(`app.saves.boss.${boss}`)} onCommit={(value) => onEdit({ op: "set_boss_influence", index: i + 1, value })} />
            </div>
          ))}
        </div>
        <p className="ed-note">{t("app.saves.bossHint", draft.bossInfluenceRequired)}</p>
      </Card>
      <Card title={t("app.saves.tutorials")}>
        <div className="ed-lines">
          <div className="ed-line">
            <span>{t("app.saves.tutorialStage")}</span>
            <NumField value={draft.tutorialStage ?? 0} min={0} max={null} label={t("app.saves.tutorialStage")} onCommit={(value) => onEdit({ op: "set_tutorial_stage", value })} />
          </div>
          {draft.flags.map((f) => (
            <div className="ed-line" key={f.key}>
              <span>{t(`app.saves.flag.${f.key}`)}</span>
              <button role="switch" aria-checked={f.value} aria-label={t(`app.saves.flag.${f.key}`)} className={f.value ? "sw on" : "sw"} onClick={() => onEdit({ op: "set_flag", key: f.key, value: !f.value })} />
            </div>
          ))}
        </div>
      </Card>
      <Card title={t("app.saves.lifetime")} wide>
        <div className="ed-life">
          {Object.keys(draft.lifetime).map((key) => (
            <div className="ed-line" key={key}>
              <span>{t(`app.saves.life.${key}`)}</span>
              <NumField value={draft.lifetime[key]} min={0} max={null} label={t(`app.saves.life.${key}`)} onCommit={(value) => onEdit({ op: "set_lifetime", key, value })} />
            </div>
          ))}
        </div>
      </Card>
    </div>
  );
}

function ModsTab({ draft, onEdit }: { draft: EditorState; onEdit: (e: Edit) => void }) {
  if (draft.mods.length === 0) return <p className="muted pad">{t("app.saves.noModData")}</p>;
  return (
    <div className="ed-mods">
      {draft.mods.map((m) => <ModCard key={m.id} mod={m} onEdit={onEdit} />)}
    </div>
  );
}

function ModCard({ mod, onEdit }: { mod: ModSection; onEdit: (e: Edit) => void }) {
  const off = mod.state === "disabled";
  const name = pick(mod.name, mod.id);
  const unlock = (u: ModSection["items"][number]["unlock"]) =>
    typeof u === "object" && u !== null ? t("app.saves.rankN", u.rank) : u === "free" ? t("app.saves.unlockFree") : u === "base" ? t("app.saves.unlockBase") : "";
  return (
    <section className={`ed-card ed-mod ${mod.state}`}>
      <h3 className="ed-mod-head">
        <span className="ed-mod-name">{name}</span>
        <span className="mono">{mod.id}</span>
        {mod.state !== "enabled" && <span className={`save-chip ${mod.state}`}>{t(`app.saves.mod.${mod.state}`)}</span>}
        {mod.state === "removed" && <button className="btn small danger ed-mod-clear" onClick={() => onEdit({ op: "mod_clear", mod_id: mod.id })}><TrashIcon />{t("app.saves.deleteAllData")}</button>}
      </h3>
      {off && <p className="ed-note">{t("app.saves.modDisabledHint")}</p>}
      {mod.state === "removed" && (
        <>
          <p className="ed-note">{t("app.saves.modRemovedHint")}</p>
          <ul className="ed-rows">
            {mod.data.map((d) => (
              <li key={d.key} className="ed-line">
                <span className="mono">{d.key}</span>
                <span className="mono ed-val">{String(d.value)}</span>
                <button className="bare" onClick={() => onEdit({ op: "mod_delete", mod_id: mod.id, key: d.key })} aria-label={t("app.saves.delete")} data-tip={t("app.saves.delete")}><TrashIcon /></button>
              </li>
            ))}
          </ul>
        </>
      )}
      {mod.fields.length > 0 && (
        <div className="ed-lines">
          {mod.fields.map((f) => <ModField key={f.id} mod={mod} field={f} off={off} onEdit={onEdit} />)}
        </div>
      )}
      {mod.items.length > 0 && (
        <ul className={off ? "ed-rows off" : "ed-rows"}>
          {mod.items.map((i) => (
            <li key={i.name} className="ed-line">
              <span>{modText(i.label, i.name)}<small className="muted ed-kind">{t(`app.saves.modKind.${i.kind}`)}</small></span>
              {i.unlock === "manual" ? (
                <button role="switch" aria-checked={i.owned === true} aria-label={modText(i.label, i.name)} className={i.owned ? "sw on" : "sw"} disabled={off}
                  onClick={() => onEdit({ op: "mod_item_owned", mod_id: mod.id, name: i.name, owned: !i.owned })} />
              ) : (
                <span className="muted">{unlock(i.unlock)}</span>
              )}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function ModField({ mod, field, off, onEdit }: { mod: ModSection; field: ModFieldState; off: boolean; onEdit: (e: Edit) => void }) {
  const label = modText(field.label, field.id);
  const inactive = off || !fieldEnabled(field, mod.fields);
  const set = (value: string | number | boolean) => onEdit({ op: "mod_set", mod_id: mod.id, key: field.id, value });
  let control;
  if (field.kind === "toggle") {
    const on = field.value === true;
    control = <button role="switch" aria-checked={on} aria-label={label} className={on ? "sw on" : "sw"} disabled={inactive} onClick={() => set(!on)} />;
  } else if (field.kind === "number") {
    control = <NumField value={typeof field.value === "number" ? field.value : field.min ?? 0} min={field.min} max={field.max} label={label} disabled={inactive} onCommit={set} />;
  } else {
    const options = (field.choices ?? []).map((c: ModChoice) => ({ value: choiceId(c), label: typeof c === "string" ? c : modText(c.label ?? null, c.id) }));
    control = inactive
      ? <span className="muted">{options.find((o) => o.value === field.value)?.label ?? ""}</span>
      : <Select value={String(field.value ?? "")} options={options} label={label} onChange={set} />;
  }
  return <div className={inactive ? "ed-line off" : "ed-line"}><span>{label}</span>{control}</div>;
}
