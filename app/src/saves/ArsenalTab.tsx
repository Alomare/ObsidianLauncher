// The editor's Arsenal: weapons, stratagems and perks, one table per category with a search box over them.
// Every table has the same three columns: the owned switch (centered), the item with why it is owned under
// the name, and the upgrades.
import { useMemo, useState } from "react";
import { CheckIcon, CloseIcon, LockIcon, SearchIcon } from "../Icons";
import { t } from "../i18n";
import { groupItems, hasBit, ownedRoute, withBit } from "./logic";
import type { Edit, EditorState, ItemState } from "./types";

type Section = "all" | "weapons" | "stratagems" | "perks";
const SECTIONS: Section[] = ["all", "weapons", "stratagems", "perks"];

const label = (key: string, fallback: string) => {
  const text = t(key);
  return text === key ? fallback : text;
};

export function ArsenalTab({ draft, onEdit }: { draft: EditorState; onEdit: (e: Edit) => void }) {
  const [section, setSection] = useState<Section>("all");
  const [query, setQuery] = useState("");
  const blocks = useMemo(() => {
    const out: { title: string; items: ItemState[]; id: Section }[] = [
      { id: "weapons", title: t("app.saves.primaryWeapons"), items: draft.primaryWeapons },
      { id: "weapons", title: t("app.saves.sidearms"), items: draft.sidearmWeapons },
      { id: "stratagems", title: t("app.saves.stratagems"), items: draft.stratagems },
      { id: "perks", title: t("app.saves.perks"), items: draft.perks },
    ];
    return out.filter((b) => section === "all" || b.id === section).map((b) => ({ ...b, groups: groupItems(b.items, query) })).filter((b) => b.groups.length > 0);
  }, [draft, section, query]);
  const shownUnknown = draft.unknownUnlocks.filter((n) => n.toLowerCase().includes(query.trim().toLowerCase()));
  const owned = [draft.primaryWeapons, draft.sidearmWeapons, draft.stratagems, draft.perks].flat().filter((i) => i.owned).length;
  const total = draft.primaryWeapons.length + draft.sidearmWeapons.length + draft.stratagems.length + draft.perks.length;

  return (
    <div className="ar">
      <div className="ar-tools">
        <div className="seg" role="radiogroup" aria-label={t("app.saves.tab.arsenal")}>
          {SECTIONS.map((s) => (
            <button key={s} role="radio" aria-checked={s === section} className={s === section ? "on" : undefined} onClick={() => setSection(s)}>{t(`app.saves.arsenal.${s}`)}</button>
          ))}
        </div>
        <label className="search">
          <SearchIcon />
          <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder={t("app.saves.searchItems")} aria-label={t("app.saves.searchItems")} spellCheck={false}
            onKeyDown={(e) => e.key === "Escape" && setQuery("")} />
          {query && <button className="bare" onClick={() => setQuery("")} aria-label={t("app.mods.clearSearch")} data-tip={t("app.mods.clearSearch")}><CloseIcon /></button>}
        </label>
        <span className="muted ar-count">{t("app.saves.ownedCount", owned, total)}</span>
      </div>

      {blocks.length === 0 && shownUnknown.length === 0 && <p className="muted pad">{t("app.saves.noMatch")}</p>}

      {blocks.map((b) => (
        <section key={b.title} className="ar-block">
          <h3 className="cap ar-title">{b.title}</h3>
          {b.groups.map((g) => (
            <div key={g.category ?? ""} className="ar-table" role="table" aria-label={g.category ? label(`app.saves.cat.${g.category}`, g.category) : b.title}>
              <div className="ar-th" role="row">
                <span className="th-col" role="columnheader">{t("app.saves.col.owned")}</span>
                <span className="th-col left" role="columnheader">{g.category ? label(`app.saves.cat.${g.category}`, g.category) : b.title}</span>
                <span className="th-col left" role="columnheader">{t("app.saves.col.upgrades")}</span>
              </div>
              {g.items.map((it) => <ItemRow key={it.name} item={it} rank={draft.rank} onEdit={onEdit} />)}
            </div>
          ))}
        </section>
      ))}

      {shownUnknown.length > 0 && (
        <section className="ar-block">
          <h3 className="cap ar-title">{t("app.saves.unknownItems")}</h3>
          <p className="muted ar-note">{t("app.saves.unknownHint")}</p>
          <div className="ar-table" role="table">
            {shownUnknown.map((name) => (
              <div key={name} className="ar-row" role="row">
                <span className="ar-switch" role="cell" />
                <span className="mono ar-raw" role="cell">{name}</span>
                <span role="cell"><button className="btn small" onClick={() => onEdit({ op: "set_owned", item: name, owned: false })}>{t("app.saves.removeFromSave")}</button></span>
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

function ItemRow({ item, rank, onEdit }: { item: ItemState; rank: number; onEdit: (e: Edit) => void }) {
  const route = ownedRoute(item, rank);
  const locked = route === "dlc";
  const changeable = route === "unlocked" || route === "none";
  const on = route === "none" ? item.owned : !locked;
  const note = route === "dlc" ? t("app.saves.paidDlc")
    : route === "entitled" ? t("app.saves.ownedOnSteam")
    : route === "rank" ? t("app.saves.comesWithRank", item.grantedRank ?? 0)
    : route === "fixed" ? t("app.saves.alwaysOwned")
    : route === "none" && item.grantedRank !== null ? t("app.saves.rankNeeded", item.grantedRank) : null;
  const noteTip = route === "dlc" ? t("app.saves.paidDlcHint") : route === "entitled" ? t("app.saves.ownedOnSteamHint") : undefined;
  return (
    <div className={locked ? "ar-row dlc" : "ar-row"} role="row">
      <span className="ar-switch" role="cell">
        {locked ? (
          <span className="ar-lock" role="img" aria-label={t("app.saves.paidDlc")} data-tip={t("app.saves.paidDlcHint")}><LockIcon /></span>
        ) : (
          <button role="switch" aria-checked={on} aria-label={item.display} disabled={!changeable}
            className={on ? "sw on" : "sw"} data-tip={changeable ? undefined : note ?? undefined}
            onClick={() => changeable && onEdit({ op: "set_owned", item: item.name, owned: !item.inUnlocks })} />
        )}
      </span>
      <div className="ar-name" role="cell">
        <span className="ar-disp" data-tip-title={item.display} data-tip={item.description || undefined}>{item.display}</span>
        {note && <span className={route === "entitled" ? "ar-sub on" : "ar-sub"} data-tip={noteTip}>{note}</span>}
      </div>
      <div className="ar-upcell" role="cell">
        {!locked && item.maxUpgrade > 0 && (item.upgradeKind === "bitmask"
          ? <Checks item={item} onEdit={onEdit} />
          : <Stepper item={item} onEdit={onEdit} />)}
      </div>
    </div>
  );
}

function Checks({ item, onEdit }: { item: ItemState; onEdit: (e: Edit) => void }) {
  return (
    <div className="ar-ups">
      {item.upgrades.map((u) => {
        const on = hasBit(item.upgrade, u.value);
        return (
          <button key={u.name} role="checkbox" aria-checked={on} className={item.owned ? "ar-up" : "ar-up off"} disabled={!item.owned} data-tip={item.owned ? undefined : t("app.saves.ownFirst")}
            onClick={() => onEdit({ op: "set_upgrade", item: item.name, value: withBit(item.upgrade, u.value, !on) })}>
            <span className={on ? "ck on" : "ck"} aria-hidden="true">{on && <CheckIcon />}</span>
            <span>{u.display}</span>
          </button>
        );
      })}
    </div>
  );
}

function Stepper({ item, onEdit }: { item: ItemState; onEdit: (e: Edit) => void }) {
  const set = (value: number) => onEdit({ op: "set_upgrade", item: item.name, value });
  const shown = item.upgrades.find((u) => u.value === item.upgrade)?.display;
  return (
    <div className="ar-step">
      <div className={item.owned ? "stepper" : "stepper off"} role="group" aria-label={t("app.saves.upgradeLevel")} data-tip={item.owned ? undefined : t("app.saves.ownFirst")}>
        <button disabled={!item.owned || item.upgrade <= 0} onClick={() => set(item.upgrade - 1)} aria-label={t("app.saves.lower")}>-</button>
        <span>{item.upgrade} / {item.maxUpgrade}</span>
        <button disabled={!item.owned || item.upgrade >= item.maxUpgrade} onClick={() => set(item.upgrade + 1)} aria-label={t("app.saves.higher")}>+</button>
      </div>
      <span className="ar-level">{item.upgrade === 0 ? t("app.saves.noUpgrade") : shown ?? t("app.saves.levelN", item.upgrade)}</span>
    </div>
  );
}
