// Short messages at the bottom of the window: results of imports and actions, and errors.
import { useSyncExternalStore } from "react";
import { logNotice } from "./noticeLog";

export type Notice = { id: number; kind: "info" | "error"; text: string; leaving?: boolean };
const LIFETIME_MS = 6000;
/** How long a notice takes to fade on its way out (styles.css, notice-out). */
const OUT_MS = 140;
/** Errors take longer to read and matter more. */
const ERROR_LIFETIME_MS = 14000;

let list: Notice[] = [];
let next = 1;
const listeners = new Set<() => void>();
const emit = () => listeners.forEach((f) => f());

/** A notice fades out, then goes; with animations off it goes at once. */
export function dismiss(id: number) {
  const n = list.find((x) => x.id === id);
  if (!n || n.leaving) return;
  const drop = () => { list = list.filter((x) => x.id !== id); emit(); };
  if (typeof document !== "undefined" && document.documentElement.dataset.motion === "off") return drop();
  list = list.map((x) => (x.id === id ? { ...x, leaving: true } : x));
  emit();
  setTimeout(drop, OUT_MS);
}

export function notify(kind: Notice["kind"], text: string) {
  const n = { id: next++, kind, text };
  list = [...list.slice(-4), n];
  logNotice(kind, text);
  emit();
  setTimeout(() => dismiss(n.id), kind === "error" ? ERROR_LIFETIME_MS : LIFETIME_MS);
}

export const useNotices = () =>
  useSyncExternalStore((f) => { listeners.add(f); return () => { listeners.delete(f); }; }, () => list);
