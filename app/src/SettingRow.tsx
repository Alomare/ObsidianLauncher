// The parts the Settings subtabs are made of: a group under a heading (with a "?" that opens the
// Wiki where the setting is explained), and fields with the label above the control and a note below.
import type { ReactNode } from "react";
import { t } from "./i18n";
import { go } from "./nav";

/** A round "?" beside a heading: the tooltip says what the group is for, a click opens the Wiki at `page`. */
export function Help({ page, tip }: { page: string; tip?: string }) {
  return (
    <button className="help-q" aria-label={t("app.settings.readMore")} data-tip-title={tip ? undefined : t("app.settings.readMore")}
      data-tip={tip ? `${tip}\n${t("app.scope.wikiHint")}` : t("app.scope.wikiHint")} onClick={() => go("wiki", page)}>?</button>
  );
}

export function Group({ title, help, tip, children, className }: { title: string; help?: string; tip?: string; children: ReactNode; className?: string }) {
  return (
    <section className={className ? `set-sec ${className}` : "set-sec"}>
      <h2>
        <span className="cap">{title}</span>
        {help && <Help page={help} tip={tip} />}
      </h2>
      {children}
    </section>
  );
}

/** One setting: the label, the control, then a line about the current choice. */
export function Field({ label, note, children }: { label?: string; note?: ReactNode; children: ReactNode }) {
  return (
    <div className="field">
      {label && <span className="field-label">{label}</span>}
      <div className="field-ctl">{children}</div>
      {note && <small className="field-note">{note}</small>}
    </div>
  );
}
