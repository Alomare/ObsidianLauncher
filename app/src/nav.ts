// Where the window is: the tab, and on the Wiki tab the page and the heading a link asked for. Links in
// Wiki pages and buttons elsewhere move around with `go`.
import { useSyncExternalStore } from "react";

export type Tab = "home" | "store" | "mods" | "saves" | "commander" | "wiki" | "settings";
export type Place = { tab: Tab; wiki: string; anchor: string };

let place: Place = { tab: "mods", wiki: "", anchor: "" };
const listeners = new Set<() => void>();

/** Opens a tab; on the Wiki, `sub` is a page id, or "page#heading" for a part of it. */
export function go(tab: Tab, sub?: string) {
  const [wiki, anchor = ""] = (sub ?? "").split("#");
  place = tab === "wiki" && sub !== undefined ? { tab, wiki, anchor } : { ...place, tab, anchor: "" };
  listeners.forEach((f) => f());
}

/** The heading a link asked for has been shown; scrolling is the reader's again. */
export function landed() {
  if (place.anchor) place = { ...place, anchor: "" };
}

export const useNav = () =>
  useSyncExternalStore((f) => { listeners.add(f); return () => { listeners.delete(f); }; }, () => place);
