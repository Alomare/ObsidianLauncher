// Short messages at the bottom of the window: results of imports and actions, and errors.
import { logNotice } from "./noticeLog";
import { createStore } from "./store";

export type Notice = { id: number; kind: "info" | "error"; text: string; leaving?: boolean };
const LIFETIME_MS = 6000;
/** How long a notice takes to fade on its way out (styles.css, notice-out). */
const OUT_MS = 140;
/** Errors take longer to read and matter more. */
const ERROR_LIFETIME_MS = 14000;

const store = createStore<Notice[]>([]);
let next = 1;

/** A notice fades out, then goes; with animations off it goes at once. */
export function dismiss(id: number) {
  const n = store.get().find((x) => x.id === id);
  if (!n || n.leaving) return;
  const drop = () => store.set((list) => list.filter((x) => x.id !== id));
  if (typeof document !== "undefined" && document.documentElement.dataset.motion === "off") return drop();
  store.set((list) => list.map((x) => (x.id === id ? { ...x, leaving: true } : x)));
  setTimeout(drop, OUT_MS);
}

export function notify(kind: Notice["kind"], text: string) {
  const n = { id: next++, kind, text };
  logNotice(kind, text);
  store.set((list) => [...list.slice(-4), n]);
  setTimeout(() => dismiss(n.id), kind === "error" ? ERROR_LIFETIME_MS : LIFETIME_MS);
}

export const useNotices = () => store.use();
