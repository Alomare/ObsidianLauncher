// One release in a list of versions, folded to its head (version, badges, date) or open on its notes.
// The changelog in Settings, the Versions tab of the drawer and the versions of a Store mod page draw
// their entries with it.
import type { ReactNode } from "react";
import { ChevronIcon } from "./Icons";
import { language, t } from "./i18n";
import { Markdown, parse } from "./Markdown";
import { openUrl } from "./opener";

/** A release date as the lists show it, in the UI language. */
export const releaseDate = (iso: string | null | undefined) => (iso ? new Date(iso).toLocaleDateString(language(), { dateStyle: "medium" }) : "");

type Props = {
  version: string;
  date: string | null | undefined;
  /** The classes of the entry ("upd-entry new"); `open` is added while it is open. */
  className: string;
  open: boolean;
  onToggle: () => void;
  /** Beside the version in the head: New, Installed, the channel. */
  badges?: ReactNode;
  notes: string | null | undefined;
  /** The release page, linked under the notes. */
  url?: string | null;
  /** Above the notes when open, such as the button that switches to this version. */
  children?: ReactNode;
};

export function ReleaseEntry({ version, date, className, open, onToggle, badges, notes, url, children }: Props) {
  return (
    <li className={open ? `${className} open` : className}>
      <button className="upd-head" aria-expanded={open} onClick={onToggle}>
        <ChevronIcon />
        <b>{version}</b>
        {badges}
        <span className="date">{releaseDate(date)}</span>
      </button>
      {open && (
        <div className="upd-body">
          {children}
          {notes?.trim() ? <Markdown tokens={parse(notes).tokens} /> : <p className="muted">{t("app.update.noNotes")}</p>}
          {url && <button className="link small" onClick={() => openUrl(url).catch(() => {})}>{t("app.update.onGithub")}</button>}
        </div>
      )}
    </li>
  );
}
