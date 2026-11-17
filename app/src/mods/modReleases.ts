// The releases of a mod's GitHub repository, for the Changelogs in the mod's details. Each mod is
// asked about once per launch, page by page as the reader asks for more; GitHub allows a stranger
// 60 calls an hour, which the update checks share, so a failure waits a while before the next try.
import { useEffect } from "react";
import { call, errorText } from "../api";
import { createStore } from "../store";
import type { Release } from "../updates";

export type ModReleases = { list: Release[]; pages: number; more: boolean; loading: boolean; error: string | null; at: number };
type Page = { releases: Release[]; more: boolean };

const RETRY_MS = 5 * 60_000;
/** A quick walk through the list asks GitHub only about the mod the reader stops on. */
const SETTLE_MS = 350;
const store = createStore<ReadonlyMap<string, ModReleases>>(new Map());
const empty: ModReleases = { list: [], pages: 0, more: true, loading: false, error: null, at: 0 };

function put(id: string, next: ModReleases) {
  store.set((m) => new Map(m).set(id, next));
}

/** Asks for the next page of `id`, unless one is on the way or GitHub said there are no more. */
export async function loadMore(id: string) {
  const cur = store.get().get(id) ?? empty;
  if (cur.loading || (!cur.more && cur.pages > 0)) return;
  put(id, { ...cur, loading: true, error: null });
  try {
    const page = await call<Page>("mod_releases", { id, page: cur.pages + 1 });
    const seen = new Set(cur.list.map((r) => r.version));
    put(id, { list: [...cur.list, ...page.releases.filter((r) => !seen.has(r.version))], pages: cur.pages + 1, more: page.more, loading: false, error: null, at: Date.now() });
  } catch (e) {
    put(id, { ...cur, loading: false, error: errorText(e), at: Date.now() });
  }
}

/** Forgets what is known of `ids`, after Check for Updates: a Versions tab showing one of them, or
 *  opened later, asks GitHub again. */
export function forgetReleases(ids: string[]) {
  store.set((m) => {
    const next = new Map(m);
    for (const id of ids) if (!m.get(id)?.loading) next.delete(id);
    return next;
  });
}

/** The releases of `id` known so far; the first page loads when `on` holds for a moment. */
export function useModReleases(id: string, on: boolean): ModReleases {
  const mine = store.use((m) => m.get(id));
  useEffect(() => {
    if (!on) return;
    const cur = store.get().get(id);
    const due = !cur || (cur.pages === 0 && !cur.loading && (!cur.error || Date.now() - cur.at > RETRY_MS));
    if (!due) return;
    const timer = setTimeout(() => loadMore(id), SETTLE_MS);
    return () => clearTimeout(timer);
  }, [id, on, !!mine]);
  return mine ?? empty;
}
