// The editor's Arsenal: one board of icon tiles (weapons, stratagems by group, perks) under a search box, and an
// inspector beside it for the selected item, with the owned switch and the upgrades. A tile selects the item;
// the check in the tile corner turns ownership on or off without leaving the board.
import { useMemo, useState, type CSSProperties } from "react";
import { CheckIcon, CloseIcon, LockIcon, ModsIcon, SearchIcon } from "../Icons";
import { t } from "../i18n";
import { artFor } from "./art";
import { groupItems, itemMatches, levelPick, ownedRoute, upgradeMarks, withBit, hasBit, type OwnedRoute } from "./logic";
import type { Edit, EditorState, ItemState } from "./types";

type Kind = "weapon" | "stratagem" | "perk";

const label = (key: string, fallback: string) => {
  const text = t(key);
  return text === key ? fallback : text;
};

/** What the owned controls of an item can do, and the line that says why. */
function ownership(item: ItemState, rank: number) {
  const route: OwnedRoute = ownedRoute(item, rank);
  const locked = route === "dlc";
  const changeable = route === "unlocked" || route === "none";
  const on = route === "none" ? item.owned : !locked;
  const note = route === "dlc" ? t("app.saves.paidDlc")
    : route === "entitled" ? t("app.saves.ownedOnSteam")
    : route === "rank" ? t("app.saves.comesWithRank", item.grantedRank ?? 0)
    : route === "fixed" ? t("app.saves.alwaysOwned")
    : route === "none" && item.grantedRank !== null ? t("app.saves.rankNeeded", item.grantedRank) : null;
  const noteTip = route === "dlc" ? t("app.saves.paidDlcHint") : route === "entitled" ? t("app.saves.ownedOnSteamHint") : undefined;
  return { route, locked, changeable, on, note, noteTip };
}

const flip = (item: ItemState, onEdit: (e: Edit) => void) => onEdit({ op: "set_owned", item: item.name, owned: !item.inUnlocks });

function Picture({ name, className }: { name: string; className?: string }) {
  const src = artFor(name);
  return src
    ? <img className={className} src={src} alt="" draggable={false} />
    : <span className={className ? `${className} ar-glyph` : "ar-glyph"} aria-hidden="true"><ModsIcon /></span>;
}

export function ArsenalTab({ draft, onEdit }: { draft: EditorState; onEdit: (e: Edit) => void }) {
  const [query, setQuery] = useState("");
  const [sel, setSel] = useState<string>(() => draft.primaryWeapons[0]?.name ?? draft.stratagems[0]?.name ?? "");
  const weaponsAll = useMemo(() => [...draft.primaryWeapons, ...draft.sidearmWeapons], [draft]);
  const every = useMemo(() => [...weaponsAll, ...draft.stratagems, ...draft.perks], [weaponsAll, draft]);
  const weapons = weaponsAll.filter((i) => itemMatches(i, query));
  const strats = groupItems(draft.stratagems, query);
  const perks = draft.perks.filter((i) => itemMatches(i, query));
  const unknown = draft.unknownUnlocks.filter((n) => n.toLowerCase().includes(query.trim().toLowerCase()));
  const count = (list: ItemState[]) => t("app.saves.ownedCount", list.filter((i) => i.owned).length, list.length);
  const kindOf = (i: ItemState): Kind => (draft.stratagems.includes(i) ? "stratagem" : draft.perks.includes(i) ? "perk" : "weapon");
  const selected = every.find((i) => i.name === sel) ?? null;
  const selectedUnknown = !selected && draft.unknownUnlocks.includes(sel) ? sel : null;
  const tile = (i: ItemState, kind: Kind) => <Tile key={i.name} item={i} kind={kind} rank={draft.rank} selected={i.name === sel} onSelect={() => setSel(i.name)} onEdit={onEdit} />;
  const nothing = weapons.length + perks.length + strats.length + unknown.length === 0;

  return (
    <div className="ar">
      <div className="ar-board">
        <div className="ar-tools">
          <label className="search">
            <SearchIcon />
            <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder={t("app.saves.searchItems")} aria-label={t("app.saves.searchItems")} spellCheck={false}
              onKeyDown={(e) => e.key === "Escape" && setQuery("")} />
            {query && <button className="bare" onClick={() => setQuery("")} aria-label={t("app.mods.clearSearch")} data-tip={t("app.mods.clearSearch")}><CloseIcon /></button>}
          </label>
          <span className="ar-count">{count(every)}</span>
        </div>

        {nothing && <p className="muted pad">{t("app.saves.noMatch")}</p>}

        {weapons.length > 0 && (
          <section className="ar-sec">
            <h3 className="rec-h cap">{t("app.saves.arsenal.weapons")}<span className="rec-h-n">{count(weaponsAll)}</span></h3>
            <div className="ar-grid">{weapons.map((i) => tile(i, "weapon"))}</div>
          </section>
        )}

        {strats.length > 0 && (
          <section className="ar-sec">
            <h3 className="rec-h cap">{t("app.saves.stratagems")}<span className="rec-h-n">{count(draft.stratagems)}</span></h3>
            <div className="ar-sgroups">
              {strats.map((g) => {
                const all = draft.stratagems.filter((i) => i.category === g.category);
                return (
                  <div key={g.category ?? ""} className="ar-sg" style={{ flexGrow: all.length, "--cols": Math.min(all.length, all.length > 4 ? 4 : 2) } as CSSProperties}>
                    <div className="ar-sg-head cap"><span>{g.category ? label(`app.saves.cat.${g.category}`, g.category) : ""}</span><span>{all.filter((i) => i.owned).length}/{all.length}</span></div>
                    <div className="ar-sgrid">{g.items.map((i) => tile(i, "stratagem"))}</div>
                  </div>
                );
              })}
            </div>
          </section>
        )}

        {perks.length > 0 && (
          <section className="ar-sec">
            <h3 className="rec-h cap">{t("app.saves.perks")}<span className="rec-h-n">{count(draft.perks)}</span></h3>
            <div className="ar-grid perks">{perks.map((i) => tile(i, "perk"))}</div>
          </section>
        )}

        {unknown.length > 0 && (
          <section className="ar-sec">
            <h3 className="rec-h cap">{t("app.saves.unknownItems")}</h3>
            <div className="ar-grid perks">
              {unknown.map((name) => (
                <div key={name} className={name === sel ? "ar-tile perk sel" : "ar-tile perk"}>
                  <button className="ar-pick" aria-pressed={name === sel} onClick={() => setSel(name)} data-tip-title={name} data-tip={t("app.saves.unknownHint")}>
                    <Picture name={name} className="ar-art" />
                    <span className="ar-tname mono">{name}</span>
                  </button>
                </div>
              ))}
            </div>
          </section>
        )}
      </div>

      {selected && <Inspector item={selected} kind={kindOf(selected)} rank={draft.rank} onEdit={onEdit} />}
      {selectedUnknown && (
        <aside className="ar-insp">
          <div className="ar-insp-cat cap">{t("app.saves.unknownItems")}</div>
          <h2 className="mono">{selectedUnknown}</h2>
          <div className="ar-insp-pic"><Picture name={selectedUnknown} className="ar-art" /></div>
          <p className="ar-insp-desc">{t("app.saves.unknownHint")}</p>
          <button className="btn small" onClick={() => { onEdit({ op: "set_owned", item: selectedUnknown, owned: false }); setSel(draft.primaryWeapons[0]?.name ?? ""); }}>{t("app.saves.removeFromSave")}</button>
        </aside>
      )}
    </div>
  );
}

function Tile({ item, kind, rank, selected, onSelect, onEdit }: { item: ItemState; kind: Kind; rank: number; selected: boolean; onSelect: () => void; onEdit: (e: Edit) => void }) {
  const o = ownership(item, rank);
  const marks = o.locked ? [] : upgradeMarks(item);
  const cls = ["ar-tile", kind, o.locked ? "dlc" : !item.owned ? "off" : "", selected ? "sel" : ""].filter(Boolean).join(" ");
  return (
    <div className={cls}>
      <button className="ar-pick" aria-pressed={selected} onClick={onSelect} aria-label={item.display} data-tip-title={item.display} data-tip={o.note ?? undefined}>
        <Picture name={item.name} className="ar-art" />
        {kind !== "stratagem" && <span className="ar-tname">{item.display}</span>}
      </button>
      {o.locked ? (
        <span className="ar-tlock" aria-hidden="true"><LockIcon /></span>
      ) : (
        <button role="checkbox" aria-checked={o.on} aria-disabled={!o.changeable} className={o.changeable ? "ar-own" : "ar-own fixed"} aria-label={item.display}
          data-tip-title={item.display} data-tip={o.changeable ? undefined : o.note ?? undefined}
          onClick={() => o.changeable && flip(item, onEdit)}>
          <span className={o.on ? "ck on" : "ck"} aria-hidden="true">{o.on && <CheckIcon />}</span>
        </button>
      )}
      {marks.length > 0 && <span className="ar-pips" aria-hidden="true">{marks.map((m, n) => <i key={n} className={m ? "on" : undefined} />)}</span>}
    </div>
  );
}

function Inspector({ item, kind, rank, onEdit }: { item: ItemState; kind: Kind; rank: number; onEdit: (e: Edit) => void }) {
  const o = ownership(item, rank);
  const group = item.category ? label(`app.saves.cat.${item.category}`, item.category)
    : kind === "perk" ? t("app.saves.perks") : kind === "stratagem" ? t("app.saves.stratagems") : t("app.saves.sidearms");
  const marks = upgradeMarks(item);
  const set = (value: number) => onEdit({ op: "set_upgrade", item: item.name, value });
  return (
    <aside className="ar-insp" key={item.name}>
      <div className="ar-insp-cat cap">{group}</div>
      <h2>{item.display}</h2>
      <div className={o.locked || !item.owned ? "ar-insp-pic off" : "ar-insp-pic"}><Picture name={item.name} className={`ar-art ${kind}`} /></div>
      {item.description && <p className="ar-insp-desc">{item.description.trim()}</p>}
      <div className="ar-insp-own">
        {o.locked
          ? <span className="ar-lock" role="img" aria-label={t("app.saves.paidDlc")}><LockIcon /></span>
          : <button role="switch" aria-checked={o.on} aria-label={t("app.saves.col.owned")} aria-disabled={!o.changeable} className={o.on ? "sw on" : "sw"}
              data-tip={o.changeable ? undefined : o.note ?? undefined} onClick={() => o.changeable && flip(item, onEdit)} />}
        <span className="ar-insp-ownl">{t("app.saves.col.owned")}</span>
        {o.note && <span className={o.route === "entitled" ? "ar-insp-note on" : "ar-insp-note"} data-tip={o.noteTip}>{o.note}</span>}
      </div>
      {!o.locked && item.upgrades.length > 0 && (
        <>
          <div className="ar-insp-cat cap">{t("app.saves.col.upgrades")}</div>
          <div className="ar-insp-ups" role="group" aria-label={t("app.saves.col.upgrades")}>
            {item.upgrades.map((u, n) => {
              const on = marks[n];
              const next = item.upgradeKind === "bitmask" ? withBit(item.upgrade, u.value, !hasBit(item.upgrade, u.value)) : levelPick(item.upgrade, u.value);
              return (
                <button key={u.name} role="checkbox" aria-checked={on} aria-disabled={!item.owned} className={item.owned ? "ar-insp-up" : "ar-insp-up off"}
                  data-tip={item.owned ? undefined : t("app.saves.ownFirst")} onClick={() => item.owned && set(next)}>
                  <span className={on ? "ck on" : "ck"} aria-hidden="true">{on && <CheckIcon />}</span>
                  <span className="cap">{u.display}</span>
                </button>
              );
            })}
          </div>
        </>
      )}
    </aside>
  );
}
