// Calls into the Rust side, and the one snapshot of the library and profile the UI renders.
import { invoke } from "@tauri-apps/api/core";
import { withTask } from "./busy";
import { t } from "./i18n";
import { notify } from "./notices";
import { createStore } from "./store";
import type { CmdError, Snapshot } from "./types";

export const call = <T,>(cmd: string, args?: Record<string, unknown>) => invoke<T>(cmd, args);

const isCmdError = (e: unknown): e is CmdError => typeof e === "object" && e !== null && "code" in e && "message" in e;

/** A core error in the UI language when the catalog has it (`app.error.<code>`), else the English text. */
export function errorText(e: unknown): string {
  if (!isCmdError(e)) return String(e);
  const key = `app.error.${e.code}`;
  const text = t(key);
  return text === key ? e.message : text;
}

const snapStore = createStore<Snapshot | null>(null);
/** The revision of the snapshot on screen. Commands run side by side and their answers can come back in
 *  any order: one built before the snapshot shown (a lower `rev`) is dropped, so a switch can't flip back. */
let shownRev = -Infinity;
export function setSnapshot(s: Snapshot) {
  if (typeof s.rev === "number") {
    if (s.rev < shownRev) return;
    shownRev = s.rev;
  }
  snapStore.set(s);
}
export const currentSnapshot = () => snapStore.get();
export const useSnapshot = () => snapStore.use();

export async function refresh() {
  try { setSnapshot(await call<Snapshot>("snapshot")); } catch (e) { notify("error", errorText(e)); }
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const RETRY_MS = 2500;

/** Asks for the first snapshot until one arrives. Right after a start a call can hang, so a call that has
 *  not answered after a while gets another beside it; a failure is told once, not at every try. */
export async function loadSnapshot() {
  let told = false;
  while (!snapStore.get()) {
    const started = Date.now();
    const attempt = call<Snapshot>("snapshot").then(
      (s) => { setSnapshot(s); },
      (e) => { if (!told) { told = true; notify("error", errorText(e)); } },
    );
    await Promise.race([attempt, sleep(RETRY_MS)]);
    if (snapStore.get()) return;
    const left = RETRY_MS - (Date.now() - started);
    if (left > 0) await sleep(left);
  }
}

/** The status bar line while a command runs (`app.task.<cmd>`), "Working" for the others. */
export function taskText(cmd: string): string {
  const text = t(`app.task.${cmd}`);
  return text === `app.task.${cmd}` ? t("app.task.working") : text;
}

/** Runs a command that returns a snapshot. Errors become a notice; the result says whether it worked.
 * A command that takes longer than a blink shows in the status bar. */
export async function run(cmd: string, args?: Record<string, unknown>): Promise<boolean> {
  try {
    setSnapshot(await withTask(taskText(cmd), () => call<Snapshot>(cmd, args)));
    return true;
  } catch (e) {
    notify("error", errorText(e));
    return false;
  }
}
