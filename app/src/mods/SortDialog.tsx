// Sort Mods: what the sorter in the core would do to the load order, shown before anything changes.
// The whole resulting order is listed, with the moved mods marked and the reason for each move; requests
// that clash are listed apart, and those mods stay where they were.
import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { CloseIcon } from "../Icons";
import { t } from "../i18n";

export type RuleKind = "before" | "after" | "before_patches" | "after_patches" | "first" | "last";
export type SortRule = { mod: string; kind: RuleKind; other?: string };
export type SortTie = { other: string; later: boolean; files: string[] };
export type SortMove = { id: string; from: number; to: number; rules: SortRule[]; tie?: SortTie };
export type SortClash = { kind: "both_first" | "both_last" | "cycle" | "locked"; mods: string[]; rules: SortRule[] };
export type SortPlan = { order: string[]; moves: SortMove[]; conflicts: SortClash[]; notes: { mod: string; missing: string }[] };

type Name = (id: string) => string;

/** "A", "A and B", "A, B and C". */
export function listOf(items: string[]): string {
  if (items.length < 2) return items.join("");
  return t("app.sort.and", items.slice(0, -1).join(", "), items[items.length - 1]);
}

/** A request as seen from the moved mod `id`. */
export function ruleText(r: SortRule, id: string, name: Name): string {
  if (r.mod === id) return t(`app.sort.own.${r.kind}`, r.other ? name(r.other) : "");
  if (r.other === id) return t(`app.sort.theirs.${r.kind}`, name(r.mod));
  return ruleLine(r, name);
}

/** A request on its own: "A loads after B". */
export function ruleLine(r: SortRule, name: Name): string {
  return t(`app.sort.rule.${r.kind}`, name(r.mod), r.other ? name(r.other) : "");
}

/** Why a mod moved, in one line. */
export function moveText(m: SortMove, name: Name): string {
  const why = m.rules.map((r) => ruleText(r, m.id, name)).join(". ");
  if (!m.tie) return why;
  const n = m.tie.files.length;
  const what = n === 1 ? m.tie.files[0] : t("app.sort.nFiles", n);
  const tie = t(m.tie.later ? "app.sort.tieLater" : "app.sort.tieEarlier", name(m.tie.other), what);
  return why ? `${why}. ${tie}` : tie;
}

/** What clashed, and what to do about it. */
export function clashText(c: SortClash, name: Name): string {
  const mods = listOf(c.mods.map(name));
  const fix = c.kind === "locked" ? t("app.sort.fixLocked") : t("app.sort.fix");
  if (c.kind === "both_last" || c.kind === "both_first") {
    const end = c.kind === "both_last" ? "Last" : "First";
    return `${t(c.mods.length === 2 ? `app.sort.both${end}` : `app.sort.all${end}`, mods)} ${fix}`;
  }
  const rules = c.rules.map((r) => ruleLine(r, name)).join("; ");
  return `${t(c.kind === "locked" ? "app.sort.locked" : "app.sort.cycle", rules)} ${fix}`;
}

/** Bars of three lengths with an arrow: the Sort button. */
export const SortIcon = () => (
  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <path d="M4 6h9M4 12h6M4 18h3M17.5 4.5v15M14 16l3.5 3.5L21 16" />
  </svg>
);

const MoveArrow =({ up }: { up: boolean }) => (
  <svg className="sort-arrow" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="square" aria-hidden="true">
    <path d={up ? "M12 20V5M6 11l6-6 6 6" : "M12 4v15M6 13l6 6 6-6"} />
  </svg>
);

type Props = {
  plan: SortPlan;
  name: Name;
  /** Mods that are off, greyed as on the Mods tab. */
  off: Set<string>;
  onApply: () => void;
  onClose: () => void;
};

export function SortDialog({ plan, name, off, onApply, onClose }: Props) {
  const main = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    main.current?.focus();
    const key = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [onClose]);
  const moves = new Map(plan.moves.map((m) => [m.id, m]));
  const n = plan.moves.length;
  const lead = n ? null : plan.conflicts.length ? t("app.sort.nothing") : t("app.sort.inOrder");
  return createPortal(
    <div className="modal-back" onPointerDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal sort" role="dialog" aria-modal="true" aria-label={t("app.sort.title")}>
        <div className="head">
          <h2 className="cap">{t("app.sort.title")}</h2>
          <button className="bare close" onClick={onClose} aria-label={t("app.close")} data-tip={t("app.close")}><CloseIcon /></button>
        </div>
        {lead && <p className="lead">{lead}</p>}
        {n > 0 && (
          <div className="sort-scroll">
            <table className="sort-table">
              <thead>
                <tr><th className="num">#</th><th>{t("app.sort.colMod")}</th><th>{t("app.sort.colChange")}</th></tr>
              </thead>
              <tbody>
                {plan.order.map((id, i) => {
                  const m = moves.get(id);
                  const files = m?.tie?.files ?? [];
                  const cls = [m && "moved", off.has(id) && "off"].filter(Boolean).join(" ") || undefined;
                  return (
                    <tr key={id} className={cls}>
                      <td className="num">{i + 1}</td>
                      <td className="nm">{name(id)}</td>
                      <td className="change">
                        {m && (
                          <span className="why" data-tip-title={files.length ? t("app.sort.sharedFiles") : undefined} data-tip={files.length ? files.join("\n") : undefined}>
                            <span className="from-to"><MoveArrow up={m.to < m.from} />{t("app.sort.fromTo", m.from + 1, m.to + 1)}</span>
                            <span className="reason">{moveText(m, name)}</span>
                          </span>
                        )}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
        {plan.conflicts.length > 0 && (
          <>
            <h3>{t("app.sort.notMoved")}</h3>
            <ul className="sort-notes">{plan.conflicts.map((c, i) => <li key={i}>{clashText(c, name)}</li>)}</ul>
          </>
        )}
        {plan.notes.length > 0 && (
          <>
            <h3>{t("app.sort.skipped")}</h3>
            <ul className="sort-notes muted">{plan.notes.map((x, i) => <li key={i}>{t("app.sort.missing", name(x.mod), x.missing)}</li>)}</ul>
          </>
        )}
        <div className="modal-foot">
          {n > 0 ? (
            <>
              <button className="btn" onClick={onClose}>{t("app.cancel")}</button>
              <button className="btn play" ref={main} onClick={onApply}>{t("app.sort.apply")}</button>
            </>
          ) : (
            <button className="btn" ref={main} onClick={onClose}>{t("app.close")}</button>
          )}
        </div>
      </div>
    </div>,
    document.body,
  );
}
