// The releases of a mod's GitHub repository, for the Changelogs in the mod's details. Each mod is
// asked about once per launch, page by page as the reader asks for more; GitHub allows a stranger
// 60 calls an hour, which the update checks share, so a failure waits a while before the next try.
import { useEffect, useState } from "react";
import { call, errorText } from "../api";
import type { Release } from "../updates";

export type ModReleases = { list: Release[]; pages: number; more: boolean; loading: boolean; error: string | null; at: number };
type Page = { releases: Release[]; more: boolean };

const RETRY_MS = 5 * 60_000;
/** A quick walk through the list asks GitHub only about the mod the reader stops on. */
const SETTLE_MS = 350;
const cache = new Map<string, ModReleases>();
const listeners = new Set<() => void>();
const empty: ModReleases = { list: [], pages: 0, more: true, loading: false, error: null, at: 0 };

function put(id: string, next: ModReleases) {
  cache.set(id, next);
  listeners.forEach((f) => f());
}

/** Asks for the next page of `id`, unless one is on the way or GitHub said there are no more. */
export async function loadMore(id: string) {
  const cur = cache.get(id) ?? empty;
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

/** The releases of `id` known so far; the first page loads when `on` holds for a moment. */
export function useModReleases(id: string, on: boolean): ModReleases {
  const [, bump] = useState(0);
  useEffect(() => {
    const f = () => bump((n) => n + 1);
    listeners.add(f);
    return () => { listeners.delete(f); };
  }, []);
  useEffect(() => {
    if (!on) return;
    const cur = cache.get(id);
    const due = !cur || (cur.pages === 0 && !cur.loading && (!cur.error || Date.now() - cur.at > RETRY_MS));
    if (!due) return;
    const timer = setTimeout(() => loadMore(id), SETTLE_MS);
    return () => clearTimeout(timer);
  }, [id, on]);
  return cache.get(id) ?? empty;
}
