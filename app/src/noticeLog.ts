// Every notice shown, kept to read again from the status bar: the last 100 from the past day, oldest
// first, also after a restart (browser storage of this window). The log never shows anything by itself.
import { useSyncExternalStore } from "react";

export type Logged = { id: number; at: number; kind: "info" | "error"; text: string };
const KEY = "obsidian.notifications";
const MAX = 100;
/** The same text again this soon (a button pressed twice) updates the last entry instead. */
const SAME_MS = 5000;
/** A notice a day old is dropped. */
export const MAX_AGE_MS = 24 * 60 * 60 * 1000;
/** The most a timer can wait (about 24.8 days). */
const TIMER_LIMIT_MS = 2 ** 31 - 1;

/** The log with one more notice, at most MAX long. */
export function addTo(log: Logged[], kind: Logged["kind"], text: string, at: number): Logged[] {
  const last = log[log.length - 1];
  if (last && last.text === text && last.kind === kind && at - last.at < SAME_MS) return [...log.slice(0, -1), { ...last, at }];
  return [...log, { id: (last?.id ?? 0) + 1, at, kind, text }].slice(-MAX);
}

/** The log without the notices a day old or older; the same array when none is. */
export function freshOnly(log: Logged[], now: number): Logged[] {
  const kept = log.filter((n) => now - n.at < MAX_AGE_MS);
  return kept.length === log.length ? log : kept;
}

function load(): Logged[] {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? "[]");
    return Array.isArray(v) ? v.filter((x) => x && typeof x.text === "string" && typeof x.at === "number") : [];
  } catch {
    return [];
  }
}

let entries: Logged[] = load();
let timer: ReturnType<typeof setTimeout> | undefined;
const listeners = new Set<() => void>();

function save() {
  try { localStorage.setItem(KEY, JSON.stringify(entries)); } catch { /* storage off: this session keeps them */ }
}

/** Drops the notices that aged out, then waits for the next one to age. */
function expire() {
  clearTimeout(timer);
  const kept = freshOnly(entries, Date.now());
  if (kept !== entries) {
    entries = kept;
    save();
    listeners.forEach((f) => f());
  }
  if (entries.length) {
    const oldest = Math.min(...entries.map((n) => n.at));
    timer = setTimeout(expire, Math.min(Math.max(oldest + MAX_AGE_MS - Date.now(), 1000), TIMER_LIMIT_MS));
  }
}

export function logNotice(kind: Logged["kind"], text: string) {
  const now = Date.now();
  entries = addTo(freshOnly(entries, now), kind, text, now);
  save();
  listeners.forEach((f) => f());
  expire();
}

expire();

export const useNoticeLog = () =>
  useSyncExternalStore(
    (f) => {
      listeners.add(f);
      // A computer asleep can hold a timer back, so opening the list checks again.
      expire();
      return () => { listeners.delete(f); };
    },
    () => entries,
  );
