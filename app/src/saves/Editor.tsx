// The save editor, for Sandbox saves with the game closed: Overview, Character, Arsenal, Campaign and Mods.
// Changes wait as edits until Save writes them all at once; the page always shows the save as it will be.
// Every value sits on the grid of the options panel: the name with a line under it, the control filling the
// middle cell, and a button to undo a change waiting on that row.
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { call, errorText, setSnapshot, taskText } from "../api";
import { withTask } from "../busy";
import { Confirm } from "../Confirm";
import { ArrowIcon, CheckIcon, CloseIcon, PenIcon, ResetIcon, TrashIcon, WarnIcon } from "../Icons";
import { language, pick, t } from "../i18n";
import { notify } from "../notices";
import { Select } from "../Select";
import { Help } from "../SettingRow";
import type { Snapshot } from "../types";
import { ArsenalTab } from "./ArsenalTab";
import { choiceId, draftOf, fieldEnabled, parseWhole, playTime, rankProgress, slotChoices, stage, topXp } from "./logic";
import { COSMETIC_SLOTS, type CosmeticSlot, type Edit, type EditorState, type ModChoice, type ModFieldState, type ModSection, type ModText, type SavesDone } from "./types";

type Tab = "overview" | "character" | "arsenal" | "campaign" | "mods";
const TABS: Tab[] = ["overview", "character", "arsenal", "campaign", "mods"];
type Bulk = "max" | "reset" | null;

const num = (n: number) => n.toLocaleString(language());

/** A mod's own text: a table by language, or plain words. */
const modText = (v: ModText, fallback: string) => (typeof v === "string" ? v : pick(v, fallback));

/** What a row needs for the undo button: the waiting edits, and how to drop the ones of that row. */
type Undo = { pending: Map<string, Edit>; revert: (keys: string[]) => void };

export function SaveEditor({ id, snap, onClose, startRename = false }: { id: string; snap: Snapshot; onClose: () => void; startRename?: boolean }) {
  const [base, setBase] = useState<EditorState | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const [pending, setPending] = useState<Map<string, Edit>>(new Map());
  const [tab, setTab] = useState<Tab>("overview");
  const [bulk, setBulk] = useState<Bulk>(null);
  const [leaving, setLeaving] = useState(false);
  const [saving, setSaving] = useState(false);
  const [renaming, setRenaming] = useState(startRename);

  useEffect(() => {
    let live = true;
    call<EditorState>("saves_editor_state", { id }).then((s) => live && setBase(s)).catch((e) => live && setFailed(errorText(e)));
    return () => { live = false; };
  }, [id]);

  const draft = useMemo(() => (base ? draftOf(base, pending) : null), [base, pending]);
  const edit = useCallback((e: Edit) => { setPending((p) => (base ? stage(p, base, e) : p)); }, [base]);
  const revert = useCallback((keys: string[]) => setPending((p) => {
    const next = new Map(p);
    for (const k of keys) next.delete(k);
    return next;
  }), []);
  const undo: Undo = { pending, revert };
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

  // Renaming writes at once and leaves the waiting edits alone.
  const rename = async (name: string) => {
    if (!base || name === base.name) return true;
    try {
      const r = await withTask(taskText("saves_rename"), () => call<SavesDone>("saves_rename", { id, name }), true);
      setSnapshot(r.snapshot);
      setBase({ ...base, name });
      return true;
    } catch (e) {
      notify("error", errorText(e));
      return false;
    }
  };

  const count = pending.size;
  const back = () => (count ? setLeaving(true) : onClose());
  const backBtn = (
    <button className="btn icon head-tool ed-back" onClick={back} aria-label={t("app.saves.backToList")} data-tip={t("app.saves.backToList")}><ArrowIcon /></button>
  );

  if (!draft) {
    return (
      <div className="ed">
        <div className="head ed-head">{backBtn}</div>
        {failed && <p className="save-note warn">{failed}</p>}
      </div>
    );
  }

  const canWrite = !running && !saving;
  return (
    <div className="ed">
      <div className="head ed-head">
        {backBtn}
        <div className="ed-title">
          {renaming && !running ? (
            <TitleEdit name={draft.name} onCancel={() => setRenaming(false)} onConfirm={async (name) => { const ok = await rename(name); if (ok) setRenaming(false); }} />
          ) : (
            <button className="ed-rename" disabled={running} onClick={() => setRenaming(true)} aria-label={t("app.saves.renameTitle", draft.name)}
              data-tip-title={t("app.saves.rename")} data-tip={running ? t("app.action.closeGameFirst") : t("app.saves.renameHint")}>
              <h1 className="cap">{draft.name}</h1><PenIcon />
            </button>
          )}
          <span className="save-chip sandbox">{t("app.saves.sandbox")}</span>
          <Help page="saves#the-save-editor" tip={t("app.saves.editorHelpTip")} />
        </div>
        <div className="ed-savebar">
          {count > 0 && <span className="ed-dirty">{t(count === 1 ? "app.saves.unsavedOne" : "app.saves.unsaved", count)}</span>}
          <button className="btn icon head-tool" disabled={!count || saving} onClick={() => setPending(new Map())} aria-label={t("app.saves.discard")}
            data-tip-title={t("app.saves.discard")} data-tip={t("app.saves.discardHint")}><ResetIcon /></button>
          <button className="btn go head-tool" disabled={!count || !canWrite} onClick={() => write([...pending.values()], t("app.saves.written"))}
            data-tip={running ? t("app.action.closeGameFirst") : undefined}><CheckIcon /><span className="label">{t("app.saves.save")}</span></button>
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
        {tab === "character" && <CharacterTab draft={draft} onEdit={edit} undo={undo} />}
        {tab === "arsenal" && <ArsenalTab draft={draft} onEdit={edit} />}
        {tab === "campaign" && <CampaignTab draft={draft} onEdit={edit} undo={undo} />}
        {tab === "mods" && <ModsTab draft={draft} onEdit={edit} undo={undo} />}
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

/** The save name turned into a box in place, with confirm and cancel beside it. Enter confirms, Escape cancels. */
function TitleEdit({ name, onConfirm, onCancel }: { name: string; onConfirm: (v: string) => Promise<void>; onCancel: () => void }) {
  const [v, setV] = useState(name);
  const [busy, setBusy] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => { input.current?.select(); }, []);
  const go = async () => {
    const next = v.trim();
    if (!next || busy) return;
    setBusy(true);
    await onConfirm(next);
    setBusy(false);
  };
  return (
    <form className="ed-title-edit" onSubmit={(e) => { e.preventDefault(); go(); }}>
      <input ref={input} className="cap" value={v} maxLength={40} spellCheck={false} disabled={busy} aria-label={t("app.saves.name")} onChange={(e) => setV(e.target.value)}
        onKeyDown={(e) => { if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); onCancel(); } }} />
      <button type="submit" className="btn icon head-tool go" disabled={!v.trim() || busy} aria-label={t("app.saves.rename")} data-tip={t("app.saves.rename")}><CheckIcon /></button>
      <button type="button" className="btn icon head-tool" disabled={busy} onClick={onCancel} aria-label={t("app.cancel")} data-tip={t("app.cancel")}><CloseIcon /></button>
    </form>
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

/** A panel of rows under a title, with an optional line about the whole group under the rows. */
function Panel({ title, note, tools, children, className }: { title: ReactNode; note?: ReactNode; tools?: ReactNode; children: ReactNode; className?: string }) {
  return (
    <section className={className ? `ed-card ${className}` : "ed-card"}>
      <h3 className="ed-card-head"><span className="cap">{title}</span>{tools}</h3>
      {children}
      {note && <p className="ed-note">{note}</p>}
    </section>
  );
}

function Switch({ on, label, disabled, onFlip }: { on: boolean; label: string; disabled?: boolean; onFlip: () => void }) {
  return <button role="switch" aria-checked={on} aria-label={label} className={on ? "sw on" : "sw"} disabled={disabled} onClick={onFlip} />;
}

/** One row of the grid. `keys` names the waiting edits the row's undo drops; with none, the third cell
 *  holds `end` or stays empty. */
function Row({ label, desc, children, keys, undo, end, off }: { label: ReactNode; desc?: ReactNode; children?: ReactNode; keys?: string[]; undo?: Undo; end?: ReactNode; off?: boolean }) {
  const changed = !!keys && !!undo && keys.some((k) => undo.pending.has(k));
  return (
    <li className={off ? "opt off" : "opt"}>
      <div className="opt-text">
        <div className="l">{label}</div>
        {desc && <div className="d">{desc}</div>}
      </div>
      <div className="opt-cell">{children}</div>
      <div className="opt-cell">
        {keys && undo ? (
          <button className="bare" disabled={!changed} onClick={() => undo.revert(keys)} aria-label={t("app.saves.undoRow")} data-tip={changed ? t("app.saves.undoRow") : undefined}><ResetIcon /></button>
        ) : end}
      </div>
    </li>
  );
}

function OverviewTab({ draft, dirty, blocked, onBulk }: { draft: EditorState; dirty: boolean; blocked: boolean; onBulk: (b: Bulk) => void }) {
  const l = draft.lifetime;
  const title = draft.ranks.find((r) => r.level === draft.rank)?.title ?? "";
  const off = blocked || dirty;
  const why = dirty ? t("app.saves.saveOrDiscardFirst") : undefined;
  return (
    <div className="ed-cols">
      <Panel title={t("app.saves.statsTitle")}>
        <dl className="kv">
          <div><dt>{t("app.saves.stat.rank")}</dt><dd>{draft.rank}<small className="kv-sub">{title}</small></dd></div>
          <div><dt>{t("app.saves.stat.xp")}</dt><dd>{num(draft.xp)}</dd></div>
          <div><dt>{t("app.saves.stat.samples")}</dt><dd>{num(draft.samples)}</dd></div>
          <div><dt>{t("app.saves.researchPoints")}</dt><dd>{num(draft.researchPoints)}</dd></div>
          <div><dt>{t("app.saves.stat.missions")}</dt><dd>{t("app.saves.missionsValue", num(l.missions_won ?? 0), num(l.missions_fought ?? 0))}</dd></div>
          <div><dt>{t("app.saves.stat.kills")}</dt><dd>{num(l.kills ?? 0)}</dd></div>
          <div><dt>{t("app.saves.stat.deaths")}</dt><dd>{num(l.deaths ?? 0)}</dd></div>
          <div><dt>{t("app.saves.stat.played")}</dt><dd>{playTime(l.time_played ?? 0)}</dd></div>
        </dl>
      </Panel>
      <Panel title={t("app.saves.bulkTitle")} note={dirty ? <span className="warn">{t("app.saves.saveOrDiscardFirst")}</span> : undefined}>
        <ul className="opts opt-table">
          <Row label={t("app.saves.maxEverything")} desc={t("app.saves.maxHint")}>
            <button className="btn small go ed-fill" disabled={off} onClick={() => onBulk("max")} data-tip={why}>{t("app.saves.maxShort")}</button>
          </Row>
          <Row label={t("app.saves.resetProgression")} desc={t("app.saves.resetHint")}>
            <button className="btn small danger ed-fill" disabled={off} onClick={() => onBulk("reset")} data-tip={why}>{t("app.saves.resetShort")}</button>
          </Row>
        </ul>
      </Panel>
    </div>
  );
}

function CharacterTab({ draft, onEdit, undo }: { draft: EditorState; onEdit: (e: Edit) => void; undo: Undo }) {
  const prog = rankProgress(draft.ranks, draft.xp);
  const slotOptions = (slot: CosmeticSlot) => slotChoices(draft.cosmetics[slot], draft.customization[slot]).map((i) => {
    const c = draft.cosmetics[slot][i - 1];
    return { value: String(i), label: c.owned || slot === "gender" ? c.display : t("app.saves.cosmeticNotOwned", c.display) };
  });
  const pickCosmetic = (slot: CosmeticSlot, value: string) => {
    const index = Number(value);
    const c = draft.cosmetics[slot][index - 1];
    // Gender is a choice, not something a save owns. Owned DLC is granted by Steam and needs no unlock.
    if (c && !c.owned && !c.locked && slot !== "gender") onEdit({ op: "set_owned", item: c.name, owned: true });
    onEdit({ op: "set_customization", slot, index });
  };
  /** A slot's undo also drops the unlock that picking an item not owned added. */
  const slotKeys = (slot: CosmeticSlot) => [`cos:${slot}`, ...draft.cosmetics[slot].map((c) => `own:${c.name}`)];
  return (
    <div className="ed-cols">
      <Panel title={t("app.saves.rankXp")}>
        <ul className="opts opt-table">
          <Row label={t("app.saves.stat.rank")} keys={["xp"]} undo={undo}>
            <Select fill value={String(draft.rank)} label={t("app.saves.stat.rank")}
              options={draft.ranks.map((r) => ({ value: String(r.level), label: `${r.level}. ${r.title}` }))}
              onChange={(v) => onEdit({ op: "set_rank", rank: Number(v) })} />
          </Row>
          <Row label={t("app.saves.stat.xp")} keys={["xp"]} undo={undo}
            desc={prog.need === null ? t("app.saves.topRank") : t("app.saves.xpToNext", num(prog.need), `${prog.next!.level}. ${prog.next!.title}`)}>
            <div className="ed-stack">
              <NumField value={draft.xp} min={0} max={topXp(draft.ranks)} label={t("app.saves.stat.xp")} onCommit={(xp) => onEdit({ op: "set_xp", xp })} />
              <div className="ed-meter" aria-hidden="true"><i style={{ width: `${prog.fraction * 100}%` }} /></div>
            </div>
          </Row>
        </ul>
      </Panel>
      <Panel title={t("app.saves.stat.samples")}>
        <ul className="opts opt-table">
          <Row label={t("app.saves.stat.samples")} desc={t("app.saves.samplesHint", draft.samplesPerPoint, num(draft.maxSamples))} keys={["samples"]} undo={undo}>
            <NumField value={draft.samples} min={0} max={draft.maxSamples} label={t("app.saves.stat.samples")} onCommit={(samples) => onEdit({ op: "set_samples", samples })} />
          </Row>
          <Row label={t("app.saves.researchPoints")}>
            <span className="ed-value">{num(draft.researchPoints)}</span>
          </Row>
        </ul>
      </Panel>
      <Panel title={t("app.saves.cosmetics")} note={t("app.saves.cosmeticsHint")}>
        <ul className="opts opt-table">
          {COSMETIC_SLOTS.map((slot) => (
            <Row key={slot} label={t(`app.saves.slot.${slot}`)} keys={slotKeys(slot)} undo={undo}>
              <Select fill value={String(draft.customization[slot] ?? "")} label={t(`app.saves.slot.${slot}`)} options={slotOptions(slot)} onChange={(v) => pickCosmetic(slot, v)} />
            </Row>
          ))}
        </ul>
      </Panel>
    </div>
  );
}

const BOSSES = ["bugs", "cyborgs", "illuminate"];

function CampaignTab({ draft, onEdit, undo }: { draft: EditorState; onEdit: (e: Edit) => void; undo: Undo }) {
  return (
    <div className="ed-cols">
      <Panel title={t("app.saves.bossInfluence")} note={t("app.saves.bossHint", draft.bossInfluenceRequired)}>
        <ul className="opts opt-table">
          {BOSSES.map((boss, i) => (
            <Row key={boss} label={t(`app.saves.boss.${boss}`)} keys={[`boss:${i + 1}`]} undo={undo}>
              <NumField value={draft.bossInfluence[i] ?? 0} min={0} max={draft.bossInfluenceRequired} label={t(`app.saves.boss.${boss}`)} onCommit={(value) => onEdit({ op: "set_boss_influence", index: i + 1, value })} />
            </Row>
          ))}
        </ul>
      </Panel>
      <Panel title={t("app.saves.tutorials")}>
        <ul className="opts opt-table">
          <Row label={t("app.saves.tutorialStage")} keys={["stage"]} undo={undo}>
            <NumField value={draft.tutorialStage ?? 0} min={0} max={null} label={t("app.saves.tutorialStage")} onCommit={(value) => onEdit({ op: "set_tutorial_stage", value })} />
          </Row>
          {draft.flags.map((f) => (
            <Row key={f.key} label={t(`app.saves.flag.${f.key}`)} keys={[`flag:${f.key}`]} undo={undo}>
              <Switch on={f.value} label={t(`app.saves.flag.${f.key}`)} onFlip={() => onEdit({ op: "set_flag", key: f.key, value: !f.value })} />
            </Row>
          ))}
        </ul>
      </Panel>
      <Panel title={t("app.saves.lifetime")}>
        <ul className="opts opt-table">
          {Object.keys(draft.lifetime).map((key) => (
            <Row key={key} label={t(`app.saves.life.${key}`)} keys={[`life:${key}`]} undo={undo}>
              <NumField value={draft.lifetime[key]} min={0} max={null} label={t(`app.saves.life.${key}`)} onCommit={(value) => onEdit({ op: "set_lifetime", key, value })} />
            </Row>
          ))}
        </ul>
      </Panel>
    </div>
  );
}

function ModsTab({ draft, onEdit, undo }: { draft: EditorState; onEdit: (e: Edit) => void; undo: Undo }) {
  if (draft.mods.length === 0) return <p className="muted pad">{t("app.saves.noModData")}</p>;
  return (
    <div className="ed-cols">
      {draft.mods.map((m) => <ModPanel key={m.id} mod={m} onEdit={onEdit} undo={undo} />)}
    </div>
  );
}

function ModPanel({ mod, onEdit, undo }: { mod: ModSection; onEdit: (e: Edit) => void; undo: Undo }) {
  const off = mod.state === "disabled";
  const name = pick(mod.name, mod.id);
  const unlock = (u: ModSection["items"][number]["unlock"]) =>
    typeof u === "object" && u !== null ? t("app.saves.rankN", u.rank) : u === "free" ? t("app.saves.unlockFree") : u === "base" ? t("app.saves.unlockBase") : "";
  const title = (
    <>
      <span className="ed-mod-name">{name}</span>
      <span className="mono ed-mod-id">{mod.id}</span>
      {mod.state !== "enabled" && <span className={`save-chip ${mod.state}`}>{t(`app.saves.mod.${mod.state}`)}</span>}
    </>
  );
  const tools = mod.state === "removed" && mod.data.length > 0
    ? <button className="btn small icon danger ed-mod-clear" onClick={() => onEdit({ op: "mod_clear", mod_id: mod.id })} aria-label={t("app.saves.deleteAllData")} data-tip={t("app.saves.deleteAllData")}><TrashIcon /></button>
    : undefined;
  const note = off ? t("app.saves.modDisabledHint") : mod.state === "removed" ? t("app.saves.modRemovedHint") : undefined;
  return (
    <Panel title={title} tools={tools} note={note} className={`ed-mod ${mod.state}`}>
      <ul className="opts opt-table">
        {mod.state === "removed" && mod.data.map((d) => (
          <Row key={d.key} label={<span className="mono">{d.key}</span>}
            end={<button className="bare" onClick={() => onEdit({ op: "mod_delete", mod_id: mod.id, key: d.key })} aria-label={t("app.saves.delete")} data-tip={t("app.saves.delete")}><TrashIcon /></button>}>
            <span className="mono ed-value"><span className="ed-trunc">{String(d.value)}</span></span>
          </Row>
        ))}
        {mod.fields.map((f) => <ModField key={f.id} mod={mod} field={f} off={off} onEdit={onEdit} undo={undo} />)}
        {mod.items.map((i) => (
          <Row key={i.name} label={modText(i.label, i.name)} desc={t(`app.saves.modKind.${i.kind}`)} off={off} keys={[`item:${mod.id}:${i.name}`]} undo={undo}>
            {i.unlock === "manual"
              ? <Switch on={i.owned === true} label={modText(i.label, i.name)} disabled={off} onFlip={() => onEdit({ op: "mod_item_owned", mod_id: mod.id, name: i.name, owned: !i.owned })} />
              : <span className="ed-value muted">{unlock(i.unlock)}</span>}
          </Row>
        ))}
      </ul>
    </Panel>
  );
}

function ModField({ mod, field, off, onEdit, undo }: { mod: ModSection; field: ModFieldState; off: boolean; onEdit: (e: Edit) => void; undo: Undo }) {
  const label = modText(field.label, field.id);
  const inactive = off || !fieldEnabled(field, mod.fields);
  const set = (value: string | number | boolean) => onEdit({ op: "mod_set", mod_id: mod.id, key: field.id, value });
  let control;
  if (field.kind === "toggle") {
    const on = field.value === true;
    control = <Switch on={on} label={label} disabled={inactive} onFlip={() => set(!on)} />;
  } else if (field.kind === "number") {
    control = <NumField value={typeof field.value === "number" ? field.value : field.min ?? 0} min={field.min} max={field.max} label={label} disabled={inactive} onCommit={set} />;
  } else {
    const options = (field.choices ?? []).map((c: ModChoice) => ({ value: choiceId(c), label: typeof c === "string" ? c : modText(c.label ?? null, c.id) }));
    control = inactive
      ? <span className="ed-value muted">{options.find((o) => o.value === field.value)?.label ?? ""}</span>
      : <Select fill value={String(field.value ?? "")} options={options} label={label} onChange={set} />;
  }
  return <Row label={label} off={inactive} keys={[`mod:${mod.id}:${field.id}`]} undo={undo}>{control}</Row>;
}
