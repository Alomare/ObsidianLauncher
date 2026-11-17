// What the app is working on, shown in the status bar ("Importing X…"). Several tasks can run at
// once; the newest one shows. A locking task (deploy, import) also locks Purge, Deploy and Play.
import { createStore } from "./store";

type Task = { id: number; text: string; lock: boolean };
type State = { text: string | null; locked: boolean };

let tasks: Task[] = [];
let next = 1;
const store = createStore<State>({ text: null, locked: false });

function publish() {
  store.set({ text: tasks.at(-1)?.text ?? null, locked: tasks.some((t) => t.lock) });
}

/** Shows `text` until the returned function is called. */
export function startTask(text: string, lock = false): () => void {
  const task = { id: next++, text, lock };
  tasks = [...tasks, task];
  publish();
  return () => {
    tasks = tasks.filter((t) => t.id !== task.id);
    publish();
  };
}

/** Runs `work`, showing `text` only when it takes longer than a blink. A locking task locks at once,
 *  before `work` starts, so nothing that checks the lock can slip in between. */
export async function withTask<T>(text: string, work: () => Promise<T>, lock = false): Promise<T> {
  let end: (() => void) | null = lock ? startTask(text, true) : null;
  const timer = lock ? undefined : setTimeout(() => (end = startTask(text)), 120);
  try {
    return await work();
  } finally {
    clearTimeout(timer);
    (end as (() => void) | null)?.();
  }
}

/** Whether a locking task runs now. */
export const isLocked = () => store.get().locked;
export const useBusy = () => store.use((s) => s.text);
export const useLocked = () => store.use((s) => s.locked);
