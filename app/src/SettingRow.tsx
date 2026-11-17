// The parts the Settings subtabs are made of: a group under a heading (with a "?" that opens the
// Wiki where the setting is explained), and fields with the label above the control and a note below.
import type { ReactNode } from "react";
import { t } from "./i18n";
import { go } from "./nav";

/** A round "?" beside a heading: the tooltip says what the group is for, with the WIKI tag, and a click
    opens the Wiki at `page`. */
export function Help({ page, tip }: { page: string; tip?: string }) {
  return (
    <button className="help-q" aria-label={t("app.settings.readMore")} data-tip-title={t("app.settings.readMore")}
      data-tip={tip} data-tip-wiki="" onClick={() => go("wiki", page)}>?</button>
  );
}

/** `count` puts a number in warning colours after the title, for things that need a look. */
export function Group({ title, help, tip, count, children, className }: { title: string; help?: string; tip?: string; count?: number; children: ReactNode; className?: string }) {
  return (
    <section className={className ? `set-sec ${className}` : "set-sec"}>
      <h2>
        <span className="cap">{title}</span>
        {!!count && <span className="set-count">{count}</span>}
        {help && <Help page={help} tip={tip} />}
      </h2>
      {children}
    </section>
  );
}

/** The settings of a group as a table (the shape of the Access table in Account): one row a setting. */
export function Rows({ label, children }: { label: string; children: ReactNode }) {
  return <div className="patch-table set-rows" role="list" aria-label={label}>{children}</div>;
}

/** One setting: the name and a line about the current choice at the left, the control at the right. */
export function Row({ title, note, help, children }: { title: string; note?: ReactNode; help?: string; children: ReactNode }) {
  return (
    <div className="pt-row set-row" role="listitem">
      <span className="set-row-text">
        <span className="set-row-name"><b>{title}</b>{help && <Help page={help} />}</span>
        {note && <small>{note}</small>}
      </span>
      <span className="set-row-ctl">{children}</span>
    </div>
  );
}
